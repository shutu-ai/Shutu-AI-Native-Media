//! 媒体会话骨架：一个用户可见的媒体会话 = 图规格 + 生命周期状态。
//!
//! 仅类型与状态机枚举；真正的会话调度在后续任务实现（M1-T2 起）。

/// 会话标识（进程内唯一；跨进程标识属 contracts/ 契约层，此处不定义）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SessionId(pub u64);

/// 会话生命周期状态（骨架枚举；迁移规则随 M1-T2 具体化）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    New,
    Starting,
    Running,
    Degraded,
    Stopping,
    Ended,
}

/// 媒体会话骨架。
#[derive(Debug, Clone)]
pub struct MediaSession {
    id: SessionId,
    state: SessionState,
    spec: crate::graph::MediaGraphSpec,
}

impl MediaSession {
    /// 以恒定策略创建新会话（骨架：状态恒为 [`SessionState::New`]）。
    #[must_use]
    pub fn new(spec: crate::graph::MediaGraphSpec) -> Self {
        Self {
            id: SessionId(0),
            state: SessionState::New,
            spec,
        }
    }

    #[must_use]
    pub fn id(&self) -> SessionId {
        self.id
    }

    #[must_use]
    pub fn state(&self) -> SessionState {
        self.state
    }

    #[must_use]
    pub fn spec(&self) -> &crate::graph::MediaGraphSpec {
        &self.spec
    }
}
