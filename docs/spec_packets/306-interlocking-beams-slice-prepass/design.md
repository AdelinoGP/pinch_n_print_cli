# Design: 306-interlocking-beams-slice-prepass

## Controlling Code Paths

- Primary code path: a new ungated kernel `crates/slicer-core/src/algos/interlocking/` (`voxel.rs` + `mod.rs`), driven by a new host built-in `crates/slicer-runtime/src/builtins/interlocking_producer.rs`, registered on a new host-only stage `PrePass::InterlockingBeams` in `crates/slicer-runtime/src/prepass.rs` immediately after the `PrePass::PaintSegmentation` `run_builtin_stage` call, writing back through `Blackboard::replace_slice_ir`.
- Neighboring tests/fixtures: `crates/slicer-runtime/tests/executor/cube_4color_phase5_tdd.rs` (the `slice_cube` / `motion` helpers and the `resources/cube_4color.3mf` fixture the end-to-end AC reuses), `crates/slicer-scheduler/tests/contract/stage_list_consistency_tdd.rs` (`HOST_ONLY_STAGES`), `crates/slicer-core/src/algos/bridge_over_infill.rs` (the ungated-`algos`-module precedent), `crates/slicer-runtime/src/slice_postprocess_prepass.rs` (`commit_shell_classification_builtin`, the clone-mutate-`replace_slice_ir` precedent).
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

- **The kernel is ungated.** `crates/slicer-core/src/algos/mod.rs` gates every module except `bridge_over_infill` behind `host-algos`. `interlocking` follows `bridge_over_infill`: `pub mod interlocking;` with no `cfg`, and no `required-features` on the `algo_interlocking_tdd` test target. This is deliberate — a gated test target under a bare `cargo test -p slicer-core` compiles to zero tests and prints a clean `ok` (CLAUDE.md §"Feature-gated test files report green when they don't compile"). AC-21 pins it.
- **`InterlockingParams` is a watched struct from the moment it exists.** Six named `pub` fields under `crates/*/src` puts it on the `cargo xtask check-literals` watchlist (`docs/21_data_defaults_and_fixtures.md`): every **test** literal needs `..` FRU or an `// exhaustive: <reason>` waiver; production `src/` literals stay exhaustive. The gate runs before committing and as the `cargo xtask test` preflight.
- **`PrepassExecutionError` gains a variant, and its `Display` impl is a hand-written `match`, not a derive.** Adding `Interlocking { source: InterlockingBuiltinError }` without the matching `Display` arm is a compile error in the same file; both land in the same step. The enum derives `Clone + PartialEq + Eq`, so `InterlockingBuiltinError` must derive them too — the `PaintSegmentation` variant carries a `String` precisely because its source type does not.
- **The `interlocking_beam` rename has a nine-site blast radius across three files** (`crates/slicer-ir/src/resolved_config.rs` ×4, `crates/slicer-core/src/algos/paint_segmentation/mod.rs` ×3, `crates/slicer-runtime/tests/executor/cube_4color_phase5_tdd.rs` ×2), verified at authoring. `resolved_config.rs`'s sites include the `cli` declaration, the hand-written `PartialEq` arm, and the `to_config_map` omission comment — the field is not covered by a derive, so a missed site is a silent behaviour change, not a compile error. `docs/07_implementation_status.md` carries two further mentions, updated at the completion gate; `docs/spec_packets/_OLD/96_paint-segmentation-phase5-width-limit.md` also mentions it and is **out of bounds** (another packet's directory).
- <!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- **Canonical's `ignored_gap_ = 100u` is a scaled constant and must be divided by 100.** It is `100` in OrcaSlicer's 1 nm units — 0.0001 mm — so it becomes `1` internal unit here, not `100`. It is the morphological-closing radius that merges the two meshes into one volume in `compute_unioned_volume_regions`; carrying it over unscaled would close a 0.01 mm gap instead of a 0.0001 mm one. The same division applies to `rounding_errors = 5` in `handle_thin_areas`, which becomes a sub-unit value and must be handled explicitly (see §Data and Contract Notes).
- Schema/version constants and event-specific locking: not applicable — this packet bumps no public version constant and emits no progress event.

## Code Change Surface

- Selected approach: a whole-object host prepass built-in over the committed `SliceIR`, mirroring canonical's `PrintObject`-scoped call. The producer clones `Vec<SliceIR>` off the blackboard, groups regions by `object_id`, resolves each object's `ResolvedConfig` through `RegionMapIR::config_for`, gates on the six keys, runs the kernel per object, and commits with `replace_slice_ir` — the shape `commit_shell_classification_builtin` already uses.
- Exact functions, traits, manifests, tests, and fixtures:
  - `crates/slicer-core/src/algos/interlocking/voxel.rs` — **new**. `Vec3Units` (i64 triple, internal units), `GridPoint3` (i64 triple, cell coordinates), `DilationKernelType::{Cube, Diamond, Prism}`, `DilationKernel::new(kernel_size: GridPoint3, kind: DilationKernelType) -> Self` with `relative_cells: Vec<GridPoint3>`, `VoxelGrid { cell_size: Vec3Units }` with `to_grid_coord(coord: i64, dim: usize) -> i64`, `to_grid_point`, `to_lower_coord`, `to_lower_corner`, `to_polygon(p: GridPoint3) -> Polygon`, `walk_line`, `walk_polygons`, `walk_areas`, `walk_dilated_polygons`, `walk_dilated_areas`, `dilate`. Callbacks are `&mut dyn FnMut(GridPoint3) -> bool` returning `false` to stop the walk early, matching canonical's `process_cell_func` contract.
  - `crates/slicer-core/src/algos/interlocking/mod.rs` — **new**. `InterlockingParams { beam: bool, beam_width_mm: f32, beam_layer_count: i32, depth_cells: i32, orientation_deg: f32, boundary_avoidance_cells: i32 }`; `InterlockingRegionPair { region_index_a: usize, region_index_b: usize, outer_wall_width_a_units: i64, outer_wall_width_b_units: i64 }`; `generate_interlocking_structure(layers: &mut [SliceIR], object_id: &ObjectId, pairs: &[InterlockingRegionPair], params: &InterlockingParams) -> bool` plus the private `get_shell_voxels`, `add_boundary_cells`, `compute_unioned_volume_regions`, `generate_microstructure`, `apply_microstructure_to_outlines`, `grow_border_areas_perpendicular`, `handle_thin_areas`, and an `expolygons_rotate` / `polygons_rotate` helper pair (none exists in `polygon_ops` today).
  - `crates/slicer-core/src/algos/mod.rs` — add `pub mod interlocking;` with no `cfg`.
  - `crates/slicer-core/src/algos/paint_segmentation/mod.rs` — rename three `mmu_segmented_region_interlocking_beam` sites to `interlocking_beam` (the `run_phase5_width_limit` read, its doc comment, and the driver test's struct literal). No behaviour change.
  - `crates/slicer-ir/src/resolved_config.rs` — rename four sites; add five new `cli` declarations in the same block.
  - `crates/slicer-runtime/src/builtins/interlocking_producer.rs` — **new**. `commit_interlocking_builtin(bb: &mut Blackboard) -> Result<(), InterlockingBuiltinError>`, `InterlockingBuiltinError` (deriving `Debug + Clone + PartialEq + Eq`), and the private `tool_index_for_region` / `region_pairs_for_object` / `role_width_context_from` helpers.
  - `crates/slicer-runtime/src/builtins/mod.rs` — add the module declaration and the `pub use` of the two public items.
  - `crates/slicer-runtime/src/prepass.rs` — add the `PrepassExecutionError::Interlocking` variant, its `Display` arm, and the `run_builtin_stage` registration.
  - `crates/slicer-scheduler/src/execution_plan.rs` — insert `"PrePass::InterlockingBeams"` into `STAGE_ORDER`.
  - `crates/slicer-scheduler/tests/contract/stage_list_consistency_tdd.rs` — add the stage to `HOST_ONLY_STAGES` and assert its positional relations.
  - New tests: `crates/slicer-core/tests/algo_interlocking_tdd.rs`, `crates/slicer-ir/tests/resolved_config_interlocking_tdd.rs`, `crates/slicer-runtime/tests/executor/prepass_interlocking_stage_order_tdd.rs`, `crates/slicer-runtime/tests/executor/cube_4color_interlocking_tdd.rs`, plus two `mod` lines in `crates/slicer-runtime/tests/executor/main.rs`.
- Rejected alternatives and reasons:
  - **A guest `interlocking` module** (the tier table's owner). Rejected: `PrepassStageOutput` has no `SliceIR` variant, so a prepass module cannot return mutated slices; a `Layer::` module sees one layer while the voxel cell spans `2 * beam_layer_count`; and a second coarse `SliceIR` reader/writer on a shared stage cycles `validate_cycles`. Three independently sufficient reasons, set out in `requirements.md` §Problem Statement.
  - **Splitting P89 and P90 into two packets** (the queue's shape). Rejected: canonical's enabling gate reads `interlocking_depth`, a P90 key, so a P89-only packet cannot open its own gate without hardcoding it, and `interlocking_orientation` threads through every function the packet would author. Ticket 97 is dissolved.
  - **Keeping `mmu_segmented_region_interlocking_beam` and aliasing the canonical name onto it.** Rejected: the map's ticket-07 ruling is to standardise to Orca's names rather than document a rename layer, and an alias is a second spelling that drifts. The key has exactly nine code sites — a rename is cheaper than an alias.
  - **A new `PaintSemantic` or a per-region extruder field on `SlicedRegion`** to carry tool identity. Rejected: `variant_chain`'s `("material", PaintValue::ToolIndex(n))` already is that identity, produced by `PrePass::PaintSegmentation` and consumed by `region_mapping`; adding a field would be a second source of truth for the same fact and a schema change this packet does not need.
  - **Reusing `xor` from `polygon_ops` for canonical's `xor_ex(skin, layers[n-1])`.** Accepted, not rejected — `polygon_ops::xor` is the exact operation and is ungated. Noted here because the skin step is the one place canonical uses XOR rather than difference, and substituting `difference_ex` would silently drop the downward-facing skin.

## Files in Scope (read + edit)

Target at most 3 primary files; justify extras and consider splitting. The extras below are each a single mechanical insertion (one module line, one enum variant, one stage string) and are budgeted into the step that owns them rather than a step of their own.

- `crates/slicer-core/src/algos/interlocking/voxel.rs` — role: the voxel grid and dilation kernels; expected change: new file, ported from canonical `VoxelUtils` with the attribution header.
- `crates/slicer-core/src/algos/interlocking/mod.rs` — role: the generator; expected change: new file, ported from canonical `InterlockingGenerator` with the attribution header.
- `crates/slicer-runtime/src/builtins/interlocking_producer.rs` — role: the host seam; expected change: new file, region pairing + per-object gate + `replace_slice_ir`.
- `crates/slicer-ir/src/resolved_config.rs` — role: the six keys; expected change: five new `cli` lines and one rename across four sites. Ranged edit only.
- `crates/slicer-core/src/algos/paint_segmentation/mod.rs` — role: the rename's second read site; expected change: three identifier renames, no behaviour change. Ranged edit only.
- `crates/slicer-runtime/src/prepass.rs` — role: stage registration; expected change: one error variant, one `Display` arm, one `run_builtin_stage` block. Ranged edit only.
- `crates/slicer-scheduler/src/execution_plan.rs` — role: declared stage order; expected change: one string inserted into `STAGE_ORDER`.

## Read-Only Context

Include ranges for files over 300 lines.

- `crates/slicer-ir/src/slice_ir.rs` — over 3000 lines; open only the `ExPolygon` / `Polygon` / `Point2` declarations, `mm_to_units` / `units_to_mm`, `SlicedRegion` / `SliceIR`, `RegionKey` / `RegionMapIR::config_for`, `PaintValue`, `MODIFIER_FOOTPRINT_REGION_ID` / `is_modifier_namespace_id` — purpose: exact field names and the hole-winding convention (`contour` CCW, `holes` CW).
- `crates/slicer-core/src/polygon_ops.rs` — locate by symbol; open only `union_ex`, `intersection_ex`, `difference_ex`, `xor`, `offset`, `opening_ex`, `closing_ex`, `OffsetJoinType` — purpose: signatures and the join-type argument, and to confirm no rotation helper exists.
- `crates/slicer-core/src/flow.rs` — `RoleWidthContext` and `resolve_role_width` only — purpose: the 4-argument order `(role, first_layer, bridge, context)`, which is not `(role, context, first_layer)`.
- `crates/slicer-runtime/src/prepass.rs` — the `PrepassExecutionError` enum, its `Display` arms, and the `PrePass::PaintSegmentation` `run_builtin_stage` block — purpose: the registration shape and the insertion point.
- `crates/slicer-runtime/src/slice_postprocess_prepass.rs` — `commit_shell_classification_builtin` only — purpose: the clone-mutate-`replace_slice_ir` precedent and its error type shape.
- `crates/slicer-runtime/tests/executor/cube_4color_phase5_tdd.rs` — whole file, under 200 lines — purpose: the `slice_cube` / `motion` helpers AC-20 reuses and the two rename sites.
- `crates/slicer-scheduler/tests/contract/stage_list_consistency_tdd.rs` — the `HOST_ONLY_STAGES` const and the two partition tests — purpose: what adding a host-only stage obliges.
- `crates/slicer-core/src/algos/bridge_over_infill.rs` — the module header only — purpose: the ungated-module precedent and the attribution-header format in context.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` — delegate; never load.
- `docs/spec_packets/_OLD/**` and every other `docs/spec_packets/<other>/` directory — another packet's files; never edit, and inspect only through a SUMMARY dispatch.
- `crates/slicer-gcode/src/serialize.rs` — `ORCA_CONFIG_PADDING` is map rule 2 non-evidence, carries no twin for any of these six keys, and ticket 132 owns its derivation. Never opened by this packet.
- `crates/slicer-core/src/algos/paint_segmentation/width_limit.rs` — Phase 5's kernel. The rename does not reach it (it names only the two `mmu_segmented_region_*` keys canonical really does spell that way).
- `target/`, `Cargo.lock`, generated code, vendored dependencies — never load.
- Unrelated crates (`slicer-gcode`, `slicer-wasm-host`, `modules/core-modules/**`) — delegate symbol lookups; do not browse.

## Expected Sub-Agent Dispatches

- Question: what are `DilationKernel`'s exact `CUBE` / `DIAMOND` / `PRISM` construction loops and bounds?; scope: `OrcaSlicerDocumented/src/libslic3r/Feature/Interlocking/VoxelUtils.cpp`; return: `SNIPPETS` (≤30 lines); purpose: Step 1, AC-1.
- Question: what are the exact bodies of `walkLine`, `_walkAreas`, `walkAreas` and `dilate`, including the half-cell XY translation `_walkAreas` assumes?; scope: `OrcaSlicerDocumented/src/libslic3r/Feature/Interlocking/VoxelUtils.cpp`; return: `SNIPPETS` (≤30 lines each, one dispatch per function); purpose: Step 1, AC-2 – AC-4.
- Question: what does `generateMicrostructure` compute for `middle` and `width[2]`, and how does phase 1 differ from phase 0?; scope: `OrcaSlicerDocumented/src/libslic3r/Feature/Interlocking/InterlockingGenerator.cpp`; return: `SNIPPETS` (≤30 lines); purpose: Step 2, AC-5.
- Question: what is the exact sequence of set operations in `applyMicrostructureToOutlines`, including which structure phase index each layer uses?; scope: `OrcaSlicerDocumented/src/libslic3r/Feature/Interlocking/InterlockingGenerator.cpp`; return: `SNIPPETS` (≤30 lines); purpose: Step 2, AC-6 – AC-8.
- Question: what are the exact offsets and intersections in `growBorderAreasPerpendicular` and `handleThinAreas`, and what value does `close_gaps` take?; scope: `OrcaSlicerDocumented/src/libslic3r/Feature/Interlocking/InterlockingGenerator.cpp`; return: `SNIPPETS` (≤30 lines each); purpose: Step 3, AC-10.
- Question: what are the `min`, `max`, `sidetext` and default of each of the six `interlocking_*` `ConfigOptionDef`s, and what is each one's `ConfigOption*` type?; scope: `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp`; return: `FACT` (≤5 lines); purpose: Step 4, AC-13.
- Question: which test/non-test files construct a `RoleWidthContext` literal today?; scope: `crates/**/*.rs`; return: `LOCATIONS` (≤20); purpose: Step 6 — the producer builds one and must match the existing construction idiom rather than invent a second.
- Question: does any file outside the three named in `requirements.md` §In Scope still contain `mmu_segmented_region_interlocking_beam` after Step 4?; scope: `crates/**`, `modules/**`, `docs/**`; return: `LOCATIONS` (≤20); purpose: Step 4 exit, AC-14.

## Data and Contract Notes

- IR/manifest contracts: no IR schema change and no manifest change. `SlicedRegion.polygons` is mutated in place on a cloned `Vec<SliceIR>`; `SliceIR::schema_version` is untouched. No module declares any of the six keys, and none is added to `to_config_map`, so no `ConfigView` sees them.
- WIT boundary: untouched. Nothing in this packet's change surface feeds the guest build, so the wasm-staleness constraint does not apply and the snippet is deliberately absent.
- **Region-pair identity.** Canonical iterates `PrintObject::printing_region` pairs and skips those whose `extruder(FlowRole::frExternalPerimeter)` match. This tree has no per-region extruder field on `SlicedRegion`; the equivalent is the material paint variant, `("material", PaintValue::ToolIndex(n))` in `SlicedRegion.variant_chain`, produced by `PrePass::PaintSegmentation`. Regions with no `"material"` entry take the object's base tool index. Regions whose `region_id` is `MODIFIER_FOOTPRINT_REGION_ID` or satisfies `is_modifier_namespace_id` are excluded from pairing and from the voxelised volume (AC-N5). This is deviation row 1.
- **Per-region external-perimeter width.** Canonical's `printing_region.flow(print_object, frExternalPerimeter, 0.1).scaled_width()` becomes `mm_to_units(resolve_role_width(ExtrusionRole::OuterWall, false, false, &ctx))` where `ctx` is a `RoleWidthContext` built from that region's `ResolvedConfig`. Canonical's `0.1` argument is a nominal layer height used only to derive flow; `resolve_role_width` is layer-height-independent, so no equivalent argument is needed.
- **Rotation has no helper in this tree.** `polygon_ops` has no rotate function (the only `rotate` in `slicer-core` is a private helper inside `bridge_over_infill.rs`). The kernel authors `polygons_rotate` / `expolygons_rotate` locally, rotating about the origin by radians with `f64` sin/cos and rounding to `i64` units. Rotation is applied before the voxel walk and unapplied on output, so any rounding must be symmetric: rotate by `+θ` then by `−θ` must return every point to within 1 unit of itself, or beams will drift. Assert this as part of AC-8's fixture rather than trusting it.
- **`ignored_gap_` and `rounding_errors` are canonical constants in 1 nm units.** `ignored_gap_ = 100` becomes `1` internal unit; `rounding_errors = 5` becomes `0.05` units, i.e. sub-unit. Where a sub-unit closing/offset radius would round to zero, use `1` unit and record the choice in the ADR — a zero-radius `closing` is a different operation from a small one, and silently rounding to zero would drop canonical's rounding-error tolerance in `handle_thin_areas`.
- Determinism/scheduler constraints: the cell sets are gathered into `HashSet<GridPoint3>` for membership but must be **sorted before iteration** wherever iteration order affects output — `apply_microstructure_to_outlines` accumulates into per-layer `ExPolygons` and `handle_thin_areas` accumulates into `near_interlock_per_layer`, and both feed `union_ex`, whose output vertex order depends on input order. Use `BTreeSet<GridPoint3>` or sort into a `Vec` before the accumulation loops. Canonical uses `std::unordered_set` and does not have this obligation because it has no byte-identicality contract; this port does.

## Locked Assumptions and Invariants

- The six keys default to canonical's values, and `interlocking_beam = false` by default, so **every existing print is bitwise unchanged** (AC-11). No behaviour lock is introduced for default configurations.
- The stage is host-only and absent from `VALID_STAGES`: no module manifest can ever target `PrePass::InterlockingBeams`. Reversing that later is a schema change, not a config change.
- The rename is one-way: `mmu_segmented_region_interlocking_beam` is retired with no alias, so a config file or 3MF using the old spelling silently loses the setting. This matches the map's ticket-07 ruling and ticket 100's precedent, and matches how this port already treats unimplemented Orca keys (silently dropped, no reject list).
- The two keys canonical genuinely spells `mmu_segmented_region_*` keep those names and stay on Phase 5. Ticket 98 (P91) is not pre-empted.

## Risks and Tradeoffs

- **The voxel walk is the largest single port in this packet and the least testable in isolation.** Mitigated by Step 1 landing `voxel.rs` with AC-1 – AC-4 before any generator code exists: a wrong `to_grid_coord` floor or a skipped cell in `walk_line` would otherwise surface as "the beams look slightly wrong" three steps later.
- **Performance is unmeasured.** The generator voxelises every layer of every differently-toned region pair; on an object with many painted variants the pair count is quadratic in distinct tools. Canonical has the same shape and ships it, and the whole path is gated off by default, so this packet does not optimise — but it does not claim a cost either. If a `--instrument-stderr` run after landing shows the stage dominating, that is a follow-up, not a defect in this design.
- **`handle_thin_areas` is the hardest function to assert.** Its output depends on two morphological opens, a four-way intersection and an iteration count derived from `detect / min_line`. AC-10 asserts the observable consequence (thin band gains area, only under air filtering) rather than the intermediate sets, which is weaker than a golden but is the strongest invariant available without a runnable canonical.
- **A partial rename is a silent behaviour change in one place.** `resolved_config.rs`'s hand-written `PartialEq` arm compares the field by name; renaming the field but not the arm is a compile error, but renaming the `cli` string literal without the field (or vice versa) compiles and silently stops the key resolving. Step 4's exit condition greps for the old spelling across `crates/` and `modules/` rather than relying on the compiler.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 2 — the generator body, five ported functions)
- Highest-risk dispatch and required return format: the per-function canonical reads for `applyMicrostructureToOutlines` and `handleThinAreas`, which must return `SNIPPETS` capped at 30 lines each, one function per dispatch. A single combined "summarise the generator" dispatch will overflow and lose the set-operation ordering that AC-7 and AC-10 assert.

## Open Questions

- `[FWD]` Canonical's `handleThinAreas` sets both regions' slices with surface type `stInternal`. This tree's `SlicedRegion` has no surface-type field on `polygons` — the equivalent classification lives in `top_shell_index` / `bottom_shell_index`, set by `PrePass::ShellClassification`, which runs **before** this stage. The implementer must decide whether the interlocked regions need reclassification and, if so, whether that is a re-run of the shell pass (out of scope here) or an accepted divergence. Resolve by measuring: if AC-18's support-candidate assertion passes and the shell indices on the interlocked regions still describe their footprint, record the divergence in the ADR and proceed; if the indices are visibly wrong on the beam boundary, stop and raise it rather than re-running the shell pass inside this packet.
- `[FWD]` Canonical's `throw_on_cancel` callback threads through every loop. This tree's prepass built-ins take no cancellation token. Port the structure without the callback and leave the loop shapes intact so a token can be threaded later; do not invent a cancellation mechanism in this packet.
