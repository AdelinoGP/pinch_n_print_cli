---
status: implemented
packet: 199-literal-gate-enforcement
task_ids:
  - TASK-321
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 199-literal-gate-enforcement

## Goal

Flip the struct-literal churn gate to enforced: wire `cargo xtask check-literals` (workspace-wide enforce mode) into `cargo xtask test`'s preflight ahead of the guest-freshness gate, promote the command into CLAUDE.md's required-before-commit set and enforced-state wording (CLAUDE.md rule section, CLAUDE.md gated-entry-point section, `docs/21_data_defaults_and_fixtures.md`), repair CLAUDE.md §"Feature-gated test files" (stale slicer-gcode/host-algos claim; new slicer-sdk `--features test` hazard), and convert the no-sweep-packet residue (slicer-model-io, slicer-helpers, slicer-macros) so the workspace-wide gate exits 0.

## Scope Boundaries

This packet owns enforcement wiring (`xtask/src/test.rs`, `xtask/src/main.rs` USAGE, a thin preflight entry in `xtask/src/check_literals.rs` if the packet-194 scan API is CLI-shaped only), the CLAUDE.md/docs-21 enforced-state flip, and residue conversion in the five crates no sweep packet covered (grounded residue: slicer-model-io, slicer-helpers, slicer-macros; pnp-cli-locator and slicer-schema verified clean 2026-08-07). No checker-semantics change (packet 194 owns the tool), no re-sweep of packets 196–198 areas, no `Default` impl added anywhere, no production-code behavior change.

## Prerequisites and Blockers

- Depends on: packets 194 (`cargo xtask check-literals` CLI + rule docs), 195 (fixture bases), 196, 197, 198 (area sweeps green). All five must be `implemented` — the workspace-wide enforce gate cannot exit 0 while any sweep area is red, and the flip edits text that packet 194 authored.
- Unblocks: nothing (terminal packet of `docs/specs/struct-literal-churn-gate-plan.md`; closing it closes the plan).
- Activation blockers: any of packets 194, 195, 196, 197, 198 not yet `implemented`.

## Acceptance Criteria

State ACs only here; `requirements.md` references their IDs.

- **AC-1. Given** the fully swept and residue-converted tree, **when** the gate runs workspace-wide in enforce mode with no path filter, **then** it exits 0. | `cargo xtask check-literals; test $? -eq 0 && echo PASS`
- **AC-2. Given** the residue crates no sweep packet covered, **when** the area gate runs on the three grounded-residue roots, **then** it exits 0 (tests, benches, `#[cfg(test)]` mods in src). | `cargo xtask check-literals crates/slicer-model-io crates/slicer-macros crates/slicer-helpers; test $? -eq 0 && echo PASS`
- **AC-3. Given** the preflight wiring in `test_command` (`xtask/src/test.rs`), **when** the xtask preflight unit tests run, **then** `preflight_blocks_on_violating_fixture_tree` and `preflight_passes_on_clean_fixture_tree` both pass, and `xtask/src/test.rs` references the `check_literals` module. | `rg -q 'check_literals' xtask/src/test.rs && mkdir -p target && cargo test -p xtask preflight 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-4. Given** CLAUDE.md's Build & Test Commands block, **when** grepped, **then** `cargo xtask check-literals` appears on a line carrying the `# required before committing` marker, next to the existing clippy gate line. | `rg -q 'cargo xtask check-literals.*# required before committing' CLAUDE.md && echo PASS`
- **AC-5. Given** the CLAUDE.md rule section authored by packet 194 and the gated-entry-point section, **when** grepped post-flip, **then** the literal marker `not yet a required gate` is ABSENT from CLAUDE.md, the rule section carries the replacement anchor `enforced since packet 199`, and CLAUDE.md documents that `cargo xtask test` runs the `check-literals preflight` before the guest-freshness gate. | `! rg -q 'not yet a required gate' CLAUDE.md && rg -q 'enforced since packet 199' CLAUDE.md && rg -q 'check-literals preflight' CLAUDE.md && echo PASS`
- **AC-6. Given** `docs/21_data_defaults_and_fixtures.md`, **when** grepped post-flip, **then** its gate-off phrasing is gone (`not yet a required gate` absent) and the enforced-state anchor `enforced since packet 199` is present. | `! rg -q 'not yet a required gate' docs/21_data_defaults_and_fixtures.md && rg -q 'enforced since packet 199' docs/21_data_defaults_and_fixtures.md && echo PASS`
- **AC-7. Given** CLAUDE.md §"Feature-gated test files report green when they don't compile", **when** grepped post-repair, **then** the stale claim listing `slicer-gcode` among the crates depending on `slicer-core` with `host-algos` is gone, the section states slicer-gcode has `no production \`slicer-core\` dependency` (only a packet-196 dev-dep on `slicer-sdk` with feature `test`), and the slicer-sdk hazard is documented with the correct invocation `cargo test -p slicer-sdk --features test`. | `! rg -qF 'because \`slicer-gcode\`, \`slicer-runtime\`' CLAUDE.md && rg -qF 'no production \`slicer-core\` dependency' CLAUDE.md && rg -qF 'cargo test -p slicer-sdk --features test' CLAUDE.md && echo PASS`
- **AC-8. Given** the pre-conversion baselines `target/gate-199-<crate>-baseline.txt` (captured in Step 1 via the identical pipeline, one file per residue crate), **when** the three residue crates' suites re-run post-conversion, **then** every `test result` line reports `0 failed` and each sorted, time-stripped summary multiset is byte-identical to its baseline. | `for c in slicer-model-io slicer-helpers slicer-macros; do mkdir -p target && cargo test -p $c 2>&1 | tee target/test-output.log >/dev/null; grep -E '^test result' target/test-output.log | sed 's/; finished in .*//' | sort > target/gate-199-$c-post.txt; test "$(grep -vc ' 0 failed' target/gate-199-$c-post.txt)" -eq 0 && diff target/gate-199-$c-baseline.txt target/gate-199-$c-post.txt || { echo FAIL:$c; exit 1; }; done; echo PASS`

- **AC-9. Given** `.github/workflows/ci.yml`, **when** parsed and grepped post-wiring, **then** the `docs-guard` job contains a step invoking `check-literals` in enforce mode (no `--report`, no path filter), the file remains valid YAML, and the four pre-existing job names (`fmt`, `docs-guard`, `clippy`, `test`) are all still present. | `rg -q 'check-literals' .github/workflows/ci.yml && ! rg -q 'check-literals[[:space:]]+[^[:space:]]' .github/workflows/ci.yml && python -c "import yaml,sys; d=yaml.safe_load(open('.github/workflows/ci.yml')); j=set(d['jobs']); assert {'fmt','docs-guard','clippy','test'} <= j, j; assert any('check-literals' in str(s) for s in d['jobs']['docs-guard']['steps']), 'step not in docs-guard'" && echo PASS`

## Negative Test Cases

- **AC-N1. Given** an xtask unit test that builds a throwaway fixture workspace root (a `crates/<name>/src/lib.rs` defining a pub struct with 5 named fields plus a `crates/<name>/tests/*.rs` containing an exhaustive literal of it, no waiver), **when** the preflight helper runs against that root, **then** it returns a nonzero blocking code; and against a clean twin fixture (same literal with a `..` rest) it returns 0. | `mkdir -p target && cargo test -p xtask preflight 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-N2. Given** a temporary violating file injected into the real tree at `crates/slicer-ir/tests/data/gate_probe_199_tmp.rs` (an exhaustive `ObjectMesh` literal; the path is scanned as `crates/*/tests/**` but is not a cargo test target, and the command removes it before exiting), **when** the CLI gate runs on `crates/slicer-ir` with the probe present and again after cleanup, **then** the first run exits 1 printing a violation line naming the probe file and `exhaustive literal of watched type`, and the second exits 0. | `mkdir -p crates/slicer-ir/tests/data && printf 'fn probe() {\n    let _ = ObjectMesh { id: 0 };\n}\n' > crates/slicer-ir/tests/data/gate_probe_199_tmp.rs; cargo xtask check-literals crates/slicer-ir > /tmp/gp199.txt 2>&1; ec1=$?; rm -f crates/slicer-ir/tests/data/gate_probe_199_tmp.rs; rmdir crates/slicer-ir/tests/data 2>/dev/null; cargo xtask check-literals crates/slicer-ir; ec2=$?; test $ec1 -eq 1 && grep -q 'exhaustive literal of watched type' /tmp/gp199.txt && grep -q 'gate_probe_199_tmp' /tmp/gp199.txt && test $ec2 -eq 0 && echo PASS`
- **AC-N3. Given** the same temporary probe file present in the tree, **when** `cargo xtask test -p xtask` runs, **then** it aborts before running any test, exits nonzero, and prints the packet-defined failure line containing `check-literals preflight failed` (cleanup runs unconditionally; a false-negative path is bounded because xtask has no `slicer-core` dependency, so the auto-appended `--features slicer-core/host-algos` makes cargo error out fast instead of running a suite). | `mkdir -p crates/slicer-ir/tests/data && printf 'fn probe() {\n    let _ = ObjectMesh { id: 0 };\n}\n' > crates/slicer-ir/tests/data/gate_probe_199_tmp.rs; cargo xtask test -p xtask > /tmp/gpt199.txt 2>&1; ec=$?; rm -f crates/slicer-ir/tests/data/gate_probe_199_tmp.rs; rmdir crates/slicer-ir/tests/data 2>/dev/null; test $ec -ne 0 && grep -q 'check-literals preflight failed' /tmp/gpt199.txt && ! grep -qE '^test result' /tmp/gpt199.txt && echo PASS`
- **AC-N4. Given** the frozen waiver format (`// exhaustive: <reason>`, reason mandatory), **when** the whole enforced surface is grepped, **then** no waiver comment anywhere in `crates/` or `modules/` has an empty reason. | `rg -n '// exhaustive:[[:space:]]*$' crates modules; test $? -eq 1 && echo PASS`
- **AC-N5. Given** the pre-conversion counts in `target/gate-199-assert-baseline.txt` and `target/gate-199-testattr-baseline.txt` (captured in Step 1 via the identical pipelines), **when** `assert!`/`assert_eq!`/`assert_ne!` occurrences and `#[test]` attributes are re-counted across the three residue crate roots post-conversion, **then** both counts are unchanged (conversion is construction-syntax-only; no assertion added, removed, or weakened). | `a=$(rg -o 'assert(_eq|_ne)?!' crates/slicer-model-io crates/slicer-helpers crates/slicer-macros | wc -l); t=$(rg -o '#\[test\]' crates/slicer-model-io crates/slicer-helpers crates/slicer-macros | wc -l); test "$a" = "$(cat target/gate-199-assert-baseline.txt)" && test "$t" = "$(cat target/gate-199-testattr-baseline.txt)" && echo PASS`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask check-literals; test $? -eq 0 && echo PASS`
- Packet-close acceptance ceremony only (queue-final packet; plan closure): `cargo xtask test --summary --workspace` — dispatched to a sub-agent with a `FACT pass/fail` return per CLAUDE.md §Test Discipline; never run inline and never used as an AC command.

## Authoritative Docs

- `docs/specs/struct-literal-churn-gate-plan.md` - short; direct read; locked decisions 2 and 4 govern the wiring, waiver policy, and this packet's scope.
- `docs/21_data_defaults_and_fixtures.md` - authored by packet 194; direct read at implementation time (rule text whose gate-off phrasing this packet flips; conversion + waiver rules for the residue).
- `CLAUDE.md` §Build & Test Commands, §Test Discipline (incl. "Feature-gated test files" and "`cargo xtask test` — the gated entry point"), §Guest WASM Staleness - direct read of named sections only; four of them are edit targets.

## Doc Impact Statement (Required)

Specific same-packet doc edits:

- `CLAUDE.md` Build & Test Commands: `cargo xtask check-literals` line with the `# required before committing` marker - `rg -q 'cargo xtask check-literals.*# required before committing' CLAUDE.md`
- `CLAUDE.md` rule section (packet-194-authored): gate-off marker removed, replaced by `enforced since packet 199` - `! rg -q 'not yet a required gate' CLAUDE.md && rg -q 'enforced since packet 199' CLAUDE.md`
- `CLAUDE.md` §"`cargo xtask test` — the gated entry point": pipeline description now names the `check-literals preflight` step ahead of `build-guests --check` - `rg -q 'check-literals preflight' CLAUDE.md`
- `CLAUDE.md` §"Feature-gated test files report green when they don't compile": stale slicer-gcode claim end-state repair + slicer-sdk `--features test` hazard addendum - `! rg -qF 'because \`slicer-gcode\`, \`slicer-runtime\`' CLAUDE.md && rg -qF 'cargo test -p slicer-sdk --features test' CLAUDE.md`
- `docs/21_data_defaults_and_fixtures.md`: gate-off phrasing replaced with enforced-state wording (`enforced since packet 199`; names the `cargo xtask test` preflight and the required-before-commit status) - `! rg -q 'not yet a required gate' docs/21_data_defaults_and_fixtures.md && rg -q 'enforced since packet 199' docs/21_data_defaults_and_fixtures.md`

Doc greps are appended to the ACs (AC-4 through AC-7).

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
