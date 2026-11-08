# t118 — The compiled-component cache: full e2e bucket 21.4 min → 12.6 min

Route: [ticket 118](../../issues/118-compiled-component-cache.md), executed under
the human's 2026-11-04 milestone: **the whole `slicer-runtime` e2e bucket at most
15 minutes on this PC, idle, no external load.**

## Headline

**The full bucket is 755.12 s (12.59 min), 172 passed / 1 ignored, on a quiet
box.** Before this take the bucket was a censored >40-minute run (2026-11-03)
with a measured/cited floor of **4,065 s (68 min)** and a **5,263 s (≈88 min)**
linear extrapolation. The milestone is met with **2.4 min of headroom**.

Three levers, all measured:

1. **Process-global compiled-component cache** (`component_cache.rs`): one
   engine per `profile` flag and one component per
   `(profile, canonical path, mtime, length)`. A debug slice of
   `resources/calicat.stl` through 24 modules fell from **11.732 s to 2.813 s
   warm** (cold 15.424 s, which populates the cache); the G-code is
   byte-identical (`d12cdfd1…`).
2. **Validated on-disk wasmtime cache** (`Config::cache`): a `pnp_cli`
   invocation is one process, so the process cache cannot help it; the disk
   cache makes the second and later processes hit. Cache size after the run:
   **15 MB** under `%TEMP%/pnp-wasmtime-cache-v1` (overridable with
   `PNP_WASMTIME_CACHE_DIR`, disable with `PNP_DISABLE_WASMTIME_CACHE=1`).
3. **Tier-2 capture parallelism**: `execute_captured_stages_with_support_tools`
   ran the selected layers in a serial `for` loop while the ordinary path used
   `par_iter`; it now uses the same rayon parallelism and returns the first
   failing layer's error in layer order. The lightning corpus tests also run
   their five independent cases in parallel.

## Table 1 — module walls, serial, warm cache

| module | before (s) | after (s) | delta |
| --- | ---: | ---: | ---: |
| `preparation_runtime_lifecycle_tdd` | ~1,800 (ticket's 14-of-16-in-30-min) | **267.60** | ~6.7x |
| `slice_end_to_end_tdd` | 200.86 | **49.96** | −75.1 % |
| `slicing_precision_integration_tdd` | 86.66 | **11.37** | −86.9 % |
| `preparation_visual_capture_tdd` | 96.22 | **22.67** | −76.4 % |
| `modifier_infill_tdd` (post-t114-merge) | 26.78 | **7.14** | −73.3 % |
| `calicat_internal_bridge_gating_e2e_tdd` | 24.61 | **5.43** | −77.9 % |

## Table 2 — the full bucket

| state | wall | note |
| --- | ---: | --- |
| 2026-11-03 baseline | >40 min (censored) | killed at 40 min, 93 passed, 9 stale-binary failures |
| t114 measured/cited floor | 4,065 s (67.8 min) | 854 s remainder + 1,411 s lightning + 1,800 s prep |
| t114 extrapolated | 5,263 s (87.7 min) | linear whale extrapolation |
| after process + disk cache + layer parallelism | **1,282.18 s (21.37 min)** | 172 tests, warm cache |
| after corpus-case parallelism | **755.12 s (12.59 min)** | 172 passed / 1 ignored, quiet box |

Run at the default 12 test threads (`cargo test -p slicer-runtime --test e2e`;
the `xtask test` entry point adds ~11 s of preflights, still under 15 min).

## Correctness

- Full suite: **172 passed, 0 failed, 1 ignored** (the `#[ignore]` recorder).
- Calicat G-code SHA-256 unchanged before/after the cache:
  `d12cdfd10dab66a0010a0b5d5fca53d9387f4e96227ad5e74d0ec18197c0e260`.
- Every module in Table 1 passes with its assertions intact; the t114 audit's
  adopted merges are still in place.
- `cargo clippy --workspace --all-targets -- -D warnings` clean;
  `check-literals` 0; `check-test-quality --report` 0.
- Cache invalidation is by file identity; a rewritten `.wasm` misses. Failures
  are never cached. Per-run isolation is unchanged (fresh instance pools and
  stores per load); only the compiled artifact is shared.

## Disclosures

- **Warm cache.** The 755 s run uses the disk cache populated by the preceding
  runs; a cold first run pays one ~15 s compile and then hits. The process cache
  is per test binary, so the full-suite process pays one load.
- **Scope.** This is the `slicer-runtime` e2e bucket (the subject of t114); the
  other buckets were not re-measured.
- **Box.** Windows, debug profile, 12 logical cores. External load was idle at
  the start (13 % sample, no game/other builds); the suite itself saturates the
  cores.
- **Model correction.** t114 modelled a cache ceiling of ≈4.5 min from
  `10 s + ~145 slices × 1.8 s`. The measured 12.6 min is higher because the
  lightning captures do 38–52 s of real pipeline work each, not 1.8 s: the
  remaining wall is distinct capture work, not load.

## Evidence

- `summary.json` — every number above, machine-readable.
- `verify.py` — re-derives the arithmetic and checks the tree for the cache and
  the parallel capture loop; `--self-test` mutates the set and requires rejection.
- `REPRODUCE.md` — the exact commands.
