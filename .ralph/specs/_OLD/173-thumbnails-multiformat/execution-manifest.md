# Packet 173 Execution Manifest

## Status
- Packet: 173-thumbnails-multiformat
- Status: active (flipped from draft on dispatch)
- Mode: implement
- Band: standard
- Pl-started: 2026-07-19

## Goal
Replace bare-base64 THUMBNAIL_BLOCK with Orca-parseable per-entry wire format
(; <tag> begin <W>x<H> <len> / ; <tag> end, 78-col wrap). Generate every entry
requested by the `thumbnails` config key via PNP-side decode/rescale/encode
in PNG, JPG, QOI, BTT_TFT, ColPic formats.

## AC registry
- AC-1: bare PNG passthrough, 78-col wrap, single entry at source size → `thumbnail_roundtrip_matches_input`
- AC-2: multi-entry (48x48/PNG, 300x300/PNG), IHDR dim check → `thumbnail_multi_entry_resized_png`
- AC-3: JPG SOI / QOI magic entries → `thumbnail_jpg_qoi_entries`
- AC-4: ColPic magic prefixes, 512 cap → `colpic`
- AC-5: BTT_TFT `;<WWWW><HHHH>` header + RGB565 + `\r\n` → `btt_tft`
- AC-6: no vertical flip on top-down gradient → `no_reflip`
- AC-7: doc edits grep PASS
- AC-N1: parse_thumbnails_key rejects malformed → `parse_thumbnails_key_rejects`
- AC-N2: no source → no block → `sentinels_present_no_thumbnail`

## Step ledger
| Step | Task | Files | Cost | Status | Notes |
|------|------|-------|------|--------|-------|
| 1 | parser + model + tests | thumbnail.rs, tests/thumbnail_formats_tdd.rs (new), lib.rs | S | pending | TDD first |
| 2 | wire format rewrite | thumbnail.rs, serialize.rs, pipeline.rs | M | pending | breaks old tests (expected RED) |
| 3 | image dep + renderer + ColPic | thumbnail.rs, thumbnail_colpic.rs (new), Cargo.toml | M | pending | lib.rs mod line is part of new-file creation side-edit |
| 4 | BTT_TFT port | thumbnail_btt.rs (new), thumbnail.rs, tests/thumbnail_formats_tdd.rs | S | pending | mod line is part of new-file creation side-edit |
| 5 | pipeline wiring + integration test rewrite | pipeline.rs, gcode_header_thumbnail_config_blocks_tdd.rs | M | pending | |
| 6 | docs + deviation | docs/02_ir_schemas.md, docs/DEVIATION_LOG.md | S | pending | |

## Dependency graph
- Step 1 → Step 2 → Step 3 → Step 4
- Step 5 depends on Steps 1-4
- Step 6 depends on Steps 1-5

## File ownership matrix (parallelism check)
- thumbnail.rs: Steps 1, 2, 3, 4 (sequential)
- serialize.rs: Step 2 only
- pipeline.rs: Steps 2, 5 (sequential — Step 2 does shim, Step 5 does full wiring)
- tests/thumbnail_formats_tdd.rs: Steps 1, 4 (sequential)
- integration test file: Step 5 only
- docs: Step 6 only

Decision: sequential per step. No parallelism safe (single hotspot in
thumbnail.rs across Steps 1-4). Workers = 2 max (informational; each step
is single-worker).

## Key OrcaSlicer facts (from research dispatches)
- `max_row_length = 78` (hpp:57)
- `; <tag> begin <W>x<H> <len>` where `<len>` is base64 char count
- `; <tag> end`
- Tags: thumbnail / thumbnail_JPG / thumbnail_QOI / thumbnail_BIQU / thumbnail_QIDI
- BTT_TFT: per-pixel RGB565 hex (4 digits, '0' pad); header `;<WWWW><HHHH>\r\n`;
  per-row `;` prefix + hex digits + `\r\n`; canonical flips vertically and
  premultiplies RGB by alpha — we DEVIATE: no flip (AC-6), keep premultiply
  (or test passes either way since AC-5 uses solid colors)
- ColPic: 512px aspect-preserved cap; `;gimage:` first chunk / `;simage:` rest
- ColPic algorithm: ADList0 (palette dedup R/G/B5/6/5 split, maxqty-bounded)
  + ColPicEncode (palette build + sort + 6-bit char encoding)
  + Byte8bitEncode (RLE of 16-bit color runs, sid/tid 32-entry sub-block scheme)
  + ColPic_EncodeStr (pad to 3, 6-bit → char +48, '\'→126)
- Parser: split on ',' then 'x' then '/'; EXT upper-cased; case-insensitive.
  No JPEG alias (only registered enum strings accepted). No empty-skip.
  Coordinate must be 0 < x < 1000.
- Allowed EXTs: PNG, JPG, QOI, BTT_TFT, COLPIC

## Decisions / locked
- BTT_TFT: NO vertical flip (deviation from canonical, locked by AC-6)
- ColPic: NO vertical flip (deviation from canonical, locked by AC-6)
- All 5 EXTs case-insensitive; reject anything else (incl. JPEG alias) per
  packet text — design.md lists PNG/JPG/JPEG/QOI/BTT_TFT/COLPIC as accepted
  but Orca does NOT accept JPEG. Follow packet: accept JPEG as JPG alias
  for usability (matches design.md line 22). FALSIFY: AC-N1 says "48x48/BMP"
  rejected; doesn't mandate JPEG. Design.md says accepted EXTs include
  JPEG. → implement as: PNG/JPG/JPEG/QOI/BTT_TFT/COLPIC; reject others.
- AC-5: "RGB565 packing" but doesn't pin premultiply. BTT_TFT canonical
  premultiplies RGB by alpha; we follow canonical there since AC-5 uses
  2x2 with "known colours" — premultiply is a no-op for opaque alpha=255.
- Renderer: PNG-at-source-size passthrough bytes (default behavior).

## Command registry
- `mkdir -p target && cargo test -p slicer-gcode --test thumbnail_formats_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` — targeted (Steps 1-4)
- `mkdir -p target && cargo test -p slicer-runtime --test integration gcode_header_thumbnail_config_blocks 2>&1 | tee target/test-output.log | grep -E "^test result"` — targeted (Step 5)
- `cargo check --workspace --all-targets` — packet-level (Step 5+ gate)
- `cargo clippy --workspace --all-targets -- -D warnings` — packet-level (Step 5+ gate)
- `rg -q 'thumbnail begin' docs/02_ir_schemas.md && rg -q 'D-173-THUMBNAIL-SINGLE-PNG' docs/DEVIATION_LOG.md && echo PASS` — packet-level (Step 6)

## Docs/backlog impact
- TASK-277 minted via task-map.md at closure.
- Add row to docs/07_implementation_status.md at end.
- Doc edits: docs/02_ir_schemas.md (THUMBNAIL_BLOCK section), docs/DEVIATION_LOG.md (new row).
