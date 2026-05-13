# Task Map: 56_threemf-modifier-and-subtype-sidecar-ingestion

## Purpose

This packet introduces four new TASK IDs (TASK-190..193) not present in `docs/07_implementation_status.md` at packet-author time (verified by sub-agent Explore against the file). Step 11 of `implementation-plan.md` appends them as new rows after TASK-181 (the current high-water mark). This file maps each new TASK to the implementation steps that satisfy it, the deviations it touches, and the OrcaSlicer references applicable to its scope.

## Task-to-Step Mapping

| TASK ID | Topic | Implementation steps | Deviations addressed | Authoritative docs | OrcaSlicer ref(s) |
|---|---|---|---|---|---|
| TASK-190 | Parse `Metadata/model_settings.config` sidecar; classify `<part subtype>`; surface typed per-part metadata. | Step 0 (gate), Step 1 (TDD-RED), Step 2 (parser implementation). | DEV-049 (missing/malformed sidecar fallback); DEV-047 (unknown subtype downgrade). | `docs/02_ir_schemas.md` lines 192-211; `docs/08_coordinate_system.md`. | `OrcaSlicerDocumented/src/libslic3r/Format/bbs_3mf.cpp` — sidecar parser function name (LOCATIONS dispatch at Step 1). |
| TASK-191 | Branch `resolve_object` to route non-`normal_part` geometry into `ObjectMesh.modifier_volumes`; drop paint data on non-`normal_part` rows; bump `MeshIR.schema_version` 1.0.0 → 1.1.0. | Step 3 (TDD-RED), Step 4 (impl), Step 11 (doc bump). | DEV-048 (paint dropped on non-normal parts). | `docs/02_ir_schemas.md` lines 5, 62-244; `docs/01_system_architecture.md` :107-114. | `OrcaSlicerDocumented/src/libslic3r/Format/bbs_3mf.cpp` — `<part subtype>` branching function name (LOCATIONS dispatch at Step 1, reused at Step 4). |
| TASK-192 | Wire each subtype's downstream consumer: `modifier_part` → region-mapping direct stamp; `negative_part` → new host stage `apply_negative_part_subtract`; `support_enforcer`/`blocker` → synthetic `PaintRegionIR` via paint-segmentation piggyback. | Step 5 (manifest gate), Step 6 (modifier_part overlap stamp), Step 7 (negative_part subtract stage), Step 8 (support enforcer/blocker piggyback). | None (no new deviations). Inherits DEV-045 (Packet 51) overlay path for support paints. | `docs/01_system_architecture.md` :107-114; `docs/04_host_scheduler.md` prepass / region-mapping ordering; `docs/03_wit_and_manifest.md` (manifest schema). | `OrcaSlicerDocumented/src/libslic3r/Format/bbs_3mf.cpp` — fuzzy_skin overlap, negative-part subtract, support enforcer/blocker geometry function names (three LOCATIONS dispatches at Steps 6, 7, 8). |
| TASK-193 | TDD coverage: sidecar parser unit suite, fixture-backed E2E on `benchy_4color.3mf`, synthetic-fixture E2E for negative/support subtypes, no-regression sweep against `benchy_painted.3mf`. | Step 1, Step 3, Step 7, Step 8 (all author tests); Step 9 (regression sweep); Step 10 (clippy); Step 12 (acceptance ceremony). | All three deviations exercised via tests. | This packet's `packet.spec.md` Acceptance Criteria section. | None new. |

## Deviation Map

| Deviation ID (recommended) | Title | Registered by step | Closed by step |
|---|---|---|---|
| DEV-047 | Partial subtype coverage; unknown subtypes downgrade to `normal_part` with `log::warn!`. | Step 11 | Step 11 (registered as Closed by Packet 56). |
| DEV-048 | Paint data on non-`normal_part` rows dropped at load time with `log::warn!`. | Step 11 | Step 11 (registered as Closed by Packet 56). |
| DEV-049 | Missing or malformed `Metadata/model_settings.config` is non-fatal; loader logs warning and treats every part as `normal_part`. | Step 11 | Step 11 (registered as Closed by Packet 56). |

Recommended numbering verified at Step 11 via FACT dispatch ("highest existing DEV-### in `docs/DEVIATION_LOG.md`"); bump if 047/048/049 are claimed by another in-flight packet.

## OrcaSlicer Reference Schedule

| Step | Question | Return format |
|---|---|---|
| Step 1 | "Name the function(s) in `OrcaSlicerDocumented/src/libslic3r/Format/bbs_3mf.cpp` that parse `Metadata/model_settings.config` and the function(s) that branch on `<part subtype>`." | LOCATIONS, ≤ 8 entries. |
| Step 6 | "Name the function(s) that apply `modifier_part` fuzzy_skin overlay to a region in OrcaSlicer." | LOCATIONS, ≤ 5 entries. |
| Step 7 | "Name the function(s) in `bbs_3mf.cpp` or adjacent files that perform negative-part per-layer subtract." | LOCATIONS, ≤ 5 entries. |
| Step 8 | "Name the function(s) that emit `support_enforcer` / `support_blocker` geometry into the slicer's paint pipeline." | LOCATIONS, ≤ 5 entries. |

All OrcaSlicer reads are delegate-only. Function names are cited in the packet artifacts; no source snippets are pasted.

## Cross-Packet Dependencies

| Dependency | Direction | Note |
|---|---|---|
| Packet 50 (DEV-044) | This packet depends on | `FacetPaintData` ingestion. Used by the paint-drop-on-modifier path's regression check. |
| Packet 50a (TASK-180b) | This packet depends on | TriangleSelector subdivision parsing. Indirect: not directly invoked but `paint_supports` semantic is the consumer for support-enforcer/blocker piggyback. |
| Packet 50b (TASK-180b) | This packet depends on | MMU tool-index + multi-channel paint co-presence. Indirect, same as above. |
| Packet 51 (DEV-045) | This packet depends on | `paint_overrides: BTreeMap<PaintSemantic, ResolvedConfig>`; the `support_enforcer`/`blocker` piggyback emits `PaintRegionIR` consumed by Packet 51's overlay. |
| Future packet (per-modifier `extruder` consumer) | This packet unblocks | `config_delta` carries the value; no consumer yet. |

## Notes for Implementer

- This packet does not modify any prior packet's `.ralph/specs/` directory. No cross-packet mutations.
- `cargo test --workspace` runs exactly once, at Step 12 acceptance ceremony, via worker FACT dispatch. Per CLAUDE.md Test Discipline, do not dispatch it speculatively at intermediate steps.
- The L-aggregate context cost is the dominant risk. The user has authorized the broad scope at packet-author time; activation Q1 must still be resolved (split vs. override) before flipping to `status: active`.
