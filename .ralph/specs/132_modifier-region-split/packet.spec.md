---
status: draft
packet: 132_modifier-region-split
task_ids:
  - TASK-257
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 132_modifier-region-split

## Goal

Make modifier volumes geometrically real: slice each modifier mesh per layer, intersect with
the owning region's partitioned fill polygons, and split them into wall-less sub-regions that
carry their own region identity + config binding (`ModifierScope` beyond `AllFeatures`) while
sharing the base region's walls (`wall_source_region_id = base`) — per ADR-0030.

## Scope Boundaries

Host-only geometry and config-binding work: modifier-mesh slicing, fill-polygon splitting at
region partition, `ModifierScope` extension, and `wall-source-region-id` population for
modifier sub-regions. No WIT change (the contract fields shipped in 130; the config accessor
in 131). No perimeter generation at modifier boundaries — walls stay merged on the base. No
e2e 3MF fixture (that is M3, packet 136); tests here construct objects + modifier volumes
programmatically.

## Prerequisites and Blockers

- Depends on: `130_infill-postprocess-contract` (`wall-source-region-id` field exists),
  `131_per-region-config-delivery` (sub-region config is deliverable and testable).
- Unblocks: `133_infill-linker-module` (real wall-less-sibling fixtures), `136` (M3 e2e).
- Activation blockers: none — semantics locked by ADR-0030; the exact IR plumbing is bounded
  by Step 1's discovery contract (see `design.md` §Open Questions).

## Acceptance Criteria

- **AC-1. Given** a rectangular object region with a centered modifier volume overlapping the
  layer, **when** region partition runs, **then** the base region's `sparse_infill_area`
  excludes the modifier footprint, the sub-region's `sparse_infill_area` equals the
  footprint∩wall-inset intersection, and the union of both equals the pre-split
  `sparse_infill_area` within 1% area tolerance. | `cargo test -p slicer-runtime --test executor -- modifier_split_partition_conservation 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-2. Given** the split from AC-1, **when** `run_infill_postprocess` views are built,
  **then** the sub-region's `wall_source_region_id == Some(<base region_id>)` and the base's
  is `None`. | `cargo test -p slicer-runtime --test executor -- modifier_split_wall_source 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-3. Given** the split from AC-1, **when** `Layer::Perimeters` output commits, **then**
  `PerimeterIR` contains wall loops ONLY for the base region — zero wall loops keyed to the
  sub-region. | `cargo test -p slicer-runtime --test executor -- modifier_split_no_subregion_walls 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-4. Given** a modifier carrying `infill_density = 0.40` over a base of `0.15`, **when**
  a test guest reads `infill_density` per region via the packet-131 accessor, **then** it
  reads 0.40 inside the sub-region and 0.15 on the base. | `cargo test -p slicer-runtime --test contract -- modifier_split_subregion_density 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-5. Given** a layer whose Z lies above the modifier volume's top, **when** the layer is
  processed, **then** no sub-region exists on that layer (region set identical to the
  no-modifier case). | `cargo test -p slicer-runtime --test executor -- modifier_split_z_scoping 2>&1 | tee target/test-output.log | grep "^test result"`

## Negative Test Cases

- **AC-N1. Given** an object with no modifier volumes, **when** sliced, **then** `SliceIR`
  and the partition output are identical to pre-packet behavior, and the
  `resources/regression_wedge.stl` default-config g-code SHA is byte-identical. | `cargo test -p slicer-runtime --test e2e -- wedge 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-N2. Given** a modifier mesh whose slice at the layer Z is degenerate (empty or
  zero-area intersection with the region), **when** the split runs, **then** no sub-region is
  created and no panic occurs (fallback: base config everywhere). | `cargo test -p slicer-runtime --test executor -- modifier_split_degenerate_no_split 2>&1 | tee target/test-output.log | grep "^test result"`

## Verification

- `cargo test -p slicer-runtime --test executor -- modifier_split 2>&1 | tee target/test-output.log | grep "^test result"`
- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`

## Authoritative Docs

- `docs/adr/0030-modifier-splits-fill-not-perimeters.md` — short; load in full (binding).
- `docs/specs/modifier-region-infill.md` §Phase M1 — short; load in full.
- `docs/02_ir_schemas.md` — delegate; `SlicedRegion` / `RegionMapIR` sections only.
- `docs/08_coordinate_system.md` — delegate a SUMMARY if unit questions arise.

## Doc Impact Statement (Required)

- `docs/02_ir_schemas.md` §region partition / SlicedRegion — modifier sub-region semantics
  (wall-less, shares base walls, per-sub-region config binding) — `rg -q 'modifier sub-region' docs/02_ir_schemas.md`

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
