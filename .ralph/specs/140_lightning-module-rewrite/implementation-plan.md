# Implementation Plan: 140_lightning-module-rewrite

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first, then implementation, then the narrowest falsifying validation.
- Each step honors the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. The fields below are not optional metadata — they are the budget contract for this step.

## Steps

### Step 1: Orca sampling-side FACT + RED suite

- Task IDs:
  - `TASK-265`
- Objective: settle the `[FWD]` (delegated `Filler::_fill_surface_single` SUMMARY); classify
  the existing 323-line test file (keep / adapt / delete, each deletion naming the stub
  behavior it encoded); author the new RED tests (`samples_tree_ir_raw_emit`,
  `empty_trees_emit_nothing`).
- Precondition: packets 137–139 closed.
- Postcondition: `[FWD]` resolved and recorded; test classification recorded in the test-file
  header; new tests RED.
- Files allowed to read (with line-range hints when > 300 lines):
  - `modules/core-modules/lightning-infill/src/lib.rs` — full (one read; it gets replaced)
  - `modules/core-modules/lightning-infill/tests/lightning_infill_tdd.rs` — full (one read)
- Files allowed to edit (≤ 3):
  - `modules/core-modules/lightning-infill/tests/lightning_infill_tdd.rs`
- Files explicitly out-of-bounds for this step: production `lib.rs` (RED first);
  `OrcaSlicerDocumented/**` directly.
- Expected sub-agent dispatches:
  - the Filler SUMMARY dispatch (design §Expected Sub-Agent Dispatches)
  - "Run `cargo test -p lightning-infill … | grep -E '^test |^test result'`; FACT per-test" —
    RED confirmation
- Context cost: `M`
- Authoritative docs: `docs/specs/lightning-infill-parity.md` §L4.
- OrcaSlicer refs: FillLightning.cpp — one delegated SUMMARY.
- Verification:
  - RED state FACT
- Exit condition: `[FWD]` resolved; new tests RED; classification recorded.

### Step 2: GREEN — the sampler rewrite

- Task IDs:
  - `TASK-265`
- Objective: replace the stub body with the sampler (view → raw SparseInfill emission,
  mm conversion at the boundary, origin discipline preserved); delete `build_branches` +
  grid machinery; adapt kept tests; AC-1, AC-2, AC-N2 green.
- Precondition: Step 1 exit condition.
- Postcondition: module suite green; structural greps clean.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-sdk/src/views.rs` — the lightning view accessor region only
- Files allowed to edit (≤ 3):
  - `modules/core-modules/lightning-infill/src/lib.rs`
  - `modules/core-modules/lightning-infill/tests/lightning_infill_tdd.rs`
- Files explicitly out-of-bounds for this step: `crates/slicer-core/src/algos/lightning/**`
  (triage fence).
- Expected sub-agent dispatches:
  - "Run `cargo test -p lightning-infill …`; FACT + counts; SNIPPETS ≤20 on failure"
  - "Run `cargo xtask build-guests --check`; FACT; rebuild if STALE"
- Context cost: `M`
- Authoritative docs: ADR-0029 sampler contract (delegate).
- OrcaSlicer refs: none.
- Verification:
  - AC-1, AC-2, AC-N2 pipe commands — FACT each
- Exit condition: module green; stub grep-gone.

### Step 3: Pipeline uniformity + byte-identity guard

- Task IDs:
  - `TASK-265`
- Objective: add `lightning_pipeline_linked` (AC-3: lightning-configured slice → linker →
  linked multi-point sparse polylines) and run the wedge guard (AC-N1).
- Precondition: Step 2 exit condition.
- Postcondition: AC-3, AC-N1 green.
- Files allowed to read: one neighboring executor test (idiom).
- Files allowed to edit (≤ 3):
  - `crates/slicer-runtime/tests/executor/lightning_pipeline_linked_tdd.rs` (new) + harness
    mod line
- Files explicitly out-of-bounds for this step: module + linker sources (triage fence:
  failures are diagnosed to emission vs linking and routed, not patched here beyond the
  ≤ 20-line deviation allowance).
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-runtime --test executor -- lightning_pipeline_linked …`;
    FACT"
  - "Run `cargo test -p slicer-runtime --test e2e -- wedge …`; FACT"
- Context cost: `S`
- Authoritative docs: none new.
- OrcaSlicer refs: none.
- Verification:
  - AC-3, AC-N1 pipe commands — FACT each
- Exit condition: uniformity + identity green.

### Step 4: Closure — DEV-081, contained bless, roadmap ceremony

- Task IDs:
  - `TASK-265`
- Objective: flip DEV-081 to Closed (packet 140); re-bless lightning-affected expectations
  (two consecutive identical runs; per-expectation justification); docs/07 closure sweep for
  TASK-262…265; run the roadmap-close `cargo xtask test --workspace --summary` ceremony.
- Precondition: Step 3 exit condition (bless only after geometry/pipeline green).
- Postcondition: AC-4, AC-5 green; ceremony PASS recorded; packet + roadmap closed.
- Files allowed to read: none directly (all delegated).
- Files allowed to edit (≤ 3):
  - `docs/DEVIATION_LOG.md` (DEV-081 row)
  - lightning-affected expectation files (bless waves)
  - `docs/07_implementation_status.md` (via dispatch)
- Files explicitly out-of-bounds for this step: everything else.
- Expected sub-agent dispatches:
  - "Bless sweep: per expectation, FACT old→new + justification"
  - "Run `cargo xtask build-guests --check` then `cargo xtask test --workspace --summary`;
    verdict block ONLY"
  - "Doc edits + the two Doc Impact greps; FACT each"
- Context cost: `S` (all delegated)
- Authoritative docs: `CLAUDE.md` §Test Discipline.
- OrcaSlicer refs: none.
- Verification:
  - AC-4 + AC-5 pipe commands + the ceremony verdict — FACT each
- Exit condition: DEV-081 Closed; ceremony PASS; TASK-262…265 closed.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | FACT + classification + RED |
| Step 2 | M | the rewrite |
| Step 3 | S | pipeline + guard |
| Step 4 | S | closure (delegated) |

## Packet Completion Gate

- All steps complete.
- Every step exit condition is met.
- Packet acceptance criteria green (each verification command dispatched and returned PASS).
- `docs/07_implementation_status.md` updated for TASK-262…TASK-265 (via worker dispatch —
  never edited by loading the full backlog into the implementer's context).
- Reopened or superseded packet status transitions reconciled (DEV-081 closure recorded).
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md`.
- Confirm packet-level verification commands are green (including the workspace `--summary`
  verdict).
- Record any remaining packet-local risk explicitly before moving to `status: implemented`.
- Confirm the implementer's peak context usage stayed under 70%; if not, log it as a
  packet-authoring lesson for future spec-packet-generator runs.
