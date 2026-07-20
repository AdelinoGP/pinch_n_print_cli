---
status: draft
packet: 123
task_ids:
  - TASK-288
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: support-planner-to-buildplate-pruning

## Goal

Add `to_buildplate: bool` tracking to `PlannedSupportNode` (defined at `modules/core-modules/support-planner/src/lib.rs:92` with current fields `x, y, dist_to_top`), prune branches whose move target lies inside `collision_polys` AND cannot reach the build plate, and honor a new `support_on_build_plate_only` config (default `false`): when true, every contact whose `to_buildplate` would be `false` is rejected at creation time. Add a `build_plate_only_emits_no_to_model_branches` invariant to the wedge harness that asserts: with `support_on_build_plate_only = true` forced on, every emitted branch endpoint chain terminates at the build plate, not on the model.

## Scope Boundaries

Touches `modules/core-modules/support-planner/src/lib.rs` — `PlannedSupportNode` gains one field (line 92), the contact-creation path at lines 380-416 sets it based on the contact XY's relationship to `SupportGeometryView.outlines` for the object at that layer, the propagation pruning at lines 711-723 (the `point_in_any_expoly(collision_polys, cx, cy)` branch where nodes are dropped) is extended to also prune when `to_buildplate = true` AND clamped target is in `collision_polys`, and `support_on_build_plate_only` is plumbed through `on_print_start` (line 114) and a new struct field on `SupportPlanner` (line 68-90). Adds the wedge invariant and a `support_on_build_plate_only: bool` (default `false`) entry to `support-planner.toml` `[config.schema]`. The `to_buildplate` flag is internal to the planner; it does NOT cross the IR boundary.

## Prerequisites and Blockers

- Depends on: `122_support-planner-multi-neighbour-mst` (TASK-287) for symmetric merges (pruning interacts with merge geometry — a merged branch where one contributor's path was pruned behaves differently from one where two were pruned). The packet's pruning logic does not strictly require TASK-287 to land first, but the soft dependency keeps the dependency graph intact.
- Unblocks: future work toward `to_model`-vs-`to_buildplate` strategy selection (deferred).
- Activation blockers: none hard. The packet's pruning logic is independent of the propagation algorithm: it operates on the per-layer active-nodes set (lines 711-723) regardless of how the move target was synthesized (single-neighbour or multi-neighbour). The packet 122 multi-neighbour aggregation is a **soft dependency** — the new prune rule works against both. If 122 lands first, the `to_buildplate` flag's propagation across multi-neighbour aggregations follows the rule in `requirements.md` §Step Completion Expectations. If 123 lands first, the propagation is single-neighbour and the rule is trivial: a moved node inherits its source's `to_buildplate`.

## Acceptance Criteria

- **AC-1. Given** `modules/core-modules/support-planner/src/lib.rs`, **when** searched, **then** `struct PlannedSupportNode` (line 92) contains a field `to_buildplate: bool`. | `rg -A5 'struct PlannedSupportNode' modules/core-modules/support-planner/src/lib.rs | rg -q 'to_buildplate'`
- **AC-2. Given** the contact creation path in `plan_for_object` (lines 380-416), **when** searched, **then** new contacts have `to_buildplate` set to `true` if the contact XY lies outside the object's projected footprint at that layer, `false` otherwise. The "object's projected footprint at layer L" is sourced from `SupportGeometryView.outlines` for `(obj.object_id, L)` — the same data the `LayerCollisionCache.collision_polys[L]` already carries. | `cargo test -p support-planner --test to_buildplate_tdd -- contact_xy_outside_footprint_sets_to_buildplate_true --nocapture 2>&1 | tee target/test-output.log`
- **AC-3. Given** a node whose `to_buildplate = true` AND whose move target (post-`clamp_to_avoidance`) lies inside `collision_polys`, **when** the propagation runs, **then** the node is dropped (NOT propagated to the next layer). The existing "node-clamped-out" diagnostic (code 1002 at line 712) is emitted. | `cargo test -p support-planner --test to_buildplate_tdd -- unreachable_buildplate_node_pruned --nocapture 2>&1 | tee target/test-output.log`
- **AC-4. Given** `support-planner` with `support_on_build_plate_only = true` and an overhang contact at XY inside the object's projected footprint, **when** `plan_for_object` runs, **then** the contact is NOT added to `contacts_by_layer` (rejected at creation, before the per-layer loop starts). | `cargo test -p support-planner --test to_buildplate_tdd -- buildplate_only_rejects_to_model_contacts --nocapture 2>&1 | tee target/test-output.log`
- **AC-5. Given** `support-planner.toml [config.schema]`, **when** searched, **then** the entry `support_on_build_plate_only` (type `bool`, default `false`, group `Support`) is present. | `rg -B1 -A4 'support_on_build_plate_only' modules/core-modules/support-planner/support-planner.toml`
- **AC-6. Given** `support_invariants_wedge_tdd.rs`, **when** searched, **then** a new `#[test] fn build_plate_only_emits_no_to_model_branches` exists. The test forces `support_on_build_plate_only = true` on the wedge fixture's config (via the `support_wedge::prepare_wedge_context(true)` pattern with a config override; or by setting the config key before calling the prepass), runs the planner, and asserts every branch endpoint chain terminates at `z ≤ 1e-3 mm` (the build plate) or at a contact point at the overhang's origin layer — not on the model body at intermediate z values. | `rg -q 'fn build_plate_only_emits_no_to_model_branches' crates/slicer-runtime/tests/integration/support_invariants_wedge_tdd.rs`
- **AC-7. Given** the new wedge invariant, **when** run, **then** it PASSES. | `cargo test -p slicer-runtime --test support_invariants_wedge_tdd -- build_plate_only_emits_no_to_model_branches --nocapture 2>&1 | tee target/test-output.log`
- **AC-8. Given** the regenerated wedge goldens (re-anchored by this packet — the default config has `support_on_build_plate_only = false`, so the goldens shift only minimally, by the pruning of any to-model branches that were already being dropped by `clamp_to_avoidance`), **when** the golden-regression test runs without `SUPPORT_WEDGE_REGEN_GOLDEN=1`, **then** the tolerance check PASSES (count drift ≤ 10%, endpoint Hausdorff ≤ 0.5 mm). | `cargo test -p slicer-runtime --test support_golden_regression_wedge_tdd 2>&1 | tee target/test-output.log`
- **AC-9. Given** all previous wedge invariants (7 from packet 119 + 1 curvature from packet 121 + 1 symmetry from packet 122 = 9 total), **when** run after this packet's changes, **then** ALL PASS. | `cargo test -p slicer-runtime --test support_invariants_wedge_tdd 2>&1 | tee target/test-output.log`

## Negative Test Cases

- **AC-N1. Given** `support_on_build_plate_only = false` (default) AND a contact at XY inside the object's projected footprint, **when** the planner runs, **then** the contact IS added (no rejection) and the node has `to_buildplate = false`. | `cargo test -p support-planner --test to_buildplate_tdd -- default_config_does_not_reject_to_model_contacts --nocapture 2>&1 | tee target/test-output.log`
- **AC-N2. Given** a `to_buildplate = false` node whose clamped target lies inside `collision_polys`, **when** the propagation runs, **then** the node is NOT pruned by THIS packet's logic (the existing `point_in_any_expoly(collision_polys, cx, cy)` drop still fires for ALL nodes regardless of `to_buildplate`, but the new `to_buildplate = true`-only prune condition does NOT fire for `to_buildplate = false` nodes — they continue with existing behavior). The intent: the new prune rule is strictly a *tightening* for build-plate-bound branches, not a relaxation. | `cargo test -p support-planner --test to_buildplate_tdd -- to_model_node_with_collision_not_pruned_by_new_rule --nocapture 2>&1 | tee target/test-output.log`

## Verification

- `cargo xtask build-guests --check`
- `cargo test -p support-planner --test to_buildplate_tdd 2>&1 | tee target/test-output.log`
- `cargo test -p slicer-runtime --test support_invariants_wedge_tdd 2>&1 | tee target/test-output.log`
- `cargo test -p slicer-runtime --test support_golden_regression_wedge_tdd 2>&1 | tee target/test-output.log`
- `cargo clippy --workspace --all-targets -- -D warnings`

## Authoritative Docs

- `docs/specs/support-modules-orca-port.md` §C5 — directly.
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp::TreeSupport::drop_nodes` (~line 2752 area) — confirm the `unsupported_branch_leaves` deque + branch pruning loop. DELEGATED per OrcaSlicer Reference Obligations.
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp::generate_contact_points` (~line 3486) — confirm how `to_buildplate` is initialized at contact creation. DELEGATED.
- `modules/core-modules/support-planner/src/lib.rs:92-103` — `PlannedSupportNode` struct (the field-addition site).
- `modules/core-modules/support-planner/src/lib.rs:380-416` — contact creation (the `to_buildplate` initialization site).
- `modules/core-modules/support-planner/src/lib.rs:711-723` — propagation prune block (the new prune rule's site).
- `modules/core-modules/support-planner/src/lib.rs:68-90` — `SupportPlanner` struct (the `support_on_build_plate_only` field-addition site).
- `modules/core-modules/support-planner/src/lib.rs:114-191` — `on_print_start` (the config read site).
- `modules/core-modules/support-planner/src/lib.rs:381-491` — raft block (NOT touched by this packet, but the new contact-initialization must run for raft too — confirm during Step 1).

## Doc Impact Statement (Required)

- `docs/specs/support-modules-orca-port.md` §Validation Strategy — append the `build_plate_only_emits_no_to_model_branches` invariant to the v1 invariant list (Step 5 of the implementation plan). Verification: `rg -q 'build_plate_only_emits_no_to_model_branches' docs/specs/support-modules-orca-port.md`.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp::TreeSupport::drop_nodes` (~line 2752 area) — confirm the `unsupported_branch_leaves` deque + branch pruning loop.
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp::generate_contact_points` (~line 3486) — confirm how `to_buildplate` is initialized at contact creation.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
