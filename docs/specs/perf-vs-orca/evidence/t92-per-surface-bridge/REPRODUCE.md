# Reproduce t92 — per-surface bridge boolean locality screen

All commands run from the workspace root. The model `tmp/base.stl` is
user-supplied and gitignored. The probe is host-only and env-gated
(`PNP_T92_PROBE=1`); it never changes geometry.

## Preconditions

```bash
cargo xtask build-guests --check                # exit 0
cargo xtask build-guests --accelerated --check  # exit 0
```

Both gates must be green **before** the probe snapshot is built, and again after
the probe is reverted. The probe touches two host-compiled files:

- `crates/slicer-runtime/src/slice_postprocess_prepass.rs` (per-layer terms)
- `crates/slicer-core/src/algos/bridge_over_infill.rs` (anchor-offset split)

`bridge_over_infill.rs` sits in the guest dependency closure, so the freshness
fingerprint goes stale **while the probe is applied** and returns green when it
is reverted. Do not run `cargo xtask dist` while the probe is applied: it would
rebuild the guest WASMs and the take has no need for that. The probe snapshot
reuses HEAD's standing `target/dist/developer/modules`.

## 1. Probe snapshot (HEAD modules + probe host binary)

```bash
git apply docs/specs/perf-vs-orca/evidence/t92-per-surface-bridge/probe.patch
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t92-per-surface-bridge/build-probe.ps1
```

`build-probe.ps1` runs `cargo build -p pnp-cli --release` and stages
`target/release/pnp_cli.exe` plus a copy of the standing 24-module tree under
`.local-artifacts/perimeter-reference-preparation/t92-per-surface-bridge/probe/snapshots/probe/`.

## 2. Screening capture (ordinary base classic supports-off)

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t92-per-surface-bridge/run-probe.ps1
```

One instrumented slice: `--module-dir <probe snapshot>/modules`,
`--no-default-module-paths --no-integrated-modules`, `--instrument-stderr
--profile --profile-verbose`, `RAYON_NUM_THREADS=12`, `PNP_T92_PROBE=1`.
External `GetProcessTimes` wall + process CPU are recorded. The run is gated on
the **post-t81 ordinary reference** for this cell
(`292df50179c4033d6204a0353e038534b4d3a98381126998bb5722a41a571a80`,
21,023,739 bytes, from `t91-exact-z-memo/references.json`): any output drift
aborts the capture. `run-<id>.run.json` records wall, CPU, ratio and SHAs.

## 3. Reduce

```bash
python docs/specs/perf-vs-orca/evidence/t92-per-surface-bridge/extract.py
```

Writes `probe-rows.json` (terms, wall equivalents, 130 layer rows, 14 angle
rows, prepass stage brackets). The committed file is run 2; run 1's raw
summaries are preserved as `probe-summary-run1.txt` (delete the capture's
`.run.json` to reduce a foreign capture with the committed run's constants).

## 4. Verify (checkout-only)

```bash
python docs/specs/perf-vs-orca/evidence/t92-per-surface-bridge/verify.py --self-test
```

Re-derives the screening ceiling and the whole-layer bracket from
`probe-rows.json`, checks output identity against the committed reference set,
reconciles every per-layer span with its own terms, checks the anchor-call
count, and requires the measured-dead verdict (ceiling below the floor,
whole-layer bracket above it). 15/15 negative controls are rejected.

## 5. Revert

```bash
git checkout -- crates/slicer-core/src/algos/bridge_over_infill.rs \
  crates/slicer-runtime/src/slice_postprocess_prepass.rs
cargo xtask build-guests --check                # exit 0
cargo xtask build-guests --accelerated --check  # exit 0
```

The probe-built `target/release/pnp_cli.exe` is then stale; rebuild it only if
you need it. Nothing under `target/dist/` or `target/dist-accelerated/` was
written by this take.
