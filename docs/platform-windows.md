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

## 6. 音频栈与 SysVAD 书面评估（M0-T5，2026-11-11）

> 依据 run `G0-20261111-212232-D720`（纯书面研究，不构建不安装驱动）；完整结论与官方引用见该 run `decision.md`（全部来源访问日期 2026-11-11）。

### 6.1 Windows 音频栈（官方 [S17]）

应用 API（推荐 **WASAPI**；枚举推荐 Windows.Devices.Enumeration）→ 音频服务（audiosrv.dll 流设置/策略；AudioEndpointBuilder 发现设备、创建软件端点）→ Audio Engine（audiodg.exe 加载 audioeng.dll 混音/处理/APO）→ 驱动 **port/miniport（WaveRT）**。Track A（第三方虚拟音频设备）与 Track B（SysVAD）工作在同一端点/驱动层——Track A 不改变「非自研驱动」边界。

### 6.2 SysVAD 构建条件（官方 README）与本机缺口

- 要求：VS + SDK + **WDK**；**v142（VS 2019）工具集**（含 ATL/Spectre 组件）；WIL git submodule；部署/测试需**第二台目标机**；组件化 INF（Win10 1809+）；原样 sample 有 **HLK 预期失败项**（Hardware Offload of Audio Processing Test）⇒ 原样不可过 HLK 提交。
- 本机：仅 v143（VS 2022 BuildTools 17.14）/ v145（VS 2026 18.9）⇒ **无 v142，构建条件有缺口**（书面确认；实际构建待实测确认且须先 AUTH_REQUIRED）。
- WDK 配套（官方 download-the-wdk，ms.date 2026-07-22）：VS 2026 → WDK 28000.2526；VS 2022 → WDK 26100.6584（SDK/WDK build 号须一致，QFE 可不同）；**EWDK** = 自包含命令行构建环境（独立许可条款）——隔离测试机免装 VS 的候选路径。

### 6.3 驱动签名四路径（官方 [S5]，ms.date 2026-03-23）

| 路径 | 一句话定位 | 对本项目 |
|---|---|---|
| HLK tested + dashboard signed | 零售推荐；Server 2016+ 只加载 HLK 驱动 | Beta 零售长期路径（非 G0–G2） |
| Attestation signing | 免 HLK、仅测试分发；需 **EV 证书 + Partner Center**；不可上 WU 零售 | Track B 测试机候选；金额待询价 |
| Preproduction signing | **Secure Boot 开启**下测试（设备 provisioning 信任） | 测试机策略候选（减少安全降级窗口） |
| Test signing | `bcdedit /set TESTSIGNING ON` + 测试证书；官方警告可能需挂起 BitLocker/Secure Boot | **仅限专用隔离测试机；本机永久排除** |

### 6.4 硬边界（逐字执行基线 §2.2/ADR-004）

1. 不得在主力办公机/企业受控机上为驱动试验禁用安全功能；不建议绕过 Secure Boot/BitLocker。
2. 测试签名仅限专用、可恢复的隔离测试机，按官方流程操作后恢复并留证；**本机（家庭版主力机）不承担驱动测试签名**，`bcdedit /set TESTSIGNING ON` 在本机为禁止操作（AUTH_REQUIRED，不降级为 SYS_CHANGE）。
3. 企业管控受阻记录并上报，不得规避（R-11）。

### 6.5 测试机与 Track A/B 策略

- 第二台隔离测试机：**unconfirmed 保持** ⇒ Track B 全程 HOLD；M2-T6 未授权/未触发记 NOT_RUN（ADR-011 D1，复评触发：测试机确认 + 授权 GRANTED + 构建路径选定）。
- Track A 候选（许可/分发调研完成，**不默认随产品捆绑**；兼容性只写实测，M2-T3）：
  | 候选 | 许可 | 捆绑路径 |
  |---|---|---|
  | VB-CABLE（VB-Audio） | Donationware | webshop **Volume Licensing**（官方原文）；分发条款细节页不可达，捆绑前须 vendor 书面条款 |
  | VoiceMeeter（VB-Audio） | Donationware（30 天后邀购） | 同 VB-Audio 条款体系 |
  | Virtual Audio Cable（Muzychenko） | 商业 $30 Home / $50 Business，永久授权 | **官方 custom（proprietary）version**：改名 + 仅与客户应用使用 |
- 口径红线：Track A 成功不得表述为「自研虚拟麦克风驱动完成」；G2 汇总须给 Track B 显式结论。
