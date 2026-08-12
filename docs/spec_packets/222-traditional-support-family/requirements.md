# Requirements: traditional-support-family

## Packet Metadata
- Grouped task IDs: `TASK-333`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement
The live traditional renderer is `Layer::Support` but writes `SupportIR` without reading `SupportPlanIR` (`modules/core-modules/traditional-support/traditional-support.toml:9-18`). This packet introduces a genuine cross-layer traditional planner and removes per-layer eligibility/filler behavior from the renderer.

## In Scope
- Create `traditional-support-planner` at `PrePass::SupportGeometry` with `support-planner` and `support-family:traditional` claims.
- Detect contact areas across model layers, propagate base geometry downward, create top/bottom interfaces, avoid obstacles, and select model/plate termination surfaces.
- Emit universal structural `SupportPlanIR` roles, identities, anchored support heights, provenance, and decline records.
- Make `traditional-support` require traditional plan entries and scan-fill only their semantic polygons into structured `SupportIR`.
- Implement and test `support_interface_top_layers`, `support_interface_bottom_layers`, contact/interface spacing, `support_top_z_distance_mm`, `support_layer_height_mm`, and relevant support filament/contact settings already exposed by the resolved config; add `support_base_pattern` (Orca `PrintConfig.cpp:6867` parity) as a new traditional-family config key owned by the `traditional-support-planner` manifest.

## Out of Scope
- TASK-331 exact-Z seam/schema decision, tree family, mixed-family routing, final Orca closure, and raft scheduling.

## Authoritative References
- `modules/core-modules/traditional-support/traditional-support.toml` - current renderer claims and config surface.
- `modules/core-modules/tree-support/tree-support.toml` - paired family manifest pattern.
- `crates/slicer-sdk/src/traits.rs` and `docs/05_module_sdk.md` - `LayerModule` support hooks, delegated bounded reads.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp:374` `PrintObjectSupportMaterial::generate(PrintObject&)` - main orchestration.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp:2095` `top_contact_layers(...)` - contact-area/overhang detection (top contacts).
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp:2592` `bottom_contact_layers_and_layer_support_areas(...)` - contact areas plus obstacle map.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp:1451` - overhang grow into contact.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp:2760` `raft_and_intermediate_support_layers(...)` - placeholder intermediate/base layers.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp:2953` `generate_base_layers(...)` - downward propagation of support to plate.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp:3068`/`:3070` - projected-down geometry merge and trim by obstacle collision polygons.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp:3074` - base layer marking and termination of down-grow.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp:3106` `trim_support_layers_by_object(...)` - collision handling with gap_support_object/gap_xy offsets.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp:3208` `clip_by_pillars(...)` - collision/obstacle avoidance around pillars.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp:480`/`OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp:47` `generate_interface_layers(...)` - roof/floor interface.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp:2735` `trim_top_contacts_by_bottom_contacts(...)` - terminates top contacts by bottom contacts.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp:523` `generate_support_layers(...)`; `:555` `generate_support_toolpaths(...)`; `:1980` `merge_contact_layers(...)`; `:487` `generate_raft_base(...)`.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp:374` `PrintObjectSupportMaterial::generate(PrintObject&)`.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp:2095` `top_contact_layers(...)`.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp:2592` `bottom_contact_layers_and_layer_support_areas(...)`.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp:1451`, `:2760`, `:2953`, `:3068`/`:3070`, `:3074`, `:3106`, `:3208`.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp:480`/`OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp:47` `generate_interface_layers(...)`.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp:2735`, `:523`, `:555`, `:1980`, `:487`.

<!-- snippet: parity-evidence -->
## Parity Evidence Standard

Every key this packet implements carries evidence per the map's ticket 02 standard:

- **Canonical read + described behaviour.** For each key, cite the canonical consumer (file + function, never line numbers) and describe its behaviour in `requirements.md`. Reads of `OrcaSlicerDocumented/` are delegated per the orca-delegation snippet.
- **Invariants, not goldens.** Behaviour is pinned with invariant/property tests (counts preserved, mappings hold, emitted values equal expected). Golden G-code comparison is not part of the standard — the checkout is not built and cannot be run.
- **Ported Orca tests are acceptable evidence.** When `OrcaSlicerDocumented/tests/fff_print/` covers the behaviour, port its assertions into PnP's suite with the standard porting header (`docs/ORCASLICER_ATTRIBUTION.md`).
- **Plumbing keys** (a threshold feeding an existing decision point): the default resolves to the canonical value AND a test proves the value reaches the consumer. No behavioural test required.
- **Unverifiable behaviour:** surface the key and the reason to the human first; only with their sign-off file a `docs/DEVIATION_LOG.md` row (single source of truth, CI-checked by `cargo xtask check-deviations`) and proceed with documented scope. Never defer the key or block the packet on unverifiability alone, and never file a row without the human having been asked.

## Acceptance Summary
AC-1..AC-6 cover contact/base/interface/obstacle planning, termination, anchored rendering, family selection, and disabled/declined behavior. AC-N1..AC-N2 cover invalid bodies and missing/mismatched planned geometry.

## Verification Matrix
| Requirement | Evidence | Command |
| --- | --- | --- |
| Cross-layer contacts and base/interface propagation | planner test | `cargo test -p traditional-support-planner --test traditional_family_tdd contact_area_planning -- --exact` |
| Obstacle-safe termination | planner test | `cargo test -p traditional-support-planner --test traditional_family_tdd base_interface_obstacle -- --exact` |
| Planned polygon-only rendering | renderer test | `cargo test -p traditional-support --test traditional_family_tdd planned_polygon_renderer -- --exact` |
| Family manifest selection | bounded grep | `rg -q 'support-family:traditional' modules/core-modules/traditional-support/traditional-support.toml && rg -q 'support-family:traditional' modules/core-modules/traditional-support-planner/traditional-support-planner.toml && rg -q 'support-generator' modules/core-modules/traditional-support/traditional-support.toml && rg -q 'support-planner' modules/core-modules/traditional-support-planner/traditional-support-planner.toml` |
| Invalid/missing plan rejection | planner/renderer tests | `cargo test -p traditional-support --test traditional_family_tdd mismatched_or_missing_plan -- --exact` |

## Doc Impact Statement
- `docs/02_ir_schemas.md` traditional family role section - `rg -q 'traditional-support-planner' docs/02_ir_schemas.md`
- `docs/03_wit_and_manifest.md` family claims section - `rg -q 'support-family:traditional' docs/03_wit_and_manifest.md`
- `docs/04_host_scheduler.md` traditional dispatch section - `rg -q 'traditional-support-planner' docs/04_host_scheduler.md`
- `docs/15_config_keys_reference.md` traditional support key ownership - `rg -q 'support_base_pattern' docs/15_config_keys_reference.md`
