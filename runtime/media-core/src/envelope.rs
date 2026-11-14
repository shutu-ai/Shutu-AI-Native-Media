//! FrameEnvelope — 传输无关的版本化帧封装契约（M1-T3，2026-11-14 修订后定义）。
//!
//! 契约文档：`contracts/ipc/frame-envelope-v0.1-draft.md`（与本实现同步维护）；
//! 金样本锚点：`tests/contract/golden/frame_envelope_v1.txt`（Rust ↔ 未来 C++ 消费端共享）。
//!
//! 边界（AGENTS §4 / M1-T3 字段 1/4）：
//! - 本模块只做**头部编解码 + 布局合法性验证**——不含任何跨进程传输实现
//!   （禁止 Global Section / 共享内存 Ring / 特权 Broker / 产品级 Named Pipe）。
//! - 二进制布局显式小端、固定 96 字节头、payload 按平面 stride/offset 描述，
//!   **不假定平面紧密排列**（M1-T2 T4 公式核对法）。
//! - 与 M1-T2 NV12 实测布局一致性锚点：720p30 stride=[1280,1280]、
//!   offsets=[0,921600]、payload=1,382,400（contracts/media/README.md §caps 决策记录）。

use std::fmt;

/// 头部魔数：`SVFE`（Shutu Video Frame Envelope）。
pub const MAGIC: [u8; 4] = *b"SVFE";

/// v1 头部版本号。
pub const HEADER_VERSION_V1: u16 = 1;

/// v1 头部固定大小（字节）。
pub const HEADER_SIZE_V1: usize = 96;

/// 平面描述槽位数（v1 固定 4 槽；未用槽必须全零）。
pub const MAX_PLANES: usize = 4;

/// 平面表在头内的起始偏移。
const PLANE_TABLE_OFFSET: usize = 64;

/// 单平面表项大小（stride u32 + offset u32）。
const PLANE_ENTRY_SIZE: usize = 8;

/// 像素格式代码（v1 契约注册表；扩展格式须升版本，不得复用 v1 代码）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvelopePixelFormat {
    /// 代码 1：NV12——Y 平面 + 半分辨率 interleaved UV 平面（双平面）。
    Nv12,
    /// 代码 2：BGRA——4 字节单平面（ADR-012 S6 的 4 字节代表格式）。
    Bgra,
}

impl EnvelopePixelFormat {
    /// v1 契约格式代码。
    #[must_use]
    pub fn code(self) -> u16 {
        match self {
            Self::Nv12 => 1,
            Self::Bgra => 2,
        }
    }

    /// 由代码反查格式（未知代码 → `None`）。
    #[must_use]
    pub fn from_code(code: u16) -> Option<Self> {
        match code {
            1 => Some(Self::Nv12),
            2 => Some(Self::Bgra),
            _ => None,
        }
    }

    /// v1 契约规定的每格式平面数。
    #[must_use]
    pub fn plane_count(self) -> u16 {
        match self {
            Self::Nv12 => 2,
            Self::Bgra => 1,
        }
    }
}

/// 单平面布局描述（stride 与相对 payload 起点的 offset）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaneLayout {
    /// 行跨度（字节）；必须 ≥ 该平面按格式公式导出的最小跨度。
    pub stride: u32,
    /// 平面数据相对 payload 起点的偏移（字节）。
    pub offset: u32,
}

impl PlaneLayout {
    /// 全零占位（未用槽）。
    pub const ZERO: Self = Self {
        stride: 0,
        offset: 0,
    };
}

/// 解码/校验错误（七类契约异常的显式错误码；`Display` 文本即诊断）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    /// 输入短于 96 字节固定头。
    Truncated {
        /// 需要的字节数。
        needed: usize,
        /// 实际字节数。
        got: usize,
    },
    /// 魔数不是 `SVFE`（坏魔数）。
    InvalidMagic,
    /// 头部版本不受支持（版本不符）。
    UnsupportedVersion {
        /// 实际读到的版本。
        found: u16,
    },
    /// v1 保留 flags 字段非零。
    BadFlags {
        /// 实际 flags。
        found: u16,
    },
    /// header_size 与 v1 固定值不符（坏长度类）。
    BadHeaderSize {
        /// 实际 header_size。
        found: u32,
    },
    /// 未知像素格式代码。
    UnknownPixelFormat {
        /// 实际代码。
        code: u16,
    },
    /// 平面数与格式规定不符。
    InvalidPlaneCount {
        /// 该格式规定的平面数。
        expected: u16,
        /// 实际平面数。
        found: u16,
    },
    /// 宽或高为零（非法尺寸）。
    InvalidDimensions {
        /// 实际宽。
        width: u16,
        /// 实际高。
        height: u16,
    },
    /// `plane_count` 之后的槽位非零（严格布局：未用槽必须全零）。
    TrailingPlaneDataNotZero {
        /// 首个非零槽位下标。
        plane: usize,
    },
    /// 平面 stride 小于该平面按格式公式的最小跨度。
    PlaneStrideTooSmall {
        /// 平面下标。
        plane: usize,
        /// 最小跨度（字节）。
        min: u64,
        /// 实际 stride。
        found: u32,
    },
    /// 后续平面 offset 落在前序平面数据区之内（布局次序违规）。
    PlaneOrdering {
        /// 平面下标。
        plane: usize,
        /// 允许的最小 offset。
        min_offset: u64,
        /// 实际 offset。
        found: u32,
    },
    /// 平面尺寸算术溢出（checked 算术拒绝回绕——溢出防护契约）。
    PlaneOverflow {
        /// 平面下标。
        plane: usize,
    },
    /// 声明的 payload 长度小于布局几何要求（坏长度/溢出路径的统一出口）。
    PayloadTooShort {
        /// 布局要求的最小字节数。
        required: u64,
        /// 声明的 payload_len。
        actual: u32,
    },
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Truncated { needed, got } => {
                write!(f, "truncated header: need {needed} bytes, got {got}")
            }
            Self::InvalidMagic => write!(f, "invalid magic (expected SVFE)"),
            Self::UnsupportedVersion { found } => {
                write!(f, "unsupported header version {found} (supported: 1)")
            }
            Self::BadFlags { found } => {
                write!(f, "reserved flags must be 0 in v1, got {found:#06x}")
            }
            Self::BadHeaderSize { found } => {
                write!(f, "bad header_size {found} (v1 requires {HEADER_SIZE_V1})")
            }
            Self::UnknownPixelFormat { code } => write!(f, "unknown pixel format code {code}"),
            Self::InvalidPlaneCount { expected, found } => {
                write!(
                    f,
                    "plane count {found} != expected {expected} for this format"
                )
            }
            Self::InvalidDimensions { width, height } => {
                write!(f, "invalid dimensions {width}x{height} (must be nonzero)")
            }
            Self::TrailingPlaneDataNotZero { plane } => {
                write!(
                    f,
                    "plane slot {plane} beyond declared plane_count must be zero"
                )
            }
            Self::PlaneStrideTooSmall { plane, min, found } => {
                write!(f, "plane {plane} stride {found} < required min {min}")
            }
            Self::PlaneOrdering {
                plane,
                min_offset,
                found,
            } => write!(
                f,
                "plane {plane} offset {found} < required min {min_offset} (overlaps earlier plane)"
            ),
            Self::PlaneOverflow { plane } => {
                write!(
                    f,
                    "plane {plane} size arithmetic overflow (checked math refused)"
                )
            }
            Self::PayloadTooShort { required, actual } => {
                write!(f, "payload_len {actual} < layout-required min {required}")
            }
        }
    }
}

impl std::error::Error for DecodeError {}

/// 错误变体的稳定名称（golden vectors 的 `KIND` 断言锚点；跨语言对照用）。
#[must_use]
pub fn error_kind(err: &DecodeError) -> &'static str {
    match err {
        DecodeError::Truncated { .. } => "Truncated",
        DecodeError::InvalidMagic => "InvalidMagic",
        DecodeError::UnsupportedVersion { .. } => "UnsupportedVersion",
        DecodeError::BadFlags { .. } => "BadFlags",
        DecodeError::BadHeaderSize { .. } => "BadHeaderSize",
        DecodeError::UnknownPixelFormat { .. } => "UnknownPixelFormat",
        DecodeError::InvalidPlaneCount { .. } => "InvalidPlaneCount",
        DecodeError::InvalidDimensions { .. } => "InvalidDimensions",
        DecodeError::TrailingPlaneDataNotZero { .. } => "TrailingPlaneDataNotZero",
        DecodeError::PlaneStrideTooSmall { .. } => "PlaneStrideTooSmall",
        DecodeError::PlaneOrdering { .. } => "PlaneOrdering",
        DecodeError::PlaneOverflow { .. } => "PlaneOverflow",
        DecodeError::PayloadTooShort { .. } => "PayloadTooShort",
    }
}

/// 版本化帧封装头（传输无关；M1-T3）。
///
/// 不变量（由 [`FrameEnvelope::validate`] 强制）：
/// - 格式/平面数/尺寸合法；未用平面槽全零；
/// - 每平面 stride ≥ 格式公式最小值；后续平面 offset 不越入前序平面数据区；
/// - `payload_len` ≥ 布局几何要求的最小字节数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameEnvelope {
    /// 帧序号（同一 generation 内从 1 起单调递增；重放检测的输入）。
    pub seq: u64,
    /// 代际序号（源/图实例重建时递增；读端可显式拒绝旧代际）。
    pub generation: u64,
    /// 展示时间戳（ns；时钟语义由上游定义，本契约只承载不解释）。
    pub pts_ns: u64,
    /// 期望帧时长（ns；0 = 未知）。
    pub duration_ns: u64,
    /// 媒体图配置版本。
    pub graph_version: u64,
    /// 像素格式。
    pub format: EnvelopePixelFormat,
    /// 帧宽（像素；必须 > 0）。
    pub width: u16,
    /// 帧高（像素；必须 > 0）。
    pub height: u16,
    /// 平面布局表；`format.plane_count()` 之后的槽必须为 [`PlaneLayout::ZERO`]。
    pub planes: [PlaneLayout; MAX_PLANES],
    /// 声明的 payload 总字节数（必须 ≥ 布局几何要求的最小值）。
    pub payload_len: u32,
}

impl FrameEnvelope {
    /// 校验布局几何，返回 payload 要求的最小字节数（u64 全程 checked 算术）。
    ///
    /// # Errors
    /// 上述不变量任一被违反时返回对应 [`DecodeError`]。
    pub fn validate(&self) -> Result<u64, DecodeError> {
        let declared = usize::from(self.format.plane_count());
        for (slot, plane) in self.planes.iter().enumerate().skip(declared) {
            if *plane != PlaneLayout::ZERO {
                return Err(DecodeError::TrailingPlaneDataNotZero { plane: slot });
            }
        }
        if self.width == 0 || self.height == 0 {
            return Err(DecodeError::InvalidDimensions {
                width: self.width,
                height: self.height,
            });
        }
        let w = u64::from(self.width);
        let h = u64::from(self.height);
        let required = match self.format {
            EnvelopePixelFormat::Nv12 => self.validate_nv12(w, h)?,
            EnvelopePixelFormat::Bgra => self.validate_bgra(w, h)?,
        };
        if required > u64::from(self.payload_len) {
            return Err(DecodeError::PayloadTooShort {
                required,
                actual: self.payload_len,
            });
        }
        Ok(required)
    }

    /// NV12 双平面几何：Y stride ≥ width；UV stride ≥ width；UV offset ≥ Y 平面末尾；
    /// 要求字节数 = UV offset + UV stride × ceil(height/2)。
    fn validate_nv12(&self, w: u64, h: u64) -> Result<u64, DecodeError> {
        let y = self.planes[0];
        let uv = self.planes[1];
        let y_stride = u64::from(y.stride);
        let uv_stride = u64::from(uv.stride);
        if y_stride < w {
            return Err(DecodeError::PlaneStrideTooSmall {
                plane: 0,
                min: w,
                found: y.stride,
            });
        }
        if uv_stride < w {
            return Err(DecodeError::PlaneStrideTooSmall {
                plane: 1,
                min: w,
                found: uv.stride,
            });
        }
        let y_end = checked_plane_end(y.offset, y_stride, h, 0)?;
        if u64::from(uv.offset) < y_end {
            return Err(DecodeError::PlaneOrdering {
                plane: 1,
                min_offset: y_end,
                found: uv.offset,
            });
        }
        let chroma_h = h.div_ceil(2);
        let uv_end = checked_plane_end(uv.offset, uv_stride, chroma_h, 1)?;
        Ok(y_end.max(uv_end))
    }

    /// BGRA 单平面几何：stride ≥ width×4；要求字节数 = offset + stride × height。
    fn validate_bgra(&self, w: u64, h: u64) -> Result<u64, DecodeError> {
        let p = self.planes[0];
        let min_stride = w
            .checked_mul(4)
            .ok_or(DecodeError::PlaneOverflow { plane: 0 })?;
        let stride = u64::from(p.stride);
        if stride < min_stride {
            return Err(DecodeError::PlaneStrideTooSmall {
                plane: 0,
                min: min_stride,
                found: p.stride,
            });
        }
        checked_plane_end(p.offset, stride, h, 0)
    }

    /// 编码为 96 字节小端头。先做完整校验——非法封装拒绝编码。
    ///
    /// # Errors
    /// [`FrameEnvelope::validate`] 的全部错误；payload 长度超 `u32` 时按
    /// [`DecodeError::PayloadTooShort`] 拒绝（不可能表达即非法）。
    pub fn encode_header(&self) -> Result<[u8; HEADER_SIZE_V1], DecodeError> {
        self.validate()?;
        let mut b = [0u8; HEADER_SIZE_V1];
        b[0..4].copy_from_slice(&MAGIC);
        b[4..6].copy_from_slice(&HEADER_VERSION_V1.to_le_bytes());
        // b[6..8] flags 保持 0（v1 保留）
        b[8..12].copy_from_slice(&(HEADER_SIZE_V1 as u32).to_le_bytes());
        b[12..16].copy_from_slice(&self.payload_len.to_le_bytes());
        b[16..24].copy_from_slice(&self.seq.to_le_bytes());
        b[24..32].copy_from_slice(&self.generation.to_le_bytes());
        b[32..40].copy_from_slice(&self.pts_ns.to_le_bytes());
        b[40..48].copy_from_slice(&self.duration_ns.to_le_bytes());
        b[48..50].copy_from_slice(&self.format.code().to_le_bytes());
        b[50..52].copy_from_slice(&self.format.plane_count().to_le_bytes());
        b[52..54].copy_from_slice(&self.width.to_le_bytes());
        b[54..56].copy_from_slice(&self.height.to_le_bytes());
        b[56..64].copy_from_slice(&self.graph_version.to_le_bytes());
        for (slot, plane) in self.planes.iter().enumerate() {
            let base = PLANE_TABLE_OFFSET + slot * PLANE_ENTRY_SIZE;
            b[base..base + 4].copy_from_slice(&plane.stride.to_le_bytes());
            b[base + 4..base + 8].copy_from_slice(&plane.offset.to_le_bytes());
        }
        Ok(b)
    }

    /// 从字节切片解码并做完整校验（结构 + 语义 + 几何）。
    ///
    /// # Errors
    /// 头部结构不符（魔数/版本/flags/长度）、语义非法（格式/平面数/尺寸）、
    /// 或几何校验失败时返回对应 [`DecodeError`]。
    pub fn decode_header(bytes: &[u8]) -> Result<Self, DecodeError> {
        if bytes.len() < HEADER_SIZE_V1 {
            return Err(DecodeError::Truncated {
                needed: HEADER_SIZE_V1,
                got: bytes.len(),
            });
        }
        let rd16 = |off: usize| u16::from_le_bytes([bytes[off], bytes[off + 1]]);
        let rd32 = |off: usize| {
            u32::from_le_bytes([bytes[off], bytes[off + 1], bytes[off + 2], bytes[off + 3]])
        };
        let rd64 = |off: usize| {
            let mut b = [0u8; 8];
            b.copy_from_slice(&bytes[off..off + 8]);
            u64::from_le_bytes(b)
        };
        if bytes[0..4] != MAGIC {
            return Err(DecodeError::InvalidMagic);
        }
        let version = rd16(4);
        if version != HEADER_VERSION_V1 {
            return Err(DecodeError::UnsupportedVersion { found: version });
        }
        let flags = rd16(6);
        if flags != 0 {
            return Err(DecodeError::BadFlags { found: flags });
        }
        let header_size = rd32(8);
        if usize::try_from(header_size) != Ok(HEADER_SIZE_V1) {
            return Err(DecodeError::BadHeaderSize { found: header_size });
        }
        let format = EnvelopePixelFormat::from_code(rd16(48))
            .ok_or(DecodeError::UnknownPixelFormat { code: rd16(48) })?;
        let plane_count = rd16(50);
        let expected_planes = format.plane_count();
        if plane_count != expected_planes {
            return Err(DecodeError::InvalidPlaneCount {
                expected: expected_planes,
                found: plane_count,
            });
        }
        let mut planes = [PlaneLayout::ZERO; MAX_PLANES];
        for (slot, plane) in planes.iter_mut().enumerate() {
            let base = PLANE_TABLE_OFFSET + slot * PLANE_ENTRY_SIZE;
            *plane = PlaneLayout {
                stride: rd32(base),
                offset: rd32(base + 4),
            };
        }
        let env = Self {
            seq: rd64(16),
            generation: rd64(24),
            pts_ns: rd64(32),
            duration_ns: rd64(40),
            graph_version: rd64(56),
            format,
            width: rd16(52),
            height: rd16(54),
            planes,
            payload_len: rd32(12),
        };
        env.validate()?;
        Ok(env)
    }
}

/// 单平面末尾（offset + stride×plane_height），全程 checked；溢出按契约拒绝。
fn checked_plane_end(
    offset: u32,
    stride: u64,
    plane_height: u64,
    plane: usize,
) -> Result<u64, DecodeError> {
    let size = stride
        .checked_mul(plane_height)
        .ok_or(DecodeError::PlaneOverflow { plane })?;
    u64::from(offset)
        .checked_add(size)
        .ok_or(DecodeError::PlaneOverflow { plane })
}

#[cfg(test)]
mod tests {
    use super::{
        error_kind, DecodeError, EnvelopePixelFormat, FrameEnvelope, PlaneLayout, HEADER_SIZE_V1,
        MAGIC, MAX_PLANES,
    };

    /// M1-T2 实测布局锚点：NV12 1280x720@30，stride=[1280,1280]、offsets=[0,921600]、
    /// payload=1,382,400（contracts/media/README.md §caps 决策记录）。
    fn canonical_nv12() -> FrameEnvelope {
        FrameEnvelope {
            seq: 1,
            generation: 1,
            pts_ns: 33_333_333,
            duration_ns: 33_333_333,
            graph_version: 1,
            format: EnvelopePixelFormat::Nv12,
            width: 1280,
            height: 720,
            planes: [
                PlaneLayout {
                    stride: 1280,
                    offset: 0,
                },
                PlaneLayout {
                    stride: 1280,
                    offset: 921_600,
                },
                PlaneLayout::ZERO,
                PlaneLayout::ZERO,
            ],
            payload_len: 1_382_400,
        }
    }

    /// BGRA 720p30：stride = 1280×4 = 5120，单平面，payload = 3,686,400。
    fn canonical_bgra() -> FrameEnvelope {
        FrameEnvelope {
            format: EnvelopePixelFormat::Bgra,
            planes: [
                PlaneLayout {
                    stride: 5120,
                    offset: 0,
                },
                PlaneLayout::ZERO,
                PlaneLayout::ZERO,
                PlaneLayout::ZERO,
            ],
            payload_len: 3_686_400,
            ..canonical_nv12()
        }
    }

    #[test]
    fn m1t2_measured_nv12_layout_is_canonical() {
        let env = canonical_nv12();
        let required = env
            .validate()
            .unwrap_or_else(|e| panic!("canonical nv12 must validate: {e}"));
        assert_eq!(required, 1_382_400);
        // M1-T2 T4 公式核对法：UV offset = Y stride × 高；总长 = UV offset + UV stride × 高/2。
        assert_eq!(
            env.planes[1].offset as u64,
            u64::from(env.planes[0].stride) * 720
        );
        assert_eq!(
            1_382_400_u64,
            921_600 + u64::from(env.planes[1].stride) * 360
        );
    }

    #[test]
    fn roundtrip_nv12_preserves_all_fields() {
        let env = canonical_nv12();
        let bytes = env
            .encode_header()
            .unwrap_or_else(|e| panic!("encode failed: {e}"));
        assert_eq!(bytes.len(), HEADER_SIZE_V1);
        let back =
            FrameEnvelope::decode_header(&bytes).unwrap_or_else(|e| panic!("decode failed: {e}"));
        assert_eq!(back, env);
    }

    #[test]
    fn roundtrip_bgra_preserves_all_fields() {
        let env = canonical_bgra();
        let bytes = env
            .encode_header()
            .unwrap_or_else(|e| panic!("encode failed: {e}"));
        let back =
            FrameEnvelope::decode_header(&bytes).unwrap_or_else(|e| panic!("decode failed: {e}"));
        assert_eq!(back, env);
    }

    /// 期望解码被拒：返回错误本体（否则 panic）。
    fn decode_err(bytes: &[u8]) -> DecodeError {
        match FrameEnvelope::decode_header(bytes) {
            Ok(_) => panic!("decode must be rejected for this input"),
            Err(e) => e,
        }
    }

    /// 期望校验被拒：返回错误本体（否则 panic）。
    fn validate_err(env: &FrameEnvelope) -> DecodeError {
        match env.validate() {
            Ok(_) => panic!("validate must be rejected for this envelope"),
            Err(e) => e,
        }
    }

    /// 编码 canonical 头并补丁指定字节区间（用于编码期校验会拒绝的非法输入）。
    fn patched(range: std::ops::Range<usize>, value: &[u8]) -> [u8; HEADER_SIZE_V1] {
        let mut bytes = canonical_nv12()
            .encode_header()
            .unwrap_or_else(|e| panic!("fixture encode failed: {e}"));
        bytes[range].copy_from_slice(value);
        bytes
    }

    #[test]
    fn canonical_header_bytes_match_layout_spec() {
        let bytes = canonical_nv12()
            .encode_header()
            .unwrap_or_else(|e| panic!("encode failed: {e}"));
        assert_eq!(bytes.len(), HEADER_SIZE_V1);
        assert_eq!(&bytes[0..4], &MAGIC);
        assert_eq!(&bytes[4..6], &1u16.to_le_bytes()); // version
        assert_eq!(&bytes[6..8], &[0, 0]); // flags（v1 保留）
        assert_eq!(&bytes[8..12], &96u32.to_le_bytes()); // header_size
        assert_eq!(&bytes[12..16], &1_382_400u32.to_le_bytes()); // payload_len
        assert_eq!(&bytes[16..24], &1u64.to_le_bytes()); // seq
        assert_eq!(&bytes[32..40], &33_333_333u64.to_le_bytes()); // pts_ns
        assert_eq!(&bytes[48..50], &1u16.to_le_bytes()); // NV12
        assert_eq!(&bytes[50..52], &2u16.to_le_bytes()); // plane_count
        assert_eq!(&bytes[52..54], &1280u16.to_le_bytes()); // width
        assert_eq!(&bytes[54..56], &720u16.to_le_bytes()); // height
        assert_eq!(&bytes[64..68], &1280u32.to_le_bytes()); // plane0 stride
        assert_eq!(&bytes[68..72], &0u32.to_le_bytes()); // plane0 offset
        assert_eq!(&bytes[72..76], &1280u32.to_le_bytes()); // plane1 stride
        assert_eq!(&bytes[76..80], &921_600u32.to_le_bytes()); // plane1 offset
        assert!(bytes[80..96].iter().all(|&b| b == 0)); // 未用槽全零
    }

    #[test]
    fn truncated_input_rejected() {
        let bytes = canonical_nv12()
            .encode_header()
            .unwrap_or_else(|e| panic!("encode failed: {e}"));
        let err = decode_err(&bytes[..HEADER_SIZE_V1 - 1]);
        assert_eq!(error_kind(&err), "Truncated");
        match err {
            DecodeError::Truncated { needed, got } => assert_eq!((needed, got), (96, 95)),
            other => panic!("wrong variant: {other:?}"),
        }
    }

    #[test]
    fn bad_magic_rejected() {
        let bytes = patched(0..4, b"XXXX");
        assert_eq!(error_kind(&decode_err(&bytes)), "InvalidMagic");
    }

    #[test]
    fn unsupported_version_rejected() {
        let bytes = patched(4..6, &2u16.to_le_bytes());
        let err = decode_err(&bytes);
        assert_eq!(error_kind(&err), "UnsupportedVersion");
        match err {
            DecodeError::UnsupportedVersion { found } => assert_eq!(found, 2),
            other => panic!("wrong variant: {other:?}"),
        }
    }

    #[test]
    fn bad_flags_rejected() {
        let bytes = patched(6..8, &1u16.to_le_bytes());
        assert_eq!(error_kind(&decode_err(&bytes)), "BadFlags");
    }

    #[test]
    fn bad_header_size_rejected() {
        let bytes = patched(8..12, &97u32.to_le_bytes());
        assert_eq!(error_kind(&decode_err(&bytes)), "BadHeaderSize");
    }

    #[test]
    fn unknown_pixel_format_rejected() {
        let bytes = patched(48..50, &9u16.to_le_bytes());
        let err = decode_err(&bytes);
        assert_eq!(error_kind(&err), "UnknownPixelFormat");
    }

    #[test]
    fn wrong_plane_count_rejected() {
        let bytes = patched(50..52, &1u16.to_le_bytes());
        let err = decode_err(&bytes);
        assert_eq!(error_kind(&err), "InvalidPlaneCount");
        match err {
            DecodeError::InvalidPlaneCount { expected, found } => {
                assert_eq!((expected, found), (2, 1));
            }
            other => panic!("wrong variant: {other:?}"),
        }
    }

    #[test]
    fn zero_dimensions_rejected() {
        let mut env = canonical_nv12();
        env.width = 0;
        assert_eq!(error_kind(&validate_err(&env)), "InvalidDimensions");
    }

    #[test]
    fn trailing_plane_slot_nonzero_rejected() {
        let mut env = canonical_nv12();
        env.planes[2] = PlaneLayout {
            stride: 1,
            offset: 0,
        };
        let err = validate_err(&env);
        assert_eq!(error_kind(&err), "TrailingPlaneDataNotZero");
        match err {
            DecodeError::TrailingPlaneDataNotZero { plane } => assert_eq!(plane, 2),
            other => panic!("wrong variant: {other:?}"),
        }
    }

    #[test]
    fn nv12_stride_below_width_rejected_via_decode() {
        // plane0 stride 1280 → 800（< width；非法输入无法经 encode 产生，直接补字节）
        let bytes = patched(64..68, &800u32.to_le_bytes());
        let err = decode_err(&bytes);
        assert_eq!(error_kind(&err), "PlaneStrideTooSmall");
        match err {
            DecodeError::PlaneStrideTooSmall { plane, min, found } => {
                assert_eq!((plane, min, found), (0, 1280, 800));
            }
            other => panic!("wrong variant: {other:?}"),
        }
    }

    #[test]
    fn nv12_uv_offset_overlapping_y_rejected() {
        let mut env = canonical_nv12();
        env.planes[1].offset = 1024; // < Y 平面末尾 921600
        let err = validate_err(&env);
        assert_eq!(error_kind(&err), "PlaneOrdering");
        match err {
            DecodeError::PlaneOrdering {
                plane,
                min_offset,
                found,
            } => {
                assert_eq!((plane, min_offset, found), (1, 921_600, 1024));
            }
            other => panic!("wrong variant: {other:?}"),
        }
    }

    #[test]
    fn payload_below_layout_requirement_rejected() {
        let mut env = canonical_nv12();
        env.payload_len = 1_000;
        let err = validate_err(&env);
        assert_eq!(error_kind(&err), "PayloadTooShort");
        match err {
            DecodeError::PayloadTooShort { required, actual } => {
                assert_eq!((required, actual), (1_382_400, 1_000));
            }
            other => panic!("wrong variant: {other:?}"),
        }
    }

    #[test]
    fn extreme_offset_lands_on_payload_too_short() {
        // offset 超界（u32::MAX）在 v1 字段宽度下不触发 u64 溢出，
        // 契约出口是 PayloadTooShort（required 超过声明 payload_len）。
        let mut env = canonical_bgra();
        env.planes[0].offset = u32::MAX;
        assert_eq!(error_kind(&validate_err(&env)), "PayloadTooShort");
    }

    #[test]
    fn bgra_stride_below_4bpp_minimum_rejected() {
        let mut env = canonical_bgra();
        env.planes[0].stride = 100;
        let err = validate_err(&env);
        assert_eq!(error_kind(&err), "PlaneStrideTooSmall");
        match err {
            DecodeError::PlaneStrideTooSmall { plane, min, .. } => {
                assert_eq!((plane, min), (0, 5_120));
            }
            other => panic!("wrong variant: {other:?}"),
        }
    }

    #[test]
    fn error_kind_names_are_stable() {
        let names = [
            (DecodeError::InvalidMagic, "InvalidMagic"),
            (DecodeError::Truncated { needed: 96, got: 0 }, "Truncated"),
            (DecodeError::BadFlags { found: 1 }, "BadFlags"),
            (DecodeError::BadHeaderSize { found: 0 }, "BadHeaderSize"),
            (
                DecodeError::UnsupportedVersion { found: 9 },
                "UnsupportedVersion",
            ),
            (
                DecodeError::UnknownPixelFormat { code: 7 },
                "UnknownPixelFormat",
            ),
            (
                DecodeError::InvalidPlaneCount {
                    expected: 2,
                    found: 3,
                },
                "InvalidPlaneCount",
            ),
            (
                DecodeError::InvalidDimensions {
                    width: 0,
                    height: 0,
                },
                "InvalidDimensions",
            ),
            (
                DecodeError::TrailingPlaneDataNotZero { plane: 3 },
                "TrailingPlaneDataNotZero",
            ),
            (
                DecodeError::PlaneStrideTooSmall {
                    plane: 0,
                    min: 1,
                    found: 0,
                },
                "PlaneStrideTooSmall",
            ),
            (
                DecodeError::PlaneOrdering {
                    plane: 1,
                    min_offset: 1,
                    found: 0,
                },
                "PlaneOrdering",
            ),
            (DecodeError::PlaneOverflow { plane: 0 }, "PlaneOverflow"),
            (
                DecodeError::PayloadTooShort {
                    required: 1,
                    actual: 0,
                },
                "PayloadTooShort",
            ),
        ];
        for (err, name) in names {
            assert_eq!(error_kind(&err), name);
        }
    }

    #[test]
    fn planes_table_has_four_slots() {
        assert_eq!(MAX_PLANES, 4);
        assert_eq!(canonical_nv12().planes.len(), MAX_PLANES);
    }
}
