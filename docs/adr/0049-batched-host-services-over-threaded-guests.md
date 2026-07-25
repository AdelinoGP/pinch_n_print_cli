# ADR-0049 — Batched host services over threaded guests

<!-- filename: 0049-batched-host-services-over-threaded-guests -->

## Status

Accepted (2026-07-24). Authored after a measurement pass over prepass parallelism on a 0.1 mm benchy, and a toolchain feasibility probe.

## Context

Prepass is the dominant phase and is executed strictly serially. On a 0.1 mm
benchy (480 layers, 1 object, 12 cores) the slice took 46.4 s: prepass 30.0 s
(65%), per-layer 14.9 s (32%), postpass 0.92 s (2%). `execute_prepass_with_instrumentation`
is a flat `for module in &stage.modules` loop with a blackboard commit after each
module; the postpass loop has the same shape. Per-layer already fans out across
rayon with one WASM instance per worker, gated on the module manifest's
`[hints] layer-parallel-safe` — the only parallelism opt-in module owners have.

Within prepass the two hot serial items were `com.core.support-planner`
(11.1 s, a user WASM module) and `host:shell_classification` (8.5 s, a host
built-in). `host:slice` (7.6 s) was measured 95% inside its existing rayon map,
and `host:overhang_annotation` (2.2 s) is already parallel.

A native replay harness over `support-planner` (captured benchy prepass IR,
release, idle machine, two runs) split its runtime as: collision-cache build
98.0%, contact detection 1.1%, MST propagation 0.8%. The cache build is 603
independent `offset_polygons` calls at ~7 ms each. **The module's own
non-geometry compute is under 2% of its runtime.**

Those 603 calls do **not** currently cross the WIT boundary. `slicer_core::polygon_ops`
is not gated behind the `host-algos` feature — it is backed by `clipper2-rust`,
which compiles to wasm32 — and `slicer_sdk::host::offset_polygons` has no
`cfg(target_arch = "wasm32")` bridge. Only `medial_axis` and
`generate_arachne_walls` have one, because boostvoronoi and rayon are
native-only. So today a guest calling `offset_polygons` runs clipper2 **inside
the sandbox, single-threaded**. The `offset-polygons` import declared in
`slicer:common/host-services` exists but the SDK never calls it.

This makes the batching case stronger, not weaker, but for a different reason
than "fewer boundary crossings": moving the work host-side both takes it out of
wasm into native code *and* makes it eligible for the rayon pool. Since a guest
cannot spawn threads (below), moving the work to the host is the **only** way to
parallelize it at all. The offsetting cost is against that: 603 polygon sets now
have to be marshalled across the boundary that they currently never touch.

Two mechanisms could address this:

- **Threaded guest** — the module spawns its own worker threads inside one host
  call (see `CONTEXT.md`).
- **Batched host service** — the guest stays single-threaded and hands the host
  a whole batch in one call; the host fans out over the rayon pool it already
  owns.

The threaded-guest option carried a locked prior decision to make host services
safe for concurrent re-entry from worker threads. Two findings undercut it.

**Concurrent host re-entry appears structurally impossible.** The component
`TypedFunc::call` takes `impl AsContextMut` — an exclusive borrow of the `Store`
— and calls `validate_sync_call()` on entry. Host imports execute inside that
exclusive call. Several guest threads cannot be inside host imports on one
`Store` at once.

**Rust cannot emit a component that spawns a thread.** Probed directly:

- Every guest here builds for `wasm32-unknown-unknown` and is componentized with
  `wasm-tools component new` (`xtask/src/build_guests.rs`). A `std::thread::spawn`
  compiled for that target links `std::sys::thread::unsupported::Thread::new` and
  carries `"operation not supported on this platform"`. Adding
  `-C target-feature=+atomics,+bulk-memory,+mutable-globals` and rebuilding std
  with nightly `-Z build-std` does **not** change this — the unsupported stub is
  still linked and the emitted memory is still unshared.
- `wasm32-wasip1-threads` does have a real thread implementation, but it imports
  its shared memory from `env` and imports `wasi:thread-spawn` — a core-module
  WASI mechanism with no component-model equivalent. It cannot be componentized
  by the adapter-less `wasm-tools component new` invocation this repo uses.
- wasmtime's own `Config::wasm_threads` documentation states that the core
  threads proposal "does not actually include the ability to spawn threads";
  spawning intrinsics for components sit behind the unstable
  `shared-everything-threads` proposal.

Notably, the **engine side is already ready**: a component containing a core
module with `shared` memory compiles under the repo's current `WasmEngine::new`
configuration unchanged (`wasm_threads` defaults on), and
`wasm_shared_everything_threads(true)` is accepted. The blocker is entirely the
guest toolchain.

## Decision

**Parallelism offered to module owners is delivered as batched host services,
not as guest-internal threads.**

A batched import takes per-item parameters and returns results in input order:

```wit
record offset-request {
  polygons: list<ex-polygon>,
  delta-mm: f32,
  join:     offset-join-type,
}
offset-polygons-batch: func(requests: list<offset-request>) -> list<list<ex-polygon>>;
```

The first cut covers five of `slicer:common/host-services`' geometry functions —
`offset-polygons`, `clip-polygons`, `simplify-polygon`, `raycast-z-down`,
`surface-normal-at`. `medial-axis` and `generate-arachne-walls` are deferred:
their only caller (`arachne-perimeters`) is a per-layer module that already
receives host fan-out, `generate-arachne-walls` writes to a process-global
(`HOST_ARACHNE_WALL_SEQUENCE_CAPTURE`) whose ordering would become
completion-order under a fan-out, and `crates/slicer-core/src/arachne/pipeline.rs`
already uses rayon internally.

Singular forms stay. Fan-out is gated on an estimated-work cost model rather than
item count, because per-item cost spans roughly three orders of magnitude across
the five services. Batch calls record one module-access-audit entry plus a batch
size, added as a new field rather than by changing `runtime_reads: Vec<String>`.

**The SDK's batch wrappers must bridge to the host on `wasm32`.** A batch helper
that simply loops the existing in-guest `polygon_ops` call would move no work
and gain nothing; the point of the batch is to reach the host. On native targets
the wrapper stays local so module unit tests and native harnesses keep working
without a runtime.

The threaded-guest track is **not cancelled** — it is blocked on the guest build
pipeline, and it remains the only mechanism that could parallelize a module's own
algorithm. `build_wasm_instance_pool`'s `SharedMemoryRejected` rule (a module gets
host fan-out **or** internal threads, never both) stands pending that work.

## Consequences

- Module owners get a deterministic parallelism opt-in with no new concurrency
  semantics: batch results are returned in input order, so output cannot depend on
  worker count or scheduling. Verified by a forced-serial versus forced-parallel
  byte-comparison at varying worker counts, with the canonical parity suite as
  backstop.
- The capability is **narrower than "let modules parallelize themselves."** It
  covers host-provided operations only. A module bottlenecked on its own compute
  gets nothing from it. That is an accepted limitation, not an oversight: it buys
  98% of the one workload measured, and the general mechanism is not currently
  constructible.
- Adopting a batch form **moves geometry out of the guest and into the host**,
  which is a behavioural change even though both sides call the same
  `slicer_core::polygon_ops` code. Floating-point results should be identical
  (same crate, same inputs), but this is the kind of change the canonical parity
  suite exists to catch, and adoption should be one module at a time with the
  suite green in between.
- **That check cannot currently be done by G-code byte comparison.** The
  pipeline is non-deterministic run-to-run on an unmodified binary (DEV-093):
  around 9 sliver inner-wall loops flip in or out per run, ~106 differing lines,
  and it persists at `RAYON_NUM_THREADS=1`. Any whole-output diff of that size
  and shape is inside the noise band, so it cannot attribute a behaviour change
  to a code change. Verifying `support-planner`'s adoption properly is blocked
  on DEV-093.
- Marshalling cost is now on the wrong side of the ledger for callers whose
  polygon sets are large and whose per-call compute is small. The estimated-work
  threshold does not model marshalling. If a batch adoption ever measures slower,
  that is the first thing to check.
- Nothing here depends on auditing host services for concurrent re-entry. A batch
  is one call with one `&mut` host context; the fan-out happens in native code
  with the guest blocked, so only the *algorithm* must be thread-safe.
  `slicer_core::polygon_ops`, `arachne/`, and the medial-axis algos contain no
  `static mut`, `thread_local!`, or interior-mutable statics.
- `CONTEXT.md`'s **Threaded guest** and **Parallel-safe host service** entries
  describe a mechanism that cannot be built on the current guest target. Both are
  annotated accordingly so nobody designs against them meanwhile. In particular,
  "Parallel-safe host service" describes concurrent re-entry from worker threads —
  which the `Store` signature appears to forbid — and is *not* what a batched
  service is.
- Re-opening the threaded-guest track requires a change to how all 34 guests are
  built, not merely a wasmtime configuration change. The engine is already ready;
  do not re-probe that half.
