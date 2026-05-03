# Design: external-surface-classification-at-slice

## Controlling Code Paths

- Primary code path:
  - `crates/slicer-host/src/layer_slice.rs:48` — `execute_layer_slice` (host built-in `Layer::Slice`); the only place `SlicedRegion` instances are produced for the live path.
  - `crates/slicer-host/src/wit_host.rs:2517` — `sliced_region_to_data` converts `SlicedRegion` to the WIT `SliceRegionData` resource handed to guest infill modules.
  - `crates/slicer-host/src/layer_executor.rs:295-310` — production caller of `execute_layer_slice`; reads from `Blackboard`.
- Neighboring tests or fixtures:
  - `crates/slicer-host/tests/layer_slice_tdd.rs` — existing `execute_layer_slice` regressions (8 callers; all need the new arguments).
  - `crates/slicer-host/tests/live_top_bottom_fill_tdd.rs` — commit-side preservation tests (must remain green).
  - `crates/slicer-host/tests/benchy_end_to_end_tdd.rs:1164,1368` — the two failing acceptance tests this packet unlocks.
- OrcaSlicer comparison surface:
  - `OrcaSlicerDocumented/src/libslic3r/LayerRegion.cpp` — `process_external_surfaces` (parity reference, not a port — we use facet projection rather than inter-layer subtraction).

## Architecture Constraints

- **No new fine-layer-height slicing pass.** `docs/04_host_scheduler.md` reserves prepass slicing for coarser support layers; this packet must not add a new slicing pass.
- **No change to per-layer parallel execution.** `docs/04_host_scheduler.md §Per-Layer Execution` runs layers via `par_iter`; no synchronization between layer N and N±1 may be introduced.
- **Use `PrePass::MeshAnalysis` output as the classification source.** `SurfaceClassificationIR` is already on the blackboard (`Arc<SurfaceClassificationIR>`, immutable post-prepass).
- **No WIT, SDK, manifest, or dispatch signature changes.** The flags ride on the existing `SlicedRegion` IR struct.
- **Schema version bump is additive-minor.** New booleans default to `false`; serialized v1.0.0 deserializes via Serde defaulting (verify via `crates/slicer-ir/tests/ir_tests.rs` round-trip).

## Code Change Surface

- Selected approach:
  - Extend `SlicedRegion` with three boolean fields; populate them inside `execute_layer_slice` using a local `classify_region_surfaces` helper that consults `SurfaceClassificationIR` (per-facet `FacetClass` and `bridge_regions`) and the layer's adjacent Z values.
  - Per-facet test: world-Z window + any-vertex-XY-in-region-polygon. Coarser than full polygon-polygon intersection but sufficient for the Benchy parity tests this packet must satisfy. Documented as a deviation; tightening is packet 36's responsibility.
  - WIT-data conversion at `wit_host.rs:2545-2547` reads the new fields.
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - `crates/slicer-ir/src/slice_ir.rs:1006-1021` — add `is_top_surface`, `is_bottom_surface`, `is_bridge` to `SlicedRegion`. Update `SliceIR.schema_version` constant from `1.0.0` to `1.1.0`.
  - `crates/slicer-host/src/layer_slice.rs` — extend `execute_layer_slice` signature; add private `classify_region_surfaces` helper; populate the new fields.
  - `crates/slicer-host/src/layer_executor.rs:295-310` — production caller adapts to new signature.
  - `crates/slicer-host/src/wit_host.rs:2545-2547` — replace hardcoded `false`s with reads off `SlicedRegion`.
  - `crates/slicer-host/tests/external_surface_classification_tdd.rs` — NEW — covers all `packet.spec.md` ACs.
  - Mechanical updates (literal `SlicedRegion {}` constructors): `crates/slicer-host/tests/dispatch_tdd.rs:2188`, `crates/slicer-host/tests/live_layer_support_tdd.rs:476,1241,1560`, `crates/slicer-host/tests/macro_all_worlds_roundtrip_tdd.rs:418`, `crates/slicer-host/tests/slice_postprocess_paint_annotation_tdd.rs:307`, `crates/slicer-ir/tests/ir_tests.rs:439`. Each gets `is_top_surface: false, is_bottom_surface: false, is_bridge: false`.
  - Mechanical updates (`execute_layer_slice` test callers): `crates/slicer-host/tests/layer_slice_tdd.rs:145,159,256,257,319,369,370,444`. Each adds `None, None, None`.
- Rejected alternatives that were considered and why they were not chosen:
  - **New PrePass classification stage** — rejected: requires the implementer to either re-slice all layers (forbidden by architecture constraint) or add inter-layer state (forbidden by the parallel constraint).
  - **Inter-layer polygon subtraction at dispatch time** — rejected: would require synchronization or a global pre-pass; conflicts with `docs/04 §Per-Layer Execution`.
  - **New `ExternalSurfaceIR` blackboard slot threaded through `dispatch.rs`** — rejected: introduces a new IR + new threading + new prepass stage for data that can ride on the existing `SlicedRegion`.
  - **Compute flags inside `wit_host::sliced_region_to_data` directly** — rejected: that function would need to take `MeshIR + SurfaceClassificationIR + adjacent Z values`, blowing the WIT-data conversion's narrow purpose.

## Files in Scope (read + edit)

Primary edit targets (≤ 3 per step; aggregate ≤ 4 across the packet):

- `crates/slicer-ir/src/slice_ir.rs` — role: schema definition; expected change: add 3 fields to `SlicedRegion`, bump `SliceIR.schema_version`.
- `crates/slicer-host/src/layer_slice.rs` — role: host built-in slice + new classifier; expected change: extend `execute_layer_slice` signature, populate flags, add `classify_region_surfaces` helper.
- `crates/slicer-host/src/layer_executor.rs` — role: production caller; expected change: thread `surface_classification` and adjacent-layer Z from blackboard.
- `crates/slicer-host/src/wit_host.rs` — role: WIT-data conversion; expected change: replace 3 hardcoded `false`s with reads off `SlicedRegion`. Lines 2545–2547 only.

New file:

- `crates/slicer-host/tests/external_surface_classification_tdd.rs` — role: TDD; expected: ≥ 7 tests covering ACs and negative cases.

Mechanical edits (small, exhaustive list — see Code Change Surface above): 6 test files in `crates/slicer-host/tests/` and 1 in `crates/slicer-ir/tests/`. These edits are 1-3 lines each.

## Read-Only Context

- `crates/slicer-host/src/mesh_analysis.rs` — read lines `113-330` only — purpose: confirm the world-Z transform and FacetClass writeout. Do NOT modify.
- `crates/slicer-host/src/blackboard.rs` — read lines `192-220` only — purpose: confirm the `surface_classification()` accessor pattern.
- `docs/02_ir_schemas.md` — read the `SliceIR` and `SurfaceClassificationIR` sections only (search by symbol). Confirm the additive-minor versioning rule.
- `docs/08_coordinate_system.md` — read directly. Confirm the 100 nm/unit conversion.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` — delegate parity checks; never load.
- `target/`, `Cargo.lock`, generated code — never load.
- `crates/slicer-host/src/dispatch.rs` — > 2000 lines; this packet does NOT touch dispatch.
- `crates/slicer-host/src/prepass.rs` — this packet does NOT add a prepass stage.
- `wit/` — no WIT signature changes.
- `crates/slicer-sdk/` — no SDK changes; the trait surface is already complete.
- `modules/core-modules/` — no module changes; `rectilinear-infill` already reads the SDK flags.

## Expected Sub-Agent Dispatches

- "Run `cargo build --workspace`; return FACT (pass) or SNIPPETS (compilation error with file:line + ≤ 20 lines)" — purpose: validate Step 1 schema bump compiles.
- "Run `cargo test -p slicer-host --test external_surface_classification_tdd`; return FACT pass/fail with the failing test name + assertion only" — purpose: validate Steps 2–4 TDD.
- "Run `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_gcode_contains_top_and_bottom_surface_evidence benchy_feature_evidence_failures_name_the_missing_family`; return FACT pass/fail" — purpose: validate the user-visible acceptance.
- "Find every literal `SlicedRegion {` constructor across `crates/`; return LOCATIONS" — purpose: enumerate mechanical fix-ups for Step 1.
- "Summarize `docs/02_ir_schemas.md` § SliceIR additive-minor rule; return FACT (one sentence with the exact rule)" — purpose: confirm schema-bump compliance for Step 1.
- "Summarize `docs/04_host_scheduler.md` § Per-Layer Execution; return SUMMARY ≤ 200 words" — purpose: confirm no parallelism violations.

## Data and Contract Notes

- IR or manifest contracts touched:
  - `slicer_ir::SlicedRegion` — additive minor (3 boolean fields).
  - `slicer_ir::SliceIR.schema_version` — `1.0.0` → `1.1.0`.
  - `SurfaceClassificationIR` and `LayerPlanIR` — read-only; no schema change.
- WIT boundary considerations:
  - The `slice-region-data` WIT host record (`crates/slicer-host/src/wit_host.rs:140-142`) already carries `is_top_surface`, `is_bottom_surface`, `is_bridge`. This packet only fills them with non-`false` values; no WIT changes.
- Determinism or scheduler constraints:
  - `classify_region_surfaces` is a pure function over `(object_mesh, surface_data, region_polygons, layer_z, next_layer_z, prev_layer_z)`; deterministic.
  - `execute_layer_slice` remains pure given its inputs; per-layer parallelism is preserved because `SurfaceClassificationIR` is `Arc`-shared and read-only.

## Locked Assumptions and Invariants

- `PrePass::MeshAnalysis` is the sole producer of `FacetClass::TopSurface` / `BottomSurface` / `Bridge` and `BridgeRegion.facet_indices`. Any future re-classification mechanism must commit to the same `SurfaceClassificationIR` contract.
- `LayerPlanIR.global_layers` is sorted by ascending Z (per `docs/02_ir_schemas.md`); the implementer relies on `global_layers[i+1].z` and `global_layers[i-1].z` being monotonically adjacent.
- 100 nm/unit coordinate convention; facet-vertex XY conversion via `Point2::from_mm` / `mm_to_units` from `slicer-helpers`.
- `Blackboard::surface_classification()` returns `Some` whenever the prepass has run; `None` is only valid for synthetic test fixtures.

## Risks and Tradeoffs

- **Centroid / any-vertex-in-polygon false negatives.** A facet whose centroid sits outside the region polygon but whose XY footprint partially overlaps is missed. Acceptable for Benchy parity; tightening is packet 36's job. Registered in `docs/DEVIATION_LOG.md`.
- **Bridge classification depends on `SurfaceClassificationIR.bridge_regions` already being populated.** If `mesh_analysis.rs` does not currently populate `bridge_regions[*].facet_indices` for typical objects, bridge flagging will never light up. The first implementation step verifies this assumption before building on it; if `bridge_regions` is currently empty, a small additional fix in `mesh_analysis.rs` is in scope (else escalate to packet 36).
- **Schema bump cascade.** Tests pattern-matching `SlicedRegion` exhaustively must add the 3 new fields. Enumerated in the Code Change Surface; mechanical.

## Context Cost Estimate

- Aggregate: `M` (4 primary edits + 7 mechanical fix-ups + 1 new test file).
- Largest single step: `M` (Step 4: `execute_layer_slice` extension and classifier helper).
- Highest-risk dispatch: `cargo test --workspace` after Step 5 — could return long output. Mitigation: dispatch with FACT-only return ("pass" or "the failing test name + assertion + ≤ 20 lines").

## Open Questions

- None blocking activation. Bridge-region population in `mesh_analysis.rs` is verified in Step 0 (read-only discovery); if absent, the Step-3 plan absorbs a small fix-up in `mesh_analysis.rs` rather than escalating.
