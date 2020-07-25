use log::{debug, error};
use rusty_ffmpeg::ffi;

use std::{ffi::CString, marker::PhantomData, slice};

struct GraphParser<'buffer> {
    ptr: *const u8,
    end: *const u8,
    _marker: PhantomData<&'buffer u8>,
}

#[derive(Debug, Default)]
struct FilterGraph<'buffer> {
    // Used in filter creation
    scale_sws_opts: Option<&'buffer [u8]>,
}

#[derive(Debug, Default)]
struct FilterContext {
    /// index of the filter(0..num_filter)
    index: usize,

    /// name of the filter
    filt_name: String,

    /// name of the filter instance
    inst_name: String,

    /// currently not used, maybe used later when graph is lazy initialized.
    args: String,

    /// Used in input and output linking
    nb_inputs: usize,
    nb_outputs: usize,
}

struct FilterLink {
    from_filter: usize,
    from_pad_idx: usize,
    to_filter: usize,
    to_pad_idx: usize,
}

/// Customized version of `AVFilterInOut` for convenient purpose
#[derive(Debug, Clone)]
struct FilterInOut<'buffer> {
    name: Option<&'buffer [u8]>,
    pad_idx: usize,
    /// Index of filter in the filter array, is None when it is an unlinked input
    filter_ctx: Option<usize>,
}

impl<'buffer> GraphParser<'buffer> {
    fn new(bytes: &'buffer str) -> Self {
        let ptr = bytes.as_ptr();
        unsafe {
            Self {
                ptr,
                // length of &str is length of inner bytes array
                end: ptr.add(bytes.len()),
                _marker: PhantomData,
            }
        }
    }

    fn get(&mut self) -> Option<u8> {
        (self.ptr < self.end).then(|| unsafe {
            let x = *self.ptr;
            self.ptr = self.ptr.add(1);
            x
        })
    }

    fn peek(&self) -> Option<u8> {
        (self.ptr < self.end).then_some(unsafe { *self.ptr })
    }

    fn peek_len(&self, len: usize) -> Option<&'buffer [u8]> {
        unsafe {
            (self.end.offset_from(self.ptr) as usize >= len)
                .then_some(slice::from_raw_parts(self.ptr, len))
        }
    }

    fn peek_until<F>(&self, f: F) -> Option<&'buffer [u8]>
    where
        F: Fn(u8) -> bool,
    {
        let mut end = None;
        let mut it = self.ptr;
        while it < self.end {
            if f(unsafe { *it }) {
                end = Some(it);
                break;
            }
            it = unsafe { it.add(1) };
        }

        end.map(|end| unsafe {
            slice::from_raw_parts(self.ptr, end.offset_from(self.ptr) as usize)
        })
    }

    fn peek_until_end<F>(&self, f: F) -> &'buffer [u8]
    where
        F: Fn(u8) -> bool,
    {
        let mut it = self.ptr;
        while it < self.end && !f(unsafe { *it }) {
            it = unsafe { it.add(1) };
        }

        unsafe { slice::from_raw_parts(self.ptr, it.offset_from(self.ptr) as usize) }
    }

    fn remaining(&self) -> &'buffer [u8] {
        self.peek_until_end(|_| false)
    }

    fn skip_ws(&mut self) {
        let mut it = self.ptr;
        while it < self.end {
            match unsafe { *it } {
                b' ' | b'\n' | b'\r' | b'\t' => it = unsafe { it.add(1) },
                _ => break,
            }
        }
        self.ptr = it;
    }

    fn skip(&mut self, i: usize) {
        let dest = unsafe { self.ptr.add(i) };
        self.ptr = if dest <= self.end { dest } else { self.end };
    }

    fn parse_sws_flags(&mut self, graph: &mut FilterGraph<'buffer>) -> Result<(), ()> {
        // IMPROVEMENT reorganize the processing flow than the original FFmpeg
        if self.peek_len(10) != Some(b"sws_flags=") {
            return Ok(());
        }

        // keep the 'flags=' part
        self.skip(4);

        let p = if let Some(x) = self.peek_until(|x| x == b';') {
            x
        } else {
            error!("sws_flags not terminated with ';'.");
            return Err(());
        };

        graph.scale_sws_opts = Some(p);

        self.skip(graph.scale_sws_opts.unwrap().len() + 1);
        Ok(())
    }

    fn parse_inputs(
        &mut self,
        curr_inputs: &mut Vec<FilterInOut<'buffer>>,
        open_outputs: &mut Vec<FilterInOut<'buffer>>,
    ) -> Result<(), ()> {
        let mut parsed_inputs = vec![];

        for pad in 0.. {
            if self.peek() != Some(b'[') {
                break;
            }
            self.skip(1);

            let name = match self.peek_until(|x| x == b']') {
                Some(x) => x,
                None => return Err(()),
            };

            self.skip(name.len() + 1);

            // `extract_inout(name, open_outputs)`
            let new_input = open_outputs
                .iter()
                .enumerate()
                .find_map(|(i, open_output)| (open_output.name == Some(name)).then_some(i))
                .map(|i| open_outputs.remove(i))
                .unwrap_or(FilterInOut {
                    name: Some(name),
                    pad_idx: pad,
                    filter_ctx: None,
                });

            parsed_inputs.push(new_input);
            self.skip_ws();
        }
        *curr_inputs = parsed_inputs
            .iter()
            .chain(curr_inputs.iter())
            .cloned()
            .collect();
        Ok(())
    }

    fn create_filter(
        ctx: &mut FilterGraph,
        name: &[u8],
        args: &[u8],
        index: usize,
    ) -> Option<FilterContext> {
        let mut inst_name = format!("Parsed_{}_{}", String::from_utf8_lossy(name), index);
        let mut filt_name = String::from(String::from_utf8_lossy(name));
        if let Some(index) = name
            .iter()
            .enumerate()
            .find_map(|(i, &x)| (x == b'@').then_some(i))
        {
            if index + 1 != name.len() {
                inst_name = String::from_utf8_lossy(name).into();
                filt_name =
                    String::from_utf8_lossy(unsafe { slice::from_raw_parts(name.as_ptr(), index) })
                        .into();
            }
        }

        let filt = {
            let filt_name_c = CString::new(filt_name.clone()).unwrap();
            let filt = unsafe { ffi::avfilter_get_by_name(filt_name_c.as_ptr()) };
            if filt.is_null() {
                error!("No such filter: '{}'", filt_name);
                return None;
            }
            filt
        };

        let args = {
            let args: String = String::from_utf8_lossy(args).into();
            if filt_name == "scale"
                && (args.is_empty() || !args.contains("flags"))
                && ctx.scale_sws_opts.is_some()
            {
                let scale_sws_opts = String::from_utf8_lossy(ctx.scale_sws_opts.unwrap());
                if args.is_empty() {
                    scale_sws_opts.into()
                } else {
                    format!("{}:{}", args, scale_sws_opts)
                }
            } else {
                args
            }
        };

        // nb_inputs and nb_outputs cannot be determined only by:
        // ```rust
        // let filt = find filter
        // nb_inputs = ffi::avfilter_pad_count(filt.inputs),
        // nb_outputs = ffi::avfilter_pad_count(filt.outputs),
        // ```
        // the nb_inputs and nb_outputs can be changed with `avfilter_init_str`
        // with or without specific args.
        let (nb_inputs, nb_outputs) = unsafe {
            let inst_name_c = CString::new(inst_name.clone()).unwrap();
            let args_c = CString::new(args.clone()).unwrap();

            let graph = ffi::avfilter_graph_alloc().as_mut().unwrap();
            graph.nb_threads = 1;
            let filt_ctx =
                ffi::avfilter_graph_alloc_filter(graph as *mut _, filt, inst_name_c.as_ptr());
            if filt_ctx.is_null() {
                error!("Error creating filter '{}'\n", filt_name);
                return None;
            }
            let ret = ffi::avfilter_init_str(filt_ctx, args_c.as_ptr());
            if ret < 0 {
                if args.is_empty() {
                    error!("Error initializing filter '{}'", filt_name);
                } else {
                    error!(
                        "Error initializing filter '{}' with args '{}'",
                        filt_name, args
                    );
                }
            }
            let filt_ctx = filt_ctx.as_ref().unwrap();
            (filt_ctx.nb_inputs as usize, filt_ctx.nb_outputs as usize)
        };
        Some(FilterContext {
            index,
            filt_name,
            inst_name: inst_name.clone(),
            nb_inputs,
            nb_outputs,
            args,
        })
    }

    fn parse_filter(
        &mut self,
        index: usize,
        filt_ctx: &mut FilterContext,
        graph: &mut FilterGraph,
    ) -> Result<(), ()> {
        let name = self.peek_until_end(|x| match x {
            b'=' | b',' | b';' | b'[' => true,
            _ => false,
        });
        self.skip(name.len());

        let opts = if self.peek() == Some(b'=') {
            self.skip(1);

            let opts = self.peek_until_end(|x| match x {
                b'[' | b']' | b',' | b';' => true,
                _ => false,
            });

            self.skip(opts.len());

            opts
        } else {
            b""
        };

        let trim = |s: &[u8]| {
            let begin = (0..s.len()).find(|&i| match s[i] {
                b' ' | b'\n' | b'\t' => false,
                _ => true,
            });
            let end = (0..s.len()).rev().find(|&i| match s[i] {
                b' ' | b'\n' | b'\t' => false,
                _ => true,
            });
            match (begin, end) {
                (Some(begin), Some(end)) => s[begin..=end].to_vec(),
                _ => vec![],
            }
        };

        let (name, opts) = (trim(name), trim(opts));

        *filt_ctx = match Self::create_filter(graph, &name, &opts, index) {
            Some(x) => x,
            None => return Err(()),
        };

        Ok(())
    }

    fn link_filter_inouts(
        index: usize,
        links: &mut Vec<FilterLink>,
        filt_ctx: &mut FilterContext,
        curr_inputs: &mut Vec<FilterInOut<'buffer>>,
        open_inputs: &mut Vec<FilterInOut<'buffer>>,
    ) -> Result<(), ()> {
        for pad in 0..filt_ctx.nb_inputs {
            let mut p = if curr_inputs.is_empty() {
                FilterInOut {
                    name: None,
                    filter_ctx: None,
                    pad_idx: 0,
                }
            } else {
                curr_inputs.remove(0)
            };

            if let Some(i) = p.filter_ctx {
                links.push(FilterLink {
                    from_filter: i,
                    from_pad_idx: p.pad_idx,
                    to_filter: index,
                    to_pad_idx: pad,
                });
            } else {
                p.filter_ctx = Some(filt_ctx.index);
                p.pad_idx = pad;
                open_inputs.push(p);
            }
        }

        if !curr_inputs.is_empty() {
            error!(
                r#"Too many inputs specified for the "{}" filter."#,
                filt_ctx.filt_name
            );
            return Err(());
        }

        for pad in 0..filt_ctx.nb_outputs {
            curr_inputs.push(FilterInOut {
                name: None,
                filter_ctx: Some(filt_ctx.index),
                pad_idx: pad,
            })
        }

        Ok(())
    }

    fn parse_outputs(
        &mut self,
        index: usize,
        links: &mut Vec<FilterLink>,
        curr_inputs: &mut Vec<FilterInOut<'buffer>>,
        open_inputs: &mut Vec<FilterInOut<'buffer>>,
        open_outputs: &mut Vec<FilterInOut<'buffer>>,
    ) -> Result<(), ()> {
        // BTW, the `curr_inputs` is actually `curr_outputs`.
        loop {
            if self.peek() != Some(b'[') {
                break;
            }
            self.skip(1);

            let name = match self.peek_until(|x| x == b']') {
                Some(x) => x,
                None => return Err(()),
            };

            self.skip(name.len() + 1);

            let mut input = if curr_inputs.is_empty() {
                error!(
                    "No output pad can be associated to link label '{}'.",
                    String::from_utf8_lossy(name)
                );
                return Err(());
            } else {
                curr_inputs.remove(0)
            };

            // Fix dangling open_inputs
            let open_input = open_inputs
                .iter()
                .enumerate()
                .find_map(|(i, open_input)| (open_input.name == Some(name)).then_some(i))
                .map(|i| open_inputs.remove(i));

            if let Some(open_input) = open_input {
                // All the FilterInOut in curr_inputs(which is all the outputs)
                // should link to a filter_ctx.
                let in_index = open_input.filter_ctx.unwrap();

                links.push(FilterLink {
                    from_filter: index,
                    from_pad_idx: input.pad_idx,
                    to_filter: in_index,
                    to_pad_idx: open_input.pad_idx,
                });
            } else {
                input.name = Some(name);
                open_outputs.push(input);
            }
            self.skip_ws();
        }
        Ok(())
    }
}

pub fn avfilter_graph_parse2(filters: &str) -> Result<(), ()> {
    let mut graph = FilterGraph::default();

    let mut parser = GraphParser::new(filters);

    let mut filters = vec![];
    let mut links = vec![];

    parser.skip_ws();

    parser.parse_sws_flags(&mut graph).unwrap();

    let mut curr_inputs = vec![];
    let mut open_inputs = vec![];
    let mut open_outputs = vec![];

    for index in 0.. {
        let mut filter = FilterContext::default();

        parser.skip_ws();

        parser.parse_inputs(&mut curr_inputs, &mut open_outputs)?;

        parser.parse_filter(index, &mut filter, &mut graph)?;

        GraphParser::link_filter_inouts(
            index,
            &mut links,
            &mut filter,
            &mut curr_inputs,
            &mut open_inputs,
        )?;

        parser.parse_outputs(
            index,
            &mut links,
            &mut curr_inputs,
            &mut open_inputs,
            &mut open_outputs,
        )?;

        parser.skip_ws();

        filters.push(filter);

        // IMPROVEMENT reorganize the program flow
        match parser.peek() {
            Some(b',') => parser.skip(1),
            Some(b';') => {
                open_outputs.append(&mut curr_inputs);
                parser.skip(1)
            }
            Some(_) => {
                error!(
                    r#"Unable to parse graph description substring: "{}""#,
                    String::from_utf8_lossy(parser.remaining())
                );
                return Err(());
            }
            None => break,
        }
    }

    open_outputs.append(&mut curr_inputs);

    let scale_sws_opts_serialization = |graph: &FilterGraph| {
        if let Some(scale_sws_opts) = graph.scale_sws_opts {
            let size = scale_sws_opts.len() + 1;
            println!(
                r#"
av_freep(&graph->scale_sws_opts);
if (!(graph->scale_sws_opts = av_mallocz({})))
    return AVERROR(ENOMEM);
av_strlcpy(graph->scale_sws_opts, "{}", {});
"#,
                size,
                String::from_utf8_lossy(scale_sws_opts),
                size,
            );
        }
    };

    let filter_serialization = |filter: &FilterContext, code_name: &str| {
        // We can ensure file can be always found here.
        // TODO change *filt_ctx to filter(also since it's expanded, it should be turned in to the inst_name(consider the @ in it...)), change log_ctx to graph, change ctx to graph
        println!(
            r#"
AVFilterContext* {} = avfilter_graph_alloc_filter(ctx, avfilter_get_by_name("{}"), "{}");
if (!{}) {{
    av_log(log_ctx, AV_LOG_ERROR,
        "Error creating filter '{}'\n");
    return AVERROR(ENOMEM);
}}
avfilter_init_str({}, "{}");
"#,
            code_name,
            filter.filt_name,
            filter.inst_name,
            code_name,
            filter.filt_name,
            code_name,
            filter.args,
        );
    };

    let filter_link_serialization = |filters_code_name: &[String], link: &FilterLink| {
        println!(
            r#"
if ((ret = avfilter_link({}, {}, {}, {}))) {{
    av_log(log_ctx, AV_LOG_ERROR,
            "Cannot create the link {}:{} -> {}:{}\n",
    return ret;
}}
"#,
            filters_code_name[link.from_filter],
            link.from_pad_idx,
            filters_code_name[link.to_filter],
            link.to_pad_idx,
            filters_code_name[link.from_filter],
            link.from_pad_idx,
            filters_code_name[link.to_filter],
            link.to_pad_idx,
        );
    };

    let inout_serialization =
        |filters_code_name: &[String], inout: &FilterInOut, code_name: &str| {
            // TODO: Should AVFilterInOut::name be initialized? currently I
            // don't see it's usage at last. So it's not initialized currently.
            // If name initializing is needed, it should also be malloced like
            // what we do to scale_sws_flags because it will be freed elsewhere.
            println!(
                r#"
AVFilterInOut *{};
if (!({} = av_mallocz(sizeof(AVFilterInOut)))) {{
    av_free(name);
    return AVERROR(ENOMEM);
}}
{}->pad_idx = {};
{}->filt_ctx = {};
"#,
                code_name,
                code_name,
                code_name,
                inout.pad_idx,
                code_name,
                filters_code_name[inout.filter_ctx.unwrap()]
            );
        };

    let inout_link_serialization = |from_code_name: &str, to_code_name: &str| {
        println!(
            r#"
{}->next = {};
"#,
            from_code_name, to_code_name
        );
    };

    scale_sws_opts_serialization(&graph);

    let filters_code_name = filters
        .iter()
        .enumerate()
        .fold(vec![], |mut vec, (i, filter)| {
            vec.push(format!("filter_{}_{}", filter.filt_name, i));
            vec
        });

    let inputs_code_name = open_inputs
        .iter()
        .enumerate()
        .fold(vec![], |mut vec, (i, _input)| {
            vec.push(format!("input_{}", i));
            vec
        });

    let outputs_code_name =
        open_outputs
            .iter()
            .enumerate()
            .fold(vec![], |mut vec, (i, _output)| {
                vec.push(format!("output_{}", i));
                vec
            });

    // Create filter:
    for (filter, code_name) in filters.iter().zip(filters_code_name.iter()) {
        filter_serialization(filter, code_name);
    }

    // Create links:
    for link in links.iter() {
        filter_link_serialization(&filters_code_name, link)
    }

    // Create inputs:
    for (input, code_name) in open_inputs.iter().zip(inputs_code_name.iter()) {
        inout_serialization(&filters_code_name, input, code_name);
    }

    // Create outputs:
    for (output, code_name) in open_outputs.iter().zip(outputs_code_name.iter()) {
        inout_serialization(&filters_code_name, output, code_name);
    }

    // Link inputs:
    println!(
        r#"
*inputs = {};
*outputs = {};
"#,
        inputs_code_name[0], outputs_code_name[0]
    );

    for i in 1..inputs_code_name.len() {
        inout_link_serialization(&inputs_code_name[i - 1], &inputs_code_name[i]);
    }

    // Link outputs:
    for i in 1..outputs_code_name.len() {
        inout_link_serialization(&outputs_code_name[i - 1], &outputs_code_name[i]);
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn get() {
        let mut p = GraphParser::new("abcd");
        assert_eq!(p.get(), Some(b'a'));
        assert_eq!(p.get(), Some(b'b'));
        assert_eq!(p.get(), Some(b'c'));
        assert_eq!(p.get(), Some(b'd'));
        assert_eq!(p.get(), None);
        assert_eq!(p.get(), None);
    }

    #[test]
    fn remaining() {
        let mut p = GraphParser::new("abcd");
        assert_eq!(p.remaining(), b"abcd");
        p.skip(1);
        assert_eq!(p.remaining(), b"bcd");
        p.skip(3);
        assert_eq!(p.remaining(), b"");
    }

    #[test]
    fn peek_len() {
        let mut p = GraphParser::new("abcdefgh");
        p.skip(1);
        assert_eq!(p.peek_len(3), Some(b"bcd" as &[u8]));
        assert_eq!(p.peek_len(3), Some(b"bcd" as &[u8]));
        p.skip(2);
        assert_eq!(p.peek_len(5), Some(b"defgh" as &[u8]));
        assert_eq!(p.peek_len(6), None);
    }

    #[test]
    fn peek() {
        let mut p = GraphParser::new("abcd");
        assert_eq!(p.peek(), Some(b'a'));
        p.skip(1);
        assert_eq!(p.peek(), Some(b'b'));
        p.skip(1);
        assert_eq!(p.peek(), Some(b'c'));
        p.skip(1);
        assert_eq!(p.peek(), Some(b'd'));
        p.skip(1);
        assert_eq!(p.peek(), None);
    }

    #[test]
    fn peek_until() {
        let mut p = GraphParser::new("abcd;cdef;a;");
        assert_eq!(p.peek_until(|x| x == b';'), Some(b"abcd" as &[u8]));
        p.skip(5);
        assert_eq!(p.peek_until(|x| x == b';'), Some(b"cdef" as &[u8]));
        p.skip(5);
        assert_eq!(p.peek_until(|x| x == b';'), Some(b"a" as &[u8]));
        p.skip(5);
        assert_eq!(p.peek_until(|x| x == b';'), None);
    }

    #[test]
    fn peek_until_end() {
        let mut p = GraphParser::new("abcd;cdef;a;");
        assert_eq!(p.peek_until_end(|x| x == b';'), b"abcd");
        p.skip(5);
        assert_eq!(p.peek_until_end(|x| x == b';'), b"cdef");
        p.skip(5);
        assert_eq!(p.peek_until_end(|x| x == b';'), b"a");
        p.skip(5);
        assert_eq!(p.peek_until_end(|x| x == b';'), b"");
    }

    #[test]
    fn skip_ws() {
        let mut p = GraphParser::new("\r\n\t  \r\r\n\t\t\n\n\r");
        p.skip_ws();
        assert_eq!(None, p.peek());
        let mut p = GraphParser::new("a\r\n\t \t\r\r\n\t\t\n\n\r");
        p.skip_ws();
        assert_eq!(Some(b'a'), p.peek());
        let mut p = GraphParser::new("\r\n \t\ta\r\r\n\t\t\n\n\r");
        p.skip_ws();
        assert_eq!(Some(b'a'), p.peek());
        let mut p = GraphParser::new("\r\n\t\t \r\r\n\t\t\n\n\ra");
        p.skip_ws();
        assert_eq!(Some(b'a'), p.peek());
    }

    #[test]
    fn skip() {
        let mut p = GraphParser::new("\r\n\t\ta\t\r\t\t\t\t\t\r\n\n\n\r");
        p.skip(5);
        p.skip_ws();
        assert_eq!(None, p.peek());
        let mut p = GraphParser::new("\r\n\t\ta\t\r\t\t\t\t\t\r\n\n\n\r");
        p.skip(4);
        p.skip_ws();
        assert_eq!(Some(b'a'), p.peek());
    }

    #[test]
    fn sws_flags() {
        let graph = &mut FilterGraph::default();
        let mut p = GraphParser::new("sws_flags=emm;");
        assert!(p.parse_sws_flags(graph).is_ok());
        assert_eq!(graph.scale_sws_opts, Some(b"flags=emm" as &[u8]));
        assert_eq!(None, p.peek());

        let mut p = GraphParser::new("sws_flags=emm");
        assert!(p.parse_sws_flags(graph).is_err());

        let graph = &mut FilterGraph::default();
        let mut p = GraphParser::new("sws_flag=emm");
        assert!(p.parse_sws_flags(graph).is_ok());
        assert!(graph.scale_sws_opts.is_none());

        let mut p = GraphParser::new("sws_flags=;");
        assert!(p.parse_sws_flags(graph).is_ok());
        assert_eq!(graph.scale_sws_opts, Some(b"flags=" as &[u8]));
        assert_eq!(None, p.peek());
    }

    #[test]
    fn inputs() {
        let curr_inputs = &mut vec![];
        let open_outputs = &mut vec![];
        let mut p = GraphParser::new("[foo][bar]fakefilter[abc][def]");
        assert!(p.parse_inputs(curr_inputs, open_outputs).is_ok());
        assert_eq!(curr_inputs[0].name, Some(b"foo" as &[u8]));
        assert_eq!(curr_inputs[1].name, Some(b"bar" as &[u8]));
    }

    #[test]
    fn filter_wrong() {
        let filter = &mut FilterContext::default();
        let graph = &mut FilterGraph::default();
        assert!(GraphParser::new("asdbfajsdfkaslkdf[abc][def]")
            .parse_filter(0, filter, graph)
            .is_err());
        assert!(GraphParser::new("overlayoverlay[abc][def]")
            .parse_filter(0, filter, graph)
            .is_err());
        assert!(GraphParser::new("fakefilter[abc][def]")
            .parse_filter(0, filter, graph)
            .is_err());
        assert!(GraphParser::new("setsetset[abc][def]")
            .parse_filter(0, filter, graph)
            .is_err());
        assert!(GraphParser::new("foobar[abc][def]")
            .parse_filter(0, filter, graph)
            .is_err());
        assert!(GraphParser::new("nullnull[abc][def]")
            .parse_filter(0, filter, graph)
            .is_err());
    }

    #[test]
    fn filter_no_swscale_opts() {
        let filter = &mut FilterContext::default();
        let graph = &mut FilterGraph::default();
        let mut p = GraphParser::new("split[abc][def]");
        assert!(p.parse_filter(42, filter, graph).is_ok());
        assert_eq!(filter.index, 42);
        assert_eq!(filter.filt_name, "split");
        assert_eq!(filter.inst_name, "Parsed_split_42");
        assert_eq!(filter.args, "");
        assert_eq!(filter.nb_inputs, 1);
        assert_eq!(filter.nb_outputs, 2);
    }

    #[test]
    fn filter_have_swscale_opts() {
        let filter = &mut FilterContext::default();
        let graph = &mut FilterGraph {
            scale_sws_opts: Some(b"flags=+accurate_rnd+bitexact"),
        };
        let mut p = GraphParser::new("scale[abc]");
        assert!(p.parse_filter(0, filter, graph).is_ok());
        assert_eq!(filter.index, 0);
        assert_eq!(filter.filt_name, "scale");
        assert_eq!(filter.inst_name, "Parsed_scale_0");
        assert_eq!(filter.args, "flags=+accurate_rnd+bitexact");
        assert_eq!(filter.nb_inputs, 1);
        assert_eq!(filter.nb_outputs, 1);
    }

    #[test]
    fn filter_with_opts() {
        let filter = &mut FilterContext::default();
        let graph = &mut FilterGraph::default();
        let mut p = GraphParser::new("overlay=5:5[abc]");
        assert!(p.parse_filter(666, filter, graph).is_ok());
        assert_eq!(filter.index, 666);
        assert_eq!(filter.filt_name, "overlay");
        assert_eq!(filter.inst_name, "Parsed_overlay_666");
        assert_eq!(filter.args, "5:5");
        assert_eq!(filter.nb_inputs, 2);
        assert_eq!(filter.nb_outputs, 1);

        let filter = &mut FilterContext::default();
        let graph = &mut FilterGraph {
            scale_sws_opts: Some(b"flags=+accurate_rnd+bitexact"),
        };
        let mut p = GraphParser::new("scale=5:5[abc]");
        assert!(p.parse_filter(666, filter, graph).is_ok());
        assert_eq!(filter.index, 666);
        assert_eq!(filter.filt_name, "scale");
        assert_eq!(filter.inst_name, "Parsed_scale_666");
        assert_eq!(filter.args, "5:5:flags=+accurate_rnd+bitexact");
        assert_eq!(filter.nb_inputs, 1);
        assert_eq!(filter.nb_outputs, 1);
    }

    #[test]
    fn outputs() {
        let open_inputs = &mut vec![];
        let curr_inputs = &mut vec![];
        let open_outputs = &mut vec![];

        let links = &mut vec![];
        let filter = &mut FilterContext::default();
        let graph = &mut FilterGraph::default();

        let mut p = GraphParser::new("[foo][bar]overlay=5:5[abc]");
        assert!(p.parse_inputs(curr_inputs, open_outputs).is_ok());
        assert_eq!(curr_inputs[0].name, Some(b"foo" as &[u8]));
        assert_eq!(curr_inputs[1].name, Some(b"bar" as &[u8]));

        assert!(p.parse_filter(666, filter, graph).is_ok());
        assert_eq!(filter.index, 666);
        assert_eq!(filter.filt_name, "overlay");
        assert_eq!(filter.inst_name, "Parsed_overlay_666");
        assert_eq!(filter.args, "5:5");
        assert_eq!(filter.nb_inputs, 2);
        assert_eq!(filter.nb_outputs, 1);

        assert!(
            GraphParser::link_filter_inouts(666, links, filter, curr_inputs, open_inputs).is_ok()
        );

        assert!(p
            .parse_outputs(666, links, curr_inputs, open_inputs, open_outputs)
            .is_ok());
        assert!(curr_inputs.is_empty());
        assert_eq!(open_inputs[0].name, Some(b"foo" as &[u8]));
        assert_eq!(open_inputs[1].name, Some(b"bar" as &[u8]));
        assert_eq!(open_outputs[0].name, Some(b"abc" as &[u8]));
    }

    #[test]
    fn good_filtergraph() {
        assert!(avfilter_graph_parse2(
            "split [main][tmp]; [tmp] crop=iw:ih/2:0:0, vflip [flip]; [main][flip] overlay=0:H/2",
        )
        .is_ok());

        assert!(avfilter_graph_parse2("[foo]split [main][tmp]; [tmp] crop=iw:ih/2:0:0, vflip [flip]; [main][flip] overlay=0:H/2[bar]").is_ok());

        // https://stackoverflow.com/questions/55455922/ffmpeg-using-video-filter-with-complex-filter
        assert!(avfilter_graph_parse2(
            "
            [0]crop = \
                w = in_w-2*150 : \
                h = in_h \
                [a] ;
            [a]pad = \
                width = 980 : \
                height = 980 : \
                x = 0 :
                y = 0 :
                color = black
                [b] ;
            [b]subtitles = 
                filename = subtitles.ass
                [c] ;
            [c][1]overlay = \
                x = 0 :
                y = 0
            "
        )
        .is_ok());

        // https://superuser.com/questions/781875/ffmpeg-error-vf-af-filter-and-filter-complex-cannot-be-used-together
        assert!(
            avfilter_graph_parse2("[0:v]scale=854:-2[scaled]; [scaled][1:v]overlay=5:5[out]")
                .is_ok()
        );

        // https://github.com/Vincit/ffmpeg/blob/master/tests/fate/ffmpeg.mak
        assert!(avfilter_graph_parse2(
            "sws_flags=+accurate_rnd+bitexact;[0:0]scale=720:480[v];[v][1:0]overlay[v2]"
        )
        .is_ok());

        // https://stackoverflow.com/questions/13390714/superimposing-two-videos-onto-a-static-image/13405214#13405214
        assert!(avfilter_graph_parse2(
            "[1:v]scale=(iw/2)-20:-1[a]; \
            [2:v]scale=(iw/2)-20:-1[b]; \
            [0:v][a]overlay=10:(main_h/2)-(overlay_h/2):shortest=1[c]; \
            [c][b]overlay=main_w-overlay_w-10:(main_h/2)-(overlay_h/2)[video]"
        )
        .is_ok());

        // https://trac.ffmpeg.org/wiki/FilteringGuide#Examples
        assert!(avfilter_graph_parse2(
            "[0:v]pad=iw*2:ih*2[a]; \
            [1:v]negate[b]; \
            [2:v]hflip[c]; \
            [3:v]edgedetect[d]; \
            [a][b]overlay=w[x]; \
            [x][c]overlay=0:h[y]; \
            [y][d]overlay=w:h[out]"
        )
        .is_ok());

        // https://trac.ffmpeg.org/wiki/FilteringGuide#Examples
        assert!(avfilter_graph_parse2(
            "[1:v]negate[a]; \
            [2:v]hflip[b]; \
            [3:v]edgedetect[c]; \
            [0:v][a]hstack=inputs=2[top]; \
            [b][c]hstack=inputs=2[bottom]; \
            [top][bottom]vstack=inputs=2[out]"
        )
        .is_ok());

        // https://superuser.com/questions/977743/ffmpeg-possible-to-apply-filter-to-only-part-of-a-video-file-while-transcoding
        assert!(avfilter_graph_parse2(
            "[0:v]trim=start=0:duration=90[a];[0:v]trim=start=90:duration=30,setpts=PTS-STARTPTS[b];[b]hflip[c];[a][c]concat[d];[0:v]trim=start=120:duration=60,setpts=PTS-STARTPTS[e];[d][e]concat[out1]"
        ).is_ok());
    }

    #[test]
    fn bad_filtergraph() {
        // https://askubuntu.com/a/268278
        // outdated filtergraph where there are too many inputs specified for the "setpts" filter
        assert!(avfilter_graph_parse2(
            "[0:v][1:v]setpts=PTS-STARTPTS,overlay=20:40[bg]; \
            [bg][2:v]setpts=PTS-STARTPTS,overlay=(W-w)/2:(H-h)/2[v]; \
            [1:a][2:a]amerge=inputs=2[a]"
        )
        .is_err());

        // https://askubuntu.com/a/741206
        assert!(avfilter_graph_parse2(
            "movie=wlogo.png [watermark]; [in][watermark] overlay=main_w-overlay_w-10:10 [out]"
        )
        .is_err());
    }
}
