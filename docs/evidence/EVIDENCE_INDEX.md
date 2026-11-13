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
| G0-20261112-191515-3C36 | 2026-11-12~13 | M0-T4 | G0 | Frame Server 真机事实探明（F1-F11：LocalService/Session 0/按需启停/7036 缺失/模块加载/sdshow）+ 最小 IPC 身份 PoC 12 项矩阵（跨会话 T5/T6/T7a/T7b/T9 全实测）+ SC#3 服务重启恢复 + 多用户同账户双会话拉流 + R-02=GO 决策（Named Pipe 首选/共享内存备选/管道 DACL 安全边界五条） | 真机 | 707514d | 未归档（开发期保留，G0 判定后归档） | M0-T4 判定已完成（PASS）；perf.json N/A（decision.md §8）；SYS_CHANGE 3 项均经确认并回滚（decision.md §6）；K1-K7 未验证项（decision.md §7）；PoC 源码 tests/contract/ipc-identity-poc/ 随状态更新 commit 入库 |
| G0-20261113-153144-57a0 | 2026-11-13 | M0-T6 | G0 | contracts/ 三子目录骨架 + tools/collect-evidence v1.0（init/manifest/validate/verify/selftest 16+9 场景全过，幂等，dogfood 生成本 run manifest）+ docs/adr/ 机制补齐（README/TEMPLATE/编号检查⇒ADR-012）+ ADR-012（R-02 正式化 CONDITIONAL_GO，七条安全设计要求）+ TECH_DEBT 登记（P1-1/2/3 OPEN） | 真机 | 5353c17 | 未归档（开发期保留，G0 判定后归档） | M0-T6 判定已完成（PASS）；perf/device-enumeration/screenshots 不适用（decision.md §6）；system_changes=[]（零系统变更）；SBOM 无新增（stdlib-only）；补充只读事实 V7=FrameServer per-service SID（decision.md §4） |
| G0-20261113-201619-b4dc | 2026-11-13 | M0-T3 | G0 | 双独立应用消费与生命周期验证：Chrome(getUserMedia 测试页) 独立/并发(不同设备变体)/Stop/Start/使用中移除 ENDED/重注册手动恢复全路径 PASS；Windows 相机 FAIL(稳定消费能力，切换间歇 CameraSwitchFailed 且不可自愈，首次渲染成功有证)；生命周期 A1→B2→A3→B4 四轮循环无残留 + SC-5 完整回滚终态干净 | 真机 | eb7286c | 未归档（开发期保留，G0 判定后归档） | M0-T3 判定已完成（PASS 含已知缺陷）；perf.json N/A（decision.md §6 K4）；SYS_CHANGE 10 次 UAC 逐项批准（SC-1~SC-6 台账，decision.md §5）；manifest 86 文件 INTEGRITY-OK + 人工审计：**4 文件 media_content=true（真实室内场景，30 天保留期至 2026-12-13，用户测试知情同意）**；偏差 D1-D13；测试页与控制服务入库 tests/fixtures/getusermedia/ |
| G0-20261113-223247-7bc7 | 2026-11-13 | M0-T7 | G0 | G0 Gate 汇总（AGGREGATE）：六任务审计链勾稽 + collect-evidence verify 全覆盖（5 run 修复后 INTEGRITY-OK + b4dc 前缀哈希纯追加证明与遗留轮询器用户授权处置）+ 逐截图人工复核（D13 勘误、4 media_content=true 核验）+ 门审时点终态无残留四断言 + G0 九项强制条件判定 + 摄像头缺陷四能力裁定（KI-G0-1/KI-G0-2 独立登记，G1 责任 M1-T6）+ 状态一致性修复（38/38） | 真机+聚合 | 6ee14bd | 未归档（用户确认 Gate 后归档；media 保留约束随 b4dc 行） | M0-T7 判定=PASS；**G0=GO 已于 2026-11-13 经用户确认生效，M1 解锁**（AGENTS §9）；perf.json N/A；environment.json 为 8532 run 字节级拷贝（SHA-256 相等证明 c1c12be5…）；screenshots/ 仅指针（隐私：不复制媒体、归档不延长保留期）；完整性修复台账与 `_m0t7-backup/` 备份见 run decision.md §4；b4dc 遗留轮询器经用户终止后全部七 run INTEGRITY-OK（7/7） |

## 使用说明

1. 每个 run 关闭时追加一行；字段缺失用 `—` 并在备注说明。
2. `证据级别` 取值同验收矩阵：`真机` / `CI` / `Mock（标注边界）` / `书面`。
3. run 含 `media_content=true` 文件时，备注必须写明同意记录与删除期限（≤30 天）。
4. 含 NOT_RUN 任务的 run，备注列注明 `NOT_RUN:<任务ID>`。

