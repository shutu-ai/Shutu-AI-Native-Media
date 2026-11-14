//! 管线指标（原子计数 + 不可变快照）。
//!
//! 回调路径只做原子增量与字段写入（无阻塞、无分配热点）；快照为不可变克隆值。
//! 遵守基线 §11-5：指标只含时序/计数元数据，不含帧内容。

use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

/// PTS 时钟来源说明（快照引用）。
pub const CLOCK_SOURCE_PTS: &str = "GStreamer pipeline clock running-time (ns since PLAYING)";
/// 接收时间时钟来源说明（快照引用）。
pub const CLOCK_SOURCE_RECV: &str = "recv_monotonic_us = std::time::Instant since pipeline start (process-local monotonic); recv_wall_unix_ms = std::time::SystemTime UNIX epoch ms";

/// 帧头部哈希采样字节数（≤1024 字节；回调内唯一的缓冲读取，用于确定性构图验证）。
pub(crate) const FRAME_HEAD_HASH_BYTES: usize = 1024;

/// 单帧元数据（**不含像素内容**；`head_hash_fnv1a` 为首段 ≤1024 字节 FNV-1a 64）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameInfo {
    /// 接收序号（本管线内从 0 单调递增）。
    pub seq: u64,
    /// PTS（纳秒；管线时钟 running-time；无 PTS 记 0）。
    pub pts_ns: u64,
    /// 声明时长（纳秒；无时长记 0）。
    pub duration_ns: u64,
    /// 实际协商像素格式（如 "NV12"）。
    pub format: &'static str,
    /// 实际宽度。
    pub width: u32,
    /// 实际高度。
    pub height: u32,
    /// 实际平面数（NV12 = 2）。
    pub n_planes: u32,
    /// 各平面行跨距（字节；NV12 = [Y 行跨距, UV 行跨距, 0, 0]）。
    pub strides: [u64; 4],
    /// 各平面起始偏移（字节；NV12 = [0, Y 平面字节数, 0, 0]）。
    pub plane_offsets: [u64; 4],
    /// 映射总字节数（**不假定平面紧密排列**——以协商 stride/offset 为准）。
    pub map_size: u64,
    /// 接收时刻（SystemTime UNIX epoch 毫秒；墙钟，可能受系统时间调整影响）。
    pub recv_wall_unix_ms: u64,
    /// 接收时刻（管线启动以来 Instant 微秒；进程内单调钟，fps 计算用）。
    pub recv_monotonic_us: u64,
    /// 首段 ≤1024 字节 FNV-1a 64 哈希（确定性构图证据）。
    pub head_hash_fnv1a: u64,
}

/// 管线指标（内部原子计数；`snapshot()` 产出不可变快照）。
#[derive(Debug, Default)]
pub struct PipelineMetrics {
    next_seq: AtomicU64,
    frames_received: AtomicU64,
    queue_overruns: AtomicU64,
    pts_monotonic_violations: AtomicU64,
    caps_parse_failures: AtomicU64,
    map_size_mismatches: AtomicU64,
    duration_min_ns: AtomicU64,
    duration_max_ns: AtomicU64,
    first_pts_ns: AtomicI64,
    last_pts_ns: AtomicI64,
    first_frame_hash: AtomicU64,
    last_frame_hash: AtomicU64,
    first_recv_us: AtomicU64,
    last_recv_us: AtomicU64,
    started_unix_ms: AtomicU64,
    start_epoch: OnceLock<Instant>,
    last_frame: Mutex<Option<FrameInfo>>,
    last_error: Mutex<Option<String>>,
}

impl PipelineMetrics {
    /// 标记启动时刻（幂等：仅首个 start 生效）。
    pub fn mark_started(&self) {
        let _ = self.start_epoch.set(Instant::now());
        if self.started_unix_ms.load(Ordering::Relaxed) == 0 {
            self.started_unix_ms
                .store(wall_unix_ms(), Ordering::Relaxed);
        }
    }

    /// 重置 PTS 单调性作用域（每次 `start` 调用；重启后 running-time 归零，
    /// 跨周期比较会假报违例——单调性是**运行内**不变量）。
    pub fn reset_pts_scope(&self) {
        self.last_pts_ns.store(-1, Ordering::Relaxed);
        self.pts_monotonic_violations.store(0, Ordering::Relaxed);
    }

    /// 记录一帧（回调内调用；返回分配的 seq）。
    pub fn record_frame(&self, mut info: FrameInfo) -> u64 {
        info.seq = self.next_seq.fetch_add(1, Ordering::Relaxed);
        info.recv_monotonic_us = self
            .start_epoch
            .get_or_init(Instant::now)
            .elapsed()
            .as_micros() as u64;

        let pts = i64::try_from(info.pts_ns).unwrap_or(i64::MAX);
        let prev = self.last_pts_ns.swap(pts, Ordering::Relaxed);
        if prev >= 0 && pts < prev {
            self.pts_monotonic_violations
                .fetch_add(1, Ordering::Relaxed);
        }
        let _ = self
            .first_pts_ns
            .compare_exchange(-1, pts, Ordering::Relaxed, Ordering::Relaxed);

        if info.duration_ns > 0 {
            // 0 = 未观测到带时长帧（Default 初值）；首帧直接落地，其后取 min。
            let _ =
                self.duration_min_ns
                    .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |cur| {
                        Some(if cur == 0 {
                            info.duration_ns
                        } else {
                            cur.min(info.duration_ns)
                        })
                    });
            let _ =
                self.duration_max_ns
                    .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |cur| {
                        Some(cur.max(info.duration_ns))
                    });
        }

        let us = info.recv_monotonic_us;
        let _ =
            self.first_recv_us
                .compare_exchange(0, us.max(1), Ordering::Relaxed, Ordering::Relaxed);
        self.last_recv_us.store(us, Ordering::Relaxed);

        let _ = self.first_frame_hash.compare_exchange(
            0,
            info.head_hash_fnv1a,
            Ordering::Relaxed,
            Ordering::Relaxed,
        );
        self.last_frame_hash
            .store(info.head_hash_fnv1a, Ordering::Relaxed);

        self.frames_received.fetch_add(1, Ordering::Relaxed);
        if let Ok(mut slot) = self.last_frame.lock() {
            *slot = Some(info);
        }
        info.seq
    }

    /// 记录一次 queue overrun（丢帧证据之一）。
    pub fn record_queue_overrun(&self) {
        self.queue_overruns.fetch_add(1, Ordering::Relaxed);
    }

    /// 记录一次 caps 解析失败。
    pub fn record_caps_parse_failure(&self) {
        self.caps_parse_failures.fetch_add(1, Ordering::Relaxed);
    }

    /// 记录一次映射尺寸与协商布局不符。
    pub fn record_map_size_mismatch(&self) {
        self.map_size_mismatches.fetch_add(1, Ordering::Relaxed);
    }

    /// 记录管线错误消息（保留最近一条）。
    pub fn record_error(&self, msg: String) {
        if let Ok(mut slot) = self.last_error.lock() {
            *slot = Some(msg);
        }
    }

    /// 产出不可变快照（控制线程调用）。
    #[must_use]
    pub fn snapshot(&self) -> MetricsSnapshot {
        let frames_received = self.frames_received.load(Ordering::Relaxed);
        let first_us = self.first_recv_us.load(Ordering::Relaxed);
        let last_us = self.last_recv_us.load(Ordering::Relaxed);
        let actual_fps = if frames_received >= 2 && first_us > 0 && last_us > first_us {
            let frames_delta = (frames_received - 1) as f64;
            let span_us = (last_us - first_us) as f64;
            frames_delta * 1_000_000.0 / span_us
        } else {
            0.0
        };
        let last_frame = self.last_frame.lock().map(|g| *g).unwrap_or_default();
        let last_error = self
            .last_error
            .lock()
            .map(|g| g.clone())
            .unwrap_or_default();
        MetricsSnapshot {
            frames_received,
            actual_fps,
            queue_overruns: self.queue_overruns.load(Ordering::Relaxed),
            pts_monotonic_violations: self.pts_monotonic_violations.load(Ordering::Relaxed),
            caps_parse_failures: self.caps_parse_failures.load(Ordering::Relaxed),
            map_size_mismatches: self.map_size_mismatches.load(Ordering::Relaxed),
            duration_min_ns: self.duration_min_ns.load(Ordering::Relaxed),
            duration_max_ns: self.duration_max_ns.load(Ordering::Relaxed),
            first_pts_ns: self.first_pts_ns.load(Ordering::Relaxed),
            last_pts_ns: self.last_pts_ns.load(Ordering::Relaxed),
            first_frame_hash: self.first_frame_hash.load(Ordering::Relaxed),
            last_frame_hash: self.last_frame_hash.load(Ordering::Relaxed),
            started_unix_ms: self.started_unix_ms.load(Ordering::Relaxed),
            last_frame,
            last_error,
            clock_source_pts: CLOCK_SOURCE_PTS,
            clock_source_recv: CLOCK_SOURCE_RECV,
        }
    }
}

/// 指标快照（不可变；Clone 语义）。
#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    /// 已接收帧数（appsink 回调实际看到）。
    pub frames_received: u64,
    /// 实际帧率（首帧→末帧接收间隔估算；样本不足 2 帧为 0.0）。
    pub actual_fps: f64,
    /// queue overrun 计数（旧帧被丢弃的信号级证据）。
    pub queue_overruns: u64,
    /// PTS 单调性违例次数（**运行内**不变量；每次 `start` 重置作用域）。
    pub pts_monotonic_violations: u64,
    /// caps 解析失败次数。
    pub caps_parse_failures: u64,
    /// 映射尺寸与协商布局不符次数。
    pub map_size_mismatches: u64,
    /// 观测到的最小帧时长（纳秒；0 = 未观测到带时长帧）。
    pub duration_min_ns: u64,
    /// 观测到的最大帧时长（纳秒）。
    pub duration_max_ns: u64,
    /// 首帧 PTS（纳秒；-1 = 无帧）。
    pub first_pts_ns: i64,
    /// 末帧 PTS（纳秒；-1 = 无帧）。
    pub last_pts_ns: i64,
    /// 首帧头部哈希（确定性构图跨启动比对）。
    pub first_frame_hash: u64,
    /// 末帧头部哈希。
    pub last_frame_hash: u64,
    /// 启动墙钟时刻（UNIX epoch ms；0 = 未启动）。
    pub started_unix_ms: u64,
    /// 最近一帧元数据（None = 尚无帧）。
    pub last_frame: Option<FrameInfo>,
    /// 最近一条错误消息（None = 无错误）。
    pub last_error: Option<String>,
    /// PTS 时钟来源说明。
    pub clock_source_pts: &'static str,
    /// 接收时间时钟来源说明。
    pub clock_source_recv: &'static str,
}

/// 当前墙钟（UNIX epoch 毫秒；时钟异常记 0，不 panic）。
#[must_use]
pub(crate) fn wall_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// FNV-1a 64 位哈希（无依赖；确定性构图验证用）。
#[must_use]
pub(crate) fn fnv1a64(data: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in data {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{fnv1a64, FrameInfo, PipelineMetrics};

    fn sample_frame(pts_ns: u64) -> FrameInfo {
        FrameInfo {
            seq: 0,
            pts_ns,
            duration_ns: 33_333_333,
            format: "NV12",
            width: 1280,
            height: 720,
            n_planes: 2,
            strides: [1280, 1280, 0, 0],
            plane_offsets: [0, 921_600, 0, 0],
            map_size: 1_382_400,
            recv_wall_unix_ms: 1,
            recv_monotonic_us: 1,
            head_hash_fnv1a: 42,
        }
    }

    #[test]
    fn seq_assigns_monotonically() {
        let m = PipelineMetrics::default();
        let a = m.record_frame(sample_frame(0));
        let b = m.record_frame(sample_frame(33_333_333));
        assert_eq!(a + 1, b, "seq must be monotonically increasing");
        let s = m.snapshot();
        assert_eq!(s.frames_received, 2);
        assert_eq!(s.pts_monotonic_violations, 0);
        assert_eq!(s.last_frame.map(|f| f.seq), Some(b));
    }

    #[test]
    fn pts_regression_is_counted() {
        let m = PipelineMetrics::default();
        m.record_frame(sample_frame(100));
        m.record_frame(sample_frame(50));
        assert_eq!(m.snapshot().pts_monotonic_violations, 1);
    }

    #[test]
    fn duration_bounds_tracked() {
        let m = PipelineMetrics::default();
        m.record_frame(sample_frame(0));
        let mut f = sample_frame(33_333_333);
        f.duration_ns = 41_000_000;
        m.record_frame(f);
        let s = m.snapshot();
        assert_eq!(s.duration_min_ns, 33_333_333);
        assert_eq!(s.duration_max_ns, 41_000_000);
    }

    #[test]
    fn fnv1a64_known_vector() {
        // FNV-1a 64 of "a" = 0xaf63dc4c8601ec8c（公开测试向量）。
        assert_eq!(fnv1a64(b"a"), 0xaf63_dc4c_8601_ec8c);
    }

    #[test]
    fn fps_computed_from_two_frames() {
        let m = PipelineMetrics::default();
        m.mark_started();
        m.record_frame(sample_frame(0));
        std::thread::sleep(Duration::from_millis(20));
        m.record_frame(sample_frame(33_333_333));
        let s = m.snapshot();
        assert!(
            s.actual_fps > 0.0,
            "fps should be computable: {}",
            s.actual_fps
        );
    }
}
