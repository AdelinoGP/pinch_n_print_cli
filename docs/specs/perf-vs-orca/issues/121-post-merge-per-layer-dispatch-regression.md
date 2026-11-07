# Post-merge per-layer dispatch regression: localize and remove the per-call cost

Type: task
Label: wayfinder:task
Status: resolved (2026-11-06; the per-layer cost is removed — per-layer 42,406 →
6,758 ms, below the 7,265 ms pre-merge bracket; four-cell paired CPU −27…−46 %
in 12/12 pairs; the complete 8-cell post-merge Orca scoreboard records PnP at
2.78x–5.11x ordinary, 2.76x–5.10x accelerated, with ordinary ≡ accelerated bytes
in 8/8 — see ## Answer)
Assignee: 2026-11-05 OpenCode wayfinder session — the human said "Continue"
  after the t119 resolution; 121 is the first AFK frontier ticket in order
  (117 is the human's grilling)
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

## Answer

**The per-layer transport was deriving prepared-region geometry for every
module call, and for `Layer::PerimetersPostProcess` it was deriving data the
stage never consumes. Both are host-side and output-neutral; removing them
accounts for the whole measured regression.**

### Diagnosis

A temporary `PNP_T121_PROBE` bracket inside the per-module dispatch (debug
build, `tmp/3dbenchy.stl` classic supports-off) split the call into setup /
transport / guest. Transport assembly averaged **51.7 ms** against **11.6 ms**
of guest work, and inside it `prepare_regions` averaged **45.9 ms** (86 % of
the transport, 82 % of the call). The setup maps were 73 µs. The transport is
assembled per module call even though its geometry inputs are layer-level, so
every module re-derived the same prepared regions the arena had already
computed.

The same probe run showed the module attribution: after the first fast path
every module returned to its pre-merge bracket except `com.core.fuzzy-skin`
and `com.core.seam-placer` (55.7 / 55.3 s against 162 / 211 ms pre-merge),
the two `Layer::PerimetersPostProcess` modules. That stage is outside
`prepare_dispatch_region_views`' prepared-region contract, but the transport
still synthesized the prepared view for it per call.

### Fix

`crates/slicer-wasm-host/src/marshal/transport.rs`:

- `reuse_arena_preparation`: reuse the arena's `PreparedRegionData` only when
  the perimeter-source projection is the identity **and** every projected
  origin is staged. A modifier footprint/child or a missing origin takes the
  fresh derivation.
- `assemble_layer_transport_for_stage(…, derive_prepared)`: `false` delivers
  the empty prepared view for a stage outside the prepared-region contract.
  The test-facing `assemble_layer_transport` wrapper keeps `true`.
- `crates/slicer-wasm-host/src/dispatch.rs` passes
  `stage_export != "Layer::PerimetersPostProcess"`.

### Measured

| cell | wall pre → fix (median) | wall Δ | CPU pre → fix (median) | CPU Δ | output |
| --- | --- | ---: | --- | ---: | --- |
| benchy classic-off | 7,073 → 6,062 ms | −14.29 % | 42.84 → 26.89 s | −37.23 % | identical |
| benchy classic-on | 11,081 → 10,056 ms | −9.25 % | 58.73 → 42.66 s | −27.36 % | identical |
| base classic-off | 85,451 → 53,434 ms | −37.47 % | 810.02 → 435.95 s | −46.18 % | identical |
| base classic-on | 164,383 → 126,731 ms | −22.91 % | 1,131.25 → 746.55 s | −34.01 % | identical |

Three interleaved pairs per cell, same release binaries and module tree.
**CPU negative 12/12; wall 11/12.** The exception is benchy-off pair 2 at
**+0.66 %** wall with **−37.23 %** CPU (that pair's pre arm ran 6,031 ms against
7,074/7,073 ms in the other pairs); the cell median is −14.29 %.

**Per-layer bracket (base classic-off, instrumented):** pre-merge **7,265 ms**
→ unfixed merged **42,406 ms** → fixed **6,758 ms**, i.e. below the pre-merge
bracket with identical G-code. The t115 post-merge value (42,886 ms) is
reproduced by this take's unfixed arm within 1.1 %.

### Verification

- `cargo test -p slicer-wasm-host --lib` — 96 passed.
- `cargo test -p slicer-runtime --test contract` — 315 passed (including the
  wave-overhangs native/WASM parity leg that relies on the synthesized view
  for `Layer::Infill`).
- `cargo test -p slicer-runtime --test integration` — 376 passed, 4 ignored.
- `cargo xtask test --summary -p slicer-runtime --test e2e` — 172 passed,
  1 ignored, 695.77 s.
- `cargo clippy --workspace --all-targets -- -D warnings` — clean.
- `cargo xtask check-literals` — 0; `check-test-quality --report` — 0.
- Regression seam: `reuse_decision_requires_identity_and_every_staged_origin`,
  `identity_reuse_matches_the_fresh_preparation`,
  `non_prepared_stage_delivers_the_empty_view`.

### Ledger correction

The acceptance names the **t115 post-merge** output hashes. They predate
**t116**, which moved every cell's output (canonical flat-bridge
classification). The current references are `35600ee4…` (benchy-off),
`596f6da5…` (benchy-on), `6a6a3d9d…` (base-off) and `8f46fe92…` (base-on); the
fix is output-neutral against them. Any post-merge scoreboard must use the t116
set.

### Scoreboard (complete)

The **fresh post-merge Orca scoreboard** (t115 step 3) is recorded on the
post-fix tree: 8 cells × 3 tools × (1 warmup + 3 measured). PnP ordinary and
accelerated G-code bytes are identical in **8/8** cells, and PnP loses every
cell: **ordinary/Orca 2.78x–5.11x**, **accelerated/Orca 2.76x–5.10x** (best
benchy arachne-off 2.78x, worst base classic-on 5.11x). The destination's 8/8 is
not met; the required cut is **64%–80%** of today's wall. Table, raw rows,
reducer output and verifier:
[`evidence/t121-per-layer-dispatch/SCOREBOARD.md`](../evidence/t121-per-layer-dispatch/SCOREBOARD.md).

Evidence:
[`evidence/t121-per-layer-dispatch/`](../evidence/t121-per-layer-dispatch/FINDINGS.md)
— `verify.py` exits 0 checkout-only; `--self-test` rejects 8/8 negative
controls.
