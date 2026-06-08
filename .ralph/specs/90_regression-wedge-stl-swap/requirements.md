# Requirements: 90_regression-wedge-stl-swap

## Packet Metadata

- Grouped task IDs:
  - `TASK-240` — Retire benchy.stl (11 MB) and ship regression_wedge.stl (≤ 50 KB).
- Backlog source: `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P0b — benchy.stl → regression_wedge.stl swap"
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

`resources/benchy.stl` is the canonical real-mesh end-to-end test fixture for `crates/slicer-runtime`. It is consumed by 42 tests in `crates/slicer-runtime/tests/e2e/benchy_end_to_end_tdd.rs` plus 4 reference sites in `crates/slicer-runtime/tests/common/slicer_cache.rs`, `crates/slicer-model-io/tests/stl_roundtrip_tdd.rs`, `crates/slicer-runtime/tests/integration/live_module_loading_tdd.rs`, and `crates/pnp-cli/tests/slice_instrumentation_fork_tdd.rs`. The fixture is 11 MB on disk and roughly 200,000 triangles. It produces three problems:

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

## In Scope

- Author `resources/regression_wedge.stl` deterministically (≤ 50 KB; engineered feature inventory above).
- Rename `crates/slicer-runtime/tests/e2e/benchy_end_to_end_tdd.rs` → `crates/slicer-runtime/tests/e2e/slice_end_to_end_tdd.rs`.
- Function-prefix sweep: `benchy_*` → `slice_*` for CLI-SHAPE tests; `benchy_*` → `wedge_*` for SHAPE-DEPENDENT tests where the assertion targets a wedge feature. STRUCTURAL tests use whichever prefix reads more naturally.
- Update the mod-declaration file `crates/slicer-runtime/tests/e2e.rs` (or equivalent harness file) to declare the renamed module.
- Update the 4 known non-test references: `crates/slicer-runtime/tests/common/slicer_cache.rs:135`, `crates/slicer-model-io/tests/stl_roundtrip_tdd.rs:15-17`, `crates/slicer-runtime/tests/integration/live_module_loading_tdd.rs:332`, `crates/pnp-cli/tests/slice_instrumentation_fork_tdd.rs:32`.
- Sweep for residual `benchy\.stl` substrings under `crates/`, `modules/`, `docs/`, `.ralph/`.
- Delete `resources/benchy.stl`.
- Document the authoring procedure for `regression_wedge.stl` in the closure log (source, byte SHA-256, regeneration instructions).
- Measure and record before/after wall-clock for `cargo test -p slicer-runtime` (cold cache).

## Out of Scope

- 3MF fixture retirement (`benchy_4color.3mf`, `benchy_painted.3mf`) — packet 89 (P0a).
- Paint pipeline changes — P1a onwards.
- Adding new SHAPE-DEPENDENT tests beyond those that already exist in `benchy_end_to_end_tdd.rs`.
- Changing any production code under `crates/*/src/`.
- Adding `regression_wedge.stl`-based fixtures to crates other than the four already documented.

## Authoritative Docs

- `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P0b" — scope and feature inventory.
- `crates/slicer-runtime/tests/e2e/benchy_end_to_end_tdd.rs` — read only test bodies (range-read by test name; file is likely > 500 lines).
- `docs/06_e2e_testing.md` if it exists — read only sections relevant to the CLI-SHAPE / SHAPE-DEPENDENT / STRUCTURAL classification convention.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- None. The wedge is engineered to satisfy pinch_n_print's existing assertion classes, not to mirror an OrcaSlicer fixture.

## Acceptance Summary

- Positive cases: `AC-1` through `AC-8` from `packet.spec.md`. Refinement: AC-5's ironing marker is asserted on a specific layer range computed from the wedge's height — the wedge produces ironing at the top section (last few layers above the 25 × 25 mm flat top region); the assertion should target that range, not a global "any ironing pass" check.
- Negative cases: `AC-N1` (no silently weakened assertions), `AC-N2` (determinism across regenerations), `AC-N3` (≤ 50 KB).
- Cross-packet impact: independent of packet 89; compounding wall-clock benefit when both land.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo check --workspace --all-targets` | Workspace compiles after migrations | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | No lint warnings introduced | FACT pass/fail |
| `cargo test -p slicer-runtime --test e2e slice_end_to_end 2>&1 \| tee target/test-output.log` | AC-4, AC-5 — migrated 42 tests pass | FACT pass/fail, SNIPPETS on failure |
| `cargo test -p slicer-model-io --test stl_roundtrip_tdd 2>&1 \| tee target/test-output.log` | AC-6 — STL round-trip test passes | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration live_module_loading 2>&1 \| tee target/test-output.log` | AC-6 — live module loading test passes | FACT pass/fail |
| `cargo test -p pnp-cli --test slice_instrumentation_fork_tdd 2>&1 \| tee target/test-output.log` | AC-6 — CLI instrumentation test passes | FACT pass/fail |
| `rg -n --glob '!.ralph/specs/90_regression-wedge-stl-swap/**' 'benchy\.stl' crates/ modules/ docs/ .ralph/ ; test $? -eq 1` | AC-3 — zero residual references | FACT pass/fail |
| `test ! -f resources/benchy.stl && test -f resources/regression_wedge.stl && [ $(wc -c < resources/regression_wedge.stl) -le 51200 ]` | AC-1, AC-2 — files in correct state | FACT pass/fail |
| `cargo clean -p slicer-runtime && time cargo test -p slicer-runtime 2>&1 \| tee target/test-output.log` | AC-7 — wall-clock measurement | FACT with elapsed time; record in closure log |

## Step Completion Expectations

- Step 1 (wedge authoring) MUST produce a deterministic file before Step 2 begins. If the authoring is non-deterministic (e.g., timestamps in the binary STL), the procedure must be revised before continuing.
- Step 2 (function-prefix sweep) is performed in the same Git commit as the file rename so `git log --follow` tracks the migration as a coherent unit.
- Step 5 (`benchy.stl` deletion) must NOT run until every other step is green. If a residual reference is found post-delete, the deletion is rolled back via `git checkout HEAD~1 resources/benchy.stl` (or the equivalent) and the missing reference fixed.
- AC-N1 review (assertion-diff) is performed during Step 2 before commit, NOT post-hoc — review-after-commit creates a temptation to gloss over weakened assertions.

## Context Discipline Notes

- `crates/slicer-runtime/tests/e2e/benchy_end_to_end_tdd.rs` may exceed 600 lines (42 tests, each ~10-30 lines + helpers). Read by test name, not in full. Use `Grep -n '^#\[test\]'` to locate test boundaries, then `Read` with `offset`/`limit` around each test of interest.
- The wedge STL is a binary file; never `Read` it. Delegate any structural inspection (face count, bounding box) to a sub-agent that runs a parser.
- `resources/benchy.stl` is 11 MB; under no circumstances `Read` it directly. It's only deleted, not inspected.
