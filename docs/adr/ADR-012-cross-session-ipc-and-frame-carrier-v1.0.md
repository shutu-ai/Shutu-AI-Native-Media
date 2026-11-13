# ADR-012 · 跨会话 IPC 通道与媒体帧承载策略（R-02 正式化，CONDITIONAL_GO）

> **状态**：Accepted（2026-11-13，依据 M0-T4 真机 run `G0-20261112-191515-3C36` + M0-T6 run `G0-20261113-153144-57a0` 补充只读查询；判定 M0-T4=PASS、R-02=GO）
> **影响范围**：`contracts/ipc/`（冻结输入）、M1-T3/M1-T4/M1-T5 实现约束、`docs/TECH_DEBT.md`（P1-1/2/3）；架构基线 v1.0 **文件零改动**（见 §5）
> **修订性质**：对基线 §5.1 视频通道预设（"预分配共享内存 Ring"）的**记录性修订**——其用户侧前提被 M0-T4 实测否定；本 ADR 不降低任何安全门槛，反而收紧（显式 DACL + 七条安全设计要求）

## 1. 背景

R-02（基线 §12）：Frame Server 中源组件无法访问用户进程 IPC｜高｜M0 先测 Local Service / Session 0 / ACL。M0-T4（run `G0-20261112-191515-3C36`）以最小 IPC 身份认证 PoC（`tests/contract/ipc-identity-poc/`，12 项矩阵）与 Frame Server 真机探测完成验证，结论：R-02=GO，Named Pipe 为控制通道首选；基线 §5.1 预设的共享内存方案其用户侧前提被实测否定。按基线 §14.3 与 M0-T4 decision.md §4.4，该决策须以 ADR 正式化——即本文件。基线 §5.1 描述的"帧经共享内存 Ring 从 Runtime 到 MediaSource"通道策略自本 ADR 起修订为 §3 D2 的**条件式决策**。

## 2. 事实基础

### 2.1 已验证事实（真机证据）

| # | 事实 | 来源 |
|---|---|---|
| V1 | Frame Server 运行身份 = NT AUTHORITY\LocalService（S-1-5-19），进程 SessionId=0，`svchost.exe -k Camera`，DEMAND_START，完全按需启停 | run 3C36 F1-F4（sc qc/CIM/运行采样） |
| V2 | **LS(Session 0) 可 OS 级连接用户进程创建、DACL 显式授 S-1-5-19 的命名管道**；DACL 不授即拒（gle=5） | run 3C36 PoC T5/T6（logs/poc-ls-probe-v3.txt 等） |
| V3 | **跨会话服务端进程级身份校验不可行**：用户侧服务进程 OpenProcess→OpenProcessToken 查 LS 客户端失败 → fail-closed 拒绝按设计生效；同会话下该校验链路正常（T1/T3） | run 3C36 PoC T5/T1/T3 |
| V4 | **非提权用户进程不能创建 Global\ 命名对象**（SeCreateGlobalPrivilege 不在交互令牌，T7b gle=5）；LS 写→用户读反向可行（T7a）但与视频生产方向相反 | run 3C36 PoC T7a/T7b |
| V5 | Local\ 命名空间按会话隔离，跨会话不可见（T9） | run 3C36 PoC T9 |
| V6 | SCM 7036 事件不记录 FrameServer 启停——生命周期监控只能 `sc query` 轮询（事件驱动轮询方法已沉淀 staging/probe-watch*.ps1） | run 3C36 N1（wevtutil 实测） |
| V7 | **SERVICE_SID_TYPE = UNRESTRICTED**：Frame Server 进程令牌携带 per-service SID **NT SERVICE\FrameServer = S-1-5-80-3915894004-2104103821-3047269622-1811662266-774708259**（SID 类型与 SID 值为 M0-T6 只读实测；"管道 DACL 授该 SID 可放行 Frame Server 客户端"属推断，见 U1） | run 57a0 logs/service-sidtype.txt（sc qsidtype/showsid） |

### 2.2 未验证假设及其处置

| # | 假设 | 处置 |
|---|---|---|
| U1 | 以 `NT SERVICE\FrameServer`（V7 的 per-service SID）替代 `S-1-5-19` 作为管道 DACL 主体，Frame Server 客户端仍可连接 | **M1-T4 强制实测项**（P1-1 验收内容之一）；成功则最小授权显著收紧（不再放行其他 LocalService 进程），失败则维持 S-1-5-19 + 更强应用层握手 |
| U2 | Named Pipe 承载 NV12/BGRA 帧流（720p30/1080p30）吞吐、时延、CPU、背压可达标 | P1-3：M1-T4/T5 实测，按 §3 D2 决定是否保留管道承载 |
| U3 | `PIPE_REJECT_REMOTE_CLIENTS` 与 `FILE_FLAG_FIRST_PIPE_INSTANCE` 在本场景的实际行为（MS 文档语义明确，未实测） | M1-T4 安全测试用例覆盖（P1-1 验收内容之一） |
| U4 | 不同 Windows 账户的多用户行为（M0-T4 仅验证同账户双会话） | KNOWN_ISSUES K1 沿用；复评触发=多账户测试环境可用 |
| U5 | LS 侧对用户进程的 OpenProcess 反向校验（服务端之外的第二道进程级校验）在最小特权下可行 | 不依赖：本 ADR 身份模型不要求任何方向的进程级 token 校验（见 D3-S4） |
| U6 | PoC 的异步取消路径（`server.cpp` IoWithTimeout/ConnectNamedPipe 超时循环）在竞争条件下的行为 | **不验证、直接重写**：设计已确认存在取消完成竞争（P1-2），产品代码禁止复制（§4 P1-2） |

## 3. 决策

### D1 · 控制通道 = Named Pipe，用户会话侧为服务端（Accepted）

- Runtime（用户会话）创建命名管道并持 DACL；Frame Server 内 MediaSource 作为客户端连接。
- 依据：V2（连通成立）+ V3（进程级校验不可行 ⇒ 身份边界收敛到 DACL + 应用层握手）。
- 管道名：厂商前缀固定名（如 `shutu-ipc-...`），全局命名空间（`\\.\pipe\`）；多会话实例在名字中带会话标识防冲突（S4）。

### D2 · 媒体帧承载 = CONDITIONAL_GO（当前首选：管道复用；最终由 M1 实测决定）

- **当前首选**：帧数据与控制消息同走 Named Pipe（通道复用，协议带复用标记）。
- **条件**：是否保留管道承载以 **P1-3 实测**为准（NV12/BGRA × 720p30/1080p30 的吞吐、p95/p99 时延、CPU 占用、背压行为）。**实测达标前，管道帧承载不得标记为生产可用**；本 ADR 的 CONDITIONAL_GO 即指此条件。
- **未达标时**：切换 D4 备选，并提交新 ADR（编号顺延）。

### D3 · 安全设计要求（contracts/ipc 冻结与 M1-T4/T5 实现的强制约束）

| # | 要求 | 内容 |
|---|---|---|
| S1 | **LocalService SID 非服务专属身份** | `S-1-5-19` 是共享账户身份：同机任何以 LocalService 运行的进程/计划任务都持该 SID ⇒ DACL 授 S-1-5-19 **≠ 只授 Frame Server**。因此：①应用层握手为强制附加层（版本协商 + 随机 challenge/确认）；②M1-T4 实测 U1——若 `NT SERVICE\FrameServer`（V7）可作 DACL 主体则立即收紧；③管道授权范围最小化，禁止扩大到 Users/Everyone 组补位 |
| S2 | **管道 DACL 最小授权** | SDDL 使用 `D:P`（保护 DACL，拒绝继承）；ACE 仅 `(A;;GRGW;;;<service-identity>)` 一条（service-identity 按 U1 结果取 `NT SERVICE\FrameServer` 或 `S-1-5-19`）；禁止 Everyone/NULL SID/ANONYMOUS/交互用户组授权；服务端创建即设显式 DACL，不依赖默认 DACL |
| S3 | **防远程连接与实例抢占** | 服务端以 `FILE_FLAG_FIRST_PIPE_INSTANCE` 创建——名称已被占用（squatting）即创建失败并告警拒绝，不得降级为连接既有实例；创建后设置 `PIPE_REJECT_REMOTE_CLIENTS`；客户端连接后以 `GetNamedPipeServerProcessId` 校验服务端 PID 并验证其映像路径/签名（防连到冒名服务端） |
| S4 | **双向身份验证与多用户隔离** | 服务端→客户端：DACL（OS 级）+ 同会话下可选 token 校验（V3：跨会话不可行，不做产品依赖）+ 应用层握手 challenge；客户端→服务端：DACL + S3 的服务端进程校验 + 握手响应；多用户隔离：DACL 不授其他交互用户（V2 的 T6 同型保证）；Local\ 命名空间跨会话不可见（V5）⇒ 统一用全局命名空间 + DACL 承担隔离；管道名含会话标识防多用户/多实例同名抢占 |
| S5 | **PoC 异步取消逻辑已知风险（禁止复制）** | `tests/contract/ipc-identity-poc/server.cpp` 的 `IoWithTimeout` 在超时后调用 `CancelIo` 但**不等待取消完成**即返回（OVERLAPPED 栈变量与事件句柄立即失效——已取消 IRP 可能仍在引用，存在 use-after-free 窗口）；`ConnectNamedPipe` 2s 超时循环中 `CancelIo→DisconnectNamedPipe` 与客户端恰好完成连接存在竞争窗口（静默丢弃刚连上的客户端）；`CancelIo`（非 `CancelIoEx`）取消句柄上全部未决 I/O，多实例场景过度取消。**产品实现必须改用 `CancelIoEx` + 等待取消完成（GetOverlappedResult bWait=TRUE）+ 每实例独立事件**，并按 P1-2 增加竞争测试。M0-T4 PoC 中的现有实现**不得未经修正直接复制到产品** |
| S6 | **NV12/BGRA 理论吞吐与 M1 验证条件** | 理论值（十进制单位；公式 `MB/s = w×h×Bpp×fps / 1e6`，透明可复核）：<br>• NV12（Bpp=1.5）：720p30 ≈ 39.6 MB/s（317 Mb/s）；1080p30 ≈ 93.3 MB/s（746 Mb/s）<br>• BGRA（Bpp=4）：720p30 ≈ 105.5 MB/s（844 Mb/s）；1080p30 ≈ 248.8 MB/s（1990 Mb/s）<br>注：M0-T4 decision.md §4.2 的"≈221 Mb/s"为先期粗估（口径未注明），**以本表为准**；原 run 证据按规范不修改。<br>**M1 验证条件（P1-3 验收）**：NV12/BGRA × 720p30/1080p30 实测满帧率不丢帧、p95/p99 时延有界（阈值由 M1-T4/T5 任务定义并与 NFR-03/04 对齐）、CPU 增量记录在案、下游背压行为有界（无无限堆积）。全部达标 ⇒ 关闭 P1-3 并确认 D2；任一不达标 ⇒ 启动 D4 |
| S7 | **Shared Memory 备选方案及其启用条件** | 备选 = LS 特权 broker 服务（安装器注册的独立最小服务）创建 `Global\` section，用户侧经控制通道交换映射信息；方向依据 V4（LS 创建/用户读可行，与视频生产方向一致）。**启用条件（全部满足）**：①P1-3 实测判定管道承载不达标；②提交新 ADR 说明选型与安装影响；③broker 服务的安装/注册走 SYS_CHANGE（运行期）并在 M5 安装器验收中覆盖。禁止在未满足①前"预防性"实现 broker（R-10 同型风险） |

### D4 · 备选方案（Hold，不预先实现）

见 S7。在 D2 条件判定完成前，备选只保留设计描述，不写实现代码。

## 4. 挂接项：P1 技术债登记

三个 P1 问题已在 **`docs/TECH_DEBT.md`** 建立正式登记（P1-1 管道安全验证 / P1-2 异步取消重写 / P1-3 帧承载实测），每项含问题 ID、严重程度、关联代码、责任任务、验收标准与关闭证据要求；**未完成验证前不得标记 CLOSED**。M1-T4（P1-1/P1-2/P1-3）与 M1-T5（P1-1/P1-3）为责任任务；其任务文档已加注引用。

## 5. 对基线的影响与变更控制

- **架构基线 v1.0 文件零改动**（SHA-256 pin 不变，`tools/check-task-consistency.py` 把守）。
- 本 ADR 记录对基线 §5.1 的**表述修订**：视频通道"预分配共享内存 Ring（用户进程创建，供服务读）"自本 ADR 起修订为 D2 的条件式策略——因 V4 实测否定其用户侧前提。基线版本的相应升级留待基线评审流程（须用户确认，基线 §14.3）。
- `contracts/ipc/` 冻结（M1-T4/M2-T1）不得与本 ADR 冲突；偏离即新 ADR。

## 6. 后果

- **正面**：R-02 正式关闭为 CONDITIONAL_GO；跨会话 IPC 的身份模型从"进程级 token 校验"（不可行）收敛为"DACL + 应用层握手 + 服务端进程校验"（全部可验证）；per-service SID（V7）为最小授权提供了收紧路径；三笔技术债有了明确的闭环责任与验收条件。
- **代价**：管道帧承载的可行性悬置到 M1 实测（M1-T4/T5 的验收负担加重）；broker 备选若启用需安装一个服务（M5 安装器复杂度+1）。
- **新增约束**：M1-T4/T5 实现受 §3 D3 七条强制约束；`tests/contract/ipc-identity-poc/` 源码只能作为证据探针引用，不得作为产品代码基线。
