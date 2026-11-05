# Post-merge per-layer dispatch regression: localize and remove the per-call cost

Type: task
Label: wayfinder:task
Status: open
Assignee: (unclaimed)
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [ticket 115](115-post-merge-reconciliation.md)'s reconciliation
(2026-11-04). The merge `a5e9a403` left the matched job's toolpath unchanged
but made every per-layer module call slower.

## Question

Which code path added by the 2026-11-03 merge charges a per-call cost to every
per-layer module dispatch — and can it be removed without changing output?

## What ticket 115 measured

- Paired pre-merge vs merged A/B, four classic cells, three interleaved pairs,
  **process CPU rises in 12/12 pairs**: benchy classic-off +24.11 %, benchy
  classic-on +16.96 %, base classic-off +53.57 %, base classic-on +34.88 %.
- The cost is entirely in the **per-layer phase**: benchy 1.9x–2.2x, base
  5.1x–6.0x. The prepass is unchanged (70,662 → 70,463 ms instrumented on base
  classic-off; −0.3 %).
- The per-call cost is **independent of guest work**: `com.core.fuzzy-skin` and
  `com.core.seam-placer` go from 162/211 ms to 56,068/56,226 ms accumulated over
  the *same* 495 calls; ten more modules land at a near-uniform ~28.5 s.
- The cost **grows with layer index** (`fuzzy-skin` 55 ms at layer 0 → 141 ms at
  the last layers), which points at a per-call cost proportional to committed
  state rather than at guest execution.
- The **accelerated** dist build does not show it: post-merge benchy classic-off
  instrumented per-layer is 2,318 ms against ordinary 4,807–5,860 ms with the
  same output hash. That build goes through the controlled accelerated compiler
  policy, so the suspect surface is a build-mode-sensitive host path.
- Output is byte-identical apart from one config line; the call count is
  unchanged. Whatever this is, it is pure overhead.

## Suspect surface (from the merge diff, not a diagnosis)

`crates/slicer-runtime/src/layer_executor.rs` (+409/−125),
`crates/slicer-wasm-host/src/dispatch.rs` (+856/−632),
`crates/slicer-wasm-host/src/marshal/native.rs` (+748/−126),
`crates/slicer-wasm-host/src/preparation*.rs`, and
`crates/slicer-runtime/src/preparation_lifecycle.rs` (+1,336) all sit on the
per-layer call path and all arrived with the merge. `variant_chain` identity
clones per region per module and the `CommittedPreparationProjection` path are
the first two shapes to price.

## Phase 1 (required before any candidate)

1. Localize the cost to host or guest. A mixed arm (merged host + pre-merge
   modules) fails at instantiation, so the discriminator must be a temporary
   probe build, not a mixed run.
2. Name the structure whose size grows with layer index, and show the per-call
   cost scaling with it.
3. Price at least three removal shapes against the **0.80 %** floor. The
   available headroom is large: the regression alone is +19…+42 % wall on the
   classic cells, so a shape that clears it is a gap-closer, not a small effect.

## Acceptance (when a candidate exists)

- Standing proof gate: both arms byte-identical within themselves, ordinary
  cells reproduce the **post-merge** reference set recorded in
  [`evidence/t115-post-merge/summary.json`](../evidence/t115-post-merge/summary.json)
  `output_audit`, ordinary ≡ accelerated on all eight cells.
- Standing paired wall/CPU gate on the pre-declared cells at the 0.80 % floor,
  **and** the per-layer phase must return to the pre-merge bracket
  (`evidence/t115-post-merge/summary.json` `instrumented_base_classic_off`).
- A fresh post-merge Orca scoreboard (ticket 115's step 3) is this ticket's
  closing measurement, so the map gets a ratio of record for the merged tree.
- Guest freshness both modes, workspace clippy `--all-targets -D warnings`,
  `check-literals`, `check-test-quality --report`.

## Stop rule

If the cost is intrinsic to the preparation-lifecycle contract (i.e. it buys a
capability the map needs), record the measured price and escalate the
capability/performance trade to the human as a grilling ticket instead of
weakening the contract.
