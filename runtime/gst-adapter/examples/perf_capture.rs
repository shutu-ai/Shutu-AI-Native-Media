//! M1-T2 性能采样示例：10 秒持续运行的帧率 / PTS / 队列有界性原始数据（JSON 输出）。
//!
//! 用法（需 PATH 含 GStreamer MSVC bin，见 scripts/ci.ps1）：
//! `cargo run -p gst-adapter --example perf_capture`
//!
//! 测量方法：进程内 videotestsrc(is-live, smpte) 720p30 NV12 固定管线；
//! appsink 回调仅记录元数据（零拷贝、零耗时钩子）；每 250 ms 采样 queue 积压上限。

use std::time::{Duration, Instant};

use gst_adapter::{PipelineConfig, TestVideoPipeline};

fn main() {
    let pipeline = TestVideoPipeline::build(PipelineConfig::default_720p30()).unwrap_or_else(|e| {
        eprintln!("build failed: {e}");
        std::process::exit(1);
    });
    pipeline.start().unwrap_or_else(|e| {
        eprintln!("start failed: {e}");
        std::process::exit(1);
    });

    let mut max_level_buffers: u32 = 0;
    let mut max_level_bytes: u32 = 0;
    let mut samples = 0u64;
    let began = Instant::now();
    while began.elapsed() < Duration::from_secs(10) {
        std::thread::sleep(Duration::from_millis(250));
        if let Ok((b, by)) = pipeline.queue_levels() {
            max_level_buffers = max_level_buffers.max(b);
            max_level_bytes = max_level_bytes.max(by);
            samples += 1;
        }
    }

    let snap = pipeline.snapshot();
    pipeline.stop().unwrap_or_else(|e| {
        eprintln!("stop failed: {e}");
        std::process::exit(1);
    });

    let Some(f) = snap.last_frame else {
        eprintln!("no frames received in sample window");
        std::process::exit(1);
    };

    println!("{{");
    println!("  \"sample_window_s\": 10,");
    println!("  \"queue_level_samples\": {samples},");
    println!("  \"max_queue_level_buffers\": {max_level_buffers},");
    println!("  \"max_queue_level_bytes\": {max_level_bytes},");
    println!("  \"frames_received\": {},", snap.frames_received);
    println!("  \"actual_fps\": {:.3},", snap.actual_fps);
    println!(
        "  \"pts_monotonic_violations\": {},",
        snap.pts_monotonic_violations
    );
    println!("  \"queue_overruns\": {},", snap.queue_overruns);
    println!("  \"caps_parse_failures\": {},", snap.caps_parse_failures);
    println!("  \"map_size_mismatches\": {},", snap.map_size_mismatches);
    println!("  \"duration_min_ns\": {},", snap.duration_min_ns);
    println!("  \"duration_max_ns\": {},", snap.duration_max_ns);
    println!("  \"first_pts_ns\": {},", snap.first_pts_ns);
    println!("  \"last_pts_ns\": {},", snap.last_pts_ns);
    println!("  \"first_frame_hash\": {},", snap.first_frame_hash);
    println!("  \"last_frame_hash\": {},", snap.last_frame_hash);
    println!("  \"last_frame_seq\": {},", f.seq);
    println!("  \"last_frame_format\": \"{}\",", f.format);
    println!("  \"last_frame_width\": {},", f.width);
    println!("  \"last_frame_height\": {},", f.height);
    println!("  \"last_frame_n_planes\": {},", f.n_planes);
    println!("  \"last_frame_stride_y\": {},", f.strides[0]);
    println!("  \"last_frame_stride_uv\": {},", f.strides[1]);
    println!("  \"last_frame_offset_uv\": {},", f.plane_offsets[1]);
    println!("  \"last_frame_map_size\": {},", f.map_size);
    println!("  \"clock_source_pts\": \"{}\",", snap.clock_source_pts);
    println!("  \"clock_source_recv\": \"{}\"", snap.clock_source_recv);
    println!("}}");
}
