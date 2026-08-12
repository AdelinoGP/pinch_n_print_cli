# Requirements: tree-support-family

## Packet Metadata
- Grouped task IDs: `TASK-332`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement
The live `support-planner` is tree-specific (`modules/core-modules/support-planner/support-planner.toml:2-17`) but is globally selected even for traditional support. The live tree renderer is `Layer::Support` with flat support output (`modules/core-modules/tree-support/tree-support.toml:9-18`). This packet gives that algorithm an explicit family boundary and makes geometry structural before rendering.

## In Scope
- Rename/split the planner module and claims to `tree-support-planner` with `support-family:tree`.
- Sample distributed corner, contour, and interior contacts from `SupportAnalysisIR` demands.
- Use the host exact-Z support query service exported by TASK-331 at every body Z and tighten the envelope for branch radius, clearance, and routing.
- Emit universal structural `SupportPlanIR` body/interface roles, skeleton metadata, anchored heights, provenance, and decline records.
- Make `tree-support` consume only tree-attributed plan entries and render semantic polygons into structured `SupportIR`.
- Cover config keys `tree_support_branch_angle`, `tree_support_branch_diameter`, `tree_support_branch_diameter_angle`, `tree_support_branch_distance`, `tree_support_wall_count`, `support_interface_top_layers`, `support_interface_bottom_layers`, `support_layer_height_mm`, and `support_top_z_distance_mm`.

## Out of Scope
- Host exact-Z schema ownership and WIT migration decision, TASK-331 blockers.
- Traditional planning, mixed-family routing, final Orca differential closure, and raft scheduling.

## Authoritative References
- `modules/core-modules/support-planner/src/lib.rs` - existing tree algorithm; bounded reads only.
- `modules/core-modules/tree-support/src/lib.rs` and `modules/core-modules/tree-support/tree-support.toml` - current renderer surface.
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp:3388` `TreeSupport::generate_contact_points()` - grid sampling over the bounding box, rather than triangle-centroid-only contacts.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportSpotsGenerator.cpp:1130` `full_search(...)` - distributed support-point generation over overhang areas; stability check at `:845`.
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp:1839` `TreeSupport::get_collision(radius, layer_nr)`, `:1823` `get_avoidance(radius, obj_layer_nr)`, and `:1855` `get_collision_polys(radius, layer_nr)` - radius-aware exact-Z collision and avoidance.
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp:2652` `TreeSupport::drop_nodes()` - builds the tree/MST from contacts downward; `MinimumSpanningTree` is at `:2823-2834`.
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp:1969` `TreeSupport::draw_circles()` - per-layer collision-radius body emission.
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp:2050` area/interface (`roof_areas`) emission inside `draw_circles`; `:1772`/`:1792` `calc_branch_radius(...)` taper; `:2143` top-interface/termination on model/plate.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp:3388` `TreeSupport::generate_contact_points()` - grid sampling over the bounding box, rather than triangle-centroid-only contacts.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportSpotsGenerator.cpp:1130` `full_search(...)` - distributed support-point generation over overhang areas; stability check at `:845`.
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp:1839` `TreeSupport::get_collision(radius, layer_nr)`, `:1823` `get_avoidance(radius, obj_layer_nr)`, and `:1855` `get_collision_polys(radius, layer_nr)` - radius-aware exact-Z collision and avoidance.
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp:2652` `TreeSupport::drop_nodes()` - builds the tree/MST from contacts downward; `MinimumSpanningTree` is at `:2823-2834`.
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp:1969` `TreeSupport::draw_circles()` - per-layer collision-radius body emission.
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp:2050` area/interface (`roof_areas`) emission inside `draw_circles`; `:1772`/`:1792` `calc_branch_radius(...)` taper; `:2143` top-interface/termination on model/plate.

<!-- snippet: parity-evidence -->
## Parity Evidence Standard

Every key this packet implements carries evidence per the map's ticket 02 standard:

- **Canonical read + described behaviour.** For each key, cite the canonical consumer (file + function, never line numbers) and describe its behaviour in `requirements.md`. Reads of `OrcaSlicerDocumented/` are delegated per the orca-delegation snippet.
- **Invariants, not goldens.** Behaviour is pinned with invariant/property tests (counts preserved, mappings hold, emitted values equal expected). Golden G-code comparison is not part of the standard — the checkout is not built and cannot be run.
- **Ported Orca tests are acceptable evidence.** When `OrcaSlicerDocumented/tests/fff_print/` covers the behaviour, port its assertions into PnP's suite with the standard porting header (`docs/ORCASLICER_ATTRIBUTION.md`).
- **Plumbing keys** (a threshold feeding an existing decision point): the default resolves to the canonical value AND a test proves the value reaches the consumer. No behavioural test required.
- **Unverifiable behaviour:** surface the key and the reason to the human first; only with their sign-off file a `docs/DEVIATION_LOG.md` row (single source of truth, CI-checked by `cargo xtask check-deviations`) and proceed with documented scope. Never defer the key or block the packet on unverifiability alone, and never file a row without the human having been asked.

## Acceptance Summary
AC-1..AC-6 cover distributed planning, collision-safe structural geometry, termination, rendering identity, family selection, and disable/decline behavior. AC-N1..AC-N2 cover invalid geometry and attribution enforcement.

## Verification Matrix
| Requirement | Evidence | Command |
| --- | --- | --- |
| Distributed contacts and tree geometry | planner test | `cargo test -p tree-support-planner --test tree_family_tdd distributed_contacts -- --exact` |
| Radius-aware collision and termination | planner test | `cargo test -p tree-support-planner --test tree_family_tdd radius_aware_collision -- --exact` |
| Renderer identity and polygon construction | renderer test | `cargo test -p tree-support --test tree_family_tdd polygon_renderer_identity -- --exact` |
| Family manifest selection | bounded grep | `rg -q 'support-family:tree' modules/core-modules/tree-support/tree-support.toml && rg -q 'support-family:tree' modules/core-modules/tree-support-planner/tree-support-planner.toml && rg -q 'support-generator' modules/core-modules/tree-support/tree-support.toml && rg -q 'support-planner' modules/core-modules/tree-support-planner/tree-support-planner.toml` |
| Rejection and no fallback | planner/renderer tests | `cargo test -p tree-support-planner --test tree_family_tdd invalid_body_rejected -- --exact` |

## Doc Impact Statement
- `docs/02_ir_schemas.md` tree family role/structural geometry section - `rg -q 'tree-support-planner' docs/02_ir_schemas.md`
- `docs/03_wit_and_manifest.md` family claims section - `rg -q 'support-family:tree' docs/03_wit_and_manifest.md`
- `docs/04_host_scheduler.md` tree dispatch section - `rg -q 'tree-support-planner' docs/04_host_scheduler.md`
