# Implementation Plan: 129_clip-polylines

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first, then implementation, then the narrowest falsifying validation.
- Each step honors the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. The fields below are not optional metadata — they are the budget contract for this step.

## Steps

### Step 1: RED — author the 8 `clip_polylines_*` tests

- Task IDs:
  - `TASK-254`
- Objective: land the failing TDD suite pinning the six geometric guarantees + two negative
  cases (AC-1…AC-6, AC-N1, AC-N2), with a temporary `todo!()`-free stub so the suite compiles
  and fails on assertions (stub returns `Vec::new()`).
- Precondition: clean working tree on the packet branch; `cargo check -p slicer-core` green.
- Postcondition: `polygon_ops_tdd.rs` contains 8 new tests named exactly as in the AC pipe
  commands; `clip_polylines` stub exists and compiles; AC-1…AC-6 tests FAIL, AC-N1/AC-N2 may
  incidentally pass (stub returns empty).
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-core/tests/polygon_ops_tdd.rs` — full (test file; check for reusable
    square/hole fixtures)
  - `crates/slicer-core/src/polygon_ops.rs` — imports region ~lines 1-110 only
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/tests/polygon_ops_tdd.rs`
  - `crates/slicer-core/src/polygon_ops.rs` (stub only)
- Files explicitly out-of-bounds for this step:
  - clipper2-rust crate source; `OrcaSlicerDocumented/**`; gyroid module
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-core --test polygon_ops_tdd -- clip_polylines 2>&1 | tee
    target/test-output.log | grep -E '^test |^test result'`; return FACT: list of pass/fail
    per test" — confirm RED state
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/infill-parity-rectilinear-gyroid-linker.md` — lines 124-176 (§Phase 0) only
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p slicer-core --test polygon_ops_tdd -- clip_polylines 2>&1 | tee target/test-output.log | grep "^test result"` — dispatch as FACT
- Exit condition: 6 named positive tests RED, suite compiles, stub in place.

### Step 2: GREEN — implement `clip_polylines` on `Clipper64`

- Task IDs:
  - `TASK-254`
- Objective: replace the stub with the real implementation: convert `&[ExPolygon]` contours +
  holes to closed clip `Paths64`, feed polylines via `add_open_subject`, run
  `ClipType::Intersection` + `FillRule::NonZero`, convert `solution_open` back to
  `Vec<Vec<Point2>>`; rustdoc lists the six guarantees.
- Precondition: Step 1 exit condition met (RED suite in place).
- Postcondition: all 8 `clip_polylines_*` tests pass; no other `polygon_ops_tdd` test
  regressed; rustdoc present.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-core/src/polygon_ops.rs` — imports + existing wrapper idiom (~lines 1-150)
  - `crates/slicer-core/tests/polygon_ops_tdd.rs` — the new tests
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/polygon_ops.rs`
- Files explicitly out-of-bounds for this step:
  - clipper2-rust crate source (on signature mismatch: dispatch the contingency FACT from
    `design.md`, do not open the crate)
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-core --test polygon_ops_tdd 2>&1 | tee target/test-output.log |
    grep '^test result'`; return FACT pass/fail + counts; on failure SNIPPETS ≤20 lines"
  - (Contingency) "Exact public signatures of `Clipper64::{add_open_subject, add_clip,
    execute}` in clipper2-rust 1.0.3; FACT ≤5 lines"
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/infill-parity-rectilinear-gyroid-linker.md` — lines 124-176 only
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p slicer-core --test polygon_ops_tdd 2>&1 | tee target/test-output.log | grep "^test result"` — FACT
  - `cargo clippy -p slicer-core --all-targets -- -D warnings` — FACT
- Exit condition: full `polygon_ops_tdd` binary green; clippy clean.

### Step 3: Doc Impact + workspace/guest gates

- Task IDs:
  - `TASK-254`
- Objective: add the `clip_polylines` mention to `docs/05_module_sdk.md`'s helper-surface
  line; run the packet gates.
- Precondition: Step 2 exit condition met.
- Postcondition: Doc Impact grep hits; `cargo check --workspace --all-targets` green;
  `cargo xtask build-guests --check` adjudicated (clean, or rebuilt then clean).
- Files allowed to read (with line-range hints when > 300 lines):
  - `docs/05_module_sdk.md` — lines 55-75 only
- Files allowed to edit (≤ 3):
  - `docs/05_module_sdk.md`
- Files explicitly out-of-bounds for this step:
  - everything else
- Expected sub-agent dispatches:
  - "Run `cargo check --workspace --all-targets`; FACT pass/fail"
  - "Run `cargo xtask build-guests --check`; FACT clean or STALE list; if STALE, run
    `cargo xtask build-guests` and re-return FACT"
  - "Run `rg -q 'clip_polylines' docs/05_module_sdk.md && echo HIT`; FACT"
- Context cost: `S`
- Authoritative docs:
  - `docs/05_module_sdk.md` — lines 55-75
- OrcaSlicer refs: none.
- Verification:
  - the three dispatches above — each FACT
- Exit condition: all three FACTs green.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | test authoring against a known-fixture file |
| Step 2 | S | single-function implementation |
| Step 3 | S | doc line + gates, all delegated |

## Packet Completion Gate

- All steps complete.
- Every step exit condition is met.
- Packet acceptance criteria green (each verification command dispatched and returned PASS).
- `docs/07_implementation_status.md` updated for TASK-254 (via worker dispatch — never edited
  by loading the full backlog into the implementer's context).
- Reopened or superseded packet status transitions reconciled (none expected).
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md`.
- Confirm packet-level verification commands are green.
- Record any remaining packet-local risk explicitly before moving to `status: implemented`.
- Confirm the implementer's peak context usage stayed under 70%; if not, log it as a
  packet-authoring lesson for future spec-packet-generator runs.
