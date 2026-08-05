---
status: implemented
packet: 25_wit-canonical-surface-lock
task_ids:
  - TASK-144
  - TASK-145
---

# 25_wit-canonical-surface-lock

## Goal

Restore and lock the canonical WIT surface so the disk WIT files under `wit/` are the unambiguous source of truth, and future drift between host/macro embedded WIT strings and the disk canonical is caught by regression tests.

## Problem Statement

The canonical WIT surface (disk files under `wit/`) has drifted from the live host/macro embedded WIT strings. Specifically: (1) `wit/world-prepass.wit` uses old `mesh-id`/`paint-region-id` signatures for `run-mesh-segmentation`/`run-paint-segmentation` instead of the live `mesh-object-view`/`paint-segmentation-object-view` signatures; (2) `wit/deps/ir-types.wit` lacks the seam-related layer-world members (`push-reordered-wall-loop`, `push-resolved-seam`, `resolved-seam` on perimeter-region-view). The drift detection tests did not catch these because they did not assert on the specific signatures that changed.

## Architecture Constraints

- The disk WIT files must be the source of truth (per TASK-144/TASK-145 contract).
- Embedded WIT strings in `wit_host.rs` and `slicer-macros/src/lib.rs` are derived from disk files via `include_str!` or similar — they must be regenerated after disk file edits.
- The drift detection tests must assert on specific member names, not just package names.
- Doc updates must only touch sections that are authoritative WIT/manifest references.

## Data and Contract Notes

- WIT package names must remain canonical: `slicer:world-layer@1.0.0`, `slicer:world-prepass@1.0.0`.
- The seam contract: `resolved-seam` on the read side (perimeter-region-view), reordered/resolved seam writes on the builder side (perimeter-output-builder).
- Doc naming must match manifest naming style: `PerimeterIR.resolved-seam` in prose.

## Risks and Tradeoffs

- **Risk**: If the disk file is updated but the embedded strings in `wit_host.rs`/`lib.rs` are not regenerated, tests will fail.
  - Mitigation: The build system regenerates embedded strings from disk files; if it doesn't, the drift tests will catch it.
- **Risk**: If `wit/deps/ir-types.wit` already has some of the seam members, adding them again would be a no-op or error.
  - Mitigation: Read the current file first (Step 3 audit).
