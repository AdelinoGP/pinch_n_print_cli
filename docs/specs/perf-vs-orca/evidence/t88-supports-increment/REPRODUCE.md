# Reproduce — t88 supports-on increment attribution

Route: [ticket 88](../../issues/88-supports-on-increment-attribution.md). All
commands run from the workspace root on Windows (PowerShell 7 for the harness).
The raw capture tree is gitignored under `target/t88-supports-increment/`; the
committed `captures/*.json` are its reduced form (`extract.py`).

## 1. Preconditions

```powershell
git rev-parse HEAD                      # recorded as captured_commit in inputs.json
cargo xtask build-guests --check        # must exit 0
cargo build --release --bin pnp_cli     # the wall-capture binary
```

The four wall captures use `target/release/pnp_cli.exe` with
`--module-dir modules/core-modules --no-default-module-paths
--no-integrated-modules --instrument-stderr`, ordinary mode, 12 rayon threads.

```powershell
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t88-supports-increment/run_capture.ps1 `
    -Cell base-classic-off -BatchId t88-wall-base-classic-off
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t88-supports-increment/run_capture.ps1 `
    -Cell base-classic-on  -BatchId t88-wall-base-classic-on
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t88-supports-increment/run_capture.ps1 `
    -Cell base-arachne-off -BatchId t88-wall-base-arachne-off
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t88-supports-increment/run_capture.ps1 `
    -Cell base-arachne-on  -BatchId t88-wall-base-arachne-on
```

Each batch writes `rows.csv`, `inputs.json`, `diagnose.json`, the G-code and
the instrumented JSONL. The harness validates the output with
`resources/perimeter-acceptance/validate_measurement.ps1`, checks the two
non-vacuous roles, and records the process wall/CPU with `GetProcessTimes`.

## 2. Probe build

The probe is env-gated (`PNP_T88_PROBE=1`) and preserved as
[`probe.patch`](probe.patch). It adds stopwatch terms only; it changes no
output.

```powershell
git apply docs/specs/perf-vs-orca/evidence/t88-supports-increment/probe.patch
cargo build --release --bin pnp_cli
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t88-supports-increment/run_capture.ps1 `
    -Cell base-classic-on  -Label probe -Probe -BatchId t88-probe-base-classic-on
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t88-supports-increment/run_capture.ps1 `
    -Cell base-classic-off -Label probe -Probe -BatchId t88-probe-base-classic-off
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t88-supports-increment/run_capture.ps1 `
    -Cell base-arachne-on  -Label probe -Probe -BatchId t88-probe-base-arachne-on
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t88-supports-increment/run_capture.ps1 `
    -Cell base-arachne-off -Label probe -Probe -BatchId t88-probe-base-arachne-off
git apply -R docs/specs/perf-vs-orca/evidence/t88-supports-increment/probe.patch
```

Every probe run's output SHA is checked against the post-t81 reference set
(`base classic-off 292df501…`, `classic-on 22200671…`, `arachne-off 01a1434c…`,
`arachne-on 0b29e1bf…`).

**Primary quiet set.** The four primary captures in `captures/` were re-run
when the machine went quiet (5 % load) with `t88b-` batch IDs for classic
off/on, arachne off and the branch arm; the quiet arachne-on run is
`t88-probe-base-arachne-on` (there was no earlier arachne-on probe). The
`t88-` probe captures of classic off/on and the `t88-wall-*` captures are the
load-affected corroborating pass. `extract.py` keeps both, keyed by batch;
`reduce.py` and `verify.py` prefer the `t88b` batch and fall back to `t88`.

## 3. `tree_support_branch_distance` measurement arm

Config-only: a copy of `pnp-classic-supports-on.json` with
`"tree_support_branch_distance": 5.0` added, under the gitignored
`target/t88-supports-increment/configs/`. The arm is a measurement, not a port;
its output intentionally differs from the 1 mm reference.

```powershell
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t88-supports-increment/run_capture.ps1 `
    -Cell base-classic-on -Label branch5 -Probe -BatchId t88-branch5-base-classic-on `
    -ConfigPath target/t88-supports-increment/configs/pnp-classic-supports-on-branch5.json
```

## 4. Reduce and verify

```powershell
python docs/specs/perf-vs-orca/evidence/t88-supports-increment/extract.py --raw target/t88-supports-increment
python docs/specs/perf-vs-orca/evidence/t88-supports-increment/reduce.py
python docs/specs/perf-vs-orca/evidence/t88-supports-increment/verify.py
python docs/specs/perf-vs-orca/evidence/t88-supports-increment/verify.py --self-test
```

`verify.py` is checkout-only: it reads `captures/*.json`, `summary.json` and the
four t87 scoreboard row CSVs. `--self-test` perturbs the committed summary and
captures and expects every perturbation to be rejected.

## 5. Disclosures

- The captures ran on the user's Windows desktop under sustained external load
  (68–73 % total CPU; Discord/Steam/Opera/Deadlock). Instrumented walls are
  attribution-only and are never quoted as speed claims
  (`docs/17_agent_debugging.md`). Within-run shares are the stable signal; the
  quiet cell walls are the t87 scoreboard medians.
- The probe build differs from HEAD only in the four `crates/` files in
  `probe.patch`; the source is restored with `git apply -R` before the take
  closes, and `cargo xtask build-guests --check` is re-run.
- `tree_support_branch_distance = 5.0` is a measurement arm. Changing the key
  is geometry-changing parity work and needs a separate human ruling; no
  production config or source is changed by this take.
