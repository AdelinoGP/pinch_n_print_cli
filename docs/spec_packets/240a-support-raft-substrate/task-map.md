# Task Map: 240-support-raft

Crosswalk for the TASK-409..TASK-418 allocation (queue row #7 of
`docs/specs/support-families-anchored-entities-plan.md`). IDs are exclusive to
this packet; packets 238c owns TASK-381..398, row #6 owns 399..408, rows 8+
start at 419.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-409` | `Step 1` | `docs/specs/support-families-anchored-entities-plan.md` §12 | `crates/slicer-ir/tests/{signed_layer_indices_tdd,sliced_region_raft_fill_tdd}.rs` | none | S | Red-first IR contract |
| `TASK-410` | `Step 2` | `docs/02_ir_schemas.md` | `crates/slicer-ir/src/slice_ir.rs`, `crates/slicer-sdk/src/traits.rs`, `crates/slicer-macros/src/lib.rs` + sweep fallout | none | M | u32→i32 migration; split trigger if >~20 files |
| `TASK-411` | `Step 3` | `docs/02_ir_schemas.md` | `slice_ir.rs`, `crates/slicer-schema/wit/deps/ir-types.wit`, marshal legs | none | M | raft_fill carrier + accessor + schema bump |
| `TASK-412` | `Step 4` | `docs/03_wit_and_manifest.md`, ADR-0009 | `modules/core-modules/raft-default/*` (manifest+Cargo+wit-guest) | none | M | new guest; rebuild in-step |
| `TASK-413` | `Step 4` | same | same | none | M | claim:raft-fill single holder resolves |
| `TASK-414` | `Step 5` | `docs/08_coordinate_system.md` | `raft-default/src/lib.rs`, runtime integration cases | `SupportCommon.cpp::generate_raft_base` (delegated) | M | geometry port; determinism |
| `TASK-415` | `Step 5` | same | same | `SupportCommon.cpp::generate_support_layers` (analogue only) | M | negative-prefix ordering; zero anchored entities |
| `TASK-416` | `Step 6a` | `docs/15_config_keys_reference.md` | four support manifests, `validation_tdd.rs`, regenerated config doc | `PrintConfig.cpp init_fff_params` (defaults FACT) | M | wire-or-record + claim-conflict negative |
| `TASK-417` | `Step 6b` | same + §13 traps | `contract/raft_bounds_tdd.rs`, `contract/main.rs` (registration) | none | S | AC-N2/AC-5 green; mod registered |
| `TASK-418` | `Steps 7+8` | ADR-0009 (formal amendment), `docs/02_ir_schemas.md`, `docs/19_visual_debug.md` | docs edits; tmp/p240-* artifacts | references comparison | S | DEV-124 record; ADR Decision-5 claim reassignment amendment; human gate |

Copy costs from `implementation-plan.md`. Aggregate is M; no row is L.
