---
status: implemented
packet: 161-visual-debug-agent-verification
task_ids:
  - TASK-271
backlog_source: docs/07_implementation_status.md
context_cost_estimate: L
copy_note: Packets 157-160 are committed and landed (HEAD 56b9acc0/a22b0305); this packet consumes their real symbols and absorbs the remaining tap-inventory, validation-hardening, cleanup, and doc-reconcile gaps versus the original design (commit a453158). Governed by ADR-0040, ADR-0041, and the ADR-0037 amendment (packet 161).
---

# Packet Contract: 161-visual-debug-agent-verification

## Goal

Close the visual-pipeline-debug queue against its original design: implement the
remaining stage taps across all three capture mechanisms, make request selection
fail closed, add the agent skill, correct the drifted spec/IR docs, and prove the
whole surface is contract-complete, deterministic, and absent from ordinary slicing.

## Scope Boundaries

This packet OWNS: the remaining visual-debug tap capture (Blackboard-read and
PostPass-whole-print classes) atop the shipped arena taps; RegionMapping rendered
via region-key join and LayerPlanning as an overlay (no synthetic-diagram mode);
two-phase fail-closed request validation with an explicit `{start,end}` range and
z-only resolution; the `visual_debug_gcode.rs` cleanup and the `TravelMove`
doc fix; the `docs/02` `SupportGeometryIR` reconcile and the drifted spec
tap-inventory field corrections; the agent skill plus examples; and contract,
determinism, and no-overhead verification of the full surface. It does NOT add a
new module, WIT, or Blackboard API (ADR-0037), change scheduler edges, alter
ordinary `pnp_cli slice` behavior, port OrcaSlicer source, or add pixel/perceptual
comparison.

## Prerequisites and Blockers

- Depends on: landed packets 157 (`crates/pnp-cli/src/visual_debug.rs`), 158/159
  (`crates/slicer-runtime/src/layer_executor.rs`, `visual_debug_render.rs`), 160
  (`crates/pnp-cli/src/visual_debug_gcode.rs`); `ADR-0037`, `ADR-0038`,
  `ADR-0040`, `ADR-0041`, and the `ADR-0037` amendment (packet 161).
- Unblocks: closure of TASK-271 and the visual-pipeline-debug packet queue.
- Activation blockers: aggregate cost is `L` (XL surface across three subsystems);
  the packet must pass independent `spec-review --preflight`, and preflight may
  require a split before activation. No `[FWD]` contracts remain — all consumed
  seams are grounded landed symbols.

## Acceptance Criteria

- **AC-1. Given** a model request selecting a Blackboard-read tap (MeshAnalysis, SeamPlanning, SupportGeometry, PaintSegmentation, RegionMapping, OverhangAnnotation, `Layer::Slice`, or `Layer::PaintRegionAnnotation`/`SlicePostProcess`), **when** capture runs, **then** it reads the committed Blackboard slot after `prepare_prepass_context` with no per-layer arena execution, and the contract test pins each tap's exact source fields (including `SeamPlanIR.chosen_candidate.point` and `region_key`, not `seam_xy`; `RegionPlan.config` as a `ConfigId`), its `CapturedIr` schema version, and its tap/layer identity, failing on a missing or renamed field. | `cargo test -p slicer-runtime --all-targets --test visual_debug_blackboard_tap_tdd -- blackboard_tap_capture_contracts --exact`
- **AC-2. Given** a model request selecting a PostPass tap (`PostPass::LayerFinalization` or `PostPass::GCodeEmit`), **when** capture runs, **then** the full pipeline prefix executes (all layers -> finalization -> `execute_postpass`), the captured IR is the finalized `Vec<LayerCollectionIR>` or emitted `GCodeIR`, only the request's selected layers are rendered, and the manifest's `executed_stage_ids`/`executed_layer_indices` record the whole-print closure. | `cargo test -p slicer-runtime --all-targets --test visual_debug_postpass_tap_tdd -- postpass_whole_print_tap_contracts --exact`
- **AC-3. Given** a RegionMapping tap, **when** it renders, **then** it joins `RegionMapIR.entries` to `SliceIR` regions on `(global_layer_index, object_id, region_id, variant_chain)` and draws real region polygons tinted by `RegionPlan`; LayerPlanning has no standalone tap and its sync/non-planar/active-region flags are available only as a `diagnostic_overlay` annotation; no synthetic-diagram render symbol exists. | `cargo test -p slicer-runtime --all-targets --test visual_debug_render_tap_tdd -- regionmapping_join_and_layerplanning_overlay --exact`
- **AC-4. Given** a bundle mixing Point2 (100 nm) sources and f32-mm sources (`SeamPlanIR` seam point, `SupportPlanIR.branch_segments`, `GCodeIR::Move`), **when** it renders, **then** every image shares one correct model-wide XY viewport: Point2 geometry uses `units_to_mm()` and mm sources are projected without 100 nm rescaling. | `cargo test -p slicer-runtime --all-targets --test visual_debug_render_tap_tdd -- mixed_unit_shared_viewport --exact`
- **AC-5. Given** identical valid model and standalone-G-code requests (the model request including a whole-print PostPass tap), **when** each bundle is generated twice into clean directories, **then** complete `manifest.json` bytes, image/warning/layer/tap ordering, PNG paths, and every PNG byte are identical for both modes. | `cargo test -p pnp-cli --all-targets --test visual_debug_agent_determinism_tdd -- visual_debug_bundles_are_byte_deterministic --exact`
- **AC-6. Given** a geometry-defect report and no timing/DAG question, **when** an agent follows the visual-debug skill, **then** the skill selects `pnp_cli visual-debug`, reads `manifest.json` before PNGs, states `debug-pipeline` is independent rather than a prerequisite, and provides a working model-mode and standalone-G-code example. | `python3 -c "from pathlib import Path; p=Path('.claude/skills/visual-debug/SKILL.md').read_text(); assert 'pnp_cli visual-debug' in p and 'manifest.json' in p and 'debug-pipeline' in p and 'independent' in p and 'model' in p and 'gcode' in p"`
- **AC-7. Given** an ordinary valid slice with visual debugging not requested, **when** it is run under the packet's repeated-run harness, **then** no visual-debug capture, allocation, serialization, rendering, process invocation, or visual-debug manifest/PNG is observed and the harness reports the ordinary-slice path as opt-out. | `cargo test -p slicer-runtime --all-targets --test visual_debug_agent_overhead_tdd -- ordinary_slice_has_no_visual_debug_overhead --exact`
- **AC-8. Given** the drifted design and IR docs, **when** they are inspected, **then** `docs/02_ir_schemas.md` carries a normative `SupportGeometryIR` definition (`support_layer_height_mm`, `support_top_z_distance_mm`, `SupportGeometryKey`) and `docs/specs/visual-pipeline-debug.md` no longer names `seam_xy`, references `chosen_candidate` and `region_key`, marks `RegionPlan.config` as a `ConfigId`, drops the standalone LayerPlanning row, and describes RegionMapping as a `SliceIR` join. | `python3 -c "from pathlib import Path; d=Path('docs/02_ir_schemas.md').read_text(); assert 'SupportGeometryIR' in d and 'support_layer_height_mm' in d and 'SupportGeometryKey' in d; s=Path('docs/specs/visual-pipeline-debug.md').read_text(); assert 'seam_xy' not in s and 'chosen_candidate' in s and 'ConfigId' in s"`
- **AC-9. Given** the packet-160 cleanup and the `TravelMove` doc drift, **when** the sources are inspected, **then** `visual_debug_gcode.rs` no longer contains the stale "not yet wired" header or a blanket `#![allow(dead_code)]`, and the `TravelMove` struct body (its field doc comments) states millimeters rather than "100 nm". | `python3 -c "from pathlib import Path; g=Path('crates/pnp-cli/src/visual_debug_gcode.rs').read_text(); assert 'not yet wired' not in g and '#![allow(dead_code)]' not in g; t=Path('crates/slicer-ir/src/slice_ir.rs').read_text(); i=t.find('pub struct TravelMove'); body=t[i:t.find(chr(10)+'}', i)]; assert '100 nm' not in body and ('millimeter' in body or ' mm' in body)"`
- **AC-N1. Given** a request naming an unknown visualization kind, **when** `validate_request` runs, **then** it fails closed before any render or bundle write with a named-field error, and no manifest or PNG is produced. | `cargo test -p pnp-cli --all-targets --test visual_debug_validation_tdd -- unknown_visualization_kind_rejected --exact`
- **AC-N2. Given** a `diagnostic_overlay` visualization against a G-code source, **when** `validate_request` runs, **then** it is rejected as a source/visualization mismatch before any render or bundle write, never silently dropped. | `cargo test -p pnp-cli --all-targets --test visual_debug_validation_tdd -- diagnostic_overlay_on_gcode_source_rejected --exact`
- **AC-N3. Given** a `LayerSelector::Name` selector or a malformed `{start,end}` range, **when** `validate_request` runs, **then** `Name` is rejected (layers are anonymous) and the range variant's `deny_unknown_fields` rejects the malformed object instead of parsing it as an empty `Detail`. | `cargo test -p pnp-cli --all-targets --test visual_debug_validation_tdd -- anonymous_name_and_malformed_range_rejected --exact`
- **AC-N4. Given** a valid `{start,end}` range and a z-only `Detail`, **when** they are resolved against the schedule (model: `LayerPlanIR.global_layers`; gcode: parsed `;Z:`), **then** each resolves to a real layer, and a selector matching no layer fails closed before any bundle write. | `cargo test -p pnp-cli --all-targets --test visual_debug_validation_tdd -- range_and_zonly_selectors_resolve_or_fail_closed --exact`
- **AC-N5. Given** an agent asks why a slice is slow, a DAG edge is missing, or a manifest is invalid, **when** the visual-debug skill is applied, **then** it routes to `debug-pipeline` with the exact `slice --instrument-stderr`, `dag`, or `module diagnose` command and says not to use `pnp_cli visual-debug`. | `python3 -c "from pathlib import Path; p=Path('.claude/skills/visual-debug/SKILL.md').read_text(); assert all(x in p for x in ('slice --instrument-stderr','pnp_cli dag','pnp_cli module diagnose','timing','DAG','do not use pnp_cli visual-debug'))"`

## Verification

- `cargo test -p slicer-runtime --all-targets --test visual_debug_blackboard_tap_tdd -- blackboard_tap_capture_contracts --exact`
- `cargo test -p slicer-runtime --all-targets --test visual_debug_postpass_tap_tdd -- postpass_whole_print_tap_contracts --exact`
- `cargo test -p slicer-runtime --all-targets --test visual_debug_render_tap_tdd -- regionmapping_join_and_layerplanning_overlay --exact`
- `cargo test -p pnp-cli --all-targets --test visual_debug_validation_tdd -- anonymous_name_and_malformed_range_rejected --exact`
- `cargo test -p pnp-cli --all-targets --test visual_debug_agent_determinism_tdd -- visual_debug_bundles_are_byte_deterministic --exact`
- `cargo clippy --workspace --all-targets -- -D warnings`

## Authoritative Docs

- `docs/specs/visual-pipeline-debug.md` - the complete design; scope, success criteria, tap inventory (field names corrected by this packet), bundle contract, determinism, and no-overhead criteria.
- `docs/adr/0037-render-pngs-from-ir-stage-taps-not-gcode-only.md` (with the packet-161 Amendment retiring the synthetic-diagram mode: RegionMapping join + LayerPlanning overlay), `docs/adr/0040-visual-debug-tap-capture-spans-three-mechanisms.md`, `docs/adr/0041-visual-debug-request-selection-fails-closed.md` - the accepted decisions this packet implements.
- `docs/adr/0038-visual-debug-skill-pairs-with-debug-pipeline.md` - independent skill pairing and evidence boundaries.
- `docs/02_ir_schemas.md` - IR versioning rules and the `SupportGeometryIR` reconcile target.
- `docs/08_coordinate_system.md` - the 10,000 units/mm canonical system and the mm-vs-100 nm hazard.
- `docs/19_visual_debug.md` - agent-facing usage guide; request authoring, manifest-first inspection, warnings, resolution cost.
- `docs/17_agent_debugging.md` - the `debug-pipeline` evidence boundary (timing/DAG/manifest) this skill must not claim.
- `docs/07_implementation_status.md` - bounded lookup of TASK-271; task ownership.

## Doc Impact Statement (Required)

- **IR-schema docs and design spec** - this packet adds a normative `SupportGeometryIR` definition to `docs/02_ir_schemas.md` and corrects drifted tap-inventory field names, the LayerPlanning row, and the RegionMapping description in `docs/specs/visual-pipeline-debug.md`. It changes no WIT, scheduler, claim, host-service, or SDK contract; the `CapturedIr` and `ValidationError` additions are internal to the runtime/CLI visual-debug path and add no module-visible API (ADR-0037).

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
