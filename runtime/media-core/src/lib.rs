//! media-core — 平台无关的媒体会话/图/策略/度量类型骨架。
//!
//! 设计边界（基线 §3 三平面解耦，ADR-001/002）：
//! - 本 crate **零外部依赖**，不得引入 Windows / GStreamer / IPC 具体实现；
//!   所有平台细节由 gst-adapter（GStreamer）与 device-bridge（设备桥接）承载。
//! - 本 crate 只承载类型与纯函数骨架；图调度、帧传输协议（Ring Buffer / 管道）
//!   属后续任务（M1-T2~T5），此处不实现（R-10 防过度设计）。

pub mod envelope;
pub mod graph;
pub mod metrics;
pub mod policy;
pub mod session;

/// FrameEnvelope v1 契约的公开再导出（M1-T3；契约文档
/// `contracts/ipc/frame-envelope-v0.1-draft.md`，金样本
/// `tests/contract/golden/frame_envelope_v1.txt`）。
pub mod prelude {
    pub use crate::envelope::{
        error_kind, DecodeError, EnvelopePixelFormat, FrameEnvelope, PlaneLayout, HEADER_SIZE_V1,
        MAGIC, MAX_PLANES,
    };
}

/// Crate 级编译期健全性断言：骨架类型可构造、可比较（无隐藏依赖泄漏）。
#[cfg(test)]
mod sanity {
    use crate::graph::MediaGraphSpec;
    use crate::policy::FramePolicy;
    use crate::session::{MediaSession, SessionState};

    #[test]
    fn skeleton_types_compose() {
        let spec = MediaGraphSpec::new(FramePolicy::DEFAULT);
        let session = MediaSession::new(spec);
        assert_eq!(session.state(), SessionState::New);
        assert_eq!(session.spec().policy(), &FramePolicy::DEFAULT);
    }
}
