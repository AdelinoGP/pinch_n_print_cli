# Implementation Plan: 138_lightning-distancefield-treenode

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".
- All `cargo check`, `cargo clippy`, and `cargo test` invocations must use `--all-targets`
  where applicable so the test, bench, and example targets compile.

## Steps

### Step 1: `DistanceField` port (RED→GREEN)

- Task IDs: `TASK-263`
- Objective: use the separate `algo_lightning_tdd.rs` integration-test home; author AC-1 +
  AC-N1(distance-field half) tests from hand-computed
  4×4-cell cases (RED); port `DistanceField` (attribution header; constants ÷ 100, cited)
  to GREEN; determinism sub-test included.
- Precondition: packet 137 closed; clean tree; the `algos/lightning/mod.rs` skeleton
  exists (with the 139 wiring point comment).
- Postcondition: `lightning_distance_field` tests green; `mod.rs` exports the type.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/src/algos/mod.rs` — full (small file).
  - `crates/slicer-core/src/algos/mesh_analysis.rs` — test-home convention check
    (ranged; FACT target).
  - `crates/slicer-ir/src/slice_ir.rs` — `Point2` + `mm_to_units` accessors (ranged).
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/algos/lightning/distance_field.rs` (new)
  - `crates/slicer-core/src/algos/lightning/mod.rs` (one `pub use` export line)
  - the test home file (decided by FACT; either the co-located `#[cfg(test)]` block in
    `distance_field.rs` itself or a new `crates/slicer-core/tests/algo_lightning_tdd.rs`)
- Blast-radius discipline: none — both files are net-new; no struct-literal sites exist
  today.
- Files explicitly out-of-bounds for this step: `OrcaSlicerDocumented/**` directly;
  `tree_node.rs` (Step 2).
- Expected sub-agent dispatches:
  - "FACT: which test-home convention does `crates/slicer-core/src/algos/mesh_analysis.rs`
    use (co-located `#[cfg(test)]` or a separate `tests/algo_*_tdd.rs`)?; LOCATIONS ≤ 5"
    — Step 1 driver.
  - the DistanceField SUMMARY + SNIPPETS dispatches (design §Expected Sub-Agent
    Dispatches)
  - the constants FACT (supporting radius, value + units + Orca file:line)
   - "Run `cargo test -p slicer-core --features host-algos --all-targets -- lightning_distance_field …`; FACT + counts"
- Context cost: `M`
- Authoritative docs: `docs/08_coordinate_system.md` (delegate).
- OrcaSlicer refs: `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/DistanceField.{hpp,cpp}`
  — delegate.
- Verification:
  - AC-1 pipe command — FACT
- Exit condition: AC-1 green; AC-N1 distance-field half green; header present.

### Step 2: `TreeNode` port (RED→GREEN)

- Task IDs: `TASK-263`
- Objective: use `Rc<RefCell<Node>>` ownership with no back-edges; author AC-2/3/4 + AC-N1
  (tree half) tests (RED); port `TreeNode` section-by-section
  (attachment → propagate → straighten → reroot → prune) to GREEN; constants FACT first
  (smoothing magnitude, prune length, propagate move bound).
- Precondition: Step 1 exit condition.
- Postcondition: `lightning_tree_node_*` tests green; public API for 139 frozen.
- Files allowed to read: own lightning module + `Point2` accessor.
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/algos/lightning/tree_node.rs` (new)
  - `crates/slicer-core/src/algos/lightning/mod.rs` (one `pub use` export line)
  - the test home file (decisions in Step 1 lock the location; add the tree-node tests
    beside the distance-field tests)
- Blast-radius discipline: none beyond the test home — the new `tree_node.rs` is
  net-new.
- Files explicitly out-of-bounds for this step: `OrcaSlicerDocumented/**` directly.
- Expected sub-agent dispatches:
  - the ownership FACT (back-edge presence/absence) + the five `TreeNode.cpp` section
    dispatches
  - the constants FACT (smoothing magnitude, prune length, move bound)
   - "Run `cargo test -p slicer-core --features host-algos --all-targets -- lightning_tree_node …`; FACT + counts;
    SNIPPETS ≤ 20 on failure"
- Context cost: `M`
- Authoritative docs: none new.
- OrcaSlicer refs: `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/TreeNode.{hpp,cpp}`
  — delegate, sectioned (≥ 5 dispatches covering attachment, propagate, straighten,
  reroot, prune).
- Verification:
  - AC-2, AC-3, AC-4 pipe commands — FACT each
- Exit condition: all TreeNode ACs green. **Split tripwire:** if the port exceeds M
  mid-flight (i.e. Step 2's section count balloons or RC/arena choice forces a
  redesign), STOP and split — (propagate + straighten land; reroot + prune become a
  successor packet) — never rate L and continue. Record the split in this packet's
  `requirements.md` §Step Completion Expectations.

### Step 3: Totality + attribution + gates

- Task IDs: `TASK-263`
- Objective: complete AC-N1 (empty-input totality across both primitives) if not already
  covered by Steps 1/2, AC-5 attribution grep, determinism test for the pair (two
  identical runs → identical results, no hash containers), and the packet gates.
- Precondition: Step 2 exit condition.
- Postcondition: full `lightning` suite green; gates green; public API frozen.
- Files allowed to read: own lightning module + tests.
- Files allowed to edit (at most 3): the test home file.
- Files explicitly out-of-bounds for this step: everything else.
- Expected sub-agent dispatches:
   - "Run `cargo test -p slicer-core --features host-algos --all-targets -- lightning …`; FACT + counts"
   - "Run `cargo clippy -p slicer-core --all-targets --features host-algos -- -D warnings` + `cargo xtask
    build-guests --check`; FACT each"
- Context cost: `S`
- Authoritative docs: none new.
- OrcaSlicer refs: none.
- Verification:
  - AC-N1 + AC-5 pipe commands — FACT each
   - the `cargo test -p slicer-core --features host-algos --all-targets -- lightning` pipe — FACT
- Exit condition: all packet ACs green; API frozen.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | DistanceField (383 C++ lines total, sectioned) |
| Step 2 | M | TreeNode (750 C++ lines total, sectioned; split tripwire armed) |
| Step 3 | S | totality + gates + API freeze |

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch (TASK-263 flip),
  never a full backlog read.
- Reconcile reopened/superseded status transitions (none expected).
- Public API of `DistanceField` and `tree_node` graph operations is frozen; any 139
  signature change is a recorded deviation in 139 (per the cross-step invariant).
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged
  swarm ESCALATION; otherwise record a packet-authoring lesson.
- Record any remaining packet-local risk explicitly before moving to `status: implemented`.
