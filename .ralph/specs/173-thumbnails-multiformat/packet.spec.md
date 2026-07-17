---
status: draft
packet: 173-thumbnails-multiformat
task_ids:
  - TASK-277
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 173-thumbnails-multiformat

## Goal

Replace the bare-base64 `THUMBNAIL_BLOCK` with OrcaSlicer's parseable per-entry wire format (`; <tag> begin <W>x<H> <len>` / `; <tag> end`, 78-col wrap) and generate every entry requested by the `thumbnails` config key ("WxH/EXT,...") from the single `--thumbnail` source PNG via PNP-side decode/rescale/encode in PNG, JPG, QOI, BTT_TFT, and ColPic formats.

## Scope Boundaries

All changes are host-side: `crates/slicer-gcode` (wire format, `thumbnails` key parsing, resize/transcode, ColPic and BTT_TFT codec ports), `crates/slicer-runtime/src/pipeline.rs` (`run_postpass_with_thumbnail` wiring), and the existing roundtrip test. The CLI surface is untouched — `--thumbnail <png>` stays a single `Option<PathBuf>` (`crates/pnp-cli/src/main.rs:70-72`); this is a user-decided deviation from fork ticket 011 that the packet must flag in the fork-facing contract note (the fork now renders ONE high-res PNG, not one per size). No WIT, IR, module, or guest-WASM change.

## Prerequisites and Blockers

- Depends on: nothing (packet 55's `THUMBNAIL_BLOCK` plumbing is shipped and is extended in place).
- Unblocks: fork ticket 011 (printer-parseable thumbnails); any future printer-profile-driven thumbnail defaults.
- Activation blockers: none.

## Acceptance Criteria

- **AC-1. Given** a valid source PNG and no `thumbnails` config key, **when** the pipeline serializes G-code, **then** the region between `; THUMBNAIL_BLOCK_START` and `; THUMBNAIL_BLOCK_END` contains exactly one entry framed by `; thumbnail begin <W>x<H> <len>` and `; thumbnail end` (W/H = source PNG dimensions, `<len>` = base64 character count), with every base64 line `; `-prefixed and at most 78 base64 characters. | `mkdir -p target && cargo test -p slicer-runtime --test integration thumbnail_roundtrip_matches_input 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-2. Given** `thumbnails = "48x48/PNG,300x300/PNG"` in raw config plus a source PNG, **when** the pipeline serializes G-code, **then** the thumbnail region contains two `; thumbnail begin` entries with header dimensions `48x48` and `300x300`, and decoding each entry's base64 yields a PNG whose IHDR width/height equal the header dimensions. | `mkdir -p target && cargo test -p slicer-runtime --test integration thumbnail_multi_entry_resized_png 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-3. Given** `thumbnails = "64x64/JPG,64x64/QOI"`, **when** the pipeline serializes G-code, **then** the region contains one `; thumbnail_JPG begin 64x64 <len>` entry whose decoded payload starts with the JPEG SOI marker `0xFFD8` and one `; thumbnail_QOI begin 64x64 <len>` entry whose decoded payload starts with the QOI magic `qoif`. | `mkdir -p target && cargo test -p slicer-runtime --test integration thumbnail_jpg_qoi_entries 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-4. Given** a 4-pixel RGBA image and a `ColPic` spec, **when** `encode_colpic` runs, **then** its output begins with `;gimage:`, continuation chunks (when the payload exceeds one chunk) begin with `;simage:`, and a source larger than 512px is scaled down aspect-preserved so neither output dimension exceeds 512. | `mkdir -p target && cargo test -p slicer-gcode --test thumbnail_formats_tdd colpic 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-5. Given** a 2x2 RGBA image with known pixel colours and a `BttTft` spec, **when** `encode_btt_tft` runs, **then** the output starts with the `;<WWWW><HHHH>` header (4-hex-digit width then height), every data line carries RGB565 hex values matching the expected `((r>>3)<<11)|((g>>2)<<5)|(b>>3)` packing, and every line ends with `\r\n`. | `mkdir -p target && cargo test -p slicer-gcode --test thumbnail_formats_tdd btt_tft 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-6. Given** a source PNG whose rows encode a known top-down gradient, **when** it is transcoded to any target format, **then** the first output row corresponds to the first source row (no vertical flip is introduced). | `mkdir -p target && cargo test -p slicer-gcode --test thumbnail_formats_tdd no_reflip 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-7. Given** the docs edits land, **when** grepping the tree, **then** the fork-facing single-source-PNG contract note and the deviation row exist. | `rg -q 'thumbnail begin' docs/02_ir_schemas.md && rg -q 'D-173-THUMBNAIL-SINGLE-PNG' docs/DEVIATION_LOG.md && echo PASS`

## Negative Test Cases

- **AC-N1. Given** a malformed `thumbnails` value (`"48x/PNG"`, `"48x48"`, or `"48x48/BMP"`), **when** `parse_thumbnails_key` runs, **then** it returns `Err` naming the offending entry verbatim, and the pipeline surfaces that error instead of emitting a thumbnail block. | `mkdir -p target && cargo test -p slicer-gcode --test thumbnail_formats_tdd parse_thumbnails_key_rejects 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N2. Given** no `--thumbnail` source (no `thumbnail_path` in raw config), **when** the pipeline serializes G-code — even with a `thumbnails` key present — **then** no `THUMBNAIL_BLOCK_START` sentinel appears (unchanged packet-55 behaviour). | `mkdir -p target && cargo test -p slicer-runtime --test integration sentinels_present_no_thumbnail 2>&1 | tee target/test-output.log | grep -E "^test result"`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `mkdir -p target && cargo test -p slicer-runtime --test integration gcode_header_thumbnail_config_blocks 2>&1 | tee target/test-output.log | grep -E "^test result"`

## Authoritative Docs

- `docs/02_ir_schemas.md` — delegated bounded lookup of the CONFIG_BLOCK / THUMBNAIL_BLOCK serialization section only.
- `docs/ORCASLICER_ATTRIBUTION.md` — direct read (short); exact porting header text for the ColPic / BTT_TFT codec files.
- `docs/07_implementation_status.md` — delegated; TASK-277 minted at closure via `task-map.md`.

## Doc Impact Statement (Required)

- `docs/02_ir_schemas.md` — THUMBNAIL_BLOCK section: document the per-entry wire format (five tags, 78-col wrap, outer sentinels retained) and the fork-facing single-source-PNG contract (fork renders ONE high-res PNG; sizes/formats come from the `thumbnails` config key; deviation from fork ticket 011) - `rg -q 'thumbnail begin' docs/02_ir_schemas.md`
- `docs/DEVIATION_LOG.md` — new row `D-173-THUMBNAIL-SINGLE-PNG` recording the fork-ticket-011 deviation (single source PNG + PNP-side resize instead of one fork render per size) - `rg -q 'D-173-THUMBNAIL-SINGLE-PNG' docs/DEVIATION_LOG.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/GCode/Thumbnails.cpp` — `ColPic_EncodeStr` (plus internal `ColPicEncode`, `Byte8bitEncode`, `ADList0`), `compress_thumbnail_btt_tft` (with `get_hex`, `rjust`), `compress_thumbnail_colpic` (512px cap, aspect preserved), the `compress_thumbnail` dispatch switch, and `make_and_check_thumbnail_list` (the `"WxH/EXT,..."` parser).
- `OrcaSlicerDocumented/src/libslic3r/GCode/Thumbnails.hpp` — `export_thumbnails_to_file` (the `; <tag> begin WxH len` / `; <tag> end` writer, `max_row_length = 78`) and the per-format `tag()` overrides (`thumbnail`, `thumbnail_JPG`, `thumbnail_QOI`, `thumbnail_BIQU`, `thumbnail_QIDI`).

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
