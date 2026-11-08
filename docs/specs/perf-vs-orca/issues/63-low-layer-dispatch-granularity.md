# Independent low-layer dispatch and Arachne finishing-tail reduction

Type: task
Label: wayfinder:task
Status: resolved (2026-10-13 — diagnosis/design take; mechanism established,
premise found stale at HEAD, experiment justified-NO; see Answer)
Assignee: 2026-10-13 session — first frontier ticket in order on map
resumption
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: 61

## Question

Does rayon's indexed-layer split granularity keep the two expensive low layers
from running concurrently, and can a generic dispatch-granularity change remove
the resulting finishing tail while preserving deterministic output?

The [Benchy current bottleneck refresh](61-benchy-current-bottleneck-refresh.md)
observes layers 0 → 1 → 2 sequentially and layer 2 finishing last in all six
captures. In profiling-off Arachne supports-on, layer intervals are 0–438,
438–4,108 and 4,108–5,393 ms; all other layers finish by 3,053 ms. The resulting
2,340 ms low-layers-only interval is measured **opportunity evidence, not a
predicted saving**. Supports-off also retains a 1,038 ms interval.

`execute_per_layer_with_instrumentation_and_support_tools`
(`crates/slicer-runtime/src/layer_executor.rs`) runs
`global_layers.par_iter().map(...).collect()` without an explicit split-size
bound. Existing traces have no worker IDs: same-worker ownership and rayon leaf
grouping are hypotheses, not established causes.

## Scope and decision evidence

1. Establish worker/task identity, queue availability and module-pool waits on
   the actual low-layer intervals. Distinguish a leaf-granularity constraint from
   pool serialization or ordinary work imbalance before choosing a candidate.
2. If granularity is causal, compare a generic bounded-split/independent-task
   design against the indexed baseline. No Benchy-specific layer numbers,
   captured-duration priority table or cross-run warm-history requirement.
3. Retain per-layer stage ordering and immutable Blackboard boundaries. Collect
   results/audits/anchored collections in authoritative global-layer order,
   not completion order. Preserve cancellation, failure propagation, module
   `layer_parallel_safe`/pool rules, and visual-debug identities. Consult
   `docs/01_system_architecture.md` and `docs/04_host_scheduler.md` before any
   scheduling change.
4. Decide whether an output-identical generic experiment is justified. Validate
   actual overlap, not summed module elapsed or a simulated makespan alone.
   Both generators and supports-on/off are controls; base and different thread
   counts need qualification before broad claims.

## Acceptance and boundaries

Diagnosis/design first, no production edit or timing campaign authorized by
ticket creation. A candidate needs fresh output qualification and the standing
paired ordinary + accelerated uninstrumented wall/CPU gate; scheduling can
change utilization without reducing instructions, so CPU corroboration must
be measured rather than waived. Any exception belongs to the human.

Independent of [Infill-linker post-hoist residual](62-infill-linker-post-hoist-residual.md):
neither investigation blocks the other, but changing linker cost requires a
fresh scheduling baseline and their timing campaigns cannot overlap.

Evidence: [refresh findings](../evidence/perf-refresh-2026-10-13/FINDINGS.md),
the reduced per-layer intervals/module calls and checkout-only verifier.

## Answer — resolution comment, 2026-10-13

Scope items 1/2/4 answered with two evidence sources; no production edit, no
uninstrumented timing campaign, no candidate and no commit beyond this
tracker/evidence set. Full detail and re-derivation commands:
[t63 findings](../evidence/t63-lowlayer-dispatch/FINDINGS.md).

**1. Worker/task identity, queue availability, module-pool waits —
established.** A fresh worker-identity probe (env-gated observation-only
patch, preserved as `evidence/t63-lowlayer-dispatch/probe.patch`, applied and
reverted; seven instrumented Benchy runs at post-t65 HEAD `980fde14` —
arachne/classic × on/off at 12 threads plus arachne-on at 6 and 24) shows:

- **Layers 0/1/2 are rayon's initiator-worker left spine**: in all five
  12-thread runs they carry worker index 0 and start depth-first
  (each within 0–112 ms of its predecessor's end; at 6 threads worker 0 runs
  layers 0–3 before its deque empties). The ticket's same-worker hypothesis
  is confirmed; it was a leaf-granularity property of the indexed split, not
  pool serialization or queue starvation.
- **Module-pool waits are zero at 6 and 12 threads** — structurally
  excluded: every per-layer-tier module in the plan is
  `layer-parallel-safe = true`, so `build_wasm_instance_pool`
  (`crates/slicer-wasm-host/src/pool.rs`) sizes each pool at
  `available_parallelism` (one slot per max-concurrent dispatch). Measured
  waits appear only at 24 threads (11 events, 13–42 ms), where workers
  outnumber slots — the pool mechanics confirmed from the other side.
- The first wave of starts is a halving-cascade boundary set
  (`{0,3,7,15,30,60,…}`-shaped in every capture); the front field is saturated
  (12 open), so the low layers' late starts are the spine structure, with
  layer-3 starting at ≈0 ms on its own worker as the direct contrast.
- Module brackets cover layer elapsed inside the heavy layers (arachne-on L1:
  3,369 of 3,669 ms pre-t65), leaving ~300 ms of host-side inter-module work
  inside the tail layers — a residual the
  [host:native attribution](60-host-native-attribution.md) ticket owns, not a
  pool wait.

**2. Granularity is causal, demonstrated by counterfactual.** At 24 workers
the initiator's spine empties: layer 1 is stolen at ≈0 ms onto worker 17 and
runs concurrently with layer 0; layer 2 lands on worker 12 and ends at
1,887 ms — mid-field, not last; the run's last-finishing layer is an ordinary
one (194/237, 4 ms apart) and the lone-open tail is 0. Same job, same binary,
only steal width. Conversely at 6 threads the spine deepens (0–3 sequential)
and the deficit grows to 51.9% of the per-layer phase.

**4. The experiment is NOT justified at current HEAD — the premise's measured
opportunity does not survive.** Per-layer-phase scheduling deficit (observed
span − Σ layer elapsed ÷ threads) across the six t61 captures (pre-t65,
quiet): **1,079–3,022 ms (12.7–49.9%** of the phase; arachne-on instrumented
2,288 ms with a 2,340 ms lone-open tail). Across the seven post-t65 probe
runs: **161–969 ms (2.4–7.0%)**, lone-open 0–61 ms; the closest-quietness
comparison (probe arachne-on r1, 26.5 s wall, vs the t61 arachne-on
instrumented capture, 25.7 s wall) is 174 ms vs 2,288 ms at otherwise matched
conditions. Attribution: the committed t65 batch-execute change cut the
serialized low-layer chunk the spine was holding — layer-1 linker dispatch
**3,191 → 1,228–1,662 ms**, layers 0–2 linker accumulated **4.50–6.23 s →
1.39–1.80 s (≈3–4×)**. The map's route note attached to this ticket (the t65
commit is a linker-cost change; 63's family needs a fresh scheduling
baseline) was the correct constraint: the motivating "2,340 ms interval" was
a pre-t65 measurement. The measured remainder at HEAD (~0.12–2.3% of
whole-slice wall on Benchy, inclusive of load inflation) is below the map's
"~1% effect cannot matter against the 6.28–26.24× gap" bar; a keep/drop
campaign for it is rejected by measurement, not by judgment.

**3. Design record for any future re-open** (kept, not started):

- Candidate shape (generic): single-item leaves — `with_max_len(1)`-class
  granularity on the indexed iterator — makes every layer dispatchable the
  moment a worker frees; the 24-thread counterfactual demonstrates the end
  state from the steal-width direction. Rayon's indexed `collect()`
  preserves result order, so the authoritative global-layer-order
  collection, per-layer stage sequencing, immutable-Blackboard boundaries,
  cancellation and failure propagation of
  `execute_per_layer_with_instrumentation_and_support_tools`
  (`crates/slicer-runtime/src/layer_executor.rs`) are unaffected; pool sizes
  already cover every dispatch count a 12-worker host can create.
- Bounded split the other way (chunks ≈ n/threads) is measured the wrong
  direction: it deepens the serialized spine (the 6-thread run is the
  small-scale demonstration).
- Re-open condition: a future fresh scheduling baseline showing a large
  deficit again (e.g. a change re-inflating low-layer module work). The
  pre-t65 42–50% deficits are the shape to watch for; this verifier detects
  it (`summarize` fields in `summary.json`/`probe_summary.json`).

Qualification of the probe batch: `cargo xtask build-guests --check` exit 0
before it; all-external module provenance; all seven outputs byte-identical
to their t58 finalization qualification references with clean completion
status. Disclosed limitations: probe runs are instrumented (attribution
only) and ran under visible background churn (arachne-on r2 34.9 s vs r1
26.5 s wall); no base.stl probe run (the deficit shape's Benchy
demonstration is the deliverable; base qualification stays with any future
experiment); the t61/t63 deficit comparison mixes tree and kind
(instrumented vs profiled) within the recorded sets — the headline
comparison is kind-matched.
