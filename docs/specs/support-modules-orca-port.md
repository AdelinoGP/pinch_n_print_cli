# Support Modules — OrcaSlicer Parity Plan

## Context

The `tree-support`, `traditional-support`, and `support-planner` core modules
were stood up across packets 28, 30, 31a, and 31b as scaffolding ports of
OrcaSlicer's tree-support pipeline (`TreeSupport.cpp`, `SupportMaterial.cpp`,
`SupportCommon.cpp` in `OrcaSlicerDocumented/src/libslic3r/Support/`). The
scaffolding shipped the algorithmic *shape* of OrcaSlicer's
`TreeSupport::detect_overhangs` + `TreeSupport::drop_nodes` but not its
numerical output. Packet 31b documented this explicitly:

> *"The goldens are deterministic Pinch 'n Print self-captures; the test serves
> as a regression anchor against drift in `support-planner`'s own output, not
> as an external OrcaSlicer parity check. External OrcaSlicer numerical
> parity is not in scope of this packet."* — `.ralph/specs/_OLD/31b_support-planner-algorithmic-parity/packet.spec.md`

OrcaSlicer numerical parity was not deliverable then because the codebase did
not yet have the infrastructure (paint kernel parity, region-splitting model,
extracted helpers) it required. `TASK-163b` remains open in
`docs/07_implementation_status.md` against this gap.

This spec plans the next iteration of support work. It is sequenced after
packet 95 lands, because
P95 deletes `PaintRegionIR` — the IR all three support modules currently
read from. The plan splits into three remediation buckets keyed by
relationship to P95, and two execution blocks (B-now, C-plan) keyed by what
the architecture supports today.

A sibling spec, `raft-default-module.md`, owns the raft-rendering portion of
this work. The seam is `SupportPlanIR.raft_plan` — defined here, consumed
there.

## Authoritative References

- **OrcaSlicer reference sources**:
  - `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` (3,834 LOC) — `TreeSupport::generate`, `detect_overhangs`, `generate_contact_points`, `drop_nodes`, `smooth_nodes`, `draw_circles`, `generate_toolpaths`.
  - `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.hpp` — `SupportNode`, `TreeSupportData`, `LayerHeightData`, `TreeNodeType`.
  - `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp` + `.hpp` — `PrintObjectSupportMaterial::generate` (11-stage pipeline) for traditional-support reference.
  - `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp` — `generate_raft_base` (referenced by `raft-default-module.md`).
- **Project architecture**:
  - `docs/01_system_architecture.md:123-148` — `PrePass::SupportGeometry` stage contract.
  - `docs/01_system_architecture.md:231-247` — `Layer::Support` stage contract.
  - `docs/02_ir_schemas.md:845-921` — `SupportIR`, `SupportPlanIR`.
  - `CONTEXT.md` — claim, blackboard, active region, paint semantic vocabulary.
- **Paint pipeline dependency**:
  - `docs/specs/paint-pipeline-orca-parity-roadmap.md` — D8 (delete `PaintRegionIR`), D14 (modifier-volume support → `segment_annotations`).
  - the packet-95 spec — P95 scope and deletion sweep.
- **Predecessor packets (closed; in `.ralph/specs/_OLD/`)**:
  - `28_tree-support-multi-layer-propagation`, `30_support-planner-prepass-wit-plumbing`, `31a_support-geometry-prepass-and-layer-height`, `31b_support-planner-algorithmic-parity`.

## Audit Findings

Three buckets keyed by relationship to P95 and to the post-paint architecture.

### Bucket A — Made moot by P95

The deletion of `PaintRegionIR` (P95 sub-step 16) removes the call sites these
findings target. P95 itself does not migrate the support modules — its
deletion sweep is scoped to `crates/` and the support modules live under
`modules/core-modules/`. P95 must stub the call sites to keep the workspace
compiling; the proper migration is the first item of Block C.

| Finding | Disposition |
|---|---|
| `support_paint_policy` in `tree-support` and `traditional-support` reads `PaintRegionIR` via `point_in_paint_region(...)` at the polygon centroid. Geometrically wrong for non-convex or hole-bearing ExPolygons (centroid may lie in a hole). | Replaced wholesale by Block C migration to `SlicedRegion.segment_annotations[SupportEnforcer/Blocker]`. |
| Both modules duplicate `support_paint_policy` byte-for-byte. | Replaced by shared helper in Block C migration. |
| `support-planner::collect_paint_enforcer_contacts` / `collect_paint_blocker_polygons` source from `MeshObjectView.paint_layers.facet_values`, which is fed by the broken paint kernel (`paint_segmentation.rs:298-362` XY-shadow bug). | Source IR replaced by P95 paint kernel correction; planner reads via the new IR shape in Block C. |
| `tree-support`, `traditional-support`, `support-planner` manifests all declare `PaintRegionIR` in `[ir-access].reads`. | Manifest updates land with the Block C migration; P95 stubs them to drop the dead read. |

### Bucket B — Surgical, lands now

Independent of paint pipeline. Each item is a self-contained correctness or
honesty fix with an obvious local oracle. These constitute **Block B**
(below).

| Finding | Module |
|---|---|
| Doc-comment overpromises Orca parity. `support-planner` declares "Port of OrcaSlicer's `TreeSupport::detect_overhangs` + `TreeSupport::drop_nodes`" — the implementation is algorithmic shape, not parity. `tree-support` doc-comment describes a "tree-style branching support generator" but the fallback is a per-layer 2-D grid MST. | all three |
| `tapered_radius` has no tip cone. Formula `clamp(branch_radius + tan(angle) * dist * h, branch_radius, MAX)` floors at `branch_radius` and never produces a tapered tip. Orca's `calc_branch_radius` second overload uses `radius = mm_to_top` while `mm_to_top <= base_radius` (45° cone), then linear above. | `support-planner` |
| `inflate_polygon` is a DIY vertex-offset that self-intersects on sharp inward corners and silently produces wrong avoidance for non-convex outlines or any hole-bearing polygon. `slicer_core::polygon_ops::offset` already exists at `polygon_ops.rs:185` with Clipper-equivalent behavior. | `support-planner` |
| `support_interface_bottom_layers` is read from config (line 156), stored on the struct (line 178), never used. Dead state. | `support-planner` |
| `BASE_SPEED = 50.0` is hardcoded in tree-support, traditional-support, and rectilinear-infill as the normalization base for `speed_factor`. The convention is undocumented; future readers wonder why 50. | three modules + rectilinear-infill (note only) |
| `max_branches_per_layer = 1024` hard cap silently truncates contacts on dense overhangs (`support-planner/src/lib.rs:326`, `:341`, `:434`). Drops are silent — no diagnostic. | `support-planner` |
| `support-planner.node-clamped-out` warning uses `log(LogLevel::Warn, &format!("..."))` with a string-prefixed payload. `TASK-163b` records this as needing promotion to a typed `Diagnostic` channel via the prepass output WIT. | `support-planner` + `world-prepass.wit` |

### Bucket C — Deferred to post-P95 architecture

These either depend on the post-paint IR shape, require an oracle that v1
invariant tests do not provide, or carry an architectural decision that
warrants its own design space. These constitute **Block C** (below).

| Finding | Why deferred |
|---|---|
| `support_paint_policy` migration to `SlicedRegion.segment_annotations`. | Hard prerequisite on P95 landing; first item of Block C. |
| `support-planner` is missing `smooth_nodes` (Orca runs 100-iter Laplacian smoothing on each branch chain — `TreeSupport.cpp:3153`). Output branches are jagged stairsteps. | Algorithmic change with no local oracle; needs the validation infrastructure that Block C sets up. |
| Single-neighbour propagation in `support-planner`. Orca's `drop_nodes` uses multi-neighbour MST adjacency to synthesize move targets; the current planner picks the lowest-distance neighbour and moves toward it. Produces asymmetric branches for nodes with ≥3 MST neighbours. | Changes branch connectivity — algorithmic, needs the validation harness. |
| `support-planner` has no `to_buildplate` notion, no unsupported-branch pruning. Orca tracks `to_buildplate` per node and prunes branches that can't reach the build plate (`TreeSupport.cpp:2752`). `support_on_build_plate_only` config is unhonored. | Conceptual extension beyond v1 surface; depends on validation. |
| Raft geometry is not emitted by the current support planner. Packet 119 emits the optional configuration-only `SupportPlanIR.raft_plan` seam; no raft polygons or raft-layer geometry cross that seam. | Owned by the sibling `raft-default-module.md` spec. Packet 124 owns geometry generation and downstream rendering. |
| `support-planner` uses `f32` geometry throughout; Orca uses scaled `coord_t` (i64 at 1e-6 mm). | Large change, low immediate benefit, no failing test to chase. Documented as future work. |
| `traditional-support` either consumes `SupportPlanIR` (post-P95, with `segment_annotations`-driven enforcer/blocker) or stays explicitly per-layer. Design decision. | Depends on post-paint IR shape and on a deliberate decision about whether the rectilinear scan-line filler is planner-aware. |

## Design Decisions

| # | Area | Decision |
|---|---|---|
| D1 | Spec scope vs. P95 | All work in this plan happens after P95 lands. P95 keeps the modules compiling via stubs (`support_paint_policy` returns `DefaultEligible`; `PaintRegionIR` reads dropped from manifests). |
| D2 | Bucket B cutline | Items in Bucket B that have an obvious local oracle (tip cone, `inflate_polygon`, dead-field cleanup, doc honesty, diagnostic channel) ship as Block B. Items that change algorithm or connectivity defer to Block C. |
| D3 | Validation strategy | Block C lands behind the union of: (a) six invariant tests on `regression_wedge.stl` (see §Validation Strategy); (b) self-capture golden regression on `regression_wedge.stl` with branch-count ±10% and endpoint Hausdorff ≤ 0.5mm tolerances. Either failure fails CI. |
| D4 | Orca-reference oracle | `TASK-163b-orca-ref` (real OrcaSlicer reference output) remains deferred. Blocked on fixture + Orca-runner infrastructure that does not exist. Listed in §Open Follow-ups. |
| D5 | Raft as separate concern | Raft rendering does NOT live in `support-planner`. `SupportPlanIR.raft_plan` is the seam; raft synthesis and rendering are owned by `raft-default-module.md`. |
| D6 | Raft rendering pattern | Raft fill is rendered by whichever `Layer::Infill` module(s) declare `claim:raft-fill`. A new `ExtrusionRole::RaftInfill` variant + `claim:raft-fill` mapping extend the existing per-role-per-claim dispatch pattern. No pattern duplication, no shared library. See ADR-0009. |
| D7 | Pattern-library extraction | Rejected. Breaks the multi-language module promise (a C++ TPMS-Infill module cannot import a Rust library). Existing duplication between `rectilinear-infill` and `traditional-support` is acknowledged as out of scope and not addressed by this plan. |
| D8 | `support_interface_bottom_layers` | Block B deletes the dead struct field and parse. Block B emits `not_implemented` log if the user sets the key to anything other than `-1` (default). Real implementation deferred to a future packet block. |
| D9 | `BASE_SPEED` normalization | `BASE_SPEED = 50.0` documented as the convention (normalization base for `speed_factor`; downstream consumers multiply through). Not changed. |
| D10 | 1024-contact cap | Cap retained for runtime bounding. Block B adds `log(LogLevel::Warn, ...)` with structured fields `{layer, object_id, dropped_count, kept_count}` once per layer when triggered. |
| D11 | Typed Diagnostic channel | Block B adds `record diagnostic { severity: severity-level, code: u32, layer: option<s32>, object-id: option<string>, message: string }` to `world-prepass.wit`. See ADR-0010. |
| D12 | TASK-163b split | Closes `TASK-163b` as written; opens `TASK-163b-diagnostic` (in Block B) and `TASK-163b-orca-ref` (deferred). |

## Reusable Building Blocks (already in the workspace)

| Need | Use | Path |
|---|---|---|
| Polygon offset (Clipper-backed) | `polygon_ops::offset(polys, delta_mm, join, arc_tol)` | `crates/slicer-core/src/polygon_ops.rs:185` |
| Polygon union/intersection/difference | `polygon_ops::{union, intersection, difference}` | `crates/slicer-core/src/polygon_ops.rs:93-108` |
| Polygon simplicity validation | `validate_polygon_simplicity` | `crates/slicer-core/src/polygon_ops.rs:131` |
| ExPolygon-aware helpers post-P95 | `union_ex`, `intersection_ex`, `difference_ex`, `opening`, `closing_ex` | Added by P95 sub-step 0 (the packet-95 spec AC-1) |
| Per-role-per-claim dispatch | `SliceRegionView::should_emit(role)` + `held_claims` | `crates/slicer-sdk/src/views.rs:330-359` |
| Per-region fill area carriers | `SliceRegionView::{infill_areas, top_solid_fill, bottom_solid_fill, bridge_areas}` | `crates/slicer-sdk/src/views.rs:228-302` |
| Coordinate conversion | `Point2::from_mm`, `mm_to_units`, `units_to_mm` | `crates/slicer-ir/src/slice_ir.rs` |
| Test fixture | `resources/regression_wedge.stl` — 45° overhang, deliberate bridge, top + bottom surfaces | per P0b in `paint-pipeline-orca-parity-roadmap.md` |

## Block B — Surgical fixes (post-P95)

Independent of paint architecture. Each item has a local oracle; no `regression_wedge.stl` invariant infrastructure required. Lands after P95 closes.

### B1 — Doc-comment honesty across the three modules

**Goal**: rewrite the lead doc-comments to describe what the code actually does, not what an earlier port aspiration claimed.

**Scope**:
- `modules/core-modules/tree-support/src/lib.rs:1-12`: replace "tree-style branching support generator" with "Per-layer 2-D grid-MST infill with optional `SupportPlanIR` consumption. Not a port of OrcaSlicer's `TreeSupport`; pre-planned branch geometry from `support-planner` is emitted directly when present, and the grid-MST filler is the per-layer fallback."
- `modules/core-modules/traditional-support/src/lib.rs:1-16`: replace with "Per-layer rectilinear scan-line filler for `Layer::Support`. Eligibility comes entirely from upstream `SurfaceClassificationIR.needs_support` plus the paint precedence rules in `docs/01`. This module does not detect overhangs, allocate intermediate layers, or generate raft / interface / contact layers — those are out of scope and not provided in v1."
- `modules/core-modules/support-planner/src/lib.rs:1-35`: replace "Port of OrcaSlicer's `TreeSupport::detect_overhangs` + `TreeSupport::drop_nodes`" with "Multi-layer support planner inspired by OrcaSlicer's `TreeSupport::drop_nodes`. Implements the algorithmic shape (detect → contact → top-down MST propagation → emit) but not numerical parity. Block C of `docs/specs/support-modules-orca-port.md` is the path toward correctness on engineered invariants."

**Verification**:
```bash
cargo build -p tree-support -p traditional-support -p support-planner
cargo test -p tree-support -p traditional-support -p support-planner 2>&1 | tee target/test-output.log
```

### B2 — `support_interface_bottom_layers` cleanup

**Goal**: remove dead state; preserve the user-facing config key with a `not_implemented` signal.

**Scope**:
- Delete the field from `SupportPlanner` struct (`support-planner/src/lib.rs:75`).
- Delete the parse block (`:156-160`).
- In `on_print_start`, when `config.get("support_interface_bottom_layers")` is `Some(v)` and `v != Int(-1)`, emit `log(LogLevel::Warn, "support-planner: support_interface_bottom_layers is not yet implemented; set to -1 (default) to suppress this warning")`. Once per `on_print_start`, not per layer.
- Keep the config key in `support-planner.toml [config.schema]` so the user-facing surface is unchanged; add `# Not yet implemented — see docs/specs/support-modules-orca-port.md` comment in the toml.

**Verification**:
```bash
cargo test -p support-planner 2>&1 | tee target/test-output.log
# Negative test: set support_interface_bottom_layers = 3; assert the warning was logged exactly once.
```

### B3 — `BASE_SPEED` documented as convention

**Goal**: document the normalization convention so future readers don't re-derive it.

**Scope**:
- Add a `/// # Speed normalization` section to each module's lead doc-comment block (the three support modules and `rectilinear-infill` for symmetry) explaining: "`speed_factor = configured_speed / BASE_SPEED`. `BASE_SPEED = 50.0` is the project-wide normalization base; downstream gcode-emit multiplies `speed_factor` through to the feed rate. The base value is shared across all per-role speed modules and is not configurable in v1."
- No code change.

**Verification**: doc-only; `cargo doc --no-deps -p support-planner -p tree-support -p traditional-support` succeeds.

### B4 — 1024-contact silent-truncation diagnostic

**Goal**: turn the silent data loss into a logged signal so users notice when the cap fires.

**Scope**:
- At `support-planner/src/lib.rs:326`, `:341`, and `:434`, replace the `continue` / `truncate` with an accumulating per-layer counter.
- At the end of each layer's contact-collection pass, if the counter is non-zero, emit `log(LogLevel::Warn, format!("support-planner: max_branches_per_layer cap exceeded at layer {layer_index}, object {object_id}: dropped {dropped_count}, kept {kept_count}"))`. Once per layer per object, not per drop.

**Verification**:
```bash
cargo test -p support-planner 2>&1 | tee target/test-output.log
# Positive test: synthesize an overhang fixture that produces >1024 contacts at one layer;
# assert the warning is logged exactly once for that layer.
```

### B5 — Tip cone in `tapered_radius`

**Goal**: restore the 45° tip cone that Orca's `calc_branch_radius` produces and that the current formula floors out.

**Scope**:
- `support-planner/src/lib.rs:878-887` `tapered_radius`. Replace:
  ```rust
  let expanded = branch_radius + tan_diameter_angle * (dist_to_top as f32) * effective_layer_height;
  expanded.clamp(branch_radius, MAX_BRANCH_RADIUS_MM)
  ```
  With:
  ```rust
  let mm_to_top = (dist_to_top as f32) * effective_layer_height;
  let radius = if mm_to_top <= branch_radius {
      // 45° tip cone: linear from 0 at the tip to branch_radius at depth = branch_radius
      mm_to_top.max(0.0)
  } else {
      // Linear widening above the tip
      branch_radius + (mm_to_top - branch_radius) * tan_diameter_angle
  };
  radius.clamp(0.0, MAX_BRANCH_RADIUS_MM)
  ```
- Update the doc-comment to describe the two-piece formula.

**Verification**:
```bash
cargo test -p support-planner tapered_radius 2>&1 | tee target/test-output.log
```
Unit tests asserting:
- `tapered_radius(branch_radius=2.5, dist_to_top=0, h=0.2)` returns 0.0 (tip).
- `tapered_radius(branch_radius=2.5, dist_to_top=10, h=0.2)` returns `min(2.0, 2.5)` = 2.0 (still on cone).
- `tapered_radius(branch_radius=2.5, dist_to_top=15, h=0.2)` returns 2.5 (boundary).
- `tapered_radius(branch_radius=2.5, dist_to_top=50, h=0.2)` returns >2.5 (above cone).

### B6 — `inflate_polygon` replacement

**Goal**: replace the DIY vertex-offset (geometrically wrong on non-convex outlines and silently broken on holes) with the Clipper-backed `polygon_ops::offset`.

**Scope**:
- Delete `support-planner/src/lib.rs:889-936` `inflate_polygon`.
- Replace call sites (`:226`) to use `slicer_core::polygon_ops::offset(polys, delta_mm, JoinType::Miter, arc_tol_mm)`. Confirm `JoinType` and `arc_tol_mm` defaults match what the planner needs (miter limit ≥ 1.2 to match Orca's `offset_ex(.., jtMiter, 1.2)` calls; arc_tol negligible for miter joins).
- Confirm the helper handles hole-bearing ExPolygons correctly (Clipper2's `offset_ex` does; verify the `slicer-core` wrapper preserves that behavior).

**Verification**:
```bash
cargo test -p support-planner -p slicer-core polygon_ops 2>&1 | tee target/test-output.log
```
New unit tests:
- L-shaped concave polygon: post-offset polygon is also L-shaped (no self-intersection at the concave corner).
- Polygon with one hole: post-offset polygon retains the hole at the proportionally-eroded shape.

### B7 — Typed Diagnostic channel (`TASK-163b-diagnostic`)

**Goal**: replace string-prefixed log calls with a typed channel that downstream tooling can read programmatically.

**Scope**:
- `crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit`: add
  ```wit
  record diagnostic {
      severity: severity-level,
      code: u32,
      layer: option<s32>,
      object-id: option<string>,
      message: string,
  }

  enum severity-level {
      trace,
      debug,
      info,
      warn,
      error,
  }
  ```
  Add a `push-diagnostic: func(d: diagnostic)` host import (or output-builder method) to the prepass world. Run `cargo xtask build-guests` per the WIT/Type Changes Checklist in `CLAUDE.md`.
- `slicer-sdk` exposes a `Diagnostics` builder; planner calls `diagnostics.push(Diagnostic { ... })` instead of `log(LogLevel::Warn, format!(...))`.
- Migrate the three existing string-prefixed log calls in `support-planner` (node-clamped-out at `:633`, B4's max-branches-cap, B2's not-implemented-bottom-layers).

**Verification**:
```bash
cargo xtask build-guests --check
cargo test -p slicer-runtime -p support-planner diagnostic 2>&1 | tee target/test-output.log
```
- WIT bindgen succeeds across all 20 guests.
- Round-trip test: planner emits a `Diagnostic` via WIT; host receives it with all fields preserved.
- Existing `host-services.log` string-prefix path is removed for the three call sites; no `support-planner.node-clamped-out:` string survives `rg`.

## Block C — Algorithmic depth and post-paint migration (post-P95)

Sequenced after Block B. Lands behind the §Validation Strategy harness — no
algorithmic change ships without invariants and self-capture in place.

### C1 — Validation harness on `regression_wedge.stl` (implemented)

**Goal**: provide the current-contract invariant test set and self-capture
golden regression. Packet 119 implements this harness; it is the local oracle
for later Block C work, not an external OrcaSlicer parity check.

**Scope**:
- New test file `crates/slicer-runtime/tests/integration/support_invariants_wedge_tdd.rs`:
  current observable support invariants, including the origin-contact exception
  in AC-2 and the public `dist_to_top_mm` and raft-plan checks.
- New self-capture golden file `resources/golden/support_regression_wedge_branch_count.txt` (single integer).
- New self-capture golden file `resources/golden/support_regression_wedge_endpoints.txt` (sorted list of `(x, y, z)` triples).
- Both goldens are generated from the current planner output after the
  prerequisite support fixes; they are regression anchors, not OrcaSlicer
  reference output.
- New test file `crates/slicer-runtime/tests/integration/support_golden_regression_wedge_tdd.rs`:
  asserts branch count within 10% of the captured baseline and symmetric
  endpoint Hausdorff distance at most 0.5 mm. Either failure fails the test.

AC-2 permits a finite endpoint with `dist_to_top_mm` within `1e-6` mm of zero
to remain on or inside the model outline when it is the raw origin-contact tip
required to reach an overhang centroid. Every finite propagated endpoint with
`dist_to_top_mm > 0.0` remains subject to the outside-collision predicate, and
the test requires at least one propagated endpoint check.

**Verification**:
```bash
cargo test -p slicer-runtime --all-targets --test integration -- support_invariants_wedge_tdd 2>&1 | tee target/test-output.log
cargo test -p slicer-runtime --all-targets --test integration -- support_golden_regression_wedge_tdd 2>&1 | tee target/test-output.log
```

### C2 — Migrate paint enforcer/blocker eligibility to `SlicedRegion.segment_annotations`

**Goal**: restore paint-driven support gating against the post-P95 IR shape. This is the first algorithmic Block C item because P95 stubs `support_paint_policy` to `DefaultEligible` — enforcer/blocker behavior is disabled until C2 lands.

**Scope**:
- `tree-support`, `traditional-support`: rewrite `support_paint_policy` to read from `SlicedRegion.segment_annotations` per D14 of the paint roadmap. Move the common helper to `slicer-core::paint_policy` (or similar) so the two modules share one implementation.
- `support-planner`: rewrite `collect_paint_enforcer_contacts` / `collect_paint_blocker_polygons` to source contacts from the new IR shape (per the paint kernel's per-facet output, now Z-correct post-P95).
- Update the three module manifests' `[ir-access].reads` to declare the new IR dependency.
- Geometric correctness: replace centroid-in-polygon with polygon-intersection-with-paint-region. An L-shaped overhang whose centroid lies in a hole or notch must still be enforced if any part of it overlaps the enforcer region.

**Verification**:
```bash
cargo test -p tree-support -p traditional-support -p support-planner -p slicer-runtime paint_policy_segment_annotations 2>&1 | tee target/test-output.log
```
- Invariant 4 (every overhang facet ⇒ contact point at origin layer) continues to hold against the new path.
- Self-capture golden regression continues to hold against the new path.
- New tests:
  - L-shaped enforcer region over a flat overhang: support is generated for the full L, not just the centroid cell.
  - Enforcer region covering only one half of a region's polygon: support is generated for that half, not for the half that has no enforcement and a `needs_support=false` classification.

### C3 — `smooth_nodes` port

**Goal**: replace stairstep branches with smoothed ones. Mirrors OrcaSlicer `TreeSupport.cpp:3153` (100-iteration Laplacian smoothing on each branch chain).

**Scope**:
- New function `support-planner::smooth_chains(nodes: &mut [PlannedSupportNode], iterations: usize)`.
- For each branch chain (root-to-tip), 100 iterations of three-point Laplacian smoothing: `p[i] = (p[i-1] + p[i] + p[i+1]) / 3`.
- Radii smoothed the same way: `r[i] = (r[i-1] + r[i] + r[i+1]) / 3`.
- Endpoints (root and tip) held fixed.
- Run as a final pass after the top-down MST propagation completes, before emitting `SupportPlanEntry.branch_segments`.

**Verification**:
- Invariants 1-5 continue to hold.
- New invariant: branch curvature ≤ a documented threshold per segment-pair. Added to the v1 invariant list.
- Self-capture golden regression: branch endpoints shift (expected); golden is regenerated and the diff is reviewed for "smoother, not warped" before committing.

### C4 — Multi-neighbour MST propagation

**Goal**: replace single-neighbour move target with multi-neighbour move target per Orca's `drop_nodes` (`TreeSupport.cpp:2625`).

**Scope**:
- `support-planner/src/lib.rs:586-660` propagation block.
- For each surviving node, walk all MST neighbours (not just the lowest-distance one) and synthesize a move target from their centroid (weighted by reciprocal distance, optionally — confirm the Orca formula).
- Update the propagation to clamp against `avoidance_polys` after target synthesis.

**Verification**:
- Invariants 1-5 continue to hold.
- New invariant: branches with ≥3 incoming MST edges produce symmetric merge geometry. Added to the v1 invariant list.
- Self-capture golden regression: branch connectivity shifts (expected); golden is regenerated and the diff is reviewed before committing.

### C5 — `to_buildplate` tracking and unsupported-branch pruning

**Goal**: honor `support_on_build_plate_only` and prune branches that can't reach the build plate.

**Scope**:
- Add `to_buildplate: bool` to `PlannedSupportNode`.
- At contact-point creation, set `to_buildplate = true` if the contact's XY lies outside the model's projected footprint at that layer (Orca's heuristic in `generate_contact_points`).
- During propagation, when a node's move target lies inside `collision_polys` AND `to_buildplate` is true, prune the node and propagate the prune upward through the chain.
- Honor `support_on_build_plate_only` config: if true, every contact must have `to_buildplate = true` or it is rejected.

**Verification**:
- Invariant 1 (every branch reaches build plate or a contact point) becomes strictly checkable on the build-plate-only path.
- New test: object suspended above the build plate with `support_on_build_plate_only = true`; no branches are emitted that would rest on the model.

### C6 — `SupportPlanIR.raft_plan` seam (implemented)

**Goal**: define the seam with `raft-default-module.md` without claiming that
the support planner owns raft geometry.

**Current contract**:
- `SupportPlanIR` is schema version 1.2.0 and carries
  `raft_plan: Option<RaftPlan>`.
- `RaftPlan` is a configuration-only record with
  `raft_layers: u32`, `raft_first_layer_density: f32`,
  `base_raft_layers: u32`, and `interface_raft_layers: u32`.
- `support-planner` emits one `RaftPlan` through the support-geometry WIT
  `push-raft-plan` seam when `support_raft_layers > 0`. A zero value leaves the
  option as `None`; the output builder rejects a second plan in one invocation.
- The three additive configuration keys mirrored into the record are
  `raft_first_layer_density`, `base_raft_layers`, and
  `interface_raft_layers`; `support_raft_layers` controls whether the option is
  present.
- No footprint, layer specification, Z gap, or raft polygon is carried in
  `RaftPlan`. Raft geometry and rendering are deferred to packet 124 and the
  sibling `raft-default-module.md` spec.

**Verification**:
```bash
cargo test -p slicer-runtime --all-targets --test integration -- support_invariants_wedge_tdd::enabled_raft_config_is_emitted_as_raft_plan 2>&1 | tee target/test-output.log
cargo test -p slicer-runtime --all-targets --test integration -- support_invariants_wedge_tdd::disabled_raft_config_has_no_raft_plan 2>&1 | tee target/test-output.log
```
- The enabled test checks the exact configuration values `2`, `0.4`, `1`,
  and `1`.
- The disabled test checks `raft_plan.is_none()` for
  `support_raft_layers = 0`.

### C7 — `traditional-support` ↔ `SupportPlanIR` contract

**Goal**: decide and document whether `traditional-support` consumes `SupportPlanIR` post-P95.

**Scope**: This is a design decision the spec needs to resolve before C7 implementation. Proposal: `traditional-support` does NOT consume `SupportPlanIR` and remains a per-layer scan-line filler. Rationale: rectilinear support is fundamentally per-layer (no cross-layer state); the planner's value (organic top-down branch propagation) does not apply. The decision is documented in the module's doc-comment (B1) and in `docs/01_system_architecture.md`'s `Layer::Support` description.

**Verification**: doc-only; no code change. `traditional-support.toml [ir-access].reads` does not include `SupportPlanIR`.

## Validation Strategy

Packet 119's C1 harness establishes the current local checks. Later Block C
algorithmic work must preserve these checks unless it adds and documents a
deliberate new invariant. The harness is self-captured and does not establish
numerical OrcaSlicer parity.

### Invariants v1

The current invariant tests are in
`crates/slicer-runtime/tests/integration/support_invariants_wedge_tdd.rs` and
run against `resources/regression_wedge.stl` with default config and
`support_enabled = true`.

1. **Finite branch paths.** `SupportPlanIR.entries` is non-empty for enabled
   support, every branch has at least two points, and every point's coordinate
   and width fields are finite.
2. **Collision endpoint handling.** Every endpoint is either a finite origin
   contact with `dist_to_top_mm` within `1e-6` mm of zero, or a finite positive
   propagated point outside its layer's outer contour after holes are excluded.
   Origin-contact tips are exempt because their raw centroids must reach the
   overhang and may lie on or inside the model outline. The test requires at
   least one propagated endpoint check and does not silently skip missing
   geometry.
3. **Layer Z.** Every non-raft branch point has the `z` value of its
   `LayerPlanIR` layer within `1e-4` mm.
4. **Overhang coverage.** Every qualifying non-base downward wedge facet has
   an emitted endpoint within `tree_support_branch_distance` mm of its
   centroid on the first layer at or above the centroid Z.
5. **Radius bounds.** Every emitted radius (`width / 2`) is finite,
   non-negative, and no greater than `MAX_BRANCH_RADIUS_MM = 6.0`.
6. **Disabled raft prefix.** With `support_raft_layers = 0`, no support-plan
   entry has a negative `global_layer_index`.

The list is documented as "v1, expected to grow." Specifically, C3 adds a
curvature invariant; C4 adds a multi-neighbour-symmetry invariant; C5 adds a
build-plate-only invariant. Each C-item that introduces a new invariant
documents it before merging.

Packet 119 also verifies the additive public seams:

7. **Self-capture stability.** Branch count remains within 10% of the
   committed wedge baseline and symmetric endpoint Hausdorff distance remains
   at most 0.5 mm.
8. **Per-point support distance.** `Point3WithWidth.dist_to_top_mm` is finite
   and non-negative for every emitted point, and at least one positive value is
   observed. The current test does not claim a parent-chain ordering that is
   not represented in the public IR.
9. **Optional raft configuration.** Enabled raft settings produce one
   `RaftPlan` with the exact configured values; `support_raft_layers = 0`
   produces `None`. Geometry is deferred to packet 124.

### Self-capture golden regression

Captured on `regression_wedge.stl` with default config + `support_enabled = true`
after the prerequisite support fixes and before later Block C algorithmic
changes. These are Pinch 'n Print self-captures, not OrcaSlicer reference data.

Two artifacts:
- `resources/golden/support_regression_wedge_branch_count.txt` — single integer, the total `SupportPlanEntry.branch_segments` count across all entries.
- `resources/golden/support_regression_wedge_endpoints.txt` — sorted list of `(x, y, z)` triples for every branch endpoint, formatted as one triple per line.

Tolerance:
- Branch count within 10% of the captured baseline.
- Endpoint Hausdorff distance at most 0.5 mm against the captured baseline.

Failure on either fails CI. Each C-item that intentionally changes branch
output (C3 smooth_nodes, C4 multi-neighbour) re-captures the golden with a
commit message explaining the diff; reviewers verify the new shape is
"intended different, not regression."

## Cross-spec dependencies

`raft-default-module.md` is the sibling spec that owns raft rendering. The
seam:

- **This spec defines** the current `SupportPlanIR.raft_plan: Option<RaftPlan>`
  configuration seam (C6).
- **Packet 124 and `raft-default-module.md` define** raft geometry,
  `ExtrusionRole::RaftInfill`, `claim:raft-fill`, the carrier
  (`SliceRegionView.raft_fill` versus synthetic raft layers), stage placement,
  and the per-infill-module dispatch addition.

No work in `raft-default-module.md` blocks Block B. Block C6 is the first
item with a hard `raft-default` interaction.

## Open Follow-ups

Recorded explicitly so they don't drift into "we'll get to that someday."

- **`TASK-163b-orca-ref`** — Replace self-capture goldens with real OrcaSlicer reference output. Blocked on: (a) a non-benchy fixture that's small enough for CI and complex enough for "Orca reference output" to be meaningful; (b) version-pinned OrcaSlicer in CI or as a release artifact; (c) agreed comparison metric beyond branch-count + Hausdorff. Owner: not assigned. Update `docs/07_implementation_status.md` to reflect this.
- **`rectilinear-infill` / `traditional-support` scan-line duplication.** Each has its own copy of the scan-line math (`rectilinear-infill/src/lib.rs:180-340`, `traditional-support/src/lib.rs:211-358`). Real fix is WIT-interface pattern services (modules invoking each other's algorithms across language boundaries). Out of scope here. Tracked separately.
- **`smooth_nodes` parameter tuning.** Orca uses 100 iterations as a constant; v1 will too. If output is over-smoothed in real-world prints, this becomes a config key.
- **f32 → coord_t (i64 at 1e-6 mm) precision.** Documented but not scheduled. Triggered if invariant 2 (collision-free) starts failing on dense models with large XY coordinates.
- **`support_interface_bottom_layers` real implementation.** Deferred. The not-implemented warning from B2 surfaces it; real implementation lands as a future packet.

## Out of Scope

- Replacement of `MinimumSpanningTree::prim` (Prim, O(V²)) with heap-based MST.
- Soluble multi-extruder interface support material.
- Catchup / variable-per-region effective layer-height interactions in support.
- Tree-organic (`smsTreeOrganic`) mode delegation to `TreeSupport3D` algorithm.
- Sharp-tail propagation, cantilever detection, OverhangCluster aggregation.
- Slim / Strong / Hybrid tree-support style variants.
- Adaptive support layer heights (`plan_layer_heights`).
- Support layer interface emission (top + bottom interface bands).
- Real implementation of `support_interface_bottom_layers`.
- `rectilinear-infill` / `traditional-support` scan-line duplication cleanup.
- Pattern-library extraction.

## TASK Ledger

Proposed new TASK rows for `docs/07_implementation_status.md` (renumber as appropriate before committing):

| TASK | Block | Description |
|---|---|---|
| TASK-163b-diagnostic | B7 | Typed Diagnostic channel on `world-prepass.wit`. Migrate three `support-planner` log call sites. |
| TASK-163b-orca-ref | follow-up | Real OrcaSlicer reference output. Blocked-on: fixture + Orca-runner infrastructure. |
| TASK-250 | B1 | Doc-comment honesty across `tree-support`, `traditional-support`, `support-planner`. |
| TASK-251 | B2 | `support_interface_bottom_layers` dead-state cleanup + `not_implemented` warning. |
| TASK-252 | B3 | `BASE_SPEED` documented as convention. |
| TASK-253 | B4 | 1024-contact silent-truncation diagnostic. |
| TASK-254 | B5 | Tip cone in `tapered_radius`. |
| TASK-255 | B6 | `inflate_polygon` replacement with `polygon_ops::offset`. |
| TASK-290 | C1 | Validation harness on `regression_wedge.stl` (invariants + self-capture golden). |
| TASK-261 | C2 | Migrate `support_paint_policy` to `SlicedRegion.segment_annotations`. |
| TASK-262 | C3 | `smooth_nodes` Laplacian smoothing port. |
| TASK-263 | C4 | Multi-neighbour MST propagation. |
| TASK-264 | C5 | `to_buildplate` tracking + unsupported-branch pruning + `support_on_build_plate_only`. |
| TASK-265 | C6 | `SupportPlanIR.raft_plan` field + `ExtrusionRole::RaftInfill` + `claim:raft-fill` mapping. |
| TASK-266 | C7 | `traditional-support` ↔ `SupportPlanIR` contract documented as "does not consume." |
| TASK-270 | follow-up | `rectilinear-infill` / `traditional-support` scan-line duplication — separate design conversation, depends on WIT-pattern-services architecture. |

## ADRs Produced by This Spec

- **ADR-0009 — Raft rendering reuses the `Layer::Infill` role/claim pattern.** Documents D6 + D7 + the rejection of a Rust shared library. Filed at `docs/adr/0009-raft-as-layer-infill-role.md`.
- **ADR-0010 — Typed Diagnostic channel on `world-prepass`.** Documents D11. Filed at `docs/adr/0010-typed-diagnostic-channel.md`.
