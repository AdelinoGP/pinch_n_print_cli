# Design: 56c_threemf-negative-and-support-subtype-routing

## Controlling Code Paths

### State after Packets 56 and 56b (precondition for this packet)

- `crates/slicer-host/src/model_loader.rs::resolve_object` routes ALL non-`NormalPart` parts into `ObjectMesh.modifier_volumes` with typed `config_delta.fields[ConfigKey::from("subtype")] == ConfigValue::String(...)`. `MeshIR.schema_version == 1.1.0`. Paint dropped on non-`NormalPart` rows.
- `crates/slicer-host/src/region_mapping.rs::execute_region_mapping` accepts per-object `&[ModifierVolume]` and stamps `fuzzy_skin.apply-to-all` for `modifier_part` overlaps only.
- `crates/slicer-host/src/pipeline.rs` threads per-object `modifier_volumes` into the region-mapping call.
- `crates/slicer-host/src/paint_segmentation.rs` is unchanged from Packet 50 / 50b / 51's state — it emits `PaintRegionIR` from `paint_data` (triangle attributes) but has no awareness of `modifier_volumes`.
- No host stage between prepass and region-mapping performs negative-part subtraction.

### After this packet

- `crates/slicer-host/src/negative_part_subtract.rs` is a NEW file containing:
  ```
  pub fn apply_negative_part_subtract(
      slice_ir: &mut SliceIR,
      modifier_volumes: &[ModifierVolume],
  );
  ```
  For each `ModifierVolume` whose `config_delta.fields[ConfigKey::from("subtype")] == ConfigValue::String("negative_part")`, project the mesh per layer (reuse the slicer entry-point identified by Step 2's FACT dispatch). For each layer Z in the volume's extent, compute `slicer_core::polygon_ops::difference(slice_ir.layers[i].polygons, projected_negative_at_z)` and replace `slice_ir.layers[i].polygons` with the result. Outside the extent, layers are untouched.
- `crates/slicer-host/src/pipeline.rs` calls `apply_negative_part_subtract(&mut slice_ir, &modifier_volumes_for_object)` immediately after `execute_prepass_*` returns the populated `SliceIR` and BEFORE either `paint_segmentation` or `execute_region_mapping`. Step 2 FACT dispatch identifies the exact insertion line.
- `crates/slicer-host/src/paint_segmentation.rs` augments its existing `harvest_paint_segmentation_ir` entry (or equivalent — Step 3 FACT dispatch returns the entry-point name) to accept `&[ModifierVolume]`. For each `support_enforcer` / `support_blocker` modifier volume, project per layer; emit a `PaintRegionIR.per_layer[N].semantic_regions` entry under `PaintSemantic::SupportEnforcer` or `PaintSemantic::SupportBlocker` whose polygon set is the per-layer projection. The new entries are merged into the existing `PaintRegionIR` produced by the paint-data path; if a layer already has an entry for `SupportEnforcer` (from paint-supports semantic painting on triangles), the synthetic-volume polygons are unioned in via `slicer_core::polygon_ops::union` (Step 3 verifies the union helper exists; if not, switch to insertion + downstream-overlay tolerance).
- The synthetic `PaintRegionIR` entries flow through Packet 51's existing `paint_overrides` overlay path — no new region-mapping code beyond Packet 56b's `modifier_part` stamp.

## Neighboring Tests / Fixtures

- `crates/slicer-host/tests/benchy_4color_modifier_part_e2e_tdd.rs` — Packet 56b's fixture-backed E2E. Must stay green.
- `crates/slicer-host/tests/threemf_sidecar_classification_tdd.rs` — Packet 56's parser suite. Must stay green.
- `crates/slicer-host/tests/threemf_transform_tdd.rs` — uses in-memory `zip::write::ZipWriter` for synthetic 3MF; this packet's test reuses the same pattern.
- `crates/slicer-host/tests/benchy_painted_e2e_tdd.rs`, `benchy_painted_overrides_e2e_tdd.rs` — paint pipeline regression baseline.
- `crates/slicer-host/tests/threemf_subtypes_synthetic_e2e_tdd.rs` — NEW. Synthetic 3MF archives with negative + support sidecars.

## Architecture Constraints

- Scaled integer units: `slicer_core::polygon_ops` operates on `Point2` in scaled integer units. All modifier projections convert to `Point2::from_mm(x, y)` before any `polygon_ops` call.
- Per-layer projection: reuse the existing slicer entry-point that already projects paint-data strokes to layer Z (same function Packet 56b identifies at its Step 5; for this packet's Step 2 FACT, re-confirm the function name or re-use Packet 56b's discovery cached in `region_mapping.rs`).
- Pipeline ordering: `execute_prepass_*` → `apply_negative_part_subtract` → `paint_segmentation` → `execute_region_mapping`. The order is critical because paint segmentation must see post-subtract polygons (otherwise paint on a region subtracted by a negative volume would emit phantom paint regions). Step 2 FACT dispatch confirms the prepass return point.
- Determinism: pipeline order is locked. `apply_negative_part_subtract` is purely functional given `SliceIR + modifier_volumes`; no global state.
- WIT boundary: clean (re-confirmed at Packet 56b Step 0). This packet introduces no IR types and does not re-check.
- `support_*` synthetic `PaintRegionIR` polygons MUST be union-merged into any existing `PaintRegionIR` at the same layer/semantic (in case paint-supports triangle painting AND a `support_enforcer` volume coexist on the same model). Step 3 FACT dispatch returns the union helper.

## Selected Approach (Locked Decisions)

| Decision | Locked choice | Justification |
|---|---|---|
| `negative_part` consumer placement | **New host stage `apply_negative_part_subtract`** between prepass and region-mapping. Activation Q3 = Option 1 (locked at original-packet-author time). | Keeps `region_mapping.rs` focused on config overlays. Negative-part geometry is independently testable. |
| `negative_part` subtract API | `apply_negative_part_subtract(&mut SliceIR, &[ModifierVolume])`. Mutates `SliceIR` in place. | Simple. Matches the existing pipeline pattern of stages that mutate `SliceIR` between stages. |
| `support_*` consumer placement | **Paint-segmentation piggyback.** Augment `paint_segmentation.rs` to consume `&[ModifierVolume]` and emit synthetic `PaintRegionIR` entries. | Reuses Packet 51's `paint_overrides` overlay. Zero new region-mapping code. |
| `support_*` `PaintRegionIR` merge strategy | Union the synthetic polygon set with any existing entry at the same `(layer, semantic)`. If `slicer_core::polygon_ops::union` does not exist, fall back to insertion + downstream-overlay tolerance. | Avoids dropping paint-painted support regions when a volume coexists. |
| `support_*` `PaintRegionIR` polygon source | Per-layer projection of the world-space modifier mesh. Matches `modifier_part`'s overlap projection in Packet 56b. | Consistent projection convention. |
| Layer projection function | Reuse the slicer entry-point identified by Packet 56b Step 5's FACT dispatch. Step 2 of this packet re-runs the same dispatch (or reads the function name from `region_mapping.rs`). | Single source of projection truth. |
| Pipeline insertion point | Between `execute_prepass_*`'s return and the first call to `paint_segmentation` / `execute_region_mapping`. Step 2 FACT dispatch returns the exact line. | Activation Q3 = Option 1. |
| Synthetic-fixture builder | In-memory `zip::write::ZipWriter` with hand-built `3D/3dmodel.model` + `Metadata/model_settings.config` strings. Reuse the pattern from `threemf_transform_tdd.rs`. | No on-disk fixture; tests are hermetic. |
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

1. `crates/slicer-host/src/negative_part_subtract.rs` — NEW. ~120 lines for the stage + helper.
2. `crates/slicer-host/src/pipeline.rs` — ~5 added lines (one new call).
3. `crates/slicer-host/src/paint_segmentation.rs` — ~80 added lines for the synthetic `PaintRegionIR` emission helper.
4. `crates/slicer-host/tests/threemf_subtypes_synthetic_e2e_tdd.rs` — NEW. Synthetic-fixture builder + ~8 test functions.
5. `docs/07_implementation_status.md` — append TASK-192b, TASK-192c, TASK-193 rows.

Each step picks at most three of these.

## Read-only Context the Implementer Needs

| Path | Lines | Purpose |
|---|---|---|
| `crates/slicer-ir/src/slice_ir.rs` | search for `SliceIR`, `PaintRegionIR`, `PaintSemantic` | Existing shapes (informational). |
| `crates/slicer-host/src/pipeline.rs` | search for `execute_prepass_` and `execute_region_mapping` calls | Insertion point. |
| `crates/slicer-host/src/paint_segmentation.rs` | full (delegate FACT for length first) | Entry-point function and existing `PaintRegionIR` assembly. |
| `crates/slicer-host/src/region_mapping.rs` | search for the layer-projection function name | Reused for `negative_part` projection. |
| `crates/slicer-host/tests/threemf_transform_tdd.rs` | search for `ZipWriter::new` | Synthetic 3MF builder pattern. |
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
- **`modules/core-modules/fuzzy-skin/manifest.toml`** — gated by Packet 56b. Immutable here.
- `target/`, `Cargo.lock`, generated code.

## Data and Contract Notes

- `SliceIR` layer polygon storage: Step 2 FACT dispatch returns the exact field path (likely `slice_ir.layers[i].polygons: Vec<Polygon>` or `slice_ir.layers[i].outlines: Vec<Polygon>` — verify before editing).
- `PaintRegionIR.per_layer[N].semantic_regions`: a `BTreeMap<PaintSemantic, RegionSet>` or similar. Step 3 FACT dispatch returns the exact shape.
- `PaintSemantic` enum variants `SupportEnforcer` and `SupportBlocker` exist (Packet 50b precedent). Step 3 confirms.
- `slicer_core::polygon_ops::difference` — Clipper2-backed. Signature: `pub fn difference(subject: &[Polygon], clip: &[Polygon]) -> Vec<Polygon>` (or similar; Step 2 confirms).
- `slicer_core::polygon_ops::union` — confirm existence at Step 3. If absent, fall back to insertion + downstream-overlay tolerance.
- `ModifierVolume.mesh` is in world space (Packet 56b's invariant). Per-layer projection slices at layer Z directly.

## Risks and Tradeoffs

| Risk | Mitigation |
|---|---|
| Negative-part subtract changes per-layer polygons. Downstream stages (paint segmentation, region mapping) must see the post-subtract polygons. | Insert the stage BEFORE both paint segmentation and region mapping. Step 4 AC asserts paint-on-negative-part fixture sees the reduced polygon area. |
| Support enforcer / blocker piggyback re-uses Packet 50b / 51's paint-supports semantic. If those packets' test surfaces drift, this packet's tests may regress. | Step 5 regression sweep explicitly re-runs Packet 50b / 51 regression tests. |
| Sidecar `<part id>` and synthetic 3MF `<object id>` collisions in test fixtures. | Synthetic fixtures use explicit, well-spaced IDs (1, 2, 3) per Packet 56's AC-5 convention. |
| `slicer_core::polygon_ops::union` might not exist. | Step 3 FACT dispatch confirms; fall back to insertion if missing. |
| `apply_negative_part_subtract` mutates `SliceIR` — accidental double-application if pipeline runs the stage twice. | Stage is purely functional; idempotent if same modifier_volumes input (running twice subtracts the same negative twice, which IS a bug — but the pipeline calls each stage once by construction). |
| Performance: per-layer × per-volume × per-triangle projection is O(L × V × T). For synthetic 5×5×5 mm cube volumes (12 triangles), trivial. For high-triangle modifiers, hotspot risk. | Out of scope. Log a TODO in `negative_part_subtract.rs`. |
| The synthetic-fixture builder duplicates ~50 lines from `threemf_transform_tdd.rs`. | Acceptable. Refactoring the builder to a shared helper is out of scope; would touch a Packet 56b-adjacent file. |

## Open Questions Blocking Activation

- **Q1 (Packet status).** Packets 56 AND 56b must be `status: implemented` before this packet activates. Verify by grep on each packet's `packet.spec.md` at Step 0.

## Locked Assumptions and Invariants

1. WIT scope is clean (confirmed by Packets 56 / 56b; not re-checked here).
2. `ObjectMesh.modifier_volumes` is populated for ALL non-`NormalPart` subtypes by Packet 56b. This packet consumes that plumbing.
3. The pipeline runs: `execute_prepass_*` → `apply_negative_part_subtract` → `paint_segmentation` → `execute_region_mapping`. Order is critical.
4. `negative_part` subtract is a pipeline stage, NOT a region-mapping inline operation.
5. `support_enforcer` / `support_blocker` emit synthetic `PaintRegionIR` entries; they do NOT introduce new region-mapping code.
6. Existing tests for Packets 50 / 50b / 51 / 56 / 56b stay GREEN.
7. No new IR types introduced.
8. No new deviations registered.
9. `cargo test --workspace` runs exactly once at Step 7 acceptance ceremony.
10. The terminal packet of the three-way split closes the original `56_threemf-modifier-and-subtype-sidecar-ingestion` slice in full. No follow-up packet is needed in this scope; consumers like `extruder` per-modifier override remain future work.
