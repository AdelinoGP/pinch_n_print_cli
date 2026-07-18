# Task Map: 113a-arachne-parity-closures

This packet spans 6 distinct work items that close 2 `D-112-*` deviations plus 4 audit findings, and adds supporting evidence for a third deviation (`D-112-MMU-TOPOLOGY`) that stays open and re-targets to P113b. The task map below shows how each packet step maps to the M2 plan items in `docs/specs/perimeter-modules-orca-parity-roadmap.md` and the corresponding deviation closures. No `TASK-###` entries exist in `docs/07_implementation_status.md` for this work per the packet-112 handoff; the M2 plan doc + `docs/DEVIATION_LOG.md` are the authoritative crosswalk.

## Task Crosswalk

| Packet Step | Deviation Closed | M2 Plan Reference | Authoritative Doc |
|---|---|---|---|
| Step 1: Visvalingam + `dp_epsilon` → `visvalingam_area_threshold` rename + NEW negative test | `D-112-SIMPLIFY-DP` | `T-226` (Phase 12, §"M2 — Real Arachne") | `docs/02_ir_schemas.md`, `docs/03_wit_and_manifest.md` |
| Step 2: 4 new `ArachneParams` fields + `BeadingFactoryParams` threading | `D-112-THIN-WALL-WIDENING` (residual, factory half) | `T-218` (Phase 11, §"M2 — Real Arachne") | `docs/03_wit_and_manifest.md`, `docs/08_coordinate_system.md` |
| Step 3: 3 net-new manifest entries + 4 already-registered reads + NEW defaults test | `D-112-THIN-WALL-WIDENING` (residual, pipeline half) | `T-218` (Phase 11, §"M2 — Real Arachne") | `docs/08_coordinate_system.md` |
| Step 4: MMU unit test on `paint_segmentation` (NEW test file) | (supporting evidence for `D-112-MMU-TOPOLOGY`; deviation stays open) | `T-231` (Phase 13, §"M2 — Real Arachne") | `docs/specs/orca-mmu-perimeter-investigation.md`, `docs/12_architecture_gate_metrics.md` |
| Step 5: Simplify executor + tighten guard + fixture dir + add closure-log section | (audit findings, not registered deviations) | `T-231` (Phase 13, §"M2 — Real Arachne") | N/A |
| Step 6: Close 2 deviations (1 stays open) + workspace gate | All 3 above | (administrative) | `docs/DEVIATION_LOG.md` (lines 40-50) |

## Deviation Disposition (Post-Packet)

| Deviation | Status After P113a | Mechanism |
|---|---|---|
| `D-112-SIMPLIFY-DP` | **CLOSED** | Step 1 ports Visvalingam + width-weighted area gate |
| `D-112-THIN-WALL-WIDENING` (residual) | **CLOSED** | Steps 2+3 wire all 7 config keys through `arachne_params_from_config` (4 already in manifest, 3 net-new) |
| `D-112-MMU-TOPOLOGY` | **STAYS OPEN** (re-targeted to P113b) | Step 4 adds unit test on `paint_segmentation` as supporting evidence (the geometric partition invariant is upstream of the actual symptom, which is governed by `arachne-perimeters` output topology). P113b's quad/rib pass + faithful `connectJunctions` is the new target. |
| `D-112-CENTRALITY-ADAPT` | Still open (P113b) | Quad/rib topology pass required |
| `D-112-PROPAGATION-ADAPT` | Still open (P113b) | Faithful transition marking requires quad/rib topology |
| `D-112-SELFCAPTURED-BASELINES` | Still open (accepted) | No OrcaSlicer binary; matches D-109 precedent |
| (unregistered) `connectJunctions` adaptation | Still open (P113b) | Faithful `connectJunctions` requires quad/rib topology |
| (audit) loader source-guard | **CLOSED** | Step 5 tightens `live_module_loading_tdd.rs:626` to exact-match `_with_config` |
| (audit) `cube_4color_arachne/` fixture dir | **CLOSED** | Step 5 creates the directory + `expected_perimeter_ir.json` golden |
| (audit) closure-log commit diff stat | **CLOSED** | Step 5 adds "M2 — Real Arachne" section recording `148 files, +13,981/−206` |

## Cross-Packet Dependencies

- **Depends on P112** (`d9466fd7`, `status: implemented`): the existing Arachne pipeline source, fixtures, and host-service bridge.
- **Unblocks P113b**: the topology chain (quad/rib pass, faithful centrality, per-NODE bead_count, faithful transitions, faithful `connectJunctions`) can begin once P113a ships, since P113b re-validates downstream stages against the topology-changed input.
- **Does NOT depend on ADR-0033.** The original packet draft listed "ADR-0033 (Algorithm Faithfulness as OrcaSlicer Parity Definition)" as a P113a dependency. That ADR does not exist in `docs/adr/` and the user has not asked for it. The acceptance criteria assert algorithm fidelity via OrcaSlicer code references (`OrcaSlicerDocumented/.../ExtrusionLine.cpp:248` etc.) — code references are sufficient on their own; a formal ADR is not required for this packet.

## OrcaSlicer Reference Paths (per `requirements.md` §OrcaSlicer Reference Obligations)

- `OrcaSlicerDocumented/src/libslic3r/Arachne/utils/ExtrusionLine.cpp:248` — `calculateExtrusionAreaDeviationError(A, B, C)` (Step 1)
- `OrcaSlicerDocumented/src/libslic3r/Arachne/utils/ExtrusionLine.cpp:152` — call site in the simplification loop (Step 1)
- `OrcaSlicerDocumented/src/libslic3r/MultiMaterialSegmentation.cpp:494` — `extract_colored_segments()` (Step 4)
