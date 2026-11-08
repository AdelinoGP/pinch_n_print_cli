# Reproduce — t118 compiled-component cache

All commands from the workspace root, Windows Git Bash, debug profile, idle box.

## 0. Preconditions

```bash
cargo xtask build-guests --check     # exit 0
cargo build --bin pnp_cli            # the locator's stale gate
```

## 1. One slice: before, cold, warm

```bash
# warm (the disk cache is already populated by an earlier run)
TIMEFORMAT='wall=%3R'
time target/debug/pnp_cli.exe slice \
  --model resources/calicat.stl \
  --module-dir modules/core-modules --no-default-module-paths > /dev/null

# cold: remove the disk cache first (or set a fresh PNP_WASMTIME_CACHE_DIR)
rm -rf "$TEMP/pnp-wasmtime-cache-v1"
time target/debug/pnp_cli.exe slice \
  --model resources/calicat.stl \
  --module-dir modules/core-modules --no-default-module-paths > /dev/null

# correctness: must equal d12cdfd1…
target/debug/pnp_cli.exe slice \
  --model resources/calicat.stl \
  --module-dir modules/core-modules --no-default-module-paths | sha256sum
```

The measured samples are 11.732 s (HEAD before the cache), 15.424 s (cold) and
2.813 s (warm). The G-code hash is
`d12cdfd10dab66a0010a0b5d5fca53d9387f4e96227ad5e74d0ec18197c0e260` in every
arm.

## 2. Module walls

```bash
for m in preparation_runtime_lifecycle_tdd slice_end_to_end_tdd \
         slicing_precision_integration_tdd preparation_visual_capture_tdd \
         modifier_infill_tdd calicat_internal_bridge_gating_e2e_tdd; do
  cargo test -p slicer-runtime --test e2e -- --test-threads 1 "$m::" 2>&1 \
    | grep -E '^test result:'
done
```

`test result: … finished in <seconds>` is the module wall; the t118 table
records 267.60 / 49.96 / 11.37 / 22.67 / 7.14 / 5.43 s.

## 3. The milestone run

```bash
# idle box; default 12 test threads is the documented dev-loop command
time cargo test -p slicer-runtime --test e2e
# test result: ok. 172 passed; 0 failed; 1 ignored; … finished in 755.12s
```

The gated entry point (`cargo xtask test -p slicer-runtime --test e2e`) adds the
t108-measured ~11 s of preflights; it is still under the 900 s limit.

## 4. Verifier

```bash
python docs/specs/perf-vs-orca/evidence/t118-component-cache/verify.py
python docs/specs/perf-vs-orca/evidence/t118-component-cache/verify.py --self-test
```

Both exit 0 from a plain checkout. The cache is a runtime optimization; no
committed fixture or reference G-code changes.
