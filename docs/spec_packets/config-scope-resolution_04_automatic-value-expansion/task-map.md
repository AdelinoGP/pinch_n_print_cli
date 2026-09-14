# Task Map: automatic-value-expansion

This explicit single-task crosswalk is required by the batch queue and preflight structure gate.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-565` | Step 1 (FORWARD-DEP/base census) | approved plan §Expansion; ADR-0068 amendment | packet-1 export verification; percent/base read inventory | Delegated only | `S` | Prevents fictional producer names and packet-10 scope absorption. |
| `TASK-565` | Step 2 (expansion engine) | approved plan §Expansion; ADR-0067/0068; `docs/22_test_quality.md` | `crates/slicer-config/src/lib.rs`; `crates/slicer-config/tests/automatic_value_expansion_tdd.rs` | `Flow.cpp::new_from_config_width` / `auto_extrusion_width`; `PrintConfig.cpp::PrintConfigDef::init_fff_params`; `GCode.cpp::GCode::_extrude`; `SupportParameters.hpp::number_of_support_interface_bottom_layers` | `M` | Exports `ExpansionContext`, `ExpansionError`, and `expand_automatic_values`; independent literal oracle includes all four overhang percentages over `outer_wall_speed`. |
| `TASK-565` | Step 3 (typed bases) | approved plan Registry/Expansion decisions and RC-8 | support declaration; overhang-classifier manifest; `SPEED_META`; registry tests | `PrintConfig.cpp::PrintConfigDef::init_fff_params`; `GCode.cpp::GCode::_extrude` (zero/volumetric boundary only) | `S` | Adds support/nozzle and four overhang/outer-wall bases with host/module type agreement; numeric overhang zero remains zero. |
| `TASK-565` | Step 4 (runtime/prepass placement) | ADR-0068; `docs/02_ir_schemas.md` interner contract | `crates/slicer-runtime/{Cargo.toml,src/run.rs,src/pipeline.rs,src/prepass.rs}`; registered integration regression | None | `M` | Expands global/object before binding, the emitter tool map before use, and prepass-rebuilt tool/paint maps before RegionMapping can intern them in both production entry points. |
| `TASK-565` | Step 5 (support width) | approved plan RC-8/Phase B | `slicer-ir` resolved config; runtime support consumers | `Flow.cpp::auto_extrusion_width` | `S` | Removes the duplicate support-width implementation without absorbing Phase-C speed handling. |
| `TASK-565` | Step 6 (role dispatch) | approved plan Phase B/C boundary | `crates/slicer-core/src/flow.rs`; role context call sites/tests | `Flow.cpp::new_from_config_width` | `S` | Preserves precedence while moving nozzle auto into Phase B. |
| `TASK-565` | Step 7 (guest mirror retirement) | ADR-0068 no-placeholder invariant | four support guest source/test trees | `SupportParameters.hpp::number_of_support_interface_bottom_layers` | `M` | Leaves one owner for the two config-only `-1` rules. |
| `TASK-565` | Step 8 (visual/generator/docs) | `docs/19_visual_debug.md`; `docs/02_ir_schemas.md`; `docs/11_operational_governance_and_acceptance_gate.md` | deterministic request/config; `xtask/src/gen_config_docs.rs::KeyRow`/`module_rows`/`render_table`; docs 02/15 | None | `S` | Required visual evidence, generated `Base key` column locked by `render_table_preserves_base_key`, and no-bump compatibility record. |

## Exports to Dependent Queue Rows

| Export | Crate/module | Shape | Consumer |
| --- | --- | --- | --- |
| `ExpansionContext` | `slicer_config` | `pub struct { nozzle_diameter_mm: f64, tool_bases: BTreeMap<u32, BTreeMap<String, f64>> }` | packet 5; packet 10 only for its remaining volumetric/context-dependent rules |
| `ExpansionError` | `slicer_config` | public enum with `UnknownBaseKey`, `MissingAutoBase`, `NonPositiveBase` named-field variants | packet 5/runtime error mapping |
| `expand_automatic_values` | `slicer_config` | `fn(&ConfigSchemaRegistry, &mut ResolvedConfig, &ExpansionContext, Option<u32>) -> Result<(), ExpansionError>` | packet 5 scope resolver before interning; includes all four overhang-speed percentages, so packet 10 must not re-own them |

This map owns only approved queue row 4 / `TASK-565`, including the Phase-B overhang-speed percent family. It does not close packet 5's scope resolver or packet 10's emitter volumetric/geometry-dependent automatic values.
