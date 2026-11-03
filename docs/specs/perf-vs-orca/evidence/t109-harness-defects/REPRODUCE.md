# t109 reproduction

All commands run from the repository root on Windows with the pinned toolchain.

## 1. Verify the committed set (static checks, no cargo)

```bash
python docs/specs/perf-vs-orca/evidence/t109-harness-defects/verify.py --no-cargo
```

## 2. Verify the narrow claims with cargo

```bash
python docs/specs/perf-vs-orca/evidence/t109-harness-defects/verify.py
```

This runs the new selector unit test and the corrected documented narrow
command with `-- --list` and requires 9 matching tests.

## 3. Full end-to-end proof

```bash
cargo xtask test -p xtask --summary
```

Expected: `test result: ok. 193 passed; 0 failed`, `VERDICT: PASS`, exit 0.
Before the fix this command exits 101 with `the package 'xtask' does not
contain this feature: slicer-core/host-algos`.

## 4. Feature still injected for slicer-core

```bash
cargo xtask test -p slicer-core --lib --no-run --summary
```

Expected: the printed cargo command contains
`--features slicer-core/host-algos`, and the run exits 0.

## 5. Negative control for the old documented command

```bash
cargo test -p slicer-runtime --test contract core_module_ir_access_contract_tdd
```

Expected: `0 passed; 314 filtered out` — the green-looking no-op that motivated
the A2 fix.

## 6. Full workspace gates

```bash
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt -p xtask --check
cargo test -p xtask
```

## Adjacent red, not fixed

```bash
cargo test -p slicer-config --test registry_census_tdd real_toml_manifest_census_rederives_expected_scope
```

Expected before and after this take: `left: 293`, `right: 281`. The failure is
pre-existing and is not caused by the injection change (plain cargo, no xtask).
