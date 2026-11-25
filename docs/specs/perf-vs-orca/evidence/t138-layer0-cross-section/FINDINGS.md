# t138: layer-0 cross-section parity — the slice plane, the anchor, and the support engine

**Status:** resolved 2026-11-10 — Phase-1 attribution. No production edit, no
timing campaign, no G-code reference move.

**Headline.** The layer-0 cross-section gap behind the t135 adhesion oracle's
2/8 is a **slice-plane convention difference**. PnP slices every layer at its
**top** (`z = first_layer_height + n × layer_height`); canonical OrcaSlicer
slices at the layer **midpoint** (`compute_slice_z` = `0.5 × (lo + hi)`). On
`base.stl`'s steep first-layer foot, 0.1 mm of Z is **3.42x** of cross-section
area; on the benchy's shallow hull it is **1.3%**. The wall-width difference
(0.4 vs 0.42 mm) and the brim anchor convention (wall path vs slice contour) are
second-order. The four supports-on cells are a **support-engine** difference
(Orca's first-layer support is **10.6–11.7x** denser), which no slicing change
fixes.

## 1. The measured convention

The same model mesh is the ground truth for both arms. `place_bare_mesh_on_bed`
(`crates/slicer-model-io/src/loader.rs`) drops the STL so `world_z = stl_z −
min_z`; slicing the STL at `min_z + z_top` and `min_z + z_mid`, then offsetting
inward by each arm's outer-wall half-width (PnP 0.20 mm, Orca 0.21 mm), gives a
hull area directly comparable to each arm's emitted first-layer outer wall.

| Fixture | Layer | Plane separation | PnP vs top | PnP vs mid | Orca vs top | Orca vs mid |
|---|---|---|---|---|---|---|
| base | 0 | **253.2%** | **1.36%** | 248.44% | 70.17% | **2.01%** |
| base | 10 | **161.3%** | **0.10%** | 161.05% | 63.16% | **4.02%** |
| base | 50 | 0.53% | 0.02% | 0.52% | 0.54% | 0.10% |
| base | 100 | 0.92% | 0.02% | 0.90% | 4.62% | 3.82% |
| base | 200 | 0.62% | 0.03% | 0.64% | 0.72% | 0.03% |
| benchy | 0 | 1.48% | 0.04% | 1.44% | 1.13% | 0.14% |
| benchy | 10 | 1.28% | 0.04% | 1.24% | 1.14% | 0.03% |
| benchy | 50 | 0.47% | 0.03% | 0.44% | 0.39% | 0.03% |
| benchy | 100 | 0.16% | 0.05% | 0.11% | 0.07% | 0.05% |
| benchy | 150 | 0.29% | 0.02% | 0.27% | 0.09% | 0.02% |

PnP fits the **top** plane on every one of the ten samples (0.02–1.36% error);
Orca fits the **mid** plane on every one (0.02–4.02%). The base L0/L10 rows are
the discriminators: the two candidate planes differ there by 1.6–2.5x, so a 1%
match is not a coincidence. The benchy rows are shallow, so their plane
separation is only 0.2–1.5%; they corroborate by the one-sided fit (PnP top
better, Orca mid better), not by magnitude.

The base layer count is the same fact from the other side: **PnP 495 layers,
last Z 99.0; Orca 496, last Z 99.2** (object height 99.1598 mm). Orca's last
layer midpoint is still below the object top, so it keeps a layer whose print
top is 99.2. The convention therefore lives in **two** PnP sites: the slice
producer (`batch_slice_objects_by_layer` / `execute_prepass_slice_single_layer_impl`)
and the planner's termination (`generate_object_layers_f64` / `merge_same_height`
in `modules/core-modules/layer-planner-default/src/lib.rs`, which stop at
`z > height` on the top). Changing only the slice producer moves the sample
plane but keeps 495 layers; the fix ticket carries both.

Canonical source, at the pinned `v2.5.0` = `40ecc66f78`:
`new_layers` / `compute_slice_z` (`PrintObjectSlice.cpp`) set `slice_z = 0.5 ×
(lo + hi)` for layer 0 and for every layer when ZAA is off
(`zaa_enabled = 0` in the matched Orca appendix); `generate_object_layers`
(`Slicing.cpp`) records the layer boundaries. PnP's
`batch_slice_objects_by_layer` (`crates/slicer-core/src/algos/prepass_slice.rs`)
feeds `layer.z` straight to `slice_mesh_ex`, and `merge_same_height`
(`modules/core-modules/layer-planner-default/src/lib.rs`) defines that `z` as
`first_layer_height + n × layer_height` — the layer top. The non-cached path
`execute_prepass_slice_single_layer_impl` does the same at `layer.z`.

## 2. What each measured difference is

| Difference | Class | Measured evidence |
|---|---|---|
| Base layer-0 object footprint: PnP 9.728 mm / 3.122 mm² vs Orca 5.566 mm / 0.914 mm² | **Slicing — slice plane** (dominant) | STL at top/0.20 = 3.165 mm², at mid/0.21 = 0.896 mm²; PnP 1.36% off top, Orca 2.01% off mid. Same-plane wall-width effect: **3.3%** |
| Benchy layer-0 footprint: +1.29% | **Slicing — slice plane** | STL top/mid differ 1.48%; observed PnP and Orca sit on the expected planes within 0.04/0.14% |
| Base supports-off brim path: +8.09% | **Slicing (plane) + anchor convention** | 22 loops: +88.04 mm contour term (top vs mid perimeter) − 27.65 mm anchor term = +60.39 mm predicted; **+57.39 mm observed** (5.0% residual) |
| Base supports-off skirt: +3.23% / +3.10% | **Slicing (plane), through the brim footprint** | Skirt input hull +8.68% / +8.48% (the brim dominates it); the outward skirt offset dilutes it to +3.2% |
| Base supports-on skirt: +12.48% | **Support engine** | Support path **11.68x** denser on Orca (9,329 vs 799 mm), coverage **11.46x**; support hull is 15.28% wider on PnP; the skirt input hull ratio 1.1528 maps to the skirt ratio 1.1248 |
| Benchy supports-on brim: +54.51% swept | **Support engine** (clipping input) | Orca support coverage **10.40x**; PnP's sparser support clips less brim away; brim run counts 19 vs 39 |
| Benchy supports-on skirt: +3.39% | **Support engine** | Skirt input hull −4.33% (support shape), skirt output −3.39%; the offset dilutes the input |
| Brim run counts 22/30 (benchy-off), 19/39 (benchy-on), 0/0 (base-on) | **Cosmetic feature-splitting** | Reported by the oracle, not gated; the base-on 0/0 agreement is the whole-ring clip matching |
| First-layer height | **No difference** | Both arms resolve 0.2 mm (`first_layer_height` / `layer_height`) |
| Wall width 0.4 vs 0.42 | **Slicing — wall-width resolution** | Base L0 same-plane area effect **3.3%**; benchy **0.18%**. It cannot carry a 3.42x gap |

The anchor convention is a representation gap, not a rule error: canonical
`make_brim` anchors on the object's slice contour; PnP's `SkirtBrim::generate`
(`modules/core-modules/skirt-brim/src/lib.rs`) reads `ExtrusionRole::OuterWall`
paths because the layer IR carries beads, not contours. The wall path sits half
an outer-wall width inside the contour (0.20 mm PnP, 0.21 mm Orca), so PnP's
brim ring is 0.20 mm wider inward. The 2πr anchor term above is an
approximation; the 3.0 mm (5.0%) residual is the non-circular correction.

## 3. The support engine is not a slicing difference

The supports-on cells are dominated by the support planner, not by the adhesion
module or the slice plane:

| Cell | Support path PnP / Orca | Support coverage PnP / Orca | Skirt-input hull PnP/Orca | Skirt output PnP/Orca |
|---|---|---|---|---|
| base-classic-on | 798.9 / 9,329 mm (**11.68x**) | 319.5 / 3,661.1 mm² (**11.46x**) | 1.1528 | 1.1248 |
| benchy-classic-on | 344.6 / 3,641.4 mm (**10.57x**) | 137.8 / 1,433.7 mm² (**10.40x**) | 0.9567 | 0.9661 |

On base, PnP's support extends wider (hull +15.28%) and is 11.7x sparser; its
skirt is 12.5% larger. On benchy, the support hull areas are nearly equal
(−0.72%) but the *shape* differs enough that the union hull is 4.3% smaller and
the skirt 3.4% smaller. In both cells the skirt output tracks the skirt input
hull, which is exactly what `_make_skirt`'s convex-hull construction predicts:
the rule is consistent, the input is the engine's. This is PnP's tree support
vs Orca's `tree_strong`, not a layer-0 sampling effect. Matching it would be a
support-engine program, not an adhesion-module or slicing fix.

## 4. Can the adhesion oracle ever reach 8/8 without a slicing change?

**No.** The current oracle is **2/8** (both benchy supports-off cells).

- **Base supports-off fails only the 3% skirt band** (+3.23% / +3.10%); the
  brim swept area (+8.09% / +7.30%) is inside the 12% band. The skirt input is
  the brim footprint (+8.7%), whose dominant term is the 3.42x layer-0 contour
  difference. The module cannot fix its own input.
- **The four supports-on cells** are outside the bands on the support engine
  (brim +54.5% on benchy; skirt +12.5% on base; skirt −3.4% on benchy). No
  slicing change moves the 10–12x support-density gap.

So: without any slicing change the ceiling is **6/8** (all but base supports-off,
and only with perfect support parity); with the slice-plane change alone the
expected ceiling is **4/8** (the supports-off cells); **8/8 needs both**. The
strict geometric 8/8 is an **engine-level** acceptance, not a module-level one.
The t135 decision to record 2/8 as "residual attributed outside the module"
stands, and this take now names the two residuals precisely: the slice plane
(a PnP slicing convention) and the support engine.

The anchor convention is a second-order term on base supports-off. It is
expected to sit inside both bands once the plane is fixed (the brim band is
12%; the anchor is −3.9% of the brim path), but whether the skirt lands inside
3% is a campaign question, because the 0.20 mm anchor shift also moves the brim
footprint. If it does not, the next question is the contour-input gap: feeding
the slice contour (or an outward half-wall reconstruction) into `skirt-brim`.
That is canonical-fidelity work, not a matched-settings asymmetry, so it is not
licensed by the t130 ruling and stays out of scope unless the human grants a
t81-style exception.

## 5. The price of the slicing change

The fix is `PrePass::Slice` sampling at `layer.z − 0.5 ×
active.effective_layer_height` (canonical midpoint), leaving `layer.z` as the
print top for every downstream consumer. It is a **fairness/correctness**
change, not a performance lever:

- It changes the **sample plane**, not the work set. The measured layer-0
  operand effect is base raw contour **5.329 → 2.314 mm² (2.30x)** and benchy
  **634.46 → 627.75 mm² (1.01x)** — one layer of 495 (base) / 240 (benchy),
  **0.20% / 0.42%** of the layers. No wall claim is taken, and the mechanism
  cannot contribute to the destination's **59.1–73.5%** required cut.
- The price is the acceptance ceremony: a fresh **16-cell reference set**, a
  delta audit, and the scoreboard re-derivation. Every G-code hash moves; base
  gains a layer (495 → 496); the support planner's input moves, so supports-on
  output moves beyond a sub-layer shift.
- Blast radius: the two slice sites are
  `batch_slice_objects_by_layer` and
  `execute_prepass_slice_single_layer_impl` (`crates/slicer-core/src/algos/prepass_slice.rs`).
  `layer.z` stays the print top for exact-Z validation, support-plan Z, and
  layer-range resolution; only the slice producer's plane moves. Per-object
  midpoint semantics for multi-height global layers need a design decision
  before the change is general.
- Licence: the t111/t128 **prepass implementation** exception covers
  `PrePass::Slice`, and the t130 close-asymmetries policy is the fairness
  rationale (the arms currently sample different planes before any new ratio of
  record). It is not a KEEP/DROP candidate and does not enter the route
  arithmetic.

Graduated as [Layer-plane parity: sample at the canonical midpoint](../../issues/139-layer-plane-parity.md).

## 6. Disclosures and limits

- The raw campaign tree (`target/matched-pair/t135-scoreboard3/`) is the t135
  Stage-1 source; the in-repo summary is derived from it by `reduce_t138.py`.
  Verification needs only the committed summary.
- The ground-truth section is the same STL both arms loaded, placed by
  `-min_z` per `docs/08_coordinate_system.md`; the hull metric is translation
  invariant (the arms place the model at different origins).
- The base L100 Orca row is 3.8% off the mid prediction and the base L10 row 4.0%
  — small-feature sensitivity of the hull, not a plane reversal (top would be
  4.6% / 63.2% off). The two discriminating rows carry the convention.
- The anchor decomposition is a 2πr approximation; the residual is 3.0 mm (5.0%).
- The support coverage is each arm's own bead-width buffer of the first-layer
  support paths; it is an engine comparison, not a printed-area claim.
- Orca's side is the installed 2.5.0 binary; the source-vs-binary caveat of t128
  is unchanged. One pass per cell; no new timing.
- The brim run counts quoted from the oracle are 22/30 and 19/39; this take's
  independent re-parse counts 18/39 on benchy-on because one run is split at a
  travel. Path lengths and areas are unaffected.

## 7. Evidence

- [`reduce_t138.py`](reduce_t138.py) — reads the raw t135 g-code and the two
  heavy fixtures, writes [`reduced/summary.json`](reduced/summary.json).
- [`verify.py`](verify.py) — 8 claim groups re-derived from the summary alone;
  `--self-test` rejects **12/12** negative controls.
- [`REPRODUCE.md`](REPRODUCE.md) — prerequisites and commands.
- `cargo xtask build-guests --check` and `cargo xtask build-guests --accelerated
  --check` both exit 0 on this tree (2026-11-10); `cargo xtask check-deviations`
  exits 0 and records the divergence as **DEV-201** in `docs/DEVIATION_LOG.md`.
