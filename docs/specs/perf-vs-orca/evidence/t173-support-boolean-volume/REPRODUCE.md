# t173 reproduce — support-path boolean volume

Attribution-only take: the probe build is temporary and reverted; nothing in the
production tree changes. Ticket:
[`173-support-geometry-boolean-volume.md`](../../issues/173-support-geometry-boolean-volume.md).

## Prerequisites

- `tmp/3dbenchy.stl` (the 2026-11-12 replacement) and `tmp/base.stl`:
  user-supplied, never committed.
- The matched-pair configs at
  `docs/specs/perf-vs-orca/evidence/matched-pair/configs/`.
- The 24 external core modules under `modules/core-modules/`.
- Rust toolchain as pinned by the workspace; a 12-logical-core box;
  `python` (stdlib only).
- Baseline HEAD: `bc487ff4` (tracker commit; last code commit `48a5e583`,
  t172). `probe.patch` applies to that tree.

## 1. Rebuild the probe

```bash
git apply docs/specs/perf-vs-orca/evidence/t173-support-boolean-volume/probe.patch
cargo build --release -p pnp-cli
cargo xtask build-guests          # the slicer-core change makes every guest stale
```

The patch adds `crates/slicer-core/src/t173_probe.rs`, env-gated seams in
`polygon_ops.rs` / `slicer-wasm-host/src/host.rs` / `slicer-runtime`, and the
temporary `t173-probe` feature plus `t173_tag!` call-site tags in
`modules/core-modules/tree-support-planner`. Without `PNP_T173_PROBE=1` every
entry point is one relaxed atomic load; the binary is otherwise the production
binary and reproduces t172's reference set.

## 2. The census and the `--profile` cross-check

```bash
cd docs/specs/perf-vs-orca/evidence/t173-support-boolean-volume
python run_probe_t173.py --tag probe      # 5 cells, one pass each, hash-checked
python reduce_probe_t173.py --tag probe   # -> reduced/census-probe.json
python reduce_profile_t173.py --tag probe # -> reduced/profile-detail-probe.json
```

Writes `target/t173-probe-runs/probe/<cell>.stderr.txt` (gitignored); the
reducer reads the `t173-row` and `t173-batch` lines. Every run asserts its
G-code SHA-256 against t172's active reference set; a mismatch fails the run.

## 3. The same-window decomposition (two base cells)

`--instrument-stderr` and the probe in one run, so the stage bracket and the
batch walls share a window:

```bash
# base-classic-on and base-arachne-on, --instrument-stderr (no --no-progress-events)
# -> target/t173-combined/<cell>.pass1.jsonl and target/t173-probe-runs/combined/<cell>.stderr.txt
python reduce_probe_t173.py --tag combined
python reduce_instrumented_t173.py --raw ../../../../../target/t173-combined \
  --out reduced/instrumented-combined.json
```

These two are corroboration/decoration only; the price basis is §2's quiet
probe window.

## 4. The instrumented split (three passes per cell)

```bash
python run_instrumented_t173.py --passes 3
python reduce_instrumented_t173.py
```

`--instrument-stderr` inflates wall; the script records the wall and the
G-code hash but no wall claim is taken from it. All 15 passes must be clean and
reference-matched.

## 5. Re-derive and verify

```bash
python arithmetic.py        # route, split, census, decomposition, shape prices
python verify.py            # 0 = every quoted number re-derives
python verify.py --self-test # 8/8 negative controls rejected
```

`verify.py` is checkout-only: it reads the committed `reduced/` set plus
`t163` / `t169` / `t172`'s committed artifacts and t172's `reference.json`.

## Regimes, and why they are never mixed

- **Probe thread-time is not wall.** Each row is accumulated across worker
  threads and inflated by the probe's own bookkeeping; the batch rows are the
  only wall numbers and they include the same overhead.
- **Shares are instrumented.** The stage share comes from `--instrument-stderr`
  and is multiplied by t172's uninstrumented paired wall — a composition across
  windows, labelled an estimate.
- **Route is t163 × (1 − t169) × (1 − t172).** It is an estimate, not a ratio
  of record; t161 forbids carrying a required cut across windows as a
  measurement, and t174 is the fresh board.
- **Shape prices are ceilings.** They price the measured operand share and
  assume zero implementation cost; they are not gate results.

## Restore the production tree

```bash
git checkout -- crates modules/core-modules/tree-support-planner
rm -f crates/slicer-core/src/t173_probe.rs
cargo build --release -p pnp-cli
cargo xtask build-guests --check   # must exit 0
```

The reverted binary must reproduce t172's reference set on `base-classic-on`
(`0e13ff7b…`).
