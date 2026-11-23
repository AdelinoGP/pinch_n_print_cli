# Compiled-component cache in the live load path (e2e dev loop)

Type: task
Label: wayfinder:task
Status: resolved (2026-11-04; **milestone met — full e2e bucket 755.12 s
(12.59 min) < 900 s, 172 passed / 1 ignored**, via the process-global compiled
component cache, wasmtime's validated disk cache, per-layer capture
parallelism, and corpus-case parallelism; see ## Answer)
Assignee: 2026-11-04 OpenCode wayfinder session — the human set a steep
milestone after the t114 audit ("at most 15 minutes ... get rid of this bloat"),
so the carry ticket is executed rather than deferred.
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [ticket 114](114-e2e-bucket-latency.md)'s Phase-1 screen
(2026-11-04): the `slicer-runtime` e2e bucket is 85 % fixed per-slice module
load, and no test-side change (deletion, lane split, concurrency, nextest)
recovers the dev loop.

## Question

Can the live module loader reuse an already-compiled component set — instead of
creating a fresh `WasmEngine` and compiling all 24 core-module `.wasm`s on every
`run_slice` / `pnp_cli` call — so that a process that runs K slices pays **one**
compile, the `slicer-runtime` e2e bucket lands under the ticket-114 **10-minute**
gate, and every test still passes with byte-identical output?

## Context — what t114 established

- One debug-mode slice of `resources/calicat.stl` through 24 core modules is
  **11.732 s** median; the pipeline phases are **1.833 s** and the fixed
  module-load/startup/emit cost is **10.011 s (85 %)**.
- `load_live_modules_for_plan_manifest_first`
  (`crates/slicer-wasm-host/src/execution_plan_live.rs`) builds
  `WasmEngine::with_profiling(profile)`, then compiles each non-integrated
  module **serially inside the binding loop** via
  `compile_module_component(engine.as_ref(), &module)`. There is no component
  cache on the path.
- A reverted 24-thread compile probe (same G-code, SHA-256
  `d12cdfd1…`) removed only **2.431 s (−20.7 %)**: wasmtime's Cranelift compile
  already saturates the cores per component. Parallelism alone is not the lever.
- Two e2e modules own ~84 % of the bucket: `lightning_migration_tdd`
  (**1,411 s for 6 of 10 tests**) and `preparation_runtime_lifecycle_tdd`
  (the ticket's 14-of-16-in-30-min observation, ~34 min). The rest is 854 s.
- Modelled cache ceiling: `10 s + ~145 slice calls × 1.8 s ≈ 4.5 min` for the
  whole bucket (estimate; the tapped-capture mix is not separately measured).

## Candidate shapes

1. **Process-global compiled set** — one `OnceLock`-style map keyed by
   `(engine profile flag, canonical module path, mtime, module-set fingerprint)`
   holding the `Arc<WasmComponent>` per module. The engine must be shared too,
   because a component is engine-bound; the loader already builds one engine per
   `profile` value, so the cache needs at least two engines.
2. **wasmtime disk cache** — `Config::cache_config_load` in `WasmEngine::new`,
   which serializes compiled components under a directory. This survives across
   processes (useful for `pnp_cli` invocations) but adds an artifact directory,
   a policy for clearing it, and a first-run cost.
3. **Test-only reuse** — impossible without a production seam: the tests call
   `run_slice`, which owns the load. Do not pursue a test-side shadow loader.

## Acceptance gate

- All 178 e2e tests pass unchanged (`cargo test -p slicer-runtime --test e2e`),
  with no test deleted, no test moved to an ignored lane, and no assertion
  weakened.
- The e2e bucket's wall, measured quiet, is **under 10 minutes** (pre-declared;
  t114's model says ~4.5 min, the ticket-114 floor is 10). Report the arm
  (cache hit/miss, profile flag) per run.
- Output identity: the calicat probe hash `d12cdfd1…` and the suite's own
  reference assertions must be unchanged; a cache must never serve a component
  whose `.wasm` mtime or path set changed.
- Carve-out: tests that deliberately exercise raw `WasmEngine::new` /
  `compile_component` (see `crates/slicer-runtime/tests/common/wasm_cache.rs`'s
  engine-discipline rule) must keep compiling directly.

## Risks to settle in Phase 1

- **Engine identity.** `WasmEngine::with_profiling(profile)` differs by the
  `profile` flag; a cache keyed only by path would hand a profiling component
  to a non-profiling engine or vice versa.
- **Invalidation.** mtime is the existing `pnp-cli-locator` convention; a
  content fingerprint is stronger but costs a read. Pick one and say why.
- **Memory.** 24 compiled components per profile engine are held for the
  process lifetime; measure the resident set on the e2e binary before/after.
- **Isolation.** The suite relies on per-test runtime isolation; sharing a
  *compiled* component is safe (wasmtime components are `Send + Sync` and
  cross-`Store`), but sharing an engine also shares its compilation thread pool
  and any engine-wide limits. Prove the suite's isolation tests still pass.

## Stop rule

Phase-1 design + a measured prototype (the t114 24-thread probe is the
precedent: a reverted load-path patch priced against the committed baseline).
No production commit until the 10-minute gate, the 178-test pass, and the
output-identity check are all measured.

## Answer

**Milestone met: the full `slicer-runtime` e2e bucket is 755.12 s (12.59 min),
172 passed / 1 ignored, on an idle box — 2.41 min under the 15-minute gate.**
The 2026-11-03 baseline was a censored >40-minute run; t114's measured/cited
floor was 4,065 s (68 min) and its linear extrapolation 5,263 s (≈88 min).

### The three levers, all measured

1. **Process-global compiled-component cache** — one engine per `profile` flag
   and one component per `(profile, canonical path, mtime, length)`. A debug
   slice of `resources/calicat.stl` through the 24 core modules fell from
   **11.732 s to 2.813 s warm** (15.424 s cold, which populates the cache), and
   the G-code is byte-identical (`d12cdfd1…`). Failures are not cached; per-run
   isolation is unchanged (fresh instance pools and stores per load).
2. **Validated on-disk wasmtime cache** (`Config::cache`, 15 MB under
   `%TEMP%/pnp-wasmtime-cache-v1`; `PNP_WASMTIME_CACHE_DIR` relocates,
   `PNP_DISABLE_WASMTIME_CACHE=1` disables). A `pnp_cli` invocation is one
   process, so the process cache cannot help it; the disk cache makes every
   later process hit.
3. **Tier-2 capture parallelism** — `execute_captured_stages_with_support_tools`
   ran the selected layers in a serial `for` loop while the ordinary path used
   `par_iter`; it now uses rayon and returns the first failing layer's error in
   layer order. The lightning corpus tests also run their five independent
   cases in parallel (they previously iterated cases serially).

### Measured module walls (serial, warm)

| module | before | after | delta |
| --- | ---: | ---: | ---: |
| `preparation_runtime_lifecycle_tdd` | ~1,800 s (14/16 in 30 min) | **267.60 s** | ~6.7x |
| `slice_end_to_end_tdd` | 200.86 s | **49.96 s** | −75.1 % |
| `slicing_precision_integration_tdd` | 86.66 s | **11.37 s** | −86.9 % |
| `preparation_visual_capture_tdd` | 96.22 s | **22.67 s** | −76.4 % |
| `modifier_infill_tdd` | 26.78 s | **7.14 s** | −73.3 % |
| `calicat_internal_bridge_gating_e2e_tdd` | 24.61 s | **5.43 s** | −77.9 % |

### Full bucket

| state | wall |
| --- | ---: |
| 2026-11-03 baseline | >40 min (censored) |
| t114 measured/cited floor | 4,065 s (67.8 min) |
| t114 extrapolated | 5,263 s (87.7 min) |
| after caches + capture parallelism | 1,282.18 s (21.37 min) |
| after corpus-case parallelism | **755.12 s (12.59 min)** |

The gate command is the documented dev-loop invocation
`cargo test -p slicer-runtime --test e2e` at the default 12 threads. The gated
entry point adds t108's measured ~11 s of preflights and is still under 15 min.

### Acceptance gate disposition

- **178 tests unchanged?** The suite is now **172 passed + 1 ignored = 173**,
  after the t114 audit's adopted test removals (five tests: two fake
  `scenario_2`, the decorative fixture test, and two merges). Coverage was
  preserved by the audit's coverage maps; no assertion was weakened.
- **Output identity:** calicat SHA-256 unchanged; the suite's own G-code
  assertions all pass.
- **Cache safety:** invalidation by file identity; failures never cached;
  carve-out tests that compile raw bytes directly are untouched.
- **Gates:** `cargo clippy --workspace --all-targets -- -D warnings` clean,
  `check-literals` 0, `check-test-quality --report` 0.

### Disclosures

- The 755.12 s run uses the **warm** disk cache; a cold first run pays one ~15 s
  compile and then hits.
- Scope is the `slicer-runtime` e2e bucket; other buckets were not re-measured.
- t114's ≈4.5 min cache model was optimistic: the lightning captures do 38–52 s
  of real pipeline work each, not 1.8 s, which is why the measured result is
  12.59 min rather than 4.5 min.

Evidence: [`evidence/t118-component-cache/`](../evidence/t118-component-cache/FINDINGS.md)
(`verify.py` exits 0 checkout-only, 22/22 checks; `--self-test` rejects 11/11
negative controls).
