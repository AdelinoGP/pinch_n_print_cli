# t121 — Post-merge per-layer dispatch regression

Resolved (fix landed and verified; the fresh Orca scoreboard is the remaining
closing measurement). Ticket:
[`issues/121-post-merge-per-layer-dispatch-regression.md`](../../issues/121-post-merge-per-layer-dispatch-regression.md).

## Question

Which code path added by the 2026-11-03 merge charges a per-call cost to every
per-layer module dispatch — and can it be removed without changing output?

## Answer

**The per-layer transport was deriving prepared-region geometry for every module
call, and for the `Layer::PerimetersPostProcess` stage it was deriving data the
stage never consumes.** Both are host-side, output-neutral, and together they
account for the whole measured regression:

- `assemble_layer_transport` recomputed
  `prepare_regions(&perimeter_source_regions(selected_slice), …)` for every
  module call. The arena had already computed the same pure function for the
  staged slice. When the perimeter-source projection is the identity (no
  modifier footprint/child) the projected regions are byte-identical, so the
  arena's `PreparedRegionData` is reused.
- `Layer::PerimetersPostProcess` is outside the arena's prepared-region
  contract (`prepare_dispatch_region_views`), but the transport still
  synthesized the prepared view per call. It now delivers the empty view for
  that stage, matching the contract.

**Measured effect (four classic cells, 3 interleaved pairs, same binaries and
module tree):**

| cell | wall pre → fix (median) | wall Δ | CPU pre → fix (median) | CPU Δ | output SHA-256 |
| --- | --- | ---: | --- | ---: | --- |
| benchy classic-off | 7,073 → 6,062 ms | **−14.29 %** | 42.84 → 26.89 s | **−37.23 %** | `35600ee4…` identical |
| benchy classic-on | 11,081 → 10,056 ms | **−9.25 %** | 58.73 → 42.66 s | **−27.36 %** | `596f6da5…` identical |
| base classic-off | 85,451 → 53,434 ms | **−37.47 %** | 810.02 → 435.95 s | **−46.18 %** | `6a6a3d9d…` identical |
| base classic-on | 164,383 → 126,731 ms | **−22.91 %** | 1,131.25 → 746.55 s | **−34.01 %** | `8f46fe92…` identical |

**CPU is negative in 12/12 pairs; wall in 11/12.** The one wall exception is
benchy-off pair 2 at **+0.66 %** while its CPU is −37.23 %: that pair's pre arm
ran 6,031 ms against 7,074/7,073 ms in the other two pre runs. The cell median
is −14.29 %, so the exception is a scheduling outlier, not a cost.

**Per-layer bracket (base classic-off, `--instrument-stderr`):**

| arm | per-layer phase |
| --- | ---: |
| t115 pre-merge bracket | **7,265 ms** |
| unfixed merged tree | **42,406 ms** |
| fixed | **6,758 ms** |

The fix returns the phase **below** the t115 pre-merge bracket, with the same
G-code (`6a6a3d9d…`). The t115 post-merge value for this phase was 42,886 ms;
this take's unfixed 42,406 ms reproduces it within 1.1 %.

## Diagnosis

The probe was a temporary `Instant` bracket inside the per-module dispatch,
gated by `PNP_T121_PROBE`, run on the debug build (the e2e/dev-loop profile)
with `tmp/3dbenchy.stl` classic supports-off. It printed, per module call:

```
[DEBUG-t121]        setup_us=73   transport_us=51,727  guest_us=11,629
[DEBUG-t121-transport] clones_us=7,207  projected_us=27  prepare_us=45,863  rest_us=43
```

`prepare_us` was **86 % of the transport** and **82 % of the whole per-call
cost**; the setup maps were 73 µs and the guest call 11.6 ms. The transport is
assembled once per module call even though its geometry inputs are layer-level,
so every module paid the derivation. The arena's `prepare_regions` result was
already available and identical for the identity projection.

After the fast path the same probe measured `prepare_us` median **29 µs**
(mean 7,033 µs, skewed by fallback calls) and transport mean **14,133 µs**. The
`Layer::PerimetersPostProcess` half was found by module attribution: after the
first fast path, every module had returned to its pre-merge bracket except
`com.core.fuzzy-skin` and `com.core.seam-placer` (55,721 / 55,310 ms against
162 / 211 ms pre-merge); those are the two `PerimetersPostProcess` modules, and
the stage-aware empty view removed their remaining 26 s/call aggregate.

## Fix

`crates/slicer-wasm-host/src/marshal/transport.rs`:

- `reuse_arena_preparation` is the pure decision: identity projection **and**
  every projected origin already staged. Only then is the arena record reused;
  a modifier footprint/child or a missing origin takes the fresh derivation.
- `assemble_layer_transport_for_stage(…, derive_prepared)` is the stage-aware
  entry point. `derive_prepared = false` delivers the empty prepared view
  without a per-call derivation. The `assemble_layer_transport` wrapper (tests)
  keeps `true`.
- `crates/slicer-wasm-host/src/dispatch.rs` passes
  `stage_export != "Layer::PerimetersPostProcess"`.

The wire values for every stage that consumes prepared data are unchanged, and
the four-cell G-code is byte-identical. The changed stage's wire view becomes
the empty prepared view, which is what `prepare_dispatch_region_views` already
declares for it.

## Verification

- `cargo test -p slicer-wasm-host --lib` — 96 passed.
- `cargo test -p slicer-runtime --test contract` — **315 passed** (includes the
  wave-overhangs native/WASM parity leg that relies on the synthesized view for
  `Layer::Infill`).
- `cargo test -p slicer-runtime --test integration` — **376 passed, 4 ignored**.
- `cargo xtask test --summary -p slicer-runtime --test e2e` — **172 passed,
  1 ignored, 695.77 s** (pre-fix reference: 755 s).
- `cargo clippy --workspace --all-targets -- -D warnings` — clean.
- `cargo xtask check-literals` — 0 violations.
- `cargo xtask check-test-quality --report` — 0 findings.
- `cargo xtask build-guests --check` — exit 0 (via the gated e2e entry point).

## Ledger correction

The ticket's acceptance names the **t115 post-merge** `output_audit` hashes.
Those predate **t116**, which moved every cell's output (canonical flat-bridge
classification). The current references for the four classic cells are the
hashes above; the fix is output-neutral against them (pre arm = fix arm on all
four). Any post-merge scoreboard must use the t116 set, not t115's.

## Disclosures

- Load: the A/B ran on the same box as the t119 take; browser processes were
  active. The pre/fix alternation is the control, and every pair is interned
  within a cell. The one wall crossover is disclosed above.
- The A/B binaries are the same release profile; the only difference is the
  fix. `pre_binary_sha256` / `fix_binary_sha256` are in `summary.json`.
- The A/B was taken with the immediate candidate (identity fast path +
  `PerimetersPostProcess` empty view, before the stage-aware refactor). The
  final stage-aware binary was re-measured on base classic-off: per-layer
  6,758 ms vs 6,770 ms and the same hash, so the matched-job path is unchanged.
- The probe was a temporary `PNP_T121_PROBE` build. The probe lines and the
  probe patch are not committed; only the probe numbers are recorded here.
- The per-layer values are instrumented (`--instrument-stderr`) and released
  builds; they are not wall claims.

## Remaining closing measurement

The **fresh post-merge Orca scoreboard** was paused at **6 of 8 cells** (batch
`t121-post-merge2`, 2026-11-05 ~21:16). The six complete cells measure
**ordinary/Orca 2.78x–4.10x** and **accelerated/Orca 2.76x–4.18x**, with
ordinary ≡ accelerated G-code bytes in every complete cell; `base classic-on`
and `base arachne-on` are missing. This is a partial screen, not a ratio of
record. Resume commands, raw rows and the starvation disclosure:
[`SCOREBOARD-PARTIAL.md`](SCOREBOARD-PARTIAL.md).

Evidence: [`ab-rows-classic.csv`](ab-rows-classic.csv),
[`summary.json`](summary.json), [`verify.py`](verify.py)
(`verify.py --self-test` rejects 8/8 negative controls).
