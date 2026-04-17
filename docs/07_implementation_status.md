# Implementation Status

Last updated: 2026-04-16

## Status Markers

- `[x]` complete
- `[~]` partially complete / in progress
- `[ ]` not started

## Program Status

- MVP status: COMPLETE.
- Historical implementation phases A through G are complete.
- This document now tracks the remaining work: OrcaSlicer feature parity, architecture-conformance cleanup, and acceptance-gate evidence.
- Phase H remains open because the live Benchy run is not yet acceptance-grade.

## Milestone Summary

- [x] Phase A — Foundation (TASK-001 to TASK-006)
- [x] Phase B — Core Algorithms (TASK-010 to TASK-015)
- [x] Phase C — Host Scheduler (TASK-020 to TASK-036)
- [x] Phase D — SDK Tooling (TASK-040 to TASK-058)
- [x] Phase E — MVP Core Modules & CLI (TASK-070 to TASK-077)
- [x] Phase F — Post-MVP & Advanced Features (TASK-081 to TASK-097)
- [x] Phase G — Pipeline Wiring & WASM Integration (TASK-100 to TASK-113)
- [~] Phase H — End-to-End Integration & Review

## Current Acceptance Snapshot

- The live pipeline now produces `.gcode` for the Benchy STL.
- The output is still below the Phase H acceptance bar.
- Known live-output gaps on the Benchy path: top/bottom surface infill, support structures, seam placement on real wall loops, and travel retraction / unretraction behavior.
- Architecture Acceptance Gate is blocked on the remediation backlog below.

## Active Remediation Backlog

- Coverage check (2026-04-17): every open `DEV-###` row in `docs/DEVIATION_LOG.md` is now owned by at least one task below.
- Parent tasks marked `[~]` are umbrella items retained for continuity; they close only when all listed child tasks land.

### Workstream 1 — Manifest and contract conformance

- [x] TASK-121 Populate `[ir-access]` for all 17 core-module manifests per docs/01 Stage I/O Contract. Covers DEV-002. Must turn `core_module_ir_access_contract_tdd.rs` green.
- [x] TASK-122 Populate `[config.schema]` for all 17 core-module manifests so the `config-schema` CLI returns real per-module schemas. Covers DEV-008.
- [~] TASK-123 Feed `ModuleAccessAudit` from every live execution path and pass populated `access_audits` into validation. Covers DEV-003.
- [ ] TASK-123a Record prepass execution audits and plumb them into `DagValidationRequest.access_audits`. Covers DEV-003.
- [ ] TASK-123b Record per-layer execution audits and plumb them into `DagValidationRequest.access_audits`. Covers DEV-003.
- [ ] TASK-123c Record postpass execution audits and add a live-path regression proving populated audits reach validation. Covers DEV-003.
- [ ] TASK-124 Enforce undeclared runtime read/write faults at the WIT boundary and add a negative harness for layer-time undeclared access. Continues DEV-003 after TASK-123 lands.
- [ ] TASK-125 Enforce the docs/01 Claim Transition Matrix for non-transitionable claims (`perimeter-generator`, `seam-placer`, `layer-planner`, `mesh-analyzer`). Covers DEV-004 and must turn `claim_transition_matrix_tdd.rs` green.
- [ ] TASK-126 Fix `WriteConflict.orderable` so it reports `true` only when ordering can actually resolve the pair; add both positive and negative semantics tests. Scheduler conflict-ordering cleanup required for the docs/04 contract.
- [ ] TASK-144 Consolidate host, macro, and guest codegen onto one canonical shared WIT source rooted in `wit/`. Covers DEV-014.
- [ ] TASK-145 Normalize WIT package/version identifiers and restore missing members across the canonical WIT surface, generated bindings, schema constants, and test guests; add drift-detection regression coverage. Continues DEV-014.
- [ ] TASK-146 Add host-side `wit_world` allowlist validation using the canonical identifiers and reject mismatched manifests at startup. Covers the validation slice of DEV-014 and DEV-026.
- [ ] TASK-149 Widen the WIT types so `ExtrusionRole::Custom(String)`, `PaintSemantic::Custom(String)`, and `WallFeatureFlags.custom` can cross the boundary losslessly. Covers DEV-016.
- [ ] TASK-150 Update host, macro, and guest converters to preserve the widened custom payloads and add round-trip WIT regression tests. Continues DEV-016.

### Workstream 2 — Runtime correctness and scheduler guarantees

- [ ] TASK-127 Enforce the non-planar Z envelope `[layer.z, layer.z + effective_layer_height]` at output-commit boundaries. Covers DEV-005.
- [~] TASK-128 Resolve prepass segmentation input-shape gaps on the macro/WIT path so segmentation modules stop receiving hollow SDK inputs. Covers DEV-025.
- [ ] TASK-128a Provide usable `MeshSegmentation` inputs on the macro path by sourcing real geometry for `MeshObjectView` instead of object-id-only shells. Covers DEV-025.
- [ ] TASK-128b Provide usable `PaintSegmentation` inputs on the macro path, including transform matrices, paint layers, and participating layer indices. Continues DEV-025.
- [~] TASK-129 Close the remaining non-segmentation WIT-boundary gaps on live execution paths. Covers DEV-006.
- [ ] TASK-129a Pass real postpass GCode command lists into `dispatch_postpass_gcode_call` and add coverage for per-command content crossing the WIT boundary. Covers DEV-006.
- [ ] TASK-129b Add live-path boundary coverage for layer-world deep-copy behavior outside native fallback code. Continues DEV-006.
- [ ] TASK-129c Add live-path boundary coverage for finalization-world deep-copy behavior outside native fallback code. Continues DEV-006.
- [~] TASK-130 Finish the `#[slicer_module]` prepass segmentation bridge for macro-authored modules. Covers DEV-025.
- [ ] TASK-130a Drain `PaintSegmentationOutput` back through WIT `push-paint-region` so macro-authored modules can emit paint regions without hand-written `wit-guest` glue. Covers DEV-025.
- [ ] TASK-130b Add end-to-end macro-path regression tests proving `MeshSegmentation` and `PaintSegmentation` round-trip real data through WIT. Continues DEV-025.
- [ ] TASK-131 Add a regression guard for the documented `resolve_active_regions` O(1) contract. Scheduler performance guard needed for runtime-budget evidence.
- [ ] TASK-132 Add structured RegionMap overflow coverage for the 1000-entry cap, including top-contributor and remediation messaging. Hardens the existing bounds path needed for DEV-026 evidence.
- [ ] TASK-133 Add a pool-behavior test proving `layer_parallel_safe = false` serializes concurrent WASM acquisition. Scheduler concurrency guard for the docs/04 instance-pool contract.
- [ ] TASK-134 Add a catch-up-layer propagation test that verifies `is_catchup_layer`, `catchup_z_bottom`, and `effective_layer_height` survive every per-layer stage. Guards the documented catch-up-layer propagation contract across every per-layer stage.
- [ ] TASK-147 Implement live mesh-data wiring for `raycast_z_down` and cover hit/miss semantics across the WIT worlds. Covers DEV-015.
- [ ] TASK-148 Implement `surface_normal_at` and `object_bounds` on the same mesh-query backing surface, replacing the current stub/trap behavior with tested results. Continues DEV-015.
- [ ] TASK-157 Add fixture-level integration coverage for non-identity object transforms so transformed STL/3MF inputs prove correct world-space Z behavior through planning. Covers DEV-027.
- [ ] TASK-158 Promote world-space Z extent to one canonical derived contract surface, either first-class IR or explicitly documented config-only behavior, then regression-lock transformed-object behavior. Continues DEV-027.

### Workstream 3 — Benchy parity and missing OrcaSlicer behavior

- [~] TASK-120 Produce a fully sliced Benchy `.gcode` with tree supports enabled as the Phase H end-to-end acceptance run.
- [ ] TASK-120a Restore top/bottom surface fill generation on the live Benchy path. Covers DEV-009.
- [ ] TASK-120b Restore support generation on the live Benchy path. Covers DEV-009.
- [ ] TASK-120c Restore seam placement on real wall-loop seam candidates. Covers DEV-009.
- [~] TASK-120d Restore live Benchy travel behavior on the path-optimization or emit path. Covers DEV-009 and the travel-behavior slice of DEV-023.
- [ ] TASK-120d1 Decide where retraction policy lives (`path-optimization-default` vs emit path) and implement retract/no-retract decisions for live travel moves. Covers DEV-009.
- [ ] TASK-120d2 Emit matching retract/unretract pairs, z-hop interactions, and Benchy regression assertions for the chosen travel-policy surface. Covers DEV-009.
- [ ] TASK-135 Add Benchy regression assertions for supports, top/bottom fills, seams, and retract/unretract pairs. Supports DEV-009 acceptance evidence.
- [ ] TASK-142 Port `SkirtBrim` live geometry from legacy `process()` into `run_finalization()` using `LayerCollectionView` and `FinalizationOutputBuilder`. Covers DEV-013.
- [ ] TASK-143 Port `WipeTower` live geometry from legacy `process()` into `run_finalization()` and retire the legacy-only finalization path. Continues DEV-013.
- [ ] TASK-151 Teach `path-optimization-default` to consume seam-placement output and stop acting as a comment-only slot filler on real wall loops. Covers the non-retraction portion of DEV-023 and supports TASK-120c.
- [~] TASK-152 Expand `path-optimization-default` beyond comment-only output into a real optimization stage with deterministic travel ordering, module-side z-hop planning, and explicit coverage for the remaining DEV-023 feature gaps. Continues DEV-023 and supports TASK-120d.
- [ ] TASK-152a Add deterministic nearest-neighbor-style travel sequencing in `path-optimization-default`, with regression coverage on real per-layer entities instead of preserving `assemble_ordered_entities` order. Continues DEV-023 and supports TASK-120d.
- [ ] TASK-152b Emit module-level tool-change ordering for mixed-tool layers and regression coverage proving `LayerTools`-equivalent sequencing crosses the live path. Continues DEV-023.
- [ ] TASK-152c Decide whether fan-speed / cooling overrides belong in path optimization or remain intentionally unsupported; either implement the chosen override surface with regression coverage or document the rejection path explicitly in docs/05 and docs/07. Continues DEV-023.
- [ ] TASK-152d Add cross-object ordering in `path-optimization-default` so per-layer planning can sequence entities across objects instead of treating each layer in object isolation; cover deterministic mixed-object cases. Continues DEV-023.
- [ ] TASK-152e Add role-aware bridge / overhang reordering so bridge-sensitive entities can be prioritized and regression-lock the behavior on bridge-tagged inputs. Continues DEV-023.
- [ ] TASK-152f Coordinate `path-optimization-default` with `SkirtBrim` and `WipeTower` outputs so wipe / brim travel decisions stop ignoring finalization geometry; add integration coverage across the finalization boundary. Continues DEV-023 and supports TASK-142 and TASK-143.

### Workstream 4 — Progress events and Python bridge coverage

- [ ] TASK-136 Add end-to-end progress-event coverage proving paint-annotation failure codes 501-504 reach the JSONL emitter on the live pipeline path. Supports DEV-010 acceptance evidence and guards the live path after DEV-019 closure.
- [~] TASK-137 Resolve the Python postpass live-path decision and closure evidence. Covers DEV-024. Exactly one of TASK-137b or TASK-137c should land after TASK-137a.
- [ ] TASK-137a Decide whether Python postpass is a supported live-path backend or a test-only facility, and record the policy target in docs/05 and docs/07. Covers DEV-024.
- [ ] TASK-137b If Python is intended to be live, add explicit runtime selection for `PythonPostpassRunner` and acceptance coverage through the production pipeline. Continues DEV-024.
- [ ] TASK-137c If Python is intentionally non-live, remove stale live-path expectations from docs/05 and docs/07 and close DEV-024 on the documentation path. Alternate close path for DEV-024.
- [x] TASK-138 Close the Python `Init` phase coverage gap. `crates/slicer-host/tests/python_bridge_init_phase_tdd.rs` is green.

### Workstream 5 — Governance and closure drift

- [~] TASK-139 Close the DEV-020 source/docs drift around dead fallback runners and Phase G closure notes.
- [ ] TASK-139a Remove dead `Noop*Runner` remnants from the source tree and any stale tests or wiring. Covers DEV-020.
- [ ] TASK-139b Correct the Phase G closure notes in docs/07 once the source cleanup lands so docs and source agree. Covers DEV-020.
- [ ] TASK-140 Evaluate the Architecture Acceptance Gate using docs/11 and docs/12 once TASK-120 and its subtasks are complete. Covers DEV-010 and the evidence gaps in DEV-026.
- [~] TASK-141 Keep the planning/governance docs synchronized with the live dependency graph and deviation registry. Supports DEV-030.
- [ ] TASK-141a Update docs/07 dependency ordering and workstream sequencing whenever a remediation task is added, split, or closed. Supports DEV-030.
- [ ] TASK-141b Keep `docs/DEVIATION_LOG.md` synchronized with every open architectural deviation, linked task IDs, and close status. Supports DEV-030 and live-registry hygiene for the acceptance gate.
- [ ] TASK-141c Remove stale blocker summaries and closure notes from docs/11 and docs/12 as their owning tasks land. Supports DEV-030.
- [ ] TASK-154 Enforce `min_host_version` at startup and add semver pass/fail coverage for compatible and incompatible manifests. Covers DEV-026.
- [ ] TASK-155 Make manifest-schema validation surface a real `Schema` failure for missing or invalid schema declarations, with CLI and host regression tests. Continues DEV-026.
- [ ] TASK-156 Add runtime-budget evidence collection for docs/12 memory, host-call, and full-slice time thresholds, plus reproducible benchmark/report hooks. Continues DEV-026.

## Open Deviation Map

Use `docs/14_deviation_audit_history.md` only for retired XML-era numbering and audit provenance. The list below is the current live map.

- DEV-002 — Core-module `ir-access` declarations are incomplete.
- DEV-003 — Runtime IR-access enforcement is dormant because access audits are not fed from live execution.
- DEV-004 — Claim Transition Matrix is not enforced for non-transitionable claims.
- DEV-005 — Non-planar Z envelope enforcement is missing.
- DEV-006 — Postpass GCode command content and executable WIT-boundary coverage still have live gaps.
- DEV-008 — Core-module `config.schema` declarations are empty.
- DEV-009 — Benchy Phase H output is only partially correct on the live path.
- DEV-010 — Acceptance-gate evidence and governance closure are still open.
- DEV-013 — Finalization core modules still keep live behavior in legacy `process()` instead of `run_finalization()`.
- DEV-014 — WIT compatibility is split across multiple sources and still drifts.
- DEV-015 — Mesh-query host services remain stubs.
- DEV-016 — Custom string payloads are dropped at the WIT boundary.
- DEV-020 — Phase G still overstates completion because dead `Noop*Runner` code remains.
- DEV-023 — PathOptimization remains an MVP slot-filler rather than a real optimization stage.
- DEV-024 — Python postpass support exists but is not on the live path.
- DEV-025 — Prepass segmentation SDK↔WIT shapes are still misaligned.
- DEV-026 — Host semver, manifest-schema validation, and runtime budget evidence remain incomplete.
- DEV-027 — Transform-aware world-space Z lacks fixture-level integration coverage and a first-class IR surface.
- DEV-030 — Planning and remediation docs still lag the real dependency graph.

## Tests Added as Gap Locks

- [x] `crates/slicer-host/tests/core_module_ir_access_contract_tdd.rs` — enumerates missing manifest IR contracts and guards the Stage I/O Contract.
- [x] `crates/slicer-host/tests/claim_transition_matrix_tdd.rs` — guards the non-transitionable claim matrix and transitionable-claim sanity cases.
- [x] `crates/slicer-host/tests/python_bridge_init_phase_tdd.rs` — closes the Python `Init` phase classification gap.

## Architecture Acceptance Gate

- Status: BLOCKED BY OPEN REMEDIATION TASKS
- Blocking tasks: TASK-120a, TASK-120b, TASK-120c, TASK-120d1, TASK-120d2, TASK-121, TASK-123a, TASK-123b, TASK-123c, TASK-125, TASK-127, TASK-128a, TASK-128b, TASK-129a, TASK-129b, TASK-129c, TASK-130a, TASK-130b, TASK-136, TASK-140, TASK-144, TASK-145, TASK-146, TASK-149, TASK-150, TASK-154, TASK-155, TASK-156

### Evidence Links

- Determinism: pending Phase H parity closure
- Recoverability: pending runtime access enforcement and progress-event coverage
- Resource bounds: pending RegionMap overflow, `resolve_active_regions`, and runtime-budget evidence collection
- Coupling control: pending manifest contract cleanup, claim transition enforcement, and custom-payload preservation
- Compatibility: pending WIT-source consolidation, `wit_world` validation, host semver/schema validation, and acceptance-gate evaluation
- Operability: pending Benchy acceptance run, finalization parity, and progress-event validation

### Notes

- Use `./docs/11_operational_governance_and_acceptance_gate.md` as the rubric.
- Metric thresholds are defined in `./docs/12_architecture_gate_metrics.md`.

## Blocked Tasks

- None. The remaining work is prioritized, not externally blocked.

## Governance Checklist Status

- Module/claim rollout checklist: IN PROGRESS
- Compatibility policy checks: NOT STARTED
- Release checklist: NOT STARTED
