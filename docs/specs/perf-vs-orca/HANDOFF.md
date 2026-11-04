# Handoff — pipeline runtime speed (e2e dev loop)

Written 2026-11-04, after [ticket 118](issues/118-compiled-component-cache.md)
met the 15-minute milestone. Scope: the **`slicer-runtime` e2e bucket**
(`cargo test -p slicer-runtime --test e2e`). The slice-throughput work is a
different axis; see the [map](map.md) for that.

## Mission

Keep cutting the full-bucket wall. The current gate is **755.12 s (12.59 min),
172 passed / 1 ignored**, on an idle 12-core Windows box, warm cache, default
12 test threads. The human's bar was 15 minutes and is met; the next target is
whatever the next measured critical path allows. **Do not trade coverage or
assertion strength for wall time.**

## Start here

1. Confirm the gate on a quiet box: `cargo test -p slicer-runtime --test e2e`
   (expect ~755 s, 172 passed / 1 ignored). If it is far off, the disk cache is
   cold (one ~15 s compile) or the box is loaded — fix that before measuring.
2. Read the critical-path table below. The next change should target
   `module_plan` (373 s) and `typed_projection` (345 s), not the aggregate.
3. Do move 1: parallelize the independent legs inside those two tests, then
   re-run the **full bucket** (a module-only run will not show the wall).
4. If you need a bigger structural lever after that, move 3 (prepass/preparation
   reuse) is the next real one; design it as a Phase-1 screen before editing.

## What landed (commit `a3068536`)

| lever | file | effect |
| --- | --- | --- |
| Process-global compiled-component cache, one engine per `profile`, keyed by `(profile, canonical path, mtime, len)` | `crates/slicer-wasm-host/src/component_cache.rs` | calicat slice **11.732 s → 2.813 s warm**; failures never cached; per-run pools/stores unchanged |
| Loader uses the shared engine + cached components | `crates/slicer-wasm-host/src/execution_plan_live.rs` | every in-process `run_slice` after the first reuses the compiled set |
| wasmtime validated **disk** cache (15 MB, `%TEMP%/pnp-wasmtime-cache-v1`) | `crates/slicer-wasm-host/src/instance.rs`, `Cargo.toml` (`wasmtime` feature `cache`) | separate `pnp_cli` processes hit (2.813 s warm vs 15.424 s cold) |
| Capture layers use rayon (was a serial `for`) | `crates/slicer-runtime/src/layer_executor.rs` (`execute_captured_stages_with_support_tools`) | same parallelism as the ordinary path; first failing layer's error kept deterministic |
| Lightning corpus cases run in parallel | `crates/slicer-runtime/tests/e2e/lightning_migration_tdd.rs` (`module_plan`, `fresh_consumers`) | 5 independent cases overlap |

Env: `PNP_DISABLE_WASMTIME_CACHE=1` disables the disk cache;
`PNP_WASMTIME_CACHE_DIR` relocates it.

## Where the time goes now

Measured module walls (serial, warm) and the full bucket:

| item | wall | note |
| --- | ---: | --- |
| full bucket | **755.12 s** | 172 passed / 1 ignored, 12 threads |
| `preparation_runtime_lifecycle_tdd` | 267.60 s | 16 tests, 111 pipeline starts |
| `slice_end_to_end_tdd` | 49.96 s | 42 tests, ~10 `pnp_cli` runs |
| `preparation_visual_capture_tdd` | 22.67 s | 4 tests |
| `slicing_precision_integration_tdd` | 11.37 s | 3 tests |
| `modifier_infill_tdd` | 7.14 s | 1 test |
| `calicat_internal_bridge_gating_e2e_tdd` | 5.43 s | 1 test |

The full-suite log's `>60 s` notices name the **critical-path tests**. Measured
individually (warm cache, `--exact`, one process each):

| test | isolated test wall |
| --- | ---: |
| `lightning_migration_tdd::module_plan_matches_recorded_baseline` | **373.31 s** |
| `lightning_migration_tdd::typed_projection_is_atomic_and_neutral` | **344.92 s** |
| `lightning_migration_tdd::fresh_consumers_emit_baseline_segments` | **227.69 s** |
| `lightning_migration_tdd::failed_or_malformed_plan_never_falls_back` | **136.12 s** |
| `preparation_runtime_lifecycle_tdd::shared_selection_all_target_kinds` | **91.27 s** |
| `lightning_migration_tdd::equivalence_rejects_corrupted_baseline` | **46.01 s** |

The five measured lightning tests sum to **1,128 s** isolated but the bucket is 755 s
because they overlap at 12 threads. The wall is bounded by the two ~350 s tests
plus contention: **`module_plan` and `typed_projection` are the tail.**

**A single lightning capture is 38.7 s and a full slice of the same case is
52.0 s** (measured on `pooled_regions_classic`). The remaining wall is *real
pipeline work*, not module loading: do not expect another **component** cache to
fix it (a prepass/preparation memo is move 3).

## Ranked next moves

1. **Parallelize the independent legs inside the critical-path tests.** These
   are the cheapest wins now and need no production change:
   - `module_plan`: per case it runs the WASM capture then the native capture;
     the two arms are independent → run them concurrently (≈2x on that test).
   - `fresh_consumers`: per case it runs preparation-only, a visual capture,
     and a cancellation run; those three legs are independent (the audit's
     table) → run them concurrently (≈3x).
   - `typed_projection`: four visual captures + two full slices are independent
     except the two G-code outputs are compared → run all six, join, assert.
   Expect the bucket to fall toward **6–8 min** if these tests are the tail.
   Verify with the full suite, not just the module.

2. **Share the lightning corpus captures across tests** — [ticket 119](issues/119-lightning-corpus-consolidation.md),
   **41 → 32** active invocations, per-test table and assertion-preservation
   list in [`evidence/t114-e2e-latency/AUDIT.md`](evidence/t114-e2e-latency/AUDIT.md).
   This is now claim hygiene plus aggregate CPU; it reduces contention but not
   a single test's critical path.

3. **Cross-test prepass/preparation reuse** (the next big structural lever, not
   yet ticketed). Each of the 5 lightning cases is prepared separately by every
   corpus test, and `preparation_runtime_lifecycle` runs 111 starts. A memo
   keyed by `(mesh, config, module set)` at the committed-prepass/preparation
   boundary would cut the 38–52 s per capture, not just the load. High risk:
   mutable blackboard/prepared stores and lifecycle observers. Design it as a
   ticket with a Phase-1 screen before editing; a cache bug here corrupts every
   parity assertion.

4. **Prune/merge the remaining small slice modules** (aggregate ~1.5 min):
   painted/cube ~55 s, `infill_overlap` ~24 s, wedge report ~16 s, and the
   `slice_end_to_end` 42 → 9 claim-hygiene merge. These run concurrently, so
   they only matter if they enter the tail; measure first.

5. **Do not** re-open these: `--test-threads 1` (slower overall now),
   `cargo nextest` (process-per-test loses the process cache and the in-process
   `slicer_cache`/`wasm_cache` sharing), and parallelising the 24-component
   compile (the 24-thread probe saved only 2.43 s; the cache made it moot).

## Measurement protocol

```bash
# preconditions
cargo xtask build-guests --check          # exit 0
cargo build --bin pnp_cli                 # locator stale gate; xtask test does it
# idle box: close games/browsers/other builds; record a CPU sample
cargo test -p slicer-runtime --test e2e    # default 12 threads, warm cache
```

- Report **warm-cache** wall as the gate, and **cold** (+~15 s, one process)
  separately.
- The gated entry point is `cargo xtask test -p slicer-runtime --test e2e`
  (+~11 s preflights). Use it for the acceptance ceremony, plain `cargo test`
  for iteration.
- A single module measurement is not the gate: the suite runs tests
  concurrently, so only the full bucket tells you whether a change moved the
  wall.
- Keep `target/test-output.log` per the repo rule.

## Invariants and traps

- **Canonical parity is the highest priority.** The e2e bucket asserts G-code
  content and baseline corpora; never delete the only coverage of a distinct
  path, never weaken an assertion to get green. The t114 audit's remaining
  deletions are listed with coverage maps in `AUDIT.md`.
- **Calicat reference hash** (cache correctness): `d12cdfd1…`.
- **Cache invalidation** is by file identity (mtime + length). A test that
  rewrites a `.wasm` changes the key; a failure is never cached.
- **pnp_cli staleness**: plain `cargo test` does not build `pnp_cli`; nine e2e
  tests fail in 0.1 s on the locator gate when it predates `crates/*/src/**`.
  `cargo xtask test` handles it (`ensure_pnp_cli_fresh`).
- **Guest freshness**: `cargo xtask build-guests --check` must be exit 0 before
  attributing a component/dispatch failure to your change.
- **Disk cache**: `%TEMP%/pnp-wasmtime-cache-v1` (15 MB). Clearing it only
  changes cold/warm; it can never serve a wrong artifact (wasmtime validates).
- **Load sensitivity**: the 2026-11-03 baseline was polluted by a game and a
  sibling checkout. Measure idle and disclose the load; do not compare across
  a busy arm.

## Read next

- [t114 evidence](evidence/t114-e2e-latency/FINDINGS.md) — the 85 %-load screen,
  the inventory, and the adversarial audit (`AUDIT.md`).
- [t118 evidence](evidence/t118-component-cache/FINDINGS.md) — the cache numbers
  and the milestone.
- [Ticket 119](issues/119-lightning-corpus-consolidation.md) — the remaining
  lightning consolidation, with the per-test plan.
- [Map](map.md) — destination, Decisions so far, current frontier.

## Definition of done for the next iteration

- Full bucket measured idle, warm cache, default threads; before/after numbers
  in a new evidence directory with `verify.py` + negative controls.
- All 172 tests still pass (or a smaller count only with a coverage map), the
  calicat hash unchanged, and the gates green (`clippy --workspace
  --all-targets -- -D warnings`, `check-literals`, `check-test-quality
  --report`).
- Record what was **not** claimed (noise, warm/cold, single box) in the ticket.
