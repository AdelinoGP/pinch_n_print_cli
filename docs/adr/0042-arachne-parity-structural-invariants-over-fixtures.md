# ADR-0042 — Arachne Parity Is Certified by Structural Invariants + LLM-Visual OrcaSlicer Comparison, Not Absolute-Unit Fixtures

<!-- filename: 0042-arachne-parity-structural-invariants-over-fixtures -->

## Status

Accepted (2026-07-16). Authored during the "Arachne Parity Recovery" campaign
(`docs/specs/arachne-parity-recovery.md`), opened 2026-07-15 after an audit of
the Arachne/Classic perimeter pipelines found the Arachne module "severely
lacking" despite a green test board.

## Context

The campaign's opening finding was a trust failure in the certifying
instruments, not only a feature gap: **the test suite could not detect the
brokenness.** Every pre-existing Arachne parity fixture was a *self-captured
baseline* — a snapshot of PnP's own prior output, recorded because this
environment has no OrcaSlicer binary to capture reference output from
(`docs/DEVIATION_LOG.md` rows `D-109-SELF-CAPTURED-FIXTURES` and
`D-112-SELFCAPTURED-BASELINES`). A self-captured snapshot regression-locks
PnP's *own current output*: green means "unchanged from the snapshot," never
"correct." A visibly broken pipeline can therefore sit behind a 100%-green
board.

The campaign produced direct proof of this, not just the general argument:

- **D5 (dominant benchy defect, resolved 2026-07-16, commit `5d0e1bcf`).**
  Arachne was dropping ~16mm of the bow cross-section entirely across ~40
  layers — geometry loss severe enough to be visible at a glance once
  compared against a `wall_generator=classic` render — yet the pre-existing
  self-captured Arachne fixtures were green the whole time this bug shipped.
  After the fix, two tapered-wedge self-captured baselines (backing
  `bead_count_tapered_wedge` and `generate_toolpaths_tapered_wedge`) drifted
  to "emit more" geometry — the *correct* direction — which is exactly the
  failure mode a self-captured baseline cannot distinguish from a regression:
  it had locked in the dropped-geometry state as the expected value.
- **D4 (inner-wall giant-bead over-extrusion, resolved 2026-07-16).** The
  regression pin that actually caught this class of bug is a structural,
  unit-independent width invariant — "no bead width > ~2× `optimal_width`" —
  not a numeric fixture comparison. The bug (a beading-propagation pass-order
  inversion) produced beads up to 19.6mm wide against a 0.45mm nozzle; a
  self-captured baseline would have recorded 19.6mm as the new normal on next
  capture.
- **`propagation_fills_gap_from_central_neighbor` literally asserted the
  defect while passing.** This pre-existing test asserted
  `bead_count == Some(4)` on a propagated joint — the exact wrong behavior
  the D4 fix corrected — and stayed green because its fixture gives every
  vertex `distance_to_boundary = 5.0`: under uniform thickness, "copy the
  propagated beading" and "recompute at the destination" are indistinguishable,
  so the fixture could never exercise the bug it was nominally guarding. A
  fixture that cannot vary the quantity under test is not a test of it.

Absolute-numeric OrcaSlicer fixtures were considered and also rejected, for a
different reason: PnP's coordinate system is **1 unit = 100 nm**
(`docs/08_coordinate_system.md`), and diverges from OrcaSlicer's in absolute
units by construction (rounding policy, float/algorithm drift, and — per the
campaign's D3 finding below — even the two slicers' Z-sampling planes are
offset). An absolute-coordinate equality fixture is flaky-by-construction
*even when the output is geometrically correct*. ADR-0035 already rejected
building a real OrcaSlicer C++ checkout for numeric golden fixtures on
infrastructure-cost grounds; this ADR rejects absolute-numeric fixtures on
correctness grounds even where the infrastructure exists.

The campaign settled on a **two-instrument model** instead
(`docs/specs/arachne-parity-recovery.md`, "The two-instrument model"):

| Instrument | Purpose | Basis | Committed? |
|---|---|---|---|
| LLM-visual steering | *Find* defects; localize where geometry first breaks | Claude renders PnP and canonical OrcaSlicer gcode via `pnp_cli visual-debug` and compares them with multimodal vision, semantically not pixel-exact | No — `tmp/orcaSlicer_arachne_benchy.gcode` uncommitted; tests never read it |
| Structural-invariant regression | *Prevent* regressions; gate CI | Unit-independent assertions (closure within tolerance, loop count/nesting, bead-count sequence, transitions-present, no self-intersection, coverage ratio, no bead wider than ~2× optimal width) on synthetic fixtures reproducing benchy error classes | Yes — committed, host-algos-gated |

The trade-off this ADR accepts: giving up automated, absolute-numeric parity
against OrcaSlicer (which is unattainable without the coordinate-system
caveat above, and was never attainable at all under self-captured baselines)
in exchange for tests that certify a *geometric correctness property* rather
than *bitwise sameness with a possibly-broken prior run*. That trade-off —
and the fact that it is a real trade-off, not a free upgrade — is what makes
this ADR-worthy: a future maintainer could reasonably ask "why don't we just
diff against OrcaSlicer gcode," and the answer needs to be on record.

The LLM-visual instrument also proved unreliable as an *adjudicator* of
mechanism, only as a *flag* that two renders differ. Three separate false
positives occurred in one session, all on the same underlying symptom ("PnP's
geometry looks different/broken here"):

1. **Layer-96 "fragmentation."** A filament-lines render of PnP's final gcode
   was read as "severe wall fragmentation." A structural stage-walk (model-mode
   `Layer::Slice` + `Layer::Perimeters` typed IR, checking `closure_gap`)
   disproved it: the mesh cross-section at that layer genuinely has 5 disjoint
   closed polygons pre-Arachne, and Arachne correctly emitted 5×3=15 closed
   loops, `closure_gap=0.0000` throughout. What looked like broken fragments
   in a line render was 5 legitimate separate closed islands.
2. **D1 "faceting."** Visually read as perimeter over-simplification. Refuted
   by structural point-density comparison: PnP's outer-wall vertex density
   matched OrcaSlicer's at the sampled layers; the visual impression was a
   viewport/autofit artifact (PnP's skirt+brim inflating the toolpath bbox).
3. **D3 "mid-hull loop fragmentation."** Visually read as PnP splitting a
   cross-section OrcaSlicer keeps whole. Refuted by a direct plane-triangle
   mesh cross-section probe: the topology split (3→5 outer-wall loops) is a
   *real* mesh feature both slicers reproduce, just at Z labels offset by
   ~0.25mm between the two slicers' own layer-Z conventions. At the
   geometrically aligned layers, PnP and OrcaSlicer loop geometry matched
   pointwise (e.g. PnP's 147-pt loop vs. Orca's 137-pt loop at the same plane,
   bboxes within ~1%).

## Decision

**Automated tests for Arachne correctness MUST be unit-independent structural
invariants**, not fixture-equality checks. Examples of the invariant class
(non-exhaustive; see `docs/specs/arachne-parity-recovery.md` "two-instrument
model" table and D4's Track-B invariants): closure within tolerance, loop
count/nesting, bead-count sequence, transitions-present, no self-intersection,
coverage ratio vs. a known-correct reference, and "no bead wider than ~2×
`optimal_width`". Green on these tests means "structurally right," not
"byte-identical to a prior run."

**Self-captured baselines (snapshots of PnP's own output) are REJECTED as
parity oracles.** They regression-lock PnP's own — possibly broken — output,
so a green suite proves only "unchanged from the broken snapshot," as D5
demonstrated directly. They are **demoted to change-detectors**: retained
where a structural invariant does not yet exist for a given fixture, useful
for catching *unintended* drift, but never cited as evidence of OrcaSlicer
parity and never sufficient grounds to claim a code path is correct.

**Absolute-numeric OrcaSlicer fixtures are ALSO rejected as an automated
basis**, independent of the self-captured-baseline problem. PnP's coordinate
system (1 unit = 100nm, `docs/08_coordinate_system.md`) diverges from
OrcaSlicer's absolute-unit output by design; an absolute-coordinate equality
test is flaky-by-construction even when the underlying geometry is correct.

**The OrcaSlicer oracle (`tmp/orcaSlicer_arachne_benchy.gcode`, uncommitted)
is an LLM-visual/steering reference only.** Automated tests never read it. It
exists to help a Claude agent *locate* where PnP output diverges from
OrcaSlicer's, not to certify correctness numerically or by itself.

**Process corollary — the LLM-visual instrument FLAGS, it never
adjudicates.** A multimodal read of a rendered PNG may correctly identify
"these two renders differ," but the *mechanism* — is this a real defect, a
closed-island vs. open-wall ambiguity, a viewport artifact, or a
layer-alignment artifact — MUST ALWAYS be settled structurally, by reading
gcode or typed IR, never concluded from the image alone. The campaign
produced three false positives (layer-96 "fragmentation," D1 "faceting," D3)
by skipping this step; all three were correctly refuted only once someone
went to the structural evidence.

**D3 corollary — a comparison is only valid at an aligned sampling plane.**
"PnP splits what OrcaSlicer keeps whole" (or any topology-difference claim
between the two slicers) is a *comparison* claim, and before attributing a
difference to a defect, the two references must be confirmed to sample the
same physical mesh plane. **Z labels are not an alignment** — PnP and
OrcaSlicer were found to assign Z labels to the same physical plane with a
~0.25mm offset (a separate, real slice-plane Z-convention deviation, out of
Arachne scope, tracked in the campaign doc's Track A findings). The cheapest
discriminator: sweep the metric across a Z band in both references and look
for the same transition at a shifted offset (shifted-but-identical = alignment
artifact; present in one and absent in the other = real defect).

## Rejected alternatives

- **Keep self-captured baselines as the sole automated basis, accepted as a
  known limitation (the pre-campaign status quo, D-109/D-112).** Rejected:
  D5 proved this concretely — a severe, visible geometry-dropping bug shipped
  and stayed green under self-captured baselines for as long as the bug
  existed, and `propagation_fills_gap_from_central_neighbor` shows a fixture
  can assert the *defect itself* and still pass, because a fixture that
  cannot vary the quantity under test (there: `distance_to_boundary`) is not
  a test of it.
- **Build a real OrcaSlicer C++ checkout to generate absolute-numeric golden
  fixtures.** Already considered and declined in ADR-0035 on
  infrastructure-cost grounds (multi-hour CMake+vcpkg+MSVC lift,
  disproportionate to invariant-based testing). This ADR adds an independent,
  standing reason even if that infrastructure existed: PnP's 1-unit=100nm
  coordinate system and the D3-discovered Z-sampling offset mean
  absolute-coordinate equality would be flaky-by-construction against real
  OrcaSlicer output too, not just against an unavailable one.
- **Trust the LLM-visual read as the final word on whether a rendered
  difference is a defect.** Rejected: three false positives in one session
  (layer-96, D1, D3) — every one required a structural gcode/IR check to
  correctly resolve, and two of the three (layer-96, D3) were visually
  indistinguishable from real defects without that check.

## Consequences

- New and existing Arachne test fixtures must be evaluated against a
  structural-invariant checklist before being trusted as parity evidence;
  self-captured fixtures that only assert "output equals last capture" are
  change-detectors, not parity oracles, and should be labeled as such in
  their own doc comments (matching the D-109/D-112 "Honesty note" convention
  already established for the pre-campaign fixtures).
- Track B completed the conversion of the self-captured Arachne oracles to a
  measured source-geometry corpus. The measured coverage threshold is `0.99`
  (pinned, not derived — see ADR-0047),
  with a repeatability margin of `0.000000` (the maximum same-subject,
  same-Z repeated-run delta, below the `0.02` cap). The five coverage subjects
  are `tapered_wedge`, `narrow_strip_widening`, `max_bead_count_cap`,
  `complex_multi_feature`, and `cube_4color_arachne`.
- The conversion deleted 19 self-captured JSON oracles: eight core snapshots
  and eleven `expected_perimeter_ir.json` files. Eight core tests now construct
  their source geometry in memory, and the runtime corpus is exercised by the
  standalone `arachne_structural_invariants` test binary.
- The D5 sanity discriminator is part of the contract: broken coverage `0.668`
  fails, while fixed coverage `0.990` passes. This prevents the coverage floor
  from becoming a vacuous re-recording of the known tapered-wedge failure.
- Any future agent proposing an absolute-numeric OrcaSlicer fixture as an
  automated Arachne parity test must be pointed at this ADR and at
  `docs/08_coordinate_system.md`'s 1-unit=100nm rule.
- Any future agent treating an LLM-visual read as sufficient evidence of a
  defect (without a corresponding gcode/IR structural check) is reintroducing
  exactly the failure mode this ADR documents three instances of.
- `docs/DEVIATION_LOG.md` row `D-109-SELF-CAPTURED-FIXTURES` remains an
  open/accepted-limitation record of the underlying no-OrcaSlicer-binary
  constraint. `D-112-SELFCAPTURED-BASELINES` records the replacement of its 19
  JSON oracles by the measured structural corpus above; its status remains open
  until the packet gate verifies the runtime corpus.

## Future reviewers

- If an OrcaSlicer reference-capture environment ever becomes available in
  this build environment (the blocker cited by D-109/D-112), that changes the
  self-captured-baseline calculus but does **not** by itself revive
  absolute-numeric fixtures as an automated basis — the coordinate-system
  divergence and the Z-sampling-plane offset (D3) are independent obstacles
  that would still need to be resolved (e.g. by comparing in a
  unit-normalized, Z-aligned space) before absolute-numeric equality could be
  anything but flaky.
- The D3-discovered slice-plane Z-convention deviation (~0.25mm offset
  between PnP's and OrcaSlicer's Z-to-mesh-plane mapping) is a real, separate,
  currently-open deviation that should get its own packet and likely its own
  ADR if it turns out to require a design decision (confirm PnP's `slice_z`
  derivation against OrcaSlicer's `Layer::slice_z` first, per the campaign
  doc's Track A notes) — it is out of this ADR's scope, which is about how
  Arachne correctness is *certified*, not about the Z-convention defect
  itself.
- If a future packet finds a structural invariant that itself turns out to be
  too weak (passes on genuinely broken geometry, mirroring
  `propagation_fills_gap_from_central_neighbor`'s failure mode), tighten or
  replace the invariant and record why in that packet — do not fall back to a
  self-captured or absolute-numeric fixture as the fix.
