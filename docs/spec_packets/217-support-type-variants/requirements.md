# Requirements: support-type-variants

## Packet Metadata

- Grouped task IDs: `TASK-326`
- Backlog source: `docs/07_implementation_status.md` (queue row 5 in the approved remediation plan)
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

The scheduler already resolves `support_type` to one of two claim holders, but the planner does not distinguish auto from manual variants: it always collects both detected overhang facets and painted enforcers. This slice introduces that missing mode boundary without changing the established two-module claim split.

## In Scope

- Preserve `SUPPORT_GENERATOR_CONFIG_KEY` (`"support_type"`) and `support_generator_preferred_module_id` behavior: values beginning with `tree` or `hybrid` select `com.core.tree-support`; all other and absent values select `com.core.traditional-support`.
- Make the support-planner read the raw `support_type` value through its `ConfigView` and derive an auto/manual mode, with auto as the existing default for recognized auto values and manual selecting enforcers only.
- Add or update the support-planner manifest config schema needed for `support_type` to reach the guest planner.
- In manual mode, skip `detect_overhang_facets` and use only `collect_paint_enforcer_contacts`; retain blocker handling and propagation for enforcer contacts.
- Add targeted planner/scheduler regression tests and model-mode visual-debug gates covering auto overhangs and manual enforcers-only behavior. The gates use `tmp/visual-debug-tree.json` and `tmp/visual-debug-support-manual.json`, request both `PrePass::SupportGeometry` and `Layer::Support`, assert a non-empty PNG entry for every requested `PrePass::SupportGeometry` layer, and assert exact non-empty `Layer::Support` `typed_capture.value.support_paths` paths in `manifest.json`.

## Out of Scope

- Changing `support_generator_preferred_module_id`, claim names, module IDs, or the two-way module split.
- RC-1/RC-4 geometry fixes, fallback clipping, `needs_support`, raft/interface geometry, and G-code emission.
- Numerical Orca parity for branch positions or radii.

## Authoritative Docs

- `docs/specs/support-generation-remediation-plan.md` - direct full read; approved decision and dependency order.
- `docs/specs/support-generation-defect-verified-findings.md` - direct ranges 28-55 and 157-176; verified claim resolution and mode fix direction.
- `docs/01_system_architecture.md` - delegated targeted summary for support stages and claim ownership.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp` — confirm automatic overhang detection versus manual/enforcer support policy.
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` — compare tree mode's auto/manual contact source boundary.

## Acceptance Summary

- Positive: `AC-1` through `AC-4` in `packet.spec.md`.
- Negative: `AC-N1` in `packet.spec.md`.
- Cross-packet impact: exports `SupportGenerationMode` and the planner's mode-selection behavior for packet 6's geometry-to-G-code verification.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-scheduler --all-targets support_type_tree_manual_selects_tree_support_holder` | Preserve tree manual claim resolution | FACT pass/fail |
| `cargo test -p support-planner --all-targets --test to_buildplate_tdd` | Planner mode and contact-source regression coverage | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo run -q -p pnp-cli --bin pnp_cli -- visual-debug --request tmp/visual-debug-tree.json --output target/vd-support-type-auto --overwrite >/dev/null && jq -e '. as $m | ([10,125,130] | all(.[]; . as $layer | any($m.images[]; .tap == "PrePass::SupportGeometry" and .layer_index == $layer and ((.png_path // "") | length > 0)))) and ([10,125,130] | all(.[]; . as $layer | any($m.images[]; .tap == "Layer::Support" and .layer_index == $layer and ((.typed_capture.value.support_paths // []) | length > 0))))' target/vd-support-type-auto/manifest.json` | Visual-debug auto gate: PNG evidence for planner tap and typed paths for consumer tap | FACT manifest assertion |
| `cargo run -q -p pnp-cli --bin pnp_cli -- visual-debug --request tmp/visual-debug-support-manual.json --output target/vd-support-type-manual --overwrite >/dev/null && jq -e '. as $m | ([0,10,30] | all(.[]; . as $layer | any($m.images[]; .tap == "PrePass::SupportGeometry" and .layer_index == $layer and ((.png_path // "") | length > 0)))) and ([0,10,30] | all(.[]; . as $layer | any($m.images[]; .tap == "Layer::Support" and .layer_index == $layer and ((.typed_capture.value.support_paths // []) | length > 0))))' target/vd-support-type-manual/manifest.json` | Visual-debug manual gate: PNG evidence for planner tap and exact typed paths for consumer tap | FACT manifest assertion |
| `cargo check --workspace --all-targets` | Compile all affected targets | FACT pass/fail |

## Step Completion Expectations

- Auto mode remains the default for existing `support_type` values that select a module and for absent planner mode input.
- Scheduler module selection is independently tested from planner contact-source selection.
- Visual-debug evidence reads `manifest.json` before PNGs and checks auto versus manual support presence by exact tap/layer entries.

## Context Discipline Notes

Do not load generated `support-planner.wasm`; source edits feed guest WASM and require the staleness gate. Keep Orca reads delegated and restrict visual-debug output to `manifest.json` plus bounded assertions.
