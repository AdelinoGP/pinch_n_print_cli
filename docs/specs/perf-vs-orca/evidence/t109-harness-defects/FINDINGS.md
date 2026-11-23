# t109 — `xtask test` feature injection and the stale narrow-run example

Route: [ticket 109](../../issues/109-xtask-host-algos-injection-and-narrow-run-example.md).
Harness-correctness take at HEAD `59b1b21f` plus the ticket's changes. No
production geometry, IR, guest or scheduler behaviour is touched.

## Headline

Both defects recorded by [ticket 108](../../issues/108-dev-loop-latency-preflight-concurrency.md)
are fixed and verified with commands that can fail:

- **A1.** `cargo xtask test -p xtask --summary` now exits **0**. Before the fix
  the same command exited **101** with `the package 'xtask' does not contain
  this feature: slicer-core/host-algos`, because `test_command` injected a
  qualified feature name on every selection. The injection now fires only when
  cargo's selection includes `slicer-core` itself.
- **A2.** The narrow-run example in `AGENTS.md` now runs the module it names:
  `cargo test -p slicer-scheduler --test scheduler_contract
  core_module_ir_access_contract_tdd` matches **9 tests, 9 passed**. Before the
  fix the documented command pointed at `slicer-runtime`'s `contract` bucket and
  reported **0 passed; 314 filtered out** — a green-looking no-op.

## A1 — the feature injection

### Before

The recorded t108 finding is reproducible by reverting the predicate and running
the gated entry point:

```
$ cargo xtask test -p xtask --summary
error: the package 'xtask' does not contain this feature: slicer-core/host-algos
exit 101
```

The same command failed for `slicer-config`, `slicer-ir`, `slicer-scheduler`,
`slicer-gcode` and `slicer-helpers` — every package that does not depend on
`slicer-core`.

### After

```
$ cargo xtask test -p xtask --summary
check-literals: 0 violation(s) in 0 file(s) (watchlist: 159 types)
check-test-quality: 0 finding(s) in 0 file(s) [report mode]
xtask test: running `cargo test -p xtask -- --skip arachne_parity_pipeline_concentric_infill_uses_arachne` (summary mode)
test result: ok. 193 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.94s
VERDICT: PASS
exit 0
```

A second foreign package reaches its tests instead of cargo's feature error:

```
$ cargo xtask test -p slicer-config --summary
...
test result: FAILED. 5 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
exit 101
```

That failure is **pre-existing and unrelated** (see Disclosures).

`slicer-core` still receives the feature, so the Arachne parity gate is intact:

```
$ cargo xtask test -p slicer-core --lib --no-run --summary
xtask test: running `cargo test -p slicer-core --lib --no-run --features slicer-core/host-algos -- --skip arachne_parity_pipeline_concentric_infill_uses_arachne` (summary mode)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1m 17s
exit 0
```

The `--manifest-path` form is covered too:

```
$ cargo xtask test --manifest-path xtask/Cargo.toml --no-run --summary
xtask test: running `cargo test --manifest-path xtask/Cargo.toml --no-run -- --skip ...`
exit 0

$ cargo xtask test --manifest-path crates/slicer-core/Cargo.toml --lib --no-run --summary
xtask test: running `cargo test --manifest-path crates/slicer-core/Cargo.toml --lib --no-run --features slicer-core/host-algos -- --skip ...`
exit 0
```

### Implementation

- `build_test_arguments` (`xtask/src/test.rs`) is a pure function holding the
  two argument policies — the conditional `host-algos` injection and the
  quarantine `--skip` roster — so both are testable without spawning cargo.
  `test_command` splits caller args at `--` and delegates to it.
- `selection_includes_slicer_core` parses `-p` / `--package`, `--workspace` /
  `--all`, `--exclude`, `--manifest-path`, the attached forms (`-pfoo`,
  `--package=foo`), and cargo's `name@version` pkgid form. Precedence:
  package selectors, then workspace, then manifest path, then the
  virtual-workspace default (all members).
- `manifest_declares_package` reads `[package].name` for the
  `--manifest-path` form; an unreadable manifest returns `false` so cargo
  reports the real error instead of a feature-resolution error.
- `QUARANTINED_TESTS` moved to module scope so the pure builder can apply it.
- Five unit tests: a 13-case selector table (including `name@version`), exact
  argument vectors for a foreign package and for `slicer-core`, the
  caller-feature override, and the manifest-path branches.

## A2 — the narrow-run example

### Before

```
$ cargo test -p slicer-runtime --test contract core_module_ir_access_contract_tdd
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 314 filtered out
```

`core_module_ir_access_contract_tdd` is a module of
`crates/slicer-scheduler/tests/contract/main.rs`, and the scheduler's target is
declared as `scheduler_contract` in `crates/slicer-scheduler/Cargo.toml`.

### After

```
$ cargo test -p slicer-scheduler --test scheduler_contract core_module_ir_access_contract_tdd
running 9 tests
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 17 filtered out; finished in 0.06s
```

`AGENTS.md` now carries that command, with the bucket note still scoped to
`slicer-runtime` (whose buckets really are `unit|contract|executor|integration|e2e`).

## Verification

| command | result |
| --- | --- |
| `cargo test -p xtask` | **193 passed, 0 failed** |
| `cargo clippy -p xtask --all-targets -- -D warnings` | exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 |
| `cargo fmt -p xtask --check` | exit 0 |
| `cargo xtask check-literals` (run inside `xtask test`) | 0 violations |
| `cargo xtask check-test-quality --report` (run inside `xtask test`) | 0 findings |
| `verify.py` (checkout-only static + narrow cargo checks) | exit 0 |

## Disclosures and adjacent findings

- **`slicer-config`'s census is red before and after this take.**
  `real_toml_manifest_census_rederives_expected_scope`
  (`crates/slicer-config/tests/registry_census_tdd.rs`) asserts **281** declared
  keys and the manifests declare **293**. Plain
  `cargo test -p slicer-config --test registry_census_tdd
  real_toml_manifest_census_rederives_expected_scope` reproduces **293 vs 281**
  with no xtask involvement, so the failure is not introduced by the injection
  change. It is **not fixed and not weakened** here: a self-captured census
  baseline is outside this ticket's question. A1's fix does make the package
  reachable through the gated entry point, so the red is now visible where it
  was previously hidden behind cargo's feature error.
- **`--no-run --summary` prints a false alarm.** A successful build-only run
  prints `(no test result: lines found — build may have failed before tests
  ran)` and then `VERDICT: PASS`. The verdict is correct (it follows the exit
  code); the message is misleading. Recorded, not fixed.
- **The ignored local mirror `CLAUDE.md` still carries the old line.** It is
  gitignored (`.gitignore` line 48); the tracked `AGENTS.md` is corrected.
  Other copies under `.local-artifacts/` and `tmp/` are captured artifacts and
  were deliberately not edited.
- **No wall or CPU claim.** This is harness correctness, so the map's 0.80 %
  floor, paired acceptance and KEEP/DROP rules do not apply; the take does not
  move the destination.

## Evidence files

- `summary.json` — the claims above in machine-readable form.
- `verify.py` — checkout-only re-derivation (static checks, plus narrow cargo
  runs; `--no-cargo` skips the runs; `--full` adds the end-to-end
  `cargo xtask test -p xtask --summary`).
- `REPRODUCE.md` — the exact command sequence.
