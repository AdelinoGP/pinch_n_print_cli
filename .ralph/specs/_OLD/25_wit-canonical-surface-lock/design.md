# Design: 25_wit-canonical-surface-lock

## Controlling Code Paths

1. **`wit/world-prepass.wit`** — canonical prepass world; needs `run-mesh-segmentation` and `run-paint-segmentation` signatures updated.
2. **`wit/deps/ir-types.wit`** — canonical layer-world IR handles; needs seam members added.
3. **`crates/slicer-host/tests/wit_drift_detection_tdd.rs`** — drift detection tests; needs specific signature assertions added.
4. **`crates/slicer-host/src/wit_host.rs`** — live host implementation; embedded WIT strings derived from disk.
5. **`crates/slicer-macros/src/lib.rs`** — embedded macro WIT strings; also derived from disk.
6. **`docs/03_wit_and_manifest.md`** — authoritative doc sections to synchronize.

## Architecture Constraints

- The disk WIT files must be the source of truth (per TASK-144/TASK-145 contract).
- Embedded WIT strings in `wit_host.rs` and `slicer-macros/src/lib.rs` are derived from disk files via `include_str!` or similar — they must be regenerated after disk file edits.
- The drift detection tests must assert on specific member names, not just package names.
- Doc updates must only touch sections that are authoritative WIT/manifest references.

## Implementation Approach

### Step 1: Audit current state

Before changing anything, read the current `wit/world-prepass.wit`, `wit/deps/ir-types.wit`, `crates/slicer-host/src/wit_host.rs`, and `crates/slicer-macros/src/lib.rs` to confirm what signatures are currently live in each location. The plan's Step 2 vocabulary decision gates this step — if the vocabulary is already settled in Packet 24, this step is just confirmation.

### Step 2: Update `wit/world-prepass.wit`

Change `run-mesh-segmentation` signature from old form to:
```
run-mesh-segmentation: func(mesh: mesh-object-view) -> result<_, string>
```

Change `run-paint-segmentation` signature from old form to:
```
run-paint-segmentation: func(paint: paint-segmentation-object-view) -> result<_, string>
```

Use the live signatures from `wit_host.rs`/`slicer-macros/src/lib.rs` as the reference.

### Step 3: Update `wit/deps/ir-types.wit`

Add to `perimeter-output-builder` interface (if not already present):
```
push-reordered-wall-loop: func(wall-loop: wall-loop-view) -> result<_, string>
push-resolved-seam: func(pos: point3, wall-index: u32) -> result<_, string>
```

Add to `perimeter-region-view` interface (if not already present):
```
resolved-seam: func() -> option<resolved-seam-data>
```

Note: Check current state first — some of these may already be present. Only add missing members.

### Step 4: Expand `wit_drift_detection_tdd.rs`

Add new tests or assertions within existing tests:
- `prepass_segmentation_uses_mesh_object_view`: asserts that canonical prepass world string contains `mesh-object-view` (not `mesh-id`)
- `prepass_segmentation_uses_paint_segmentation_object_view`: asserts that canonical prepass world string contains `paint-segmentation-object-view` (not `paint-region-id`)
- `seam_resolved_in_perimeter_region_view`: asserts that canonical layer-world IR handle surface contains `resolved-seam`
- `seam_push_methods_in_perimeter_output_builder`: asserts that canonical layer-world IR handle surface contains `push-reordered-wall-loop` and `push-resolved-seam`

The existing `macro_uses_canonical_dep_includes` test already validates that the macro includes the disk deps — the new tests validate the specific content.

### Step 5: Update `docs/03_wit_and_manifest.md`

Update the perimeter-region-view section to list `resolved-seam` as a readable field.
Update the perimeter-output-builder section to list `push-wall-loop`, `push-reordered-wall-loop`, and `push-resolved-seam` as builder methods.

Only update these sections — do not expand other doc sections in the same pass.

## Data and Contract Notes

- WIT package names must remain canonical: `slicer:world-layer@1.0.0`, `slicer:world-prepass@1.0.0`.
- The seam contract: `resolved-seam` on the read side (perimeter-region-view), reordered/resolved seam writes on the builder side (perimeter-output-builder).
- Doc naming must match manifest naming style: `PerimeterIR.resolved-seam` in prose.

## Risks and Tradeoffs

- **Risk**: If the disk file is updated but the embedded strings in `wit_host.rs`/`lib.rs` are not regenerated, tests will fail.
  - Mitigation: The build system regenerates embedded strings from disk files; if it doesn't, the drift tests will catch it.
- **Risk**: If `wit/deps/ir-types.wit` already has some of the seam members, adding them again would be a no-op or error.
  - Mitigation: Read the current file first (Step 3 audit).

## Open Questions

- Q1: Does `wit/deps/ir-types.wit` currently have any seam-related members?
  - **Resolution needed before Step 3**: Read the file and compare against the expected members.
- Q2: Does `build-core-modules.sh` regenerate embedded WIT strings?
  - **Resolution**: Check the script; if not, the post-wit-file-edit build must include a regeneration step.

## Locked Assumptions

1. Disk WIT files are the source of truth per TASK-144/TASK-145.
2. Embedded WIT strings are derived from disk files and regenerate on build.
3. The seam contract is: `resolved-seam` (read) / `push-resolved-seam` + `push-reordered-wall-loop` (write).
4. `docs/03_wit_and_manifest.md` updates are limited to the two specific sections.
