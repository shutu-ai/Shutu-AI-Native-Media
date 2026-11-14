# contracts/media/ — 媒体会话契约（占位）

> **状态**：占位（M0-T6 建立目录，2026-11-13）。无协议字段。
> **冻结点**：v0.1 @ M2-T1（Session / Track / Clock / Event / Policy 契约冻结 + 契约自动化测试）。

## 规划内容（M2-T1 交付，此处仅登记范围防遗漏）

- Session 生命周期与状态机（创建/激活/暂停/关闭）。
- Track 类型与音视频联合路径约束（Audio/Video Output/Input Track 语义）。
- Clock 与时间轴（同一 Session 音视频同步基准，NFR-04/05 的契约前提）。
- Event 通道语义（健康、降级、设备状态事件的分类与投递保证）。
- Policy 挂载点（授权、采样率约束等策略的声明位置，§11 授权控制的落点）。
- 帧格式头（M1-T3 ring 帧格式的首个具体化输入：格式、代际序号、PTS、有界性）。

## 约束

- 具体字段、枚举与消息编码一律在冻结任务（M2-T1）产出——本占位不预定义（R-10 同型风险）。
- 冻结前修改本 README 范围描述无须 ADR；冻结后走 ADR（见上级 README 冻结流程）。

## caps 决策记录（M1-T2 实测，2026-11-14；非协议冻结，字段冻结仍归 M2-T1）

M1_TASKS M1-T2 字段 5-2 要求「格式由本任务实测确定并写入 contracts 占位文档」，记录如下：

- **决策**：`video/x-raw, format=NV12, width=1280, height=720, framerate=30/1`（固定 caps，禁动态重协商，R-04）。
- **选型理由**：
  1. 与 media-core `FramePolicy::TARGET_720P30`（1280x720@30 **NV12**）对齐——媒体策略类型先于管线存在（M1-T1），管线以策略为输入（gst-adapter → media-core 依赖方向实证）。
  2. NV12 是 Windows 摄像头/虚拟设备通路的主流 YUV 交付格式；M0 侧 Frame Server 拉流为 YUV 系，选定 NV12 使进程内测试管线的帧布局经验可直接迁移到后续设备承载（M1-T3~T6）。
  3. NV12 双平面（Y + 半分辨率 UV）使 stride/offset 处理成为强制验证点——M1-T2 测试 T4 以公式核对布局（UV 偏移 = Y stride×高；总长 = UV 偏移 + UV stride×高/2），杜绝「假定紧密排列」。
  4. BGRA（候选）为 4 字节单平面、单帧 3.7 MB（vs NV12 1.38 MB），在无逐像素消费需求的测试管线里徒增 2.7 倍带宽与内存上限压力。
- **实测协商结果**（run G1-20261114-085728-657b，`perf_capture` 10 s 窗口）：
  - 协商格式 `NV12`，1280x720，2 平面；`stride = [1280, 1280]`（无对齐 padding）；`plane_offsets = [0, 921600]`；`map_size = 1_382_400` 字节（恰 = 1280×720×1.5）。
  - PTS 单调无违例（10 s / 299 帧）；帧 duration 恒 33.333 ms；实测 29.978 fps（is-live 源实时节奏）。
- **映射代码**：`runtime/gst-adapter/src/caps.rs`（media-core PixelFormat → caps 字符串；NV12/RGB 两映射，测试覆盖）。
