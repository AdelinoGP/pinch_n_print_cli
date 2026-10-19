# `build_wall_flags` annotation-free fastpath

**Final disposition:** user accepted the candidate for CPU savings and committed
it; wall improvement remains unproven. Historical provisional conclusions below
are superseded. **Evidence correction:** quiet-validation runs labeled Arachne
actually used the Classic config. Do not use those rows as Arachne evidence.
Earlier correctly configured interleaved Arachne runs remain valid. See the
correction at the top of `FINDINGS.md` and §13 of `tmp/PERF-HANDOFF.md`.

Date: 2026-09-07. This is a bounded, uncommitted optimization experiment.

## Candidate

`crates/slicer-core/src/perimeter_utils.rs` now seeds the existing defaults and
`variant_fuzzy`, then checks only effective `Material::ToolIndex` and
`FuzzySkin::Flag(true)` values. If neither semantic can affect the selected
polygon (or any polygon reachable by reprojection), it returns immediately with
the existing `ExteriorSurface`/`Interior` fallback. The reprojection, index
sampling, transition calculation, and callers are unchanged.

## Artifact isolation

The baseline host and complete `modules/core-modules` tree were copied to
`tmp/perf-flags/baseline-artifacts/` before editing. Baseline diagnosis is in
`baseline-diagnose.json`; candidate diagnosis is `candidate-diagnose.json`.
Both diagnosis commands exited 0 and selected 23 external modules, including
the classic and Arachne perimeter guests. Baseline and candidate artifact
hashes are recorded in the command log/session output; candidate freshness was
checked with `cargo xtask build-guests --check` (exit 0) after rebuilding.

The harness used `tmp/alloc-bench/run_bench.ps1`, 12 Rayon workers, no report,
and no instrumentation. The initial batches were separate baseline/candidate
batches. Follow-up batches used the required baseline, candidate, baseline,
candidate order for each generator. Raw CSV, G-code, and stderr JSONL files
are in this directory. Every completed run exited 0.

## Measurements

Medians are shown with observed ranges in seconds; baseline and candidate were
run in separate batches against their isolated host/module trees.

| fixture / generator | baseline wall | candidate wall | baseline CPU | candidate CPU |
|---|---:|---:|---:|---:|
| Benchy / classic (3 runs) | 23.8106 (22.0081–27.6537) | 20.9957 (20.9821–21.5860) | 155.8594 (154.4219–155.8594) | 142.8906 (141.2656–144.1250) |
| Benchy / Arachne (3 runs) | 21.3138 (20.5626–24.3675) | 18.1202 (17.8115–18.9587) | 79.5469 (79.0156–80.2969) | 73.2812 (72.8125–73.8281) |
| base / classic (2 runs) | 336.9057 (277.3875–396.4238) | 248.3027 (243.8671–252.7383) | 2457.3360 (2455.7344–2458.9375) | 2242.0547 (2230.0312–2254.0781) |
| base / Arachne (2 runs) | 249.8346 (240.5731–259.0961) | 132.4323 (131.7608–133.1038) | 1086.7188 (1072.7969–1100.6406) | 1002.4766 (1000.7656–1004.1875) |

CPU reduction is consistent across both generators and both fixtures (about
8% in each small sample). Benchy wall medians improved in both batches. Base
classic wall is not reliable because its two baseline runs varied widely;
base Arachne wall was much tighter and improved in both runs. Peak working set
was not a target and showed no consistent reduction.

Semantic checks: Arachne G-code bytes, line counts, type-marker counts, and
hashes were identical baseline/candidate. Classic retains its known small
run-to-run variation; line/type counts stayed comparable and all runs exited
0. Baseline and candidate base stderr `slice_complete` records both reported
zero fatal and zero non-fatal errors for both generators in the captured runs.

## Recommendation

**PROVISIONAL candidate; ACCEPTANCE INCONCLUSIVE.** The fastpath is small,
annotation-aware, covered by focused tests, and shows consistent CPU improvement
in both requested generators across supports-off Benchy, original tree-support
Benchy, and repeated base measurements. Controlled interleaved wall readings
moved lower on Benchy but remain load-qualified; with two repeats per side, the
controlled-wall acceptance gate is not demonstrated. Base wall readings are
inconclusive. No source changes were made during this validation.

## Follow-up acceptance measurements

The following are separate from the initial-batch table above. They record
every sample's wall and CPU values, rather than only medians.

### Interleaved Benchy, supports disabled

Each generator ran baseline, candidate, baseline, candidate with 12 Rayon
workers. Baseline used the preserved executable and complete module snapshot;
candidate used the release executable and current module directory.

| generator | sample | wall s | CPU s | bytes | exit |
|---|---:|---:|---:|---:|---:|
| classic | baseline 1 | 36.1951 | 165.8438 | 4,392,360 | 0 |
| classic | candidate 1 | 36.1311 | 141.0938 | 4,393,009 | 0 |
| classic | baseline 2 | 36.4534 | 166.9688 | 4,392,386 | 0 |
| classic | candidate 2 | 33.9544 | 147.4531 | 4,392,827 | 0 |
| Arachne | baseline 1 | 26.7840 | 80.4688 | 4,017,930 | 0 |
| Arachne | candidate 1 | 25.5349 | 78.4688 | 4,017,930 | 0 |
| Arachne | baseline 2 | 25.9159 | 80.5312 | 4,017,930 | 0 |
| Arachne | candidate 2 | 24.7800 | 79.2344 | 4,017,930 | 0 |

Medians: classic wall 36.3243/35.0428 s and CPU 166.4063/144.2735 s;
Arachne wall 26.3500/25.1575 s and CPU 80.5000/78.8516 s. Wall is
load-qualified and not claimed as a controlled speed result. All captures were
`ok` with zero fatal and zero non-fatal errors.

### Interleaved Benchy, original tree-support configuration

This used `tmp/alloc-bench/config.json` (`tree(auto)`, supports enabled) for
classic and an equivalent copy with `wall_generator: "arachne"` for Arachne.
The order was baseline, candidate, baseline, candidate.

| generator | sample | wall s | CPU s | bytes | exit |
|---|---:|---:|---:|---:|---:|
| classic | baseline 1 | 44.9295 | 168.8438 | 7,640,939 | 0 |
| classic | candidate 1 | 44.5201 | 167.8438 | 7,640,623 | 0 |
| classic | baseline 2 | 44.2824 | 174.3594 | 7,640,187 | 0 |
| classic | candidate 2 | 44.0760 | 162.9219 | 7,640,376 | 0 |
| Arachne | baseline 1 | 35.2290 | 98.9531 | 7,265,355 | 0 |
| Arachne | candidate 1 | 34.4486 | 92.8906 | 7,265,355 | 0 |
| Arachne | baseline 2 | 34.2534 | 98.1562 | 7,265,355 | 0 |
| Arachne | candidate 2 | 34.5100 | 96.3906 | 7,265,355 | 0 |

Medians: classic wall 44.60595/44.29805 s and CPU 171.6016/165.38285 s;
Arachne wall 34.7412/34.4793 s and CPU 98.55465/94.6406 s. All captures
were `ok` with zero fatal and zero non-fatal errors. Arachne hashes matched;
classic remained nondeterministic.

### Repeated base, original tree-support configuration

Two baseline and two candidate runs completed per generator. The first grouped
command hit its 40-minute command limit after all classic rows and the first
Arachne baseline; the remaining three Arachne rows were then completed
individually. The limit was a harness timeout, not a slice failure.

| generator | sample | wall s | CPU s | bytes | exit |
|---|---:|---:|---:|---:|---:|
| classic | baseline 1 | 566.5472 | 2905.4688 | 30,163,402 | 0 |
| classic | candidate 1 | 505.3584 | 2559.8750 | 30,163,520 | 0 |
| classic | baseline 2 | 524.3012 | 2788.1250 | 30,163,988 | 0 |
| classic | candidate 2 | 429.6910 | 2561.8594 | 30,163,688 | 0 |
| Arachne | baseline 1 | 320.6457 | 1376.3906 | 28,371,594 | 0 |
| Arachne | candidate 1 | 386.2650 | 1351.5000 | 28,371,594 | 0 |
| Arachne | baseline 2 | 372.9869 | 1376.2344 | 28,371,594 | 0 |
| Arachne | candidate 2 | 332.7867 | 1333.5000 | 28,371,594 | 0 |

Medians: classic wall 545.4242/467.5247 s and CPU 2846.7969/2560.8672 s;
Arachne wall 346.8163/359.52585 s and CPU 1376.3125/1342.5000 s. Base wall
is load-qualified and inconclusive. Every capture reported `degraded: true`,
29,108 non-fatal errors, and zero fatal errors for both baseline and candidate,
preserving DEV167 visibility.

## Quiet-machine controlled wall follow-up

Raw artifacts are under `tmp/perf-flags/quiet-validation/`. Before timing,
`cargo xtask build-guests --check` returned exit 0. Each side/generator got one
explicit warmup, then ABBA BAAB (four samples per side), 12 Rayon workers, no
report, and no instrumentation. All measured runs exited 0.

Candidate/base median CPU ratios were 0.9139 Classic and 0.9143 Arachne with
supports off, and 0.9268 Classic and 0.9201 Arachne with supports on. Wall
ratios were 1.0114/0.9760 (Classic/Arachne) supports off and 1.0174/0.9745
supports on. CPU is favorable; wall acceptance remains inconclusive because
ranges overlap and the before/after load snapshots (12% then 7% aggregate CPU
load) cannot prove continuous idle conditions. Recommendation remains
**INCONCLUSIVE awaiting user decision**; the candidate is not auto-dropped.
