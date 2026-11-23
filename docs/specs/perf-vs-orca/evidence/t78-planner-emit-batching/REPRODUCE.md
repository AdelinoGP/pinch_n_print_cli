# Reproduce t78 — tree-support planner emit-pass batching

All commands run from the workspace root. The model fixtures `tmp/base.stl`
and `tmp/3dbenchy.stl` are user-supplied and gitignored. The candidate is
[`candidate.patch`](candidate.patch) (a diff against the pre-take HEAD
`6fdb02ac`); the committed reduced observations are [`summary.json`](summary.json),
which [`verify.py`](verify.py) checks checkout-only.

## Preconditions

```bash
cargo xtask build-guests --check                    # exit 0
cargo xtask build-guests --accelerated --check      # exit 0
```

## Candidate arm

```bash
git apply docs/specs/perf-vs-orca/evidence/t78-planner-emit-batching/candidate.patch
cargo xtask build-guests                            # rebuild the planner guest
cargo xtask build-guests --accelerated              # its accelerated twin
cargo build --release --bin pnp_cli
```

## Exactness before timing

The serial `build_roles` path is kept as the reference implementation;
`build_roles_batched` is production. The native parity test compares them
per item:

```bash
cargo test --manifest-path modules/core-modules/tree-support-planner/Cargo.toml
```

`build_roles_batched_matches_the_serial_path_per_item` drives four items that
exercise the carve, union, no-collision early return and territory intersect
phases and asserts item-for-item equality with `build_roles`. The end-to-end
gate is byte identity: both arms' G-code must hash to the preserved t58
reference `f60e5e9b8c0d836cf73170e645c18f9193b91d47e694d335cfb1d78032682319`
for base classic supports-on.

## Same-machine pair and attribution

One uninstrumented run per arm, external `GetProcessTimes`, `RAYON_NUM_THREADS=12`,
external modules only:

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t78-planner-emit-batching/run-cell-uninstrumented.ps1 `
  -Model tmp/base.stl `
  -Config docs/specs/perf-vs-orca/evidence/matched-pair/configs/pnp-classic-supports-on.json `
  -Out target/t78-base-classic-on.gcode -Err target/t78-base-classic-on.stderr.jsonl
```

The instrumented companion adds `--instrument-stderr --profile
--profile-verbose` and is attribution-only (the standing harness rejects
`module_complete`/`stage_complete` events in a timing cell):

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t78-planner-emit-batching/run-cell-attribution.ps1 `
  -Model tmp/base.stl `
  -Config docs/specs/perf-vs-orca/evidence/matched-pair/configs/pnp-classic-supports-on.json `
  -Out target/t78-base-classic-on-prof.gcode -Err target/t78-base-classic-on-prof.stderr.jsonl
```

## Baseline arm

The candidate is the only working-tree change in the guest, so the baseline is
the same tree with it stashed. `git stash push` (never `checkout`) is the rule:

```bash
git stash push -- modules/core-modules/tree-support-planner/src/lib.rs
cargo xtask build-guests                            # rebuild the pre-take guest
# ... rerun both scripts above with -baseline output names ...
git stash pop
cargo xtask build-guests
cargo xtask build-guests --accelerated
```

## Verification

```bash
python docs/specs/perf-vs-orca/evidence/t78-planner-emit-batching/verify.py
```

## Standing gate (run 2026-10-22)

The 16-cell proof matrix and the 6-pair campaign are the ticket's own gate.
Both drivers are resumable: a cell is skipped when its run dir already exists,
and a `measure` cell must have exactly 14 rows to count as complete.

```bash
# 1. 16-cell output identity (2 arms x 16 cells)
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t78-planner-emit-batching/run-all-proof.ps1
python docs/specs/perf-vs-orca/evidence/t78-planner-emit-batching/build-qualification.py

# 2. 6 alternating pairs on the four pre-declared classic supports-on cells
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t78-planner-emit-batching/run-all-measure.ps1
python docs/specs/perf-vs-orca/evidence/t78-planner-emit-batching/reduce_ab.py

# 3. checkout-only verification (take summary + matrix + campaign)
python docs/specs/perf-vs-orca/evidence/t78-planner-emit-batching/verify.py
```

Snapshots for both arms and both modes are built from
`.local-artifacts/perimeter-reference-preparation/t78-planner-emit-batching/snapshots/`:
`cargo xtask dist --edition developer` (and `--accelerated`) on the candidate
tree, then the same with the module patch stashed. The only module file that
differs between the two arms is `tree-support-planner.wasm`; the `pnp_cli.exe`
binaries are byte-identical.

The observed 2026-10-22 result is **PASS** in all four cells with 24/24 retained
pairs; see [`FINDINGS.md`](FINDINGS.md) and [`ab-reduction.json`](ab-reduction.json).

