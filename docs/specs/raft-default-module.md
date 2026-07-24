# `raft-default` Module — Design Sketch

## Context

`raft-default` is the default raft-region-synthesizer module: it reads
`SupportPlanIR.raft_plan` (defined in `docs/specs/support-modules-orca-port.md`
§C6) and produces the raft polygons + Z values that downstream `Layer::Infill`
modules render. It does NOT render fill paths itself. Pattern variety
(rectilinear, grid, lightning, honeycomb, community patterns) comes from
whichever `Layer::Infill` module(s) declare `claim:raft-fill`.

This design satisfies three constraints:

1. **Multi-language module promise.** No Rust-library extraction — a C++
   TPMS-Infill module can render raft via `claim:raft-fill` like any other
   fill role.
2. **No pattern duplication.** Raft rendering reuses the fill-math already
   implemented in `rectilinear-infill`, `lightning-infill`, `gyroid-infill`,
   etc. `raft-default` ships zero pattern code.
3. **Blackboard single-writer compliance.** `raft-default` writes its own IR
   (raft regions); `support-planner` keeps sole ownership of `SupportPlanIR`.

See ADR-0009 for the architectural decision behind the role/claim direction.

## Authoritative References

- `docs/specs/support-modules-orca-port.md` — sibling spec. Defines the seam: `SupportPlanIR.raft_plan`, `ExtrusionRole::RaftInfill`, `claim:raft-fill`.
- `docs/01_system_architecture.md` — stage and claim conventions.
- `docs/adr/0009-raft-as-layer-infill-role.md` — direction.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp` — `generate_raft_base` for reference behavior (footprint shape, layer staggering, gap layer treatment).
- `crates/slicer-sdk/src/views.rs:228-359` — existing per-role fill-area carriers + `should_emit` dispatch.

## Inputs

| IR | Source | Used For |
|---|---|---|
| `SupportPlanIR.raft_plan: Vec<RaftPlan>` | `support-planner` (PrePass::SupportGeometry) | Per-object raft footprint, layer specs, gap, density. |
| `LayerPlanIR` | host built-in | Reference for layer-height and Z conventions. |
| Config: `raft_pattern`, `raft_z_gap_mm`, `raft_first_layer_density`, `raft_layer_height_mm`, `raft_expansion_mm` | module manifest | Raft-specific config namespace. |

## Output

The output shape is the central open question. Two viable carriers, each
with consequences. **This spec records both candidates and the trade-off; the
choice lands as a sub-decision at implementation time.**

### Carrier (a) — Synthetic raft layers below model

`raft-default` extends the global layer iteration with synthetic layers below
model Z=0. Each synthetic layer carries one `SliceRegion` per object with
raft, whose `raft_fill: Vec<ExPolygon>` is the raft footprint at that layer's
Z. Iteration in `Layer::Infill` naturally includes raft layers; the infill
module sees them as ordinary regions tagged with `raft_fill`.

Required changes:
- `LayerPlanIR` accepts synthetic-below-model layers (negative or `RaftLayerIndex` variant on the layer-index type).
- `SliceIR` accepts entries at synthetic layer indices.
- `SliceRegionView` gains a `raft_fill: Vec<ExPolygon>` field, populated only on raft layers.
- The host's gcode emitter walks raft layers first (Z ordering already handles this if `raft_layer.z < model_layer.z`).

Pro: single layer-iteration path; the existing infill loop "just works" with the new role variant. Con: contract changes to two foundational IRs.

### Carrier (b) — Separate `RaftRegionIR` consumed by a parallel iteration

`raft-default` writes a new `RaftRegionIR { entries: Vec<RaftRegionEntry> }`
on the blackboard. Each entry is keyed `(raft_layer_index, object_id)` and
carries the raft footprint at that layer. A new stage `Layer::Raft` (or a
parallel sub-loop inside `Layer::Infill`) iterates `RaftRegionIR.entries`
and dispatches to fill modules that declare `claim:raft-fill`.

Required changes:
- New IR (`RaftRegionIR`) on the blackboard.
- New stage (`Layer::Raft`) in the scheduler, or `Layer::Infill` gains a second iteration sub-loop.
- `SliceRegionView` does NOT change; raft regions are a sibling carrier with their own view type.

Pro: clean separation; foundational IRs unchanged. Con: parallel iteration / new stage adds scheduler surface.

**Recommendation in this spec**: lean toward **(a)** — the cost of carrier
contract changes is bounded (`LayerPlanIR` and `SliceIR` are the only two
affected, and their layer-index abstraction is the right place to model "this
layer Z exists below model 0"). The implementation packet picks one with
explicit rationale and a one-paragraph justification appended to ADR-0009.

## Stage Placement

| Stage | Module | Role |
|---|---|---|
| `PrePass::SupportGeometry` | `support-planner` (existing) | Emits `SupportPlanIR.entries` + `SupportPlanIR.raft_plan`. |
| `PrePass::RaftSynthesis` (NEW) | `raft-default` (this module) | Reads `SupportPlanIR.raft_plan`, populates raft regions per Carrier (a) or (b). |
| `Layer::Infill` | existing infill modules + `raft-fill` claim | Renders raft via `ExtrusionRole::RaftInfill`. |

`PrePass::RaftSynthesis` is a new stage rather than an extension of
`PrePass::SupportGeometry` because:
- Raft can be needed without supports (adhesion-raft for warpy materials). Decoupling stages preserves that workflow.
- The blackboard single-writer rule already prevents two modules from writing `SupportPlanIR`; a new stage avoids that pitfall.
- The scheduler DAG can express "raft synthesis runs after support planning, before layer execution" cleanly.

Claim: `raft-synthesizer` (single holder per object).

## Module Manifest Sketch

```toml
[module]
id           = "com.core.raft-default"
version      = "0.1.0"
display-name = "Raft Synthesizer (default)"
description  = "Synthesizes raft regions from SupportPlanIR.raft_plan for downstream Layer::Infill rendering"
author       = "modular-slicer"
license      = "MIT"
wit-world    = "slicer:world-prepass@1.0.0"

[stage]
id = "PrePass::RaftSynthesis"

[ir-access]
reads  = ["SupportPlanIR", "LayerPlanIR"]
writes = ["RaftRegionIR"]  # or "SliceIR" if Carrier (a) is chosen

[claims]
holds    = ["raft-synthesizer"]
requires = []

[compatibility]
incompatible-with = []
requires          = []
min-host-version  = "0.1.0"
min-ir-schema     = "1.0.0"
max-ir-schema     = "5.0.0"

[config.schema]
[config.schema.raft_z_gap_mm]
type    = "float"
default = 0.2
min     = 0.0
max     = 2.0
display = "Raft Z Gap"
group   = "Raft"

[config.schema.raft_first_layer_density]
type    = "float"
default = 1.0
min     = 0.5
max     = 1.0
display = "Raft First Layer Density"
group   = "Raft"

[config.schema.raft_layer_height_mm]
type    = "float"
default = 0.2
min     = 0.05
max     = 0.5
display = "Raft Layer Height"
group   = "Raft"

[config.schema.raft_expansion_mm]
type    = "float"
default = 2.0
min     = 0.0
max     = 10.0
display = "Raft Expansion (outside object footprint)"
group   = "Raft"

[hints]
estimated-ms-per-layer = 1
layer-parallel-safe    = true
```

`raft_pattern` is NOT a config key on `raft-default`. Pattern selection is
controlled by which `Layer::Infill` module holds `claim:raft-fill`. This is
the same model as `claim:top-fill` / `claim:bottom-fill` already use.

## Infill Module Updates (lands with this module)

Each existing infill module gains a small dispatch block, mirroring the
existing `TopSolidInfill` / `BottomSolidInfill` cases. Reference: the
`rectilinear-infill::run_infill` pattern at `lib.rs:139-172`. Add:

```rust
// Raft fill emission (when this module holds claim:raft-fill).
if !region.raft_fill().is_empty() && region.should_emit(ExtrusionRole::RaftInfill) {
    let paths = self.fill_expolygon_multi(
        region.raft_fill(),
        line_spacing,
        std_cos_a,
        std_sin_a,
        z,
        speed_factor,
        ExtrusionRole::RaftInfill,
    );
    for path in paths {
        output.push_solid_path(path);  // raft is a "solid" output channel
    }
}
```

For modules with non-trivial fill APIs (lightning-infill, gyroid-infill), the
shape is the same: detect raft regions, call the module's existing fill
function with the raft polygon, tag output with `RaftInfill`.

A module that declares `claim:raft-fill` in its manifest signals intent. Any
infill module can declare the claim by adding it to `[claims].holds`. The
default in v1 is that `rectilinear-infill` declares it (matches Orca's
default raft pattern); users who want pattern variety swap the claim to a
different infill module.

## Synthesizer Behavior

For each `RaftPlan` in `SupportPlanIR.raft_plan`:

1. **Compute the expanded footprint.** Offset `plan.footprint` outward by `raft_expansion_mm` (default 2.0) so the raft extends slightly past the object base. Use `slicer_core::polygon_ops::offset`.
2. **Per raft layer** (from `plan.layers`):
   - Compute the layer Z (taken from `RaftLayerSpec.z`).
   - Compute the layer height (`RaftLayerSpec.height`).
   - For Carrier (a): synthesize a `LayerPlanIR` entry at this Z, and a `SliceIR` entry with one `SlicedRegion` per object carrying the expanded footprint as `raft_fill`.
   - For Carrier (b): emit a `RaftRegionEntry { raft_layer_index, object_id, z, height, footprint }`.
3. **First-layer adjustment.** For the bottom-most raft layer (highest `dist_to_bed`), if `raft_first_layer_density < 1.0`, scale the polygon's `infill_density` view-side (carried as a per-region density override) so the renderer fills sparsely. The default of 1.0 produces dense fill.
4. **Multi-object dedup.** If two objects' expanded footprints overlap, union them into a single raft region. Otherwise emit them as separate regions on the same raft layer.

The synthesizer does NOT make pattern decisions, does NOT compute fill paths,
does NOT touch extrusion math.

## Open Seams (this spec does not commit; raft-default implementation packet picks)

- **Carrier (a) vs (b).** Pick at implementation time with a one-paragraph addendum to ADR-0009. Default: (a).
- **`Layer::Infill` second iteration vs new `Layer::Raft` stage.** Coupled to Carrier choice. (a) implies natural iteration in `Layer::Infill`; (b) requires either a sub-loop or a new stage.
- **Per-layer density override for first-layer raft.** Mechanism for surfacing `raft_first_layer_density` to the infill module — separate `region.raft_density_override()` accessor, or a flag on `SliceRegionView` the module reads alongside `infill_density`.
- **gap layer geometry.** Orca's `generate_raft_base` emits a special "gap layer" between top raft and model bottom with reduced flow. v1 of `raft-default` may skip the gap-layer concept and rely on `raft_z_gap_mm` for vertical clearance; gap-layer with reduced flow is a v2 enhancement.

## Test Plan

- Unit: `raft-default` reads a synthetic `RaftPlan` with 3 layers, emits 3 raft regions per object.
- Unit: multi-object fixture with overlapping footprints — raft regions union correctly.
- Integration: `regression_wedge.stl` with `support_raft_layers = 3`, `enable_support = true`. Round-trip through the prepass, assert:
  - `RaftRegionIR.entries.len() == 3` (or, for Carrier (a), 3 new layers in `LayerPlanIR`).
  - Each raft region's footprint contains the object's first-layer XY bbox.
  - First raft layer Z = `z_bed - raft_layer_height * 3`.
- Integration: `Layer::Infill` step with `rectilinear-infill` declaring `claim:raft-fill`. Assert raft layers produce `ExtrusionRole::RaftInfill` paths whose XY extents cover the expanded footprint.
- Negative: `support_raft_layers = 0`. Assert zero raft regions emitted.
- Negative: no infill module declares `claim:raft-fill`. Assert raft regions exist in the IR but produce zero paths; a `LogLevel::Warn` diagnostic is emitted indicating no raft renderer is loaded.

## Out of Scope

- Pattern algorithms (live in the infill modules, not here).
- `Layer::Support` interactions (raft does not interact with the support generator's branch paths beyond sharing the build-plate Z).
- Multi-extruder raft material selection (`raft_filament`).
- Raft skirt / brim coordination.
- Honeycomb / lightning / community patterns as their own raft-specific modules (not needed — the infill modules provide them).

## TASK Ledger

| TASK | Description |
|---|---|
| TASK-280 | `raft-default` module skeleton: manifest, `PrePass::RaftSynthesis` stage registration, claim. |
| TASK-281 | Carrier decision (a) vs (b) — pick at packet kickoff, append to ADR-0009. |
| TASK-282 | Synthesizer implementation per Carrier choice. |
| TASK-283 | Infill module dispatch additions (`rectilinear-infill`, `lightning-infill`, `gyroid-infill`). |
| TASK-284 | `claim:raft-fill` added to `rectilinear-infill.toml` as v1 default. |
| TASK-285 | Integration tests on `regression_wedge.stl`. |
| TASK-286 | Negative test: no raft renderer loaded → `LogLevel::Warn` diagnostic. |
