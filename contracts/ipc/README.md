# contracts/ipc/ — 进程间通信契约（占位）

> **状态**：占位（M0-T6 建立目录，2026-11-13）。无协议字段。
> **技术决策来源**：R-02 跨会话 IPC 决策已由 **ADR-012** 正式化
> （`docs/adr/ADR-012-cross-session-ipc-and-frame-carrier-v1.0.md`）——通道选型、
> 安全设计要求、帧承载条件（CONDITIONAL_GO）以该 ADR 为准，本目录冻结时不得偏离。

## 规划内容（M1-T4 交付，此处仅登记范围）

- `control-v0.1.md`：Named Pipe 控制通道消息表——HELLO/HELLO_ACK、HEARTBEAT、
  FORMAT_ANNOUNCE、HEALTH、SHUTDOWN；每消息带版本与长度头（M1-T4 实施步骤 1）。
- 帧承载通道：按 ADR-012 D2 条件式决策——Named Pipe 复用为当前首选，
  是否承载媒体帧以 M1-T4/T5 实测（P1-3）为准；未实测达标前**不得标记为生产可用**。
- 安全设计要求（ADR-012 D3 七条）为契约冻结的强制输入：
  DACL 最小授权、防远程连接与实例抢占、双向身份验证、多用户隔离、
  异步取消正确性（P1-2）、吞吐验证条件（P1-3）、备选启用条件（D4）。

## 约束

- 消息字段定义属 M1-T4；本占位不预定义（R-10 同型风险）。
- M1-T4 落地 `control-v0.1.md` 时同步建立 `tests/contract/` 与 `tests/security/` 用例
  （版本不符拒绝、坏长度拒绝、未授权 SID 拒绝、路径操纵无效）。
- M0-T4 PoC 源码（`tests/contract/ipc-identity-poc/`）为**证据探针**，
  其实现（尤其 server.cpp 的异步取消逻辑）**不得未经修正复制进产品**（P1-2，见 `docs/TECH_DEBT.md`）。
