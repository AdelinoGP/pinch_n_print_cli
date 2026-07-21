---
status: draft
packet: 139_lightning-layer-generator
task_ids:
  - TASK-264
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 139_lightning-layer-generator

## Goal

Port the lightning orchestration — `Lightning::Layer` (`generateNewTrees`,
`reconnectRoots`, `convertToLines`) and `Generator` (`generateInitialInternalOverhangs`
+ the two top-down all-layers passes of `generateTrees`, `getTreesForLayer`) — into
`crates/slicer-core/src/algos/lightning/`, wire the packet-137 producer so
`PrePass::LightningTreeGen` commits real per-layer tree segments into
`LightningTreeIR`, and add the per-region refinement that closes
`D-137-LIGHTNING-PER-OBJECT-COLLAPSE` (`region_id: RegionId` on `LightningTreeEntry`,
per-region HashMap keying in the host dispatch, `region_id` honored by the SDK
accessor).

## Scope Boundaries

Host-side orchestration port plus producer wiring — completes the generation pipeline
behind the 137 seam. The `lightning-infill` module still runs its stub (rewired in 140),
so no user-visible output changes for lightning prints yet, and non-lightning prints stay
skipped (skip promise preserved). Cross-layer behavior (overhang seeding, continuity,
determinism) is the test focus. The 138 primitive APIs are frozen at this packet's start
— any signature change forced by the orchestration port is a recorded deviation here, with
the 138 tests co-updated in the same step, never left red between steps.

## Prerequisites and Blockers

- **FORWARD-DEP on `137_lightning-prepass-contract`** (status: `implemented`) —
  packet 139 needs `crates/slicer-core/src/algos/lightning/mod.rs` (skeleton
  with `generate_lightning_trees(...)` + `// 139 wiring point` marker),
  `LightningTreeIR` with the 2-point integer-unit `tree_edge_segments` shape,
  the blackboard commit slot + accessor, and the WIT read-view
  `lightning-tree-segments` method on `paint-region-layer-view`. Names +
  shapes match 137's plan; reconciled at 137 close.
- **FORWARD-DEP on `138_lightning-distancefield-treenode`** (status: `draft`) —
  packet 139 needs `DistanceField::{new, unsupported_point, update}` and the
  `tree_node` graph operations (propagate, straighten, reroot, prune) frozen
  at 138 close. 138's API freeze is recorded in 138's `requirements.md`
  §Step Completion Expectations; 139 records any signature change as a
  deviation in the same step.
- **DEVIATION-CLOSURE DEP on packet 137's review** — this packet must add
  `region_id: RegionId` to `LightningTreeEntry` (mirroring
  `SupportPlanEntry.region_id` at `crates/slicer-ir/src/slice_ir.rs:1129`),
  update the host dispatch HashMap keying in
  `crates/slicer-wasm-host/src/dispatch.rs:1383` from `wildcard_region = "*"`
  to the actual `region_id`, and update the SDK accessor
  `lightning_tree_segments_for` in `crates/slicer-sdk/src/traits.rs:195-199`
  to honor its `region_id` argument. Closes
  `D-137-LIGHTNING-PER-OBJECT-COLLAPSE` in `docs/DEVIATION_LOG.md`.
- Unblocks: `140_lightning-module-rewrite`.
- Activation blockers: 137 and 138 must both be `status: implemented`
  (forward-deps above).

## Acceptance Criteria

- **AC-1. Given** a two-layer synthetic object where layer `N`'s sparse outline extends
  beyond layer `N-1`'s, **when** `generate_initial_internal_overhangs` runs, **then** the
  overhang region for layer `N` equals `outline(N) − dilated(outline(N-1))` (ported
  dilation constant, ÷100), within Clipper tolerance. | `cargo test -p slicer-core -- lightning_generator_overhangs 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-2. Given** a synthetic prism with a single internal overhang near its top, **when**
  `generate_trees` runs top-down, **then** trees exist on every layer between the overhang
  and its support ground, and each layer's tree endpoints lie within the per-layer move
  distance of the layer below's trees or outline (continuity, ported bound). | `cargo test -p slicer-core -- lightning_generator_tree_continuity 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-3. Given** a generated object with at least two `SliceRegion`s on the same
  `(object, layer)`, **when** the producer commits the per-layer `LightningTreeIR`, **then**
  each `LightningTreeEntry` carries its `region_id` and the host dispatch's
  `lightning_tree_segments` HashMap (`crates/slicer-wasm-host/src/dispatch.rs:1383`)
  keys on the actual `region_id` (not the wildcard `*` from packet 137's skeleton) — two
  regions on the same `(object, layer)` get distinct segment buckets; the SDK accessor
  `lightning_tree_segments_for(object_id, region_id)` returns exactly the queried region's
  segments. | `cargo test -p slicer-runtime --test executor -- lightning_producer_per_region_keying 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-4. Given** the same input run twice, **when** the committed `LightningTreeIR`s are
  compared, **then** they are byte-identical (whole-pipeline determinism over the 138
  primitives) — and the per-region keying is stable (the same input produces the same
  `(region_id → segments)` map across runs). | `cargo test -p slicer-core -- lightning_generator_deterministic 2>&1 | tee target/test-output.log | grep "^test result"`

## Negative Test Cases

- **AC-N1. Given** a uniform prism with no internal overhangs, **when** generation runs,
  **then** the committed `LightningTreeIR` is valid with zero tree segments on every
  layer (no spurious trees). | `cargo test -p slicer-core -- lightning_generator_no_overhang_no_trees 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-N2. Given** a default-config wedge slice (no lightning holder), **when** run,
  **then** the g-code SHA is byte-identical (skip path untouched). | `cargo test -p slicer-runtime --test e2e -- wedge 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-N3. Given** two regions on the same `(object, layer)` with different committed
  segments, **when** `PaintRegionLayerView::lightning_tree_segments_for(object_id,
  region_id)` is called, **then** it returns only the queried region's segments (no
  cross-region leakage) — the `region_id` argument is honored, not discarded. |
  `cargo test -p slicer-runtime --test contract -- lightning_tree_per_region_roundtrip 2>&1 | tee target/test-output.log | grep "^test result"`

## Verification

- `cargo test -p slicer-core -- lightning 2>&1 | tee target/test-output.log | grep "^test result"`
- `cargo test -p slicer-runtime --test executor -- lightning 2>&1 | tee target/test-output.log | grep "^test result"`
- `cargo clippy --workspace --all-targets -- -D warnings`

## Authoritative Docs

- `docs/specs/lightning-infill-parity.md` §Phase L3 — full read (short).
- `docs/adr/0029-lightning-prepass-tree-generator.md` — delegate SUMMARY.
- `docs/ORCASLICER_ATTRIBUTION.md` — header template.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/Layer.hpp` (92 lines) / `.cpp` (448 lines) — `generateNewTrees`, `reconnectRoots`, `convertToLines` (sectioned dispatches; the 540-line total is the largest single read in this packet — ≥ 4 sections).
- `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/Generator.hpp` (138 lines) / `.cpp` (285 lines) — constructor inputs (with density coupling), `generateInitialInternalOverhangs`, `generateTrees` two-pass structure, `getTreesForLayer` (sectioned).
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillLightning.cpp` (37 lines) / `.hpp` (42 lines) — `build_generator` inputs (the density coupling handed to the generator constructor).

## Doc Impact Statement (Required)

**`none`** — completes the implementation behind the packet-137 contract; the IR, stage,
and view documentation landed with 137 (docs/02 + docs/03), and the architecture with
ADR-0029. No new public surface.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
