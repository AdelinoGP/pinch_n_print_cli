---
status: draft
packet: 217-support-type-variants
task_ids:
  - TASK-326
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 217-support-type-variants

## Goal

Make `support_type` select an explicit auto/manual planner mode while preserving the existing tree-versus-traditional `support-generator` claim winner.

## Scope Boundaries

This packet changes support-planner configuration and contact collection only. The scheduler's `support_generator_preferred_module_id` remains the two-way tree/hybrid versus traditional resolver, and fallback geometry, raft geometry, interface geometry, and G-code emission remain out of scope.

## Prerequisites and Blockers

- FORWARD-DEP (activation blocker): this packet cannot activate until `support-planner-defect-fix` (`TASK-322`) is implemented; it consumes that packet's `MIN_BRANCH_RADIUS` / planner-geometry baseline.
- Unblocks: `TASK-327` / packet `support-gcode-e2e`.
- Activation blockers: `[BLOCK]` `TASK-322` / `support-planner-defect-fix` is still draft and must be implemented before this packet activates.

## Acceptance Criteria

- **AC-1. Given** `tmp/visual-debug-tree.json` uses `support_type: "tree(auto)"` and the model has an overhang with no painted enforcer, **when** the model-mode visual-debug request runs, **then** `manifest.json` has a non-empty `PrePass::SupportGeometry` PNG entry for each requested layer `10`, `125`, and `130`, a non-empty `Layer::Support` capture for each of those layers whose `typed_capture.value.support_paths` is non-empty, and the scheduler selects `com.core.tree-support`. The planner PNGs are the geometry evidence for detected-overhang contacts because this tap's typed capture is not serializable. | `cargo test -p slicer-scheduler --all-targets support_type_tree_selects_tree_support_holder && cargo run -q -p pnp-cli --bin pnp_cli -- visual-debug --request tmp/visual-debug-tree.json --output target/vd-support-type-auto --overwrite >/dev/null && jq -e '. as $m | ([10,125,130] | all(.[]; . as $layer | any($m.images[]; .tap == "PrePass::SupportGeometry" and .layer_index == $layer and ((.png_path // "") | length > 0)))) and ([10,125,130] | all(.[]; . as $layer | any($m.images[]; .tap == "Layer::Support" and .layer_index == $layer and ((.typed_capture.value.support_paths // []) | length > 0))))' target/vd-support-type-auto/manifest.json`
- **AC-2. Given** `support_type` is `classic(auto)` or `normal(auto)` and the support planner receives an overhang mesh with no painted enforcer, **when** the model-mode visual-debug request runs, **then** the scheduler selects `com.core.traditional-support` and the `Layer::Support` output is present only for detected overhang eligibility rather than the model's full cross-section. | `cargo test -p slicer-scheduler --all-targets support_type_normal_falls_back_to_traditional_support_holder && rg -q 'support_type|overhang_areas' modules/core-modules/traditional-support/src/lib.rs`
- **AC-3. Given** `tmp/visual-debug-support-manual.json` uses `support_type: "tree(manual)"` and the model has painted `support_enforcer` facets plus unpainted overhang facets, **when** the model-mode visual-debug request runs, **then** `manifest.json` has a non-empty `PrePass::SupportGeometry` PNG entry for each requested layer `0`, `10`, and `30`, a non-empty `Layer::Support` capture for each of those layers whose `typed_capture.value.support_paths` carries the exact generated path coordinates, and `manual_mode_uses_enforcer_contacts_only` proves no contact is created from `detect_overhang_facets`; the support tap has no entity corresponding solely to the unpainted overhang. The planner PNGs are the geometry evidence because this tap's typed capture is null. | `cargo test -p support-planner --all-targets --test to_buildplate_tdd manual_mode_uses_enforcer_contacts_only && cargo run -q -p pnp-cli --bin pnp_cli -- visual-debug --request tmp/visual-debug-support-manual.json --output target/vd-support-type-manual --overwrite >/dev/null && jq -e '. as $m | ([0,10,30] | all(.[]; . as $layer | any($m.images[]; .tap == "PrePass::SupportGeometry" and .layer_index == $layer and ((.png_path // "") | length > 0)))) and ([0,10,30] | all(.[]; . as $layer | any($m.images[]; .tap == "Layer::Support" and .layer_index == $layer and ((.typed_capture.value.support_paths // []) | length > 0))))' target/vd-support-type-manual/manifest.json`
- **AC-4. Given** `support_type` is `tree(auto)` or `tree(manual)`, **when** the scheduler resolves the claim, **then** both values resolve to `com.core.tree-support`, and given `classic(auto)` or `classic(manual)`, both resolve to `com.core.traditional-support`; no new claim holder or third support module is introduced. | `cargo test -p slicer-scheduler --all-targets support_type_tree_manual_selects_tree_support_holder && cargo test -p slicer-scheduler --all-targets support_type_tree_selects_tree_support_holder && cargo test -p slicer-scheduler --all-targets support_type_normal_falls_back_to_traditional_support_holder && rg -q 'fn support_generator_preferred_module_id' crates/slicer-scheduler/src/execution_plan.rs`

## Negative Test Cases

- **AC-N1. Given** `support_type` is absent or an unrecognized string, **when** module loading resolves the `support-generator` claim, **then** the existing fallback remains `com.core.traditional-support` and the planner does not treat the value as manual mode. | `cargo test -p slicer-scheduler --all-targets support_type_absent_defaults_to_traditional_support_holder support_type_unrecognized_value_falls_back_to_traditional_support_holder`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-scheduler --all-targets support_type_tree_manual_selects_tree_support_holder`

## Authoritative Docs

- `docs/specs/support-generation-remediation-plan.md` - direct full read; approved queue row 5 and resolved 4-variant decision.
- `docs/specs/support-generation-defect-verified-findings.md` - direct ranges 28-55, 157-176, and 178-231; verified architecture, mode intent, and visual-debug evidence.
- `docs/00_project_overview.md` - direct ranges 86-114 and 130-152; normative document map and crate roles.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp` — confirm `support_type` auto/manual semantics and that automatic detection is distinct from painted enforcers.
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` — compare the tree-support mode boundary without changing PNP's two-way claim split.

## Doc Impact Statement (Required)

**`none`** - this packet changes planner behavior and module configuration plumbing without changing IR, WIT, scheduler claim names, or manifest contracts.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
