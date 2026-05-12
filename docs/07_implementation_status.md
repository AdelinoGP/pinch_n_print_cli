# Implementation Status

Last updated: 2026-05-08

## Status Markers

- `[x]` complete
- `[~]` partially complete / in progress
- `[ ]` not started

## Program Status

- MVP status: COMPLETE.
- Historical implementation phases A through G are complete.
- This document now tracks the remaining work: OrcaSlicer feature parity, architecture-conformance cleanup, and acceptance-gate evidence.
- Phase H acceptance gate review unblocked 2026-05-08 by packet `27_phase-h-final-validation` (TASK-120 closed). Remaining Phase H items (TASK-120c live seam placement, TASK-130 macro segmentation bridge [x] closed 2026-05-08 by packet 43-rev1) are tracked separately and do not block acceptance-gate review entry.

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
- Known live-output gaps on the Benchy path: top/bottom surface infill, support structures, seam placement on real wall loops, travel retraction / unretraction behavior, and OrcaSlicer-compatible GCode comment metadata required for native preview visualization.
- Architecture Acceptance Gate is blocked on the remediation backlog below.

## Active Remediation Backlog

- Coverage check (2026-04-17): every open `DEV-###` row in `docs/DEVIATION_LOG.md` is now owned by at least one task below.
- Parent tasks marked `[~]` are umbrella items retained for continuity; they close only when all listed child tasks land.

### Workstream 1 — Manifest and contract conformance

- [x] TASK-121 Populate `[ir-access]` for all 17 core-module manifests per docs/01 Stage I/O Contract. Covers DEV-002. Must turn `core_module_ir_access_contract_tdd.rs` green.
- [x] TASK-122 Populate `[config.schema]` for all 17 core-module manifests so the `config-schema` CLI returns real per-module schemas. Covers DEV-008.
- [x] TASK-123 Feed `ModuleAccessAudit` from every live execution path and pass populated `access_audits` into validation. Covers DEV-003.
- [x] TASK-123a Record prepass execution audits and plumb them into `DagValidationRequest.access_audits`. Covers DEV-003.
- [x] TASK-123b Record per-layer execution audits and plumb them into `DagValidationRequest.access_audits`. Covers DEV-003.
- [x] TASK-123c Record postpass execution audits and add a live-path regression proving populated audits reach validation. Covers DEV-003.
- [x] TASK-124 Enforce undeclared runtime read/write faults at the WIT boundary and add a negative harness for layer-time undeclared access. Continues DEV-003 after TASK-123 lands.
- [x] TASK-125 Enforce the docs/01 Claim Transition Matrix for non-transitionable claims (`perimeter-generator`, `seam-placer`, `layer-planner`, `mesh-analyzer`). Covers DEV-004 and must turn `claim_transition_matrix_tdd.rs` green.
- [x] TASK-126 Fix `WriteConflict.orderable` so it reports `true` only when ordering can actually resolve the pair; add both positive and negative semantics tests. Scheduler conflict-ordering cleanup required for the docs/04 contract.
- [x] TASK-144 Consolidate host, macro, and guest codegen onto one canonical shared WIT source rooted in `wit/`. Covers DEV-014. **Closed 2026-04-24 — disk `wit/world-prepass.wit` carries `mesh-object-view` and `paint-segmentation-object-view` signatures; drift detection tests confirm all seam-related members present; packet `25_wit-canonical-surface-lock` implemented.**
- [x] TASK-145 Normalize WIT package/version identifiers and restore missing members across the canonical WIT surface, generated bindings, schema constants, and test guests; add drift-detection regression coverage. Continues DEV-014. Added `wit_drift_detection_tdd.rs` (now 19 tests). **Closed 2026-04-24 — all prepass segmentation signature assertions and seam-member assertions present and green; packet `25_wit-canonical-surface-lock` implemented.**
- [x] TASK-146 Add host-side `wit_world` allowlist validation using the canonical identifiers and reject mismatched manifests at startup. Covers the validation slice of DEV-014 and DEV-026. Added `validate_wit_world` in `manifest.rs`; added `wit_world_mismatch` and `wit_world_major_version_mismatch` tests in `manifest_ingestion_tdd.rs`.
- [x] TASK-149 Widen the WIT types so `ExtrusionRole::Custom(String)`, `PaintSemantic::Custom(String)`, and `WallFeatureFlags.custom` can cross the boundary losslessly. Covers DEV-016.
- [x] TASK-150 Update host, macro, and guest converters to preserve the widened custom payloads and add round-trip WIT regression tests. Continues DEV-016.

### Workstream 2 — Runtime correctness and scheduler guarantees

- [x] TASK-127 Enforce the non-planar Z envelope `[layer.z, layer.z + effective_layer_height]` at output-commit boundaries. Covers DEV-005.
- [x] TASK-128 Resolve prepass segmentation input-shape gaps on the macro/WIT path so segmentation modules stop receiving hollow SDK inputs. Covers DEV-025.
- [x] TASK-128a Provide usable `MeshSegmentation` inputs on the macro path by sourcing real geometry for `MeshObjectView` instead of object-id-only shells. Covers DEV-025.
- [x] TASK-128b Provide usable `PaintSegmentation` inputs on the macro path, including transform matrices, paint layers, and participating layer indices. Continues DEV-025.
- [x] TASK-129 Close the remaining non-segmentation WIT-boundary gaps on live execution paths. Covers DEV-006.
- [x] TASK-129a Pass real postpass GCode command lists into `dispatch_postpass_gcode_call` and add coverage for per-command content crossing the WIT boundary. Covers DEV-006.
- [x] TASK-129b Add live-path boundary coverage for layer-world deep-copy behavior outside native fallback code. Continues DEV-006.
- [x] TASK-129c Add live-path boundary coverage for finalization-world deep-copy behavior outside native fallback code. Continues DEV-006.
- [x] TASK-130 Finish the `#[slicer_module]` prepass segmentation bridge for macro-authored modules. Covers DEV-025. **Closed 2026-05-08 — packet 43-rev1 macro-prepass-segmentation-output-drain; full PaintSegmentation drain wired; DEV-025 mismatch 3 closed.**
- [x] TASK-130a Drain `PaintSegmentationOutput` back through WIT `push-paint-region` so macro-authored modules can emit paint regions without hand-written `wit-guest` glue. Covers DEV-025. **Closed 2026-05-08 — packet 43-rev1; drain loop in `build_prepass_world_glue` iterates `sdk_output.regions()` and calls `_output.push_paint_region`; Step 2.5 fixed macro inline-WIT scope; Step 2.6 fixed host `layer-idx` alias drift.**
- [x] TASK-130b Add end-to-end macro-path regression tests proving `MeshSegmentation` and `PaintSegmentation` round-trip real data through WIT. Continues DEV-025. **Closed 2026-05-08 — packet 43-rev1; `macro_paint_segmentation_output_roundtrip_tdd.rs` (AC-5, AC-6, AC-7, AC-8, AC-9, AC-14, NEG-1, NEG-2) and multi-stage fixture tests all pass.**
- [x] TASK-130c Widen paint-region transport (SDK ExPolygon-bearing, WIT typed paint-value-input variant, host harvest 1:1) so paint regions carry hole-bearing polygons and Custom/typed values without coercion. Covers DEV-025. **Closed 2026-05-08 — packet 42 paint-region-transport-widening acceptance; all 10 ACs + 4 negative tests pass; DEV-025 mismatches 4 (paint value channel string-coerced) and 5 (SDK paint-region polygons hole-blind) closed; mismatch 3 remains open (closes in packet 43).**
- [x] TASK-131 Add a regression guard for the documented `resolve_active_regions` O(1) contract. Scheduler performance guard needed for runtime-budget evidence.
- [x] TASK-132 Add structured RegionMap overflow coverage for the 1000-entry cap, including top-contributor and remediation messaging. Hardens the existing bounds path needed for DEV-026 evidence.
- [x] TASK-133 Add a pool-behavior test proving `layer_parallel_safe = false` serializes concurrent WASM acquisition. Scheduler concurrency guard for the docs/04 instance-pool contract.
- [x] TASK-134 Add a catch-up-layer propagation test that verifies `is_catchup_layer`, `catchup_z_bottom`, and `effective_layer_height` survive every per-layer stage. Guards the documented catch-up-layer propagation contract across every per-layer stage.
- [x] TASK-147 Implement live mesh-data wiring for `raycast_z_down` and cover hit/miss semantics across the WIT worlds. Covers DEV-015.
- [x] TASK-148 Implement `surface_normal_at` and `object_bounds` on the same mesh-query backing surface, replacing the current stub/trap behavior with tested results. Continues DEV-015.
- [x] TASK-157 Add fixture-level integration coverage for non-identity object transforms so transformed STL/3MF inputs prove correct world-space Z behavior through planning. Covers DEV-027. **Closed 2026-04-21 — 5 integration tests: translated_object_z_floor_tdd, rotated_object_world_extent_tdd, transformed_model_world_z_tdd, multi_object_transform_world_z_tdd, non_uniform_scale_tdd.**
- [x] TASK-158 Promote world-space Z extent to one canonical derived contract surface, either first-class IR or explicitly documented config-only behavior, then regression-lock transformed-object behavior. Continues DEV-027. **Closed 2026-04-21 — Option A IR field added (ObjectMesh.world_z_extent); world_z_canonical_surface_tdd + world_z_below_floor_tdd regression tests added.**
- [x] TASK-166 Resolve per-object/per-region config from CLI JSON or print profile in the `RegionMapIR` builder (`dispatch.rs` §`PrePass::RegionMapping`) instead of unconditionally calling `ResolvedConfig::default()`. Thread resolved config through to `RegionPlan` entries so `top_shell_layers`, `bottom_shell_layers`, and user-tunable fields reach their assigned values end-to-end. Add CLI integration tests proving `--config '{"top_shell_layers": N}'` propagates through to produced IR and is consumed by downstream stages. Covers DEV-040. Unblocks packets 36 (bridge-detector config), 37 (per-claim module selection), and future tunable-behavior work. **Closed 2026-05-04 by packet 35a (resolved-config-propagation).**
- [x] TASK-167 Implement bridge detection (Orca parity) in packet 36: `assemble_bridge_areas` function detecting bridging from downward-facing facets with anchors; `execute_mesh_analysis_with` plumbs `bridge_regions` to the live path; `rectilinear-infill` emits `BridgeInfill` paths at `bridge_orientation_deg`. Closes DEV-035 (any-vertex-in-polygon approximation) and DEV-036 (empty bridge_regions in production). **Closed 2026-05-05.** Reopened 2026-05-05 by TASK-168 (packet 36-rev1) — closure was premature; algorithms were heuristic stubs (bbox cluster from TopSurface; bbox direction; AABB footprint; centroid set-difference). Will be re-closed by packet 36-rev1 acceptance. Re-closed 2026-05-05 by packet 36-rev1 / TASK-168 with the correct algorithmic implementation (down-facing cluster seed; half-edge anchor-run anchor_width and bridge_direction; facet-projection xy_footprint; real polygon-set partition_expoly_by_bridges; constant-sourced schema versions; rotated-bridge fixtures).
- [x] TASK-168 Implement packet 36-rev1 bridge detector remediation: rewrite mesh-adjacency analysis (down-facing cluster seed, half-edge anchor-width run, facet-projection xy_footprint, longest-anchor-edge bridge_direction); add CURRENT_*_SCHEMA_VERSION constants and validate_polygon_simplicity helper; switch OffsetJoinType to Miter; replace centroid set-difference with real polygon ops; rewrite tests with rotated fixtures; reopen DEV-035/036 and re-close with correct rationale; register DEV-041 (slicer-helpers boundary amendment). Closed 2026-05-05 — all 14 AC commands pass; `cargo test --workspace`, `cargo clippy --workspace -- -D warnings`, and `./modules/core-modules/build-core-modules.sh` green; docs/02_ir_schemas.md and docs/13_slicer_helpers_crate.md updated; DEV-035, DEV-036, DEV-041 re-closed with correct rationale.
- [x] TASK-167 Implement fill-role-claims for packet 37: register `claim:top-fill`, `claim:bottom-fill`, `claim:bridge-fill`, and `claim:sparse-fill` in the claim catalog (`docs/03_wit_and_manifest.md`); add per-claim module selection in the infill pipeline so each role can be satisfied by a dedicated module. Closes DEV-037. **Closed 2026-05-06 — packet 37 fill-role-claims Step 5 acceptance.**
- [x] TASK-169 Implement packet 38-rev1 top-surface-ironing: object-scope ironing module emitting `BlockKind::Ironing` strokes at `PostPass::LayerFinalization` with reduced-flow rate, configurable spacing, top-surface-only detection, and config validation. Closes the predecessor packet 38_top-surface-ironing (which placed the module at the wrong stage `Layer::InfillPostProcess` and failed acceptance). **Closed 2026-05-07 — packet 38-rev1 top-surface-ironing Step 5 acceptance; 8 module-level tests pass (AC-1..5, NEG-1..3); benchy E2E (AC-6) confirms `;TYPE:Ironing` and `;TYPE:Top surface` both present in Benchy G-code; `cargo test --workspace --no-fail-fast` green (1727 passed, 0 failed); `cargo clippy --workspace -- -D warnings` clean.**
- [x] TASK-170 Implement packet 39_stable-entity-ids: foundation refactor introducing stable per-layer-monotonic `entity_id: u64` on `LayerCollectionIR.ordered_entities` entries; migrate `TravelMove` anchor from positional index to `entity_id`. Producers issue IDs at construction; `gcode_emit` resolves travel anchors via per-layer `HashMap<u64, usize>` lookup. Pure refactor — zero G-code byte change. Foundation for packet `40_finalization-mutation-builder`. **Closed 2026-05-07 — packet 39_stable-entity-ids Step 5 acceptance; all 7 TDD tests pass (3 new test files); workspace-wide fixture sweep confirms G-code byte-identical; `cargo test --workspace --no-fail-fast` green; `cargo clippy --workspace -- -D warnings` clean.**
- [x] TASK-171 Promote `FinalizationOutputBuilder` from push-only to a true mutation builder (push_with_priority, modify_entity, sort_layer_by, insert_synthetic_layer_after) backed by `ExtrusionRole::default_priority()` per-role table. Replace `dispatch.rs:2891` `splice(0..0, ...)` prepend with extend + ID-stamp + stable-sort by priority + apply recorded mutations. Migrate `top-surface-ironing` to land entities at `ExtrusionRole::Ironing.default_priority()` so ironing G-code emits AFTER top-fill within each top layer (closes the deferred print-order concern from Packet 38-rev1). Backwards-compatible: skirt-brim's existing call site preserved via `push_entity_to_layer → push_entity_with_priority(..., 0)` alias. Foundation for future PostPass mutation modules (`SequentialPrintOrder`, `MinLayerTimeEnforcer`, `FlushVolumeCalculator`, `PrimeTower`). **Closed 2026-05-07 — packet 40_finalization-mutation-builder Step 5 + Step 6 acceptance; AC-1..AC-8 + NEG-1..NEG-3 PASS, ironing emits after top-fill in Benchy.**
- [x] TASK-172 Refactor `FinalizationOutputBuilder`'s mutation methods (`modify_entity`, `sort_layer_by`, `insert_synthetic_layer_after`) from closure-based APIs to serializable-enum-based APIs (`EntityMutation`, `SortKey`, `SyntheticLayerData`) so they round-trip cleanly across the WIT boundary. Wire the `slicer-macros` `run_finalization` drain-back loop to forward `merge_ops` via WIT. Add a WASM-side round-trip test guest at `test-guests/finalization-mutation-roundtrip-guest/` and host-side end-to-end test at `crates/slicer-host/tests/finalization_mutation_roundtrip_tdd.rs` proving a guest's `modify_entity(layer, id, EntityMutation::SetSpeedFactor(0.5))` actually mutates the host-side IR. Closes `DEV-041`. Establishes the WIT round-trip contract that future PostPass mutation modules (`SequentialPrintOrder`, `MinLayerTimeEnforcer`, `FlushVolumeCalculator`, `PrimeTower`) will consume. **Closed 2026-05-08 — packet 41_finalization-mutation-enum-refactor Step 5 + Step 6A acceptance; mutation enums (EntityMutation, SortKey, SyntheticLayerData) serialized via WIT; slicer-macros drain-back wired for merge_ops round-trip; AC-1..AC-5 + NEG-1..NEG-3 pass; test guest and E2E host tests green.**

- [x] TASK-180 Wire 3MF fuzzy_skin paint metadata through host loader (covers DEV-044). **Closed 2026-05-11 — Packet 50.**

### Workstream 3 — Benchy parity and missing OrcaSlicer behavior

- [x] TASK-119 Restore OrcaSlicer-identical GCode comment contracts on the live emit path so the original OrcaSlicer preview and visualization surfaces remain fully functional. Covers DEV-009 and must land before TASK-120. **Closed 2026-04-21 — `orca_type_label` in `gcode_emit.rs:77` canonicalizes all `;TYPE:` spelling; layer headers at `:137`; seam preservation via entity-order iteration; whole-postpass regression in `postpass_gcode_emit_contract_tdd.rs`. All 6 acceptance tests pass.**
- [x] TASK-119a Enumerate the OrcaSlicer-native GCode comment contract required for preview compatibility (`;LAYER_CHANGE`, `;Z:`, `;HEIGHT:`, `;TYPE:`, object/feature markers, and other viewer-required tokens) and codify it as one canonical emit spec with shared constants. Covers DEV-009. **Closed 2026-04-21 — `orca_type_label` and `orca_layer_headers` helpers in `gcode_emit.rs` serve as the canonical spec surface.**
- [x] TASK-119b Emit the canonical OrcaSlicer comment sequence with matching spelling, ordering, and boundary placement across the live host/finalization GCode path instead of partial or ad hoc annotations. Covers DEV-009. **Closed 2026-04-21 — `DefaultGCodeEmitter::emit_gcode` emits headers, TYPE labels, and seam-preserved paths from the live host path.**
- [x] TASK-119c Add compatibility regressions proving emitted `.gcode` preserves OrcaSlicer's native layer navigation, feature coloring, and toolpath visualization semantics end-to-end. Supports DEV-009 acceptance evidence. **Closed 2026-04-21 — `postpass_gcode_emit_contract_tdd.rs` locks whole-postpass byte-deterministic round-trip; `gcode_emit_tdd.rs` covers all 6 acceptance criteria.**
- [x] TASK-120 Produce a fully sliced Benchy `.gcode` with tree supports enabled as the Phase H end-to-end acceptance run. **Closed 2026-05-08 — packet `27_phase-h-final-validation` Phase H final validation gate. Focused 6-test matrix green (96 tests passed: `core_module_ir_access_contract_tdd` 7, `pipeline_tdd` 18, `wit_drift_detection_tdd` 20, `live_layer_support_tdd` 14 incl. all four live-dispatch tests, `live_seam_path_tdd` 7, `benchy_end_to_end_tdd` 30 incl. `benchy_with_support_enabled` / `benchy_support_marker_present` / `benchy_support_deterministic`); `cargo build --workspace` and `cargo clippy --workspace -- -D warnings` clean. TASK-120b live-evidence citation and Workstream 3 Benchy-with-tree-support test names verified intact (added by packet 26). WASM artifact rebuild (`./modules/core-modules/build-core-modules.sh`) is the recommended pre-acceptance step and runs out-of-band on a Git Bash / POSIX shell; checked-in `.wasm` artifacts under `modules/core-modules/` were the basis for this validation. Phase H acceptance gate review readiness unblocked; remaining Phase H items (TASK-120c, TASK-130) tracked separately.**
- [x] TASK-120a Restore top/bottom surface fill generation on the live Benchy path. Covers DEV-009.
- [x] TASK-164 Wire `is_top_surface`, `is_bottom_surface`, `is_bridge` into `SlicedRegion` at slice time and expose them via the `slice-region-view` WIT resource so live infill can emit `TopSolidInfill`/`BottomSolidInfill`/`BridgeInfill` roles. Bumps `SliceIR.schema_version` 1.0.0 → 1.1.0. Covers DEV-009. **Closed 2026-05-03 — packet `12-rev1_external-surface-classification-at-slice`; `classify_region_surfaces` in `layer_slice.rs:120`; WIT accessors added at `ir-types.wit:72-74`; macro adapter and gyroid-infill wired; 7 acceptance tests + benchy ACs 5+6 pass. Open follow-ups: DEV-035 (polygon intersection), DEV-036 (bridge detection), WASM rebuild.**
- [x] TASK-165 Multi-layer top/bottom solid-fill window via per-region `top_shell_layers`/`bottom_shell_layers` config keys. Extend `classify_region_surfaces` to compute Z window as a sum of next/prev N layer Zs instead of single-layer window. Thread `RegionMapIR` through `execute_layer_slice` and look up N per `(layer, object, region)`. Defaults to 3 layers (matches codebase convention, differs from OrcaSlicer). **Closed 2026-05-04 — packet `35_multi-layer-top-bottom-thickness`; multi-layer Z-window logic in `layer_slice.rs:135–160`; window truncation at object extent; all 8 acceptance tests green (AC 1–6 + NEG 1–2); `cargo test --workspace` includes pre-existing tree-support IR-access test failure unrelated to this packet; `cargo clippy` and `cargo build` clean. Follows packet `12-rev1`; unblocks packets 36 (bridge-detector), 37 (fill-role-claims), 38 (top-surface-ironing).**
- [x] TASK-120b Restore support generation on the live Benchy path. Covers DEV-009. **Re-closed 2026-04-24 (packet `26_live-support-module-evidence`) — `live_support_generation_tdd.rs` now carries two tiers: Section A keeps the original `commit_layer_outputs_for_test` commit-path tests, Section B adds real live-dispatch evidence loading the checked-in `tree-support.wasm` and `traditional-support.wasm` through `WasmRuntimeDispatcher::dispatch_layer_call` + `LayerStageRunner::run_stage` and asserting non-empty `SupportIR.support_paths` with `ExtrusionRole::SupportMaterial` (`tree_support_live_dispatch_produces_non_empty_support_ir`, `traditional_support_live_dispatch_produces_non_empty_support_ir`), determinism across repeated runs (`support_deterministic_across_repeated_runs`), and SupportEnforcer-over-SupportBlocker paint precedence at the WIT boundary (`support_enforcer_blocker_paint_precedence`). Benchy acceptance is also wired: `benchy_end_to_end_tdd.rs` adds `benchy_with_support_enabled`, `benchy_support_marker_present`, `benchy_support_deterministic`, `benchy_no_support_marker_when_disabled`, and `tree_support_active_holder` against a filtered module-dir fixture and `resources/test_config/benchy-tree-support.json`. Original 2026-04-21 closure on the synthetic commit-helper tier is superseded by this real live-dispatch evidence.**
- [~] TASK-120c Restore seam placement on real wall-loop seam candidates. Covers DEV-009. **Reopened 2026-04-21 — packet `22_live-seam-contract-repair` replaces the incomplete closure from packet 14-rev1. Remaining live-path gaps: `seam-placer` still reads `resolved_seam` instead of selecting from `PerimeterIR.regions[*].seam_candidates`, `convert_perimeter_output` broadcasts one chosen seam across origin buckets, and rotated-wall replacement can erase sibling walls unless the full region wall set is re-emitted.**
- [x] TASK-120d Restore live Benchy travel behavior on the path-optimization or emit path. Covers DEV-009 and the travel-behavior slice of DEV-023. **Closed 2026-04-24 — packet `15_live-travel-retraction-policy` makes `path-optimization-default` the canonical retract/no-retract decision surface. Inter-region travel emits Retract + Move + ZHop (if configured) + Unretract; intra-region travel is suppressed. Host dispatch accepts `GcodeCommandCollected::Retract/Unretract` into `LayerArena.deferred_retracts`. All 7 acceptance tests green; clippy clean.**
- [x] TASK-120d1 Decide where retraction policy lives (`path-optimization-default` vs emit path) and implement retract/no-retract decisions for live travel moves. Covers DEV-009. **Closed with TASK-120d — policy lives in `path-optimization-default`; `DefaultGCodeEmitter` is serialization-only.**
- [x] TASK-120d2 Emit matching retract/unretract pairs, z-hop interactions, and Benchy regression assertions for the chosen travel-policy surface. Covers DEV-009. **Closed with TASK-120d — `travel_policy_tdd.rs` covers external/internal/z-hop/determinism; `live_travel_policy_tdd.rs` covers host dispatch deferred-queue routing and orphan-free no-retract path.**
- [x] TASK-135 Add Benchy regression assertions for supports, top/bottom fills, seams, and retract/unretract pairs. Supports DEV-009 acceptance evidence. **Closed 2026-05-08 — packet `21_benchy-acceptance-evidence` lands all six feature-evidence assertions in `crates/slicer-host/tests/benchy_end_to_end_tdd.rs` (support, top/bottom surface, balanced retract/unretract, live seam before emit, targeted missing-family diagnostics, determinism). Producer packet chain satisfied: 11 (emission contract), 12-rev1 (external surface classification), 13 (live support), 22 (live seam contract repair, supersedes 14/14-rev1), 15 (live travel/retraction). Seam evidence split across packet 22 (current live path) and packet 23-rev1 (PrePass seam-planning) per design. All six acceptance tests green; unblocks TASK-140 acceptance-gate evaluation.**
- [x] TASK-142 Port `SkirtBrim` live geometry from legacy `process()` into `run_finalization()` using `LayerCollectionView` and `FinalizationOutputBuilder`. Covers DEV-013. **Closed 2026-04-25 — packet `16_skirt-brim-finalization-live-path` implements `run_finalization()` with `LayerCollectionView` bbox discovery and `FinalizationOutputBuilder::push_entity_to_layer()` for skirt/brim geometry. Host dispatch updated to batch-prepend entity pushes so finalization entities precede model entities. 5 acceptance tests pass, clippy clean. WASM rebuild (`build-core-modules.sh`) required to activate the live WASM path.**
- [x] TASK-143 Port `WipeTower` live geometry from legacy `process()` into `run_finalization()` and retire the legacy-only finalization path. Continues DEV-013. **Closed 2026-04-25 (packet 17):** `WipeTower::run_finalization()` implemented via `LayerCollectionView` + `FinalizationOutputBuilder`; all 5 acceptance tests pass; `wipe-tower.wasm` rebuilt; DEV-013 fully closed.
- [~] TASK-151 Teach `path-optimization-default` to consume seam-placement output and stop acting as a comment-only slot filler on real wall loops. Covers the non-retraction portion of DEV-023 and supports TASK-120c. **Reopened 2026-04-21 — packet `22_live-seam-contract-repair` restores the remaining live contract gap: `path_optimization_emit_layer_markers=false` still fails on the host path because `path-optimization-default` emits marker comments unconditionally.**
- [x] TASK-159 Add `PrePass::SeamPlanning` plus a canonical `SeamPlanIR` blackboard contract so seam choices can be scored from global mesh/layer-plan context and injected into `Layer::PerimetersPostProcess`. Continues DEV-009, deepens Orca parity, and supports TASK-120c plus TASK-135. **Closed 2026-04-24 — packet `23-rev1_prepass-seam-planning-orca-parity` fixes WIT boundary (`run-seam-planning` now accepts `list<MeshObjectView>`), updates dispatch.rs to pass `MeshObjectView` geometry, fixes seam_arm type, lowers curvature threshold to 0.2, and rebuilds all affected WASM binaries. All 9 acceptance tests green, clippy clean. Unblocks TASK-135.**
- [x] TASK-161 Establish `SupportPlanIR` and cross-layer support planning produced inside `PrePass::SupportGeometry` (host built-in commits `SupportGeometryIR`; `support-planner` guest emits `SupportPlanIR` from coarse geometry). Continues DEV-009, deepens Orca parity, and supports TASK-120.
- [x] TASK-162 Surface `LayerPlanIR.layers` and `RegionMapIR.entries` to the prepass guest via new WIT views (`layer-plan-view`, `region-segmentation-view`) so `support-planner` walks the real layer plan and emits one entry per `(layer, object, region)`. Closes the v1 layer-height-agnostic and single-region carve-outs from packet `28_tree-support-multi-layer-propagation`. Wired by packet `30_support-planner-prepass-wit-plumbing`.
- [x] TASK-163 (partial) Establish `SupportGeometryIR`, `PrePass::SupportGeometry`, `support_layer_height_mm`, and `support_top_z_distance_mm` as the architectural foundation for variable-height support planning. Support planner emits at coarse support resolution; emitter interpolates to model resolution near column tops. Continues TASK-120 acceptance evidence. Wired by packet `31a_support-geometry-prepass-and-layer-height`. **Closed 2026-04-29 (packet 31a):** `SupportGeometryIR` committed to blackboard; `PrePass::SupportGeometry` host built-in wired; `support_layer_height_mm` / `support_top_z_distance_mm` plumbed through WIT to `support-planner`; all 27 regression tests green; clippy clean.
- [x] TASK-163 (algorithmic) Close the five algorithmic v1 limitations (avoidance/collision cache from `SupportGeometryView`, radius tapering, raft + interface layers, wall-count-aware move scaling, OrcaSlicer config keys) on the foundation established by packet `31a_support-geometry-prepass-and-layer-height`. Continues TASK-120 acceptance evidence. Wired by packet `31b_support-planner-algorithmic-parity`.
- [ ] TASK-163b Replace the self-captured `resources/golden/benchy_tree_support_orca_*` snapshots with real OrcaSlicer reference output extracted from `resources/test_models/benchy.stl` + `resources/test_config/benchy-tree-support.json`. Current goldens prove planner stability across runs but not parity with OrcaSlicer. Also promote the `support-planner.node-clamped-out` warning from `host-services.log` to a typed `Diagnostic` channel via the prepass output WIT. Continues TASK-120 / TASK-163.
- [x] TASK-152 Expand `path-optimization-default` beyond comment-only output into a real optimization stage with deterministic travel ordering, module-side z-hop planning, and explicit coverage for the remaining DEV-023 feature gaps. Continues DEV-023 and supports TASK-120d. **Closed 2026-04-29 via packets 18-20 — all sub-tasks (152a-152h) landed: entity ordering (packet 18), tool-change and cooling policy (packet 19), finalization-aware travel coordination (packet 20), layer-collection-builder surface (packet 33).**
- [x] TASK-152a Add deterministic nearest-neighbor-style travel sequencing in `path-optimization-default`, with regression coverage on real per-layer entities instead of preserving `assemble_ordered_entities` order. Continues DEV-023 and supports TASK-120d. **Closed 2026-04-26 — packet 18.**
- [x] TASK-152b Emit module-level tool-change ordering for mixed-tool layers and regression coverage proving `LayerTools`-equivalent sequencing crosses the live path. Continues DEV-023. **Closed 2026-04-29 — packet 19 implements per-tool grouping inside `path-optimization-default` via `set-entity-order` and `push-tool-change`; tests drive live WASM dispatch.**
- [x] TASK-152c Decide whether fan-speed / cooling overrides belong in path optimization or remain intentionally unsupported; either implement the chosen override surface with regression coverage or document the rejection path explicitly in docs/05 and docs/07. Continues DEV-023. **Closed 2026-04-29 — packet 19 documents fan-speed and cooling overrides as intentionally unsupported on the live `Layer::PathOptimization` surface; rejection wording locked in docs/05_module_sdk.md § Layer Stage Module Surface Rejections and docs/07_implementation_status.md (this entry).**
- [x] TASK-152d Add cross-object ordering in `path-optimization-default` so per-layer planning can sequence entities across objects instead of treating each layer in object isolation; cover deterministic mixed-object cases. Continues DEV-023. **Closed 2026-04-26 — packet 18.**
- [x] TASK-152e Add role-aware bridge / overhang reordering so bridge-sensitive entities can be prioritized and regression-lock the behavior on bridge-tagged inputs. Continues DEV-023. **Closed 2026-04-26 — packet 18.**
- [x] TASK-152f Coordinate `path-optimization-default` with `SkirtBrim` and `WipeTower` outputs so wipe / brim travel decisions stop ignoring finalization geometry; add integration coverage across the finalization boundary. Continues DEV-023 and supports TASK-142 and TASK-143. **Closed 2026-04-29 via packet 20 — finalization-aware travel coordination.**
- [x] TASK-152g Add `layer-collection-builder` WIT resource (`set-entity-order(items: list<tuple<u32, bool>>)`) and wire it through host bindings, SDK, and the `LayerModule::run_path_optimization` trait. Host validates and applies the proposal; host fallback preserved. Module migration deferred to packet 33. Continues DEV-023. **Closed 2026-04-28 — packet 33 consumes the layer-collection-builder surface end-to-end.**
- [x] TASK-152h Move the deterministic NN entity-ordering algorithm from `slicer-host::layer_executor::order_entities_by_nearest_neighbor` into `path-optimization-default::run_path_optimization` using the `layer-collection-builder` surface from packet 32. Delete the host helper. Mark packet 18 superseded. **Closed 2026-04-28 — packet 33.**

### Workstream 4 — Progress events and Python bridge coverage

- [ ] TASK-136 Add end-to-end progress-event coverage proving paint-annotation failure codes 501-504 reach the JSONL emitter on the live pipeline path. Supports DEV-010 acceptance evidence and guards the live path after DEV-019 closure.
- [~] TASK-137 Resolve the Python postpass live-path decision and closure evidence. Covers DEV-024. Exactly one of TASK-137b or TASK-137c should land after TASK-137a.
- [ ] TASK-137a Decide whether Python postpass is a supported live-path backend or a test-only facility, and record the policy target in docs/05 and docs/07. Covers DEV-024.
- [ ] TASK-137b If Python is intended to be live, add explicit runtime selection for `PythonPostpassRunner` and acceptance coverage through the production pipeline. Continues DEV-024.
- [ ] TASK-137c If Python is intentionally non-live, remove stale live-path expectations from docs/05 and docs/07 and close DEV-024 on the documentation path. Alternate close path for DEV-024.
- [x] TASK-138 Close the Python `Init` phase coverage gap. `crates/slicer-host/tests/python_bridge_init_phase_tdd.rs` is green.

### Workstream 5 — Governance and closure drift

- [~] TASK-139 Close the DEV-020 source/docs drift around dead fallback runners and Phase G closure notes.
- [ ] TASK-139a Remove dead `Noop*Runner` remnants from the source tree and any stale tests or wiring. Covers DEV-020.
- [ ] TASK-139b Correct the Phase G closure notes in docs/07 once the source cleanup lands so docs and source agree. Covers DEV-020.
- [ ] TASK-140 Evaluate the Architecture Acceptance Gate using docs/11 and docs/12 once TASK-120 and its subtasks are complete. Covers DEV-010 and the evidence gaps in DEV-026.
- [~] TASK-141 Keep the planning/governance docs synchronized with the live dependency graph and deviation registry. Supports DEV-030.
- [ ] TASK-141a Update docs/07 dependency ordering and workstream sequencing whenever a remediation task is added, split, or closed. Supports DEV-030.
- [ ] TASK-141b Keep `docs/DEVIATION_LOG.md` synchronized with every open architectural deviation, linked task IDs, and close status. Supports DEV-030 and live-registry hygiene for the acceptance gate.
- [ ] TASK-141c Remove stale blocker summaries and closure notes from docs/11 and docs/12 as their owning tasks land. Supports DEV-030.
- [ ] TASK-154 Enforce `min_host_version` at startup and add semver pass/fail coverage for compatible and incompatible manifests. Covers DEV-026.
- [ ] TASK-155 Make manifest-schema validation surface a real `Schema` failure for missing or invalid schema declarations, with CLI and host regression tests. Continues DEV-026.
- [ ] TASK-156 Add runtime-budget evidence collection for docs/12 memory, host-call, and full-slice time thresholds, plus reproducible benchmark/report hooks. Continues DEV-026.

## Open Deviation Map

Use `docs/14_deviation_audit_history.md` only for retired XML-era numbering and audit provenance. The list below is the current live map.

- ~~DEV-002~~ — **Closed.** All 17 core-module manifests now populate `[ir-access]` declarations; `core_module_ir_access_contract_tdd.rs` green (TASK-121).
- ~~DEV-003~~ — **Closed.** `ModuleAccessAudit` records now populated from all live prepass/layer/postpass paths and plumbed into `DagValidationRequest.access_audits`; undeclared runtime accesses rejected at the WIT boundary (TASK-123–124).
- ~~DEV-004~~ — **Closed.** Claim Transition Matrix now enforced for all non-transitionable claims; `claim_transition_matrix_tdd.rs` green (TASK-125).
- ~~DEV-005~~ — **Closed.** Non-planar Z envelope `[layer.z, layer.z + effective_layer_height]` now enforced at output-commit boundaries (TASK-127).
- ~~DEV-006~~ — **Closed.** Postpass GCode command bodies now cross the WIT boundary via real command lists; layer-world and finalization-world deep-copy boundary coverage also landed (TASK-129a/b/c).
- ~~DEV-008~~ — **Closed.** All 17 core-module manifests now populate `[config.schema]` declarations (TASK-122).
- DEV-009 — Benchy Phase H output is only partially correct on the live path, including missing OrcaSlicer-compatible GCode comment metadata for native preview visualization.
- DEV-010 — Acceptance-gate evidence and governance closure are still open.
- ~~DEV-013~~ — **Closed 2026-04-25.** Both `SkirtBrim` (packet 16) and `WipeTower` (packet 17) now implement `run_finalization()` on the live path.
- ~~DEV-014~~ — **Closed 2026-04-24.** Macro, host, and test-guest codegen consolidated onto one canonical WIT source in `wit/`; package/version literals normalized; `validate_wit_world` enforces allowlist at startup (TASK-144–146, packet `25_wit-canonical-surface-lock`).
- ~~DEV-015~~ — **Closed.** `raycast_z_down`, `surface_normal_at`, and `object_bounds` now backed by real mesh-query implementations; hit/miss semantics verified across all WIT worlds (TASK-147–148).
- ~~DEV-016~~ — **Closed 2026-04-20.** `ExtrusionRole::Custom(String)`, `PaintSemantic::Custom(String)`, and `WallFeatureFlags.custom` now cross the WIT boundary losslessly (TASK-149–150).
- DEV-020 — Phase G still overstates completion because dead `Noop*Runner` code remains.
- DEV-023 — PathOptimization remains an MVP slot-filler rather than a real optimization stage.
- DEV-024 — Python postpass support exists but is not on the live path.
- ~~DEV-025~~ — **Closed 2026-05-08.** All five SDK↔WIT prepass segmentation mismatches resolved: mismatches 1+2 by TASK-128a/128b, mismatch 3 by packet `43-rev1_macro-prepass-segmentation-output-drain` (TASK-130a/130b), mismatches 4+5 by packet 42 (TASK-130c).
- DEV-026 — Host semver, manifest-schema validation, and runtime budget evidence remain incomplete.
- ~~DEV-027~~ — **Closed 2026-04-21.** `ObjectMesh.world_z_extent` added as first-class derived IR field; 7 integration fixture tests and transform error paths added (TASK-157–158, packet `10_transform-aware-world-z`).
- DEV-030 — Planning and remediation docs still lag the real dependency graph.
- DEV-040 — User-supplied config silently ignored in `RegionMapIR` builder; `ResolvedConfig::default()` unconditionally used; packets 36/37 and future tunable-behavior work blocked (TASK-166).
- DEV-044 — Paint data has no user-reachable input surface on the live binary path: `load_3mf` parses geometry only and silently discards Bambu/Orca paint metadata; no CLI paint flag exists. PaintSegmentation contract is green (DEV-025 closed) but unfalsifiable end-to-end. Closure: Packet 50 (`paint-input-3mf-ingestion`).
- DEV-045 — RegionMap is paint-blind: no `paint_config:<semantic>:<key>` namespace in `config_resolution.rs`; `RegionPlan` has no paint-semantic dimension; per-paint settings cannot differentiate GCode. Closure: Packet 51 (`paint-semantic-region-overrides`). Depends on DEV-044.

## Tests Added as Gap Locks

- [x] `crates/slicer-host/tests/core_module_ir_access_contract_tdd.rs` — enumerates missing manifest IR contracts and guards the Stage I/O Contract.
- [x] `crates/slicer-host/tests/claim_transition_matrix_tdd.rs` — guards the non-transitionable claim matrix and transitionable-claim sanity cases.
- [x] `crates/slicer-host/tests/python_bridge_init_phase_tdd.rs` — closes the Python `Init` phase classification gap.

## Architecture Acceptance Gate

- Status: BLOCKED BY OPEN REMEDIATION TASKS
- Blocking tasks: TASK-120c, TASK-136, TASK-140, TASK-154, TASK-155, TASK-156

### Evidence Links

- Determinism: pending Phase H parity closure
- Recoverability: pending runtime access enforcement and progress-event coverage
- Resource bounds: pending RegionMap overflow, `resolve_active_regions`, and runtime-budget evidence collection
- Coupling control: pending manifest contract cleanup, claim transition enforcement, and custom-payload preservation
- Compatibility: pending WIT-source consolidation, `wit_world` validation, host semver/schema validation, and acceptance-gate evaluation
- Operability: pending Benchy acceptance run, OrcaSlicer-compatible GCode comment parity, finalization parity, and progress-event validation

### Notes

- Use `./docs/11_operational_governance_and_acceptance_gate.md` as the rubric.
- Metric thresholds are defined in `./docs/12_architecture_gate_metrics.md`.

## Blocked Tasks

- None. The remaining work is prioritized, not externally blocked.

## Governance Checklist Status

- Module/claim rollout checklist: IN PROGRESS
- Compatibility policy checks: NOT STARTED
- Release checklist: NOT STARTED
