//! PNG thumbnail base64 encoding for the G-code header.
//!
//! Extracted from `crates/slicer-runtime/src/gcode_emit.rs` (packet 86).

use thiserror::Error;

/// Supported thumbnail image formats, each with its byte-exact OrcaSlicer tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThumbnailFormat {
    /// PNG thumbnail.
    Png,
    /// JPEG thumbnail.
    Jpg,
    /// QOI thumbnail.
    Qoi,
    /// BTT TFT thumbnail.
    BttTft,
    /// QIDI ColPic thumbnail.
    ColPic,
}

impl ThumbnailFormat {
    /// The wire-format thumbnail key tag used in the G-code header comment.
    pub fn tag(&self) -> &'static str {
        match self {
            ThumbnailFormat::Png => "thumbnail",
            ThumbnailFormat::Jpg => "thumbnail_JPG",
            ThumbnailFormat::Qoi => "thumbnail_QOI",
            ThumbnailFormat::BttTft => "thumbnail_BIQU",
            ThumbnailFormat::ColPic => "thumbnail_QIDI",
        }
    }

    /// Map an upper-cased extension token to a format, or `None` if unknown.
    fn from_ext(ext: &str) -> Option<ThumbnailFormat> {
        match ext {
            "PNG" => Some(ThumbnailFormat::Png),
            "JPG" | "JPEG" => Some(ThumbnailFormat::Jpg),
            "QOI" => Some(ThumbnailFormat::Qoi),
            "BTT_TFT" => Some(ThumbnailFormat::BttTft),
            "COLPIC" => Some(ThumbnailFormat::ColPic),
            _ => None,
        }
    }
}

/// A single requested thumbnail resolution + format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThumbnailSpec {
    /// Thumbnail width in pixels.
    pub width: u32,
    /// Thumbnail height in pixels.
    pub height: u32,
    /// Thumbnail image format.
    pub format: ThumbnailFormat,
}

impl ThumbnailSpec {
    /// Construct a new thumbnail spec.
    pub fn new(width: u32, height: u32, format: ThumbnailFormat) -> Self {
        ThumbnailSpec {
            width,
            height,
            format,
        }
    }
}

/// Errors raised while parsing a `thumbnails` config key value.
#[derive(Error, Debug, PartialEq, Eq)]
pub enum ThumbnailError {
    /// A thumbnail spec entry could not be parsed.
    #[error("malformed thumbnail spec `{entry}`: {reason}")]
    MalformedSpec {
        /// The offending entry text verbatim.
        entry: String,
        /// Human-readable reason for the rejection.
        reason: String,
    },
    /// A source image could not be decoded (e.g. not a valid PNG).
    #[error("failed to decode thumbnail source image: {0}")]
    Decode(String),
    /// A rendered image could not be encoded to its target format.
    #[error("failed to encode thumbnail to {format}: {reason}")]
    Encode {
        /// The target format tag.
        format: &'static str,
        /// Human-readable reason for the failure.
        reason: String,
    },
}

/// Parse a `thumbnails` config-key value into a list of [`ThumbnailSpec`].
///
/// Format: a comma-separated list of `WxH/EXT` entries. Each entry is split on
/// `x` (dimensions) then `/` (extension). The extension is upper-cased and
/// matched case-insensitively. JPEG is accepted as an alias of JPG (design.md).
/// Surrounding whitespace on entries/components is trimmed (design.md whitespace
/// handling is FWD).
pub fn parse_thumbnails_key(value: &str) -> Result<Vec<ThumbnailSpec>, ThumbnailError> {
    if value.trim().is_empty() {
        return Ok(Vec::new());
    }
    let mut specs = Vec::new();
    for entry in value.split(',') {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }
        let parts: Vec<&str> = entry.splitn(2, '/').collect();
        if parts.len() != 2 {
            return Err(ThumbnailError::MalformedSpec {
                entry: entry.to_string(),
                reason: "missing extension (expected WxH/EXT)".to_string(),
            });
        }
        let dims = parts[0].trim();
        let ext = parts[1].trim().to_uppercase();
        let dim_parts: Vec<&str> = dims.splitn(2, 'x').collect();
        if dim_parts.len() != 2 {
            return Err(ThumbnailError::MalformedSpec {
                entry: entry.to_string(),
                reason: "missing height (expected WxH/EXT)".to_string(),
            });
        }
        let width =
            dim_parts[0]
                .trim()
                .parse::<u32>()
                .map_err(|_| ThumbnailError::MalformedSpec {
                    entry: entry.to_string(),
                    reason: format!("invalid width `{}`", dim_parts[0].trim()),
                })?;
        let height =
            dim_parts[1]
                .trim()
                .parse::<u32>()
                .map_err(|_| ThumbnailError::MalformedSpec {
                    entry: entry.to_string(),
                    reason: format!("invalid height `{}`", dim_parts[1].trim()),
                })?;
        let format =
            ThumbnailFormat::from_ext(&ext).ok_or_else(|| ThumbnailError::MalformedSpec {
                entry: entry.to_string(),
                reason: format!("unsupported extension `{ext}`"),
            })?;
        specs.push(ThumbnailSpec::new(width, height, format));
    }
    Ok(specs)
}

/// Encode raw bytes to a Base64 string (RFC 4648 standard alphabet, no line breaks).
///
/// Hand-rolled to avoid requiring `base64` as a non-dev dependency.
pub fn encode_base64(data: &[u8]) -> String {
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

/// Body of a rendered thumbnail entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThumbnailBody {
    /// Base64 payload, pre-encoded (PNG/JPG/QOI).
    Base64(String),
    /// Self-framed text (ColPic / BTT_TFT). Spliced verbatim into the block.
    Raw(String),
}

/// A fully rendered thumbnail ready to be framed into the G-code header block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedThumbnail {
    /// Thumbnail image format (carries its Orca tag).
    pub format: ThumbnailFormat,
    /// Thumbnail width in pixels.
    pub width: u32,
    /// Thumbnail height in pixels.
    pub height: u32,
    /// Encoded body of the thumbnail.
    pub body: ThumbnailBody,
}

/// Decode a Base64 string (RFC 4648 standard alphabet, no line breaks) back to
/// bytes. Inverse of [`encode_base64`].
pub fn decode_base64(s: &str) -> Result<Vec<u8>, ThumbnailError> {
    let mut buf: Vec<u8> = Vec::with_capacity(s.len() / 4 * 3);
    let mut acc: u32 = 0;
    let mut nbits: u32 = 0;
    for c in s.bytes() {
        if c == b'=' {
            continue;
        }
        let val = match c {
            b'A'..=b'Z' => c as u32 - b'A' as u32,
            b'a'..=b'z' => c as u32 - b'a' as u32 + 26,
            b'0'..=b'9' => c as u32 - b'0' as u32 + 52,
            b'+' => 62,
            b'/' => 63,
            _ => {
                return Err(ThumbnailError::Decode(format!(
                    "invalid base64 character {:?}",
                    c as char
                )))
            }
        };
        acc = (acc << 6) | val;
        nbits += 6;
        if nbits >= 8 {
            nbits -= 8;
            buf.push((acc >> nbits) as u8);
        }
    }
    Ok(buf)
}

/// Render each requested [`ThumbnailSpec`] from a single source PNG.
///
/// Decodes `source_png` once, then for each spec resizes with
/// `FilterType::CatmullRom` and encodes into the requested format. A PNG spec
/// whose requested dimensions exactly match the source is passed through with no
/// re-encode (source bytes spliced verbatim).
pub fn render_thumbnail_entries(
    source_png: &[u8],
    specs: &[ThumbnailSpec],
) -> Result<Vec<RenderedThumbnail>, ThumbnailError> {
    use std::io::Cursor;

    use crate::thumbnail_btt::encode_btt_tft;
    use crate::thumbnail_colpic::encode_colpic;

    let src = image::load_from_memory_with_format(source_png, image::ImageFormat::Png)
        .map_err(|e| ThumbnailError::Decode(e.to_string()))?;
    let rgba = src.to_rgba8();
    let src_w = rgba.width();
    let src_h = rgba.height();

    let mut out = Vec::with_capacity(specs.len());
    for spec in specs {
        let body = match spec.format {
            ThumbnailFormat::Png if spec.width == src_w && spec.height == src_h => {
                ThumbnailBody::Base64(encode_base64(source_png))
            }
            ThumbnailFormat::Png => {
                let resized = image::imageops::resize(
                    &rgba,
                    spec.width,
                    spec.height,
                    image::imageops::FilterType::CatmullRom,
                );
                let mut buf: Vec<u8> = Vec::new();
                resized
                    .write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Png)
                    .map_err(|e| ThumbnailError::Encode {
                        format: "PNG",
                        reason: e.to_string(),
                    })?;
                ThumbnailBody::Base64(encode_base64(&buf))
            }
            ThumbnailFormat::Jpg => {
                let resized = image::imageops::resize(
                    &rgba,
                    spec.width,
                    spec.height,
                    image::imageops::FilterType::CatmullRom,
                );
                let mut buf: Vec<u8> = Vec::new();
                {
                    let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 90);
                    enc.encode_image(&resized)
                        .map_err(|e| ThumbnailError::Encode {
                            format: "JPG",
                            reason: e.to_string(),
                        })?;
                }
                ThumbnailBody::Base64(encode_base64(&buf))
            }
            ThumbnailFormat::Qoi => {
                let resized = image::imageops::resize(
                    &rgba,
                    spec.width,
                    spec.height,
                    image::imageops::FilterType::CatmullRom,
                );
                let mut buf: Vec<u8> = Vec::new();
                resized
                    .write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Qoi)
                    .map_err(|e| ThumbnailError::Encode {
                        format: "QOI",
                        reason: e.to_string(),
                    })?;
                ThumbnailBody::Base64(encode_base64(&buf))
            }
            ThumbnailFormat::BttTft => {
                let resized = image::imageops::resize(
                    &rgba,
                    spec.width,
                    spec.height,
                    image::imageops::FilterType::CatmullRom,
                );
                ThumbnailBody::Raw(encode_btt_tft(&resized))
            }
            ThumbnailFormat::ColPic => {
                let resized = image::imageops::resize(
                    &rgba,
                    spec.width,
                    spec.height,
                    image::imageops::FilterType::CatmullRom,
                );
                ThumbnailBody::Raw(encode_colpic(&resized)?)
            }
        };
        out.push(RenderedThumbnail {
            format: spec.format,
            width: spec.width,
            height: spec.height,
            body,
        });
    }
    Ok(out)
}

/// Maximum base64 characters emitted per `; ` line (Orca row length).
pub const MAX_ROW_LENGTH: usize = 78;

/// Produce the THUMBNAIL_BLOCK text (OrcaSlicer wire format, packet 173 Step 2).
///
/// Outer sentinels `; THUMBNAIL_BLOCK_START` / `; THUMBNAIL_BLOCK_END` wrap a
/// sequence of per-entry frames. Each Base64 entry is framed as:
///
/// ```text
/// ; <tag> begin <W>x<H> <len>
/// ; <base64_chunk_1>
/// ; <base64_chunk_2>
/// ...
/// ; <tag> end
/// ```
///
/// where each `; ` line carries at most [`MAX_ROW_LENGTH`] base64 characters.
/// Raw entries are spliced verbatim (assumed self-framed, ending in `\n`).
pub fn serialize_thumbnail_block(entries: &[RenderedThumbnail]) -> String {
    let mut out = String::new();
    out.push_str("; THUMBNAIL_BLOCK_START\n");
    for entry in entries {
        let tag = entry.format.tag();
        match &entry.body {
            ThumbnailBody::Base64(s) => {
                let len = s.len();
                let w = entry.width;
                let h = entry.height;
                out.push_str(&format!("; {tag} begin {w}x{h} {len}\n"));
                let mut remaining = s.as_str();
                while remaining.len() > MAX_ROW_LENGTH {
                    out.push_str("; ");
                    out.push_str(&remaining[..MAX_ROW_LENGTH]);
                    out.push('\n');
                    remaining = &remaining[MAX_ROW_LENGTH..];
                }
                if !remaining.is_empty() {
                    out.push_str("; ");
                    out.push_str(remaining);
                    out.push('\n');
                }
                out.push_str(&format!("; {tag} end\n"));
            }
            ThumbnailBody::Raw(s) => {
                out.push_str(s);
            }
        }
    }
    out.push_str("; THUMBNAIL_BLOCK_END\n");
    out
}
