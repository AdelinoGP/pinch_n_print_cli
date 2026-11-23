# Single-ownership layer collection through finalization and G-code emission

Type: task
Label: wayfinder:task
Status: resolved (2026-10-27; human-approved KEEP — gate met in all four
pre-declared base supports-on cells: wall −5.54 / −7.56 / −6.54 / −7.64 %,
process CPU −1.13 … −1.67 %, 24/24 pairs byte-identical, zero exclusions;
candidate committed `7b22fccc` — see ## Answer)
Assignee: 2026-10-27 OpenCode wayfinder session
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

## Answer

**Resolved 2026-10-27, human-approved KEEP, candidate committed `7b22fccc`.
Gate met in all four pre-declared cells** — wall falls, process CPU **flat or
lower**, every retained pair byte-identical, one SHA per arm, zero exclusions:

| cell | baseline wall | candidate wall | paired wall | paired CPU | pairs |
| --- | ---: | ---: | ---: | ---: | ---: |
| ordinary base classic-on | 187.41 s | 175.44 s | **−5.54 %** | **−1.21 %** | 6/6 |
| ordinary base arachne-on | 190.10 s | 175.29 s | **−7.56 %** | **−1.35 %** | 6/6 |
| accelerated base classic-on | 203.47 s | 187.55 s | **−6.54 %** | **−1.13 %** | 6/6 |
| accelerated base arachne-on | 193.93 s | 178.56 s | **−7.64 %** | **−1.67 %** | 6/6 |

Paired wall deltas span **−1.56 … −13.20 %**: all 24 pairs moved the right way,
every pair is byte-identical, each arm produced exactly one G-code SHA, and no
pair was excluded for starvation. Measured wall deltas are
**12.0 / 14.8 / 15.9 / 15.4 s** — larger than this ticket's pre-declared net
ceiling of **10.76 s (classic) / 10.29 s (arachne)**, because the one surviving
drop costs less than the emitter's cloned drop did (below).

**The candidate is one ownership pass**, not three independent fixes:

1. `execute_layer_finalization_owned` (`crates/slicer-runtime/src/layer_finalization.rs`)
   moves the caller's `Vec<LayerCollectionIR>` into the module loop and returns
   the committed collection; the borrowing entry points are a pointer-swap
   wrapper. The rollback snapshot no production caller read is gone, and a
   fatal module/validation error now consumes the working collection instead of
   leaving the pre-stage snapshot — the semantic delta this ticket
   pre-authorized, pinned by
   `finalization_fatal_error_is_instrumented_and_abort_consumes_the_working_collection`.
2. `GCodeEmitter::emit_gcode_owned` (`crates/slicer-gcode/src/emit.rs`) has a
   default implementation that forwards to `emit_gcode`, so every other emitter
   source is unaffected; `DefaultGCodeEmitter` overrides it to rotate in place
   on the caller's vector and emit from it. `emit_gcode` now forwards to it with
   `to_vec()`, so the two entry points share one emission path.
3. `execute_postpass_with_owned_capture` hands the collection to
   `emit_gcode_owned` on the production path, so the collection is dropped
   **once**, inside the `host:gcode_emit` bracket, instead of being cloned there
   and dropped again at postpass teardown. The visual-debug capture sinks keep
   the borrowed path and receive the collection **by move** (no second clone).

**Attribution (instrumented, ordinary mode, ms — attribution only, never a wall
claim):** the candidate deletes the three whole-print operations t88 had named,
and the emitter bracket shrinks:

| term | classic base | classic cand | arachne base | arachne cand |
| --- | ---: | ---: | ---: | ---: |
| finalization before first module (clone) | 1,765 | **0** | 1,684 | **1** |
| finalization after last module (commit drop) | 3,685 | **0** | 3,758 | **0** |
| `host:gcode_emit` bracket (walk + one drop) | 5,179 | 3,616 | 5,642 | 2,961 |
| postpass teardown residual (phase − brackets) | 4,490 | **2** | 3,723 | **3** |
| postpass phase wall | 17,228 | 5,774 | 17,161 | 5,091 |

The postpass phase falls **66.5 / 70.3 %**; accelerated-mode captures agree
(finalization 6,751 → 820 ms classic, 6,687 → 854 ms arachne; residual
3,740 → 3 ms / 3,693 → 2 ms; emitter 5,398 → 3,578 / 5,354 → 3,572 ms). The
remaining single drop is **1.56 s (classic) / 2.68 s (arachne)** cheaper than
the baseline's clone-then-drop pair because no fresh whole-print copy is made
before it — the reason the measured win beats the ticket's net estimate.

**Exactness:** 16/16 proof cells byte-identical between arms, ordinary cells
equal the derived post-t81 reference set, ordinary ≡ accelerated **8/8**, all
runs clean and non-vacuous; the new `gcode_emit_owned_parity_tdd` (6 tests) pins
owned-vs-borrowed `GCodeIR`/text equality, the rotation fixture's exact entity
order, its `;TYPE:` sequence, the layer 1 "inherited tool prints before any tool
change" ordering, the single-tool negative control (no `T<n>`, authored order),
the error path and the metadata layer count. Both guest freshness gates exit 0;
the candidate is host-only — `provenance.json` records byte-identical 24-module
trees per mode and distinct `pnp_cli.exe` SHAs per arm, and every measure row
carries its arm's `exe_sha256`.

**Pre-existing reds, not this candidate's:** 23 `--test contract` failures
(measured identical with and without the candidate), 3 `--test executor` and 50
`--test integration` (diagnosed by error signature: the post-t81 mandatory
`resolution` key and one CONFIG_BLOCK expectation predating it). Disclosed in
`REPRODUCE.md`.

**Not done, by design:** the ticket's optional parallel drop. Teardown is no
longer material (2–3 ms), so the condition in candidate shape 3 is not met; the
remaining drop sits inside the emitter bracket and is recorded as map fog, not
smuggled into this take. Evidence:
[`evidence/t90-postpass-ownership/`](../evidence/t90-postpass-ownership/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only, 11/11 negative controls
rejected).
