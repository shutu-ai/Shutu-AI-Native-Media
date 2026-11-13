//! 帧策略类型骨架：分辨率 / 帧率 / 像素格式。
//!
//! 数值骨架取 M0-T2/M0-T3 实测基线（640x480@30 合成帧）；正式 NFR 指标在 M1-T4/T5 测量。

/// 像素格式（骨架枚举：仅列出已实测或已规划的值）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    /// M0-T2/M0-T3 实测合成帧格式（TestVCam 基线）。
    Rgb,
    /// 规划中的 YUV 通路（M1-T2 起）。
    Nv12,
}

/// 帧策略（恒定；变更走显式重建，不原地修改——不可变设计约定）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FramePolicy {
    pub width: u32,
    pub height: u32,
    /// 帧率，单位：帧/秒（整数；骨架期不支持分数帧率）。
    pub fps: u32,
    pub format: PixelFormat,
}

impl FramePolicy {
    /// M0 实测基线：640x480@30 RGB。
    pub const DEFAULT: Self = Self {
        width: 640,
        height: 480,
        fps: 30,
        format: PixelFormat::Rgb,
    };

    /// 基线目标：720p30（基线 §9 M1 验收口径）。
    pub const TARGET_720P30: Self = Self {
        width: 1280,
        height: 720,
        fps: 30,
        format: PixelFormat::Nv12,
    };

    /// 骨架期合法性检查（负值不可表达；此处只查非零与上限）。
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.width > 0
            && self.height > 0
            && self.fps > 0
            && self.width <= 7680
            && self.height <= 4320
            && self.fps <= 240
    }
}

#[cfg(test)]
mod tests {
    use super::{FramePolicy, PixelFormat};

    #[test]
    fn default_policy_is_valid() {
        assert!(FramePolicy::DEFAULT.is_valid());
        assert_eq!(FramePolicy::DEFAULT.width, 640);
        assert_eq!(FramePolicy::DEFAULT.fps, 30);
    }

    #[test]
    fn target_720p30_is_valid_and_distinct() {
        assert!(FramePolicy::TARGET_720P30.is_valid());
        assert_eq!(FramePolicy::TARGET_720P30.format, PixelFormat::Nv12);
        assert_ne!(FramePolicy::DEFAULT, FramePolicy::TARGET_720P30);
    }

    #[test]
    fn zero_dimensions_rejected() {
        let bad = FramePolicy {
            width: 0,
            ..FramePolicy::DEFAULT
        };
        assert!(!bad.is_valid());
    }
}
