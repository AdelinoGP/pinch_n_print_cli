# t101 reproduce

Everything below runs from the repo root on Windows with `pwsh`. The probe is
host-only: no guest is rebuilt and no guest artifact is touched, so the guest
freshness gates are unaffected by the probe itself.

```text
# 0. a guest tree and a developer module tree must exist
cargo xtask build-guests --check
cargo xtask dist --edition developer

# 1. apply the probe (host-only: shape census + assembly window + drop census,
#    slicer-runtime re-export, pnp-cli allocator install)
git apply docs/specs/perf-vs-orca/evidence/t101-side-table/probe.patch

# 2. build the probe host binary and stage it with the standing developer tree
pwsh docs/specs/perf-vs-orca/evidence/t101-side-table/build-probe.ps1

# 3. the table arm, sampling-free drop timing (the ceiling's free side)
pwsh docs/specs/perf-vs-orca/evidence/t101-side-table/run-probe.ps1 `
    -Generator classic -Tag t101-a7

# 4. the skip arm, sampling-free drop timing
pwsh docs/specs/perf-vs-orca/evidence/t101-side-table/run-probe.ps1 `
    -Generator classic -Tag t101-b6 -SkipTable

# 5. repeat both arms; the spread is what decides whether the wall is usable
pwsh docs/specs/perf-vs-orca/evidence/t101-side-table/run-probe.ps1 `
    -Generator classic -Tag t101-a8
pwsh docs/specs/perf-vs-orca/evidence/t101-side-table/run-probe.ps1 `
    -Generator classic -Tag t101-b7 -SkipTable -Census

# 6. the censused table arm (block counts + the shape census)
pwsh docs/specs/perf-vs-orca/evidence/t101-side-table/run-probe.ps1 `
    -Generator classic -Tag t101-a9 -Census

# 7. reduce the captures into the committed summary, then verify it
python docs/specs/perf-vs-orca/evidence/t101-side-table/extract.py
python docs/specs/perf-vs-orca/evidence/t101-side-table/verify.py --self-test

# 8. revert the probe and re-check the guests
git checkout -- crates/slicer-gcode/src/lib.rs crates/slicer-gcode/src/emit.rs `
    crates/slicer-runtime/src/lib.rs crates/slicer-runtime/src/layer_executor.rs `
    crates/pnp-cli/src/main.rs
rm -f crates/slicer-gcode/src/probe_t101.rs
cargo xtask build-guests --check
cargo xtask build-guests --accelerated --check
```

The raw captures land under
`.local-artifacts/perimeter-reference-preparation/t101-side-table/probe/captures/`
(gitignored). `extract.py` is the only path from them to the committed
`summary.json`, and `verify.py` reads nothing but `summary.json`, so a plain
checkout can re-verify every claim.

`PNP_T101_PROBE` prints the shape census and the `T101 ASSEMBLE` per-layer
lines. `PNP_T101_CENSUS` additionally arms the counting allocator over the
whole-print drop; leave it unset for the drop timings that feed the ceiling,
because the counters are only proven cheap, not proven free.
