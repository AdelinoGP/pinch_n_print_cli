# Implementation Plan: 130_infill-postprocess-contract

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first, then implementation, then the narrowest falsifying validation.
- Each step honors the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. The fields below are not optional metadata — they are the budget contract for this step.

## Steps

### Step 1: WIT contract edit + SDK types/trait + macros glue

- Task IDs:
  - `TASK-255`
- Objective: land the six `perimeter-region-view` fields, the `run-infill-postprocess`
  `prior-infill` param, the `world-layer` 1.1.0 bump, the SDK struct/accessors/trait
  signature/builder setters, and the macros glue arm.
- Precondition: clean tree; packet 129 closed (serial order); `cargo check --workspace` green
  at baseline.
- Postcondition: `cargo build --tests` compiles `slicer-schema`, `slicer-sdk`,
  `slicer-macros`; downstream crates MAY be red (expected until Step 3).
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-schema/wit/deps/ir-types.wit` — the `perimeter-region-view` +
    `slice-region-view` resource regions (copy the partitioned-polygon field idiom from
    slice-region-view)
  - `crates/slicer-sdk/src/views.rs` — lines 19-120 (SliceRegionView field idiom) + 490-600
  - `crates/slicer-sdk/src/traits.rs` — lines 320-400
  - `crates/slicer-macros/src/lib.rs` — the existing layer-world arm only (rg
    `run_infill_postprocess`)
- Files allowed to edit (≤ 3 per wave; this step has two waves):
  - Wave A: `crates/slicer-schema/wit/deps/ir-types.wit`,
    `crates/slicer-schema/wit/deps/world-layer/world-layer.wit`
  - Wave B: `crates/slicer-sdk/src/views.rs`, `crates/slicer-sdk/src/traits.rs`,
    `crates/slicer-macros/src/lib.rs` (+ `crates/slicer-sdk/src/test_support/fixtures.rs`)
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-wasm-host/**` (Step 2), all module bodies, `docs/03` in full
- Expected sub-agent dispatches:
  - "Run `cargo build --tests 2>&1 | tail -40`; FACT pass or LOCATIONS first-error batch" —
    WIT checklist step, run immediately after Wave A and again after Wave B
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0028-…` — full read (binding); `CLAUDE.md` §WIT/Type Changes Checklist
- OrcaSlicer refs: none.
- Verification:
  - `cargo check -p slicer-schema -p slicer-sdk -p slicer-macros --all-targets` — FACT
- Exit condition: schema/sdk/macros compile with the new contract; drift red-state elsewhere
  acknowledged (not "fixed" by reverting).

### Step 2: Host population — dispatch arm + marshaling

- Task IDs:
  - `TASK-255`
- Objective: populate the six fields at the `run_infill_postprocess` dispatch arm (four
  polygons copied from the `SliceIR` region; `tool-index` via the three-level precedence;
  `wall-source-region-id` via the hoisted virtual-variant predicate) and marshal
  `prior-infill` from the committed `InfillIR`.
- Precondition: Step 1 exit condition.
- Postcondition: `cargo check -p slicer-wasm-host --all-targets` green; the dispatch arm
  builds views with real data.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-wasm-host/src/dispatch.rs` — lines 420-520 (postprocess arm + neighboring
    view-builder idiom) only
  - `crates/slicer-runtime/src/region_partition.rs` — lines 112-150 (predicate to hoist)
  - `crates/slicer-core/src/algos/region_mapping.rs` — lines 640-680 (material-tool idiom)
  - `crates/slicer-ir/src/slice_ir.rs` — lines 1660-1920 (`InfillRegion`/`InfillIR`)
- Files allowed to edit (≤ 3):
  - `crates/slicer-wasm-host/src/dispatch.rs`
  - `crates/slicer-wasm-host/src/marshal/out.rs`
  - `crates/slicer-runtime/src/region_partition.rs` (only if hoisting the predicate; else
    swap for the host file the predicate lands in)
- Files explicitly out-of-bounds for this step:
  - everything under `modules/`, `crates/slicer-gcode`, executor tests
- Expected sub-agent dispatches:
  - "Run `cargo check -p slicer-wasm-host -p slicer-runtime --all-targets 2>&1`; FACT or
    LOCATIONS ≤30"
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0028-…` §Amendment (derivation rules)
- OrcaSlicer refs: none.
- Verification:
  - `cargo check -p slicer-wasm-host --all-targets` — FACT
- Exit condition: host crates compile; population logic in place with the precedence order
  pinned in one function.

### Step 3: Workspace blast-radius sweep

- Task IDs:
  - `TASK-255`
- Objective: fix every remaining exhaustive construction/match on `PerimeterRegionView`
  across the workspace (~30 files; empty/None defaults where tests don't care), driven by
  compiler errors — not by browsing.
- Precondition: Step 2 exit condition.
- Postcondition: `cargo check --workspace --all-targets` green;
  `cargo clippy --workspace --all-targets -- -D warnings` clean.
- Files allowed to read: only files named in the delegated compiler-error LOCATIONS batches.
- Files allowed to edit: the same files (mechanical field additions only; ≤ 3 per wave,
  repeat waves until clean).
- Files explicitly out-of-bounds for this step:
  - any file the compiler did not flag
- Expected sub-agent dispatches:
  - "Run `cargo check --workspace --all-targets 2>&1`; group errors by file; LOCATIONS ≤30
    per batch" — repeat until FACT clean
  - "Run `cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -20`; FACT"
- Context cost: `M`
- Authoritative docs: none new.
- OrcaSlicer refs: none.
- Verification:
  - `cargo check --workspace --all-targets` — FACT
  - `cargo clippy --workspace --all-targets -- -D warnings` — FACT
- Exit condition: both gates green.

### Step 4: Guest rebuild + echo test-guest + contract tests (RED→GREEN)

- Task IDs:
  - `TASK-255`
- Objective: rebuild all guests; add the postprocess echo test-guest; write the new contract
  tests (`infill_postprocess_prior_ir`, `infill_postprocess_partitioned_polygons`,
  `infill_postprocess_tool_index_precedence`, `infill_postprocess_wall_source`,
  `infill_postprocess_absent_module_preserves_infill`) plus the `wit_drift_detection_tdd`
  update and the SDK builder test; drive to green.
- Precondition: Step 3 exit condition.
- Postcondition: contract suite green including the five new tests; SDK tests green; guests
  fresh.
- Files allowed to read (with line-range hints when > 300 lines):
  - one existing test-guest directory (template); `crates/slicer-runtime/tests/contract/`
    harness `main.rs` + one neighboring contract test as idiom
- Files allowed to edit (≤ 3 per wave):
  - `crates/slicer-wasm-host/test-guests/<new echo guest>/**` (new)
  - `crates/slicer-runtime/tests/contract/infill_postprocess_contract_tdd.rs` (new) +
    harness `main.rs` mod line
  - `crates/slicer-runtime/tests/contract/wit_drift_detection_tdd.rs`
- Files explicitly out-of-bounds for this step:
  - module bodies; executor/e2e buckets (touched only if AC-N2's suite run flags them)
- Expected sub-agent dispatches:
  - "Run `cargo xtask build-guests --check`; FACT clean or STALE; rebuild if stale"
  - "Run `cargo test -p slicer-runtime --test contract 2>&1 | tee target/test-output.log |
    grep '^test result'`; FACT + counts; SNIPPETS on failure"
  - "Run `cargo test -p slicer-sdk 2>&1 | tee target/test-output.log | grep '^test result'`;
    FACT"
- Context cost: `M`
- Authoritative docs: `docs/adr/0028-…` §Amendment (AC semantics).
- OrcaSlicer refs: none.
- Verification:
  - the three dispatches above — each FACT
- Exit condition: AC-1…AC-5, AC-N1, AC-N2 verification commands all green.

### Step 5: Doc Impact + gate ceremony

- Task IDs:
  - `TASK-255`
- Objective: land the `docs/03_wit_and_manifest.md` and `docs/05_module_sdk.md` sections
  (six fields, `prior-infill` param, 1.1.0, full-re-emit contract sentence); run the packet
  gates.
- Precondition: Step 4 exit condition.
- Postcondition: Doc Impact greps hit; the three `packet.spec.md` §Verification gates green.
- Files allowed to read (with line-range hints when > 300 lines):
  - `docs/03_wit_and_manifest.md` — rg-located sections only
  - `docs/05_module_sdk.md` — the `run_infill_postprocess` section only
- Files allowed to edit (≤ 3):
  - `docs/03_wit_and_manifest.md`, `docs/05_module_sdk.md`
- Files explicitly out-of-bounds for this step: code.
- Expected sub-agent dispatches:
  - "Run the two Doc Impact rg greps + the three gate commands; FACT each"
- Context cost: `S`
- Authoritative docs: the two being edited.
- OrcaSlicer refs: none.
- Verification:
  - `rg -q 'wall-source-region-id' docs/03_wit_and_manifest.md && echo HIT` — FACT
  - `rg -q 'prior-infill|prior_infill' docs/05_module_sdk.md && echo HIT` — FACT
- Exit condition: greps hit; gates green.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | WIT + SDK + macros (two waves) |
| Step 2 | M | dispatch population + marshal |
| Step 3 | M | compiler-driven sweep, fully delegated reads |
| Step 4 | M | echo guest + 5 contract tests + drift |
| Step 5 | S | docs + gates |

Aggregate M: the sweep and test steps are wide but shallow; every heavy read is delegated.

## Packet Completion Gate

- All steps complete.
- Every step exit condition is met.
- Packet acceptance criteria green (each verification command dispatched and returned PASS).
- `docs/07_implementation_status.md` updated for TASK-255 (via worker dispatch — never edited
  by loading the full backlog into the implementer's context).
- Reopened or superseded packet status transitions reconciled (none expected).
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md`.
- Confirm packet-level verification commands are green.
- Record any remaining packet-local risk explicitly before moving to `status: implemented`.
- Confirm the implementer's peak context usage stayed under 70%; if not, log it as a
  packet-authoring lesson for future spec-packet-generator runs.
