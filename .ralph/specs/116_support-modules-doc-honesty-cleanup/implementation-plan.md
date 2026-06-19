# Implementation Plan: support-modules-doc-honesty-cleanup

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs (`TASK-250` doc-comment, `TASK-251` field cleanup + warning, `TASK-252` BASE_SPEED note).
- TDD first for the B2 warning behavior (Step 3 lands the negative tests as RED before Step 4 deletes the field and lands the warning code that turns them GREEN).
- Each step honors the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. The fields below are not optional metadata — they are the budget contract for this step.

## Steps

### Step 1: Read source spec and confirm replacement targets

- Task IDs: `TASK-250`, `TASK-251`, `TASK-252`
- Objective: confirm the exact replacement doc-comment text from §B1, the exact warning string from §B2, the exact `# Speed normalization` section template from §B3, and the current `//!` block line ranges in the four target files.
- Precondition: spec doc `docs/specs/support-modules-orca-port.md` available at HEAD.
- Postcondition: implementer has a single mental note of the three replacement strings, the field-delete site, and the line ranges of each `//!` block.
- Files allowed to read (with line-range hints):
  - `docs/specs/support-modules-orca-port.md` — §B1 (≈30 lines), §B2 (≈20 lines), §B3 (≈10 lines), §D8 (≈10 lines), §D9 (≈5 lines)
  - `modules/core-modules/tree-support/src/lib.rs` — lines 1-12
  - `modules/core-modules/traditional-support/src/lib.rs` — lines 1-16
  - `modules/core-modules/support-planner/src/lib.rs` — lines 1-35 (doc-comment block), 70-80 (struct field), 150-180 (parse block)
  - `modules/core-modules/rectilinear-infill/src/lib.rs` — lines 1-20
- Files allowed to edit (≤ 3): none in this step.
- Files explicitly out-of-bounds for this step:
  - `OrcaSlicerDocumented/**` — not consulted
  - `crates/**` — out of scope
- Expected sub-agent dispatches: none.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-modules-orca-port.md` — read §B1-B3, §D8-D9 directly (under 100 lines combined)
- OrcaSlicer refs: none.
- Verification:
  - The implementer can recite (a) the first sentence of each new module doc-comment, (b) the exact warning message string, (c) the exact `# Speed normalization` paragraph.
- Exit condition: discovery notes captured (≤ 10 bullets); next step ready to write.

### Step 2: Rewrite lead doc-comments in tree-support, traditional-support, support-planner

- Task IDs: `TASK-250`
- Objective: replace the lead `//!` block of all three support modules with the §B1 text, preserving the existing `#![warn(...)]` directives immediately below.
- Precondition: Step 1 complete; replacement text confirmed.
- Postcondition: AC-1, AC-2, AC-3 grep substrings present in their respective files.
- Files allowed to read: same as Step 1.
- Files allowed to edit (≤ 3):
  - `modules/core-modules/tree-support/src/lib.rs`
  - `modules/core-modules/traditional-support/src/lib.rs`
  - `modules/core-modules/support-planner/src/lib.rs`
- Files explicitly out-of-bounds for this step:
  - `modules/core-modules/rectilinear-infill/src/lib.rs` — handled in Step 4
  - the `.toml` file — handled in Step 5
- Expected sub-agent dispatches:
  - "Run `cargo build -p tree-support -p traditional-support -p support-planner`; return FACT pass/fail" — purpose: confirm doc-comment edits don't break compile (rare but possible if a `//!` directive is misformatted).
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-modules-orca-port.md` §B1 — text source
- OrcaSlicer refs: none.
- Verification:
  - `rg -q 'Per-layer 2-D grid-MST infill with optional SupportPlanIR consumption' modules/core-modules/tree-support/src/lib.rs` — FACT pass
  - `rg -q 'Per-layer rectilinear scan-line filler for Layer::Support' modules/core-modules/traditional-support/src/lib.rs` — FACT pass
  - `rg -q 'Multi-layer support planner inspired by OrcaSlicer' modules/core-modules/support-planner/src/lib.rs` — FACT pass
- Exit condition: three grep verifications return PASS; `cargo build` of the three crates PASS.

### Step 3: Author negative tests for AC-5, AC-N1, AC-N2 as RED

- Task IDs: `TASK-251`
- Objective: create `modules/core-modules/support-planner/tests/interface_bottom_layers_warning_tdd.rs` with three test functions exercising the new `LogLevel::Warn` path. Tests target the function names referenced in AC-5, AC-N1, AC-N2 (`on_print_start_not_implemented_warning`, `on_print_start_no_warning_at_default`, `on_print_start_no_warning_when_absent`). The tests MUST fail (RED) before Step 4 lands the implementation that turns them green.
- Precondition: Step 2 complete; doc-comment edits don't affect the parse block.
- Postcondition: file exists; tests compile; tests fail (RED) because the field is still present and the warning is not yet emitted.
- Files allowed to read:
  - `modules/core-modules/support-planner/src/lib.rs` — current `on_print_start` (lines ≈150-185)
  - `crates/slicer-sdk/src/host.rs` — `log` fn signature + nearest existing test-support helper (delegate the second part as a small SUMMARY if needed)
  - `crates/slicer-sdk/src/test_support/fixtures.rs` — confirm the existing `log_test_support` interface used by the existing `orca_parity_tdd.rs` tests
- Files allowed to edit (≤ 3):
  - `modules/core-modules/support-planner/tests/interface_bottom_layers_warning_tdd.rs` (new file)
- Files explicitly out-of-bounds for this step:
  - `modules/core-modules/support-planner/src/lib.rs` — do not edit here; Step 4 owns the field deletion and warning emission
- Expected sub-agent dispatches:
  - "Find the existing pattern in `support-planner/tests/` for asserting `LogLevel::Warn` messages emitted by `on_print_start`; return SNIPPETS ≤ 30 lines showing the test-support helper used (likely `slicer_sdk::host::test_support`)" — purpose: write the new tests in the existing idiom rather than inventing one.
  - "Run `cargo test -p support-planner --test interface_bottom_layers_warning_tdd`; return FACT (expected: all three fail)" — confirm RED state.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-modules-orca-port.md` §B2 — warning message text
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p support-planner --test interface_bottom_layers_warning_tdd` — FACT three failures (RED).
- Exit condition: three tests compile and report assertion failures (not compile errors).

### Step 4: Delete `support_interface_bottom_layers` field; replace parse with warning emission

- Task IDs: `TASK-251`
- Objective: turn Step 3's tests from RED to GREEN by deleting the struct field, deleting the parse-and-store block, and replacing it with the conditional `LogLevel::Warn` emission specified in §B2.
- Precondition: Step 3 tests are RED.
- Postcondition: AC-4 grep returns zero matches; AC-5, AC-N1, AC-N2 tests are GREEN.
- Files allowed to read:
  - `modules/core-modules/support-planner/src/lib.rs` — current implementation
- Files allowed to edit (≤ 3):
  - `modules/core-modules/support-planner/src/lib.rs`
- Files explicitly out-of-bounds for this step:
  - the `.toml` file — handled in Step 5
- Expected sub-agent dispatches:
  - "Run `cargo build -p support-planner`; return FACT pass/fail" — purpose: confirm field removal didn't leave dangling references.
  - "Run `cargo test -p support-planner --test interface_bottom_layers_warning_tdd`; return FACT pass/fail; SNIPPETS ≤ 20 lines on failure" — purpose: gate RED→GREEN transition.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-modules-orca-port.md` §B2 — exact warning string
- OrcaSlicer refs: none.
- Verification:
  - `! rg -q 'support_interface_bottom_layers' modules/core-modules/support-planner/src/lib.rs` — FACT pass (no field/parse remains in Rust)
  - `cargo test -p support-planner --test interface_bottom_layers_warning_tdd` — FACT pass (all three GREEN)
  - `cargo test -p support-planner` — FACT pass (existing tests unaffected)
- Exit condition: AC-4 verification PASS; new test file PASS; full `-p support-planner` test suite PASS.

### Step 5: Add TOML comment + rectilinear-infill BASE_SPEED note + the three other modules' `# Speed normalization` sections

- Task IDs: `TASK-251` (TOML comment), `TASK-252` (BASE_SPEED notes)
- Objective: add the `# Not yet implemented — see docs/specs/support-modules-orca-port.md §B2` comment to `support-planner.toml` next to the `support_interface_bottom_layers` schema; add the `# Speed normalization` section to the lead `//!` blocks of `tree-support`, `traditional-support`, `support-planner`, `rectilinear-infill`.
- Precondition: Steps 2 and 4 complete (doc-comment blocks already opened/touched).
- Postcondition: AC-6 grep matches in all four module files; the TOML comment is present.
- Files allowed to read:
  - `modules/core-modules/support-planner/support-planner.toml` — confirm the schema entry's current line range
  - `modules/core-modules/rectilinear-infill/src/lib.rs` lines 1-20
- Files allowed to edit (≤ 3):
  - `modules/core-modules/support-planner/support-planner.toml`
  - `modules/core-modules/rectilinear-infill/src/lib.rs`
  - `modules/core-modules/{tree-support,traditional-support,support-planner}/src/lib.rs` — extension to the doc-comment block already opened in Step 2. **This step touches the same three files as Step 2 by extending the `//!` block; treat them as one combined surface to keep the 3-files-edit ceiling per step honest, but the implementer must consider this Step's edits the *trailing* section of the doc-comment region and not interleave them with Step 2's lead-sentence edits.**
- Files explicitly out-of-bounds for this step:
  - `modules/core-modules/gyroid-infill/`, `modules/core-modules/lightning-infill/` — `BASE_SPEED` consumers in these modules are deferred (see Out of Scope in requirements.md).
- Expected sub-agent dispatches:
  - "Run `for m in tree-support traditional-support support-planner rectilinear-infill; do rg -q '# Speed normalization' modules/core-modules/$m/src/lib.rs || { echo MISSING: $m; exit 1; }; done`; return FACT pass/fail" — purpose: gate AC-6.
  - "Run `cargo clippy -p tree-support -p traditional-support -p support-planner -p rectilinear-infill --all-targets -- -D warnings`; return FACT pass/fail; SNIPPETS ≤ 20 lines on failure" — purpose: doc-comment lints don't regress.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-modules-orca-port.md` §B3 — section text
- OrcaSlicer refs: none.
- Verification:
  - AC-6 grep loop FACT pass
  - `cargo clippy ... -- -D warnings` FACT pass
  - `rg -q '# Not yet implemented — see docs/specs/support-modules-orca-port.md §B2' modules/core-modules/support-planner/support-planner.toml` — FACT pass
- Exit condition: AC-6 PASS; clippy PASS; TOML comment present.

### Step 6: Guest WASM staleness gate + final packet verification

- Task IDs: `TASK-250`, `TASK-251`, `TASK-252`
- Objective: confirm guest `.wasm` artifacts are not stale after src/lib.rs edits; run the full packet verification matrix and produce a closure summary.
- Precondition: Steps 2-5 complete; every prior AC verification has reported PASS.
- Postcondition: every AC verification command in `packet.spec.md` returns PASS; `cargo xtask build-guests --check` reports `up to date` (or, after a rebuild, returns clean).
- Files allowed to read: none beyond prior steps.
- Files allowed to edit (≤ 3): none — this step is verification only.
- Files explicitly out-of-bounds for this step:
  - `target/**` — never load directly
- Expected sub-agent dispatches:
  - "Run `cargo xtask build-guests --check`; return FACT (`up to date` or `STALE: <which>`)" — if STALE, dispatch a follow-up "Run `cargo xtask build-guests`; return FACT pass/fail" then re-check.
  - "Run all six positive-AC verification commands and the two negative-case commands in sequence; return FACT (PASS / FAIL list)." — packet-level gate.
- Context cost: `S`
- Authoritative docs: none additional.
- OrcaSlicer refs: none.
- Verification:
  - `cargo xtask build-guests --check` — FACT `up to date`
  - Full packet AC matrix (AC-1 through AC-6, AC-N1, AC-N2) — FACT all PASS
- Exit condition: closure summary recorded; `packet.spec.md` ready to be flipped to `status: implemented` by the maintainer after they sign off on the acceptance ceremony.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Discovery only; spec read + line-range confirmation. |
| Step 2 | S | Three doc-comment rewrites. |
| Step 3 | S | RED tests authored; no src/lib.rs edit. |
| Step 4 | S | Field delete + warning emission; turns Step 3 GREEN. |
| Step 5 | S | TOML comment + four `# Speed normalization` sections. |
| Step 6 | S | Verification-only gate; no edits. |

Aggregate: `S`. No step is L; no step is M.

## Packet Completion Gate

- All six steps complete.
- Every step exit condition met.
- Every packet acceptance criterion command from `packet.spec.md` dispatched and returned PASS (AC-1 through AC-6, AC-N1, AC-N2).
- `docs/07_implementation_status.md` updated to mark `TASK-250`, `TASK-251`, `TASK-252` as `[x]` (via worker dispatch — never edit the full backlog into the implementer's context).
- `cargo xtask build-guests --check` returns `up to date`.
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md` (AC-1 through AC-6, AC-N1, AC-N2). Confirm FACT pass on every one.
- Confirm packet-level gate commands are green: `cargo build`, `cargo clippy`, `cargo test -p support-planner`, `cargo xtask build-guests --check`.
- Confirm the implementer's peak context usage stayed under 70%; log it as a packet-authoring lesson for future spec-packet-generator runs if not.
- No packet-local risk remains; mark `TASK-250`, `TASK-251`, `TASK-252` `[x]` in `docs/07_implementation_status.md` and transition `packet.spec.md` to `status: implemented`.
