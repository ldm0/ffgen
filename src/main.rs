// For the `&raw *` used in the macro of options.rs, will be stabilized later
#![feature(raw_ref_op)]
// For the half open range in match in `split_commandline()`'s AVOption part
#![feature(exclusive_range_pattern)]
#![feature(half_open_range_patterns)]
#![feature(ptr_offset_from)]
#![feature(bool_to_option)]
mod cmdutils;
mod ffmpeg;
mod ffmpeg_opt;
mod graph_parser;
mod options;

use env_logger;

use std::env;

fn main() {
    env::set_var("RUST_LOG", "debug");
    env_logger::init();
    ffmpeg::ffmpeg();
}
