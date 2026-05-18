# Requirements: external-surface-classification-at-slice

## Packet Metadata

- Grouped task IDs:
  - `TASK-164` (NEW — to be added to `docs/07_implementation_status.md` as a follow-up under TASK-120a)
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `active`
- Aggregate context cost: `M`

## Problem Statement

Two acceptance tests in `crates/slicer-host/tests/benchy_end_to_end_tdd.rs` are failing on the live Benchy path:

- `benchy_gcode_contains_top_and_bottom_surface_evidence` — expects `;TYPE:Top surface` and `;TYPE:Bottom surface` blocks.
- `benchy_feature_evidence_failures_name_the_missing_family` — names `top_surface` as the missing family.

Packet `12_live-top-bottom-surface-fill` shipped only the *receiving half* of the contract: SDK fields on `SliceRegionView` (`crates/slicer-sdk/src/views.rs:35,38,41`), role-selection logic in `modules/core-modules/rectilinear-infill/src/lib.rs:108-116`, and `convert_infill_output` role preservation through commit. The *sending half* — populating those flags from prepass classification down into the live `SliceRegionData` resource — was never wired. `crates/slicer-host/src/wit_host.rs:2545-2547` hardcodes the three flags to `false`, so every region reaches the infill module looking like sparse and the markers are never emitted.

This packet narrows packet 12 by closing the host-side wiring without expanding scope: it uses the per-facet classification already produced by `PrePass::MeshAnalysis` (`crates/slicer-host/src/mesh_analysis.rs:281-323`) and projects it onto layer regions inside the existing per-layer host built-in `execute_layer_slice` (`crates/slicer-host/src/layer_slice.rs:48`). No new prepass, no extra slicing pass, no parallelism changes, no IR threading through dispatch.

## In Scope

- Schema-additive `is_top_surface`, `is_bottom_surface`, `is_bridge` boolean fields on `slicer_ir::SlicedRegion` (`crates/slicer-ir/src/slice_ir.rs:1006-1021`).
- `SliceIR.schema_version` minor bump `1.0.0` → `1.1.0`.
- New private helper `classify_region_surfaces(object_mesh, surface_data, region_polygons, layer_z, next_layer_z, prev_layer_z) -> (bool, bool, bool)` colocated in `crates/slicer-host/src/layer_slice.rs`.
- Extended signature `execute_layer_slice(mesh, layer, surface_class: Option<&SurfaceClassificationIR>, next_layer_z: Option<f32>, prev_layer_z: Option<f32>)`.
- Production caller `layer_executor.rs:295-310` reads `blackboard.surface_classification()` and the adjacent layers from `blackboard.layer_plan().global_layers`.
- WIT-data conversion `wit_host.rs:2545-2547` reads the three new fields off `SlicedRegion`.
- New TDD file `crates/slicer-host/tests/external_surface_classification_tdd.rs` covering the cases listed in `packet.spec.md`.
- Mechanical updates to literal `SlicedRegion {}` constructions across the workspace and `execute_layer_slice` test callers.
- One-line `docs/02_ir_schemas.md` schema-bump entry; one-line `docs/DEVIATION_LOG.md` entry; `docs/14_deviation_audit_history.md` reference; `docs/07_implementation_status.md` TASK-164 entry.

## Out of Scope

- Polygon-polygon overlap or any other replacement for the any-vertex-in-polygon approximation (`docs/DEVIATION_LOG.md` registered).
- Multi-layer top/bottom thickness — packet 35.
- Full Orca bridge-detector parity (anchor width, min-bridge-length, expansion margins) — packet 36.
- Per-surface fill pattern/density variation — packet 37.
- Top-surface ironing — packet 38.
- WIT or SDK signature changes (the `slice-region-data` host record at `wit_host.rs:140-142` already carries the flag fields; SDK already exposes setters/getters at `slicer-sdk/src/views.rs:119-174`).
- WASM core-module rebuild (no WIT signatures change).
- Dispatch-path threading of any new IR.
- Bridge-region polygon assembly or `bridge_areas` field — out of scope for this packet (packet 36 owns it).

## Authoritative Docs

- `docs/01_system_architecture.md` — pipeline tiers; § "Tier 2 — Per-Layer" only. Document is > 300 lines: delegate SUMMARY.
- `docs/02_ir_schemas.md` — `SurfaceClassificationIR`, `SliceIR`, `SlicedRegion` schemas; additive-minor versioning rule. Read directly; only the relevant sections (search by symbol).
- `docs/04_host_scheduler.md` — Per-Layer Execution; Blackboard Structure. Document is > 600 lines: delegate SUMMARY of those two sections only.
- `docs/08_coordinate_system.md` — 100 nm/unit, `Point2::from_mm` / `mm_to_units`. Read directly.

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/LayerRegion.cpp` — `LayerRegion::process_external_surfaces()` is the parity touchstone for top/bottom surface assignment. We deliberately do NOT borrow Orca's inter-layer polygon-subtraction; we use mesh-facet projection through `SurfaceClassificationIR` instead. Document the divergence in `docs/DEVIATION_LOG.md`.
- `OrcaSlicerDocumented/src/libslic3r/Surface.hpp` — Surface types `stTop`, `stBottom`, `stBottomBridge` define the role taxonomy our `ExtrusionRole::TopSolidInfill` / `BottomSolidInfill` / `BridgeInfill` mirror.

All OrcaSlicer reads MUST be delegated; never load this tree into the implementer's own context.

## Acceptance Summary

- Positive cases (see `packet.spec.md` §Acceptance Criteria):
  - `classify_region_surfaces` flags top, bottom, and bridge correctly when facets satisfy z-window + centroid-in-polygon.
  - `execute_layer_slice` writes the three flags onto each `SlicedRegion`.
  - Benchy E2E G-code contains `;TYPE:Top surface` and `;TYPE:Bottom surface` blocks.
  - `SliceIR.schema_version == 1.1.0`.
- Negative cases (see `packet.spec.md` §Negative Test Cases):
  - `surface_classification: None` keeps flags `false` (preserves existing test fixtures).
  - Centroid outside region polygon does not flag.
  - Z window mismatch does not flag.
- Measurable outcomes:
  - Two pre-existing failing tests now PASS.
  - `cargo test --workspace` PASS.
  - `cargo clippy --workspace -- -D warnings` PASS.
- Cross-packet impact:
  - Unblocks packet 35 (`multi-layer-top-bottom-thickness`), packet 36 (`bridge-detector-orca-parity`), packet 38 (`top-surface-ironing`).

## Verification Commands

- `cargo test -p slicer-host --test external_surface_classification_tdd -- --nocapture`
- `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_gcode_contains_top_and_bottom_surface_evidence benchy_feature_evidence_failures_name_the_missing_family -- --nocapture`
- `cargo test -p slicer-ir slice_ir_schema_version_is_one_one_zero -- --nocapture`
- `cargo test --workspace`
- `cargo clippy --workspace -- -D warnings`

All commands above are delegation-friendly; pass/fail is observable from exit code or a single failing assertion.

## Step Completion Expectations

For each step in `implementation-plan.md`, the following must be captured (see that file for per-step values):

- Precondition stated explicitly.
- Postcondition stated as an observable artifact (file change, passing test, schema field present).
- Falsifying check: a single command that fails until the step is done.
- Files allowed to read: line ranges where the file is > 300 lines.
- Files allowed to edit: ≤ 3.
- Expected sub-agent dispatches: at minimum the `cargo test` execution as FACT pass/fail.
- Step context cost: S or M (no L).

## Context Discipline Notes

- Large files in the read-only path that MUST be ranged or delegated:
  - `docs/04_host_scheduler.md` (> 600 lines) — delegate.
  - `crates/slicer-host/src/wit_host.rs` (~3000 lines) — read only the `sliced_region_to_data` function and surrounding 60 lines (lines `2517-2580`).
  - `crates/slicer-host/src/dispatch.rs` (> 2000 lines) — out of scope; do NOT open.
- OrcaSlicer trees the implementer must NOT load directly:
  - `OrcaSlicerDocumented/` — every reference cited must be delegated.
- Likely temptation reads (and why to skip):
  - `crates/slicer-host/src/dispatch.rs` — the prior plan iterations wanted to thread surface classification through dispatch. This packet does NOT touch dispatch; flags travel inside `SlicedRegion`.
  - `crates/slicer-host/src/prepass.rs` — this packet does NOT add a new prepass.
  - `wit/` — no WIT signature changes; do not open.
- Sub-agent return-format hints for the heaviest dispatches:
  - cargo runs → FACT (PASS / FAIL with the failing assertion only)
  - `docs/04_host_scheduler.md` summary → SUMMARY ≤ 200 words, focused on Per-Layer Execution + Blackboard Structure
  - OrcaSlicer parity verification → FACT (yes/no the cited file contains the expected symbol)
