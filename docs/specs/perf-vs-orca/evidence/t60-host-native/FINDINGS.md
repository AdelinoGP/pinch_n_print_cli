# host:native attribution: the bucket is fully named (t60)

**Resolution in one line:** in all four classic matched cells the `host:native`
bucket — every marked native `polygon_ops` call no host-built-in bracket claimed
— is **100 % attributed to named owners with zero residual**
(10,647 / 45,099 / 23,874 / 174,265 calls for benchy-off / benchy-on /
base-off / base-on), split across five site families: the region-preparation
derivations, the `region_partition` fill-partition hook, guest-requested host
services from per-layer and prepass modules, the support-plan aggregation, and
the rayon batch-offset items; the top supports-off reducible shape graduated to
[ticket 67](../../issues/67-region-preparation-derivation-sharing.md). No
production source changed in this take.

Route: [host:native attribution: name the per-layer native geometry bucket](../../issues/60-host-native-attribution.md).

## Method and evidence

- The **T60-PROBE** host instrument — a thread-local owner override consulted
  by the native profile sink, plus per-bracket live-span totals, plus a
  worker-thread owner for rayon batch items (the sink's flush consults
  `slicer_wasm_host::batch::probe_owner()`) — was applied to eight host files
  (`profiling_report.rs`, `region_partition.rs`, `layer_executor.rs`,
  `blackboard.rs`, `prepass.rs`, `layer_finalization.rs`, `slicer-wasm-host`'s
  `batch.rs` and `host.rs`), preserved as `probe.patch` (341 lines), applied,
  measured, and **reverted** in this session. Production behaviour and produced
  geometry are unchanged; the probe only tags existing native marks with an
  owner and accumulates guard spans.
- Four fresh ordinary profiled captures (12 threads, matched configs,
  `--profile --profile-verbose --instrument-stderr`, external-module
  provenance, `cargo xtask build-guests --check` exit 0 pre-batch). **Every
  output is byte-identical to its preserved t58 qualification reference**
  (benchy-off `34886ccb…`, benchy-on `76eb4e1d…`, base-off `20e7204c…`,
  base-on `f60e5e9b…`), status ok, degraded=false, zero fatal/non-fatal errors.
- Reduced set: `captures/*.json` + `summary.json` re-derivable by `reduce.py`
  from the raw tree (`target/t60-host-native/`, one-shot, gitignored);
  `verify.py` exits 0 against the in-repo set alone, re-reading the reference
  hashes from the committed t58 qualification and the pre-take bucket counts
  from the committed refreshes, and rejects six mutations (output, owner
  count, dropped owner, probe entry, status, fabricated residual).
- **Units:** every wall figure below is the native sink's accumulated
  thread-time (ADR-0055 / ticket 30), never run wall. `span_ms` in the captures
  are the probe brackets' own live spans. These are attribution captures; they
  make no wall or speed claim. No uninstrumented timing ran.

## The owner split (all four classic cells; calls and accumulated wall)

Baselines (the pre-take `host:native` counts, re-read from the committed
refreshes): benchy cells from `perf-refresh-2026-10-13`, base cells from
`perf-refresh-2026-10-09/reduced/classic-cells.json`. The right-hand columns
are this take's captures.

| owner (probe) | benchy-off | benchy-on | base-off | base-on |
| --- | ---: | ---: | ---: | ---: |
| `d:classic-perimeters` — guest host-service calls in the Perimeters dispatch | 5.91 s | 6.31 s | **89.26 s** | 85.83 s |
| `derive_ordinary` — `ensure_prepared_regions` derivation | 1.48 s | 1.47 s | 40.53 s | 39.42 s |
| `derive_perimeter_source` — `ensure_prepared_perimeter_source_regions` derivation | 1.47 s | 1.54 s | 40.18 s | 40.06 s |
| `region_partition` — `sync_perimeter_infill_areas_into_slice` (3 calls/layer) | 1.26 s | 1.32 s | 10.04 s | 9.84 s |
| `p:seam-planner-default` — prepass guest host-service offset | 0.95 s | 0.94 s | 13.67 s | 14.17 s |
| `d:infill-linker` — linker host-service offset | 0.53 s | 0.62 s | 3.10 s | 3.16 s |
| `p:tree-support-planner` — planner host-service calls | — | 6.49 s | — | **160.33 s** |
| `d:tree-support` — renderer host-service offset | — | 3.79 s | — | 40.29 s |
| `svc_offset_batch` — rayon batch-offset items | — | 0.25 s | — | 2.74 s |
| `plan_aggregation` — host support-plan aggregation clips | — | 0.22 s | — | 2.09 s |
| `d:rectilinear-infill` | 0.001 s | 0.001 s | 0.005 s | 0.004 s |
| **bucket total** | **11.60 s** | **22.94 s** | **176.78 s** | **397.94 s** |
| **residual `host:native`** | **0** | **0** | **0** | **0** |

Call-count reconciliation (per scope, exact; `verify.py` re-derives it):

| cell | scope | pre-take bucket | attributed | residual |
| --- | --- | ---: | ---: | ---: |
| benchy-off | offset / clip | 2,454 / 8,193 | 2,454 / 8,193 | 0 / 0 |
| benchy-on | offset / clip | 15,733 / 29,366 | 15,733 / 29,366 | 0 / 0 |
| base-off | offset / clip | 5,196 / 18,678 | 5,196 / 18,678 | 0 / 0 |
| base-on | offset / clip | 58,391 / 115,874 | 58,391 / 115,874 | 0 / 0 |

What the owners are, code-derived:

- **`derive_ordinary` / `derive_perimeter_source`** — the two `LayerArena`
  prepared-region slots. Each derivation runs once per layer
  (`prepare_regions` → per-region `SliceRegionView::derive_needs_support` plus
  one `intersection_ex` per overhang quartile band): 661 clips per benchy layer
  and 1,926 per base layer, per derivation.
- **`region_partition`** — `sync_perimeter_infill_areas_into_slice`
  (`crates/slicer-runtime/src/region_partition.rs`) runs 3× per layer (the
  `Layer::Perimeters` commit plus one per `Layer::PerimetersPostProcess`
  module — seam-placer and fuzzy-skin): 6,120 clips/benchy, 13,320/base.
- **`d:*` guest-requested host services** — per-layer module dispatches whose
  guests call the WIT host services (`offset_polygons`, `clip_polygons`) rather
  than computing in-guest: classic-perimeters 8 offsets + 3 clips per layer;
  infill-linker 1.2 offsets per layer; (supports-on) tree-support 7.6–10.9
  offsets per layer.
- **`p:*` prepass guest host services** — `seam-planner-default` 1 offset per
  layer in both support states; `tree-support-planner` on supports-on cells.
- **`svc_offset_batch`** — the planner's `offset_polygons_batch` items fanned
  out over rayon: 6,480 items/benchy-on, 24,255/base-on — the latter is exactly
  ticket 22's independently measured batch-item count, and it is why this share
  of the bucket was invisible to a calling-thread bracket.
- **`plan_aggregation`** — the host-side support-plan aggregation
  (`aggregate_support_plan_irs_…`, `prepass.rs`) whose territory clipping pays
  clips only on supports-on cells (the step runs, but emits no marks, when the
  plans are empty).

## Wall context (disclosed; no claim)

Accumulated bucket thread-time versus the same capture's per-layer phase wall:
benchy-off 11.60 s / 7.52 s, benchy-on 22.94 s / 8.35 s, base-off 176.78 s /
120.74 s, base-on 397.94 s / 145.56 s. Mean marked concurrency ≈1.5 (off,
parallel per-layer sites) and ≈2.7 (on, plus prepass/batch sites). Prepass
owners (`p:seam-planner-default`, `p:tree-support-planner`) run on the serial
prepass thread, where a single-threaded span is wall; the batch-item owner is
rayon-fanned and its accumulated value is concurrency-inflated. **No wall
share is measured per owner** — the per-owner brackets' spans and accumulated
walls are in the captures, and the standing paired A/B decides any candidate.

## Reducible shapes named

1. **Region-preparation derivation sharing** (graduated to
   [ticket 67](../../issues/67-region-preparation-derivation-sharing.md)):
   the two derivations process **the same region sets with the same inputs** on
   the matched jobs (`perimeter_source_regions` = `slice.regions` when no
   modifier sub-regions exist), yet both are computed once per layer —
   80.7 s accumulated on base-off (≈46 % of the bucket), 79.5 s base-on,
   ≈2.9–3.0 s on the benchy cells. Candidate: share the derived per-region data
   (identity-keyed) across the two projections, keeping separate derivation
   when the region sets differ (modifier sub-regions), exactness-qualified,
   then the standing paired ordinary + accelerated A/B.
2. **classic-perimeters per-layer host-service round trips** (largest
   supports-off owner; no ticket): 8 offsets + 3 clips per layer, 89.3 s
   accumulated on base-off and 85.8 s on base-on vs 5.9–6.3 s on benchy. The
   counts are layer-shaped, not geometry-shaped; what is reducible there needs
   its own call-site analysis (which `offset_polygons` sites, batching,
   in-guest migration) before a candidate exists.
3. **`region_partition` re-runs the whole partition 3× per layer** (10.0 s
   accumulated base-off, 1.26 s benchy-off): the Perimeters and
   PerimetersPostProcess commits each trigger the full precedence partition;
   dedup needs semantic analysis of what the post-process commit can change.
4. **Supports-on bucket share is the already-mined support machine**: planner
   (160.3 s base-on accumulated; the batch items 2.74 s), renderer (40.3 s),
   aggregation (2.09 s) — the same surfaces tickets 22/32/44/53/55/57/65
   investigate; this take names the bucket's share of them but opens nothing
   new there.

## Disclosure

- Probe-bracket overhead: two `Instant::now()` calls per bracket entry
  (15,849 entries on the base cells, 7,689 on the benchy cells) plus one
  thread-local read per flush; no geometry or output effect — all four outputs
  are byte-identical to their references.
- Instrumented/profiled runs inflate wall (PERF-HANDOFF §3.2); no timing claim
  uses them. The four wall/cpu columns in `runs.csv` are diagnostic context
  only and were not used for any statement above.
- Arachne cells were not re-run (the ticket's focus is the supports-off side;
  the generator-independent owners measured here cover the classic cells). The
  accelerated mode was not run (attribution is mode-invariant at the host
  boundary; the standing A/B gates any candidate).
- Earlier scratch captures for intermediate probe revisions are under
  `target/t60-host-native/` (`*v2*`, `v4-check-*`, `v5-check-*`,
  `runs-v4.csv`); only the final revision's captures are reduced into
  `captures/`. The probe patch ships as the re-appliable artifact.

## Evidence map

- `probe.patch` — the full T60-PROBE diff (8 files); applied for the captures,
  reverted before the resolution commit.
- `run-t60.ps1` — the capture driver (freshness + provenance preflight, four
  jobs, reference-hash and clean-status gates, wall/CPU recorded).
- `captures/*.json` — per-cell reduction (metadata, phases, warnings, probe
  totals, wall-owner table with scopes).
- `summary.json` — owner tables + per-scope reconciliation.
- `reduce.py` — raw-tree reducer and checkout-only summary re-derivation.
- `verify.py` — checkout-only verifier (output identity, clean runs, probe
  shape, per-scope full attribution, summary consistency) + six rejected
  negative controls.
