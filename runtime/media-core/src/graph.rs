//! 媒体图规格骨架：声明式描述「要跑什么」，不绑定任何具体图实现。
//!
//! GStreamer 图由 gst-adapter 消费本类型构建；本 crate 不知道 GStreamer 存在。

use crate::policy::FramePolicy;

/// 图状态（与具体图引擎解耦的抽象状态）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphState {
    Detached,
    Building,
    Playing,
    Error,
}

/// 图规格骨架：恒定帧策略 + 占位标识。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaGraphSpec {
    policy: FramePolicy,
}

impl MediaGraphSpec {
    #[must_use]
    pub fn new(policy: FramePolicy) -> Self {
        Self { policy }
    }

    #[must_use]
    pub fn policy(&self) -> &FramePolicy {
        &self.policy
    }
}
