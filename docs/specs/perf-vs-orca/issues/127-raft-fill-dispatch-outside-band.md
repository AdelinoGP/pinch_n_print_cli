# Raft-fill dispatch outside the raft band: decline the no-op per-layer call

Type: task / candidate campaign
Label: wayfinder:task
Status: resolved (2026-11-08; measured **DROP** — output-identical in all eight
  cells and the raft-default dispatch really disappears (495→0 / 240→0), but the
  cost is the `Layer::Infill` stage's shared prepared-region view
  materialisation, paid by whichever Infill module is dispatched first;
  `gyroid-infill` pays the same p50 in the candidate. Two of eight cells pass the
  gate. See ## Answer)
Assignee: 2026-11-08 OpenCode wayfinder session — the human said "Continue the
  perf vs orca map" (no ticket named); 127 is the first unblocked, unclaimed
  AFK frontier ticket in order (128 is the human's grilling).
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Post-t123 phase split refresh and per-layer/emit attribution](126-post-t123-phase-split-refresh.md)'s
  measured anomaly (2026-11-08). `com.core.raft-default` pays **61–63 ms on
  every one of the 495 layers** of each base cell while its body returns before
  doing any work (`base_raft_layers = 1`, `interface_raft_layers = 0` ⇒
  `raft_layers = 2`). Every call exceeds 10 ms on base; only 6–20 of 240 exceed
  10 ms on benchy. Wall-equivalent ceiling **2.6x–8.25x** the 0.80 % floor.

## Question

Can the raft-fill dispatch be declined on every layer outside the raft band
without changing output, and does the removal clear the standing 0.80 % paired
wall floor with CPU corroboration in all eight cells?

## Shape

The module's own body already proves the call is a no-op outside the band:

```text
let Some(plan) = paint.raft_plan() else { return Ok(()) };
if !paint.is_raft() || plan.raft_layers == 0 { return Ok(()) };
if layer_index >= plan.raft_layers { return Ok(()) };
```

The selection index already receives `layer.is_raft` and already emits a
`Raft` target only inside the band
(`build_layer_selection_index`'s `if layer.is_raft && matches!(module.stage_id, …)`
branch). A raft-fill module today is not gated by that target — only
support-family modules set `invoke = !targets.is_empty()` — so the dispatch runs
on every layer. The candidate makes the raft-fill claim obey the same
target-existence rule (or an equivalent exact gate) so the call is declined
outside the band.

**The dispatch, not the body, is the cost:** `module_start` is emitted
immediately before dispatch and `module_complete` after it returns, so the
61–63 ms is view materialisation / input construction. Declining the dispatch
removes it regardless of which part of the path pays it.

## Invariants the candidate must preserve

1. **Output identity.** Skipping must be byte-identical in all eight cells on
   both arms. The body's own guard is the proof obligation; do not weaken it.
2. **The raft-plan invariant.** `expected_layers == 1 + base_raft_layers +
   interface_raft_layers` is a fatal check today, reached only on raft layers.
   If the decline removes the only path that validates it, validate it once at
   plan/selection-build time (or on the raft layers) so a malformed plan still
   fails closed.
3. **Stage coverage.** `Layer::Infill` must still have an invoked module on
   non-raft layers (the other infill modules cover it); assert it rather than
   assume it.
4. **The net is measured, not assumed.** t126's 17-layer test (another module
   first pays 0 ms, raft-default still pays ~30 ms) says the cost is
   module-specific, but the campaign must prove the wall actually moves — the
   t125 precedent is a byte-identical batch that failed its paired gate.

## Acceptance

- **16-cell output:** baseline ordinary ≡ baseline accelerated ≡ the t123
  reference set, and candidate ordinary ≡ candidate accelerated, in all eight
  cells; every run `degraded=false`, `fatal_error_count=0`,
  `non_fatal_error_count=0`; both guest-freshness checks exit 0.
- **Standing paired gate:** 3 interleaved pairs per cell, 0.80 % of each cell's
  own baseline median wall with a CPU decrease, in **all eight** cells; zero
  starvation exclusions; any spoiled cell re-taken whole and disclosed.
- **Ledger:** report the per-cell wall/CPU deltas, the floor multiples, and the
  measured call-count drop in the instrumented stream (495 → ~2 on base).
- Evidence under `evidence/t127-raft-band-dispatch/` with a checkout-only
  `verify.py` and negative controls; a KEEP/DROP ruling on the human's side as
  the standing protocol requires.

## Evidence starts

- [`evidence/t126-post-t123-phase-split/FINDINGS.md`](../evidence/t126-post-t123-phase-split/FINDINGS.md)
  §6 — the anomaly and its per-call distribution.
- [`evidence/t126-post-t123-phase-split/summary.json`](../evidence/t126-post-t123-phase-split/summary.json)
  — `module_call_stats.per_layer["com.core.raft-default"]` in every cell.
- `crates/slicer-wasm-host/src/selection.rs` — the `Raft` target branch and the
  `invoke` rules.
- `modules/core-modules/raft-default/src/lib.rs` — the body guard and the
  `expected_layers` invariant.
- [`evidence/t125-occupancy-offset-batching/FINDINGS.md`](../evidence/t125-occupancy-offset-batching/FINDINGS.md)
  — the precedent for a byte-identical candidate that failed its gate.

## Answer

**DROP: the decline removes the call but not the cost.** The cost the ticket
priced is the `Layer::Infill` stage's **shared prepared-region view
materialisation**, paid by whichever Infill module is dispatched first on a
layer. `com.core.raft-default` happens to be scheduled first today. With
raft-default declined, `com.core.gyroid-infill` pays the same per-layer cost
(p50 **60 / 58 / 61 / 57 ms** on the four base cells against raft-default's
**56 / 60 / 66 / 66 ms**), on the same layers. The dispatch really does
disappear — **495 → 0** calls on each base cell, **240 → 0** on each benchy
cell — and the output is byte-identical in **8/8** cells on both build arms, but
the per-layer phase wall does not fall.

The standing 8-cell gate fails: **2 of 8 cells pass** (benchy classic-on
+1.97 %, base classic-on +3.06 %); the other six are below the 0.80 % floor or
regress on wall, and base arachne-on also regresses on CPU. The verdict is the
same under ticket 117's narrower representation-safe gate (four pass, four fail
on the two arachne-off wall regressions).

Evidence: [`evidence/t127-raft-band-dispatch/`](../evidence/t127-raft-band-dispatch/FINDINGS.md).
The output campaign is 32 runs, all four arms equal to the post-t123 reference
set with identical guest modules; the measurement of record is a cell-local
three-interleaved-pair ABBA batch, with the first, interrupted batch disclosed
and preserved. `verify.py` re-derives 528 checks checkout-only and `--self-test`
rejects 18/18 negative controls.

**Ledger corrections.** t126's 61–63 ms is stage-shared preparation, not a
raft-fill-specific cost, so its 2.64x–8.25x "ceiling" is not a saving; the
realised wall effect is ≤ ~1 % and below this rig's resolution (2–9 % within-cell
drift on the base cells). The matched job actually resolves **no raft band at
all** (`support_raft_layers = 0`): the candidate declines all 495 calls, and the
two in-band calls t126's premise implied do not exist, which the probe confirms
directly. t126's 17-layer "other module first pays 0 ms" control
cannot be a dispatch-order claim: the same-layer streams show the first
dispatched Infill module always pays the preparation. Any future per-layer
shape priced from a module bracket must first show the cost is module-specific
by declining the module and watching the bracket move.

The candidate source, its two tests and both staged binaries are retired; only
[`candidate.patch`](../evidence/t127-raft-band-dispatch/candidate.patch) remains.
No Orca arm was run and no ratio was re-quoted: nothing landed, so
[ticket 124](124-post-t123-orca-scoreboard.md)'s scoreboard stands.
