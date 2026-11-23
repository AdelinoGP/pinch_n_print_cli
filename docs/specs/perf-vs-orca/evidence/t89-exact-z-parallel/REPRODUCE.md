# Reproduce t89 — parallel exact-Z validation

All commands run from the workspace root. The model fixtures `tmp/base.stl` and
`tmp/3dbenchy.stl` are user-supplied and gitignored. The candidate is the
working-tree change to
`crates/slicer-wasm-host/src/support_aggregation.rs`, preserved as
[`candidate.patch`](candidate.patch); the committed observations are
`proof-rows.json`, `pregate-rows.json`, `probe.json` and the `*.json` reductions
`verify.py` checks checkout-only.

This candidate is **host-only**: both arms execute the same 24 external guests.
The arms differ only in `pnp_cli.exe`, which links `slicer-wasm-host`.

## Preconditions

```bash
cargo xtask build-guests --check                    # exit 0
cargo xtask build-guests --accelerated --check      # exit 0
```

Both gates must be green **before** the snapshots are built.

If the candidate is not in the working tree, apply it first:

```bash
git apply docs/specs/perf-vs-orca/evidence/t89-exact-z-parallel/candidate.patch
```

## Snapshots (two arms, two modes)

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t89-exact-z-parallel/build-snapshots.ps1 -Arm candidate -Modes both
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t89-exact-z-parallel/build-snapshots.ps1 -Arm baseline  -Modes both
```

The baseline arm is the same tree with the support-aggregation source
`git stash`ed (path-limited; never a checkout). The script pops the stash in a
`finally` block. Snapshots land under
`.local-artifacts/perimeter-reference-preparation/t89-exact-z-parallel/snapshots/<mode>-<arm>/`.

## Exactness before timing

```bash
cargo test -p slicer-wasm-host --test contract support_plan_validation
```

`support_plan_aggregation_preserves_serial_entry_and_diagnostic_order`
derives the expected `retained` / `unmet` / `diagnostics` sequence three ways
(fixture verdict table, per-entry aggregation, multi-entry aggregate) and
asserts they agree element for element.

## Proof matrix (16 cells x 2 arms)

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t89-exact-z-parallel/run-all-proof.ps1
python docs/specs/perf-vs-orca/evidence/t89-exact-z-parallel/extract.py
python docs/specs/perf-vs-orca/evidence/t89-exact-z-parallel/reduce.py --raw
```

`run-all-proof.ps1` is resumable (one-shot per cell dir). Ordinary cells are
checked against `references.json`, which `derive_references.py` re-derives from
the committed t81 campaign rows; accelerated cells are gated on arm-to-arm
identity and their agreement with the ordinary cell is reported.

## Pre-gate (3 alternating pairs, quiet machine)

This take did **not** run the four-cell standing campaign: the pre-gate already
fails a clause by more than a factor of two, which is the stop-discipline
condition.

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t89-exact-z-parallel/run-pregate.ps1 -Pairs 3
python docs/specs/perf-vs-orca/evidence/t89-exact-z-parallel/extract.py
python docs/specs/perf-vs-orca/evidence/t89-exact-z-parallel/reduce.py --raw
```

The machine must be otherwise idle: the harness pins `RAYON_NUM_THREADS=12`.
Rows land under `.local-artifacts/.../pregate/rows.csv` and are reduced into
`pregate-rows.json`; every retained pair must be byte-identical.

## Probe (attribution, never a wall claim)

The probe is preserved as [`probe.patch`](probe.patch) and was reverted before
the pre-gate. It adds env-gated counters (`PNP_T89_PROBE=1`) to
`ExactZQueryService::query` and to the aggregation's parallel phase. It applies
on top of `candidate.patch` (the human-approved DROP reverted the candidate), so
apply that first and revert it afterwards:

```bash
git apply docs/specs/perf-vs-orca/evidence/t89-exact-z-parallel/candidate.patch
git apply docs/specs/perf-vs-orca/evidence/t89-exact-z-parallel/probe.patch
cargo build --release --bin pnp_cli
PNP_T89_PROBE=1 RAYON_NUM_THREADS=12 ./target/release/pnp_cli.exe slice \
  --model tmp/base.stl \
  --config docs/specs/perf-vs-orca/evidence/matched-pair/configs/pnp-classic-supports-on.json \
  --output target/t89-split.gcode --module-dir modules/core-modules \
  --no-default-module-paths --no-integrated-modules \
  2> target/t89-split.stderr.log
git apply -R docs/specs/perf-vs-orca/evidence/t89-exact-z-parallel/probe.patch
git apply -R docs/specs/perf-vs-orca/evidence/t89-exact-z-parallel/candidate.patch
python docs/specs/perf-vs-orca/evidence/t89-exact-z-parallel/extract.py
```

`extract.py` reduces the probe's `T89PROBE` / `T89SPLIT` lines into
`probe.json`; the raw stderr stays gitignored.

## Verification

```bash
python docs/specs/perf-vs-orca/evidence/t89-exact-z-parallel/derive_references.py
python docs/specs/perf-vs-orca/evidence/t89-exact-z-parallel/verify.py --self-test
```

`verify.py` exits 0 against the in-repo set alone (no `.local-artifacts`),
re-derives the reference table from the t81 campaign rows, asserts the measured
**gate failure** (wall down, CPU above the ceiling), and rejects its eight
negative controls.
