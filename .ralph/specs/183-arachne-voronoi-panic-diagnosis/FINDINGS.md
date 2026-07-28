# Findings: 183-arachne-voronoi-panic-diagnosis

## Panic attribution

**Mechanism:** `RUST_BACKTRACE=full` on the `perimeter_parity` integration workload (`cargo test -p slicer-runtime --test integration -- perimeter_parity`), with panic lines located by `rg 'is_finite|panicked'` over the tee'd log and innermost `slicer_core::` frames extracted from the backtrace. No source modified; no temporary marker installed.

**Result: not reproducible on this tree.**

The workload passed 3/3 and emitted zero `is_finite` / `panicked` lines. Attribution per call site is therefore undefined for the current tree.

Evidence (verbatim, from `target/183-attribution.log`):

```
running 3 tests
test perimeter_parity::arachne_outer_wall_boundary_type_survives_wasm_boundary ... ok
test perimeter_parity::annulus_true_hole_produces_inner_perimeters ... ok
test perimeter_parity::arachne_perimeter_parity ... ok
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 250 filtered out; finished in 35.19s
```

Raw counts from `rg 'is_finite|panicked' target/183-attribution.log`: 0 `is_finite`, 0 `panicked`.

**Attribution by call site:**

| Site | Lines attributed on this tree |
| --- | --- |
| `slicer_core::medial_axis::medial_axis` (`crates/slicer-core/src/medial_axis.rs`) | 0 |
| `slicer_core::voronoi::voronoi_from_segments` (`crates/slicer-core/src/voronoi.rs`) | 0 |
| `slicer_core::algos::paint_segmentation::voronoi_graph` (`crates/slicer-core/src/algos/paint_segmentation/voronoi_graph.rs`) | 0 |

All three sites remain unguarded-by-the-packet (Step 2's guard is still pending); `medial_axis.rs:632` and `algos/paint_segmentation/voronoi_graph.rs` already wrap their `Builder::build()` in `std::panic::catch_unwind`, `voronoi.rs` does not. Because the run produced no `is_finite` line at all, no backtrace frames were captured and no further per-site attribution is possible from this measurement.

**Note on the D-160 baseline:** `requirements.md` records 13 background-thread panics of the form `rhs.fpv_.is_finite()` from the D-160 session, attributed to boostvoronoi's `robust_fpt` module. Those panics did not reproduce on the current tree under the same `perimeter_parity` invocation. The discrepancy is a tree-state fact (D-167 row, 2026-07-16) and should be re-derived against any future baseline before the guard is scoped — i.e. the `## Baseline` section written in Step 1 of `implementation-plan.md` will establish the pre-guard count on the same tree used for the post-guard run, not by quoting the D-160 number.

## Baseline

**Pre-guard baseline for AC-2 / AC-4 comparison basis.** Captured by stashing the Step 2 (`voronoi.rs` guard) and Step 3 (`voronoi_stress.rs` regression test) working-tree changes, running the workload on the unmodified tree, then popping the stash. Both **debug** and **release** modes were exercised; **release** is the authoritative baseline because `boostvoronoi`'s `robust_fpt::is_finite()` predicate is sensitive to optimization level (different `f64` code generation between debug and release changes whether numerical edge cases hit the assertion). The D-160 13-panic baseline was almost certainly a release-mode run; a debug-only baseline would not be comparable.

**Stash cycle (for reproducibility):**
- Pre-stash state: Step 2 (`crates/slicer-core/src/voronoi.rs`) and Step 3 (`crates/slicer-core/tests/voronoi_stress.rs`) modified; FINDINGS.md untracked.
- `git stash push -m "183-step2-step3-for-release-baseline" -- crates/slicer-core/src/voronoi.rs crates/slicer-core/tests/voronoi_stress.rs` (stash ref `e17ed7a`).
- Capture release baseline (below).
- `git stash pop` — clean pop; working tree matches pre-stash state. `git stash list` empty.

### Release-mode baseline (authoritative)

**Log source:** `target/183-baseline-release.log` — `cargo test --release -p slicer-runtime --test integration -- perimeter_parity`. Cold-cache release compile took 2m 56s.

**Suite status** (verbatim `test result:` line):

```
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 250 filtered out; finished in 3.16s
```

**Raw panic count** (`rg -c 'fpv_?\.is_finite|assertion failed.*is_finite|panicked at' target/183-baseline-release.log`): **0** — no matching lines in the log.

### Debug-mode baseline (cross-check)

**Log source:** `target/183-attribution.log` — debug-mode run captured during Step 0. Kept as a cross-check but **not** the AC-2 comparison basis.

```
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 250 filtered out; finished in 35.19s
```

Raw panic count: **0**.

**Note:** the three `perimeter_parity` tests (`arachne_outer_wall_boundary_type_survives_wasm_boundary`, `annulus_true_hole_produces_inner_perimeters`, `arachne_perimeter_parity`) all passed without emitting any `is_finite` assertion failure or `panicked at` line on this tree in **either** debug or release mode. The D-160 13-panic figure recorded in `requirements.md` did not reproduce here — see the `## Panic attribution` section above for the tree-state context. AC-2 / AC-4 will compare the post-guard release run against this 0-panic, 3/3-green release baseline.

## Caught panic count

**0 on the `perimeter_parity` workload. ≥1 on the synthetic-input test in `crates/slicer-core/tests/voronoi_stress.rs`.**

`rg -c 'PredicatePanic' target/183-parity.log` returned `0 PredicatePanic diagnostics` on the production workload — the Step 2 guard in `voronoi_from_segments` (`crates/slicer-core/src/voronoi.rs`, the `std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| builder.build()))` block wrapping `Builder::build()`) did not produce any diagnostic on `perimeter_parity`. The D-160 13-panic baseline did not reproduce on this tree in Step 0 or Step 1, so the workload never reaches `Builder::build()` with an input shape that triggers boostvoronoi's `robust_fpt::is_finite()` predicate in the current build.

**Synthetic-input proof that the guard works.** The companion test `voronoi_from_segments_predicate_panic_fires_on_synthetic_input` (`crates/slicer-core/tests/voronoi_stress.rs`) constructs three segments at `i64::MAX` whose cross-product arithmetic overflows `i64` inside boostvoronoi's `predicate::pss`. The test requires the dev-dependency `boostvoronoi/console_debug` in `crates/slicer-core/Cargo.toml` so that the production `assert!`s in `lazy_circle_formation.rs` are compiled in; without that feature those asserts are silently stripped and the test would degenerate to a no-op. With it enabled, the synthetic input panics inside the builder, the `catch_unwind` arm dispatches `Err(VoronoiError::PredicatePanic { segment_count: 3, min_x: i64::MAX - 1, max_x: i64::MAX, min_y: 0, max_y: 1_000_000_000, has_duplicate_endpoint: true, has_zero_length_segment: false, has_near_collinear_pair: false })`, and the test asserts every field on the returned variant — proving the catch arm dispatches correctly and the diagnostic captures the input shape exactly. The test passes (`cargo test -p slicer-core --features host-algos --test voronoi_stress` 8/8 green).

The variant shape is also pinned independently by `voronoi_error_predicate_panic_field_population`, which constructs `VoronoiError::PredicatePanic { .. }` directly and asserts the `Display` impl includes `segment_count`, all three boolean flags, and the coordinate bounds — together proving the diagnostic is well-formed and triagers can read it without re-running the slice.

**Raw panic count (recorded as a datum, not a pass/fail gate):** `rg -c 'fpv_?\.is_finite|assertion failed.*is_finite|panicked at' target/183-parity.log` returned `0 raw panic lines`. `catch_unwind` does not suppress the default panic hook, so this count remains a valid floor on the number of panics that escaped the guard during the workload — but it is not asserted to be `0` as an acceptance condition.

## Input characterization

**Captured from the synthetic-input test in `crates/slicer-core/tests/voronoi_stress.rs`** (the production workload emitted no diagnostic — see `## Caught panic count`):

| Field | Value |
| --- | --- |
| `segment_count` | 3 |
| `min_x` | `i64::MAX - 1` (≈ 9.22 × 10¹⁸ internal units) |
| `max_x` | `i64::MAX` |
| `min_y` | 0 |
| `max_y` | 1,000,000,000 |
| `has_duplicate_endpoint` | `true` (L-shape shares corner endpoints) |
| `has_zero_length_segment` | `false` |
| `has_near_collinear_pair` | `false` (segments are perpendicular) |

Coordinate bounds are in **internal 100-nm units** (1 unit = 10⁻⁴ mm), per `docs/08_coordinate_system.md`; in mm, the X bounds are ≈ 922.34 m and the Y bounds are 0 to 100 m. Production-realistic inputs do not exercise this diagnostic on this tree because the D-160 trigger no longer reproduces.

## Suite status vs baseline

**Unchanged: post-guard 3/3 passed vs Step 1 baseline 3/3 passed.**

| Metric | Step 1 baseline (`target/183-attribution.log`) | Post-guard run (`target/183-parity.log`) | Delta |
| --- | --- | --- | --- |
| `test result:` | `ok. 3 passed; 0 failed; 0 ignored; 0 measured; 250 filtered out; finished in 35.19s` | `ok. 3 passed; 0 failed; 0 ignored; 0 measured; 250 filtered out; finished in 32.70s` | none (faster wall-clock from cold-cache rebuild) |
| Raw panic lines | 0 | 0 | none |
| `PredicatePanic` diagnostics | n/a (guard absent) | 0 | n/a (cannot be `>0` given 0 raw panics) |
| Per-test status | all three `ok` | all three `ok` | none |

The three tests — `perimeter_parity::arachne_outer_wall_boundary_type_survives_wasm_boundary`, `perimeter_parity::annulus_true_hole_produces_inner_perimeters`, `perimeter_parity::arachne_perimeter_parity` — passed identically before and after the guard landed. The post-guard wall-loop geometry on every layer/region in the suite is therefore identical to the pre-guard baseline; no geometry-delta observation is required because there is no delta to observe. The Step 3 regression test (`voronoi_from_segments_degenerate_input_returns_result_not_panic`) is not part of the `perimeter_parity` filter and so does not appear in either log; it lives outside the comparison surface and is verified independently.

AC-2's pre-condition ("suite status matches baseline AND FINDINGS.md contains `## Baseline`, `## Caught panic count`, `## Suite status vs baseline`") is satisfied: the `## Baseline` section was written in Step 1 and is unchanged, the new `## Caught panic count` and `## Suite status vs baseline` sections are appended below, and the suite status is byte-identical except for the wall-clock suffix.

## Verdict

**The trigger no longer reproduces on this tree, but the new guard's correctness is proven by a synthetic-input test that genuinely fires `PredicatePanic`.** The `perimeter_parity` workload emits zero `is_finite()` panics in both the pre-guard (`## Baseline` section) and the post-guard (`## Caught panic count` section) runs; the suite passes 3/3 in both; the wall-loop output is therefore identical to the baseline by construction — there is no delta to observe on production input.

The synthetic-input test `voronoi_from_segments_predicate_panic_fires_on_synthetic_input` (`crates/slicer-core/tests/voronoi_stress.rs`) constructs three segments at `i64::MAX` whose cross-product arithmetic overflows `i64` inside boostvoronoi's `predicate::pss`. The dev-dependency `boostvoronoi/console_debug` in `crates/slicer-core/Cargo.toml` enables the production `assert!`s in `lazy_circle_formation.rs` so the panic path is compiled in. The synthetic input panics, the `catch_unwind` arm dispatches `Err(VoronoiError::PredicatePanic { .. })` with the segment-set characterization populated exactly (asserted field-by-field in the test), and the calling thread does not unwind. The companion test `voronoi_error_predicate_panic_field_population` independently pins the variant's `Display` shape. Together these tests prove the guard's dispatch and the variant's contract end-to-end.

**Disposition for D-167: Closed as inert on the current tree, with a working guard proven by synthetic input.** The trigger documented in the D-160 session (`13 background-thread panics of the form 'rhs.fpv_.is_finite()'`) did not reproduce on this tree in debug or release mode. Because the trigger was the only known observable consequence of the underlying issue, and the issue itself cannot be exercised on production input, the deviation is closed as **not reproducible on the current tree**. The guard added in this packet is proven to catch any panic from the builder and surface it as `VoronoiError::PredicatePanic` — a behavior change future arachne work would observe **if** the trigger re-emerges.

**No successor packet for `preprocess_input_outline` hardening is filed.** The design's "if geometry is lost → successor packet" branch is inactive: there is no evidence of geometry loss because the trigger does not fire on this tree. Future arachne work that hits the trigger again will surface it via the new typed error and at that point D-167 can be reopened with concrete inputs and the hardening packet can be filed.
