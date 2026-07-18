---
status: implemented
packet: 89
task_ids: [TASK-239]
---

# 89_benchy-3mf-retirement

## Goal

Retire `resources/benchy_4color.3mf` (2.6 MB) and `resources/benchy_painted.3mf` (2.5 MB) as test fixtures by migrating every consuming test to the engineered cube fixtures (`cube_4color.3mf`, `cube_fuzzyPainted.3mf`) landed in cherry-pick `5c272ef970fee2b861081799169a3ddb87e179c9`, strengthening assertions to per-face deterministic checks where the cube's engineered semantics allow, and deleting the two benchy 3MF files plus the accompanying README so no test in the workspace continues to parse a multi-megabyte 3MF whose paint geometry is opaque.

## Problem Statement

`resources/benchy_4color.3mf` (2.6 MB, multi-material benchy with arbitrary paint distribution) and `resources/benchy_painted.3mf` (2.5 MB, painted benchy with mixed paint semantics) are consumed by ~33 tests across `crates/slicer-runtime/tests/e2e/`, `crates/slicer-model-io/tests/`, and `crates/slicer-runtime/tests/common/`. Each test consumes the model through `cached_load_model` / `cached_run`, but the fixtures contribute three concrete problems:

1. **Test wall-clock penalty.** Even with the per-process cache at `crates/slicer-runtime/tests/common/model_cache.rs`, first-touch parse + slice on a 2.5 MB 3MF dominates the e2e bucket cold-cache wall time. The cache header comment explicitly motivates itself with these files as the cost example.
2. **Opaque paint geometry.** The benchy paint is hand-painted in OrcaSlicer with no per-face determinism — tests can only assert weak "some color exists" or "g-code differs from unpainted" properties. They cannot say "cube face +X is ToolIndex 1 between layers 4 and 6", which is the kind of assertion the paint-segmentation port (P3, P4) needs to verify.
3. **Storage bloat.** 5.2 MB of opaque-geometry 3MFs sit in the repo. Cherry-pick `5c272ef970fee2b861081799169a3ddb87e179c9` landed `resources/cube_4color.3mf` (37 KB) and `resources/cube_fuzzyPainted.3mf` (27 KB) — engineered cubes with each face deliberately assigned a known paint semantic — plus 24 deterministic RED tests in `cube_4color_paint_tdd.rs` + `cube_fuzzy_painted_tdd.rs`. The migration was not done by that cherry-pick; this packet finishes it.

This packet does not change any production code in `crates/slicer-core`, `crates/slicer-ir`, or `crates/slicer-runtime/src/`. It is a fixture-and-test migration.

## Architecture Constraints

- This packet edits only test sources and the test-fixture cache doc-comment. No path under `wit/**`, `crates/slicer-macros/**`, `crates/slicer-sdk/**`, `crates/slicer-ir/**`, `crates/slicer-schema/**`, or any `modules/core-modules/*/src/**` is touched. The `wasm-staleness` snippet does **not** apply (no guest WASM input is modified).
- No path that participates in geometry math (polygon ops, mesh ops, mm↔unit conversion) is touched. The `coord-system` snippet does **not** apply.
- Fixture authoring constraint: any newly-authored `.3mf` MUST be deterministic — same input authoring tool + same input vertices/triangles + same paint assignments must produce byte-identical output. The team's 3MF authoring helper (if used) is single-threaded and stable; document the authoring command in this packet's closure log so any future regeneration is reproducible.
- Constraint: no file rename happens without an accompanying assertion rewrite in the same Git commit. A bare rename followed by a separate edit destroys `git log --follow` accuracy on the assertion content.

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
- **Risk: a newly-authored fixture takes time to produce reliably.** Mitigation: prefer adjusting assertions against `cube_4color.3mf` over authoring `cube_cilindrical_modifier.3mf` / `cube_rotated_component.3mf`. Only author when a test's STRUCTURAL property genuinely cannot be expressed against the existing cube.
- **Tradeoff: storage reclaim vs. test-coverage breadth.** The migration trades a benchy-shaped real-world regression surface for engineered-cube assertions that are stronger per-test but cover a smaller geometry envelope. P0b's regression_wedge.stl is the companion mitigation — together the two packets cover the assertion classes benchy was carrying.
