# InfillPostProcess empty-output protocol: clipped-to-nothing must not resurrect raw paths

Type: task
Status: resolved
Assignee: wayfinder session (ses_f108b7d42ffeVYkBrLhguEXSFO), 2026-09-29
Blocked by: 35

## Question

[Benchy classic sparse-fill domain at middle layers](35-benchy-classic-sparse-fill-domain.md)
found that the `Layer::InfillPostProcess` stage's empty-output protocol turns the
infill linker's *correct* clipping verdict into a *print of the raw geometry it
rejected*.

Chain: `GyroidInfill::fill_expolygon` emits raw waves over a bbox expanded by
`10 × spacing_mm` (by design — the linker owns clipping). The linker clips them
per role and drops what survives under `0.8 × spacing_mm`; on the affected layers
nothing survives. `deconstruct_layer_ctx`
(`crates/slicer-wasm-host/src/dispatch.rs`) then returns `Ok(None)` because every
bucket is empty, and `layer_executor.rs`'s apply treats `None` as "the invocation
committed nothing", preserving the prior `InfillIR` — the raw envelope.

Measured impact on matched Benchy/classic/supports-off: 60 layers, 54,324 of
79,940 printed sparse mm (68.0%), 3,054,585 of 4,486,477 sparse bytes; at layer
104, 88.7% of the revived 1,741 mm path lies outside the part cross-section, and
the length is 39× the absolute upper bound set by the layer's 17.7 mm² sparse
claim at 100% density. Arachne: 16 layers, 20.9%.

Work:

- Decide the protocol shape. The candidate recorded in the evidence is "the
  linker always commits the (possibly empty) replacement set rather than
  signalling absence", but it must not break ADR-0028 §Amendment Change 3's
  preservation rule for genuinely absent modules ("a stage with zero registered
  modules never produces a commit — the prior `InfillIR` is preserved"). Any
  shape has to make "clipped to nothing" distinguishable from "did not run" at
  the stage boundary, and must say what the other `Layer::InfillPostProcess`
  consumers (current and future) inherit.
- Check the blast radius across the other stages that share the empty-output
  convention (`Layer::Infill`, `Layer::Support`, `Layer::SupportPostProcess`,
  `Layer::AnchoredEvents`): the finding is about the linker, but the convention
  is generic. Decide whether the fix is linker-local, stage-local, or a change to
  the shared convention, and record which.
- Preserve the ADR-0025 containment contract: a known-empty role boundary is
  `Some(empty)` and clips away; `None` (no boundary resolvable) still passes
  through. Do not re-collapse the two.
- Then measure under the map's standing protocol: paired ordinary + accelerated
  A/B, interleaved repeats, starvation exclusion, uninstrumented probe-free runs,
  isolated snapshots, `--module-dir modules/core-modules`, output disclosure per
  the fairness contract. Keep/drop returns to the human; no auto-commit.

This is a containment defect, not a parity question: PNP prints paths its own
linker rejected, outside the part. It also adds real work to the classic cells,
so a measured win is route-relevant. Do not weaken canonical parity to reduce
bytes.

Evidence: [`evidence/t35-fill-domain/FINDINGS.md`](../evidence/t35-fill-domain/FINDINGS.md),
`linker-vs-printed.csv`, `probe.patch`.

## Answer

**Fixed and measured 2026-09-29. The protocol is stage-local to
`Layer::InfillPostProcess`, and a ran invocation that re-emits nothing now
commits the empty replacement set instead of `Ok(None)`.**

**Shape.** The stage's contract is replace-with-complete-re-emit (ADR-0028
§Amendment Change 3), so zero paths is a verdict, not an absence. The two are
now separated *at the producer*, in all three places that build the commit
(`deconstruct_layer_ctx`, its native twin `commit_native_layer_response`, and
the test-leg mirror in `commit_hec_for_test`), sharing one constructor
`empty_infill_replacement` (`crates/slicer-wasm-host/src/marshal/out.rs`).
Absence still means no commit, and is still reached by every path that never
enters the commit producer: zero registered modules, a region-split skip, a
fatal missing component.

**Blast radius: stage-local, not a change to the shared convention.** The
distinguishing rule is "does the stage's contract make its output a complete
replacement set", and only `Layer::InfillPostProcess` does. `Layer::Infill`
keeps `Ok(None)` (merge stage: empty output is genuinely no contribution);
`Layer::Support` and `Layer::AnchoredEvents` are unaffected;
`Layer::SupportPostProcess` is deliberately unchanged because its shipped
consumer (`support-surface-ironing`) emits only ironing paths and the arm
replaces wholesale — forcing an empty commit there would erase the layer's
support whenever ironing is disabled, which is the default. Full table in
`evidence/t37-empty-commit-protocol/FINDINGS.md` §1.

**ADR-0025 containment contract preserved.** `RoleBoundaries::for_role`'s
`Some(empty)` (clip away) vs `None` (pass through) distinction is untouched —
`git diff --stat -- modules/` is empty for this ticket.

**Functional verification (real G-code, ordinary mode, matched
Benchy/classic/supports-off; guests fresh; `status ok`, `degraded=false`, zero
fatal and non-fatal).** Printed sparse output on the 60 empty-linker layers goes
to **zero** (layer 104: 2,562 segments / 1,741.2 mm → 0 / 0.0), every other layer
is byte-identical, and the worst sparse overshoot past the layer's own wall bbox
drops from **21.78 mm to 0.00 mm** (`verify_containment.py`). Classic totals:
106,442 → 34,307 segments and 79,939.9 → 25,616.2 mm. Arachne behaves the same on
its 16 layers (47,659 → 37,606 segments). Negative control: base.stl is
byte-identical (225,004 segments, 164,905 mm, 0 differing layers), matching
ticket 28's finding that base has no all-empty linker layers.

**Paired ordinary + accelerated A/B (6 repeats/arm/cell, 12 threads, quiet
machine, no starvation exclusions; final candidate `d5d3275d…` ordinary /
`490f5ff5…` accelerated).** Paired median wall deltas: classic-off ordinary
−0.17 s, classic-off accelerated −0.13 s, arachne-off ordinary −0.03 s,
arachne-off accelerated −0.01 s. The one directional signal (classic-off
ordinary, 6/6 pairs) is ~0.8% of the cell's wall and its CPU is **flat**
(+0.04 s, 3/6), so under the map's verdict metric (median uninstrumented wall,
process CPU corroborating) **no wall win is measured in any cell**. A first
repeat on an earlier candidate that predates one predicate alignment measured
the same flat picture (−0.13/−0.01/+0.08/+0.25 s); both row sets are kept.

**Recommendation to the human: KEEP.** The case is correctness, not speed. The
paired evidence, hashes, disclosure table and raw rows are in
`evidence/t37-empty-commit-protocol/` (`FINDINGS.md`, `ab/*.csv`, per-layer
CSVs, `run_ab.ps1`, `verify_containment.py`); the protocol is recorded in
ADR-0028 §Amendment 2026-09-29, `docs/02_ir_schemas.md` (IR 8), the
`run_infill_postprocess` trait doc, and deviation row DEV-196. No auto-commit.
