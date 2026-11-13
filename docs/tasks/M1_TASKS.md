# M1 任务明细 — Rust/GStreamer 媒体基础（Gate G1）

> **来源**：`docs/ARCHITECTURE_BASELINE.md` §9 M1（架构基线 v1.0）。
> **门禁前置**：G0 = GO（否则本里程碑全部任务 BLOCKED）。
> **G1 验收原文**（§9）：Windows Camera 与第二款应用持续使用 30 分钟；AI 为 0 依赖；Runtime 杀进程后设备不崩溃并进入安全备用画面；队列有界。
> **基线原文任务单**：① testsrc→processing→appsink 基础视频图；② 共享内存视频 Ring + Named Pipe 控制（ACL、序号、时钟、故障状态）；③ MediaSource 帧源改为 Runtime 已准备帧；④ safe fallback、重连、720p 30fps、格式协商、状态指标；⑤ 物理摄像头输入（需权限、防争抢）。

## 任务总览

| ID | 目标摘要 | 前置依赖 | AUTH | 状态 |
|---|---|---|---|---|
| M1-T1 | Rust workspace 三 crate 骨架 + CI + threat-model 初版 + SBOM 机制 | G0=GO | — | 未开始 |
| M1-T2 | `testsrc → processing → appsink` 基础视频图 | M1-T1 | — | 未开始 |
| M1-T3 | 共享内存视频 Ring Buffer（有界、代际序号、契约测试） | M1-T1 | — | 未开始 |
| M1-T4 | Named Pipe 控制通道（版本化、ACL、身份校验） | M1-T1 | — | 未开始 |
| M1-T5 | VirtualCamera MediaSource 改为 IPC ring 读帧 | M0-T4, M1-T3, M1-T4 | — | 未开始 |
| M1-T6 | 端到端验证 + 多消费者/用户切换/重复安装卸载 | M1-T2, M1-T3, M1-T4, M1-T5 | — | 未开始 |
| M1-T7 | 物理摄像头输入接入（权限、防争抢） | M1-T6 | — | 未开始 |
| M1-T8 | **G1 Gate 汇总**（30min、kill 备用画面、NFR-01/03/07 初步；AGG） | M1-T6, M1-T7 终态（AGG） | — | 未开始 |

---

## M1-T1 · Rust workspace 骨架 + 威胁模型初版 + SBOM 机制启动

**1. Task ID 与目标**
创建 Rust workspace：`runtime/media-core`（session/graph/policy/metrics 骨架）、`runtime/gst-adapter`（GStreamer 封装空壳）、`runtime/device-bridge`（IPC/共享内存空壳）；打通 `cargo build/test` 与 CI 脚本；建立 `docs/threat-model.md` 初版；启动「引入依赖即记 SBOM」机制。

**2. 对应需求 FR / 架构 ADR**
- ADR-001（Rust 用户态运行时）、ADR-002（GStreamer 底座）、ADR-006（进程隔离）；基线 §3 目标架构、§8 依赖规则、§9 M1、§13 目录结构。

**3. 前置依赖**
G0 = GO。

**4. 允许修改的模块与文件范围**
- 新建：`runtime/` 三 crate、workspace `Cargo.toml`、CI 配置、`docs/threat-model.md`、`docs/licensing-sbom` 索引
- 不允许：任何 OS 设备侧改动（虚拟摄像头 C++ 项目不动）。

**5. 实施步骤**
1. 建 workspace 与三 crate 骨架；每个 crate 空实现 + 最小单元测试；确定 crate 间依赖方向（gst-adapter、device-bridge 依赖 media-core 的类型，反向禁止）。
2. 接入 gstreamer-rs（记录版本与许可证进 SBOM；MSVC 工具链验证 `cargo build` 真实通过）。
3. CI 脚本（本地可运行）：`cargo fmt --check`、`cargo clippy`、`cargo test`。
4. `docs/threat-model.md` 初版：按 §11 列出信任边界（Frame Server / media-service / Worker / 控制面）、待完善项。
5. SBOM 机制：约定 `licensing-sbom.json` 追加规则（版本、commit、来源 URL、许可证、分发条款、链接方式，§8）。

**6. 自动化测试要求**
- 三 crate 各有可运行的单元测试（哪怕是骨架断言）；CI 全绿。

**7. 真实环境验收要求**
- `cargo build/test` 在本机 MSVC 工具链真实通过并留存 `build.log`；gstreamer-rs 能链接真实 GStreamer DLL（MSVC 版）。

**8. 完成标准（DoD）**
- workspace 构建测试全绿；threat-model 初版与 SBOM 机制入库；crate 边界与 §13 目录一致。

**9. 风险及失败处理**
- gstreamer-rs 与本机 GStreamer 版本不匹配 → 固定版本组合记录进 environment.json；不静默升级。
- 过度设计（R-10）→ 骨架只含空实现与边界，不提前写协议与调度器。

**10. 应保留的测试证据**
`artifacts/G1/<run-id>/build.log`、`test-results.json`、`licensing-sbom.json`（gstreamer-rs 等）。

**11. 状态**：未开始

---

## M1-T2 · 基础视频管线：testsrc → processing → appsink

**1. Task ID 与目标**
在 `gst-adapter` 中构建首条真实管线：`videotestsrc → videoconvert → appsink`，Rust 侧收到带 PTS 的帧；配置固定 caps（720p BGRA 或 NV12，由实测选定并记录）、有界队列；提供可编程的启停与状态查询。

**2. 对应需求 FR / 架构 ADR**
- FR-004（原始媒体旁路模式的最小形态）、FR-006（格式协商、时间戳、观测）；ADR-002；基线 §9 M1-1、§4.4（时间模型）；参考 [S6][S7][S8]。

**3. 前置依赖**
M1-T1。

**4. 允许修改的模块与文件范围**
- 修改：`runtime/gst-adapter/`（管线封装）、`runtime/media-core/`（最小状态/指标类型）
- 新建：`tests/integration/`（gst-adapter 集成测试）

**5. 实施步骤**
1. 封装 Pipeline 生命周期：CREATED→PLAYING→STOPPED 的安全转换与错误回调。
2. `videotestsrc`（固定 pattern）→ capsfilter（720p，格式由本任务实测确定并写入 contracts 占位文档）→ appsink（回调取帧）。
3. 帧到达时记录：PTS、duration、序号；暴露帧计数/丢帧计数指标。
4. 队列上限设置（`queue max-size-*`），验证超限行为（丢帧而非无限增长）。
5. 优雅停止：EOS 处理、无泄漏退出（测试断言）。

**6. 自动化测试要求**
- 集成测试：启动后 N 秒内收到 ≥N 帧；PTS 单调递增；stop 后线程与资源无泄漏（可用 cargo test + 真实 GStreamer）。

**7. 真实环境验收要求**
- 在本机真实 GStreamer 环境运行测试并留存输出；**进程内管线测试通过仍只是 CI 级证据，不得表述为虚拟设备验证**。

**8. 完成标准（DoD）**
- 管线可编程启停；帧流指标可得；测试全绿；caps 决策记录在案。

**9. 风险及失败处理**
- GStreamer 动态图/协商复杂（R-04）→ 只用固定 caps 固定管线，禁用动态重协商路径。
- appsink 回调阻塞 → 回调内只做拷贝入队，禁止任何重活（§3.4）。

**10. 应保留的测试证据**
`build.log`、`test-results.json`、`perf.json`（帧率/PTS 单调性原始数据）。

**11. 状态**：未开始

---

## M1-T3 · 共享内存视频 Ring Buffer 与契约测试

**1. Task ID 与目标**
在 `device-bridge` 中实现预分配共享内存视频 Ring：固定槽位 + 帧描述符（格式、尺寸、代际序号 generation seq、PTS、graph_version）；写端（Rust）单写多读；**契约测试覆盖坏长度、版本不符、旧句柄/旧代际、seq 重放**（基线 §10.1 Contract 测试要求）。

**2. 对应需求 FR / 架构 ADR**
- FR-003（统一 Runtime 输出被设备读取）、FR-006（缓冲边界）；ADR-005、ADR-006；基线 §4.2（MediaFrameEnvelope 逻辑契约）、§5.3（视频数据行）、§9 M1-2、§10.1。

**3. 前置依赖**
M1-T1。

**4. 允许修改的模块与文件范围**
- 修改：`runtime/device-bridge/`
- 新建：`contracts/ipc/`（v0.1 二进制布局头：魔数、版本、长度、对齐、端序、句柄生命周期）
- 新建：`tests/contract/`

**5. 实施步骤**
1. 定义共享内存节头布局（对齐 §4.2：session/track 标识以定长 ID 或哈希表达，避免热路径动态字符串；魔数+版本+长度字段必备）。
2. 实现 Ring：N 槽位环形、每槽带代际序号；读者通过代际判断新帧/旧帧。
3. 写端 API：`write_frame(desc, payload)` 满时按策略覆盖最旧帧（记录丢弃计数）。
4. 读端 API：`acquire_reader()` / `read_latest(generation)`；旧代际返回明确错误码。
5. 契约测试：坏魔数、坏长度、版本不符、旧句柄重用、seq 重放、写端崩溃后读端行为。
6. ACL：创建共享内存对象时设置最小 ACL（详单记录；跨会话访问按 M0-T4 决策执行）。

**6. 自动化测试要求**
- 单元测试（环形逻辑、代际翻转）；契约测试（上述 6 类异常全部有断言）。

**7. 真实环境验收要求**
- 真实双进程测试（写端/读端独立进程）通过；**进程内单进程测试只能算单元级证据**。

**8. 完成标准（DoD）**
- Ring 有界（内存上限固定）；契约测试全绿；布局文档（contracts/ipc/v0.1）入库。

**9. 风险及失败处理**
- 跨会话 ACL 不可行（R-02 兑现）→ 停止本任务，回到 M0-T4 决策的备选路径，走 ADR。
- 代际回绕/句柄重用 → 用 64 位代际 + 显式失效码；契约测试钉死行为。

**10. 应保留的测试证据**
`test-results.json`（契约测试原始输出）、`contracts/ipc/v0.1` 文档、`build.log`。

**11. 状态**：未开始

---

## M1-T4 · Named Pipe 控制通道（版本化 + ACL + 身份校验）

**1. Task ID 与目标**
实现本地控制通道：连接握手（协议版本协商）、身份校验（调用者 SID 校验，沿用 M0-T4 PoC 结论）、消息版本化、限速与超时、错误码表；为虚拟摄像头提供「服务在线/离线、格式、健康状态」控制面。**技术债责任方**：`docs/TECH_DEBT.md` P1-1（管道最小权限/身份认证/跨会话访问控制/攻击场景测试）、P1-2（异步 I/O 取消机制重写 + 超时竞争/取消完成/句柄泄漏自动化测试——M0-T4 PoC 的 server.cpp 现有实现不得未经修正复制进产品）、P1-3（帧承载吞吐实测的传输侧）；实现约束遵循 **ADR-012** §3 D3 七条（含 U1：NT SERVICE\FrameServer per-service SID 作 DACL 主体的实测）。

**2. 对应需求 FR / 架构 ADR**
- FR-006（故障降级与观测）、FR-009（授权控制的前提通道）；ADR-006；基线 §5.3（控制与状态行）、§10.1 安全测试（Named Pipe ACL）、§11-3（最小权限）。

**3. 前置依赖**
M1-T1（M0-T4 的 PoC 结论作为输入）。

**4. 允许修改的模块与文件范围**
- 修改：`runtime/device-bridge/`
- 新建：`contracts/ipc/control-v0.1`（消息表）、`tests/contract/`、`tests/security/`

**5. 实施步骤**
1. 消息表：HELLO/HELLO_ACK、HEARTBEAT、FORMAT_ANNOUNCE、HEALTH、SHUTDOWN；每消息带版本与长度头。
2. 服务端 Pipe 创建：显式 DACL，仅允许目标账户（media-service 与虚拟摄像头侧）访问；校验客户端身份后才开始消息交换。
3. 超时与限速：读写字节超时；异常客户端（超频发送）断连并记录。
4. 版本协商：版本不符 → 拒绝并回明确错误码（契约测试断言）。
5. 安全测试：非授权用户连接被拒、伪造长度/版本被拒、路径操纵（管道名注入）无效。

**6. 自动化测试要求**
- 契约测试：版本不一致、坏长度、坏句柄、重放 seq（§10.1 Contract 测试）。
- 安全测试：未授权 SID 拒绝、ACL 生效（§10.1 安全测试）。

**7. 真实环境验收要求**
- 真实跨进程（最好跨用户上下文）测试留存日志；**Mock IPC 对象上的通过不构成验收证据**。

**8. 完成标准（DoD）**
- 控制通道测试全绿；错误码表文档化；安全检查单通过。

**9. 风险及失败处理**
- ACL 与 Frame Server 侧账户冲突（R-02）→ 引用 M0-T4 决策；若决策失效，停止并升级为 ADR。
- 消息风暴拖垮服务端 → 限速 + 有界接收缓冲，测试钉死。

**10. 应保留的测试证据**
`test-results.json`、`security-checklist.md`（ACL/身份校验检查单）、`contracts/ipc/control-v0.1`。

**11. 状态**：未开始

---

## M1-T5 · VirtualCamera MediaSource 改为 IPC ring 读帧

**1. Task ID 与目标**
在官方 Sample 基础上新增 IPC 输入：MF Source 从 `device-bridge` 共享内存 ring 读取已准备帧，输出 Video Sample；服务断开时输出**稳定占位帧**并上报设备健康；重连后恢复。**不在 Frame Server COM 回调中做网络/AI/超预算等待**（§5.1 禁用模式）。**技术债责任方**：`docs/TECH_DEBT.md` P1-1（客户端侧对接验证）、P1-3（帧承载实测的消费侧）；**帧承载通道按 ADR-012 §3 D2 条件式决策为准**（共享内存 ring 用户侧前提已被 M0-T4 T7b 实测否定，管道复用为当前首选，最终以 P1-3 实测结果修正实现路径）。

**2. 对应需求 FR / 架构 ADR**
- FR-003（设备读统一 Runtime 输出）、FR-007（模型/服务故障不拖垮设备）、FR-006；ADR-003、ADR-006；基线 §5.1 最小路径 4~6 步、§3.4 进程边界表、§9 M1-3/4。

**3. 前置依赖**
M0-T4（R-02 决策）、M1-T3（ring）、M1-T4（控制通道）。

**4. 允许修改的模块与文件范围**
- 修改：`platforms/windows/virtual-camera/`（在 Sample 结构上扩展 IPC Input；与上游 Sample 的差异逐条记录）
- 允许引用：`contracts/ipc/` v0.1；**禁止**修改 `runtime/`（Rust 侧在本任务冻结）。

**5. 实施步骤**
1. 在 Media Source 内实现 ring 读端（映射共享内存、按代际取最新帧）。
2. 帧格式转换：ring 帧（NV12/BGRA）→ MF MediaBuffer/Sample，保持 PTS 传递。
3. 断连降级：写端消失/心跳超时 → 切换内置占位帧（静态测试卡），置健康标志；重连自动恢复并记录事件。
4. 读取预算：单次回调内 ring 读取/拷贝设定硬预算（如 ≤10ms 量级，写入文档）；超预算走占位帧路径。
5. C++ 侧单元测试（可用性受 MF 环境限制时，明确记录哪些部分只能集成验证）。

**6. 自动化测试要求**
- 可自动化部分：ring 读端逻辑（纯 C++ 测试）、降级状态机。
- 其余（MF 呈现）由 M1-T6 真机集成覆盖。

**7. 真实环境验收要求**
- 编译产物真实注册并在 Windows Camera 呈现来自 ring 的帧（此为本任务与 M1-T6 共同的真机证据；本任务至少完成「编译+注册+读到测试帧」的真机冒烟）。Mock 帧仅在 C++ 单元测试中允许。

**8. 完成标准（DoD）**
- IPC Input 可用、断连占位帧生效；对 Sample 的修改清单文档化；无回调内超预算等待（代码审查项）。

**9. 风险及失败处理**
- Frame Server 内无法打开用户会话创建的共享内存（R-02）→ 按 M0-T4 决策切换备选（可信本地代理）；改动走 ADR。
- MF 回调阻塞导致系统级卡顿 → 硬预算 + 占位帧兜底；禁止无限等待（§5.1 严禁）。

**10. 应保留的测试证据**
`build.log`（C++ 构建含警告清单）、`test-results.json`、`screenshots/`（真机冒烟）、`device-enumeration.txt`。

**11. 状态**：未开始

---

## M1-T6 · 端到端验证与鲁棒性基础（多消费者/用户切换/重复安装卸载）

**1. Task ID 与目标**
Rust Runtime → 共享内存 ring → 虚拟摄像头 → 真实应用的完整链路：受控 720p30 稳定输出、格式协商生效、断连安全降级、重连恢复；并完成基线 §5.1 第 7 步要求的基础场景：**多消费者并发、用户切换、重复安装/卸载**。

**2. 对应需求 FR / 架构 ADR**
- FR-001、FR-003、FR-006、FR-007、FR-008（安装卸载清理）；ADR-003、ADR-005、ADR-006；基线 §5.1、§9 M1-3/4、§12 R-03/R-12；NFR-01（本任务初步）、NFR-09（基础场景）。

**3. 前置依赖**
M1-T2、M1-T3、M1-T4、M1-T5。

**4. 允许修改的模块与文件范围**
- 修改：`runtime/device-bridge/`、`runtime/media-core/`（指标与降级逻辑）、`platforms/windows/virtual-camera/`（端到端联调修复）、`tests/integration/`、`tools/collect-evidence/`
- 新建：`tests/fault-injection/`（kill 场景脚本）

**5. 实施步骤**
1. 联调链路：Runtime 写 720p30 测试卡 → Windows Camera 持续渲染；帧率与 PTS 统计落盘。
2. 故障演练：kill Rust Runtime → 虚拟摄像头切占位帧、**设备不崩溃、注册不消失**；重启 Runtime → 恢复（NFR-06 预演，正式指标在 G1 汇总）。
3. 多消费者并发：Windows Camera + Chromium/OBS 同时打开；记录共享/互斥事实（衔接 M0-T3 结论，R-03/R-12）。
4. 用户切换：切换会话后设备可见性与帧流行为记录。
5. 重复安装/卸载：注册-卸载循环 ≥3 次，检查注册表与文件残留、孤儿进程（NFR-09 基础清单）。
6. 格式协商：caps 变更（720p→640x360）走一遍，记录协商结果与失败处理。

**6. 自动化测试要求**
- 集成脚本：设备枚举断言、帧率采样 ≥30s、kill-恢复序列自动化、卸载残留检查（注册表键/文件/进程清单比对）。

**7. 真实环境验收要求**
- 全部场景在真实 Windows + 真实应用中执行并留存截图/日志/原始数据。**CI 通过 ≠ 本任务验收**。

**8. 完成标准（DoD）**
- 720p30 稳定（≥30s 无死锁）；kill 后占位帧生效且可恢复；多消费者/用户切换/重复装卸事实记录完整（含失败场景）；残留检查干净。

**9. 风险及失败处理**
- 多应用互斥（R-03/R-12）→ 记录为已知限制进 decision.md，控制面策略（显式互斥/共享）留 M2/M5 完善。
- 残留清理失败（FR-008 风险）→ 列清单修复；不得带残留进入 M2。

**10. 应保留的测试证据**
`perf.json`（帧率/PTS）、`test-results.json`、`device-enumeration.txt`、`screenshots/`、故障注入时序日志。

**11. 状态**：未开始

---

## M1-T7 · 物理摄像头输入接入

**1. Task ID 与目标**
Runtime 增加物理摄像头采集源（经由 OS 共享能力打开，禁止盲目二次独占），作为 ring 帧源之一；OS 隐私权限流程可见可撤销；物理设备断连时降级到测试卡。

**2. 对应需求 FR / 架构 ADR**
- FR-005（真实摄像头输入 + 设备授权控制）、FR-006、FR-009（采集可见可撤销）；基线 §9 M1-5、§12 R-03；§11-2（显式媒体状态）。

**3. 前置依赖**
M1-T6（端到端链路稳定）。

**4. 允许修改的模块与文件范围**
- 修改：`runtime/gst-adapter/`（采集源封装）、`runtime/media-core/`（源切换与授权状态）
- 新建：`tests/integration/`（采集源测试）

**5. 实施步骤**
1. 以共享模式打开系统默认摄像头（记录设备名/驱动版本进 environment.json）；确认 OS 隐私设置（设置 → 隐私和安全性 → 摄像头）对采集的影响并记录。
2. `mfvideosrc`/`ksvideosrc`（按 gst 插件实测选定）→ capsfilter → appsink，接入现有 ring 写端。
3. 源切换：测试卡 ↔ 物理摄像头，无缝切换（帧流不断、PTS 连续性记录）。
4. 断连演练：拔/禁用摄像头 → 降级测试卡 + 健康事件；重插恢复。
5. 权限撤销演练：关闭 OS 摄像头权限 → 采集停止且不崩溃，状态可见（NFR-08 相关）。

**6. 自动化测试要求**
- 集成测试：源切换序列、断连-恢复序列可脚本化重放；帧率统计。

**7. 真实环境验收要求**
- 必须使用**真实物理摄像头**；无摄像头则本任务 HOLD 并记录（不得用虚拟源冒充物理设备完成验收）。

**8. 完成标准（DoD）**
- 物理源输出进入 ring；切换/断连/权限撤销行为符合 §4.3（权限撤销立即阻止采集进入处理）并有证据。

**9. 风险及失败处理**
- 设备被其他应用占用（R-03）→ 以共享模式打开 + 失败时明确报错，禁止独占重试风暴。
- OS 权限行为与预期不符 → 记录最小复现 + 官方依据，不猜测。

**10. 应保留的测试证据**
`perf.json`、`test-results.json`、`screenshots/`（真机画面）、`environment.json`（设备信息段）。

**11. 状态**：未开始

---

## M1-T8 · G1 Gate 汇总（GO / HOLD / STOP）

**1. Task ID 与目标**
按 G1 验收原文逐句验收：Windows Camera 与第二款应用持续使用 30 分钟、AI 零依赖、杀 Runtime 后设备不崩溃并进入安全备用画面、队列有界；输出 NFR-01（30fps/30min）、NFR-07（队列有界）初步测量与 **NFR-03 初测**（Camera 输出新增时延，同步闪光测试卡法）；按 §14.2 出 `GATE=G1` 判定并经用户确认。

**2. 对应需求 FR / 架构 ADR**
- FR-001、FR-003、FR-006、FR-007、FR-008；ADR-003、ADR-005、ADR-006、ADR-010；基线 §9 Gate G1、§7 NFR-01/NFR-07、§14.2。

**3. 前置依赖**
M1-T6、M1-T7 到达**任意终态**（已完成 / HOLD / STOP / BLOCKED / NOT_RUN——AGGREGATE 语义，ADR-011 D2；T7 的非完成结论须显式记录）。上游失败/阻塞时本任务照常产出 HOLD/STOP 报告；`STATUS=GO` 仍要求全部强制条件通过。

**4. 允许修改的模块与文件范围**
- 生成：`artifacts/G1/<final-run-id>/`（全套九文件）
- 修改：`docs/tasks/MASTER_TASK_PLAN.md` 状态表

**5. 实施步骤**
1. 30 分钟连续运行：双应用消费 720p30，采样帧率、内存、队列水位（NFR-01/07 测量条件按 §7 记录：Windows Build、CPU/GPU、依赖版本、场景、采样窗口）。
2. kill 演练正式化：kill Runtime ≥3 次，记录设备注册保持、备用画面切换时间、恢复时间（NFR-06 预演数据，正式验收在 M3-T7）。
3. 队列有界验证：关闭消费端 + 减速处理端 30 分钟，内存/队列无持续线性增长（NFR-07）。
4. NFR-03 初测：同步闪光测试卡法测 Camera 输入→可消费帧新增时延（p50/p95），测量条件全记录。
5. 证据组装（九文件）+ 自检 G1 验收句。
6. §14.2 十字段汇报 → 用户确认 GO/HOLD/STOP。

**6. 自动化测试要求**
- 全部集成/契约/故障注入测试套件重跑全绿；性能采样脚本输出 perf.json。

**7. 真实环境验收要求**
- OS_EVIDENCE 必须是真实设备 + 真实应用的 30 分钟记录。**任何 Mock/CI 结果不得替代**；缺真机证据 ⇒ HOLD。

**8. 完成标准（DoD）**
- G1 判定发布并确认；状态表更新；GO ⇒ M2 解锁。

**9. 风险及失败处理**
- 30 分钟不稳定 → 记录崩溃/死锁 dump，STOP 项进入 M1 返工清单，不带病进入音频里程碑。

**10. 应保留的测试证据**
完整 `artifacts/G1/<final-run-id>/` 九文件（perf.json 含 30min 原始采样）。

**11. 状态**：未开始
