# Seam inset offset: is the prepass offset redundant with the per-layer loop projection?

Type: task / Phase-1 attribution + candidate screen
Label: wayfinder:task
Status: resolved (2026-11-14; Phase-1 attribution + candidate screen — the prepass
  inset offset is **not** redundant: deleting it moves **456/496** base-classic-off
  layers (+1,177 / −1,149 points, 142 layers with a changed total `E`) and fails
  two unit tests that pin the inset contract. The measured offset term is
  **1,248 ms** (`PrePass::SeamPlanning` 1,779 → 531 ms, 70.15 %), not t129's
  1,587 ms / 92.4 %. The canonical loop projection is a **per-layer relocation**
  of candidate generation, graduated to [153](153-seam-loop-relocation-ruling.md)
  as a human ruling. See ## Answer.)
Assignee: 2026-11-14 OpenCode wayfinder session — the human said "Continue the
  perf vs orca map"; 1–151 are resolved, so the map's route arithmetic graduates
  the last named member of the sweep-volume work-set family from Not yet
  specified: t129's **S3**, the seam inset offset
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Prepass work-set and per-op parity against canonical v2.5.0](129-prepass-work-set-per-op-parity.md)
  §2.4 / §4 row S3 — `PrePass::SeamPlanning` is **1,718 ms** on base classic-off
  and **92.4 %** of it is `seam-planner-default`'s per-region inset offset
  (`offset_polygons_batch`, `-0.5 × width`); canonical `SeamPlacer.cpp` has
  **zero inset offsets** and instead projects the chosen point onto the printed
  external-perimeter loop. The clean ceiling is **1,587 ms** (4.7x the 0.80 %
  floor, **5.7 %** of base classic-off's required cut; 1,507 ms / 1.6 % on
  base classic-on). See
  [`evidence/t129-prepass-work-set-parity/FINDINGS.md`](../evidence/t129-prepass-work-set-parity/FINDINGS.md).

## Question

Does the prepass inset offset buy anything, or is it redundant with the
per-layer projection that already exists?

The seam path has two projections today:

1. **Prepass, `seam-planner-default` (`run_region_planning_entries`,
   `modules/core-modules/seam-planner-default/src/lib.rs`).** The planner picks a
   candidate on the raw region boundary, then offsets every region's polygons by
   `-0.5 × width` in one `offset_polygons_batch` and projects the chosen point
   onto that inset (`project_point_onto_inset_boundary`). This is S3's cost.
2. **Per-layer, `seam-placer` (`run_wall_postprocess`,
   `modules/core-modules/seam-placer/src/lib.rs`).** For `aligned` /
   `aligned_back`, it takes the injected `resolved_seam`, projects it onto the
   **actual wall loops** (`project_onto_wall_segment`, unlimited radius), inserts
   the projected point into the loop when it is interior, and rotates the loop to
   start there. This is canonical `SeamPlacer`'s loop projection, already
   implemented.

The matched job resolves `seam_position = aligned`, so every region goes through
both. The question is whether the per-layer projection lands on the same wall
segment whether it is fed the inset-projected point or the raw chosen point. If
it does, the prepass offset is a pure approximation of a point the next stage
re-projects anyway, and the **1,587 ms** is a clean deletion.

Resolve in order:

1. **Re-derive the stage at HEAD.** Measure `PrePass::SeamPlanning` on base
   classic-off (and base classic-on if cheap) on the post-t151 tree, and confirm
   the offset share. t129's 1,718 ms / 92.4 % is pre-t148/t150/t151; those takes
   do not touch the seam module, but the number is a ledger fact and must be
   re-derived before it is quoted.
2. **Screen the deletion, one cell first.** Candidate: remove the
   `offset_polygons_batch` call and `project_point_onto_inset_boundary` from
   `run_region_planning_entries`, and emit the raw chosen position. Nothing else
   changes; the per-layer `seam-placer` still projects onto the wall loops. Run
   base classic-off against the **t151 active reference** and compare G-code
   bytes. This is the falsifier: if the raw point lands on a different wall
   segment, the G-code moves.
3. **If identical, extend to all eight ordinary cells** (16 runs, baseline +
   candidate) and require 8/8 byte-identity. If any cell moves, stop, census the
   differing seam sections/layers, and price the parity cost instead of landing.
4. **Price the net saving.** A positive screen means the clean ceiling is the
   whole offset term, less the per-layer projection cost that was already being
   paid. Quote it as a share of the cell's required cut at the current ratios of
   record.

## Pre-declared gate

- **Screen (base classic-off):** G-code byte-identical to
  [`evidence/t151-skirt-hull-contour/reference.json`](../evidence/t151-skirt-hull-contour/reference.json).
  A single differing byte fails the clean-deletion reading and switches the
  take to a parity-delta census; it is not "noise".
- **Extension (8 cells ordinary):** 8/8 byte-identical, both arms sharing one
  host binary. Any cell that moves is disclosed with its delta audit and is not
  waived.
- **Timing:** the screen is a deletion candidate, so a positive result graduates
  a measured deletion ticket with the standing paired A/B; this take does not
  claim a wall number from the screen.
- **No production landing without the gate.** If the screen is positive, the
  candidate is preserved as a patch and the landing is a separate measured
  deletion under the prepass exception (t111/t128), with the fresh 16-cell
  reference and the paired gate.

## Route arithmetic (indicative)

At t129's same-run shares, removing the whole offset term is **1,587 ms** on
base classic-off = **5.7 %** of that cell's required cut, and **1,507 ms** /
**1.6 %** on base classic-on. On benchy the S3 share is **5.5 % / 2.6 %** of the
prepass, which is below the destination's required fraction. This is a tranche,
not a destination step; it matters because it is the last member of the clean
work-set tranche named by t129 and because the screen may show it is free.

## Answer

**Resolved 2026-11-14 — S3 is not a clean work-set deletion; the prepass inset
offset is load-bearing.** The screen removed `offset_polygons_batch` and
`project_point_onto_inset_boundary` from `run_region_planning_entries`
(`modules/core-modules/seam-planner-default/src/lib.rs`) and emitted the raw
chosen position; the per-layer `seam-placer` still projects the injected seam
onto the actual wall loops. On base classic-off:

- **Baseline reproduces the t151 active reference exactly**
  (`9bb18f0e8173…`). The candidate is `4265d619dbf1…`, **+1,158 bytes / +28
  lines**.
- **The delta is a geometry delta, not a rotation.** 456 of 496 layers have a
  different printed-XY multiset (+1,177 / −1,149 points), 142 layers have a
  different total `E`, and the first difference is the layer-0 **Skirt** (the
  skirt hull's brim input inherits the wall loop's start point). A pure loop
  rotation would preserve the multiset and the `E` total.
- **The unit tests falsify it too.** Baseline
  `cargo test --manifest-path modules/core-modules/seam-planner-default/Cargo.toml --lib`
  is **31/0**; the candidate is **29/2**. The failures are
  `batched_inset_offsets_match_the_serial_reference_per_region` (the t85
  batching contract: the raw `(0, 0)` replaces the serial reference's inset
  `(0.2, 0.19999981)`) and
  `candidate_less_region_still_emits_one_keyed_plan_entry` (the fallback must
  project the first sharpest vertex onto the inset boundary).

**The offset's price, re-derived at HEAD.** `PrePass::SeamPlanning` is **1,779
ms** on base classic-off with the offset, **531 ms** without it, so the offset
term is **1,248 ms = 70.15 %** of the stage. t129's 1,718 ms / 92.4 % is not
reproduced; the residual 531 ms is candidate generation, scoring and entry
emission. The 1,248 ms is **5.9x** the 0.80 % floor on the 26.254 s single-run
cell (floor 210 ms), **4.75 %** of the wall, and **10.5 %** of base
classic-off's current required cut (45.27 % at the 1.827x ratio of record). It
is a tranche, not a destination step.

**What the result means for S3.** The per-layer projection does not preserve the
seam because it receives a different source point: the inset point and the raw
boundary vertex select different wall segments, insert different seam vertices,
and rotate the loops differently. The prepass offset is therefore not an
approximation of a point the next stage re-projects anyway; it is part of the
current seam pipeline's output contract.

The canonical replacement is **not** a prepass-local deletion. Canonical
`SeamPlacer::collect_points` builds candidates from the printed
external-perimeter loop points, and those loops exist only after per-layer
perimeter generation. Matching canonical means moving candidate generation and
scoring into the per-layer phase — a stage relocation that changes computed
geometry there, which the map's out-of-scope section says needs its own
t81-style exception. Its net is the 1,248 ms offset term minus the moved
candidate-generation/scoring work (the 531 ms residual, or the part that does
not transfer) and minus any per-layer loop-collection cost; it is unmeasured.
The scope ruling is graduated to
[Seam candidate generation on the printed loops: authorize the per-layer relocation?](153-seam-loop-relocation-ruling.md)
(HITL, unblocked).

**Disclosures.** One box, one run per arm; no paired wall claim and no null band
are taken from this screen. No production change: the candidate patch is
preserved in the evidence directory, the source and guest are restored to HEAD,
and `cargo xtask build-guests --check` exits 0. The screen tests redundancy, not
canonical parity: both arms project onto the wall loop, and the candidate's
source point is the raw boundary vertex, not a printed loop point. Evidence:
[`evidence/t152-seam-inset-offset/`](../evidence/t152-seam-inset-offset/FINDINGS.md)
(`verify.py` PASS, `--self-test` 4/4 mutations rejected).
