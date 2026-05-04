# Design: bridge-detector-orca-parity

## Controlling Code Paths

- Primary code path:
  - `crates/slicer-host/src/mesh_analysis.rs` — extends `execute_mesh_analysis_with` to compute the new `BridgeRegion` fields via mesh half-edge adjacency.
  - `crates/slicer-host/src/layer_slice.rs` — extends `classify_region_surfaces` (or adds sibling `assemble_bridge_areas`) to compute `bridge_areas` polygons via Minkowski offset + intersect.
  - `crates/slicer-host/src/wit_host.rs` — `slice-region-data` host record gains two fields; `sliced_region_to_data` populates them.
  - `wit/` — `slice-region-data` WIT type extended.
  - `crates/slicer-sdk/src/views.rs` — `SliceRegionView` gains `bridge_areas()` and `bridge_orientation_deg()` accessors.
  - `crates/slicer-macros` — `#[slicer_module]` codegen for the new fields.
  - `modules/core-modules/rectilinear-infill/src/lib.rs` — emit `BridgeInfill` over `bridge_areas` at `bridge_orientation_deg`.
- Neighboring tests or fixtures:
  - `crates/slicer-host/tests/bridge_detector_tdd.rs` (NEW).
  - `modules/core-modules/rectilinear-infill/tests/bridge_infill_emission_tdd.rs` (NEW).
  - `crates/slicer-host/tests/benchy_end_to_end_tdd.rs` — append.
  - 12-rev1's `external_surface_classification_tdd.rs` and 35's `multi_layer_thickness_tdd.rs` must remain green (they should, since `is_bridge` semantics widen but the previously-flagged cases still flag).
- OrcaSlicer comparison surface:
  - `OrcaSlicerDocumented/src/libslic3r/BridgeDetector.hpp` / `.cpp`.

## Architecture Constraints

- **No new fine-layer slicing pass** (inherited).
- **No new per-layer state** crossing `par_iter` boundaries.
- **Mesh adjacency analysis happens at PrePass.** Per-layer state stays inside `execute_layer_slice`.
- **Polygon offset uses `slicer-helpers`.** Step 0 FACT confirms availability of a Minkowski/offset utility; if absent, add one in scope (small).
- **Defaults match Orca.** Confirmed via FACT delegation.
- **WIT signature change requires WASM rebuild.** All infill-stage core modules MUST be rebuilt; verify `./modules/core-modules/build-core-modules.sh` succeeds before marking the packet implemented.
- **Schema bumps:** `SurfaceClassificationIR` → `1.1.0` (additive minor on `BridgeRegion`); `SliceIR` → `1.2.0` (additive minor on `SlicedRegion`). Both are additive minors per `docs/02_ir_schemas.md` rules.
- **Deviation closure**: This packet closes two deviations registered by packet `12-rev1_external-surface-classification-at-slice`:
  - **DEV-035** (any-vertex-in-polygon approximation in `crates/slicer-host/src/layer_slice.rs::classify_region_surfaces`) — replaced by polygon-polygon intersection via `assemble_bridge_areas` (Minkowski offset + intersect).
  - **DEV-036** (`crates/slicer-host/src/mesh_analysis.rs:213` `bridge_regions` initialized empty and never pushed) — closed by mesh-half-edge adjacency analysis in `execute_mesh_analysis_with`.

## Code Change Surface

- Selected approach:
  - **PrePass-time mesh adjacency analysis** — implement a half-edge graph internally (or via `slicer-helpers` if it has one — Step 0 FACT). Walk each cluster of bridge-eligible facets; identify anchor edges (edges shared with non-bridge facets); compute span and anchor width; pick optimal `bridge_direction_deg`. Apply `min_bridge_length_mm` and `anchor_width_mm` filters; populate `is_valid`.
  - **Pre-compute `xy_footprint`** for each bridge cluster as the union of facet XY projections in 100 nm units. One-time cost at PrePass.
  - **Slice-time `assemble_bridge_areas`** — for each region, iterate valid `BridgeRegion`s whose `xy_footprint` intersects the region's `infill_areas`. Compute the bridge polygon as `xy_footprint ∩ region.infill_areas`. Offset by `+expansion_margin_mm` (Minkowski sum). Intersect with `region.infill_areas` to keep the result inside the region. Result populates `SlicedRegion.bridge_areas`.
  - **`bridge_orientation_deg`** chosen as the orientation of the longest valid bridge intersecting the region (or the area-weighted average — pick one and document).
  - **`rectilinear-infill`** updated: emit `BridgeInfill` paths over `bridge_areas` at `bridge_orientation_deg`; emit `SparseInfill` over `infill_areas \ bridge_areas` (set difference).
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - `crates/slicer-ir/src/slice_ir.rs:336-344, 1006-1021` — extend `BridgeRegion` and `SlicedRegion`. Bump both schema versions.
  - `crates/slicer-host/src/mesh_analysis.rs` — add `MeshAnalysisConfig` struct; extend `execute_mesh_analysis_with` signature; add `compute_bridge_metrics` private function.
  - `crates/slicer-host/src/layer_slice.rs` — add `assemble_bridge_areas` helper; populate the new `SlicedRegion` fields.
  - `crates/slicer-host/src/layer_executor.rs:295-310` — pass `MeshAnalysisConfig` (resolved from blackboard config) into `execute_mesh_analysis` invocation; pass `surface_classification` into `execute_layer_slice` (already done in 12-rev1).
  - `wit/slicer.wit` (or wherever `slice-region-data` is declared) — add the two new fields.
  - `crates/slicer-host/src/wit_host.rs:140-175, 2517-2580` — extend the host record; populate the new fields in `sliced_region_to_data`.
  - `crates/slicer-sdk/src/views.rs` — add accessors.
  - `crates/slicer-macros/src/lib.rs` — codegen for the new fields (if macro mediates the WIT/SDK boundary).
  - `modules/core-modules/rectilinear-infill/src/lib.rs:95-135` — split fill emission for `bridge_areas` vs `infill_areas \ bridge_areas`.
  - `modules/core-modules/rectilinear-infill/tests/bridge_infill_emission_tdd.rs` (NEW).
  - `crates/slicer-host/tests/bridge_detector_tdd.rs` (NEW).
  - `crates/slicer-host/tests/benchy_end_to_end_tdd.rs` — append `benchy_gcode_contains_bridge_infill_evidence`.
- Rejected alternatives considered:
  - **Half-edge analysis on demand at slice time** — rejected: per-layer hot path; do it once at PrePass.
  - **Use `xy_footprint` as the bridge polygon directly without expansion** — rejected: anchoring is critical for print quality; without expansion bridges separate from supporting walls.
  - **Bridge orientation per-bridge-region only (no per-region merge)** — rejected: a slice region can intersect multiple bridge clusters; pick a single orientation per slice region for simplicity in the infill module.
  - **Defer WIT change and ride bridge polygons inside `infill_areas` with a marker** — rejected: would force every infill module to re-implement bridge handling; explicit `bridge_areas` field is the architecturally honest answer.

## Files in Scope (read + edit)

Primary edit targets (≤ 3 per step; aggregate across packet ≤ 6 — split into multiple steps):

- Step "Schema": `crates/slicer-ir/src/slice_ir.rs`.
- Step "Mesh adjacency": `crates/slicer-host/src/mesh_analysis.rs`.
- Step "Slice assembly": `crates/slicer-host/src/layer_slice.rs`.
- Step "WIT change": `wit/<file>.wit`, `crates/slicer-host/src/wit_host.rs`, `crates/slicer-sdk/src/views.rs` (3 files; one step).
- Step "Macros + module": `crates/slicer-macros/src/lib.rs`, `modules/core-modules/rectilinear-infill/src/lib.rs` (2 files; one step).

New test files:
- `crates/slicer-host/tests/bridge_detector_tdd.rs`.
- `modules/core-modules/rectilinear-infill/tests/bridge_infill_emission_tdd.rs`.

## Read-Only Context

- `OrcaSlicerDocumented/src/libslic3r/BridgeDetector.hpp` / `.cpp` — delegate FACT/SUMMARY only; never load.
- `crates/slicer-helpers/src/lib.rs` — public API only via symbol search; identify polygon offset (Minkowski) helpers.
- `docs/02_ir_schemas.md` — `BridgeRegion` and `SlicedRegion` sections; additive-minor rule.
- `docs/03_wit_and_manifest.md` — § "WIT/Type Changes Checklist".
- `docs/04_host_scheduler.md` — delegate SUMMARY of relevant sections.
- `docs/08_coordinate_system.md` — read directly.
- `docs/13_slicer_helpers_crate.md` — read directly.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` — delegate only.
- `target/`, `Cargo.lock`, generated code — never load.
- `crates/slicer-host/src/dispatch.rs` — > 2000 lines; out of scope.
- `crates/slicer-host/src/prepass.rs` — out of scope (we extend `execute_mesh_analysis` directly, not via prepass dispatch).
- All other core modules (`gyroid-infill`, `lightning-infill`, etc.) — out of scope unless the WASM rebuild step fails for one specifically; then range-read its WIT-binding manifest only.

## Expected Sub-Agent Dispatches

- "Does `slicer-helpers` expose a Minkowski sum / polygon offset / `expand_polygon` utility? Return FACT yes/no with file:line and signature." — purpose: validate Step 0.
- "Find `min_bridge_length`, `anchor_width_mm`, `expansion_margin` defaults in `OrcaSlicerDocumented/src/libslic3r/BridgeDetector.cpp`. Return FACT (numeric values only)." — purpose: validate Step 0.
- "Summarize `OrcaSlicerDocumented/src/libslic3r/BridgeDetector.cpp::detect_angle` algorithm in ≤ 200 words. Return SUMMARY." — purpose: inform Step 2.
- "Run `cargo test -p slicer-host --test bridge_detector_tdd`; return FACT pass/fail per test." — purpose: validate Step 2 + Step 3.
- "Run `./modules/core-modules/build-core-modules.sh`; return FACT pass/fail with the failing module name on fail." — purpose: validate Step 4 (WASM rebuild).
- "Run `cargo test -p rectilinear-infill --test bridge_infill_emission_tdd`; return FACT pass/fail." — purpose: validate Step 5.
- "Run `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_gcode_contains_bridge_infill_evidence`; return FACT pass/fail." — purpose: validate Step 6.

## Data and Contract Notes

- IR or manifest contracts touched:
  - `BridgeRegion` — additive minor (5 new fields).
  - `SurfaceClassificationIR.schema_version` — `1.0.0` → `1.1.0`.
  - `SlicedRegion` — additive minor (2 new fields: `bridge_areas`, `bridge_orientation_deg`).
  - `SliceIR.schema_version` — `1.1.0` → `1.2.0` (this packet bumps; 12-rev1 already moved 1.0 → 1.1).
- WIT boundary considerations:
  - `slice-region-data` host record gains 2 fields. Per `docs/03 §WIT/Type Changes Checklist` — search every `wit_host.rs`, `dispatch.rs`, and `wit_guest` for the affected type and update.
  - Verify type identity matches across boundaries (e.g., `list<expolygon>` consistent everywhere).
  - Run `cargo build --tests` after WIT changes per the checklist.
- Determinism or scheduler constraints:
  - Mesh adjacency analysis is pure over `(MeshIR, MeshAnalysisConfig)`; deterministic.
  - Slice-time bridge assembly is pure over `(layer_z, region_polygons, bridge_regions, expansion_margin_mm)`; deterministic.
  - Polygon offset operations from Clipper-style libraries are deterministic when the input is deterministic.

## Locked Assumptions and Invariants

- `MeshIR.objects[*].mesh.indices` is in triangle order (3 indices per facet); same assumption used by `mesh_analysis.rs:146-178`.
- The mesh is "manifold enough" for half-edge analysis — i.e., each interior edge is shared by exactly 2 facets. Non-manifold meshes (T-junctions, missing edges) yield degraded but valid `BridgeRegion` metrics; do NOT panic.
- 100 nm/unit coordinate convention.
- Polygon offsets use Clipper-style `MitterLimit`/`RoundJoin` semantics with a small mitter limit to handle sharp anchor corners (negative test case explicitly covers self-intersection avoidance).

## Risks and Tradeoffs

- **Mesh adjacency edge cases.** Non-manifold meshes (very common for STL files) may produce incomplete half-edge graphs. Strategy: degrade gracefully — if anchor-width cannot be computed, fall back to `f32::INFINITY` (passes the filter); if span cannot be computed, fall back to `0.0` (fails the filter). Never panic on real-world STLs.
- **Polygon offset producing degenerate output.** Mitigated by the offset round-join + an explicit validation check (negative test case).
- **WIT change blast radius.** Per `docs/03` checklist, this is the real risk. Step 4 explicitly searches every `wit_host.rs` / `dispatch.rs` / `wit_guest` site for `slice-region-data` and updates them in lockstep.
- **WASM rebuild break.** All infill-stage modules link against the SDK; if the SDK API changes in a non-additive way, every module must rebuild. Step 5 verifies via the build script before declaring victory.
- **Performance.** Mesh adjacency is one-time at PrePass (cheap for Benchy-scale meshes). Polygon offset at slice time is per-region, per-layer; for typical objects under 5 bridge regions per layer, sub-millisecond.

## Context Cost Estimate

- Aggregate: `M`.
- Largest single step: `M` (Step 4: WIT extension + 3 coupled file edits).
- Highest-risk dispatch: WASM rebuild (`./modules/core-modules/build-core-modules.sh`) — pass FACT-only return capturing the failing module name on failure.

## Open Questions

- Step 0 dispatch resolves: does `slicer-helpers` already expose a Minkowski offset? If yes, use it; if no, this packet adds a thin wrapper around Clipper. Either way, S cost.
- Step 0 dispatch resolves: numeric defaults from Orca (`min_bridge_length`, `anchor_width_mm`, `expansion_margin`). Likely values: `min_bridge_length = 5.0` mm, `anchor_width_mm = 0.5` mm × extrusion-width-multiplier, `expansion_margin = 1.0` mm — all subject to confirmation.
