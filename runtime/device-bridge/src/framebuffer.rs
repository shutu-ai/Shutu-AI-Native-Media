//! 有界帧缓冲抽象（M1-T3：**传输无关**，进程内语义 + 契约级并发纪律）。
//!
//! 职责边界（M1_TASKS M1-T3 修订后定义 / AGENTS §4）：
//! - 只定义「生产者 push / 消费者 read_latest」的**有界缓冲语义**：
//!   帧数上限 + 字节上限、满载策略（拒绝新帧 / 逐出最旧）、代际推进重置序号、
//!   重放拒绝、旧代际读取拒绝。契约测试见 `tests/contract`。
//! - **不实现任何跨进程传输**（禁止 Windows Global Section、共享内存 Ring、
//!   特权 Broker、产品级 Named Pipe）——真实跨进程承载归 M1-T4/T5，
//!   不得以本抽象冒充已完成（不得通过 Mock 假装跨进程已验证）。
//! - 临界区内无 I/O、无分配重负载；`Mutex` 仅保护内部队列（毒化时恢复内部
//!   状态继续运行，不 panic）。
//! - 帧描述符为 `media_core::FrameEnvelope`（校验过的封装头）；payload 本体
//!   留在传输层，缓冲只按 `payload_len` 做字节预算。

use std::collections::VecDeque;
use std::fmt;
use std::sync::Mutex;

use media_core::envelope::{DecodeError, FrameEnvelope};

/// 缓冲上限（帧数 + 字节双预算；NFR-07 有界性的类型化表达）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BufferLimits {
    /// 队列内最大帧数。
    pub max_frames: usize,
    /// 队列内 payload 字节预算上限（按各帧 `payload_len` 累计）。
    pub max_bytes: u64,
}

impl Default for BufferLimits {
    /// 4 帧 / 32 MiB（与 M1-T2 背压实测的 queue-max=4 对齐；32 MiB ≈ 23 帧 NV12 720p）。
    fn default() -> Self {
        Self {
            max_frames: 4,
            max_bytes: 32 * 1024 * 1024,
        }
    }
}

/// 满载策略。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OverfullPolicy {
    /// 拒绝新帧（背压上游）——M1-T2 appsink 路径语义。
    #[default]
    RejectNew,
    /// 逐出最旧帧直到新帧能放入（消费端停摆时保新鲜度）。
    DropOldest,
}

/// push 失败原因。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PushError {
    /// 封装头未通过 FrameEnvelope 校验。
    InvalidEnvelope(DecodeError),
    /// 代际倒退（旧代际的生产者帧）。
    GenerationRejected {
        /// 缓冲当前最新代际。
        latest: u64,
        /// 被拒帧的代际。
        got: u64,
    },
    /// 同代际内序号回退/重复（重放）。
    SeqReplay {
        /// 该代际已见最大序号。
        last_seq: u64,
        /// 被拒帧的序号。
        got: u64,
    },
    /// 帧数上限且策略为 RejectNew。
    TooManyFrames {
        /// 上限。
        limit: usize,
    },
    /// 字节预算不足（新帧单独就超预算，或 DropOldest 逐空后仍放不下）。
    TooManyBytes {
        /// 预算上限。
        limit: u64,
        /// 新帧所需字节。
        needed: u64,
    },
}

impl fmt::Display for PushError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::InvalidEnvelope(ref e) => write!(f, "invalid envelope: {e}"),
            Self::GenerationRejected { latest, got } => {
                write!(f, "stale generation {got} (latest {latest})")
            }
            Self::SeqReplay { last_seq, got } => {
                write!(f, "seq replay {got} (last seen {last_seq})")
            }
            Self::TooManyFrames { limit } => write!(f, "frame limit {limit} reached"),
            Self::TooManyBytes { limit, needed } => {
                write!(f, "byte budget {limit} exceeded, frame needs {needed}")
            }
        }
    }
}

impl std::error::Error for PushError {}

/// read_latest 失败原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadError {
    /// 缓冲为空。
    Empty,
    /// 最新帧代际与请求代际不符（旧代际拒绝；超前代际同样不匹配）。
    StaleGeneration {
        /// 缓冲内最新帧的代际。
        latest: u64,
        /// 请求的代际。
        requested: u64,
    },
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Empty => write!(f, "buffer empty"),
            Self::StaleGeneration { latest, requested } => {
                write!(f, "latest generation {latest} != requested {requested}")
            }
        }
    }
}

impl std::error::Error for ReadError {}

/// 缓冲水位（观测用；perf/指标上报的数据源）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BufferLevels {
    /// 当前帧数。
    pub frames: usize,
    /// 当前 payload 字节合计。
    pub bytes: u64,
    /// 累计逐出帧数（DropOldest 策略的丢弃可观测性；RejectNew 恒 0）。
    pub dropped: u64,
}

/// 传输无关的有界帧缓冲接口（M1-T4/T5 的承载实现将复用同一语义）。
pub trait FrameBuffer: Send {
    /// 压入一帧（封装头）。校验 → 代际/序号检查 → 容量策略。
    ///
    /// # Errors
    /// [`PushError`] 各变体；成功返回压入后的水位。
    fn push(&self, envelope: FrameEnvelope) -> Result<BufferLevels, PushError>;

    /// 读取缓冲内最新一帧（不移除），并要求其代际等于 `generation`。
    ///
    /// # Errors
    /// [`ReadError::Empty`] / [`ReadError::StaleGeneration`]。
    fn read_latest(&self, generation: u64) -> Result<FrameEnvelope, ReadError>;

    /// 当前水位。
    fn levels(&self) -> BufferLevels;
}

/// `FrameBuffer` 的默认实现：`Mutex<VecDeque>` 保护的定点容量队列。
pub struct BoundedFrameBuffer {
    limits: BufferLimits,
    policy: OverfullPolicy,
    inner: Mutex<Inner>,
}

struct Inner {
    queue: VecDeque<FrameEnvelope>,
    bytes: u64,
    /// 累计逐出帧数（丢弃可观测）。
    dropped: u64,
    /// 已见最新代际（首次 push 前为 None）。
    generation: Option<u64>,
    /// 当前代际内已见最大序号（代际推进时重置为 0）。
    last_seq: u64,
}

impl BoundedFrameBuffer {
    /// 以显式上限与策略构造。
    #[must_use]
    pub fn new(limits: BufferLimits, policy: OverfullPolicy) -> Self {
        Self {
            limits,
            policy,
            inner: Mutex::new(Inner {
                queue: VecDeque::new(),
                bytes: 0,
                dropped: 0,
                generation: None,
                last_seq: 0,
            }),
        }
    }

    /// 缺省上限（4 帧 / 32 MiB）+ 缺省策略（RejectNew）。
    #[must_use]
    pub fn with_defaults() -> Self {
        Self::new(BufferLimits::default(), OverfullPolicy::default())
    }

    /// 毒化恢复：锁毒化时取回内部状态继续（缓冲语义不受 panic 传播破坏）。
    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl FrameBuffer for BoundedFrameBuffer {
    fn push(&self, envelope: FrameEnvelope) -> Result<BufferLevels, PushError> {
        // validate() 通过即封装头合法；payload_len 允许大于布局最小值（尾部余量），
        // 字节预算按声明的 payload_len 计。
        envelope.validate().map_err(PushError::InvalidEnvelope)?;
        let needed = u64::from(envelope.payload_len);
        if needed > self.limits.max_bytes {
            return Err(PushError::TooManyBytes {
                limit: self.limits.max_bytes,
                needed,
            });
        }

        let mut inner = self.lock();
        match inner.generation {
            Some(g) if envelope.generation < g => {
                return Err(PushError::GenerationRejected {
                    latest: g,
                    got: envelope.generation,
                });
            }
            Some(g) if envelope.generation == g => {
                if envelope.seq == 0 || envelope.seq <= inner.last_seq {
                    return Err(PushError::SeqReplay {
                        last_seq: inner.last_seq,
                        got: envelope.seq,
                    });
                }
            }
            _ => {
                // 新代际（含首帧）：代际推进，序号表重置。
                inner.generation = Some(envelope.generation);
                inner.last_seq = 0;
            }
        }

        // 容量策略：先按策略腾位，再检查字节预算。
        if self.policy == OverfullPolicy::RejectNew {
            if inner.queue.len() >= self.limits.max_frames {
                return Err(PushError::TooManyFrames {
                    limit: self.limits.max_frames,
                });
            }
            if inner.bytes + needed > self.limits.max_bytes {
                return Err(PushError::TooManyBytes {
                    limit: self.limits.max_bytes,
                    needed: inner.bytes + needed,
                });
            }
        } else {
            // DropOldest：逐出最旧直到「帧数 < 上限 且 放入后字节不超预算」。
            while inner.queue.len() >= self.limits.max_frames
                || inner.bytes + needed > self.limits.max_bytes
            {
                let Some(evicted) = inner.queue.pop_front() else {
                    break;
                };
                inner.bytes -= u64::from(evicted.payload_len);
                inner.dropped += 1;
            }
            debug_assert!(inner.bytes + needed <= self.limits.max_bytes);
        }

        inner.last_seq = envelope.seq;
        inner.queue.push_back(envelope);
        inner.bytes += needed;
        Ok(BufferLevels {
            frames: inner.queue.len(),
            bytes: inner.bytes,
            dropped: inner.dropped,
        })
    }

    fn read_latest(&self, generation: u64) -> Result<FrameEnvelope, ReadError> {
        let inner = self.lock();
        let back = inner.queue.back().ok_or(ReadError::Empty)?;
        if back.generation != generation {
            return Err(ReadError::StaleGeneration {
                latest: back.generation,
                requested: generation,
            });
        }
        Ok(*back)
    }

    fn levels(&self) -> BufferLevels {
        let inner = self.lock();
        BufferLevels {
            frames: inner.queue.len(),
            bytes: inner.bytes,
            dropped: inner.dropped,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BoundedFrameBuffer, BufferLimits, FrameBuffer, OverfullPolicy, PushError, ReadError,
    };
    use media_core::envelope::{EnvelopePixelFormat, FrameEnvelope, PlaneLayout};

    /// 4x4 BGRA 最小合法封装（stride 16，payload 64 字节）——测试用微帧。
    fn bgra_tiny(seq: u64, generation: u64) -> FrameEnvelope {
        FrameEnvelope {
            seq,
            generation,
            pts_ns: seq * 33_333_333,
            duration_ns: 33_333_333,
            graph_version: 1,
            format: EnvelopePixelFormat::Bgra,
            width: 4,
            height: 4,
            planes: [
                PlaneLayout {
                    stride: 16,
                    offset: 0,
                },
                PlaneLayout::ZERO,
                PlaneLayout::ZERO,
                PlaneLayout::ZERO,
            ],
            payload_len: 64,
        }
    }

    fn push_ok(buf: &BoundedFrameBuffer, env: FrameEnvelope) -> super::BufferLevels {
        match buf.push(env) {
            Ok(levels) => levels,
            Err(e) => panic!("push must succeed: {e}"),
        }
    }

    fn push_err(buf: &BoundedFrameBuffer, env: FrameEnvelope) -> PushError {
        match buf.push(env) {
            Ok(_) => panic!("push must be rejected"),
            Err(e) => e,
        }
    }

    #[test]
    fn invalid_envelope_rejected() {
        let buf = BoundedFrameBuffer::with_defaults();
        let mut env = bgra_tiny(1, 1);
        env.width = 0;
        match push_err(&buf, env) {
            PushError::InvalidEnvelope(_) => {}
            other => panic!("wrong variant: {other:?}"),
        }
        assert_eq!(buf.levels(), super::BufferLevels::default());
    }

    #[test]
    fn seq_monotonic_and_replay_rejected() {
        let buf = BoundedFrameBuffer::with_defaults();
        push_ok(&buf, bgra_tiny(1, 1));
        push_ok(&buf, bgra_tiny(2, 1));
        push_ok(&buf, bgra_tiny(3, 1));
        // 重放：重复旧序号 / 序号 0。
        match push_err(&buf, bgra_tiny(2, 1)) {
            PushError::SeqReplay { last_seq, got } => assert_eq!((last_seq, got), (3, 2)),
            other => panic!("wrong variant: {other:?}"),
        }
        match push_err(&buf, bgra_tiny(0, 1)) {
            PushError::SeqReplay { got, .. } => assert_eq!(got, 0),
            other => panic!("wrong variant: {other:?}"),
        }
        // 序号跳空允许（上游丢帧不属重放）。
        push_ok(&buf, bgra_tiny(10, 1));
    }

    #[test]
    fn generation_advance_resets_seq() {
        let buf = BoundedFrameBuffer::with_defaults();
        push_ok(&buf, bgra_tiny(1, 1));
        push_ok(&buf, bgra_tiny(2, 1));
        // 新代际从序号 1 重新开始——合法。
        push_ok(&buf, bgra_tiny(1, 2));
        // 旧代际帧被拒。
        match push_err(&buf, bgra_tiny(3, 1)) {
            PushError::GenerationRejected { latest, got } => {
                assert_eq!((latest, got), (2, 1));
            }
            other => panic!("wrong variant: {other:?}"),
        }
    }

    #[test]
    fn stale_generation_read_rejected() {
        let buf = BoundedFrameBuffer::with_defaults();
        match buf.read_latest(1) {
            Err(ReadError::Empty) => {}
            other => panic!("expected Empty, got {other:?}"),
        }
        push_ok(&buf, bgra_tiny(1, 2));
        match buf.read_latest(1) {
            Err(ReadError::StaleGeneration { latest, requested }) => {
                assert_eq!((latest, requested), (2, 1));
            }
            other => panic!("expected StaleGeneration, got {other:?}"),
        }
        let latest = buf
            .read_latest(2)
            .unwrap_or_else(|e| panic!("read_latest(2) must succeed: {e}"));
        assert_eq!(latest.seq, 1);
    }

    #[test]
    fn reject_new_policy_when_full() {
        let buf = BoundedFrameBuffer::new(
            BufferLimits {
                max_frames: 2,
                max_bytes: 1024,
            },
            OverfullPolicy::RejectNew,
        );
        push_ok(&buf, bgra_tiny(1, 1));
        push_ok(&buf, bgra_tiny(2, 1));
        match push_err(&buf, bgra_tiny(3, 1)) {
            PushError::TooManyFrames { limit } => assert_eq!(limit, 2),
            other => panic!("wrong variant: {other:?}"),
        }
        // 拒绝不改变水位与序号表。
        assert_eq!(buf.levels().frames, 2);
        match push_err(&buf, bgra_tiny(2, 1)) {
            PushError::SeqReplay { got, .. } => assert_eq!(got, 2),
            other => panic!("rejected frame must not advance seq table: {other:?}"),
        }
    }

    #[test]
    fn drop_oldest_policy_evicts_until_fit() {
        let buf = BoundedFrameBuffer::new(
            BufferLimits {
                max_frames: 2,
                max_bytes: 1024,
            },
            OverfullPolicy::DropOldest,
        );
        push_ok(&buf, bgra_tiny(1, 1));
        push_ok(&buf, bgra_tiny(2, 1));
        let levels = push_ok(&buf, bgra_tiny(3, 1));
        assert_eq!(levels.frames, 2);
        assert_eq!(levels.bytes, 128);
        assert_eq!(levels.dropped, 1, "丢弃可观测：逐出恰 1 帧");
        let latest = buf
            .read_latest(1)
            .unwrap_or_else(|e| panic!("read_latest must succeed: {e}"));
        assert_eq!(latest.seq, 3);
    }

    #[test]
    fn byte_cap_rejects_single_oversized_frame() {
        let buf = BoundedFrameBuffer::new(
            BufferLimits {
                max_frames: 4,
                max_bytes: 32,
            },
            OverfullPolicy::DropOldest,
        );
        // 单帧 64 字节 > 32 字节预算：即使 DropOldest 也无处可逃。
        match push_err(&buf, bgra_tiny(1, 1)) {
            PushError::TooManyBytes { limit, needed } => {
                assert_eq!((limit, needed), (32, 64));
            }
            other => panic!("wrong variant: {other:?}"),
        }
    }

    #[test]
    fn drop_oldest_frees_bytes_for_large_frame() {
        // 预算恰好 2 帧（128 字节）：新帧到位前须逐出 1 帧。
        let buf = BoundedFrameBuffer::new(
            BufferLimits {
                max_frames: 8,
                max_bytes: 128,
            },
            OverfullPolicy::DropOldest,
        );
        push_ok(&buf, bgra_tiny(1, 1));
        push_ok(&buf, bgra_tiny(2, 1));
        let levels = push_ok(&buf, bgra_tiny(3, 1));
        assert_eq!(levels.frames, 2);
        assert_eq!(levels.bytes, 128);
        assert_eq!(levels.dropped, 1);
        assert_eq!(
            buf.levels().dropped,
            1,
            "水位查询与 push 返回值共享同一累计计数"
        );
    }

    #[test]
    fn default_limits_match_m1t2_backpressure_anchor() {
        let limits = BufferLimits::default();
        assert_eq!(limits.max_frames, 4, "与 M1-T2 背压实测 queue-max=4 对齐");
        assert_eq!(limits.max_bytes, 33_554_432);
    }

    #[test]
    fn trait_object_send_across_threads() {
        // 对象安全 + Send（M1-T4/T5 的承载实现将以 dyn FrameBuffer 跨线程交付）。
        fn assert_send<T: Send>() {}
        assert_send::<BoundedFrameBuffer>();
        assert_send::<Box<dyn FrameBuffer>>();
        let buf: Box<dyn FrameBuffer> = Box::new(BoundedFrameBuffer::with_defaults());
        let levels = std::thread::spawn(move || {
            buf.push(bgra_tiny(1, 1))
                .unwrap_or_else(|e| panic!("push through dyn must succeed: {e}"))
        })
        .join()
        .unwrap_or_else(|_| panic!("worker thread must not panic"));
        assert_eq!(levels.frames, 1);
        assert_eq!(levels.bytes, 64);
    }
}
