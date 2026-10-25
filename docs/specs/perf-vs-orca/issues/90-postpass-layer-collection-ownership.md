# Single-ownership layer collection through finalization and G-code emission

Type: task
Label: wayfinder:task
Status: open (graduated 2026-10-25 from the supports-on increment attribution; unclaimed)
Assignee: none
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Supports-on increment attribution at post-resolution-port HEAD](88-supports-on-increment-attribution.md)

## Question

Can the postpass seams stop cloning and dropping the whole
`Vec<LayerCollectionIR>` — once for finalization, once for G-code emission, and
once at postpass teardown — and does the resulting wall win pass the standing
paired gate?

## Measured premise

The [t88 attribution](../evidence/t88-supports-increment/FINDINGS.md) measured
three whole-print copy/drop seams on base supports-on. They are cheap on
supports-off (the collection is small) and expensive on supports-on (support
entities multiply the allocation count):

| seam | base classic on | base arachne on | source |
| --- | ---: | ---: | --- |
| `PostPass::LayerFinalization` | **6.96 s** (1.84 clone + 0.78 modules + 4.34 old-vector drop) | **6.92 s** (1.98 + 0.84 + 4.10) | stage gaps around the four modules |
| `host:gcode_emit` | **6.34 s** (0.19 walk/estimate/M73; **~6.15 s drop**) | **6.09 s** (0.22; **~5.87 s drop**) | `DefaultGCodeEmitter::emit_gcode`'s cloned collection is dropped before return |
| postpass teardown / reconcile gap | **4.58 s** | **4.21 s** | `postpass` phase wall minus its four stage brackets |

The probe's `SystemTime` marks show the emitter's `layer_irs.to_vec()` clone
itself completes in ~5 ms; the 6 s is the **drop of the cloned collection** at
the end of `emit_gcode`. The same collection is dropped again by the postpass
teardown gap. The first (finalization) clone pays the cold-page cost; the
emitter clone reuses the pages freed by the finalization drop, which is why the
clone is cheap and the drops are not.

The finalization clone is `let mut candidate = layers.clone();` plus
`*layers = candidate;` (`crates/slicer-runtime/src/layer_finalization.rs`); the
source comment there already records that the pre-stage snapshot is only
observable after a mid-stage abort, where production aborts the slice. The
emitter clone exists because `apply_cross_layer_tool_rotation` mutates; on the
measured job that rotation is a no-op (0.026 ms). The postpass then drops the
original `reconciled_layers` at function exit.

## Candidate shape

One ownership pass, not three independent micro-fixes:

1. Move the owned layer vector into `execute_layer_finalization` (or take it
   with `std::mem::take`) and return it, instead of cloning the whole vector for
   a rollback snapshot that production never reads.
2. Let `DefaultGCodeEmitter::emit_gcode` take ownership of the finalized vector
   (a new `emit_gcode_owned` trait method with a default implementation that
   forwards to `emit_gcode(&layers)` keeps every other emitter source
   compatible). Rotate in place only when a layer actually needs rotation; the
   measured job needs none.
3. Drop the collection once, as the emitter consumes it, instead of cloning it
   and dropping both copies. If teardown is still material, drop independent
   layers in parallel.

Any change that alters emitted bytes, tool ordering, travel reconciliation or
the finalization stage's committed output is out of bounds. The mid-stage abort
state may change from "pre-stage snapshot" to "unavailable"; production aborts
the slice in that path, which the existing comment already records as
unobservable.

## Pre-declared floor

**0.80 % of the cell wall**, the map's established screening floor
(t83/t84). The t88 quiet instrumented ceiling is **16.91 s of 184.12 s**
(9.18 %) classic and **16.16 s of 187.97 s** (8.60 %) arachne. Because the
candidate leaves one collection to be dropped once, the **net upper estimate**
is **10.76 s (5.84 %)** classic and **10.29 s (5.47 %)** arachne. The candidate's
own paired campaign must measure its actual wall win; the floor screens the
shape.

## Proof gate

- 16-cell output-identity matrix against the post-t81 references, both modes.
- Standing paired ordinary + accelerated gate on the four base supports-on
  cells (both modes), 6 alternating pairs per cell, 12 threads, zero starvation
  exclusions, every pair byte-identical.
- Explicit negative control: the no-tool-change job must not emit a `T<n>` or
  reorder an entity; a synthetic tool-change fixture must still rotate exactly
  as today.
- `host:gcode_emit`'s `GCodeIR` and the serialized bytes are unchanged.
- Both guest freshness gates exit 0 (host-only candidate).

## Deliverables

1. The ownership refactor with the compatibility-preserving trait method.
2. A test pinning tool rotation and `;TYPE:`/entity order on a multi-tool
   fixture, plus the existing finalization-abort test's updated expectation.
3. The paired campaign results and a measured recommendation to the human.
   No auto-commit.
