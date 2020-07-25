//! This file corresponds to ffmpeg.\[ch\]
use log::{debug, error, info};
use once_cell::sync::Lazy;
use rusty_ffmpeg::{avutil::avutils::*, ffi};

use std::{
    env,
    ffi::{CStr, CString},
    ptr,
    sync::Mutex,
};

use crate::{
    cmdutils::{OptionGroup, SpecifierOpt},
    ffmpeg_opt,
};

use ffmpeg_opt::ffmpeg_parse_options;

static RECEIVED_NB_SIGNALS: Lazy<Mutex<isize>> = Lazy::new(|| Mutex::new(0));
static TRANSCODE_INIT_DONE: Lazy<Mutex<isize>> = Lazy::new(|| Mutex::new(0));

unsafe extern "C" fn decodec_interrupt_cb(_ctx: *mut libc::c_void) -> libc::c_int {
    let received_nb_signals: &isize = &RECEIVED_NB_SIGNALS.lock().unwrap();
    let transcode_init_done: &isize = &TRANSCODE_INIT_DONE.lock().unwrap();
    if received_nb_signals > transcode_init_done {
        1
    } else {
        0
    }
}

pub const INT_CB: ffi::AVIOInterruptCB = ffi::AVIOInterruptCB {
    callback: Some(decodec_interrupt_cb),
    opaque: ptr::null_mut(),
};

pub unsafe fn remove_avoptions(a: &mut *mut ffi::AVDictionary, b: *mut ffi::AVDictionary) {
    let mut t = ptr::null();
    let empty = CString::new("").unwrap();

    loop {
        t = ffi::av_dict_get(b, empty.as_ptr(), t, ffi::AV_DICT_IGNORE_SUFFIX as i32);
        if let Some(t) = t.as_ref() {
            ffi::av_dict_set(a, t.key, ptr::null(), ffi::AV_DICT_MATCH_CASE as i32);
        } else {
            break;
        }
    }
}

pub unsafe fn assert_avoptions(a: *mut ffi::AVDictionary) {
    let empty = CString::new("").unwrap();
    if let Some(t) = ffi::av_dict_get(
        a,
        empty.as_ptr(),
        ptr::null(),
        ffi::AV_DICT_IGNORE_SUFFIX as i32,
    )
    .as_ref()
    {
        let t_key = CStr::from_ptr(t.key);
        error!("Option {} not found.", t_key.to_string_lossy());
    };
}

#[derive(Debug, Default)]
pub struct StreamMap {
    pub disabled: isize,
    pub file_index: isize,
    pub stream_index: isize,
    pub sync_file_index: isize,
    pub sync_stream_index: isize,
    pub linklabel: String,
}

#[derive(Debug, Default)]
pub struct AudioChannelMap {
    // input
    pub file_idx: isize,
    pub stream_idx: isize,
    pub channel_idx: isize,
    // output
    pub ofile_idx: isize,
    pub ostream_idx: isize,
}

#[derive(Debug)]
pub struct OptionsContext<'a, 'group> {
    pub g: &'a mut OptionGroup<'group>,

    // input/output options
    pub start_time: i64,
    pub start_time_eof: i64,
    pub seek_timestamp: isize,
    pub format: String,

    pub codec_names: Vec<SpecifierOpt>,
    pub audio_channels: Vec<SpecifierOpt>,
    pub audio_sample_rate: Vec<SpecifierOpt>,
    pub frame_rates: Vec<SpecifierOpt>,
    pub frame_sizes: Vec<SpecifierOpt>,
    pub frame_pix_fmts: Vec<SpecifierOpt>,

    // input options
    pub input_ts_offset: i64,
    pub loops: isize,
    pub rate_emu: isize,
    pub accurate_seek: isize,
    pub thread_queue_size: isize,

    pub ts_scale: Vec<SpecifierOpt>,
    pub dump_attachment: Vec<SpecifierOpt>,
    pub hwaccels: Vec<SpecifierOpt>,
    pub hwaccel_devices: Vec<SpecifierOpt>,
    pub hwaccel_output_formats: Vec<SpecifierOpt>,
    pub autorotate: Vec<SpecifierOpt>,

    // output options
    pub stream_maps: Vec<StreamMap>,
    // ATTENTION here does the nb_audio* is the length of the audio* array?
    // I'm not sure. Currently I assume they are the same. If not we need to a a integer here.
    // AudioChannelMap *audio_channel_maps; /* one info entry per -map_channel */
    // int           nb_audio_channel_maps; /* number of (valid) -map_channel settings */
    pub audio_channel_maps: Vec<AudioChannelMap>,
    pub metadata_global_manual: isize,
    pub metadata_streams_manual: isize,
    pub metadata_chapters_manual: isize,
    pub attachments: Vec<String>,

    pub chapters_input_file: isize,

    pub recording_time: i64,
    pub stop_time: i64,
    pub limit_filesize: u64,
    pub mux_preload: f32,
    pub mux_max_delay: f32,
    pub shortest: isize,
    pub bitexact: isize,

    pub video_disable: isize,
    pub audio_disable: isize,
    pub subtitle_disable: isize,
    pub data_disable: isize,

    // indexed by output file stream index
    pub streamid_map: Vec<isize>,

    pub metadata: Vec<SpecifierOpt>,
    pub max_frames: Vec<SpecifierOpt>,
    pub bitstream_filters: Vec<SpecifierOpt>,
    pub codec_tags: Vec<SpecifierOpt>,
    pub sample_fmts: Vec<SpecifierOpt>,
    pub qscale: Vec<SpecifierOpt>,
    pub forced_key_frames: Vec<SpecifierOpt>,
    pub force_fps: Vec<SpecifierOpt>,
    pub frame_aspect_ratios: Vec<SpecifierOpt>,
    pub rc_overrides: Vec<SpecifierOpt>,
    pub intra_matrices: Vec<SpecifierOpt>,
    pub inter_matrices: Vec<SpecifierOpt>,
    pub chroma_intra_matrices: Vec<SpecifierOpt>,
    pub top_field_first: Vec<SpecifierOpt>,
    pub metadata_map: Vec<SpecifierOpt>,
    pub presets: Vec<SpecifierOpt>,
    pub copy_initial_nonkeyframes: Vec<SpecifierOpt>,
    pub copy_prior_start: Vec<SpecifierOpt>,
    pub filters: Vec<SpecifierOpt>,
    pub filter_scripts: Vec<SpecifierOpt>,
    pub reinit_filters: Vec<SpecifierOpt>,
    pub fix_sub_duration: Vec<SpecifierOpt>,
    pub canvas_sizes: Vec<SpecifierOpt>,
    pub pass: Vec<SpecifierOpt>,
    pub passlogfiles: Vec<SpecifierOpt>,
    pub max_muxing_queue_size: Vec<SpecifierOpt>,
    pub guess_layout_max: Vec<SpecifierOpt>,
    pub apad: Vec<SpecifierOpt>,
    pub discard: Vec<SpecifierOpt>,
    pub disposition: Vec<SpecifierOpt>,
    pub program: Vec<SpecifierOpt>,
    pub time_bases: Vec<SpecifierOpt>,
    pub enc_time_bases: Vec<SpecifierOpt>,
}

impl<'a, 'group> OptionsContext<'a, 'group> {
    pub fn new(group: &'a mut OptionGroup<'group>) -> Self {
        Self {
            g: group,
            stop_time: i64::MAX,
            mux_max_delay: 0.7,
            start_time: AV_NOPTS_VALUE,
            start_time_eof: AV_NOPTS_VALUE,
            recording_time: i64::MAX,
            limit_filesize: u64::MAX,
            chapters_input_file: isize::MAX,
            accurate_seek: 1,

            // fields below are set with default options.

            // input/output options
            seek_timestamp: 0,
            format: Default::default(),

            codec_names: vec![],
            audio_channels: vec![],
            audio_sample_rate: vec![],
            frame_rates: vec![],
            frame_sizes: vec![],
            frame_pix_fmts: vec![],

            // input options
            input_ts_offset: 0,
            loops: 0,
            rate_emu: 0,
            thread_queue_size: 0,

            ts_scale: vec![],
            dump_attachment: vec![],
            hwaccels: vec![],
            hwaccel_devices: vec![],
            hwaccel_output_formats: vec![],
            autorotate: vec![],

            // output options
            stream_maps: vec![],
            audio_channel_maps: vec![],
            metadata_global_manual: 0,
            metadata_streams_manual: 0,
            metadata_chapters_manual: 0,
            attachments: vec![],

            mux_preload: 0.,
            shortest: 0,
            bitexact: 0,

            video_disable: 0,
            audio_disable: 0,
            subtitle_disable: 0,
            data_disable: 0,

            // indexed by output file stream index
            streamid_map: vec![],

            metadata: vec![],
            max_frames: vec![],
            bitstream_filters: vec![],
            codec_tags: vec![],
            sample_fmts: vec![],
            qscale: vec![],
            forced_key_frames: vec![],
            force_fps: vec![],
            frame_aspect_ratios: vec![],
            rc_overrides: vec![],
            intra_matrices: vec![],
            inter_matrices: vec![],
            chroma_intra_matrices: vec![],
            top_field_first: vec![],
            metadata_map: vec![],
            presets: vec![],
            copy_initial_nonkeyframes: vec![],
            copy_prior_start: vec![],
            filters: vec![],
            filter_scripts: vec![],
            reinit_filters: vec![],
            fix_sub_duration: vec![],
            canvas_sizes: vec![],
            pass: vec![],
            passlogfiles: vec![],
            max_muxing_queue_size: vec![],
            guess_layout_max: vec![],
            apad: vec![],
            discard: vec![],
            disposition: vec![],
            program: vec![],
            time_bases: vec![],
            enc_time_bases: vec![],
        }
    }
}

pub fn ffmpeg() {
    // TODO: May need to change to Vec<u8> for non-UTF8 args.
    let args: Vec<String> = env::args().collect();

    ffmpeg_parse_options(&args);
}
