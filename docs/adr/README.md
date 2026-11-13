# docs/adr/ — 架构决策记录（ADR）

> **编号与位置约定**：
> - **ADR-001 ~ ADR-010**：内嵌于架构基线 `docs/ARCHITECTURE_BASELINE.md`（唯一事实来源），不在本目录重复成文。
> - **ADR-011 起**：决策文件落本目录，命名 `ADR-<nnnn>-<slug>-v<ver>.md`（如 `ADR-012-cross-session-ipc-and-frame-carrier-v1.0.md`），编号单调递增、不复用。
> - `ADR-011` 为 2026-11-11 验收修订（Gate 语义/交付分级/证据规范），已在本目录。
> - 任务文档中出现的 `ADR-0xx` 引用：001~010 指基线章节，011+ 指本目录文件。

## 生命周期

```text
Proposed（草案，可变） → Accepted（已接受，内容不可变——修订只能出新版本文件或新 ADR）
Superseded（被后续 ADR 取代时，在头部标注 "Superseded by ADR-0xx"，原文件保留不删）
```

- Accepted 的 ADR **文件不可原地改写**；修订 = 新文件 `ADR-<nnnn>-<slug>-v<ver+1>.md`（同号升版）或新编号 ADR。
- ADR 是**决策记录**，不是基线：ADR 不直接修改 `docs/ARCHITECTURE_BASELINE.md`；
  基线文件升级须经用户确认并升版本号（基线 §14.3），SHA-256 pin 由
  `tools/check-task-consistency.py` 把守。

## 何时必须提交新 ADR（基线 §14.3 触发条件清单）

1. 改变三平面边界（基线 §3：Runtime 平面 / OS Adapter 平面 / Provider 平面）。
2. 改变 `contracts/` 已冻结协议（冻结流程见 `contracts/README.md`）。
3. 改变 OS Adapter 策略（虚拟设备实现路径、Frame Server 交互方式、跨会话 IPC 通道）。
4. 改变音频签名策略 / 交付形态决策前提（Track A/B 边界、驱动签名路径）。
5. 性能数字调整：仅限固定硬件 + 固定测试方法下；原始 benchmark 不得删除。

## 模板

新 ADR 从 [`TEMPLATE.md`](TEMPLATE.md) 复制起步。最小结构：头部（状态/影响范围/修订性质）、
背景、已验证事实与未验证假设（如涉及实测决策）、决策（分条编号 D1/D2/…）、
后果、对基线的影响与变更控制声明。

## 现有 ADR 索引

| 编号 | 标题 | 状态 | 文件 |
|---|---|---|---|
| ADR-001~010 | （基线内嵌，见基线目录） | Accepted | `docs/ARCHITECTURE_BASELINE.md` |
| ADR-011 | Gate 语义修订、交付分级与证据归档规范（任务体系 v1.1） | Accepted（2026-11-11） | [ADR-011-gate-semantics-and-evidence-v1.1.md](ADR-011-gate-semantics-and-evidence-v1.1.md) |
| ADR-012 | 跨会话 IPC 通道与媒体帧承载策略（R-02 正式化，CONDITIONAL_GO） | Accepted（2026-11-13） | [ADR-012-cross-session-ipc-and-frame-carrier-v1.0.md](ADR-012-cross-session-ipc-and-frame-carrier-v1.0.md) |
