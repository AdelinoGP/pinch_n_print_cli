---
status: implemented
packet: 55_gcode-header-thumbnail-config-blocks
task_ids:
  - TASK-184
  - TASK-185
---

# 55_gcode-header-thumbnail-config-blocks

## Goal

Emit the OrcaSlicer textual G-code envelope that PinchAndPrint currently omits, so downstream tooling (Octoprint, Bambu Handy, Klipper UI) can show previews / stats and the file is reproducible:

1. `HEADER_BLOCK_START` / `HEADER_BLOCK_END` enclosing layer count, filament diameter, filament density, max Z, and the ordered filament list — emitted at the top of every produced `.gcode` file.
2. `THUMBNAIL_BLOCK_START` / `THUMBNAIL_BLOCK_END` enclosing a Base64-encoded PNG, conditional on a new `--thumbnail <png-path>` slicer-cli flag (when the flag is absent the block is omitted, matching OrcaSlicer CLI behavior).
3. Per-role extrusion-width comments (e.g., `; outer_wall_line_width = 0.42`) immediately after `HEADER_BLOCK_END`, sourced from the effective `ConfigView`.
4. `CONFIG_BLOCK_START` / `CONFIG_BLOCK_END` appended at the end of the file, enclosing every key in the effective `ConfigView` (user-passed values merged with registered defaults) as `; key = value` lines.

The sentinel literals, line ordering, comment prefix style, and Base64 column wrapping must match OrcaSlicer's wire format exactly so unmodified downstream parsers consume the file.

## Problem Statement

PinchAndPrint's final `.gcode` file is a bare stream of motion commands with the OrcaSlicer-compatible in-body comments (`;TYPE:`, `;LAYER_CHANGE`, `;Z:`, `;HEIGHT:` — completed by TASK-119) but no surrounding envelope:

- No `HEADER_BLOCK_START..HEADER_BLOCK_END` carrying layer count, filament diameter/density, max Z, and filament order.
- No `THUMBNAIL_BLOCK_START..THUMBNAIL_BLOCK_END` carrying a Base64-encoded PNG preview.
- No `; <line_width_key> = <value>` extrusion-width comments after the header.
- No `CONFIG_BLOCK_START..CONFIG_BLOCK_END` trailing dump of the effective configuration.

Consequence: downstream tooling (Octoprint, Bambu Handy, Klipper UI, slicer file-managers) cannot show previews or print stats, and a `.gcode` file cannot be re-sliced or audited because the configuration that produced it is not recoverable from the file. This packet closes that gap with OrcaSlicer-byte-format-parity for sentinels, line prefixes, and key spellings.

This packet does NOT reopen or supersede any prior packet. TASK-119 (live in-body comment contract) is left intact; this packet adds envelope structure around it. Predecessor packets 52, 53, 54 modify `gcode_emit.rs` at disjoint sites (feedrate, cooling, skirt/brim, M82/M83) and are unaffected.

## Architecture Constraints

- `GCodeIR` MUST NOT change. The envelope is computed from `PrintMetadata` + `ConfigView` + `LayerCollectionIR.z` already available to the serializer.
- The serializer remains pure: file I/O for the thumbnail PNG happens once at the top of `serialize_gcode()` (or in a thin wrapper called before it) and the bytes are passed in. Failure modes (file not found, bad PNG magic) become `Result::Err` propagated up to `main.rs` for a clean non-zero exit. The serializer never touches the filesystem for any other reason.
- Determinism: `serialize_config_block` iterates `ConfigView` in deterministic, key-sorted (lexical) order. No `HashMap` iteration leaks non-determinism into the file.
- Coordinate system: `max_z_height` is emitted in millimeters. Internal units are 100 nm (see `docs/08_coordinate_system.md`). Conversion is local to the helper.
- The four config keys (`filament_diameter`, `filament_density`, `max_z_height`, `thumbnail_path`) are registered in `config_schema.rs` with defaults; the registration is the schema source of truth (no parallel constants).

## Data and Contract Notes

- IR or manifest contracts touched: NONE. `PrintMetadata`, `ConfigView`, `LayerCollectionIR` are read but not modified. No new IR fields.
- WIT boundary considerations: NONE. No WIT files touched, no module manifests modified.
- Determinism or scheduler constraints: `serialize_config_block` MUST iterate keys in deterministic order. Use a sorted iterator (`BTreeMap` clone or `keys.sort()`); a `HashMap` iteration would make the output non-reproducible across runs.
- Config-schema validation: `thumbnail_path` is `ConfigValue::String`; an empty string is the explicit "no thumbnail" sentinel and is the registered default. The serializer treats `path.is_empty()` as "skip thumbnail block".

## Locked Assumptions and Invariants

- The sentinel literals `HEADER_BLOCK_START`, `HEADER_BLOCK_END`, `THUMBNAIL_BLOCK_START`, `THUMBNAIL_BLOCK_END`, `CONFIG_BLOCK_START`, `CONFIG_BLOCK_END` are byte-for-byte OrcaSlicer's — no PinchAndPrint-flavored variants. Confirmed by FACT dispatch 1.
- Comment line prefix is OrcaSlicer's (most likely `; ` — confirmed by FACT dispatch 1). The same prefix is used inside all four blocks.
- HEADER block is emitted BEFORE any `;TYPE:` or motion command. CONFIG block is emitted AFTER the final motion command. Width comments and the optional THUMBNAIL block sit between HEADER_END and the first motion command.
- The block ordering is HEADER → width comments → THUMBNAIL (optional) → motion body → CONFIG.
- `--thumbnail` is the sole way to inject a thumbnail; absence is OrcaSlicer-CLI-parity (no block).
- PNG validation is magic-only (`89 50 4E 47 0D 0A 1A 0A`); no IHDR parsing, no size validation, no re-encoding.
- Effective config = `ConfigView` constructed from user-passed `config_source` merged with `config_schema.rs` defaults. CONFIG_BLOCK enumerates exactly that view.
- `filament_used_mm` (already in `PrintMetadata`) is the source of truth for the filament-order line. Tools with `filament_used_mm[i] > 0` are emitted in ascending index order.

## Risks and Tradeoffs

- **Risk: ConfigValue → string formatting drift from OrcaSlicer.** Floats may be `0.16` vs `0.160000` vs `0.16f`. Mitigation: keep the formatter simple (`{:.4}` for floats, strip trailing zeros, `to_string()` for ints/bools); the AC `config_block_includes_user_passed` accepts `22` OR `22.0` to leave room.
- **Risk: width-key list drift.** OrcaSlicer's emitted set may not match PinchAndPrint's registered set. Mitigation: AC4 names five specific keys (`outer_wall_line_width`, `inner_wall_line_width`, `sparse_infill_line_width`, `top_surface_line_width`, `support_line_width`). Step 2 registers any missing from this list with OrcaSlicer-parity defaults; Step 4 emits exactly these five.
- **Risk: large PNG inflating the file unboundedly.** Mitigation: out of scope for this packet; document in `requirements.md` Out-of-Scope. Acceptance is byte-roundtrip, not byte-budget. The chosen test fixture (`resources/fake_thumb.png`, 940×940, ≈132 KB) deliberately exercises the >100 KB case so the chunking, prefix, and roundtrip assertions stress the Base64 path; tests still finish in well under a second.
- **Risk: CONFIG_BLOCK at file tail causes naive parsers to choke on `; ` lines after the last `G1`.** Mitigation: this is OrcaSlicer's behavior; any parser that mishandles it is non-conformant. AC `block_ordering_header_before_body_config_after` codifies the placement.
- **Tradeoff: by routing `--thumbnail` through `config_source` rather than a new pipeline parameter, the API stays stable but a misleading "thumbnail_path is a config key" registration enters `config_schema.rs`.** Accepted; the field's semantic is documented in its `ConfigFieldSchema::description`.
- **Tradeoff: no real per-print thumbnail.** Accepted at the user's explicit decision; the `--thumbnail` ingestion path makes follow-up rendering work additive (the wire-format helper is reused).

## Implementation Deviations

Recorded at packet closure (2026-05-14). None are blocking; all are within the accepted risk envelope.

- **DEV-A: `ThumbnailAwareSerializer` wrapper type** (vs. selected approach of "inline inside `DefaultGCodeSerializer::serialize_gcode()`"). The implementation wraps `DefaultGCodeSerializer` in a `ThumbnailAwareSerializer` struct (`gcode_emit.rs:928`) that handles THUMBNAIL_BLOCK and CONFIG_BLOCK injection, while HEADER_BLOCK and width comments remain inside `DefaultGCodeSerializer::serialize_gcode()`. Rationale: threading `thumbnail_bytes: Option<Vec<u8>>` into the existing serializer call chain would have required changing `GCodeSerializer` trait or `serialize_gcode()`'s signature, cascading into all existing test callers. The wrapper keeps `DefaultGCodeSerializer`'s API stable. Functional result is identical.

- **DEV-B: `thumbnail_path` excluded from CONFIG_BLOCK via `pipeline.rs` removal** (vs. no explicit documentation of this exclusion in the original design). `thumbnail_path` is an invocation-time routing key, not a print parameter; including it in CONFIG_BLOCK would embed a machine-local absolute path that breaks file portability. The fix: `pipeline.rs::run_pipeline_with_raw_config` calls `effective_config.remove("thumbnail_path")` immediately after bytes are extracted, so the key is consumed before `ThumbnailAwareSerializer` sees the map. `serialize_config_block` requires no hardcoded filter.

- **DEV-C: PNG file I/O and validation in `pipeline.rs`** (vs. design's "failure modes become `Result::Err` propagated up to `main.rs`"). All thumbnail file reading and PNG magic validation happens inside `run_pipeline_with_raw_config` (`pipeline.rs:257-273`). `main.rs` only inserts the path string into `config_source`; errors propagate as `PipelineError::Postpass` and are caught by the existing `match run_pipeline_with_raw_config(...)` handler in `main.rs`, which exits non-zero. The serializer itself remains pure (no file I/O).
