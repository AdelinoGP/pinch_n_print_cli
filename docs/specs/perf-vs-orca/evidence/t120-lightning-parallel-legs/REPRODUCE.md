# Reproduce the lightning test-leg concurrency screen

Run from the workspace root on Windows, in a debug build. Choose a quiet machine
for an idle result and record its load; the checked-in screen had active browser
load. Do not run the two arms concurrently. The before arm requires the
pre-change source; the after arm requires this change.

```powershell
cargo xtask build-guests --check  # must exit 0; rebuild if stale
cargo build --bin pnp_cli

# Warm the disk cache and capture the calicat G-code in a file, preserving bytes.
cmd /c 'target\debug\pnp_cli.exe slice --model resources\calicat.stl --module-dir modules\core-modules --no-default-module-paths > target\calicat-perf.gcode 2> target\calicat-perf.stderr.log'
Get-FileHash target/calicat-perf.gcode -Algorithm SHA256

# Before, on the pre-change source; preserve the log before the next test run.
cargo test -p slicer-runtime --test e2e 2>&1 | Tee-Object -FilePath target/test-output.log
Copy-Item target/test-output.log target/e2e-before-parallel.log

# After, on the changed source. The xtask writes target/test-output.log itself.
cargo xtask test --summary -p slicer-runtime --test e2e
Copy-Item target/test-output.log target/e2e-after-parallel.log

cargo clippy --workspace --all-targets -- -D warnings
cargo xtask check-literals
cargo xtask check-test-quality --report
python docs/specs/perf-vs-orca/evidence/t120-lightning-parallel-legs/verify.py --self-test
```

The checked-in verifier reads the reduced recorded evidence, not new output
under `target/`. To compare a new pair, inspect the complete logs and rederive
the per-test roster and libtest summaries; do not overwrite the historical
measurement with a new run under different conditions.
