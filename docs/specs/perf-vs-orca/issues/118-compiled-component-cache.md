# Compiled-component cache in the live load path (e2e dev loop)

Type: task
Label: wayfinder:task
Status: open
Assignee: (unclaimed)
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
