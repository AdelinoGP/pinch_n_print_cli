# t139: canonical mid-layer sampling — the plane, the slicer bridge, and the residuals

**Status:** resolved 2026-11-11 — fairness implementation + 16-cell re-take.
**Ticket:** [Layer-plane parity: sample at the canonical midpoint](../../../issues/139-layer-plane-parity.md).

**Headline.** `PrePass::Slice` now samples every layer at canonical
`compute_slice_z`'s band midpoint (`0.5 × (lo + hi)`), `layer.z` stays the print
top for every consumer, and the host-derived layer schedule plus the planner's
termination follow canonical `generate_object_layers`' midpoint rule. Base gains
its 496th layer (last Z 99.0 → 99.2); the benchy stays at 240 / 48.0. The
layer-0 outer wall moves onto the STL midpoint section on every cell — the base
fixture's steep foot goes from 1.4 % off the top plane to 2.4 % off the mid
plane, with the two candidate planes separated by **3.31x**. The matched job has
a **new reference set**, byte-identical on both build arms; the baseline arm
reproduces t135's set exactly.

Two production sites had to move with the plane, and one pre-existing slicer
defect had to be fixed for the change to be correct:

- the host schedule (`layer_top_zs`) and the planner (`generate_object_layers_f64`,
  `merge_same_height`, `merge_different_heights`) now keep a layer while its
  midpoint is below the object height;
- modifier volumes (parameter modifiers, support enforcer/blocker footprints,
  paint segmentation) are sampled on the same plane as the object cross-section
  at all four slice sites, so the Tier-2 `modifier_sub_region_id` hashes still
  match;
- `push_unique_intersection` (`crates/slicer-core/src/triangle_mesh_slicer.rs`)
  de-duplicates cut points by **topology only**, not by coincident coordinates.
  A grazing facet can cut the plane at two points that quantise to the same XY
  but carry different edge keys; merging them dropped the segment that bridged
  two loops, and the whole cross-section collapsed. At base's new 69.3 mm
  midpoint plane the old code produced **0.0025 mm²** (a 14-point sliver) where
  the correct section is **671.2054 mm²**; the emitter then skipped the layer
  entirely. With the fix the slice matches the trimesh section to the unit and
  every one of base's 496 layers is emitted.

## 1. Reference set (new currency)

16 cells = 8 matched cells × 2 build arms; ordinary and accelerated are
byte-identical. The baseline column is t135's Stage-1 set, reproduced exactly.

| Cell | Baseline (t135) | Candidate ordinary = accelerated | Bytes |
|---|---|---|---|
| benchy-classic-off | `7db0641b…` | `93e21fee…` | 4,084,733 |
| benchy-classic-on | `c02aa4e9…` | `b3f1eccc…` | 8,732,233 |
| benchy-arachne-off | `a2f7a1fe…` | `3510e876…` | 4,229,137 |
| benchy-arachne-on | `2326455d…` | `96cdac85…` | 8,873,041 |
| base-classic-off | `7ef84447…` | `ca14d055…` | 21,184,281 |
| base-classic-on | `1ddea2b7…` | `585c6a8e…` | 38,004,599 |
| base-arachne-off | `defae354…` | `d8fdc1d9…` | 21,045,270 |
| base-arachne-on | `4aa1847c…` | `c9fee59a…` | 37,866,953 |

Full hashes and the baseline/candidate/disk three-way check are in
[`reduced/delta.json`](reduced/delta.json); `verify.py` re-derives them without
the raw tree.

## 2. Layer grid and the layer-0 STL fit

| Fixture | Layers | Last Z | L0 outer-wall hull, old → new | STL top (0.20 in) | STL mid (0.20 in) |
|---|---|---|---|---|---|
| benchy | 240 → 240 | 48.0 → 48.0 | 519.821 → **513.195** | 520.028 | **513.415** |
| base | 495 → **496** | 99.0 → **99.2** | 3.122 → **0.932** | 3.165 | **0.955** |
| base (arachne) | 495 → **496** | 99.0 → **99.2** | 3.095 → **0.757** | 3.165 | **0.955** |

The old arm fits the top section (base 1.4 %, benchy 0.04 %); the new arm fits
the mid section (base classic 2.4 %, benchy 0.04 %). The base top/mid
separation is **3.31x**, which is the t138 discriminator; benchy's separation is
1.3 %. The base arachne generator's layer-0 hull is 20.7 % below the mid
section — its own first-layer wall convention, recorded here rather than
explained away (the classic/arachne hulls differ on the same plane: 0.932 vs
0.757).

## 3. Delta audit (feature / role / loop inventory)

The `;TYPE:` feature **set** is identical in all eight cells and no common layer
is missing. The number of layers on which a role appears moves with the sampled
cross-section — a sub-layer effect, not a rule change:

- benchy supports-off: Bottom surface 6 → 5, Top surface 31 → 36, Internal
  Bridge 12 → 11;
- base supports-off: Bottom surface 34 → 37, Top surface 64 → 66, Gap infill
  157 → 171, Bridge 53 → 44, Sparse infill 483 → 490, Internal Bridge 6 → 4,
  Outer wall 494 → 496;
- supports-on adds the support roles: base Support 431 → 400, Support interface
  70 → 59.

Per-cell section tables are in `reduced/delta.json`.

## 4. Adhesion oracle: still 2/8, and the base residual flipped sign

The t135 skirt/brim oracle is unchanged (same Orca reference). The supports-off
benchy cells pass. The base supports-off cells still fail the 3 % **skirt** band,
but now as an **undershoot**: swept area −4.3 % / −4.4 % (was +1.5 %) and outer
loop −5.7 % / −6.6 % (was +3.2 % / +3.1 %). The brim itself improved from +8.1 %
to −5.2 % swept (inside the 12 % band).

That is the t138 prediction inverted: removing the 0.10 mm top-plane offset from
the brim contour removed slightly more than the +8.1 % overshoot, leaving the
module's 0.20 mm inward anchor (it anchors `make_brim` on the emitted wall path
instead of the slice contour) as an undershoot. The ticket's fallback applies:
the contour-input question is graduated, not fixed here.

The four supports-on cells remain support-engine-limited (benchy brim swept
+68.9 %; base skirt growth +4.2 mm), exactly as t138 measured.

## 5. Old-vs-new paired A/B (attribution only)

3 interleaved pairs per cell, ordinary arms. Wall = process creation-to-exit,
CPU = kernel + user via the process handle. Candidate/baseline medians:

| Cell | Wall | CPU |
|---|---|---|
| benchy-classic-off | 0.930x | 0.991x |
| benchy-classic-on | 0.970x | 0.991x |
| benchy-arachne-off | 0.923x | 0.990x |
| benchy-arachne-on | 0.935x | 0.988x |
| base-classic-off | 0.984x | 0.990x |
| base-classic-on | 0.947x | 0.963x |
| base-arachne-off | 0.970x | 0.991x |
| base-arachne-on | 0.950x | 0.963x |

The supports-on gain is the support plan shrinking on the mid-plane input
(its filament total drops 18.5 %). No wall claim is taken: this is a fairness
fix, and the destination's required cut is unchanged.

## 6. Scoreboard re-derivation (8 cells × 3 tools, 3 runs)

| Cell | Orca (s) | PnP ordinary (s) | Ratio | PnP accelerated (s) | Ratio |
|---|---|---|---|---|---|
| benchy-classic-off | 1.87 | 4.42 | 2.370x | 4.51 | 2.417x |
| benchy-classic-on | 2.42 | 7.88 | 3.257x | 7.72 | 3.194x |
| benchy-arachne-off | 1.94 | 4.43 | 2.279x | 4.49 | 2.311x |
| benchy-arachne-on | 2.56 | 7.89 | 3.082x | 7.87 | 3.072x |
| base-classic-off | 12.47 | 36.14 | 2.899x | 35.34 | 2.834x |
| base-classic-on | 22.11 | 92.55 | 4.187x | 92.38 | 4.179x |
| base-arachne-off | 13.05 | 35.94 | 2.754x | 35.81 | 2.744x |
| base-arachne-on | 22.85 | 93.55 | 4.094x | 92.97 | 4.068x |

**Ratios of record: 2.279x–4.187x ordinary / 2.311x–4.179x accelerated;
required cut 56.1 %–76.1 %.** Both arms emit identical bytes in every cell.
This supersedes t135's 2.445x–3.769x, but **not by improvement of the code
alone**: the machine-state shift also moved both arms (Orca base supports-on
27.86 → 22.85 s, PnP 105.01 → 93.55 s between the two campaigns), so the ratio's
movement mixes the plane change with a faster box. The destination is unchanged
and 0 of 8 cells are won.

## 7. Residuals filed

- **DEV-202**: under canonical sampling the p241 `agg` arm's abrupt-drop count on
  `SupportAdversarial.stl` is legacy 3 / control 1 / agg 6 (was 3 / 0 / 0 at the
  top planes, which cut exactly through the slot roofs), and the calicat
  internal-bridge sites move one layer later (4.45/18.45/29.45 →
  4.65/18.65/29.65, same areas). `agg` is opt-in and the calicat residual is a
  site-index question, not new/lost geometry; both are pinned in tests.
- The base supports-off skirt residual (anchor on the wall path, not the slice
  contour) graduates as a contour-input decision.

## Reproduce

See [`REPRODUCE.md`](REPRODUCE.md); `delta_audit.py` and `reduce_t139.py` read
the gitignored raw tree, `reduce_scoreboard.py` reads the scoreboard rows, and
`verify.py` (7 claim groups, 22/22 negative controls) needs only `reduced/`.
