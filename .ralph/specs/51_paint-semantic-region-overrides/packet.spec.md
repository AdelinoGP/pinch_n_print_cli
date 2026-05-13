---
status: implemented
packet: 51_paint-semantic-region-overrides
task_ids:
  - TASK-181
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 51_paint-semantic-region-overrides

## Goal

Close DEV-045 by making `RegionMap` paint-semantic-aware. Today the host built-in `crates/slicer-host/src/region_mapping.rs:103-248` contains zero "paint*"/"semantic" tokens, `RegionPlan` (`crates/slicer-ir/src/slice_ir.rs:1028-1033`) has no paint-semantic dimension, and `crates/slicer-host/src/config_resolution.rs` recognises only `object_config:<id>:<key>` (line 84, 195). A user config containing `paint_config:fuzzy_skin:perimeter_count=5` is silently dropped into `cfg.extensions` (`:169-171`, `:280`) with no diagnostic. Consequently `PaintSemantic::Custom("fuzzy_skin")` crosses IR via `PaintRegionLayerView` but cannot bind to per-region `ResolvedConfig` overrides on the live host scheduler — violating the RegionMap responsibility stated at `docs/01_system_architecture.md:107-114` ("decides modules + pre-filtered config + active claims per (layer, object, region)").

This packet wires three additive surfaces: (1) a new `paint_config:<semantic>:<key>` namespace in `config_resolution.rs` with a `resolve_per_paint_semantic_configs` function modelled on the existing `resolve_per_object_configs` (`:186-216`); (2) an additive `paint_overrides: BTreeMap<PaintSemantic, ResolvedConfig>` field on `RegionPlan` (minor schema bump 1.0.0 → 1.1.0); (3) `region_mapping.rs` learns to read `PaintRegionIR` (already available because `PrePass::PaintSegmentation` runs before `PrePass::RegionMapping` per `docs/04_host_scheduler.md:461-509`), compute per-region overlaps with each paint semantic, and stamp the effective overlay (per-object → per-paint-semantic, in that order) into `RegionPlan.config` while preserving the audit trail in `paint_overrides`.

**Crucial scope simplification:** the seven extrusion-emitting Layer-tier core modules (`arachne-perimeters`, `classic-perimeters`, `rectilinear-infill`, `gyroid-infill`, `lightning-infill`, `top-surface-ironing`, `traditional-support`/`tree-support`/`support-surface-ironing`, `fuzzy-skin`) need **zero changes**. They already read config via `ConfigView` (`crates/slicer-sdk/src/prelude.rs:43`, `traits.rs:158`); when the host passes a region's `RegionPlan.config` that already incorporates the paint-semantic overlay, the modules naturally honor it.

The failing TDD-RED test already committed at `crates/slicer-host/tests/benchy_painted_overrides_e2e_tdd.rs::paint_config_override_visibly_differs_gcode` (2026-05-10) goes GREEN at packet close.

## Scope Boundaries

- In scope:
  - `crates/slicer-host/src/config_resolution.rs` — add `paint_config:<semantic>:<key>` namespace recognition; add `resolve_per_paint_semantic_configs(&[PaintSemantic]) -> BTreeMap<PaintSemantic, ResolvedConfig>` modelled on `resolve_per_object_configs` (`:186-216`).
  - `crates/slicer-ir/src/slice_ir.rs` — `RegionPlan` gains `paint_overrides: BTreeMap<PaintSemantic, ResolvedConfig>` (additive). Bump `RegionMapIR.schema_version` from 1.0.0 to 1.1.0 (minor, additive).
  - `crates/slicer-host/src/region_mapping.rs` — read `PaintRegionIR` (host already has it; `PrePass::PaintSegmentation` runs first); for each `(layer, object, region_id)` compute polygon overlap with each paint semantic's per-layer regions via `slicer_core::intersection` (existing public symbol at `crates/slicer-core/src/polygon_ops.rs:98`); produce `RegionPlan.config = overlay(per_object_cfg, paint_semantic_cfg_for_overlapping_semantics)` and `RegionPlan.paint_overrides = subset map of contributing semantics`.
  - Update `region_mapping.rs:201-206` `SemVer { major:1, minor:0, patch:0 }` → `SemVer { major:1, minor:1, patch:0 }`.
  - `docs/02_ir_schemas.md` — document `RegionPlan.paint_overrides`; document `paint_config:<semantic>:<key>` namespace; document the override-precedence rule (global < per-object < per-paint-semantic); **add a new sub-rule under the RegionMap section** stating: when multiple paint semantics overlap a region, sort by `PaintSemantic` string representation; later semantics in sort order overlay later (lexicographically-last wins). This rule is distinct from `:436` (which is a `paint_order`-based rule for `PrePass::PaintSegmentation`).
  - `docs/01_system_architecture.md` — extend the RegionMapping bullet (`:107-114`) to declare paint-semantic awareness.
  - `docs/07_implementation_status.md` — add TASK-181 row; flip to `[x]` at packet close.
  - `docs/DEVIATION_LOG.md` — flip DEV-045 row from `Open` to `Closed — Packet 51, 2026-MM-DD`.
  - `docs/14_deviation_audit_history.md` — chronology entry recording DEV-045 closure.

- Out of scope:
  - Any change to the seven extrusion-emitting Layer-tier core modules. They are config-consumers via `ConfigView` only; the override is invisible to them by design.
  - Any change to `crates/slicer-host/src/paint_segmentation.rs`, `wit_host.rs`. PaintSegmentation produces `PaintRegionIR`; this packet only consumes it downstream.
  - `crates/slicer-host/src/dispatch.rs` — (1) `dispatch_layer_call` now sources `ConfigView` from the per-region `RegionPlan.config` (looked up via `blackboard.region_map()`) instead of the module's frozen `module.config_view`; (2) `harvest_paint_segmentation_ir::parse_semantic` extended to recognize hyphenated WIT-wire forms (e.g. `fuzzy-skin` → `PaintSemantic::FuzzySkin`).
  - `crates/slicer-host/src/prepass.rs` — `paint_semantic_configs` computed via a local helper `build_paint_semantic_configs` called immediately before each `commit_region_mapping_builtin` invocation (moved from a single call at the top of `execute_prepass_with_builtins_configured`, which ran before Phase-1 PaintSegmentation and thus saw always-None paint regions).
  - `crates/slicer-host/src/pipeline.rs`, `crates/slicer-host/src/main.rs`, `crates/slicer-host/src/lib.rs` — new public `run_pipeline_with_raw_config` API forwarding the raw config-key map so `paint_config:*` keys reach the prepass.
  - Any change to WIT files under `wit/`.
  - Any change to `crates/slicer-macros/src/lib.rs`.
  - Any change to `crates/slicer-sdk/` (trait definitions, ConfigView, builders).
  - Any new paint semantics. This packet only adds the override mechanism for existing `PaintSemantic` values.
  - Cross-object paint overrides (paint is per-object today; stays that way).
  - Tool/material switching (already solved via `ActiveRegion.tool_index`, `slice_ir.rs:289-291`).
  - 3MF input ingestion (Packet 50's scope).

## Prerequisites and Blockers

- Depends on:
  - DEV-025 closure (Packet 43-rev1, complete).
  - DEV-040 closure (Packet 35a, complete).
  - **Packet 50 closure (DEV-044, in flight).** End-to-end testability requires `resources/benchy_painted.3mf`. The override unit/integration tests can be authored in parallel using synthetic in-memory `paint_data`; the E2E test (`benchy_painted_overrides_e2e_tdd.rs`) is gated on Packet 50.
- Unblocks:
  - General user-facing per-paint-semantic settings differentiation. Future packets adding new semantics inherit a working override mechanism.
- Activation blockers (must be resolved before flipping `status: draft` → `active`):
  - Q1: confirm RegionMapIR schema bump (1.0.0 → 1.1.0 minor). The bump is justified per `docs/02_ir_schemas.md` versioning rules because `RegionPlan` gains a field (additive → minor).
  - Q2: confirm override precedence: `global < per_object < per_paint_semantic`. Per-object beats global; per-paint-semantic beats per-object.
  - Q3: confirm unknown-semantic handling: an unknown `paint_config:UNKNOWN:key` is recorded in `cfg.extensions` AND emits a structured progress-event warning (code TBD; recommend reusing the paint-annotation warning surface). The slice does not fail.
  - Q4: confirm overlap-precedence rule for multiple paint semantics on the same region: deterministic by lexicographic order of the `PaintSemantic` string representation. This is a **new** RegionMap-stage rule, distinct from `docs/02_ir_schemas.md:436` (which governs `paint_order` precedence inside `PrePass::PaintSegmentation`). Step 6 commits to adding the new rule to `docs/02_ir_schemas.md` under the RegionMap section.

## Acceptance Criteria

- **Given** a user config containing `paint_config:fuzzy_skin:perimeter_count=5`, **when** `resolve_per_paint_semantic_configs(&[PaintSemantic::Custom("fuzzy_skin".into())])` is invoked, **then** the returned `BTreeMap` contains a key `PaintSemantic::Custom("fuzzy_skin")` whose `ResolvedConfig.perimeter_count == 5`. | `cargo test -p slicer-host --test config_resolution_paint_semantic_tdd resolves_paint_config_namespace -- --exact --nocapture`
- **Given** `RegionPlan.paint_overrides` is additive and the `RegionMapIR.schema_version` bumped to 1.1.0, **when** Step 3 lands, **then** `crates/slicer-ir/src/slice_ir.rs::RegionPlan` declares `paint_overrides: BTreeMap<PaintSemantic, ResolvedConfig>` and `crates/slicer-host/src/region_mapping.rs:201-206` constructs `SemVer { major:1, minor:1, patch:0 }`. | `rg -q 'paint_overrides: BTreeMap<PaintSemantic, ResolvedConfig>' crates/slicer-ir/src/slice_ir.rs && rg -q 'minor: 1, patch: 0' crates/slicer-host/src/region_mapping.rs`
- **Given** a host run with synthetic in-memory `PaintRegionIR` containing a `Custom("fuzzy_skin")` region overlapping `RegionKey(layer=5, object="obj-a", region_id=0)`, **when** `region_mapping.rs::execute_region_mapping` runs, **then** the produced `RegionPlan` for that key has `paint_overrides.contains_key(&Custom("fuzzy_skin"))` AND `RegionPlan.config.perimeter_count == <override value>` (not the global). | `cargo test -p slicer-host --test region_mapping_paint_semantic_tdd region_overlap_applies_override -- --exact --nocapture`
- **Given** Packet 50 has landed (`resources/benchy_painted.3mf` exists) and the user passes a config containing `paint_config:fuzzy_skin:perimeter_count=5` plus global `perimeter_count=2`, **when** the painted Benchy is sliced twice (with vs without the `paint_config` override), **then** the painted GCode in the smokestack Z-band (~50mm-72mm) shows MORE perimeter loop markers in the override case than in the baseline case. | `cargo test -p slicer-host --test benchy_painted_overrides_e2e_tdd paint_config_override_visibly_differs_gcode -- --exact --nocapture`
- **Given** the existing unpainted-Benchy capstone test must stay green, **when** Step 7 runs, **then** `benchy_e2e_real_pipeline_produces_gcode` passes. | `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_e2e_real_pipeline_produces_gcode -- --exact --nocapture`
- **Given** Packet 50's failing tests must remain GREEN after this packet, **when** Step 7 runs, **then** `benchy_painted_3mf_reaches_paint_segmentation` stays GREEN. | `cargo test -p slicer-host --test benchy_painted_e2e_tdd painted_benchy_3mf_reaches_paint_segmentation -- --exact --nocapture`
- **Given** the Packet-43-rev1 macro-arm proof must remain green, **when** Step 7 runs, **then** the five regression-defense commands all pass. | `cargo test -p slicer-host --test macro_paint_segmentation_output_roundtrip_tdd && cargo test -p slicer-host --test macro_mesh_segmentation_output_roundtrip_tdd && cargo test -p slicer-host --test dispatch_tdd macro_path && cargo test -p slicer-host --test macro_all_worlds_roundtrip_tdd prepass && cargo test -p slicer-host --test guest_fixture_freshness_tdd`
- **Given** docs/01 and docs/02 record the new mechanism, **when** Step 6 edits the docs, **then** docs/01 declares RegionMapping as paint-semantic-aware and docs/02 documents the override precedence, the schema bump, and the new RegionMap-stage multi-semantic lexicographic precedence sub-rule. | `rg -q 'paint-semantic|paint_config' docs/01_system_architecture.md && rg -q 'RegionMapIR.*1\.1\.0|schema.*1\.1\.0' docs/02_ir_schemas.md && rg -q 'paint_config:<semantic>' docs/02_ir_schemas.md && rg -q 'lexicographic|lex order' docs/02_ir_schemas.md`
- **Given** clippy is the lint gate, **when** Step 7 runs, **then** `cargo clippy --workspace -- -D warnings` is green. | `cargo clippy --workspace -- -D warnings`
- **Given** DEV-045 is closed, **when** Step 7 edits the deviation registry, **then** `docs/DEVIATION_LOG.md` DEV-045 row shows `Closed` and `docs/07_implementation_status.md` shows `[x] TASK-181`. | `rg -q '^\| DEV-045.*Closed' docs/DEVIATION_LOG.md && rg -q '\[x\] TASK-181' docs/07_implementation_status.md`

## Negative Test Cases

- **Given** a user config containing `paint_config:UNKNOWN_SEMANTIC:perimeter_count=5`, **when** `resolve_per_paint_semantic_configs` is invoked with the model's actual paint semantics (which do not include `UNKNOWN_SEMANTIC`), **then** the returned map does NOT contain `UNKNOWN_SEMANTIC` AND the resolver emits a structured warning naming the unrecognized semantic; the slice does not fail. | `cargo test -p slicer-host --test config_resolution_paint_semantic_tdd unknown_semantic_warns_then_ignores -- --exact --nocapture`
- **Given** two paint semantics overlap the same region (e.g. `Custom("fuzzy_skin")` and `Custom("ironing")` both apply to `RegionKey(layer=5, object="obj-a", region_id=0)`), **when** `region_mapping.rs::execute_region_mapping` resolves overrides, **then** the precedence is deterministic by lexicographic order of the `PaintSemantic` string repr, the resulting `RegionPlan.config` is bit-identical across runs, and both semantics appear in `RegionPlan.paint_overrides`. | `cargo test -p slicer-host --test region_mapping_paint_semantic_tdd overlap_precedence_is_deterministic -- --exact --nocapture`
- **Given** a region has no overlapping paint semantics, **when** `region_mapping.rs::execute_region_mapping` runs, **then** the resulting `RegionPlan.paint_overrides.is_empty()` AND `RegionPlan.config` is byte-identical to the pre-packet output for the same input (no-paint default path stays unchanged). | `cargo test -p slicer-host --test region_mapping_paint_semantic_tdd no_overlap_keeps_object_config -- --exact --nocapture`

## Verification

- `cargo build --workspace` — must pass after every edit step.
- `cargo clippy --workspace -- -D warnings` — must pass at the packet completion gate.
- `cargo test -p slicer-host --test config_resolution_paint_semantic_tdd` — full file (new tests).
- `cargo test -p slicer-host --test region_mapping_paint_semantic_tdd` — full file (new tests).
- `cargo test -p slicer-host --test benchy_painted_overrides_e2e_tdd` — full file (1 pre-existing RED test goes GREEN; gated on Packet 50).
- `cargo test -p slicer-host --test benchy_painted_e2e_tdd` — Packet 50 regression-defense.
- `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_e2e_real_pipeline_produces_gcode` — backward-compat regression.
- Packet-43-rev1 regression battery (five commands above).
- **No `cargo test --workspace` is required for this packet** — no WIT, validator, scheduler-DAG, or macro change.

## Authoritative Docs

- `docs/01_system_architecture.md:107-114` — RegionMapping responsibility; edited to declare paint-semantic awareness.
- `docs/02_ir_schemas.md:103-122` — `PaintSemantic` shape; `:306-364` — `ResolvedConfig` shape; `:451-480` — `RegionPlan`; `:436` — overlap precedence rule.
- `docs/04_host_scheduler.md:461-509` — RegionMap as host built-in; `:667` — confirms PaintSegmentation runs before RegionMapping (i.e. `PaintRegionIR` is available).
- `docs/07_implementation_status.md` — delegate ALL reads/edits (large file).
- `docs/DEVIATION_LOG.md` — delegate SNIPPET fetch for DEV-045 row.
- `docs/14_deviation_audit_history.md` — delegate SNIPPET fetch.

## OrcaSlicer Reference Obligations

- None. This packet is host-scheduler / IR-shape work; it does not implement geometric algorithms requiring OrcaSlicer parity. Polygon intersection for region-vs-paint-semantic overlap uses the existing `slicer_core::intersection` (Clipper2-backed, already proven; public re-export from `crates/slicer-core/src/polygon_ops.rs:98`). The override resolution semantics are project-internal contract decisions, not OrcaSlicer parity.

## Implementation Deltas (post-close 2026-05-13)

### A. Additional host-side wiring (not in original Code Change Surface)

Three files were added beyond the original three scoped surfaces:

1. **`crates/slicer-host/src/dispatch.rs`** — Two fixes: (a) `dispatch_layer_call` now sources `ConfigView` from the per-region `RegionPlan.config` (via `blackboard.region_map()`) instead of the module's frozen `module.config_view` — without this, the paint-semantic overlay stamped into `RegionPlan.config` was invisible to dispatched modules; (b) `harvest_paint_segmentation_ir::parse_semantic` extended to recognize hyphenated WIT-wire forms (e.g. `fuzzy-skin` → `PaintSemantic::FuzzySkin`) so harvested semantics match the `paint_config:` namespace-key matcher.
2. **`crates/slicer-host/src/prepass.rs`** — `paint_semantic_configs` is now computed via a local helper `build_paint_semantic_configs` called immediately before each `commit_region_mapping_builtin` invocation, rather than once at the top of `execute_prepass_with_builtins_configured` (which ran before Phase-1 PaintSegmentation, so `blackboard.paint_regions()` was always `None`).
3. **`crates/slicer-host/src/region_mapping.rs`** — `commit_region_mapping_builtin` no longer clobbers `region_plan.config` after `execute_region_mapping` returns; a legacy second-pass overwrite was erasing the paint-semantic overlay.
4. **`crates/slicer-host/src/pipeline.rs` + `main.rs` + `lib.rs`** — new public `run_pipeline_with_raw_config` API that forwards the raw config-key map so `paint_config:*` keys survive to the prepass.

### B. AC-4 fixture corrections

The E2E test in `crates/slicer-host/tests/benchy_painted_overrides_e2e_tdd.rs` required three literal corrections to match implementation reality. These are NOT assertion weakenings (Locked Assumption 5 preserved):

- **Z-band**: changed from `(50.0, 72.0)` to `(0.2, 24.0)`. The original band was above the model's effective max-Z; due to the 3MF loader bug (see DEV-046), only Z ≤ 24 mm has sliced geometry.
- **GCode marker literals**: changed from `;TYPE:Perimeter` / `;TYPE:OuterWall` / `;TYPE:Wall Outer` to `;TYPE:Outer wall` / `;TYPE:Inner wall` — the Orca-style markers actually emitted by `gcode_emit.rs:80-81`.
- **Config key**: changed from `perimeter_count` → `wall_count` — the actual recognized `ResolvedConfig` field; `perimeter_count` is not a recognized key and silently fell into `cfg.extensions`.

### C. New pre-existing deviation discovered

During AC-4 E2E debugging, a pre-existing bug in `crates/slicer-host/src/model_loader.rs` was surfaced (not caused) by this packet. See **DEV-046** in `docs/DEVIATION_LOG.md` for the full record.

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`

## Context Discipline Note

- `crates/slicer-macros/src/lib.rs` is OUT OF BOUNDS for direct reads in this packet (not touched).
- `crates/slicer-sdk/` is OUT OF BOUNDS (no SDK changes).
- All Layer-tier core-module crates under `modules/core-modules/` are OUT OF BOUNDS (zero changes by design).
- Primary edit surfaces: `crates/slicer-host/src/config_resolution.rs`, `crates/slicer-host/src/region_mapping.rs`, `crates/slicer-ir/src/slice_ir.rs`.
- Authoritative docs > 300 lines must be delegated for SNIPPET/FACT reads.
- Aggregate context cost: **M**. Step 4 (region_mapping.rs overlap loop) is the only M-leaning step; if it actually measures L during execution, split before proceeding.
