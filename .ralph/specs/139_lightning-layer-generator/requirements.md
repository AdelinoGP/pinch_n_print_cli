# Requirements: 139_lightning-layer-generator

## Packet Metadata

- Grouped task IDs: `TASK-264`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

With the seam (137) and primitives (138) in place, the generator itself is still missing:
the 137 producer commits empty trees, so a lightning-configured print gets no benefit
from the new architecture. The orchestration is where OrcaSlicer's cross-layer semantics
live — the top-down overhang seeding and the two-pass tree growth — and it is the reason
the whole PrePass architecture exists (ADR-0029). This packet makes the seam real.

## In Scope

- `crates/slicer-core/src/algos/lightning/layer.rs` (new) — port of
  `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/Layer.{hpp,cpp}`
  (`generateNewTrees`, `reconnectRoots`, `convertToLines`); attribution header.
- `crates/slicer-core/src/algos/lightning/generator.rs` (new) — port of
  `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/Generator.{hpp,cpp}`
  (`generateInitialInternalOverhangs`, `generateTrees` two-pass, `getTreesForLayer`);
  attribution header; all constants ÷ 100.
- Producer wiring: the 137 skeleton's `// 139 wiring point` is replaced by the real
  per-object driver. `generate_lightning_trees(...)` builds the generator over the
  committed `SliceIR` sparse outlines (per-object, top-down) and stores the
  `convertToLines` output as `LightningTreeIR` entries.
- **DEVIATION CLOSURE (D-137-LIGHTNING-PER-OBJECT-COLLAPSE):**
  - `LightningTreeEntry` gains `region_id: RegionId` field (a `u64` type alias
    at `slice_ir.rs:36`; mirrors `SupportPlanEntry.region_id: RegionId` at
    `slice_ir.rs:1129` — same shape, same default of `0`). The
    `Default` impl stays derive-driven; the field is the per-region
    identifier within a `(object, layer)` pair. The IR's
    `tree_edge_segments: Vec<[Point2; 2]>` are scoped to
    `(object_id, region_id, global_layer_index)`.
  - The host dispatch HashMap keying at
    `crates/slicer-wasm-host/src/dispatch.rs:1383` is fixed: the `wildcard_region =
    String::from("*")` line is replaced by `entry.region_id.to_string()` (or
    equivalent decimal form). Mirrors the `support-plan-segments` keying at
    `dispatch.rs:1353`.
  - The SDK accessor `PaintRegionLayerView::lightning_tree_segments_for` at
    `crates/slicer-sdk/src/traits.rs:195-199` updates from `_region_id: u64` to
    `region_id: u64` (no longer underscore-prefixed) and filters entries by both
    `layer_index` and `region_id` (not just `object_id`).
  - A new SDK-level per-region roundtrip test (AC-N3) is added in
    `crates/slicer-runtime/tests/contract/lightning_tree_per_region_roundtrip_tdd.rs`
    (new file; register in `tests/contract/main.rs`): two regions on the same
    `(object, layer)`, each with distinct committed segments, and the accessor
    returns only the queried region's segments. The existing
    `lightning_tree_view_roundtrip_tdd.rs` (137) is updated to include a
    per-region assertion alongside the existing host-side projection test.
- Tests per AC-1…AC-4, AC-N1, AC-N2, AC-N3 (synthetic multi-layer fixtures
  constructed programmatically; no new resource files).

## Out of Scope

- The `lightning-infill` module (140) — it still runs its stub; the view still returns
  the committed trees unread by the module.
- `FillLightning::Filler` per-layer fill semantics beyond `convertToLines` (the
  module's sampling side, 140).
- Any tuning/divergence from Orca constants.

## Authoritative Docs

- `docs/specs/lightning-infill-parity.md` §L3 — full (short).
- `docs/adr/0029-lightning-prepass-tree-generator.md` — delegate SUMMARY (constructor
  sequence + memory note).
- `docs/ORCASLICER_ATTRIBUTION.md`; `docs/08_coordinate_system.md` (delegate).

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/Layer.cpp` (448 lines) — sectioned: tree seeding per layer, root reconnection, line conversion (≥ 4 sections).
- `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/Generator.cpp` (285 lines) — sectioned: constructor inputs, overhang pass, growth pass, layer accessor.
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillLightning.cpp` (37 lines) — `build_generator` construction inputs (the density coupling).

## Acceptance Summary

- Positive cases: `AC-1`–`AC-4` in `packet.spec.md`. Refinements: AC-2's continuity
  bound is THE cross-layer parity property (a tree that jumps farther than the move
  distance per layer is un-printable); AC-3 pins seam-fills-seam (generator output ==
  IR content) **at per-region granularity** (the 137 placeholder was per-object
  only; 139 adds `region_id` so two regions on the same `(object, layer)` get
  distinct segment buckets); AC-4 extends 138's determinism to the whole pipeline
  (now includes the per-region keying dimension).
- Negative cases: `AC-N1` (no overhang → no trees), `AC-N2` (wedge byte-identity),
  `AC-N3` (per-region accessor isolation — the `D-137-LIGHTNING-PER-OBJECT-COLLAPSE`
  closure proof).
- Cross-packet impact: 140 samples exactly what this packet commits, keyed by
  `region_id`; the per-layer segment ordering AND the per-region keying frozen
  here are 140's input contract.
- DEVIATION CLOSURE: this packet closes
  `D-137-LIGHTNING-PER-OBJECT-COLLAPSE` via the per-region IR field + dispatch
  keying + SDK projection. The skip-predicate remains print-wide (per the
  investigation's conclusion — `ResolvedConfig::sparse_fill_holder` is print-wide,
  not per-region, and the per-region predicate was the unrecoverable half of the
  deviation).

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-core -- lightning_generator 2>&1 \| tee target/test-output.log \| grep "^test result"` | AC-1/2/4/N1 | FACT + counts |
| `cargo test -p slicer-runtime --test executor -- lightning_producer_per_region_keying 2>&1 \| tee target/test-output.log \| grep "^test result"` | AC-3 per-region wiring | FACT |
| `cargo test -p slicer-runtime --test contract -- lightning_tree_per_region_roundtrip 2>&1 \| tee target/test-output.log \| grep "^test result"` | AC-N3 per-region SDK isolation | FACT |
| `cargo test -p slicer-runtime --test e2e -- wedge 2>&1 \| tee target/test-output.log \| grep "^test result"` | AC-N2 | FACT |
| `cargo clippy --workspace --all-targets -- -D warnings` + `cargo check --workspace --all-targets` | gates | FACT each |
| `cargo xtask build-guests --check` | workspace habit | FACT |

## Step Completion Expectations

- Cross-step invariant: the 138 primitive APIs are frozen — any signature change forced
  by the orchestration port is a recorded deviation here, with the 138 tests updated in
  the same step (never left red between steps).
- The 137 producer skeleton's `// 139 wiring point` comment is removed in Step 4 and the
  real driver body lands in its place.

## Context Discipline Notes

- Large files in the read-only path that MUST be ranged or delegated: `Layer.cpp` (448)
  and `Generator.cpp` (285) — sectioned dispatches only (≥ 4 sections each); the
  committed-`SliceIR` access pattern (delegate LOCATIONS from the support-geometry
  producer's input handling).
- Likely temptation reads: `FillLightning.cpp` beyond the construction inputs at
  `build_generator` — the Filler's fill-time behavior is 140's concern.
- Sub-agent return-format hints: section dispatches SUMMARY + SNIPPETS ≤ 30; constants
  FACT with file:line; cargo FACT.
