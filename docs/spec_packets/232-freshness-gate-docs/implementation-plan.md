# Implementation Plan: 232-freshness-gate-docs

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation. For a documentation packet the "test" is the AC grep: run it **before** the edit to confirm it currently reports FAIL, then after to confirm PASS. A grep that passes before the edit is a broken AC, not a finished step.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Rewrite `CLAUDE.md` §"Guest WASM Staleness (MUST follow)"

- Task IDs: `TASK-343`
- Objective: replace the mtime framing, the hand-maintained input-path list, and the 2026-07-25 anecdote's live-instruction status with the artifact-verified, exit-code model, keeping the section's MUST-follow enforcement tone.
- Precondition: packets 230 and 231 are `status: implemented`; AC-1, AC-2 and AC-3's greps report their pre-edit state (AC-1 FAIL, AC-2 FAIL).
- Postcondition: AC-1, AC-2 and AC-3 all report PASS.
- Files allowed to read, with ranges when over 300 lines:
  - `CLAUDE.md` — read only the "## Guest WASM Staleness (MUST follow)" section, which ends at the next `##` heading ("## WIT/Type Changes Checklist").
  - `docs/specs/guest-freshness-artifact-verification-plan.md` — moderate; read locked decisions C1, C2, C9 and C11 as amended, plus Round 5 finding R5-3.
- Files allowed to edit (at most 3):
  - `CLAUDE.md`
- Files explicitly out of bounds:
  - `docs/03_wit_and_manifest.md`, `docs/05_module_sdk.md` — later steps.
  - All of `crates/`, all of `xtask/src/` except `wit_verify.rs` (Step 6), every other `docs/spec_packets/` directory.
- Blast-radius discipline: not applicable; no struct field and no schema/version constant is added anywhere in this packet.
- Expected sub-agent dispatches:
  - Question: what exit-code constants and reporting behaviour did packet 230 ship in `check_command` — names, values, and what is printed in each case? scope: `xtask/src/build_guests.rs`; return: `FACT` (<=5 lines).
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/guest-freshness-artifact-verification-plan.md` — C1, C2, C9, C11, R5-3; direct read.
  - `CLAUDE.md` — the section being edited; direct read.
- OrcaSlicer refs:
  - None. No parity surface in this packet.
- Verification:
  - `rg -q 'You MUST run .--check. \(and rebuild if stale\) after editing any of these paths' CLAUDE.md && echo FAIL || echo PASS` — FACT PASS/FAIL (AC-1).
  - `rg -q 'exit code' CLAUDE.md && rg -q 'Artifact-verified freshness' CLAUDE.md && echo PASS || echo FAIL` — FACT PASS/FAIL (AC-2).
  - `if rg -q 'shared_crates' CLAUDE.md; then rg -q 'historical|Historical|no longer|superseded' CLAUDE.md && echo PASS || echo FAIL; else echo PASS; fi` — FACT PASS/FAIL (AC-3).
- Exit condition: all three greps PASS, and the rewritten section still forbids deflecting a guest failure as "unrelated" without a clean `--check`. Falsified if AC-2's `Artifact-verified freshness` reference is added before Step 4 defines the term in `CONTEXT.md` — in that case reorder so the glossary term lands first, and record the reorder.

### Step 2: Reconcile `docs/03_wit_and_manifest.md`'s two gates

- Task IDs: `TASK-343`
- Objective: drop "(mtime-based)" from the staleness-guard table row; restate the "### Build & Freshness Contract (Normative)" `--check` bullet as artifact verification with an exit-code answer; and rewrite the paragraph that introduces the host-side gate so the doc presents **two independent gates** with each rule attributed to its owner, naming `build_script_check_mode_reports_freshness` there as a test that currently asserts nothing (AC-7).
- Precondition: Step 1 complete; AC-4, AC-5 and AC-6 report their pre-edit state.
- Postcondition: AC-4, AC-5, AC-6 and AC-7 report PASS.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/03_wit_and_manifest.md` — very long; locate with `rg -n 'mtime-based|Build & Freshness Contract'` and read only two +/-40-line windows.
  - `crates/slicer-runtime/tests/contract/guest_fixture_freshness_tdd.rs` — short; read `GUESTS`, `guest_components_are_not_stale`, `build_script_check_mode_reports_freshness`. **Read-only**; this packet never edits it.
- Files allowed to edit (at most 3):
  - `docs/03_wit_and_manifest.md`
- Files explicitly out of bounds:
  - `crates/slicer-runtime/tests/contract/guest_fixture_freshness_tdd.rs` — documented, never modified (AC-N2).
  - `CLAUDE.md` (done), `docs/05_module_sdk.md` (Step 3), every `docs/spec_packets/` directory other than this one.
- Blast-radius discipline: not applicable.
- Expected sub-agent dispatches:
  - Question: how many entries does `GUESTS` in `crates/slicer-runtime/tests/contract/guest_fixture_freshness_tdd.rs` have, and what staleness rule does `guest_components_are_not_stale` apply? scope: that file; return: `FACT` (<=5 lines). The count goes into the doc from this dispatch, not from the packet.
- Context cost: `M`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` — the two sections being edited; ranged read.
  - `docs/specs/guest-freshness-artifact-verification-plan.md` — Round 5 finding R5-12; direct read.
- OrcaSlicer refs:
  - None.
- Verification:
  - `rg -q 'mtime-based' docs/03_wit_and_manifest.md && echo FAIL || echo PASS` — FACT PASS/FAIL (AC-4).
  - `rg -q 'if any source is newer than its artifact' docs/03_wit_and_manifest.md && echo FAIL || echo PASS` — FACT PASS/FAIL (AC-5).
  - `rg -q 'guest_fixture_freshness_tdd' docs/03_wit_and_manifest.md && rg -q 'two independent' docs/03_wit_and_manifest.md && echo PASS || echo FAIL` — FACT PASS/FAIL (AC-6).
  - `rg -q 'build_script_check_mode_reports_freshness' docs/03_wit_and_manifest.md && rg -q '^## Reported, not fixed' docs/spec_packets/232-freshness-gate-docs/requirements.md && echo PASS || echo FAIL` — FACT PASS/FAIL (AC-7).
- Exit condition: four greps PASS and the guest count written into the doc equals the `FACT` dispatch's count. Falsified if the doc states "10 test guests" — the tree said 8 on 2026-08-19; write what the dispatch returns.

### Step 3: `docs/05_module_sdk.md` bullet and `docs/07` symbol repins

- Task IDs: `TASK-343`
- Objective: correct the "**Guest rebuild obligation.**" bullet's "canonical pre-test gate" claim, and repin the two fabricated symbols in `docs/07_implementation_status.md`'s TASK-146b row.
- Precondition: Steps 1-2 complete; AC-8, AC-9, AC-10 report their pre-edit state (all FAIL).
- Postcondition: AC-8, AC-9, AC-10 report PASS.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/05_module_sdk.md` — very long; locate with `rg -n 'Guest rebuild obligation'` and read one +/-40-line window.
  - `docs/07_implementation_status.md` — long, with one very long row; **do not read**. Delegate the two string replacements.
- Files allowed to edit (at most 3):
  - `docs/05_module_sdk.md`
  - `docs/07_implementation_status.md` (through the dispatch below)
- Files explicitly out of bounds:
  - `docs/adr/**` — Step 4.
  - `CONTEXT.md`, `.claude/skills/**`, `.github/workflows/ci.yml`, `xtask/**`.
- Blast-radius discipline: not applicable.
- Expected sub-agent dispatches:
  - Question: in `docs/07_implementation_status.md`, replace `stage_wit_mtime(ws_root, stage_id)` with the real symbol `stage_wit_snapshot`, noting it was retired by packet 231, and replace `compute_shared_mtime` with `compute_shared_freshness`, likewise noting retirement. Do not otherwise reflow the row. scope: `docs/07_implementation_status.md`; return: `FACT pass/fail` plus the two confirming greps.
- Context cost: `S`
- Authoritative docs:
  - `CLAUDE.md` §"In-Tree Citation Style (MUST follow)" — direct read; this step is the citation-style repair.
  - `CLAUDE.md` §"Test Discipline", subsection "`cargo xtask test` — the gated entry point" — direct read; it defines the entry point that `docs/05` must now name.
- OrcaSlicer refs:
  - None.
- Verification:
  - `rg -q 'is the canonical pre-test gate' docs/05_module_sdk.md && echo FAIL || echo PASS` — FACT PASS/FAIL (AC-8).
  - `if rg -q 'stage_wit_mtime|compute_shared_mtime' docs/07_implementation_status.md; then echo FAIL; else echo PASS; fi` — FACT PASS/FAIL (AC-9, AC-10).
- Exit condition: both greps PASS and the TASK-146b row is otherwise byte-identical apart from the two repins plus their retirement notes. Falsified if the dispatch rewrites or truncates the row.

### Step 4: ADR-0014 and ADR-0045 amendments

- Task IDs: `TASK-343`
- Objective: add a nested `###` amendment entry under ADR-0014's existing `## Amendments` heading and correct its `slicer-core` Consequences claim; repin ADR-0045's three `compute_shared_mtime` occurrences and add a `## Amendment — <YYYY-MM-DD> (packets 229/230/231)` section in that file's house style.
- Precondition: Step 3 complete; AC-11 and AC-12 report their pre-edit state.
- Postcondition: AC-11 and AC-12 report PASS; `docs/adr/0054-host-side-test-support-crate.md` is untouched.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/adr/0014-xtask-guest-discovery-via-validated-filesystem-walk.md` — short; full read.
  - `docs/adr/0045-per-stage-versioned-interfaces-over-monolithic-tier-worlds.md` — long; read only the windows around the three `compute_shared_mtime` occurrences and around the existing `## Amendment — 2026-08-05 (packets 163/164)` heading (for house style).
- Files allowed to edit (at most 3):
  - `docs/adr/0014-xtask-guest-discovery-via-validated-filesystem-walk.md`
  - `docs/adr/0045-per-stage-versioned-interfaces-over-monolithic-tier-worlds.md`
- Files explicitly out of bounds:
  - `docs/adr/0054-host-side-test-support-crate.md` — packet 231 owns it; AC-N3 asserts it is unchanged.
  - Every other file under `docs/adr/`.
  - `CONTEXT.md`, `.claude/skills/**`, `.github/workflows/ci.yml`.
- Blast-radius discipline: not applicable. No new ADR slot is allocated: both amendments are added to existing ADRs, so no `docs/adr/NNNN-` number is consumed.
- Expected sub-agent dispatches:
  - Question: does any file under `docs/` other than ADR-0045 and `docs/07_implementation_status.md` contain the string `compute_shared_mtime` or `stage_wit_mtime`? scope: `docs/**`; return: `LOCATIONS` (<=20 entries). If any is found outside this packet's file list, report it rather than editing outside scope.
- Context cost: `S`
- Authoritative docs:
  - `docs/adr/0014-...md` — full read; its `## Amendments` structure is the template for the new entry.
  - `docs/adr/0045-...md` — ranged read; `## Amendment — 2026-08-05 (packets 163/164)` fixes the heading style.
- OrcaSlicer refs:
  - None.
- Verification:
  - `if rg -q 'compute_shared_mtime' docs/adr/0045-per-stage-versioned-interfaces-over-monolithic-tier-worlds.md; then echo FAIL; else rg -q '^## Amendment — 20[0-9][0-9]-[0-9][0-9]-[0-9][0-9] \(packets 229/230/231\)' docs/adr/0045-per-stage-versioned-interfaces-over-monolithic-tier-worlds.md && echo PASS || echo FAIL; fi` — FACT PASS/FAIL (AC-11).
  - `rg -q 'dependency closure' docs/adr/0014-xtask-guest-discovery-via-validated-filesystem-walk.md && rg -q 'shared_crates' docs/adr/0014-xtask-guest-discovery-via-validated-filesystem-walk.md && echo PASS || echo FAIL` — FACT PASS/FAIL (AC-12).
  - `git diff --name-only $(git merge-base HEAD master) | rg '^docs/adr/0054-' && echo FAIL || echo PASS` — FACT PASS/FAIL (AC-N3).
- Exit condition: AC-11, AC-12 and AC-N3 all PASS, and the new ADR-0045 heading matches the file's existing amendment-heading style character for character apart from date and packet numbers. Falsified if a new ADR file is created — this step amends, never authors.

### Step 5: `CONTEXT.md` term, `wasm-staleness` snippet, `spec-review` bullet

- Task IDs: `TASK-343`
- Objective: define `### Artifact-verified freshness` in the glossary, rewrite the canonical snippet to an exit-code contract with its marker intact, and correct the reviewer trap bullet.
- Precondition: Step 4 complete; AC-13, AC-14, AC-15 report their pre-edit state.
- Postcondition: AC-13, AC-14 and AC-15 report PASS.
- Files allowed to read, with ranges when over 300 lines:
  - `CONTEXT.md` — long; read only lines 1-10 to confirm the `# Context Glossary` / `## Terms` / `### <Term>` structure, then append.
  - `.claude/skills/spec-packet-generator/references/snippets/wasm-staleness.md` — short; full read.
  - `.claude/skills/spec-review/SKILL.md` — read only the "## Known traps" list.
  - `docs/specs/guest-freshness-artifact-verification-plan.md` — moderate; read "## CONTEXT.md term (packet 4 owns the wording)" for the verbatim term text and R5-3 for the snippet rationale.
- Files allowed to edit (at most 3):
  - `CONTEXT.md`
  - `.claude/skills/spec-packet-generator/references/snippets/wasm-staleness.md`
  - `.claude/skills/spec-review/SKILL.md`
- Files explicitly out of bounds:
  - `.claude/skills/spec-packet-generator/SKILL.md` and every other file under `.claude/skills/` — the snippet's own body is the contract; the skill's one-line reference to it needs no change.
  - Every `docs/spec_packets/` directory other than this one — in particular the six carrying grep-form ACs, which stay as they are.
  - `.github/workflows/ci.yml`, `xtask/**`.
- Blast-radius discipline: the snippet is copied **verbatim** into future packets' `design.md` files. Its blast radius is forward-looking, not in-tree: existing copies in already-written packets are frozen by user ruling and must not be updated. Do not sweep the tree for old copies.
- Expected sub-agent dispatches:
  - Question: which files under `docs/spec_packets/` currently contain the `<!-- snippet: wasm-staleness -->` marker? scope: `docs/spec_packets/**`; return: `FACT` with the count only. Purpose: confirm the frozen-copy population is known and deliberately left alone; do **not** edit any of them.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/guest-freshness-artifact-verification-plan.md` §"CONTEXT.md term (packet 4 owns the wording)" — the term's wording is fixed there; direct read.
  - `.claude/skills/spec-packet-generator/SKILL.md` §"Packet Ownership" — read the snippet rules (verbatim-or-absent, marker retained) before rewriting the snippet body.
- OrcaSlicer refs:
  - None.
- Verification:
  - `rg -q '^### Artifact-verified freshness' CONTEXT.md && rg -q 'embedded WIT world' CONTEXT.md && echo PASS || echo FAIL` — FACT PASS/FAIL (AC-15).
  - `rg -q '<!-- snippet: wasm-staleness -->' .claude/skills/spec-packet-generator/references/snippets/wasm-staleness.md && rg -q 'exit' .claude/skills/spec-packet-generator/references/snippets/wasm-staleness.md && echo PASS || echo FAIL` — FACT PASS/FAIL (AC-13).
  - `rg -q 'if .STALE:., rebuild and re-run' .claude/skills/spec-review/SKILL.md && echo FAIL || echo PASS` — FACT PASS/FAIL (AC-14).
- Exit condition: three greps PASS, the snippet still opens its copy-block with the unchanged marker, and no file under `docs/spec_packets/` other than this packet's own directory was touched. Falsified if the snippet's rewrite drops the applies-to list instead of updating it (see `design.md` §Open Questions, fourth `[FWD]`).

### Step 6: CI wiring and non-vacuous verifier tests

- Task IDs: `TASK-343`
- Objective: add `cargo test -p xtask` to `.github/workflows/ci.yml`'s `test` job after the `Install wasm-tools` step; and, only if a skip-on-present path survived packets 229/230, convert it to a hard failure in `xtask/src/wit_verify.rs`.
- Precondition: Step 5 complete; AC-16 reports FAIL (no `-p xtask` in CI).
- Postcondition: AC-16 and AC-17 report PASS.
- Files allowed to read, with ranges when over 300 lines:
  - `.github/workflows/ci.yml` — short; read the `test` job only.
  - `xtask/src/wit_verify.rs` — read only the `#[cfg(test)] mod tests` block's skip-guards.
- Files allowed to edit (at most 3):
  - `.github/workflows/ci.yml`
  - `xtask/src/wit_verify.rs` (conditional; may end as a verified no-op)
- Files explicitly out of bounds:
  - `xtask/src/build_guests.rs`, `xtask/src/test.rs`, `xtask/src/main.rs`, `xtask/src/dist.rs` — AC-N2 permits only `wit_verify.rs`.
  - Everything under `crates/`.
  - All doc files finished in Steps 1-5.
- Blast-radius discipline: not applicable; no struct field or version constant.
- Expected sub-agent dispatches:
  - Question: after packets 229/230, does any test in `xtask/src/wit_verify.rs` print `skipping` and return early on a path reachable when `wasm-tools` resolves on `PATH` and the artifact exists? scope: `xtask/src/wit_verify.rs`; return: `LOCATIONS` (<=20 entries).
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/guest-freshness-artifact-verification-plan.md` — Round 5 finding R5-10 and the "CI is safe for C9" note confirming `taiki-e/install-action` installs `wasm-tools` in both the `test` and `dist-editions` jobs; direct read.
- OrcaSlicer refs:
  - None.
- Verification:
  - `mkdir -p target && awk '/^  test:/{f=1;next} f && /^  [a-z][a-z0-9-]*:/{f=0} f' .github/workflows/ci.yml > target/ci-test-job.txt && rg -q 'cargo test -p xtask' target/ci-test-job.txt && rg -n 'tool: wasm-tools|cargo test -p xtask' target/ci-test-job.txt | head -1 | rg -q 'wasm-tools' && echo PASS || echo FAIL` — FACT PASS/FAIL (AC-16; the awk slice restricts the assertion to the `test` job, so a step added to `dist-editions` cannot satisfy it).
  - `if rg -q 'skipping' xtask/src/wit_verify.rs; then echo FAIL; else mkdir -p target && cargo test -p xtask wit_verify 2>&1 | tee target/test-output.log | rg -q '^test result: ok\.' && echo PASS || echo FAIL; fi` — FACT PASS/FAIL (AC-17).
  - `git diff --name-only $(git merge-base HEAD master) | rg '^(crates/|xtask/src/)' | rg -v '^xtask/src/wit_verify\.rs$' | rg . && echo FAIL || echo PASS` — FACT PASS/FAIL (AC-N2).
- Exit condition: AC-16, AC-17 and AC-N2 PASS. If the `LOCATIONS` dispatch returns zero skip-on-present paths, record "verified no-op: packet 229 left `xtask/src/wit_verify.rs` free of skip-on-present early returns" as the exit note and leave the file unedited — do not manufacture an edit. Falsified if AC-17 is reported PASS from a run in which the real-artifact tests silently skipped.

### Step 7: Ledger row, cross-file sweep, closure gates

- Task IDs: `TASK-343`
- Objective: add the `TASK-343` row to `docs/07_implementation_status.md`, run the cross-file deleted-symbol sweep, and run the packet's closure gates.
- Precondition: Steps 1-6 complete; every per-step AC grep PASSes.
- Postcondition: AC-18, AC-N1 and AC-N4 report PASS; both workspace gates green.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/07_implementation_status.md` — long; **do not read**. Dispatch the append.
- Files allowed to edit (at most 3):
  - `docs/07_implementation_status.md` (through the dispatch below)
- Files explicitly out of bounds:
  - Every file edited in Steps 1-6 — reopening one means that step's exit condition was wrong.
  - Every `docs/spec_packets/` directory other than this one.
- Blast-radius discipline: not applicable.
- Expected sub-agent dispatches:
  - Question: re-derive the highest `TASK-###` present in `docs/07_implementation_status.md`; if `TASK-343` is already taken, report the next free ID instead of writing. Then append one row under `### Workstream 5 — Governance and closure drift` in that section's terser local style (a status box, the bare ID, then a verb phrase, no em-dash after the ID — compare `- [~] TASK-139 Close the DEV-020 source/docs drift ...`), describing the artifact-verified freshness documentation pass, the ADR-0014/ADR-0045 amendments, the exit-code snippet rewrite, and the `cargo test -p xtask` CI wiring. Confirm with a grep. scope: `docs/07_implementation_status.md`; return: `FACT pass/fail` plus the appended line.
- Context cost: `S`
- Authoritative docs:
  - `CLAUDE.md` §"Ledger Facts Must Be Re-derived, Not Quoted (MUST follow)" — direct read; `TASK-343` is a ledger fact and must be re-derived here, not taken from this plan.
- OrcaSlicer refs:
  - None.
- Verification:
  - `rg -q '^- \[.\] TASK-343 ' docs/07_implementation_status.md && echo PASS || echo FAIL` — FACT PASS/FAIL (AC-18).
  - `if rg -q 'compute_shared_mtime|stage_wit_mtime|shared_input_paths' CLAUDE.md docs/03_wit_and_manifest.md docs/05_module_sdk.md docs/07_implementation_status.md CONTEXT.md docs/adr/0014-xtask-guest-discovery-via-validated-filesystem-walk.md docs/adr/0045-per-stage-versioned-interfaces-over-monolithic-tier-worlds.md; then echo FAIL; else echo PASS; fi` — FACT PASS/FAIL (AC-N4).
  - `git diff --name-only $(git merge-base HEAD master) | rg '^docs/spec_packets/' | rg -v '^docs/spec_packets/232-freshness-gate-docs/' | rg . && echo FAIL || echo PASS` — FACT PASS/FAIL (AC-N1).
  - `cargo check --workspace --all-targets` — FACT pass/fail.
  - `cargo clippy --workspace --all-targets -- -D warnings` — FACT pass/fail.
  - `cargo xtask build-guests --check; echo "exit=$?"` — FACT: record the exit code, as the rewritten snippet now demands. Never grep for `STALE:`.
- Exit condition: AC-18, AC-N1, AC-N4 PASS; both workspace gates green; the `--check` exit code recorded. Falsified if the ID written differs from the re-derived free ID without that renumbering being reported.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | One `CLAUDE.md` section; one FACT dispatch for the exit codes |
| Step 2 | M | Two anchors in a 1706-line doc plus a two-gates paragraph grounded in a second file |
| Step 3 | S | One `docs/05` sentence; two delegated `docs/07` repins |
| Step 4 | S | Two ADR amendments; one small file read whole, one ranged |
| Step 5 | S | Glossary term, snippet rewrite, one skill bullet |
| Step 6 | S | One CI step; one conditional test-guard edit |
| Step 7 | S | Delegated ledger append, cross-file sweep, workspace gates |

Aggregate: `M`. No step is L; no split is required before activation.

## Packet Completion Gate

- All seven steps and their exit conditions complete.
- Every pipe-suffixed AC command in `packet.spec.md` returns PASS, re-run fresh rather than quoted from the step that first passed it.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- No reopened or superseded packet status transitions apply.
- Confirm packets 229, 230 and 231 are `status: implemented` before flipping this packet's status; this packet documents their shipped behaviour and is meaningless ahead of them.
- Confirm `docs/specs/guest-freshness-artifact-verification-plan.md`'s Packet Queue rows for `229`-`232` are updated, and that the plan file and all four packet directories are committed together per the plan's commit rule.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC command and the three packet-level gate commands.
- Record `cargo xtask build-guests --check`'s exit code as evidence; quote no timing figure that was not measured in the session, including any CI-duration claim for the added `cargo test -p xtask` step.
- Record remaining packet-local risk: doc greps verify text and not truth; the six frozen packets keep unsound grep-form ACs by user ruling; `AC-16`'s ordering check is positional and would need revisiting if the CI jobs are reordered; `build_script_check_mode_reports_freshness` remains vacuous and is reported, not fixed.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` where the subcommand supports it, so the test, bench, and example targets compile.
