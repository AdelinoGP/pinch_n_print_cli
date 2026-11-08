# Reproduce t76

All commands run from the workspace root. The model fixtures `tmp/base.stl`
and `tmp/3dbenchy.stl` are user-supplied and gitignored; the frozen snapshot
tree under `.local-artifacts/perimeter-reference-preparation/t76-meshanalysis-union/`
is gitignored. The committed reduced set (`qualification.json`,
`ab-reduction.json`) is what `verify.py` reads.

## Preconditions

```bash
cargo xtask build-guests --check                    # exit 0
cargo xtask build-guests --accelerated --check      # exit 0
```

## Attribution / census

```bash
cargo test --release -p slicer-core --features host-algos \
  --test t76_union_probe_tdd -- --nocapture
```

The probe is `probe_t76_union_probe_tdd.rs.txt` in this directory; copy it to
`crates/slicer-core/tests/t76_union_probe_tdd.rs` (it is not committed).
Fixture absence makes it print `SKIP` and pass.

## Candidate

`candidate.patch` is the diff against the pre-take HEAD `0691f050`:

```bash
git apply docs/specs/perf-vs-orca/evidence/t76-footprint-union/candidate.patch
cargo build --release --bin pnp_cli
```

The candidate is inert below 100,000 projected facets, so all synthetic tests
and 3D Benchy keep the one-shot union.

## Snapshots (both arms, both modes)

```bash
# candidate applied
cargo xtask dist --edition developer
cargo xtask dist --edition developer --accelerated
# copy target/dist/developer and target/dist-accelerated/developer into
#   .local-artifacts/perimeter-reference-preparation/t76-meshanalysis-union/snapshots/{ordinary,accelerated}-candidate

# candidate stashed (git stash push -- crates/slicer-core/src/algos/mesh_analysis.rs)
cargo build --release -p pnp-cli                    # ordinary baseline binary
cargo xtask dist --edition developer --accelerated  # accelerated baseline
# copy the binary + the candidate snapshot's modules into .../{ordinary,accelerated}-baseline
# git stash pop
```

Each mode's module set is byte-identical between its two arms (the change is
inside `host-algos`, which no guest compiles); ordinary and accelerated module
sets differ from each other by construction.

## Proof (16 cells, both modes)

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t76-footprint-union/run-all-proof.ps1
python docs/specs/perf-vs-orca/evidence/t76-footprint-union/build-qualification.py
```

## Standing paired gate

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t76-footprint-union/run-all-measure.ps1
python docs/specs/perf-vs-orca/evidence/t76-footprint-union/reduce_ab.py
python docs/specs/perf-vs-orca/evidence/t76-footprint-union/verify.py
```

Pre-declared: gate cells are the two base classic supports-on cells (6
alternating pairs each, 2 warmups per arm, `RAYON_NUM_THREADS=12`, external
`GetProcessTimes`, frozen snapshots); wall must fall and paired-median CPU must
not exceed +1.0 %. The two benchy classic supports-on cells are measured
controls (the candidate is below its threshold there).
