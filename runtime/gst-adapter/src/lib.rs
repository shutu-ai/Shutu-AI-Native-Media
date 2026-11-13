//! gst-adapter — GStreamer 封装空壳（M1-T1 骨架）。
//!
//! 边界（ADR-002 / R-10）：本 crate 只做初始化与环境验证；
//! 不实现任何媒体管线（testsrc→appsink 属 M1-T2）、不做共享内存/管道承载。
//!
//! 所有 GStreamer 具体类型禁止泄漏到 media-core；反向依赖（media-core → 本 crate）禁止。

/// 初始化 GStreamer 运行时（真实 DLL 加载验证入口）。
///
/// # Errors
/// GStreamer 运行时初始化失败（DLL 缺失 / 版本不符 / 注册表扫描失败）时返回错误字符串。
pub fn init() -> Result<GstRuntimeInfo, String> {
    gstreamer::init().map_err(|e| format!("gst init failed: {e}"))?;
    let version = gstreamer::version();
    Ok(GstRuntimeInfo {
        major: version.0,
        minor: version.1,
        micro: version.2,
    })
}

/// GStreamer 运行时标识（构建 + 链接验证证据用）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GstRuntimeInfo {
    pub major: u32,
    pub minor: u32,
    pub micro: u32,
}

impl GstRuntimeInfo {
    /// 版本至少达到指定最低要求（骨架期固定 1.24 下限；本机实测 1.28.3）。
    #[must_use]
    pub fn is_at_least(&self, major: u32, minor: u32) -> bool {
        (self.major, self.minor) >= (major, minor)
    }
}

/// 查询指定名称的 element factory 是否存在（真实插件注册表扫描）。
#[must_use]
pub fn has_element_factory(name: &str) -> bool {
    gstreamer::ElementFactory::find(name).is_some()
}

#[cfg(test)]
mod tests {
    use super::{has_element_factory, init, GstRuntimeInfo};

    /// 真实 MSVC 链接验证：加载 GStreamer MSVC DLL、初始化运行时、版本断言。
    /// 前置：PATH 含 C:\gstreamer\1.0\msvc_x86_64\bin（见 scripts/ci.ps1 / .cargo/config.toml 注释）。
    #[test]
    fn gstreamer_runtime_links_and_initializes() {
        let info = init().unwrap_or_else(|e| panic!("gst runtime init failed: {e}"));
        assert!(
            info.is_at_least(1, 24),
            "gstreamer runtime too old: {info:?} (require >= 1.24)"
        );
    }

    /// 插件注册表真实扫描：核心测试源 element 必须可枚举（M1-T2 管线的前置依赖）。
    #[test]
    fn videotestsrc_factory_present() {
        init().unwrap_or_else(|e| panic!("gst runtime init failed: {e}"));
        assert!(
            has_element_factory("videotestsrc"),
            "videotestsrc factory missing"
        );
    }

    /// 版本比较边界（纯逻辑，无运行时依赖）。
    #[test]
    fn version_comparison_boundaries() {
        let v = GstRuntimeInfo {
            major: 1,
            minor: 28,
            micro: 3,
        };
        assert!(v.is_at_least(1, 24));
        assert!(v.is_at_least(1, 28));
        assert!(!v.is_at_least(1, 29));
        assert!(!v.is_at_least(2, 0));
    }
}
