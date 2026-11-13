# 威胁模型（初版 v0.1）

> **状态**：M1-T1 初版——列信任边界与已知缓解，标注待完善项；随 M1-T4/T5（IPC 实现）与 M2/M3（设备与音频）迭代。
> **依据**：架构基线 §11（隐私与安全）、ADR-004/006/012、M0 真机事实（Frame Server 身份/Session 0/管道 DACL 边界，run 3C36）；AGENTS §3-§5。
> **方法**：STRIDE-lite 按信任边界枚举；初版不评分，只登记「威胁 → 现状/计划缓解 → 状态」。

## 1. 系统组件与信任等级

| 组件 | 信任等级 | 说明 |
|---|---|---|
| Frame Server（svchost -k Camera） | **OS 高**（Session 0，LocalService S-1-5-19） | 系统服务；经 M0-T4 实测其进程令牌含 SeCreateGlobalPrivilege 与 per-service SID |
| Runtime（media-core/gst-adapter/device-bridge） | 用户会话（Session 1） | 本仓库产品代码；无特权（M0-T4 T1 实测：无 SeCreateGlobalPrivilege） |
| 虚拟摄像头 MediaSource（C++ Sample 派生） | OS 服务内加载（Frame Server 进程） | M0-T4 F 系列实测：DLL 加载进 svchost——**其缺陷即服务级缺陷** |
| 控制面（apps/media-lab） | 用户会话 | 用户交互入口 |
| Worker / 低优先级处理（规划） | 用户会话，**必须与热路径隔离** | 基线 §3.4：不得进入 OS 回调/音频热路径 |
| 第三方依赖（crates.io / GStreamer SDK） | 供应链 | SBOM 滚动登记（docs/licensing-sbom.json，AGENTS §6） |

## 2. 信任边界与威胁登记

### B1：用户会话 Runtime ↔ Frame Server（跨会话管道边界）

- **事实**：唯一 OS 级身份边界 = 管道 DACL 显式授 S-1-5-19（M0-T4 T5/T6：不授即拒 gle=5，fail-closed）；跨会话进程级 token 校验不可行（T5）。
- 威胁：①非授权进程伪装 Frame Server 连接管道（Spoofing）；②管道消息注入/重放（Tampering）；③服务端阻塞/资源耗尽（DoS，禁 Frame Server COM 回调阻塞——基线 §5.1）；④消息内容越权（Elevation）。
- 缓解（现状/计划）：DACL 保护描述符 D:P 禁 Everyone/NULL SID（ADR-012 S2）；应用层握手（版本+challenge）与消息长度/超时上限（ADR-012 S3，M1-T4 实现验收）；fail-closed 断连（S4）；日志只记 PID/结果/错误码（S7）。
- **状态：设计定案、实现未开始（M1-T4/T5 = P1-1/P1-2 责任）**。per-service SID（NT SERVICE\FrameServer）收紧候选待实测（ADR-012 U1）。

### B2：Runtime ↔ 虚拟摄像头设备（进程隔离，ADR-006）

- 威胁：①Runtime 崩溃/被杀导致设备状态残留（DoS/防残留，FR-008）；②帧内容污染下游（Tampering）；③设备侧读写竞态（P1-2 异步取消风险域）。
- 缓解：Runtime 独立进程隔离；断连占位帧降级与注册保持（M1-T6 验收）；残留检查清单（NFR-09）。
- **状态：M1-T6 验收，未实现**。

### B3：控制面 ↔ Runtime

- 威胁：①未授权控制命令（如远程触发设备注销）；②控制面泄露会话/设备元数据。
- 缓解（计划）：控制面仅本机回环 + 本用户会话 ACL；命令白名单；审计日志。
- **状态：未设计（M2+ 控制面任务），初版占位登记**。

### B4：热路径隔离（基线 §3.4 / §5.1）

- 威胁：低优先级处理（识别/网络/模型调用）进入 OS 驱动回调或音频热路径 → 卡顿/死锁（DoS）。
- 缓解：架构禁令（AGENTS §5 禁止事项，逐任务字段复查）；Frame Server COM 回调中禁止网络/AI/超预算等待（**严禁未验证的无限等待**）。
- **状态：架构约束在册；M1-T2 起每次管线实现时复查**。

### B5：外部输入（识别文本 = 不可信输入）

- 威胁：来自语音/视频识别的文本触发未授权命令/路径/设备策略变更（基线 §11-6）。
- 缓解：文本仅作数据流处理；命令通道与数据通道物理分离；任何由文本触发的动作需显式授权层。
- **状态：M3+ 生效前的架构红线（在册，无实现面）**。

### B6：供应链（crates.io / GStreamer SDK / 官方 Sample 源）

- 威胁：依赖投毒、许可证不合规、版本漂移（MSRS/GStreamer 最低版本错配）。
- 缓解：逐包 SBOM（docs/licensing-sbom.json v0.4，Cargo.lock 为闭包权威）；MSRS pin 记录（kstring 2.0.2）；升级须复测记录进 environment.json（M1-T1 字段 9，不静默升级）。
- **状态：机制已建立并运行**。

## 3. 隐私（基线 §11 映射）

1. 默认不保存原始媒体；调试采集须「调试开关 + 明确同意 + 短期限时」三条件（G0 实例：b4dc 4 文件，保留期至 2026-12-13）。
2. 日志/指标只含时序、设备、异常与性能元数据（media-core metrics 骨架已按此约束注释）。
3. 截图类证据须脱敏；个人场景内容不入公开仓库（artifacts/ gitignore，G0 审计验证）。
4. 未授权后台监听/静默采集 = 红线（AGENTS §3-§5；G0 全程 SYS_CHANGE 台账实践）。

## 4. 待完善项（迭代计划）

1. B1 管道协议具体威胁建模（消息格式、重放窗口、握手段差）——随 M1-T4 设计稿产出。
2. B2 故障注入威胁清单——随 M1-T6 fault-injection 场景产出。
3. B3 控制面授权模型——M2 控制面任务开启时正式化（可能出新 ADR）。
4. STRIDE 完整评分（DREAD 或类似）——G2 前完成。
5. 供应链：crates.io 依赖的 SAST/audit 接入（cargo-audit / cargo-deny 评估）——CI 接入时定。
