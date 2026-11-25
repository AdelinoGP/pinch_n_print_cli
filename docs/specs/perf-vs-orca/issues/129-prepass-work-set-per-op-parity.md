# Prepass work-set and per-op parity against canonical v2.5.0

Type: task / Phase-1 attribution
Label: wayfinder:task
Status: resolved (2026-11-08; Phase-1 decomposition — **the work set is not the
  lever; per-op cost / operand locality is**, and the prepass-only route wins
  base classic-off only. See ## Answer)
Assignee: 2026-11-08 OpenCode wayfinder session — the human said "Continue the
  perf vs orca map" (no ticket named); 129 is the first unblocked, unclaimed
  AFK frontier ticket in order (130 is the human's grilling).
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Destination arithmetic after t123: re-aim, rewrite, or park?](128-destination-arithmetic-ruling.md)'s
  2026-11-08 ruling (keep 8/8; authorise the engine rewrite; this decomposition
  is its first gate).

## Question

Where does PnP's prepass spend work that canonical OrcaSlicer does not do — or
does more expensively — stage by stage, with op counts and per-op cost separated;
and which named shapes can remove enough of it to make the 8/8 destination
reachable?

## Why this ticket exists

[The destination ruling](128-destination-arithmetic-ruling.md) kept the 8/8
destination and authorised a rewrite of the engine's geometric work, on the
measured fact that the prepass is **62.1–87.1 %** of every cell, the required
removal is **72–101 %**, and Orca's own prepass on the same job costs **≈2.1 s
(base-off) / ≈10.4 s (base-on)** against PnP's **33.0 / 102.8 s**
([t126](../evidence/t126-post-t123-phase-split/FINDINGS.md), [t110](../evidence/t110-orca-phases/FINDINGS.md)).

The remaining PnP stages are all clipper-bound and every named shape has already
been screened:

- `ShellClassification` — 78–81 % two full-layer Miter offsets on
  `deep_infill_area` (`+1.5 ×` / `−4.5 ×` spacing), already moved into the
  parallel qualification phase by [t75](75-shellclassification-phase-b-commit.md).
- `OverhangAnnotation` — 99.7 % the parallel `annotate_overhangs` kernel, 85.4 %
  of its work the three round offsets per band layer at 11.86x
  ([t84](84-base-common-floor-attribution.md)).
- `SeamPlanning` — 92 % one serial native offset per region (t84).
- `SupportGeometry` + `SupportAnalysis` — the planner: occupancy batch DROP
  ([t125](125-strong-planner-occupancy-offset-batching.md)), union/simplify dead
  ([t53](53-tree-planner-union-simplify-current-job-attribution.md),
  [t55](55-large-body-union-corpus-and-internals.md)), clip core 31–36 % native
  ([t113](113-strong-planner-phase1.md)).
- `MeshAnalysis` — the exact footprint union, kept ([t76](76-meshanalysis-footprint-union.md));
  `PrePass::Slice` is already down **94.9 %** ([t123](123-facet-bridge-path-retirement.md)).

What no take has measured is the one fact the rewrite depends on: **what work
does canonical do, and what does PnP do, for the same stage?** Without it, a
rewrite is scope without a denominator.

## Canonical source pin (2026-11-08)

Local checkout: `D:\slicerProject\Orca(pnp_gui)` — fork of
`AdelinoGP/OrcaSlicer`, upstream `OrcaSlicer/OrcaSlicer`.

Checked out **`v2.5.0` =
`40ecc66f78ed72dc368e0c1ebe40d77f00cc8a19`** (2026-11-07 09:44 +0800, "Bump
version to 2.5.0"), detached HEAD. The tag's parent `4f33435636` is the last
functional commit; the tag itself changes only `version.inc` and the flatpak
metainfo, so the tree is behaviourally the release branch tip.

Disclosure: the rig's measured binary is the installed
`C:\Program Files\OrcaSlicer\orca-slicer.exe` (files dated 2026-11-06 23:00
local, embedded version `2.5.0`), one day before the tag. Source and binary are
not byte-proven identical; reconcile before quoting an op-level comparison as a
wall claim.

Restore of the fork's branch state after this work: `git switch pnp/main`
(previous superproject HEAD `9f329f13ae`, whose tree records the submodule at
`a63fd14e`), then `git -C pinch_n_print_cli checkout 32370802` to restore the
submodule worktree's prior commit
`323708021aef6f20502066039d22b3568544b1` (it was clean). `git submodule update`
would instead restore the recorded `a63fd14e`. The submodule directory is still
on disk but is now untracked, because the upstream tag carries no such
submodule.

Canonical citations are by **file + function**, never line numbers, per
`AGENTS.md`. Delegate `OrcaSlicerDocumented/` reads to a sub-agent; never load
the tree into the implementer's context.

## Scope

Stage order, each with the measured stage wall on the reference cells (t126,
instrumented attribution, never a wall claim):

1. **Supports-off cluster** — base classic-off `ShellClassification` 14.7 s +
   `OverhangAnnotation` 9.7 s (24.4 s of a 42.15 s cell), plus `SeamPlanning`
   1.7 s and `MeshAnalysis` 4.3 s.
2. **Support planner** — base classic-on `SupportGeometry` 55.8 s +
   `SupportAnalysis` 13.1 s (68.9 s of a 117.97 s cell).
3. **benchy-classic-off per-layer residual** — t126's counterfactual lands at
   **1.02x** with a zero prepass, and the phase is 97–99 % native clipper ops
   ([t68](68-classic-perimeters-host-round-trips.md)).

For every stage return three columns:

- **Canonical work set** — the function(s) that compute the same result, cited
  by file + function, and what they operate on: operand size, boolean ops per
  layer/region, join types, spacings, and how many passes per layer.
- **PnP work set** — the same, from the in-tree producer.
- **Op counts vs per-op cost** — measured where possible, derived otherwise,
  and **kept separate**. This is the decisive split:
  - if per-op cost is comparable and PnP's op count is higher, the route is a
    work-set port;
  - if the op count is comparable and per-op cost is 5–10x, the route is the
    geometry kernel;
  - if both are comparable, no rewrite on this stage can pay.

Each named shape gets a ceiling denominated by the map's **0.80 %** floor and by
the prepass fraction its cell needs (72–101 %).

## Pre-declared gate (kill gate)

The decomposition graduates a rewrite candidate only if it names, for at least
one stage, a shape whose measured or derived ceiling is at least the prepass
fraction that stage's cell needs. **If the decomposition shows PnP already
computes canonical's work set at canonical's per-op cost, the 8/8 destination is
not closable by this route: stop and return the finding to
[the destination ruling](128-destination-arithmetic-ruling.md) for re-aim or
park.** No production edit in this take.

## Acceptance

- A Phase-1 report with the per-stage three-column table and a ranked shape
  list, each shape carrying its floor multiple and the prepass fraction it
  serves.
- Every canonical citation resolves at the pin above and names file + function.
- A recorded **go/no-go on the rewrite**, on this ticket and in the map's fog.
- No production edit, no G-code reference move, both guest-freshness checks
  untouched.

## Evidence starts

- [t126](../evidence/t126-post-t123-phase-split/FINDINGS.md) §1–§3 — the split
  and the counterfactual arithmetic.
- [t110](../evidence/t110-orca-phases/FINDINGS.md) — Orca's observable phase
  walls on the same rig.
- [t75](75-shellclassification-phase-b-commit.md) — the two full-layer Miter
  offsets and the kept parallel qualification.
- [t84](84-base-common-floor-attribution.md) — `OverhangAnnotation` and
  `SeamPlanning` decompositions.
- [t113](113-strong-planner-phase1.md) and
  [t125](125-strong-planner-occupancy-offset-batching.md) — the planner's terms
  and the failed batch.
- [t68](68-classic-perimeters-host-round-trips.md) — the per-layer phase is
  97–99 % native clipper ops.

## Answer

**Resolved 2026-11-08 as a Phase-1 decomposition. The work set is not the
lever; per-op cost and operand locality are.** Canonical does *more* per-layer
Clipper work than PnP in the shell/external-surface cluster, yet its **whole
pre-G-code phase** is **15.7x / 9.8x** cheaper on base classic-off / -on, so PnP
does **not** already compute canonical's work set at canonical's per-op cost. The ticket's kill
condition is not met; a work-set-only rewrite is nevertheless not the route.

Three findings:

1. **The clean PnP-local work-set deletions are a partial tranche, not the
   route.** Replacing the overhang quartile bands (`PrePass::OverhangAnnotation`,
   9,689 ms base-off), the whole-mesh facet union (`PrePass::MeshAnalysis`,
   4,260 ms) and the seam inset (`PrePass::SeamPlanning`, 92.4 % of 1,718 ms)
   removes **15.5 s** of base classic-off's 42.15 s cell = **55.8 % of the
   27.8 s cut** the cell needs. On base classic-on it is **1.6 s of 92.0 s =
   17.5 %**; on benchy-off / -on it is **32.1 % / 14.9 %**. A work-set port
   cannot reach the destination.
2. **"Prepass at canonical cost" is not "zero prepass".** Substituting
   canonical's measured prepass (2.103 / 10.445 / 0.0505 / 0.5498 s) while
   holding PnP's other phases wins **base classic-off only** (0.787x); it ties
   base classic-on (1.012x) and loses benchy classic-off (1.053x) and benchy
   classic-on (1.075x). Full **geometry** parity wins all four (0.335x / 0.661x
   / 0.550x / 0.704x) because PnP keeps its fast import. **The benchy per-layer
   phase is part of the first program, not a last mile.**
3. **The kernel is at parity.** t70 measured the vendored Rust Clipper2 at
   parity or faster than the C++ twin. The 15.7x is above the kernel: it is the
   operand size and shape PnP feeds it. Canonical bbox-clips its expensive
   intersections (`clip_clipper_polygons_with_subject_bbox`), caches collision /
   avoidance by radius and layer (`TreeSupportData::calculate_collision` /
   `calculate_avoidance`), and computes overhang speed and seam quality as
   per-vertex `LinesDistancer` queries. PnP's hot ops are whole-layer polygon
   operations (3 overhang-band offsets + ~10 clips/layer; a 326,546-facet union;
   whole-layer `deep_infill_area` offsets; 495 per-region seam insets) and its
   planner occupancy is uncached at two sites (32.7 s / 864 calls, t113).

The stage verdicts (canonical work set vs PnP work set vs op count / per-op
cost):

| stage | canonical | PnP | verdict |
| --- | --- | --- | --- |
| `PrePass::Slice` | one batched `slice_mesh_ex` walk | one batched `slice_mesh_ex` walk + per-layer classification | same work; per-op cost |
| `PrePass::ShellClassification` | `detect_surfaces_type` (3 diffs + 2 openings) + `discover_vertical_shells` + `bridge_over_infill` + `process_external_surfaces` wave expansion (30–40 ops/layer) | Pass 1 (2 diffs + ≤4 offsets) + Pass 2 (horizontal shadow cascade) + internal-bridge gate (2 whole-layer offsets); **no wave expansion** (`DEV-200`) | **PnP counts less (≈15–25 vs ≈45–60 ops/layer) and costs ≥7x more**; work-set swap, not deletion |
| `PrePass::OverhangAnnotation` | `detect_overhangs_for_lift` (1 diff + 3 offsets) + per-vertex `LinesDistancer` distance at emit | 3 round offsets + ~9.8 clips per band layer; 4-band polygon partition | **PnP counts ~3x more and stages the work in the prepass**; work-set + locality |
| `PrePass::SeamPlanning` | `SeamPlacer::init`: loop-point candidates, per-vertex distances, no offset | 1 per-region `offset(−0.5 × width)` inset + projection | **extra work**; small |
| `PrePass::MeshAnalysis` | per-layer overhang polygons only | 2.46 M facet classes + half-edge map + 326 k-facet union | **extra work** |
| `SupportGeometry` / `SupportAnalysis` / planner | legacy `TreeSupport` path, collision/avoidance cached by `{radius, layer}` | same legacy path; occupancy uncached at 2 sites | **caching shape**; largest supports-on term |

**Go/no-go: GO for the engine rewrite, NO-GO for a work-set-only rewrite.** The
measured lever is operand locality and host geometry cost. The first
implementation ticket should be a **per-op cost decomposition of the common
prepass stages** (path conversion, offset sweep, PolyTree rebuild, allocation,
parallel scheduling) plus the operand-locality shapes (bbox-clipped
internal-bridge offsets, radius/layer occupancy cache, localized bridge
expansion); the three clean deletions above are its measured first tranche. The
literal single-stage gate in this ticket fires — no single stage's ceiling
reaches its cell's required prepass fraction (the largest, overhang bands, is
23.0 % of the base-off cell against 84 %) — but the route-level program is not
killed. The next gate is graduated as
[Common-stage per-op cost decomposition](131-common-stage-per-op-cost-decomposition.md).

**Ledger corrections.** (1) t126/t128's "zero prepass" counterfactual is not the
same as "prepass at canonical cost"; canonical's prepass is 2.1 / 10.4 / 0.05 /
0.55 s, not zero. (2) The t128 route order ("prepass first, then the
benchy-classic-off per-layer residual") under-scopes the per-layer phase: at
canonical prepass cost, benchy-on's per-layer phase is 1,644 ms against a
2,462 ms Orca wall and benchy-off's is 1,450 ms against 1,716 ms. The per-layer
phase must be in the same program on both benchy cells. (3) PnP's shell stage
does not need a "work-set port" to delete Pass 2: canonical does not run
`discover_horizontal_shells` at the matched `ensure_vertical_shell_thickness =
evstAll`, but it does run `discover_vertical_shells` and the external-surface
wave expansion that PnP omits — the port is a swap, not a pure saving.

**Evidence:** [`evidence/t129-prepass-work-set-parity/`](../evidence/t129-prepass-work-set-parity/FINDINGS.md)
(`verify.py` exits 0 checkout-only; `--self-test` rejects 8/8 negative controls;
`--orca-check` resolves 14 citations at `40ecc66f78`). No production edit, no
candidate, no G-code reference move; both guest-freshness checks untouched.

**Disclosures:** canonical per-stage walls do not exist on this rig (t110's CLI
log, ETW and `--pipe` channels all failed), so the canonical per-stage op counts
are source-derived and the per-op ratio is a phase-level bound. The measured
binary is the installed 2.5.0 (2026-11-06), not byte-proven identical to the
`v2.5.0` source pin (2026-11-07). The PnP per-op numbers come from pre-t123
probes (t75/t84/t76/t113) and are used for shares and per-call prices, not as
current wall claims; the current stage walls are t126's.
