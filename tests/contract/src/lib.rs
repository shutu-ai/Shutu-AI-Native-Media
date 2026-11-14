//! FrameEnvelope v1 + 有界帧缓冲契约测试（M1-T3）。
//!
//! 金样本：`golden/frame_envelope_v1.txt`——由纯 Python `struct.pack`
//! 独立推导生成（provenance：run `G1-20261114-104836-24c7/golden_gen.py`），
//! 与被测 Rust 实现**零共享代码**；本 crate 内置手写解析器消费之。
//! 该文件是 Rust ↔ 未来 C++ 消费端（M1-T4/T5）的共享契约锚点。
//!
//! 边界（AGENTS §4）：本 crate 只测传输无关语义（编解码 + 缓冲契约）；
//! 跨进程传输测试归 M1-T4/T5，**不得在此以 Mock 假装已验证**。

/// 金样本文件（构建期内嵌；变更即触发重编译）。
#[cfg(test)]
const GOLDEN: &str = include_str!("../golden/frame_envelope_v1.txt");

#[cfg(test)]
mod support {
    /// 单条金样本向量。
    #[derive(Debug)]
    pub(crate) struct GoldenVector {
        /// 向量名（文件内唯一）。
        pub(crate) name: String,
        /// ERROR 向量的期望错误类别（KIND 稳定名）；VALID 向量为 `None`。
        pub(crate) expect_kind: Option<String>,
        /// 连接后的 HEX 载荷。
        pub(crate) hex: String,
        /// FIELDS 行原文（VALID 向量）。
        pub(crate) fields: Option<String>,
    }

    /// 手写行格式解析器：`VALID/ERROR/HEX/FIELDS/END`，`#` 注释与空行跳过。
    pub(crate) fn parse_golden(text: &str) -> Vec<GoldenVector> {
        let mut out: Vec<GoldenVector> = Vec::new();
        let mut current: Option<GoldenVector> = None;
        for (idx, line) in text.lines().enumerate() {
            let line = line.trim_end();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some(rest) = line.strip_prefix("VALID ") {
                if current.is_some() {
                    panic!("golden line {}: vector not closed by END", idx + 1);
                }
                current = Some(GoldenVector {
                    name: rest.trim().to_string(),
                    expect_kind: None,
                    hex: String::new(),
                    fields: None,
                });
            } else if let Some(rest) = line.strip_prefix("ERROR ") {
                if current.is_some() {
                    panic!("golden line {}: vector not closed by END", idx + 1);
                }
                let mut parts = rest.split_whitespace();
                let name = parts.next().unwrap_or_else(|| {
                    panic!("golden line {}: ERROR vector missing name", idx + 1)
                });
                let kind = parts.next().unwrap_or_else(|| {
                    panic!("golden line {}: ERROR vector missing KIND", idx + 1)
                });
                current = Some(GoldenVector {
                    name: name.to_string(),
                    expect_kind: Some(kind.to_string()),
                    hex: String::new(),
                    fields: None,
                });
            } else if let Some(rest) = line.strip_prefix("HEX ") {
                match current.as_mut() {
                    Some(v) => v.hex.push_str(rest.trim()),
                    None => panic!("golden line {}: HEX outside vector", idx + 1),
                }
            } else if let Some(rest) = line.strip_prefix("FIELDS ") {
                match current.as_mut() {
                    Some(v) => v.fields = Some(rest.trim().to_string()),
                    None => panic!("golden line {}: FIELDS outside vector", idx + 1),
                }
            } else if line == "END" {
                match current.take() {
                    Some(v) => out.push(v),
                    None => panic!("golden line {}: END without open vector", idx + 1),
                }
            } else {
                panic!("golden line {}: unrecognized line: {line}", idx + 1);
            }
        }
        if current.is_some() {
            panic!("golden file ends with an unclosed vector");
        }
        out
    }

    /// 十六进制 → 字节（奇数长度即坏文件，断言失败报告）。
    pub(crate) fn hex_to_bytes(hex: &str) -> Vec<u8> {
        assert_eq!(hex.len() % 2, 0, "odd-length hex in vector payload");
        (0..hex.len())
            .step_by(2)
            .map(|i| {
                u8::from_str_radix(&hex[i..i + 2], 16)
                    .unwrap_or_else(|e| panic!("bad hex byte at {i}: {e}"))
            })
            .collect()
    }

    /// 断言解码结果的字段与 FIELDS 行一致。
    pub(crate) fn assert_fields(env: &FrameEnvelope, fields: &str) {
        let expected: Vec<(&str, &str)> = fields
            .split(',')
            .filter_map(|kv| {
                let (k, v) = kv.split_once('=')?;
                Some((k.trim(), v.trim()))
            })
            .collect();
        let format_name = match env.format {
            EnvelopePixelFormat::Nv12 => "Nv12",
            EnvelopePixelFormat::Bgra => "Bgra",
        };
        let actual: Vec<(&str, String)> = vec![
            ("seq", env.seq.to_string()),
            ("generation", env.generation.to_string()),
            ("pts_ns", env.pts_ns.to_string()),
            ("duration_ns", env.duration_ns.to_string()),
            ("graph_version", env.graph_version.to_string()),
            ("format", format_name.to_string()),
            ("width", env.width.to_string()),
            ("height", env.height.to_string()),
            ("payload_len", env.payload_len.to_string()),
        ];
        for (k, v) in &expected {
            let got = actual
                .iter()
                .find(|(ak, _)| ak == k)
                .map(|(_, av)| av.as_str())
                .unwrap_or_else(|| panic!("FIELDS key {k} not comparable"));
            assert_eq!(got, *v, "field {k} mismatch");
        }
        assert_eq!(
            expected.len(),
            actual.len(),
            "FIELDS key count must cover all asserted fields"
        );
    }

    /// 解码必成（否则 panic 带诊断）。
    pub(crate) fn decode_ok(bytes: &[u8]) -> FrameEnvelope {
        match FrameEnvelope::decode_header(bytes) {
            Ok(env) => env,
            Err(e) => panic!("expected successful decode, got: {e}"),
        }
    }

    use media_core::envelope::{EnvelopePixelFormat, FrameEnvelope};
}

#[cfg(test)]
mod golden_tests {
    use crate::support::{decode_ok, hex_to_bytes, parse_golden};
    use crate::GOLDEN;

    use media_core::envelope::{
        error_kind, FrameEnvelope, PlaneLayout, HEADER_SIZE_V1, MAX_PLANES,
    };

    type GoldenVector = crate::support::GoldenVector;

    fn vectors() -> Vec<GoldenVector> {
        parse_golden(GOLDEN)
    }

    fn find<'a>(vs: &'a [GoldenVector], name: &str) -> &'a GoldenVector {
        vs.iter()
            .find(|v| v.name == name)
            .unwrap_or_else(|| panic!("vector {name} missing from golden file"))
    }

    #[test]
    fn golden_file_has_expected_shape() {
        let vs = vectors();
        let valid = vs.iter().filter(|v| v.expect_kind.is_none()).count();
        let errors = vs.iter().filter(|v| v.expect_kind.is_some()).count();
        assert_eq!(valid, 4, "4 VALID vectors");
        assert_eq!(errors, 13, "13 ERROR vectors");
        let mut names: Vec<&str> = vs.iter().map(|v| v.name.as_str()).collect();
        names.sort_unstable();
        let before = names.len();
        names.dedup();
        assert_eq!(names.len(), before, "vector names must be unique");
    }

    #[test]
    fn valid_vectors_decode_roundtrip_and_match_fields() {
        for v in vectors().into_iter().filter(|v| v.expect_kind.is_none()) {
            let bytes = hex_to_bytes(&v.hex);
            assert_eq!(
                bytes.len(),
                HEADER_SIZE_V1,
                "vector {} must carry exactly one 96-byte header",
                v.name
            );
            let env = decode_ok(&bytes);
            let fields = v
                .fields
                .as_deref()
                .unwrap_or_else(|| panic!("VALID vector {} missing FIELDS", v.name));
            crate::support::assert_fields(&env, fields);
            // 重编码必须逐字节一致（编解码互逆）。
            let reencoded = env
                .encode_header()
                .unwrap_or_else(|e| panic!("re-encode of {} failed: {e}", v.name));
            assert_eq!(
                reencoded.as_slice(),
                bytes.as_slice(),
                "roundtrip {}",
                v.name
            );
        }
    }

    #[test]
    fn error_vectors_rejected_with_expected_kind() {
        for v in vectors().into_iter().filter(|v| v.expect_kind.is_some()) {
            let bytes = hex_to_bytes(&v.hex);
            let kind = match FrameEnvelope::decode_header(&bytes) {
                Ok(_) => panic!("ERROR vector {} must be rejected", v.name),
                Err(e) => error_kind(&e).to_string(),
            };
            assert_eq!(
                kind,
                v.expect_kind.as_deref().unwrap_or_default(),
                "vector {} error kind mismatch",
                v.name
            );
        }
    }

    #[test]
    fn truncated_vector_carries_partial_bytes() {
        let vs = vectors();
        let v = find(&vs, "truncated_94");
        assert_eq!(hex_to_bytes(&v.hex).len(), HEADER_SIZE_V1 - 2);
    }

    #[test]
    fn m1t2_measured_layout_anchor_via_golden() {
        // contracts/media/README.md §caps：M1-T2 实测 NV12 布局必须作为断言锚点。
        let env = decode_ok(&hex_to_bytes(
            &find(&vectors(), "nv12_720p30_canonical").hex,
        ));
        assert_eq!(env.width, 1280);
        assert_eq!(env.height, 720);
        assert_eq!(env.payload_len, 1_382_400);
        assert_eq!(env.planes[0].stride, 1280);
        assert_eq!(env.planes[0].offset, 0);
        assert_eq!(env.planes[1].stride, 1280);
        assert_eq!(env.planes[1].offset, 921_600);
        assert_eq!(env.planes.len(), MAX_PLANES);
        for slot in env.planes.iter().skip(2) {
            assert_eq!(*slot, PlaneLayout::ZERO);
        }
    }
}

#[cfg(test)]
mod buffer_contract_tests {
    use crate::support::{decode_ok, hex_to_bytes, parse_golden};
    use crate::GOLDEN;

    use device_bridge::framebuffer::{
        BoundedFrameBuffer, BufferLimits, FrameBuffer, OverfullPolicy, PushError, ReadError,
    };
    use media_core::envelope::{error_kind, EnvelopePixelFormat, FrameEnvelope, MAX_PLANES};

    /// 从金样本 VALID 向量构造可 push 的封装（改写 seq/generation 供缓冲用例）。
    fn valid_env(name: &str, seq: u64, generation: u64) -> FrameEnvelope {
        let vs = parse_golden(GOLDEN);
        let mut env = decode_ok(&hex_to_bytes(
            &vs.iter()
                .find(|v| v.name == name && v.expect_kind.is_none())
                .unwrap_or_else(|| panic!("vector {name} missing"))
                .hex,
        ));
        assert_eq!(env.planes.len(), MAX_PLANES);
        env.seq = seq;
        env.generation = generation;
        env
    }

    fn push_ok(buf: &BoundedFrameBuffer, env: FrameEnvelope) {
        buf.push(env)
            .unwrap_or_else(|e| panic!("push must succeed: {e}"));
    }

    fn push_err(buf: &BoundedFrameBuffer, env: FrameEnvelope) -> PushError {
        buf.push(env)
            .err()
            .unwrap_or_else(|| panic!("push must be rejected"))
    }

    #[test]
    fn golden_valid_envelopes_accepted() {
        // 全部 VALID 向量（含 payload 余量与 1920 对齐 stride 变体）均可入缓冲。
        let buf = BoundedFrameBuffer::new(
            BufferLimits {
                max_frames: 16,
                max_bytes: 64 * 1024 * 1024,
            },
            OverfullPolicy::RejectNew,
        );
        for (idx, name) in [
            "nv12_720p30_canonical",
            "bgra_720p30_canonical",
            "nv12_payload_margin",
            "nv12_padded_stride_1920",
        ]
        .into_iter()
        .enumerate()
        {
            // 同代际互异序号（枚举下标 +1：非零且单调）。
            let env = valid_env(name, idx as u64 + 1, 1);
            push_ok(&buf, env);
        }
        assert_eq!(buf.levels().frames, 4);
    }

    #[test]
    fn replay_rejected_and_generation_advance_resets_seq() {
        let buf = BoundedFrameBuffer::with_defaults();
        push_ok(&buf, valid_env("bgra_720p30_canonical", 1, 1));
        push_ok(&buf, valid_env("bgra_720p30_canonical", 2, 1));
        match push_err(&buf, valid_env("bgra_720p30_canonical", 2, 1)) {
            PushError::SeqReplay { got, .. } => assert_eq!(got, 2),
            other => panic!("wrong variant: {other:?}"),
        }
        // 新代际允许序号回到 1；旧代际随后被拒。
        push_ok(&buf, valid_env("bgra_720p30_canonical", 1, 2));
        match push_err(&buf, valid_env("bgra_720p30_canonical", 2, 1)) {
            PushError::GenerationRejected { got, .. } => assert_eq!(got, 1),
            other => panic!("wrong variant: {other:?}"),
        }
    }

    #[test]
    fn stale_generation_read_rejected_and_latest_matches() {
        let buf = BoundedFrameBuffer::with_defaults();
        push_ok(&buf, valid_env("nv12_720p30_canonical", 1, 5));
        match buf.read_latest(4) {
            Err(ReadError::StaleGeneration { latest, requested }) => {
                assert_eq!((latest, requested), (5, 4));
            }
            other => panic!("expected StaleGeneration, got {other:?}"),
        }
        let latest = buf
            .read_latest(5)
            .unwrap_or_else(|e| panic!("read_latest(5) must succeed: {e}"));
        assert_eq!(latest.generation, 5);
        assert_eq!(latest.seq, 1);
    }

    #[test]
    fn reject_new_when_frames_full() {
        let buf = BoundedFrameBuffer::new(
            BufferLimits {
                max_frames: 2,
                max_bytes: 8 * 1024 * 1024,
            },
            OverfullPolicy::RejectNew,
        );
        push_ok(&buf, valid_env("bgra_720p30_canonical", 1, 1));
        push_ok(&buf, valid_env("bgra_720p30_canonical", 2, 1));
        match push_err(&buf, valid_env("bgra_720p30_canonical", 3, 1)) {
            PushError::TooManyFrames { limit } => assert_eq!(limit, 2),
            other => panic!("wrong variant: {other:?}"),
        }
    }

    #[test]
    fn drop_oldest_evicts_and_keeps_fresh_frame() {
        let buf = BoundedFrameBuffer::new(
            BufferLimits {
                max_frames: 2,
                max_bytes: 8 * 1024 * 1024,
            },
            OverfullPolicy::DropOldest,
        );
        push_ok(&buf, valid_env("bgra_720p30_canonical", 1, 1));
        push_ok(&buf, valid_env("bgra_720p30_canonical", 2, 1));
        push_ok(&buf, valid_env("bgra_720p30_canonical", 3, 1));
        assert_eq!(buf.levels().frames, 2);
        let latest = buf
            .read_latest(1)
            .unwrap_or_else(|e| panic!("read_latest must succeed: {e}"));
        assert_eq!(latest.seq, 3);
    }

    #[test]
    fn error_vector_bytes_rejected_at_decode_and_push() {
        // ERROR 向量的字节在 decode 层被拒（带正确 KIND）；
        // 同几何的非法封装即使绕过 decode 也进不了缓冲（push 内部再校验）。
        let vs = parse_golden(GOLDEN);
        let bad = vs
            .iter()
            .find(|v| v.name == "stride_too_small_800")
            .unwrap_or_else(|| panic!("vector stride_too_small_800 missing"));
        let bytes = hex_to_bytes(&bad.hex);
        match FrameEnvelope::decode_header(&bytes) {
            Err(e) => assert_eq!(error_kind(&e), "PlaneStrideTooSmall"),
            Ok(_) => panic!("fixture stride_too_small_800 must be invalid"),
        }
        let buf = BoundedFrameBuffer::with_defaults();
        let mut env = valid_env("bgra_720p30_canonical", 1, 1);
        env.width = 0;
        match push_err(&buf, env) {
            PushError::InvalidEnvelope(_) => {}
            other => panic!("wrong variant: {other:?}"),
        }
        assert_eq!(buf.levels().frames, 0);
    }

    #[test]
    fn nv12_and_bgra_coexist_in_one_buffer() {
        let buf = BoundedFrameBuffer::new(
            BufferLimits {
                max_frames: 8,
                max_bytes: 64 * 1024 * 1024,
            },
            OverfullPolicy::RejectNew,
        );
        push_ok(&buf, valid_env("nv12_720p30_canonical", 1, 1));
        push_ok(&buf, valid_env("bgra_720p30_canonical", 2, 1));
        let latest = buf
            .read_latest(1)
            .unwrap_or_else(|e| panic!("read_latest must succeed: {e}"));
        assert_eq!(latest.format, EnvelopePixelFormat::Bgra);
        assert_eq!(buf.levels().bytes, 1_382_400 + 3_686_400);
    }
}
