//! Module-private wire codec for prepared lightning plan pieces (D3).
//!
//! A prepared piece is one layer of the kernel plan, named
//! `lightning-layer-{global_layer_index}`. The bytes are module-local: no host
//! code decodes them, discovers views by name or receives them serialized, so
//! the format is this module's private contract rather than a shared schema.
//!
//! Wire layout (little-endian, fixed width, no float on the wire):
//!
//! ```text
//! header: b"LTPL", u16 format version, u32 entry count
//! entry:  u32 object-id length + UTF-8
//!         u64 region id
//!         u32 global layer index
//!         u32 variant-chain count + per element (u32 key length + UTF-8,
//!             u8 paint tag, tag payload)
//!         u32 edge count + per edge four i64 coordinates in units (1 unit =
//!             100 nm, docs/08)
//! ```
//!
//! Coordinates are integer units, never millimetres, so a piece round-trips
//! bit-exactly on any host. `PaintValue::Scalar` is the one painted value that
//! carries a float; it is written as `f32::to_bits()` (a `u32`, not a float
//! field) because `PaintValue`'s own equality is bit-based, so a bit-exact
//! encoding is what "identity round-trip" means here.
//!
//! Decode failures are this module's own scoped codes, in the same `574xx`
//! family as the row-03/05 fixture decoder convention: `57410` for a body this
//! decoder rejects, `57411` for a well-formed header carrying an unsupported
//! format version. The inherited missing-piece path stays outside this file:
//! `PreparedPlanView::require` owns `57401`.

use slicer_ir::{PaintValue, Point2};
use slicer_sdk::error::ModuleError;

use crate::kernel::{KernelPlan, PlanEntry};

/// Prefix of every piece this module publishes: `lightning-layer-{index}`.
///
/// Verified against the only name validation the preparation store performs
/// (an empty name is `InvalidName`; there is no charset rule), so the spelling D3
/// settles is accepted as written.
pub(crate) const PIECE_NAME_PREFIX: &str = "lightning-layer-";

/// Piece-body magic: the four bytes every well-formed piece starts with.
const MAGIC: [u8; 4] = *b"LTPL";

/// Wire format version this encoder writes and this decoder accepts.
const FORMAT_VERSION: u16 = 1;

/// This module's malformed-piece code (`57410`).
const MALFORMED_PIECE_CODE: u32 = 57410;

/// This module's unsupported-version code (`57411`).
const UNSUPPORTED_VERSION_CODE: u32 = 57411;

/// Paint tags on the wire. A new `PaintValue` variant needs a new tag, and an
/// unknown tag is malformed rather than silently dropped.
const TAG_FLAG: u8 = 0;
const TAG_SCALAR: u8 = 1;
const TAG_TOOL_INDEX: u8 = 2;
const TAG_CUSTOM: u8 = 3;

/// Smallest possible encoded entry: object-id length, region id, layer index,
/// chain count and edge count, each with an empty payload.
const MIN_ENTRY_BYTES: usize = 4 + 8 + 4 + 4 + 4;

/// Smallest possible chain element: key length plus paint tag, both empty.
const MIN_CHAIN_ELEMENT_BYTES: usize = 4 + 1;

/// One encoded edge: four `i64` coordinates.
const MIN_EDGE_BYTES: usize = 4 * 8;

/// Return the piece name for one global layer index.
pub(crate) fn piece_name(global_layer_index: i32) -> String {
    format!("{PIECE_NAME_PREFIX}{global_layer_index}")
}

/// Encode every plan entry on `global_layer_index` as one prepared piece body.
///
/// A layer with no entries encodes the empty plan (header only), which is what
/// the `empty_plan` diagnostic path publishes. Entries for other layers are not
/// silently folded in: the caller iterates the plan's own layer population.
///
/// The length fields are `u32` by contract. On the deployed target (`wasm32`)
/// `usize` is 32 bits, so the narrowing below cannot fail; on a 64-bit test host
/// it would take a single field larger than 4 GiB, which no committed input can
/// carry.
pub(crate) fn encode_layer_piece(plan: &KernelPlan, global_layer_index: i32) -> Vec<u8> {
    let entries = plan
        .entries
        .iter()
        .filter(|entry| entry.global_layer_index == global_layer_index);

    let mut body = Vec::new();
    body.extend_from_slice(&MAGIC);
    body.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
    // The count is written before the entries it describes, so the population
    // is materialised first; a piece is one layer, so this stays bounded by the
    // layer's own entry count.
    let layer_entries: Vec<&PlanEntry> = entries.collect();
    push_len(&mut body, layer_entries.len());
    for entry in layer_entries {
        push_len(&mut body, entry.object_id.len());
        body.extend_from_slice(entry.object_id.as_bytes());
        body.extend_from_slice(&entry.region_id.to_le_bytes());
        // Bit-preserving: the wire field is `u32` by D3, and `i32 as u32 as i32`
        // is the identity for every value, including negatives.
        body.extend_from_slice(&(entry.global_layer_index as u32).to_le_bytes());
        push_len(&mut body, entry.variant_chain.len());
        for (key, value) in &entry.variant_chain {
            push_len(&mut body, key.len());
            body.extend_from_slice(key.as_bytes());
            encode_paint_value(&mut body, value);
        }
        push_len(&mut body, entry.tree_edge_segments.len());
        for [start, end] in &entry.tree_edge_segments {
            for point in [start, end] {
                body.extend_from_slice(&point.x.to_le_bytes());
                body.extend_from_slice(&point.y.to_le_bytes());
            }
        }
    }
    body
}

/// Decode one prepared piece body, rejecting anything this decoder cannot read
/// exactly.
///
/// `name` is used only to name the piece in a malformed-body message; the
/// version failure names the version it read. Both failures are fatal, so a
/// corrupted plan never degrades into an empty layer.
pub(crate) fn decode_layer_piece(name: &str, bytes: &[u8]) -> Result<KernelPlan, ModuleError> {
    decode_body(bytes).map_err(|failure| match failure {
        DecodeFailure::Malformed => ModuleError::fatal(
            MALFORMED_PIECE_CODE,
            format!("malformed lightning plan piece: {name}"),
        ),
        DecodeFailure::UnsupportedVersion(version) => ModuleError::fatal(
            UNSUPPORTED_VERSION_CODE,
            format!("unsupported lightning plan piece version: {version}"),
        ),
    })
}

/// Why a body failed to decode, before it becomes a `ModuleError`.
enum DecodeFailure {
    /// Truncated, misaligned, over-long, unknown-tagged or trailing bytes.
    Malformed,
    /// A structurally valid header carrying a format version we do not read.
    UnsupportedVersion(u16),
}

/// Decode a body with a bounds check on every read.
fn decode_body(bytes: &[u8]) -> Result<KernelPlan, DecodeFailure> {
    let mut reader = Reader::new(bytes);
    if reader.take(MAGIC.len()) != Some(&MAGIC[..]) {
        return Err(DecodeFailure::Malformed);
    }
    let version = reader.u16().ok_or(DecodeFailure::Malformed)?;
    if version != FORMAT_VERSION {
        return Err(DecodeFailure::UnsupportedVersion(version));
    }

    let entry_count = reader.u32().ok_or(DecodeFailure::Malformed)?;
    let mut entries = Vec::with_capacity(bounded_capacity(
        entry_count,
        reader.remaining(),
        MIN_ENTRY_BYTES,
    ));
    for _ in 0..entry_count {
        let object_id = reader.string().ok_or(DecodeFailure::Malformed)?;
        let region_id = reader.u64().ok_or(DecodeFailure::Malformed)?;
        let global_layer_index = reader.u32().ok_or(DecodeFailure::Malformed)? as i32;

        let chain_count = reader.u32().ok_or(DecodeFailure::Malformed)?;
        let mut variant_chain = Vec::with_capacity(bounded_capacity(
            chain_count,
            reader.remaining(),
            MIN_CHAIN_ELEMENT_BYTES,
        ));
        for _ in 0..chain_count {
            let key = reader.string().ok_or(DecodeFailure::Malformed)?;
            let value = reader.paint_value().ok_or(DecodeFailure::Malformed)?;
            variant_chain.push((key, value));
        }

        let edge_count = reader.u32().ok_or(DecodeFailure::Malformed)?;
        let mut tree_edge_segments = Vec::with_capacity(bounded_capacity(
            edge_count,
            reader.remaining(),
            MIN_EDGE_BYTES,
        ));
        for _ in 0..edge_count {
            let start = reader.point().ok_or(DecodeFailure::Malformed)?;
            let end = reader.point().ok_or(DecodeFailure::Malformed)?;
            tree_edge_segments.push([start, end]);
        }

        entries.push(PlanEntry {
            object_id,
            global_layer_index,
            region_id,
            tree_edge_segments,
            variant_chain,
        });
    }

    // A piece is exactly one body: trailing bytes mean the writer and this
    // decoder disagree about the layout, which is a malformed body, not an
    // extension point (the version field is the extension point).
    if reader.remaining() != 0 {
        return Err(DecodeFailure::Malformed);
    }
    Ok(KernelPlan { entries })
}

// ============================================================================
// Encoding helpers
// ============================================================================

/// Append a `u32` length prefix.
fn push_len(body: &mut Vec<u8>, len: usize) {
    body.extend_from_slice(
        &u32::try_from(len)
            .expect("LTPL length fields are u32; the wasm32 target's usize is 32 bits")
            .to_le_bytes(),
    );
}

/// Append one painted value: a tag byte and its fixed-width payload.
fn encode_paint_value(body: &mut Vec<u8>, value: &PaintValue) {
    match value {
        PaintValue::Flag(flag) => {
            body.push(TAG_FLAG);
            body.push(u8::from(*flag));
        }
        PaintValue::Scalar(scalar) => {
            body.push(TAG_SCALAR);
            // Bits, not a float field: the decoder restores the exact `f32`, and
            // `PaintValue`'s equality is bit-based.
            body.extend_from_slice(&scalar.to_bits().to_le_bytes());
        }
        PaintValue::ToolIndex(index) => {
            body.push(TAG_TOOL_INDEX);
            body.extend_from_slice(&index.to_le_bytes());
        }
        PaintValue::Custom(text) => {
            body.push(TAG_CUSTOM);
            push_len(body, text.len());
            body.extend_from_slice(text.as_bytes());
        }
    }
}

/// Capacity hint bounded by the bytes actually left to read, so a corrupt count
/// cannot request a multi-gigabyte allocation.
fn bounded_capacity(count: u32, remaining: usize, min_item_bytes: usize) -> usize {
    (count as usize).min(remaining / min_item_bytes)
}

// ============================================================================
// Decoding reader
// ============================================================================

/// A forward-only bounded reader: every accessor returns `None` instead of
/// reading past the end, so no call site can slice out of bounds.
struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn remaining(&self) -> usize {
        self.bytes.len() - self.offset
    }

    fn take(&mut self, len: usize) -> Option<&'a [u8]> {
        let end = self.offset.checked_add(len)?;
        let slice = self.bytes.get(self.offset..end)?;
        self.offset = end;
        Some(slice)
    }

    fn u16(&mut self) -> Option<u16> {
        Some(u16::from_le_bytes(self.take(2)?.try_into().ok()?))
    }

    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }

    fn u64(&mut self) -> Option<u64> {
        Some(u64::from_le_bytes(self.take(8)?.try_into().ok()?))
    }

    fn i64(&mut self) -> Option<i64> {
        Some(i64::from_le_bytes(self.take(8)?.try_into().ok()?))
    }

    /// Read a `u32`-length-prefixed UTF-8 string.
    fn string(&mut self) -> Option<String> {
        let len = self.u32()?;
        let bytes = self.take(len as usize)?;
        String::from_utf8(bytes.to_vec()).ok()
    }

    fn paint_value(&mut self) -> Option<PaintValue> {
        let tag = self.take(1)?[0];
        Some(match tag {
            TAG_FLAG => PaintValue::Flag(match self.take(1)?[0] {
                0 => false,
                1 => true,
                _ => return None,
            }),
            TAG_SCALAR => PaintValue::Scalar(f32::from_bits(self.u32()?)),
            TAG_TOOL_INDEX => PaintValue::ToolIndex(self.u32()?),
            TAG_CUSTOM => PaintValue::Custom(self.string()?),
            _ => return None,
        })
    }

    fn point(&mut self) -> Option<Point2> {
        Some(Point2 {
            x: self.i64()?,
            y: self.i64()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One entry pinned to the golden test's layer, carrying one tool-index
    /// chain element and one edge.
    fn golden_entry() -> PlanEntry {
        PlanEntry {
            object_id: "o".to_string(),
            global_layer_index: 3,
            region_id: 2,
            tree_edge_segments: vec![[Point2 { x: 1, y: 2 }, Point2 { x: 3, y: 4 }]],
            variant_chain: vec![("m".to_string(), PaintValue::ToolIndex(1))],
        }
    }

    /// The D3 wire layout, written out by hand: magic, version, one entry with
    /// object `"o"`, region `2`, layer `3`, one `("m", ToolIndex(1))` chain
    /// element and one `(1,2)-(3,4)` edge in units.
    fn golden_bytes() -> Vec<u8> {
        let mut expected = Vec::new();
        // b"LTPL", format version 1, entry count 1.
        expected.extend_from_slice(&[0x4C, 0x54, 0x50, 0x4C]);
        expected.extend_from_slice(&[0x01, 0x00]);
        expected.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]);
        // object-id length 1, b"o".
        expected.extend_from_slice(&[0x01, 0x00, 0x00, 0x00, b'o']);
        // region id 2 (u64), layer index 3 (u32).
        expected.extend_from_slice(&[0x02, 0, 0, 0, 0, 0, 0, 0]);
        expected.extend_from_slice(&[0x03, 0x00, 0x00, 0x00]);
        // chain count 1; key length 1, b"m"; tool-index tag 2; index 1.
        expected.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]);
        expected.extend_from_slice(&[0x01, 0x00, 0x00, 0x00, b'm']);
        expected.push(0x02);
        expected.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]);
        // edge count 1; (1,2)-(3,4) as four i64 units.
        expected.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]);
        expected.extend_from_slice(&[0x01, 0, 0, 0, 0, 0, 0, 0]);
        expected.extend_from_slice(&[0x02, 0, 0, 0, 0, 0, 0, 0]);
        expected.extend_from_slice(&[0x03, 0, 0, 0, 0, 0, 0, 0]);
        expected.extend_from_slice(&[0x04, 0, 0, 0, 0, 0, 0, 0]);
        expected
    }

    #[test]
    fn piece_name_is_the_d3_layer_spelling() {
        assert_eq!(piece_name(0), "lightning-layer-0");
        assert_eq!(piece_name(247), "lightning-layer-247");
        assert!(piece_name(12).starts_with(PIECE_NAME_PREFIX));
    }

    /// The exit criterion's "literal expected body": the encoded bytes are
    /// compared to the layout written out by hand, so endianness, field order,
    /// fixed widths and integer coordinates are all pinned. A float coordinate
    /// would change these bytes.
    #[test]
    fn encoded_bytes_match_the_d3_wire_layout() {
        let plan = KernelPlan {
            entries: vec![golden_entry()],
        };
        assert_eq!(encode_layer_piece(&plan, 3), golden_bytes());
    }

    /// Round-trip preserves the full identity: object id, region id, layer
    /// index, ordered variant chain (every paint variant, including the
    /// float-carrying one, compared bit-exactly) and every edge coordinate,
    /// including negative and non-zero-fractional ones.
    #[test]
    fn round_trip_preserves_full_identity() {
        let plan = KernelPlan {
            entries: vec![
                PlanEntry {
                    object_id: "obj-\u{00e9}".to_string(),
                    global_layer_index: 7,
                    region_id: 5,
                    tree_edge_segments: vec![
                        [Point2 { x: -1, y: 2 }, Point2 { x: -3, y: 4 }],
                        [Point2 { x: 0, y: -9_000_000 }, Point2 { x: 12, y: 13 }],
                    ],
                    variant_chain: vec![
                        ("material".to_string(), PaintValue::ToolIndex(2)),
                        ("fuzzy_skin".to_string(), PaintValue::Flag(true)),
                        ("angle".to_string(), PaintValue::Scalar(0.1 + 0.2)),
                        (
                            "label".to_string(),
                            PaintValue::Custom("left \u{2192} right".to_string()),
                        ),
                    ],
                },
                PlanEntry {
                    object_id: "obj-b".to_string(),
                    global_layer_index: 7,
                    region_id: 6,
                    tree_edge_segments: Vec::new(),
                    variant_chain: Vec::new(),
                },
                // Another layer: must not travel in layer 7's piece.
                PlanEntry {
                    object_id: "obj-c".to_string(),
                    global_layer_index: 8,
                    region_id: 5,
                    tree_edge_segments: vec![[Point2 { x: 1, y: 1 }, Point2 { x: 2, y: 2 }]],
                    variant_chain: vec![("material".to_string(), PaintValue::Flag(false))],
                },
            ],
        };

        let piece = encode_layer_piece(&plan, 7);
        let decoded = decode_layer_piece(&piece_name(7), &piece).expect("layer 7 decodes");

        assert_eq!(decoded.entries.len(), 2);
        assert_eq!(
            decoded,
            KernelPlan {
                entries: plan.entries[..2].to_vec(),
            }
        );
        assert!(
            decoded
                .entries
                .iter()
                .all(|entry| entry.global_layer_index == 7),
            "a layer's piece carries only its own entries"
        );

        // The float-carrying paint value survives bit-exactly, not just
        // approximately: `PaintValue`'s equality is `to_bits`.
        let PaintValue::Scalar(angle) = decoded.entries[0].variant_chain[2].1 else {
            panic!("expected the scalar paint value");
        };
        assert_eq!(angle.to_bits(), (0.1_f32 + 0.2_f32).to_bits());
    }

    #[test]
    fn empty_layer_encodes_a_header_only_piece() {
        let plan = KernelPlan::default();
        let piece = encode_layer_piece(&plan, 4);

        // magic + version + zero entry count.
        assert_eq!(piece.len(), 4 + 2 + 4);
        assert_eq!(piece[..4], MAGIC);
        let decoded = decode_layer_piece(&piece_name(4), &piece).expect("empty layer decodes");
        assert_eq!(decoded.entries, Vec::new());
    }

    /// Every strict prefix of a valid body is malformed: a truncated buffer
    /// must never decode, and it must fail with `57410`, not with the
    /// unsupported-version code.
    #[test]
    fn every_truncation_is_malformed() {
        let plan = KernelPlan {
            entries: vec![golden_entry()],
        };
        let piece = encode_layer_piece(&plan, 3);

        for end in 0..piece.len() {
            let error = decode_layer_piece("lightning-layer-3", &piece[..end])
                .expect_err("a truncated body must not decode");
            assert_eq!(error.code, 57410, "truncated at {end}: {}", error.message);
            assert!(error.fatal, "truncated at {end} must be fatal");
            assert_eq!(
                error.message,
                "malformed lightning plan piece: lightning-layer-3"
            );
        }
    }

    #[test]
    fn unsupported_version_is_57411() {
        let plan = KernelPlan {
            entries: vec![golden_entry()],
        };
        let piece = encode_layer_piece(&plan, 3);

        for version in [0_u16, 2, u16::MAX] {
            let mut patched = piece.clone();
            patched[4..6].copy_from_slice(&version.to_le_bytes());

            let error = decode_layer_piece("lightning-layer-3", &patched)
                .expect_err("an unsupported version must not decode");
            assert_eq!(error.code, 57411);
            assert!(error.fatal, "57411 is fatal");
            assert_eq!(
                error.message,
                format!("unsupported lightning plan piece version: {version}")
            );
        }
    }

    /// A bad magic is a malformed body, not a version mismatch: `57411` means
    /// "this is a lightning piece from an incompatible writer".
    #[test]
    fn bad_magic_is_57410_not_57411() {
        let plan = KernelPlan {
            entries: vec![golden_entry()],
        };
        let mut piece = encode_layer_piece(&plan, 3);
        piece[0] = b'X';

        let error = decode_layer_piece("lightning-layer-3", &piece).expect_err("bad magic");
        assert_eq!(error.code, 57410);
        assert!(error.fatal);
    }

    /// A length prefix larger than the body is rejected without allocating it.
    #[test]
    fn overlong_object_id_length_is_malformed() {
        let mut piece = encode_layer_piece(
            &KernelPlan {
                entries: vec![golden_entry()],
            },
            3,
        );
        // Overwrite the entry's object-id length with the largest u32.
        piece[10..14].copy_from_slice(&u32::MAX.to_le_bytes());

        let error = decode_layer_piece("lightning-layer-3", &piece).expect_err("overlong id");
        assert_eq!(error.code, 57410);
        assert!(error.fatal);
    }

    /// The same bound applies to the nested counts: a chain count that cannot
    /// fit the remaining body fails at the first missing key.
    #[test]
    fn overlong_chain_count_is_malformed() {
        let mut piece = encode_layer_piece(
            &KernelPlan {
                entries: vec![golden_entry()],
            },
            3,
        );
        // Entry layout: header 10, object-id length 4 + "o" 1, region 8, layer
        // 4, then the chain count.
        let chain_count_at = 10 + 4 + 1 + 8 + 4;
        piece[chain_count_at..chain_count_at + 4].copy_from_slice(&u32::MAX.to_le_bytes());

        let error = decode_layer_piece("lightning-layer-3", &piece).expect_err("overlong chain");
        assert_eq!(error.code, 57410);
        assert!(error.fatal);
    }

    #[test]
    fn unknown_paint_tag_is_malformed() {
        let mut piece = encode_layer_piece(
            &KernelPlan {
                entries: vec![golden_entry()],
            },
            3,
        );
        // The tool-index tag sits right after the 1-byte key.
        let tag_at = 10 + 4 + 1 + 8 + 4 + 4 + 4 + 1;
        assert_eq!(piece[tag_at], TAG_TOOL_INDEX);
        piece[tag_at] = 0xFF;

        let error = decode_layer_piece("lightning-layer-3", &piece).expect_err("unknown tag");
        assert_eq!(error.code, 57410);
        assert!(error.fatal);
    }

    #[test]
    fn invalid_utf8_is_malformed() {
        let mut piece = encode_layer_piece(
            &KernelPlan {
                entries: vec![golden_entry()],
            },
            3,
        );
        // Replace the object id's only byte with a lone continuation byte.
        assert_eq!(piece[14], b'o');
        piece[14] = 0x80;

        let error = decode_layer_piece("lightning-layer-3", &piece).expect_err("invalid utf8");
        assert_eq!(error.code, 57410);
        assert!(error.fatal);
    }

    #[test]
    fn trailing_bytes_are_malformed() {
        let plan = KernelPlan {
            entries: vec![golden_entry()],
        };
        let mut piece = encode_layer_piece(&plan, 3);
        piece.push(0x00);

        let error = decode_layer_piece("lightning-layer-3", &piece).expect_err("trailing byte");
        assert_eq!(error.code, 57410);
        assert!(error.fatal);
    }
}
