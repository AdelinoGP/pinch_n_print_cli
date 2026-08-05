# Requirements: 138_lightning-distancefield-treenode

## Packet Metadata

- Grouped task IDs: `TASK-263`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Full lightning parity (roadmap decision, 2026-07-01 grilling) requires the canonical
generator, and the generator is built from two primitives the workspace lacks: the
unsupported-cell `DistanceField` that decides **where** trees must grow, and the `TreeNode`
graph that decides **how** they grow, straighten, reroot, and prune across layers. These
are the subtlest 750 lines of the 2,175-LOC OrcaSlicer port (the plan's "3,317 LOC" figure
overstates the source files; verified: `DistanceField.{hpp,cpp}` = 383, `TreeNode.{hpp,cpp}`
= 750, `Layer.{hpp,cpp}` = 540, `Generator.{hpp,cpp}` = 423, `FillLightning.{hpp,cpp}` =
79). Landing them alone, with hand-computed unit cases, keeps packet 139's orchestration
port mechanical instead of monolithic, and the 750-line TreeNode is split across
attachment/propagate/straighten/reroot/prune sections so no single dispatch is ever a
whole-file dump.

## In Scope

- `crates/slicer-core/src/algos/lightning/distance_field.rs` (new) — port of
  `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/DistanceField.{hpp,cpp}`: cell grid
  from outlines + overhang, unsupported queries, radius-consuming `update`.
- `crates/slicer-core/src/algos/lightning/tree_node.rs` (new) — port of
  `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/TreeNode.{hpp,cpp}`: node graph +
  attachment, `propagate_to_next_layer`, straightening, rerooting, pruning.
- Exports from the packet-137 `crates/slicer-core/src/algos/lightning/mod.rs` (skeleton
  unchanged otherwise).
- Unit tests per AC-1…AC-4, AC-N1 (hand-computed small cases) + a determinism test (two
  identical runs → identical results — no hash-container iteration).
- OrcaSlicer attribution headers on both files; all length constants ÷ 100, cited by
  Orca file:line in test comments.

## Out of Scope

- `Lightning::Layer` and `Generator` (139); any producer wiring (the 137 skeleton stays
  empty; the `// 139 wiring point` comment stays in `mod.rs`).
- WIT/IR/module changes.
- Performance tuning beyond the port's own structure (grid resolution constants are
  ported, not re-derived).
- Density-derived resolution; 138 takes `supporting_radius` as the `DistanceField` constructor
  parameter and derives `m_cell_size = supporting_radius / 6` internally. 139 supplies the
  resolved supporting radius.

## Authoritative Docs

- `docs/specs/lightning-infill-parity.md` §L2 — full (short).
- `docs/ORCASLICER_ATTRIBUTION.md` — header.
- `docs/08_coordinate_system.md` — delegate SUMMARY.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/DistanceField.hpp` / `.cpp` — full primitive semantics (sectioned dispatches: cell representation, seeding loop, `update`).
- `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/TreeNode.hpp` / `.cpp` — full primitive semantics (sectioned dispatches: the 750-line total needs ≥ 5 sections — ownership, attachment, propagate, straighten, reroot, prune).

## Acceptance Summary

- Positive cases: `AC-1`–`AC-5` in `packet.spec.md`. Refinements: each behavioral AC uses a
  hand-computed case small enough to verify on paper (4×4 cells; 3-node branches) —
  ported constants (supporting radius, smoothing magnitude, prune length) enter the tests
  as the ported constants, cited by Orca file:line in test comments so the chain is
  auditable.
- Negative cases: `AC-N1` (empty-input totality across both primitives).
- Cross-packet impact: 139 consumes these APIs — public signatures freeze at this
  packet's close (changes afterward are recorded deviations in 139 with the 138 tests
  co-updated in the same step, never left red between steps).

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-core --features host-algos --all-targets -- lightning 2>&1 \| tee target/test-output.log \| grep "^test result: ok"` | full primitive suite | FACT + counts |
| `cargo clippy -p slicer-core --all-targets --features host-algos -- -D warnings` | lint gate | FACT |
| `cargo xtask build-guests --check` (rebuild if STALE) | workspace habit | FACT |
| the AC-5 attribution rg | headers present | FACT |
| the AC-N1 empty-input rg | no-panic totality | FACT |

## Step Completion Expectations

- Cross-step invariant: each step's "Files allowed to edit" includes the test home; a
  primitive's tests land in the SAME step as the primitive (no orphaned RED between
  steps).
- The 138 public APIs freeze at the end of Step 3. If Step 3's tests reveal that 139 will
  need a different shape, the API is amended in Step 3 with a recorded deviation (never
  punted to 139).

## Context Discipline Notes

- Large files in the read-only path that MUST be ranged or delegated: ALL OrcaSlicer
  reads. TreeNode.{hpp,cpp} totals 750 lines — five sectioned dispatches minimum; never
  a whole-file SUMMARY.
- Likely temptation reads: CuraEngine lightning documentation/lore — out of scope; the
  OrcaSlicer sources are the only canon.
- Sub-agent return-format hints: section dispatches return SUMMARY (semantics) + SNIPPETS
  ≤ 30 lines (the exact loop being ported); constants return FACT with file:line; cargo
  gates return FACT.
