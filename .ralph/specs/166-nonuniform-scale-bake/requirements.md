# Requirements: 166-nonuniform-scale-bake

## Packet Metadata

- Grouped task IDs: `TASK-272`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `S`

## Problem Statement

The OrcaSlicer-frontend fork (fork-gaps wave-1 plan, Packet C / item 6) needs non-uniformly-scaled objects to slice. The plan framed `validate_non_uniform_scale` (`crates/slicer-model-io/src/loader.rs:2551`) as a "deliberate policy rejection"; **grounding falsified this** — the function has zero production call sites (only its definition and `tests/non_uniform_scale_tdd.rs` reference it), so the rejection never fires on the live load path. The 3MF loader already fully bakes build-item and component transforms into vertices (`apply_transform_to_mesh`, loader.rs:457, invoked at loader.rs:517 during component resolution driven from the build-item transform at loader.rs:1911-1914) and into paint strokes (`apply_transform_to_paint_data`, loader.rs:463), then sets `ObjectMesh.transform` to identity (loader.rs:228). The remaining work is therefore: (1) delete the dead validator, its `NonUniformScaleUnsupported` error variant, its `Display` arm, and its TDD test file so the false "unsupported" signal cannot be resurrected; (2) prove per-axis baking with positive tests that do not exist today; (3) audit downstream consumers for hidden uniform-scale assumptions.

## In Scope

- Delete `validate_non_uniform_scale` (loader.rs:2551-2567 as grounded, including its doc comment starting near loader.rs:2538).
- Delete the `ModelLoadError::NonUniformScaleUnsupported` variant (loader.rs:49-56) and its `Display` arm (loader.rs:81-84).
- Delete `crates/slicer-model-io/tests/non_uniform_scale_tdd.rs` (it exists solely to exercise the deleted validator).
- Remove the corresponding `[[test]]` entry from `crates/slicer-model-io/Cargo.toml` if one exists (verify; loader test binaries may be auto-discovered).
- Add `crates/slicer-model-io/tests/nonuniform_scale_bake_tdd.rs` with the three tests named in AC-1/AC-2/AC-4 (non-uniform vertex baking, non-uniform paint-triangle baking, uniform-scale regression).
- Downstream audit (read-only, delegated): confirm no consumer of `ObjectMesh.transform` or of mesh geometry extracts a single scalar scale factor or assumes uniform scale. Known consumers to check: `crates/slicer-core/src/algos/prepass_slice.rs` (`transform_point3` at lines 100, 153, 554, 771), `crates/slicer-core/src/algos/mesh_analysis.rs:120`, `crates/slicer-core/src/algos/paint_segmentation/mod.rs:1012`, `crates/slicer-core/src/algos/paint_segmentation/painted_line_collection.rs:349`. Record the audit inventory in the packet's closure log.

## Out of Scope

- Any change to `ObjectMesh`, `Transform3d`, or `transform_point3` semantics (all already full-matrix and non-uniform-capable).
- STL/OBJ loading paths (they carry no transforms).
- `validate_world_z_floor` and every other loader validation (must remain byte-identical in behavior — AC-N1).
- End-to-end slice/golden-compare of a non-uniformly-scaled model (the wave-1 plan lists this under cross-packet end-to-end verification; it needs no code from this packet beyond what the loader tests prove).
- Fork-side (OrcaSlicer frontend) changes.

## Authoritative Docs

- `docs/02_ir_schemas.md` — 1811 lines; delegate a LOCATIONS lookup for the `ObjectMesh`/`Transform3d` section only.
- `docs/08_coordinate_system.md` — direct read only if unit-space assertions become necessary (tests assert mm-space `Point3`, so expected unnecessary).

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-4` (per-axis vertex baking, per-axis paint baking, symbol deletion grep, uniform-scale regression).
- Negative: `AC-N1` (world-Z validation not weakened), `AC-N2` (no collateral loader regression).
- Cross-packet impact: none — no other wave-1 packet touches `slicer-model-io`. The wave-1 plan's end-to-end non-uniform 3MF slice check is a plan-level ceremony, not owned here.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only the gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `mkdir -p target && cargo test -p slicer-model-io --test nonuniform_scale_bake_tdd 2>&1 | tee target/test-output.log | grep "^test result"` | AC-1/AC-2/AC-4: per-axis baking proven | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cd F:/slicerProject/pinch_n_print && grep -rn "NonUniformScaleUnsupported\|validate_non_uniform_scale" --include="*.rs" crates modules; test $? -eq 1 && echo PASS || echo FAIL` | AC-3: symbols fully deleted | FACT PASS/FAIL |
| `mkdir -p target && cargo test -p slicer-model-io --test world_z_below_floor_tdd 2>&1 | tee target/test-output.log | grep "^test result"` | AC-N1: sibling validation untouched | FACT pass/fail |
| `mkdir -p target && cargo test -p slicer-model-io 2>&1 | tee target/test-output.log | grep -E "^test result" | grep -E "[1-9][0-9]* failed" && echo FAIL || echo PASS` | AC-N2: whole-crate regression sweep | FACT PASS/FAIL |
| `cargo check --workspace --all-targets` | deleted variant leaves no dangling references anywhere | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | commit gate | FACT pass/fail |

## Step Completion Expectations

The downstream audit (Step 1) must complete before the deletion step (Step 2) so a discovered uniform-scale assumption can re-scope the packet rather than surface as a post-deletion regression. No shared scratch state otherwise.

## Context Discipline Notes

`crates/slicer-model-io/src/loader.rs` is 2980 lines — never load in full; open only the grounded windows listed in `design.md`. The downstream audit is delegated (LOCATIONS + FACT), never a direct browse of `slicer-core`.
