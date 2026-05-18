# Requirements: 60_configurable-slicing-precision

## Packet Metadata

- Grouped task IDs:
  - `TASK-201` (new; files under DEV-009 umbrella per `docs/07_implementation_status.md:184`)
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Three sites in the slicing pipeline are pinned to the finest-possible precision with no user knob, making slicing slower than necessary and producing G-code that is finer than printer hardware can resolve:

1. **G-code XY format is hardcoded `{:.4}`** in `crates/slicer-host/src/gcode_emit.rs:1304` — emits 0.0001 mm resolution. OrcaSlicer uses 3 decimals (`XYZF_EXPORT_DIGITS = 3`, 1 µm). Printers step in ~10 µm increments, so the extra digit is wasted precision plus extra file size.
2. **No polyline simplification exists at G-code emit.** `simplify_polygon_points` in `crates/slicer-core/src/triangle_mesh_slicer.rs:349` removes only *exactly* collinear vertices (integer `cross == 0` test). OrcaSlicer additionally runs a real Douglas-Peucker pass with a tunable `resolution` (default 0.01 mm) right before emit. We have neither the algorithm nor the knob.
3. **Clipper2 offset arc tolerance is hardcoded `0.0`** in `crates/slicer-core/src/polygon_ops.rs:207`. Clipper2 treats `0.0` as "use default", which is `delta/500` — extremely fine. Every rounded perimeter corner gets dense vertices, multiplying downstream work.

Additionally, OrcaSlicer's `slice_closing_radius` (default `0.049 mm`) fuses hairline cracks at slice time via an inflate→deflate round-trip. We have no equivalent.

This packet adds seven OrcaSlicer-style numeric knobs to `ResolvedConfig` and wires them through the slice and emit paths. All seven default to OrcaSlicer-aligned values for out-of-the-box parity; setting any to its "legacy" value (`0.0` for the tolerances and radius; `4` for the decimals) reproduces current behavior exactly. No preset enum, no G2/G3 arc fitting.

This is a fresh backlog item — no existing TASK-### in `docs/07_implementation_status.md` covers Douglas-Peucker simplification, Clipper2 arc tolerance, min-segment-length filtering, G-code XY decimals, or slice closing radius. The closest precedents — TASK-153 (per-role feedrate, +26 keys), TASK-154-cooling (+8 keys), TASK-182 (overhang speed, +4 keys), TASK-181 (paint_config namespace) — establish the additive `declare_resolved_config!` pattern this packet replicates.

## In Scope

- Declaring 7 new resolved-config keys in `crates/slicer-ir/src/resolved_config.rs:328` (inside the existing `declare_resolved_config!` block).
- Implementing iterative Douglas-Peucker (`simplify_polyline_mm`) and a `drop_short_segments_mm` helper in `crates/slicer-helpers/src/decimate.rs` (extending the existing file, not creating a new module).
- Applying `slice_closing_radius` as a Clipper2 `+r / -r` round-trip per layer in `crates/slicer-core/src/triangle_mesh_slicer.rs`, gated by `slice_closing_radius > 0.0`.
- Adding `arc_tolerance_mm: f32` parameter to `slicer_core::polygon_ops::offset` at `crates/slicer-core/src/polygon_ops.rs:185` and threading the new arg through every direct caller (`slicer-sdk/src/host.rs:253`, `slicer-host/src/wit_host.rs:2412, :3193`, `slicer-host/src/layer_slice.rs:11`, `slicer-core/benches/polygon_ops.rs`, `modules/core-modules/classic-perimeters/src/lib.rs:112, :184`, `modules/core-modules/arachne-perimeters/src/lib.rs:157, :250`).
- Adding `[config.schema.perimeter_arc_tolerance]` to `classic-perimeters.toml` and `arachne-perimeters.toml`, and reading the value in each module to pass into `offset(...)` calls. Non-perimeter callers pass the new default (`0.0125`) directly — no per-module key for them in this packet.
- Parameterizing `format_coord(value, decimals)` in `gcode_emit.rs:1304` and updating ONLY XYZ call sites (`:314 Z`, `:317 height`, `:1093 X`, `:1096 Y`, `:1099 Z`). F (`:1112`), E (`:1127`, `:1134`, `:1151`, `:1158`), and temperature (`:1194`) keep current behavior.
- Per-`ExtrusionRole` tolerance dispatch in the polyline-emit sites of `gcode_emit.rs` — perimeters/walls/brim → `gcode_resolution`, infill → `infill_resolution`, support → `support_resolution`, travel → no simplification.
- Rebuilding WASM guests (`./modules/core-modules/build-core-modules.sh --check` + rebuild if STALE) and test guests after the perimeter-module and `slicer-ir` edits.
- Unit tests for D-P, min-segment, `format_coord`, per-role dispatch, arc-tolerance vertex-count, `slice_closing_radius` (fuse + no-op), manifest registration, and one integration test in `crates/slicer-host/tests/` comparing legacy vs default-precision slice output (with a byte-identical golden for the legacy path).
- Adding `TASK-201` to `docs/07_implementation_status.md` (via worker dispatch at packet close — never load the full backlog).

## Out of Scope

- G2/G3 arc fitting (`enable_arc_fitting`).
- XY/contour/hole/elephant-foot compensation.
- A preset enum (`slice_precision = draft|normal|high`).
- Replacing the existing exact-collinearity `simplify_polygon_points` at `triangle_mesh_slicer.rs:349`.
- Arachne min-bead/transition family (`wall_transition_length`, `min_bead_width`, …).
- Per-module manifest keys for `infill_resolution` / `support_resolution` (they stay packet-wide, read from `ResolvedConfig` at emit; only `perimeter_arc_tolerance` is per-module because perimeter modules call `offset` directly).
- Adding range/validation on the IR-level keys. The per-module manifest entry on `perimeter_arc_tolerance` carries `min = 0.0`, `max = 1.0`, but the seven `ResolvedConfig` keys are unvalidated like their neighbors.
- Touching other parameters of `inflate_paths_64` (miter limit, end-type) — only arc tolerance.
- Modifying the F (feedrate) and E (extrusion) formatting; only XYZ adopts the new decimal parameter.

## Authoritative Docs

- `docs/02_ir_schemas.md` — **PRIMARY**. Field-addition rules for `ResolvedConfig`, IR versioning, determinism constraints. Load directly if ≤ 300 lines; delegate SUMMARY otherwise.
- `docs/03_wit_and_manifest.md` — manifest `[config.schema.*]` schema; required for `perimeter_arc_tolerance` registration. Delegate SUMMARY if > 300 lines.
- `docs/05_module_sdk.md` — confirms whether `perimeter_arc_tolerance` reads through the SDK config accessor (it does, same pattern as `wall_count` / `line_width` already used in classic-perimeters). Delegate SUMMARY.
- `docs/08_coordinate_system.md` — **CRITICAL**. 1 unit = 100 nm. Multiply mm values by `10_000.0` when feeding `inflate_paths_64` (note: `arc_tolerance` is f64 in the Clipper2 API). Load directly (small file).
- `docs/13_slicer_helpers_crate.md` — places D-P alongside existing helpers in `slicer-helpers`. Load directly.
- `docs/01_system_architecture.md` — confirms slice (slicer-core, host) vs emit (slicer-host) ownership. Closing-radius lives at slice; D-P lives at emit. Delegate SUMMARY.
- `docs/07_implementation_status.md` — `TASK-201` to be added under DEV-009 (`:184`). **Never read in full; delegate the line-edit at packet close.**

## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated; never load these into the implementer's own context.

- `OrcaSlicerDocumented/src/libslic3r/MultiPoint.cpp:179` — `MultiPoint::_douglas_peucker`. Port iterative form (stack-based), squared perpendicular distance, preserves first/last. Borrow: algorithm shape. Do not borrow: 64-bit-unit coordinate system (we operate in mm-space f32 at emit).
- `OrcaSlicerDocumented/src/libslic3r/libslic3r.h` — confirms `RESOLUTION = 0.0125`, `SPARSE_INFILL_RESOLUTION = 0.04`, `SUPPORT_RESOLUTION = 0.0375`. These are the **exact** defaults this packet adopts.
- `OrcaSlicerDocumented/src/libslic3r/GCode/GCodeWriter.hpp:234` — `XYZF_EXPORT_DIGITS = 3` confirms the default for `gcode_xy_decimals`. **Note**: OrcaSlicer applies 3 decimals to F (feedrate) too via the same constant. This packet does NOT change F formatting — F stays with the current `format_coord` behavior to avoid blast-radius creep. F can be adopted into `gcode_xy_decimals` in a follow-up.
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp:5658` — `slice_closing_radius` (default `0.049 mm`). Adopted verbatim.
- `OrcaSlicerDocumented/src/libslic3r/PrintObjectSlice.cpp:192,1393` — application site of `slice_closing_radius`: inflate by `+r`, deflate by `-r` per-layer. Mirror semantics exactly.
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp:4838` — `resolution` option def (default `0.01 mm`, template `0.012 mm`). Cross-check: our default `0.0125` matches `libslic3r.h::RESOLUTION` (which is what `print.gcode_resolution` reads via PrintConfig).

## Acceptance Summary

Positive cases (each backed by an exact verification command in `packet.spec.md`):

- All 7 new fields exist on `ResolvedConfig::default()` with the values specified.
- `simplify_polyline_mm` collapses collinear runs to endpoints; identity at `tolerance_mm <= 0.0`.
- `drop_short_segments_mm` drops segments shorter than threshold; preserves first AND last point.
- `format_coord(v, 3)` formats to 3 decimals with existing trailing-zero stripping.
- `polygon_ops::offset(..., arc_tolerance_mm=0.5)` produces strictly fewer vertices on rounded corners than `arc_tolerance_mm=0.0`.
- `slice_closing_radius = 0.04` fuses a `0.05 mm` gap; `0.01` does not.
- Per-`ExtrusionRole` tolerance dispatch routes correctly at emit (perimeter / infill / support / travel).
- Both perimeter module manifests declare `perimeter_arc_tolerance` with the specified TOML fields.
- Default-precision slice of a fixture STL emits at least 5% fewer `G1 X Y` lines than legacy-precision.

Negative cases:

- `simplify_polyline_mm` returns input unchanged for `tolerance_mm = 0.0` or negative values.
- Legacy-mode slice (all 7 keys at legacy values) produces byte-identical G-code to a pre-recorded golden — proves zero-impact legacy path.
- `slice_closing_radius = 0.0` skips the offset round-trip entirely (layer polygons byte-identical to no-radius control).

Measurable outcomes:

- 7 new fields visible on `ResolvedConfig`, each with the listed default.
- Two new public functions in `slicer-helpers/src/decimate.rs`: `simplify_polyline_mm` and `drop_short_segments_mm`.
- Updated `slicer_core::polygon_ops::offset` signature with `arc_tolerance_mm: f32` as the 4th positional parameter.
- Updated `format_coord(value: f32, decimals: u32) -> String` signature, called with `gcode_xy_decimals` at 5 specified XYZ sites only.
- `[config.schema.perimeter_arc_tolerance]` block present in both perimeter module manifests.
- Slice path in `triangle_mesh_slicer.rs` performs `+r/-r` offset round-trip when radius > 0.
- Integration test passes with the 5%-fewer-lines assertion AND a byte-identical legacy golden.
- `cargo clippy --workspace -- -D warnings` clean.
- WASM guests rebuilt (`./modules/core-modules/build-core-modules.sh --check` returns FRESH after rebuild).

Cross-packet impact:

- Unblocks future XY/elephant-foot compensation packet (precision plumbing now exists).
- Unblocks future preset-enum packet (numeric keys can be mapped to `draft/normal/high` later).
- Does NOT modify any other packet's directory; new `TASK-201` is a fresh entry.

## Verification Commands

- `cargo check --workspace` — type check; cheap heartbeat.
- `cargo test -p slicer-ir --test resolved_config_defaults_tdd -- new_precision_keys_have_orca_defaults` — AC-1.
- `cargo test -p slicer-helpers --lib -- decimate::tests` — AC-2, AC-3, AC-4, NEG-1.
- `cargo test -p slicer-host --test gcode_emit_format_tdd -- format_coord_decimals` — AC-5.
- `cargo test -p slicer-core --test polygon_ops_tdd -- offset_arc_tolerance_reduces_vertex_count` — AC-6.
- `cargo test -p slicer-core --test triangle_mesh_slicer_tdd -- slice_closing_radius_fuses_gap_within_two_r` — AC-7 (positive) and `slice_closing_radius_zero_is_noop` — NEG-3.
- `cargo test -p slicer-host --test gcode_emit_per_role_tolerance_tdd -- per_role_tolerance_dispatch` — AC-8.
- `cargo test -p slicer-host --test module_manifest_tdd -- perimeter_modules_declare_arc_tolerance` — AC-9.
- `cargo test -p slicer-host --test slicing_precision_integration_tdd -- default_emits_fewer_lines_than_legacy legacy_zero_matches_golden` — AC-10, NEG-2.
- `./modules/core-modules/build-core-modules.sh --check` — WASM freshness gate (after every edit to perimeter modules or `slicer-ir`/`slicer-sdk`/`slicer-helpers`).
- `./test-guests/build-test-guests.sh --check` — test-guest freshness gate (same trigger).
- `cargo clippy --workspace -- -D warnings` — packet-close gate per `CLAUDE.md`.

All commands above are delegation-friendly: each produces either an exit code, a single PASS/FAIL line, or a small structured assertion. The integration test (`slicing_precision_integration_tdd`) is the heaviest; sub-agents must return `FACT pass/fail` plus the assertion line on failure, not the full G-code diff.

## Step Completion Expectations

See `implementation-plan.md` for per-step preconditions, postconditions, files-in-scope, sub-agent dispatches, and context costs.

## Context Discipline Notes

Documented context-budget hazards for this packet:

- **Large files in the read-only path that MUST be ranged or delegated:**
  - `crates/slicer-host/src/gcode_emit.rs` — large file (> 1300 lines). Range-read around the 5 XYZ call sites (`:314`, `:317`, `:1093-1099`) and the `format_coord` definition (`:1304`). Delegate any wider audit.
  - `crates/slicer-ir/src/resolved_config.rs` — likely > 400 lines. Range-read the macro block at `:328-399` (per the original prompt's anchor); do not read the head/middle.
  - `crates/slicer-ir/src/slice_ir.rs` — large. Range-read around `ExtrusionRole` definition at `:1318` (~`:1280-1370`); do not browse.
  - `crates/slicer-core/src/triangle_mesh_slicer.rs` — range-read around `:341-360` (the `simplify_polygon_points` call site and definition). Do not browse the rest.
  - `crates/slicer-core/src/polygon_ops.rs` — range-read around `:180-220` (`offset` and `inflate_paths_64` site).
- **OrcaSlicer trees the implementer must NOT load directly:** entire `OrcaSlicerDocumented/`. All reads delegate, all returns are LOCATIONS or SNIPPETS of ≤ 30 lines.
- **Likely temptation reads:** the full backlog (`docs/07_implementation_status.md`), the integration-test golden G-code file, the per-module `wit-guest/` shims, all 4 `offset_polygons` impls in `wit_host.rs` (only `:2412` and `:3193` are direct `polygon_ops::offset` callers; the others are WIT-resource impls). Skip all of these.
- **Sub-agent return-format hints:**
  - "Audit direct callers of `slicer_core::polygon_ops::offset` after the signature change; return `LOCATIONS` only (file:line + 1-line context)."
  - "Run `cargo check --workspace`; return `FACT pass/fail`. On fail, `SNIPPETS` of the first 3 errors, ≤ 20 lines."
  - "Summarize `docs/02_ir_schemas.md` section on additive `ResolvedConfig` field rules; return `SUMMARY` ≤ 200 words."
