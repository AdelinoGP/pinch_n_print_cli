# Requirements: support-planner-geometric-correctness

## Packet Metadata

- Grouped task IDs:
  - `TASK-254` — Tip cone in `tapered_radius` (B5 from `docs/specs/support-modules-orca-port.md`)
  - `TASK-255` — `inflate_polygon` replacement with `slicer_core::polygon_ops::offset` (B6)
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `S`

## Problem Statement

`support_planner::tapered_radius` is mathematically incorrect at the branch tip. The current formula `radius = clamp(branch_radius + tan(angle) * dist * h, branch_radius, MAX)` floors at `branch_radius`, so the function never returns a value below `branch_radius`. The intended 45° tip cone — where the radius linearly tapers from 0 at the tip (dist_to_top = 0) up to `branch_radius` at depth `branch_radius` — is unreachable. Visually, every branch in the planner's output has a flat-headed-nail tip instead of an organic teardrop. Matches OrcaSlicer's `TreeSupport::calc_branch_radius` second overload, which uses the two-piece formula this packet ports.

Separately, `support_planner::inflate_polygon` is a hand-rolled vertex-offset routine using averaged adjacent-edge normals. It self-intersects on inward sharp corners, drops vertices at degenerate edges (`len_prev < 1e-8`), and silently ignores holes (the function inflates only the outer contour vertices, never traverses `expoly.holes`). Avoidance polygons computed from any non-convex or hole-bearing object outline are wrong; node-clamp decisions made downstream propagate the error.

A correctly-implemented Clipper-equivalent offset routine already exists in the workspace at `crates/slicer-core/src/polygon_ops.rs:205` as `pub fn offset(...)`. Calling it from the planner replaces the broken DIY routine with no algorithmic novelty.

This packet closes both correctness gaps in one slice — both are geometric correctness fixes localized to `support-planner/src/lib.rs`, both have self-evident unit-test oracles, and they share the same code review surface.

## In Scope

- Replace the body of `tapered_radius` in `modules/core-modules/support-planner/src/lib.rs` (current line 888) with the two-piece formula:
  ```
  let mm_to_top = (dist_to_top as f32) * effective_layer_height;
  let radius = if mm_to_top <= branch_radius {
      mm_to_top.max(0.0)         // 45° tip cone
  } else {
      branch_radius + (mm_to_top - branch_radius) * tan_diameter_angle
  };
  radius.clamp(0.0, MAX_BRANCH_RADIUS_MM)
  ```
- Update the function-level doc-comment on `tapered_radius` to describe the two-piece formula and cite OrcaSlicer's `calc_branch_radius` second overload.
- Delete the `fn inflate_polygon(...)` helper (current line 901) and remove the helper's private call site by replacing it with a call to `slicer_core::polygon_ops::offset`. The original call site is in the prepass-side avoidance build (`run_support_geometry`'s collision/avoidance cache loop; `LayerCollisionCache.avoidance_polys.push(inflated)`).
- Verify the `slicer_core::polygon_ops::offset` signature matches what the planner needs (delta in mm, miter join, miter limit ≥ 1.2 to match Orca's `offset_ex(_, jtMiter, 1.2)`, hole-bearing input/output preserved).
- Add a new test file `modules/core-modules/support-planner/tests/tapered_radius_tip_cone.rs` with the four tests in AC-1, AC-2, AC-3, AC-4 plus AC-N1.
- Add a new test file `modules/core-modules/support-planner/tests/avoidance_offset_concave.rs` with the two tests in AC-6 and AC-7.

## Out of Scope

- Changes to `smooth_nodes`, MST propagation, `to_buildplate` tracking, raft handling, paint policy — covered by sibling Block C packets.
- Refactoring `tapered_radius` callers (the function signature is preserved).
- Reorganizing or renaming `slicer_core::polygon_ops::offset` — the helper is consumed as-is.
- Removing other DIY geometric helpers from `support-planner` that aren't `inflate_polygon` (e.g., `point_in_polygon`, `closest_point_on_segment`).

## Authoritative Docs

- `docs/specs/support-modules-orca-port.md` — read §B5 (≈10 lines), §B6 (≈10 lines), §D2 (≈5 lines) directly.
- `docs/08_coordinate_system.md` — ≈30 lines; read directly. Confirms `1 unit = 100 nm` so `delta_mm` is converted correctly when passed to `polygon_ops::offset`.
- `crates/slicer-core/src/polygon_ops.rs` — read the `pub fn offset(...)` definition at line 205 (±20 lines). Confirm the function takes mm-valued `delta`, the join enum type, and the return type. Do NOT read the rest of the file.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` — `TreeSupport::calc_branch_radius` (second overload). Confirm the two-piece formula: 45° tip cone for `mm_to_top <= base_radius` (`radius = mm_to_top`), linear above (`radius = base_radius + (mm_to_top - base_radius) * diameter_angle_scale_factor`), clamp to `[MIN_BRANCH_RADIUS, MAX_BRANCH_RADIUS]`. Confirm that for `support_interface_top_layers > 0`, Orca additionally enforces `radius = max(radius, base_radius)` — but **this packet does NOT port that branch** (interface-aware radius widening is deferred; B5's scope is the tip cone only).

## Acceptance Summary

- Positive cases: `AC-1` through `AC-7` from `packet.spec.md`.
  - `AC-1` through `AC-4` + `AC-N1` are pure-math assertions on the new `tapered_radius` body; numerical tolerances are `1e-6`.
  - `AC-5` is a static grep: the DIY `inflate_polygon` function and all its call sites are gone; `polygon_ops::offset` is called instead.
  - `AC-6` and `AC-7` are geometric correctness assertions on the offset call: concave inputs produce non-self-intersecting outputs; hole-bearing inputs preserve holes.
- Negative cases: `AC-N1` from `packet.spec.md`.
- Cross-packet impact: `121_support-planner-smooth-nodes` and `122_support-planner-multi-neighbour-mst` will inherit the corrected `tapered_radius` and avoidance polygons. The validation harness packet (`119_support-validation-wedge-harness`) becomes more meaningful because invariant 5 (radius monotone non-decreasing with `dist_to_top`) now actually holds at the tip.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo check -p support-planner --all-targets` | Compile gate. | FACT pass/fail |
| `cargo clippy -p support-planner --all-targets -- -D warnings` | No lint regression after the routine deletion. | FACT pass/fail |
| `cargo test -p support-planner --test tapered_radius_tip_cone 2>&1 \| tee target/test-output.log` | Gates AC-1 through AC-4 + AC-N1. | FACT pass/fail; SNIPPETS ≤ 20 lines on failure |
| `cargo test -p support-planner --test avoidance_offset_concave 2>&1 \| tee target/test-output.log` | Gates AC-6 and AC-7. | FACT pass/fail; SNIPPETS ≤ 20 lines on failure |
| `! rg -q 'fn inflate_polygon' modules/core-modules/support-planner/src/lib.rs` | AC-5: DIY routine removed. | FACT pass/fail |
| `! rg -q 'inflate_polygon\(' modules/core-modules/support-planner/src/lib.rs` | AC-5: call sites removed. | FACT pass/fail |
| `rg -q 'slicer_core::polygon_ops::offset' modules/core-modules/support-planner/src/lib.rs` | AC-5: replacement helper called. | FACT pass/fail |
| `cargo test -p support-planner 2>&1 \| tee target/test-output.log` | Full planner suite green; the existing `radius_tapers_with_distance_to_top` test in `tests/orca_parity_tdd.rs` MUST be updated or removed since its expected behavior assumed the floor-at-`branch_radius` semantics. | FACT pass/fail; SNIPPETS ≤ 30 lines on failure |
| `cargo xtask build-guests --check` | Guest WASM up to date after src/lib.rs edit. | FACT pass/fail |

## Step Completion Expectations

- The existing test `radius_tapers_with_distance_to_top` in `modules/core-modules/support-planner/tests/orca_parity_tdd.rs` (introduced by packet 31b) embeds the OLD broken behavior: it asserts the topmost width equals `branch_diameter = 5.0` because `tapered_radius(0)` returned `branch_radius = 2.5`. The new tip cone returns `0.0` at `dist_to_top = 0`. Step 3 of the implementation plan owns the migration of this test — either re-anchor its assertion to the new tip cone (preferred; matches Orca more closely) or remove the test and re-state the intent in the new `tapered_radius_tip_cone` test file. The implementer MUST NOT leave the old test in conflict with the new behavior.
- The `polygon_ops::offset` call replaces the planner's prior `inflate_polygon` call site only — it does NOT add new call sites elsewhere in the planner. Step 4 of the implementation plan asserts the call count is exactly 1 in `support-planner/src/lib.rs`.

## Context Discipline Notes

- Large files in the read-only path that MUST be ranged or delegated:
  - `crates/slicer-core/src/polygon_ops.rs` — read only `pub fn offset(...)` signature at line 205 (±20 lines). Do NOT read the whole file (320+ lines).
  - `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` — delegate `calc_branch_radius` SUMMARY (≤ 200 words). Never load the 3,834-LOC file.
  - `modules/core-modules/support-planner/src/lib.rs` — 1,000+ lines; range-read around the two edit sites (lines 880-940 covers both).
- Likely temptation reads (skip these):
  - Other DIY geometric helpers in `support-planner/src/lib.rs` (e.g., `closest_point_on_segment`, `point_in_polygon`) — out of scope; do not open or evaluate them in this packet.
  - `modules/core-modules/support-planner/tests/orca_parity_tdd.rs` — the existing tests use the old behavior; the implementer needs to know the ONE test that requires migration, but should not browse the file end-to-end. Delegate: "Find the test in `tests/orca_parity_tdd.rs` that calls `tapered_radius` directly; return SNIPPETS ≤ 20 lines."
- Sub-agent return-format hints for heaviest dispatches:
  - `cargo test -p support-planner` (full suite) — FACT pass/fail on first pass; on failure SNIPPETS ≤ 30 lines with the failing test names + assertion text. Do NOT paste the full cargo output.
  - "Summarize OrcaSlicer `calc_branch_radius` second overload" — SUMMARY ≤ 200 words; no code unless explicitly requested.
