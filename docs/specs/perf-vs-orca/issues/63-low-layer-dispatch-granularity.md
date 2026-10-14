# Independent low-layer dispatch and Arachne finishing-tail reduction

Type: task
Label: wayfinder:task
Status: open
Assignee: unassigned
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
