# Implementation Plan: support-plan-raft-plan-and-raftinfill-role

## Execution Rules

- Work one atomic step at a time; map every step to `TASK-289`.
- Use TDD: the new sdk test (Step 5) is written against the AC-4/AC-N1/AC-N3 behavioral contract and is the gate for Steps 2-4.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".
- The audit (Step 1) MUST run before the variant is added (Step 2). The audit's `[explicit]` list determines which sites Step 4 must touch; adding the variant before the audit is in hand would land a non-exhaustive-match compile error.

## Steps

### Step 1: Audit + discovery (read-only, may be parallelized)

- Task IDs: `TASK-289`
- Objective: enumerate every workspace `match role` site (tagged `[explicit]` or `[wildcard]`), every test that hard-asserts on `CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION` literal `1.2.0`, and confirm the WIT mirror file + bindgen name convention.
- Precondition: packet status `active`.
- Postcondition: an audit table is captured in the worker's return; the planner can map Step 4 arm additions and Step 2 schema-bump fallout without further discovery.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/slice_ir.rs` — lines 1655-1700 and 245-260
  - `crates/slicer-sdk/src/views.rs` — lines 480-520
  - `crates/slicer-schema/wit/deps/types.wit` — full read (small)
  - `docs/adr/0009-raft-as-layer-infill-role.md` — full read (94 lines)
  - `docs/specs/support-modules-orca-port.md` — lines 380-420 (§C6, §C7)
  - `crates/slicer-ir/tests/` — bounded `rg` only
  - `crates/slicer-runtime/tests/` — bounded `rg` only
- Files allowed to edit (at most 3): none.
- Files explicitly out of bounds:
  - `OrcaSlicerDocumented/**`
  - `target/`, `Cargo.lock`, generated code
  - `support-planner/**`
  - `docs/specs/raft-default-module.md` (read for alignment only)
- Expected sub-agent dispatches:
  - "Locate every workspace `match role` site that switches on `ExtrusionRole` and tag each as `[explicit]` (no `_ =>`) or `[wildcard]` (has `_ =>`)"; scope: `crates/ modules/ --type rust`; return: `LOCATIONS` ≤ 20 entries with tag; purpose: AC-6 + Step 4.
  - "Locate every test in `crates/slicer-ir/tests/` and `crates/slicer-runtime/tests/` that hard-asserts on the literal `1.2.0` (or `SemVer { major: 1, minor: 2, ... }`)"; scope: bounded `rg`; return: `LOCATIONS` ≤ 20 entries; purpose: Step 2 fallout.
  - "Confirm `ExtrusionRole` is mirrored in `crates/slicer-schema/wit/deps/types.wit` (NOT `ir-types.wit`) and report the current named-member count + bindgen name convention (e.g., `top-solid-infill` → `TopSolidInfill`)"; scope: `crates/slicer-schema/wit/`; return: `SNIPPETS` ≤ 20 lines; purpose: Step 3 WIT edit.
- Context cost: `S`
- Authoritative docs:
  - `docs/adr/0009-raft-as-layer-infill-role.md` — full read
  - `docs/specs/support-modules-orca-port.md` §C6, §C7 — range read
- OrcaSlicer refs: none.
- Verification:
  - The audit dispatch returns ≥ 14 `[explicit]`+`[wildcard]`-tagged entries.
  - The 1.2.0 literal search returns the exact set of tests to update in Step 2.
  - The WIT mirror search confirms `raft-infill` will be added to `types.wit`, not `ir-types.wit`.
- Exit condition: audit table captured; the planner can proceed to Step 2 without re-reading the audit source.

### Step 2: Add `RaftInfill` variant to `ExtrusionRole`; bump `CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION` to 1.3.0; update fallout tests

- Task IDs: `TASK-289`
- Objective: introduce the new enum variant and the schema bump atomically, with every fallout test updated in the same step.
- Precondition: Step 1 audit captured (specifically: the list of tests that hard-assert on `1.2.0`).
- Postcondition: `RaftInfill` is in `ExtrusionRole`; `CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION` is `1.3.0`; every fallout test is updated; `cargo build -p slicer-ir` succeeds; `cargo test -p slicer-ir` succeeds.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/slice_ir.rs` — lines 1655-1700 and 245-260 (enum + schema constant).
  - The fallout tests identified by Step 1 (specific test files only).
- Files allowed to edit (at most 3):
  - `crates/slicer-ir/src/slice_ir.rs` — 2 lines added (variant + literal).
  - Each fallout test file — at most 1 line per test.
- Files explicitly out of bounds:
  - `support-planner/**` — Step 4 owns the planner.
  - `crates/slicer-sdk/src/views.rs` — Step 3.
  - `crates/slicer-schema/wit/**` — Step 3.
- Blast-radius discipline (mandatory):
  - **Struct-literal blast radius**: adding `RaftInfill` to `ExtrusionRole` extends every exhaustive `match` surface. The 14 `match role` sites from the audit will not compile until Step 4 adds the arms. **Mitigation**: this step is load-bearing first; `cargo build -p slicer-ir` (NOT workspace-wide) is the gate. Workspace-wide build is Step 4's gate.
  - **Test-assertion fallout**: every test that hard-asserts on `1.2.0` (enumerated by Step 1) is updated in this step. The bump and its fallout land together.
- Expected sub-agent dispatches:
  - "Run `cargo build -p slicer-ir`"; return: `FACT` pass/fail; `SNIPPETS` ≤ 30 lines FIRST error on fail.
  - "Run `cargo test -p slicer-ir`"; return: `FACT` pass/fail; `SNIPPETS` ≤ 30 lines on failure.
  - "Run `rg 'RaftInfill' crates/slicer-ir/src/slice_ir.rs`"; return: `FACT` (line of insertion).
  - "Run `rg 'minor: 3' crates/slicer-ir/src/slice_ir.rs`"; return: `FACT` (confirms bump).
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0009-raft-as-layer-infill-role.md` §Consequences — confirms the semver-minor bump.
- OrcaSlicer refs: none.
- Verification:
  - AC-1 grep PASS.
  - AC-5 grep PASS.
  - `cargo build -p slicer-ir` PASS.
  - `cargo test -p slicer-ir` PASS.
  - No fallout test remains at the old `1.2.0` value.
- Exit condition: `ExtrusionRole` carries `RaftInfill`; schema is at `1.3.0`; `slicer-ir` builds and tests pass.

### Step 3: Add `claim:raft-fill` arm in `should_emit`; add `raft-infill` to WIT mirror; rebuild guests

- Task IDs: `TASK-289`
- Objective: extend the role/claim dispatch and the WIT mirror, then rebuild all 20 guests.
- Precondition: Step 2 complete (`RaftInfill` exists in the enum).
- Postcondition: `should_emit` has the new arm; WIT mirror has the new member; all 20 guests are fresh; `cargo build -p slicer-sdk` and `cargo test -p slicer-wasm-host --test wit_boundary_tdd` both PASS.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-sdk/src/views.rs` — lines 480-520.
  - `crates/slicer-schema/wit/deps/types.wit` — full read.
- Files allowed to edit (at most 3):
  - `crates/slicer-sdk/src/views.rs` — 1 line.
  - `crates/slicer-schema/wit/deps/types.wit` — 1 line.
- Files explicitly out of bounds:
  - `support-planner/**`
  - The 14 `match role` sites (Step 4).
  - `crates/slicer-sdk/tests/` (Step 5).
- Expected sub-agent dispatches:
  - "Run `cargo build -p slicer-sdk`"; return: `FACT` pass/fail.
  - "Run `cargo xtask build-guests`"; return: `FACT` pass/fail; do NOT paste rebuild log.
  - "Run `cargo xtask build-guests --check`"; return: `FACT` clean / STALE.
  - "Run `cargo test -p slicer-wasm-host --test wit_boundary_tdd`"; return: `FACT` pass/fail; `SNIPPETS` ≤ 30 lines on failure.
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0009-raft-as-layer-infill-role.md` §Decision — the role/claim extension is canonical.
- OrcaSlicer refs: none.
- Verification:
  - AC-2 grep PASS.
  - AC-3 grep PASS.
  - AC-7 `cargo test -p slicer-wasm-host --test wit_boundary_tdd` PASS.
  - `cargo xtask build-guests --check` clean.
- Exit condition: claim arm live, WIT mirror live, guests fresh, WIT round-trip green.

### Step 4: Add `RaftInfill` arm at every `[explicit]` `match role` site (workspace-wide exhaustive audit)

- Task IDs: `TASK-289`
- Objective: add the `ExtrusionRole::RaftInfill =>` arm at every site tagged `[explicit]` in Step 1's audit. Sites tagged `[wildcard]` are exempt.
- Precondition: Step 1 audit captured; Step 2 added the variant (workspace-wide build currently fails with non-exhaustive-match errors at the `[explicit]` sites).
- Postcondition: every workspace `match role` site either has the new arm or uses a `_ =>` wildcard; `cargo build --workspace --all-targets` PASS.
- Files allowed to read, with ranges when over 300 lines:
  - The 14 audit sites (range reads only; ≤ 30 lines per site).
- Files allowed to edit (at most 3):
  - Each `[explicit]` site (1 line per site; expect 5-6 sites: `gcode::emit`, `gcode::serialize`, `overhang-classifier-default`, `path-optimization-default`, the two `marshal::leaf` sites). The `gcode_feedrate_emission_tdd.rs` test and the two `runtime` test sites are NOT edited preemptively — they are touched only if a compile error requires it.
- Files explicitly out of bounds:
  - `OrcaSlicerDocumented/**`
  - `target/`, `Cargo.lock`
  - The `[wildcard]` sites (no edit).
- Expected sub-agent dispatches:
  - "Run `cargo build --workspace --all-targets`"; return: `FACT` pass/fail; `SNIPPETS` ≤ 30 lines FIRST error.
  - "Run `rg -l 'match .*ExtrusionRole' crates/ modules/ --type rust | xargs -I{} sh -c 'rg -q "ExtrusionRole::RaftInfill" "{}" || rg -q "_ =>" "{}"'`"; return: `FACT` pass/fail (exit 0 only if every site has the arm or a wildcard).
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0009-raft-as-layer-infill-role.md` — confirms the additive role extension.
- OrcaSlicer refs: none.
- Verification:
  - AC-6 grep PASS.
  - AC-N2 `cargo build --workspace --all-targets` PASS.
  - Every `[explicit]` site has the new arm; every `[wildcard]` site does not need one.
- Exit condition: workspace compiles; audit gate PASS.

### Step 5: Author `should_emit_raft_fill_claim_tdd.rs` (AC-4, AC-N1, AC-N3)

- Task IDs: `TASK-289`
- Objective: write the behavioral test that proves the new role/claim arm works (held_claims match → emit; no match → suppress; empty claims → suppress) and wire it into the slicer-sdk test binary.
- Precondition: Step 3 added the claim arm; Step 4 added the variant arms at all `[explicit]` sites.
- Postcondition: `cargo test -p slicer-sdk --test should_emit_raft_fill_claim_tdd` PASS for AC-4, AC-N1, AC-N3.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-sdk/src/views.rs` — lines 480-520 (the dispatch under test).
  - `crates/slicer-sdk/tests/` — read 1-2 existing `*_tdd.rs` tests for the wiring convention.
- Files allowed to edit (at most 3):
  - `crates/slicer-sdk/tests/should_emit_raft_fill_claim_tdd.rs` — new.
- Files explicitly out of bounds:
  - The non-sdk test sites (Step 4's audit covered those).
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-sdk --test should_emit_raft_fill_claim_tdd`"; return: `FACT` per-test pass/fail; `SNIPPETS` ≤ 30 lines on failure.
- Context cost: `M`
- Authoritative docs:
  - `crates/slicer-sdk/src/views.rs::should_emit` — the dispatch under test.
- OrcaSlicer refs: none.
- Verification:
  - AC-4 PASS.
  - AC-N1 PASS.
  - AC-N3 PASS.
- Exit condition: behavioral coverage in place; the silent-true-fallback risk is gated by a passing test.

### Step 6: Verify C7 state (read-only; no edits expected)

- Task IDs: `TASK-289`
- Objective: confirm the `traditional-support` lead `//!` block still documents the C7 non-consumption and the manifest still excludes `SupportPlanIR` from `reads`.
- Precondition: nothing.
- Postcondition: AC-8 and AC-9 PASS.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/traditional-support/src/lib.rs` — first 30 lines.
  - `modules/core-modules/traditional-support/traditional-support.toml` — full read.
- Files allowed to edit (at most 3): none.
- Files explicitly out of bounds:
  - All other modules.
- Expected sub-agent dispatches:
  - "Run AC-8 grep"; return: `FACT` pass/fail.
  - "Run AC-9 grep"; return: `FACT` pass/fail.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-modules-orca-port.md` §C7 — the decision text.
- OrcaSlicer refs: none.
- Verification:
  - AC-8 PASS.
  - AC-9 PASS.
- Exit condition: C7 verified; if either AC fails, the packet reports a regression in `traditional-support`'s state, not a packet authoring defect (the packet does not edit those files; the regression would belong to a separate packet).

### Step 7: Final packet verification + closure

- Task IDs: `TASK-289`
- Objective: re-run every pipe-suffixed AC command and confirm the workspace lints cleanly.
- Precondition: Steps 1-6 complete.
- Postcondition: every AC PASS; clippy clean; guests fresh; `packet.spec.md` ready for `status: implemented`.
- Files allowed to read: none beyond prior.
- Files allowed to edit: none.
- Expected sub-agent dispatches:
  - "Run AC-1 through AC-9 + AC-N1 + AC-N2 + AC-N3 commands sequentially"; return: `FACT` (PASS / FAIL list).
  - "Run `cargo clippy --workspace --all-targets -- -D warnings`"; return: `FACT` pass/fail; `SNIPPETS` ≤ 20 lines FIRST error on fail.
  - "Run `cargo xtask build-guests --check`"; return: `FACT` clean.
- Context cost: `S`
- Authoritative docs: none beyond prior.
- OrcaSlicer refs: none.
- Verification:
  - Every AC PASS.
  - `cargo clippy --workspace --all-targets -- -D warnings` PASS.
  - `cargo xtask build-guests --check` clean.
- Exit condition: closure summary recorded; `packet.spec.md` ready for `status: implemented`.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Discovery + audit. |
| Step 2 | M | IR + schema bump + fallout tests. |
| Step 3 | M | Claim arm + WIT mirror + guest rebuild. |
| Step 4 | M | 14-site `match role` audit + arm additions. |
| Step 5 | M | Behavioral test (AC-4, AC-N1, AC-N3). |
| Step 6 | S | C7 verification (read-only). |
| Step 7 | S | Final verification. |

Aggregate: `M`. No step is L. Step 2-5 each are `M` because they edit source + run a focused build/test; the audit (Step 1) and the final verify (Step 7) are `S`.

## Packet Completion Gate

- All seven steps complete; every exit condition met.
- AC-1 through AC-9 + AC-N1 + AC-N2 + AC-N3 all PASS.
- `docs/07_implementation_status.md` is updated with the new `TASK-289` row marked `[x]` (via worker dispatch, not a full backlog read).
- `cargo xtask build-guests --check` clean.
- `packet.spec.md` ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC command from `packet.spec.md`.
- Confirm gate commands green: `cargo xtask build-guests --check`, `cargo build --workspace --all-targets`, the new `should_emit_raft_fill_claim_tdd` test, `cargo clippy --workspace --all-targets -- -D warnings`.
- Mark `TASK-289` `[x]` in `docs/07_implementation_status.md` (worker dispatch).
- Transition `packet.spec.md` to `status: implemented`.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands use `--all-targets` so the test, bench, and example targets compile.
