# Task Map: 241-support-agg-rasterizer

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-419` | `Step 1` | `docs/specs/support-families-anchored-entities-plan.md` §7 | `crates/slicer-runtime/tests/integration/support_agg_rasterizer_tdd.rs` (new submodule) + `main.rs` `mod` line, `crates/slicer-runtime/tests/fixtures/golden/` (dir does not exist — create it) + `p241_baseline.json` | — | S | Pre-port measurement baseline; nothing else may precede it |
| `TASK-420` | `Step 2` | plan §3 Ruling 7 | none (design.md note only) | `SupportMaterial.cpp` class `SupportGridPattern` (delegated) | S | Read-only canonical fidelity probe before coding — NOT the port; the port is Steps 3-4 |
| `TASK-421` | `Step 3` | `docs/08_coordinate_system.md` (constraint) | `modules/core-modules/traditional-support-planner/src/agg_raster.rs`, crate Cargo.toml, `tests/agg_rasterizer_tdd.rs` | constructor + statics (Step-2 return) | M | Grid construction; AC-2 |
| `TASK-422` | `Step 4` | — | `agg_raster.rs`, `tests/agg_rasterizer_tdd.rs` | `extract_support` / `contours_simplified` / `seed_fill_block` | M | Seed fill + extraction + island filter; AC-3/AC-4 |
| `TASK-423` | `Step 5` | `docs/03_wit_and_manifest.md` §Config Field Types Reference (enum row) | manifest knob, `lib.rs` `from_config` parse block, `docs/15_config_keys_reference.md` | — | S | Knob declaration + module-side rejection (defense-in-depth; the host `ConfigBoundsIndex` already rejects bad enum values first); AC-1/AC-N1 |
| `TASK-424` | `Step 6` | plan §3 Ruling 8 | `lib.rs` propagation loop branch, `agg_raster.rs` glue, `tests/agg_rasterizer_tdd.rs` routing test | instantiation site (canonical `generate_support_layers` region) | M | agg default, legacy selectable; AC-5 |
| `TASK-424` | `Step 6b` | plan §3 Ruling 8 | `tests/traditional_family_tdd.rs` assertion re-baselining only | — | S | Legacy-guard reconciliation under the agg default; AC-N2. Split from Step 6 to hold the 3-file edit cap |
| `TASK-425` | `Step 7` | plan §7 E1/E2 | integration measurement tests | `fb7b995050` / `a95607d7bf` symptoms (plan-cited) | M | Measurement gate; AC-6/AC-7/AC-8 |
| `TASK-426` | `Step 8` | plan §13 T7 | wedge proof + measured hint update | — | S | Real-mesh validation |
| `TASK-427` | `Step 9` | plan §8 | `docs/07_implementation_status.md` rows, doc greps | — | S | Closure gates |
| `TASK-428` | `Step 9` | plan §8 | recorded-metrics appendix in requirements.md | — | S | Registration + human-gate readiness |

Copy costs from `implementation-plan.md`. Split before activation if any row is L or aggregate
exceeds M.
