//! device-bridge — 虚拟设备桥接空壳（M1-T1 骨架）。
//!
//! 架构守卫（本 crate 的首要职责，见 ADR-012 与基线 §5.1）：
//! 1. **传输策略不得硬编码为共享内存唯一方式**——ADR-012 已裁定控制通道首选
//!    Named Pipe、帧承载待 M1-T4/T5 实测（P1-3，技术债 OPEN）后定案；
//!    共享内存 broker 方案仅作备选且需新 ADR。
//! 2. 不提前实现媒体 IPC 协议、Ring Buffer 或 Frame Server 客户端（R-10；
//!    跨进程传输归属 M1-T4/T5）。M1-T3 新增 [`framebuffer`] 为**进程内、
//!    传输无关**的有界帧缓冲抽象——它不是跨进程验证的替代品。
//! 3. 不复制 M0-T4 PoC 的异步取消实现（P1-2 风险登记在案）。

pub mod framebuffer;

/// M1-T3 有界帧缓冲的公开再导出（传输无关；跨进程承载见 M1-T4/T5）。
pub mod prelude {
    pub use crate::framebuffer::{
        BoundedFrameBuffer, BufferLevels, BufferLimits, FrameBuffer, OverfullPolicy, PushError,
        ReadError,
    };
}

/// 帧承载传输候选（架构枚举；**仅类型守卫，无任何实现**）。
///
/// 选择规则（ADR-012 D2）：默认候选 = [`TransportKind::NamedPipe`]（帧随控制通道）；
/// [`TransportKind::SharedMemory`] 需实测吞吐不足且引入时必须新 ADR；
/// 实测达标前任何一种都**不得标记为生产可用**。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportKind {
    /// ADR-012 首选：帧复用 Named Pipe 控制通道（DACL = S-1-5-19 显式授权）。
    NamedPipe,
    /// 备选：LS 特权 broker 创建 Global\ section（ADR-012 U5/U6 未验证假设）。
    SharedMemory,
    /// 预留：其他承载（如独立 broker 进程）——引入须新 ADR。
    Proxy,
}

/// 传输策略声明（骨架：只记录「当前评估中的候选」，不做选择、不做实现）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransportPolicy {
    pub preferred: TransportKind,
    /// 是否已有实测数据支持该偏好（M1-T4/T5 前恒为 false —— P1-3）。
    pub measurement_backed: bool,
}

impl TransportPolicy {
    /// ADR-012 缺省策略：管道首选、无实测背书（P1-3 OPEN 的类型化表达）。
    pub const ADR012_DEFAULT: Self = Self {
        preferred: TransportKind::NamedPipe,
        measurement_backed: false,
    };
}

#[cfg(test)]
mod tests {
    use super::{TransportKind, TransportPolicy};

    /// 架构守卫：缺省策略必须是「管道首选 + 无实测背书」——
    /// 若未来有人把缺省改成 SharedMemory-only 或声称已有实测，此测试即失败。
    #[test]
    fn default_policy_matches_adr012() {
        let p = TransportPolicy::ADR012_DEFAULT;
        assert_eq!(p.preferred, TransportKind::NamedPipe);
        assert!(!p.measurement_backed, "P1-3 未实测前不得声称有背书");
    }

    /// 传输候选含共享内存但**不以其为唯一/缺省**（ADR-012 架构红线）。
    #[test]
    fn shared_memory_is_option_not_default() {
        let all = [
            TransportKind::NamedPipe,
            TransportKind::SharedMemory,
            TransportKind::Proxy,
        ];
        assert!(all.contains(&TransportKind::SharedMemory));
        assert_ne!(
            TransportPolicy::ADR012_DEFAULT.preferred,
            TransportKind::SharedMemory
        );
    }
}
