# t100 reproduce

Everything below runs from the repo root on Windows. The probe is host-only:
no guest is rebuilt and no guest artifact is touched, so the guest freshness
gates are unaffected by the probe itself.

```text
# 1. apply the probe (host-only: slicer-gcode drop census + counting allocator,
#    slicer-runtime re-export, pnp-cli allocator install)
git apply docs/specs/perf-vs-orca/evidence/t100-allocation-arena/probe.patch

# 2. build the probe host binary and stage it with the standing developer tree
pwsh docs/specs/perf-vs-orca/evidence/t100-allocation-arena/build-probe.ps1

# 3. the census run (sampling-free drop, no counterfactuals)
pwsh docs/specs/perf-vs-orca/evidence/t100-allocation-arena/run-probe.ps1 `
    -Generator classic -Tag t100-classic-census

# 4. the bulk counterfactuals in both pricing orders, at the byte volume the
#    census reported (1,358,397,544 bytes; 1 block = per-print arena,
#    495 blocks = one arena per layer)
pwsh docs/specs/perf-vs-orca/evidence/t100-allocation-arena/run-probe.ps1 `
    -Generator classic -Tag t100-classic-bulk -Bulk both `
    -BulkBytes 1358397544 -BulkBlocks 1,495

# 5. the shape + sample + copy build (same allocator; the sampling
#    instrumentation and the copy arm were added after step 3)
pwsh docs/specs/perf-vs-orca/evidence/t100-allocation-arena/run-probe.ps1 `
    -Generator classic -Tag t100-classic-bulk2 -Bulk after `
    -BulkBytes 1358397544 -BulkBlocks 1,495

# 6. the allocator arm: swap the probe binary's global allocator to mimalloc.
#    Three-line delta on top of probe.patch (Cargo.lock changes with it):
#      crates/pnp-cli/Cargo.toml:  mimalloc = { version = "0.1.52", default-features = false }
#      crates/pnp-cli/src/main.rs: static ALLOC: Counting<mimalloc::MiMalloc> = Counting::new(mimalloc::MiMalloc);
#    then build with `cargo build --offline -p pnp-cli --release` and run:
pwsh docs/specs/perf-vs-orca/evidence/t100-allocation-arena/run-probe.ps1 `
    -Generator classic -Tag t100-classic-mimalloc -Bulk after `
    -BulkBytes 1358397544 -BulkBlocks 1,495

# 7. reduce the captures into the committed summary, then verify it
python docs/specs/perf-vs-orca/evidence/t100-allocation-arena/extract.py
python docs/specs/perf-vs-orca/evidence/t100-allocation-arena/verify.py --self-test

# 8. revert the probe (including the mimalloc delta) and re-check the guests
git checkout -- crates/slicer-gcode/src/emit.rs crates/slicer-gcode/src/lib.rs `
    crates/slicer-runtime/src/lib.rs crates/pnp-cli/src/main.rs `
    crates/pnp-cli/Cargo.toml Cargo.lock
rm -f crates/slicer-gcode/src/probe_t100.rs
cargo xtask build-guests --check
cargo xtask build-guests --accelerated --check
```

The raw captures land under
`.local-artifacts/perimeter-reference-preparation/t100-allocation-arena/probe/captures/`
(gitignored). `extract.py` is the only path from them to the committed
`summary.json`, and `verify.py` reads nothing but `summary.json`, so a plain
checkout can re-verify every claim.
