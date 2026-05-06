# Task Map: bridge-detector-orca-parity-fixes

## Backlog ↔ Step Mapping

This packet has one new task ID (`TASK-168`) and one reopened task ID (`TASK-167`). Every implementation step maps to TASK-168; TASK-167 is the audit-trail anchor for the original packet 36 work that this remediation supersedes.

| `docs/07` Task | Maps to step(s) | Notes |
| --- | --- | --- |
| `TASK-167` (reopened) | Step 1 (closure-reversal only) | Original packet 36 task. Reopened in `docs/07_implementation_status.md` as `[ ]` with reopen note pointing at TASK-168. Closes again as part of Step 8's acceptance ceremony when the algorithmic rewrites land. |
| `TASK-168` (new) | Steps 0 – 8 | The remediation work itself. |

## Closure Marker Reversal Map

| Closure marker | Current state (start of packet) | Target state after Step 1 | Target state after Step 8 |
| --- | --- | --- | --- |
| `DEV-035` | Closed (incorrect rationale: "assemble_bridge_areas in mesh_analysis.rs") | Open (with reopen-rationale text) | Closed (correct rationale: real polygon set-difference in `rectilinear-infill`) |
| `DEV-036` | Closed (incorrect rationale: "execute_mesh_analysis_with populates bridge_regions") | Open (with reopen-rationale text) | Closed (correct rationale: real mesh half-edge adjacency analysis in `compute_bridge_metrics`) |
| `DEV-NNN` (new — slicer-helpers boundary) | n/a | Open (registered in Step 1) | Closed (rationale: `docs/13` updated to cross-reference `slicer-core::polygon_ops`; spec amended) |
| `TASK-167` | `[x]` Closed 2026-05-05 by packet 36 | `[ ]` Reopened 2026-05-05 by TASK-168 (packet 36-rev1) | `[x]` Closed (again) by packet 36-rev1 acceptance |
| `TASK-168` | n/a | `[ ]` New | `[x]` Closed by packet 36-rev1 acceptance |
| Packet 36 status | `implemented` | `superseded` (by 36-rev1) | unchanged from Step 1 |

## Authoritative-Doc ↔ Step Mapping

Different docs govern different steps; this table tells the implementer which doc is load-bearing for each step (range-read or delegate; do not load full files).

| Step | Authoritative docs | OrcaSlicer refs |
| --- | --- | --- |
| Step 0 | `docs/13_slicer_helpers_crate.md` (informational only — confirms the spec-amendment rationale) | none |
| Step 1 | `docs/DEVIATION_LOG.md`, `docs/07_implementation_status.md` | none |
| Step 2 | `docs/02_ir_schemas.md` (additive-minor rule, unchanged) | none |
| Step 3 | `docs/04_host_scheduler.md` (cited rationale only — divergence paragraph) | `BridgeDetector.hpp/.cpp` (cited divergence; FACT/SUMMARY only) |
| Step 4 | none | none |
| Step 5 | none | none |
| Step 6 | `docs/03_wit_and_manifest.md` § "WIT/Type Changes Checklist" | none |
| Step 7 | none | none |
| Step 8 | `docs/02_ir_schemas.md`, `docs/13_slicer_helpers_crate.md` | none |

## AC ↔ Step Mapping

Every AC in `packet.spec.md` has at least one implementing step and one verifying step.

| AC | Implementing step(s) | Verifying step | Test file |
| --- | --- | --- | --- |
| AC-1 cluster seed | Step 3 | Step 7 (test) + Step 8 (workspace) | `bridge_detector_tdd.rs` |
| AC-2 anchor_width from edge run | Step 3 | Step 7 + Step 8 | `bridge_detector_tdd.rs` |
| AC-3 xy_footprint facet projection | Step 3 | Step 7 + Step 8 | `bridge_detector_tdd.rs` |
| AC-4 direction from anchor edge | Step 3 | Step 7 + Step 8 | `bridge_detector_tdd.rs` |
| AC-5 rotated min-length filter | Step 3 | Step 7 + Step 8 | `bridge_detector_tdd.rs` |
| AC-6 rotated anchor-width filter | Step 3 | Step 7 + Step 8 | `bridge_detector_tdd.rs` |
| AC-7 expansion margin observable | Step 3 (xy_footprint) + Step 4 (Miter) | Step 7 + Step 8 | `bridge_detector_tdd.rs` |
| AC-8 real set difference | Step 5 | Step 7 + Step 8 | `bridge_infill_emission_tdd.rs` |
| AC-9 bridge orientation precedence | Step 5 (branch fix) | Step 7 + Step 8 | `bridge_infill_emission_tdd.rs` |
| AC-10 schema versions constant-sourced | Step 2 | Step 7 + Step 8 | `ir_tests.rs` |
| AC-11 Benchy E2E exact marker | Step 5 (set-difference enables real BridgeInfill) + Step 7 (test substring tighten) | Step 7 + Step 8 | `benchy_end_to_end_tdd.rs` |
| NEG-1 V-shape sharp anchor | Step 2 (`validate_polygon_simplicity`) + Step 4 (`Miter`) | Step 7 + Step 8 | `bridge_detector_tdd.rs` |
| NEG-2 empty bridge_areas inhibits BridgeInfill | Step 5 (branch fix) | Step 7 + Step 8 | `bridge_infill_emission_tdd.rs` |
| NEG-3 top-only mesh produces no bridges | Step 3 (cluster seed) | Step 7 + Step 8 | `bridge_detector_tdd.rs` |

## Spec-Review Finding ↔ Fix Mapping

This packet was generated from the spec-review of packet 36. Each Critical / High / Medium finding is addressed below.

| Spec-review finding | Severity | Implementing step | Notes |
| --- | --- | --- | --- |
| Mesh adjacency theatrical (wrong cluster, anchor-edge dead, bbox direction, AABB footprint) | Critical | Step 3 | Single-file rewrite. |
| Centroid heuristic instead of set difference in `rectilinear-infill` | Critical | Step 5 | `partition_expoly_by_bridges` body replacement. |
| AC tests don't enforce AC text (4 broken/weak tests) | Critical | Step 7 | Test rewrites. |
| "Orca defaults" attribution fictional | Critical | Step 3 | Doc comments rewritten as project policy with rationale. |
| `slicer-helpers` mandate violated (polygon ops actually in `slicer-core`) | High | Step 1 (DEV register) + Step 8 (`docs/13` update) | Spec amendment. |
| `MeshAnalysisConfig` field-name + shape deviation | High | Step 3 | Rename + consolidate. |
| `docs/02_ir_schemas.md` not updated | High | Step 8 | Banner + new field listings + stale-comment removal. |
| `OffsetJoinType::Square` vs spec's miter/round | High | Step 4 | Switch to `Miter` (or `Round` if not exposed). |
| Host bindgen impls — unverified grep miss | High | Step 0 (FACT) + Step 6 (verify or add) | One-line confirmation step. |
| Missing `task-map.md` | Medium | This packet | Generated as part of `36-rev1`. |
| Wrong `task_ids: TASK-166` declaration | Medium | This packet | New packet declares `TASK-168`; reopens TASK-167. |
| No defensive checks on `expansion_margin_mm < 0 / NaN` | Medium | Step 4 | Sanity guard added. |
| Stale TDD scaffolding comments | Medium | Step 7 | Cleaned during test rewrite. |

## Cross-Packet Dependency Map

| Packet | Relationship | Notes |
| --- | --- | --- |
| `12-rev1_external-surface-classification-at-slice` | Architectural-divergence precedent | Cited in `requirements.md` and `design.md`; reused as the rationale for not porting `detect_angle`. |
| `35_multi-layer-top-bottom-thickness` | Independent | No code overlap; runs before this packet's bridge detection. |
| `35a` (resolved-config-propagation) | Independent | Closed TASK-166; unrelated to this packet's TASK-167 / TASK-168. |
| `36_bridge-detector-orca-parity` | Superseded | Status flipped to `superseded` in Step 1. IR/WIT/SDK plumbing is kept; algorithms are rewritten. |
| `37` (per-surface fill pattern variation) | Out of scope | May consume `bridge_areas` semantics; benefits from the corrected values. |
| `38` (top-surface ironing) | Out of scope | Independent. |
| Future "bridge_speed / bridge_flow_ratio" packet | Not yet scoped | Would consume `bridge_orientation_deg` for cooling/speed overrides. |
| Future "slice-time anchor_regions refinement" packet | Not yet scoped | Would require a new scheduler primitive granting controlled N-1 layer access. Listed as out of scope here. |
