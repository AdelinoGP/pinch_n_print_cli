# Implementation Plan: 138_lightning-distancefield-treenode

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first, then implementation, then the narrowest falsifying validation.
- Each step honors the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. The fields below are not optional metadata — they are the budget contract for this step.

## Steps

### Step 1: `DistanceField` port (RED→GREEN)

- Task IDs:
  - `TASK-263`
- Objective: author the AC-1 + AC-N1(distance-field half) tests from hand-computed 4×4-cell
  cases (RED), then port `DistanceField` (attribution header; constants ÷ 100, cited) to
  green; determinism sub-test included.
- Precondition: packet 137 closed; clean tree.
- Postcondition: `lightning_distance_field` tests green; `mod.rs` exports the type.
- Files allowed to read: `crates/slicer-core/src/algos/mod.rs`; one `algo_*_tdd.rs`
  (convention).
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/algos/lightning/distance_field.rs` (new)
  - `crates/slicer-core/src/algos/lightning/mod.rs` (export)
  - the test home file
- Files explicitly out-of-bounds for this step: `OrcaSlicerDocumented/**` directly;
  `tree_node.rs` (Step 2).
- Expected sub-agent dispatches:
  - the DistanceField SUMMARY + SNIPPETS dispatches (design §Expected Sub-Agent Dispatches)
  - the constants FACT (radius)
  - "Run `cargo test -p slicer-core -- lightning_distance_field …`; FACT + counts"
- Context cost: `M`
- Authoritative docs: `docs/08_coordinate_system.md` (delegate).
- OrcaSlicer refs: DistanceField.{hpp,cpp} — delegate.
- Verification:
  - AC-1 pipe command — FACT
- Exit condition: AC-1 green; header present.

### Step 2: `TreeNode` port (RED→GREEN)

- Task IDs:
  - `TASK-263`
- Objective: resolve the ownership `[FWD]` (back-edge FACT → Rc vs arena), author AC-2/3/4 +
  AC-N1(tree half) tests (RED), port `TreeNode` section-by-section (attachment →
  propagate → straighten → reroot → prune) to green.
- Precondition: Step 1 exit condition.
- Postcondition: `lightning_tree_node_*` tests green; public API for 139 frozen.
- Files allowed to read: own lightning module.
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/algos/lightning/tree_node.rs` (new)
  - `crates/slicer-core/src/algos/lightning/mod.rs` (export)
  - the test home file
- Files explicitly out-of-bounds for this step: `OrcaSlicerDocumented/**` directly.
- Expected sub-agent dispatches:
  - the ownership FACT + the five TreeNode.cpp section dispatches
  - the constants FACT (smoothing magnitude, prune length)
  - "Run `cargo test -p slicer-core -- lightning_tree_node …`; FACT + counts; SNIPPETS ≤20
    on failure"
- Context cost: `M`
- Authoritative docs: none new.
- OrcaSlicer refs: TreeNode.{hpp,cpp} — delegate, sectioned.
- Verification:
  - AC-2, AC-3, AC-4 pipe commands — FACT each
- Exit condition: all TreeNode ACs green. If the port exceeds M mid-flight, STOP and split
  (propagate/straighten landed; reroot/prune become a successor packet) — never rate L and
  continue.

### Step 3: Totality + attribution + gates

- Task IDs:
  - `TASK-263`
- Objective: complete AC-N1 (empty-input totality across both primitives), AC-5 attribution
  grep, determinism test for the pair, and the packet gates.
- Precondition: Step 2 exit condition.
- Postcondition: full `lightning` suite green; gates green.
- Files allowed to read: own lightning module + tests.
- Files allowed to edit (≤ 3): the test home file.
- Files explicitly out-of-bounds for this step: everything else.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-core -- lightning …`; FACT + counts"
  - "Run `cargo clippy -p slicer-core --all-targets -- -D warnings` + `cargo xtask
    build-guests --check`; FACT each"
- Context cost: `S`
- Authoritative docs: none new.
- OrcaSlicer refs: none.
- Verification:
  - AC-N1 + AC-5 pipe commands — FACT each
- Exit condition: all packet ACs green.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | DistanceField (444 C++ lines, sectioned) |
| Step 2 | M | TreeNode (1,100 C++ lines, sectioned; split tripwire armed) |
| Step 3 | S | totality + gates |

## Packet Completion Gate

- All steps complete.
- Every step exit condition is met.
- Packet acceptance criteria green (each verification command dispatched and returned PASS).
- `docs/07_implementation_status.md` updated for TASK-263 (via worker dispatch — never edited
  by loading the full backlog into the implementer's context).
- Reopened or superseded packet status transitions reconciled (none expected).
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md`.
- Confirm packet-level verification commands are green.
- Record any remaining packet-local risk explicitly before moving to `status: implemented`.
- Confirm the implementer's peak context usage stayed under 70%; if not, log it as a
  packet-authoring lesson for future spec-packet-generator runs.
