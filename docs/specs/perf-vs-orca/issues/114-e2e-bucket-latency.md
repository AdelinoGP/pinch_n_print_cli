# E2E bucket latency: the full runtime e2e suite is the dev-loop bottleneck

Type: task
Label: wayfinder:task
Status: resolved (2026-11-04; Phase-1 screen — the bucket is 85 % per-slice
module load, two modules own ~84 % of its wall, and **no deletion or lane split
recovers the dev loop**; the carrying shape is a compiled-component cache in the
live load path, graduated as [ticket 118](118-compiled-component-cache.md). An
**adversarial audit postscript** corrects the first pruning claim: eight
claim-fix/merge changes were adopted, one measured **−11.98 s**, and the
lightning corpus consolidation is graduated as
[ticket 119](119-lightning-corpus-consolidation.md). See ## Answer and
## Postscript — adversarial audit)
Assignee: 2026-11-04 OpenCode wayfinder session — the human said "continue the
perf vs orca map" (no ticket named); the frontier re-derivation at HEAD
`7eeff93e` found 114/115/116/117 open and unclaimed, so the first frontier
ticket in order, 114, was claimed.
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: the 2026-11-03 merge validation (human-directed dev-loop
scope, continuing the [t108](108-dev-loop-latency-preflight-concurrency.md) /
[t109](109-xtask-host-algos-injection-and-narrow-run-example.md) axis).

## Question

Why does the `slicer-runtime` e2e test bucket exceed **40 minutes** on this
host, which tests dominate, and what is the cheapest change that brings a normal
full-suite run back into a usable dev-loop window **without hiding a real
failure or narrowing coverage**?

This is the **test-execution** axis, distinct from t108's **compile/test-loop**
axis. t108 measured the compile side (first codegen of the `contract` bucket
**994 s**, incremental **17.26 s**, relink-only **12.8 s**, bucket execution
**99.8 s**) and left five levers unpriced: `cargo-nextest`, `sccache`, cold-build
`debug`, cold-build linker, and the CI job graph. This ticket owns what happens
*after* the binaries are built: the e2e bucket's own wall time.

## Observed facts (2026-11-03, this host)

- `cargo test -p slicer-runtime --test e2e` was killed at **40 minutes** with
  **93 tests passed**, **25** "running for over 60 seconds" notices, and **no
  failures observed**.
- The `preparation_runtime_lifecycle` module alone: **14 of 16** tests completed
  within **30 minutes**, each crossing the 60-second notice; no failures observed
  before the filter run timed out.
- Same host, for contrast: `slicer-runtime` contract **313 tests / 42 s**; unit
  **98 / 108 s**; executor **238 / ~408 s**; integration **380 / ~1151 s**.
- The full `cargo xtask test --workspace` cannot complete in a normal working
  session because the e2e bucket alone is the long pole. This is **execution**
  time, not compile time.
- The slow tests are real coverage (preparation lifecycle, full-slice e2e), so
  deletion must be evidence-led rather than a blanket cut. The two
  archive-dependent pilot tests were removed separately because their fixture
  cannot exist on a fresh checkout; that is the precedent, not the general case.

**Human ruling (2026-11-03):** cutting/deleting e2e tests is **fully in scope**
for this ticket. The earlier "no deletion without a ruling" constraint is
lifted; pruning is an expected lever, not a last resort. Deletions still carry
the coverage rationale below. The `preparation_runtime_lifecycle_tdd` module is
the first inventory target: **16 tests, 14 completed in 30 minutes**, each over
the 60-second notice.

## Scope and method

1. **Inventory per-test wall time** for the e2e bucket. Use a harness timer, or
   price `cargo-nextest` against t108's **5 s/invocation** floor. Rank the top
   20 tests and attribute each to:
   - actual slicing/model work;
   - per-test setup (engine/module load, temp trees, verifier compilation);
   - serialization or global locks (shared caches, temp directory names);
   - fixed sleeps or timeout waits.
2. **Price the obvious levers on the top tests**, measured rather than assumed:
   shared engine/cache reuse; smaller model/slice fixtures; `--test-threads`
   (including whether the default is actually parallel for this bucket);
   process-per-test isolation (`cargo-nextest`); moving the slowest tests to a
   separate `slow` lane or `#[ignore]`-by-default suite.
3. **Pre-declare a target and a gate**, for example "default full suite under
   10 minutes" or "fast lane under 5 minutes, slow lane runnable in CI/nightly".
   The gate is paired wall time on the same tests with no coverage loss and no
   weakened assertions.
4. **Separate the axes.** Keep this ticket on execution time; do not re-open
   t108's compile levers or the CI job graph here.
5. **Prune.** Identify tests that are redundant with a narrower
   unit/contract/integration test, environment-dependent (the removed pilot
   archive tests are the precedent), duplicated across fixtures, or asserting
   scaffolding rather than product behavior. For each deletion record: the test,
   its measured cost, the behavior it covered, and the surviving test(s) that
   cover the same behavior. Delete only where that coverage map is non-empty;
   never delete the only coverage of a distinct path.

## Stop rule

Phase-1 inventory, lever pricing and pruning. **The human authorized e2e test
deletion on 2026-11-03**, so pruning is an expected outcome, not a last resort.
Every deletion must carry the coverage rationale and be recorded in the ticket.
Do not weaken assertions on surviving tests to obtain a pass, and do not delete
the only coverage of a distinct path. A lever that only moves gate overhead (not
the top tests) is recorded, not adopted. No production edit.

## Deliverable

A ranked inventory, a lever table with measured deltas (or honest bounds), a
**pruning list with the coverage map**, a proposed lane/tier design, and a
recommendation. Each deleted or moved test is named with its measured cost and
the behavior that remains covered.

## Evidence starts

- The 2026-11-03 merge-validation logs (`target/test-output.log`,
  `target/test-e2e-prep.log`) while they still exist; re-derive if gone.
- [t108 dev-loop latency](../evidence/t108-dev-loop-latency/FINDINGS.md) and
  [t109 harness defects](../evidence/t109-harness-defects/FINDINGS.md).
- `crates/slicer-runtime/tests/e2e/main.rs` and the e2e module list.
- `docs/22_test_quality.md` for what may be skipped/ignored and how to record it.

## Answer

**Verdict: Phase-1 screen — the bucket's wall is fixed per-slice module load,
not test count, and no test-side change reaches a usable default window.** The
carrying shape is a **compiled-component cache** in the live load path,
graduated as [ticket 118](118-compiled-component-cache.md). No test was deleted,
moved to an ignored lane, or had an assertion weakened in this take.

### What the wall is made of

One debug-mode slice of `resources/calicat.stl` through the 24 production core
modules costs a median **11.732 s** wall. The instrumented run splits it:

| component | measured |
| --- | ---: |
| validation phase | 4 ms |
| prepass phase | 425 ms |
| per-layer phase | 1,174 ms |
| postpass phase | 230 ms |
| **pipeline total** | **1,833 ms (15 %)** |
| **module load + startup + emit** | **10,011 ms (85 %)** |

The loader creates a fresh `WasmEngine` and compiles every on-disk `.wasm`
component **serially inside the binding loop** in
`load_live_modules_for_plan_manifest_first`
(`crates/slicer-wasm-host/src/execution_plan_live.rs`), with no cache anywhere
on the path. Every `run_slice` / `pnp_cli` call pays it again: a two-slice
module costs 24–29 s, and `slice_end_to_end_tdd`'s 42 tests cost 201 s because
they are backed by **~10 `pnp_cli` invocations** (23 `cached_run` call sites over
3 keys + 7 uncached determinism runs) instead of 42 — the same work collapsed.

A reverted 24-thread compile probe (byte-identical G-code) removed only
**2.431 s (−20.7 %)**, so **parallelising the loop is not the lever**; reusing
an already-compiled set is.

### Which tests dominate

Serial module inventory, 178 tests covered:

- **`lightning_migration_tdd` — 1,411 s for 5 completed tests + 1 in flight**
  (sweep stopped; ~39 min for its 10 tests). Its corpus tests each iterate
  **5 cases × 2 arms** through `run_slice_preparation_visual_capture` with all
  layers and two taps — 10 full captures per test, re-run by several tests.
- **`preparation_runtime_lifecycle_tdd` — the ticket's own 14-of-16-in-30-min
  observation** (~34 min extrapolated); 39 `run_slice` calls over 16 tests with
  no in-process sharing.
- Everything else sums to **854 s (14.2 min)**; the largest remainder is
  `slice_end_to_end_tdd` at 201 s (already cached), then
  `preparation_visual_capture_tdd` 96 s, `slicing_precision_integration_tdd`
  87 s, `modifier_support_territory_e2e_tdd` 82 s,
  `painted_fixture_parity_tdd` 65 s.

The measured/cited floor is **4,065 s ≈ 68 min**; the linear extrapolations put
the full serial suite at **≈ 5,263 s ≈ 88 min**. The two whales are **83.8 %**
of it. That is the >40-minute observation.

### Pruning audit — nothing is safely deletable

The human authorized deletion conditional on a non-empty coverage map. The
audit found **no test redundant with a narrower unit/contract/integration test
or with another e2e test**: the slice-running modules assert distinct contracts
(routing, determinism, label/direction, E-mode, capture lifecycle), and the
cheap 42-test `slice_end_to_end` module is already the shared-work pattern. The
two archive-dependent pilot tests were removed on 2026-11-03 for a
fixture-cannot-exist reason, which does not generalise. Deleting distinct
coverage to buy wall time is the trade the stop rule forbids.

### Lane split — not a viable substitute

A fast lane can only be built by excluding **every** slice-running module: the
whales plus the 832 s remainder, leaving ≈ 22 s of pure-unit/model-IO tests.
That hides coverage instead of making it cheaper, so it is recorded as the
fallback, not adopted.

### Concurrency and nextest

`libtest` defaults to 12 threads and each test already saturates the box (one
`lightning` test process used **6.8 cores** over 5 s), so the default
oversubscribes. `mm_real_fixture_gcode_tdd::mm_painted_fixture_t0_t1` measured
**23.25 s alone vs 200.622 s** under 12 concurrent processes — an **upper
bound**, not a clean factor, because a game held ~6.6 cores during that arm.
`cargo nextest` (0.9.151) is **not adopted**: it runs one process per test, so
it discards the in-process `slicer_cache`/`wasm_cache` sharing `cargo test`
gives the bucket for free.

### Ledger correction — the 40-minute baseline was under-counted

Nine `pnp_cli`-backed e2e tests fail in **0.1 s** on the stale-binary gate when
`target/debug/pnp_cli.exe` predates `crates/*/src/**`; plain `cargo test` does
not build it, while `cargo xtask test` does (`ensure_pnp_cli_fresh`,
`xtask/src/test.rs`). The 2026-11-03 "93 passed in 40 minutes, no failures"
observation therefore **excluded those nine tests' real cost** (each 12–29 s),
so the true baseline is larger than 40 minutes.

### Evidence

[`evidence/t114-e2e-latency/`](../evidence/t114-e2e-latency/FINDINGS.md) —
`verify.py` exits 0 checkout-only (29/29 checks) and `--self-test` rejects
11/11 negative controls, including the probe-left-in-tree and
cache-appears-in-loader mutations.

## Postscript — adversarial audit (2026-11-04, same day)

**The "no test is safely deletable" claim in the Answer was too strong.** A
five-agent adversarial audit of all 28 e2e modules (three reviewers, two
explorers; `docs/22_test_quality.md` taxonomy) found tests that did not test
their names, a test that had never run, and ~11 minutes of repeated
observations. The full per-test tables and coverage maps are in
[`evidence/t114-e2e-latency/AUDIT.md`](../evidence/t114-e2e-latency/AUDIT.md).

**Adopted and verified in this take (all assertions preserved):**

1. Deleted `cube_painted_e2e_tdd::painted_3mf_fixture_is_committed` — a
   decorative `path.exists()` test whose sibling re-asserts the same path.
2. Deleted `scenario_traces_tdd::scenario_2_higher_paint_order_wins_for_custom_overlap`
   and `::scenario_2_equal_paint_order_conflicting_values_are_fatal` — both
   asserted on test-local vectors; `execute_region_mapping_with_cap` was never
   called. Production ordering/duplicate-key coverage stays in
   `crates/slicer-core/tests/algo_region_mapping_tdd.rs` and
   `crates/slicer-runtime/tests/integration/region_mapping_tdd.rs`.
3. Moved the one real `PaintSemantic` D6 order assertion to
   `crates/slicer-ir/tests/ir_tests.rs::paint_semantic_derived_ordering_ranks_material_first`
   (passes).
4. Renamed `cube_painted_overrides_e2e_tdd::paint_config_override_visibly_differs_gcode`
   to `::paint_config_override_parses_cleanly_on_the_live_path`; the body only
   asserts clean parsing, and the module doc says the G-code diff is
   intentionally not asserted.
5. **Repaired a test that never ran:**
   `threemf_fixture_e2e_tdd::support_blocker_emits_paint_regions_from_disk`
   now probes `bridge_support_enforcers.3mf` (3 `support_blocker` parts), so the
   disk-path segmentation claim is actually exercised (passes).
6. Tightened `slice_end_to_end_tdd::slice_e2e_real_pipeline_produces_gcode`:
   empty G-code now fails instead of skipping the first-line check.
7. Merged `modifier_infill_boundary_anchoring` into
   `modifier_infill_two_densities` (same fixture, same CLI args, identical
   slice): **38.76 s → 26.78 s (−11.98 s, −30.9 %)**.
8. Merged `slicing_precision_integration_tdd::legacy_slice_structural_invariants`
   into `legacy_precision_run_is_deterministic` (one slice removed, every
   assertion preserved): 86.66 s → 86.71 s, i.e. **below noise and not claimed**.

**Why the 88 minutes survive deletion.** The bucket performs **~237 pipeline
starts** for 168 tests; at the measured **10.0 s** fixed module load that is
**~39.5 min of pure loading**, plus ~1.8 s of pipeline per start, plus the
tapped captures. Perfect deduplication (~11 min) leaves ~77 min. The carrying
fix is still the compiled-component cache
([ticket 118](118-compiled-component-cache.md)).

**Graduated, not executed here:**
[ticket 119](119-lightning-corpus-consolidation.md) owns the largest module
(`lightning_migration_tdd`, **41 → 32 active invocations**, ~423 s estimated,
every assertion preserved), including five false-green fixes the audit found
(vacuous `windows(2)` sharing, a local-Boolean "participation" assertion, an
`unwrap_or(0)` cleanup check that passes without a disposal record, an
own-owner "foreign owner" claim, and a zero-slice artifact test that belongs in
contract scope). The remaining merges (42→9 in `slice_end_to_end` without wall
change, painted/cube ~55 s, `infill_overlap` ~24 s, wedge report ~16 s) are
recorded in the audit for the next e2e take. One audit proposal was
**falsified on inspection**: `preparation_only_uses_production_prefix` cannot
merge into `capture_is_semantically_neutral`, because the latter's captures are
full slices and the preparation-only boundary would be lost.

**Postscript 2 — the milestone was met the same day.** After this audit the human
set a steep gate: the whole e2e bucket at most 15 minutes on this PC. [Ticket
118](118-compiled-component-cache.md) was executed instead of deferred: the
process-global compiled-component cache, wasmtime's validated disk cache,
per-layer capture parallelism and corpus-case parallelism bring the full bucket
to **755.12 s (12.59 min), 172 passed / 1 ignored**, under the gate with 2.41 min
of headroom. The measured module walls are in
[`evidence/t118-component-cache/`](../evidence/t118-component-cache/FINDINGS.md).
The remaining corpus consolidation ([ticket 119](119-lightning-corpus-consolidation.md))
is now claim hygiene, not latency.
