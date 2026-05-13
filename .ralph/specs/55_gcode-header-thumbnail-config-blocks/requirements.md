# Requirements: 55_gcode-header-thumbnail-config-blocks

## Packet Metadata

- Grouped task IDs:
  - `TASK-156` — Emit OrcaSlicer-parity `HEADER_BLOCK`, extrusion-width comments, and `CONFIG_BLOCK` envelope in the final `.gcode` file.
  - `TASK-157` — Accept an external PNG via a new `--thumbnail <path>` slicer-cli flag and emit it as `THUMBNAIL_BLOCK`; omit the block when the flag is absent.
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

PinchAndPrint's final `.gcode` file is a bare stream of motion commands with the OrcaSlicer-compatible in-body comments (`;TYPE:`, `;LAYER_CHANGE`, `;Z:`, `;HEIGHT:` — completed by TASK-119) but no surrounding envelope:

- No `HEADER_BLOCK_START..HEADER_BLOCK_END` carrying layer count, filament diameter/density, max Z, and filament order.
- No `THUMBNAIL_BLOCK_START..THUMBNAIL_BLOCK_END` carrying a Base64-encoded PNG preview.
- No `; <line_width_key> = <value>` extrusion-width comments after the header.
- No `CONFIG_BLOCK_START..CONFIG_BLOCK_END` trailing dump of the effective configuration.

Consequence: downstream tooling (Octoprint, Bambu Handy, Klipper UI, slicer file-managers) cannot show previews or print stats, and a `.gcode` file cannot be re-sliced or audited because the configuration that produced it is not recoverable from the file. This packet closes that gap with OrcaSlicer-byte-format-parity for sentinels, line prefixes, and key spellings.

This packet does NOT reopen or supersede any prior packet. TASK-119 (live in-body comment contract) is left intact; this packet adds envelope structure around it. Predecessor packets 52, 53, 54 modify `gcode_emit.rs` at disjoint sites (feedrate, cooling, skirt/brim, M82/M83) and are unaffected.

## In Scope

- New TDD test `crates/slicer-host/tests/gcode_header_thumbnail_config_blocks_tdd.rs` covering every AC and negative case.
- Four new config keys registered in `crates/slicer-host/src/config_schema.rs`:
  - `filament_diameter` (f32, default `1.75`).
  - `filament_density` (f32, default `1.24`, PLA-aligned with OrcaSlicer).
  - `max_z_height` (f32, default `256.0` — printer build-volume Z; emitted as a derived value when the slice's actual top Z is smaller).
  - `thumbnail_path` (String, default `""` — empty means no thumbnail block).
- Any extrusion-width key required by AC4 that is not already registered by an existing packet, with defaults matching OrcaSlicer's `0.4` mm nozzle profile.
- Four new helpers in `crates/slicer-host/src/gcode_emit.rs`:
  - `serialize_header_block(metadata: &PrintMetadata, cfg: &ConfigView, max_z_mm: f32) -> String`.
  - `serialize_width_comments(cfg: &ConfigView) -> String`.
  - `serialize_thumbnail_block(png_bytes: &[u8]) -> String` with PNG-magic validation, OrcaSlicer-parity Base64 chunking (≤ 76 chars per line, `; ` prefix).
  - `serialize_config_block(cfg: &ConfigView) -> String` — iterates the effective `ConfigView` in deterministic key-sorted order and emits `; key = value` lines, bracketed by the sentinel pair.
- Wiring all four into `DefaultGCodeSerializer::serialize_gcode()` so HEADER + width comments + (optional) THUMBNAIL appear before any motion line, and CONFIG_BLOCK appears after the last motion line.
- New `--thumbnail <path>` flag on `slicer-cli` (`crates/slicer-host/src/cli.rs`); the parser writes `("thumbnail_path", ConfigValue::String(path))` into `config_source` before `run_pipeline_with_raw_config()`.
- Filesystem read + PNG-magic check happen inside the serializer (or a helper invoked just before serialization) so a missing/invalid file fails the run with a clear diagnostic and produces no `.gcode`.

## Out of Scope

- Generating a thumbnail (no GL renderer, no software rasterizer). Only ingestion of a user-supplied PNG.
- Multi-thumbnail support (OrcaSlicer's `thumbnails = "48x48/PNG,300x300/PNG"`).
- Non-PNG formats (JPG / QOI / BTT_TFT / ColPic).
- Resizing or re-encoding the supplied PNG.
- Computing or emitting `estimated_print_time_s`; the field is already populated (currently 0) by the existing pipeline and is not part of the four required header lines.
- Modifying any predecessor packet's edits or the `GCodeIR` schema.
- Cross-platform path encoding edge cases beyond UTF-8 (Windows backslashes in `--thumbnail` are accepted via `std::path::Path`).

## Authoritative Docs

- `docs/01_system_architecture.md` — delegate SUMMARY (finalization role only).
- `docs/02_ir_schemas.md` — load directly the `PrintMetadata` (~`:1634-:1660`), `LayerCollectionIR` (~`:1524-:1540`), and `ConfigView` (~`:467-:520`) sections; file is large so delegate any wider read.
- `docs/03_wit_and_manifest.md` — load directly the config-schema-validation section; delegate if you cannot locate it in under 60 lines.
- `docs/07_implementation_status.md` — delegate insertion of TASK-156 + TASK-157 rows; never load the whole file.
- `docs/08_coordinate_system.md` — load directly the unit-conversion lines (≤ 30 lines) for the 100-nm → mm conversion when computing `max_z_height`.

## OrcaSlicer Reference Obligations

All reads delegated. The implementation borrows OrcaSlicer's exact wire format and nothing else (no rendering, no config schema, no GUI logic).

- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` lines `2644`, `2704`, `2715`, `2728`, `3588`, `3604` — `HEADER_BLOCK_*` / `CONFIG_BLOCK_*` sentinel literals and the comment line prefix.
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` lines `2650`, `2675-2681`, `2688`, `2695-2701` — header field names and value formatting (`total layer number`, `filament_diameter`, `filament_density`, `max_z_height`, `filament`).
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` lines `2752-2760` — extrusion-width comment format and the list of width keys emitted.
- `OrcaSlicerDocumented/src/libslic3r/GCode/Thumbnails.hpp` lines `111`, `129` — `THUMBNAIL_BLOCK_*` sentinels, line prefix, Base64 column width.
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp::append_full_config()` line `5599` + surrounding ≤ 30 lines — iteration order, separator (` = `), and skip rules.

Deliberately NOT borrowed: OrcaSlicer's full PrintConfig key set (we emit only what PinchAndPrint's `ConfigView` actually carries), OrcaSlicer's GL thumbnail pipeline (no rendering), OrcaSlicer's multi-format thumbnail dispatcher (PNG only).

## Acceptance Summary

- Positive cases: HEADER_BLOCK present with the four required field lines + filament order; extrusion-width comments after the header; THUMBNAIL_BLOCK present iff `--thumbnail` supplied and bytes round-trip Base64; CONFIG_BLOCK at file tail covers the effective `ConfigView`; block ordering is HEADER → width → THUMBNAIL → body → CONFIG.
- Negative cases: missing-sentinel file fails the test; `--thumbnail nonexistent.png` exits non-zero with `file not found`; `--thumbnail not_a_png.bin` exits non-zero with `invalid PNG magic`; empty `ConfigView` still emits sentinel pair with zero key lines; `; total layer number:` mismatching `LayerCollectionIR.layers.len()` fails the test.
- Measurable outcomes: every AC verification command in `packet.spec.md` returns exit 0; HEADER field count `grep -E '^; (total layer number|filament_diameter|filament_density|max_z_height):' <file> | wc -l == 4`; `grep -cE '^; (CONFIG|HEADER)_BLOCK_(START|END)' <file> == 4` (with no thumbnail) or `== 6` (with thumbnail, adding the two THUMBNAIL sentinels — actual count 4 + 2 = 6).
- Cross-packet impact: unblocks future thumbnail-rendering packet and future `.gcode` reproducibility / parity-diff packet. No predecessor packet is reopened.

## Verification Commands

- `cargo test -p slicer-host --test gcode_header_thumbnail_config_blocks_tdd` — primary.
- `cargo test -p slicer-host --test orca_comment_contract_tdd` — regression (envelope must not displace in-body comments).
- `cargo check --workspace` — type-check gate.
- `cargo clippy --workspace -- -D warnings` — lint gate.

All commands above are delegation-friendly: targeted, parseable, < 200 lines of output on success.

## Step Completion Expectations

Per implementation-plan.md steps; the rollup is restated here for quick scanning.

- Precondition: prior steps' postconditions hold; `cargo check --workspace` is green at entry.
- Postcondition: this step's verification dispatch returns FACT pass.
- Falsifying check: the step's targeted `cargo test` invocation returns non-zero, or the step's discovery dispatch returns a contradictory FACT.
- Files allowed to read: see `design.md` "Read-Only Context" and per-step lists in `implementation-plan.md`. Line ranges apply to any file > 300 lines (`gcode_emit.rs`, `slice_ir.rs`).
- Files allowed to edit (≤ 3 per step): see per-step list in `implementation-plan.md`.
- Expected sub-agent dispatches: five OrcaSlicer FACT dispatches (enumerated in `packet.spec.md`); one config-schema completeness LOCATIONS dispatch (Step 6); one `cargo test` FACT dispatch per step.
- Step context cost: all `S` except Step 5 (`--thumbnail` flag + PNG validation + Base64 chunking) which is `M`.

## Context Discipline Notes

- Large files in the read-only path requiring range-reads:
  - `crates/slicer-host/src/gcode_emit.rs` (~490 lines) — read `:370-:490` (serializer body) only.
  - `crates/slicer-ir/src/slice_ir.rs` (> 1600 lines) — read only the four small ranges enumerated under Authoritative Docs.
  - `docs/02_ir_schemas.md` — range-read or delegate.
- OrcaSlicer trees the implementer must NOT load directly: every `OrcaSlicerDocumented/` path. All five FACT dispatches are pre-shaped in `packet.spec.md`.
- Likely temptation reads (skip):
  - The full OrcaSlicer `GCode.cpp` (> 6000 lines) — only the five enumerated line ranges matter; everything else is body-emission logic this packet does not touch.
  - The full `slice_ir.rs` — only four struct definitions matter.
  - Predecessor packets 52/53/54's design.md / implementation-plan.md — confirmed disjoint by the predecessor SUMMARY dispatch; do not re-read them.
- Sub-agent return-format hints for the heaviest dispatches:
  - OrcaSlicer FACTs: ≤ 12 lines, no code blocks > 4 lines.
  - Config-schema completeness LOCATIONS (Step 6): ≤ 40 entries (key name + default + type per line).
  - `cargo test` runs: FACT pass/fail; SNIPPETS ≤ 20 lines on first failing assertion.
