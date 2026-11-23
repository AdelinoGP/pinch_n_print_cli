# `xtask test` feature injection and the stale narrow-run example

Type: task
Label: wayfinder:task
Status: resolved (2026-11-02; **A1 + A2 fixed** — the `host-algos` injection
now fires only when cargo's selection includes `slicer-core`, and the
`AGENTS.md` narrow-run example now runs the 9 tests it names instead of zero;
see ## Answer)
Assignee: 2026-11-02 OpenCode wayfinder session — the human selected **fix the two
harness defects** from six fog routes after the frontier re-derivation found
**no open child** (108/108 resolved)
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [ticket 108](108-dev-loop-latency-preflight-concurrency.md)'s
adjacent findings A1 and A2, which were recorded rather than fixed because they
were outside that ticket's question.

## Question

Two defects in the dev-loop entry points, both recorded by t108 and neither a
speed lever:

- **A1 — the unconditional feature injection breaks `-p <package>`.** `test_command`
  (`xtask/src/test.rs`) appends `--features slicer-core/host-algos` whenever the
  caller did not choose features. That is valid only when `slicer-core` is among
  the packages cargo will build. `cargo xtask test -p xtask` (and `-p
  slicer-config`, `slicer-ir`, `slicer-scheduler`, `slicer-gcode`,
  `slicer-helpers`) exits 101 with `the package 'X' does not contain this
  feature`. The feature exists to turn on the ~34 Arachne parity files gated by
  `#![cfg(feature = "host-algos")]` in `slicer-core`, so narrowing the injection
  to selections that include `slicer-core` preserves the gate.
- **A2 — the documented narrow-run example runs zero tests.** `AGENTS.md` names
  `cargo test -p slicer-runtime --test contract
  core_module_ir_access_contract_tdd` as the narrow example. The module lives in
  `crates/slicer-scheduler/tests/contract/core_module_ir_access_contract_tdd.rs`,
  and the scheduler's target is `scheduler_contract` (an explicit `[[test]]` in
  `crates/slicer-scheduler/Cargo.toml`), so the documented command builds the
  runtime's contract bucket, filters everything out, and prints a green-looking
  `0 passed; N filtered out`.

Fix both, and prove each with a command that can fail.

## Method (pre-declared; deviations in ## Answer)

- **A1.** Extract the selection predicate from `test_command` into a pure
  function over the forwarded cargo args and unit-test it at the table level:
  `-p slicer-core`, `-p xtask`, `--workspace`, `--workspace --exclude
  slicer-core`, `--package=slicer-core`, `-pX`, no selector, and
  `--manifest-path`. Then run the gated entry point end-to-end on one package
  without a `slicer-core` dependency and on one with it.
- **A2.** Correct the crate and target in `AGENTS.md`, then run the corrected
  command with `-- --list` and require a non-zero match count.
- **No weakened gates.** The injection still fires for every selection that
  includes `slicer-core`; the only change is that it no longer asks cargo to
  activate a feature on a package that cannot have it. No test is skipped and no
  assertion is relaxed.

## Stop rule

If the predicate cannot separate a valid from an invalid activation without a
hand-maintained package roster, stop and report; do not paper over it with a
retry-on-cargo-error fallback.

## Answer

**Verdict: both defects fixed, with no test weakened and no feature gate lost.**

### A1 — the injection follows the selection

Before, `cargo xtask test -p xtask --summary` exited **101** with
`the package 'xtask' does not contain this feature: slicer-core/host-algos`;
the same failure hit `slicer-config`, `slicer-ir`, `slicer-scheduler`,
`slicer-gcode` and `slicer-helpers`. The feature exists to turn on
`slicer-core`'s ~34 Arachne parity files, and every in-repo package that
depends on `slicer-core` and needs the feature already enables it in its own
manifest (`slicer-runtime` and `slicer-wasm-host` directly; `slicer-sdk` via
its non-wasm dependency). So the injection is narrowed to selections that
include `slicer-core` itself.

Implementation (`xtask/src/test.rs`):

- `build_test_arguments` is a pure function holding both argument policies —
  the conditional injection and the quarantine `--skip` roster — and
  `test_command` delegates to it after splitting caller args at `--`.
- `selection_includes_slicer_core` parses `-p` / `--package`, `--workspace` /
  `--all`, `--exclude` and `--manifest-path`, including the attached forms
  (`-pfoo`, `--package=foo`) and cargo's `name@version` pkgid form. Precedence:
  package selectors, then workspace, then manifest path, then the
  virtual-workspace default (every member).
- `manifest_declares_package` reads `[package].name` for `--manifest-path`; an
  unreadable manifest returns `false` so cargo reports the real error rather
  than a feature-resolution error.
- `QUARANTINED_TESTS` moved to module scope so the pure builder can apply it.
- Five unit tests: a 13-case selector table, exact argument vectors for a
  foreign package and for `slicer-core`, the caller-feature override, and the
  three manifest-path arms.

End-to-end proof:

| command | before | after |
| --- | --- | --- |
| `cargo xtask test -p xtask --summary` | exit 101, feature error | exit 0, `193 passed; 0 failed` |
| `cargo xtask test -p slicer-config --summary` | exit 101, feature error | reaches the tests (one pre-existing red, below) |
| `cargo xtask test -p slicer-core --lib --no-run --summary` | exit 0 | exit 0, command still contains `--features slicer-core/host-algos` |
| `cargo xtask test --manifest-path xtask/Cargo.toml --no-run --summary` | exit 101 | exit 0, no feature injected |
| `cargo xtask test --manifest-path crates/slicer-core/Cargo.toml --lib --no-run --summary` | exit 0 | exit 0, feature injected |

The `slicer-core` arm compiles with the feature (`Finished test profile … in
1m 17s`), so the gate that keeps the Arachne suite from silently compiling to
empty no-ops is intact.

### A2 — the example names a real target

Before, `cargo test -p slicer-runtime --test contract
core_module_ir_access_contract_tdd` built the runtime's contract bucket and
reported **0 passed; 314 filtered out**. The module lives in
`crates/slicer-scheduler/tests/contract/main.rs`, and the scheduler's target is
the explicit `[[test]] name = "scheduler_contract"`. `AGENTS.md` now documents:

```
cargo test -p slicer-scheduler --test scheduler_contract core_module_ir_access_contract_tdd
```

which reports **9 passed; 0 failed; 17 filtered out**. The bucket note now
says `slicer-runtime`'s buckets are
`unit|contract|executor|integration|e2e`, which is where those five binaries
belong.

### Verification

- `cargo test -p xtask` — **193 passed, 0 failed**.
- `cargo clippy -p xtask --all-targets -- -D warnings` — exit 0.
- `cargo clippy --workspace --all-targets -- -D warnings` — exit 0.
- `cargo fmt -p xtask --check` — exit 0.
- `cargo xtask check-literals` — 0 violations; `check-test-quality --report` —
  0 findings (both run inside `xtask test`).
- `verify.py` exits 0 checkout-only (`--no-cargo`), with cargo, and with
  `--full`; `verify.py --self-test` rejects 5/5 negative controls.
- Evidence: [`evidence/t109-harness-defects/`](../evidence/t109-harness-defects/FINDINGS.md).

### Adjacent findings (recorded, not fixed)

- **`slicer-config`'s census is red before and after.**
  `real_toml_manifest_census_rederives_expected_scope`
  (`crates/slicer-config/tests/registry_census_tdd.rs`) asserts **281** declared
  keys against **293** in the manifests. Plain
  `cargo test -p slicer-config --test registry_census_tdd
  real_toml_manifest_census_rederives_expected_scope` reproduces **293 vs 281**
  with no xtask involvement, so the failure is not introduced here and is
  **not weakened**. A1's fix makes the package reachable through the gated
  entry point, so the red is now visible where cargo's feature error used to
  hide it.
- **`--no-run --summary` prints a false alarm.** A successful build-only run
  prints `(no test result: lines found — build may have failed before tests
  ran)` and then `VERDICT: PASS`. The verdict follows the exit code and is
  correct; only the message is misleading.
- **The ignored local `CLAUDE.md` mirror still carries the old line.** It is
  gitignored (`.gitignore` line 48); the tracked `AGENTS.md` is corrected.
  Captured copies under `.local-artifacts/` and `tmp/` were not touched.

### Disclosures

No wall or CPU claim, no benchmark, no production geometry/IR/guest/runtime
edit: this is a host harness and documentation fix, so the map's 0.80 % floor,
paired acceptance and KEEP/DROP rules do not apply and the destination is
unmoved. Both guest-freshness gates reported fresh inside the `xtask test`
runs. The only test-count change is +5 unit tests in `xtask`; no test was
skipped, narrowed or deleted.

