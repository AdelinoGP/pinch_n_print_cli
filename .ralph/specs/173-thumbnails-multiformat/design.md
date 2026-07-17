# Design: 173-thumbnails-multiformat

## Controlling Code Paths

- Primary code path: `crates/slicer-runtime/src/pipeline.rs::run_postpass_with_thumbnail` (reads `thumbnail_path` from raw config, validates `PNG_MAGIC` at `pipeline.rs:413`, wraps the serializer) → `crates/slicer-gcode/src/serialize.rs::ThumbnailAwareSerializer` (`serialize.rs:490-530`, injects the block after `HEADER_BLOCK_END`) → `crates/slicer-gcode/src/thumbnail.rs::serialize_thumbnail_block` (`thumbnail.rs:43`, current bare-base64 format, hand-rolled `base64_encode` at `thumbnail.rs:10`).
- Neighboring tests/fixtures: `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` (registered as `mod gcode_header_thumbnail_config_blocks_tdd;` in `tests/integration/main.rs:18`) — `thumbnail_roundtrip_matches_input` (line 584) and `thumbnail_base64_chunking_orca_parity` (~line 621) assert the old 76-col bare format; `slicer-gcode` integration tests are standalone top-level files (no aggregator).
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

- Host-side only: no file under `modules/`, `crates/slicer-schema/wit/`, `slicer-sdk`, `slicer-macros`, or `slicer-ir` is touched, so the guest-WASM staleness gate is not triggered by this packet (integration tests still need previously built guests on disk, as today).
- Thumbnails are pixel-space; the 1-unit=100nm slicer coordinate system does not apply — no `from_mm`/`mm_to_units` conversions anywhere in this packet.
- Config key strings snake_case: the key is `thumbnails` (single word; matches Orca's option name).
- Ported codec files carry the standard porting header from `docs/ORCASLICER_ATTRIBUTION.md`; citations by file + function only, never line numbers.

## Code Change Surface

- Selected approach: keep `--thumbnail` single-PNG; move all fan-out into `slicer-gcode`. New `ThumbnailFormat`/`ThumbnailSpec` model + `parse_thumbnails_key`; a pure `render_thumbnail_entries(source_png, specs) -> Result<Vec<RenderedThumbnail>, ThumbnailError>` doing decode/resize/encode; `serialize_thumbnail_block` re-shaped to take rendered entries and emit inner framing at 78 cols. `ThumbnailAwareSerializer` stores `Vec<RenderedThumbnail>` instead of raw bytes; `run_postpass_with_thumbnail` parses the key (default: one PNG entry at source size, source bytes passed through) and calls the renderer, converting `ThumbnailError` to the existing `PostpassError::GCodeSerialization`.
- Exact symbols (all net-new in `slicer-gcode` unless noted):
  - `pub enum ThumbnailFormat { Png, Jpg, Qoi, BttTft, ColPic }` with `pub fn tag(&self) -> &'static str` returning `thumbnail` / `thumbnail_JPG` / `thumbnail_QOI` / `thumbnail_BIQU` / `thumbnail_QIDI` (`thumbnail.rs`).
  - `pub struct ThumbnailSpec { pub width: u32, pub height: u32, pub format: ThumbnailFormat }` (`thumbnail.rs`).
  - `pub fn parse_thumbnails_key(value: &str) -> Result<Vec<ThumbnailSpec>, ThumbnailError>` (`thumbnail.rs`) — mirrors canonical `make_and_check_thumbnail_list` (`Thumbnails.cpp`): split on `,`, then `x`, then `/`; EXT upper-cased; accepted EXTs PNG/JPG/JPEG/QOI/BTT_TFT/COLPIC.
  - `pub enum ThumbnailError` (thiserror; variants for malformed spec entry — carrying the offending entry text — decode failure, encode failure) (`thumbnail.rs`).
  - `pub struct RenderedThumbnail { pub format: ThumbnailFormat, pub width: u32, pub height: u32, pub body: ThumbnailBody }` where `ThumbnailBody` is `Base64(String)` (PNG/JPG/QOI payload bytes pre-encoded) or `Raw(String)` (ColPic / BTT_TFT self-framed text) (`thumbnail.rs`).
  - `pub fn render_thumbnail_entries(source_png: &[u8], specs: &[ThumbnailSpec]) -> Result<Vec<RenderedThumbnail>, ThumbnailError>` (`thumbnail.rs`) — decodes once, resizes per spec (`image::imageops::resize`, `FilterType::CatmullRom`), encodes per format; PNG spec matching source dimensions passes source bytes through.
  - `pub(crate) fn encode_colpic(rgba: &image::RgbaImage) -> Result<String, ThumbnailError>` in new `crates/slicer-gcode/src/thumbnail_colpic.rs` — port of canonical `ColPic_EncodeStr` + `ColPicEncode` + `Byte8bitEncode` + `ADList0` (`Thumbnails.cpp`); applies the 512px aspect-preserved cap of canonical `compress_thumbnail_colpic`; frames output as `;gimage:` first chunk / `;simage:` subsequent chunks.
  - `pub(crate) fn encode_btt_tft(rgba: &image::RgbaImage) -> String` in new `crates/slicer-gcode/src/thumbnail_btt.rs` — port of canonical `compress_thumbnail_btt_tft` + `get_hex` + `rjust` (`Thumbnails.cpp`); `;<WWWW><HHHH>` 4-hex-digit header, RGB565 rows, `\r\n` endings.
  - `serialize_thumbnail_block` signature change (existing, `thumbnail.rs:43`): `pub fn serialize_thumbnail_block(entries: &[RenderedThumbnail]) -> String` — outer sentinels retained; per Base64 entry `; <tag> begin <W>x<H> <len>` + `; `-prefixed lines wrapped at `MAX_ROW_LENGTH = 78` + `; <tag> end` (canonical `export_thumbnails_to_file`, `Thumbnails.hpp`); Raw entries spliced verbatim between sentinels.
  - `ThumbnailAwareSerializer::new` second parameter becomes `Option<Vec<RenderedThumbnail>>` (existing, `serialize.rs:497`).
  - Existing hand-rolled `base64_encode` (`thumbnail.rs:10`) is retained and reused — no `base64` runtime dependency added.
- Dependency choice (recorded per plan): `image` crate, `default-features = false, features = ["png", "jpeg", "qoi"]`, added to `crates/slicer-gcode/Cargo.toml`. License MIT OR Apache-2.0 — compatible with this AGPLv3 project. Chosen over `png`+`jpeg-encoder`+`qoi` separately (three deps, hand-rolled resize) and over `zune-image` (less mature qoi encode surface); `image` provides decode, `imageops::resize`, and all three encoders behind one API.
- Rejected alternatives: (a) one CLI flag per size/format (fork ticket 011 shape) — rejected by user decision, fork renders one high-res PNG; (b) doing resize/encode in `pnp-cli` — rejected: the `thumbnails` key arrives via raw_config/3MF inside the runtime, and library users of `run_slice` must get identical output; (c) emitting BTT/ColPic outside the outer sentinels exactly like Orca — rejected: existing consumers/tests key on the outer sentinels, and the plan mandates retaining them.

## Files in Scope (read + edit)

Six files exceed the 3-primary guidance; justified: two are net-new attribution-header codec ports that cannot live in `thumbnail.rs` (per-file attribution requirement), one is a manifest line, and one is a test rewrite mandated by the plan. Steps keep ≤3 edits each.

- `crates/slicer-gcode/src/thumbnail.rs` - role: wire format + spec model + parser + renderer; expected change: rewrite around new symbols above.
- `crates/slicer-gcode/src/thumbnail_colpic.rs` (new) - role: ColPic port; expected change: created with attribution header.
- `crates/slicer-gcode/src/thumbnail_btt.rs` (new) - role: BTT_TFT port; expected change: created with attribution header.
- `crates/slicer-gcode/src/serialize.rs` - role: `ThumbnailAwareSerializer` payload type; expected change: `Vec<u8>` → `Vec<RenderedThumbnail>` (plus `lib.rs` re-exports and Cargo.toml `image` dep as secondary edits).
- `crates/slicer-runtime/src/pipeline.rs` - role: `run_postpass_with_thumbnail` wiring; expected change: parse `thumbnails` key, call renderer, map errors.
- `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` - role: roundtrip/chunking tests; expected change: rewrite AC-10/AC-11 to parse inner framing + 78 cols; add multi-entry and JPG/QOI tests.

## Read-Only Context

- `crates/slicer-gcode/src/serialize.rs` - lines `480-535` only - purpose: `ThumbnailAwareSerializer` injection points (HEADER_BLOCK_END splice).
- `crates/slicer-runtime/src/pipeline.rs` - lines `412-480` only - purpose: thumbnail extraction, `PNG_MAGIC`, effective-config build, serializer wrap.
- `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` - lines `140-260` and `580-650` only - purpose: `slice_to_gcode` helper and the two format-asserting tests.
- `docs/ORCASLICER_ATTRIBUTION.md` - full (short) - purpose: header text.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load
- `crates/pnp-cli/**` - no CLI change in this packet; do not browse
- `modules/**`, `crates/slicer-schema/wit/**` - untouched; do not browse

## Expected Sub-Agent Dispatches

- Question: verbatim algorithm of `ColPic_EncodeStr`, `ColPicEncode`, `Byte8bitEncode`, `ADList0` (palette build, RLE scheme, chunking into `;gimage:`/`;simage:`); scope: `OrcaSlicerDocumented/src/libslic3r/GCode/Thumbnails.cpp`; return: `SNIPPETS` (≤3 × 30 lines) + `SUMMARY`; purpose: Step 3 port.
- Question: exact line/header format of `compress_thumbnail_btt_tft` (`get_hex` packing, `rjust` width, `\r\n` placement, `;WWWWHHHH` header) and of `export_thumbnails_to_file`'s begin/end lines incl. `max_row_length`; scope: `OrcaSlicerDocumented/src/libslic3r/GCode/Thumbnails.cpp` + `.hpp`; return: `SNIPPETS`; purpose: Steps 2 and 4.
- Question: does `make_and_check_thumbnail_list` accept `JPEG` as alias, and how are whitespace/empty entries handled?; scope: `OrcaSlicerDocumented/src/libslic3r/GCode/Thumbnails.cpp`; return: `FACT`; purpose: Step 1 parser edge cases.
- Question: locate the THUMBNAIL_BLOCK / CONFIG_BLOCK serialization section heading in `docs/02_ir_schemas.md`; scope: `docs/02_ir_schemas.md`; return: `LOCATIONS`; purpose: Step 6 doc edit anchor.

## Data and Contract Notes

- IR/manifest contracts: none touched. `thumbnails` is a raw-config key (string), never a module-manifest key; it remains in CONFIG_BLOCK (unlike invocation-time `thumbnail_path`, stripped at `pipeline.rs:456`).
- WIT boundary: none.
- Determinism: rendering is pure (same PNG + same specs → identical bytes); `image` resize is deterministic. Entry order in the block = order of specs in the `thumbnails` string (default entry first when key absent).
- Fork-facing contract (the deviation to flag): fork renders ONE high-res top-down PNG and passes it via `--thumbnail`; requested sizes/formats travel in the `thumbnails` config key; PNP owns resize/transcode. Recorded in docs/02 note + `D-173-THUMBNAIL-SINGLE-PNG`.

## Locked Assumptions and Invariants

- Outer `; THUMBNAIL_BLOCK_START` / `; THUMBNAIL_BLOCK_END` sentinels are retained around all entries (PNP extension over canonical Orca output; existing tests and any fork parsing depend on them).
- Source PNGs are top-down; transcoders never flip rows (locked by AC-6).
- Base64 wrap width is 78 (canonical `export_thumbnails_to_file` `max_row_length`); the old 76 is dead.
- Tag strings are byte-exact to Orca's `tag()` overrides, including the mixed-case `thumbnail_JPG`/`thumbnail_QOI`/`thumbnail_BIQU`/`thumbnail_QIDI`.

## Risks and Tradeoffs

- ColPic is the riskiest port (palette + RLE state machine); mitigated by unit tests on tiny known images and delegated verbatim snippets of the canonical functions.
- `image` adds compile time to `slicer-gcode`; accepted for one audited, dual-licensed dep versus three.
- Signature change of `serialize_thumbnail_block` / `ThumbnailAwareSerializer::new` breaks any out-of-tree caller; in-tree callers are exactly the two listed (verified via grep — only `serialize.rs` and `lib.rs` re-export reference `serialize_thumbnail_block`).
- Textual-merge risk with draft packet 171 in `serialize.rs` (different functions); whichever lands second rebases trivially.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (ColPic port)
- Highest-risk dispatch and required return format: ColPic algorithm extraction — `SNIPPETS` (≤3 × 30 lines) + `SUMMARY`; reject anything larger and redispatch per-function.

## Open Questions

- `[FWD]` Does canonical `make_and_check_thumbnail_list` treat `JPEG` as an alias of `JPG` and skip empty entries? Resolve via the Step-1 FACT dispatch; parser must match whatever canonical accepts.
- `[FWD]` Exact `image` version pin (0.25.x at authoring time) — pick the latest 0.25 patch at implementation and record it in the Cargo.toml edit.
