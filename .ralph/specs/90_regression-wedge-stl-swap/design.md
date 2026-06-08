# Design: 90_regression-wedge-stl-swap

## Controlling Code Paths

- Primary code paths: test sources only — `crates/slicer-runtime/tests/e2e/benchy_end_to_end_tdd.rs` (renamed), the 4 known non-test reference sites, and a single `mod` declaration in the harness file. No production source under `crates/*/src/` is modified.
- Neighboring tests or fixtures: `resources/cube_4color.3mf` and related cube fixtures (out of scope here — packet 89's territory) demonstrate the engineered-fixture pattern this packet follows for STL meshes.
- OrcaSlicer comparison surface: none.

## Architecture Constraints

- This packet edits test sources and adds one binary resource. No path under `wit/**`, `crates/slicer-macros/**`, `crates/slicer-sdk/**`, `crates/slicer-ir/**`, `crates/slicer-schema/**`, or any `modules/core-modules/*/src/**` is touched. The `wasm-staleness` snippet does **not** apply.
- No path that participates in geometry math (polygon ops, mesh ops, mm↔unit conversion) is touched — the wedge STL is *consumed* by tests that already exercise those paths, but the paths themselves are not edited. The `coord-system` snippet does **not** apply.
- Determinism constraint: the wedge STL must be byte-identical across regenerations. Binary STL files include no inherent non-determinism (no timestamps in the format), so any non-determinism would come from the authoring tool. The closure log MUST document the authoring tool + parameters so future regeneration is reproducible.
- File-rename constraint: rename + body edits + mod-declaration update land in one Git commit so `git log --follow` is accurate.

## Code Change Surface

- Selected approach: author the wedge first (the longest single task; gates every downstream step); then sweep tests in order of size (small reference sites first, then the 42-test file).
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - **Authored binary**: `resources/regression_wedge.stl`.
  - **Renamed test file**: `crates/slicer-runtime/tests/e2e/benchy_end_to_end_tdd.rs` → `slice_end_to_end_tdd.rs`; function-prefix sweep `benchy_*` → `slice_*` / `wedge_*` per the test classification.
  - **mod declaration**: `crates/slicer-runtime/tests/e2e.rs` — rename `mod benchy_end_to_end_tdd;` → `mod slice_end_to_end_tdd;`.
  - **Reference site 1**: `crates/slicer-runtime/tests/common/slicer_cache.rs:135` (the cache-key construction or doc-comment naming benchy.stl).
  - **Reference site 2**: `crates/slicer-model-io/tests/stl_roundtrip_tdd.rs:15-17` (the round-trip test's input STL path).
  - **Reference site 3**: `crates/slicer-runtime/tests/integration/live_module_loading_tdd.rs:332` (the live-module-loading integration test's input STL).
  - **Reference site 4**: `crates/pnp-cli/tests/slice_instrumentation_fork_tdd.rs:32` (the CLI instrumentation fork test's input STL).
  - **Deleted binary**: `resources/benchy.stl`.
- Rejected alternatives that were considered and why they were not chosen:
  - **Keep benchy.stl as a `#[ignore]` real-world regression mesh**: introduces residual surface area — every future test author has to decide whether to opt back into the slow fixture. Rejected.
  - **Use one of the existing cube STLs (extracted from `cube_4color.3mf`) instead of authoring a new wedge**: cubes have no overhang, no bridge, no ironable region; would fail every SHAPE-DEPENDENT marker assertion. Rejected.
  - **Author the wedge in parametric OpenSCAD checked into the repo, generating the STL at test time**: introduces a build-time dependency on OpenSCAD. Rejected — the STL is a one-time engineered artifact; the SCAD source (or equivalent) is documented in the closure log for future regeneration but is not a workspace dependency.

## Files in Scope (read + edit)

- `resources/regression_wedge.stl` — CREATE — role: replacement engineered mesh; expected change: new binary, ≤ 50 KB.
- `resources/benchy.stl` — DELETE.
- `crates/slicer-runtime/tests/e2e/benchy_end_to_end_tdd.rs` → `slice_end_to_end_tdd.rs` — role: 42 e2e tests; expected change: rename + function-prefix sweep + fixture-path swap.
- `crates/slicer-runtime/tests/e2e.rs` — role: harness mod declarations; expected change: single `mod` rename.
- `crates/slicer-runtime/tests/common/slicer_cache.rs` — role: cache module that references benchy.stl at line 135; expected change: one-line swap.
- `crates/slicer-model-io/tests/stl_roundtrip_tdd.rs` — role: STL round-trip test; expected change: input-path swap on lines 15-17.
- `crates/slicer-runtime/tests/integration/live_module_loading_tdd.rs` — role: live-module-loading integration test; expected change: one-line swap at line 332.
- `crates/pnp-cli/tests/slice_instrumentation_fork_tdd.rs` — role: CLI fork test; expected change: one-line swap at line 32.

Above the "≤ 3" target because this is a sweeping migration, but each file change is small in delta. The per-step plan in `implementation-plan.md` keeps each step to ≤ 3 files.

## Read-Only Context

- `docs/specs/paint-pipeline-orca-parity-roadmap.md` — read §"P0b" only (60-line section).
- `crates/slicer-runtime/tests/e2e/benchy_end_to_end_tdd.rs` — read by test name (range-read; file is large). Use the test-classification table in the roadmap to determine each test's category before editing.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — delegate parity checks (none expected for this packet).
- `target/`, `Cargo.lock`, generated code — never load.
- `resources/benchy.stl` — binary 11 MB; never `Read` directly.
- `resources/regression_wedge.stl` (after authoring) — binary; never `Read` directly; delegate any structural-inspection question.
- Any path under `crates/slicer-core/src/`, `crates/slicer-ir/src/`, `crates/slicer-runtime/src/` — this packet does not edit production source.

## Expected Sub-Agent Dispatches

- "Open `crates/slicer-runtime/tests/e2e/benchy_end_to_end_tdd.rs` and return a LOCATIONS list of every `#[test]` function: function name + 1-line summary of the assertion shape (CLI-SHAPE / SHAPE-DEPENDENT / STRUCTURAL)" — purpose: feed the function-prefix sweep in Step 2.
- "Run `cargo test -p slicer-runtime --test e2e slice_end_to_end`; return FACT pass/fail, SNIPPETS on failure (≤ 20 lines around the failing test's `FAILED` line)" — purpose: validate Step 2.
- "Run `wc -c < resources/regression_wedge.stl`; return FACT (single integer)" — purpose: AC-1 / AC-N3 size check.
- "Run `sha256sum resources/regression_wedge.stl`; return FACT (single hash)" — purpose: AC-8 determinism / closure-log record.
- "Run `rg -n 'benchy\.stl' crates/ modules/ docs/ .ralph/ --glob '!.ralph/specs/90_regression-wedge-stl-swap/**'`; return LOCATIONS or empty" — purpose: residual-reference sweep.

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

## Context Cost Estimate

- Aggregate: `M` (wedge authoring is the largest single task; the rest are mechanical sed-style edits).
- Largest single step: `M` (Step 1 — wedge authoring + verification it has the documented features).
- Highest-risk dispatch: the test-inventory LOCATIONS dispatch in Step 1 (must classify all 42 tests; FUTURE will diverge from the roadmap audit if any test was added between roadmap-write and now — the dispatch return must be cross-checked against the roadmap classification).

## Open Questions

- `[FWD]` — Is OpenSCAD (or another parametric CAD tool) available in the implementer's environment for wedge authoring? If not, the wedge may need to be authored externally and the STL imported as a one-shot artifact. Either is acceptable; the procedure is documented in the closure log regardless. Resolvable mid-flight.
- `[FWD]` — Are the 4 reference sites' line numbers (135, 15-17, 332, 32) still accurate at implementation time? They were captured from the roadmap; the implementer's first dispatch (residual-reference inventory via `rg`) confirms current line numbers and supersedes the cited numbers if the files have drifted.
