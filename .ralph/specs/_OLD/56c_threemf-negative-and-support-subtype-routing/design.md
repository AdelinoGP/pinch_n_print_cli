# Design: 56c_threemf-negative-and-support-subtype-routing

## Controlling Code Paths

### Design Correction Note

The original design (prepass.rs phase-0 insertion with `&mut [SliceIR]`) was architecturally infeasible — `Vec<SliceIR>` is produced per-layer during execution, after prepass returns. Corrected insertion site is `layer_executor.rs::run_paint_annotation`; signature is `(&mut SliceIR, &[ModifierVolume])` (singular). Paint segmentation does not consume SliceIR polygons (it reads triangle paint attributes), so the original "paint segmentation sees post-subtract polygons" invariant is geometrically moot; the actual invariant is "paint annotation and subsequent per-layer consumers see post-subtract polygons".

### State after Packets 56, 56b, and 64 (precondition for this packet)

- `crates/slicer-host/src/model_loader.rs::resolve_object` routes ALL non-`NormalPart` parts into `ObjectMesh.modifier_volumes` with typed `config_delta.fields[ConfigKey::from("subtype")] == ConfigValue::String(...)`. `MeshIR.schema_version == 1.1.0`. Paint dropped on non-`NormalPart` rows.
- `crates/slicer-host/src/region_mapping.rs::execute_region_mapping` is **unchanged** from Packet 56b's cleanup — no modifier-volume parameter and no stamping. Modifier-part fuzzy skin moved to the paint annotation pipeline (`slice_postprocess.rs`) in 56b.
- `crates/slicer-host/src/pipeline.rs` was **not modified** by Packet 56b. The pipeline calls `execute_prepass_with_builtins_configured` (`pipeline.rs:166` / `pipeline.rs:234`), which internally orchestrates phase-0 built-ins (region-mapping commit), phase-1 user prepass stages (including `PrePass::PaintSegmentation` at `prepass.rs:418`/`:503`), and phase-2 RegionMapping. `Vec<SliceIR>` is NOT produced inside prepass — it is produced per-layer during parallel layer execution in `crates/slicer-host/src/layer_executor.rs::execute_layer_slice` (after prepass returns). Modifier-volume consumption happens through existing pipeline hooks: `layer_executor.rs::run_paint_annotation` for fuzzy-skin projections. This packet inserts `apply_negative_part_subtract` as a per-layer call inside `layer_executor.rs::run_paint_annotation`, BEFORE paint annotation begins, operating on each layer's `SliceIR` as it is dequeued from the layer arena.
- `crates/slicer-host/src/paint_segmentation.rs` is now a **host-native fallback** for `PrePass::PaintSegmentation` (Packet 64 migration). `execute_paint_segmentation` takes 4 parameters: `(Arc<MeshIR>, Arc<SurfaceClassificationIR>, Arc<LayerPlanIR>, union_paint_regions_at_harvest: bool)`. It emits `PaintRegionIR` from `paint_data` (triangle attributes) but has no awareness of `modifier_volumes` at packet-author time.
- No host stage performs negative-part subtraction on the per-layer `SliceIR`.

### After this packet

- `crates/slicer-host/src/negative_part_subtract.rs` is a NEW file containing:
  ```
  pub fn apply_negative_part_subtract(
      slice_ir: &mut SliceIR,
      modifier_volumes: &[ModifierVolume],
  );
  ```
  `SliceIR` is **per-layer** (`SliceIR { z: f32, global_layer_index: u32, regions: Vec<SlicedRegion>, … }` — verified at `crates/slicer-ir/src/slice_ir.rs:1102`). Layer polygons live at `slice_ir.regions[i].polygons: Vec<ExPolygon>` (`slice_ir.rs:1074`). For each layer, the call resolves `slice_ir.z` against each modifier's Z extent; modifiers outside the extent are skipped; otherwise project the modifier mesh at `slice_ir.z` via `slice_mesh_ex` and apply `polygon_ops::difference` per region. The function takes a singular `&mut SliceIR` because `layer_executor.rs` invokes it per-layer as each `SliceIR` is dequeued from the layer arena. For each `ModifierVolume` whose `config_delta.fields[ConfigKey::from("subtype")] == ConfigValue::String("negative_part")`, project the mesh at `slice_ir.z` using `slicer_core::slice_mesh_ex(&mv.mesh, &[slice_ir.z])`, take the resulting `Vec<ExPolygon>` for that single Z, and replace every `slice_ir.regions[ri].polygons` with `slicer_core::polygon_ops::difference(&slice_ir.regions[ri].polygons, &projection)`. If the projection is empty (layer outside the modifier's Z extent), the region is untouched.
- `crates/slicer-host/src/layer_executor.rs` (NOT `pipeline.rs`, NOT `prepass.rs`) is the insertion site. `apply_negative_part_subtract(&mut slice_ir, &modifier_volumes_for_object)` is called inside `run_paint_annotation` after `arena.take_slice()` returns the layer's `SliceIR` and BEFORE the paint annotation loop begins. This guarantees paint annotation and all subsequent per-layer consumers see post-subtract polygons. The exact insertion line is confirmed at Step 2 via FACT dispatch.
- `crates/slicer-host/src/paint_segmentation.rs` augments `execute_paint_segmentation` at `paint_segmentation.rs:253` (signature `(Arc<MeshIR>, Arc<SurfaceClassificationIR>, Arc<LayerPlanIR>, union_paint_regions_at_harvest: bool) -> Result<Arc<PaintRegionIR>, PaintSegmentationError>`). A new second loop over `mesh_ir.objects` is added at line 383, after the existing paint-data loop completes. Modifier volumes are NOT threaded as a new parameter — they are read directly from `mesh_ir.objects[].modifier_volumes` (Packet 56b populated). For each `support_enforcer` / `support_blocker` modifier volume, project per layer once via `slice_mesh_ex(&mv.mesh, &layer_zs)` (batched over all layer Zs, not per-layer). Emit `SemanticRegion` entries under the matching `PaintSemantic` variant, merging into any existing `Vec<SemanticRegion>` via `polygon_ops::union`. This second loop runs regardless of the `union_paint_regions_at_harvest` flag (which controls only the paint-data path's per-semantic union step).
- The synthetic `PaintRegionIR` entries flow through Packet 51's existing `paint_overrides` overlay path — no new region-mapping code beyond Packet 56b's `modifier_part` stamp.

## Neighboring Tests / Fixtures

- `crates/slicer-host/tests/benchy_4color_modifier_part_e2e_tdd.rs` — Packet 56b's fixture-backed E2E. Must stay green.
- `crates/slicer-host/tests/threemf_sidecar_classification_tdd.rs` — Packet 56's parser suite. Must stay green.
- `crates/slicer-host/tests/threemf_transform_tdd.rs` — 3MF transform regression baseline (Packet 56).
- `crates/slicer-host/tests/benchy_painted_e2e_tdd.rs`, `benchy_painted_overrides_e2e_tdd.rs` — paint pipeline regression baseline.
- `crates/slicer-host/tests/threemf_subtypes_synthetic_e2e_tdd.rs` — NEW. IR-level tests exercising `apply_negative_part_subtract` and `execute_paint_segmentation` directly with in-memory IR structs.

## Architecture Constraints

- Scaled integer units: `slicer_core::polygon_ops` operates on `ExPolygon` in scaled integer units. All modifier projections produced by `slice_mesh_ex` are already in scaled integer units and require no conversion before `polygon_ops::difference` / `::union` calls.
- Per-layer projection: use `slicer_core::slice_mesh_ex(&mv.mesh, &[slice_ir.z])` (verified at `crates/slicer-core/src/triangle_mesh_slicer.rs:46`; signature `(&IndexedTriangleSet, &[f32]) -> Vec<Vec<ExPolygon>>` — one `Vec<ExPolygon>` per requested Z). The per-layer call site uses a single-element Z slice because `apply_negative_part_subtract` is invoked once per layer from `layer_executor.rs::run_paint_annotation`. This is the same function Packet 56b uses for modifier-part fuzzy-skin projections in `layer_executor.rs::run_paint_annotation` (call pattern at `layer_executor.rs:559-562`).
- Layer-executor ordering: inside `run_paint_annotation`, the sequence becomes `arena.take_slice()` → `apply_negative_part_subtract(&mut slice_ir, &modifier_volumes)` → paint annotation loop → downstream per-layer consumers. The order is critical because paint annotation and all subsequent per-layer consumers must see post-subtract polygons (otherwise per-layer paint annotation operating on a region subtracted by a negative volume would emit phantom regions). Step 2 FACT dispatch confirms the exact insertion line within `layer_executor.rs::run_paint_annotation`.
- Determinism: per-layer call order is locked. `apply_negative_part_subtract` is purely functional given `&mut SliceIR + modifier_volumes`; no global state.
- WIT boundary: clean (re-confirmed at Packet 56b Step 0). This packet introduces no IR types and does not re-check.
- `support_*` synthetic `PaintRegionIR` polygons MUST be union-merged into any existing `Vec<SemanticRegion>` at the same `(layer, semantic)` key (in case paint-supports triangle painting AND a `support_enforcer` volume coexist on the same model). Union helper verified: `slicer_core::polygon_ops::union(&[ExPolygon], &[ExPolygon]) -> Vec<ExPolygon>` at `polygon_ops.rs:93`.

## Selected Approach (Locked Decisions)

| Decision | Locked choice | Justification |
|---|---|---|
| `negative_part` consumer placement | **New host stage `apply_negative_part_subtract`** invoked per-layer inside `layer_executor.rs::run_paint_annotation`, on each layer's `SliceIR` as it is dequeued from the arena, BEFORE paint annotation begins. Activation Q3 = Option 1 (original intent: "between prepass and region-mapping") now realized as "after prepass returns and before per-layer paint annotation" since `Vec<SliceIR>` is produced per-layer during execution, not during prepass. | Keeps `region_mapping.rs` focused on config overlays. Negative-part geometry is independently testable. Per-layer placement at the layer-executor seam guarantees paint annotation and all downstream per-layer consumers see post-subtract polygons. |
| `negative_part` subtract API | `apply_negative_part_subtract(&mut SliceIR, &[ModifierVolume])` (singular `SliceIR` — one layer at a time, since `layer_executor.rs` invokes it per-layer). Mutates the layer's `SliceIR.regions[ri].polygons` in place. `SliceIR` is per-layer (`SliceIR { z, global_layer_index, regions: Vec<SlicedRegion>, … }`). | Matches the actual `SliceIR` shape (`crates/slicer-ir/src/slice_ir.rs:1102`) and the per-layer invocation pattern in `layer_executor.rs::run_paint_annotation`. |
| `support_*` consumer placement | **Paint-segmentation piggyback.** Augment `paint_segmentation.rs::execute_paint_segmentation` at line 383 with a second `for object in &mesh_ir.objects` loop that reads `modifier_volumes` directly (no new parameter beyond `union_paint_regions_at_harvest` already added by Packet 64) and emits synthetic `PaintRegionIR` entries. | Reuses Packet 51's `paint_overrides` overlay. Zero new region-mapping code. `mesh_ir` already carries the volumes (Packet 56b populated); threading a separate `&[ModifierVolume]` from `pipeline.rs` would duplicate data. |
| `support_*` `PaintRegionIR` merge strategy | Union the synthetic polygon set with any existing `Vec<SemanticRegion>` at the same `(layer, semantic)` via `slicer_core::polygon_ops::union` (verified to exist at `polygon_ops.rs:93`). | Avoids dropping paint-painted support regions when a volume coexists. |
| `support_*` `PaintRegionIR` polygon source | Per-layer projection of the world-space modifier mesh. Matches `modifier_part`'s overlap projection in Packet 56b. | Consistent projection convention. |
| Layer projection function | `slicer_core::slice_mesh_ex(&mv.mesh, &[slice_ir.z])`. Returns `Vec<Vec<ExPolygon>>` — one set of ExPolygons per requested Z (here a single Z since the call site is per-layer). Same function used by Packet 56b in `layer_executor.rs::run_paint_annotation`. | Consistent projection across all three subtype consumers. |
| Pipeline insertion point | Inside `crates/slicer-host/src/layer_executor.rs::run_paint_annotation`, called per-layer on each `SliceIR` dequeued from the arena, BEFORE paint annotation begins. Step 2 FACT dispatch returns the exact insertion line. `pipeline.rs` and `prepass.rs` are NOT modified — `Vec<SliceIR>` is produced per-layer in `layer_executor.rs::execute_layer_slice` (after prepass returns), so the subtract must land at the per-layer seam to precede paint annotation. | Activation Q3 = Option 1 locked the semantic ordering ("between prepass and region-mapping" was the original intent), now replaced with "before per-layer paint annotation, on each layer's `SliceIR` as it is dequeued from the arena, after prepass has completed" — required by the real pipeline topology where `SliceIR` does not exist at prepass time. |
| Synthetic-fixture builder | In-memory IR structs: `box_mesh()` builds `IndexedTriangleSet`, `modifier_volume_with_subtype()` builds `ModifierVolume`, `mesh_ir_with_modifier()` assembles `Arc<MeshIR>`. Tests exercise `apply_negative_part_subtract` and `execute_paint_segmentation` directly — no 3MF archive parsing. | No on-disk fixture; tests are hermetic and fast. |
| Negative-test handling for degenerate volumes | Zero-triangle `negative_part` → no-op subtract (Clipper2 returns input unchanged for empty subtrahend). Zero-triangle `support_*` → emit no `PaintRegionIR` entries; no warning. | Degenerate is not an error. |
| Workspace test discipline | `cargo test --workspace` runs exactly ONCE at acceptance ceremony (Step 7), dispatched via FACT. Iterative steps use targeted commands. | This is the terminal packet of the three-way split; a workspace-wide gate at closure is the correct cadence. |

## Rejected Alternatives

| Alternative | Reason rejected |
|---|---|
| Inline `negative_part` subtract inside `execute_region_mapping` | Mixes geometry mutation with config overlay. Activation Q3 = Option 1 explicitly picked a separate stage. |
| Inline `negative_part` subtract inside `paint_segmentation` | Paint segmentation must see post-subtract polygons — running subtract here would force a two-pass paint segmentation. |
| Subtract once into a new `SliceIR2` rather than mutating in place | Doubles memory. Mutating in place is the existing pattern for prepass stages. |
| Emit `support_*` polygons as a NEW IR field (`ModifierVolume.support_polygons_per_layer`) | YAGNI. `PaintRegionIR` already carries the right shape; reusing it is zero IR cost. |
| Skip `support_*` synthetic emission and require user to paint supports manually | Negates Bambu sidecar compatibility — the whole point is that a `support_enforcer` part in the sidecar should behave like a painted enforcer triangle volume. |
| `negative_part` subtract as a BUILTIN module rather than a host stage | Builtin modules live in `modules/core-modules/`. Negative subtract is a pipeline concern, not a per-region computation; host stage is the right abstraction. |
| Re-check WIT mirror gate at this packet's Step 0 | No IR types introduced. Packet 56b's Step 0 confirms; no fresh check needed. |

## Code Change Surface (≤ 3 primary files per step)

Primary files this packet edits:

1. `crates/slicer-host/src/negative_part_subtract.rs` — NEW. 63 lines for the stage. Signature: `pub fn apply_negative_part_subtract(slice_ir: &mut SliceIR, modifier_volumes: &[ModifierVolume])` (line 20).
2. `crates/slicer-host/src/layer_executor.rs` — ~3 added lines inside `run_paint_annotation` (insert per-layer call after `arena.take_slice()` at line 628 and before the paint annotation loop; pass `&obj.modifier_volumes`). Actual call at line 635.
3. `crates/slicer-host/src/paint_segmentation.rs` — ~45 added lines at lines 383-427 for the synthetic `PaintRegionIR` emission (reads `mesh_ir.objects[].modifier_volumes` in a second loop after the paint-data path completes; no new parameter on `execute_paint_segmentation`).
4. `crates/slicer-host/tests/threemf_subtypes_synthetic_e2e_tdd.rs` — NEW. 682 lines. IR-level test builders (`box_mesh`, `modifier_volume_with_subtype`, `mesh_ir_with_modifier`, etc.) + 10 test functions exercising `apply_negative_part_subtract` and `execute_paint_segmentation` directly.
5. `docs/07_implementation_status.md` — append TASK-192b, TASK-192c, TASK-193 rows (lines 145-147).
6. `crates/slicer-host/src/lib.rs` — declare `pub mod negative_part_subtract` (line 25).

Each step picks at most three of these.

## Read-only Context the Implementer Needs

| Path | Lines | Purpose |
|---|---|---|---|
| `crates/slicer-ir/src/slice_ir.rs` | narrow reads around `SliceIR` (≈ line 1100), `SlicedRegion` (≈ line 1068), `PaintRegionIR` (≈ line 945), `LayerPaintMap` (≈ line 936), `SemanticRegion` (≈ line 923), `PaintSemantic::SupportEnforcer/Blocker` (≈ lines 180/182), `ModifierVolume` (≈ line 252), `ConfigDelta` (≈ line 231) | Existing shapes (informational). |
| `crates/slicer-host/src/paint_segmentation.rs` | full (623 lines, just over the 600-line ceiling); entry point `execute_paint_segmentation` at line 253 | Host-native paint segmentation entry point and `PaintRegionIR` assembly. Augmented at line 383 with modifier_volumes second loop. |
| `crates/slicer-host/src/layer_executor.rs` | edit at `run_paint_annotation` (line 613) — per-layer `apply_negative_part_subtract` call at line 635, after `arena.take_slice()` at line 628 and before the paint annotation loop. | Per-layer insertion site for the negative-part subtract stage. |
| `crates/slicer-host/tests/threemf_subtypes_synthetic_e2e_tdd.rs` | full (682 lines) | IR-level test builders (`box_mesh`, `modifier_volume_with_subtype`, `mesh_ir_with_modifier`, `layer_plan_with_z_values`) and 10 test functions. |
| `docs/04_host_scheduler.md` | prepass / region-mapping ordering section | Delegate SUMMARY. |
| `docs/02_ir_schemas.md` | `PaintRegionIR`, `PaintSemantic` block search | Read narrow section. |
| `docs/08_coordinate_system.md` | full (small) | Scaled integer units. |

## Out-of-Bounds Files (must not be loaded directly)

- `crates/slicer-macros/src/lib.rs` (>2300 lines).
- `crates/slicer-sdk/` — all files.
- `crates/slicer-ir/` — read only narrow sections; do not load full files.
- All `wit/**`, `crates/slicer-host/src/wit_host.rs`, `dispatch.rs` — clean by Packets 56 / 56b; do not re-check.
- `OrcaSlicerDocumented/**` — always delegate via Explore agent with LOCATIONS. Two dispatches total in this packet (Steps 2 and 3).
- **`crates/slicer-host/src/model_loader.rs`** — owned by Packet 56 (parser) and Packet 56b (`resolve_object`). Immutable in this packet per Cross-Packet Mutation Rule.
- **`crates/slicer-host/src/region_mapping.rs`** — owned by Packet 56b. Immutable here.
- **`crates/slicer-host/src/pipeline.rs`** — NOT edited by this packet. The per-layer insertion lands inside `layer_executor.rs::run_paint_annotation`, not `pipeline.rs`. Narrow read for cross-reference only.
- **`crates/slicer-host/src/prepass.rs`** — NOT edited by this packet. `Vec<SliceIR>` is produced per-layer in `layer_executor.rs::execute_layer_slice`, after prepass returns; the subtract therefore lands in the layer executor, not prepass. Narrow read for cross-reference only.
- **`modules/core-modules/fuzzy-skin/manifest.toml`** — gated by Packet 56b. Immutable here.
- `target/`, `Cargo.lock`, generated code.

Note: `crates/slicer-host/src/layer_executor.rs` is **in scope** for this packet (per-layer insertion inside `run_paint_annotation`). It is not owned by Packets 56 / 56b — both packets confirmed no edits to `layer_executor.rs::run_paint_annotation` beyond Packet 56b's modifier-part fuzzy-skin call (which this packet's edit sits alongside). Cross-Packet Mutation Rule satisfied.

## Data and Contract Notes

- `SliceIR` is **per-layer** (`crates/slicer-ir/src/slice_ir.rs:1102`): `pub struct SliceIR { schema_version: SemVer, global_layer_index: u32, z: f32, regions: Vec<SlicedRegion> }`. One `SliceIR` instance represents one layer. The host pipeline produces a `Vec<SliceIR>` (one per layer); `apply_negative_part_subtract` takes `&mut [SliceIR]` and iterates layer by layer.
- `SlicedRegion` layer polygon storage (`slice_ir.rs:1068-1074`): `pub struct SlicedRegion { object_id, region_id, polygons: Vec<ExPolygon>, infill_areas: Vec<ExPolygon>, … }`. Per-layer 2D polygons live at `slice_irs[li].regions[ri].polygons`.
- `PaintRegionIR` shape (`slice_ir.rs:945-950`): `pub struct PaintRegionIR { schema_version, per_layer: HashMap<u32, LayerPaintMap> }`. Access is `paint_region_ir.per_layer.get(&global_layer_index)`, NOT array indexing.
- `LayerPaintMap` shape (`slice_ir.rs:936-941`): `pub struct LayerPaintMap { global_layer_index, semantic_regions: HashMap<PaintSemantic, Vec<SemanticRegion>> }`.
- `SemanticRegion` shape (`slice_ir.rs:923-932`): `pub struct SemanticRegion { object_id, polygons: Vec<ExPolygon>, value: PaintValue, paint_order: u64, aabb: Option<Aabb> }`. A `HashMap<PaintSemantic, Vec<SemanticRegion>>` lookup returns `Option<&Vec<SemanticRegion>>`; per-layer area is the sum of `region.polygons` area across all returned `SemanticRegion`s.
- `PaintSemantic::SupportEnforcer` (`slice_ir.rs:180`) and `PaintSemantic::SupportBlocker` (`slice_ir.rs:182`) exist as enum variants (Packet 50b precedent, confirmed).
- `slicer_core::polygon_ops::difference` — Clipper2-backed. Verified signature (`polygon_ops.rs:103`): `pub fn difference(subject: &[ExPolygon], clip: &[ExPolygon]) -> Vec<ExPolygon>`.
- `slicer_core::polygon_ops::union` — verified at `polygon_ops.rs:93`: `pub fn union(subject: &[ExPolygon], clip: &[ExPolygon]) -> Vec<ExPolygon>`.
- `slicer_core::slice_mesh_ex` — verified at `triangle_mesh_slicer.rs:46`: `pub fn slice_mesh_ex(mesh: &IndexedTriangleSet, zs: &[f32]) -> Vec<Vec<ExPolygon>>`. In `paint_segmentation.rs:383-427`, it is called once per volume with `&layer_zs` (all layer Zs batched), not per-layer.
- `ModifierVolume.mesh: IndexedTriangleSet` (`slice_ir.rs:252-265`) is in world space (Packet 56b's invariant). Per-layer projection slices at layer Z directly.
- `ModifierVolume.config_delta: ConfigDelta` (`slice_ir.rs:231-235`) has `fields: HashMap<ConfigKey, ConfigValue>` with `ConfigKey = String` and `ConfigValue::String(String)`. The access `mv.config_delta.fields.get("subtype")` (used at `paint_segmentation.rs:384`) returns `Option<&ConfigValue>`.

## Risks and Tradeoffs

| Risk | Mitigation |
|---|---|---|
| Negative-part subtract changes per-layer polygons. Downstream per-layer consumers (paint annotation and beyond) must see the post-subtract polygons. | Stage inserted as a per-layer call inside `layer_executor.rs::run_paint_annotation` at line 635, AFTER `arena.take_slice()` at line 628 and BEFORE the paint annotation loop. AC-5 asserts downstream consumers see post-subtract polygons. |
| Per-layer insertion inside `layer_executor.rs::run_paint_annotation` sits alongside Packet 56b's modifier-part fuzzy-skin call. Risk of unintended interaction. | Insertion at line 635 is additive — placed before the paint annotation loop and before Packet 56b's fuzzy-skin call so both see post-subtract polygons. |
| Support enforcer / blocker piggyback re-uses Packet 50b / 51's paint-supports semantic. If those test surfaces drift, this packet's tests may regress. | Step 4 regression sweep explicitly re-runs Packet 50b / 51 regression tests. |
| Support enforcer/blocker loop runs in `execute_paint_segmentation` which is now a host-native fallback for `PrePass::PaintSegmentation` (Packet 64). If a WASM module registers for `PrePass::PaintSegmentation`, the host-native piggyback is bypassed and support volumes emit nothing. | Packet 64 deleted the WASM module; the host-native path is the sole path. If a future WASM module registers for this stage, the support-volume emission must be re-verified. |
| `apply_negative_part_subtract` mutates the `&mut SliceIR` in place — accidental double-application if `run_paint_annotation` runs the stage twice per layer. | Stage is purely functional; `run_paint_annotation` dequeues each `SliceIR` from the arena exactly once per layer, so the call fires exactly once per layer by construction. Call is at exactly one site (line 635). |
| Performance: per-layer × per-volume × per-triangle projection is O(L × V × T). The per-layer call site makes one `slice_mesh_ex` call per volume per layer. For synthetic 5×5×5 mm cube volumes (12 triangles), trivial. | Out of scope. TODO logged in `negative_part_subtract.rs`. |
| The synthetic-fixture builder uses IR struct builders instead of 3MF archive parsing. This tests consumer logic directly but does not exercise the full 3MF parse → IR → consumers pipeline. | 3MF-to-IR parsing is tested by Packet 56's `threemf_sidecar_classification_tdd.rs` and Packet 56b's `benchy_4color_modifier_part_e2e_tdd.rs`. This packet's IR-level tests complement those with focused consumer-logic coverage. |
| `PaintRegionIR.per_layer` is `HashMap<u32, LayerPaintMap>` — code MUST use `.get(&n)` / `.get_mut(&n)` / `.entry(n).or_insert_with(...)`. | AC prose uses explicit `HashMap` access; tests use the real API. In `paint_segmentation.rs:383-427`, entries are created via `.entry(layer.index).or_insert_with(...)`. |

## Open Questions Blocking Activation

- **Q1 (Packet status).** Packets 56 AND 56b must be `status: implemented` before this packet activates. Verify by grep on each packet's `packet.spec.md` at Step 0.

## Locked Assumptions and Invariants

1. WIT scope is clean (confirmed by Packets 56 / 56b; not re-checked here).
2. `ObjectMesh.modifier_volumes` is populated for ALL non-`NormalPart` subtypes by Packet 56b. This packet consumes that plumbing.
3. Inside `layer_executor.rs::run_paint_annotation` (line 613), the per-layer ordering is: `arena.take_slice()` (line 628) → `apply_negative_part_subtract(&mut slice_ir, &modifier_volumes)` → paint annotation loop → downstream per-layer consumers. Order is critical: paint annotation and all subsequent per-layer consumers must see post-subtract polygons. Paint segmentation runs earlier inside prepass and operates on triangle paint attributes (not `SliceIR` polygons), so it is unaffected by this stage.
4. `negative_part` subtract is a per-layer call invoked from `layer_executor.rs::run_paint_annotation`, NOT a `prepass.rs` phase-0 built-in (the original design was infeasible because `Vec<SliceIR>` does not exist at prepass time), NOT a `pipeline.rs` insertion, and NOT a region-mapping inline operation.
5. `support_enforcer` / `support_blocker` emit synthetic `PaintRegionIR` entries; they do NOT introduce new region-mapping code. `paint_segmentation.rs` reads modifier volumes directly from `mesh_ir.objects[].modifier_volumes` at line 383. No new parameter was added to `execute_paint_segmentation` beyond `union_paint_regions_at_harvest: bool` (introduced by Packet 64). The second modifier-volume loop runs unconditionally (not gated by `union_paint_regions_at_harvest`).
6. `apply_negative_part_subtract` operates on `&mut SliceIR` (singular — one layer at a time) and mutates `slice_ir.regions[ri].polygons` in place. `layer_executor.rs::run_paint_annotation` invokes it per-layer for each `SliceIR` dequeued from the arena.
7. `PaintRegionIR.per_layer` is `HashMap<u32, LayerPaintMap>`; access via `.get(&n)` / `.get_mut(&n)`. `LayerPaintMap.semantic_regions` is `HashMap<PaintSemantic, Vec<SemanticRegion>>`; per-layer area for a given semantic is the sum across all returned `SemanticRegion`s' `polygons`.
8. Existing tests for Packets 50 / 50b / 51 / 56 / 56b stay GREEN.
9. No new IR types introduced.
10. No new deviations registered.
11. `cargo test --workspace` runs exactly once at Step 7 acceptance ceremony.
12. The terminal packet of the three-way split closes the original `56_threemf-modifier-and-subtype-sidecar-ingestion` slice in full. No follow-up packet is needed in this scope; consumers like `extruder` per-modifier override remain future work.
