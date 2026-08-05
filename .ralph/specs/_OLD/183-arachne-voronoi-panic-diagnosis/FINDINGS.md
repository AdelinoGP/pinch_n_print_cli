# Findings: 183-arachne-voronoi-panic-diagnosis

## Panic attribution

**Mechanism:** `RUST_BACKTRACE=full` on the `perimeter_parity` integration workload (`cargo test -p slicer-runtime --test integration -- perimeter_parity`), with panic lines located by `rg 'is_finite|panicked'` over the tee'd log and innermost `slicer_core::` frames extracted from the backtrace. No source modified; no temporary marker installed.

**Result: measured baseline observed zero `is_finite` panic events.**

The workload passed 3/3 and emitted zero `is_finite` / `panicked` lines, so no call site produced an observed event and attribution is N/A for this workload. This result is distinct from the three structurally possible boostvoronoi entry points below; they are candidate paths, not observed call-site attribution.

Evidence (verbatim, from `target/183-attribution.log`):

```
running 3 tests
test perimeter_parity::arachne_outer_wall_boundary_type_survives_wasm_boundary ... ok
test perimeter_parity::annulus_true_hole_produces_inner_perimeters ... ok
test perimeter_parity::arachne_perimeter_parity ... ok
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 250 filtered out; finished in 35.19s
```

Raw counts from `rg 'is_finite|panicked' target/183-attribution.log`: 0 `is_finite`, 0 `panicked`.

**Three structurally possible boostvoronoi entry points (not observed attribution):**

| Site | Baseline observation |
| --- | --- |
| `slicer_core::medial_axis::medial_axis` (`crates/slicer-core/src/medial_axis.rs`) | No observed `is_finite` event |
| `slicer_core::voronoi::voronoi_from_segments` (`crates/slicer-core/src/voronoi.rs`) | No observed `is_finite` event |
| `slicer_core::algos::paint_segmentation::voronoi_graph` (`crates/slicer-core/src/algos/paint_segmentation/voronoi_graph.rs`) | No observed `is_finite` event |

These three sites remain structural candidates only. Because the run produced no `is_finite` line at all, no backtrace frames were captured and no further per-site attribution is possible from this measurement; attribution remains N/A.

**Note on the D-160 baseline:** `requirements.md` records 13 background-thread panics of the form `rhs.fpv_.is_finite()` from the D-160 session, attributed to boostvoronoi's `robust_fpt` module. Those panics did not reproduce on the current tree under the same `perimeter_parity` invocation. The discrepancy is a tree-state fact (D-167 row, 2026-07-16) and should be re-derived against any future baseline before the guard is scoped — i.e. the `## Baseline` section written in Step 1 of `implementation-plan.md` will establish the pre-guard count on the same tree used for the post-guard run, not by quoting the D-160 number.

## Baseline

**Pre-guard baseline for AC-2 / AC-4 comparison basis.** Captured by stashing the Step 2 (`voronoi.rs` guard) and Step 3 (`voronoi_stress.rs` regression test) working-tree changes, running the workload on the unmodified tree, then popping the stash. Both **debug** and **release** modes were exercised. The Step-0/Step-1 comparison and the post-guard parity comparison use **debug-profile** logs; the release run is a separate cross-check only and has no release post-guard counterpart. The debug Step-1 baseline is therefore the AC-2 / AC-4 comparison basis. The D-160 13-panic baseline was almost certainly a release-mode run; a debug-only baseline would not be comparable.

**Stash cycle (for reproducibility):**
- Pre-stash state: Step 2 (`crates/slicer-core/src/voronoi.rs`) and Step 3 (`crates/slicer-core/tests/voronoi_stress.rs`) modified; FINDINGS.md untracked.
- `git stash push -m "183-step2-step3-for-release-baseline" -- crates/slicer-core/src/voronoi.rs crates/slicer-core/tests/voronoi_stress.rs` (stash ref `e17ed7a`).
- Capture release baseline (below).
- `git stash pop` — clean pop; working tree matches pre-stash state. `git stash list` empty.

### Release-mode baseline (separate cross-check)

**Log source:** `target/183-baseline-release.log` — `cargo test --release -p slicer-runtime --test integration -- perimeter_parity`. Cold-cache release compile took 2m 56s.

**Suite status** (verbatim `test result:` line):

```
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 250 filtered out; finished in 3.16s
```

**Raw panic count** (`rg -c 'fpv_?\.is_finite|assertion failed.*is_finite|panicked at' target/183-baseline-release.log`): **0** — no matching lines in the log.

### Debug-mode baseline (AC-2 comparison basis)

**Log source:** `target/183-attribution.log` — debug-mode run captured during Step 0. This is the Step-0/Step-1 comparison basis for AC-2 and AC-4; the release baseline is a separate cross-check only.

```
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 250 filtered out; finished in 35.19s
```

Raw panic count: **0**.

**Note:** the three `perimeter_parity` tests (`arachne_outer_wall_boundary_type_survives_wasm_boundary`, `annulus_true_hole_produces_inner_perimeters`, `arachne_perimeter_parity`) all passed without emitting any `is_finite` assertion failure or `panicked at` line on this tree in **either** debug or release mode. The D-160 13-panic figure recorded in `requirements.md` did not reproduce here — see the `## Panic attribution` section above for the tree-state context. AC-2 / AC-4 compare the post-guard debug run against this 0-panic, 3/3-green debug baseline; the separate release baseline has no release post-guard counterpart.

## Caught panic count

**0 on the `perimeter_parity` workload. ≥1 on the synthetic-input test in `crates/slicer-core/tests/voronoi_panic_regression.rs`.**

`rg -c 'PredicatePanic' target/183-parity.log` returned `0 PredicatePanic diagnostics` on the production workload — the Step 2 guard in `voronoi_from_segments` (`crates/slicer-core/src/voronoi.rs`, the `std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| builder.build()))` block wrapping `Builder::build()`) did not produce any diagnostic on `perimeter_parity`. The D-160 13-panic baseline did not reproduce on this tree in Step 0 or Step 1, so the workload never reaches `Builder::build()` with an input shape that triggers boostvoronoi's `robust_fpt::is_finite()` predicate in the current build.

**Synthetic-input proof that the guard works.** The companion test `voronoi_from_segments_predicate_panic_fires_on_synthetic_input` (`crates/slicer-core/tests/voronoi_panic_regression.rs`) constructs three segments at `i64::MAX` whose cross-product arithmetic overflows `i64` inside boostvoronoi's `predicate::pss`. The test explicitly enables the `slicer-core` feature `voronoi-panic-regression`, which enables `boostvoronoi/console_debug` only for this target; without that feature the target is not built. With it enabled, the synthetic input panics inside the builder, the `catch_unwind` arm dispatches `Err(VoronoiError::PredicatePanic { segment_count: 3, min_x: i64::MAX - 1, max_x: i64::MAX, min_y: 0, max_y: 1_000_000_000, has_duplicate_endpoint: true, has_zero_length_segment: false, has_near_collinear_pair: false })`, and the test asserts every field on the returned variant — proving the catch arm dispatches correctly and the diagnostic captures the input shape exactly. The test passes 1/1 under the explicit feature.

The variant shape is also pinned independently by `voronoi_error_predicate_panic_field_population`, which constructs `VoronoiError::PredicatePanic { .. }` directly and asserts the `Display` impl includes `segment_count`, all three boolean flags, and the coordinate bounds — together proving the diagnostic is well-formed and triagers can read it without re-running the slice.

**Raw panic count (recorded as a datum, not a pass/fail gate):** `rg -c 'fpv_?\.is_finite|assertion failed.*is_finite|panicked at' target/183-parity.log` returned `0 raw panic lines`. `catch_unwind` does not suppress the default panic hook, so raw panic lines may be printed for caught panics as well as uncaught panics. They are noise/count datum only: this count cannot distinguish catch status or attribute a panic to a call site, and is not a floor for panics escaping the guard.

## Input characterization

Owning layer/region IDs: none (caught panic count = 0).

**Captured from the synthetic-input test in `crates/slicer-core/tests/voronoi_panic_regression.rs`** (the production workload emitted no diagnostic — see `## Caught panic count`):

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

**Debug-profile comparison: unchanged.** Post-guard 3/3 passed vs Step 1 debug-profile baseline 3/3 passed.

| Metric | Step 1 debug-profile baseline (`target/183-attribution.log`) | Post-guard debug-profile run (`target/183-parity.log`) | Delta |
| --- | --- | --- | --- |
| `test result:` | `ok. 3 passed; 0 failed; 0 ignored; 0 measured; 250 filtered out; finished in 35.19s` | `ok. 3 passed; 0 failed; 0 ignored; 0 measured; 250 filtered out; finished in 32.70s` | none (faster wall-clock from cold-cache rebuild) |
| Raw panic lines | 0 | 0 | none |
| Caught builder panics (`PredicatePanic` diagnostics) | n/a (guard absent) | 0 | n/a |
| Per-test status | all three `ok` | all three `ok` | none |

The three tests — `perimeter_parity::arachne_outer_wall_boundary_type_survives_wasm_boundary`, `perimeter_parity::annulus_true_hole_produces_inner_perimeters`, `perimeter_parity::arachne_perimeter_parity` — retained their individual pass status in the debug-profile comparison. The release baseline is a separate cross-check and has no release post-guard counterpart, so no release before/after comparison is claimed. Geometry comparison: not applicable. The caught panic count is 0 and no affected computation was observed; therefore there is no affected layer/region or meaningful wall-loop geometry delta to compare. Identical suite status is not evidence of identical geometry. The Step 3 regression test (`voronoi_from_segments_degenerate_input_returns_result_not_panic`) is not part of the `perimeter_parity` filter and so does not appear in either log; it lives outside the comparison surface and is verified independently.

AC-2's evidence is explicit: the debug-profile suite status is unchanged at 3/3 versus 3/3, the release baseline has no post-guard counterpart, and geometry comparison is not applicable because the caught panic count is zero and no affected computation was observed. The `## Baseline`, `## Caught panic count`, and `## Suite status vs baseline` sections are present; raw panic lines remain a recorded datum rather than a generic stderr assertion.

## Verdict

**No panicking computation was observed in the measured workload, so no live geometry was shown discarded. Unseen degenerate inputs remain unhardened and are not covered by this closure claim.** The `perimeter_parity` workload emits zero `is_finite()` panics in both the pre-guard (`## Baseline` section) and the post-guard (`## Caught panic count` section) runs; the suite passes 3/3 in both.

The synthetic-input test `voronoi_from_segments_predicate_panic_fires_on_synthetic_input` (`crates/slicer-core/tests/voronoi_panic_regression.rs`) constructs three segments at `i64::MAX` whose cross-product arithmetic overflows `i64` inside boostvoronoi's `predicate::pss`. The explicitly invoked `voronoi-panic-regression` feature enables `boostvoronoi/console_debug` for this target only, so the panic path is compiled in without leaking diagnostics into workspace test binaries. The synthetic input panics, the `catch_unwind` arm dispatches `Err(VoronoiError::PredicatePanic { .. })` with the segment-set characterization populated exactly (asserted field-by-field in the test), and the calling thread does not unwind. The companion test `voronoi_error_predicate_panic_field_population` independently pins the variant's `Display` shape. Together these tests prove the guard's dispatch and the variant's contract end-to-end. Removing the production catch arm was also tested: the synthetic target failed because the builder panic escaped the guarded call.

**Disposition for D-167: Closed for the measured workload, not as a general degenerate-input hardening claim.** The trigger documented in the D-160 session (`13 background-thread panics of the form 'rhs.fpv_.is_finite()'`) did not reproduce on this tree in debug or release mode. The guard added in this packet is proven to catch the synthetic builder panic and surface it as `VoronoiError::PredicatePanic`; unseen degenerate inputs remain unhardened and are outside this closure claim.

**No successor packet for `preprocess_input_outline` hardening is filed.** The measured workload did not produce an affected computation, so it neither demonstrated geometry loss nor established geometry equivalence. Future arachne work that hits the trigger again will surface it via the new typed error and can reopen D-167 with concrete inputs; unseen degenerate inputs are not covered by this closure.

## workspace Voronoi-debug allocation issue workspace gate

The required historical reproduction at packet 183's commit `b903620c`
returned exit 173 at
`cube_4color_gcode_output_tdd::mmu_no_oversized_alloc_repeat`, with the exact
single allocation `1,744,830,464` bytes (1.625 GiB).

After the remediation, `cargo xtask test --summary --workspace` reached the
executor normally: its summary reported **194 passed, 0 failed**, and the log
contained no `OOM-GUARD TRIPPED` or `requested SINGLE allocation` marker. The
overall workspace run still returned 101, but for an unrelated existing unit
failure: `layer_collection_builder_tdd::macro_drain_invokes_host_get_ordered_entities_exactly_once`
asserted `got 5`, expected `1`. That **got 5** observation and its isolated
reproduction are the resolved pre-ceremony fixture regression. After restoring
the macro-based guest, the current targeted result is exactly 1 call and passes;
the historical failure was not caused by this remediation. The improved summary
printed the failing test and panic block instead of only `VERDICT: FAIL`.

## Independent workspace gate blocker

Clean HEAD reproduces `support-surface-ironing --test ironing_tdd` with 11 tests: 5 passed and 6 failed: `flow_rate_applied`, `paths_at_correct_z`, `rectilinear_pattern`, `spacing_affects_density`, `square_region_produces_paths`, and `width_matches_config`.

These failures are outside packet 183 and remain after the packet's targeted checks pass. The traced cause is that shared fixtures set `ironing_enabled` and `infill_areas`, but `SupportSurfaceIroning::run_support_postprocess` reads `region.polygons()`, sees an empty collection, and skips `fill_expolygon`, producing no paths.

This needs its own fixture/contract packet and is not fixed or waived by packet 183.

## workspace Voronoi-debug allocation issue abort-path evidence

The exact test `pnp_cli_rebuild_abort_is_nonzero_with_named_failure_detail` in `xtask/src/test.rs` passes under `cargo test -p xtask` (41/41 green). It proves that the synthetic controlled runner returns a nonzero status for the simulated `pnp_cli` rebuild abort and reports a named failure detail. This is controlled-runner evidence, not a real `pnp_cli` kill.
