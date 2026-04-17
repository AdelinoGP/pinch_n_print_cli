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

### Workstream 1 — Manifest and contract conformance

- [ ] TASK-121 Populate `[ir-access]` for all 17 core-module manifests per docs/01 Stage I/O Contract. Covers DEV-002. Must turn `core_module_ir_access_contract_tdd.rs` green.
- [ ] TASK-122 Populate `[config.schema]` for all 17 core-module manifests so the `config-schema` CLI returns real per-module schemas. Covers DEV-008.
- [ ] TASK-123 Feed `ModuleAccessAudit` from live prepass / layer / postpass execution paths and pass populated `access_audits` into validation. Covers DEV-003.
- [ ] TASK-124 Enforce undeclared runtime read/write faults at the WIT boundary and add a negative harness for layer-time undeclared access. Continues DEV-003 after TASK-123 lands.
- [ ] TASK-125 Enforce the docs/01 Claim Transition Matrix for non-transitionable claims (`perimeter-generator`, `seam-placer`, `layer-planner`, `mesh-analyzer`). Covers DEV-004 and must turn `claim_transition_matrix_tdd.rs` green.
- [ ] TASK-126 Fix `WriteConflict.orderable` so it reports `true` only when ordering can actually resolve the pair; add both positive and negative semantics tests. Scheduler conflict-ordering cleanup required for the docs/04 contract.

### Workstream 2 — Runtime correctness and scheduler guarantees

- [ ] TASK-127 Enforce the non-planar Z envelope `[layer.z, layer.z + effective_layer_height]` at output-commit boundaries. Covers DEV-005.
- [ ] TASK-128 Implement the remaining prepass-side boundary fixes so segmentation-capable modules stop receiving hollow SDK inputs. Covers DEV-006 and DEV-025.
- [ ] TASK-129 Add live-path boundary coverage for layer and finalization WIT deep-copy behavior so the closed data-copy paths stay regression-locked outside native fallback code. Covers the remaining coverage portion of DEV-006.
- [ ] TASK-130 Finish the `#[slicer_module]` prepass segmentation bridge so `MeshSegmentation` receives usable inputs and `PaintSegmentation` drains output back through WIT. Covers DEV-025.
- [ ] TASK-131 Add a regression guard for the documented `resolve_active_regions` O(1) contract. Scheduler performance guard needed for runtime-budget evidence.
- [ ] TASK-132 Add structured RegionMap overflow coverage for the 1000-entry cap, including top-contributor and remediation messaging. Hardens the existing bounds path needed for DEV-026 evidence.
- [ ] TASK-133 Add a pool-behavior test proving `layer_parallel_safe = false` serializes concurrent WASM acquisition. Scheduler concurrency guard for the docs/04 instance-pool contract.
- [ ] TASK-134 Add a catch-up-layer propagation test that verifies `is_catchup_layer`, `catchup_z_bottom`, and `effective_layer_height` survive every per-layer stage. Guards the documented catch-up-layer propagation contract across every per-layer stage.

### Workstream 3 — Benchy parity and missing OrcaSlicer behavior

- [~] TASK-120 Produce a fully sliced Benchy `.gcode` with tree supports enabled as the Phase H end-to-end acceptance run.
- [ ] TASK-120a Restore top/bottom surface fill generation on the live Benchy path. Covers DEV-009.
- [ ] TASK-120b Restore support generation on the live Benchy path. Covers DEV-009.
- [ ] TASK-120c Restore seam placement on real wall-loop seam candidates. Covers DEV-009.
- [ ] TASK-120d Implement travel retraction / unretraction decisions in the live path-optimization or emit path. Covers DEV-009 and the live travel-behavior gap in DEV-023.
- [ ] TASK-135 Add Benchy regression assertions for supports, top/bottom fills, seams, and retract/unretract pairs. Supports DEV-009 acceptance evidence.

### Workstream 4 — Progress events and Python bridge coverage

- [ ] TASK-136 Add end-to-end progress-event coverage proving paint-annotation failure codes 501-504 reach the JSONL emitter on the live pipeline path. Supports DEV-010 acceptance evidence and guards the live path after DEV-019 closure.
- [ ] TASK-137 Resolve the Python `ConfigEncoding` phase gap: either add an injectable failure path and test it, or document the phase as unreachable in docs/05. Supports DEV-024 by either closing the remaining Python live-path gap or marking the phase intentionally unreachable.
- [x] TASK-138 Close the Python `Init` phase coverage gap. `crates/slicer-host/tests/python_bridge_init_phase_tdd.rs` is green.

### Workstream 5 — Governance and closure drift

- [ ] TASK-139 Remove dead `Noop*Runner` remnants or correct the Phase G closure notes so docs and source agree. Covers DEV-020.
- [ ] TASK-140 Evaluate the Architecture Acceptance Gate using docs/11 and docs/12 once TASK-120 and its subtasks are complete. Covers DEV-010 and the evidence gaps in DEV-026.
- [ ] TASK-141 Keep `docs/DEVIATION_LOG.md` synchronized with every open architectural deviation and close rows as fixes land. Supports DEV-030 and live-registry hygiene for the acceptance gate.

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
- Blocking tasks: TASK-120a, TASK-120b, TASK-120c, TASK-120d, TASK-121, TASK-123, TASK-125, TASK-127, TASK-128, TASK-129, TASK-130, TASK-136, TASK-140

### Evidence Links

- Determinism: pending Phase H parity closure
- Recoverability: pending runtime access enforcement and progress-event coverage
- Resource bounds: pending RegionMap overflow and `resolve_active_regions` guards
- Coupling control: pending manifest contract cleanup and claim transition enforcement
- Compatibility: pending acceptance-gate evaluation
- Operability: pending Benchy acceptance run and progress-event validation

### Notes

- Use `./docs/11_operational_governance_and_acceptance_gate.md` as the rubric.
- Metric thresholds are defined in `./docs/12_architecture_gate_metrics.md`.

## Blocked Tasks

- None. The remaining work is prioritized, not externally blocked.

## Governance Checklist Status

- Module/claim rollout checklist: IN PROGRESS
- Compatibility policy checks: NOT STARTED
- Release checklist: NOT STARTED
