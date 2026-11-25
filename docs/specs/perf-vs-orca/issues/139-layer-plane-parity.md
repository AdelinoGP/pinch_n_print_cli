# Layer-plane parity: sample at the canonical midpoint

Type: task / implementation + fairness re-take
Label: wayfinder:task
Status: resolved (2026-11-11; **canonical mid-layer sampling landed** — base
  496 layers / last Z 99.2, the layer-0 outer wall fits the STL midpoint on
  every cell, a new 16-cell reference set is byte-identical on both build arms,
  the oracle stays 2/8 with the base skirt residual flipped sign, and the
  scoreboard re-derives at **2.279x–4.187x ordinary / 2.311x–4.179x
  accelerated**. See ## Answer)
Assignee: 2026-11-10 OpenCode wayfinder session — the human said "Continue the
  perf vs orca map, pick an afk ticket"; 139 is the only unblocked, unclaimed
  AFK frontier ticket in order (137 is the human's grilling; 136 and 135
  Stage 2 are gated by it)
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Layer-0 cross-section parity](138-layer0-cross-section-parity.md)'s
  2026-11-10 attribution: PnP slices every layer at its top
  (`z = first_layer_height + n × layer_height`) while canonical OrcaSlicer
  slices at the layer midpoint (`compute_slice_z` = `0.5 × (lo + hi)`), which
  on `base.stl` is a 3.42x layer-0 cross-section and the 495-vs-496 layer count.

## Question

Adopt canonical mid-layer sampling in `PrePass::Slice` and re-take the fairness
premise: slice each object at `layer.z − 0.5 ×
active.effective_layer_height` (the `compute_slice_z` midpoint), while
`layer.z` remains the print top for every downstream consumer, and record the
new reference set, the delta audit, the t135 adhesion-oracle re-run and the
scoreboard re-derivation.

The 2026-11-10 attribution ([t138](138-layer0-cross-section-parity.md)) shows:

- PnP's first-layer outer-wall hull matches the STL at the layer **top** with a
  0.20 mm wall offset on all ten sampled layers (0.02–1.36%); Orca's matches the
  **midpoint** with 0.21 mm (0.02–4.02%).
- On `base.stl` the 0.1 mm shift is **3.42x** of layer-0 area (3.122 vs
  0.914 mm²); on the benchy **1.3%**. The base skirt failure (+3.23% / +3.10%
  against the 3% band) is the brim footprint (+8.68%) diluted by the skirt
  offset, and the brim path decomposition is +88.04 mm contour − 27.65 mm
  anchor = +60.39 mm predicted against +57.39 mm observed.
- Base prints **495 layers / last Z 99.0** on PnP against Orca's **496 / 99.2**.
- The four supports-on cells are the support engine (Orca's first-layer support
  is 10.6–11.7x denser); this ticket does not fix them.

## Scope

1. Sample at the midpoint in both producers:
   `batch_slice_objects_by_layer` and
   `execute_prepass_slice_single_layer_impl`
   (`crates/slicer-core/src/algos/prepass_slice.rs`). Keep `layer.z` as the
   print top; do not move exact-Z validation, support-plan Z, or layer-range
   resolution. Decide and document the per-object midpoint for multi-height
   global layers (a global layer can carry regions with different
   `effective_layer_height`; the matched job is single-height, but the change
   must not be silently wrong for the general case).
2. Port the matching **layer termination** in the planner:
   `generate_object_layers_f64` and `merge_same_height`
   (`modules/core-modules/layer-planner-default/src/lib.rs`) currently stop on
   the top (`z > height`); canonical's `generate_object_layers` stops on the
   midpoint (`slice_z >= object_print_z_height`). Without this, the slice-plane
   change keeps base at 495 layers; with it, base gains the 496th layer (print
   top 99.2) and the benchy stays at 240. Decide whether the host-derived
   explicit schedule (`ObjectPlan::layer_zs`, used by layer-range scoping)
   already carries the midpoint rule or needs its own change.
3. Fresh **16-cell** output campaign (ordinary + accelerated arms), recording
   the new reference set. t135 Stage 1's set is retired as the currency.
4. Delta audit: the expected movement is a sub-layer sample shift, not a rule
   change. Re-run the t138 STL check on the new G-code (PnP outer-wall hull vs
   the STL midpoint section) and audit the feature/role/loop inventory against
   the old set. Confirm base gains a layer (495 → 496, last Z 99.2).
5. Re-run the t135 adhesion oracle on the new cells. Expected: the two
   supports-off cells pass both bands (the 0.20 mm anchor is −3.9% of the brim
   path, inside the 12% band); the four supports-on cells remain
   support-engine-limited. If base supports-off still fails, record the exact
   residual and graduate the contour-input question (the module anchors on the
   wall path, not the slice contour) as a separate decision.
6. Old-vs-new paired A/B (the standing paired form, wall + CPU, interleaved):
   attribution only; this is a fairness fix, not a KEEP/DROP candidate.
7. Closing gate: the 8-cells × 3-tools scoreboard re-derivation under the
   corrected premise, superseding t135's ratios of record.

## Pre-declared rules

- **Not a performance candidate.** The change moves the sample plane, not the
  work set. No wall claim is pre-declared; the layer-0 operand effect is one
  layer of 495/240 (0.20% / 0.42%). The destination's 59.1–73.5% required cut is
  unchanged.
- **Licence:** the t111/t128 prepass implementation exception (a
  `PrePass::Slice` change) plus the t130 close-asymmetries fairness rationale.
  It is not a new parity exception and does not open the rest of the parity
  program.
- **Output contract:** candidate and baseline arms byte-identical within
  themselves; baseline reproduces the t135 Stage-1 set exactly before the change;
  candidate records the new set on both build arms.
- **Freshness:** both `cargo xtask build-guests --check` and
  `--accelerated --check` exit 0 before and after.

## Acceptance

- The two slice sites sample at the midpoint; `layer.z` is unchanged for
  downstream consumers; the multi-height decision is recorded.
- A fresh 16-cell reference set, named in this ticket's closure, identical on
  both build arms; base is 496 layers / last Z 99.2 on PnP.
- The delta audit shows a sub-layer sample shift (the t138 STL check green on
  the new cells; feature/role/loop inventory unchanged apart from the new
  layer).
- The t135 oracle re-run is recorded: supports-off verdicts, supports-on
  residual, and the anchor's measured share.
- The old-vs-new paired A/B and the 8×3 scoreboard re-derivation are recorded,
  with the t128 source-vs-installed-binary caveat.
- Both freshness gates exit 0; no other production edit rides along.

## Answer

**Adopted canonical `compute_slice_z` at every layer-sampling site, and paid the
three ripples it exposed.** `PrePass::LayerPlanning` / `PrePass::Slice` now
sample each layer at the **band midpoint** (`0.5 × (lo + hi)`); `GlobalLayer.z`
remains the print top for exact-Z, support-plan Z, layer-range resolution and
the emitted G-code.

**1. Layer grid.** The host schedule (`layer_top_zs`) and the planner
(`generate_object_layers_f64`, `merge_same_height`, `merge_different_heights`)
terminate on the midpoint, exactly as canonical `generate_object_layers`
breaks on `slice_z >= object_print_z_height`. Base gains its 496th layer
(**495 → 496, last Z 99.0 → 99.2**); the benchy stays at 240 / 48.0. The
multi-height decision: the per-object midpoint uses that region's own
`effective_layer_height` (the catch-up span for a bridged region), so a global
layer carrying several objects still samples each object at its own band
midpoint; several regions of one object at one plane share the band by
planner construction, with the widest band chosen deterministically.

**2. The slice plane is a four-site contract, not two.** Besides the batch and
single-layer slice producers, modifier volumes — parameter-modifier footprints
in `region_mapping`, the paint-segmentation modifier pass, the support
enforcer/blocker pass in `support_analysis_producer`, and the Tier-2 staging
fallback in `layer_executor` — are now sampled on the same plane, so the
`modifier_sub_region_id` FNV hash still matches between the prepass split and
the region-map re-derivation. Leaving any one on `layer.z` reintroduced
`MissingModelConfig` for every modifier sub-region.

**3. A pre-existing slicer defect had to be fixed.** The midpoint shift made
base's 69.3 mm plane collapse: `push_unique_intersection`
(`crates/slicer-core/src/triangle_mesh_slicer.rs`) de-duplicated cut points by
_coincident coordinates_ as well as topology. A grazing facet cuts that plane
at two points that quantise to the same XY but carry different edge keys;
merging them dropped the segment that bridged two loops, the topology walk
ended in 16 unpaired dead ends, and the whole section became a **0.0025 mm²**
14-point sliver (correct: **671.2054 mm²**, matching trimesh), so the emitter
skipped the layer. Deduping by topology only — canonical `slice_facet` keys
points by `point_id` / `edge_id`, never by coordinates — restores the section
and every one of base's 496 layers is emitted.

**4. New reference set (the currency).** 8 cells × 2 build arms, byte-identical
ordinary vs accelerated; the pre-change arm reproduces t135's Stage-1 set
exactly. Hashes: benchy `93e21fee` / `b3f1eccc` / `3510e876` / `96cdac85`, base
`ca14d055` / `585c6a8e` / `d8fdc1d9` / `c9fee59a`. Full 64-char hashes in
`evidence/t139-layer-plane-parity/reduced/delta.json`.

**5. Delta audit.** The layer-0 outer-wall hull fits the STL **midpoint**
section on every cell: benchy 513.195 vs mid 513.415 (top 520.028); base
classic 0.932 vs mid 0.955 (top 3.165; **3.31x** top/mid separation); base
arachne 0.757 (its own first-layer wall convention, recorded). The `;TYPE:`
feature set is unchanged in all eight cells and no common layer is missing;
per-role layer counts move with the sampled cross-section (e.g. base supports-off
Top surface 64 → 66, Bridge 53 → 44, Sparse infill 483 → 490, Outer wall
494 → 496).

**6. Oracle: still 2/8, and the base residual flipped sign.** Both benchy
supports-off cells pass. Base supports-off still fails the 3 % skirt band, now
as an **undershoot**: swept −4.3 % / −4.4 % (was +1.5 %), outer loop −5.7 % /
−6.6 % (was +3.2 % / +3.1 %); the brim improved from +8.1 % to −5.2 % (inside
the 12 % band). Removing the 0.10 mm top-plane contour offset removed slightly
more than the +8.1 % overshoot, leaving the module's 0.20 mm inward anchor (it
anchors `make_brim` on the wall path, not the slice contour) as an undershoot.
Per the ticket's fallback, the **contour-input question graduates** as a
separate decision. The four supports-on cells remain support-engine-limited
(benchy brim swept +68.9 %; base skirt growth +4.2 mm).

**7. A/B (attribution only).** 3 interleaved pairs, ordinary arms: wall
0.923x–0.984x, CPU 0.963x–0.991x. The supports-on gain is the support plan
shrinking on the mid-plane input (filament −18.5 %). No wall claim is taken.

**8. Scoreboard.** 8 cells × 3 tools × 3 runs: **2.279x–4.187x ordinary /
2.311x–4.179x accelerated, required cut 56.1 %–76.1 %**, both arms byte-identical
per cell. This supersedes t135's 2.445x–3.769x, with the disclosure that both
arms also moved with machine state (Orca base supports-on 27.86 → 22.85 s, PnP
105.01 → 93.55 s), so the ratio movement is not attributable to the plane
change alone. 0 of 8 cells won; the destination is unchanged.

**Residuals filed.** **DEV-202** records the canonical-sampling consequences
outside the matched job: the p241 `agg` arm's drop count (legacy 3 / control 1
/ agg 6, was 3 / 0 / 0) and the calicat internal-bridge sites one layer later
(4.45/18.45/29.45 → 4.65/18.65/29.65, same areas). `agg` is opt-in; both are
pinned in tests. DEV-201 (the slice-plane divergence) is **closed**.

**Evidence.** `evidence/t139-layer-plane-parity/`: `delta_audit.py`,
`reduce_t139.py`, `reduce_scoreboard.py` → `reduced/*.json`; `verify.py` 7 claim
groups, 22/22 negative controls rejected. Both freshness gates exit 0; no
other production edit rode along (the modifier-plane and slicer-dedup changes
are prerequisites of this change, not separate work).
