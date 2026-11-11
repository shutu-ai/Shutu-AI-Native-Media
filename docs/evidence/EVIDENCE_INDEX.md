# 证据索引（EVIDENCE_INDEX）

> **版本**：v1.0（2026-11-11，ADR-011 D5；规范见 `docs/tasks/EVIDENCE_SPEC.md`）
> **用途**：所有 run 的**元数据级**持久索引（`artifacts/` 不入库，本索引入库）。审计链：任务 ID → run-id → manifest → git commit。
> **红线**：本索引只写元数据——不得包含原始媒体内容、设备序列号、用户身份信息（基线 §11-5）。

## 索引表

| run-id | 日期 | 任务 ID | Gate | 主题 | 证据级别 | git commit | 归档 | 备注 |
|---|---|---|---|---|---|---|---|---|
| G0-20261111-174257-8532 | 2026-11-11 | M0-T1 | G0 | 环境盘点脚本 v1.0 首次真机运行 + 工具链实测（VS/GStreamer/Rust/SDK） | 真机 | 21035c8 | 未归档（开发期保留，G0 判定后归档） | M0-T1 判定已完成；screenshots/perf/device-enumeration 不适用（decision.md §5）；unconfirmed：第二隔离测试机 |

## 使用说明

1. 每个 run 关闭时追加一行；字段缺失用 `—` 并在备注说明。
2. `证据级别` 取值同验收矩阵：`真机` / `CI` / `Mock（标注边界）` / `书面`。
3. run 含 `media_content=true` 文件时，备注必须写明同意记录与删除期限（≤30 天）。
4. 含 NOT_RUN 任务的 run，备注列注明 `NOT_RUN:<任务ID>`。
