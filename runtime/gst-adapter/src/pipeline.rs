//! 可编程视频测试管线（M1-T2）：
//! `videotestsrc(is-live, 固定 pattern) → videoconvert → capsfilter → queue → [identity?] → appsink`。
//!
//! 线程模型：
//! - 控制 API（`start` / `stop` / `send_eos` / `wait_for_eos_or_error`）从用户线程调用，
//!   生命周期状态由 `Mutex` 保护；
//! - 帧回调在 GStreamer streaming 线程执行——只做原子计数与 ≤1024 字节头部哈希，
//!   不做推理 / 网络 / 帧内容拷贝 / 无限阻塞（基线 §3.4）；`consume_delay_us` 是
//!   测试专用背压注入（默认 0）；
//! - 所有等待均带超时（默认 5 s），无无限阻塞（AGENTS §5）。
//!
//! 缓冲上限（基线 §3.4）：queue（max-size-buffers/bytes/time + leaky=downstream）与
//! appsink（max-buffers + drop）双层有界；下游变慢时旧帧在 queue 被丢弃并触发
//! overrun 计数——内存不随积压增长（集成测试断言）。
//!
//! **CI 级证据声明**：本管线是进程内 GStreamer 测试，不代表 OS 虚拟摄像头验证
//! （AGENTS §4 / M1-T2 字段 7）。

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use gstreamer::prelude::*;
use gstreamer::{ClockTime, Element, State, StateChangeSuccess};

use media_core::policy::FramePolicy;

use crate::caps::caps_for;
use crate::config::{PipelineConfig, TestPattern};
use crate::metrics::{FrameInfo, MetricsSnapshot, PipelineMetrics, FRAME_HEAD_HASH_BYTES};
use crate::AdapterError;

/// 状态变化确认超时（秒）——禁止无限等待。
const STATE_CHANGE_TIMEOUT_S: u64 = 5;
/// EOS / 错误等待的默认上限由调用方传入；此处仅限定 consume delay 钩子的上限。
const MAX_CONSUME_DELAY_US: u64 = 1_000_000;

/// 管线生命周期状态（明确三态 + 错误态；迁移只经控制 API）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LifecycleState {
    /// 已构建，未启动。
    Created,
    /// PLAYING（已确认到达）。
    Playing,
    /// 已停止（NULL 已确认；重复 stop 幂等）。
    Stopped,
    /// 错误态（携带最近错误消息；stop 仍可执行以释放资源）。
    Error(String),
}

/// EOS / 错误等待结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EosOutcome {
    /// 收到 EOS（正常结束）。
    Eos,
    /// 收到 bus ERROR（消息文本）。
    Error(String),
    /// 超时未收到 EOS/ERROR。
    Timeout,
}

/// 视频测试管线句柄（Clone 不共享；同一实例的控制 API 可跨线程调用）。
pub struct TestVideoPipeline {
    pipeline: gstreamer::Pipeline,
    queue: Element,
    metrics: Arc<PipelineMetrics>,
    state: Mutex<LifecycleState>,
    config: PipelineConfig,
}

impl TestVideoPipeline {
    /// 构建管线（CREATED 态；未启动）。
    ///
    /// # Errors
    /// GStreamer 初始化失败、factory 缺失、元素构建/链接失败时返回错误。
    pub fn build(config: PipelineConfig) -> Result<Self, AdapterError> {
        let _ = crate::init()?;
        let caps = caps_for(&config.policy)?;

        let src = plain_element("videotestsrc")?;
        src.set_property("is-live", true);
        src.set_property_from_str("pattern", config.pattern.as_str());
        let convert = plain_element("videoconvert")?;
        let capsfilter = plain_element("capsfilter")?;
        capsfilter.set_property("caps", caps.clone());
        let queue = plain_element("queue")?;
        queue.set_property("max-size-buffers", config.queue_max_buffers);
        queue.set_property("max-size-bytes", config.queue_max_bytes);
        queue.set_property("max-size-time", config.queue_max_time_ns);
        // leaky=downstream：满时丢弃最旧缓冲（明确丢帧策略；内存不随积压增长）。
        queue.set_property_from_str("leaky", "downstream");

        let appsink = plain_element("appsink")?;
        let appsink_ref = appsink
            .clone()
            .downcast::<gstreamer_app::AppSink>()
            .map_err(|_| AdapterError("appsink downcast failed".to_string()))?;
        appsink_ref.set_caps(Some(&caps));
        appsink_ref.set_max_buffers(config.appsink_max_buffers);
        appsink_ref.set_drop(true);
        appsink_ref.set_sync(true);

        let metrics = Arc::new(PipelineMetrics::default());
        let m_cb = Arc::clone(&metrics);
        let delay_cb = Arc::clone(&config.consume_delay_us);
        appsink_ref.set_callbacks(
            gstreamer_app::AppSinkCallbacks::builder()
                .new_sample(move |sink| on_new_sample(sink, &m_cb, &delay_cb))
                .build(),
        );

        let m_overrun = Arc::clone(&metrics);
        queue.connect("overrun", false, move |_| {
            m_overrun.record_queue_overrun();
            None
        });

        let pipeline = gstreamer::Pipeline::new();
        pipeline
            .add_many([&src, &convert, &capsfilter, &queue])
            .map_err(|e| AdapterError(format!("add elements failed: {e}")))?;
        pipeline
            .add(&appsink)
            .map_err(|e| AdapterError(format!("add appsink failed: {e}")))?;
        Element::link_many([&src, &convert, &capsfilter, &queue])
            .map_err(|e| AdapterError(format!("link upstream failed: {e}")))?;
        match config.inject_error_after_frames {
            None => {
                queue
                    .link(&appsink)
                    .map_err(|e| AdapterError(format!("link queue→appsink failed: {e}")))?;
            }
            Some(n) => {
                // 仅错误路径测试注入：queue → identity(error-after=n) → appsink。
                let identity = plain_element("identity")?;
                identity.set_property("error-after", i32::try_from(n).unwrap_or(i32::MAX));
                pipeline
                    .add(&identity)
                    .map_err(|e| AdapterError(format!("add identity failed: {e}")))?;
                queue
                    .link(&identity)
                    .map_err(|e| AdapterError(format!("link queue→identity failed: {e}")))?;
                identity
                    .link(&appsink)
                    .map_err(|e| AdapterError(format!("link identity→appsink failed: {e}")))?;
            }
        }

        Ok(Self {
            pipeline,
            queue,
            metrics,
            state: Mutex::new(LifecycleState::Created),
            config,
        })
    }

    /// 视频规格（只读）。
    #[must_use]
    pub fn policy(&self) -> FramePolicy {
        self.config.policy
    }

    /// queue 缓冲条数上限（只读）。
    #[must_use]
    pub fn queue_max_buffers(&self) -> u32 {
        self.config.queue_max_buffers
    }

    /// appsink 缓冲条数上限（只读）。
    #[must_use]
    pub fn appsink_max_buffers(&self) -> u32 {
        self.config.appsink_max_buffers
    }

    /// 合成图案（只读）。
    #[must_use]
    pub fn pattern(&self) -> TestPattern {
        self.config.pattern
    }

    /// 生命周期状态（克隆快照）。
    ///
    /// # Errors
    /// 状态锁中毒时返回错误。
    pub fn state(&self) -> Result<LifecycleState, AdapterError> {
        self.lock_state().map(|g| g.clone())
    }

    /// 启动到 PLAYING（带 5 s 状态确认超时；Stopped 后可再次启动）。
    ///
    /// # Errors
    /// 已在 PLAYING、状态切换失败或超时、状态锁中毒时返回错误。
    pub fn start(&self) -> Result<(), AdapterError> {
        {
            let st = self.lock_state()?;
            if *st == LifecycleState::Playing {
                return Err(AdapterError("pipeline already PLAYING".to_string()));
            }
        }
        self.pipeline
            .set_state(State::Playing)
            .map_err(|e| self.fail_start(format!("set PLAYING failed: {e}")))?;
        let (ret, current, _pending) = self
            .pipeline
            .state(ClockTime::from_seconds(STATE_CHANGE_TIMEOUT_S));
        if ret != Ok(StateChangeSuccess::Success) || current != State::Playing {
            return Err(self.fail_start(format!(
                "PLAYING not confirmed within {STATE_CHANGE_TIMEOUT_S}s (ret={ret:?}, current={current:?})"
            )));
        }
        self.metrics.mark_started();
        // PTS 单调性是**运行内**不变量：重启后管线 running-time 归零，
        // 跨周期比较会假报违例——每次 start 重置 PTS 作用域（周期内计数）。
        self.metrics.reset_pts_scope();
        let mut st = self.lock_state()?;
        *st = LifecycleState::Playing;
        Ok(())
    }

    /// 停止到 NULL（幂等：重复调用返回 Ok；错误态亦可停止以释放资源）。
    ///
    /// # Errors
    /// 状态切换失败或超时、状态锁中毒时返回错误。
    pub fn stop(&self) -> Result<(), AdapterError> {
        {
            let st = self.lock_state()?;
            if *st == LifecycleState::Stopped {
                return Ok(());
            }
        }
        self.pipeline
            .set_state(State::Null)
            .map_err(|e| self.record_and_fail(format!("set NULL failed: {e}")))?;
        let (ret, current, _pending) = self
            .pipeline
            .state(ClockTime::from_seconds(STATE_CHANGE_TIMEOUT_S));
        if ret != Ok(StateChangeSuccess::Success) || current != State::Null {
            return Err(self.record_and_fail(format!(
                "NULL not confirmed within {STATE_CHANGE_TIMEOUT_S}s (ret={ret:?}, current={current:?})"
            )));
        }
        let mut st = self.lock_state()?;
        *st = LifecycleState::Stopped;
        Ok(())
    }

    /// 发送 EOS（正常结束；不自动停止——停止由调用方 `stop` 执行）。
    ///
    /// # Errors
    /// 状态锁中毒时返回错误。
    pub fn send_eos(&self) -> Result<(), AdapterError> {
        let _st = self.lock_state()?;
        let sent = self.pipeline.send_event(gstreamer::event::Eos::new());
        if !sent {
            return Err(AdapterError(
                "EOS event not accepted (pipeline not flowing?)".to_string(),
            ));
        }
        Ok(())
    }

    /// 等待 bus EOS 或 ERROR（带超时；超时返回 [`EosOutcome::Timeout`]，不无限等待）。
    ///
    /// # Errors
    /// bus 缺失或状态锁中毒时返回错误。
    pub fn wait_for_eos_or_error(&self, timeout_secs: u64) -> Result<EosOutcome, AdapterError> {
        let bus = self
            .pipeline
            .bus()
            .ok_or_else(|| AdapterError("pipeline has no bus".to_string()))?;
        let msg = bus.timed_pop_filtered(
            ClockTime::from_seconds(timeout_secs),
            &[gstreamer::MessageType::Eos, gstreamer::MessageType::Error],
        );
        match msg {
            None => Ok(EosOutcome::Timeout),
            Some(m) => match m.view() {
                gstreamer::MessageView::Eos(..) => Ok(EosOutcome::Eos),
                gstreamer::MessageView::Error(view) => {
                    let text = format!(
                        "bus error: {} (debug: {})",
                        view.error(),
                        view.debug().unwrap_or_default()
                    );
                    self.metrics.record_error(text.clone());
                    if let Ok(mut st) = self.lock_state() {
                        *st = LifecycleState::Error(text.clone());
                    }
                    Ok(EosOutcome::Error(text))
                }
                _ => Ok(EosOutcome::Timeout),
            },
        }
    }

    /// 指标快照（不可变）。
    #[must_use]
    pub fn snapshot(&self) -> MetricsSnapshot {
        self.metrics.snapshot()
    }

    /// queue 当前积压（缓冲条数, 字节数）——有界性验证用。
    ///
    /// # Errors
    /// 属性读取失败或状态锁中毒时返回错误。
    pub fn queue_levels(&self) -> Result<(u32, u32), AdapterError> {
        let _st = self.lock_state()?;
        // queue 的 current-level-* 属性类型为 guint（u32）。
        let buffers = self.queue.property::<u32>("current-level-buffers");
        let bytes = self.queue.property::<u32>("current-level-bytes");
        Ok((buffers, bytes))
    }

    /// 调整测试专用消费延迟（微秒；0 = 正常路径）。
    pub fn set_consume_delay_us(&self, us: u64) {
        self.config.consume_delay_us.store(us, Ordering::Relaxed);
    }

    fn lock_state(&self) -> Result<std::sync::MutexGuard<'_, LifecycleState>, AdapterError> {
        self.state
            .lock()
            .map_err(|e| AdapterError(format!("lifecycle state lock poisoned: {e}")))
    }

    /// 启动失败统一处理：回退 NULL + 状态置 Error + 返回错误。
    fn fail_start(&self, msg: String) -> AdapterError {
        let _ = self.pipeline.set_state(State::Null);
        if let Ok(mut st) = self.lock_state() {
            *st = LifecycleState::Error(msg.clone());
        }
        self.metrics.record_error(msg.clone());
        AdapterError(msg)
    }

    /// 通用失败处理：状态置 Error + 返回错误。
    fn record_and_fail(&self, msg: String) -> AdapterError {
        if let Ok(mut st) = self.lock_state() {
            *st = LifecycleState::Error(msg.clone());
        }
        self.metrics.record_error(msg.clone());
        AdapterError(msg)
    }
}

/// 元素工厂构建助手（无属性；属性一律构建后 `set_property` 设置，规避 builder 泛型体操）。
fn plain_element(factory: &str) -> Result<Element, AdapterError> {
    gstreamer::ElementFactory::make(factory)
        .build()
        .map_err(|e| AdapterError(format!("element {factory} build failed: {e}")))
}

/// appsink 回调：只做元数据记录（原子量 + ≤1024 字节头部哈希），零重活。
fn on_new_sample(
    sink: &gstreamer_app::AppSink,
    metrics: &PipelineMetrics,
    delay_us: &AtomicU64,
) -> Result<gstreamer::FlowSuccess, gstreamer::FlowError> {
    let sample = match sink.pull_sample() {
        Ok(s) => s,
        Err(_) => return Err(gstreamer::FlowError::Eos),
    };
    let wall_ms = crate::metrics::wall_unix_ms();

    let Some(video_info) = sample
        .caps()
        .and_then(|c| gstreamer_video::VideoInfo::from_caps(c).ok())
    else {
        metrics.record_caps_parse_failure();
        return Ok(gstreamer::FlowSuccess::Ok);
    };
    let Some(buffer) = sample.buffer() else {
        metrics.record_caps_parse_failure();
        return Ok(gstreamer::FlowSuccess::Ok);
    };

    let pts_ns = buffer.pts().map_or(0, ClockTime::nseconds);
    let duration_ns = buffer.duration().map_or(0, ClockTime::nseconds);

    let Ok(mapped) = buffer.map_readable() else {
        metrics.record_map_size_mismatch();
        return Ok(gstreamer::FlowSuccess::Ok);
    };
    let slice = mapped.as_slice();
    if slice.len() != video_info.size() {
        // 映射长度与协商布局不符——按协商布局为准记录失配（不假定紧密排列）。
        metrics.record_map_size_mismatch();
    }

    let head_end = slice.len().min(FRAME_HEAD_HASH_BYTES);
    let head_hash = crate::metrics::fnv1a64(&slice[..head_end]);

    let n_planes = video_info.n_planes() as usize;
    let strides_raw = video_info.stride();
    let offsets_raw = video_info.offset();
    let mut strides = [0u64; 4];
    let mut plane_offsets = [0u64; 4];
    for (i, slot) in strides.iter_mut().enumerate().take(n_planes.min(4)) {
        *slot = strides_raw
            .get(i)
            .copied()
            .map_or(0, |v| u64::try_from(v).unwrap_or(0));
    }
    for (i, slot) in plane_offsets.iter_mut().enumerate().take(n_planes.min(4)) {
        *slot = offsets_raw
            .get(i)
            .copied()
            .map_or(0, |v| u64::try_from(v).unwrap_or(0));
    }

    let frame = FrameInfo {
        seq: 0, // record_frame 内分配
        pts_ns,
        duration_ns,
        format: video_info.format().to_str(),
        width: video_info.width(),
        height: video_info.height(),
        n_planes: video_info.n_planes(),
        strides,
        plane_offsets,
        map_size: slice.len() as u64,
        recv_wall_unix_ms: wall_ms,
        recv_monotonic_us: 0, // record_frame 内补记
        head_hash_fnv1a: head_hash,
    };
    metrics.record_frame(frame);

    // 测试专用背压注入（生产恒 0；上限 1 s，杜绝无限阻塞）。
    let d = delay_us.load(Ordering::Relaxed);
    if d > 0 {
        std::thread::sleep(Duration::from_micros(d.min(MAX_CONSUME_DELAY_US)));
    }
    Ok(gstreamer::FlowSuccess::Ok)
}

#[cfg(test)]
mod tests {
    use super::{LifecycleState, TestVideoPipeline};
    use crate::config::PipelineConfig;

    /// 构建即 CREATED；未启动无帧。
    #[test]
    fn build_enters_created_state() {
        let p = TestVideoPipeline::build(PipelineConfig::default_720p30())
            .unwrap_or_else(|e| panic!("build failed: {e}"));
        assert_eq!(
            p.state().unwrap_or_else(|e| panic!("state: {e}")),
            LifecycleState::Created
        );
        assert_eq!(p.snapshot().frames_received, 0);
        assert_eq!(p.policy().width, 1280);
        assert_eq!(p.policy().height, 720);
        assert_eq!(p.queue_max_buffers(), 4);
        assert_eq!(p.appsink_max_buffers(), 2);
    }

    /// 重复 stop 幂等（未启动过也安全）。
    #[test]
    fn stop_is_idempotent_before_start() {
        let p = TestVideoPipeline::build(PipelineConfig::default_720p30())
            .unwrap_or_else(|e| panic!("build failed: {e}"));
        p.stop().unwrap_or_else(|e| panic!("stop1: {e}"));
        p.stop().unwrap_or_else(|e| panic!("stop2: {e}"));
        assert_eq!(
            p.state().unwrap_or_else(|e| panic!("state: {e}")),
            LifecycleState::Stopped
        );
    }
}
