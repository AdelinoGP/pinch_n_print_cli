# Design: 306-interlocking-beams-slice-postprocess

## Controlling Code Paths

- Primary code path: a new ungated **analysis** kernel `crates/slicer-core/src/algos/interlocking/` (`voxel.rs` + `mod.rs`), driven by a new host built-in `crates/slicer-runtime/src/builtins/interlocking_lattice_producer.rs` on a new host-only stage `PrePass::InterlockingLattice` registered immediately after the `PrePass::PaintSegmentation` `run_builtin_stage` call, committing a new `InterlockingLatticeIR`; and a new guest module `modules/core-modules/interlocking-beams/` on `Layer::SlicePostProcess` that reads that IR plus its own layer's `SliceIR` and returns `LayerStageCommit::SlicePostProcess { polygon_updates, .. }`.
- Neighboring tests/fixtures: `crates/slicer-runtime/tests/executor/cube_4color_phase5_tdd.rs` (the `slice_cube` / `motion` helpers and the `resources/cube_4color.3mf` fixture), `crates/slicer-scheduler/tests/contract/stage_list_consistency_tdd.rs` (`HOST_ONLY_STAGES`), `crates/slicer-core/src/algos/bridge_over_infill.rs` (the ungated-`algos`-module precedent), `modules/core-modules/seam-placer/seam-placer.toml` (the narrow-write manifest precedent), `modules/core-modules/lightning-infill/` (the prepass-IR-consumed-in-a-layer-stage precedent).
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

- **The seam is `Layer::SlicePostProcess`, and the commit type is why.** `LayerStageCommit::SlicePostProcess { polygon_updates: Vec<(RegionKey, Vec<ExPolygon>)>, path_z_updates }` (`crates/slicer-ir/src/stage_io.rs`) is merged by replacing `existing.regions[ridx].polygons` (`crates/slicer-runtime/src/layer_executor.rs`), which is exactly canonical's `slices.set(...)`. The `RegionKey` carries `variant_chain`, which is how the two interlocked material variants are distinguished. Do not reach for a host built-in for the application half — the earlier revision of this packet did, on the false premise that `PrepassStageOutput`'s missing `SliceIR` variant applies to layer stages. It does not; that limitation is prepass-only.
- **The write path must stay narrow.** `writes = ["SliceIR.regions.polygons"]`, never `["SliceIR"]`. Two coarse `SliceIR` reader-writers on one stage cycle `validate_cycles` (`crates/slicer-scheduler/src/validation.rs`) → `topological_sort` (`crates/slicer-scheduler/src/topology.rs`) with `SchedulerError::CyclicDependency`, and packet 303 plans a coarse `elefant-foot` on this stage. `seam-placer.toml` and `part-cooling.toml` are the shipping precedents for dotted paths. AC-N6 pins it.
- **The analysis kernel is ungated.** `crates/slicer-core/src/algos/mod.rs` gates every module except `bridge_over_infill` behind `host-algos`. `interlocking` follows `bridge_over_infill`: `pub mod interlocking;` with no `cfg`, and no `required-features` on the `algo_interlocking_tdd` target. A gated test target under a bare `cargo test -p slicer-core` compiles to zero tests and prints a clean `ok` (CLAUDE.md §"Feature-gated test files report green when they don't compile"). AC-23 pins it.
- <!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
- <!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- **Canonical's `ignored_gap_ = 100u` and `rounding_errors = 5` are 1 nm constants.** `ignored_gap_` becomes `1` internal unit, not `100` — it is the closing radius that merges the two meshes into one volume for `layer_regions`, which the **module** derives from its own layer. `rounding_errors` becomes sub-unit; where a sub-unit closing/offset radius would round to zero, use `1` unit and record the choice in the ADR, because a zero-radius `closing` is a different operation from a small one.
- **A module cannot read a key its own manifest does not declare.** `ConfigView::from_declared` (`crates/slicer-ir/src/slice_ir.rs`) whitelists the raw source by the module's schema keys, so an undeclared key is filtered out and the guest's `unwrap_or` fallback wins silently. All six keys are `[config.schema]` rows **and** `ResolvedConfig` fields; neither alone is sufficient.
- **`PrepassExecutionError` gains a variant, and its `Display` impl is a hand-written `match`.** Adding `InterlockingLattice { source: InterlockingLatticeBuiltinError }` without the arm is a compile error in the same file. The enum derives `Clone + PartialEq + Eq`, so the source type must too — the `PaintSegmentation` variant carries a `String` precisely because its source does not, and that fallback is available.
- **The `interlocking_beam` rename has a nine-site blast radius across three files** (`crates/slicer-ir/src/resolved_config.rs` ×4, `crates/slicer-core/src/algos/paint_segmentation/mod.rs` ×3, `crates/slicer-runtime/tests/executor/cube_4color_phase5_tdd.rs` ×2), verified at authoring. `resolved_config.rs`'s sites include the `cli` declaration, the hand-written `PartialEq` arm and the `to_config_map` omission comment — the field is not covered by a derive, so a missed site is a silent behaviour change, not a compile error.
- Schema/version constants: `InterlockingLatticeIR` is a **new** IR, so it introduces its own `CURRENT_INTERLOCKING_LATTICE_SCHEMA_VERSION` rather than bumping an existing constant. Do not touch `CURRENT_SLICE_IR_SCHEMA_VERSION` — no existing IR gains a field.

## Code Change Surface

- Selected approach: split along canonical's own function boundary. The host prepass answers "which cells does the beam lattice occupy?", which needs every layer because `add_boundary_cells` XORs layer N against N−1. The guest module answers "given those cells, what shape are this layer's two regions now?", which needs nothing but its own layer. The beam **pattern** lives in the module so a community fork can replace it.
- Exact functions, traits, manifests, tests, and fixtures:
  - `crates/slicer-core/src/algos/interlocking/voxel.rs` — **new**. `Vec3Units`, `GridPoint3`, `DilationKernelType::{Cube, Diamond, Prism}`, `DilationKernel::new(kernel_size: GridPoint3, kind: DilationKernelType)` with `relative_cells: Vec<GridPoint3>`, `VoxelGrid { cell_size: Vec3Units }` with `to_grid_coord`, `to_grid_point`, `to_lower_coord`, `to_lower_corner`, `to_polygon(p) -> Polygon`, `walk_line`, `walk_polygons`, `walk_areas`, `walk_dilated_polygons`, `walk_dilated_areas`, `dilate`. Callbacks are `&mut dyn FnMut(GridPoint3) -> bool`, `false` stopping the walk, matching canonical's `process_cell_func`.
  - `crates/slicer-core/src/algos/interlocking/mod.rs` — **new**. `InterlockingParams { beam, beam_width_mm, beam_layer_count, depth_cells, orientation_deg, boundary_avoidance_cells }`; `InterlockingPair { region_key_a, region_key_b, cells }`; `InterlockingLattice { cell_size, rotation_rad, pairs }`; `compute_interlocking_lattice(layers: &[SliceIR], object_id: &ObjectId, params: &InterlockingParams) -> InterlockingLattice` plus private `get_shell_voxels`, `add_boundary_cells`, and a `polygons_rotate` / `expolygons_rotate` helper pair (none exists in `polygon_ops`).
  - `crates/slicer-core/src/algos/mod.rs` — add `pub mod interlocking;` with no `cfg`.
  - `crates/slicer-ir/src/` — **new** `InterlockingLatticeIR` + `CURRENT_INTERLOCKING_LATTICE_SCHEMA_VERSION`, registered alongside the other prepass IRs.
  - `crates/slicer-schema/wit/` — **new** WIT record(s) for the lattice; edit the canonical source only, per `CLAUDE.md` §"WIT/Type Changes Checklist" (both host `bindgen!` and the guest macro read these files directly; there is no inline copy).
  - `crates/slicer-wasm-host/src/binding.rs` — `LayerStageInput` gains `interlocking_lattice: Option<Arc<InterlockingLatticeIR>>`, mirroring `lightning_tree_ir`.
  - `crates/slicer-runtime/src/builtins/interlocking_lattice_producer.rs` — **new**. `commit_interlocking_lattice_builtin`, `InterlockingLatticeBuiltinError`, and private `tool_index_for_region` / `region_pairs_for_object` helpers.
  - `crates/slicer-runtime/src/builtins/mod.rs` — module declaration and `pub use`.
  - `crates/slicer-runtime/src/prepass.rs` — the new error variant, its `Display` arm, and the `run_builtin_stage` registration.
  - `crates/slicer-runtime/src/layer_executor.rs` — thread the lattice onto `LayerStageInput` at the `Layer::SlicePostProcess` call site.
  - `crates/slicer-scheduler/src/execution_plan.rs` — insert `"PrePass::InterlockingLattice"` into `STAGE_ORDER`.
  - `crates/slicer-scheduler/tests/contract/stage_list_consistency_tdd.rs` — add it to `HOST_ONLY_STAGES` and assert positions.
  - `modules/core-modules/interlocking-beams/` — **new** guest module scaffolded with `pnp_cli module new`: `interlocking-beams.toml` (`[stage] id = "Layer::SlicePostProcess"`, `reads = ["SliceIR"]`, `writes = ["SliceIR.regions.polygons"]`, `[claims]` empty, six `[config.schema]` rows) and `src/lib.rs` owning `generate_microstructure`, `apply_microstructure_to_layer`, `grow_border_areas_perpendicular`, `handle_thin_areas`.
  - `crates/slicer-ir/src/resolved_config.rs` — rename four sites; add five new `cli` declarations.
  - `crates/slicer-core/src/algos/paint_segmentation/mod.rs` — rename three sites; no behaviour change.
  - New tests: `crates/slicer-core/tests/algo_interlocking_tdd.rs`, `crates/slicer-ir/tests/resolved_config_interlocking_tdd.rs`, `crates/slicer-scheduler/tests/unit/interlocking_stage_sharing_tdd.rs`, `crates/slicer-runtime/tests/executor/prepass_interlocking_lattice_tdd.rs`, `crates/slicer-runtime/tests/executor/layer_interlocking_beams_tdd.rs`, `crates/slicer-runtime/tests/executor/cube_4color_interlocking_tdd.rs`, plus module-crate unit tests and the aggregator `mod` lines.
- Rejected alternatives and reasons:
  - **The whole pass as a host prepass built-in** — what an earlier revision of this packet specified. Rejected: its decisive reason, "a guest module cannot write slices", is a fact about `PrepassStageOutput` and does not hold at `Layer::SlicePostProcess`, which has `polygon_updates` for exactly this. Its second reason conflated the algorithm's global **analysis** with its per-layer **application**; only the former is global. Keeping the pattern in host code would also freeze the one genuinely swappable part of the algorithm, against rule 4.
  - **The whole pass as a guest module, no prepass** — rejected: `add_boundary_cells` computes `skin = xor_ex(layers[n], layers[n-1])`, and a layer stage sees only `LayerStageInput.slice` for its own layer. There is no cross-layer read at that seam, and inventing one would be a much larger change than a prepass IR.
  - **Putting the beam polygons in the IR instead of the cell set** — rejected: it would move `generate_microstructure` back into the host and freeze the pattern. The IR carries cells; the module stamps the pattern.
  - **Splitting P89 and P90 into two packets** — rejected: canonical's P89 gate reads `interlocking_depth`, a P90 key. Ticket 97 is dissolved.
  - **Keeping `mmu_segmented_region_interlocking_beam` and aliasing** — rejected: ticket 07's ruling is to standardise to Orca's names; an alias is a second spelling that drifts, and the key has only nine sites.
  - **A new `PaintSemantic` or a per-region extruder field on `SlicedRegion`** — rejected: `variant_chain`'s `("material", PaintValue::ToolIndex(n))` already is that identity and a second source of truth would be a schema change for nothing.

## Files in Scope (read + edit)

This packet's surface is wider than three files because it adds an IR across the WIT boundary **and** a new guest module. `implementation-plan.md` splits it so no step edits more than three; the list below is the union, not one step's budget.

- `crates/slicer-core/src/algos/interlocking/voxel.rs` — role: grid + kernels; change: new file, ported from canonical `VoxelUtils`.
- `crates/slicer-core/src/algos/interlocking/mod.rs` — role: the analysis; change: new file, ported from canonical's analysis functions.
- `modules/core-modules/interlocking-beams/src/lib.rs` + `interlocking-beams.toml` — role: the microstructure and outline rewrite; change: new module.
- `crates/slicer-ir/src/` (IR module + `resolved_config.rs`) — role: the new IR and the six keys; change: new type, five `cli` lines, one rename across four sites.
- `crates/slicer-schema/wit/` — role: the WIT record for the lattice; change: canonical source only.
- `crates/slicer-runtime/src/builtins/interlocking_lattice_producer.rs` — role: the analysis seam; change: new file.
- `crates/slicer-runtime/src/prepass.rs`, `layer_executor.rs`, `builtins/mod.rs` — role: registration and threading; change: ranged edits only.
- `crates/slicer-scheduler/src/execution_plan.rs` — role: declared stage order; change: one string.
- `crates/slicer-core/src/algos/paint_segmentation/mod.rs` — role: the rename's second read site; change: three identifier renames.

## Read-Only Context

- `crates/slicer-ir/src/stage_io.rs` — the `LayerStageCommit::SlicePostProcess` variant only — purpose: the exact `polygon_updates` tuple shape.
- `crates/slicer-runtime/src/layer_executor.rs` — over 4000 lines; the `LayerStageCommit::SlicePostProcess` merge arm and the `Layer::SlicePostProcess` dispatch entry only, located by `rg -n 'SlicePostProcess'`.
- `crates/slicer-wasm-host/src/binding.rs` — `LayerStageInput` and its `lightning_tree_ir` field only — purpose: the field to mirror.
- `crates/slicer-ir/src/slice_ir.rs` — over 3000 lines; only `ExPolygon` / `Polygon` / `Point2`, `mm_to_units` / `units_to_mm`, `SlicedRegion` / `SliceIR`, `RegionKey` / `RegionMapIR::config_for`, `ConfigView::from_declared`, `PaintValue`, `MODIFIER_FOOTPRINT_REGION_ID` / `is_modifier_namespace_id` — purpose: field names and the hole-winding convention (`contour` CCW, `holes` CW).
- `crates/slicer-core/src/polygon_ops.rs` — `union_ex`, `intersection_ex`, `difference_ex`, `xor`, `offset`, `opening_ex`, `closing_ex`, `OffsetJoinType` only — purpose: signatures, and to confirm no rotation helper exists.
- `crates/slicer-core/src/flow.rs` — `RoleWidthContext` and `resolve_role_width` only — purpose: the argument order `(role, first_layer, bridge, context)`, which is **not** `(role, context, first_layer)`.
- `modules/core-modules/seam-placer/seam-placer.toml` and `modules/core-modules/part-cooling/part-cooling.toml` — whole files, short — purpose: the narrow dotted `writes` precedent.
- `modules/core-modules/lightning-infill/` — the manifest and the entrypoint signature only — purpose: how a layer-stage module consumes a prepass IR.
- `crates/slicer-runtime/tests/executor/cube_4color_phase5_tdd.rs` — whole file, under 200 lines — purpose: the `slice_cube` / `motion` helpers and the two rename sites.
- `crates/slicer-scheduler/tests/contract/stage_list_consistency_tdd.rs` — the `HOST_ONLY_STAGES` const and the two partition tests.
- `crates/slicer-core/src/algos/bridge_over_infill.rs` — the module header only — purpose: the ungated-module precedent and the attribution header in context.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` — delegate; never load.
- `docs/spec_packets/_OLD/**` and every other `docs/spec_packets/<other>/` directory — another packet's files; never edit. In particular **do not amend packet 303's manifest** to resolve the stage-sharing question: this packet's narrow writes resolve it from its own side.
- `crates/slicer-gcode/src/serialize.rs` — `ORCA_CONFIG_PADDING` is map rule 2 non-evidence, carries no twin for any of these six keys, and ticket 132 owns its derivation.
- `crates/slicer-core/src/algos/paint_segmentation/width_limit.rs` — Phase 5's kernel; the rename does not reach it.
- `target/`, `Cargo.lock`, generated code, vendored dependencies — never load.
- Unrelated crates and modules — delegate symbol lookups; do not browse.

## Expected Sub-Agent Dispatches

- Question: what are `DilationKernel`'s exact `CUBE` / `DIAMOND` / `PRISM` construction loops and bounds?; scope: `OrcaSlicerDocumented/src/libslic3r/Feature/Interlocking/VoxelUtils.cpp`; return: `SNIPPETS` ≤30 lines; purpose: Step 1, AC-1.
- Question: what are the exact bodies of `walkLine`, `_walkAreas`, `walkAreas` and `dilate`, including the half-cell XY translation `_walkAreas` assumes?; scope: same file; return: `SNIPPETS` ≤30 lines each, one dispatch per function; purpose: Step 1, AC-2 – AC-4.
- Question: what are `getShellVoxels` and `addBoundaryCells` verbatim, including the `xor_ex` skin step and the `opening_ex(skin, cell_size.x()/2)` filter?; scope: `.../InterlockingGenerator.cpp`; return: `SNIPPETS` ≤30 lines each; purpose: Step 2, AC-5 – AC-7.
- Question: what does `generateMicrostructure` compute for `middle` and `width[2]`, and how is phase 1 derived from phase 0?; scope: same file; return: `SNIPPETS` ≤30 lines; purpose: Step 3, AC-10.
- Question: what is the exact sequence of set operations in `applyMicrostructureToOutlines`, and which structure-phase index does each layer read?; scope: same file; return: `SNIPPETS` ≤30 lines; purpose: Step 3, AC-11 – AC-13.
- Question: what are `growBorderAreasPerpendicular` and `handleThinAreas` verbatim, including `close_gaps` and the four-way intersection?; scope: same file; return: `SNIPPETS` ≤30 lines each; purpose: Step 4, AC-14.
- Question: what are the `min`, `max`, `sidetext`, `ConfigOption*` type and default of each of the six `interlocking_*` `ConfigOptionDef`s?; scope: `.../PrintConfig.cpp`; return: `FACT` ≤5 lines; purpose: Step 6, AC-19.
- Question: how is a prepass IR registered end to end — schema constant, WIT record, `bindgen!` host side, guest macro side, `LayerStageInput` field — using `LightningTreeIR` as the worked example?; scope: `crates/slicer-ir/**`, `crates/slicer-schema/**`, `crates/slicer-wasm-host/**`; return: `LOCATIONS` ≤20; purpose: Step 5, the widest blast radius in the packet.
- Question: which test/non-test files construct a `RoleWidthContext` literal today, and with which field-init idiom?; scope: `crates/**/*.rs`, `modules/**/*.rs`; return: `LOCATIONS` ≤20; purpose: Step 4.
- Question: does the stage DAG accept a dotted write path against a coarse read of the same root without emitting a reverse edge — what exactly does `seam-placer` rely on?; scope: `crates/slicer-scheduler/src/dag.rs`; return: `SNIPPETS` ≤30 lines; purpose: Step 7, AC-N6 — **if this returns that dotted paths do not in fact avoid the reverse edge, AC-N6 is unsatisfiable and the packet is blocked on ticket 148.**

## Data and Contract Notes

- **IR contract.** `InterlockingLatticeIR` carries `cell_size: Vec3Units`, `rotation_rad: f32`, and per `object_id` a `Vec<InterlockingPair { region_key_a: RegionKey, region_key_b: RegionKey, cells: Vec<GridPoint3> }>`. It carries **cells, not beam polygons**, so the pattern stays in the module. The module selects the cells covering its layer by `cell.z * cell_size.z <= layer < (cell.z + 1) * cell_size.z`, which is arithmetic, not a neighbour read.
- **Manifest contract.** `[stage] id = "Layer::SlicePostProcess"`; `reads = ["SliceIR"]`; `writes = ["SliceIR.regions.polygons"]`. **The lattice is deliberately not a declared IR read.** `validate_ir_reads` (`crates/slicer-scheduler/src/validation.rs`) resolves every declared read against a writer at an earlier stage, and the lattice's producer is a host built-in that mints no module node — so declaring it would leave an unsatisfiable read. `lightning-infill` is the precedent: it consumes `LightningTreeIR` through `LayerStageInput` while its manifest declares only `reads = ["SliceIR"]`; `[claims]` empty — the six keys are scalar parameters of one pass, not an enum selecting between competing algorithms, so rule 4 mints no claim. All six keys appear as `[config.schema]` rows.
- **WIT boundary.** New records for the lattice. Follow `CLAUDE.md` §"WIT/Type Changes Checklist": edit the canonical source at `crates/slicer-schema/wit/` only, search `wit_host.rs` / `dispatch.rs` / `wit_guest` for the affected type, verify type identity across the boundary (a `list<grid-point3>` on one side and a `Vec<GridPoint3>` on the other must resolve to the same record), and run `cargo build --tests` after.
- **Region-pair identity.** Canonical skips region pairs whose `extruder(frExternalPerimeter)` match. The equivalent here is the material paint variant, `("material", PaintValue::ToolIndex(n))` in `variant_chain`, produced by `PrePass::PaintSegmentation`; regions with no `"material"` entry take the object's base tool. Regions whose `region_id` is `MODIFIER_FOOTPRINT_REGION_ID` or satisfies `is_modifier_namespace_id` are excluded from pairing and from the voxelised volume (AC-N5). Deviation row 1.
- **Per-region external-perimeter width.** Canonical's `printing_region.flow(print_object, frExternalPerimeter, 0.1).scaled_width()` becomes `mm_to_units(resolve_role_width(ExtrusionRole::OuterWall, false, false, &ctx))` built from that region's config. Canonical's `0.1` is a nominal layer height used only to derive flow; `resolve_role_width` is layer-height-independent.
- **Rotation has no helper in this tree.** `polygon_ops` has none (the only `rotate` in `slicer-core` is private to `bridge_over_infill.rs`). Both the kernel and the module author one, rotating about the origin by radians with `f64` sin/cos rounded to `i64` units. Apply/unapply must be symmetric to within 1 unit or beams drift; AC-13 asserts it.
- **Determinism.** Cell sets use `HashSet<GridPoint3>` for membership but must be **sorted before iteration** wherever order reaches output — the IR's `cells` vector is serialised and must be deterministic, and the module's accumulation feeds `union_ex`, whose vertex order depends on input order. Emit `cells` sorted (`BTreeSet` or an explicit sort) in the producer. Canonical uses `std::unordered_set` and has no byte-identicality contract; this port does.

## Locked Assumptions and Invariants

- `interlocking_beam` defaults to `false`, so **every existing print is bitwise unchanged** — the lattice is empty, the module emits zero `polygon_updates` (AC-15), and no default behaviour is locked.
- `PrePass::InterlockingLattice` is host-only and absent from `VALID_STAGES`: no module manifest can target it. Reversing that is a schema change.
- `Layer::SlicePostProcess` gains its **second** planned occupant, and this packet establishes narrow writes as the price of sharing it. Any third occupant inherits that constraint; ticket 148 owns the standing rule.
- The rename is one-way: `mmu_segmented_region_interlocking_beam` is retired with no alias, so a config or 3MF using the old spelling silently loses the setting — matching ticket 07's ruling, ticket 100's precedent, and how this port already treats unimplemented Orca keys.

## Risks and Tradeoffs

- **The new IR is the real cost of this design, and it is not small.** Schema constant, WIT record, host and guest binding, `LayerStageInput` field, guest rebuild. A host-only design would need none of it. The packet pays it to keep the swappable half of the algorithm in a module and to give `Layer::SlicePostProcess` a justified occupant. If Step 5 turns out to be larger than one step, split it rather than widening the step's edit list.
- **AC-N6 is a genuine risk, not a formality.** It rests on dotted write paths avoiding the reverse `IrWriteRead` edge. `seam-placer` shipping beside `fuzzy-skin` is strong evidence, but the dispatch in §Expected Sub-Agent Dispatches must confirm the mechanism in `dag.rs` before Step 7 is written. If it does not hold, this packet is blocked on ticket 148 and must say so rather than widen its writes.
- **`handle_thin_areas` is the hardest function to assert.** Its output depends on two morphological opens, a four-way intersection and an iteration count derived from `detect / min_line`. AC-14 asserts the observable consequence rather than intermediate sets — weaker than a golden, but the strongest invariant available without a runnable canonical.
- **Performance is unmeasured.** The analysis voxelises every layer of every differently-toned region pair; pair count is quadratic in distinct tools. Canonical has the same shape and ships it, and the path is gated off by default. This packet does not optimise and does not claim a cost. If `--instrument-stderr` later shows the prepass dominating, that is a follow-up.
- **A partial rename is silent in one place.** `resolved_config.rs`'s hand-written `PartialEq` compares by field name; renaming the `cli` string literal without the field (or vice versa) compiles and silently stops the key resolving. Step 6's exit greps for the old spelling rather than relying on the compiler.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 5 — the new IR across the WIT boundary)
- Highest-risk dispatch and required return format: the `dag.rs` dotted-path question above, `SNIPPETS` ≤30 lines. It gates AC-N6, and AC-N6 gates whether this packet can share a stage with 303.

## Open Questions

- `[FWD]` Canonical's `handleThinAreas` sets both regions' slices with surface type `stInternal`. This tree's `SlicedRegion` has no surface-type field on `polygons`; the equivalent classification lives in `top_shell_index` / `bottom_shell_index`, set by `PrePass::ShellClassification`, which runs before `Layer::SlicePostProcess`. Decide whether the interlocked regions need reclassification. Measure first: if the shell indices still describe the footprint after the rewrite, record the divergence in the ADR and proceed; if they are visibly wrong on the beam boundary, raise it rather than re-running the shell pass inside this packet.
- `[FWD]` Canonical's `throw_on_cancel` callback threads through every loop. This tree's prepass built-ins take no cancellation token. Port the structure without the callback and leave the loop shapes intact so a token can be threaded later; do not invent a cancellation mechanism here.
