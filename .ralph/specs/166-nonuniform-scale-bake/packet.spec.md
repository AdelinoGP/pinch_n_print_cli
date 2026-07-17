---
status: implemented
packet: 166-nonuniform-scale-bake
task_ids:
  - TASK-272
backlog_source: docs/07_implementation_status.md
context_cost_estimate: S
---

# Packet Contract: 166-nonuniform-scale-bake

## Goal

Delete the dead `validate_non_uniform_scale` policy rejection and its `NonUniformScaleUnsupported` error variant from `crates/slicer-model-io/src/loader.rs`, and prove with tests that non-uniform-scale 3MF transforms are baked per-axis into mesh vertices and paint-data triangles by the existing transform-baking path.

## Scope Boundaries

This packet touches only `crates/slicer-model-io` (loader source and its test files) plus a downstream read-only audit for uniform-scale assumptions. No IR, WIT, scheduler, or module changes; `ObjectMesh.transform` semantics (identity after 3MF baking; full-matrix `transform_point3` downstream) are unchanged. The full scope list lives in `requirements.md`.

## Prerequisites and Blockers

- Depends on: none.
- Unblocks: OrcaSlicer-frontend fork drag-in of non-uniformly-scaled objects (fork-gaps wave-1 item 6).
- Activation blockers: none.

## Acceptance Criteria

- **AC-1. Given** a 3MF whose `<build>/<item>` transform has per-axis scale `(1.0, 2.0, 3.0)`, **when** the model is loaded via `slicer_model_io::load_model`, **then** loading returns `Ok`, and a known input vertex `(1.0, 1.0, 1.0)` is baked to `(1.0, 2.0, 3.0)` in `ObjectMesh.mesh.vertices` (within `1e-4` per axis) while `ObjectMesh.transform` remains identity. | `mkdir -p target && cargo test -p slicer-model-io --test nonuniform_scale_bake_tdd -- nonuniform_scale_bakes_vertices_per_axis 2>&1 | tee target/test-output.log | grep "^test result"`

- **AC-2. Given** the same non-uniform-scale 3MF carrying a `paint_color` sub-facet stroke, **when** loaded, **then** each stroke triangle vertex in `FacetPaintData.layers[*].strokes[*].triangles` is transformed by the same per-axis scale (a triangle vertex at `(1.0, 1.0, 1.0)` becomes `(1.0, 2.0, 3.0)` within `1e-4`). | `mkdir -p target && cargo test -p slicer-model-io --test nonuniform_scale_bake_tdd -- nonuniform_scale_bakes_paint_triangles 2>&1 | tee target/test-output.log | grep "^test result"`

- **AC-3. Given** the workspace after the deletion, **when** grepping production and test sources, **then** no occurrence of `NonUniformScaleUnsupported` or `validate_non_uniform_scale` remains under `crates/` or `modules/` (excluding `.claude/worktrees/`). | `cd F:/slicerProject/pinch_n_print && grep -rn "NonUniformScaleUnsupported\|validate_non_uniform_scale" --include="*.rs" crates modules; test $? -eq 1 && echo PASS || echo FAIL`

- **AC-4. Given** a 3MF with a uniform scale transform (e.g. `2.0` on all axes), **when** loaded, **then** the baked vertices are identical to the pre-packet behavior (vertex `(1.0, 1.0, 1.0)` bakes to `(2.0, 2.0, 2.0)` within `1e-4`) — uniform-scale behavior is unchanged. | `mkdir -p target && cargo test -p slicer-model-io --test nonuniform_scale_bake_tdd -- uniform_scale_baking_unchanged 2>&1 | tee target/test-output.log | grep "^test result"`

## Negative Test Cases

- **AC-N1. Given** the deletion of the non-uniform-scale validator, **when** the remaining loader validations run, **then** `validate_world_z_floor` still rejects an object below the floor with `ModelLoadError::WorldZBelowFloor` — no other transform/placement validation was weakened. | `mkdir -p target && cargo test -p slicer-model-io --test world_z_below_floor_tdd 2>&1 | tee target/test-output.log | grep "^test result"`

- **AC-N2. Given** the full `slicer-model-io` test suite after deleting `tests/non_uniform_scale_tdd.rs`, **when** the crate's tests run, **then** all remaining loader tests pass (zero failures), proving no collateral loader regression. | `mkdir -p target && cargo test -p slicer-model-io 2>&1 | tee target/test-output.log | grep -E "^test result" | grep -E "[1-9][0-9]* failed" && echo FAIL || echo PASS`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `mkdir -p target && cargo test -p slicer-model-io --test nonuniform_scale_bake_tdd 2>&1 | tee target/test-output.log | grep "^test result"`

## Authoritative Docs

- `docs/02_ir_schemas.md` — read only the `ObjectMesh` / `Transform3d` schema section (delegate a LOCATIONS lookup; the file is 1811 lines).
- `docs/08_coordinate_system.md` — direct read of the mm↔unit conversion rules only if the new tests assert unit-space values (they assert mm-space `Point3` floats, so likely not needed).

## Doc Impact Statement (Required)

- **`none`** — the packet deletes dead validation code and adds tests; no IR, WIT, scheduler, claim, manifest, host-service, or SDK contract changes. `ObjectMesh`/`Transform3d` schemas are untouched.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
