# Task Map: 241-support-agg-rasterizer

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-419` | `Step 1` | `docs/specs/support-families-anchored-entities-plan.md` §7 | `crates/slicer-runtime/tests/integration/support_agg_rasterizer_tdd.rs` (new), golden baseline JSON | — | S | Pre-port measurement baseline; nothing else may precede it |
| `TASK-420` | `Step 2` | plan §3 Ruling 7 | none (design.md note only) | `SupportMaterial.cpp` `SupportGridPattern` (delegated) | S | Canonical fidelity probe before coding |
| `TASK-421` | `Step 3` | `docs/08_coordinate_system.md` (constraint) | `modules/core-modules/traditional-support-planner/src/agg_raster.rs`, crate Cargo.toml, `tests/agg_rasterizer_tdd.rs` | constructor + statics (Step-2 return) | M | Grid construction; AC-2 |
| `TASK-422` | `Step 4` | — | `agg_raster.rs`, `tests/agg_rasterizer_tdd.rs` | `extract_support` / `contours_simplified` / `seed_fill_block` | M | Seed fill + extraction + island filter; AC-3/AC-4 |
| `TASK-423` | `Step 5` | `docs/03_wit_and_manifest.md` §Config Field Types | manifest knob, `lib.rs` parse block, `docs/15_config_keys_reference.md` | — | S | Knob declaration + rejection; AC-1/AC-N1 |
| `TASK-424` | `Step 6` | plan §3 Ruling 8 | `lib.rs` propagation loop branch | instantiation site (~`generate_support_layers`) | M | agg default, legacy selectable; AC-5/AC-N2 |
| `TASK-425` | `Step 7` | plan §7 E1/E2 | integration measurement tests | `fb7b995050` / `a95607d7bf` symptoms (plan-cited) | M | Measurement gate; AC-6/AC-7/AC-8 |
| `TASK-426` | `Step 8` | plan §13 T7 | wedge proof + measured hint update | — | S | Real-mesh validation |
| `TASK-427` | `Step 9` | plan §8 | `docs/07_implementation_status.md` rows, doc greps | — | S | Closure gates |
| `TASK-428` | `Step 9` | plan §8 | recorded-metrics appendix in requirements.md | — | S | Registration + human-gate readiness |

Copy costs from `implementation-plan.md`. Split before activation if any row is L or aggregate
exceeds M.
