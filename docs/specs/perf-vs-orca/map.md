# Wayfinder map: Slice performance vs OrcaSlicer

Label: wayfinder:map
Effort: `docs/specs/perf-vs-orca/` (local-markdown tracker; wayfinder maps live
under `docs/specs/` in this repo, per `docs/specs/orca-feature-gap`)
Charted 2026-09-22 from `docs/specs/perf-vs-orca/evidence/PERF-HANDOFF.md` and the mode-comparative addendum
of the 2026-09-22 session (grilling decisions Q1–Q8).

## Destination

PNP beats OrcaSlicer on matched settings: **lower median uninstrumented
wall-clock** (`< 1.0x`) on the **binding arm** (`pnp-ordinary`, the current
production default; `pnp-accelerated` measured and disclosed beside it) in
every cell of the `{benchy, base} × {classic, arachne} × {supports off, on}`
matrix, under the matched-pair protocol with outputs disclosed. **Amended
2026-11-21 ([t176](issues/176-destination-cpu-corroboration-ruling.md), Option
C):** cross-engine process CPU serves as a **CPU integrity guard** (tight
process CPU, unstarved `cpu/wall`, matched non-degraded output) paired with a
separate **PnP/Orca CPU efficiency disclosure** beside every cell (within-PnP
**candidate A/B CPU corroboration** rules remain unchanged). **Cell closure:**
a wide-margin cell (`ratio <= 0.85x`) closes **provisionally** on a quiet `n=3`
strict-separation board with zero starvation flags; a close cell
(`0.85x < ratio < 1.00x`) requires `>= 4` retained unstarved runs per arm with
strict separation (starved outliers `< 0.75 × group best` excludable per
`SCOREBOARD.md` while `>= 4` unstarved runs remain); provisional wins carry
forward across later commits that pass their own paired non-regression gate
(re-opened only by a later Orca board `>= 1.0x` or an unwaived regression).
**8 of 8 cells won on one final single-HEAD `n >= 4` closing board closes the
map** (a follow-on `<= 0.50x` / 2× faster effort is planned as a fresh map
afterward, while high-leverage speedup tracks that push cells toward `0.50x`
are prioritized now).

**Active reference set (2026-11-22, [t182](issues/182-host-spatial-hierarchical-phase3-union.md)): t182's set is the currency — benchy `4c67ef0b…` / `a8c93535…` / `fd631384…` / `7a8d87ae…`, base `02ffa2d6…` / `7e115231…` / `e806296f…` / `da1e41d4…` (classic-off / classic-on / arachne-off / arachne-on; all four supports-off cells unchanged from t175/t172; only the four supports-on cells superseded from t175 by explicit human KEEP ruling for sub-micron Clipper2 scanbeam/associativity rounding strictly inside `Support` with zero `Support interface` movement and `0/1,226,364` non-support part XY diffs), byte-identical on the ordinary and accelerated arms (16/16); the baseline arm of the same campaign reproduced t175's set 16/16 (`4c67ef0b…` / `fdca4e2d…` / `fd631384…` / `2869186c…`, `02ffa2d6…` / `7b90e749…` / `e806296f…` / `f77633af…`), and t175/t172/t169 remain superseded. Every hash below this sentence that predates it is historical for comparison purposes and must be re-recorded before use.**

**Benchy reference currency (2026-11-13, [t145](issues/145-benchy-fixture-rebaseline.md)): the benchy fixture was replaced on 2026-11-12T20:48, and every benchy number measured before that boundary is HISTORICAL.** The replacement (`tmp/3dbenchy.stl`, `6ab57f1c…`) is a re-tessellation of the same model — 225,786 → 225,706 triangles, |Δbounds| ≤ 4.05e-7 mm, ten of ten sampled cross-sections equal to the old export's recorded values — but the slice is **not** output-neutral: one internal-bridge layer (L40, z = 8.2, +57.298 mm of path) and the tree-support engine (−1,370.181 mm over 176 layers) move, so all four benchy hashes move. **Superseded active set = t168's** ([`evidence/t168-bridge-opening/reduced/output.json`](evidence/t168-bridge-opening/reduced/output.json), 2026-11-19: benchy `d6cb4acd…` / `21c6969f…` / `5c0f846e…` / `6fe4b09b…`, base `9e360450…` / `f77cf265…` / `cd896bab…` / `3044c4d6…`, byte-identical on the ordinary and accelerated arms). t166's set is superseded but was reproduced **8/8** by the t168 baseline arm in the same campaign (benchy `276782a7…` / `44564d29…` / `46111bb0…` / `12c5e45c…`, base `0a1ee3df…` / `bc5cb5e1…` / `cadebffa…` / `77f2b655…`); t151's set is superseded (reproduced **8/8** by both the t166 and t168 baseline arms); for the history, t151's (2026-11-14) was benchy `3d28d1b3…` / `eb202973…` / `39ad6289…` / `77c3c578…`, base `9bb18f0e…` / `eccbd523…` / `2f249cdb…` / `1811425f…` (each list is classic-off / classic-on / arachne-off / arachne-on; the two base supports-on hashes are unchanged from t148 by measurement because those cells are inert). **t148's set is superseded but was reproduced 8/8 by the t151 baseline arm in the same campaign**: benchy `1ae0cfa8…` / `37788518…` / `8faf02b1…` / `f86d8897…`, base `c0b9014f…` / `eccbd523…` / `17f63e93…` / `1811425f…`. t145's four supports-on hashes (`d9fc80ba…` / `cd7d4a2d…` / `585c6a8e…` / `c9fee59a…`) are **superseded** because that job printed less support than it demanded; **t139's four benchy hashes are retired** (`93e21fee…` / `b3f1eccc…` / `3510e876…` / `96cdac85…`); t143's four are the superseded interim set. Rule: **a benchy number is either pre-swap (historical) or post-swap (current); never mixed, never quoted across the boundary.** Retired by that rule: t139's benchy hashes and every comparison built on them; t138's benchy **g-code** figures (its benchy **STL** figures survive — the cross-sections reproduce exactly); t135 Stage 1's benchy oracle numbers; t142's benchy A/B (20.00 / 10.56 / 20.23 / 12.66 %) and every pre-swap benchy A/B pair; every benchy scoreboard median (t121 / t124 / t139) and any composite containing a benchy cell; t126's "benchy classic-off 1.02x at zero prepass" and t134's benchy planner ratios. Post-swap and current: t143's benchy rows (fixture mtime 20:48, depth runs 20:55+), t144's hashes / oracle / A-B, t145's re-baseline, and t148's supports-on re-record. **Post-t148 scoreboard (4 supports-on cells × 2 tools, one window): 2.705x / 2.829x / 3.356x / 3.248x ordinary/Orca** (benchy classic-on / benchy arachne-on / base classic-on / base arachne-on), CPU corroborating — **required cut 63.0 % / 64.6 % / 70.2 % / 69.2 %**; t145's 8-cell post-swap scoreboard **1.853x–3.455x ordinary/Orca** stands for the four supports-off cells and is historical for the four supports-on cells (that job was support-incomplete). The accelerated arm has no current scoreboard.

**Destination status (2026-11-08, [t128](issues/128-destination-arithmetic-ruling.md)):
the target is unchanged; the route is re-scoped to an engine rewrite.** 0 of 8
cells won. PnP is **2.36x–4.65x** slower on the ordinary arm (**2.40x–4.67x**
accelerated) and needs a **58%–79%** wall cut to win. The post-t123 split says
the only term that can close that cut is the prepass (**62.1–87.1 %** of every
cell; required removal **72–101 %**), and that **benchy classic-off cannot be
won by prepass deletion alone** (**1.02x** at zero prepass, because its
per-layer + postpass + outside phases already exceed Orca's whole wall). The
remaining prepass stages are clipper-bound and every named shape has been
screened, so the ruling **keeps 8/8 and authorises a rewrite of the engine's
geometric work** — the prepass first (the supports-off cluster, then the
planner), then the benchy-classic-off per-layer residual — under t111's prepass
exception carried forward to the prepass implementation. **The route's first
gate was the work-set / per-op parity decomposition against canonical
OrcaSlicer `v2.5.0` (`40ecc66f78`, 2026-11-07):** [Prepass work-set and
per-op parity against canonical v2.5.0](issues/129-prepass-work-set-per-op-parity.md),
**resolved 2026-11-08: the work set is not the lever; per-op cost and operand
locality are.** Canonical does more per-layer Clipper work than PnP in the
shell cluster, yet its whole prepass is 15.7x / 9.8x cheaper on base classic-off
/ -on; the Rust Clipper2 kernel is at parity (t70), so the gap is operand size
and host plumbing. The clean PnP-local deletions are 55.8 % of base-off's required cut
and 17.5 % of base-on's; a work-set-only rewrite cannot reach 8/8. **Prepass at
canonical cost wins base classic-off only (0.787x), ties base classic-on
(1.012x) and loses both benchy cells (1.053x / 1.075x); full geometry parity wins
all four (0.335x / 0.661x / 0.550x / 0.704x).** The route is a full engine-parity
program, and the benchy per-layer phase is part of it. The next gate was
[Common-stage per-op cost decomposition](issues/131-common-stage-per-op-cost-decomposition.md),
**resolved 2026-11-09 (see below)**.
The matched-settings claim's brim asymmetry is graduated as
[Matched settings vs the 8 mm brim](issues/130-brim-matched-settings-ruling.md).

**Destination status (2026-11-09,
[t131](issues/131-common-stage-per-op-cost-decomposition.md)): the per-op
wrapper is dead as a lever; the route is operand volume plus planner
scheduling.** The Clipper2 sweep is **98.2–99.9 %** of every common prepass
stage's measured primitive thread-time and the whole non-sweep wrapper
(conversion, WIT marshal, PolyTree rebuild, allocation) is **0.19–0.56 %** of
that primitive time, so
the required 58–79 % wall cut must come from **fewer/smaller sweep operands**,
not from plumbing. The first priced shapes are **S6** (exact occupancy memo:
**14,227.7 ms** thread / **6,504 ms** wall on base classic-on) and **S5**
(bounded-delta windowed internal-bridge offsets: **21.0 s** thread / **2.5 s**
wall on base classic-off), and the largest measured scheduling lever is the
planner's **2.19x** effective parallelism (**25,625 ms** wall at 4x on
base-on). No single shape reaches a cell's required cut. **Frontier:**
[S6 occupancy memo](issues/132-planner-occupancy-memo.md),
[S5 windowed offsets](issues/133-internal-bridge-windowed-offsets.md),
[planner effective parallelism](issues/134-planner-effective-parallelism.md),
and the human's brim ruling
[130](issues/130-brim-matched-settings-ruling.md).

**Destination status (2026-11-09,
[t132](issues/132-planner-occupancy-memo.md)): S6 is banked; the frontier is S5
and planner scheduling.** The occupancy memo reproduces t131's duplicate census
exactly (**864 calls / 431 hits** base classic-on, **365 / 179** benchy
classic-on), is byte-identical across all 16 arm/cell runs, and clears the
supports-on floor at **7.39x–16.54x** with CPU corroborating: **+1.18 s**
(benchy classic-on), **+0.54 s** (benchy arachne-on), **+14.31 s** (base
classic-on), **+15.87 s** (base arachne-on). The measured wall saving is
**1.006x** the duplicate thread-time on base classic-on — the duplicate offsets
were serial, so t131's 6.5 s wall-equivalent understated the shape by 2.2x — and
the four supports-off cells are inert (`enable_support: false` returns before
either memo site), their mixed-sign medians disclosed as noise. This is **15.5 %**
of base classic-on's required prepass cut: a banked keep, not a destination step.
**Frontier:** [S5 windowed offsets](issues/133-internal-bridge-windowed-offsets.md),
[planner effective parallelism](issues/134-planner-effective-parallelism.md),
and the human's brim ruling
[130](issues/130-brim-matched-settings-ruling.md).

**Destination status (2026-11-09,
[t133](issues/133-internal-bridge-windowed-offsets.md)): S5 is banked exact;
the frontier is planner scheduling and the brim ruling.** Windowing the
**negative** `deep_infill_area` erosion only (candidate-bbox operand grown by the
miter reach) is **byte-identical in all 16 output cells** — every arm
reproduces t123's reference set — and clears the 0.80 % floor in **8/8** at
**1.66x–10.94x** with CPU corroborating: benchy **+0.19 / +0.13 / +0.14 /
+0.13 s**, base classic-off **+2.69 s (+6.34 %)**, base classic-on **+1.36 s
(+1.33 % over 12 pairs)**, base arachne-off **+3.84 s (+8.76 %)**, base
arachne-on **+4.77 s (+4.46 %)**. Windowing the positive offset too is
**rejected**: `internal_bridge_angles` reads the whole-layer clip area, and the
variant moves 8 benchy bridge layers by up to 22.09 mm. Route arithmetic: this is
**9.7 %** of base classic-off's required prepass cut and **1.5 %** of base
classic-on's — a banked keep, not a destination step. **Frontier:**
[planner effective parallelism](issues/134-planner-effective-parallelism.md)
and the human's brim ruling
[130](issues/130-brim-matched-settings-ruling.md).

**Destination status (2026-11-09,
[t134](issues/134-planner-effective-parallelism.md)): planner scheduling is a
tranche, not the route; the frontier is two supports-on shapes and the brim
ruling.** The planner's effective parallelism is **2.64x** at the stage
bracket and **2.76x** at the module on base classic-on (**2.09x / 2.31x** on
benchy classic-on), up from t131's **2.19x** because t132 removed 14.2 s of
*serial* duplicate occupancy work. The **105.1 s** of stage clipper thread-time
is **95.3 %** layer-independent emit pass and **2.5 %** recurrence-bound drop
loop; inside the emit pass the 433 per-layer occupancy offsets are **14,249 ms
wall at 0.99x**, while the clip core is **17,811 ms wall at 4.81x**. The
occupancy batch ceiling is **11.2 s** (**11.1x** floor, **12.1 %** of base-on's
required cut) and clip-to-12x is **10.7 s** (**10.6x**, **11.5 %**); combined
**21.9 s** (**21.7x**, **23.7 %**), still below the ticket's **50.66 s**
threshold. Even a zero-cost planner leaves **2.87x–3.25x** on the four
supports-on cells. **The literal all-8-cell gate is structurally unreachable:**
the whole `PrePass::SupportGeometry` stage is **0.64–0.81x** the 0.80 % floor
on the supports-off cells and the planner module alone is **0.32–0.43x**.
Graduated [t135](issues/135-cross-layer-occupancy-batching.md) (cross-layer
occupancy batch) and [t136](issues/136-planner-clip-batch-fanout.md) (clip
fan-out diagnosis), both blocked by the human's gate ruling
[t137](issues/137-planner-scheduling-gate.md).
**Frontier:** [137](issues/137-planner-scheduling-gate.md) (HITL) and the
human's brim ruling [130](issues/130-brim-matched-settings-ruling.md) (HITL);
[135](issues/135-cross-layer-occupancy-batching.md) and
[136](issues/136-planner-clip-batch-fanout.md) are blocked by 137.

**Destination status (2026-11-09,
[t130](issues/130-brim-matched-settings-ruling.md)): the matched-settings
claim is closed by ruling — both adhesion/priming asymmetries die in a staged
re-take folded into t135.** The job keeps one adhesion feature and one priming
skirt at shared authored values — `brim_width = 8`, `skirt_loops = 6`,
`skirt_distance = 3.0`, `skirt_height = 1` — asserted per run on both arms, and
both surfaces are replaced by canonical geometry (port `make_brim` (`Brim.cpp`)
and the `Print.cpp` skirt construction at the pinned `v2.5.0` = `40ecc66f78`
into `com.core.skirt-brim`, replacing the bounding-box rectangles; OrcaSlicer
attribution header). Standing policy: **close asymmetries by default** before
any new ratio of record, with **parity work licensed only for closing
asymmetries** (the third recorded exception in Out of scope); open-ended route
parity still takes the per-key t81-style path. The work is staged into
[135](issues/135-cross-layer-occupancy-batching.md): Stage 1 (unblocked) is the
re-take — port, matched keys, fail-closed asserts, a fresh 16-cell campaign
recording the **new reference set** with its own old-vs-new paired attribution
— Stage 2 is the occupancy batch under
[137](issues/137-planner-scheduling-gate.md)'s gate ruling, and the closing
gate is the 8-cells × 3-tools scoreboard re-derivation. Until Stage 1 lands,
t124's ratios of record stand as historical and every scoreboard keeps the
disclosure. **Frontier:**
[137](issues/137-planner-scheduling-gate.md) (HITL) and
[135](issues/135-cross-layer-occupancy-batching.md) Stage 1 (AFK, takeable
now); [136](issues/136-planner-clip-batch-fanout.md) and t135 Stage 2 are
gated by 137.

**Destination status (2026-11-10,
[t135 Stage 1](issues/135-cross-layer-occupancy-batching.md)): the
matched-settings asymmetry is CLOSED, the new reference set is the currency, and
the frontier is the human gate [137](issues/137-planner-scheduling-gate.md) plus
t135 Stage 2.** `com.core.skirt-brim` now emits canonical geometry at the pinned
`v2.5.0` (`40ecc66f78`): the brim is the contour-anchored annulus filled with
concentric lines (`make_brim` / `tryExPolygonOffset`) and clipped against the
first-layer support *region*, the skirt is the convex hull of the first
`skirt_height` layers plus the brim footprint (`Print::_make_skirt`), and both
use the canonical **flow spacing** `line_width - first_layer_height x (1 - pi/4)`
(`Flow::rounded_rectangle_extrusion_spacing`). **Seven** matched keys are
authored and asserted per run on both arms: `brim_width = 8`,
`brim_type = outer_only`, `skirt_loops = 6`, `skirt_distance = 3`,
`skirt_height = 1`, `initial_layer_line_width = 0.4`,
`tree_support_brim_width = 0`. The last three are measured corrections:
`auto_brim` silently overrides an explicit width; **Orca resolved a 0.5 mm
first-layer width against PnP's 0.4**, which quantised the same 8 mm to 7.31 vs
8.0 mm (the t82 "0.4 on both sides" claim is falsified by the resolved
appendix); and Orca resolved a 3 mm support brim under the same
`; FEATURE: Brim` label. **Output:** the baseline arm reproduces t123's eight
hashes, and the candidate records the **new reference set** (`7db0641b…`,
`c02aa4e9…`, `a2f7a1fe…`, `2326455d…`, `7ef84447…`, `1ddea2b7…`, `defae354…`,
`4aa1847c…`), identical on both build arms. **Delta audit:** every motion line
outside `Skirt`/`Brim` is identical with the E field stripped, |dE| <= 0.00098 mm
in 8/8. **Parity oracle: strict 2/8, not widened.** Benchy supports-off passes
outright (brim work +2.4/+2.6 %, brim area +1.3 %, 22 lines); base supports-off
is 22/22 lines at +8.1/+7.3 % brim work and +3.2/+3.1 % skirt area against a 3 %
band, the residual being the anchor convention (PnP anchors on the outer-wall
path, Orca on the slice contour, which g-code does not carry); supports-on is
dominated by the support engine (base clips the whole brim on both arms at 0/0
loops but the skirt hull differs 12.5 %; benchy's Orca brim fragments into 39
runs around its support pads against PnP's 19). Radial extent is 8/8 within
0.92 mm. **A/B:** wall 0.963–1.046, CPU 0.992–1.026 — below the box's paired
noise floor. **Scoreboard:** **2.445x–3.769x ordinary / 2.420x–3.745x
accelerated**, required cut **59.1 %–73.5 %** — t124's 2.36x–4.65x is superseded,
not improved (the adhesion job and the first-layer width both changed).
**Frontier:** [137](issues/137-planner-scheduling-gate.md) (HITL),
[135](issues/135-cross-layer-occupancy-batching.md) Stage 2 (gated by 137) and
the new [layer-0 cross-section parity](issues/138-layer0-cross-section-parity.md)
(AFK, takeable now — the oracle's remaining residual is upstream of the module);
[136](issues/136-planner-clip-batch-fanout.md) stays gated by 137.

**Destination status (2026-11-10,
[t138](issues/138-layer0-cross-section-parity.md)): the layer-0 residual is a
slice-plane convention, not a module rule; the supports-off fix is a fairness
ticket and supports-on is the engine.** PnP slices every layer at its **top**
(`z = first_layer_height + n × layer_height`); canonical OrcaSlicer slices at the
layer **midpoint** (`compute_slice_z` = `0.5 × (lo + hi)` in
`PrintObjectSlice.cpp`, ZAA off). Ten sampled layers put PnP on the top plane
(0.02–1.36% error) and Orca on the mid plane (0.02–4.02%); the decisive base L0
and L10 rows separate the planes by 253% / 161%. On `base.stl` the 0.1 mm shift
is **3.42x** of layer-0 area (3.122 vs 0.914 mm²); on the benchy **1.3%**. It is
also visible in the layer count: base **495 / last Z 99.0** on PnP against
**496 / 99.2** on Orca. The base supports-off skirt failure (+3.23% / +3.10%
against 3%) is the brim footprint (+8.68%) diluted by the skirt offset; the
brim path decomposes +88.04 mm contour − 27.65 mm anchor = +60.39 mm predicted
vs +57.39 mm observed. Wall width (0.4 vs 0.42) is a 3.3% same-plane effect;
first-layer height is equal. The four supports-on cells are the **support
engine**: Orca's first-layer support is **10.6–11.7x** denser by path and
**10.4–11.5x** by coverage, and the skirt output tracks the support-dominated
input hull (base 1.1528 → 1.1248; benchy 0.9567 → 0.9661). **The adhesion
oracle is 2/8 and cannot reach 8/8 without a slicing change:** without one the
ceiling is 6/8 (all but base supports-off, only with perfect support parity);
with the slice-plane change alone it is expected **4/8** (the supports-off
cells); 8/8 needs both. The slice-plane change is a fairness fix, not a route
step — base layer-0 operand 5.329 → 2.314 mm² (2.30x) but one layer of 495
(0.20%), benchy 634.46 → 627.75 (1.01x, 0.42%); no wall claim, and the
59.1–73.5% required cut is unchanged. **Frontier:**
[139](issues/139-layer-plane-parity.md) (AFK, takeable now), the human's
[137](issues/137-planner-scheduling-gate.md) (HITL), and
[135](issues/135-cross-layer-occupancy-batching.md) Stage 2 /
[136](issues/136-planner-clip-batch-fanout.md) (gated by 137).

**Destination status (2026-11-11,
[t139](issues/139-layer-plane-parity.md)): the slice plane is canonical, the
matched job has a new reference set, and the route arithmetic is unchanged;
the frontier is the human's gate [137](issues/137-planner-scheduling-gate.md),
the new brim-anchor ruling [140](issues/140-brim-contour-input.md), and t135
Stage 2 / t136 (gated by 137).** `PrePass::Slice`, the host layer schedule and
the planner now sample at canonical `compute_slice_z`'s band **midpoint**
(`0.5 × (lo + hi)`); `layer.z` stays the print top. Base prints **496 layers /
last Z 99.2** (was 495 / 99.0); the benchy stays 240 / 48.0. The layer-0
outer-wall hull fits the STL midpoint on every cell (base classic 0.932 vs mid
0.955 / top 3.165; benchy 513.195 vs 513.415 / 520.028). The change also fixed
a pre-existing slicer defect at base's new 69.3 mm plane
(`push_unique_intersection` merged distinct-topology cut points; **671.2054
mm² → 0.0025 mm²**, layer vanished) and moved modifier-volume sampling onto the
same plane at all four sites so the Tier-2 hashes still match. **Reference set:**
`93e21fee…` / `b3f1eccc…` / `3510e876…` / `96cdac85…` / `ca14d055…` /
`585c6a8e…` / `d8fdc1d9…` / `c9fee59a…`, byte-identical on both build arms; the
pre-change arm reproduces t135's set exactly. **Oracle: still 2/8** — base
supports-off now fails the 3 % skirt band as an **undershoot** (−4.3 % / −4.4 %
swept, −5.7 % / −6.6 % outer loop; brim +8.1 % → −5.2 %), the inward wall-path
anchor now standing alone; supports-on is the support engine. **A/B
(attribution):** wall 0.923x–0.984x, CPU 0.963x–0.991x; supports-on filament
−18.5 %. **Scoreboard: 2.279x–4.187x ordinary / 2.311x–4.179x accelerated,
required cut 56.1 %–76.1 %**, superseding t135's 2.445x–3.769x with the
disclosure that both arms also moved with machine state (Orca base-on 27.86 →
22.85 s, PnP 105.01 → 93.55 s), so the movement is not attributable to the plane
alone. **DEV-201 is closed; DEV-202 records the canonical-sampling residuals
outside the matched job** (p241 `agg` drops 3/0/0 → 3/1/6; calicat internal-bridge
sites one layer later at the same areas). **Frontier:**
[137](issues/137-planner-scheduling-gate.md) (HITL),
[140](issues/140-brim-contour-input.md) (HITL, new),
[135](issues/135-cross-layer-occupancy-batching.md) Stage 2 and
[136](issues/136-planner-clip-batch-fanout.md) (gated by 137).

**Destination status (2026-11-11,
[t141](issues/141-overhang-quartile-band-deletion.md)): the overhang band
partition is deletable work, and the shell program has a priced candidate; the
frontier is t142, the human's gate and brim ruling, and the two gated planner
tickets.** The 4-band polygon partition's **value has no production reader** —
`overhang-classifier-default` reads only `overhang_quartile.is_some()` and
computes speed from `overhang_distance_mm` via `calculate_speed`; the presence
bit is the sign of the same signed distance that per-vertex loop already
computes against the same previous-layer boundary (`prev_layer_boundary`).
Price on the post-t139 tree (probe, `evidence/t141-overhang-bands/`): base
classic-off **8.964 / 9.558 / 10.583 s** — **22.1–26.5 %** of the 39.9–40.6 s
same-session wall and **34.3–40.5 %** of the cell's 26.15 s required cut — base
arachne-off **9.450 s**, base classic-on **11.195 s**, benchy classic-off
**0.954 s**, benchy classic-on **0.817 s**. Operation counts are **3 offsets + 6
whole-layer booleans per band layer** in the cumulative stage over **486 of
496** base layers and **239 of 240** benchy layers; the band thread-time
converts at 11.6–11.7x on base, reproducing t84's 11.86x. The deletion is not a
route step on its own (t139's 2.899x base classic-off moves to about 2.19x),
but it is the largest single unpriced shell shape and the first whose **work
set** can go. Two build forks are recorded: the per-layer boundary key
(`prev_layer_boundaries` exists only for overhang layers) and the visual-debug
band tap. Graduated [t142](issues/142-overhang-band-partition-deletion.md).
**Frontier:** t142 (AFK, takeable now),
[137](issues/137-planner-scheduling-gate.md) (HITL),
[140](issues/140-brim-contour-input.md) (HITL),
[135](issues/135-cross-layer-occupancy-batching.md) Stage 2 and
[136](issues/136-planner-clip-batch-fanout.md) (gated by 137).

**Destination status (2026-11-11,
[t142](issues/142-overhang-band-partition-deletion.md)): the overhang band
partition is deleted, the quartile is distance-derived, and the matched job's
G-code did not move; the frontier is the human's gate and two HITL rulings.**
`annotate_overhangs` now commits only the unsupported area + the previous-layer
footprint (Shape A: the per-layer `difference_ex` key test is kept), and
`overhang_quartile` is bucketed from
`signed_distance_to_boundary` against the producer's band width, so the 3 round
offsets + 6 whole-layer booleans per overhang layer are gone; the band R-tree
and its point-in-polygon query are gone with them. **Output: byte-identical in
8/8 cells**, and every hash is t139's reference value
(`93e21fee…`/`b3f1eccc…`/`3510e876…`/`96cdac85…`/`ca14d055…`/`585c6a8e…`/
`d8fdc1d9…`/`c9fee59a…`), stable across all 48 A/B rows — the reference set does
not move. **Timing (3 interleaved pairs, ordinary): 26.25 % / 11.76 % / 25.58 % /
9.69 % / 20.00 % / 10.56 % / 20.23 % / 12.66 %** wall saved on base classic-off /
-on / arachne-off / -on and benchy classic-off / -on / arachne-off / -on;
candidate faster in **24/24** pairs (ratios 0.738–0.940), process CPU lower in
**8/8** (14.6 %–32.6 %), no cell regresses. Composing with t139's paired
scoreboard (indicative, not a re-taken scoreboard): **1.818x–3.697x** ordinary,
required cut **45.0 %–73.0 %**; base classic-off **2.899x → 2.138x**, base
classic-on **4.187x → 3.695x**. Disclosures: the **accelerated timing arm is not
taken** (the controlled-driver `RUSTC` wrapper + MSVC 14.51 mangles the
`libmimalloc-sys` verbatim source path; the same build without the wrapper
succeeds), but accelerated **output parity was checked** with a disclosed manual
`--cfg pnp_perimeter_spatial_accelerated` host plus the accelerated guest
artifacts — **8/8 cells emit the same hashes**; the 16 output cells are 8 ×
{baseline, candidate} ordinary; two
pre-existing reds are recorded, not caused (`integrated_parity_skirt_brim_tdd`'s
missing `brim_object_gap`, and `layer_slice_tdd`'s z = 0.1 layer now sampling
z = 0.0 under t139's midpoint plane); two latent-vacuous quartile fixtures were
repaired. Evidence:
[`evidence/t142-overhang-band-deletion/`](evidence/t142-overhang-band-deletion/FINDINGS.md)
(`verify.py` re-derives every number; `--self-test` 4/4 controls).
**Frontier:** [137](issues/137-planner-scheduling-gate.md) (HITL),
[140](issues/140-brim-contour-input.md) (HITL),
[135](issues/135-cross-layer-occupancy-batching.md) Stage 2 and
[136](issues/136-planner-clip-batch-fanout.md) (gated by 137). No new AFK
ticket: the deletion answered its own question and opened none.

**Destination status (2026-11-12,
[t137](issues/137-planner-scheduling-gate.md)): the four-cell planner gate is
confirmed and generalized as the standing rule for representation-safe
planner-only candidates; [135](issues/135-cross-layer-occupancy-batching.md)
Stage 2 and [136](issues/136-planner-clip-batch-fanout.md) are unblocked on the
frontier.** Because the whole `PrePass::SupportGeometry` stage is
**0.64x–0.81x** the 0.80 % floor on the four supports-off cells (and the planner
module alone is **0.32x–0.43x**), no planner-only candidate can meet the literal
all-8-cell 0.80 % gate. The human ruled to extend the [t117](issues/117-strong-planner-carry-ruling.md)
four-cell form to t135 Stage 2 and t136, and to generalize it as the standing
gate rule for any representation-safe planner-only candidate where supports-off
is structurally inert (`enable_support: false` declines before the candidate's
work): positive 0.80 % paired wall/CPU improvement in the four supports-on
cells, byte-identical output in all eight against the active reference set
([t139](issues/139-layer-plane-parity.md)'s set), and no median wall/CPU
regression in the four supports-off cells. The 8/8 destination is unchanged.
**Frontier:** [135](issues/135-cross-layer-occupancy-batching.md) Stage 2 (AFK,
unblocked), [136](issues/136-planner-clip-batch-fanout.md) (AFK, unblocked), and
[140](issues/140-brim-contour-input.md) (HITL).

**Destination status (2026-11-12,
[t135 Stage 2](issues/135-cross-layer-occupancy-batching.md)): the planner's
serial occupancy half is batched and banked; the frontier is [136](issues/136-planner-clip-batch-fanout.md)
and the human's [140](issues/140-brim-contour-input.md).** The emit pass's
per-layer occupancy offsets — the 433–434 singular `+support_object_xy_distance`
offsets t134 measured at **0.99x** — are now issued as **109 `offset_polygons_batch`
calls / 434 requests** over the existing `EMIT_BATCH_LAYERS = 4` chunk, with the
content memo seeded from the batch so both memo sites answer from it. Each
request is the same clipper op with the same arguments and the host preserves
input order, so exactness is by construction. **Output byte-identical in 8/8
cells**, equal to t139's reference set
(`93e21fee…`/`b3f1eccc…`/`3510e876…`/`96cdac85…`/`ca14d055…`/`585c6a8e…`/
`d8fdc1d9…`/`c9fee59a…`) and stable across every A/B row; the accelerated
**guest** artifacts emit the same eight hashes (hybrid with the ordinary host —
the accelerated host still fails in the controlled driver on `libmimalloc-sys`,
unchanged from t142). **The batch's own fan-out replaces the 4.81x proxy:
3.52x** on both base supports-on cells (18.6 s summed request time → 5.29 s
wall), **2.39x/2.24x** on benchy classic-on/arachne-on. **Paired wall:
+2.93 % / +7.22 % / +10.80 % / +13.31 %** (benchy classic-on / arachne-on, base
classic-on / arachne-on) at **3.66x–16.64x** the 0.80 % floor, negative in 19 of
20 supports-on pairs; base saves **9.76 s / 12.37 s**. **The CPU record is flat,
not lower** (+1.09…−1.34 %), and the take measured the box's own noise rather
than assuming it: an identical-arm null control bounds a pair at **±11.02 %
wall / ±5.60 % CPU**, so the strict CPU-lower form is disclosed **0/4** and the
declared gate reads CPU as *not contradicted*. The four supports-off cells are
structurally inert (`enable_support: false` returns before `plan_for_object`)
and read **−0.59 / +0.44 / +2.69 / −0.35 %** wall and **−0.23…+0.40 %** CPU, all
inside the null band; the strict non-negative-sign form is disclosed **2/4**,
and base arachne-off settled from −3.06 % to −0.35 % at 24 pairs against a
baseline-vs-baseline null median of −2.06 % on that cell. Route: **10.6 %** of
base classic-on's 92.5 s required cut and **13.2 %** of arachne-on's, close to
t134's 11.2 s ceiling for the shape; indicative base classic-on composition
**3.695x → ~3.30x**, so the operand-volume route and the 8/8 destination are
unchanged. **Frontier:** [136](issues/136-planner-clip-batch-fanout.md) (AFK,
unblocked) and the human's [140](issues/140-brim-contour-input.md) (HITL).

**Destination status (2026-11-12,
[t136](issues/136-planner-clip-batch-fanout.md)): the clip core's deficit is one
phase wide, not scheduling-wide; [143](issues/143-union-phase-request-grouping.md)
carries the tranche and the human's [140](issues/140-brim-contour-input.md) is
still the HITL frontier.** The clip core is **15,841.8 ms** on base classic-on
and the 4.81x is two different numbers: `carve_run` — the phase the batch API
was built for — issues **65,233 requests** over 312 calls at the saturated
256-request cap and runs at **11.56x** (40,977.6 ms thread in 3,544.7 ms),
while `union_run` issues **482 requests** over 109 calls (**4.4 per call**) and
runs at **3.66x** (37,237.5 ms in 10,176.2 ms) while owning **64.2 %** of the
clip-core wall. The union phase's summed per-call critical-path floor is
**10,168.3 ms** against a **10,176.2 ms** wall — **8 ms** of the phase is
anything but each call's longest request — so this is a **parallelism-width**
deficit, not load imbalance: at 4.4 requests per call 7.6 of 12 threads are idle
by construction. The additive split is **6,587.6 ms** work floor (**41.6 %**),
**7,224.0 ms** tail-times-chunking (**45.6 %**), **144.8 ms** dispatch excess
(**0.9 %**), **1,393.7 ms** marshal+decode (**8.8 %**) and **491.7 ms** guest
CPU (**3.1 %**), so the ticket's guest-assembly candidate is refuted and the
256 cap is saturated where the work is. Merging the union phase across four
chunk-batches takes its measured floor to **3,465.0 ms** (**5,480.2 ms** at 8
layers, **3,103.1 ms** whole-phase); LPT's 4/3 bound puts the achievable saving
at **5,556–6,711 ms** = **7.3 %** of base classic-on's 92.5 s required cut,
**6.6x** the 1,010 ms floor. That is a tranche of t135 Stage 2's order, not a
destination step; graduated
[143](issues/143-union-phase-request-grouping.md) (memory is its first check:
`EMIT_BATCH_LAYERS` 4 → 16 quadruples the buffered collision sets). **Frontier:**
[143](issues/143-union-phase-request-grouping.md) (AFK, unblocked) and the
human's [140](issues/140-brim-contour-input.md) (HITL).

**Destination status (2026-11-12,
[t140](issues/140-brim-contour-input.md)): the brim anchor is a real geometry
defect, licensed as the fourth scoped exception and graduated as
[t144](issues/144-brim-anchor-reconstruction.md); the frontier is t143 and
t144, both AFK.** `SkirtBrim::generate` anchored `make_brim` on the outer-wall
**centerline**, 0.20 mm inside canonical's slice contour, so the brim's inner
boundary sat 0.1 mm inside the part outline and the innermost bead overlapped
the footprint instead of touching it; the whole ring was 0.2 mm inward on every
cell. The human kept t138's classification (not a t130 setting asymmetry),
granted a **scoped t81-style exception** for the `skirt-brim` anchor inputs
only, and ruled the reconstruction **module-side** (offset the filled footprint
outward by half the resolved outer-wall width; the same for the `skirt_height`
layer hulls), not a SliceIR/WIT addition. **Ceremony:** fresh 16-cell reference
set on both arms, delta audit, the four supports-off oracle cells re-run, a
3-pair wall/CPU A/B as attribution only, and **no** 8-cells × 3-tools scoreboard
re-derivation (the surface is ~235 ms; t139's ratios stand with the
adhesion-geometry disclosure).
**Stop rule:** the t139 fairness-fix form — geometry correctness plus a bounded
audit; the oracle is a measure, not a revert gate, and a band miss graduates the
variable-width envelope or the direct contour. The oracle ceiling from this fix
is **4/8** (supports-on is out of scope). **Frontier:**
[143](issues/143-union-phase-request-grouping.md) (AFK, unblocked) and
[144](issues/144-brim-anchor-reconstruction.md) (AFK, unblocked); no HITL ticket
remains open.

**Destination status (2026-11-12,
[t143](issues/143-union-phase-request-grouping.md)): the planner's union tail is
banked at buffering depth 12, depth 16 refutes its own merge-4 projection, and
the benchy fixture swap retires the benchy reference set; the frontier is
[144](issues/144-brim-anchor-reconstruction.md) and the new
[145](issues/145-benchy-fixture-rebaseline.md), both AFK.** The planner's union
phase was one request deep (109 calls × 4.4 requests, 3.73x fan-out); raising
`EMIT_BATCH_LAYERS` 4 → 12 merges it into 37 calls of ~13 requests and takes the
probe's union wall **10,093.1 → 4,848.7 ms (−52.0 %)** and the clip core
**14,158.5 → 8,934.5 ms** on base classic-on. **Depth 16 fails its own
bracket:** the same 482 requests cost **46.92 s** of thread-time against
**37.68 s** at depth 4 (+24 %) once sixteen layers of collision sets are live,
so its union wall is **5,793.2 ms** against the **4,620 ms** LPT bound; depth 8
lands on the merge-2 floor (5,459.7 vs 5,480.2 ms) and 12 is the measured
optimum (chosen after the table, disclosed). **Paired A/B (3 interleaved pairs):
supports-on wall +6.72 / +7.36 / +12.80 / +13.41 %** (benchy classic-on /
arachne-on, base classic-on / arachne-on) at **8.40x–16.76x** the floor,
**+9.56 s on base classic-on = 10.3 %** of its 92.5 s required cut and **2.7x**
the ticket's 3,500 ms DROP bar; supports-off inside the null band and
structurally inert. **Output byte-identical in 8/8**; base cells reproduce
t139's set exactly, benchy cells match the take's baseline on the **replacement**
benchy export. **Disclosed gaps:** the strict CPU-lower clause passes **1/4**
(−1.26…−1.83 % on the rest, inside the ±1.88–1.91 % null CPU band: a wider
fan-out burns more CPU to finish sooner), no accelerated timing arm, no Orca arm,
3 pairs per cell, and today's base classic-on baseline is 74.70 s against
t139's 126.27 s cell so only paired deltas are claimed. **Fixture swap:** the
user-supplied `tmp/3dbenchy.stl` was replaced with a different export of the
same model (240 layers / 48.0 mm); t139's four benchy hashes are retired,
`base.stl` is byte-identical, and the new benchy baseline is
`a89e449b…` / `dceb7a48…` / `3286449f…` / `be786718…`. **Frontier:**
[144](issues/144-brim-anchor-reconstruction.md) (AFK, unblocked) and
[145](issues/145-benchy-fixture-rebaseline.md) (AFK, unblocked).

**Destination status (2026-11-12,
[t144](issues/144-brim-anchor-reconstruction.md)): the anchor is the
reconstructed contour, the matched job moves 6/8 cells, and the adhesion oracle
reaches 3/4 supports-off (3/8 overall); the next shape is the Arachne residual
decision [146](issues/146-variable-width-anchor-shape.md), and the route
arithmetic is unchanged.** `SkirtBrim::generate` now offsets the filled
outer-wall footprint outward by **half the per-object modal non-zero
`Point3WithWidth.width`** (Round, 0.0125 mm) before the existing
gap/quantisation/`tryExPolygonOffset` steps, and does the same to each
`skirt_height > 1` layer hull before it joins the skirt hull; the support clip,
the 0.1 mm gap and every other brim step are untouched, module-only, no contract
change. **Reference set:** the baseline reproduces the active pre-t144 set in
**8/8**; the candidate is byte-identical on the ordinary and accelerated arms in
**8/8** and moves in **6/8** — `1ae0cfa8…` / `d9fc80ba…` / `8faf02b1…` /
`cd7d4a2d…` / `c0b9014f…` / `585c6a8e…` / `17f63e93…` / `c9fee59a…` — recorded
as [`evidence/t144-brim-anchor/reference.json`](evidence/t144-brim-anchor/reference.json),
the active set for later output tickets. The two inert cells are base
supports-on: the first-layer support footprint clips the whole brim annulus, so
no brim path (nor a brim hull point) reaches the output; measured, disclosed,
and their delta audit is zero changed `E` lines. **Delta audit 8/8:** no motion
outside `Skirt`/`Brim` moves with `E` stripped; max `|dE|` **0.00098 mm**.
**A/B (attribution only):** 3 pairs/cell, medians inside a measured six-cell
null band (worst **±24.82 % wall / ±4.52 % CPU**); no wall claim. **Oracle:**
3/4 supports-off pass — base classic-off flips from **−5.65 %** to **+2.89 %**
skirt outer, base Arachne-off remains **+3.45 %** (0.45 points out), both benchy
cells pass at +1.32 / +1.34 %; supports-on is support-engine-limited (t138/t139)
and its output is byte-identical in this take. The uniform modal width is the
pre-declared approximation on Arachne, so the miss graduates
[t146](issues/146-variable-width-anchor-shape.md) (per-segment envelope vs
direct contour vs accept 3/8) rather than reverting the anchor. No scoreboard
re-derivation: t139's **2.279x–4.187x ordinary / 2.311x–4.179x accelerated,
required cut 56.1 %–76.1 %** stand with the adhesion-geometry disclosure. Both
freshness gates exit 0; the accelerated **host** remains unbuildable
(`libmimalloc-sys`, t142), so accelerated output used the ordinary host plus the
accelerated component tree. **Frontier:**
[146](issues/146-variable-width-anchor-shape.md) (HITL) and
[145](issues/145-benchy-fixture-rebaseline.md) (AFK, unblocked).

**Destination status (2026-11-13,
[t145](issues/145-benchy-fixture-rebaseline.md)): the benchy currency is re-based
and the post-swap scoreboard is re-derived; the frontier is the human's
[t146](issues/146-variable-width-anchor-shape.md) and the new supports-on
attribution [147](issues/147-supports-on-body-rejection-attribution.md).** The
replacement `tmp/3dbenchy.stl` is a **tessellation-only** re-export — 225,786 →
225,706 triangles, |Δbounds| ≤ 4.05e-7 mm, bounds and 240-layer / 48.0 mm
schedule equal, and **ten of ten** sampled cross-sections (5 layers × mid/top)
equal to t138's recorded old-export values — but the slice is **not**
output-neutral. Against the old export at one output-neutral code change apart
(all four base cells **byte-identical** across the two takes, the control that
makes the benchy delta a fixture effect): outer-wall path −1.591 mm of
31,891.789 (**−0.0050 %**) and loop area −1e-6, per-layer area median/p95 0.0
with one layer above 0.1 % (L153, −0.113 %); **internal bridge on L40
(z = 8.2) only** 554.316 → 611.614 mm (**+57.298 mm, +7.344 %**) and
28.028 → 31.866 mm² (**+13.694 %**), identical in all four cells; **support
−1,370.181 mm (−2.238 %)** over 176 supports-on layers while its max-loop area
rises **+1.021 %**; support interface +1.174 %, brim +0.825 % / +0.851 %,
skirt +0.352 % follow the changed footprint; bytes classic-off −812,
classic-on −79,465, arachne-off +118, arachne-on −78,773. **Currency:** t144's
four benchy hashes are active and reproduced in 8/8 cells today (one hash across
three measured runs per cell, and the four base cells reproduce t144's base
hashes in the same campaign); t139's four are retired; t143's four are the
superseded interim set. **Post-swap scoreboard (8 cells × 2 tools, one window,
1 warmup + 3 measured, matched keys asserted per Orca run): 1.853x–3.455x
ordinary/Orca, required cut 46.03 %–71.06 % (19.05 %–56.59 % at 1.5x)** —
benchy 2.043x / 2.683x / 1.853x / 2.580x, base 2.078x / 3.001x / 1.984x /
3.455x; the lowest bars are now the two benchy supports-off cells (46.03 %,
51.05 %) and the highest the two base supports-on cells (66.67 %, 71.06 %).
**Disclosures:** the accelerated **timing** arm is not taken — `cargo xtask dist
--accelerated` still fails today in `libmimalloc-sys` (`c1xx` C1083 on
`'\\mimalloc-static.cc'`), and a manual-cfg host is barred as a timing artifact
by `docs/23`; every PnP supports-on run is `degraded: true` with **101** (benchy)
/ **44,474** (base) code-1200 `exact-Z occupancy` rejections, identical on both
exports and therefore not swap-caused, graduated as
[t147](issues/147-supports-on-body-rejection-attribution.md); load 9 % / 12 %;
three runs per cell; Orca g-code hashes differ per run (per-run timestamp) while
bytes are constant. No causal claim is taken from the cross-take ratio movement
(t139's box was slower). Evidence:
[`evidence/t145-benchy-rebaseline/`](evidence/t145-benchy-rebaseline/)
(`verify.py` 98/98 checks, 25/25 negative controls; `reduce_t145.py` regenerates
the reduced set; `reference.json` is the currency). **Frontier:**
[t146](issues/146-variable-width-anchor-shape.md) (HITL) and
[t147](issues/147-supports-on-body-rejection-attribution.md) (AFK, unblocked,
new).

**Destination status (2026-11-13,
[t147](issues/147-supports-on-body-rejection-attribution.md)): the supports-on
"unmet demand" is a planner/validator plane mismatch, not redundant demand; the
four supports-on ratios are supports-incomplete and a fairness ticket is
graduated.** The probe is output-neutral (both probe G-code hashes are the t145
currency) and compares every rejected entry against both planes: **all 33
entries overlap the print plane and none overlaps the midpoint payload the
planner carved** — benchy L170 **5.8342 mm² (2.276 % of the body)**, base 32
entries **8.4e-6–1.4706 mm²** (sum **6.8920 mm²**, median 0.0805 mm², ≤ 0.069 %
of body). The 44,474 base diagnostics are **(body, demand) pairs**: 2,892
unique demands, **zero** uniquely dropped (every one is also carried by a
retained entry). The loss is the **layer**: each rejected entry is the only
entry at its layer, so **32 complete base layers** (`15, 18, 19, 27, 33–36,
38–41, 80, 99, 132, 155, 156, 177, 192, 216, 218, 259, 266, 316, 329, 351,
352, 362, 387, 395, 411, 414`) and **1 benchy layer** (L170) are absent from the
emitted G-code: **36,466.736 mm (10.2765 %)** of base support path and
**192.601 mm (0.233 %)** of benchy path by neighbour interpolation. The emitted
check is not anecdotal: **19/32** base and **1/1** benchy overhangs directly
above a missing layer have Orca support at that plane (benchy candidate 68:
PnP 0 / Orca 36.85 % at z = 34.2; second candidate 76: PnP 0 / Orca 14.7–32.7 %
at 36.0–36.4) and none in PnP; 13/32 are unsupported by both tools at that
exact plane and are not counted. **Classification (b) defect**, on two layers:
`layer_occupancy_payload`/`build_roles_batched` carve
`SupportAnalysisIR::model_occupancy` at `layer_sample_plane_z` = `layer.z −
0.5 h` while `validate_entry` queries exact-Z at `anchor_z = layer.z`; the
zero-tolerance gate then drops the whole body (canonical `draw_circles` would
keep **250.5 of 256.3 mm²** benchy and ≥ 99.93 % of every base body). **Ruling:
the four supports-on ratios of record are not quotable as a matched-settings claim**
(PnP paid less work than the matched job demands), they stand only with the
disclosure above, and no new supports-on ratio of record until the fix lands.
Supports-off cells have zero rejections and are unaffected. Graduated
[t148](issues/148-supports-on-plane-carve.md) (AFK, unblocked) to carve the
print plane and re-record the four supports-on cells. Evidence:
[`evidence/t147-supports-rejection/`](evidence/t147-supports-rejection/FINDINGS.md)
(`verify.py` checks every number from `census.json`; 6/6 negative controls
rejected; `reduce_t147.py` regenerates the reduced set; `probe.patch` is the
temporary, non-landing probe). **Frontier:**
[t146](issues/146-variable-width-anchor-shape.md) (HITL) and
[t148](issues/148-supports-on-plane-carve.md) (AFK, unblocked, new).

**Destination status (2026-11-13,
[t148](issues/148-supports-on-plane-carve.md)): the supports-on output is no
longer support-incomplete, the four supports-on ratios are re-derived, and the
frontier is the human's [t146](issues/146-variable-width-anchor-shape.md).**
The planner now carves emitted roles against the print plane the host gate
validates, published as `SupportAnalysisIR::print_plane_occupancy` (additive
schema 2.1.0; WIT `print-plane-occupancy`, package `2.1.0`) and batched per
object from the world mesh at the quantized `anchor_z` plane; the midpoint
payload is kept as the conservative extra and `validate_entry` still runs
zero-tolerance on the carved result. **Zero `exact-Z occupancy` diagnostics on
all four supports-on cells** (101 / 101 / 44,474 / 44,474 → 0), every run
`degraded: false`, the **33 dropped layers reappear** (32 base + 1 benchy, zero
lost), and support path rises **+36,700.688 / +36,700.716 mm** (base classic /
arachne) and **+294.871 mm** (both benchy cells) — base recovery is 36,723.3 mm
against t147's 36,466.7 mm estimate (+0.70 %). **Supports-off is byte-identical
in 4/4** to the t144 active set with zero diagnostics. **Ratios of record
(ordinary, supports-on): 2.705x / 2.829x / 3.356x / 3.248x** (benchy classic-on /
benchy arachne-on / base classic-on / base arachne-on), CPU corroborating; the
**active reference set is t148's**
[`reference.json`](evidence/t148-print-plane-carve/reference.json) and t145's
four supports-on hashes are superseded (t145's supports-off hashes carry
forward). **Coverage clause read twice and disclosed:** plane presence is
**20/20** (every t147 Orca-supported candidate now sits on a plane that prints
support); polygon coverage on t147's > 5 % metric is **11/19 base, 0/1 benchy**
and is **not** a pass gate because the retained-layer control reads the same
0.000–0.021 — a pre-existing tree-interface geometry gap. **No causal wall cost
claimed:** PnP process CPU rises (base classic-on 492.7 → 588.7 s median) but the
window's Orca arm moved too (23.414 → 21.598 s), and t145's base arachne-on PnP
sample was deschedule-skewed. Evidence:
[`evidence/t148-print-plane-carve/`](evidence/t148-print-plane-carve/FINDINGS.md)
(`verify.py` 360/360, 9/9 negative controls; `reference.json` is the currency).
**Frontier:** [t146](issues/146-variable-width-anchor-shape.md) (HITL) only.

**Destination status (2026-11-13,
[t149](issues/149-meshanalysis-work-set-readers.md)): `PrePass::MeshAnalysis`'s
work set is a bridge detector nothing reads, and the t129 S2 "per-layer overhang
polygons" port is answered as not the lever; the frontier is the human's
[t146](issues/146-variable-width-anchor-shape.md) and the new deletion
[t150](issues/150-meshanalysis-bridge-retirement.md).** The stage is **5,325.8
ms** on base classic-off and **85.68 %** of it is `find_bridge_clusters` +
`compute_bridge_metrics` — 7,383,702 directed half-edge insertions, 3,199 BFS
clusters, one footprint union per cluster — and **no production reader consumes
`bridge_regions`**. The only read output is
`overhang_regions[].xy_footprint` (**529.1 ms**), through
`SliceRegionView::derive_needs_support`; the producer's `needs_support` flag is
ignored by that consumer, and `surface_groups` resolves a `nonplanar_surface`
production never sets. Sub-phase on base classic-off: classify 187.0 /
overhang footprint 529.1 / eligibility 0.2 / bridge 4,563.2 (sum 5,279.5,
unaccounted 46.3); base classic-on 4,832.9 with bridge 4,150.2; benchy
classic-off 195.1 with bridge 95.1; a profiled run reconciles 4,914.1 ms probe
against 5,639.5 ms `host:mesh_analysis`. **The S2 port would replace 529.1 ms,
not the stage:** after the unread half goes, **762.6 ms** remains, before
paying canonical's per-layer diff. Route: **12.95 %** of the base-off cell,
**16.18x** the 0.80 % floor and **24.96 %** of its indicative cut (2.078x →
2.387x, cut 51.88 % → 58.11 %); base-on **4.85 % / 6.07x / 6.91 %**; benchy-off
**2.57 % / 3.21x / 5.03 %**. No production change in this take. **Frontier:**
[t146](issues/146-variable-width-anchor-shape.md) (HITL) and
[t150](issues/150-meshanalysis-bridge-retirement.md) (AFK, unblocked, new).

**Destination status (2026-11-13,
[t146](issues/146-variable-width-anchor-shape.md)): the last supports-off oracle
miss is a skirt-hull input defect, not a variable-width anchor error; neither
offered anchor shape can pass the cell, and the fix is graduated as
[t151](issues/151-skirt-hull-brim-contour.md).** The oracle's relative
difference is `abs(pnp - orca) / orca` (`oracle_skirt_brim.py::rel`) and carries
**no direction** — every supports-off cell is an **undershoot**, so t144's
"−5.65 % → +2.89 %" is a magnitude history, corrected in place in t144, its
FINDINGS and t146. Against the 3.0 % skirt band: base classic-off **2.89 %**
(0.11 pt headroom, passes), base arachne-off **3.45 %**, benchy classic-off
**1.32 %**, benchy arachne-off **1.34 %** — Arachne adds **+0.56 points** on
base and **+0.02** on the benchy, and the dominant term is shared with the
classic cell, where the uniform-width reconstruction is exact by construction.
So at most 0.56 of the 3.45 points can be a width effect. **Mechanism:** the
ring distance already matches canonical (`Print::_make_skirt`'s
`skirt_distance - spacing/2` plus `+= spacing` per loop is PnP's
`skirt_distance + spacing/2 + i * spacing`; outermost **4.964 mm** both) but the
**hull input** does not — canonical appends
`m_objectBrimAreasByInstance[...]` `area.contour.points`, the brim **area's
outer contour**, while `SkirtBrim::generate` extends `brim_points` from the
emitted beads' `path.points`, the outermost **centreline**. The offset is half a
flow spacing: measured brim→skirt bbox step **5.136 / 5.130 / 5.152 / 5.138 mm**
on Orca, **5.143** predicted, **4.962–4.964** on PnP. **Decomposition of the
~0.29 mm radial deficit: ~0.10 mm brim-extent term (inside its 12 % band) +
0.1786 mm skirt-hull term + ≤0.04 mm Arachne width.** The per-segment envelope
can only recover the Arachne increment and the direct contour only the anchor
term (already exact on classic), so **neither can pass the cell**; the direct
contour also reverses t140's module-side ruling for a ~235 ms surface. **Ruling:**
land the canonical fold, read as inside t140's exception ("the layer-0 object
footprint and the `skirt_height` layer hulls"), module-only, no new clipper
operation, **no wall claim**; the estimate is base classic-off **~0.4 %** and
base arachne-off **~0.9 %**, supports-off oracle **3/4 → 4/4**. t148's reference
set stays the currency until t151's ceremony re-records it. **Frontier:**
[t151](issues/151-skirt-hull-brim-contour.md) (AFK, unblocked, new) and
[t150](issues/150-meshanalysis-bridge-retirement.md) (AFK, unblocked).

**Destination status (2026-11-13,
[t150](issues/150-meshanalysis-bridge-retirement.md)): the MeshAnalysis bridge
product is retired and banked; the frontier is
[t151](issues/151-skirt-hull-brim-contour.md).** `classify_object` no longer
computes `bridge_regions` — the field stays in the IR as an empty vector and the
clusterer behind it is deleted — so `PrePass::MeshAnalysis` falls **4,445 → 732
ms (−83.53 %)** on base classic-off and **4,616 → 582 ms (−87.39 %)** on base
classic-on, and **168 → 95 ms** on benchy classic-off. **Identity is 8/8 cells /
16/16 runs** against t148's active reference set (`1ae0cfa8…` /
`37788518229d…` / `8faf02b1…` / `f86d8897…` / `c0b9014f…` / `eccbd523…` /
`17f63e93…` / `1811425f…`), byte-for-byte in both arms, `degraded: false`, zero
exact-Z rejections, and every A/B pair carries the same hash on both arms.
**Paired wall (medians, negative = candidate faster): −12.08 % base
classic-off (15.09x the 0.80 % floor), −4.06 % base classic-on (5.08x, six
pairs), −9.37 % / −5.78 % base arachne-off/-on, −1.77 % / −4.95 % / −4.87 % /
−3.20 % benchy classic-off/-on/arachne-off/-on**; CPU lower or flat in all eight
(−1.77 … −0.10 %). **The identical-arm null control (6 pairs, base
classic-off) bounds a pair at ±3.67 % wall / −0.17…+0.50 % CPU**, so base
classic-off is 3.3x that band, base classic-on only 1.1x (disclosed; 6/6 pairs
candidate-faster), and benchy classic-off's −1.77 % sits **inside** it — reported
as the ticket required, not claimed. **Route:** base classic-off **2.078x →
1.827x**, cut 51.88 % → 45.27 %; base classic-on 3.356x → 3.220x; the remaining
six move 1.77–9.37 % of their ratios of record. A tranche of t142's order, not
a destination step, and the 8/8 destination is unchanged. Two ticket
corrections, both tree-verified: the whole-print-input guest keeps `bridges=1`
because its surface product is a **literal** (`Product::Literal`), not
mesh-analysis output, and `algo_mesh_analysis_tdd` had no bridge assertions —
the eight retired cases are all in `bridge_detector_tdd.rs`. Disclosed: no
accelerated timing arm (`libmimalloc-sys`, unchanged since t142), one box and
one window, box load not measured. Evidence:
[`evidence/t150-meshanalysis-retirement/`](evidence/t150-meshanalysis-retirement/FINDINGS.md)
(`verify.py` 71 checks, 5/5 negative controls). **Frontier:**
[t151](issues/151-skirt-hull-brim-contour.md) (AFK, unblocked) only.

**Destination status (2026-11-14,
[t151](issues/151-skirt-hull-brim-contour.md)): the skirt hull now folds the
canonical brim-area contour, the last supports-off oracle miss closes, and the
frontier is EMPTY — the map returns to the human for the next shape.** The
module extends `brim_points` from the brim **area's outer contour**
(`area[i].contour`, the `outer − inner [− support]` result already in hand),
matching canonical `Print::_make_skirt`'s `m_objectBrimAreasByInstance[...]`
`area.contour.points`, and no longer from the emitted beads' `path.points`. The
measured radial move is the canonical half flow spacing: **0.178–0.180 mm** on
every moving cell, **brim bbox identical in all six**, and the brim→skirt step
(`max_x(Skirt) − max_x(Brim)`) closes **4.962–4.965 → 5.140–5.144 mm** against
t146's Orca **5.139 mm** and **5.143 mm** prediction. **Output:** the baseline
reproduces t148's set in **8/8**; the candidate moves **6/8** and is
byte-identical across the ordinary and accelerated arms in **8/8**. **The
pre-declared "all eight move" is falsified and disclosed:** the two base
supports-on cells cannot move because the first-layer support footprint clips the
whole brim area (`area` empty ⇒ `brim_points` empty), and the delta audit proves
inertness line-by-line — **zero moved sections, zero changed `E` lines** there.
**Delta audit:** `Skirt` is the only moved section in the six moving cells and
every motion line outside it is identical with `E` stripped in 8/8; the literal
"`|dE|` outside `Skirt` is zero" clause is **falsified at ≤ 0.00098 mm on 3–35 of
up to 508,651 lines** (the accumulator re-rounding t144 measured at the same
magnitude), disclosed rather than waived. **Oracle (four supports-off cells):
benchy classic-off 1.32 → 0.05 %, benchy arachne-off 1.34 → 0.04 %, base
classic-off 2.89 → 0.37 %, base arachne-off 3.45 (MISS) → 0.94 % — 4/4, up from
3/4** (4/8 overall); brim rel-diffs unchanged in all four, and the residual is
now the ~0.10 mm brim-extent term plus ≤ 0.04 mm Arachne width, exactly t146's
decomposition. **A/B (attribution only):** 3 interleaved pairs × 8 cells, every
median (−4.70 … +6.43 % wall) inside the **measured** same-arm null band
(**±12.86 % wall / ±6.98 % CPU**), pair spread to +49.77 %, absolute walls ~2x the
output campaign's — **no wall claim** (the surface is t103's ~235 ms). **The new
16-cell set is the currency** ([`reference.json`](evidence/t151-skirt-hull-contour/reference.json));
t148's set is superseded but reproduced 8/8 as the control. No route term moves:
this is a fairness fix, and the 8/8 destination and every ratio of record stand
with their disclosures. **Drive-by, not caused:** `integrated_parity_skirt_brim_tdd`
(t142's pre-existing red) omitted two declared keys and now authors them at their
registry defaults — it passes and compares native against wasm finalization on
the changed module. Evidence:
[`evidence/t151-skirt-hull-contour/`](evidence/t151-skirt-hull-contour/FINDINGS.md)
(`verify.py` PASS, 16/16 mutations rejected; `audit_delta.py` self-checked;
`reduce_t151.py` regenerates the summary). **Frontier: none** — 1–151 are
resolved; the next shape is the human's choice from the fog (below), and the
largest unpriced members remain the sweep-volume work-set family's **canonical
loop projection** (t129's S3, 1,587 ms ceiling, 5.7 % of base-off's required cut)
and the per-layer program on the benchy cells.

**Destination status (2026-11-14,
[t152](issues/152-seam-inset-offset-redundancy.md)): S3 is not a clean
deletion; the prepass seam inset offset is load-bearing, and the frontier is
the human's per-layer relocation ruling.** Deleting
`offset_polygons_batch` + `project_point_onto_inset_boundary` from
`run_region_planning_entries` (`modules/core-modules/seam-planner-default/src/lib.rs`)
and emitting the raw chosen position moves **456 of 496** base-classic-off
layers (+1,177 / −1,149 printed points, 142 layers with a changed total `E`),
with the first difference in the layer-0 **Skirt**; the candidate hash is
`4265d619…` against the t151 reference `9bb18f0e…` (+1,158 bytes / +28 lines).
The module's own unit tests falsify it: **31/0 baseline vs 29/2 candidate**
(`batched_inset_offsets_match_the_serial_reference_per_region`,
`candidate_less_region_still_emits_one_keyed_plan_entry`). The offset term is
**1,248 ms** (`PrePass::SeamPlanning` **1,779 → 531 ms**, **70.15 %** of the
stage), not t129's 1,587 ms / 92.4 %; it is **5.9x** the 0.80 % floor on the
26.254 s single-run cell and **~10.5 %** of base classic-off's current required
cut. **The canonical replacement is a per-layer relocation**, not a prepass
deletion: canonical `SeamPlacer::collect_points` builds candidates from the
printed external-perimeter loop points, which exist only after per-layer
perimeter generation, so matching it means moving candidate generation and
scoring into the per-layer phase — a geometry change there that needs its own
t81-style exception. The net is the 1,248 ms minus the moved 531 ms residual and
any loop-collection cost; unmeasured. **Frontier:**
[153](issues/153-seam-loop-relocation-ruling.md) (HITL, unblocked — authorize,
decline, or re-scope the relocation). Evidence:
[`evidence/t152-seam-inset-offset/`](evidence/t152-seam-inset-offset/FINDINGS.md)
(`verify.py` PASS, 4/4 mutations rejected).

**Destination status (2026-11-14,
[t153](issues/153-seam-loop-relocation-ruling.md)): the relocation is DECLINED
— structurally impossible and already decided — and the 1,248 ms seam offset
stays in the fog with its blocker.** The human's instruction was to verify the
prior attempt before trying it, and the record is decisive. **ADR-0046**
(status accepted) asks exactly this question and rejects the per-layer
`seam-placer` as impossible, because per-layer modules are re-instantiated per
call and run in parallel across layers (ADR-0045), so a per-layer module can
never see two layers and can never chain anything. **Packet 168** discovered
the constraint during grounding and recorded it as an architecture constraint,
not a preference. The ported candidate builder settles the rest:
`build_seam_candidates`
(`modules/core-modules/seam-planner-default/src/visibility.rs`) scores **every
contour vertex of every layer** and its per-layer terms read the **previous**
layer's contours (`prev_dist`) as well as the current layer's (`curr_dist`), so
candidate generation and scoring — not just the alignment chaining — are
cross-layer. The only sanctioned conduit is one-way and prepass-first (ADR-0020:
`SeamPlanIR` → host-injected `resolved_seam`), so moving generation onto the
printed loops needs a two-phase pipeline (all per-layer perimeters before seam
planning) or a host-orchestrated seam pass; both are architecture work past
this destination's route, and ADR-0046's alternatives already rejected the host
path. **S3 closes as "not a clean deletion and not relocatable".** No exception
is spent and nothing moves: `nearest` / `rear` / `random` could in principle
generate per layer (no chaining), but the matched job is `aligned`, so that
buys the destination nothing. No production edit, no measurement, no
reference-set movement. **Frontier: none** — 1–153 are resolved; the next shape
is the human's pick from the fog (the per-layer program on the benchy cells
remains the largest unpriced member).

**Destination status (2026-11-14,
[t154](issues/154-benchy-per-layer-carrier-attribution.md)): the benchy
per-layer program is margin, not route; the per-layer requirement that remains
is base classic-on's 1.096 s, and the frontier is the Infill-stage block.** At
HEAD the split is prepass **63.5–82.4 %**, per-layer **14.4–29.6 %**, postpass
**2.6–6.5 %**, outside **0.1–5.3 %**, with a per-layer host gap of **0–56 ms
(≤ 0.65 %)** in every cell. Substituting canonical's measured prepass cost (t110)
and holding every other phase at its measured share **wins 7 of 8 cells**:
benchy arachne-off **0.668x**, base arachne-off **0.698x**, benchy classic-off
**0.740x**, base classic-off **0.743x**, benchy classic-on **0.857x**, benchy
arachne-on **0.901x**, base arachne-on **0.967x** — and **base classic-on
1.051x**, needing **1.096 s** (9.6 % of its 11.46 s per-layer phase). The
required prepass cut is **64.3–84.5 %**, largest on benchy arachne-on (84.5 %),
base classic-on (83.7 %) and base arachne-on (82.5 %), **not** on benchy
classic-off. **This supersedes t128/t129's "prepass at canonical cost loses both
benchy cells (1.053x / 1.075x)"**, which paired t126's per-layer phases with
t124's Orca walls; the benchy cells are no longer "part of the first program" on
that arithmetic. **Carriers:** supports-off is carried by the perimeters module
(**46.8–55.1 %** of per-layer thread, 77–98 % in-module self fuel), the first
`Layer::Infill` dispatch (**17.6–37.3 %**, stage-shared; t127's
"prepared-region view" name cannot hold at HEAD because
`Layer::Perimeters` materialises that view first through
`ensure_prepared_perimeter_source_regions` and `Layer::Infill` hits the memo)
and `infill-linker` (**5.3–20.5 %**, 99.8–100 % self fuel); supports-on by
`tree-support` (**36.7–47.8 %**), which is a **work-set/phase difference**
against canonical — `PrintObject::_generate_support_material` →
`TreeSupport::generate` is one whole-object pass, so PnP pays support planning
and then a per-layer render. The per-op split does not exist at the current
instrumentation surface (named scopes explain ≤ 22.8 % of a carrier's fuel), and
that limit is the finding: there is no plumbing fat inside the carriers. The
t126→t154 per-layer fall (**−21.6 % to −40.1 %** on the benchy cells) is measured
but **not attributed**. **Frontier:**
[t155](issues/155-infill-stage-block-base-on-residual.md) (AFK, unblocked, new)
— identify the first-`Layer::Infill` block (p50 **63 ms**/layer on base
classic-off against its own nil work) and price it against base classic-on's
1.096 s. Evidence:
[`evidence/t154-benchy-per-layer/`](evidence/t154-benchy-per-layer/FINDINGS.md)
(`summary.json` 8 cells, `arithmetic.json`, `carriers.json`, eight reduced
`profile-*.json`; `verify.py` PASS, 11/11 negative controls).

**Destination status (2026-11-14,
[t155](issues/155-infill-stage-block-base-on-residual.md)): the first
`Layer::Infill` block is the prepared-region derivation, paid twice per layer,
and the second payment is redundant with a 2.33 s ceiling on the one cell the
destination still needs; the frontier is the candidate
[156](issues/156-prepared-memo-across-fill-sync.md).** The block is
`prepare_dispatch_region_views` → `prepare_slice_regions` →
`derive_needs_support`, **97.5 %** of the first-Infill bracket (p50 52.4 ms of
53.6 ms on base classic-off) and **97.3 %** on base classic-on (p50 53.3 ms of
54.8 ms). **Marshalling, the instance pool and `LayerStageInput` are falsified**
at ~0.95 ms total (1.8 %): pool lease 5 µs, linker 196, component instantiation
83, region push 58, typed export 593, teardown 25. **The arena memo works**:
every non-first module in a stage reads the prepared slots in 5 µs (3.0–4.0 ms
per module over 496 calls). **The block is paid twice** because
`sync_perimeter_infill_areas_into_slice` takes and re-stages the slice after
every perimeter commit (three per layer: once after `Layer::Perimeters`, once
per `PerimetersPostProcess` commit), and each `set_slice` clears
`clear_prepared_regions`. **The second derivation is provably redundant:** the
sync was audited by hashing the fields `prepare_regions` reads (region order and
count, `region_id`, `object_id` length, `nonplanar_surface` presence, every
polygon coordinate) before and after — **0 of 1,488 syncs changed any of them**
on base-off, base-on and the decline arm. **Stage-sharing re-proven at HEAD:**
declining `raft-default` moves the whole block to `gyroid-infill`
(**24,533.1 ms**, p50 50.0 ms), every other Infill module stays under 3.5 ms, and
the hash does not move (`9bb18f0e…`). **`Layer::Support` is not a host block:**
`tree-support`'s `prep_views` is 5 µs; its 85.9 ms p50 bracket on base-on is the
in-module render. **Inside the derivation, 94.3–94.4 % is one
`intersection_ex(region.polygons, footprint)` per layer** — the object has
exactly **one** non-empty overhang footprint on every layer (max 1 over 1,488
calls) and the answer was **true in every measured call**; base-on evaluates it a
third time in the prepass eligibility pass (1,488 calls, 72.8 s thread). **Price
(ceilings = thread / t154 parallelism; walls of record carried):** redundant
Infill-side derivation **25.99 s / 26.10 s** thread, **2.382 s** base-off
(**12.15x** the 196 ms floor, 21.5 % of the 11.10 s cut) and **2.335 s** base-on
(**4.20x** the 556 ms floor, **2.13x** the 1.096 s requirement); indicatively
base-on goes **1.051x → ~0.943x** at canonical prepass cost, a chain not a
claim. **Verdict:** removable — a proposed candidate (preserve the memo across
the fill sync, clear on the modifier-footprint path) with its ceiling, **not an
adopted change**; the standing eight-cell gate is graduated as
[156](issues/156-prepared-memo-across-fill-sync.md). **Disclosed:** no wall claim
from any probe or instrumented pass; no ratio of record; box load not sampled;
no accelerated arm; the pre-probe pass's 63 ms p50 against the later 52.4 ms is
a slower box state across every phase, not attributed; no design for a cheaper
`needs_support` predicate (its 94 % share is measured, its replacement is not).
Evidence: [`evidence/t155-infill-stage-block/`](evidence/t155-infill-stage-block/FINDINGS.md)
(`probe.json` eight runs, `summary.json`, `probe.patch` reverted and
`git apply --check` clean, `verify.py` PASS 9/9, `--self-test` 9/9 mutations
rejected; restored tree `cargo xtask build-guests --check` exit 0 and the
restored binary reproduces `9bb18f0e…`). **Destination status (2026-11-15,
[156](issues/156-prepared-memo-across-fill-sync.md)): the prepared-region memo
candidate is exact and CPU-lower in all eight cells, but it is a DROP — the
standing wall gate fails on benchy Arachne-on (−2.94 %) and base Arachne-on
(+0.17 %), and the production change is reverted.** The candidate preserved
`LayerArena`'s prepared-region memo across
`sync_perimeter_infill_areas_into_slice` (fill writes do not touch any input
`prepare_regions` reads; the modifier-footprint path still cleared), and output
was byte-identical in 8/8 to t151's active set. Five interleaved pairs per cell
on a quiet window gave **+2.15 / +3.95 / +3.24 / −2.94 %** on the benchy cells
and **+3.91 / +4.87 / +10.09 / +0.17 %** on the base cells; CPU medians were
lower in all eight (e.g. base classic-off 206.4 → 183.0 s) but the supports-on
Arachne walls did not follow the removed derivation. The same-arm null band is
**9.52 % wall / 0.75 % CPU any-pair absolute**; only base Arachne-off's wall
effect is outside the wall band, so most passing wall numbers are small against
the instrument. The 3-pair first pass was extended only after disclosure; an
85 %-load extension was invalidated and preserved in
[`evidence/t156-prepared-memo/reduced/loaded-window.json`](evidence/t156-prepared-memo/reduced/loaded-window.json).
**Frontier: none** — 1–156 are resolved; the next shape is a human pick from the
fog.

**Destination status (2026-11-16,
[157](issues/157-needs-support-predicate-attribution.md)): `commit_support_analysis_builtin`
is the sole production decision consumer of `derive_needs_support`, and an exact
fast-path boolean predicate inside `SliceRegionView::derive_needs_support`
collapses all three call sites (`PrePass::SupportAnalysis`, `Layer::Perimeters`,
`Layer::Infill`) by 91.0x–120.0x with 0 / 3,680 mismatches; the frontier is the
candidate [158](issues/158-needs-support-fast-predicate.md).** Tracing past
`__slicer_adapt_slice_regions` (`crates/slicer-macros/src/lib.rs`) to
`SliceRegionView::needs_support` (`crates/slicer-sdk/src/views.rs`) across the
entire workspace proves **zero production guest modules** read
`region.needs_support()`: Packet 224 retired `region.needs_support()` from
`TreeSupport::run_support` (`modules/core-modules/tree-support/src/lib.rs`)
because `PrePass::SupportAnalysis` already gates auto-support contact detection
on `needs_support` when producing `SupportAnalysisIR`, and
`TraditionalSupport::run_support` never read it. On `3dbenchy.stl` (`240`
layers, `a_verts_mean = 1,890.1`, `b_polys = 201`, `b_verts = 6,107`),
`derive_needs_support` costs **3.24–3.39 ms/call** (`1,571.3 ms` thread across 2
sites on `benchy classic-off`; `2,403.2 ms` across 3 sites on `benchy
classic-on`), explaining why `t156` (removing only 1 of 3 sites) was sub-noise
on `benchy`; on `base.stl` (`496` layers, `a_verts_mean = 5,155.7`, `b_polys =
994`, `b_verts = 26,633`), it costs **49.4–60.9 ms/call** (`51,651.6 ms` across 2
sites on `base classic-off`; `84,792.0 ms` across 3 sites on `base classic-on`).
An exact boolean fast path inside `SliceRegionView::derive_needs_support`
(strict bbox overlap prefilter + strict-interior point-in-`ExPolygon` witness in
exact `i128` arithmetic, with `intersection_ex` fallback on overlapping pairs
when no witness is found) produces **0 / 3,680** mismatches across all 4 probed
cells (`35.7–37.0 µs/call` on `benchy`, **91.0x–92.5x** speedup; `410.2–561.6
µs/call` on `base`, **113.0x–120.0x** speedup) and **8/8** byte-identical G-code
runs against t151's active set. Under `PNP_T157_FAST_PREDICATE=1`, combined
`prep_views_ms` (`Layer::Perimeters` + `Layer::Infill`) drops **−76.0 % to
−77.1 %** on `benchy` and **−93.0 % to −93.8 %** on `base` (`64,875.1 → 4,011.5
ms` on `base classic-on`), while `PrePass::SupportAnalysis` eligibility wall on
`base classic-on` drops **2,100.5 → 41.5 ms (50.6x)**. Priced against `t154`'s
ledger (`ESTIMATE`, not a ratio of record): **230.6 ms** (`benchy classic-off`,
**6.00 %** of wall, **7.50x** floor, `2.0068x → 1.8864x`), **237.3 ms** (`benchy
classic-on`, **3.78 %** of wall, **4.73x** floor, `2.5711x → 2.4739x`),
**4,692.5 ms** (`base classic-off`, **19.15 %** of wall, **23.94x** floor,
`1.8270x → 1.4772x`), and **7,414.4 ms** (`base classic-on`, **10.66 %** of
wall, **13.33x** floor, `3.2197x → 2.8765x`). On `base classic-on`, the **5.346
s** per-layer wall-equivalent saving alone is **4.88x** `t154`'s **1.096 s**
non-prepass residual requirement, taking its canonical-prepass projection from
`1.0508x` to **`0.8033x` (`17.349 s vs 21.598 s`)**. Probe reverted,
`probe.patch` preserved, `cargo xtask build-guests --check` exits 0, and
[158](issues/158-needs-support-fast-predicate.md) is graduated. Evidence:
[`evidence/t157-needs-support-predicate/`](evidence/t157-needs-support-predicate/FINDINGS.md).

**Destination status (2026-11-16; human-amended 2026-11-17,
[158](issues/158-needs-support-fast-predicate.md)): the exact fast-path boolean
predicate in `SliceRegionView::derive_needs_support` is KEEP by the human's
explicit ruling and is landed.** It is byte-identical in **8/8**, lowers process
CPU by **5.88 %–21.38 %** in **8/8**, and passes all four `base` cells at
**6.78x–18.58x** the floor (saving **4.23–6.37 s** wall, **20/20** pairs
candidate-faster). The original mechanical gate remained DROP because
`benchy-classic-on` recorded **+0.766 %** against the 0.80 % floor and
`benchy-arachne-off` recorded **−0.076 %**; the human reviewed the complete
evidence and ruled **“I accept it as a keep.”** The ruling changes the
disposition, not the measurements.

Landing `expolygons_intersect_area_positive` inside
`SliceRegionView::derive_needs_support` (`crates/slicer-sdk/src/views.rs`)
reproduces `t151`'s active reference set in **8/8** cells across 16 output runs,
80 A/B runs, and 18 null runs (`exit 0`, `degraded=false`, zero diagnostics).
Over 5 interleaved pairs per cell, process CPU medians fall in **8/8** cells
(**5.3x–19.1x** the worst-case single-pair CPU null band of `1.117 %`; `39/40`
pairs candidate-lower). `base-classic-off` saves **4.249 s** (**+14.87 %**),
`base-classic-on` **4.676 s** (**+5.42 %**), `base-arachne-off` **4.233 s**
(**+14.64 %**), and `base-arachne-on` **6.369 s** (**+7.52 %**). On the two
mechanically failing `benchy` cells, median within-pair savings are positive
(**+2.69 %**, `4/5` wins; **+3.16 %**, `3/5` wins), but the null control covered
only three `base` cells, so no `benchy` null-band claim is made.

The indicative post-t158 ratio composition is **1.9122x / 2.5514x / 1.7641x /
2.6482x** on `benchy` and **1.5554x / 3.0452x / 1.5349x / 2.8302x** on `base`
(classic-off / classic-on / arachne-off / arachne-on). Subtracting the measured
savings from t154's canonical-prepass projections makes **8/8** projected cells
win, with `base-classic-on` **1.0508x → 0.8343x**. These are cross-window
compositions, not a fresh Orca scoreboard or a measured destination close.
Evidence and the human ruling:
[`evidence/t158-needs-support-fast-predicate/`](evidence/t158-needs-support-fast-predicate/FINDINGS.md)
(original campaign `verify.py --self-test` exit 0, 6/6 negative controls rejected;
route arithmetic re-derived by `route_arithmetic.py`). **Frontier: none** —
1–158 are resolved; the next shape is the human's pick from the fog.

**Destination status (2026-11-17,
[159](issues/159-tree-support-render-prepass-fold.md)): the tree-support fold
is ruled an EXPERIMENT, not adoption; the frontier is
[160](issues/160-tree-support-prepass-batch-probe.md).** The human picked fog
item (a) and ruled to measure whether the planner/renderer structure costs wall.
Canonical `TreeSupport::generate` (`TreeSupport.cpp`) has the same two halves
(`draw_circles`, then a per-layer `tbb::parallel_for` in `generate_toolpaths`),
so t154's "pays support work twice" is corrected to a **phase-placement**
difference. t160 holds the work constant: an env-gated host probe runs the
unchanged `tree-support` guest for all layers in one whole-object pass after
`PrePass::SupportGeometry`, plus a null-renderer ceiling arm, judged per
supports-on cell (hurts = ≥ 0.80 % floor and outside the null band), with
canonical `generate_toolpaths` timing as the per-op control. Probe only;
adoption is a separate ruling. **Frontier:**
[160](issues/160-tree-support-prepass-batch-probe.md) (AFK, unblocked).

**Destination status (2026-11-17,
[160](issues/160-tree-support-prepass-batch-probe.md)): the fold is a pure
placement move, output is byte-identical 8/8, and the batch arm is slower in
every supports-on cell — the per-layer structure is exonerated; the frontier is
empty (1–160 resolved) and the next shape is a human pick from the fog.** The
read-set digest (`plan` / `view` / `paint`) matches at both dispatch points on
**every layer of all eight cells** (496/496 base, 240/240 benchy per side), so
the renderer reads only prepass-committed state and t160's item-3 stop does not
fire. The batch arm reproduces t151's active set in **8/8**, all runs clean.
Paired campaign (5 interleaved pairs/cell, ordinary): the batch arm is slower
in all eight separate-arm medians, the four supports-on cells at **−1.69 % /
−1.71 % / −3.13 % / −3.77 %** (pair wins 1/5, 2/5, 0/5, 1/5), with the batch
pass's own wall **3.9–4.2 s** on the base cells and **0.49 s** on the benchy
cells, charged inside the per-layer phase bracket. The same-arm null band is
**9.50 % wall any-pair / 1.91 % median** (CPU 5.89 %) — the deltas sit inside
it, and no cell is faster, so the null control cannot make any cell "hurts".
The null-renderer ceiling is **9.16–12.61 %** of cell wall: the renderer's
cost is the work, not its placement, so a scheduling change cannot recover it.
Supports-off cells are skipped when `enable_support=false` (no batch dispatch,
output identical; their ±1.2–4.3 % wall movement is disclosed box drift).
Probe reverted, `probe.patch` preserved, `build-guests --check` exit 0. Evidence:
[`evidence/t160-tree-support-batch/`](evidence/t160-tree-support-batch/FINDINGS.md)
(`verify.py` PASS, `--self-test` 9/9). **Frontier: none** — 1–160 resolved.

**Destination status (2026-11-17,
[161](issues/161-post-t158-scoreboard-refresh.md)): the ratios of record are
re-measured on the current tree in one window — 1.366x–2.718x, 0 of 8 won — and
the board records that the Orca arm moved too, so required cuts are no longer
comparable across windows.** One campaign, 8 cells × {Orca, PnP ordinary},
1 warmup + 3 measured runs per cell, interleaved, matched keys asserted on all 24
Orca runs, PnP output **8/8 identical** to t151's active set (`exit 0`,
`degraded: false`, zero exact-Z diagnostics). **benchy 1.796x / 2.671x / 1.794x /
2.591x** (4.076 / 8.144 / 3.790 / 7.337 s against 2.269 / 3.050 / 2.113 /
2.831 s), **base 1.429x / 2.718x / 1.366x / 2.622x** (22.603 / 70.754 / 22.505 /
71.377 s against 15.812 / 26.030 / 16.474 / 27.219 s); required cut **26.80 %–
63.21 %**, and the two base supports-off cells carry a **negative 1.5x cut**
(−4.94 % / −9.80 %). **The tightest cells are now the base supports-off pair**,
where t145's were the benchy supports-off pair. **The instrument changed:** the
installed OrcaSlicer is **2.5.1** (binaries dated 2026-11-13 23:59–11-14 00:00,
`orca-slicer.exe` `5013b1bd…`), not the **2.5.0** the canonical-source pin
records; the vendor cache stamp is unchanged (`2.08.00.16`), so only the
executable moved. Orca measured **+5.4…+24.9 %** above t145's medians in
**8/8** cells while its **CPU** moved only **−2.2…+11.7 %**, and its effective
parallelism (CPU/wall) fell in **8/8** (−2.2…−17.4 %) while PnP's did not — so
the movement is efficiency, not work, and the binary is the leading explanation
(2.5.0 is uninstalled, so box state cannot be excluded; benchy classic-off's
4,077,463 Orca bytes are t145's exact figure, so at least that job is
unchanged). So the board **supersedes t145's and t148's as the ratios of
record** — they are 2.5.0 numbers — and the tracker may not carry a required cut
across the tool boundary or across windows. Two readings survive the movement: the two benchy supports-on cells moved
**+24.3 % / +24.9 %** on both arms while their ratio moved **−0.5 % / +0.4 %**
(the paired ratio absorbs a ~25 % box shift), and the base supports-off
improvement is independently corroborated (the landed deltas predict −25.2 % /
−22.6 % of t145's PnP wall; measured −18.9 % / −24.0 %). **benchy classic-off's
−12.1 % ratio movement is Orca-side** (PnP rose +4.2 %), so it is not banked as
engine work. Prepass phase shares (63.5–82.4 %) and the route projection remain
**pre-t158** and are not re-derived here. Disclosed: no accelerated arm — the
ordinary-only driver skips the accelerated gate, and checking that tree found
**45 stale guests** (the whole accelerated namespace, pre-existing); they were
rebuilt after the campaign (`--accelerated --check` now exits 0, ordinary too)
and `dist --accelerated` still fails at exit 1 in `libmimalloc-sys` (`C1083` on
`'\\mimalloc-static.cc'`, the t142 MSVC-14.51 verbatim-path failure), so
accelerated output parity is not re-verified here. **Corrected the same day by
[162](issues/162-accelerated-host-verbatim-path.md): that failure was a verbatim
`CARGO_TARGET_DIR`, not the toolchain, and the accelerated arm builds again.**
Also: one Orca outlier of
29.807 s kept in the record, load 62 samples
(2 % / 69.5 % / 100 %), `tree_strong` pin. Evidence:
[`evidence/t161-scoreboard-refresh/`](evidence/t161-scoreboard-refresh/FINDINGS.md)
(`verify.py` PASS 226/226, `--self-test` 16/16). **Frontier: none** — 1–161
resolved; the next shape is a human pick from the fog, and the board's own
consequence is that the phase split must be re-attributed before any further
route arithmetic.

**Destination status (2026-11-18,
[162](issues/162-accelerated-host-verbatim-path.md)): the accelerated host build
is fixed and the arm is available; no timing claim is taken.** The `c1xx C1083`
that closed every accelerated arm since t142 was a **verbatim `CARGO_TARGET_DIR`**
— `workspace_root()`'s `canonicalize()` spelling rode into every build script's
`OUT_DIR` and into `cl.exe`'s argument vector, where MSVC read
`-c \\?\D:\...\mimalloc-static.cc` as `\\mimalloc-static.cc`. The wrapper,
MSVC 14.51 and `libmimalloc-sys` were never involved. `dist --accelerated` now
exits 0 with 1 binary + 24 modules, and **8 cells × 2 arms are output-identical
16/16** to t151's active set. The manual-cfg prohibition in `docs/23` stands and
was never the obstacle. **Frontier: none** — 1–162 resolved; the obvious next
shape is the accelerated arm on the eight cells, which the map has not measured
since the arm broke.

**Destination status (2026-11-18,
[163](issues/163-accelerated-timing-board.md)): the accelerated arm is measured
on all eight cells and it is not a route lever; the ordinary arm's arithmetic
stands.** One window, three tools, 8 cells × 1 warmup + 3 measured runs: both
PnP arms are **byte-identical to t151's active set in 8/8**, and the accelerated
arm's wall effect is **−13.2…+4.4 %** (median cell **−1.1 %**, mean −1.9 %)
against per-cell spreads of 2.9–21.6 %, so only `benchy arachne-off` (**−13.2 %**)
and `base classic-off` (**+4.4 %**) exceed their own spread. Its **process CPU**
is the tight channel: lower in **6/8** (−0.8…−4.3 %). **0 of 8 cells are won on
either arm** (required cut 28.0–64.7 % ordinary / 29.9–64.3 % accelerated), and
**on the tightest cell the arm is worse** — `base classic-off` 1.389x ordinary
against 1.450x accelerated. The take also gave an unplanned cross-window
control: against t161 both arms moved together and the paired ratio barely
moved (benchy classic-off −0.8 % for 3.5–4.3 % wall movement, base classic-off
−2.8 % for 11.8–15.1 %), independently confirming t161's method claim. **The
accelerated arm is now available as corroboration for any candidate** and should
be checked for every future candidate, since it can move a cell by −13…+4 %.
**Frontier: none** — 1–163 resolved.

**Destination status (2026-11-18,
[164](issues/164-phase-re-attribution.md)): the split is re-attributed on one
window, the prepass now owns 67.0–87.2 % of every cell, and at canonical prepass
cost **all eight cells win (0.396x–0.890x) — but the two benchy supports-on cells
are the binding ones and they rest on a 1.53x/1.59x slack in canonical's own
prepass cost.** One window, two tools (OrcaSlicer 2.5.1 `5013b1bd…`, PnP ordinary
`2c44d38f…` at HEAD `c83d253b`), 8 cells × 1 warmup + 3 measured, 64 rows,
matched keys asserted on all 24 Orca runs, **8/8 output identity** to t151's set,
every run clean; plus **24 instrumented passes** (3 per cell) also **24/24**
identical. Load 99 samples, min 6.8 % / median 66.6 % (45.9 % during the scoreboard window) / max 100 %. **Split at
HEAD: prepass 67.0–87.2 % (was 63.5–82.4 %), per-layer 9.3–26.3 % (was
14.4–29.6 %), postpass 3.2–6.8 %, outside 0.1–2.3 %**, repeat spread 0.72–3.91 pp,
per-layer host gap 0–1 ms. The prepass share **rose on all eight cells**: base
classic-off **+14.85 pp** and base arachne-off **+11.51 pp** (t158's predicate took
**4.25 / 4.23 s** out of the per-layer phase of 22 s cells), base supports-on
**+4.81 / +5.23 pp** (4.68 / 6.37 s against 69–71 s walls), benchy **+1.07…+6.00
pp**. `MeshAnalysis` is 687–697 ms base / 86 ms benchy; the prepass brackets are
`host:shell_classification` (11.2 s, 68.4 % of base-off's prepass),
`SupportGeometry` (34.9 s, 54.9 % of base-on's) and `SupportAnalysis` (12.7 s,
20.0 %). **Board: 1.379x–2.884x, 0/8 won, required cut 27.5 %–65.3 %** — the same
order as t163's 24-hour-old board, cited as corroboration and never composed.
**Arithmetic (estimate): benchy 0.611x / 0.874x / 0.649x / 0.890x, base 0.400x /
0.786x / 0.396x / 0.718x; no cell needs anything from the non-prepass phases.** The
binding cell **moves from `base classic-on`** (t154's 1.051x, needing 1.096 s from
the per-layer phase) **to `benchy arachne-on` at 0.890x**; `base classic-on` is now
**0.786x** because t158 removed 4.68 s of per-layer wall there. The constraint is
the **input**, not the result: the four supports-on cells have 1.50x–1.75x slack
between their prepass budget and canonical's prepass cost, and the benchy
supports-on cells have the least (0.840 s and 0.874 s of budget against 0.550 s);
the substitution implies a prepass cut of 82.6 %–98.1 %. Disclosed: an instrumented
wall is never a wall claim (shares are multiplied by the scoreboard wall, never
substituted), the shares and walls are one window, the single external input is
t110's tap value held fixed, and no null band is claimed because nothing was
adopted. Graduated [165](issues/165-canonical-prepass-cost.md) to verify that input
on 2.5.1. Evidence:
[`evidence/t164-phase-re-attribution/`](evidence/t164-phase-re-attribution/FINDINGS.md)
(`verify.py` PASS, every number re-derived; `--self-test` 16/16).
**Frontier:** [165](issues/165-canonical-prepass-cost.md) (AFK, unblocked, new).

**Destination status (2026-11-18, re-route session; no ticket resolved): one
untested premise and one parked parity tranche, neither of which changes the
route arithmetic yet.** t164's 8/8 requires an **82.6–98.1 %** prepass cut, and
nothing below touches the supports-on cells' `SupportGeometry` /
`SupportAnalysis` terms, which carry the binding cells. Canonical code facts
below were read from the local `OrcaSlicerDocumented` checkout
(`3e1daccd7c`, a nightly), **not** the v2.5.0 pin or the 2.5.1 binary, so they
must be re-verified at the pin. The operand-volume program (t129 → t131)
reads its gap as operand volume because "the kernel is at parity" (t70). But
t69/t70's twin was **C++ Clipper2**. Canonical's hot path is
`ClipperUtils.cpp` on **ClipperLib (Clipper1)**, with per-contour
`offset_expolygon_inner` and `ShortestEdgeLength` decimation, and canonical's
operand sizes were never measured. Separately, the fog's Pass 1 `apply_opening`
lead (t27, "small prize") was priced by an env-gated probe on base
classic-off. PnP opens top/bottom diffs at radius `line_width / 2` with
**Round** joins, while canonical `detect_surfaces_type` uses
`external_perimeter_width / 10` with **jtMiter/3**. The opening is **53.4 /
59.3 s** of thread-time (974 calls, about half of the stage), and canonical
parameters cut it to **14.3 / 12.6 s**, with the Pass 1+2 wall **5.4–5.7 →
2.7–3.0 s**. The cell-level effect is not established: n = 2, instrumented,
and downstream work grows as more canonical solid area survives. The erosion
deletes 95.4 % of its 5.2 M input points at 10–11 µs per point, so the cost is
operand *shape* (sliver inversion), not volume. Evidence:
[`evidence/t166-shell-opening-parity/`](evidence/t166-shell-opening-parity/FINDINGS.md).
Graduated [166](issues/166-shell-opening-canonical-parity.md) (the parity
port + 8-cell campaign) and
[167](issues/167-canonical-clipperutils-replay.md) (replay PnP's operands
through canonical's own ClipperUtils, which tests the premise the whole route
rests on). **Frontier:** [165](issues/165-canonical-prepass-cost.md),
[166](issues/166-shell-opening-canonical-parity.md),
[167](issues/167-canonical-clipperutils-replay.md) (all AFK, unblocked,
independent).

**Destination status (2026-11-19,
[165](issues/165-canonical-prepass-cost.md)): t164's input is verified on
2.5.1, so the 8/8 projection stands; the frontier is 166 and 167.** The re-run
tap (OrcaSlicer 2.5.1 `5013b1bd…`, current matched job asserted on every run,
tap and `GetProcessTimes` agreeing within 0.861 %) gives canonical prepass
**104.7 ms (bound) / 569.7 / 2,172.4 / 10,368.1 ms** for benchy-off / benchy-on
/ base-off / base-on, against t110's 50.5 (bound) / 549.8 / 2,103.0 /
10,444.8 ms. At canonical prepass cost **8/8 still win (0.400x–0.898x)**. The
binding cell is **`benchy arachne-on` at 0.898x** (was 0.890x), slack **1.47x**
(was 1.53x). So the route's requirement is unchanged: an **82.6–98.1 %**
prepass cut. A side observation, not a claim: this window's Orca walls are
within −1.3…+0.9 % of t110's 2.5.0 walls, which weakens t161's reading that the
2.5.1 binary caused Orca's movement. Evidence:
[`evidence/t165-canonical-prepass-cost/`](evidence/t165-canonical-prepass-cost/FINDINGS.md)
(`verify.py` PASS, `--self-test` 11/11). **Frontier:**
[166](issues/166-shell-opening-canonical-parity.md),
[167](issues/167-canonical-clipperutils-replay.md).

**Destination status (2026-11-19,
[166](issues/166-shell-opening-canonical-parity.md)): the top/bottom opening is
canonical and the benchy cluster gains 8-21 % wall, but the change is a benchy
lever and the speed claim is DEGRADED; the frontier is
[167](issues/167-canonical-clipperutils-replay.md) and the new
[168](issues/168-bridge-opening-auto-sentinel.md).** Pass 1 now opens its
neighbour diff with canonical `detect_surfaces_type`'s own call,
`opening_ex(diff, external_perimeter_width / 10)` with `jtMiter` and miter limit
3, instead of `line_width / 2` with `Round` joins; Pass 2's shadow width is
carried separately (`ShellClassificationParams`) so the cascade cannot move.
**Two defects surfaced.** The region config authors `outer_wall_line_width = 0`,
canonical's **auto** sentinel, and resolving it literally disables the opening
entirely (measured no-op: 5,201,451 points in and out, 6.8 ms for 974 calls), so
`resolve_shell_external_width_mm` resolves it the way `Flow` does (0.4 mm ->
radius 0.04 mm); the same sentinel reaches the **bridge** classifier, where
canonical's `0.1 x fw` opening has therefore been a no-op on a path with live
readers, graduated as [168](issues/168-bridge-opening-auto-sentinel.md).
**Rejected on measurement:** also porting canonical's `ApplySafetyOffset` (one
workspace unit at this quantisation) made base classic-off slower (20.2 vs
19.2 s) and moved output; the radius-only arm reproduces the t166 probe's hash
`0a1ee3df...` exactly. **Campaign:** 80 paired runs, 5 interleaved pairs/cell,
every exit 0. Wall saved benchy **+20.435 / +7.915 / +21.395 / +9.277 %**
(**25.5x / 9.9x / 26.7x / 11.6x** the floor, 5/5 pairs faster each), base
**+3.448 / +2.468 / +0.310 / +0.734 %** (**4.31x / 3.09x / 0.39x / 0.92x**);
**process CPU lower in 8/8** (+2.340... +10.938 %). Null control (3 pairs x 3
cells): wall bands 6.09-11.54 % any-pair and 0.50-5.35 % median, CPU bands
0.74-1.74 % any-pair. So the benchy cluster is decisive, `base classic-on`
passes, `base classic-off` clears the floor but sits inside this window's
3.17 % null median band (DEGRADED), and the two base Arachne cells are
**sub-floor on wall while CPU-lower - unresolved by this instrument, not proven
neutral** (5/8 PASS, 1 DEGRADED, 2 sub-floor). **Parity:** top/bottom solid path
grows in all eight cells (+5.8...+783.6 mm) but the ticket's "never less" is
**falsified per layer** (12-29 top and 1-15 bottom layers lose path in every
cell) because Pass 1 classifies each layer against its neighbours - a
relabelling, not monotone growth (depth-0 seeds 361/311 -> 481/451 on base
classic-off). **Output:** all eight baselines reproduce t151's active set
exactly, all eight candidates move, and **t166's set is the new currency**
([`reference.json`](evidence/t166-shell-opening-parity/reference.json)).
**Route (estimate, cross-window):** composing the paired medians with t164's
board takes the benchy supports-off required cuts **49.45 % -> 36.47 %** and
**47.21 % -> 32.84 %**, the supports-on pair **65.33 % -> 62.35 %** and
**64.08 % -> 60.40 %**, and moves the `base` cells 0.3-3.4 %; **0 of 8 cells are
won** and the binding constraint (canonical prepass cost, t164/t165) is
untouched. **Gap:** no accelerated arm (two full `dist --accelerated` builds plus
its own campaign); t163 bounds that arm at -13.2...+4.4 % wall, which cannot
overturn the benchy numbers but could matter for the two sub-floor base Arachne
cells. Evidence:
[`evidence/t166-shell-opening-parity/`](evidence/t166-shell-opening-parity/FINDINGS.md)
(`verify.py` PASS with 7 negative controls; `parity_delta.py` PASS with 5).
**Frontier:** [167](issues/167-canonical-clipperutils-replay.md) (AFK,
unblocked) and [168](issues/168-bridge-opening-auto-sentinel.md) (AFK,
unblocked, new).

**Destination status (2026-11-19,
[t167](issues/167-canonical-clipperutils-replay.md)): Branch A fires
(`canonical_ge_3x_faster_on_hot_offset_sites`) — canonical `v2.5.0` is
3.0x–12.8x faster on every hot offset site and 2.5x–3.3x faster on canonical
shell openings, falsifying `t70`'s "kernel at parity" reading on offsets (while
confirming it on booleans) and graduating a wrapper + Clipper2 2.0.1 port as
the prepass route.** **Premise correction at the `v2.5.0` pin (`40ecc66f78`,
2026-11-07):** OrcaSlicer already migrated `ClipperUtils.cpp` completely off
Clipper1 onto **Clipper2 2.0.1** on 2026-10-02 (`222c6a2df5`, `#15969`,
deleting `deps_src/clipper/`); the ticket's Clipper1 premise came from the
older `OrcaSlicerDocumented` checkout (`3e1daccd7c`). All four engines
(`rust_pnp` `vendor/clipper2-rust` 1.1.0 + `mimalloc`, `cpp_c2_1_5_2` t69 twin,
`orca_v2_5_0` canonical Clipper2 2.0.1 + `ClipperUtils.cpp`, `orca_clipper1`
pre-2.5.0 ClipperLib 6.2.6 + `ClipperUtils.cpp`) were replayed on 413 cases
(295 `t69` + 118 newly captured `ShellClassification` Pass 1 `apply_opening`
and `gate_internal_bridge_sites` operands from `base classic-off`,
output-verified byte-identical to `t166`'s `0a1ee3df...` reference; capture
probe reverted and preserved as `probe-t167-dump.patch`; zero PnP production
change). **Single-thread (fresh process/case, 1 warmup + 5 repeats, fwd/rev,
405 unique cases):** booleans stay at parity (`rust_pnp / orca_v2_5_0`:
`difference` **0.79x/0.78x**, `intersection` **1.24x/1.24x**, `union`
**1.39x/1.41x**; `cpp_c2_1_5_2 / orca_v2_5_0` **0.95x–1.00x**), whereas
**`offset` is 4.96x/5.02x faster** in `orca_v2_5_0` (**4.00x** vs `t69`'s own
bare C++ Clipper2 1.5.2 twin): `IB_DEEP_NEG` **12.76x/12.28x**, `IB_DEEP_POS`
**4.56x/4.64x**, `t69`'s `base-t68sites` **3.92x/3.97x** (every site `A0`–`K`
**3.02x–6.08x**), `benchy-t68sites` **3.11x/3.13x**; `opening_ex` is
**2.49x/2.55x** (`SH_OPEN_TOP_CANON` **2.45x/2.57x**, `SH_OPEN_BOT_CANON`
**2.45x/2.56x**). **12-way concurrent loaded (320 base cases × 12 staggered
workers, fwd/rev):** whole-corpus wall `rust_pnp` **77.13/64.36 s** vs
`orca_v2_5_0` **23.09/21.75 s** (**3.34x/2.96x**; fwd `rust_pnp` began at 57 %
host load, rev at 1 %); load widens canonical's lead (`rust_pnp` degrades
1.54x–1.75x vs 1.27x–1.46x for `orca_v2_5_0`): `opcode/offset` **5.78x/5.16x**,
`base-t68sites` **5.12x/4.87x**, `SH_OPEN_TOP_CANON` **3.26x/2.91x**,
`SH_OPEN_BOT_CANON` **3.00x/2.79x**, `IB_DEEP_POS` **5.19x/4.49x**,
`IB_DEEP_NEG` **12.62x/11.36x**. **Allocator disclosure:** `rust_pnp` uses
`mimalloc` while the C++ shims use MinGW CRT `malloc` (not Orca's `tbbmalloc`),
making C++ speedups conservative lower bounds. **3-way ablation
(`ablation_decimation.json`, 118 prepass cases, 10 repeats):** (1)
`append_offset_path` edge decimation (`0.005 * |delta|`) alone delivers
**8.27x** (`IB_DEEP_NEG`), **3.56x** (`IB_DEEP_POS`), **2.04x–2.10x** (`0.20 mm`
legacy openings), **1.26x–1.29x** (`0.04 mm` canonical openings); (2) Clipper2
`1.5.2 -> 2.0.1` + flat-path `offset2_ex` pass 1 (zero decimation) alone
delivers **1.72x–1.76x** on `SH_OPEN_*_CANON`; (3) Rust 1.1 -> C++ 1.5.2 is
**1.09x–1.47x**. **Geometry & Clipper1 ruling:** `orca_v2_5_0` has **0 empty
mismatches** and **0.0044 %** median Δarea across 405 unique cases (`IB_DEEP_NEG`
+6.44 % median Δarea is canonical collapsing ~5 µm pinholes instead of
inflating them into 3.2 mm craters — so edge decimation is a non-output-neutral
canonical-parity change); **Clipper1 is ruled out** (`orca_v2_5_0` beats
`orca_clipper1` on `offset`, `base-t68sites`, `union`, and `SH_OPEN_*_CANON`,
and Clipper1 has a hairline-sliver inversion bug on `IB_DEEP_NEG` L133/L493).
Evidence: [`evidence/t167-canonical-clipperutils-replay/`](evidence/t167-canonical-clipperutils-replay/FINDINGS.md)
(`verify.py --self-test` PASS, 6/6 negative controls rejected). **Active
reference set unchanged (t166's set).** **Frontier:**
[168](issues/168-bridge-opening-auto-sentinel.md) (AFK, unblocked),
[169](issues/169-canonical-offset-edge-decimation-and-flat-opening.md) (AFK,
unblocked, new — wrapper port of edge decimation + flat-path `offset2_ex` and
re-pricing of t129's shapes), and
[170](issues/170-clipper2-2-0-1-kernel-port-screen.md) (AFK, unblocked, new —
Clipper2 `1.1.0 -> 2.0.1` kernel upgrade screen).

**Destination status (2026-11-19,
[168](issues/168-bridge-opening-auto-sentinel.md)): the bridge opening's auto
sentinel is fixed through one shared policy, the parity fix lands on
correctness grounds, and it buys no measurable wall; the frontier is 169 and
170.** `resolve_external_perimeter_width_mm`
(`crates/slicer-core/src/algos/prepass_slice.rs`) now requires a **positive**
absolute `outer_wall_line_width` and falls through to `line_width`, then 0.4 mm,
so the matched job's `outer_wall_line_width = 0` no longer disables canonical
`detect_surfaces_type`'s `opening_ex(diff, fw / 10)`; the shell stage's copy is
now a one-line delegate, so the two openings share one policy. **Falsified
pre-fix** by `external_perimeter_width_treats_zero_extension_as_auto` (old arm
returns 0.0, test reads 0.4). **Output:** baseline reproduces t166 in **8/8**,
candidate moves **8/8**, accelerated arm byte-identical to ordinary **16/16**;
all 286 runs `degraded: false`, zero exact-Z diagnostics. **Audit:** bridge path
falls in **8/8** (−10.5 / −10.5 / −5.0 / −5.0 / −11.4 / −11.4 / −8.1 / −8.1 mm),
internal bridge rises 8/8 (+3.6 / +2.0 mm), `Gap infill` inert, and sparse
infill rises only on `base classic` (+490.2 mm, +0.28 %) while falling
0.01–0.02 % on the other six cells — a partial reversal, reported. The
per-layer census is a relabelling (19–26 base layers gain bridge path, 27–33
lose it), and the anchor-direction proxy moves ≥ 45° on 12/48 / 13/61 / 2/7
bridge layers. **Paired wall (10 pairs/cell):** benchy +1.99 / +1.29 / +1.17 /
+1.71 %, base +4.09 / −0.14 / +4.15 / +0.73 %; null bands 4.89–9.86 % any-pair,
so **0/4 `base` cells clear the wall gate** (both supports-off medians inside
their band) while the benchy clause passes. **CPU lower in 8/8** (10/10 pairs in
five) and 8/8 on the accelerated arm — reported as the consistent channel, not
as a wall win. No ratio of record changes; the route arithmetic stands and the
8/8 destination is untouched. Evidence:
[`evidence/t168-bridge-opening/`](evidence/t168-bridge-opening/FINDINGS.md)
(`verify.py` PASS, `--self-test` 8/8; `audit_t168.py --self-test` 5/5).
**Frontier:** [169](issues/169-canonical-offset-edge-decimation-and-flat-opening.md)
(AFK, unblocked) and
[170](issues/170-clipper2-2-0-1-kernel-port-screen.md) (AFK, unblocked).

**Destination status (2026-11-20,
[t169](issues/169-canonical-offset-edge-decimation-and-flat-opening.md)): the
canonical pre-offset edge decimation is the route's first banked multi-cell win
since t158 — gate PASSED in 8/8 cells, +21.9…+25.1 % on the four `base` cells,
and `base classic-off` is estimated within 3.9 % of Orca; the frontier is 170.**
`polygon_ops` now harvests its rings exactly as canonical
`ClipperUtils.cpp::append_offset_path` does — wrap-around trim, collapse against
the **last kept** vertex, `< 3` drop, canonical's asymmetric degenerate-contour
branch, hole/winding normalisation — in `offset_paths` (`offset`,
`offset_with_miter_limit`, `opening`, `closing_ex`, pass 1 of `offset2_ex`) and in
`offset_paths_flat` for **pass 2**, whose threshold is `|delta2| * 0.005`, not
pass 1's. **Arm 1 was already in the tree:** flat-path pass-1 `offset2_ex`
landed 2026-07-11 (`a5433f93`) and was extracted into `offset2_ex_first_pass` by
`b56f429a`, so there is no Arm-1 delta to measure and t167's `f2` factor was
partly already banked; the whole measured effect is Arm 2 plus pass-2
decimation. **Campaign:** one window, 4 arms staged (`e7b67146…` / `b2e8261e…` /
`0784a754…` / `f56c9271…`), module trees `6899f824…` / `9c1da3bf…` / `82d74609…`
/ `9504e039…` — the trees move because `classic-perimeters` and
`arachne-perimeters` call the decimated helpers **in-process**, so this is not a
host-only change. **Output:** baseline reproduces t168 **8/8**, candidate moves
**8/8**, accelerated arms byte-identical **16/16**; the new currency is
[`reference.json`](evidence/t169-offset-decimation-flat-opening/reference.json)
and t168's set is superseded. **Paired wall (5 interleaved pairs/cell):** benchy
**+7.98 / +15.21 / +5.74 / +14.76 %**, base **+25.08 / +21.91 / +23.43 /
+22.41 %**; null band **3.06 %** any-pair / **1.99 %** median wall, so every
cell clears the 0.80 % floor (base at **27.4x–31.4x**) and every cell sits
outside its own band; **process CPU lower 8/8** (5/5 pairs in every cell) and
**8/8** on the accelerated arm (3 pairs/cell, wall +6.2…+25.2 %). **Audit:** every
section moves somewhere in every cell, total path −0.029…+0.616 %;
`Internal Bridge` **−72 / −73 %** on the two base supports-off cells is the
expected pinhole collapse, while `Support` **+0.42 / +1.13 %** on the supports-on
cells is a direction the port does not predict and is reported, not
rationalised. **Attribution (`--profile`):** `polygon_ops::offset` self wall
**−41.0…−55.5 %**, `offset2_ex` **−12.6…−25.1 %**; guest fuel falls 13.5–21.5 %
on the classic cells but **rises 1.4–3.1 %** on the three Arachne cells (the scan
costs more than the sweep saves on already-sparse geometry), disclosed.
**Route (estimate, cross-window):** composing the paired medians with t163's
ordinary board gives `base classic-off` **1.389x → ~1.04x** (3.9 % short),
`base arachne-off` ~1.12x, `base classic-on` ~2.12x, `base arachne-on` ~2.03x and
benchy ~1.64 / ~2.38 / ~1.83 / **~2.41x**; the binding cell moves to
**`benchy arachne-on` (58.5 % cut)**. **Exception:** landed under the **fifth
recorded t81-style exception** (per-layer geometry), granted in-session on these
numbers. Evidence:
[`evidence/t169-offset-decimation-flat-opening/`](evidence/t169-offset-decimation-flat-opening/FINDINGS.md)
(`verify.py` PASS, `--self-test` 6/6; `audit_t169.py --self-test` 4/4).
**Frontier:** [170](issues/170-clipper2-2-0-1-kernel-port-screen.md) (AFK,
unblocked) and
[171](issues/171-post-t169-prepass-reprice.md) (AFK, unblocked, new).

**Destination status (2026-11-20,
[t170](issues/170-clipper2-2-0-1-kernel-port-screen.md)): the "Clipper2 2.0.1
kernel upgrade" premise is falsified — the remaining offset gap is one missing
Orca fork patch, and it is estimated to win the first two cells.** The patch is
canonical `v2.5.0`'s `clipper.offset.cpp::OffsetConcaveCrossing` (≈30 lines,
**not** upstream 2.0.1, **not** in `clipper2-rust` 1.2.0, which is still a 1.5.4
port). Applied to bare Clipper2 1.5.2 it reproduces canonical 2.0.1
**byte-for-byte on 87/87** IB + `0.04 mm Miter 3` opening cases; without it,
2.0.1 is **0.78–0.87x** of 1.5.2 on those openings, so the kernel version
contributes nothing. Ported into a scratch `vendor/clipper2-rust` (which is a
**1.5.4** port despite crate 1.1.0) it captures **81–97 %** of the remaining
per-call gap: prepass IB offsets **1.86x** (1,326.3 → 711.4 ms; 1.13x of
canonical), prepass `SH_OPEN_*` **1.63x** (751.4 → 459.6 ms; 1.05x), `t68sites`
base per-layer offsets **3.17x** (713.7 → 225.2 ms; 1.20x), benchy **2.56x**
(116.1 → 45.4 ms; 1.11x); per-site the classic-perimeter offsets go
**2.95–4.57x → 1.07–1.27x** and vertex totals converge on canonical (`A0`
1.082 → 1.007, `A1` 1.073 → 0.998). Booleans are untouched (0.61–0.94x). The
scratch crate passes **403/403** tests; the patch is frozen in the evidence
dir. **Price (post-t169, two-step estimate):** central **+16.4 %** on
`base classic-off` (required **3.9 %**) and **+15.3 %** on `base arachne-off`
(required **10.3 %**), **5.2–11.4 %** elsewhere; conservative host-only clears
the 0.80 % floor in **8/8** (5.2x–16.3x). **But the kernel route is now bounded:**
even at zero cost for every host and guest offset, six cells stay short
(`base classic-on` 23.3 % vs 52.7 %; `base arachne-on` 21.6 % vs 50.8 %; the four
benchy cells 10.7–26.1 % vs 39.0–58.5 %), so those six need a second phase, not a
bigger kernel patch. The patch moves geometry and needs a **t81-style exception**
(the sixth, if granted) before it can land. Evidence:
[`evidence/t170-clipper2-201-screen/`](evidence/t170-clipper2-201-screen/FINDINGS.md)
(`verify.py` PASS, `--self-test` 6/6). **Frontier:**
[172](issues/172-port-offset-concave-crossing.md) (HITL ruling first, then the
standing gate) and [171](issues/171-post-t169-prepass-reprice.md) (AFK,
unclaimed).

**Destination status (2026-11-20,
[t171](issues/171-post-t169-prepass-reprice.md)): the split re-derives and the
route re-aims — the supports-off cluster is offset-bound and is t172's, the
supports-on cluster is **boolean**-bound, and S5/S6 are landed keeps with no
residual worth a ticket.** The post-t169 phase split is prepass
**55.9–83.7 %** (was 67.0–87.2 % at t164), per-layer **12.0–34.7 %** (was
9.3–26.3 %), postpass **4.0–8.3 %**, outside **0.2–3.3 %**; the prepass share fell
**3.5–12.3 pp** on all eight cells, by an amount that tracks each cell's t169
saving. The stage split is two clusters: `ShellClassification` **32.2–44.0 %** of
the supports-off cells (57.4–58.6 % of their prepass) and `SupportGeometry`
**38.9–54.9 %** of the supports-on cells (55.6–66.1 % of theirs). Expressed as
"what fraction of the largest stage the cut needs", the four **base** cells each
have a single sufficient term (**8.9 % / 24.2 %** of `ShellClassification` on the
supports-off pair; **96.0 % / 92.4 %** of `SupportGeometry` on the supports-on
pair), and **no benchy cell does** (its largest stage is **114.5 % / 140.7 % /
143.1 % / 150.3 %** of what the cell needs; the top two still leave 0.93x / 1.08x
/ 1.06x / 1.12x). The per-op census is the re-aim: host clipper thread-time is
**offset**-dominated on the supports-off cells (**42.9–51.2 %** offset,
29.5–34.4 % clip) and **clip**-dominated on the supports-on cells (**53.1–60.9 %**
clip, 34.1–39.4 % offset); on `base classic-on` the `host:native` bucket alone
holds **236.3 s of 343.4 s** (169,586 calls, clip 186.6 s) and the support-engine
guests own **57.7–68.0 %** of the fuel. **S5's** landed win rescales to **9.01 %**
of base classic-off's wall with a **0.83 %** residual (at the floor, 91.6 % of the
whole-layer cost already harvested); **S6's** rescales to **5.86 %** of base
classic-on with a **5.89 %** residual (11 % of that cell's cut) and is **inert on
the four supports-off cells by construction**. No production edit, no reference
move; all 24 instrumented passes reproduce t169's set byte-for-byte. Evidence:
[`evidence/t171-post-t169-reprice/`](evidence/t171-post-t169-reprice/FINDINGS.md)
(`verify.py` PASS, `--self-test` 8/8). **Frontier:**
[172](issues/172-port-offset-concave-crossing.md) (HITL ruling first, then the
standing gate) and
[173](issues/173-support-geometry-boolean-volume.md) (AFK, unblocked, new).

**Destination status (2026-11-20,
[t172](issues/172-port-offset-concave-crossing.md)): canonical
`OffsetConcaveCrossing` lands under the sixth recorded t81-style exception —
strict gate PASSED 8/8 (+14.9…+23.0 % wall, 5/5 pairs and CPU corroborating in
every cell), both base supports-off cells are now estimated to beat OrcaSlicer
(~0.82x and ~0.89x), and the Clipper2 kernel route is exhausted for the six
remaining cells; the frontier is 173 and the new scoreboard refresh 174.**
`ClipperOffset::offset_polygon` (`vendor/clipper2-rust/src/offset.rs`) now
ports OrcaSlicer `v2.5.0` (`40ecc66f78`) `clipper.offset.cpp::OffsetConcaveCrossing`
with `.round()` integer conversion and the AGPLv3 attribution header (excluding
`DoRound`, `GetLineIntersectPt`, `c2_offset_as_group`, and any `polygon_ops`
changes per the human's Q1 ruling). **Falsifiers:** two new unit tests in
`vendor/clipper2-rust/src/offset_tests.rs` were watched failing before the patch
(`96 != 32` raw pre-union vertices on a 32-gon inward offset) and green after
(**405/405** + 3 doc-tests); the pre-patch self-captured fixture
`crates/slicer-core/tests/fixtures/clip_polylines_prepared_reference.json` was
updated for two inflated rings (`hole_and_island` cyclic rotation by 1,
`dense_ring` ≤ 100 nm on concave corners; all clipped polylines identical).
**Campaign & output:** four arms staged (`b2e8261e…` / `9f3fcba8…` /
`f56c9271…` / `490c6dc2…`, module trees `9c1da3bf…` / `bd132f43…` / `9504e039…`
/ `91821d61…`); baseline reproduces t169 **8/8** on both arms (`16/16`),
candidate moves **8/8**, accelerated candidate is byte-identical to ordinary
**8/8** (`16/16`); active currency is now
[`reference.json`](evidence/t172-offset-concave-crossing/reference.json) (t169
superseded). **Paired wall (5 interleaved pairs/cell, ordinary):** benchy
**+23.01 / +18.41 / +21.49 / +17.37 %**, base **+21.41 / +14.89 / +19.88 /
+15.00 %** (**5/5** wins in every cell, **18.6x–28.8x** the 0.80 % floor,
strictly outside every cell's same-window any-pair wall null band of
2.29–13.69 %); **CPU +19.20…+30.67 %** (**5/5** in every cell); **accelerated
corroboration (3 pairs/cell):** wall **+14.19…+22.63 %** and CPU
**+19.33…+30.93 %** (**3/3** in every cell). Both base supports-off cells beat
t169's estimated required cuts (**+21.41 % vs 3.9 %**, **+19.88 % vs 10.3 %**).
**Delta audit:** total extruding path **−0.745…+0.007 %** with no feature loss
(`Outer wall` ≤ 0.0012 %, `Inner wall` ≤ 0.042 %; `Support` −1.57 % benchy /
−0.69 % base, reported not rationalised). **Route (estimate, cross-window, not
a ratio of record):** composing with t169 × t163 puts `base classic-off` at
**~0.82x** (−18.2 %) and `base arachne-off` at **~0.89x** (−10.6 %), while six
cells stay above 1.0x (benchy ~1.26 / ~1.94 / ~1.44 / **~1.99x** binding cell
at 49.8 % cut; base supports-on ~1.80 / ~1.73x). **Kernel route exhausted:**
with t169 + t172 landed, only 1.04–1.30x per-call offset overhead remains
(t170), so no further Clipper2 kernel work can close the six remaining cells.
Evidence:
[`evidence/t172-offset-concave-crossing/`](evidence/t172-offset-concave-crossing/FINDINGS.md)
(`verify.py` PASS, `--self-test` 8/8; `audit_t172.py --self-test` 4/4).
**Frontier:** [173](issues/173-support-geometry-boolean-volume.md) (AFK,
unblocked) and [174](issues/174-post-t172-scoreboard-refresh.md) (AFK,
unblocked, new — verify 2/8 cells won on a same-window two-tool board).

**Destination status (2026-11-20,
[t173](issues/173-support-geometry-boolean-volume.md)): the support path's
clip mass is the planner's phase-1 carve and phase-3 union, no single boolean
shape clears either base supports-on cell, and the benchy pair is not a boolean
program; the frontier is 174 and the new phase-1 tiling candidate 175.** The
`host:native` clip bucket on `base classic-on` is
`com.core.tree-support-planner`: **196.6 s** clip thread / **18.00 s** wall,
**85.2 %** of all clip thread, with **142.7 s / 12.41 s** in
`planner::batched_phase1_carve` (**68,381** calls, **426.9 M** clip points,
mean **6,241/call** over **4.34** paths against a **98.7**-point subject,
**11.5x** fan-out) and **51.6 s / 5.21 s** in
`planner::batched_phase3_union` (**480** calls, **9.9x**). The two sites are
**98.8 %** of the planner's clip thread. Host built-ins are the rest
(`shell_classification` 13.5 s, `support_analysis` 3.8 s, `mesh_analysis`
2.9 s, `slice` 3.2 s, `overhang_annotation` 2.5 s) plus 7.8 s of unnamed
per-layer host calls. **Stage decomposition (same window, base classic-on):**
**62.2 %** host clip, **3.3 %** host offset, **34.5 %** remainder; on
`benchy classic-on` it is **38.8 %** host clip and **56.1 %** remainder, and
the planner is 10.8 % of cell guest fuel. **Operand shape:** only **285,368 of
426,868,700** clip points (**0.0669 %**) lie inside the subject bbox, but the
polygons are huge — a polygon-level bbox filter removes only **28.5 %**.
**Prices (ceilings, fraction of the cell's required cut):** phase-1 bbox/tile
window **60.8 % / 61.6 %** (leaving **1.150x / 1.120x**); polygon-bbox filter
**17.6 % / 17.8 %**; phase-3 union free **26.0 % / 27.4 %** (no shape); all
planner clip free **89.7 % / 92.1 %** (**0.948x / 0.926x**); all planner clip +
offset free **94.7 % / 97.6 %** (**0.913x / 0.891x**). Phase-1 window + phase-3
free is **0.969x / 0.945x**, the first projected crossing, but phase 3 is a
ceiling, not a mechanism. The benchy pair gets **23.2 % / 21.8 %** from
phase 1 and **35 % / 34 %** from all clip free. **Re-aim:** the base pair is a
**combination** (phase-1 tiling + a phase-3 companion), and the benchy pair
needs the per-layer/guest program. Evidence:
[`evidence/t173-support-boolean-volume/`](evidence/t173-support-boolean-volume/FINDINGS.md)
(`verify.py` PASS, `--self-test` 8/8; 5 probe runs + 15 instrumented passes
reproduce t172's set; `probe.patch` reverted). **Frontier:**
[174](issues/174-post-t172-scoreboard-refresh.md) (AFK, unblocked) and
[175](issues/175-tiled-collision-carve.md) (AFK, unblocked, new — exact phase-1
tiling candidate + standing gate; not a destination step alone).

**Destination status (2026-11-21,
[t174](issues/174-post-t172-scoreboard-refresh.md)): official ratios of record
are now 0.769x–1.870x ordinary (0.766x–1.916x accelerated) in one quiet
interleaved window; PnP decisively wins 2 of 8 cells on unstarved wall clock
with strict separation (`base classic-off` 0.769x / 0.766x, `base arachne-off`
0.792x / 0.774x), but burns 1.81x–2.13x Orca's process CPU via ~5.2x vs
~1.9–2.2x thread concurrency, so the official Destination tally stays 0 of 8
pending a human ruling ([176](issues/176-destination-cpu-corroboration-ruling.md));
the binding cell shifts to `benchy classic-on` (1.870x / 46.5 % cut).** Zero
production edits; both freshness gates exit 0; stamped identities at HEAD
`0ce4be39` (code `48a5e583`): OrcaSlicer **2.5.1** (`5013b1bd…`, vendor
`02.08.00.16`, `tree_strong`), PnP ordinary `06b05b6b…` / tree `e75be5c0…`, PnP
accelerated `490c6dc2…` / tree `481ff759…`. Quiet board
(`t174-quiet-board`, 96 rows, 0/72 starvation flags, steady 10.8–14.1 GB free
RAM, load 85 samples 14/79/100 %): **16/16** cell × arm outputs byte-identical
to t172's active currency (`degraded: false`, **0** code-1200 diagnostics),
**10/10** matched Orca keys on all 32 Orca runs. **Official ratios of record
(ord / acc, wall):** benchy **1.144x / 1.118x** (cut 12.6 / 10.6 %), **1.870x /
1.916x** (cut **46.5 / 47.8 %**, binding), **1.155x / 1.064x** (cut 13.4 /
6.0 %; acc only 0.15 s behind Orca), **1.665x / 1.674x** (cut 40.0 / 40.3 %);
base **0.769x / 0.766x** (−23.1 / −23.4 % wall, strict sep 3.66 / 3.26 s, CPU
**2.126x / 2.066x**), **1.777x / 1.704x** (cut 43.7 / 41.3 %), **0.792x /
0.774x** (−20.8 / −22.6 % wall, strict sep 2.37 / 2.15 s, CPU **1.844x /
1.810x**), **1.738x / 1.684x** (cut 42.5 / 40.6 %). **Accelerated vs ordinary:**
faster in **6/8** wall (−7.86…+2.45 %, median **−2.28 %**; faster in all 4 base
cells, reversing t163's +4.4 % `base classic-off`), lower CPU in **7/8**
(−2.91…+0.52 %, median **−1.16 %**). **Why benchy beat t172's estimates:**
t169's cross-window chain had omitted t166's landed benchy shell-opening gains
(+7.9…+21.4 %); `base` supports-on confirms t173's cuts (43.7 / 42.5 % vs
44.5 / 42.1 % est). **Below-fold ~1.5x park bound crossed** by all four
supports-off cells. **Disclosed:** an initial loaded-box take
(`t174-post-t172-scoreboard`, external ~10 GB load exiting mid-`base-arachne-on`,
4 starvation flags, 64–98 % spreads) is invalidated as primary and preserved as
`reduced/loaded-window.json` (`t156` precedent; still 16/16 identity and 2/8
wall wins at 0.832x / 0.863x). Evidence:
[`evidence/t174-post-t172-scoreboard/`](evidence/t174-post-t172-scoreboard/FINDINGS.md)
(`verify.py` PASS 1280/1280, `--self-test` 18/18). **Frontier:**
[175](issues/175-tiled-collision-carve.md) (AFK, unblocked) and
[176](issues/176-destination-cpu-corroboration-ruling.md) (HITL grilling,
unblocked, new — CPU-corroboration & `n=3` vs `n>=4` cell-closure ruling).

**Destination status (2026-11-21,
[t175](issues/175-tiled-collision-carve.md)): the planner's phase-1 collision
carve is tiled and banked (+31.3 / +30.9 % wall on `base` supports-on, +16.1 /
+16.6 % on `benchy` supports-on, CPU corroborating, beating t173's host-only
ceiling on every supports-on cell), landed by explicit human KEEP ruling with
6/8 cells byte-identical and 2 `base` supports-on hashes updated for 1 µm
Clipper2 distant-scanbeam unclamping; `base` supports-on drops to est ~1.22x /
~1.20x (crossing below ~1.5x park), phase-3 union is now ~92 % of remaining
`host:native` clip thread, and the sole open ticket on the frontier is the
human's [176](issues/176-destination-cpu-corroboration-ruling.md).**
Guest-only exact non-perturbing per-layer tile index (`CollisionTileIndex` in
`modules/core-modules/tree-support-planner/src/collision_tile_index.rs`, wired
into `build_roles_batched` Phase 1 in `lib.rs`; zero host/WIT/schema edits,
identical host exes on both arms). **Sweep (`base-classic-on`, 1 / 2 / 5 / 10 mm,
3 alternating rounds, load 1.43 → 5.2 %):** `baseline` 41.024 s, `1 mm` 28.336 s
(+30.93 %), **`2 mm` 28.311 s (+30.99 %, chosen by lowest wall)**, `5 mm`
28.587 s (+30.32 %), `10 mm` 29.124 s (+29.01 %); all four tiled arms emit the
exact same SHA-256 `7b90e749…`. **Measured partition cost & operand reduction
(`--profile`, `probe.patch` reverted):** on `base-classic-on`, `tile_index_build`
**45.9 ms** (1.44 B fuel, 430 layers) + `tile_index_query` **84.6 ms** (1.55 B,
68,381 queries) = **130.5 ms** collapses `phase1_clip_batches` **12,436.6 →
174.4 ms (−98.6 %)**, sent requests **68,381 → 30,639** (37,742 empty-gated,
23,468 cache hits), Phase-1 clip points **426,868,700 → 3,629,342 (−99.15 %**,
0.85 % remaining vs t173's 1.62 % estimate), net planner guest fuel **62.27 →
48.58 B (−22.0 %)**, and `host:native` clip thread **173.90 → 56.02 s**;
`benchy-classic-on` build + query **17.9 ms**, clip points **28,230,127 →
251,534 (−99.11 %)**. **Standing `t137` gate (5 interleaved pairs/cell ordinary,
3-pair null, 3-pair accelerated; load 0–6 %; 208/208 runs clean `exit=0`,
`degraded=false`, `0` code-1200):** `benchy-classic-on` **+16.06 %** wall (5/5,
20.1× floor, null 2.02 %) / **+26.75 %** CPU (acc +16.42 / +25.20 %);
`benchy-arachne-on` **+16.62 %** (5/5, 20.8×, null 2.29 %) / **+25.36 %** (acc
+15.84 / +27.06 %); `base-classic-on` **+31.32 %** (5/5, 39.2×, null 1.10 %) /
**+40.86 %** (acc +31.39 / +40.97 %); `base-arachne-on` **+30.90 %** (5/5,
38.6×, null 1.08 %) / **+40.10 %** (acc +31.53 / +40.39 %); four supports-off
cells −0.58…+0.74 % ord (−0.73…+0.55 % acc), structurally inert (**4/4
INERT-PASS**). **Output & Clipper2 proof:** baseline reproduces t172 **16/16**,
candidate ord ≡ acc **16/16**, **6/8 cells byte-identical** (all four
supports-off + both benchy supports-on); only `base-classic-on` (−1 B, 7/976,781
lines, −0.00071 mm) and `base-arachne-on` (−1 B, 5/972,259 lines, −0.00071 mm)
move by **1 µm** on four XY vertices strictly in `Support` / `Support interface`
(zero part-geometry movement). Exhaustive 68,381-call audit proved **0**
`Empty`/`Covered` mismatches and **9/9 (100 %)** single-vertex differences
caused by `vendor/clipper2-rust/src/engine.rs::add_new_intersect_node` clamping
shallow intersections to global `scanline_list` Y coordinates injected by
unrelated distant collision vertices 14–36 mm away (`8/9` bulk + 9th isolated
`full_eq_win_with_dummy_far_y = true`). **Cross-window route estimate vs t174
quiet board (not a ratio of record):** `base-classic-on` **~1.221x / ~1.169x**
ord/acc (71.6 % of cut closed vs t173's 60.8 % ceiling), `base-arachne-on`
**~1.201x / ~1.153x** (72.8 % vs 61.6 %), `benchy-arachne-on` **~1.389x /
~1.409x** (41.6 % vs 21.8 %), `benchy-classic-on` (still binding) **~1.570x /
~1.602x** (34.5 % vs 23.2 %). Evidence:
[`evidence/t175-tiled-collision-carve/`](evidence/t175-tiled-collision-carve/FINDINGS.md)
(`verify.py` PASS, `--self-test` 13/13; `audit_t175.py --self-test` 6/6).
**Frontier:** [176](issues/176-destination-cpu-corroboration-ruling.md) (HITL
grilling, unblocked); phase-3 companion remains unnamed in **Not yet specified**.

**Destination status (2026-11-21,
[t176](issues/176-destination-cpu-corroboration-ruling.md)): Destination ruling
settled — official running tally moves `0/8 → 2/8` provisionally won
(`base-classic-off` 0.769x, `base-arachne-off` 0.792x), Destination wording
amended to wall-clock win + CPU integrity guard + efficiency disclosure, bar
kept at `< 1.0x` (fresh `<= 0.50x` effort planned afterward), and four AFK
speedup-discovery tickets graduate (`177`, `178` unblocked; `179`, `180` blocked
by `177`).** Zero production code edits (tracker + [`GLOSSARY.md`](../../../GLOSSARY.md)
only). Human rulings: (1) **Option C** — cross-engine Destination cell closure
uses lower median uninstrumented wall clock guarded by the **CPU integrity
guard** (tight CPU, unstarved `cpu/wall`, matched non-degraded output), with
PnP/Orca CPU ratio disclosed beside every cell as a separate efficiency metric
(within-PnP **candidate A/B CPU corroboration** unchanged); (2) **`pnp-ordinary`
is the binding arm** (`pnp-accelerated` disclosed alongside; swaps only if
formally adopted as production default); (3) **margin-tiered provisional
closure + final `n >= 4` closing board** — quiet `n=3` strict separation with
`0` starvation closes provisionally when `ratio <= 0.85x`, close cells
(`0.85x < ratio < 1.00x`) and the final single-HEAD 8-of-8 closing board
require `>= 4` retained unstarved runs per arm with strict separation (starved
outliers excludable per `SCOREBOARD.md` if `>= 4` unstarved runs remain); (4)
provisional wins **carry forward** across later commits passing paired
non-regression gates (re-opened only by a later Orca board `>= 1.0x` or an
unwaived regression); (5) Destination stays **`< 1.0x` in 8/8** as a closeable
milestone with a fresh `<= 0.50x` effort afterward, prioritizing high-leverage
speedups now. Under this ruling, [t174](issues/174-post-t172-scoreboard-refresh.md)'s
quiet-window wins on `base-classic-off` (**0.7687x** ord, 3.66 s sep, 0
starvation, CPU `2.126x` disclosed) and `base-arachne-off` (**0.7921x** ord,
2.37 s sep, 0 starvation, CPU `1.844x` disclosed) — both structurally inert and
byte-identical under [t175](issues/175-tiled-collision-carve.md) — close
provisionally (**2/8**). Added four canonical terms to `GLOSSARY.md` (**CPU
integrity guard**, **Candidate CPU corroboration**, **Binding arm**,
**Provisional cell win**). **Frontier:**
[177](issues/177-post-t175-full-matrix-attribution.md) (AFK, unblocked —
post-t175 8-cell attribution + `0.50x` headroom map),
[178](issues/178-planner-phase3-union-attribution.md) (AFK, unblocked —
planner Phase-3 union mechanism screen toward 4/8),
[179](issues/179-per-layer-tree-support-and-infill-linker-screen.md) (AFK,
blocked by `177` — `tree-support` renderer & `infill-linker` guest screen), and
[180](issues/180-shell-classification-post-t172-residual-screen.md) (AFK,
blocked by `177` — `ShellClassification` post-t172 residual screen).

**Destination status (2026-11-21,
[Post-t175 full-matrix attribution and 0.50x headroom map](issues/177-post-t175-full-matrix-attribution.md)):
full post-t175 attribution is re-derived (zero production edits, 32/32 runs
byte-identical to `t175` currency) — all 8 cells are now `clip_polygons`-dominated,
5 of the 6 open cells are single-stage `< 1.0x`-winnable (leaving ONLY binding
`benchy-classic-on` at ~1.570x est. needing a 3–4 track combination), NO
`benchy` cell can reach `<= 0.50x` via prepass deletion alone, and two new
bottlenecks surfaced (`SupportGeometry` 6.1–7.8 % post-planner host tail
graduated as [`SupportGeometry` post-planner host tail attribution and reduction screen](issues/181-support-geometry-post-planner-host-tail.md),
plus `benchy` `L2` `infill-linker` 5.8–7.9 % idle-core straggler tail fed into
unblocked [Per-layer `tree-support` renderer and `infill-linker` guest-fuel screen](issues/179-per-layer-tree-support-and-infill-linker-screen.md)).**
Stamped ordinary arm at HEAD `24848d9e` (code `5b138f5c`, `cargo xtask build-guests --check`
exit `0`, `pnp_cli.exe` `f3075232…`, 24 core `.wasm` tree `b46eb4cf…`),
`RAYON_NUM_THREADS=12`, unsandboxed (initial sandboxed capture inflated
`per-layer` ~2.8× on `benchy` and was discarded). **Phase split (3 passes/cell,
spread `0.65–5.04 pp`, reconciliation `<= 4.09 %` prepass / `<= 2.04 %`
postpass):** prepass **53.5–77.7 %** (−2.4 to −7.5 pp vs `t171`), per-layer
**14.3–34.3 %**, postpass **6.8–11.8 %** (highest on `benchy` supports-on at
**11.8 %**), outside **0.3–4.1 %**. **Per-op census (`--profile`, attribution
only):** host Clipper thread-time dropped **−36.8…−40.3 %** supports-off and
**−58.4…−59.7 %** supports-on (`base-classic-on` `343.38 → 140.34 s`), flipping
**8/8** cells to `clip_polygons`-dominant (**46.5–51.6 %** off, **54.0–58.6 %**
on vs `offset` **30.8–37.8 %**). **Route (`ESTIMATE ONLY`, `t174 × t175`):**
`base` supports-on (`~1.221x / ~1.201x`, 18.1 / 16.7 % cut) closes via
`PrePass::SupportGeometry` (**44.4 / 43.4 %** → **0.678x / 0.679x**); `benchy`
supports-off (`1.144x / 1.155x`, 12.6 / 13.4 % cut) is now single-stage
winnable via `PrePass::ShellClassification` (**27.9 / 27.0 %** → **0.825x /
0.843x**, running at only **4.5×** concurrency on `benchy` vs **8.1–8.6×** on
`base`) OR `per-layer` (**32.4 / 34.3 %**, where `L2` `infill-linker` alone
causes a **134 / 104 ms = 7.9 / 5.8 % cell** 11-idle-core straggler tail, while
`perimeters` outranks `infill-linker` in elapsed thread-time); `benchy-arachne-on`
(`~1.389x`, 28.0 % cut) crosses marginally at zero `SupportGeometry` (30.4 % →
0.966x) and closes cleanly multi-track; binding `benchy-classic-on` (`~1.570x`,
36.3 % cut) cannot close from any single stage (`SupportGeometry` 31.7 % →
1.071x) nor from `ShellClassification + per-layer` (33.1 % → 1.051x) and needs
`SupportGeometry` + `ShellClassification` + `tree-support` (+ `SupportAnalysis`).
**`<= 0.50x` headroom:** zeroing all of `prepass` leaves `benchy` at
**0.501x–0.537x** (all 4 short), proving the follow-on 2× faster map requires
BOTH prepass and per-layer cuts across `benchy` (and `ShellClassification +
per-layer` = 58.2–59.5 % → 0.321x on `base` supports-off). **New bottleneck:**
`2,293 / 2,017 ms` (**7.8 / 7.0 %** cell, `base-on`) and `215 / 216 ms`
(**6.2 / 6.1 %** cell, `benchy-on`) elapse in `PrePass::SupportGeometry` *after*
`tree-support-planner::module_complete`. Evidence:
[`evidence/t177-post-t175-attribution/FINDINGS.md`](evidence/t177-post-t175-attribution/FINDINGS.md)
(`verify.py` PASS 5/5, `--self-test` 12/12). **Frontier (all 4 AFK, unblocked):**
[Planner Phase-3 batched union attribution and mechanism screen](issues/178-planner-phase3-union-attribution.md),
[Per-layer `tree-support` renderer and `infill-linker` guest-fuel screen](issues/179-per-layer-tree-support-and-infill-linker-screen.md),
[`ShellClassification` post-t172 residual attribution and speedup screen](issues/180-shell-classification-post-t172-residual-screen.md),
and [`SupportGeometry` post-planner host tail attribution and reduction screen](issues/181-support-geometry-post-planner-host-tail.md).

**Destination status (2026-11-21,
[Planner Phase-3 batched union attribution and mechanism screen](issues/178-planner-phase3-union-attribution.md)):
Phase-3 union is attributed (4.43 s wall / 45.52 s thread = `15.68 %` of
`base-classic-on`, 99.99 % `ROLE_BODY` monolithic sweeps over 1,024-circle
clusters with 7.64× area overlap and 28.1 % strictly convex-contained),
Mechanisms A/B/C and `subject=all` are falsified as sufficient levers, and a
host-side spatial hierarchical union + fast convex-containment pruning mechanism
(`M9n_host_nested_prune_hier_k8`, −91.4 % Phase-3 wall / −92.4 % thread on
`base`, −69.3 % wall on `benchy`, `< 1 µm` worst-case boundary Hausdorff, zero
WIT/guest changes, 100 % supports-off inert) graduates as
[Host-side spatial hierarchical Phase-3 union with convex-containment pruning](issues/182-host-spatial-hierarchical-phase3-union.md).**
Zero production edits landed (`probe.patch` cleanly reverted, both freshness
gates exit `0`); **12/12** capture runs exit `0`, `degraded = false`, `0`
code-1200, byte-identical to `t175`'s active currency, and classic ≡ arachne on
Phase-3 operands (`{benchy: true, base: true}`). Clean uninstrumented Phase-3
wall: `base` **`4,434.3 / 4,404.6 ms`** (`15.68 / 15.39 %` cell, `10.27–10.28×`
par, `sum(batch_max) = 99.1 / 98.7 %` wall, WIT/IR conversion `0.23 %`),
`benchy` **`154.5 / 153.8 ms`** (`4.48 / 4.32 %`, `10.03–10.06×`). Offline
12-thread replay over 100 % of real captured calls (`480/480` `base` + `202/202`
`benchy` exact vertex-for-vertex baseline replay, `0` mismatches): singletons
are only `0.39 %` (`base`, 530/530 canonical/simple) and `0.29 %` (`benchy`,
63/63), while **88.5 %** of `base` vertices sit in giant connected components
(`>= 65` polys, max `1,023`), falsifying standalone `M2` CC-split; Phase-4
`expolygons_simplify` never runs (`avg_nodes = 8 / 5 <= 200`) and forced
pre-union RDP (`M6`) is catastrophically lossy (`870–3,902 mm²` XOR); LPT/global
batching (`M5a/b`) hits the `3.79 s` 12-thread floor (`0 %` CPU cut); `M1`
(`subject=all`) is speed-neutral; exact duplicates = `0`. Exhaustive dummy-far-Y
auditing proved **`147/147` `base`** and **`27/27` `benchy`** (100.0 %) of
`M2`'s diffs are `vendor/clipper2-rust/src/engine.rs::add_new_intersect_node`
distant-scanbeam clamping (`0.162 / 0.130 µm` boundary Hausdorff). Because
`modules/core-modules/tree-support-planner/src/lib.rs::run_clip_batches` is the
**sole caller** of `clip_polygons_batch` in the repo, wiring
`slicer_core::polygon_ops::union_spatial_tree` strictly into
`clip_polygons_batch` `ClipOperation::Union` requires zero WIT/guest changes and
is 100 % structurally inert on supports-off. **Route (`ESTIMATE ONLY`,
`t174 × t175 × t178`):** `M9n_k8` clears `t137`'s 4-cell floor by
**3.7×–17.9×** (`base-classic-on` `−4,052 ms` = `−14.33 %` → `1.046x` ord /
`1.002x` acc; `base-arachne-on` `−4,025 ms` = `−14.06 %` → `1.032x` ord /
**`0.991x` acc**; `benchy-classic-on` `−107 ms` = `−3.10 %` → `1.521x`;
`benchy-arachne-on` `−107 ms` = `−2.99 %` → `1.347x`), closing **79.3 / 84.0 %**
of `base` supports-on's remaining ordinary gap; stacking with half-to-full
[`SupportGeometry` post-planner host tail attribution and reduction screen](issues/181-support-geometry-post-planner-host-tail.md) (`7.8 / 7.0 %`)
crosses `< 1.0x` cleanly on both ordinary `base` cells (**`~0.998x → 0.951x`**
and **`~0.990x → 0.948x`**, toward **4/8**). Evidence:
[`evidence/t178-planner-phase3-union/FINDINGS.md`](evidence/t178-planner-phase3-union/FINDINGS.md)
(`verify.py` PASS 6/6, `--self-test` 14/14). **Frontier (all 4 AFK, unblocked):**
[Per-layer `tree-support` renderer and `infill-linker` guest-fuel screen](issues/179-per-layer-tree-support-and-infill-linker-screen.md),
[`ShellClassification` post-t172 residual attribution and speedup screen](issues/180-shell-classification-post-t172-residual-screen.md),
[`SupportGeometry` post-planner host tail attribution and reduction screen](issues/181-support-geometry-post-planner-host-tail.md),
and [Host-side spatial hierarchical Phase-3 union with convex-containment pruning](issues/182-host-spatial-hierarchical-phase3-union.md).

**Destination status (2026-11-21,
[Per-layer `tree-support` renderer and `infill-linker` guest-fuel screen](issues/179-per-layer-tree-support-and-infill-linker-screen.md)):
both per-layer guests are attributed with zero production edits (`130/130` runs
byte-identical to `t175`, `probe.patch` cleanly reverted, both guest freshness
gates exit `0`) — `t66` was DROPPED (never landed), so `tree-support` is
`98.1–98.5 %` reducible in exact guest fuel (`80.1–80.3 %` dead
`paint_policy_for` + `15.6–16.3 %` `skeleton_wall_count` + `1.5–2.8 %`
redundant `role_union`) and `infill-linker` is `73.0–78.6 %` reducible
(`49.6–52.9 %` `clip_batched` geometric attribution/repair, `70.8–71.3 %` of
`benchy` `L2`), and combining `TS-2 + IL-ALL` eliminates both standalone gate
traps (`L2` straggler unmasking under standalone `TS-2`, sub-floor `0.52 %` on
`base-classic-on` under standalone `IL-ALL`) to clear the literal 8-cell
`0.80 %` screen floor at `2.6×–12.8×` (`24/24` wall wins, `24/24` CPU wins),
graduating
[Exact per-layer guest acceleration: `tree-support` renderer fast paths and `infill-linker` open-subject tagging](issues/183-per-layer-tree-support-and-infill-linker-exact-acceleration.md).**
Premise correction: `t66` was dropped in Oct 2026, so
`crates/slicer-sdk/src/traits.rs::PaintRegionLayerView::paint_policy_for` still
walks `723.5 M` (`benchy`) / `6.76 B` (`base`) ring edges across `0` paint
annotations (`paint_has_ann = 0`, `paint_cover_hits = 0`);
`modules/core-modules/tree-support/src/lib.rs::skeleton_wall_count` rescans
contour bboxes `358.8 M` / `3.43 B` times when `99.21 / 99.06 %` of skeleton
points have `count <= best`; and `TreeSupport::run_support` re-runs in-guest
`union_ex` on already-disjoint planner/regularized roles (`206 / 517` calls).
`TS-2` cuts `tree-support` fuel **`−98.45 %` (`64.7×`, `41.94 → 0.65 B`)** on
`benchy-classic-on` and **`−98.06 %` (`51.4×`, `388.79 → 7.56 B`)** on
`base-classic-on` (surviving wall is real `host:native` `offset_polygons`:
`2,001 / 6,345` calls). Inside `com.core.infill-linker`, native open-subject
`source_id: u32` tagging in `vendor/clipper2-rust` (`add_open_subject_tagged` +
`execute_open_tagged` — exact because open paths never self-intersect or join
in Clipper2) removes `attributed_runs` + `1,878–4,002` ambiguity repairs
(`IL-1`, `−49.7…−53.2 %` fuel), and pairing dead `source_segments` (`union_groups
== 0` in 8/8) + `O(1)` uniform metadata (`IL-2`, `8.5–21.7 %`) + bbox-pruned
`project_on_ring` & `O(E)` sorted-ring `nearest_pair_candidates` (`IL-3`,
`10.8–16.2 %`) cuts fuel **`−72.96 / −73.47 %` (`benchy`)** and
**`−78.60 / −78.28 %` (`base`)**, collapsing `benchy` `L2` `infill-linker`
`186 → 42 ms` (`classic-off`) and `237 → 84 ms` (`arachne-off`). **Unsandboxed
screen (attribution only, not a landing gate; `104/104` byte-identical to
`t175`):** standalone `arm_ts2` unmasks `L2`'s tail on `benchy-classic-on`
(`50 → 110 ms`, `c/w` `4.57× → 3.97×`, `−1.39 %` wall despite `+11.94 %` CPU)
and standalone `arm_il` is capped at `146 ms` (`0.52 % < 0.80 %`) on
`base-classic-on`; combined `TS-2 + IL-ALL` cuts `per_layer` wall
`−44.6 / −34.6 / −44.3 / −41.1 %` on supports-on and `−29.1 / −23.1 / −7.0 /
−12.2 %` on supports-off, clearing `0.80 %` in **8/8 cells** (`benchy`
`+10.26 / +4.62 / +5.32 / +6.71 %`, `base` `+2.50 / +6.20 / +2.11 / +6.58 %`,
`2.64×–12.82×` floor, `24/24` wall wins, `24/24` CPU wins `+3.33…+13.75 %`,
`8/8` strict CPU sep, `7/8` strict wall sep). No standalone `with_max_len(1)`
ticket graduates (`L2` tail drops to `5–6 ms` on three benchy cells and `31 ms`
on `benchy-classic-off`, where `~159 ms` of `L2` is 3× host `region_partition`).
**Route (`ESTIMATE ONLY`, `t174 × t175 × t179`):** `benchy-classic-off`
`1.144x → ~1.026x` ord / `~1.003x` acc (`81.6 %` of `< 1.0x` cut closed),
`benchy-arachne-off` `1.155x → ~1.093x` / `~1.007x`, `base-classic-on`
`1.221x → ~1.145x` (**`~0.970x` stacked with `t182`, `~0.875x` with
`t182 + t181`**), `base-arachne-on` `1.201x → ~1.122x` (**`~0.953x` with
`t182`, `~0.869x` with `t182 + t181`**), `benchy-classic-on` `1.570x → ~1.498x`
(`1.404x` `t177`-implied; `~1.351x` with `t182 + t181`), `benchy-arachne-on`
`1.389x → ~1.295x` (`~1.169x` with `t182 + t181`). Evidence:
[`evidence/t179-per-layer-guest-screen/FINDINGS.md`](evidence/t179-per-layer-guest-screen/FINDINGS.md)
(`verify.py` PASS `6/6`, `--self-test` `14/14`). **Frontier (all 4 AFK,
unblocked):**
[`ShellClassification` post-t172 residual attribution and speedup screen](issues/180-shell-classification-post-t172-residual-screen.md),
[`SupportGeometry` post-planner host tail attribution and reduction screen](issues/181-support-geometry-post-planner-host-tail.md),
[Host-side spatial hierarchical Phase-3 union with convex-containment pruning](issues/182-host-spatial-hierarchical-phase3-union.md),
and [Exact per-layer guest acceleration: `tree-support` renderer fast paths and `infill-linker` open-subject tagging](issues/183-per-layer-tree-support-and-infill-linker-exact-acceleration.md).

**Destination status (2026-11-22,
[`ShellClassification` post-t172 residual attribution and speedup screen](issues/180-shell-classification-post-t172-residual-screen.md)):
`PrePass::ShellClassification` is fully attributed with zero production edits
(`15/15` probed runs byte-identical to `t175/reference.json`, `3/3` qual
clean, `4/4` classic/arachne & supports-off/on call/vertex invariance, `100 %`
call reconciliation vs `t177`, `probe.patch` cleanly reverted, both guest
freshness gates exit `0`), and a 100 % `EXACT_BIT_IDENTICAL` four-mechanism
host bundle (`EXACT_BUNDLE`: `E1 + E2 + E3 + E4a`) cuts stage wall `−56.06 %`
(`benchy`: `470.77 → 206.83 ms`) and `−44.58 %` (`base`: `3,307.57 → 1,833.22 ms`),
clearing the literal 8-cell `0.80 %` gate at `6.94×–20.60×` floor with zero
output drift and crossing `< 1.0x` on both `benchy` supports-off cells alone
(`ESTIMATE ONLY`), graduating
[Exact `ShellClassification` acceleration: windowed `unsupported_span_areas`, lazy/clustered internal-bridge commit, parallel Pass-2 merge/diff, and $O((C+E)\log(C+E))$ external-bridge scoring](issues/184-shell-classification-exact-acceleration.md).**
Why `benchy` stalled at `5.10×` (`4.57×` Clipper-only) vs `base` `8.91×`:
**30.19 %** (`141.19 ms`) of `benchy` (`24.70 %` / `841.28 ms` `base`) is
serial (`B5d_ib_phaseB_com` `117.44 / 558.53 ms` — `gate_internal_bridge_sites`
does not use existing `cluster_candidate_layers`, re-runs full-layer
`gather_areas_w_depth` on `6/7` `None` recomputes on `base`, burns `111.60 ms`
unmarked pure-CPU $O(S \times A) + O(S^2)$ in `determine_bridging_angle` on
`benchy`, and runs a dead final `difference` in `internal_bridge_angles` for
`102.6 ms` on `base` — plus `B2b + B2d + B2e` serial Pass-2 merges/diffs
`19.24 / 254.85 ms`, where `68.9 / 75.7 %` of `internal_solid_fill` diffs have
no Pass-2 contrib); `B5b_ib_phaseA` (`128.96 ms` `benchy`, `2.57×` par;
`350.75 ms` `base`) is monopolized on `benchy` by one layer (`L1` = `128.89 ms`,
`ib_unsup_solids_grow = 102.41 ms` on `#3DBenchy` letter holes outputting `0`
polys while only `22.0 / 12.1 %` of `lower_fills` and `26.1 / 14.0 %` of
`lower_solids` verts lie inside `internal_solid_fill`'s padded bbox);
`B5c_ib_phaseB_pre` (`51.09 / 711.34 ms`) returns `None` on **`72.1 %` (`31/43`
`benchy`)** and **`93.2 %` (`109/117` `base`)** of layers, wasting **`53.5 %`
(`258.4 ms`)** and **`79.8 %` (`6,445.9 ms`)** of thread time on eager
whole-layer `gather` + `ib_comp_pos_offset` (`4,016.4 ms`, #1 Clipper site on
`base`); and `B4_ext_bridge` burns `80.0 / 834.1 ms` unmarked CPU in quadratic
`detect_bridging_direction_deg`. Canonical audit (`OrcaSlicerDocumented/src/libslic3r/`,
file + function only): `PrintObject::bridge_over_infill` uses
`clustered_layers_for_threads` + `AABBTreeLines::LinesDistancer`,
`PrintObject::discover_vertical_shells` runs parallel passes, and
`ClipperUtils.cpp::offset_expolygon_inner` offsets negative deltas per
`ExPolygon` (premise correction vs `t129`: `OrcaSlicerDocumented`'s
`PrintObject::bridge_over_infill` has neither `boundary_by_spacing` nor
`clip_clipper_polygon_with_subject_bbox_templ`). Offline 12-thread exact IR
replay (`240` `benchy` + `496` `base` layers, `M0_baseline` bit-identical):
**`EXACT_BUNDLE`** (`E1` parallel Pass-2 merge/diff + `E2` lazy/monotonic/clustered
IB commit + BVH/$O(S)$ angle + region-invariant cache + dead-op skip + `E3`
exact `11 * spacing` windowed `unsupported_span_areas` + `E4a` $O((C+E)\log(C+E))$
`detect_bridging_direction_deg`) is **`EXACT_BIT_IDENTICAL`** on all `736`
layers (`−56.06 %` `benchy`, `−44.58 %` `base`); `E5` (Pass-2 neighbor bbox
filter) is exact but speed-neutral; `E4b` (`89.9°`), `L1` (`13.97 µm`), and
prototype `L3` (`499.2 µm`) are falsified as exact; `L2` (per-`ExPolygon` Pass-2
negative offset, `0.174 / 0.825 µm` sub-micron scanbeam shift) clears `0.80 %`
marginally over `EXACT_BUNDLE` in only `2/8` cells (`0.37–0.76 %` `benchy`,
`0.67–0.69 %` `base-on`) and is parked cleanly in `## Not yet specified`.
**Route (`ESTIMATE ONLY`, `t174 × t175 × t180`):** `t184` clears `0.80 %` across
**8/8 cells at `6.94×–20.60×`** (`benchy-classic-off` `15.63 %` → **`0.9650x`**
ord / **`0.9432x`** acc; `benchy-arachne-off` `15.16 %` → **`0.9798x`** ord /
**`0.9028x`** acc; `base-classic-off` `16.48 %` → **`0.6420x`**;
`base-arachne-off` `15.25 %` → **`0.6713x`**; `benchy-classic-on` `7.76 %` →
`1.4481x`; `benchy-arachne-on` `7.53 %` → `1.2840x`; `base-classic-on`
`5.55 %` → `1.1529x`; `base-arachne-on` `5.74 %` → `1.1322x`); disjoint-stage
stack `t184 + t183 + t182` puts **6/8 cells `< 1.0x`** (`0.8477x / 0.9184x /
0.6228x / 0.6546x / 0.9023x / 0.8843x` ord, and `1.0649x` `benchy-arachne-on` /
`1.2291x` `benchy-classic-on` with `t181`). Evidence:
[`evidence/t180-shell-classification-residual/FINDINGS.md`](evidence/t180-shell-classification-residual/FINDINGS.md)
(`verify.py` PASS `7/7`, `--self-test` `15/15`). **Frontier (all 4 AFK,
unblocked):**
[`SupportGeometry` post-planner host tail attribution and reduction screen](issues/181-support-geometry-post-planner-host-tail.md),
[Host-side spatial hierarchical Phase-3 union with convex-containment pruning](issues/182-host-spatial-hierarchical-phase3-union.md),
[Exact per-layer guest acceleration: `tree-support` renderer fast paths and `infill-linker` open-subject tagging](issues/183-per-layer-tree-support-and-infill-linker-exact-acceleration.md),
and [Exact `ShellClassification` acceleration: windowed `unsupported_span_areas`, lazy/clustered internal-bridge commit, parallel Pass-2 merge/diff, and $O((C+E)\log(C+E))$ external-bridge scoring](issues/184-shell-classification-exact-acceleration.md).

**Destination status (2026-11-22,
[`SupportGeometry` post-planner host tail attribution and reduction screen](issues/181-support-geometry-post-planner-host-tail.md)):
the post-planner host tail (`212.91–214.61 ms` `benchy-on`, `1,939.31–1,946.49 ms`
`base-on`, `100 %` single-threaded `1.0×`) is fully attributed with zero
production edits (`22/22` runs byte-identical to `t175/reference.json`, `6/6`
`--instrument-stderr` qual clean, `2/2` supports-off inert, classic ≡ arachne
counters, `probe.patch` cleanly reverted, clean release `pnp_cli.exe` rebuilt,
both guest freshness checks exit `0`), and the 100 % exact, zero-tolerance-preserving
`M12345` host bundle (`M1` `print_plane_occupancy` reuse + `M2` bbox cull + `M3`
zero-clone aggregation + `M4` exact `i128` topological disjointness certificate +
`M5` order-preserving Rayon validation) cuts the tail `−99.66 %` (`210.83 → 0.71 ms`,
`benchy`) and `−99.85 %` (`1,971.68 → 2.95 ms`, `base`), clearing the standing
`t137` 4-cell planner gate at `7.57×–9.69×` floor (`7.59×–8.68×` probe scale) with
zero output drift and zero tolerance weakening, graduating
[Exact `SupportGeometry` post-planner host-tail acceleration: `print_plane_occupancy` reuse, zero-clone aggregation, and exact `i128` disjointness certificate](issues/185-support-geometry-post-planner-host-tail-acceleration.md).**
Premise clarification: WIT result decode (`crates/slicer-wasm-host/src/dispatch.rs::harvest_support_plan_ir`)
and WASM store drop run inside `runner.run_stage` *before*
`tree-support-planner::module_complete`; the entire post-planner tail executes
serially (`1.0×`) in `crates/slicer-runtime/src/prepass.rs::execute_prepass_with_instrumentation_collecting`
→ `crates/slicer-wasm-host/src/support_aggregation.rs::aggregate_support_plan_irs_degrading_with_attributed_diagnostics`
→ `crates/slicer-wasm-host/src/exact_z_query.rs::ExactZQueryService` (`99.0–99.5 %`
in three terms, reconciling within `0.065–0.54 ms` of `--instrument-stderr` and
`<= 0.018 ms` additive residual): (1) **`39.11–40.24 %`** (`85.67–85.70 ms`
`benchy`, `761.21–764.83 ms` `base`) in `ExactZQueryService::prefetch_cross_sections`
calling single-threaded `slicer_core::slice_mesh_ex` over `200` / `432` planes
that live verification proves are **`200/200` (`benchy`) and `432/432` (`base`)
— `100.0 %` polygon-for-polygon identical** to `SupportAnalysisIR::print_plane_occupancy`
already sitting on the blackboard (`0` missing planes; unseeded planes retain full
lazy `slice_mesh_ex` fallback); (2) **`57.44–59.06 %`** (`122.56–123.27 ms`
`benchy`, `1,138.45–1,149.51 ms` `base`) in serial `overlaps_any` running `2,894`
(`benchy`) / `5,292` (`base`) unfiltered Clipper2 `intersection` calls (`0` positive
overlaps), of which **`71.39 %` (`2,066`) / `60.45 %` (`3,199`)** have strictly
disjoint bounding boxes (`61.5–61.9 ms` / `462.6–464.8 ms`) and **`28.61 %` (`828`)
/ `39.55 %` (`2,093`)** are bbox-overlapping concave-bay / hole-interior pairs
(`60.2–60.5 ms` / `670.6–679.2 ms`); and (3) **`1.21–1.68 %`** (`3.4–3.6 ms` /
`23.4–24.4 ms`) in three redundant deep clones of `SupportPlanIR` + dead `identities`
map. Standalone 12-thread exact IR replay (`5` interleaved rounds, `M0_baseline`
`0.9902×` `benchy` / `1.0167×` `base` vs live probe, `7/7` modes bit-identical):
`M1235` (parallel Clipper2 without `M4`) hits `95.28 ms` on `base` but burns
`1,125 ms` CPU (`+67 %` vs serial `M123` — reproducing `t89`'s parallel-Clipper
CPU trap), whereas `M4`'s exact `i128` topological disjointness certificate
(homotopy invariance of `FillRule::NonZero` winding number with exact Clipper2
fallback on any crossing/touch/containment) proves **`828/828` (`benchy`) and
`2,093/2,093` (`base`)** disjoint (`0` Clipper2 calls), taking serial `M1234` to
`3.38 ms` (`−98.40 %`) / `15.03 ms` (`−99.24 %`, CPU `−99.21 %`) and parallel
`M12345` to **`0.71 ms` (`−99.66 %`, `295.7×`) / `2.95 ms` (`−99.85 %`, `667.9×`)**.
Zero-tolerance adversarial proof: **`10,918/10,918`** pairs (`8,186` real +
`2,732` synthetic adversarial cases down to `1 unit = 100 nm` edge grazes and hole
crossings) match raw Clipper2 with **`0` FN / `0` FP**. **Route (`ESTIMATE ONLY`,
`t174 × t175 × t177`):** `t185` cuts `benchy-classic-on` `−6.21 %` (`1.4726x`),
`benchy-arachne-on` `−6.06 %` (`1.3044x`), `base-classic-on` `−7.75 %` (`1.1260x`),
`base-arachne-on` `−6.97 %` (`1.1174x`); stacking `t185 + t182` closes **both
ordinary `base` supports-on cells `< 1.0x`** (**`0.9511x` / `0.9485x`** ord,
`0.9110x / 0.9107x` acc → **4/8**); stacking all four open implementation tracks
`t182 + t183 + t184 + t185` puts **6/8 cells `< 1.0x`** (`0.8477x / 0.9184x /
0.6228x / 0.6546x / 0.8077x / 0.8005x` ord) with `benchy-arachne-on` at `1.0651x`
and binding `benchy-classic-on` at `1.2294x`. Evidence:
[`evidence/t181-support-geometry-host-tail/FINDINGS.md`](evidence/t181-support-geometry-host-tail/FINDINGS.md)
(`verify.py` PASS `7/7`, `--self-test` `14/14`). **Frontier (all 4 AFK, unblocked
implementation & standing-gate tracks):**
[Host-side spatial hierarchical Phase-3 union with convex-containment pruning](issues/182-host-spatial-hierarchical-phase3-union.md),
[Exact per-layer guest acceleration: `tree-support` renderer fast paths and `infill-linker` open-subject tagging](issues/183-per-layer-tree-support-and-infill-linker-exact-acceleration.md),
[Exact `ShellClassification` acceleration: windowed `unsupported_span_areas`, lazy/clustered internal-bridge commit, parallel Pass-2 merge/diff, and $O((C+E)\log(C+E))$ external-bridge scoring](issues/184-shell-classification-exact-acceleration.md),
and [Exact `SupportGeometry` post-planner host-tail acceleration: `print_plane_occupancy` reuse, zero-clone aggregation, and exact `i128` disjointness certificate](issues/185-support-geometry-post-planner-host-tail-acceleration.md).

**Destination status (2026-11-22, [Host-side spatial hierarchical Phase-3 union with convex-containment pruning](issues/182-host-spatial-hierarchical-phase3-union.md)): landed (`worker-k8`) by explicit human KEEP ruling — literal standing `t137` 4-cell planner gate PASSED (4/4 supports-on PASS, 4/4 supports-off INERT-PASS; 20/20 ordinary wall & CPU wins, 12/12 accelerated wall wins), banking +13.85 / +14.00 % wall on `base` supports-on (`~1.052x / ~1.033x` est. ordinary, crossing `< 1.0x` at `~0.998x / ~0.997x` est. accelerated) and +1.90 / +2.41 % wall on `benchy` supports-on (`+4.58 / +3.81 %` acc); active reference currency advances to [`evidence/t182-spatial-hierarchical-phase3-union/reference.json`](evidence/t182-spatial-hierarchical-phase3-union/reference.json).**
Host-only `slicer_core::polygon_ops::union_spatial_tree` (`crates/slicer-core/src/polygon_ops_spatial_union.rs`, selected `worker-k8`: `k = 8`, `nested_par = false`), wired strictly into batched `ClipOperation::Union` when `subject.len() + clip.len() > 8` (`crates/slicer-wasm-host/src/host.rs::clip_polygons_batch` and native fallback `crates/slicer-sdk/src/host_batch.rs::clip_polygons_batch`; zero WIT/guest source edits). Sweep on `base-classic-on` (`{worker, nested} × k ∈ {4, 8, 16}`, 1 warmup + 3 rounds, load `4 → 5 %`): `worker-kN ≡ nested-kN` byte-identical across all three `k`; `worker-k8` (`24.262 s`, `+15.30 %` wall / `+23.60 %` CPU, tight spread `0.337 s`) sits within `0.23 %` of `nested-k4` while cutting the `Support` path delta 4.9× (`−0.242 mm` vs `−1.195 mm`). Standing `t137` gate (5 interleaved pairs/cell ord, 3-pair null, 3-pair acc; load `0–9 %`; `229/229` clean runs, `0` degraded, `0` code-1200): `base-classic-on` **+13.85 %** wall (`5/5`, `17.31×` floor, null `0.30 / 0.44 %`, CPU **+23.07 %**, acc **+14.64 / +24.20 %**), `base-arachne-on` **+14.00 %** (`5/5`, `17.51×`, null `0.55 / 1.71 %`, CPU **+22.62 %**, acc **+13.57 / +23.39 %**), `benchy-classic-on` **+1.90 %** (`5/5`, `2.37×`, null `0.51 / 4.80 %`, CPU **+7.25 %** = `7.1×` null any-pair CPU, acc **+4.58 / +4.79 %**; strict any-pair `INSIDE-NULL-ANY-PAIR` disclosed), `benchy-arachne-on` **+2.41 %** (`5/5`, `3.02×`, null `1.25 / 3.65 %`, CPU **+4.70 %**, acc **+3.81 / +3.73 %**; strict any-pair `INSIDE-NULL-ANY-PAIR` disclosed); 4/4 supports-off **INERT-PASS** (`−1.07…+1.64 %` ord) and **byte-identical to `t175`**. Output & 8-cell G-code audit: baseline reproduces `t175` **16/16**, candidate ord ≡ acc **16/16**; **0 / 1,226,364** non-support G1 motion lines moved XY (max `|dE|` ≤ `0.00048 mm`), zero `Support interface` movement, all shifts strictly inside `Support` (`−0.01777 mm` = `−0.000022 %` `benchy`, `−0.24238 mm` = `−0.000062 %` `base`). **Route (`ESTIMATE ONLY`, `t174 × t175 × t182`, not a ratio of record):** `base-classic-on` `1.221x → ~1.052x` ord / **`~0.998x` acc** (`76.6 %` cut closed; `+t185`: `~0.970x` ord), `base-arachne-on` `1.201x → ~1.033x` ord / **`~0.997x` acc** (`83.6 %` closed; `+t185`: `~0.961x` ord), `benchy-classic-on` (binding) `1.570x → ~1.540x` ord / `~1.528x` acc, `benchy-arachne-on` `1.389x → ~1.355x` ord / `~1.355x` acc. Official running tally stays **2/8 provisionally won** pending a fresh Orca scoreboard refresh. Evidence: [`evidence/t182-spatial-hierarchical-phase3-union/FINDINGS.md`](evidence/t182-spatial-hierarchical-phase3-union/FINDINGS.md) (`verify.py` PASS, `--self-test` `15/15`; `audit_t182.py --self-test` `7/7`). **Remaining frontier (3 AFK, unblocked exact implementation & standing-gate tracks, all measuring against `t182/reference.json`):** [Exact per-layer guest acceleration: `tree-support` renderer fast paths and `infill-linker` open-subject tagging](issues/183-per-layer-tree-support-and-infill-linker-exact-acceleration.md), [Exact `ShellClassification` acceleration: windowed `unsupported_span_areas`, lazy/clustered internal-bridge commit, parallel Pass-2 merge/diff, and $O((C+E)\log(C+E))$ external-bridge scoring](issues/184-shell-classification-exact-acceleration.md), and [Exact `SupportGeometry` post-planner host-tail acceleration: `print_plane_occupancy` reuse, zero-clone aggregation, and exact `i128` disjointness certificate](issues/185-support-geometry-post-planner-host-tail-acceleration.md).

**Destination status (2026-11-23, [Exact per-layer guest acceleration: `tree-support` renderer fast paths and `infill-linker` open-subject tagging](issues/183-per-layer-tree-support-and-infill-linker-exact-acceleration.md)): landed — literal standing all-8-cell `0.80 %` paired gate PASSED (`8/8 PASS`: `+2.20…+9.59 %` ordinary wall, `39/40` wins, `40/40` CPU wins `+3.10…+16.54 %`, accelerated `+1.85…+7.97 %` wall / `+3.53…+15.92 %` CPU), with `224/224` measured runs 100 % byte-identical to `t182/reference.json` (zero reference change, zero geometry exceptions, zero gate waivers), crossing `< 1.0x` on BOTH ordinary `base` supports-on cells (`~0.973x / ~0.966x` est.) and `benchy-arachne-off` accelerated (`~0.989x` est.).**
Production exact bundle across five crates (zero WIT changes): `TS-1a` (`crates/slicer-core/src/paint_policy.rs::has_support_paint` + `crates/slicer-sdk/src/traits.rs::PaintRegionLayerView::paint_policy_for` empty-paint guard & `Blocked` early exit), `TS-1b`/`TS-2` (`modules/core-modules/tree-support/src/lib.rs::skeleton_wall_count` precomputed AABBs + `count <= best` skip; `unify_role_regions` / `expolygons_pairwise_bbox_disjoint` safe bypass of redundant in-guest `union_ex`), `IL-1` (`vendor/clipper2-rust/src/engine.rs` & `engine_public.rs` native open-subject `source_id: u32` tagging + `Clipper64::add_open_subject_tagged` / `execute_open_tagged` wired into `crates/slicer-core/src/polygon_ops.rs::PreparedPolylineClip::clip_batched`, deleting 249 lines of geometric re-attribution and ambiguity repairs), `IL-2` (`modules/core-modules/infill-linker/src/orchestrate.rs` skip dead `source_segments` in `link_region_group` + bitwise-exact `O(1)` `uniform_point_metadata`), and `IL-3` (`modules/core-modules/infill-linker/src/graph.rs` & `connect.rs` precomputed `BoundaryEdge`/AABBs, cross-ring `project_bounded`, hole AABB guard, `O(E)` sorted-ring circular `nearest_pair_candidates`). Deterministic `--profile` guest fuel (`16/16` byte-identical): `com.core.tree-support` **−96.42 % (`27.95×`, `41.94 → 1.50 B`)** `benchy` / **−97.06 % (`33.99×`, `388.78 → 11.44 B`)** `base` (strict pairwise AABB-disjointness preserves unconditional soundness on merged overlapping inputs); `com.core.infill-linker` **−74.03 / −74.56 / −79.34 / −79.05 %** (`3.85×–4.84×`, beating `t179` screen); total guest fuel **−23.92…−40.17 %** supports-off / **−69.73…−73.83 %** supports-on. Standing 8-cell paired gate (unsandboxed, `RAYON_NUM_THREADS=12`, load `ab` `2 → 3 %`, `null` `1 → 9 %`, `accel` `0 → 1 %`; `224/224` clean runs, `0` degraded, `0` code-1200, `0` hash mismatches): `benchy-classic-off` **+9.59 %** (`5/5`, `11.99×` floor, null `0.90/1.86 %`, CPU **+6.24 %**, acc **+7.23/+3.87 %**), `benchy-classic-on` **+5.60 %** (`4/5`, `7.00×`, null `0.68/1.39 %`, CPU **+16.54 %**, acc **+7.54/+15.92 %**), `benchy-arachne-off` **+8.36 %** (`5/5`, `10.45×`, null `1.22/1.75 %`, CPU **+8.45 %**, acc **+7.09/+7.06 %**), `benchy-arachne-on` **+5.73 %** (`5/5`, `7.16×`, null `3.29/3.33 %`, CPU **+14.60 %**, acc **+6.77/+15.47 %**), `base-classic-off` **+6.58 %** (`5/5`, `8.23×`, null `0.90/1.53 %`, CPU **+3.10 %**, acc **+2.39/+3.59 %**), `base-classic-on` **+7.43 %** (`5/5`, `9.29×`, null `0.85/1.00 %`, CPU **+15.40 %**, acc **+7.27/+15.06 %**), `base-arachne-off` **+2.20 %** (`5/5`, `2.75×`, null `0.49/3.56 %` — `4.47×` null median, strict any-pair `INSIDE-NULL-ANY-PAIR` disclosed, CPU **+3.34 %** = `4.25×` null any-pair CPU, acc **+1.85/+3.53 %**), `base-arachne-on` **+6.46 %** (`5/5`, `8.07×`, null `0.59/4.29 %`, CPU **+14.24 %**, acc **+7.97/+14.66 %**). **Route (`ESTIMATE ONLY`, `t174 × t175 × t182 × t183`, not a ratio of record):** `base-classic-off` **`0.718x`** / `0.747x`, `base-arachne-off` **`0.775x`** / `0.760x`, `base-classic-on` **`0.973x`** / `0.926x` (**crosses `< 1.0x`!**), `base-arachne-on` **`0.966x`** / `0.917x` (**crosses `< 1.0x`!**), `benchy-classic-off` `1.034x` / `1.037x` (`76.3 %` of gap closed), `benchy-arachne-off` `1.058x` / **`0.989x`**, `benchy-arachne-on` `1.277x` / `1.264x`, binding `benchy-classic-on` `1.454x` / `1.413x`. Official running tally stays **2/8 provisionally won** pending a fresh Orca scoreboard refresh. Active reference currency remains [`evidence/t182-spatial-hierarchical-phase3-union/reference.json`](evidence/t182-spatial-hierarchical-phase3-union/reference.json). Evidence: [`evidence/t183-per-layer-guest-exact-accel/FINDINGS.md`](evidence/t183-per-layer-guest-exact-accel/FINDINGS.md) (`verify.py` PASS, `--self-test` `16/16`). **Remaining frontier (2 AFK, unblocked exact implementation & standing-gate tracks, both measuring against `t182/reference.json`):** [Exact `ShellClassification` acceleration: windowed `unsupported_span_areas`, lazy/clustered internal-bridge commit, parallel Pass-2 merge/diff, and $O((C+E)\log(C+E))$ external-bridge scoring](issues/184-shell-classification-exact-acceleration.md) and [Exact `SupportGeometry` post-planner host-tail acceleration: `print_plane_occupancy` reuse, zero-clone aggregation, and exact `i128` disjointness certificate](issues/185-support-geometry-post-planner-host-tail-acceleration.md).

**Destination status (2026-11-23, [Exact `ShellClassification` acceleration: windowed `unsupported_span_areas`, lazy/clustered internal-bridge commit, parallel Pass-2 merge/diff, and $O((C+E)\log(C+E))$ external-bridge scoring](issues/184-shell-classification-exact-acceleration.md)): landed — literal standing all-8-cell `0.80 %` gate PASSED 8/8 on the pre-registered pooled verdict, `448/448` measured runs byte-identical to `t182/reference.json` (zero reference change, zero geometry exceptions, zero gate waivers); both benchy supports-off cells cross `< 1.0x` on the estimate (`~0.877x / ~0.885x` ord).** Pooled ordinary walls `+15.15 / +8.75 / +16.35 / +8.49 %` benchy and `+12.31 / +4.99 / +10.72 / +5.76 %` base (classic-off / classic-on / arachne-off / arachne-on), CPU lower in all eight. Route (`ESTIMATE ONLY`, `t174 × t175 × t182 × t183 × t184`, not a ratio of record): benchy `0.877 / 1.327 / 0.885 / 1.169x` ord, base `0.630 / 0.925 / 0.692 / 0.911x` ord. Official running tally stays **2/8 provisionally won** pending a fresh Orca scoreboard. Active reference currency remains [`evidence/t182-spatial-hierarchical-phase3-union/reference.json`](evidence/t182-spatial-hierarchical-phase3-union/reference.json). Evidence: [`evidence/t184-shell-classification-exact-accel/FINDINGS.md`](evidence/t184-shell-classification-exact-accel/FINDINGS.md). **Remaining frontier (1 AFK):** [Exact `SupportGeometry` post-planner host-tail acceleration: `print_plane_occupancy` reuse, zero-clone aggregation, and exact `i128` disjointness certificate](issues/185-support-geometry-post-planner-host-tail-acceleration.md).

## Notes

Domain: slice throughput on Windows dev hardware. Canonical evidence base:
`docs/specs/perf-vs-orca/evidence/PERF-HANDOFF.md` (measurement recipe and traps) and
`docs/specs/perf-vs-orca/evidence/perf-emit-walls/FINDINGS.md` (current attribution, incl. its addendum).
Every number in this tracker is a ledger fact — re-derive at the point of use.

- **Ledger caveat:** `docs/specs/perf-vs-orca/evidence/PERF-HANDOFF.md`'s DEV ids drift. Its "DEV-166" is
  the `check_split_owner` unbounded recursion — ledger row **DEV-173**; its
  "DEV-167" is the tree-support routing-cell defect — ledger row **DEV-174**.
  Re-derive ids from `docs/DEVIATION_LOG.md`, never from the handoff.
- **Evidence policy:** every claim's headline evidence lives in
  `docs/specs/perf-vs-orca/evidence/` (in-repo) and must be re-derivable from a
  plain checkout. A gitignored raw capture tree may be cited only as the
  fuller, optional source that the in-repo reduced set is derived from (and
  the reducer script must live in-repo); verification scripts must exit 0
  against the in-repo set without the raw tree. The only other gitignored
  references allowed are the heavy model fixtures `tmp/3dbenchy.stl` and
  `tmp/base.stl` — user-supplied and licence-encumbered, they must never be
  committed.
- **The accelerated host build's `c1xx C1083` was a path-spelling bug, not a
  toolchain bug (corrected 2026-11-18,
  [t162](issues/162-accelerated-host-verbatim-path.md)).** Tickets t142–t161
  recorded it as "the controlled-driver `RUSTC` wrapper + MSVC 14.51 mangles the
  `libmimalloc-sys` verbatim source path" and closed every accelerated arm on
  that sentence. It was never the wrapper, never MSVC 14.51 and never
  libmimalloc: `workspace_root()` returned `cwd.canonicalize()`, whose Windows
  spelling is `\\?\D:\...`, and `dist` set `CARGO_TARGET_DIR` from it, so
  `cc-rs` handed `cl.exe` an extended-length source path. Fixed at the root; the
  arm builds and is output-identical 16/16. **Read any pre-t162 "the accelerated
  host is unbuildable" sentence as historical.**

- **OrcaSlicer tool boundary (2026-11-17, [t161](issues/161-post-t158-scoreboard-refresh.md)).**
  The installed rig binary is **2.5.1** (`%APPDATA%/OrcaSlicer/OrcaSlicer.conf`
  header `OrcaSlicer 2.5.1`; installed binaries dated 2026-11-13 23:59:44 –
  2026-11-14 00:00:44; `orca-slicer.exe` sha256 `5013b1bd…`, 292,160 bytes), not
  the **2.5.0** recorded in the canonical-source-pin note below. The vendor
  profile cache stamp is unchanged at `02.08.00.16`, so the loaded profile tree
  and the asserted matched keys are unchanged — **only the executable moved**.
  Rule: an Orca number is either **pre-2026-11-13 23:59 (2.5.0)** or
  **post-2026-11-13 23:59 (2.5.1)**; never mixed, and never composed across the
  boundary. t145's and t148's boards are 2.5.0; t161's board is the first full
  board on 2.5.1. The rig gates the *profile* version and never records the
  *application* version, which is how this swap went unnoticed — a future
  campaign must stamp the executable identity.

- **Canonical source pin (2026-11-08, t128 ruling).** The local checkout
  `D:\slicerProject\Orca(pnp_gui)` is on **`v2.5.0` =
  `40ecc66f78ed72dc368e0c1ebe40d77f00cc8a19`** (2026-11-07, detached HEAD); its
  parent `4f33435636` is the last functional commit and the tag itself changes
  only `version.inc` and the flatpak metainfo. Every parity ticket from here
  cites canonical at this pin, by file + function, never line numbers. The
  rig's measured binary is the installed `C:\Program Files\OrcaSlicer`
  (files dated 2026-11-06 23:00, embedded version `2.5.0`) — one day before the
  tag and not byte-proven identical, so reconcile source vs binary before
  quoting an op-level comparison as a wall claim. The fork's previous state was
  `pnp/main` @ `9f329f13ae` (whose tree records the submodule at `a63fd14e`),
  with the `pinch_n_print_cli` worktree at
  `323708021aef6f20502066039d22b3568544b1`. Restore with `git switch pnp/main`,
  then `git -C pinch_n_print_cli checkout 32370802` for that exact state;
  `git submodule update` would instead restore the recorded `a63fd14e`.
- **The Orca baseline is RE-DERIVED (ticket 103, 2026-10-31 campaign).** The
  2026-10-27 OrcaSlicer **2.5.0** install (`v2.5.0-beta`) ships the vendor as one
  binary preset cache (`resources/profiles/BBL.opc`) and resolves `--load-settings`
  against the loaded preset bundle, so the rig's file-path form failed. The rig now
  loads a version-gated source-manifest vendor tree
  (`evidence/orca-rig-repair/prepare_orca_vendor.ps1` → `target/orca-vendor/`) and
  applies the matched keys as CLI overrides. The repair falsified the archived Orca
  side (its runs carried Orca's hardcoded filament defaults — density 0, max
  volumetric speed 2, first-layer temp 200), and [ticket 103](issues/103-orca-rebaseline-repaired-rig.md)
  has now re-measured **all eight cells on both arms today-vs-today**: **Orca wins
  all 8 at 3.52x–8.25x**, fresh Orca is **−24…−35 %** on benchy but **+6…+18 %** on
  base, and the t98 inference "every archived multiple is optimistic for PnP" is
  **falsified for the four base cells**. The 2.5 default flips (`enable_arc_fitting`,
  `elefant_foot_compensation` 0 → 0.15, `brim_width` 0 → 5) are **pinned back** in
  `New-OrcaOverrideArgs` and asserted per run, so no ratio may be quoted from before
  this take. Fresh Orca measurement is available again.

**Next-session handoff (e2e dev loop):** `docs/specs/perf-vs-orca/HANDOFF.md` —
the 755 s bucket's critical path (`module_plan` 373 s, `typed_projection`
345 s), ranked next moves, measurement protocol and traps.

**Lightning test-leg screen:** [ticket 120](issues/120-lightning-test-leg-concurrency.md)
parallelizes independent test legs with the existing Rayon pool. One warm,
shared-machine pair measured **798.61 s → 700.16 s** with the same 172 passed
and one ignored; active browser load means this is not an idle-box benchmark.
See its [evidence](evidence/t120-lightning-parallel-legs/FINDINGS.md) for the
conditions, correctness gates and limits. Ticket 119 remains open.

**Post-merge reconciliation (2026-11-04, ticket 115):** the merge `a5e9a403` is
**output-neutral on the matched job** — all 8 cells grow by exactly the 34-byte
line `; prepared_plan_diagnostic = none`, and a line diff on the four classic
cells confirms that is the only change — but it **regresses the per-layer
phase 1.9x–6.0x** (benchy 2,608 → 5,816 ms; base 15,341 → 92,418 ms) with
process CPU **+17…+54 % in 12/12 paired runs**; the prepass is unchanged
(70,662 → 70,463 ms, −0.3 %). Call counts are identical while small modules jump
to a near-uniform ~28.5 s accumulated (`fuzzy-skin` 162 → 56,068 ms), and the
per-call cost grows with layer index. The accelerated dist build does not show
it (2,318 ms per-layer on benchy classic-off vs ordinary 4,807–5,860 ms, same
hash), which points at a build-mode-sensitive host path. **No ratio of record
may be quoted from the pre-fix merged tree** — t103/t106 ratios and t104/t110
splits are pre-merge. The post-merge scoreboard is now recorded by
[ticket 121](issues/121-post-merge-per-layer-dispatch-regression.md) at
**2.78x–5.11x ordinary** (accelerated 2.76x–5.10x) after the per-layer fix. See
[ticket 115](issues/115-post-merge-reconciliation.md)
and [`evidence/t115-post-merge/`](evidence/t115-post-merge/FINDINGS.md), and
[ticket 121](issues/121-post-merge-per-layer-dispatch-regression.md).

Skills to consult: `diagnosing-bugs` for regressions, `debug-pipeline` and
`visual-debug` per the repo `AGENTS.md`; the repo test discipline (narrow runs,
`cargo xtask test`, guest freshness) governs any code change.

**Current selection scope (2026-11-23, the t183 take):** the human said
"/wayfinder continue the perf vs orca map" (no ticket named); **183 was the
first unblocked, unclaimed AFK frontier ticket in numerical order**, so it was
claimed, implemented (`TS-1a`, `TS-1b`, `TS-2`, `IL-1`, `IL-2`, `IL-3` across
`vendor/clipper2-rust`, `crates/slicer-core`, `crates/slicer-sdk`,
`modules/core-modules/tree-support`, and `modules/core-modules/infill-linker`;
zero WIT changes, zero geometry exceptions, zero gate waivers), verified on the
**literal standing all-8-cell `0.80 %` paired gate** (`8/8 PASS`:
`+2.20…+9.59 %` ordinary wall, `39/40` wins, `40/40` CPU wins
`+3.10…+16.54 %`, accelerated `+1.85…+7.97 %` wall / `+3.53…+15.92 %` CPU,
`224/224` measured runs 100 % byte-identical to `t182/reference.json`,
deterministic `--profile` guest fuel `−96.42 / −97.06 %` `tree-support` and
`−74.03…−79.34 %` `infill-linker`, crossing `< 1.0x` on both ordinary `base`
supports-on cells at `~0.973x / ~0.966x` est.), and **resolved & landed** with
active reference currency unchanged at
[`evidence/t182-spatial-hierarchical-phase3-union/reference.json`](evidence/t182-spatial-hierarchical-phase3-union/reference.json).
**Frontier (2 AFK exact implementation & standing-gate tracks, both measuring against `t182/reference.json`):**
[Exact `ShellClassification` acceleration: windowed `unsupported_span_areas`, lazy/clustered internal-bridge commit, parallel Pass-2 merge/diff, and $O((C+E)\log(C+E))$ external-bridge scoring](issues/184-shell-classification-exact-acceleration.md)
and [Exact `SupportGeometry` post-planner host-tail acceleration: `print_plane_occupancy` reuse, zero-clone aggregation, and exact `i128` disjointness certificate](issues/185-support-geometry-post-planner-host-tail-acceleration.md).

**Previous selection scope (2026-11-22, the t182 take):** the human said
"/wayfinder continue the perf vs orca map" (no ticket named); **182 was the
first unblocked, unclaimed AFK frontier ticket in numerical order**, so it was
claimed, implemented (`crates/slicer-core/src/polygon_ops_spatial_union.rs`,
wired strictly into `clip_polygons_batch` `ClipOperation::Union` when
`subject.len() + clip.len() > 8`; zero WIT/guest source edits), swept across
`{worker, nested} × k ∈ {4, 8, 16}` (`worker-k8` selected), measured on the
standing `t137` 4-cell planner gate (`+13.85 / +14.00 %` `base` supports-on,
`+1.90 / +2.41 %` `benchy` supports-on, `20/20` ord wall & CPU wins, `12/12`
acc wall wins, `4/4` supports-off `INERT-PASS` and byte-identical to `t175`,
`0/1,226,364` non-support part XY diffs, `Support` only `−0.01777 mm` benchy /
`−0.24238 mm` base), and **resolved & landed by explicit human KEEP ruling**,
advancing active reference currency to
[`evidence/t182-spatial-hierarchical-phase3-union/reference.json`](evidence/t182-spatial-hierarchical-phase3-union/reference.json).
**Frontier (3 AFK exact implementation & standing-gate tracks, all measuring against `t182/reference.json`):**
[Exact per-layer guest acceleration: `tree-support` renderer fast paths and `infill-linker` open-subject tagging](issues/183-per-layer-tree-support-and-infill-linker-exact-acceleration.md),
[Exact `ShellClassification` acceleration: windowed `unsupported_span_areas`, lazy/clustered internal-bridge commit, parallel Pass-2 merge/diff, and $O((C+E)\log(C+E))$ external-bridge scoring](issues/184-shell-classification-exact-acceleration.md),
and [Exact `SupportGeometry` post-planner host-tail acceleration: `print_plane_occupancy` reuse, zero-clone aggregation, and exact `i128` disjointness certificate](issues/185-support-geometry-post-planner-host-tail-acceleration.md).

**Previous selection scope (2026-11-22, the t181 take):** the human said
"/wayfinder continue the perf vs orca map" (no ticket named); **181 was the
first unblocked, unclaimed AFK frontier ticket in numerical order**, so it was
claimed and resolved as an **attribution + candidate screen (zero production
code edits; `probe.patch` cleanly reverted, clean release `pnp_cli.exe` rebuilt,
both guest freshness gates exit `0`)**. Result: attributed the `100 %`
single-threaded (`1.0×`) post-planner host tail (`212.91–214.61 ms` `benchy-on`,
`1,939.31–1,946.49 ms` `base-on`; `22/22` runs byte-identical to `t175`, `6/6`
qual clean, `2/2` supports-off inert, classic ≡ arachne counters, `632/632`
`print_plane_occupancy` planes polygon-for-polygon identical), proved via
12-thread exact IR replay + a `10,918`-pair adversarial suite (`0` FN / `0` FP)
that the 100 % exact, zero-tolerance-preserving `M12345` host bundle cuts the
tail `−99.66 %` (`benchy`) / `−99.85 %` (`base`) and clears the standing `t137`
4-cell planner gate at `7.57×–9.69×` floor (closing both ordinary `base`
supports-on cells `< 1.0x` when stacked with `t182`, `ESTIMATE ONLY`), and
graduated
[Exact `SupportGeometry` post-planner host-tail acceleration: `print_plane_occupancy` reuse, zero-clone aggregation, and exact `i128` disjointness certificate](issues/185-support-geometry-post-planner-host-tail-acceleration.md).
**Frontier (4 AFK implementation & standing-gate tracks):**
[Host-side spatial hierarchical Phase-3 union with convex-containment pruning](issues/182-host-spatial-hierarchical-phase3-union.md),
[Exact per-layer guest acceleration: `tree-support` renderer fast paths and `infill-linker` open-subject tagging](issues/183-per-layer-tree-support-and-infill-linker-exact-acceleration.md),
[Exact `ShellClassification` acceleration: windowed `unsupported_span_areas`, lazy/clustered internal-bridge commit, parallel Pass-2 merge/diff, and $O((C+E)\log(C+E))$ external-bridge scoring](issues/184-shell-classification-exact-acceleration.md),
and [Exact `SupportGeometry` post-planner host-tail acceleration: `print_plane_occupancy` reuse, zero-clone aggregation, and exact `i128` disjointness certificate](issues/185-support-geometry-post-planner-host-tail-acceleration.md).

**Previous selection scope (2026-11-22, the t180 take):** the human said
"/wayfinder continue the perf vs orca map" (no ticket named); **180 was the
first unblocked, unclaimed AFK frontier ticket in numerical order**, so it was
claimed and resolved as an **attribution + candidate screen (zero production
code edits; `probe.patch` cleanly reverted, clean release `pnp_cli.exe` rebuilt,
both guest freshness gates exit `0`)**. Result: attributed the full post-t172
`PrePass::ShellClassification` residual (`15/15` byte-identical to `t175`,
`100 %` call reconciliation vs `t177`, `4/4` classic/arachne & supports-off/on
invariance), proved via 12-thread exact IR replay (`240` `benchy` + `496` `base`
layers) that `EXACT_BUNDLE` (`E1 + E2 + E3 + E4a`) cuts stage wall `−56.06 %`
(`benchy`) and `−44.58 %` (`base`) with **`EXACT_BIT_IDENTICAL`** output
(clearing the literal 8-cell `0.80 %` gate at `6.94×–20.60×` and crossing
`< 1.0x` on both `benchy` supports-off cells alone, `ESTIMATE ONLY`), falsified
`E4b`/`L1`/`L3` as exact, parked sub-micron `L2` (`2/8` marginal cells) in
`## Not yet specified`, and graduated
[Exact `ShellClassification` acceleration: windowed `unsupported_span_areas`, lazy/clustered internal-bridge commit, parallel Pass-2 merge/diff, and $O((C+E)\log(C+E))$ external-bridge scoring](issues/184-shell-classification-exact-acceleration.md).
**Frontier:**
[`SupportGeometry` post-planner host tail attribution and reduction screen](issues/181-support-geometry-post-planner-host-tail.md),
[Host-side spatial hierarchical Phase-3 union with convex-containment pruning](issues/182-host-spatial-hierarchical-phase3-union.md),
[Exact per-layer guest acceleration: `tree-support` renderer fast paths and `infill-linker` open-subject tagging](issues/183-per-layer-tree-support-and-infill-linker-exact-acceleration.md),
and [Exact `ShellClassification` acceleration: windowed `unsupported_span_areas`, lazy/clustered internal-bridge commit, parallel Pass-2 merge/diff, and $O((C+E)\log(C+E))$ external-bridge scoring](issues/184-shell-classification-exact-acceleration.md).

**Previous selection scope (2026-11-21, the t176 take):** the human said
"Continue the perf vs orca map" (no ticket named); **176 was the sole open
frontier ticket**, so it was claimed and resolved as a **HITL grilling ruling
(zero production code edits; tracker + `GLOSSARY.md` only)**. Result: Option C
amends `## Destination` so cross-engine cell closure uses unstarved median wall
clock on the **binding arm** (`pnp-ordinary`, production default;
`pnp-accelerated` disclosed beside it) guarded by the **CPU integrity guard**
plus a separate PnP/Orca CPU efficiency disclosure (within-PnP **candidate A/B
CPU corroboration** unchanged); margin-tiered provisional closure (`n=3` strict
separation with 0 starvation at `ratio <= 0.85x`, `>= 4` retained unstarved
runs with strict separation for close cells and the final single-HEAD 8/8
closing board, allowing starved-outlier exclusion per `SCOREBOARD.md` while
`>= 4` unstarved runs remain); provisional wins carry forward unless regressed
or reversed by a later Orca board; running tally moves **0/8 → 2/8 provisionally
won** (`base-classic-off` 0.769x, `base-arachne-off` 0.792x); Destination stays
`< 1.0x` (8/8) with a fresh `<= 0.50x` (2× faster) effort planned afterward.
Added 4 terms to `GLOSSARY.md` and graduated four structured AFK tickets.
**Frontier:** [177](issues/177-post-t175-full-matrix-attribution.md)
(AFK, unblocked), [178](issues/178-planner-phase3-union-attribution.md) (AFK,
unblocked), [179](issues/179-per-layer-tree-support-and-infill-linker-screen.md)
(AFK, blocked by `177`), and
[180](issues/180-shell-classification-post-t172-residual-screen.md) (AFK,
blocked by `177`).

**Previous selection scope (2026-11-21, the t175 take):** the human said
"Continue the perf vs orca map" (no ticket named); **175 was first on the
frontier** (leaving 176 unclaimed), so it was claimed and resolved as a
**measured planner candidate (guest-only exact per-layer collision tile index,
`2 mm` chosen by sweep), landed by explicit human KEEP ruling**. Result: clears
the standing `t137` 4-cell planner gate in **4/4 supports-on** (**+31.32 /
+30.90 %** base, **+16.06 / +16.62 %** benchy, 5/5 wins, 20.1–39.2× floor, CPU
+25.36…+40.86 %, acc +15.84…+31.53 %; **4/4 supports-off INERT-PASS**), beating
t173's host-only ceiling on every supports-on cell at a **130.5 ms** build+query
cost (`−99.15 %` clip points, `−22.0 %` planner guest fuel). Output: **6/8
byte-identical** to t172 (including both benchy supports-on); only the 2 `base`
supports-on cells move by **1 µm** on 4 XY vertices in Support/Support interface
(−1 B, 7 and 5 lines, 0 part movement), exhaustively proven 9/9 caused by
Clipper2 `add_new_intersect_node` distant-scanbeam clamping; active reference
set updated for those two hashes only (`t175/reference.json`, ordinary ≡
accelerated 16/16). Phase-3 union (~92 % of remaining `host:native` clip thread)
stays unnamed in **Not yet specified**.
**Frontier:** [176](issues/176-destination-cpu-corroboration-ruling.md) (HITL
grilling, unblocked).

**Previous selection scope (2026-11-21, the t174 take):** the human said
"Continue the perf vs orca map" (no ticket named); **174 was first on the
frontier** (leaving 175 unclaimed), so it was claimed and resolved as a
**measured three-tool eight-cell scoreboard refresh: zero production edits,
no reference move**. Result: on a clean quiet-box interleaved board
(`t174-quiet-board`, 96 rows, 0/72 starvation flags, 16/16 byte-identical to
t172's currency, 10/10 matched Orca keys), PnP sits at **0.769x–1.870x**
ordinary (**0.766x–1.916x** accelerated) and decisively beats OrcaSlicer on
wall clock in **2 of 8** cells with strict separation (`base classic-off`
0.769x / 0.766x, `base arachne-off` 0.792x / 0.774x), while burning
**1.81x–2.13x** Orca's process CPU via ~5.2x vs ~1.9–2.2x concurrency — so the
official Destination tally stays **0/8** pending a human ruling. The binding
cell shifts to `benchy classic-on` (**1.870x / 46.5 % cut** ord, **1.916x /
47.8 %** acc); all four supports-off cells cross the ~1.5x below-fold park
bound; an earlier loaded-box take is disclosed and preserved as
`reduced/loaded-window.json`. Graduated
[176](issues/176-destination-cpu-corroboration-ruling.md).
**Frontier:** [175](issues/175-tiled-collision-carve.md) (AFK, unblocked) and
[176](issues/176-destination-cpu-corroboration-ruling.md) (HITL grilling,
unblocked).

**Previous selection scope (2026-11-20, the t171 take):** the human said
"Continue the perf vs orca map" (no ticket named); **171 was first on the
frontier** (leaving 172, which needs a human ruling first, unclaimed), so it was
claimed and resolved as a **re-price: no production edit, no candidate, no
reference move**. Result: the phase split re-derives (prepass 55.9–83.7 %,
per-layer 12.0–34.7 %), the stage split names two clusters, the per-op census
finds the supports-on cells are **boolean**-bound (clip 53.1–60.9 % of host
clipper thread-time) while the supports-off cells are offset-bound
(42.9–51.2 %), and S5/S6 turn out to be **landed keeps with no residual worth a
ticket**. Two ticket premises were corrected (no `PrePass::InternalBridges`
stage; S5/S6 are not candidates). Graduated
[173](issues/173-support-geometry-boolean-volume.md).
**Frontier:** [172](issues/172-port-offset-concave-crossing.md) (HITL ruling
first, then the standing gate) and
[173](issues/173-support-geometry-boolean-volume.md) (AFK, unblocked).

**Previous selection scope (2026-11-20, the t169 take):** the human said
"Continue the perf vs orca map" (no ticket named); **169 was first on the
frontier** (leaving 170 unclaimed), so it was claimed and resolved as a
**measured candidate that passes the gate in 8/8 and lands**: canonical
pre-offset edge decimation in `polygon_ops`, +21.9…+25.1 % wall on the base
cells, +5.7…+15.2 % on benchy, 5/5 pairs each, CPU lower 8/8 on both arms, and
output moving 8/8 by design. The per-layer fence (t128) was put to the human
in-session with these numbers and the **fifth t81-style exception** was granted;
the change is committed as `9c3125d5`. t169's set is the currency.
**Frontier:** [170](issues/170-clipper2-2-0-1-kernel-port-screen.md) and
[171](issues/171-post-t169-prepass-reprice.md) (both AFK, unblocked).

**Previous selection scope (2026-11-19, the t168 take):** the human said
"Continue the perf vs orca map" (no ticket named); **168 was first on the
frontier** (leaving 169 and 170 unclaimed), so it was claimed and resolved as a
**canonical-parity fix that lands, with no measurable wall**: the bridge
opening's auto sentinel is resolved through one shared policy, bridge path falls
8/8, CPU is lower 8/8 on both arms, the anchored direction is re-derived on
12/48 / 13/61 / 2/7 bridge layers, and the wall gate fails 0/4 `base` with the
supports-off medians inside their null bands. t166's reference set is
superseded; **t168's set is the currency**. **Frontier:**
[169](issues/169-canonical-offset-edge-decimation-and-flat-opening.md),
[170](issues/170-clipper2-2-0-1-kernel-port-screen.md) (both AFK, unblocked,
independent).

**Previous selection scope (2026-11-19, the t167 take):** the human said
"Continue the perf vs orca map" (no ticket named); **167 was first on the
frontier** (leaving 168 unclaimed), so it was claimed and resolved as a
**canonical-side measurement (zero PnP production change): Branch A fires
(`canonical_ge_3x_faster_on_hot_offset_sites`)**. Headline: OrcaSlicer `v2.5.0`
(`40ecc66f78`) already migrated off Clipper1 onto Clipper2 2.0.1 (`222c6a2df5`)
and is **4.96x–5.02x** faster on `offset` (**5.16x–5.78x** 12-way loaded;
`IB_DEEP_NEG` **12.28–12.76x**, `IB_DEEP_POS` **4.56–4.64x**, `base-t68sites`
**3.92–3.97x** single / **4.87–5.12x** loaded) and **2.45x–2.59x** faster on
`opening_ex` (**2.79x–3.26x** loaded on canonical `0.04 mm Miter 3`), while
booleans remain at parity (**0.79x–1.41x**). 3-way ablation isolates
`append_offset_path` edge decimation (`0.005 * |delta|`, **3.56x–8.27x** on
`IB_DEEP_*`) plus Clipper2 2.0.1 / flat-path `offset2_ex` (**1.72x–1.76x** on
`SH_OPEN_*_CANON`) and rules out Clipper1 (slower on offsets/booleans + sliver
inversion bug on L133/L493). Active reference set unchanged (`t166`).
**Frontier:** [168](issues/168-bridge-opening-auto-sentinel.md),
[169](issues/169-canonical-offset-edge-decimation-and-flat-opening.md), and
[170](issues/170-clipper2-2-0-1-kernel-port-screen.md) (all AFK, unblocked).

**Previous selection scope (2026-11-19, the t166 take):** the human said
"Continue the perf vs orca map" (no ticket named); **166 was first on the
frontier** (165 had just resolved, leaving 166 and 167), so it was claimed and
resolved as a **measured candidate: parity fix lands, speed claim DEGRADED**.
Headline: the top/bottom opening is now canonical `detect_surfaces_type`'s
`opening_ex(diff, external_perimeter_width / 10)` with `jtMiter` / 3, the benchy
cluster saves **7.9–21.4 %** wall (25.5x / 9.9x / 26.7x / 11.6x the floor), the
`base` cells save **0.3–3.4 %**, **process CPU is lower in 8/8**, and the strict
wall gate reads **5/8 PASS, 1 DEGRADED, 2 sub-floor**. Output moves in 8/8 and
t166's set is the currency. Two auto-sentinel defects surfaced: the shell-side
one is fixed here, and the bridge-side one is graduated as
[168](issues/168-bridge-opening-auto-sentinel.md). The safety-offset variant was
measured and rejected. **No accelerated arm** (a disclosed gap). **Frontier:**
[167](issues/167-canonical-clipperutils-replay.md) and
[168](issues/168-bridge-opening-auto-sentinel.md), both AFK and unblocked.**

**Previous selection scope (2026-11-19, the t165 take):** after a re-route
session that graduated [166](issues/166-shell-opening-canonical-parity.md) and
[167](issues/167-canonical-clipperutils-replay.md), the human said "pick up
the first ticket", and 165 was first on the frontier. It was claimed and
resolved as **input verified** (8/8 at canonical prepass stands, binding
`benchy arachne-on` 0.898x). No production change. **Frontier now:**
[166](issues/166-shell-opening-canonical-parity.md),
[167](issues/167-canonical-clipperutils-replay.md).

**Previous selection scope (2026-11-18, the t164 take):** the human said
"Continue the perf vs orca map" (no ticket named); **1–163 were resolved and the
frontier was empty**, so the two live fog items were put to the human and the
human picked **phase re-attribution at HEAD**. It was charted as
[164](issues/164-phase-re-attribution.md) (`wayfinder:task`), claimed, and
resolved as a measured board + instrumented split + arithmetic. Headline: **the
prepass is 67.0–87.2 % of every cell and at canonical prepass cost all eight
cells win (0.396x–0.890x); `base classic-on` stops being the loser and the
binding cell becomes `benchy arachne-on` at 0.890x.** 8/8 board identity, 24/24
instrumented identity, every run clean. The route's residual risk is now a
**bound on one input**: the four supports-on cells have 1.50x–1.75x slack on
t110's canonical prepass cost, so [165](issues/165-canonical-prepass-cost.md) was
graduated to verify it on 2.5.1. No production change. **Frontier now:
[165](issues/165-canonical-prepass-cost.md) (AFK, unblocked, new).**

**Previous selection scope (2026-11-17, the t160 take):** the human said
"Continue the perf vs orca map" (no ticket named); **160 was the only
unblocked, unclaimed frontier ticket** (1–159 resolved), a `wayfinder:task`.
It was claimed and resolved as a measured probe: input read set identical
8/8, output byte-identical 8/8, batch arm slower in every supports-on cell
(−1.69 % to −3.77 %), null band 9.50 % any-pair, null-renderer ceiling
9.16–12.61 %, so the per-layer structure is **exonerated**. Probe reverted,
`probe.patch` preserved, `build-guests --check` exit 0, `verify.py` PASS and
`--self-test` 9/9. **Frontier now: none** — 1–160 resolved; the next shape is
a human pick from the fog (the renderer's own per-op work is the remaining
carrier, and fog item (b) still owns the fresh paired scoreboard).

**Previous selection scope (2026-11-17, the t159 take):** the human said
"Continue the perf vs orca map" (no ticket named); the frontier was empty
(1–158 resolved), so the human was asked to pick from the fog and chose item (a)
(fold the per-layer `tree-support` render into the prepass). It was charted as
[159](issues/159-tree-support-render-prepass-fold.md) (`wayfinder:grilling`),
claimed, and resolved by human ruling: **experiment, not adoption**; graduated
[160](issues/160-tree-support-prepass-batch-probe.md). Before the take, the
uncommitted t157/t158 work was committed at the human's request after clippy
`--all-targets`, `check-literals`, `build-guests --check` and `layer_module_tdd`
(72/72) passed. **Frontier now:**
[160](issues/160-tree-support-prepass-batch-probe.md) (AFK, unblocked, new).

**Previous selection scope (2026-11-16, the t158 take):** the human said
"Continue with the next one" (no ticket named); **158 was the only unblocked,
unclaimed AFK frontier ticket** (1–157 resolved; no HITL ticket open), a
`wayfinder:task`, and was claimed and mechanically resolved as **DROP**. On
2026-11-17 the human explicitly amended that disposition to **KEEP**. Landing
`expolygons_intersect_area_positive` inside `SliceRegionView::derive_needs_support`
(`crates/slicer-sdk/src/views.rs`) is byte-identical in **8/8** cells to `t151`'s
active reference set across 16 output runs, 80 A/B runs, and 18 null runs, lowers
process CPU in **8/8** cells by **5.88 %–21.38 %** (**5.3x–19.1x** the `1.12 %`
any-pair CPU null band; `39/40` pairs candidate-lower), and clears the `0.80 %`
wall floor in **all four `base` cells** (`+5.42 %` to `+14.87 %` wall, `6.78x` to
`18.58x` floor, `−4.23` to `−6.37 s`, `20/20` pairs candidate-faster), realising
`90.5 %` of `t157`'s `4.69 s` wall ceiling on `base-classic-off`. However, on
`benchy` the entire three-site `derive_needs_support` ceiling is only `~231–237
ms` (`~3.0 %` of `benchy` supports-on wall), so single-run wall jitter puts two
`benchy` cells below the separate-arm `0.80 %` wall floor over 5 interleaved
pairs: `benchy-classic-on` **+0.766 %** (`0.96x` floor, about `3 ms` short despite
winning `4/5` pairs with a **+2.69 %** pairwise median) and `benchy-arachne-off`
**−0.076 %** (`−0.10x` floor despite winning `3/5` pairs with a **+3.16 %**
pairwise median). The human's KEEP ruling restores the production code and unit
tests; the original mechanical failure and missing `benchy` null control remain
disclosed. `candidate.patch` and `RULING.md` are preserved in
`evidence/t158-needs-support-fast-predicate/`.
**No production commit; no fresh Orca scoreboard; no accelerated arm.** **Frontier now:
none** — 1–158 resolved; next is a human pick from the fog.

**Previous selection scope (2026-11-16, the t157 take):** the human said
"Continue the perf vs orca map" (no ticket named); **1–156 were resolved and the
frontier was empty**, so the map's own signpost from Not yet specified (`item
(d) the surviving predicate`) graduated as
[157](issues/157-needs-support-predicate-attribution.md) (AFK Phase-1
attribution + probe, following the t149/t152/t154 precedent) and was claimed and
resolved. Headline: **`commit_support_analysis_builtin` is the sole production
decision consumer of `derive_needs_support` in the entire pipeline (after Packet
224 retired `region.needs_support()` from `TreeSupport::run_support`), and an
exact fast-path boolean predicate inside `SliceRegionView::derive_needs_support`
collapses all three call sites (`PrePass::SupportAnalysis`, `Layer::Perimeters`,
`Layer::Infill`) by 91.0x–120.0x with 0 / 3,680 mismatches.** Under
`PNP_T157_FAST_PREDICATE=1`, combined `prep_views_ms` falls **−76.0…−77.1 %** on
`benchy` and **−93.0…−93.8 %** on `base`, and `PrePass::SupportAnalysis`
eligibility wall on `base classic-on` falls **2,100.5 → 41.5 ms (50.6x)**, with
**8/8** probe runs byte-identical to t151's active set. Priced against `t154`'s
ledger (`ESTIMATE`, not a ratio of record): **230.6 ms / 237.3 ms** on `benchy
classic-off / on` (**7.50x / 4.73x** floor) and **4.693 s / 7.414 s** on `base
classic-off / on` (**23.94x / 13.33x** floor), where `base classic-on`'s **5.346
s** per-layer saving is **4.88x** its **1.096 s** non-prepass requirement (`1.0508x
→ 0.8033x` at canonical prepass). Probe reverted, `probe.patch` preserved,
`cargo xtask build-guests --check` exit 0, and
[158](issues/158-needs-support-fast-predicate.md) graduated. **Frontier now:
[158](issues/158-needs-support-fast-predicate.md) (AFK, unblocked, new).**

**Previous selection scope (2026-11-15, the t156 take):** the human said
"Continue the perf vs orca map" (no ticket named); **156 was the only unblocked,
unclaimed frontier ticket** (1–155 resolved; no HITL ticket open), a
`wayfinder:task`, and was claimed and resolved as **DROP**. The prepared-region
memo candidate is exact (8/8 byte-identical to the t151 active set; every run
clean) and removes the redundant derivation (process CPU lower in 8/8), but the
quiet 5-pair wall gate fails on **benchy Arachne-on (−2.94 %)** and **base
Arachne-on (+0.17 %)**; the other six clear the 0.80 % floor, base Arachne-off
by **+10.09 %**. The same-arm null band is **9.52 % wall / 0.75 % CPU** any-pair
absolute, so most passing wall effects are small against the instrument. A
5-pair extension that began at **85 % load** was invalidated and preserved in
`reduced/loaded-window.json`; the gate is the fresh quiet 5-pair window (2 % →12 %
load sample). Production code/tests were reverted; `candidate.patch` and
`red-test.log` are preserved. **No production commit; no ratio of record; no
accelerated arm.** **Frontier now: none** — 1–156 resolved; next is a human pick
from the fog (the cheaper `needs_support` predicate remains the unpriced survivor).

**Previous selection scope (2026-11-14, the t155 take):** the human said
"Continue the perf vs orca map" (no ticket named); **155 was the only unblocked,
unclaimed frontier ticket** (1–154 resolved; no HITL ticket open), a
`wayfinder:task`, and was claimed and resolved as a **Phase-1 attribution plus a
probe** — the ticket's stop rule fires on the removable branch, so no production
edit. Headline: **the first-`Layer::Infill` block is the prepared-region
derivation, paid twice per layer, and the second payment is redundant.** A
five-file env-gated probe splits every bracket: `prep_views` is 97.5 % of the
first-Infill bracket on base-off (p50 52.4 of 53.6 ms) and 97.3 % on base-on
(53.3 of 54.8 ms); the entire dispatch path (pool lease, linker,
instantiation, region marshal, export call, teardown) is ~0.95 ms, and the arena
memo works (5 µs for every non-first module). The block is paid twice because
the fill-polygon sync takes and re-stages the slice three times per layer and
each `set_slice` clears the prepared slots; the second derivation is **provably
redundant** (0/1,488 syncs changed any field `prepare_regions` reads). Inside
it, 94.3–94.4 % is one `intersection_ex(region.polygons, footprint)` per layer,
with exactly one non-empty footprint per object per layer and a true answer in
every measured call; base-on evaluates it a third time in the prepass
eligibility pass. `tree-support` is separated and is not a host block (5 µs
prep; its 85.9 ms p50 bracket on base-on is the render). **Ceilings:** 2.382 s
base-off (12.15x floor, 21.5 % of the 11.10 s cut) and 2.335 s base-on (4.20x
floor, **2.13x the 1.096 s requirement**); indicatively base-on goes 1.051x →
~0.943x. Graduated [156](issues/156-prepared-memo-across-fill-sync.md) (AFK,
unblocked) with the standing eight-cell gate; the cheaper-predicate shape stays
in the fog. Disclosed: no wall claim, no ratio of record, no load sample, no
accelerated arm. **Frontier now: [156](issues/156-prepared-memo-across-fill-sync.md).**

**Current selection scope (2026-11-14, the t154 take):** the human said
"Continue the perf vs orca map" (no ticket named); **1–153 were resolved and the
frontier was empty**, so the map's own signpost graduated the **benchy per-layer
program** from Not yet specified as
[154](issues/154-benchy-per-layer-carrier-attribution.md) (AFK, following the
t152/t149 precedent; no human pick was needed because the ticket is a
measurement + attribution, not a route commitment). Headline: **the benchy
per-layer program is margin, not route** — at canonical prepass cost **7/8 cells
win**, the benchy cells by 9.9–33.2 %, and the only loser is **base classic-on
at 1.051x**, needing **1.096 s** from the per-layer side. The required prepass
cut is now **64.3–84.5 %**, largest on benchy arachne-on/base classic-on/base
arachne-on, and t128/t129's "prepass-only parity loses both benchy cells" is
superseded. The per-layer carriers are kernel-bound; the requests per-op split
does not exist at the current instrumentation surface (named scopes ≤ 22.8 % of
a carrier's fuel). One ledger correction: t127's "prepared-region view" name for
the first-`Layer::Infill` block cannot hold at HEAD, because `Layer::Perimeters`
materialises that view first and `Layer::Infill` hits the memo. **Frontier now:
[t155](issues/155-infill-stage-block-base-on-residual.md)** (AFK, unblocked) —
identify that block and price it against base classic-on's 1.096 s.

**Previous selection scope (2026-11-14, the t153 take):** the human said
"Continue the perf vs orca map" (no ticket named); **153 was the only
unblocked, unclaimed frontier ticket** (1–152 resolved), a HITL grilling, and
was claimed and resolved as a **DECLINE**. The human's answer was "Don't think
that is possible, it was attempted before, verify it before trying it"; the
verification found **ADR-0046** (accepted) already decides the placement
question against the per-layer module, **packet 168** discovered the
cross-layer constraint during grounding, and `build_seam_candidates`'
per-layer terms read the previous layer's contours — so the relocation fails
even without the alignment chaining. Headline: **the 1,248 ms seam offset is
the price of the cross-layer aligned seam in this architecture, not a
relocatable term.** S3 closes as "not a clean deletion and not relocatable"; no
exception is spent, no code or output moves. **Frontier now: none** — the next
shape is the human's pick from the fog.

**Previous selection scope (2026-11-14, the t152 take):** the human said
"Continue the perf vs orca map" (no ticket named); **1–151 were resolved and no
AFK ticket was on the frontier**, so the map's route arithmetic graduated the
last named member of the sweep-volume work-set family from Not yet specified —
t129's **S3**, the seam inset offset — as
[t152](issues/152-seam-inset-offset-redundancy.md), following the t149
precedent. Headline: **the prepass inset offset is not redundant.** Deleting it
moves **456/496** base-classic-off layers (+1,177 / −1,149 points, 142 layers
with a changed total `E`, first difference in the layer-0 Skirt) and breaks two
unit tests (**31/0 → 29/2**); the baseline reproduces t151 (`9bb18f0e…`), the
candidate is `4265d619…` (+1,158 bytes). The measured offset term is **1,248
ms** (`PrePass::SeamPlanning` 1,779 → 531 ms, 70.15 %), **5.9x** the 0.80 %
floor and **~10.5 %** of base classic-off's current cut — but it is not
bankable without a parity decision. The canonical loop projection is a
**per-layer relocation** of candidate generation onto the printed loops, which
needs a t81-style exception; the ruling is graduated as
[t153](issues/153-seam-loop-relocation-ruling.md) (HITL). No production change;
the patch is preserved. **Frontier now: [153](issues/153-seam-loop-relocation-ruling.md)
(HITL, unblocked).**

**Previous selection scope (2026-11-14, the t151 take):** the human said
"Continue the perf vs orca map" (no ticket named); **151 was the only unblocked,
unclaimed AFK frontier ticket** (146 was resolved by the t146 take, and no HITL
ticket was open), and was claimed and resolved as a **measured fairness fix**.
Headline: **the skirt hull folds the brim area's outer contour instead of the
beads' centreline, the skirt moves out by the canonical half flow spacing
(0.178–0.180 mm), the brim is provably untouched, and the supports-off adhesion
oracle reaches 4/4 (from 3/4)** — base arachne-off 3.45 % → **0.94 %**, base
classic-off 2.89 % → **0.37 %**, benchy 1.32/1.34 % → **0.05/0.04 %**, against
t146's ~0.4 / ~0.9 % estimates. Output: baseline 8/8 against t148's set,
candidate **6/8** and byte-identical across arms 8/8; the two base supports-on
cells are inert by construction (support clip empties the brim area) and the
audit proves **zero moved sections / zero E changes** there. Two pre-declared
items falsified and disclosed, not widened: "all eight move" (six do) and
"`|dE|` outside `Skirt` is zero" (≤ 0.00098 mm on ≤ 0.034 % of lines). A/B
attribution-only, every median inside a **measured ±12.86 % wall null band**, no
wall claim. The new 16-cell set is the currency; no route term moves. A falsifier
test was added and shown red on the pre-change library (`22.3` vs `22.5`), and
t142's dead `integrated_parity_skirt_brim_tdd` was revived as a disclosed
drive-by. **Frontier now: EMPTY — 1–151 resolved; the next shape is the human's
pick from the fog.**

**Previous selection scope (2026-11-13, the t150 take):** the human said
"Continue the perf vs orca map" (no ticket named); **150 was the first
unblocked, unclaimed AFK frontier ticket in order** (151 is the other), and was
claimed and resolved as a **measured deletion**. Headline: **the MeshAnalysis
bridge half is gone and the stage falls 4,445 → 732 ms on base classic-off**,
with all 16 output runs byte-identical to t148's active set. Paired medians
**−12.08 % / −4.06 %** on the two base cells (15.09x / 5.08x the floor) and
−1.77 … −9.37 % on the other six, CPU lower or flat everywhere; base
classic-off's 12.08 % is 3.3x the measured ±3.67 % null wall band, base
classic-on is only 1.1x it (disclosed), benchy classic-off is inside it
(reported, not claimed). Route: base classic-off **2.078x → 1.827x**, cut
51.88 % → 45.27 %; the other seven move 1.77–9.37 %. Two ticket corrections,
tree-verified: the whole-print-input guest's `bridges=1` is a **literal**
fixture, not mesh-analysis output, so it stays; `algo_mesh_analysis_tdd` had no
bridge assertions. Test retirement: AC-1…AC-6 + NEG-3 in
`bridge_detector_tdd.rs`, with the justification in the file header and the
file's `#![allow(dead_code)]` removed. No new ticket graduated: the deletion
answered its own question and opened none. **Frontier now:
[151](issues/151-skirt-hull-brim-contour.md) (AFK, unblocked).**

**Previous selection scope (2026-11-13, the t149 take):** the human said
"Continue the perf vs orca map, pick an AFK ticket". **No AFK ticket was on the
frontier** — 1–148 are resolved and [146](issues/146-variable-width-anchor-shape.md)
is the human's HITL grilling — so the map's route arithmetic graduated the
largest remaining unpriced shape of the **sweep-volume work-set family** from
Not yet specified as [t149](issues/149-meshanalysis-work-set-readers.md), a
Phase-1 attribution of `PrePass::MeshAnalysis` (t129's S2 row). Headline: **the
stage is 85.68 % a bridge detector no production code reads** — `bridge_regions`
costs **4,563.2 ms** of 5,325.8 ms on base classic-off (7,383,702 directed
half-edge insertions, 3,199 clusters, per-cluster unions), while the only read
output, `overhang_regions[].xy_footprint`, costs **529.1 ms**. The per-layer
overhang-polygons port is answered as **not the lever** (it would replace 529.1
ms and leave 762.6 ms before adding canonical's per-layer diff). Route: 12.95 %
of the cell, 16.18x the floor, 24.96 % of base-off's indicative cut; **2.078x →
2.387x** if the half disappears. No production change, no campaign; graduated
[t150](issues/150-meshanalysis-bridge-retirement.md) with a 16-cell
byte-identical gate. **Frontier now:**
[146](issues/146-variable-width-anchor-shape.md) (HITL) and
[150](issues/150-meshanalysis-bridge-retirement.md) (AFK, unblocked, new).

**Previous selection scope (2026-11-13, the t148 take):** the human said
"Continue the perf vs orca map, pick an AFK ticket"; **t148 was the first
unblocked, unclaimed AFK frontier ticket in order** ([146](issues/146-variable-width-anchor-shape.md)
is the human's HITL grilling) and was claimed and resolved as a **fairness fix
plus re-record**. Headline: **the exact-Z rejections are gone because the
planner now carves against the plane the gate validates** — `print_plane_occupancy`
(additive schema 2.1.0, WIT package 2.1.0), batched per object from the world
mesh at the quantized `anchor_z`, carved beside the midpoint payload; the
validator is untouched and still zero-tolerance. **All four supports-on cells:
diagnostics 101 / 101 / 44,474 / 44,474 → 0, `degraded: false`, 33 layers
recovered (32 base + 1 benchy), none lost, support path +36,700.688 / +36,700.716
mm base and +294.871 mm benchy; supports-off byte-identical 4/4.** **Ratios of
record re-derived: 2.705x / 2.829x / 3.356x / 3.248x**, CPU corroborating; the
active reference set is t148's and t145's four supports-on hashes are
superseded. **Disclosed:** the coverage clause's polygon reading is 11/19 base,
0/1 benchy and is not a PnP criterion (retained-layer control reads the same);
process CPU rose visibly but no causal wall cost is claimed because Orca's own
medians moved in the same window. **Frontier now:
[t146](issues/146-variable-width-anchor-shape.md) (HITL) only.**

**Previous selection scope (2026-11-12, the t143 take):** the human said
"Continue the perf vs orca map" (no ticket named); **143 was the first
unblocked, unclaimed AFK frontier ticket in order** ([144](issues/144-brim-anchor-reconstruction.md)
is the other) and was claimed and resolved as a **measured KEEP**. Headline:
**the planner's emit buffering depth is the union phase's width lever, and 12
layers is the measured optimum — not the pre-declared 16.** The union tail
(109 calls × 4.4 requests, 3.73x) collapses to 37 calls of ~13 requests at
depth 12: union wall **10,093.1 → 4,848.7 ms**, clip core **14,158.5 → 8,934.5
ms**, time-to-first-layer 69.7 → 56.4 s. Depth 16 fails its own bracket
(5,793.2 ms against the 4,620 ms LPT bound) because the same 482 requests cost
**+24 %** thread-time once sixteen layers are buffered; 8 lands on the merge-2
floor. **Paired A/B:** supports-on wall **+6.72 / +7.36 / +12.80 / +13.41 %**
at 8.40x–16.76x the floor; base classic-on **9.56 s saved (10.3 % of its 92.5 s
cut, 2.7x the DROP bar)**; supports-off inside the null band and structurally
inert. **Output byte-identical in 8/8**; base == t139, benchy == the
replacement-fixture baseline. **Disclosed:** strict CPU-lower 1/4 (a fan-out
change burns more CPU to finish sooner; inside the null CPU band), no
accelerated timing arm, no Orca arm, 3 pairs per cell, and the box's base
classic-on cell is 74.70 s today against t139's 126.27 s so only paired deltas
are claimed. **Fixture swap surfaced and graduated:** `tmp/3dbenchy.stl` was
replaced with a different export of the same model, retiring t139's four benchy
hashes; `base.stl` is intact and t145 carries the re-baseline. **Frontier now:
[144](issues/144-brim-anchor-reconstruction.md) (AFK, unblocked) and
[145](issues/145-benchy-fixture-rebaseline.md) (AFK, unblocked, new).**

**Previous selection scope (2026-11-12, the t140 take):** the human said
"Continue the perf vs orca map" (no ticket named); the first frontier ticket by
number was the human's HITL
[Brim anchor](issues/140-brim-contour-input.md), so it was claimed and
**resolved with the human** over three rounds. Headline: **the 0.20 mm
brim-anchor gap is real printed geometry — the brim's inner boundary sits
0.1 mm inside the part outline and the innermost bead overlaps the footprint by
~0.2 mm — and the human granted it a scoped t81-style exception rather than
stretching t130.** The reconstruction is module-side — offset the filled
footprint and the multi-layer skirt hulls by half the resolved outer-wall
width — with no SliceIR/WIT change; the standing fairness ceremony applies (fresh
16-cell reference set, delta audit, four supports-off oracle cells, 3-pair A/B
as attribution only), with **no** 8-cells × 3-tools scoreboard re-derivation,
and t139's
fairness-fix form is the stop rule (the oracle is a measure, not a revert gate;
a band miss graduates the variable-width envelope or the direct contour). The
oracle ceiling from this fix is **4/8**; supports-on remains out of scope. No
production edit in this take (tracker-only). **Frontier now:**
[143](issues/143-union-phase-request-grouping.md) (AFK) and
[144](issues/144-brim-anchor-reconstruction.md) (AFK); no HITL ticket remains
open.

**Previous selection scope (2026-11-12, the t137 take):** the human said
"Continue the perf vs orca map" (no ticket named). Both unblocked frontier
tickets were HITL grillings (`137` and `140`) and **137 was the first in
order**, so [Planner scheduling gate](issues/137-planner-scheduling-gate.md) was
claimed and **resolved with the human**. Headline: **extend the t117 four-cell
gate form to [t135](issues/135-cross-layer-occupancy-batching.md) Stage 2 and
[t136](issues/136-planner-clip-batch-fanout.md), and generalize it as the
standing rule for any representation-safe planner-only candidate where the four
supports-off cells are structurally inert** — positive 0.80 % paired wall/CPU
improvement on the four supports-on cells, byte-identical output in all eight
against the active reference set ([t139](issues/139-layer-plane-parity.md)'s
set), and no median wall/CPU regression on the four supports-off cells. Both
"require a supports-off leg" and "park the planner scheduling program" were
declined. No production edit in this take (tracker-only). **Frontier now:
[135](issues/135-cross-layer-occupancy-batching.md) Stage 2 (AFK, unblocked),
[136](issues/136-planner-clip-batch-fanout.md) (AFK, unblocked), and
[140](issues/140-brim-contour-input.md) (HITL).**

**Previous selection scope (2026-11-11, the t142 take):** the human said
"continue the perf vs orca map, pick an afk ticket". **t142 was the first
unblocked, unclaimed AFK frontier ticket in order** (137 and 140 are the
human's grillings; 136 / 135 Stage 2 are gated by 137) and was claimed and
resolved as a measured BUILD. Headline: **the 4-band polygon partition is
deleted and the quartile is derived from the signed distance the consumer
already measures** — `annotate_overhangs` now commits only the unsupported area
+ previous-layer footprint, `PerimeterSpatialContext` takes the producer's band
width, and the band R-tree is gone. Output is **byte-identical in 8/8 cells**
and equal to t139's reference set, stable across all 48 A/B rows; the paired
A/B (3 interleaved pairs) saves **9.69 %–26.25 %** of the wall (base
classic-off 26.25 %, base classic-on 11.76 %, benchy classic-off 20.00 %) with
the candidate faster in **24/24** pairs and process CPU lower in **8/8**
(14.6 %–32.6 %). Composing with t139's paired scoreboard gives an indicative
**1.818x–3.697x** ordinary and a required cut of **45.0 %–73.0 %** (not a
re-taken scoreboard). Gaps disclosed: the accelerated **timing** arm is not taken
(the controlled-driver `RUSTC` wrapper + MSVC 14.51 mangles the
`libmimalloc-sys` verbatim source path; the same build without the wrapper
succeeds), while accelerated **output parity** was checked with a disclosed
manual `--cfg pnp_perimeter_spatial_accelerated` host plus the accelerated guest
artifacts (8/8 same hashes); the box was
not verified idle (the output-mode base-on run took 117.4 s vs its 94.8 s A/B
median), and two pre-existing reds are recorded
(`integrated_parity_skirt_brim_tdd`'s missing `brim_object_gap`, confirmed red
at HEAD via `git stash`; `layer_slice_tdd`'s z = 0.1 layer under t139's midpoint
plane). Two latent-vacuous quartile fixtures were repaired. Evidence:
[`evidence/t142-overhang-band-deletion/`](evidence/t142-overhang-band-deletion/FINDINGS.md)
(`verify.py` re-derives every number; `--self-test` 4/4 controls).

**Previous selection scope (2026-11-11, the t141 take):** the human said
"Continue the perf vs orca map, pick an afk ticket". **No AFK ticket was on the
frontier** — 137 and 140 are the human's grillings and 136 / 135 Stage 2 are
gated by 137 — so the map's own route arithmetic graduated the largest
unpriced shell shape from the "sweep-volume work-set family" fog as
[141](issues/141-overhang-quartile-band-deletion.md) and worked it as a
Phase-1 attribution. Headline: **the overhang quartile band partition is
deletable work** — its value has no production reader, its only read is a
presence bit equal to the sign of the signed distance the perimeter annotation
already computes against the same previous-layer boundary, and it costs
**8.964 / 9.558 / 10.583 s** on base classic-off (22–27 % of the wall, 34–41 %
of the required cut), **9.450 s** on base arachne-off, **11.195 s** on base
classic-on, **0.954 s** on benchy classic-off and **0.817 s** on benchy
classic-on. The
counts (3 offsets + 6 booleans per band layer, 486/496 base and 239/240 benchy)
and the 11.6–11.7x thread→wall conversion reproduce t84. Two build forks are
recorded (the per-layer boundary key and the visual-debug tap) and the deletion
is graduated as [142](issues/142-overhang-band-partition-deletion.md). The box
was not verified idle, the three base-off probes span 18 % (8.96–10.58 s), and
no A/B or scoreboard was taken; no wall claim is made for a landed change. Evidence:
[`evidence/t141-overhang-bands/`](evidence/t141-overhang-bands/FINDINGS.md)
(`verify.py` 7 rows / 3 sources; `--self-test` 4/4 negative controls).
**Frontier now: 142 (AFK, takeable) and 137 / 140 (HITL); 135 Stage 2 / 136 are
gated by 137.**

**Previous selection scope (2026-11-10, fortieth session — the t138 take):** the
human said "Continue the perf vs orca map" (no ticket named); **138 was the
first unblocked, unclaimed AFK frontier ticket in order** (137 is the human's
grilling; 136 and 135 Stage 2 are gated by it), so it was claimed and resolved as
a Phase-1 attribution. Headline: **the layer-0 cross-section gap is the slice
plane** — PnP samples every layer at its top, canonical at the midpoint
(`compute_slice_z`); ten sampled layers put each arm on its expected plane (PnP
0.02–1.36% off top, Orca 0.02–4.02% off mid), and base L0/L10 separate the
candidate planes by **253% / 161%**. The 0.1 mm shift is **3.42x** of layer-0
area on base and **1.3%** on the benchy; base prints **495 layers / Z 99.0** on
PnP against Orca's **496 / 99.2**. The base-off skirt (+3.23% / +3.10%) is the
brim footprint (+8.68%, decomposed +88.04 mm contour − 27.65 mm anchor against
+57.39 mm observed) diluted by the skirt offset; the supports-on cells are the
support engine (Orca's first-layer support **10.6–11.7x** denser, coverage
**10.4–11.5x**, and the skirt output tracks the support-dominated input hull);
run counts are cosmetic; first-layer height is equal. **The oracle is 2/8 and no
8/8 is reachable without a slicing change:** ceiling **6/8** without one
(perfect support parity), expected **4/8** with the plane fix alone, 8/8 needs
both. The fix is graduated as [139](issues/139-layer-plane-parity.md) — a
fairness fix, not a route step (base layer-0 operand 5.329 → 2.314 mm² on 1 of
495 layers; no wall claim; the 59.1–73.5% cut is unchanged). No production edit,
no timing campaign, no reference move. Evidence:
[`evidence/t138-layer0-cross-section/`](evidence/t138-layer0-cross-section/FINDINGS.md)
(`verify.py` 8 claim groups, **12/12** negative controls). **Frontier now: 139
(AFK, takeable now) and 137 (HITL); 135 Stage 2 / 136 are gated by 137.**

**Previous selection scope (2026-11-10, thirty-ninth session — the t135 Stage-1 take):**
the human said "Continue the perf vs orca map" (no ticket named); **135 Stage 1
was the first unblocked, unclaimed frontier ticket in order** (137 is the
human's grilling, 136 and 135 Stage 2 are gated by it), so it was claimed and
resolved. Headline: **the matched-settings asymmetry is closed, including the
adhesion flow model, and the port is output-isolated to the adhesion feature.**
The module now emits canonical geometry at `40ecc66f78` with the canonical flow
spacing; the matched job authors **seven** keys on both arms (`brim_width = 8`,
`brim_type = outer_only`, `skirt_loops = 6`, `skirt_distance = 3`,
`skirt_height = 1`, `initial_layer_line_width = 0.4`,
`tree_support_brim_width = 0`) and asserts them per run. The baseline reproduces
t123's eight hashes; the candidate records the new set (`7db0641b…`,
`c02aa4e9…`, `a2f7a1fe…`, `2326455d…`, `7ef84447…`, `1ddea2b7…`, `defae354…`,
`4aa1847c…`), identical on both build arms. Delta audit: every motion line
outside `Skirt`/`Brim` is identical with the E field stripped, |dE| <= 0.00098 mm.
The oracle is **2/8 strict**: benchy supports-off passes (+2.4/+2.6 % brim work,
+1.3 % brim area), base supports-off is 22/22 lines at +8.1/+7.3 % brim work
with skirt +3.2/+3.1 % against a 3 % band (anchor convention), and supports-on is
dominated by the support engine (0/0 brim on base, skirt 12.5 %; Orca's benchy
brim fragments into 39 runs vs 19). A/B below the noise floor (wall 0.963–1.046,
CPU 0.992–1.026). Scoreboard: **2.445x–3.769x ordinary / 2.420x–3.745x
accelerated**, required cut **59.1 %–73.5 %**, superseding t124. **Frontier now:
137 (HITL) and 135 Stage 2 (gated by 137); 136 gated by 137.**

**Previous selection scope (2026-11-09, thirty-eighth session — the t130
take):** the human said "Continue the perf vs orca map" (no ticket named);
both frontier tickets were HITL and **130 was the first in order**, so
[Matched settings vs the 8 mm brim](issues/130-brim-matched-settings-ruling.md)
was claimed and **resolved with the human** (nine questions over four rounds).
Headline: **the adhesion asymmetry closes — and a second one surfaced and
closed with it.** Re-derived facts: PnP's matched job prints 20 brim + 6 skirt
loops of bounding-box rectangles (module defaults; the matched configs author
no skirt/brim key) while Orca's chain resolves `skirt_loops = 0` and the rig
pins `brim_width = 0` — two asymmetries on one surface. Ruling: port canonical
`make_brim` + the `Print.cpp` skirt construction (hull-anchored) with the
OrcaSlicer attribution header, match at `brim_width = 8` / `skirt_loops = 6` /
`skirt_distance = 3.0` / `skirt_height = 1` with per-run fail-closed asserts on
both arms (the t103 pin re-scopes from "2.4-era values" to "the matched job").
Policy: close-by-default with **parity work licensed only to close
asymmetries** (route-wide lifting of the parity exclusion declined; a repo ADR
declined — the Out-of-scope entry is the established record). Sequencing: the
**staged fold into t135** (a separate re-take ticket declined; a fused single
candidate rejected as the t115 failure mode) — Stage 1 runs unblocked and
records the new reference set with its own paired attribution, Stage 2 stays
gated by 137, and the 8-cells × 3-tools scoreboard re-derivation is the closing
gate. No production edit in this take (tracker-only, plus the new
`GLOSSARY.md` term **matched settings** / **asymmetry**). **Frontier now: 137
(HITL) and 135 Stage 1 (AFK); 135 Stage 2 / 136 gated by 137.**

**Previous selection scope (2026-11-09, thirty-seventh session — the t134 take):**
the human said "Continue the perf vs orca map" (no ticket named); **134 was the
first unblocked, unclaimed AFK child in order** (130 is the human's grilling),
so it was claimed and **resolved as a Phase-1 attribution**. Headline: **the
planner's serial wall is the per-layer occupancy offset, not the clip
fan-out.** The stage carries **105.1 s** of clipper thread-time: **95.3 %** is
in the layer-independent emit pass, **2.5 %** in the recurrence-bound drop
loop. `p6d_occupancy` is **14,249 ms wall / 14,169 ms thread = 0.99x** (433
unique offsets); `p6a_clip_core` is **17,811 ms wall / 85,641 ms thread =
4.81x**. The occupancy batch ceiling is **11.2 s** (**11.1x** floor, **12.1 %**
of base-on's 92.5 s cut); clip-to-12x is **10.7 s** (**10.6x**, **11.5 %**);
combined **21.9 s** (**21.7x**, **23.7 %**), still a tranche. **The all-8-cell
gate is structurally unreachable:** the whole `SupportGeometry` stage is
**0.64–0.81x** the floor on supports-off, the planner module **0.32–0.43x**.
Go/no-go: **NO-GO as the route, GO (narrowed) for two supports-on tranches** —
[t135](issues/135-cross-layer-occupancy-batching.md) (occupancy batch, exact)
and [t136](issues/136-planner-clip-batch-fanout.md) (clip fan-out diagnosis) —
both blocked by the human's gate ruling
[t137](issues/137-planner-scheduling-gate.md). Even a zero-cost planner
leaves **2.87x–3.25x** on the four supports-on cells, so the route remains
operand volume / geometry. Disclosures: one pass per cell, three classic-on
repeats spread **2.45 %** (base) / **1.17 %** (benchy); instrumented walls are
not wall claims; the occupancy fan-out uses the clip core's measured 4.81x as
a proxy. Evidence:
[`evidence/t134-planner-parallelism/`](evidence/t134-planner-parallelism/FINDINGS.md)
(`verify.py` 94 checks, 12/12 negative controls; all eight G-code hashes equal
t123's candidate-ordinary set). **Frontier now: 137 and 130 (HITL); 135/136
are blocked by 137.**

**Current selection scope (2026-11-09, thirty-sixth session — the t133 take):**
the human said "Continue the perf vs orca map" (no ticket named); **133 was the
first unblocked, unclaimed AFK child in order** (130 is the human's grilling), so
it was claimed and **resolved as an exact measured KEEP**. Headline: **window the
negative erosion only; the positive offset is globally consumed.** The
formulation is `internal_bridge_offset_window`: the union bbox of the layer's
`internal_solid_fill` candidates grown by `(2 x 4.5 + 1) x qualify_spacing_mm`,
applied to the `deep_infill_area` operand of the `-4.5 x spacing` erosion. Both
consumers (`qualified.retain` and the angle filter) read the eroded set only
inside a candidate, and the window boundary is outside the erosion's
`2 x 4.5 x spacing` reach, so the covered set is unchanged. **The first candidate
windowed both offsets and is rejected:** it is confined to the bridge role (no
structural geometry lines) but moves 8 benchy bridge layers by up to **22.09 mm**
because `internal_bridge_angles` intersects the whole-layer `expansion_area` with
`deep_infill_clip_area` and samples that whole-layer anchor set. **16-cell
output:** 8 cells x 4 arms, one hash per cell, every hash t123's reference set
(`90638bf3…`, `711c6334…`, `2ba5d84c…`, `a972b0f2…`, `b3b4ac5e…`, `4e31a2db…`,
`6b865d4a…`, `62d6eedc…`); delta audit is equality, **zero moved lines**. **Paired
gate:** **8/8** at **1.66x–10.94x** the 0.80 % floor with CPU corroborating in
every cell — benchy **4.83 → 4.64 / 8.84 → 8.71 / 5.01 → 4.87 / 8.92 → 8.79 s**,
base **42.51 → 39.82 / 102.45 → 101.09 / 43.82 → 39.98 / 106.95 → 102.18 s**.
**Sequential sampling disclosed:** 3 pairs per cell, extended to 6 for benchy
arachne-on and 12 for base classic-on (pre-declared, all pairs retained; the
12-pair base-on median is **+1.36 s, 1.66x floor**, 7/12 wall-positive and 12/12
CPU-positive). **Ledger correction to t131:** the erosion's thread-to-wall
conversion is **2.80x / 3.96x / 3.27x / 6.42x**, not the stage's 5.25–8.78x, so
the probe understated benchy-off by 1.7x and base-off by 2.3x. Route arithmetic:
**9.7 %** of base-off's 27.7 s required cut, **1.5 %** of base-on's 92.5 s — a
banked keep, not a destination step. Disclosures: box load **21.2 %** before the
campaign and **10.9 %** later (DWM/Chrome/Xbox/Discord), 6 physical / 12 logical
cores; three measurement windows; no Orca arm. Evidence:
[`evidence/t133-windowed-bridge-offsets/`](evidence/t133-windowed-bridge-offsets/FINDINGS.md)
(`verify.py` 357 checks, 17/17 negative controls; `rejected-full-window/`
preserves the positive-offset experiment and its audits). **Frontier now: 134
(AFK) and 130 (HITL).**

**Previous selection scope (2026-11-09, thirty-fifth session — the t132 take):**
the human said "Continue the perf vs orca map" (no ticket named); **132 was the
first unblocked, unclaimed AFK child in order** (130 is the human's grilling), so
it was claimed and **resolved as a measured BUILD**. Headline: **the duplicate
occupancy half is serial and worth 12–13 % of both base supports-on cells.** A
per-layer content-keyed memo (`OccupancyMemoEntry`, `occupancy_offset_key`,
`inflate_model_occupancy_memoized`) serves the role carve's
`+support_object_xy_distance` offset from the emit check's when the payload is
byte-equal, comparing the payload on a hash hit so the memo is exact. Probe
census (counters reverted before the campaign): **864 calls / 433 distinct / 431
hits** on base classic-on and **365 / 186 / 179** on benchy classic-on —
identical to t131. **16-cell output:** one hash per cell across four arms, every
hash ticket 123's reference set; the host binary is identical between arms.
**Paired gate:** supports-on **4/4** at **15.78x / 7.39x / 14.97x / 16.54x** the
floor (benchy classic-on **9.33 → 8.15 s**, benchy arachne-on **9.17 → 8.62 s**,
base classic-on **119.45 → 105.14 s**, base arachne-on **119.88 → 104.02 s**),
CPU **+1.11 / +0.48 / +13.25 / +19.50 s**. The four supports-off cells are inert
by construction (`enable_support: false` returns from
`run_support_geometry_with_analysis` before `plan_for_object`), so their
mixed-sign medians are noise; the strict sign read is **6/8** and is recorded
rather than hidden. **Ledger correction to t131:** the duplicate half converts at
**1.006x** its thread-time on base classic-on (**14.31 s**, not the **6.5 s**
priced at the stage's 2.19x) because the duplicate calls were serial; benchy
classic-on saves **1.93x** its 0.61 s thread-time, mechanism unmeasured. Route
arithmetic is unchanged: **15.5 %** of base classic-on's 92.5 s required prepass
cut. Disclosures: two measurement windows plus a restart-completed pair; the
probe arm is not the campaign arm; box load 2 % / 7 % / 4 %; no Orca arm; the
multi-region caveat holds. Evidence:
[`evidence/t132-planner-occupancy-memo/`](evidence/t132-planner-occupancy-memo/FINDINGS.md)
(`verify.py` 176 checks, 11/11 negative controls rejected; both freshness gates
exit 0). **Frontier now: 133 (AFK), 134 (AFK), 130 (HITL).**

**Previous selection scope (2026-11-09, thirty-fourth session — the t131 take):**
the human said "Continue the perf vs orca map" (no ticket named); **131 was the
only open AFK child** (130 is the human's grilling), so it was claimed and
**resolved as a Phase-1 per-op decomposition**. Headline: **the per-op wrapper
is not the lever.** On every common prepass stage the Clipper2 sweep holds
**98.2–99.9 %** of the measured primitive thread-time; path conversion, WIT
marshalling, PolyTree rebuild and allocation together hold **0.1–1.8 %** (base
classic-off non-sweep **543 ms of 280,657 ms = 0.19 %**; base classic-on
**1,075 ms of 517,928 ms = 0.21 %**). The 9.8–15.7x phase gap is **operand
volume inside the sweep**: `ShellClassification` runs **19.5 calls/layer** at
**4,608 points/call** (**44,463,545 points**, **89,825/layer**) and the planner
**290 calls/layer** at **2,040 points/call** (**293,219,629 points**,
**592,363/layer**), with the kernel at parity (t70). **S5 is now priced but not
exact:** the two internal-bridge whole-layer offsets are **22,964 → 1,937 ms**
of thread-time on 130 base-off layers (**16.06x / 8.81x**), wall-equivalent
**2,520 ms** (**7.4x** floor, **9.1 %** of the cell's required cut), but
positive exact on **115/130** and negative on **36/42** layers (max **0.71 /
0.59 mm²**, net **−1.27 / +1.18 mm²**). **S6 is exact by construction:** the
`+0.35 mm` occupancy family is **864 calls over 433 distinct payloads**, the
**431** duplicates cost **14,227.7 ms** of thread-time (**6,504 ms** wall at
2.19x, **6.4x** floor, **7.0 %** of base-on's required cut), and the sibling
`−0.6 / −0.8 mm` collision family is **1,350/1,350 distinct** — no memo hits.
**Scheduling is the surprise:** the planner delivers **122,657.6 ms** of sweep
thread-time in **56,289 ms** of wall (**2.19x**; benchy-on **1.68x**) against
**8.34–11.80x** elsewhere, so at 4x it would cut **25,625 ms** (**27.7 %** of
base-on's required cut, **25.4x** floor) — larger than S5 and S6 combined on
that cell. **Gate:** the first branch fails on every stage (overhead 0.9–2.6 %
of the stage's share) and the second fails literally (S5 17.7 %, S6 12.8 %,
planner-at-4x 50.6 % of the stage's share), but the route-level stop rule does
not fire because the operands *can* be localised (S6 exactly, S5 with a
bounded delta) and the planner's parallelism is a separate wall lever. Go/no-go:
**NO-GO for a per-op rewrite; GO (narrowed) for operand locality and planner
scheduling.** Graduated: [S6 occupancy memo](issues/132-planner-occupancy-memo.md),
[S5 windowed internal-bridge offsets](issues/133-internal-bridge-windowed-offsets.md),
[planner effective parallelism](issues/134-planner-effective-parallelism.md) —
all AFK, open, unclaimed. **Frontier now: 130 (HITL, the human's grilling) and
132/133/134 (AFK).** Evidence:
[`evidence/t131-per-op-decomposition/`](evidence/t131-per-op-decomposition/FINDINGS.md)
(`verify.py` 150/150 checks, prose re-derived; `--self-test` rejects 11/11
negative controls; four cells × `probe` / `probe+s5`, every G-code hash equal to
the t123 candidate-ordinary set; probe preserved as `probe.patch` and reverted;
`cargo xtask build-guests --check` exit 0; rebuilt ordinary binary
`f3d50217…` reproduces `90638bf3…`). Standing paired acceptance and KEEP/DROP
rules unchanged.

**Previous selection scope (2026-11-08, thirty-third session — the t129 take):**
the human said "Continue the perf vs orca map" (no ticket named; 129 was the
only open AFK child and 130 is the human's grilling), so 129 was claimed and
**resolved as a Phase-1 decomposition**. Headline: **the work set is not the
lever; per-op cost and operand locality are.** Canonical does *more* per-layer
Clipper work than PnP in the shell cluster, yet its **whole prepass** is
**15.7x / 9.8x** cheaper on base classic-off / -on; the Rust Clipper2 kernel is
at parity (t70), so the
15.7x is above the kernel — operand size, path conversion, allocation, and
parallel scheduling. The clean PnP-local work-set deletions (overhang quartile
bands 9,689 ms, whole-mesh facet union 4,260 ms, seam inset 92.4 % of 1,718 ms)
are **15.5 s = 55.8 % of base-off's 27.8 s required cut** and **1.6 s = 17.5 %**
of base-on's; they are a first tranche, not the route. **Prepass at canonical
cost wins base classic-off only (0.787x), ties base classic-on (1.012x) and
loses both benchy cells (1.053x / 1.075x); full geometry parity wins all four
(0.335x / 0.661x / 0.550x / 0.704x).** Ledger corrections: "zero prepass" is not
"prepass at canonical cost" (canonical's prepass is 2.1 / 10.4 / 0.05 / 0.55 s);
and the per-layer phase belongs in the first program on both benchy cells, not
a later last mile. The go/no-go is **GO for the engine rewrite, NO-GO for a
work-set-only rewrite**; the next gate is
[Common-stage per-op cost decomposition](issues/131-common-stage-per-op-cost-decomposition.md)
(AFK, open, unclaimed). The brim decision remains
[Matched settings vs the 8 mm brim](issues/130-brim-matched-settings-ruling.md)
(HITL). **Frontier now: 130 (HITL) and 131 (AFK).** Evidence:
[`evidence/t129-prepass-work-set-parity/`](evidence/t129-prepass-work-set-parity/FINDINGS.md)
(`verify.py` 4 cells / 6 shapes / 20 prose claims; `--self-test` 8/8 negative
controls; `--orca-check` 14 citations at `40ecc66f78`). No production edit, no
G-code reference move; both freshness checks untouched. Standing paired
acceptance and KEEP/DROP rules unchanged.

**Previous selection scope (2026-11-08, thirty-second session — the t128 take):**
the human said "Continue the perf vs orca map" (no ticket named; 127 was
resolved and 128 was the only open child), so 128 was claimed and **resolved
with the human**. Headline: **the 8/8 destination stays; the route is re-scoped
from a prepass campaign to an engine rewrite, gated on a work-set/per-op parity
decomposition against canonical `v2.5.0`.** The ruling rests on the measured
arithmetic (prepass 62.1–87.1 % of every cell; required removal 72–101 %;
benchy classic-off 1.02x even at zero prepass) and on the stage census: every
remaining prepass stage is clipper-bound and every named shape has been
screened (t75/t84/t113/t125/t53/t55/t76), so the 0.80 %-per-shape machinery
would need 72–99x its floor. The route ticket is
[Prepass work-set and per-op parity against canonical v2.5.0](issues/129-prepass-work-set-per-op-parity.md)
(AFK, open, unclaimed); its kill gate returns the destination to 128 for re-aim
or park if PnP already computes canonical's work set at canonical's per-op cost.
The secondary decision — the **8 mm brim vs pinned Orca 0** — is graduated as
[Matched settings vs the 8 mm brim](issues/130-brim-matched-settings-ruling.md)
(HITL, open, unclaimed). The local Orca checkout is pinned at `v2.5.0`
(`40ecc66f78`); the source and the measured binary are disclosed as not
byte-proven identical. **Frontier now: 129 (AFK, open and unclaimed) and 130
(HITL).** Standing paired acceptance and KEEP/DROP rules unchanged.

**Previous selection scope (2026-11-08, thirty-first session — the t127 take):**
the human said "Continue the perf vs orca map" (no ticket named; 127 and 128
were open), so the first unblocked AFK frontier ticket,
[Raft-fill dispatch outside the raft band](issues/127-raft-fill-dispatch-outside-band.md),
was claimed and **resolved as a measured DROP**. Headline: **the decline removes
the call but not the cost.** The candidate is output-identical in **8/8 cells**
on both build arms with every guest module identical between arms (32-run output
campaign, all hashes equal to the t123 reference set), and the raft-default
dispatch really disappears (**495 → 0** on each base cell, **240 → 0** on
benchy). But the 61–63 ms per call is the **`Layer::Infill` stage's shared
prepared-region view materialisation**, paid by whichever Infill module is
dispatched first: in the candidate `com.core.gyroid-infill` pays the same p50
(**60 / 58 / 61 / 57 ms** against raft-default's **56 / 60 / 66 / 66 ms**) on the
same layers, and layer 0 of base classic-off shows 26 ms in raft-default / 1 ms
in gyroid flipping to 0 / 25 ms. The per-layer phase wall does not fall and the
gate passes **2/8** cells (benchy classic-on +1.97 %, base classic-on +3.06 %);
ticket 117's narrower gate also fails 4/8. **Ledger correction to t126:** its
61–63 ms "ceiling" is not a saving, and its 17-layer "other module first pays
0 ms" control cannot be a dispatch-order claim. **Disclosures:** the first AB
batch was split by a 14.7-hour session interruption and two visible stalls and
is preserved, not used; the measurement of record is a cell-local
three-interleaved-pair ABBA batch (load 42 % → 15 %, base within-cell spread
2–9 %); the `Layer::Support` half and raft-enabled jobs are unit-tested only; no
Orca arm was run, so t124's scoreboard stands. Candidate, two tests and both
staged binaries retired; patch preserved. Evidence:
[`evidence/t127-raft-band-dispatch/`](evidence/t127-raft-band-dispatch/FINDINGS.md)
(`verify.py` 528 claims checkout-only; `--self-test` rejects 18/18 negative
controls). **Frontier now: no open AFK child; 128 is the human's grilling.**

**Previous selection scope (2026-11-08, thirtieth session — the t126 take):**
the human said "Continue the perf vs orca map" (no ticket named; all 125
children were resolved), was offered the live fog routes, and selected **refresh
the post-t123 split, then attribute per-layer/emit**.
[Post-t123 phase split refresh and per-layer/emit attribution](issues/126-post-t123-phase-split-refresh.md)
was graduated, claimed and **resolved as a measurement + Phase-1 attribution**.
Headline: **the split is re-derived on all eight cells and it re-prices the
map's next stage.** Prepass **62.1–87.1 %**, per-layer **10.6–33.3 %**, postpass
**1.8–4.9 %**. `PrePass::Slice` is no longer a term (**41,472 → 2,110 ms** on
base classic-off, **−94.9 %**); the supports-off prepass is now
`ShellClassification` + `OverhangAnnotation` (**24.4 s of a 42.5 s cell** on base
classic-off) and the supports-on prepass is `SupportGeometry` +
`SupportAnalysis` (**68.9 s of a 126.3 s cell** on base classic-on). **The
destination arithmetic is the headline:** deleting the whole per-layer phase
leaves **1.67x–4.14x** in every cell; deleting the whole prepass reaches
**0.48x–1.02x**, and on **benchy classic-off even a zero prepass loses
(1.02x)**. The required prepass removal is **72 %–101 %** of today's prepass.
The per-layer phase has **no globally idle wall** (module-busy union = phase
wall on the base cells; 2.0–2.3 % = 0.47–0.90x floor on benchy). Per-layer
carriers: perimeters (3.82x–15.84x floor ceiling), tree-support on supports-on
(4.66x–9.63x), raft-default (**2.64x–8.25x**), infill-linker (1.17x–8.28x). The
postpass is not a route (largest bracket 0.85–2.61x). One exact shape is
graduated: [Raft-fill dispatch outside the raft band](issues/127-raft-fill-dispatch-outside-band.md)
— `com.core.raft-default` pays **61–63 ms on all 495 base layers** while its
body returns before doing any work (`raft_layers = 2`); the 17-layer
other-module-first control shows the cost is module-specific, not stage
preparation. The destination decision is graduated as
[Destination arithmetic after t123: re-aim, rewrite, or park?](issues/128-destination-arithmetic-ruling.md)
(HITL). Evidence:
[`evidence/t126-post-t123-phase-split/`](evidence/t126-post-t123-phase-split/FINDINGS.md)
(`verify.py` holds 617/617 claims checkout-only; `--self-test` rejects 10/10
negative controls). **Frontier now: 127 open and unclaimed; 128 is the human's
grilling.** Standing paired acceptance and KEEP/DROP rules unchanged.

**Previous selection scope (2026-11-07, twenty-ninth session — the occupancy
batch candidate):** the human explicitly selected
[Strong planner occupancy-offset batching: representation-safe four-cell campaign](issues/125-strong-planner-occupancy-offset-batching.md).
It was claimed and resolved as a measured **DROP**. The candidate batched only
the per-origin role-carve offsets inside the bounded emit flush; the per-layer
emit-check half stayed singular. **Output proof passed everywhere:** baseline
ordinary ≡ baseline accelerated ≡ candidate ordinary ≡ candidate accelerated ≡
the post-t123 set in 8/8 cells, with 32 output runs + 48 paired runs all clean
and zero exclusions. **The paired gate failed in 6/8 cells:** only benchy
arachne-on (+1.89 % wall, +0.5782 s CPU) and base arachne-on (+10.88 %,
+2.3125 s) passed. Base classic-on improved wall +6.13 % but CPU regressed
1.7656 s; benchy classic-on regressed wall −17.90 %. The candidate and both
WASMs were retired, the production diff is empty, and both freshness checks are
green again. Full planner tests: **148 passed**. Evidence:
[`evidence/t125-occupancy-offset-batching/`](evidence/t125-occupancy-offset-batching/FINDINGS.md)
(`verify.py` 442 checks, 10/10 negative controls rejected). **Frontier now: no
open child.** The per-layer/emit program remains the next stage's fog; the
unmeasured per-layer occupancy half is not an automatic rescue ticket.

**Previous selection scope (2026-11-07, twenty-eighth session — the Strong
planner carry ruling):** the human said "Continue the perf vs orca map" (no
ticket named). The frontier held only
[Strong planner carry: fold the occupancy batching into the Slice campaign, amend the gate, or park it?](issues/117-strong-planner-carry-ruling.md),
so it was claimed and resolved **with the human**. The original Slice-carry route
is stale: the landed Slice candidates are already in the baseline and cannot be
counted again.
The human **amended this representation-safe candidate's gate**: the 0.80 %
paired wall floor with CPU corroboration applies to the four supports-on cells;
all eight outputs must remain byte-identical; the four supports-off cells must
show no median wall or CPU regression. The **8/8 destination is unchanged**. The
candidate is graduated as
[Strong planner occupancy-offset batching: representation-safe four-cell campaign](issues/125-strong-planner-occupancy-offset-batching.md),
open and unclaimed. This was a ruling-only take: no production edit, build or
timing campaign. **Frontier now:**
[Strong planner occupancy-offset batching: representation-safe four-cell campaign](issues/125-strong-planner-occupancy-offset-batching.md)
is open and unclaimed. The per-layer/emit program remains the next stage's fog.

**Previous selection scope (2026-11-07, twenty-seventh session — the t124 take):**
the human said "Continue the perf vs orca map, pick up the next afk ticket" (no
ticket named). The frontier held **124 open and unclaimed** (117 is the human's
grilling), so 124 was claimed and **resolved as a measurement**. Headline: **the
post-t123 scoreboard is re-derived — PnP loses all eight cells at 2.36x–4.65x
ordinary / 2.40x–4.67x accelerated**, and the required cut to beat Orca is
**58%–79%** of today's PnP wall (**72%–86%** for a 1.5x margin). Every cell's
ratio improved against t121's pre-t123 **2.78x–5.11x**, but the move is a mix of
both arms (PnP ordinary medians **−25.7% … +0.7%**; Orca medians
**−12.3% … +11.5%**), so **no causal claim is taken from the cross-take
comparison** — t123's paired A/B remains the causal measurement. The base
supports-off PnP deltas (**−24.2% / −25.7%**) agree with t123's paired
**−22.2% / −20.9%**. **Proof:** 96 rows = 8 cells × 3 tools × (1 warmup + 3
measured), **zero starvation exclusions**, PnP ordinary ≡ accelerated bytes in
8/8, every PnP hash equal to the t123 candidate set, 64/64 PnP runs
`degraded=false` / `non_fatal=0`, 32/32 Orca runs pinned
(`enable_arc_fitting = disabled`, `elefant_foot_compensation = 0`,
`brim_width = 0`) with the repaired filament leg `1.24 / 12 / 220`. Both
freshness gates exit 0; the arms are the t123 candidate binaries
(`7737a74f…` / `a6ae2adc…`) and `git diff c598f098..HEAD` is docs-only.
**Disclosed:** the first batch was killed by the host mid-cell at 13:14 local
(79 of 96 runs); its `base classic-on` partial was discarded whole (named in
`discarded_partial_sources`) and re-taken in its own interleaved batch, and
`base arachne-on` followed in a second — each cell is internally paired, so no
ratio spans windows. See
[ticket 124](issues/124-post-t123-orca-scoreboard.md) and
[`evidence/t124-post-t123-scoreboard/`](evidence/t124-post-t123-scoreboard/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only, 10/10 negative controls
rejected). The standing adhesion asymmetry (PnP 8 mm brim vs pinned Orca 0) is
disclosed. **Frontier now: no open AFK child; 117 is the human's grilling.**
The per-layer/emit program remains the next stage and is unchanged by this
take.

**Previous selection scope (2026-11-06/07, twenty-sixth session — the t123 take):**
the human said "Continue the perf vs orca map, pick up the next afk ticket" (no
ticket named). The frontier held **123 open and unclaimed** (117 is the human's
grilling), so 123 was claimed and **resolved as a BUILD**. Headline: **retiring
the facet bridge path and classifying canonical's whole unsupported bottom set
is output-bounded and clears the 0.80 % floor in all eight cells.** The
PnP-local facet split — `classify_region_surfaces`' facet point-in-polygon scan,
`assemble_bridge_areas`' 1.0 mm `expansion_margin_mm` offset, the flat-family
special case, the postpass `gate_bridge_areas_by_unsupported_span`, and the
bottom-surface footprint cache — is replaced by
`classify_unsupported_bridge_areas` applying canonical
`PrintObject::detect_surfaces_type`'s
`opening_ex(diff(current_raw, safety_offset(previous_raw)), 0.1 x
external_perimeter_width)` to the region's whole unsupported set.
**16-cell output: met** — baseline ordinary ≡ baseline accelerated ≡ the
post-t116 set in all eight cells, candidate ordinary ≡ candidate accelerated,
candidate moves every cell; benchy grows **+873 / +832 / +1,281 / +1,159** bytes
and base shrinks **~20.3 KB** in every cell. **Delta audit: 0 structural
geometry lines in all eight cells** — every wall/support difference is
`E`-token-only on unchanged `G1` motion or a non-motion command (`M73`,
`M106`/`M107` fan, appendix); no coordinate, feedrate, travel, skirt, brim or
raft line moved. **Paired gate (3 interleaved pairs):** median wall **4.91 →
4.53 / 8.92 → 8.65 / 5.00 → 4.65 / 8.93 → 8.68 s** (benchy classic-off/on,
arachne-off/on) and **49.28 → 38.33 / 116.48 → 108.07 / 48.92 → 38.68 / 117.88
→ 108.52 s** (base), i.e. **−2.73 % … −22.23 %**, **3.4x–27.8x** the floor,
with process CPU **−1.0 … −110.2 s** in every cell; every cell is strictly
separated on wall and CPU. **Ledger:** the base win is mostly the removed facet
work (t122 measured the facet path at 27x the canonical classifier for a
1.04x set), and the benchy win is the same removal on a set that grows 1.85x;
the removed bottom-surface footprint is an extra saving the t122 screen did not
price. `DEV-200` records the unported wave expansion / zone trim; `DEV-039`
(the retired bounding-box fallback) is closed. See
[ticket 123](issues/123-facet-bridge-path-retirement.md) and
[`evidence/t123-facet-retirement/`](evidence/t123-facet-retirement/FINDINGS.md)
(`verify.py` exits 0 checkout-only, 309 checks; `--self-test` rejects 11/11
negative controls). The now-stale scoreboard is graduated as
[124](issues/124-post-t123-orca-scoreboard.md): every t121 ratio predates this
take.
**Frontier now: 124 open and unclaimed; 117 is the human's grilling.** Standing
paired acceptance and KEEP/DROP rules unchanged.

**Previous selection scope (2026-11-06, twenty-fifth session — the t122 take):**
the human said "Continue the perf vs orca map" (no ticket named). The frontier
held **122 open and unclaimed** (117 is the human's grilling), so 122 was
claimed and **resolved**. Headline: **the bridge-expansion half is mis-staged
as a `PrePass::Slice` change and its cost consumes the classification win on
benchy; the facet-path retirement alone is a strong BUILD.** Canonical's
`LayerRegion::process_external_surfaces` expands bridges into `stInternalSolid`
/ `stInternal` / `stTop` after perimeters and fill surfaces exist; in PnP the
sparse zone and `perimeter.infill_areas` are not created until the
`Layer::Perimeters` commit partition, while `PrePass::Slice` has none of the
three zones. A faithful port is therefore a per-layer partition change, not a
prepass one. Measured (serial thread-time, probe): classification-only nets
**+1,122.9 ms** benchy / **+94,852.4 ms** base; the expansion costs **1,151.9 /
4,976.6 ms**, i.e. **102.6 %** of the benchy saving and 5.2 % of the base
saving. **Classification-only clears the 0.80 % floor in all eight cells at
12.5x–24.5x (benchy) and 89.7x–213.9x (base); the combined shape is negative in
all four benchy cells.** Delta is bounded on both fixtures: benchy added
618.44 / removed 18.45 mm², expansion reach ≤300.64 mm²; base added 564.17 /
removed 384.82 mm², reach ≤513.85 mm². **Ledger correction:** the current facet
path's pre-gate polygon sum is 14,232.05 mm² (benchy) / 419,106.22 mm² (base) —
at most 5.0 % / 1.1 % survives the gate, so on base the expensive facet path
produces an almost identical set (1.04x) at 27x the canonical classifier's
cost. The classification-only candidate is graduated as
[ticket 123](issues/123-facet-bridge-path-retirement.md). See
[ticket 122](issues/122-prep-slice-wave-expansion-facet-retirement.md),
[`evidence/t122-wave-expansion/`](evidence/t122-wave-expansion/FINDINGS.md)
(`verify.py` exits 0 checkout-only, 14/14 negative controls rejected).
**Frontier now: 123 open and unclaimed; 117 is the human's grilling.** Standing
paired acceptance and KEEP/DROP rules unchanged.

**Previous selection scope (2026-11-05, twenty-fourth session — the t121 take):**
the human said "Continue" after the t119 resolution, then paused the Orca
campaign overnight and resumed it. **121 is resolved.** Headline: **the per-layer
transport was deriving prepared-region geometry for every module call, and for
`Layer::PerimetersPostProcess` it derived data the stage never consumes.** The
first is fixed by reusing the arena's `PreparedRegionData` when the
perimeter-source projection is the identity and every projected origin is
staged; the second by passing `derive_prepared = false` for that stage. **Four
classic cells, 3 interleaved pairs:** CPU **−27 % … −46 %** in **12/12 pairs**,
wall **−9.3 % … −37.5 %** in **11/12** (the exception is benchy-off pair 2 at
+0.66 % wall with −37.23 % CPU; the cell median is −14.29 %), and all four
G-code hashes identical pre/fix. **Per-layer bracket (base-off, instrumented):
7,265 pre-merge → 42,406 unfixed → 6,758 fixed**, below the pre-merge bracket.
Residual cost after the first fast path was isolated to `fuzzy-skin` and
`seam-placer` (the `PerimetersPostProcess` stage), which the stage-aware empty
view returned to their pre-merge aggregate. **Ledger correction:** the t115
`output_audit` hashes predate t116 (which moved every cell); the current
references are `35600ee4…` / `596f6da5…` / `6a6a3d9d…` / `8f46fe92…`, and the
fix is output-neutral against them. **Scoreboard (complete):** the
fresh post-merge Orca scoreboard is recorded — 8 cells × 3 tools × (1 warmup +
3 measured). PnP ordinary and accelerated G-code bytes are identical in **8/8**,
and PnP **loses every cell**: ordinary/Orca **2.78x–5.11x**, accelerated/Orca
**2.76x–5.10x** (best benchy arachne-off 2.78x, worst base classic-on 5.11x).
The destination's 8/8 is not met; the required cut is **64%–80%** of today's
PnP wall (76%–87% for a 1.5x margin). See
[ticket 121](issues/121-post-merge-per-layer-dispatch-regression.md),
[`evidence/t121-per-layer-dispatch/`](evidence/t121-per-layer-dispatch/FINDINGS.md)
(`verify.py` exits 0 checkout-only, 8/8 negative controls rejected) and the
[scoreboard](evidence/t121-per-layer-dispatch/SCOREBOARD.md)
(`verify_scoreboard.py --self-test` rejects 6/6).
**Frontier now: 122 open and unclaimed; 117 is the human's grilling.** Standing
paired acceptance and KEEP/DROP rules unchanged.

**Previous selection scope (2026-11-05, twenty-third session — the t119 take):**
the human said "continue the perf vs orca map, pick up the next afk ticket" (no
ticket named; **117** is the human's grilling, so **119** was the first AFK
frontier ticket in order). **119 is resolved.** Headline: **the lightning module
now observes each `(case, arm)` once — 41 → 32 active invocations — and the
serial module wall measured 688.89 s → 620.44 s (−9.94 %), with every
audit-listed assertion preserved.** A process-global `OnceLock<Corpus>` builds
five cases × two arms = 10 runs; `module_plan` re-checks both arms against the
recorded plan, `fresh_consumers` reuses the WASM arm for its emitted-segment
claims and keeps only its preparation + cancellation legs, and the legacy,
comparator, selection and retirement tests read the shared capture instead of
re-taking it. The runtime trace agrees entry by entry: 32
`lightning-invocation:` lines (`run_visual` 13, `run_visual_native` 5,
preparation 6, full slice 8). Audit fixes: the typed-projection scene-sharing
assertion now requests two layers (it was vacuous on one), the selection
test-local Boolean assertion is gone, `max_retained_after_disposal()` is
`Option<u64>` so cleanup cannot pass without a disposal, and the foreign-owner
block is a real cross-store `InvalidOwner` control; the zero-slice selection
predicate and store controls moved to a contract test. Disclosures: the box was
**not idle** (Opera/Stremio/Chrome, 43 % sampled), the arms are same-session
rather than interleaved, and a second after observation measured 611.64 s; the
saving is smaller than t114's pre-cache estimate because t118 had already made
each capture cheap. No full-bucket wall and no aggregate CPU claim. See
[ticket 119](issues/119-lightning-corpus-consolidation.md) and
[`evidence/t119-lightning-corpus/`](evidence/t119-lightning-corpus/FINDINGS.md)
(`verify.py` exits 0 checkout-only, 7/7 negative controls rejected).
**Frontier now: 121 and 122 open and unclaimed; 117 is the human's grilling.**
Standing paired acceptance and KEEP/DROP rules unchanged.

**Previous selection scope (2026-11-04/05, twenty-second session — the t116 take):**
the human said "continue the perf vs orca map, pick up the next afk ticket" (no
ticket named; **117** is the human's grilling, so **116** was the first AFK
frontier ticket in order), then paused the campaign overnight and resumed it.
**116 is resolved.** Headline: **the canonical flat-bridge rule is built, output-
verified and 8/8 above the floor.** The PnP-local enclosure closing
(`closing_ex(support, 12 mm, Miter)` + `≥ 10 %` refill) is replaced by canonical's
`opening_ex(flat_bridge, 0.1 × external_perimeter_width, Miter, 3.0)` over PnP's
flat family, with canonical's `ApplySafetyOffset` (1-unit lower inflate) and the
lower-layer guard (`lower_layer == nullptr` ⇒ `stBottom`); the
`flat_bridge_closing_join` key is retired end to end. **16-cell output: met** —
baseline ordinary ≡ baseline accelerated ≡ the t115 post-merge set, candidate
ordinary ≡ candidate accelerated, candidate moves every cell. **Delta stop rule:
not triggered** — `audit_delta.py` finds **0 structural geometry lines** in all
eight cells. **Paired gate: 8/8** at **11.1x–30.5x** the 0.80 % floor (median wall
−8.9 % … −24.4 %; CPU −16.6 … −288.9 s), narrowest base classic-on. Two ledger
corrections: the baseline check is against the **post-merge** set (the ticket's
post-t81 wording predates the merge), and after the key retirement **no declared
config key carries `wire-type: enum`**, so the scheduler manifest's enum
assertions retire too. Disclosures: the campaign was interrupted twice (session
recycle, then a requested overnight pause), so pairs 1–2 and pair 3 come from two
windows ~4 h apart; each pair is internally paired. No Orca arm — the post-merge
scoreboard stays [ticket 121](issues/121-post-merge-per-layer-dispatch-regression.md)'s
closing measurement. See [ticket 116](issues/116-prep-slice-canonical-bridge-classification.md)
and [`evidence/t116-prep-slice-canonical-bridge/`](evidence/t116-prep-slice-canonical-bridge/FINDINGS.md)
(`verify.py` exits 0 checkout-only, 316 checks; `--self-test` rejects 10/10
negative controls). **Frontier now: 119, 121 and 122 open and unclaimed; 117 is the
human's grilling.** Standing paired acceptance and KEEP/DROP rules unchanged.

**Previous selection scope (2026-11-04, twenty-first session — the t115 take):**
the human said "continue the perf vs orca map" (no ticket named) and asked that
any uncommitted work be committed; the t120 take's six files were committed
first as `9a7371aa`. The frontier at HEAD `f70e8ba7` held four open children —
[115](issues/115-post-merge-reconciliation.md),
[116](issues/116-prep-slice-canonical-bridge-classification.md),
[117](issues/117-strong-planner-carry-ruling.md) (the human's grilling) and
[119](issues/119-lightning-corpus-consolidation.md) — so the first frontier
ticket in order, **115**, was claimed and resolved the same session. Headline:
**the merge is output-neutral and per-layer-expensive.** Every matched cell's
ordinary output grows by exactly **+34 bytes** (the added config line
`; prepared_plan_diagnostic = none`), and a direct diff on the four classic cells
returns that single hunk — no toolpath re-record is needed, only the hash set
moving (`22200671… → 4b6d2843…` on base classic-on). Painted fixtures do move:
`cube_4color.3mf` loses **1.08 %** of its bytes and redistributes up to
**8.19 %** of extrusion between tools while total extrusion moves **−0.11 %**,
consistent with the classic paint-attribution fix; the map quotes no painted
ratio, so nothing it says depends on it. The **performance** side is the real
finding: a paired pre-merge-vs-merged A/B (preserved t103 developer snapshot vs
the current ordinary build, 3 interleaved pairs, load 71 % disclosed) puts
**process CPU up in 12/12 pairs** (+24.1 / +17.0 / +53.6 / +34.9 % medians,
benchy-off / benchy-on / base-off / base-on) and the whole cost in the
**per-layer phase** (incrementally 2.23x / 1.91x / 6.02x / 5.08x; instrumented
base-off 7,265 → 42,886 ms = 5.90x with the prepass flat at −0.3 %). Module
brackets show the same 495 call counts with `fuzzy-skin` 162 → 56,068 ms and ten
more modules at a near-uniform ~28.5 s, and the per-call cost rises with layer
index (55 → 141 ms) — a cost proportional to committed state, not guest work.
The accelerated dist build is unaffected (2,318 ms per-layer on benchy-off vs
ordinary 4,807–5,860 ms, same hash), so a build-mode-sensitive host path is the
suspect. All structural keeps are present (t78/t81/t90/t95/t101/t102); their
prices were not re-measured because the regression dominates their phase. The
ruling: **no ratio may be quoted against the merged tree**, the post-merge Orca
scoreboard is deferred (not dropped) to the fix's closing gate, references move
to the post-merge set, and the regression is graduated as
[121](issues/121-post-merge-per-layer-dispatch-regression.md). Evidence:
[`evidence/t115-post-merge/`](evidence/t115-post-merge/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only, 403 checks, 6/6 negative
controls). Frontier now: **116, 119, 121** open and unclaimed; **117** is the
human's grilling. Standing paired acceptance and KEEP/DROP rules unchanged.

**Previous selection scope (2026-11-04, nineteenth session — the t114 take):** the
human said "continue the perf vs orca map" (no ticket named). The frontier at HEAD
`7eeff93e` held four open children — [114](issues/114-e2e-bucket-latency.md),
[115](issues/115-post-merge-reconciliation.md),
[116](issues/116-prep-slice-canonical-bridge-classification.md),
[117](issues/117-strong-planner-carry-ruling.md) — so the first frontier ticket in
order, **114**, was claimed and **resolved the same session as a Phase-1 screen**.
Headline: **the e2e bucket is 85 % fixed per-slice module load.** One debug slice
of `resources/calicat.stl` through the 24 core modules is **11.732 s** median; the
instrumented run puts **1.833 s** in the four pipeline phases and **10.011 s** in
module load + startup + emit. The loader builds a fresh `WasmEngine` and compiles
every on-disk component **serially inside the binding loop** in
`load_live_modules_for_plan_manifest_first`
(`crates/slicer-wasm-host/src/execution_plan_live.rs`) with **no cache**. Inventory
(serial, 178 tests covered): the non-whale modules sum to **854 s (14.2 min)**,
`lightning_migration_tdd` measured **1,411 s for 6 of its 10 tests** (~39 min
extrapolated) and `preparation_runtime_lifecycle_tdd` is the ticket's recorded
**14-of-16-in-30-min** module (~34 min extrapolated) — the two whales are **83.8 %**
of the **5,263 s (≈88 min)** extrapolated suite, whose measured/cited floor is
**4,065 s (≈68 min)**. The counter-example is `slice_end_to_end_tdd`: 42 tests in
**201 s** because **~10 `pnp_cli` runs** back all of them (23 `cached_run` call
sites over 3 keys + 7 uncached determinism runs). A reverted 24-thread
compile probe (same G-code, `d12cdfd1…`) removed only **2.431 s (−20.7 %)** — the
compile already saturates the cores per component, so **parallelising the loop is
not the lever**. **Pruning: the first pass's "none justified" was too strong.** A
five-agent adversarial audit (postscript in the ticket, tables in
[`evidence/t114-e2e-latency/AUDIT.md`](evidence/t114-e2e-latency/AUDIT.md)) found
decorative, fake, success-only and never-running tests plus **~11 min** of
repeated observations (423 s lightning + 120 s prep + ~95 s small-module merges
+ 12 s precision). Eight claim fixes/merges were adopted — a dead disk
support-blocker test revived, both test-local `scenario_2` fakes deleted (the one
real `PaintSemantic` order claim moved to `slicer-ir`), a lying override test
renamed, an empty-tolerant pipeline test tightened, and
`modifier_infill_boundary_anchoring` merged into its identical sibling
(**38.76 s → 26.78 s, −11.98 s**); the precision merge removed a slice but landed
inside noise (**86.66 → 86.71 s**, not claimed). The ~**237 pipeline starts**
still dominate: at 10.0 s each that is **~39.5 min of pure loading**, so
deduplicating all ~11 min still leaves ~77 min — the cache remains the carrying
fix. **Lane split: not viable** — a fast lane would have to exclude every
slice-running module, leaving ≈ 22 s of pure-unit/model-IO tests. **Concurrency:**
`libtest`'s 12 threads oversubscribe a box each test already saturates (one
`lightning` test process used **6.8 cores**); `mm_painted_fixture_t0_t1` measured
**23.25 s alone vs 200.622 s** under 12 concurrent processes, an **upper bound**
because a game held ~6.6 cores during that arm. `cargo nextest` 0.9.151 is **not
adopted** (process-per-test discards the in-process `slicer_cache`/`wasm_cache`
sharing). **Ledger correction:** the 40-minute baseline was **under-counted** —
nine `pnp_cli`-backed e2e tests fail in 0.1 s on the stale-binary gate under plain
`cargo test` (which does not build `pnp_cli`), so their 12–29 s each were absent
from the 93-passed observation; `cargo xtask test` owns that preflight
(`ensure_pnp_cli_fresh`). The carry decision was graduated as
[118](issues/118-compiled-component-cache.md) and **executed the same session**
after the human set a steep milestone: the process-global component cache,
wasmtime's validated disk cache, per-layer capture parallelism and corpus-case
parallelism bring the full bucket to **755.12 s (12.59 min), 172 passed / 1
ignored — under the 15-minute gate** (t114's ≈4.5 min model was optimistic; the
lightning captures do 38–52 s of real pipeline work each). Evidence:
[`evidence/t114-e2e-latency/`](evidence/t114-e2e-latency/FINDINGS.md)
(`verify.py` exits 0 checkout-only, 29/29 checks; `--self-test` rejects 11/11
negative controls). Frontier now: **115, 116, 119** open and unclaimed, **117**
is the human's grilling; [119](issues/119-lightning-corpus-consolidation.md) owns
the remaining lightning corpus consolidation (**41 → 32** active invocations, all
assertions preserved) — the bucket already clears its gate without it, so it is
claim hygiene rather than latency. Standing paired acceptance and KEEP/DROP rules
unchanged.

**Previous selection scope (2026-11-04, eighteenth session — the t113 take):** the
human said "continue the perf vs orca map" (no ticket named). The frontier at HEAD
`19a5edfe` held four open children — [113](issues/113-strong-planner-phase1.md)
(blocked by the already-resolved 112), [114](issues/114-e2e-bucket-latency.md),
[115](issues/115-post-merge-reconciliation.md),
[116](issues/116-prep-slice-canonical-bridge-classification.md) — so the first
frontier ticket in order, **113**, was claimed and **resolved in the same
session as a Phase-1 stop**. Headline: the planner's largest term is **model-occupancy
inflation**, not the clip core. At HEAD the `com.core.tree-support-planner`
module is **58,498 ms** of a **200,099 ms** base classic-on cell (**3,279 ms** of
**15,333 ms** benchy-on; **84,337 ms** of **416,375 ms** base arachne-on;
**4,133 ms** of **27,122 ms** benchy arachne-on). Its split is **56.2 % / 46.0 %
/ 51.4 % / 41.4 %** `inflate_model_occupancy`, **31.0 % / 28.6 % / 35.9 % /
32.4 %** clip core, **6.2 % / 10.8 % / 6.0 % / 11.1 %** drop loop, **2.4 % /
6.0 % / 2.2 % / 5.4 %** contacts (order: base classic, benchy classic, base
arachne, benchy arachne). The occupancy inflation is **one singular
`host::offset_polygons` per call**, split evenly between the per-layer and
per-origin sites; batching it through `offset_polygons_batch` is exact by
construction (same `slicer_core::polygon_ops::offset`, input-order results;
t65/t78 precedent), needs **no canonical-parity exception**, and clears the
**0.80 %** floor on all four supports-on cells at **12.58x / 7.44x / 7.97x /
4.76x** the floor under t78's measured **2.6x** clamp (18.73x / 11.08x / 11.87x
/ 7.09x ideal), with **zero** expected output delta (the probe is
byte-identical to the HEAD baseline on base classic-on `4b6d2843…`, benchy
classic-on `3b8fa86a…`, benchy arachne-on `d4644c14…`). **But the all-8-cell
gate is structural:** the whole `PrePass::SupportGeometry` stage on the
supports-off cells is **300 ms = 0.22 %** of base classic-off and **46 ms =
0.51 %** of benchy classic-off, below the floor even if deleted, and the planner
module alone is **141 / 23 ms = 0.13x / 0.32x**. No planner-only or
support-stage shape can clear 0.80 % on those cells. The stop rule fired: no
candidate ticket, no production edit. The carry decision — fold the batch into
[116](issues/116-prep-slice-canonical-bridge-classification.md)'s Slice
campaign, amend the planner gate to the four supports-on cells with a
no-regression bound, or park it — is graduated as
[117](issues/117-strong-planner-carry-ruling.md) (open, unclaimed, unblocked;
HITL grilling). **Ledger correction:** t88/t107's "no host batch shape in the
non-clip residual" is falsified — the avoidance ladder, offsets, MST and node
moves are **6.2 % / 10.8 %** of the planner, while the occupancy offset batch is
**56.2 % / 46.0 %**. The `87,824 ms` t104 figure is an instrumented bracket; its
composition (≈49.3 s occupancy, ≈27.2 s clip, ≈5.5 s drop) is the transferable
result, not its absolute. Evidence:
[`evidence/t113-strong-planner-phase1/`](evidence/t113-strong-planner-phase1/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only, 6/6 negative controls rejected).
Frontier now: **114, 115, 116, 117** open and unclaimed; 116 is takeable and 117
is the human's grilling. Standing paired acceptance and KEEP/DROP rules
unchanged.

**Previous selection scope (2026-11-03, seventeenth session — the t112 take):** the
human said "continue the perf vs orca map" (no ticket named). The frontier at HEAD
`2dade816` held four open children — [112](issues/112-prep-slice-parity-phase1.md),
[113](issues/113-strong-planner-phase1.md) (blocked by 112),
[114](issues/114-e2e-bucket-latency.md), [115](issues/115-post-merge-reconciliation.md)
— so the first frontier ticket in order, **112**, was claimed and **resolved the
same day as build-P1**. Headline: **canonical OrcaSlicer has no enclosure closing.**
`PrintObject::detect_surfaces_type` labels *every* bottom area the layer below does
not cover as `stBottomBridge` — `opening_ex(diff_ex(slices,
safety_offset(lower_slices), Yes), external_perimeter_width / 10)` — and the only
canonical bridge closing is `merge_bridges`' `closing_ex` at **≈0.15–0.17 mm**
(0.55 × 0.65 × 1.05 × solid-infill spacing) on the *bridge ∪ wave-anchor* surface,
a hole filler. PnP's 12 mm `closing_ex(support)` + **≥ 10 %** refill gate is a
PnP-local discriminator with **no canonical counterpart and no `DEV` row**
(**~70–80×** canonical's radius, different operand, different consumer), so no
shape can preserve its verdicts *and* claim parity: the parity move is to **replace**
the gate. Measured at HEAD: the closing is **86.8 %** (benchy) / **70.8 %** (base) of
`PrePass::Slice`'s accumulated thread-time, and shape **P1** (canonical opening on
PnP's flat family, every surviving component kept) has a net ceiling of
**1,893 / 37,682 ms** = **15.3–24.7 % / 23.4–50.0 %** of the cell = **19.1x–62.5x**
the **0.80 %** floor in **all eight** cells, with the replacement opening
**146x / 853x** cheaper than the closing. Delta, measured on both fixtures and
audited per layer: **182 / 239** benchy and **437 / 469** base committed layers
change; net **+8.86 mm² (+2.0 %)** / **+115.90 mm² (+35.9 %)** of flat bridge area —
the gate rejected **68.4 / 187.2 mm²** that canonical calls bridges while the
0.04 mm opening removed **59.58 / 71.27 mm²** of sub-0.08 mm features. The first
layer also pays a 12 mm closing whose result `gate_bridge_areas_by_unsupported_span`
clears (**595.8 ms** benchy, 1.1 ms base). **P2** (full canonical classification,
retiring the facet-derived `assemble_bridge_areas`) is **deferred**: its delta is
unbounded until canonical's wave expansion (`propagate_waves_ex`) is ported, and
PnP's fixed 1.0 mm `expansion_margin_mm` is not it. **P3** (radius swap on the
existing gate) is **rejected** on correctness. The candidate is graduated as
[116](issues/116-prep-slice-canonical-bridge-classification.md) (open, unclaimed,
unblocked) under t111's 16-cell contract. Evidence:
[`evidence/t112-prep-slice-parity/`](evidence/t112-prep-slice-parity/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only, 9/9 negative controls rejected).
Frontier now: **113, 114, 115, 116** open and unclaimed — 113's blocker (112) is
resolved, so it is takeable. Standing paired acceptance and KEEP/DROP rules
unchanged.

**Previous selection scope (2026-11-03, sixteenth session — the prepass-rewrite
ruling):** the human said "continue the perf vs orca map" (no ticket named). The
frontier re-derivation at HEAD `f4a73892` found **110/110 children resolved — no
open child, no claim** — so the fog routes were offered and the human selected
**rule on the prepass rewrite**, graduated as
[111](issues/111-prepass-rewrite-scope-ruling.md), claimed and **resolved the
same day**. The ruling **authorizes a scoped canonical-parity exception for the
prepass**: both carrying shapes are covered, **`PrePass::Slice` is the first
campaign** (41,472 / 44,558 ms; 51.2 % of base classic-off, 19.5 % of base
classic-on) and the Strong planner is second (87,824 ms; 38.4 % of base
classic-on), graduated as [113](issues/113-strong-planner-phase1.md) and blocked
by 112. Candidate acceptance needs a fresh **16-cell** reference set, arms
byte-identical within themselves, the baseline reproducing the post-t81 set, the
candidate-vs-old-reference G-code delta audited before timing, and the standing
paired wall/CPU gate at the **0.80 %** floor in **all 8 cells**. The first step
is a **Phase-1 parity analysis**, graduated as
[112](issues/112-prep-slice-parity-phase1.md) (open, unclaimed); no production
edit happens before it clears the floor with a bounded delta. The rewrite is
**stage 1** of a multi-stage engine program — deleting both largest prepass
stages still leaves base at **2.86x–4.00x** inside t104's instrumented
arithmetic, so the per-layer/emit program is the next stage's fog. One ledger
caveat is carried into the ticket and remains a fog note: t110's base-on
**9.49x / 16.3x / 19.6x / 13.7x** figures divide an **instrumented** PnP span by
Orca's quiet wall; t104 discloses that pass at **+43.6 %** over its t103 median
and the uninstrumented paired record is **6.02x** (t106), so those base-on
multiples are upper bounds until corrected.

**Post-merge directive (2026-11-03, after master merge `a5e9a403`):** the human
directed that the two archive-dependent `preparation_capability_pilot_tdd` tests
be deleted — done in `69ef50f3`, because the gitignored pre-edit archive
`target/preparation-pilot/plain-before.component.wasm` cannot exist on a fresh
checkout and the current guest cannot reproduce its recorded SHA; the remaining
nine pilot tests pass — and that two tickets be opened from the merge
validation. [114](issues/114-e2e-bucket-latency.md) owns the **e2e-bucket
execution latency**: the full `slicer-runtime` e2e suite exceeds **40 minutes**
and is the dev-loop long pole, so this continues t108/t109's work on the
**execution** side rather than the compile side. The human also ruled that
**cutting/deleting e2e tests is fully in scope** on 114; the ticket now carries
a pruning pass with a per-deletion coverage map rather than a no-deletion
constraint. [115](issues/115-post-merge-reconciliation.md)
owns **post-merge reconciliation**: re-baseline the perf keeps, audit master's
semantic drift, the classic paint-attribution output change, and the
config/disclosure deltas before the map quotes another ratio. Both are open and
unclaimed; the 16th session does not claim either.

**Previous selection scope (2026-11-03, fifteenth session — the t110 take):** the
human said "continue the perf vs orca map" (no ticket named). The frontier
re-derivation at HEAD `1e72d9e3` found **109/109 children resolved — no open child,
no claim** — so six fog routes were offered and the human selected **Orca-side
attribution**, graduated as [110](issues/110-orca-side-attribution.md), claimed and
**resolved the same day as measurement-only**. Headline: **no per-stage timing
channel works on this install** (the `--logfile` CLI log buffers — 27 stage markers
arrive in **3 bursts**, 18 of 27 together, and carries no timestamps at `--debug`
3/4/5; ETW is refused without elevation — "Failed to enable the policy to profile
system performance" / "NT Kernel Logger: Invalid flags (0x3ec)"; `--pipe` accepted
the flag (`Will use pipe …`) but never connected to a listening server in two
attempts), so the finest observable boundary is a **~15 ms directory tap** on
`plate_1.gcode.tmp`. What it measures reframes the map's headline: **Orca's base
cells are import-dominated — 73.3 % / 42.0 % of wall is startup + model import
(10.13 s on the 123 MB `base.stl` against PnP's ≤ 0.708 s, ≥ 14.3x)** — and its whole
slicing phase is only **0.64 / 1.28 / 3.68 / 14.01 s** (benchy-off / benchy-on /
base-off / base-on) against PnP's **7.37 / 11.78 / 80.22 / 228.32 s**, i.e.
**11.6x / 9.2x / 21.8x / 16.3x**. The ratios of record (5.86x / 9.49x / 4.34x / 4.96x
today) therefore **understate the engine gap by 1.7x–3.7x**: PnP's fast import has been
offsetting part of a much larger slicing deficit, and PnP's **per-layer phase alone**
(1.54 / 1.74 / 6.54 / 12.70 s) is larger than Orca's **entire slicing**
(0.64 / 1.28 / 3.68 / 14.01 s) in three of the four cells. The prepass is the carrying phase
on **both** engines (Orca 57.1 % / 74.6 % of its base slicing; PnP 89.5 % / 89.4 %),
so the destination's prepass-rewrite ruling is not a PnP phase-mix artefact; on the
t105-pinned supports-on job Orca's prepass grows **+8.34 s** where PnP's support
stages add **~113.9 s** (**13.7x** on the increment; **8.4x** for the planner alone
against Orca's whole supports-on prepass). The feature stack is nearly
free on Orca's side — 1 wall loop, 1 shell, 0 % infill and no support still costs
**13,639.6 ms** against the full cell's 13,814.5 ms — which kills the "Orca does less
prepass work" explanation. **Ledger correction:** t104's Table 3 compares against
"Orca's whole slice (2.03 / 2.29 / 15.28 / 19.49 s)", which are *process* walls that
are 66–73 % import on base; t104's shares and counterfactuals stand, only that
reading changes. Evidence:
[`evidence/t110-orca-phases/`](evidence/t110-orca-phases/FINDINGS.md) (`verify.py`
exits 0 checkout-only, 7/7 negative controls rejected). Frontier now: **no open
child**; the destination's prepass-rewrite decision is unchanged and still the
human's to rule on, now with the engine multiple measured. Standing paired
acceptance and KEEP/DROP rules unchanged.

**Previous selection scope (2026-11-02, fourteenth session — the t109 take):** the
human said "continue the perf vs orca map" (no ticket named). The frontier
re-derivation at HEAD `59b1b21f` found **108/108 children resolved — no open
child, no claim** — so six fog routes were offered and the human selected **fix
the two harness defects** recorded by t108, graduated as
[109](issues/109-xtask-host-algos-injection-and-narrow-run-example.md), claimed
and **resolved the same day**. Headline: `xtask test`'s `--features
slicer-core/host-algos` injection is now conditional on cargo's selection
including `slicer-core`, so `cargo xtask test -p xtask --summary` exits **0**
with **193 passed** where it previously exited **101** with
`the package 'xtask' does not contain this feature: slicer-core/host-algos`;
the same error had blocked `slicer-config`, `slicer-ir`, `slicer-scheduler`,
`slicer-gcode` and `slicer-helpers`. The injection still fires for `-p
slicer-core`, for the workspace and for a `--manifest-path` naming
`slicer-core` (proof: the `slicer-core` arm compiles with the feature; a
`slicer-config` arm now reaches its tests). `AGENTS.md`'s narrow example now
names the module's real home — `cargo test -p slicer-scheduler --test
scheduler_contract core_module_ir_access_contract_tdd` — and runs **9 tests, 9
passed**, where the old command ran **0 passed; 314 filtered out**. The fix is
a pure `build_test_arguments` plus a selector parser covering
`-p`/`--package`, `--workspace`/`--all`, `--exclude`, `--manifest-path`,
attached forms and `name@version`, with five unit tests. **Two adjacent
findings recorded, not fixed:** `slicer-config`'s
`real_toml_manifest_census_rederives_expected_scope` is red **before and after**
(**293 vs 281**, reproduced with plain cargo; the fix only makes it reachable),
and `--no-run --summary` prints a build-may-have-failed line beside `VERDICT:
PASS`. Evidence: [`evidence/t109-harness-defects/`](evidence/t109-harness-defects/FINDINGS.md)
(`verify.py` exits 0 checkout-only, with cargo and `--full`; 5/5 negative
controls rejected). Frontier now: **no open child**; the destination's
prepass-rewrite decision is unchanged and still the human's to rule on, and the
dev-loop axis still has its five unpriced levers (cold-build `debug`,
cold-build linker, `cargo-nextest`, `sccache`, CI). Standing paired acceptance
and KEEP/DROP rules unchanged.

**Previous selection scope (2026-11-02, thirteenth session — the t108 take):** the
human said "continue the perf vs orca map, create and complete a ticket focused
on improving Testing and Compilation speed", quoting a discussion of Rust
iteration-speed practice (ra-jit/Blaze, mold/lld, sccache/rust-cache,
cargo-nextest, selective `opt-level`, CI caching, agent-loop discipline). That is
a **human-directed scope extension**: this map's destination is slice
throughput, and the dev loop's compile/test latency is now an axis of the effort
by explicit ruling. [108](issues/108-dev-loop-latency-preflight-concurrency.md)
was graduated, claimed and **resolved the same day as KEEP**. Headline: the three
`xtask test` preflights were strictly serial read-only scans; they now run
concurrently, and a representative invocation falls from a **median 18.771 s to
10.802 s** (**−7.969 s, −42.4 %**, four interleaved pairs with frozen harness
binaries; the slowest after sample 13.214 s is faster than the fastest before
sample 18.438 s; the pre-declared floor was **5 s**). Reporting order is
preserved, so a literals or test-quality failure still aborts before a guest
rebuild, and the freshness check keeps the single-toolchain-probe `Invocation`
on the calling thread. The inventory is the rest of the output: warm no-op
check/clippy 0.8–2.8 s, a one-line edit to `slicer-ir` / `slicer-core` 65.3 / 72.0 s,
the first codegen of the `contract` bucket **994 s** against **17.26 s**
incremental, relink-only 12.8 s, bucket execution 99.8 s, and a 89.7 MB contract
binary beside a 770 MB PDB. **Four premises falsified:** check/clippy *do* share
artifacts (1.7–2.4 s alternation, zero re-checks); feature sets do *not* thrash
for `slicer-runtime` (0.72 s no-op); the linker is *not* the incremental
bottleneck; and `debug = 0` is *slower* on the incremental path (28.24 vs
17.26 s). **Two adjacent harness defects recorded, not fixed:** the injected
`--features slicer-core/host-algos` makes `cargo xtask test -p <pkg>` fail for
every package that does not depend on `slicer-core` (A1), and `AGENTS.md`'s
narrow example filter `core_module_ir_access_contract_tdd` matches **no test**
(A2). **Blaze (ra-jit) was tried, not datasheeted:** #1527 ships no Windows
archive (packaging only, installer still unlanded), and #1591 builds here in
**57m34s** and starts a daemon, but needs two manual workarounds (`hard_link`
stand-ins read with `read_link` → `os error 4390` on every later start; a default
profile ACL refused by the store's `Access::Alone` check) and then refuses a
`serde` + `serde_json` binary at **241 std instances that need the LLVM backend**
(the author's LLVM build stops at `f16`/`f128` on Windows x64). Its LLVM rebuild
was abandoned after **92 minutes**. **On WSL2 it runs and loses to cargo on this repository:** against cargo 1.96.0 in Ubuntu 24.04 (12 CPUs), blaze's warm edit is **23-33 ms vs 248-779 ms** on a toy crate, but on `slicer-helpers` cold build is **193 s vs 54 s**, warm edit **12.4 s vs 0.95-1.13 s** (crate root), **11.9 s vs 1.4-3.3 s** (module) and **15.8 s vs 2.4-3.5 s** (function body), and `blaze test` is **312 s vs 49 s** (all 18 tests pass under both). **No blaze speedup is claimed for this repo.** Not priced and left as fog:
cold-build `debug` level, cold-build linker, `cargo-nextest`, `sccache`, and the
CI job-graph/caching half. Evidence:
[`evidence/t108-dev-loop-latency/`](evidence/t108-dev-loop-latency/FINDINGS.md)
(`verify.py` exits 0 checkout-only, 33/33 claims, 12/12 negative controls).
Frontier now: **no open child**; the destination's prepass-rewrite decision is
unchanged and still the human's to rule on, and the dev-loop axis has its own
unpriced fog. Standing paired acceptance and KEEP/DROP rules unchanged.

**Previous selection scope (2026-11-02, twelfth session — the t107 take):** the
human said "continue the perf vs orca map" (no ticket named). The frontier
re-derivation at HEAD `9a1921ac` found **106/106 children resolved — no open
child, no claim** — so five fog routes were offered and the human selected
**screen `PrePass::SupportGeometry`**, graduated as
[107](issues/107-support-geometry-producer-decomposition.md), claimed, and
**resolved the same day as measured-dead — the premise named the wrong stage.**
Headline: `PrePass::SupportGeometry` is **two stages sharing one id**, and
ticket 104's reducer summed them. Re-pairing t104's own committed raw streams
gives a **host producer of 9 ms on base classic-on and 0 ms on benchy classic-on**
(**0.0068x** and **0.00x** the 0.80 % floor) against a **planner module stage of
91,893 ms = 40.13 %** and **3,260 ms = 26.17 %** of their cells — the tree
planner **87,824 ms (95.6 %)** + the traditional planner 300 ms on base, and
2,943 + 30 ms on benchy. The producer's whole job is a coarse outline set —
**577 polygons / 444,259 points over 240 layers (~7.1 MB)** on benchy-on, per
`support-preview` — so its sub-millisecond-to-9-ms bracket is real work, and the
sentinel intermediate entries sit inside the same 0–9 ms. **No shape on the
producer can clear the floor**, so the ticket's stop rule fired before any
candidate pricing: no candidate, no campaign, no production edit, no probe build
(a probe patch was authored and reverted unbuilt once the bracket split
answered the question). The take's real output is a **ledger correction**:
t104's 91,902 / 3,260 ms is essentially the **planner**, i.e. t88's 55.5 s term
(implied **64.5 s / 3.25 s** today) that the map already carries as fog — the
supports-on lever is unchanged and the producer was never part of it. Evidence:
[`evidence/t107-support-geometry-producer/`](evidence/t107-support-geometry-producer/FINDINGS.md)
(`verify.py` exits 0 checkout-only, 71/71 claims, 9/9 negative controls). Frontier
now: **no open child**; the destination's prepass-rewrite decision is unchanged
and still the human's to rule on, with the supports-on carrying shape now
correctly named as the planner. Standing paired acceptance and KEEP/DROP rules
unchanged.

**Previous selection scope (2026-11-02, eleventh session — the t106 take):** the
human said "continue the perf vs orca map" (no ticket named). The frontier
re-derivation at HEAD `e2294453` found **105 of 106 children resolved** — ticket
106 open and unclaimed, exactly as the tenth-session note predicted — so it was
claimed and worked. Headline: **post-pin PnP ratios are 6.02x / 6.11x (base classic /
arachne) and 4.77x / 4.53x (benchy classic / arachne)**, every post-pin Orca
median is slower than ticket 103's pre-pin one (**+37.0 / +14.5 / +13.7 / +5.7 %**),
and the park still needs **66.9–75.5 %** of each cell removed. The t105 transfer
**agreed on two cells and missed on two** (base arachne-on **−17.95 %**, benchy
classic-on **+3.89 %** against a **0.093 s** tolerance); the pinned job is
**byte-identical to t105's `tree_strong` arm** on all four cells yet **1.9–9.6 %
faster today**, so the transfer's error is two cross-day drifts rather than the
pin, and it is retired as an absolute predictor. The take ran quiet (load
16.4 % → 3.3 %, no starvation exclusions) after an unrelated game process and a
**sibling checkout's** `cargo test` e2e binary had exited. Frontier now: **no
open child**; the destination's prepass-rewrite decision is unchanged and still
the human's to rule on. Standing paired acceptance and KEEP/DROP rules
unchanged.

**Previous selection scope (2026-11-02, tenth session — the t105 take):** the
human said "continue the perf vs orca map" (no ticket named). The frontier
re-derivation at HEAD `81219a66` found **104/104 children resolved — no open
child, no claim** — so five routes were offered from t104's post-resolution
state (closing-chain rewrite shape; supports-on engine fairness; redraw the
destination; park the map; lift the park and harvest per-layer leads) and the
human selected **supports-on engine fairness**, graduated as
[105](issues/105-supports-on-engine-style-fairness.md). The take measured
Orca's organic vs `tree_strong` arm on the four supports-on cells (1 warmup + 4
interleaved paired rounds; the campaign was paused at the human's request and
resumed) and the human ruled **pin to `tree_strong`**. Headline: the pin costs
Orca **+39.6 / +44.4 %** wall on the two base cells and **+7.6 / +9.5 %** on the
two benchy cells, moves the four supports-on ratios by **−7.1…−30.8 %** (best
supported-on cell still ~4.3x, park still needs 65–73 % cuts), and changes the
job (**+23.5–35.3 %** bytes). The pin is implemented as the standing rig's
default (supports-on only, per-run appendix assertion); **t103's supports-on
Orca medians are pre-pin**, and the re-derivation is graduated as
[106](issues/106-post-pin-supports-on-scoreboard.md). Frontier now: **ticket 106
only, open and unclaimed**. Standing paired acceptance and KEEP/DROP rules
unchanged.

**Previous selection scope (2026-11-02, ninth session — the t104 take):** the
human said "continue the perf vs orca map" (no ticket named). The frontier
re-derivation at HEAD `e61c5655` found **103/103 children resolved — no open
child, no claim** — so six fog routes were offered (prepass-vs-per-layer
attribution; supports-on fairness re-run; Orca-side attribution; `apply_opening`
round-join parity; `region_partition` dedup; re-aim the destination; adhesion
asymmetry) and the human selected **prepass-vs-per-layer attribution**, graduated
as [104](issues/104-prepass-vs-per-layer-attribution.md), claimed, and
**resolved the same day as measurement-only**. Headline: **the wall is the
prepass** — **68.5–89.5 %** of every classic cell, against **7.9–20.1 %** for the
per-layer pipeline — so the park's 57–82 % cut maps onto removing **77.8–93.9 %
of the prepass**; deleting the *entire* per-layer phase never reaches the park
(**3.02x / 4.51x / 4.66x / 7.60x** remain, benchy-off / base-off / benchy-on /
base-on) while deleting the entire prepass would (**0.52x / 0.87x / 1.19x /
1.25x**), and the **two biggest prepass stages together still leave 1.55x–3.34x**
(closest: base classic-off at **1.55x**, 3 % short). `PrePass::Slice` is the
single largest measured term — **41,472 ms = 51.2 %** of base classic-off
(28.4 / 18.1 / 19.5 % of the other three) — and on the supports-on cells
`PrePass::SupportGeometry` dominates (**40.1 %** of base classic-on, 26.2 % of
benchy classic-on; with `SupportAnalysis` ~50 % of that cell), the `DEV-156`
style-substitution site — **corrected 2026-11-02 by
[ticket 107](issues/107-support-geometry-producer-decomposition.md): that 40.1 %
is the planner module stage that shares the stage id, not the host producer
(9 ms); read this row as producer + planner.** **Ledger corrections:** t73's 2026-10-18 base-off split
(prepass 115.0 s / per-layer **94.3 s** of a 214.5 s instrumented wall) is
superseded — the per-layer phase is now **8.7 %** (7,016 ms of 80,924 ms), and
the change happened between t73 and t91 with **no per-KEEP decomposition
claimed** — while t91's 2026-10-27 base-on prepass share (149,723 / 168,319 =
**89.0 %**) re-measures at **89.4 %**, so the split's semantics are unchanged.
Every per-layer fog entry is now bounded by that 6.5–12.7 s phase. **Walls are
not claimed**: all four uninstrumented sets ran under external load (a sibling
checkout's integration-test binary, then `clippy-driver` + two browser
processes), inflation **1.32x–1.80x**, load falling 95.6 % → 16.8 % mid-batch; the
human ruled "proceed with noise disclosed"; ratios of record are t103's paired
quiet medians and shares are one instrumented pass per cell, clean and
byte-identical to the t87 references, with a benchy repeat moving no share by
more than **5.92 pp**. Arachne unmeasured. No candidate, no campaign, no
production edit; `verify.py --self-test` exits 0 checkout-only (70/70 claims,
9/9 negative controls). Frontier now: **no open child**; the destination has
become an explicit **prepass-rewrite decision** whose two named shapes (the slice
closing's consumer, and the support-engine style question) are the human's to
rule on. Standing paired acceptance and KEEP/DROP rules unchanged.

**Previous selection scope (2026-10-31 → 2026-11-01, eighth session — the ticket
103 take):** the human said "continue the perf vs orca map" (no ticket named). The
frontier re-derivation at HEAD `c135e3e7` found exactly one open child,
[103](issues/103-orca-rebaseline-repaired-rig.md), which was claimed and is
**resolved (2026-10-31 campaign / 2026-11-01 disposal)**. The rig now carries the
ruling's three pins in `New-OrcaOverrideArgs`
(`evidence/matched-pair/run_scoreboard.ps1`) and `Test-OrcaEvidence` asserts each
resolved value per run (`enable_arc_fitting = disabled`,
`elefant_foot_compensation = 0`, `brim_width = 0`) plus the filament leg
(`1.24 / 12 / 220`), rejecting the archived hardcoded-default signature
(`0 / 2 / 200`) outright — all **32 Orca runs** resolved the six values, so a
future install fails closed instead of drifting. **Both arms were rebuilt from
HEAD and measured today-vs-today** (ordinary `4cb96942…`, accelerated
`80897e62…`, 24/24 external modules, both freshness gates exit 0; vendor stamp
installed `2.8.0.16` == upstream `02.08.00.16`).

**Headline: Orca still wins all 8 cells at 3.52x–8.25x** — benchy classic-off
**3.78x/3.78x**, benchy arachne-off **3.52x/3.59x**, benchy classic-on
**5.43x/5.50x**, benchy arachne-on **4.78x/4.85x**, base classic-off
**4.93x/4.96x**, base arachne-off **4.72x/4.58x**, base classic-on
**8.25x/8.21x**, base arachne-on **7.00x/7.53x** (ordinary/accelerated). The
**park stays binding**: every cell needs a **57–82 %** wall cut to reach ~1.5x,
i.e. **72–102x** the map's whole 0.80 % floor; the release bound is **3.52x**
versus 2026-10-25's 2.71x, and the cause is the **Orca side** moving fast on
benchy (−24…−35 %), not a PnP regression (PnP benchy −2.9…+1.9 %; base
supports-on −6…−14 % from t102). **The archive's error was not
one-directional:** fresh Orca is +6…+18 % on base, so t98's one-cell inference
"every recorded multiple is optimistic for PnP" is **falsified for the four base
cells** (base classic-on 9.31x → 8.25x, base arachne-on 9.36x → 7.00x). No
mechanism is asserted for the base slowdown. The **adhesion asymmetry** is
measured beside the numbers: PnP resolves `brim_width = 8` in all 16 groups,
pinned-back Orca `0` in all 32 runs; its price is ~235 ms, but the job is not
identical and closing it is its own take with its own human decision.

Proof gate: PnP 16/16 `degraded=false`/`non_fatal=0`, ordinary ≡ accelerated
byte-for-byte in all 8 cells, and the PnP output is **exactly the t87 post-t81
reference set** in all eight cells (only Orca moved). Two starvation exclusions
are **named** (`base-arachne-on-orca-m3`, `base-arachne-on-pnp-ordinary-m3`);
three whole-cell re-runs were attempted and all were spoiled by external load (a
competing integration-test binary in another checkout, then sustained browser
CPU at 78–96 %); the human was asked and instructed the take to proceed with the
noise disclosed, and all three spoiled CSVs are preserved. That cell's
two-clean-round band is 6.95–7.05x / 7.23–7.53x and its verdict is unaffected.
Evidence: [`evidence/t103-orca-rebaseline/`](evidence/t103-orca-rebaseline/SCOREBOARD.md)
(`verify.py --self-test` exits 0 checkout-only, 13/13 negative controls
detected). Frontier now: **no open child remains**; the fog and the next route
are the human's. Standing paired acceptance and KEEP/DROP rules unchanged.

**Previous selection scope (2026-10-31, seventh session — the ticket 101 take):**
the human said "continue the perf vs orca map" (no ticket named). The frontier
re-derivation at HEAD `681c0daa` found exactly one open child,
[101](issues/101-support-identity-table-materialisation.md), which was claimed
and is **resolved (2026-10-31) as a Phase-1 screen: both exact families are below
the 0.80 % floor, so the ticket's own stop rule fires; no candidate, no campaign,
no production edit, recommended family 3 (keep)**. The shape census is one
deterministic census on all three table arms: **23,751 rows over 431 bodies**
(mean **55.1** paths per body), **34,662,046** demand ids, per-row list length p50
**1,322** / max **2,847**, **`mixed_bodies = 0`** (every row of a body carries a
byte-identical list), **418 distinct list contents**, and **428,726 retained ids =
1.24 %** of the blocks if one identity is shared per body — so the table is **one
body's demand list cloned once per emitted support path**, and the 1.2486 GB it
holds is 92 % of the collection's bytes. Both sides are measured **in situ**: the
allocation side by a per-layer `Instant` window around
`assemble_ordered_entities_with_support_identities`
(`crates/slicer-runtime/src/layer_executor.rs`) — table **413.3 / 433.8 /
452.7 ms** vs skip **21.1 / 17.3 ms** — and the free side by a **sampling-free**
`drop(layer_irs)` timing — table **432.4 / 494.1 / 434.4 ms** vs skip **2.614 /
2.890 ms**, with the census reproducing t100's **34,796,481 blocks /
1,358,397,544 bytes** exactly. **Ceiling 821.6 ms uncontended / 926.8 ms most
generous = 0.50–0.56 % of the cell = 0.62–0.70x the 1,318 ms floor**, and that
charges 100 % of the *parallel* assembly time to the critical path. The headline
is a **ledger correction to t100**: its 3.468 s free-side ceiling was a
**system-heap** artifact, and t102's mimalloc KEEP cut the same free
**3.526 s → 0.432–0.494 s (7.1–8.2x)** — the cause-level route had already been
banked by the allocator swap. The wall is **not claimed**: five runs on one
binary spread **150.195–154.793 s (3.0 %)**, 5.0x the whole ceiling, on a machine
the owner's browser was loading at 15–56 % of 12 cores. Consumers re-derived:
written only by the assembly, **read by no production path** (only
`structured_support_identity.rs` and `raft_geometry.rs` under
`crates/slicer-runtime/tests/integration/`), set empty by
`crates/slicer-macros/src/lib.rs` and `crates/slicer-runtime/src/anchored_rows.rs`,
absent from `crates/slicer-schema/wit/`, and proven unread end-to-end by **5/5
runs byte-identical to the post-t81 reference** with the table absent. Evidence:
[`evidence/t101-side-table/`](evidence/t101-side-table/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only, 12/12 claims, 15/15 negative
controls rejected). The resolution empties the frontier of children, so the
long-standing Orca re-baseline fog was graduated as
[103](issues/103-orca-rebaseline-repaired-rig.md) — **the only open child, and it
blocks any honest answer to "which cells are still lost"**. Its blocking ruling
was given the same day — **"pinned back"**: OrcaSlicer 2.5's version-flipped
defaults (`enable_arc_fitting`, `elefant_foot_compensation` 0 → 0.15,
`brim_width` 0 → 5) are pinned to the 2.4-era values, so the pins land in
`New-OrcaOverrideArgs` (`docs/specs/perf-vs-orca/evidence/matched-pair/run_scoreboard.ps1`)
and are asserted per run — so 103 is **unblocked and unclaimed**. The ruling
surfaced one **standing adhesion asymmetry** that every future scoreboard must
disclose: PnP's matched cell prints an **8 mm brim** (its skirt-brim module
default; layer 1 opens with 26 concentric rectangles, 12,040 mm of first-layer
extrusion against 2,167 mm on layer 2) while pinned-back Orca prints none. Its
slice-time price is small (`com.core.skirt-brim` reports **235 ms** for the whole
print) but the job is not identical, and closing it means changing the PnP side
of every cell and re-deriving the reference byte set — a separate take with its
own human decision. The take order is the
human's. Standing paired acceptance and KEEP/DROP rules unchanged.

**Previous selection scope (2026-10-31, sixth session — the ticket 102 take):**
the human ruled the allocator lever in scope after ticket 100 ("the map is a
speed study… feel free to test it"), so
[102](issues/102-global-allocator-substitution.md) was graduated, claimed and is
**resolved (2026-10-31, human-approved KEEP — candidate committed)**: `pnp_cli`
installs `AccountingAllocator<mimalloc::MiMalloc>`, the packager slot
`docs/16_slicer_report.md` already prescribes. Proof **16/16** (both arms
byte-identical, 8/8 ordinary cells equal to the t91 references, 8/8 cross-mode,
distinct binaries, identical 24-module trees), and the standing four-cell gate
**PASSED**: wall **−6.19 / −5.64 / −5.54 / −4.80 %** (−10.19 / −9.42 / −9.04 /
−7.41 s) with process CPU **−1.74 / −2.18 / −1.54 / −1.88 %**,
**24/24 pairs byte-identical**, 6/6 pairs per cell, zero starvation exclusions,
23/24 pairs wall-negative (accelerated arachne-on #6 is **+0.18 %**; that cell's
median is −4.80 %). Pre-gate **−7.74 % / CPU −2.25 %** on 3/3 pairs. The effect
is **~5–6 %** — far larger than the free-only price (**1.35–1.66 %**, t100) —
because the allocator acts on every allocation in the process, but it removes
**no allocations**: the census still shows the same 34,796,481 blocks, each
freed more cheaply. The cause-level route stays
[101](issues/101-support-identity-table-materialisation.md) (open, unclaimed).
Disclosures: the pre-gate ran while a stray integration-test binary pinned ~8
cores (walls 227–266 s); the campaign started at load median **1 %** after that
process exited (`target/t102-load-precampaign.txt`); today's baseline walls are
152–156 s against t91's 164.803 / 166.931 s, so ratios are today-vs-today and the
floor (t91's larger walls) is conservative; the allocator is process-wide, so no
stage attribution is claimed; memory is not gated; one transient `cl.exe`
failure on the accelerated build passed on retry. Evidence:
[`evidence/t102-allocator/`](evidence/t102-allocator/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only, 12/12 claims, 12/12 negative
controls rejected). Frontier now: **ticket 101 only, open and unclaimed**; the
take order is the human's. Standing paired acceptance and KEEP/DROP rules
unchanged.

**Previous selection scope (2026-10-30, fifth session — the ticket 100 take):**
the human said "continue the perf vs orca map" (no ticket named). The frontier
re-derivation at HEAD `cc2c54e6` found **99/99 child tickets resolved — no open
child, no claim** — so the session offered six measured fog routes and the human
selected **allocation-volume arena / bulk-free**. The fog patch graduated as
[100](issues/100-allocation-volume-layer-collection.md) and is **resolved
(2026-10-30) as a Phase-1 screen — measured-dead as scoped, no candidate, no
campaign, probe reverted**. The census: the whole-print free on ordinary base
classic supports-on is **34,796,481 blocks / 1,358,397,544 bytes / 3,526,544 µs**
(**101.3 ns per block**), of which **34,662,668 = 99.6 %** are 8–15-byte strings
sampled as `__skirt_…` / `demand-N` — the `support_entity_identities` side table
that `assemble_ordered_entities_with_support_identities` clones **per support
path** (23,751 rows × ~1,457 demand ids). Geometry is **0.094 %** of the blocks
and **7.8 %** of the bytes. A bulk free of the same bytes costs
**44.5–63.8 ms**, so the ceiling is **3.468 s = 2.10 %** of the cell =
**2.63x** the 1,318 ms floor — but flattening a live collection costs
**73.3–115.1 ms** of copy **plus the same storm** (**+73 ms worse**), and an
at-creation arena aimed at the geometry would remove 0.094 % of the blocks, so
the family dies on **aim and reachability**, not on price. The allocator arm
(mimalloc) cuts the drop **3.527 → 1.302 s measured** (**1.35 %**, 1.69x the
floor; 1.66 % / 2.07x corrected for the probe's 0.506 s sampling overhead), but
the shipped allocator is a **packager** decision
(`docs/16_slicer_report.md` §"Global allocator contract") and it removes no
work. The census graduates
[101](issues/101-support-identity-table-materialisation.md) (open, unclaimed):
price the table's allocation side, audit its consumers, and rule between
share-per-body, prune/gate (the t20 pattern) and keep. Evidence:
[`evidence/t100-allocation-arena/`](evidence/t100-allocation-arena/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only, 12/12 claims, 13/13 negative
controls rejected). Frontier now: **ticket 101 only, open and unclaimed**; the
take order is the human's. Standing paired acceptance and KEEP/DROP rules
unchanged.

**Previous selection scope (2026-10-30, fourth session — the ticket 99 take):**
the human said "continue the perf vs orca map" (no ticket named). The frontier
re-derivation at HEAD `6c1a2334` again found **98/98 child tickets resolved**
(t98's resolution was still uncommitted; it was committed first as `1240999c`),
so the session offered six measured fog routes and the human selected
**split-and-union the clip dilation**, the last representation-safe shape t96
left open. The fog patch graduated as
[99](issues/99-clip-split-union-dilation.md) and is **resolved (2026-10-30) as
REJECTED on price**: the most favourable split (offset every `deep_infill_area`
polygon on its own, then union the pieces) is **0.8230x / 0.8252x** of the
production call — order-independent within **0.27 %**, both arms byte-identical
to `292df501…` — but that is **314.3 / 298.1 ms** wall-equivalent =
**0.43 / 0.41 %** of the 73.17 s cell, **0.51–0.54x the 585.4 ms floor**. Even the
union-free bound (offsets only, 0.8056x) is **0.47 %**. No candidate, no campaign,
probe reverted. The reusable attribution: the call is **~80 % per-path Miter
offset**, **~19 % batched internal union/cleanup**, **~2 %** an explicit union of
the already-rounded pieces — splitting **partitions the cleanup** rather than
skipping work. On top, the split arm is byte-identical on **1/130** calls and
set-equal on **2/130** (`5,329` differing polygons, deterministic in both
orders), so Clipper2's discretized dilation does not distribute over the union.
The only surviving shape is t96's parity one (`determine_bridging_angle`), which
is out of scope absent a human ruling. Evidence:
[`evidence/t99-clip-split-union/`](evidence/t99-clip-split-union/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only, 6/6 negative controls rejected).
Frontier now: **no open child remains**; the fog and the next route are the
human's. Standing paired acceptance and KEEP/DROP rules unchanged.

**Previous selection scope (2026-10-30, third session — the ticket 98 take):** the human
said "continue the perf vs orca map" (no ticket named). The frontier re-derivation at
HEAD `6c1a2334` again found **all 97 child tickets resolved — no open child, no
claim** — so the session offered six measured fog routes (allocation-volume arena;
split-and-union clip-area dilation; `region_partition` 3x-per-layer dedup; repair the
Orca rig; supports-on engine fairness ruling) and the human selected **repair the Orca
rig**. The fog patch graduated as [98](issues/98-orca-rig-repair.md) and is **resolved
(2026-10-30)** as a repair that also falsified the archived Orca side. The 2.5 install
resolves `--load-settings` through a vendor preset bundle, so the rig now loads a
version-gated source-manifest tree (pinned to the tag whose `BBL.json` version equals
the installed `BBL.opc` stamp, `02.08.00.16` == `2.8.0.16`) plus CLI overrides, and
pins `enable_arc_fitting = 0` because 2.5 flipped that default to enabled.
Measured on the screening cell (benchy classic-off, median of three): **1.978 s** wall
/ 6.50 s CPU / 4,000,709 bytes against the archive's 2.787 s / 9.27 s / 3,890,392 —
because the archived Orca applied its **hardcoded filament defaults** (density 0,
max volumetric speed 2, first-layer temp 200) while the repaired rig resolves the
vendor chain (1.24 / 12 / 220). The archived Orca side was therefore doing less work
than the matched job asks: **every archived Orca median needs re-derivation before a
ratio is quoted**, and PnP's recorded multiples are optimistic for PnP. No ratio was
requoted. Evidence: [`evidence/orca-rig-repair/`](evidence/orca-rig-repair/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only, 4 claims, 7/7 negative controls
rejected). Frontier now: **no open child remains**; the fog and the next route are the
human's. Standing paired acceptance and KEEP/DROP rules unchanged.

**Previous selection scope (2026-10-30, second session — the ticket 97 take):** the
human said "continue the perf vs orca map" (no ticket named). The frontier
re-derivation at HEAD `f48a0488` again found **all 96 child tickets resolved — no
open child, no claim** — so the session offered six measured fog routes (match
the 5 mm branch distance; base + benchy bottleneck refresh; `region_partition`
dedup; split-and-union clip dilation; allocation-volume arena; host-removal gate
ruling) and the human selected **match 5 mm branch distance**. The fog patch
graduated as [97](issues/97-match-branch-distance-fairness.md) and is **resolved
(2026-10-30) as REJECTED on fairness — the premise is false, no profile edit, no
campaign.** Orca's `support_style = default` on a tree family resolves to
**organic**, and the organic engine reads only
`tree_support_branch_distance_organic`, resolved to **1 mm**. The normal
`tree_support_branch_distance = 5 mm` that the deviations table compares is inert
on that path. PnP is already at 1 mm, so the t88 arm was a 5x work reduction, not
a match. The ticket surfaced the real supports-on fairness gap — Orca organic vs
PnP's **Strong** substitution (`DEV-156`) — and the infrastructure blocker that
the 2026-10-27 Orca install no longer accepts the rig's file-path
`--load-settings`. Evidence: [`evidence/t97-branch-distance/`](evidence/t97-branch-distance/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only, 5/5 negative controls rejected).
Frontier now: **no open child remains**; the fog and the next route are the
human's. Standing paired acceptance and KEEP/DROP rules unchanged.

**Previous selection scope (2026-10-30, on the ticket 96 take):** the human said
"continue the perf vs orca map" (no ticket named). The frontier re-derivation at
HEAD `0b25a8b9` found **all 95 child tickets resolved — no open child, no
claim** — so the session offered six measured fog routes (windowed clip-area
dilation; match `tree_support_branch_distance` 5 mm; arena layer collection;
planner residual vs canonical; scoreboard + increment refresh; apply-opening
join parity) and the human selected **windowed clip-area dilation**. The fog
patch graduated as [96](issues/96-windowed-clip-area-dilation.md) and is
**resolved (2026-10-30) as a Phase-1 screen — REJECTED on exactness, no
candidate, no campaign, probe reverted.** The price is real and above the floor:
the whole-layer `+1.5 × spacing` dilation is 11.67–11.81 s accumulated over 130
calls, and a candidate-box window (8 × spacing, operand 11 × spacing) removes
**91 %** of it — **10.65–10.78 s saved = 1.57–1.60 %** of the ordinary base
classic-off cell, 2.0x the floor, the two runs agreeing within 1.2 %. But the
windowed arm deterministically changes the internal-bridge **direction**
(`5fc8fa72…` / 21,023,889 bytes against `292df501…` / 21,023,739; first differing
block is an `Internal Bridge` at `;Z:21.4`, 8 → 10 moves), and at that layer the
production and windowed clip areas are set-equal **inside the window**
(`mismatch_polys = 0`). The failure is the anchor consumer:
`internal_bridge_angles`' `expansion_area ∩ clip_area` term feeds
`boundary_anchor_polylines`; with qualification and `area` unchanged, the nearest
anchor must have lain beyond the window (or the cut introduced a closer ring), so
the anchor reach is not bounded by the planned 3.3 × spacing and no finite
candidate-driven window is safe. This is the
mirror of t93, where the same operand's **erosion** was safe because its
consumers are bounded by `2 × spacing`. The wall screen was never reached (the
single uninstrumented window-arm wall, 76.557 s, is unpaired and not a claim).
Fog now carries the surviving shapes: keep the whole operand and make the offset
itself cheaper (split-and-union over disjoint polygons, unmeasured), or change
the angle consumer (parity work). Evidence:
[`evidence/t96-windowed-clip-area/`](evidence/t96-windowed-clip-area/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only, 6/6 negative controls rejected).

**Previous selection scope (2026-10-29 → 2026-10-30, on the ticket 95 take):**
the human said "continue the perf vs orca map" (no ticket named). The frontier
re-derivation at HEAD `8f19a6a9` found **all 94 child tickets resolved — no open
child, no claim** — so the session offered six measured fog routes (batch the
exact-Z cross-sections; windowed clip-area dilation; arena layer-collection free;
rule on the four-cell gate; port support branch distance; re-measure the engine
gap) and the human selected **batch exact-Z cross-sections**. The fog patch
graduated as [95](issues/95-exact-z-cross-section-batching.md) and is
**resolved (2026-10-30, human-approved KEEP, candidate committed `afe9c361`)**.

The premise, re-verified against source before the offer: `validate_entry`
(`crates/slicer-wasm-host/src/support_aggregation.rs`) runs per entry in a
**serial** loop; each `ExactZQueryService::query` miss called
`cross_section_at_z`, which is `slice_mesh_ex(mesh, &[z])` — a **one-element**
Z array over the **2,461,234** triangles of `tmp/base.stl`'s world mesh. t91
measured **431 queries / 431 distinct `(object, region, z)` / 431 misses** — the
cache never hit, so every entry paid a full walk.

Phase 1 (probe, 3 slices, outputs byte-identical to the post-t81 references):
**431/431 planes equal polygon-for-polygon**, **6.42 / 6.72 / 6.84 s** of
single-Z walks collapsing to **0.99–1.05 s** in one batched `slice_mesh_ex` walk
= **5.37 / 5.73 / 5.83 s** saved = **3.26 / 3.48 / 3.49 %** of the cell,
**4.07–4.37x** the 0.80 % floor. The batch buys **6.4–6.9x**, not 431x, because
the batched walk pays two binary searches over 431 sorted planes and a straddle
test per triangle and is memory-latency-bound.

Candidate: per-`(object, z_units)` cross-section memo plus
`ExactZQueryService::prefetch_cross_sections`, called once per object from
`try_aggregate_support_plans_with_policy` before the serial loop; the
per-`(object, region, z)` result contract, the single-Z miss path and the
per-entry degradation are unchanged; host-only, no IR/guest/geometry change.
**Proof 16/16 PASS** (both arms byte-identical, 8/8 ordinary cells equal to the
post-t81 references, 8/8 cross-mode equal, 24 identical modules per mode,
four distinct host binaries). **Standing gate PASS in all four pre-declared base
supports-on cells**, 6/6 pairs each, zero starvation exclusions, 24/24 pairs
byte-identical: wall **−3.97 / −3.19 / −2.86 / −2.64 %** and CPU **−0.70 /
−0.70 / −0.90 / −0.15 %** (ordinary/accelerated × classic/arachne), wall deltas
**−6.17 / −5.68 / −5.43 / −5.49 s**, CPU sum **−27.2 s**. cpu/wall rises in every
cell (6.57→6.76, 6.67→6.87, 6.46→6.63, 6.39→6.61) — serial work removal
concentrating the same parallel work, the t93 artefact, not extra work. One
**disclosed** non-negative pair (accelerated classic-on pair 4, **+2.35 %**,
during a load event that slowed both arms ~10 s in that pair); the gated
statistic is the paired median. Pre-commit: workspace clippy clean,
`check-literals` 0, `check-test-quality --report` 0, contract 163/0, support
integration 22/0, both freshness gates exit 0. Evidence:
[`evidence/t95-exact-z-batch/`](evidence/t95-exact-z-batch/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only, 7/7 negative controls rejected).
Frontier now: **no open child remains**; the fog and the next route are the
human's. Standing paired acceptance and KEEP/DROP rules unchanged.

**Previous selection scope (2026-10-29, on the ticket 94 take):** the human said
"continue the perf vs orca map" (no ticket named). The frontier re-derivation
at HEAD `57234b81` found **all 93 child tickets resolved — no open child, no
claim** — so the session offered five measured above-floor leads from the fog.
The human selected the **remaining whole-print collection drop** and settled the
gate as **strict wall in all four cells** (not CPU-corroborated wall-neutral).
The fog patch graduated as
[94](issues/94-remaining-whole-print-drop.md) and is **resolved (2026-10-29) as
a Phase-1 screen — measured-dead, no candidate, no campaign, probe reverted.**
The split: the `host:gcode_emit` bracket is **93-94 % drop** — walk 176-253 ms,
free **3,242-3,558 ms**, bracket 3,415-3,811 ms — and the post-emission tail a
deferred free could hide inside is only **1,367-1,603 ms** (`machine-gcode-emit`
0.53-0.60 s + `GCodeSerialize` 0.72-0.92 s). **Parallel free is a wash**
(classic −0.321 s = +0.19 %, arachne +0.200 s = −0.12 %), so the free does not
scale on Windows' process heap; **layer-by-layer consumption is a structural
zero** (serial walk); and the **deferred-overlap absolute ceiling is the tail
itself — 0.97 % classic / 0.82 % arachne of the cell, 1.22x / 1.02x the 0.80 %
floor** — before any overlap loss, so no shape had margin for the strict
four-cell campaign. The screen leaves a precise structural lead in the fog: the
free is an **allocation-volume** cost (~5.3 GB peak working set, t79), not a
drop-strategy cost; only fewer/larger allocations (arena or bulk-free layer
collection) can move it. Evidence:
[`evidence/t94-remaining-drop/`](evidence/t94-remaining-drop/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only, 8/8 negative controls rejected).
Frontier now: **no open child remains**; the fog and the next route are the
human's. Standing paired acceptance and KEEP/DROP rules unchanged.

**Previous selection scope (2026-10-28 → 2026-10-29, on the ticket 93 take):**
the human said "continue the perf vs orca map" (no ticket named); the frontier
re-derivation found [93](issues/93-whole-layer-deep-infill-offsets.md) as the
only open child, and the session took it. It is **resolved (2026-10-29) as
measured GATE NOT MET — human-approved DROP, candidate reverted**, with the
re-appliable `candidate.patch` preserved. Phase 1 closed three of four shapes:
lazy dilation **0 of 130** calls skippable; cheaper predicate rejected (no
Miter-join equivalence); duplicate recompute **9** calls, **0** identical
operands, absolute ceiling **0.28 %**. The survivor, **windowed erosion**,
measured **9,244.1 / 9,311.3 ms** accumulated saving = **1,006.9 / 1,045.6 ms
wall-eq = 1.38 / 1.43 %** of the cell against the **0.585 s** floor, the two
runs agreeing within **3.70 %**. The candidate (erosion operand clipped to the
pre-retain qualified bbox grown by `14 × spacing`) passed **16/16** proof cells
byte-identical, **8/8** ordinary cells equal to the post-t81 references, **8/8**
cross-mode equal, 24 identical modules per mode. The paired campaign passed the
wall clause in both ordinary cells (**−3.31 % / −0.26 %** wall, **−1.19 % /
−0.48 %** CPU) and missed it in both accelerated cells (**+0.13 % / +0.62 %**
wall, **−0.81 % / −0.45 %** CPU), so the pre-declared four-cell wall clause is
**NOT MET** and the human dropped the candidate. The CPU fall matches the probe
(9.7 s vs 9.2 s) and the wall transfer is lifted by cpu/wall 9.65 → 9.84. One
campaign interruption is disclosed (server restart during ordinary classic-on;
that cell re-run whole, partial preserved); the campaign timed only the four
pre-declared base classic cells, so benchy/arachne gains are unmeasured.
Frontier now: **no open child remains**; the fog and the next route are the
human's. Standing paired acceptance and KEEP/DROP rules unchanged. Evidence:
[`evidence/t93-deep-infill-offsets/`](evidence/t93-deep-infill-offsets/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only, 18/18 negative controls
rejected).

**Previous selection scope (2026-10-18, on the t73 take):** the frontier was
re-derived on resume — the only open child was
[consume-only-when-read annotations](issues/20-consume-only-when-read-annotations.md),
parked below-fold — and the human selected a **Base-cell bottleneck refresh**
from the route options. [73](issues/73-base-current-bottleneck-refresh.md) was
graduated, claimed and **resolved (2026-10-18) as a measurement-only take**:
eight fresh ordinary base captures (four fuel profiles + four
instrumentation-only corroborations) at HEAD `86c15182` (slicer sources = the
t70 keep `7127b4d6`), every output byte-identical to its t58 qualification
reference, guest freshness exit 0, 24/24 external modules. The base cells are
**prepass-dominated** — the common floor is 104–115 s on every cell and the
supports-on cells add 134–149 s of support stages — while the per-layer phase
is 94–117 s on classic and 16–23 s on Arachne. Three precise tickets were
graduated in recommendation order: [LayerFinalization supports-on whole-print
view cost](issues/74-layerfinalization-supports-on-view-cost.md) (four module
brackets of 45.0–48.5 s against a 0.30–1.39 s supports-off control, with
0.042 % of the slice's guest fuel), [ShellClassification phase-B serial commit
reduction](issues/75-shellclassification-phase-b-commit.md) (26.0–31.4 s on
every base cell; t27's serial commit is still 25–30 s), and [MeshAnalysis exact
footprint union reduction](issues/76-meshanalysis-footprint-union.md)
(19.4–22.2 s serial over 326,546 facets; bbox replacement unsafe, exact
reduction only). Ticket 20 received the fresh base context (per-layer
94.3/98.2 s; classic-perimeters 98.8/95.0 % of fuel; t18's 57.9 % base scan
share) with the below-fold park **unchanged** — the t72 best cell is still
3.30x. No candidate, timing campaign, production edit or production-source commit. Evidence:
[`evidence/perf-refresh-base-2026-10-18/`](evidence/perf-refresh-base-2026-10-18/FINDINGS.md)
(`verify.py` exits 0 checkout-only with four negative controls rejected;
`analyze.py` reproduces the tables). Frontier now:
[20](issues/20-consume-only-when-read-annotations.md) parked below-fold and the
three new tickets unclaimed; the take order is the human's. Standing paired
acceptance and KEEP/DROP rules unchanged.

**Previous selection scope (2026-10-18 → 2026-10-19, on the ticket 74 take):**
the human's "continue" named no ticket, so the session took the first frontier
ticket in order, [74](issues/74-layerfinalization-supports-on-view-cost.md);
it is now **resolved (2026-10-19, human-approved KEEP, candidate committed
`091e6bc9`)**. Attribution: **96.1 % of the 61,555 ms supports-on module sum is
whole-print copy/free churn** (input snapshot clone 12,112 ms, store drop
16,653 ms, merge rollback clone 13,446 ms, old-vector drop 16,940 ms; guest call
1,816 ms = 2.9 %), owned by the full `LayerCollectionIR` clone/free and its
IR-only `support_entity_identities` side table (23,751 rows supports-on vs 0
supports-off), not by module reads or the 1.96x output-byte ratio. The candidate
(Arc-shared input snapshot + no-op/append-only merge fast paths) measured
**61,555 → 2,775 ms (−95.5 %)** with supports-off unchanged (1,290 → 1,252 ms)
and outputs byte-identical to the t58 references. **Proof gate 16/16 PASS**
(baseline and candidate byte-identical, clean, non-vacuous in every cell;
`qualification.json`), with snapshots built from `67ef742b` and provenance
recorded. The paired ordinary + accelerated A/B ran on a quiet machine on
2026-10-19 and **PASSED**: 6/6 pairs in every cell, paired wall deltas −3.13 % /
−12.71 % / −3.89 % / −14.23 % and corroborating CPU deltas −0.91 % / −2.30 % /
−0.84 % / −2.97 % (ordinary/accelerated × benchy/base). Base supports-on wall
falls **46.0 s ordinary** and **47.5 s accelerated**, inside t73's 45.0–48.5 s
bracket attribution, and every pair's output is byte-identical. Frontier now:
[75](issues/75-shellclassification-phase-b-commit.md) and
[76](issues/76-meshanalysis-footprint-union.md) unclaimed;
[20](issues/20-consume-only-when-read-annotations.md) parked below-fold; the take
order is the human's. The probe take's machine drift (baseline wall 571.9 s vs
t73's 408.7 s) is why its brackets are an internal split, not wall evidence.
Evidence:
[`evidence/t74-finalization-view/`](evidence/t74-finalization-view/FINDINGS.md)
(`verify.py` exits 0 checkout-only, three negative controls rejected).

**Current selection scope (2026-10-28, on the ticket 92 take):** the human's
"continue the perf vs orca map" named no ticket; the frontier re-derivation at
claim time found [92](issues/92-per-surface-bridge-boolean-locality.md) as the
only open child, and the session took it. Ticket 92 is **resolved
(2026-10-28) as a Phase-1 screen — measured-dead, no candidate, no production
edit, probe reverted.** One ordinary base classic supports-off slice, two probe
runs, outputs byte-identical to the post-t81 reference `292df501…` and clean.
The ticket's three Phase-2 shapes (per-surface bbox gate, hoistable `1.3 ×`
anchor offset, many-piece tiling) have an **absolute upper-bound ceiling of
0.341–0.371 % of the cell** against the **0.80 %** floor (2,429.6 / 2,303.2 ms
accumulated = 271.1 / 249.7 ms wall-equivalent); deleting the whole
`internal_bridge_angles` step as well still reaches only 0.40–0.44 %. Shape
counts refute tiling outright — 130 calls carry **196 surfaces** (mean 1.51,
**max 7**) against Orca's 128-piece threshold — and the per-surface
`offset`/`intersection`/qualification is **0.25 s accumulated** against
`internal_bridge_angles`' 2.58 s (t75's 1.5 s clip bracket does not transfer;
its 2.4 s angles bracket does).

The screen found a **different, larger term of a different shape**: the same
function's two **whole-layer offsets on the unioned deep sparse-infill area** —
`offset(deep_infill_area, +1.5 × spacing)` **12.5 / 11.4 s accumulated** over
the 130 calls (median 81.6 ms, max 222.4 ms) and
`offset(deep_infill_area, −4.5 × spacing)` **10.0 / 9.3 s** over the 14
qualified calls (max 428.3 ms) — together **2.24–2.51 s wall-equivalent =
3.06–3.43 % of the cell**, ~4x the floor and ~9x t92's own ceiling. Both are
whole-layer operands (the clip area is also the angles operand; the erosion is
the `retain` predicate), so consumer bbox-gating cannot remove them; they are
out of t92's question rather than a reason to keep it open. Graduated
[ticket 93](issues/93-whole-layer-deep-infill-offsets.md) to price the lazy
dilation, windowed erosion, cheaper-predicate and duplicate-recompute shapes
with its own Phase-1 screen. Disclosures: instrumented wall 79.0–83.0 s against
the uninstrumented 73.17 s cell (probe + `--profile` overhead, never a wall
claim), ~7–9 % inter-run drift on the bucket with the verdict unchanged.
Evidence:
[`evidence/t92-per-surface-bridge/`](evidence/t92-per-surface-bridge/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only, 15/15 negative controls
rejected). Frontier now: [93](issues/93-whole-layer-deep-infill-offsets.md)
only, open and unclaimed; the take order is the human's. Standing paired
acceptance and KEEP/DROP rules unchanged.

**Previous selection scope (2026-10-27, on the ticket 91 take):** the human's
"continue the perf vs orca map" named no ticket; the frontier re-derivation at
claim time found [91](issues/91-exact-z-object-geometry-memo.md) and
[92](issues/92-per-surface-bridge-boolean-locality.md) open, and the session took
the first in order. Ticket 91 is **resolved (2026-10-27, human-approved KEEP,
candidate committed `f10cef5a`)** as a measured **GATE NOT MET on one cell's
strict CPU clause** plus a wall pass in all four: wall
**−5.52 / −3.84 / −2.68 / −4.95 %** (ordinary/accelerated × classic/arachne)
with process CPU **−1.22 / −0.66 / +0.08 / −0.32 %**, **24/24 pairs
byte-identical**, one SHA per arm, zero starvation exclusions, 16/16 proof cells
byte-identical and 8/8 ordinary cells equal to the derived post-t81 references.
The **+0.08 pp** is `accelerated-base-classic-on` — 0.8 s of ~1,053 s process
CPU, inside that cell's own repeat spread — and it is why the ticket's
pre-declared `CPU ≤ baseline` clause is NOT MET; the human accepted the
sub-noise miss and kept the candidate. One memo pass: the exact-Z service builds
the world mesh, the plate and the constant top-Z cross-section **once per
object** instead of on every miss, leaving the per-`(object, region, z)` cache
contract unchanged. Direct probe (attribution only; both outputs equal the
reference SHA): **431 queries, 431 misses, 431 geometry builds → 1**,
`ExactZQueryService::query` **22,636.3 → 8,922.3 ms (−60.6 %)** and the
`validate_entry` loop **24,335.0 → 10,004.3 ms**; the remaining 8.9 s is the
requested-Z cross-section. The `post_planner_gap_ms` bracket falls in all four
cells (14.7→9.9 / 18.4→7.4 / 21.8→9.9 / 13.8→12.1 s). The memo retains one
world-space mesh per object — **42.25 MiB** measured for `tmp/base.stl`
(1,230,602 vertices + 7,383,702 indices) — recorded, not gated. Both freshness
gates exit 0, `verify.py --self-test` exits 0 checkout-only with 13/13 negative
controls rejected. Frontier now:
[92](issues/92-per-surface-bridge-boolean-locality.md) only, open and unclaimed;
the take order is the human's. Standing paired acceptance and KEEP/DROP rules
unchanged. Evidence:
[`evidence/t91-exact-z-memo/`](evidence/t91-exact-z-memo/FINDINGS.md).

**Current selection scope (2026-10-27, on the ticket 90 take):** the human's
"continue the perf vs orca map" named no ticket; the frontier re-derivation at
claim time found [90](issues/90-postpass-layer-collection-ownership.md) and
[91](issues/91-exact-z-object-geometry-memo.md) open, and the session took the
first in order. Ticket 90 is **resolved (2026-10-27, human-approved KEEP,
candidate committed `7b22fccc`)** — the standing paired gate **PASSED in all
four pre-declared base supports-on cells**: wall **−5.54 / −7.56 / −6.54 /
−7.64 %** (ordinary/accelerated × classic/arachne) with process CPU
**−1.13 … −1.67 %** (flat or lower, not merely inside the +3.0 % ceiling),
**24/24 pairs byte-identical**, one SHA per arm, zero starvation exclusions,
every pair moving the right way (−1.56 … −13.20 %). One ownership pass: the
finalization stage takes the caller's `Vec<LayerCollectionIR>`
(`execute_layer_finalization_owned`) instead of cloning a rollback snapshot,
`GCodeEmitter::emit_gcode_owned` lets the emitter rotate and walk the caller's
own vector, and the postpass hands it over, so the collection is dropped once
instead of cloned and dropped twice. Attribution (instrumented, attribution
only): the **1,765 / 1,684 ms clone**, the **3,685 / 3,758 ms commit drop** and
the **4,490 / 3,723 ms postpass teardown drop** are gone, the one remaining drop
lives inside the `host:gcode_emit` bracket and costs **1.56 / 2.68 s less** than
the old clone+drop pair, and the postpass phase falls **66.5 / 70.3 %** — which
is why the measured wall deltas (**12.0 / 14.8 / 15.9 / 15.4 s**) beat the
ticket's pre-declared net ceiling of **10.76 / 10.29 s**. Exactness: 16/16 proof
cells byte-identical and equal to the derived post-t81 references, ordinary ≡
accelerated **8/8**, both freshness gates exit 0, host-only candidate proven by
`provenance.json` (byte-identical 24-module trees per mode, distinct binaries
per arm). Both the abort-state change (a fatal finalization error now consumes
the working collection) and the pre-existing post-t81 `resolution`-fixture reds
(23 contract — measured identical with and without the candidate — plus 3
executor and 50 integration diagnosed by signature) are disclosed. Frontier now:
[91](issues/91-exact-z-object-geometry-memo.md) and the concurrent screening
take's [92](issues/92-per-surface-bridge-boolean-locality.md), both open and
unclaimed; the take order is the human's. Standing paired acceptance and
KEEP/DROP rules unchanged. Evidence:
[`evidence/t90-postpass-ownership/`](evidence/t90-postpass-ownership/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only, 11/11 negative controls
rejected).

**Previous selection scope (2026-10-27, PR screening take):** at the human's
request, three merged OrcaSlicer performance PRs were screened against this
map's measured shapes. [#16741 "Speed up slicing of finely detailed
models"](https://github.com/OrcaSlicer/OrcaSlicer/pull/16741) has no PnP
counterpart for either half: the emitter has no per-extrusion object-mass
locator, and the dense-below bridge shortcut fires only when every fill surface
of the lower layer prints at 100 % density, which the matched job's 20 % sparse
infill never reaches. [#16733 "Speed up slicing of objects with many
parts"](https://github.com/OrcaSlicer/OrcaSlicer/pull/16733) has no per-part
contact test to accelerate: the loader merges normal 3MF parts into one mesh and
`SkirtBrim::generate_brim_entities` builds one print-wide rectangular loop set.
[#15985 "Faster slicing of layers cut into many
pieces"](https://github.com/OrcaSlicer/OrcaSlicer/pull/15985) maps only its
neighbourhood-restriction / fill-boundary-hoist mechanisms, and only to the
per-surface bridge qualification: the t75-named clip+angles terms are 3.9 s
accumulated ≈ 0.55 % of the current base classic-off cell, below the 0.80 %
floor before the unmeasured per-surface count. One screening ticket graduated:
[92](issues/92-per-surface-bridge-boolean-locality.md) — Phase 1 measures the
per-surface share and closes measured-dead unless it clears the floor; Phase 2
is conditional. Frontier now:
[90](issues/90-postpass-layer-collection-ownership.md),
[91](issues/91-exact-z-object-geometry-memo.md) and
[92](issues/92-per-surface-bridge-boolean-locality.md); the take order is the
human's. Standing paired acceptance and KEEP/DROP rules unchanged.

**Previous selection scope (2026-10-27, on the ticket 89 take):** the human's
"continue the perf vs orca map" named no ticket; the frontier re-derivation
found [89](issues/89-support-plan-exact-z-validation-parallel.md) and
[90](issues/90-postpass-layer-collection-ownership.md) open, and the session
took the first in order. Ticket 89 is **resolved (2026-10-27) as a measured
GATE NOT MET — human-approved DROP, candidate reverted**. Exactness passed
in full: 16/16 cells byte-identical between arms, 8/8 ordinary ≡ accelerated,
ordinary cells equal to the derived post-t81 references, contract suite
160/160 (including a new 64-entry order test that derives its expectation three
independent ways). The standing four-cell campaign was **not run by the stop
discipline**: a 3-pair pre-gate on ordinary base classic-on at 2 % load
measured wall **−5.50 %** against process CPU **+6.78 %**, more than twice the
pre-declared **+3.0 %** work-preserving ceiling, every pair byte-identical. The
probe shows the CPU is contention, not extra work — 431 entries = 431 distinct
`(object, z)` = 431 queries = 431 misses — and splits the tail into
requested-Z cross-section **50.96 %**, constant top-Z cross-section **43.39 %**,
world-mesh build **5.65 %** of 92.73 s accumulated thread-time for a 7.99 s
wall, against t88's **15.24 s serial** tail: ≈6x thread-time for the same
queries at 12-way concurrency (cross-run), so the kernel is
memory/allocator-bound. The ticket's
optional `(object, z)` cache is **measured worthless here** (one entry per
anchor Z) and was not implemented. One candidate graduated:
[91](issues/91-exact-z-object-geometry-memo.md) — memoize the z-independent
terms, **49.04 %** of the tail and ≈**7.5 s / 6.8 s** ceiling (**4.1 % / 3.6 %**
of the cell), which removes work instead of spreading it and should lower CPU.
Two pre-existing reds in `slice_region_view_contract_tdd` were repaired to make
the contract suite runnable (t81's mandatory `resolution` key; t20's
consumer-live stamp gate); neither touches the candidate. Evidence:
[`evidence/t89-exact-z-parallel/`](evidence/t89-exact-z-parallel/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only, 8/8 negative controls
rejected). Frontier now:
[90](issues/90-postpass-layer-collection-ownership.md) and
[91](issues/91-exact-z-object-geometry-memo.md), open and unclaimed; the take
order is the human's. Standing paired acceptance and KEEP/DROP rules unchanged.

**Previous selection scope (2026-10-25, on the ticket 88 take):** the human's
"continue the perf vs orca map" named no ticket; the frontier re-derivation
found [88](issues/88-supports-on-increment-attribution.md) as the only open
child, and the session took it in order. It is **resolved (2026-10-25) as a
measurement-only attribution — no candidate, no production edit, no keep/drop
campaign.** Four quiet (5 % load) instrumented base captures at HEAD
`5c7fa03e`, all byte-identical to the post-t81 references, plus a
corroborating loaded pass (68–73 % desktop load) and a config-only
`tree_support_branch_distance = 5.0` arm. The supports-on increment is
**106.8 s classic / 109.7 s arachne** and is **~69 % the support geometry
itself** (planner 55.5 s = 52.0 % of the classic increment; canonical contact
detection 10.2 s; per-layer render 5.9 s wall) plus **~31 % host-side**: a
**serial 15.2 / 13.9 s exact-Z validation loop** (99.3 % of the post-planner
gap, every entry independent) and **one finalization clone + two whole-collection
drops** (finalization 6.8 s, emission 6.3 s, teardown 4.6 s; the probe's
`SystemTime` marks show the emitter's clone is ~5 ms and the 6 s is the drop).
Two candidates graduate on the map's 0.80 % floor:
[89](issues/89-support-plan-exact-z-validation-parallel.md) — parallelize the
validation, **8.28 % / 7.42 %** of the cell — and
[90](issues/90-postpass-layer-collection-ownership.md) — one ownership path
through finalization/emission, **5.84 % / 5.47 % net** (9.18 % / 8.60 % upper).
Measured dead/geometry-bound: the SupportAnalysis eligibility fold (2.24 s
gross, no above-floor removable term), the contact kernel (10.2 s, parallel canonical), the
planner (55.5 s, support geometry, t78 batching in place) and the per-layer
render (5.9 s wall, already parallel). The **`tree_support_branch_distance`
5 mm arm is −7.25 s (−3.94 %) and ~6.8 s of the increment (6.4 %)** — a modest
driver, not the reason; the parity ruling stays with the human. The probe also
caught a measurement trap: an `Instant` clock read disagreed with the event
timestamps, so a 5 ms clone read as 4.09 s; all probe claims use the
`SystemTime` marks and the stage brackets. The probe is reverted; both
freshness gates exit 0. Evidence:
[`evidence/t88-supports-increment/`](evidence/t88-supports-increment/FINDINGS.md)
(`verify.py` exit 0 checkout-only, `--self-test` 8/8 negative controls
rejected). Frontier now: **tickets 89 and 90, open and unclaimed**; the take
order is the human's. Standing paired acceptance and KEEP/DROP rules unchanged.

**Previous selection scope (2026-10-25, on the ticket 87 take):** the human's
"continue the perf vs orca map" named no ticket; the frontier re-derivation
found **86/86 children resolved**, and the human selected the **Scoreboard
refresh** route from the options presented.
[87](issues/87-scoreboard-refresh-post-resolution-port.md) is **resolved
(2026-10-25) as a measurement-only take — no candidate, no production edit.**
The first fully resolution-matched scoreboard, measured at HEAD `6e71c1a1`
with both standing trees rebuilt from clean HEAD: **Orca still wins all 8
cells at 2.71x–9.36x**, CPU corroborating in every cell (PnP burns
4.1x–21.3x Orca's CPU). Best cell **2.71x** (benchy classic-off accelerated;
2.75x ordinary; 2.74x benchy arachne-off), worst **9.36x** (base arachne-on
ordinary). **No cell within ~1.5x — the below-fold park is still binding**,
though the release bound has moved **3.30x → 2.71x**. PnP ordinary wall fell
**39.7–66.2 %** in all eight cells since 2026-10-18 (t74/t75/t76/t20/t78/t81/t85
+ t65 on Arachne), while **Orca also measured 16.2–35.3 % faster today**
(machine drift, disclosed; ratios are today-vs-today). Output: PnP 16/16
`degraded=false`/`non_fatal=0`, ordinary ≡ accelerated **byte for byte in all
8 cells**, one SHA per group, new post-t81 reference set recorded; the base
SHAs reproduce the t83/t84/t85 references. 96 rows / 72 measured, zero
starvation exclusions, zero validation failures, both freshness gates exit 0.
Disclosed: an accidental duplicate benchy round (preserved as
`rows-benchy-duplicate-round.csv`, not ledger input; the two rounds agree
within ~1 %, and the clean ledger re-run is 4–6 % slower on PnP, so **benchy
uncertainty is larger than base's**). The headline finding — every supports-on
cell is the worse of its pair, and the base increment is **+97.90 / +113.38 s**
for PnP against **+4.50 / +5.79 s** for Orca while PnP emits *fewer* support
sections — graduated
[88](issues/88-supports-on-increment-attribution.md) (open, unclaimed).
Evidence: [`evidence/scoreboard-refresh-20261025/`](evidence/scoreboard-refresh-20261025/SCOREBOARD.md)
(`verify.py` exit 0 checkout-only, `--self-test` 8/8 negative controls
detected). Frontier now: ticket 88 only; the take order is the human's.
Standing paired acceptance and KEEP/DROP rules unchanged.

**Previous selection scope (2026-10-25, on the ticket 85 take):** the human's
"continue the perf vs orca map" named no ticket, so the session took the only
frontier ticket, [85](issues/85-seam-inset-offset-batching.md) — and it is
**resolved (2026-10-25, human-approved KEEP)**. The candidate batches the seam
planner's 495 per-region inset offsets into **one** `offset_polygons_batch`
(two guest passes, unchanged per-request arguments, unchanged entry order and
fallback path), and the standing gate **PASSED in all four pre-declared base
classic cells**: wall **−7.46 % / −2.72 %** (ordinary) and
**−7.03 % / −3.86 %** (accelerated), process CPU **+0.34 … +0.47 %** (ceiling
+3.0 %), **24/24 pairs byte-identical**, one SHA per arm, **zero starvation
exclusions**, zero pair moving the wrong way (individual wall ratios
0.887–0.977). Output identity is **16/16** against the derived post-t81
reference set, with cross-mode equality **8/8**.

Attribution: the seam stage falls **8,356 → 1,737 ms** (off) and
**8,288 → 1,703 ms** (on) in instrumented captures — **≈7x batch occupancy** on
the offset request set (conservative bound 4.4x), against this ticket's
pre-declared **1.15x** DROP floor. **t78's 2.6x pricing was too pessimistic**,
which is why ticket 84's predicted 5.43 % (off) is beaten by the measured
7.46 %; the shape is now 2.3 % of a base cell, so the seam stage has left the
base common floor. The parity case is exact by construction (no geometry
change) and pinned by
`batched_inset_offsets_match_the_serial_reference_per_region`
(`modules/core-modules/seam-planner-default/src/lib.rs`), which compares the
batched path entry-for-entry with the pre-batching serial loop on a fixture
whose two regions carry different insets.

**One infra prerequisite was repaired inside this take:**
[86](issues/86-accelerated-shim-workspace-root.md) — a cold accelerated build
died in `proc-macro2`'s build script because the rustc shim resolved the
workspace root from CWD (t71's runtime-walk contract, wrong for the one caller
whose CWD is a cargo-registry package). Fixed by exporting
`PNP_XTASK_WORKSPACE_ROOT` from the three accelerated entry points and
re-validating it in `resolve_shim_root` (`xtask/src/rustc_driver.rs`;
`cargo test -p xtask` 187/0). No candidate behaviour depends on it; without it
the accelerated arm of this gate could not be measured.

Disclosures: the two arms' `pnp_cli.exe` differ (the seam planner is linked into
the host via `slicer-integrated-modules`; both arms run
`--no-integrated-modules`, so it is inert), guest fuel rises +3.1 %, and the
pre-campaign sanity pair taken under desktop load is not used anywhere.
Evidence: [`evidence/t85-seam-inset-batching/`](evidence/t85-seam-inset-batching/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only, 8/8 negative controls rejected).
Frontier now: **no open child remains**; the fog and the next route are the
human's. Standing paired acceptance and KEEP/DROP rules unchanged.

**Previous selection scope (2026-10-24, on the ticket 84 take):** the human's
"continue the perf vs orca map" named no ticket; the frontier re-derivation
found **no open child** (83/83 resolved) and the human selected the **Base
common-floor attribution** route.
[84](issues/84-base-common-floor-attribution.md) is **resolved as an
attribution take — one candidate graduated, two shapes measured dead; no
production edit, probe reverted.** The seam stage's wall is **92.4 / 93.2 %** one
serial native `offset_polygons` per region (495 calls, 8,195 / 8,177 ms of a
9.14 / 8.97 s stage = **8.83 / 3.92 %** of the 92.8 / 208.6 s cell walls); the
candidate list is 0.29 / 0.13 % and host projection + harvest are under 1 %. The
overhang stage is **engine-bound**: 99.7 % is the parallel `annotate_overhangs`
kernel and 85.4 % of its work is the canonical three round offsets per band
layer (1,455 calls) at **11.86x** measured parallelism; the only operand-side
shape (bbox window) retains **97.84 %** of the previous points, a
**0.189 / 0.080 %** ceiling. Graduated
[85](issues/85-seam-inset-offset-batching.md) (open, unclaimed): batch the inset
offsets through the existing `offset_polygons_batch`, ceiling **8.83 / 3.92 %**
upper and **5.43 / 2.41 %** at t78's measured 2.6x. Frontier now: ticket 85
only; t83's `tree_support_branch_distance` observation stays fog. Evidence:
[`evidence/t84-common-floor/`](evidence/t84-common-floor/FINDINGS.md)
(`verify.py --self-test` exits 0, 8/8 negative controls rejected).

**Previous selection scope (2026-10-23 → 2026-10-24, on the ticket 83 take):** the human's
"continue the perf vs orca map" named no ticket; the frontier re-derivation
found [83](issues/83-arachne-wall-simplification-tolerance-pricing.md) as the
only open child and the session took it. It is **resolved (2026-10-24) as a
measurement-only pricing take — DROP, below the map's pre-declared floor; no
candidate, no parity port, no production edit.** One binary, one module tree,
two config-only arms (default 0.05/0.005 mm vs canonical 0.5/0.025 mm), 2
alternating instrumented pairs per Arachne cell plus 6 uninstrumented pairs on
the two borderline cells. The tolerance is **real work**: arachne guest fuel
falls **−10.76 %** (benchy) / **−16.21 %** (base), whole-job fuel −3.41/−1.41 %
and −11.56/−3.08 %, G-code bytes −3.84…−8.23 % with identical TYPE counts, all
runs clean and byte-deterministic per arm (the default arm reproduces t81's
base-arachne-off reference `01a1434c…`). But the module is only **2.9–6.9 % of
cell wall** at **5.1–10.7x** phase parallelism, so the occupancy-normalized
ceiling is **0.039–0.376 %** of wall against the pre-declared **0.80 %** floor;
the generous fuel-proportional bound reaches only 0.844 % (base-off, whose
measured elapsed moved +0.8 % the wrong way). The direct wall check agrees:
base-off **+0.17 % wall / +0.09 % CPU**, benchy-on **+3.18 % wall / +0.17 %
CPU** (paired medians; benchy-on's instrumented −2.9 % was load drift). `D-168`
is retained with this measured magnitude. An observation, not a ticket:
`tree_support_branch_distance` (Orca 5 mm vs PnP 1 mm, supports-on) is the other
t82 "PnP does more work" geometry key, unmeasured and parity-scoped. Frontier
now: **no open child remains**; the fog and the next route are the human's.
Standing paired acceptance and KEEP/DROP rules unchanged. Evidence:
[`evidence/t83-arachne-tolerance-price/`](evidence/t83-arachne-tolerance-price/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only, 5/5 negative controls rejected).

**Previous selection scope (2026-10-23, on the ticket 82 take):** the human's
"continue the perf vs orca map" named no ticket; the frontier re-derivation
found **no open child** (81/81 resolved) and the human selected the
**Fairness-key parity audit** from the route options.
[82](issues/82-fairness-key-parity-audit.md) was graduated, claimed and
**resolved (2026-10-23) as a measurement-only take — no candidate, no timing,
no production edit**. Eight Orca runs produced a **618-key resolved-config
appendix per cell** (identical key set in all eight; only `wall_generator`,
`enable_support` and `print_settings_id` vary by cell) and every key was
classified: 227 in pre-emission geometry scope, 14 `geometry-differs`, 18
`absent-material`, 112 equal (value / unit / rename / canonical clamp), 40
emission-only, 102 absent-and-inert, 332 absent-and-not-geometry.

**Verdict: no third `resolution`-class lever at scale.** The only unmatched
geometry pair where PnP does materially more work than the matched Orca job is
the Arachne wall-simplification tolerance pair — `wall_maximum_resolution`
**0.5 mm vs 0.05 mm** and `wall_maximum_deviation` **0.025 mm vs 0.005 mm**
(PnP 10x / 5x tighter), Arachne cells only, **magnitude unmeasured**. It is
already logged as `D-168-ARACHNE-SIMPLIFY-FALLBACKS-TIGHTER-THAN-CANONICAL`
(re-derive the row from `docs/DEVIATION_LOG.md`) but has never been priced
against wall, and **the generated deviations table cannot see it**: that table
compares against `docs/ORCA_CONFIG_REFERENCE.md`, dated 2026-05-19, and **82 of
the 618 resolved keys postdate the snapshot** — the same blind spot that made
`resolution` a lucky catch rather than a systematic one. Everything else runs
in the other direction or is inert: Orca resolves `top_shell_layers` 4 vs PnP 3,
`support_interface_top_layers` 3 vs 2, `precise_outer_wall` 1 vs 0, and ports
`ensure_vertical_shell_thickness` / `detect_narrow_internal_solid_infill` /
tree-support brim that PnP lacks, so the measured ratios **understate** PnP's
per-unit-work deficit where they differ at all.

**Controls matched in all eight cells:** `resolution = 0.01` (ticket 81's port
is visible in the appendix) and `slice_closing_radius = 0.049` (same key, same
default, applied before the resolution simplify — same order as canonical).
The `slice_closing_radius` half of the fog entry is therefore retired;
`enable_arc_fitting` remains unexpressible and inert (`= 0` on the matched
job). **Observations recorded, not defects proven:** `classic-perimeters.toml`
declares `detect_thin_wall = true` while `arachne-perimeters.toml` declares
`false` and the emitted block records `0` (two modules, one key, two defaults —
matched with Orca by accident); `support_interface_bottom_layers` is declared
`-1` but emits `2`; `brim_width` (8 mm vs 0) is unresolved at the output level
because both captured cells emit a `Brim` role; and Orca's overhang speed bands
are live (50/30/10) while PnP's matched config leaves all four at 0, so t20's
consume-only gate is valid for PnP's job but the two engines are not doing the
same work there. Evidence:
[`evidence/t82-fairness-key-audit/`](evidence/t82-fairness-key-audit/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only, 9/9 negative controls rejected;
`key-table.csv` carries all 618 rows). Frontier now: the pricing take is
graduated as
[83](issues/83-arachne-wall-simplification-tolerance-pricing.md) (open,
unclaimed) — price the Arachne tolerance match on the four Arachne cells with
its own pre-declared gate — and the fog carries the snapshot-staleness method
note. The take order is the human's. Standing paired acceptance and KEEP/DROP
rules unchanged.

**Previous selection scope (2026-10-23, on the ticket 81 take):** the human's
"continue the perf vs orca map" named no ticket, so the session took the only
frontier ticket, [81](issues/81-resolution-slicing-time-contour-simplification.md),
and **resolved it (2026-10-23, human-approved KEEP)**. Nine decisions were put to
the human first; the one that mattered was the scope split, and they ruled that
**the map carries this port end to end** rather than handing the parity work to a
spec packet — an explicit carve-out from the Out-of-scope bullet below, recorded
here because it changes what this map is allowed to do.

The port: `resolution` (default 0.01 mm, `normalize_fdm` floor 0.001 mm) applied
at four canonical call sites — the pre-wall per-surface simplify in both
perimeter modules (region granularity; PnP has no `Surface` model), the gap
`douglas_peucker` before `medial_axis`, the classic innermost-contour simplify
before the infill boundary, and the arachne infill contour. Canonical's fifth
site (classic inner-part pass) has no PnP counterpart and is recorded, not
invented. The `0.2 ×` arc-fitting branch is unexpressible (no arc-fitting key)
and inert on the matched job.

**Parity**: the DP is canonical-exact (closed-ring seeding, `<=` acceptance,
strictly-greater tie-break) and the invariant suite pins its actual guarantee;
the union step runs on `clipper2-rust` where canonical uses legacy Clipper 6.4.2
with `StrictlySimple`, so set semantics match and representation does not —
disclosed, not claimed. **Confinement 20/20**: `Layer::Slice` typed IR identical
in all eight cells, `Layer::Perimeters` differs in all eight, support silhouette
byte-identical in the four supports-on cells. **Campaign (gate met)**: classic
cells **−14.31 / −9.00 %** (benchy) and **−22.24 / −10.87 %** (base) wall with CPU
−18…−27 %; Arachne cells wall-neutral (−0.07…−1.87 %); `base arachne-off` failed
its first six pairs at +2.30 %, stopped the campaign by rule, and re-ran at
−2.77 % (combined n=12 **−1.07 %**, spread ±9) — that cell is wall-neutral within
measurement resolution, and the spread is the finding. CPU fell in every cell.
Every cell is byte-deterministic across its runs, and the baseline arm
reproduces the standing t58 reference `34886ccb…`.

Because the port changes geometry, the byte-identity proof regime is replaced by
invariants + stage confinement + determinism, with per-cell candidate SHAs
recorded as the new reference set ([evidence](evidence/t81-resolution-port/FINDINGS.md),
`campaign/results.json`). Frontier now: **no open child remains**; the fog's
`slice_closing_radius` / `enable_arc_fitting` question and the un-graduated leads
are the human's to route. Standing paired acceptance and KEEP/DROP rules
unchanged.

**Previous selection scope (2026-10-22, on the ticket 80 take):** the frontier
re-derivation found no open child (79/79 resolved), and the human's route answer
challenged the map's own fairness contract instead of choosing a route — "has this
study been running on the same resolution as Orca does? The resolution defaults
for PNP are higher". [80](issues/80-resolution-fairness-premise.md) was
graduated, claimed and **resolved as a measurement-only premise audit**: the job
has **not** been resolution-matched. Orca's matched profile inherits
`resolution = 0.01 mm` and applies it as a **slicing-time contour
simplification before walls and infill are generated**, while PnP implements only
emission-time per-role D-P; re-running the same profile at Orca's canonical
0.001 mm floor costs Orca **+9.5 % to +65.2 % wall** across all eight cells
(median **+40.2 %**; CPU **+21 % to +92 %**; G-code **+9 % to +91 %**), 3
alternating appendix-verified pairs per cell. A first attempt at a `resolution =
0` arm was rejected by the appendix gate, which is how the canonical **0.001 mm
clamp** was found — "disabled" is not expressible in OrcaSlicer. The disclosure
is now in the fairness contract above; the implementation candidate is graduated
as [81](issues/81-resolution-slicing-time-contour-simplification.md) (open,
unclaimed) and is explicitly **canonical parity work, not output-preserving**,
so its proof gate cannot be byte-identity against the t58 references. The
feature-gap tracker's `03-asset-scoped-gap.md` adjudication (per-domain emission
keys as `resolution`'s counterpart) covers only the emission half and needs
re-deriving there. No production edit, no PnP build, no candidate, no scoreboard
revision. Evidence:
[`evidence/t80-resolution-fairness/`](evidence/t80-resolution-fairness/FINDINGS.md)
(`verify.py` exits 0 checkout-only with 5/5 negative controls rejected).
Frontier now: **ticket 81 only**; the take order is the human's. Standing paired
acceptance and KEEP/DROP rules unchanged.

**Previous selection scope (2026-10-22, on the ticket 79 take):** the frontier
re-derivation found **no open child** (78 children, all resolved) and returned
the route to the human, who selected the **t78 constant sweep**.
[79](issues/79-tree-support-planner-emit-batch-constant-sweep.md) was
graduated, claimed and **resolved (2026-10-22) as a measurement-only take —
DROP, no constant changed**. The chunk sweep found the committed
`EMIT_BATCH_LAYERS = 4` at the knee (module bracket 92,875 / 75,503 / **61,519**
/ 65,492 / 62,574 / 76,813 ms at 1 / 2 / 4 / 8 / 16 / 32); the cap sweep chose
`CLIP_BATCH_MAX_REQUESTS = 4096` on a 5.5 % bracket edge over `256` after a
load-confounded pass, but the frozen-snapshot pre-gate confirmation measured it
**+4.97 % and +7.63 % slower on wall** in two byte-identical pairs (paired
median **+6.30 %**, CPU +1.34 %). The standing 16-cell proof and 6-pair campaign
were **not started** by the stop discipline. Source restored byte-identical to
HEAD (blob `a59a11e5…`), both freshness gates exit 0, standing snapshots
rebuilt from the restored HEAD. Evidence:
[`evidence/t79-emit-batch-sweep/`](evidence/t79-emit-batch-sweep/FINDINGS.md)
(`verify.py` exits 0 checkout-only with 13/13 negative controls rejected).
Frontier now: **no open child remains**; the remaining fog is the human's to
route. Standing paired acceptance and KEEP/DROP rules unchanged.

**Previous selection scope (2026-10-22, on the ticket 78 take):** the human's
"continue" named no ticket, so the session took the only frontier ticket,
[78](issues/78-tree-support-planner-emit-batching.md), and **resolved it
(2026-10-22, human-approved KEEP, candidate committed `73a6f1e8`)**.
`build_roles` stays as the serial reference; `build_roles_batched` runs the emit
pass four layers at a time through `clip_polygons_batch` (per-region carve, role
union, set-wide difference, three body-minus-interface rounds, territory
intersect), keeping entry order and every per-call argument identical. **Proof
16/16 PASS**: baseline and candidate are byte-identical in every cell and equal
to the preserved t58 references; the snapshots differ in exactly one module file
per mode (`tree-support-planner.wasm`) and their binaries are byte-identical.
**Standing gate PASS in all four pre-declared classic supports-on cells**,
6 alternating pairs each, 24/24 retained, zero starvation exclusions, every pair
byte-identical: wall **−13.41 % / −12.30 % / −7.22 % / −16.66 %** and CPU
**+0.51 % / +1.51 % / +0.74 % / +2.80 %** (ordinary/accelerated × benchy/base;
accelerated-base's +2.80 % sits 0.2 pp under the pre-declared +3.0 % ceiling).
Attribution: `PrePass::SupportGeometry` **127,723 → 81,362 ms**,
`com.core.tree-support-planner` **109,018 → 60,038 ms** — the 79.7 s clip core
falls to ~30.7 s, a **2.6x** effective speedup rather than the 12x idealized
ceiling. Guest suite 146/0, including an item-for-item parity test against
`build_roles`; guest and workspace clippy clean, `check-literals` 0,
`check-test-quality --report` 0, both freshness gates exit 0. Frontier now: **no
open child remains**; the unswept `EMIT_BATCH_LAYERS`/`CLIP_BATCH_MAX_REQUESTS`
lever and the rest of the fog are the human's to route. Standing paired
acceptance and KEEP/DROP rules unchanged. Evidence:
[`evidence/t78-planner-emit-batching/`](evidence/t78-planner-emit-batching/FINDINGS.md)
(`verify.py` exits 0 checkout-only with 19/19 negative controls rejected).

**Previous selection scope (2026-10-22, on the ticket 77 take):** the frontier
re-derivation found **no open child** (all 76 children resolved), so the route
returned to the human, who selected the **tree-support planner union/carve**
option. [77](issues/77-tree-support-planner-batch-parallel-contract.md) was
graduated, claimed and **resolved as a measured attribution + contract take** —
no production edit, no timing campaign, no commit beyond its records. Measured
at HEAD on base classic supports-on with a reverted host/guest probe whose
output is byte-identical to the t58 reference `f60e5e9b…`: the planner module is
**126.674 s** of the 147.040 s `PrePass::SupportGeometry` stage, and its
**79.659 s** `clip_polygons` core is **99.03 % inside `build_roles`** — 472 role
unions (52.4 %, median 79.2 ms, mean 312 input polygons) and 69,161 per-region
carves (44.9 %, median 0.474 ms). The three shapes the fog pointed at are
refuted by measurement: the `layer_outlines_below` prefix chain is **0.13 %**,
t76's intra-call partition has no target (max call 436 ms) and its sliver
tolerance cannot carry printed geometry, and the ladder unions are 0.7 %. The
surviving contract is **cross-call batching** through the existing
`clip_polygons_batch` — exact per call, input-ordered, fanned out on estimated
work — needing guest-side buffering, not host capability. Idealized ceiling
(arithmetic, unmeasured): **≈ 72.3 s off a 147.0 s stage**. One candidate
graduated: [emit-pass batched carve and role union](issues/78-tree-support-planner-emit-batching.md)
(open, unclaimed). Frontier now: **ticket 78 only**; the take order is the
human's. Standing paired acceptance and KEEP/DROP rules unchanged. Evidence:
[`evidence/t77-planner-contract/`](evidence/t77-planner-contract/FINDINGS.md)
(`verify.py` exits 0 checkout-only with 8/8 negative controls rejected; raw
capture under `target/t77-probe2/`, reproducible from `probe.patch`).

**Previous selection scope (2026-10-21 → 2026-10-22, on the ticket 20 take):**
the human lifted the below-fold park and selected
[20](issues/20-consume-only-when-read-annotations.md) explicitly ("Lift the
park; take ticket 20"). It is now **resolved (2026-10-22, human-approved KEEP,
candidate committed `053904aa`)**. The candidate is the audit's conditional
stamp: a shared classifier predicate
(`slicer_sdk::config_predicates::overhang_annotations_consumed`) is read by
both perimeter generators, and `PathAnnotationMode::Unconsumed` skips both
per-vertex exact scans when nobody reads the stamps. The dynamic premise check
logged `consumed=false` on 495 base classic-off layers; the probe was removed
before any snapshot build. **Proof 16/16 PASS** against the preserved t58
references; the standing paired gate **PASSED in all four pre-declared classic
supports-off cells**, 6/6 pairs each, wall **−25.93 % / −34.47 % / −14.38 % /
−18.10 %** and CPU **−37.67 % / −39.61 % / −24.47 % / −21.58 %**
(ordinary/accelerated × benchy/base), every pair byte-identical. The park lift
was candidate-scoped; the destination and ticket 12's ~1.5x condition are
unchanged. Frontier now: no open child remains; the fog holds the un-graduated
structural leads and the `is_bridge` parity observation. The take order is the
human's.

**Previous selection scope (2026-10-21, on the ticket 76 take):** the human's
"continue" resumed the already-claimed
[76](issues/76-meshanalysis-footprint-union.md); it is now **resolved
(2026-10-21, human-approved KEEP, candidate committed `465f2fd3`)**. The base
overhang union was 326,546 facets costing 15.0-15.9 s of a 19.5-20.0 s stage;
exact input reductions were refuted (0 duplicates and 0 zero-area facets; the
containment drop of 5.9 % saves ~1.5 s; any input reorder changes the serialized
footprint), so the candidate is a work-preserving partition: above 100,000
facets, Morton-ordered centroids are split into a fixed 48 chunks, unioned in
parallel, and merged by one final union. Production stage **19.5-20.0 s ->
4.8-5.9 s**; the region differs only by sub-resolution slivers (4.2e-8
relative) and polygon ordering. **Proof 16/16 PASS**; standing gate **6/6 pairs
in every cell, every pair byte-identical**, wall **-4.46 % / -5.21 %** and CPU
**-0.14 % / -0.93 %** (ordinary/accelerated base classic-on), controls
+2.11 % / +0.71 % wall inside overlapping min-max ranges with flat CPU. Three
campaigns were lost to host stalls and a requested pause; their partial trees
are preserved and unused, and a detached fourth campaign produced the
measurements. Evidence:
[`evidence/t76-footprint-union/`](evidence/t76-footprint-union/FINDINGS.md)
(`verify.py` exits 0 checkout-only with four negative controls rejected).
Frontier now: only [20](issues/20-consume-only-when-read-annotations.md),
parked below-fold; no unclaimed child remains. Standing paired acceptance and
KEEP/DROP rules unchanged.

**Previous selection scope (2026-10-19 to 2026-10-21, on the ticket 75 take):** the human's
"continue" named no ticket, so the session took the first frontier ticket in
order, [75](issues/75-shellclassification-phase-b-commit.md); it is now
**resolved (2026-10-19, human-approved KEEP, candidate committed `07b97cad`)**.
Attribution: the serial phase-B commit was **78–81 % two full-layer Miter
offsets** on base (65 % on Benchy) and only 8–18 % the `internal_bridge_angles`
input step, refuting t27's iteration-1 candidate shape; only 9 of 121 base
entries (13 of 43 Benchy entries) carry a non-empty lower-candidate bridge set.
The candidate precomputes each layer's exact commit result in the parallel
qualification phase and recomputes only those affected layers, so no geometry
function changed. Candidate stage walls: **40.8 → 12.6 s** (base classic-off),
**28.1 → 12.8 s** (base classic-on), **30.1 → 13.6 s** (base arachne-on),
**5.75 → 1.77 s** (Benchy classic-on); **proof gate 16/16 PASS**; standing gate
**24/24 pairs retained**, every pair byte-identical, wall **−3.08 % / −3.48 % /
−3.06 % / −4.11 %** and CPU **+0.42 % / +1.08 % / +0.86 % / +0.51 %**
(ordinary/accelerated × benchy/base). The ordinary-base **+1.08 %** is a
0.08 pp miss of the pre-declared +1.0 % work-preserving ceiling; the human
accepted it on the ticket-27 serialization precedent. The first timing campaign
was evicted by the harness after 4/6 ordinary-base pairs; the partial set is
preserved as `measure-interrupted-20261019` and not used, and the cell was
re-run whole in a detached replacement campaign. Evidence:
[`evidence/t75-shellclass-phase-b/`](evidence/t75-shellclass-phase-b/FINDINGS.md)
(`qualification.json`, `ab-reduction.json`, `split.json`, `verify.py` exit 0
with four negative controls). Frontier now:
[76](issues/76-meshanalysis-footprint-union.md) unclaimed;
[20](issues/20-consume-only-when-read-annotations.md) parked below-fold; the
take order is the human's. Standing paired acceptance and KEEP/DROP rules
unchanged.

**Previous selection scope (2026-10-18, on the t72 take):** the frontier was
re-derived on resume — the only open child was
[20](issues/20-consume-only-when-read-annotations.md), parked below-fold — and
the human selected a **Scoreboard refresh** from the route options (the map's
first full matched-pair re-run since 2026-09-22).
[72](issues/72-scoreboard-refresh.md) was graduated, claimed and **resolved
(2026-10-18) as a measurement-only take: Orca still wins all 8 cells
(3.30x–18.03x median wall; process CPU corroborating in every cell), no cell
within ~1.5x, so the [Gap budget per cell](issues/12-gap-budget-per-cell.md)
below-fold park condition is re-derived and still binding.** PNP ordinary wall
improved in 7 of 8 cells vs the 2026-09-22 table (−13.9 % to −34.6 %; benchy
classic-off +5.4 %), with the Orca-side drift (+1.5 %…+47 %, disclosed) meaning
part of the raw gap improvement is machine state, not PNP work. The DEV-174
supports-on taint is retired at the output level (all 16 PNP groups
`degraded=false`, `non_fatal=0`; ordinary ≡ accelerated bytes in all 8 cells).
Two infrastructure evictions of long background batches were disclosed and
recovered per the t57 discipline (interrupted cells re-run whole; evicted
partial CSVs preserved; base classic-on/arachne-on assembled from per-tool
chunks under a disclosed interleave deviation). Zero starvation exclusions,
n=3 per tool, both freshness gates green in every preflight. Evidence:
[`evidence/scoreboard-refresh-20261017/`](evidence/scoreboard-refresh-20261017/SCOREBOARD.md)
(`verify.py` exits 0 checkout-only + self-test). No production edit, no
candidate, no production-source commit. Frontier now:
[20](issues/20-consume-only-when-read-annotations.md) parked below-fold; the
take order is the human's. Standing paired acceptance and KEEP/DROP rules
unchanged.

**Previous selection scope (2026-10-17, on the t71 take):** the human selected
[71](issues/71-xtask-compile-time-workspace-root.md) from the route options
presented after t70's closeout and approved the commit. It is **resolved
(2026-10-17) as a verified infra repair, committed `f067efa2`**: ticket 70's
xtask compile-time-root mechanism was reproduced contained in a detached
worktree and refined (a foreign invocation overwrites the shared
`target/debug/xtask.exe` with a foreign-baked binary; while the foreign root
is alive the main tree's gates silently scan the foreign tree
indistinguishably, and only once it dies do they print the recorded false
green — "cannot read <dead>\crates" + `watchlist: 0 types` + exit 0). The
fix: `workspace_root()` resolves at runtime (walk up from CWD to the nearest
`Cargo.toml` + `crates/` ancestor; loud panic otherwise; the compile-time
macro is gone from the production path; one fix point covers every
subcommand and derived artifact path), both gates refuse a zero-file scan
(exit 1, every mode incl. `--report`), and the two `#[cfg(test)]` helpers
resolve runtime-first with the compile-time fallback. Behaviorally proven
with a foreign-checkout-built binary: CWD in the main tree → real 159-type
watchlist, exit 0; CWD in a repo-less void → loud exit 101.
`cargo test -p xtask` 185/0 (9 new), workspace clippy exit 0,
check-literals green (159), check-test-quality --report 0 findings,
build-guests --check exit 0; no guest touched. Evidence:
[`evidence/t71-xtask-worktree-root/`](evidence/t71-xtask-worktree-root/FINDINGS.md).
Commit decision is the human's — given 2026-10-17 (approved), the fix is
committed `f067efa2`. Frontier now:
[20](issues/20-consume-only-when-read-annotations.md) parked below-fold; the
take order is the human's. Standing paired acceptance and KEEP/DROP rules
unchanged.

**Previous selection scope (2026-10-17, on the t70 take):** the frontier was
re-derived on resume — open:
[20](issues/20-consume-only-when-read-annotations.md) parked below-fold —
and the human selected [70](issues/70-clipper-in-tree-vendoring-first-candidate.md)
as the route for this take ("Pick 70 (candidate take)"). It is **resolved
(2026-10-17) as a kept production change**: `clipper2-rust` 1.1.0 vendored
in-tree with sealed provenance, the engine's hot-loop software `nearbyint`
round-half-to-even emulation taken to the hardware `round_ties_even`
(44-line candidate, MSRV 1.77 in the vendored crate), value-exact 295/295
corpus cases, 16/16 output-identity cells also matching the t58 preserved
standing references, and the standing paired gate PASS in all four
pre-declared classic-supports-on cells, both modes (wall −15.59 to −18.30 %,
CPU corroborated, 6/6 pairs, zero starvation exclusions). Human-approved
KEEP; candidate committed `7127b4d6`; standing dirs purged pre-commit and
refreshed at the kept HEAD. The closeout surfaced one upstream follow-up —
xtask's `workspace_root()` bakes the compile-time
`CARGO_MANIFEST_DIR`, so a worktree invocation poisons the shared target's
cached xtask until its build units are purged (gates then silently scan zero
files — details on ticket 70's answer) — for the human to route. It is now
ticketed as [71](issues/71-xtask-compile-time-workspace-root.md) (open,
unclaimed; charted on continuation 2026-10-17). Frontier now:
[71](issues/71-xtask-compile-time-workspace-root.md) and
[20](issues/20-consume-only-when-read-annotations.md) parked below-fold; the
take order is the human's. The standing paired acceptance
and human KEEP/DROP rules are unchanged.

**Previous selection scope (2026-10-16, on the t69 take):** the frontier
was re-derived on resume — open:
[20](issues/20-consume-only-when-read-annotations.md) parked below-fold and
[69](issues/69-clipper-vendoring-optimization.md) unclaimed — and the human
selected [69](issues/69-clipper-vendoring-optimization.md) as the route for
this take ("rely on subagents"). It is **resolved (2026-10-16) as a measured
headroom verdict — not parity**: the sealed pristine engine runs **1.42–2.86x
slower than the canonical Clipper2 C++ 1.5.2** per opclass on identical
matched-workload inputs (offset 2.03–2.09x / union 2.61–2.63x / intersection
1.94–2.00x / difference 1.60–1.76x; 295 cases × two passes), with idealized
base-off accumulated ceilings of ≈ 256–258 s (offset) / 199–200 s (closing_ex)
/ 80–82 s (clip_polygons) and a directly-sampled CP-only diagnostic scale of
≈ 1.2–1.5 % — above the t68 below-floor context, so one candidate ticket is
graduated:
[70](issues/70-clipper-in-tree-vendoring-first-candidate.md) (open,
unclaimed). No candidate was implemented, adopted, or timed against the
slicer; the capture probe is reverted (both capture runs byte-identical to
the t58 references), both freshness gates green, no auto-commit. Frontier
now: [70](issues/70-clipper-in-tree-vendoring-first-candidate.md) and
[20](issues/20-consume-only-when-read-annotations.md) (parked below-fold);
the take order is the human's. The standing paired acceptance and human
KEEP/DROP rules are unchanged.

**Previous selection scope (2026-10-16, on the t68 take):** the frontier was
re-derived on resume — open:
[20](issues/20-consume-only-when-read-annotations.md) only, parked
below-fold, with the route decision back to the human — and the human
selected the **classic-perimeters per-layer host-service round trips** lead
from the t60 un-graduated set as the route for this take.
[68](issues/68-classic-perimeters-host-round-trips.md) was graduated,
claimed and **resolved as an attribution take — no candidate**: the 8+3
per-layer round trips decompose per site (guest-observed + host conversion/
ops split) to **97.55 % (benchy-off) / 99.02 % (base-off) native clipper
ops**; the entire round-trip overhead is 2.45 % / 0.98 % and every
reduction direction t60 named (site-level, batching, in-guest migration)
has a measured ceiling ≤ 0.80 % of run wall even if perfectly serialized
(≤ 0.07 % saturated) — below any gate this map has applied. Both captures
byte-identical to the t58 references; probe reverted, freshness green; no
production edit, no timing campaign, no auto-commit. Evidence:
[`evidence/t68-host-roundtrips/`](evidence/t68-host-roundtrips/FINDINGS.md).
Ticket 67's stale `Status: claimed` line was flipped to `resolved` (its
KEEP was already committed as `f4c542a3` / `84ec520a`). Frontier now:
[20](issues/20-consume-only-when-read-annotations.md) remains the only open
child (parked below-fold); the `region_partition` dedup lead remains
un-graduated — route decision back to the human. The standing paired
acceptance and human KEEP/DROP rules are unchanged.

**Same-session addendum (2026-10-16, on the human's direction):** after t68
measured 97–99 % of the round-trip bucket as native clipper ops, the human
directed a new exploration ticket on the engine itself —
[69](issues/69-clipper-vendoring-optimization.md): vendor `clipper2-rust`
in-tree and optimize its hot paths, with the headroom measured against the
bundled canonical Clipper2 C++ first, byte-exactness and the standing gate
unchanged. It is open and unclaimed. Frontier now:
[20](issues/20-consume-only-when-read-annotations.md) (parked below-fold) and
[69](issues/69-clipper-vendoring-optimization.md) (unclaimed) — the take
order is the human's.

**Previous selection scope (2026-10-16, on the t67 take):** the frontier was
re-derived on resume — open:
[67](issues/67-region-preparation-derivation-sharing.md) first and
[20](issues/20-consume-only-when-read-annotations.md) parked below-fold — and
the session took the first frontier ticket in order:
[67](issues/67-region-preparation-derivation-sharing.md) — **resolved
(2026-10-16) as measured GATE NOT MET with a human-approved KEEP**: the wall
(destination) clause passed in all four pre-declared classic supports-on
cells (paired medians −0.11 %, −4.26 %, −0.50 %, −0.28 %; 6/6 retained pairs,
zero starvation exclusions; 16/16 qualification byte-identical to the
preserved t58 references) with the ordinary base-classic-on win
CPU-corroborated (−0.60 %), while the CPU corroboration clause missed in
accelerated-benchy by +0.19 % (+0.31 s on a 118 s run) — the human kept the
candidate over that sub-noise miss and it is committed (`f4c542a3`) with its
tests. Frontier now:
[20](issues/20-consume-only-when-read-annotations.md) is the only open child
(audited, parked below-fold), awaiting the human's route decision. The
standing paired acceptance and human KEEP/DROP rules are unchanged; new
questions require the human's route decision.

**Previous selection scope (2026-10-14, on the t60 take):** the frontier was
re-derived on resume — open:
[60](issues/60-host-native-attribution.md) first and
[20](issues/20-consume-only-when-read-annotations.md) parked below-fold — and
the session took the first frontier ticket in order:
[60](issues/60-host-native-attribution.md) — **resolved (2026-10-14) with the
`host:native` bucket fully attributed: residual zero and exact scope-call
reconciliation in all four classic cells** (owner families: the two
region-preparation derivations, the `region_partition` commit hook,
classic-perimeters/linker guest host-service calls, the prepass seam-planner,
and on supports-on the planner/renderer/batch/aggregation support machine;
outputs byte-identical to the t58 references; probe reverted, no production
edit). One candidate graduated:
[67](issues/67-region-preparation-derivation-sharing.md). Frontier now:
[67](issues/67-region-preparation-derivation-sharing.md) (open, awaiting the
human's route decision) first, [20](issues/20-consume-only-when-read-annotations.md)
parked below-fold. The standing paired acceptance and human KEEP/DROP rules are
unchanged; new questions require the human's route decision.

**Previous selection scope (2026-10-14, on the t59 take):** the frontier was
re-derived on resume — open:
[59](issues/59-prep-slice-flat-bridge-closing-reduction.md),
[60](issues/60-host-native-attribution.md), and
[20](issues/20-consume-only-when-read-annotations.md) (parked below-fold) —
and the session took the first frontier ticket in order:
[59](issues/59-prep-slice-flat-bridge-closing-reduction.md) — **resolved
(2026-10-14) as verdict (b): no representation-safe reduction** (the enclosure
closing's per-component 10 % gate is decided by miter-offset artifacts from
~72–74 degenerate sliver paths in `support`; a set-preserving local-closing
candidate flips verdicts on 38.5 % of benchy and 68.2 % of base call layers;
far-guards have zero hits; the refuted shape's ceiling is ≈12 % of the
closing's accumulated thread-time). No production edit, no timing campaign,
nothing graduated. Frontier now: [60](issues/60-host-native-attribution.md)
first, [20](issues/20-consume-only-when-read-annotations.md) parked
below-fold. The standing paired acceptance and human KEEP/DROP rules are
unchanged; new questions require the human's route decision.

**Previous selection scope (2026-10-14, after the t66 take):** the human
selected [66](issues/66-paint-policy-dead-walk-reduction.md) from the open
frontier; the take resolved as a **measured GATE NOT MET — human-approved
DROP** (annotations-empty fast path; 16/16 identity qualification; benchy
wall flat inside paired noise while base wall and all-cell CPU improved;
candidate retired to the committed re-appliable patch, no production edit).
Next: re-derive the open frontier on resume. The standing paired acceptance
and human KEEP/DROP rules are unchanged; new questions require the human's
route decision.

**Previous selection scope (2026-10-14, after the t64 take):** on
resumption the human said "continue" without naming a ticket, so the session
took the first frontier ticket in order —
[64](issues/64-tree-support-renderer-attribution.md) — **resolved as an
attribution take (2026-10-14)**: the probed renderer's fuel is owned by a
**dead paint-policy scan** (`paint_policy_for` 79.7%: 0 covers / 0
eligibility checks in 836 queries — walk without a covering region, exact
reduction shapes identified) with skeleton containment second at 15.9%;
`scan_fill_region` — the ticket's named suspect — is measured **0.43%
inclusive, no-useful-reduction**; wall relevance is bounded (renderer
accumulated worker 6.79–8.35 s over 12-thread phases of 8.2–8.7 s =
6.5–8.5% occupancy, 0–2 ms on the last-finishing layer). The reduction
graduated to
[66](issues/66-paint-policy-dead-walk-reduction.md) (open, awaiting human
route/KEEP-DROP later; ticket creation is not execution approval). The
existing paired-mode acceptance and human KEEP/DROP rules are unchanged; new
questions require the human's route decision.

**Previous selection scope (2026-10-13, after the t63 take):** the dispatch-granularity mechanism is
established at worker level, the ticket's measured opportunity is shown
stale at post-t65 HEAD (deficit 42–50% → 2.4–7.0%; lone-open tail gone),
the generic experiment is recommended NOT opened, and the fresh scheduling
baseline the t65 route note required has now been taken (seven probe runs;
see the ticket's Answer and
[evidence](evidence/t63-lowlayer-dispatch/FINDINGS.md)).

Standing decisions for this effort (2026-09-22):

- **Execution is carried into this map**: tickets are worked as measurement /
  implementation experiment sessions — deliberate override of wayfinder's
  "plan, don't do" default.
- **Verdict metric**: median uninstrumented wall-clock; process CPU must
  corroborate (never contradict) on within-PnP **candidate A/B gates**.
  **Amended 2026-11-21 ([t176](issues/176-destination-cpu-corroboration-ruling.md)):**
  on cross-engine **Destination cell closure** (PnP vs OrcaSlicer on the
  binding arm `pnp-ordinary`), process CPU serves as a **CPU integrity guard**
  (tight CPU, unstarved `cpu/wall`) with PnP/Orca CPU ratio disclosed as a
  separate efficiency metric rather than requiring `CPU_pnp <= CPU_orca`.
  `cpu/wall` ratio quoted per sample, starved samples (`< 0.75 × group best`)
  excluded (§10.3 trap; `0` starvation required on `n=3` wide-margin provisional
  wins, `>= 4` retained unstarved runs required on close cells and the final
  8/8 closing board).
- **Fairness contract**: matched settings define the job (nozzle, layer height,
  wall and infill counts, wall generator, supports). Every comparison discloses
  output stats (bytes, TYPE counts, degraded flag); a known work-skipping
  defect disqualifies that cell.
  **Resolution was NOT matched until 2026-10-23** (disclosed 2026-10-22 by
  [ticket 80](issues/80-resolution-fairness-premise.md), closed by
  [ticket 81](issues/81-resolution-slicing-time-contour-simplification.md): PnP
  now implements the same slicing-time simplification). The disclosure below is
  kept because it is what every pre-t81 ratio in this map was measured under.**
  **Pre-t81 disclosure:** The matched Orca
  profile inherits `resolution = 0.01 mm` and applies it as a *slicing-time*
  contour simplification before walls and infill are generated
  (`PerimeterGenerator`'s `surface_simplify_resolution` → `simplify_p`,
  `Fill.cpp`); PnP implements no slicing-time equivalent, only emission-time
  per-role D-P (`gcode_resolution` / `infill_resolution` / `support_resolution`),
  whose defaults are exactly Orca's internal per-role constants — faithful
  porting of a *different* mechanism.
  Running Orca at its canonical 0.001 mm floor instead costs it **+9.5 % to
  +65.2 % wall** across the eight cells (median **+40.2 %**, CPU +21 % to
  +92 %) — so every ratio in this map compares PnP at full contour detail
  against an Orca doing materially less geometric work. Correction factor per
  cell is `1 + Δ` from
  [`evidence/t80-resolution-fairness/summary.json`](evidence/t80-resolution-fairness/summary.json);
  apply it to a current ratio, never quote it as a scoreboard. The
  **fairness-counterfactual column** (raw ÷ (1 + Δ), 2026-10-23) is derived by
  [`corrected_scoreboard.py`](evidence/t81-resolution-port/corrected_scoreboard.py)
  into [`corrected-scoreboard.json`](evidence/t81-resolution-port/corrected-scoreboard.json):
  benchy 3.25/2.50/3.55/3.23, base 8.35/6.75/12.63/11.96 (ordinary, raw ÷ (1+Δ)).
  No verdict flips and the best cell moves **3.30x → 2.50x**, so the ~1.5x park
  condition stays binding. Raw ratios remain the headline.
  **Supports-on engine-style pin (2026-11-02, human ruling on
  [ticket 105](issues/105-supports-on-engine-style-fairness.md); supersedes
  ticket 97's mismatch disclosure):** the matched job now pins Orca's tree
  style to **`tree_strong`**, so both sides run the classic engine class PnP
  implements. Orca's canonical default on a tree family is `smsTreeOrganic`
  (`SupportParameters` → `generate_tree_support_3D`); PnP substitutes **Strong**
  for every canonically-organic input (`DEV-156`). The pin is a **protocol
  choice, not Orca's default job**, and it is disclosed beside every
  supports-on ratio: measured on 2026-11-02 over 4 interleaved paired rounds,
  it costs Orca **+39.6 / +44.4 %** wall on the two base cells and
  **+7.6 / +9.5 %** on the two benchy cells, and changes output
  (**+23.5–35.3 %** bytes; support-interface sections 230 → 423 base
  classic-on, 7 → 57 benchy classic-on). The **effective branch distance is
  1 mm on both sides** (`tree_support_branch_distance_organic` on the Orca side,
  the normal-key default on the PnP side); the deviations table's 5 mm
  normal-key row is inert. **Post-pin scoreboard, re-derived by
  [ticket 106](issues/106-post-pin-supports-on-scoreboard.md) (2026-11-02, 1
  warmup + 3 measured per cell, load 16.4 % → 3.3 %, no starvation
  exclusions):** Orca **6.02x / 6.11x** (base classic / arachne) and
  **4.77x / 4.53x** (benchy classic / arachne), so the park still needs
  **66.9–75.5 %** of each cell removed. Ticket 103's supports-on Orca medians are
  **superseded (pre-pin)** and may be quoted only with that label. The pinned job
  is **byte-identical to ticket 105's `tree_strong` arm** on all four cells while
  running **1.9–9.6 % faster today**, so t105's derived transfer is **retired as
  an absolute predictor** — it agreed on two cells and missed base arachne-on by
  **−17.95 %** — and any style price must be paired same-session, with the day
  named beside any absolute median.
- **Win matrix**: all 8 cells decide, so support-correctness defects are route
  work — a degraded slice is not Orca's job.
- **Measurement mode**: paired ordinary + accelerated runs are the default
  keep/drop gate; the human may accept a one-sided win case by case.
- **Output-changing prepass candidates (2026-11-03, [ticket
  111](issues/111-prepass-rewrite-scope-ruling.md)):** for the **prepass only**,
  the byte-identity proof is replaced by a fresh **16-cell** reference set
  (8 cells x ordinary / accelerated), candidate and baseline arms byte-identical
  **within themselves**, the baseline reproducing the post-t81 set, the
  candidate-vs-old-reference G-code delta audited (feature / TYPE /
  support-interface counts, byte volume, confinement to prepass-derived geometry)
  **before** timing, and the standing paired wall/CPU gate at the **0.80 %**
  floor in **all 8 cells**. The exception is scoped to `PrePass::Slice` (first,
  [ticket 112](issues/112-prep-slice-parity-phase1.md)) and the Strong planner
  (second, [ticket 113](issues/113-strong-planner-phase1.md)); no other parity
  work is authorized, and the next parity question goes back to the human.
- **Timing discipline**: timing/acceptance tickets chain in the take order set
  by [Gap budget per cell](issues/12-gap-budget-per-cell.md) (number order
  superseded 2026-09-23; chain: 15 ✅ → 17 ✅ → 27's candidate → **22 ✅
  (attribution; produced the emit-pass carve candidate)** → **[emit-pass carve
  gate acceptance](issues/32-emit-carve-gate-representation-and-ab.md) ✅**
  → **21 ✅ (attribution, 2026-10-01; produced the
  → **[clip-universe hoist](issues/44-clip-universe-hoist.md) candidate, subsequently
  resolved as KEEP approved under the human-approved fresh-baseline gate)** → **25 ✅ (attribution, 2026-10-01; produced the
  [overhang-quartile predicate narrowing](issues/45-overhang-quartile-predicate-narrowing.md)
   candidate **resolved as human-approved DROP/inconclusive (2026-10-05;
   output-identical bbox alternative fails the ordinary-wall clause; candidate
   source and active artifacts restored)**; its gate prerequisite
  [Frozen-job output drift boundary](issues/46-frozen-job-output-drift-boundary.md)
  **is resolved (2026-10-05: intended behavior; current refs `5a61a106…` and `e1088b3d…`+alias;
  frozen `7049a06d…` / t44 `1b71f83d…` retire as this job's timing baselines)** — post-ticket-47 the fan-command reference
  boundary (see Decisions: ticket 47) also applies to any fresh reference
  45 derives)** → below-fold
  polish — 15, 17, 22
  closed 2026-09-23/24); attribution-only tickets are parallel-takeable, but no
  substage split starts before [host:slice closing_ex span
  contradiction](issues/24-host-slice-closing-span-contradiction.md) closes.
   Claim (`Status: claimed`) before any work.
   2026-09-29 route update: the human parked 27's candidate search after
   accepting the sibling-region memo DROP; attribution-only 28 is resolved.
   Its new [Benchy classic sparse-fill domain at middle layers](issues/35-benchy-classic-sparse-fill-domain.md)
   and [Modal G-code Z/F token redundancy and speed gate](issues/36-modal-gcode-token-redundancy.md)
   are distinct frontier investigations, not already-accepted optimizations.
   2026-09-29 second update: 35 resolved as a **containment defect, not a fill
   domain** — the linker's clipped-to-nothing verdict is overridden by the
   empty-output preservation rule, printing 68% of classic Benchy's sparse
   output outside the part. Its fix is graduated to [InfillPostProcess
   empty-output protocol](issues/37-infillpostprocess-empty-output-protocol.md),
   which touches the linker/host contract rather than the fill geometry and
   needs the standing paired A/B before any keep. 36 stays open and parallel.
   2026-09-29 third update: **37 resolved and fixed** — a ran
   `Layer::InfillPostProcess` invocation now commits the empty replacement set,
   so the linker's clip verdict supersedes the raw envelope. Stage-local (only
   this stage's contract is a complete replacement set); `Layer::Infill`,
   `Layer::Support`, `Layer::SupportPostProcess` and `Layer::AnchoredEvents`
   keep their existing empty-output semantics. Verified on real G-code (60
   classic layers → zero sparse, worst overshoot 21.78 mm → 0.00 mm; base.stl
   byte-identical as the negative control) and paired ordinary + accelerated
   A/B measured **no wall win with corroborating CPU** — the case is
   containment, not speed.
   Recommended KEEP to the human; not auto-committed.
   **2026-10-03 update: 36 claimed and resolved; DROP approved by the human.**
   The human selected the
   [Modal G-code Z/F token redundancy and speed gate](issues/36-modal-gcode-token-redundancy.md)
   as this session's take from the open frontier. Resolution:
   **DROP/inconclusive, approved by the human 2026-10-03.** The modal Z/F
   candidate is proven semantics-preserving by an independent interpreter
   (`f49b5ec5…` / `c237fb01…` normalized move-stream SHA, both cells, both
   modes) and removes 27.0–27.1% of G-code bytes, but it fails the pre-declared
   CPU-corroboration gate in 3 of 4 batches (wall −0.3…−1.7%, inside the job's
   own spread; CPU flat-to-higher), and a ~1% effect cannot matter against this
   destination's 6.28–26.24x gap. The candidate stays uncommitted and is
   preserved as a re-appliable patch; a byte/bandwidth-bound consumer would be
   a separate decision. A second audit session re-derived every figure from the
   raw captures and replayed both arms byte-for-byte (see the ticket's audit
   section), corrected one `FINDINGS.md` gist sentence on the CPU deltas,
   purged the take's candidate-built binaries from the standing
   `target/dist-accelerated/developer/` and `target/debug/` dirs (rebuilt from
   HEAD), and disclosed the t47-caused `empty_end_gcode_emits_no_block` red test
   on the ticket.
   See [Modal G-code Z/F token redundancy and speed gate](issues/36-modal-gcode-token-redundancy.md).
- **Recipe trap (new 2026-09-29):** a `Layer::InfillPostProcess` (or any stage)
   invocation that emits no paths is indistinguishable from a stage that did not
   run — the host preserves the prior `InfillIR`. Never read "the linker produced
   nothing" as "no linking was needed"; check the linker's own kept-length from
   `evidence/t35-fill-domain/probe.patch` before attributing output to the fill
   generator.
- **Recipe trap (narrowed 2026-09-29 by 37):** the trap above applies to
   *instrumented analysis of the pre-fix tree*. From the fix onward a ran
   `Layer::InfillPostProcess` that re-emits nothing commits the empty
   replacement set, so "no sparse output on this layer" is a real verdict on
   both engines. The distinction still matters for `Layer::Infill` (merge
   stage, unchanged) and for genuinely absent stages. See
   [InfillPostProcess empty-output protocol](issues/37-infillpostprocess-empty-output-protocol.md).
- **Recipe traps** (each has produced or nearly produced a wrong conclusion):
  instrumented runs never mixed into wall claims (§3.2); accumulated worker
  elapsed is not CPU (§3.3); the native `--profile` table's wall columns are
  accumulated per-thread spans (`flush_native` sums concurrent worker spans),
  never wall — do not compare them to `module_complete`/phase walls
  ([host:slice closing_ex span contradiction](issues/24-host-slice-closing-span-contradiction.md),
  2026-09-23; the semantics are now stated at the definition sites, the
  rendered table, and `17_agent_debugging.md` — see [profile wall-column
  semantics](issues/30-profile-wall-column-semantics.md), 2026-10-07);
  `--module-dir modules/core-modules` is mandatory
  (§3.4); `cargo xtask build-guests --check` must exit 0 first (§3.5); base.stl
  wall varies ±13% so take repeats (§3.6); accelerated runs use the complete
  `cargo xtask dist --accelerated` snapshot at `target/dist-accelerated/developer/`
  — the bare artifacts dir silently loads integrated modules, and dual
  `--module-dir` does not shadow (2026-09-22 recipe addendum); criterion's
  console `time:` is the regression *slope* in Linear sampling mode while its
  `change:` line is mean-based — read `target/criterion/**/estimates.json` (or
  `evidence/t15-criterion-refresh/baselines.json`) and compare like-for-like
  ([Criterion bench refresh](issues/15-criterion-bench-refresh.md), 2026-09-23);
  a PNP single-slice subprocess pays ~0.77 s of module compile *before* the
  `validation` phase event, so phase walls sum to ~10% of
  `slice_complete.elapsed_ms` — never read phase-sum as run wall
  ([Criterion bench refresh](issues/15-criterion-bench-refresh.md), 2026-09-23);
  a scratch harness in its own cargo workspace inherits only the *caret*
  requirement, so its lock floats to the newest compatible release and a
  version A/B silently measures the wrong version — pin the harness dep and
  assert the resolved version from the harness's own `Cargo.lock`
  ([clipper2 cost/output verdict](issues/17-clipper2-cost-output-verdict.md),
  2026-09-24); host-side stage walls differed materially between base captures
  with the same host probe but different guest bracket sets while call counts
  and class shares held (two ticket-22 captures: `clip` 146.8 s vs 194.0 s,
  in-guest carve counts byte-identical) — quote shares/counts as the stable
  signal and disclose wall drift rather than averaging it away; external load
  is plausible but its cause was not isolated
  ([Tree-planner substage
  attribution](issues/22-tree-planner-substage-attribution.md), 2026-09-24).
- **Recipe trap (new 2026-10-27, t89):** a memory-bound host kernel can make
  parallelization *burn* CPU. Twelve-way concurrent exact-Z cross-sections ran
  **92.73 s of accumulated thread-time for a 7.99 s wall**, while the same 431
  queries cost **15.24 s serial** (t88) — a **−5.50 %** wall win for a
  **+6.78 %** process-CPU rise, measured on a 2 %-load machine over three
  alternating pairs. Price any future parallel candidate in thread-time
  (wall × occupancy), not wall alone, and treat "work-preserving" as false the
  moment process CPU rises. A work-reducing serial shape (memoize what does not
  depend on the varying input) is the alternative to check first
  ([ticket 91](issues/91-exact-z-object-geometry-memo.md)).
- **Recipe trap (new 2026-10-27, t91):** a per-object memo is invisible to the
  per-key cache counters. After the z-independent terms were memoized the
  exact-Z service still reported **431 queries / 431 misses** on the base job,
  because the cache key carries the requested Z; the memo was visible only in a
  **geometry-term build** counter (**431 → 1**) and in the query-time split
  (**22.6 → 8.9 s**). Price a memo by the term it builds, not by the hit rate
  ([ticket 91](issues/91-exact-z-object-geometry-memo.md)).
- **Recipe trap (new 2026-10-28, t92):** a per-item / per-surface shape must be
  priced against the per-layer operand build it rides on. The internal-bridge
  qualification's per-surface offsets and intersections are **0.25 s**
  accumulated, while the two whole-layer offsets that build their operands are
  **20.7–22.5 s** — ~80x larger and invisible to a probe that starts after the
  operands exist. Put the operand construction in the same bracket as the
  consumer, or the screen prices the wrong order of magnitude
  ([ticket 92](issues/92-per-surface-bridge-boolean-locality.md)).
- **Recipe trap (new 2026-10-25, t88):** a probe's `Instant` bracket can disagree
  with the event timestamps by the width of an earlier phase (clock-read
  hoisting or a non-monotonic QPC on this host), so a 5 ms clone can read as
  4.09 s while the run is otherwise correct. Anchor probe marks to
  `SystemTime` (`at_ms`) or black-box the clock, and cross-check every
  `Instant` delta against the event timestamps before quoting it
  ([ticket 88](issues/88-supports-on-increment-attribution.md)).
- Measured keep/drop recommendations return to the human; candidates are never
   auto-committed.
  **Prepared-cache experiment completed; DROP approved (2026-10-06):**
  [Prepared winding-coordinate cache](issues/51-prepared-winding-coordinate-cache-experiment.md)
  implemented the exact cache and passed exactness/output-identity, but the
  initial campaign was inconclusive and the one explicitly authorized repeat
  failed accelerated wall under the unchanged both-modes gate (ordinary passed).
  The human approved **DROP and restore**: candidate source/tests retired to
  `HEAD`, the preserved patch remains re-appliable, rebuilt guests are
  byte-identical to archived baselines, and both freshness checks are green.
  No commit, adoption retry, one-sided exception or new ticket; see
   [findings](evidence/t51-winding-cache/FINDINGS.md).
   **Current route (2026-10-07):** the human confirmed the
   [Scanbeam TopX projection-cache experiment scope](issues/56-scanbeam-topx-cache-experiment-scope.md)
   after the completed large-body-union diagnostic. Its
   [execution child](issues/57-scanbeam-topx-cache-experiment.md): the isolated
   shared-engine candidate passed exactness and the entire fresh 16-cell output
   matrix byte-for-byte in both modes, but the frozen paired timing campaign
   **stopped at its first cell — `ordinary-benchy-classic-on` — with the
   candidate slower on both wall and process CPU in 6 of 6 retained pairs**
   (wall +2.14%, CPU +2.78%, no starvation exclusions). Because the both-modes
   gate requires every timed cell to pass, the remaining three cells were not
   run by rule, and the measured recommendation is **DROP — human accepted
   2026-10-07** (no commit, adoption or retry; the standing authorization was
   confirmed to cover the externally interrupted-cell rerun). Two external
   orchestration stops
   (a self-inflicted command timeout and a harness session eviction) were
   recovered by re-proving completed cells from raw bytes and rerunning only
   untouched cells; no gate-failing cell was ever rerun. Production restored,
   both freshness checks green; see
   [findings](evidence/t57-topx-cache/FINDINGS.md). Earlier
   below-fold/serial-floor parking remains in force.
   **Next route (2026-10-07):** after t57's DROP the human selected the small
   non-timing ticket family. This session took the first in number order,
   [mesh_ops fixture limits](issues/29-mesh-ops-fixture-limits.md), and
   resolved it (fixture strategy recorded, both criterion leaves replaced,
   guards + negative control, fresh baselines). Tickets 30/31/34 remain open
   frontier; no route-performance work was authorized or performed by that take.
   **Same route, continued (2026-10-07):** this session then took the next in
   number order, [profile wall-column semantics](issues/30-profile-wall-column-semantics.md),
   and resolved it — the accumulated-thread-time unit is now stated at the type
   docs, module docs, rendered table, ADR-0055, `09_progress_events.md`, and
   `17_agent_debugging.md`, with the rendered note pinned by a test.
   **Same route, third take (2026-10-07):** the session then took
   [clipper2 1.2.0 geometry delta](issues/31-clipper2-1-2-0-geometry-delta.md) and
   resolved it — **stay on 1.1.0, no bump**. The 2-of-367 fixture delta was
   reproduced and attributed to `clean_collinear`, which is a
   **canonical-convergence fix** the repo's bundled canonical Clipper2 1.5.2
   already has, so 1.1.0 is the diverging side; DEV-173's posture is unchanged
   and the bump buys nothing this map needs. `PolyFace64` is a 1.1.0 addition
   (premise corrected) and remains unused. Ticket 34 was then the last open
   frontier ticket in this family; no timing run, candidate, adoption or commit
   was part of the take.
   **Same route, fourth take (2026-10-07) — the family is now complete:** the
   session took that last one,
   [Config padding disclosure boundary and viewer-key gate](issues/34-config-padding-disclosure-boundary.md),
   and resolved it. The premise was **corrected before acting**: `ORCA_CONFIG_PADDING`
   is inert in every production path (the registry projection reaches 233 pairs
   while the padding loop breaks at 96, so it never fires; the table only serves
   the test/legacy callers). The audit nevertheless found **two real wire-format
   defects in the block itself** and fixed both: booleans were rendered
   `true`/`false` where OrcaSlicer's wire literals are `1`/`0` (23 production
   lines; the substitution branch in `ConfigBase::set_deserialize_raw` —
   `enum_looks_like_true_value` is true only for `enabled`/`on` — loaded every
   rendered `true` as `false`, e.g. `detect_overhang_wall` and
   `support_remove_small_overhang`), and
   four enum values used PnP-internal spellings
   (`wall_sequence`, `wall_generator`, `wall_direction`, and the `seam_mode` alias
   rendering `rear` for Orca's `back`), which Orca's strict map lookup answers by
   substituting the option's own default. Two more keys PnP models as `bool` but
   Orca declares as `coEnum` (`dont_filter_internal_bridges`,
   `enable_extra_bridge_layer`) went through the same translator. The table itself
   was audited row by row: 10 values were corrected (7 to the pinned upstream
   default, 3 more in enum domain), 4 rows fabricated keys the viewer's
   `GCodeProcessor` consumes (contract-forbidden; swapped for neutral keys), 3 rows
   named keys OrcaSlicer cannot load
   (`support_material`, `extra_perimeters` removed; `wipe_tower_width` retained as
   a `handle_legacy` rename), and 4 rows named keys absent from OrcaSlicer entirely
   and were replaced with real cosmetic keys. A freshly generated production block
   now shows **0 strict wire violations** against the pinned source. No timing,
   geometry, adoption or default change; no guest touched; both freshness gates and
   all required code gates pass. The residual — the padding-only fallback block
   still cannot satisfy Orca's `>= 80` loadable-pair gate on its own because the
   remainder are PnP's own key names, which no table edit can fix — is recorded on
   the ticket rather than silently patched.
   **Next route (2026-10-08):** with the small non-timing family complete, the
   human reopened [offset2_ex call reduction](issues/19-offset2-ex-call-reduction.md) —
   its below-fold park is lifted for that candidate's take; its blockers 12/17
   are resolved; acceptance is the standing paired ordinary + accelerated A/B
   (timing chain position 4, keep/drop to the human, no auto-commit).
   **Take complete (2026-10-08):** the reopen's premise was re-derived at HEAD
   and **is stale** — the ticket's 720-call `min_width_top` headline site does
   not execute on the matched classic job (0 calls; 13 layers / 0.037% of module
   fuel with its gate forced on), and the two other named shapes are dead or not
   output-preserving. The surviving candidate is a shape the ticket did not
   name: the `classify_narrow_island` opening-emptiness short-circuit (two-pass
   `opening_ex` → shared single-erosion predicate; site calls 720 → 480, module
   fuel 750.03 B → 732.43 B ≈ −2.35% at the probe, output byte-identical). The
   standing paired A/B **passes both modes** — ordinary −0.65% median wall /
   −2.08% median CPU, accelerated −1.47% / −1.04%, six retained pairs per mode,
   no starvation exclusions, all 32 outputs byte-identical to `34886ccb…`.
   Measured **KEEP approved by the human and committed with this ticket
   (2026-10-08)**; both freshness gates are green. A
   provenance constraint was measured: any text change to `polygon_ops.rs`
   shifts the accelerated guest's artifact hash (name-section `.llvm.<hash>`
   suffixes) even for a comment, so the shipped source keeps the measured bytes
   and takes a manifest-level lint allow instead of rewriting the expression.
   See [findings](evidence/t19-offset2-call-graph/RESULTS.md) and the ticket's
   Answer for the full evidence set.
   **Second take (2026-10-08, routing claim + premise audit):** the session took
   the first numbered open child, [consume-only-when-read
   annotations](issues/20-consume-only-when-read-annotations.md), as a routing
   claim only — t19's lift was candidate-scoped and the below-fold park in
   [Gap budget per cell](issues/12-gap-budget-per-cell.md) still binds, so no
   implementation, experiment or timing started. The claim resolved into a
   read-only premise audit whose verdict **keeps the premise and sharpens it**:
   the only production site making a decision on the two per-vertex stamps is
   `OverhangClassifierDefault::run_finalization`
   (`modules/core-modules/overhang-classifier-default/src/lib.rs`), and it
   early-returns — twice — before reading any stamp whenever
   `enable_overhang_speed` is false or all four `overhang_1_4_speed` …
   `overhang_4_4_speed` bands are 0; none of the four matched configs authors
   those keys (manifest defaults `true` / `0.0`), so **on every matched cell the
   dominant measured query cost is stamped and never read**. The surviving
   candidate shape is therefore a conditional stamp gated on the consumer's own
   liveness predicate (value-identical when live) — a fuel→wall question the
   standing paired A/B still decides. Route returned to the human: t20 take vs
   [serial host floor](issues/27-serial-host-prepass-floor.md). See the ticket's
   Comments for the full consumer/gate audit.
   **Closure (2026-10-09):** the human-selected
   [serial host floor](issues/27-serial-host-prepass-floor.md) take completed —
   probe-3 attributed 99.8% of `commit_support_analysis_builtin`'s serial sweep
   to one predicate (`SliceRegionView::derive_needs_support`'s per-region
   `intersection_ex` against the object-global overhang footprint: 24.19 s of
   the 38.20 s base supports-on stage wall) and moved it to an order-preserving
   indexed rayon pre-pass (dual-compute 0 mismatches on 495 base + 240 benchy
   regions; 8.7x on the predicate). The standing four-cell paired A/B measured
   base median wall −18.198 s ordinary / −19.380 s accelerated at ≈+0.5% median
   process CPU (benchy −1.105 s / −0.943 s, the only cell passing the strict
   wall-and-CPU clause; base was accepted as a serialization fix), all 48
   outputs byte-identical, and the **human KEPT the candidate 2026-10-09**;
   committed with the ticket. The ticket resolves with this keep; the bbox
   filter was refuted and the `from_ir` clone hypothesis measured dead.
   Frontier: [consume-only-when-read annotations](issues/20-consume-only-when-read-annotations.md)
   is now the only open child — premise audited, unblocked — awaiting the
   human's route decision; the below-fold park in
   [Gap budget per cell](issues/12-gap-budget-per-cell.md) is unchanged, and
   the structural route it conditions on is now fully resolved (24/22/27/28 +
   the re-scoped 18), with no cell inside its ~1.5x release bound. The
   remaining serial-floor leads stay in the fog above. This closure session
   started no candidate, experiment or timing run.
   **Refresh (2026-10-09):** a re-run attribution session executed the fresh
   classic-cell pass (four matched classic cells at HEAD, profiled +
   instrumented; the 2026-10-09 t27 A/B base captures re-reduced; evidence in
   `evidence/perf-refresh-2026-10-09/`, `verify.py` exit 0). Headline finding:
   `PostPass::LayerFinalization` on base classic supports-on measures
   100.2–121.3 s in **all 24** probe-free 2026-10-09 A/B runs — never
   attributed — and decomposes to the four finalization modules' per-module
   whole-print `layers.clone()` + drop (stage 110.1 s = module brackets 62.7 s
   + gaps 47.4 s; module guest fuel ≤ 1.4 B of 13,090 B) plus a further pre-emit
   `to_vec`+reconcile copy (≈11.1 s between stage brackets); base supports-off
   is the 2.0 s control. Also refreshed: `host:slice`'s `closing_ex` remains the
   largest host scope on base (384–389 s accumulated; the flat-bridge enclosure
   in `assemble_flat_bridge_areas`), `host:native` carries 197.2 s accumulated
   on base supports-off with no named owner, and guest fuel remains
   `classic-perimeters` self (the ticket-20 target). **Three new tickets were
   brought to the table for the human's selection — none parked, none claimed:
   [58](issues/58-layerfinalization-whole-print-copying.md),
   [59](issues/59-prep-slice-flat-bridge-closing-reduction.md),
   [60](issues/60-host-native-attribution.md).** The below-fold park is
   unchanged; no candidate, experiment or timing campaign ran in the refresh.
   **Take completed (2026-10-10 → 2026-10-12, ticket 58):** the human selected
   [58](issues/58-layerfinalization-whole-print-copying.md); the take
   implemented the clone-once finalization stage + owned pre-emit capture
   candidate (three runtime files, uncommitted), qualified a fresh 16-cell
   output-identity matrix (ordinary cells reproduced the preserved production
   references; base-arachne-on fresh ref `a1bc8110…`), and **PASSED the
   standing paired gate in all four pre-declared cells** with 6/6 retained
   pairs and zero starvation exclusions: base-classic-on wall medians
   **610.0 → 539.5 s (−10.48 %) ordinary and 601.0 → 547.9 s (−8.76 %)
   accelerated**, CPU corroborating (−2.33 % / −2.45 %); benchy-classic-on
   −1.21 % / −1.44 %. Attribution (instrumented single runs,
   corroboration-only): the LayerFinalization stage dropped
   **113.2 → 70.4 s (−37.7 %)**, mechanism exactly as the ticket hypothesized —
   the three inter-bracket commit-drop gaps (8.0/13.9/13.1 s) became **0 ms**
   (one 2.7 s upfront clone, one 10.1 s stage-commit tail); the postpass phase
   went 132.3 → 87.4 s. The campaign ran under disclosed background load
   (Opera churn; one other-checkout test binary for a window), survived three
   interruptions (two opencode server restarts → partial cells deleted and
   re-run per the t57 recovery discipline; one harness 600 s-timeout
   self-stop → per-run timeout raised to 1800 s, infrastructure-only and
   disclosed), and closed with the standing snapshots restored and both
   freshness gates green. **Recommendation to the human: KEEP — human KEPT
   it 2026-10-12; no auto-commit was made before the ruling — the
   candidate and evidence are committed (code `e7159687`, docs
   `e4e93dea`).**
   Evidence: `evidence/t58-finalization-copies/FINDINGS.md` (+
   `qualification.json`, `ab-reduction.json`).
   **Tracker catch-up (2026-10-12, same session):** the previously
   uncommitted t29/t30 source changes and the t29/t31/t45/t49-t57 tracker
   and evidence sets are now committed (byte-pinned in `.gitattributes`;
   every checkout-only evidence verifier exits 0); t58's tracker text was
   reconciled to the committed KEEP. The frontier is unchanged and still
   awaits the human's selection:
   [20](issues/20-consume-only-when-read-annotations.md) (audited, parked
   by the below-fold park),
   [59](issues/59-prep-slice-flat-bridge-closing-reduction.md),
   [60](issues/60-host-native-attribution.md).
- **Production-mode route (2026-09-30):** pursue conditional accelerated
  adoption; ordinary stays the actual default pending the existing acceptance
  gate. See [Accelerated perimeter-spatial adoption decision](issues/13-accelerated-adoption-decision.md)
   and its [acceptance campaign](issues/38-accelerated-adoption-acceptance-campaign.md).
  **Adoption preflight (2026-09-30):** both mode-specific guest freshness checks
  passed, but the real acceptance CSV reader loses non-fatal counts and the
  existing references predate correctness repairs. Campaign timing has not
  started; its prerequisite blocking edges carry the new frontier. Evidence:
   [Accelerated adoption campaign preflight](evidence/t38-adoption-preflight/PREFLIGHT.md).
   **Runner prerequisite repaired:** see
   [Acceptance runner CSV status preservation](issues/39-acceptance-runner-status-roundtrip.md).
   **References now prepared/frozen:** see
   [Current-job adoption reference preparation policy](issues/40-acceptance-reference-refresh-policy.md).
   The historical corpus remains intact; use the frozen manifest's new corpus
   root explicitly after its identity gate. No campaign timing has started;
    full adoption gates remain pending.
    **Resumption blocker (2026-09-30):** the manifest-named frozen ordinary
    snapshot and current-job corpus roots are now absent; the read-only identity
    gate failed before comparison or timing. Historical corpus, manifest and
    build-input hashes still match. The campaign is blocked on
    [Missing frozen adoption inputs recovery decision](issues/41-missing-frozen-adoption-inputs-recovery.md);
    do not silently recreate the missing payloads. Evidence:
    [Frozen-input difference audit](evidence/t38-adoption-campaign/freeze-difference.json).
    **Recovery authorized:** the human chose a new, separate ordinary-only
    preparation with the unchanged six-cell job and checks. Durable payloads
    will live under `.local-artifacts/perimeter-reference-preparation/`, outside
    `target/`; previous evidence and the historical corpus remain preserved.
    **Replacement technically prepared/frozen:** all six ordinary references
    passed the unchanged checks and main-session identity verification outside
    `target/`. Evidence:
    [Durable preparation findings](evidence/t41-reference-repreparation/FINDINGS.md).
    **Recovery accepted:** the human accepted the validated durable set and
    the recorded preparation-only preflight restart exception, then requested
    continuation. The
    [acceptance campaign](issues/38-accelerated-adoption-acceptance-campaign.md)
     resumes under the unchanged gates; no automatic retry, commit or default
     switch is authorized.
     **Campaign completed (2026-10-01): inconclusive.** Classic supports-off
     Benchy passed; Arachne wall ranges overlapped and the prescribed stop left
     four cells unrun. No retry or default switch occurred. The human route is
      [Accelerated adoption route after inconclusive campaign](issues/42-accelerated-adoption-inconclusive-route.md).
      **Investigation selected:** saved Arachne evidence localizes the finishing
       tail to layers 1 and 2. The subsequently authorized two-slice diagnostic
       pair identified infill-linker dispatch as the owner, not Arachne perimeter
       dispatch. No acceptance retry or fix ran. Next lead: low-layer linker
       subcost attribution, requiring separate authorization. See
       [Arachne critical-tail module attribution](issues/43-arachne-critical-tail-module-attribution.md).
      **Next lead taken and answered (2026-10-01):** the human authorized
       [infill-linker attribution](issues/21-infill-linker-attribution.md) as
       the next map step. Result: the tail is ~99.95% guest execution, and the
       linker's cost is 97.65% the **per-path re-clip** — 64.84% of all linker
       fuel is the repeated clip-universe pre-inflate, 32.65% the clipper
       execute; connectivity is 2.01%. The split is numerically mode-invariant
       (linker is the #1 accelerated fuel consumer at 59.1%). One candidate:
       hoist the clip-universe preparation out of the per-path loop (fuel
       ceiling ≈ 61.4% of linker ≈ 28.1% of slice fuel as a uniform-count
       estimate; 63.7% of linker layer-aware; both estimates under a same-cost
       assumption), needing its own scope plus the standing paired A/B. No
       fix, commit or acceptance retry occurred; ordinary remains production
       and the adoption gate stays inconclusive. Evidence:
       [linker subcost findings](evidence/t21-linker-subcost/FINDINGS.md).
      **Next chain link taken and answered (2026-10-01):**
       [Arachne graph-construction attribution](issues/25-arachne-graph-attribution.md)
       resolved with an attribution that **reverses the old study's focus
       boundary**: the module's cost is 86.4% two guest per-vertex queries
       (`signed_distance_to_boundary` 48.3%, `overhang_quartile` 38.1%), while
       the whole host pipeline is ~18% of module elapsed; the host internals
       still split into the old two leads (triple offset 41.6%, boostvoronoi
       sweep 33.5%) and the 364.932 ms remainder is closed. One candidate (the
       quartile predicate's `eps = 0.0` boundary pre-pass, fuel ceiling ≈
       21–27% of module fuel as a cross-domain bound) awaits its own scope plus
       the paired A/B. No fix, commit or acceptance retry; ordinary remains
       production. Evidence:
       [Arachne re-attribution findings](evidence/t25-arachne-attribution/FINDINGS.md).
       **Oracle qualification attempt (2026-10-02):** the
       [Integrated/external matched-output oracle gate](issues/33-integrated-external-matched-output-oracle-gate.md)
       FAILED — integrated dispatch output-diverges (native
       `Layer::InfillPostProcess` views miss the partitioned-`SliceIR`
       enrichment the WASM leg performs; the linker falls back to the union
       boundary and prints paths the external leg clips away), reproducing
       perf-split Finding 3's TYPE deltas on a fresh tree. Integrated timings
       stay disqualified and the integrated edition gains no timing evidence.
       The enabling `raft-default` integration-row repair (packet-240b
       registry drift; `xtask dist --edition integrated` was unbuildable) is
       committed (`4a8330a6`); the named dispatch fix was resolved, verified
       and KEEP-approved under
       [Native postprocess-view enrichment repair](issues/47-native-postprocess-view-enrichment.md)
       (2026-10-02 → 2026-10-03; supersedes this FAILED verdict — the gate now
       PASSES from proof, so integrated timings are usable as an oracle on the
       examined cell).

## Decisions so far

- [Exact `ShellClassification` acceleration: windowed `unsupported_span_areas`, lazy/clustered internal-bridge commit, parallel Pass-2 merge/diff, and $O((C+E)\log(C+E))$ external-bridge scoring](issues/184-shell-classification-exact-acceleration.md): **resolved 2026-11-23, measured candidate — literal standing all-8-cell `0.80 %` gate PASSED 8/8 on the pre-registered pooled verdict (10 pairs + 6 null pairs/cell; `+4.99…+16.35 %` ord wall, 78/80 wins; CPU lower 8/8 `+4.85…+10.68 %`; acc `+7.76…+18.65 %`), `448/448` runs byte-identical to `t182/reference.json`, zero reference change** — exact host-only bundle: parallel Pass-2 merge + `A \ A` skip, qualify-first internal-bridge gate, dependency clusters + validated speculative commit (0 misses), hoisted anchors + BVH/windowed `determine_bridging_angle`, polygon/hole-filtered `unsupported_span_areas_near`, filtered prefix-sum + exact re-score external-bridge selector. Stage `−55…−57 %` benchy / `−33…−37 %` base. Disclosed: window 1 7/8 (noisy null), strict any-pair 7/8, guest recompilation fuel drift `−0.70…−1.36 %` with a host-only attribution batch. Route (`ESTIMATE ONLY`): benchy supports-off `0.877x / 0.885x` ord. Evidence: [`evidence/t184-shell-classification-exact-accel/FINDINGS.md`](evidence/t184-shell-classification-exact-accel/FINDINGS.md) (`verify.py` PASS, `--self-test` 16/16).
<!-- one line per closed ticket; the ticket holds the detail -->
- [Exact per-layer guest acceleration: `tree-support` renderer fast paths and `infill-linker` open-subject tagging](issues/183-per-layer-tree-support-and-infill-linker-exact-acceleration.md): **resolved 2026-11-23, measured candidate — literal standing all-8-cell `0.80 %` gate PASSED (`8/8 PASS`: `+2.20…+9.59 %` ord wall, `39/40` wins, `40/40` CPU wins `+3.10…+16.54 %`, acc `+1.85…+7.97 %` wall / `+3.53…+15.92 %` CPU), `224/224` measured runs 100 % byte-identical to `t182/reference.json` (zero reference change, zero geometry exceptions, zero waivers), crossing `< 1.0x` on both ordinary `base` supports-on cells (`~0.973x / ~0.966x` est.)** — landed `TS-1a` (`crates/slicer-core/src/paint_policy.rs::has_support_paint` + `crates/slicer-sdk/src/traits.rs::PaintRegionLayerView::paint_policy_for`), `TS-1b`/`TS-2` (`modules/core-modules/tree-support/src/lib.rs::skeleton_wall_count` precomputed AABBs + `count <= best` skip; `unify_role_regions` / `expolygons_pairwise_bbox_disjoint` safe bypass), `IL-1` (`vendor/clipper2-rust` `LocalMinima`/`OutRec` `source_id: u32`, `Clipper64::add_open_subject_tagged` / `execute_open_tagged`, wired into `crates/slicer-core/src/polygon_ops.rs::PreparedPolylineClip::clip_batched`, deleting 249 lines of geometric re-attribution), `IL-2` (`modules/core-modules/infill-linker/src/orchestrate.rs` dead `source_segments` skip + `O(1)` `uniform_point_metadata`), and `IL-3` (`modules/core-modules/infill-linker/src/graph.rs` & `connect.rs` precomputed `BoundaryEdge`/AABBs, cross-ring `project_bounded`, hole AABB guard, `O(E)` circular `nearest_pair_candidates`). Deterministic `--profile` guest fuel (`16/16` byte-identical): `tree-support` **−96.42 % (`27.95×`)** `benchy` / **−97.06 % (`33.99×`)** `base`, `infill-linker` **−74.03 / −74.56 / −79.34 / −79.05 %** (`3.85×–4.84×`), total slice fuel **−23.92…−40.17 %** off / **−69.73…−73.83 %** on. Standing 8-cell paired gate (unsandboxed, `RAYON_NUM_THREADS=12`, load `0–9 %`, `0` degraded, `0` code-1200): `benchy-classic-off` **+9.59 %** (`5/5`, `11.99×`, CPU **+6.24 %**, acc **+7.23/+3.87 %**), `benchy-classic-on` **+5.60 %** (`4/5`, `7.00×`, CPU **+16.54 %**, acc **+7.54/+15.92 %**), `benchy-arachne-off` **+8.36 %** (`5/5`, `10.45×`, CPU **+8.45 %**, acc **+7.09/+7.06 %**), `benchy-arachne-on` **+5.73 %** (`5/5`, `7.16×`, CPU **+14.60 %**, acc **+6.77/+15.47 %**), `base-classic-off` **+6.58 %** (`5/5`, `8.23×`, CPU **+3.10 %**, acc **+2.39/+3.59 %**), `base-classic-on` **+7.43 %** (`5/5`, `9.29×`, CPU **+15.40 %**, acc **+7.27/+15.06 %**), `base-arachne-off` **+2.20 %** (`5/5`, `2.75×`, `4.47×` null median; strict any-pair `INSIDE-NULL-ANY-PAIR` disclosed, CPU **+3.34 %**, acc **+1.85/+3.53 %**), `base-arachne-on` **+6.46 %** (`5/5`, `8.07×`, CPU **+14.24 %**, acc **+7.97/+14.66 %**). Cross-window `ESTIMATE ONLY` route (`t174 × t175 × t182 × t183`): `base` **4/4 `< 1.0x`** (`0.718 / 0.973 / 0.775 / 0.966x` ord, `0.747 / 0.926 / 0.760 / 0.917x` acc), `benchy` `1.034 / 1.454 / 1.058 / 1.277x` ord (`1.037 / 1.413 / 0.989 / 1.264x` acc). Evidence: [`evidence/t183-per-layer-guest-exact-accel/FINDINGS.md`](evidence/t183-per-layer-guest-exact-accel/FINDINGS.md) (`verify.py` PASS, `--self-test` `16/16`).
- [Host-side spatial hierarchical Phase-3 union with convex-containment pruning](issues/182-host-spatial-hierarchical-phase3-union.md): **resolved 2026-11-22, measured candidate — literal standing `t137` 4-cell planner gate PASSED (4/4 supports-on PASS: +13.85 / +14.00 % base, +1.90 / +2.41 % benchy, 20/20 ord wall & CPU wins, acc +14.64 / +13.57 / +4.58 / +3.81 %; 4/4 supports-off INERT-PASS), landed (`worker-k8`) by explicit human KEEP ruling advancing `reference.json` for the 4 moved supports-on hashes (zero `Support interface` or part-geometry movement; 4/4 supports-off byte-identical to `t175`)** — host-only `slicer_core::polygon_ops::union_spatial_tree` (`crates/slicer-core/src/polygon_ops_spatial_union.rs`, `DEFAULT_SPATIAL_UNION_CHUNK_SIZE = 8`, `DEFAULT_SPATIAL_UNION_NESTED_PAR = false`: exact `i128` canonical-winding guard with monolithic fallback, $O(V)$ `is_convex_ccw` + exact `i128` $O(\log V)$ / inscribed-disk convex-containment pruning, deterministic X-sweep bbox CC bypassing Clipper2 on canonical singletons, 2D Morton Z-order sort, strictly-terminating chunked hierarchical union), wired strictly into batched `ClipOperation::Union` when `subject.len() + clip.len() > 8` (`crates/slicer-wasm-host/src/host.rs::clip_polygons_batch` and native fallback `crates/slicer-sdk/src/host_batch.rs::clip_polygons_batch`; zero WIT/guest source edits). Sweep on `base-classic-on` (`{worker, nested} × k ∈ {4, 8, 16}`, load `4 → 5 %`): `worker-kN ≡ nested-kN` byte-identical across all `k`; `worker-k8` (`24.262 s`, `+15.30 %` wall / `+23.60 %` CPU, tight spread `0.337 s`) sits within `0.23 %` of `nested-k4` while cutting the `Support` path delta 4.9× (`−0.242 mm` vs `−1.195 mm`). Standing `t137` gate (5 pairs/cell ord, 3-pair null, 3-pair acc; load `0–9 %`; `229/229` clean runs, `0` degraded, `0` code-1200): `base-classic-on` **+13.85 %** (`5/5`, `17.31×` floor, null `0.30 / 0.44 %`, CPU **+23.07 %**, acc **+14.64 / +24.20 %**), `base-arachne-on` **+14.00 %** (`5/5`, `17.51×`, null `0.55 / 1.71 %`, CPU **+22.62 %**, acc **+13.57 / +23.39 %**), `benchy-classic-on` **+1.90 %** (`5/5`, `2.37×`, null `0.51 / 4.80 %`, CPU **+7.25 %** = `7.1×` null any-pair CPU, acc **+4.58 / +4.79 %**), `benchy-arachne-on` **+2.41 %** (`5/5`, `3.02×`, null `1.25 / 3.65 %`, CPU **+4.70 %**, acc **+3.81 / +3.73 %**; strict any-pair `INSIDE-NULL-ANY-PAIR` on both benchy cells transparently disclosed); supports-off `−1.07…+1.64 %` ord, 4/4 byte-identical to `t175`. Output & 8-cell G-code audit: baseline reproduces `t175` **16/16**, candidate ord ≡ acc **16/16**, **0 / 1,226,364** non-support G1 motion lines moved XY (max `|dE|` ≤ `0.00048 mm`), zero `Support interface` movement, `Support` only `−0.01777 mm` (`−0.000022 %`, `+3 B / 0` lines) `benchy` and `−0.24238 mm` (`−0.000062 %`, `−56 B / −1` line) `base`. Cross-window `ESTIMATE ONLY` route (`t174 × t175 × t182`): `base-classic-on` `1.221x → ~1.052x` ord / **`~0.998x` acc** (`76.6 %` cut closed; `+t185` `~0.970x`), `base-arachne-on` `1.201x → ~1.033x` ord / **`~0.997x` acc** (`83.6 %` closed; `+t185` `~0.961x`), `benchy-classic-on` (binding) `1.570x → ~1.540x` ord / `~1.528x` acc, `benchy-arachne-on` `1.389x → ~1.355x` ord / `~1.355x` acc. Active currency: [`evidence/t182-spatial-hierarchical-phase3-union/reference.json`](evidence/t182-spatial-hierarchical-phase3-union/reference.json). Evidence: [`evidence/t182-spatial-hierarchical-phase3-union/FINDINGS.md`](evidence/t182-spatial-hierarchical-phase3-union/FINDINGS.md) (`verify.py` PASS, `--self-test` `15/15`; `audit_t182.py --self-test` `7/7`).
- [Per-layer `tree-support` renderer and `infill-linker` guest-fuel screen](issues/179-per-layer-tree-support-and-infill-linker-screen.md): **resolved 2026-11-21, attribution + exact fuel ablation + unsandboxed screen (zero production edits; `probe.patch` cleanly reverted; both guest freshness gates exit `0`; `130/130` runs byte-identical to `t175`, `16/16` exact integer fuel folds, `verify.py --self-test` `14/14`) — premise correction (`t66` was DROPPED in Oct 2026, never landed): `tree-support` is `98.1–98.5 %` reducible in exact fuel (`80.1–80.3 %` dead `paint_policy_for` walking `723.5 M / 6.76 B` ring edges across `0` paint annotations + `15.6–16.3 %` `skeleton_wall_count` where `99.1–99.2 %` of points have `count <= best` + `1.5–2.8 %` redundant in-guest `role_union`) and `infill-linker` is `73.0–78.6 %` reducible (`49.6–52.9 %` `clip_batched` geometric re-attribution + `1,878–4,002` ambiguity repairs, `70.8–71.3 %` of `benchy` `L2`; `8.5–21.7 %` dead `source_segments` / `3.2–30.3 M` uniform metadata scans; `10.8–16.2 %` `project_on_ring` / `O(E²)` pair scan); combining `TS-2 + IL-ALL` avoids both standalone gate traps (`L2` straggler unmasking `50 → 110 ms` under standalone `TS-2`; `0.52 % < 0.80 %` `base-classic-on` cap under standalone `IL-ALL`) and clears the literal 8-cell `0.80 %` screen floor at `2.64×–12.82×` (`+2.11…+10.26 %` wall, `24/24` wins; `+3.33…+13.75 %` CPU, `24/24` wins, `8/8` strict sep), graduating [Exact per-layer guest acceleration: `tree-support` renderer fast paths and `infill-linker` open-subject tagging](issues/183-per-layer-tree-support-and-infill-linker-exact-acceleration.md)** — `TS-2` cuts `tree-support` fuel `41.94 → 0.65 B` (`−98.45 %`, `64.7×`) on `benchy-classic-on` and `388.79 → 7.56 B` (`−98.06 %`, `51.4×`) on `base-classic-on` (safe bypass requires `regularized.is_some()` or pairwise bbox disjointness; surviving wall is `2,001 / 6,345` `host:native` `offset_polygons` calls); `IL-ALL` (`vendor/clipper2-rust` `add_open_subject_tagged` / `execute_open_tagged` + `IL-2` + `IL-3`) cuts `infill-linker` fuel `−72.96 / −73.47 %` (`benchy`) and `−78.60 / −78.28 %` (`base`), collapsing `L2` `infill-linker` `186 → 42 ms` (`classic-off`) and `237 → 84 ms` (`arachne-off`) and eliminating the `L2` tail on 3/4 benchy cells (`5–6 ms`; `31 ms` residual on `benchy-classic-off` where `~159 ms` of `L2` is 3× host `region_partition`, so no standalone `with_max_len(1)` ticket graduates). `ESTIMATE ONLY` route (`t174 × t175 × t179`): `benchy-classic-off` `1.144x → ~1.026x` ord / `~1.003x` acc (`81.6 %` of gap closed), `benchy-arachne-off` `1.155x → ~1.093x` / `~1.007x`, `base` supports-on `1.221x / 1.201x → ~1.145x / ~1.122x` (**`~0.970x / ~0.953x` stacked with `t182`, `~0.875x / ~0.869x` with `t182 + t181`**), `benchy-classic-on` `1.570x → ~1.498x` (`1.404x` `t177`-implied), `benchy-arachne-on` `1.389x → ~1.295x`. Evidence: [`evidence/t179-per-layer-guest-screen/FINDINGS.md`](evidence/t179-per-layer-guest-screen/FINDINGS.md).
- [Planner Phase-3 batched union attribution and mechanism screen](issues/178-planner-phase3-union-attribution.md): **resolved 2026-11-21, attribution + 29-mechanism offline screen (zero production edits; `probe.patch` cleanly reverted, 12/12 capture runs byte-identical to `t175`, `verify.py --self-test` 14/14) — Phase 3 is 99.99 % `ROLE_BODY` monolithic Clipper2 sweeps over 1,024-circle overlapping clusters (`7.64×` area overlap, `28.1 %` strictly convex-contained, `88.5 %` in giant components); Mechanisms A/B/C and `subject=all` are falsified as sufficient levers, while host-side fast convex-containment pruning + bbox CC + Morton Z-order spatial hierarchical union (`M9n_host_nested_prune_hier_k8`) cuts Phase-3 wall −91.4 % / thread −92.4 % on `base` (`0.949 µm` Hausdorff) and −69.3 % wall on `benchy` (`0.400 µm` Hausdorff), graduating [Host-side spatial hierarchical Phase-3 union with convex-containment pruning](issues/182-host-spatial-hierarchical-phase3-union.md)** — clean uninstrumented Phase-3 wall is `4,434.3 / 4,404.6 ms` (`15.68 / 15.39 %` of `base` supports-on, `45.52 s` thread, `10.27×` par, `sum(batch_max) = 99.1 %`, WIT/IR conv `0.23 %`) and `154.5 / 153.8 ms` (`4.48 / 4.32 %` `benchy`, `1.55 s` thread, `10.03×`); classic ≡ arachne on operands. Exact baseline replay passes `480/480` `base` + `202/202` `benchy` (`0` mismatches). Singletons are only `0.39 %` (`base`, 530/530 canonical/simple) and `0.29 %` (`benchy`, 63/63) while `88.5 %` of `base` vertices sit in giant components (`max = 1,023` polys), falsifying standalone `M2`; `avg_nodes = 8 / 5` (`simplify_active_calls = 0`) and forced pre-union RDP (`M6`) is catastrophically lossy (`870–3,902 mm²` XOR); LPT/global batching (`M5a/b`) hits the `3.79 s` floor (`0 %` CPU cut); `M1` (`subject=all`) is speed-neutral; exact duplicates = `0`. Dummy-far-Y auditing proved `147/147` `base` + `27/27` `benchy` (100.0 %) of `M2`'s diffs are `add_new_intersect_node` distant-scanbeam clamping (`0.162 / 0.130 µm` Hausdorff). Because `tree-support-planner` is the sole caller of `clip_polygons_batch` in the repo, host-side `union_spatial_tree` inside `clip_polygons_batch` `Union` needs zero WIT/guest changes and is 100 % supports-off inert. `ESTIMATE ONLY` route (`t174 × t175 × t178`): `M9n_k8` clears `t137` by `3.7×–17.9×` (`−4.05 s` / `−14.33 %` `base-classic-on` → `1.046x` ord / `1.002x` acc; `−4.03 s` / `−14.06 %` `base-arachne-on` → `1.032x` ord / `0.991x` acc; `−107 ms` / `−3.10 / −2.99 %` `benchy`), closing `79.3 / 84.0 %` of `base`'s ordinary gap and crossing `< 1.0x` on both ordinary `base` cells (`~0.95x`) when stacked with [`SupportGeometry` post-planner host tail attribution and reduction screen](issues/181-support-geometry-post-planner-host-tail.md). Evidence: [`evidence/t178-planner-phase3-union/FINDINGS.md`](evidence/t178-planner-phase3-union/FINDINGS.md).
- [Post-t175 full-matrix attribution and 0.50x headroom map](issues/177-post-t175-full-matrix-attribution.md): **resolved 2026-11-21, attribution + headroom pricing (zero production edits; 32/32 runs byte-identical to `t175`, `verify.py --self-test` 12/12) — all 8 cells are now `clip_polygons`-dominated, 5/6 open cells have a single-stage `< 1.0x` ceiling (only binding `benchy-classic-on` ~1.570x requires a 3–4 track combination), NO `benchy` cell can hit `<= 0.50x` from prepass alone, unblocks `179` & `180`, and graduates `181`** — prepass fell −2.4…−7.5 pp vs `t171` to **53.5–77.7 %**, per-layer **14.3–34.3 %**, postpass **6.8–11.8 %** (`11.8 %` on `benchy-on`), outside **0.3–4.1 %** (3 unsandboxed passes/cell; discarded sandboxed first capture that inflated `per-layer` ~2.8× on `benchy`). `--profile` host Clipper thread-time dropped −36.8…−40.3 % off (`base-classic-off` `88.85 → 53.40 s`) and −58.4…−59.7 % on (`base-classic-on` `343.38 → 140.34 s`), flipping **8/8** cells to `clip_polygons`-dominant (**46.5–58.6 %** vs `offset` **30.8–37.8 %**). `ESTIMATE ONLY` route (`t174 × t175`): `base-on` (`~1.221x / ~1.201x`) closes via `SupportGeometry` (**44.4 / 43.4 %** → 0.678x / 0.679x); `benchy-off` (`1.144x / 1.155x`) is now single-stage winnable via `ShellClassification` (**27.9 / 27.0 %** → 0.825x / 0.843x, only **4.5×** concurrency on `benchy` vs **8.1–8.6×** on `base`) or `per-layer` (**32.4 / 34.3 %**); `benchy-arachne-on` (`~1.389x`) needs multi-track (`SupportGeometry + ShellClassification + tree-support` = 53.3 % → 0.648x); binding `benchy-classic-on` (`~1.570x`, 36.3 % cut) cannot close from any single stage (`SupportGeometry` 31.7 % → 1.071x) or from `ShellClassification + per-layer` (33.1 % → 1.051x) and needs `SupportGeometry + ShellClassification + tree-support (+ SupportAnalysis)`. `<= 0.50x` theorem: zeroing all of `prepass` leaves `benchy` at **0.501x–0.537x** (all 4 short), so the follow-on 2× map structurally requires both prepass and per-layer cuts. Two new bottlenecks: (1) **graduated [`SupportGeometry` post-planner host tail attribution and reduction screen](issues/181-support-geometry-post-planner-host-tail.md)** (`2,293 / 2,017 ms` = **7.8 / 7.0 %** `base-on`, `215 / 216 ms` = **6.2 / 6.1 %** `benchy-on` elapsing *after* `tree-support-planner::module_complete`); (2) **sharpened [Per-layer `tree-support` renderer and `infill-linker` guest-fuel screen](issues/179-per-layer-tree-support-and-infill-linker-screen.md)** with the **`L2` `infill-linker` single-layer straggler tail** (`134 / 104 ms` = **7.9 / 5.8 %** cell with 11/12 cores idle on `benchy-off`) plus the supports-off fuel-vs-wall inversion (`perimeters` #1 in elapsed thread-time ahead of `infill-linker`). Evidence: [`evidence/t177-post-t175-attribution/FINDINGS.md`](evidence/t177-post-t175-attribution/FINDINGS.md).
- [Destination ruling: CPU-corroboration semantics and sample count for cell closure](issues/176-destination-cpu-corroboration-ruling.md): **resolved 2026-11-21, human ruling (zero production edits) — Option C amends `## Destination` to unstarved wall-clock wins + CPU integrity guard + separate efficiency disclosure (`pnp-ordinary` binding arm, margin-tiered provisional closure + final single-HEAD `n>=4` 8/8 board), moving the official running tally `0/8 → 2/8` provisionally won (`base-classic-off` 0.769x, `base-arachne-off` 0.792x), keeping `< 1.0x` (8/8) with a fresh `<= 0.50x` effort afterward, and graduating `177`, `178`, `179`, `180`** — separates cross-engine **CPU integrity guard** (tight CPU, unstarved `cpu/wall`, matched non-degraded output, plus PnP/Orca CPU ratio disclosed as efficiency — accounting for Orca's ~10.0 s serial STL import vs PnP's ~5.2x-core slicing) from unchanged within-PnP **candidate A/B CPU corroboration**. **`pnp-ordinary`** (production default) binds cell closure (`pnp-accelerated` disclosed alongside; swaps only if adopted as production default). Wide-margin cells (`ratio <= 0.85x`) close provisionally on quiet `n=3` strict separation with 0 starvation flags (`base-classic-off` 0.7687x / 3.66 s sep and `base-arachne-off` 0.7921x / 2.37 s sep from `t174`, structurally inert under `t175`); close cells (`0.85x < ratio < 1.00x`) and the final single-HEAD 8/8 closing board require `>= 4` retained unstarved runs per arm with strict separation (starved outliers `< 0.75 × best` excludable per `SCOREBOARD.md` while `>= 4` unstarved runs remain); provisional wins carry forward across later commits passing paired non-regression gates. Destination bar stays `< 1.0x` (8/8) with a follow-on `<= 0.50x` (2× faster) effort planned as a fresh map afterward, prioritizing high-leverage speedup tracks now. Added 4 canonical terms to [`GLOSSARY.md`](../../../GLOSSARY.md) (**CPU integrity guard**, **Candidate CPU corroboration**, **Binding arm**, **Provisional cell win**) and graduated [177](issues/177-post-t175-full-matrix-attribution.md) (unblocked), [178](issues/178-planner-phase3-union-attribution.md) (unblocked), [179](issues/179-per-layer-tree-support-and-infill-linker-screen.md) (blocked by `177`), and [180](issues/180-shell-classification-post-t172-residual-screen.md) (blocked by `177`).
- [Tiled collision operand for the planner's phase-1 carve](issues/175-tiled-collision-carve.md): **resolved 2026-11-21, measured candidate — standing `t137` 4-cell planner gate PASSED (4/4 supports-on PASS: +31.32 / +30.90 % base, +16.06 / +16.62 % benchy, 5/5 wins, CPU +25.36…+40.86 %, acc +15.84…+31.53 %; 4/4 supports-off INERT-PASS), landed by explicit human KEEP ruling updating only the 2 `base` supports-on hashes for 1 µm Clipper2 distant-scanbeam unclamping** — guest-only exact non-perturbing per-layer tile index (`modules/core-modules/tree-support-planner/src/collision_tile_index.rs::CollisionTileIndex`, wired into `lib.rs::build_roles_batched` Phase 1 only; zero host/WIT/schema edits). Sweep on `base-classic-on` (`1 / 2 / 5 / 10 mm`, 3 alternating rounds, load `1.43 → 5.2 %`): `baseline` 41.024 s, `1 mm` 28.336 s (+30.93 %), **`2 mm` 28.311 s (+30.99 %, chosen by lowest wall)**, `5 mm` 28.587 s (+30.32 %), `10 mm` 29.124 s (+29.01 %); all four tiled sizes share SHA-256 `7b90e749…`. Attribution (`--profile`, `probe.patch` reverted): on `base-classic-on`, `tile_index_build` **45.9 ms** (1.44 B fuel, 430 layers) + `tile_index_query` **84.6 ms** (1.55 B, 68,381 queries) = **130.5 ms** collapses `phase1_clip_batches` **12,436.6 → 174.4 ms (−98.6 %)**, sent requests **68,381 → 30,639** (37,742 empty-gated, 23,468 cache hits), Phase-1 clip points **426,868,700 → 3,629,342 (−99.15 %**, 0.85 % remaining vs t173's 1.62 % estimate), net planner fuel **62.27 → 48.58 B (−22.0 %)**, and `host:native` clip thread **173.90 → 56.02 s** (`benchy-classic-on`: 17.9 ms build+query, clip points `28.23 M → 251.5 k` = **−99.11 %**). Paired A/B (5 pairs/cell, load `1 → 3 %`, null 3 pairs `1 → 0 %`, acc 3 pairs `2 → 6 %`; 208/208 runs clean `exit=0`, `degraded=false`, `0` code-1200): `benchy-classic-on` **+16.06 %** (20.1× floor, null 2.02 %, CPU +26.75 %, acc +16.42 / +25.20 %), `benchy-arachne-on` **+16.62 %** (20.8×, null 2.29 %, CPU +25.36 %, acc +15.84 / +27.06 %), `base-classic-on` **+31.32 %** (39.2×, null 1.10 %, CPU +40.86 %, acc +31.39 / +40.97 %), `base-arachne-on` **+30.90 %** (38.6×, null 1.08 %, CPU +40.10 %, acc +31.53 / +40.39 %); supports-off −0.58…+0.74 % ord (−0.73…+0.55 % acc). Output: baseline reproduces t172 **16/16**, candidate ord ≡ acc **16/16**, **6/8 byte-identical** (all 4 supports-off + both benchy supports-on); only `base-classic-on` (−1 B, 7/976,781 lines, −0.00071 mm) and `base-arachne-on` (−1 B, 5/972,259 lines, −0.00071 mm) move by **1 µm** on 4 XY vertices strictly in `Support` / `Support interface` (0 part movement), exhaustively proven 9/9 caused by `vendor/clipper2-rust/src/engine.rs::add_new_intersect_node` clamping to unrelated distant vertices' global `scanline_list` Y coordinates (`8/9` bulk + 9th isolated `full_eq_win_with_dummy_far_y = true`). Cross-window route estimate vs t174 quiet board: `base-classic-on` **~1.221x / ~1.169x** (71.6 % of cut closed vs t173's 60.8 % ceiling), `base-arachne-on` **~1.201x / ~1.153x** (72.8 % vs 61.6 %), `benchy-arachne-on` **~1.389x / ~1.409x** (41.6 % vs 21.8 %), `benchy-classic-on` (binding) **~1.570x / ~1.602x** (34.5 % vs 23.2 %); Phase-3 union is now **~92.1 %** of remaining `host:native` clip thread on `base-classic-on` and remains unnamed in fog. Active currency: [`evidence/t175-tiled-collision-carve/reference.json`](evidence/t175-tiled-collision-carve/reference.json). Evidence: [`evidence/t175-tiled-collision-carve/FINDINGS.md`](evidence/t175-tiled-collision-carve/FINDINGS.md) (`verify.py` PASS, `--self-test` 13/13; `audit_t175.py --self-test` 6/6).
- [Post-t172 eight-cell two-tool scoreboard refresh](issues/174-post-t172-scoreboard-refresh.md): **resolved 2026-11-21, measured board (zero production edits) — official ratios of record are 0.769x–1.870x ordinary / 0.766x–1.916x accelerated (16/16 byte-identical to t172's currency), with 2/8 decisive unstarved wall-clock wins (`base classic-off` 0.769x / 0.766x, `base arachne-off` 0.792x / 0.774x, strict separation on all 3 runs) but 0/8 under literal `CPU <= Orca` (1.81x–2.13x CPU via ~5.2x vs ~1.9–2.2x concurrency); tally stays 0/8 pending HITL [176](issues/176-destination-cpu-corroboration-ruling.md), and `benchy classic-on` becomes the binding cell (1.870x / 46.54 % cut)** — stamped at HEAD `0ce4be39` (code `48a5e583`, both guest freshness gates exit 0; OrcaSlicer **2.5.1** `5013b1bd…` vendor `02.08.00.16` `tree_strong`, PnP ord `06b05b6b…` / `e75be5c0…`, PnP acc `490c6dc2…` / `481ff759…`). Quiet-box interleaved board (`t174-quiet-board`, 8 cells × 3 tools × 1 warmup + 3 measured = 96 rows, `RAYON_NUM_THREADS=12`, **0/72** starvation flags, 85 load samples 14/79/100 % CPU, steady 10.8–14.1 GB free RAM, **10/10** matched Orca keys on 32/32 Orca runs, **0** code-1200 diagnostics, `degraded: false`): benchy **1.1438x / 1.1179x** (cut 12.57 / 10.55 %), **1.8704x / 1.9162x** (cut **46.54 / 47.81 %**, binding), **1.1548x / 1.0640x** (cut 13.40 / 6.01 %; acc 0.15 s from parity), **1.6652x / 1.6743x** (cut 39.95 / 40.27 %); base **0.7687x / 0.7656x** (−23.13 / −23.44 % wall, strict sep 3.66 / 3.26 s, CPU 2.126x / 2.066x), **1.7773x / 1.7041x** (cut 43.73 / 41.32 %), **0.7921x / 0.7739x** (−20.79 / −22.61 % wall, strict sep 2.37 / 2.15 s, CPU 1.844x / 1.810x), **1.7382x / 1.6843x** (cut 42.47 / 40.63 %). Accelerated beats ordinary in **6/8** wall (−7.86…+2.45 %, median **−2.28 %**; faster in all 4 base cells) and **7/8** CPU (−2.91…+0.52 %, median **−1.16 %**). Explains benchy surprise vs t172 estimates (t169's chain had omitted t166's +7.9…+21.4 % benchy gains); confirms t173's base supports-on cuts; all four supports-off cells cross the ~1.5x below-fold park bound. Disclosed: initial loaded-box campaign (`t174-post-t172-scoreboard`, external ~10 GB load ending mid-`base-arachne-on` with +10.44 GB RAM release, 4 starvation flags, 64–98 % spreads) invalidated as primary and preserved as `reduced/loaded-window.json` + `reduced/loaded-window-load-samples.csv` (`t156` precedent; still 16/16 identity and 2/8 wall wins at 0.832x / 0.863x); no null band, no `--profile`. Graduated [176](issues/176-destination-cpu-corroboration-ruling.md). Evidence: [`evidence/t174-post-t172-scoreboard/`](evidence/t174-post-t172-scoreboard/FINDINGS.md) (`verify.py` PASS 1280/1280, `--self-test` 18/18).
- [Port Orca's `OffsetConcaveCrossing` into `vendor/clipper2-rust`](issues/172-port-offset-concave-crossing.md): **resolved 2026-11-20, measured candidate — strict Q2 gate PASSED 8/8 (+14.89…+23.01 % wall, 5/5 pairs and +19.20…+30.67 % CPU in every cell), landed under the sixth recorded t81-style exception; `base classic-off` (~0.82x est) and `base arachne-off` (~0.89x est) are projected to beat OrcaSlicer, and the Clipper2 kernel route is exhausted for the six non-winning cells** — human granted the sixth scoped t81-style exception (Q1) and strict landing gate (Q2) before production edits. Ported canonical `v2.5.0` (`40ecc66f78`) `clipper.offset.cpp::OffsetConcaveCrossing` (≈30 lines, `.round()` conversion, AGPLv3 attribution header; `DoRound` / `GetLineIntersectPt` / `c2_offset_as_group` excluded) into `vendor/clipper2-rust/src/offset.rs` (`ClipperOffset::offset_polygon`). Falsifiers in `offset_tests.rs` watched failing pre-patch (`96 != 32` raw pre-union vertices on a 32-gon inward offset) and green after (**405/405** + 3 doc-tests); `clip_polylines_prepared_reference.json` updated for two pre-patch inflated rings (`hole_and_island` cyclic start shift by 1, `dense_ring` ≤ 100 nm on concave corners; all clipped polylines identical). Four staged arms (`b2e8261e…` / `9f3fcba8…` / `f56c9271…` / `490c6dc2…`): baseline reproduces t169 **8/8** on both arms (`16/16`), candidate moves **8/8** and is byte-identical between ordinary and accelerated (**8/8**); active currency is now [`evidence/t172-offset-concave-crossing/reference.json`](evidence/t172-offset-concave-crossing/reference.json) (t169 superseded). Paired ordinary medians (5 pairs/cell): benchy **+23.01 / +18.41 / +21.49 / +17.37 %**, base **+21.41 / +14.89 / +19.88 / +15.00 %** (**18.6x–28.8x** the 0.80 % floor, strictly outside per-cell any-pair wall null bands of 2.29–13.69 %; both base supports-off cells beat their t169 estimated cuts of 3.9 % and 10.3 %); CPU **+19.20…+30.67 %** (5/5 every cell); accelerated corroboration (3 pairs/cell) wall **+14.19…+22.63 %** / CPU **+19.33…+30.93 %** (3/3 every cell). Delta audit: total extruding path **−0.745…+0.007 %**, no feature loss (`Outer wall` ≤ 0.0012 %, `Inner wall` ≤ 0.042 %, `Support` −1.57 % benchy / −0.69 % base). Cross-window route composition (t163 × t169 × t172, estimate only): `base classic-off` **~0.82x**, `base arachne-off` **~0.89x**, benchy ~1.26 / ~1.94 / ~1.44 / **~1.99x** (binding cell, 49.8 % cut), base supports-on ~1.80 / ~1.73x. Disclosed: 30-min timeout pause between rows 75 and 76 of `ab` (`p5 base-classic-on` spans pause; other 39 pairs back-to-back); no `--profile` pass. Graduated [174](issues/174-post-t172-scoreboard-refresh.md). Evidence: [`evidence/t172-offset-concave-crossing/FINDINGS.md`](evidence/t172-offset-concave-crossing/FINDINGS.md) (`verify.py` PASS, `--self-test` 8/8; `audit_t172.py --self-test` 4/4).
- [Support-path boolean volume](issues/173-support-geometry-boolean-volume.md): **resolved 2026-11-20, attribution + shape pricing (no production edit) — the support path's clip mass is `tree-support-planner`'s phase-1 carve and phase-3 union, and no single boolean-volume shape clears either base supports-on cell; the base pair needs a combination and the benchy pair is not a boolean program** — the `host:native` clip bucket on `base classic-on` is the planner (**196.6 s** thread / **18.00 s** wall, **85.2 %** of all clip thread), with `planner::batched_phase1_carve` at **68,381** calls / **142.7 s** / **12.41 s** / **11.5x** fan-out and `planner::batched_phase3_union` at **480** / **51.6 s** / **5.21 s** / **9.9x**; the two sites are **98.8 %** of planner clip thread. Operand: **426.9 M** clip points (mean **6,241/call** over **4.34** paths) against **98.7** subject points, but only **285,368** clip points (**0.0669 %**) lie inside the subject bbox; a polygon-level bbox filter removes **28.5 %**. Stage decomposition (same window, base classic-on): **62.2 %** host clip / **3.3 %** host offset / **34.5 %** remainder; benchy classic-on is **38.8 %** host clip / **56.1 %** remainder; planner fuel is 10.8 % of cell fuel. Prices (ceilings, fraction of required cut): phase-1 bbox/tile window **60.8 % / 61.6 %** (1.150x / 1.120x), polygon-bbox filter **17.6 % / 17.8 %**, phase-3 union free **26.0 % / 27.4 %** (no shape), all planner clip free **89.7 % / 92.1 %** (**0.948x / 0.926x**), all planner clip+offset free **94.7 % / 97.6 %** (**0.913x / 0.891x**); phase-1 window + phase-3 free is **0.969x / 0.945x** but phase 3 is a ceiling. Benchy: phase-1 window **23.2 % / 21.8 %**, all clip free **35 % / 34 %**. Re-aim: base pair = phase-1 tiling + a phase-3 companion; benchy pair = per-layer/guest program. Disclosed: one probe pass/cell, identical operands read 134–216 s across takes (box state), probe thread/wall inflated, route is t172's estimate (t174 is the board); no accelerated arm; all 5 probe + 15 instrumented runs reproduce t172's set; probe reverted. Graduated [175](issues/175-tiled-collision-carve.md). Evidence: [`evidence/t173-support-boolean-volume/FINDINGS.md`](evidence/t173-support-boolean-volume/FINDINGS.md) (`verify.py` PASS, `--self-test` 8/8).
- [Re-price the prepass work-set, the phase split and S5/S6 after t169](issues/171-post-t169-prepass-reprice.md): **resolved 2026-11-20, attribution + re-price (no production edit) — the split re-derives on the post-t169 tree and the route re-aims: the supports-off cluster is offset-bound and is t172's, the supports-on cluster is boolean-bound, and S5/S6 are landed keeps with no residual worth a ticket** — phase split (median of three instrumented passes, definitions unchanged from t164): prepass **55.9–83.7 %** (was 67.0–87.2 %), per-layer **12.0–34.7 %** (was 9.3–26.3 %), postpass 4.0–8.3 %, outside 0.2–3.3 %; the prepass share fell **3.5–12.3 pp** and the per-layer share rose **2.7–9.9 pp** on all eight cells, tracking each cell's t169 saving (base −21.9…−25.1 %, benchy −5.7…−15.2 %). Stage split: `ShellClassification` **32.2–44.0 %** of the supports-off cells (57.4–58.6 % of the prepass), `SupportGeometry` **38.9–54.9 %** of the supports-on cells (55.6–66.1 %). Expressed as the fraction of the largest stage the cut needs: base classic-off **8.9 %** and base arachne-off **24.2 %** of `ShellClassification` (zeroing it leaves 0.58x / 0.64x), base classic-on **96.0 %** and base arachne-on **92.4 %** of `SupportGeometry` (0.95x / 0.92x), and **no benchy cell has a sufficient single stage** (114.5 / 140.7 / 143.1 / 150.3 % of its largest; 1.08x / 1.24x / 1.41x / 1.47x at zero). Per-op census (`--profile`, thread-time attribution): host clipper work is **offset**-dominated on the supports-off cells (**42.9–51.2 %** offset, 29.5–34.4 % clip, 16.1–18.3 % `offset2_ex`) and **clip**-dominated on the supports-on cells (**53.1–60.9 %** clip, 34.1–39.4 % offset, 4.2–6.0 % `offset2_ex`); host clipper thread-time is 7.2–88.8 s on the supports-off cells and 20.6–**343.4 s** on the supports-on cells, where `host:native` (the tool's unattributed bucket) holds **236.3 s of 343.4 s** on base classic-on (169,586 calls: clip 186.6 s, offset 49.7 s) and `host:support_analysis` holds 35.7 s (offset 32.6 s) — `host:native`'s 8.6x thread-to-wall ratio fits `SupportGeometry` (27.6 s) and cannot fit the per-layer phase (5.94 s), a **labelled inference** on which no quoted number depends. Guest fuel splits the same way: `tree-support` **57.7–68.0 %** plus `tree-support-planner` **10.4–10.7 %** on the supports-on cells, `classic-perimeters` 65.0 % / `arachne-perimeters` 56.0 % on the supports-off ones. **S5** (landed t133) rescales by ×0.4451–0.5901 to a **9.01 %** landed win on base classic-off with a **0.83 %** residual (91.6 % of the whole-layer cost already harvested; 0.41 % / 0.22 % elsewhere) — at the floor, not a lever. **S6** (landed t132) rescales to **5.86 %** of base classic-on's wall with a **5.89 %** residual (**11 %** of that cell's cut), and is **inert on the four supports-off cells by construction** (`enable_support: false`), so it can never satisfy an 8-cell gate. **Premise corrections:** there is no `PrePass::InternalBridges` stage (the internal-bridge work runs inside `PrePass::ShellClassification` via `commit_shell_classification_builtin` → `compute_internal_bridge_layer`), and S5/S6 are landed, so neither can be re-measured by ablation. No production edit, no reference move; all 24 instrumented passes reproduce t169's set byte-for-byte and the exe sha256 `b2e8261e…` equals t169's candidate build. Graduated [173](issues/173-support-geometry-boolean-volume.md). Evidence: [`evidence/t171-post-t169-reprice/`](evidence/t171-post-t169-reprice/FINDINGS.md) (`verify.py` PASS, `--self-test` 8/8).
- [Clipper2 1.1.0 → 2.0.1 kernel upgrade screen](issues/170-clipper2-2-0-1-kernel-port-screen.md): **resolved 2026-11-20, attribution + screen (no production change) — the kernel premise is falsified: the remaining offset gap is one missing Orca fork patch, `clipper.offset.cpp::OffsetConcaveCrossing`, worth 2.95–4.57x → 1.07–1.27x on the Rust port's classic-perimeter offsets and 2.38–2.70x → 1.04–1.30x on the canonical openings, and it wins the first two cells on paper** — the vendored crate is version 1.1.0 but a **1.5.4** port; upstream 1.2.0 is still 1.5.4 and has no patch, and there is no 2.x. Applying the patch to bare Clipper2 1.5.2 reproduces canonical 2.0.1 **byte-for-byte on 87/87** IB + `0.04 mm Miter 3` opening cases, while without it 2.0.1 is **0.78–0.87x** of 1.5.2 on those openings — the version contributes nothing. Ported into a scratch `vendor/clipper2-rust` it captures **81–97 %** of the remaining per-call gap (prepass IB **1.86x**, `SH_OPEN_*` **1.63x**, `t68sites` base **3.17x** / benchy **2.56x**), vertex totals converge on canonical (`A0` 1.082 → 1.007, `A1` 1.073 → 0.998), booleans are untouched (0.61–0.94x), and the scratch crate passes **403/403** tests. **Price (post-t169, two-step estimate):** central **+16.4 %** on `base classic-off` (required **3.9 %**) and **+15.3 %** on `base arachne-off` (required **10.3 %**), **5.2–11.4 %** elsewhere; conservative host-only clears the 0.80 % floor in **8/8** (5.2x–16.3x). **But the kernel route is bounded:** even at zero cost for every host and guest offset, six cells stay short (`base classic-on` 23.3 % vs 52.7 %; `base arachne-on` 21.6 % vs 50.8 %; the four benchy cells 10.7–26.1 % vs 39.0–58.5 %), so those six need a second phase. **Sub-factors:** flat-path pass-1 is already in every driver and PnP (Arm 1 landed 2026-07-11); `PreserveCollinear(false)` is not a factor on offsets (both sides pass `false`; Rust's boolean default `true` is a parity residual with booleans already faster); `c2_offset_expolygons` groups all 118 prepass cases (`stray_holes = 0`) and the per-path fallback fires on only 6/170 `t68sites` cases. **Exception:** the patch moves per-layer geometry and needs a **t81-style ruling** (the sixth, if granted) plus a fresh reference set. Graduated [172](issues/172-port-offset-concave-crossing.md) (HITL ruling first, then the standing gate). Evidence: [`evidence/t170-clipper2-201-screen/`](evidence/t170-clipper2-201-screen/FINDINGS.md) (`verify.py` PASS, `--self-test` 6/6).
- [Port canonical `append_offset_path` edge decimation into `polygon_ops`](issues/169-canonical-offset-edge-decimation-and-flat-opening.md): **resolved 2026-11-20, measured candidate — the gate PASSES in 8/8 cells, the base cluster gains 21.9–25.1 % wall and `base classic-off` lands within an estimated 3.9 % of Orca, and it is the route's first banked multi-cell win since t158** — `polygon_ops` harvests its rings exactly as canonical `v2.5.0` (`40ecc66f78`) `ClipperUtils.cpp::append_offset_path` does: wrap-around trim, collapse against the **last kept** vertex, `< 3` drop, canonical's asymmetric degenerate-contour branch (`delta < 0` drops, `delta >= 0` keeps), hole and winding normalisation; `offset_paths` serves `offset` / `offset_with_miter_limit` / `opening` / `closing_ex` / pass 1 of `offset2_ex`, and `offset_paths_flat` decimates **pass 2** at `|delta2| * 0.005` (not pass 1's threshold), because canonical's `c2_offset2_closed` harvests both passes. The factor is dimensionless and transfers to PnP's 100 nm grid without rescaling. **Arm 1 is not a delta:** flat-path pass-1 `offset2_ex` landed 2026-07-11 (`a5433f93`) and was extracted into `offset2_ex_first_pass` by `b56f429a`, so t167's `f2` was partly already banked and the whole measured effect is Arm 2 plus pass-2 decimation. **One window, four arms** (`e7b67146…` ordinary baseline / `b2e8261e…` candidate / `0784a754…` / `f56c9271…`), module trees `6899f824…` / `9c1da3bf…` / `82d74609…` / `9504e039…` — the trees move because `classic-perimeters` (`offset2_ex`, `opening_ex`) and `arachne-perimeters` (`offset2_ex`, `offset_with_miter_limit`) call the decimated helpers **in-process**, so the port is not host-only. **Output:** baseline reproduces t168 **8/8**, candidate moves **8/8**, accelerated arms byte-identical to ordinary **16/16**; t169's set is the new currency and t168's is superseded. **Paired wall (5 interleaved pairs/cell):** benchy **+7.98 / +15.21 / +5.74 / +14.76 %**, base **+25.08 / +21.91 / +23.43 / +22.41 %**, 5/5 pairs faster in every cell; null band **3.06 %** any-pair / **1.99 %** median wall, so all eight cells clear the 0.80 % floor (base at **27.4x–31.4x**) and sit outside their own band; **process CPU lower 8/8**, 5/5 pairs in every cell. **Accelerated corroboration (3 pairs/cell):** wall **+7.11 / +15.37 / +6.22 / +13.93 / +25.18 / +22.71 / +24.25 / +21.91 %**, 3/3 every cell, CPU 3/3 every cell. **Audit:** every section moves somewhere in every cell, total path **−0.029…+0.616 %**; `Internal Bridge` **−72.0 / −73.2 %** (−9.25 / −9.84 mm) on the two base supports-off cells is the expected pinhole collapse at the `−1.6069 mm` offsets' 8.03 µm threshold, while `Support` **+0.42 / +1.13 %** (+345.61 / +4,383.20 mm) on the supports-on cells is a direction the port does not predict and is reported, not rationalised. **Attribution (`--profile`, never a wall claim):** `polygon_ops::offset` self wall **−41.0…−55.5 %**, `offset2_ex` **−12.6…−25.1 %**; guest fuel (deterministic) falls **13.5–21.5 %** on the classic cells but **rises 1.4–3.1 %** on the three Arachne cells — the scan costs more than the sweep saves on already-sparse geometry, disclosed, and those cells still win on their host-side prepass offsets. **Route (estimate, cross-window):** composing the paired medians with t163's ordinary board gives `base classic-off` **1.389x → ~1.04x** (3.9 % short — the first single-digit gap in this map), `base arachne-off` ~1.12x, `base classic-on` ~2.12x, `base arachne-on` ~2.03x, benchy ~1.64 / ~2.38 / ~1.83 / **~2.41x**; the binding cell moves to **`benchy arachne-on` at 58.5 % required cut**, so t128's "benchy classic-off cannot be won by prepass deletion alone" survives in the stronger form that neither can benchy arachne-on. **Exception:** landed under the **fifth recorded t81-style exception** (per-layer geometry), granted in-session on these numbers; without it the change would have been preserved as a proposal per t152's precedent. **Falsifiers watched red before the fix:** the 4 µm pinhole test (a ~2 mm crater without the arm) and the wrap-trim unit test (5 vertices instead of 4). **Not ported, pre-existing, disclosed:** canonical `c2_offset_as_group`'s per-path fallback, `c2_offset_setup`'s miter/arc clamps, and the zero-delta conventions. Evidence: [`evidence/t169-offset-decimation-flat-opening/`](evidence/t169-offset-decimation-flat-opening/FINDINGS.md) (`verify.py` PASS, `--self-test` 6/6; `audit_t169.py --self-test` 4/4).
- [Bridge opening radius resolves to zero through the auto sentinel](issues/168-bridge-opening-auto-sentinel.md): **resolved 2026-11-19, canonical-parity fix landed — the bridge opening is live through one shared width policy, bridge path falls 8/8 and process CPU is lower 8/8 on both build arms, but no cell's wall effect clears the measured null band, so the standing gate fails on wall (0/4 `base`) and nothing is banked as speed** — `resolve_external_perimeter_width_mm` (`crates/slicer-core/src/algos/prepass_slice.rs`) now requires a **positive** absolute `outer_wall_line_width` (both `Float` and absolute `FloatOrPercent`) and falls through to `line_width`, then 0.4 mm; the matched job's `outer_wall_line_width = 0` is canonical's **auto** sentinel, and reading it literally ran canonical `detect_surfaces_type`'s `opening_ex(diff, fw / 10)` at radius zero, so every sub-printable sliver survived as a bridge. The shell stage's copy (t166's `resolve_shell_external_width_mm`, `crates/slicer-runtime/src/slice_postprocess_prepass.rs`) is now a one-line delegate, so the two openings share one policy. **Falsifier:** `external_perimeter_width_treats_zero_extension_as_auto` is red under the pre-fix arm (0.0 vs 0.4) and green after; `external_perimeter_width_treats_zero_percent_form_as_auto` pins the absolute-`FloatOrPercent` zero. **Output:** baseline reproduces t166's set **8/8**, candidate moves **8/8** (`d6cb4acd…` / `21c6969f…` / `5c0f846e…` / `6fe4b09b…` / `9e360450…` / `f77cf265…` / `cd896bab…` / `3044c4d6…`, bytes −2,373 / −2,327 / −2,112 / −2,218 / +24,435 / +24,313 / −5,288 / −5,328), accelerated arm identical 16/16, all 286 runs `degraded: false` with zero code-1200. **Delta audit:** bridge path falls in **8/8** (−10.5 / −10.5 / −5.0 / −5.0 / −11.4 / −11.4 / −8.1 / −8.1 mm), internal bridge rises 8/8 (+3.6 / +2.0 mm), `Gap infill` inert; sparse infill rises only on `base classic` (**+490.2 mm, +0.28 %**) and falls 0.01–0.02 % on the other six cells — a partial reversal of the expected direction, reported not widened; the per-layer census (19–26 base layers gain bridge path, 27–33 lose it; first differing layer 3) says the opening relabels the bottom set, and the anchor-direction proxy (dominant printed bridge angle, not the IR field) moves ≥ 45° on 12/48 / 13/61 / 2/7 bridge layers. **Paired wall (10 interleaved pairs/cell, ordinary):** benchy **+1.99 / +1.29 / +1.17 / +1.71 %**, base **+4.09 / −0.14 / +4.15 / +0.73 %**; null bands 4.89–9.86 % any-pair on wall, so both supports-off base medians sit **inside** their band and both supports-on medians are at zero — **gate FAILED on wall, 0/4 `base`**, benchy clause passes (no median regression). **Process CPU lower in 8/8** (10/10 pairs in five; outside the per-cell null on both supports-off base cells, inside on the supports-on pair), and 8/8 on the accelerated arm (3 pairs/cell). No ratio of record changes; route arithmetic and the 8/8 destination stand. Disclosed: wall instrument weak in this window (one 2.7 s benchy pair at −24.51 %), partial null coverage, ten pairs per cell after a disclosed extension from five, and no accelerated null band.
- [Canonical-kernel replay: how fast are Orca's own ClipperUtils ops on PnP's prepass operands?](issues/167-canonical-clipperutils-replay.md): **resolved 2026-11-19, canonical-side measurement (zero PnP production change) — Branch A fires (`canonical_ge_3x_faster_on_hot_offset_sites`): canonical `v2.5.0` migrated to Clipper2 2.0.1 (`222c6a2df5`), is 4.96x–5.02x faster on `offset` (5.16x–5.78x 12-way loaded) and 2.45x–3.26x faster on `opening_ex`, while booleans remain at parity (0.79x–1.41x)** — premise correction: at the `v2.5.0` pin (`40ecc66f78`), OrcaSlicer deleted Clipper1 (`deps_src/clipper/`) and runs Clipper2 2.0.1 + `ClipperUtils.cpp` (`append_offset_path` edge decimation `0.005 * |delta|`, flat-path pass-1 `offset2_ex`, `PreserveCollinear(false)` booleans, 1 nm ×100). Replayed 413 cases (295 `t69` + 118 newly captured `ShellClassification` Pass 1 `apply_opening` + `gate_internal_bridge_sites` operands on `base classic-off`, output-verified byte-identical to `t166`'s `0a1ee3df…` reference; probe reverted and saved as `probe-t167-dump.patch`) across four engines (`rust_pnp` `vendor/clipper2-rust` 1.1.0 + `mimalloc`, `cpp_c2_1_5_2` t69 twin, `orca_v2_5_0`, `orca_clipper1`). Single-thread (fresh process/case, 1 warmup + 5 repeats, fwd/rev, 405 unique): `difference` **0.79/0.78x**, `intersection` **1.24/1.24x**, `union` **1.39/1.41x** (booleans at parity, confirming `t70`), vs `offset` **4.96/5.02x** (**4.00x** vs `t69`'s bare C++ 1.5.2 twin; `IB_DEEP_NEG` **12.76/12.28x**, `IB_DEEP_POS` **4.56/4.64x**, `base-t68sites` **3.92/3.97x**, `benchy-t68sites` **3.11/3.13x**), `opening_ex` **2.49/2.55x** (`SH_OPEN_*_CANON` **2.45–2.57x**). 12-way concurrent loaded (320 base cases × 12 workers, fwd/rev): whole-batch wall `rust_pnp` **77.13/64.36 s** vs `orca_v2_5_0` **23.09/21.75 s** (**3.34/2.96x**); load widens canonical's lead (`opcode/offset` **5.78/5.16x**, `base-t68sites` **5.12/4.87x**, `SH_OPEN_TOP_CANON` **3.26/2.91x**, `SH_OPEN_BOT_CANON` **3.00/2.79x**, `IB_DEEP_POS` **5.19/4.49x**, `IB_DEEP_NEG` **12.62/11.36x**). Allocator disclosure: `rust_pnp` uses `mimalloc` vs MinGW CRT `malloc` on C++ shims, so C++ speedups are conservative lower bounds vs Orca's `tbbmalloc`. 3-way ablation (`ablation_decimation.json`, 118 cases, 10 repeats): `append_offset_path` edge decimation (`0.005 * |delta|`) delivers **8.27x** (`IB_DEEP_NEG`), **3.56x** (`IB_DEEP_POS`), **2.04–2.10x** (`0.20 mm` openings), **1.26–1.29x** (`0.04 mm` openings); Clipper2 `1.5.2 → 2.0.1` + flat-path `offset2_ex` pass 1 (no decimation) delivers **1.72–1.76x** on `SH_OPEN_*_CANON`; Rust 1.1 → C++ 1.5.2 is **1.09–1.47x**. Geometry: `orca_v2_5_0` has **0 empty mismatches** and **0.0044 %** median Δarea (`IB_DEEP_NEG` +6.44 % median Δarea is canonical collapsing ~5 µm pinholes rather than inflating them into 3.2 mm craters — non-output-neutral canonical parity); **Clipper1 is ruled out** (slower on offsets/booleans + degenerate hairline-sliver inversion bug on `IB_DEEP_NEG` L133/L493). Graduated [169](issues/169-canonical-offset-edge-decimation-and-flat-opening.md) and [170](issues/170-clipper2-2-0-1-kernel-port-screen.md). Evidence: `evidence/t167-canonical-clipperutils-replay/` (`verify.py --self-test` PASS, 6/6 negative controls rejected).
- [Port canonical's top/bottom opening into ShellClassification Pass 1](issues/166-shell-opening-canonical-parity.md): **resolved 2026-11-19, measured candidate — the opening is canonical, the benchy cluster saves 7.9–21.4 % wall, the speed claim is DEGRADED (5/8) with process CPU lower in 8/8, and two auto-sentinel defects surfaced** — Pass 1 now runs canonical `detect_surfaces_type`'s `opening_ex(diff, external_perimeter_width / 10)` with `jtMiter` / miter limit 3 instead of `line_width / 2` with `Round`; `ShellClassificationParams` carries the opening radius and Pass 2's shadow width separately so the cascade cannot move. **Defect 1:** `outer_wall_line_width = 0` is canonical's auto sentinel, not a zero-width wall — resolving it literally disables the opening (measured no-op: 5,201,451 points in and out, 6.8 ms / 974 calls), so `resolve_shell_external_width_mm` resolves it as `Flow` does (0.4 mm → 0.04 mm). **Defect 2:** the same sentinel reaches `classify_unsupported_bridge_areas`, so canonical's bridge opening has been a no-op on a path with live readers — graduated [168](issues/168-bridge-opening-auto-sentinel.md). **Rejected:** also porting `ApplySafetyOffset` (20.2 vs 19.2 s on base classic-off, output moved). **Campaign (80 paired runs, 5 pairs/cell):** benchy **+20.435 / +7.915 / +21.395 / +9.277 %** (25.5x / 9.9x / 26.7x / 11.6x floor, 5/5 pairs each), base **+3.448 / +2.468 / +0.310 / +0.734 %** (4.31x / 3.09x / 0.39x / 0.92x); CPU lower 8/8; null bands 6.09–11.54 % wall any-pair / 0.50–5.35 % median, 0.74–1.74 % CPU any-pair, so `base classic-off` is DEGRADED and the two base Arachne cells are sub-floor on wall and unresolved by the instrument. **Parity:** top/bottom solid path grows in 8/8 cells but the “never less” clause is falsified per layer (12–29 top / 1–15 bottom layers lose path); the change is a relabelling across neighbours. **Output:** baselines reproduce t151 8/8, all eight candidates move, t166's set is the new currency. **Route (estimate):** benchy required cuts 49.45 → 36.47 % and 47.21 → 32.84 %, supports-on 65.33 → 62.35 % and 64.08 → 60.40 %, base 0.3–3.4 %; 0/8 won, binding constraint untouched. Gap: no accelerated arm. Evidence: `evidence/t166-shell-opening-parity/`.
- [Verify canonical's prepass cost on 2.5.1](issues/165-canonical-prepass-cost.md): **verified (2026-11-19)** — canonical prepass 104.7 ms (bound) / 569.7 / 2,172.4 / 10,368.1 ms, within −0.7…+3.6 % of t110 on the points; 8/8 still win at canonical prepass, binding `benchy arachne-on` 0.898x with 1.47x slack.
- [Phase re-attribution at HEAD: shares, walls and route arithmetic in one window](issues/164-phase-re-attribution.md): **resolved 2026-11-18, measured board + instrumented split + arithmetic — the prepass is now 67.0–87.2 % of every cell, all eight cells win at canonical prepass cost (0.396x–0.890x), and the binding cell moves to `benchy arachne-on` at 0.890x** — one window, two tools (OrcaSlicer 2.5.1 `5013b1bd…`, PnP ordinary `2c44d38f…` at HEAD `c83d253b`), 8 cells × 1 warmup + 3 measured, 64 rows, matched keys on all 24 Orca runs, **8/8 output identity** to t151's set and every run clean; **24 instrumented passes** (3/cell) also **24/24** identical; load 99 samples 6.8/66.6/100 % (45.9 % in the scoreboard window). **Split:** prepass **67.0–87.2 %**, per-layer **9.3–26.3 %**, postpass **3.2–6.8 %**, outside **0.1–2.3 %**, repeat spread 0.72–3.91 pp, per-layer host gap 0–1 ms; the prepass share rose on **all eight** cells (**+14.85 pp** base classic-off, **+11.51 pp** base arachne-off, **+4.81/+5.23 pp** base supports-on, **+1.07…+6.00 pp** benchy) exactly where t158's 4.23–6.37 s of per-layer removal landed. `MeshAnalysis` 687–697 ms base / 86 ms benchy; base-off's prepass is 68.4 % `host:shell_classification`, base-on's 54.9 % `SupportGeometry` + 20.0 % `SupportAnalysis`. **Board: 1.379x–2.884x, 0/8 won, required cut 27.5 %–65.3 %**, corroborating t163 without composing with it. **Arithmetic (estimate): benchy 0.611x/0.874x/0.649x/0.890x, base 0.400x/0.786x/0.396x/0.718x; `needed_from_other = 0` everywhere**, so the prepass alone suffices — the reversal of t154's shape. **The binding cell is `benchy arachne-on` (0.890x)**, and the constraint is the input: the four supports-on cells carry **1.50x–1.75x** slack on t110's canonical prepass cost, the benchy pair only **1.53x/1.59x**; the substitution implies a prepass cut of **82.6 %–98.1 %**. Graduated [165](issues/165-canonical-prepass-cost.md) to verify that input on 2.5.1. Disclosed: instrumented walls are never wall claims, shares × scoreboard walls only, one window, one external input, no null band. Evidence: `evidence/t164-phase-re-attribution/` (`verify.py` PASS, every number re-derived; `--self-test` 16/16).
- [Accelerated timing arm: three-tool eight-cell board on the current tree](issues/163-accelerated-timing-board.md): **resolved 2026-11-18, measured board — the accelerated arm is output-exact 8/8 and worth −13.2…+4.4 % wall (median cell −1.1 %), so it is real but not a route lever** — one window, three tools (OrcaSlicer 2.5.1, PnP ordinary, PnP accelerated), 8 cells × 1 warmup + 3 measured runs, 96 rows. **Both PnP arms byte-identical to t151's active set in 8/8** (`exit 0`, `degraded: false`, matched keys on all 24 Orca runs). Board: benchy **1.782x/1.757x, 2.802x/2.703x, 1.940x/1.684x, 2.829x/2.805x** and base **1.389x/1.450x, 2.710x/2.751x, 1.456x/1.426x, 2.618x/2.611x** (ordinary/accelerated). **0 of 8 won on either arm**; required cut 28.0–64.7 % ordinary, 29.9–64.3 % accelerated. Wall deltas **−13.2 / −3.5 / −2.1 / −1.4 / −0.8 / −0.3 / +1.5 / +4.4 %**, median **−1.1 %**, mean −1.9 %, against per-cell spreads 2.9–21.6 % — only `benchy arachne-off` (−13.2 %) and `base classic-off` (+4.4 %) exceed their own spread; **process CPU is the tight channel and is lower in 6/8** (−0.8…−4.3 %). **On the tightest cell the arm is worse** (`base classic-off` 1.389x → 1.450x). Unplanned cross-window control: both arms moved together against t161 and the paired ratio stayed intact (benchy-off −0.8 % for 3.5–4.3 % movement; base-off −2.8 % for 11.8–15.1 %), independently confirming t161's method claim; no absolute wall is compared with t161's. Disclosed: no null band (board not candidate), load 114 samples 1/76/100 %, one window only. Evidence: `evidence/t163-accelerated-board/` (`verify.py` PASS 366/366, `--self-test` 14/14).
- [Accelerated host build: the `libmimalloc-sys` C1083 is a verbatim-path bug, not the wrapper](issues/162-accelerated-host-verbatim-path.md): **resolved 2026-11-18, defect fix — the accelerated host builds again (exit 0, 1 binary + 24 modules) and the map's ten-ticket attribution was wrong** — `build_guests::workspace_root()` returns `cwd.canonicalize()`, which on Windows is the verbatim `\\?\D:\...` spelling; `dist.rs` set `CARGO_TARGET_DIR` from it, cargo propagated it into every build script's `OUT_DIR`, and `cc-rs` 1.2.59 (no verbatim handling at all) passed it unchanged into `cl.exe`, which read `-c \\?\D:\...\out\mimalloc-static.cc` as the UNC path `\\mimalloc-static.cc`. **Four builds separate the hypotheses:** fresh-dir no-wrapper **green** (5.2 s); fresh-dir + the driver's shim as `RUSTC` + `PNP_ACCELERATED`/`PNP_ACCELERATED_MODE`/`PNP_XTASK_*` **green** (6.2 s); ordinary `CARGO_TARGET_DIR` **green**; verbatim `CARGO_TARGET_DIR` **red in ~5 s with the dist's exact error**. So the wrapper, MSVC 14.51 and `libmimalloc-sys` were never involved. **Fix:** flatten the root with the existing `rustc_driver::strip_verbatim_prefix` (which gained `\\?\UNC\` handling) before it becomes `CARGO_TARGET_DIR`; a **second bug** surfaced — `relative_input_path` compared raw spellings, so a flattened root against `canonicalize`d closure manifests leaked absolute paths into every guest fingerprint and made all 60 guests read stale, now flattened on both sides (and spelling-independent, which is what packet 71 wanted). **Verified:** dist exit 0, `module diagnose` pass with 24 modules provenance `external`, **8 cells × 2 arms output identity 16/16** against t151's active set (all `exit 0`, `degraded: false`), both freshness gates 0/0, clippy clean, three regression tests each red before the fix. **Corrected:** every accelerated disclosure from t142 to t161. **Not claimed:** no accelerated timing number — the arm is available, not measured. Evidence: `evidence/t162-accelerated-host-verbatim-path/` (`verify.py` PASS 139/139, `--self-test` 12/12).
- [Post-t158 eight-cell scoreboard: re-derive the ratios of record on the current tree](issues/161-post-t158-scoreboard-refresh.md): **resolved 2026-11-17, measured board — 1.366x–2.718x and 0 of 8 won in one interleaved window, and the Orca arm itself ran 5.4–24.9 % slower than on t145's board in all eight cells** — 8 cells × {Orca, PnP ordinary}, 1 warmup + 3 measured runs/cell, matched keys asserted on all 24 Orca runs, PnP output **8/8 identical** to t151's active set with `exit 0`, `degraded: false` and zero exact-Z diagnostics. Table: benchy **1.796x / 2.671x / 1.794x / 2.591x** (4.076 / 8.144 / 3.790 / 7.337 s against 2.269 / 3.050 / 2.113 / 2.831 s), base **1.429x / 2.718x / 1.366x / 2.622x** (22.603 / 70.754 / 22.505 / 71.377 s against 15.812 / 26.030 / 16.474 / 27.219 s); required cut **26.80 %–63.21 %**, and the two base supports-off cells carry a **negative 1.5x cut** (−4.94 % / −9.80 %). **The tightest cells changed from the benchy supports-off pair (t145) to the base supports-off pair.** The constraining finding: the installed OrcaSlicer is **2.5.1** (binaries dated 2026-11-13 23:59–11-14 00:00, `5013b1bd…`), not the **2.5.0** the map records, with the profile stamp unchanged (`2.08.00.16`); Orca moved **+5.4…+24.9 %** in wall in 8/8 cells while its CPU moved only **−2.2…+11.7 %** and its effective parallelism fell in 8/8 (−2.2…−17.4 %) while PnP's did not, so the movement is efficiency not work and the binary is the leading explanation, with box state not excludable (2.5.0 uninstalled; benchy classic-off's 4,077,463 bytes equal t145's figure), so **no ratio or required cut may be composed across the tool boundary** — t145's 46.03 %–71.06 % and today's 26.80 %–63.21 % are each correct for their window. Two readings that survive: the benchy supports-on cells moved **+24.3 % / +24.9 %** on both arms while their ratio moved **−0.5 % / +0.4 %** (the paired ratio absorbs a ~25 % box shift), and the base supports-off improvement is independently corroborated (landed deltas predict −25.2 % / −22.6 % of t145's PnP wall; measured −18.9 % / −24.0 %). Prepass phase shares and the route projection were **not** re-derived (pre-t158). Disclosed: no accelerated arm — the ordinary-only driver skips the accelerated gate, and checking that tree found **45 stale guests** (the whole accelerated namespace, pre-existing); rebuilt after the campaign (`--accelerated --check` now exits 0, ordinary too) and `dist --accelerated` still fails at exit 1 in `libmimalloc-sys` (`C1083` on `'\\mimalloc-static.cc'`, t142's MSVC-14.51 verbatim-path failure). **Corrected 2026-11-18 by [162](issues/162-accelerated-host-verbatim-path.md): a verbatim `CARGO_TARGET_DIR`, not the toolchain; the arm builds again.** Also one Orca outlier 29.807 s on base classic-off m2 (largest of three, disclosed not dropped), load 62 samples min 2 % / median 69.5 % / max 100 %, `tree_strong` pin. Evidence: `evidence/t161-scoreboard-refresh/` (`verify.py` PASS 226/226, `--self-test` 16/16).
- [Tree-support render placement probe: whole-object batch vs per-layer dispatch](issues/160-tree-support-prepass-batch-probe.md): **resolved 2026-11-17, measured probe — the fold is a pure placement move and the batch arm is slower in every supports-on cell, so the per-layer structure is EXONERATED** — one env-gated host binary, four arms, work held constant. Read-set digest (`plan` / `view` / `paint`) identical at both dispatch points on **every layer of all eight cells** (496/496 base, 240/240 benchy per side); batch arm byte-identical to t151's active set in **8/8**, all runs clean. 5 interleaved pairs/cell: batch slower in all eight separate-arm medians, supports-on **−1.69 % / −1.71 % / −3.13 % / −3.77 %** (pair wins 1/5, 2/5, 0/5, 1/5); batch-pass wall **3.9–4.2 s** base / **0.49 s** benchy inside the per-layer phase bracket. Same-arm null band **9.50 % wall any-pair / 1.91 % median** (CPU 5.89 %); null-renderer ceiling **9.16–12.61 %** of cell wall, so the renderer's cost is the work, not its placement. Supports-off skipped (`enable_support=false`), output identical, wall movement disclosed as box drift. Probe reverted, `probe.patch` preserved, `build-guests --check` exit 0. Evidence: `evidence/t160-tree-support-batch/` (`verify.py` PASS, `--self-test` 9/9).
- [Tree-support render: fold the per-layer `Layer::Support` render into the prepass?](issues/159-tree-support-render-prepass-fold.md): **resolved 2026-11-17, human ruling — EXPERIMENT, not adoption** — measure whether per-layer renderer placement costs wall with the work held constant: env-gated host batch of the unchanged guest after `PrePass::SupportGeometry` plus a null-renderer ceiling, per-cell verdict on the four supports-on cells (hurts = ≥ 0.80 % floor and outside the null band), canonical `generate_toolpaths` as the per-op control, probe only. Corrects t154's "pays support work twice" to a phase-placement difference (canonical `generate_toolpaths` is also a per-layer `tbb::parallel_for`). Graduated [160](issues/160-tree-support-prepass-batch-probe.md); domain model unchanged.
- [Exact fast-path boolean predicate in `SliceRegionView::derive_needs_support`](issues/158-needs-support-fast-predicate.md): **resolved 2026-11-16; human-amended 2026-11-17, KEEP** — replacing the unconditional Clipper2 `intersection_ex` inside `SliceRegionView::derive_needs_support` (`crates/slicer-sdk/src/views.rs`) with `expolygons_intersect_area_positive` (strict bbox overlap prefilter + strict-interior point-in-`ExPolygon` witness in `i128`, with `intersection_ex` fallback on overlapping pairs when no witness is found) passes all 72 `layer_module_tdd` tests (including 4 new boundary/hole/`+`-fallback cases), is byte-identical in **8/8** cells against `t151`'s active reference set across 16 output runs, 80 A/B runs, and 18 null runs (`exit 0`, `degraded=false`, zero diagnostics), lowers process CPU in **8/8** cells by **5.88 %–21.38 %** (**5.3x–19.1x** the `1.117 %` any-pair CPU null band; `39/40` pairs candidate-lower), and clears the `0.80 %` wall floor in **all four `base` cells** (`base-classic-off` **+14.87 %** / `−4.25 s` / `18.58x` floor / `−44.63 CPU s`, `base-classic-on` **+5.42 %** / `−4.68 s` / `6.78x` floor / pairwise **+8.67 %** / `−71.08 CPU s`, `base-arachne-off` **+14.64 %** / `−4.23 s` / `18.30x` floor / `−43.64 CPU s`, `base-arachne-on` **+7.52 %** / `−6.37 s` / `9.40x` floor / `−67.84 CPU s`; `20/20` pairs candidate-faster). However, because the entire three-site `derive_needs_support` ceiling on `benchy` is only `~231–237 ms` (`~3.0 %` of `benchy` supports-on wall), single-run wall jitter puts two `benchy` cells below the literal eight-cell separate-arm `0.80 %` wall floor over 5 interleaved pairs (`benchy-classic-on` **+0.766 %**, `0.96x` floor despite **+2.69 %** pairwise median and `4/5` pair wins; `benchy-arachne-off` **−0.076 %**, `−0.10x` floor despite **+3.16 %** pairwise median and `3/5` pair wins). The human explicitly accepted the candidate as KEEP on 2026-11-17; production code and unit tests are landed, with the original mechanical gate failure and missing `benchy` null control disclosed in `RULING.md`; guest freshness green (`cargo xtask build-guests --check` exit 0), `candidate.patch` preserved in [`evidence/t158-needs-support-fast-predicate/`](evidence/t158-needs-support-fast-predicate/FINDINGS.md) (`verify.py --self-test` exit 0, 6/6 negative controls rejected).
- [The surviving `needs_support` predicate: reader census, three-site cost, and exact fast-path probe](issues/157-needs-support-predicate-attribution.md): **resolved (2026-11-16; Phase-1 attribution + probe — `commit_support_analysis_builtin` is the sole production decision consumer, and an exact fast-path boolean predicate inside `SliceRegionView::derive_needs_support` collapses all three call sites by 91.0x–120.0x with 0 / 3,680 mismatches)** — workspace-wide reader census proves zero guest modules in `modules/` read `SliceRegionView::needs_support()` after Packet 224 retired `region.needs_support()` from `TreeSupport::run_support` (`modules/core-modules/tree-support/src/lib.rs`); only `commit_support_analysis_builtin` (`crates/slicer-runtime/src/builtins/support_analysis_producer.rs`) makes a production decision on `derive_needs_support`, while the two per-layer calls in `prepare_regions` (`crates/slicer-wasm-host/src/marshal/prepared.rs`) marshal a value read only by contract/unit tests. On `3dbenchy.stl` (`240` layers, `a_verts_mean = 1,890.1`, `b_polys = 201`, `b_verts = 6,107`), full `intersection_ex` costs **3.24–3.39 ms/call** (`1,571.3 ms` thread across 2 sites on `benchy classic-off`; `2,403.2 ms` across 3 sites on `benchy classic-on`); on `base.stl` (`496` layers, `a_verts_mean = 5,155.7`, `b_polys = 994`, `b_verts = 26,633`), it costs **49.4–60.9 ms/call** (`51,651.6 ms` across 2 sites on `base classic-off`; `84,792.0 ms` across 3 sites on `base classic-on`). Dual-computing an exact boolean fast path (strict bbox overlap prefilter + strict-interior point-in-`ExPolygon` witness in `i128`, with `intersection_ex` fallback on overlapping pairs when no witness is found) yields **0 / 3,680** mismatches across all 4 probed cells (`35.7–37.0 µs/call` on `benchy`, **91.0x–92.5x**; `410.2–561.6 µs/call` on `base`, **113.0x–120.0x**) and **8/8** byte-identical G-code runs against t151's active set. Under `PNP_T157_FAST_PREDICATE=1`, combined `prep_views_ms` (`Layer::Perimeters` + `Layer::Infill`) falls **−76.0…−77.1 %** on `benchy` and **−93.0…−93.8 %** on `base` (`64,875.1 → 4,011.5 ms` on `base classic-on`), and `PrePass::SupportAnalysis` eligibility wall on `base classic-on` falls **2,100.5 → 41.5 ms (50.6x)**. Priced against `t154`'s ledger (`ESTIMATE`, not a ratio of record): **230.6 ms** (`benchy classic-off`, **6.00 %** of wall, **7.50x** floor, `2.0068x → 1.8864x`), **237.3 ms** (`benchy classic-on`, **3.78 %** of wall, **4.73x** floor, `2.5711x → 2.4739x`), **4,692.5 ms** (`base classic-off`, **19.15 %** of wall, **23.94x** floor, `1.8270x → 1.4772x`), and **7,414.4 ms** (`base classic-on`, **10.66 %** of wall, **13.33x** floor, `3.2197x → 2.8765x`), where `base classic-on`'s **5.346 s** per-layer saving is **4.88x** its **1.096 s** non-prepass requirement (`1.0508x → 0.8033x` at canonical prepass). Probe reverted, `probe.patch` preserved, `cargo xtask build-guests --check` exit 0, and [158](issues/158-needs-support-fast-predicate.md) graduated. Evidence: [`evidence/t157-needs-support-predicate/`](evidence/t157-needs-support-predicate/FINDINGS.md) (`verify.py --self-test` exit 0).
- [Prepared-region memo across the fill sync](issues/156-prepared-memo-across-fill-sync.md): **resolved 2026-11-15, DROP** — preserving the memo across `sync_perimeter_infill_areas_into_slice` is exact (8/8 byte-identical to t151's active set) and removes the redundant derivation (process CPU lower in 8/8), but the quiet 5-pair wall gate fails: benchy Arachne-on **−2.94 %** and base Arachne-on **+0.17 %** against the 0.80 % floor. The two failing cells are supports-on Arachne, where the saved work was not on the critical path; the same-arm null band is **9.52 % wall / 0.75 % CPU** any-pair absolute, so only base Arachne-off's **+10.09 %** is outside it. A 5-pair extension that began at 85 % load was invalidated and preserved as `reduced/loaded-window.json`; production code/tests reverted, patch preserved.
- [The `Layer::Infill` first-dispatch block and base classic-on's 1.096 s residual](issues/155-infill-stage-block-base-on-residual.md): **resolved (2026-11-14; Phase-1 attribution + probe — the block is the prepared-region derivation, paid twice per layer, and the second payment is redundant)** — a five-file env-gated probe splits every module bracket on base classic-off/on. **The block is `prepare_dispatch_region_views` → `prepare_slice_regions` → `derive_needs_support`: 97.5 % of the first-Infill bracket (p50 52.4 ms of 53.6 ms) on base-off and 97.3 % (53.3 of 54.8 ms) on base-on.** All named alternatives are measured out: the whole dispatch path is ~0.95 ms (1.8 %) — pool lease 5 µs, linker 196, instantiation 83, region push 58, export call 593, teardown 25 — and `LayerStageInput` is 5 µs; the arena memo itself works (every non-first module reads the prepared slots in 5 µs, 3.0–4.0 ms per module over 496 calls). **Paid twice** because `sync_perimeter_infill_areas_into_slice` takes and re-stages the slice after every perimeter commit (three per layer) and each `set_slice` clears the memo; **the second derivation is provably redundant** — hashing the fields `prepare_regions` reads (region order/count, `region_id`, `object_id` length, `nonplanar_surface` presence, every polygon coordinate) before and after the sync gives **0 changed of 1,488** on base-off, base-on and the decline arm. **Stage-sharing re-proven at HEAD:** declining `raft-default` moves the block to `gyroid-infill` (24,533.1 ms, p50 50.0 ms), others stay ≤3.5 ms, hash unchanged (`9bb18f0e…`). **`Layer::Support` separated:** `tree-support`'s prep_views is 5 µs; its 85.9 ms p50 bracket on base-on is the in-module render (85.4 ms across linker→call→teardown), a work-set/phase difference, not a host block. **Composition:** 94.3–94.4 % of the derivation is one `intersection_ex(region.polygons, footprint)` per layer; exactly **one** non-empty overhang footprint exists per object per layer (max 1 over 1,488 calls) and the answer was **true in all 992/992 and 1,488/1,488** measured calls; base-on evaluates it a third time in the prepass eligibility pass (1,488 calls, 72.8 s thread), supports-off only twice. The value is delivered (the guest adapter `__slicer_adapt_slice_regions` calls `needs-support()` per region), so deletion is a contract change, not a free win. **Price (ceilings = thread / t154 parallelism; walls of record carried; floor 0.008 × wall):** redundant Infill-side derivation **25,986.3 ms / 26,101.2 ms** thread → **2.382 s** base-off (**12.15x** the 196 ms floor, 36.5 % of the per-layer thread, 21.5 % of the 11.10 s cut) and **2.335 s** base-on (**4.20x** the 556 ms floor, 23.1 % of the phase, **2.13x** the 1.096 s requirement); indicatively base-on moves **1.051x → ~0.943x** at canonical prepass cost (an estimate, not a claim). **Verdict:** removable — proposed candidate, not adopted: preserve the prepared memo across the fill sync, clear on the modifier-footprint path where the region list can change. Graduated [t156](issues/156-prepared-memo-across-fill-sync.md) with the standing eight-cell gate. Disclosed: no wall claim, no ratio of record, no load sample, no accelerated arm, no design for a cheaper predicate, and the pre-probe pass's 63 ms p50 is a slower box state, not a measurement of cause. Evidence: [`evidence/t155-infill-stage-block/`](evidence/t155-infill-stage-block/FINDINGS.md) (`probe.json` eight runs, `summary.json`, `probe.patch` reverted with `git apply --check` clean; `verify.py` PASS 9/9, `--self-test` 9/9 mutations rejected; restored tree's guest freshness exit 0 and the restored binary reproduces `9bb18f0e…`).
- [Benchy per-layer program: which carrier holds the wall, and what does canonical do differently?](issues/154-benchy-per-layer-carrier-attribution.md): **resolved (2026-11-14; Phase-1 attribution + arithmetic re-derivation — the benchy per-layer program is margin, not route)** — eight instrumented ordinary passes at HEAD (`71c03a3e`, binary `e1b8fa69…`) are clean and byte-identical to t151's active set; the split is prepass 63.5–82.4 %, per-layer 14.4–29.6 %, postpass 2.6–6.5 %, outside 0.1–5.3 %, with a per-layer host gap of 0–56 ms. **At canonical prepass cost 7/8 cells win** (benchy arachne-off 0.668x, base arachne-off 0.698x, benchy classic-off 0.740x, base classic-off 0.743x, benchy classic-on 0.857x, benchy arachne-on 0.901x, base arachne-on 0.967x) and **base classic-on is the only loser at 1.051x, needing 1.096 s = 9.6 % of its per-layer phase**; the required prepass cut is **64.3–84.5 %**, largest on benchy arachne-on/base classic-on/base arachne-on. This **supersedes t128/t129's benchy loss** (those paired t126's per-layer phases with t124's Orca walls) and moves the per-layer requirement to **base classic-on**. **Carriers:** supports-off perimeters **46.8–55.1 %** of per-layer thread (77–98 % in-module self fuel; `offset2_ex` the only scope at 21.8–22.8 %), first `Layer::Infill` dispatch **17.6–37.3 %** (stage-shared; t127's "prepared-region view" name fails at HEAD because `ensure_prepared_perimeter_source_regions` materialises that view in `Layer::Perimeters` and `Layer::Infill` hits the memo), `infill-linker` **4.4–20.5 %** (99.8–100 % self fuel); supports-on by `tree-support` **36.7–47.8 %**, a work-set/phase difference because canonical generates support in `PrintObject::_generate_support_material` (one whole-object pass). **The requested per-op split does not exist at the current instrumentation surface** — named scopes explain ≤ 22.8 % of any carrier's fuel, and that is the finding: no plumbing fat. The t126→t154 per-layer fall (**−21.6 % to −40.1 %** on benchy) is measured but **not attributed**. Graduated [t155](issues/155-infill-stage-block-base-on-residual.md) (AFK). Evidence: [`evidence/t154-benchy-per-layer/`](evidence/t154-benchy-per-layer/FINDINGS.md) (`verify.py` PASS, 11/11 negative controls).
- [Seam candidate generation on the printed loops: authorize the per-layer relocation?](issues/153-seam-loop-relocation-ruling.md): **resolved (2026-11-14; human ruling — DECLINED, the relocation is structurally impossible and was already decided)** — the human's instruction was to verify the prior attempt before trying it. **ADR-0046** (status accepted) evaluates the same two candidate homes and rejects the per-layer `seam-placer` as impossible: per-layer modules are re-instantiated per call and run in parallel across layers (ADR-0045, packet 102), so a per-layer module can never see two layers; the only sanctioned cross-layer conduit is one-way and prepass-first (ADR-0020's `SeamPlanIR` → host-injected `resolved_seam`). **Packet 168** discovered the constraint during grounding and recorded it as an architecture constraint ("All cross-layer machinery MUST live in the `PrePass::SeamPlanning` stage; `seam-placer` only consumes"). The take adds the stronger fact: `build_seam_candidates` (`modules/core-modules/seam-planner-default/src/visibility.rs`) scores **every contour vertex of every layer** and its per-layer terms read the **previous** layer's contours (`prev_dist`) as well as the current layer's (`curr_dist`), so candidate generation and scoring — not just the chaining — are cross-layer; the builder is dormant by design because packet 178 moved production candidates to per-region `SliceIR` geometry. Moving generation onto the printed loops is therefore circular (the prepass runs before the loops exist) and needs a two-phase pipeline or a host-orchestrated pass, both already rejected in ADR-0046's alternatives and both architecture work past this route. **S3 closes as "not a clean deletion and not relocatable"; the 1,248 ms is the price of the cross-layer aligned seam and stays in the fog with that blocker.** No exception spent, no `nearest`/`rear`/`random` relocation (the matched job is `aligned`, so it would buy nothing), no production edit, no measurement, no reference-set movement.
- [Seam inset offset: is the prepass offset redundant with the per-layer loop projection?](issues/152-seam-inset-offset-redundancy.md): **resolved (2026-11-14; Phase-1 + candidate screen — not redundant, S3 is not a clean deletion)** — removing `offset_polygons_batch` + `project_point_onto_inset_boundary` from `run_region_planning_entries` (`modules/core-modules/seam-planner-default/src/lib.rs`) and emitting the raw chosen point moves **456 of 496** base-classic-off layers (+1,177 / −1,149 printed points, 142 layers with a changed total `E`; first difference in the layer-0 **Skirt**) and breaks the module's own tests (**31/0 baseline vs 29/2 candidate**). The baseline reproduces the t151 reference `9bb18f0e…`; the candidate is `4265d619…` (+1,158 bytes / +28 lines). **The offset term is 1,248 ms** (`PrePass::SeamPlanning` **1,779 → 531 ms**, 70.15 %), not t129's 1,587 ms / 92.4 %, and is **5.9x** the 0.80 % floor / **~10.5 %** of base classic-off's current required cut. The per-layer `seam-placer` re-projects onto the wall loops but does **not** preserve the seam because the source point differs; the canonical loop projection needs candidate generation on the printed external-perimeter loops, which exist only in the per-layer phase — a **stage relocation** needing a t81-style exception, graduated [t153](issues/153-seam-loop-relocation-ruling.md). No production change; patch preserved. Evidence: [`evidence/t152-seam-inset-offset/`](evidence/t152-seam-inset-offset/FINDINGS.md) (`verify.py` PASS, 4/4 mutations rejected).
- [Skirt hull input: fold the brim area's contour, not the bead centreline](issues/151-skirt-hull-brim-contour.md): **resolved (2026-11-14; measured fairness fix — the skirt hull folds the brim area's outer contour, the matched job moves 6/8 cells, the brim is provably untouched, and the supports-off adhesion oracle reaches 4/4 from 3/4)** — `SkirtBrim::generate` (`modules/core-modules/skirt-brim/src/lib.rs`) extends `brim_points` from `area[i].contour` (the `outer − inner [− support]` result already in hand) and drops the fold of the emitted beads' `path.points`, matching canonical `Print::_make_skirt`'s `m_objectBrimAreasByInstance[...]` `area.contour.points` (`make_brim` fills that map after the no-brim subtraction). **Measured radial move: 0.178–0.180 mm on every moving cell** (half the 0.35708 mm flow spacing), brim bbox identical in all six, and the brim→skirt step **4.962–4.965 → 5.140–5.144 mm** against t146's Orca **5.139** and prediction **5.143**. **Output:** baseline reproduces t148's set in **8/8**, candidate moves **6/8** and is byte-identical across the ordinary and accelerated arms in **8/8**; the new 16-cell set is the currency ([`reference.json`](evidence/t151-skirt-hull-contour/reference.json)). **The pre-declared "all eight move" is falsified and disclosed:** the two base supports-on cells are inert because the first-layer support footprint clips the whole brim area (`area` empty ⇒ `brim_points` empty), and the audit proves **zero moved sections / zero changed `E` lines** there rather than relying on hash equality. **Delta audit 8/8:** `Skirt` is the only moved section in the six moving cells and every motion line outside it is identical with `E` stripped; the literal "`|dE|` outside `Skirt` is zero" clause is falsified at **≤ 0.00098 mm on 3–35 of up to 508,651 lines** (accumulator re-rounding, t144's magnitude), disclosed not waived. **Oracle:** benchy classic-off **1.32 → 0.05 %**, benchy arachne-off **1.34 → 0.04 %**, base classic-off **2.89 → 0.37 %**, base arachne-off **3.45 (MISS) → 0.94 %** — **4/4**, 4/8 overall; brim rel-diffs unchanged in all four, residual = the ~0.10 mm brim-extent term plus ≤ 0.04 mm Arachne width, exactly t146's decomposition. **A/B:** 3 interleaved pairs × 8 cells, every median (−4.70 … +6.43 %) inside a **measured ±12.86 % wall / ±6.98 % CPU** same-arm null band, no wall claim (t103's ~235 ms surface). **Falsifier:** `skirt_hull_folds_the_brim_area_contour_not_the_bead_centreline` pins `x_max = 22.5` vs the centreline's `22.3` and was verified red on the pre-change library. **Drive-by, not caused:** `integrated_parity_skirt_brim_tdd` (t142's red) now authors the two declared keys it omitted and passes. No route term moved. Evidence: [`evidence/t151-skirt-hull-contour/`](evidence/t151-skirt-hull-contour/FINDINGS.md) (`verify.py` PASS, 16/16 mutations rejected).
- [Variable-width anchor: per-segment envelope or the direct contour?](issues/146-variable-width-anchor-shape.md): **resolved (2026-11-13; premise refuted by measurement — the last supports-off oracle miss is a skirt-hull input defect, not a variable-width anchor error, and neither offered shape can pass the cell)** — the oracle's relative difference is `abs(pnp - orca) / orca` (`oracle_skirt_brim.py::rel`) and carries **no direction**: every supports-off cell is an **undershoot** (PnP prints less brim and a smaller skirt), so t144's "−5.65 % → +2.89 %" is a magnitude history, corrected in place. Arachne adds **+0.56 points** on base (2.89 → 3.45 %) and **+0.02** on the benchy (1.32 → 1.34 %), and the dominant term is shared with the classic cell, where the t144 reconstruction is exact by construction. **Mechanism:** canonical `Print::_make_skirt` folds `m_objectBrimAreasByInstance[...]` `area.contour.points` — the brim **area's outer contour** — into the skirt hull, while `SkirtBrim::generate` folds the emitted brim beads' **centrelines**; the ring distance already matches (outermost 4.964 mm both). The gap is half a flow spacing: measured brim→skirt bbox step **5.139 mm** on Orca against **5.143** predicted and **4.963** on PnP (re-derivable from `growth_mm`, now carried by `evidence/t144-brim-anchor/reduced/summary.json`). **Decomposition of the ~0.29 mm radial deficit: ~0.10 mm brim-extent term (inside its 12 % band) + 0.1786 mm skirt-hull term + ≤0.04 mm Arachne width.** Envelope and direct contour are both refuted as cell passes (≤0.04 mm and an anchor that is already exact on classic); graduated [t151](issues/151-skirt-hull-brim-contour.md), read as inside t140's "`skirt_height` layer hulls" exception, estimated to take base classic-off to ~0.4 % and base arachne-off to ~0.9 % and the supports-off oracle **3/4 → 4/4**. No wall claim (~235 ms surface). **Glossary:** "Anchor" added.
- [MeshAnalysis work set: which outputs does production read, and what is the unread half worth?](issues/149-meshanalysis-work-set-readers.md): **resolved (2026-11-13; Phase-1 attribution — the stage is 85.68 % a bridge detector with no production reader, and the t129 S2 per-layer-overhang-polygons port is answered as not the lever)** — `classify_object` (`crates/slicer-core/src/algos/mesh_analysis.rs`) makes **4,563.2 ms** of the stage's **5,325.8 ms** on base classic-off in `find_bridge_clusters` + `compute_bridge_metrics`: **7,383,702** directed half-edge insertions into a `HashMap`, **3,199** BFS clusters, one footprint union per cluster, anchor-width/span metrics. The only production-read output is `OverhangRegion::xy_footprint` (**529.1 ms**), via `SliceRegionView::derive_needs_support`; `OverhangRegion::needs_support` is dead (the SDK re-derives), `surface_groups` resolves a `nonplanar_surface` production only writes as `None`, and `bridge_regions`' readers are the blackboard size estimate, visual debug, an undeclared prep `surface_objects` read, and a test guest. Sub-phase base-off: classify 187.0 / footprint 529.1 / eligibility 0.2 / bridge 4,563.2; base-on 4,832.9 with bridge 4,150.2; benchy-off 195.1 with bridge 95.1. **S2 is not the lever:** the port would replace 529.1 ms and leave 762.6 ms before canonical's per-layer diff. Route: **12.95 %** of the base-off cell, **16.18x** the 0.80 % floor, **24.96 %** of its indicative cut (2.078x → 2.387x, cut 51.88 % → 58.11 %); base-on 4.85 % / 6.07x; benchy-off 2.57 % / 3.21x. Graduated [t150](issues/150-meshanalysis-bridge-retirement.md) with a 16-cell byte-identical gate. Evidence: [`evidence/t149-meshanalysis-readers/`](evidence/t149-meshanalysis-readers/FINDINGS.md) (`verify.py` 43 claims, 4/4 negative controls; `probe.patch` non-landing; production restored).
- [Supports-on plane carve: align the planner's collision occupancy with the print plane](issues/148-supports-on-plane-carve.md): **resolved (2026-11-13; fairness fix landed — the planner carves emitted roles against the print plane the host's exact-Z gate validates, published as an additive `print_plane_occupancy` carrier (schema 2.1.0). All four supports-on cells go from 101 / 101 / 44,474 / 44,474 diagnostics to zero, the 33 dropped layers reappear with none lost, support path rises 36.70 m base / 0.295 m benchy, supports-off is byte-identical 4/4, and the re-derived ratios of record are 2.705x / 2.829x / 3.356x / 3.248x.** Coverage clause read twice: plane presence 20/20; polygon coverage 11/19 base, 0/1 benchy, disclosed and not used as a gate because the retained-layer control reads the same near-zero. No causal wall-cost claim (the window's Orca arm moved too). Evidence: `evidence/t148-print-plane-carve/` (`verify.py` 360/360, 9/9 negative controls; `reference.json` is the new currency).
- [Supports-on output is support-incomplete: what the `exact-Z occupancy` body rejections drop](issues/147-supports-on-body-rejection-attribution.md): **resolved (2026-11-13; Phase-1 attribution — the rejections are a planner/validator plane mismatch, not redundant demand, and the four supports-on ratios are supports-incomplete)** — all 33 rejected entries overlap the **print plane** and **none** overlaps the midpoint payload the planner carved: benchy L170 **5.8342 mm² (2.276 % of the body)**, base 32 entries **8.4e-6–1.4706 mm²** (sum **6.8920 mm²**, median 0.0805 mm², ≤ 0.069 % of body). The 44,474 base diagnostics are **(body, demand) pairs**: **2,892** unique demands and **zero** uniquely dropped. The loss is the layer — each rejected entry is the only entry at its layer, so **32 complete base layers** and **1 benchy layer** are absent from the emitted G-code: **36,466.736 mm (10.2765 %)** of base support path and **192.601 mm (0.233 %)** of benchy path by neighbour interpolation; **19/32** base and **1/1** benchy overhangs above a missing layer have Orca support at that plane and none in PnP (benchy candidate 68: 0 vs 36.85 % at z = 34.2; candidate 76: 0 vs 14.7–32.7 %), 13/32 are unsupported by both tools at that exact plane and are not counted. **Classification (b) defect:** the planner carves `SupportAnalysisIR::model_occupancy` at `layer.z − 0.5 h` while `validate_entry` queries `anchor_z = layer.z`, and the zero-tolerance gate drops the whole body (canonical `draw_circles` would keep 250.5 of 256.3 mm² on benchy and ≥ 99.93 % of every base body). **Ruling:** the four supports-on ratios of record are **not quotable as a matched-settings claim**; they stand only with the disclosure and no new supports-on ratio until [t148](issues/148-supports-on-plane-carve.md) closes the asymmetry. Supports-off cells have zero rejections. Evidence: [`evidence/t147-supports-rejection/`](evidence/t147-supports-rejection/FINDINGS.md) (`verify.py` checks every number from `census.json`; 6/6 negative controls rejected).
- [Benchy fixture replacement: re-record the benchy reference set and retire cross-swap benchy ratios](issues/145-benchy-fixture-rebaseline.md): **resolved (2026-11-13; measured re-baseline — the replacement is a tessellation-only re-export, the slice is not output-neutral, and the post-swap scoreboard is 1.853x–3.455x)** — the replacement `tmp/3dbenchy.stl` (`6ab57f1c…`, 11,285,384 bytes) is **225,706 triangles against the old export's 225,786 (−80, −0.035 %)**, with byte-identical bounds (|Δ| ≤ 4.05e-7 mm) and the same 240-layer / 48.0 mm schedule; re-cutting t138's five sampled planes (mid and top, inward 0.20 offset, convex hull) reproduces **all ten** old-export areas exactly at 3 decimals, and `tmp/base.stl` is byte-identical to t138's record (`b26c8c79…`). **The printed object does not move but the output does:** against the old export one output-neutral code change apart (all four **base cells byte-identical** across the two takes — the control), the outer wall changes by **−1.591 mm of 31,891.789 (−0.0050 %)** and −1e-6 of loop area with per-layer area median/p95 **0.0** and one layer over 0.1 % (L153, −0.113 %), while **internal bridge moves on L40 (z = 8.2) alone** — 554.316 → 611.614 mm (**+57.298 mm, +7.344 %**), 28.028 → 31.866 mm² (**+13.694 %**), identical in all four cells — and the **tree-support engine** moves **−1,370.181 mm (−2.238 %)** over 176 supports-on layers with its max-loop area **+1.021 %**; bytes classic-off **−812**, classic-on **−79,465**, arachne-off **+118**, arachne-on **−78,773**. **Currency:** t144's four benchy hashes (`1ae0cfa8…` / `d9fc80ba…` / `8faf02b1…` / `cd7d4a2d…`) are the active set and reproduce in **8/8** cells today (one hash across three measured runs per cell; the four base cells reproduce t144's base hashes in the same campaign), recorded in [`evidence/t145-benchy-rebaseline/reference.json`](evidence/t145-benchy-rebaseline/reference.json); t139's four (`93e21fee…` / `b3f1eccc…` / `3510e876…` / `96cdac85…`) are **retired**; t143's four are the superseded interim set. **Classification: tessellation-only at the model level, not byte-neutral at the output level** (the falsifier — a systematic cross-section, bounds, layer-schedule or outer-wall displacement — is absent). **Post-swap scoreboard (8 cells × 2 tools, one window): 1.853x–3.455x ordinary/Orca, required cut 46.03 %–71.06 %, 19.05 %–56.59 % at 1.5x** — benchy 2.043x / 2.683x / 1.853x / 2.580x (3.912 / 6.553 / 3.674 / 6.686 s against 1.915 / 2.442 / 1.983 / 2.591 s), base 2.078x / 3.001x / 1.984x / 3.455x; the lowest bars are the two benchy supports-off cells, the highest the two base supports-on cells. **Disclosed:** no accelerated timing arm (`cargo xtask dist --accelerated` still fails today, `c1xx` C1083 on `'\\mimalloc-static.cc'`; a manual-cfg host is barred as a timing artifact by `docs/23`), every PnP supports-on run is `degraded: true` with **101** (benchy) / **44,474** (base) code-1200 `exact-Z occupancy` rejections — identical on both exports, so not swap-caused, graduated as [t147](issues/147-supports-on-body-rejection-attribution.md) — load 9 % / 12 %, three runs per cell (base arachne-on PnP spread 21.2 %), Orca hashes differ per run while bytes are constant, and no causal claim from the cross-take movement. Evidence: [`evidence/t145-benchy-rebaseline/`](evidence/t145-benchy-rebaseline/FINDINGS.md) (`verify.py` 98/98 checks, 25/25 negative controls rejected).
- [Brim anchor reconstruction: feed `make_brim` the slice contour from the wall path](issues/144-brim-anchor-reconstruction.md): **resolved (2026-11-12; measured fairness fix — the anchor is the reconstructed contour, the matched job moves 6/8 cells, and the adhesion oracle reaches 3/4 supports-off / 3/8 overall)** — `SkirtBrim::generate` (`modules/core-modules/skirt-brim/src/lib.rs`) offsets the filled outer-wall footprint outward by **half the per-object modal non-zero `Point3WithWidth.width`** (Round, 0.0125 mm; every-zero falls back to `line_width`) before the existing 0.1 mm gap / flow-spacing quantisation / `tryExPolygonOffset` steps, and offsets each `skirt_height > 1` layer hull likewise before it joins the skirt hull (convex hull commutes with the Minkowski sum; that path is unit-tested and disclosed as not exercised by the matched job). Module-only, no `SliceIR`/WIT change, support clip untouched; the fourth scoped parity exception covers exactly these inputs. **Reference set:** the baseline reproduces the active pre-t144 set in **8/8**; the candidate is byte-identical on the ordinary and accelerated arms in **8/8** and moves in **6/8** (`1ae0cfa8…` / `d9fc80ba…` / `8faf02b1…` / `cd7d4a2d…` / `c0b9014f…` / `585c6a8e…` / `17f63e93…` / `c9fee59a…`), now the active set in [`evidence/t144-brim-anchor/reference.json`](evidence/t144-brim-anchor/reference.json). **The pre-declared "moves in all eight" is falsified and disclosed:** base classic-on and base arachne-on are byte-identical because the support footprint clips the whole brim annulus, so no brim path (or hull point) reaches the output; their delta audit has **zero** changed `E` lines. **Delta audit 8/8:** no motion outside `Skirt`/`Brim` moves with `E` stripped; max `|dE|` **0.00098 mm**. **A/B (attribution only):** 3 pairs/cell, median wall Δ **+1.84 / −2.86 / −0.55 / +0.43 %** benchy and **+1.32 / +1.63 / +1.18 / −5.58 %** base, CPU within ±1.2 %; every median inside a measured six-cell same-arm null band (**±24.82 % wall / ±4.52 % CPU**), so no wall claim. **Oracle 3/4 supports-off:** base classic-off flips from **−5.65 %** to **+2.89 %** skirt-outer and passes (brim +5.17 → +1.14 %), base Arachne-off remains **+3.45 %** (0.45 points outside the 3 % band), both benchy cells pass (+1.32 / +1.34 %); supports-on is support-engine-limited and byte-identical in this take. The uniform modal width is the pre-declared Arachne approximation, so the miss graduates [t146](issues/146-variable-width-anchor-shape.md) (per-segment envelope vs direct contour vs accept 3/8) rather than reverting the anchor. No scoreboard re-derivation: t139's **2.279x–4.187x ordinary / 2.311x–4.179x accelerated, cut 56.1 %–76.1 %** stand with the adhesion-geometry disclosure. Both freshness gates exit 0; the accelerated host remains unbuildable (`libmimalloc-sys`, t142), so the accelerated output arm ran the ordinary host with the accelerated component tree. Evidence: [`evidence/t144-brim-anchor/`](evidence/t144-brim-anchor/FINDINGS.md) (`verify.py` re-derives every claim; 13/13 negative controls).
- [Union-phase request grouping: batch the planner's union requests across four chunk-batches](issues/143-union-phase-request-grouping.md): **resolved (2026-11-12; measured KEEP — buffering depth 12 collapses the union tail, depth 16 refutes its own merge-4 projection, and base classic-on saves 9.56 s / 12.80 %)** — the planner's union phase was one request deep (109 calls × 4.4 requests, 3.73x fan-out); `EMIT_BATCH_LAYERS` 4 → 12 merges it into **37 calls of ~13 requests** and takes the probe's union wall **10,093.1 → 4,848.7 ms (−52.0 %)** and the clip core **14,158.5 → 8,934.5 ms** on base classic-on, with benchy classic-on **447.9 → 200.3 ms**. **The pre-declared 16-layer form fails its bracket:** the same 482 requests cost **46.92 s** of thread-time against **37.68 s** at depth 4 (+24 %) once sixteen layers of collision sets and the sixteen-way occupancy payload compete for cache/bandwidth, so its union wall is **5,793.2 ms** against the **4,620 ms** LPT bound; depth 8 lands on t136's merge-2 floor (5,459.7 vs 5,480.2 ms) and **12 was measured after the table and chosen** (disclosed, not pre-declared). Peak working set is lower at 8/12 than at 4 on base (−85.8 MiB) and rises only at 16 (+78.6 MiB); time-to-first-layer improves 69.7 → 56.4 s (−19.0 %). **Output byte-identical in 8/8**, byte counts equal, exit 0, shared host binary (`b238d640…`), only the planner guest differs (`68b6f811…` → `f6632de0…`); base cells reproduce t139's set exactly, benchy cells match the take's baseline on the replacement benchy export. **Paired A/B (3 interleaved pairs, 48 runs): supports-on wall +6.72 / +7.36 / +12.80 / +13.41 %** (benchy classic-on / arachne-on, base classic-on / arachne-on) at **8.40x–16.76x** the 0.80 % floor, 11/12 pairs positive; base classic-on **74.70 → 65.13 s = 9.56 s saved = 10.3 %** of its 92.5 s required cut and **2.7x** the ticket's 3,500 ms DROP bar. Supports-off: +2.64 / −0.18 / −1.92 / −3.31 % wall, all inside the same-arm null band (2.10–4.40 % wall, 0.37–4.24 % CPU) and structurally inert; the candidate was faster on both benchy supports-off cells in the single output runs. **Disclosed gaps:** the strict CPU-lower clause passes **1/4** (+0.63 % benchy classic-on; −1.26…−1.83 % on the rest, inside the ±1.88–1.91 % null CPU band — a wider fan-out burns more CPU to finish sooner), no accelerated timing arm, no Orca arm, 3 pairs per cell, and today's base classic-on baseline is 74.70 s against t139's 126.27 s cell so only paired deltas are claimed. Route: a tranche of t135 Stage 2's order, not a destination step; indicative composition t142's 3.695x × (1 − 0.1280) ≈ **3.22x**. **Fixture swap graduated:** the user-supplied `tmp/3dbenchy.stl` was replaced 2026-11-12 with a different export of the same model (240 layers / 48.0 mm), t139's four benchy hashes are retired, and the re-baseline is [t145](issues/145-benchy-fixture-rebaseline.md). Evidence: [`evidence/t143-union-grouping/`](evidence/t143-union-grouping/FINDINGS.md) (`verify.py` 200 checks, 24/24 negative controls).
- [Brim anchor: feed the slice contour (or reconstruct it) into `skirt-brim`?](issues/140-brim-contour-input.md): **resolved (2026-11-12; human ruling — the anchor gap is real printed geometry, gets its own scoped t81-style exception, and is closed module-side)** — `SkirtBrim::generate` (`modules/core-modules/skirt-brim/src/lib.rs`) anchors the brim on the `ExtrusionRole::OuterWall` **centerline**, 0.20 mm inside canonical `make_brim`'s slice contour, so the brim's inner boundary sits 0.1 mm **inside** the part outline and the innermost bead overlaps the footprint by ~0.2 mm instead of its inner edge touching it; the ring is 0.2 mm inward on every cell. [t139](issues/139-layer-plane-parity.md) measures base supports-off at **−4.3 / −4.4 %** swept and **−5.7 / −6.6 %** outer loop against the 3 % skirt band (brim −5.2 %, inside 12 %); both benchy supports-off cells pass; supports-on is the out-of-scope support engine. The human kept [t138](issues/138-layer0-cross-section-parity.md)'s classification (**not** a [t130](issues/130-brim-matched-settings-ruling.md) setting asymmetry) and granted the **fourth recorded parity exception**, scoped to the `skirt-brim` anchor inputs. Shape: **uniform half-width reconstruction** — offset the filled footprint outward by half the resolved outer-wall width (the loop's modal point width, fallback `line_width`; the repo's own `boundary_offset = 0.5 x width` convention) and the `skirt_height` layer hulls likewise; module-only, no SliceIR/WIT change, staged to a variable-width envelope or the direct contour only if a base Arachne cell still misses. Ceremony: fresh 16-cell reference set, delta audit, four supports-off oracle cells, 3-pair A/B as attribution only, **no** scoreboard re-derivation. Stop rule: t139's fairness-fix form (geometry correctness + bounded audit; the oracle is a measure, not a revert gate). Oracle ceiling **4/8**; route arithmetic unchanged (skirt-brim ~235 ms, no wall claim). Graduated [t144](issues/144-brim-anchor-reconstruction.md). No production edit in this take.
- [Planner clip-batch fan-out: why is the clip core at 4.81x and what scheduling change reaches 12x?](issues/136-planner-clip-batch-fanout.md): **resolved (2026-11-12; Phase-1 attribution — the carve batches are already at 11.56x and the union phase is one request deep)** — the clip core is **15,841.8 ms** on base classic-on (p2 15,938.2 / p3 15,370.0), splitting additively into **6,587.6 ms** work floor (`thread/12`, 41.6 %), **7,224.0 ms** tail-times-chunking (45.6 %), **144.8 ms** dispatch excess (0.9 %), **1,393.7 ms** marshal+decode (8.8 %) and **491.7 ms** guest CPU (3.1 %); end-to-end fan-out **4.99x**, which is t134's 4.81x from a faster run. **The phase contrast is the finding:** `carve_run` issues **65,233 requests** over 312 calls at the saturated 256-request cap (**p50 256** per call) and runs at **11.56x** (40,977.6 ms thread in 3,544.7 ms); `union_run` issues **482 requests** over 109 calls (**p50 4.4** per call) and runs at **3.66x** (37,237.5 ms in 10,176.2 ms) while owning **64.2 %** of the clip-core wall. Its summed per-call `max(thread/12, longest request)` is **10,168.3 ms** against a **10,176.2 ms** wall — **8 ms** of the phase is anything but each call's longest request — so 7.6 of 12 threads are idle by construction: a **parallelism-width** deficit, not imbalance, and re-ordering cannot help. **Candidates:** the 256 cap has **no headroom** (carve is 11.56x against 12.0x, ≤ 0.13 s), dispatch excess is refuted (**0.14 s**), guest assembly is refuted (**0.49 s**, of which `carve_build` is 305.5 ms), marshal is real but below the tranche bar (**1.4 s**, operand locality). **Chunk size is the lever**, through the *phase's* request count: merging the union phase across 2 / 4 / whole-phase chunk-batches takes its floor from 10,176.2 ms to **5,480.2 / 3,465.0 / 3,103.1 ms**; the 16-layer form recovers **6,711.2 ms = 94.9 %** of the 7,073 ms tail gap, LPT-bounded to a **5,556–6,711 ms** saving = **7.3 %** of base-on's 92.5 s cut = **6.6x** the 1,010 ms floor. Benchy reproduces at 17x smaller scale (910.2 ms clip core, carve 11.18x, union 3.71x, 294.0 ms saving). **GO (narrowed)** for union-phase request grouping under the standing [t137](issues/137-planner-scheduling-gate.md) four-cell form, exact by construction; graduated [t143](issues/143-union-phase-request-grouping.md), whose first check is memory (`EMIT_BATCH_LAYERS` 4 → 16 quadruples the buffered collision sets). **NO-GO** on the ticket's three named mechanisms as the explanation, and **NO-GO** on splitting heavy union requests without its own work-set ruling. No production edit; probe reverted. Evidence: [`evidence/t136-clip-fanout/`](evidence/t136-clip-fanout/FINDINGS.md) (`verify.py` 155 checks, 27/27 negative controls; six probe G-code hashes equal the post-t139 reference set).
- [Planner scheduling gate: apply the t117 four-cell form to t135/t136?](issues/137-planner-scheduling-gate.md): **resolved (2026-11-12; human ruling — extend the t117 four-cell form to t135 Stage 2 and t136, and generalize it as the standing rule for any representation-safe planner-only candidate where supports-off is structurally inert)** — on the four supports-off cells the whole `PrePass::SupportGeometry` stage is **0.64x–0.81x** the 0.80 % floor and the planner module is **0.32x–0.43x**, so no planner-only change can meet the literal all-8-cell 0.80 % gate. Any representation-safe planner-only candidate (`PrePass::SupportGeometry` / `PrePass::SupportAnalysis` / `com.core.tree-support-planner`) where `enable_support: false` declines before the candidate's work now runs under the standing four-cell form without needing its own gate ticket: positive 0.80 % paired wall/CPU improvement in the four supports-on cells, byte-identical output in all eight against the active reference set ([t139](issues/139-layer-plane-parity.md)'s set), and no median wall/CPU regression in the four supports-off cells. "Require a supports-off leg" and "Park the planner scheduling program" are both declined; the 8/8 destination is unchanged. [t135](issues/135-cross-layer-occupancy-batching.md) Stage 2 and [t136](issues/136-planner-clip-batch-fanout.md) are unblocked on the frontier. No production edit in this take.
- [Overhang band partition deletion: derive the quartile from the distance the consumer already has](issues/142-overhang-band-partition-deletion.md): **resolved (2026-11-11; measured BUILD — the partition is deleted, the quartile is distance-derived, and the matched job's G-code is byte-identical)** — `annotate_overhangs` returns only `overhang_areas` + `prev_layer_boundaries` (Shape A: the per-layer `difference_ex` key test stays; Shape B was rejected because populating the boundary on every layer would flip `overhang_distance_mm` from `None` to `Some` with no measured benefit), and `PerimeterSpatialContext` takes the producer's band width instead of band polygons; `overhang_quartile = overhang_quartile_from_distance(signed_distance, width)` with an epsilon at the 0.0001 mm Clipper quantisation. The band R-tree, `indexed_overhang_quartile` and `legacy_overhang_quartile` are deleted; classic and arachne measure the distance once and stamp both fields. `SurfaceClassificationIR` gains `overhang_areas` + `overhang_band_width_mm` (production leaves the band polygons empty), the scalar rides the WIT `slice-region-view.overhang-band-width-mm`, `extra_perimeters_on_overhangs` reads the new area map, and the visual-debug renderer reconstructs bands at render time. **Output byte-identical in 8/8 cells** and equal to t139's reference set, stable across 48 A/B rows. **Paired A/B: 26.25 / 11.76 / 25.58 / 9.69 / 20.00 / 10.56 / 20.23 / 12.66 % wall saved**, candidate faster in **24/24** pairs, CPU lower in **8/8**; indicative composed ratios **1.818x–3.697x**, required cut **45.0–73.0 %**. Disclosed gap: no accelerated **timing** arm (driver × MSVC verbatim-path bug on `libmimalloc-sys`); accelerated **output parity** was checked with a disclosed manual `--cfg` host + accelerated guests (8/8 same hashes). Evidence: [`evidence/t142-overhang-band-deletion/`](evidence/t142-overhang-band-deletion/FINDINGS.md) (`verify.py` re-derives every number; `--self-test` 4/4 controls).
- [Overhang quartile bands: is the 4-band polygon partition read by anything?](issues/141-overhang-quartile-band-deletion.md): **resolved (2026-11-11; Phase-1 attribution + price — the quartile VALUE has no production reader, the only read is a presence bit equal to the sign of the distance the consumer already computes, and the partition is the largest unpriced shell shape)** — stamped at `expolygon_to_path3d_indexed` / `expolygon_to_path3d` / `arachne-perimeters`, and every other production site copies it or tests `.is_some()`; `overhang-classifier-default` computes speed from `overhang_distance_mm` (`calculate_speed`), and `overhang_areas` has no reader in `modules/`. The four bands' union is exactly `current \ previous`, so presence = outside the previous boundary = `signed_distance > 0` (raw, not the `+0.5 x width` stamp), against the very same `prev_layer_boundary`. Price on the post-t139 tree: base classic-off **8.964 / 9.558 / 10.583 s** (22.1–26.5 % of the 39.9–40.6 s wall, 34.3–40.5 % of the 26.15 s cut), base arachne-off **9.450 s**, base classic-on **11.195 s**, benchy classic-off **0.954 s**, benchy classic-on **0.817 s**; **3 offsets + 6 booleans per band layer** over 486/496 and 239/240; 11.6–11.7x thread→wall reproduces t84. Two forks: keep the per-layer boundary key (one `difference_ex`, ≥85.4 % of the stage still saved) or populate every layer and prove inertness; and re-home the visual-debug band tap. Not a route step alone (2.899x → ~2.19x on base classic-off). Graduated [t142](issues/142-overhang-band-partition-deletion.md). Evidence: [`evidence/t141-overhang-bands/`](evidence/t141-overhang-bands/FINDINGS.md) (`verify.py` 7 rows / 3 sources; `--self-test` 4/4 negative controls).
- [Layer-plane parity: sample at the canonical midpoint](issues/139-layer-plane-parity.md): **resolved (2026-11-11; the plane is canonical, the reference set is re-recorded, and the route arithmetic is unchanged)** — `PrePass::Slice` + the host schedule + the planner terminate/sample on `compute_slice_z`'s band midpoint; base **495 → 496 layers / 99.0 → 99.2**, benchy 240/48.0; layer-0 hull fits the STL mid section on all 8 cells (base classic 0.932 vs mid 0.955 / top 3.165 = **3.31x** separation; arachne 0.757 recorded). **Four modifier-volume slice sites** moved onto the same plane (region mapping, paint segmentation, support enforcer/blocker, Tier-2 staging) to keep the `modifier_sub_region_id` hashes equal. **Slicer defect fixed:** `push_unique_intersection` deduped by coincident coordinates, collapsing base's 69.3 mm section to 0.0025 mm² (correct 671.2054) and dropping the layer; topology-only dedupe restores all 496. **Reference set:** `93e21fee…` / `b3f1eccc…` / `3510e876…` / `96cdac85…` / `ca14d055…` / `585c6a8e…` / `d8fdc1d9…` / `c9fee59a…`, arms identical; baseline reproduces t135. **Oracle 2/8** (base supports-off skirt −4.3 % / −4.4 % swept, −5.7 % / −6.6 % outer loop, brim +8.1 % → −5.2 %; supports-on = support engine). **A/B** wall 0.923x–0.984x, CPU 0.963x–0.991x. **Scoreboard** **2.279x–4.187x ordinary / 2.311x–4.179x accelerated, cut 56.1 %–76.1 %** (supersedes t135 with the machine-state disclosure). DEV-201 closed; DEV-202 filed (p241 agg, calicat sites). Graduated [Brim anchor: feed the slice contour into `skirt-brim`](issues/140-brim-contour-input.md). Evidence: [`evidence/t139-layer-plane-parity/`](evidence/t139-layer-plane-parity/FINDINGS.md) (`verify.py` 7 claim groups, 22/22 negative controls).
- [Layer-0 cross-section parity: do the two arms print the same first layer?](issues/138-layer0-cross-section-parity.md): **resolved (2026-11-10; Phase-1 attribution — the residual is a slice-plane convention, the base skirt is its second-order consequence, supports-on is the support engine)** — PnP slices every layer at its **top** (`first_layer_height + n × layer_height`), canonical OrcaSlicer at the layer **midpoint** (`compute_slice_z` = `0.5 × (lo + hi)`, ZAA off). Slicing the same STL at top/0.20 and mid/0.21 and comparing translation-invariant hulls puts PnP on the top plane (0.02–1.36% error) and Orca on the mid plane (0.02–4.02%) on all ten sampled layers; base L0/L10 separate the candidate planes by **253% / 161%**. On `base.stl` the 0.1 mm shift is **3.42x** of layer-0 area (3.122 vs 0.914 mm²); on the benchy **1.3%**; base is **495 layers / Z 99.0** on PnP vs **496 / 99.2** on Orca. **Classification:** base footprint = slice plane (same-plane wall-width effect 3.3%); base-off brim +8.09% = +88.04 mm contour − 27.65 mm anchor (predicted +60.39 vs observed +57.39); base-off skirt +3.23% = brim footprint +8.68% diluted by the offset; base-on skirt +12.48% and benchy-on brim +54.51% = support engine (Orca support path 10.6–11.7x denser, coverage 10.4–11.5x; skirt tracks the support-dominated input hull); run counts 22/30, 19/39, 0/0 = cosmetic splitting; first-layer height equal. **Oracle: 2/8; no 8/8 without a slicing change** — ceiling 6/8 without one (perfect support parity), expected 4/8 with the plane fix alone, 8/8 needs both; the strict form is an engine-level acceptance. **Price:** fairness fix, not a route step — base layer-0 operand 5.329 → 2.314 mm² (2.30x) on 1 of 495 layers, benchy 1.01x on 1 of 240; no wall claim, 59.1–73.5% cut unchanged; the price is the 16-cell re-take, delta audit and scoreboard re-derivation, plus the multi-height midpoint decision. Graduated [Layer-plane parity](issues/139-layer-plane-parity.md). Evidence: [`evidence/t138-layer0-cross-section/`](evidence/t138-layer0-cross-section/FINDINGS.md) (`verify.py` 8 claim groups, `--self-test` 12/12 controls).
- [Staged fold: canonical skirt-brim re-take and cross-layer occupancy offset batching](issues/135-cross-layer-occupancy-batching.md): **Stage 1 resolved (2026-11-10) — the canonical skirt/brim port lands, the adhesion job and its flow model are matched on both arms, and the new reference set replaces t123's; the strict parity oracle is 2/8 with every residual attributed outside the module** — brim = contour-anchored annulus (`make_brim` / `tryExPolygonOffset`) clipped against the first-layer support *region*, skirt = hull of the first `skirt_height` layers plus the brim footprint (`Print::_make_skirt`), both at the canonical flow spacing `line_width - h x (1 - pi/4)`, at `40ecc66f78` with the OrcaSlicer attribution header; module keys `brim_object_gap = 0.1`, `min_skirt_length = 0` (non-zero fails closed) and `first_layer_height`. **Seven matched keys**, three of them re-derived: `brim_type = outer_only` (`auto_brim` overrides `brim_width`), `initial_layer_line_width = 0.4` (**Orca resolved 0.5 against PnP's 0.4**, so 8 mm quantised to 7.31 vs 8.0 mm — the t82 "0.4 both sides" claim is falsified by the appendix) and `tree_support_brim_width = 0`. **Output:** baseline reproduces t123; candidate records `7db0641b…` / `c02aa4e9…` / `a2f7a1fe…` / `2326455d…` / `7ef84447…` / `1ddea2b7…` / `defae354…` / `4aa1847c…`, identical across arms. **Delta audit:** all motion outside `Skirt`/`Brim` identical with E stripped, |dE| <= 0.00098 mm. **Oracle:** benchy supports-off passes (+2.4/+2.6 % brim work, +1.3 % brim area, 22 lines); base supports-off is 22/22 lines at +8.1/+7.3 % brim work with skirt +3.2/+3.1 % against a 3 % band (anchor convention); supports-on is support-engine dominated (0/0 brim on base, skirt 12.5 %; Orca's benchy brim 39 runs vs 19). **A/B:** wall 0.963–1.046, CPU 0.992–1.026 — below the noise floor. **Scoreboard:** 2.445x–3.769x ordinary / 2.420x–3.745x accelerated, cut 59.1 %–73.5 %, superseding t124. **Stage 2 resolved (2026-11-12): the cross-layer occupancy batch is a measured KEEP** — the chunk's 434 per-layer `+support_object_xy_distance` offsets (t134's **0.99x** serial half) are issued as **109 `offset_polygons_batch` calls / 434 requests** over the `EMIT_BATCH_LAYERS = 4` chunk with the memo seeded from the batch, exactness by construction; the batch's own fan-out is **3.52x** on both base supports-on cells (18.6 s request time → 5.29 s wall) and **2.39x/2.24x** on benchy, replacing the 4.81x proxy. **Output byte-identical 8/8** to t139's set on both build arms (accelerated guests hybrid with the ordinary host — the accelerated host build still fails on `libmimalloc-sys`). **Paired wall +2.93 / +7.22 / +10.80 / +13.31 %** at **3.66x–16.64x** the floor, 19 of 20 supports-on pairs; base saves **9.76 s / 12.37 s**. **CPU is flat, not lower** (+1.09…−1.34 %); an identical-arm null control bounds the box at **±11.02 % wall / ±5.60 % CPU**, so the strict CPU-lower form is disclosed **0/4** and the gate reads CPU as *not contradicted*. Supports-off is inert (`enable_support: false` returns before `plan_for_object`) at **−0.59 / +0.44 / +2.69 / −0.35 %** wall, all inside the null band; strict sign form disclosed **2/4**, base arachne-off settling −3.06 % → −0.35 % at 24 pairs. Route: **10.6 %** of base classic-on's 92.5 s cut, **13.2 %** of arachne-on's; indicative base classic-on composition **3.695x → ~3.30x**. Evidence: [`evidence/t135-occupancy-batch/`](evidence/t135-occupancy-batch/FINDINGS.md) (`verify.py` re-derives every number, 8/8 negative controls).

- [Matched settings vs the 8 mm brim: close the asymmetry or keep it disclosed?](issues/130-brim-matched-settings-ruling.md): **resolved (2026-11-09; human ruling — close both adhesion/priming asymmetries by canonical port + shared authored values, staged into t135; disclosure declined)** — the job keeps an 8 mm brim and a 6-loop / 3.0 mm / 1-layer skirt on both arms (`brim_width = 8`, `skirt_loops = 6`, `skirt_distance = 3.0`, `skirt_height = 1`, asserted per run fail-closed both sides; the Orca pin amends `--brim-width=0` → `8` plus three skirt overrides, and the t103 "2.4-era" doctrine re-scopes to "the matched job"), and `com.core.skirt-brim`'s bounding-box rectangles are replaced by canonical geometry ported from the pinned `v2.5.0` = `40ecc66f78` (`make_brim` (`Brim.cpp`) + the `Print.cpp` skirt construction, OrcaSlicer attribution header). Fact base: PnP printed 20 brim + 6 skirt loops where Orca's chain resolved `skirt_loops = 0` / brim `0` — two asymmetries, one surface, ~235 ms of PnP wall. Standing policy: **close asymmetries by default before any new ratio of record; parity work licensed only to close asymmetries** (third recorded exception in Out of scope; route-wide lifting declined, repo ADR declined). Sequencing: staged fold into [t135](issues/135-cross-layer-occupancy-batching.md) (a fused single candidate rejected as the t115 failure mode) — Stage 1 (unblocked) is the re-take recording the **new reference set** (t123's set retired as the currency) with its own old-vs-new paired attribution, Stage 2 is the batch under [t137](issues/137-planner-scheduling-gate.md)'s gate ruling, and the closing gate is the 8-cells × 3-tools scoreboard re-derivation superseding t124's ratios (until then they stand as historical and the disclosure rides every scoreboard). No production edit in this take.
- [Planner effective parallelism](issues/134-planner-effective-parallelism.md): **resolved (2026-11-09; Phase-1 attribution — NO-GO as the route, GO (narrowed) for two supports-on tranches)** — the planner stage runs at **2.64x / 2.66x / 2.09x / 2.11x** on the four supports-on cells (**2.76x / 2.79x / 2.31x / 2.32x** module-only), up from t131's 2.19x because t132 removed 14.2 s of serial duplicate occupancy; shell is 9.26x, overhang 11.72x, SupportAnalysis 11.46x on the same base classic-on run. The **105,053 ms** stage thread-time is **96.3 %** layer-independent emit pass, **2.5 %** recurrence-bound drop loop, **1.2 %** pre-loop; the `drop_nodes` barrier is 2,402 ms of a 37,639 ms module. Inside the emit pass, `p6d_occupancy` is **14,249 ms wall / 14,169 ms thread = 0.99x** (433 unique offsets) and `p6a_clip_core` is **17,811 / 85,641 = 4.81x**. Ceilings: occupancy batch **11.2 s** at 4.81x (8.7–13.0 s over 2.6x–12x) = **11.1x** floor = **12.1 %** of base-on's 92.5 s cut; clip-to-12x **10.7 s** = **10.6x** = **11.5 %**; combined **21.9 s = 21.7x = 23.7 %**, still below the ticket's 50.66 s threshold. **The literal all-8-cell gate is structurally unreachable:** the whole `SupportGeometry` stage is **0.78x / 0.64x / 0.80x / 0.81x** the 0.80 % floor on supports-off and the planner module is **0.32–0.43x**. Even a zero-cost planner leaves **3.25x / 3.21x / 2.89x / 2.87x** on the supports-on cells against t124's Orca medians. Graduated [t135](issues/135-cross-layer-occupancy-batching.md) (exact occupancy batch) and [t136](issues/136-planner-clip-batch-fanout.md) (clip fan-out diagnosis), both blocked by the human's gate ruling [t137](issues/137-planner-scheduling-gate.md). Evidence: [`evidence/t134-planner-parallelism/`](evidence/t134-planner-parallelism/FINDINGS.md) (`verify.py` 94 checks, 12/12 negative controls; eight probe G-code hashes equal t123's candidate-ordinary set; no production edit, probe reverted).
- [Internal-bridge offsets on a candidate-bbox operand](issues/133-internal-bridge-windowed-offsets.md): **resolved (2026-11-09; exact measured KEEP — window the negative erosion only, 8/8 at 1.66x–10.94x the floor)** — `internal_bridge_offset_window` clips the `deep_infill_area` operand of the `-4.5 x spacing` erosion to the union bbox of the layer's `internal_solid_fill` candidates grown by `(2 x 4.5 + 1) x qualify_spacing`; both consumers (`qualified.retain`, the `internal_unsupported_area` filter in `internal_bridge_angles`) are candidate-local and the window boundary is outside the erosion's `2 x 4.5 x spacing` miter reach, so the covered set is unchanged. **16-cell output:** 8 cells x 4 arms, one hash per cell, all equal to t123's reference set (`90638bf3…`, `711c6334…`, `2ba5d84c…`, `a972b0f2…`, `b3b4ac5e…`, `4e31a2db…`, `6b865d4a…`, `62d6eedc…`), so the delta audit is equality — **zero moved lines**. **The first candidate windowed both offsets and is rejected:** confined to the bridge role (0 structural geometry lines) but moves **8 benchy bridge layers** by up to **22.09 mm** and 5 base bridge lines, because `internal_bridge_angles` intersects the whole-layer sparse `expansion_area` with `deep_infill_clip_area` and samples that anchor set; the artifact and both audits are preserved. **Paired gate:** **8/8** at **1.66x–10.94x** the 0.80 % floor with CPU corroborating — benchy **4.83 → 4.64 / 8.84 → 8.71 / 5.01 → 4.87 / 8.92 → 8.79 s**, base **42.51 → 39.82 / 102.45 → 101.09 / 43.82 → 39.98 / 106.95 → 102.18 s**. **Sequential sampling:** 3 pairs per cell, extended to 6 for benchy arachne-on and 12 for base classic-on (pre-declared; all pairs retained; 12-pair base-on **+1.36 s, 1.66x floor**, 7/12 wall-positive, 12/12 CPU-positive). **Ledger correction to t131:** the erosion converts thread-time to wall at **2.80x / 3.96x / 3.27x / 6.42x**, not the stage's 5.25–8.78x. Route arithmetic: **9.7 %** of base-off's 27.7 s required cut and **1.5 %** of base-on's 92.5 s — a banked keep, not a destination step. Disclosures: box load **21.2 %** before / **10.9 %** later (DWM/Chrome/Xbox/Discord), 6 physical / 12 logical cores; three windows; no Orca arm. Evidence: [`evidence/t133-windowed-bridge-offsets/`](evidence/t133-windowed-bridge-offsets/FINDINGS.md) (`verify.py` 357 checks, 17/17 negative controls).
- [Planner occupancy memo: serve the duplicate half of the singular offset imports from a content cache](issues/132-planner-occupancy-memo.md): **resolved (2026-11-09; measured BUILD — the duplicate half is serial and worth 12–13 % of both base supports-on cells)** — a per-layer content-keyed memo (`OccupancyMemoEntry` + `occupancy_offset_key` + `inflate_model_occupancy_memoized`) serves the role carve's `+support_object_xy_distance` offset from the emit check's when the payload is byte-equal, with a payload comparison on a hash hit so the memo is exact rather than probabilistically exact. Probe census reproduces t131 exactly: **864 calls / 433 distinct / 431 hits** on base classic-on and **365 / 186 / 179** on benchy classic-on. **16-cell output:** every one of the 8 cells × 4 arms emits one hash and it is ticket 123's reference set (`90638bf3…`, `711c6334…`, `2ba5d84c…`, `a972b0f2…`, `b3b4ac5e…`, `4e31a2db…`, `6b865d4a…`, `62d6eedc…`); the host binary is identical between arms and only the planner guest differs. **Paired gate:** supports-on **4/4** at **15.78x / 7.39x / 14.97x / 16.54x** the floor — benchy classic-on **9.33 → 8.15 s (+1.18 s, +12.62 %)**, benchy arachne-on **9.17 → 8.62 s (+0.54 s)**, base classic-on **119.45 → 105.14 s (+14.31 s, +11.98 %)**, base arachne-on **119.88 → 104.02 s (+15.87 s)** — with CPU **+1.11 / +0.48 / +13.25 / +19.50 s**. **Supports-off:** inert by construction (`enable_support: false` returns from `SupportPlanner::run_support_geometry_with_analysis` before `plan_for_object`), so the mixed-sign medians (+0.04 / −0.07 / +0.05 / +0.40 s wall) are noise inside a 0.7 s / 4.9 s spread; the strict sign read is **6/8** and is recorded. **Ledger correction to t131:** the duplicate half converts at **1.006x** its thread-time (**14.31 s**, not the 6.5 s priced at the stage's 2.19x), because the duplicate occupancy calls were serial; benchy classic-on saves **1.93x** its 0.61 s thread-time, mechanism unmeasured. Route arithmetic unchanged: **15.5 %** of base classic-on's 92.5 s required prepass cut. Evidence: [`evidence/t132-planner-occupancy-memo/`](evidence/t132-planner-occupancy-memo/FINDINGS.md) (`verify.py` 176 checks, 11/11 negative controls rejected; both freshness gates exit 0).
- [Common-stage per-op cost decomposition](issues/131-common-stage-per-op-cost-decomposition.md): **resolved (2026-11-09; Phase-1 decomposition — the per-op wrapper is not the lever, the sweep is 98.2–99.9 %, and operand size is the gap)** — probe seams in `polygon_ops`/`host`/`prepass`/`pipeline` split every common stage into WIT marshal / IR→Path64 conversion / Clipper2 sweep / PolyTree rebuild; the non-sweep wrapper is **0.19 %** of base-off primitive thread-time (**543 of 280,657 ms**) and **0.21 %** of base-on (**1,075 of 517,928 ms**), so the ticket's first gate branch fails by two orders of magnitude. Operand volume: shell **19.5 calls/layer at 4,608 points/call** (**44,463,545 points**, 89,825/layer), planner **290 calls/layer at 2,040 points/call** (**293,219,629 points**, 592,363/layer), kernel at parity (t70). **S5** priced at **22,964 → 1,937 ms** thread on 130 base-off layers (16.06x / 8.81x), wall **2,520 ms** (7.4x floor), but **not exact** (115/130 + 36/42 layers, max 0.71 / 0.59 mm², net −1.27 / +1.18 mm²). **S6** priced exact: `+0.35 mm` occupancy is **864 calls / 433 payloads**, duplicates **14,227.7 ms** thread → **6,504 ms** wall (6.4x floor), while the `−0.6/−0.8 mm` collision family is **1,350/1,350 distinct** (no hits). **Scheduling:** planner **2.19x** effective parallelism (benchy-on 1.68x) against 8.34–11.80x elsewhere → **25,625 ms** at 4x on base-on (27.7 % of its required cut). Gate: first branch fails everywhere (0.9–2.6 % of each stage's share), second fails literally (S5 17.7 %, S6 12.8 %, planner-at-4x 50.6 %), but the route-level stop rule does not fire. Go/no-go: **NO-GO for a per-op rewrite; GO (narrowed) for operand locality and planner scheduling.** Graduated [S6 memo](issues/132-planner-occupancy-memo.md), [S5 windowed offsets](issues/133-internal-bridge-windowed-offsets.md), [planner parallelism](issues/134-planner-effective-parallelism.md). Evidence: [`evidence/t131-per-op-decomposition/`](evidence/t131-per-op-decomposition/FINDINGS.md) (`verify.py` 150/150 checks + prose; 11/11 negative controls; four cells × two flag sets, every hash equal to the t123 candidate-ordinary set). No production edit; probe reverted; `build-guests --check` exit 0.
- [Prepass work-set and per-op parity against canonical v2.5.0](issues/129-prepass-work-set-per-op-parity.md): **resolved (2026-11-08; Phase-1 decomposition — the work set is not the lever, per-op cost / operand locality is)** — canonical does *more* per-layer Clipper work than PnP in the shell cluster (≈45–60 vs ≈15–25 ops/layer; PnP omits `process_external_surfaces`' wave expansion, `DEV-200`), yet its **whole prepass** is **15.7x / 9.8x** cheaper on base classic-off / -on, so PnP does not already compute canonical's work set at canonical's per-op cost and the kill condition is not met. The Rust Clipper2 kernel is at parity (t70), so the gap is above the kernel: canonical bbox-clips (`clip_clipper_polygons_with_subject_bbox`), caches collision/avoidance by `{radius, layer}`, and uses per-vertex `LinesDistancer` queries; PnP's hot ops are whole-layer and its planner occupancy is uncached (32.7 s / 864 calls, t113). The clean PnP-local deletions (overhang bands 9,689 ms, whole-mesh facet union 4,260 ms, seam inset 92.4 % of 1,718 ms) are **15.5 s = 55.8 % of base-off's 27.8 s cut** and **1.6 s = 17.5 %** of base-on's — a first tranche, not the route. **Prepass at canonical cost wins base classic-off only (0.787x), ties base classic-on (1.012x) and loses both benchy cells (1.053x / 1.075x); full geometry parity wins all four (0.335x / 0.661x / 0.550x / 0.704x).** Go/no-go: **GO for the engine rewrite, NO-GO for a work-set-only rewrite**; the next gate is [common-stage per-op cost decomposition](issues/131-common-stage-per-op-cost-decomposition.md). Evidence: [`evidence/t129-prepass-work-set-parity/`](evidence/t129-prepass-work-set-parity/FINDINGS.md) (`verify.py` 4 cells / 6 shapes / 20 claims; 8/8 negative controls; 14 citations at `40ecc66f78`). No production edit.
- [Destination arithmetic after t123: re-aim, rewrite, or park?](issues/128-destination-arithmetic-ruling.md): **resolved (2026-11-08; human ruling — keep 8/8 and authorise an engine rewrite, gated on a work-set/per-op parity decomposition)** — the target is unchanged and the route is re-scoped: the prepass is the only term that can buy the 58–79 % cut (62.1–87.1 % of every cell; required removal 72–101 %), **benchy classic-off is unwinnable by prepass deletion alone (1.02x at zero prepass)**, and every remaining prepass stage is clipper-bound with its named shapes already screened (t75/t84/t113/t125/t53/t55/t76), so the 0.80 %-per-shape machinery would need 72–99x its floor. The rewrite runs the prepass first (supports-off cluster, then the planner), then the benchy-classic-off per-layer residual, under t111's prepass exception carried to the implementation; the first gate is [the work-set/per-op parity decomposition](issues/129-prepass-work-set-per-op-parity.md), whose kill gate returns the destination here if PnP already matches canonical's work set and per-op cost. Canonical is pinned at `v2.5.0` = `40ecc66f78` (2026-11-07) in the local checkout, disclosed against the installed 2.5.0 binary (2026-11-06, not byte-proven identical). The brim asymmetry is graduated as [Matched settings vs the 8 mm brim](issues/130-brim-matched-settings-ruling.md). No production edit in this take.
- [Raft-fill dispatch outside the raft band: decline the no-op per-layer call](issues/127-raft-fill-dispatch-outside-band.md): **resolved (2026-11-08; measured DROP — 2/8 cells pass)** — output byte-identical in all eight cells on both build arms and every guest module identical between arms; the dispatch really disappears (**495 → 0** calls on each base cell, **240 → 0** on benchy) and the call count is the only thing that moved. The 61–63 ms the module was charged is the **`Layer::Infill` stage's shared prepared-region view materialisation**, paid by whichever Infill module is dispatched first: with raft-default declined, `com.core.gyroid-infill` pays the same p50 (**60 / 58 / 61 / 57 ms** against **56 / 60 / 66 / 66 ms**) on the same layers, and the per-layer phase wall does not fall. The gate passes only benchy classic-on (+1.97 %) and base classic-on (+3.06 %); ticket 117's narrower gate also fails 4/8. **Ledger correction to t126:** its 61–63 ms and 2.64x–8.25x "ceiling" are not a saving, and its 17-layer "other module first pays 0 ms" control cannot be a dispatch-order claim. Candidate and tests retired; patch preserved. Evidence: [`evidence/t127-raft-band-dispatch/`](evidence/t127-raft-band-dispatch/FINDINGS.md) (`verify.py` 528 checks; 18/18 negative controls).
- [Post-t123 phase split refresh and per-layer/emit attribution](issues/126-post-t123-phase-split-refresh.md): **resolved (2026-11-08; measurement + Phase-1 attribution — the split is re-derived on all eight cells, and even a zero prepass loses benchy classic-off at 1.02x)** — eight instrumented passes (Arachne measured for the first time), all clean and byte-identical to the t123 reference set. Prepass **62.1–87.1 %**, per-layer **10.6–33.3 %**, postpass **1.8–4.9 %**; `PrePass::Slice` fell **41,472 → 2,110 ms** on base classic-off (**−94.9 %**), so the supports-off prepass is now `ShellClassification` + `OverhangAnnotation` (**24.4 s of 42.5 s**) and the supports-on prepass is `SupportGeometry` + `SupportAnalysis` (**68.9 s of 126.3 s**). Deleting the per-layer phase leaves **1.67x–4.14x**; deleting the prepass reaches **0.48x–1.02x**; the required prepass removal is **72–101 %**. The per-layer phase has no globally idle wall (0 ms gap on base; 2.0–2.3 % on benchy). Carriers: perimeters (3.82x–15.84x floor ceiling), tree-support supports-on (4.66x–9.63x), raft-default (2.64x–8.25x), infill-linker (1.17x–8.28x); the postpass is not a route. Graduated [raft-fill dispatch outside the raft band](issues/127-raft-fill-dispatch-outside-band.md) (61–63 ms on all 495 base layers while the body early-outs) and the HITL [destination-arithmetic ruling](issues/128-destination-arithmetic-ruling.md). Evidence: [`evidence/t126-post-t123-phase-split/`](evidence/t126-post-t123-phase-split/FINDINGS.md) (`verify.py` 617/617 claims; 10/10 negative controls).
- [Strong planner occupancy-offset batching: representation-safe four-cell campaign](issues/125-strong-planner-occupancy-offset-batching.md): **resolved (2026-11-07; measured DROP — 2/8 cells pass)** — per-origin role-carve offsets batched inside the bounded emit flush, output byte-identical in all 16 arm/cell combinations and all 80 output/paired runs clean, but six cells fail the amended gate; base classic-on gains +6.13 % wall with CPU regressing 1.7656 s, while benchy classic-on regresses −17.90 % wall. Candidate and WASMs retired, production restored, 148 planner tests passed. Evidence: [`evidence/t125-occupancy-offset-batching/`](evidence/t125-occupancy-offset-batching/FINDINGS.md) (`verify.py` 442 checks; 10/10 negative controls).
- [Strong planner carry: fold the occupancy batching into the Slice campaign, amend the gate, or park it?](issues/117-strong-planner-carry-ruling.md): **resolved (2026-11-07; human ruling — amend this representation-safe candidate's gate)** — the original Slice carrier is already banked, so the occupancy-offset batch proceeds with the 0.80 % paired wall/CPU gate on the four supports-on cells, byte-identical output in all eight, and no median wall/CPU regression on the four supports-off cells; the 8/8 destination is unchanged. Candidate graduated as [Strong planner occupancy-offset batching: representation-safe four-cell campaign](issues/125-strong-planner-occupancy-offset-batching.md).
- [Post-t123 Orca scoreboard](issues/124-post-t123-orca-scoreboard.md): **resolved (2026-11-07; measurement-only — PnP loses all 8 cells at 2.36x–4.65x ordinary / 2.40x–4.67x accelerated)** — fresh 8 cells × 3 tools × (1 warmup + 3 measured), interleaved, on the post-t123 tree; **zero starvation exclusions**; 96 rows, PnP ordinary ≡ accelerated bytes in 8/8, every PnP hash equal to t123's candidate set, 64/64 PnP `degraded=false` / `non_fatal=0`, 32/32 Orca runs pinned (arc disabled, elephant-foot 0, brim 0) with the repaired filament leg `1.24 / 12 / 220`; required cut **58%–79%** of today's PnP wall (**72%–86%** for 1.5x). Every ratio improved against t121's pre-t123 **2.78x–5.11x**, but the Orca side also moved (**−12.3% … +11.5%**), so no causal claim is taken; the base supports-off PnP deltas (**−24.2% / −25.7%**) agree with t123's paired **−22.2% / −20.9%**. The first batch was killed by the host mid-cell: its `base classic-on` partial is discarded whole and that cell plus `base arachne-on` were re-taken in interleaved batches. Evidence: [`evidence/t124-post-t123-scoreboard/`](evidence/t124-post-t123-scoreboard/FINDINGS.md) (`verify.py --self-test` exits 0 checkout-only, 10/10 negative controls rejected).
- [Facet bridge-path retirement: canonical classification over the whole unsupported set](issues/123-facet-bridge-path-retirement.md): **resolved (2026-11-07; BUILD — 8/8 cells clear the 0.80 % floor at 3.4x–27.8x, CPU corroborating)** — `PrePass::Slice` retires the facet point-in-polygon scan, the 1.0 mm `expansion_margin_mm` offset, the flat-family special case, the postpass unsupported-span gate and the bottom-surface footprint cache, and applies canonical `PrintObject::detect_surfaces_type`'s `opening_ex(diff(current_raw, safety_offset(previous_raw)), 0.1 × external_perimeter_width)` to the region's whole unsupported set (`classify_unsupported_bridge_areas`). **16-cell output:** baseline ordinary ≡ baseline accelerated ≡ the post-t116 set in all eight cells, candidate ordinary ≡ candidate accelerated, candidate moves every cell; benchy **+873 / +832 / +1,281 / +1,159** bytes, base **−20,294 / −20,255 / −20,767 / −20,745**. **Delta audit: 0 structural geometry lines in all eight cells** — wall/support differences are `E`-token-only on unchanged `G1` motion or non-motion commands (`M73`, `M106`/`M107` fan, appendix); no coordinate, feedrate, travel, skirt, brim or raft line moved. **Paired gate (3 interleaved pairs):** median wall **−7.60 / −3.04 / −7.02 / −2.73 %** (benchy classic-off/on, arachne-off/on) and **−22.23 / −7.23 / −20.94 / −7.94 %** (base), **3.4x–27.8x** the floor, process CPU **−1.0 … −110.2 s** in every cell; all eight strictly separated on wall and CPU. **Ledger:** the base win is mostly the removed facet work (1.04x set at 27x the canonical classifier); the benchy set grows 1.85x; the removed footprint is an extra saving the t122 screen did not price. `DEV-200` records the unported expansion/trim; `DEV-039` is closed. Evidence: [`evidence/t123-facet-retirement/`](evidence/t123-facet-retirement/FINDINGS.md) (`verify.py` exits 0 checkout-only, 309 checks; `--self-test` rejects 11/11 negative controls).

- [`PrePass::Slice` parity Phase-1: wave expansion and the facet-path retirement](issues/122-prep-slice-wave-expansion-facet-retirement.md): **resolved (2026-11-06; split verdict — STOP on the combined expansion shape, BUILD the classification-only retirement)** — canonical expands bridges into `stInternalSolid` / `stInternal` / `stTop` **after** perimeters and fill surfaces exist (`LayerRegion::process_external_surfaces`), but PnP's sparse zone and `perimeter.infill_areas` are created only at the `Layer::Perimeters` commit partition; `PrePass::Slice` has none of the three zones. A faithful port is therefore a per-layer change, not a prepass one. The probe reconstructs production inputs (fidelity exact on six sampled layers; canonical area 1,306.72 / 4,653.28 mm² reproduces t112's 1,307.30 / 4,654.62; flat 447.16 / 438.76 reproduces t116's 447.27 / 438.92). Classification-only net **+1,122.9 ms** benchy / **+94,852.4 ms** base; the expansion costs **1,151.9 / 4,976.6 ms** = **102.6 %** of the benchy saving and 5.2 % of the base saving. **The classification-only shape clears the 0.80 % floor in all eight cells at 12.5x–24.5x (benchy) and 89.7x–213.9x (base); the combined shape is negative in all four benchy cells**, so the all-8-cell gate fails. Delta is bounded: benchy added 618.44 / removed 18.45 mm², expansion reach ≤300.64 mm²; base added 564.17 / removed 384.82 mm², reach ≤513.85 mm². **Ledger correction:** the current facet path's pre-gate polygon sum is 14,232.05 / 419,106.22 mm² and at most 5.0 % / 1.1 % survives the gate — on base it produces an almost identical set (1.04x) at 27x the canonical classifier's cost, and `classify_region_surfaces` is 18,275.8 ms of that cost on its own (the `is_bridge` boolean is re-derivable). The classification-only candidate is [ticket 123](issues/123-facet-bridge-path-retirement.md). Evidence: [`evidence/t122-wave-expansion/`](evidence/t122-wave-expansion/FINDINGS.md) (`verify.py` exits 0 checkout-only; `--self-test` rejects 14/14 negative controls).

- [Post-merge per-layer dispatch regression: localize and remove the per-call cost](issues/121-post-merge-per-layer-dispatch-regression.md): **resolved (2026-11-06; the per-layer cost is removed, and the first post-merge Orca scoreboard is recorded at 2.78x–5.11x ordinary)** — the per-layer transport was deriving `prepare_regions` for every module call even though the arena had already computed the same pure function for the staged slice, and it synthesized the prepared view for `Layer::PerimetersPostProcess` even though that stage is outside the prepared-region contract. Reuse the arena record when the perimeter-source projection is the identity and every projected origin is staged; pass `derive_prepared=false` for the non-prepared stage. Four classic cells, 3 interleaved pairs: CPU **−27%…−46%** in **12/12** pairs, wall −9.3%…−37.5% in 11/12 (one +0.66% wall with CPU −37.23%; cell median −14.29%), all four G-code hashes identical. Per-layer base-off **42,406 → 6,758 ms**, below the 7,265 ms pre-merge bracket. The complete 8-cell scoreboard (Orca + PnP ordinary + PnP accelerated, 1 warmup + 3 measured) has ordinary ≡ accelerated bytes in 8/8 and PnP losing all 8 at **2.78x–5.11x ordinary / 2.76x–5.10x accelerated**; the destination's 8/8 is not met, and the required cut is 64%–80%. Ledger correction: the t115 output hashes predate t116; the current references are `35600ee4…` / `596f6da5…` / `6a6a3d9d…` / `8f46fe92…`. Evidence: [`evidence/t121-per-layer-dispatch/`](evidence/t121-per-layer-dispatch/FINDINGS.md) (`verify.py` 8/8 controls; `verify_scoreboard.py` 6/6).

- [Lightning corpus consolidation: share the 5-case captures across the 10 tests](issues/119-lightning-corpus-consolidation.md): **resolved (2026-11-05; corpus shared — 41 → 32 active invocations, 688.89 s → 620.44 s serial module wall, −9.94 %, every assertion preserved)**: a process-global `OnceLock<Corpus>` holds one observation per `(case, arm)` (5 cases × 2 arms = 10 runs) plus its lifecycle trace; the five `fresh_consumers` visual legs, the legacy/comparator/retirement positive captures and the selection external-artifact run read it. The runtime trace emitted exactly 32 starts (`run_visual` 13 / native 5 / preparation 6 / full slice 8), matching the source inventory. The saving is smaller than t114's pre-cache ~423 s estimate because t118 had already made each capture cheap (≈7.6 s of wall per removed capture here), so this is claim hygiene and aggregate CPU, not a large wall lever. Audit fixes: the typed-projection `Arc::ptr_eq` scene check now requests two layers (it was vacuous at one); the test-local selection Boolean is dropped; `max_retained_after_disposal` returns `Option` and cleanup requires `Some(0)`; a real cross-owner `InvalidOwner` control replaces the mislabelled foreign-owner block; the zero-slice selection predicate and store controls moved to `tests/contract/lightning_selection_contract_tdd.rs` (2 tests, 0.00 s). The box was **not idle** (43 % CPU sampled, browsers running), the arms are same-session not interleaved, a second after run measured 611.64 s, and no full-bucket wall or per-test split is claimed. Evidence: [`evidence/t119-lightning-corpus/`](evidence/t119-lightning-corpus/FINDINGS.md) (`verify.py` exits 0 checkout-only; `--self-test` rejects 7/7 negative controls).

- [`PrePass::Slice` candidate: canonical flat-bridge classification](issues/116-prep-slice-canonical-bridge-classification.md): **resolved (2026-11-05; BUILD — 8/8 cells clear the 0.80 % floor at 11.1x–30.5x, CPU corroborating)**: the PnP-local enclosure discriminator is gone, replaced by canonical's `opening_ex(flat_bridge, 0.1 × external_perimeter_width, Miter, 3.0)` over PnP's flat family, canonical's `ApplySafetyOffset` (1-unit lower-layer inflate) and the `lower_layer == nullptr ⇒ stBottom` guard; the `flat_bridge_closing_join` key is retired end to end. **16-cell output:** baseline ordinary ≡ baseline accelerated ≡ the t115 post-merge reference set, candidate ordinary ≡ candidate accelerated, candidate moves every cell. **Delta audit: 0 structural geometry lines in all eight cells** — the wall/support deltas are extrusion-token-only (same `G1` motion, different `E`) on bridge-flagged segments, plus the retired config appendix row and `M73` counters; no coordinate, feedrate, travel, skirt, brim or raft line moved. **Paired gate (3 interleaved pairs):** median wall **7.23 → 5.79 / 11.58 → 10.21 / 7.27 → 5.94 / 11.75 → 10.12 s** (benchy classic-off/on, arachne-off/on) and **106.83 → 83.98 / 176.41 → 160.73 / 106.39 → 80.39 / 185.47 → 164.41 s** (base), i.e. **−8.9 % … −24.4 %**, **11.1x–30.5x** the floor, with process CPU **−16.6 … −288.9 s** in every cell. Narrowest multiple base classic-on at **11.1x**; one disclosed wall-range overlap there (0.55 s). Seven of eight cells strictly separated on wall, all eight on CPU. **Ledger corrections:** the baseline check moved from the post-t81 set (the ticket's pre-merge wording) to the t115 **post-merge** set, which is what HEAD produces; and the retirement of `flat_bridge_closing_join` leaves **no declared key carrying `wire_type: "enum"`**, so the scheduler manifest's enum-metadata assertions retire with it. Disclosures: the campaign was interrupted twice (session recycle mid-pair-2, then a requested pause), so pairs 1–2 and pair 3 come from two windows ~4 h apart — each pair is internally paired; host load was not independently measured (the slicer saturates its own 12 threads); no Orca arm, so no ratio of record — the post-merge scoreboard remains [ticket 121](issues/121-post-merge-per-layer-dispatch-regression.md)'s closing measurement. Evidence: [`evidence/t116-prep-slice-canonical-bridge/`](evidence/t116-prep-slice-canonical-bridge/FINDINGS.md) (`verify.py` exits 0 checkout-only, 316 checks; `--self-test` rejects 10/10 negative controls).

- [Post-merge reconciliation: re-baseline the perf keeps and audit master's semantic drift](issues/115-post-merge-reconciliation.md): **resolved (2026-11-04; the merge is output-neutral on the matched job and regresses the per-layer phase)** — all eight matched cells grow by exactly **+34 bytes**, the added config line `; prepared_plan_diagnostic = none`, and a direct line diff on the four classic cells returns that single hunk (base classic-on `22200671… → 4b6d2843…`, benchy classic-off `3672dc7e… → 39298f40…`), so the reference set moves by a disclosure line and no toolpath needs re-recording. Painted fixtures do move: `cube_4color.3mf` **−1.08 %** bytes with up to **8.19 %** of extrusion redistributed between tools (T0 −8.19 / T1 −3.18 / T2 +1.73 / T3 +5.06 %) while total extrusion moves **−0.11 %**, and `painted_seams.3mf` keeps 111,958 moves with 9,608 reordered lines — the classic paint-attribution fix and variant-chain ordering, with no painted ratio of record affected. Performance: a three-pair interleaved A/B (preserved t103 developer snapshot `4cb96942…` vs the merged ordinary build, load 71 % disclosed) puts **process CPU up in 12/12 pairs** (+24.11 / +16.96 / +53.57 / +34.88 % medians) and the cost entirely in the **per-layer phase** (2.23x / 1.91x / 6.02x / 5.08x; instrumented base-off 7,265 → 42,886 ms = 5.90x, prepass flat at −0.3 %); call counts are unchanged while `fuzzy-skin` goes 162 → 56,068 ms and ten modules land at ~28.5 s, with the per-call cost rising 55 → 141 ms across layers. The accelerated build does not show it (2,318 vs 4,807–5,860 ms, same hash), pointing at a build-mode-sensitive host path. All keeps are structurally present (t78/t81/t90/t95/t101/t102) but their prices were not re-measured. Ruling: **no ratio of record may be quoted against the merged tree**; the post-merge Orca scoreboard is deferred to the fix's closing gate; references move to the post-merge set; the regression is [ticket 121](issues/121-post-merge-per-layer-dispatch-regression.md). Evidence: [`evidence/t115-post-merge/`](evidence/t115-post-merge/FINDINGS.md) (`verify.py --self-test` exits 0 checkout-only, 403 checks, 6/6 negative controls rejected).

- [Lightning test-leg concurrency](issues/120-lightning-test-leg-concurrency.md): **resolved (2026-11-04; test-only screen — 798.61 s → 700.16 s on one warm shared-machine pair)** — `module_plan_matches_recorded_baseline` joins its WASM and native arms, `fresh_consumers_emit_baseline_segments` schedules preparation-only / visual-consumer / cancellation legs on the Rayon pool, and `typed_projection_is_atomic_and_neutral` schedules its projection / failure / empty-scene / G-code runs and joins the two G-code results before comparing. 172 passed / 1 ignored with identical rosters, calicat SHA `d12cdfd1…` unchanged, −98.45 s (−12.33 %) with browser load active, so no idle-box or statistically established effect is claimed; the 6–8 minute projection was not reached. No production code, assertion, roster entry or pipeline invocation removed. Ticket 119 remains open. Evidence: [`evidence/t120-lightning-parallel-legs/`](evidence/t120-lightning-parallel-legs/FINDINGS.md) (`verify.py --self-test` 6/6 mutations rejected).

- [Compiled-component cache in the live load path (e2e dev loop)](issues/118-compiled-component-cache.md): **resolved (2026-11-04; milestone met — full e2e bucket 755.12 s / 12.59 min < 900 s, 172 passed / 1 ignored)** — a process-global `(profile, canonical path, mtime, length)` component cache plus one shared engine per profile cuts a debug calicat slice from **11.732 s to 2.813 s warm** (15.424 s cold; byte-identical G-code `d12cdfd1…`); wasmtime's validated 15 MB disk cache makes separate `pnp_cli` processes hit; the per-layer capture loop now uses the same rayon parallelism as the ordinary path, and the lightning corpus tests run their five independent cases in parallel. Module walls: prep **~1,800 → 267.60 s**, `slice_end_to_end` **200.86 → 49.96 s**, precision **86.66 → 11.37 s**, visual capture **96.22 → 22.67 s**. Full bucket **1,282.18 s → 755.12 s** after case parallelism (21.37 → 12.59 min). Evidence: [`evidence/t118-component-cache/`](evidence/t118-component-cache/FINDINGS.md) (`verify.py` 22/22 checks, 11/11 negative controls).


- [E2E bucket latency: the full runtime e2e suite is the dev-loop bottleneck](issues/114-e2e-bucket-latency.md): **resolved (2026-11-04; Phase-1 screen — the bucket is 85 % per-slice module load, no deletion or lane split recovers it, and the carrying shape is graduated as [ticket 118](issues/118-compiled-component-cache.md))** — one debug slice of `resources/calicat.stl` through the 24 core modules is **11.732 s** median, split **1.833 s** pipeline (validation 4 / prepass 425 / per-layer 1,174 / postpass 230 ms) and **10.011 s** module load + startup + emit; `load_live_modules_for_plan_manifest_first` builds a fresh `WasmEngine` and compiles every on-disk component **serially in the binding loop** with no cache. Serial inventory of 178 tests: the non-whale modules sum to **854 s**, `lightning_migration_tdd` measured **1,411 s for 6 of 10 tests** (~39 min extrapolated) and `preparation_runtime_lifecycle_tdd` is the ticket's recorded **14-of-16-in-30-min** module (~34 min) — the two whales are **83.8 %** of the **5,263 s (≈88 min)** extrapolated suite, whose measured/cited floor is **4,065 s (≈68 min)**. `slice_end_to_end_tdd` is the counter-example: 42 tests in **201 s** because ~10 `pnp_cli` runs back them (23 cached call sites over 3 keys + 7 uncached determinism runs). A reverted 24-thread compile probe (byte-identical G-code `d12cdfd1…`) removed only **2.431 s (−20.7 %)**, so parallelising the loop is **not** the lever; a compiled-component cache is (modelled ceiling **≈ 4.5 min**, `10 s + ~145 slices × 1.8 s`). **Pruning: the first pass's "no deletion justified" was corrected by an adversarial audit** — eight claim fixes/merges adopted (a never-running disk support-blocker test revived; both test-local `scenario_2` fakes deleted and the one real `PaintSemantic` order claim moved to `slicer-ir`; a lying override test renamed; an empty-tolerant pipeline test tightened; `modifier_infill` merged into its identical sibling, **38.76 s → 26.78 s / −11.98 s**; the precision merge landed inside noise, 86.66 → 86.71 s, not claimed). The audit also estimated **~11 min** of duplicate work; the largest piece is graduated as [ticket 119](issues/119-lightning-corpus-consolidation.md) (**41 → 32** active invocations, all assertions preserved). **Lane split: not viable** (a fast lane would be ≈ 22 s of unit/model-IO tests). **The ~237 pipeline starts** are the real bloat: 10.0 s of load each ≈ **39.5 min**, so even perfect deduplication leaves ~77 min; the cache is the carrying fix. **Ledger correction:** the 40-minute baseline was **under-counted** — nine `pnp_cli`-backed tests fail in 0.1 s on the stale-binary gate under plain `cargo test`, which does not build `pnp_cli`; `cargo xtask test` owns that preflight. Evidence: [`evidence/t114-e2e-latency/`](evidence/t114-e2e-latency/FINDINGS.md) (`verify.py` exits 0 checkout-only, 30/30 checks, 12/12 negative controls rejected).

- [Strong planner Phase 1: sub-term decomposition and the occupancy-offset shape](issues/113-strong-planner-phase1.md): **resolved (2026-11-04; Phase-1 screen — stop under the all-8-cell contract, no candidate, no production edit)** — the planner's largest term is **model-occupancy inflation** (`inflate_model_occupancy`'s singular `host::offset_polygons`), **56.2 % / 46.0 % / 51.4 % / 41.4 %** of the planner on base classic-on / benchy classic-on / base arachne-on / benchy arachne-on, split almost evenly between the per-layer emit check and the per-origin role carve; the clip core is **31.0 % / 28.6 % / 35.9 % / 32.4 %**, the drop loop **6.2 % / 10.8 % / 6.0 % / 11.1 %**, contacts **2.4 % / 6.0 % / 2.2 % / 5.4 %**. Batching the offsets through `offset_polygons_batch` is exact by construction (same `slicer_core::polygon_ops::offset` per request, input-order results; t65/t78 precedent), needs **no canonical-parity exception**, and clears the **0.80 %** floor on all four supports-on cells at **12.58x / 7.44x / 7.97x / 4.76x** the floor under t78's measured **2.6x** clamp (18.73x / 11.08x / 11.87x / 7.09x ideal), with zero expected output delta (probe byte-identical to the HEAD baseline on base classic-on `4b6d2843…`, benchy classic-on `3b8fa86a…`, benchy arachne-on `d4644c14…`). **But the all-8-cell gate is structural:** the whole `PrePass::SupportGeometry` stage on the supports-off cells is **300 ms = 0.22 %** of base classic-off and **46 ms = 0.51 %** of benchy classic-off, below the floor even if deleted; the planner module alone is **141 / 23 ms = 0.13x / 0.32x**. No planner-only or support-stage shape can clear 0.80 % there, so the stop rule fired and the carry decision is graduated as [ticket 117](issues/117-strong-planner-carry-ruling.md). **Ledger correction:** t88/t107's "no host batch shape in the non-clip residual" is falsified — the ladders, offsets, MST and node moves are **6.2 % / 10.8 %** of the planner, while the occupancy offset batch is **56.2 % / 46.0 %**; the t104 `87,824 ms` is an instrumented bracket whose composition (≈49.3 s occupancy / ≈27.2 s clip / ≈5.5 s drop) is the transferable result. Evidence: [`evidence/t113-strong-planner-phase1/`](evidence/t113-strong-planner-phase1/FINDINGS.md) (`verify.py --self-test` exits 0 checkout-only, 6/6 negative controls rejected).

- [`PrePass::Slice` parity Phase 1: canonical closing vs the artefact-bound gate](issues/112-prep-slice-parity-phase1.md): **resolved (2026-11-03; Phase-1 recommendation — build P1, no production edit)** — canonical has **no enclosure closing**: `PrintObject::detect_surfaces_type` marks every unsupported bottom `stBottomBridge` after `opening_ex(diff_ex(slices, safety_offset(lower_slices), Yes), external_perimeter_width / 10)`, and `merge_bridges`' `closing_ex` is a **≈0.15–0.17 mm** hole filler on the bridge ∪ wave-anchor surface. PnP's 12 mm `closing_ex(support)` + **≥ 10 %** refill gate is a PnP-local discriminator with **no canonical counterpart and no `DEV` row**, so the parity move is to **replace** it, not to compute it cheaper. Shape **P1** = canonical opening over PnP's flat family, every surviving component kept, plus canonical's `ApplySafetyOffset` (1-unit lower inflate) and the width resolved as `resolve_contact_params` does. **Ceiling:** closings are **86.8 %** (benchy) / **70.8 %** (base) of `PrePass::Slice`'s thread-time; net ceiling **1,893 / 37,682 ms** = **15.3–24.7 % / 23.4–50.0 %** of the cell = **19.1x–62.5x** the 0.80 % floor in **all 8** cells, replacement opening **146x / 853x** cheaper. **Delta:** **182 / 239** benchy and **437 / 469** base committed layers change; net **+8.86 mm² (+2.0 %)** / **+115.90 mm² (+35.9 %)** (gate rejected **68.4 / 187.2 mm²**; 0.04 mm opening removed **59.58 / 71.27 mm²** of sub-0.08 mm features); layer 0 pays a **595.8 ms** benchy closing whose result the next stage clears. **P2** (full classification + `assemble_bridge_areas` retirement) **deferred** — canonical's wave expansion (`propagate_waves_ex`) has no PnP counterpart, so its delta is unbounded; **P3** (radius swap) **rejected** on correctness. Consumer map at HEAD adds `arachne-perimeters`, `infill-linker`, `rectilinear-infill`, packet 234a's angle inputs and paint splits to t59's list. Candidate graduated as [ticket 116](issues/116-prep-slice-canonical-bridge-classification.md). Evidence: [`evidence/t112-prep-slice-parity/`](evidence/t112-prep-slice-parity/FINDINGS.md) (`verify.py --self-test` exits 0 checkout-only, 9/9 negative controls rejected).

- [Prepass rewrite: scope, output contract and acceptance](issues/111-prepass-rewrite-scope-ruling.md): **resolved (2026-11-03; human ruling — authorized, scoped and staged)** — a **canonical-parity exception scoped to the prepass** (the second recorded exception after ticket 81's `resolution` port; the rest of the parity program stays out of scope, and the next parity question returns to the human). Both carrying shapes are covered, but **`PrePass::Slice` is the first campaign** (**41,472 ms = 51.2 %** of base classic-off, **44,558 ms = 19.5 %** of base classic-on, present in all 8 cells) and the **Strong planner** is second (**87,824 ms = 38.4 %** of base classic-on, **2,943 ms = 23.6 %** of benchy-on; graduated as [ticket 113](issues/113-strong-planner-phase1.md), blocked by 112; it may still turn out representation-safe and need no exception). Acceptance: a fresh **16-cell** reference set, candidate and baseline arms byte-identical **within themselves**, the baseline reproducing the post-t81 set, the candidate-vs-old-reference G-code delta audited (feature / TYPE / support-interface counts, byte volume, confinement to prepass-derived geometry) **before** timing, and the standing paired wall/CPU gate at the **0.80 %** floor in **all 8 cells** — no four-cell subset. First step is a **Phase-1 parity and consumer analysis**, graduated as [ticket 112](issues/112-prep-slice-parity-phase1.md); no production edit before it clears the floor with a bounded delta, and a shape that only clears base classic-off is not enough. **Staging:** the rewrite is **stage 1** of a multi-stage engine program; the 8/8 destination stays, and the per-layer/emit program is the next stage's fog (deleting both largest prepass stages still leaves base at **2.86x–4.00x** inside t104's instrumented arithmetic against Orca's quiet walls). **Ledger caveat:** t110's base-on **9.49x / 16.3x / 19.6x / 13.7x** multiples divide instrumented PnP spans by quiet Orca walls (t104: that pass ran **+43.6 %** over its t103 median; the uninstrumented paired record is **6.02x**, t106), so they are treated as upper bounds until corrected. **Postscript (2026-11-04, [t113](issues/113-strong-planner-phase1.md)):** the all-8-cell clause is structurally unattainable for the Strong planner — the whole `PrePass::SupportGeometry` stage on the supports-off cells is **0.22–0.51 %** of the cell, below the floor even if deleted — so the carry decision is [ticket 117](issues/117-strong-planner-carry-ruling.md).

- [Orca-side attribution: what its matched-job wall is made of](issues/110-orca-side-attribution.md): **resolved (2026-11-03; measurement-only — no per-stage timing channel works on this install, and the phase split that is observable reverses the map's reading of its own ratios)** — the CLI log buffers (27 stage markers in **3 bursts**, 18 of 27 together, no timestamps at `--debug` 3/4/5), ETW is refused without elevation (`wpr` policy failure / `xperf` invalid flags), and `--pipe` accepted the flag but never connected in two attempts, so the boundary is a **~15 ms directory tap** on `plate_1.gcode.tmp`. Orca's cells are **import-dominated**: startup + model import is **1,229.8 ms (65.9 %) / 1,229.8 (49.0 %) / 10,130.0 (73.3 %) / 10,130.0 (42.0 %)** of the benchy-off / benchy-on / base-off / base-on wall (1,867.5 / 2,509.8 / 13,814.5 / 24,140.3 ms), and Orca's import on the same 123 MB `base.stl` is **≥ 14.3x** PnP's (≤ 0.708 s outside-phase time). Orca's **whole slicing** phase is **637.8 / 1,280.1 / 3,684.5 / 14,010.3 ms** against PnP's **7,365 / 11,784 / 80,216 / 228,317 ms** = **11.6x / 9.2x / 21.8x / 16.3x**, so the end-to-end ratios of record (5.86x / 9.49x / 4.34x / 4.96x today) **understate the engine gap by 1.7x–3.7x** — PnP's fast import was offsetting part of a much larger slicing deficit. Prepass multiples **≥ 40x (bound) / 17.5x / 34.4x / 19.6x**, emit multiples **5.0x / 4.4x / 5.8x / 7.3x**; the prepass is the carrying phase on **both** engines (Orca 57.1 % / 74.6 % of base slicing, PnP 89.5 % / 89.4 %), and on the t105-pinned supports-on job Orca's prepass grows **+8.34 s** where PnP's support stages add **~113.9 s** (**13.7x**; the planner alone is **8.4x** Orca's whole supports-on prepass). The feature stack is nearly free for Orca: 1 wall loop, 1 shell, 0 % infill, no support = **13,639.6 ms** against 13,814.5 ms (**+0.1 s**), which kills "Orca does less prepass work". **Ledger correction to t104:** its Table 3's "Orca's whole slice (2.03 / 2.29 / 15.28 / 19.49 s)" are *process* walls, 66–73 % import on base; its shares and counterfactuals are unaffected. Disclosures: `--info` is an upper bound on import, so Orca prepass/slicing are lower bounds and the multiples upper bounds; 1.8–6.5 % repeat spread; load 0.25–0.43 system CPU; mixed vintage (PnP t104, Orca today, ~9 % faster than t103); the abandoned STL-prefix control (non-manifold truncation → `--info` repair never returned) and the failed `--pipe` attempts are recorded. Evidence: [`evidence/t110-orca-phases/`](evidence/t110-orca-phases/FINDINGS.md) (`verify.py` exits 0 checkout-only; 7/7 negative controls rejected). **Ledger caveat (2026-11-03, t111):** the base-on **9.49x / 16.3x / 19.6x / 13.7x** figures divide instrumented PnP spans by Orca's quiet walls; the uninstrumented paired record is **6.02x** (t106), so they are upper bounds until corrected.

- [Harness defects A1/A2: conditional `host-algos` and the narrow-run example](issues/109-xtask-host-algos-injection-and-narrow-run-example.md): **resolved (2026-11-02; both fixed — harness/doc take, no floor applies)** — `xtask test` injected `--features slicer-core/host-algos` on every selection, so `cargo xtask test -p <pkg>` exited 101 for the six packages that do not depend on `slicer-core`; `build_test_arguments` now injects only when the selection includes `slicer-core` (parsing `-p`/`--package`, `--workspace`/`--all`, `--exclude`, `--manifest-path`, attached forms and `name@version`), and the pure predicate carries five unit tests. `cargo xtask test -p xtask --summary` goes from exit 101 to **exit 0 / 193 passed**, `-p slicer-config` reaches its tests, and `-p slicer-core` still compiles with the feature. `AGENTS.md`'s narrow example pointed at `slicer-runtime`'s `contract` bucket, which matched **0 tests (314 filtered)**; it now names `slicer-scheduler`'s real `scheduler_contract` target and runs **9 passed, 0 failed**. Adjacent, not fixed: `slicer-config`'s census is red before and after (**293 vs 281**, plain cargo reproduces), and `--no-run --summary` prints a false build-failure line beside `VERDICT: PASS`. Evidence: [`evidence/t109-harness-defects/`](evidence/t109-harness-defects/FINDINGS.md) (`verify.py` exit 0; 5/5 negative controls rejected).

- [Dev-loop latency: the `xtask test` gate](issues/108-dev-loop-latency-preflight-concurrency.md): **resolved (2026-11-02; human-directed scope extension — KEEP, candidate committed)** — the three `xtask test` preflights (`check-literals`, `check-test-quality --report`, `build-guests --check`) were strictly serial read-only scans; they now run concurrently behind `run_preflights_with` (`xtask/src/test.rs`), with the guest-freshness check on the calling thread because `Invocation` owns a non-`Send` canonical-WIT loader closure. **Median invocation 18.771 s → 10.802 s (−7.969 s, −42.4 %)**, four interleaved pairs with frozen harness binaries, non-overlapping arms (slowest after 13.214 s < fastest before 18.438 s), pre-declared floor **5 s** cleared 1.6x; components measure 8.196/5.621/2.731 s and 9.375/7.686/4.543 s (serial sums 16.548/21.604 s), so the attainable saving is `sum − max` = 8.35/12.23 s and the end-to-end delta lands inside it. Reporting order is unchanged, so a source-gate failure still aborts before any guest rebuild. Inventory: warm no-op check/clippy 0.8–2.8 s, one-line edit to `slicer-ir`/`slicer-core` 65.3/72.0 s, **first codegen of the `contract` bucket 994 s vs 17.26 s incremental**, relink-only 12.8 s, bucket execution 99.8 s, contract binary 89,697,792 B beside a 770,166,784 B PDB. **Falsified:** check/clippy artifact split (1.7–2.4 s alternation, 0 re-checks — the earlier minutes-scale readings were a dirty test target), feature-set thrash for `slicer-runtime` (0.72 s no-op), linker share on incremental relinks, and `debug = 0` as a speedup on the incremental path (28.24 vs 17.26 s). **Adjacent defects recorded, not fixed:** A1, the unconditional `--features slicer-core/host-algos` injection breaks `cargo xtask test -p <pkg>` for packages that do not depend on `slicer-core` (`-p xtask`, `slicer-ir`, `slicer-config`, `slicer-scheduler`, `slicer-gcode`, `slicer-helpers`); A2, `AGENTS.md`'s narrow filter `core_module_ir_access_contract_tdd` matches **0 tests** (`314 filtered out`). Not priced → fog: cold-build `debug` level, cold-build linker, `cargo-nextest`, `sccache`, CI job graph/caching. **Blaze (ra-jit) was tried, not datasheeted:** #1527 has no installable Windows archive (packaging only, installer unlanded); #1591 builds here in **57m34s** and starts a daemon but needs manual ACL and `hard_link`/`read_link` workarounds, then refuses a `serde` + `serde_json` binary at **241 std instances that need the LLVM backend** (the author's LLVM build stops at the `f16`/`f128` build-script codegen); its LLVM rebuild was abandoned after **92 minutes**. **On WSL2 it runs and loses to cargo on this repository:** against cargo 1.96.0 in Ubuntu 24.04 (12 CPUs), blaze's warm edit is **23-33 ms vs 248-779 ms** on a toy crate, but on `slicer-helpers` cold build is **193 s vs 54 s**, warm edit **12.4 s vs 0.95-1.13 s** (crate root), **11.9 s vs 1.4-3.3 s** (module) and **15.8 s vs 2.4-3.5 s** (function body), and `blaze test` is **312 s vs 49 s** (all 18 tests pass under both). **No blaze speedup is claimed for this repo.** Evidence: [`evidence/t108-dev-loop-latency/`](evidence/t108-dev-loop-latency/FINDINGS.md) (`verify.py` exits 0 checkout-only, 33/33 claims, 12/12 negative controls).

- [Support-geometry producer decomposition](issues/107-support-geometry-producer-decomposition.md): **resolved (2026-11-02; measurement-only Phase-1 screen — measured-dead, the premise named the wrong stage; no candidate, no campaign, no production edit)** — `PrePass::SupportGeometry` is **two stages sharing one stage id**, and ticket 104's reducer summed them. Re-pairing t104's committed raw streams: the **host producer is 9 ms (0.0039 % of the cell, 0.0068x the 1.318424 s floor) on base classic-on and 0 ms on benchy classic-on**; the **planner module stage is 91,893 ms (40.13 %, 69.70x floor)** and **3,260 ms (26.17 %, 32.85x floor)** — `com.core.tree-support-planner` **87,824 / 2,943 ms (95.6 / 90.3 %)** plus `com.core.traditional-support-planner` 300 / 30 ms. The supports-off brackets (231 / 43 ms) are the planner merely declining with the producer at 4 / 0 ms. The producer's whole job is a **coarse outline set — 577 polygons / 444,259 points over 240 layers (~7.1 MB)** on benchy-on per `support-preview` — which is why a deep copy costs sub-millisecond to 9 ms and why the sentinel intermediate entries (exactly 1 on benchy-on) are inside the same bracket; **the entire producer is two orders of magnitude below the floor, so no shape on it can be a route** and the stop rule fired before candidate pricing. The real output is the **ledger correction**: t104's 91,902 / 3,260 ms is essentially the **planner**, i.e. [ticket 88](issues/88-supports-on-increment-attribution.md)'s 55.5 s term refreshed to an implied **64.5 s** of the 160.857 s base-on cell and **3.25 s** of the 12.403 s benchy-on cell, and the supports-on carrying shape of the destination's prepass-rewrite decision is the planner, not the geometry producer. Disclosures: no new slice pass, no wall claim (the split is a bracket property of t104's 2026-11-02 instrumented streams), no probe build (patch authored and reverted unbuilt — `target/release/pnp_cli.exe` remains `4cb96942…` on a clean tree), Arachne unmeasured, preview used for counts only. Evidence: [`evidence/t107-support-geometry-producer/`](evidence/t107-support-geometry-producer/FINDINGS.md) (`verify.py` exits 0 checkout-only, 71/71 claims, 9/9 negative controls rejected).

- [Post-pin supports-on scoreboard re-derivation](issues/106-post-pin-supports-on-scoreboard.md): **resolved (2026-11-02; measurement-only — no PnP run, no config change, no production edit)** — with Orca pinned to `tree_strong` the four supports-on ratios are **6.02x / 6.11x** (base classic / arachne) and **4.77x / 4.53x** (benchy classic / arachne); the park still needs **66.9–75.5 %** cuts. Every post-pin Orca median is slower than ticket 103's pre-pin one (**+37.04 / +14.49 / +13.74 / +5.69 %**), so all four ratios fall from the archive (8.25x → 6.02x, 7.00x → 6.11x, 5.43x → 4.77x, 4.78x → 4.53x). The t105 transfer **agreed on two cells and missed two**: base arachne-on measured **26.365 s** against a predicted **32.135 s** (**−17.95 %**, tolerance 4.670 s), benchy classic-on **2.600 s** against **2.503 s** (**+3.89 %**, 0.004 s past a 0.093 s tolerance). The misses are **vintage, not the pin**: the pinned job is **byte-identical to t105's `tree_strong` arm on all four cells** (44,803,778 / 43,802,018 / 9,481,130 / 10,331,360 B, **+23.46–35.34 %** over t103's pre-pin volumes; support-interface sections 423 / 426 / 57 / 58) yet runs **1.9–9.6 % faster today**, while t103's base arachne-on baseline sits **+12.25 %** above t105's organic arm on **2** retained samples with a 4.67 s spread. Orca g-code **SHA changes every run** (the appendix embeds timing), so SHA is never a job-identity signal on the Orca side; output volume is. Take ran quiet (load 16.4 % → 3.3 %, 12/12 measured samples retained, no starvation exclusions) after a game process and a **sibling checkout's** `cargo test` e2e binary exited; `build-guests --check` exit 0. Evidence: [`evidence/t106-post-pin-scoreboard/`](evidence/t106-post-pin-scoreboard/FINDINGS.md) (`verify.py --self-test` exits 0 checkout-only, 75/75 claims, 9/9 negative controls rejected).

- [Supports-on engine-style fairness](issues/105-supports-on-engine-style-fairness.md): **resolved (2026-11-02; measurement + human ruling — PIN Orca to `tree_strong`)** — the pin costs Orca **+39.6 / +44.4 %** wall on the two base cells (20.449 → 29.532 s; 20.514 → 28.627 s) and **+7.6 / +9.5 %** on the two benchy cells (2.422 → 2.651 s; 2.655 → 2.857 s), over 1 warmup + 4 measured per arm per cell with the arms interleaved. The job changes materially: **+23.5–35.3 %** G-code bytes and support-interface sections **230 → 423** (base classic-on) / **7 → 57** (benchy classic-on); PnP's byte volume on the base cells sits **within 3.4–5.1 %** of the `tree_strong` arm and **27.4–30.8 %** above organic, the output-level corroboration of `DEV-156`. The four supports-on ratios move by **−7.1…−30.8 %** (within-take 7.87x → 5.45x and 7.86x → 5.63x base; 5.12x → 4.68x and 4.60x → 4.28x benchy; derived transfer to the record 8.25x → 5.71x, 7.00x → 5.01x, 5.43x → 4.96x, 4.78x → 4.45x) — the best supported-on cell is still ~4.3x away and the park needs **65–73 %** cuts, so this is a fairness fix, not a route step. Implemented as the standing rig's default (`-OrcaSupportStyle tree_strong`, supports-on only, with a per-run appendix assertion; `vendor` restores the organic escape hatch); **ticket 103's supports-on Orca medians are pre-pin** and [ticket 106](issues/106-post-pin-supports-on-scoreboard.md) re-derives them. Disclosures: load peaked at 55.6 % in round 3; two benchy arachne-on organic samples were starved and excluded from pairing (4/4 base pairs positive, +5.39…+9.75 s); benchy classic-on is 3/4 positive with one load-inflated organic round; absolute ratios are mixed-vintage (today's organic differs from t103's Orca by −10.9…+5.9 %). Evidence: [`evidence/t105-tree-strong/`](evidence/t105-tree-strong/FINDINGS.md) (`verify.py --self-test` exits 0 checkout-only, 74/74 claims, 6/6 negative controls rejected).

- [Prepass versus per-layer: what a cell's wall is made of](issues/104-prepass-vs-per-layer-attribution.md): **resolved (2026-11-02; measurement-only — no candidate, no campaign, no production edit)** — the wall is the **prepass**: **68.5–89.5 %** of every classic cell (base-off 89.5 %, base-on 89.4 %, benchy-on 77.0 %, benchy-off 68.5 %) against **7.9–20.1 %** for the per-layer pipeline, so the park's 57–82 % cut maps onto removing **77.8–93.9 % of the prepass**; deleting the whole per-layer phase leaves **3.02x / 4.51x / 4.66x / 7.60x** (never the park), deleting the whole prepass reaches **0.52x / 0.87x / 1.19x / 1.25x**, and the two biggest prepass stages together leave **1.55x–3.34x** (closest: base classic-off **1.55x**, 3 % short). `PrePass::Slice` is the map's largest measured term (**41,472 ms = 51.2 %** of base classic-off; 28.4 / 18.1 / 19.5 % elsewhere) and is bounded by t59's representation-safety verdict on the closing; `PrePass::SupportGeometry` is **40.1 %** of base classic-on and 26.2 % of benchy classic-on (with `SupportAnalysis` ~50 % of that cell) at the `DEV-156` style-substitution site. Ledger: t73's 2026-10-18 base-off split (prepass 115.0 s / per-layer **94.3 s** of a 214.5 s wall) is superseded — per-layer is now **8.7 %** — while t91's 2026-10-27 base-on prepass share **89.0 %** re-measures at **89.4 %** (same semantics); every per-layer fog entry is bounded by a **6.5–12.7 s** phase. **Walls not claimed** (external load, inflation **1.32x–1.80x**, human-approved disclosure, load fell 95.6 % → 16.8 % mid-batch); ratios of record are t103's paired quiet medians; instrumented passes clean and byte-identical to the t87 references (`3672dc7e…`, `eb4817b4…`, `292df501…`, `22200671…`); benchy repeat moves no share by more than **5.92 pp**; Arachne unmeasured; `verify.py --self-test` exits 0 checkout-only (70/70 claims, 9/9 negative controls). Evidence: [`evidence/t104-phase-split/`](evidence/t104-phase-split/FINDINGS.md).

- [Orca re-baseline on the repaired rig](issues/103-orca-rebaseline-repaired-rig.md): **resolved (2026-10-31 campaign, 2026-11-01 disposal; measurement-only — the archived Orca table is superseded, no production edit)** — the rig carries the ruling's three pins and asserts them per run, and **Orca still wins all 8 cells at 3.52x–8.25x** today-vs-today (benchy classic-off 3.78/3.78, arachne-off 3.52/3.59, classic-on 5.43/5.50, arachne-on 4.78/4.85; base classic-off 4.93/4.96, arachne-off 4.72/4.58, classic-on 8.25/8.21, arachne-on 7.00/7.53, ordinary/accelerated). Fresh Orca: **−27.2 / −24.0 / −35.2 / −30.0 %** on the four benchy cells but **+10.2 / +18.3 / +6.1 / +15.6 %** on the four base cells, so **t98's "every archived multiple is optimistic for PnP" is falsified for the base cells** (base classic-on 9.31x → 8.25x, base arachne-on 9.36x → 7.00x) — the archive's error ran both ways and its gaps (2.71x–9.36x) are not comparable. Park re-derived and **still binding**: every cell needs **57–82 %** of its wall removed to reach ~1.5x = **72–102x** the whole 0.80 % floor; release bound **3.52x** (2026-10-25: 2.71x), moved by Orca on benchy, not by PnP (benchy −2.9…+1.9 %; base supports-on −6…−14 % from t102). Supports-on stays the concentration (+36…+67 % over its off sibling). Proof: 16/16 PnP groups `degraded=false`/`non_fatal=0`, ordinary ≡ accelerated bytes in 8/8, PnP output **exactly the t87 post-t81 reference set** in 8/8, both freshness gates exit 0, 32/32 Orca runs resolved the three pins and the filament leg. Two named starvation exclusions (`base-arachne-on-orca-m3`, `base-arachne-on-pnp-ordinary-m3`), three whole-cell re-runs attempted and all spoiled by external load (competing integration-test binary, then 78–96 % browser load); the human approved proceeding with the noise disclosed, all CSVs preserved, and that cell's band (6.95–7.05x / 7.23–7.53x) does not change its verdict. The brim asymmetry (PnP 8 mm vs pinned Orca 0) is disclosed beside the numbers and remains its own human decision. Evidence: [`evidence/t103-orca-rebaseline/`](evidence/t103-orca-rebaseline/SCOREBOARD.md) (`verify.py --self-test` exits 0 checkout-only, 13/13 negative controls detected).

- [Support-attribution side table materialisation](issues/101-support-identity-table-materialisation.md): **resolved (2026-10-31; Phase-1 screen — both exact families are BELOW the floor, so the ticket's own stop rule fires; no candidate, no campaign, no production edit, recommended family 3 "keep")** — the shape census (one deterministic census, identical on all three table arms) is **23,751 rows over 431 bodies** (`rows/body` p50 **50**, mean **55.1**), **34,662,046** demand ids, per-row list length p50 **1,322** / max **2,847**, **`mixed_bodies = 0`** (every row of a body carries a byte-identical list), only **418 distinct list contents** and **428,726 retained ids = 1.24 %** of the blocks — i.e. the table is **one body's demand list cloned once per emitted support path**, and sharing per body would be content-exact and keep **1.24 %** of the blocks and of the **1.2486 GB** heap. Both sides are measured in situ: the allocation side by a per-layer `Instant` window around `assemble_ordered_entities_with_support_identities` (`crates/slicer-runtime/src/layer_executor.rs`) (**412.6 ms** median, table **413.3 / 433.8 / 452.7 ms** vs skip **21.1 / 17.3 ms**), the free side by a sampling-free `drop(layer_irs)` timing (**491.2 ms** median, table **432.4 / 494.1 / 434.4 ms** vs skip **2.614 / 2.890 ms**; 34,796,481 blocks / 1,358,397,544 bytes reproduced, skip arm 62,751 / 107,020,175). **Combined ceiling 821.6 ms uncontended / 926.8 ms most generous = 0.50–0.56 % of the cell = 0.62–0.70x the 1,318 ms floor**, charging 100 % of the parallel assembly time to the critical path. **The headline is a ledger correction to t100**: its 3.468 s free-side ceiling was a **system-heap** artifact, and t102's mimalloc KEEP cut the same free **3.526 s → 0.432–0.494 s (7.1–8.2x)**; the cause-level route had already been banked by the allocator swap. The wall is **not claimed**: five runs on one binary spread **150.195–154.793 s (3.0 %)**, 5.0x the whole ceiling. Consumer audit re-derived: written only by the assembly, **read by no production path** (only `structured_support_identity.rs` and `raft_geometry.rs` under `crates/slicer-runtime/tests/integration/`, both fixture-level), set empty by `crates/slicer-macros/src/lib.rs` and `crates/slicer-runtime/src/anchored_rows.rs`, absent from `crates/slicer-schema/wit/` — and proven unread end-to-end by **5/5 runs byte-identical to the post-t81 reference** (`22200671…` / 42,507,912 bytes) with the table absent. Disclosures: the machine was **not quiet** (owner's browser at 15–56 % of 12 cores; wall drifted 151.3 → 154.6 s), so the ceiling is instrumented and the wall is not claimed; the table arm's drops spread 14 % and the ceiling uses the `min`/`max` envelope, over-charging the lever; the allocation side is summed, not critical-path, so the true wall contribution is smaller; only classic-on was screened; the 1.2486 GB heap figure is derived and agrees with the census to 0.2 %; memory is out of scope (DEV-026) so it is recorded and not priced; probe reverted, both freshness gates exit 0. Evidence: [`evidence/t101-side-table/`](evidence/t101-side-table/FINDINGS.md) (`probe.patch`, `summary.json`, `extract.py`, `verify.py`; `verify.py --self-test` exits 0 checkout-only, 12/12 claims, 15/15 negative controls rejected).

- [Global allocator substitution on the host binary](issues/102-global-allocator-substitution.md): **resolved (2026-10-31; human-approved KEEP — the standing four-cell gate PASSES, candidate committed)** — `pnp_cli` becomes `AccountingAllocator<mimalloc::MiMalloc>` (two host files plus the contract doc), the packager slot `docs/16_slicer_report.md` §"Global allocator contract" already prescribes, so `--report` accounting keeps its shape. Proof **16/16** (both arms byte-identical in every cell, both clean, non-vacuous roles; **8/8** ordinary cells equal to `evidence/t91-exact-z-memo/references.json`; **8/8** ordinary ≡ accelerated; identical 24-module trees, distinct host binaries `db153219…`/`1de06f27…` ordinary and `7f5ad486…`/`9986754e…` accelerated). Pre-gate on ordinary base classic-on: **−7.74 %** wall / **−2.25 %** CPU in 3/3 pairs. Campaign, 2 warmups + 6 alternating pairs per cell: wall **−6.19 / −5.64 / −5.54 / −4.80 %** (−10.19 / −9.42 / −9.04 / −7.41 s) and process CPU **−1.74 / −2.18 / −1.54 / −1.88 %** (ordinary classic-on / arachne-on, accelerated classic-on / arachne-on), **24/24 pairs byte-identical**, 6/6 retained per cell, zero starvation exclusions, **23/24 pairs wall-negative** (accelerated arachne-on #6 **+0.18 %**, cell median −4.80 %); floor multiples **7.73 / 7.05 / 6.92 / 6.00x** against 0.80 % of t91's walls. The **~5–6 %** effect is much larger than the free-only price (**3.527 → 1.302 s = 1.35–1.66 %**, t100) because the allocator acts on every allocation in the process, but it removes **no allocations** — the census still shows the same **34,796,481** blocks, each freed more cheaply, so the cause-level route stayed [101](issues/101-support-identity-table-materialisation.md) (now resolved: below the floor; see that row). Disclosures: the pre-gate ran under a stray integration-test binary pinning ~8 cores (walls 227–266 s) which exited before the campaign; the pre-campaign snapshot records load median **1 %**; today's baseline walls (152–156 s) differ from t91's 164.803 / 166.931 s, so ratios are today-vs-today and the floor is conservative; the change is process-wide (no stage attribution); memory is not gated; one transient `cl.exe` failure on the accelerated build passed on retry (no artifact). Evidence: [`evidence/t102-allocator/`](evidence/t102-allocator/FINDINGS.md) (`verify.py --self-test` exits 0 checkout-only, 12/12 claims, 12/12 negative controls rejected).

- [Allocation volume of the whole-print layer collection](issues/100-allocation-volume-layer-collection.md): **resolved (2026-10-30; Phase-1 screen — the arena / bulk-free family as scoped is measured-dead: mis-aimed and construction-unreachable; no candidate, no campaign, probe reverted)** — the whole-print free inside `host:gcode_emit` is **34,796,481 blocks / 1,358,397,544 bytes / 3,526,544 µs** (**101.3 ns per block**, reproducing t94's 3.558 s) on ordinary base classic supports-on, and **34,662,668 blocks = 99.6 %** of them are **8–15-byte strings** sampled as `__skirt_…` support family/body ids and `demand-N` ids. That is the `support_entity_identities` side table that `assemble_ordered_entities_with_support_identities` (`crates/slicer-runtime/src/layer_executor.rs`) clones **per support path** — 23,751 rows, each carrying the body's whole `demand_ids` list (**~1,457 ids derived** from the histogram, 34,615,166 strings). Geometry is a rounding error of the shape: **32,673 blocks = 0.094 %** and **105.8 MB = 7.8 %** of the bytes; entities are 30,076 and points 2,782,098. **Bulk-free counterfactual** (same byte volume, pages touched, both pricing orders): **44.5–63.8 ms** for one block after the drop (189.7 ms before it, collection live) and **58.3 / 120.5 ms** for 495 blocks (one arena per layer), so the **ceiling is 3.468 s = 2.10 % of the cell = 2.63x** the 1,318 ms floor — **above** the floor. The family still dies twice: flattening a live collection costs a **73.3–115.1 ms** copy **plus the same 3.527 s storm** (3.60 s, **+73 ms worse**), and an at-creation arena aimed at the geometry would remove **0.094 %** of the blocks. **Allocator arm** (mimalloc): drop **3.527 → 1.302 s measured** (**2.225 s = 1.35 %**, 1.69x floor; **0.796 s / 2.731 s = 1.66 % / 2.07x** after subtracting the probe's measured **0.506 s** sampling overhead) — above the floor, but the shipped allocator is a **packager** decision under `docs/16_slicer_report.md` §"Global allocator contract" and it removes no work. Disclosures: instrumented walls (200.7–270.7 s) are not wall claims; the sampling instrumentation costs 0.506 s; the `before` bulk arms ran with the collection live (2–3x the `after` prices, conservative arm used); the copy arm was first **elided by the optimiser** (`copy_us = 0`) and re-measured with a `black_box`-ed checksum; only classic-on was censused; both freshness gates exit 0; probe reverted. Graduated [101](issues/101-support-identity-table-materialisation.md). Evidence: [`evidence/t100-allocation-arena/`](evidence/t100-allocation-arena/FINDINGS.md) (`verify.py --self-test` exits 0 checkout-only, 12/12 claims, 13/13 negative controls rejected).

- [Split and union the whole-layer clip-area dilation](issues/99-clip-split-union-dilation.md): **resolved (2026-10-30; measured REJECTED on price — the most favourable split is 0.41–0.43 % of the cell, about half the 0.80 % floor, and not representation-safe on top; no candidate, no campaign, probe reverted)** — offsetting every `deep_infill_area` polygon separately and unioning the pieces is genuinely cheaper per call (**0.8230x / 0.8252x**, two opposite pricing orders agreeing within **0.27 %**, both arms reproducing `292df501…` / 21,023,739 bytes) but the call's own share is too small for a 17.5 % cut to clear the floor: **2,434.2 / 2,464.8 ms accumulated** saving = **314.3 / 298.1 ms** wall-equivalent at each run's own cpu/wall = **0.43 / 0.41 %** of the 73.17 s cell, i.e. **0.51–0.54x the 585.4 ms floor**. The union-free lower bound (per-polygon offsets only, 0.8056 / 0.8069x) is **0.47 %** — still below — so no split shape in the family can clear it, because the offsets are **80.6 %** of production. The reusable decomposition: **~80 % per-path Miter offset, ~19 % batched internal union/cleanup, ~2 %** an explicit union of the already-rounded pieces — splitting partitions the cleanup instead of skipping work. Exactness is independently fatal: byte-identical on **1/130** calls, set-equal on **2/130**, **5,329** differing polygons, identical in both orders. Disclosures: instrumented walls only (the two probe walls are 27.0 / 17.9 s above the 73.17 s baseline and 9.1 s apart for the same work, so the production spans they denominate are inflated and only the ratio is read); the differing area was not quantified because the price already decides; probe reverted; both freshness gates exit 0. Evidence: [`evidence/t99-clip-split-union/`](evidence/t99-clip-split-union/FINDINGS.md) (`verify.py --self-test` exits 0 checkout-only, 6/6 negative controls rejected).

- [Match Orca's tree-support branch distance in the matched job](issues/97-match-branch-distance-fairness.md): **resolved (2026-10-30; REJECTED on fairness — the premise is false, no profile edit, no campaign)** — `support_style = default` is not style-neutral. Canonical `SupportParameters` resolves it to `smsTreeOrganic` on a tree family (`// Orca: use organic as default`), `PrintObject::_generate_support_material` / `TreeSupport::generate` fork that style to `generate_tree_support_3D`, and the organic engine's config struct (`TreeSupportCommon.hpp`) reads **only** `tree_support_branch_distance_organic`. The matched job resolves it to **1 mm**; PnP's normal-key default is also **1 mm** (`tree-support-planner.toml`), and the supports-on profiles set neither key. So the effective distances are already matched, and t88's 5 mm arm (184,115 → 176,869 ms, −3.94 %, G-code −0.83 %, changed SHA) is a **5x PnP branch-spacing reduction in a different engine**, not a match. The real gap is the style substitution: `TreeSupportStyle::from_config` aliases every canonically-organic input (`default`/`organic`/`grid`/`snug` on a tree family) to the classic engine's **Strong** style because the organic engine is unimplemented (`DEV-156`; organic port queued in `docs/specs/support-generation-remediation-plan.md`). The deviations table's `tree_support_branch_distance` row (PnP 1.0 vs Orca 5.0) compares the **inert** normal key; the effective `_organic` key is 1.0 on both sides and is not registered by PnP. Disclosures: the 2026-10-27 Orca install (`2.5.0-beta`) rejects the rig's file-path `--load-settings` (`Preset was not found in the loaded bundle`) and no longer ships `resources/profiles/BBL/{machine,process}`, so Phase 0 rests on canonical source + `DEV-156`, not a fresh Orca run; that also blocks Orca-side scoreboard refreshes until the rig is updated or a stock Orca restored. Neither freshness gate applies (docs-only; no guest or code change). Evidence: [`evidence/t97-branch-distance/`](evidence/t97-branch-distance/FINDINGS.md) (`verify.py --self-test` exits 0 checkout-only, 5/5 negative controls rejected).

- [Windowed clip area in internal-bridge qualification](issues/96-windowed-clip-area-dilation.md): **resolved (2026-10-30; Phase-1 screen — REJECTED on exactness, no candidate, no campaign, probe reverted)** — the whole-layer `+1.5 × spacing` dilation is **11,672.7–11,813.6 ms** accumulated over 130 calls; a candidate-box window (`8 × spacing`, operand `11 × spacing`, result clipped back) keeps **12.2 %** of the deep-area ring vertices and saves **10,653.5–10,780.5 ms = 1.17 s wall-eq = 1.57–1.60 %** of the ordinary base classic-off cell (**2.0x** the 0.80 % floor, two runs within **1.2 %**). It is **not representation-safe**: the windowed arm emits `5fc8fa72…` / 21,023,889 bytes against `292df501…` / 21,023,739, identically in two runs, with the first differing block an `Internal Bridge` at `;Z:21.4` (8 → 10 moves). At that layer the two clip areas are **set-equal inside the window** (`mismatch_polys = 0`), so qualification and per-candidate `area` are unchanged; the failure is `internal_bridge_angles`' `expansion_area ∩ clip_area` term → `boundary_anchor_polylines`: with qualification and `area` unchanged, the nearest anchor must have lain **beyond the window** (or the cut introduced a closer ring), so the anchor reach is not bounded by the planned `3.3 × spacing` and the window moves it. No finite candidate-driven window is safe (mirror of t93, where the same operand's erosion was safe because its consumers are bounded by `2 × spacing`). Secondary: **116/130** calls show a non-zero inside-window set difference (**1,071** slivers). The wall screen was not reached; the single uninstrumented window wall (76.557 s) is unpaired and not a claim. Disclosures: instrumented walls attribution only; margins not swept (failure is unbounded by construction); both freshness gates exit 0. Evidence: [`evidence/t96-windowed-clip-area/`](evidence/t96-windowed-clip-area/FINDINGS.md) (`verify.py --self-test` exits 0 checkout-only, 6/6 negative controls rejected).

- [Batched exact-Z cross-sections in the support validation loop](issues/95-exact-z-cross-section-batching.md): **resolved (2026-10-30; human-approved KEEP, candidate committed `afe9c361`)** — the 431 anchor Z values were 431 distinct `(object, region, z)` keys, so the exact-Z cache never hit and every entry paid `slice_mesh_ex(mesh, &[z])`, a one-element Z array over `tmp/base.stl`'s **2,461,234** triangles. Phase 1 (probe, 3 slices, all outputs equal to the post-t81 references): **431/431** planes equal polygon-for-polygon; **6.42 / 6.72 / 6.84 s** of single-Z walks collapse to **0.99–1.05 s** in one batched walk at the object's sorted Z set = **5.37 / 5.73 / 5.83 s** saved = **3.26 / 3.48 / 3.49 %** of the cell, **4.07–4.37x** the 0.80 % floor; the batch buys **6.4–6.9x**, not 431x (two binary searches over 431 planes plus a straddle test per triangle, memory-latency-bound). Candidate: per-`(object, z_units)` cross-section memo + `ExactZQueryService::prefetch_cross_sections`, called once per object before the serial `validate_entry` loop; per-`(object, region, z)` contract, single-Z miss path and per-entry degradation unchanged; host-only. **Proof 16/16** (arms byte-identical, 8/8 ordinary equal to the post-t81 references, 8/8 cross-mode equal, 24 identical modules per mode). **Gate PASS in all four base supports-on cells**, 6/6 pairs, zero exclusions, **24/24 pairs byte-identical**: wall **−3.97 / −3.19 / −2.86 / −2.64 %**, CPU **−0.70 / −0.70 / −0.90 / −0.15 %**, wall deltas **−6.17 / −5.68 / −5.43 / −5.49 s**. One **disclosed** non-negative pair (accelerated classic-on #4, **+2.35 %**, during a load event that slowed both arms ~10 s). Evidence: [`evidence/t95-exact-z-batch/`](evidence/t95-exact-z-batch/FINDINGS.md) (`verify.py --self-test` exits 0 checkout-only, 7/7 negative controls rejected).

- [Remaining whole-print collection drop inside G-code emission](issues/94-remaining-whole-print-drop.md): **resolved (2026-10-29; Phase-1 screen — measured-dead, no candidate, no campaign, probe reverted)** — the `host:gcode_emit` bracket is **93-94 % drop**: rotation 6-7 µs, walk **176-253 ms**, free **3,242-3,558 ms**, bracket 3,415-3,811 ms (ordinary base supports-on, 495 layers, both cells byte-identical to `22200671…` / `0b29e1bf…`). **Parallel free (`into_par_iter().for_each(drop)`) is a wash**: classic **−0.321 s (+0.19 %** of the cell), arachne **+0.200 s (−0.12 %)** — 12 threads buy 9 % in one cell and lose 6 % in the other, so Windows' process-heap serialisation, not thread count, sets the cost. **Layer-by-layer consumption is a structural zero**: the walk is serial and totals 176-253 ms, so interleaving the same serial frees with it overlaps nothing. **Deferred free overlapping the post-emission tail** has an absolute ceiling equal to the tail — **1,603 ms classic (0.97 %)** and **1,367 ms arachne (0.82 %)** against the **1,318 / 1,335 ms** floor (**1.22x / 1.02x**) — before any overlap loss, wake-up or contention with the concurrent 42 MB serialisation, so under the pre-declared **strict four-cell wall** gate the screen does not justify a campaign. What the screen leaves is structural, not strategic: the free is an **allocation-volume** cost (~5.3 GB peak working set, t79), and only fewer/larger allocations (arena or bulk-free collection) can move it. Disclosures: load 8 % at start (not a quiet campaign), one probe run per arm per cell, instrumented walls never a wall claim (arachne serial vs parallel walls differ 4.8 % in the *opposite* direction of the 0.20 s drop difference). Evidence: [`evidence/t94-remaining-drop/`](evidence/t94-remaining-drop/FINDINGS.md) (`verify.py --self-test` exits 0 checkout-only, 8/8 negative controls rejected).

- [Whole-layer deep-infill offsets in internal-bridge qualification](issues/93-whole-layer-deep-infill-offsets.md): **resolved (2026-10-29; measured GATE NOT MET on the strict four-cell wall clause — human-approved DROP, candidate reverted; `candidate.patch` preserved)** — Phase 1 screened four shapes against the map's **0.80 %** floor: **lazy dilation dead** (0 of 130 calls skippable; 130 of 130 have a surface reaching the deep area), **cheaper predicate rejected** (Miter dilation/erosion are not adjoint), **duplicate recompute dead** (9 recomputes, 0 with an identical deep-area operand; deleting all 9 reaches only **0.28 %**), and **windowed erosion above floor** (**9,244.1 / 9,311.3 ms** accumulated saving = **1,006.9 / 1,045.6 ms wall-eq = 1.38 / 1.43 %** of the cell; two runs agree within 3.70 %). The candidate clipped the `−4.5 × spacing` erosion operand to the pre-retain qualified bbox grown by `14 × spacing`; **16/16** proof cells byte-identical between arms, **8/8** ordinary cells equal to the post-t81 references, **8/8** cross-mode equal, 24 identical modules per mode, unit + both calicat internal-bridge e2e tests pass. Campaign (6/6 pairs, zero exclusions, every pair byte-identical): ordinary classic-off **−3.31 % wall / −1.19 % CPU**, ordinary classic-on **−0.26 % / −0.48 %**, accelerated classic-off **+0.13 % / −0.81 %**, accelerated classic-on **+0.62 % / −0.45 %**. The CPU fall matches the probe's 9.2 s (9.7 s measured) and the wall transfer is lifted by cpu/wall 9.65 → 9.84; the accelerated wall misses are inside each cell's own repeat range but positive, so the pre-declared clause is NOT MET and the human ruled DROP. Disclosures: server restart during ordinary classic-on (cell re-run whole, partial preserved), campaign timed only the four base classic cells. Evidence: [`evidence/t93-deep-infill-offsets/`](evidence/t93-deep-infill-offsets/FINDINGS.md) (`verify.py --self-test` exit 0 checkout-only, 18/18 negative controls rejected).

- [Per-surface bridge boolean locality in ShellClassification](issues/92-per-surface-bridge-boolean-locality.md): **resolved (2026-10-28; Phase-1 screen — measured-dead, no candidate, probe reverted)** — the ticket's Phase-2 shapes (per-surface bbox gate, hoistable `1.3 ×` anchor offset, many-piece tiling) have an **absolute upper-bound ceiling of 0.341–0.371 %** of the base classic supports-off cell against the map's **0.80 %** floor (**2,429.6 / 2,303.2 ms accumulated = 271.1 / 249.7 ms wall-equivalent**, two probe runs, outputs byte-identical to `292df501…` and clean); deleting the entire `internal_bridge_angles` step as well reaches only 0.40–0.44 %. Shape counts: **130 calls, 196 surfaces (mean 1.51, max 7)**, `deep_infill_clip_area` 228 polygons (mean 1.75, max 7) — Orca's 128-piece tiling threshold never fires — and the per-surface `offset`/`intersection`/qualification is **0.25 s accumulated** against `internal_bridge_angles`' 2.58 s (t75's 2.4 s angles bracket transfers, its 1.5 s clip bracket does not: today 0.10 s). The screen surfaced a **larger term of a different shape**: two **whole-layer offsets on `deep_infill_area`** — `+1.5 × spacing` **12.5 / 11.4 s accumulated** (130 calls, median 81.6 ms) and `−4.5 × spacing` **10.0 / 9.3 s** (14 qualified calls, max 428.3 ms) — together **3.06–3.43 % of the cell**, ~4x the floor and ~9x this ticket's ceiling; whole-layer operands, so consumer bbox-gating cannot remove them. Graduated [93](issues/93-whole-layer-deep-infill-offsets.md). Disclosures: instrumented wall 79.0–83.0 s vs the uninstrumented 73.17 s cell; ~7–9 % inter-run bucket drift, verdict unchanged; both freshness gates exit 0 before and after. Evidence: [`evidence/t92-per-surface-bridge/`](evidence/t92-per-surface-bridge/FINDINGS.md) (`verify.py --self-test` exit 0 checkout-only, 15/15 negative controls rejected).

- [Memoize z-independent exact-Z query terms per object](issues/91-exact-z-object-geometry-memo.md): **resolved (2026-10-27; human-approved KEEP — wall pass in all four pre-declared base supports-on cells, strict CPU clause missed by +0.08 pp in one, candidate committed `f10cef5a`)** — `ExactZQueryService` memoizes the world mesh, the bounds-derived plate and the constant top-Z cross-section **per object**, keeping the per-`(object, region, z)` cache contract unchanged; the plate is built once instead of on each of 431 misses. Wall **−5.52 / −3.84 / −2.68 / −4.95 %** and CPU **−1.22 / −0.66 / +0.08 / −0.32 %** (ordinary/accelerated × classic/arachne), **24/24 pairs byte-identical**, one SHA per arm, zero exclusions; 16/16 proof cells byte-identical, 8/8 ordinary cells equal to the derived post-t81 references, 8/8 cross-mode equal. The **+0.08 pp** miss is 0.8 s of ~1,053 s process CPU inside that cell's own repeat spread, disclosed and accepted; the ordinary counterpart of the same job measured **−1.22 %**. Probe (attribution only): **431 queries / 431 misses / 431 geometry builds → 1**, `query` **22,636.3 → 8,922.3 ms (−60.6 %)**, `validate_entry` **24,335.0 → 10,004.3 ms**; the request-Z cross-section is the only per-Z work left. `post_planner_gap_ms` falls 14.7→9.9 / 18.4→7.4 / 21.8→9.9 / 13.8→12.1 s. Retained memory: **42.25 MiB** per object on `tmp/base.stl` (recorded, not gated). Evidence: [`evidence/t91-exact-z-memo/`](evidence/t91-exact-z-memo/FINDINGS.md) (`verify.py --self-test` exit 0 checkout-only, 13/13 negative controls rejected).

- [Single-ownership layer collection through finalization and G-code emission](issues/90-postpass-layer-collection-ownership.md): **resolved (2026-10-27; human-approved KEEP — gate met in all four pre-declared base supports-on cells, candidate committed `7b22fccc`)** — wall **−5.54 / −7.56 / −6.54 / −7.64 %** (ordinary/accelerated × classic/arachne) with process CPU **−1.13 … −1.67 %**, **24/24 pairs byte-identical**, one SHA per arm, zero starvation exclusions, every pair moving the right way (−1.56 … −13.20 %). One ownership pass: `execute_layer_finalization_owned` mutates the caller's `Vec<LayerCollectionIR>` and returns it instead of deep-cloning a rollback snapshot no production caller reads (a fatal abort now consumes the working collection, the ticket's pre-authorized delta), `GCodeEmitter::emit_gcode_owned` lets `DefaultGCodeEmitter` rotate and emit the caller's own vector (defaulted so every other emitter is untouched), and `execute_postpass_with_owned_capture` hands the collection over, so it is dropped **once** inside the `host:gcode_emit` bracket instead of being cloned there and dropped again at teardown; the visual-debug capture sinks keep the borrowed path and receive the collection by move. Attribution (instrumented, attribution only): clone **1,765 / 1,684 ms**, commit drop **3,685 / 3,758 ms** and teardown drop **4,490 / 3,723 ms** all gone, the single remaining drop **1.56 / 2.68 s cheaper** than the old clone+drop pair, postpass phase **−66.5 / −70.3 %** — hence measured wall deltas **12.0 / 14.8 / 15.9 / 15.4 s** beat the pre-declared net ceiling **10.76 / 10.29 s**. Exactness 16/16 byte-identical to the derived post-t81 references, ordinary ≡ accelerated **8/8**, both freshness gates exit 0, host-only candidate pinned by `provenance.json` (equal 24-module trees, distinct binaries). Evidence: [`evidence/t90-postpass-ownership/`](evidence/t90-postpass-ownership/FINDINGS.md) (`verify.py --self-test` exit 0 checkout-only, 11/11 negative controls rejected).

- [Parallel exact-Z validation](issues/89-support-plan-exact-z-validation-parallel.md): **resolved (2026-10-27; human-approved DROP — measured GATE NOT MET on the CPU clause, candidate reverted)** — the order-preserving fold is **exact** (16/16 cells byte-identical, 8/8 cross-mode, 160/160 contract, new 64-entry three-way order test) but **not work-preserving**: a 3-pair pre-gate on ordinary base classic-on at 2 % load measured wall **−5.50 %** against process CPU **+6.78 %**, more than twice the pre-declared **+3.0 %** ceiling, so the four-cell campaign was not run. The probe rules out duplicated work (431 entries = 431 distinct `(object, z)` = 431 queries = 431 misses) and splits the tail into requested-Z cross-section **50.96 %**, constant top-Z cross-section **43.39 %**, mesh build **5.65 %** of **92.73 s** accumulated thread-time for a **7.99 s** wall versus t88's **15.24 s** serial — ≈6.1x thread-time per query at 12-way concurrency, i.e. memory/allocator-bound. The optional `(object, z)` cache is **measured worthless** (one entry per anchor Z). Graduated [91](issues/91-exact-z-object-geometry-memo.md): memoize the z-independent terms (**49.04 %** of the tail, ≈**7.5 / 6.8 s** ceiling = **4.1 / 3.6 %** of the cell), which removes work and should lower CPU. Two pre-existing fixture reds (t81 `resolution`, t20 consumer-live stamps) repaired to run the contract suite. Evidence: [`evidence/t89-exact-z-parallel/`](evidence/t89-exact-z-parallel/FINDINGS.md) (`verify.py --self-test` exit 0 checkout-only, 8/8 negative controls rejected).

- [Supports-on increment attribution at post-resolution-port HEAD](issues/88-supports-on-increment-attribution.md): **resolved (2026-10-25; measurement-only — no candidate, no production edit)** — at HEAD `5c7fa03e`, four quiet (5 % load) instrumented base captures plus a loaded corroborating pass and a config-only `tree_support_branch_distance = 5.0` arm; all 1 mm outputs byte-identical to the post-t81 references. The increment is **106.8 s classic / 109.7 s arachne** and is **~69 % support geometry** (planner 55.5 s = 52.0 % of the classic increment, canonical contact detection 10.2 s, per-layer render 5.9 s wall) plus **~31 % host-side** — a **serial exact-Z validation loop** at **15.24 / 13.94 s** (99.3 % of the post-planner gap; every entry independent; cache keys on `region_id` although the computation is object-wide) and **one finalization clone + two whole-collection drops** (finalization 6.76 s, emission 6.26 s = a ~6.15 s drop after a ~5 ms clone, postpass teardown gap 4.58 s). Graduated [89](issues/89-support-plan-exact-z-validation-parallel.md) (parallelize the validation; **8.28 / 7.42 %** of the cell) and [90](issues/90-postpass-layer-collection-ownership.md) (one ownership path; **5.84 / 5.47 %** net, 9.18 / 8.60 % upper). Measured dead: the SupportAnalysis eligibility fold (2.24 s gross, no above-floor removable term), the contact kernel (10.2 s, parallel canonical), the planner (55.5 s, support geometry; t78 batching in place), the per-layer render (5.9 s wall, already parallel). The **`tree_support_branch_distance` 5 mm arm is −7.25 s (−3.94 %)** and **~6.8 s of the increment (6.4 %)** — a modest driver; the parity ruling returns to the human. Probe trap recorded: an `Instant` clock read disagreed with the event timestamps by the width of an earlier phase, so a 5 ms clone read as 4.09 s; all probe claims use the `SystemTime` marks and stage brackets. Evidence: [`evidence/t88-supports-increment/`](evidence/t88-supports-increment/FINDINGS.md) (`verify.py` exit 0 checkout-only, `--self-test` 8/8 negative controls rejected).

- [Matched-pair scoreboard refresh at post-resolution-port HEAD](issues/87-scoreboard-refresh-post-resolution-port.md): **resolved (2026-10-25; measurement-only — the first fully resolution-matched 8-cell scoreboard)** — at HEAD `6e71c1a1` with both trees rebuilt, Orca still wins all 8 at **2.71x–9.36x**, CPU corroborating (PnP burns 4.1x–21.3x Orca's CPU). Best cell **2.71x** benchy classic-off accelerated (2.75x ordinary; 2.74x benchy arachne-off), worst **9.36x** base arachne-on ordinary. **No cell within ~1.5x → the below-fold park is still binding**, though the release bound moved **3.30x → 2.71x**. PnP ordinary walls fell **39.7–66.2 %** since 2026-10-18; Orca also measured **16.2–35.3 % faster** (drift, disclosed; today-vs-today). **Supports-on is the concentration:** base off 5.18–5.28x vs on 9.20–9.36x, PnP increment **+97.90 / +113.38 s** vs Orca **+4.50 / +5.79 s** with *fewer* PnP support sections. 96 rows / 72 measured, 1 warmup + 3 measured per group, **zero starvation exclusions**, zero validation failures; PnP ordinary ≡ accelerated bytes/SHA in all 8 cells; new post-t81 reference set recorded (base SHAs reproduce t83/t84/t85). Disclosure: an accidental duplicate benchy round is preserved but not ledger input. Graduated [88](issues/88-supports-on-increment-attribution.md). Evidence: [`evidence/scoreboard-refresh-20261025/`](evidence/scoreboard-refresh-20261025/SCOREBOARD.md) (`verify.py` exit 0 checkout-only, `--self-test` 8/8 negative controls).

- [Seam planner inset-offset batching](issues/85-seam-inset-offset-batching.md): **resolved (2026-10-25; human-approved KEEP — gate met in all four base classic cells)** — the seam planner's 495 per-region inset offsets become **one** `offset_polygons_batch`; wall **−7.46 / −2.72 %** (ordinary) and **−7.03 / −3.86 %** (accelerated), CPU **+0.34…+0.47 %**, 24/24 pairs byte-identical, 16/16 output identity, zero exclusions; seam stage **8,356 → 1,737 ms** = **≈7x batch occupancy** (conservative 4.4x, floor 1.15x), so t84's 5.43 % pricing is beaten and the stage drops to 2.3 % of a base cell. Parity is exact by construction, pinned by a per-entry serial-reference test.

- [Accelerated shim workspace-root resolution](issues/86-accelerated-shim-workspace-root.md): **resolved (2026-10-25; infra prerequisite of the t85 take)** — the rustc shim resolved the workspace root from CWD, which is a cargo-registry package directory when cargo compiles a dependency's `build.rs`, so every cold accelerated build died; the accelerated entry points now export `PNP_XTASK_WORKSPACE_ROOT` and `resolve_shim_root` re-validates it (env → CWD → binary dir). `cargo test -p xtask` 187/0, a 48-guest cold accelerated build completes, both freshness gates exit 0.

- [Base common-floor attribution: SeamPlanning and OverhangAnnotation](issues/84-base-common-floor-attribution.md): **resolved (2026-10-24; attribution take — one candidate graduated, two shapes measured dead; no production edit, probe reverted)** — at HEAD `ce8f3992` on base classic off/on, both arms byte-identical to the t81 references `292df501…` / `22200671…`. **SeamPlanning** (9.14 / 8.97 s = 9.84 / 4.30 % of cell wall) is **92.4 / 93.2 %** one serial native `offset_polygons` per region: 495 calls, 8,195 / 8,177 ms; candidate generation 94.6 / 94.3 ms, scoring 16.6 / 16.1 ms, harvest 58.1 / 44.7 ms, host view projection 4.2 / 11.4 ms — the candidate list is **not** the wall problem. **OverhangAnnotation** (10.01 / 10.30 s = 10.78 / 4.94 %) is **99.7 %** the parallel `annotate_overhangs` kernel; 1,455 round offsets (485 band layers × 3) are **85.4 %** of its native work at **11.86x** parallelism; host gather + merge + clone + commit is **0.18 %**. Graduated [85](issues/85-seam-inset-offset-batching.md): batch the inset offsets through the existing `offset_polygons_batch` — upper bound **8.83 / 3.92 %** of wall, **5.43 / 2.41 %** at t78's measured 2.6x. Dropped with measured ceilings: overhang bbox-window operand (retains **97.84 %** of previous points → **0.189 / 0.080 %**) and the seam candidate list (**0.288 / 0.127 %**). Evidence: [`evidence/t84-common-floor/`](evidence/t84-common-floor/FINDINGS.md) (`verify.py --self-test` exits 0, 8/8 negative controls rejected).

- [Arachne wall-simplification tolerance pricing](issues/83-arachne-wall-simplification-tolerance-pricing.md): **resolved (2026-10-24; measurement-only pricing take — DROP, below the map's pre-declared 0.80 % wall floor; no candidate, no parity port, no production edit)** — one binary and module tree, two config-only arms (default 0.05/0.005 mm vs canonical 0.5/0.025 mm), 2 alternating instrumented pairs per Arachne cell plus a 6-pair uninstrumented wall check on the two borderline cells. The tolerance is **real work**: arachne guest fuel **−10.76 %** (benchy) / **−16.21 %** (base), whole-job fuel −3.41/−1.41 % and −11.56/−3.08 %, G-code bytes **−3.84…−8.23 %** with identical TYPE counts, all runs clean and byte-deterministic per arm, and the default arm reproduces t81's base-arachne-off reference `01a1434c…`. It is not a wall lever: the module is only **2.9–6.9 % of cell wall** at **5.1–10.7x** phase parallelism, so the occupancy-normalized ceiling is **0.039–0.376 %** of wall (fuel-proportional bound 0.844 % best case, whose measured elapsed moved +0.8 % the wrong way). The direct wall check agrees — base-off **+0.17 % wall / +0.09 % CPU**, benchy-on **+3.18 % wall / +0.17 % CPU** (paired medians; benchy-on's instrumented −2.9 % was load drift). `D-168` is retained with the measured magnitude. Observed, not ticketed: `tree_support_branch_distance` (Orca 5 mm vs PnP 1 mm, supports-on) is the other t82 "PnP does more work" geometry key, unmeasured and parity-scoped. Evidence: [`evidence/t83-arachne-tolerance-price/`](evidence/t83-arachne-tolerance-price/FINDINGS.md) (`verify.py --self-test` exit 0, 5/5 negative controls rejected).

- [Fairness-key parity audit of the matched job](issues/82-fairness-key-parity-audit.md): **resolved (2026-10-23; measurement-only — all 618 resolved keys classified, no candidate, no timing)** — eight Orca runs gave a 618-key resolved appendix per cell; only `wall_generator`, `enable_support` and `print_settings_id` vary. **Verdict: no third `resolution`-class lever at scale.** The one unmatched geometry pair where PnP does materially more work is the Arachne tolerance pair — `wall_maximum_resolution` 0.5 vs 0.05 mm and `wall_maximum_deviation` 0.025 vs 0.005 mm (PnP 10x / 5x tighter), Arachne cells only, magnitude unmeasured, already `D-168` but invisible to the generated deviations table because **82 of the 618 keys postdate its 2026-05-19 upstream snapshot**. Everything else either makes Orca do more (`top_shell_layers` 4 vs 3, `support_interface_top_layers` 3 vs 2, `precise_outer_wall` 1 vs 0, `ensure_vertical_shell_thickness`, `detect_narrow_internal_solid_infill`, tree-support brim), is layer-0-only, shape-only, or inert. **Controls matched in all eight cells:** `resolution = 0.01` and `slice_closing_radius = 0.049` (same key, default and order as canonical), retiring the fog's `slice_closing_radius` half. Observations: `detect_thin_wall` declared `true` by classic-perimeters but emitted `0`; `support_interface_bottom_layers` declared `-1`, emitted `2`; `brim_width` 8 vs 0 unresolved at output level; Orca's overhang bands live while PnP's are 0. Recommended next step is a pricing take on the Arachne match, not implemented here. Evidence: [`evidence/t82-fairness-key-audit/`](evidence/t82-fairness-key-audit/FINDINGS.md) (`verify.py --self-test` exit 0, 9/9 negative controls rejected).

- [Slicing-time contour simplification (`resolution`) port](issues/81-resolution-slicing-time-contour-simplification.md): **resolved (2026-10-23; human-approved KEEP — committed)** — `resolution` (default 0.01 mm, canonical 0.001 mm floor applied in the resolver so the appendix discloses what the modules apply) now runs at four canonical call sites: pre-wall simplify in both perimeter modules (region granularity — PnP has no `Surface` model), the gap `douglas_peucker` before `medial_axis`, the classic innermost contour before the infill boundary, and the arachne infill contour. Canonical's classic inner-part site has no PnP counterpart (recorded, not invented); the `0.2 ×` arc-fitting branch is unexpressible and inert on the matched job. The DP is canonical-exact (closed-ring seeding, `<=` acceptance, strictly-greater tie-break, tolerance converted across the 1e6 → 1e4 units/mm scale); the union runs on `clipper2-rust` against canonical's legacy Clipper 6.4.2 `StrictlySimple` union, so set semantics match and representation does not (disclosed). **Confinement 20/20** on typed IR (`Layer::Slice` identical in all 8 cells, `Layer::Perimeters` differs in all 8, support silhouette identical supports-on). **Campaign: gate met, 8/8 wall-negative and 8/8 CPU-negative** — classic −14.31/−9.00 % (benchy) and −22.24/−10.87 % (base) wall with CPU −18…−27 %; Arachne −0.07…−1.87 % (wall-neutral); `base arachne-off` failed its first six pairs at +2.30 %, stopped by rule, re-ran at −2.77 % (n=12 −1.07 %, spread ±9 — neutral within resolution). Deterministic (one SHA per arm/cell) and the baseline reproduces the t58 reference `34886ccb…`. This is a **geometry-affecting keep**: the byte-identity proof regime is replaced by invariants + confinement + determinism, with new per-cell references in `evidence/t81-resolution-port/campaign/`. Evidence: [`evidence/t81-resolution-port/`](evidence/t81-resolution-port/FINDINGS.md).

- [Resolution-matched fairness premise for the matched job](issues/80-resolution-fairness-premise.md): **resolved (2026-10-22; measurement-only — the job was NOT resolution-matched; disclosure recorded, one candidate graduated)** — Orca's matched profile inherits `resolution = 0.01 mm` and consumes it as a **slicing-time** contour simplification before wall/infill generation (`PerimeterGenerator.cpp`'s `surface_simplify_resolution` → `union_ex(surface.expolygon.simplify_p(...))` and `ex.douglas_peucker(...)`, `Fill.cpp`'s `resolution` read), while PnP implements only emission-time per-role D-P (`gcode_resolution` 0.0125 / `infill_resolution` 0.04 / `support_resolution` 0.0375) and a collinear-only slicer filter. Re-running the same profile at Orca's canonical 0.001 mm floor (`DynamicPrintConfig::normalize_fdm` clamps anything below it, so "disabled" is inexpressible) costs Orca **+58.4 / +65.2 / +32.1 / +47.3 % wall** (benchy classic-off/on, arachne-off/on) and **+37.5 / +42.8 / +9.5 / +30.9 %** (base), median **+40.2 %**, CPU **+21 % to +92 %**, G-code **+9 % to +91 %**, 3 alternating appendix-verified pairs per cell; `base/arachne-off` wall is noise-limited (CPU +46.3 % stable). The default arm reproduces the historical matched Orca bytes (−0.0003 %). Follow-ups: the map's fairness contract now carries the disclosure, and [81](issues/81-resolution-slicing-time-contour-simplification.md) is graduated as **canonical parity work, not an output-preserving optimization**. Evidence: [`evidence/t80-resolution-fairness/`](evidence/t80-resolution-fairness/FINDINGS.md) (`verify.py` exits 0 checkout-only with 5/5 negative controls rejected).

- [Tree-support planner emit-batch chunk and request-cap sweep](issues/79-tree-support-planner-emit-batch-constant-sweep.md): **resolved (2026-10-22; measurement-only — DROP, no constant changed)** — the chunk sweep found the committed `EMIT_BATCH_LAYERS = 4` at the knee (`com.core.tree-support-planner` module bracket 92,875 / 75,503 / **61,519** / 65,492 / 62,574 / 76,813 ms at 1 / 2 / 4 / 8 / 16 / 32, instrumented screening only), and the cap sweep chose `CLIP_BATCH_MAX_REQUESTS = 4096` on a 5.5 % bracket edge over `256` (58,113 vs 61,519 ms; 3.4 % in an interleaved paired window) after the `64` leader failed to replicate and a pass-3 host-load spike confounded the single-sample cells. **The frozen-snapshot pre-gate confirmation rejected it:** two alternating byte-identical pairs measured the candidate **+4.97 % and +7.63 % slower on wall** (paired median **+6.30 %**, CPU +1.34 %), so the 16-cell proof and 6-pair campaign were **not started** by the stop discipline. Peak working set ~5.3 GB at `4 / 256`. Planner source restored byte-identical to HEAD (blob `a59a11e5…`), both guest freshness gates exit 0, standing `target/dist` and `target/dist-accelerated` rebuilt from the restored HEAD; no production edit, adoption or commit. Evidence: [`evidence/t79-emit-batch-sweep/`](evidence/t79-emit-batch-sweep/FINDINGS.md) (`verify.py` exits 0 checkout-only with 13/13 negative controls rejected; `candidate.patch` is the one-line re-appliable 256 → 4096 diff).

- [Tree-support planner emit-pass batched carve and role union](issues/78-tree-support-planner-emit-batching.md): **resolved (2026-10-22; human-approved KEEP — candidate committed `73a6f1e8`)** — the emit pass buffers four layers of `build_roles` inputs and issues each clip phase as one cross-item `clip_polygons_batch` call (per-region carve, role union, set-wide difference, three body-minus-interface rounds, territory intersect), with `build_roles` kept as the serial reference, `largest_part` shared by both carve paths, and entry order unchanged. **Proof 16/16 PASS** — baseline and candidate byte-identical in every cell *and* equal to the preserved t58 references. **Standing gate PASS in all four pre-declared classic supports-on cells**, 24/24 pairs retained, zero starvation exclusions, every pair byte-identical: wall **−13.41 % / −12.30 % / −7.22 % / −16.66 %** and CPU **+0.51 % / +1.51 % / +0.74 % / +2.80 %** (ordinary/accelerated × benchy/base; accelerated-base's +2.80 % is 0.2 pp under the pre-declared +3.0 % ceiling). Attribution: `PrePass::SupportGeometry` **127,723 → 81,362 ms** and `com.core.tree-support-planner` **109,018 → 60,038 ms**, i.e. the 79.7 s clip core falls to ~30.7 s (**2.6x effective**, not the 12x idealized ceiling); `EMIT_BATCH_LAYERS = 4` and `CLIP_BATCH_MAX_REQUESTS = 256` remain unswept. Evidence: [`evidence/t78-planner-emit-batching/`](evidence/t78-planner-emit-batching/FINDINGS.md) (`verify.py` exits 0 checkout-only with 19/19 negative controls rejected).

- [Tree-support planner native union/carve batch-or-parallel host contract](issues/77-tree-support-planner-batch-parallel-contract.md): **resolved (2026-10-22; measured attribution + contract — no production edit, no timing campaign; one candidate graduated)** — the planner's `clip_polygons` core is **79.659 s of a 126.674 s module stage** (stage `PrePass::SupportGeometry` 147.040 s), and **99.03 % of that core (78.885 s) sits inside `build_roles`**: **472 role unions** (52.4 %; median 79.2 ms, p95 301.6 ms, mean 312 input polygons, max 1,023) plus **69,161 per-region carves** (44.9 %; median 0.474 ms, 1.96 clip polygons). The shapes the fog pointed at are **refuted by measurement**: the `layer_outlines_below` running-union prefix chain is **494 calls / 0.102 s / 0.13 %** (each layer contributes ~1 polygon; the accumulator stays ~1k vertices), t76's intra-call partition has **no target** (no union above 436 ms) and its sliver tolerance does not transfer because the union output *is* printed support geometry, and the 20,275 `ladder_union` calls are 0.662 s. The surviving contract is **cross-call batching** through the existing `clip_polygons_batch` (`crate::batch::map_batch`, fan-out at 4,096 primitives, input-order results): each batch item is the same single operation with the same arguments, so exactness is **per-call identical** (t65 precedent), unlike a partition. Idealized ceiling **72.3 s off a 147.0 s stage** (arithmetic, unmeasured; excludes marshalling 69k requests, buffering memory, imperfect scaling and the 47.0 s of non-clip module time). Probe (`probe.patch`) output byte-identical to the t58 reference `f60e5e9b…`, reverted; both freshness gates exit 0. Evidence: [`evidence/t77-planner-contract/`](evidence/t77-planner-contract/FINDINGS.md) (`verify.py` exits 0 checkout-only, 8/8 negative controls rejected). Graduated: [emit-pass batched carve and role union](issues/78-tree-support-planner-emit-batching.md).

- [Consume-only-when-read annotations](issues/20-consume-only-when-read-annotations.md): **resolved (2026-10-22; human-approved KEEP — candidate committed `053904aa`)** — the only semantic reader of `overhang_quartile` / `overhang_distance_mm` is `OverhangClassifierDefault::run_finalization`, which returns before reading a stamp when `enable_overhang_speed` is false or all four `overhang_{1..4}_4_speed` bands are zero; none of the matched configs authors those keys, so the stamps were unread work. The candidate shares that exact predicate through `slicer_sdk::config_predicates::overhang_annotations_consumed` (the classifier calls the same helper), declares the five keys on both perimeter manifests, and selects `PathAnnotationMode::Unconsumed` when the predicate is false, skipping both per-vertex exact scans. Dynamic premise check: `consumed=false` on 495 base classic-off layers. **Proof 16/16 PASS** against the t58 references; standing gate **PASS in all four pre-declared classic supports-off cells**, 6/6 pairs, every pair byte-identical: wall **−25.93 % / −34.47 % / −14.38 % / −18.10 %** and CPU **−37.67 % / −39.61 % / −24.47 % / −21.58 %** (ordinary/accelerated × benchy/base). `is_bridge` was deliberately untouched (no consumer liveness predicate; t18's hot split is the two scan families). Evidence: [`evidence/t20-consume-only/`](evidence/t20-consume-only/FINDINGS.md) (`verify.py` exits 0 checkout-only with four negative controls rejected).

- [MeshAnalysis exact footprint union](issues/76-meshanalysis-footprint-union.md): **resolved (2026-10-21; human-approved KEEP — candidate committed `465f2fd3`)** — the base overhang union (326,546 facets, 15.0–15.9 s of a 19.5–20.0 s stage) could not be reduced exactly at its inputs (0 duplicate / 0 zero-area facets; the 5.9 % containment drop saves ~1.5 s; any reorder changes the serialized footprint), so the candidate is a work-preserving partition: above 100,000 facets, Morton-ordered centroids split into 48 chunks, unioned in parallel, merged by one final union. Stage **19.5–20.0 → 4.8–5.9 s**; the region differs only by sub-resolution slivers (4.2e-8 relative) and polygon ordering. Proof **16/16 PASS**; standing gate **6/6 pairs in every cell, every pair byte-identical**, wall **−4.46 % ordinary / −5.21 % accelerated** base classic-on, CPU **−0.14 % / −0.93 %**. Evidence: [`evidence/t76-footprint-union/`](evidence/t76-footprint-union/FINDINGS.md) (`verify.py` exits 0 checkout-only with four negative controls rejected).

- [ShellClassification phase-B serial commit](issues/75-shellclassification-phase-b-commit.md): **resolved (2026-10-19; human-approved KEEP — candidate committed `07b97cad`)** — the serial phase-B commit is **78.1 % / 81.0 % / 79.5 % two full-layer Miter offsets** on the three base cells (65.1 % on Benchy), not the `internal_bridge_angles` input step (2.4/1.4/1.8/0.48 s; 8–18 %); the t27 sibling-region memo remains refuted. Only **9 of 121** base entries (13 of 43 Benchy) carry a non-empty lower-candidate bridge set, the sole order-dependent input. The candidate precomputes each layer's exact commit result in the parallel qualification phase and recomputes only those affected layers; no geometry function changed. Candidate stage walls **40.8 → 12.6 s / 28.1 → 12.8 s / 30.1 → 13.6 s / 5.75 → 1.77 s**; proof gate **16/16 PASS** against the t58 references; standing gate **24/24 pairs retained**, every pair byte-identical, wall **−3.08 % / −3.48 % / −3.06 % / −4.11 %** and CPU **+0.42 % / +1.08 % / +0.86 % / +0.51 %** (ordinary/accelerated × benchy/base). Ordinary base's **+1.08 %** is a 0.08 pp miss of the pre-declared +1.0 % work-preserving CPU ceiling; the human accepted it on the ticket-27 serialization precedent. The first timing campaign was evicted after 4/6 ordinary-base pairs; the partial set is preserved as `measure-interrupted-20261019` and not used, and the cell was re-run whole detached. Evidence: [`evidence/t75-shellclass-phase-b/`](evidence/t75-shellclass-phase-b/FINDINGS.md) (`split.json`, `qualification.json`, `ab-reduction.json`, `candidate.patch`, `probe.patch`, `verify.py` exit 0 with four negative controls).

- [LayerFinalization supports-on whole-print view cost](issues/74-layerfinalization-supports-on-view-cost.md): **resolved (2026-10-19; human-approved KEEP — candidate committed `091e6bc9`)** — the supports-on brackets' **96.1 % copy/free churn** (61,555 ms module sum; input snapshot clone 12,112 ms, store drop 16,653 ms, merge rollback clone 13,446 ms, old-vector drop 16,940 ms; guest call 1,816 ms = 2.9 %) is owned by the whole `LayerCollectionIR` clone/free, dominated by the IR-only `support_entity_identities` side table (23,751 rows supports-on vs 0 supports-off), not by module reads or the 1.96x output-byte ratio. The candidate shares the ordered entity snapshot with the view via `Arc` and skips the merge rollback clone only when the order-lock remap is a no-op and the stream has no fallible op (fallible streams keep the rollback; the malformed-merge rollback contract stays green). Probe: **61,555 → 2,775 ms (−95.5 %)**, supports-off unchanged 1,290 → 1,252 ms, outputs byte-identical to the t58 references; proof gate **16/16 PASS**; standing paired gate **PASS** with 6/6 pairs in every cell: wall −3.13 % ordinary-benchy / −12.71 % ordinary-base / −3.89 % accelerated-benchy / −14.23 % accelerated-base, CPU −0.91 % / −2.30 % / −0.84 % / −2.97 %, all 24 pairs byte-identical. Base supports-on wall falls **46.00 s ordinary / 47.52 s accelerated**, inside t73's 45.0–48.5 s bracket attribution. Pre-commit gates: workspace clippy clean, check-literals 0, check-test-quality 0 findings, 24 executor + 10 contract finalization tests pass, both freshness gates exit 0. Evidence: [`evidence/t74-finalization-view/`](evidence/t74-finalization-view/FINDINGS.md) (`qualification.json`, `ab-reduction.json`, `candidate.patch`, `verify.py`).

- [Base current bottleneck refresh](issues/73-base-current-bottleneck-refresh.md): **resolved (2026-10-18; measurement-only take — eight fresh ordinary base captures at post-t70 HEAD, every output byte-identical to its t58 reference; three tickets graduated, t20 park unchanged)** — base is **prepass-dominated**: instrumented wall 214.5/408.7/130.2/360.2 s (classic-off/classic-on/arachne-off/arachne-on), prepass 115.0/237.6/109.2/259.9 s, per-layer 94.3/98.2/16.1/19.8 s, postpass 2.4/69.1/1.4/76.6 s; the common prepass floor is 104–115 s on every cell, supports-on adds 134–149 s of support stages, and ticket 59's Slice verdict (40.0–44.9 s) stands. New leads in recommendation order: **supports-on LayerFinalization view cost** (four whole-print module brackets 10.1–12.5 s each, 45.0–48.5 s total vs 0.30–1.39 s supports-off, while guest fuel is 5.45 B of 12,964 B = 0.042 %), **ShellClassification phase-B serial commit** (26.0–31.4 s; t27's serial commit still 25–30 s), **MeshAnalysis exact union** (19.4–22.2 s serial over 326,546 facets; bbox replacement unsafe). t20 updated with the base context (classic per-layer 94.3/98.2 s; module 98.8/95.0 % of fuel; t18's 57.9 % base scan share) and the below-fold park re-derived as still binding (best cell 3.30x). No candidate, timing run, production edit or production-source commit. Evidence: [`evidence/perf-refresh-base-2026-10-18/`](evidence/perf-refresh-base-2026-10-18/FINDINGS.md) (`verify.py` exit 0 checkout-only + four negative controls rejected; `analyze.py` reproduces the tables).

- [Matched-pair scoreboard refresh](issues/72-scoreboard-refresh.md): **resolved (2026-10-18; measurement-only take — Orca still wins all 8 cells at 3.30x–18.03x, no cell within ~1.5x; DEV-174 taint retired at the output level; two infrastructure evictions disclosed and recovered)** — full refreshed table in the evidence set: median-wall gaps ord/acc — benchy classic-off 5.14/4.34, arachne-off 3.30/3.31, classic-on 5.87/5.09, arachne-on 4.76/4.63; base classic-off 11.48/9.20, arachne-off 7.39/7.24, classic-on 18.03/16.75, arachne-on 15.65/15.50 (n=3 per tool, zero starvation exclusions, CPU corroborating in every cell, PNP 4.5x–41.5x Orca's CPU). PNP ordinary wall improved vs 2026-09-22 in 7 of 8 cells (−13.9 %…−34.6 %; benchy classic-off +5.4 %), while Orca-side walls drifted +1.5 %…+47 % (machine state, disclosed) — ratios are today-vs-today, not a PNP-work claim. Supports-on cells untainted for the first time (all 16 PNP groups `degraded=false`, `non_fatal=0`; ordinary ≡ accelerated bytes). Evidence: `evidence/scoreboard-refresh-20261017/` (`SCOREBOARD.md`, `SUMMARY.txt`, nine curated row CSVs + two `.interrupted.csv` disclosures, `REPRODUCE.md`, `verify.py` exit 0 + self-test).

- [xtask compile-time workspace root poisons gates from alternate checkouts](issues/71-xtask-compile-time-workspace-root.md): **resolved (2026-10-17; verified infra repair, committed `f067efa2`; docs reconciliation this commit)** — ticket 70's poisoning mechanism reproduced in a detached worktree and refined (foreign invocation overwrites the shared `xtask.exe` with a foreign-baked binary; alive foreign root = silent wrong-tree scan, dead = the recorded false green `watchlist: 0 types` + exit 0). Fix: runtime root resolution by `Cargo.toml`+`crates/` walk from CWD (loud panic otherwise; compile-time macro removed from the production path; one fix point covers all subcommands and derived artifact paths) + zero-scan refusal (exit 1) in both gates, every mode incl. `--report` + runtime-first `#[cfg(test)]` helpers. Foreign-built binary proof: CWD=main → real 159-type watchlist, exit 0; CWD=void → exit 101 loud. `cargo test -p xtask` 185/0 (9 new), workspace clippy 0, check-literals 159-green, check-test-quality --report 0 findings, build-guests --check exit 0. Evidence: [`evidence/t71-xtask-worktree-root/`](evidence/t71-xtask-worktree-root/FINDINGS.md).

- [Clipper engine vendoring and hot-path optimization](issues/69-clipper-vendoring-optimization.md): **resolved (2026-10-16; headroom take — measured headroom, not parity; one candidate graduated, none implemented)** — a dual-engine harness (pristine sealed `clipper2-rust` 1.1.0 vs canonical C++ 1.5.2, MinGW g++ -O2) over a fresh host-side input-dump corpus of the 11 t68 sites (both matched jobs, capture runs byte-identical to the t58 references) plus the t55 body-union corpus measured, on 295 cases × two passes (forward + reversed), **Rust/C++ median ratios offset 2.03–2.09x / union 2.61–2.63x / intersection 1.94–2.00x / difference 1.60–1.76x** (every site above 1 in both passes; 46/295 ordered-hash divergences = the known t31 `clean_collinear` delta, disclosed not investigated). Idealized base-off accumulated ceilings if Rust matched C++: offset 256.5–258.3 s / closing_ex 198.9–200.3 s / clip_polygons 79.5–81.6 s (t60 scope totals re-derived at take time); directly-sampled CP-only ceilings ≈ 46.7–46.8 s base / 2.5–2.7 s benchy, an occupancy-style diagnostic scale of ≈ 1.48 % / ≈ 1.21–1.28 % against t68's instrumented walls — above the ≤ 0.80 % below-floor context, hence candidate-justified. Coverage caveats stated on the ticket (RP leg captured intersections only; region-prep closing/offset ceilings conditional on within-opclass transfer; no engine-internal cost center measured). Probe reverted, both freshness gates green, `verify.py` exits 0 checkout-only with four negative controls rejected. Evidence: `evidence/t69-clipper-engine/` (FINDINGS, HARNESS, CAPTURE, corpus, results, drivers, verifier). Graduated: [70](issues/70-clipper-in-tree-vendoring-first-candidate.md).

- [Clipper in-tree vendoring and first value-exact engine candidate](issues/70-clipper-in-tree-vendoring-first-candidate.md): **resolved (2026-10-17; human-approved KEEP — candidate committed `7127b4d6`)** — vendored `clipper2-rust` 1.1.0 in-tree as a plain workspace member with sealed provenance (`vendor-provenance.json`; byte-identical registry extraction, explicit `path` dep so guest freshness fingerprints cover engine sources), profiled the engine internally on the t69 corpus (all phases at parity except `do_top_of_scanbeam` 2.19–4.26x and `build_intersect_list` 1.25–2.26x vs C++; root cause = software `nearbyint` round-half-to-even emulation on the per-scanline `top_x` path), and shipped a 44-line candidate (`nearbyint_f64` finite fast path via `round_ties_even`, MSRV 1.77): value-exact 295/295 corpus cases, 16/16 proof cells byte-identical to the baseline arm AND to the t58 preserved standing references, and the standing paired gate **PASS in all four pre-declared classic-supports-on cells, both modes (6/6 pairs, zero starvation exclusions): wall −15.59 %/−13.70 %/−16.30 %/−18.30 %, CPU −12.25 %/−13.45 %/−13.71 %/−16.82 %** (ordinary/accelerated × benchy/base; order as on the ticket). Candidate-built standing binaries purged per the t36 precedent (rebuilt from the pre-take HEAD in a detached worktree, copied byte-exact back; candidate sha set absent; both freshness gates exit 0) and refreshed at the kept HEAD after the commit (force path; the refreshed standing module byte-matches the take-era measured candidate module `a04a77f7…`); FINDINGS has the full measured record. A v1 scanbeam-walk candidate was measured value-exact, a wash, and dropped. Closeout disclosure: xtask's compile-time-baked root lets a worktree invocation silently zero out the main tree's pre-commit gate scans (contained + upstream follow-up named on the ticket). Evidence: `evidence/t70-clipper-candidate/` (FINDINGS, vendor-provenance.json, candidate.patch, profiling/, results/, campaign/ with qualification.json + ab-reduction.json, verifiers in-repo).

- [Classic-perimeters per-layer host-service round trips](issues/68-classic-perimeters-host-round-trips.md): **resolved (2026-10-16; attribution take — no candidate; the bucket is 97.55 %/99.02 % native clipper ops)** — the T68-PROBE (two-sided: a dispatch-recorder plus per-call conversion/ops/conversion split in the host services, `now_us` site brackets in the guest; `probe.patch`, reverted) decomposes all 8 offsets + 3 clips per layer on benchy-off (5,726.4 ms guest-observed, 2,640 calls) and base-off (87,900.3 ms, 5,442 calls; both outputs byte-identical to the t58 references `34886ccb…`/`20e7204c…`): **ops 97.55 %/99.02 %, host conversions 0.23 %/0.11 %, guest marshalling + ABI 2.23 %/0.86 %**. Site shares (base): A0 21.4 %, A1 22.3 %, K 9.6 % — the walls/infill output themselves — and the gap machinery B–J 46.8 % feeding the width-band morph (98.6 % of raw candidate vertices discarded; gap fill emitted on 162/495 layers). Every t60-named direction's ceiling measured below any gate: migration ≤ remainder+conversions minus the wasm-ops penalty; batching ≤ per-crossing fixed costs on a saturated phase; site-level has no dead/duplicate site and no exact algebraic shortcut under byte-exactness. Wall relevance: 6.3 %/6.6 % of per-layer phase occupancy; the removable overhead ≤ 0.33 %/0.80 % serial and ≤ 0.03 %/0.07 % saturated of run wall. Thin-wall clip never fires (0 calls, dynamically confirmed); base layer 270 skips B/C/D. Two-run ±11 % accumulated drift with ~1-point-stable shares; t60 cross-check −1.5 %/−3.1 %. No production edit, no timing campaign, no auto-commit; probe reverted, freshness green. Evidence: `evidence/t68-host-roundtrips/` (`FINDINGS.md`, `probe.patch`, `run-t68.ps1`, `captures/`, `verify.py` exits 0 with seven negative controls rejected).

- [Region-preparation derivation sharing across region projections](issues/67-region-preparation-derivation-sharing.md): **resolved (2026-10-16; measured GATE NOT MET on one CPU-corroboration clause — human-approved KEEP, committed `f4c542a3` with tests)** — `LayerArena` serves the perimeter and infill region projections from one prepared-region derivation when the perimeter source projection onto the regions is the identity (modifier sub-regions that restore base geometry keep the two derivations separate; no second cache, no clone). Qualification: 16 proof cells byte-identical (`build-qualification.py` pass 16/16, `verify-references.py` 16/16 against the preserved t58 references). Standing paired gate on the four pre-declared classic supports-on cells (56 timed runs, 6/6 pairs everywhere, zero starvation exclusions): paired-median wall Δ −0.11 % ordinary-benchy / **−4.26 % ordinary-base** / −0.50 % accelerated-benchy / −0.28 % accelerated-base; CPU Δ −0.44 % / −0.60 % / **+0.19 %** / −0.11 %. Read plainly: the only delta outside run noise is ordinary base-classic-on (−4.26 % paired and −4.12 % unpaired, 4 of 6 pairs by 2–9 %, CPU-corroborated ≈ −14 s, consistent with t60's attributed duplicate derivation); ordinary-benchy and accelerated-base flip sign between paired and unpaired medians (±0.3 %, inside noise) and the benchy cells were pre-declared at/below the 0.1–0.5 % floor; the single clause miss is +0.31 s of CPU on the 118 s accelerated-benchy slice. Evidence: `evidence/t67-region-prep-sharing/FINDINGS.md` (+ `qualification.json`, `ab-reduction.json`).

- [host:native attribution: name the per-layer native geometry bucket](issues/60-host-native-attribution.md): **resolved (2026-10-14; bucket fully attributed — residual `host:native` = 0 with exact per-scope call reconciliation in all four classic cells; probe reverted, no production edit)** — four fresh ordinary profiled captures (outputs byte-identical to the t58 references; clean) carry the temporary `T60-PROBE` host instrument (thread-local sink owner + worker-thread owner for rayon batch items; `probe.patch`, reverted). Owners: the two region-preparation derivations (`ensure_prepared_regions` / `ensure_prepared_perimeter_source_regions`; 1.48+1.47 s benchy-off, 40.53+40.18 s base-off accumulated, once per layer, 661/1,926 clips each), `region_partition`'s `sync_perimeter_infill_areas_into_slice` 3×/layer (1.26 s benchy-off / 10.04 s base-off; 6,120/13,320 clips), guest-requested host services from per-layer dispatches (classic-perimeters 5.91/89.26 s; infill-linker 0.53/3.10 s; supports-on tree-support 3.79/40.29 s), the prepass seam-planner (0.95/13.67 s) and — supports-on — the support machine now named rather than suspected: tree-support-planner 6.49/160.33 s, its rayon batch-offset items 0.25/2.74 s (24,255 items = ticket 22's independent count), and support-plan aggregation clips 0.22/2.09 s. The supports-off headline bucket (benchy 11.60 s / base 176.78 s accumulated; base-off baseline was 197.2 s) is now a named split, not an unknown. One candidate graduated:
  [Region-preparation derivation sharing across region projections](issues/67-region-preparation-derivation-sharing.md) (the two derivations are identical work over identical region sets on the matched jobs; ≈80.7 s accumulated base-off); named but un-graduated leads (classic-perimeters host round trips; `region_partition` dedup) are in the map's fog. Evidence: `evidence/t60-host-native/` (`FINDINGS.md`, `summary.json`, `captures/`, `probe.patch`, `run-t60.ps1`, `verify.py` exits 0 from the committed set alone with six negative controls rejected).

- [PrePass::Slice flat-bridge closing reduction](issues/59-prep-slice-flat-bridge-closing-reduction.md): **resolved (2026-10-14; verdict (b) — no representation-safe reduction; no production edit, no timing campaign)** — the closing is consumed only as the per-component ≥10 % gate; on real layers that gate is decided by miter-offset artifacts from ~72–74 degenerate sliver paths in `difference`'s output (drilled: benchy 161 raw 325.3 mm² vs 215.4 normalized on the same point set; base 8 raw 31.5 mm² vs 72.1 normalized — on base the raw closing is smaller than its own input), so a set-preserving local-closing candidate flips verdicts on 92/239 benchy (38.5 %) and 320/469 base (68.2 %) call layers — refuted; far-guards have zero hits (0 of 14,951 components); baseline gate margins reach 4e-5; the refuted shape's measured ceiling is ≈12 % of the closing's accumulated thread-time. Evidence: `evidence/t59-flat-bridge-closing/` (`FINDINGS.md`, `probe.patch`, `reduced/`, `verify.py` exits 0).

- [Support paint-policy dead-walk reduction](issues/66-paint-policy-dead-walk-reduction.md): **resolved (2026-10-14; measured GATE NOT MET on the wall clause — human-approved DROP, candidate retired, no production edit)** — the annotations-empty fast path (`slicer_core::paint_policy::has_support_paint` plus the early return in `PaintRegionLayerView::paint_policy_for`) passed the full 16-cell output-identity qualification (baseline == candidate == preserved t58 references; `build-qualification.py` 16/16 and `verify-references.py` 16/16, all clean), then the standing paired gate on the four pre-declared classic supports-on cells (6/6 pairs everywhere, zero starvation exclusions, 56/56 runs byte-identical): **CPU improved in all four cells (−0.56…−1.22 %)** and **base wall improved (−0.4977 % ordinary / −2.1098 % accelerated)**, but **benchy wall sat inside paired noise (+0.0017 % / +0.1272 %)**, below the pre-declared wall < 0 % clause — the dead walk is a real ~1 %-class CPU cost whose wall transfer is base-only. Candidate retired to `HEAD` with the re-appliable artifacts committed (`evidence/t66-paint-walk/candidate.patch` sha `8ccfbeb1…` + the new-test-file copy, both byte-identical to the as-measured state); no adoption retry; re-entry needs a new pre-declaration. Interruption recoveries disclosed (one harness eviction kill mid-proof, two further evictions survived by the detached relaunch; background load; zero exclusions). Evidence: `evidence/t66-paint-walk/FINDINGS.md`.

- [Tree-support renderer self-fuel and scan-fill attribution](issues/64-tree-support-renderer-attribution.md): **resolved (2026-10-14; attribution take, probe reverted, no production edit, no timing campaign)** — four fresh profiled captures (outputs byte-identical to the t58 references, all four cells); the renderer's 43.510 B dispatch fuel splits: **`paint_policy_for` 34.697 B (79.7%)** — per-query counters pin a **dead walk** (1 rebuilt region / ~428 vertices / ~1,285 rings / ~893K edge tests × ~46.5 instr each; **0 covers, 0 eligibility checks in 836 queries** → `DefaultEligible` every time), two exact reduction shapes graduated to [Support paint-policy dead-walk reduction](issues/66-paint-policy-dead-walk-reduction.md); **`skeleton_wall_count` 6.921 B (15.9%)** (bbox-scan walks 377.7 M of it; ranked second); **`scan_fill_region` 0.187 B (0.43%) inclusive — the ticket's named suspect measured no-useful-reduction** (4,443 rows / 2.8 M edge walks / 2.008 conversions per edge; role splits exact); wall relevance bounded: renderer accumulated worker 6.79–8.35 s over 12-thread phases = **6.5–8.5% machine occupancy**, 0–2 ms on the last-finishing layer, freeing the dead walk's share bounded ≈0.45–0.55 s of those phases' walls (attribution-only bounds, never a wall claim). Supports-off controls match the t61 baselines with the probe silent (0 counts). Ordinary mode only (disclosed); the graduated experiment carries the both-modes paired gate. Evidence: `evidence/t64-tree-support-renderer/` (`FINDINGS.md`, `probe.patch`, `verify.py`).

- [Independent low-layer dispatch and Arachne finishing-tail reduction](issues/63-low-layer-dispatch-granularity.md): **resolved (2026-10-13; diagnosis/design, no production edit, no timing campaign)** — the granularity mechanism is confirmed with worker-level evidence (a reverted env-gated probe, `evidence/t63-lowlayer-dispatch/probe.patch`): at 12 threads layers 0/1/2 are rayon's initiator left spine on worker 0 in all five fresh runs (0,1,2,3 at six threads), module-pool waits are zero at ≤12 threads (every per-layer-tier manifest is `layer-parallel-safe`; waits measured only at 24 threads, max 42 ms), and the 24-thread counterfactual collapses the serialization entirely (layer 1 stolen at ≈0 ms, layer 2 ending mid-field — same job/binary, only steal width). **The premise's measured opportunity is stale at HEAD**: the per-layer-phase scheduling deficit collapsed from 1,079–3,022 ms (12.7–49.9%) across the six pre-t65 t61 captures to 161–969 ms (2.4–7.0%) post-t65 with the lone-open tail gone (0–61 ms), because committed t65 cut the low-layer linker chunk the spine held (layer-1 linker dispatch 3,191 → 1,228–1,662 ms; layers 0–2 accumulated 4.50–6.23 → 1.39–1.80 s, ≈3–4×). Recommendation: do **not** open the output-identical dispatch-granularity experiment now (measured ceiling ≈0.12–2.3% of Benchy slice wall, below the map's ~1%-effect bar); the generic candidate shape (`with_max_len(1)`-class single-item leaves; bounded-chunk designs measured the wrong direction) and the re-open condition (a fresh scheduling baseline re-developing a 40%+-class deficit) are recorded on the ticket. Fresh scheduling baseline for 63's family: taken (the probe runs); outputs byte-identical to the t58 references; loader/provenance checks pass. Evidence: `evidence/t63-lowlayer-dispatch/` (`FINDINGS.md`, probe patch, `verify.py`).

- [Infill-linker batch-execute experiment](issues/65-infill-linker-batch-execute-experiment.md): **resolved (2026-10-13); human-approved KEEP — committed with this ticket** — one batched `Clipper64` + execute per `link_paths_without_offset` invocation with re-derived per-subject attribution (`PreparedPolylineClip::clip_batched`, `crates/slicer-core/src/polygon_ops.rs`; per-subject `clip` repair for ambiguous junction-kiss subjects, whole-call fallback reserved for unattributable segments). In-guest replay-oracle probe: 0 mismatches over 12,022–19,290 subjects per cell, both modes, outputs byte-identical to the preserved references; 10 differential tests. Qualification 16/16 byte-identical, then the standing paired gate on the 8 pre-declared arachne cells (6/6 pairs everywhere, zero starvation exclusions): **benchy arachne wall −20.01/−21.08 % (off) and −12.24/−10.84 % (on), CPU corroborating; base cells a wash** (−0.07/−0.52/−1.65 %; ordinary-base-arachne-on +0.43 % median loss, ranges overlapping) — GATE NOT MET on the letter, the failing cell explicitly accepted in the human's ruling. Production artifacts already in the candidate state; both freshness gates green. Evidence: `evidence/t65-linker-batch/FINDINGS.md`.

- [Infill-linker post-hoist residual and exact clip-work reduction](issues/62-infill-linker-post-hoist-residual.md): **resolved (2026-10-13)** — the t44 hoist confirmed working (prepare now 2.60% of linker fuel, once per invocation); the post-hoist cost is the **per-path engine round trip: `execute` 85.96% + `engine_add` 4.42% of linker fuel** across 12,022 clip calls / 393 invocations, with the prepared universe re-fed **57.3× per preparation** (classic 49.5×) and per-call cost universe-dominated (fit −2.8M + 7,990·U, R² 0.869). Qualification: 8/8 cells byte-identical both modes before the probe; all six probe captures output-neutral; cross-mode scope agreement ≤5.4e-4 relative. Candidate (batch one engine + one execute per invocation with re-derived per-subject attribution, exactness conditions measured and documented) **graduated to** [Infill-linker batch-execute experiment](issues/65-infill-linker-batch-execute-experiment.md); same-cost ceiling ≈12.58% of current linker fuel, slice share 44.28% → ≈5.57% (estimates, not measurements). Attribution/representation only; production restored, freshness green. Evidence: `evidence/t62-linker-post-hoist/FINDINGS.md`.

- [Benchy current bottleneck refresh after clone-once finalization](issues/61-benchy-current-bottleneck-refresh.md): fresh output-identical Benchy attribution graduates the post-hoist linker residual, low-layer dispatch granularity and support-renderer questions; existing leads retained, no optimization or speed verdict.

- [LayerFinalization whole-print copying](issues/58-layerfinalization-whole-print-copying.md): **resolved (2026-10-12); human-approved KEEP — committed with this ticket (`e7159687`, docs `e4e93dea`)** — the clone-once finalization stage + owned pre-emit capture candidate (three runtime files) passed the fresh 16-cell output-identity qualification (ordinary cells reproduce the preserved production references; base-arachne-on fresh ref `a1bc8110…`), then the standing paired gate **in all four pre-declared cells, 6/6 pairs each, zero starvation exclusions**: base-classic-on wall medians **−10.48 % ordinary (610.0 → 539.5 s) / −8.76 % accelerated (601.0 → 547.9 s)**, CPU corroborating (−2.33 % / −2.45 %); benchy-classic-on −1.21 % / −1.44 % with byte-identical outputs everywhere. Attribution: LayerFinalization stage 113.2 → 70.4 s (−37.7 %), inter-bracket drop gaps → 0 ms, one 2.7 s upfront clone + 10.1 s commit tail; postpass phase 132.3 → 87.4 s. Campaign context disclosed (background load; three interruption recoveries incl. a harness-timeout raise to 1800 s, infrastructure-only); snapshots restored, freshness gates green. Evidence: `evidence/t58-finalization-copies/FINDINGS.md`.

- [Serial host floor](issues/27-serial-host-prepass-floor.md): **resolved (2026-10-09)** — the post-park premise search found the new safe call reduction the ticket waited for: 99.8% of `commit_support_analysis_builtin`'s serial sweep (`crates/slicer-runtime/src/builtins/support_analysis_producer.rs`) was one predicate — `SliceRegionView::derive_needs_support`'s per-region exact `intersection_ex` against the object-global overhang footprint (`crates/slicer-sdk/src/views.rs`), 24.19 s of the 38.20 s base supports-on stage wall — now moved to an order-preserving indexed rayon pre-pass (dual-compute 495/495 base + 240/240 benchy regions, 0 mismatches; 8.7x on the predicate). Standing paired A/B: base median wall **−18.198 s ordinary / −19.380 s accelerated** (6/6 and 5/6 pairs) at ≈+0.5% median process CPU; benchy −1.105 s / −0.943 s. All 48 outputs byte-identical; the strict wall-and-CPU clause passes on ordinary/benchy only and the **human KEPT the base wall win 2026-10-09** (≈3.4–3.7% of arm-median wall; accepted as a serialization fix, not a scoreboard claim), committed with the ticket. The probe-3 bbox-index filter alternative was refuted (~45% pruning, ~12% saving) and the `from_ir` deep-clone hypothesis measured dead. Remaining leads (flat-bridge representation analysis, `apply_opening` parity, teardown early release) stay in the fog. Evidence: `evidence/t27-serial-floor/FINDINGS-PROBE3.md`.

- [offset2_ex call reduction](issues/19-offset2-ex-call-reduction.md): the reopen's premise was re-derived at HEAD and **is stale** — the ticket's 720-call `min_width_top` headline site does not execute on the matched classic job (0 calls; 13 layers / 0.037% of module fuel with its gate forced on), and its other two named shapes are dead or not output-preserving. The surviving candidate is a shape the ticket did not name: the `classify_narrow_island` opening-emptiness short-circuit (two-pass `opening_ex` → shared single-erosion predicate; site calls 720 → 480, attributed module fuel 750.03 B → 732.43 B ≈ −2.35%; output byte-identical). The standing paired A/B **passes both modes** — ordinary −0.65% median wall / −2.08% median CPU, accelerated −1.47% / −1.04%, six retained pairs per mode, no starvation exclusions, all 32 outputs byte-identical to `34886ccb…` — with the effect small and not a gap-closer. Measured **KEEP approved by the human and committed with this ticket (2026-10-08)**; both freshness gates green. A provenance constraint was measured: any text change to `polygon_ops.rs` shifts the accelerated guest's artifact hash (name-section `.llvm.<hash>` suffixes) even for a comment, so the shipped source keeps the measured bytes and takes a manifest-level lint allow instead of rewriting the expression. Evidence: [findings](evidence/t19-offset2-call-graph/RESULTS.md) and the ticket's Answer.

- [Large body-union exact corpus and native-internals investigation](issues/55-large-body-union-corpus-and-internals.md): largest/hole-bearing full-role replay and paired-mode output identity pass; native sweep counters establish unchanged-edge `TopX` recomputation and recommend a separately scoped projection cache, while exact polygon deduplication is falsified for the selected corpus. No candidate or speedup claim; sources/artifacts restored and both freshness gates pass.

- [Large body-union investigation scope](issues/54-large-body-union-investigation-scope.md): human authorized immediate largest/hole-bearing real-input capture, exact full-role replay and native-union internal attribution on the unchanged paired-mode matched jobs; production candidate and speed campaign remain separate approvals.

- [Tree-planner union/simplify current-job attribution](issues/53-tree-planner-union-simplify-current-job-attribution.md): measured all four cells with byte-identical clean baseline/probe outputs; simplification never executes, native body union dominates, and no expensive removable call was established. Recommendation: park simplify/four-role batching; a separate large-body-union scope must address largest/hole-bearing replay gaps. Production sources/artifacts restored and both freshness gates pass.

- [Tree-planner union/simplify investigation scope](issues/52-tree-planner-union-simplify-investigation-scope.md): human authorized fresh Benchy/base classic supports-on attribution in both modes, preserving the exact current chain and qualifying temporary probes against fresh outputs; diagnostic execution is separate, with no candidate or speed campaign authorized.

- [Prepared winding-coordinate cache: exactness + standing paired A/B](issues/51-prepared-winding-coordinate-cache-experiment.md): **DROP approved by the human (2026-10-06)** — the exact per-context contour-coordinate cache passed seven exactness tests in both modes, the precision negative control, byte-identical current outputs, and all code gates, but the standing both-modes wall/CPU gate failed on accelerated wall: the initial campaign was inconclusive (2/6 ordinary, 1/6 accelerated pairs retained) and the one human-authorized repeat retained 6/6 in both modes with ordinary passing (−1.35% wall / −2.08% CPU medians) and accelerated failing (+1.35% wall / −1.16% CPU, ranges overlapping). Candidate source/tests retired to `HEAD`, re-appliable patch and both campaigns preserved, rebuilt guests byte-identical to archived baselines, both freshness gates green. No commit, adoption retry, one-sided exception, further retry or new ticket.

- [Prepared winding-coordinate cache scope](issues/50-prepared-winding-coordinate-cache-scope.md): human authorized one eager per-context contour-coordinate cache experiment for signed-distance sign on the same supports-off Benchy Arachne job, with independent exactness and the unchanged paired-mode end-to-end wall/CPU gate; execution graduated separately, no candidate or speed claim yet.

- [Signed-distance sign-versus-distance attribution](issues/49-signed-distance-sign-versus-distance-attribution.md): fresh paired-mode probes localize the accelerated residual to exact winding classification; output checks, non-overlapping fuel accounting and production restoration pass. An exact prepared winding-coordinate cache is recommended for a separate scope decision, not implemented or speed-approved.

- [Signed-distance investigation scope](issues/48-signed-distance-investigation-scope.md): human authorized fresh supports-off Benchy Arachne sign-versus-distance attribution in ordinary and accelerated modes; diagnosis only, with current-output checks. No candidate implementation, speed campaign, commit, adoption retry, or default change authorized.

- [Overhang-quartile query: predicate narrowing + standing paired A/B](issues/45-overhang-quartile-predicate-narrowing.md): **DROP approved by the human (2026-10-05)** — blanket boundary-pass removal changes exact-edge verdicts; the output-identical inclusive-bbox alternative fails the standing paired-mode speed gate on ordinary wall. Candidate code/tests retired together into a re-applicable patch, evidence preserved, active distributions restored; no commit, adoption retry or default switch.

- [Frozen-job output drift boundary](issues/46-frozen-job-output-drift-boundary.md): **intended behavior, not a defect (2026-10-05)** — the frozen→current drift decomposes into exactly two documented intent programs: t44-era→current is identical move stream (92,519 moves) + the t47 fan repair (+277 lines) − the `support_overhang_angle` alias line (30 B), with `current + alias == 11e7d38a…` (t44 job) and `== e1088b3d…` (classic-off) byte-for-byte; frozen→t44-era is the config-scope-resolution resolved-view program (start block / fill 45° / wall F3000 / registry CONFIG_BLOCK 102→240 keys). All eras re-derive their own references (`7049a06d…`, `1b71f83d…`), all complete clean, zero containment violations. **Boundary: frozen `7049a06d…` and t44 `1b71f83d…` retire as timing baselines for this job; current refs are `5a61a106…` (t44 job) and `e1088b3d…`+alias (classic-off); fan-bearing jobs need post-t47 refs.** Ticket 45's gate prerequisite is satisfied; not map-wide authorization.

- [Native postprocess-view enrichment repair](issues/47-native-postprocess-view-enrichment.md): **KEEP approved by the human (2026-10-03); t33 oracle gate re-run from proof PASSES (benchy-classic-off, byte-identical arms `e1088b3d…` across proof + 3 ABBA pairs; frozen t44-job corroborates `11e7d38a…`)** — three native-dispatch projection gaps fixed: (1) the scoped `Layer::InfillPostProcess` partitioned-`SliceIR` view enrichment (`native_infill_postprocess_regions`), (2) the ordered-entities snapshot the native path-optimization/anchored-events arms never received (travel reordering silently skipped), and (3) the finalization WIT annotation channel whose absence made the EXTERNAL leg silently drop part-cooling's `M106`/`M107` fan commands on every job (package `slicer:finalization-layer-finalization` 1.0.0→1.1.0; disclosed intentional output change on BOTH legs — pre-repair external baselines like t44's `1b71f83d…` are outdated for fan-bearing jobs; post-repair references: classic-off `e1088b3d…`, t44-job `11e7d38a…`). Integrated timings are usable as an oracle on the examined cell, bounded to it; timing campaigns/adoption/default changes still need their own authorization.

- [Integrated/external matched-output oracle gate](issues/33-integrated-external-matched-output-oracle-gate.md): **gate FAILED (2026-10-02)** — integrated dispatch is not output-equivalent; the native `Layer::InfillPostProcess` request omits the partitioned-`SliceIR` view enrichment the WASM leg performs (`push_infill_postprocess_regions`), the infill linker silently falls back to the union boundary, and the native leg prints paths the external leg clips away (+572.5 mm sparse, +698.7 mm top surface etc. on the t44 job), reproducing Finding 3's TYPE deltas; integrated timings stay disqualified. Enabling repair: `raft-default` got its missing 24th integration row (packet-240b registry drift; `xtask dist --edition integrated` was unbuildable before it). Named fix scoped and now taken as [Native postprocess-view enrichment repair](issues/47-native-postprocess-view-enrichment.md) — **superseded there: the gate now PASSES from proof (2026-10-03, see that ticket's answer)**.

- [Clip-universe preparation hoist](issues/44-clip-universe-hoist.md): **KEEP approved (2026-10-02)** — immutable per-invocation preparation preserves the tested fresh-baseline output and passes the standing paired ordinary + accelerated wall/CPU gate; no adoption retry or default-mode switch, with pre-existing frozen-job drift disclosed separately in [Frozen-job output drift boundary](issues/46-frozen-job-output-drift-boundary.md).

- [Arachne graph-construction attribution](issues/25-arachne-graph-attribution.md): **the module's cost is two guest per-vertex spatial queries — 86.4% of module fuel (`signed_distance_to_boundary` 48.3%, `overhang_quartile` 38.1%) — not the host pipeline, which is only ~18% of module elapsed** (the 75.3% ticket 06 measured was of the enclosing pipeline interval, not the module); inside the host service preprocess is 43.3% (stage-1 triple offset 41.6%) and graph construction 38.0% (boostvoronoi sweep 33.5%), closing the old 364.932 ms remainder; acceleration cuts the module 1.73x but the two queries still hold 82.0% accelerated; one candidate — the `overhang_quartile` predicate's `eps = 0.0` boundary pre-pass measures 56.5–71.2% of the predicate across four preserved synthetic-ring runs, fuel ceiling ≈ 21–27% of module fuel as a cross-domain bound — awaits its own scope + paired A/B, with no fix, commit or acceptance retry.

- [infill-linker attribution](issues/21-infill-linker-attribution.md): the linker's cost is 97.65% the per-path re-clip — 64.84% of linker fuel is the per-call clip-universe pre-inflate, 32.65% the clipper execute — and the layers 1–2 tail is ~99.95% guest execution (host prep/marshalling ~1.7 ms); one candidate (hoist the universe preparation per invocation, fuel ceiling ≈ 61.4% of linker ≈ 28.1% of slice by the uniform-count estimate, 63.7% of linker layer-aware, both estimates under a same-cost assumption) awaits its own scope + paired A/B, with the split mode-invariant and the linker the #1 accelerated fuel consumer at 59.1%.

- [Arachne critical-tail module attribution](issues/43-arachne-critical-tail-module-attribution.md): authorized diagnostic pair completed with clean byte-identical output and frozen inputs; infill-linker dispatch occupies 90.01–96.69% of layers 1/2 (its host-vs-guest subcosts were subsequently split by [infill-linker attribution](issues/21-infill-linker-attribution.md): ~99.95% guest); no acceptance retry or production change.

- [Accelerated adoption route after inconclusive campaign](issues/42-accelerated-adoption-inconclusive-route.md): human chose saved-evidence-first Arachne investigation, then authorized the diagnostic pair; ordinary remains production pending unchanged acceptance gates.

- [Accelerated production adoption acceptance campaign](issues/38-accelerated-adoption-acceptance-campaign.md): controlled snapshots and one real campaign completed; adoption remains inconclusive after Arachne wall overlap, with four cells unrun; route returned to the human, ordinary unchanged.

- [Missing frozen adoption inputs recovery decision](issues/41-missing-frozen-adoption-inputs-recovery.md): human accepted the new validated ordinary-only frozen set outside `target/` and the recorded preparation-only preflight restart exception; campaign continuation authorized under unchanged gates.

- [Current-job adoption reference preparation policy](issues/40-acceptance-reference-refresh-policy.md): human-authorized ordinary-only references prepared and frozen for the unchanged six-cell job, with clean-output and targeted repair checks; historical corpus preserved, full adoption acceptance still pending.

- [Acceptance runner CSV status preservation](issues/39-acceptance-runner-status-roundtrip.md): real CSV non-fatal counts and validated generator markers now survive the runner gate; regression checks pass without relaxing adoption gates.

- [Accelerated perimeter-spatial adoption decision](issues/13-accelerated-adoption-decision.md): human confirmed conditional accelerated adoption under the existing exactness and strict CPU/wall campaign gate; ordinary remains the default pending acceptance.

- [Baseline and measurement recipe](issues/01-baseline-and-measurement-recipe.md): the three-fixture PNP-vs-Orca baseline is captured but configuration-unmatched (no speed multiple is defensible from it), and the six measurement traps plus reproduction commands are the standing protocol.
- [shell_classification guard ordering](issues/02-shell-classification-guard-ordering.md): the stage's cost was two full-layer `offset` calls computed before the early-outs that discard them; guards hoisted, erosion made lazy, bridge gate parallelized — benchy stage 17.8 s → 2.96 s, landed.
- [Allocator A/B closes the contention hypothesis](issues/03-allocator-ab-contention-hypothesis.md): mimalloc/snmalloc show no wall win and +5–23% peak memory; the inflated-CPU reading was external machine load in summed worker elapsed. Do not repeat without new contention evidence.
- [Profile-first refresh and dispatch split](issues/04-profile-refresh-and-dispatch-split.md): prepass dominates and tree-support-planner is the largest serial module; the user chose the perimeter/dispatch split, which attributed ~97% of clustered per-layer dispatch cost to host-side marshalling.
- [Prepared-region arena cache](issues/05-prepared-region-arena-cache.md): host region preparation memoized in `LayerArena` (landed `bf11d055`) — ~66% of preparation calls served from cache, process CPU −3.8% benchy / −5.8% base, guest work untouched. KEEP.
- [Perimeter attribution study](issues/06-perimeter-attribution-study.md): classic fuel 85.8% wall assembly / 8.0% gap fill; Arachne interval 75.3% preprocess/graph construction — attribution only, and pre-ticket-07 (re-attribute before trusting at HEAD).
- [Wall-flags annotation-free fast path](issues/07-wall-flags-annotation-fast-path.md): accepted and committed on repeatable process-CPU savings (wall benefit never proven; acceptance revised for this candidate only); follow-up audit also caught and repaired the quiet harness's Classic-labeled-Arachne bug.
- [emit_walls premise falsified and classic re-attribution](issues/08-emit-walls-premise-falsified.md): the inset clone/store is 0.0014% of module fuel (lead closed); classic's real cost is the per-vertex spatial queries (69.8% of guest fuel, ordinary mode) with `offset2_ex` at 15.2% — leads re-ranked.
- [Accelerated-mode pair](issues/09-accelerated-mode-pair.md): acceleration cuts total guest fuel −35.8% but the hot queries only ~1.9× (necessary, likely not sufficient); ranking rescales (classic self 60.4%, `offset2_ex` 23.7%, infill-linker 15.3%), and the accelerated-snapshot recipe traps are now recorded.
- [Unreachable batch-query dead end](issues/10-unreachable-batch-queries-dead-end.md): spatial-indexing `raycast_z_down_batch` / `surface_normal_at_batch` would optimize an unreached path (no production callers). Do not pursue.
- [clipper2 1.1.0 upstream evidence](issues/16-clipper2-1-1-0-upstream-evidence.md): 1.1.0 is purely additive (PolyFace64 face extraction) — polygon-op cost and output for identical inputs unchanged, and `check_split_owner`'s unbounded recursion is byte-identical with unchanged reach in both versions.
- [Matched-pair rig and first scoreboard](issues/11-matched-pair-rig-and-scoreboard.md): the matched job (0.4/0.20 mm, 2 walls, 20% gyroid, tree(auto) supports, per-cell generator) is rigged with per-run output-evidence validation and measured on all 8 cells — Orca wins every cell (median wall 6.3–26.2x, process CPU corroborating), accelerated mode buys 0–16% wall, and base supports-on is tainted by DEV-174 (`degraded=true`, 172,181 non-fatals); scoreboard in `evidence/matched-pair/SCOREBOARD.md`.
- [Support-correctness repair](issues/14-support-correctness-repair.md): the DEV-174 dropped-band defect is fixed and kept — the complete-body extent gate measured identity aggregates as one body against the deleted routing cell's 104.86 mm cap; now measured per body cross-section against a 419.43 mm plate bound (ADR-0059 Ruling 3 / D-287). base.stl supports-on runs clean (`degraded=false`, `non_fatal=0`) in both generators and both PNP modes, band layers 108–258 restored (151→0 zero-Support layers), so the four tainted cells are untainted for future scoreboard revisions; evidence in `evidence/dev174-repair/`.
- [Gap budget per cell](issues/12-gap-budget-per-cell.md): the serial terms bind every cell — with per-layer work free the prepass floor alone still loses benchy ~2–2.5x and base supports-on ~15–20x, a measured slice-outside wall costs 2.3x (5.5x post-repair) of Orca's budget on base, and the CPU work-density deficit is 6.98x→22.9x per output byte — so no old candidate closes a 6.28–26.24x gap. Route re-ranked: 24 → 22 → new [Serial host floor](issues/27-serial-host-prepass-floor.md) and [Classic output-volume surplus](issues/28-classic-output-volume-surplus.md), tickets 19/20 demoted below-fold, acceleration confirmed at 1.00–1.19x wall.
- [host:slice closing_ex span contradiction](issues/24-host-slice-closing-span-contradiction.md): the contradiction is a units mismatch — `module_complete`'s ~3.2 s is real wall while the profile table's ~22 s `closing_ex` spans are accumulated per-thread spans (`fold_marks` is thread-correct; the aggregation sums concurrent worker spans) — so the lead retires with no ~22 s cost to promote, and the 22/27 substage splits inherit "work-share yes, wall claims no"; same-run capture in `evidence/t24-span-contradiction/SAME-RUN.md`.
- [Criterion bench refresh](issues/15-criterion-bench-refresh.md): all 7 benches now have on-disk baselines (82 leaves, `base == new`) and are trustworthy — `gate_evidence` 885.9 ms vs the 10 s bound, `shell_classification` ms-scale with no short-circuit; two fixture limits recorded not fixed (`repair/cube` scans a clean 12-tri mesh, `decimate/cube_default` is rejected by the default `max_error = 0.01`); and the criterion console's `time:` is the regression *slope* in Linear mode, not the mean (up to +8.61% off) — cost A/Bs must compare like-for-like. Evidence in `evidence/t15-criterion-refresh/`, unblocking [clipper2 cost/output verdict](issues/17-clipper2-cost-output-verdict.md).
- [clipper2 cost/output verdict](issues/17-clipper2-cost-output-verdict.md): **stay on 1.1.0** — the 1.0.3→1.1.0 bump is measured cost-neutral and output-identical (367 fixtures through the full entry-point surface, incl. 24 real benchy layers; `diff -rq` exit 0, tree SHA equal, both sides self-reproducing), with compiled bodies 230/240 bit-identical as the load-independent backstop; the geometry modules are byte-identical and `check_split_owner`'s reach is unchanged, so DEV-173's posture is unaffected; evidence in `evidence/t17-clipper2-ab/`. The harness trap (a scratch workspace's lock floats the caret req to the newest release — it silently measured 1.2.0 while labelled 1.1.0) is recorded for future dependency A/Bs, and it surfaced a real onward finding: **1.1.0→1.2.0 is not output-neutral** (2/367 fixtures lose one collinear vertex; 1.2.0's `clean_collinear` changed) — a future 1.2.0 bump needs its own geometry A/B.
- [Tree-planner substage attribution](issues/22-tree-planner-substage-attribution.md): ADR-0049's 98%-collision-cache lead **does not survive to the matched job** — the emit pass is 97.2% base / 92.6% benchy of the per-object planner, with per-region carve the largest sub-term (46.8% of base emit wall), followed by union+simplify (27.8%) and model-occupancy inflate (23.4%); the ladders are 4.27 s of 269.6 s (1.6%). Batched host offsets run (50 batches on base), but batched clips/simplify do not. The carve alone makes 147,993 clips on base; the whole stage makes 171,636 / 194.0 s of core boolean calls. A bbox gate could avoid 78,832 base / 11,018 benchy disjoint *carve* calls as set operations, but its net wall and output equivalence are unmeasured; the probe showed one disjoint rectangle's contour differs from a verbatim input. Evidence in `evidence/t22-planner-substage/`.
- [Emit-pass carve gate: representation safety + paired A/B](issues/32-emit-carve-gate-representation-and-ab.md): the naive bbox skip changes representation and a reachable lone-disc simplification result; guarded winding and lone-region repair preserve the tested production chain exactly. Independently measured ordinary + accelerated pairs favor the candidate on process CPU in every completed pair and on paired median wall in every cell, but wall is load-qualified (cpu/wall below quiet reference). **Human accepted KEEP; committed with this ticket.** Evidence in `evidence/t32-carve-gate/`.
- [Accelerated residual query attribution](issues/18-accelerated-residual-query-attribution.md): the hot queries' ~1.9× residual is **the exact winding predicate loop after candidate reduction** — `inside_scan`+`quartile_scan` are ~89% of base's accelerated query fuel (collect is 24–55× cheaper), same shape under Arachne (79–81% of module) — while the u128-envelope suspect is falsified (f64 envelopes; the u128 bridge precheck is ~0.03% with its unsafe fallback never firing) and Linear fallbacks fire often but cost 0.07–1.96%; per-query scan fuel grows 3.5× benchy→base while collect stays flat, locating the superlinearity in the scan. Next candidates (2-D per-polygon bbox records, pre-converted vertex cache + short-circuits, descending quartile first-hit) stay below-fold — fuel→wall transfer 0–16%, no gap-closer. Evidence in `evidence/t18-accelerated-residual/`.
- [Integrated-parity oracle experiment](issues/23-integrated-parity-oracle.md): native region eligibility and per-region config precedence were repaired and narrowly tested, but integrated timings remain disqualified as a module-work oracle pending the [Integrated/external matched-output oracle gate](issues/33-integrated-external-matched-output-oracle-gate.md) against real full-slice outputs.
- [Config appendix must reflect resolved settings](issues/26-config-appendix-resolved-settings.md): one-to-one Orca-named appendix aliases now disclose PNP's effective wall count, infill direction, and wall-path precision instead of static padding; sparse-density padding now matches its documented default. Remaining padding-only disclosure semantics are the separate [Config padding disclosure boundary and viewer-key gate](issues/34-config-padding-disclosure-boundary.md).
- [Serial host floor](issues/27-serial-host-prepass-floor.md) — **work item 1** (2026-09-27; items 2–5 still open): slice-outside wall R measured same-run is **0.15–0.23x** of Orca's budget on the worst cell (benchy ≈ 0.2 s, base 1.6–3.4 s) — ticket 12's 2.3x/5.5x R came from subtracting instrumented phase sums from uninstrumented process wall (§3.2), so every c-min in its budgets table is 10–35x too high and R retires as a budget term; the elapsed tail is attributed (pre-validation ~1.1–1.5 s incl. the ~0.77 s module-compile fixed term; prepass→per_layer handoff ~0.75 s; post-postpass = 32–38 s of pre-repair DEV-174 diagnostic replays — an artifact, not route work — plus `estimate_print`+scan at 6.8 s accelerated vs 16.9 s ordinary on identical 54.5 MB output, ordinary/accelerated delta unexplained, probe next). Evidence in `evidence/t27-serial-floor/`.
- [Serial host floor](issues/27-serial-host-prepass-floor.md) — **work items 2–5** (2026-09-27, `PERF-T27-PROBE`, 4 uninstrumented runs, guests fresh): all four prepass built-ins attributed at the matched job and **`PrePass::Slice` (47–56 s wall on base, #2 serial block) adopted into the ticket**; ShellClassification's serial gate-commit loop (25–30 s) is the largest confirmed serial block outside Slice; the overhang footprint exact union (17–18 s serial) has an exact-intersection support-eligibility consumer (bbox replacement unsafe; premise audit below); SupportAnalysis's serial sweep is 24.7 s and its detect pass is parallel-saturated (11.7x); OverhangAnnotation's interior is parallel (needs a wall bracket before candidates graduate). Probe code stays out of commits. Evidence in `evidence/t27-serial-floor/FINDINGS-SUBSTAGE.md`.
- [Serial host floor](issues/27-serial-host-prepass-floor.md) — **probe iteration 2 and premise audit** (2026-09-28): phase-B's two Miter offsets are measured work, but the proposed sibling-region memo has zero eligible siblings in the matched single-region jobs and its key does not prove identical input; drop that candidate, not the cost. `PrePass::Slice` fuel is ~70% `assemble_flat_bridge_areas` (parallel-saturated), but a safe reduction needs representation analysis. **The 6.8-vs-16.9 s tail mystery resolves to the pipeline-boundary teardown** (`estimate_print` itself is ~0.1 s; teardown 7.5–17 s vintage-dependent, below-fold); OverhangAnnotation wall-bracketed at 12.2–14.2 s (11–16x parallel, below-fold). Evidence in `evidence/t27-serial-floor/FINDINGS-SUBSTAGE-2.md` and `evidence/t27-serial-floor/PREMISE-AUDIT.md`.
- [Serial host floor](issues/27-serial-host-prepass-floor.md) — **human route decision** (2026-09-29): accepted DROP of the phase-B sibling-region memo for the matched jobs and parked further serial-floor candidate search until an output-safe, measurably useful reduction has a real-layer IR oracle. The failed flat-bridge mask elimination remains dropped; `apply_opening` parity is separate. Prioritize the already-open [Classic output-volume surplus](issues/28-classic-output-volume-surplus.md) attribution next. Serial host floor stays open and unclaimed; no candidate committed in this take.
- [Classic output-volume surplus](issues/28-classic-output-volume-surplus.md): fresh ordinary matched output census attributes 90.6% of Benchy's classic-vs-Arachne byte excess to actual sparse XY paths, not gap fill; base's classic excess is mostly wall bytes and sparse is smaller than Arachne. Historical Orca captures and a middle Benchy layer expose a fill-domain discrepancy, not a proven safe deletion. PNP also repeats Z/F modal tokens on every XY+E move; its speed impact is unmeasured. Follow-ups: [Benchy classic sparse-fill domain at middle layers](issues/35-benchy-classic-sparse-fill-domain.md) and [Modal G-code Z/F token redundancy and speed gate](issues/36-modal-gcode-token-redundancy.md). Evidence: `evidence/t28-output-volume/FINDINGS.md`.
- [Benchy classic sparse-fill domain at middle layers](issues/35-benchy-classic-sparse-fill-domain.md): **not a fill-domain question — a commit-protocol containment hole.** The linker clips the raw gyroid waves to nothing on the burst layers (probe: 1,741 mm in, 0 paths kept at classic L104), the host reads that empty output as "committed nothing", and the preserved prior `InfillIR` prints the raw, unclipped envelope — 54,324 of 79,940 printed sparse mm (68.0%) and 3,054,585 of 4,486,477 sparse bytes across 60 classic layers, with 88.7% of the L104 revived path outside the part cross-section and 1,741 mm printed against a 44.2 mm absolute bound from the 17.7 mm² sparse claim. Arachne 16 layers / 20.9%; base has zero all-empty layers and classic prints fewer sparse mm than Arachne there, which explains the ticket-28 fixture divergence. No fix made — the protocol change is graduated to [InfillPostProcess empty-output protocol](issues/37-infillpostprocess-empty-output-protocol.md). Evidence: `evidence/t35-fill-domain/FINDINGS.md`.
- [InfillPostProcess empty-output protocol](issues/37-infillpostprocess-empty-output-protocol.md): **fixed — a ran `Layer::InfillPostProcess` that re-emits nothing commits the empty replacement set, so the linker's clip verdict supersedes the raw envelope.** The fix is stage-local (only this stage's contract is replace-with-complete-re-emit; `Layer::Infill` keeps merge semantics and `Layer::SupportPostProcess` stays additive) and lands at the producer on both legs (`deconstruct_layer_ctx`, `commit_native_layer_response`) plus the test mirror, all sharing `empty_infill_replacement`. `RoleBoundaries::for_role`'s `Some(empty)`-vs-`None` containment distinction is untouched (`modules/` diff empty). Real-G-code verification: the 60 classic burst layers go to zero sparse output and every other layer is byte-identical (classic 106,442 → 34,307 segments, 79,939.9 → 25,616.2 mm; Arachne 47,659 → 37,606), worst overshoot past the layer's own wall bbox 21.78 mm → 0.00 mm, and base.stl is byte-identical as the negative control (225,004 segments both sides, 0 differing layers). Paired ordinary + accelerated A/B (6 repeats/arm/cell, quiet machine, no starvation exclusions) measures **no wall win with corroborating CPU** in any of four benchy supports-off batches: paired median wall −0.17/−0.13/−0.03/−0.01 s, and the one directional cell (classic-off ordinary, 6/6) has flat CPU (+0.04 s, 3/6) — the case is containment, not speed, so this is a correctness keep, not a route-performance step. Contract recorded in ADR-0028 §Amendment 2026-09-29, `docs/02_ir_schemas.md` IR 8, the `run_infill_postprocess` trait doc, and DEV-196. **KEEP recommended to the human; not auto-committed.** Evidence: `evidence/t37-empty-commit-protocol/`.
- [Modal G-code Z/F token redundancy and speed gate](issues/36-modal-gcode-token-redundancy.md): **DROP approved by the human (2026-10-03)** — an audit session re-derived every figure from the raw captures with its own census and per-move effective-Z/F walk (not the take's verifier), replayed both arms from `source-worktree.diff` byte-for-byte (baseline `e1088b3d…`, candidate `132f9501…` / `8099ce5c…`; `--reduce` regenerates `evidence.json` exactly), and found one `FINDINGS.md` gist sentence wrong on the CPU deltas (only classic-off accelerated has a positive paired median CPU delta; corrected in place). The candidate modal `Z`/`F` serializer (`ModalZfState` in `DefaultGCodeSerializer::serialize_gcode`; non-comment `Raw` and `ToolChange` invalidation; bare no-op lines for neutral moves) is proven **semantics-preserving by an independent interpreter** (`verify-t36.py`, normalized move-stream SHA identical per cell/mode: classic-off `f49b5ec5…`, arachne-off `c237fb01…`; E tokens and line counts identical; `--falsify` detects sabotage) and removes **27.10% / 26.98% of bytes** (4,332,881→3,158,453; 4,126,888→3,013,383; Z tokens 105,070→520 / 99,716→521, F 105,549→1,039 / 100,195→955), but it **fails the pre-declared CPU-corroboration gate in 3 of 4 batches** (wall −0.3…−1.7%, inside the job's own 0.38–1.41 s spread; CPU flat-to-higher; only arachne-off ordinary passes both clauses) and a ~1% effect cannot matter against the destination's 6.28–26.24x gap. **No commit, no production change; the candidate stays uncommitted** — if a byte/bandwidth-bound consumer matters later the implementation and oracle are ready (a separate decision). The take's candidate-built binaries were purged from `target/dist-accelerated/developer/` and `target/debug/` (rebuilt from HEAD; both emitted the candidate byte stream until then) and the ticket discloses the t47-caused `empty_end_gcode_emits_no_block` red test as a non-caused pre-existing failure. Evidence: `evidence/t36-modal-zf/`.

- [Scanbeam TopX projection-cache experiment scope](issues/56-scanbeam-topx-cache-experiment-scope.md): human confirmed one conservative exact-version shared host/guest trial, with independent engine tests, full-matrix output qualification and paired-mode timing bounded to the measured classic supports-on jobs; temporary integration only, execution handed off separately without implementation or speed approval.

- [Scanbeam TopX projection-cache experiment](issues/57-scanbeam-topx-cache-experiment.md): **DROP accepted by the human (2026-10-07)** — the isolated shared-engine projection cache passed independent native/actual-WASM exactness and the full fresh 16-cell output matrix byte-for-byte in both modes, but the frozen paired campaign stopped by rule at its first cell with the candidate slower on both wall (+2.14%) and process CPU (+2.78%) in 6 of 6 retained pairs; candidate preserved, production restored, both freshness gates exit 0, no commit or adoption.

- [mesh_ops fixture limits](issues/29-mesh-ops-fixture-limits.md): **resolved (2026-10-07)** — the two fixture-limited criterion leaves now measure successful operations: `repair/wedge` on tracked `resources/regression_wedge.stl` (real damage: 12 reoriented, 56 open edges closed) and `decimate/calicat_default` on tracked `resources/calicat.stl` (default error budget reaches 876→438); `cube.step` untouched, in-bench guards plus a companion test with a `cube.step` negative control make the no-op shape falsifiable, fresh `base == new` baselines in `evidence/t29-mesh-ops-fixtures/`, pre-ticket leaves (2.947 µs / 3.745 µs) superseded and not comparable. Bench-trustworthiness only; moves no cell.

- [clipper2 1.2.0 geometry delta](issues/31-clipper2-1-2-0-geometry-delta.md): **stay on 1.1.0; the bump is parked, no manifest change (2026-10-07)** — ticket 17's "2 of 367 fixtures differ" is reproduced exactly (both a single deleted, (near-)collinear spur vertex: `benchy__layer01_opening` cross 0, `benchy__layer07_closing` cross −3) and attributed to `clean_collinear`, but the delta is a **canonical-convergence fix, not a divergence**: the repo's bundled canonical Clipper2 1.5.2 matches **1.2.0**'s `startOp = op2; continue` restart, 1.1.0's is the non-canonical one, and upstream's 1.2.0 note names the fix (zero-width spurs in union output; `preserve_collinear` defaults on and this repo never clears it on `Clipper64`). DEV-173's posture does **not** improve: `check_split_owner`, `recursive_check_owners`, `is_valid_owner`, `fix_self_intersects` and the poly-creation helpers are byte-identical across the bump. The rest of the feature-off delta is plumbing plus additive API; `using_z` is off by default so it is no cost argument, and `PolyFace64`/`poly_tree_to_faces64` is a **1.1.0** addition (corrected), unused here, and does not expose the island-in-hole nesting `expolygons_from_tree` needs. Matched-cell disclosure not run (nothing moved). No bump, pin, vendoring, timing run or commit; `slicer-core` compiles unmodified against 1.2.0. Evidence: `evidence/t31-clipper2-120/` (`verify.py` exits 0 against the in-repo set).

- [profile wall-column semantics](issues/30-profile-wall-column-semantics.md): **resolved (2026-10-07, docs/comments only)** — the `--profile` wall columns are now documented as accumulated thread-time, never wall: each value sums one span per completed activation and `flush_native` adds concurrent worker activations, so a run-wide total can exceed the run's own wall-clock by the mean concurrency and is never comparable to `module_complete`/phase walls or process CPU. Semantics recorded in the `profiling_report.rs` module doc (with ticket 24's `closing_ex` ≈34 s vs 3,889 ms exemplar) and type docs (`ScopeTotals`, `ProfileScopeRow`, `ProfileModuleRow`, `ProfileSummary::wall_total_ns`, `ProfileUnit::WallNs`, `flush_native`), `slicer-wasm-host`'s `ScopeTotals` docs, the rendered wall heading and closing note, ADR-0055, `docs/09_progress_events.md`, and `docs/17_agent_debugging.md`; the rendered note is pinned by `summary_always_states_the_observer_effect_contract`. No behavior change, timing run, candidate or commit.

- [Config padding disclosure boundary and viewer-key gate](issues/34-config-padding-disclosure-boundary.md): **resolved (2026-10-07, disclosure/correctness only)** — premise corrected: `ORCA_CONFIG_PADDING` never fires on any production path (the registry projection reaches 233 pairs; the loop breaks at 96), so its values are unreachable there and only the test/legacy callers consume the table. The audit nonetheless found two **live** wire-format defects in the emitted block: booleans rendered as `true`/`false` where OrcaSlicer's wire literals are `1`/`0` (23 production lines; `ConfigBase::set_deserialize_raw`'s substitution branch — `enum_looks_like_true_value` is true only for `enabled`/`on` — loaded every rendered `true` as `false`, e.g. `detect_overhang_wall` and `support_remove_small_overhang`), and four enum values carrying PnP-internal spellings (`wall_sequence` `InnerOuter`, `wall_generator` `Classic`, `wall_direction` `counter_clockwise`, and the `seam_mode` alias rendering `rear`) that Orca's strict `enum_keys_map` lookup answers by replacing the option with its own default; two keys PnP models as `bool` but Orca declares `coEnum` (`dont_filter_internal_bridges`, `enable_extra_bridge_layer`) needed the same treatment. Fixed in one documented `translate_appendix_enum` table applied only at the appendix (module `ConfigView`s unchanged). Table audited row by row: 10 values were corrected (7 to the pinned upstream default, 3 more in enum domain), 3 rows named keys OrcaSlicer cannot load (`support_material`/`extra_perimeters` removed as non-Orca keys; `wipe_tower_width` kept — `handle_legacy` renames it), and 4 rows named keys absent from OrcaSlicer entirely, replaced with real cosmetic keys. A freshly generated production block now measures **0 strict wire violations**; the edited table compares **0 default mismatches** against the pinned `PrintConfig.cpp`. Residual: the padding-only fallback block still cannot satisfy Orca's `>= 80` loadable-pair gate because the remaining unloadable lines are PnP's own key names (registry-projected, not table rows) — recorded, not patched. No timing, geometry, adoption, default or guest change; all required gates pass.

- [Repair the Orca rig](issues/98-orca-rig-repair.md): **rig measures Orca again on the 2.5.0 install, and the repair falsified the archived Orca side.** OrcaSlicer 2.5 resolves `--load-settings` through a vendor preset bundle (`PresetBundle::resolve_preset_config`: file-identity match, else a `"from": "system"` manifest walk that flattens `inherits`/`include`), and the install ships only `BBL.opc` with no preset JSON tree. The rig now stages a version-gated source-manifest tree from the tag whose `BBL.json` version equals the installed cache stamp (`02.08.00.16` == `2.8.0.16`, fail-closed) and applies the matched keys as CLI overrides, with `enable_arc_fitting` pinned to 0 because 2.5 flipped that default to enabled. Three measured runs of benchy classic-off: **1.978 s** wall / 6.50 s CPU / 4,000,709 bytes, against the archive's 2.787 s / 9.27 s / 3,890,392 — the archived Orca applied its hardcoded **filament** defaults (density 0, max volumetric speed 2, first-layer temp 200) while the process chain resolved in both eras (300 mm/s walls). So the archived Orca under-measured Orca's own work: PnP's recorded multiples are optimistic for PnP and every archived Orca median needs re-derivation (fog). No ratio requoted. Evidence: `evidence/orca-rig-repair/` (`verify.py --self-test` exit 0, 4 claims, 7/7 negative controls rejected).

- [`ShellClassification` post-t172 residual attribution and speedup screen](issues/180-shell-classification-post-t172-residual-screen.md): **resolved (2026-11-22, attribution + candidate screen only; zero production code edits; `probe.patch` cleanly reverted, both guest freshness checks exit `0`)** — attributed `PrePass::ShellClassification` (`15/15` byte-identical to `t175/reference.json`, `100 %` call reconciliation vs `t177`, `4/4` classic/arachne & supports-off/on invariance) to serial Phase-B internal-bridge commit + Pass-2 merges/diffs (`30.2 %` `benchy` / `24.7 %` `base`), the `L1` `#3DBenchy` letter-hole straggler in `unsupported_span_areas` (`128.9 ms`), wasted `None` Phase-B precomputes (`72.1 %` `benchy` / `93.2 %` `base`, burning `4,016 ms` in `ib_comp_pos_offset`), and unmarked quadratic CPU loops (`determine_bridging_angle`, `detect_bridging_direction_deg`). Proved via 12-thread exact IR replay (`240` `benchy` + `496` `base` layers) that the 100 % `EXACT_BIT_IDENTICAL` four-mechanism bundle (`E1 + E2 + E3 + E4a`) cuts stage wall **`−56.06 %` (`benchy`) / `−44.58 %` (`base`)**, clearing the literal 8-cell `0.80 %` floor at `6.94×–20.60×` with zero output drift and crossing `< 1.0x` on both `benchy` supports-off cells alone (`ESTIMATE ONLY`). Falsified `E4b`/`L1`/`L3` as exact; parked sub-micron `L2` (`0.174 / 0.825 µm`, clears `0.80 %` marginally in only `2/8` cells) rather than contaminating the exact ticket; graduated [Exact `ShellClassification` acceleration: windowed `unsupported_span_areas`, lazy/clustered internal-bridge commit, parallel Pass-2 merge/diff, and $O((C+E)\log(C+E))$ external-bridge scoring](issues/184-shell-classification-exact-acceleration.md). Evidence: [`evidence/t180-shell-classification-residual/FINDINGS.md`](evidence/t180-shell-classification-residual/FINDINGS.md) (`verify.py` PASS `7/7`, `--self-test` `15/15`).

- [`SupportGeometry` post-planner host tail attribution and reduction screen](issues/181-support-geometry-post-planner-host-tail.md): **resolved (2026-11-22, attribution + candidate screen only; zero production code edits; `probe.patch` cleanly reverted, clean release `pnp_cli.exe` rebuilt, both guest freshness checks exit `0`)** — attributed the `100 %` single-threaded (`1.0×`) post-planner host tail (`212.91–214.61 ms` `benchy-on`, `1,939.31–1,946.49 ms` `base-on`; `22/22` runs byte-identical to `t175/reference.json`, `6/6` qual clean, `2/2` supports-off inert, classic ≡ arachne counters) to duplicate single-threaded `slice_mesh_ex` re-slicing (`39.1–40.2 %`, where `200/200` `benchy` and `432/432` `base` planes are `100 %` polygon-for-polygon identical to `SupportAnalysisIR::print_plane_occupancy` already on the blackboard), unfiltered serial `overlaps_any` Clipper2 intersections (`57.4–59.1 %`, `71.4 / 60.5 %` strictly bbox-disjoint and `28.6 / 39.6 %` concave-bay/hole pairs), and `3×` redundant `SupportPlanIR` deep clones (`1.2–1.7 %`). Proved via 12-thread exact IR replay + a `10,918`-pair zero-tolerance adversarial suite (`0` FN / `0` FP) that the 100 % exact `M12345` host bundle (`M1` PPO reuse with unseeded fallback + `M2` bbox cull + `M3` zero-clone + `M4` exact `i128` topological disjointness certificate + `M5` order-preserving Rayon validation) cuts the host tail **`−99.66 %` (`210.83 → 0.71 ms`, `benchy`) / `−99.85 %` (`1,971.68 → 2.95 ms`, `base`)** with `0` Clipper2 fallbacks, clearing the standing `t137` 4-cell planner gate at `7.57×–9.69×` floor (`7.59×–8.68×` probe scale) with zero output drift and zero tolerance weakening; graduated [Exact `SupportGeometry` post-planner host-tail acceleration: `print_plane_occupancy` reuse, zero-clone aggregation, and exact `i128` disjointness certificate](issues/185-support-geometry-post-planner-host-tail-acceleration.md). Evidence: [`evidence/t181-support-geometry-host-tail/FINDINGS.md`](evidence/t181-support-geometry-host-tail/FINDINGS.md) (`verify.py` PASS `7/7`, `--self-test` `14/14`).

## Not yet specified

- **The Clipper2 kernel route is now EXHAUSTED for the six non-winning cells;
  the post-t175 speedup discovery tracks are graduated
  (2026-11-20 → 2026-11-21, [t170](issues/170-clipper2-2-0-1-kernel-port-screen.md),
  [t171](issues/171-post-t169-prepass-reprice.md),
  [t172](issues/172-port-offset-concave-crossing.md),
  [t174](issues/174-post-t172-scoreboard-refresh.md),
  [t175](issues/175-tiled-collision-carve.md),
  [t176](issues/176-destination-cpu-corroboration-ruling.md)).** Both `t169`'s
  edge decimation and `t172`'s `OffsetConcaveCrossing` have landed, `t174`'s
  quiet-box board plus `t176`'s Destination ruling moved the running tally to
  **2/8 provisionally won** (`base-classic-off` **0.769x / 0.766x** ord/acc,
  `base-arachne-off` **0.792x / 0.774x**), and `t175`'s exact 2 mm phase-1
  collision tile index banked **+31.32 / +30.90 %** wall on `base` supports-on
  (est. **~1.221x / ~1.201x** ord) and **+16.06 / +16.62 %** on `benchy`
  supports-on (est. `benchy-arachne-on` **~1.389x**, binding `benchy-classic-on`
  **~1.570x**). [Post-t175 full-matrix attribution and 0.50x headroom map](issues/177-post-t175-full-matrix-attribution.md),
  [Planner Phase-3 batched union attribution and mechanism screen](issues/178-planner-phase3-union-attribution.md),
  [Per-layer `tree-support` renderer and `infill-linker` guest-fuel screen](issues/179-per-layer-tree-support-and-infill-linker-screen.md),
  [`ShellClassification` post-t172 residual attribution and speedup screen](issues/180-shell-classification-post-t172-residual-screen.md),
  and [`SupportGeometry` post-planner host tail attribution and reduction screen](issues/181-support-geometry-post-planner-host-tail.md)
  are ALL now **resolved** (2026-11-21 → 2026-11-22), and the first implementation
  track — [Host-side spatial hierarchical Phase-3 union with convex-containment pruning](issues/182-host-spatial-hierarchical-phase3-union.md)
  (`worker-k8`, host-only inside `clip_polygons_batch` `Union`, zero WIT/guest
  source edits, 4/4 supports-off byte-identical to `t175`) — is now **resolved &
  landed** (2026-11-22, KEEP by explicit human ruling advancing active currency
  to [`evidence/t182-spatial-hierarchical-phase3-union/reference.json`](evidence/t182-spatial-hierarchical-phase3-union/reference.json),
  banking **+13.85 / +14.00 %** wall on `base` supports-on to est. **`~1.052x /
  ~1.033x` ord** and **`~0.998x / ~0.997x` acc**, closing `76.6 / 83.6 %` of the
  remaining `base` gap, and **+1.90 / +2.41 %** ord / **+4.58 / +3.81 %** acc on
  `benchy` supports-on to est. `~1.540x / ~1.355x` ord), and the second
  implementation track — [Exact per-layer guest acceleration: `tree-support` renderer fast paths and `infill-linker` open-subject tagging](issues/183-per-layer-tree-support-and-infill-linker-exact-acceleration.md)
  (`TS-1a/1b/2 + IL-1/2/3`, zero WIT changes, zero geometry exceptions, zero
  gate waivers) — is now **resolved & landed** (2026-11-23, literal standing
  all-8-cell `0.80 %` paired gate **8/8 PASS**: `+2.20…+9.59 %` ord wall,
  `39/40` wins, `40/40` CPU wins `+3.10…+16.54 %`, acc `+1.85…+7.97 %` wall /
  `+3.53…+15.92 %` CPU, `224/224` measured runs 100 % byte-identical to
  `t182/reference.json`, crossing `< 1.0x` on both ordinary `base` supports-on
  cells to est. **`0.973x / 0.966x` ord** and `0.926x / 0.917x` acc — all 4
  `base` cells now `< 1.0x` — plus `benchy-arachne-off` acc at **`0.989x`** and
  `benchy-classic-off` at `1.034x` ord). The third implementation track — [Exact `ShellClassification` acceleration: windowed `unsupported_span_areas`, lazy/clustered internal-bridge commit, parallel Pass-2 merge/diff, and $O((C+E)\log(C+E))$ external-bridge scoring](issues/184-shell-classification-exact-acceleration.md) — is now **resolved & landed** (2026-11-23, pooled literal 8-cell gate 8/8 PASS, `448/448` runs byte-identical to `t182/reference.json`, benchy supports-off est. `~0.877x / ~0.885x` ord). That leaves **one unblocked AFK exact implementation & standing-gate frontier ticket** (measuring 100 % output-identical against `t182/reference.json`):
  [Exact `SupportGeometry` post-planner host-tail acceleration: `print_plane_occupancy` reuse, zero-clone aggregation, and exact `i128` disjointness certificate](issues/185-support-geometry-post-planner-host-tail-acceleration.md)
  (100 % output-preserving, zero-tolerance-preserving `M12345` host-only
  acceleration; zero WIT/guest changes, zero geometry exceptions, standing `t137`
  4-cell planner gate with `16/16` byte-identical output to active currency; replay cuts
  host tail `−99.66 %` `benchy` / `−99.85 %` `base`, projecting `6.06–7.75 %`
  supports-on cell cuts at `7.57×–9.69×` floor; stacked on landed `t182`, closes both
  ordinary `base` supports-on cells `< 1.0x` at `~0.970x / ~0.961x`, and the
  4-track stack `t182 + t183 + t184 + t185` puts **6/8 cells `< 1.0x`**).
  **Remaining fog:** (a) reserve stages not yet ticketed if `182`–`185` leave a
  residual on binding `benchy-classic-on` (`~1.229x` est.) or `benchy-arachne-on`
  (`~1.065x` est.), or for the `<= 0.50x` map —
  specifically `PrePass::SupportAnalysis` (`9.5–10.0 %` `base-on`, `7.8–7.9 %`
  `benchy-on`; `84 % offset` in `host:support_analysis`, `3.2–3.4×` concurrency
  on `benchy`, plus `t181` observed single-threaded `slice_mesh_ex` inside
  `collect_print_plane_occupancy` taking `~85 ms` `benchy` / `~745 ms` `base`),
  `PostPass::LayerFinalization` (`4.8–5.1 %` `benchy-on`;
  whole postpass `11.8 %`), `PrePass::Slice` (`9.1–14.0 %` supports-off,
  `4.5–5.2×` concurrency), the surviving `~159 ms` 3× host `region_partition`
  inside `benchy` `L2` (standalone `with_max_len(1)` Rayon granularity has only
  `31 ms` = `1.9 %` on `benchy-classic-off` and `<= 0.3 %` on 7/8 cells
  post-`t183`), and the surviving post-`t184` `ShellClassification` sub-micron
  canonical-offset residual — canonical `ClipperUtils.cpp::offset_expolygon_inner`-style
  per-`ExPolygon` negative offset in Pass-2 `project_shadow` (`L2`: `0.174 /
  0.825 µm` scanbeam drift; `−12.9 ms` `benchy` / `−177.1 ms` `base` marginal
  over `EXACT_BUNDLE`, clearing `0.80 %` in only `2/8` cells standalone —
  `1.98 / 1.83 %` `base-off`, `0.67–0.69 %` `base-on`, `0.37–0.76 %` `benchy`)
  plus fixed Pass-1 per-`ExPolygon` `opening_ex` with exact `2× miter_limit * r`
  CC dilation (`B1_pass1` `64.8 ms` `benchy` / `676.2 ms` `base` residual),
  parked for the `<= 0.50x` follow-on rather than contaminating `t184`'s 100 %
  exact gate; (b) timing of the next official three-tool Orca scoreboard refresh
  once candidates from `182`–`185` land and push additional cells across `< 1.0x`;
  and (c) the planned follow-on **2× faster (`<= 0.50x`)** fresh map once this
  map closes at 8/8 `< 1.0x` (now proven by `t177` to require both prepass and
  per-layer cuts on all four `benchy` cells).

- **t129's operand-volume shapes are ANSWERED and both were already landed
  ([t171](issues/171-post-t169-prepass-reprice.md), 2026-11-20).** The map had
  priced **S6** (exact occupancy memo, 14,227.7 ms thread / 6,504 ms wall on
  `base classic-on`) and **S5** (bounded-delta windowed internal-bridge offsets,
  21.0 s thread / 2.5 s wall on `base classic-off`) against the pre-t169
  primitive, and neither number may be quoted again — but they were not
  candidates: `inflate_model_occupancy_memoized` and
  `internal_bridge_offset_window` are production since t132/t133. Rescaled by
  t169's measured `offset` ratio, **S5's landed win is 9.01 %** of base
  classic-off's wall with a **0.83 %** residual (at the 0.80 % floor; 91.6 % of
  the whole-layer cost is already harvested) and **S6's is 5.86 %** of base
  classic-on's with a **5.89 %** residual (**11 %** of that cell's cut). S6 is
  inert on the four supports-off cells by construction. **No operand-volume
  shape survives as a route term**; the re-price did not re-measure those two,
  and it says so.

- **The two base Arachne cells' t166 effect is unresolved by the wall
  instrument (2026-11-19, [t166](issues/166-shell-opening-canonical-parity.md)).**
  The opening saves **+0.310 %** and **+0.734 %** there — below the 0.80 % floor —
  while process CPU falls **+7.081 %** and **+2.340 %** against a 0.74–1.54 %
  CPU null band, and the same change reads **+20.4 % / +21.4 %** on the two
  benchy supports-off cells. The wall null median on `base-classic-off` alone was
  **3.17 %** in this window, so a sub-1 % effect cannot be read there without a
  longer window or the accelerated arm. Two unticketed shapes: (a) an
  accelerated-arm corroboration (needs two `dist --accelerated` builds, the
  baseline included); (b) a quiet-window re-take of just those two cells with
  more pairs. Neither is a route lever until a human picks one.

- **The accelerated arm's headroom is bounded by the perimeters module's wall
  share, and it already harvests two thirds of that module's own work (profiled
  2026-11-18, [t163](issues/163-accelerated-timing-board.md) §7).** On base
  classic-off the accelerated build cuts `classic-perimeters` self fuel
  **109.995 B → 37.191 B (−66.2 %)** and total wasm fuel **190.753 B → 117.949 B
  (−38.2 %)** with byte-identical output, yet the module accumulates only
  **12.90 s** of thread-time against a 21.2 s run — which is why the board reads
  −1.1 % median and **+4.4 %** on that cell. Two shapes remain, neither ticketed:
  (a) the indexed path costs **43 % more wall per unit of fuel** (0.0896 →
  0.1285 s/B), the signature of memory-latency-bound index traversal — a tuning
  shape bounded by the module's share (≈3 % on base); (b) **widen the cfg's
  scope** to the host Clipper work that owns the base cells' wall
  (`host:shell_classification` = 104.2 s accumulated thread-time, 73 % of native
  work) — a design problem, not a tuning one. **No ticket until a human picks
  one**; the arm is not the route either way.

- **The phase split is re-attributed (ANSWERED 2026-11-18 by
  [t164](issues/164-phase-re-attribution.md): prepass **67.0–87.2 %** of wall,
  per-layer **9.3–26.3 %**, postpass **3.2–6.8 %**, outside **0.1–2.3 %**, from
  three instrumented passes per cell on the same tree and in the same window as
  the paired board).** The old split — prepass 63.5–82.4 %, per-layer 14.4–29.6 %,
  postpass 2.6–6.5 %, outside 0.1–5.3 % — was measured on the **t154** tree,
  before [t158](issues/158-needs-support-fast-predicate.md) landed a measured
  4.23–6.37 s on the four base cells; the prepass share rose on **all eight**
  cells (+14.85 pp on base classic-off, +11.51 pp on base arachne-off, +4.81 /
  +5.23 pp on the base supports-on pair, +1.07…+6.00 pp on benchy), exactly where
  the landed per-layer work moved. Every projection built on the old shares is
  stale, and route arithmetic may now be quoted from this split **inside this
  window only**. The re-derivation also moves the route's binding cell: all
  eight cells win at canonical prepass cost (0.396x–0.890x), `base classic-on` is
  no longer the loser (0.786x), and the two benchy supports-on cells carry the
  constraint at 0.890x / 0.874x with **1.53x / 1.59x** slack on t110's canonical
  prepass input — that input is **verified on 2.5.1 by
  [165](issues/165-canonical-prepass-cost.md)** (2026-11-19: 0.898x / 0.882x,
  slack 1.47x / 1.53x; 8/8 unchanged).

- **A single-arm cross-window wall comparison is unsafe, and the instrument is
  not version-stable (method note, 2026-11-17,
  [t161](issues/161-post-t158-scoreboard-refresh.md)).** Orca measured
  **+5.4…+24.9 %** slower than t145's board in all eight cells while its **CPU**
  moved only **−2.2…+11.7 %**, and the installed binary had changed under the
  map (2.5.0 → 2.5.1 at 2026-11-13 23:59, see the *OrcaSlicer tool boundary*
  note). So the movement is efficiency, not work, and it is **not** established
  as box state: the tool changed, 2.5.0 is gone, and the two cannot be separated.
  The paired ratio is what absorbs a common shift — the two benchy supports-on
  cells moved **+24.3 % / +24.9 %** on both arms while their ratio moved
  **−0.5 % / +0.4 %**. Therefore: quote ratios from one interleaved window or
  not at all; a delta measured against a prior window's *wall* is not a
  measurement of the code; "required cut" inherits the same caveat; and any
  campaign must stamp the Orca **executable** identity, because the rig's
  profile-stamp gate cannot see a binary swap. The map's history of quoting
  `t121 → t124 → t139 → t145 → t148` numbers side by side should be read under
  this note.

- **The benchy per-layer program is margin, not route (t154, 2026-11-14).** The
  canonical-prepass substitution now wins **7 of 8** cells and the benchy cells
  clear the line by 9.9–33.2 %; the only loser is **base classic-on at 1.051x**,
  which needs **1.096 s** (9.6 % of its per-layer phase). The residual per-layer
  terms there are `tree-support` **36.7 %** of per-layer thread (a
  work-set/phase difference — canonical generates support in one whole-object
  pass, not per layer), the perimeters module **33.4 %** (kernel-bound: 77 %+
  in-module self fuel) and the first-`Layer::Infill` block **23.5 %** (identity
  **answered by [t155](issues/155-infill-stage-block-base-on-residual.md)**:
  the prepared-region derivation paid twice per layer, 94 % of it one
  `intersection_ex`; the memo-preserving candidate
  ([t156](issues/156-prepared-memo-across-fill-sync.md)) was exact and CPU-lower
  but **dropped** on supports-on Arachne wall). **Open fog:**
  (a) the per-layer `tree-support` render fold — **ANSWERED by
  [160](issues/160-tree-support-prepass-batch-probe.md) (2026-11-17):** the
  read set is identical at both dispatch points on every layer of all eight
  cells (so it is a pure placement move), but the batch arm is **slower in
  every supports-on cell** (−1.69 % to −3.77 %) against a 9.50 % any-pair null
  band, and the null-renderer ceiling is **9.16–12.61 %** of cell wall. The
  "twice" reading was already corrected (canonical `generate_toolpaths` is also
  per-layer parallel). What remains is **not placement**: the renderer's cost
  is the work itself, so a per-op/algorithm reduction or a deletion is the only
  shape left on this carrier — no ticket is implied until a human picks one;
  (b) the arithmetic's ledger
  chain (t145/t148 walls × t150 deltas × this take's shares) — **ANSWERED by
  [161](issues/161-post-t158-scoreboard-refresh.md) (2026-11-17):** the fresh
  paired window gives **1.366x–2.718x** and **0 of 8 won**, required cut
  **26.80 %–63.21 %**, and it proves the sensitivity was larger than ±5 %: the
  Orca arm alone moved **+5.4…+24.9 %** in all eight cells against t145's
  medians, so t145's `46.03 %–71.06 %` and today's `26.80 %–63.21 %` are each
  correct only for their own window. t145's and t148's ratios are **superseded**
  as the ratios of record and no required cut may be carried across windows;
  (c) **no cause is attributed** for
  the t126→t154 per-layer fall (−21.6 % to −40.1 % on the benchy cells); t139's
  mid-layer sampling and t142's quartile derivation are candidates, not findings;
  (d) **the surviving predicate (attributed and priced by
  [t157](issues/157-needs-support-predicate-attribution.md); candidate
  [t158](issues/158-needs-support-fast-predicate.md) resolved 2026-11-16; human-amended 2026-11-17, KEEP and landed)** — `t157` proved that
  `commit_support_analysis_builtin` is the sole production decision consumer of
  `derive_needs_support` (after Packet 224 retired `region.needs_support()` from
  `TreeSupport::run_support`), and that an exact boolean fast path inside
  `SliceRegionView::derive_needs_support` (strict bbox overlap prefilter +
  strict-interior point-in-`ExPolygon` witness in `i128`, with `intersection_ex`
  fallback on overlapping pairs) yields **0 / 3,680** mismatches across `benchy`
  and `base`, speeds up all three call sites (`PrePass::SupportAnalysis`,
  `Layer::Perimeters`, `Layer::Infill`) by **91.0x–120.0x**, and prices at
  **230.6–237.3 ms** on `benchy` and **4.693–7.414 s** on `base` (taking `base
  classic-on`'s canonical-prepass projection from `1.0508x` to **`0.8033x`**).
  **[t158](issues/158-needs-support-fast-predicate.md) ran the standing eight-cell
  paired campaign (5 interleaved pairs/cell = 80 runs):** byte-identical 8/8 to
  `t151`'s active set, process CPU lower in **8/8** by **5.88 %–21.38 %**
  (`5.3x–19.1x` the `1.12 %` any-pair CPU null band; `39/40` pairs), and all four
  `base` cells clear the `0.80 %` wall floor in **20/20** pairs (`+5.42 %` to
  `+14.87 %` wall, `6.78x` to `18.58x` floor, `−4.23` to `−6.37 s`, realising
  `90.5 %` of `t157`'s `4.69 s` wall ceiling on `base-classic-off`), but two
  `benchy` cells miss the literal separate-arm `0.80 %` wall floor (`benchy-classic-on`
  **+0.766 %**, `0.96x` floor despite **+2.69 %** pairwise median and `4/5` wins;
  `benchy-arachne-off` **−0.076 %**, `−0.10x` floor despite **+3.16 %** pairwise
  median and `3/5` wins) because the entire three-site `benchy` ceiling (`~231–237
  ms`, `~3.0 %` of `benchy` supports-on wall) sits inside single-run wall jitter.
  The human subsequently ruled **KEEP** on 2026-11-17 and the candidate is landed;
  the original gate failure and missing `benchy` null control remain disclosed.
  `candidate.patch` and the ruling are preserved in
  [`evidence/t158-needs-support-fast-predicate/`](evidence/t158-needs-support-fast-predicate/FINDINGS.md).
  The measured route composition now puts `base classic-on`'s canonical-prepass
  projection at **0.8343x** and all eight projected cells below 1.0x; this is not
  a fresh Orca scoreboard or a measured destination close.

- **The adhesion fairness program is complete (t151, 2026-11-14): the supports-off oracle is
  4/4 and the last residual is the ~0.10 mm brim-extent term plus ≤ 0.04 mm of Arachne
  width, both inside their bands.** The 0.1786 mm skirt-hull input term is gone, so no
  further skirt/brim geometry ticket is implied; the four supports-on cells remain
  support-engine-limited (t138/t139) and out of scope for adhesion parity. **No ticket
  graduated from this take**, and with 1–151 resolved the frontier is empty: the next
  shape is a human pick from this section.

- **Supports-on completeness is closed (t148, 2026-11-13): the four supports-on
  cells now print the support they plan, so their ratios are quotable and the
  required cut is 63.0 % / 64.6 % / 70.2 % / 69.2 % (benchy classic-on /
  benchy arachne-on / base classic-on / base arachne-on).** The 33 dropped
  layers are back (32 base + 1 benchy), supports-off is byte-identical, and no
  route term changed: the four supports-off cells (1.853x / 2.043x / 1.984x /
  2.078x) and the engine-parity program carry the destination arithmetic.

- **The operand-volume program is the route (t129 → t131, 2026-11-09).** The
  work-set question and the per-op question are both answered: canonical does
  more per-layer Clipper work than PnP in the shell cluster, yet its whole
  prepass is 15.7x / 9.8x cheaper, the Rust Clipper2 kernel is at parity (t70),
  and the wrapper is **0.19–0.56 %** of primitive thread-time. **The gap is the
  operand volume fed to the sweep** (shell **89,825** points/layer, planner
  **592,363**/layer), so the route is fewer/smaller operands, not plumbing. The
  clean PnP-local deletions are **55.8 % / 17.5 % / 32.1 % / 14.9 %** of the
  required cut (base-off / base-on / benchy-off / benchy-on), a first tranche,
  not the route. **Prepass at canonical cost wins base classic-off only
  (0.787x), ties base classic-on (1.012x) and loses both benchy cells (1.053x /
  1.075x); full geometry parity wins all four (0.335x / 0.661x / 0.550x /
  0.704x).** **S6 is banked (t132, 2026-11-09): the duplicate occupancy half is
  exact, serial and worth +1.18 / +0.54 / +14.31 / +15.87 s on the four
  supports-on cells (7.39x–16.54x the floor), 15.5 % of base classic-on's
  required prepass cut.** **S5 is banked exact (t133, 2026-11-09): windowing the
  negative erosion only is byte-identical and clears 8/8 at 1.66x–10.94x, worth
  +0.19 / +0.13 / +2.69 / +1.36 s on the classic cells (9.7 % of base-off's
  required cut, 1.5 % of base-on's).** **Planner parallelism is answered
  (t134, 2026-11-09): the stage is 2.64x with 14,249 ms of serial per-layer
  occupancy (0.99x) against a 4.81x clip core; the occupancy batch and the
  clip-to-12x diagnostic are two supports-on tranches (t135/t136), and even a
  zero-cost planner leaves 2.87x–3.25x, so they are not the route; the gate
  decision [t137](issues/137-planner-scheduling-gate.md) is **resolved
  2026-11-12** (standing four-cell form confirmed for representation-safe
  planner-only candidates).** **Both t135/t136 planner shapes are now
  answered:**
  [135](issues/135-cross-layer-occupancy-batching.md) Stage 2 landed
  2026-11-12 (the cross-layer occupancy batch, 3.52x fan-out, +10.80 % /
  +13.31 % on the base supports-on cells, byte-identical 8/8) and
  [136](issues/136-planner-clip-batch-fanout.md) resolved 2026-11-12 with the
  clip core split by phase: carve **11.56x**, union **3.66x** at 4.4 requests
  per call owning 64.2 % of the wall, so the remaining planner scheduling shape
  is union-phase request grouping — **resolved 2026-11-12 as a KEEP at
  buffering depth 12**: the union wall falls **10,093 → 4,849 ms** on base
  classic-on (clip core 14,159 → 8,935 ms) and the paired wall saves 9.56 s
  (12.80 %, 10.3 % of the cell's cut); depth 16 failed its own merge-4 bracket,
  and the measured optimum is 12, not the pre-declared 16. **t135
  Stage 1 landed 2026-11-10 (the canonical skirt/brim re-take): the
  matched-settings asymmetry is closed and the new reference set (subsequently
  updated by [t139](issues/139-layer-plane-parity.md)) is the
  currency, so no future ratio may be quoted against t123's set.**
  The **sweep-volume work-set family** is now fully screened. Its **per-vertex
  overhang distance** member is priced, ticketed, and **landed**
  ([t141](issues/141-overhang-quartile-band-deletion.md) /
  [t142](issues/142-overhang-band-partition-deletion.md): the 4-band partition
  is deleted and the quartile is distance-derived, saving 26.25 % on base
  classic-off with byte-identical G-code in 8/8). **Per-layer overhang
  polygons** is priced and **refuted as the lever**
  ([t149](issues/149-meshanalysis-work-set-readers.md)): the union that port
  would replace is **529.1 ms** of a **5,325.8 ms** stage, and the **4,563.2
  ms** is a bridge detector with no production reader, graduated for deletion
  as [t150](issues/150-meshanalysis-bridge-retirement.md). **Canonical loop
  projection** (t129's S3) is priced and **answered**
  ([t152](issues/152-seam-inset-offset-redundancy.md)): the prepass inset offset
  is **not** redundant — deleting it moves **456/496** base-off layers and
  fails two unit tests — and its measured term is **1,248 ms** (70.15 % of the
  stage, 5.9x the floor, ~10.5 % of base-off's current cut), not t129's
  1,587 ms / 92.4 %. Its canonical replacement is **DECLINED as structurally
  impossible** ([t153](issues/153-seam-loop-relocation-ruling.md), 2026-11-14):
  aligned seams are cross-layer (ADR-0046), a per-layer module cannot see two
  layers, and canonical candidate scoring itself reads the previous layer's
  contours, so the "per-layer relocation" would need a two-phase pipeline (all
  per-layer perimeters before seam planning) or a host-orchestrated seam pass —
  architecture work, not a t81 key exception, and already rejected in
  ADR-0046's alternatives. **Re-entry requires the destination to be redrawn to
  include that restructure.** The per-layer
  phase joins the first program on both benchy cells.

- **The bridge wave expansion is a per-layer partition change, and its benchy
  cost is the blocker (surfaced 2026-11-06 by
  [t122](issues/122-prep-slice-wave-expansion-facet-retirement.md)).** Canonical's
  `expand_expolygons` / `wave_seeds` / `propagate_waves_ex` / `merge_bridges`
  grow classified bridges into `stInternalSolid` / `stInternal` / `stTop` and
  trim those zones. PnP's sparse zone and `perimeter.infill_areas` do not exist
  until the `Layer::Perimeters` commit partition, so the port belongs there,
  not in `PrePass::Slice`. The tight wall-inset zone-proxy screen prices the
  expansion at **1,151.9 ms** thread-time on benchy (**102.6 %** of the
  classification saving) and **4,976.6 ms** on base (5.2 %); the combined shape
  is negative in all four benchy cells, while the classification-only
  retirement clears all eight. **The classification half landed as
  [ticket 123](issues/123-facet-bridge-path-retirement.md) on 2026-11-07**
  (8/8 at 3.4x–27.8x); only the expansion/trim remains in this patch.
  Re-enter only with a per-zone implementation at
  the Perimeters commit, a measured benchy expansion cost below roughly **1.1 s**
  thread-time, and an explicit `detect_bridge_directions` vs PnP-direction
  decision. Do not re-open the old prepass framing.

- **The per-layer/emit program is measured and cannot close the destination
  (t111 → t126, 2026-11-08).** [t126](issues/126-post-t123-phase-split-refresh.md)
  re-derived the split on all eight cells after the prepass rewrite landed: the
  per-layer phase is **10.6–33.3 %** and deleting it entirely leaves
  **1.67x–4.14x** against Orca, while the postpass is **1.8–4.9 %** with no
  bracket above **2.61x** floor. The per-layer phase has no globally idle wall
  (module-busy union = phase wall on the base cells; 2.0–2.3 % on benchy), so
  there is no cross-worker idle pool to reclaim; a per-worker host cost between
  dispatches is not resolved by that instrument. **[t127](issues/127-raft-fill-dispatch-outside-band.md)
  then measured the one graduated shape and it is a DROP (2026-11-08): the
  61–63 ms raft-default bracket is the `Layer::Infill` stage's shared
  prepared-region view materialisation, paid by whichever Infill module is
  dispatched first, so declining it transfers the cost to `gyroid-infill` and
  the per-layer phase wall does not move.** Any future per-layer shape priced
  from a module bracket must first show the cost is module-specific by
  declining the module and watching the bracket move. The remaining shapes are
  left here with their prices, now discounted by that lesson: tree-support's
  per-layer emit-check half (≤**4.66x / 4.72x** for the whole supports-on call;
  the half is smaller, and
  [125](issues/125-strong-planner-occupancy-offset-batching.md)'s DROP is a live
  warning that this family does not automatically transfer), `infill-linker`
  (**1.17x–8.28x**, no shape identified), and the perimeters
  (**3.82x–15.84x**, but t68 measured 97–99 % native clipper ops, so the ceiling
  is not a shape). The per-origin occupancy batch must not be re-run as-is, and
  no per-layer batch candidate inherits rescue status. **After the t128 ruling
  the per-layer program is not a route to 8/8 by itself, but it is no longer out
  of scope either:** once the prepass is gone, benchy classic-off needs only a
  small per-layer cut to cross 1.0x (**zero prepass lands at 1.02x**), so the
  per-layer residual is the last mile on that cell and route work under the
  engine rewrite. **[t129](issues/129-prepass-work-set-per-op-parity.md)
  sharpened this:** at canonical prepass cost, the per-layer phase is
  **1,450 ms of benchy-off's 1,716 ms Orca wall** and **1,644 ms of
  benchy-on's 2,462 ms**, so the per-layer phase is part of the first program on
  both benchy cells, not a last mile only on classic-off. "Zero prepass" is not
  "prepass at canonical cost" (canonical's prepass is 0.05 / 0.55 / 2.10 / 10.44 s).
- **t110's base-on multiples need a ledger correction (surfaced 2026-11-03 by the
  t111 re-derivation).** The t110 answer's "end-to-end today" **9.49x** divides
  PnP's **instrumented** base-on slice span (**228,997 ms**) by Orca's quiet
  process wall (**24,140.3 ms**); t104's own disclosure says that instrumented
  pass ran **+43.6 %** over its t103 median and that instrumented walls are never
  wall claims. The uninstrumented paired record is **160.857 s** (t106) ->
  **6.02x**, or 6.66x against today's Orca. The derived **16.3x** slicing,
  **19.6x** prepass and **13.7x** support-increment multiples inherit the same
  inflation, so they are **upper bounds**, not ratios of record. The qualitative
  finding (the engine gap exceeds the wall ratio) survives; the numbers need
  re-deriving from t104's implied walls (share x the wall of record) before any
  of them is quoted as a scoreboard anchor. This is a bounded measurement-only
  task, not a route; it graduates when the human picks it.

- **A first-touch vendor tree makes the warmup load-bearing (t98 method note).** The
  first Orca run against a freshly staged vendor tree costs ~10 s more wall than the
  steady state with process CPU flat (10.084 s / 11.684 s vs 1.97–2.04 s measured
  after), and a warm tree with no cache beside the manifest still runs at 2.0 s. So the
  premium is one-time, but a `-Warmups 0` batch against a fresh tree would be
  contaminated by roughly the whole steady-state cell time. Any campaign on this rig
  must keep `-Warmups 1` or greater.

- **The surviving whole-print free is allocation-volume-bound, not a
  drop-strategy problem (surfaced 2026-10-29 by
  [ticket 94](issues/94-remaining-whole-print-drop.md); graduated as
  [ticket 100](issues/100-allocation-volume-layer-collection.md) on
  2026-10-30 — the human selected this route from six measured fog routes).** The `host:gcode_emit`
  free is **3,242-3,558 ms** for 495 layers on the base supports-on cells, at
  ~5.3 GB peak working set (t79). The free does **not** parallelise (12 threads:
  classic −0.32 s, arachne +0.20 s), layer-by-layer consumption overlaps
  nothing (the walk is 176-253 ms serial), and the deferred-overlap ceiling is
  the **1.37-1.60 s** post-emission tail — at the 0.80 % floor before overlap
  loss. The lever was thought to be **fewer/larger allocations: an arena or
  bulk-free layer collection** (one allocation per layer or per print).
  **[Ticket 100](issues/100-allocation-volume-layer-collection.md) priced that
  family on 2026-10-30 and it is dead as scoped:** the census is
  **34,796,481 blocks / 1.358 GB / 3.53 s (101.3 ns per block)**, but **99.6 %**
  of the blocks are **8–15-byte strings** — the `support_entity_identities` side
  table cloned per support path — while geometry is **0.094 %** of the blocks /
  **7.8 %** of the bytes. A bulk free of the same bytes costs **44.5–63.8 ms**
  (ceiling **3.468 s = 2.10 % = 2.63x** the floor), yet flattening a live
  collection pays a **73.3–115.1 ms** copy **plus the same storm** (**+73 ms
  worse**), so only arena-at-creation is reachable and it would aim at 0.094 %
  of the blocks. **The surviving shape was the table itself, and
  [ticket 101](issues/101-support-identity-table-materialisation.md) priced it on
  2026-10-31: below the floor (combined ceiling 821.6–926.8 ms = 0.50–0.56 % =
  0.62–0.70x), so the family is closed too.** Its shape is worth keeping: 23,751
  rows over 431 bodies (55.1 paths per body), each row carrying the body's whole
  ~1,459-id list, `mixed_bodies = 0`, 418 distinct contents, 1.2486 GB of heap —
  **one body's list cloned once per emitted support path**. The arithmetic that
  matters now: under mimalloc a block costs **~12.6 ns to free** (t101) and the
  whole table **~0.8 s**, so **re-enter the arena / block-count question only if
  the block count stops being dominated by a structure with no production
  reader**, and re-derive any per-block price before quoting it — t100's
  101.3 ns was the system heap.

- **The shipped allocator is now mimalloc, and the whole-process swap is worth
  ~5–6 % of a supports-on cell (measured 2026-10-30/31 by
  [ticket 100](issues/100-allocation-volume-layer-collection.md) and
  [ticket 102](issues/102-global-allocator-substitution.md), **resolved KEEP**).**
  The free alone fell **3.527 → 1.302 s measured** (**1.35 %** of the cell;
  **1.66 %** after the probe correction) on the same 34,796,481-block collection,
  and the process-wide swap passed the standing four-cell gate at wall
  **−6.19 / −5.64 / −5.54 / −4.80 %** with process CPU **−1.74 / −2.18 / −1.54 /
  −1.88 %**, **24/24 pairs byte-identical** and zero exclusions. The process-wide
  effect is the larger number because **every** allocation in the pipeline gets
  cheaper, not just the whole-print free. It removes **no allocations** — the
  census is unchanged — so the per-block cost now sets what a block-count
  reduction is worth. Two prices, don't conflate them: the **free alone** is
  **~12.6 ns per block** measured directly (t101: 432.4–494.1 ms over
  34,796,481 blocks), while t102's **~37 ns per block** is the *process-wide*
  effect implied by its wall delta, not a free-side price. The cause-level route
  [101](issues/101-support-identity-table-materialisation.md) is **resolved
  below the floor** on that cheaper price. Any future allocator experiment
  (snmalloc, jemalloc) must re-run the same four-cell gate;
  `docs/16_slicer_report.md` §"Global allocator contract" now names mimalloc as
  the shipped choice and keeps the accounting wrapper outside it.

- **A free-strategy census must sample block contents, not just sizes
  (method note, 2026-10-30, [ticket 100](issues/100-allocation-volume-layer-collection.md)).**
  The whole-print free presented as a generic "allocation-volume" problem for
  two takes (t94, t79) until the allocator probe copied the first bytes of the
  dominant size class out of each block: **99.6 %** of the blocks were 8–15-byte
  `String`s, not geometry, which inverted the target. A histogram alone cannot
  distinguish a million tiny geometry buffers from a million duplicated
  attribution strings, and the two want opposite fixes. Any future
  "N blocks are too many" screen should sample contents and attribute the
  dominant class to a producer before pricing a representation change.

- **The whole-layer clip-area dilation is priced, and its last
  representation-safe shape is closed (t96 window rejected 2026-10-30;
  [ticket 99](issues/99-clip-split-union-dilation.md) split rejected
  2026-10-30).** `offset(deep_infill_area, +1.5 × spacing)` is
  **11.4–12.8 s accumulated** across t92/t93/t96 (**1.57–1.85 %** of the base
  classic-off cell). Its cost decomposes **~80 % per-path Miter offset,
  ~19 % batched internal union/cleanup, ~2 %** an explicit union of the
  already-rounded pieces (t99, 130 calls, two pricing orders). **Both named
  shapes are closed:** a candidate-box window removes **91 %** of the call but
  changes the bridge direction (t96 — `internal_bridge_angles`'
  `expansion_area ∩ clip_area` term feeds a *global* nearest-anchor choice, so
  no finite candidate-driven window is safe), and the split-and-union family
  tops out at **0.41–0.43 %** of the cell — **0.51–0.54x the 0.80 % floor**, and
  not even set-equal (2/130). **No in-scope shape remains on this call.** The
  only standing shape is to change the angle consumer
  (`determine_bridging_angle`) so the clip area need not be whole — OrcaSlicer
  geometry-parity work, which the map holds out of scope unless the human makes
  a t81-style exception. Re-enter only if that consumer changes.

- **A host-side work removal can be wall-neutral in accelerated cells
  (surfaced 2026-10-29 by
  [ticket 93](issues/93-whole-layer-deep-infill-offsets.md)).** The windowed
  erosion removed **6.5 / 3.2 s** of process CPU in the two accelerated base
  classic cells while their paired wall moved **+0.13 / +0.62 pp** — inside
  those cells' own 6–8 % repeat ranges, and with CPU falling in every cell. A
  strict four-cell wall clause can therefore fail on a real work removal.
  Price the gate per mode, and settle before the campaign whether a
  CPU-corroborated wall-neutral cell is acceptable — not after it.

- **An inside-the-window set check is not a consumer-equality proof when a
  consumer is a global nearest-neighbour choice (t96, 2026-10-30).** Ticket 96's
  production and windowed clip areas were **set-equal inside the window** on the
  layer whose bridge direction changed (`mismatch_polys = 0`), because the
  consumer (`determine_bridging_angle`) samples candidate edges but scans every
  anchor ring globally. A window proof must bound each consumer's **reach**;
  comparing the windowed set to the production set bounds the operand, not the
  consumer.

- **A batched whole-mesh walk scales with per-triangle cost, not with plane
  count (surfaced 2026-10-30 by
  [ticket 95](issues/95-exact-z-cross-section-batching.md)).** Folding 431
  single-plane `slice_mesh_ex` walks into one sorted-Z walk bought
  **6.4–6.9x**, not 431x: the batched walk pays two binary searches over the
  plane array plus a straddle test per triangle, and it is memory-latency-bound
  on the 1.23 M-vertex array (**0.99–1.05 s** for 2.46 M triangles). Any future
  "batch N planes/queries into one walk" estimate must price the per-triangle
  overhead at N rather than assume N-fold. Method note, not route work.

- **Ownership changes the cost of what it removes — do not price the survivor
  from the old bracket (surfaced 2026-10-27 by
  [ticket 90](issues/90-postpass-layer-collection-ownership.md)).** Ticket 90's
  pre-declared net ceiling (10.76 / 10.29 s) assumed the one surviving
  whole-print drop would still cost what the baseline's cloned drop cost. The
  measured wall deltas were **12.0 / 14.8 / 15.9 / 15.4 s** and the surviving
  drop was **1.56 / 2.68 s cheaper**, because no fresh whole-print copy exists
  to free. Any future clone/drop-removal estimate that carries the old drop cost
  forward inherits the same bias; measure the survivor in the candidate arm, or
  state the ceiling as a range. Method note, not route work (same class as the
  `map_batch` occupancy note below).

- **`map_batch` occupancy must be measured per request set, not borrowed
  (surfaced 2026-10-25 by [ticket 85](issues/85-seam-inset-offset-batching.md)).**
  Ticket 84 priced the seam batching at t78's measured **2.6x** effective
  speedup for the same `map_batch` fan-out; the candidate actually achieved
  **≈7x** (8,356 → 1,737 ms), so the pricing under-predicted the wall win by
  roughly a third of the recoverable time. Any future batch-adoption candidate
  that copies that 2.6x as a planning constant inherits the same bias; price
  with the request set's own measured occupancy, or state the ceiling as a
  range. Method note, not route work — it graduates only with its own reason.

- **Fairness machinery reads a stale upstream snapshot (surfaced 2026-10-23 by
  [ticket 82](issues/82-fairness-key-parity-audit.md)).** The generated
  *Deviations from OrcaSlicer* table in `docs/15_config_keys_reference.md` (27
  rows) compares PnP defaults against `docs/ORCA_CONFIG_REFERENCE.md`, dated
  **2026-05-19**. Ticket 82 measured **82 of the 618 keys the matched job
  resolves absent from that snapshot**, including both Arachne tolerance keys —
  which is why the deviation that makes PnP do 10x/5x more Arachne wall
  simplification never appeared in the table. Any future fairness audit
  inherits this blind spot: the matched job's own G-code appendix is the only
  authority for what the comparison engine resolves. Regenerating the snapshot
  (or making the generator read an appendix) is method work, not route work;
  it graduates only with its own reason.
- **`enable_arc_fitting` is unexpressible and inert on the matched job
  (settled 2026-10-23 by [ticket 82](issues/82-fairness-key-parity-audit.md)).**
  Orca resolves `enable_arc_fitting = 0` in all eight cells, so there is no work
  difference today. It matters only if arc fitting is ever ported, because it
  halves `resolution`'s tolerance
  ([ticket 81](issues/81-resolution-slicing-time-contour-simplification.md)'s
  disclosures). No action while it stays 0.

- **The supports-on branch-distance lead is dead, and the style mismatch is now
  a human ruling (resolved 2026-10-30 → 2026-11-02; see
  [ticket 97](issues/97-match-branch-distance-fairness.md) and
  [ticket 105](issues/105-supports-on-engine-style-fairness.md)).**
  `support_style = default` on a tree family resolves to `smsTreeOrganic`
  (`SupportParameters`), `TreeSupport::generate` forks it to
  `generate_tree_support_3D`, and the organic engine reads only
  `tree_support_branch_distance_organic` — resolved to **1 mm**, the same as
  PnP's normal-key default. The normal 5 mm key is inert on that path; t88's arm
  was a 5x PnP branch-spacing reduction, not a match. PnP substitutes **Strong**
  for every canonically-organic input (`DEV-156`; organic port queued in
  `docs/specs/support-generation-remediation-plan.md`). The human ruled **pin
  Orca to `tree_strong`**; the measured price, the protocol disclosure and the
  post-pin scoreboard now live in the fairness contract above, re-derived by
  [ticket 106](issues/106-post-pin-supports-on-scoreboard.md) to **6.02x / 6.11x
  / 4.77x / 4.53x**.

- **Fan-command reference boundary (graduated from ticket 47, 2026-10-03;
  boundary settled by ticket 46, 2026-10-05).**
  The finalization annotation repair intentionally changed BOTH legs' output:
  external now emits part-cooling's `M106`/`M107` fan commands. Every
  pre-repair external reference (t44's accepted `1b71f83d…`, the t30-era
  matched-pair captures) is outdated for fan-bearing jobs. Ticket 46's
  frozen-job drift question is **resolved**: current refs `5a61a106…` (t44
  job) / `e1088b3d…`+alias (classic-off); frozen `7049a06d…` and t44
  `1b71f83d…` retire as this job's timing baselines. Any future keep/drop gate
  on a job with fan commands must use post-repair references or strip
  `M106`/`M107` before comparing (the ticket-46 evidence set carries the
  reconstruction: current + `; support_overhang_angle = 30` == the t47 refs).
  For ticket 45 (next chain link), the fresh-baseline gate may proceed against
  the current references; it is not map-wide authorization.
- **Painted-wall performance work.** No representative painted workload exists,
  so the painted path of `build_wall_flags` (nearest-original reprojection
  reuse) is not even selectable as a candidate. Needs a fixture/scenario before
  it can graduate into a ticket.
- **Classic `is_bridge` empty-area parity guard (t20 observation, 2026-10-22).**
  Arachne guards its per-vertex `context.is_bridge` loop with
  `!bridge_areas.is_empty()`; classic calls it for every ring vertex even when
  the region has no bridge areas. The call is not in t18's measured hot set
  (the two exact scan families are), so this graduates only with its own
  measured reason; it was deliberately excluded from the t20 candidate.
- **New candidates from the structural splits.** [Gap budget per cell](issues/12-gap-budget-per-cell.md)'s
  route tickets (24/22/27/28 and the re-scoped 18) may surface further
  candidates inside the serial floor, the query path, or the marshalling
  residual; graduate each as it appears. [Arachne graph-construction
  attribution](issues/25-arachne-graph-attribution.md)'s split (2026-10-01)
  re-derived the module's shape: the two per-vertex queries are 86.4% of guest
   fuel and its candidate graduated the now-resolved
  [overhang-quartile predicate narrowing](issues/45-overhang-quartile-predicate-narrowing.md)
   (2026-10-05: human-approved DROP of the bbox alternative; blanket boundary
   removal is not output-preserving). That take did not measure fuel or turn
    the synthetic pre-pass bound into a slice-wall saving. Additional query
     candidate shapes beyond the graduated signed-distance investigation and
      its completed exact prepared winding-coordinate cache experiment
      (human-approved DROP, 2026-10-06) remain
     unspecified; distance-query alternatives need their own reason and scope,
     not implicit permission from that split.    [infill-linker
   attribution](issues/21-infill-linker-attribution.md)'s split graduated the
   now-resolved [clip-universe preparation hoist](issues/44-clip-universe-hoist.md)
    (2026-10-01). The post-hoist linker residual graduated to
   [Infill-linker post-hoist residual and exact clip-work reduction](issues/62-infill-linker-post-hoist-residual.md)
   and is **resolved (2026-10-13)**: the current split is measured —
   per-path `execute` 85.96% + `engine_add` 4.42% of linker fuel, connect
   4.73%, prepare 2.60%, metadata 1.25% — and its candidate graduated to
   [Infill-linker batch-execute experiment](issues/65-infill-linker-batch-execute-experiment.md).
   Post-batch linker alternatives (the connect/prepare/metadata residual and
   any subject-side costs the experiment exposes) await that experiment's
   outcome rather than inheriting any assumed split. [Tree-planner substage
  attribution](issues/22-tree-planner-substage-attribution.md) graduated the
   now-resolved [Emit-pass carve gate: representation safety + paired
   A/B](issues/32-emit-carve-gate-representation-and-ab.md). Its sibling
   union/simplify lead graduated to
   [Tree-planner union/simplify investigation scope](issues/52-tree-planner-union-simplify-investigation-scope.md)
   and its now-completed diagnostic after the human selected that route
   on 2026-10-06. The clarified large-body-union question graduated to
   [Large body-union investigation scope](issues/54-large-body-union-investigation-scope.md)
   and its completed exact-corpus/native-internals diagnostic. The now-concrete
    projection-cache question graduated to
    [Scanbeam TopX projection-cache experiment scope](issues/56-scanbeam-topx-cache-experiment-scope.md)
     and its [completed execution child](issues/57-scanbeam-topx-cache-experiment.md):
     candidate exactness and the full fresh 16-cell output matrix passed
     byte-for-byte in both modes, but paired timing measured the candidate
     **slower in 6 of 6 retained pairs** on the first cell (wall +2.14%, CPU
     +2.78%), so the campaign stopped by rule and the **DROP was accepted by
     the human on 2026-10-07**. The isolated candidate is preserved, not
     adopted. Permanent dependency adoption and alternative algorithmic shapes
     remain unspecified.
   [Serial host
  floor](issues/27-serial-host-prepass-floor.md)'s probe batches (items 1–5 +
  iteration 2) surfaced three graduate-when-probed questions resolved on
  2026-09-28 (phase-B = two costly offsets, sibling memo dropped for the matched job by human decision on 2026-09-29;
  estimate_print tail = pipeline teardown, below-fold; OverhangAnnotation
  wall = parallel) and one since graduated: the flat-bridge reduction inside
   `PrePass::Slice` graduated to
   [ticket 59](issues/59-prep-slice-flat-bridge-closing-reduction.md)
   (representation analysis first — the tested `refilled`-mask elimination
   failed real-layer IR equivalence and was dropped; fresh 2026-10-09
   anchors: `closing_ex` 384–389 s accumulated on base cells) and **is now
   resolved as a no-safe-reduction verdict (2026-10-14; see Decisions — the
   gate is artifact-bound and irreducible)**. The smaller
   `apply_opening` lead is a separate geometry-parity question, not a
   representation-preserving replacement. Below-fold polish (`offset2_ex` call
  reduction, `split_top_surfaces` secondary work, the
  `emit_walls` memory-shape idea, and the ticket-18 query-internals candidates
  — 2-D per-polygon bbox records, pre-converted winding vertex cache +
  short-circuits, descending quartile first-hit) stays parked — the park
  decision is in
  [Gap budget per cell](issues/12-gap-budget-per-cell.md), and ticket 18's
  measured fuel attribution keeps its candidates in the same below-fold class —
  until a cell lands
  within ~1.5x of Orca or the structural tickets come back short.
  **Consume-only-when-read was lifted by the human and resolved as KEEP
  `053904aa` (2026-10-22; see Decisions), so it is no longer part of the
  below-fold list.**
  **Re-derived 2026-10-18 on the
  [scoreboard refresh](issues/72-scoreboard-refresh.md): the best cell is
  3.30x (benchy arachne-off) — the park is still binding.**
  **Re-derived 2026-10-25 on the
  [post-resolution-port refresh](issues/87-scoreboard-refresh-post-resolution-port.md):
  the best cell is 2.71x (benchy classic-off accelerated) and no cell is within
  ~1.5x — still binding, with the release bound moved 3.30x → 2.71x.**
  **Re-derived 2026-11-21 on
  [t174](issues/174-post-t172-scoreboard-refresh.md): the ~1.5x release bound is
  now CROSSED in all four supports-off cells — `base classic-off` 0.769x /
  0.766x, `base arachne-off` 0.792x / 0.774x, `benchy classic-off` 1.144x /
  1.118x, and `benchy arachne-off` 1.155x / 1.064x (ordinary / accelerated) — so
  the below-fold park is lifted for the benchy supports-off tail (the four
  supports-on cells remain 1.665x–1.916x and still need structural cuts).**
  **Base refresh (2026-10-18):** the fresh base split graduated three
  tickets — [supports-on finalization view
  cost](issues/74-layerfinalization-supports-on-view-cost.md) (**resolved
  2026-10-19, KEEP `091e6bc9`**), [ShellClassification phase-B
  commit](issues/75-shellclassification-phase-b-commit.md) (**resolved
  2026-10-19, KEEP `07b97cad`**) and [MeshAnalysis exact union](issues/76-meshanalysis-footprint-union.md)
  (**resolved 2026-10-21, KEEP `465f2fd3`**) - and the remaining un-graduated base leads stay in the fog:
  the tree-support planner's 97-120 s serial native union/carve now has its
  measured batch/parallel contract ([ticket 77](issues/77-tree-support-planner-batch-parallel-contract.md),
  2026-10-22): **99.03 % of its 79.659 s clip core is inside `build_roles`**
  (472 big role unions + 69,161 tiny per-region carves, both independent per
  item), so the shape is **cross-call batching** through the existing
  `clip_polygons_batch` — exact per call — and the graduated candidate
  [emit-pass batched carve and role union](issues/78-tree-support-planner-emit-batching.md)
  is **resolved 2026-10-22, human-approved KEEP `73a6f1e8`**: proof 16/16 and
  the standing gate PASS in all four classic supports-on cells (wall
  −13.41/−12.30/−7.22/−16.66 %, CPU +0.51/+1.51/+0.74/+2.80 %), taking the
  79.7 s clip core to ~30.7 s (2.6x effective). The
  `EMIT_BATCH_LAYERS`/`CLIP_BATCH_MAX_REQUESTS` lever was
  [swept and closed](issues/79-tree-support-planner-emit-batch-constant-sweep.md)
  on 2026-10-22: the committed `4 / 256` is the measured choice (chunk knee at
  `4`; the `4096` cap's instrumented bracket edge did not transfer — **+6.30 %
  wall** in the frozen-snapshot confirmation), so no constant changed.
  The prefix chain, t76's intra-call partition and the ladder unions are
  measured dead (0.13 %, no target above 436 ms, 0.7 %). Still fog: the
  `PrePass::OverhangAnnotation` (9.3-10.9 s wall) and `SeamPlanning`
  (8.6-10.9 s serial) are **resolved by
  [ticket 84](issues/84-base-common-floor-attribution.md)** (2026-10-24). Seam
  is 92-93 % one serial native inset offset per region and graduates as
  [85](issues/85-seam-inset-offset-batching.md); overhang is engine-bound
  (99.7 % kernel, 85.4 % canonical three-offset work, 11.86x parallel, operand
  window retains 97.84 %), so no shape ticket exists and only an engine-level
  speedup moves it. The rest of the base common floor is ticket 59's closed
  Slice verdict. After t75 the residual phase-B serial work is the
  depth gather, the angles step and the 9/121 affected-layer recompute; the
  stage's Pass 1 `apply_opening` round-join arc-tolerance-0 anomaly (t27
  candidate (a)) **graduated 2026-11-18 as
  [166](issues/166-shell-opening-canonical-parity.md)** (priced at 53–59 s
  thread-time on base classic-off, canonical parameters 12.6–14.3 s; see the
  re-route status block) — previously **bounded
  by [t126](issues/126-post-t123-phase-split-refresh.md): `PrePass::ShellClassification`
  is **14.7 s** of the base classic-off cell and **15.3 s** of base classic-on,
  beside `PrePass::OverhangAnnotation`'s **9.7 / 10.3 s**, so no shape inside the
  stage can supply the destination's now-measured **72–101 %** prepass cut.**
  (t104's 14.0 / 22.0 s figures are superseded; its base-on pass carries a
  disclosed +43.6 % instrumented inflation.)
- **Named leads from the t60 host:native attribution (2026-10-14).**
  [host:native attribution](issues/60-host-native-attribution.md)
  names two reducible shapes beyond the now-resolved (KEEP 2026-10-16)
  [Region-preparation derivation sharing across region projections](issues/67-region-preparation-derivation-sharing.md):
  the **classic-perimeters per-layer host-service round trips** (8
  `offset_polygons` + 3 `clip_polygons` calls per layer; 89.26 s accumulated
  base-off and 85.83 s base-on vs 5.91–6.31 s benchy — the largest
  supports-off owner) **resolved 2026-10-16 as an attribution take with no
  candidate** ([ticket 68](issues/68-classic-perimeters-host-round-trips.md);
  the bucket is 97–99 % native clipper ops and every named reduction
  direction's ceiling is sub-noise — see the ticket and
  [`evidence/t68-host-roundtrips/`](evidence/t68-host-roundtrips/FINDINGS.md));
  the **`region_partition` 3×-per-layer re-run** (10.04 s accumulated base-off;
  dedup needs semantic analysis of what the post-process commit can change)
  remains un-graduated, awaiting the human's route decision — **bounded by
  t104: the whole per-layer phase is 6.54 s implied wall of the 75.38 s
  base-off cell (12.70 s of base-on), so no per-layer shape can reach the
  park.** The standing
  paired A/B governs any candidate.
- **Gap-candidate pipeline cost (t68 observation, 2026-10-16).** 46.8 %
  (base-off) / 49.5 % (benchy-off) of the classic-perimeters round-trip
  bucket produces raw gap candidates of which the width-band morph discards
  ~98.6 % of the raw vertex volume on every layer (D 4,632 + H 2,776
  verts/call → 105 verts/call into the filter; gap fill emitted on 162/495
  base layers). Re-enter only if the parity program ever re-derives the
  gap-fill algebra in a representation-identical form.
- **Orca-side attribution is now measured, and the remaining depth is
  elevation-gated (resolved 2026-11-03 by
  [ticket 110](issues/110-orca-side-attribution.md)).** The map carried "little
  introspection" as an assumption; t110 measured it: the CLI log buffers (27 stage
  markers in **3 bursts**, no timestamps at any `--debug` level), ETW kernel sampling
  is refused without elevation, and `--pipe` never connected in two attempts. The
  observable split has Orca's base wall **42–73 %** model import + startup and its
  whole slicing phase at **0.64 / 1.28 / 3.68 / 14.01 s** against PnP's
  **7.37 / 11.78 / 80.22 / 228.32 s** (**11.6x / 9.2x / 21.8x / 16.3x**), with the
  prepass carrying both engines and the pinned supports-on prepass increment at
  **8.34 s** against PnP's **~113.9 s** (**13.7x**). The only deeper route left needs something
  this rig does not have: an elevated ETW session, a working `--pipe` connection
  (re-test before pricing it), or an instrumented Orca build. Re-enter only with one
  of those in hand — otherwise the phase split is the floor of what this rig can see,
  and no substage estimate should be quoted.
- **Occupancy gate: the latent aggregate-drop fired and is closed; the gate
  itself stays.**
  [Support-correctness repair](issues/14-support-correctness-repair.md)
  un-widened only the extent bound. The exact-Z occupancy check later **did**
  fire on every supports-on matched run — [t147](issues/147-supports-on-body-rejection-attribution.md)
  censused 101 benchy / 44,474 base `(body, demand)` rejections — because the
  planner carved the layer **midpoint** while `validate_entry` queried
  `anchor_z` = `layer.z`. [t148](issues/148-supports-on-plane-carve.md) closed
  the plane mismatch planner-side (`print_plane_occupancy`, additive 2.1.0) and
  kept the host gate at zero tolerance as the tripwire: all four supports-on
  cells now report zero rejections and print the support they plan. Re-enter
  only if a future cell fires the gate again; do not weaken it to a tolerance.

- **Dense-infill bridge shortcut and per-extrusion object mass (screened
  2026-10-27, OrcaSlicer
  [#16741](https://github.com/OrcaSlicer/OrcaSlicer/pull/16741)).** The PR's
  dense-below path (`prints_dense` → `diff_expanded`) fires only when every fill
  surface of the lower layer prints at 100 % density. The matched job resolves
  `sparse_infill_density = 20 %`, so the path cannot fire on any map cell. PnP's
  `gate_internal_bridge_sites` already pools dense `infill_areas` into
  `lower_layer_solids` under its own `density >= 0.999` branch, but
  `unsupported_span_areas` still pays the full four-op chain. Re-enter only with
  a 100 %-infill model-class arm, which the map does not measure. The PR's other
  half, the per-extrusion object-mass locator, has no PnP counterpart: the
  emitter walks `ordered_entities` with no per-object lookup, and
  `PostPass::GCodeEmit` is 0.17 s off / 7.13 s on, of which t88 attributes
  ~6.15 s to the collection drop.
- **Per-part brim contact grouping (screened 2026-10-27, OrcaSlicer
  [#16733](https://github.com/OrcaSlicer/OrcaSlicer/pull/16733)).** PnP has no
  per-part identity: `resolve_object` (`crates/slicer-model-io/src/loader.rs`)
  merges normal 3MF parts into one solid mesh, and
  `SkirtBrim::generate_brim_entities`
  (`modules/core-modules/skirt-brim/src/lib.rs`) builds one print-wide
  rectangular loop set from `compute_bbox`. There is no contact test to
  accelerate. Re-enter only if brims ever become per-part. **(2026-11-09,
  [t130](issues/130-brim-matched-settings-ruling.md): the premise is partly
  superseded — Stage 1 of
  [135](issues/135-cross-layer-occupancy-batching.md) retires
  `generate_brim_entities` for canonical `make_brim` geometry, which is
  object-anchored. Whether canonical's per-object brim re-opens this screen is
  a Stage-1 finding; the per-part identity gap in `resolve_object` is
  untouched.)**

- **Dev-loop compile/test levers not yet priced (2026-11-02, from t108).** The
  inventory in [ticket 108](issues/108-dev-loop-latency-preflight-concurrency.md)
  leaves five levers unpriced rather than rejected: the **cold-workspace effect
  of `debug = 1` / `debug = 0`** (needs two clean target dirs), **linker
  substitution on a cold full build** (the incremental relink is only 12.8 s,
  but cold full builds were not measured), **`cargo-nextest`** on the five test
  buckets, **`sccache`**, and the **CI half** (cache keys, job parallelism, path
  filters, `cancel-in-progress`). **Blaze (formerly ra-jit) was tried rather than
  assumed.** Its two open Windows PRs were built and run on this host
  (#1527 ships no archive; #1591 builds in **57m34s** and starts a daemon), and
  the spike stops for reasons stronger than platform eligibility: two port bugs
  (`stand_ins` makes `hard_link` stand-ins and reads them with `read_link`, so
  every daemon start after the first fails with `os error 4390`; the store's
  `Access::Alone` check refuses a default profile ACL) and a **refusal to compile
  any crate with a build script** — 241 std instances need the LLVM backend, and
  the author's LLVM build stops at the `f16`/`f128` codegen. Its LLVM rebuild was
  abandoned after **92 minutes**. On WSL2 the released Linux build does run and is
  **slower than cargo on this repository in all three measured paths** (cold build
  193 s vs 54 s, warm edit 12.4 s vs 0.95-1.13 s at the crate root and 15.8 s vs
  2.4-3.5 s for a function body, `blaze test` 312 s vs 49 s; the 23-33 ms win is
  real only on a toy crate). **No blaze speedup is claimed for this repository**;
  re-enter only when upstream changes the workspace re-check shape, lands
  f16/f128 codegen and proc-macro/build-script support on Windows, or the
  dependency graph shrinks enough for the measured inversion to move.
  The measured weight sits in **first codegen of a bucketed test target**
  (994 s once, 17.26 s incremental), so a lever that only moves gate overhead is
  not the route. Any adoption must beat t108's **5 s per invocation** floor on
  paired medians. Both harness-correctness findings recorded by this take are now
  fixed by [ticket 109](issues/109-xtask-host-algos-injection-and-narrow-run-example.md):
  the `--features slicer-core/host-algos` injection is conditional on a
  `slicer-core` selection, and the `AGENTS.md` narrow example runs its 9 tests
  instead of zero.

## Out of scope

- **Matching Orca's `tree_strong` support engine to make the supports-on
  adhesion-oracle cells pass.** [t138](issues/138-layer0-cross-section-parity.md)
  measured Orca's first-layer support **10.6–11.7x** denser by path and
  **10.4–11.5x** by coverage than PnP's tree support, which is why the four
  supports-on cells sit outside the oracle bands (brim +54.5% swept on benchy;
  skirt +12.5% on base). Making them pass is an engine-parity program for an
  evidence check, not a speed lever, and it is not a matched-settings asymmetry.
  It returns only if the human grants a t81-style exception or redraws the
  destination to include support-engine parity.

- **Pruning the support-attribution table as a memory or hygiene change.**
  [Ticket 101](issues/101-support-identity-table-materialisation.md) measured the
  table at **1.2486 GB** of heap with **no production reader** and priced its
  removal at **822–927 ms = 0.62–0.70x the floor** — below the floor on the map's
  own currency, so it is not route work. Deleting the field would still reclaim a
  quarter of the process's peak working set and retire a dead `pub` IR surface,
  but **memory is not judged here** (DEV-026, first bullet below) and a contract
  deletion is not a speed lever. Returns only if the destination is redrawn to
  include memory, or if a production consumer appears and makes the table work
  rather than dead weight.
- **Memory instrumentation / peak-RSS bounds (DEV-026).** The destination's
  verdict is speed (wall, CPU corroborating); memory is not judged
  (2026-09-22 scoping decision). Returns only if the destination is redrawn.
- **OrcaSlicer geometry-parity work.** The repo's parity program runs as spec
  packets; this map treats output as disclosed evidence only (the fairness
  contract), not as a parity target.
  **Six recorded exceptions.** (1) **2026-10-23:** the human ruled that ticket
  81's `resolution` port be carried in-map end to end, because the fairness
  contract needed the key implemented rather than merely disclosed. That
  exception is scoped to that key. (2) **2026-11-03:** [ticket
  111](issues/111-prepass-rewrite-scope-ruling.md) authorizes a **scoped
  canonical-parity exception for the prepass** — `PrePass::Slice` first, the
  Strong planner second — under the fresh-16-cell-reference, all-8-cell paired
  gate; no production edit happens before the Phase-1 analysis
  ([ticket 112](issues/112-prep-slice-parity-phase1.md)). This exception is
  scoped to the prepass and its shapes, is not a standing licence for the rest of
  the parity program, and the next such question still goes back to the human.
  **Extension (2026-11-08, [Destination arithmetic](issues/128-destination-arithmetic-ruling.md)):**
  the t111 exception is carried forward from prepass *shapes* to the prepass
  *implementation* — the supports-off cluster and the planner may be replaced by
  canonical's work set at canonical's cost, gated stage-by-stage by the
  work-set/per-op decomposition
  ([129](issues/129-prepass-work-set-per-op-parity.md)). It does **not** extend
  to the per-layer phase: the benchy-classic-off per-layer residual may be
  attacked as a speed shape, but any change that alters computed geometry there
  still needs its own t81-style exception from the human.
  **Standing four-cell gate for representation-safe planner-only candidates
  (2026-11-07, [Strong planner carry](issues/117-strong-planner-carry-ruling.md);
  generalized 2026-11-12 by [Planner scheduling gate](issues/137-planner-scheduling-gate.md)):**
  any representation-safe planner-only candidate (`PrePass::SupportGeometry` /
  `PrePass::SupportAnalysis` / `com.core.tree-support-planner`) where the four
  supports-off cells are structurally inert (`enable_support: false` declines
  before the candidate's work) uses the four supports-on cells for its positive
  0.80 % paired wall/CPU improvement gate, requires byte-identical output in all
  eight against the active reference set, and requires no median wall/CPU
  regression in the four supports-off cells. This is not a parity exception and
  leaves the 8/8 destination unchanged. *(Note, 2026-11-21,
  [t175](issues/175-tiled-collision-carve.md): Clipper2's
  `vendor/clipper2-rust/src/engine.rs::add_new_intersect_node` clamps shallow
  intersections to global `scanline_list` Y coordinates contributed by unrelated
  distant `clip` vertices; any Clipper2 clip-operand reduction on `base.stl`
  therefore unclamps those ~1 µm intersections and requires an explicit human
  KEEP ruling on the moved hashes rather than literal 8/8 byte-identity.)*
   **(3) 2026-11-09:** [Matched settings vs the 8 mm brim](issues/130-brim-matched-settings-ruling.md)
   rules that parity work is licensed **to close matched-settings asymmetries**:
   when the arms diverge on a job-affecting setting, the divergence is closed —
   by authored shared values and/or a port of the canonical implementation, with
   the OrcaSlicer attribution header — before any new ratio of record is quoted,
   and disclosure is a named exception rather than the default. This is a
   fairness licence only: open-ended route parity work still needs its own
   t81-style per-key exception, and the t111/t128 prepass exception is
   unchanged. Its first application is the ruling’s Stage-1 work (canonical
   `make_brim` / `Print.cpp` skirt port + matched `brim_width = 8` /
   `skirt_loops = 6` / `skirt_distance = 3.0` / `skirt_height = 1`), folded
   into [t135](issues/135-cross-layer-occupancy-batching.md).
   **(4) 2026-11-12:** [Brim anchor: feed the slice contour (or reconstruct
   it) into `skirt-brim`?](issues/140-brim-contour-input.md) grants a **scoped
   t81-style exception for the `com.core.skirt-brim` anchor inputs only** — the
   layer-0 object footprint and the `skirt_height` layer hulls — so the module
   may reconstruct canonical `make_brim`'s slice-contour anchor from the
   outer-wall path plus half its width. It is the fourth recorded exception, is
   **not** a licence for other postpass parity work, and does not change the
   *Asymmetry* glossary definition or the t130 close-asymmetries ruling. The
   work is [t144](issues/144-brim-anchor-reconstruction.md).
   **(5) 2026-11-20:** [Port canonical `append_offset_path` edge decimation into
   `polygon_ops`](issues/169-canonical-offset-edge-decimation-and-flat-opening.md)
   grants a **t81-style exception for the per-layer phase's offset geometry**,
   scoped to canonical's pre-offset edge decimation
   (`ClipperOffsetShortestEdgeFactor = 0.005`) and its **two decimated passes** —
   nothing else in `polygon_ops`' wrapping, and no other per-layer geometry
   licence. It was granted in-session on the measured campaign (+21.9…+25.1 %
   wall on the four `base` cells, 5/5 pairs each, gate PASSED 8/8) because the
   decimation also compiles into `classic-perimeters` and `arachne-perimeters`,
   which call the helpers in-process. It is **not** a licence for `c2_offset_as_group`,
   the `c2_offset_setup` clamps, or any further per-layer port: those still return
   to the human. The t111/t128 prepass exception is unchanged.
   **(6) 2026-11-20:** [Port Orca's `OffsetConcaveCrossing` into
   `vendor/clipper2-rust`](issues/172-port-offset-concave-crossing.md) grants a
   **t81-style exception** scoped strictly to porting canonical `v2.5.0`
   (`40ecc66f78`) `clipper.offset.cpp::OffsetConcaveCrossing` (with `.round()`
   integer conversion and the AGPLv3 attribution header) into
   `vendor/clipper2-rust/src/offset.rs` (`ClipperOffset::offset_polygon`),
   covering all downstream host and guest `polygon_ops` offset callers
   (prepass, per-layer `classic-perimeters` / `arachne-perimeters`, off-matrix
   `lightning-infill` / `wave-overhangs`, and host-bridged offset callers).
   Granted by the human before production edits (Q1 ruling) and landed after
   passing the strict Q2 gate in 8/8 (+14.89…+23.01 % wall, 5/5 pairs and CPU
   corroborating in every cell). Explicitly **excludes** `DoRound`
   (`std::round` vs `ceil`), `GetLineIntersectPt`, `c2_offset_as_group`, and
   any other `clipper2-rust` or `polygon_ops` wrapper changes; those still
   return to the human.
- **Making the resolved-config-less fallback block satisfy Orca's viewer gate.**
  Ticket 34's residual — the test/legacy path's `CONFIG_BLOCK` cannot reach 80
  *loadable* pairs because the remainder are PnP's own key names — is past this
  destination: every measured production path passes a registry projection (233
  pairs, 161 loadable), so no cell's disclosure depends on it. Whether those
  entry points should synthesize a viewer block at all is a product decision, not
  route work; it returns only if the destination is redrawn.
