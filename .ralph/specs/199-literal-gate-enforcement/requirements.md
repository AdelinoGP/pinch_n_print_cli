# Requirements: 199-literal-gate-enforcement

## Packet Metadata

- Grouped task IDs: `TASK-321`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Packets 194–198 built the struct-literal churn gate and swept the nine high-traffic areas green, but the gate is still advisory: `cargo xtask test` does not run it, CLAUDE.md marks the rule `not yet a required gate`, and five workspace crates were in no sweep packet's scope. Until enforcement flips on, the failure the plan measured (`docs/specs/struct-literal-churn-gate-plan.md`: `a579fc18`'s 165-file filler sweep, and the prior TASK-200a–e fix that landed completely yet decayed because "it produced no ongoing rule") recurs with the next added field. This packet is the plan's locked decision 4 (wiring, last, only after sweeps are green) plus the accumulated obligations assigned to the queue-final row: residue conversion, the CLAUDE.md stale-fact repair for the slicer-gcode/host-algos claim (which caused a real authoring error in packet 196), the slicer-sdk `--features test` hazard addendum, and the whole-plan closing gates.

## In Scope

- Preflight wiring: `test_command` (`xtask/src/test.rs`) runs the check-literals workspace enforce scan before `build_guests::check_command`, aborting with exit 1 and the failure line `xtask test: check-literals preflight failed` (plus a pointer to `docs/21_data_defaults_and_fixtures.md`) when violations exist. The `--summary-from` parse-only path stays gate-free (no test run = no gate, matching the existing guest-freshness behavior).
- A testable preflight helper in xtask (reusing packet 194's scan entry point; a thin wrapper in `xtask/src/check_literals.rs` only if that entry is CLI-shaped) plus two unit tests: blocking on a violating fixture tree, passing on a clean twin.
- `xtask/src/main.rs` USAGE text: the `test [ARGS...]` description names the check-literals preflight step.
- CLAUDE.md flip: required-before-commit line in Build & Test Commands; rule-section marker `not yet a required gate — enforcement flips on in packet 199` replaced with `enforced since packet 199` wording; gated-entry-point section (`cargo xtask test`) pipeline description updated to name the preflight.
- CLAUDE.md §"Feature-gated test files report green when they don't compile": end-state repair of the stale claim that `slicer-gcode` depends on `slicer-core` with `host-algos` (verified false 2026-08-07; the correction is already applied in the uncommitted working tree — the step verifies end-state and finalizes wording against the post-196 tree: no production `slicer-core` dependency; only a packet-196 dev-dep on `slicer-sdk` with feature `test`, which pulls `slicer-core/host-algos` into slicer-gcode's dev graph only). Same section gains the slicer-sdk hazard. The precise wording is NOT assumed: packet 198 adds `[[test]] required-features = ["test"]` entries, but `crates/slicer-sdk/Cargo.toml` also carries a self dev-dep (`slicer-sdk = { path = ".", features = ["test"] }`) which may already enable the feature for a bare `-p slicer-sdk` run. The step MUST measure first — run `cargo test -p slicer-sdk` and `cargo test -p slicer-sdk --features test`, compare the number of test binaries each builds — and write whichever wording the evidence supports (either "bare runs silently skip N binaries; always pass `--features test`", or "the self dev-dep enables it; `--features test` is belt-and-braces"). `cargo test -p slicer-sdk --features test` is the documented invocation either way. Re-verify the three remaining host-algos dependents (`crates/slicer-runtime/Cargo.toml`, `crates/slicer-sdk/Cargo.toml` non-wasm32 target dep, `crates/slicer-wasm-host/Cargo.toml` — all verified 2026-08-07) before restating them.
- `docs/21_data_defaults_and_fixtures.md`: gate-off phrasing replaced with enforced-state wording.
- Residue conversion in the crates no sweep packet covered, under the sweep rules (FRU over base, omit default-equal fields, reasoned waivers only where exhaustiveness is the intent, never change assertions). Grounded inventory (2026-08-07, approximate pre-tool scan — re-derive via `cargo xtask check-literals --report` in Step 1):
  - `crates/slicer-model-io`: exhaustive `ObjectMesh` literals in `tests/model_writer_roundtrip_tdd.rs` (3 sites), `tests/threemf_writer_roundtrip_tdd.rs` (1), `tests/world_z_below_floor_tdd.rs` (1), `tests/world_z_canonical_surface_tdd.rs` (1), and the `#[cfg(test)]` mod of `src/loader.rs` (`make_object`, 1).
  - `crates/slicer-helpers`: exhaustive `ObjectMesh` literals in `tests/decimate_tdd.rs` (1) and `tests/repair_tdd.rs` (1), each inside the file's single constructor helper.
  - `crates/slicer-macros`: `tests/slicer_module_tdd.rs` — two exhaustive `Self { .. }`-less literals inside `impl InfillOutputBuilder` / `impl PerimeterOutputBuilder` mock blocks whose names collide with watched `crates/slicer-sdk/src/builders.rs` types (last-path-segment matching + impl-target tracking fire on them).
  - `crates/pnp-cli-locator`, `crates/slicer-schema`: verified clean 2026-08-07 (no test dirs with watched literals; `#[cfg(test)]` mods clean); covered by the workspace gate, no edits expected.
  - Any newly appeared residue found by the Step-1 re-derivation anywhere outside packets 196–198's areas is also this packet's to convert, same rules.
- CI enforcement: `.github/workflows/ci.yml` gains one gate step. Grounded 2026-08-07: the workflow has four jobs (`fmt`, `docs-guard`, `clippy`, `test`) on `push`/`pull_request` to `main`/`master`, and the `test` job invokes `cargo test -p ...` directly — it never calls `cargo xtask test`, so the Step-4 preflight alone would leave CI blind to violations. The step lands in `docs-guard`, which already runs an xtask guard (`cargo run -q -p xtask -- check-deviations --check`) and needs no toolchain component; `check-literals` is parse-only, so it needs no guest WASMs and no release build. The workspace root `.cargo/config.toml` defines `xtask = "run --quiet -p xtask --"`, so either invocation form works; match the neighbouring step's explicit `cargo run -q -p xtask --` form.
- Waiver-inventory audit: enumerate every `// exhaustive:` waiver workspace-wide, verify each carries a reason, and record the re-derived final count in the packet-close notes (never frozen into an AC).
- Whole-plan closing gates (this is the terminal queue row): workspace-wide gate exit 0, `cargo check`/`cargo clippy --workspace --all-targets` clean, and the acceptance-ceremony `cargo xtask test --summary --workspace` dispatch.

## Out of Scope

- Checker semantics, watchlist derivation, violation formats, exit codes (packet 194 owns them; this packet only consumes the CLI contract).
- Re-sweeping or editing any packet 196/197/198 area file, and `crates/slicer-wasm-host/test-guests/**` (rule-exempt).
- Adding `Default` impls anywhere (residue conversion uses bases/waivers only; packet-195 locks stand).
- Production-code behavior changes; module manifests; WIT/IR/schema contracts.
- CI jobs other than the one gaining the gate step: `fmt`, `test`, and the rest of `docs-guard`'s existing steps are untouched. In particular the `test` job's `cargo test -p ...` invocations are NOT rerouted through `cargo xtask test` — that reroute is a larger change (guest-build ordering, runtime) and is deliberately out of scope; the standalone gate step covers CI enforcement instead.
- Renaming the slicer-macros mock types (the `#[slicer_module]` macro expansion requires those exact type names in scope; waivers are the sanctioned conversion there).

## Authoritative Docs

- `docs/specs/struct-literal-churn-gate-plan.md` - short; direct read (locked decisions 2 and 4; queue row 6).
- `docs/21_data_defaults_and_fixtures.md` - authored by packet 194; direct read at implementation time (edit target + conversion rules).
- `CLAUDE.md` - named sections only: Build & Test Commands, Test Discipline (both subsections), Guest WASM Staleness (edit targets and constraints).

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-9` (`AC-9` covers the CI gate step).
- Negative: `AC-N1` through `AC-N5`.
- Cross-packet impact: packet 194's AC-9 grep (`not yet a required gate` present) is intentionally invalidated by this flip — that AC was a point-in-time authoring gate, not a standing invariant; packets 196–198's area gates remain green invariants and are re-proven transitively by AC-1.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only the closure gates.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo xtask check-literals; test $? -eq 0 && echo PASS` | AC-1 workspace-wide enforce green | FACT pass/fail |
| `cargo xtask check-literals crates/slicer-model-io crates/slicer-macros crates/slicer-helpers; test $? -eq 0 && echo PASS` | AC-2 residue-area gate | FACT pass/fail |
| `rg -q 'check_literals' xtask/src/test.rs && mkdir -p target && cargo test -p xtask preflight 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | AC-3/AC-N1 preflight wiring + unit tests | FACT pass/fail; SNIPPETS <=20 lines on failure |
| AC-N2 probe command (verbatim from `packet.spec.md`) | CLI gate catches an injected violation and is self-cleaning | FACT pass/fail |
| AC-N3 probe command (verbatim from `packet.spec.md`) | `cargo xtask test` aborts pre-test on violations | FACT pass/fail |
| `rg -q 'cargo xtask check-literals.*# required before committing' CLAUDE.md && echo PASS` | AC-4 commit-gate line | FACT pass/fail |
| `! rg -q 'not yet a required gate' CLAUDE.md && rg -q 'enforced since packet 199' CLAUDE.md && rg -q 'check-literals preflight' CLAUDE.md && echo PASS` | AC-5 CLAUDE.md flip | FACT pass/fail |
| `! rg -q 'not yet a required gate' docs/21_data_defaults_and_fixtures.md && rg -q 'enforced since packet 199' docs/21_data_defaults_and_fixtures.md && echo PASS` | AC-6 docs/21 flip | FACT pass/fail |
| `! rg -qF 'because \`slicer-gcode\`, \`slicer-runtime\`' CLAUDE.md && rg -qF 'no production \`slicer-core\` dependency' CLAUDE.md && rg -qF 'cargo test -p slicer-sdk --features test' CLAUDE.md && echo PASS` | AC-7 stale-fact repair + sdk hazard | FACT pass/fail |
| AC-8 three-crate baseline loop (verbatim from `packet.spec.md`) | residue suites green, multiset unchanged | FACT pass/fail |
| AC-9 CI-wiring command (verbatim from `packet.spec.md`) | gate step present in `docs-guard`, enforce mode, YAML valid, four jobs intact | FACT pass/fail |
| `rg -n '// exhaustive:[[:space:]]*$' crates modules; test $? -eq 1 && echo PASS` | AC-N4 waiver reasons | FACT pass/fail |
| AC-N5 count-compare command (verbatim from `packet.spec.md`) | construction-syntax-only invariant | FACT pass/fail |
| `cargo check --workspace --all-targets` | closure gate | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | closure gate | FACT pass/fail |
| `cargo xtask test --summary --workspace` | acceptance ceremony ONLY (queue-final packet; plan closure per CLAUDE.md §Test Discipline) — dispatch to a sub-agent | FACT pass/fail |

Commands must have small, parseable output suitable for delegation.

## Step Completion Expectations

- Step 1's baselines (`target/gate-199-*-baseline.txt`) MUST be captured before any residue edit; AC-8/AC-N5 diff against them. `target/` is wiped by cleans — if a clean intervenes, re-capture baselines from the pre-edit git state, never from the edited tree.
- Residue conversion (Steps 2–3) completes before the docs flip (Step 5) so CLAUDE.md never advertises a required gate the tree fails; the wiring step (Step 4) may land in either order relative to Step 5 but after Steps 2–3.
- The Step-1 `--report` re-derivation is authoritative over this packet's grounded inventory: convert what the tool reports, not what the packet lists, and record any delta in the close notes.

## Context Discipline Notes

- `crates/slicer-model-io/src/loader.rs` is >3000 lines: ranged reads only, targeting the `#[cfg(test)]` mod's `make_object` (locate with `rg -n 'fn make_object' crates/slicer-model-io/src/loader.rs`).
- `docs/specs/_OLD/default-builder-migration.md` (1449 lines) is NOT needed by this packet; do not open it.
- Predecessor packets' `design.md`/`implementation-plan.md` files are out of bounds; consume their exports via `packet.spec.md` or a SUMMARY dispatch only.
- Waiver-inventory audit and docs/07 updates go through worker dispatches, never full-file reads of `docs/07_implementation_status.md`.
