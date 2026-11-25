# Brim anchor: feed the slice contour (or reconstruct it) into `skirt-brim`?

Type: grilling
Label: wayfinder:grilling
Status: resolved (2026-11-12; human ruling — the anchor gap gets its own
  scoped t81-style exception, it is reconstructed module-side from the wall
  path + half width, and the work graduates as
  [t144](144-brim-anchor-reconstruction.md). See ## Answer)
Assignee: dev (2026-11-12 OpenCode wayfinder session — the human said
  "Continue the perf vs orca map"; 140 was the first frontier ticket in order)
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Layer-plane parity: sample at the canonical midpoint](139-layer-plane-parity.md)'s
  2026-11-11 oracle re-run. With the slice plane at the canonical midpoint, the
  base supports-off cells still fail the 3 % skirt band — now as an **undershoot**
  (swept −4.3 % / −4.4 %, outer loop −5.7 % / −6.6 %; the brim itself moved from
  +8.1 % to −5.2 % and passes its 12 % band).

## Question

`com.core.skirt-brim` anchors `make_brim` on the emitted **outer-wall path**,
which sits half an outer-wall width (0.20 mm PnP / 0.21 mm Orca) inside the
slice contour that canonical `make_brim` anchors on. At the old top-plane
sampling that inward anchor partly cancelled the top-plane contour overshoot;
with the canonical plane it is the whole residual, and it now makes PnP's brim
and skirt slightly **smaller** than Orca's instead of larger.

Should the module consume the slice contour directly (a `SliceIR` contour, or a
reconstruction by an outward half-wall offset of the outer-wall path), and is
that licensed by the t130 close-asymmetries ruling — or does it need a t81-style
per-key exception?

Considerations on file:

- The representation gap: layer IR carries beads, not contours; the t135 Stage-1
  oracle measured the anchor term at −27.65 mm of the +88.04 mm contour term,
  with a 5.0 % non-circular residual (t138 §2).
- The residual is small and inside the brim band; only the skirt's 3 % band
  fails, on two cells. A t130 close-asymmetry reading would license it; t138's
  answer said it is "canonical-fidelity work, not a matched-settings asymmetry"
  and "stays out of scope unless the human grants a t81-style exception". The
  two rulings need reconciling.
- If licensed: the reconstruction must be exact enough not to reintroduce a
  plane-dependent residual; a direct contour would need a SliceIR field (a
  schema addition) or a module-side offset whose accuracy is measurable against
  the t139 oracle in one campaign.

## Answer

**Resolved 2026-11-12 (HITL, four decisions). The anchor gap is real printed
geometry, so it gets its own scoped exception and is closed — not disclosed. It
is reconstructed module-side, and the work is graduated as
[t144](144-brim-anchor-reconstruction.md). This is a fairness fix, not a route
step.**

**1. Licence: a new scoped t81-style exception, not a t130 asymmetry.** The
t130 ruling licenses parity work "only to close matched-settings asymmetries",
and its first application was this very `make_brim` port; t138 then classified
the input gap as canonical-fidelity work, not a setting divergence. The human
kept that classification: the gap is a **setting-matched, implementation-level
geometry divergence**, so it is recorded as the **fourth** parity exception in
the map's Out of scope entry, scoped to the `com.core.skirt-brim` anchor inputs
(the object footprint and the `skirt_height` layer hulls). It is not a standing
licence for other postpass parity work, and the *Asymmetry* glossary definition
is unchanged.

**2. Why act at all.** The gap is not only an oracle nicety. `SkirtBrim::generate`
builds the brim anchor from the `ExtrusionRole::OuterWall` **centerline**
(`entity.path.points`), while canonical `make_brim` anchors on the slice
contour, which is half an outer-wall width (0.20 mm PnP / 0.21 mm Orca) outside
it. Consequence: the brim area's inner boundary is 0.1 mm **inside** the part
outline instead of 0.1 mm outside it, so the innermost brim bead is centered on
the part outline (≈0.2 mm of bead overlaps the part footprint) where canonical's
innermost bead's inner edge just touches it. The whole ring is 0.2 mm inward on
**every cell**. The oracle sees it as base supports-off's skirt undershoot
(−4.3 / −4.4 % swept, −5.7 / −6.6 % outer loop against the 3 % band; the brim is
−5.2 %, inside its 12 % band); the two benchy supports-off cells pass, and the
four supports-on cells stay support-engine-limited and out of scope. The oracle's
ceiling from this fix alone is **4/8**, and the skirt-brim surface is ~235 ms of
the job.

**3. Shape: uniform half-width reconstruction, module-only.** Offset each
object's filled outer-wall footprint outward by `0.5 × the resolved outer-wall
width` before the existing `make_brim` pipeline, and do the same for the
`skirt_height` layer hull inputs. No SliceIR/WIT/schema change: the finalization
world exposes only `print-entity-view`, and `SliceIR`'s contour would need a
contract addition for a ~235 ms surface. The convention already exists in the
repo — `Point3WithWidth.overhang_distance_mm`'s `boundary_offset = 0.5 × width` —
so the reconstruction uses the codebase's own slice-boundary model. **Staged:**
if a base Arachne cell still misses the 3 % band (its outer-wall width varies per
point), the ticket prices the variable-width bead envelope (module-only) or the
direct contour (contract addition) as its own decision; it does not open with the
expensive shape.

**4. Ceremony: standing fairness ceremony, no scoreboard re-derivation.** Fresh
16-cell output campaign on both build arms (the reference set must move — the
brim exists in all eight cells and the supports-on brim is clipped by support
geometry), delta audit (`Skirt`/`Brim` motion only, bounded `|dE|`), oracle
re-run on the four supports-off cells, and a 3-pair interleaved wall/CPU A/B as
attribution only. No 8-cells × 3-tools scoreboard re-derivation: the surface is
~235 ms and no ratio of record moves; t139's scoreboard stands with the
disclosure that the adhesion geometry moved again. t143 stays ahead on the
frontier by number; if it is taken after t144 lands, it checks against the
active reference set, not t139's.

**5. Stop rule: fairness-fix model (t139 precedent).** The landing gate is
geometry correctness (the anchor reconstructs the contour on every layer) plus a
bounded output audit; the oracle is the acceptance measure, not a pass/fail
gate. A band miss is disclosed and graduates the next shape — the candidate is
not reverted for missing a band. Reverting would leave the brim-part overlap
defect in place for a priming-skirt percentage.

**No production edit in this take** (tracker-only: this ticket, the new t144, the
map's destination status, selection scope, decisions index and the fourth
exception).
