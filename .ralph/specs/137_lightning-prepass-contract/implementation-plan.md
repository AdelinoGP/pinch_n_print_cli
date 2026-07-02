# Implementation Plan: 137_lightning-prepass-contract

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first, then implementation, then the narrowest falsifying validation.
- Each step honors the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. The fields below are not optional metadata — they are the budget contract for this step.

## Steps

### Step 1: `LightningTreeIR` + stage registration

- Task IDs:
  - `TASK-262`
- Objective: add `LightningTreeIR` (+ version constant, blackboard slot/commit accessor,
  `slicer-ir` shape test) and the `PrePass::LightningTreeGen` stage entry (+ stage-order test
  update); resolve the `[FWD]` max-ir-schema question by FACT dispatch.
- Precondition: packet 136 closed; clean tree.
- Postcondition: AC-1 and AC-2 verification commands green; workspace compiles.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-ir/src/slice_ir.rs` — `SupportPlanIR` region (~1046) + version constants
  - `crates/slicer-scheduler/src/execution_plan.rs` — lines 19-41
- Files allowed to edit (≤ 3):
  - `crates/slicer-ir/src/slice_ir.rs`
  - `crates/slicer-scheduler/src/execution_plan.rs` (+ its stage-order test)
  - `crates/slicer-ir/tests/` (shape test file)
- Files explicitly out-of-bounds for this step: WIT files (Step 3), producer (Step 2).
- Expected sub-agent dispatches:
  - "FACT: does adding a NEW IR type require bumping a global max_ir_schema constant (packet
    91 precedent)? ≤5 lines"
  - "Run `cargo test -p slicer-ir -- lightning_tree_ir …` + `cargo test -p slicer-scheduler
    -- stage_order …`; FACT each"
- Context cost: `M`
- Authoritative docs: ADR-0029; `docs/02_ir_schemas.md` versioning rules (delegate).
- OrcaSlicer refs: none.
- Verification:
  - AC-1, AC-2 pipe commands — FACT each
- Exit condition: IR + stage in; tests green; `[FWD]` recorded resolved.

### Step 2: Producer skeleton + skip predicate

- Task IDs:
  - `TASK-262`
- Objective: `crates/slicer-core/src/algos/lightning/mod.rs` skeleton
  (`generate_lightning_trees` returning a valid empty IR; the 139 wiring point marked) +
  runtime builtin wrapper registered at the stage; skip when no region's
  `sparse_fill_holder` resolves to `lightning-infill` (NO commit on skip); executor
  skip/commit test (AC-3).
- Precondition: Step 1 exit condition.
- Postcondition: AC-3 green.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-core/src/algos/support_geometry.rs` — lines 80-140 (pattern)
  - the support-producer's runtime wrapper (LOCATIONS dispatch first, then ranged)
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/algos/lightning/mod.rs` (new)
  - the runtime builtin wrapper file
  - `crates/slicer-runtime/tests/executor/lightning_prepass_tdd.rs` (new) + harness mod line
- Files explicitly out-of-bounds for this step: WIT/SDK (Step 3).
- Expected sub-agent dispatches:
  - "LOCATIONS ≤10: where the support-geometry producer is registered + committed"
  - "Run `cargo test -p slicer-runtime --test executor -- lightning_prepass …`; FACT"
- Context cost: `M`
- Authoritative docs: ADR-0029 (skip promise).
- OrcaSlicer refs: none.
- Verification:
  - AC-3 pipe command — FACT
- Exit condition: skip/commit behavior pinned green.

### Step 3: WIT read-view + SDK accessor + roundtrip test

- Task IDs:
  - `TASK-262`
- Objective: add the lightning-tree read-view to `ir-types.wit` (+ world bump), SDK accessor,
  macros glue; extend/author the test guest to echo tree segments; contract roundtrip test
  (AC-4) + wit-drift rows (AC-N2); guest rebuild.
- Precondition: Step 2 exit condition.
- Postcondition: AC-4, AC-N2 green; guests fresh.
- Files allowed to read (with line-range hints when > 300 lines):
  - the view-pattern LOCATIONS results (ranged); `crates/slicer-sdk/src/views.rs` (target
    region)
- Files allowed to edit (≤ 3 per wave):
  - Wave A: `crates/slicer-schema/wit/deps/ir-types.wit` (+ world file)
  - Wave B: `crates/slicer-sdk/src/views.rs`, `crates/slicer-macros/src/lib.rs`
  - Wave C: test guest + `crates/slicer-runtime/tests/contract/` roundtrip + drift files
- Files explicitly out-of-bounds for this step: `modules/core-modules/**`.
- Expected sub-agent dispatches:
  - "Run `cargo build --tests 2>&1 | tail -40`; FACT or LOCATIONS ≤30" — after WIT edit
  - "Run `cargo xtask build-guests --check`; FACT; rebuild if STALE"
  - "Run `cargo test -p slicer-runtime --test contract -- 'lightning_tree_view_roundtrip|wit_drift' …`; FACT"
- Context cost: `M`
- Authoritative docs: CLAUDE.md §WIT/Type Changes Checklist.
- OrcaSlicer refs: none.
- Verification:
  - AC-4, AC-N2 pipe commands — FACT each
- Exit condition: roundtrip + drift green.

### Step 4: Byte-identity guard + Doc Impact + gates

- Task IDs:
  - `TASK-262`
- Objective: wedge SHA guard (AC-N1); docs/02 + docs/03 sections; packet gates.
- Precondition: Step 3 exit condition.
- Postcondition: all ACs green; Doc Impact greps hit.
- Files allowed to read: the two docs (rg-located sections).
- Files allowed to edit (≤ 3):
  - `docs/02_ir_schemas.md`, `docs/03_wit_and_manifest.md`
- Files explicitly out-of-bounds for this step: code.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-runtime --test e2e -- wedge …`; FACT"
  - "Run `cargo clippy --workspace --all-targets -- -D warnings` + the two Doc Impact greps;
    FACT each"
- Context cost: `S`
- Authoritative docs: the two being edited.
- OrcaSlicer refs: none.
- Verification:
  - `rg -q 'LightningTreeIR' docs/02_ir_schemas.md && echo HIT` — FACT
- Exit condition: greps hit; gates green.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | IR + stage |
| Step 2 | M | producer skeleton + skip |
| Step 3 | M | WIT view + roundtrip |
| Step 4 | S | guard + docs |

## Packet Completion Gate

- All steps complete.
- Every step exit condition is met.
- Packet acceptance criteria green (each verification command dispatched and returned PASS).
- `docs/07_implementation_status.md` updated for TASK-262 (via worker dispatch — never edited
  by loading the full backlog into the implementer's context).
- Reopened or superseded packet status transitions reconciled (none expected).
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md`.
- Confirm packet-level verification commands are green.
- Record any remaining packet-local risk explicitly before moving to `status: implemented`.
- Confirm the implementer's peak context usage stayed under 70%; if not, log it as a
  packet-authoring lesson for future spec-packet-generator runs.
