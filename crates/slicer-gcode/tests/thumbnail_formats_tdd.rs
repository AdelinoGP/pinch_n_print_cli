//! TDD test slice for packet 173 — thumbnail multi-format parsing.

use slicer_gcode::{
    decode_base64, encode_base64, encode_btt_tft, encode_colpic, encode_colpic_with_capped_dims,
    parse_thumbnails_key, render_thumbnail_entries, serialize_thumbnail_block, RenderedThumbnail,
    ThumbnailBody, ThumbnailFormat, ThumbnailSpec,
};

/// Build a solid `width x height` RGBA image with the given fill color (per pixel
/// a constant color keeps ColPic runs long and deterministic).
fn solid_rgba(width: u32, height: u32, r: u8, g: u8, b: u8) -> image::RgbaImage {
    image::RgbaImage::from_pixel(width, height, image::Rgba([r, g, b, 255]))
}

/// Build a small PNG byte buffer from an `RgbaImage` (used as `source_png`).
fn png_bytes(img: &image::RgbaImage) -> Vec<u8> {
    let mut buf = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
        .unwrap();
    buf
}

#[test]
fn parse_thumbnails_key_accepts_multi_entry_png() {
    let specs = parse_thumbnails_key("48x48/PNG,300x300/PNG").unwrap();
    assert_eq!(specs.len(), 2);
    assert_eq!(specs[0].width, 48);
    assert_eq!(specs[0].height, 48);
    assert!(matches!(specs[0].format, ThumbnailFormat::Png));
    assert_eq!(specs[1].width, 300);
    assert_eq!(specs[1].height, 300);
    assert!(matches!(specs[1].format, ThumbnailFormat::Png));
}

#[test]
fn parse_thumbnails_key_case_insensitive_ext() {
    let specs =
        parse_thumbnails_key("32x32/jpg,32x32/JpeG,32x32/BTT_TFT,32x32/COLPIC,32x32/QOI").unwrap();
    assert_eq!(specs.len(), 5);
    assert!(matches!(specs[0].format, ThumbnailFormat::Jpg));
    assert!(matches!(specs[1].format, ThumbnailFormat::Jpg));
    assert!(matches!(specs[2].format, ThumbnailFormat::BttTft));
    assert!(matches!(specs[3].format, ThumbnailFormat::ColPic));
    assert!(matches!(specs[4].format, ThumbnailFormat::Qoi));
}

#[test]
fn parse_thumbnails_key_rejects_malformed_wxh_no_h() {
    let err = parse_thumbnails_key("48x/PNG").unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("48x/PNG"),
        "error Display missing token: {msg}"
    );
}

#[test]
fn parse_thumbnails_key_rejects_missing_ext() {
    let err = parse_thumbnails_key("48x48").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("48x48"), "error Display missing token: {msg}");
}

#[test]
fn parse_thumbnails_key_rejects_unknown_ext() {
    let err = parse_thumbnails_key("48x48/BMP").unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("48x48/BMP"),
        "error Display missing token: {msg}"
    );
}

#[test]
fn parse_thumbnails_key_accepts_single_entry() {
    let specs = parse_thumbnails_key("16x16/PNG").unwrap();
    assert_eq!(specs.len(), 1);
    assert_eq!(specs[0].width, 16);
    assert_eq!(specs[0].height, 16);
    assert!(matches!(specs[0].format, ThumbnailFormat::Png));
}

#[test]
fn thumbnail_format_tag_strings() {
    assert_eq!(ThumbnailFormat::Png.tag(), "thumbnail");
    assert_eq!(ThumbnailFormat::Jpg.tag(), "thumbnail_JPG");
    assert_eq!(ThumbnailFormat::Qoi.tag(), "thumbnail_QOI");
    assert_eq!(ThumbnailFormat::BttTft.tag(), "thumbnail_BIQU");
    assert_eq!(ThumbnailFormat::ColPic.tag(), "thumbnail_QIDI");
}

#[test]
fn parse_thumbnails_key_empty_string_yields_empty() {
    let specs = parse_thumbnails_key("").unwrap();
    assert!(specs.is_empty());
}

#[test]
fn parse_thumbnails_key_trims_whitespace() {
    // note: design.md lists whitespace handling as FWD. We trim around each
    // comma-separated entry and its components so surrounding spaces are tolerated.
    let specs = parse_thumbnails_key(" 16x16/PNG , 32x32/PNG ").unwrap();
    assert_eq!(specs.len(), 2);
    assert_eq!(specs[0].width, 16);
    assert_eq!(specs[0].height, 16);
    assert!(matches!(specs[0].format, ThumbnailFormat::Png));
    assert_eq!(specs[1].width, 32);
    assert_eq!(specs[1].height, 32);
    assert!(matches!(specs[1].format, ThumbnailFormat::Png));
}

// ---- packet 173 Step 2: Orca-parseable per-entry wire framing ----

#[test]
fn serialize_thumbnail_block_single_png_entry_framing() {
    let body = "A".repeat(78);
    let len = body.len();
    let entry = RenderedThumbnail {
        format: ThumbnailFormat::Png,
        width: 48,
        height: 48,
        body: ThumbnailBody::Base64(body),
    };
    let out = serialize_thumbnail_block(&[entry]);
    assert!(
        out.starts_with("; THUMBNAIL_BLOCK_START\n"),
        "must start with sentinel: {out:?}"
    );
    assert!(
        out.contains(&format!("; thumbnail begin 48x48 {len}\n")),
        "missing begin line: {out:?}"
    );
    assert!(
        out.contains("; thumbnail end\n"),
        "missing end line: {out:?}"
    );
    let b64_lines: Vec<&str> = out
        .lines()
        .filter(|l| {
            l.starts_with("; ")
                && !l.contains("begin")
                && !l.contains("end")
                && !l.starts_with("; THUMBNAIL")
        })
        .map(|l| &l[2..])
        .collect();
    assert_eq!(b64_lines.len(), 1, "expected exactly one base64 line");
    assert_eq!(b64_lines[0].len(), 78, "base64 line must be 78 chars");
    assert!(
        out.ends_with("; THUMBNAIL_BLOCK_END\n"),
        "must end with sentinel: {out:?}"
    );
}

#[test]
fn serialize_thumbnail_block_78_col_wrap() {
    let body = "A".repeat(200);
    let entry = RenderedThumbnail {
        format: ThumbnailFormat::Png,
        width: 10,
        height: 10,
        body: ThumbnailBody::Base64(body),
    };
    let out = serialize_thumbnail_block(&[entry]);
    let b64_lines: Vec<&str> = out
        .lines()
        .filter(|l| {
            l.starts_with("; ")
                && !l.contains("begin")
                && !l.contains("end")
                && !l.starts_with("; THUMBNAIL")
        })
        .map(|l| &l[2..])
        .collect();
    // 200 = 78 + 78 + 44
    assert_eq!(
        b64_lines,
        vec!["A".repeat(78), "A".repeat(78), "A".repeat(44)]
    );
    for l in &b64_lines {
        assert!(l.len() <= 78, "line exceeds 78: len {}", l.len());
    }
}

#[test]
fn serialize_thumbnail_block_multi_entry_ordering() {
    let e1 = RenderedThumbnail {
        format: ThumbnailFormat::Png,
        width: 48,
        height: 48,
        body: ThumbnailBody::Base64("AAAA".to_string()),
    };
    let e2 = RenderedThumbnail {
        format: ThumbnailFormat::Jpg,
        width: 32,
        height: 32,
        body: ThumbnailBody::Base64("BBBB".to_string()),
    };
    let out = serialize_thumbnail_block(&[e1, e2]);
    let begin_png = out.find("; thumbnail begin 48x48 4\n").expect("png begin");
    let end_png = out.find("; thumbnail end\n").expect("png end");
    let begin_jpg = out
        .find("; thumbnail_JPG begin 32x32 4\n")
        .expect("jpg begin");
    let end_jpg = out.rfind("; thumbnail_JPG end\n").expect("jpg end");
    assert!(begin_png < end_png, "png begin before png end");
    assert!(end_png < begin_jpg, "png before jpg");
    assert!(begin_jpg < end_jpg, "jpg begin before jpg end");
}

#[test]
fn serialize_thumbnail_block_raw_body_spliced_verbatim() {
    let entry = RenderedThumbnail {
        format: ThumbnailFormat::ColPic,
        width: 0,
        height: 0,
        body: ThumbnailBody::Raw("foo\nbar\n".to_string()),
    };
    let out = serialize_thumbnail_block(&[entry]);
    let start = out.find("; THUMBNAIL_BLOCK_START\n").unwrap();
    let end = out.find("; THUMBNAIL_BLOCK_END\n").unwrap();
    let middle = &out[start + "; THUMBNAIL_BLOCK_START\n".len()..end];
    assert!(
        middle.contains("foo\nbar\n"),
        "raw body not spliced: {middle:?}"
    );
}

#[test]
fn serialize_thumbnail_block_empty_entries_emits_just_sentinels() {
    let out = serialize_thumbnail_block(&[]);
    assert_eq!(
        out, "; THUMBNAIL_BLOCK_START\n; THUMBNAIL_BLOCK_END\n",
        "empty block must be exactly the two sentinels: {out:?}"
    );
}

#[test]
fn rendered_thumbnail_format_tag_png() {
    let entry = RenderedThumbnail {
        format: ThumbnailFormat::Png,
        width: 48,
        height: 48,
        body: ThumbnailBody::Base64(encode_base64(&[0u8; 3])),
    };
    let out = serialize_thumbnail_block(&[entry]);
    assert!(
        out.contains("; thumbnail begin 48x48 "),
        "begin line must use Png tag: {out:?}"
    );
}

// ---- packet 173 Step 3: multi-format renderer + ColPic codec ----

#[test]
fn colpic_magic_prefix_gimage() {
    // 4x4 image, below the 512 cap so a single ;gimage: chunk is emitted.
    let img = solid_rgba(4, 4, 200, 100, 50);
    let s = encode_colpic(&img).unwrap();
    assert!(
        s.starts_with(";gimage:"),
        "colpic output must start with ;gimage:, got: {s:?}"
    );
}

#[test]
fn colpic_512_cap_aspect_preserved() {
    // 1024x256 wide image: cap scales the longer dim to 512, shorter
    // aspect-preserved -> 128. Use the capped-dims helper to prove the cap
    // actually fired and the aspect ratio was preserved.
    let wide = solid_rgba(1024, 256, 10, 20, 30);
    let (capped, w, h) = encode_colpic_with_capped_dims(&wide).unwrap();
    assert_eq!(
        (w, h),
        (512, 128),
        "1024x256 must cap to 512x128 (aspect ratio preserved)"
    );
    assert!(
        capped.starts_with(";gimage:"),
        "capped colpic must still start with ;gimage:"
    );

    let small = solid_rgba(256, 64, 10, 20, 30);
    let (small_enc, sw, sh) = encode_colpic_with_capped_dims(&small).unwrap();
    assert_eq!(
        (sw, sh),
        (256, 64),
        "256x64 is below the cap and must stay unchanged"
    );
    assert_ne!(
        capped, small_enc,
        "512-capped wide image should encode differently from a 256x64 image"
    );
    assert!(
        !capped.is_empty(),
        "colpic output must be non-empty after cap"
    );
}

/// AC-4 pure-output proof: a 1024x256 image and a 512x128 image with the same
/// uniform color must produce byte-identical ColPic output, because the cap
/// resizes the 1024x256 source down to 512x128 before encoding. If they differ,
/// the cap didn't fire.
#[test]
fn colpic_512_cap_dims() {
    let wide = solid_rgba(1024, 256, 10, 20, 30);
    let (wide_enc, w, h) = encode_colpic_with_capped_dims(&wide).unwrap();
    assert_eq!((w, h), (512, 128), "cap must yield 512x128 for 1024x256");

    let capped_src = solid_rgba(512, 128, 10, 20, 30);
    let capped_enc = encode_colpic(&capped_src).unwrap();

    assert_eq!(
        wide_enc, capped_enc,
        "1024x256 (capped) and 512x128 (same color) must encode identically"
    );
}

#[test]
fn colpic_subsequent_chunk_uses_simage() {
    // 256x256 with diverse colors yields data > one 256-char chunk, so a
    // ;simage: continuation chunk must appear.
    let mut img = image::RgbaImage::new(256, 256);
    for y in 0..256u32 {
        for x in 0..256u32 {
            let c = ((x ^ y) % 256) as u8;
            img.put_pixel(
                x,
                y,
                image::Rgba([c, c.wrapping_mul(3), c.wrapping_mul(7), 255]),
            );
        }
    }
    let s = encode_colpic(&img).unwrap();
    assert!(
        s.contains(";simage:"),
        "large colpic must emit at least one ;simage: continuation chunk: {s:?}"
    );
}

#[test]
fn no_reflip_top_down_gradient() {
    // Vertical gradient: top row red, bottom row blue. After a 16x16 Png
    // render the decoded top row must stay red-ish and bottom row blue-ish
    // (no vertical flip introduced, AC-6).
    let w = 16u32;
    let h = 16u32;
    let mut img = image::RgbaImage::new(w, h);
    for y in 0..h {
        let t = y as f32 / (h - 1) as f32;
        let r = (255.0 * (1.0 - t)) as u8;
        let b = (255.0 * t) as u8;
        for x in 0..w {
            img.put_pixel(x, y, image::Rgba([r, 0, b, 255]));
        }
    }
    let source = png_bytes(&img);
    let entries =
        render_thumbnail_entries(&source, &[ThumbnailSpec::new(16, 16, ThumbnailFormat::Png)])
            .unwrap();
    let body = match &entries[0].body {
        ThumbnailBody::Base64(b) => b.clone(),
        _ => panic!("expected base64 body"),
    };
    let decoded = decode_base64(&body).unwrap();
    let reloaded = image::load_from_memory(&decoded).unwrap().to_rgba8();
    let top = reloaded.get_pixel(0, 0);
    let bottom = reloaded.get_pixel(0, h - 1);
    assert!(top[0] > top[2], "top row should be red-ish: {top:?}");
    assert!(
        bottom[2] > bottom[0],
        "bottom row should be blue-ish: {bottom:?}"
    );
}

/// Build a vertical gradient image: top row = red, bottom row = blue. Used to
/// prove that no vertical flip is introduced by the multi-format renderer.
fn gradient_rgba(w: u32, h: u32) -> image::RgbaImage {
    let mut img = image::RgbaImage::new(w, h);
    for y in 0..h {
        let t = y as f32 / (h - 1) as f32;
        let r = (255.0 * (1.0 - t)) as u8;
        let b = (255.0 * t) as u8;
        for x in 0..w {
            img.put_pixel(x, y, image::Rgba([r, 0, b, 255]));
        }
    }
    img
}

/// AC-6: JPG render must not vertically flip a top-down gradient.
#[test]
fn no_reflip_jpg() {
    let w = 32u32;
    let h = 32u32;
    let source = png_bytes(&gradient_rgba(w, h));
    let entries =
        render_thumbnail_entries(&source, &[ThumbnailSpec::new(w, h, ThumbnailFormat::Jpg)])
            .unwrap();
    let body = match &entries[0].body {
        ThumbnailBody::Base64(b) => decode_base64(b).unwrap(),
        _ => panic!("expected base64 body"),
    };
    let reloaded = image::load_from_memory(&body).unwrap().to_rgba8();
    let top = reloaded.get_pixel(0, 0);
    let bottom = reloaded.get_pixel(0, h - 1);
    assert!(top[0] > top[2], "top row should be red-ish: {top:?}");
    assert!(
        bottom[2] > bottom[0],
        "bottom row should be blue-ish: {bottom:?}"
    );
}

/// AC-6: QOI render must not vertically flip a top-down gradient.
#[test]
fn no_reflip_qoi() {
    let w = 32u32;
    let h = 32u32;
    let source = png_bytes(&gradient_rgba(w, h));
    let entries =
        render_thumbnail_entries(&source, &[ThumbnailSpec::new(w, h, ThumbnailFormat::Qoi)])
            .unwrap();
    let body = match &entries[0].body {
        ThumbnailBody::Base64(b) => decode_base64(b).unwrap(),
        _ => panic!("expected base64 body"),
    };
    let reloaded = image::load_from_memory(&body).unwrap().to_rgba8();
    let top = reloaded.get_pixel(0, 0);
    let bottom = reloaded.get_pixel(0, h - 1);
    assert!(top[0] > top[2], "top row should be red-ish: {top:?}");
    assert!(
        bottom[2] > bottom[0],
        "bottom row should be blue-ish: {bottom:?}"
    );
}

/// AC-6: ColPic must not vertically flip.
///
/// The ColPic codec derives its palette per-image, sorts it by quantity
/// descending, and RLE-emits palette indices. An exact reverse of rows where
/// every color has the SAME quantity re-encodes to the SAME bytes (the codec
/// is symmetric for that case), so a naive flip test would pass even under a
/// flip. To make orientation observable we use a top-down gradient whose TOP
/// color (red) recurs in the first two rows, while green/blue/yellow each
/// appear once below it. The double red makes the run sequence non-palindromic
/// under the quantity-sort, so a vertical flip changes both the palette
/// first-occurrence order and the run sequence — a non-flipping encoder MUST
/// emit a different string than the flipped version.
#[test]
fn no_reflip_colpic() {
    let w = 4u32;
    let h = 5u32;

    // Top-down gradient: red at the top (two rows), then green, blue, yellow.
    // Four distinct colors; red is the most-quantized (top) color.
    let rows_top_down = [
        (255, 0, 0, 255),   // red (top, row 0)
        (255, 0, 0, 255),   // red (row 1)
        (0, 255, 0, 255),   // green
        (0, 0, 255, 255),   // blue
        (255, 255, 0, 255), // yellow
    ];
    let mut top_down = image::RgbaImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let (r, g, b, a) = rows_top_down[y as usize];
            top_down.put_pixel(x, y, image::Rgba([r, g, b, a]));
        }
    }
    let td_enc = encode_colpic(&top_down).unwrap();

    // Bottom-up = exact vertical flip of the top-down gradient (reverse rows):
    // yellow, blue, green, red, red.
    let mut bottom_up = image::RgbaImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let (r, g, b, a) = rows_top_down[(h - 1 - y) as usize];
            bottom_up.put_pixel(x, y, image::Rgba([r, g, b, a]));
        }
    }
    let bu_enc = encode_colpic(&bottom_up).unwrap();

    assert!(
        !td_enc.is_empty(),
        "ColPic output must be non-empty for a gradient"
    );
    // Orientation-sensitive: the double-red top row makes the flip change the
    // encoded run sequence, so a non-flipping encoder MUST emit a different
    // string than the flipped version. Identical output would mean the encoder
    // is orientation-blind (flipped).
    assert_ne!(
        td_enc, bu_enc,
        "ColPic encoding is NOT orientation-sensitive — a flip would collapse \
         the top-down gradient and its vertical flip to identical output"
    );
    // Deterministic, order-sensitive: re-encoding the same image is stable.
    assert_eq!(
        td_enc,
        encode_colpic(&top_down).unwrap(),
        "ColPic encoding must be deterministic"
    );
}

#[test]
fn jpg_entry_starts_with_soi_marker() {
    let source = png_bytes(&solid_rgba(32, 32, 123, 45, 200));
    let entries =
        render_thumbnail_entries(&source, &[ThumbnailSpec::new(32, 32, ThumbnailFormat::Jpg)])
            .unwrap();
    let body = match &entries[0].body {
        ThumbnailBody::Base64(b) => b.clone(),
        _ => panic!("expected base64 body"),
    };
    let decoded = decode_base64(&body).unwrap();
    assert_eq!(
        &decoded[0..2],
        &[0xFF, 0xD8],
        "JPEG must start with SOI marker"
    );
}

#[test]
fn qoi_entry_starts_with_qoif_magic() {
    let source = png_bytes(&solid_rgba(32, 32, 12, 34, 56));
    let entries =
        render_thumbnail_entries(&source, &[ThumbnailSpec::new(32, 32, ThumbnailFormat::Qoi)])
            .unwrap();
    let body = match &entries[0].body {
        ThumbnailBody::Base64(b) => b.clone(),
        _ => panic!("expected base64 body"),
    };
    let decoded = decode_base64(&body).unwrap();
    assert_eq!(&decoded[0..4], b"qoif", "QOI must start with qoif magic");
}

// ---- packet 173 Step 4: BTT_TFT (BIQU) RGB565 codec ----

/// Build a `width x height` RGBA image from a row-major list of RGBA pixels.
fn rgba_from_rows(width: u32, height: u32, pixels: &[(u8, u8, u8, u8)]) -> image::RgbaImage {
    assert_eq!(
        (width as usize) * (height as usize),
        pixels.len(),
        "pixel count must match dims"
    );
    let mut img = image::RgbaImage::new(width, height);
    for (i, &(r, g, b, a)) in pixels.iter().enumerate() {
        let x = (i % width as usize) as u32;
        let y = (i / width as usize) as u32;
        img.put_pixel(x, y, image::Rgba([r, g, b, a]));
    }
    img
}

#[test]
fn btt_tft_header_8_hex_with_crlf() {
    let mut img = image::RgbaImage::new(2, 2);
    for y in 0..2u32 {
        for x in 0..2u32 {
            img.put_pixel(x, y, image::Rgba([255, 0, 0, 255]));
        }
    }
    let out = encode_btt_tft(&img);
    assert_eq!(
        out, ";00020002\r\n;F800F800\r\n;F800F800\r\n",
        "2x2 all-red BTT_TFT frame mismatch: {out:?}"
    );
}

#[test]
fn btt_tft_rgb565_packing_known_pixels() {
    // row-major, width=2: (255,0,0) (0,255,0) / (0,0,255) (255,255,255)
    let pixels = [
        (255, 0, 0, 255),
        (0, 255, 0, 255),
        (0, 0, 255, 255),
        (255, 255, 255, 255),
    ];
    let img = rgba_from_rows(2, 2, &pixels);
    let out = encode_btt_tft(&img);
    assert_eq!(
        out, ";00020002\r\n;F80007E0\r\n;001FFFFF\r\n",
        "known-pixel BTT_TFT frame mismatch: {out:?}"
    );
}

#[test]
fn btt_tft_every_line_ends_with_crlf() {
    let mut img = image::RgbaImage::new(3, 3);
    for y in 0..3u32 {
        for x in 0..3u32 {
            img.put_pixel(x, y, image::Rgba([(x * 40) as u8, (y * 40) as u8, 10, 255]));
        }
    }
    let out = encode_btt_tft(&img);
    // Split on \n; each segment (except a possible trailing empty one) must end
    // with \r (i.e. the \r\n pair). No bare \n anywhere.
    assert!(!out.contains("\n\r"), "no CRLF reversal allowed: {out:?}");
    let segments: Vec<&str> = out.split('\n').collect();
    // 3 data rows + 1 header = 4 lines, each \r\n => 4 splits with trailing "".
    for seg in &segments[..segments.len() - 1] {
        assert!(
            seg.ends_with('\r'),
            "every line must end with CRLF; got segment: {seg:?}"
        );
    }
    assert!(out.ends_with("\r\n"), "output must end with CRLF: {out:?}");
}

#[test]
fn btt_tft_no_vertical_flip() {
    // Row 0 (top): red, red. Row 1 (bottom): blue, blue.
    let pixels = [
        (255, 0, 0, 255),
        (255, 0, 0, 255),
        (0, 0, 255, 255),
        (0, 0, 255, 255),
    ];
    let img = rgba_from_rows(2, 2, &pixels);
    let out = encode_btt_tft(&img);
    assert_eq!(
        out, ";00020002\r\n;F800F800\r\n;001F001F\r\n",
        "no-flip BTT_TFT frame mismatch: {out:?}"
    );
    // Red row must appear before the blue row (source order, not flipped).
    let red = ";F800F800\r\n";
    let blue = ";001F001F\r\n";
    let red_pos = out.find(red).expect("red row missing");
    let blue_pos = out.find(blue).expect("blue row missing");
    assert!(
        red_pos < blue_pos,
        "red (top) row must precede blue (bottom) row; order flipped: {out:?}"
    );
}
