# Requirements: 89_benchy-3mf-retirement

## Packet Metadata

- Grouped task IDs:
  - `TASK-239` — Benchy 3MF fixture retirement (replaces benchy_4color.3mf + benchy_painted.3mf with cube fixtures from cherry-pick 5c272ef)
- Backlog source: `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P0a — Benchy 3MF retirement"
- Packet status: `active`
- Aggregate context cost: `M`

## Problem Statement

`resources/benchy_4color.3mf` (2.6 MB, multi-material benchy with arbitrary paint distribution) and `resources/benchy_painted.3mf` (2.5 MB, painted benchy with mixed paint semantics) are consumed by ~33 tests across `crates/slicer-runtime/tests/e2e/`, `crates/slicer-model-io/tests/`, and `crates/slicer-runtime/tests/common/`. Each test consumes the model through `cached_load_model` / `cached_run`, but the fixtures contribute three concrete problems:

1. **Test wall-clock penalty.** Even with the per-process cache at `crates/slicer-runtime/tests/common/model_cache.rs`, first-touch parse + slice on a 2.5 MB 3MF dominates the e2e bucket cold-cache wall time. The cache header comment explicitly motivates itself with these files as the cost example.
2. **Opaque paint geometry.** The benchy paint is hand-painted in OrcaSlicer with no per-face determinism — tests can only assert weak "some color exists" or "g-code differs from unpainted" properties. They cannot say "cube face +X is ToolIndex 1 between layers 4 and 6", which is the kind of assertion the paint-segmentation port (P3, P4) needs to verify.
3. **Storage bloat.** 5.2 MB of opaque-geometry 3MFs sit in the repo. Cherry-pick `5c272ef970fee2b861081799169a3ddb87e179c9` landed `resources/cube_4color.3mf` (37 KB) and `resources/cube_fuzzyPainted.3mf` (27 KB) — engineered cubes with each face deliberately assigned a known paint semantic — plus 24 deterministic RED tests in `cube_4color_paint_tdd.rs` + `cube_fuzzy_painted_tdd.rs`. The migration was not done by that cherry-pick; this packet finishes it.

This packet does not change any production code in `crates/slicer-core`, `crates/slicer-ir`, or `crates/slicer-runtime/src/`. It is a fixture-and-test migration.

## In Scope

- Audit each test in `crates/slicer-runtime/tests/e2e/benchy_4color_modifier_part_e2e_tdd.rs`, `…/e2e/benchy_painted_e2e_tdd.rs`, `…/e2e/benchy_painted_overrides_e2e_tdd.rs`, `…/e2e/threemf_fixture_e2e_tdd.rs` (lines 540-924), `crates/slicer-runtime/tests/integration/threemf_paint_drop_on_modifier_tdd.rs`, `…/integration/threemf_transform_tdd.rs`, `crates/slicer-model-io/tests/model_loader_tdd.rs`, `…/threemf_sidecar_classification_tdd.rs` per the test classification in the roadmap (STRUCTURAL / CLI-SHAPE / SHAPE-DEPENDENT).
- Rewrite each test body to consume the appropriate cube fixture (`cube_4color.3mf` for multi-color tests; `cube_fuzzyPainted.3mf` for fuzzy-skin paint tests; `cube_cilindrical_modifier.3mf` and `cube_rotated_component.3mf` if authored).
- Rename each migrated file from `benchy_*` to `cube_*` (the `cube_*` prefix matches the existing RED tests' naming convention).
- Strengthen SHAPE-DEPENDENT assertions to per-face cube assertions wherever the cube fixture's known per-face paint allows.
- Author `resources/cube_cilindrical_modifier.3mf` (small derivative of `cube_4color.3mf` adding one modifier-part component) only if no existing cube fixture can substitute.
- Author `resources/cube_rotated_component.3mf` (small derivative adding one 45° rotated component) only if no existing cube fixture can substitute.
- Update the doc-comment block at `crates/slicer-runtime/tests/common/model_cache.rs:5-8` to reference the cube fixtures (the cache code itself is unchanged).
- Delete `resources/benchy_4color.3mf`, `resources/benchy_painted.3mf`, `resources/benchy_painted.README.md`.
- Sweep for residual `benchy_4color` / `benchy_painted` substrings under `crates/`, `modules/`, `docs/`, `.ralph/`.

## Out of Scope

- `resources/benchy.stl` retirement — that is packet P0b (90).
- Paint-segmentation kernel changes — P3 (95) territory.
- `MeshSegmentation` host kernel wiring — P2 (94) territory.
- `RegionMapping` cross-product expansion — P1c (93) territory.
- Test cache implementation changes (only the header doc-comment is edited, not the cache code).
- Any test currently passing on the cube fixtures (`cube_4color_paint_tdd.rs`, `cube_fuzzy_painted_tdd.rs`) — these are the migration *targets*, not migration *subjects*.
- Removing or rewriting RED tests in `cube_4color_paint_tdd.rs` / `cube_fuzzy_painted_tdd.rs` — they stay RED until P3/P4 flip them GREEN.

## Authoritative Docs

- `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P0a — Benchy 3MF retirement" (~50 lines; read directly) — packet scope and per-file migration table.
- `docs/specs/orca-paint-segmentation-parity.md` §9 "Test Strategy" (lines ~942-980) — context on why cube fixtures replace benchy (read this section only; the file is ~1021 lines so delegate any other-section read). There is no §"Fixture Strategy" heading.
- `crates/slicer-runtime/tests/common/model_cache.rs` (47 lines; read in full) — the cache module whose doc-comment is updated.

No `docs/01`–`docs/08` references are needed; this packet does not alter contract or architecture.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- None. This packet does not borrow algorithmic behavior from OrcaSlicer. (The fixtures it migrates *to* — the cube 3MFs — already encode the parity assumptions baked in by the cherry-pick.)

## Acceptance Summary

- Positive cases: `AC-1` through `AC-10` from `packet.spec.md`. Strengthened-assertion refinement: every SHAPE-DEPENDENT test migrated from `benchy_4color_modifier_part_e2e_tdd.rs` now asserts on a specific cube face's known `PaintSemantic::Material(ToolIndex(N))` value where the cube paints are deterministic (face +X = ToolIndex 1, face -X = ToolIndex 2, face +Y = ToolIndex 3, face -Y = ToolIndex 4 per the cube_4color authoring convention recorded in `docs/specs/orca-paint-segmentation-parity.md`).
- Negative cases: `AC-N1` (no silently weakened assertions), `AC-N2` (≤ 100 KB cap on any newly-authored fixture).
- Cross-packet impact: P0b is independent; P1a onwards unblocked (none depend on P0a explicitly but the wall-clock improvement compounds).

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo check --workspace --all-targets` | Workspace compiles after renames | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | No lint warnings introduced | FACT pass/fail |
| `cargo test -p slicer-runtime --test e2e cube_4color_modifier_part 2>&1 \| tee target/test-output.log` | AC-3 — migrated modifier-part tests pass | FACT pass/fail, SNIPPETS on failure (≤ 20 lines around `FAILED`) |
| `cargo test -p slicer-runtime --test e2e cube_painted 2>&1 \| tee target/test-output.log` | AC-4 — migrated painted-e2e tests pass | FACT pass/fail |
| `cargo test -p slicer-runtime --test e2e threemf_fixture 2>&1 \| tee target/test-output.log` | AC-5 — threemf_fixture_e2e tests pass after edits | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration threemf_paint_drop_on_modifier 2>&1 \| tee target/test-output.log` | AC-6 — paint-drop-on-modifier passes (file lives in tests/integration/) | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration threemf_transform 2>&1 \| tee target/test-output.log` | AC-7 — transform tests pass (file lives in tests/integration/) | FACT pass/fail |
| `cargo test -p slicer-model-io --test model_loader_tdd 2>&1 \| tee target/test-output.log` | AC-8a — model_loader tests pass | FACT pass/fail |
| `cargo test -p slicer-model-io --test threemf_sidecar_classification_tdd 2>&1 \| tee target/test-output.log` | AC-8b — sidecar tests pass | FACT pass/fail |
| `rg -n --glob '!.ralph/specs/**' 'benchy_4color\.3mf\|benchy_painted\.3mf' crates/ modules/ docs/ .ralph/ ; test $? -eq 1` | AC-2 — zero residual references (sibling spec packets under `.ralph/specs/**` excluded since they may reference retired basenames in their own scope) | FACT pass/fail (exit 1 == no matches == pass) |
| `test ! -f resources/benchy_4color.3mf && test ! -f resources/benchy_painted.3mf && test ! -f resources/benchy_painted.README.md` | AC-1 — files deleted | FACT pass/fail |

All verification commands above are delegation-friendly: each is a single command with a binary exit code and small parseable output. `cargo test --workspace` is NOT a verification command for this packet — the migrated tests live in two crates (`slicer-runtime`, `slicer-model-io`) and the per-crate, per-bucket invocations are faster to dispatch and more diagnostic on failure.

## Step Completion Expectations

- Steps 1 and 2 (audit + classification) run before any code edit; if Step 1 reveals an assertion category not classified in the roadmap, escalate before proceeding rather than guess.
- Steps 3–6 (per-file rewrites) can land in any order internally, but the migrated file rename happens atomically with the assertion rewrites in the same Git commit so the rename history stays followable.
- Step 7 (file deletion) MUST run after all migrated tests pass — never delete the source fixtures before the migration is verified green.
- Step 8 (residual grep) is the cheapest falsifying check; if it returns any match outside this packet's own directory, the deletion is rolled back, the matching test fixed, and the step re-run.

## Context Discipline Notes

- `docs/specs/orca-paint-segmentation-parity.md` is ~1021 lines. The implementer should range-read only §"Fixture Strategy" (and any other section explicitly cited by an AC) — do NOT load the file in full.
- `crates/slicer-runtime/tests/e2e/threemf_fixture_e2e_tdd.rs` is the largest migration target (~1000+ lines, of which only lines 540-924 reference benchy_4color). Use a ranged `Read` (`offset: 540, limit: 384`) when editing that region; do not load the full file.
- The cube fixtures themselves (`resources/cube_4color.3mf`, `resources/cube_fuzzyPainted.3mf`) are binary 3MF files — never `Read` them. Dispatch any structural question (object count, face count, paint distribution) to a sub-agent that runs the appropriate test-helper extraction.
