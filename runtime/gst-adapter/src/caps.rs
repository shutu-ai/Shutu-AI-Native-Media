//! caps 映射：media-core 帧策略 → GStreamer 固定 caps。
//!
//! 格式选型（M1-T2 实测决策，全文记录于 contracts/media/README.md §caps 决策）：
//! **NV12 1280x720@30** —— 对齐 `FramePolicy::TARGET_720P30`；YUV 通路为虚拟摄像头
//! 转发的规划格式；多平面 stride/offset 处理是本任务的显式验证点。
//! 禁止动态重协商（R-04 / M1-T2 字段 9）：caps 固定，且 capsfilter 与 appsink 双侧钉死。

use media_core::policy::{FramePolicy, PixelFormat};

use crate::AdapterError;

/// 像素格式 → caps `format` 字段字符串。
#[must_use]
pub fn format_str(format: PixelFormat) -> &'static str {
    match format {
        PixelFormat::Nv12 => "NV12",
        PixelFormat::Rgb => "RGB",
    }
}

/// 帧策略 → 固定 caps（整数帧率以 n/1 分数表达）。
///
/// # Errors
/// 宽/高超出 i32 表达范围时返回错误（`FramePolicy::is_valid` 上限 7680x4320 内不可达）。
pub fn caps_for(policy: &FramePolicy) -> Result<gstreamer::Caps, AdapterError> {
    let width = i32::try_from(policy.width)
        .map_err(|_| AdapterError(format!("width {} exceeds i32", policy.width)))?;
    let height = i32::try_from(policy.height)
        .map_err(|_| AdapterError(format!("height {} exceeds i32", policy.height)))?;
    let fps = i32::try_from(policy.fps)
        .map_err(|_| AdapterError(format!("fps {} exceeds i32", policy.fps)))?;
    Ok(gstreamer::Caps::builder("video/x-raw")
        .field("format", format_str(policy.format))
        .field("width", width)
        .field("height", height)
        .field("framerate", gstreamer::Fraction::new(fps, 1))
        .build())
}

#[cfg(test)]
mod tests {
    use media_core::policy::{FramePolicy, PixelFormat};

    use super::{caps_for, format_str};

    #[test]
    fn caps_for_target_720p30_is_fixed_nv12() {
        crate::init().unwrap_or_else(|e| panic!("gst init failed: {e}"));
        let caps = caps_for(&FramePolicy::TARGET_720P30)
            .unwrap_or_else(|e| panic!("caps build failed: {e}"));
        let s = caps.to_string();
        assert!(s.contains("video/x-raw"), "caps media type wrong: {s}");
        assert!(s.contains("format=(string)NV12"), "format wrong: {s}");
        assert!(s.contains("width=(int)1280"), "width wrong: {s}");
        assert!(s.contains("height=(int)720"), "height wrong: {s}");
        assert!(s.contains("framerate=(fraction)30/1"), "fps wrong: {s}");
    }

    #[test]
    fn format_str_mapping_is_stable() {
        assert_eq!(format_str(PixelFormat::Nv12), "NV12");
        assert_eq!(format_str(PixelFormat::Rgb), "RGB");
    }
}
