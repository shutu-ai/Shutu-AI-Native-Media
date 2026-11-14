# FrameEnvelope v0.1 草案 —— 传输无关帧封装契约

> **状态**：草案 v0.1（M1-T3，2026-11-14）。随实现冻结于 run `G1-20261114-104836-24c7`；
> 参考 Rust 实现 `runtime/media-core/src/envelope.rs`（契约测试 `tests/contract`）。
> **性质**：传输无关——只定义「一帧的封装头如何编码与验证」，不定义帧如何跨进程搬运
> （承载语义归 M1-T4/T5 + P1-3 实测；本契约不含任何传输实现）。
> **冻结点**：M2-T1 与 Session/Track/Clock 契约一并冻结（届时升 v0.1 → v1.0；
> 冻结前修改走本文档版本递增；冻结后走 ADR）。

## 1. 二进制布局（v1，固定 96 字节头，显式小端 LE）

| 偏移 | 大小 | 字段 | 类型 | v1 约束 |
|---|---|---|---|---|
| 0 | 4 | MAGIC | `u8[4]` | 恒 `SVFE`（0x53 0x56 0x46 0x45） |
| 4 | 2 | HEADER_VERSION | `u16 LE` | 恒 1 |
| 6 | 2 | FLAGS | `u16 LE` | 恒 0（v1 保留，非零即拒） |
| 8 | 4 | HEADER_SIZE | `u32 LE` | 恒 96 |
| 12 | 4 | PAYLOAD_LEN | `u32 LE` | ≥ 布局几何要求的最小字节数（允许尾部余量） |
| 16 | 8 | SEQ | `u64 LE` | 同代际内从 1 起严格递增（跳空允许，回退/重复 = 重放） |
| 24 | 8 | GENERATION | `u64 LE` | 代际序号，源重建时递增；新代际重置 SEQ |
| 32 | 8 | PTS_NS | `u64 LE` | 展示时间戳（ns；时钟语义由上游承载，本契约不解释） |
| 40 | 8 | DURATION_NS | `u64 LE` | 期望帧时长 ns（0 = 未知） |
| 48 | 2 | PIXEL_FORMAT | `u16 LE` | 1 = NV12；2 = BGRA（扩展格式须升版本，不得复用 v1 代码） |
| 50 | 2 | PLANE_COUNT | `u16 LE` | NV12=2；BGRA=1 |
| 52 | 2 | WIDTH | `u16 LE` | > 0 |
| 54 | 2 | HEIGHT | `u16 LE` | > 0 |
| 56 | 8 | GRAPH_VERSION | `u64 LE` | 媒体图配置版本 |
| 64 | 32 | PLANE_TABLE | 4 × `(u32 stride, u32 offset)` | 已声明平面之外的槽必须全零 |
| 96 | — | PAYLOAD | `u8[PAYLOAD_LEN]` | 按平面表寻址；本契约不定义内容 |

对齐与字节序：**全程小端**；结构体按字段边界对齐即可（头部无填充字节，
`HEADER_SIZE` 恒 96 可作编译期断言）。C++ 侧用 `#pragma pack` 或逐字段
memcpy 解析，禁止直接 `reinterpret_cast` 到未打包结构体。

## 2. 平面几何规则（按格式；不假定平面紧密排列）

- **NV12**（2 平面）：
  - Y：`stride0 ≥ width`；UV：`stride1 ≥ width`；
  - `offset1 ≥ offset0 + stride0 × height`（UV 不得越入 Y 数据区）；
  - 要求字节数 = `offset1 + stride1 × ceil(height / 2)`。
- **BGRA**（1 平面）：`stride0 ≥ width × 4`；要求字节数 = `offset0 + stride0 × height`。
- 通用：要求字节数 ≤ `PAYLOAD_LEN`；算术全程 64 位 checked，溢出即拒
  （v1 字段宽度下公共 API 不可达，作为未来扩展的防御性出口保留）。
- stride 允许大于格式最小值（对齐 padding）；offset 允许尾部余量——
  下游**必须按平面表寻址**，不得假设紧凑排列（M1-T2 T4 公式核对法）。

## 3. 错误码（稳定 KIND 名；golden 断言与跨语言对照锚点）

| KIND | 触发条件 |
|---|---|
| `Truncated` | 输入 < 96 字节 |
| `InvalidMagic` | MAGIC ≠ `SVFE` |
| `UnsupportedVersion` | HEADER_VERSION ≠ 1 |
| `BadFlags` | FLAGS ≠ 0 |
| `BadHeaderSize` | HEADER_SIZE ≠ 96 |
| `UnknownPixelFormat` | PIXEL_FORMAT 不在 {1, 2} |
| `InvalidPlaneCount` | PLANE_COUNT 与格式规定不符 |
| `InvalidDimensions` | 宽或高为 0 |
| `TrailingPlaneDataNotZero` | 已声明平面之外的槽非零 |
| `PlaneStrideTooSmall` | 平面 stride < 格式公式最小值 |
| `PlaneOrdering` | 后续平面 offset 越入前序平面数据区 |
| `PlaneOverflow` | 平面尺寸算术溢出（checked 拒绝回绕） |
| `PayloadTooShort` | 声明 payload 长度 < 布局几何要求 |

## 4. 金样本（Golden Vectors）

- 文件：`tests/contract/golden/frame_envelope_v1.txt`（构建期内嵌）。
- 推导：纯 Python `struct.pack` 独立生成（provenance：
  `artifacts/G1/G1-20261114-104836-24c7/golden_gen.py`），与被测实现零共享代码。
- 行格式：`VALID <name>` / `ERROR <name> <KIND>` 开头，`HEX <128 hex>` 行
  （每行 64 字节）拼接为头字节，VALID 以 `FIELDS k=v,...` 声明期望字段，
  `END` 结束；`#` 注释与空行忽略。
- 覆盖：4 VALID（NV12/BGRA canonical、payload 余量、1920 对齐 stride）
  + 13 ERROR（全部错误码路径，含 `overflow_extreme_offset` → `PayloadTooShort`）。
- **一致性锚点**：`nv12_720p30_canonical` 与 M1-T2 实测布局逐字节对齐
  （stride=[1280,1280]、offsets=[0,921600]、payload=1,382,400——
  contracts/media/README.md §caps）。未来 C++ 消费端（M1-T4/T5）实现前，
  必须先通过同一金样本文件。

## 5. 缓冲语义（传输无关；`device-bridge::framebuffer`）

- 双预算：`max_frames`（默认 4，对齐 M1-T2 背压实测 queue-max）+
  `max_bytes`（默认 32 MiB）；满载策略 `RejectNew`（背压上游）或
  `DropOldest`（逐出最旧至新帧可放入；单帧超预算恒拒）。
- 入帧校验：封装头必须先通过 §2 几何校验；代际倒退拒、同代际序号回退/
  重复拒（重放）、新代际重置序号表。
- 出帧：`read_latest(generation)`——最新帧代际与请求不符即拒（旧代际拒绝；
  超前代际同样不匹配）。
- 丢弃可观测：`BufferLevels.dropped` 为 DropOldest 策略下的累计逐出帧数
  （指标上报数据源；RejectNew 恒 0）。
- 实现纪律：临界区内无 I/O、无分配重负载；锁毒化恢复内部状态不 panic。

## 6. 边界声明（防 Mock 冒充）

本契约与其测试均为**进程内、传输无关**语义。任何「帧已被另一进程消费」的
结论**不得**引用本契约——跨进程承载的实测归 M1-T4（Named Pipe 候选 +
P1-3 矩阵）与 M1-T5（MediaSource 接入），且在此之前任何承载不得标记为
生产可用（ADR-012 D2）。
