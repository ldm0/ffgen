# ffgen

## Input 

```
PKG_CONFIG_PATH="$HOME/ffmpeg_build/lib/pkgconfig" cargo run -- -i input.png -vf "sws_flags=+accurate_rnd+bitexact;[in]scale=720:480, split [main][tmp]; [tmp] crop=iw:ih/2:0:0, vflip [flip]; [main][flip] overlay=0:H/2[out]" output.png
```

## Output

```
[2020-07-30T20:34:55Z DEBUG ffgen::cmdutils] Splitting the commandline.
[2020-07-30T20:34:55Z DEBUG ffgen::cmdutils] Reading option '-i' ...
[2020-07-30T20:34:55Z DEBUG ffgen::cmdutils]  matched as input url with argument 'input.png'.
[2020-07-30T20:34:55Z DEBUG ffgen::cmdutils] Reading option '-vf' ...
[2020-07-30T20:34:55Z DEBUG ffgen::cmdutils]  matched as option 'vf' (set video filters) with argument '"sws_flags=+accurate_rnd+bitexact;[in]scale=720:480, split [main][tmp]; [tmp] crop=iw:ih/2:0:0, vflip [flip]; [main][flip] overlay=0:H/2[out]"'.
[2020-07-30T20:34:55Z DEBUG ffgen::cmdutils] Reading option 'output.png' ...
[2020-07-30T20:34:55Z DEBUG ffgen::cmdutils]  matched as output url.
[2020-07-30T20:34:55Z DEBUG ffgen::cmdutils] Finished splitting the commandline.

finish_group(octx, 1, "input.png");


add_opt(octx, find_option(options, "vf"), "vf", "sws_flags=+accurate_rnd+bitexact;[in]scale=720:480, split [main][tmp]; [tmp] crop=iw:ih/2:0:0, vflip [flip]; [main][flip] overlay=0:H/2[out]");


finish_group(octx, 0, "output.png");


av_freep(&graph->scale_sws_opts);
if (!(graph->scale_sws_opts = av_mallocz(29)))
    return AVERROR(ENOMEM);
av_strlcpy(graph->scale_sws_opts, "flags=+accurate_rnd+bitexact", 29);


AVFilterContext* filter_scale_0 = avfilter_graph_alloc_filter(ctx, avfilter_get_by_name("scale"), "Parsed_scale_0");
if (!filter_scale_0) {
    av_log(log_ctx, AV_LOG_ERROR,
        "Error creating filter 'scale'\n");
    return AVERROR(ENOMEM);
}
avfilter_init_str(filter_scale_0, "720:480:flags=+accurate_rnd+bitexact");


AVFilterContext* filter_split_1 = avfilter_graph_alloc_filter(ctx, avfilter_get_by_name("split"), "Parsed_split_1");
if (!filter_split_1) {
    av_log(log_ctx, AV_LOG_ERROR,
        "Error creating filter 'split'\n");
    return AVERROR(ENOMEM);
}
avfilter_init_str(filter_split_1, "");


AVFilterContext* filter_crop_2 = avfilter_graph_alloc_filter(ctx, avfilter_get_by_name("crop"), "Parsed_crop_2");
if (!filter_crop_2) {
    av_log(log_ctx, AV_LOG_ERROR,
        "Error creating filter 'crop'\n");
    return AVERROR(ENOMEM);
}
avfilter_init_str(filter_crop_2, "iw:ih/2:0:0");


AVFilterContext* filter_vflip_3 = avfilter_graph_alloc_filter(ctx, avfilter_get_by_name("vflip"), "Parsed_vflip_3");
if (!filter_vflip_3) {
    av_log(log_ctx, AV_LOG_ERROR,
        "Error creating filter 'vflip'\n");
    return AVERROR(ENOMEM);
}
avfilter_init_str(filter_vflip_3, "");


AVFilterContext* filter_overlay_4 = avfilter_graph_alloc_filter(ctx, avfilter_get_by_name("overlay"), "Parsed_overlay_4");
if (!filter_overlay_4) {
    av_log(log_ctx, AV_LOG_ERROR,
        "Error creating filter 'overlay'\n");
    return AVERROR(ENOMEM);
}
avfilter_init_str(filter_overlay_4, "0:H/2");


if ((ret = avfilter_link(filter_scale_0, 0, filter_split_1, 0))) {
    av_log(log_ctx, AV_LOG_ERROR,
            "Cannot create the link filter_scale_0:0 -> filter_split_1:0\n",
    return ret;
}


if ((ret = avfilter_link(filter_split_1, 1, filter_crop_2, 0))) {
    av_log(log_ctx, AV_LOG_ERROR,
            "Cannot create the link filter_split_1:1 -> filter_crop_2:0\n",
    return ret;
}


if ((ret = avfilter_link(filter_crop_2, 0, filter_vflip_3, 0))) {
    av_log(log_ctx, AV_LOG_ERROR,
            "Cannot create the link filter_crop_2:0 -> filter_vflip_3:0\n",
    return ret;
}


if ((ret = avfilter_link(filter_split_1, 0, filter_overlay_4, 0))) {
    av_log(log_ctx, AV_LOG_ERROR,
            "Cannot create the link filter_split_1:0 -> filter_overlay_4:0\n",
    return ret;
}


if ((ret = avfilter_link(filter_vflip_3, 0, filter_overlay_4, 1))) {
    av_log(log_ctx, AV_LOG_ERROR,
            "Cannot create the link filter_vflip_3:0 -> filter_overlay_4:1\n",
    return ret;
}


AVFilterInOut *input_0;
if (!(input_0 = av_mallocz(sizeof(AVFilterInOut)))) {
    av_free(name);
    return AVERROR(ENOMEM);
}
input_0->pad_idx = 0;
input_0->filt_ctx = filter_scale_0;


AVFilterInOut *output_0;
if (!(output_0 = av_mallocz(sizeof(AVFilterInOut)))) {
    av_free(name);
    return AVERROR(ENOMEM);
}
output_0->pad_idx = 0;
output_0->filt_ctx = filter_overlay_4;


*inputs = input_0;
*outputs = output_0;
```

## Input

```
PKG_CONFIG_PATH="$HOME/ffmpeg_build/lib/pkgconfig" cargo run -- -i input.mkv -vf scale=320:240 output.mp4
```

## Output

```
[2020-07-30T20:36:15Z DEBUG ffgen::cmdutils] Splitting the commandline.
[2020-07-30T20:36:15Z DEBUG ffgen::cmdutils] Reading option '-i' ...
[2020-07-30T20:36:15Z DEBUG ffgen::cmdutils]  matched as input url with argument 'input.mkv'.
[2020-07-30T20:36:15Z DEBUG ffgen::cmdutils] Reading option '-vf' ...
[2020-07-30T20:36:15Z DEBUG ffgen::cmdutils]  matched as option 'vf' (set video filters) with argument '"scale=320:240"'.
[2020-07-30T20:36:15Z DEBUG ffgen::cmdutils] Reading option 'output.mp4' ...
[2020-07-30T20:36:15Z DEBUG ffgen::cmdutils]  matched as output url.
[2020-07-30T20:36:15Z DEBUG ffgen::cmdutils] Finished splitting the commandline.

finish_group(octx, 1, "input.mkv");


add_opt(octx, find_option(options, "vf"), "vf", "scale=320:240");


finish_group(octx, 0, "output.mp4");


AVFilterContext* filter_scale_0 = avfilter_graph_alloc_filter(ctx, avfilter_get_by_name("scale"), "Parsed_scale_0");
if (!filter_scale_0) {
    av_log(log_ctx, AV_LOG_ERROR,
        "Error creating filter 'scale'\n");
    return AVERROR(ENOMEM);
}
avfilter_init_str(filter_scale_0, "320:240");


AVFilterInOut *input_0;
if (!(input_0 = av_mallocz(sizeof(AVFilterInOut)))) {
    av_free(name);
    return AVERROR(ENOMEM);
}
input_0->pad_idx = 0;
input_0->filt_ctx = filter_scale_0;


AVFilterInOut *output_0;
if (!(output_0 = av_mallocz(sizeof(AVFilterInOut)))) {
    av_free(name);
    return AVERROR(ENOMEM);
}
output_0->pad_idx = 0;
output_0->filt_ctx = filter_scale_0;


*inputs = input_0;
*outputs = output_0;
```
