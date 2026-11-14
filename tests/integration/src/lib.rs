//! G1 集成测试（M1-T2）：gst-adapter 管线生命周期 / 帧获取 / PTS / 有界缓冲 / EOS / 错误路径。
//!
//! **证据级别声明（AGENTS §4）**：以下全部为**进程内 GStreamer 测试（CI 级证据）**，
//! 不得表述为 OS 虚拟摄像头/设备验证。
//!
//! 等待纪律：所有等待均带显式超时，无无限阻塞（AGENTS §5）。

#[cfg(test)]
mod pipeline_integration {
    use std::time::{Duration, Instant};

    use gst_adapter::{
        EosOutcome, LifecycleState, MetricsSnapshot, PipelineConfig, TestVideoPipeline,
    };

    /// Result → panic 助手（测试专用；工作区禁 unwrap/expect lint 下的替代写法）。
    fn ok<T, E: std::fmt::Display>(r: Result<T, E>) -> T {
        r.unwrap_or_else(|e| panic!("unexpected error: {e}"))
    }

    /// 在超时内等待帧数达标（50 ms 轮询）。
    fn wait_frames(p: &TestVideoPipeline, min_frames: u64, timeout: Duration) -> MetricsSnapshot {
        let began = Instant::now();
        loop {
            let s = p.snapshot();
            if s.frames_received >= min_frames {
                return s;
            }
            if began.elapsed() >= timeout {
                panic!(
                    "timeout waiting for {min_frames} frames; got {} (last_error={:?}, caps_parse_failures={}, map_mismatches={})",
                    s.frames_received, s.last_error, s.caps_parse_failures, s.map_size_mismatches
                );
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// T1：创建 → 启动 → 停止 → 重复启动（生命周期转换 + 幂等 stop + 双启动拒绝）。
    #[test]
    fn t1_lifecycle_roundtrip_and_idempotent_stop() {
        let p = ok(TestVideoPipeline::build(PipelineConfig::default_720p30()));
        assert_eq!(ok(p.state()), LifecycleState::Created);

        ok(p.start());
        assert_eq!(ok(p.state()), LifecycleState::Playing);
        // 已在 PLAYING 时再次 start 必须报错（明确拒绝，不静默）。
        assert!(p.start().is_err(), "double start must be rejected");

        ok(p.stop());
        assert_eq!(ok(p.state()), LifecycleState::Stopped);
        ok(p.stop()); // 幂等
        assert_eq!(ok(p.state()), LifecycleState::Stopped);

        // Stopped 后可再次启动（重复启停能力）。
        ok(p.start());
        assert_eq!(ok(p.state()), LifecycleState::Playing);
        ok(p.stop());
        assert_eq!(ok(p.state()), LifecycleState::Stopped);
    }

    /// T2：3 秒内收到 ≥45 帧（真实帧获取 + 限时达标）。
    #[test]
    fn t2_receives_enough_frames_within_timeout() {
        let p = ok(TestVideoPipeline::build(PipelineConfig::default_720p30()));
        ok(p.start());
        let s = wait_frames(&p, 45, Duration::from_secs(3));
        ok(p.stop());
        assert!(s.caps_parse_failures == 0, "caps parse failures: {s:?}");
        assert!(s.actual_fps > 15.0, "actual fps too low: {}", s.actual_fps);
    }

    /// T3：PTS 单调递增、duration 合理（30fps 标称 33.33 ms，容忍 20~67 ms）。
    #[test]
    fn t3_pts_monotonic_and_duration_sane() {
        let p = ok(TestVideoPipeline::build(PipelineConfig::default_720p30()));
        ok(p.start());
        wait_frames(&p, 50, Duration::from_secs(3));
        let s = p.snapshot();
        ok(p.stop());

        assert_eq!(
            s.pts_monotonic_violations, 0,
            "PTS monotonicity violated: {s:?}"
        );
        assert!(
            s.first_pts_ns >= 0 && s.last_pts_ns > s.first_pts_ns,
            "pts range wrong: {s:?}"
        );
        let lo = 20_000_000u64;
        let hi = 67_000_000u64;
        assert!(
            s.duration_min_ns >= lo && s.duration_max_ns <= hi,
            "duration out of sane range: min={}ns max={}ns",
            s.duration_min_ns,
            s.duration_max_ns
        );
        let last = s.last_frame.unwrap_or_else(|| panic!("no last frame"));
        assert!(
            last.duration_ns >= lo && last.duration_ns <= hi,
            "last frame duration off: {}ns",
            last.duration_ns
        );
    }

    /// T4：caps 与实际帧格式匹配；NV12 双平面 stride/offset/总长按协商布局核对
    /// （**不假定平面紧密排列**——公式对齐：UV 偏移 = Y 行跨距×高；总长 = UV 偏移 + UV 跨距×高/2）。
    #[test]
    fn t4_caps_and_layout_match_nv12_720p() {
        let p = ok(TestVideoPipeline::build(PipelineConfig::default_720p30()));
        ok(p.start());
        wait_frames(&p, 30, Duration::from_secs(3));
        let s = p.snapshot();
        ok(p.stop());

        assert_eq!(s.caps_parse_failures, 0, "caps parse failures: {s:?}");
        assert_eq!(s.map_size_mismatches, 0, "map size mismatches: {s:?}");
        let f = s.last_frame.unwrap_or_else(|| panic!("no frame"));
        assert_eq!(f.format, "NV12", "negotiated format: {}", f.format);
        assert_eq!(f.width, 1280);
        assert_eq!(f.height, 720);
        assert_eq!(f.n_planes, 2, "NV12 must have 2 planes");
        // stride 不得小于裸行宽（可能带对齐 padding——必须按协商值处理）。
        assert!(
            f.strides[0] >= 1280,
            "Y stride < raw width: {}",
            f.strides[0]
        );
        assert!(
            f.strides[1] >= 1280,
            "UV stride < raw width: {}",
            f.strides[1]
        );
        // 平面布局公式核对（半高 UV 平面）。
        let height = u64::from(f.height);
        assert_eq!(
            f.plane_offsets[1],
            f.strides[0] * height,
            "UV plane offset != Y stride * height (layout not honored)"
        );
        assert_eq!(
            f.map_size,
            f.plane_offsets[1] + f.strides[1] * (height / 2),
            "map size != offsets + plane strides (layout not honored)"
        );
        // 记录实测值（证据引用）。
        println!(
            "T4 negotiated layout: stride_y={} stride_uv={} offset_uv={} map_size={}",
            f.strides[0], f.strides[1], f.plane_offsets[1], f.map_size
        );
    }

    /// T5：5 轮 start→取帧→stop 无崩溃、无卡死；每轮都重新收到帧（资源可重建）。
    #[test]
    fn t5_repeated_start_stop_cycles_are_clean() {
        let p = ok(TestVideoPipeline::build(PipelineConfig::default_720p30()));
        let mut prev_total = 0u64;
        for cycle in 0..5u32 {
            ok(p.start());
            wait_frames(&p, prev_total + 10, Duration::from_secs(3));
            let s = p.snapshot();
            assert!(
                s.frames_received > prev_total,
                "cycle {cycle}: no new frames after restart"
            );
            prev_total = s.frames_received;
            let t0 = Instant::now();
            ok(p.stop());
            assert!(
                t0.elapsed() < Duration::from_secs(6),
                "cycle {cycle}: stop took over 5s timeout budget"
            );
        }
        ok(p.stop());
        let fin = p.snapshot();
        // PTS 单调性为运行内不变量（每次 start 重置作用域）——此断言核查**末轮**无违例；
        // 布局失配跨周期累计，断言全周期为 0。
        assert!(
            fin.pts_monotonic_violations == 0 && fin.map_size_mismatches == 0,
            "anomalies in final cycle (pts) or accumulated (map): {fin:?}"
        );
        println!("T5: 5 cycles, total frames={prev_total}");
    }

    /// T6：故意放慢消费（回调 200 ms/帧）→ 有界队列 + 丢帧策略生效，内存不随积压增长。
    ///
    /// 内存不增长的有界性论证（实测代理）：queue max-size-buffers=4 / bytes=8 MiB 为
    /// GStreamer 硬上限，采样期内 queue 积压始终 ≤ 上限、overrun 持续丢弃旧帧——
    /// 缓冲占用被常量上限封顶，不随时间增长。
    #[test]
    fn t6_slow_consumer_backpressure_is_bounded() {
        let p = ok(TestVideoPipeline::build(PipelineConfig::default_720p30()));
        p.set_consume_delay_us(200_000); // 5 fps 消费 vs 30 fps 生产
        ok(p.start());

        let began = Instant::now();
        let mut max_level_buffers: u32 = 0;
        let mut received_last = 0u64;
        let mut stalled_samples = 0u32;
        while began.elapsed() < Duration::from_secs(6) {
            std::thread::sleep(Duration::from_millis(250));
            let (lvl, _) = ok(p.queue_levels());
            max_level_buffers = max_level_buffers.max(lvl);
            let now = p.snapshot();
            if now.frames_received == received_last {
                stalled_samples += 1;
            }
            received_last = now.frames_received;
        }
        let s = p.snapshot();
        ok(p.stop());
        p.set_consume_delay_us(0); // 恢复（防御性；实例即将销毁）

        assert!(
            s.queue_overruns > 0,
            "no overrun observed under slow consumer: {s:?}"
        );
        assert!(
            u64::from(max_level_buffers) <= u64::from(p.queue_max_buffers()),
            "queue level exceeded configured max: {max_level_buffers}"
        );
        assert!(s.frames_received >= 5, "consumer starved entirely: {s:?}");
        assert!(
            s.actual_fps < 15.0 && s.actual_fps > 0.0,
            "slow-consumer fps out of expected band: {}",
            s.actual_fps
        );
        // 消费流未死锁：慢消费下仍持续有帧到达（允许短暂停顿，但 6 s 内不超 12 个采样点）。
        assert!(
            stalled_samples <= 12,
            "receiver stalled too long under slow consumer: {stalled_samples} samples"
        );
        println!(
            "T6: received={} fps={:.2} overruns={} max_level_buffers={max_level_buffers}",
            s.frames_received, s.actual_fps, s.queue_overruns
        );
    }

    /// T7：EOS 在限时内退出（send_eos → bus EOS → stop），无超时悬挂。
    #[test]
    fn t7_eos_terminates_within_timeout() {
        let p = ok(TestVideoPipeline::build(PipelineConfig::default_720p30()));
        ok(p.start());
        wait_frames(&p, 10, Duration::from_secs(3));
        ok(p.send_eos());
        let t0 = Instant::now();
        let outcome = ok(p.wait_for_eos_or_error(5));
        assert_eq!(outcome, EosOutcome::Eos, "expected EOS: {outcome:?}");
        assert!(
            t0.elapsed() < Duration::from_secs(6),
            "EOS wait exceeded budget"
        );
        ok(p.stop());
        assert_eq!(ok(p.state()), LifecycleState::Stopped);
    }

    /// T8：错误路径限时退出（identity error-after 注入 → bus ERROR → Error 态 → stop 释放）。
    #[test]
    fn t8_error_path_exits_within_timeout() {
        let cfg = PipelineConfig {
            inject_error_after_frames: Some(10),
            ..PipelineConfig::default_720p30()
        };
        let p = ok(TestVideoPipeline::build(cfg));
        ok(p.start());
        let t0 = Instant::now();
        let outcome = ok(p.wait_for_eos_or_error(8));
        assert!(
            matches!(outcome, EosOutcome::Error(ref m) if !m.is_empty()),
            "expected bus error outcome: {outcome:?}"
        );
        assert!(
            t0.elapsed() < Duration::from_secs(9),
            "error wait exceeded budget"
        );
        // 错误态状态机：生命周期状态记录的正是 bus 错误消息。
        let EosOutcome::Error(ref msg) = outcome else {
            panic!("unreachable: outcome checked above");
        };
        assert_eq!(ok(p.state()), LifecycleState::Error(msg.clone()));
        ok(p.stop());
        assert_eq!(ok(p.state()), LifecycleState::Stopped);
        let s = p.snapshot();
        assert!(
            s.last_error.as_ref().is_some_and(|m| !m.is_empty()),
            "error message must be recorded in metrics"
        );
    }

    /// T9：确定性构图——两段独立启动的管线首帧头部哈希一致（静态 SMPTE + 固定 caps）。
    #[test]
    fn t9_deterministic_pattern_hash_stable_across_restarts() {
        let hash_of_run = |label: &str| -> u64 {
            let p = ok(TestVideoPipeline::build(PipelineConfig::default_720p30()));
            ok(p.start());
            let s = wait_frames(&p, 30, Duration::from_secs(3));
            ok(p.stop());
            assert_ne!(s.first_frame_hash, 0, "{label}: zero hash");
            s.first_frame_hash
        };
        let h1 = hash_of_run("run1");
        let h2 = hash_of_run("run2");
        assert_eq!(h1, h2, "composition not deterministic: {h1} vs {h2}");
    }

    /// T10：10 秒持续运行——快速消费下不掉帧、PTS 无违例、积压有界（稳定性基线）。
    #[test]
    fn t10_sustained_10s_stability() {
        let p = ok(TestVideoPipeline::build(PipelineConfig::default_720p30()));
        ok(p.start());
        let began = Instant::now();
        let mut max_level_buffers: u32 = 0;
        while began.elapsed() < Duration::from_secs(10) {
            std::thread::sleep(Duration::from_millis(250));
            let (lvl, _) = ok(p.queue_levels());
            max_level_buffers = max_level_buffers.max(lvl);
        }
        let s = p.snapshot();
        ok(p.stop());

        assert!(
            s.frames_received >= 270,
            "expected >=270 frames in 10s (30fps), got {}",
            s.frames_received
        );
        assert_eq!(s.pts_monotonic_violations, 0, "violations: {s:?}");
        assert_eq!(s.map_size_mismatches, 0, "mismatches: {s:?}");
        assert_eq!(s.caps_parse_failures, 0, "caps failures: {s:?}");
        assert_eq!(
            s.queue_overruns, 0,
            "unexpected drops at fast consumption: {s:?}"
        );
        assert!(
            u64::from(max_level_buffers) <= u64::from(p.queue_max_buffers()),
            "queue boundedness violated: {max_level_buffers}"
        );
        assert!(
            s.actual_fps >= 25.0,
            "sustained fps too low: {}",
            s.actual_fps
        );
        println!(
            "T10: 10s received={} fps={:.2} max_level_buffers={max_level_buffers}",
            s.frames_received, s.actual_fps
        );
    }
}
