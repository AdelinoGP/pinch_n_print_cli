# Reproduce t91 — per-object memo for the z-independent exact-Z terms

All commands run from the workspace root. The model fixtures `tmp/base.stl` and
`tmp/3dbenchy.stl` are user-supplied and gitignored. The candidate is the
working-tree change to two host files:

- `crates/slicer-wasm-host/src/exact_z_query.rs` (the memo)
- `crates/slicer-wasm-host/tests/contract/exact_z_support_query.rs` (its
  identity contract)

The committed reduced observations are `proof-rows.json`, `measure-rows.json`,
`attribution.json`, `provenance.json` and the `*.json` reductions `verify.py`
checks checkout-only.

## Preconditions

```bash
cargo xtask build-guests --check                    # exit 0
cargo xtask build-guests --accelerated --check      # exit 0
```

Both gates must be green **before** the snapshots are built. The candidate is
host-only, so both arms ship byte-identical module trees; `provenance.json`
records the per-file module SHAs and both arms' `pnp_cli.exe` SHAs.

## Snapshots (two arms, two modes)

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t91-exact-z-memo/build-snapshots.ps1 -Arm candidate -Modes both
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t91-exact-z-memo/build-snapshots.ps1 -Arm baseline  -Modes both
```

The baseline arm is the same tree with `crates/slicer-wasm-host/src/exact_z_query.rs`
`git stash`ed (path-limited; never a checkout). The script pops the stash in a
`finally` block and writes `provenance/<mode>-<arm>.json`.

## Exactness before timing

```bash
cargo test -p slicer-wasm-host --test contract exact_z
cargo test -p slicer-runtime --test integration support_family_closure
cargo test -p slicer-runtime --test contract support_plan_validation
```

`exact_z_memoizes_z_independent_terms_per_object`
(`crates/slicer-wasm-host/tests/contract/exact_z_support_query.rs`) pins the
identity contract: two Z values on one object build the object's terms once and
observe identical termination surfaces / plate, two objects never share them,
and the requested-Z occupancy still tracks the requested Z.

## Proof matrix (16 cells x 2 arms)

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t91-exact-z-memo/run-all-proof.ps1
python docs/specs/perf-vs-orca/evidence/t91-exact-z-memo/extract.py
python docs/specs/perf-vs-orca/evidence/t91-exact-z-memo/reduce.py --raw
```

`run-all-proof.ps1` is resumable (one-shot per cell dir). Ordinary cells are
checked against `references.json`, which `derive_references.py` re-derives from
the committed t81 campaign rows; accelerated cells are gated on arm-to-arm
identity and their agreement with the ordinary cell is reported.

## Paired A/B (4 cells, 6 alternating pairs each)

Requires `qualification.json` from the step above; every measure cell is
guarded on it. The pre-declared timing set is the four base supports-on cells,
`{ordinary, accelerated} x {classic, arachne}`.

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t91-exact-z-memo/run-all-measure.ps1
python docs/specs/perf-vs-orca/evidence/t91-exact-z-memo/extract.py
python docs/specs/perf-vs-orca/evidence/t91-exact-z-memo/reduce.py --raw
```

The machine must be otherwise idle: the harness pins `RAYON_NUM_THREADS=12`,
and a starved sample (cpu/wall below 0.5x its arm's median) is excluded as a
whole pair, never resampled. The CPU clause is the ticket's **flat or lower**
(`<= 0.0 %` paired median), not the map's +3.0 % work-preserving ceiling: this
candidate removes work instead of spreading it.

## Attribution (stage walls, never a wall claim)

One instrumented capture per arm per measured cell:

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t91-exact-z-memo/run-all-attribution.ps1
python docs/specs/perf-vs-orca/evidence/t91-exact-z-memo/extract.py
```

`post_planner_gap_ms` is the `PrePass::SupportGeometry` stage wall minus the
planner modules' own walls — the bracket that carries the exact-Z validation
loop (`aggregate_support_plan_irs_degrading_with_attributed_diagnostics`). Its
drop is where the memo removes work; it is not the cell's wall delta.

## Verification

```bash
python docs/specs/perf-vs-orca/evidence/t91-exact-z-memo/derive_references.py
python docs/specs/perf-vs-orca/evidence/t91-exact-z-memo/verify.py --self-test
```

`verify.py` exits 0 against the in-repo set alone (no `.local-artifacts`),
re-derives the reference table from the t81 campaign rows, and rejects its
negative controls.

## Pre-existing reds that are not this candidate's

At the take's HEAD `a1185c0b`, the `slicer-runtime` contract / executor /
integration suites carry the post-ticket-81 `resolution` fixture failures
disclosed by t90 (23 contract, 3 executor, 50 integration). They are measured /
diagnosed there and are unchanged by a host memo in
`crates/slicer-wasm-host/src/exact_z_query.rs`.
