# Requirements: 58_extruder-per-modifier-gcode

## Problem Statement

Packet 56's sidecar parser (`model_loader.rs`) parses `<metadata key="...">` from 3MF `<part>` elements and stores values in `ModifierVolume.config_delta.fields`. Currently, only the `"subtype"` key is consumed downstream (by `paint_segmentation.rs`, `negative_part_subtract.rs`, and `layer_executor.rs` modifier projections). All other keys — `"extruder"`, `"fuzzy_skin"`, `"matrix"`, and any future extensible keys — are parsed but silently ignored: they sit inert in `config_delta.fields` and never reach `RegionPlan.config`.

In OrcaSlicer, this is handled by `PrintApply::generate_print_object_regions()` which iterates every model volume, merges `config_delta` overrides into `PrintRegion.config`, and creates distinct regions when the resulting config differs. The modifier's config then drives every downstream stage — wall generation, infill density, speed, extruder selection — through the `PrintRegion.config` → `LayerRegion` → toolpath pipeline.

The infrastructure for config overrides already exists in our codebase:
- `overlay_resolved()` (`region_mapping.rs:100-193`) — merges one `ResolvedConfig` over another
- `paint_overrides: BTreeMap<PaintSemantic, ResolvedConfig>` (`RegionPlan`, slice_ir.rs:1126) — per-paint-semantic config overlays
- `RegionPlan.config.extensions: HashMap<String, ConfigValue>` — overflow bucket for unknown config keys
- `build_paint_semantic_configs()` (`prepass.rs:432-470`) — resolves `paint_config:<semantic>:<key>` from CLI config
- `overlapping_semantics_for_region()` (`region_mapping.rs:200-237`) — finds which paint semantics overlap a region's polygons

What's missing is the first step in the chain: reading `config_delta.fields` from modifier volumes, finding which volumes overlap a region, and stamping their keys into `RegionPlan.config.extensions`. Once stamped, every downstream stage automatically consumes the modifier's config — because they already read `RegionPlan.config`.

The previous version of this packet proposed a narrow workaround: route only `extruder` through `PaintValue::ToolIndex` in `paint_segmentation.rs`. That approach limits the mechanism to exactly one key and couples config routing to the paint pipeline. This revision implements the general mechanism: stamp ALL keys from `config_delta` into `config.extensions`, achieving OrcaSlicer parity for Region Config Modifiers.

## Task IDs

- **TASK-208** — Config delta stamping: in `region_mapping.rs`, for each region, find overlapping `ModifierVolume` entries and stamp their `config_delta.fields` (except `"subtype"`) into `RegionPlan.config.extensions` via `overlay_resolved()`. Add config-extensions-driven `required_tool` fallback in `layer_executor.rs` so the `"extruder"` key in `config.extensions` selects the tool when no paint-derived tool exists.
- **TASK-209** — GCode and config integration tests: extend `threemf_fixture_e2e_tdd.rs` with 7 new/updated tests (turn 2 RED tests from Packet 57 GREEN, add 5 new). Assert `T0`/`T1` tool commands from config-stamped extruder, assert non-extruder keys survive end-to-end, assert backward compatibility, assert priority ordering for conflicting modifiers.

## In Scope

- **Write:**
  - `crates/slicer-host/src/region_mapping.rs` — add `stamp_modifier_config_deltas()` function called inside `execute_region_mapping_with_cap()` (after per-object config resolution, before constructing `RegionPlan`). For each region, find `ModifierVolume` entries whose `applies_to` scope overlaps, read `config_delta.fields`, exclude `"subtype"`, and stamp remaining keys into a `ResolvedConfig.extensions` overlay applied via `overlay_resolved()`.
  - `crates/slicer-host/src/layer_executor.rs` — add config-extensions-driven `required_tool` fallback: when `dominant_tool_index()` returns `None` (no paint-derived tool), check `region_plan.config.extensions["extruder"]` and use its `ConfigValue::Int(n)` value for `region_id`-as-tool assignment.
  - `crates/slicer-host/tests/threemf_fixture_e2e_tdd.rs` — add 7 tests: `config_delta_extruder_stamped_into_extensions` (AC-1), `extruder_per_object_vs_support_extruder` (AC-2, was RED), `config_delta_non_extruder_key_survives` (AC-3, new), `negative_part_extruder_does_not_affect_subtract` (AC-4, was RED), `subtype_only_modifier_stamps_no_extensions` (AC-N1, new), `conflicting_extruder_modifier_priority_wins` (AC-N2, new), and regression pass-through for existing 9 tests from Packet 57.
  - `docs/07_implementation_status.md` — append TASK-208 and TASK-209 rows after TASK-207.

- **Read-only:**
  - `crates/slicer-host/src/region_mapping.rs` — `overlay_resolved()` (lines 100-193), `overlapping_semantics_for_region()` (lines 200-237), `execute_region_mapping_with_cap()` (lines 279+).
  - `crates/slicer-host/src/layer_executor.rs` — `dominant_tool_index()` (line 756-766), `region_id`-as-tool assignment (line 762).
  - `crates/slicer-ir/src/slice_ir.rs` — `ModifierVolume`, `ConfigDelta`, `ConfigValue`, `ModifierScope` definitions.
  - `resources/bridge_support_enforcers.3mf` — fixture with extruder=0 on support parts, extruder=1 on parent objects.

## Out of Scope

- Changes to `paint_segmentation.rs` — no longer needed. The config-stamping approach routes config keys without paint indirection.
- Changes to `gcode_emit.rs` — existing `required_tool = region_key.region_id as u32` mechanism is sufficient (extended by `layer_executor.rs` fallback).
- Changes to `model_loader.rs` — extruder and other metadata parsing already exists.
- Changes to `region_mapping.rs` `paint_overrides` population — existing TASK-181 infrastructure is reused, not modified.
- Adding `extruder` field to `RegionPlan`, `SlicedRegion`, or any IR struct — the `extensions` bucket carries config keys without IR schema changes.
- WASM guest changes, WIT changes, SDK changes, macros changes.
- Per-layer Z-range modifier intervals — every modifier applies globally across all layers (matches current `ModifierScope::AllFeatures` behavior).
- Creating distinct `PrintRegion`-equivalent IR types — existing `RegionPlan` with config overrides is sufficient for per-region behavior.
- `negative_part` extruder semantics — extruder on negative_part is metadata baggage for geometry-only subtract operations.
- Per-object extruder assignment via object-level metadata — separate concern already handled by material paint.
- `config_delta.fields` keys that map to typed `ResolvedConfig` fields — only the `extensions` bucket is used. Typed field stamping is future work.

## Authoritative Docs

- `docs/02_ir_schemas.md` — `ModifierVolume.config_delta` (`ConfigDelta { fields: HashMap<ConfigKey, ConfigValue> }`), `RegionPlan.config.extensions`, `ConfigValue` enum variants. Delegate narrow search for `extensions` field documentation.
- `docs/04_host_scheduler.md` — region-mapping prepass ordering, paint-semantic-aware config overlay flow. Delegate SUMMARY.
- `docs/01_system_architecture.md` — `extensions` bucket contract. Delegate narrow search if > 100 lines on topic.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintApply.cpp` — `generate_print_object_regions()` at line 1049: merges modifier volume config delta into parent region config via `region_config_from_model_volume()`.
- `OrcaSlicerDocumented/src/libslic3r/Print.hpp` — `PrintRegionConfig` and `VolumeRegion` structs: data structures carrying per-region config overrides with modifier parent chaining.
- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — `slicing_parameters()` at line 3695: per-volume region config computation for extruder collection.

## Acceptance Summary (references ACs by ID)

- **AC-1** — `config_delta_extruder_stamped_into_extensions`: `RegionPlan.config.extensions["extruder"] = ConfigValue::Int(0)` for regions overlapping support_enforcer modifier volumes.
- **AC-2** — `extruder_per_object_vs_support_extruder` (was RED in Packet 57): full pipeline GCode output contains both `T0` and `T1` commands.
- **AC-3** — `config_delta_non_extruder_key_survives`: non-extruder keys (`"fuzzy_skin"`) from `config_delta.fields` appear in `RegionPlan.config.extensions`.
- **AC-4** — `negative_part_extruder_does_not_affect_subtract` (was RED in Packet 57): extruder on negative_part is benign; polygon output unchanged.
- **AC-5** — `threemf_subtypes_synthetic_e2e_tdd` stays GREEN — no regression from config stamping on fixtures without non-subtype keys.
- **AC-6** — `gcode_emit_tdd` stays GREEN — existing tool-change tests unaffected.
- **AC-7** — `cargo clippy --workspace -- -D warnings` clean.
- **AC-8** — `docs/07` registration greps: exactly one match for TASK-208 and TASK-209 in `docs/07_implementation_status.md`.
- **AC-9** — `docs/04_host_scheduler.md` mentions modifier config delta stamping in region-mapping section.
- **AC-N1** — `subtype_only_modifier_stamps_no_extensions`: modifier with only `"subtype"` key stamps no extensions.
- **AC-N2** — `conflicting_extruder_modifier_priority_wins`: conflicting extruder values resolved by modifier priority (higher priority wins via `overlay_resolved` last-writer semantics).

## Verification Commands

| Command | Purpose | Delegation hint |
|---------|---------|----------------|
| `cargo test -p slicer-host --test threemf_fixture_e2e_tdd` | All fixture E2E tests GREEN | FACT pass/fail per test |
| `cargo test -p slicer-host --test threemf_subtypes_synthetic_e2e_tdd` | 56c regression | FACT pass/fail |
| `cargo test -p slicer-host --test gcode_emit_tdd` | Tool-change regression | FACT pass/fail |
| `cargo test -p slicer-host --test threemf_sidecar_classification_tdd` | 56 regression | FACT pass/fail |
| `cargo test -p slicer-host --test benchy_painted_e2e_tdd --test benchy_painted_overrides_e2e_tdd` | Paint pipeline regression | FACT pass/fail per file |
| `cargo clippy --workspace -- -D warnings` | Lint gate | FACT pass/fail |

## Step Completion Expectations

Cross-step invariants:
- The `overlay_resolved()` function (lines 100-193 of `region_mapping.rs`) is the single merge point. Config stamping calls it once per overlapping modifier. No step may introduce a second merge mechanism.
- `config_delta.fields` keys are `HashMap<String, ConfigValue>`. The stamping code checks `config_value != ConfigValue::default()` before stamping (protects against default-valued keys from empty modifier construction).

## Context Discipline Notes

- Large files in the read-only path: `region_mapping.rs` is ~500 lines — delegate overview to sub-agent, then read only the target functions (lines 100-193 `overlay_resolved`, lines 279+ `execute_region_mapping_with_cap`).
- Likely temptation reads: `paint_segmentation.rs` — this packet no longer edits it. `model_loader.rs` — config_delta parsing already confirmed. Skip both.
- Sub-agent return-format: all test runs return FACT pass/fail or SNIPPETS ≤ 20 lines on failure.
