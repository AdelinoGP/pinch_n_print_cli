# Raft-fill dispatch outside the raft band: decline the no-op per-layer call

Type: task / candidate campaign
Label: wayfinder:task
Status: open (unclaimed)
Assignee: —
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

_To be recorded on resolution._
