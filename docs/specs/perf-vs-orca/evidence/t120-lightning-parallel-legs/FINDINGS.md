# Lightning test-leg concurrency: shared-machine screen

The `slicer-runtime` e2e bucket took **798.61 s before** and **700.16 s after**
the test-only change, a reduction of **98.45 s (12.33%)** in this one measured
pair. Both runs passed **172 tests, with one ignored**. The ignored recorder and
every individual test name and outcome match across the two runs.

The starting point was the handoff after ticket 118. In
`crates/slicer-runtime/tests/e2e/lightning_migration_tdd.rs`, for each corpus
case, `module_plan_matches_recorded_baseline` now joins its independently constructed
WASM and native runs; `fresh_consumers_emit_baseline_segments` schedules the
preparation-only, visual-consumer and cancellation legs independently.
`typed_projection_is_atomic_and_neutral` schedules its independent projection,
failure, empty-scene and G-code runs together, joining the two G-code results
before comparing them. All use the existing Rayon pool. Each leg still creates
its own per-run state and observer; cancellation has its own flag. The existing
compiled-component cache remains shared. No production code, test assertions,
test roster, or pipeline invocation was removed.

## Conditions and limits

- Windows, debug build, Ryzen 5 7600X (6 cores / 12 logical processors), default
  test and Rayon thread settings, warm component cache. Guests were rebuilt
  after `cargo xtask build-guests --check` reported stale fingerprints; the
  subsequent check exited 0. The CLI was rebuilt before the first run.
- The user chose to continue with the machine's current load. Five CPU samples
  before each run averaged approximately 12.9% and 13.3%, respectively, but
  active browsers consumed material CPU during both runs. One five-second
  process sample showed the four busiest Opera processes using roughly 1.2
  cores in the baseline and 2.0 cores in the comparison. These samples do not
  characterize the full run; no idle-machine or statistically established
  speedup is claimed.
- One run per arm. The handoff's separate **755.12 s** quiet-box measurement
  is historical context, not the paired baseline for the percentage above.
  The 6–8 minute projection was not reached in this screen. Cold-cache
  performance and other test buckets were not measured.
- The baseline used `cargo test -p slicer-runtime --test e2e`; acceptance
  used `cargo xtask test --summary -p slicer-runtime --test e2e`. The latter
  appends the repository's unrelated Arachne quarantine skip; neither e2e
  roster contains that test. Both figures are libtest's bucket time, excluding
  compilation and preflights. The acceptance command took 721.96 s including
  its preflights and test-target rebuild.

## Correctness and gates

The calicat G-code SHA-256 was
`d12cdfd10dab66a0010a0b5d5fca53d9387f4e96227ad5e74d0ec18197c0e260` before and
after, matching the ticket-118 reference. The gated e2e run, workspace clippy
with `--all-targets -D warnings`, `check-literals`, and
`check-test-quality --report` passed. No baseline fixture was changed.

`before.log` and `after.log` retain the test-result rows and libtest summaries
from the two runs; complete local outputs are under `target/` when available.
`summary.json` records the measurements and samples. `verify.py --self-test`
checks the recorded rosters, result counts, arithmetic, and reference hash and
rejects six mutations. This validates the reduced evidence's consistency; it
does not reproduce the runs or independently establish the recorded timings.

Ticket 119's capture sharing remains open. Cross-test prepass/preparation reuse
still requires a separate Phase-1 screen of ownership, isolation and lifecycle
semantics before any production cache is attempted.
