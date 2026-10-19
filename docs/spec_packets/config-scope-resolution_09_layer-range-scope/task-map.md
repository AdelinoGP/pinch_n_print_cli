# Task Map: layer-range-scope

This single-task crosswalk is emitted because TASK-570 spans three forward-dependency interfaces, a canonical attribution correction, a binary fixture, two production entry paths, an owner-approved WIT/guest scope expansion (Amendment 1), and a manifest-schema addition; it prevents those ownership edges from disappearing across implementation steps.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-570` | `Step 1` | plan queue row 9; predecessor packet contracts | landed packet-03/05/07 exports and caller/literal inventories | none | `S` | Done: reconciliation and blast-radius inventories recorded in `design.md` §Landing Notes. |
| `TASK-570` | `Steps 2a–2b` | coordinate and test-quality docs | deterministic fixture; model-IO parser, ordinal mapper, tests | `bbs_3mf.cpp` importer/exporter functions | `M` | Grounds exact XML and one-based ordinal linkage without exceeding three edits per step. |
| `TASK-570` | `Step 3` | ADR-0068; ADR-0069 | typed `ConfigScope::LayerRange`, carrier, admission/conflict validation | none | `M` | Owns the net-new packet-03 variant and packet-07 load rejection. |
| `TASK-570` | `Step 4` | plan Resolution/Layer range; test quality | profile/top-Z API, `ResolutionTarget.layer_top_z`, both resolver seams, tests | `Slicing.cpp::layer_height_profile_from_ranges` | `M` | Implements the approved earlier-starting trim/gap correction and catch-up top-Z inheritance. |
| `TASK-570` | `Step 5a` | WIT/schema docs | `prepass-layer-planning@3.0.0`, `layer-zs`, SDK/dispatch/macros/tests | none | `M` | Owner-approved Amendment 1 surface; breaking WIT bump. |
| `TASK-570` | `Step 5b` | WIT/schema docs; layer planner tests | guest variable-step schedule + catch-up, uniform fallback | none | `M` | Guest consumes explicit host schedules. |
| `TASK-570` | `Step 6` | scheduler architecture | runtime carrier, wasm-host ingestion, per-layer region config application | none | `M` | Wires one typed set to both production setup paths and resolves ranges per layer. |
| `TASK-570` | `Step 7` | visual-debug doc | additive `scheduled_layer_zs` manifest field (schema 1.3) | none | `S` | Makes AC-6 observable from a real bundle. |
| `TASK-570` | `Step 8` | visual-debug; test quality | runtime integration and pnp-cli visual-debug tests | none | `M` | Proves fixture behavior through both real paths and manifest output. |
| `TASK-570` | `Step 9` | IR schemas; scheduler; WIT docs | docs and delegated gates | reuse bounded evidence | `S` | Closes docs/quality without editing the source plan or predecessor packets. |
