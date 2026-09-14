# Task Map: layer-range-scope

This single-task crosswalk is emitted because TASK-570 spans three forward-dependency interfaces, a canonical attribution correction, a binary fixture, and two production entry paths; it prevents those ownership edges from disappearing across implementation steps.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-570` | `Step 1` | plan queue row 9; predecessor packet contracts | landed packet-03/05/07 exports and caller/literal inventories | none | `S` | Reconciles all FORWARD-DEPs before edits. |
| `TASK-570` | `Steps 2a–2b` | coordinate and test-quality docs | deterministic fixture; model-IO parser/tests | `bbs_3mf.cpp` importer/exporter functions | `M` | Grounds exact XML and one-based object ordinal linkage without exceeding three edits per step. |
| `TASK-570` | `Step 3` | ADR-0068; ADR-0069 | typed `ConfigScope::LayerRange`, carrier, admission/conflict validation | none | `M` | Owns the net-new packet-03 variant and packet-07 load rejection. |
| `TASK-570` | `Step 4` | plan Resolution/Layer range; test quality | packet-05 `query_z_grid`/`resolve_scope_stack` and focused tests | `Slicing.cpp::layer_height_profile_from_ranges` | `M` | Implements approved earlier-starting trim/gap correction and catch-up top-Z inheritance. |
| `TASK-570` | `Step 5` | scheduler architecture | pnp-cli ordinary/visual adapters and runtime carrier | none | `M` | Wires one typed set to both production setup paths. |
| `TASK-570` | `Step 6` | visual-debug; test quality | runtime integration and pnp-cli visual-debug tests | none | `M` | Proves fixture behavior through both real paths and manifest output. |
| `TASK-570` | `Step 7` | IR schemas; scheduler architecture | docs and delegated gates | reuse bounded evidence | `S` | Closes docs/quality without editing the source plan or predecessor packets. |
