# collect-evidence — run 证据目录与 manifest 自动化（M0-T6）

EVIDENCE_SPEC（`docs/tasks/EVIDENCE_SPEC.md`）v1.0 的生成器与校验器。Python 3 标准库实现，
无第三方依赖（SBOM：无新增条目）。

## 用法

```text
python tools/collect-evidence/collect_evidence.py init     --gate G0 --task M0-T6 [--runs-root artifacts] [--run-id G0-...-4hex]
python tools/collect-evidence/collect_evidence.py manifest --run-dir artifacts/G0/<run-id> [--task ...] [--commit <hex>] [--notes "..."] [--machine-json <file>]
python tools/collect-evidence/collect_evidence.py validate --run-dir artifacts/G0/<run-id>
python tools/collect-evidence/collect_evidence.py verify   --run-dir artifacts/G0/<run-id>
python tools/collect-evidence/collect_evidence.py --selftest
```

退出码：`0` 通过；`1` 校验/完整性失败；`2` 用法错误或自测失败。

## 行为约定

| 子命令 | 行为 |
|---|---|
| `init` | 创建 `artifacts/<gate>/<run-id>/`（run-id 自动生成，`--run-id` 可固定用于复现测试）。**目标已存在 ⇒ REFUSED（exit 1），不破坏已有 run**（EVIDENCE_SPEC §1：run-id 不得复用，不覆盖历史）。 |
| `manifest` | 扫描 run 目录生成 `manifest.json` + `SHA256SUMS.txt`（排除这两个文件自身；文件按 path 排序）。已有 manifest 时**保留元数据仅刷新 files[]** ⇒ 同目录同内容重复执行**字节级幂等**。首次生成必须 `--task`。`--commit` 缺省时自动取 `git rev-parse HEAD`。`--machine-json` 从 JSON 文件读入 machine 段（os_build/cpu/gpu/ram_gb）。 |
| `validate` | schema（manifest.schema.json）+ 格式语义：run-id 格式、gate 一致性、commit hex、sha256 64hex、路径安全（相对/正斜杠/无 `..`/无盘符）、task_ids 非空、files 非空、system_changes 四字段、路径唯一。不读磁盘内容。 |
| `verify` | 先 validate，再核对磁盘：存在（MISSING）、大小+哈希（MODIFIED）、未列文件（EXTRA）、`SHA256SUMS.txt` 与 manifest 逐行一致（SUMS-MISMATCH）。任一问题 ⇒ exit 1。 |

`description`/`redacted`/`media_content`/`system_changes`/`not_run_tasks` 等审计语义字段
由调用方在 run 关闭前手工补齐（工具不臆造内容——"不把预期值当作实测值"）；
`validate` 会校验这些字段的**格式**，`verify` 校验磁盘一致性。

## 自测覆盖（`--selftest`，16 项）

合法（T1）、重复执行字节级幂等（T2/T3）、非法样例（T4 sha256 格式 / T5 path 缺失 /
T6 缺 run_id / T7 路径逃逸）、完整性（T8 篡改 MODIFIED / T9 删除 MISSING / T10 新增 EXTRA /
T11 SUMS-MISMATCH）、修复恢复（T12）、init 拒绝已有目录且不破坏（T13）、参数校验（T14）、
gate 一致性（T15）、端到端（T16）。

## 与既有脚本的关系

- M0-T4 run 曾用临时脚本 `gen-manifest.py` 生成 manifest（schema 与本工具一致）；
  自 M0-T6 起**统一使用本工具**，历史 run 的 manifest 不追溯重写。
- 幂等性测试另见 `tools/verify-environment/check_idempotency.py`（M0-T1，environment.json 用）。
