# Destination arithmetic after t123: re-aim, rewrite, or park?

Type: grilling
Label: wayfinder:grilling
Status: resolved (2026-11-08; human ruling — **keep 8/8 and authorise an
  engine rewrite**, gated on the work-set / per-op parity decomposition against
  canonical `v2.5.0` (`40ecc66f78`). See ## Answer)
Assignee: 2026-11-08 OpenCode wayfinder session — the human said "Continue the
  perf vs orca map" (no ticket named); 128 is the only open child and the only
  frontier ticket.
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Post-t123 phase split refresh and per-layer/emit attribution](126-post-t123-phase-split-refresh.md)'s
  counterfactual table (2026-11-08), denominated by
  [t124](124-post-t123-orca-scoreboard.md)'s ratios of record.

## Question

Given the post-t123 split, what should the map's destination be?

The measured arithmetic says the current 8/8 destination needs a prepass
rewrite of **72–101 %** of today's prepass:

| cell | ratio of record | cut to beat | prepass share | if prepass 0 | prepass fraction needed |
| --- | ---: | ---: | ---: | ---: | ---: |
| benchy classic-off | 2.70x | 63 % | 62.1 % | **1.02x** | **101 %** |
| benchy classic-on | 3.61x | 72 % | 76.4 % | 0.85x | 95 % |
| benchy arachne-off | 2.50x | 60 % | 62.2 % | 0.94x | 96 % |
| benchy arachne-on | 3.59x | 72 % | 73.4 % | 0.95x | 98 % |
| base classic-off | 2.95x | 66 % | 78.3 % | 0.64x | 84 % |
| base classic-on | 4.63x | 78 % | 87.1 % | 0.60x | 90 % |
| base arachne-off | 2.36x | 58 % | 79.8 % | 0.48x | 72 % |
| base arachne-on | 4.65x | 79 % | 86.7 % | 0.62x | 91 % |

Three facts bound the decision:

1. **The per-layer/emit program cannot close the map.** Deleting the whole
   per-layer phase leaves 1.67x–4.14x in every cell; the postpass is 1.8–4.9 %
   of a cell and its largest bracket is 0.85–2.61x floor (t126).
2. **benchy classic-off cannot be won by prepass deletion alone** — its prepass
   share (62.1 %) is below its required cut (63.0 %), so the counterfactual
   lands at **1.02x**.
3. **No prepass stage dominates any more.** `PrePass::Slice` fell to 2.1 s on
   base classic-off (−94.9 %); the remaining prepass is a cluster:
   `ShellClassification` + `OverhangAnnotation` on supports-off, the support
   planner on supports-on. The screens on those stages are t59/t68/t76/t84's:
   mostly native-clipper-bound with sub-noise reductions, and the planner's
   representation-safe batch already failed its gate (t125).

So the destination cannot be reached by adding one more in-map shape; it needs a
decision about the shape of the effort itself.

## The ruling this ticket must produce

Pick one and record it in the map's **Destination** section:

- **Keep 8/8 and authorize a prepass rewrite.** Name the stages in scope
  (the supports-off cluster, the support planner, or both), the parity
  exception that governs them, and the acceptance (the standing 16-cell /
  all-8-cell gate). This is the t111 shape carried one stage further.
- **Re-aim.** For example: beat Orca on the cells where a zero prepass wins
  (seven of eight), or adopt a 1.5x bound on the best cells, or a per-fixture
  target. State which cells the map stops judging.
- **Park.** Stop the effort with the evidence intact and record the release
  bound.

Secondary decisions to settle while grilling, if they bear on the choice:

- Whether the **brim asymmetry** (PnP 8 mm vs pinned Orca 0) is closed or stays
  a standing disclosure — it bears on every ratio above.
- Whether the **adhesion/asymmetry disclosures** are adequate for a "matched
  settings" claim at whatever destination is chosen.

## Acceptance

- A recorded ruling in the map's **Destination** section, with the chosen
  target, the cells it judges, and the acceptance gate that follows from it.
- Any resulting route work graduated as its own ticket; the fog rewritten to
  match.
- No production edit in this take.

## Evidence starts

- [`evidence/t126-post-t123-phase-split/FINDINGS.md`](../evidence/t126-post-t123-phase-split/FINDINGS.md)
  §1–§3 — the split and the counterfactual arithmetic.
- [`evidence/t124-post-t123-scoreboard/SCOREBOARD.md`](../evidence/t124-post-t123-scoreboard/SCOREBOARD.md)
  — the ratios of record.
- [`evidence/t104-phase-split/FINDINGS.md`](../evidence/t104-phase-split/FINDINGS.md)
  — the pre-t123 arithmetic this take supersedes.
- The map's **Out of scope** entry on the prepass parity exception (t111) and
  the **Not yet specified** entries on the surviving prepass terms.

## Answer

**Ruling (2026-11-08, human): keep the 8/8 destination and authorise an engine
rewrite; the first gate is a work-set / per-op parity decomposition against
canonical OrcaSlicer `v2.5.0` (`40ecc66f78`).** The option "re-aim" and the
option "park" were both on the table and both declined.

**Why.** The arithmetic is now closed: the prepass is 62.1–87.1 % of every
cell, the required removal is 72–101 %, and benchy classic-off lands at 1.02x
even with a zero prepass (its per-layer + postpass + outside phases already
exceed Orca's whole wall). The remaining prepass is a cluster of clipper-bound
stages, each with its named shapes already screened (t75, t84, t113, t125,
t53/t55, t76) — the 0.80 %-per-shape machinery would need 72–99x its floor to
buy the cut. Orca's own prepass on the same job costs ≈2.1 s (base-off) /
≈10.4 s (base-on) against PnP's 33.0 / 102.8 s, so the target cost is
demonstrated by the competitor; the gap is implementation, not physics.

**Scope of the rewrite.** In stage order:

1. The supports-off cluster — `ShellClassification` + `OverhangAnnotation`,
   then `SeamPlanning` and `MeshAnalysis`.
2. The support planner — `SupportGeometry` + `SupportAnalysis`.
3. The benchy-classic-off per-layer residual (the last mile: it needs only a
   small cut once the prepass is gone, but its phase is 97–99 % native clipper
   ops).

t111's scoped canonical-parity exception is carried forward **from prepass
shapes to the prepass implementation**; it does not extend to the per-layer
phase, so any geometry change there still needs its own t81-style exception.
Acceptance remains the standing 16-cell reference set and the all-8-cell paired
wall/CPU gate.

**First gate (kill gate).** [Prepass work-set and per-op parity against
canonical v2.5.0](129-prepass-work-set-per-op-parity.md) must separate PnP's op
count from its per-op cost per stage. If PnP already computes canonical's work
set at canonical's per-op cost, the destination returns to this ticket for
re-aim or park.

**Canonical source pin.** The local checkout `D:\slicerProject\Orca(pnp_gui)`
was moved from `pnp/main` @ `9f329f13ae` to **detached HEAD at `v2.5.0` =
`40ecc66f78ed72dc368e0c1ebe40d77f00cc8a19`** (2026-11-07; the tag changes only
`version.inc` and the flatpak metainfo over its parent `4f33435636`). The
checkout used `-f` because upstream carries a case-only path rename
(`FlashForge` → `Flashforge`) that a case-insensitive filesystem refuses
otherwise; the superproject tree was clean, so no tracked content was
discarded. The `pinch_n_print_cli` submodule directory is still on disk (at
`323708021aef6f20502066039d22b3568544b1`) but is now untracked, because the
upstream tag carries no such submodule. Restore with `git switch pnp/main`, then
`git -C pinch_n_print_cli checkout 32370802` for the exact prior state (the
superproject's tree records `a63fd14e`; `git submodule update` would restore
that instead).

**Disclosure.** The rig's measured binary is the installed OrcaSlicer
`2.5.0` (files dated 2026-11-06 23:00 local, embedded version `2.5.0`), one day
before the tag; source and binary are not byte-proven identical. Any op-level
comparison must reconcile them before it is quoted as a wall claim.

**Graduated.** The route ticket is
[129](129-prepass-work-set-per-op-parity.md) (AFK, open, unclaimed). The
secondary decision — **8 mm brim vs pinned Orca 0** — is
[Matched settings vs the 8 mm brim](130-brim-matched-settings-ruling.md) (HITL,
open, unclaimed); it was not settled here, so the destination's
"matched settings" wording still carries the t103/t124 disclosure until it is.

**This take made no production edit and ran no timing campaign.** It changed
only this tracker (this ticket, the map, and the two graduated tickets) and the
local Orca checkout.
