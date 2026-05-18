---
status: implemented
packet: 60_configurable-slicing-precision
task_ids:
  - TASK-201
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 60_configurable-slicing-precision

## Goal

Add seven OrcaSlicer-style numeric precision knobs to `ResolvedConfig`, implement a real Douglas-Peucker simplifier in `slicer-helpers`, wire D-P + min-segment sweep into the host G-code emit by `ExtrusionRole`, parameterize XYZ decimal output, plumb a configurable Clipper2 arc tolerance through `slicer_core::polygon_ops::offset`, and apply a `slice_closing_radius` inflate/deflate round-trip at mesh slice — all with zero-cost legacy behavior when each key is set to `0.0` / `4`.

## Scope Boundaries

- In scope:
  - Adding 7 keys to `declare_resolved_config!` in `crates/slicer-ir/src/resolved_config.rs:328` (block extends to ~`:399`): `gcode_resolution`, `infill_resolution`, `support_resolution`, `min_segment_length`, `gcode_xy_decimals`, `perimeter_arc_tolerance`, `slice_closing_radius`.
  - Implementing `simplify_polyline_mm` (iterative Douglas-Peucker, squared-distance, preserves endpoints) and `drop_short_segments_mm` in `crates/slicer-helpers/src/decimate.rs` (extend the existing file).
  - Per-role tolerance dispatch in `crates/slicer-host/src/gcode_emit.rs` polyline-emit sites, keyed off `ExtrusionRole` (defined at `crates/slicer-ir/src/slice_ir.rs:1318`).
  - Parameterizing `format_coord` in `crates/slicer-host/src/gcode_emit.rs:1304` to accept a decimal count, and updating ONLY the XYZ call sites (`:314`, `:317`, `:1093`, `:1096`, `:1099`). F (feedrate, `:1112`), E (`:1127`, `:1134`, `:1151`, `:1158`), and temperature (`:1194`) keep their current behavior.
  - Adding `arc_tolerance_mm: f32` parameter to `slicer_core::polygon_ops::offset` (`crates/slicer-core/src/polygon_ops.rs:185`) and threading the new arg through every direct caller.
  - Adding `[config.schema.perimeter_arc_tolerance]` entry to `modules/core-modules/classic-perimeters/classic-perimeters.toml` and `modules/core-modules/arachne-perimeters/arachne-perimeters.toml`, and reading the value in each module's `src/lib.rs` to pass into `offset(...)` calls.
  - Applying `slice_closing_radius` as a `+r / -r` Clipper2 offset round-trip on each slice layer after `simplify_polygon_points` at `crates/slicer-core/src/triangle_mesh_slicer.rs:349`, gated by `slice_closing_radius > 0.0`.
  - Rebuilding WASM guests per `CLAUDE.md` "Guest WASM Staleness" rule.
  - Adding unit tests for D-P, min-segment, `format_coord`, `slice_closing_radius`, and one integration test in `crates/slicer-host/tests/` comparing legacy vs default-precision slice output.

- Out of scope:
  - G2/G3 arc fitting (`enable_arc_fitting`) — Klipper-hostile; deferred.
  - XY/contour/hole/elephant-foot compensation — separate quality packet.
  - Preset enum (`slice_precision = draft|normal|high`) — user chose numeric keys only.
  - Replacing the existing exact-collinearity `simplify_polygon_points` at `triangle_mesh_slicer.rs:349` — kept as a cheap first stage.
  - Arachne min-bead/transition family (`wall_transition_length`, `min_bead_width`, …) — wall-quality, separate packet.
  - Introducing per-module config keys for `infill_resolution` / `support_resolution` — these stay packet-wide (read from `ResolvedConfig` at emit, not registered as module manifest schema). Only `perimeter_arc_tolerance` is registered per-module (because perimeter modules read it directly to call `offset`).
  - Touching `inflate_paths_64`'s other defaults (`miter_limit = 2.0`, `EndType::Polygon`) — only `arc_tolerance` is being made configurable.
  - Adding range/validation logic on the new keys at the `ResolvedConfig` level. The per-module manifest entry on `perimeter_arc_tolerance` does carry `min = 0.0` per the existing manifest pattern, but the IR-level keys are unvalidated like their neighbors.

## Prerequisites and Blockers

- Depends on: none (additive macro extension; existing emit/offset/slice paths are stable post-19e5791 "macro-driven ResolvedConfig as single source of truth").
- Unblocks: a follow-up packet for XY/elephant-foot compensation, and a future preset-enum packet if user direction changes.
- Activation blockers: none. The F-and-temperature-vs-XYZ ambiguity in the original prompt is resolved in design.md by splitting call-site adoption (see "Locked Assumptions").

## Acceptance Criteria

- **Given** a freshly-constructed `ResolvedConfig::default()`, **when** any of the seven new fields is read, **then** the value equals exactly: `gcode_resolution=0.0125`, `infill_resolution=0.04`, `support_resolution=0.0375`, `min_segment_length=0.05`, `gcode_xy_decimals=3`, `perimeter_arc_tolerance=0.0125`, `slice_closing_radius=0.049` (all `f32` except `gcode_xy_decimals: u32`). | `cargo test -p slicer-ir --test resolved_config_defaults_tdd -- new_precision_keys_have_orca_defaults --nocapture`
- **Given** the polyline `[(0.0, 0.0), (1.0, 0.0), (2.0, 0.0)]` with `tolerance_mm = 0.1`, **when** `simplify_polyline_mm` is called, **then** the result is exactly `[(0.0, 0.0), (2.0, 0.0)]` (collinear interior point removed). | `cargo test -p slicer-helpers --lib -- decimate::tests::dp_collapses_collinear_to_endpoints --nocapture`
- **Given** any non-empty polyline, **when** `simplify_polyline_mm(&pts, 0.0)` is called, **then** the returned `Vec` is structurally equal to the input (length matches and each point's f32 bit pattern matches). | `cargo test -p slicer-helpers --lib -- decimate::tests::dp_zero_tolerance_is_identity --nocapture`
- **Given** the polyline `[(0.0, 0.0), (0.01, 0.0), (0.02, 0.0), (1.0, 0.0)]` with `min_segment_length = 0.05`, **when** `drop_short_segments_mm` is applied, **then** the result equals `[(0.0, 0.0), (1.0, 0.0)]` — micro-segments dropped, first AND last point preserved verbatim. | `cargo test -p slicer-helpers --lib -- decimate::tests::min_segment_drops_micro_and_preserves_endpoints --nocapture`
- **Given** `format_coord(1.23456, 3)` and `format_coord(1.0, 3)` and `format_coord(1.10000, 3)`, **when** evaluated, **then** results equal `"1.235"`, `"1"`, and `"1.1"` respectively (existing trailing-zero stripping preserved on the new parameterized form). | `cargo test -p slicer-host --test gcode_emit_format_tdd -- format_coord_decimals --nocapture`
- **Given** a square contour with one rounded corner offset by `+1.0 mm` using `OffsetJoinType::Round`, **when** `slicer_core::polygon_ops::offset(&shape, 1.0, OffsetJoinType::Round, arc_tolerance_mm)` is called with `arc_tolerance_mm = 0.5` and again with `arc_tolerance_mm = 0.0`, **then** the `0.5`-tolerance result has **strictly fewer vertices** on the rounded corner than the `0.0`-tolerance result. | `cargo test -p slicer-core --test polygon_ops_tdd -- offset_arc_tolerance_reduces_vertex_count --nocapture`
- **Given** two horizontally-separated unit squares at distance `0.05 mm` apart in the same slice layer, **when** the slice path runs with `slice_closing_radius = 0.04`, **then** the resulting layer contains exactly one fused polygon (the `+0.04 / -0.04` round-trip bridges a `0.05 mm` gap since gap ≤ 2 × radius). And when run again with `slice_closing_radius = 0.01`, the layer contains exactly two separate polygons. | `cargo test -p slicer-core --test triangle_mesh_slicer_tdd -- slice_closing_radius_fuses_gap_within_two_r --nocapture`
- **Given** a `LayerCollectionIR` containing one perimeter polyline, one infill polyline, one support polyline, and one travel polyline (each constructed with intentional sub-tolerance wobble), **when** the host emit pass runs with the default `ResolvedConfig`, **then** the perimeter polyline is simplified with tolerance `0.0125`, the infill with `0.04`, the support with `0.0375`, and the travel polyline is emitted unchanged (no simplification applied to `ExtrusionRole::Travel` moves). The unit test asserts vertex counts post-emit match a precomputed table. | `cargo test -p slicer-host --test gcode_emit_per_role_tolerance_tdd -- per_role_tolerance_dispatch --nocapture`
- **Given** the manifests for `classic-perimeters` and `arachne-perimeters`, **when** parsed, **then** each declares `[config.schema.perimeter_arc_tolerance]` with `type = "float"`, `default = 0.0125`, `min = 0.0`, `max = 1.0`. | `cargo test -p slicer-host --test module_manifest_tdd -- perimeter_modules_declare_arc_tolerance --nocapture`
- **Given** a small fixture STL (`crates/slicer-host/tests/fixtures/...`), **when** sliced once with default precision and once with every new key forced to its legacy value (`gcode_resolution=0`, `infill_resolution=0`, `support_resolution=0`, `min_segment_length=0`, `gcode_xy_decimals=4`, `perimeter_arc_tolerance=0`, `slice_closing_radius=0`), **then** the legacy-precision output's `G1 X Y` line count is **strictly greater than** the default-precision output's `G1 X Y` line count, by at least 5% (a conservative floor against false positives from fixture choice). | `cargo test -p slicer-host --test slicing_precision_integration_tdd -- default_emits_fewer_lines_than_legacy --nocapture`

## Negative Test Cases

- **Given** any non-empty polyline `pts` and `tolerance_mm` equal to `0.0` or `-0.5`, **when** `simplify_polyline_mm(&pts, tolerance_mm)` is called, **then** the function returns the input unchanged (no panic, no error path; legacy zero-cost behavior). | `cargo test -p slicer-helpers --lib -- decimate::tests::dp_non_positive_tolerance_is_identity --nocapture`
- **Given** the same fixture STL as the integration test, **when** sliced with every new key zeroed (legacy mode), **then** the resulting G-code matches a pre-recorded byte-identical golden under `crates/slicer-host/tests/fixtures/golden/`; any byte difference fails the test. This proves the legacy-zero path is genuinely zero-impact, not just "approximately the same". | `cargo test -p slicer-host --test slicing_precision_integration_tdd -- legacy_zero_matches_golden --nocapture`
- **Given** `slice_closing_radius = 0.0`, **when** the slice path runs on any input mesh, **then** the layer polygons are byte-identical to the layer polygons produced by the same input on the pre-packet code path (no offset round-trip executes; verified by a sentinel counter or by direct equality check against a non-radius-applied control run). | `cargo test -p slicer-core --test triangle_mesh_slicer_tdd -- slice_closing_radius_zero_is_noop --nocapture`

## Verification

- `cargo check --workspace` — type check after every step.
- `cargo clippy --workspace -- -D warnings` — packet-close gate, per `CLAUDE.md`.
- `./modules/core-modules/build-core-modules.sh --check` — mandatory after editing perimeter module manifests/src or `slicer-ir`/`slicer-sdk`/`slicer-helpers`. Rebuild (`build-core-modules.sh` without `--check`) if STALE.
- `./test-guests/build-test-guests.sh --check` — same rationale; rebuild if STALE.

## Authoritative Docs

- `docs/02_ir_schemas.md` — `ResolvedConfig` field-addition rules and IR versioning. Load directly (delegate if > 300 lines). **Primary doc.**
- `docs/03_wit_and_manifest.md` — manifest TOML schema for `[config.schema.*]` block; delegate SUMMARY if > 300 lines.
- `docs/05_module_sdk.md` — confirms whether `perimeter_arc_tolerance` needs SDK accessor changes; delegate SUMMARY.
- `docs/08_coordinate_system.md` — 1 unit = 100 nm; consult for the `mm * 10_000.0 → units` conversion in `slice_closing_radius` and `arc_tolerance` plumbing. Load directly (small).
- `docs/13_slicer_helpers_crate.md` — where Douglas-Peucker fits in the helpers surface. Load directly.
- `docs/01_system_architecture.md` — confirms slice vs emit ownership (closing-radius at slice, D-P at emit). Delegate SUMMARY.
- `docs/07_implementation_status.md` — must be amended to add `TASK-201` under DEV-009 umbrella as part of packet close. **Do NOT read in full — delegate the amendment via worker dispatch.**

## OrcaSlicer Reference Obligations

All OrcaSlicer reads delegate to a sub-agent; never load these into the implementer's context.

- `OrcaSlicerDocumented/src/libslic3r/MultiPoint.cpp:179` — `MultiPoint::_douglas_peucker` reference implementation. Port iterative form with squared perpendicular distance; preserve first/last.
- `OrcaSlicerDocumented/src/libslic3r/libslic3r.h` — confirms `RESOLUTION = 0.0125 mm`, `SPARSE_INFILL_RESOLUTION = 0.04 mm`, `SUPPORT_RESOLUTION = 0.0375 mm` (our chosen defaults).
- `OrcaSlicerDocumented/src/libslic3r/GCode/GCodeWriter.hpp:234` — confirms `XYZF_EXPORT_DIGITS = 3` (our default for `gcode_xy_decimals`) and `E_EXPORT_DIGITS = 5` (we keep E formatting unchanged at `{:.5}`).
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp:5658` — `slice_closing_radius` option def (default `0.049 mm`).
- `OrcaSlicerDocumented/src/libslic3r/PrintObjectSlice.cpp:192,1393` — inflate→deflate semantics for closing-radius application; mirror exactly.
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp:4838` — `resolution` option def (cross-check our naming and default magnitude).

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`

## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream implementers MUST:

- Treat `design.md`'s "Files in Scope" as the authoritative read+edit list.
- Honor `design.md`'s "Out-of-Bounds Files" — `OrcaSlicerDocumented/`, generated WASM, large test goldens, full `docs/07` must not be loaded directly.
- Delegate every `cargo` run (return `FACT pass/fail` or `SNIPPETS` on failure ≤ 20 lines).
- Stop reading at 60% context; hand off at 85%.

Aggregate context cost is M; no single step is L. If any step grows beyond M during implementation, split before continuing.
