use libc::c_void;
use log::{debug, error, info};
use rusty_ffmpeg::{
    avutil::{avutils::*, error::*},
    ffi,
};
use std::{
    ffi::{CStr, CString},
    ptr, slice,
};

use crate::{
    cmdutils::{
        // need to remove the directly imported functions
        init_parse_context,
        split_commandline,
        uninit_parse_context,
    },
    ffmpeg::{self, OptionsContext, INT_CB},
    graph_parser::avfilter_graph_parse2,
    options::*,
};

enum OptGroup {
    GroupOutFile = 0,
    GroupInFile = 1,
}

pub fn ffmpeg_parse_options(args: &[String]) {
    let mut octx = init_parse_context(&*GROUPS);

    let mut filtergraph = None;

    split_commandline(&mut octx, &args, &*OPTIONS, &*GROUPS, &mut filtergraph)
        .expect("split_commandline() failed!");
    // println!("{:#?}", octx);

    if let Some(filtergraph) = filtergraph {
        avfilter_graph_parse2(&filtergraph).unwrap();
    }

    /*
    parse_optgroup(None, &octx.global_opts).expect("parse_optgroup() failed!");

    open_files(
        &mut octx.groups[OptGroup::GroupInFile as usize],
        "input",
        open_input_file,
    )
    .unwrap();

    init_complex_filters();

    open_files(
        &mut octx.groups[OptGroup::GroupOutFile as usize],
        "output",
        open_output_file,
    )
    .unwrap();

    check_filter_outputs();
    */

    uninit_parse_context(&mut octx);
}
