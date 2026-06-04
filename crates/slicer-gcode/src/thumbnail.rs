//! PNG thumbnail base64 encoding for the G-code header.
//!
//! Extracted from `crates/slicer-runtime/src/gcode_emit.rs` (packet 86).

use std::fmt::Write;

/// Encode raw bytes to a Base64 string (RFC 4648 standard alphabet, no line breaks).
///
/// Hand-rolled to avoid requiring `base64` as a non-dev dependency.
fn base64_encode(data: &[u8]) -> String {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let combined = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((combined >> 18) & 63) as usize] as char);
        out.push(TABLE[((combined >> 12) & 63) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[((combined >> 6) & 63) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TABLE[(combined & 63) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

/// Produce the THUMBNAIL_BLOCK text (OrcaSlicer wire format, packet 55 Step 5).
///
/// Format (FACT from OrcaSlicerDocumented/src/libslic3r/GCode/Thumbnails.hpp:111-129):
/// - Sentinel: `; THUMBNAIL_BLOCK_START`
/// - Base64 lines: `; <chunk>` where each chunk is ≤ 76 characters (OrcaSlicer max_row_length)
/// - Sentinel: `; THUMBNAIL_BLOCK_END`
///
/// No metadata header line is emitted so that the region between the sentinels
/// contains only base64 data (required for the roundtrip test to decode cleanly).
pub fn serialize_thumbnail_block(png_bytes: &[u8]) -> String {
    const MAX_ROW_LENGTH: usize = 76;
    let encoded = base64_encode(png_bytes);
    let mut out = String::new();
    writeln!(out, "; THUMBNAIL_BLOCK_START").unwrap();
    let mut remaining = encoded.as_str();
    while remaining.len() > MAX_ROW_LENGTH {
        writeln!(out, "; {}", &remaining[..MAX_ROW_LENGTH]).unwrap();
        remaining = &remaining[MAX_ROW_LENGTH..];
    }
    if !remaining.is_empty() {
        writeln!(out, "; {}", remaining).unwrap();
    }
    writeln!(out, "; THUMBNAIL_BLOCK_END").unwrap();
    out
}
