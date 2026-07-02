---
status: draft
packet: 138_lightning-distancefield-treenode
task_ids:
  - TASK-263
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 138_lightning-distancefield-treenode

## Goal

Port the two lightning primitives into `crates/slicer-core/src/algos/lightning/`:
`DistanceField` (unsupported-cell grid: seeding, nearest-unsupported queries, radius-consuming
updates; from `Fill/Lightning/DistanceField.{hpp,cpp}`) and `TreeNode` (tree graph:
attachment, `propagateToNextLayer`, straightening, rerooting, pruning; from
`Fill/Lightning/TreeNode.{hpp,cpp}`), TDD'd against hand-computed small cases.

## Scope Boundaries

Pure host-side algorithm port with unit tests — no stage, IR, WIT, or module change (the 137
seam is untouched; the orchestration that calls these lands in 139). Both files carry the
OrcaSlicer attribution header. All OrcaSlicer length constants are divided by 100.

## Prerequisites and Blockers

- Depends on: `137_lightning-prepass-contract` (the `algos/lightning/` home exists).
- Unblocks: `139_lightning-layer-generator`.
- Activation blockers: none.

## Acceptance Criteria

- **AC-1. Given** a `DistanceField` built from a small hand-computed outline/overhang pair
  (one 4×4-cell overhang square), **when** queried, **then** it yields an unsupported point
  inside the overhang; **when** updated with a support point, **then** all cells within the
  supporting radius are consumed (unsupported count decreases by the hand-computed cell
  count). | `cargo test -p slicer-core -- lightning_distance_field 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-2. Given** a `TreeNode` root with one child at distance d on layer N, **when**
  `propagate_to_next_layer` runs with per-layer move distance m < d, **then** the resulting
  layer-N−1 node positions moved toward their targets by at most m (ported move-bound
  semantics), and parent/child attachment is preserved. | `cargo test -p slicer-core -- lightning_tree_node_propagate 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-3. Given** a 3-node dog-leg branch, **when** straightening runs with the ported
  smoothing magnitude, **then** the middle node moves toward the chord (total path length
  strictly decreases; endpoints fixed). | `cargo test -p slicer-core -- lightning_tree_node_straighten 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-4. Given** a tree with one leaf branch shorter than the ported prune length and one
  longer, **when** pruning runs, **then** the short branch is removed and the long one
  survives. | `cargo test -p slicer-core -- lightning_tree_node_prune 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-5. Given** the two new files, **when** grepped, **then** both carry the OrcaSlicer
  attribution header per `docs/ORCASLICER_ATTRIBUTION.md`. | `rg -l 'OrcaSlicer' crates/slicer-core/src/algos/lightning/distance_field.rs crates/slicer-core/src/algos/lightning/tree_node.rs | wc -l | grep -q '^2$' && echo ATTR-OK`

## Negative Test Cases

- **AC-N1. Given** empty outlines / an empty tree, **when** any primitive operation runs
  (query, update, propagate, straighten, prune), **then** it returns empty results without
  panicking. | `cargo test -p slicer-core -- lightning_empty_inputs_no_panic 2>&1 | tee target/test-output.log | grep "^test result"`

## Verification

- `cargo test -p slicer-core -- lightning 2>&1 | tee target/test-output.log | grep "^test result"`
- `cargo clippy -p slicer-core --all-targets -- -D warnings`
- `cargo xtask build-guests --check`

## Authoritative Docs

- `docs/specs/lightning-infill-parity.md` §Phase L2 — full read (short).
- `docs/adr/0029-…` — delegate SUMMARY (already internalized in 137).
- `docs/ORCASLICER_ATTRIBUTION.md` — header template.
- `docs/08_coordinate_system.md` — delegate SUMMARY (÷100 rule for every distance constant).

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/DistanceField.hpp` / `.cpp` (219/225 lines) — cell grid representation, seeding from outlines/overhang, `update` radius consumption, nearest-unsupported query.
- `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/TreeNode.hpp` / `.cpp` (471/629 lines) — node graph, `propagateToNextLayer`, straightening magnitude, rerooting, prune length semantics (dispatch section-by-section).

## Doc Impact Statement (Required)

**`none`** — internal host-side algorithm primitives with no public pipeline surface yet; the
IR/stage contract was documented by packet 137, and the algorithm's pipeline role is
documented by ADR-0029 + `docs/specs/lightning-infill-parity.md` (already landed).

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
