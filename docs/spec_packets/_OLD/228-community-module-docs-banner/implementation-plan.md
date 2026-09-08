# Implementation Plan: 228-community-module-docs-banner

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Add the four backlog rows to `docs/07_implementation_status.md`

- Task IDs: `TASK-339`
- Objective: Insert `TASK-336`..`TASK-339` as open-checkbox rows in the existing Workstream 5 block, mirroring the TASK-330..335 format.
- Precondition: `rg -c '^- \[ \] TASK-33[6-9] —' docs/07_implementation_status.md` returns `0` (verified absent).
- Postcondition: `rg -c '^- \[ \] TASK-33[6-9] —' docs/07_implementation_status.md` returns `4`, one row each for 336/337/338/339. AC-3 and AC-N1 green.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/07_implementation_status.md` - lines `320-325` only (row format).
- Files allowed to edit (at most 3):
  - `docs/07_implementation_status.md`
- Files explicitly out of bounds:
  - `docs/07_implementation_status.md` beyond lines 320-325 (never read wholesale).
  - `docs/spec_packets/225-*/**, 226-*/**, 227-*/**` (draft dirs do not exist; never read).
- Blast-radius discipline: not applicable.
- Expected sub-agent dispatches:
  - Question: what is the exact row format and surrounding block for TASK-330..335? scope: `docs/07_implementation_status.md` lines 320-325; return: `SNIPPETS` (≤10 lines).
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/community-modules-dragon-curve-plan.md` - direct read of the packet queue table (the four one-sentence goals become the row descriptions).
- OrcaSlicer refs:
  - none.
- Verification:
  - `rg -c '^- \[ \] TASK-33[6-9] —' docs/07_implementation_status.md` - FACT: exact count 4.
  - `rg -q '^- \[ \] TASK-336 —' docs/07_implementation_status.md` - FACT pass/fail.
- Exit condition: AC-3 and AC-N1 pass.

### Step 2: Add the labeled-example note to `docs/14_submodule_programming_languages.md`

- Task IDs: `TASK-339`
- Objective: Append the social-rule sentence to §Community-module context only if 225's edits do not already carry it.
- Precondition: Step 1 done (backlog rows land first so the spec-path references are stable).
- Postcondition: `docs/14` §Community-module context states real community modules are authored in forks as pinned submodules and never added here, with the committed Dragon Curve as a labeled example only. AC-2 green.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/14_submodule_programming_languages.md` - lines `96-171` (§Community-module context).
- Files allowed to edit (at most 3):
  - `docs/14_submodule_programming_languages.md`
- Files explicitly out of bounds:
  - `docs/14_submodule_programming_languages.md` lines 1-95 (language table; not needed).
  - `docs/feasibility-probes/*` (225's raw records).
- Blast-radius discipline: not applicable.
- Expected sub-agent dispatches:
  - Question: does §Community-module context already contain the labeled-example sentence? scope: that section only; return: `SNIPPETS` (≤20 lines).
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/community-modules-dragon-curve-infill.md` §1 - direct read (the labeled-example wording).
- OrcaSlicer refs:
  - none.
- Verification:
  - `rg -q 'labeled example' docs/14_submodule_programming_languages.md` - FACT pass/fail (AC-2).
  - `rg -q 'never added' docs/14_submodule_programming_languages.md` - FACT pass/fail.
- Exit condition: AC-2 passes.

### Step 3: Add the `CLAUDE.md` Community Modules section

- Task IDs: `TASK-339`
- Objective: Insert a brief `## Community Modules` section matching the file's short-section style.
- Precondition: Steps 1-2 done (the section references the same docs paths).
- Postcondition: `CLAUDE.md` carries the section with the fork/submodule rule, the labeled-example statement, and the two doc references. AC-1 green.
- Files allowed to read, with ranges when over 300 lines:
  - `CLAUDE.md` - lines `1-27` (section order).
- Files allowed to edit (at most 3):
  - `CLAUDE.md`
- Files explicitly out of bounds:
  - `CLAUDE.md` beyond the section-order region (the rest is build/test discipline, not needed).
- Blast-radius discipline: not applicable.
- Expected sub-agent dispatches:
  - none (direct edit; the section-order region is small).
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/community-modules-dragon-curve-infill.md` §1, §7 - direct read (the social rule wording).
- OrcaSlicer refs:
  - none.
- Verification:
  - `rg -q '^## Community Modules' CLAUDE.md` - FACT pass/fail.
  - `rg -q 'never added to this repository' CLAUDE.md` - FACT pass/fail.
- Exit condition: AC-1 passes.

### Step 4: Update the spec status line

- Task IDs: `TASK-339`
- Objective: Amend `docs/specs/community-modules-dragon-curve-infill.md` line 3 to append the packet queue plan path.
- Precondition: Steps 1-3 done.
- Postcondition: The status line retains the grilling-complete note and names `docs/specs/community-modules-dragon-curve-plan.md`. AC-4 green.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/specs/community-modules-dragon-curve-infill.md` - line `3` only.
- Files allowed to edit (at most 3):
  - `docs/specs/community-modules-dragon-curve-infill.md`
- Files explicitly out of bounds:
  - `docs/specs/community-modules-dragon-curve-infill.md` beyond line 3 (already read in the governing step; this edit is the status line only).
- Blast-radius discipline: not applicable.
- Expected sub-agent dispatches:
  - none.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/community-modules-dragon-curve-plan.md` - direct read (the plan file path).
- OrcaSlicer refs:
  - none.
- Verification:
  - `rg -q 'Status: grilling complete; ready to be broken into spec packets by a downstream session' docs/specs/community-modules-dragon-curve-infill.md` - FACT pass/fail.
  - `rg -q 'docs/specs/community-modules-dragon-curve-plan.md' docs/specs/community-modules-dragon-curve-infill.md` - FACT pass/fail.
- Exit condition: AC-4 passes.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | four-row targeted insert |
| Step 2 | S | additive docs/14 note |
| Step 3 | S | CLAUDE.md section |
| Step 4 | S | status line append |

Split before activation if aggregate cost exceeds M or any step is L.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read (this packet IS the backlog-row editor; the final status flip of TASK-339 is done in the same targeted edit).
- Reconcile reopened/superseded status transitions.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk (the docs/14 overlap with 225's eventual edit).
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
