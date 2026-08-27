---
status: draft
packet: 243-object-scoped-overhang-annotation
task_ids:
  - TASK-353
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 243-object-scoped-overhang-annotation

## Goal

Make both host-only overhang annotation maps on `SurfaceClassificationIR` object-scoped
(`HashMap<ObjectId, HashMap<u32, …>>`) with a major `SurfaceClassificationIR` schema bump
(1.3.0 → 2.0.0), so the marshal and the two perimeter consumers stop measuring against other
objects' boundaries in multi-object scenes.

## Scope Boundaries

This is a host-only field-type replacement on `SurfaceClassificationIR` plus the marshal re-keying
and the mechanical test fallout it forces. The WIT accessor signatures (`SliceRegionView`'s
`overhang_quartile_polygons()` / `prev_layer_boundary()`) are unchanged — the object dimension is
resolved host-side at the marshal lookup, never crossed over WIT. No new geometry, no new config
keys, no scheduler or module changes; the perimeter modules (`classic-perimeters`,
`arachne-perimeters`) need no code change because they consume the unchanged view accessor.

## Prerequisites and Blockers

- Depends on: nothing (first packet in the wave-overhangs queue).
- Unblocks: 244-order-locked-extrusion-sequences (which consumes the object-scoped
  `prev_layer_boundaries` shape as the `prev_object_boundary` source for wave bridge fill).
- Activation blockers: none known; the change is host-only and reversible via the schema constant.

## Acceptance Criteria

State ACs only here; `requirements.md` references their IDs.

- **AC-1. Given** the tree with `CURRENT_SURFACE_CLASSIFICATION_SCHEMA_VERSION` bumped to the major
  `SemVer { major: 2, minor: 0, patch: 0 }`, **when** the constant-sourced schema test runs, **then**
  `bridge_detector_schema_versions_are_constant_sourced` asserts the constant equals
  `SemVer { major: 2, minor: 0, patch: 0 }` and `SurfaceClassificationIR::default().schema_version`
  equals the constant. |
  `cargo test -p slicer-ir --test ir_tests -- bridge_detector_schema_versions_are_constant_sourced --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed" && echo P243_SCHEMA_2_0_0`
- **AC-2. Given** a two-object mesh (`two_object_overhang_mesh()` in
  `crates/slicer-runtime/tests/executor/prepass_overhang_annotation_stage_order_tdd.rs`), **when**
  `commit_overhang_annotation_builtin` runs, **then** `SurfaceClassificationIR.overhang_quartile_polygons`
  is keyed by `ObjectId` first (exactly two keys, one per object) and each object's layer-1 entry
  carries only that object's own polygons — no cross-object quartile merge. |
  `cargo test -p slicer-runtime --test executor -- overhang_annotation_scopes_bands_by_object --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed" && echo P243_PRODUCER_OBJECT_SCOPED`
- **AC-3. Given** a two-object scene whose objects overlap in XY at the same global layer, **when**
  `sliced_region_to_data` (`crates/slicer-wasm-host/src/marshal/in_.rs`) assembles one object's
  `SliceRegionData`, **then** that region's `overhang_quartile_polygons` and `prev_layer_boundary`
  are read from `overhang_quartile_polygons.get(&region.object_id)` /
  `prev_layer_boundaries.get(&region.object_id)` and contain only that object's polygons. |
  `cargo test -p slicer-runtime --test contract -- overhang_areas_object_scoped_no_cross_object_leak --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed" && echo P243_MARSHAL_OBJECT_SCOPED`

## Negative Test Cases

- **AC-N1. Given** a two-object overlapping-footprint scene, **when** the marshal assembles object
  B's region view, **then** object A's boundary polygons are absent from B's `prev_layer_boundary`
  (the cross-object contamination that motivated this packet is rejected — a leak fails the test). |
  `cargo test -p slicer-runtime --test contract -- overhang_areas_object_scoped_no_cross_object_leak --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed" && echo P243_NO_CROSS_OBJECT_LEAK`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-ir --test ir_tests -- bridge_detector_schema_versions_are_constant_sourced --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed" && cargo test -p slicer-runtime --test executor -- overhang_annotation_scopes_bands_by_object --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed" && cargo test -p slicer-runtime --test contract -- overhang_areas_object_scoped_no_cross_object_leak --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed" && echo P243_ALL_ACS_PASS`

## Authoritative Docs

- `docs/02_ir_schemas.md` - direct range read of §"IR 2 — SurfaceClassificationIR" (lines ~387-400)
  and §"IR Versioning Contract" (lines ~1633-1641); the doc is over 300 lines so only these ranges
  are read directly.
- `docs/08_coordinate_system.md` - not read; this packet re-keys existing polygons, it does not
  convert units.

## Doc Impact Statement (Required)

- `docs/02_ir_schemas.md` §"IR 2 — SurfaceClassificationIR" - update `Current schema_version: 1.3.0`
  to `2.0.0` (major, field-type change per the IR Versioning Contract table) and rewrite the
  keying prose ("keyed by GLOBAL layer index" → "keyed by object id first, then global layer
  index") plus the packet-193 provenance note. |
  `rg -q 'Current schema_version: 2\.0\.0' docs/02_ir_schemas.md && rg -q 'keyed by object id first' docs/02_ir_schemas.md && echo P243_DOCS_UPDATED`

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
