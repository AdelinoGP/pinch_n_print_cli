# Prepass work-set and per-op parity against canonical v2.5.0

Type: task / Phase-1 attribution
Label: wayfinder:task
Status: open (unclaimed)
Assignee: —
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

_To be recorded on resolution._
