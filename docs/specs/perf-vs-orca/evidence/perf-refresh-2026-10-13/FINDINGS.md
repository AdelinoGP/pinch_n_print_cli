# Benchy refresh after clone-once finalization

**Three new investigation questions are justified:** the infill linker's
post-hoist residual, independent scheduling of the two expensive low layers,
and tree-support **rendering** (not the already-investigated planner).
The existing annotation, flat-bridge and unattributed-native tickets retain
their own questions; none is silently reopened or duplicated.

Route: [Benchy current bottleneck refresh after clone-once finalization](../../issues/61-benchy-current-bottleneck-refresh.md).

## Method and evidence

- Rebuilt the ordinary release executable with
  `cargo build --release --bin pnp_cli` after discovering the existing executable
  predates the accepted finalization change. No production source was edited.
- `cargo xtask build-guests --check` exited 0 before the batch; isolated
  `module diagnose` passed with every loaded module external. Input/artifact
  hashes and captured source revision are in `inputs.json`.
- Six sequential ordinary-mode Benchy slices, 12 rayon threads, the map's
  existing matched configs: 0.4 mm nozzle, 0.2 mm layers, two walls, 20% gyroid,
  classic/arachne and supports off/on. Four fuel profiles use
  `--profile --profile-verbose --instrument-stderr`; two additional Arachne
  runs use `--instrument-stderr` without profiling.
- Every output is byte-identical to its accepted finalization qualification
  reference (`../t58-finalization-copies/qualification.json`), including TYPE
  counts. Every completion is `ok`, `degraded=false`, zero fatal/non-fatal
  errors. **This is not a warning-free or geometry-correctness claim**; see the
  warning disclosure below.
- Optional fuller raw source: `target/perf-refresh-2026-10-13/`. The in-repo
  `captures/` retains phase and serial brackets, every layer's start/end,
  every per-layer module completion/fuel count, process measurements, output
  census and the full aggregate profile. `reduce.py` reconstructs that set
  from raw captures and hashes their G-code/JSONL; `verify.py` runs using only
  in-repo evidence and re-derives `summary.json`.

**Units:** fuel is guest instructions, not native cost. Host profile values and
per-layer module sums are accumulated thread-time, not slice wall or process
CPU. Instrumented and profiled runs are attribution only; no fresh
uninstrumented timing, candidate A/B, accelerated run, base run or Orca run was
performed. No speedup or scoreboard revision follows from these captures.

## Fresh phase anatomy

All entries below are diagnostic phase brackets in seconds, not acceptance
timings. Do not combine the two observer modes into a median.

| ordinary Benchy cell | observer | prepass | per-layer | postpass |
| --- | --- | ---: | ---: | ---: |
| classic supports-off | fuel profile | 8.518 | 9.287 | 0.512 |
| classic supports-on | fuel profile | 17.189 | 9.516 | 2.241 |
| arachne supports-off | fuel profile | 8.519 | 6.054 | 0.302 |
| arachne supports-on | fuel profile | 16.928 | 6.059 | 5.859 |
| arachne supports-off | instrumentation only | 8.117 | 5.841 | 0.350 |
| arachne supports-on | instrumentation only | 17.328 | 5.393 | 1.420 |

The Arachne supports-on profiled postpass is noticeably slower than its
profiling-off counterpart; individual module brackets also drift. Neither is
a repeatable regression signal or evidence for a new copying candidate.

Serial costs still matter: on profiling-off Arachne supports-on,
`com.core.tree-support-planner` is 6.957 s, `host:shell_classification` 3.234 s,
`host:slice` 2.609 s, and `host:support_analysis` 1.457 s. Planner work already
has the completed carve/union investigation lineage; do not rerun its rejected
projection-cache experiment merely because the stage is still costly.

## New lead: post-hoist linker residual

| fuel-profile cell | linker fuel, billion | share of all guest fuel | module self | linker fuel on layers 0–2 |
| --- | ---: | ---: | ---: | ---: |
| classic supports-off | 79.912 | 9.78% | 99.75% | 65.97% |
| classic supports-on | 79.912 | 9.22% | 99.75% | 65.97% |
| arachne supports-off | 72.716 | 44.51% | 99.73% | 76.52% |
| arachne supports-on | 72.715 | 34.18% | 99.73% | 76.52% |

The expensive Arachne calls persist without profiling: layer 1 linker dispatch
3.182/3.191 s (supports off/on), layer 2 1.389/1.179 s. They are part of the
same-run finishing path, not a summed-worker inference.

Code-derived shape: `link_paths_without_offset`
(`modules/core-modules/infill-linker/src/orchestrate.rs`) already prepares the
universe once, but still invokes `PreparedPolylineClip::clip` for each path.
`PreparedPolylineClip::clip_open_subjects`
(`crates/slicer-core/src/polygon_ops.rs`) creates a new `Clipper64`, adds the
whole prepared clip set and executes each independent intersection. This
identifies a precise **question**, not the measured owner of all module self:
the current profile does not split engine input construction, execution,
metadata conversion and connectivity. The old pre-hoist percentage split is
not a current-job oracle.

Graduated: [Infill-linker post-hoist residual and exact clip-work reduction](../../issues/62-infill-linker-post-hoist-residual.md).

## New lead: low-layer dispatch granularity

Every capture starts layers 0, 1 and 2 sequentially, with layer 2 starting only
after layer 1 completes; layer 2 finishes last in all six runs. For example,
profiling-off Arachne supports-on has these phase-relative intervals:

| layer | start, ms | end, ms |
| --- | ---: | ---: |
| 0 | 0 | 438 |
| 1 | 438 | 4,108 |
| 2 | 4,108 | 5,393 |

All other layers finish by 3,053 ms, leaving a 2,340 ms interval occupied only
by layers 1/2. Supports-off also has a finishing interval (1,038 ms), while
the two Arachne profiles show 3,194/2,978 ms. These are **observed intervals,
not predicted savings**.

`execute_per_layer_with_instrumentation_and_support_tools`
(`crates/slicer-runtime/src/layer_executor.rs`) uses indexed
`global_layers.par_iter().map(...).collect()` with no explicit split-size
bound. The trace has no worker IDs and does **not** prove which worker owns
the pair. It does justify checking whether rayon leaf granularity prevents
these independent expensive layers from overlapping, and whether a generic
granularity change helps without fixture-specific scheduling or output drift.

Graduated: [Independent low-layer dispatch and Arachne finishing-tail reduction](../../issues/63-low-layer-dispatch-granularity.md).
This is independent of the linker-cost question, but the two effects must not
be double-counted and their timing campaigns must not overlap.

## New lead: support renderer self-work

`com.core.tree-support` (the `Layer::Support` renderer) executes 43.432 billion
guest instructions on Classic supports-on and 43.431 billion on Arachne
supports-on, with **96.85% module self**. It is 20.42% of total Arachne
supports-on guest fuel, distinct from `com.core.tree-support-planner`'s
5.275 billion instructions. Those fuel totals do not compare their wall
importance: the planner also invokes native services and is serial.

Renderer module elapsed sums are 7.389 s Classic / 6.402 s Arachne, accumulated
across parallel layers, **not** phase wall or CPU. A wall contribution remains
unmeasured. Supports-off renderer fuel is only 0.109/0.107 billion, a useful
negative control.

Candidate owners to discriminate in `TreeSupport::run_support`
(`modules/core-modules/tree-support/src/lib.rs`) include interface
regularization, role union, skeleton containment, and `scan_fill_region`.
The latter walks every contour/hole edge for every scan row and converts
integer coordinates repeatedly; that loop is verified code, **not yet a
profile-attributed root cause**. Boundary transport and output construction
also need their own brackets.

Graduated: [Tree-support renderer self-fuel and scan-fill attribution](../../issues/64-tree-support-renderer-attribution.md).

## Existing leads, not new tickets

- [Consume-only-when-read annotations](../../issues/20-consume-only-when-read-annotations.md):
  Classic perimeter module self remains 639.971 billion fuel in both support
  cells; the perimeter module is 89.62%/84.51% of whole-job fuel. This supports
  revisiting the existing lead if selected, but does not dynamically prove its
  consumer-liveness audit or lift its standing park.
- [PrePass::Slice flat-bridge closing reduction](../../issues/59-prep-slice-flat-bridge-closing-reduction.md):
  `host:slice` still leads native work, with `closing_ex` 23.069–25.128 s of
  accumulated thread-time across the four profiles; representation analysis
  remains the prerequisite, not a set-algebra shortcut.
- [host:native attribution](../../issues/60-host-native-attribution.md):
  fresh supports-off accumulated work is 11.797 s Classic / 5.716 s Arachne;
  supports-on 21.755/15.595 s. Owner and wall shares remain unresolved.

## Output and warning disclosure

| cell | bytes | SHA-256 prefix | open-loop warnings | malformed-layer-marker warnings |
| --- | ---: | --- | ---: | ---: |
| classic supports-off | 4,332,737 | `34886ccb…` | 0 | 155 |
| classic supports-on | 8,658,663 | `76eb4e1d…` | 0 | 179 |
| arachne supports-off | 4,126,744 | `b76d846a…` | 12 | 152 |
| arachne supports-on | 8,452,736 | `ae9d9061…` | 12 | 177 |

TYPE counts and complete hashes live in each reduced capture. Both Arachne
observer modes reproduce the same per-cell warnings and output. Marker warnings
report fallback to the prior layer-Z context; open-loop warnings explicitly
report a missing closing edge. Their semantic impact was not investigated in
this performance-only refresh. No warning is hidden by the clean completion
status, and no matched-work Orca win is asserted.

One process measurement is an outlier: Arachne supports-off instrumentation
has 26.981 s process wall but 15.504 s `slice_complete.elapsed_ms` (11.477 s
outside that timer), versus 0.178–0.954 s in the other five captures. Its cause
is unisolated; do not call it module cost, instrumentation overhead or
starvation. It was retained/disclosed, not excluded or resampled. The
same-run low-layer brackets remain diagnostic evidence, not a wall verdict.

Historical uninstrumented context, **separate from this batch**: the accepted
finalization campaign's ordinary/accelerated Benchy Classic supports-on
candidate medians are 26.111/28.240 s wall, 137.328/121.297 s CPU (six pairs
each), re-derived from that campaign's `ab-reduction.json`. These are not
fresh Benchy-vs-Orca comparisons and do not fill the Arachne/other-cell gaps.

## Verification and scope

```bash
python docs/specs/perf-vs-orca/evidence/perf-refresh-2026-10-13/verify.py
python docs/specs/perf-vs-orca/evidence/perf-refresh-2026-10-13/reduce.py
# Optional reconstruction of captures from the existing raw tree, no re-slice:
python docs/specs/perf-vs-orca/evidence/perf-refresh-2026-10-13/reduce.py --raw-root target/perf-refresh-2026-10-13
```

The verifier passes from the reduced set alone; its negative controls reject
altered fuel totals, output hashes, missing layers and integrated provenance.
Validation also passed: the release build, ordinary freshness/provenance before
capture, accelerated freshness after capture (no accelerated slices), all six
output-reference checks, Python/PowerShell syntax checks, relative-link checks
and `git diff --check`. Rust tests and workspace clippy were not run: no Rust,
WIT, manifest or production configuration was changed and no commit is being
made. The rebuilt release executable and raw files under `target/` are the
only non-documentation artifacts produced; standing accelerated snapshots were
not changed.
This take closes only the refresh. New tickets are unclaimed investigations;
no implementation, acceptance retry, mode switch or commit was performed.
