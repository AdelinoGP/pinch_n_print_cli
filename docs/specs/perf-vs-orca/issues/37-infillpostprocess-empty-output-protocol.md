# InfillPostProcess empty-output protocol: clipped-to-nothing must not resurrect raw paths

Type: task
Status: open
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
