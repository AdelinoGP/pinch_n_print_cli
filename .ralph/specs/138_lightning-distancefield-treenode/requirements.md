# Requirements: 138_lightning-distancefield-treenode

## Packet Metadata

- Grouped task IDs:
  - `TASK-263`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Full lightning parity (roadmap decision, 2026-07-01 grilling) requires the canonical
generator, and the generator is built from two primitives the workspace lacks: the
unsupported-cell `DistanceField` that decides WHERE trees must grow, and the `TreeNode` graph
that decides HOW they grow, straighten, reroot, and prune across layers. These are the
subtlest 1,544 lines of the 3,317-LOC port — landing them alone, with hand-computed unit
cases, keeps packet 139's orchestration port mechanical instead of monolithic.

## In Scope

- `crates/slicer-core/src/algos/lightning/distance_field.rs` — port of
  `Fill/Lightning/DistanceField.{hpp,cpp}`: cell grid from outlines + overhang, unsupported
  queries, radius-consuming `update`.
- `crates/slicer-core/src/algos/lightning/tree_node.rs` — port of
  `Fill/Lightning/TreeNode.{hpp,cpp}`: node graph + attachment, `propagate_to_next_layer`,
  straightening, rerooting, pruning.
- Unit tests per AC-1…AC-4, AC-N1 (hand-computed small cases) + a determinism test (two
  identical runs → identical results — no hash-container iteration).
- OrcaSlicer attribution headers on both files; all length constants ÷ 100.

## Out of Scope

- `Lightning::Layer` and `Generator` (139); any producer wiring (the 137 skeleton stays
  empty).
- WIT/IR/module changes.
- Performance tuning beyond the port's own structure (grid resolution constants are ported,
  not re-derived).

## Authoritative Docs

- `docs/specs/lightning-infill-parity.md` §L2 — full (short).
- `docs/ORCASLICER_ATTRIBUTION.md` — header.
- `docs/08_coordinate_system.md` — delegate SUMMARY.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/DistanceField.hpp` / `.cpp` — full primitive semantics (sectioned dispatches).
- `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/TreeNode.hpp` / `.cpp` — full primitive semantics (sectioned dispatches; the 629-line cpp needs ~5 section dispatches).

## Acceptance Summary

- Positive cases: `AC-1`–`AC-5` in `packet.spec.md`. Refinements: each behavioral AC uses a
  hand-computed case small enough to verify on paper (4×4 cells; 3-node branches) — ported
  constants (radius, smoothing magnitude, prune length) enter the tests as the ported
  constants, cited by Orca file:line in test comments.
- Negative cases: `AC-N1` (empty-input totality).
- Cross-packet impact: 139 consumes these APIs — public signatures freeze at this packet's
  close (changes afterward are recorded deviations in 139).

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-core -- lightning 2>&1 \| tee target/test-output.log \| grep "^test result"` | full primitive suite | FACT + counts |
| `cargo clippy -p slicer-core --all-targets -- -D warnings` | lint gate | FACT |
| `cargo xtask build-guests --check` (rebuild if STALE) | slicer-core feeds guests | FACT |
| the AC-5 attribution rg | headers present | FACT |

## Step Completion Expectations

None. (Two independent primitives, one step each, then a shared gate step.)

## Context Discipline Notes

- Large files in the read-only path that MUST be ranged or delegated: ALL OrcaSlicer reads
  (TreeNode.cpp is 629 lines — five sectioned dispatches minimum; never a whole-file
  SUMMARY).
- Likely temptation reads: CuraEngine lightning documentation/lore — out of scope; the
  OrcaSlicer sources are the only canon.
- Sub-agent return-format hints: section dispatches return SUMMARY (semantics) + SNIPPETS
  ≤30 lines (the exact loop being ported); constants return FACT with file:line.
