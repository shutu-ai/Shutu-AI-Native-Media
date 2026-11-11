# 证据索引（EVIDENCE_INDEX）

> **版本**：v1.0（2026-11-11，ADR-011 D5；规范见 `docs/tasks/EVIDENCE_SPEC.md`）
> **用途**：所有 run 的**元数据级**持久索引（`artifacts/` 不入库，本索引入库）。审计链：任务 ID → run-id → manifest → git commit。
> **红线**：本索引只写元数据——不得包含原始媒体内容、设备序列号、用户身份信息（基线 §11-5）。

## 索引表

| run-id | 日期 | 任务 ID | Gate | 主题 | 证据级别 | git commit | 归档 | 备注 |
|---|---|---|---|---|---|---|---|---|
| G0-20261111-174257-8532 | 2026-11-11 | M0-T1 | G0 | 环境盘点脚本 v1.0 首次真机运行 + 工具链实测（VS/GStreamer/Rust/SDK） | 真机 | 21035c8 | 未归档（开发期保留，G0 判定后归档） | M0-T1 判定已完成；screenshots/perf/device-enumeration 不适用（decision.md §5）；unconfirmed：第二隔离测试机 |
| G0-20261111-182351-1DB8 | 2026-11-11 | M0-T2 | G0 | 官方 VirtualCamera Sample 原样构建 + 注册/拉流/注销 3 轮循环 + Windows Camera 合成画面像素级留证 + 完整回滚 | 真机 | 22397a0 | 未归档（开发期保留，G0 判定后归档） | M0-T2 判定已完成；perf/security-checklist 不适用（decision.md §6）；SYS_CHANGE 3 项（decision.md §4）；SBOM 0.2 五新增条目 |
| G0-20261111-212232-D720 | 2026-11-11 | M0-T5 | G0 | SysVAD 书面风险评估：WDK/EWDK 版本矩阵 + 签名四路径成本 + 隔离测试机策略（Track B HOLD）+ Track A 三候选许可/分发（不默认随产品捆绑） | 书面 | 377e5d5 | 未归档（开发期保留，G0 判定后归档） | M0-T5 判定已完成；纯书面 run：build/test/perf/device-enumeration/screenshots 不适用（decision.md §10）；system_changes=[]；SBOM 0.3 六条 research-only；测试机 unconfirmed 保持 |

## 使用说明

1. 每个 run 关闭时追加一行；字段缺失用 `—` 并在备注说明。
2. `证据级别` 取值同验收矩阵：`真机` / `CI` / `Mock（标注边界）` / `书面`。
3. run 含 `media_content=true` 文件时，备注必须写明同意记录与删除期限（≤30 天）。
4. 含 NOT_RUN 任务的 run，备注列注明 `NOT_RUN:<任务ID>`。

