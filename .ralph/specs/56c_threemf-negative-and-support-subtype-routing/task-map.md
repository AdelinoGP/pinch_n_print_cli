# Task Map: 56c_threemf-negative-and-support-subtype-routing

## Purpose

This packet introduces three new TASK IDs (TASK-192b, TASK-192c, TASK-193) not present in `docs/07_implementation_status.md` at packet-author time. Step 5 of `implementation-plan.md` appends them as new rows after the TASK-192a row registered by Packet 56b. This file maps each new TASK to the implementation steps that satisfy it, the deviations it touches, and the OrcaSlicer references applicable to its scope.

This packet is the terminal packet in the three-way split of the original `56_threemf-modifier-and-subtype-sidecar-ingestion` packet. Packet 56 owns TASK-190 (sidecar parser). Packet 56b owns TASK-191 (resolve_object branching + schema bump) and TASK-192a (modifier_part region overlap). This packet owns TASK-192b (negative_part host stage), TASK-192c (support enforcer/blocker piggyback), and TASK-193 (synthetic-fixture E2E coverage + no-regression sweep).

## Task-to-Step Mapping

| TASK ID | Topic | Implementation steps | Deviations addressed | Authoritative docs | OrcaSlicer ref(s) |
|---|---|---|---|---|---|
| TASK-192b | New host stage `apply_negative_part_subtract` inserted between prepass and region-mapping; per-layer 2D `slicer_core::polygon_ops::difference` against parent's slice polygons for each `negative_part` modifier volume. | Step 2 (impl + pipeline insertion). | None. | `docs/04_host_scheduler.md` (prepass / region-mapping ordering); `docs/08_coordinate_system.md`. | `OrcaSlicerDocumented/src/libslic3r/Format/bbs_3mf.cpp` — negative-part subtract function name (LOCATIONS dispatch at Step 2). |
| TASK-192c | Synthetic `PaintRegionIR` emission for `support_enforcer` and `support_blocker` modifier volumes via paint-segmentation piggyback. Flows through Packet 51's `paint_overrides` overlay. | Step 3 (paint-segmentation augment + pipeline thread). | None (inherits DEV-045 (Packet 51) overlay path). | `docs/02_ir_schemas.md` (PaintRegionIR / PaintSemantic narrow read); `docs/04_host_scheduler.md`. | `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — support enforcer/blocker geometry function name (LOCATIONS dispatch at Step 3). |
| TASK-193 | TDD coverage: synthetic-fixture E2E for negative_part, support_enforcer, support_blocker, ordering correctness, degenerate cases. No-regression sweep. Acceptance ceremony with workspace test dispatch. | Step 1 (test scaffolding + RED), Step 4 (regression + clippy), Step 5 (doc registration), Step 6 (pre-ceremony verification), Step 7 (acceptance ceremony + workspace gate). | All deviations covered indirectly via existing tests (no new DEVs). | This packet's `packet.spec.md` Acceptance Criteria section. | None new. |

## Deviation Map

| Deviation ID | Title | Registered by step | Closed by step | Owner packet |
|---|---|---|---|---|
| DEV-047 | Partial subtype coverage; unknown subtype downgrade. | — | Already closed by Packet 56. | Packet 56. NOT this packet. |
| DEV-048 | Paint dropped on non-`NormalPart` rows. | — | Already closed by Packet 56b. | Packet 56b. NOT this packet. |
| DEV-049 | Missing/malformed sidecar fallback. | — | Already closed by Packet 56. | Packet 56. NOT this packet. |
| (none new) | This packet introduces no new deviations. The negative-part subtract and support enforcer/blocker piggyback are contract-conformant behaviors. | — | — | — |

The packet acceptance criterion `! rg -q '^\| DEV-.*Closed.*Packet 56c' docs/DEVIATION_LOG.md` asserts zero DEV rows attributed to this packet.

## OrcaSlicer Reference Schedule

| Step | Question | Return format |
|---|---|---|
| Step 2 | "Name the function(s) in `OrcaSlicerDocumented/src/libslic3r/Format/bbs_3mf.cpp` (or sibling) that perform negative-part per-layer subtract." | LOCATIONS, ≤ 5 entries. |
| Step 3 | "Name the function(s) in `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` (or sibling) that emit `support_enforcer` / `support_blocker` geometry into the slicer's paint pipeline." | LOCATIONS, ≤ 5 entries. |

All OrcaSlicer reads are delegate-only. Function names are cited in `requirements.md` and `design.md`; no source snippets are pasted.

## Cross-Packet Dependencies

| Dependency | Direction | Note |
|---|---|---|
| Packet 56 (`56_threemf-sidecar-parser`) | This packet depends on | Provides `parse_3mf_sidecar`, `PartSubtype` enum, and the `_sidecar` parameter on `resolve_object`. Step 0 FACT verifies `status: implemented`. |
| Packet 56b (`56b_threemf-modifier-part-ir-routing`) | This packet depends on | Provides `resolve_object` branching, `MeshIR.schema_version == 1.1.0`, and populated `ObjectMesh.modifier_volumes` for ALL non-`NormalPart` subtypes including `negative_part` and `support_*` (which had no consumer until this packet). Step 0 FACT verifies `status: implemented`. |
| Packet 50b | This packet depends on | Provides `PaintSemantic::SupportEnforcer` / `PaintSemantic::SupportBlocker` enum variants. Indirect dependency confirmed at Step 3 FACT dispatch. |
| Packet 51 (DEV-045) | This packet depends on | Provides `paint_overrides: BTreeMap<PaintSemantic, ResolvedConfig>` overlay; this packet's synthetic `PaintRegionIR` entries flow through it. |
| `slicer_core::polygon_ops::difference` and (conditionally) `::union` | Library deps | Public Clipper2-backed exports. |
| (none) | This packet unblocks | Terminal packet in three-way split. Future packets for `extruder` per-modifier override, etc., are unrelated future work. |

## Notes for Implementer

- This packet does not modify any prior packet's `.ralph/specs/` directory. Predecessor status flips happen in their own ceremonies.
- **`cargo test --workspace` runs exactly once**, at Step 7 acceptance ceremony, via worker FACT dispatch. This is the only packet in the three-way split that runs the workspace gate; it is justified because this is the terminal closure of the original `56_threemf-modifier-and-subtype-sidecar-ingestion` slice.
- This packet does NOT touch `crates/slicer-host/src/model_loader.rs` or `region_mapping.rs` — those are owned by Packets 56 / 56b and are immutable per Cross-Packet Mutation Rule.
- The synthetic-fixture builder in `threemf_subtypes_synthetic_e2e_tdd.rs` may duplicate ~50 lines of the in-memory `zip::write::ZipWriter` pattern from `threemf_transform_tdd.rs`. Refactoring to a shared helper is out of scope (would require touching a Packet 56b-adjacent file or introducing a new test-helper crate).
- Once this packet closes, the 3MF loader handles all five OrcaSlicer / Bambu Studio `<part subtype>` values (`normal_part`, `modifier_part`, `negative_part`, `support_enforcer`, `support_blocker`) end-to-end. The original packet's L-aggregate scope is closed across three packets totaling 3 M each.
