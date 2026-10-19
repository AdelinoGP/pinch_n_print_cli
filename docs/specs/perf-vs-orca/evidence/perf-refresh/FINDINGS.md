# Performance refresh: profile first, then decide

## User-selected follow-up

After reviewing the measured ranking, the user selected **perimeter and dispatch
split** for the next session: separate classic-perimeter algorithm work from
dispatch, conversion and IR setup, and investigate the clustered per-layer module
costs. This overrides the recommendation below as the next action. No production
edits begin in this profiling session; the detailed experiment design remains to
be agreed next session.

## Result

Recommend **tree-support planner substage attribution** as the next experiment.
The planner is the largest serial module on both fixtures. Its base module
elapsed is 96.630 s and 92.850 s across the two instrumented runs; benchy is
3.299 s. The repeated base ranking is stable despite changing machine load.
This establishes a target, not the internal cause or an optimization win.

## Scope and evidence

User-approved scope: base primary, benchy cross-check, saved tree-support config,
default threading, uninstrumented repeated baselines plus instrumented attribution;
scratch tooling and documentation only. Production optimization requires a new
decision. No allocator experiment or Orca comparison was performed.

- `run.py`: external Windows measurement harness. CPU comes from final
  `GetProcessTimes` on the retained child handle after exit, avoiding the previous
  harness's last-polled CPU truncation. A small child-process probe verified this
  API path before slicing.
- `environment.json`: executable/config/model hashes, platform, relevant inherited
  environment and initial Git status. No Rayon environment override was present;
  Windows/Python reports 12 logical CPUs. Actual Rayon pool size was not directly
  observed; the default policy was preserved.
- `config.json`: snapshot of `tmp/alloc-bench/config.json`.
- `diagnose.json`: all 23 surviving modules have external provenance; pass true.
- `results.jsonl`: per-run commands, wall, process CPU, system busy CPU, G-code
  hashes/sizes, TYPE counts, phase summaries, slice stats and completion status.
- Individual JSONL traces, stdout captures and G-code outputs are retained here.
- `summarize.py` reproduces `MEASUREMENTS.md` entirely from saved evidence.

## Load limits and baseline interpretation

The original schedule completed a warmup and three uninstrumented measurements
per model, then attribution per model. It showed changing external load, so a
second batch of three uninstrumented measurements per model and a second base
attribution run followed. No measured samples were silently discarded.

Uninstrumented median wall observations:

| Fixture | First batch | Repeat batch |
| --- | ---: | ---: |
| base | 502.827 s | 462.152 s |
| benchy | 30.163 s | 28.926 s |

These are **not controlled quiet-machine baselines**. Other-system busy CPU per
wall second remained 0.887–2.085 even across the repeated base baseline samples.
GetSystemTimes busy delta minus child CPU measures aggregate competing/system
work, not scheduling delay or the identities of interfering processes. Whole-run
CPU/wall is useful context but cannot tell whether an individual stage was starved.
No arbitrary CPU/wall cutoff was used to manufacture a clean subset.

The instrumentation runs happened under different load and finished faster than
some baseline runs. Their ratios do not establish instrumentation overhead.
Historical speedup ratios and the historical allocator conclusions were not
retested. Further unattended repetitions are unlikely to establish a quiet window;
a controlled window is needed before quantitative A/B optimization claims.

## Attribution and next-experiment ranking

See `MEASUREMENTS.md` for the complete phase and module tables. Module elapsed
uses wall time. Parallel per-layer sums are accumulated worker elapsed, **not
process CPU or whole-slice wall**. Serial phase time also includes work outside
module brackets; module tables are not a complete phase decomposition.

1. **Tree-support planner boundary/substage attribution.** Inspect and measure
   cache construction, batched host calls, guest computation, and host-side
   validation/commit separately. `TreeVolumes` calls
   `slicer_sdk::host_batch::batch_offset` in
   `modules/core-modules/tree-support-planner/src/lib.rs`.
   `offset_polygons_batch` (`crates/slicer-wasm-host/src/host.rs`) wraps
   `crate::batch::map_batch`, including WIT/IR conversion and geometry, but
   records no explicit batch-boundary duration. This supports investigating the
   historical cache hypothesis; it does not confirm it. In base attribution run
   1, `PrePass::SupportGeometry` is 111.359 s versus planner module 96.630 s, so
   the surrounding stage deserves separate accounting too.
   Proposed discriminator: if cache/batch work dominates, pursue its request
   shapes and reuse; if guest work or host validation dominates, follow that
   measured branch instead. Do not equate WASM fuel with native host geometry cost.
2. **Classic perimeters plus shared layer overhead.** It dominates accumulated
   per-layer elapsed (1725.920 s on first base attribution; 100.133 s on benchy).
   Meanwhile six base modules cluster at 34.407–34.672 s each: rectilinear,
   gyroid, lightning, top ironing, support ironing and wave overhangs. That
   repeats the shared-overhead signature, but does not prove its source. Separate
   dispatch/conversion/IR setup from useful algorithm work before optimizing
   those apparently equal modules individually.
3. **Native slicing.** `host:slice` costs 46.409–56.190 s on base attribution,
   the next-largest serial module. Its internal split was not measured here.
4. **Support analysis, shell classification, mesh analysis.** Their repeated base
   elapsed values are 37.791 s, 32.203 s and 22.354 s respectively. Shell remains
   meaningful but is no longer first. The surviving footprint union and opening
   tolerance remain unverified internal hypotheses, not current priority findings.

## Correctness observations

Every subprocess exited zero and emitted slice completion. Every base run was
degraded with 29,108 non-fatal errors; benchy was non-degraded with zero. The base
trace contains tree-support planner code-1200 routing-cell-collision rejections,
consistent with the handoff's recorded problem; no new root-cause diagnosis or
proof of exactly which support layers are missing was performed.

The historical assertion that TYPE section counts are stable is too broad:
`Inner wall` varies 525–527 on base and 244–251 on benchy across this capture.
Layer counts remain 495 and 240; other TYPE counts are stable. Prediction and
filament stats also vary slightly, as detailed in `MEASUREMENTS.md`. Hashes and
section counts alone therefore cannot prove fine-grained geometry parity. No
production geometry changed in this session and no parity fix is claimed.

## Validation and gaps

- `cargo build --release -p pnp-cli`: passed; Cargo printed future-incompatibility
  notices for nom and quick-xml.
- `cargo xtask build-guests --check`: exit 0 before measurement.
- Module diagnosis: passed; expected external-shadow warnings retained in JSON.
- Measurement helper probe, complete slice schedule, and saved-log summarization:
  passed. All raw evidence is retained, including load-affected observations.
- No production files changed; no Rust test suite or clippy run was needed for
  scratch measurement scripts and notes. No commit was made.
- Quiet-window baseline, internal planner attribution, memory measurement and
  matched Orca configuration comparison remain uncompleted.
- Side effect: new scratch logs and G-code consume disk space; retained for audit.
