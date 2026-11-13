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

## 7. Frame Server 实测事实与跨会话 IPC（M0-T4，2026-11-12~13）

> 依据 run `G0-20261112-191515-3C36`（真机；判定 M0-T4=PASS、R-02=GO）。完整事实表、PoC 矩阵与决策全文见 run `decision.md`；PoC 源码 `tests/contract/ipc-identity-poc/`。

### 7.1 Frame Server 服务事实（F1-F11 浓缩）

| 事实 | 实测内容 |
|---|---|
| 服务标识 | FrameServer / Windows Camera Frame Server；TYPE=20 WIN32_SHARE_PROCESS；DEMAND_START；BINARY_PATH=`System32\svchost.exe -k Camera`；依赖 rpcss（sc qc + CIM 双源） |
| 运行身份 | **NT AUTHORITY\LocalService（S-1-5-19）**；运行中进程 **SessionId=0**（Session 0 隔离实测确认）；17 线程、句柄 440~462 |
| 生命周期 | **完全按需启停**：仅活跃拉流期间 RUNNING，流结束数秒~数十秒自动停，多轮 PID 各异；**SCM 7036 事件不记录其启停**（wevtutil 实测）——生命周期监控只能 `sc query` 轮询 |
| DLL 加载 | 提权模块表确认 **VirtualCameraMediaSource.dll 已加载进 svchost**（连同 frameserver.dll/FrameServerCore.dll/MF* 全栈）；非提权 `tasklist /m` 返回「暂缺」为权限事实，非未加载 |
| 服务 SDDL | 交互用户(IU)仅查询类权限（CC/LC/SW/LO/CR/RC），无 RP/WP/DC——用户会话代码只能观测服务状态，启停由摄像头激活栈驱动 |
| 恢复与其他 | sc stop 后再拉流 0s 按需恢复；DLL 被服务缓存锁定（注销后需 `sc stop frameserver` 释放，M0-T2 发现6）；`ServiceProfiles\LocalService` 非管理员不可读 |

### 7.2 跨会话 IPC 身份事实（PoC 双侧实测）

| 主体 | SID | Session | SeCreateGlobalPrivilege |
|---|---|---|---|
| 交互用户（Runtime 侧原型） | S-1-5-21-…-1001 | 1 | **无** |
| Frame Server（LocalService） | S-1-5-19 | 0 | **有** |

实测结论（测试 ID 对应 run test-results.json）：

1. **LS(Session 0) 可 OS 级连接用户进程创建、DACL 显式授 S-1-5-19 的命名管道**（T5 连通成功）；不授即拒 gle=5（T6 反证）——**管道 DACL 即 OS 级身份边界**。
2. **跨会话服务端 OpenProcess+TokenUser 校验不可行**（T5：用户侧服务进程无法打开 LS 客户端进程查 token，fail-closed 拒绝生效）；同会话下该校验链路工作正常（T1/T3），保留为纵深。
3. **非提权用户进程不能创建 Global\ 命名对象**（T7b gle=5，SeCreateGlobalPrivilege 不在交互令牌）——基线 §5.1「用户进程创建共享内存 Ring 供服务读」的用户侧前提被实测否定；LS 写→用户读方向可行（T7a）但与视频生产方向相反。
4. Local\ 命名空间按会话隔离，跨会话不可见（T9）。

### 7.3 R-02 决策摘要（G0 GO 前置条件；全文见 run decision.md §4）

- **控制通道**：Named Pipe——用户会话 Runtime 为服务端 + DACL 显式授 S-1-5-19（D:P 保护，禁 Everyone/NULL SID），Frame Server 内 MediaSource 为客户端。**首选**。
- **帧数据承载**：首选=**管道复用**（帧随控制通道；720p30/1080p30 吞吐与时延留 M1-T4/T5 实测，≈221 Mb/s 量级）；备选=LS 特权 broker 服务创建 Global\ section（仅当实测吞吐不足，引入需新 ADR）。
- **安全边界五条**（写入 M1-T4/T5 实现约束）：①管道 DACL=唯一 OS 身份边界；②放弃跨会话进程级 token 校验、保留同会话纵深 + 应用层握手（版本+challenge）附加层；③帧通道只读消费 + 消息长度/超时上限（禁 Frame Server COM 回调阻塞）；④fail-closed（任何校验失败即断连）；⑤日志只记 PID/结果/错误码，不含帧内容。
- 正式化路径：M0-T6 建立 `docs/adr/` 后以首个 ADR 正式化（涉及基线 §5.1 视频通道策略调整，基线 §14.3）。

### 7.4 探测方法学沉淀（M1 复用）

- 服务按需启停使「定时快照」必然脱靶：用**事件驱动轮询**（0.4s `sc query`，RUNNING 即捕获）+ **拉流循环扩窗**（官方 streamtest 连发）；脚本留档 run `staging/probe-watch*.ps1`。
- LS 侧探针走计划任务（`/ru NT AUTHORITY\LOCALSERVICE`）：**触发点必须未过期**（用 `/sc daily /st 23:59` + 手动 `/run`；`/sc once /st 00:00` 会在 /run 前被调度器清除）；session 0 内 `timeout` 不可靠，用 `ping -n` 延时；wrapper 输出在 `start /b` 后可能截断（关键输出须在截断点前落盘或写独立文件）。

### 7.5 服务 SID 类型补充（M0-T6 只读查询，2026-11-13）

`sc qsidtype frameserver` = **UNRESTRICTED**；per-service SID = **NT SERVICE\FrameServer**
（`S-1-5-80-3915894004-2104103821-3047269622-1811662266-774708259`，`sc showsid`）。
Frame Server 进程令牌携带该组 SID——以它作为管道 DACL 主体（替代共享的 S-1-5-19）可收紧最小授权；
该主体是否实际放行 Frame Server 客户端为推断，属 ADR-012 U1，M1-T4 强制实测（技术债 P1-1 验收内容）。

## 8. 双应用消费与生命周期实测（M0-T3，2026-11-13）

> 依据 run `G0-20261113-201619-b4dc`（真机；判定 M0-T3=PASS，含 App A 已知缺陷）。完整判定矩阵、偏差清单 D1-D13 与 SYS_CHANGE 台账见 run `decision.md`；测试页与控制服务 `tests/fixtures/getusermedia/`。

### 8.1 消费矩阵事实（Windows 11 26200.9974 真机）

| 应用（版本） | 枚举 vcam | 渲染合成帧 | 并发消费 | Stop/Start 恢复 |
|---|---|---|---|---|
| Chrome 157.0.8103.51（getUserMedia） | ✅ 系统级 2 机可见 | ✅ 640x480@30，图案=M0-T2 基线 | ✅ 与相机应用（物理头）经同一 Frame Server 并发无干扰 | ✅ track 重建 225ms；重注册后**不自动恢复，手动重开正常** |
| Windows 相机 2026.2608.1.0（UWP） | ✅ 选择器可见 | ⚠️ **首次渲染成功**（多次截图证实），但**摄像头间切换间歇抛 `0xA00F4241<CameraSwitchFailed>(0x80004003)`，错误态持久不可自愈**（重启应用/SE 检查/Frame Server 重启均无效） | ❌ 阻断（不稳定消费者） | ❌ 卡死错误态无法恢复 |
| 设备移除时另一应用行为 | Chrome：track **ENDED**（未观察到 mute 前置），0 错误；相机应用（物理头位）：零影响持续渲染 | | | |

### 8.2 生命周期时序事实（Frame Server）

| 事件 | 实测时延 | 备注 |
|---|---|---|
| track 全停 → FS STOPPED | ~36 s | 需求服务拆除延迟 |
| 空闲 → getUserMedia 请求 → FS RUNNING | ~0.6 s | 按需冷启动 |
| 使用中移除 → FS 状态 | **不中断**（其他摄像头会话维持 RUNNING） | 多会话并存实证 |
| 全部消费者退出 → FS 降停（终态观察） | 分钟级（本 run 8.5 min 后 STOPPED） | 与 36s 路径不同的空闲超时路径，M1 复测 |
| 注销后 DLL 目录删除被拒 | svchost 缓存锁定 | `sc stop frameserver` 释放后删除成功；FS 随后按需自恢复 |
| 重复注册/注销 | A1→B2→A3→B4 四轮循环实例 ID 确定性一致，无残留 | 终态：设备 ABSENT、COM=no、目录已删 |

### 8.3 测试方法学沉淀（M1 复用）

- **窗口遮挡会停摆 rVFC 计帧**（presentedFrames 不增长）但 track 事件与网络通道正常——帧数证据须前台化窗口采集；后台期以事件流为准。
- **PrintWindow 对 UWP/Chrome 可能取到陈旧合成帧**（遮挡/过渡态）——截图判定须配合 report JSONL 交叉验证；本次两处证据命名失准（D13）均由收口逐图审计发现，**截图判读必须人工逐图复核，不得只看文件名**。
- 提权 .cmd 必须 CRLF+ASCII（LF/UTF-8 中文 REM 在 GBK cmd.exe 静默失败）；提权上下文 `copy` 有静默失败先例，用 `PowerShell Copy-Item -ErrorAction Stop` + MARK 步骤标记。
- /cmd 文件通道按值去重（`cmd !== cmdFileSeen`），重发同值命令无效。
- Chrome 多实例防污染：`taskkill //F` 后重启须加 `--no-session-crash-bubble --hide-crash-restore-bubble`。
- **run 的 staging 脚本必须自带停止条件**（M0-T7 审计教训 L-1）：M0-T3 遗留 `frameserver-poll.sh` 无限轮询依赖人工 kill 且会话收口遗漏，导致 run 无法收口固化（Git Bash 下 `taskkill /F` 会因 MSYS 路径转义失败，须写 `//F`）——收口检查单须含「无遗留进程」项。
