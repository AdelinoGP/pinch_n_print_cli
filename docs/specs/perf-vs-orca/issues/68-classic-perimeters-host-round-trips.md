# Classic-perimeters per-layer host-service round trips

Type: task
Status: resolved (2026-10-16; attribution take — the round-trip bucket is 97–99 % native clipper ops, every named reduction direction's measured ceiling is below the bar; no candidate, no production edit, no timing campaign; probe reverted; see ## Resolution)
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

## Resolution

**Verdict (attribution take, 2026-10-16): no candidate — the per-layer
host-service round-trip bucket is 97.55 % (benchy-off) / 99.02 % (base-off)
native clipper operations, and every reduction direction t60 named for this
lead has a measured ceiling below any gate this map has applied.** No
production edit, no timing campaign, no auto-commit; the probe is reverted
and both freshness checks are green.

- **Method.** The T68-PROBE (two-sided; preserved as `probe.patch`, applied,
  measured, reverted) records, per host-service call attributed to the
  classic-perimeters dispatch: service, arg, input/output polygon and vertex
  counts, and the conversion/ops/conversion wall split; guest-side `now_us`
  brackets around all 11 call sites measure the full guest-observed round
  trip per site and layer. Two ordinary captures: benchy-off (17.575 s
  instrumented) and base-off (263.530 s instrumented, the largest owner
  cell). **Both outputs byte-identical to the preserved t58 references**
  (`34886ccb…` / `20e7204c…`), clean status, zero non-fatals.
- **The split.** Base-off (87,900.3 ms guest-observed, 5,442 calls): ops
  87,142.1 ms (99.02 %), host conversions 100.5 ms (0.11 %), guest
  marshalling + ABI remainder 758.2 ms (0.86 %). Benchy-off (5,726.4 ms,
  2,640 calls): ops 5,598.9 ms (97.55 %), conversions 13.4 ms, remainder
  127.5 ms. Site shares (base): A0 21.4 %, A1 22.3 %, K 9.6 % — the walls
  and infill boundary themselves — and the gap machinery B/C/D/E/F/G/H/J
  46.8 % (49.5 % benchy), which feeds the width-band morph that discards
  ~98.6 % of the raw candidate vertex volume per layer before the ~105-vert
  filter input and the 162/495-layer output.
- **Ceilings (the no-candidate reasoning).** In-guest migration can remove at
  most the remainder + conversions (0.98 % base / 2.45 % benchy) while
  replacing native clipper with the wasm twin on 97–99 % of the cost — net
  negative. Batching can remove only per-crossing fixed costs (per-item
  marshalling unchanged; the 12-thread-saturated phase makes fan-out
  zero-sum; benchy sits at/below the 4,096-primitive threshold). Site-level
  reduction: every site feeds exactly one consumer or the output (D = f(B,C),
  H = f(F,G), G = f(E); A0/A1 walls, K infill) — no dead or duplicated site;
  no representation-identical algebraic shortcut exists for the miter-offset
  composition, and byte-exactness forbids representation changes. The
  entire removable overhead is ≤ 0.33 % (serial) / 0.03 % (saturated) of base
  run wall and ≤ 0.80 % / 0.07 % on benchy — below the smallest wall effect
  this map has ever kept.
- **Dynamics confirmed.** Thin-wall protrusion clip: 0 calls across all 240
  benchy + 495 base layers (the static census inference holds; its gate
  resolves false on the matched job — recorded as a config-resolution
  observation, the classic manifest default `true` vs arachne's `false`
  reconciled per key, not chased here). Base layer 270 skips the
  gap-collection trio (empty inner inset).
  Wall relevance (t64-style): bucket = 6.3 % base / 6.6 % benchy of per-layer
  phase thread-time; last layer carries 2.3 ms / 4.2 ms.
- **Stability + cross-checks.** Two independent base runs: 87.90 s vs
  78.46 s guest-observed (±11 % accumulated drift, the §3.6 class) with
  decomposition shares stable within ~1 point; vs t60's committed owner wall:
  −1.5 % base / −3.1 % benchy, same exact call counts. `verify.py` exits 0
  against the in-repo set alone, enforcing output identity, the exact call
  census, guest ≥ host per site, the ops-share/overhead/wall-bound margins,
  the bucket containment, the ±25 % t60 band, and rejecting seven negative
  controls.
- **Evidence:** `evidence/t68-host-roundtrips/` (`FINDINGS.md`, `probe.patch`,
  `run-t68.ps1`, `captures/`, `summary.json`, `reduce.py`, `verify.py`).
  The `t68 probe (mirror of evidence/t68-host-roundtrips/probe.patch)` stash
  entry is retained, never dropped.
- **Frontier.** [20](20-consume-only-when-read-annotations.md) remains the
  only open child (parked below-fold); the `region_partition` dedup lead
  remains un-graduated — route decision back to the human.

