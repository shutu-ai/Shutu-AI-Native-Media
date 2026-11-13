# contracts/ — 统一契约目录（骨架，v0 未冻结）

> **状态**：占位骨架（M0-T6 建立，2026-11-13）。**本目录当前不含任何具体协议字段定义**——
> 协议内容属后续任务交付物：M1-T3（ring 帧格式头）、M1-T4（control-v0.1 消息表）、
> M2-T1（media v0.1 冻结）、M4-T1（capability v0.2）。
> 本骨架只定义目录约定、冻结流程与版本兼容承诺（M0-T6 DoD 边界：不做协议设计，R-10 同型风险）。

## 目录约定

| 子目录 | 内容（规划） | 首个具体化任务 | 冻结点 |
|---|---|---|---|
| `media/` | Session / Track / Clock / Event / Policy、帧格式头 | M1-T3（帧格式头为首个输入） | v0.1 @ M2-T1 |
| `ipc/` | 控制通道消息表（control-v0.1）、帧承载通道策略 | M1-T4 | v0.1 @ M1-T4，M2-T1 汇审 |
| `capability/` | Capability / Policy / Event 协议（音视频联合路径） | M4-T1 | v0.2 @ M4-T1 |

## 契约冻结流程（M2-T1 起生效）

1. 任务在对应子目录以 `<name>-v<minor>` 命名落文档（如 `control-v0.1.md`），附契约测试计划。
2. 冻结 = 该版本文件加入 `tests/contract/` 契约测试覆盖并在任务 run 的 decision.md 声明；
   冻结后同目录新增文件不影响已冻结版本。
3. 冻结版本的**破坏性变更**：升版本号 + 提交新 ADR（`docs/adr/`，触发条件见其 README），
   禁止原地改写已冻结文件。

## 版本兼容承诺（冻结后生效）

1. 冻结版本在所属里程碑内不可变；兼容性修复升 minor，语义破坏升 major，均须 ADR。
2. `tests/contract/` 钉死已发布版本行为：旧版本句柄、重放 seq、坏长度/坏版本必须被拒绝。
3. 运行时代码（runtime/、platforms/）只允许引用 contracts/ 内的常量与结构定义，
   禁止私自内联协议字段——发现即视为偏离基线 §14.3，走 ADR。
4. 跨平面（media ↔ ipc ↔ capability）引用经各 README 声明的显式接口点，不横向 import。
