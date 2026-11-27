---
status: draft
packet: 298-organic-tree-support-keys
task_ids: []
backlog_source: docs/specs/orca-feature-gap/issues/79-author-packet-p72-support-tree-supports-tree-support.md (wayfinder map: Close the OrcaSlicer FFF feature gap)
context_cost_estimate: M
copy_note: Queue packet from the wayfinder map "Close the OrcaSlicer FFF feature gap"; authored under ticket 79 (P72).
---

# Packet Contract: 298-organic-tree-support-keys

## Goal

Make the 8 P72 organic tree-support keys drive the tree pipeline at canonical value semantics — six organic branch parameters selecting the planner's geometry on the explicit-organic style path, two brim keys driving a new first-layer brim stage in the renderer — with zero declaration-only keys and default-path identity for every classic style.

## Scope Boundaries

P72 is eight Tier B keys owned by the two tree modules (`tree-support-planner`, `tree-support`), all live in canonical's slicing pipeline and zero-occurrence as behaviour in this tree. Claim-time grounding (ticket 79) holds all eight in: six are read only by the organic engine (`TreeSupportCommon.hpp` settings constructor, `TreeSupport3D.cpp` area generation), which this port does not implement (DEV-156 Strong substitution stands), and two are read by the classic engine's `TreeSupport::draw_circles` brim path, which this port never built. The packet wires the six params into the planner behind the explicit-organic style gate (recorded divergence DEV-189 — canonical would also use them for default/grid/snug-on-tree) and builds the renderer brim stage on the same gate, so classic styles and defaults stay byte-identical. Rule 4 does not fire: the values parameterise one planner's internal style-gated selection plus one renderer's stage, not cross-module algorithm selection — there are no claim holders to create, mirroring the `seam_position` precedent in the map Notes.

## Prerequisites and Blockers

- Depends on: nothing. All symbols below are live on HEAD (verified at authoring); `SupportPlanner::from_config` / `TreeSupport::from_config`, the style helpers, the `ConfigView` accessors, and both manifests are landed tree code, not packet dependencies. Exact canonical formulas the wiring mirrors are borrowed via implementer-side delegated oracle reads (design.md lists them).
- Related work, not a blocker: ticket 122 (prime-tower body — no shared helper, no dep); ticket 124 (print-validate-level stage — the `Print.cpp` diameter/tip validations ride it, named non-borrow); ticket 125 (per-tool model — explicitly NOT this packet's axis: all eight keys are canonical scalars, carried by the existing per-object overlay, 290/291 precedent); ticket 132 (CONFIG_BLOCK reader contract — canonical spellings ride there; no padding edits here, rule 2); remediation-plan row 7 `organic-tree-engine` (the full `TreeSupport3D.cpp` port — this packet does NOT build it and does NOT retire DEV-156).
- Unblocks: wayfinder ticket 79 (P72 closes when this packet is authored). No edge to any other draft packet (238b is `implemented` and owns none of these keys; no draft packet mentions any of the eight outside a handoff suggestion).
- Activation blockers: none. One deviation ID is minted (DEV-189, verified absent from the log and all drafts at authoring); if implementation surfaces another, re-derive `max(DEV-*)` over `docs/DEVIATION_LOG.md` + `docs/spec_packets/*/` before writing the row.

## Acceptance Criteria

- **AC-1. Given** the two tree module manifests, **when** their `[config.schema]` sections are inspected, **then** the planner declares `tree_support_angle_slow` (float, default 25.0), `tree_support_branch_angle_organic` (float, default 40.0), `tree_support_branch_diameter_organic` (float, default 2.0), `tree_support_branch_distance_organic` (float, default 1.0), `tree_support_tip_diameter` (float, default 0.8), `tree_support_top_rate` (percent, default `"30%"`), and the renderer declares `tree_support_auto_brim` (bool, default true), `tree_support_brim_width` (float, default 3.0), plus the `support_style` gate row (enum, default `"default"`, all 7 canonical values). | `rg -q '\[config\.schema\.tree_support_angle_slow\]' modules/core-modules/tree-support-planner/tree-support-planner.toml && rg -q 'default = 25\.0' modules/core-modules/tree-support-planner/tree-support-planner.toml && rg -q '\[config\.schema\.tree_support_branch_angle_organic\]' modules/core-modules/tree-support-planner/tree-support-planner.toml && rg -q 'default = 40\.0' modules/core-modules/tree-support-planner/tree-support-planner.toml && rg -q '\[config\.schema\.tree_support_branch_diameter_organic\]' modules/core-modules/tree-support-planner/tree-support-planner.toml && rg -q 'default = 2\.0' modules/core-modules/tree-support-planner/tree-support-planner.toml && rg -q '\[config\.schema\.tree_support_branch_distance_organic\]' modules/core-modules/tree-support-planner/tree-support-planner.toml && rg -q 'default = 1\.0' modules/core-modules/tree-support-planner/tree-support-planner.toml && rg -q '\[config\.schema\.tree_support_tip_diameter\]' modules/core-modules/tree-support-planner/tree-support-planner.toml && rg -q 'default = 0\.8' modules/core-modules/tree-support-planner/tree-support-planner.toml && rg -q '\[config\.schema\.tree_support_top_rate\]' modules/core-modules/tree-support-planner/tree-support-planner.toml && rg -q 'default = "30%"' modules/core-modules/tree-support-planner/tree-support-planner.toml && rg -q '\[config\.schema\.tree_support_auto_brim\]' modules/core-modules/tree-support/tree-support.toml && rg -q 'default = true' modules/core-modules/tree-support/tree-support.toml && rg -q '\[config\.schema\.tree_support_brim_width\]' modules/core-modules/tree-support/tree-support.toml && rg -q 'default = 3\.0' modules/core-modules/tree-support/tree-support.toml && rg -q '\[config\.schema\.support_style\]' modules/core-modules/tree-support/tree-support.toml 2>&1 | tee target/test-output.log | tail -3`
- **AC-2. Given** a tree-support fixture sliced with a classic style (`tree_strong`) and non-default organic keys set (`tree_support_branch_angle_organic = 10.0`, `tree_support_branch_diameter_organic = 9.0`), **when** its `SupportPlanIR` is compared to the same fixture without the organic keys, **then** the plans are identical (organic keys are inert off the explicit-organic path). | `cargo test -p tree-support-planner --test organic_params_tdd organic_keys_inert_under_classic_style 2>&1 | tee target/test-output.log | tail -5`
- **AC-3. Given** explicit `support_style = organic` on the same fixture, **when** `tree_support_branch_angle_organic` is `10.0` versus `60.0` (all else default), **then** the two `SupportPlanIR` outputs differ in branch geometry, and both differ from the `tree_support_branch_angle`-driven classic run (the organic set is selected, not the classic set). | `cargo test -p tree-support-planner --test organic_params_tdd explicit_organic_selects_organic_set 2>&1 | tee target/test-output.log | tail -5`
- **AC-4. Given** explicit `support_style = organic`, **when** `tree_support_tip_diameter = 9.0` exceeds `tree_support_branch_diameter_organic = 2.0`, **then** the tip radius saturates at the branch radius (canonical tip-clamped-to-diameter), and a non-default `tree_support_angle_slow` run differs from the default-`25.0` run (the slow cap drives the plan). | `cargo test -p tree-support-planner --test organic_params_tdd tip_clamped_to_diameter_and_slow_cap_drives_plan 2>&1 | tee target/test-output.log | tail -5`
- **AC-5. Given** explicit `support_style = organic`, **when** `tree_support_top_rate` is `"10%"` versus `"30%"` (default), **then** the top-contact branch radii of the `"10%"` run are strictly smaller and the `"30%"` run matches the unset-key run (percent string reaches the guest-side scaling). | `cargo test -p tree-support-planner --test organic_params_tdd top_rate_percent_scales_top_radius 2>&1 | tee target/test-output.log | tail -5`
- **AC-6. Given** explicit `support_style = organic` with `tree_support_auto_brim = false`, **when** the renderer runs with `tree_support_brim_width = 5.0` versus `0.0`, **then** the `5.0` run emits brim loops of width 5.0 mm around build-plate-contact tree bases and the `0.0` run emits none (identical support bodies otherwise). | `cargo test -p tree-support --test tree_brim_tdd fixed_brim_width_drives_base_loops 2>&1 | tee target/test-output.log | tail -5`
- **AC-7. Given** explicit `support_style = organic` with default `tree_support_auto_brim = true`, **when** the renderer runs over tapered nodes, **then** the derived brim width varies with node radius (it differs from the fixed-`3.0` run on at least one base), while a classic-style run with identical brim keys emits no brim loops at all (gate holds in the renderer). | `cargo test -p tree-support --test tree_brim_tdd auto_brim_derives_width_and_gates_on_organic 2>&1 | tee target/test-output.log | tail -5`
- **AC-8. Given** the packet's disposition table and the deviation log, **when** they are read, **then** the table lists exactly the 8 P72 keys, all wired, with zero declaration-only keys, `DEV-189` names this packet's three divergences in `docs/DEVIATION_LOG.md`, the regenerated `docs/15` tables contain the new keys, and the doc-15 freshness gate passes. | `rg -q 'declaration-only keys: 0' docs/spec_packets/298-organic-tree-support-keys/requirements.md && rg -q 'DEV-189' docs/DEVIATION_LOG.md && rg -q 'tree_support_tip_diameter' docs/15_config_keys_reference.md && cargo xtask gen-config-docs --check 2>&1 | tee target/test-output.log | tail -5`

## Negative Test Cases

- **AC-N1. Given** explicit `support_style = organic` with `tree_support_branch_diameter_organic = 0.1` (below the canonical 1.0 minimum), **when** the planner builds, **then** construction succeeds (no rejection — canonical minima are GUI hints, ticket-113 rule) and the effective diameter saturates at the structural floor rather than erroring. | `cargo test -p tree-support-planner --test organic_params_tdd below_min_saturates_without_rejection 2>&1 | tee target/test-output.log | tail -5`
- **AC-N2. Given** explicit `support_style = organic`, **when** `organic_substitution_requested` is evaluated, **then** it still returns true (the DEV-156 Strong substitution and its once-per-slice code-1005 Warn condition survive this packet — the packet makes the params live, not the engine). | `cargo test -p tree-support-planner --test organic_params_tdd substitution_gate_survives 2>&1 | tee target/test-output.log | tail -5`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p tree-support-planner --test organic_params_tdd 2>&1 | tee target/test-output.log | tail -5`

## Authoritative Docs

- `docs/01_system_architecture.md` - delegated SUMMARY (support-family claim seam; rule-4 trigger test does not fire).
- `docs/08_coordinate_system.md` - direct range read (mm↔unit boundary helpers for brim/radius geometry).
- `docs/DEVIATION_LOG.md` - direct read of DEV-156 row (substitution this packet preserves) + ID-convention sample.
- `docs/specs/support-generation-remediation-plan.md` - delegated SUMMARY (row 7 organic-engine scope boundary; no packet to collide with).

## Doc Impact Statement (Required)

- `docs/DEVIATION_LOG.md` section "DEV-189" - `rg -q 'DEV-189' docs/DEVIATION_LOG.md`.
- `docs/15_config_keys_reference.md` generated tables gain the 8 keys - `rg -q 'tree_support_tip_diameter' docs/15_config_keys_reference.md`.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupportCommon.hpp` — organic settings-constructor semantics for the six param keys (radian conversion, clamps, tip≤diameter, top-rate application, slow-angle cap).
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` — `TreeSupport::draw_circles` brim path (auto-derived vs fixed width selection, first-layer/base conditions).
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — canonical defaults/min/max for all eight keys (already captured in requirements.md; re-verify before mirroring).
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport3D.cpp` — `generate_support_areas` enforcer-overhang-offset use of tip diameter (named non-borrow: volumetric engine not ported).

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
