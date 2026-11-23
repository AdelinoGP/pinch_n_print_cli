# t94 findings — remaining whole-print drop inside G-code emission

Ticket: [Remaining whole-print collection drop inside G-code emission](../issues/94-remaining-whole-print-drop.md).
Take: 2026-10-29, HEAD `57234b81899d0b0b551187c50ada8e7c380d74e6`.
Probe: host-only, `PNP_T94_PROBE=1` (+ `PNP_T94_PARALLEL=1` for the parallel
free arm), applied to `crates/slicer-gcode/src/emit.rs`, reverted before the
take closed; the diff is [`probe.patch`](probe.patch).

**Final ruling: Phase 1 measured-dead — no candidate, no campaign, no
production edit.** The ticket's two declared shapes are closed on measurement,
and the one shape that survives the screen (defer the free into the post-emission
tail) has an **absolute ceiling at the 0.80 % floor**, before any overlap loss.

## Screening cell and method

Two probe slices per arm: **ordinary base classic supports-on** and **ordinary
base arachne supports-on**, the two cells whose t90 brackets carry the drop
(3,616 / 2,961 ms). Matched configs
`evidence/matched-pair/configs/pnp-<generator>-supports-on.json`, `tmp/base.stl`,
`RAYON_NUM_THREADS=12`. The probe splits `DefaultGCodeEmitter::emit_gcode_owned`
into cross-layer rotation, the emission walk, and the collection drop, and the
parallel arm replaces the serial `drop(layer_irs)` with
`into_par_iter().for_each(drop)`. Every run is clean (`degraded=false`,
`fatal=0`, `non_fatal=0`) and its G-code is **byte-identical to the post-t81
reference** (`22200671…` / `0b29e1bf…`, 42,507,912 / 42,330,265 bytes).

Instrumented walls are **not** wall claims: `--profile --profile-verbose` and
the probe's `eprintln` inflate them. They supply the split and the run's own
cpu/wall; the share is wall-equivalent ÷ the cell wall. Cell walls are t91's
kept candidate-arm ordinary walls (`evidence/t91-exact-z-memo/FINDINGS.md`) —
164.803 s classic, 166.931 s arachne — because t91 is the last production
change and t92/t93 changed no source. Floor = **0.80 %** of the cell wall.

## The split

Probe microseconds from the `T94 EMIT` line; bracket milliseconds from the
`host:gcode_emit` `module_complete` event; tail from that event's start to the
postpass `phase_complete` event. `parallel` is the counterfactual arm.

| cell | arm | rotate | walk | **drop** | bracket | tail after emission |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| classic-on | serial | 0.006 ms | 252.794 ms | **3,558.053 ms** | 3,811 ms | **1,603 ms** |
| classic-on | parallel | 0.007 ms | 176.403 ms | **3,237.379 ms** | 3,415 ms | 1,370 ms |
| arachne-on | serial | 0.007 ms | 195.281 ms | **3,242.391 ms** | 3,438 ms | **1,367 ms** |
| arachne-on | parallel | 0.007 ms | 192.669 ms | **3,442.068 ms** | 3,636 ms | 1,395 ms |

Read plainly: the bracket is **93-94 % drop**. Rotation is a no-op (6-7 µs,
as t90 recorded), the walk is 176-253 ms, and the free is 3.24-3.56 s. The
tail after emission — the only work a deferred free could overlap — is
**1.37-1.60 s**: `machine-gcode-emit` 0.53-0.60 s plus `GCodeSerialize`
0.72-0.92 s plus dispatch gaps.

## Shape verdicts

| shape | verdict | measured |
| --- | --- | --- |
| layer-by-layer consumption | **measured-dead (structural zero)** | the walk is serial and totals **176-253 ms**; freeing each layer as it is emitted interleaves the same serial frees with that work instead of overlapping them. No other thread runs during the walk. |
| parallel free (`into_par_iter`) | **measured-dead** | classic **−0.321 s (＋0.19 % of cell)**, arachne **+0.200 s (−0.12 %)** — a wash across the two cells, i.e. **0.0-0.2 %**, vs the **1.32-1.34 s** floor. The free does not scale: 12 threads buy 9 % in one cell and lose 6 % in the other. Windows' process heap serialises the frees; the work is not parallelisable at this allocation shape. |
| defer the free into the post-emission tail | **at the floor, no margin** | absolute ceiling = the tail: **1.603 s classic = 0.97 %**, **1.367 s arachne = 0.82 %** of the cell, against a **1.318 / 1.335 s** floor (**1.22x / 1.02x**). That is the *ceiling* before any overlap loss, thread wake-up, or allocator contention with the concurrent 42 MB serialisation. It cannot win by more than the tail, and it cannot win the whole tail. |

Cross-check that the wall readings are noise-bound at this sample size: the
rachne serial arm's instrumented wall is 164.664 s and the parallel arm's
172.553 s **while the drop was 0.20 s *slower* in the parallel arm** — a 4.8 %
wall difference from a 0.2 s work difference. One run per arm per cell, so no
wall claim is made from these; the split is internal and stable (drop spread
±5 % across four runs).

## What this leaves (fog, not this ticket)

The drop is **not a drop-strategy problem**; it is an **allocation-volume**
problem. ~5.3 GB peak working set is measured on this cell class (t79), and
3.2-3.6 s of serial heap-walk free is the price. The shape that could move it
is fewer/larger allocations — an arena or bulk-free layer collection — which is
a structural IR/allocator change, not a postpass or emitter tweak. Parallel
free, deferred free and layer-by-layer consumption are all measured at or
below the floor. Recorded in the map's fog.

## Disclosures

- Machine load was **8 %** before the first run (not a quiet campaign); Phase 1
  is attribution, and the internal split is stable across the four runs. No
  wall claim is made.
- One probe run per arm per cell (four runs total); **no paired campaign was
  started**, because the screen does not leave a shape with margin above the
  floor.
- The parallel arm changes no emitted byte: both arms produce the same SHA in
  both cells. It is a counterfactual, never a candidate.
- The arachne drop (3.24 s) is larger than t90's bracket-minus-walk estimate
  (2.77 s) because t90's 2,961 ms was a different run and did not split the
  bracket; today's arachne bracket is 3.438 s serial. The premise is re-derived,
  not inherited.
- The probe is stashed, not deleted; `probe.patch` is the exact diff against
  HEAD `57234b81`. Both guest freshness gates exit 0 at close — the ordinary
  gate before and after the take, the accelerated gate at close; the probe is
  host-only.
- `verify.py --self-test` exits 0 against the committed `summary.json` alone,
  with 8/8 negative controls rejected.

## Reproduce

```text
pwsh docs/specs/perf-vs-orca/evidence/t94-remaining-drop/build-probe.ps1
pwsh .../run-probe.ps1 -Generator classic -Parallel 0
pwsh .../run-probe.ps1 -Generator classic -Parallel 1
pwsh .../run-probe.ps1 -Generator arachne -Parallel 0
pwsh .../run-probe.ps1 -Generator arachne -Parallel 1
python docs/specs/perf-vs-orca/evidence/t94-remaining-drop/extract.py
python docs/specs/perf-vs-orca/evidence/t94-remaining-drop/verify.py --self-test
```
