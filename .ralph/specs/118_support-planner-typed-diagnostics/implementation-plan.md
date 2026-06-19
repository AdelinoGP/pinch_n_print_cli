# Implementation Plan: support-planner-typed-diagnostics

## Execution Rules

- One atomic step at a time.
- Each step maps back to `TASK-253` (cap diagnostic) or `TASK-163b-diagnostic` (typed channel + migrations).
- The WIT change is the load-bearing first step; subsequent steps depend on guest bindgen being fresh.
- TDD where the failing test is feasible: Step 5 lands tests RED before Step 6 lands the host-side collection that turns them GREEN; Step 7 lands the planner-side tests RED before Step 8 migrates the call sites.

## Steps

### Step 1: Confirm WIT addition shape via dispatches + read ADR-0010

- Task IDs: `TASK-163b-diagnostic`
- Objective: confirm the exact lines to add to `world-prepass.wit`, the SDK output-builder location, and the host audit struct location before any edit.
- Precondition: ADR-0010 available; current `world-prepass.wit` available.
- Postcondition: working notes capture (a) the verbatim record + enum + method snippets, (b) the SDK type to extend, (c) the host audit struct + drain path.
- Files allowed to read:
  - `docs/adr/0010-typed-diagnostic-channel.md` — directly (≈90 lines)
  - `docs/specs/support-modules-orca-port.md` §B4, §B7, §D10, §D11 — directly (≤ 50 lines combined)
  - `crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit` — directly (≤ 200 lines)
  - `CLAUDE.md` §"WIT/Type Changes Checklist" + §"Guest WASM Staleness" — directly (≤ 60 lines combined)
- Files allowed to edit (≤ 3): none in this step.
- Files explicitly out-of-bounds for this step:
  - `OrcaSlicerDocumented/**` — no Orca behavior being ported
  - All guest manifests + bindgen outputs — read after rebuild via grep, never browsed
- Expected sub-agent dispatches:
  - "Summarize `docs/03_wit_and_manifest.md` §how to add a new type to a world's deps/* file; return SUMMARY ≤ 200 words." — purpose: confirm conventional shape.
  - "Locate the prepass output-builder impl in `crates/slicer-sdk/src/`; return LOCATIONS file:line + 1-line context ≤ 10 entries." — purpose: find SDK edit target.
  - "Locate `PrepassStageAudit` (or equivalent) in `crates/slicer-runtime/src/prepass.rs`; return SNIPPETS ≤ 30 lines showing the struct definition + commit path." — purpose: find host edit target.
- Context cost: `S`
- Authoritative docs:
  - `docs/adr/0010-typed-diagnostic-channel.md` — directly
- OrcaSlicer refs: none.
- Verification:
  - Implementer can name (a) every field of the new `Diagnostic` record, (b) the SDK type to extend, (c) the host audit struct field name and drain path.
- Exit condition: discovery notes captured.

### Step 2: Add `diagnostic` record + `severity-level` enum to world-prepass.wit; trigger guest rebuild

- Task IDs: `TASK-163b-diagnostic`
- Objective: edit `world-prepass.wit` to add the new types and the `push-diagnostic` method on the prepass output-builder interface; run the guest rebuild; gate on `wit_drift_detection_tdd`.
- Precondition: Step 1 working notes complete.
- Postcondition: AC-1 grep evidence holds; `cargo xtask build-guests --check` reports `up to date`; `wit_drift_detection_tdd` PASS.
- Files allowed to read: same as Step 1.
- Files allowed to edit (≤ 3):
  - `crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit`
- Files explicitly out-of-bounds for this step:
  - Bindgen output, guest manifests
- Expected sub-agent dispatches:
  - "Run `cargo xtask build-guests`; return FACT pass/fail. Do NOT paste the rebuild log." — purpose: rebuild all 20 guests.
  - "Run `cargo xtask build-guests --check`; return FACT (`up to date` or `STALE: <list>`)." — purpose: post-rebuild gate.
  - "Run `cargo test -p slicer-runtime --test wit_drift_detection_tdd`; return FACT pass/fail; SNIPPETS ≤ 20 lines on failure." — purpose: bindgen drift gate. **NOTE**: this test must be extended to assert the new types are present — Step 2 includes that extension as part of the same diff so the gate is meaningful.
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0010-typed-diagnostic-channel.md` — verbatim shape
- OrcaSlicer refs: none.
- Verification:
  - `cargo xtask build-guests --check` FACT `up to date`.
  - `cargo test -p slicer-runtime --test wit_drift_detection_tdd` FACT pass.
  - `rg -q 'record diagnostic' crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit` FACT pass.
  - `rg -q 'enum severity-level' crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit` FACT pass.
- Exit condition: WIT change is durable; all guests rebuilt; bindgen consistent.

### Step 3: SDK output-builder `Diagnostic` struct + `push_diagnostic` impl

- Task IDs: `TASK-163b-diagnostic`
- Objective: expose the Rust-side `Diagnostic` struct + `push_diagnostic` method on the prepass output-builder.
- Precondition: Step 2 complete; guest bindgen has the new types.
- Postcondition: SDK compiles; `slicer-sdk` exposes `Diagnostic` and `push_diagnostic`.
- Files allowed to read:
  - `crates/slicer-sdk/src/lib.rs` — locate the output-builder via Step 1 LOCATIONS
- Files allowed to edit (≤ 3):
  - `crates/slicer-sdk/src/lib.rs` (or the located submodule)
- Files explicitly out-of-bounds for this step:
  - The host runtime — Step 4 owns that.
- Expected sub-agent dispatches:
  - "Run `cargo build -p slicer-sdk`; return FACT pass/fail; on fail SNIPPETS ≤ 20 lines." — purpose: SDK compile gate.
  - "Run `cargo build --workspace`; return FACT pass/fail; on fail SNIPPETS ≤ 30 lines with FIRST error." — purpose: confirm no downstream breaks.
- Context cost: `S`
- Authoritative docs: same as Step 1.
- OrcaSlicer refs: none.
- Verification:
  - `cargo build -p slicer-sdk` FACT pass.
  - `cargo build --workspace` FACT pass.
- Exit condition: SDK exposes the new API; workspace compiles.

### Step 4: Host-side prepass audit gains `diagnostics: Vec<Diagnostic>`; drain path wired

- Task IDs: `TASK-163b-diagnostic`
- Objective: collect guest-emitted `Diagnostic` values into the per-stage audit with order preservation; resolve the `[FWD]` open question about `on_print_start` diagnostic emission via the inspection described in design.md.
- Precondition: Step 3 complete; SDK round-trip API exists.
- Postcondition: host audit struct has `diagnostics: Vec<Diagnostic>`; drain path collects guest emissions FIFO; the `[FWD]` decision is recorded as a comment in `prepass.rs` near the implementation.
- Files allowed to read:
  - `crates/slicer-runtime/src/prepass.rs` — locate `PrepassStageAudit` via Step 1 SNIPPETS
  - any module-instantiation path that calls `on_print_start` (delegate LOCATIONS if not obvious)
- Files allowed to edit (≤ 3):
  - `crates/slicer-runtime/src/prepass.rs`
- Files explicitly out-of-bounds for this step:
  - `modules/core-modules/support-planner/src/lib.rs` — Step 8 owns the migrations
- Expected sub-agent dispatches:
  - "Find where `on_print_start` is invoked from the host side for prepass modules; return LOCATIONS ≤ 5 entries with surrounding context." — purpose: resolve the `[FWD]` open question.
  - "Run `cargo build --workspace`; return FACT pass/fail." — purpose: host change compiles.
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0010-typed-diagnostic-channel.md`
- OrcaSlicer refs: none.
- Verification:
  - `cargo build --workspace` FACT pass.
  - Inspection comment present in `prepass.rs` documenting the `on_print_start` plumbing decision.
- Exit condition: audit struct extended; drain path wired; `[FWD]` resolved.

### Step 5: Author `prepass_diagnostic_roundtrip_tdd` as RED (AC-3, AC-N2)

- Task IDs: `TASK-163b-diagnostic`
- Objective: write the round-trip integration test that asserts a guest `push_diagnostic` call surfaces in the host audit; RED before Step 6 if the host drain is incomplete (Step 4 may have already turned it partially GREEN — note this in the test commit).
- Precondition: Step 4 complete; host has audit field + drain.
- Postcondition: `crates/slicer-runtime/tests/integration/prepass_diagnostic_roundtrip_tdd.rs` exists; tests compile; AC-3 + AC-N2 either GREEN (if Step 4 fully implemented the drain) or RED (and Step 6 owns turning them green).
- Files allowed to read:
  - `crates/slicer-runtime/tests/integration/` — confirm the test-runner pattern used by neighboring integration tests
  - `crates/slicer-runtime/tests/common/` — fixture cache, slicer cache patterns
- Files allowed to edit (≤ 3):
  - `crates/slicer-runtime/tests/integration/prepass_diagnostic_roundtrip_tdd.rs` (new file)
- Files explicitly out-of-bounds for this step:
  - `support-planner/src/lib.rs` — separate tests in Step 7
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-runtime --test prepass_diagnostic_roundtrip_tdd`; return FACT (RED or GREEN per the assertion outcome)" — purpose: confirm initial state.
- Context cost: `S`
- Authoritative docs: same as Step 1.
- OrcaSlicer refs: none.
- Verification:
  - Test file compiles; assertion behavior reported.
- Exit condition: tests exist; either Step 4 already made them GREEN (note in next-step input) or Step 6 owns the gap.

### Step 6: Close any AC-3 / AC-N2 gap if Step 5 left them RED

- Task IDs: `TASK-163b-diagnostic`
- Objective: complete the drain plumbing if AC-3 or AC-N2 still RED.
- Precondition: Step 5 tests authored; gap (if any) documented.
- Postcondition: AC-3 + AC-N2 GREEN.
- Files allowed to read:
  - `crates/slicer-runtime/src/prepass.rs`
- Files allowed to edit (≤ 3):
  - `crates/slicer-runtime/src/prepass.rs` (if needed)
- Files explicitly out-of-bounds for this step:
  - same as Step 4
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-runtime --test prepass_diagnostic_roundtrip_tdd`; return FACT pass/fail; SNIPPETS ≤ 20 lines on failure." — purpose: gate RED→GREEN.
- Context cost: `S`
- Authoritative docs: same as Step 1.
- OrcaSlicer refs: none.
- Verification:
  - AC-3 + AC-N2 FACT pass.
- Exit condition: round-trip works end-to-end.

### Step 7: Author `support_planner_diagnostic_emission_tdd` as RED (AC-4, AC-5, AC-6, AC-N1)

- Task IDs: `TASK-253`, `TASK-163b-diagnostic`
- Objective: write the four integration tests for support-planner emission. Tests fail RED because the planner still uses string-prefixed `log(...)` for the existing two warnings, and the cap-exceeded path has no diagnostic at all.
- Precondition: Step 6 complete; round-trip works.
- Postcondition: file exists; four tests compile; all four fail RED.
- Files allowed to read:
  - `crates/slicer-runtime/tests/` patterns (delegated in Step 5)
  - `modules/core-modules/support-planner/src/lib.rs` around lines 326, 341, 434, 633 (range-read)
- Files allowed to edit (≤ 3):
  - `crates/slicer-runtime/tests/integration/support_planner_diagnostic_emission_tdd.rs` (new file)
- Files explicitly out-of-bounds for this step:
  - `support-planner/src/lib.rs` — Step 8 owns the migration
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-runtime --test support_planner_diagnostic_emission_tdd`; return FACT (expected: all four fail)." — confirm RED.
- Context cost: `M`
- Authoritative docs: same as Step 1.
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p slicer-runtime --test support_planner_diagnostic_emission_tdd` — FACT all four failures.
- Exit condition: tests compile; RED state confirmed.

### Step 8: Migrate `support-planner` call sites + add 1024-cap counter; rebuild guests

- Task IDs: `TASK-253`, `TASK-163b-diagnostic`
- Objective: replace the three `log(LogLevel::Warn, ...)` calls with `output.push_diagnostic(Diagnostic { ... })`; add the per-layer drop counter for the 1024-cap path; emit one Diagnostic per layer when the counter is non-zero; rebuild the support-planner guest.
- Precondition: Step 7 tests RED.
- Postcondition: AC-7 grep evidence holds; AC-4 + AC-5 + AC-6 + AC-N1 GREEN; support-planner guest rebuilt and `cargo xtask build-guests --check` is clean.
- Files allowed to read:
  - `modules/core-modules/support-planner/src/lib.rs` around the three migration sites
- Files allowed to edit (≤ 3):
  - `modules/core-modules/support-planner/src/lib.rs`
- Files explicitly out-of-bounds for this step:
  - All other guest crates — they don't adopt the channel in this packet.
- Expected sub-agent dispatches:
  - "Run `cargo build -p support-planner`; return FACT pass/fail." — purpose: planner compiles after migration.
  - "Run `cargo xtask build-guests`; return FACT pass/fail. Do NOT paste the rebuild log." — purpose: rebuild the support-planner guest .wasm.
  - "Run `cargo xtask build-guests --check`; return FACT (`up to date` or `STALE: <list>`)." — purpose: post-rebuild gate.
  - "Run `cargo test -p slicer-runtime --test support_planner_diagnostic_emission_tdd`; return FACT pass/fail; SNIPPETS ≤ 30 lines on failure." — gate RED→GREEN.
  - "Run the three AC-7 grep commands; return FACT pass/fail." — gate AC-7.
- Context cost: `M`
- Authoritative docs: same as Step 1.
- OrcaSlicer refs: none.
- Verification:
  - AC-4, AC-5, AC-6, AC-N1 FACT pass.
  - AC-7 three greps FACT pass.
  - `cargo xtask build-guests --check` FACT `up to date`.
- Exit condition: all planner-emission ACs GREEN; legacy strings gone.

### Step 9: Update `docs/02_ir_schemas.md` and `docs/03_wit_and_manifest.md` per Doc Impact Statement

- Task IDs: `TASK-163b-diagnostic`
- Objective: land the documentation entries committed in `packet.spec.md` §Doc Impact.
- Precondition: Steps 2-8 complete; the implementation matches the documentation about to be written.
- Postcondition: both Doc Impact greps PASS.
- Files allowed to read:
  - `docs/02_ir_schemas.md` — locate the appropriate section to insert near (delegate LOCATIONS if > 300 lines).
  - `docs/03_wit_and_manifest.md` — locate the appropriate section (delegate LOCATIONS).
- Files allowed to edit (≤ 3):
  - `docs/02_ir_schemas.md`
  - `docs/03_wit_and_manifest.md`
- Files explicitly out-of-bounds for this step:
  - Other doc files — not impacted.
- Expected sub-agent dispatches:
  - "Locate the existing 'IR 9' / 'IR 9b' / 'IR 9c' sections in `docs/02_ir_schemas.md`; return LOCATIONS for the closing line of the last 'IR' section." — purpose: find insertion point.
  - "Locate the world-prepass section in `docs/03_wit_and_manifest.md`; return LOCATIONS ≤ 5 entries." — purpose: find insertion point.
- Context cost: `S`
- Authoritative docs: none additional.
- OrcaSlicer refs: none.
- Verification:
  - `rg -q 'record diagnostic' docs/02_ir_schemas.md` FACT pass.
  - `rg -q 'severity-level' docs/02_ir_schemas.md` FACT pass.
  - `rg -q 'record diagnostic' docs/03_wit_and_manifest.md` FACT pass.
- Exit condition: Doc Impact Statement satisfied.

### Step 10: Final packet verification + close

- Task IDs: `TASK-253`, `TASK-163b-diagnostic`
- Objective: re-dispatch all AC commands; gate workspace lint; confirm closure.
- Precondition: Steps 1-9 complete.
- Postcondition: all ACs PASS; workspace lint clean.
- Files allowed to read: none beyond prior steps.
- Files allowed to edit (≤ 3): none — verification only.
- Files explicitly out-of-bounds for this step:
  - `target/**`
- Expected sub-agent dispatches:
  - "Run AC-1 through AC-7 + AC-N1 + AC-N2 commands sequentially; return FACT (PASS / FAIL list)." — packet-level gate.
  - "Run `cargo clippy --workspace --all-targets -- -D warnings`; return FACT pass/fail; on fail SNIPPETS ≤ 20 lines with FIRST error." — lint gate.
- Context cost: `S`
- Authoritative docs: none additional.
- OrcaSlicer refs: none.
- Verification:
  - Full AC matrix FACT all PASS.
  - Workspace clippy FACT pass.
- Exit condition: closure summary recorded; `packet.spec.md` ready for `status: implemented`.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Discovery + delegated SUMMARY + LOCATIONS. |
| Step 2 | M | WIT change + 20-guest rebuild. |
| Step 3 | S | SDK struct + impl. |
| Step 4 | M | Host audit field + drain + `on_print_start` decision. |
| Step 5 | S | Roundtrip test as RED. |
| Step 6 | S | Close gap if any. |
| Step 7 | M | Four planner emission tests as RED. |
| Step 8 | M | Three call-site migrations + cap counter + guest rebuild. |
| Step 9 | S | Two doc edits. |
| Step 10 | S | Verification gate. |

Aggregate: `M`. No step is L.

## Packet Completion Gate

- All ten steps complete; each exit condition met.
- AC-1 through AC-7 + AC-N1 + AC-N2 all PASS.
- `docs/02_ir_schemas.md` and `docs/03_wit_and_manifest.md` updated; their greps PASS.
- `docs/07_implementation_status.md` marks `TASK-253` and `TASK-163b-diagnostic` `[x]` (via worker dispatch).
- `cargo xtask build-guests --check` `up to date`.
- `packet.spec.md` ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC command from `packet.spec.md`.
- Confirm packet-level gate commands green: `cargo xtask build-guests --check`, `cargo build --workspace`, `cargo test -p slicer-runtime --test wit_drift_detection_tdd`, `cargo test -p slicer-runtime --test prepass_diagnostic_roundtrip_tdd`, `cargo test -p slicer-runtime --test support_planner_diagnostic_emission_tdd`, `cargo clippy --workspace --all-targets -- -D warnings`.
- Confirm implementer's peak context usage stayed under 70%.
- Mark `TASK-253` and `TASK-163b-diagnostic` `[x]`; transition `packet.spec.md` to `status: implemented`.
