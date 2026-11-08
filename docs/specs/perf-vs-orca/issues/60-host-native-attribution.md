# host:native attribution: name the per-layer native geometry bucket

Type: task
Label: wayfinder:task
Status: resolved (2026-10-14; the bucket is fully attributed — zero residual
and exact scope-call reconciliation in all four classic cells; one reduction
candidate graduated to
[Region-preparation derivation sharing across region projections](67-region-preparation-derivation-sharing.md);
no production edit — see ## Answer)
Assignee: claimed and resolved by the 2026-10-14 OpenCode wayfinder session
(continue-the-map invocation; claim recorded before any work; first frontier
ticket in order after
[t59](59-prep-slice-flat-bridge-closing-reduction.md) resolved 2026-10-14) —
surfaced by the 2026-10-09 perf refresh
([evidence](../evidence/perf-refresh-2026-10-09/FINDINGS.md))
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none

## Question

The fuel profile's `host:native` fallback bucket (`UNATTRIBUTED_NATIVE_MODULE`,
`crates/slicer-runtime/src/profiling_report.rs`) carries every marked
`polygon_ops` call no host-built-in bracket claimed — e.g. native geometry
reached from the per-layer tier. Fresh attribution at HEAD (2026-10-09
refresh):

| cell | host:native (accumulated) | offset | clip_polygons |
| --- | ---: | ---: | ---: |
| base classic supports-on | 398.2 s | 192.9 s / 58,391 calls | 205.4 s / 115,874 calls |
| base classic supports-off | 197.2 s | 102.9 s / 5,196 calls | 94.3 s / 18,678 calls |
| benchy classic supports-on | 22.9 s | 13.6 s / 15,733 calls | 9.3 s / 29,366 calls |
| benchy classic supports-off | 12.4 s | 7.4 s / 2,454 calls | 5.0 s / 8,193 calls |

The supports-off cells carry a stable per-layer shape (≈10.5 `offset` + ≈37.7
`clip_polygons` calls per layer) nearly independent of object size, so at least
part of the bucket is a per-layer host path rather than the (already-mined)
support machine. **No wall share is known**: the accumulated figures could be
spread across the ~11x-parallel per-layer map or be serial work — do not size
this as wall before measuring.

Deliverable: attribute the bucket at HEAD — split by owner and run a same-run
wall bracket per owner — and name any reducible shape. Candidate owners to
discriminate include `sync_perimeter_infill_areas_into_slice`
(`crates/slicer-runtime/src/region_partition.rs`, run on every Perimeters and
PerimetersPostProcess commit), region preparation (`prepare_regions`,
`crates/slicer-wasm-host/src/marshal/prepared.rs`, covered by ticket 05's arena
cache), anchored-event synthesis, and the per-layer arena/commit paths.

## Acceptance

Attribution only: owner split + per-owner brackets at the matched jobs, with
clean outputs. A reduction candidate graduates separately into an execution
ticket with the standing paired A/B; a "nothing reducible" verdict resolves
this ticket with the owner split as its evidence. No candidate, commit or
default change in this take unless the human extends it.

## Notes

- The supports-on part of the bucket likely overlaps the support-geometry
  machine mined by tickets 22/32/44/53/55/57; the supports-off part is the
  unexplained side and the take's focus.
- Evidence: `evidence/perf-refresh-2026-10-09/FINDINGS.md` (host table and
   per-layer counts).

## Comments

### Benchy bucket refresh — 2026-10-13 (no owner probe or take started)

The [Benchy current bottleneck refresh](61-benchy-current-bottleneck-refresh.md)
measures supports-off `host:native` at 11.797 s Classic / 5.716 s Arachne
accumulated thread-time, supports-on 21.755/15.595 s. These fresh profiles
broaden the generator coverage, not the attribution: owners and wall shares
remain unresolved, so this ticket is retained rather than duplicated.
Evidence: [fresh findings](../evidence/perf-refresh-2026-10-13/FINDINGS.md).

## Answer — fully attributed, zero residual; one candidate graduated

**Resolved 2026-10-14.** The bucket is attributed end to end. Four fresh
ordinary profiled captures (12 threads, matched configs, external-module
provenance, freshness exit 0 pre-batch; every output byte-identical to its t58
qualification reference `34886ccb…` / `76eb4e1d…` / `20e7204c…` / `f60e5e9b…`,
clean, zero fatal/non-fatal) carry a temporarily instrumented host build — the
`T60-PROBE` thread-local owner override in the native profile sink, plus a
worker-thread owner for rayon batch items — applied, measured, and reverted;
it ships as `probe.patch`.

**Every bucket call is now owned, with residual `host:native` = 0** in all four
classic cells (10,647 / 45,099 / 23,874 / 174,265 calls; the per-scope
reconciliation against the pre-take counts is exact — benchy from the
2026-10-13 refresh, base from the 2026-10-09 refresh):

- **region-preparation derivations** — `ensure_prepared_regions` 1.48 s /
  1.47 s / 40.53 s / 39.42 s and `ensure_prepared_perimeter_source_regions`
  1.47 s / 1.54 s / 40.18 s / 40.06 s (benchy-off / benchy-on / base-off /
  base-on accumulated); each derives once per layer, 661 clips/benchy layer,
  1,926/base;
- **`region_partition`** — `sync_perimeter_infill_areas_into_slice` runs 3×
  per layer: 1.26 s / 1.32 s / 10.04 s / 9.84 s (6,120 / 13,320 clips);
- **guest-requested host services from per-layer dispatches** —
  classic-perimeters 5.91 s / 6.31 s / 89.26 s / 85.83 s (8 offsets + 3 clips
  per layer); infill-linker 0.53 s / 0.62 s / 3.10 s / 3.16 s; on supports-on
  also tree-support renderer 3.79 s / 40.29 s;
- **prepass guest host services** — seam-planner-default 0.95 s / 0.94 s /
  13.67 s / 14.17 s (1 offset per layer); on supports-on also the
  tree-support-planner 6.49 s / 160.33 s;
- **batch fan-out** — the planner's `offset_polygons_batch` rayon items,
  0.25 s / 2.74 s (6,480 / 24,255 items; the latter is exactly ticket 22's
  independent batch-item count);
- **support-plan aggregation** — host territory clips on supports-on cells,
  0.22 s / 2.09 s.

**Reducible shapes.** One candidate graduated:
[Region-preparation derivation sharing across region projections](67-region-preparation-derivation-sharing.md)
(the two derivations are the same computation over identical region sets on the
matched jobs; ≈80.7 s accumulated on base-off, ≈46 % of the bucket). Named but
not graduated: the classic-perimeters per-layer host-service round trips (the
largest supports-off owner; needs its own call-site analysis) and the
`region_partition` 3×-per-layer re-run. The supports-on share of the bucket is
the already-mined support machine (planner/renderer/batch/aggregation), now
named rather than suspected. No wall share is claimed for any owner; all wall
figures are accumulated thread-time, and the standing paired A/B governs any
candidate.

Evidence: [findings](../evidence/t60-host-native/FINDINGS.md) — owner tables,
per-scope reconciliation, method, disclosure; `summary.json` / `captures/`
re-verifiable by `verify.py` (exit 0 from the committed set alone; six negative
controls rejected); capture driver `run-t60.ps1`; probe `probe.patch`.
