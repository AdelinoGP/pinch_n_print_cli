# Design: 89_benchy-3mf-retirement

## Controlling Code Paths

- Primary code paths: test sources only — `crates/slicer-runtime/tests/e2e/*.rs`, `crates/slicer-model-io/tests/*.rs`, and the doc-comment block at `crates/slicer-runtime/tests/common/model_cache.rs:1-13`. No source under `crates/*/src/` is modified.
- Neighboring tests or fixtures: `crates/slicer-runtime/tests/executor/cube_4color_paint_tdd.rs` and `crates/slicer-runtime/tests/executor/cube_fuzzy_painted_tdd.rs` (the RED test suites authored in cherry-pick `5c272ef`) are the *naming convention reference* — the migrated files take the `cube_*` prefix. These files themselves are NOT edited by this packet.
- OrcaSlicer comparison surface: none for this packet (fixture migration only).

## Architecture Constraints

- This packet edits only test sources and the test-fixture cache doc-comment. No path under `wit/**`, `crates/slicer-macros/**`, `crates/slicer-sdk/**`, `crates/slicer-ir/**`, `crates/slicer-schema/**`, or any `modules/core-modules/*/src/**` is touched. The `wasm-staleness` snippet does **not** apply (no guest WASM input is modified).
- No path that participates in geometry math (polygon ops, mesh ops, mm↔unit conversion) is touched. The `coord-system` snippet does **not** apply.
- Fixture authoring constraint: any newly-authored `.3mf` MUST be deterministic — same input authoring tool + same input vertices/triangles + same paint assignments must produce byte-identical output. The team's 3MF authoring helper (if used) is single-threaded and stable; document the authoring command in this packet's closure log so any future regeneration is reproducible.
- Constraint: no file rename happens without an accompanying assertion rewrite in the same Git commit. A bare rename followed by a separate edit destroys `git log --follow` accuracy on the assertion content.

## Code Change Surface

- Selected approach: file-by-file migration, ordered by ease (small files first) so most of the workspace is migrated before the trickiest STRUCTURAL test (the modifier-part 7-test file) is reached. Atomic per-file commit: rename + assertion rewrite + import-path update in the same commit.
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - **Renamed test files** (8 total): `benchy_4color_modifier_part_e2e_tdd.rs` → `cube_4color_modifier_part_e2e_tdd.rs`; `benchy_painted_e2e_tdd.rs` → `cube_painted_e2e_tdd.rs`; `benchy_painted_overrides_e2e_tdd.rs` → `cube_painted_overrides_e2e_tdd.rs`; the others (`threemf_*.rs`, `model_loader_tdd.rs`) keep their names — only their bodies change.
  - **Edited test files** (in-place body edits): `crates/slicer-runtime/tests/e2e/threemf_fixture_e2e_tdd.rs` (lines 540-924); `crates/slicer-runtime/tests/integration/threemf_paint_drop_on_modifier_tdd.rs`; `crates/slicer-runtime/tests/integration/threemf_transform_tdd.rs`; `crates/slicer-model-io/tests/model_loader_tdd.rs`; `crates/slicer-model-io/tests/threemf_sidecar_classification_tdd.rs`.
  - **Test-module registry**: `crates/slicer-runtime/tests/e2e.rs` (or whichever file declares `mod benchy_4color_modifier_part_e2e_tdd;` / similar) — rename the `mod` declarations.
  - **Doc-comment update**: `crates/slicer-runtime/tests/common/model_cache.rs` lines 5-8 (the motivating-example block).
  - **Deleted files**: `resources/benchy_4color.3mf`, `resources/benchy_painted.3mf`, `resources/benchy_painted.README.md`.
  - **Possibly-authored fixtures** (only if existing cube assets cannot substitute): `resources/cube_with_modifier_part.3mf`, `resources/cube_rotated_component.3mf`.
- Rejected alternatives that were considered and why they were not chosen:
  - **Keep one benchy fixture as a "manual regression" with `#[ignore]`**: adds residual surface area for future drift; the wedge STL (P0b) plus the cube 3MFs together cover every assertion class the benchy files were carrying. Rejected.
  - **Author a single new 3MF that combines modifier-part + rotated component + multi-color paint**: violates the "minimum-feature fixture per test" principle. Each derived cube fixture exercises exactly one feature, mirroring the regression_wedge.stl design choice in P0b. Rejected.
  - **Symlink old benchy basenames to new cube fixtures during a transition window**: hides the migration in history and makes `rg benchy` reports lie. Rejected — full delete only.

## Files in Scope (read + edit)

The 8 renamed/edited test files plus the cache doc-comment block. Listed below grouped by ownership:

- `crates/slicer-runtime/tests/e2e/benchy_4color_modifier_part_e2e_tdd.rs` → renamed to `cube_4color_modifier_part_e2e_tdd.rs`; role: 7 STRUCTURAL + SHAPE-DEPENDENT modifier-part tests; expected change: full body rewrite to consume cube fixtures.
- `crates/slicer-runtime/tests/e2e/benchy_painted_e2e_tdd.rs` → renamed to `cube_painted_e2e_tdd.rs`; role: 2 painted-vs-unpainted tests; expected change: body rewrite + assertion strengthening.
- `crates/slicer-runtime/tests/e2e/benchy_painted_overrides_e2e_tdd.rs` → renamed to `cube_painted_overrides_e2e_tdd.rs`; role: 1 paint-overrides CLI test; expected change: body rewrite.
- `crates/slicer-runtime/tests/e2e/threemf_fixture_e2e_tdd.rs` (lines 540-924); role: 13 STRUCTURAL refs to benchy_4color; expected change: in-range body edits.
- `crates/slicer-runtime/tests/integration/threemf_paint_drop_on_modifier_tdd.rs`; role: 1 STRUCTURAL test; expected change: body edit.
- `crates/slicer-runtime/tests/integration/threemf_transform_tdd.rs`; role: 4 STRUCTURAL transform refs; expected change: body edits.
- `crates/slicer-model-io/tests/model_loader_tdd.rs`; role: 6 STRUCTURAL refs in a 37-test file; expected change: targeted in-place edits.
- `crates/slicer-model-io/tests/threemf_sidecar_classification_tdd.rs`; role: 9 STRUCTURAL refs; expected change: body edits.
- `crates/slicer-runtime/tests/common/model_cache.rs` lines 1-13; role: doc-comment with motivating example; expected change: replace benchy basenames with cube basenames + correct sizes.
- `crates/slicer-runtime/tests/e2e.rs` (mod-declaration file; small); role: harness mod declarations; expected change: rename `mod benchy_*` to `mod cube_*`.
- `resources/benchy_4color.3mf`, `resources/benchy_painted.3mf`, `resources/benchy_painted.README.md` — DELETE.
- `resources/cube_with_modifier_part.3mf`, `resources/cube_rotated_component.3mf` — CREATE only if necessary.

Total: ≤ 12 primary edit targets (9 renames/edits + 3 deletes + 0–2 fixture creations). Above the "≤ 3" target because this is a sweeping fixture migration; each file is small in delta (one to a few `replace_all` sed-style operations); the per-step plan in `implementation-plan.md` breaks the work into atomic units no single step touches more than 3 files.

## Read-Only Context

- `docs/specs/paint-pipeline-orca-parity-roadmap.md` — read §"P0a — Benchy 3MF retirement" only (file is ~932 lines; range-read).
- `docs/specs/orca-paint-segmentation-parity.md` — read §9 "Test Strategy" only (lines ~942-980; file is ~1021 lines; delegate any other-section read). There is no §"Fixture Strategy" heading; ground-truth face mapping lives in the test-source headers cited below.
- `crates/slicer-runtime/tests/executor/cube_4color_paint_tdd.rs` and `cube_fuzzy_painted_tdd.rs` — naming-convention reference and per-face paint-semantic ground truth. Read in full only if needed to verify a strengthened assertion's expected value; otherwise delegate "what `PaintSemantic::Material(ToolIndex)` value does face +X carry on `cube_4color.3mf`?" as a FACT dispatch.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — delegate any parity check (this packet should not need any).
- `target/`, `Cargo.lock`, generated code — never load.
- The cube `.3mf` files themselves — binary; never `Read` directly. Delegate structural questions to a sub-agent.
- `crates/slicer-core/src/algos/paint_segmentation.rs`, `crates/slicer-core/src/algos/mesh_segmentation.rs`, `crates/slicer-core/src/algos/region_mapping.rs` — out of bounds for this packet (P2/P3/P1c territory).
- Any path under `crates/slicer-runtime/src/`, `crates/slicer-ir/src/`, `crates/slicer-scheduler/src/` — this packet does not edit source.

## Expected Sub-Agent Dispatches

- "What `PaintSemantic` value does each of the six cube faces carry in `resources/cube_4color.3mf`? Return as a SUMMARY ≤ 100 words including the face-to-ToolIndex mapping" — purpose: feed strengthened assertions in Steps 3–4.
- "Run `cargo test -p slicer-runtime --test e2e cube_4color_modifier_part`; return FACT pass/fail or SNIPPETS (≤ 20 lines around `FAILED` or `panicked at`) on failure" — purpose: validate Step 3 exit condition.
- "Run `rg -nE 'benchy_(4color|painted)\.3mf' crates/ modules/ docs/ .ralph/ --glob '!.ralph/specs/89_benchy-3mf-retirement/**'`; return LOCATIONS (file:line, ≤ 20 entries)" — purpose: residual-reference sweep before deletion.
- "Run `cargo clippy --workspace --all-targets -- -D warnings`; return FACT pass/fail" — purpose: workspace gate after migration.
- "Run `cargo check --workspace --all-targets`; return FACT pass/fail" — purpose: ensure mod-declaration renames are coherent.
- "Find every `mod benchy_*` declaration under `crates/slicer-runtime/tests/`; return LOCATIONS" — purpose: catch every rename target.

## Data and Contract Notes

- IR or manifest contracts touched: none.
- WIT boundary considerations: none.
- Determinism or scheduler constraints: none.
- The cube fixtures encode an authoring convention (face +X = ToolIndex 1, etc.) that this packet documents in the migrated test bodies via comments referencing `docs/specs/orca-paint-segmentation-parity.md` §"Fixture Strategy". Any deviation from that mapping in a future fixture re-authoring would silently break the strengthened assertions; the convention reference is therefore documented as a Locked Assumption below.

## Locked Assumptions and Invariants

- **Cube fixture authoring convention** (corrected against `crates/slicer-runtime/tests/executor/cube_4color_paint_tdd.rs:6-12` ground truth — the prior version of this assumption was incorrect; see `packet.spec.md` §Closure Log §Packet-doc deviations applied):
  - `cube_4color.3mf` six-face mapping:
    | Face | PaintSemantic | Source line |
    | --- | --- | --- |
    | +X (Right, X≈137.5) | `Material(ToolIndex(1))` — fully green | `cube_4color_paint_tdd.rs:10` |
    | -X (Left,  X≈112.5) | `Material(ToolIndex(0))` — orange with 4-color hex-subdivided circles | `cube_4color_paint_tdd.rs:9` |
    | +Y (Back,  Y≈117.5) | `Material(ToolIndex(2))` — fully blue | `cube_4color_paint_tdd.rs:8` |
    | -Y (Front, Y≈92.5)  | Mixed `Material(ToolIndex(0..=3))` — 4 colors banded by height (hex-subdivided) | `cube_4color_paint_tdd.rs:7` |
    | +Z (Top,   Z≈24.9)  | Mixed — half `Material(ToolIndex(3))` red / half `Material(ToolIndex(0))` orange | `cube_4color_paint_tdd.rs:11` |
    | -Z (Bottom,Z≈0.1)   | Mixed — one face `Material(ToolIndex(2))` blue / one unpainted | `cube_4color_paint_tdd.rs:12` |
  - `cube_fuzzyPainted.3mf` six-face mapping:
    | Face | PaintSemantic | Source line |
    | --- | --- | --- |
    | +X | `FuzzySkin(Flag(true))` (hex-subdivided fuzzy circle) | `cube_fuzzy_painted_tdd.rs:8` |
    | -X | unpainted / `None` | `cube_fuzzy_painted_tdd.rs:9` |
    | +Y | Mixed — half `FuzzySkin(Flag(true))` / half unpainted | `cube_fuzzy_painted_tdd.rs:7` |
    | -Y | `FuzzySkin(Flag(true))` (fully painted) | `cube_fuzzy_painted_tdd.rs:6` |
    | +Z | `FuzzySkin(Flag(true))` (hex-subdivided fuzzy circle) | `cube_fuzzy_painted_tdd.rs:10` |
    | -Z | unpainted / `None` | `cube_fuzzy_painted_tdd.rs:11` |
  - The implication: only `+X`, `+Y`, and `-X` (orange) of `cube_4color.3mf` are pure-uniform faces suitable for the simplest "this face is `Material(ToolIndex(N))`" assertions; the banded/mixed faces (-Y, ±Z) require either a layer/Y-coordinate-bracketed assertion or substitution with a uniform face. If the fixture is ever re-authored, this table and every strengthened assertion must be updated in the same commit.
- **Per-process cache invariant**: `cached_load_model` and `cached_run` continue to share fixtures across tests within one process. After migration the cache key space shrinks (fewer distinct fixtures); this is a wall-clock improvement, not a behavior change.
- **Newly-authored fixture size ceiling**: any `cube_*.3mf` authored by this packet is ≤ 100 KB on disk (AC-N2).

## Risks and Tradeoffs

- **Risk: a SHAPE-DEPENDENT assertion does not migrate cleanly** because the cube's paint distribution differs from benchy's. Mitigation: per-test classification in Step 1 catches this; if a true mismatch is found, the assertion is rewritten to target a different geometric feature the cube *does* have, with a comment explaining the substitution. Last-resort fallback: a single `#[ignore = "no cube-fixture equivalent — see comment"]` holdout that documents the gap explicitly, never silent weakening.
- **Risk: a newly-authored fixture takes time to produce reliably.** Mitigation: prefer adjusting assertions against `cube_4color.3mf` over authoring `cube_with_modifier_part.3mf` / `cube_rotated_component.3mf`. Only author when a test's STRUCTURAL property genuinely cannot be expressed against the existing cube.
- **Tradeoff: storage reclaim vs. test-coverage breadth.** The migration trades a benchy-shaped real-world regression surface for engineered-cube assertions that are stronger per-test but cover a smaller geometry envelope. P0b's regression_wedge.stl is the companion mitigation — together the two packets cover the assertion classes benchy was carrying.

## Context Cost Estimate

- Aggregate: `M` (8 step-units across modest test-file edits, plus one fixture-deletion step, plus one residual-grep step).
- Largest single step: `M` (Step 3 — `cube_4color_modifier_part_e2e_tdd.rs` rewrite with 7 test bodies).
- Highest-risk dispatch: the "cube face paint distribution" SUMMARY (must be specific enough to feed strengthened assertions; required return format: SUMMARY ≤ 100 words including a face→ToolIndex mapping table).

## Open Questions

- `[RESOLVED]` (Step 1 dispatch, this run) — Both `resources/cube_with_modifier_part.3mf` and `resources/cube_rotated_component.3mf` are `absent` at packet start. Steps 3 and 5 each evaluate first whether the existing `cube_4color.3mf` can satisfy the test's STRUCTURAL/SHAPE property; the new fixtures are authored only on substitution failure.
