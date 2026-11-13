# M0 任务明细 — 环境与可行性验证（Gate G0）

> **来源**：`docs/ARCHITECTURE_BASELINE.md` §9 M0（架构基线 v1.0，唯一事实来源）。
> **门禁前置**：无（首个里程碑）。**门禁产出**：G0。
> **G0 验收原文**（§9）：camera synthetic works；实际测试程序、Windows Build、安装和卸载步骤可重复；SysVAD 风险已形成书面判断。**若摄像头样例均失败，STOP，先排除工具链/环境再写引擎**。
> **字段约定**：每项任务含 11 个字段；状态一律 `未开始`，只有真实完成并保留证据后方可由执行者更新（更新规则见 `MASTER_TASK_PLAN.md`）。

## 任务总览

| ID | 目标摘要 | 前置依赖 | AUTH | 状态 |
|---|---|---|---|---|
| M0-T1 | 环境盘点脚本 + 工具链补齐（VS Build Tools、GStreamer MSVC 版） | — | — | 已完成（run G0-20261111-174257-8532） |
| M0-T2 | 官方 VirtualCamera Sample 构建、注册、枚举、卸载 | M0-T1 | — | 已完成（run G0-20261111-182351-1DB8） |
| M0-T3 | 双独立应用消费验证 + 生命周期与可复现步骤 | M0-T2 | — | 已完成（run G0-20261113-201619-b4dc；App A 已知缺陷见 decision.md） |
| M0-T4 | Frame Server 进程约束验证 + IPC 身份 PoC + R-02 决策 | M0-T2 | — | 已完成（run G0-20261112-191515-3C36） |
| M0-T5 | SysVAD 风险书面评估（不构建、不安装驱动） | — | —（升级为构建/安装任务时须另行授权） | 已完成（run G0-20261111-212232-D720） |
| M0-T6 | `contracts/` 骨架 + 证据基线设施 + `docs/adr/` 约定 | M0-T1 | — | 已完成（run G0-20261113-153144-57a0） |
| M0-T7 | **G0 Gate 汇总**：证据组装 + GO/HOLD/STOP 判定（AGG） | M0-T3~T6 终态（AGG） | — | 已完成（run G0-20261113-223247-7bc7；**G0=GO 经用户确认 2026-11-13**，M1 解锁） |

---

## M0-T1 · 环境盘点脚本与工具链补齐

**1. Task ID 与目标**
建立 `tools/verify-environment` 只读探测脚本，自动生成符合基线 §10.3 格式的 `environment.json`；根据盘点结果补齐缺失工具链（Visual Studio Build Tools C++ 工作负载、GStreamer MSVC 版 runtime+devel）。工具链安装属**普通开发软件安装**（非驱动、非安全策略），执行前仍须向用户逐项确认。

**2. 对应需求 FR / 架构 ADR**
- 支撑 FR-001 / FR-002 / FR-005 的环境前提；基线 §9 M0-1、§10.2、§13（`tools/verify-environment` 需实施）。
- 相关风险：R-07（环境差异）、R-11（安全软件/管控阻止）。

**3. 前置依赖**
无。本任务是全体系第一个可执行任务。

**4. 允许修改的模块与文件范围**
- 新建：`tools/verify-environment/`（探测脚本 + schema）
- 新建/修改：`docs/platform-windows.md`（环境快照章节）
- 修改：`.gitignore`（如需补充）
- 生成：`artifacts/G0/<run-id>/environment.json`

**5. 实施步骤**
1. 编写只读探测脚本（PowerShell 为主）：记录 Windows 精确 Build/UBR/版本、CPU/GPU 及驱动版本、RAM/磁盘、git/Rust（rustc、cargo、rustup、默认 target）/Python、Visual Studio 与 MSVC 组件、Windows SDK 版本、WDK 有无、GStreamer 版本与路径、物理摄像头/麦克风在位性与隐私权限状态、安全软件/VPN/企业管控线索、多用户账户可建性、第二台隔离驱动测试机有无。
2. 定义 `environment.json` schema（字段名、类型、必填/可选），与基线 §10.3 对齐。
3. 在本机运行：已确认具备 Build 26200.9711（25H2）、git 2.56、rustup/cargo、Windows SDK 10.0.26100/22621、Python 3.14；已确认缺失 VS C++ 工具链与 GStreamer——逐项向用户确认后安装：
   - VS 2022 Build Tools（勾选 MSVC v143、Windows 11 SDK、CMake；不改任何 SDK 安全设置）；
   - GStreamer **MSVC 版** runtime + devel（不使用 MinGW 版）。
4. 安装后重跑脚本，`cl.exe`、`gst-launch-1.0 --version`、`cargo --version` 全部真实执行并记录。
5. 将环境快照写入 `docs/platform-windows.md`；无法安装或探测失败的项标注「不安装（原因）」。

**6. 自动化测试要求**
- schema 校验：对 `environment.json` 做字段齐全性/类型检查（好/坏样例各一组）。
- 幂等性：脚本重复运行结果一致（除时间戳）。

**7. 真实环境验收要求**
- `environment.json` 必须在目标机真实生成；版本命令输出真实留存。**Mock / 容器内模拟 / 手工编写的环境数据一律无效**。

**8. 完成标准（DoD）**
- 探测脚本入库且测试通过；`environment.json` 完整；缺失工具链已补齐或有明确「不安装 + 原因」记录；`docs/platform-windows.md` 含环境快照。

**9. 风险及失败处理**
- 安装被管控阻止（R-11）→ 记录为环境缺口并上报用户决策，**不得绕过企业/系统管控**。
- GStreamer 装成 MinGW 版 → 卸载重装 MSVC 版；ABI 与 Rust msvc 目标不匹配是 M1 的阻塞隐患。
- VS 安装体积大/失败 → 可先装 Build Tools 最小组件集，满足 Sample 构建即可。

**10. 应保留的测试证据**
`artifacts/G0/<run-id>/environment.json`、`build.log`（工具安装与版本命令输出）、`screenshots/`（版本命令截图，脱敏）、`licensing-sbom.json`（VS Build Tools、GStreamer 条目）。

**11. 状态**：已完成（run-id G0-20261111-174257-8532；2026-11-11——DoD 四条达成：脚本+自测 5/5、environment.json schema VALID/幂等 YES、工具链已补齐且 WDK 记录不安装原因、platform-windows.md 快照落档）

---

## M0-T2 · 官方 VirtualCamera Sample 构建、注册、卸载

**1. Task ID 与目标**
在受控环境**原样**构建 `microsoft/Windows-Camera` 仓库 `Samples/VirtualCamera` 示例（Synthetic source 模式），完成注册、枚举、Start/Stop/Remove/卸载；不修改 SDK 安全设置，不在示例中添加任何 AI 代码（§2.1「工程含义」）。

**2. 对应需求 FR / 架构 ADR**
- FR-001（注册、枚举、启停）；ADR-003、ADR-009；基线 §2.1、§5.1、§9 M0-2；参考 [S1][S2]。

**3. 前置依赖**
M0-T1（工具链就绪）。

**4. 允许修改的模块与文件范围**
- 新建：`platforms/windows/virtual-camera/`（Sample 获取脚本、构建配置、复现文档；**不在此阶段产出产品代码**）
- 新建：`third_party/`（Sample 来源 commit、许可证记录）
- 生成：`artifacts/G0/<run-id>/`

**5. 实施步骤**
1. 克隆官方仓库并**记录 commit/tag、许可证、依赖**进 SBOM（§8 依赖规则）。
2. 按 Sample README 用 VS/MSBuild 构建（记录所用 SDK 与工具集版本）；不修改 SDK 安全设置。
3. 按官方说明注册虚拟摄像头（官方 Sample 属 OS 认可路径；**禁止启用测试签名或绕过任何系统校验**）。
4. 在 Windows Camera 中枚举并打开设备，确认合成画面渲染。
5. Start / Stop / Remove 循环至少 3 次，观察设备出现/消失与系统稳定性。
6. 将完整可复现步骤（含精确命令、Build、版本）固化为文档。

**6. 自动化测试要求**
- 设备枚举断言脚本：注册后枚举含目标设备、Remove 后枚举不含；退出码可被后续任务复用。

**7. 真实环境验收要求**
- 真实 Windows Camera 应用看到并渲染合成画面（截图留证）；卸载后设备从系统消失（枚举日志留证）。**任何模拟枚举或 Mock 结果不作为本任务证据**。

**8. 完成标准（DoD）**
- 构建/注册/枚举/启停/卸载全流程可重复且文档化；若样例构建或运行失败，**触发 G0 STOP 路径**（§9：先排除工具链/环境再写引擎），不得带病进入 M1。

**9. 风险及失败处理**
- Build 版本不支持 / SDK 不匹配（R-07）→ 按 M0-T1 环境数据定位差异；必要时升 SDK（在普通软件安装范围内）。
- 注册权限不足 → 按官方文档的会话/系统授权模式处理；**不得**关闭 Secure Boot/设备防护。
- Sample 与当前 Build 26200 存在 API 差异 → 记录最小复现与官方依据（§14.1），不得把推测写成结论。

**10. 应保留的测试证据**
`environment.json`、`build.log`、`device-enumeration.txt`、`screenshots/`（Windows Camera 渲染合成画面）、`test-results.json`（启停循环结果）、`licensing-sbom.json`。

**11. 状态**：已完成（2026-11-11，run `G0-20261111-182351-1DB8`）——3 轮注册/拉流/注销循环全通过；Windows Camera 合成画面像素级留证；完整回滚验证（ABSENT + COM=no + 目录已删）。关键事实与偏差（BuildTools 机器 MSI 不可构建 → README 键 + 官方 Installer 控制台替代）见 run `decision.md`；复现步骤见 `platforms/windows/virtual-camera/README.md`。

---

## M0-T3 · 双独立应用消费与生命周期验证

**1. Task ID 与目标**
至少两个**相互独立**的应用（应用 A：Windows Camera；应用 B：Chromium `getUserMedia` 页面或 OBS）能枚举、打开、关闭虚拟摄像头；记录应用互斥/共享行为与可复现步骤，为 M1 多消费者设计（R-03/R-12）提供事实输入。

**2. 对应需求 FR / 架构 ADR**
- FR-001、ADR-009；基线 §9 M0-3、§10.2（兼容性矩阵最低组合）。

**3. 前置依赖**
M0-T2。

**4. 允许修改的模块与文件范围**
- 修改：`docs/platform-windows.md`（双应用验证记录）
- 新建：`tests/fixtures/`（getUserMedia 测试页、枚举脚本）
- 生成：`artifacts/G0/<run-id>/`

**5. 实施步骤**
1. 准备应用 A（Windows Camera）与应用 B（Chromium + 本地 getUserMedia 测试页，或 OBS——记录精确版本）。
2. 分别单独打开，确认各自渲染合成画面。
3. 并发打开 A+B，记录帧争夺/互斥/共享的实际行为。
4. 对设备执行 Stop/Start，观察两个应用的表现（自动恢复/需手动重开/黑屏）。
5. Remove 设备后观察两个应用表现与系统日志。
6. 记录全矩阵结果（应用 × 操作 × 结果），版本号齐全。

**6. 自动化测试要求**
- 复用 M0-T2 枚举断言脚本；（可选）测试页 JS 输出采集帧计数，作为「应用真实读到帧」的量化证据。

**7. 真实环境验收要求**
- 两个真实应用各自渲染合成画面，截图 + 版本记录齐全。**任一应用失败必须如实记录失败现象**，不得用「理论可行」替代实测结论。

**8. 完成标准（DoD）**
- 双应用矩阵完成；互斥/共享行为有事实记录；步骤可重复。

**9. 风险及失败处理**
- 应用互斥导致并发失败（R-03、R-12）→ 如实记录互斥事实，作为 M1-T6/M1-T7 的设计输入；这不是 M0 失败条件，但必须写进 decision.md。
- OBS/浏览器版本差异 → 记录版本；兼容性问题留到 M5-T4 矩阵系统化。

**10. 应保留的测试证据**
`device-enumeration.txt`、`screenshots/`（双应用画面）、`test-results.json`（应用 × 操作矩阵）、`environment.json`（应用版本段）。

**11. 状态**：**已完成（2026-11-13，run `G0-20261113-201619-b4dc`，STATUS=PASS 含已知缺陷）**——应用 B（Chrome 157.0.8103.51）全路径 PASS（独立/并发(不同设备变体)/Stop/Start/移除 ENDED/重注册手动恢复）；应用 A（Windows 相机 2026.2608.1.0）首次渲染合成帧成功但切换间歇 CameraSwitchFailed(0xA00F4241) 且错误态不可自愈，判定 FAIL（稳定消费能力），已作为 R-03/R-12 设计输入写入 run decision.md §2/§2a；重复注册/注销 A1→B2→A3→B4 四轮无残留，SC-5 完整回滚终态干净。证据九文件齐全 + manifest 人工审计（INTEGRITY-OK）。

---

## M0-T4 · Frame Server 进程约束验证与 IPC 身份 PoC（R-02 决策门）

**1. Task ID 与目标**
查明 Sample Media Source 在 Frame Server 中的真实运行形态：服务账户、Session、进程位数、文件/桌面访问限制、COM 激活、生命周期；验证 Frame Server 服务重启与多用户场景；设计最小 IPC 身份认证 PoC；**输出 R-02 备选方案书面决策（可信本地代理 vs 共享内存服务直连）。此决策未通过则 G0 不得 GO**（§3.4、§12 R-02）。

**2. 对应需求 FR / 架构 ADR**
- FR-001/FR-003 的实现前提；ADR-006（进程隔离、本地 IPC）；基线 §2.1、§3.4（Windows 特别事项）、§12 R-02、§9 M0-4；参考 [S3]。

**3. 前置依赖**
M0-T2（Sample 已在运行）。

**4. 允许修改的模块与文件范围**
- 修改：`docs/platform-windows.md`（Frame Server 事实章节）
- 新建：`tools/verify-environment/` 扩展（进程/ACL 探测子命令）
- 新建：`tests/contract/`（身份认证 PoC 及其测试；PoC 为独立探针程序，**不并入产品**）
- 生成：`artifacts/G0/<run-id>/decision.md`

**5. 实施步骤**
1. 用 Process Explorer / `tasklist /svc` / Process Monitor 定位承载 Media Source 的 Frame Server 进程，记录：进程名、PID、账户（预期为低权限服务账户）、Session（预期 0）、位数、加载的 Sample DLL 路径。
2. 用 `icacls` / `Get-Acl` 记录该账户对用户目录、`%PROGRAMDATA%`、Named Object 目录的默认访问权。
3. 编写最小 IPC 身份认证 PoC：服务端 Named Pipe 校验客户端 SID 属 Frame Server 服务账户；非授权客户端被拒绝（含测试用例）。
4. 重启 Frame Server 服务（`net stop/start` 或服务管理器），观察虚拟摄像头与 Media Source 的恢复行为；记录恢复时间与日志。
5. 多用户观察：快速用户切换至第二会话，记录设备可见性与 Media Source 行为。
6. 撰写 R-02 决策记录：实测事实表 → 可行路径（共享内存 + ACL 的安全边界）→ 备选路径（用户会话内可信本地代理中转）→ 各自成本/风险 → 结论与未解决问题（引用 [S3]）。

**6. 自动化测试要求**
- PoC 程序单元/集成测试：授权客户端连接成功、非授权 SID 被拒、无句柄泄漏；测试随 `tests/contract/` 入库。

**7. 真实环境验收要求**
- 全部观察必须来自真实 Frame Server 进程（进程树截图、`whoami` 输出、ACL dump）。**在同一交互式会话中用普通进程模拟成功不能替代本任务验证**（§3.4 明文）。

**8. 完成标准（DoD）**
- 账户/Session/位数/ACL 事实表完成；服务重启与多用户行为有记录；R-02 决策有书面结论、依据与备选方案；若跨会话 IPC 被证实不可行且备选未获认可 → G0 记 HOLD/STOP 并写明阻塞点。

**9. 风险及失败处理**
- R-02 兑现（Frame Server 账户无法访问用户会话对象，R-02 高风险本体）→ 决策文档给出备选路径与成本，**交用户决策**；不得擅自修改系统服务配置或 ACL 系统默认值。
- 探测需要提权 → 仅使用只读查询类工具；任何提权操作单独列出征得用户同意。

**10. 应保留的测试证据**
`environment.json`（Frame Server 补充段）、`device-enumeration.txt`、`screenshots/`（进程树、whoami、ACL）、`security-checklist.md`（身份认证 PoC 检查单）、`test-results.json`（PoC 测试）、`decision.md`。

**11. 状态**：已完成（run `G0-20261112-191515-3C36`，2026-11-12~13）——判定 **M0-T4=PASS、R-02=GO**（run `decision.md` §1/§4）。Frame Server 事实 F1-F11 真机探明：LocalService（S-1-5-19）/Session 0/完全按需启停/SCM 7036 不记录启停/提权模块表确认 VirtualCameraMediaSource.dll 加载/sdshow 交互用户仅查询权限（§2）。最小 IPC 身份 PoC（源码入库 `tests/contract/ipc-identity-poc/`）12 项矩阵：11 pass + 1 partial-with-finding（T5：LS 可 OS 级连接用户 DACL 授权管道，但跨会话服务端进程级 token 校验不可行→fail-closed 按设计拒绝）；T7b 实测否定基线 §5.1 共享内存用户侧前提（用户无 SeCreateGlobalPrivilege 不能建 Global\）；T7a 反向（LS 写→用户读）可行；T6/T9 反证（§3）。SC#3 服务重启 0s 按需恢复 + 多用户同账户双会话拉流 29/30 帧通过（§5）。SYS_CHANGE 3 项（设备注册/LS 探针计划任务/服务重启）均经用户确认并完整回滚（ABSENT OK，§6）。R-02 决策：Named Pipe 首选（帧随控制通道，720p30 吞吐留 M1-T4/T5 实测）、LS broker Global section 备选（仅吞吐不足时，需新 ADR）、管道 DACL=OS 身份边界五条（§4.3）；待 M0-T6 建立 `docs/adr/` 后正式化为 ADR（涉及基线 §5.1 策略调整，§14.3）。perf.json N/A（无性能验收指标）；K1-K7 未验证项见 run `decision.md` §7

---

## M0-T5 · SysVAD 风险书面评估（不构建不安装）

**1. Task ID 与目标**
纯书面研究并输出风险结论：SysVAD 构建条件（WDK 版本矩阵）、驱动签名路径与成本（attestation / HLK / 测试签名）、隔离测试机要求、许可与分发条款。**本任务禁止构建或安装任何驱动**；输出第二台隔离测试机策略与 Track B 启动条件。

**2. 对应需求 FR / 架构 ADR**
- FR-002（虚拟麦克风另设门禁）；ADR-004（不假定仅用摄像头 API 造麦克风；不在日常工作机开测试签名）；基线 §2.2、§5.2、§9 M0-5、§12 R-01；参考 [S4][S5][S17][S18]。

**3. 前置依赖**
无（可与 M0-T1 并行）。

**4. 允许修改的模块与文件范围**
- 修改：`docs/platform-windows.md`（音频章节）、`docs/licensing.md`（SysVAD/WDK 条目）
- 生成：`artifacts/G0/<run-id>/decision.md`（音频部分）、`licensing-sbom.json`

**5. 实施步骤**
1. 梳理 SysVAD 对 WDK/EWDK 版本、目标 Windows 版本的兼容矩阵（官方文档引用 + 访问日期）。
2. 梳理签名路径与成本：attestation signing、HLK 认证、测试签名的适用范围与限制（[S5]）。
3. 明确硬边界并写入文档：**不得在主力办公机/企业受控机关闭安全功能为驱动试验让路；不建议绕过 Secure Boot/BitLocker；测试签名仅限专用、可恢复的测试机并按官方流程操作后恢复**（§2.2）。
4. 评估第二台隔离驱动测试机的有无；**无测试机 ⇒ Track B 全程 HOLD**，主线按 Track A 推进。
5. 调研 Track A（用户主动安装、许可明确的第三方虚拟音频路由工具）的候选清单、许可证与分发条件（不默认随产品捆绑）。
6. 输出 decision.md：风险、成本、GO/HOLD 触发条件、Track A/B 边界声明。

**6. 自动化测试要求**
- 无（研究类任务）。检查单：每条结论必须附官方来源引用；凡未经实测的判断一律标注「待实测确认」。

**7. 真实环境验收要求**
- 不适用（本任务**禁止**真实驱动操作）。任务完成的标志是文档完整，而非任何驱动行为。

**8. 完成标准（DoD）**
- decision.md 完成；本任务为纯书面研究、无需授权，但 SysVAD 相关任何构建/安装任务启动前必须另行登记 AUTH_REQUIRED 并获用户显式授权（PENDING 不得启动）。

**9. 风险及失败处理**
- 低估签名/HLK 成本 → 所有成本结论标注待确认；G2 复评（M2-T6）。
- 官方文档与实际工具链冲突 → 按 §14.3 记录「事实依据 + PoC 证据 + 决策」，走 ADR 更新基线版本，禁止静默覆盖。

**10. 应保留的测试证据**
`decision.md`（含引用清单）、`licensing-sbom.json`（SysVAD、WDK、Track A 候选工具条目）。

**11. 状态**：已完成（run G0-20261111-212232-D720，2026-11-11）——纯书面研究，六步全部完成，无任何构建/安装/系统变更；产出 run `decision.md`（六步结论 + 官方引用清单，访问日期 2026-11-11）+ run `licensing-sbom.json` 0.3（6 条 research-only 条目：SysVAD/WDK/EWDK/VB-Cable/VoiceMeeter/VAC）+ `docs/platform-windows.md` §6（音频栈/构建矩阵/签名四路径/硬边界）+ `docs/licensing.md` v0.1；关键结论：SysVAD main 需 v142 工具集（本机无，缺口书面确认）、原样 sample 不可过 HLK、本机永不执行测试签名、测试机 unconfirmed ⇒ Track B HOLD（复评触发三条件已登记）、Track A 三候选许可/分发完成（不默认随产品捆绑）；未实测判断已标「待实测确认」；SysVAD 任何构建/安装启动前须另行 AUTH_REQUIRED

---

## M0-T6 · contracts/ 骨架与证据基线设施

**1. Task ID 与目标**
建立最小 `contracts/` 目录骨架（media / ipc / capability 占位与冻结流程说明）、证据自动化设施（`tools/collect-evidence` 初版 + §10.3 九文件 schema）、`docs/adr/` 目录与 ADR 模板及变更控制约定（§14.3）。

**2. 对应需求 FR / 架构 ADR**
- FR-006（观测与故障降级的记录前提）、ADR-005（统一契约）、ADR-010（证据化验收）；基线 §9 M0-6、§10.3、§13、§14.3。

**3. 前置依赖**
M0-T1（脚本运行时环境）。

**4. 允许修改的模块与文件范围**
- 新建：`contracts/media/`、`contracts/ipc/`、`contracts/capability/`（README 级占位）
- 新建：`tools/collect-evidence/`
- 新建：`docs/adr/0001-template.md` 等
- 修改：`.gitignore`（确认 `artifacts/` 忽略）

**5. 实施步骤**
1. `contracts/README`：目录约定、契约 v0.1 冻结流程（M2-T1 执行）、版本兼容承诺。
2. 证据九文件 JSON schema：`environment / build / test-results / perf / device-enumeration / screenshots / security-checklist / licensing-sbom / decision`（对齐 §10.3）。
3. `tools/collect-evidence` 初版：`--gate --run-id` 创建 `artifacts/<gate>/<run-id>/` 骨架；`--from verify-environment` 拉取 environment.json；校验 schema。
4. ADR 模板 + 触发条件清单（改三平面边界、`contracts/` 协议、OS Adapter 策略、音频签名策略 ⇒ 新 ADR，§14.3）。
5. 幂等性检查：重复运行 collect-evidence 不破坏已有 run-id 目录。

**6. 自动化测试要求**
- schema 校验测试（合法/非法样例）；collect-evidence 幂等与参数校验测试。

**7. 真实环境验收要求**
- 在本机真实生成一次 `artifacts/G0/<run-id>/` 骨架（不伪造内容），验证脚本在真实 PowerShell/目标机上运行。

**8. 完成标准（DoD）**
- 目录、模板、工具可用；后续所有 Gate 汇总任务可直接复用；不做任何具体协议字段定义（协议属 M1-T3 / M2-T1）。

**9. 风险及失败处理**
- 过度设计（提前设计协议细节，R-10 同型风险）→ 本任务只做占位与 schema，协议内容留待对应任务。

**10. 应保留的测试证据**
`test-results.json`（schema/幂等测试）、`build.log`、`licensing-sbom.json`（脚本依赖，若有第三方库）。

**11. 状态**：已完成（run `G0-20261113-153144-57a0`，2026-11-13）——contracts/ 三子目录占位 README（冻结流程+版本承诺，无协议字段，R-10 同型风险受控）；`tools/collect-evidence` v1.0（init/manifest/validate/verify/--selftest，stdlib-only；字节级幂等；init 拒绝已存在 run；selftest 16/16 + CLI 合法/非法/重复执行场景 9/9，test-results.json 25 项全过）；`docs/adr/` 机制补齐（README 编号/生命周期/§14.3 触发条件清单 + TEMPLATE + ADR-011 索引登记；ADR-011 已在库 ⇒ 编号检查后新 ADR=**ADR-012**）；**ADR-012**：R-02 正式化 CONDITIONAL_GO——Named Pipe 控制通道 Accepted、帧承载=管道复用首选且是否保留由 M1-T4/T5 实测（P1-3）决定（未达标不得标记生产可用）、七条安全设计要求 S1-S7、共享内存备选启用条件（D4）；新增实测事实 V7（FrameServer SERVICE_SID_TYPE=UNRESTRICTED + per-service SID，只读查询）；`docs/TECH_DEBT.md` 新建（P1-1/P1-2/P1-3 正式登记 OPEN，未验证不得 CLOSED）；M1-T4/T5 任务字段挂接技术债；基线文件零改动（§5.1 修订记录于 ADR-012 §5）；零系统状态变更（SYS_CHANGE=[]）；偏差 3 项记 run decision.md §3（TEMPLATE 文件名/–-machine-json 替代 –-from/adr 目录已存在）

---

## M0-T7 · G0 Gate 汇总（GO / HOLD / STOP）

**1. Task ID 与目标**
组装全部 G0 证据到最终 run-id 目录，按基线 §14.2 十字段模板输出 `GATE=G0` 判定；**摄像头样例失败 ⇒ STOP**；R-02 决策未通过 ⇒ HOLD/STOP。判定结果须经用户确认。

**2. 对应需求 FR / 架构 ADR**
- ADR-009（首个里程碑必须验证 OS 识别并消费设备）、ADR-010（证据化验收）；基线 §9 Gate G0、§14.2。

**3. 前置依赖**
M0-T3、M0-T4、M0-T5、M0-T6 到达**任意终态**（已完成 / HOLD / STOP / BLOCKED / NOT_RUN——AGGREGATE 语义，ADR-011 D2）。上游失败/阻塞时本任务照常产出 HOLD/STOP 报告；`STATUS=GO` 仍要求全部强制条件通过。

**4. 允许修改的模块与文件范围**
- 生成：`artifacts/G0/<final-run-id>/`（全套）
- 修改：`docs/tasks/MASTER_TASK_PLAN.md`（状态表）

**5. 实施步骤**
1. 将 M0-T2~T6 证据汇总入最终 run-id 目录（九文件齐全）。
2. 对照 G0 验收原文逐句自检：camera synthetic works？安装/卸载步骤可重复？SysVAD 风险书面判断完成？
3. 复核 M0-T4 的 R-02 决策：通过 / 不通过。
4. 按 §14.2 填写十字段（GATE、STATUS、IMPLEMENTED、TESTS、OS_EVIDENCE、PERFORMANCE、SECURITY、LICENSING、KNOWN_ISSUES、NEXT_GATE）。
5. 向用户提交判定，等待确认 GO/HOLD/STOP。
6. 更新 MASTER_TASK_PLAN 状态表；GO ⇒ 解锁 M1；HOLD/STOP ⇒ 显式记录阻塞点。

**6. 自动化测试要求**
- 证据完整性检查：九文件存在、schema 合法、无占位符遗留。

**7. 真实环境验收要求**
- `OS_EVIDENCE` 字段必须引用 M0-T2/T3 的真实设备枚举与应用验证证据。**机器无法测试 OS 设备时必须给 HOLD，不得用 Mock/CI 替代 Windows 设备验证**（§14.2 明文）。

**8. 完成标准（DoD）**
- GATE=G0 汇报发布且用户确认；状态表更新；下游解锁或阻塞原因显式记录。

**9. 风险及失败处理**
- 证据缺角仍报 GO → 禁止；任何真实设备证据缺失必须 HOLD。
- STOP 时 → 按基线 §9：先排除工具链/环境再写引擎，复盘后重新走 M0。

**10. 应保留的测试证据**
完整 `artifacts/G0/<final-run-id>/` 九文件。

**11. 状态**：已完成（run G0-20261113-223247-7bc7；GATE=G0 判定建议 GO——**经用户确认 2026-11-13 正式置 G0=GO**，M1 解锁；已知缺陷 KI-G0-1/KI-G0-2 挂 M1-T6 强制闭环项）
