# Shutu AI-Native Media — 开发任务总控计划

> **版本**：v1.1（2026-11-11 验收修订，依据 ADR-011；初版 v1.0 于 2026-11-10 规划）
> **事实来源**：`docs/ARCHITECTURE_BASELINE.md`（架构基线 v1.0，零改动）> 本文件 > 各里程碑任务文档 > AGENTS.md。任何冲突以基线为准。
> **文档族**：本文件（总控）｜`M0_TASKS.md`~`M6_TASKS.md`（任务明细）｜`TASK_DEPENDENCIES.md`（依赖与门禁）｜`ACCEPTANCE_MATRIX.md`（验收矩阵）｜`AGENTS.md`（Agent 执行规则）｜`docs/adr/`（变更决策）｜`docs/tasks/EVIDENCE_SPEC.md`（证据归档规范）。

## 1. 目标与范围声明

本计划将基线 §9 的 M0–M6 里程碑拆解为 **51 项原子任务**（v1.1 起新增 M5-T9，ADR-011），每项含 11 个必填字段（ID 与目标 / FR·ADR 追溯 / 前置依赖 / 文件范围 / 实施步骤 / 自动化测试 / 真实环境验收 / DoD / 风险与失败处理 / 测试证据 / 状态）。

**本次规划（2026-11-10）只建立了任务体系，未实现任何产品功能**。2026-11-11 验收修订（ADR-011，`PLANNING_GATE=GO`）后按门禁纪律开始执行；实时状态以 §3 注册表为准，所有技术结论在对应任务的真实环境验收完成前一律视为「待验证」。

## 2. 里程碑地图与门禁

```mermaid
graph LR
  M0[M0 环境与可行性验证<br/>7 任务 → G0] --> M1[M1 Rust/GStreamer 媒体基础<br/>8 任务 → G1]
  M1 --> M2[M2 统一音视频会话与音频验证<br/>7 任务 → G2]
  M2 --> M3[M3 AI 能力扩展<br/>9 任务 → G3]
  M3 --> M4[M4 AI-Native 统一能力<br/>6 任务 → G4]
  M4 --> M5[M5 Windows 产品化 Beta<br/>9 任务 → G5]
  M5 -.->|条件触发 + 用户批准| M6[M6 Linux/macOS + IM<br/>5 任务]
```

门禁语义、阻塞规则、任务级依赖表见 `TASK_DEPENDENCIES.md`（以其中 `BLOCKED_BY` 列为机器判读依据）。

## 3. 全量任务注册表（51 项）

> 状态更新规则见 §6。`AUTH` = 需授权门（当前 PENDING：M2-T6、M5-T9、M6-T2 内核分支）。可选任务未授权/未触发时记 `NOT_RUN`（ADR-011 D1），Gate 汇总任务为 AGGREGATE（报告必达，ADR-011 D2）。

| ID | 任务名称 | 前置依赖 | AUTH | 状态 |
|---|---|---|---|---|
| M0-T1 | 环境盘点脚本与工具链补齐 | — | — | 已完成（run G0-20261111-174257-8532） |
| M0-T2 | 官方 VirtualCamera Sample 构建/注册/卸载 | M0-T1 | — | 已完成（run G0-20261111-182351-1DB8） |
| M0-T3 | 双独立应用消费与生命周期验证 | M0-T2 | — | 未开始 |
| M0-T4 | Frame Server 约束验证 + IPC 身份 PoC + R-02 决策 | M0-T2 | — | 未开始 |
| M0-T5 | SysVAD 风险书面评估 | — | — | 未开始 |
| M0-T6 | contracts 骨架 + 证据基线设施 + adr 约定 | M0-T1 | — | 未开始 |
| M0-T7 | **G0 Gate 汇总** | M0-T3/T4/T5/T6 | — | 未开始 |
| M1-T1 | Rust workspace + threat-model 初版 + SBOM 机制 | G0=GO | — | 未开始 |
| M1-T2 | testsrc→appsink 基础视频图 | M1-T1 | — | 未开始 |
| M1-T3 | 共享内存视频 Ring + 契约测试 | M1-T1 | — | 未开始 |
| M1-T4 | Named Pipe 控制通道 | M1-T1 | — | 未开始 |
| M1-T5 | MediaSource 改 IPC ring 读帧 | M0-T4, M1-T3, M1-T4 | — | 未开始 |
| M1-T6 | 端到端验证 + 多消费者/用户切换/重复装卸 | M1-T2/T3/T4/T5 | — | 未开始 |
| M1-T7 | 物理摄像头输入接入 | M1-T6 | — | 未开始 |
| M1-T8 | **G1 Gate 汇总**（NFR-01/03/07 初步） | M1-T6, M1-T7 | — | 未开始 |
| M2-T1 | Session/Track/Clock/Event/Policy 契约 v0.1 冻结 | G1=GO | — | 未开始 |
| M2-T2 | 物理麦克风捕获 + PCM + 同步测试源 | M2-T1 | — | 未开始 |
| M2-T3 | Track A 音频桥接 + NFR-02 | M2-T2 | — | 未开始 |
| M2-T4 | 音视频同 Session + NFR-04/05 | M2-T3 | — | 未开始 |
| M2-T5 | 鲁棒性矩阵 + 权限撤销（NFR-08 初版） | M2-T4 | — | 未开始 |
| M2-T6 | SysVAD Track B PoC（**可选**） | M0-T5 + 授权 | **AUTH·PENDING** | 未开始（未授权时记 NOT_RUN，不阻塞 G2） |
| M2-T7 | **G2 Gate 汇总** + Track B 显式结论 | M2-T4/T5；M2-T6 终态（AGG） | — | 未开始 |
| M3-T1 | 本地视频效果节点 | G2=GO | — | 未开始 |
| M3-T2 | 本地音频效果节点 | M3-T1 | — | 未开始 |
| M3-T3 | ai-adapter（Manifest/授权/取消/超时/健康/降级） | M3-T1 | — | 未开始 |
| M3-T4 | 本地 VAD/ASR 字幕语义事件 | M3-T3 | — | 未开始 |
| M3-T5 | AI 音频替换 Output Track | M3-T2, M3-T3 | — | 未开始 |
| M3-T6 | 双 Provider A/B harness（NFR-10） | M3-T4 | — | 未开始 |
| M3-T7 | 故障注入 + 撤销 + 安全测试（NFR-06/07/08） | M3-T5, M3-T6 | — | 未开始 |
| M3-T8 | 多 AI 节点串联（FR-004） | M3-T1/T2/T3 | — | 未开始 |
| M3-T9 | **G3 Gate 汇总** | M3-T7, M3-T8 | — | 未开始 |
| M4-T1 | Capability/Policy/Event 契约 v0.2 | G3=GO | — | 未开始 |
| M4-T2 | 稀疏视频采样 + 语义事件 + 防注入 | M4-T1 | — | 未开始 |
| M4-T3 | Generated Video Track + 来源元数据 | M4-T1 | — | 未开始 |
| M4-T4 | A+V 关联时间轴 + 过期帧丢弃 | M4-T2, M4-T3 | — | 未开始 |
| M4-T5 | 「单模型替代多节点」mock 集成测试 | M4-T4 | — | 未开始 |
| M4-T6 | **G4 Gate 汇总** | M4-T5 | — | 未开始 |
| M5-T1 | Control GUI | G4=GO | — | 未开始 |
| M5-T2 | 安装/升级/卸载/回滚 + uninstall-verify | M5-T1 | — | 未开始 |
| M5-T3 | 版本固化/SHA-256/SBOM 封版/签名计划（只成文） | M5-T2 | — | 未开始 |
| M5-T4 | 应用兼容性矩阵 | M5-T2 | — | 未开始 |
| M5-T5 | Release Evidence（NFR-06~09 全量）+ 清洁镜像 | M5-T2 | — | 未开始 |
| M5-T6 | 音频交付形态决策 | M5-T3；M5-T9 终态（AGG） | — | 未开始 |
| M5-T7 | threat-model 完整化 + 安全审查 | M5-T5 | — | 未开始 |
| M5-T8 | **G5 Gate 汇总**（判定声明 L1/L2） | M5-T4/T5/T6/T7；M5-T9 终态（AGG） | — | 未开始 |
| M5-T9 | 执行代码签名/驱动签名（**条件触发**） | M5-T3 + 授权 | **AUTH·PENDING** | 未开始（未触发/未授权时记 NOT_RUN，不阻塞 G5） |
| M6-T1 | OS Adapter 契约评审 v1 | G5=GO + 用户批准 | — | 未开始 |
| M6-T2 | Linux PipeWire spike | M6-T1 + 用户批准 | — | 未开始 |
| M6-T3 | macOS CoreMediaIO/Core Audio spike | M6-T1 + 用户批准 | — | 未开始 |
| M6-T4 | LiveKit/WebRTC 消费者 spike | M6-T1 + 用户批准 | — | 未开始 |
| M6-T5 | AI-Native IM 集成评估报告 | M6-T2/T3/T4 | — | 未开始 |

**状态统计**：未开始 49 ｜ 进行中 0 ｜ 已完成 2（M0-T1、M0-T2）｜ BLOCKED（门禁）M1–M6 全部 ｜ AUTH·PENDING（不阻塞主线 Gate）M2-T6、M5-T9——未授权时记 NOT_RUN（ADR-011）。

## 4. 当前环境登记（2026-11-10 规划期探测；2026-11-11 工具链补齐后更新）

> 正式来源以 M0-T1 生成的 `environment.json` 为准；下表为规划期快照，供 M0-T1 对照。
> **探测误报修正（2026-11-11）**：早前「VS C++ 工具链未检测到」系 vswhere 探测参数错误导致的**误报**——本机实际存在 VS 2022 与 VS 2026 两套完整 BuildTools（cl.exe 编译验证可用）。该修正作为环境探测方法论教训记入 M0-T1 参考输入。

**已具备**：

| 项 | 值 | 满足的门槛 |
|---|---|---|
| OS | Windows 11 家庭中文版 25H2，Build 26200.9711 | ≥22000（MFCreateVirtualCamera 门槛，§2.1）✅ |
| git | 2.56.0.windows.4 | 版本控制 ✅ |
| Rust | 1.95.0（msvc host，2026-11-11 链接验证通过） | ADR-001 工具链 ✅ |
| Windows SDK | 10.0.26100.0 / 10.0.22621.0 | VirtualCamera Sample 构建 ✅ |
| VS C++ 工具链 | VS 2022 BuildTools 17.14（MSVC v143 14.44）+ VS 2026 BuildTools 18.9（v145）+ CMake 4.2.3（2026-11-11 确认，探测误报修正） | M0-T2 C++ 编译、Rust msvc 链接 ✅ |
| GStreamer | 1.28.3 MSVC x86_64（`C:\gstreamer\1.0\msvc_x86_64`，2026-11-11 经用户确认安装，SHA256 校验通过） | M1-T2 起管线任务 ✅ |
| Python | 3.14.7 | 可选外部 Worker（§6.2）预留 ✅ |
| 硬件 | Intel Ultra 7 255H / Arc 140T 16GB / 31.4GB RAM / 298GB 可用 | 满足开发与 CPU 基线 ✅ |

**缺口（M0-T1 处理）**：

| 缺口 | 影响 | 处置 |
|---|---|---|
| WDK 未安装 | 仅影响 Track B（M2-T6） | 符合预期：不装；驱动工作仅在授权隔离测试机（本机为家庭版，不承担驱动测试签名） |
| 无第二台隔离驱动测试机（待确认） | M2-T6 只能 HOLD；Track B 书面推进 | M0-T1 探测确认；M0-T5 输出测试机策略 |
| 待现场确认项 | 物理摄像头/麦克风在位性与隐私权限、GPU 驱动版本（R-07）、安全软件/VPN/管控（R-11）、多用户账户可建性（§5.1-7） | 全部记入 `environment.json`（M0-T1） |

## 5. 执行规则（浓缩；完整版见 AGENTS.md）

1. **只做当前 Gate 的任务**：G(k) 未 GO 之前，M(k+1) 全部任务 BLOCKED；跨 Gate 改动须书面理由（§14.1）。
2. **选择任务**：按 `TASK_DEPENDENCIES.md` 任务边表，选 `BLOCKED_BY` 全部满足且非 AUTH-PENDING 的任务。
3. **执行与证据**：按任务文档 11 字段执行；证据写入 `artifacts/<gate>/<run-id>/`（九文件，§10.3）；**Mock/CI 通过 ≠ 真机验证**。
4. **授权红线**：驱动安装、测试签名、安全策略变化一律走 `AUTH_REQUIRED` 登记表；PENDING 不得启动（§14.1、ADR-004）。
5. **汇报（AGGREGATE，报告必达）**：每个 Gate 汇总任务在上游到达任意终态（已完成/HOLD/STOP/BLOCKED/NOT_RUN）时**必须产出** §14.2 十字段报告；`STATUS=GO ⇔ 全部强制条件通过`，判定经用户确认；G5 = GO 必须声明交付等级 `GO(LEVEL=L1|L2)`（ADR-011 D2/D4）。
6. **SYS_CHANGE**：虚拟设备注册/注销、系统服务安装/重启、开发工具安装、PATH/环境变量修改，执行前须向用户单次确认（操作内容、影响范围、回滚方法），并写入该 run `decision.md` 的 `system_changes[]` 与 `environment.json` 增量（ADR-011 D6）；驱动安装/测试签名/安全策略变化仍走 AUTH_REQUIRED，不降级。
7. **变更控制**：改三平面边界、contracts 协议、OS Adapter 策略、音频签名策略 ⇒ 新 ADR（§14.3）；性能数字只在固定硬件与方法下调整，原始 benchmark 不删除。
8. **禁止编造**：FPS、时延、编译成功、驱动签名、安装与兼容性证明一律以留存证据为准（§14.1）。

## 6. 状态更新规则

- 任务状态取值：`未开始 → 进行中 → 已完成 / BLOCKED / HOLD / NOT_RUN`。可选任务未授权/未触发时记 `NOT_RUN`（ADR-011 D1，须在对应 run 的 decision.md 记录原因与复评触发条件）；Gate 汇总任务可在上游非完成终态下产出 HOLD/STOP 报告（ADR-011 D2）；只有**汇总 Gate 任务与用户确认**能把整个里程碑置为通过。
- 置「已完成」须同时满足：① 11 字段中 DoD 全部达成；② 证据文件已存在并标注 run-id；③ 无越 Gate 改动；④ 自动化测试全绿。
- 真实环境验收未完成时，自动化部分最多记「进行中（CI 已过，真机待验）」，**不得记已完成**。
- 状态修改必须在任务文档「状态」行与本文 §3 注册表同步更新，并在文末变更记录追加一行（不得覆盖历史）。

## 7. 文档相互关系

```
ARCHITECTURE_BASELINE.md（唯一事实来源）
   │  追溯
   ├── docs/tasks/MASTER_TASK_PLAN.md（本文件：注册表 + 状态 + 规则）
   ├── docs/tasks/M0~M6_TASKS.md（任务明细，11 字段/项）
   ├── docs/tasks/TASK_DEPENDENCIES.md（门禁 + BLOCKED_BY + AUTH 登记表）
   ├── docs/tasks/ACCEPTANCE_MATRIX.md（FR/NFR/R/§15 × 任务 × 证据）
   ├── docs/tasks/EVIDENCE_SPEC.md（证据索引与归档规范，ADR-011 D5）
   └── AGENTS.md（Agent 执行规则浓缩）
```

## 8. 变更记录

| 日期 | 版本 | 变更 | 依据 |
|---|---|---|---|
| 2026-11-10 | v1.0 | 初版：50 项任务（M0:7/M1:8/M2:7/M3:9/M4:6/M5:8/M6:5）、G0–G5 门禁体系、验收矩阵、AGENTS.md | 架构基线 v1.0 + 用户任务规划指令 |
| 2026-11-11 | v1.1 | 验收修复（ADR-011）：M2-T6 改可选+NOT_RUN 不阻塞 G2；六个 Gate 任务 AGGREGATE 化（报告必达）；M5-T3/M5-T9 拆分（任务总数 50→51）；G5 双交付等级 GO(LEVEL=L1/L2)；新增 EVIDENCE_SPEC 证据索引归档规范；新增 SYS_CHANGE 系统状态变更确认机制；环境表补齐 VS 工具链/GStreamer 并记录探测误报修正 | ADR-011 + 用户验收指令（架构基线 v1.0 零改动） |
| 2026-11-11 | 状态变更 | M0-T1 未开始 → **已完成**（run G0-20261111-174257-8532）：verify-environment v1.0 + schema + 自测 5/5 + 幂等 YES；environment.json 真机生成（Build 26200.9974）；工具链实测落档；WDK 记录不安装原因；docs/platform-windows.md 快照；一致性检查 38/38 | M0_TASKS DoD 四条 + MASTER §6 规则 + EVIDENCE_SPEC |
| 2026-11-11 | 状态变更 | M0-T2 未开始 → **已完成**（run G0-20261111-182351-1DB8）：官方 Windows-Camera Sample 原样构建（commit 626f8b19，MIT，SBOM 0.2 五新增条目）；3 轮注册→拉流（TestVCam 30 帧校验×3）→Windows Camera 合成画面像素级留证→注销循环全通过，设备实例 ID 逐轮一致；完整回滚（/Uninstall + COM 键删 + 目录删 + Frame Server DLL 缓存处置）终态断言 ABSENT/COM=no；SYS_CHANGE 台账 3 项均经用户确认；偏差（BuildTools 无 VS IDE→MSI 不可构建，README 键 + 官方 Installer 控制台替代）记 run decision.md §3；复现文档 `platforms/windows/virtual-camera/README.md` | M0_TASKS M0-T2 DoD + AGENTS §3 SYS_CHANGE + 基线 §14.1/§14.3（无 ADR 级边界触碰） |
