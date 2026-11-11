# 证据索引与归档规范（EVIDENCE_SPEC）

> **版本**：v1.0（2026-11-11，依据 ADR-011 D5）
> **上级规范**：`docs/ARCHITECTURE_BASELINE.md` §10.3（九文件）、§11-5（脱敏与最小保留）、§14.1（证据留痕）。
> **适用范围**：所有 `artifacts/<gate>/<run-id>/` 运行证据的命名、清单、索引入库、归档与审计。本规范只管理**元数据与归档流程**，不改变九文件要求。

## 1. run-id 命名规则

```text
run-id := G<k>-<yyyymmdd>-<hhmmss>-<4hex>
示例：  G1-20261112-143005-a3f2
```

- `G<k>` = 该 run 所属 Gate（任务级 run 归入任务所在里程碑对应的 Gate；跨 Gate 汇总用最终 Gate）。
- 时间为**本地时间**，取 run 开始时刻；`4hex` 为随机后缀防碰撞。
- run-id 一经写入索引**不得复用**；重跑生成新 run-id，旧 run 保留（不覆盖历史）。

## 2. run 目录结构与 manifest.json

```text
artifacts/<gate>/<run-id>/
├── manifest.json              # 本规范定义的运行清单（强制）
├── environment.json           # 九文件之一
├── build.log
├── test-results.json
├── perf.json
├── device-enumeration.txt
├── screenshots/
├── security-checklist.md
├── licensing-sbom.json
├── decision.md                # 含 system_changes[]（ADR-011 D6）与 NOT_RUN 记录（ADR-011 D1）
└── SHA256SUMS.txt             # 由 manifest 生成，便于离线校验（强制）
```

`manifest.json` 最小 schema（`tools/collect-evidence` 按 M0-T6 实现生成器）：

```json
{
  "run_id": "G1-20261112-143005-a3f2",
  "task_ids": ["M1-T6"],
  "gate": "G1",
  "created_local": "2026-11-12T14:30:05+08:00",
  "machine": { "os_build": "", "cpu": "", "gpu": "", "ram_gb": null },
  "git_commit": "<40 hex，生成证据时的工作树 commit>",
  "tool_versions": { "gstreamer": "", "rustc": "", "vs_buildtools": "" },
  "files": [
    { "path": "perf.json", "sha256": "<64 hex>", "bytes": 0,
      "description": "NFR-01/03 采样", "redacted": true, "media_content": false }
  ],
  "system_changes": [],
  "not_run_tasks": [],
  "notes": ""
}
```

- `media_content=true` 的文件受 §5 保留期约束；`redacted=false` 须在 notes 说明原因。
- `not_run_tasks[]` 记录本次以 NOT_RUN 处理的可选任务及原因（审计 NOT_RUN 合法性）。
- `system_changes[]` 每项含：`action`（注册设备/重启服务/安装工具/修改 PATH…）、`scope`、`rollback`、`confirmed_by_user`（bool）。

## 3. 索引入库（docs/evidence/EVIDENCE_INDEX.md）

- 每个 run 关闭时向 `docs/evidence/EVIDENCE_INDEX.md` 追加一行（模板见该文件头部表格）。
- 索引**仅含元数据**（run-id、日期、任务、Gate、主题、证据级别、commit、归档名、备注），**不得**包含原始媒体内容、设备序列号、用户身份信息。
- 索引随代码一同提交 git——这是 `artifacts/`（gitignore）之外唯一的持久审计入口。

## 4. 归档与完整性

- 归档：`artifacts/<gate>/_archives/<run-id>.zip` + `<run-id>.zip.sha256`（归档时机：该 Gate 汇总判定完成后）。
- `SHA256SUMS.txt` 覆盖 run 内全部文件；校验命令 `sha256sum -c SHA256SUMS.txt`（或 PowerShell `Get-FileHash` 对照）。
- 归档 zip 与 manifest 中任一 sha256 不符 ⇒ 该 run 证据**视为失效**，相关验收结论必须重新取证，不得引用失效 run 支持 GO 判定。

## 5. 脱敏与保留期（基线 §11-5 延伸）

- 截图须去除个人身份信息（账户名、真实人像、聊天内容）；默认不采集真实摄像头画面/麦克风声音。
- 原始媒体样本仅在：调试开关开启 + 用户明确同意 + 有限时保存时存在；`media_content=true` 文件**最长保留 30 天**，到期删除并在索引备注「已按期删除」。
- 日志只含时序、设备、异常与性能元数据，不含媒体内容（基线 §11-5）。

## 6. 审计链（四级勾稽）

```text
任务 ID（docs/tasks/*）
  → run-id（docs/evidence/EVIDENCE_INDEX.md）
    → manifest.json（artifacts/<gate>/<run-id>/，或归档 zip 内）
      → git commit（manifest.git_commit，可追溯当时的任务实现）
```

验收矩阵、Gate 报告、decision.md 中引用的任何证据都必须能沿此链勾稽到；断链的证据不得用于 GO 判定。
