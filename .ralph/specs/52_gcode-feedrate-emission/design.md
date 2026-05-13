# Design: 52_gcode-feedrate-emission

## Controlling Code Paths

- Primary code path:
  - `crates/slicer-host/src/gcode_emit.rs:218-229` — print-move builder, currently sets `f: None`.
  - `crates/slicer-host/src/gcode_emit.rs:282` — z-hop up builder, currently `f: None`.
  - `crates/slicer-host/src/gcode_emit.rs:309` — z-hop down builder, currently `f: None`.
  - `crates/slicer-host/src/gcode_emit.rs:293-299` — travel-move builder, ALREADY propagates upstream `tm.f`. This is the reference for what the print-move builder should look like.
  - `crates/slicer-host/src/gcode_emit.rs:424-426` — serializer F-token write site. Unchanged in this packet.
  - `crates/slicer-host/src/config_schema.rs:104-176` — `ConfigValue` enum + validation API. Twenty-six new keys added.
- Neighboring tests or fixtures:
  - `crates/slicer-host/tests/gcode_emit_tdd.rs` — regression; this packet must not break the `;TYPE:` / OrcaSlicer-comment contract (the `emits_orca_*` tests).
  - `crates/slicer-host/tests/benchy_painted_overrides_e2e_tdd.rs` — exercises the live pipeline against the Benchy fixture; expected to gain F-token assertions in a follow-up but not modified here.
- OrcaSlicer comparison surface:
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — per-role lookup pattern.
  - `OrcaSlicerDocumented/src/libslic3r/GCodeWriter.cpp` — mm/s → mm/min conversion rule.

## Architecture Constraints

- IR contract in `docs/02_ir_schemas.md` is unchanged. `GCodeCommand::Move.f: Option<f32>` already exists; this packet only changes the producers that pass `None`.
- The serializer remains stateless w.r.t. config. The config is consulted inside the BUILDER (the function in `gcode_emit.rs` that walks the `LayerCollectionIR` and produces `GCodeCommand`s), not inside the serializer's match arms.
- The unit convention from `docs/08_coordinate_system.md` applies: internal coordinates are 10⁻⁴ mm, but feedrate is mm/min in G-code output. The conversion is `mm/s * 60 → mm/min`, rounded to integer (OrcaSlicer parity).
- No module-side WIT change. `ExtrusionPath3D.speed_factor` is already on the IR boundary.

## Code Change Surface

- Selected approach: **builder-side resolution.** Add `pub fn resolve_feedrate(&self, role: &ExtrusionRole, speed_factor: f32) -> Option<f32>` as a method on `DefaultGCodeEmitter` in `gcode_emit.rs`. Visibility is `pub` (not private) so the packet's integration tests can probe the resolver directly without round-tripping through `emit_gcode`; this is a deliberate test-affordance deviation from an earlier "private helper" draft. The role-to-key match covers all 13 named ExtrusionRole variants plus Custom(String). First-layer detection switches outer/inner/sparse to initial_layer_speed/initial_layer_infill_speed variants. Call it from the print-move builder and the two z-hop builders. The travel-move builder continues to use the upstream `tm.f` when set; when `tm.f` is `None`, fall back to `resolve_feedrate(&ExtrusionRole::Custom("Travel"), 1.0)`.
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - `crates/slicer-host/src/gcode_emit.rs`:
    - Add `fn resolve_feedrate(role, speed_factor, config) -> Option<f32>` (private, method on DefaultGCodeEmitter using self.feedrate_config).
    - Change three `f: None` literals to `f: resolve_feedrate(&role, speed_factor, &config)` where `speed_factor` is the `ExtrusionPath3D.speed_factor` for print moves and `1.0` for z-hops.
    - Add `FeedrateConfig` struct to `gcode_emit.rs` holding all 26 speed values. Store on `DefaultGCodeEmitter`.
  - `crates/slicer-host/src/config_schema.rs`:
    - Register twenty-six `ConfigField`s: outer_wall_speed, inner_wall_speed, thin_wall_speed, top_surface_speed, bottom_surface_speed, sparse_infill_speed, bridge_speed, internal_bridge_speed, support_speed, support_interface_speed, gap_infill_speed, ironing_speed, skirt_speed, wipe_tower_speed, prime_tower_speed, travel_speed, travel_speed_z, initial_layer_speed, initial_layer_infill_speed, initial_layer_travel_speed, wipe_speed, overhang_1_4_speed, overhang_2_4_speed, overhang_3_4_speed, overhang_4_4_speed, filament_ironing_speed. Each `ConfigValue::Float` (mm/s). Defaults: outer_wall_speed=60.0, inner_wall_speed=60.0, thin_wall_speed=30.0, top_surface_speed=100.0, bottom_surface_speed=100.0, sparse_infill_speed=100.0, bridge_speed=25.0, internal_bridge_speed=37.5, support_speed=80.0, support_interface_speed=80.0, gap_infill_speed=30.0, ironing_speed=20.0, skirt_speed=50.0, wipe_tower_speed=90.0, prime_tower_speed=90.0, travel_speed=120.0, travel_speed_z=0.0, initial_layer_speed=30.0, initial_layer_infill_speed=60.0, initial_layer_travel_speed=120.0, wipe_speed=96.0, overhang_1_4_speed=0.0, overhang_2_4_speed=0.0, overhang_3_4_speed=0.0, overhang_4_4_speed=0.0, filament_ironing_speed=0.0. Percentage-based defaults are pre-resolved to absolute mm/s (frontend handles derivation). Overhang defaults = 0 = disabled (use role speed). filament_ironing_speed = per-tool modifier defaulting to 0 (use global ironing_speed).
  - `crates/slicer-host/tests/gcode_feedrate_emission_tdd.rs` — NEW; ≥ 12 tests (one per AC + each negative case + overhang/filament/wipe tests), covering all ExtrusionRole variants.
- Rejected alternatives:
  - **Resolve speed inside the serializer.** Rejected: the serializer would have to consume config, breaking its current stateless shape and complicating the future cooling post-pass (packet 53) that wants to mutate F values after the builder has run.
  - **Annotate `GCodeCommand::Move` with `role_speed_default: Option<f32>` and resolve in a post-pass.** Rejected: introduces IR churn and creates two sources of truth (annotation vs config); contradicts the architecture constraint that IR is unchanged.
  - **Compute speed inside `layer_executor.rs:607` and write into `ExtrusionPath3D.speed_factor`.** Rejected: `speed_factor` is documented as a multiplier, not an absolute value; conflating the two would break module-side speed scaling.

## Files in Scope (read + edit)

- `crates/slicer-host/src/gcode_emit.rs` — primary edit; range-read `:200-:320` and `:380-:480` only; expected change: add `resolve_feedrate` helper, wire three call sites.
- `crates/slicer-host/src/config_schema.rs` — primary edit; load directly (small, < 300 lines per reconnaissance); expected change: register twenty-six speed keys with defaults.
- `crates/slicer-host/tests/gcode_feedrate_emission_tdd.rs` — NEW; sole new file in this packet.

## Read-Only Context

- `crates/slicer-ir/src/slice_ir.rs` — read lines `1280-1330` (ExtrusionPath3D) and `1460-1530` (TravelMove + LayerCollectionIR). Purpose: confirm `speed_factor` semantics and `TravelMove.f` shape.
- `crates/slicer-host/src/layer_executor.rs` — read lines around `:600-:640` only (the `drain_region_to_print_entities` function). Purpose: confirm where `speed_factor` is set so tests can stub it.
- `docs/02_ir_schemas.md` — delegate a SUMMARY for the `GCodeCommand` and `ExtrusionPath3D` sections.
- `docs/08_coordinate_system.md` — load directly (< 200 lines); confirm mm/min convention.
- `docs/DEVIATION_LOG.md` — load directly; locate DEV-009 to append a remediation note.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` — delegate every read; never load.
- `target/`, `Cargo.lock`, generated WIT bindings under `crates/*/wit/bindings*` — never load.
- `crates/slicer-host/src/dispatch.rs` — out of scope for this packet; delegate any symbol lookup.
- `crates/slicer-host/src/pipeline.rs` — out of scope (not needed for this slice; `run_pipeline_with_raw_config` is the touchpoint for packets 53 and 54, not 52).
- `modules/core-modules/skirt-brim/`, `modules/core-modules/wipe-tower/`, etc. — unrelated; do not browse.
- Full `docs/07_implementation_status.md` — delegate the TASK-153 insertion via worker dispatch.

## Expected Sub-Agent Dispatches

- "Return verbatim the OrcaSlicer default values (mm/s) for `outer_wall_speed`, `inner_wall_speed`, `sparse_infill_speed`, `internal_solid_infill_speed`, `top_surface_speed`, `travel_speed`, `initial_layer_speed`, `initial_layer_travel_speed`. Scope: `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp`. Return: FACT, one row per key in format `key = <number> mm/s`, ≤ 12 lines."
- "Run `cargo test -p slicer-host --test gcode_feedrate_emission_tdd`; return FACT (pass) or SNIPPETS (failing assertion + ≤ 20 lines per test)." — purpose: validate every step.
- "Show the existing config-resolution call sites that pass a `ConfigView` / `ResolvedConfig` into the gcode-emit builder; return LOCATIONS ≤ 5." — purpose: confirm the parameter-threading change in Step 2.
- "Append a new TASK-153 row under Phase H in `docs/07_implementation_status.md` (title: 'Per-role feedrate emission on live G-code path') and append a DEV-009 remediation entry to `docs/DEVIATION_LOG.md`; return EDITED/NOT-EDITED." — purpose: backlog hygiene in Step 5.
- "Run `cargo test -p slicer-host --test gcode_emit_tdd`; return FACT pass/fail." — purpose: regression check after the F-token wiring lands.

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

## Context Cost Estimate

- Aggregate: M.
- Largest single step: Step 2 (resolver + wiring) — M.
- Highest-risk dispatch: OrcaSlicer default-table lookup; required return format = FACT, ≤ 16 lines, one key per row, mm/s only.

## Open Questions

None. Step 1 discovery confirmed: (a) config must be threaded through DefaultGCodeEmitter (no existing config in emit path), (b) the twenty-six OrcaSlicer default values are locked (percentage-based ones pre-resolved to absolute mm/s), (c) the rounding rule preserves up to 3 decimal digits but produces integers for whole-number mm/s inputs, (d) all 13 ExtrusionRole variants + travel are mapped to config keys.
