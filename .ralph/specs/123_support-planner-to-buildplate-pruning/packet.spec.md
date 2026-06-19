---
status: draft
packet: 123
task_ids:
  - TASK-264
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: support-planner-to-buildplate-pruning

## Goal

Add `to_buildplate: bool` tracking to `PlannedSupportNode`, prune branches whose move target lies inside `collision_polys` AND cannot reach the build plate, and honor `support_on_build_plate_only` config: when true, every contact whose `to_buildplate = false` is rejected at creation time.

## Scope Boundaries

Touches `support-planner/src/lib.rs` — `PlannedSupportNode` gains one field, the contact-creation path sets it based on the contact XY's relationship to the object's projected footprint, the propagation pruning logic uses it, and the `support_on_build_plate_only` config is plumbed through. Adds a build-plate-only invariant to the wedge harness. No IR change (the `to_buildplate` flag is internal to the planner).

## Prerequisites and Blockers

- Depends on: `122_support-planner-multi-neighbour-mst` (pruning interacts with merge geometry).
- Unblocks: future work toward `to_model`-vs-`to_buildplate` strategy selection (deferred).
- Activation blockers: predecessor packet `status: implemented`.

## Acceptance Criteria

- **AC-1. Given** `modules/core-modules/support-planner/src/lib.rs`, **when** searched, **then** `struct PlannedSupportNode` contains a field `to_buildplate: bool`. | `rg -q 'to_buildplate' modules/core-modules/support-planner/src/lib.rs && rg -A2 'struct PlannedSupportNode' modules/core-modules/support-planner/src/lib.rs | rg -q 'to_buildplate'`
- **AC-2. Given** the contact creation path in `plan_for_object`, **when** searched, **then** new contacts have `to_buildplate` set to `true` if the contact XY lies outside the object's projected footprint at that layer, `false` otherwise. | `cargo test -p support-planner --test to_buildplate_tdd -- contact_xy_outside_footprint_sets_to_buildplate_true --nocapture 2>&1 | tee target/test-output.log`
- **AC-3. Given** a node whose `to_buildplate = true` AND whose move target lies inside `collision_polys`, **when** the propagation runs, **then** the node is pruned (dropped from `active_nodes` and not propagated to the next layer). | `cargo test -p support-planner --test to_buildplate_tdd -- unreachable_buildplate_node_pruned --nocapture 2>&1 | tee target/test-output.log`
- **AC-4. Given** `support-planner` with `support_on_build_plate_only = true` and an overhang contact at XY inside the object's projected footprint, **when** `plan_for_object` runs, **then** the contact is NOT added to `contacts_by_layer` (rejected at creation). | `cargo test -p support-planner --test to_buildplate_tdd -- buildplate_only_rejects_to_model_contacts --nocapture 2>&1 | tee target/test-output.log`
- **AC-5. Given** `support-planner.toml [config.schema]`, **when** searched, **then** the entry `support_on_build_plate_only` (type bool, default false) is present. | `rg -q 'support_on_build_plate_only' modules/core-modules/support-planner/support-planner.toml`
- **AC-6. Given** `support_invariants_wedge_tdd.rs`, **when** searched, **then** a new `#[test] fn build_plate_only_emits_no_to_model_branches` exists. It sets `support_on_build_plate_only = true` on the wedge fixture's config, runs the planner, and asserts every branch endpoint chain terminates at the build plate (not on the model). | `rg -q 'fn build_plate_only_emits_no_to_model_branches' crates/slicer-runtime/tests/integration/support_invariants_wedge_tdd.rs`
- **AC-7. Given** the new wedge invariant, **when** run, **then** it PASSES. | `cargo test -p slicer-runtime --test support_invariants_wedge_tdd -- build_plate_only_emits_no_to_model_branches --nocapture 2>&1 | tee target/test-output.log`
- **AC-8. Given** the regenerated wedge goldens, **when** the golden-regression test runs, **then** the tolerance check PASSES (the default config has `support_on_build_plate_only = false`, so the goldens shift only minimally — by the pruning of any to-model branches that were already coincidentally being dropped by `clamp_to_avoidance`). | `cargo test -p slicer-runtime --test support_golden_regression_wedge_tdd 2>&1 | tee target/test-output.log`
- **AC-9. Given** all previous wedge invariants, **when** run after this packet's changes, **then** ALL PASS. | `cargo test -p slicer-runtime --test support_invariants_wedge_tdd 2>&1 | tee target/test-output.log`

## Negative Test Cases

- **AC-N1. Given** `support_on_build_plate_only = false` (default) AND a contact at XY inside the object's projected footprint, **when** the planner runs, **then** the contact IS added (no rejection) and the node has `to_buildplate = false`. | `cargo test -p support-planner --test to_buildplate_tdd -- default_config_does_not_reject_to_model_contacts --nocapture 2>&1 | tee target/test-output.log`

## Verification

- `cargo xtask build-guests --check`
- `cargo test -p support-planner --test to_buildplate_tdd 2>&1 | tee target/test-output.log`
- `cargo test -p slicer-runtime --test support_invariants_wedge_tdd 2>&1 | tee target/test-output.log`
- `cargo test -p slicer-runtime --test support_golden_regression_wedge_tdd 2>&1 | tee target/test-output.log`

## Authoritative Docs

- `docs/specs/support-modules-orca-port.md` §C5 — directly.

## Doc Impact Statement (Required)

- `docs/specs/support-modules-orca-port.md` §Validation Strategy — append the new invariant to the v1 list (Step 5 of the implementation plan). Verification: `rg -q 'build_plate_only_emits_no_to_model_branches' docs/specs/support-modules-orca-port.md`.

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
