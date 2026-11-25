# Ticket 127 — declining the raft-fill dispatch moves its cost, it does not remove it

Route: [Raft-fill dispatch outside the raft band](../../issues/127-raft-fill-dispatch-outside-band.md).

## Verdict

**DROP.** The candidate is output-identical in all eight cells on both build
arms, and the `com.core.raft-default` dispatch really disappears (495 → 0 on
the base cells, 240 → 0 on benchy). But the 61–63 ms the module was charged on
every ordinary layer is **not its own work**: it is the `Layer::Infill` stage's
shared prepared-region view materialisation, paid by whichever Infill module is
dispatched **first** on that layer. `raft-default` is scheduled first today; in
the candidate `com.core.gyroid-infill` pays the same cost instead (p50
**60 / 58 / 61 / 57 ms** against raft-default's **56 / 60 / 66 / 66 ms** on the
four base cells). The per-layer phase wall does not fall, and the standing
8-cell gate fails.

The candidate source, its two tests and the staged binaries are retired; only
the evidence and the patch remain.

## Candidate shape

A binding whose **only** declared claim is `claim:raft-fill` and whose stage is
`Layer::Infill` or `Layer::Support` is declined on any layer that is not a
physical raft layer:

```rust
if !layer.is_raft && raft_band_only_binding(module) {
    result.invoke = false;
    return Ok(result);
}
```

`build_layer_selection_index` therefore inserts `invoke = false` with no
targets for those layers, and the runtime skips the call before
instrumentation. The gate is exact by the module body's own guard
(`!paint.is_raft() || plan.raft_layers == 0` … `layer_index >= plan.raft_layers`),
and the matched job has `support_raft_layers = 0`, so every layer is outside the
band. The patch is preserved as [`candidate.patch`](candidate.patch); its unit
test proves the decline short-circuits before config resolution and that an
extra claim or a non-raft stage keeps the binding scheduled, and its contract
test derives the stage-coverage invariant from the committed manifests.

## Output and provenance proof

The output campaign contains **32 runs**: eight cells × four arms
(baseline/candidate × ordinary/accelerated). All 32 exited 0 with
`completion_status="ok"`, `degraded=false` and `non_fatal_error_count=0`; every
generator/claim-holder validation passed.

- In every cell, baseline ordinary ≡ baseline accelerated ≡ candidate ordinary ≡
  candidate accelerated, and all four hashes equal the post-t123 reference.
- The ordinary host binaries are `7737a74f…` (baseline) and `bcf7baca…`
  (candidate); the accelerated host binaries are `a6ae2adc…` and `d00d8163…`.
- Every guest module is **identical** between arms: the ordinary
  `raft-default.wasm` is `c0a8a17e…` in all four arm snapshots and the
  accelerated one is `895adfd3…`, and a recursive diff of the module trees
  reports no difference.
- Both freshness checks exited 0 before the campaign. After the source was
  retired, `cargo xtask dist` rebuilt both shipped binaries; each differs from
  its campaign baseline in exactly **28** PE-header bytes (the linker's
  timestamp/debug-GUID stamp), and the candidate binaries differ from the
  baseline in ~23 M bytes, so the measured arms are distinct builds.

## Paired gate

Wall and CPU are medians of three alternating, interleaved ordinary-arm pairs.
Wall improvement is baseline minus candidate, so positive values favour the
candidate. The floor is 0.80 % of each cell's own same-run baseline median wall;
the ticket requires the floor and a CPU decrease in **all eight** cells.

| cell | baseline wall | candidate wall | wall delta | floor multiple | CPU delta | gate |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| benchy classic-off | 4.777 s | 4.763 s | **+0.29 %** | +0.37x | +0.0781 s | **FAIL — below floor** |
| benchy classic-on | 8.909 s | 8.733 s | **+1.97 %** | +2.47x | +0.4531 s | PASS |
| benchy arachne-off | 4.880 s | 4.910 s | **−0.62 %** | −0.77x | +0.1719 s | **FAIL — wall regressed** |
| benchy arachne-on | 8.893 s | 9.299 s | **−4.57 %** | −5.71x | −0.8750 s | **FAIL — wall and CPU regressed** |
| base classic-off | 40.618 s | 40.399 s | **+0.54 %** | +0.67x | +3.3281 s | **FAIL — below floor** |
| base classic-on | 118.061 s | 114.446 s | **+3.06 %** | +3.83x | +3.2032 s | PASS |
| base arachne-off | 42.893 s | 44.055 s | **−2.71 %** | −3.39x | +0.1876 s | **FAIL — wall regressed** |
| base arachne-on | 118.997 s | 118.103 s | **+0.75 %** | +0.94x | −6.4062 s | **FAIL — below floor and CPU regressed** |

Two cells pass, six fail. The verdict is the same under ticket 117's narrower
representation-safe gate (floor on the four supports-on cells, no median wall
or CPU regression on the four supports-off cells): benchy arachne-off and base
arachne-off still regress on wall, so four cells would pass and four would fail.

The instrumented transfer below explains the spread: the true wall effect is
at or below this rig's run-to-run drift (within-cell spread of 2–9 % on the base
cells), so individual cell verdicts move with the ambient load, not with the
candidate.

## Instrumented transfer proof

Both arms were run once per cell in the same session with `--instrument-stderr`;
the streams are reduced by [`analyze_transfer.py`](analyze_transfer.py) into
[`reduced/transfer.json`](reduced/transfer.json). Attribution only: no wall
claim is taken from an instrumented pass.

| cell | first Infill dispatch, baseline | its p50 | first Infill dispatch, candidate | its p50 | per-layer phase |
| --- | --- | ---: | --- | ---: | ---: |
| benchy classic-off | raft-default | 5 ms | gyroid-infill | 4 ms | 1,596 → 1,432 ms |
| benchy classic-on | raft-default | 4 ms | gyroid-infill | 5 ms | 1,574 → 1,602 ms |
| benchy arachne-off | raft-default | 5 ms | gyroid-infill | 5 ms | 1,753 → 1,796 ms |
| benchy arachne-on | raft-default | 5 ms | gyroid-infill | 5 ms | 2,001 → 1,845 ms |
| base classic-off | raft-default | 56 ms | gyroid-infill | 60 ms | 6,727 → 7,495 ms |
| base classic-on | raft-default | 60 ms | gyroid-infill | 58 ms | 12,013 → 11,265 ms |
| base arachne-off | raft-default | 66 ms | gyroid-infill | 61 ms | 8,920 → 8,207 ms |
| base arachne-on | raft-default | 66 ms | gyroid-infill | 57 ms | 13,132 → 11,185 ms |

On every layer of both arms the Infill stage runs its modules serially and the
first dispatch absorbs the stage's preparation; the transfer is exact on the
same layer. Layer 0 of `base classic-off` shows it directly: baseline spends
26 ms in raft-default and 1 ms in gyroid; the candidate spends 0 ms in
raft-default and 25 ms in gyroid.

The per-layer phase walls are **not** evidence in either direction: their
spread across the two instrumented passes (up to ±2 s on the base cells) is
larger than any candidate effect, which is the same conclusion the paired gate
reaches.

## Ledger corrections

- **t126 §6/§7 over-priced the shape.** The 61–63 ms per call is the Infill
  stage's shared prepared-region view materialisation, charged to the first
  module in the stage; it is not a raft-fill-specific cost and declining the
  dispatch does not remove it. The 2.64x–8.25x floor multiples are therefore not
  a saving; the realised wall effect is ≤ ~1 % and below this rig's resolution.
- **t126's 17-layer control does not support the opposite reading.** "Another
  module first pays 0 ms, raft-default still pays ~30 ms" cannot be a
  dispatch-order claim: the same-layer streams here show the first-dispatched
  Infill module always pays the preparation, and when raft-default is declined
  gyroid-infill pays it.
- **The wider lesson for the per-layer/emit program:** per-module brackets
  attribute shared stage preparation to whichever module happens to run first.
  Any candidate priced from a module bracket must first show the cost is
  module-specific, by declining the module and watching the bracket move.

## Disclosures

- **Ordinary arm only** for the gate. The accelerated arm is used only for
  output identity, as the standing protocol does.
- **The first AB batch (`ab1`) is spoiled and preserved, not used.** It was cut
  in two by a 14.7-hour session interruption between pairs 2 and 3, and two of
  its runs stalled visibly (candidate 137.25 s on `base classic-on` against a
  111–120 s level; baseline 48.49 s on `base arachne-off` against 41–45 s). Its
  reducer verdict was also DROP (3/8 pass).
- **The measurement of record (`ab2`) deviates in campaign order.** The six
  runs of each cell were taken back-to-back (`-Cells <cell>` per invocation,
  cell-local pairing) instead of sweeping all cells per pair, so drift stays
  inside a cell rather than between pairs. Each cell still holds three
  interleaved ABBA pairs. Host load sampled **42 %** before the batch and
  **15 %** after it; the base-cell baseline medians move 2–9 % within the batch.
- **`Layer::Support` is not exercised.** No raft-band-only Support binding
  exists in the roster, so the stage-general half of the gate is covered by the
  unit test only.
- **Raft-enabled jobs are not in the matched matrix.** All eight cells resolve
  `support_raft_layers = 0`, so the decline fires on every layer. A job with a
  raft band keeps the in-band dispatch unchanged (unit-tested); no campaign run
  exercises it.
- **No Orca arm and no scoreboard refresh.** Nothing landed, so ticket 124's
  ratios of record stand unchanged.

## Evidence files

- [`candidate.patch`](candidate.patch) — the retired candidate (production
  change plus its two tests).
- [`run_t127.ps1`](run_t127.ps1) — output and paired campaigns.
- [`run_instrumented.py`](run_instrumented.py),
  [`analyze_transfer.py`](analyze_transfer.py),
  [`reduce_instrumented.py`](reduce_instrumented.py) — the dispatch-transfer
  proof.
- [`reduce_t127.py`](reduce_t127.py) — reduces the raw rows into the committed
  set.
- [`reduced/summary.json`](reduced/summary.json),
  [`reduced/instrumented.json`](reduced/instrumented.json),
  [`reduced/transfer.json`](reduced/transfer.json) — the committed observations.
- [`verify.py`](verify.py) — checkout-only verifier; `--self-test` rejects
  **18/18** negative controls.
- [`REPRODUCE.md`](REPRODUCE.md) — the full command sequence.
