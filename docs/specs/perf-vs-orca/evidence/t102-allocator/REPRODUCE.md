# t102 reproduce

Global allocator substitution (`mimalloc`, wrapped by `AccountingAllocator`)
versus the Windows system heap. Everything runs from the repo root on Windows.

```text
# 1. the candidate (two host files + the contract doc)
#      crates/pnp-cli/Cargo.toml      mimalloc dependency
#      crates/pnp-cli/src/main.rs     AccountingAllocator<mimalloc::MiMalloc>
#      docs/16_slicer_report.md       the contract's example
#      Cargo.lock                     resolved by the first candidate build

# 2. snapshots for both arms, both modes (the baseline arm stashes the
#    candidate's three build inputs, builds, and pops them again)
pwsh docs/specs/perf-vs-orca/evidence/t102-allocator/build-snapshots.ps1 -Arm baseline  -Modes both
pwsh docs/specs/perf-vs-orca/evidence/t102-allocator/build-snapshots.ps1 -Arm candidate -Modes both

# 3. qualification: one proof run per arm in all 16 cells (8 cells x 2 modes)
pwsh docs/specs/perf-vs-orca/evidence/t102-allocator/run-all-proof.ps1

# 4. pre-gate: 1 warmup + 3 alternating pairs on ordinary base classic-on
pwsh docs/specs/perf-vs-orca/evidence/t102-allocator/run-pregate.ps1

# 5. reduce the captures into the committed rows, then the derived artifacts
python docs/specs/perf-vs-orca/evidence/t102-allocator/extract.py
python docs/specs/perf-vs-orca/evidence/t102-allocator/reduce.py

# 6. the four-cell campaign (only after qualification.json passes)
pwsh docs/specs/perf-vs-orca/evidence/t102-allocator/run-all-measure.ps1

# 7. re-reduce with the campaign rows and verify
python docs/specs/perf-vs-orca/evidence/t102-allocator/extract.py
python docs/specs/perf-vs-orca/evidence/t102-allocator/reduce.py
python docs/specs/perf-vs-orca/evidence/t102-allocator/verify.py --self-test
```

Raw captures land under
`.local-artifacts/perimeter-reference-preparation/t102-allocator/` (gitignored).
`extract.py` is the only path from them to the committed `*-rows.json`, and
`verify.py` reads nothing but the committed artifacts, so a plain checkout can
re-verify every claim.

The two arms ship byte-identical guest modules (the candidate is host-only);
`provenance/<mode>-<arm>.json` pins each arm's commit, dirty source set, host
binary SHA and per-module SHAs.
