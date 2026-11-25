# Variable-width anchor: per-segment envelope or the direct contour?

Type: grilling
Label: wayfinder:grilling
Status: resolved (2026-11-13; the premise is refuted — the last supports-off
  oracle miss is not a variable-width anchor error. It decomposes into ~0.10 mm
  of brim-extent term (inside its 12 % band) plus a **0.1786 mm skirt-hull input
  defect**: canonical `Print::_make_skirt` folds the brim **area's outer
  contour** into the skirt hull, while `SkirtBrim::generate` folds the outermost
  brim bead's **centreline**. The Arachne variable-width increment is at most
  ~0.04 mm. Neither offered shape can pass the cell; the fix is graduated as
  [t151](151-skirt-hull-brim-contour.md). See ## Answer.)
Assignee: 2026-11-13 OpenCode wayfinder session — the human said "Continue the perf vs orca map" and chose this HITL grilling over the AFK t150
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Brim anchor reconstruction](144-brim-anchor-reconstruction.md)'s
  2026-11-12 oracle re-run: after the uniform modal-width reconstruction, the
  base Arachne supports-off cell still misses the 3 % skirt outer-loop band at
  **+3.45 %** (base classic-off now passes at +2.89 %; both benchy supports-off
  cells pass at +1.32 % / +1.34 %). The design pre-declared this case: a base
  Arachne miss prices the next shape rather than reverting the anchor.

## Question

Base Arachne's outer wall varies per point, so the t144 **uniform** form (one
modal width per object) cannot reconstruct the contour exactly. The measured
residual is the last supports-off oracle miss. Which shape takes it:

1. **Per-segment variable-width bead envelope (module-only).** Offset each
   outer-wall segment outward by its own half width and union the segment
   envelopes into the anchor footprint. It stays inside the t140 scoped
   exception (the `com.core.skirt-brim` anchor inputs), needs no contract
   change, and its cost is bounded by the same ~235 ms postpass surface. Open
   question: does the envelope produce a *fair* contour (round joins at
   segment ends, self-intersection cleanup) or a visible scallop?
2. **Direct slice contour (contract change).** Add the layer-0 slice contour
   (and the `skirt_height` layer hull contours) to the finalization world's
   reads, i.e. a `layer-collection-view` addition plus an IR schema bump and
   host plumbing. Exact by construction, but a contract change for a ~3 %
   adhesion band on one cell.
3. **Accept 3/8 and stop.** The adhesion oracle is a measure, not a revert
   gate; the route's destination arithmetic is untouched by any of these.

The answer must also price the chosen shape against the destination: the
skirt-brim surface is ~235 ms of the job, so none of these is a route step and
none carries a wall claim.

## Data the answer starts from

- Oracle (candidate arm, supports-off): benchy classic-off brim +1.79 % /
  skirt outer +1.32 %; benchy arachne-off +1.73 % / +1.34 %; base classic-off
  **+1.14 % / +2.89 %** (passes); base arachne-off **+1.86 % / +3.45 %**
  (misses by 0.45 points).
- Baseline arm for the same cells: brim +5.17 % / +6.73 % and skirt outer
  +5.65 % / +6.62 % on the two base cells — the uniform reconstruction closed
  most of a ~4-point gap.
- The supports-on cells are support-engine-limited and out of scope
  (t138/t139); their adhesion output is byte-identical before/after t144
  because the support footprint clips the entire brim area.
- The active matched-job reference set is t144's candidate set (see
  [t144](144-brim-anchor-reconstruction.md)'s answer); any later shape re-records
  it under the fairness ceremony.

## Answer

**Resolved 2026-11-13: neither offered shape. The last supports-off oracle miss
is not a variable-width anchor error — it is a skirt-hull input defect.
`SkirtBrim::generate` folds the outermost brim bead's centreline into the skirt
hull where canonical `Print::_make_skirt` folds the brim area's outer contour;
the difference is exactly half a flow spacing. The fix is graduated as
[t151](151-skirt-hull-brim-contour.md).**

### 1. The premise, tested

The ticket's premise is that Arachne's per-point width variation is what the
uniform modal-width reconstruction cannot represent, and that this is the
residual. Three measurements on t144's own artifacts falsify it.

**The prose lost the sign.** The oracle's relative difference is
`abs(pnp - orca) / orca` (`oracle_skirt_brim.py::rel`, under
`evidence/t135-skirt-brim-retake/`), so it carries no direction. Every
supports-off cell is an **undershoot** — PnP prints a smaller brim and a smaller
skirt than Orca — before and after t144: base classic-off goes 586.47 -> 603.64
mm² against a fixed 621.60 mm², base arachne-off 580.48 -> 600.15 against the
same. "−5.65 % → +2.89 %" is a magnitude history, not a direction flip. Corrected
in place in t144's Status and Answer, in
`evidence/t144-brim-anchor/FINDINGS.md`, and in this ticket's `## Data` above.

**The Arachne increment is small and not consistent.** Against the 3.0 % skirt
band (`evidence/t144-brim-anchor/reduced/summary.json`):

| cell | brim swept (12 %) | skirt outer (3 %) |
| --- | ---: | ---: |
| benchy classic-off | 1.79 % | 1.32 % |
| benchy arachne-off | 1.73 % | 1.34 % |
| base classic-off | 1.14 % | 2.89 % (0.11 pt headroom) |
| base arachne-off | 1.86 % | 3.45 % |

Arachne adds **+0.56 points** on base and **+0.02 points** on the benchy. A
width-reconstruction term would not be fixture-dependent in that way.

**The dominant term is shared with a cell where the reconstruction is exact.** On
classic the outer-wall width is uniform, so the t144 reconstruction
(centreline + half width) is exact by construction — and that cell still sits
2.89 % short. At most 0.56 of arachne-off's 3.45 points can therefore be a width
effect.

### 2. What the residual actually is

Canonical `Print::_make_skirt` (`Print.cpp`) and `SkirtBrim::generate`
(`modules/core-modules/skirt-brim/src/lib.rs`) agree on the ring distance:
canonical starts at `skirt_distance - spacing/2` and adds `spacing` per loop, PnP
uses `skirt_distance + spacing/2 + i * spacing`; the outermost centreline is
**4.964 mm** in both.

They disagree on the hull input:

- canonical appends `m_objectBrimAreasByInstance[...]` `area.contour.points` to
  `occupied_points` — the brim **area's outer contour**;
- PnP extends `brim_points` from `path.points` of the emitted brim beads (the
  `brim_points.extend(...)` in `SkirtBrim::generate`) — the outermost bead's
  **centreline**, half a flow spacing inside that boundary.

Measured per axis as `pnp_skirt_growth_mm - pnp_brim_growth_mm` (and the Orca
pair) in `evidence/t144-brim-anchor/reduced/summary.json`, base classic-off — the
reduced set now carries `growth_mm` (t146 extended `reduce_t144.py`; `verify.py`
still exits 0 against the enriched set). The fuller, optional source is t144's
raw G-code tree (`target/t144/output/gcode/base-classic-off-candidate-ordinary.gcode`
against `target/matched-pair/t144-oracle/gcode/base-classic-off-orca-m1.gcode`):

| arm | xmin | xmax | ymin | ymax |
| --- | ---: | ---: | ---: | ---: |
| Orca | 5.136 | 5.130 | 5.152 | 5.138 |
| PnP | 4.964 | 4.963 | 4.962 | 4.963 |

Predicted for the canonical construction: 4.964 + 0.35708/2 = **5.143**. Measured
mean **5.139**, residual 0.004 mm. The model is confirmed to a few microns.

**Decomposition of the ~0.29 mm radial skirt deficit:** ~0.10 mm brim-extent term
(the ring's own outer boundary is short by that much; inside its 12 % band and
shared with classic) + **0.1786 mm skirt-hull input term** + at most ~0.04 mm
Arachne width term.

### 3. Why neither offered shape takes it

- **Per-segment envelope** can move the anchor by at most the half-width
  variation, whose whole measured effect on this metric is the Arachne increment
  (0.02–0.04 mm, 0.02–0.56 points). It cannot address the 0.1786 mm term, and
  would leave ~2.9 points on the cell.
- **Direct slice contour** corrects the anchor, which is already exact on the
  classic cell that nearly misses; it cannot move the skirt-hull term either. It
  is a contract change (`layer-collection-view` plus an IR schema bump and host
  plumbing) for a ~235 ms surface, and it reverses t140's module-side ruling.

### 4. Ruling

Land the canonical fold: the skirt hull takes the brim **area's outer contour**,
not the outermost bead's centreline. Module-only in `com.core.skirt-brim`, no new
clipper operation, read as inside t140's exception, which names "the layer-0
object footprint and the `skirt_height` layer hulls". **No wall claim:** the
surface is t103's ~235 ms module-reported whole-print time, and no A/B on it has
cleared a measured null band.

Pre-declared effect — an **estimate** from the measured 0.1786 mm radial term and
the measured outer-loop perimeter (87.244 mm), ΔA ≈ P·δ ≈ 15.6 mm²; the ceremony
measures the outcome:

- base classic-off 603.64 -> ~619.3 mm² against Orca 621.60 -> **~0.4 %** (band 3.0 %)
- base arachne-off 600.15 -> ~615.8 mm² -> **~0.9 %**
- supports-off oracle **3/4 -> 4/4**

Ceremony (t151): fresh 16-cell reference set on both arms, a `Skirt`-only delta
audit (no motion outside `Skirt`/`Brim` with `E` stripped), the four supports-off
oracle cells, the accelerated **output** arm, and three paired wall/CPU runs as
attribution only.

**Not taken:** the per-segment envelope (refuted as a cell pass), the direct
contour (contract change, refuted as a cell pass), and accepting 3/8 (superseded
by a named, in-scope defect). The ~0.10 mm brim-extent residual stays **disclosed**
— it is inside its 12 % band, and the anchor is exact on the cell where the width
is uniform.

