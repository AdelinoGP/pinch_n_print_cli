# Requirements: support-planner-to-buildplate-pruning

## Packet Metadata

- Grouped task IDs:
  - `TASK-288` (renumbered from source-plan `TASK-264`; `TASK-264` is now used by `docs/07_implementation_status.md` for Lightning Layer per packet 139).
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

`support-planner` has no notion of which contacts must reach the build plate vs. which can rest on the model. OrcaSlicer's `TreeSupport` tracks `to_buildplate` per node; nodes that can't reach the build plate without a collision-free path are pruned (`unsupported_branch_leaves` in `TreeSupport.cpp:2752`). `support_on_build_plate_only` config (true ⇒ reject every `to_model` contact at creation time) is currently unhonored.

The current planner silently emits branches that pass through air or rest on impossible geometry whenever `clamp_to_avoidance` can't snap a target inside `avoidance_polys` — those nodes simply lose the propagation step but their ancestor segments are still emitted.

## In Scope

- Add `to_buildplate: bool` field to `PlannedSupportNode` (line 92 of `modules/core-modules/support-planner/src/lib.rs`). Default `true` for new contacts whose XY lies outside the object's projected footprint at the contact's layer; `false` otherwise.
- Compute "object's projected footprint at layer L" lazily once per layer using existing `LayerCollisionCache.collision_polys[L]` (per `support-planner/src/lib.rs:103`; the `LayerCollisionCache` itself is defined at the planner module level). The "outside the footprint" test is `!point_in_any_expoly(collision_polys, x, y)`.
- Prune nodes in the propagation pass: if `node.to_buildplate == true` AND the moved target is inside `collision_polys` (the existing check at lines 711-723), drop the node. The drop propagates upward in the chain (the entire chain becomes unsupported), matching Orca's `unsupported_branch_leaves` behavior. The existing "node-clamped-out" diagnostic (code 1002) is emitted as today; this packet's new condition is a STRICTLY ADDITIONAL pruning trigger for `to_buildplate = true` nodes (a tightening, not a relaxation).
- Add `support_on_build_plate_only: bool` (default `false`) to `support-planner.toml` `[config.schema]` and read it in `on_print_start` (line 114). Store on `SupportPlanner` struct (line 68-90) as a new field.
- When the config is `true`: at contact creation in `plan_for_object` (lines 380-416), reject every contact whose `to_buildplate` would be `false`. Specifically: when iterating contact points, if the contact XY lies inside the object's projected footprint AND `support_on_build_plate_only` is `true`, `continue` past the contact (do not push to `contacts_by_layer[layer_idx]`).
- Add wedge harness invariant `build_plate_only_emits_no_to_model_branches` (the 10th invariant; the 9th is the symmetry invariant from packet 122). The test forces `support_on_build_plate_only = true` on the wedge fixture's config and asserts every branch endpoint chain terminates at the build plate (`z ≤ 1e-3 mm`).
- Add `modules/core-modules/support-planner/tests/to_buildplate_tdd.rs` with AC-2, AC-3, AC-4, AC-N1, AC-N2.
- Regenerate goldens via `SUPPORT_WEDGE_REGEN_GOLDEN=1` (small shift; the default config doesn't change behavior, so the only shift is from the new prune rule for `to_buildplate = true` nodes that were already being dropped by `clamp_to_avoidance`).
- Update `docs/specs/support-modules-orca-port.md` §Validation Strategy invariant list to add `build_plate_only_emits_no_to_model_branches`.

## Out of Scope

- The full `to_model` strategy (nodes whose `to_buildplate = false` are *kept* on the model rather than pruned). Future work; this packet's prune-on-collision logic only fires for `to_buildplate = true` nodes.
- A diagnostic for "user set `support_on_build_plate_only = true` but the planner had to reject N contacts." Future work (would emit a typed `Diagnostic` per TASK-163b-diagnostic / packet 118's channel).
- Performance optimization of the per-layer projected-footprint computation. The existing `LayerCollisionCache` already provides per-layer data; no recomputation needed.
- Touching `tree-support` or `traditional-support` — they consume the planner output via the existing `support_plan_segments_for` path; the `to_buildplate` flag is internal.

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
- Negative cases: AC-N1 (default config keeps current behavior), AC-N2 (to_model node with collision is NOT pruned by the new rule).
- Cross-packet impact: invariant list grows; goldens shift minimally.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo xtask build-guests --check` | WASM gate. | FACT pass/fail |
| `cargo test -p support-planner --test to_buildplate_tdd 2>&1 \| tee target/test-output.log` | AC-2, AC-3, AC-4, AC-N1, AC-N2. | FACT pass/fail; SNIPPETS ≤ 20 lines on failure |
| `cargo test -p slicer-runtime --test support_invariants_wedge_tdd 2>&1 \| tee target/test-output.log` | AC-6, AC-7, AC-9. | FACT pass/fail; SNIPPETS ≤ 30 lines on failure |
| `SUPPORT_WEDGE_REGEN_GOLDEN=1 cargo test -p slicer-runtime --test support_golden_regression_wedge_tdd -- current_wedge_output_stays_within_self_capture_tolerance 2>&1 \| tee target/test-output.log` | AC-8 re-anchor. | FACT pass/fail; SNIPPETS ≤ 20 lines on failure |
| `cargo test -p slicer-runtime --test support_golden_regression_wedge_tdd 2>&1 \| tee target/test-output.log` | AC-8 (no-env tolerance). | FACT pass/fail |
| `rg -B1 -A4 'support_on_build_plate_only' modules/core-modules/support-planner/support-planner.toml` | AC-5 manifest. | FACT pass/fail |
| `rg -q 'build_plate_only_emits_no_to_model_branches' docs/specs/support-modules-orca-port.md` | Doc Impact. | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Lint gate. | FACT pass/fail |

## Step Completion Expectations

- The `to_buildplate` flag MUST be propagated through the multi-neighbour aggregation in TASK-287's algorithm: when a moved node has `to_buildplate = true`, its successors inherit the flag. If any contributor is `to_buildplate = false`, the successor is `to_buildplate = false` (the chain accommodates the model-anchored ancestor). This rule is encoded in Step 3 of the implementation plan. (If TASK-287 is not yet implemented, the propagation is single-neighbour and the rule is trivial: a moved node inherits its source's `to_buildplate`.)
- The integration with `clamp_to_avoidance` is: when `to_buildplate = true` AND the clamped target lies inside `collision_polys` (the snap-back failed to find a collision-free position), the node is pruned. The existing `clamp_to_avoidance` call site (line 707) decides "snap or prune"; this packet adds the prune branch as an ADDITIONAL trigger (a `to_buildplate = true` prune) without changing the existing collision-target prune for all nodes.
- The `to_buildplate` flag's interaction with the raft block (lines 442-491) is: raft contacts do NOT participate in `to_buildplate` semantics (raft is below the model; the build plate is the raft's destination, not the model's). Step 1's discovery confirms whether the raft block's contact-pushing code path also needs the `to_buildplate = false` rejection; the implementer adds it ONLY if the contact-creation path is shared between model and raft layers.

## Context Discipline Notes

- Large files in the read-only path that MUST be ranged or delegated:
  - `modules/core-modules/support-planner/src/lib.rs` — 1590 lines; range-read contact creation (lines 380-416), propagation prune (lines 711-723), `SupportPlanner` struct (lines 68-90), `on_print_start` (lines 114-191), and `LayerCollisionCache` (line 103 + the surrounding context).
  - OrcaSlicer `TreeSupport.cpp` — delegate SUMMARY of `unsupported_branch_leaves` and `to_buildplate` initialization.
- Likely temptation reads (skip):
  - Orca's `support_on_build_plate_only` config path through `PrintConfig.cpp` — the semantics are documented in this packet's spec; do not derive them by reading config plumbing.
- Sub-agent return-format hints: SUMMARY ≤ 200 words for Orca; FACT pass/fail for cargo runs.
