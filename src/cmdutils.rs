use bitflags::bitflags;
use libc::c_void;
use log::{debug, error, info};
use once_cell::sync::Lazy;
use rusty_ffmpeg::{
    avutil::{avutils::*, error::*},
    ffi,
};

use std::{
    collections::HashMap,
    default,
    ffi::{CStr, CString},
    fmt, marker, mem, ptr, slice,
    sync::Mutex,
};

use crate::ffmpeg::OptionsContext;

enum OptGroup {
    GroupOutfile = 0,
    GroupInfile = 1,
}

bitflags! {
    #[derive(Default)]
    pub struct OptionFlag: u64 {
        const NONE          = 0x0000;
        const HAS_ARG       = 0x0001;
        const OPT_BOOL      = 0x0002;
        const OPT_EXPERT    = 0x0004;
        const OPT_STRING    = 0x0008;
        const OPT_VIDEO     = 0x0010;
        const OPT_AUDIO     = 0x0020;
        const OPT_INT       = 0x0080;
        const OPT_FLOAT     = 0x0100;
        const OPT_SUBTITLE  = 0x0200;
        const OPT_INT64     = 0x0400;
        const OPT_EXIT      = 0x0800;
        const OPT_DATA      = 0x1000;
        const OPT_PERFILE   = 0x2000;
        const OPT_OFFSET    = 0x4000;
        const OPT_SPEC      = 0x8000;
        const OPT_TIME      = 0x10000;
        const OPT_DOUBLE    = 0x20000;
        const OPT_INPUT     = 0x40000;
        const OPT_OUTPUT    = 0x80000;
    }
}

static mut format_opts: *mut ffi::AVDictionary = ptr::null_mut();
static mut codec_opts: *mut ffi::AVDictionary = ptr::null_mut();
static mut sws_dict: *mut ffi::AVDictionary = ptr::null_mut();
static mut swr_opts: *mut ffi::AVDictionary = ptr::null_mut();
static mut resample_opts: *mut ffi::AVDictionary = ptr::null_mut();

pub union OptionOperation {
    pub dst_ptr: *mut c_void,
    pub func_arg: fn(*mut c_void, &str, &str) -> i64,
    pub off: usize,
}

impl fmt::Debug for OptionOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("(Union)OptionOperation")
            .field("val", unsafe { &self.off })
            .finish()
    }
}

impl default::Default for OptionOperation {
    fn default() -> Self {
        OptionOperation { off: 0 }
    }
}

#[derive(Debug, Default)]
pub struct OptionDef<'a> {
    pub name: &'a str,
    pub help: &'a str,
    pub argname: Option<&'a str>,
    pub flags: OptionFlag,
    pub u: OptionOperation,
}

/// Though OptionOperation contains pointer, we still need it to impl Send and
/// Sync, we can ensure its safety.
unsafe impl<'a> marker::Send for OptionDef<'a> {}

/// Though OptionOperation contains pointer, we still need it to impl Send and
/// Sync, we can ensure its safety.
unsafe impl<'a> marker::Sync for OptionDef<'a> {}

/// Currently move the flags out of the struct.
#[derive(Debug, Default)]
pub struct OptionGroupDef<'global> {
    pub name: &'global str,
    pub sep: Option<&'global str>,
    pub flags: OptionFlag,
}

/// Original name is `Option` in FFmpeg, but it's a wide-use type in Rust.
/// So I rename it to `OptionKV`.
#[derive(Debug, Clone)]
pub struct OptionKV<'global> {
    pub opt: &'global OptionDef<'global>,
    pub key: String,
    pub val: String,
}

// TODO maybe split the lifetime here
#[derive(Debug, Clone)]
pub struct OptionGroup<'global> {
    pub group_def: &'global OptionGroupDef<'global>,
    pub arg: String,
    pub opts: Vec<OptionKV<'global>>,
    pub codec_opts: *mut ffi::AVDictionary,
    pub format_opts: *mut ffi::AVDictionary,
    pub resample_opts: *mut ffi::AVDictionary,
    pub sws_dict: *mut ffi::AVDictionary,
    pub swr_opts: *mut ffi::AVDictionary,
}

impl<'global> OptionGroup<'global> {
    pub fn new_global() -> Self {
        static GLOBAL_GROUP: OptionGroupDef = OptionGroupDef {
            name: "global",
            sep: None,
            flags: OptionFlag::NONE,
        };
        OptionGroup {
            group_def: &GLOBAL_GROUP,
            arg: String::new(),
            opts: vec![],
            codec_opts: ptr::null_mut(),
            format_opts: ptr::null_mut(),
            resample_opts: ptr::null_mut(),
            sws_dict: ptr::null_mut(),
            swr_opts: ptr::null_mut(),
        }
    }

    /// This function is specially used for cur_group before it's
    /// refactored into tuple.
    pub fn new_anonymous() -> Self {
        static NEVER_USE_GROUP: OptionGroupDef = OptionGroupDef {
            name: "never_used",
            sep: None,
            flags: OptionFlag::NONE,
        };
        OptionGroup {
            group_def: &NEVER_USE_GROUP,
            arg: String::new(),
            opts: vec![],
            codec_opts: ptr::null_mut(),
            format_opts: ptr::null_mut(),
            resample_opts: ptr::null_mut(),
            sws_dict: ptr::null_mut(),
            swr_opts: ptr::null_mut(),
        }
    }
}

/// A list of option groups that all have the same group type
/// (e.g. input files or output files)
#[derive(Debug)]
pub struct OptionGroupList<'global> {
    pub group_def: &'global OptionGroupDef<'global>,
    pub groups: Vec<OptionGroup<'global>>,
}

#[derive(Debug)]
pub struct OptionParseContext<'global> {
    /// Global options
    pub global_opts: OptionGroup<'global>,
    /// Options that can find a OptionGroupDef
    pub groups: Vec<OptionGroupList<'global>>,
    /// Parsing state
    /// Attention: The group_def in the cur_group has never been used, so we just
    /// use create a placeholder. More attractive option is changing the
    /// cur_group from OptionGroup to tuple (arg: String, opts: Vec<OptionKV>).
    pub cur_group: OptionGroup<'global>,
}

pub union SpecifierOptValue {
    pub str: *mut u8,
    pub i: isize,
    pub i64: i64,
    pub ui64: u64,
    pub f: f32,
    pub dbl: f64,
}

impl fmt::Debug for SpecifierOptValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("(Union)SpecifierOptValue")
            .field("val", unsafe { &self.i })
            .finish()
    }
}

impl default::Default for SpecifierOptValue {
    fn default() -> Self {
        SpecifierOptValue { i: 0 }
    }
}

#[derive(Debug, Default)]
pub struct SpecifierOpt {
    pub specifier: String,
    pub u: SpecifierOptValue,
}

/// This function accepts moved Option value with the OptionsContext it references to unchanged.
pub fn parse_optgroup<'ctxt>(
    mut optctx: Option<&mut OptionsContext>,
    g: &OptionGroup,
) -> Result<(), ()> {
    debug!(
        "Parsing a group of options: {} {}.",
        g.group_def.name, g.arg
    );
    for o in g.opts.iter() {
        if !g.group_def.flags.is_empty() && !g.group_def.flags.intersects(o.opt.flags) {
            error!(
                "Option {} ({}) cannot be applied to \
                   {} {} -- you are trying to apply an input option to an \
                   output file or vice versa. Move this option before the \
                   file it belongs to.",
                o.key, o.opt.help, g.group_def.name, g.arg
            );
            return Err(());
        }
        debug!(
            "Applying option {} ({}) with argument {}.",
            o.key, o.opt.help, o.val
        );
        write_option(&mut optctx, o.opt, &o.key, &o.val)?
    }
    debug!("Successfully parsed a group of options.");
    Ok(())
}

/// `context` is the `opt`, `num_str` is usually the `arg`
pub fn parse_number(
    context: &str,
    numstr: &str,
    num_type: OptionFlag,
    min: f64,
    max: f64,
) -> Result<f64, String> {
    let numstr_ptr = CString::new(numstr).unwrap().as_ptr();
    let mut tail: *mut libc::c_char = ptr::null_mut();
    let d = unsafe { ffi::av_strtod(numstr_ptr, &mut tail) };
    let error = if tail.is_null() {
        format!("Expected number for {} but found: {}", context, numstr)
    } else {
        if d < min || d > max {
            format!(
                "The value for {} was {} which is not within {} - {}",
                context, numstr, min, max
            )
        } else if num_type == OptionFlag::OPT_INT64 && d as i64 as f64 != d {
            format!("Expected int64 for {} but found {}", context, numstr)
        } else if num_type == OptionFlag::OPT_INT && d as isize as f64 != d {
            format!("Expected int for {} but found {}", context, numstr)
        } else {
            return Ok(d);
        }
    };
    Err(error)
}

fn parse_time(context: &str, timestr: &str, is_duration: bool) -> Result<i64, String> {
    let mut us = 0;
    let timestr_ptr = CString::new(timestr).unwrap().as_ptr();
    if unsafe { ffi::av_parse_time(&mut us, timestr_ptr, if is_duration { 1 } else { 0 }) } > 0 {
        Err(format!(
            "Invalid {} specification for {}: {}",
            if is_duration { "duration" } else { "date" },
            context,
            timestr
        ))
    } else {
        Ok(us)
    }
}

/// If failed, panic with some description.
/// TODO: change this function to return corresponding Result later
fn write_option(
    optctx: &mut Option<&mut OptionsContext>,
    po: &OptionDef,
    opt: &str,
    arg: &str,
) -> Result<(), ()> {
    let dst: *mut c_void = if po
        .flags
        .intersects(OptionFlag::OPT_OFFSET | OptionFlag::OPT_SPEC)
    {
        if let &mut Some(ref mut optctx) = optctx {
            *optctx as *mut _ as *mut c_void
        } else {
            panic!("some option contains OPT_OFFSET or OPT_SPEC but in global_opts")
        }
    } else {
        unsafe { po.u.dst_ptr }
    };

    if po.flags.contains(OptionFlag::OPT_SPEC) {
        let so = dst as *mut Vec<SpecifierOpt>;
        let so = unsafe { so.as_mut() }.unwrap();
        let s = opt.find(':').map_or("", |i| &opt[i + 1..]);
        so.push(SpecifierOpt {
            specifier: s.to_owned(),
            u: Default::default(),
        });
    }

    if po.flags.contains(OptionFlag::OPT_STRING) {
        let dst = dst as *mut String;
        let dst = unsafe { dst.as_mut() }.unwrap();
        *dst = arg.to_owned();
    } else if po
        .flags
        .intersects(OptionFlag::OPT_STRING | OptionFlag::OPT_INT)
    {
        let dst = dst as *mut isize;
        let dst = unsafe { dst.as_mut() }.unwrap();
        // IMPROVEMENT FFmpeg uses i32::{MIN, MAX} here but it's int though many
        // c compiler still treat int as 32bit, but I think for Rust age, we
        // need to change it.
        *dst = parse_number(
            opt,
            arg,
            OptionFlag::OPT_INT64,
            isize::MIN as f64,
            isize::MAX as f64,
        )
        .unwrap() as isize;
    } else if po.flags.contains(OptionFlag::OPT_INT64) {
        let dst = dst as *mut i64;
        let dst = unsafe { dst.as_mut() }.unwrap();
        *dst = parse_number(
            opt,
            arg,
            OptionFlag::OPT_INT64,
            i64::MIN as f64,
            i64::MAX as f64,
        )
        .unwrap() as i64;
    } else if po.flags.contains(OptionFlag::OPT_TIME) {
        let dst = dst as *mut i64;
        let dst = unsafe { dst.as_mut() }.unwrap();
        *dst = parse_time(opt, arg, true).unwrap();
    } else if po.flags.contains(OptionFlag::OPT_FLOAT) {
        let dst = dst as *mut f32;
        let dst = unsafe { dst.as_mut() }.unwrap();
        *dst = parse_number(
            opt,
            arg,
            OptionFlag::OPT_INT64,
            i64::MIN as f64,
            i64::MAX as f64,
        )
        .unwrap() as f32;
    } else if po.flags.contains(OptionFlag::OPT_DOUBLE) {
        let dst = dst as *mut f64;
        let dst = unsafe { dst.as_mut() }.unwrap();
        *dst = parse_number(
            opt,
            arg,
            OptionFlag::OPT_INT64,
            i64::MIN as f64,
            i64::MAX as f64,
        )
        .unwrap();
    } else if unsafe { po.u.off } != 0 {
        let optctx = if let &mut Some(ref mut optctx) = optctx {
            *optctx as *mut _ as *mut c_void
        } else {
            ptr::null_mut()
        };
        let func = unsafe { po.u.func_arg };
        let ret = func(optctx, opt, arg);
        // TODO av_err2str() still haven't been implemented
        if ret < 0 {
            error!(
                "Failed to set value '{}' for option '{}': {}",
                arg, opt, "av_err2str()"
            );
            return Err(());
        }
    }
    if po.flags.contains(OptionFlag::OPT_EXIT) {
        panic!("exit as required");
    }
    Ok(())
}

enum ArgOperation {
    /// opt arg
    AddOpt(String, String),
    /// group_idx opt
    FinishGroup(usize, String),
    /// opt arg
    OptDefault(String, String),
}

// TODO the Err in returned Result need to be a ERROR enum
pub fn split_commandline<'ctxt, 'global>(
    octx: &'ctxt mut OptionParseContext<'global>,
    args: &[String],
    options: &'global [OptionDef],
    groups: &'global [OptionGroupDef],
    filtergraph: &mut Option<String>,
) -> Result<(), ()> {
    let (argc, argv) = (args.len(), args);

    let mut operations = vec![];

    // The init_parse_context is moved outside.

    debug!("Splitting the commandline.");

    let mut optindex = 1;
    let mut dashdash = None;

    while optindex < argc {
        let opt = &argv[optindex];
        optindex += 1;

        debug!("Reading option '{}' ...", opt);

        if opt == "--" {
            dashdash = Some(optindex);
            continue;
        }

        // unnamed group separators, e.g. output filename
        if !opt.starts_with('-') || opt.len() <= 1 || dashdash == Some(optindex - 1) {
            // IMPROVEMENT original FFmpeg uses 0 rather than enum value here,
            // we can use the enum value since we know we are using FFmpeg.
            let out_index = OptGroup::GroupOutfile as usize;
            finish_group(octx, out_index, opt);
            operations.push(ArgOperation::FinishGroup(out_index, opt.into()));
            debug!(" matched as {}.", groups[out_index].name);
            continue;
        }

        // Jump over prefix `-`
        let opt = &opt[1..];

        // Named group separators, e.g. -i
        if let Some(group_idx) = match_group_separator(groups, opt) {
            let arg = match argv.get(optindex) {
                Some(arg) => arg,
                None => return Err(()),
            };
            optindex += 1;

            finish_group(octx, group_idx, arg);
            operations.push(ArgOperation::FinishGroup(group_idx, arg.into()));
            debug!(
                " matched as {} with argument '{}'.",
                groups[group_idx].name, arg
            );
            continue;
        }

        // Normal options
        if let Some(po) = find_option(options, opt) {
            let arg = if po.flags.intersects(OptionFlag::OPT_EXIT) {
                // Optional argument, e.g. -h

                // Yes, we cannot use unwrap_or() here because a coercion needed.
                let arg = match argv.get(optindex) {
                    Some(x) => x,
                    None => "",
                };
                optindex += 1;
                arg
            } else if po.flags.intersects(OptionFlag::HAS_ARG) {
                let arg = match argv.get(optindex) {
                    Some(x) => x,
                    None => return Err(()),
                };
                optindex += 1;
                arg
            } else {
                "1"
            };

            // match vf af filter_complex, For presentation purpose
            match opt {
                "vf" | "af" | "filter_complex" => *filtergraph = Some(arg.to_string()),
                _ => {}
            }

            add_opt(octx, po, opt, arg);
            operations.push(ArgOperation::AddOpt(opt.into(), arg.into()));
            debug!(
                " matched as option '{}' ({}) with argument '{:?}'.",
                po.name, po.help, arg
            );
            continue;
        }

        // AVOptions
        if let Some(arg) = argv.get(optindex) {
            // Process common options and process AVOption by the way(the
            // function name is not that self-explaining), **where some global
            // option directory is fulfilled**(this is extremely weird for me to
            // understand).
            let ret = opt_default(ptr::null_mut(), opt, arg);
            if ret >= 0 {
                // We can put it here because currently opt_default() only
                // returns 0 or AVERROR_OPTION_NOT_FOUND.
                operations.push(ArgOperation::OptDefault(opt.into(), arg.into()));
                debug!(" matched as AVOption '{}' with argument '{}'.", opt, arg);
                optindex += 1;
                continue;
            } else if ret != AVERROR_OPTION_NOT_FOUND {
                error!("Error parsing option '{}' with argument '{}'.\n", opt, arg);
                return Err(());
            }
        }

        // boolean -nofoo options
        if opt.starts_with("no") {
            if let Some(po) = find_option(options, &opt[2..]) {
                if po.flags.contains(OptionFlag::OPT_BOOL) {
                    add_opt(octx, po, opt, "0");
                    operations.push(ArgOperation::AddOpt(opt[2..].into(), opt.into()));
                    debug!(
                        " matched as option '{}' ({}) with argument 0.",
                        po.name, po.help
                    );
                    continue;
                }
            }
        }

        error!("Unrecognized option '{}'.", opt);
        return Err(());
    }

    if !octx.cur_group.opts.is_empty()
        || unsafe { !codec_opts.is_null() }
        || unsafe { !format_opts.is_null() }
        || unsafe { !resample_opts.is_null() }
    {
        debug!("Trailing option(s) found in the command: may be ignored.");
    }

    debug!("Finished splitting the commandline.");

    let operation_serialzation = |operation: &ArgOperation| match operation {
        ArgOperation::AddOpt(opt, arg) => {
            println!(
                r#"
add_opt(octx, find_option(options, "{}"), "{}", "{}");
"#,
                opt, opt, arg
            );
        }
        ArgOperation::FinishGroup(group_idx, opt) => {
            println!(
                r#"
finish_group(octx, {}, "{}");
"#,
                group_idx, opt
            );
        }
        ArgOperation::OptDefault(opt, arg) => {
            println!(
                r#"
opt_default(NULL, "{}", "{}");
"#,
                opt, arg
            );
        }
    };

    for operation in operations.iter() {
        operation_serialzation(operation);
    }

    Ok(())
}

fn opt_default(_: *mut c_void, opt: &str, arg: &str) -> i32 {
    if opt == "debug" || opt == "fdebug" {
        // TODO implement equivalent function of av_log_set_level()
        info!("debug is the default");
    }
    let opt_stripped = CString::new(opt.split(':').next().unwrap()).unwrap();
    // This is unicode-safe because it's only used when first char is ascii.
    let opt_nohead = opt.get(1..).map(|x| CString::new(x).unwrap());

    let opt_c = CString::new(opt).unwrap();
    let arg_c = CString::new(arg).unwrap();

    let (opt_ptr, arg_ptr) = (opt_c.as_ptr(), arg_c.as_ptr());

    let mut cc = unsafe { ffi::avcodec_get_class() };
    let mut fc = unsafe { ffi::avformat_get_class() };
    /* Currently not supported, they seems to be used less often.
    let sc = sws_get_class();
    let swr_class = swr_get_class();
    */

    let mut consumed = false;

    let mut o;
    if {
        o = opt_find(
            &mut cc as *mut _ as *mut c_void,
            opt_stripped.as_ptr(),
            ptr::null(),
            0,
            ffi::AV_OPT_SEARCH_CHILDREN | ffi::AV_OPT_SEARCH_FAKE_OBJ,
        );
        !o.is_null()
    } || ((opt.starts_with('v') || opt.starts_with('a') || opt.starts_with('s')) && {
        o = opt_find(
            &mut cc as *mut _ as *mut c_void,
            opt_nohead.unwrap().as_ptr(),
            ptr::null(),
            0,
            ffi::AV_OPT_SEARCH_FAKE_OBJ,
        );
        !o.is_null()
    }) {
        // Shouldn't be null, so unwrap.
        let o = unsafe { o.as_ref() }.unwrap();
        let flags = if o.type_ == ffi::AVOptionType_AV_OPT_TYPE_FLAGS
            && (arg.starts_with('-') || arg.starts_with('+'))
        {
            ffi::AV_DICT_APPEND
        } else {
            0
        };
        unsafe { ffi::av_dict_set(&mut codec_opts as *mut _, opt_ptr, arg_ptr, flags as _) };
        consumed = true;
    }
    let o = opt_find(
        &mut fc as *mut _ as *mut c_void,
        opt_ptr,
        ptr::null(),
        0,
        ffi::AV_OPT_SEARCH_CHILDREN | ffi::AV_OPT_SEARCH_FAKE_OBJ,
    );
    if let Some(o) = unsafe { o.as_ref() } {
        let flags = if o.type_ == ffi::AVOptionType_AV_OPT_TYPE_FLAGS
            && (arg.starts_with('-') || arg.starts_with('+'))
        {
            ffi::AV_DICT_APPEND
        } else {
            0
        };
        unsafe { ffi::av_dict_set(&mut format_opts as *mut _, opt_ptr, arg_ptr, flags as _) };
        consumed = true;
    }

    // TODO: init things about SWRESAMPLE SWSCALE

    if consumed {
        0
    } else {
        AVERROR_OPTION_NOT_FOUND
    }
}

/// Whether a valid option is found.
fn opt_find(
    obj: *mut c_void,
    name: *const libc::c_char,
    unit: *const libc::c_char,
    opt_flags: u32,
    search_flags: u32,
) -> *const ffi::AVOption {
    let o = unsafe { ffi::av_opt_find(obj, name, unit, opt_flags as i32, search_flags as i32) };
    if o.is_null() {
        ptr::null()
    } else if unsafe { o.as_ref() }.unwrap().flags == 0 {
        ptr::null()
    } else {
        o
    }
}

fn match_group_separator(groups: &[OptionGroupDef], opt: &str) -> Option<usize> {
    groups
        .iter()
        .enumerate()
        .find_map(|(i, optdef)| Some(i).filter(|_| optdef.sep == Some(opt)))
}

/// Finish parsing an option group. Move current parsing group into specific group list
/// # Parameters
/// `group_idx`     which group definition should this group belong to
/// `arg`           argument of the group delimiting option
fn finish_group(octx: &mut OptionParseContext, group_idx: usize, arg: &str) {
    let mut new_group = octx.cur_group.clone();
    new_group.arg = arg.to_owned();
    new_group.group_def = octx.groups[group_idx].group_def;
    unsafe {
        new_group.sws_dict = sws_dict;
        new_group.swr_opts = swr_opts;
        new_group.codec_opts = codec_opts;
        new_group.format_opts = format_opts;
        new_group.resample_opts = resample_opts;
    }

    octx.groups[group_idx].groups.push(new_group);

    unsafe {
        codec_opts = ptr::null_mut();
        format_opts = ptr::null_mut();
        resample_opts = ptr::null_mut();
        sws_dict = ptr::null_mut();
        swr_opts = ptr::null_mut();
    }
    init_opts();

    octx.cur_group = OptionGroup::new_anonymous();
}

fn init_opts() {
    let flags = CString::new("flags").unwrap();
    let bicubic = CString::new("bicubic").unwrap();
    unsafe { ffi::av_dict_set(&mut sws_dict as *mut _, flags.as_ptr(), bicubic.as_ptr(), 0) };
}

fn uninit_opts() {
    unsafe {
        ffi::av_dict_free(&mut swr_opts as *mut _);
        ffi::av_dict_free(&mut sws_dict as *mut _);
        ffi::av_dict_free(&mut format_opts as *mut _);
        ffi::av_dict_free(&mut codec_opts as *mut _);
        ffi::av_dict_free(&mut resample_opts as *mut _);
    }
}

fn find_option<'global>(
    options: &'global [OptionDef<'global>],
    name: &str,
) -> Option<&'global OptionDef<'global>> {
    let name = name.split(':').next()?;
    options.iter().find(|&option_def| option_def.name == name)
}

/// Add an option instance to currently parsed group.
fn add_opt<'ctxt, 'global>(
    octx: &'ctxt mut OptionParseContext<'global>,
    opt: &'global OptionDef<'global>,
    key: &str,
    val: &str,
) {
    let global = !opt
        .flags
        .intersects(OptionFlag::OPT_PERFILE | OptionFlag::OPT_SPEC | OptionFlag::OPT_OFFSET);
    let g = if global {
        // Here we can ensure that global_opts's flags doesn't contains either OPT_SPEC or OPT_OFFSET
        &mut octx.global_opts
    } else {
        &mut octx.cur_group
    };
    g.opts.push(OptionKV {
        opt: opt,
        key: key.to_owned(),
        val: val.to_owned(),
    })
}

pub fn init_parse_context<'global>(
    groups: &'static [OptionGroupDef<'global>],
) -> OptionParseContext<'global> {
    OptionParseContext {
        groups: groups
            .iter()
            .map(|group| OptionGroupList {
                group_def: group,
                groups: vec![],
            })
            .collect(),
        global_opts: OptionGroup::new_global(),
        cur_group: OptionGroup::new_anonymous(),
    }
}

pub fn uninit_parse_context(octx: &mut OptionParseContext) {
    octx.groups.iter_mut().for_each(|list| {
        list.groups.iter_mut().for_each(|group| unsafe {
            ffi::av_dict_free(&mut group.codec_opts as *mut _);
            ffi::av_dict_free(&mut group.format_opts as *mut _);
            ffi::av_dict_free(&mut group.resample_opts as *mut _);
            ffi::av_dict_free(&mut group.sws_dict as *mut _);
            ffi::av_dict_free(&mut group.swr_opts as *mut _);
        })
    });
    uninit_opts();
}

#[cfg(test)]
mod types_tests {
    use super::*;

    #[test]
    fn fmt_debug_option_operation_default() {
        let optop: OptionOperation = Default::default();
        assert_eq!(format!("{:?}", optop), "(Union)OptionOperation { val: 0 }");
    }

    #[test]
    fn fmt_debug_option_operation() {
        let optop: OptionOperation = OptionOperation { off: 123_456 };
        assert_eq!(
            format!("{:?}", optop),
            "(Union)OptionOperation { val: 123456 }"
        );
    }
}
