---
status: implemented
packet: 31a-REV2_revert-prepass-support-generation
task_ids:
  - TASK-161
  - TASK-162
  - TASK-163-foundation
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
supersedes:
  - 31a_support-geometry-prepass-and-layer-height
  - 31a-REV1_support-geometry-prepass-and-layer-height
---

# Packet Contract: 31a-REV2_revert-prepass-support-generation

## Goal

Revert the prepass-stage notion of `PrePass::SupportGeneration` everywhere it appears — host code, SDK, WIT, schema, manifests, tests, docs, predecessor spec packets (`28`, `30`, `31a`, `31a-REV1`), the in-flight successor spec packet (`31b`), and source-code comments — and consolidate all coarse prepass support planning into a single `PrePass::SupportGeometry` slot. Within that slot the host built-in runs first and commits `SupportGeometryIR`; guest modules then run via the unified WIT export `run-support-geometry` and emit `SupportPlanIR`. All actual support generation (paths, extrusions, tree branches, walls) remains constrained to `Layer::Support`. The substantive foundation work from `31a` (IR shape, blackboard slot, host built-in, config keys, WIT view records) and the execution-order fix from `31a-REV1` are absorbed verbatim as ACs of this packet so a fresh agent running this packet alone produces the full intended end state.

## Scope Boundaries

- In scope:
  - Removal of every `PrePass::SupportGeneration` reference (stage id, WIT export `run-support-generation`, SDK builder `SupportGenerationOutput`, trait method `run_support_generation`, schema entry, macro arm, host stage routing, `STAGE_ORDER` entry, `required_slots` row, blackboard `SupportPlan` slot policy attribution).
  - Introduction of the unified WIT export `run-support-geometry` with merged signature `(list<mesh-object-view>, layer-plan-view, region-segmentation-view, support-geometry-view)` returning the existing geometry-output record extended with `list<support-plan-entry>`.
  - Repurpose of `support-planner` core module: `support-planner.toml` `stage.id` flips to `PrePass::SupportGeometry`; trait impl renames to `run_support_geometry`; module description rewritten.
  - Carry-forward of all correct work from packet `31a` (IR `SupportGeometryIR` shape and key, `BlackboardPrepassSlot::SupportGeometry`, `commit_support_geometry()`, `support_geometry()` accessor, host built-in `crates/slicer-host/src/support_geometry.rs`, intermediate model-resolution outline logic with `u32::MAX` sentinel, WIT view records `support-geometry-view-entry` / `support-geometry-view`, manifest config keys `support_layer_height_mm` and `support_top_z_distance_mm` with their bounds).
  - Carry-forward of the execution-order fix from packet `31a-REV1` (`execute_prepass()` runs before built-in commitment; no `stage_requires_region_map` two-phase helper; tree-support plans without `PrePass::LayerPlanning` succeed without `LayerPlanIR` errors).
  - Test files renamed via `git mv` to preserve history: `prepass_support_generation_tdd.rs` → `prepass_support_geometry_tdd.rs`; `prepass_support_generation_layer_plan_tdd.rs` → `prepass_support_geometry_layer_plan_tdd.rs`; `live_support_generation_tdd.rs` → `live_layer_support_tdd.rs` (latter targets `Layer::Support`, not the prepass slot, so reflects its actual scope).
  - SDK builder rename `SupportGenerationOutput` → `SupportGeometryOutput`.
  - Documentation rewrite across `docs/01`, `docs/02`, `docs/03`, `docs/04`, `docs/05`, `docs/10`, including Stage I/O table, required-slots table, glossary entries, and `SupportPlanIR` producer attribution.
  - `docs/07_implementation_status.md` reconciliation: TASK-161 rewritten in place to describe consolidated outcome (checkbox stays `[ ]` until this packet closes it); TASK-162 stays `[x]`.
  - Cross-packet edits (explicit override of the standing Cross-Packet Mutation Rule, by user directive):
    - Packets `31a`, `31a-REV1`: flip `status: superseded`; prepend HEAD admonition explicitly listing which substantive ACs are absorbed by which 31a-REV2 ACs (no work lost).
    - Packets `28`, `30`: status preserved as `implemented`; prepend HEAD admonition; rewrite stage-name references in body (`PrePass::SupportGeneration` → `PrePass::SupportGeometry`, `run-support-generation` → `run-support-geometry`).
    - Packet `31b`: status preserved as `draft`; prepend HEAD note about dep rebase onto 31a-REV2; normalize ~6 references; algorithmic content untouched.
  - Codebase comment sweep: every doc-comment / line-comment / block-comment reference to `PrePass::SupportGeneration`, `run-support-generation`, or stage-level "support generation" prose normalized to the new vocabulary.
- Out of scope:
  - TASK-163 algorithmic work (avoidance/collision cache, radius taper, raft + interface layers, wall-count-aware move scaling, OrcaSlicer config keys). Owned by packet `31b`.
  - New algorithmic content of any kind. This packet is structurally a revert + consolidation.
  - OrcaSlicer parity reads. No new behavior is being ported; nothing to compare.
  - Any change to `Layer::Support` consumer logic in `tree-support` or `traditional-support` beyond comment normalization. The consumer-side contract (`SupportPlanIR` read via SDK accessor) is unchanged.
  - GUI wiring of `support_layer_height_mm` / `support_top_z_distance_mm`.

## Prerequisites and Blockers

- Depends on:
  - Working-tree state of `31a` and `31a-REV1` partial implementation (visible in `git status` at packet authoring time). The packet is designed to run regardless of whether those changes are committed, staged, or unstaged.
- Unblocks:
  - Packet `31b_support-planner-algorithmic-parity` — once 31a-REV2 lands, 31b can be reactivated with normalized references.
  - Any future packet that reads or writes the `PrePass::SupportGeometry` slot.
- Activation blockers:
  - None outstanding. All four design questions resolved (host-first ordering, `run-support-geometry` export name + merged signature, implemented-packet treatment via admonition + body rewrite, TASK-161 rewritten in place).

## Acceptance Criteria

- **Given** the workspace, **when** `rg "PrePass::SupportGeneration" crates modules wit docs` runs, **then** zero matches are reported. | `! rg -q "PrePass::SupportGeneration" crates modules wit docs`
- **Given** the workspace, **when** `rg "run-support-generation|run_support_generation" crates modules wit docs` runs, **then** zero matches are reported. | `! rg -q "run-support-generation|run_support_generation" crates modules wit docs`
- **Given** the workspace, **when** `rg "SupportGenerationOutput" crates modules wit docs` runs, **then** zero matches are reported. | `! rg -q "SupportGenerationOutput" crates modules wit docs`
- **Given** `wit/world-prepass.wit`, **when** the file is read, **then** it declares `export run-support-geometry: func(...)` whose parameter list contains `list<mesh-object-view>`, `layer-plan-view`, `region-segmentation-view`, and `support-geometry-view`. | `rg -q "run-support-geometry" wit/world-prepass.wit && rg -q "support-geometry-view" wit/world-prepass.wit && rg -q "layer-plan-view" wit/world-prepass.wit && rg -q "region-segmentation-view" wit/world-prepass.wit`
- **Given** `slicer-ir`, **when** the test asserting `SupportGeometryIR`'s shape runs, **then** it confirms keys `(global_support_layer_index: u32, object_id, region_id) → Vec<ExPolygon>` and the `u32::MAX` sentinel for intermediate model-resolution outline layers. | `cargo test -p slicer-ir --test support_geometry_ir_shape_tdd -- --exact support_geometry_ir_keys_and_sentinel`
- **Given** `slicer-host`, **when** the blackboard roundtrip test runs, **then** `BlackboardPrepassSlot::SupportGeometry` is committed via `commit_support_geometry()` and read via `support_geometry()` accessor, returning `Arc<SupportGeometryIR>`. | `cargo test -p slicer-host --test blackboard_support_geometry_slot_tdd -- --exact support_geometry_slot_roundtrip`
- **Given** a prepass plan with stage `PrePass::SupportGeometry` and one guest module of that stage, **when** `execute_prepass_with_builtins` runs, **then** the host built-in commits `SupportGeometryIR` strictly before the guest's `run-support-geometry` is invoked, and the guest observes a non-empty `SupportGeometryView`. | `cargo test -p slicer-host --test prepass_support_geometry_tdd -- --exact host_builtin_runs_before_guest`
- **Given** `support-planner.toml`, **when** the manifest is read, **then** `stage.id == "PrePass::SupportGeometry"` and `description` makes no reference to `PrePass::SupportGeneration`. | `rg -q '^id\s*=\s*"PrePass::SupportGeometry"' modules/core-modules/support-planner/support-planner.toml && ! rg -q "PrePass::SupportGeneration" modules/core-modules/support-planner/support-planner.toml`
- **Given** `support-planner.toml` and `tree-support.toml`, **when** the manifests are read, **then** both expose `support_layer_height_mm` (default 0.0, min 0.05, max 1.0) and `support_top_z_distance_mm` (default 0.0, min 0.0, max 5.0). | `rg -q "support_layer_height_mm" modules/core-modules/support-planner/support-planner.toml && rg -q "support_layer_height_mm" modules/core-modules/tree-support/tree-support.toml && rg -q "support_top_z_distance_mm" modules/core-modules/support-planner/support-planner.toml && rg -q "support_top_z_distance_mm" modules/core-modules/tree-support/tree-support.toml`
- **Given** a prepass plan whose stage list does not include `PrePass::LayerPlanning`, **when** `execute_prepass_with_builtins` runs against that plan, **then** the run completes without a `PrepassExecutionError::StagePrerequisite` referencing `LayerPlanIR`, because `LayerPlanIR` is committed inside `execute_prepass()` before any built-in observes the blackboard. | `cargo test -p slicer-host --test prepass_execution_order_tdd -- --exact tree_support_plan_succeeds_without_layer_planning_stage`
- **Given** a `Layer::Support` execution that consumes `SupportPlanIR` committed by `PrePass::SupportGeometry`, **when** the live integration test runs, **then** tree-support emits branch segments matching the planner's `branch_segments` for every `(layer, object, region)` entry. | `cargo test -p slicer-host --test live_layer_support_tdd -- --exact tree_support_consumes_support_plan_ir_from_support_geometry_stage`
- **Given** `docs/07_implementation_status.md`, **when** the TASK-161 line is read, **then** its text describes "SupportPlanIR + cross-layer support planning produced inside `PrePass::SupportGeometry`" and contains no reference to `PrePass::SupportGeneration`; the checkbox stays `[ ]`. | `rg -q "TASK-161.*PrePass::SupportGeometry" docs/07_implementation_status.md && ! rg -q "TASK-161.*PrePass::SupportGeneration" docs/07_implementation_status.md`
- **Given** `.ralph/specs/31a_*/packet.spec.md` and `.ralph/specs/31a-REV1_*/packet.spec.md`, **when** their frontmatter is read, **then** both have `status: superseded` and a HEAD admonition naming `31a-REV2` with an explicit AC absorption mapping. | `rg -q "^status: superseded" .ralph/specs/31a_support-geometry-prepass-and-layer-height/packet.spec.md && rg -q "^status: superseded" .ralph/specs/31a-REV1_support-geometry-prepass-and-layer-height/packet.spec.md && rg -q "Superseded by 31a-REV2" .ralph/specs/31a_support-geometry-prepass-and-layer-height/packet.spec.md && rg -q "Superseded by 31a-REV2" .ralph/specs/31a-REV1_support-geometry-prepass-and-layer-height/packet.spec.md`
- **Given** `.ralph/specs/28_*/packet.spec.md` and `.ralph/specs/30_*/packet.spec.md`, **when** the files are read, **then** each has the HEAD admonition pointing at `31a-REV2`, the body contains zero `PrePass::SupportGeneration` references, and the body contains zero `run-support-generation` references; status frontmatter remains `implemented`. | `rg -q "Superseded by 31a-REV2" .ralph/specs/28_tree-support-multi-layer-propagation/packet.spec.md && rg -q "Superseded by 31a-REV2" .ralph/specs/30_support-planner-prepass-wit-plumbing/packet.spec.md && ! rg -q "PrePass::SupportGeneration" .ralph/specs/28_tree-support-multi-layer-propagation/ .ralph/specs/30_support-planner-prepass-wit-plumbing/ && ! rg -q "run-support-generation" .ralph/specs/28_tree-support-multi-layer-propagation/ .ralph/specs/30_support-planner-prepass-wit-plumbing/`
- **Given** `.ralph/specs/31b_*/packet.spec.md`, **when** the file is read, **then** frontmatter shows `status: draft`, the file carries the HEAD note "Dependency rebased onto 31a-REV2", and references to `PrePass::SupportGeneration` / `run-support-generation` are normalized to the new vocabulary. | `rg -q "^status: draft" .ralph/specs/31b_support-planner-algorithmic-parity/packet.spec.md && rg -q "Dependency rebased onto 31a-REV2" .ralph/specs/31b_support-planner-algorithmic-parity/packet.spec.md && ! rg -q "PrePass::SupportGeneration" .ralph/specs/31b_support-planner-algorithmic-parity/`
- **Given** the workspace, **when** the backpressure gates run, **then** `cargo build --workspace`, `cargo test --workspace`, and `cargo clippy --workspace -- -D warnings` all exit 0. | `cargo build --workspace && cargo test --workspace && cargo clippy --workspace -- -D warnings`
- **Given** the WASM core modules build script, **when** invoked, **then** it builds the repurposed `support-planner:support_planner_guest` against the new `run-support-geometry` export and exits 0. | `./modules/core-modules/build-core-modules.sh`
- **Given** `modules/core-modules/paint-segmentation/wit-guest/src/lib.rs`, **when** the inline WIT world block is read, **then** it declares the `support-geometry-output` record (not the obsolete `support-generation-output` resource) and the `run-support-geometry` export. | `rg -q "support-geometry-output" modules/core-modules/paint-segmentation/wit-guest/src/lib.rs && ! rg -q "support-generation-output|run-support-generation" modules/core-modules/paint-segmentation/wit-guest/src/lib.rs`
- **Given** a prepass execution plan whose stage list omits both `PrePass::LayerPlanning` and `PrePass::SurfaceClassification` but includes `PrePass::SupportGeometry`, **when** `execute_prepass_with_builtins` runs, **then** the run completes without `commit_region_mapping_builtin`-related errors and without `LayerPlanIR`-missing or `SupportGeometryIR`-missing prerequisite errors. | `cargo test -p slicer-host --test prepass_execution_order_tdd -- --exact tree_support_plan_succeeds_without_layer_planning_stage`

## Negative Test Cases

- **Given** a module manifest declaring `stage.id = "PrePass::SupportGeneration"`, **when** the host loads it, **then** load fails with `PrepassExecutionError::UnknownStage` (or the project's named variant) referencing the removed stage id. | `cargo test -p slicer-host --test manifest_unknown_stage_tdd -- --exact pre_pass_support_generation_manifest_rejected`
- **Given** a guest module that exports the obsolete `run-support-generation` symbol, **when** the host introspects the WIT instance, **then** instantiation fails with a missing-import / unknown-export error rather than silently routing to a host stub. | `cargo test -p slicer-host --test wit_instantiation_tdd -- --exact obsolete_run_support_generation_export_rejected`
- **Given** the workspace, **when** `rg` searches for the obsolete identifiers in non-admonition contexts, **then** zero matches are reported across `crates/`, `modules/`, `wit/`, and `docs/`; the only remaining matches are inside the explicit HEAD admonitions of packets `28`, `30`, `31a`, `31a-REV1` (where they are part of the historical-record explanation of the rename). | `! rg -q "PrePass::SupportGeneration|run-support-generation|run_support_generation|SupportGenerationOutput" crates modules wit docs`

## Verification

- `cargo build --workspace` — top-level compile gate; dispatch as FACT pass/fail.
- `cargo test --workspace` — full test suite gate; dispatch and consume only test-name + assertion on failure.
- `cargo clippy --workspace -- -D warnings` — lint gate; dispatch as FACT pass/fail.
- `./modules/core-modules/build-core-modules.sh` — WASM core-module build gate; dispatch as FACT pass/fail.
- `rg "PrePass::SupportGeneration|run-support-generation|run_support_generation|SupportGenerationOutput" crates modules wit docs` — global zero-hit sweep; dispatch as FACT (count or empty).
- `rg "PrePass::SupportGeneration|run-support-generation" .ralph/specs/` — spec-tree sweep; dispatch as LOCATIONS, all hits must be inside HEAD-admonition lines explaining the rename.

## Authoritative Docs

- `docs/01_system_architecture.md` — read lines 100–230, 370–410, 525–540 only (full file > 500 lines; delegate full reads). Sections: Tier 1 stage definitions, Stage I/O Contract table, Cross-Stage Dependency Matrix, Claim System.
- `docs/02_ir_schemas.md` — read lines 75–85, 680–700 only (full file > 700 lines). Sections: `MeshIR.ObjectMesh` doc comment; `SupportPlanIR` definition and producer attribution.
- `docs/03_wit_and_manifest.md` — read lines 540–565 only. Section: WIT prepass export catalogue.
- `docs/04_host_scheduler.md` — read lines 95–110, 660–680, 905–920. Sections: `STAGE_ORDER`, `required_slots()` table, lifecycle diagram.
- `docs/05_module_sdk.md` — read lines 130–220. Section: `PrepassModule` trait and manifest examples.
- `docs/07_implementation_status.md` — read lines 95–105 only (TASK-161 / TASK-162 / TASK-163 rows). Edit TASK-161 in place.
- `docs/10_glossary_and_scenario_traces.md` — read lines 25–35, 125–160. Sections: glossary entry; scenario traces involving support stages.

For each doc the implementer must delegate full-file reads via SUMMARY when ranged reads aren't sufficient. Doc reads outside the listed ranges are not justified by this packet.

## OrcaSlicer Reference Obligations

- None. This packet is a structural revert and consolidation. No new behavior is being ported from OrcaSlicer; nothing to compare. Implementer must not load anything from `OrcaSlicerDocumented/`.

## Packet Files

- `requirements.md` — measurable outcomes, cross-packet impact ledger, predecessor-AC absorption table.
- `design.md` — selected approach (host-first → guest reads `SupportGeometryIR`), code change surface, out-of-bounds list.
- `implementation-plan.md` — 12 atomic steps with files-to-read / files-to-edit / dispatches / context cost / verification per step.
- `task-map.md` — TASK-161 / TASK-162 / TASK-163-foundation → step mapping; required because the packet spans three task IDs and supersedes two prior packets while normalizing references in three more.

## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list — do not extend it without a documented justification;
- honor `design.md`'s out-of-bounds list — `OrcaSlicerDocumented/`, `target/`, `Cargo.lock`, `crates/slicer-host/tests/benchy_end_to_end_tdd.rs` (read only the ≤ 1 affected comment line, do not load full file), and unrelated crates must not be loaded directly;
- delegate every `cargo` invocation and authoritative-doc fact-check; consume only FACT pass/fail or SNIPPETS on failure;
- delegate the codebase comment sweep per-file; the implementer should adjudicate returned diffs, not load the source files in full;
- delegate the spec-packet sweep per-packet; never load all five predecessor packet directories simultaneously;
- stop reading at 60% context and hand off at 85% — at 85% emit a numbered handoff block listing completed steps, current state, and the next concrete action.

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. No single step is rated `L`; if the implementer finds themselves needing to escalate any step to `L`, they must split before proceeding.
