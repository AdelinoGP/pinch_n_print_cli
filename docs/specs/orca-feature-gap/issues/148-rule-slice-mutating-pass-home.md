# 148 — Rule the home of the slice-mutating passes

Type: grilling
Status: open
Assignee: —
Blocked by: —
Map: ../map.md

## Question

Graduated from the map's **Not yet specified** patch filed by ticket 94 and sharpened by
ticket 95. Canonical runs four passes over the same layer polygons, in this order, inside
or immediately after `PrintObject::slice_volumes` / `PrintObject::slice`
(`PrintObjectSlice.cpp`, `PrintObject.cpp`):

conical overhang → XY size compensation → elephant foot → polyhole

Three of the four now have draft packets, and they do **not** agree on a seam:

| pass | packet | seam as authored |
| --- | --- | --- |
| conical overhang | 297 | host prepass built-in, between `PrePass::Slice` and `PrePass::OverhangAnnotation` |
| XY size compensation | 305 (ticket 95) | host prepass built-in, new host-only stage `PrePass::XySizeCompensation` |
| elephant foot | 303 (ticket 93) | **WASM module on `Layer::SlicePostProcess`** |
| polyhole | 304 (ticket 94) | host prepass built-in, new host-only stage `PrePass::PolyholeTransform` |

**Decide whether packet 303's elephant-foot belongs on `Layer::SlicePostProcess` at all,
or whether it should move to a prepass built-in beside the other three.** All four packets
are `draft` and unimplemented, so this is settleable now at near-zero cost and gets more
expensive the moment any of them merges.

### Three findings the ruling has to answer to

1. **Canonical's order is inverted for polyhole** (ticket 94). `Layer::SlicePostProcess`
   runs after every `PrePass::*` stage, so 303-then-304 becomes 304-then-303. Not cosmetic:
   canonical EFC resamples and offsets **holes**, and canonical's grouping carries a
   lone-first-layer rescue whose stated reason is "cause of first layer compensation" — the
   rescue exists *because* EFC ran first. Observable only when both features are enabled
   (both default off) and only on the first `elefant_foot_compensation_layers` layers.
2. **`Layer::SlicePostProcess` can host at most one coarse `SliceIR` mutator** (ticket 95,
   the new finding). The per-stage DAG (`crates/slicer-scheduler/src/dag.rs`) emits an
   `EdgeReason::IrWriteRead` edge whenever `reader.ir_reads()` **exactly** contains the
   writer's write path. Two modules that each declare `reads = ["SliceIR"]` /
   `writes = ["SliceIR"]` therefore produce edges in both directions, and `validate_cycles`
   → `topological_sort` (`crates/slicer-scheduler/src/validation.rs`) fails with
   `SchedulerError::CyclicDependency`. No pair in the tree does this today: `seam-placer`
   escapes it beside `fuzzy-skin` on `Layer::PerimetersPostProcess` only by declaring narrow
   writes. So the stage is not merely mis-ordered for this class of pass — it is not
   **extensible** for it. Whatever lands there second must either narrow its access mask
   (which is a real ordering contract, not a workaround to reach for casually) or go
   somewhere else.
3. **Prepass consumers see whatever the seam leaves them** (ticket 95). `PrePass::`
   `OverhangAnnotation`, `ShellClassification`, `SupportAnalysis`, `SupportGeometry` and
   `LightningTreeGen` all read the committed `SliceIR` and all run **before**
   `Layer::SlicePostProcess`. Canonical's shell classification and support generation see
   *compensated* slices. Packet 303's own deviation (a) already records that its seam is
   later than canonical's; ticket 95 declined that seam partly for this reason, and it
   applies to elephant-foot too, just at a smaller magnitude and only on the bottom layers.

### What a resolution looks like

Either:

- **"Move EFC to a prepass built-in"** — packet 303 changes its seam (kernel unaffected;
  it is already a pure function of the layer's own footprint), packet 304 changes the
  position of one `run_builtin_stage` call, and canonical's order is restored end to end. It
  also settles what the module seam for slice mutation is *for*, given nothing production
  occupies it.
- **"Accept the inversion"** — nothing changes today, and the ruling records that
  `Layer::SlicePostProcess` carries at most one coarse `SliceIR` mutator, so the next pass
  wanting that seam must narrow its access mask or be re-homed. Say so explicitly, because
  finding 2 means the constraint will be hit again.

Whichever way it goes, the answer should say **what the `Layer::SlicePostProcess` module
seam is for**, since packet 303 would be its first production occupant and ticket 95 has
now declined it once.

Resolved when the human rules, the affected packets are updated (or explicitly left as-is),
and the map's Notes record the standing rule for the next slice-mutating pass.
