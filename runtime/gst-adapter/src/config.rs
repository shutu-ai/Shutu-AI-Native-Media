//! 管线配置（不可变；变更走重建——不可变设计约定）。
//!
//! 所有缓冲队列上限在此显式声明（基线 §3.4：所有缓冲必须有界）。
//! `consume_delay_us` 是**测试专用**背压注入钩子：默认 0（回调零额外耗时），
//! 仅限集成测试故意放慢消费以验证丢帧策略；生产路径恒为 0（回调不做任何耗时工作）。

use std::sync::atomic::AtomicU64;
use std::sync::Arc;

use media_core::policy::FramePolicy;

/// 测试源合成图案（确定性：静态 SMPTE 彩条——每帧构图恒定，不随时间变化）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestPattern {
    /// SMPTE 彩条（静态；确定性合成构图，首段字节哈希跨启动稳定）。
    Smpte,
}

impl TestPattern {
    /// videotestsrc pattern 属性的枚举字符串值。
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Smpte => "smpte",
        }
    }
}

/// 视频测试管线配置。
///
/// `consume_delay_us` 为共享句柄（Clone 共享同一原子量），可在运行中调整。
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    /// 视频规格（固定 caps 来源；默认 [`FramePolicy::TARGET_720P30`] = 1280x720@30 NV12）。
    pub policy: FramePolicy,
    /// 合成图案（确定性构图）。
    pub pattern: TestPattern,
    /// queue max-size-buffers（缓冲条数上限）。
    pub queue_max_buffers: u32,
    /// queue max-size-bytes（字节上限）。
    pub queue_max_bytes: u32,
    /// queue max-size-time（纳秒上限）。
    pub queue_max_time_ns: u64,
    /// appsink 内部缓冲上限（条数）。
    pub appsink_max_buffers: u32,
    /// 测试专用消费延迟钩子（微秒；0=正常路径）。
    pub consume_delay_us: Arc<AtomicU64>,
    /// 错误路径测试注入：`Some(n)` 时在 queue 与 appsink 间插入 identity(error-after=n)。
    /// 生产路径恒为 `None`（仅由错误路径集成测试使用）。
    pub inject_error_after_frames: Option<u32>,
}

impl PipelineConfig {
    /// 有界缓冲默认值：queue 4 条 / 8 MiB / 500 ms + leaky；appsink 2 条 + drop。
    /// 视频规格 = TARGET_720P30（1280x720@30 NV12）。
    #[must_use]
    pub fn default_720p30() -> Self {
        Self {
            policy: FramePolicy::TARGET_720P30,
            pattern: TestPattern::Smpte,
            queue_max_buffers: 4,
            queue_max_bytes: 8 * 1024 * 1024,
            queue_max_time_ns: 500_000_000,
            appsink_max_buffers: 2,
            consume_delay_us: Arc::new(AtomicU64::new(0)),
            inject_error_after_frames: None,
        }
    }
}
