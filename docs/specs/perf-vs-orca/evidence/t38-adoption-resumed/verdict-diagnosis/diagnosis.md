# T38 runtime-capture verdict diagnosis

**Diagnosis: confirmed summary-verdict false positive on recovered panic-hook output.** The recorded runtime capture says `perimeter_spatial_capture_and_nonvacuity ... ok` and `test result: ok. 1 passed; 0 failed`; the saved status also records outer exit `0`. The original summary nevertheless prints `VERDICT: FAIL` after listing `boostvoronoi` `panicked at` diagnostics.

`xtask::test::print_summary` (`xtask/src/test.rs`) sets `has_failures` when any bare panic line is found. `collect_bare_panics` accepts every line containing `panicked at` except a few formatting exclusions; it does not distinguish a panic caught by `catch_unwind` from an escaping process panic. In the original `--summary` invocation, the successful Cargo result therefore cannot yield `VERDICT: PASS` while these lines are present. This explains the discrepancy in the **failure-count signal**; it does not establish that the caught panics are harmless.

The replay command was run as requested and its complete output and exit are saved here. Important replay caveat: the `--summary-from` branch in `xtask::test::test_command` (`xtask/src/test.rs`) calls `print_summary(..., false)` and then returns `0`. Thus this replay necessarily prints `VERDICT: FAIL` even for a clean source log; the replay's verdict alone is not independent evidence of a test failure. It does confirm that the saved log contains the same panic-hook lines that the scanner emits as failure detail.

`medial_axis` (`crates/slicer-core/src/medial_axis.rs`) wraps the Boost.Voronoi interaction in `catch_unwind`; its caught-panic arm becomes `Err(())`, which the caller converts to `Ok(vec![])`. That control flow is consistent with panic-hook text surviving while the pipeline/test continues. **No claim is made that this empty medial-axis fallback is geometrically correct or canonical.**

The test is not vacuous for its spatial-query contract: `perimeter_spatial_capture_and_nonvacuity` (`crates/slicer-runtime/tests/integration/perimeter_spatial_capture.rs`) requires nonempty perimeter output, populated query/candidate counters, complete processed-region capture, and strict distance-candidate pruning against the unpruned scan total. It also exercises a qualifying `only_one_wall_top` input. These assertions do not require every source region to retain its complete geometry after an individual medial-axis fallback, so the green test result is not a geometry-correctness adjudication.

The ordinary `cargo xtask build-guests --check` returned exit `0`. The test command was ordinary (no xtask `--accelerated`); the integration harness substitutes the selected perimeter generator's native entry in the captured pipeline. No stale ordinary guest or mode mismatch is evidenced as the cause.

## Hypothesis outcomes

1. **Summary scans recovered panic-hook lines — confirmed as the cause of the original `VERDICT: FAIL`.**
2. **The test silently accepts no spatial work / is vacuous — not supported for the spatial-query assertions above.** It still does not certify every region's geometry or the fallback's correctness.
3. **Stale or mode-confused guests — not supported:** ordinary freshness exit `0`; no accelerated test mode was requested.

## Scope and evidence

- No test was rerun. No source, test, policy, reference, or lockfile was edited. No timing, dist build, or workspace test was run.
- Replay output and command exit: `runtime-summary-replay.log`, `runtime-summary-replay.exit`.
- Ordinary freshness output and command exit: `ordinary-guest-freshness.log`, `ordinary-guest-freshness.exit`.
- Original immutable capture and summary: `../gates/continuation-runtime-capture-full.log` and `../gates/continuation-runtime-capture-summary.log`.
- Narrow contract context read: `CONTEXT.md`; ADR-0042 (structural invariants over self-captured fixtures), ADR-0064 (test earns-its-keep standard), ADR-0065 (test-quality gate), and `docs/22_test_quality.md` (false-green taxonomy). This diagnosis makes no OrcaSlicer geometry claim.
