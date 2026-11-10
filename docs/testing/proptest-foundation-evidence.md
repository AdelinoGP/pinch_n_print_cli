# Property-testing foundation evidence

This records the executed pre-dependency G-BASE baseline and the G-HARNESS foundation evidence. The Windows job wiring is present, but an actual GitHub Actions result remains a closure gap. Raw command output, inventories, per-test logs, and status files are under `target/evidence/`; `target/` is Git-ignored, so preserve these local captures separately before cleaning or replacing the build directory.

**Provenance:** `source_revision=a204e7106c0a18790f64f3eec76cbc678ffea534` (`git rev-parse HEAD`); dirty paths: none (`git status --porcelain` and `git diff --stat` before the baseline); date: `2026-11-10T01:50:18-03:00` (`date -Iseconds`); toolchain: `1.96.0-x86_64-pc-windows-msvc` (`rustup show active-toolchain`); OS: `MINGW64_NT-10.0-26300 x86_64` on Windows (`uname -a`); `CARGO_TARGET_DIR`: unset, with workspace `target/` confirmed by Cargo metadata. Raw identity and status captures: `target/evidence/environment.txt`, `head.txt`, `status-before.txt`, `diff-stat-before.txt`, and `metadata-paths.txt`.

## G-BASE

### Package, target, feature, and runner inventory

`cargo metadata --no-deps --format-version 1` succeeded; raw JSON is `target/evidence/metadata-nodeps.json` and the extracted inventory is `target/evidence/inventory-summary.txt`. The following are Cargo **test** targets, not counts inferred from comments:

| Package / target | Source | Cargo `required-features` |
| --- | --- | --- |
| `slicer-ir` / `ir_validation_tdd` | `crates/slicer-ir/tests/ir_validation_tdd.rs` | none |
| `slicer-ir` / `entity_id_invariants_tdd` | `crates/slicer-ir/tests/entity_id_invariants_tdd.rs` | none |
| `slicer-sdk` / `test_support_mock_host_isolation_tdd` | `crates/slicer-sdk/tests/test_support_mock_host_isolation_tdd.rs` | `test` |
| `slicer-sdk` / `test_support_log_capture_round_trip_tdd` | `crates/slicer-sdk/tests/test_support_log_capture_round_trip_tdd.rs` | `test` |

Inspection of these source files found no crate-level `#![cfg(...)]` gates or individual `#[cfg(...)]`/`#[cfg_attr(...)]` attributes; see `target/evidence/baseline-source-gates.txt` and `target/evidence/baseline-all-cfg-search-status.txt`. `test_support` and `test_prelude` (`crates/slicer-sdk/src/lib.rs`) are gated by `any(test, feature = "test")`; SDK integration tests use the explicit `test` feature. Metadata identifies `xtask` as a `bin` target at `xtask/src/main.rs`, and package `pnp-cli`'s binary target as `pnp_cli` at `crates/pnp-cli/src/main.rs`.

Feature lists from the same metadata capture:

- `slicer-ir`: `default = []`, `std = []`.
- `slicer-sdk`: `test = []` (no `default` feature entry); its manifest also has a self dev-dependency enabling `test`.
- `slicer-core`: `default = []`, `host-algos = ["dep:rayon", "dep:boostvoronoi"]`, `perimeter-spatial-test-support = []`, and `voronoi-panic-regression = ["boostvoronoi/console_debug"]`.

`test_command` (`xtask/src/test.rs`) owns existing preflights, guest freshness/rebuild, CLI freshness, and test logging. `parse_test_options` and `build_test_arguments` (`xtask/src/test.rs`) handle the existing accelerated and summary flags and inject `slicer-core/host-algos` only for a relevant package selection when the caller did not choose features. The baseline commands below intentionally used plain narrow Cargo tests, not the xtask runner.

### Existing tests: executed before edits

Each command ran on Windows with `set -o pipefail` and `2>&1 | tee target/test-output.log`; that log was copied and read before the next test. All used Cargo's unoptimized test profile with debuginfo. The IR commands selected their default features with no additional features; the SDK commands explicitly selected `test`. The quoted lines are from the archived logs, and each Cargo and tee exit was zero (separate `*-status.txt` files alongside the logs).

| Command | Outcome and archived log | Exact libtest result |
| --- | --- | --- |
| `cargo test -p slicer-ir --test ir_validation_tdd` | PASS; `target/evidence/step1-ir-validation.log` | `test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s` |
| `cargo test -p slicer-ir --test entity_id_invariants_tdd` | PASS; `target/evidence/step1-ir-entity-id.log` | `test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s` |
| `cargo test -p slicer-sdk --features test --test test_support_mock_host_isolation_tdd` | PASS; `target/evidence/step1-sdk-mock-host-isolation.log` | `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s` |
| `cargo test -p slicer-sdk --features test --test test_support_log_capture_round_trip_tdd` | PASS; `target/evidence/step1-sdk-log-capture.log` | `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s` |

Executed test names, from those logs:

- `ir_validation_tdd`: `anchored_contracts_construct_round_trip_and_order`, `dangling_travel_anchor_rejected`, `flat_layer_contract_remains_unchanged`.
- `entity_id_invariants_tdd`: `entity_id_round_trips_through_serde`, `id_gen_is_strictly_monotonic`, `id_gen_no_collision_under_contention`, `unique_per_layer_and_resolvable`.
- `test_support_mock_host_isolation_tdd`: `_a_first_install_leaves_mesh_source_dirty`, `_b_second_test_sees_cleared_state_after_reset`.
- `test_support_log_capture_round_trip_tdd`: `log_warn_round_trips_through_capture_when_installed`.

**Compile/execution split:** isolated compilation time and process startup time were not measured. Cargo's build/preparation report, which can include compilation, linking and cache checks, was respectively `1m 00s`, `3.85s`, `1m 55s`, and `2.59s` for the four commands in the table; the separate libtest execution times are in the quoted result lines. The commands ran sequentially with differing incremental-cache state; these observations are not a performance comparison.

### Before dependency graphs and resolution

All the following captures succeeded. They describe the baseline, not the graph after the property harness is added:

1. `cargo tree -e normal,build,features` → `target/evidence/tree-workspace-before.txt` (default workspace normal/build/feature graph).
2. `cargo tree -p pnp-cli -e normal,build,features` → `target/evidence/tree-pnp-cli-before.txt` (default CLI normal/build/feature graph).
3. `cargo tree -p slicer-sdk -e normal,build,features` → `target/evidence/tree-sdk-default-before.txt` (default SDK graph).
4. `cargo tree -p slicer-sdk -e normal,build,features --features test` → `target/evidence/tree-sdk-test-before.txt` (SDK `test` graph).
5. `cmp -s target/evidence/tree-sdk-default-before.txt target/evidence/tree-sdk-test-before.txt` returned exit `0`: those captured graphs are byte-identical on this host.
6. The native SDK graph includes `slicer-core`'s `host-algos` feature, including `boostvoronoi` and `rayon`; this is the existing native dependency arrangement, not a property feature.
7. `cargo tree -p slicer-ir -e normal,build,dev,features` → `target/evidence/tree-ir-dev-before.txt`; its dev section includes `postcard`, `serde_json`, and `static_assertions`, and no `slicer-sdk`. This is the before graph for the later IR-to-SDK dev-edge comparison; its cost after the edge is not measured here.
8. `cargo xtask build-guests --list` → `target/evidence/guest-list-before.txt` and `target/evidence/guest-list-before.stderr`: the command listed 60 guests; the non-`cdylib` `witness` manifest is reported separately by discovery.
9. For every listed guest, `cargo tree --locked --manifest-path <manifest> --target wasm32-unknown-unknown -e normal,build,features` succeeded. Raw per-guest graphs and stderr are in `target/evidence/guest-graphs-before/`; exact commands, manifest/artifact paths, exit codes, and search results are indexed in `target/evidence/guest-graphs-before-index.json`, with a readable summary in `target/evidence/guest-graphs-before-summary.txt`.
10. Searching the workspace, CLI and both SDK graph captures with `rg -i 'proptest|openrscad|open-rscad'` returned no matches (`target/evidence/tree-search-status.txt` records ripgrep's no-match exit `1`). The per-guest graph scan also checked `open_rscad` and recorded no matches.
11. An exhaustive `rg --hidden --no-ignore -n -i --glob 'Cargo.toml' --glob '!**/target/**' --glob '!.git/**' 'proptest|openrscad|open-rscad|open_rscad' .` returned no matches; `target/evidence/manifest-search-exhaustive-status.txt` records exit `1`. The parsed lockfile inventory `target/evidence/lockfile-inventory.txt` also found no matching package names in the workspace, guest, or vendored lockfiles.
12. **Resolution:** `proptest` is absent at baseline; there is no selected Proptest version or Proptest MSRV to report yet. The future lock resolution and compatibility check remain open. The expectation to verify after implementation is that default CLI, SDK default/`test`, and discovered guest normal/build graphs still contain neither Proptest nor OpenRSCAD packages.

The inspected guest manifest and lockfile scope is enumerated in `target/evidence/guest-manifests.json` and `guest-manifests-summary.txt`, including each manifest's `[workspace]` sentinel and local lockfile, and `target/evidence/lockfile-inventory.txt` includes the root and vendor locks. The following are the actual guest directories whose graphs were captured (the first group uses `modules/core-modules/<directory>/wit-guest/Cargo.toml`; the second uses `crates/slicer-wasm-host/test-guests/<directory>/Cargo.toml`):

- Core modules: `arachne-perimeters`, `classic-perimeters`, `fuzzy-skin`, `gyroid-infill`, `infill-linker`, `layer-planner-default`, `lightning-infill`, `machine-gcode-emit`, `overhang-classifier-default`, `part-cooling`, `path-optimization-default`, `raft-default`, `rectilinear-infill`, `seam-placer`, `seam-planner-default`, `skirt-brim`, `support-surface-ironing`, `top-surface-ironing`, `traditional-support`, `traditional-support-planner`, `tree-support`, `tree-support-planner`, `wave-overhangs`, `wipe-tower`.
- Test guests: `anchored-events-roundtrip-guest`, `dispatch-layer-infill-postprocess-guest`, `dispatch-layer-path-optimization-guest`, `dispatch-layer-perimeters-guest`, `dispatch-layer-perimeters-postprocess-guest`, `dispatch-layer-slice-postprocess-guest`, `dispatch-layer-support-postprocess-guest`, `finalization-guest`, `finalization-mutation-roundtrip-guest`, `infill-postprocess-echo-guest`, `layer-infill-guest`, `lightning-portability-guest`, `path-optimization-multi-read`, `postpass-guest`, `preparation-lifecycle-anchored-guest`, `preparation-lifecycle-guest`, `preparation-pilot-infill`, `preparation-pilot-path`, `preparation-transport-path`, `preparation-visual-guest`, `prepass-guest`, `prepass-layer-planning-guest`, `sdk-finalization-guest`, `sdk-host-bridge-guest`, `sdk-infill-postprocess-view-guest`, `sdk-layer-infill-guest`, `sdk-layer-pathopt-guest`, `sdk-postpass-text-guest`, `sdk-prepass-guest`, `sdk-support-diagnostic-guest`, `support-anchored-reach-guest`, `whole-print-input-anchored`, `whole-print-input-infill`, `whole-print-input-path`, `whole-print-input-perimeters`, `whole-print-input-support`.

### Toolchain policy, freshness, and CI

- `Cargo.toml`'s `[workspace.package]` declares edition `2021` and `rust-version = "1.91.0"` for packages opting into those workspace values. Metadata reports edition `2021` but no declared `rust-version` for the three inventoried SDK, IR, and core packages; do not treat workspace inheritance as automatic. The manifest-wide parsed inventory is `target/evidence/msrv-inventory.txt`. No build with Rust `1.91.0` was performed.
- `rust-toolchain.toml` pins channel `1.96.0`, `rustfmt`, `clippy`, and target `wasm32-unknown-unknown`. A copy is `target/evidence/rust-toolchain.toml`; full `rustc -Vv`, `cargo -V`, and `rustup show active-toolchain` output is in `target/evidence/environment.txt`.
- `cargo xtask build-guests --check` initially returned exit `1` with fingerprint mismatches (`target/evidence/guest-freshness-check.log` and `guest-freshness-check-status.txt`). Following the staleness procedure, `cargo xtask build-guests` returned exit `0` and reported `built 60 guest(s)` (`target/evidence/guest-rebuild.log` and `guest-rebuild-status.txt`); a subsequent `cargo xtask build-guests --check` returned exit `0` (`target/evidence/guest-freshness-after-rebuild.log` and `guest-freshness-after-rebuild-status.txt`). The rebuild refreshed generated guest WASM artifacts; `git status --porcelain` afterwards was empty (`target/evidence/status-after-guest-rebuild.txt`). No guest-related test failure is being attributed here.
- Existing job IDs in `.github/workflows/ci.yml`: `fmt`, `docs-guard`, `clippy`, `test`, `dist-editions`; all use `ubuntu-latest` (`target/evidence/ci-job-inventory.txt`). There was no Windows properties job at baseline.

## G-HARNESS

- [x] Record measured routine and extended `H-IR-ANCHORS` pilot settings, bounds, compile/execution context, and pilot commands. — Measured on Windows, warm builds, seed 37, via `python target/evidence/step5-pilot.py --configuration <smoke|routine|extended> --name <label>` launching `cargo test -p slicer-ir --features property_testing --test property_ir proptest_ir_travel_anchors_membership -- --exact --nocapture` with a subprocess-local `PNP_PROPERTY_CONTEXT`. `smoke` (16 cases, bounds 8/16, limits 1/1/1, shrink 16): wall 0.4691594s (cargo prep 0.25s, libtest 0.01s), 7 valid / 9 dangling. `routine` (64 cases, bounds 32/64, limits 0/0/0, shrink 64): wall 0.9220788s (prep 0.45s, libtest 0.21s), 26 / 38. `extended` (1024 cases, bounds 256/512, limits 0/0/0, shrink 256): wall 8.5795951s (prep 0.46s, libtest 7.87s), 503 / 521. Zero local/global rejects and zero shrink callbacks in all production pilots; valid/dangling counts are diagnostics, not coverage assertions. Mutation shrink pilots (iteration limits 16/64/256): 4 callbacks each, retained `[1]`, callback-run times 311.7µs / 77.2µs / 75.4µs; shrink completion reported `null` (proptest 1.11 cannot identify it). Full table: `docs/testing/properties.md#measured-profiles`; raw contexts/logs `target/evidence/step5-*`.
- [x] Record actual Windows routine and extended `H-IR-ANCHORS` executions, results, and preserved logs/artifacts. — Both passed through the registry executor; see the run table and evidence limitations below.
- [x] Capture after-state default CLI, SDK default/`test`, workspace, and IR dev graphs and compare with G-BASE; record discovered guest coverage. — The captures and metadata-based guest probe are described below; no separate per-guest after-state graph files were captured.
- [x] Measure and record the dev-graph cost of the `slicer-ir` → `slicer-sdk` test edge against the before graph. — With the new dev-only edge (`slicer-sdk` with `property_testing`, hence Proptest) in the `slicer-ir` test graph, `cargo test -p slicer-ir --test ir_validation_tdd` measured Cargo preparation 32.47s / wall 32.828s versus the G-BASE recorded 1m 00s preparation. Differing incremental-cache states prevent attributing that difference to dependency cost — the cost is **unmeasured beyond this observation**. The edge is dev-only; normal/build isolation evidence appears below.
- [x] Record the selected Proptest version, lock resolution, feature capabilities, and verified MSRV compatibility. — Proptest **1.11.0** resolved in `Cargo.lock` (workspace-managed, `default-features = false` with `std`, `fork`, `timeout` retained), verified via `cargo info proptest@1.11.0` and `cargo tree`. Declared MSRV 1.85 against workspace `rust-version = 1.91.0` and pinned toolchain 1.96.0 (compatible). The limits of the after-state OpenRSCAD-name checks are described below.
- [x] Record replay and controlled failure-probe evidence, including diagnostic artifacts. — Same-seed concrete-sequence replay proven against artifact contents (identical `case.txt` pre-shrink sequences and retained failure input across two real child runs; iteration-bounded shrinking, no elapsed-time cutoff): `property_seed_replay_and_artifacts` (`xtask/src/properties.rs`) and `property_run_same_seed_case_sequence_is_reproducible` (`crates/slicer-sdk/tests/property_harness.rs`). Controlled failure probes (missing/mismatched context, override conflict, exhaustion, panic, artifact-write failure, fork crash) retain `context.json`/`case.txt`/`failure.json`/`observations.jsonl` under `target/property-testing/`; logs `target/evidence/step4a-*`, `step4b-*`. The registry-command `--seed 42` replay also passed, as recorded below.
- [x] Record the AC-N4 defect outcome: a named concrete regression and fix if confirmed, or an explicit no-defect-observed result. — **No defect observed** in `validate_travel_anchors` (`crates/slicer-ir/src/validation.rs`) during this slice. The synthetic accept-all substitute was rejected by `property_anchor_oracle_catches_accept_all` (`xtask/src/properties.rs`). No regression was manufactured.
- [x] Record the `properties-windows` job and failure-artifact wiring, and the availability of its CI result. — `.github/workflows/ci.yml` defines `properties-windows`, named `Properties (Windows)`, on `windows-latest` with a 60-minute timeout. It installs `wasm-tools`, builds guests, runs `cargo xtask test --summary --properties --tier routine`, and on failure uploads `target/test-output.log` and `target/property-testing/` as `properties-windows-failure`. **No GitHub Actions run was available in this environment:** job execution and branch-protection/required-check configuration are not verified and remain a closure gap, not a CI pass.

### Windows registry executions

These runs were reported on Windows with `1.96.0-x86_64-pc-windows-msvc`, warm builds, and `CARGO_TARGET_DIR` unset. They select `slicer-ir/property_testing`, target `property_ir`, and `H-IR-ANCHORS`. For each row, the command's terminal summary and its combined `target/test-output.log` were copied under `target/evidence/step7/` immediately after the run, before any later run could replace them.

| Command | Exit | Libtest result | Accepted cases | Summary / combined log |
| --- | --- | --- | --- | --- |
| `cargo xtask test --summary --properties --property H-IR-ANCHORS --seed 42` | 0 | `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s` | 64, controller `outcome: passed` | `target/evidence/step7/property-seed42.stdout.log` / `property-seed42.log` |
| `cargo xtask test --summary --properties --tier routine` | 0 | `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s` | 64, controller `outcome: passed` | `target/evidence/step7/property-routine.stdout.log` / `property-routine.log` |
| `cargo xtask test --summary --properties --tier extended` | 0 | `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.92s` | 1024, controller `outcome: passed` | `target/evidence/step7/extended-summary.log` / `property-extended.log` |

Each combined log contains the discovery `--list` passes, the exact `--exact --nocapture` child run, and one matching controller begin/end pair with `completion_eligible: true`. Run token and artifact directory per row: seed 42 → `target/property-testing/0e7c091edb72935739c8f933cbb2bf5f/H-IR-ANCHORS/`; routine → `target/property-testing/065bc44ad67b16131e28cb1a61dbc046/H-IR-ANCHORS/`; extended → `target/property-testing/8ee5f13fc1e63452ee1a3f4d05f83963/H-IR-ANCHORS/`. The seed `context.json` records `seed: 42`; the routine and extended runs resolved fresh OS-entropy seeds (recorded in their own `context.json`).

Provenance is run-specific. The seed and routine reruns record `source_revision=a862e0f92bc27d5693387f797dd2b3ebf15fae4b` with the Step 7 CI/USAGE/docs edits dirty; the extended run records the same revision with no dirty paths. Each context retains its `source_state_digest`; do not substitute the later graph-capture revision for an earlier run's provenance. Libtest times in the table are not whole-command wall times, and no whole-command wall time is retained in these archives.

Other captured checks after the review fixes: `cargo test -p xtask --bin xtask -- property_ --nocapture` returned `test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 204 filtered out` (`target/evidence/step7/fix-x-property.log`); `cargo test -p xtask --bin xtask -- test::tests --nocapture` returned `test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 200 filtered out` (`target/evidence/step7/fix-x-test-module.log`); `cargo test -p slicer-sdk --features property_testing --test property_harness -- --nocapture` returned `test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out` (`target/evidence/step7/fix-suite-s.log`). `cargo xtask check-literals` returned exit 0 and `0 violation(s)`; `cargo xtask check-test-quality --report` returned exit 0 and `0 finding(s)` (logs under `target/evidence/step7/`).

### After-state dependency evidence

At capture, `git rev-parse HEAD` returned `a862e0f92bc27d5693387f797dd2b3ebf15fae4b` (`target/evidence/step7/head.txt`); the tracked dirty paths were `.github/workflows/ci.yml` and `xtask/src/main.rs` (`target/evidence/step7/status-after-capture.txt`). Each command below redirected stdout and stderr to the named capture and returned exit 0. Each capture was then searched separately with `rg -n -i 'proptest|openrscad|open-rscad|open_rscad' <capture>`; the search exit and matching-line counts are shown, and the matching lines are retained in the corresponding `search-*-after.txt` file in the same directory.

| Graph command | Capture under `target/evidence/step7/` | Search exit / matching lines |
| --- | --- | --- |
| `cargo tree -e normal,build,features` | `tree-workspace-after.txt` | 1 / 0 |
| `cargo tree -p pnp-cli -e normal,build,features` | `tree-pnp-cli-after.txt` | 1 / 0 |
| `cargo tree -p slicer-sdk -e normal,build,features` | `tree-sdk-default-after.txt` | 1 / 0 |
| `cargo tree -p slicer-sdk -e normal,build,features --features test` | `tree-sdk-test-after.txt` | 1 / 0 |
| `cargo tree -p slicer-ir -e normal,build,dev,features` | `tree-ir-dev-after.txt` | 0 / 18, all Proptest lines |

`cmp -s target/evidence/step7/tree-sdk-default-after.txt target/evidence/step7/tree-sdk-test-after.txt` returned exit 0: these after-state SDK captures are byte-identical, as were the two G-BASE SDK captures. Before, the workspace/CLI/SDK scans reported no `proptest`, `openrscad`, or `open-rscad` hits; their baseline scan did not record an `open_rscad` search. After, all four search terms have no hits in those normal/build captures. The IR dev graph previously had no SDK or Proptest; it now contains Proptest 1.11.0, including the `std`, `fork`, and `timeout` features. A prior observation, not repeated in the Step 7 captures, found `cargo tree -p slicer-ir -e normal,build` free of `slicer-sdk` and `proptest` (acyclicity). These are graph observations, not a measurement of the dev edge's cost or a byte-for-byte comparison between before and after captures.

After-state guest coverage is the Cargo-metadata walk in `property_dependency_isolation` (`xtask/src/properties.rs`), not a separately captured per-guest `cargo tree` sweep. The test traverses normal/build metadata edges and uses selected-package Cargo trees to scope active names. The retained passing probe in `target/evidence/step7/property-tests-final.log` reports `property metadata: proptest 1.11.0, rust-version 1.85, workspace 1.91.0, toolchain 1.96.0` and `property guest metadata walk: 60 guests, 1 discovery skips, elapsed 31.7953297s`; `witness` was skipped because `[lib].crate-type` lacks `cdylib`. The probe's forbidden-name predicate checks `proptest` and names containing `openrscad`; unlike the search above, it does not separately check `open-rscad` or `open_rscad`. Thus those alternate guest-name spellings and individually preserved after-state guest graphs remain unverified. G-BASE, in contrast, retained a per-guest graph sweep and checked all four search terms.

## Deferred

Real SCAD-enabled builds and runs are not an executable configuration yet and are deferred. A SCAD-enabled dependency graph is a documented future expectation only; this baseline makes no SCAD feasibility or execution claim.
