---
status: implemented
packet: 173-thumbnails-multiformat
task_ids:
  - TASK-277
---

# 173-thumbnails-multiformat

## Goal

Replace the bare-base64 `THUMBNAIL_BLOCK` with OrcaSlicer's parseable per-entry wire format (`; <tag> begin <W>x<H> <len>` / `; <tag> end`, 78-col wrap) and generate every entry requested by the `thumbnails` config key ("WxH/EXT,...") from the single `--thumbnail` source PNG via PNP-side decode/rescale/encode in PNG, JPG, QOI, BTT_TFT, and ColPic formats.

## Problem Statement

The current `THUMBNAIL_BLOCK` (`crates/slicer-gcode/src/thumbnail.rs`, 59 lines) emits only outer sentinels plus bare 76-col `; <chunk>` base64 lines. No printer firmware or Orca-family parser can locate a thumbnail in it: they all key off the inner `; <tag> begin <W>x<H> <len>` / `; <tag> end` framing that canonical `export_thumbnails_to_file` (`Thumbnails.hpp`) writes at 78 columns. Additionally, PNP can only embed the one PNG handed to `--thumbnail` — there is no way to satisfy printers that require JPG (some Klipper screens), QOI (Prusa/Orca previews), BTT_TFT RGB565 hex, or QIDI ColPic. The user-decided contract (deviating from fork ticket 011): the fork renders **one** high-res PNG; PNP decodes it, rescales per `thumbnails` config entry, and encodes every requested format itself. The existing roundtrip test asserts the wrong (current) format and must be rewritten to parse the real one.

## Architecture Constraints

- Host-side only: no file under `modules/`, `crates/slicer-schema/wit/`, `slicer-sdk`, `slicer-macros`, or `slicer-ir` is touched, so the guest-WASM staleness gate is not triggered by this packet (integration tests still need previously built guests on disk, as today).
- Thumbnails are pixel-space; the 1-unit=100nm slicer coordinate system does not apply — no `from_mm`/`mm_to_units` conversions anywhere in this packet.
- Config key strings snake_case: the key is `thumbnails` (single word; matches Orca's option name).
- Ported codec files carry the standard porting header from `docs/ORCASLICER_ATTRIBUTION.md`; citations by file + function only, never line numbers.

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
