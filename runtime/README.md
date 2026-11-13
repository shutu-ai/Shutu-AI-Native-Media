# runtime/ — Rust 用户态运行时（ADR-001）

三 crate 边界（基线 §3 三平面解耦；依赖方向 **gst-adapter / device-bridge → media-core**，反向禁止）：

| crate | 职责 | 依赖 |
|---|---|---|
| `media-core` | 平台无关类型：session / graph / policy / metrics | **零外部依赖**（设计约束） |
| `gst-adapter` | GStreamer (gstreamer-rs) 封装；所有 GStreamer 类型封闭在此 | media-core, gstreamer |
| `device-bridge` | 虚拟设备桥接；传输策略类型守卫（ADR-012：不得硬编码共享内存唯一方式） | media-core |

## 本机构建（M1-T1）

- 工具链：MSVC（rustc host `x86_64-pc-windows-msvc`），版本组合记录在 G1 run `environment.json`。
- GStreamer MSVC SDK：`C:\gstreamer\1.0\msvc_x86_64`（1.28.3，M0-T1 盘点）；`.cargo/config.toml`
  提供 `PKG_CONFIG`/`PKG_CONFIG_PATH`（SDK 自带 pkg-config.exe，无需安装）。
- **运行期 DLL**：`cargo test` 前须把 SDK `bin` 前置 PATH——直接运行 `pwsh -NoProfile -File scripts/ci.ps1`。
- 命令：`cargo fmt --all -- --check` ｜ `cargo clippy --workspace --all-targets -- -D warnings` ｜ `cargo test --workspace`

## 不做的事（R-10 / ADR-012 边界，M1-T1 时点）

- 不实现媒体管线（testsrc→appsink 属 M1-T2）
- 不实现 Ring Buffer / IPC 协议 / Frame Server 客户端（M1-T3~T5）
- 不硬编码传输选型；`TransportPolicy::ADR012_DEFAULT` = 管道首选 + 无实测背书（P1-3 OPEN）
