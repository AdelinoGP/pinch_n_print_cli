# Layer-0 cross-section parity: do the two arms print the same first layer?

Type: task / Phase-1 attribution
Label: wayfinder:task
Status: resolved (2026-11-10; Phase-1 attribution — the layer-0 residual is a
  **slice-plane convention difference**, the base skirt band is its second-order
  skirt consequence, and supports-on is the support engine)
Assignee: 2026-11-10 OpenCode wayfinder session — the human said "Continue the
  perf vs orca map" (no ticket named); 137 is the human's grilling and 136 and
  135 Stage 2 are gated by it, so 138 is the first unblocked, unclaimed AFK
  frontier ticket in order
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Staged fold: canonical skirt-brim re-take](135-cross-layer-occupancy-batching.md)'s
  2026-11-10 Stage-1 oracle residual.

## Question

The t135 Stage-1 parity oracle cannot reach 8/8 because the two arms' **layer-0
geometry differs upstream of `com.core.skirt-brim`**. On the matched job the
measured differences are:

- **Object footprint.** On `base.stl` the two slicers emit materially different
  first-layer outer-wall loops. Measured on the same cell: PnP outer-wall loop
  length 9.728 mm enclosing 3.122 mm² against Orca's 5.566 mm / 0.914 mm²; the
  brim's innermost loop encloses 6.18 mm² on PnP against 4.59 mm² on Orca. The
  brim anchors on the slice contour, which the g-code does not carry, so any
  g-code-level brim comparison inherits a ~0.2 mm anchor bias — measured as
  +8.1 % / +7.3 % brim area on the two base supports-off cells.
- **Support hull.** `_make_skirt` folds the support footprint into the skirt
  hull. PnP's tree supports are not Orca's `tree_strong`, so the skirt's outer
  loop differs by 12.5 % on both base supports-on cells even where both arms
  agree the brim is entirely clipped away (0 / 0 loops).
- **Feature composition.** Orca fragments the benchy supports-on brim into 39
  runs around its support pads against PnP's 19, so its `; FEATURE: Brim`
  section is 3× the area; the object brim cannot be separated from support-area
  brims at g-code level.

The question: **which of these are slicing differences (Z convention, contour
offset, first-layer height, wall-width resolution), which are support-engine
differences, and which are cosmetic feature-splitting** — and can the layer-0
cross-section be brought to parity (or shown equal modulo a named transform)?

Deliverable: a Phase-1 attribution that classifies each measured difference,
states whether the adhesion oracle can ever reach 8/8 without a slicing change,
and — if a slicing change is required — prices it against the destination.

## Answer

**The layer-0 cross-section gap is a slice-plane convention difference.** PnP
slices every layer at its **top** (`z = first_layer_height + n ×
layer_height`); canonical OrcaSlicer slices at the layer **midpoint**
(`compute_slice_z` = `0.5 × (lo + hi)` in `PrintObjectSlice.cpp`; ZAA is off in
the matched appendix). On `base.stl`'s steep first-layer foot, 0.1 mm of Z is
**3.42x** of cross-section area; on the benchy's shallow hull it is **1.3%**.
The same convention is visible in the layer count: base is **495 layers / last
Z 99.0** on PnP against Orca's **496 / 99.2**. That is a second PnP site — the
planner's `generate_object_layers_f64` / `merge_same_height` also stop on the
top — so the fix ticket carries both the sample plane and the termination.

**The discriminator.** Slicing the same STL at each layer's top and midpoint and
offsetting inward by each arm's outer-wall half-width (PnP 0.20 mm, Orca 0.21 mm)
gives the expected first-layer outer-wall hull. On the ten sampled layers PnP
fits the **top** plane (0.02–1.36% error) and Orca fits the **mid** plane
(0.02–4.02%) on every one. The decisive rows are base L0 and L10, where the two
candidate planes differ by **253% / 161%**: PnP is 1.36% / 0.10% off the top
plane and 248% / 161% off the mid; Orca is 2.01% / 4.02% off the mid and
70% / 63% off the top. The benchy rows are shallow (plane separation 0.2–1.5%)
and corroborate by the one-sided fit, not by magnitude.

**Classification of the measured differences:**

| Difference | Class | Evidence |
|---|---|---|
| Base L0 footprint 3.122 vs 0.914 mm² | **slicing — slice plane** (dominant) | STL top/0.20 = 3.165, mid/0.21 = 0.896; same-plane wall-width effect only 3.3% |
| Benchy L0 footprint +1.29% | **slicing — slice plane** | STL top/mid differ 1.48%; both arms sit on their expected planes within 0.04/0.14% |
| Base supports-off brim +8.09% | **slicing (plane) + anchor convention** | 22 loops: +88.04 mm contour term − 27.65 mm anchor term = +60.39 mm predicted vs **+57.39 mm observed** (5.0% residual) |
| Base supports-off skirt +3.23% / +3.10% | **slicing (plane), through the brim footprint** | Skirt input hull +8.68%; the outward skirt offset dilutes it to +3.2% |
| Base supports-on skirt +12.48% | **support engine** | Orca support path **11.68x** denser / coverage **11.46x**; support hull 15.28% wider on PnP; skirt input 1.1528 → skirt 1.1248 |
| Benchy supports-on brim +54.51% swept | **support engine** (clipping input) | Orca support coverage **10.40x**; PnP's sparser support clips less brim; runs 19 vs 39 |
| Benchy supports-on skirt +3.39% | **support engine** | Skirt input hull −4.33%, skirt output −3.39% |
| Runs 22/30, 19/39, 0/0 | **cosmetic feature-splitting** | Reported, not gated; base-on 0/0 is the whole-ring clip agreeing |
| First-layer height | **none** | Both 0.2 mm |
| Wall width 0.4 vs 0.42 | **slicing — wall-width resolution** | 3.3% / 0.18% same-plane effect; cannot carry a 3.42x gap |

The anchor is a representation gap: canonical `make_brim` anchors on the slice
contour; `SkirtBrim::generate` (`modules/core-modules/skirt-brim/src/lib.rs`)
reads the outer-wall path because the layer IR carries beads, not contours. The
path is 0.20 mm inside the contour, so PnP's ring is 0.20 mm wider inward.

**Can the oracle reach 8/8 without a slicing change? No.** It is **2/8** now.
Base supports-off fails **only the 3% skirt band** (+3.23% / +3.10%; the brim is
inside its 12% band at +8.09% / +7.30%), and the skirt input is the brim
footprint, whose dominant term is the 3.42x contour — the module cannot fix its
own input. The four supports-on cells are outside the bands on the support
engine, which no slicing change moves. Without any slicing change the ceiling is
**6/8** (all but base supports-off, and only with perfect support parity); with
the slice-plane change alone the expected ceiling is **4/8** (the supports-off
cells); **8/8 needs both**. The strict geometric 8/8 is an **engine-level**
acceptance, not a module gate. The t135 "2/8 with every residual attributed
outside the module" decision stands; this take names the two residuals.

The 0.20 mm anchor is expected to sit inside both bands once the plane is fixed
(−3.9% of the brim path), but whether the base skirt lands inside 3% is a
campaign question. If it does not, the next question is the contour-input gap
(feed the slice contour, or reconstruct it by an outward half-wall offset, into
`skirt-brim`). That is canonical-fidelity work, not a matched-settings
asymmetry; it is not licensed by the t130 ruling and stays out of scope unless
the human grants a t81-style exception.

**Price of the slicing change.** It is a fairness/correctness change, not a
performance lever: it moves the sample plane, not the work set. The measured
layer-0 operand effect is base raw contour **5.329 → 2.314 mm² (2.30x)** and
benchy **634.46 → 627.75 mm² (1.01x)** — one layer of 495 / 240 (**0.20% /
0.42%** of the layers). No wall claim is taken; the mechanism cannot contribute
to the destination's **59.1–73.5%** required cut. The price is the acceptance
ceremony: a fresh **16-cell reference set**, delta audit, and scoreboard
re-derivation. Every G-code hash moves; base gains a layer (495 → 496); the
support planner's input moves, so supports-on output moves beyond a sub-layer
shift. The two slice sites are `batch_slice_objects_by_layer` and
`execute_prepass_slice_single_layer_impl`
(`crates/slicer-core/src/algos/prepass_slice.rs`); `layer.z` stays the print top
for exact-Z, support-plan Z and layer-range consumers. Per-object midpoint
semantics for multi-height global layers need a design decision. Licence: the
t111/t128 prepass implementation exception (it is a `PrePass::Slice` change)
plus the t130 close-asymmetries fairness rationale; it is not a KEEP/DROP
candidate and does not enter the route arithmetic.

Graduated: [Layer-plane parity: sample at the canonical midpoint](139-layer-plane-parity.md)
(AFK, unblocked, takeable now).

Evidence: [`evidence/t138-layer0-cross-section/`](../evidence/t138-layer0-cross-section/FINDINGS.md)
(`reduce_t138.py` → `reduced/summary.json`; `verify.py` 8 claim groups,
`--self-test` rejects 12/12 negative controls). No production edit, no timing
campaign, no G-code reference move. Filed **DEV-201** (`docs/DEVIATION_LOG.md`)
for the knowingly-kept slice-plane divergence until the fix ticket lands.
