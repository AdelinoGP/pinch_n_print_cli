# Benchy perimeter attribution study

Date: 2026-09-06/07. This is a temporary study; no optimization is proposed or
implemented here.

## Method

- Fixture: `tmp/3dbenchy.stl` (225,786 triangles).
- Binary: `target/release/pnp_cli.exe`, external modules only from
  `modules/core-modules`.
- Dispatch verification: `tmp/perf-perimeters-study-diagnose.json` reports 23
  modules, all with provenance `external`, including both perimeter guests.
- Config: copies of `tmp/alloc-bench/config.json`, with support disabled. The
  Arachne copy additionally sets `wall_generator: "arachne"`; the classic copy
  leaves the historical default. Both retain 0.2 mm layer height, 0.5 mm
  nozzle, 3 walls, 25% sparse infill. Supports are not part of this study.
- Three probe-free runs per generator used
  `tmp/alloc-bench/run_bench.ps1`, 12 Rayon workers, capturing Stopwatch wall,
  Windows process CPU (`TotalProcessorTime`), peak working set, output bytes,
  hash, and exit code. Attribution captures used `--instrument-stderr` and
  `--profile` separately; their wall times are not baseline timings.
- `cargo xtask build-guests --check` returned exit 0 before baseline captures.

## Probe-free baselines

| generator | wall samples (s) | CPU samples (s) | CPU/wall | output bytes | completion |
|---|---:|---:|---:|---:|---|
| classic | 20.3366, 20.0881, 20.9737 | 154.9531, 156.2969, 155.4375 | 7.62, 7.78, 7.41 | 4,392,519–4,392,923 | 3/3 exit 0 |
| Arachne | 17.5736, 16.4512, 16.3696 | 76.4062, 76.5625, 76.1250 | 4.35, 4.65, 4.65 | 4,017,930 | 3/3 exit 0 |

The output hashes were stable for Arachne and varied slightly for classic;
both runs completed cleanly with no fatal or non-fatal module errors in the
profile captures. These are generator observations under one config, not a
cross-generator speed claim: the emitted geometry and work differ.

## Classic findings

The instrumented capture (`classic-instrumented.jsonl`) reports 240 external
`com.core.classic-perimeters` calls and 102.069 s accumulated worker elapsed.
The profile capture (`classic-breakdown-2.jsonl`) reports 1,219,555,495,548
fuel **for the classic module**, not the whole slice. The whole-slice guest
denominator is 1,341,279,290,146 fuel. `polygon_ops::offset2_ex` accounts for
172,132,309,250 module fuel in 960 calls (12.8% of whole-slice fuel, 14.1% of
classic-module fuel). `opening_ex` is not a separate profile row because it
delegates to `offset2_ex`.

Temporary guest scopes provide the requested module-self breakdown:

| classic scope | calls | total fuel | interpretation |
|---|---:|---:|---|
| `run_perimeters` | 240 | 1,219,054,710,565 | nearly all module work; 6,102,932 self fuel outside nested scopes |
| `emit_walls` | 240 | 1,174,105,678,478 | 96.3% of module fuel; 412,675,373 self fuel after nested scopes |
| `wall_assembly` | 240 | 1,046,237,680,537 | 85.8% of module fuel, all self in this capture |
| `gap_fill` | 240 | 97,696,936,016 | 8.0% of module fuel; only 11,948,027 self, mostly nested geometry |
| `insets` | 240 | 253,994,446 | 0.02% of module fuel |

This identifies wall assembly as the dominant measured classic block, with
gap-fill geometry a secondary candidate. The scope nesting was sequential and
non-overlapping: `run_perimeters` encloses each `emit_walls` call;
`emit_walls` encloses the inset, wall-assembly, thin-wall, gap-fill, seam, and
infill-transition work; `insets` is dropped before `wall_assembly`, which is
dropped before `gap_fill`. Core `offset2_ex` marks nest inside whichever
operation invokes them. These are fuel attribution results, not wall or CPU
shares.

The preserved probe locations were the start of `ClassicPerimeters::run_perimeters`
and `ClassicPerimeters::emit_walls` (`modules/core-modules/classic-perimeters/src/lib.rs`).
`insets` covered the iterative `for i in 0..wall_count` loop;
`wall_assembly` covered the `all_wall_polygons` iteration through
`build_ring_wall`, bridge-flag filling, wall ordering, and `push_wall_loop`;
`gap_fill` covered its conditional through emitted loops.

Host-side profile attribution is separate native wall accounting: 54.492 s
total across built-ins in the profiled run. `host:slice` is 24.712 s, led by
`polygon_ops::closing_ex` at 21.273 s; `host:native` is 11.576 s, including
6.284 s offset and 5.271 s clip; `host:shell_classification` is 10.119 s.
These are indicative profiling-wall values, not baseline wall or CPU.

**Finding:** the classic guest accounts for 90.9% of the guest-fuel denominator
in this profile, but native host time is a separate unit and cannot be combined
with that percentage. Within the classic guest, `wall_assembly` is the largest
measured block (85.8% of classic-module fuel); the earlier generic module-self
description is superseded. `offset2_ex`/`opening_ex` remains a confirmed nested
sub-scope. The capture does not isolate all dispatch setup from
other host work, so no dispatch-insignificance claim is made. A host migration
experiment for `offset2_ex` and
the `opening_ex` alias remains a contract/design question, not an authorized
change.

## Arachne findings

The instrumented capture (`arachne-instrumented.jsonl`) reports 240 external
`com.core.arachne-perimeters` calls and 15.203 s accumulated worker elapsed.
ADR-0055 fuel reports 179,110,293,459 fuel for that guest module, all as
`<module self>`; it reports no polygon scope inside the module. This is not
an algorithm-cost measurement: Arachne's algorithm is executed in the host
`generate-arachne-walls` service, which calls
`slicer_core::arachne::pipeline::run_arachne_pipeline`.

The profiled host-native section totals 49.431 s indicative wall. It includes
`host:slice` 24.741 s, `host:shell_classification` 10.530 s,
`host:overhang_annotation` 8.063 s, and `host:native` 5.969 s. Within
`host:native`, marked geometry includes 4.469 s clip, 0.760 s offset, and
0.719 s offset2_ex. Those generic rows cannot isolate the Arachne host service
from other native callers.

The temporary host probe (`arachne-stage-probe-2.jsonl`) does isolate it over
240 calls. The enclosing pipeline interval was 4,739.938 ms; input conversion
was 0.390 ms before it and output conversion was 4.338 ms after it. Named
pipeline stages sum to 4,375.007 ms: preprocess 1,782.764 ms, graph
construction 1,785.305 ms, centrality/bead-count/filtering 181.541 ms,
transitions and extra ribs 14.560 ms, beading propagation 240.315 ms, and
toolpath postprocess 370.522 ms. Thus preprocess plus graph are 3,568.069 /
4,739.938 = **75.3% of the enclosing pipeline interval** (not 81.5%). The
named-stage sum is 92.3% of that interval; the unattributed remainder is
364.932 ms (7.7%), covering setup/reordering and gaps between coarse brackets.
These are accumulated host elapsed across 240 calls, not process CPU or
whole-slice wall.

**Finding:** Arachne's host pipeline is dominated by graph construction and
input preprocessing in this capture, together 75.3% of the enclosing pipeline
interval. The conversion boundaries are directly measured and small in this
capture. This is an attribution ranking, not an optimization or speedup claim.

## Evidence-ranked next investigations

### Classic

1. Split `wall_assembly` around the `build_ring_wall` closure and its internal
   `build_wall_flags`, per-vertex bridge flag/flow loop, and
   `PerimeterOutputBuilder::push_wall_loop` calls
   (`modules/core-modules/classic-perimeters/src/lib.rs`). This is highest
   confidence because the enclosing scope measured 85.8% of classic-module
   fuel while these contributors remain combined.
2. Attribute the nested `gap_fill` path at its `opening_ex`, `offset2_ex`,
   `clip_polygons`, and medial-axis calls in `ClassicPerimeters::emit_walls`
   (`modules/core-modules/classic-perimeters/src/lib.rs`). It measured 8.0% of
   classic-module fuel and only 11,948,027 self fuel, so the question is which
   nested operation owns it.
3. Add a classic-only host bracket around input preparation, typed guest call,
   and output conversion in `WasmRuntimeDispatcher::run_stage`
   (`crates/slicer-wasm-host/src/dispatch.rs`). This is lower priority than
   the measured guest blocks, but is required before claiming dispatch or
   conversion significance.

### Arachne

1. Refine `SkeletalTrapezoidationGraph::from_polygons`
   (`crates/slicer-core/src/skeletal_trapezoidation/`) because graph
   construction measured 1,785.305 ms and tied preprocessing for the largest
   named contribution.
2. Split `preprocess_input_outline`
   (`crates/slicer-core/src/arachne/preprocess.rs`) into cleanup,
   degeneracy/duplicate filtering, and morphology portions; it measured
   1,782.764 ms.
3. Resolve the 364.932 ms coarse-bracket remainder by bracketing parameter /
   strategy setup and `reorder_by_region_order` in
   `crates/slicer-core/src/arachne/pipeline.rs`. Conversion is lower priority:
   `wit_to_ir_expolygons` and the `to_wit_lines` closure in
   `crates/slicer-wasm-host/src/host.rs` measured 0.390 ms and 4.338 ms
   accumulated, respectively.

## Caveats and next experiments

- Do not sum guest fuel, accumulated worker elapsed, native host wall, process
  CPU, or process wall; they are different units and scopes.
- `--instrument-stderr` and `--profile` affect throughput and are attribution
  captures only.
- The machine had other Cargo/test activity during the build phase; no speedup
  claim is made from these samples.
- The host-stage probe was temporary and has been removed; the final release
  binary is probe-free.

Raw files are in this directory: CSV baselines, JSONL instrumented/profile
captures, fuel summaries, and the two derived configs.

Final cleanup validation: temporary probe grep returned no matches; `git status`
and `git diff --check` were clean; `cargo build --release --bin pnp_cli` passed;
`cargo xtask build-guests --force` rebuilt production guests and the final
`cargo xtask build-guests --check` returned exit 0; final module diagnosis again
reported 23 external modules.
