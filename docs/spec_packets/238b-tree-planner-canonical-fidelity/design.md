# Design: 238b-tree-planner-canonical-fidelity

## Controlling Code Paths

- Primary code path: `modules/core-modules/tree-support-planner/src/lib.rs` (~5.9k lines;
  guest WASM core-module; port of canonical `TreeSupport.cpp`). All divergence edits land
  inside it unless named below.
- Neighboring tests/fixtures:
  - `modules/core-modules/tree-support-planner/tests/{tree_family_tdd,smooth_nodes_tdd,multi_neighbour_mst_tdd,to_buildplate_tdd,wall_clearance_tdd,diagnostics_tdd,orca_parity_tdd,slicer_module_binding_tdd}.rs`
  - New test file this packet: `tests/tree_style_styles_tdd.rs` (styles + hybrid minting)
  - Scheduler negative: `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs`
    (named binary `scheduler_integration`) — AC-N2 style rejection extends the
    `out_of_range_support_threshold_angle_is_rejected` precedent.
  - Golden tripwire (236-strengthened): `orca_parity_tdd.rs::benchy_tree_support_regression_tripwire`,
    regen gate `SUPPORT_PLANNER_REGEN_GOLDEN=1`, goldens at
    `resources/golden/benchy_tree_support_regression_*`. E3 applies: classify drift before
    any rebless; tolerances frozen (Hausdorff ≤ 0.5 mm, branch-count drift ≤ 10%).
  - IR/WIT surface for DEV-144 transport: `crates/slicer-ir/src/slice_ir.rs`
    (`SupportPlanEntry`), `crates/slicer-schema/wit/deps/prepass-support-geometry/prepass-support-geometry.wit`
    (`record support-plan-entry`), marshal legs `crates/slicer-wasm-host/src/marshal/{in_.rs,native.rs}`.
  - Host offset path for AC-6: `crates/slicer-sdk/src/host.rs::offset_polygons`,
    `crates/slicer-sdk/src/host_batch.rs::offset_polygons_batch` +
    `OffsetRequest`, `crates/slicer-core/src/polygon_ops.rs::offset/inflate_once`,
    WIT `crates/slicer-schema/wit/deps/common.wit` (`offset-polygons`, `record offset-request`).
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not
  repeat delegation rules. Canonical evidence Q1-Q10 below was captured 2026-08-22 by a
  delegated probe and TRUSTED over the plan brief where they conflict.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it. (This packet's primary surface IS a guest module — every step that touches `modules/core-modules/tree-support-planner/**`, `crates/slicer-ir/**`, `crates/slicer-schema/wit/**`, or the SDK host services MUST clear the check before trusting a green suite; T7 additionally demands one real-mesh validation because crate-suite green can hide empty-plan regressions.)
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- E9 snake_case: any new config-key string consumed here (`support_style`) stays snake_case.
- Invariant 16: no verification command may match zero tests; every command asserts its
  matched-pass count in-run.

## Plan Corrections (authoritative over the plan brief)

1. **DEV-141 kernel premise does not match this checkout.** The plan §12 bullet frames
   DEV-141 as "`smooth_outward` vs canonical `clip_narrow_corner`". The delegated probe of
   canonical `smooth_nodes` (`TreeSupport.cpp`) found NO `clip_narrow_corner` call inside
   smoothing in this checkout: smoothing is a plain 100-iteration three-point averaging of
   `(position, radius)` over unprocessed branches. DEV-141's actual content lives in
   `crates/slicer-core/src/smooth_outward.rs` (`clip_narrow_corner`, far/blocked branch target
   selection — an intentional divergence FROM canonical on the grounds canonical's ternary
   moves the wrong iterator) and belongs to the interface-regularize path, not the tree
   planner. RESOLUTION: this packet resolves DEV-141 against the OBSERVED tree-side kernel —
   there is nothing to reconcile between `smooth_outward` and the tree smoothing path; the
   implementing swarm records that resolution in the DEVIATION_LOG row when closing it.
2. **Canonical `move_out_expolys` never restores `from0`.** The in-tree comment at
   `move_out_expolys` ("Canonical restores `from0` when the push-out exceeds the budget") is
   FALSE in this checkout: canonical computes `pt_max = from + normal(outward_dir,
   scale_(max_move_distance))`, clamps to it when `dist2 > SQ(max_move_distance)`, saves
   `from0`, and NEVER uses it for restore (Q7). The implementation fixes the behavior AND the
   comment (div 5.1); the plan already anticipated this ("the in-tree comment … is false").

## The Smoothing Decision Point (div 2.1 — explicit decision, implementers record the final call)

Canonical order: `drop_nodes` → `smooth_nodes` → `draw_circles` (Q2). PnP today: node-graph
`smooth_nodes` RUNS inside `plan_for_object` after drop/move and BEFORE emit gates (verified
live), while entry-level `smooth_branches` is production-dead (called only from
`smooth_nodes_tdd.rs`). The plan's div 2.1 text describes the older state (smoothing removed).
The open question is therefore narrower than the plan states, but still a real decision:

- **Option A — reinstate-before-gates (canonical-shaped):** keep/confirm the node-graph
  `smooth_nodes` call positioned immediately before the draw/emit pass so every downstream
  collision gate validates FINAL (post-smoothing) geometry. Matches canonical ordering;
  carries DEV-143's arithmetic choice (keep f64-relaxed, round-on-commit — the deliberate,
  recorded deviation — or adopt truncating integer passes for bit-parity).
- **Option B — reasoned deviation:** remove smoothing from the production path again and
  record WHY (exact-Z collision validation prefers un-moved geometry), keeping
  `smooth_nodes` test-only.

DECISION CRITERIA to record either way: (a) does the chosen branch keep AC-N1 true — emit-time
gates validate FINAL geometry with no overlap regression case; (b) golden drift classification
under E3 when the branch flips the tripwire; (c) DEV-143 arithmetic disposition. RECOMMENDATION:
Option A (canonical default; Ruling 8 makes canonical the default where replacing legitimate
prior behavior), with DEV-143 kept as the recorded arithmetic deviation. Final call belongs to
the implementers per plan div 2.1 wording; the packet stays draft until their recorded decision
plus AC-N1 evidence exists. Either branch satisfies AC-2's either-branch contract.

## Divergence Approaches (canonical evidence Q1-Q10)

- **Top-Z gap (AC-1).** Mechanism verified live: `plan_for_object` computes
  `z_distance_top_layers = round_up_divide(scale_(z_distance_top), scale_(layer_height)) + 1`
  via `mm_to_units`, seeds `gap_layers = z_distance != 0`, and `insert_contact_point` creates
  the virtual gap node (`distance_to_top = -gap_layers`, print_z = bottom_z,
  height = z_distance_top). Purely layer-count — no mm walk remains (Q1). Work = pinning tests
  under VARIABLE layer heights (where the deleted mm walk diverged) + comment-debt cleanup.
- **Role coexistence (AC-3).** Verified live: `build_roles` unions segments+areas per role,
  carves roof/floor out of body, keeps the remainder — the div 2.2 whole-layer
  `carved.clear()` is GONE. Canonical shape (Q3): per-node routing into
  roof_gap_areas | roof_1st_layer | roof_base_areas | roof_areas | base_areas, then
  `base_areas = diff_ex(base_areas, roofs-union)`; circles never unioned into one body region.
  Work = pin coexistence with a mixed-role-layer fixture; align routing vocabulary.
- **Circle fidelity (AC-4).** Canonical: `CIRCLE_RESOLUTION = SQUARE_SUPPORT ? 4 : 100`;
  SQUARE_SUPPORT ⇔ `avg_node_per_layer > 200` (Q3). PnP has the resolution switch live
  (`circle_resolution` from `contact_stats.avg_node_per_layer`) but ALSO truncates unioned
  capsule contours via `limit_contour_vertices(BRANCH_CIRCLE_SEGMENTS=16)` in
  `structural_body_regions`. Work = retire the 16-vertex cap on emitted role contours (keep
  fine per-node circles out of the pre-classification union; swept capsules stay as the
  documented port addition joining consecutive cross-sections).
- **Collision keying (AC-5a).** Canonical bakes radius into the volume then point-in tests
  (Q4: `calculate_collision` offsets by `scale_(radius + xy_distance)` then simplifies).
  PnP emit gates read `get_collision(0.0, l)` + `body_overlaps_occupancy` disc inflation
  (F-13 interim, self-documented interim). Work = switch production gates to
  radius-bucketed `get_collision(radius, l)` + point-in (`is_inside_ex`); retain
  `body_overlaps_occupancy` for its existing unit tests only, marked test-only.
- **Avoidance keying (AC-5b).** Canonical `get_avoidance(next_radius, …)` per-node (Q4).
  PnP verified live per-node-bucketed at branch-A and move-pass sites. Work = pin + stale-
  comment cleanup only.
- **Largest-part carve (AC-5c).** Canonical `avoid_object_remove_extra_small_parts` keeps
  ONLY the largest-area surviving part (Q5). PnP `build_roles` carve keeps all parts. Work =
  after the collision difference, select the max-shoelace-area part per drawn region.
- **Miter limits (AC-6).** Canonical defaults `jtMiter` + `DefaultMiterLimit = 3.0` (Q6).
  PnP: TreeVolumes routes through `polygon_ops::offset → inflate_once` at miter 2.0;
  `sample_contact_points` erodes at `OffsetJoinType::Miter, 0.0`; the host offset path exposes
  no miter-limit parameter. Work = add miter limit to `host::offset_polygons` AND batch
  `OffsetRequest` (additive optional WIT field on `record offset-request` + singular form;
  `cargo build --tests` after the WIT edit; rebuild guests SAME step), pass 3.0 at both
  planner sites. `slicer_core::polygon_ops::offset`'s own default stays 2.0 for other callers.
- **TreeVolumes ctor (AC-7).** Canonical simplifies lslices at
  `scale_(m_radius_sample_resolution)` in the ctor BEFORE outlines-below (Q9);
  `ExPolygon::simplify(tolerance) = union_ex(simplify_p(tolerance))` can merge holes/split
  parts. PnP stores raw outlines and its guest `expolygons_simplify` preserves structure.
  Work = simplify-at-0.2mm in `TreeVolumes::new` feeding `layer_outlines_below`, using a
  union-composing variant (host clip Union after per-ring simplify).
- **to_buildplate inflation (AC-8).** Canonical merge path tests
  `!is_inside_ex(get_collision(0, obj_layer_nr_next), next_position)` — xy-inflated (Q10);
  MOVE pass uses RAW outlines (F-14 exception — preserve). PnP: contact-seeding sites
  (`insert_contact_point` footprint tests / analysis-contact seeding) and the branch-A merged
  node test raw outlines today. Work = inflate the two named site classes; leave F-14 alone.
- **move_out_expolys (AC-9a).** Q7: dilated-ring projection + pt_max clamp + bool return;
  local `from0` never restored (Plan Correction #2). Work = rewrite the guest helper to
  offset+union the polygons, project onto the dilated ring, clamp on budget exceed, return
  bool; fix comment. Callers updated to the bool contract.
- **STUDIO-4252 retry args (AC-9b).** Q8: `max_move_between_samples =
  max_move_distance + radius_sample_resolution + EPSILON` passed as BOTH args ONLY at the
  `get_collision` fallback site; ELSEWHERE dilation = `radius_sample_resolution + EPSILON`
  with max-move staying the layer budget. Work = encode BOTH sites precisely (fallback site
  currently passes the too-small dilation).
- **Mesh-path shim (AC-10).** `plan_for_object` projects overhang triangles downward when no
  analysis contacts exist (self-documented legacy shim, div 3.2). Canonical consumes host
  per-layer overhang polygons. Work = prefer host-computed polygons wherever the host
  contract supplies them; where fixtures cannot carry them (coplanar plates whose closed-solid
  cross-section is empty — the shim's stated reason), RECORD the boundary precisely instead of
  forcing a contract change; shim must remain unreachable whenever analysis contacts exist.
- **Branch-A roof counter (AC-11).** Canonical branch A inherits the PARENT counter minus
  decrement (Q8; parent = larger `dist_mm_to_top`). PnP uses `max(id, nid)` minus decrement
  citing `insert_dropped_node`'s max-merge — which is the same-position dedup path, not
  branch A. Work = seed from `parent_id`'s counter.
- **Tree styles (AC-12, Ruling 3).** `is_strong`: unweighted neighbor sums;
  `movement = direction_to_outer + move_to_neighbor_center` with dot-product gate. Hybrid:
  mint `TreeNodeType::Polygon` nodes (never minted today) under the large-flat-overhang
  contact condition with own merge/move handling. Keyed off 238a's `support_style`
  (extend `from_config`'s slim-only match into a full style enum). Negative AC-N2: unknown
  values rejected/defaulted per the 238a declaration (bounds enforcement precedent).
- **Emit simplify gating (AC-13, DEV-142).** Canonical simplifies ONLY base_areas, ONLY under
  SQUARE_SUPPORT, at `scale_(line_width/2)` (Q3); the later diff against `trimming` is the
  bottom-Z clearance trim, not collision re-diff. PnP `build_roles` simplifies every role at
  0.0125 mm unconditionally with a wrong canonical citation. Work = gate the simplify
  (body/base areas only, square-support condition, line_width/2 tolerance), correct the
  comment; watch SupportPlanIR payload growth (DEV-142's original retention reason) and record
  the measured delta if it matters.
- **Extra-wall transport (AC-14, DEV-144).** Per-node `need_extra_wall` degrades to the
  per-layer `"tree-branch-extra-wall"` capability string; no renderer consumes it. Work =
  ONE additive carrier, named identically everywhere: a parallel `wall-counts: list<u32>`
  field inside WIT `record support-plan-skeleton`
  (`crates/slicer-schema/wit/deps/prepass-support-geometry/prepass-support-geometry.wit`,
  same length as its existing `points: list<point3>`; 0 = plain node, ≥1 = extra walls at
  that node), mirrored as `wall_counts: Vec<u32>` on `SupportPlanSkeleton`
  (`crates/slicer-ir/src/slice_ir.rs`) — NOT a new `support-plan-entry` field — mapped in
  BOTH marshal legs (`crates/slicer-wasm-host/src/marshal/in_.rs` wasm dispatch projection;
  `crates/slicer-wasm-host/src/marshal/native.rs` native builder), schema minor bump
  derived-at-activation (237 precedent: `SupportAnalysisIR.cantilever_surfaces`), legs
  updated together (T9), capability string demoted to provenance note. Renderer CONSUMPTION
  (printing extra walls) is 238c's; this packet delivers the transport.

## DEV-128 Sizing (Ruling 4 — f32 mm vs canonical scaled-integer coord_t)

Measured basis (grep counts on the live tree, 2026-08-22): ~232 `f32` occurrences and ~38
position/radius-typed parameter or field sites in
`modules/core-modules/tree-support-planner/src/lib.rs`; the node arena stores positions as
`Point2` (already scaled-integer i64) — the f32 exposure is concentrated in the mm-space math
AROUND the arena (`tapered_radius`, `calc_radius`, `move_out_expolys`, projections,
`neighbour_direction_sum`, MST weights, sweep hulls), not in the node positions themselves.
Retype blast radius estimate: the planner crate compiles standalone behind
`required-features`-free wasm/native targets and is consumed by renderers via `SupportPlanIR`
(mm floats) — the retype is therefore MODULE-LOCAL: ~38 call/field sites in lib.rs plus its
8 test files. That is M-sized work (multi-day-equivalent context churn across a 5.9k-line
guest), NOT S.

**OUTCOME (Ruling 4 split rule): sized L/M ⇒ NOT implemented in this packet. WAIVER RECORDED:**
DEV-128 stays Open with its existing trigger ("invariant-2 failures on dense large-XY models");
this packet's sizing section is the recorded waiver rationale. Mitigating fact: node POSITIONS
are already integer `Point2`; the residual f32 exposure is bounded by `max_move`
(`line_width/2`) per move step and does not accumulate over smoothing chains the way the
original DEV-128 row feared (that mechanism was fixed separately as DEV-143's opposite-direction
note). Revisit only if a real collision-freedom failure is attributed to mm-space rounding.

## Code Change Surface

- Selected approach: divergence-by-divergence edits inside the planner module, each red-first
  pinned by its owning test file, with two cross-crate additions (miter-limit plumbing;
  extra-wall IR/WIT transport) owned by dedicated steps.
- Exact functions/symbols touched (all `modules/core-modules/tree-support-planner/src/lib.rs`
  unless noted):
  `plan_for_object` (shim boundary, variable-height pinning), `build_roles`
  (+simplify gating, largest-part carve), `structural_body_regions`/`limit_contour_vertices`
  (retire 16-cap on emitted contours), `TreeVolumes::{new, ensure_collision, ensure_avoidance}`
  (ctor simplify; miter limit), `sample_contact_points` (erosion miter),
  `move_out_expolys` (rewrite), STUDIO-4252 retry call site + branch-A push-out site
  (args), `insert_contact_point` + branch-A `to_buildplate` site (inflation), branch-A roof
  counter, `from_config` (style enum), move-pass movement composition (`is_strong`),
  contact minting (hybrid Polygon nodes), `smooth_nodes` (decision-point outcome),
  emit capabilities block (extra-wall provenance).
  Cross-crate: `crates/slicer-sdk/src/host.rs` + `host_batch.rs` + `crates/slicer-core/src/polygon_ops.rs`
  + `crates/slicer-schema/wit/deps/common.wit` (miter param);
  `crates/slicer-ir/src/slice_ir.rs` + `prepass-support-geometry.wit` +
  `crates/slicer-wasm-host/src/marshal/{in_.rs,native.rs}` (transport);
  scheduler bounds table (AC-N2).
- Rejected alternatives: (a) whole-module coord_t retype now — rejected by sizing above;
  (b) renderer-side smoothing — rejected, canonical smooths nodes pre-draw, and entry-level
  smoothing translates validated polygons (the exact hazard the old comment documents);
  (c) folding renderer consumption of extra walls into AC-14 — rejected (238c owns render).

## Files in Scope (read + edit)

- `modules/core-modules/tree-support-planner/src/lib.rs` - role: ALL algorithm divergences;
  expected change: symbol-local edits listed above (ranged reads only — never full-load)
- `modules/core-modules/tree-support-planner/tests/*.rs` + new `tree_style_styles_tdd.rs` -
  role: red-first pins per AC; expected change: new tests + minimal fixture updates
- `crates/slicer-sdk/src/host.rs`, `.../host_batch.rs`, `crates/slicer-core/src/polygon_ops.rs`,
  `crates/slicer-schema/wit/deps/common.wit` - role: miter-limit parameter (AC-6);
  expected change: additive signature/WIT field, 3.0 at planner sites
- `crates/slicer-ir/src/slice_ir.rs`, `crates/slicer-schema/wit/deps/prepass-support-geometry/prepass-support-geometry.wit`,
  `crates/slicer-wasm-host/src/marshal/in_.rs`, `.../native.rs` - role: DEV-144 transport;
  expected change: additive skeleton payload field + minor schema bump + both legs
- `crates/slicer-runtime/tests/executor/*` - role: AC-1 variable-height pinning home
- Justified extras: each cross-crate touch is a thin mechanical surface owned by exactly one step.

## Read-Only Context

- `docs/specs/support-families-anchored-entities-plan.md` - §12 "238b" brief, §3 rulings,
  §13 traps - authority; ranged reads
- `docs/spec_packets/224-support-family-orca-closure/handoffs/orca-divergences.md` - rows
  cited in requirements scope table
- `docs/DEVIATION_LOG.md` - DEV-128/141/142/143/144 rows only
- `OrcaSlicerDocumented/**` - delegated always

## Out-of-Bounds Files

- `modules/core-modules/tree-support/**`, `modules/core-modules/traditional-support/**` -
  238c's renderer surface; read-only here
- `modules/core-modules/support-planner/**` - legacy module (DEV-128's original home);
  untouched
- `docs/specs/support-families-anchored-entities-plan.md`, `docs/specs/support-parity-gap-register.md`,
  other packet directories, `docs/07_implementation_status.md` content beyond TASK registration
  via the closure worker - doc hygiene rules
- `target/`, `Cargo.lock`, generated code, guest artifacts under
  `modules/core-modules/*/wit-guest/target/` - never load

## Expected Sub-Agent Dispatches

- Question: enumerate struct-literal/assertion sites compiling against `SupportPlanSkeleton`
  and the WIT `support-plan-skeleton` (blast radius for the DEV-144 field); scope:
  `crates/**`, `modules/**`, exclude `target/`; return: LOCATIONS ≤20; purpose: Step transport
  pre-bake.
- Question: confirm current goldens' branch-count/Hausdorff baseline values and the regen env
  gate name; scope: `resources/golden/benchy_tree_support_regression_*`,
  `modules/core-modules/tree-support-planner/tests/orca_parity_tdd.rs`; return: FACT; purpose:
  E3 drift classification during steps touching geometry.
- Question: locate the scheduler bounds-table insertion point for enum-valued string keys
  (support_style rejection); scope: `crates/slicer-scheduler/src/**`,
  `crates/scheduler/tests/integration/config_bounds_enforcement_tdd.rs`; return: LOCATIONS
  ≤10; purpose: AC-N2.
- Question: verify `OffsetRequest` consumers count (who breaks if the WIT record gains an
  optional field); scope: `crates/slicer-wasm-host/**`, `modules/core-modules/**`; return:
  LOCATIONS ≤20; purpose: Step miter.

## Data and Contract Notes

- IR/manifest contracts: `SupportPlanIR` schema bumps MINOR (additive skeleton payload field,
  derived-at-activation per 237 precedent — no frozen future version literals in code; the
  constant literal changes in the same step as its test fallout). No manifest edits here
  (declarations are 238a's).
- WIT boundary: two additive edits — `record offset-request` optional `miter-limit` (+
  singular `offset-polygons` overload param), and `record support-plan-skeleton` gains the
  parallel `wall-counts: list<u32>` field (same length as `points`; NOT a
  `support-plan-entry` field) — canonical sources under `crates/slicer-schema/wit/deps/`
  (both host bindgen! and guest include_str! read them). `cargo build --tests` immediately
  after each WIT edit; rebuild guests in the SAME step.
- Determinism/scheduler constraints: no new claims; planner keeps its existing claim set;
  serial determinism unchanged (all edits deterministic given identical inputs); invariant
  15 (per-region entries) untouched.

## Locked Assumptions and Invariants

- F-14 exception LOCKED: the per-descendant move-pass recompute tests RAW outlines forever
  (canonical does the same); no later packet may "fix" it.
- Body/nozzle-sweep disjointness, structured declines, family attribution, support-disabled-
  emits-nothing (invariants 1–14) hold throughout; nothing here weakens a gate to get green.
- The smoothing decision's outcome is locked once recorded; flipping it later re-runs the full
  human gate.

## Risks and Tradeoffs

- T7 (highest): crate-suite green hiding empty/near-empty plans on real meshes. Mitigation:
  wedge real-mesh slice is a REQUIRED human-gate artifact, not optional; visual-debug taps at
  changed-boundary layers.
- Golden drift: circle-fidelity + simplify gating WILL move `benchy_tree_support_regression_*`
  endpoints. E3: classify first; rebless only with justification recorded; tolerances frozen.
- Largest-part carve may drop thin slivers users previously saw printed — canonical-faithful,
  but call it out in the evidence file.
- WIT additive edits ripple into both marshal legs (T9 skew hit 3× historically): both legs
  change in ONE step, verified by the seam-identity test 236 exports
  (`native_and_wasm_layer_views_are_field_identical`).
- Emitting unsimplified fine circles grows serialized plan payloads (DEV-142's retention
  reason); measure the delta during AC-13 and record it rather than silently accepting.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step styles/hybrid; mitigated by symbol-windowed reads)
- Highest-risk dispatch: SupportPlanSkeleton blast-radius LOCATIONS (must be complete or the
  transport step fails late); required return format: LOCATIONS ≤20

## Open Questions

- [FWD] Smoothing decision final call: implementers record Option A/B + DEV-143 disposition in
  §The Smoothing Decision Point before Step 3 closes (packet stays draft until recorded).
- [FWD] If 238a lands a different `support_style` value spelling than
  `default|grid|snug|organic|tree_slim|tree_strong|tree_hybrid`, Step styles reconciles to
  238a's landed enum (forward-dep reconciliation rule).
- [FWD] DEV-128 waiver stands unless a collision-freedom failure is attributed to mm-space
  rounding during this packet's validation — if that happens, escalate to the orchestrator
  for a split packet rather than expanding scope here.
- [BLOCK] None at authoring time.
