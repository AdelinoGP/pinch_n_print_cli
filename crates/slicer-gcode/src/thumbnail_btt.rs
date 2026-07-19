// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/Thumbnails.cpp
// (compress_thumbnail_btt_tft)
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------

//! BTT TFT (BIQU) thumbnail codec.
//!
//! Ports OrcaSlicer's `compress_thumbnail_btt_tft` raw RGB565 encoder. The
//! wire format is a self-framed text block:
//!
//! ```text
//! ;<WWWW><HHHH>\r\n
//! ;<rgb565_0><rgb565_1>...<rgb565_N>\r\n
//! ;<rgb565_0>...<rgb565_N>\r\n
//! ...
//! ```
//!
//! where `WWWW`/`HHHH` are 4 upper-case hex digits of width/height and each
//! subsequent line is `;` + the row's RGB565 pixels concatenated (no separator)
//! + CRLF.
//!
//! Deviation from canonical (AC-6): the canonical `compress_thumbnail_btt_tft`
//! flips the image vertically (a GL-buffer-orientation workaround). This port
//! does NOT flip; the first output row corresponds to the first source row.

/// Pack a single 8-bit RGBA pixel into a 16-bit RGB565 value.
///
/// `((r>>3)<<11) | ((g>>2)<<5) | (b>>3)`. Only the R/G/B channels are used;
/// alpha is ignored (the BTT TFT format carries no alpha).
#[inline]
fn pack_rgb565(r: u8, g: u8, b: u8) -> u16 {
    (((r >> 3) as u16) << 11) | (((g >> 2) as u16) << 5) | ((b >> 3) as u16)
}

/// Encode an `RgbaImage` into the BTT TFT raw text format.
///
/// Rows are emitted in source order (no vertical flip, AC-6). Each line is
/// terminated with CRLF (`\r\n`).
pub fn encode_btt_tft(rgba: &image::RgbaImage) -> String {
    let width = rgba.width();
    let height = rgba.height();

    let mut out = String::new();
    out.push(';');
    out.push_str(&format!("{:04X}{:04X}", width, height));
    out.push_str("\r\n");

    for y in 0..height {
        out.push(';');
        for x in 0..width {
            let p = rgba.get_pixel(x, y);
            let v = pack_rgb565(p[0], p[1], p[2]);
            out.push_str(&format!("{:04X}", v));
        }
        out.push_str("\r\n");
    }

    out
}
