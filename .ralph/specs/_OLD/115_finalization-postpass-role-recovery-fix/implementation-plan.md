# Implementation Plan: 115_finalization-postpass-role-recovery-fix

## Execution Rules

- Blocked by packet 113 — do not start until 113 has closed and the inbound role converters live in `marshal`.
- TDD: write AC-2/AC-3 tests first, confirm RED against the post-113 tree, then make GREEN with the converter change (AC-N1).
- Delegate every `cargo` run; absorb only FACT pass/fail + first failing assertion. Tee to `target/test-output.log`.
- No step edits more than 3 files or rates L.

## Steps

### Step 1 — Write the failing regression tests (RED)
- Objective: add the `marshal::leaf` round-trip test and the finalization dispatch contract test; confirm both fail on current (post-113) code.
- Precondition: 113 closed; locate the lossy inbound role converter and its call sites with `rg`.
- Postcondition: both new tests exist and FAIL (round-trip yields `Custom`); AC-N1 red run recorded.
- Read: `marshal/leaf.rs` (converter names); ADR-0021 §Amendment; a finalization dispatch fixture under `tests/common/`.
- Edit (≤2): `marshal/leaf.rs` (test mod), `tests/contract/<new>.rs`.
- Dispatches: run each new test, return FACT (expect FAIL).
- Context cost: **S**.
- Verify: both tests present and red.
- Exit condition: red runs captured.

### Step 2 — Collapse to the recovering converter (GREEN)
- Objective: delete the two lossy WIT→IR role variants (`finalization_role_wit_to_ir`, `convert_postpass_role`); repoint finalization and postpass call sites to the recovering `marshal::leaf::convert_extrusion_role`.
- Precondition: Step 1 red.
- Postcondition: AC-1, AC-2, AC-3 pass; any test that pinned the old lossy output is updated (it pinned the bug).
- Read: the lossy variant + its two call sites (located in Step 1).
- Edit (≤3): `marshal/leaf.rs`, the call-site file (`dispatch.rs` or `host.rs` or a `marshal` module, per 113's layout), and at most one pre-existing test asserting the old lossy output.
- Dispatches: `cargo test -p slicer-wasm-host --lib marshal::leaf` and `--test contract finalization_role_round_trip` (FACT pass/fail); `cargo check --workspace --all-targets` (FACT).
- Context cost: **S**.
- Verify: AC-1 grep (one converter, no lossy variant); AC-2/AC-3 green.
- Cheapest falsifier: round-trip test still yields `Custom`.

### Step 3 — Packet completion gate
- Objective: full gate green; no collateral regressions.
- Precondition: Steps 1–2 done.
- Postcondition: all ACs pass.
- Edit: none (fixes only within in-scope files).
- Dispatches: `cargo check --workspace --all-targets`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test -p slicer-wasm-host --test contract` and `--test unit` (FACT pass/fail each).
- Context cost: **S**.
- Verify: gate subset in `packet.spec.md` green; AC-1…AC-3 pass; AC-N1 transition recorded.
- Exit condition: every AC verification command passes.

## Per-Step Budget Roll-Up

S, S, S → aggregate **S**. No L step.

## Packet Completion Gate

- AC-1…AC-3 pass; AC-N1 red→green transition documented.
- `cargo check --workspace --all-targets` and `cargo clippy --workspace --all-targets -- -D warnings` clean.
- `slicer-wasm-host` contract + unit buckets `0 failed`.

## Acceptance Ceremony

Run the gate subset, then each per-AC command, recording FACTs. Does not require `cargo test --workspace`; the targeted `slicer-wasm-host` buckets cover the fix. If closure policy mandates the full suite, delegate it to a sub-agent returning only `FACT pass/fail + first failing test`.
