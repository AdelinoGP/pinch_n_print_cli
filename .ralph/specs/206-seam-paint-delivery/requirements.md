# Requirements: 206-seam-paint-delivery

## Packet Metadata

- Grouped task IDs: `TASK-322`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Seam paint has a fully-ported consumer chain and no producer. `crates/slicer-model-io/src/loader.rs` decodes 3MF `paint_seam` data into `PaintSemantic::Custom("seam_enforcer")` / `Custom("seam_blocker")` `PaintLayer`s; packet 108 built the reader (`seam_paint_boxes` → `slicer_core::perimeter_utils::apply_seam_paint_bias`) in `modules/core-modules/classic-perimeters/src/lib.rs`; `paint_annotation_type` (`modules/core-modules/seam-planner-default/src/visibility.rs`) classifies annotations for the canonical `EnforcedBlockedSeamPoint` comparator. Nothing writes those semantics into `SlicedRegion.segment_annotations`. The only production writer into that map, `build_modifier_segment_annotations` (`crates/slicer-core/src/algos/paint_segmentation/mod.rs`), keys strictly on `ModifierVolumeLayer.semantic`, i.e. `SupportEnforcer` / `SupportBlocker`.

Three defects compose into one slice:

- **DEV-123 (open half).** No writer, so the whole chain is starved. The region-split half is already closed on the working tree by `is_seam_paint_semantic`, which now filters seam semantics out of `mesh_has_any_paint`, out of the `let dominant_semantic = { … }` binding block inside `execute_paint_segmentation`'s per-object scan (a `let` block, not a function — locate it by the `let dominant_semantic` line and read its enclosing braces), and out of both `painted_subsets` accumulations. That fix has a consequence the plan did not anticipate and this packet must handle: on a **seam-only** mesh `mesh_has_any_paint` returns `false`, so `execute_paint_segmentation` short-circuits at its second guard and never reaches any writer at all.
- **DEV-133.** `paint_marker` substring-matches `"enforcer"` / `"blocker"` (and `"enforced"` / `"blocked"`), so the strings `SupportEnforcer` and `SupportBlocker` classify as seam intent. Because `build_modifier_segment_annotations` is today the only writer, support paint is currently the seam planner's *only* live paint input — the exact inversion of canonical, where `gather_enforcers_blockers` (`SeamPlacer.cpp`) reads `mv->seam_facets` exclusively.
- **DEV-134.** `modules/core-modules/arachne-perimeters/src/lib.rs` calls `generate_sharp_corner_seam_candidates` but never `apply_seam_paint_bias`, so `wall_generator = arachne` would drop the bias entirely. Canonical has no such split: seam paint is applied in `SeamPlacer.cpp` on finished loops, generator-independently.

They must land together. The writer alone makes DEV-133's leak user-visible (support paint would compete with real seam paint); the writer without DEV-134 makes seam bias silently classic-only.

No prior packet is reopened or superseded. Packet 108 (T-P98-SEAM) built the reader this packet finally feeds; its contract is preserved unchanged.

## In Scope

- A new writer module `crates/slicer-core/src/algos/paint_segmentation/seam_annotations.rs` exposing a `pub(super)`/`pub(crate)` entry point that, given the `MeshIR` and a mutable slice of `SliceIR` layers, stamps `PaintSemantic::Custom("seam_enforcer")` / `Custom("seam_blocker")` into every `SlicedRegion.segment_annotations` whose polygon edges fall inside that layer's seam-paint footprint, with values `Some(PaintValue::Flag(true))` / `None`.
- A cheap `mesh_has_seam_paint(&MeshIR) -> bool` guard so unpainted and non-seam-painted slices still return the input `Arc` without a clone.
- Rewiring `execute_paint_segmentation` (`crates/slicer-core/src/algos/paint_segmentation/mod.rs`) so the seam writer runs on **both** the short-circuit return paths (`mesh.objects.is_empty()` excepted) and the fully-processed `working` vector, without re-admitting seam semantics to `mesh_has_any_paint`, to the `let dominant_semantic = { … }` binding block, or to `painted_subsets`.
- Promotion of `seam_paint_boxes` and its helper `seam_paint_box` out of `modules/core-modules/classic-perimeters/src/lib.rs` into `crates/slicer-core/src/perimeter_utils.rs` as `pub fn seam_paint_boxes` / private `seam_paint_box`, with classic rewired onto the shared symbol.
- The `apply_seam_paint_bias` call in `modules/core-modules/arachne-perimeters/src/lib.rs`'s seam-candidate loop, which requires converting `for polygon in polygons` to `for (poly_idx, polygon) in polygons.iter().enumerate()` and `let candidates` to `let mut candidates`, and threading `region.segment_annotations()` into that scope.
- Exact-semantic classification in `paint_annotation_type` (`modules/core-modules/seam-planner-default/src/visibility.rs`): only `PaintSemantic::Custom("seam_enforcer")` → `Enforced` and `Custom("seam_blocker")` → `Blocked`; the `SupportEnforcer` / `SupportBlocker` arms are removed; the value-side `PaintValue::Custom(name)` → `paint_marker(name)` fallback is removed; `paint_marker` itself is deleted. Blocked-wins-over-Enforced precedence in `candidate_paint_classification` is preserved.
- Rewriting the existing `paint_annotations_set_point_type` test in `modules/core-modules/seam-planner-default/tests/seam_canonical_visibility_tdd.rs` — it currently encodes intent as `PaintSemantic::Custom("seam")` + `PaintValue::Custom("enforced"/"blocked")`, which the exact-match rule no longer classifies. It is re-expressed against the production encoding (two semantic keys, `PaintValue::Flag(true)`) keeping **all six** of its existing assertions — four `point_type` equality checks (`layer[0]` → `Enforced`, `layer[1]` → `Enforced`, `layer[2]` → `Blocked`, `layer[3]` → `Neutral`) and two `central_enforcer` checks (`assert!(layer[0].central_enforcer)`, `assert!(!layer[1].central_enforcer)`) — strengthened to match production, never weakened. Dropping either `central_enforcer` assertion is a weakening, not a re-expression.
- A new test file `modules/core-modules/seam-planner-default/tests/seam_paint_semantic_exactness_tdd.rs`, which must use the same `#[path = "../src/visibility.rs"] mod visibility;` source-inclusion pattern as `seam_canonical_visibility_tdd.rs` because `build_seam_candidates` / `build_seam_candidates_with_sample_count` are `pub(crate)` and not reachable from an ordinary integration-test crate.
- New tests in `crates/slicer-runtime/tests/executor/paint_channel_consumer_paths_tdd.rs` (writer behaviour, seam-only mesh, index alignment, no-spurious-key) and a new `crates/slicer-runtime/tests/integration/arachne_seam_paint_bias_tdd.rs` registered in `crates/slicer-runtime/tests/integration/main.rs` (arachne/classic agreement).
- Doc edits listed in `packet.spec.md` §Doc Impact Statement.

## Out of Scope

- The DEV-123 region-split half. `is_seam_paint_semantic` and its **five** call sites in `crates/slicer-core/src/algos/paint_segmentation/mod.rs` — two inside `mesh_has_any_paint`, one inside the `let dominant_semantic = { … }` block in `execute_paint_segmentation`, and two inside `painted_subsets`' facet loop and stroke loop — are already correct and must not be edited. `paint_channel_seam_strokes_do_not_partition_regions` is a guard, not a target.
- Any change to `SlicedRegion`, `PaintSemantic`, `PaintValue`, `RegionMapIR`, or any schema version constant. `project_seam_planning_view` (`crates/slicer-wasm-host/src/marshal/in_.rs`) was verified to forward every `segment_annotations` key and every `Option<PaintValue>` slot 1:1 (it imposes only a semantic-key sort), and the WIT records `segment-annotations-entry` / `segment-annotations-polygon` (`crates/slicer-schema/wit/deps/ir-types.wit`) already carry `paint-semantic`'s `custom(string)` case. No IR or WIT edit is required.
- `apply_seam_paint_bias`'s scoring arithmetic (the `* 0.1` enforcer multiplier, the blocker `retain`) and its documented inverted score direction.
- Shell-parameter resolution in `execute_paint_segmentation` (`region_map.configs.first()`, `RoleWidthContext`) — that is DEV-122 / packet 207.
- `build_modifier_segment_annotations` and its D14 BASE-chain-only invariant. The seam writer is a sibling, not a modification.
- Support-paint routing into the support generator.

## Authoritative Docs

- `docs/DEVIATION_LOG.md` — very large; **delegate** a ranged read returning only the DEV-123, DEV-133, DEV-134 rows. Also the AC-8 edit target.
- `docs/02_ir_schemas.md` — direct ranged read of the `SlicedRegion` section only; confirms `segment_annotations` is an existing side map needing no version bump.
- `crates/slicer-schema/wit/deps/ir-types.wit` — direct read of the `variant paint-semantic` / `record segment-annotations-entry` declarations. `docs/03_wit_and_manifest.md` mentions `segment-annotations-*` only as a wildcard and does NOT carry these definitions; read the WIT file.
- `docs/05_module_sdk.md` — direct ranged read of the "Paint-seam consumption" section only.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp` — `gather_enforcers_blockers` is the delivery shape being borrowed: it reads `mv->seam_facets` for volumes where `is_seam_painted()` holds, builds enforcer/blocker AABB trees, and `GlobalModelInfo::is_enforced` / `is_blocked` then radius-query each perimeter point with the perimeter flow width. Borrow the proximity-query shape; do NOT borrow the 3D `Point`/`coord_t` units.
- `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.hpp` — `enum class EnforcedBlockedSeamPoint { Blocked, Neutral, Enforced }` and `SeamPlacer::init` / `SeamPlacer::place_seam`; establishes that upstream applies seam paint on finished loops, generator-independently, which is the parity argument for DEV-134.
- `OrcaSlicerDocumented/src/libslic3r/TriangleSelector.hpp` — `enum class EnforcerBlockerType { NONE, ENFORCER, BLOCKER, … }`; the source of the loader's state-1/state-2 mapping. Deliberately NOT ported as a type; PnP keeps the string-named `PaintSemantic::Custom` form.
- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp` — negative evidence only: `process_classic` and `process_arachne` contain no seam-paint references, which is why the classic-only bias wiring has no canonical counterpart.

## Acceptance Summary

Criteria live in `packet.spec.md`; referenced here by ID only.

- Positive: `AC-1` through `AC-8`. Refinements not restated in their Given/When/Then text:
  - `AC-1` and `AC-3` must run on the same fixture-driven call so the index-alignment contract is proven against real loader output, not a synthetic map.
  - `AC-6`'s classic arm exists to prove *agreement*, not merely that arachne changed; a classic regression fails AC-6 as surely as an arachne one.
  - `AC-7` is a static check and must accept every name-resolution-equivalent import form (`slicer_core::perimeter_utils::seam_paint_boxes`, a braced `perimeter_utils::{…, seam_paint_boxes, …}` list — including the multi-line wrapped rendering rustfmt actually produces, where the name lands mid-line — or a bare in-scope `seam_paint_boxes` after a `use`). No alternative may be line-anchored (`^\s*seam_paint_boxes,`): measured with `rustfmt --edition 2021`, both post-fix import lists wrap into multi-line braced groups that pack `seam_paint_boxes` mid-line, so a line-anchored form fails on a correct tree. `AC-7`'s command therefore uses `rg -qU` multiline matching plus a word-bounded bare-name fallback guarded by the absence of any local `fn seam_paint_box*` in the same file.
- Negative: `AC-N1` (support-named `Custom` strings stay Neutral), `AC-N2` (no spurious seam key without seam paint), `AC-N3` (DEV-123 region-split guard unregressed).
- Cross-packet impact: packet 207 edits `execute_paint_segmentation`'s shell-parameter block in the same function this packet adds a writer call to. 207 must be authored and executed against the post-206 tree. No shared symbol is contested: 206 touches the head guards and the tail stamping; 207 touches the `configs.first()` block and `propagate_top_bottom`'s call site.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `mkdir -p target && cargo test -p slicer-runtime --test executor seam_paint_ 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | AC-1/2/3 + AC-N2 writer behaviour in one filtered run | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `mkdir -p target && cargo test -p slicer-runtime --test executor paint_channel_seam_strokes_do_not_partition_regions 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | AC-N3 DEV-123 non-regression guard | FACT pass/fail |
| `mkdir -p target && cargo test -p seam-planner-default --test seam_paint_semantic_exactness_tdd 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | AC-4/5 + AC-N1 exact-match classification | FACT pass/fail |
| `mkdir -p target && cargo test -p seam-planner-default --test seam_canonical_visibility_tdd 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | Rewritten `paint_annotations_set_point_type` and the rest of the visibility suite | FACT pass/fail |
| `mkdir -p target && cargo test -p slicer-runtime --test integration arachne_seam_paint_bias 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | AC-6 arachne/classic agreement | FACT pass/fail |
| `mkdir -p target && cargo test -p classic-perimeters --test classic_perimeters_tdd 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | Classic unregressed by the `seam_paint_boxes` promotion | FACT pass/fail |
| `rg -c '^pub fn seam_paint_boxes' crates/slicer-core/src/perimeter_utils.rs \| grep -qx 1 && echo PASS` | AC-7 promotion landed (declared exactly once) | FACT pass/fail |
| `cargo xtask build-guests --check` | Guest freshness after `slicer-core` + three core-module edits | FACT: reports `STALE:` or not |
| `cargo check --workspace --all-targets` | Whole-tree compile incl. test targets | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Lint gate | FACT pass/fail |
| `cargo xtask check-deviations --check` | Doc 07 generated deviation map not stale after the three row edits | FACT pass/fail |

## Step Completion Expectations

- The `seam_paint_boxes` promotion (implementation-plan Step 4) must land **before** the arachne call site (Step 5); otherwise arachne would need a second private copy and AC-7 could not pass.
- The exact-match change in `seam-planner-default` (Step 6) and the writer (Steps 2–3) are independent in code but must be verified together: with the writer landed and the classifier unfixed, support paint and seam paint would both classify as seam intent and AC-N1 would fail while every other AC passed.
- `cargo xtask build-guests --check` must be run (and a rebuild performed if it reports `STALE:`) after **any** step that edits `crates/slicer-core/**` or `modules/core-modules/*/src/**`, i.e. after Steps 2, 4, 5 and 6. A stale-guest failure in the runtime executor/integration buckets will look like a writer bug and is not one.

## Context Discipline Notes

- `crates/slicer-core/src/algos/paint_segmentation/mod.rs` is long. Never open it in full. The only ranges any step needs are `is_seam_paint_semantic` + `mesh_has_any_paint`, `execute_paint_segmentation`'s guard block, the `painted_subsets` accumulation, `build_modifier_segment_annotations`, and the tail of the layer loop. Locate by symbol name, then read a ±40-line window.
- `crates/slicer-runtime/tests/executor/paint_channel_consumer_paths_tdd.rs` already contains every fixture helper the new tests need (`cube_cilindrical_modifier_path`, `build_layer_plan`, `build_initial_slice_ir`, `build_region_map`, `make_single_object_mesh_ir`, `make_unit_cube_slice_ir`, `make_unit_cube_region_map`). Read the helper block once; do not re-derive fixtures.
- `modules/core-modules/classic-perimeters/src/lib.rs` and `modules/core-modules/arachne-perimeters/src/lib.rs` are both long and both currently carry uncommitted working-tree changes from the DEV-124/125/130 fixes. Ground every read against disk, locate by symbol, and never load either file whole.
- Delegate every `cargo` run and every `OrcaSlicerDocumented/` inspection.
