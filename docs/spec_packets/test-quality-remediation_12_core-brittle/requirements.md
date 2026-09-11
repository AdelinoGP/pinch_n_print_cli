# Requirements: core-brittle

## Packet Metadata

- Grouped task IDs: `core/BRITTLE`
- Backlog source: `docs/specs/test-quality-remediation-plan.md`
- Packet status: `draft`
- Aggregate context cost: `S`

## Problem Statement

The §5.1 `BRITTLE` row names three timing guards that protect real 3D Benchy regressions (118s MeshAnalysis from per-facet incremental union, O(layers) overhang sweep, ~28s Slice from per-layer footprint recomputation) but assert wall-clock thresholds, which are machine-dependent and therefore brittle in CI. The plan disposition is KEEP-review: move to a benchmark or deterministic accounting, or justify retention — never drop a perf guard without replacement. This packet is the one coherent slice that runs that review to a recorded decision for all three guards without touching production code or any other core surface.

## In Scope

- Earn-their-keep review of `compute_xy_footprint_is_fast_for_thousands_of_disjoint_facets` (`mesh_analysis.rs`, absolute 3s gate + 1200-facet equality witness): retain the guard with its Benchy regression rationale or migrate it, plus the exact marker line `// KEEP-review (core-brittle): 118s Benchy regression guard; batched union measured 61ms vs 20.6s incremental (338x); 3s threshold keeps ~50x headroom.`; test name unchanged.
- Earn-their-keep review of `annotate_overhangs_is_fast_for_many_stacked_layers` (`overhang_annotation.rs`, absolute 1s gate + empty-result witness): same decision shape, plus the exact marker line `// KEEP-review (core-brittle): O(layers) sweep guard; timed region excludes one-time slicing setup; 1s threshold on 1200 pre-sliced layers.`; test name unchanged.
- Earn-their-keep review of `prepass_slice_caches_bottom_surface_footprint_across_layers` (`algo_prepass_slice_tdd.rs`, relative 1.5x cached-vs-uncached ratio gate): same decision shape, plus the exact marker line `// KEEP-review (core-brittle): per-layer footprint-recomputation guard; cached-vs-uncached ratio self-normalizes across machines; 1.5x minimum (measured ~3.5x debug / ~2.3x release).`; test name unchanged.
- Update only the §7 `core` Ledger row in `docs/specs/test-quality-remediation-plan.md` with this packet's dispositions, surviving names, oracle tokens, representative validation, and remaining gap.

## Out of Scope

- Any production behavior, threshold, signature, or performance change; thresholds (`from_secs(3)`, `from_secs(1)`, `* 3 < * 2`) are pinned, never loosened.
- New bench targets or files (the existing `benches/polygon_ops.rs` criterion target is evaluated read-only as a migration option, never edited).
- Production instrumentation for deterministic accounting (call counters would change production signatures; rejected as out of test-only scope).
- Paint/retire/strengthen/beading/geometry/wall/support/region/bridge/flow surfaces, all other `crates/slicer-core/tests/*` targets, the committed census manifest, and later queue rows.

## Authoritative Docs

- `docs/specs/test-quality-remediation-plan.md` - §5.1 BRITTLE row, §6 lib/integration feature-correct patterns, §7 ledger ownership, Queue row #12; small sections, direct read.
- `docs/22_test_quality.md` - earn-their-keep standard and legitimate weak forms; delegated SUMMARY.
- `docs/adr/0064-existing-tests-retire-if-unjustified.md` - retire-vs-keep burden of proof; delegated SUMMARY.
- `docs/adr/0065-test-quality-gate-with-delayed-enforce-mode.md` - report-mode closure for touched files; delegated SUMMARY.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-4`; AC-1 and AC-2 pin the lib guards (runtime pass plus threshold-literal pins), AC-3 pins the integration ratio guard under the feature-correct exact filter, AC-4 pins the accumulated ledger predicate.
- Negative: `AC-N1` pins the inherited 6-name integration roster (plus the optional retire 7th name behind the FORWARD-DEP) and the bare-feature zero-test control.
- Cross-packet impact: none; predecessor `core-paint` exports nothing and this packet exports nothing (no new API, file, fixture, or target).

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only 2-3 gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| AC-1 lib filter + `from_secs(3)` pin from `packet.spec.md` | footprint guard | FACT pass/fail |
| AC-2 lib filter + `from_secs(1)` pin from `packet.spec.md` | overhang guard | FACT pass/fail |
| AC-3 exact integration filter + ratio pin from `packet.spec.md` | cache ratio guard | FACT pass/fail |
| AC-4 python ledger predicate from `packet.spec.md` | ledger accumulation | prints `core-brittle ledger predicate: PASS` |
| AC-N1 roster python + bare-feature lib filter from `packet.spec.md` | roster + gating guard | FACT pass/fail |
| `cargo check --workspace --all-targets` | workspace type gate | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail |
| `cargo xtask check-literals` | struct-literal gate | FACT pass/fail |
| `cargo xtask check-test-quality --report crates/slicer-core/src/algos/mesh_analysis.rs crates/slicer-core/src/algos/overhang_annotation.rs crates/slicer-core/tests/algo_prepass_slice_tdd.rs` | report-mode quality gate, touched scope only | FACT findings list; touched tests expect zero unwaived |

Commands produce small parseable output suitable for delegation. Every `cargo test` tees to `target/test-output.log`; check/clippy use dedicated logs and never overwrite it. Findings are read from logs, never by re-running.

## Step Completion Expectations

Order is surface-grouped: lib footprint guard first, lib overhang guard second, integration ratio guard plus gate closure third, ledger row last preserving accumulated prior content. No step depends on another step's edited assertions; the only shared-file constraint is that Step 3 preserves the 6-name roster (plus the optional retire 7th name) in `algo_prepass_slice_tdd.rs`.

## Context Discipline Notes

Packet-specific hazards: all three source files exceed 300 lines, so every implementation read uses the stated ±40-line windows or a delegated dispatch; the temptation reads to skip are `crates/slicer-core/tests/*` beyond the single owned target, `crates/slicer-core/benches/*` beyond read-only migration evaluation, and `OrcaSlicerDocumented/` (no parity question here, never load). Libtest substring filters avoid pinning brittle full module paths.
