# t114 — The e2e bucket pays ~10 s of module load per slice, and two modules own the wall

Route: [ticket 114](../../issues/114-e2e-bucket-latency.md).
Take at HEAD `7eeff93e` (2026-11-04). This is the **test-execution** axis that
continues [t108](../t108-dev-loop-latency/FINDINGS.md) (compile/preflight) and
[t109](../t109-harness-defects/FINDINGS.md) (harness defects), extended to the
`cargo test` execution side by the human's 2026-11-03 merge directive.

## Headline

**A debug-mode full slice is 85 % fixed cost.** Slicing `resources/calicat.stl`
through the 24 production core modules takes a median **11.732 s** wall; the
instrumented run reports **1.833 s** of actual pipeline work (validation 4 ms +
prepass 425 ms + per-layer 1174 ms + postpass 230 ms) and **10.011 s** of
module-load / startup / emit that is paid **once per `run_slice` or `pnp_cli`
invocation**. The live loader creates a fresh `WasmEngine` and compiles every
on-disk `.wasm` component **serially in a `for` loop**
(`load_live_modules_for_plan_manifest_first`, `crates/slicer-wasm-host/src/execution_plan_live.rs`),
with **no component cache** anywhere on the path.

The e2e bucket is built almost entirely from repeated full slices (one
`run_slice`/`pnp_cli` call per assertion family), so the bucket's wall is
dominated by that fixed cost, not by the assertions or the fixtures. Two modules
own most of it: **`lightning_migration_tdd` measured 1,411 s for 5 completed
tests plus one in flight before the sweep was stopped** — a linear extrapolation
to its 10 tests is ~39 min — and **`preparation_runtime_lifecycle_tdd` is the
ticket's own recorded 14-of-16-in-30-minutes module** (~34 min extrapolated).
Those two alone exceed the **40-minute** full-suite observation in the ticket.

A reverted probe that compiles the 24 components on 24 threads (same G-code,
byte-identical) removes only **2.431 s of the 11.732 s** (−20.7 %): wasmtime's
Cranelift compile already saturates the cores per component, so **parallelising
the loop is not the lever**. Reusing an already-compiled component set is.

## Table 1 — one slice, decomposed (calicat, debug, 24 core modules)

Plain `pnp_cli slice --model resources/calicat.stl --module-dir
modules/core-modules --no-default-module-paths`, quiet box (total CPU
0–8 % on 12 logical cores):

| arm | wall samples (s) | median (s) | pipeline phases (ms) | fixed cost (ms) |
| --- | --- | ---: | ---: | ---: |
| HEAD (serial compile) | 11.941 / 11.732 / 11.731 | **11.732** | 1,833 | **10,011** |
| probe (24-thread compile) | 8.806 / 9.335 / 9.301 | **9.301** | 1,816 | **7,168** |
| delta | | **−2.431 (−20.7 %)** | −17 | −2,843 |

`fixed = slice_complete.elapsed_ms − Σ(phase_complete.elapsed_ms)`; the
instrumented samples are 11,894 ms / 11,844 ms and 9,040 ms / 8,984 ms wall /
slice_complete. Both arms emit the **same G-code** — SHA-256
`d12cdfd10dab66a0010a0b5d5fca53d9387f4e96227ad5e74d0ec18197c0e260` — so the
probe is a load-path measurement, not an output change.

The probe is a Phase-1 pricing artifact and was **reverted** before the commit
(`git checkout -- crates/slicer-wasm-host/src/execution_plan_live.rs`). The
surviving production shape is the compiled-component **cache** the same code
path lacks, not the thread count: the probe still leaves **7.168 s** of fixed
cost, and a cached second slice would pay only the 1.816 s pipeline.

## Table 2 — module inventory, serial (one module per process, `--test-threads 1`)

Serial execution is the clean attribution arm: every test runs on a quiet box
and each module process is timed end to end. The first sweep stopped inside
`lightning_migration_tdd` once the module's dominance was established; the
second sweep covers the modules it had not reached.

| module | tests | wall (s) | note |
| --- | ---: | ---: | --- |
| `acceptance_gate_gaps_tdd` | 10 | 0.05 | pure-unit, no slice |
| `cube_4color_modifier_part_e2e_tdd` | 7 | 0.10 | metadata only |
| `typed_modifier_kind_tdd` | 3 | 1.13 | metadata only |
| `threemf_fixture_e2e_tdd` | 22 | 0.21 | model-IO, no slice |
| `threemf_subtypes_synthetic_e2e_tdd` | 9 | 0.00 | model-IO, no slice |
| `scenario_traces_tdd` | 13 | 0.00 | pure-unit, no slice |
| `mixed_density_internal_bridge_rejection_e2e_tdd` | 1 | 0.01 | one in-process slice |
| `slicer_report_html_tdd` | 7 | 0.30 | in-memory report |
| `modifier_support_type_family_e2e_tdd` | 2 | 10.28 | one slice |
| `calicat_internal_bridge_arbiter_e2e_tdd` | 1 | 11.83 | one `visual-debug` slice |
| `wedge_linked_infill_report_tdd` | 1 | 16.06 | one slice |
| `run_slice_api_tdd` | 2 | 16.33 | one slice |
| `infill_overlap_changes_gcode_tdd` | 1 | 23.84 | two `pnp_cli` slices |
| `calicat_internal_bridge_gating_e2e_tdd` | 1 | 24.61 | two slices, one process |
| `cube_painted_e2e_tdd` | 2 | 26.05 | two `pnp_cli` slices |
| `wave_overhang_bridge_fill_e2e_tdd` | 1 | 26.15 | two slices |
| `cube_painted_overrides_e2e_tdd` | 1 | 29.19 | two `pnp_cli` slices |
| `slicing_promotion_e2e_dispatch_regression_tdd` | 3 | 29.54 | two slices |
| `mm_real_fixture_gcode_tdd` | 3 | 34.08 | one slice per test (clean re-run) |
| `resolved_config_view_no_drop_tdd` | 4 | 35.21 | three slices |
| `modifier_infill_tdd` | 2 | 38.76 | two `pnp_cli` slices |
| `painted_fixture_parity_tdd` | 2 | 64.88 | two slices with paint |
| `modifier_support_territory_e2e_tdd` | 4 | 81.94 | several slices |
| `slicing_precision_integration_tdd` | 4 | 86.66 | two slices |
| `preparation_visual_capture_tdd` | 4 | 96.22 | ten captures |
| `slice_end_to_end_tdd` | 42 | 200.86 | **23 cached call sites over 3 keys** + 7 uncached determinism runs |
| `lightning_migration_tdd` | 10 | **1,411** (6 of 10) | killed; ~39 min extrapolated |
| `preparation_runtime_lifecycle_tdd` | 16 | **~1,800** (recorded) | ticket's own 14-of-16-in-30-min |

**Arithmetic.** Every module except the two whales sums to **854 s (14.2 min)**.
The measured/cited floor is therefore **854 + 1,411 + 1,800 = 4,065 s ≈ 68 min**,
and the linear extrapolations of the two whales put the full serial suite at
**≈ 88 min**. `slice_end_to_end_tdd` is the counter-example that proves the
mechanism: 42 tests cost 201 s because they are backed by **~10 `pnp_cli`
invocations** — 23 `cached_run` call sites collapsing to 3 keys
(`CoreModules`, `PartCoolingFiltered`, `TreeSupportFiltered`) plus 7
`run_pnp_cli_uncached` determinism runs — while
`calicat_internal_bridge_gating_e2e_tdd` costs 24.6 s for **two** slices that
share nothing. Every two-slice module lands at 24–29 s, i.e. **two separate
loads**; a cached second slice would land at ~13–15 s.

### Why two modules dominate

`lightning_migration_tdd`'s corpus tests each iterate `baseline.cases`
(**5 cases**) and run **both arms** (`wasm` and `native`) through
`run_slice_preparation_visual_capture` with **all layers** and **two taps** —
10 full captures per test, and at least three tests re-run the same corpus.
`preparation_runtime_lifecycle_tdd` issues **39 `run_slice` calls over 16
tests** with no in-process sharing. Both are coverage-distinct; neither is
duplicated by a narrower test.

## Table 3 — concurrency, measured with a disclosed confound

`libtest` defaults to `--test-threads = num_cpus` (12 here), and **each e2e test
already saturates the box**: a single `lightning` test process consumed **34.03
CPU-s in 5 wall-s (6.8 cores)**. Two processes of that shape on 12 cores
oversubscribe; twelve do so twelve times.

The cleanest paired sample is `mm_real_fixture_gcode_tdd::mm_painted_fixture_t0_t1`:
**23.25 s alone** (`--test-threads 1`, quiet) versus **200.622 s** under a
12-process `cargo nextest` run. That 8.6x is an **upper bound, not a clean
factor**: during that run a game held ~6.6 of 12 cores (`deadlock` ΔCPU 26.55 s
per 4 s) and Opera ~1.2, with total CPU at 59–76 %. The direction is robust —
12 concurrent tests, each with its own 12-thread pools, cannot help — but the
factor is not claimed.

`cargo nextest` (0.9.151, installed via `cargo binstall`, 6.8 s) is also **not
adopted**: it runs one process per test, so the in-process `slicer_cache` and
`wasm_cache` sharing that `cargo test` gives the bucket for free is lost, and
it needs a fresh `pnp_cli` exactly like `cargo test` (9 tests failed on the
stale-binary gate at 0.1 s in the first 12-way run). Its per-test durations are
useful, its isolation is not.

## Pruning audit — first pass was too strong; see AUDIT.md

The first pass concluded that no test was redundant with a narrower
unit/contract/integration test or with another e2e test, and that is still true
of the *behavioural claims*: the slice-running modules assert distinct contracts.
That pass was nevertheless **too strong**, because it did not ask whether a test
actually tests its name, or whether the same expensive observation is re-taken
by several tests. The adversarial audit in
[`AUDIT.md`](AUDIT.md) found decorative, fake, success-only and never-running
tests, plus **~15 minutes** of duplicate work.

**Adopted:** a never-running disk support-blocker test revived, both test-local
`scenario_2` fakes deleted (the one real `PaintSemantic` order claim moved to
`slicer-ir`), a lying override test renamed, an empty-tolerant pipeline test
tightened, the decorative fixture test removed, and
`modifier_infill_boundary_anchoring` merged into its identical sibling
(**38.76 s → 26.78 s, −11.98 s**). A precision merge removed a slice but landed
inside noise (86.66 → 86.71 s, not claimed).

**Not adopted:** the lightning corpus consolidation (**41 → 32** active
invocations, ~423 s estimated, every assertion preserved) is
[ticket 119](../../issues/119-lightning-corpus-consolidation.md); the prep
111 → 100 consolidation and the claim-hygiene merges are recorded in
`AUDIT.md`. The cost is not duplicate coverage but **~237 pipeline starts**, at
10.0 s of module load each ≈ 39.5 min; that is why the cache remains the
carrying fix.

## Lane/tier design — the honest version

The ticket's example gate was "default full suite under 10 minutes" or "fast lane
under 5 minutes, slow lane runnable in CI/nightly". The measured inventory says
a lane split **cannot** deliver either without excluding the bucket itself:

- the two whales are ~73 min (measured floor ~53 min);
- the next 16 slice-running modules are **832 s (13.9 min)**;
- what remains is **≈ 22 s** of pure-unit / model-IO modules
  (`acceptance_gate_gaps`, `cube_4color`, `threemf_fixture`,
  `threemf_subtypes`, `scenario_traces`, `report_html`, `typed_modifier`).

A "fast lane" of that 22 s would test almost nothing. A fast lane that keeps the
cheap slice modules (`mm` 34 s, `modifier_support_type_family` 10 s,
`run_slice_api` 16 s, `wedge_linked_infill_report` 16 s) is still ≈ 76 s, but it
stops being a *pipeline* lane and becomes a hand-maintained roster. The split is
recorded as the fallback, not adopted: it hides coverage instead of making the
coverage cheaper, and the production cache below makes both lanes minutes.

## Recommendation

**Graduate the production lever.** Add a compiled-component cache to the live
load path (keyed by the canonical module path + mtime + engine profile), so a
process that runs K slices pays one compile instead of K. Measured ceiling: a
cached slice costs the **1.816 s** pipeline instead of 11.732 s; a two-slice
module drops from ~24.6 s to ~13.7 s; and a full `cargo test` e2e process —
which runs all 178 tests in **one** process — pays one load. With ~145 slice
calls in the bucket, the modelled full-suite wall becomes
**≈ 10 s + 145 × 1.8 s ≈ 4.5 min** (estimate, labelled: the slice mix includes
tapped captures whose pipeline share is not separately measured). That reaches
the ticket's 10-minute gate and nearly its 5-minute fast-lane target without
deleting or ignoring a single test. The reverted 24-thread probe bounds the
cheaper sub-lever at **2.431 s** (−20.7 %) and is not sufficient alone.

The graduated ticket must also decide engine scope: the loader builds
`WasmEngine::with_profiling(profile)`, so a cache needs at least one engine per
profile flag, and tests that deliberately exercise raw `WasmEngine::new` /
`compile_component` keep their carve-out (`crates/slicer-runtime/tests/common/wasm_cache.rs`
documents the same discipline for the test-side helper).

## Ledger corrections

- The ticket's "93 tests passed in 40 minutes, no failures observed" is a
  **plain `cargo test`** run. Nine of its tests are `pnp_cli`-backed and fail in
  **0.1 s** on the stale-binary gate when `target/debug/pnp_cli.exe` predates
  `crates/*/src/**`; `cargo xtask test` builds it (`ensure_pnp_cli_fresh`,
  `xtask/src/test.rs`) but plain `cargo test` does not. The 40-minute baseline
  therefore did **not** include those nine tests' real cost (each ~12–29 s), so
  the true baseline is **larger**, not smaller.
- `cargo nextest` is not installed by default on this host; the take installed
  `0.9.151` to price it and did not commit any config for it.

## Evidence

- `summary.json` — every number above, machine-readable.
- `verify.py` — re-derives the arithmetic and structural claims from
  `summary.json`; `--self-test` mutates the set and requires rejection.
- `REPRODUCE.md` — the exact commands.
