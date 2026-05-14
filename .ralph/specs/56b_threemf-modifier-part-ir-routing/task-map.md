# Task Map: 56b_threemf-modifier-part-ir-routing

## Purpose

This packet introduces two new TASK IDs (TASK-191 and TASK-192a) not present in `docs/07_implementation_status.md` at packet-author time. Step 7 of `implementation-plan.md` appends them as new rows after the TASK-190 row registered by Packet 56. This file maps each new TASK to the implementation steps that satisfy it, the deviations it touches, and the OrcaSlicer references applicable to its scope.

This packet is the second of a three-way split of the original `56_threemf-modifier-and-subtype-sidecar-ingestion` packet. Packet 56 owns the sidecar parser (TASK-190). Packet 56c owns TASK-192b (`apply_negative_part_subtract` host stage), TASK-192c (support enforcer/blocker piggyback), and TASK-193 (synthetic-fixture E2E coverage).

## Task-to-Step Mapping

| TASK ID | Topic | Implementation steps | Deviations addressed | Authoritative docs | OrcaSlicer ref(s) |
|---|---|---|---|---|---|
| TASK-191 | Branch `resolve_object` to route non-`NormalPart` geometry into `ObjectMesh.modifier_volumes`; drop paint data on non-`NormalPart` rows; bump `MeshIR.schema_version` 1.0.0 → 1.1.0. | Step 1 (TDD-RED), Step 2 (impl + schema bump), Step 7 (doc bump). | DEV-048 (paint dropped on non-normal parts). | `docs/02_ir_schemas.md` lines 5, 62-244; `docs/01_system_architecture.md` :107-114. | `OrcaSlicerDocumented/src/libslic3r/Format/bbs_3mf.cpp` — `<part subtype>` branching function name (LOCATIONS dispatch at Step 2). |
| TASK-192a | Wire the `modifier_part` consumer: region-mapping direct stamp (Option 1) via `slicer_core::polygon_ops::intersection`; stamp `RegionPlan.config["fuzzy_skin.apply-to-all"] = true` on overlapping regions only. Includes fuzzy-skin manifest schema confirmation gate. | Step 3 (manifest gate), Step 4 (TDD-RED), Step 5 (impl + pipeline thread). | None (inherits DEV-045 (Packet 51) overlay path). | `docs/01_system_architecture.md` :107-114; `docs/03_wit_and_manifest.md` (manifest schema); `docs/08_coordinate_system.md`. | `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — fuzzy-skin overlap function name (LOCATIONS dispatch at Step 5). |

## Deviation Map

| Deviation ID (recommended) | Title | Registered by step | Closed by step | Owner packet |
|---|---|---|---|---|
| DEV-048 | Paint data on non-`NormalPart` rows (modifier, negative, support enforcer, support blocker) dropped at load time with `log::warn!`. | Step 7 | Step 7 (registered as Closed by Packet 56b). | This packet (56b). |
| DEV-047 | Partial subtype coverage; unknown subtypes downgrade to `NormalPart`. | — | Already closed by Packet 56. | Packet 56. NOT this packet. |
| DEV-049 | Missing or malformed sidecar fallback. | — | Already closed by Packet 56. | Packet 56. NOT this packet. |

Recommended numbering verified at Step 7 via FACT dispatch ("Confirm DEV-048 is the next free DEV-### slot, given DEV-047 and DEV-049 are already closed by Packet 56").

## OrcaSlicer Reference Schedule

| Step | Question | Return format |
|---|---|---|
| Step 2 | "Name the function(s) in `OrcaSlicerDocumented/src/libslic3r/Format/bbs_3mf.cpp` that branch on `<part subtype>` and route geometry into the modifier-volume container." | LOCATIONS, ≤ 5 entries. |
| Step 5 | "Name the function(s) in `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` (or sibling) that apply fuzzy-skin overlay to a region for `modifier_part`." | LOCATIONS, ≤ 5 entries. |

All OrcaSlicer reads are delegate-only. Function names are cited in this packet's `requirements.md` and `design.md`; no source snippets are pasted.

## Cross-Packet Dependencies

| Dependency | Direction | Note |
|---|---|---|
| Packet 56 (`56_threemf-sidecar-parser`) | This packet depends on | Provides `parse_3mf_sidecar` and the `_sidecar` parameter on `resolve_object`. Packet 56 MUST be `status: implemented` before this packet activates. Step 0 FACT verifies. |
| Packet 51 (DEV-045) | This packet depends on | `RegionPlan.config` overlay path. This packet's stamp is additive on the `fuzzy_skin.apply-to-all` key only; Packet 51's overlay runs first. |
| Packet 50 (DEV-044) | This packet depends on | `FacetPaintData` ingestion. Used by the paint-drop regression check (`benchy_painted_e2e_tdd`). |
| `slicer_core::polygon_ops::intersection` | Library dep | Public Clipper2-backed export. |
| Packet 56c | This packet unblocks | Packet 56c consumes `ObjectMesh.modifier_volumes` populated by this packet for `negative_part` host-stage subtract and `support_enforcer`/`support_blocker` piggyback. |
| Future packet (per-modifier `extruder` consumer) | This packet unblocks | `config_delta` carries `extruder` value when sidecar provides it; no consumer wires it yet. |

## Notes for Implementer

- This packet does not modify any prior packet's `.ralph/specs/` directory. Packet 56's status flip to `implemented` happened in Packet 56's own ceremony, not here.
- `cargo test --workspace` is NOT run at closure of this packet. The targeted regression suites in Step 6 cover the producer + `modifier_part` consumer surface.
- `resolve_object`'s signature widening to accept `sidecar: &HashMap<u32, ObjectSidecarInfo>` was performed in Packet 56 (with the underscore-prefixed `_sidecar` name). Step 2 of this packet renames the parameter to `sidecar` and branches on it.
- The IR routing for ALL four non-`NormalPart` subtypes (modifier, negative, support_enforcer, support_blocker) lands here in Step 2. Only the `modifier_part` consumer wiring is added (Steps 4-5). The other three subtypes have populated IR but no downstream effect until Packet 56c.
