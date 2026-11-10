# M6 任务明细 — Linux/macOS + IM 消费者（后续里程碑，条件触发）

> **来源**：`docs/ARCHITECTURE_BASELINE.md` §9 M6、§5.4（架构基线 v1.0）。
> **定位**：M6 属**后续里程碑**，不构成 V1 交付承诺（§1.3 Non-goals、ADR-008）。启动条件：G5 = GO **且**用户明确批准对应平台的资源投入。每个 spike 任务独立给出 GO/HOLD/STOP 结论，互不阻塞。
> **硬约束**（§5.4）：不承诺 M0–M4 实现跨平台；只承诺**跨平台 Contract 不暴露 Windows 私有句柄作为公共 API**；设备不得反向依赖 IM 才能运行（§9 M6 明文，ADR-008）。
> **本文件任务均为规划态**：细化粒度允许在启动前经一次规划评审调整，调整须记录原因。

## 任务总览

| ID | 目标摘要 | 前置依赖 | AUTH | 状态 |
|---|---|---|---|---|
| M6-T1 | OS Adapter 契约评审 v1（无 Windows 私有句柄泄漏） | G5=GO + 用户批准 | — | 未开始 |
| M6-T2 | Linux PipeWire 虚拟摄像头/音频 spike | M6-T1 + 用户批准 | — | 未开始 |
| M6-T3 | macOS CoreMediaIO / Core Audio spike | M6-T1 + 用户批准 | — | 未开始 |
| M6-T4 | LiveKit/WebRTC 消费者 spike | M6-T1 + 用户批准 | — | 未开始 |
| M6-T5 | AI-Native IM 早期集成评估报告 | M6-T2, M6-T3, M6-T4 | — | 未开始 |

---

## M6-T1 · OS Adapter 契约评审 v1

**1. Task ID 与目标**
对 `contracts/` 全量评审：确认公共契约（Track/Session/Clock/Capability/Event/Policy）中**无 Windows 私有 handle、无 Win32 专属语义泄漏为公共 API**（§5.4）；输出平台适配层（OS Adapter）的接口边界定义 v1；形成各平台差异清单（能力、授权、签名机制）。

**2. 对应需求 FR / 架构 ADR**
- FR-003、FR-006（契约的平台中性）；ADR-005、ADR-006（OS Adapter 策略）；基线 §3（三平面）、§5.4、§9 M6、§14.3（契约变更走 ADR）。

**3. 前置依赖**
G5 = GO + 用户批准启动跨平台工作。

**4. 允许修改的模块与文件范围**
- 新建：`docs/adr/000X-os-adapter-v1.md`、`contracts/` 评审记录
- 允许修改：`contracts/`（仅中性化修正，逐条 ADR）

**5. 实施步骤**
1. 静态扫描 `contracts/` 公共面：句柄类型、路径语义、权限模型、时间语义的平台耦合点清单。
2. 逐条判定：平台私有 → 下沉到 `platforms/windows/` 适配实现；平台中性 → 保留公共契约。
3. OS Adapter 接口边界 v1：`platforms/<os>/` 必须实现的适配接口集（采集、虚拟设备注册、IPC 原语、权限）。
4. 平台差异清单：Linux（PipeWire/V4L2 生态、发行版/会话管理器差异 [S10]）、macOS（CoreMediaIO 扩展/Entitlement [S11]、Core Audio Plug-in [S12]）。
5. 评审结论 + ADR 入库；必要时 contracts 小版本升级（v0.2.x，兼容性说明）。

**6. 自动化测试要求**
- 契约平台耦合静态检查（扫描脚本：禁用类型清单断言）纳入 CI。

**7. 真实环境验收要求**
- CI 级证据即可；真实平台验证属 M6-T2/T3 spike。

**8. 完成标准（DoD）**
- 评审完成、ADR 入库、扫描脚本常驻 CI；三个 spike 任务的验收标准可据此展开。

**9. 风险及失败处理**
- 大面积平台泄漏 → 不强行一次修完：分批 ADR + 迁移计划；spike 先以适配层包装推进（不阻塞研究性质）。

**10. 应保留的测试证据**
评审记录、`docs/adr/000X`、扫描脚本与输出、`test-results.json`。

**11. 状态**：未开始

---

## M6-T2 · Linux PipeWire Spike（虚拟摄像头/音频）

**1. Task ID 与目标**
限定范围研究 spike：在选定发行版（记录版本）上验证 PipeWire 生态的虚拟摄像头输出与虚拟音频源（Loopback/Combine Stream [S10]）可行性；runtime 核心移植评估（gstreamer-rs 部分应基本平移）；产出 GO/HOLD/STOP 与工作量估算。**不做产品化，不承诺任何发行版支持**。

**2. 对应需求 FR / 架构 ADR**
- FR-001/FR-002 的跨平台映射研究；ADR-002（GStreamer 跨平台优势的验证）；基线 §5.4 Linux、§9 M6。

**3. 前置依赖**
M6-T1 + 用户批准 Linux 投入（含测试环境）。

**4. 允许修改的模块与文件范围**
- 新建：`platforms/linux/`（spike 代码，隔离目录）
- 复用：`runtime/`（只读评估；必要的平台条件编译单独列出清单）

**5. 实施步骤**
1. 测试环境固化：发行版/版本、PipeWire 版本、桌面会话类型（Wayland/X11）记录进 environment.json（复用 M0-T1 schema）。
2. 虚拟摄像头路径研究：V4L2 loopback 内核模块 与 PipeWire 摄像头源两种途径的对比（内核模块涉及 DKMS——**属系统级安装，需环境所有者授权**）。
3. 虚拟音频路径：PipeWire loopback/combine 验证第三方应用（浏览器、录音工具）读取。
4. runtime 移植评估：Rust/GStreamer 代码在 Linux 编译运行的实际障碍清单。
5. 输出 spike 报告：可行路径、风险、工作量、GO/HOLD/STOP 建议。

**6. 自动化测试要求**
- spike 内可脚本化部分（端点枚举、音频环回）脚本化；其余人工记录。

**7. 真实环境验收要求**
- 必须在真实 Linux 桌面环境验证应用读取；**容器/无桌面环境验证不构成 spike 结论**。

**8. 完成标准（DoD）**
- spike 报告完成；结论经用户确认；是否升级为正式里程碑由用户决策。

**9. 风险及失败处理**
- 发行版碎片化（§5.4 明示风险）→ 结论限定在实测发行版 + 版本，禁止泛化承诺。
- V4L2 loopback 需要内核模块 → 默认不装；需要时走授权流程（本文件总约束）。

**10. 应保留的测试证据**
`artifacts/M6/<run-id>/`（环境、日志、录制样本）、spike 报告、`decision.md`。

**11. 状态**：未开始

---

## M6-T3 · macOS Spike（CoreMediaIO Camera Extension + Core Audio）

**1. Task ID 与目标**
限定范围研究 spike：CoreMediaIO Camera Extension 发布系统摄像头（macOS 12.3+ [S11]）与 Core Audio Server Plug-in / Driver Extension（[S12]）的可行性；重点核查**签名、Entitlement、系统权限（TCC）与分发机制**；Rust/GStreamer 在 macOS 的工具链现状评估。产出 GO/HOLD/STOP。**不做产品化**。

**2. 对应需求 FR / 架构 ADR**
- 同 M6-T2 性质；ADR-002；基线 §5.4 macOS、§9 M6。

**3. 前置依赖**
M6-T1 + 用户批准 macOS 投入（**必须有真实 macOS 硬件与 Apple 开发者账号条件评估**——无硬件则直接 HOLD）。

**4. 允许修改的模块与文件范围**
- 新建：`platforms/macos/`（spike，隔离目录）
- 复用：`runtime/`（只读评估）

**5. 实施步骤**
1. 硬件/账号条件确认（缺任一 → HOLD 并记录）。
2. CoreMediaIO：官方 Camera Extension 范例构建、系统扩展审批流程（用户授权对话框）、Xcode/签名链要求记录。
3. Core Audio：Audio Server Plug-in（用户态）与 Driver Extension（ DriverKit，涉及付费 Entitlement 申请）双路径对比。
4. GStreamer macOS（osxframework）与 Rust aarch64/x86_64 工具链冒烟。
5. spike 报告：签名/分发成本（Developer ID、公证）、可行路径、工作量、结论。

**6. 自动化测试要求**
- 可脚本化部分（构建、端点出现断言）脚本化；系统权限流程人工记录。

**7. 真实环境验收要求**
- 必须在真实 macOS 硬件上验证系统扩展加载与应用读取；模拟器/文档推导不构成结论。

**8. 完成标准（DoD）**
- spike 报告完成；结论经用户确认。

**9. 风险及失败处理**
- DriverKit Entitlement 申请周期不可控 → 记录为轨道风险；用户态 Plug-in 路径作为替身评估。
- 苹果签名/分发成本高 → 如实入报告；不承诺规避任何 Apple 安全机制。

**10. 应保留的测试证据**
`artifacts/M6/<run-id>/`、spike 报告、签名流程记录、`decision.md`。

**11. 状态**：未开始

---

## M6-T4 · LiveKit/WebRTC 消费者 Spike

**1. Task ID 与目标**
把 `Shutu AI Camera` / `Shutu AI Microphone` 作为普通系统设备接入 LiveKit/WebRTC 消费路径的可行性验证：浏览器端 + LiveKit 服务（自托管，Apache-2.0 [S16]）真实发布虚拟设备的音视频 Track；验证设备不反向依赖 IM（不装 IM 时设备照常工作，ADR-008）。产出 GO/HOLD/STOP 与「IM 作为消费端」的架构确认。

**2. 对应需求 FR / 架构 ADR**
- FR-003（统一输出被任意兼容应用读取的延伸验证）；ADR-008（LiveKit 作为传输层/消费者，不作为前置依赖）；基线 §8（LiveKit/Pion 定位）、§9 M6。

**3. 前置依赖**
M6-T1 + 用户批准（含自托管服务的运行资源）；**可与 M6-T2/T3 并行**（只依赖 Windows 已交付设备）。

**4. 允许修改的模块与文件范围**
- 新建：`tests/integration/`（IM 消费者场景）、`docs/`（消费者验证记录）
- **禁止**：为 IM 需求修改设备侧实现（ADR-008 硬边界）

**5. 实施步骤**
1. 自托管 LiveKit Server（版本、许可入 SBOM；仅测试网络，不暴露公网）。
2. 浏览器客户端发布 `Shutu AI Camera/Microphone`（getUserMedia → LiveKit）端到端记录。
3. 断依赖演练：停止 LiveKit/断网 → 本机虚拟设备与应用完全不受影响（ADR-008 断言）。
4. （可选）Pion WebRTC 小样例对照（[S16]，仅研究，不入 V1 核心）。
5. spike 报告：可用性、质量观察（时延/画质粗测）、结论。

**6. 自动化测试要求**
- 发布/订阅脚本化（LiveKit API/CLI）；断依赖断言自动化。

**7. 真实环境验收要求**
- 真实服务器 + 真实浏览器订阅端看到虚拟设备画面/听到音频（录制留证）。

**8. 完成标准（DoD）**
- spike 报告完成；「设备不依赖 IM」断言有证据。

**9. 风险及失败处理**
- IM 应用对设备的特殊要求（如设备枚举时机）→ 记录为该 IM 的适配问题，不改设备主线（R-09：防止 IM 需求稀释主线资源）。

**10. 应保留的测试证据**
`artifacts/M6/<run-id>/`、录制样本、`licensing-sbom.json`（LiveKit 条目）、spike 报告。

**11. 状态**：未开始

---

## M6-T5 · AI-Native IM 早期集成评估报告

**1. Task ID 与目标**
汇总 M6-T2/T3/T4 证据与基线长期愿景（§1.1），产出 AI-Native IM 集成的早期评估：语义/Agent API（受授权的本地语义 API，基线架构图 API 节点）的最小契约草案、五形态场景（§4.6）在 IM 语境下的优先级建议、资源与风险估算。**本任务只产出报告，不启动任何 IM 开发**。

**2. 对应需求 FR / 架构 ADR**
- ADR-008、ADR-005（语义 Track）；基线 §1.1、§4.6、§9 M6、§12 R-09。

**3. 前置依赖**
M6-T2、M6-T3、M6-T4（或其 HOLD/STOP 结论——部分结论亦可成文，标注缺口）。

**4. 允许修改的模块与文件范围**
- 新建：`docs/`（评估报告）、`contracts/capability/`（Agent/语义 API 契约草案，仅草案）

**5. 实施步骤**
1. 证据汇总：三 spike 的可行性与成本矩阵。
2. 语义 API 契约草案：授权范围、事件类型、隐私边界（衔接 §11-1/§11-8 与 M4-T2 防注入边界）。
3. 场景优先级建议（§4.6 五形态 × IM 价值 × 已验证可行性）。
4. 资源估算与风险登记（新 R 项建议提交风险登记册增补走 ADR）。
5. 报告评审 → 用户决策是否立项；未立项则归档。

**6. 自动化测试要求**
- 契约草案 schema 校验（若定义了机器可读格式）。

**7. 真实环境验收要求**
- 报告基于的实证必须可溯源到 M6-T2/T3/T4 证据 run-id；无实证支撑的建议单独标注。

**8. 完成标准（DoD）**
- 报告完成并评审；用户对立项与否有明确决策记录。

**9. 风险及失败处理**
- R-09（IM 稀释主线）→ 报告必须含「不立项」选项的完整理由；任何立项动作另行走门禁。

**10. 应保留的测试证据**
评估报告、契约草案、评审记录、`decision.md`。

**11. 状态**：未开始
