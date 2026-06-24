# Implementation Plan: support-planner-to-buildplate-pruning

## Execution Rules

- One atomic step at a time. Maps to `TASK-264`.
- TDD: AC-2 through AC-4, AC-N1 unit tests authored RED before implementation.

## Steps

### Step 1: Confirm Orca semantics + locate edit sites

- Task IDs: `TASK-264`
- Files allowed to read:
  - `docs/specs/support-modules-orca-port.md` §C5 — directly
  - Planner contact creation + propagation blocks (range-read)
- Sub-agent dispatches:
  - "Summarize OrcaSlicer `TreeSupport::drop_nodes` `unsupported_branch_leaves` flow; return SUMMARY ≤ 200 words. Confirm pruning semantics + `to_buildplate` interactions."
  - "Summarize OrcaSlicer `generate_contact_points` `to_buildplate` initialization rule; return SUMMARY ≤ 200 words. Confirm whether the test polygon is per-layer outline or cumulative-below."
  - "Locate `PlannedSupportNode` struct, the `contacts_by_layer.push` calls, and the post-`clamp_to_avoidance` propagation block in `support-planner/src/lib.rs`; return LOCATIONS file:line."
- Files allowed to edit: none.
- Context cost: `S`
- Verification: implementer can recite the prune rule + the footprint source.
- Exit condition: discovery captured; `[FWD]` open question resolved.

### Step 2: Author AC-2 through AC-4, AC-N1 as RED

- Files allowed to edit (≤ 3): `modules/core-modules/support-planner/tests/to_buildplate_tdd.rs` (new).
- Sub-agent dispatches:
  - "Run `cargo test -p support-planner --test to_buildplate_tdd`; return FACT (expected: all four fail)."
- Context cost: `S`
- Verification: RED state confirmed.
- Exit condition: RED.

### Step 3: Implement `to_buildplate` tracking + pruning + config plumbing

- Files allowed to edit (≤ 3):
  - `modules/core-modules/support-planner/src/lib.rs`
  - `modules/core-modules/support-planner/support-planner.toml`
- Sub-agent dispatches:
  - "Run `cargo build -p support-planner`; return FACT."
  - "Run `cargo test -p support-planner --test to_buildplate_tdd`; return FACT pass/fail."
  - "Run `cargo test -p support-planner`; return FACT (existing tests don't regress)."
  - "Run AC-5 manifest grep; return FACT."
- Context cost: `M`
- Verification: AC-1, AC-2, AC-3, AC-4, AC-5, AC-N1 PASS.
- Exit condition: feature live; unit-tested.

### Step 4: Add wedge invariant (AC-6, AC-7) + regenerate goldens + extend docs/specs invariant list

- Files allowed to edit (3, at the ceiling — the two goldens are mechanically rewritten by the xtask regen recipe and arrive paired):
  - `crates/slicer-runtime/tests/integration/support_invariants_wedge_tdd.rs`
  - `resources/golden/support_regression_wedge_branch_count.txt`
  - `resources/golden/support_regression_wedge_endpoints.txt`
- Sub-agent dispatches:
  - "Run xtask golden-regen; return FACT."
  - "Run `cargo test -p slicer-runtime --test support_invariants_wedge_tdd`; return FACT per-test."
  - "Run `cargo test -p slicer-runtime --test support_golden_regression_wedge_tdd`; return FACT."
  - "Run `cargo xtask build-guests --check`; return FACT."
- Context cost: `M`
- Verification: AC-6, AC-7, AC-8, AC-9 PASS.
- Exit condition: invariant active.

### Step 5: Update `docs/specs/support-modules-orca-port.md` §Validation Strategy invariant list

- Files allowed to edit (≤ 3):
  - `docs/specs/support-modules-orca-port.md`
- Sub-agent dispatches:
  - "Run `rg -q 'build_plate_only_emits_no_to_model_branches' docs/specs/support-modules-orca-port.md`; return FACT."
- Context cost: `S`
- Verification: Doc Impact grep PASS.
- Exit condition: docs updated.

### Step 6: Final verification + close

- Files allowed to edit: none.
- Sub-agent dispatches:
  - "Run all AC commands sequentially; return FACT (PASS / FAIL list)."
  - "Run `cargo clippy --workspace --all-targets -- -D warnings`; return FACT."
- Context cost: `S`
- Verification: all ACs PASS; clippy clean.

## Per-Step Budget Roll-Up

| Step | Cost |
| --- | --- |
| 1 | S |
| 2 | S |
| 3 | M |
| 4 | M |
| 5 | S |
| 6 | S |

Aggregate: `M`.

## Packet Completion Gate

- All ACs PASS; `cargo xtask build-guests --check` clean; `docs/07` marks `TASK-264` `[x]`; docs/specs invariant list extended.

## Acceptance Ceremony

- Re-dispatch every AC command; confirm gate commands green; mark `TASK-264` `[x]`; transition.
