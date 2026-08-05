---
status: implemented
packet: 90
task_ids: [TASK-240]
---

# 90_regression-wedge-stl-swap

## Goal

Retire `resources/benchy.stl` (11,289,384 bytes ≈ 10.77 MB, ~200k triangles) as a test fixture by authoring `resources/regression_wedge.stl` (≤ 50 KB, ~200 triangles, deliberate feature inventory: 40 mm tall body, 45° overhang on one side, flat top ≥ 25 × 25 mm, flat bottom ≥ 25 × 25 mm, 10 mm bridge gap on the front face, ironable top section ≥ 25 × 25 mm), migrating every live-code reference that currently consumes `benchy.stl` to consume `regression_wedge.stl` (renaming `crates/slicer-runtime/tests/e2e/benchy_end_to_end_tdd.rs` → `slice_end_to_end_tdd.rs` with function-prefix sweep `benchy_*` → `slice_*` or `wedge_*` and the harness-`mod` update in `crates/slicer-runtime/tests/e2e/main.rs:12`), updating the 5 known non-test reference sites (`crates/slicer-runtime/tests/common/slicer_cache.rs:135`, `crates/slicer-model-io/tests/stl_roundtrip_tdd.rs:15-17`, `crates/slicer-runtime/tests/integration/live_module_loading_tdd.rs:332`, `crates/pnp-cli/tests/slice_instrumentation_fork_tdd.rs:32`, `modules/core-modules/support-planner/tests/orca_parity_tdd.rs`), and deleting `resources/benchy.stl` so the workspace test bench wall-clock drops by the dominant multi-minute share that benchy slicing currently consumes.

> Shell context: all pipe-suffixed acceptance commands target the **Bash tool** (POSIX) per `CLAUDE.md` §"Environment". On Windows hosts they must be executed via the Bash tool, not PowerShell.

## Problem Statement

`resources/benchy.stl` is the canonical real-mesh end-to-end test fixture for `crates/slicer-runtime`. It is consumed by **42** `#[test]` functions in `crates/slicer-runtime/tests/e2e/benchy_end_to_end_tdd.rs` (declared as `mod benchy_end_to_end_tdd;` at `crates/slicer-runtime/tests/e2e/main.rs:12`) plus **5 reference sites** in:
- `crates/slicer-runtime/tests/common/slicer_cache.rs:135`
- `crates/slicer-model-io/tests/stl_roundtrip_tdd.rs:1,15-17` (the line-1 reference is a doc-comment)
- `crates/slicer-runtime/tests/integration/live_module_loading_tdd.rs:332`
- `crates/pnp-cli/tests/slice_instrumentation_fork_tdd.rs:32`
- `modules/core-modules/support-planner/tests/orca_parity_tdd.rs` (line resolved at Step 4)

The fixture is **11,289,384 bytes** (≈ 10.77 MB) on disk and roughly 200,000 triangles. It produces three problems:

1. **Wall-clock dominance.** Slicing benchy through the full pipeline takes single-digit seconds even on warm caches; cold-cache cost is multi-second per `cached_run` cache key. With 29 distinct cache keys across the e2e bucket, cold-bench wall time crosses several minutes.
2. **Unprincipled assertion surface.** The roadmap audit classifies the 42 tests into 22 CLI-SHAPE (assert exit 0 + output written + byte-identical across runs — any real STL works), 17 SHAPE-DEPENDENT (assert markers like `;TYPE:Top surface`, `;TYPE:Bridge`, `;TYPE:Ironing`, retract-pair counts, layer count > 100 — needs a real-shape mesh), and 3 STRUCTURAL (fixture-independent). The 17 SHAPE-DEPENDENT tests assert that benchy's accidental geometry produces these markers; they would assert the same against any mesh deliberately engineered to have a top surface, a bridge, and an ironable top.
3. **Storage bloat.** 11 MB of vendored mesh data in the repository for tests that only need the *shape features*, not the *aesthetic of a benchy*.

This packet retires benchy.stl by authoring `regression_wedge.stl` — a small, deterministic mesh with each shape feature deliberately engineered — and migrating every test that touches benchy.stl. The wedge feature inventory is:
- 40 mm tall (gives ≥ 180 layers at default 0.2 mm layer height — comparable to benchy's > 100).
- 45° overhang on one side (for overhang-classification tests).
- Flat top ≥ 25 × 25 mm (for top-surface marker tests + ironing tests).
- Flat bottom ≥ 25 × 25 mm (for first-layer / brim / skirt tests).
- Horizontal bridge gap ≥ 10 mm wide on the front face (for bridge marker tests).
- Ironable top section ≥ 25 × 25 mm (overlaps with flat top — the same flat top region serves both assertions).

## Architecture Constraints

- This packet edits test sources and adds one binary resource. No path under `wit/**`, `crates/slicer-macros/**`, `crates/slicer-sdk/**`, `crates/slicer-ir/**`, `crates/slicer-schema/**`, or any `modules/core-modules/*/src/**` is touched. The `wasm-staleness` snippet does **not** apply.
- No path that participates in geometry math (polygon ops, mesh ops, mm↔unit conversion) is touched — the wedge STL is *consumed* by tests that already exercise those paths, but the paths themselves are not edited. The `coord-system` snippet does **not** apply.
- Determinism constraint: the wedge STL must be byte-identical across regenerations. Binary STL files include no inherent non-determinism (no timestamps in the format), so any non-determinism would come from the authoring tool. The closure log MUST document the authoring tool + parameters so future regeneration is reproducible.
- File-rename constraint: rename + body edits + mod-declaration update land in one Git commit so `git log --follow` is accurate.

## Data and Contract Notes

- IR or manifest contracts touched: none.
- WIT boundary considerations: none.
- Determinism or scheduler constraints: the wedge STL must be byte-identical across runs of the same authoring procedure. STL binary format has no inherent non-determinism; the authoring tool is the only source.
- Wall-clock contract: AC-7's "≥ 60 seconds shorter" is a conservative floor; the roadmap estimates multi-minute reduction. If the measured improvement is less than 60 seconds, investigate before declaring AC-7 satisfied — likely cause is a cache-key-construction bug where tests still touch benchy.stl indirectly via a stale `cached_run` key.

## Locked Assumptions and Invariants

- **Wedge feature inventory is the contract**: `regression_wedge.stl` MUST contain a 40 mm-tall body, a 45° overhang on one side, a flat top ≥ 25 × 25 mm, a flat bottom ≥ 25 × 25 mm, a horizontal bridge gap ≥ 10 mm wide on the front face, and an ironable top section ≥ 25 × 25 mm. Future re-authoring must preserve these features or update the assertion table in the migrated tests.
- **Test classification is the contract**: 22 CLI-SHAPE, 17 SHAPE-DEPENDENT, 3 STRUCTURAL per the roadmap audit. The function-prefix sweep follows this classification; deviating without updating the closure log produces silent assertion drift.
- **Determinism**: the wedge STL is byte-identical across regenerations using the documented procedure. If non-deterministic, the canonical SHA-256 is pinned in the closure log and verified at every regeneration.

## Risks and Tradeoffs

- **Risk: a SHAPE-DEPENDENT test asserts on a marker the wedge doesn't produce** (e.g., a specific perimeter count that benchy happens to produce but the wedge doesn't). Mitigation: the wedge feature inventory was chosen to cover every documented marker assertion class. If a test depends on a marker not in the inventory, re-engineer the wedge (preferred) or document why the test no longer applies in the closure log.
- **Risk: wall-clock improvement is smaller than estimated** because tests have non-fixture-bound costs (compilation, host-init). Mitigation: AC-7's 60-second floor is conservative; if achieved, the packet ships even if the absolute reduction is smaller than the roadmap's multi-minute estimate.
- **Tradeoff: assertion specificity vs. test generality.** A wedge is more deterministic than a benchy but covers a smaller geometry envelope. The 22 CLI-SHAPE tests are mesh-agnostic and unaffected; the 17 SHAPE-DEPENDENT tests gain stronger assertions per the marker inventory. The 3 STRUCTURAL tests are fixture-independent.
- **Tradeoff: STL format vs. 3MF.** Could the wedge be a 3MF for tighter integration with the paint-test fixtures? Rejected — `stl_roundtrip_tdd.rs` and `slice_instrumentation_fork_tdd.rs` specifically exercise the STL loader path; the migration must preserve format symmetry.
