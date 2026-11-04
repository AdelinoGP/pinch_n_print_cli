# E2E bucket latency: the full runtime e2e suite is the dev-loop bottleneck

Type: task
Label: wayfinder:task
Status: open
Assignee: (unclaimed)
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
