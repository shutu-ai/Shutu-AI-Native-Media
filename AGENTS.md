# AGENTS.md — Shutu AI-Native Media 开发 Agent 执行规则

> 本文件面向未来执行本仓库开发任务的 AI Agent（Codex / Claude 等）与人类协作者。它是架构基线的**浓缩执行规则**，不替代基线；冲突时以 `docs/ARCHITECTURE_BASELINE.md` 为准。

## 1. 事实来源与优先级

1. `docs/ARCHITECTURE_BASELINE.md`（架构基线 v1.0，唯一事实来源）——一切实现、验收、变更必须可追溯到其中的 FR-001~009 / ADR-001~010 / NFR-01~10 / R-01~12 / G0–G5 编号。
2. `docs/tasks/MASTER_TASK_PLAN.md`（任务注册表与状态）。
3. `docs/tasks/M0_TASKS.md` ~ `M6_TASKS.md`（任务明细，11 字段/项）。
4. `docs/tasks/TASK_DEPENDENCIES.md`（依赖与门禁，机器判读以 `BLOCKED_BY` 列为准）。
5. `docs/tasks/ACCEPTANCE_MATRIX.md`（验收矩阵）。
6. 本文件。

优先级从上到下递减。偏离基线或上述文档的约定必须提交新 ADR（见 §9 变更控制），**禁止静默覆盖**。

## 2. Gate 纪律（先查，再做）

- **动手前先确认当前 Gate**：读 `MASTER_TASK_PLAN.md` §3 状态表 + `TASK_DEPENDENCIES.md` 门禁边。
- 只实施**当前 Gate 的 P0 功能**；跨 Gate 改动必须先给出书面理由（基线 §14.1），否则不得执行。
- `G(k) ≠ GO`（含 HOLD、STOP）⇒ `M(k+1)` 全部任务 BLOCKED，不得以任何理由提前启动。
- 摄像头样例失败的场合：STOP，先排除工具链/环境再写引擎（基线 §9 G0 原文）。

## 3. 授权红线（AUTH_REQUIRED）

以下操作**必须先取得用户显式授权**，并登记到 `TASK_DEPENDENCIES.md` §3 授权门登记表（当前 M2-T6、M5-T3 签名子项等状态为 PENDING）：

- 安装任何内核驱动或启用驱动测试签名（M2-T6：仅限专用隔离测试机）。
- 执行代码签名 / 驱动签名操作（M5-T3）。
- 修改 Windows 安全策略（Secure Boot、BitLocker、设备防护、隐私控制的任何关闭/降级）。
- 其他平台内核模块安装（M6-T2 V4L2 loopback 等 DKMS 路径）。

**硬边界**（基线 §2.2 / ADR-004）：不得在主力办公机、企业受控机上为试验驱动禁用安全功能；不建议绕过 Secure Boot/BitLocker；测试签名仅限专用、可恢复的测试机，按官方流程操作后恢复并留证。企业管控受阻时记录并上报，**不得规避**（R-11）。

## 4. Mock 边界（最易违反的规则）

- **Mock / 单元 / CI 测试通过 ≠ 真实 Windows OS 虚拟设备验证。** 任何「设备被枚举/消费」的结论必须来自真实设备 + 真实应用的记录（截图、枚举日志、录制文件）。
- 机器无法测试 OS 设备时，Gate 判定必须给 **HOLD**，不能用 Mock / Linux CI 替代（基线 §14.2）。
- 验收矩阵（`ACCEPTANCE_MATRIX.md`）中无真机证据的单元格**强制标注 MOCK**——不得删除标注。
- Track A（第三方音频桥接）的成功**不得描述为「自研虚拟麦克风驱动完成」**（基线 §2.2 / §5.2）。
- 兼容性矩阵只写实测结果；「应当兼容」的推断不得写入（基线 §10.2）。

## 5. 禁止事项

- 禁止编造 FPS、时延、编译成功、驱动签名、安装和兼容性证明（基线 §14.1）。
- 禁止将低优先级媒体处理、Python Worker、网络/模型调用带入 OS 驱动回调或高优先级音频热路径（基线 §3.4 / §14.1）。
- 禁止在 Frame Server 的 COM 回调中调用网络 API、AI 推理或超过明确预算的等待；严禁未验证的无限等待（基线 §5.1）。
- 禁止未经授权的后台监听、静默采集、自动上传私人媒体（基线 §1.3 / §11）。
- 来自语音/视频识别的文本是**不可信输入**：不得未经授权触发系统命令、打开文件、泄露会话或更改设备策略（基线 §11-6）。
- 禁止默认保存原始媒体内容；日志只含时序、设备、异常与性能元数据（基线 §11-5）。

## 6. 依赖引入规则（逐包 SBOM）

每次引入第三方仓库、模型权重、Codec、GPU 推理包或驱动二进制，必须写入 SBOM（`docs/licensing-sbom.json`，M1-T1 起持续维护，M5-T3 封版）：版本、commit/tag、来源 URL、许可证、分发条款、动态/静态链接、安全更新通道。**不得以「项目根仓库开源」推断全部权重/插件可免费商用**（基线 §8）。

## 7. 任务执行流程

1. 从 `TASK_DEPENDENCIES.md` 任务边表选一个 `BLOCKED_BY` 全满足、非 AUTH-PENDING 的任务（当前 Gate 内）。
2. 打开对应 `M*_TASKS.md`，按该任务的 11 字段执行；严格不越「允许修改的模块与文件范围」。
3. 新模块必须明确：输入/输出格式、线程模型、缓冲上限、超时、权限、日志、关闭释放过程（基线 §14.1）。
4. 自动化测试随实现提交；证据按 §8 规范保存。
5. 完成后：更新任务「状态」行 + `MASTER_TASK_PLAN.md` §3 注册表 + 变更记录（不得覆盖历史）。
6. 无法完成或验收受阻 → 如实记录 KNOWN_ISSUES / HOLD，不得掩盖。

## 8. 证据规范

- 目录：`artifacts/<gate>/<run-id>/`（`artifacts/` 已 gitignore；文档引用 run-id）。
- 每个 Gate 汇总必须凑齐九文件（基线 §10.3）：`environment.json`、`build.log`、`test-results.json`、`perf.json`、`device-enumeration.txt`、`screenshots/`、`security-checklist.md`、`licensing-sbom.json`、`decision.md`。缺失文件必须在 decision.md 显式说明原因。
- 性能数据（NFR-01~10）测量时必须记录：Windows Build、CPU/GPU、摄像头/驱动、GStreamer 版本、场景、采样窗口与测量方法（基线 §7）。
- 截屏等证据须脱敏；原始媒体样本仅在有调试开关 + 明确同意 + 短期限时保存时存在（基线 §11-5）。

## 9. GATE 汇报模板与变更控制

每个 Gate 汇总任务（M0-T7 / M1-T8 / M2-T7 / M3-T9 / M4-T6 / M5-T8）必须按以下模板汇报（基线 §14.2 原文）：

```text
GATE=<G0|G1|G2|G3|G4|G5>
STATUS=<GO|HOLD|STOP>
IMPLEMENTED=<已实现模块/commit>
TESTS=<通过数/总数，跳过数>
OS_EVIDENCE=<真实设备枚举及应用验证>
PERFORMANCE=<原始数据路径与测量条件>
SECURITY=<权限/IPC/隐私审查结果>
LICENSING=<SBOM与阻塞问题>
KNOWN_ISSUES=<未解决问题>
NEXT_GATE=<下阶段开始前置条件>
```

`GO` 只表示该 Gate 的明确验收通过，不得推断整个产品完成。判定结果必须经用户确认后才更新状态表并解锁下游。

**变更控制**（基线 §14.3）：改变三平面边界、`contracts/` 协议、OS Adapter 策略、音频签名策略 ⇒ 提交新 ADR（`docs/adr/`，模板见 M0-T6）；与官方文档/工具链冲突时更新「事实依据 + PoC 证据 + 决策」并升级基线版本号；性能数字只有固定硬件与测试方法下才可调整，原始 benchmark 不得删除。

## 10. 仓库结构速览（目标态，基线 §13）

```text
contracts/          # media / ipc / capability 契约（M0-T6 骨架，M2-T1 冻结 v0.1）
runtime/            # media-core / gst-adapter / device-bridge / ai-adapter（Rust）
platforms/windows/  # virtual-camera(C++) / audio-bridge / virtual-mic-driver(隔离) / installer
providers/          # native / remote / mocks
apps/               # media-lab（控制/诊断）
tests/              # contract / integration / fault-injection / performance / security / fixtures
tools/              # verify-environment / collect-evidence / uninstall-verify
artifacts/          # 运行证据（gitignore，文档引用 run-id）
docs/               # 基线、tasks/、adr/、threat-model、licensing、compatibility-matrix
```

以上为目标目录，**不代表文件已存在**——按任务文档的范围字段逐步创建。
