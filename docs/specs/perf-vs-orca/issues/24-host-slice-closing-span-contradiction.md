# host:slice closing_ex span contradiction

Type: task
Status: resolved

## Question

`host:slice`'s `closing_ex` scope spans (~22.3 s accumulated) contradict its
`module_complete` wall (~3.2 s). Which number is real — and does
`fold_marks` (`crates/slicer-wasm-host/src/profiling.rs`) handle thread-mapped
scope marks correctly?

This is lead 4 of the [emit_walls premise
falsified](08-emit-walls-premise-falsified.md) ranking. The working hypothesis
is the accumulated-worker-elapsed trap (`docs/specs/perf-vs-orca/evidence/PERF-HANDOFF.md` §3.3: summed
worker wall-clock includes waits and is not CPU), but it must be **verified**
against `fold_marks`' thread handling before either number is believed — the
§9.7 history shows a wrong reading here once motivated a dead hypothesis (the
allocator A/B).

Attribution-only: no optimization proposed or authorized until the
contradiction is explained. Small ticket — the answer either retires the
`host:slice` item from the lead list or promotes a real 22 s cost.

## Answer

Claimed and resolved 2026-09-23 (AFK agent session) — attribution only; no
optimization proposed or authorized.

**Verdict: a units mismatch, not corrupted measurement. `module_complete`'s
~3.2 s is real wall; the `closing_ex` ~22.3 s is accumulated per-thread spans
— the §3.3 trap at scope granularity. The `host:slice` item retires from the
lead list: there is no ~22 s wall cost and nothing to promote into P.**

**Which number is real — both, in different units.** `run_builtin_stage`
(`crates/slicer-runtime/src/prepass.rs`) brackets the whole built-in with one
`StageInstrumentationGuard`, and `execute_prepass_slice_all_layers`
(`crates/slicer-runtime/src/builtins/prepass_slice_producer.rs`) fans the 240
per-layer tasks out over `global_layers.par_iter()` inside it (per-layer
bridge classification / flat-bridge enclosure, each layer reaching
`polygon_ops::closing_ex` (`crates/slicer-core/src/polygon_ops.rs`) via
`bridge_over_infill` (`crates/slicer-core/src/algos/bridge_over_infill.rs`)).
So `module_complete`'s `elapsed_ms` is true wall — its own event timestamps
bracket it. The profile table's walls come from `flush_native`
(`crates/slicer-runtime/src/profiling_report.rs`), which fires once per
outermost marked-scope unwind **per thread** and merges with `entry.wall_ns
+= wall_ns` / `scope.total_wall_ns += row.total_wall_ns` — concurrent worker
spans are summed, so those columns are accumulated thread-time (CPU-shaped
work share), comparable to neither wall nor CPU-with-waits.

**Does `fold_marks` handle thread-mapped scope marks correctly — yes.**
`NativeProfileSink` (`crates/slicer-runtime/src/profiling_report.rs`) buffers
marks in the thread-local `NATIVE_STATE` and flushes per thread, stamping each
mark against that thread's own `epoch`, so `fold_marks`
(`crates/slicer-wasm-host/src/profiling.rs`) always receives one thread's
well-bracketed stream and per-pair subtraction stays within one monotone
clock. The fold is thread-blind because the thread mapping happens before it;
the guest path never crosses threads either (per-call stashes drained into
`record_call`, incl. `drain_postpass_profile`
(`crates/slicer-runtime/src/postpass.rs`)). No pairing corruption exists —
only the run-wide `+=` aggregation changes units. Latent corner, not reachable
in this tree: a `ScopeGuard` created on one thread and dropped on another
would mis-pair one buffer; every `profile::scope` use is a lexical local today.

**Same-run reproduction** (2026-09-23, this session; attribution only): one
`--profile --instrument-stderr` benchy run shows `module_complete` for
`host:slice` at **3,889 ms** (wall) while the table credits it **39.617 s**
with `polygon_ops::closing_ex` **34.277 s over 240 calls** — 8.8x, and Σ
native rows (80.162 s) exceeds the whole run's 26.928 s. The row reconciles
exactly (34,277,317,800 + 2,926,661,500 + 2,412,809,300 + 149,200 self =
39,616,937,800 ns) and activations 5,921 = 240 + 5,005 + 676, i.e. every
marked call flushed as its own per-thread activation. Capture and repro:
`evidence/t24-span-contradiction/SAME-RUN.md`. The original figures (22.7 /
23.5 / 22.3 s stable vs `module_complete` 3,152 ms,
`evidence/perf-emit-walls/FINDINGS.md`) reproduce the same shape at their
vintage.

**Downstream:**

- The lead retires: the work under `host:slice` already sits inside its ~3–4 s
  wall and is already counted in P; there is no second ~22 s term anywhere.
- What the accumulated `closing_ex` figure *is* worth: a work-share number
  (per-layer bridge-classification work, parallel) — input to the work-density
  questions in [Accelerated residual query
  attribution](18-accelerated-residual-query-attribution.md) and
  [Classic output-volume surplus](28-classic-output-volume-surplus.md); never
  a wall term.
- [Tree-planner substage attribution](22-tree-planner-substage-attribution.md)
  and [Serial host floor](27-serial-host-prepass-floor.md) inherit this
  verdict: `fold_marks` output is trustworthy per scope pair and for
  work-share, but its wall columns must never be compared to
  `module_complete`/phase walls — wall claims need same-run stopwatch brackets.
- [Serial host floor](27-serial-host-prepass-floor.md)'s `host:slice`
  exclusion dissolves (body updated 2026-09-23): the stage's serial suspects
  are the batch caches (`batch_slice_objects_by_layer`,
  `batch_bottom_surface_footprints`), its per-layer part is parallel.

Follow-up, not authorized here — opened 2026-09-24 as
[profile wall-column semantics](30-profile-wall-column-semantics.md):
document the accumulated-thread-time semantics where the columns are defined
(`ScopeTotals::total_wall_ns`, `ProfileModuleRow::total_wall_ns`, and the
`profiling_report.rs` module doc's observer-effect note). Until then the map's
recipe-traps list carries the trap.
