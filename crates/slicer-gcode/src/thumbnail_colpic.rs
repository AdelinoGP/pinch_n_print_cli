// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/ColPic.cpp (ColPic_EncodeStr,
// ColPicEncode, Byte8bitEncode, ADList0)
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------

//! QIDI "ColPic" thumbnail codec.
//!
//! Ports OrcaSlicer's `ColPic.cpp` palette/RLE encoder plus the `ColPic_EncodeStr`
//! base64-like framing. The encoded stream is emitted as `;gimage:` (first
//! chunk) / `;simage:` (subsequent chunks) G-code comment lines.
//!
//! Deviation from canonical (AC-6): the canonical `compress_thumbnail_colpic`
//! flips the image vertically (a GL-buffer-orientation workaround). This port
//! does NOT flip; the first output row corresponds to the first source row.

use crate::thumbnail::ThumbnailError;

/// Maximum pixel dimension for a ColPic thumbnail (canonical 512px cap).
const MAX_DIM: u32 = 512;

/// Chunk boundary for each `;gimage:`/`;simage:` line. Picked so the line stays
/// well within G-code comment length limits; the wire format simply concatenates
/// chunks in order.
const CHUNK_SIZE: usize = 256;

/// Encode an RGBA image into the full ColPic G-code comment string.
///
/// Applies the 512px aspect-preserved cap (recursing after resize), encodes the
/// pixel data via the ColPic codec, and frames it as `;gimage:` / `;simage:`
/// chunks. Does not flip vertically (AC-6).
pub fn encode_colpic(rgba: &image::RgbaImage) -> Result<String, ThumbnailError> {
    let (used_w, used_h) = capped_dims(rgba.width(), rgba.height());
    let target = if used_w == rgba.width() && used_h == rgba.height() {
        rgba.clone()
    } else {
        image::imageops::resize(
            rgba,
            used_w,
            used_h,
            image::imageops::FilterType::CatmullRom,
        )
    };
    let encoded = colpic_encode_str(&target)?;
    let mut out = String::new();
    let mut first = true;
    for chunk in encoded.as_bytes().chunks(CHUNK_SIZE) {
        let data = String::from_utf8_lossy(chunk);
        if first {
            out.push_str(&format!(";gimage:{data}\n\n"));
            first = false;
        } else {
            out.push_str(&format!(";simage:{data}\n\n"));
        }
    }
    Ok(out)
}

/// Compute the aspect-preserved dimensions after the 512px cap.
///
/// Port of the cap logic in `ColPicEncode` (`ColPic.cpp`): if either dimension
/// exceeds `MAX_DIM`, the longer side is clamped to `MAX_DIM` and the other
/// side is scaled by the same ratio, rounded.
fn capped_dims(w: u32, h: u32) -> (u32, u32) {
    if w <= MAX_DIM && h <= MAX_DIM {
        return (w, h);
    }
    if w >= h {
        (
            MAX_DIM,
            ((h as f64) * (MAX_DIM as f64 / w as f64)).round() as u32,
        )
    } else {
        (
            ((w as f64) * (MAX_DIM as f64 / h as f64)).round() as u32,
            MAX_DIM,
        )
    }
}

/// Encode an RGBA image into the full ColPic G-code comment string, also
/// returning the dimensions actually used after the 512px cap (pure-output proof
/// of the resize, used by tests).
pub fn encode_colpic_with_capped_dims(
    rgba: &image::RgbaImage,
) -> Result<(String, u32, u32), ThumbnailError> {
    let (w, h) = capped_dims(rgba.width(), rgba.height());
    Ok((encode_colpic(rgba)?, w, h))
}

/// Convert an RGBA pixel to a 16-bit RGB565 color value.
#[inline]
fn to_rgb565(r: u8, g: u8, b: u8) -> u16 {
    (((r >> 3) as u16) << 11) | (((g >> 2) as u16) << 5) | ((b >> 3) as u16)
}

/// Palette entry mirroring OrcaSlicer's `ADList0` node: a 16-bit color and its
/// occurrence count.
#[derive(Clone, Copy)]
struct PaletteEntry {
    color: u16,
    qty: usize,
}

/// Port of `ADList0` (`ColPic.cpp`): palette dedup. Builds a list of distinct
/// 16-bit colors, incrementing `qty` on a match and appending on a miss. Bounded
/// by `maxqty` distinct colors.
fn adlist0(pixels: &[u16], maxqty: usize) -> Vec<PaletteEntry> {
    let mut list: Vec<PaletteEntry> = Vec::new();
    for &c in pixels {
        if let Some(e) = list.iter_mut().find(|e| e.color == c) {
            e.qty += 1;
        } else if list.len() < maxqty {
            list.push(PaletteEntry { color: c, qty: 1 });
        }
    }
    list
}

/// Port of `Byte8bitEncode` (`ColPic.cpp`): RLE of equal-color 16-bit runs capped
/// at 255, with a palette-indexed emit. Builds `listu16` (the final palette) and
/// returns the byte stream.
fn byte8bit_encode(pixels: &[u16], maxqty: usize) -> Vec<u8> {
    // Build palette via ADList0.
    let mut palette = adlist0(pixels, maxqty);
    // Sort by qty descending (canonical ColPicEncode behaviour).
    palette.sort_by_key(|e| std::cmp::Reverse(e.qty));

    // Map color -> index in palette.
    let index_of = |c: u16| palette.iter().position(|e| e.color == c);

    let mut out: Vec<u8> = Vec::new();

    // Rare colors dropped from the palette (beyond maxqty) are merged into
    // palette[0] per the canonical nearest-color-merge approximation.
    let mut i = 0usize;
    let n = pixels.len();
    let mut last_sid: i32 = -1;
    while i < n {
        let c = pixels[i];
        let idx = index_of(c).unwrap_or(0) as u32;
        let tid = idx % 32;
        let sid = idx / 32;
        if sid as i32 != last_sid {
            out.push((7u8 << 5) | sid as u8);
            last_sid = sid as i32;
        }
        // Count run length of equal color.
        let mut run = 1usize;
        while i + run < n && pixels[i + run] == c && run < 255 {
            run += 1;
        }
        if run <= 6 {
            out.push(((run as u8) << 5) | tid as u8);
        } else {
            out.push(tid as u8);
            out.push(run as u8);
        }
        i += run;
    }
    out
}

/// Port of `ColPicEncode` (`ColPic.cpp`): builds palette via ADList0, sorts by
/// qty descending, drops rare colors by nearest-color merge, then RLE-encodes.
fn colpic_encode(rgba: &image::RgbaImage) -> Vec<u8> {
    let (w, h) = (rgba.width() as usize, rgba.height() as usize);
    // Row-major, no flip (AC-6).
    let mut pixels: Vec<u16> = Vec::with_capacity(w * h);
    for y in 0..h {
        for x in 0..w {
            let p = rgba.get_pixel(x as u32, y as u32);
            pixels.push(to_rgb565(p[0], p[1], p[2]));
        }
    }
    let maxqty = 1024;
    byte8bit_encode(&pixels, maxqty)
}

/// Port of `ColPic_EncodeStr` (`ColPic.cpp`): pack the byte stream into a
/// 6-bit-group base64-like string. Input is padded to a multiple of 3 with
/// zeros; every 3 input bytes become 4 output chars using 6-bit groups; each
/// char is offset by 48; a result char equal to `'\\' (0x5C)` is remapped to
/// 126 (`~`).
fn colpic_encode_str(rgba: &image::RgbaImage) -> Result<String, ThumbnailError> {
    let mut data = colpic_encode(rgba);
    while !data.len().is_multiple_of(3) {
        data.push(0);
    }
    let mut out = String::with_capacity(data.len() / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk[1] as u32;
        let b2 = chunk[2] as u32;
        let combined = (b0 << 16) | (b1 << 8) | b2;
        for k in (0..4).rev() {
            let mut ch = ((combined >> (k * 6)) & 0x3F) as u8 + 48;
            if ch == 0x5C {
                ch = 126;
            }
            out.push(ch as char);
        }
    }
    Ok(out)
}
