# Implementation Plan: 206-seam-paint-delivery

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Red tests for the seam-annotation writer

- Task IDs: `TASK-322`
- Objective: Append four failing `#[test]` fns to `crates/slicer-runtime/tests/executor/paint_channel_consumer_paths_tdd.rs` — `seam_paint_writer_populates_segment_annotations` (AC-1), `seam_paint_writer_runs_on_seam_only_mesh` (AC-2), `seam_paint_annotations_are_index_aligned_per_region` (AC-3), `seam_paint_writer_emits_no_key_without_seam_paint` (AC-N2) — reusing the file's existing helpers rather than authoring new fixtures.
- Precondition: `paint_channel_seam_strokes_do_not_partition_regions` passes on the current tree (the DEV-123 region-split half is already closed).
- Postcondition: the four new tests compile and fail with an assertion about a missing `PaintSemantic::Custom("seam_enforcer")` key; `paint_channel_seam_strokes_do_not_partition_regions` still passes.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/executor/paint_channel_consumer_paths_tdd.rs` - the module header, the helper block (`cube_cilindrical_modifier_path`, `build_layer_plan`, `build_initial_slice_ir`, `build_region_map`, `make_single_object_mesh_ir`, `make_unit_cube_slice_ir`, `make_unit_cube_region_map`), and `paint_channel_seam_strokes_do_not_partition_regions`
  - `crates/slicer-ir/src/slice_ir.rs` - `PaintLayer`, `PaintStroke`, `PaintValue`, `SlicedRegion.segment_annotations` declarations only; locate by symbol
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/executor/paint_channel_consumer_paths_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-core/src/**` (no production code in a red step)
  - `crates/slicer-model-io/src/loader.rs`
  - `OrcaSlicerDocumented/**`
- Blast-radius discipline: not applicable — no struct field and no schema/version constant is added in this step.
- Expected sub-agent dispatches:
  - Question: what does `resources/cube_cilindrical_modifier.3mf` yield for `Custom("seam_enforcer")` — facet_value count and stroke count — and at which `global_layer_index` does the lowest painted facet land for `LAYER_COUNT = 50`, `LAYER_HEIGHT_MM = 0.5`?; scope: run the existing `paint_channel_seam_strokes_do_not_partition_regions` and read its `DIAGNOSTIC` stderr line; return: `FACT` (≤5 lines)
- Context cost: `S`
- Authoritative docs:
  - `docs/02_ir_schemas.md` §"IR 6 — SliceIR" - direct ranged read of that section only (`SlicedRegion` lives there; §"IR 2" is `SurfaceClassificationIR`)
- OrcaSlicer refs:
  - none for this step
- Verification:
  - `mkdir -p target && cargo test -p slicer-runtime --test executor seam_paint_ 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT: expect `4 failed`
  - `mkdir -p target && cargo test -p slicer-runtime --test executor paint_channel_seam_strokes_do_not_partition_regions 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail; must be `ok`
- Exit condition: exactly four tests fail, each on a missing seam key rather than a compile error or a panic inside `execute_paint_segmentation`. If any fails for a different reason, stop and re-diagnose — a panic here means the fixture assumption is wrong, not the writer.

### Step 2: Implement `seam_annotations` and wire `execute_paint_segmentation`

- Task IDs: `TASK-322`
- Objective: Create `crates/slicer-core/src/algos/paint_segmentation/seam_annotations.rs` with `mesh_has_seam_paint` and `stamp_seam_paint_annotations` per `design.md` §Code Change Surface item 1, declare the module, and route `execute_paint_segmentation`'s three return paths through it per item 2.
- Precondition: Step 1's four tests fail on a missing seam key.
- Postcondition: all four pass; `paint_channel_seam_strokes_do_not_partition_regions` still passes; `cargo xtask build-guests --check` reports no `STALE:`.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/src/algos/paint_segmentation/mod.rs` - symbol-located windows ONLY: `is_seam_paint_semantic`, `mesh_has_any_paint`, the head of `execute_paint_segmentation` through the `region_map.entries.is_empty()` guard, `build_modifier_segment_annotations`, the `painted_subsets` facet/stroke accumulation arms, and the tail `Ok(Arc::new(working))`
  - `crates/slicer-core/src/algos/paint_segmentation/modifier_volumes.rs` - short; whole file - purpose: `slice_modifier_volumes`' `crate::slice_mesh_ex` call and `any_expolygon_contains_point`
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/algos/paint_segmentation/seam_annotations.rs` (new)
  - `crates/slicer-core/src/algos/paint_segmentation/mod.rs`
- Files explicitly out of bounds:
  - `crates/slicer-core/src/algos/paint_segmentation/top_bottom.rs` and the rest of the cell-decomposition machinery
  - `crates/slicer-ir/**`, `crates/slicer-schema/wit/**` (no IR or WIT change)
  - `modules/core-modules/**` (later steps)
- Blast-radius discipline: not applicable — the writer adds a map key, not a struct field or constant. It must NOT add a variant to `PaintSemantic` or `PaintValue`; both already carry what is needed.
- Expected sub-agent dispatches:
  - Question: does `execute_paint_segmentation`'s `painted_subsets` accumulation apply `obj.transform.matrix` to facet or stroke vertices, or push raw `obj.mesh.vertices` / `stroke.triangles`?; scope: `crates/slicer-core/src/algos/paint_segmentation/mod.rs` `painted_subsets` loop; return: `SNIPPETS` (≤2, ≤30 lines each) — the in-file comment claims a world transform is applied; verify against the code, not the comment
  - Question: `cargo xtask build-guests --check` — any `STALE:` line?; scope: cargo; return: `FACT` pass/fail
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md` §"IR 6 — SliceIR" - direct ranged read of that section only. It does NOT name `segment_annotations`; the no-bump justification is in `packet.spec.md` §Authoritative Docs (only map keys are added, so `CURRENT_SLICE_IR_SCHEMA_VERSION` does not move)
  - `CLAUDE.md` §"Guest WASM Staleness" - direct read
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp` - `gather_enforcers_blockers`; delegate, never load
- Verification:
  - `mkdir -p target && cargo test -p slicer-runtime --test executor seam_paint_ 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT: expect `0 failed`
  - `mkdir -p target && cargo test -p slicer-runtime --test executor paint_channel_seam_strokes_do_not_partition_regions 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail
  - `cargo xtask build-guests --check` - FACT: clean, or the list of stale guests
- Exit condition: AC-1, AC-2, AC-3 and AC-N2 all pass, AC-N3's guard is unregressed, and `--check` is clean. A `STALE:` report means rebuild and re-run before drawing any conclusion — a stale `slicer-core` runs old geometry code silently.

### Step 3: Harden the writer against the seam-only and variant-chain cases

- Task IDs: `TASK-322`
- Objective: Add in-file `#[cfg(test)]` unit tests to `seam_annotations.rs` covering the three cases the executor-bucket tests cannot isolate: a mesh with `seam_blocker` only (both semantics are handled independently), a region whose `variant_chain` is non-empty (stamped, per the `[FWD]` decision in `design.md`), and a region whose polygons have a hole (holes are not annotated; only `contour.points` slots are emitted).
- Precondition: Step 2's exits hold.
- Postcondition: the three unit tests pass under `cargo test -p slicer-core --features host-algos`.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/src/algos/paint_segmentation/modifier_volumes.rs` - its `#[cfg(test)]` block only - purpose: copy the fixture shape. Copy **all three** helpers — `cube_mesh`, `make_modifier_volume` and `mesh_with_modifier` — not just the latter two: `make_modifier_volume` and `mesh_with_modifier` both take an `IndexedTriangleSet` that every caller in that block builds with `cube_mesh(size)`, so the pair alone will not compile.
  - `crates/slicer-core/src/algos/paint_segmentation/seam_annotations.rs` - own file
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/algos/paint_segmentation/seam_annotations.rs`
- Files explicitly out of bounds:
  - `crates/slicer-core/src/algos/paint_segmentation/mod.rs` (already wired; do not re-touch)
  - `modules/core-modules/**`
- Blast-radius discipline: not applicable.
- Expected sub-agent dispatches:
  - Question: `cargo test -p slicer-core --features host-algos seam_annotations` — pass/fail plus failing assertion; scope: cargo; return: `FACT` pass/fail with ≤20 lines on failure
- Context cost: `S`
- Authoritative docs:
  - `CLAUDE.md` §"Feature-gated test files report green when they don't compile" - direct read; `slicer-core`'s `default = []` means a bare `-p slicer-core` run may silently skip gated code
- OrcaSlicer refs:
  - none for this step
- Verification:
  - `mkdir -p target && cargo test -p slicer-core --features host-algos seam_annotations --no-fail-fast 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT: expect `0 failed` and a nonzero test count
- Exit condition: the run reports a nonzero number of `seam_annotations` tests and `0 failed`. A count of zero means the tests did not compile into the binary — treat that as a failure, never as a pass.

### Step 4: Promote `seam_paint_boxes` into `slicer-core` and rewire classic

- Task IDs: `TASK-322`
- Objective: Move `seam_paint_boxes` (→ `pub fn`) and `seam_paint_box` (private) from `modules/core-modules/classic-perimeters/src/lib.rs` into `crates/slicer-core/src/perimeter_utils.rs` verbatim — same 1 mm `HALF_SIZE_MM`, same `Some(Some(PaintValue::Flag(true)))` filter — delete the classic copies, and extend classic's `use slicer_core::perimeter_utils::{…}` list.
- Precondition: Step 3's exits hold.
- Postcondition: AC-7's static check passes; packet 108's four tests in `crates/slicer-runtime/tests/integration/painted_seam_enforcer_blocker_tdd.rs` and the classic suite are unregressed; `--check` clean.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/classic-perimeters/src/lib.rs` - symbol-located windows ONLY: the `use slicer_core::perimeter_utils::{…}` import, `seam_paint_boxes`, `seam_paint_box`, and the `emit_walls` seam-candidate block that calls them
  - `crates/slicer-core/src/perimeter_utils.rs` - the `apply_seam_paint_bias` window and the file's existing `use` header
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/perimeter_utils.rs`
  - `modules/core-modules/classic-perimeters/src/lib.rs`
- Files explicitly out of bounds:
  - `modules/core-modules/arachne-perimeters/src/lib.rs` (Step 5)
  - `crates/slicer-core/src/algos/**`
- Blast-radius discipline: this step MOVES two symbols across a crate boundary. Before editing, dispatch the `LOCATIONS` inventory below and cite the result inline in the commit message; every call site must resolve to the promoted symbol afterwards. Expected inventory as authored: two call sites in classic's `emit_walls`, one internal call from `seam_paint_boxes` to `seam_paint_box`, and zero elsewhere.
- Expected sub-agent dispatches:
  - Question: list every occurrence of `seam_paint_boxes` and `seam_paint_box` across `crates/**/*.rs` and `modules/**/*.rs`, excluding `target/`; scope: those globs; return: `LOCATIONS` (≤20 entries)
  - Question: `cargo xtask build-guests --check` — any `STALE:` line?; scope: cargo; return: `FACT` pass/fail
- Context cost: `S`
- Authoritative docs:
  - `docs/05_module_sdk.md` §"Paint-seam consumption" - direct ranged read of that section only; it documents the reader contract being relocated
- OrcaSlicer refs:
  - none for this step (pure move)
- Verification:
  - AC-7's grep command from `packet.spec.md` - FACT: `PASS` or `FAIL:<file>`
  - `mkdir -p target && cargo test -p slicer-runtime --test integration painted_seam 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT: expect `0 failed`
  - `mkdir -p target && cargo test -p classic-perimeters 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT: expect `0 failed`
  - `cargo xtask build-guests --check` - FACT: clean, or the list of stale guests
- Exit condition: AC-7 passes, both suites are green, `--check` is clean, and `rg -c 'fn seam_paint_box' modules/core-modules/classic-perimeters/src/lib.rs` returns no match.

### Step 5: Add arachne's `apply_seam_paint_bias` call (DEV-134)

- Task IDs: `TASK-322`
- Objective: Write `crates/slicer-runtime/tests/integration/arachne_seam_paint_bias_tdd.rs` red (AC-6: an arachne region whose `segment_annotations[Custom("seam_blocker")][0]` flags the vertex nearest a qualifying sharp corner emits no candidate inside the blocker box, and `ClassicPerimeters` on the same fixture excludes the same corner), register its `mod` line, then make it green by converting arachne's seam loop to `for (poly_idx, polygon) in polygons.iter().enumerate()`, binding `let mut candidates`, and inserting the two `seam_paint_boxes` calls plus `apply_seam_paint_bias`.
- Precondition: Step 4's exits hold, so `slicer_core::perimeter_utils::seam_paint_boxes` exists as a `pub fn`.
- Postcondition: AC-6 passes; the arachne suite is unregressed; `--check` clean.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/arachne-perimeters/src/lib.rs` - symbol-located windows ONLY: the `use slicer_core::perimeter_utils::{…}` import, the `let polygons = region.polygons();` binding, the `generate_sharp_corner_seam_candidates` loop, and the `build_wall_flags` call that already reads `region.segment_annotations()`
  - `crates/slicer-runtime/tests/integration/painted_seam_enforcer_blocker_tdd.rs` - purpose: reuse `box_poly` / `quad_path` / `PerimeterRegionViewBuilder` / `ConfigViewBuilder` setup and `classic_perimeters_blocker_excludes_painted_corner`'s shape
  - `crates/slicer-runtime/tests/integration/main.rs` - the `mod` list only
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/integration/arachne_seam_paint_bias_tdd.rs` (new)
  - `crates/slicer-runtime/tests/integration/main.rs`
  - `modules/core-modules/arachne-perimeters/src/lib.rs`
- Files explicitly out of bounds:
  - `modules/core-modules/classic-perimeters/src/lib.rs` (must not be adjusted to make the agreement assertion pass)
  - `crates/slicer-core/src/perimeter_utils.rs`
- Blast-radius discipline: not applicable — no struct field or constant changes. The `for polygon in polygons` → `.iter().enumerate()` conversion is local to one loop.
- Expected sub-agent dispatches:
  - Question: does `crates/slicer-runtime/Cargo.toml` already dev-depend on `arachne-perimeters`, `classic-perimeters` and `seam-placer` for the `integration` bucket?; scope: `crates/slicer-runtime/Cargo.toml` `[dev-dependencies]`; return: `FACT` (≤5 lines)
  - Question: `cargo xtask build-guests --check` — any `STALE:` line?; scope: cargo; return: `FACT` pass/fail
- Context cost: `M`
- Authoritative docs:
  - `docs/05_module_sdk.md` §"Paint-seam consumption" - direct ranged read; the section this step makes generator-neutral
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.hpp` and `.../PerimeterGenerator.cpp` - delegate; the parity argument is that `SeamPlacer::init` / `place_seam` run after both generators, so no generator split exists upstream
- Verification:
  - `mkdir -p target && cargo test -p slicer-runtime --test integration arachne_seam_paint_bias 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT: expect `0 failed`
  - `mkdir -p target && cargo test -p arachne-perimeters 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT: expect `0 failed`
  - `cargo xtask build-guests --check` - FACT: clean, or the list of stale guests
- Exit condition: AC-6 passes with BOTH arms asserting (arachne excludes the corner AND classic excludes the same corner). If the classic arm fails, the two generators disagree on the `poly_idx` index base — report the divergence; do not adjust either index base to force agreement.

### Step 6: Exact-semantic classification in `seam-planner-default` (DEV-133)

- Task IDs: `TASK-322`
- Objective: Write `modules/core-modules/seam-planner-default/tests/seam_paint_semantic_exactness_tdd.rs` red with exactly three `#[test]` fns, named so the AC command filters match them: `support_semantics` (AC-4), `exact_seam_semantics` (AC-5) and `support_named_custom_semantics_are_neutral` (AC-N1) — using the `#[path = "../src/…"] mod …;` source-inclusion pattern, then delete `paint_marker`, rewrite `paint_annotation_type` to an exact two-name match dropping its `value` parameter, update its two internal call sites, and re-express `paint_annotations_set_point_type` (`seam_canonical_visibility_tdd.rs`) against the production encoding without weakening any of its **six** assertions — four `point_type` equality checks (`layer[0]` → `Enforced`, `layer[1]` → `Enforced`, `layer[2]` → `Blocked`, `layer[3]` → `Neutral`) and two `central_enforcer` checks (`assert!(layer[0].central_enforcer)`, `assert!(!layer[1].central_enforcer)`). Both `central_enforcer` assertions must survive the re-encoding; dropping either is a weakening, not a re-expression.
- Precondition: Step 5's exits hold.
- Postcondition: AC-4, AC-5 and AC-N1 pass; `seam_canonical_visibility_tdd` is fully green; `--check` clean.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/seam-planner-default/src/visibility.rs` - `paint_marker`, `paint_annotation_type`, `annotation_at`, `has_enforced_annotation`, `is_central_enforcer_vertex`, `candidate_paint_classification`, `build_seam_candidates`, `build_seam_candidates_with_sample_count`
  - `modules/core-modules/seam-planner-default/src/comparator.rs` - `EnforcedBlockedSeamPoint` and `SeamCandidate.point_type` only
  - `modules/core-modules/seam-planner-default/tests/seam_canonical_visibility_tdd.rs` - lines `1-40` (the source-inclusion preamble and `prism_setup`) and `paint_annotations_set_point_type`
- Files allowed to edit (at most 3):
  - `modules/core-modules/seam-planner-default/tests/seam_paint_semantic_exactness_tdd.rs` (new)
  - `modules/core-modules/seam-planner-default/src/visibility.rs`
  - `modules/core-modules/seam-planner-default/tests/seam_canonical_visibility_tdd.rs`
- Files explicitly out of bounds:
  - `modules/core-modules/seam-planner-default/src/comparator.rs` (the enum and its ordering are canonical and unchanged)
  - `modules/core-modules/seam-planner-default/seam-planner-default.toml` (no manifest key changes)
  - `crates/slicer-core/**`
- Blast-radius discipline: this step REMOVES a parameter from `paint_annotation_type`. Before editing, dispatch the `LOCATIONS` inventory below; every call site must be updated in this same step, and any test elsewhere that constructs `SupportEnforcer`/`SupportBlocker` annotations and asserts a seam `point_type` must be re-expressed here, not deferred.
- Expected sub-agent dispatches:
  - Question: list every call site of `paint_annotation_type` and `paint_marker`, and every test across `crates/**/tests/**` and `modules/**/tests/**` that constructs `PaintSemantic::SupportEnforcer` or `SupportBlocker` annotations and asserts on a seam `point_type`; scope: those globs; return: `LOCATIONS` (≤20 entries)
  - Question: `cargo xtask build-guests --check` — any `STALE:` line?; scope: cargo; return: `FACT` pass/fail
- Context cost: `M`
- Authoritative docs:
  - `docs/05_module_sdk.md` §"T-083 — `seam-planner-default` (PrePass) independence" - direct ranged read of that section only
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp` - `gather_enforcers_blockers` reads `mv->seam_facets` exclusively; support enforcers/blockers are consumed by the support generator. Delegate; never load.
- Verification:
  - `mkdir -p target && cargo test -p seam-planner-default --test seam_paint_semantic_exactness_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT: expect `0 failed`
  - `mkdir -p target && cargo test -p seam-planner-default --test seam_canonical_visibility_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT: expect `0 failed`
  - `cargo xtask build-guests --check` - FACT: clean, or the list of stale guests
- Exit condition: AC-4, AC-5 and AC-N1 pass and `paint_annotations_set_point_type` still carries **all six** of its original assertions under the new encoding — four `point_type` equality checks (`layer[0]` → `Enforced`, `layer[1]` → `Enforced`, `layer[2]` → `Blocked`, `layer[3]` → `Neutral`) plus `assert!(layer[0].central_enforcer)` and `assert!(!layer[1].central_enforcer)`. Verify mechanically before declaring the step done, with a check scoped to that fn's body (a file-wide `rg -c 'assert'` counts the whole suite and proves nothing): `awk '/^fn paint_annotations_set_point_type/{f=1} f&&/assert/{a++} f&&/central_enforcer/{c++} f&&/^}$/{printf "asserts=%d central_enforcer=%d\n",a,c; exit}' modules/core-modules/seam-planner-default/tests/seam_canonical_visibility_tdd.rs` must print exactly `asserts=6 central_enforcer=2` (measured on the pre-edit tree: it does). The awk range runs from the `fn` line to the first column-0 `}`, which is the fn's closing brace under rustfmt. Deleting or relaxing any of the six fails this step.

### Step 7: Doc and deviation-row closure

- Task IDs: `TASK-322`
- Objective: Flip DEV-123, DEV-133 and DEV-134 to `Closed — packet 206 …` in `docs/DEVIATION_LOG.md`; edit the two `docs/05_module_sdk.md` sections per the Doc Impact Statement; repair the stale module header of `crates/slicer-runtime/tests/executor/paint_channel_consumer_paths_tdd.rs`; regenerate the doc-07 Open Deviation Map with `cargo xtask check-deviations`.
- Precondition: Steps 1-6 all green.
- Postcondition: AC-8 and AC-9 pass; `cargo xtask check-deviations --check` exits 0.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/DEVIATION_LOG.md` - the DEV-123, DEV-133 and DEV-134 rows only; **delegate**, do not load the file
  - `docs/05_module_sdk.md` - §"Paint-seam consumption" and §"T-083 — `seam-planner-default` (PrePass) independence" only
  - `crates/slicer-runtime/tests/executor/paint_channel_consumer_paths_tdd.rs` - the module header comment only
- Files allowed to edit (at most 3):
  - `docs/DEVIATION_LOG.md`
  - `docs/05_module_sdk.md`
  - `crates/slicer-runtime/tests/executor/paint_channel_consumer_paths_tdd.rs`
- Files explicitly out of bounds:
  - `docs/07_implementation_status.md`'s Open Deviation Map - **generated**; regenerate with `cargo xtask check-deviations`, never hand-edit
  - all production source (this is a docs-and-comments step)
- Blast-radius discipline: not applicable.
- Expected sub-agent dispatches:
  - Question: return the current Status cells of DEV-123, DEV-133 and DEV-134 verbatim; scope: `docs/DEVIATION_LOG.md`; return: `SNIPPETS` (≤3, Status cells only)
  - Question: `cargo xtask check-deviations --check` — exit code?; scope: cargo; return: `FACT` pass/fail
- Context cost: `S`
- Authoritative docs:
  - `docs/DEVIATION_LOG.md` header - direct read of the "Single source of truth" note; it forbids hand-editing the generated views
- OrcaSlicer refs:
  - none for this step
- Verification:
  - AC-8's python + `check-deviations --check` command from `packet.spec.md` - FACT: `PASS`
  - AC-9's grep command from `packet.spec.md` - FACT: `PASS`
  - `cargo check --workspace --all-targets` - FACT pass/fail (the test-file header edit must not break compilation)
- Exit condition: AC-8 and AC-9 both return `PASS`, and no line of `docs/07_implementation_status.md` was edited by hand.

### Step 8: Packet gates

- Task IDs: `TASK-322`
- Objective: Run the closure gates and record the result; no source edits.
- Precondition: Steps 1-7 all green.
- Postcondition: `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo xtask build-guests --check` all pass; every pipe-suffixed AC has been re-dispatched and returned `PASS`.
- Files allowed to read, with ranges when over 300 lines:
  - `.ralph/specs/206-seam-paint-delivery/packet.spec.md` - the AC list
- Files allowed to edit (at most 3):
  - `.ralph/specs/206-seam-paint-delivery/packet.spec.md` (status transition only, at closure)
- Files explicitly out of bounds:
  - all production and test source
- Blast-radius discipline: not applicable.
- Expected sub-agent dispatches:
  - Question: `cargo clippy --workspace --all-targets -- -D warnings` — pass/fail; scope: cargo; return: `FACT` pass/fail with ≤20 lines on failure
  - Question: `cargo check --workspace --all-targets` — pass/fail; scope: cargo; return: `FACT` pass/fail with ≤20 lines on failure
- Context cost: `S`
- Authoritative docs:
  - `CLAUDE.md` §"Test Discipline" - direct read; governs whether the workspace suite runs and how it is dispatched
- OrcaSlicer refs:
  - none for this step
- Verification:
  - `cargo check --workspace --all-targets` - FACT pass/fail
  - `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail
  - `cargo xtask build-guests --check` - FACT: clean
- Exit condition: all three gates green and all nine positive plus three negative ACs re-dispatched `PASS`.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Red tests only; all fixtures already exist in the target file |
| Step 2 | M | The writer plus three-return-path rewiring; largest step |
| Step 3 | S | In-file unit tests; must run with `--features host-algos` |
| Step 4 | S | Pure symbol move; blast radius pre-inventoried |
| Step 5 | M | New integration test plus the arachne loop change |
| Step 6 | M | Discriminator rewrite plus one existing-test re-expression |
| Step 7 | S | Docs and deviation rows |
| Step 8 | S | Gates only |

Split before activation if aggregate cost exceeds M or any step is L. No step is rated L.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read: append the `TASK-322` entry naming packet 206 and the three closed deviation rows. `TASK-322` does not exist in the backlog today — the dispatch adds it. Do not hand-edit the generated Open Deviation Map in the same file.
- Reconcile reopened/superseded status transitions: none. Packet 108 (T-P98-SEAM) is neither reopened nor superseded — this packet feeds the reader it built and leaves its contract intact.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC (AC-1 … AC-9, AC-N1 … AC-N3) and the three packet-level gate commands.
- Record remaining packet-local risk: seam bias is live for the first time, so any baseline captured from a seam-painted fixture is invalidated; and `apply_seam_paint_bias` now runs in both generators, moving arachne toolpaths on seam-painted models.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
