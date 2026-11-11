# Windows 平台环境快照（M0-T1）

> **正式来源**：`artifacts/G0/G0-20261111-174257-8532/environment.json`（schema v1.0，真机生成，SHA-256 见该 run 的 `SHA256SUMS.txt`）。本文件为其人类可读快照，冲突时以 environment.json 为准。
> **生成**：2026-11-11 ｜ 探测工具：`tools/verify-environment/verify-environment.ps1`（只读） ｜ pwsh 7.6.9

## 1. 操作系统与硬件

| 项 | 值 | 满足的门槛 |
|---|---|---|
| OS | Windows 11 家庭中文版 25H2，**Build 26200.9974**（EditionID=CoreCountrySpecific） | ≥22000（MFCreateVirtualCamera 门槛）✅；**家庭版 ⇒ 不承担驱动测试签名**（基线 §2.2/ADR-004） |
| CPU | Intel Core Ultra 7 255H（16C/16T） | CPU 基线 ✅（GPU 加速留 M3 实测） |
| GPU | Intel Arc 140T 16GB（驱动 32.0.101.8801，2026-05-12）；另有第三方「GameViewer Virtual Display Adapter」虚拟显示驱动（见 §4 风险线索） | — |
| RAM / 磁盘 | 31.4GB；C: 920.3GB（可用约 296GB） | ✅ |

## 2. 开发工具链（全部实测执行版本命令）

| 工具 | 版本 | 备注 |
|---|---|---|
| git | 2.56.0.windows.4 | ✅ |
| VS 2022 BuildTools | 17.14.37301.10（MSVC v143 14.44.35207，cl 19.44.35227） | cl.exe 真实执行 ✅ |
| VS 2026 BuildTools | 18.9.12105.275（v145 14.51.36231，cl 19.51.36256） | 两套并存，默认用 2022 工具集 |
| Windows SDK | 10.0.26100.0 / 10.0.22621.0 | VirtualCamera Sample ✅ |
| Rust | rustc/cargo 1.95.0，`stable-x86_64-pc-windows-msvc` | msvc 目标链接验证 ✅ |
| CMake | 4.2.3（Kitware 独立安装） | ✅ |
| GStreamer | **1.28.3 MSVC x86_64**（`C:\gstreamer\1.0\msvc_x86_64`，gst-launch 实测） | MSVC 变体 ✅（MinGW 版未检出） |
| Python | 3.14.7 | 可选 Worker 预留 ✅ |
| WDK | **未安装（决策：不安装）** | 家庭版不承担驱动测试签名；Track B 仅限授权隔离测试机 |

## 3. 设备、隐私与安全软件线索

- 物理摄像头 3 台：HP IR Camera / Poly Camera Pro / HP 5MP Camera（PnP Status=OK）；音频设备含 Intel 智音技术（数字麦克风）等 4 项 Status=OK。
- 隐私主开关（ConsentStore）：microphone=Allow；camera 主键尚未生成（首次访问时创建，M0-T3 实测）。
- 安全软件：Windows Defender；防火墙三 profile 全启用；活动网卡：I219-LM 以太网 + BE201 Wi-Fi。**企业管控/VPN 客户端需人工复核（R-11）**。
- 本地账户含多个启用账户（多用户场景可行性线索，§5.1-7 现场验证留 M0-T3/M1-T6）。

## 4. 风险线索与教训（供后续任务对照）

1. **探测误报教训 ×2**：① vswhere 无效参数误报「无 VS」；② 长会话进程未继承安装器新写的机器环境变量误报「无 GStreamer」。verify-environment 已内置对策（合法参数 + 进程→注册表→默认路径解析链）。**任何「未检测到」结论都必须用第二方法复核后才可写入文档。**
2. GameViewer 虚拟显示适配器为第三方设备类驱动：M0-T2 注册虚拟摄像头时观察共存行为（R-07/R-11）。
3. `unconfirmed`：第二台隔离驱动测试机有无（需用户答复；影响 M0-T5 策略与 M2-T6 授权前置）。

## 5. 索引

- run-id：`G0-20261111-174257-8532`（审计链见 `docs/evidence/EVIDENCE_INDEX.md` 与 run 内 `manifest.json`）。
- 复测方法：`pwsh -NoProfile -File tools/verify-environment/verify-environment.ps1 -OutFile <run 目录>\environment.json -RunId <run-id>`；校验：`python tools/verify-environment/validate_environment.py --file <path>`。
