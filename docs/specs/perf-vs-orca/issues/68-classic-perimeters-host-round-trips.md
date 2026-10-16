# Classic-perimeters per-layer host-service round trips

Type: task
Status: claimed
Assignee: claimed 2026-10-16 — session resumed on the human's "continue"
(no ticket named); the human then selected this lead from the t60
un-graduated set as the route for the take (`region_partition` dedup and
consume-only-when-read stay un-graduated). Graduated by the 2026-10-14
[host:native attribution](60-host-native-attribution.md) take
([findings](../evidence/t60-host-native/FINDINGS.md)).
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none

## Question

Analyze the classic-perimeters per-layer host-service call sites, reduce
their measured round-trip cost with an exactness-qualified candidate, and
decide keep/drop on the standing paired gate.

t60 measured `d:classic-perimeters` — guest-requested host services in the
Perimeters dispatch — at 8 `offset_polygons` + 3 `clip_polygons` calls per
layer: **5.91 s benchy-off / 6.31 s benchy-on / 89.26 s base-off / 85.83 s
base-on** accumulated thread-time — the largest supports-off `host:native`
owner after the resolved (KEEP 2026-10-16) region-preparation sharing
([findings](../evidence/t60-host-native/FINDINGS.md)). The counts are
layer-shaped, not geometry-shaped; what is reducible needs its own call-site
analysis ("which `offset_polygons` sites, batching, in-guest migration").

## Notes

- **The per-layer call set** (measured: 1,920 offsets + 720 clips over 240
  benchy layers; 3,958 + 1,484 over 495 base layers — exactly 8 offsets + 3
  clips per layer, ±2 calls on base). All sites live in `emit_walls`
  (`modules/core-modules/classic-perimeters/src/lib.rs`), reached once per
  region per layer via `run_perimeters`'s default path (the matched job is
  single-region with `extra_perimeters_on_overhangs`/`only_one_wall_top`
  off):
  - **inset A0 / A1** — the `for i in 0..wall_count` inset loop (wall_count=2);
    A1 (innermost) stays live as `current_polygons`;
  - **gap collection B / C / D** — at the final loop iteration only:
    `shrunk_prev = off(A0, -d/2)`, `grown_cur = off(A1, +d/2)`, difference;
  - **final infill-transition E / F / G / H** — off(A1): `-w`, `-w/2`,
    then `+w/2` of E's result, difference (all feeding the same `gaps` vec);
  - **gap width-band filter J** — `opening_ex`/`offset2_ex` of `gaps` are
    in-guest (slicer-core clipper2), then one host Difference clip;
  - **infill-boundary inset K** — off(A1) by `spacing(inner_w) - overlap`.
- **Clip census pins J, not the thin-wall clip, as the third per-layer
  clip**: 720 = 2×240 + 240 (benchy) and 1,484 = 2×495 + 494 (base) only
  close with {gap-collection D, final-transition H, gap-filter J}; the
  thin-wall protrusion clip (`detect_thin_wall` gate) never fires on the
  matched job — the take's probe re-verifies this dynamically.
- **Dependency graph / batchable groups** (for the batching direction):
  A0 → {A1, B} → {C, E, F, K} (all read A1) → {D (B,C), G (E)} → {H (F,G)},
  J (filtered gaps). Crossing count 11 → ~7 via `offset_polygons_batch` /
  `clip_polygons_batch`.
- **Exactness facts** (for candidate design):
  - the batch services execute the **same per-item code path** as single
    calls (`offset_polygons_batch`/`clip_polygons_batch`,
    `crates/slicer-wasm-host/src/host.rs`; `map_batch` preserves input order,
    fan-out keyed on estimated primitive work only);
  - in-guest twins `slicer_core::polygon_ops::offset`/`clip_polygons` are
    already linked into the guest wasm (the module uses in-guest `offset2_ex`
    and `opening_ex` today), so in-guest migration is a build-available
    option — its merit is a marshalling-vs-wasm-speed measurement, not a
    portability question;
  - the single-call host path = guest-side `to_wit_expolygon` marshalling +
    host-side `wit_to_ir`/`ir_to_wit` conversions + the same clipper2 call —
    the probe should split conversion vs clipper wall per site.
- **Candidate directions** (t60's wording):
  1. batching the independent groups above (fewer crossings; possible rayon
     fan-out when estimated work ≥ the 4,096-primitive threshold);
  2. in-guest migration for sites whose conversion share dominates
     (value-identity must be shown per site);
  3. any measured site-level reduction (e.g. a provably-discarded subset).
  The take measures before choosing; no candidate is pre-approved.
- **Take plan**: (A) T68 probe tagging the 11 per-layer calls by site +
  conversion/compute split at HEAD (base-off headline; benchy-off contrast;
  instrumented captures, never wall claims); (B) candidate implementation
  per A's split with per-site value-identity tests; (C) 16-cell
  output-identity qualification against the preserved references, then the
  standing paired ordinary + accelerated A/B on pre-declared cells
  (6 retained pairs per mode, zero starvation exclusions; keep/drop to the
  human, no auto-commit). Probe reverted before any resolution commit; both
  guest freshness checks green; evidence under `evidence/t68-host-roundtrips/`.
- **Recipe constraints**: instrumented runs never mixed into wall claims;
  `--module-dir modules/core-modules` mandatory; `cargo xtask build-guests
  --check` must exit 0 before any capture; outputs disclosed per the fairness
  contract.
