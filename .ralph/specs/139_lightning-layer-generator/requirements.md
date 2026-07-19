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
- Tests per AC-1…AC-4, AC-N1, AC-N2 (synthetic multi-layer fixtures constructed
  programmatically; no new resource files).

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
  IR content); AC-4 extends 138's determinism to the whole pipeline.
- Negative cases: `AC-N1` (no overhang → no trees), `AC-N2` (wedge byte-identity).
- Cross-packet impact: 140 samples exactly what this packet commits; the per-layer
  segment ordering frozen here is 140's input contract.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-core -- lightning_generator 2>&1 \| tee target/test-output.log \| grep "^test result"` | AC-1/2/4/N1 | FACT + counts |
| `cargo test -p slicer-runtime --test executor -- lightning_producer_commits_trees 2>&1 \| tee target/test-output.log \| grep "^test result"` | AC-3 wiring | FACT |
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
