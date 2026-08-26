---
status: implemented
packet: 238b-tree-planner-canonical-fidelity
task_ids:
  - TASK-369
  - TASK-370
  - TASK-371
  - TASK-372
  - TASK-373
  - TASK-374
  - TASK-375
  - TASK-376
  - TASK-377
  - TASK-378
  - TASK-379
  - TASK-380
depends_on: 238a-support-pattern-config-keys
backlog_source: docs/specs/support-families-anchored-entities-plan.md
context_cost_estimate: M
---

# Packet Contract: 238b-tree-planner-canonical-fidelity

## Goal

Bring the tree planner's algorithms to canonical fidelity — top-Z gap, smoothing
reinstatement, role coexistence, circle fidelity, collision/avoidance keying, miter limits,
`TreeVolumes` construction, `to_buildplate` inflation, `move_out_expolys`, STUDIO-4252 retry
args, mesh-path shim boundary, branch-A roof counter, tree styles, and emit simplify gating —
so every remaining recorded divergence (orca-divergences 1.1–5.7, 7.1–8.1; DEV-141..144) has a
canonical-matching implementation or a reasoned, tested deviation.

## Scope Boundaries

This packet owns algorithm-level fidelity inside
`modules/core-modules/tree-support-planner/src/lib.rs` and its test suite, keyed on the
config keys 238a declared (`support_style`, `max_bridge_length`). Renderer flow/density,
interface semantics, and the base-interface role stay with 238c; the AGG rasterizer stays
with 241; raft stays with 240. DEV-128 is SIZED here per plan Ruling 4 (§design.md); it is
implemented only if that sizing lands at S, otherwise deferred with a recorded waiver.

## Prerequisites and Blockers

- Depends on: `238a-support-pattern-config-keys` — FORWARD DEPENDENCY on a `status: draft`
  packet. This packet consumes the keys 238a declares: `support_style`
  (`default|grid|snug|organic|tree_slim|tree_strong|tree_hybrid`, default `"default"`),
  `max_bridge_length` (float, default 10.0), plus the host typed keys
  (`support_top_z_distance`, bottom-z, line-width float_or_percent). The tree styles'
  BEHAVIOR lands here per plan Ruling 3. If 238a's landed shape renames a key or changes a
  type, this packet reconciles in its Step 1 before closing.
- Unblocks: `238c-support-renderer-flow-interfaces` (consumes role regions, per-node extra-wall
  transport, circle fidelity), and 242 closure (every divergence row dispositioned).
- Activation blockers: none. The smoothing reinstatement decision is framed as an explicit
  design decision point (`design.md` §The Smoothing Decision Point) with decision criteria;
  implementers record the final call either way — both branches carry a negative AC proving
  emit-time collision gates validate FINAL geometry.

## Acceptance Criteria

State ACs only here; `requirements.md` references their IDs. Every command tees to
`target/test-output.log` and asserts a non-zero matched-pass count (plan invariant 16);
`slicer-core` commands carry `--features host-algos` (trap T5).

- **AC-1. Given** a layer plan with variable per-layer heights and a configured non-zero
  top-Z distance, **when** contacts are generated for an overhang, **then** the contact lands
  via the canonical layer-count mechanism (`round_up_divide(z_distance / layer_height) + 1`
  over the nominal layer height) with a virtual gap node (`distance_to_top = -gap_layers`,
  `gap_layers = z_distance == 0 ? 0 : 1`) — no mm walk over actual layer Z remains in contact
  generation — and the virtual node is propagated but never extruded.
  | `mkdir -p target && cargo test -p slicer-runtime --test executor --no-fail-fast -- 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS || echo FAIL`
- **AC-2. Given** the smoothing path selected by the packet's design decision point
  (either node-graph `smooth_nodes` reinstated before emit gates, or the reasoned no-smooth
  deviation), **when** the planner runs, **then** whichever branch was chosen holds:
  the 100-iteration three-point kernel runs over `(position, radius)` with
  `max_move = support_line_width / 2` immediately before the draw/emit pass (reinstate
  branch), OR `run_support_geometry` documents the recorded deviation and emits unsmoothed
  positions (deviation branch). In BOTH cases the emit-time collision gates validate the
  FINAL emitted geometry (the gate reads post-smoothing positions when smoothing runs).
  | `mkdir -p target && cargo test -p tree-support-planner --test smooth_nodes_tdd --no-fail-fast -- 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS || echo FAIL`
- **AC-3. Given** a layer where one branch sits inside its roof band while another passes
  through as body, **when** roles are built, **then** body and interface coexist disjointly
  (per-node classification into roof/floor/body areas, body = diff(body, roofs-union)) — the
  `if !roof.is_empty() || !floor.is_empty() { carved.clear() }` whole-layer clearing is gone.
  | `mkdir -p target && cargo test -p tree-support-planner --test tree_family_tdd --no-fail-fast -- 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS || echo FAIL`
- **AC-4. Given** a normal-density model (`avg_node_per_layer <= 200`), **when** circles are
  drawn, **then** per-node contours keep fine resolution (100-gon class, never truncated to
  the 16-vertex `BRANCH_CIRCLE_SEGMENTS` cap), coarse 4-gon only under the canonical
  square-support condition, and circles are routed by role without being unioned into one
  body region before classification.
  | `mkdir -p target && cargo test -p tree-support-planner --test multi_neighbour_mst_tdd --no-fail-fast -- 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS || echo FAIL`
- **AC-5. Given** collision and avoidance queries during drop/move/emit, **when** a volume is
  fetched, **then** it is keyed on the querying node's bucketed tapered radius
  (`get_collision(radius, l)` bakes `radius + xy_distance`; `get_avoidance(next_radius, …)`
  per node) with plain point-in tests against the pre-inflated volume — the
  `get_collision(0.0, l)` + test-time disc inflation (`body_overlaps_occupancy`) F-13 interim
  is retired from production paths — and the carve replicates
  `avoid_object_remove_extra_small_parts`' largest-part selection (small surviving slivers
  dropped).
  | `mkdir -p target && cargo test -p tree-support-planner --test wall_clearance_tdd --no-fail-fast -- 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS || echo FAIL`
- **AC-6. Given** polygon offsets in `TreeVolumes::ensure_collision`/`ensure_avoidance` and
  the inner-grid erosion of `sample_contact_points`, **when** offsets run with Miter joins,
  **then** they use the canonical miter limit 3.0 (via a new miter-limit parameter on the
  host offset path, both singular and batch forms), matching canonical `offset_ex` defaults
  at both sites.
  | `cargo check --workspace --all-targets && rg -q "miter_limit" crates/slicer-sdk/src/host.rs && echo PASS || echo FAIL`
- **AC-7. Given** `TreeVolumes::new`, **when** outlines are loaded, **then** each layer's
  outlines are simplified at `radius_sample_resolution` before `layer_outlines_below` unions
  them, using a simplify that includes the final union step (canonical
  `ExPolygon::simplify = union_ex(simplify_p(tolerance))`, which may merge holes or split
  parts) rather than the structure-preserving guest variant.
  | `mkdir -p target && cargo test -p tree-support-planner --lib expolygons_simplify 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS || echo FAIL`
- **AC-8. Given** contact seeding and the branch-A merged node, **when** `to_buildplate` is
  decided at those sites, **then** the test uses the xy-distance-inflated collision volume
  (`!is_inside_ex(get_collision(0, layer), position)`), while the F-14 per-descendant recompute
  in the move pass KEEPS testing raw outlines (canonical's move pass uses `m_layer_outlines`
  there — do not "fix" the exception).
  | `mkdir -p target && cargo test -p tree-support-planner --test to_buildplate_tdd --no-fail-fast -- 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS || echo FAIL`
- **AC-9. Given** `move_out_expolys`, **when** a push-out runs at any call site (branch-A
  group-0 push-out, STUDIO-4252 retry, F-13 escape), **then** it projects onto the DILATED
  ring (`polys_dilated = union(offset(polygons, distance))`), clamps to
  `pt_max = from + normal(outward_dir, max_move_distance)` on budget exceed (never aborts to
  the original point), returns bool, and the false "Canonical restores from0" comment is
  corrected. The STUDIO-4252 retry passes `max_move_between_samples =
  max_move_distance + radius_sample_resolution + EPSILON` as BOTH the dilation and max-move
  arguments (div 7.2), while other sites keep dilation = `radius_sample_resolution +
  EPSILON` with their own max-move budget (both sites encoded distinctly).
  | `mkdir -p target && cargo test -p tree-support-planner --lib move_out 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS || echo FAIL`
- **AC-10. Given** a fixture carrying no analysis contacts (coplanar plate), **when** the
  legacy mesh-path shim would project triangles, **then** it consumes host-computed overhang
  polygons wherever the host contract allows, or records the precise boundary (which inputs
  cannot reach the host path) in `design.md` §Mesh-path boundary; the shim is unreachable
  whenever analysis contacts exist for the object.
  | `mkdir -p target && cargo test -p tree-support-planner --test diagnostics_tdd --no-fail-fast -- 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS || echo FAIL`
- **AC-11. Given** the F-11 branch-A two-leaf collapse, **when** the merged node's roof
  counter is seeded, **then** it inherits the PARENT's counter minus the decrement
  (`node_parent->support_roof_layers_below - (parent.distance_to_top >= 0 ? 1 : 0)`),
  replacing the `max(id, nid)` merge; `insert_dropped_node`'s max-merge remains only on the
  same-position dedup path.
  | `mkdir -p target && cargo test -p tree-support-planner --test multi_neighbour_mst_tdd branch_a_two_leaf_collapse_inherits_parent_roof_counter -- --exact 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS || echo FAIL`
- **AC-12. Given** `support_style = tree_strong`, **when** neighbor sums and movement are
  computed, **then** unweighted neighbor sums and
  `movement = direction_to_outer + move_to_neighbor_center` with a dot-product gate apply
  (`is_strong`); given `support_style = tree_hybrid`, `TreeNodeType::Polygon` nodes are
  minted under the large-flat-overhang condition with their own merge/move handling; slim
  behavior is unchanged.
  | `mkdir -p target && cargo test -p tree-support-planner --test tree_style_styles_tdd --no-fail-fast -- 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS || echo FAIL`
- **AC-13. Given** the emit pass, **when** role regions are simplified, **then** simplification
  is gated to the canonical condition (only body/base areas, only under the square-support
  threshold `avg_node_per_layer > 200`, at `line_width / 2` tolerance); normal-case output
  carries unsimplified fine-resolution circles, and the in-tree comment citing canonical
  justification is corrected (div 8.1 / DEV-142 disposition).
  | `mkdir -p target && cargo test -p tree-support-planner --lib build_roles 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS || echo FAIL`
- **AC-14. Given** `need_extra_wall` computed per node, **when** a plan entry carrying a
  skeleton is emitted and marshalled, **then** the flag travels as ONE additive carrier end to
  end: a parallel field `wall-counts: list<u32>` inside `record support-plan-skeleton`
  (`crates/slicer-schema/wit/deps/prepass-support-geometry/prepass-support-geometry.wit`,
  same length as `points`; 0 = plain node, ≥1 = extra walls required at that node), mirrored
  as `wall_counts: Vec<u32>` on `SupportPlanSkeleton` (`crates/slicer-ir/src/slice_ir.rs`),
  mapped identically in BOTH marshal legs (the wasm dispatch projection
  `crates/slicer-wasm-host/src/marshal/in_.rs` and the native builder
  `crates/slicer-wasm-host/src/marshal/native.rs`), with the `SupportPlanIR` schema minor
  bump derived-at-activation per the 237 precedent — instead of degrading to the per-layer
  `"tree-branch-extra-wall"` capability string, so the renderer (238c) can print extra walls
  on exactly those branches.
  | `rg -q 'wall-counts' crates/slicer-schema/wit/deps/prepass-support-geometry/prepass-support-geometry.wit && rg -q 'wall_counts' crates/slicer-ir/src/slice_ir.rs && rg -q 'wall_counts' crates/slicer-wasm-host/src/marshal/in_.rs && rg -q 'wall_counts' crates/slicer-wasm-host/src/marshal/native.rs && echo PASS || echo FAIL`

## Negative Test Cases

- **AC-N1. Given** the smoothing-reinstatement decision resolved EITHER way, **when** the
  chosen configuration runs on a fixture with model occupancy adjacent to a branch, **then**
  the rejection case proves emit-time collision gates still validate FINAL geometry: a
  smoothed (or deliberately unsmoothed) position that falls inside the inflated collision
  volume is carved/pruned exactly as the un-smoothed baseline was — no regression case exists
  where enabling smoothing (or its absence) lets emitted geometry overlap the model.
  | `mkdir -p target && cargo test -p tree-support-planner --test wall_clearance_tdd --no-fail-fast -- 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS || echo FAIL`
- **AC-N2. Given** a config supplying an unsupported `support_style` value (anything outside
  the 238a-declared enum `default|grid|snug|organic|tree_slim|tree_strong|tree_hybrid`),
  **when** the planner resolves style, **then** the value is rejected at bounds enforcement
  or deterministically defaulted to `"default"` with the resolution recorded — never silently
  mapped onto a tree behavior (E9/T8 discipline; the declaration itself is 238a's, this
  packet owns the consumer-side contract).
  | `mkdir -p target && cargo test -p slicer-scheduler --test scheduler_integration config_bounds_enforcement_tdd::rejects_unknown_support_style_value -- --exact 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS || echo FAIL`
- **AC-N3. Given** any guest-affecting step of this packet, **when** a guest, dispatch, or
  parity test fails, **then** attribution is refused until `cargo xtask build-guests --check`
  exits 0 (E4/T4; the planner IS a guest WASM module — staleness presents as count
  divergence, not instantiation error). This is verified at the acceptance ceremony by the
  exit-0 gate below, not grepping for `STALE:`.
  | `cargo xtask build-guests --check && echo PASS || echo FAIL`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask build-guests --check` (exit 0 — mandatory before attributing any
  guest/planner/parity failure; this packet edits `modules/core-modules/tree-support-planner/**`
  and possibly `crates/slicer-schema/wit/**`, all inside the staleness snippet's scope)
- Narrow suites: `cargo test -p tree-support-planner --features "" --no-fail-fast` per-step
  (see requirements matrix); WIT-touching steps add `cargo build --tests` then rebuild guests.

## Authoritative Docs

- `docs/specs/support-families-anchored-entities-plan.md` - §3 Rulings 3/4/8, §6 invariants
  (esp. 16), §7 E1-E9, §8 human gate, §12 brief "238b-tree-planner-canonical-fidelity",
  §13 traps T1/T4/T5/T6/T7/T8 (direct ranged reads at authoring time; done)
- `docs/spec_packets/224-support-family-orca-closure/handoffs/orca-divergences.md` -
  divergence rows 1.1-5.7, 7.1-8.1 (ranged read; dispositions recorded in this packet's
  requirements.md, DEVIATION_LOG untouched here)
- `docs/DEVIATION_LOG.md` rows DEV-141..DEV-144, DEV-128 - ranged read only; closure edits
  are implementation work, NOT this packet's (doc hygiene rule)
- `docs/07_implementation_status.md` - delegated SUMMARY only; TASK-369+ registration is the
  packet-owned closure step
- `docs/08_coordinate_system.md` - unit discipline checklist (ranged read)

## Human Validation Gate

Blocking per plan §8: this packet may not flip to `status: implemented` without a dated
sign-off line at the bottom of this section.

Artifacts to produce (all under `tmp/p238b-*`, gitignored — verify by direct listing, trap T1):

1. Tree G-code of the tracked fixture `crates/slicer-runtime/tests/fixtures/support-family/
   SupportTest.stl` sliced with `tmp/support-family-config-tree-matched.json` → `tmp/p238b-tree-fixture.gcode`.
2. **Non-coplanar real-mesh case (T7 — mandatory, not optional):** slice
   `resources/regression_wedge.stl` (or another tracked non-coplanar mesh if unavailable)
   through the full pipeline → `tmp/p238b-wedge.gcode`. Crate-suite green coexisted once with
   empty plans on real meshes (G-23); fixture-only evidence is insufficient.
3. Visual-debug bundle for THIS packet's boundary: `pnp_cli visual-debug` taps at the layers
   where smoothing/keying/role-coexistence deltas are visible, indexed by `manifest.json`,
   written under `tmp/p238b-vd/`.

Checklist to sign (each item names source, layer, tap, verdict; per E2 written inspection,
never a test claim):

- Termination: columns reach the plate beneath the overhang on BOTH geometries; no column
  terminates short of the plate or passes through the model.
- Coverage: every overhang region retains support contact; smoothing/keying changes did not
  starve any candidate.
- Collision freedom: no emitted body/interface region intersects model occupancy on its own
  layer (spot-checked at the visual-debug taps and against the wall-clearance fixtures).
- Interfaces: roof bands appear exactly where `support_roof_layers_below > 0` after the
  branch-A counter fix; floor bands where branches land on the model.
- Block counts vs references: `;TYPE:Support` block counts compared numerically against
  `tmp/SupportTest_Tree_Orca.gcode`, deltas recorded in the evidence file.

Evidence file: `tmp/p238b-human-validation.md` recording commands, artifact paths, layer
indices inspected, and block-count deltas.

Sign-off: **2026-08-25 — APPROVED** (human verdict delivered in the implementation
session after the branch-discretization fix: visual result approved against
`tmp/SupportTest_Tree_Orca.gcode`; comparison bundles `tmp/vdcmp/{ours,ref}`;
remaining look-and-feel deltas — trunk infill pattern (45° crosshatch), tip
solidity/density, top-layer tip counts, skeleton-based branch rendering — are
renderer-surface work explicitly assigned to `238c-support-renderer-flow-interfaces`,
which also carries any further planner fixes surfaced during its implementation).

## Doc Impact Statement (Required)

- `docs/02_ir_schemas.md` `SupportPlanIR` paragraph (per-node extra-wall transport, AC-14) -
  `rg -q 'SupportPlanIR' docs/02_ir_schemas.md`
- `docs/02_ir_schemas.md` schema-version table row for the additive SupportPlanIR bump - 
  `rg -q 'schema' docs/02_ir_schemas.md`
- `docs/15_config_keys_reference.md` unchanged this packet (declarations were 238a's; this
  packet adds none) - `rg -q 'support_style' docs/15_config_keys_reference.md`
- `docs/07_implementation_status.md` - TASK-369..380 rows registered by the packet-owned
  closure step (TASK-380), per `task-map.md` - `rg -q 'TASK-380' docs/07_implementation_status.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` — `generate_contact_points`
  (z_distance_top_layers round-up-divide +1; virtual gap node create_node(pt, -gap_layers, …));
  `smooth_nodes` (100 iterations, max_move = scale_(support_line_width/2), NO clip_narrow_corner
  kernel in this checkout — plain 3-point averaging of position+radius over unprocessed branches);
  `draw_circles` (SQUARE_SUPPORT = avg_node_per_layer > 200; CIRCLE_RESOLUTION 4/100; routing
  into roof_gap_areas | roof_1st_layer | roof_base_areas | roof_areas | base_areas;
  base_areas = diff_ex(base_areas, roofs-union); simplify ONLY base_areas ONLY under
  SQUARE_SUPPORT at scale_(line_width/2)); `drop_nodes` (branch-A parent-inherited roof
  counter; max_move_between_samples passed as BOTH move_out_expolys args at the get_collision
  fallback site only); `move_nodes` (get_avoidance(next_radius,…); to_buildplate via
  !is_inside_ex(get_collision(0,…))); `move_out_expolys` (dilated-ring projection; pt_max clamp;
  bool return; from0 saved but NEVER restored); `avoid_object_remove_extra_small_parts`
  (keeps only the largest-area surviving part)
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` — `TreeSupportData` ctor
  (lslices simplified at scale_(m_radius_sample_resolution) BEFORE building
  m_layer_outlines_below)
- `OrcaSlicerDocumented/src/libslic3r/ClipperUtils.hpp` — `DefaultMiterLimit = 3.0`;
  SUPPORT_SURFACES_OFFSET_PARAMETERS (jtSquare, 0 override used in detect_overhangs/trimming,
  NOT the tree path)
- `OrcaSlicerDocumented/src/libslic3r/ExPolygon.cpp` — `ExPolygon::simplify(tolerance)` =
  union_ex(simplify_p(tolerance)) — the union can merge holes/split parts

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
