# 验收矩阵（FR / NFR / 风险 / 发布清单 × 任务 × 证据）

> **来源**：`docs/ARCHITECTURE_BASELINE.md` §1.2（FR）、§7（NFR）、§12（风险）、§15（发布清单）。
> **证据级别取值**：`真机` = 真实 Windows（或对应平台）设备 + 真实应用验证；`CI` = 自动化测试通过；`Mock` = mock/替身素材（标注具体边界）。**无真机证据的单元格强制标注 MOCK**——基线 §10.3：「绝不将『代码通过 CI』误报为『Windows 虚拟设备已在真实应用验证』」。
> **单元格格式**：`任务ID · 级别`。`—` = 该 Gate 不覆盖此项。

## 1. 功能需求 FR-001 ~ FR-009

| 编号 | 摘要 | G0 | G1 | G2 | G3 | G4 | G5 | 证据级别 | 证据文件 | 验证应用 |
|---|---|---|---|---|---|---|---|---|---|---|
| FR-001 | 虚拟摄像头注册/枚举/启停 | M0-T2·真机；M0-T3·真机 | M1-T5·真机；M1-T6·真机 | — | — | — | M5-T2·真机；M5-T4·真机 | 真机 | device-enumeration.txt、screenshots/、test-results.json | Windows Camera、Chromium、OBS |
| FR-002 | 虚拟麦克风受控验证（Track B 另设门禁） | M0-T5·书面 | — | M2-T3·真机(Track A)；M2-T6·隔离测试机(AUTH) | — | — | M5-T4·真机；M5-T6·决策 | 真机（Track A 明确标注非自研驱动） | device-enumeration.txt、perf.json、screenshots/ | 录音工具、浏览器音频页、会议软件（按实测） |
| FR-003 | 两设备读统一 Runtime 输出、独立启停 | — | M1-T6·真机 | M2-T4·真机 | — | M4-T5·Mock素材+真机输出 | — | 真机 | perf.json、test-results.json、screenshots/ | Windows Camera + 第二应用 |
| FR-004 | 旁路/无AI/单AI/多AI 四模式 | — | M1-T2·CI（旁路雏形） | — | M3-T1·真机(单)；M3-T5·真机输出(Mock合成音替身)；M3-T8·真机(多链) | M4-T2·真机(理解)；M4-T3·真机输出(预录/合成帧替身)；M4-T5·Mock素材(端到端) | — | 真机 + Mock 素材边界标注 | test-results.json、perf.json | 同上 |
| FR-005 | 物理摄像头/麦克风采集 + 授权控制 | — | M1-T7·真机 | M2-T2·真机 | — | — | — | 真机 | environment.json、perf.json、screenshots/ | 系统隐私面板 + 应用 |
| FR-006 | 格式协商、时间戳、缓冲边界、降级、指标 | M0-T6·CI(证据设施) | M1-T2·CI；M1-T3·CI | M2-T1·CI；M2-T4·真机 | M3-T3·CI | — | M5-T5·真机 | CI + 真机混合（逐格标注） | test-results.json、perf.json | — |
| FR-007 | 模型故障不拖垮设备与非 AI 通路 | — | M1-T6·真机(Runtime kill 预演) | — | M3-T7·真机(NFR-06 正式) | — | M5-T5·真机(复测) | 真机 | perf.json、故障注入日志 | 设备持续注册 + 应用表现 |
| FR-008 | 安装/更新/卸载可靠清理 | — | M1-T6·真机(基础装卸) | — | — | — | M5-T2·真机；M5-T5·真机(NFR-09 全量) | 真机 | uninstall-verify 报告、test-results.json | 安装器 + 设备枚举 |
| FR-009 | 采集/上传/生成/AI 参与可见可批准可撤销 | — | M1-T7·真机(采集权限) | M2-T5·真机(麦克风撤销) | M3-T7·真机(AI 授权撤销) | M4-T2·真机(采样授权)；M4-T3·CI(来源元数据) | M5-T1·真机(GUI)；M5-T5·真机(NFR-08 全量) | 真机 | security-checklist.md、网络/文件追踪、screenshots/ | OS 权限面板 + Control GUI |

## 2. 非功能指标 NFR-01 ~ NFR-10（测量条件照基线 §7 记录）

| 编号 | 指标 / 门槛 | 首测任务 · Gate | 复测/终验 | 测量方法（§7） | 证据级别 | 证据文件 |
|---|---|---|---|---|---|---|
| NFR-01 | 720p30 无 AI 稳定 30fps，30min 无死锁/崩溃 | M1-T8 · G1 | M5-T5 · G5 | 统计收帧/出帧与时间戳；存日志 | 真机 | perf.json、build.log |
| NFR-02 | 48kHz 单声道 10ms 块，30min 无持续断流，underrun 统计完整 | M2-T3 · G2 | M5-T5 · G5 | 固定测试音生成，录音比对 + 缓冲事件统计 | 真机 | perf.json、录制样本 |
| NFR-03 | Camera 输出新增时延 p95 ≤150ms（无 AI） | M1-T8 · G1（初测） | M5-T5 · G5 | 同步闪光测试卡 / 时间戳摄影法 | 真机 | perf.json |
| NFR-04 | Microphone 输出新增时延 p95 ≤100ms（无 AI） | M2-T4 · G2（初测） | M5-T5 · G5（M3-T2 降噪后复测） | 电信号/声学 loopback + 波形对齐，注明路径 | 真机 | perf.json |
| NFR-05 | A/V 相对时间偏差 ≤80ms（无 AI） | M2-T4 · G2 | M5-T5 · G5 | 同步拍手/闪光素材 + PTS 关联 | 真机 | perf.json |
| NFR-06 | Kill AI Worker → 设备不中断注册，2s 内备用模式 | M1-T8 · G1（预演） | M3-T7 · G3（正式）；M5-T5 · G5（复测） | Kill Worker，观察应用与 Health Event | 真机 | perf.json、故障注入日志 |
| NFR-07 | 队列压力下内存/队列有界，无持续线性增长 | M1-T8 · G1（初步） | M3-T7 · G3（正式）；M5-T5 · G5（全量） | 关闭消费端/减速处理端 30min 采样 | 真机 | perf.json |
| NFR-08 | 权限撤销 → 禁止新增未授权输出/上传，任务取消并清理 | M2-T5 · G2（采集·初版） | M3-T7 · G3（AI 部分）；M5-T5 · G5（全量） | 权限事件 + 本地网络/文件追踪 | 真机 | security-checklist.md、追踪记录 |
| NFR-09 | 开始/停止、重启、重装无孤儿进程/遗留设备/可利用 ACL | M1-T6 · G1（基础装卸） | M5-T2 · G5（uninstall-verify）；M5-T5 · G5（重启循环） | 自动化装卸清单 + 手工复核 | 真机 | uninstall-verify 报告、test-results.json |
| NFR-10 | 模型 A/B：完成率/质量、成本、CPU/GPU、时延可比较 | — | — | — | — | — |
| ↳ | （归属）固定素材 A/B 对比 | M3-T6 · G3 | M5 发布材料可引用 | 固定素材与指标脚本，保存对照证据 | 真机（Mock 素材边界标注） | perf.json、对照报告 |

## 3. 风险登记册 R-01 ~ R-12（缓解任务与决策门）

| 风险 ID | 摘要 | 缓解任务 | 决策门 | 证据级别 |
|---|---|---|---|---|
| R-01 | 虚拟麦克风驱动开发/签名成本高 | M0-T5（书面）；M2-T6（PoC·AUTH）；M5-T6（交付决策） | G2、G5 | 书面 + 隔离测试机·真机 |
| R-02 | Frame Server 源无法访问用户进程 IPC | M0-T4（验证+备选决策）；M1-T3/T4（实现遵循决策） | **G0**（不过则不 GO） | 真机 |
| R-03 | 摄像头被独占、多应用冲突 | M1-T6（多消费者）；M1-T7（共享模式打开） | G1 | 真机 |
| R-04 | GStreamer 动态图/协商复杂 | M1-T2（固定管线）；M1-T6（caps 变更测试）；M2-T1（协商契约） | G1、G2 | CI + 真机 |
| R-05 | AI 推理背压、音画延迟 | M3-T1（有界队列+旁路）；M3-T8（级联背压）；M3-T7（NFR-06/07） | G3 | 真机 |
| R-06 | 模型/Codec/插件许可证不兼容 | M0-T2（首次 SBOM）；M1-T1（逐依赖 SBOM 机制）；M2-T3（桥接许可）；M3-T4（模型权重）；M5-T3（封版审查） | G5 | 书面 + 自动化校验 |
| R-07 | OS/GPU/编解码环境差异 | M0-T1（环境盘点）；M1-T2（CPU 基线先行）；M5-T4/T5（矩阵与镜像） | G0、G5 | 真机 |
| R-08 | 隐私与被动监听担忧 | M2-T5/M3-T7（可撤销）；M4-T3（来源透明）；M5-T1（显式状态 GUI） | G2、G3、G5 | 真机 |
| R-09 | 提前开发 IM 稀释资源 | M6-T4/T5（IM 仅作消费者与评估，不入主线） | M6 | 真机（spike） |
| R-10 | 过早设计万能 AI Graph | M0-T6（契约占位）；M2-T1（只冻结 §4 概念）；M3-T3（配置驱动固定拓扑）；M4-T1（以 §4.6 五形态为界） | 各 Gate 评审 | CI |
| R-11 | VPN/安全软件阻止安装或 IPC | M0-T1（管控线索登记）；M5-T5（清洁镜像验证） | G5 | 真机 |
| R-12 | 预览工具独占设备 | M1-T6（多消费者测试）；M5-T1（预览共享读取） | G1、G5 | 真机 |

## 4. 发布清单勾稽（基线 §15）

### §15.1 研发版可运行最低标准

| 清单项 | 覆盖任务 | Gate | 证据级别 |
|---|---|---|---|
| 1. Sample 构建/注册/运行/卸载证据完整 | M0-T2 | G0 | 真机 |
| 2. `Shutu AI Camera` 被 ≥2 独立应用识别消费 | M0-T3、M1-T6 | G0、G1 | 真机 |
| 3. Runtime 经实际 IPC 提供视频帧（非进程内 Mock） | M1-T5、M1-T6 | G1 | 真机 |
| 4. 麦克风被第三方应用识别，且说明桥接 vs 自研驱动 | M2-T3、M2-T6 | G2 | 真机 |
| 5. Session/Clock/Track 协议实现 + 契约测试 | M2-T1 | G2 | CI |
| 6. AI 关闭时设备继续正常提供媒体 | M1-T8、M3-T9（基线复测） | G1、G3 | 真机 |
| 7. AI Worker 停止/超时按规则旁路/降级 | M3-T7 | G3 | 真机 |
| 8. 队列/内存上限、时间戳与同步已测量留证 | M1-T8、M2-T4、M3-T7 | G1~G3 | 真机 |
| 9. 用户可撤销采集与远程 AI 权限；日志无默认媒体 | M2-T5、M3-T7、M5-T1 | G2、G3、G5 | 真机 |
| 10. 安装/卸载/重启可复现；驱动签名限制已记录 | M5-T2、M5-T3 | G5 | 真机 |

### §15.2 双设备可分发 Beta 附加门禁

| 清单项 | 覆盖任务 | Gate | 证据级别 |
|---|---|---|---|
| 1. 自研音频设备/合法分发策略与签名链明确达标 | M2-T6、M5-T3、M5-T6 | G2、G5 | 隔离测试机 + 书面 |
| 2. 普通用户无需关闭 OS 安全防护即可装卸 | M5-T2、M5-T7 | G5 | 真机 |
| 3. 兼容矩阵、性能报告、故障注入、威胁模型、SBOM 通过审查 | M5-T4、M5-T5、M5-T7、M5-T3 | G5 | 真机 + 书面 |
| 4. 启动与设备失效可预期、可回滚、无持久占用 | M5-T2、M5-T5 | G5 | 真机 |
| 5. 用户资料/设备 ID/媒体/调试信息最小保留 | M3-T7、M5-T7 | G3、G5 | 真机 |

## 5. Gate × Evidence 对应表（每个门的固定证据，基线 §10.3）

| Gate | 汇总任务 | 必交九文件 |
|---|---|---|
| G0 | M0-T7 | environment.json、build.log、test-results.json、perf.json、device-enumeration.txt、screenshots/、security-checklist.md、licensing-sbom.json、decision.md |
| G1 | M1-T8 | 同上（perf.json 含 NFR-01/03/07 采样） |
| G2 | M2-T7 | 同上 + trackB 子集 |
| G3 | M3-T9 | 同上 |
| G4 | M4-T6 | 同上 + 零改动 diff 证据 |
| G5 | M5-T8 | 同上 + compatibility-matrix.md 定稿 + Release Evidence |

> 目录约定：`artifacts/<gate>/<run-id>/`；任何缺失文件在该 Gate 的 decision.md 中必须显式说明原因，缺失真机证据 ⇒ 判定不得为 GO。
