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

Port the two lightning primitives into the packet-137 `algos/lightning/` home:
`DistanceField` (unsupported-cell grid: seeding, nearest-unsupported queries, radius-consuming
updates; from `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/DistanceField.{hpp,cpp}`) and
`TreeNode` (tree graph: attachment, `propagateToNextLayer`, straightening, rerooting, pruning;
from `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/TreeNode.{hpp,cpp}`), TDD'd against
hand-computed small cases with all length constants divided by 100 per the PnP coordinate
system.

## Scope Boundaries

Pure host-side algorithm port with unit tests — no stage, IR, WIT, or module change (the 137
seam is untouched; the orchestration that calls these lands in 139). Both new files carry
the OrcaSlicer attribution header. All OrcaSlicer length constants are divided by 100 at the
boundary (port-cite per test). Public API of each type freezes at this packet's close so 139
can build on it without surprise.

## Prerequisites and Blockers

- **FORWARD-DEP on draft `137_lightning-prepass-contract`** — packet 138 depends on
  137's `crates/slicer-core/src/algos/lightning/mod.rs` skeleton
  (`generate_lightning_trees(...)` with the `// 139 wiring point` marker). 137 is
  currently `status: draft`; the forward-dep is satisfied when 137 is `status:
  implemented` (which lands `mod.rs` + `lightning_tree_producer.rs` + `LightningTreeIR`
  + the WIT read-view). If 137's plan changes the `mod.rs` skeleton signature,
  138's "exports from `mod.rs`" surface must be re-evaluated.
- Unblocks: `139_lightning-layer-generator`.
- Activation blockers: 137 must be `status: implemented` (forward-dep above).

## Acceptance Criteria

- **AC-1. Given** a `DistanceField` built from a hand-computed 4×4-cell overhang square,
  **when** queried for an unsupported point, **then** it yields a point inside the
  overhang; **when** updated with a support point inside the supporting radius, **then**
  all cells within the ported supporting radius are consumed (unsupported count decreases
  by the hand-computed cell count). | `cargo test -p slicer-core -- lightning_distance_field 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-2. Given** a `TreeNode` root with one child at distance `d` on layer `N`, **when**
  `propagate_to_next_layer` runs with per-layer move distance `m < d`, **then** the
  resulting layer-`N-1` node positions moved toward their targets by at most `m` (ported
  move-bound semantics), and parent/child attachment is preserved. | `cargo test -p slicer-core -- lightning_tree_node_propagate 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-3. Given** a 3-node dog-leg branch, **when** straightening runs with the ported
  smoothing magnitude, **then** the middle node moves toward the chord (total path length
  strictly decreases; endpoints fixed). | `cargo test -p slicer-core -- lightning_tree_node_straighten 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-4. Given** a tree with one leaf branch shorter than the ported prune length and one
  longer, **when** pruning runs, **then** the short branch is removed and the long one
  survives. | `cargo test -p slicer-core -- lightning_tree_node_prune 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-5. Given** the two new files, **when** grepped for the attribution header, **then**
  both files contain a comment block matching the `docs/ORCASLICER_ATTRIBUTION.md` header
  pattern (verified by `rg` on the attribution anchor string). | `rg -l 'OrcaSlicer' crates/slicer-core/src/algos/lightning/distance_field.rs crates/slicer-core/src/algos/lightning/tree_node.rs | wc -l | grep -q '^2$' && echo ATTR-OK`

## Negative Test Cases

- **AC-N1. Given** empty outlines / an empty tree, **when** any primitive operation runs
  (query, update, propagate, straighten, prune), **then** it returns empty results without
  panicking. | `cargo test -p slicer-core -- lightning_empty_inputs_no_panic 2>&1 | tee target/test-output.log | grep "^test result"`

## Verification

- `cargo test -p slicer-core -- lightning 2>&1 | tee target/test-output.log | grep "^test result"`
- `cargo clippy -p slicer-core --all-targets -- -D warnings`
- `cargo xtask build-guests --check` (the new files do not feed guests, but the
  freshness gate is the workspace habit)

## Authoritative Docs

- `docs/specs/lightning-infill-parity.md` §Phase L2 — full read (short).
- `docs/adr/0029-lightning-prepass-tree-generator.md` — delegate SUMMARY (already
  internalized in 137).
- `docs/ORCASLICER_ATTRIBUTION.md` — header template.
- `docs/08_coordinate_system.md` — delegate SUMMARY (÷100 rule for every distance
  constant).

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/DistanceField.hpp` (209 lines) / `.cpp` (174 lines) — cell grid representation, seeding from outlines + overhang, `update` radius consumption, nearest-unsupported query.
- `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/TreeNode.hpp` (310 lines) / `.cpp` (440 lines) — node graph (NodeSPtr ownership), `propagateToNextLayer`, straightening magnitude, rerooting, prune length semantics (section-by-section; the 750-line total is the largest single read in this packet — ≥ 5 sectioned dispatches).

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
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
