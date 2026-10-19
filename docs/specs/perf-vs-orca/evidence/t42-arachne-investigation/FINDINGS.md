# Arachne adoption overlap: saved-evidence investigation

**The wall-time limit is localized to a two-layer execution tail, not to a
proven module-level defect. Adoption remains inconclusive; ordinary remains
production.** The human chose investigation, with saved evidence first. No
slice, build, acceptance retry, skipped cell, source fix or default switch ran.

## Evidence and replay

Inputs are the original T38 [runner summary](../t38-adoption-resumed/campaign/attempt-1/campaign-summary.json)
and its manifest-named durable CSV, JSONL and G-code archive. The analysis uses
all four retained samples per variant, in verified ABBA/BAAB order, without
warmups or exactness runs. Classic is a same-campaign control, not a substitute
for Arachne acceptance. Historical profiles were not pooled into this sample.

`analyze` in [analyze_saved.py](analyze_saved.py) verifies hashes, CSV/summary
timing agreement, status/generator counters, output identity, phase/layer
coverage and sample order before computing statistics. Derived evidence:
[saved-analysis.json](saved-analysis.json). Default exit 0 means analysis
integrity passed, **not** that adoption passed.

```bash
python docs/specs/perf-vs-orca/evidence/t42-arachne-investigation/analyze_saved.py
python docs/specs/perf-vs-orca/evidence/t42-arachne-investigation/analyze_saved.py --require-wall-separation
```

The first command passed. The wall-gate replay returned exit 1 as expected:
ordinary minimum 16.5563 s is below accelerated maximum 17.1664 s. This replay
reproduces the saved symptom; it is **not a live feedback loop for a code fix**.
Live reproduction, bisection and fixes are deferred because this scope is
saved-evidence-only, not authorization for new timing or artifact changes.

Recomputation matched the saved analysis. A deliberately altered summary in a
separate temporary directory was rejected at the original hash gate. A final
read-only sweep verified all original raw and tracked inventory entries;
[validation-results.json](validation-results.json) records those checks. No
original campaign/helper artifact was rewritten. These checks validate retained
evidence, not current guest freshness or current frozen snapshot integrity.

## What the retained samples show

Every Arachne sample has layers **1 and 2 (zero-based global indices)** as its
two longest invocations. Layer 2 starts immediately after layer 1 completes;
both finish after all other layers. This is an observed schedule, **not proof
that the layers have a semantic dependency or that Rayon is incorrect**.

| Arachne retained samples | Ordinary | Accelerated |
|---|---:|---:|
| All other layers completed by, range from per-layer phase start | 1.683–1.769 s | 1.379–1.390 s |
| Full per-layer phase, range | 8.276–8.662 s | 8.343–8.514 s |
| Layer 1 mean elapsed | 4.965 s | 4.918 s |
| Layer 2 mean elapsed | 3.22875 s | 3.17825 s |
| Sum of other layer elapsed intervals, mean (NOT CPU or phase wall) | 18.34825 s | 14.84825 s |
| Whole-process CPU median | 72.8672 s | 69.59375 s |
| Whole-process wall median | 17.17315 s | 17.11635 s |

The aggregate layer-interval reduction occurs mostly outside the finishing
tail. Those intervals overlap; neither their sum nor its difference is process
CPU or critical-path wall. CPU nevertheless independently separates favorably
in the native process measurements. The CPU median improves 4.49235%, while
wall median improves only 0.33075% and its ranges overlap.
All eight retained Arachne G-code files are byte-identical by verified SHA-256;
output-volume differences do not explain this particular comparison.

Classic also finishes on layers 1 and 2, but its layer-1 mean changes from
6.87725 to 5.8145 s. Its full per-layer mean improves 1.10025 s. Unlike Arachne,
the Classic win reaches the finishing tail. This is localization, not an
attribution of layer-1 cost to a particular algorithm.

### Additive wall decomposition

Use **means**, not a sum of medians, for the following accounting. Each row's
whole-process wall is decomposed into four non-overlapping phase durations,
runtime elapsed outside those phases, and process wall outside runtime elapsed.
The script verifies that component mean deltas sum to the process mean delta.

| Arachne component | Accelerated − ordinary mean |
|---|---:|
| Validation | −5 ms |
| Prepass | +72.5 ms |
| Per-layer | −89.25 ms |
| Postpass | +0.5 ms |
| Runtime outside recorded phases | +24.75 ms |
| Process outside runtime elapsed | −4.325 ms |
| **Whole-process wall** | **−0.825 ms** |

The mean per-layer gain is offset almost completely elsewhere. Ordinary
prepass spans 6.800–7.399 s, versus accelerated 7.132–7.307 s. The fastest
ordinary sample also has the fastest ordinary per-layer phase. Its favorable
timing must remain included; there is no evidence justifying removal.

Do not label the runtime residual a shutdown tail: `run_slice_with_collector`
(`crates/slicer-runtime/src/run.rs`) starts its clock before module loading and
samples `wallclock_ms` before print estimation/end events. The residual also
contains work before and between phases. The CLI's `Cmd::Slice` match arm
(`crates/pnp-cli/src/main.rs`) writes output after `run_slice` returns.
`Get-ProcessTimesSeconds` (`resources/perimeter-acceptance/run_bench.ps1`)
measures child creation-to-exit, so neither benchmark polling nor independent
validation is part of its wall interval.

## Hypothesis disposition

1. **Parallel savings miss the finishing tail: supported at layer granularity.**
   Every sample has the same two late layers, with little observed mode benefit
   there and larger interval savings elsewhere. This explains the limited
   wall transfer, but does not identify which module consumes those layers.
2. **Other phase variation masks a benefit: supported as accounting, cause open.**
   Prepass and residual mean changes offset the small per-layer gain. There is
   no contemporaneous machine-load telemetry proving descheduling, external
   load, thermal effects or cache effects. Calling this merely noise would
   overstate the evidence.
3. **A compensating module-level accelerated regression: neither proved nor
   ruled out.** Phase-level prepass/residual increases exist, but those ranges
   overlap and contain no stage/module events. No source change is justified.

The retained captures have phase and layer events, **no `module_complete`,
`stage_complete` or `profile_summary`**. Thus neither Arachne graph construction,
infill linking, host marshalling nor spatial predicates can honestly be named
as the low-layer bottleneck. Older ticket-18/25 shares cannot fill that gap.

## Next bounded probe — pending authorization

Authorize **one ordinary and one accelerated instrumented supports-off Arachne
slice**, sequentially, using the original manifest-selected snapshots, model,
config and 12-thread setting. Verify frozen identities before/after; keep new
outputs in a distinct durable diagnostic namespace. Use `--instrument-stderr`
without fuel profiling initially, or the existing benchmark harness's
`-Instrumented -ExpectedGenerator arachne` route. Do not call campaign mode.

Rank `module_complete`/`stage_complete` costs **within layers 1 and 2**, also
inspect the corresponding prepass stage durations, then choose the next probe
from measured attribution. Concurrent worker spans remain work intervals, not
additive phase wall or process CPU. Diagnostics are not retained acceptance
samples and cannot amend this campaign's verdict. Any later fuel probe, code
fix or fresh acceptance attempt needs its own scope; no such work ran here.

Tracker: [critical-tail attribution](../../issues/43-arachne-critical-tail-module-attribution.md).
