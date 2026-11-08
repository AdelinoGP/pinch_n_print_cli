# Destination arithmetic after t123: re-aim, rewrite, or park?

Type: grilling
Label: wayfinder:grilling
Status: open (unclaimed; human's decision)
Assignee: —
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

_To be recorded on resolution._
