# 技术债登记（TECH_DEBT）

> **建立**：2026-11-13（M0-T6，依据 ADR-012 §4 与 M0-T4 run `G0-20261112-191515-3C36` 审查发现）。
> **规则**：每项含问题 ID、严重程度、来源、关联代码、责任任务、验收标准、关闭证据。
> **状态机**：`OPEN`（未处理）→ `FIXED`（实现完成，待验证）→ `CLOSED`（**关闭证据 run-id 在链**才可标记）。
> **红线**：未完成真实环境验证前不得标记 CLOSED（Mock 通过 ≠ 验证完成，AGENTS §4）。

## 登记表

### P1-1 · 管道安全机制未按最小授权与攻击场景完成实现验证

| 字段 | 内容 |
|---|---|
| 问题 ID | P1-1 |
| 严重程度 | **P1（高）**——安全边界机制在产品实现中未经真机验证 |
| 来源 | M0-T4 run `G0-20261112-191515-3C36`（PoC T5/T6 证明 DACL 可行，但产品级最小授权与攻击场景未实现）；ADR-012 §3 S1-S4 |
| 关联代码 | `tests/contract/ipc-identity-poc/`（server.cpp 显式 DACL 创建路径——仅 PoC 证据，非产品）；产品实现落点 `runtime/device-bridge/`（M1-T4）与 `platforms/windows/virtual-camera/`（M1-T5） |
| 责任任务 | **M1-T4（实现+测试）**、**M1-T5（MediaSource 客户端侧对接验证）** |
| 验收标准 | ①DACL 按 ADR-012 S2 最小授权（`D:P` + 单条 `GRGW` ACE，无 Everyone/NULL SID/继承）；②U1 实测：`NT SERVICE\FrameServer`（per-service SID）与 `S-1-5-19` 两种 DACL 主体下 Frame Server 客户端连接结果，取更紧者；③`FILE_FLAG_FIRST_PIPE_INSTANCE` 抢占防护与 `PIPE_REJECT_REMOTE_CLIENTS` 生效实测（U3）；④攻击场景测试通过：未授权 SID 拒绝、非 Frame Server 的 LocalService 进程连接被拒（S1 风险实测）、管道名被抢占时服务端拒绝启动、客户端拒绝冒名服务端（`GetNamedPipeServerProcessId` 校验）、多用户会话下其他用户不可连接；⑤应用层握手（版本+challenge）拒绝非握手连接 |
| 关闭证据 | run（G1）：`test-results.json`（含④⑤全部用例的通过/拒绝记录与错误码）、`security-checklist.md`（S1-S4 逐条对照）、跨会话连接日志（LS 侧或同等身份进程） |
| 状态 | **OPEN** |

### P1-2 · PoC 异步 I/O 取消逻辑存在取消完成竞争，禁止复制进产品

| 字段 | 内容 |
|---|---|
| 问题 ID | P1-2 |
| 严重程度 | **P1（高）**——已知缺陷设计（use-after-free 窗口 + 连接竞争静默丢失）；当前无产品影响（PoC 未入产品），但若被复制即为缺陷 |
| 来源 | M0-T6 对 `tests/contract/ipc-identity-poc/server.cpp` 的代码审查（ADR-012 §3 S5） |
| 关联代码 | `tests/contract/ipc-identity-poc/server.cpp`：①`IoWithTimeout`——超时后 `CancelIo` 不等待取消完成即返回，OVERLAPPED（栈）与事件句柄立即失效，已取消 IRP 可能仍在引用（use-after-free 窗口）；②`ConnectNamedPipe` 2s 超时循环——`CancelIo→DisconnectNamedPipe` 与客户端恰好完成连接存在竞争窗口（刚连接的客户端被静默丢弃）；③使用 `CancelIo`（句柄级全部取消）而非 `CancelIoEx`（单 I/O 精确取消），多实例场景过度取消。产品实现落点：`runtime/device-bridge/` 控制通道（M1-T4） |
| 责任任务 | **M1-T4** |
| 验收标准 | ①产品实现改用 `CancelIoEx` + 取消完成等待（`GetOverlappedResult` bWait=TRUE 或事件信号确认）+ 每实例独立 OVERLAPPED/事件（无栈生命周期逃逸）；②自动化竞争测试：超时瞬间客户端连接（取消-连接竞争，客户端结果可判定、服务端状态一致）、取消完成后句柄可安全复用/关闭、重复取消幂等；③句柄泄漏断言：N 次超时-取消循环前后进程句柄数稳定（≤1 波动，同 M0-T4 T4 方法）；④静态保证：产品代码不包含 `CancelIo(` 裸调用（代码审查项） |
| 关闭证据 | run（G1）：`test-results.json`（竞争/取消完成/句柄泄漏用例结果）、`build.log`（含审查结论：无裸 `CancelIo`）、代码审查记录（server 生命周期说明） |
| 状态 | **OPEN** |

### P1-3 · 管道承载媒体帧未实测（NV12/BGRA 吞吐、时延、CPU、背压）

| 字段 | 内容 |
|---|---|
| 问题 ID | P1-3 |
| 严重程度 | **P1（高）**——ADR-012 D2 的条件未判定；未实测前管道帧承载**不得标记生产可用** |
| 来源 | M0-T4 R-02 决策（帧承载首选管道复用，明确留 M1 实测）；ADR-012 §3 S6/D2 |
| 关联代码 | 理论基线见 ADR-012 S6 表（NV12 720p30 ≈ 39.6 MB/s 等，公式透明）；测量落点：`runtime/gst-adapter/` → 管道 → MediaSource（M1-T4/M1-T5 链路） |
| 责任任务 | **M1-T4（管道传输侧测量）**、**M1-T5（MediaSource 消费侧测量）** |
| 验收标准 | ①矩阵覆盖：{NV12, BGRA} × {720p30, 1080p30} 满帧率实测（丢帧率 0）；②p95/p99 端到端时延记录（阈值由 M1-T4/T5 任务定义并与 NFR-03/04 对齐）；③CPU 占用增量记录（相对无帧承载基线）；④背压行为：下游消费慢时上游有界（队列上限/丢弃策略生效，无无限堆积）；⑤测量记录 Windows Build、CPU、场景、采样窗口与方法（基线 §7 要求），perf.json 落档 |
| 关闭证据 | run（G1）：`perf.json`（全矩阵数据）、`test-results.json`（背压/丢帧用例）、`decision.md`（判定：达标 ⇒ 确认 ADR-012 D2 管道承载并关闭本债；不达标 ⇒ 启动 ADR-012 D4 broker 备选并提交新 ADR，本债转为 SUPERSEDED 记录） |
| 状态 | **OPEN** |

## 索引

| ID | 严重程度 | 责任任务 | 状态 | 建立 |
|---|---|---|---|---|
| P1-1 | P1 | M1-T4 / M1-T5 | OPEN | 2026-11-13 |
| P1-2 | P1 | M1-T4 | OPEN | 2026-11-13 |
| P1-3 | P1 | M1-T4 / M1-T5 | OPEN | 2026-11-13 |
