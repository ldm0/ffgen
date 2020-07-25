// This is need for those global values in FFmpeg
#![allow(non_upper_case_globals)]
// This will be finally removed, but in development stage it's useful
#![allow(unused_variables)]
use libc::{c_char, c_void};
use memoffset::offset_of;
use once_cell::sync::Lazy;

use crate::{
    cmdutils::{
        OptionDef, OptionFlag, OptionGroup, OptionGroupDef, OptionGroupList, OptionKV,
        OptionOperation, OptionParseContext,
    },
    ffmpeg::OptionsContext,
};

macro_rules! void {
    ($x: expr) => {
        unsafe { &raw mut $x as *mut c_void }
    };
}

macro_rules! option_operation {
    (dst_ptr => $operation: expr) => {
        OptionOperation {
            dst_ptr: void!($operation),
        }
    };
    (func_arg => $operation: expr) => {
        OptionOperation {
            func_arg: $operation,
        }
    };
    (off => $operation: ident) => {
        OptionOperation {
            off: offset_of!(OptionsContext, $operation),
        }
    };
}

macro_rules! option_def {
    // It looks stupid but matching with `$flag: ident $(| $flags: ident)*` early is for disambiguity.
    ($name: literal, $flag: ident $(| $flags: ident)*, dst_ptr => $operation: expr, $help: literal) => {
        option_def! (
            @inner $name, $flag $(| $flags)*,
            option_operation!(dst_ptr => $operation),
            $help, None
        )
    };
    ($name: literal, $flag: ident $(| $flags: ident)*, func_arg => $operation: expr, $help: literal) => {
        option_def! (
            @inner $name, $flag $(| $flags)*,
            option_operation!(func_arg => $operation),
            $help, None
        )
    };
    ($name: literal, $flag: ident $(| $flags: ident)*, off => $operation: ident, $help: literal) => {
        option_def! (
            @inner $name, $flag $(| $flags)*,
            option_operation!(off => $operation),
            $help, None
        )
    };
    ($name: literal, $flag: ident $(| $flags: ident)*, dst_ptr => $operation: expr, $help: literal, $argname: literal) => {
        option_def! (
            @inner $name, $flag $(| $flags)*,
            option_operation!(dst_ptr => $operation),
            $help, Some($argname)
        )
    };
    ($name: literal, $flag: ident $(| $flags: ident)*, func_arg => $operation: expr, $help: literal, $argname: literal) => {
        option_def! (
            @inner $name, $flag $(| $flags)*,
            option_operation!(func_arg => $operation),
            $help, Some($argname)
        )
    };
    ($name: literal, $flag: ident $(| $flags: ident)*, off => $operation: ident, $help: literal, $argname: literal) => {
        option_def! (
            @inner $name, $flag $(| $flags)*,
            option_operation!(off => $operation),
            $help, Some($argname)
        )
    };
    (@inner $name: literal, $flag: ident $(| $flags: ident)*, $u: expr, $help: literal, $argname: expr) => {
        OptionDef {
            name: $name,
            help: $help,
            argname: $argname,
            flags: OptionFlag::$flag $(| OptionFlag::$flags)*,
            u: $u,
        }
    };
}

macro_rules! option_group_def {
    ($name: literal) => {
        option_group_def!(@inner $name, None, OptionFlag::NONE)
    };
    ($name: literal, $flags: expr) => {
        option_group_def!(@inner $name, None, $flags)
    };
    ($name: literal, $separator: literal, $flags: expr) => {
        option_group_def!(@inner $name, Some($separator), $flags)
    };
    (@inner $name: literal, $separator: expr, $flags: expr) => {
        OptionGroupDef {
            name: $name,
            sep: $separator,
            flags: $flags,
        }
    }
}

pub static GROUPS: Lazy<[OptionGroupDef; 2]> = Lazy::new(|| {
    [
        option_group_def!("output url", OptionFlag::OPT_OUTPUT),
        option_group_def!("input url", "i", OptionFlag::OPT_INPUT),
    ]
});

/// The options list is in ffmpeg_opt.c originally, but we move it here for cleanness.
///
/// Steps to create the list from ffmpeg's code:
/// 1. remove all other codes except the `options`
/// 2. remove unnecessary lines like comments and empty line and `#ifdef #ifndef #endif` things
/// 3. `\option_def!("` => `option_def!("`
/// 4. `, *\{ *&` => `, dst_ptr => `
/// 5. `, *\{ .off *= OFFSET\(` => `, off => `
/// 6. `, *\{ .func_arg = ` => `, func_arg => `
/// 7. `\},\n*    option_def!\(` => `),\n    option_def!(`
/// 8. `\) *\},` => ` },`
/// 9. ` *\},\n *` => `, `
/// 10. `\|\n *` => `| `
/// 11. `"\n *"` => `| `
/// 12. then hand tweak inharmonious codes
/// 13. `,? \),` => `),`
pub static OPTIONS: Lazy<[OptionDef; 179]> = Lazy::new(|| {
    [
        // Common options
        option_def!("L",            OPT_EXIT,               func_arg => show_license,     "show license"),
        option_def!("h",            OPT_EXIT,               func_arg => show_help,        "show help", "topic"),
        option_def!("?",            OPT_EXIT,               func_arg => show_help,        "show help", "topic"),
        option_def!("help",         OPT_EXIT,               func_arg => show_help,        "show help", "topic"),
        option_def!("-help",        OPT_EXIT,               func_arg => show_help,        "show help", "topic"),
        option_def!("version",      OPT_EXIT,               func_arg => show_version,     "show version"),
        option_def!("buildconf",    OPT_EXIT,               func_arg => show_buildconf,   "show build configuration"),
        option_def!("formats",      OPT_EXIT,               func_arg => show_formats,     "show available formats"),
        option_def!("muxers",       OPT_EXIT,               func_arg => show_muxers,      "show available muxers"),
        option_def!("demuxers",     OPT_EXIT,               func_arg => show_demuxers,    "show available demuxers"),
        option_def!("devices",      OPT_EXIT,               func_arg => show_devices,     "show available devices"),
        option_def!("codecs",       OPT_EXIT,               func_arg => show_codecs,      "show available codecs"),
        option_def!("decoders",     OPT_EXIT,               func_arg => show_decoders,    "show available decoders"),
        option_def!("encoders",     OPT_EXIT,               func_arg => show_encoders,    "show available encoders"),
        option_def!("bsfs",         OPT_EXIT,               func_arg => show_bsfs,        "show available bit stream filters"),
        option_def!("protocols",    OPT_EXIT,               func_arg => show_protocols,   "show available protocols"),
        option_def!("filters",      OPT_EXIT,               func_arg => show_filters,     "show available filters"),
        option_def!("pix_fmts",     OPT_EXIT,               func_arg => show_pix_fmts,    "show available pixel formats"),
        option_def!("layouts",      OPT_EXIT,               func_arg => show_layouts,     "show standard channel layouts"),
        option_def!("sample_fmts",  OPT_EXIT,               func_arg => show_sample_fmts, "show available audio sample formats"),
        option_def!("colors",       OPT_EXIT,               func_arg => show_colors,      "show available color names"),
        option_def!("loglevel",     HAS_ARG,                func_arg => opt_loglevel,     "set logging level", "loglevel"),
        option_def!("v",            HAS_ARG,                func_arg => opt_loglevel,     "set logging level", "loglevel"),
        option_def!("report",       NONE,                   func_arg => opt_report,       "generate a report"),
        option_def!("max_alloc",    HAS_ARG,                func_arg => opt_max_alloc,    "set maximum size of a single allocated block",   "bytes"),
        option_def!("cpuflags",     HAS_ARG | OPT_EXPERT,   func_arg => opt_cpuflags,       "force specific cpu flags",         "flags"),
        option_def!("hide_banner",  OPT_BOOL | OPT_EXPERT,  dst_ptr => hide_banner,         "do not show program banner",       "hide_banner"),
        option_def!("sources",      OPT_EXIT | HAS_ARG,     func_arg => show_sources,       "list sources of the input device", "device"),
        option_def!("sinks",        OPT_EXIT | HAS_ARG,     func_arg => show_sinks,         "list sinks of the output device",  "device"),
        // FFmpeg main options
        option_def!("f", HAS_ARG | OPT_STRING | OPT_OFFSET | OPT_INPUT | OPT_OUTPUT, off => format, "force format", "fmt"),
        option_def!("y", OPT_BOOL, dst_ptr => file_overwrite, "overwrite output files"),
        option_def!("n", OPT_BOOL, dst_ptr => no_file_overwrite, "never overwrite output files"),
        option_def!("ignore_unknown", OPT_BOOL, dst_ptr => ignore_unknown_streams, "Ignore unknown stream types"),
        option_def!("copy_unknown", OPT_BOOL | OPT_EXPERT, dst_ptr => copy_unknown_streams, "Copy unknown stream types"),
        option_def!("c", HAS_ARG | OPT_STRING | OPT_SPEC | OPT_INPUT | OPT_OUTPUT, off => codec_names, "codec name", "codec"),
        option_def!("codec", HAS_ARG | OPT_STRING | OPT_SPEC | OPT_INPUT | OPT_OUTPUT, off => codec_names, "codec name", "codec"),
        option_def!("pre", HAS_ARG | OPT_STRING | OPT_SPEC | OPT_OUTPUT, off => presets, "preset name", "preset"),
        option_def!("map", HAS_ARG | OPT_EXPERT | OPT_PERFILE | OPT_OUTPUT, func_arg => opt_map, "set input stream mapping", "[-]input_file_id[:stream_specifier][,sync_file_id[:stream_specifier]]"),
        option_def!("map_channel", HAS_ARG | OPT_EXPERT | OPT_PERFILE | OPT_OUTPUT, func_arg => opt_map_channel, "map an audio channel from one stream to another", "file.stream.channel[:syncfile.syncstream]"),
        option_def!("map_metadata", HAS_ARG | OPT_STRING | OPT_SPEC | OPT_OUTPUT, off => metadata_map, "set metadata information of outfile from infile", "outfile[,metadata]:infile[,metadata]"),
        option_def!("map_chapters", HAS_ARG | OPT_INT | OPT_EXPERT | OPT_OFFSET | OPT_OUTPUT, off => chapters_input_file, "set chapters mapping", "input_file_index"),
        option_def!("t", HAS_ARG | OPT_TIME | OPT_OFFSET | OPT_INPUT | OPT_OUTPUT, off => recording_time, "record or transcode \"duration\" seconds of audio/video", "duration"),
        option_def!("to", HAS_ARG | OPT_TIME | OPT_OFFSET | OPT_INPUT | OPT_OUTPUT, off => stop_time, "record or transcode stop time", "time_stop"),
        option_def!("fs", HAS_ARG | OPT_INT64 | OPT_OFFSET | OPT_OUTPUT, off => limit_filesize, "set the limit file size in bytes", "limit_size"),
        option_def!("ss", HAS_ARG | OPT_TIME | OPT_OFFSET | OPT_INPUT | OPT_OUTPUT, off => start_time, "set the start time offset", "time_off"),
        option_def!("sseof", HAS_ARG | OPT_TIME | OPT_OFFSET | OPT_INPUT, off => start_time_eof, "set the start time offset relative to EOF", "time_off"),
        option_def!("seek_timestamp", HAS_ARG | OPT_INT | OPT_OFFSET | OPT_INPUT, off => seek_timestamp, "enable/disable seeking by timestamp with -ss"),
        option_def!("accurate_seek", OPT_BOOL | OPT_OFFSET | OPT_EXPERT | OPT_INPUT, off => accurate_seek, "enable/disable accurate seeking with -ss"),
        option_def!("itsoffset", HAS_ARG | OPT_TIME | OPT_OFFSET | OPT_EXPERT | OPT_INPUT, off => input_ts_offset, "set the input ts offset", "time_off"),
        option_def!("itsscale", HAS_ARG | OPT_DOUBLE | OPT_SPEC | OPT_EXPERT | OPT_INPUT, off => ts_scale, "set the input ts scale", "scale"),
        option_def!("timestamp", HAS_ARG | OPT_PERFILE | OPT_OUTPUT, func_arg => opt_recording_timestamp, "set the recording timestamp ('now' to set the current time)", "time"),
        option_def!("metadata", HAS_ARG | OPT_STRING | OPT_SPEC | OPT_OUTPUT, off => metadata, "add metadata", "string=string"),
        option_def!("program", HAS_ARG | OPT_STRING | OPT_SPEC | OPT_OUTPUT, off => program, "add program with specified streams", "title=string:st=number..."),
        option_def!("dframes", HAS_ARG | OPT_PERFILE | OPT_EXPERT | OPT_OUTPUT, func_arg => opt_data_frames, "set the number of data frames to output", "number"),
        option_def!("benchmark", OPT_BOOL | OPT_EXPERT, dst_ptr => do_benchmark, "add timings for benchmarking"),
        option_def!("benchmark_all", OPT_BOOL | OPT_EXPERT, dst_ptr => do_benchmark_all, "add timings for each task"),
        option_def!("progress", HAS_ARG | OPT_EXPERT, func_arg => opt_progress, "write program-readable progress information", "url"),
        option_def!("stdin", OPT_BOOL | OPT_EXPERT, dst_ptr => stdin_interaction, "enable or disable interaction on standard input"),
        option_def!("timelimit", HAS_ARG | OPT_EXPERT, func_arg => opt_timelimit, "set max runtime in seconds in CPU user time", "limit"),
        option_def!("dump", OPT_BOOL | OPT_EXPERT, dst_ptr => do_pkt_dump, "dump each input packet"),
        option_def!("hex", OPT_BOOL | OPT_EXPERT, dst_ptr => do_hex_dump, "when dumping packets, also dump the payload"),
        option_def!("re", OPT_BOOL | OPT_EXPERT | OPT_OFFSET | OPT_INPUT, off => rate_emu, "read input at native frame rate", ""),
        option_def!("target", HAS_ARG | OPT_PERFILE | OPT_OUTPUT, func_arg => opt_target, "specify target file type (\"vcd\", \"svcd\", \"dvd\", \"dv\" or \"dv50\" | with optional prefixes \"pal-\", \"ntsc-\" or \"film-\")", "type"),
        option_def!("vsync", HAS_ARG | OPT_EXPERT, func_arg => opt_vsync, "video sync method", ""),
        option_def!("frame_drop_threshold", HAS_ARG | OPT_FLOAT | OPT_EXPERT, dst_ptr => frame_drop_threshold, "frame drop threshold", ""),
        option_def!("async", HAS_ARG | OPT_INT | OPT_EXPERT, dst_ptr => audio_sync_method, "audio sync method", ""),
        option_def!("adrift_threshold", HAS_ARG | OPT_FLOAT | OPT_EXPERT, dst_ptr => audio_drift_threshold, "audio drift threshold", "threshold"),
        option_def!("copyts", OPT_BOOL | OPT_EXPERT, dst_ptr => copy_ts, "copy timestamps"),
        option_def!("start_at_zero", OPT_BOOL | OPT_EXPERT, dst_ptr => start_at_zero, "shift input timestamps to start at 0 when using copyts"),
        option_def!("copytb", HAS_ARG | OPT_INT | OPT_EXPERT, dst_ptr => copy_tb, "copy input stream time base when stream copying", "mode"),
        option_def!("shortest", OPT_BOOL | OPT_EXPERT | OPT_OFFSET | OPT_OUTPUT, off => shortest, "finish encoding within shortest input"),
        option_def!("bitexact", OPT_BOOL | OPT_EXPERT | OPT_OFFSET | OPT_OUTPUT | OPT_INPUT, off => bitexact, "bitexact mode"),
        option_def!("apad", OPT_STRING | HAS_ARG | OPT_SPEC | OPT_OUTPUT, off => apad, "audio pad", ""),
        option_def!("dts_delta_threshold", HAS_ARG | OPT_FLOAT | OPT_EXPERT, dst_ptr => dts_delta_threshold, "timestamp discontinuity delta threshold", "threshold"),
        option_def!("dts_error_threshold", HAS_ARG | OPT_FLOAT | OPT_EXPERT, dst_ptr => dts_error_threshold, "timestamp error delta threshold", "threshold"),
        option_def!("xerror", OPT_BOOL | OPT_EXPERT, dst_ptr => exit_on_error, "exit on error", "error"),
        option_def!("abort_on", HAS_ARG | OPT_EXPERT, func_arg => opt_abort_on, "abort on the specified condition flags", "flags"),
        option_def!("copyinkf", OPT_BOOL | OPT_EXPERT | OPT_SPEC | OPT_OUTPUT, off => copy_initial_nonkeyframes, "copy initial non-keyframes"),
        option_def!("copypriorss", OPT_INT | HAS_ARG | OPT_EXPERT | OPT_SPEC | OPT_OUTPUT, off => copy_prior_start, "copy or discard frames before start time"),
        option_def!("frames", OPT_INT64 | HAS_ARG | OPT_SPEC | OPT_OUTPUT, off => max_frames, "set the number of frames to output", "number"),
        option_def!("tag", OPT_STRING | HAS_ARG | OPT_SPEC | OPT_EXPERT | OPT_OUTPUT | OPT_INPUT, off => codec_tags, "force codec tag/fourcc", "fourcc/tag"),
        option_def!("q", HAS_ARG | OPT_EXPERT | OPT_DOUBLE | OPT_SPEC | OPT_OUTPUT, off => qscale, "use fixed quality scale (VBR)", "q"),
        option_def!("qscale", HAS_ARG | OPT_EXPERT | OPT_PERFILE | OPT_OUTPUT, func_arg => opt_qscale, "use fixed quality scale (VBR)", "q"),
        option_def!("profile", HAS_ARG | OPT_EXPERT | OPT_PERFILE | OPT_OUTPUT, func_arg => opt_profile, "set profile", "profile"),
        option_def!("filter", HAS_ARG | OPT_STRING | OPT_SPEC | OPT_OUTPUT, off => filters, "set stream filtergraph", "filter_graph"),
        option_def!("filter_threads", HAS_ARG | OPT_INT, dst_ptr => filter_nbthreads, "number of non-complex filter threads"),
        option_def!("filter_script", HAS_ARG | OPT_STRING | OPT_SPEC | OPT_OUTPUT, off => filter_scripts, "read stream filtergraph description from a file", "filename"),
        option_def!("reinit_filter", HAS_ARG | OPT_INT | OPT_SPEC | OPT_INPUT, off => reinit_filters, "reinit filtergraph on input parameter changes", ""),
        option_def!("filter_complex", HAS_ARG | OPT_EXPERT, func_arg => opt_filter_complex, "create a complex filtergraph", "graph_description"),
        option_def!("filter_complex_threads", HAS_ARG | OPT_INT, dst_ptr => filter_complex_nbthreads, "number of threads for -filter_complex"),
        option_def!("lavfi", HAS_ARG | OPT_EXPERT, func_arg => opt_filter_complex, "create a complex filtergraph", "graph_description"),
        option_def!("filter_complex_script", HAS_ARG | OPT_EXPERT, func_arg => opt_filter_complex_script, "read complex filtergraph description from a file", "filename"),
        option_def!("stats", OPT_BOOL, dst_ptr => print_stats, "print progress report during encoding"),
        option_def!("attach", HAS_ARG | OPT_PERFILE | OPT_EXPERT | OPT_OUTPUT, func_arg => opt_attach, "add an attachment to the output file", "filename"),
        option_def!("dump_attachment", HAS_ARG | OPT_STRING | OPT_SPEC | OPT_EXPERT | OPT_INPUT, off => dump_attachment, "extract an attachment into a file", "filename"),
        option_def!("stream_loop", OPT_INT | HAS_ARG | OPT_EXPERT | OPT_INPUT | OPT_OFFSET, off => loops, "set number of times input stream shall be looped", "loop count"),
        option_def!("debug_ts", OPT_BOOL | OPT_EXPERT, dst_ptr => debug_ts, "print timestamp debugging info"),
        option_def!("max_error_rate", HAS_ARG | OPT_FLOAT, dst_ptr => max_error_rate, "ratio of errors (0.0: no errors, 1.0: 100% errors) above which ffmpeg returns an error instead of success.", "maximum error rate"),
        option_def!("discard", OPT_STRING | HAS_ARG | OPT_SPEC | OPT_INPUT, off => discard, "discard", ""),
        option_def!("disposition", OPT_STRING | HAS_ARG | OPT_SPEC | OPT_OUTPUT, off => disposition, "disposition", ""),
        option_def!("thread_queue_size", HAS_ARG | OPT_INT | OPT_OFFSET | OPT_EXPERT | OPT_INPUT, off => thread_queue_size, "set the maximum number of queued packets from the demuxer"),
        option_def!("find_stream_info", OPT_BOOL | OPT_PERFILE | OPT_INPUT | OPT_EXPERT, dst_ptr => find_stream_info, "read and decode the streams to fill missing information with heuristics"),
        option_def!("vframes", OPT_VIDEO | HAS_ARG  | OPT_PERFILE | OPT_OUTPUT, func_arg => opt_video_frames, "set the number of video frames to output", "number"),
        option_def!("r", OPT_VIDEO | HAS_ARG  | OPT_STRING | OPT_SPEC | OPT_INPUT | OPT_OUTPUT, off => frame_rates, "set frame rate (Hz value, fraction or abbreviation)", "rate"),
        option_def!("s", OPT_VIDEO | HAS_ARG | OPT_SUBTITLE | OPT_STRING | OPT_SPEC | OPT_INPUT | OPT_OUTPUT, off => frame_sizes, "set frame size (WxH or abbreviation)", "size"),
        option_def!("aspect", OPT_VIDEO | HAS_ARG  | OPT_STRING | OPT_SPEC | OPT_OUTPUT, off => frame_aspect_ratios, "set aspect ratio (4:3, 16:9 or 1.3333, 1.7777)", "aspect"),
        option_def!("pix_fmt", OPT_VIDEO | HAS_ARG | OPT_EXPERT  | OPT_STRING | OPT_SPEC | OPT_INPUT | OPT_OUTPUT, off => frame_pix_fmts, "set pixel format", "format"),
        option_def!("bits_per_raw_sample", OPT_VIDEO | OPT_INT | HAS_ARG, dst_ptr => frame_bits_per_raw_sample, "set the number of bits per raw sample", "number"),
        option_def!("intra", OPT_VIDEO | OPT_BOOL | OPT_EXPERT, dst_ptr => intra_only, "deprecated use -g 1"),
        option_def!("vn", OPT_VIDEO | OPT_BOOL  | OPT_OFFSET | OPT_INPUT | OPT_OUTPUT, off => video_disable, "disable video"),
        option_def!("rc_override", OPT_VIDEO | HAS_ARG | OPT_EXPERT  | OPT_STRING | OPT_SPEC | OPT_OUTPUT, off => rc_overrides, "rate control override for specific intervals", "override"),
        option_def!("vcodec", OPT_VIDEO | HAS_ARG  | OPT_PERFILE | OPT_INPUT | OPT_OUTPUT, func_arg => opt_video_codec, "force video codec ('copy' to copy stream)", "codec"),
        option_def!("sameq", OPT_VIDEO | OPT_EXPERT , func_arg => opt_sameq, "Removed"),
        option_def!("same_quant", OPT_VIDEO | OPT_EXPERT , func_arg => opt_sameq, "Removed"),
        option_def!("timecode", OPT_VIDEO | HAS_ARG | OPT_PERFILE | OPT_OUTPUT, func_arg => opt_timecode, "set initial TimeCode value.", "hh:mm:ss[:;.]ff"),
        option_def!("pass", OPT_VIDEO | HAS_ARG | OPT_SPEC | OPT_INT | OPT_OUTPUT, off => pass, "select the pass number (1 to 3)", "n"),
        option_def!("passlogfile", OPT_VIDEO | HAS_ARG | OPT_STRING | OPT_EXPERT | OPT_SPEC | OPT_OUTPUT, off => passlogfiles, "select two pass log file name prefix", "prefix"),
        option_def!("deinterlace", OPT_VIDEO | OPT_BOOL | OPT_EXPERT, dst_ptr => do_deinterlace, "this option is deprecated, use the yadif filter instead"),
        option_def!("psnr", OPT_VIDEO | OPT_BOOL | OPT_EXPERT, dst_ptr => do_psnr, "calculate PSNR of compressed frames"),
        option_def!("vstats", OPT_VIDEO | OPT_EXPERT , func_arg => opt_vstats, "dump video coding statistics to file"),
        option_def!("vstats_file", OPT_VIDEO | HAS_ARG | OPT_EXPERT , func_arg => opt_vstats_file, "dump video coding statistics to file", "file"),
        option_def!("vstats_version", OPT_VIDEO | OPT_INT | HAS_ARG | OPT_EXPERT , dst_ptr => vstats_version, "Version of the vstats format to use."),
        option_def!("vf", OPT_VIDEO | HAS_ARG  | OPT_PERFILE | OPT_OUTPUT, func_arg => opt_video_filters, "set video filters", "filter_graph"),
        option_def!("intra_matrix", OPT_VIDEO | HAS_ARG | OPT_EXPERT  | OPT_STRING | OPT_SPEC | OPT_OUTPUT, off => intra_matrices, "specify intra matrix coeffs", "matrix"),
        option_def!("inter_matrix", OPT_VIDEO | HAS_ARG | OPT_EXPERT  | OPT_STRING | OPT_SPEC | OPT_OUTPUT, off => inter_matrices, "specify inter matrix coeffs", "matrix"),
        option_def!("chroma_intra_matrix", OPT_VIDEO | HAS_ARG | OPT_EXPERT  | OPT_STRING | OPT_SPEC | OPT_OUTPUT, off => chroma_intra_matrices, "specify intra matrix coeffs", "matrix"),
        option_def!("top", OPT_VIDEO | HAS_ARG | OPT_EXPERT  | OPT_INT| OPT_SPEC | OPT_INPUT | OPT_OUTPUT, off => top_field_first, "top=1/bottom=0/auto=-1 field first", ""),
        option_def!("vtag", OPT_VIDEO | HAS_ARG | OPT_EXPERT  | OPT_PERFILE | OPT_INPUT | OPT_OUTPUT, func_arg => opt_old2new, "force video tag/fourcc", "fourcc/tag"),
        option_def!("qphist", OPT_VIDEO | OPT_BOOL | OPT_EXPERT , dst_ptr => qp_hist, "show QP histogram"),
        option_def!("force_fps", OPT_VIDEO | OPT_BOOL | OPT_EXPERT  | OPT_SPEC | OPT_OUTPUT, off => force_fps, "force the selected framerate, disable the best supported framerate selection"),
        option_def!("streamid", OPT_VIDEO | HAS_ARG | OPT_EXPERT | OPT_PERFILE | OPT_OUTPUT, func_arg => opt_streamid, "set the value of an outfile streamid", "streamIndex:value"),
        option_def!("force_key_frames", OPT_VIDEO | OPT_STRING | HAS_ARG | OPT_EXPERT | OPT_SPEC | OPT_OUTPUT, off => forced_key_frames, "force key frames at specified timestamps", "timestamps"),
        option_def!("ab", OPT_VIDEO | HAS_ARG | OPT_PERFILE | OPT_OUTPUT, func_arg => opt_bitrate, "audio bitrate (please use -b:a)", "bitrate"),
        option_def!("b", OPT_VIDEO | HAS_ARG | OPT_PERFILE | OPT_OUTPUT, func_arg => opt_bitrate, "video bitrate (please use -b:v)", "bitrate"),
        option_def!("hwaccel", OPT_VIDEO | OPT_STRING | HAS_ARG | OPT_EXPERT | OPT_SPEC | OPT_INPUT, off => hwaccels, "use HW accelerated decoding", "hwaccel name"),
        option_def!("hwaccel_device", OPT_VIDEO | OPT_STRING | HAS_ARG | OPT_EXPERT | OPT_SPEC | OPT_INPUT, off => hwaccel_devices, "select a device for HW acceleration", "devicename"),
        option_def!("hwaccel_output_format", OPT_VIDEO | OPT_STRING | HAS_ARG | OPT_EXPERT | OPT_SPEC | OPT_INPUT, off => hwaccel_output_formats, "select output format used with HW accelerated decoding", "format"),
        option_def!("videotoolbox_pixfmt", HAS_ARG | OPT_STRING | OPT_EXPERT, dst_ptr => videotoolbox_pixfmt, ""),
        option_def!("hwaccels", OPT_EXIT, func_arg => show_hwaccels, "show available HW acceleration methods"),
        option_def!("autorotate", HAS_ARG | OPT_BOOL | OPT_SPEC | OPT_EXPERT | OPT_INPUT, off => autorotate, "automatically insert correct rotate filters"),
        option_def!("aframes", OPT_AUDIO | HAS_ARG  | OPT_PERFILE | OPT_OUTPUT, func_arg => opt_audio_frames, "set the number of audio frames to output", "number"),
        option_def!("aq", OPT_AUDIO | HAS_ARG  | OPT_PERFILE | OPT_OUTPUT, func_arg => opt_audio_qscale, "set audio quality (codec-specific)", "quality"),
        option_def!("ar", OPT_AUDIO | HAS_ARG  | OPT_INT | OPT_SPEC | OPT_INPUT | OPT_OUTPUT, off => audio_sample_rate, "set audio sampling rate (in Hz)", "rate"),
        option_def!("ac", OPT_AUDIO | HAS_ARG  | OPT_INT | OPT_SPEC | OPT_INPUT | OPT_OUTPUT, off => audio_channels, "set number of audio channels", "channels"),
        option_def!("an", OPT_AUDIO | OPT_BOOL | OPT_OFFSET | OPT_INPUT | OPT_OUTPUT, off => audio_disable, "disable audio"),
        option_def!("acodec", OPT_AUDIO | HAS_ARG  | OPT_PERFILE | OPT_INPUT | OPT_OUTPUT, func_arg => opt_audio_codec, "force audio codec ('copy' to copy stream)", "codec"),
        option_def!("atag", OPT_AUDIO | HAS_ARG  | OPT_EXPERT | OPT_PERFILE | OPT_OUTPUT, func_arg => opt_old2new, "force audio tag/fourcc", "fourcc/tag"),
        option_def!("vol", OPT_AUDIO | HAS_ARG  | OPT_INT, dst_ptr => audio_volume, "change audio volume (256=normal)" , "volume"),
        option_def!("sample_fmt", OPT_AUDIO | HAS_ARG  | OPT_EXPERT | OPT_SPEC | OPT_STRING | OPT_INPUT | OPT_OUTPUT, off => sample_fmts, "set sample format", "format"),
        option_def!("channel_layout", OPT_AUDIO | HAS_ARG  | OPT_EXPERT | OPT_PERFILE | OPT_INPUT | OPT_OUTPUT, func_arg => opt_channel_layout, "set channel layout", "layout"),
        option_def!("af", OPT_AUDIO | HAS_ARG  | OPT_PERFILE | OPT_OUTPUT, func_arg => opt_audio_filters, "set audio filters", "filter_graph"),
        option_def!("guess_layout_max", OPT_AUDIO | HAS_ARG | OPT_INT | OPT_SPEC | OPT_EXPERT | OPT_INPUT, off => guess_layout_max, "set the maximum number of channels to try to guess the channel layout"),
        option_def!("sn", OPT_SUBTITLE | OPT_BOOL | OPT_OFFSET | OPT_INPUT | OPT_OUTPUT, off => subtitle_disable, "disable subtitle"),
        option_def!("scodec", OPT_SUBTITLE | HAS_ARG  | OPT_PERFILE | OPT_INPUT | OPT_OUTPUT, func_arg => opt_subtitle_codec, "force subtitle codec ('copy' to copy stream)", "codec"),
        option_def!("stag", OPT_SUBTITLE | HAS_ARG  | OPT_EXPERT  | OPT_PERFILE | OPT_OUTPUT, func_arg => opt_old2new, "force subtitle tag/fourcc", "fourcc/tag"),
        option_def!("fix_sub_duration", OPT_BOOL | OPT_EXPERT | OPT_SUBTITLE | OPT_SPEC | OPT_INPUT, off => fix_sub_duration, "fix subtitles duration"),
        option_def!("canvas_size", OPT_SUBTITLE | HAS_ARG | OPT_STRING | OPT_SPEC | OPT_INPUT, off => canvas_sizes, "set canvas size (WxH or abbreviation)", "size"),
        option_def!("vc", HAS_ARG | OPT_EXPERT | OPT_VIDEO, func_arg => opt_video_channel, "deprecated, use -channel", "channel"),
        option_def!("tvstd", HAS_ARG | OPT_EXPERT | OPT_VIDEO, func_arg => opt_video_standard, "deprecated, use -standard", "standard"),
        option_def!("isync", OPT_BOOL | OPT_EXPERT, dst_ptr => input_sync, "this option is deprecated and does nothing", ""),
        option_def!("muxdelay", OPT_FLOAT | HAS_ARG | OPT_EXPERT | OPT_OFFSET | OPT_OUTPUT, off => mux_max_delay, "set the maximum demux-decode delay", "seconds"),
        option_def!("muxpreload", OPT_FLOAT | HAS_ARG | OPT_EXPERT | OPT_OFFSET | OPT_OUTPUT, off => mux_preload, "set the initial demux-decode delay", "seconds"),
        option_def!("sdp_file", HAS_ARG | OPT_EXPERT | OPT_OUTPUT, func_arg => opt_sdp_file, "specify a file in which to print sdp information", "file"),
        option_def!("time_base", HAS_ARG | OPT_STRING | OPT_EXPERT | OPT_SPEC | OPT_OUTPUT, off => time_bases, "set the desired time base hint for output stream (1:24, 1:48000 or 0.04166, 2.0833e-5)", "ratio"),
        option_def!("enc_time_base", HAS_ARG | OPT_STRING | OPT_EXPERT | OPT_SPEC | OPT_OUTPUT, off => enc_time_bases, "set the desired time base for the encoder (1:24, 1:48000 or 0.04166, 2.0833e-5). | two special values are defined - | 0 = use frame rate (video) or sample rate (audio),| -1 = match source time base", "ratio"),
        option_def!("bsf", HAS_ARG | OPT_STRING | OPT_SPEC | OPT_EXPERT | OPT_OUTPUT, off => bitstream_filters, "A comma-separated list of bitstream filters", "bitstream_filters"),
        option_def!("absf", HAS_ARG | OPT_AUDIO | OPT_EXPERT| OPT_PERFILE | OPT_OUTPUT, func_arg => opt_old2new, "deprecated", "audio bitstream_filters"),
        option_def!("vbsf", OPT_VIDEO | HAS_ARG | OPT_EXPERT| OPT_PERFILE | OPT_OUTPUT, func_arg => opt_old2new, "deprecated", "video bitstream_filters"),
        option_def!("apre", HAS_ARG | OPT_AUDIO | OPT_EXPERT| OPT_PERFILE | OPT_OUTPUT, func_arg => opt_preset, "set the audio options to the indicated preset", "preset"),
        option_def!("vpre", OPT_VIDEO | HAS_ARG | OPT_EXPERT| OPT_PERFILE | OPT_OUTPUT, func_arg => opt_preset, "set the video options to the indicated preset", "preset"),
        option_def!("spre", HAS_ARG | OPT_SUBTITLE | OPT_EXPERT| OPT_PERFILE | OPT_OUTPUT, func_arg => opt_preset, "set the subtitle options to the indicated preset", "preset"),
        option_def!("fpre", HAS_ARG | OPT_EXPERT| OPT_PERFILE | OPT_OUTPUT, func_arg => opt_preset, "set options from indicated preset file", "filename"),
        option_def!("max_muxing_queue_size", HAS_ARG | OPT_INT | OPT_SPEC | OPT_EXPERT | OPT_OUTPUT, off => max_muxing_queue_size, "maximum number of packets that can be buffered while waiting for all streams to initialize", "packets"),
        option_def!("dcodec", HAS_ARG | OPT_DATA | OPT_PERFILE | OPT_EXPERT | OPT_INPUT | OPT_OUTPUT, func_arg => opt_data_codec, "force data codec ('copy' to copy stream)", "codec"),
        option_def!("dn", OPT_BOOL | OPT_VIDEO | OPT_OFFSET | OPT_INPUT | OPT_OUTPUT, off => data_disable, "disable data"),
        option_def!("vaapi_device", HAS_ARG | OPT_EXPERT, func_arg => opt_vaapi_device, "set VAAPI hardware device (DRM path or X11 display name)", "device"),
        option_def!("qsv_device", HAS_ARG | OPT_STRING | OPT_EXPERT, dst_ptr => qsv_device, "set QSV hardware device (DirectX adapter index, DRM path or X11 display name)", "device"),
        option_def!("init_hw_device", HAS_ARG | OPT_EXPERT, func_arg => opt_init_hw_device, "initialise hardware device", "args"),
        option_def!("filter_hw_device", HAS_ARG | OPT_EXPERT, func_arg => opt_filter_hw_device, "set hardware device used when filtering", "device"),
    ]
});

// TODO need this be enum?
const VSYNC_AUTO: isize = -1;

// In ffmpeg.h as extern value, TODO extern it
pub static mut videotoolbox_pixfmt: *mut c_char = std::ptr::null_mut();

// In cmdutils.c
pub static mut hide_banner: bool = false;

// In ffmpeg_qsv.c
pub static mut qsv_device: *mut c_char = std::ptr::null_mut();

// In ffmpeg_opt.c
pub static mut intra_only: isize = 0;
pub static mut file_overwrite: isize = 0;
pub static mut no_file_overwrite: isize = 0;
pub static mut do_psnr: isize = 0;
pub static mut input_sync: isize = 0;
pub static mut input_stream_potentially_available: isize = 0;
pub static mut ignore_unknown_streams: isize = 0;
pub static mut copy_unknown_streams: isize = 0;
pub static mut find_stream_info: isize = 1;

pub static mut audio_drift_threshold: f32 = 0.1;
pub static mut dts_delta_threshold: f32 = 10.;
pub static mut dts_error_threshold: f32 = 3600. * 30.;

pub static mut audio_volume: isize = 256;
pub static mut audio_sync_method: isize = 0;
pub static mut video_sync_method: isize = VSYNC_AUTO;
pub static mut frame_drop_threshold: f32 = 0.;
pub static mut do_deinterlace: isize = 0;
pub static mut do_benchmark: isize = 0;
pub static mut do_benchmark_all: isize = 0;
pub static mut do_hex_dump: isize = 0;
pub static mut do_pkt_dump: isize = 0;
pub static mut copy_ts: isize = 0;
pub static mut start_at_zero: isize = 0;
pub static mut copy_tb: isize = -1;
pub static mut debug_ts: isize = 0;
pub static mut exit_on_error: isize = 0;
pub static mut abort_on_flags: isize = 0;
pub static mut print_stats: isize = -1;
pub static mut qp_hist: isize = 0;
pub static mut stdin_interaction: isize = 1;
pub static mut frame_bits_per_raw_sample: isize = 0;
pub static mut max_error_rate: f32 = 2. / 3.;
pub static mut filter_nbthreads: isize = 0;
pub static mut filter_complex_nbthreads: isize = 0;
pub static mut vstats_version: isize = 2;

// In cmdutils.c in random order
fn show_license(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    print!(
        "This is free software; you can redistribute it and/or\n
    modify it under the terms of the GNU Lesser General Public\n
    License as published by the Free Software Foundation; either\n
    version 2.1 of the License, or (at your option) any later version.\n
    \n
    This is distributed in the hope that it will be useful,\n
    but WITHOUT ANY WARRANTY; without even the implied warranty of\n
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU\n
    Lesser General Public License for more details.\n
    \n
    You should have received a copy of the GNU Lesser General Public\n
    License along with this program; if not, write to the Free Software\n
    Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston, MA 02110-1301 USA\n"
    );
    0
}

fn show_help(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    println!("<help message>");
    0
}

fn show_version(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    println!("<version message>");
    0
}

fn show_buildconf(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    println!("<buildconf message>");
    0
}

fn show_formats(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    println!("<formats message>");
    0
}

fn show_muxers(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    println!("<muxers message>");
    0
}

fn show_demuxers(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    println!("<demuxers message>");
    0
}

fn show_devices(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    println!("<devices message>");
    0
}

fn show_codecs(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    println!("<codecs message>");
    0
}

fn show_decoders(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    println!("<decoders message>");
    0
}

fn show_encoders(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    println!("<encoders message>");
    0
}

fn show_bsfs(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    println!("<bsfs message>");
    0
}

fn show_protocols(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    println!("<protocols message>");
    0
}

fn show_filters(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    println!("<filers message>");
    0
}

fn show_pix_fmts(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    println!("<pix_fmts message>");
    0
}

fn show_layouts(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    println!("<layouts message>");
    0
}

fn show_sample_fmts(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    println!("<sample_fmts message>");
    0
}

fn show_colors(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    println!("<colors message>");
    0
}

fn opt_loglevel(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    println!("<loglevel message>");
    0
}

fn opt_report(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    println!("<report message>");
    0
}

fn opt_max_alloc(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    println!("<max_alloc message>");
    0
}

fn opt_cpuflags(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    println!("<cpuflags message>");
    0
}

fn show_sources(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    println!("<sources message>");
    0
}

fn show_sinks(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    println!("<sinks message>");
    0
}

fn opt_timelimit(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    unimplemented!()
}

// In ffmpeg_opt.c, in corresponding order
fn show_hwaccels(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    unimplemented!()
}

fn opt_abort_on(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    unimplemented!()
}
fn opt_sameq(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    unimplemented!()
}
fn opt_video_channel(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    unimplemented!()
}
fn opt_video_standard(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    unimplemented!()
}
fn opt_audio_codec(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    unimplemented!()
}
fn opt_video_codec(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    unimplemented!()
}
fn opt_subtitle_codec(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    unimplemented!()
}
fn opt_data_codec(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    unimplemented!()
}
fn opt_map(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    unimplemented!()
}
fn opt_attach(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    unimplemented!()
}
fn opt_map_channel(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    unimplemented!()
}
fn opt_sdp_file(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    unimplemented!()
}
fn opt_vaapi_device(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    unimplemented!()
}
fn opt_init_hw_device(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    unimplemented!()
}
fn opt_filter_hw_device(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    unimplemented!()
}

fn opt_recording_timestamp(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    unimplemented!()
}

fn opt_streamid(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    unimplemented!()
}

fn opt_target(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    unimplemented!()
}
fn opt_vstats_file(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    unimplemented!()
}
fn opt_vstats(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    unimplemented!()
}
fn opt_video_frames(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    unimplemented!()
}
fn opt_audio_frames(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    unimplemented!()
}
fn opt_data_frames(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    unimplemented!()
}
fn opt_default_new(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    unimplemented!()
}
fn opt_preset(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    unimplemented!()
}
fn opt_old2new(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    unimplemented!()
}
fn opt_bitrate(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    unimplemented!()
}
fn opt_qscale(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    unimplemented!()
}
fn opt_profile(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    unimplemented!()
}
fn opt_video_filters(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    unimplemented!()
}
fn opt_audio_filters(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    unimplemented!()
}
fn opt_vsync(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    unimplemented!()
}
fn opt_timecode(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    unimplemented!()
}
fn opt_channel_layout(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    unimplemented!()
}
fn opt_audio_qscale(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    unimplemented!()
}
fn opt_filter_complex(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    unimplemented!()
}
fn opt_filter_complex_script(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    unimplemented!()
}

fn opt_progress(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    unimplemented!()
}

#[cfg(test)]
mod command_tests {
    use super::*;

    fn opt_cpuflags(_: *mut c_void, _: &str, _: &str) -> i64 {
        0
    }

    #[test]
    fn option_def_macro() {
        let opt = option_def!(
            "cpuflags",
            HAS_ARG | OPT_EXPERT,
            func_arg => opt_cpuflags,
            "force specific cpu flags",
            "flags"
        );
        // We cannot confirm the address of function pointer though.
        assert_eq!(opt.name, "cpuflags");
        assert_eq!(opt.flags, OptionFlag::HAS_ARG | OptionFlag::OPT_EXPERT);
        assert_eq!(opt.help, "force specific cpu flags");
        assert_eq!(opt.argname, Some("flags"));
    }

    #[test]
    fn option_operation_macro() {
        // Test whether it compiles.
        let _ = option_operation!(func_arg => show_help);
    }
}
