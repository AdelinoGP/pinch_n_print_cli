---
status: implemented
packet: 25_wit-canonical-surface-lock
task_ids:
  - TASK-144
  - TASK-145
backlog_source: docs/07_implementation_status.md
note: Tasks reopened from [x] to [ ] on 2026-04-24 after audit revealed disk WIT state did not match backlog claims.
---

# Packet Contract: 25_wit-canonical-surface-lock

## Goal

Restore and lock the canonical WIT surface so the disk WIT files under `wit/` are the unambiguous source of truth, and future drift between host/macro embedded WIT strings and the disk canonical is caught by regression tests.

## Scope Boundaries

- **In scope:** `wit/world-prepass.wit` sync with live `wit_host.rs`/`slicer-macros/src/lib.rs` signatures; expansion of `wit_drift_detection_tdd.rs` to cover pre-pass segmentation signatures and seam-related layer-world members; `docs/03_wit_and_manifest.md` perimeter-region-view and perimeter-output-builder sections update.
- **Out of scope:** Postpass WIT changes surfaced during discovery (handled separately); non-seam prepass WIT members beyond the two segmentation functions; broader doc expansion.

## Prerequisites and Blockers

- **Unblocks:** None (this packet closes independently; it is a parallel track).
- **Prior state:** TASK-144 and TASK-145 were marked complete in docs/07 but disk WIT audit revealed the prepass segmentation surface was not yet updated — tasks have been reopened in docs/07_implementation_status.md.

## Acceptance Criteria

- **Given** the canonical `wit/world-prepass.wit` file, **when** `run-mesh-segmentation` is declared in that file, **then** its signature uses `mesh-object-view` (not raw `mesh-id`). | `grep -A5 'run-mesh-segmentation' wit/world-prepass.wit`
- **Given** the canonical `wit/world-prepass.wit` file, **when** `run-paint-segmentation` is declared in that file, **then** its signature uses `paint-segmentation-object-view` (not raw `paint-region-id`). | `grep -A5 'run-paint-segmentation' wit/world-prepass.wit`
- **Given** the disk canonical `wit/deps/ir-types.wit`, **when** `perimeter-output-builder` is declared, **then** it contains `push-reordered-wall-loop` and `push-resolved-seam` members. | `grep -E 'push-reordered-wall-loop|push-resolved-seam' wit/deps/ir-types.wit`
- **Given** the disk canonical `wit/deps/ir-types.wit`, **when** `perimeter-region-view` is declared, **then** it exposes `resolved-seam` as a read member. | `grep 'resolved-seam' wit/deps/ir-types.wit`
- **Given** `crates/slicer-host/tests/wit_drift_detection_tdd.rs`, **when** the test suite runs, **then** it asserts that `mesh-object-view` appears in the canonical prepass world string and `resolved-seam`, `push-reordered-wall-loop`, and `push-resolved-seam` appear in the canonical layer-world IR handle surface. | `cargo test -p slicer-host --test wit_drift_detection_tdd -- --nocapture 2>&1 | tail -30`
- **Given** `docs/03_wit_and_manifest.md` perimeter-region-view section, **when** the doc is rendered, **then** it lists `resolved-seam` as a readable field on the perimeter-region-view. | `grep -A20 'perimeter-region-view' docs/03_wit_and_manifest.md | head -25`
- **Given** `docs/03_wit_and_manifest.md` perimeter-output-builder section, **when** the doc is rendered, **then** it lists `push-wall-loop`, `push-reordered-wall-loop`, and `push-resolved-seam` as builder methods. | `grep -A20 'perimeter-output-builder' docs/03_wit_and_manifest.md | head -25`

## Negative Test Cases

- **Given** a future edit to `crates/slicer-host/src/wit_host.rs` that changes a prepass segmentation function signature without updating `wit/world-prepass.wit`, **when** `wit_drift_detection_tdd` runs, **then** it fails with a drift error. | `cd crates/slicer-host && cargo test -p slicer-host --test wit_drift_detection_tdd prepass_signature -- --nocapture 2>&1 | tail -20`
- **Given** a future edit to `crates/slicer-host/src/wit_host.rs` that adds a seam builder method without updating `wit/deps/ir-types.wit`, **when** `wit_drift_detection_tdd` runs, **then** it fails with a seam-member drift error. | `cd crates/slicer-host && cargo test -p slicer-host --test wit_drift_detection_tdd seam_members -- --nocapture 2>&1 | tail -20`

## Verification

- `cargo test -p slicer-host --test wit_drift_detection_tdd -- --nocapture`
- `cargo build --workspace`
- `cargo clippy --workspace -- -D warnings`

## Authoritative Docs

- `docs/03_wit_and_manifest.md`
- `docs/04_host_scheduler.md`
- `wit/world-prepass.wit`
- `wit/deps/ir-types.wit`

## OrcaSlicer Reference Obligations

- None.
