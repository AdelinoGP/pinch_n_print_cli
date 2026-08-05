---
status: implemented
packet: 176-support-preview-verb
task_ids:
  - TASK-291
---

# 176-support-preview-verb

## Goal

Add a `pnp_cli support-preview --input <3mf> --output <path>` verb that runs only the prepass pipeline prefix via the existing `prepare_prepass_context`, reads the committed `SupportGeometryIR` off the blackboard, and writes per-layer support polygons (contour + holes, in mm) plus layer z as a versioned, fork-facing JSON contract — no per-layer module execution, no G-code.

## Problem Statement

The OrcaSlicer-fork frontend needs a support overlay while the user paints — it must ask PNP "where would supports land for this model + config" and get geometry back fast, without paying for walls, infill, path generation, or G-code. PNP already has exactly the needed seam: `prepare_prepass_context` runs only the shared prefix through Tier 1 (prepass), and the `PrePass::SupportGeometry` stage commits `SupportGeometryIR` (coarse per-layer outline `ExPolygon`s) to the blackboard — the same slot the visual-debug `PrePass::SupportGeometry` blackboard tap reads. What is missing is a CLI verb that exposes that slot as a stable, documented JSON contract in mm.

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- Use `slicer_ir::units_to_mm(i64) -> f32` (slice_ir.rs:71) / `Point2::to_mm()` (slice_ir.rs:98) for the conversion — never a hand-rolled factor. Emit as f64 for JSON.
- `SupportGeometryIR.entries: HashMap<SupportGeometryKey, Vec<ExPolygon>>` (slice_ir.rs:1175-1185) — HashMap iteration order is nondeterministic; the handler MUST sort (by `layer_index`, then `object_id`, then `region_id`) before emission so output is byte-deterministic across runs.
- `SupportGeometryKey.global_support_layer_index == u32::MAX` is the intermediate-model-resolution sentinel (slice_ir.rs:1160-1168) — must be skipped and counted, never used to index `plan.global_layers` (it would panic).
- Latency contract: the verb runs Tier 1 (prepass) only — module loading, config resolution, plan build, slicing, mesh analysis, paint segmentation, support geometry. It never constructs a per-layer closure, never touches `execute_captured_stages` / `execute_postpass*`, so walls, infill, support path generation, path optimization, and G-code emit are all skipped by construction. `prepare_prepass_context` additionally skips the 14-pass startup DAG validation and thumbnail/CONFIG_BLOCK wiring (documented on the fn, `run.rs:729-737`).

## Data and Contract Notes

- IR/manifest contracts: read-only consumers of `SupportGeometryIR` (schema 1.x) and `ExecutionPlan.global_layers`; no IR change, no schema bump.
- WIT boundary: untouched.
- Determinism/scheduler constraints: output must be byte-deterministic for identical input+config (sorted iteration, `to_string_pretty` stable field order via struct definition) — the fork may cache previews by content hash.
- Fork-facing JSON contract is versioned independently (`schema_version: "1.0.0"` — a document version, not any IR's version; additive fields bump minor).

## Locked Assumptions and Invariants

- `SupportGeometryIR` coarse outlines are the preview product; the fork accepts geometry-stage fidelity (no per-layer support paths, no interface split) — locked by the approved plan's "run the pipeline through the support stage ONLY".
- `GlobalLayer.z` is millimeters (f32) — grounded against constructors in `crates/slicer-ir/tests/ir_tests.rs` (`z: 0.2`) and emit's mm-based layer z usage.
- Absent/empty support geometry is success (`layers: []`), never an error.

## Risks and Tradeoffs

- Fixture risk: `bridge_support_enforcers.3mf` may not drive the support-geometry module without extra config keys; Step 1's dispatch locks the fixture + keys before tests are written (fallback: `bridge.obj` with explicit `--config`). The ACs assert schema/conversion, not specific polygon shapes, so fixture substitution does not weaken them.
- Latency: prepass still slices the whole model; acceptable for paint-time use (no Tier 2/3), but the contract doc must state the cost is model-size-dependent so the fork debounces calls.
- Coarse outlines may differ from final support paths (post-plan trimming happens in Tier 2); docs/20 must state the preview is approximate by design.
