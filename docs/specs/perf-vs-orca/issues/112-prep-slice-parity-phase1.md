# `PrePass::Slice` parity Phase 1: canonical closing vs the artefact-bound gate

Type: task
Label: wayfinder:task
Status: open
Assignee: (unclaimed)
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [ticket 111](111-prepass-rewrite-scope-ruling.md)'s ruling
(Q3/Q6, 2026-11-03: scoped parity exception, `PrePass::Slice` first, Phase-1
parity analysis before any production edit).

## Question

What does canonical OrcaSlicer's closing pipeline do differently from PnP's
flat-bridge closing, and which of those differences can be expressed as a parity
shape over the per-component **>= 10 %** overlap gate? For each viable shape,
what is its pre-declared wall ceiling and output delta on the eight map cells?

## Context

- [Ticket 59](59-prep-slice-flat-bridge-closing-reduction.md) closed the
  representation-safe family. The closing result is consumed only as the
  per-component **>= 10 %** overlap gate inside
  `assemble_flat_bridge_areas`
  (`crates/slicer-core/src/algos/prepass_slice.rs`); the reported geometry is the
  untouched component. A local-closing candidate flipped gate verdicts on
  **92/239** benchy layers and **320/469** base layers; the far-rejection skip
  family had **0 hits**; the local shape was only **~12 %** cheaper in
  accumulated thread-time.
- [Ticket 104](104-prepass-vs-per-layer-attribution.md) measures
  `PrePass::Slice` at **41,472 ms (51.2 %)** of base classic-off and **44,558 ms
  (19.5 %)** of base classic-on.
- [Ticket 111](111-prepass-rewrite-scope-ruling.md) authorizes a **scoped
  canonical-parity exception** for the prepass and requires this analysis before
  any production edit. The acceptance contract for a later candidate is: fresh
  **16-cell** reference set, arms byte-identical within themselves, baseline
  reproduces the post-t81 set, the candidate-vs-old-reference G-code delta is
  audited before timing, and the standing paired wall/CPU gate at the
  **0.80 %** floor passes in **all 8 cells**.
- Canonical citation rule: cite OrcaSlicer by file + function name, **never by
  line number**. If a local checkout is unavailable, state the evidence gap and
  do not infer canonical behavior from memory.

## Scope

Source-level parity and consumer analysis only: no production edit, no candidate,
no timing campaign, no reference-set generation.

1. Re-derive the consumer/reach map of the closing result at the current HEAD.
   The definitions live in `crates/slicer-core/src/algos/prepass_slice.rs`
   (`assemble_flat_bridge_areas`, `gate_bridge_areas_by_unsupported_span`,
   `update_external_bridge_orientation`) and are called from
   `crates/slicer-runtime/src/slice_postprocess_prepass.rs`; also follow
   `sync_perimeter_infill_areas_into_slice`
   (`crates/slicer-runtime/src/region_partition.rs`), the guest consumers named
   in t59's answer, and the visual-debug SliceIR capture.
2. Pin the canonical difference: which OrcaSlicer functions and which
   configuration (`ORCA_MORPH_MITER_LIMIT`, `morph_passes`,
   `slice_closing_radius`) produce the artifact field PnP's gate sees, and what a
   parity shape would have to reproduce or replace.
3. Design **1–3** parity shapes. For each: the canonical behaviour it tracks, the
   output delta it can cause, the wall ceiling (measured, or bounded from the
   existing profile), the cells it moves, and the risk.
4. Recommend **build** (one shape) or **stop**.

## Deliverable

A resolution with:

- the consumer/reach map at HEAD;
- the canonical-vs-PnP difference, both sides named by file + function;
- the candidate shapes with pre-declared ceilings and delta bounds;
- a recommendation, and if stop, the ceiling against the 0.80 % floor on all 8
  cells and which cells fail.

## Stop rule

If no shape's ceiling clears **0.80 %** on **all 8 cells**, or the output delta
cannot be bounded, record the ceiling and stop: no candidate ticket, no
production edit. A shape that only clears the floor on base classic-off is not
enough for the all-8-cell gate.

## Acceptance

The analysis is a resolution with the evidence attached; no production edit and no
timing campaign. If a shape is recommended, the next ticket is a candidate
implementation under ticket 111's acceptance contract.

## Evidence starts

- [`evidence/t59-flat-bridge-closing/FINDINGS.md`](../evidence/t59-flat-bridge-closing/FINDINGS.md)
  and its `reduced/` set.
- [`evidence/t27-serial-floor/PREMISE-AUDIT.md`](../evidence/t27-serial-floor/PREMISE-AUDIT.md)
  and `FLAT-BRIDGE-EXPERIMENT.md`.
- [`evidence/t104-phase-split/phase-split.json`](../evidence/t104-phase-split/phase-split.json)
  (stage walls and shares).
- `crates/slicer-core/src/algos/prepass_slice.rs`,
  `crates/slicer-core/src/polygon_ops.rs` (`closing_ex`),
  `crates/slicer-runtime/src/slice_postprocess_prepass.rs`,
  `crates/slicer-runtime/src/region_partition.rs`.
- Canonical OrcaSlicer source, if a local checkout is available; otherwise record
  the evidence gap.
