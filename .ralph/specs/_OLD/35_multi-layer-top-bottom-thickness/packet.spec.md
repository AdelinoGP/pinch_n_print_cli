---
status: implemented
packet: multi-layer-top-bottom-thickness
task_ids:
  - TASK-165
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: multi-layer-top-bottom-thickness

## Goal

Honor per-region `top_shell_layers` and `bottom_shell_layers` config keys (Orca-equivalent) so a region's top/bottom solid-fill window spans N layers below/above a `TopSurface` / `BottomSurface` facet rather than the single-layer window introduced in packet `12-rev1`. The classifier widens its Z window to a sum of the next/prev N layer Zs from `LayerPlanIR.global_layers`, with N looked up per-region from `RegionMapIR.entries[*].config`.

This is a strict additive extension of packet `12-rev1`'s `classify_region_surfaces` — same algorithm, wider window, sourced from per-region resolved config.

## Scope Boundaries

- In scope:
  - threading `Option<&RegionMapIR>` into `execute_layer_slice` and through to `classify_region_surfaces`
  - looking up `top_shell_layers: u32` and `bottom_shell_layers: u32` per `(global_layer_index, object_id, region_id)` from `RegionMapIR.entries[*].config`
  - computing the multi-layer Z window by walking `LayerPlanIR.global_layers[layer_idx + 1 .. layer_idx + 1 + N]` (and symmetrically for bottom), taking the last Z in the window or `+∞` when truncated
  - default values `top_shell_layers = 3`, `bottom_shell_layers = 3` (matches the codebase's existing default for these fields, which deviates from Orca's `top_shell_layers = 4` / `bottom_shell_layers = 3` — this deviation is not introduced by packet 35; see design.md)
  - new TDD coverage for multi-layer windows including window-truncation edge cases
- Out of scope:
  - bridge-detector parity (packet 36)
  - polygon-polygon overlap replacing centroid/any-vertex test (packet 36 territory)
  - per-surface fill pattern variation (packet 37)
  - top-surface ironing (packet 38)
  - any change to dispatch, WIT, SDK, or scheduler claim system

## Prerequisites and Blockers

- Depends on:
  - packet `12-rev1_external-surface-classification-at-slice` — provides `is_top_surface` / `is_bottom_surface` fields on `SlicedRegion`, the `classify_region_surfaces` helper, and the `execute_layer_slice` signature widened with `surface_class` + adjacent Z values
- Unblocks:
  - packet `36_bridge-detector-orca-parity` — reuses the same `RegionMapIR` plumbing for bridge-detector config
  - packet `37_fill-role-claims` — reuses the same `RegionMapIR` plumbing for per-claim module selection
  - packet `38_top-surface-ironing` — needs precise topmost-layer detection (this packet provides it via the `top_shell_layers` window)
- Activation blockers:
  - packet `12-rev1` must be `implemented` first (this packet relies on its `classify_region_surfaces` helper and `SlicedRegion` schema)

## Acceptance Criteria

- **Given** a region with resolved config `top_shell_layers = 3` and a `TopSurface` facet whose `z_min` lies inside the 3-layer window above `layer.z` (i.e. `[z_n, z_{n+3})`), **when** `classify_region_surfaces` runs against the multi-layer window, **then** every layer in `[n, n+3)` flags `is_top_surface=true` for that region. | `cargo test -p slicer-host --test multi_layer_thickness_tdd top_shell_layers_three_flags_three_layers -- --exact --nocapture`
- **Given** a region with resolved config `bottom_shell_layers = 3` and a `BottomSurface` facet whose `z_max` lies inside the 3-layer window below `layer.z`, **when** the helper runs, **then** every layer in `(n-3, n]` flags `is_bottom_surface=true`. | `cargo test -p slicer-host --test multi_layer_thickness_tdd bottom_shell_layers_three_flags_three_layers -- --exact --nocapture`
- **Given** an object with only 2 active layers and a region with resolved config `top_shell_layers = 5`, **when** the helper runs on layer 0, **then** both layers flag `is_top_surface=true` (window naturally truncates to the available layers; no panic, no out-of-bounds). | `cargo test -p slicer-host --test multi_layer_thickness_tdd window_truncates_at_object_extent -- --exact --nocapture`
- **Given** a region whose resolved config omits `top_shell_layers`, **when** the helper runs, **then** the window defaults to `3` layers and the resulting flags match the `top_shell_layers = 3` case from the first AC. | `cargo test -p slicer-host --test multi_layer_thickness_tdd missing_config_uses_default_three -- --exact --nocapture`
- **Given** `execute_layer_slice` invoked with `region_map: Some(&region_map_ir)` whose `entries[*].config` carries `top_shell_layers = 2`, **when** the call returns for a layer 1 step below a `TopSurface` facet at layer 2, **then** `SliceIR.regions[0].is_top_surface == true`. | `cargo test -p slicer-host --test multi_layer_thickness_tdd execute_layer_slice_honors_region_map_top_shell_layers -- --exact --nocapture`
- **Given** the unmodified Benchy STL run end-to-end with `top_shell_layers = 4` and `bottom_shell_layers = 4`, **when** the slicer produces G-code, **then** the count of `;TYPE:Top surface` blocks AND the count of `;TYPE:Bottom surface` blocks each is at least `4`. | `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_multi_layer_top_bottom_evidence -- --exact --nocapture`

## Negative Test Cases

- **Given** `region_map: None` (test fixtures that pre-seed without `RegionMapIR`), **when** `execute_layer_slice` runs, **then** the helper falls back to default `top_shell_layers = 3`, `bottom_shell_layers = 3`, NOT to `1` (which would be a regression vs codebase defaults). | `cargo test -p slicer-host --test multi_layer_thickness_tdd none_region_map_uses_orca_defaults -- --exact --nocapture`
- **Given** `top_shell_layers = 0` (user explicitly disables top surfaces), **when** the helper runs, **then** `is_top_surface` is `false` for every layer regardless of facet position. | `cargo test -p slicer-host --test multi_layer_thickness_tdd zero_top_shell_layers_disables_flag -- --exact --nocapture`
- **Given** `bottom_shell_layers = 0` (user explicitly disables bottom surfaces), **when** the helper runs, **then** `is_bottom_surface` is `false` for every layer regardless of facet position, AND `is_bridge` detection still functions normally for the same fixture. | `cargo test -p slicer-host --test multi_layer_thickness_tdd zero_bottom_shell_layers_disables_flag -- --exact --nocapture`

## Verification

- `cargo build --workspace`
- `cargo test --workspace`
- `cargo clippy --workspace -- -D warnings`

## Authoritative Docs

- `docs/02_ir_schemas.md` — `RegionMapIR.entries[*].config` schema; resolved-config semantics. Read directly; only the relevant section.
- `docs/04_host_scheduler.md` — `PrePass::RegionMapping` builds `RegionMapIR`; `RegionMap` invariants (one entry per `(layer, object, region)`). Delegate SUMMARY of § "RegionMapIR Compilation".
- `docs/03_wit_and_manifest.md` — config-key declaration rules; Step 0 FACT confirmed `top_shell_layers` / `bottom_shell_layers` are already in the central config schema (no additions needed).

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — `discover_horizontal_shells()` propagates top/bottom solid-fill regions across `top_shell_layers` / `bottom_shell_layers` (Orca calls these `top_solid_layers` / `bottom_solid_layers` internally). Parity touchstone for the window semantics. Delegate FACT confirming the propagation direction (top: window EXTENDS DOWNWARD from a top-facing facet; bottom: EXTENDS UPWARD from a bottom-facing facet).
- `OrcaSlicerDocumented/src/libslic3r/Print.hpp` — `PrintObject::process_external_surfaces()` declaration.

All OrcaSlicer reads MUST be delegated.

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`

## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
