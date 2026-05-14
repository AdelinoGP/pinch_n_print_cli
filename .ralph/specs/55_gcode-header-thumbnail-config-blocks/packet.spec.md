---
status: draft
packet: 55_gcode-header-thumbnail-config-blocks
task_ids:
  - TASK-156   # new — HEADER_BLOCK + CONFIG_BLOCK + extrusion-width comments
  - TASK-157   # new — THUMBNAIL_BLOCK via --thumbnail CLI flag (OrcaSlicer-parity when flag absent)
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 55_gcode-header-thumbnail-config-blocks

## Goal

Emit the OrcaSlicer textual G-code envelope that PinchAndPrint currently omits, so downstream tooling (Octoprint, Bambu Handy, Klipper UI) can show previews / stats and the file is reproducible:

1. `HEADER_BLOCK_START` / `HEADER_BLOCK_END` enclosing layer count, filament diameter, filament density, max Z, and the ordered filament list — emitted at the top of every produced `.gcode` file.
2. `THUMBNAIL_BLOCK_START` / `THUMBNAIL_BLOCK_END` enclosing a Base64-encoded PNG, conditional on a new `--thumbnail <png-path>` slicer-cli flag (when the flag is absent the block is omitted, matching OrcaSlicer CLI behavior).
3. Per-role extrusion-width comments (e.g., `; outer_wall_line_width = 0.42`) immediately after `HEADER_BLOCK_END`, sourced from the effective `ConfigView`.
4. `CONFIG_BLOCK_START` / `CONFIG_BLOCK_END` appended at the end of the file, enclosing every key in the effective `ConfigView` (user-passed values merged with registered defaults) as `; key = value` lines.

The sentinel literals, line ordering, comment prefix style, and Base64 column wrapping must match OrcaSlicer's wire format exactly so unmodified downstream parsers consume the file.

## Scope Boundaries

- In scope:
  - New TDD test file `crates/slicer-host/tests/gcode_header_thumbnail_config_blocks_tdd.rs`.
  - Registering `filament_diameter`, `filament_density`, `max_z_height`, and `thumbnail_path` config keys in `crates/slicer-host/src/config_schema.rs` with OrcaSlicer-aligned defaults (1.75 mm, 1.24 g/cm³, 256.0 mm, empty-string respectively). Width keys already registered (`outer_wall_line_width` etc., as introduced by predecessor packets) are read but not re-registered; missing width keys required by AC4 are registered with OrcaSlicer-aligned defaults.
  - Three new emission helpers in `crates/slicer-host/src/gcode_emit.rs`: `serialize_header_block(&PrintMetadata, &ConfigView) -> String`, `serialize_width_comments(&ConfigView) -> String`, `serialize_config_block(&ConfigView) -> String`. Plus a thumbnail helper `serialize_thumbnail_block(png_bytes: &[u8]) -> String` that performs PNG-magic validation + 78-column Base64 chunking with `; ` prefix per OrcaSlicer's `Thumbnails.hpp` wire format.
  - Wiring all four into `DefaultGCodeSerializer::serialize_gcode()` so HEADER + width comments + thumbnail are emitted at file head, CONFIG_BLOCK at file tail.
  - New `--thumbnail <path>` flag on `slicer-cli` that injects `thumbnail_path` into `config_source` before `run_pipeline_with_raw_config()`.
- Out of scope:
  - Real per-print thumbnail rendering (no GL/software rasterizer); only external PNG ingestion via `--thumbnail`. Rendering becomes a follow-up packet if requested.
  - Multi-format thumbnails (JPG/QOI/BTT_TFT/ColPic). PNG only.
  - Multi-thumbnail OrcaSlicer config key (`thumbnails = "48x48/PNG,300x300/PNG"`). Single PNG only; the user-supplied file is emitted verbatim with no resizing.
  - Estimated print time computation for the header (`estimated_print_time_s` stays whatever the existing pipeline produces — usually 0).
  - Feedrate (packet 52), cooling fan (packet 53), skirt/brim + relative-extrusion (packet 54) emission semantics. This packet only adds envelope blocks around their output.
  - Modifying `GCodeIR` schema or any IR contract in `docs/02_ir_schemas.md`.

## Prerequisites and Blockers

- Depends on:
  - None at the source level. Predecessor packets 52/53/54 touch the same file (`gcode_emit.rs`) but at disjoint sites (feedrate `F` token, M82/M83 preamble, dispatch wiring). They do not need to land first.
- Unblocks:
  - Future "real thumbnail rendering" packet (would replace the `--thumbnail` flag's PNG-passthrough with a generated PNG; the wire-format helper is reused).
  - Future "OrcaSlicer parity gate" packet (file-level diff against reference `.gcode`).
- Activation blockers:
  - None at draft time. The two open questions in `design.md` (Q1 width-key authoritative list, Q2 ConfigValue → string formatter for CONFIG_BLOCK) are answerable during Step 2 / Step 6 via small dispatches and do not require external decisions.

## Acceptance Criteria

- **Given** a `slicer-cli` invocation that produces `out.gcode` without `--thumbnail`, **when** the file is scanned, **then** `HEADER_BLOCK_START`, `HEADER_BLOCK_END`, `CONFIG_BLOCK_START`, and `CONFIG_BLOCK_END` all appear exactly once and `THUMBNAIL_BLOCK_START` does not appear. | `cargo test -p slicer-host --test gcode_header_thumbnail_config_blocks_tdd -- sentinels_present_no_thumbnail --nocapture`
- **Given** a `slicer-cli` invocation that produces `out.gcode` with `--thumbnail $WORKSPACE_ROOT/resources/fake_thumb.png` (the committed 940×940 PNG, ≈132 KB), **when** the file is scanned, **then** `HEADER_BLOCK_START`, `THUMBNAIL_BLOCK_START`, `THUMBNAIL_BLOCK_END`, `CONFIG_BLOCK_START`, and `CONFIG_BLOCK_END` all appear exactly once. | `cargo test -p slicer-host --test gcode_header_thumbnail_config_blocks_tdd -- sentinels_present_with_thumbnail --nocapture`
- **Given** the produced header block, **when** parsed, **then** the four lines `; total layer number:`, `; filament_diameter:`, `; filament_density:`, and `; max_z_height:` each appear exactly once and each has a non-empty value after the colon. | `cargo test -p slicer-host --test gcode_header_thumbnail_config_blocks_tdd -- header_four_required_fields --nocapture`
- **Given** the produced header block emitted from a sliced fixture with exactly 12 layers, **when** the `; total layer number:` line is parsed, **then** its value equals `12`. | `cargo test -p slicer-host --test gcode_header_thumbnail_config_blocks_tdd -- header_layer_count_matches_sliced --nocapture`
- **Given** the produced header block emitted from a fixture whose highest layer Z is 9.8 mm, **when** the `; max_z_height:` line is parsed, **then** its numeric value equals `9.8` within 1e-3 mm. | `cargo test -p slicer-host --test gcode_header_thumbnail_config_blocks_tdd -- header_max_z_matches_top_layer --nocapture`
- **Given** the produced header block, **when** the `; filament:` (filament order) line is parsed, **then** it lists one tool index per used filament in first-use order, comma-separated, matching `PrintMetadata.filament_used_mm` non-zero entries. | `cargo test -p slicer-host --test gcode_header_thumbnail_config_blocks_tdd -- header_filament_order_matches_used --nocapture`
- **Given** the produced output after `HEADER_BLOCK_END`, **when** scanned for extrusion-width comments, **then** each of `outer_wall_line_width`, `inner_wall_line_width`, `sparse_infill_line_width`, `top_surface_line_width`, and `support_line_width` appears exactly once as `; <key> = <value>` with a numeric value > 0. | `cargo test -p slicer-host --test gcode_header_thumbnail_config_blocks_tdd -- width_comments_emitted --nocapture`
- **Given** a `slicer-cli` invocation with `--config user_keys.json` setting `layer_height = 0.16` and `sparse_infill_density = 22.0`, **when** `CONFIG_BLOCK_START..CONFIG_BLOCK_END` is parsed, **then** both lines `; layer_height = 0.16` and `; sparse_infill_density = 22` (or `22.0`) appear exactly once. | `cargo test -p slicer-host --test gcode_header_thumbnail_config_blocks_tdd -- config_block_includes_user_passed --nocapture`
- **Given** the produced `CONFIG_BLOCK`, **when** scanned, **then** every key present in the effective `ConfigView` (user-passed merged with registered defaults) appears exactly once as `; <key> = <value>`, with no duplicate keys and no key present outside the block. | `cargo test -p slicer-host --test gcode_header_thumbnail_config_blocks_tdd -- config_block_covers_effective_config --nocapture`
- **Given** a `--thumbnail $WORKSPACE_ROOT/resources/fake_thumb.png` invocation, **when** the bytes between `THUMBNAIL_BLOCK_START` and `THUMBNAIL_BLOCK_END` are stripped of the `; ` line prefix, concatenated, and Base64-decoded, **then** the result begins with the PNG magic `89 50 4E 47 0D 0A 1A 0A` and equals the raw bytes of `resources/fake_thumb.png` byte-for-byte (`assert_eq!(decoded, std::fs::read(workspace_path("resources/fake_thumb.png")).unwrap())`). | `cargo test -p slicer-host --test gcode_header_thumbnail_config_blocks_tdd -- thumbnail_roundtrip_matches_input --nocapture`
- **Given** a `--thumbnail $WORKSPACE_ROOT/resources/fake_thumb.png` invocation, **when** the Base64 payload is inspected, **then** every line between the sentinels begins with `; ` and the Base64 portion of each line is ≤ 76 characters (OrcaSlicer wraps at 76); the only permitted exception is the final line which may be shorter. | `cargo test -p slicer-host --test gcode_header_thumbnail_config_blocks_tdd -- thumbnail_base64_chunking_orca_parity --nocapture`
- **Given** the produced `out.gcode`, **when** scanned for block ordering, **then** the first `HEADER_BLOCK_START` byte-offset is strictly less than the first `;TYPE:` byte-offset, and the first `CONFIG_BLOCK_START` byte-offset is strictly greater than the last `;TYPE:` byte-offset. | `cargo test -p slicer-host --test gcode_header_thumbnail_config_blocks_tdd -- block_ordering_header_before_body_config_after --nocapture`

## Negative Test Cases

- **Given** an output file lacking any of `HEADER_BLOCK_START`, `HEADER_BLOCK_END`, `CONFIG_BLOCK_START`, or `CONFIG_BLOCK_END`, **when** validated, **then** the test fails. | `cargo test -p slicer-host --test gcode_header_thumbnail_config_blocks_tdd -- rejects_missing_sentinel_block --nocapture`
- **Given** a `--thumbnail nonexistent.png` invocation, **when** the slicer is run, **then** it exits non-zero with a clear `thumbnail_path: file not found` diagnostic on stderr and produces no `.gcode` output. | `cargo test -p slicer-host --test gcode_header_thumbnail_config_blocks_tdd -- rejects_missing_thumbnail_file --nocapture`
- **Given** a `--thumbnail <path>` invocation where the file is non-empty but lacks the PNG magic header (test materializes a 64-byte non-PNG file to `std::env::temp_dir()` at test start; no committed fixture), **when** the slicer is run, **then** it exits non-zero with a `thumbnail_path: invalid PNG magic` diagnostic and produces no `.gcode` output. | `cargo test -p slicer-host --test gcode_header_thumbnail_config_blocks_tdd -- rejects_non_png_thumbnail --nocapture`
- **Given** the effective `ConfigView` is empty (degenerate case), **when** the serializer runs, **then** `CONFIG_BLOCK_START` and `CONFIG_BLOCK_END` are still emitted with zero key lines between them (empty block, not a missing block). | `cargo test -p slicer-host --test gcode_header_thumbnail_config_blocks_tdd -- empty_config_view_still_emits_sentinels --nocapture`
- **Given** a `; total layer number:` line whose numeric value disagrees with `LayerCollectionIR.layers.len()`, **when** validated against the produced layers, **then** the test fails. | `cargo test -p slicer-host --test gcode_header_thumbnail_config_blocks_tdd -- rejects_layer_count_drift --nocapture`

## Verification

- `cargo test -p slicer-host --test gcode_header_thumbnail_config_blocks_tdd` — dispatch as FACT pass/fail; SNIPPETS on failure with the first failing assertion + ≤ 20 lines.
- `cargo test -p slicer-host --test orca_comment_contract_tdd` — regression; HEADER block must not break the existing `;LAYER_CHANGE`/`;TYPE:`/`;Z:`/`;HEIGHT:` sequence.
- `cargo check --workspace`
- `cargo clippy --workspace -- -D warnings`

## Authoritative Docs

- `docs/01_system_architecture.md` — finalization stage and serializer role; delegate a SUMMARY.
- `docs/02_ir_schemas.md` — `PrintMetadata`, `LayerCollectionIR`, `ConfigView`; load directly only the ≤ 60-line sections covering these three structs (file is large; delegate ranges if uncertain).
- `docs/03_wit_and_manifest.md` — config schema and manifest validation rules; load directly only the schema-validation section; delegate if > 300 lines around it.
- `docs/07_implementation_status.md` — delegate; insert new rows `TASK-156` (header + width + config blocks) and `TASK-157` (--thumbnail flag). Do NOT reopen TASK-119 series (already covers in-body `;TYPE:`/`;LAYER_CHANGE` comments, which this packet leaves untouched).
- `docs/08_coordinate_system.md` — load directly the unit-conversion section (≤ 30 lines) only when emitting `max_z_height` in millimeters from internal 100-nm units.

## OrcaSlicer Reference Obligations

All reads delegated; never load OrcaSlicer source into the implementer's context.

- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — FACT, ≤ 12 lines: exact sentinel literals (`HEADER_BLOCK_START`, `HEADER_BLOCK_END`, `CONFIG_BLOCK_START`, `CONFIG_BLOCK_END`) and the line prefix (`;` vs `; `) at lines 2644 / 2704 / 2715 / 2728 / 3588 / 3604.
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — FACT, ≤ 10 lines: exact header field names (`total layer number`, `filament_diameter`, `filament_density`, `max_z_height`, `filament`) and their value-formatting (single value vs comma-separated list) at lines 2650 / 2675–2681 / 2688 / 2695–2701.
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — FACT, ≤ 8 lines: extrusion-width comment format and the exact set of width keys emitted at lines 2752–2760.
- `OrcaSlicerDocumented/src/libslic3r/GCode/Thumbnails.hpp` — FACT, ≤ 12 lines: `THUMBNAIL_BLOCK_START` / `THUMBNAIL_BLOCK_END` literals, line-prefix format, Base64 column width, and the `gcode_thumbnails_*` token spelling at lines 111 / 129.
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp::append_full_config()` — FACT, ≤ 8 lines: how config keys are iterated, the separator between key and value (` = ` vs `=`), and whether comments are skipped (line 5599 + surrounding ≤ 30 lines).

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`

## Context Discipline Note

- `OrcaSlicerDocumented/` MUST be delegated. The packet's parity claims rest on five small FACT dispatches enumerated above; no other OrcaSlicer file should be opened.
- `crates/slicer-host/src/gcode_emit.rs` is > 480 lines — range-read `:370-:490` (the serializer body) only. The new helpers append above and inside `serialize_gcode()`; full-file reads are not needed.
- `crates/slicer-host/src/config_schema.rs` is ~170 lines — load directly. New key registrations follow the existing pattern at `:121` and `:171`.
- `crates/slicer-ir/src/slice_ir.rs` is > 1600 lines — range-read `:467-:520` (`ConfigView`), `:1218-:1230` (`Point3WithWidth`), `:1524-:1540` (`LayerCollectionIR.z`), `:1634-:1660` (`PrintMetadata`). Never load full file.
- `crates/slicer-host/src/main.rs` and `cli.rs` are short — load directly for `--thumbnail` flag wiring.
- Sub-agent return formats:
  - OrcaSlicer FACTs (5 dispatches above): ≤ 12 lines each, no code blocks > 4 lines.
  - `cargo test`: FACT pass/fail; SNIPPETS (≤ 20 lines) on first-failing-assertion.
  - Config-schema completeness check (Step 6): LOCATIONS list of every registered key, ≤ 40 entries.

### Test Fixture Convention

- **Valid PNG fixture** is the already-committed `resources/fake_thumb.png` (940×940 PNG, ≈132 KB; PNG magic verified). The test file resolves the absolute path via `concat!(env!("CARGO_MANIFEST_DIR"), "/../../resources/fake_thumb.png")` (manifest dir is `crates/slicer-host/`; two `..` segments reach workspace root). NO new PNG fixture is created.
- **Non-PNG fixture** for `rejects_non_png_thumbnail` is materialized inline at test runtime: write 64 bytes (e.g., `b"this is not a png\n"` zero-padded) to a fresh path under `std::env::temp_dir()`. No committed binary fixture.
- The thumbnail at 940×940 will Base64-encode to ≈176 KB of payload (≈2300 lines at 76 chars). Tests assert on counts and roundtrip equality — they MUST NOT pretty-print the full G-code on failure. Use `assert_eq!` with substring-prefix elision (e.g., `&actual[..120]`) when the assertion fails so failure SNIPPETS stay within the ≤ 20-line dispatch budget.

Aggregate context cost: M. No step is L. If the `serialize_config_block` value-formatting investigation (Step 6) reveals OrcaSlicer divergence on a numeric formatter (e.g., trailing-zero rules), surface it as a packet-local risk and emit the simplest OrcaSlicer-matching formatter rather than expanding scope.
