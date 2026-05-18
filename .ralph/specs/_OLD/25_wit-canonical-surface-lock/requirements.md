# Requirements: 25_wit-canonical-surface-lock

## Problem Statement

The canonical WIT surface (disk files under `wit/`) has drifted from the live host/macro embedded WIT strings. Specifically: (1) `wit/world-prepass.wit` uses old `mesh-id`/`paint-region-id` signatures for `run-mesh-segmentation`/`run-paint-segmentation` instead of the live `mesh-object-view`/`paint-segmentation-object-view` signatures; (2) `wit/deps/ir-types.wit` lacks the seam-related layer-world members (`push-reordered-wall-loop`, `push-resolved-seam`, `resolved-seam` on perimeter-region-view). The drift detection tests did not catch these because they did not assert on the specific signatures that changed.

## Grouped Task IDs

- TASK-144 (Consolidate host, macro, and guest codegen onto one canonical WIT source)
- TASK-145 (Normalize WIT package/version identifiers and restore missing members)

## In-Scope

- Update `wit/world-prepass.wit` to use `mesh-object-view` and `paint-segmentation-object-view` signatures for segmentation functions
- Expand `wit/deps/ir-types.wit` with seam-related members
- Expand `wit_drift_detection_tdd.rs` with specific assertions for: prepass segmentation signatures, `mesh-object-view`/`paint-segmentation-object-view` in canonical prepass world, `resolved-seam` in perimeter-region-view, `push-reordered-wall-loop`/`push-resolved-seam` in perimeter-output-builder
- Update `docs/03_wit_and_manifest.md` perimeter-region-view and perimeter-output-builder sections
- Postpass WIT/import issue surfaced during discovery — excluded from scope per plan decision

## Out-of-Scope

- Postpass WIT changes (separate follow-on)
- Non-seam prepass WIT members beyond the two segmentation functions
- Broader doc expansion beyond authoritative WIT/manifest sections

## Authoritative Docs

- `docs/03_wit_and_manifest.md`
- `docs/04_host_scheduler.md`
- `wit/world-prepass.wit`
- `wit/deps/ir-types.wit`
- `crates/slicer-host/src/wit_host.rs`
- `crates/slicer-macros/src/lib.rs`

## Acceptance Summary

After this packet lands:
1. Disk `wit/world-prepass.wit` uses `mesh-object-view` and `paint-segmentation-object-view` signatures.
2. Disk `wit/deps/ir-types.wit` includes `push-reordered-wall-loop`, `push-resolved-seam`, and `resolved-seam` on perimeter-region-view.
3. `wit_drift_detection_tdd.rs` asserts on all the specific members that slipped through.
4. `docs/03_wit_and_manifest.md` is synchronized with the current seam contract.
5. The WIT source of truth is on disk, not just in embedded strings.

## Verification

```
cargo test -p slicer-host --test wit_drift_detection_tdd -- --nocapture
cargo build --workspace
cargo clippy --workspace -- -D warnings
```
