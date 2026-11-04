# t113 reproduction

Take HEAD: `19a5edfe`. The probe patch is against that tree; a later tree may
need the anchors re-resolved.

## 1. Apply the probe and rebuild the planner guest

```bash
git apply docs/specs/perf-vs-orca/evidence/t113-strong-planner-phase1/probe.patch
cargo xtask build-guests
```

`build-guests` reports `STALE: tree-support-planner-guest` and rebuilds exactly
that guest. Both freshness gates must still exit 0 afterwards; the probe only
adds timing brackets and logs one `t113-planner-split` module line.

## 2. Capture one instrumented pass per cell

The standing capture driver is
`docs/specs/perf-vs-orca/evidence/t104-phase-split/run-instrumented.py`:

```bash
python docs/specs/perf-vs-orca/evidence/t104-phase-split/run-instrumented.py \
  --cells benchy-classic-off,benchy-classic-on,benchy-arachne-on,base-classic-off,base-classic-on,base-arachne-on \
  --out-dir target/t113-probe
```

The `sha` / `ref_match` lines are **not** a t113 gate: the t87/t91 references
predate the `a5e9a403` merge and no longer match HEAD. The probe's output
neutrality is checked separately in step 4.

## 3. Reduce and verify

```bash
python docs/specs/perf-vs-orca/evidence/t113-strong-planner-phase1/reduce_t113.py \
  --raw-root target/t113-probe
python docs/specs/perf-vs-orca/evidence/t113-strong-planner-phase1/verify.py --self-test
```

`reduce_t113.py` writes `reduced/summary.json`; `verify.py --self-test` exits 0
checkout-only with 6/6 negative controls rejected. The raw JSONL tree stays
gitignored.

## 4. Check the probe is output-neutral

With the probe still built, keep the probe G-code, then restore the guest and
run the same cell uninstrumented:

```bash
cargo xtask build-guests          # after `git apply -R probe.patch`
target/release/pnp_cli.exe slice \
  --model tmp/base.stl \
  --output target/t113-baseline/base-classic-on.gcode \
  --config docs/specs/perf-vs-orca/evidence/matched-pair/configs/pnp-classic-supports-on.json \
  --module-dir modules/core-modules
sha256sum target/t113-baseline/base-classic-on.gcode target/t113-probe/base-classic-on.gcode
```

The take checked three cells: base classic-on `4b6d2843…`, benchy classic-on
`3b8fa86a…`, benchy arachne-on `d4644c14…` — probe and HEAD baseline are
byte-identical.

## 5. Restore

```bash
git apply -R docs/specs/perf-vs-orca/evidence/t113-strong-planner-phase1/probe.patch
cargo xtask build-guests
cargo xtask build-guests --check
```

The restored guest must be fresh (exit 0). The probe patch is preserved and
`git apply --check` must pass against the restored tree.
