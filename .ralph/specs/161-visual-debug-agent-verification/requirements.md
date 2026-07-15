# Requirements: 161-visual-debug-agent-verification

## Packet Metadata

- Grouped task IDs: `TASK-271`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `active`
- Aggregate context cost: `L` (XL surface: three capture subsystems + validation + docs + agent surface + verification, absorbed into one packet by decision)
- Dependencies: landed packets 157/158/159/160 (real symbols, not forward contracts); `ADR-0037` (with the packet-161 Amendment), `ADR-0038`, `ADR-0040`, `ADR-0041`.

## Problem Statement

The visual-pipeline-debug queue (commit `a453158`) promised post-stage taps for
every scheduler stage, a fail-closed bundle contract, and an agent surface.
Packets 157-160 shipped only the seven `Layer::*` arena taps plus the standalone
`final_gcode` path, left three request-selection paths silently dropping requested
output, and never authored the agent skill; the design and IR docs also drifted
(stale `SeamPlanIR`/`RegionPlan` field names, missing `SupportGeometryIR` in
`docs/02`). TASK-271 closes the queue by finishing the tap inventory across the
three capture mechanisms, making selection fail closed, adding the skill, fixing
the docs and the packet-160 cleanup, and proving the full surface is
contract-complete, deterministic, and absent from ordinary slicing.

## In Scope

- **Blackboard-read taps (ADR-0040):** capture the eight taps whose source is a
  committed whole-print Blackboard slot — MeshAnalysis (`SurfaceClassificationIR`),
  SeamPlanning (`SeamPlanIR`), SupportGeometry (`SupportGeometryIR` +
  `SupportPlanIR`), PaintSegmentation (`SliceIR`), RegionMapping (`RegionMapIR`
  joined to `SliceIR`), OverhangAnnotation (`SurfaceClassificationIR.overhang_quartile_polygons`),
  `Layer::Slice` (`SliceIR`), and `Layer::PaintRegionAnnotation`/`SlicePostProcess`
  (`SliceIR.segment_annotations`) — via a capture entry point that reads the slot
  after `prepare_prepass_context` with no per-layer execution.
- **PostPass whole-print taps (ADR-0040):** capture `PostPass::LayerFinalization`
  (finalized `Vec<LayerCollectionIR>`) and `PostPass::GCodeEmit` (`GCodeIR`) after
  the full pipeline prefix; render only selected layers; record the whole-print
  closure in the manifest.
- **Render (ADR-0037 Amendment):** RegionMapping as real `SliceIR` geometry via region-key
  join tinted by `RegionPlan`; LayerPlanning signal as an opt-in `diagnostic_overlay`
  annotation on geometry taps; no synthetic-diagram render mode; correct handling
  of mixed units (Point2 100 nm vs f32 mm) in one shared viewport.
- **Fail-closed validation (ADR-0041):** two-phase validation; reject unknown
  visualization kinds, `diagnostic_overlay` on a G-code source, and `Name`
  selectors; add an explicit `{start,end}` range variant with `deny_unknown_fields`;
  resolve `Index`/range/z-only selectors against the schedule and fail closed on
  no match — never a silent partial bundle.
- **Cleanups and docs:** remove the stale header and blanket `#![allow(dead_code)]`
  in `visual_debug_gcode.rs`; correct the `TravelMove` doc comment (mm, not 100 nm);
  add a normative `SupportGeometryIR` to `docs/02_ir_schemas.md`; correct the
  drifted tap-inventory field names, LayerPlanning row, and RegionMapping
  description in `docs/specs/visual-pipeline-debug.md`.
- **Agent surface:** add an independent `.claude/skills/visual-debug/SKILL.md`
  plus model-backed and standalone-G-code examples, cross-linked to but independent
  of `debug-pipeline`.
- **Verification:** contract tests for every implemented tap (corrected fields +
  schema version), byte determinism for both source modes including a whole-print
  PostPass tap, fail-closed negatives, and the ordinary-slice no-overhead proof.

## Out of Scope

- New module, WIT, or Blackboard API; scheduler edges; module-visible tap access
  (ADR-0037 — capture reads committed slots/arena only).
- Any change to ordinary `pnp_cli slice` behavior or to G-code emission semantics
  (the GCodeEmit tap reads the emitted `GCodeIR`; it does not alter emission).
- OrcaSlicer parity or source translation; pixel/perceptual bundle comparison;
  HTML gallery or frontend integration.
- New coordinate-system logic beyond using the existing canonical helpers; no new
  mm/unit conversion math is invented — mm and 100 nm sources are projected with
  existing helpers.
- Named-layer addressing (layers are anonymous; `Name` is rejected, not resolved).

## Authoritative Docs

- `docs/specs/visual-pipeline-debug.md` - complete design; scope, success criteria, tap inventory, bundle contract, determinism, no-overhead. Field names corrected by this packet.
- `docs/adr/0037-…` (with the packet-161 Amendment), `docs/adr/0040-…`, `docs/adr/0041-…` - the accepted decisions implemented here.
- `docs/adr/0038-visual-debug-skill-pairs-with-debug-pipeline.md` - independent skill decision.
- `docs/02_ir_schemas.md` - IR versioning rules and the `SupportGeometryIR` reconcile target.
- `docs/08_coordinate_system.md` - canonical 10,000 units/mm system; mm-vs-100 nm hazard.
- `docs/19_visual_debug.md`, `docs/17_agent_debugging.md` - agent surface and the `debug-pipeline` evidence boundary.
- `docs/07_implementation_status.md` - delegated bounded lookup for TASK-271.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` (Blackboard-read taps), `AC-2` (PostPass whole-print taps),
  `AC-3` (RegionMapping join + LayerPlanning overlay + no synthetic mode), `AC-4`
  (mixed-unit shared viewport), `AC-5` (byte determinism, both modes), `AC-6`
  (agent skill + examples), `AC-7` (ordinary-slice no-overhead), `AC-8` (doc
  reconcile), `AC-9` (cleanup + `TravelMove` doc).
- Negative: `AC-N1` (unknown kind rejected), `AC-N2` (overlay on gcode rejected),
  `AC-N3` (`Name` + malformed range rejected), `AC-N4` (range/z-only resolve or
  fail closed), `AC-N5` (`debug-pipeline` routing).
- Cross-packet impact: this packet extends the landed capture (`layer_executor.rs`),
  renderer (`visual_debug_render.rs`), and CLI validation/wiring (`visual_debug.rs`,
  `visual_debug_gcode.rs`) rather than consuming them as frozen seams; it adds no
  module-visible API.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-runtime --all-targets --test visual_debug_blackboard_tap_tdd -- blackboard_tap_capture_contracts --exact` | Pin every Blackboard-read tap's corrected source fields, schema version, and prepass-only closure. | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p slicer-runtime --all-targets --test visual_debug_postpass_tap_tdd -- postpass_whole_print_tap_contracts --exact` | Assert PostPass taps capture finalized/emitted IR after the whole-print prefix and record the closure. | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p slicer-runtime --all-targets --test visual_debug_render_tap_tdd -- regionmapping_join_and_layerplanning_overlay --exact` | Assert RegionMapping join geometry, LayerPlanning overlay, and absence of a synthetic mode. | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p slicer-runtime --all-targets --test visual_debug_render_tap_tdd -- mixed_unit_shared_viewport --exact` | Assert Point2 and mm sources share one correct viewport. | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p pnp-cli --all-targets --test visual_debug_validation_tdd -- anonymous_name_and_malformed_range_rejected --exact` | Assert fail-closed rejection of `Name` and malformed range selectors. | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p pnp-cli --all-targets --test visual_debug_validation_tdd -- range_and_zonly_selectors_resolve_or_fail_closed --exact` | Assert range/z-only resolution and fail-closed on no match. | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p pnp-cli --all-targets --test visual_debug_agent_determinism_tdd -- visual_debug_bundles_are_byte_deterministic --exact` | Compare model (incl. PostPass) and G-code manifests and PNG bytes across clean runs. | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p slicer-runtime --all-targets --test visual_debug_agent_overhead_tdd -- ordinary_slice_has_no_visual_debug_overhead --exact` | Prove ordinary slice enters no visual-debug path. | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo xtask build-guests --check` | Prove guest WASM is fresh after `slicer-ir`/`slicer-runtime` edits before attributing any guest/host test failure. | FACT clean/STALE |
| `cargo check --workspace --all-targets` | Compile all changed and test targets. | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Enforce workspace lint gate. | FACT pass/fail |
| `cargo xtask test --summary --workspace` | Acceptance-ceremony full suite (gated guest-freshness entry point); only at closure after all narrow commands pass. | FACT PASS/FAIL; full-output path |

## Step Completion Expectations

- Blackboard-read taps execute prepass only (no `LayerArena`, no per-layer
  dispatch); PostPass taps execute the whole-print prefix and render only selected
  layers, with the closure recorded in the manifest.
- Every contract assertion enumerates exact, corrected source fields (not image
  existence or counts): `SeamPlanIR.chosen_candidate.point` + `region_key` (not
  `seam_xy`), `RegionPlan.config` as a `ConfigId`, mm units for seam/branch/gcode
  geometry, and each `CapturedIr` variant's schema version.
- Validation is fail-closed in both phases; no requested visualization or layer is
  ever silently omitted from a successful bundle; the manifest `warnings` fields
  carry only rendered-with-caveats notes, never dropped selections.
- Determinism tests use clean output directories and compare complete manifest/PNG
  bytes and all ordering for both source modes, including one whole-print PostPass tap.
- The overhead proof observes the ordinary slice path without adding instrumentation
  to it.
- Editing `slicer-ir`/`slicer-runtime` invalidates guest bindgen; `cargo xtask
  build-guests --check` must run before attributing any guest/host test failure to
  this packet's changes.

## Context Discipline Notes

- The change surface spans production runtime/CLI code, IR/design docs, a skill,
  and tests; use `design.md`'s files-in-scope list and delegate broad reads.
- `docs/specs/visual-pipeline-debug.md` and `docs/02_ir_schemas.md` are large; use
  only the listed ranges and delegated symbol lookups.
- Cargo commands and test-output inspection are delegated and return FACT or
  bounded failure snippets; never absorb full test output.
