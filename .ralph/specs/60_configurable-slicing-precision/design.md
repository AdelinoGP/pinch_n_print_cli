# Design: 60_configurable-slicing-precision

## Controlling Code Paths

- **IR config surface:** `crates/slicer-ir/src/resolved_config.rs:328` — `declare_resolved_config!` block. Adds 7 fields via the existing `cli "..." name: T = default => extractor;` DSL. Macro generates `ResolvedConfig`, `Default` impl, and CLI-key routing. No new types.
- **Helpers:** `crates/slicer-helpers/src/decimate.rs` — already houses geometric decimation routines (existing per `Glob`). Extend with `simplify_polyline_mm` and `drop_short_segments_mm`. Re-export from `slicer-helpers/src/lib.rs` if `decimate` is not already re-exported.
- **Polygon ops:** `crates/slicer-core/src/polygon_ops.rs:185` — `pub fn offset(polygons, delta_mm, join) -> Vec<ExPolygon>`. Add `arc_tolerance_mm: f32` as 4th positional param. The `inflate_paths_64(..., 2.0, 0.0)` call at `:207` consumes it (after `* 10_000.0` scale to Clipper2 units, cast to f64).
- **Mesh slice:** `crates/slicer-core/src/triangle_mesh_slicer.rs:341` (call to `simplify_polygon_points`) and `:349` (definition). The closing-radius round-trip lives **after** the `simplify_polygon_points` call, before the layer is finalized. Gated by `slice_closing_radius > 0.0`.
- **G-code emit (XYZ decimals):** `crates/slicer-host/src/gcode_emit.rs:1304` — `format_coord` definition. Change signature to `format_coord(value: f32, decimals: u32) -> String`. Update 5 call sites at `:314`, `:317`, `:1093`, `:1096`, `:1099`. Leave call sites at `:1112` (F), `:1127`/`:1134`/`:1151`/`:1158` (E and speed), `:1194` (temperature) UNCHANGED — they must continue calling the old behavior. Since `format_coord` will now take 2 args, those sites need their own helper. **Selected approach (see below):** split into two functions — `format_coord_with_decimals(value, decimals)` (new) and keep `format_coord(value)` as a thin wrapper that calls `format_coord_with_decimals(value, 3)` (since current behavior at non-XYZ sites is incidentally OrcaSlicer-aligned at 3 decimals for F, and the existing `{:.4}` only matters for XYZ). **Verify**: the F-site test must confirm F output remains numerically equivalent under the new wrapper.

  Actually safer: keep `format_coord(value: f32) -> String` byte-identical to current `{:.4}` behavior, add a new sibling `format_xyz(value: f32, decimals: u32) -> String`, and call `format_xyz(value, cfg.gcode_xy_decimals)` at the 5 XYZ sites. F, E, and temperature sites retain `format_coord(value)` and current decimal behavior. This is the chosen approach.
- **G-code emit (D-P + min-segment):** `crates/slicer-host/src/gcode_emit.rs` — polyline-emit sites. The emit walks `LayerCollectionIR` move groups, each carrying an `ExtrusionRole` (defined at `crates/slicer-ir/src/slice_ir.rs:1318`). Insert tolerance dispatch via a small helper `fn tolerance_for_role(role: ExtrusionRole, cfg: &ResolvedConfig) -> f32` that maps `Perimeter|ExternalPerimeter|OverhangPerimeter|Brim → cfg.gcode_resolution`, `InternalInfill|SolidInfill|TopSolidInfill|BottomSurface|Bridge → cfg.infill_resolution`, `SupportMaterial|SupportMaterialInterface → cfg.support_resolution`, `Travel|Other → 0.0` (no simplification). For each polyline: `let simplified = simplify_polyline_mm(&pts, tol); let pruned = drop_short_segments_mm(&simplified, cfg.min_segment_length); emit(&pruned);`.
- **Module manifests:** `modules/core-modules/classic-perimeters/classic-perimeters.toml` and `modules/core-modules/arachne-perimeters/arachne-perimeters.toml`. Each gets a new `[config.schema.perimeter_arc_tolerance]` block following the existing `wall_count` / `line_width` shape. Snake_case headers per `CLAUDE.md` "Config Key Naming Convention".
- **Module sources:** `modules/core-modules/classic-perimeters/src/lib.rs:14, :112, :184` and `modules/core-modules/arachne-perimeters/src/lib.rs:21, :157, :250`. Each `use slicer_core::polygon_ops::{offset, OffsetJoinType};` import stays; the `offset(...)` calls gain `cfg.perimeter_arc_tolerance` as the new 4th positional arg (read from the per-module config via the existing SDK accessor — same pattern as `wall_count`).

## Neighboring Tests and Fixtures

- `crates/slicer-core/tests/polygon_ops_tdd.rs:50` — `offset_outward_expands_bounds`. New test `offset_arc_tolerance_reduces_vertex_count` lives in the same file (or a sibling test file if file size > 300 lines — defer to implementer).
- `crates/slicer-host/tests/` — existing fixture infrastructure; the integration test reuses it. STL fixture to be small (target < 10 KB) so the legacy golden stays compact and reviewable.
- `crates/slicer-host/tests/fixtures/golden/` — new sub-directory for the legacy-mode G-code golden file. One file: `precision_legacy_<fixture-name>.gcode`.

## OrcaSlicer Comparison Surface

- `OrcaSlicerDocumented/src/libslic3r/MultiPoint.cpp:179` — `_douglas_peucker`. Borrow algorithm shape only; we operate in mm-space `f32`, not int64-units, so the squared-distance constants differ.
- `OrcaSlicerDocumented/src/libslic3r/libslic3r.h` — `RESOLUTION`, `SPARSE_INFILL_RESOLUTION`, `SUPPORT_RESOLUTION`. Exact defaults adopted.
- `OrcaSlicerDocumented/src/libslic3r/GCode/GCodeWriter.hpp:234` — `XYZF_EXPORT_DIGITS = 3`, `E_EXPORT_DIGITS = 5`. Confirms decimal count.
- `OrcaSlicerDocumented/src/libslic3r/PrintObjectSlice.cpp:192,1393` — `slice_closing_radius` application semantics. Mirror: per-layer Clipper2 `inflate(+r) → inflate(-r)`.

## Architecture Constraints

- 1 unit = 100 nm (`docs/08_coordinate_system.md`). All mm-to-units conversions go via `* 10_000.0`. The D-P pass operates in mm-space `f32` because G-code emit is already mm-space — no integer trap.
- `ResolvedConfig` extensions are additive and must not change existing field order or visibility (per the macro-driven generator at `slicer-ir/src/resolved_config.rs:67-162` comments). The 7 new fields go inside the existing block, alongside their semantic neighbors (geometry/precision section).
- Manifest schema additions (`[config.schema.<key>]`) follow snake_case headers per `CLAUDE.md` "Config Key Naming Convention".
- Module guest WASM is NOT rebuilt by `cargo build` or `cargo test`. After editing perimeter modules / `slicer-ir` / `slicer-sdk` / `slicer-helpers`, the implementer MUST run `./modules/core-modules/build-core-modules.sh --check` and rebuild if STALE (per `CLAUDE.md` "Guest WASM Staleness").
- The slice path (`slicer-core::triangle_mesh_slicer`) runs at host built-in stage; `ResolvedConfig` is already in scope there. No new plumbing.
- The host emit pass already receives `ResolvedConfig` via host-context plumbing (precedent: TASK-153 per-role feedrate, TASK-182 overhang speed, both at `gcode_emit.rs`). No new plumbing.
- `slicer_core::polygon_ops::offset` is called from both host code AND guest WASM modules. The signature change must remain ABI-stable on the Rust side (it is — Rust doesn't have ABI compat constraints between guest and host modules; each compiles its own copy).

## Code Change Surface

- **Selected approach:** additive macro extension for the 7 keys; extend `slicer-helpers/src/decimate.rs` with two new public functions; signature change on `polygon_ops::offset` (one new positional arg); sibling helper `format_xyz` in `gcode_emit.rs` (do NOT change `format_coord`); per-role tolerance dispatch via a small helper; `+r/-r` round-trip block in `triangle_mesh_slicer.rs` gated on `slice_closing_radius > 0.0`; manifest entries on both perimeter modules.

- **Exact functions, traits, manifests, tests, or fixtures expected to change:**
  - `declare_resolved_config!` block at `crates/slicer-ir/src/resolved_config.rs:328-399` — 7 new lines.
  - `crates/slicer-helpers/src/decimate.rs` — add `pub fn simplify_polyline_mm(pts: &[Point2WithMeta], tolerance_mm: f32) -> Vec<Point2WithMeta>` (exact point-type name to be confirmed against existing decimate fns) and `pub fn drop_short_segments_mm(pts: &[Point2WithMeta], min_len_mm: f32) -> Vec<Point2WithMeta>`. Plus `#[cfg(test)] mod tests { ... }` with 4 unit tests.
  - `crates/slicer-helpers/src/lib.rs` — re-export the two new fns if `decimate` is not already glob-re-exported.
  - `crates/slicer-core/src/polygon_ops.rs:185` — signature change: `pub fn offset(polygons: &[ExPolygon], delta_mm: f32, join: OffsetJoinType, arc_tolerance_mm: f32) -> Vec<ExPolygon>`. Body change: replace the hardcoded `0.0` at `:207` with `(arc_tolerance_mm * 10_000.0) as f64`.
  - `crates/slicer-core/tests/polygon_ops_tdd.rs` — add `offset_arc_tolerance_reduces_vertex_count` test.
  - `crates/slicer-core/src/triangle_mesh_slicer.rs:341-360` — after the `simplify_polygon_points` call on each layer, insert `if cfg.slice_closing_radius > 0.0 { closing_radius_round_trip(&mut layer_polygons, cfg.slice_closing_radius); }`. Define `closing_radius_round_trip` inline (or as a helper) that calls `polygon_ops::offset(..., +r, OffsetJoinType::Round, 0.0)` then `offset(..., -r, OffsetJoinType::Round, 0.0)` (use `0.0` arc tolerance for the closing-radius offset since it's a topology operation, not a finishing offset).
  - `crates/slicer-core/tests/triangle_mesh_slicer_tdd.rs` — add `slice_closing_radius_fuses_gap_within_two_r` and `slice_closing_radius_zero_is_noop` tests.
  - `crates/slicer-host/src/gcode_emit.rs:1304` — add new sibling `fn format_xyz(value: f32, decimals: u32) -> String`. Update 5 XYZ call sites (`:314`, `:317`, `:1093`, `:1096`, `:1099`) to call `format_xyz(v, cfg.gcode_xy_decimals)`. Leave `format_coord` untouched.
  - `crates/slicer-host/src/gcode_emit.rs` (polyline emit loops) — insert `tolerance_for_role` helper and apply `simplify_polyline_mm` + `drop_short_segments_mm` per polyline. Implementer must locate the exact loop(s) — likely 1-3 sites; delegate a `grep "G1 X" /-B 20` audit if uncertain.
  - `crates/slicer-host/tests/gcode_emit_format_tdd.rs` *(new or existing — implementer confirms)* — `format_coord_decimals` unit test.
  - `crates/slicer-host/tests/gcode_emit_per_role_tolerance_tdd.rs` *(new)* — `per_role_tolerance_dispatch` test.
  - `crates/slicer-host/tests/slicing_precision_integration_tdd.rs` *(new)* — `default_emits_fewer_lines_than_legacy` and `legacy_zero_matches_golden`.
  - `crates/slicer-host/tests/module_manifest_tdd.rs` *(new or existing — implementer confirms)* — `perimeter_modules_declare_arc_tolerance`.
  - `crates/slicer-host/tests/fixtures/golden/precision_legacy_*.gcode` *(new)* — byte-identical golden for legacy mode.
  - `crates/slicer-sdk/src/host.rs:253` — `offset_polygons` body: `slicer_core::polygon_ops::offset(polygons, delta_mm, to_core_join(join), 0.0)`. Default to `0.0` (legacy) at the SDK wrapper unless the caller threads a value; in this packet the perimeter modules will pass `cfg.perimeter_arc_tolerance` directly to `offset(...)` rather than via `offset_polygons`. Confirm whether the SDK wrapper needs the new param exposed too — if yes, propagate; if no, leave `0.0`. *Decision*: do NOT extend the SDK wrapper signature in this packet; the SDK function is the WIT-boundary path and changing its arity ripples through 4 `wit_host.rs` impls and WIT itself. Out of scope.
  - `crates/slicer-host/src/wit_host.rs:2412, :3193` — direct `polygon_ops::offset` callers; pass `0.0` as the new arg (no behavioral change). These call sites are host-internal helpers, not on the WIT boundary.
  - `crates/slicer-host/src/layer_slice.rs:11` (import) and call site within the file — pass `0.0` as the new arg.
  - `crates/slicer-core/benches/polygon_ops.rs:11` (import) and call sites — pass `0.0`.
  - `modules/core-modules/classic-perimeters/classic-perimeters.toml` — add `[config.schema.perimeter_arc_tolerance]` block.
  - `modules/core-modules/classic-perimeters/src/lib.rs:112, :184` — `offset(&current_polygons, inset_delta, OffsetJoinType::Miter, cfg.perimeter_arc_tolerance)`.
  - `modules/core-modules/arachne-perimeters/arachne-perimeters.toml` — same manifest block.
  - `modules/core-modules/arachne-perimeters/src/lib.rs:157, :250` — same call-site update.

- **Rejected alternatives:**
  - **Renaming `offset` to `offset_expolygons`** (suggested in the original prompt): rejected — the existing name `offset` is stable and the rename would touch ~10 imports for zero behavioral benefit.
  - **Changing `format_coord` to take a decimals parameter and updating every call site**: rejected — F (feedrate) and temperature share the formatter and adopting `gcode_xy_decimals` for them is a scope expansion the user explicitly excluded. The sibling-function approach is cleaner and confines the blast radius to the 5 XYZ sites.
  - **Adding the closing-radius round-trip to `simplify_polygon_points`**: rejected — `simplify_polygon_points` operates on a single loop's `Vec<Point2>`; the closing radius is a topological operation across all polygons in a layer (it can merge two polygons into one). Applying it inside `simplify_polygon_points` would be a semantic mismatch.
  - **Adding `infill_resolution` and `support_resolution` as per-module manifest keys**: rejected — these tolerances live in `ResolvedConfig` and are consumed by the host emit pass, not by guest modules. Per-module registration would mean infill/support modules export them but never use them. Only `perimeter_arc_tolerance` is per-module because perimeter modules call `offset(...)` directly.

## Files in Scope (read + edit)

Each step's `implementation-plan.md` entry restricts files-to-edit to ≤ 3.

- `crates/slicer-ir/src/resolved_config.rs` — role: declare 7 config keys; expected change: 7 lines inside `declare_resolved_config!` at `:328`.
- `crates/slicer-helpers/src/decimate.rs` — role: D-P + min-segment helpers; expected change: 2 new public fns + a `tests` mod with 4 unit tests.
- `crates/slicer-helpers/src/lib.rs` — role: re-export new helpers if needed; expected change: 1-2 lines (or no change if already re-exported).
- `crates/slicer-core/src/polygon_ops.rs` — role: arc_tolerance parameter; expected change: 1 signature line + 1 body line.
- `crates/slicer-core/src/triangle_mesh_slicer.rs` — role: closing-radius round-trip; expected change: 5-15 lines around `:349`.
- `crates/slicer-core/tests/polygon_ops_tdd.rs` — role: arc_tolerance test; expected change: new `#[test] fn` block.
- `crates/slicer-core/tests/triangle_mesh_slicer_tdd.rs` — role: closing-radius tests; expected change: 2 new `#[test] fn` blocks.
- `crates/slicer-host/src/gcode_emit.rs` — role: format_xyz + per-role tolerance + min-segment; expected change: 1 new fn (`format_xyz`), 1 new helper (`tolerance_for_role`), 5 XYZ call-site updates, 1-3 polyline-emit-loop updates.
- `crates/slicer-host/tests/gcode_emit_format_tdd.rs` — role: format_coord/format_xyz tests; expected change: new test file or new `#[test] fn` block.
- `crates/slicer-host/tests/gcode_emit_per_role_tolerance_tdd.rs` *(new)* — role: per-role dispatch test.
- `crates/slicer-host/tests/slicing_precision_integration_tdd.rs` *(new)* — role: legacy-vs-default integration.
- `crates/slicer-host/tests/module_manifest_tdd.rs` — role: manifest schema test.
- `crates/slicer-host/tests/fixtures/golden/precision_legacy_*.gcode` *(new)* — role: byte-identical legacy golden.
- `crates/slicer-host/src/wit_host.rs` — role: pass-through `0.0` for new `offset` arg at `:2412, :3193`.
- `crates/slicer-host/src/layer_slice.rs` — role: pass-through `0.0` for new `offset` arg.
- `crates/slicer-sdk/src/host.rs` — role: pass-through `0.0` at `:253`.
- `crates/slicer-core/benches/polygon_ops.rs` — role: pass-through `0.0`.
- `modules/core-modules/classic-perimeters/classic-perimeters.toml` — role: manifest schema; expected change: 7-line TOML block.
- `modules/core-modules/classic-perimeters/src/lib.rs` — role: read `perimeter_arc_tolerance`, pass to `offset(...)`; expected change: 2 call-site updates + 1 config read.
- `modules/core-modules/arachne-perimeters/arachne-perimeters.toml` — role: manifest schema; expected change: same 7-line block.
- `modules/core-modules/arachne-perimeters/src/lib.rs` — role: same as classic; expected change: 2 call-site updates + 1 config read.

## Read-Only Context

- `crates/slicer-ir/src/resolved_config.rs` — read lines `[1-67]` only (overview comments) plus the macro block at `[328-399]`. Do NOT scroll through the middle (`[100-327]`) — large.
- `crates/slicer-ir/src/slice_ir.rs` — read lines `[1310-1370]` only — purpose: confirm `ExtrusionRole` variants for the per-role dispatch helper.
- `crates/slicer-host/src/gcode_emit.rs` — range-read `[300-330]` (Z and HEIGHT comments), `[1080-1170]` (the G1 emit block), `[1180-1210]` (temperature path), `[1290-1320]` (`format_coord` definition). Do NOT load the file in full.
- `crates/slicer-core/src/polygon_ops.rs` — read lines `[1-50]` (imports + types) and `[180-220]` (`offset` definition).
- `crates/slicer-core/src/triangle_mesh_slicer.rs` — read lines `[330-370]` (call site + `simplify_polygon_points` definition).
- `modules/core-modules/classic-perimeters/classic-perimeters.toml` — small, load fully (≈ 70 lines).
- `modules/core-modules/arachne-perimeters/arachne-perimeters.toml` — small, load fully.
- `modules/core-modules/classic-perimeters/src/lib.rs` — range-read around `:14` (imports), `:100-130` (first `offset` call), `:170-200` (second `offset` call). Skim a config-read site already in the file (e.g. how `wall_count` is read) to confirm the SDK accessor pattern.
- `modules/core-modules/arachne-perimeters/src/lib.rs` — same pattern, ranges around `:21`, `:150-170`, `:240-260`.
- `crates/slicer-helpers/src/decimate.rs` — load fully (likely small — file exists, full read is fine).
- `docs/08_coordinate_system.md` — load fully (small, normative).
- `docs/13_slicer_helpers_crate.md` — load fully or delegate SUMMARY.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — delegate every parity check; never load. Return format: LOCATIONS or SNIPPETS ≤ 30 lines.
- `target/`, `Cargo.lock`, all generated WASM (`modules/core-modules/*/*.wasm`, `test-guests/*.component.wasm`) — never load.
- `crates/slicer-host/src/wit_host.rs` — large (5000+ lines); only touch `:2412` and `:3193` via Edit. Do NOT browse. The other `offset_polygons` impls at `:2400`, `:4135`, `:4600`, `:5215` are WIT-resource methods that delegate to the underlying offset and need to be checked for any indirect call to `polygon_ops::offset` — if they pass through, they need the new arg threaded; if they route via the SDK wrapper, leave alone. Confirm via grep, not by reading.
- `docs/07_implementation_status.md` — never read in full. The `TASK-201` entry is added at packet close via a worker dispatch that takes the new line as input.
- `crates/slicer-host/tests/fixtures/golden/precision_legacy_*.gcode` — once the golden is generated, treat it as opaque data. Diff failures dispatch a sub-agent for the SNIPPETS.
- Any unrelated crate or module — delegate symbol lookups; do not browse.

## Expected Sub-Agent Dispatches

- "Locate every polyline-emit loop in `crates/slicer-host/src/gcode_emit.rs` that produces `G1 X Y` lines. Return LOCATIONS (file:line + 1-line context per loop). Cap 5 entries." — purpose: pin Step 7's edit sites without loading the full emit file.
- "Run `cargo check --workspace`. Return FACT pass/fail. On fail, SNIPPETS of first 3 errors, ≤ 20 lines." — purpose: heartbeat after each step.
- "Run `cargo test -p <crate> --test <file> -- <test_name>`. Return FACT pass/fail. On fail, SNIPPETS of the assertion + ≤ 20 lines of code context." — purpose: per-AC verification.
- "Audit indirect callers of `polygon_ops::offset` in `crates/slicer-host/src/wit_host.rs`. Return LOCATIONS of any line that calls `slicer_core::polygon_ops::offset` directly. Cap 10 entries." — purpose: catch any wit_host.rs sites the initial grep missed.
- "Summarize OrcaSlicer's `MultiPoint::_douglas_peucker` algorithm: control flow, distance metric, endpoint handling. Return SUMMARY ≤ 200 words. No code snippets unless absolutely needed." — purpose: implementer ports the algorithm without loading `OrcaSlicerDocumented/`.
- "Run `./modules/core-modules/build-core-modules.sh --check`. Return FACT FRESH/STALE per module. On STALE, run the rebuild and return FACT pass/fail." — purpose: WASM freshness gate.
- "Append `TASK-201` line to `docs/07_implementation_status.md` under the DEV-009 umbrella (line `:184`). Return FACT inserted-at-line-N." — purpose: backlog amendment at packet close without loading the full file.
- "On the integration test failure, return the assertion line and the first 20 lines of the diff between produced and golden G-code." — purpose: keep golden diffs out of the implementer's context.

## Data and Contract Notes

- **IR contract**: `ResolvedConfig` gains 7 fields; all `Default`-derived; no schema-version bump needed because additive `ResolvedConfig` extensions are within the same IR version family per existing precedent (TASK-153 added 26 keys without a bump; TASK-182 added 4).
- **WIT boundary**: no WIT changes. The new `arc_tolerance_mm` parameter on `polygon_ops::offset` is a Rust-internal signature change; the WIT-level `offset_polygons` host service keeps its current signature (the SDK wrapper just hardcodes `0.0` for now).
- **Manifest contract**: `[config.schema.perimeter_arc_tolerance]` follows the established `wall_count` / `line_width` shape (`type`, `default`, `min`, `max`, `display`, `group`). `min = 0.0` lets legacy mode set `0.0`.
- **Determinism / scheduler**: no scheduler impact. Slice and emit stages already in the pipeline; this packet adds intra-stage simplification, not reordering.

## Locked Assumptions and Invariants

- **Zero-cost legacy path**: when each new key is set to its legacy value (`0.0` for tolerances and radius; `4` for decimals), the produced G-code MUST be byte-identical to pre-packet output. Enforced by NEG-2 (golden test).
- **F and temperature formatting unchanged**: `format_coord(value: f32) -> String` keeps current `{:.4}` behavior. Only the 5 XYZ call sites adopt the new `format_xyz(value, decimals)`. F adoption is a future packet.
- **`simplify_polyline_mm(pts, 0.0)` is identity**: returns the exact input. No allocation strategy change; same `Vec` length and element values.
- **`drop_short_segments_mm` preserves first AND last point**: critical for closed loops (where the first and last point are intentionally equal) and open paths (where the last point is a real endpoint).
- **`slice_closing_radius` round-trip uses `OffsetJoinType::Round`**: matches OrcaSlicer's `PrintObjectSlice.cpp` semantics. Arc tolerance on the round-trip is `0.0` (Clipper2 default) — closing-radius is a topology operation, not a finishing offset.
- **Per-`ExtrusionRole` dispatch table**: travel (`ExtrusionRole::Travel` or equivalent non-extrusion variant) gets tolerance `0.0` (no simplification). The exact `ExtrusionRole` variants the implementer maps must come from the variant list at `slicer-ir/src/slice_ir.rs:1318` — do not invent variants.
- **OrcaSlicer defaults adopted verbatim**: `RESOLUTION`, `SPARSE_INFILL_RESOLUTION`, `SUPPORT_RESOLUTION`, `slice_closing_radius`, `XYZF_EXPORT_DIGITS`. Source citations in `requirements.md`.

## Risks and Tradeoffs

- **Risk: golden bit-rot.** The legacy-mode golden file pinned in `tests/fixtures/golden/` will break on any unrelated G-code emit change (e.g. comment-string change, extruder-temp formatting, header alteration). Mitigation: the golden is small (one fixture STL, small mesh), the legacy mode disables every new feature, so any breakage signals a non-additive change elsewhere — which we want to catch. Reviewer note: when this golden breaks in a future packet, the breaking packet must either restore byte-identity for legacy mode or update the golden with a justification.
- **Risk: per-role dispatch miscategorizes a new `ExtrusionRole` variant.** The `tolerance_for_role` helper must be exhaustive (no wildcard arm that silently swallows new variants). Mitigation: use a `match` with explicit arms and a `// All variants must be enumerated above; new variants intentionally fail compile here.` comment (which IS a non-obvious WHY).
- **Risk: arc-tolerance regressions in non-perimeter callers.** Host call sites and the SDK wrapper currently pass `0.0` (Clipper2 default = `delta/500`, very fine). Threading `0.0` keeps current behavior at those sites. The follow-up packet can introduce per-call-site tolerances.
- **Risk: closing-radius bridges geometry users care about.** A `0.049 mm` radius can fuse intentionally-separate features that are within `0.098 mm`. Mitigation: matches OrcaSlicer's default exactly; legacy mode (`0.0`) opts out.
- **Tradeoff: D-P operates in mm-space f32, not int64 units.** OrcaSlicer's port uses int64 (`coord_t`). We choose mm `f32` because G-code emit is already mm-space. Risk: `f32` precision could mis-order interior vertices for very long polylines (`> 100 m` accumulated length). Acceptable for printer-scale geometry; out-of-scope to switch to f64.
- **Tradeoff: `format_xyz` sibling instead of changing `format_coord` signature.** Two functions instead of one is mildly more surface, but avoids touching F / E / temperature emit, which is what the user explicitly excluded. Net: less risk.

## Context Cost Estimate

- Aggregate (sum across all steps): `M`.
- Largest single step: Step 7 (per-role tolerance dispatch in `gcode_emit.rs`) — `M`. Largest because it requires locating polyline-emit loops in a large file.
- Highest-risk dispatch (the one whose return could blow budget if mis-shaped): the "locate polyline-emit loops in `gcode_emit.rs`" dispatch. Required return format: LOCATIONS, ≤ 5 entries, ≤ 1 line of context each. If the sub-agent returns code snippets instead, re-dispatch with tighter scope.

## Open Questions

None blocking activation. The following are minor implementation details the implementer resolves via grep/dispatch, not architectural unknowns:

- **Exact point type for `simplify_polyline_mm`**: the prompt suggests `Point3WithWidth`. The implementer must check `slicer-helpers/src/decimate.rs` for the existing point type used by other decimation fns and align — likely `Point2` or a `Vec3WithRole` type from `slicer-ir`. Decision: use whichever type the existing emit-time polyline carries; locate via the dispatch to find the polyline-emit loops.
- **Whether `slicer-helpers/src/lib.rs` already re-exports `decimate::*`**: a 1-line check. If yes, no re-export change needed.
- **`ExtrusionRole` variant names**: the implementer reads `slicer-ir/src/slice_ir.rs:[1310-1370]` and maps OrcaSlicer-style categories (perimeter / wall / brim / infill / support / travel) onto the actual variants. Any miscategorization is caught by the per-role dispatch test (AC-8).
- **Fixture STL choice**: implementer picks the smallest existing STL under `crates/slicer-host/tests/fixtures/` that exercises at least one perimeter, one infill, and one support polyline. If no such fixture exists, generate a tiny synthetic STL (single tall cylinder, layer height 0.2 mm, default wall count). Document the choice in the integration-test file as a comment.
