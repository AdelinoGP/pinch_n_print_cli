---
status: implemented
packet: 53_gcode-cooling-fan-emission
task_ids:
  - TASK-154
  - TASK-152d
---

# 53_gcode-cooling-fan-emission

## Goal

Emit `M106 S<n>` and `M107` part-cooling fan commands on the live G-code emit path according to a cooling profile, by introducing a new finalization-stage WASM module (`cooling`) that consumes the finalized `LayerCollectionIR` and inserts `GCodeCommand::FanSpeed` entries on layer boundaries. This supersedes the prior decision (TASK-152c, closed 2026-04-29) that placed cooling on the rejected `Layer::PathOptimization` surface — the new surface is the finalization stage, parallel to `SkirtBrim` and `WipeTower`.

## Problem Statement

The live G-code emit path produces zero `M106`/`M107` commands. Verified via reconnaissance:

- `GCodeCommand::FanSpeed { value: u8 }` exists in IR.
- `crates/slicer-host/src/gcode_emit.rs:466` serializes `FanSpeed` to `M106 S<n>` when present.
- The only call sites that CONSTRUCT `FanSpeed` are in test fixtures (`crates/slicer-sdk/src/postpass_builders.rs:141`) and the macro/dispatch plumbing (`crates/slicer-macros/src/lib.rs:826, :2895`). No live pipeline module produces `FanSpeed`.
- `crates/slicer-host/src/config_schema.rs` has no cooling-profile keys.
- `docs/07_implementation_status.md` records **TASK-152c (Closed 2026-04-29)** with rationale: "packet 19 documents fan-speed and cooling overrides as intentionally unsupported on the live `Layer::PathOptimization` surface; rejection wording locked in `docs/05_module_sdk.md` § Layer Stage Module Surface Rejections".

This packet supersedes TASK-152c by introducing a DIFFERENT surface — the finalization stage, parallel to the existing `SkirtBrim` and `WipeTower` finalization modules (`docs/05_module_sdk.md` § "Finalization Stage Module Surface"). The rejection on the path-optimization surface remains in force; what changes is that the finalization surface is now the documented home for cooling.

This packet is the second remediation against DEV-009 (Benchy live output partially correct).

## Architecture Constraints

- The cooling module lives on the **finalization stage**, not the path-optimization stage. This is the deliberate supersession of TASK-152c.
- The module receives a finalized `LayerCollectionIR` and may insert new `PrintEntity`-equivalent events that carry `GCodeCommand::FanSpeed` — exact insertion API mirrors `SkirtBrim`'s `push_entity_to_layer` pattern (per reconnaissance: skirt-brim/src/lib.rs:100 sets `role: path.role.clone()` on entities, so the same `FinalizationOutputBuilder` API is used).
- IR contract is unchanged: `GCodeCommand::FanSpeed` already exists. No new IR variants.
- WIT manifest must declare `cooling` as a `finalization` stage module reading the same `LayerCollectionView` surface that `SkirtBrim` reads, and writing `FanSpeed` events (the WIT capability set covering this is whatever `SkirtBrim` declares — confirmed in Step 1).
- Determinism: cooling decisions depend only on `(layer_index, layer_time, region_role)` → fan-speed. Layer time is currently NOT computed in the live path (would require packet 52's feedrate emission). For this packet, layer-time-based slowdown is deferred; the algorithm reduces to:
  - layer `< disable_fan_first_layers` → emit `M107` (or `M106 S0`) at layer start.
  - layer `>= disable_fan_first_layers` → emit `M106 S<fan_speed_max>` at layer start.
  - Overhang region detected (`PrintEntity` with role tag indicating overhang) → emit `M106 S<scale(overhang_fan_speed, fan_speed_max)>` at region start, restore prior at region end.
  - Last layer end → emit `M107`.
- The cooling rejection snippet in `docs/05_module_sdk.md` § "Layer Stage Module Surface Rejections" is removed entirely — cooling is now supported via the finalization-stage module, making the old "unsupported" wording misleading.

## Data and Contract Notes

- IR contracts touched: none added/changed. `GCodeCommand::FanSpeed { value: u8 }` already exists.
- WIT boundary: the new module's TOML declares `finalization` stage and the same capability set as skirt-brim (or a subset). Confirmed by Step 1 dispatch.
- Determinism: cooling decision is a pure function of `(layer_index, region_role_tags, config)`. Layer time NOT consulted (would require packet 52's per-move time computation; deferred).
- Module loading: dispatcher loads `modules/core-modules/part-cooling/part-cooling.wasm` at startup. Path mirrors `skirt-brim/skirt-brim.wasm`.

## Locked Assumptions and Invariants

- The finalization stage runs after `SkirtBrim` and before `GCodeIR` serialization. The cooling module slot is positioned between them. (Step 1 confirms exact ordering.)
- Overhang detection uses the existing `PrintEntity` role tagging that the bridge-detector packet (36-rev1) installed. If no overhang-tagged entities are present, the overhang branch is a no-op and tests `overhang_fan_bumped` use a hand-built fixture.
- `M106 S0` is treated as equivalent to `M107` for the "fan off" criterion. The module emits `M107` by convention.

## Risks and Tradeoffs

- Risk: WIT capability mismatch — the new module may need to DECLARE a capability that `SkirtBrim` does not. Mitigated by the Step 1 FACT dispatch and a follow-up `cargo build` after the manifest lands.
- Risk: dispatcher invocation order — placing cooling before skirt-brim would let cooling fan-bumps interfere with adhesion. Locked invariant: `[SkirtBrim, WipeTower (if any), Cooling] -> Serialize`. Tests assert this implicitly via the layer-2 fan-on assertion (skirt is layer 0/1 only).
- Risk: layer-time-driven slowdown algorithm is deferred — users with thin layers may see suboptimal cooling. Documented as known limitation; future packet handles it.
- Tradeoff: the new crate adds CI build time. Accepted; `build-core-modules.sh` already compiles four modules.
