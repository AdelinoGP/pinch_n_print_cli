# Task Map: 238a-support-pattern-config-keys

Crosswalk of `docs/07_implementation_status.md` registration rows to packet steps.
Registration itself is DEFERRED to the packet-owned closure step (Step 8, TASK-368): the
rows below are what that step writes, verbatim in intent.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-363` | `Step 1` | `docs/specs/support-parity-gap-register.md` (G-16) | `crates/slicer-runtime/tests/executor/` (red tests) | n/a (no canonical read) | `S` | Red-first guest-dispatch proof for undeclared keys |
| `TASK-364` | `Step 2` | gap register G-16; plan §13 T8 | `modules/core-modules/tree-support-planner/tree-support-planner.toml` + regen | `PrintConfig.cpp` (`max_bridge_length`, style enums) | `S` | Four declarations; blast radius pre-baked by dispatch |
| `TASK-365` | `Step 3` | gap register G-03 | `modules/core-modules/traditional-support-planner/traditional-support-planner.toml` + regen | `PrintConfig.cpp` (`init_fff_params`, `s_keys_map_SupportMaterialPattern`) | `S` | Spacing declaration + pattern value-set doc |
| `TASK-366` | `Step 4` | gap register G-04/G-05/G-08 (host half) | `crates/slicer-ir/src/resolved_config.rs`, `docs/config/host-keys.toml` | `PrintConfig.cpp` (`init_fff_params`), `Flow.cpp` | `M` | Eleven typed host keys incl. line-width retype |
| `TASK-367` | `Step 5a` | gap register G-05 (consumer) | `crates/slicer-core/src/algos/support_geometry.rs`, `crates/slicer-runtime/src/builtins/support_geometry_producer.rs` (test target: Step 5a-2) | `PrintConfig.cpp` (`init_fff_params`) | `S` | De-hardcode distances; AC-4 target `support_geometry_config_surface_tdd` authored in Step 5a-2 |
| `TASK-367` (cont.) | `Step 5b` | gap register G-08 (consumer) | `crates/slicer-gcode/src/serialize.rs` | `Flow.cpp` (`auto_extrusion_width`, `opt_key_to_flow_role`) | `S` | Serializer width sourcing; dead literals removed |
| `TASK-367` (cont.) | `Steps 6 / 6-2` | gap register G-09 | `crates/slicer-wasm-host/src/marshal/in_.rs`, `.../native.rs`, `.../mod.rs` (Step 6); new contract test + `contract/main.rs` registration (Step 6-2) | n/a | `S` | One shared MAX-rule helper, both legs; proof test in 6-2 |
| `TASK-367` (cont.) | `Step 7` | gap register G-09/G-16 verification | bounds negatives in `scheduler_integration` | n/a | `S` | OutOfRange ACs + invariant-16 regression net (AC-5 proof lives in Step 6-2) |
| `TASK-368` | `Step 8` | plan §14 rule 7 (doc hygiene) | `docs/07_implementation_status.md` rows above | n/a | `S` | Registration via worker dispatch, never full backlog read |

Copy costs from `implementation-plan.md`. No row is L; aggregate M. Split before activation
if any row grows to L.
