---
status: implemented
packet: 52_gcode-feedrate-emission
task_ids:
  - TASK-153
---

# 52_gcode-feedrate-emission

## Goal

Wire a per-role feedrate (F-token) into every emitted `G0`/`G1` print and travel move so the live emit path stops producing G-code whose only F value is the retract speed (`F25`). The Benchy reference output today carries no F on its 126,251 print moves; after this packet, every print and travel move either declares its own F or is preceded within the same role by an F directive.

## Problem Statement

The live G-code emit path in `crates/slicer-host/src/gcode_emit.rs` constructs every print-move and z-hop `GCodeCommand::Move` with `f: None`. Verified at file:line:

- `gcode_emit.rs:218-228` — print-move builder (comment: "Feed rate could be calculated, but tests don't require it").
- `gcode_emit.rs:282` — z-hop up builder.
- `gcode_emit.rs:309` — z-hop down builder.

The serializer (`gcode_emit.rs:424-426`) writes an `F` token only when the field is `Some`, so the produced G-code carries an F-token only on travel moves that received an upstream `Some(...)` and on retract/unretract (`F25`). For Benchy this collapses to a single distinct F value (`F25`) — leaving firmware speed undefined for every print move.

Two upstream sources of speed exist but are unused at emit time:

- `ExtrusionPath3D.speed_factor: f32` at `crates/slicer-ir/src/slice_ir.rs:1297` — set by the layer/region executor (`layer_executor.rs:607` `drain_region_to_print_entities`) but never read.
- No per-role speed keys are registered in `crates/slicer-host/src/config_schema.rs` today (`config_schema.rs:104-176` defines only the `ConfigValue` enum and generic validation; no `outer_wall_speed`-style keys).

This packet closes the gap end-to-end: register the speed keys, resolve them in the emit builder using `(role, speed_factor)`, and serialize the F token on every print and travel move.

This packet does not reopen any prior packet. It is the first remediation against DEV-009 ("Benchy Phase H output is only partially correct on the live path") for the speed-token subset.

## Architecture Constraints

- IR contract in `docs/02_ir_schemas.md` is unchanged. `GCodeCommand::Move.f: Option<f32>` already exists; this packet only changes the producers that pass `None`.
- The serializer remains stateless w.r.t. config. The config is consulted inside the BUILDER (the function in `gcode_emit.rs` that walks the `LayerCollectionIR` and produces `GCodeCommand`s), not inside the serializer's match arms.
- The unit convention from `docs/08_coordinate_system.md` applies: internal coordinates are 10⁻⁴ mm, but feedrate is mm/min in G-code output. The conversion is `mm/s * 60 → mm/min`, rounded to integer (OrcaSlicer parity).
- No module-side WIT change. `ExtrusionPath3D.speed_factor` is already on the IR boundary.

## Data and Contract Notes

- IR or manifest contracts touched: none. `GCodeCommand::Move.f` already `Option<f32>`; `ExtrusionPath3D.speed_factor` already exists.
- WIT boundary considerations: none. No host↔module ABI change.
- Determinism or scheduler constraints: `resolve_feedrate` is pure (config + role + factor → f32). Output remains deterministic.
- Unit conversion contract: `f_token_value = round(speed_mm_per_s * 60.0 * speed_factor * 1000.0) / 1000.0` preserving up to 3 decimal places (OrcaSlicer parity). For integer mm/s values * 60, the result is always an integer. `speed_factor` clamped to `[0.05, 5.0]` to avoid pathological values (clamp threshold confirmed against OrcaSlicer in Step 1's FACT dispatch).

## Locked Assumptions and Invariants

- `ExtrusionRole` enum variants in slicer-ir (OuterWall, InnerWall, ThinWall, TopSolidInfill, BottomSolidInfill, SparseInfill, BridgeInfill, SupportMaterial, SupportInterface, Ironing, Skirt, WipeTower, PrimeTower, Custom(String)) map onto the twenty registered config keys via an explicit `match` covering all variants. First-layer roles use initial_layer_speed/initial_layer_infill_speed variants. Percentage-based speeds (thin_wall, internal_bridge, initial_layer_travel, etc.) have pre-resolved absolute mm/s defaults since the frontend handles percentage derivation before passing values to the CLI/config layer.
- Discovery: DefaultGCodeEmitter::emit_gcode(&self, layer_irs: &[LayerCollectionIR], _blackboard: &Blackboard) -> Result<GCodeIR, PostpassError> receives no config parameter. Blackboard holds IR data only. DefaultGCodeEmitter holds only slicer_version: String. Config must be threaded via the DefaultGCodeEmitter constructor (add a FeedrateConfig struct) or as a new parameter. The design approach is: store a FeedrateConfig on DefaultGCodeEmitter, so resolve_feedrate accesses self.feedrate_config. The GCodeEmitter trait signature stays unchanged.
- OrcaSlicer default speed values (expanded, mm/s): outer_wall_speed=60, inner_wall_speed=60, thin_wall_speed=30, top_surface_speed=100, bottom_surface_speed=100, sparse_infill_speed=100, bridge_speed=25, internal_bridge_speed=37.5, support_speed=80, support_interface_speed=80, gap_infill_speed=30, ironing_speed=20, skirt_speed=50, wipe_tower_speed=90, prime_tower_speed=90, travel_speed=120, travel_speed_z=0, initial_layer_speed=30, initial_layer_infill_speed=60, initial_layer_travel_speed=120, wipe_speed=96, overhang_1_4_speed=0, overhang_2_4_speed=0, overhang_3_4_speed=0, overhang_4_4_speed=0, filament_ironing_speed=0. Percentage-based defaults are pre-resolved to absolute mm/s values (frontend handles derivation). Overhang defaults = 0 (disabled; use role speed). filament_ironing_speed = per-tool modifier defaulting to 0 (use global ironing_speed).
- OrcaSlicer rounding rule for F-token: F = round(speed_mm_per_s * 60 * 1000) / 1000, which preserves up to 3 decimal digits. For integer mm/s values * 60, the result is always an integer (no decimal places needed).
- Config threading: Postpass emitter path has no config access. Modules receive Arc<ConfigView> via dispatch. The approach for this packet: add a FeedrateConfig struct to gcode_emit.rs, store it on DefaultGCodeEmitter, and have emit_gcode access it via self.feedrate_config. At the postpass call site (postpass.rs:180), build FeedrateConfig from the effective config defaults.
- The first layer is detected by `Move.z` matching `layer_height` ± `epsilon`, OR via an upstream `is_first_layer: bool` already present on the layer — discovery Step 1 will confirm which signal exists. The packet's tests assert this by setting up an explicit two-layer fixture.
- `f: Some(...)` from an upstream module always wins. The builder never overwrites a module-set F.

## Risks and Tradeoffs

- Risk: the twenty-six config keys may already exist under different names elsewhere in the workspace (e.g. inside a module manifest). Mitigated by Step 0's verification dispatch.
- Risk: rounding parity with OrcaSlicer — Orca rounds with `round_to_int(f)` not `floor`. Mitigated by the FACT dispatch in Step 1 that records the exact rule.
- Tradeoff: builder-side resolution couples `gcode_emit.rs` to `config_schema.rs` more tightly than the serializer-side alternative. Accepted because the cooling packet (53) wants a clean post-pass slot, which only works if the builder has already populated F-tokens.
