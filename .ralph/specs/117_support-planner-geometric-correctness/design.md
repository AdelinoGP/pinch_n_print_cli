# Design: support-planner-geometric-correctness

## Controlling Code Paths

- Primary code paths:
  - `modules/core-modules/support-planner/src/lib.rs::tapered_radius` (current line 888) — body replaced with the two-piece formula.
  - `modules/core-modules/support-planner/src/lib.rs::inflate_polygon` (current line 901) — function deleted.
  - `modules/core-modules/support-planner/src/lib.rs::run_support_geometry` (current call site around line 226, in the `LayerCollisionCache.avoidance_polys.push(inflated)` loop) — call to `inflate_polygon` replaced with `slicer_core::polygon_ops::offset`.
- Neighboring tests/fixtures:
  - `modules/core-modules/support-planner/tests/tapered_radius_tip_cone.rs` — new file (AC-1, AC-2, AC-3, AC-4, AC-N1).
  - `modules/core-modules/support-planner/tests/avoidance_offset_concave.rs` — new file (AC-6, AC-7).
  - `modules/core-modules/support-planner/tests/orca_parity_tdd.rs` — existing file; the `radius_tapers_with_distance_to_top` test (introduced by packet 31b) needs migration to match the new tip-cone behavior. The packet either re-anchors its assertion or removes the test in favor of the new file.
- OrcaSlicer comparison surface: see `requirements.md` §OrcaSlicer Reference Obligations (delegate; never load).

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- `tapered_radius` operates entirely in **mm-valued** `f32` space — both `branch_radius` and `effective_layer_height` are passed in mm. The new formula does NOT introduce any unit conversion; the upper clamp `MAX_BRANCH_RADIUS_MM = 6.0` is already mm-valued. No coordinate conversion change is required for B5.
- `slicer_core::polygon_ops::offset` takes mm-valued `delta`. The current `inflate_polygon` call site uses `avoid_inflate = branch_radius + self.tree_support_branch_distance / 2.0` (mm) — same scalar passes directly to the replacement helper. Confirm the helper's expected delta unit at line 205 before the replacement (Step 4 dispatches this).
- Interface-aware radius widening (`radius = max(radius, base_radius)` when `support_interface_top_layers > 0`) exists in OrcaSlicer's `calc_branch_radius` but is **explicitly out of scope** for this packet. Future Block C interface work owns it.

## Code Change Surface

- Selected approach: in-place rewrite of `tapered_radius`; in-place deletion of `inflate_polygon` and substitution at its single call site.
- Exact functions/structs/tests to change:
  - `tapered_radius` (fn) — body replaced; signature preserved; doc-comment updated.
  - `inflate_polygon` (fn) — deleted.
  - `run_support_geometry::plan_for_object` (or the helper that builds `LayerCollisionCache.avoidance_polys`) — call site of `inflate_polygon(&outer, avoid_inflate)` replaced with `slicer_core::polygon_ops::offset(...)`.
  - New test files (see Files in Scope).
- Rejected alternatives:
  - **Keeping `inflate_polygon` as an internal fallback with `polygon_ops::offset` as the primary** — rejected: maintaining two routines for the same operation is the duplication problem the workspace already has. Better to delete the broken one decisively.
  - **Adding the interface-aware `radius = max(radius, base_radius)` branch to `tapered_radius`** — rejected: out of scope. The tip cone is a localized math fix; interface widening is part of broader interface-band Block C work.
  - **Porting both `calc_branch_radius` overloads from Orca (mm-based AND layer-count-based)** — rejected: the planner only uses the mm-based path. Porting both grows the packet without value.

## Files in Scope (read + edit)

The packet edits 1 source file plus 2 new test files (3 total).

- `modules/core-modules/support-planner/src/lib.rs` — role: B5 + B6 implementations; expected change: `tapered_radius` body replaced, `inflate_polygon` deleted, call site updated to `polygon_ops::offset`, function doc-comments updated.
- `modules/core-modules/support-planner/tests/tapered_radius_tip_cone.rs` — role: AC-1, AC-2, AC-3, AC-4, AC-N1 test functions; expected change: file created.
- `modules/core-modules/support-planner/tests/avoidance_offset_concave.rs` — role: AC-6, AC-7 test functions; expected change: file created.

## Read-Only Context

- `crates/slicer-core/src/polygon_ops.rs` — read `pub fn offset(...)` definition at line 205 (±20 lines). Confirm: signature, delta unit, join enum type, return type. NOT the whole file.
- `docs/specs/support-modules-orca-port.md` — §B5, §B6 only. Source of the two-piece formula text.
- `modules/core-modules/support-planner/tests/orca_parity_tdd.rs` — delegate a focused look at `radius_tapers_with_distance_to_top` to plan its migration. Do NOT read the whole file.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — delegate `calc_branch_radius` SUMMARY; never load.
- `target/`, `Cargo.lock`, generated code — never load.
- `crates/slicer-runtime/**`, `crates/slicer-host/**`, `crates/slicer-scheduler/**` — out of scope.
- `modules/core-modules/support-planner/src/lib.rs` outside lines 880-940 and around line 226 (the call site) — range-read only; do not browse the rest of the file.
- Other `modules/core-modules/*` — not edited by this packet.

## Expected Sub-Agent Dispatches

- "Summarize OrcaSlicer `TreeSupport::calc_branch_radius` second overload from `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp`; return SUMMARY ≤ 200 words confirming the two-piece formula and the upper clamp, no code snippets" — purpose: confirm B5 formula matches Orca behavior we intend to port.
- "Read `crates/slicer-core/src/polygon_ops.rs` lines 195-235 only; return SNIPPETS showing `pub fn offset` full signature + first 10 lines of body" — purpose: confirm Step 4's call shape before writing it.
- "Find the test in `modules/core-modules/support-planner/tests/orca_parity_tdd.rs` that exercises `tapered_radius` directly; return SNIPPETS ≤ 20 lines with the test body" — purpose: plan its migration in Step 3.
- "Run `cargo test -p support-planner --test tapered_radius_tip_cone`; return FACT pass/fail; SNIPPETS ≤ 20 lines on failure" — purpose: gate Steps 2-3.
- "Run `cargo test -p support-planner --test avoidance_offset_concave`; return FACT pass/fail; SNIPPETS ≤ 20 lines on failure" — purpose: gate Step 5.
- "Run `cargo xtask build-guests --check`; return FACT (`up to date` or `STALE: <which>`)" — purpose: WASM staleness gate after src/lib.rs edits.

## Data and Contract Notes

- IR contracts touched: none. `SupportPlanIR.entries[*].branch_segments[*][*].width` will visibly change (the tip width becomes 0 instead of `branch_diameter`), but the IR schema and types are unchanged.
- WIT boundary considerations: none in this packet.
- Determinism: `tapered_radius` is pure; `polygon_ops::offset` is deterministic (Clipper2 guarantees deterministic output for fixed input).

## Locked Assumptions and Invariants

- `MAX_BRANCH_RADIUS_MM = 6.0` constant stays exactly as is (matches OrcaSlicer `MAX_BRANCH_RADIUS`).
- `tapered_radius` returns `0.0` at the tip (`dist_to_top = 0`). This is intentional — the tip cone IS the radius collapsing to a point. Downstream consumers that previously assumed the tip had width `branch_diameter` (e.g., the `radius_tapers_with_distance_to_top` test in `tests/orca_parity_tdd.rs`) are migrated in Step 3.
- The packet preserves the existing `tapered_radius` signature `(branch_radius: f32, tan_diameter_angle: f32, dist_to_top: u32, effective_layer_height: f32) -> f32`. No call sites are touched.
- The packet preserves the planner's `LayerCollisionCache.avoidance_polys` field shape (`Vec<Vec<[f32; 2]>>`). If `polygon_ops::offset` returns a different shape (e.g., `Polygon` or `ExPolygon`), the call site converts at the boundary; the IR is not changed.

## Risks and Tradeoffs

- **Risk**: changing `tapered_radius(0)` from `2.5` to `0.0` is a visible output change that downstream `tree-support` consumes as `Point3WithWidth.width`. If a tip point with `width = 0.0` is later interpreted as an invalid extrusion (zero-width path), the gcode emitter could panic or skip. **Mitigation**: AC-1 anchors the new behavior in a unit test. Sibling Block C work (`122_support-planner-multi-neighbour-mst`, etc.) integrates with the new tip widths via the validation harness in packet 4.
- **Risk**: `slicer_core::polygon_ops::offset` may return slightly different geometry from the prior (broken) `inflate_polygon` for inputs that happened to give sensible-looking outputs. This is the intended outcome but may cause the existing `support-planner` orca-parity goldens to drift. **Mitigation**: goldens were always self-captures (per ADR / spec D4); the migration of `radius_tapers_with_distance_to_top` is part of this packet's scope. Sibling packet 4 sets up the full regression-wedge self-capture.
- **Tradeoff**: deleting `inflate_polygon` outright means any future caller that needs vertex-offset (rare) calls `polygon_ops::offset` directly. Acceptable: the workspace already has a sanctioned helper.

## Context Cost Estimate

- Aggregate (sum across all steps): `S`
- Largest single step: `S`
- Highest-risk dispatch: OrcaSlicer `calc_branch_radius` SUMMARY — required return format SUMMARY ≤ 200 words, no code snippets. A LOCATIONS-only return is acceptable if the formula matches what the spec already documents.

## Open Questions

None.
