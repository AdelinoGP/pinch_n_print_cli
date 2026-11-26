# Task Map: wipe-tower-geometry-keys

**This packet emits the template's skip clause:** it is a single-coherent-slice packet with `task_ids: []` (queue precedent — packets 234a, 253–264), so the `docs/07_implementation_status.md` crosswalk is N-A. Implementation is recorded against wayfinder ticket 10 (`docs/specs/orca-feature-gap/issues/10-author-packet-p03-multimaterial-prime-tower-wipe-tower.md`). Re-derive the absence of a TASK row at completion time rather than trusting this sentence.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| N-A | Step 1 | `docs/03_wit_and_manifest.md` | `modules/core-modules/wipe-tower/{wipe-tower.toml, Cargo.toml, tests/wipe_tower_config_schema_tdd.rs}` | `PrintConfig.cpp::PrintConfigDef`, `PrintConfig.hpp::WipeTowerWallType` | `S` | extends `254a`'s schema test if that packet landed first |
| N-A | Step 2 | `docs/08_coordinate_system.md` | `modules/core-modules/wipe-tower/{src/lib.rs, tests/wipe_tower_wall_tdd.rs}` | `WipeTower2.cpp::generate_support_cone_wall`, `::generate_rib_polygon`, `::generate_support_rib_wall`, file-static `rounding_polygon` | `M` | de-scale canonical's `scaled()` arithmetic to mm |
| N-A | Step 3 | — | `modules/core-modules/wipe-tower/{src/lib.rs, tests/wipe_tower_tdd.rs}` | `WipeTower2.cpp::toolchange_Wipe`, `::set_toolchange` | `S` | purge volume must stay invariant |
| N-A | Step 4 | `docs/03_wit_and_manifest.md` | `modules/core-modules/wipe-tower/{src/lib.rs, tests/bed_bounds_tdd.rs, tests/finalization_live_tdd.rs}` | `GCode.cpp::WipeTowerIntegration::transform_wt_pt`, `Print.cpp::first_layer_wipe_tower_corners` | `M` | rotation about the tower origin, not its centre |
| N-A | Step 5 | `docs/15_config_keys_reference.md` | `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs`, `crates/slicer-runtime/tests/contract/config_view_binding_tdd.rs`, generated `docs/15` | — | `M` | scheduler binary is `scheduler_integration` |

Aggregate: `L` (2 × M + 2 × M + 1 × S … re-derive from `implementation-plan.md` at review time). No single step is L; split Step 2 at the cone/rib boundary before escalating any context band.

## Supersession

This packet directory replaces its own prior revision in place (same number, same slug), authored before the map's Authoring rules 1–6. No other packet directory is modified. `254a-prime-tower-geometry-keys` and `254b-prime-tower-interface-and-ramming` share this packet's owner and land before it; neither is superseded.
