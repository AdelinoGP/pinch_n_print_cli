# Reproduce — t141 overhang quartile bands

All commands run from the workspace root on Windows (Git Bash / PowerShell).

## 1. Re-apply the probe

```bash
git apply docs/specs/perf-vs-orca/evidence/t141-overhang-bands/probe.patch
cargo build --release --bin pnp_cli            # ~7-9 min, link-bound
```

`probe.patch` adds a temporary, env-gated probe to two files:

- `crates/slicer-core/src/algos/overhang_annotation.rs` — accumulates
  `partition_into_bands`' summed thread-time, its layer count, and its
  `offset`/boolean-op counts, plus the whole `annotate_overhangs` call wall;
- `crates/slicer-runtime/src/builtins/overhang_annotation_producer.rs` — emits
  one `[t141] ...` line to stderr after the per-object loop.

The probe is a no-op unless `PNP_PROBE_OVERHANG_BANDS` is set. It is **not**
part of the production change and was reverted before this ticket closed.

## 2. Take the stage rows

```bash
export PNP_PROBE_OVERHANG_BANDS=1
./target/release/pnp_cli.exe slice \
  --model tmp/base.stl \
  --output target/t141/base-off.gcode \
  --config docs/specs/perf-vs-orca/evidence/matched-pair/configs/pnp-classic-supports-off.json \
  --module-dir modules/core-modules --no-default-module-paths --no-progress-events \
  2> target/t141/base-off.err
grep '\[t141\]' target/t141/base-off.err
```

Cells: `pnp-classic-supports-off.json`, `pnp-arachne-supports-off.json`,
`pnp-classic-supports-on.json` against `tmp/base.stl` and `tmp/3dbenchy.stl`.
Repeat `base-classic-off` once: the stage wall repeats within 10 %.

## 3. Take the same-session uninstrumented walls

Same command with `PNP_PROBE_OVERHANG_BANDS` **unset** and no stderr capture,
timing process creation-to-exit. The three walls in `probe.json` were taken
that way (`base-classic-off` 39.924 s, `base-arachne-off` 39.365 s,
`benchy-classic-off` 4.372 s). `base-classic-on` and `benchy-classic-on` were
not re-timed this session; their shares use the t139 scoreboard medians.

## 4. Verify

```bash
python docs/specs/perf-vs-orca/evidence/t141-overhang-bands/verify.py
python docs/specs/perf-vs-orca/evidence/t141-overhang-bands/verify.py --self-test
```

Both must exit 0. `--self-test` mutates one input of each class (row
arithmetic, stage share, a value comparison on the quartile, the distance
consumer) and requires each to be rejected.

## 5. Revert

```bash
git checkout -- crates/slicer-core/src/algos/overhang_annotation.rs \
  crates/slicer-runtime/src/builtins/overhang_annotation_producer.rs
```

## Caveats carried by this evidence

- The box was **not** verified idle. The three `base-classic-off` probe rows
  span 18 % in stage wall (8.96–10.58 s), and the same-session walls are **not**
  paired A/B runs; no wall claim is made for a landed change. Row p3 was taken
  after `cargo xtask build-guests` restored `--check` to exit 0; p1/p2 used the
  artifacts already on disk.
- `tmp/base.stl` and `tmp/3dbenchy.stl` are the gitignored matched fixtures and
  are never committed.
- The `[t141]` counts are per process and per object; the matched job is a
  single object, so they are also per slice.
