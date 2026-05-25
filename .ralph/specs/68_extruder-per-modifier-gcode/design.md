# Design: 58_extruder-per-modifier-gcode

## Controlling Code Paths

### Existing infrastructure (precondition)

The config resolution pipeline already exists end-to-end:

```
CLI JSON / print profile
  → config_resolution.rs (resolve_global_config, resolve_per_paint_semantic_configs)
  → prepass.rs (build_paint_semantic_configs)
  → region_mapping.rs (execute_region_mapping_with_cap)
    → per-object config resolution
    → paint_overrides population (overlapping_semantics_for_region + overlay_resolved)
    → RegionPlan { config, stage_modules, paint_overrides }
  → [all downstream stages read RegionPlan.config]
```

For tool assignment, the paint path exists:
```
PaintRegionIR → slice_postprocess annotator → SlicedRegion.boundary_paint
  → dominant_tool_index() → RegionKey.region_id = paint_tool
  → gcode_emit.rs → required_tool = region_key.region_id as u32 → T{n}
```

For modifier volumes, `config_delta.fields` is populated:
```
model_loader.rs:562-587 → ModifierVolume { config_delta: ConfigDelta { fields: { ... } } }
```

### After this packet

Add a config-stamping step inside `execute_region_mapping_with_cap()`, between per-object config resolution and paint_overrides application:

```
per-object config resolution
  → stamp_modifier_config_deltas(mesh_ir, region_polygons, &mut base_config)
    → for each ModifierVolume in mesh_ir.modifier_volumes
      → SKIP if subtype is "support_enforcer" or "support_blocker"  ← OrcaSlicer parity, see Subtype filter row
      → compute modifier 2D bbox from mesh vertices
      → if modifier bbox overlaps region extent
        → build overlay: ResolvedConfig with extensions = config_delta.fields (minus "subtype")
        → overlay_resolved(&mut base_config, &overlay)
  → paint_overrides application (existing — can override config-delta stamped values)
  → RegionPlan { config, ... }
```

And add config-extensions-driven tool fallback in `layer_executor.rs`:
```
dominant_tool_index() → if None && region_plan.config.extensions.contains("extruder")
  → read extensions["extruder"] as ConfigValue::Int(tool) → Some(tool)
```

The GCode emitter already reads `required_tool = region_key.region_id as u32` — no changes needed.

### Key flow for extruder

Applies only to modifier_volumes whose subtype is `negative_part` or `modifier_part` (see Subtype filter row below). `support_enforcer` and `support_blocker` modifier_volumes are skipped — OrcaSlicer parity (`PrintApply.cpp:590-594`).

```
3MF sidecar → model_loader.rs → ModifierVolume.config_delta.fields["extruder"] = Int(0)
  → region_mapping.rs stamp_modifier_config_deltas (subtype-filtered)
    → RegionPlan.config.extensions["extruder"] = Int(0)
  → layer_executor.rs required_tool fallback → region_id = 0
  → gcode_emit.rs → T0
```

### OrcaSlicer parity surface

OrcaSlicer's `generate_print_object_regions()` (PrintApply.cpp:1049) does two things:
1. **Config merging**: calls `region_config_from_model_volume(parent_region.config, nullptr, modifier_volume, num_extruders)` — merges the modifier's config delta into the parent region's config
2. **Region creation**: if the merged config differs from any existing `PrintRegion`, creates a new distinct region

This packet implements item 1 (config merging via stamping into extensions). Item 2 (region creation) is deferred — existing `RegionPlan` + `config` per region handles distinct behavior without creating new region types.

The OrcaSlicer comparison surface is documented in `requirements.md` §OrcaSlicer Reference Obligations. Delegate all OrcaSlicer reads — never load `OrcaSlicerDocumented/` directly.

## Architecture Constraints

- **No new IR fields**: All modifier config keys go into `RegionPlan.config.extensions: HashMap<String, ConfigValue>`. No changes to `RegionPlan`, `SlicedRegion`, `ResolvedConfig`, `MeshIR`, or any other IR struct. The `extensions` bucket already exists on `ResolvedConfig` and carries unknown config keys.
- **No new pipeline stages**: The config stamping is a function call inside existing `execute_region_mapping_with_cap()` (PrePass::RegionMapping). No stage reordering, no new PrePass/Layer/PostPass stages.
- **No WIT changes**: The change is host-side only. Guest WASM modules read `ConfigView` which already includes the `extensions` bucket via the `paint_overrides` and `config` paths.
- **Backward compatibility**: Modifier volumes with empty `config_delta.fields` (or only the `"subtype"` key) stamp no extensions. Existing behavior unchanged for all current fixtures.
- **Single merge point**: `overlay_resolved()` (region_mapping.rs:100-193) is the sole function that merges config. Config-delta stamping calls it once per overlapping modifier volume. The implementer must not introduce a second merge mechanism.
- **Determinism**: Config delta values are parsed from 3MF sidecar metadata — static per-input. No runtime non-determinism introduced.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

## Selected Approach (Locked Decisions)

| Decision | Locked choice | Justification |
|---|---|---|
| Config key storage | `RegionPlan.config.extensions` (HashMap) | Already exists on `ResolvedConfig`. Carries unknown keys without IR schema changes. Typed fields can be promoted later. |
| Config stamping location | Inside `execute_region_mapping_with_cap()`, after per-object config resolution, before paint_overrides | Per-object config is the base. Modifier config_delta overrides it. Paint overrides can further override. The ordering is: global → per-object → modifier_delta → paint. |
| Overlap detection | 2D bounding-box overlap between modifier volume mesh and region polygon extent | Simple, correct for the global-scope `ModifierScope::AllFeatures` behavior. Polygon-level overlap can be refined later without changing the stamping mechanism. |
| Merge mechanism | `overlay_resolved(&mut base, &overlay)` — the existing function at region_mapping.rs:100 | Single merge point. Already handles `extensions` key merging at line 193. |
| Subtype exclusion | (a) Filter out `"subtype"` key from `config_delta.fields` before stamping. (b) Additionally, skip the entire modifier_volume if its subtype is `support_enforcer` or `support_blocker` — see new "Subtype filter" row. | (a) `"subtype"` is routing metadata, not a config key. (b) OrcaSlicer parity (see Subtype filter row). |
| Subtype filter | `stamp_modifier_config_deltas` SKIPS modifier_volumes whose `config_delta.fields["subtype"]` is `String("support_enforcer")` or `String("support_blocker")`. Only `negative_part` and `modifier_part` subtypes participate in stamping. | Matches `OrcaSlicerDocumented/src/libslic3r/PrintApply.cpp:590-594` (`model_volume_solid_or_modifier()` excludes ENFORCER and BLOCKER from region-config merging). Without this filter, the bridge_support_enforcers.3mf fixture (parent extruder=1, enforcer extruder=0) would emit a spurious T0 in GCode that real OrcaSlicer does not emit. Packet 67's `AC-Mod-4` and `AC-Mod-5` (test names `support_enforcer_config_delta_not_stamped` and `support_blocker_config_delta_not_stamped`) are the permanent regression guards for this filter. |
| ConfigValue defaults | Skip fields where `config_value == ConfigValue::default()` (e.g., `Int(0)`, `String("")`, `Bool(false)`) | Prevents noise in extensions. An explicit `extruder=0` is `Int(0)` which IS the default — but for extruder, value 0 is meaningful (tool index 0). The default-skip check applies only to truly empty values: empty string, empty list. Special-case: `Int(0)` for extruder IS stamped. Use a key-aware default check. |
| Extruder tool routing | `layer_executor.rs`: `dominant_tool_index()` returns paint-derived tool if present. If `None`, fall back to `region_plan.config.extensions["extruder"]` cast to `u32`. | Paint-derived tools have higher priority (explicit per-facet paint). Config-stamped extruder is the default. The paint pipeline already handles MMU multi-material correctly. |
| Non-extruder keys | Stamped into extensions but not consumed by this packet | `"fuzzy_skin"`, `"matrix"`, and future keys survive in config.extensions. Downstream modules can read them via `ConfigView::get_extension(key)`. No per-key consumption in this packet. |
| GCode test assertion | String-search GCode output for `T0` and `T1`. Do NOT parse into structured commands. Position-independent search. | Simple, fast, sufficient. Entity ordering depends on path optimization which may reorder. Tool presence proves correctness. |
| Modifier priority | Higher `ModifierVolume.priority` wins. Last writer via `overlay_resolved` after sorting by priority ascending. | OrcaSlicer uses insertion order; we use explicit priority. Both achieve deterministic last-writer-wins. |
| Config-extensions type | `HashMap<String, ConfigValue>` — string keys, typed values. | Matches `config_delta.fields` exactly (same types). No conversion needed. |

## Rejected Alternatives

| Alternative | Reason rejected |
|---|---|
| Route extruder through `PaintValue::ToolIndex` in `paint_segmentation.rs` (previous packet version) | Too narrow — handles only one key, couples config routing to paint pipeline, and does not achieve OrcaSlicer parity where any config key can be overridden. |
| Add `extruder: Option<u32>` to `RegionPlan` | Unnecessary typed field. The `extensions` bucket carries it with zero IR cost. Would set precedent for adding one field per config key. |
| Store extruder on `ModifierVolume` as a typed field | Requires `slicer-ir` schema bump. `config_delta` already carries the value generically. |
| Create new `RegionPlan` entries for each modifier-driven config delta | OrcaSlicer creates distinct `PrintRegion`s, but our `RegionPlan` with `config` per region already handles distinct behavior. Creating new entries would require splitting region polygons by modifier overlap — complex, out of scope. |
| Polygon-level overlap detection | Correct but complex. Bbox overlap achieves the same result for rectangular/cuboid modifier volumes (the common case). Polygon overlap is a refinement that can follow. |
| Thread `config_delta` through a new PrePass stage | Adds complexity. Region Mapping already has all the inputs (MeshIR, polygons). A function call is simpler than a new stage. |
| Stamp config into `paint_overrides` instead of `config.extensions` | `paint_overrides` is per-PaintSemantic, requiring a semantic key. Modifier config deltas are volume-driven, not paint-driven. The `config.extensions` bucket is the correct carrier. |
| Stamp ENFORCER/BLOCKER config into RegionPlan | OrcaSlicer treats `support_enforcer` and `support_blocker` `extruder` (and all other config fields) as **decorative** (`OrcaSlicerDocumented/src/libslic3r/PrintApply.cpp:590-594` excludes them from `model_volume_solid_or_modifier()`). Stamping would cause spurious T-changes for `bridge_support_enforcers.3mf` where enforcer extruder=0 differs from the parent's extruder=1. See Subtype filter row. |

## Code Change Surface

Primary files this packet edits:

1. **`crates/slicer-host/src/region_mapping.rs`** — ~45 added lines. New function `stamp_modifier_config_deltas(mesh_ir: &MeshIR, region_extent: &BoundingBox2, base_config: &mut ResolvedConfig)` called inside `execute_region_mapping_with_cap()` after per-object config resolution. Iterates `mesh_ir.modifier_volumes`, **skips entries whose `config_delta.fields["subtype"]` is `String("support_enforcer")` or `String("support_blocker")`** (Subtype filter row), computes 2D bbox from modifier mesh vertices, checks overlap with region extent, stamps non-subtype keys into base_config.extensions via `overlay_resolved()`.

2. **`crates/slicer-host/src/layer_executor.rs`** — ~8 added lines. In the `required_tool` resolution (line ~762), add a fallback: if `dominant_tool_index()` returns `None`, check `region_plan.config.extensions.get("extruder")`. If `Some(ConfigValue::Int(n))`, use `n as u32` for `region_id`.

3. **`crates/slicer-host/tests/threemf_fixture_e2e_tdd.rs`** — ~250 added lines. 7 new/updated test functions (including 2 RED tests turned GREEN + 5 new tests).

4. **`docs/07_implementation_status.md`** — append TASK-208, TASK-209 rows.

## Files in Scope (read + edit)

- `crates/slicer-host/src/region_mapping.rs` — role: config stamping insertion point; expected change: add `stamp_modifier_config_deltas()` + call site
- `crates/slicer-host/src/layer_executor.rs` — role: tool-index fallback from config extensions; expected change: add extensions-driven `required_tool` fallback
- `crates/slicer-host/tests/threemf_fixture_e2e_tdd.rs` — role: fixture E2E test harness; expected change: add 7 config-stamping and GCode tests

## Read-Only Context

| Path | Lines | Purpose |
|---|---|---|
| `crates/slicer-host/src/region_mapping.rs` | lines 100-193 | `overlay_resolved()` — the merge function this packet calls. Confirm `extensions` handling at line 193. |
| `crates/slicer-host/src/region_mapping.rs` | lines 279-455 | `execute_region_mapping_with_cap()` — the insertion point. Confirm existing `base_config` resolution and ordering relative to `paint_overrides`. |
| `crates/slicer-host/src/region_mapping.rs` | lines 200-237 | `overlapping_semantics_for_region()` — reference pattern for polygon overlap detection. |
| `crates/slicer-host/src/layer_executor.rs` | lines 756-766 | `dominant_tool_index()` → `region_id` assignment. The fallback insertion point. |
| `crates/slicer-ir/src/slice_ir.rs` | lines 365-398 | `ConfigDelta`, `ModifierVolume`, `ModifierScope` struct definitions. |
| `crates/slicer-ir/src/resolved_config.rs` | lines 398-440 | `extensions` field on `ResolvedConfig`. |

## Out-of-Bounds Files

- `crates/slicer-host/src/paint_segmentation.rs` — no longer edited. Paint path untouched.
- `crates/slicer-host/src/gcode_emit.rs` — tool-change emission already correct. No edits.
- `crates/slicer-host/src/model_loader.rs` — config_delta parsing already exists (lines 557-587). Read-only confirmation at Step 0; no edits.
- `crates/slicer-host/src/slice_postprocess.rs` — paint annotator unchanged. No edits.
- `crates/slicer-ir/` — no IR changes needed. `ConfigDelta`, `ConfigValue`, `ResolvedConfig.extensions` all exist.
- `crates/slicer-macros/`, `crates/slicer-sdk/` — no SDK or macro involvement.
- `wit/**`, `crates/slicer-host/src/wit_host.rs`, `dispatch.rs` — WIT clean.
- `OrcaSlicerDocumented/**` — delegate only. Never load directly.
- `target/`, `Cargo.lock`, generated code.

## Expected Sub-Agent Dispatches

- "Run `cargo test -p slicer-host --test threemf_fixture_e2e_tdd -- --list` to confirm test names exist; return FACT list of test names" — Step 0 precondition check
- "At `region_mapping.rs:193`, does `overlay_resolved()` merge `extensions` keys? Return SNIPPETS (lines 190-195)" — Step 1 discovery
- "At `region_mapping.rs:279-455`, what is the per-region loop structure? Where does `base_config` get resolved relative to paint_overrides? Return SNIPPETS (the loop body outline, ≤ 30 lines)" — Step 1 discovery
- "Run `cargo test -p slicer-host --test threemf_fixture_e2e_tdd config_delta_extruder_stamped_into_extensions -- --exact --nocapture`. FACT pass/fail." — Step 3 verification
- "Run `cargo test -p slicer-host --test threemf_subtypes_synthetic_e2e_tdd`. FACT pass/fail." — Step 4 regression
- "Run `cargo clippy --workspace -- -D warnings`. FACT pass/fail." — Step 5 cleanup

## Data and Contract Notes

- `ConfigDelta.fields` is `HashMap<String, ConfigValue>`. Key `"subtype"` is excluded from stamping. All other keys are stamped.
- Subtype values `support_enforcer` and `support_blocker` cause the entire modifier_volume to be skipped during stamping (OrcaSlicer parity; see Subtype filter row and `PrintApply.cpp:590-594`).
- `ConfigValue::Int(i64)` — for `"extruder"`, cast to `u32` via `try_into().unwrap_or(0)`. Valid extruder values are 0-15.
- `ConfigValue::String(s)` — for `"fuzzy_skin"`, `"matrix"`, and future keys. Copied verbatim into extensions.
- `ConfigValue::Bool(b)`, `ConfigValue::Float(f)`, `ConfigValue::List(v)` — supported but not used by current fixtures. Future-proof.
- `ResolvedConfig.extensions` is `HashMap<String, ConfigValue>` — same type as `ConfigDelta.fields`. Direct copy after filtering.
- `overlay_resolved(&mut base, &overlay)` at region_mapping.rs:100-193 merges `extensions` at line 193: for each key in overlay.extensions, inserts into base.extensions (overwriting if present).
- `ModifierVolume.priority: u32` — higher value = higher priority. The stamping loop sorts by priority ascending before stamping, so higher-priority modifiers stamp last and win.
- `ModifierVolume.mesh: IndexedTriangleSet` — provides vertex positions in world space. The 2D bbox is computed from `(x, y)` components of all vertices.

## Locked Assumptions and Invariants

1. `config_delta.fields` uses `ConfigKey = String` and `ConfigValue` enum — same types as `ResolvedConfig.extensions`. No type conversion needed between stamp source and destination.
2. `overlay_resolved()` merges `extensions` keys by simple insertion (line 193: `base.extensions.insert(k, v)`). This is the last-writer-wins behavior needed for priority ordering.
3. The per-region loop in `execute_region_mapping_with_cap()` has access to region polygon data sufficient to compute a 2D bbox for overlap checking.
4. `mesh_ir: &MeshIR` is available in the region-mapping prepass scope (either from function parameters or from the blackboard). If not directly available, it must be threaded through.
5. `dominant_tool_index()` returns `None` when no paint-derived tool exists — the config-extensions fallback is only used in that case. Paint tools take precedence.
6. The `"subtype"` exclusion filter is correct for all current and future modifier volumes. If a future modifier type uses `"subtype"` as a config key (unlikely), the exclusion can be narrowed.
7. Modifier overlaps are computed per-region at region-mapping time. If a region's polygons span a superset of the modifier's extent, the modifier's config applies to all layers in that region. This is intentionally coarse — refinement to per-layer Z-range intervals is future work.
8. Packet 57's fixture tests exist and are compilable — this packet extends them.

## Risks and Tradeoffs

| Risk | Mitigation |
|---|---|
| Bbox overlap is too coarse — a small modifier affects far-away regions in the same object | Bbox is correct for the common case (cuboid modifiers). For complex meshes, a region that doesn't overlap the modifier's bbox won't get stamped. False positive risk is low. If this becomes an issue, polygon-level overlap is the refinement path. |
| `mesh_ir` not available in the region-mapping prepass scope | Step 1 discovery checks this. If unavailable, thread it through the function signature or store on the blackboard via a new prepass parameter. |
| Existing `config.extensions` keys from CLI config could collide with modifier-stamped keys | The `overlay_resolved` merge is last-writer-wins. Modifier config_delta stamps after per-object but before paint. If CLI sets `extensions["extruder"]`, the modifier overrides it. This is correct behavior. |
| `ConfigValue::Int(0)` for extruder is indistinguishable from an absent value | Special-case the default-skip: only skip empty strings and empty lists. `Int(0)` for `"extruder"` is meaningful. Use a key-aware check. |
| Non-extruder keys stamped into extensions but never consumed | Intentionally deferred. The stamping mechanism is the primary deliverable. Individual key consumers (e.g., `"fuzzy_skin"` → fuzzy-skin module) are follow-up packets. The AC-3 test proves the keys survive. |
| `dominant_tool_index()` already returns a tool from paint — config-extensions fallback might never be reached | In the existing `bridge_support_enforcers.3mf` fixture, modifier volumes have NO paint (`PaintValue::Flag(true)` after Packet 56c). After this packet, `PaintValue::Flag(true)` → `dominant_tool_index()` returns `None` → config-extensions fallback kicks in. The fallback path is exercised. |

## Context Cost Estimate

- Aggregate: **M** (~40-line production change in `region_mapping.rs`, ~8-line change in `layer_executor.rs`, ~250-line test extension).
- Largest single step: Step 3 (author GCode integration tests — string-searching GCode output) — M.
- No L-rated step.
- Highest-risk dispatch: Step 1 discovery — confirming `overlay_resolved` handles extensions and `mesh_ir` is available. If negative, design adjusts before any code is written.

## Open Questions

- `[FWD]` Does `execute_region_mapping_with_cap()` have direct access to `mesh_ir: &MeshIR` or does it need to be threaded through? Checked in Step 1. If unavailable, the function signature adds a `mesh_ir: &MeshIR` parameter and callers are updated.
- `[FWD]` Does `overlay_resolved()` at line 193 insert extensions keys or merge them? Checked in Step 1. If it inserts (not merges), last-writer-wins works correctly.
- `[FWD]` What is the exact type of region extent data available in the per-region loop? Checked in Step 1. If no explicit bbox exists, compute from region polygons.
