# Requirements: support-planner-to-buildplate-pruning

## Packet Metadata

- Grouped task IDs:
  - `TASK-264` — `to_buildplate` tracking + unsupported-branch pruning + `support_on_build_plate_only` (C5)
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

`support-planner` has no notion of which contacts must reach the build plate vs. which can rest on the model. OrcaSlicer's `TreeSupport` tracks `to_buildplate` per node; nodes that can't reach the build plate without a collision-free path are pruned (`unsupported_branch_leaves` in `TreeSupport.cpp:2752`). `support_on_build_plate_only` config (true ⇒ reject every `to_model` contact at creation time) is currently unhonored.

The current planner silently emits branches that pass through air or rest on impossible geometry whenever `clamp_to_avoidance` can't snap a target inside `avoidance_polys` — those nodes simply lose the propagation step but their ancestor segments are still emitted.

## In Scope

- Add `to_buildplate: bool` field to `PlannedSupportNode`. Default `true` for new contacts whose XY lies outside the object's projected footprint at the contact's layer; `false` otherwise.
- Compute "object's projected footprint at layer L" lazily once per layer using existing `SupportGeometryView.outlines` (this is the same data the collision/avoidance cache reads).
- Prune nodes in the propagation pass: if `node.to_buildplate == true` AND the moved target is inside `collision_polys`, drop the node. The drop propagates upward in the chain (the entire chain becomes unsupported), matching Orca's `unsupported_branch_leaves` behavior.
- Add `support_on_build_plate_only: bool` (default `false`) to `support-planner.toml [config.schema]` and read it in `on_print_start`.
- When the config is `true`: at contact creation in `plan_for_object`, reject every contact whose `to_buildplate` would be `false`.
- Add wedge harness invariant `build_plate_only_emits_no_to_model_branches` (forces the config to `true` and asserts).
- Add `modules/core-modules/support-planner/tests/to_buildplate_tdd.rs` with AC-2, AC-3, AC-4, AC-N1.
- Regenerate goldens (small shift since default config doesn't change behavior).
- Update `docs/specs/support-modules-orca-port.md` §Validation Strategy invariant list.

## Out of Scope

- The full `to_model` strategy (nodes whose `to_buildplate = false` are *kept* on the model rather than pruned). Future work; this packet's prune-on-collision logic only fires for `to_buildplate = true` nodes.
- A diagnostic for "user set `support_on_build_plate_only = true` but the planner had to reject N contacts." Future work (would emit a typed `Diagnostic` per packet 3's channel).
- Performance optimization of the per-layer projected-footprint computation.

## Authoritative Docs

- `docs/specs/support-modules-orca-port.md` §C5 — directly.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp::drop_nodes` (~line 2752) — `unsupported_branch_leaves` deque + branch pruning loop.
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp::generate_contact_points` (~line 3486) — `to_buildplate` initialization.

## Acceptance Summary

- Positive cases: AC-1 through AC-9.
- Negative case: AC-N1 (default config keeps current behavior).
- Cross-packet impact: invariant list grows.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo xtask build-guests --check` | WASM gate. | FACT pass/fail |
| `cargo test -p support-planner --test to_buildplate_tdd 2>&1 \| tee target/test-output.log` | AC-2, AC-3, AC-4, AC-N1. | FACT pass/fail; SNIPPETS ≤ 20 lines on failure |
| `cargo test -p slicer-runtime --test support_invariants_wedge_tdd 2>&1 \| tee target/test-output.log` | AC-6, AC-7, AC-9. | FACT pass/fail; SNIPPETS ≤ 30 lines on failure |
| `cargo test -p slicer-runtime --test support_golden_regression_wedge_tdd 2>&1 \| tee target/test-output.log` | AC-8. | FACT pass/fail |
| `rg -q 'support_on_build_plate_only' modules/core-modules/support-planner/support-planner.toml` | AC-5 manifest. | FACT pass/fail |
| `rg -q 'build_plate_only_emits_no_to_model_branches' docs/specs/support-modules-orca-port.md` | Doc Impact. | FACT pass/fail |

## Step Completion Expectations

- The `to_buildplate` flag MUST be propagated through the multi-neighbour aggregation in packet 7's algorithm: when a moved node has `to_buildplate = true`, its successors inherit the flag. If any contributor is `to_buildplate = false`, the successor is `to_buildplate = false` (the chain accommodates the model-anchored ancestor). This rule is encoded in Step 3 of the implementation plan.
- The integration with `clamp_to_avoidance` is: when `to_buildplate = true` AND the clamped target lies inside `collision_polys` (the snap-back failed to find a collision-free position), the node is pruned. The existing `clamp_to_avoidance` call site decides "snap or prune"; this packet adds the prune branch.

## Context Discipline Notes

- Large files in the read-only path that MUST be ranged or delegated:
  - `modules/core-modules/support-planner/src/lib.rs` — range-read contact creation + propagation (lines 280-450 area).
  - OrcaSlicer `TreeSupport.cpp` — delegate SUMMARY of `unsupported_branch_leaves` and `to_buildplate` initialization.
- Likely temptation reads (skip):
  - Orca's `support_on_build_plate_only` config path through `PrintConfig.cpp` — the semantics are documented in this packet's spec; do not derive them by reading config plumbing.
- Sub-agent return-format hints: SUMMARY ≤ 200 words for Orca; FACT pass/fail for cargo runs.
