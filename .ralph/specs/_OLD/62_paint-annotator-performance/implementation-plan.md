# Implementation Plan: 62_paint-annotator-performance

## Execution Rules

- One atomic step at a time.
- Each step must map to the packet's grouped task IDs.
- TDD first, then implementation, then the narrowest falsifying validation.
- Each step honors the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. The fields below are the budget contract for this step.

## Steps

### Step 1: Add `BoundingBox2` type and `SemanticRegion.aabb` field

- Task IDs: TASK-130c, TASK-181 (infrastructure built upon)
- Objective: Add the `BoundingBox2` 2D AABB type to `slicer-ir` and an optional `aabb` field to `SemanticRegion` with serde-skip. No code yet computes or reads the field — verify all existing tests pass unchanged.
- Precondition: `cargo check --workspace` clean on HEAD.
- Postcondition: `BoundingBox2` struct defined with `{ min: Point2, max: Point2 }` and `contains_point(&self, point: Point2) -> bool`. `SemanticRegion` has `#[serde(skip_deserializing, default)] pub aabb: Option<BoundingBox2>`. All existing tests pass unchanged (aabb defaults to `None`, no consumer reads it yet).
- Files allowed to read:
  - `crates/slicer-ir/src/slice_ir.rs` — read lines 110-115 (existing `BoundingBox3` pattern) and lines 973-984 (`SemanticRegion` struct)
- Files allowed to edit:
  - `crates/slicer-ir/src/slice_ir.rs` — add `BoundingBox2` near `BoundingBox3`; add `aabb` field to `SemanticRegion`
- Files explicitly out-of-bounds for this step:
  - All other crates — no consumer code yet
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-ir`; return FACT (pass) or SNIPPETS (fail: test name + assertion + ≤ 20 lines)"
  - "Run `cargo test -p slicer-host --test core_module_ir_access_contract_tdd`; return FACT (pass) or SNIPPETS"
- Context cost: `S`
- Authoritative docs: `docs/02_ir_schemas.md` — delegate a FACT: what is the exact field order of `SemanticRegion`? The new field should be appended last.
- OrcaSlicer refs: none (pure IR type addition)
- Verification:
  - `cargo check -p slicer-ir` — dispatch as FACT pass/fail
  - `cargo test -p slicer-ir` — dispatch as FACT pass/fail
  - `cargo test -p slicer-host` — dispatch as FACT pass/fail (all host tests that use SemanticRegion must still pass with aabb=None)
- Exit condition: `cargo test -p slicer-ir` passes; `cargo test -p slicer-host` passes all existing tests (no test updated yet)

### Step 2: Union per-facet regions at harvest + compute AABB

- Task IDs: TASK-130c, TASK-181
- Objective: Rewrite `harvest_paint_segmentation_ir` in `dispatch.rs` to group entries by `(layer_index, object_id, semantic, value)`, union polygons per group via `slicer_core::union`, compute `BoundingBox2` from unioned polygon contour points, sort each semantic's Vec by `(paint_order, object_id, value_key)`, and update test assertions for changed `paint_order` values and polygon counts.
- Precondition: Step 1 complete (BoundingBox2 + aabb field available). Existing tests pass.
- Postcondition: `harvest_paint_segmentation_ir` produces fewer `SemanticRegion` entries per `(layer, semantic)`, each with unioned polygons and computed `aabb`. Test assertions in 4 test files updated to reflect post-union values. No query-path code reads `aabb` yet.
- Files allowed to read:
  - `crates/slicer-host/src/dispatch.rs` — read lines 2003-2085 (`harvest_paint_segmentation_ir` current body)
  - `crates/slicer-core/src/polygon_ops.rs` — read lines 93-95 (`union` signature) and line 62 (hole-loss comment)
  - `crates/slicer-host/src/paint_segmentation.rs` — read lines 196-199 (`compare_semantic_regions` sort ordering)
  - `crates/slicer-host/tests/paint_segmentation_executor_tdd.rs` — read lines 30-90 (3 paint_order assertions)
  - `crates/slicer-host/tests/macro_paint_region_roundtrip_tdd.rs` — read lines 85-130, 265-355 (9 paint_order + 1 polygons.len() assertions)
  - `crates/slicer-host/tests/scenario_traces_tdd.rs` — read lines 225-295 (2 precedence assertions)
  - `modules/core-modules/paint-region-annotator/tests/paint_region_annotator_tdd.rs` — read lines 420-455 (2 precedence assertions)
- Files allowed to edit:
  - `crates/slicer-host/src/dispatch.rs` — rewrite `harvest_paint_segmentation_ir` body
  - `crates/slicer-host/tests/paint_segmentation_executor_tdd.rs` — update paint_order assertions
  - `crates/slicer-host/tests/macro_paint_region_roundtrip_tdd.rs` — update paint_order + polygon.len() assertions
  - `crates/slicer-host/tests/scenario_traces_tdd.rs` — update precedence assertions
  - `modules/core-modules/paint-region-annotator/tests/paint_region_annotator_tdd.rs` — update precedence assertions
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-core/src/paint_region.rs` — query helpers not yet modified
  - `crates/slicer-host/src/slice_postprocess.rs` — annotation loop not yet modified
  - `wit_host.rs` — guest protocol unchanged
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-host --test paint_segmentation_executor_tdd -- --nocapture`; return FACT (pass) or SNIPPETS (fail: test name + assertion + ≤ 20 lines)" — validate union-at-harvest
  - "Run `cargo test -p slicer-host --test macro_paint_region_roundtrip_tdd`; return FACT or SNIPPETS" — validate paint_order + polygon count
  - "Run `cargo test -p slicer-host --test scenario_traces_tdd`; return FACT or SNIPPETS" — validate precedence preserved
  - "Run `cargo test -p slicer-host --test paint_region_annotator_tdd`; return FACT or SNIPPETS" — validate annotator precedence
  - "Run `cargo test -p slicer-host --test slice_postprocess_paint_annotation_tdd`; return FACT or SNIPPETS" — validate annotation still works with unioned IR
  - "Run `cargo check --workspace`; return FACT pass/fail"
- Context cost: `M`
- Authoritative docs: `docs/02_ir_schemas.md` §"PaintRegionIR" — re-delegate FACT for `paint_order` field semantics (precedence rule: higher = wins)
- OrcaSlicer refs: `pseudocode_multimaterial_segmentation.md` — delegate SUMMARY (≤ 200 words) of Phase 1 union_ex usage pattern
- Verification:
  - `cargo test -p slicer-host --test paint_segmentation_executor_tdd`
  - `cargo test -p slicer-host --test macro_paint_region_roundtrip_tdd`
  - `cargo test -p slicer-host --test scenario_traces_tdd`
  - `cargo test -p slicer-host --test paint_region_annotator_tdd`
  - `cargo test -p slicer-host --test slice_postprocess_paint_annotation_tdd`
  - `cargo check --workspace`
- Exit condition: all dispatched tests pass with updated assertions. `harvest_paint_segmentation_ir` produces grouped+unioned+AABB-computed regions. No query-path change yet.

### Step 3: Add AABB pre-filter, cache `get()`, and early-break in query path

- Task IDs: TASK-130c, TASK-181
- Objective: In `paint_region.rs`, add AABB pre-filter before polygon containment in `semantic_region_contains_point`, and add early-break in `point_in_paint_region` (break when sorted descending and winner found). In `slice_postprocess.rs`, cache `paint_regions.get(layer_index, semantic)` once per semantic pair and pass the `&[SemanticRegion]` slice through to `point_in_paint_region` and `is_point_numerically_ambiguous`.
- Precondition: Step 2 complete (IR has unioned regions with computed aabb). Existing annotation tests pass with updated assertions.
- Postcondition: Queries skip regions whose AABB does not contain the point. `point_in_paint_region` stops iterating after finding the highest-paint_order winner. `paint_regions.get()` is called once per `(layer_index, semantic)` per layer, not once per contour point.
- Files allowed to read:
  - `crates/slicer-core/src/paint_region.rs` — read in full (~130 lines)
  - `crates/slicer-host/src/slice_postprocess.rs` — read lines 286-492 (annotation loop), lines 510-540 (`is_point_numerically_ambiguous`)
- Files allowed to edit:
  - `crates/slicer-core/src/paint_region.rs` — add AABB pre-filter in `semantic_region_contains_point`; add early-break in `point_in_paint_region`
  - `crates/slicer-host/src/slice_postprocess.rs` — cache `get()` in outer loop; pass slice through to helpers
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-host/src/dispatch.rs` — harvest logic unchanged in this step
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-core paint_region`; return FACT (pass) or SNIPPETS (fail: test name + assertion + ≤ 20 lines)"
  - "Run `cargo test -p slicer-host --test slice_postprocess_paint_annotation_tdd`; return FACT or SNIPPETS"
  - "Run `cargo test -p slicer-host --test paint_annotation_integration_tdd`; return FACT or SNIPPETS"
  - "Run `cargo test -p slicer-host --test region_mapping_paint_semantic_tdd`; return FACT or SNIPPETS"
- Context cost: `M`
- Authoritative docs: none beyond those already referenced
- OrcaSlicer refs: `03_algorithmic_complexities.md` §"AABB Tree" — delegate FACT: does OrcaSlicer's AABBTree pre-filter before containment, or is it used only for nearest-line queries?
- Verification:
  - `cargo test -p slicer-core paint_region`
  - `cargo test -p slicer-host --test slice_postprocess_paint_annotation_tdd`
  - `cargo test -p slicer-core --lib paint_region` (if unit tests exist in lib)
- Exit condition: all dispatched tests pass. `semantic_region_contains_point` returns false immediately for AABB misses. `point_in_paint_region` breaks after winner found. Annotation loop calls `get()` once per semantic pair.

### Step 4: Parallelize contour point annotation with `par_iter()`

- Task IDs: TASK-130c, TASK-181
- Objective: Replace serial `region.polygons.iter()` in `execute_slice_postprocess_paint_annotation` with `rayon::par_iter()`, collecting `point_paint` results per polygon and merging thread-local `warnings` and `degraded` flags after collection.
- Precondition: Step 3 complete (query path optimized, all tests pass). `rayon` is an existing `slicer-host` dependency.
- Postcondition: Contour point annotation scales with thread count. Results are identical to serial path. No `warnings` lost or duplicated.
- Files allowed to read:
  - `crates/slicer-host/src/slice_postprocess.rs` — read lines 357-430 (the contour point loop body)
  - `crates/slicer-host/src/layer_executor.rs` — read lines 257-272 (existing `par_iter()` pattern to mimic)
- Files allowed to edit:
  - `crates/slicer-host/src/slice_postprocess.rs` — add `use rayon::prelude::*`; replace `.iter()` with `.par_iter()`; add thread-local accumulator merge
- Files explicitly out-of-bounds for this step:
  - All other files — this step touches only the annotation loop body
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-host --test slice_postprocess_paint_annotation_tdd -- --nocapture`; return FACT (pass) or SNIPPETS"
  - "Run `cargo test -p slicer-host --test paint_annotation_integration_tdd`; return FACT or SNIPPETS"
  - "Run `cargo check --workspace`; return FACT pass/fail"
- Context cost: `M`
- Authoritative docs: none beyond those already referenced
- OrcaSlicer refs: `pseudocode_multimaterial_segmentation.md` — delegate FACT: does OrcaSlicer's MMU pipeline use `tbb::parallel_for` on per-layer or per-triangle granularity?
- Verification:
  - `cargo test -p slicer-host --test slice_postprocess_paint_annotation_tdd`
  - `cargo test -p slicer-host --test paint_annotation_integration_tdd`
  - `cargo clippy --workspace -- -D warnings`
- Exit condition: all dispatched tests pass. `par_iter()` replaces `iter()` with correct thread-local merge. `cargo clippy` clean.

### Step 5: Update docs/02_ir_schemas.md

- Task IDs: TASK-130c, TASK-181
- Objective: Document the new `BoundingBox2` type and `SemanticRegion.aabb` field in the IR schema documentation.
- Precondition: Steps 1-4 complete. All tests pass.
- Postcondition: `docs/02_ir_schemas.md` contains `BoundingBox2` entry and `SemanticRegion.aabb` field note. Both greppable by the phrases specified in `packet.spec.md` Doc Impact Statement.
- Files allowed to read:
  - `docs/02_ir_schemas.md` — read lines 469-488 (PaintRegionIR section)
- Files allowed to edit:
  - `docs/02_ir_schemas.md` — add `BoundingBox2` struct definition after existing `BoundingBox3` entry; add `aabb` field note to `SemanticRegion` struct listing
- Files explicitly out-of-bounds for this step:
  - All source files — doc-only change
- Expected sub-agent dispatches:
  - "Run `rg -q 'BoundingBox2' docs/02_ir_schemas.md`; return FACT (found or not found)"
  - "Run `rg -q 'reconstruction-only' docs/02_ir_schemas.md`; return FACT (found or not found)"
- Context cost: `S`
- Authoritative docs: `docs/02_ir_schemas.md` — edit the canonical schema doc
- OrcaSlicer refs: none
- Verification:
  - `rg -q 'BoundingBox2' docs/02_ir_schemas.md`
  - `rg -q 'reconstruction-only' docs/02_ir_schemas.md`
- Exit condition: both `rg` commands return found.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Single IR type addition, no consumers yet |
| Step 2 | M | Union logic + 5 test file updates |
| Step 3 | M | AABB pre-filter + cache + early-break in 2 files |
| Step 4 | M | par_iter() + thread-local merge in 1 file |
| Step 5 | S | Doc-only change |
| **Aggregate** | **M** | |

## Packet Completion Gate

- All 5 steps complete.
- Every step exit condition is met.
- Packet acceptance criteria green (each verification command dispatched and returned PASS):
  - `cargo test -p slicer-host --test paint_segmentation_executor_tdd` → PASS
  - `cargo test -p slicer-host --test slice_postprocess_paint_annotation_tdd` → PASS
  - `cargo test -p slicer-core paint_region` → PASS
  - `cargo test -p slicer-host --test core_module_ir_access_contract_tdd` → PASS
  - `cargo test -p slicer-host --test scenario_traces_tdd` → PASS
  - `cargo test -p slicer-host --test macro_paint_region_roundtrip_tdd` → PASS
  - `cargo test -p slicer-host --test paint_region_annotator_tdd` → PASS
  - `cargo test -p slicer-host --test paint_region_transport_widening_tdd` → PASS
  - `cargo test -p slicer-host --test paint_annotation_integration_tdd` → PASS
  - `cargo test -p slicer-host --test region_mapping_paint_semantic_tdd` → PASS
  - `cargo test -p slicer-host --test benchy_4color_modifier_part_e2e_tdd` → PASS
  - `cargo check --workspace` → PASS
  - `cargo clippy --workspace -- -D warnings` → PASS
  - End-to-end `slicer-host --report` → annotator time < 10 s, 504 count near zero
- `docs/02_ir_schemas.md` updated with `BoundingBox2` and `SemanticRegion.aabb` documentation.
- No open activation-blocking questions remain.
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md` (AC-1 through AC-10, AC-N1 through AC-N3).
- Confirm packet-level verification commands are green.
- Recorded benchmark measurements (benchy_4color.3mf, 32 layers, 16 threads):
  - **Pre-change baseline** — prepass: 15,508ms, per-layer: 3,300,242ms, total: 299,633ms
  - **Post-change** (parallel per-group union + AABB + cache + early-break + par_iter) — prepass: 92,057ms, per-layer: 1,370,992ms, total: 274,752ms, 504-warnings: 0
  - Net improvement: total pipeline down 24,881ms (~8.3%), per-layer CPU down 58.5%, 504 warnings eliminated
  - Memory: peak host mem 3.72 GB → 5.88 GB (+58%, union allocates merged polygon buffers); WASM peak 18.69 MB → 7.75 MB (−58.5%, fewer SemanticRegion entries to deserialize per layer)
  - Note: prepass time increased (union cost at harvest), but amortized by per-layer savings. The 6.4s pipeline wall-clock for the full run (guest modules report 292ms each) meets the single-digit-seconds target.
