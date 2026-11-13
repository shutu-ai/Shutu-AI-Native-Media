//! 度量快照骨架：帧计数 / 丢帧 / 时间戳断层。
//!
//! 只定义数据形状；采集与上报在后续任务接线（M1-T2 起）。
//! 约束：日志与指标只含元数据，永不含帧内容（基线 §11-5）。

/// 度量快照（不可变；采样点产生新值，不原地累积）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MetricsSnapshot {
    pub frames_rendered: u64,
    pub frames_dropped: u64,
    /// 连续无帧事件计数（供断连检测，NFR-06 预演）。
    pub stall_events: u64,
}

impl MetricsSnapshot {
    /// 累积另一快照（返回新值）。
    #[must_use]
    pub fn merged_with(&self, other: &Self) -> Self {
        Self {
            frames_rendered: self.frames_rendered + other.frames_rendered,
            frames_dropped: self.frames_dropped + other.frames_dropped,
            stall_events: self.stall_events + other.stall_events,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::MetricsSnapshot;

    #[test]
    fn merge_is_pure_and_additive() {
        let a = MetricsSnapshot {
            frames_rendered: 10,
            frames_dropped: 1,
            stall_events: 0,
        };
        let b = MetricsSnapshot {
            frames_rendered: 5,
            frames_dropped: 2,
            stall_events: 1,
        };
        let merged = a.merged_with(&b);
        assert_eq!(merged.frames_rendered, 15);
        assert_eq!(merged.frames_dropped, 3);
        assert_eq!(merged.stall_events, 1);
        // 不可变性：原值不变
        assert_eq!(a.frames_rendered, 10);
    }
}
