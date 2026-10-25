# Base common-floor attribution — SeamPlanning and OverhangAnnotation

Route: [Base common-floor attribution](../../issues/84-base-common-floor-attribution.md).
Attribution take only: no candidate, no production edit, no default change. The
probe is env-gated (`PNP_T84_PROBE=1`), preserved as [`probe.patch`](probe.patch),
and was reverted before the take closed.

## Verdict

**One shape graduates, two are measured dead.**

| stage | stage wall (base classic off / on) | share of cell wall | where the wall goes |
| --- | --- | --- | --- |
| `PrePass::OverhangAnnotation` | 10.01 s / 10.30 s | 10.78 % / 4.94 % | **99.7 %** parallel kernel; **85.4 %** of its work is the canonical three round offsets per overhang layer |
| `PrePass::SeamPlanning` | 9.14 s / 8.97 s | 9.84 % / 4.30 % | **92.4 / 93.2 %** is one serial native `offset_polygons` call per region (495 calls) |

Graduated: **batch the seam planner's per-region inset offsets** through the
existing `offset_polygons_batch` (`crates/slicer-wasm-host/src/host.rs`) — upper
bound **8.83 %** of the off cell's wall (3.92 % on), and **5.43 % / 2.41 %** at
the 2.6x effective speedup t78 measured for the same `map_batch` mechanism.
Both clear the ticket's pre-declared **0.80 %** floor.

Dropped, with measured ceilings:

| shape | ceiling (off / on cell) | why |
| --- | ---: | --- |
| Overhang bbox-window operand | 0.189 % / 0.080 % | a `current` bbox + 0.7 mm window still retains **97.84 %** of the previous layer's points |
| Seam candidate list | 0.288 % / 0.127 % | 2,553,550 candidates cost 94.6 ms generation + 16.6 ms scoring + 58.1 ms harvest |

The candidate list was the obvious suspect (one allocated `ScoredSeamCandidate`
per boundary vertex, all retained in `SeamPlanIR`) and it is **not** the wall
problem: the seam stage's wall is native clipper work, not guest compute. The
reverse is true of overhang: its host-side gather/merge/clone is 0.18 % of the
stage, so the t74/t58 whole-print-copy pattern does **not** repeat here.

## Method

- HEAD `ce8f3992`. Two ordinary captures per cell, one binary at HEAD
  (`t84-head`, profiling off) and one on the probe build (`t84-probe2`,
  `--profile --profile-verbose`), plus one operand-counter run
  (`t84-probe3`). 12 rayon threads, 24/24 external modules,
  `cargo xtask build-guests --check` exit 0 before the batch.
- Cells: `base classic supports-off` and `base classic supports-on`, the two
  base classic cells (the common floor is shared by all four base cells; t73's
  four-cell table shows both stages within 9.3–10.9 s on every one).
- Both arms produce **byte-identical** output, equal to the t81 campaign
  references: off `292df501…` / 21,023,739 B, on `22200671…` / 42,507,912 B.
  Every capture is `status=ok`, `degraded=false`, 0 fatal / 0 non-fatal.
- **Restore check:** after reversing `probe.patch`, a fresh HEAD release build
  with the rebuilt guest reproduced the off reference `292df501…` exactly
  (98.2 s process wall, +3.8 % vs the first HEAD capture — machine drift), and
  `cargo xtask build-guests --check` exited 0.
- Instrumented wall is attribution only, never a wall claim. The **cell wall**
  used for share-to-wall conversions is the HEAD run's `slice_complete`
  (92,829 ms / 208,632 ms; process walls 94.6 s / 210.7 s).
- The probe terms are stopwatch brackets: host terms in
  `commit_overhang_annotation_builtin`
  (`crates/slicer-runtime/src/builtins/overhang_annotation_producer.rs`), and
  view-call-harvest terms around the `PrePass::SeamPlanning` dispatch
  (`crates/slicer-wasm-host/src/dispatch.rs`). Guest shares come from
  `--profile` scopes added to
  `run_region_planning_entries`
  (`modules/core-modules/seam-planner-default/src/lib.rs`); native clipper
  shares come from the `host:overhang_annotation` scope rows.

## OverhangAnnotation

Terms on the probe run (ms):

| term | classic-off | classic-on | share of stage |
| --- | ---: | ---: | ---: |
| gather (`SliceIR` → owned `layer_footprints`) | 10.6 | 9.2 | 0.11 / 0.10 % |
| kernel (`annotate_overhangs`) | 9,482.7 | 9,022.3 | **99.73 / 99.73 %** |
| merge kernel maps | 1.1 | 0.9 | 0.01 % |
| `SurfaceClassificationIR` clone | 5.1 | 4.9 | 0.05 % |
| commit | 0.0 | 0.0 | 0.00 % |

The kernel's native work, from the profile's `host:overhang_annotation` scopes:

| op | calls | accumulated (off) | accumulated (on) |
| --- | ---: | ---: | ---: |
| `polygon_ops::offset` | 1,455 | 95,956.7 ms | 91,289.0 ms |
| `polygon_ops::clip_polygons` | 4,859 | 16,468.4 ms | 15,715.9 ms |

1,455 = **485 band layers × 3**, exactly `partition_into_bands`'s three round
offsets at `line_width × {0.5, 1.0, 1.5}`. The kernel's measured parallelism is
**11.86x** on both cells (native accumulated ÷ kernel wall), so the offsets are
**85.4 %** of the kernel's work and **8.72 %** of the off cell's wall if they
were free.

**No representation-safe shape was found.** The three offsets are canonical
band geometry, the kernel is already near-fully parallel across layers, and the
one operand-side idea — clip `previous` to `current`'s bbox + 0.7 mm before
offsetting — removes at most **2.16 %** of the operand points
(2,540,125 counted, 2,485,164 inside the window, `retained_frac = 0.978363`),
i.e. a ceiling of **0.189 %** (off) / **0.080 %** (on) of wall. The stage is
engine-bound, not shape-bound; making the offsets themselves cheaper is t70's
territory, not a new candidate.

## SeamPlanning

Terms on the probe run (ms):

| term | classic-off | classic-on | share of stage (off / on) |
| --- | ---: | ---: | ---: |
| host view projection | 4.2 | 11.4 | 0.05 / 0.13 % |
| guest call | 8,777.0 | 8,697.1 | 99.0 / 99.1 % |
| — `push_seam_plan` (495 pushes) | 0.04 | 0.05 | 0.00 % |
| — scope `t84.offset_project` | **8,195.4** | **8,177.4** | **92.44 / 93.19 %** |
| — scope `t84.candidates` | 94.6 | 94.3 | 1.07 % |
| — scope `t84.choose` | 16.6 | 16.1 | 0.19 / 0.18 % |
| — unscoped guest (sort, keys, entry build) | 98.4 | 109.0 | 1.11 / 1.24 % |
| host harvest → `SeamPlanIR` | 58.1 | 44.7 | 0.66 / 0.51 % |

Counts: 495 regions, 845 polygons, 2,553,550 boundary points, **2,553,550
candidates**, 495 entries. The guest fuel split is consistent
(3.86 B total: candidates 794.6 M, choose 275.7 M, offset/project 402.6 M,
2.39 B unscoped) but fuel is *not* the wall signal here: candidate generation is
20.6 % of guest fuel and 1.07 % of stage wall, while the offset scope is 10.4 %
of fuel and 92.4 % of wall. That gap is the finding — the guest waits on 495
serial host calls.

The offset is a PnP device, not canonical geometry: each region's chosen
candidate is projected onto a `-0.5 × width` inset of the region boundary
(`project_point_onto_inset_boundary`, `seam-planner-default/src/contours.rs`).
Each call is independent, input-ordered and already batched-capable:
`offset_polygons_batch` exists and routes through `map_batch`, the same
estimator t78 used for `clip_polygons_batch`.

## Shape pricing

`reduce.py` derives the ceilings from the committed captures; `summary.json` is
the reduced artifact.

Seam batching has two bounds, both above the floor in both cells:

| cell | upper bound `offset_project / cell wall` | at t78's measured 2.6x |
| --- | ---: | ---: |
| base classic off | 8.83 % | **5.43 %** |
| base classic on | 3.92 % | **2.41 %** |

The upper bound assumes every offset disappears; the 2.6x bound applies the
measured effective speedup of the same `map_batch` fan-out on the tree-support
planner (t78). The candidate ticket must measure its own batch occupancy rather
than inherit 2.6x.

## Disclosures

- **Machine drift in `t84-probe3`.** Its kernel bracket is 11,910.8 ms against
  probe2's 9,482.7 ms (+25.6 %) under the user's desktop load. Only the
  operand-window **count** (load-independent) is taken from that run; its wall
  numbers are not used anywhere.
- `--profile` inflates wall at every scope mark (docs/17_agent_debugging.md);
  scope walls are used as internal shares, never as candidate wall claims.
- One object (`base.stl`), 495 layers; counts are per-object.
- The probe build differs from HEAD in one guest artifact (seam-planner rebuilt
  with a `slicer-core` dependency and three scopes) and in the host probe
  brackets. Its output is byte-identical to the HEAD arm, and the source was
  restored to HEAD with `probe.patch` reversed.
- `D-168` and the map's fairness contract are untouched by this take.

## Reproduce

```bash
# 1. HEAD arm (profiling off), then apply probe.patch and rebuild
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t84-common-floor/run_capture.ps1 \
    -Cell base-classic-off -Label head -BatchId t84-head
# 2. probe arm (profiling on)
pwsh -NoProfile -File .../run_capture.ps1 -Cell base-classic-off -Label probe -Probe -Profile -BatchId t84-probe2
#    (the same probe build also emitted the t84-probe3 operand counter)
# 3. reduce + verify
python docs/specs/perf-vs-orca/evidence/t84-common-floor/extract.py --raw target/t84-common-floor
python docs/specs/perf-vs-orca/evidence/t84-common-floor/reduce.py
python docs/specs/perf-vs-orca/evidence/t84-common-floor/verify.py --self-test
```

`verify.py --self-test` exits 0 checkout-only and rejects 8/8 negative controls
(a moved kernel share, a wrong offset-call count, false parallelism, a reduced
seam-offset share, an inflated candidate list, a changed candidate count, a
moved probe output, and a false operand retention).
