# Requirements: 232-freshness-gate-docs

## Packet Metadata

- Grouped task IDs: `TASK-343`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

The guest-freshness contract is stated in nine places, and packets 229, 230 and 231 falsify every one of them. `CLAUDE.md`'s "## Guest WASM Staleness (MUST follow)" is the loudest: it tells every agent that freshness is mtime-based, lists by hand the paths that invalidate a guest, and closes with a 2026-07-25 anecdote about `slicer-core` having been missing from `shared_input_paths`' `shared_crates` array — an array packet 231 deletes. `docs/03_wit_and_manifest.md` states the same model twice, once as a table row reading `| `cargo xtask build-guests --check` | Stale in-tree guest (mtime-based) |` and once in the normative section "### Build & Freshness Contract (Normative)", whose `--check` bullet reads "verify only; exit 1 if any source is newer than its artifact". `docs/05_module_sdk.md` calls `--check` "the canonical pre-test gate", which `CLAUDE.md` elsewhere assigns to `cargo xtask test`.

Two docs cite symbols that never existed. `docs/07_implementation_status.md`'s TASK-146b row names `stage_wit_mtime(ws_root, stage_id)`; the real function was `stage_wit_snapshot`. ADR-0045 names `compute_shared_mtime` in one prose paragraph and two table cells; the real function was `compute_shared_freshness`. Both real functions are deleted by packet 231, so a repin must both correct the name and mark it retired. ADR-0014's Amendments section records packet 185's `shared_crates` rule, and its Consequences still assert "Touching `slicer-core` does not trigger a guest rebuild storm" — false since 2026-07-25 and true again, by a different mechanism, after packet 231.

Two authoring surfaces encode a verification form that Round 5 finding R5-3 proved unsound: `.claude/skills/spec-packet-generator/references/snippets/wasm-staleness.md` and `.claude/skills/spec-review/SKILL.md` both tell downstream agents to look for `STALE:`. A `wasm-tools`-missing infrastructure error prints no `STALE:` line, so `--check 2>&1 | rg -q 'STALE:' && echo FAIL || echo PASS` reports PASS on a check that never ran.

Finally, none of this is tested in CI. `.github/workflows/ci.yml`'s `test` job runs `cargo test -p slicer-runtime && cargo test -p pnp-cli && cargo test -p slicer-helpers` and never `-p xtask`, so every verifier test packets 229-231 add is dead in CI (R5-10).

This is one coherent slice because the nine statements are the same statement, and updating a subset leaves the tree self-contradictory in a way that is harder to detect than the current uniform staleness.

## In Scope

- **`CLAUDE.md` §"## Guest WASM Staleness (MUST follow)"** — rewrite to the artifact-verified model: `--check` decodes each guest artifact and compares its embedded WIT world against canonical; the fingerprint covers code inputs only, derived per guest from its Cargo path-dependency closure; freshness is read from the **exit code**, with a distinct non-zero code when `wasm-tools` is unavailable. Delete the hand-maintained "You MUST run `--check` … after editing any of these paths" list, which no longer has a referent. Keep the section's enforcement tone and the prohibition on deflecting a guest failure as "unrelated"; keep it a MUST-follow section.
- **The 2026-07-25 `slicer-core` anecdote** — remove, or reframe explicitly as history with the year stated and a note that the hand-maintained list no longer exists. It must not read as a live instruction.
- **`docs/03_wit_and_manifest.md`** — the staleness-guard table row (drop "(mtime-based)"), and "### Build & Freshness Contract (Normative)" (artifact-verified `--check`; exit-code contract). Reconcile the section's presentation of the second gate: state that there are **two independent gates**, the xtask artifact gate and the host-side contract test `crates/slicer-runtime/tests/contract/guest_fixture_freshness_tdd.rs`, and that the latter hardcodes its own guest list (`GUESTS`, 8 entries measured 2026-08-19), applies its own mtime rule (a guest's `src/lib.rs` newer than its artifact), is independent of xtask, and keeps working unchanged.
- **`docs/05_module_sdk.md`** — the "**Guest rebuild obligation.**" bullet: name `cargo xtask test` as the enforced pre-test entry point and state that `--check` reports freshness by exit code.
- **`docs/07_implementation_status.md`** — repin the TASK-146b row (`stage_wit_mtime` -> `stage_wit_snapshot`, retired; `compute_shared_mtime` -> `compute_shared_freshness`, retired), and add the `TASK-343` row under "### Workstream 5 — Governance and closure drift" in that section's terser local format.
- **`docs/adr/0014-...md`** — a new nested `###` entry under the existing `## Amendments` heading recording that packet 231 replaced `shared_crates` with a per-guest dependency closure; correct the Consequences claim about `slicer-core` so it is not asserted as current fact.
- **`docs/adr/0045-...md`** — repin all three `compute_shared_mtime` occurrences and add a `## Amendment — <YYYY-MM-DD> (packets 229/230/231)` section in the file's existing house style (compare `## Amendment — 2026-08-05 (packets 163/164)`), recording that per-stage WIT mtime charging is retired in favour of artifact verification.
- **`.claude/skills/spec-packet-generator/references/snippets/wasm-staleness.md`** — rewrite the copy-exactly block to an exit-code contract, preserving the `<!-- snippet: wasm-staleness -->` marker and the "copy exactly, do not paraphrase" framing. State that the grep form is forbidden and why.
- **`.claude/skills/spec-review/SKILL.md`** — "Known traps" item 4: judge by exit code, not by `STALE:`.
- **`CONTEXT.md`** — append `### Artifact-verified freshness` under `## Terms`, using the wording fixed by `docs/specs/guest-freshness-artifact-verification-plan.md` §"CONTEXT.md term (packet 4 owns the wording)".
- **`.github/workflows/ci.yml`** — add `cargo test -p xtask` to the `test` job, positioned after the `Install wasm-tools` step.
- **`xtask/src/wit_verify.rs`** — only if, after packets 229/230 land, any real-artifact test still short-circuits with a `skipping` message on a machine where `wasm-tools` and the artifact are both present: convert that path to a hard failure (R5-10). If packet 229 already left the file clean, this is a verified no-op and must be recorded as such.

## Out of Scope

- Any freshness logic: artifact decoding, the declaration model, stage resolution, the closure walk, fingerprint content or lifecycle, exit-code values. Packets 229, 230 and 231 own all of it. This packet describes; it does not decide.
- `crates/slicer-runtime/tests/contract/guest_fixture_freshness_tdd.rs` — the host-side gate is documented, not modified. Its vacuous `build_script_check_mode_reports_freshness` is reported below, not fixed.
- `docs/adr/0054-host-side-test-support-crate.md` — packet 231 discharges its Decision rule 5 obligation by conforming the `crates/pnp-cli-locator::staleness_reason` rustdoc. Exactly one packet owns that ADR's subject matter and it is 231.
- Any other packet directory (user-ruled 2026-08-19), explicitly including the six whose ACs use the grep form of the freshness check: `206`, `207`, `209`, `210a`, `210b`, `211`, `212`. Also `229` and `230`, which are never edited.
- `docs/11_operational_governance_and_acceptance_gate.md` — dropped from the surface by Round 5: it contains no `build-guests` mention at all.
- Renumbering or renaming any `docs/NN_*.md` page. The highest existing page is `docs/21_data_defaults_and_fixtures.md` (re-derive at write time); no new numbered doc page is created.

## Reported, not fixed

- **`build_script_check_mode_reports_freshness`** in `crates/slicer-runtime/tests/contract/guest_fixture_freshness_tdd.rs` is vacuous. It resolves `test-guests/build-test-guests.sh` and early-returns when that script is absent — and it is absent, as the neighbouring `xtask_build_guests_subcommand_is_wired` test's own comment records ("build-test-guests.sh was removed when the test-guests were …"). The test therefore passes without exercising anything. This packet documents the host-side gate's real behaviour in `docs/03_wit_and_manifest.md`, **names this test there as currently asserting nothing** (AC-7), and records the defect here; fixing or deleting the test is a separate slice, because doing it inside a documentation packet would put untested test-logic changes behind a doc-grep acceptance gate.

## Authoritative Docs

- `CLAUDE.md` — direct read of "## Guest WASM Staleness (MUST follow)" only.
- `docs/03_wit_and_manifest.md` — very long; ranged reads only (staleness-guard table row; "### Build & Freshness Contract (Normative)").
- `docs/05_module_sdk.md` — very long; ranged read only ("**Guest rebuild obligation.**" bullet).
- `docs/07_implementation_status.md` — long; delegate both edits.
- `docs/adr/0014-xtask-guest-discovery-via-validated-filesystem-walk.md` — short; direct read.
- `docs/adr/0045-per-stage-versioned-interfaces-over-monolithic-tier-worlds.md` — long; ranged reads around each `compute_shared_mtime` occurrence and the existing amendment heading.
- `CONTEXT.md` — long; ranged read of the header for structure, then append.
- `docs/specs/guest-freshness-artifact-verification-plan.md` — moderate; direct read of the CONTEXT.md term wording, C11 as amended, and R5-3 / R5-10 / R5-12.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-18`.
  - `AC-1`..`AC-3` cover `CLAUDE.md`, including the anecdote's disposition.
  - `AC-4`..`AC-6` cover `docs/03`, including the two-independent-gates reconciliation (R5-12). `AC-7` books the vacuous-test finding as a report.
  - `AC-8`..`AC-12` cover `docs/05`, the two `docs/07` repins, and the two ADR amendments.
  - `AC-13`..`AC-15` cover the two authoring surfaces and the glossary term.
  - `AC-16`..`AC-17` cover CI wiring and non-vacuous verifier tests (R5-10). `AC-18` is the ledger row.
- Negative: `AC-N1` (no other packet directory touched), `AC-N2` (no logic change leaked in), `AC-N3` (ADR-0054 untouched — 231 owns it), `AC-N4` (no deleted symbol survives as a live citation anywhere in the edited doc surface).
- Cross-packet impact: none outward. This packet is the queue's terminal row; nothing consumes its exports.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only 3 gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `rg -q 'mtime-based' docs/03_wit_and_manifest.md && echo FAIL \|\| echo PASS` | AC-4 | FACT PASS/FAIL |
| `rg -q 'if any source is newer than its artifact' docs/03_wit_and_manifest.md && echo FAIL \|\| echo PASS` | AC-5 | FACT PASS/FAIL |
| `rg -q 'guest_fixture_freshness_tdd' docs/03_wit_and_manifest.md && rg -q 'two independent' docs/03_wit_and_manifest.md && echo PASS \|\| echo FAIL` | AC-6 (R5-12) | FACT PASS/FAIL |
| `rg -q 'is the canonical pre-test gate' docs/05_module_sdk.md && echo FAIL \|\| echo PASS` | AC-8 | FACT PASS/FAIL |
| `if rg -q 'stage_wit_mtime\|compute_shared_mtime' docs/07_implementation_status.md; then echo FAIL; else echo PASS; fi` | AC-9, AC-10 | FACT PASS/FAIL |
| `if rg -q 'compute_shared_mtime' docs/adr/0045-per-stage-versioned-interfaces-over-monolithic-tier-worlds.md; then echo FAIL; else echo PASS; fi` | AC-11 repin half | FACT PASS/FAIL |
| `rg -q '^## Amendment — 20[0-9][0-9]-[0-9][0-9]-[0-9][0-9] \(packets 229/230/231\)' docs/adr/0045-per-stage-versioned-interfaces-over-monolithic-tier-worlds.md && echo PASS \|\| echo FAIL` | AC-11 amendment half | FACT PASS/FAIL |
| `rg -q 'dependency closure' docs/adr/0014-xtask-guest-discovery-via-validated-filesystem-walk.md && echo PASS \|\| echo FAIL` | AC-12 | FACT PASS/FAIL |
| `rg -q '<!-- snippet: wasm-staleness -->' .claude/skills/spec-packet-generator/references/snippets/wasm-staleness.md && echo PASS \|\| echo FAIL` | AC-13 marker integrity | FACT PASS/FAIL |
| `rg -q 'if .STALE:., rebuild and re-run' .claude/skills/spec-review/SKILL.md && echo FAIL \|\| echo PASS` | AC-14 | FACT PASS/FAIL |
| `rg -q '^### Artifact-verified freshness' CONTEXT.md && echo PASS \|\| echo FAIL` | AC-15 | FACT PASS/FAIL |
| `mkdir -p target && awk '/^  test:/{f=1;next} f && /^  [a-z][a-z0-9-]*:/{f=0} f' .github/workflows/ci.yml > target/ci-test-job.txt && rg -q 'cargo test -p xtask' target/ci-test-job.txt && rg -n 'tool: wasm-tools\|cargo test -p xtask' target/ci-test-job.txt \| head -1 \| rg -q 'wasm-tools' && echo PASS \|\| echo FAIL` | AC-16, anchored to the `test` job block so a step landing in `dist-editions` cannot satisfy it | FACT PASS/FAIL |
| `if rg -q 'skipping' xtask/src/wit_verify.rs; then echo FAIL; else mkdir -p target && cargo test -p xtask wit_verify 2>&1 \| tee target/test-output.log \| rg -q '^test result: ok\.' && echo PASS \|\| echo FAIL; fi` | AC-17 | FACT pass/fail |
| `rg -q '^- \[.\] TASK-343 ' docs/07_implementation_status.md && echo PASS \|\| echo FAIL` | AC-18 | FACT PASS/FAIL |
| `git diff --name-only $(git merge-base HEAD master) \| rg '^docs/spec_packets/' \| rg -v '^docs/spec_packets/232-freshness-gate-docs/' \| rg . && echo FAIL \|\| echo PASS` | AC-N1 | FACT PASS/FAIL |
| `git diff --name-only $(git merge-base HEAD master) \| rg '^(crates/\|xtask/src/)' \| rg -v '^xtask/src/wit_verify\.rs$' \| rg . && echo FAIL \|\| echo PASS` | AC-N2 | FACT PASS/FAIL |
| `git diff --name-only $(git merge-base HEAD master) \| rg '^docs/adr/0054-' && echo FAIL \|\| echo PASS` | AC-N3 | FACT PASS/FAIL |
| `cargo check --workspace --all-targets` | Nothing in the tree stopped compiling | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Closure gate | FACT pass/fail |
| `cargo xtask build-guests --check; echo "exit=$?"` | The behaviour being documented, asserted by exit code as the rewritten snippet now demands | FACT exit code |

`cargo test --workspace` is **not** in this matrix.

## Step Completion Expectations

- Every doc edit must be written against the tree as packets 229-231 left it, not against this packet's prose. If an implementer finds the shipped behaviour differs from what an AC assumes, the AC is wrong and must be reported — never write prose that matches the packet and not the code.
- The `CLAUDE.md` rewrite (Step 1) must land before the snippet and skill rewrites (Step 5), because both quote the exit-code contract that `CLAUDE.md` states normatively.
- Re-derive every ledger fact at write time, not from this document: the free TASK ID, the highest `docs/spec_packets/` number, the highest `docs/NN_*.md` page. `TASK-343` was free on 2026-08-19; a parallel packet may have claimed it since.
- Symbol citations in every edited file must follow `CLAUDE.md` §"In-Tree Citation Style": symbol name plus crate-qualified path, never a bare line number and never a bare basename. This packet exists partly because two docs pinned symbols that never existed — do not create new pins of either kind.
- The final sweep (`AC-N4`) runs after all doc steps, not per step; a symbol legitimately quoted mid-edit in one file would otherwise fail it early.

## Context Discipline Notes

- `docs/03_wit_and_manifest.md` and `docs/05_module_sdk.md` are the two largest reads in this packet and must never be opened whole. Locate the anchor with `rg -n`, then read a +/-40-line window.
- `docs/07_implementation_status.md`'s TASK-146b row is a single very long line. Do not read the file to find it; dispatch a worker to perform both string replacements and return a `FACT pass/fail` plus the two greps.
- `CONTEXT.md` is a long glossary. Read the first 10 lines for the structure and append; never read the term list.
- `xtask/src/wit_verify.rs` is long pre-229 and is rewritten by that packet. Read only the test module's skip-guards; do not review the verifier.
- Never open `docs/spec_packets/229-*` or `docs/spec_packets/230-*` `design.md` / `implementation-plan.md`, and never open `docs/spec_packets/231-*` beyond its `packet.spec.md`. Use bounded SUMMARY dispatches.
