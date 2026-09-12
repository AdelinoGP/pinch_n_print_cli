# Controlled Perimeter Builds

This document is the user-facing preparation guide for a **controlled perimeter build**
and its exact-spatial-query acceptance campaign.  The
acceptance runner is
`resources/perimeter-acceptance/run-acceptance.ps1`; it never builds an
artifact and it never commits a result automatically.

## Controlled-build policy grammar

The policy is fail-closed.  A controlled invocation is valid only when every
identity and mode rule below is satisfied:

```text
toolchain rustc 1.96.0 commit ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96
host x86_64-pc-windows-msvc
LLVM 22.1.2
targets x86_64-pc-windows-msvc wasm32-unknown-unknown
mode ordinary | accelerated
```

The driver re-reads and verifies the actual compiler identity for every
controlled session.  A version range, an unverified `rustc` on `PATH`, or an
unexpected host/LLVM/target identity is rejected.  Re-verification is
fail-closed: a rejected child compiler or probe cannot be hidden by a parent
build that happens to continue.

### Ordinary and accelerated modes

Ordinary builds use the legacy perimeter path.  They do not receive the
reserved acceleration cfg and are the reference route for exactness.

Accelerated entry points are explicit: pass `--accelerated` to
`cargo xtask build-guests`, `cargo xtask dist`, or `cargo xtask test`.  The
controlled driver injects `pnp_perimeter_spatial_accelerated` only for the
audited canonical core compilation (and its intended core unit-test
compilation).  A build script, a dependency, or an ordinary invocation cannot
grant that cfg.

Test-support features are not part of production artifacts.  Instrumented,
fuel, profile, and capture builds are attribution or correctness support, not
acceptance timing artifacts.  Host and guest artifacts must be kept in
mode-aware isolated namespaces; an ordinary artifact cannot satisfy an
accelerated lookup, and an accelerated artifact cannot silently satisfy an
ordinary lookup.

### Freshness and artifact identity

`cargo xtask build-guests --accelerated --check` validates the requested
accelerated mode, its policy/toolchain identity, the embedded WIT identity,
and lock convergence.  The exit meanings are:

| exit | meaning |
| ---: | --- |
| 0 | every requested artifact is fresh and the identity is valid |
| 1 | at least one artifact is stale, or guest lock convergence is required |
| 3 | the checker could not form an opinion, such as unreadable canonical WIT or missing `wasm-tools` |

An accelerated `--check` never validates an opposite-mode artifact.  A clean
ordinary artifact is therefore not evidence that the accelerated artifact is
fresh, and vice versa.  Rebuild the stale mode and repeat the check before
running acceptance.  The same rule applies to host snapshots: provenance must
name the actual mode, policy, compiler, guest set, and module directory used
by both variants.

### In-tree vs fingerprint staleness

`build-guests --check` judges content fingerprints of the shared-target
artifacts under `target/guests/`; it never compares mtimes.  The
integrated-parity harness
(`crates/slicer-runtime/tests/common/integrated_parity_harness.rs::assert_guest_freshness`)
judges the staged **in-tree** copies
(`modules/core-modules/*/*.wasm`, gitignored) by newest-source mtime.  A
clean `--check` (exit `0`) together with a
`guest artifact is stale: newest-source mtime is newer than artifact mtime`
test panic is therefore not a contradiction: the fingerprint is current but
the staged in-tree copy predates its sources (measured 2026-09-12:
`--check` exit `0` while the in-tree `classic-perimeters.wasm` was ~5.6 h
older than its `src/lib.rs`).  Remedy: `cargo xtask build-guests --force`
(refreshes both the shared target and the staged in-tree copies), then
re-run the failing test.

### Toolchain update procedure

The allowlist (`xtask/src/rustc_driver.rs`: `ALLOWED_RELEASE`,
`ALLOWED_COMMIT_HASH`, `ALLOWED_HOST`, `ALLOWED_LLVM_VERSION`) intentionally
fails closed on any rustup update.  To adopt a new toolchain:

1. Run `rustc -vV` and record release, commit hash, host, and LLVM version.
2. Update the four constants and the policy grammar block above to the new
   identity.
3. Re-run the packet-254 verification set: `cargo test -p xtask --bin xtask --
   accelerated_`, both `build-guests --check` modes, and the AC-1/AC-3N/AC-N2
   suites.  The `powi(2)`-lowers-to-multiply premise in `design.md`
   Architecture Constraints must be re-verified for the new compiler before
   any accelerated acceptance run.
4. Never widen the allowlist to a range: one exact identity at a time.

## Acceptance runner modes

`-DryRun` performs no corpus lookup, build, process execution, or timing.  It
creates six synthetic cells, runs the validator, and prints the JSON protocol.
Its synthetic rows intentionally include an overlapping range and an
exactness failure, so a dry run can never vacuously recommend `KEEP`.

Single-cell mode takes `-Workload`, `-ExpectedGenerator classic|arachne`, and
optionally `-CorpusRoot`.  Campaign mode takes `-Campaign` and executes the
six cells serially, with every cell in a fresh PowerShell process.  The runner
checks corpus, executable, module, and reference artifacts before invoking
the benchmark.  A missing input emits
`missing-artifact: <first-missing-path>` and exits nonzero; it never falls
back to synthetic timings.

The default executable is `target/release/pnp_cli.exe`, and the default module
directory is `target/release/modules`.  The baseline and candidate paths can
be supplied explicitly with `-BaselineExePath`, `-CandidateExePath`,
`-BaselineModuleDir`, and `-CandidateModuleDir` (or their corresponding
`PNP_PERIMETER_*` environment variables).  The coordinator builds those
snapshots before invoking the runner; the runner does not build them.

The output is written to `target/perimeter-acceptance/summary.json` and is
also printed to stdout.  Its top-level fields are `status`,
`automatic_commit`, `threads`, `samples_per_cell`, `warmup_per_cell`, and
`cells`.  `automatic_commit` is always `false`.  Each cell contains
`workload`, `generator`, `expected_generator`, `exactness_passed`,
`cpu_separated`, `wall_separated`, `baseline`, `candidate`, `cpu_ratio`,
`wall_ratio`, `generator_marker`, degraded/non-fatal/fatal counters for each
variant, and `decision`.  `baseline` and `candidate` each contain
`cpu_samples`, `wall_samples`, and the retained `rows`.  Ratios are the
candidate median divided by the baseline median; lower than one is favorable.

## Corpus

The campaign corpus is user-prepared and deliberately untracked.  Its default
root is `tmp/rtree_query_corpus/`.  Do not create a substitute corpus in the
repository to make the acceptance command pass.  The canonical layout is:

```text
tmp/rtree_query_corpus/
  supports-off-benchy/
    model.stl
    classic.json
    arachne.json
    reference-classic.gcode
    reference-arachne.gcode
  tree-support-benchy/
    model.stl
    classic.json
    arachne.json
    reference-classic.gcode
    reference-arachne.gcode
  tree-support-base/
    model.stl
    classic.json
    arachne.json
    reference-classic.gcode
    reference-arachne.gcode
```

The runner also accepts the equivalent `config-<generator>.json`,
`config/<generator>.json`, or `.ini` config spelling, and a
`reference/<generator>.gcode` directory.  The documented spelling is
preferred because the first missing path is deterministic.  Reference G-code
is precomputed under the same supported configuration and is checked before
measured rows are retained.

### Campaign protocol

There are six cells: supports-off Benchy, tree-support Benchy, and the
original tree-support base, each with Classic and Arachne.  Each cell uses 12
threads, one excluded warmup per variant, and four retained measured samples
per variant.  The measured order is `ABBA` followed by `BAAB`, where A is the
baseline host-plus-guest snapshot and B is the candidate snapshot.  No sample
is selectively discarded for load; CPU and whole-slice process wall remain
separate metrics.

Exactness is checked first against the precomputed reference output for both
variants.  A failed exactness check is `DROP`, and its timings are not
retained as acceptance evidence.  A generator marker or config disagreement,
fatal error, or change in a known degraded/non-fatal status is also `DROP`.
Identical pre-existing degraded/non-fatal counts do not reject a cell.

`KEEP` requires strict separation in every cell and metric:

```text
max(candidate CPU samples)  < min(baseline CPU samples)
max(candidate wall samples) < min(baseline wall samples)
```

An overlap or touching range in either metric is `inconclusive`, never
`KEEP`, and the campaign stops without an automatic retry.  A disjoint but
non-favorable range is `DROP`.  A campaign stops at the first failed cell;
unrun cells are recorded as `not-run` in `summary.json`.  No path in the
runner performs an automatic commit.

### Preparation and verification

Build and freshness-check the ordinary and accelerated snapshots with the
controlled driver first.  Then verify the harness copy and dry-run protocol:

```powershell
cmp resources/perimeter-acceptance/run_bench.ps1 tmp/alloc-bench/run_bench.ps1
pwsh -NoProfile -File resources/perimeter-acceptance/run-acceptance.ps1 -DryRun
pwsh -NoProfile -File resources/perimeter-acceptance/run-acceptance.ps1 -Campaign
```

The last command is intentionally fail-closed when the user-prepared corpus
is absent.  Its first missing-artifact line is evidence that no fabricated
timing campaign was attempted.
