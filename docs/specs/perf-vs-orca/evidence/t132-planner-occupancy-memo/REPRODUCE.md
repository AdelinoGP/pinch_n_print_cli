# t132 reproduce

Candidate: a per-layer, content-keyed memo serving the planner's duplicate
`+support_object_xy_distance` occupancy offsets (`inflate_model_occupancy`).
Attribution-only probe counters were reverted before the campaign; the
production tree holds only the memo and its unit test.

## Prerequisites

- `tmp/3dbenchy.stl` and `tmp/base.stl` (user-supplied, never committed).
- The matched-pair configs at
  `docs/specs/perf-vs-orca/evidence/matched-pair/configs/`.
- The 24 external core modules under `modules/core-modules/`.
- Rust toolchain as pinned by the workspace; run from the repository root.
- `pwsh` for the campaign driver.

## 1. Build the baseline arms (before editing the planner)

At the take's HEAD (`587cf86f`), with a clean tree:

```bash
cargo xtask build-guests --check             # exit 0
cargo build --release -p pnp-cli             # ordinary host binary
cargo xtask dist --accelerated --edition developer

mkdir -p target/t132-baseline
cp target/release/pnp_cli.exe target/t132-baseline/pnp_cli-ordinary.exe
cp -r modules/core-modules target/t132-baseline/modules
cp -r target/dist-accelerated/developer target/t132-baseline/accelerated
```

## 2. Build the candidate arms

Apply the ticket's planner change (the memo next to `inflate_model_occupancy`
and its two call sites), then:

```bash
cargo test -p tree-support-planner            # memo unit test + the module suite
cargo xtask build-guests                      # rebuilds the planner guest only
cargo build --release -p pnp-cli
cargo xtask dist --accelerated --edition developer

mkdir -p target/t132-candidate
cp target/release/pnp_cli.exe target/t132-candidate/pnp_cli-ordinary.exe
cp -r modules/core-modules target/t132-candidate/modules
cp -r target/dist-accelerated/developer target/t132-candidate/accelerated
```

The host binary is identical between the arms; only the planner guest differs.
Record the arm hashes:

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t132-planner-occupancy-memo/run_t132.ps1 -Mode manifest
```

## 3. Probe census (temporary, reverted)

The hit-rate census is recorded from a probe arm that increments two counters
inside `inflate_model_occupancy_memoized` and logs
`t132-occupancy-memo: calls=<n> hits=<n>` at `host::log_trace` once per planner
invocation. Build the probe guest, then run one supports-on classic cell per
fixture with trace logging on:

```bash
RUST_LOG=trace RAYON_NUM_THREADS=12 ./target/release/pnp_cli.exe slice \
  --model tmp/base.stl --output target/t132-probe/base-classic-on.gcode \
  --config docs/specs/perf-vs-orca/evidence/matched-pair/configs/pnp-classic-supports-on.json \
  --module-dir modules/core-modules --no-default-module-paths \
  2> target/t132-probe/base-on.stderr.log
grep -o 't132-occupancy-memo: calls=[0-9]* hits=[0-9]*' target/t132-probe/base-on.stderr.log
```

Both probe runs reproduce the reference G-code hash, so the counters change no
geometry. Revert the counters and rebuild the guest before the campaign.

## 4. Run the campaign

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t132-planner-occupancy-memo/run_t132.ps1 -Mode output -BatchId output
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t132-planner-occupancy-memo/run_t132.ps1 -Mode ab -Pairs 3 -BatchId ab
```

`-Mode output` writes `target/t132/output/rows.csv` (32 rows: 8 cells × 4 arms);
`-Mode ab` writes `target/t132/ab/rows.csv` (48 rows: 3 interleaved pairs ×
8 cells × 2 ordinary arms). G-code lands in the matching `gcode/` directory.
Every arm carries its own module tree, so a guest change cannot leak between
the arms through a shared directory.

The candidate is exact by construction, so no delta audit is needed beyond the
hash equality: a geometry change would move the hash, and every run must
reproduce ticket 123's candidate-ordinary set byte for byte.

## 5. Reduce and verify

```bash
cd docs/specs/perf-vs-orca/evidence/t132-planner-occupancy-memo
python reduce_t132.py            # target/t132 -> reduced/summary.json
python verify.py                # every claim re-derived from summary.json
python verify.py --self-test    # negative controls must be rejected
```

`verify.py` needs only `reduced/summary.json`; the raw tree is optional.

## 6. Restore the production tree

Nothing to restore: the memo is the take's production change. The probe
counters and the probe modules are not committed. Both freshness gates must
still exit 0:

```bash
cargo xtask build-guests --check
cargo xtask build-guests --accelerated --check
```
