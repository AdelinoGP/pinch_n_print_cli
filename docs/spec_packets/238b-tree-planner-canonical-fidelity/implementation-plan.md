# Implementation Plan: 238b-tree-planner-canonical-fidelity

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".
- Guest-affecting steps (1–9) end with `cargo xtask build-guests --check` exit 0 before their
  suite result is trusted (T4). `lib.rs` is ~5.9k lines: read ±40-line windows around cited
  symbols only.

## Steps

### Step 1: Top-Z gap variable-height pinning + forward-dep reconciliation

- Task IDs: `TASK-369`
- Objective: Pin the canonical layer-count contact mechanism under VARIABLE per-layer heights
  (the case the deleted mm walk got wrong); reconcile consumed 238a key spellings if its landed
  shape differs.
- Precondition: 238a generated (forward dep); guest freshness exit 0.
- Postcondition: new executor test passes; AC-1 command green.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/tree-support-planner/src/lib.rs` - windows around `plan_for_object`
    (~1650-1720), `insert_contact_point` (~3734-3800)
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/executor/` (new test file + `main.rs` mod line)
  - `modules/core-modules/tree-support-planner/src/lib.rs` (comment debt only)
- Files explicitly out of bounds:
  - renderer modules; scheduler; WIT
- Expected sub-agent dispatches:
  - Question: does the landed 238a enum match `default|grid|snug|organic|tree_slim|tree_strong|tree_hybrid`?; scope: `docs/spec_packets/238a-support-pattern-config-keys/packet.spec.md`, live manifest; return: FACT
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-families-anchored-entities-plan.md` - §12 "238b" top-Z bullet
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` - `generate_contact_points` (Q1); delegate only if evidence insufficient
- Verification:
  - `mkdir -p target && cargo test -p slicer-runtime --test executor -- --no-fail-fast 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - FACT pass/fail
  - `cargo xtask build-guests --check` - FACT exit code
- Exit condition: variable-height test green; freshness exit 0. Falsifying exit: test red after
  fix ⇒ stop, re-derive from Q1 evidence, do not weaken the assertion.

### Step 2: move_out_expolys rewrite (dilated ring, pt_max clamp, bool)

- Task IDs: `TASK-370`
- Objective: Canonical Q7 semantics at all three call sites; correct the false from0 comment.
- Precondition: Step 1 green.
- Postcondition: unit tests for dilated projection + clamp + bool contract green; call sites
  compile against bool.
- Files allowed to read:
  - `modules/core-modules/tree-support-planner/src/lib.rs` - `move_out_expolys` window
    (~4465-4540), branch-A push-out (~2258-2268), STUDIO-4252 retry (~2538-2552),
    `projection_onto` (~4437+)
- Files allowed to edit (at most 3):
  - `modules/core-modules/tree-support-planner/src/lib.rs`
  - `modules/core-modules/tree-support-planner/tests/to_buildplate_tdd.rs` (caller-contract tests)
- Files explicitly out of bounds: WIT; host services
- Expected sub-agent dispatches: none
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/support-families-anchored-entities-plan.md` - §12 div 5.1 bullet
- OrcaSlicer refs:
  - `TreeSupport.cpp::move_out_expolys` (Q7 — recorded evidence sufficient)
- Verification:
  - `mkdir -p target && cargo test -p tree-support-planner --lib move_out -- --nocapture 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - FACT pass/fail
  - `cargo xtask build-guests --check` - FACT exit code
- Exit condition: clamp-not-abort proven by a budget-exceed test; false comment gone.
  Falsifying exit: any call-site behavior flip that regresses wall_clearance_tdd ⇒ stop and
  classify before proceeding.

### Step 3: Smoothing decision resolution + STUDIO-4252 retry args (div 7.2, DEV-141/143 disposition)

- Task IDs: `TASK-371`
- Objective: Record the smoothing decision in design.md §The Smoothing Decision Point
  (recommend Option A); encode BOTH move_out_expolys arg sites per Q8 (fallback site gets
  max_move_between_samples as BOTH args; other sites keep dilation=resolution+EPSILON);
  resolve AC-N1's rejection case either way.
- Precondition: Step 2 merged (bool contract).
- Postcondition: decision text present; retry site updated; AC-2 + AC-N1 green.
- Files allowed to read:
  - `src/lib.rs` windows: `smooth_nodes` (~535-650), retry site (~2538-2556), branch-A push-out
  - this packet's `design.md` - decision section
- Files allowed to edit (at most 3):
  - `docs/spec_packets/238b-tree-planner-canonical-fidelity/design.md` (decision record)
  - `modules/core-modules/tree-support-planner/src/lib.rs`
  - `modules/core-modules/tree-support-planner/tests/smooth_nodes_tdd.rs`
- Files explicitly out of bounds: DEVIATION_LOG (closure is later implementation work);
  slicer-core smooth_outward
- Expected sub-agent dispatches: none
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/support-families-anchored-entities-plan.md` - §12 div 2.1 bullet
- OrcaSlicer refs:
  - `TreeSupport.cpp::smooth_nodes`, `drop_nodes` (Q2, Q8)
- Verification:
  - `mkdir -p target && cargo test -p tree-support-planner --test smooth_nodes_tdd -- --no-fail-fast 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - FACT pass/fail (AC-2/AC-N1)
  - `cargo xtask build-guests --check` - FACT exit code
- Exit condition: decision recorded; both arg sites provably distinct in tests. Falsifying
  exit: golden tripwire flips with unclassifiable drift ⇒ E3 stop.

### Step 4: Role coexistence + circle fidelity + simplify gating (AC-3/4/13)

- Task IDs: `TASK-372`
- Objective: Mixed-role layer keeps body+interface disjointly (pin existing carve); retire
  `limit_contour_vertices(16)` on emitted role contours; gate `build_roles`' simplify to
  base-areas-only under square support at line_width/2; correct its comment.
- Precondition: Steps 1-3 green.
- Postcondition: coexistence fixture green; contours carry fine vertices; AC-3/AC-4/AC-13 green.
- Files allowed to read:
  - `src/lib.rs` windows: `build_roles` (~762-850), `structural_body_regions`/`limit_contour_vertices`
    (~855-900), draw/emit pass (~2718-3000, ~3270-3300)
- Files allowed to edit (at most 3):
  - `modules/core-modules/tree-support-planner/src/lib.rs`
  - `modules/core-modules/tree-support-planner/tests/tree_family_tdd.rs`
  - `modules/core-modules/tree-support-planner/tests/multi_neighbour_mst_tdd.rs`
- Files explicitly out of bounds: renderer modules; goldens (regen is ceremony-owned)
- Expected sub-agent dispatches:
  - Question: current benchy golden endpoint values + branch count; scope: `resources/golden/benchy_tree_support_regression_*`; return: FACT
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/support-families-anchored-entities-plan.md` - §12 div 2.2/2.3/8.1 bullets
- OrcaSlicer refs:
  - `TreeSupport.cpp::draw_circles` (Q3)
- Verification:
  - `mkdir -p target && cargo test -p tree-support-planner --test tree_family_tdd -- --no-fail-fast 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - FACT pass/fail
  - `mkdir -p target && cargo test -p tree-support-planner --lib build_roles -- --exact 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - FACT pass/fail
  - `cargo xtask build-guests --check` - FACT exit code
- Exit condition: all three greens. Falsifying exit: payload growth measured catastrophic or
  golden drift unclassifiable ⇒ record, do not force.

### Step 5: Collision/avoidance keying + largest-part carve (AC-5)

- Task IDs: `TASK-373`
- Objective: Production gates read radius-baked volumes + point-in; retire production
  `body_overlaps_occupancy`; carve keeps largest surviving part only (Q4/Q5).
- Precondition: Step 4 green.
- Postcondition: emit gates radius-bucketed; largest-part selection tested; stale
  constant-radius comments removed; AC-5 green.
- Files allowed to read:
  - `src/lib.rs` windows: emit gate block (~2770-2870), `body_overlaps_occupancy` (~4119),
    `swallowed_by_collision`/`node_swallowed` closures (~2833-2870), carve sites (~3040-3200)
- Files allowed to edit (at most 3):
  - `modules/core-modules/tree-support-planner/src/lib.rs`
  - `modules/core-modules/tree-support-planner/tests/wall_clearance_tdd.rs`
  - `modules/core-modules/tree-support-planner/tests/tree_family_tdd.rs` (assertion fallout)
- Files explicitly out of bounds: slicer-sdk host path (Step 7 owns it)
- Expected sub-agent dispatches:
  - Question: enumerate tests asserting body_overlaps_occupancy / get_collision(0.0 semantics); scope: `modules/core-modules/tree-support-planner/tests/`; return: LOCATIONS ≤10
- Context cost: `M`
- Authoritative docs:
  - plan §12 div 4.1/4.2/7.1 bullets
- OrcaSlicer refs:
  - `TreeSupport.cpp::draw_circles`, `avoid_object_remove_extra_small_parts` (Q4/Q5)
- Verification:
  - `mkdir -p target && cargo test -p tree-support-planner --test wall_clearance_tdd -- --no-fail-fast 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - FACT pass/fail
  - full crate suite narrow run (tee'd) - FACT binary-count reconciliation vs prior runs
  - `cargo xtask build-guests --check` - FACT exit code
- Exit condition: no production caller of body_overlaps_occupancy remains; largest-part test
  green. Falsifying exit: collision freedom regression on wedge fixture ⇒ T7 stop-and-diagnose.

### Step 6: to_buildplate inflation split + branch-A roof counter (AC-8/11)

- Task IDs: `TASK-374`
- Objective: Contact-seeding + branch-A sites test inflated `get_collision(0,l)`; F-14 raw-
  outline exception preserved and comment-pinned; branch-A counter inherits PARENT minus
  decrement (Q8/Q10).
- Precondition: Step 5 green.
- Postcondition: AC-8/AC-11 green; F-14 exception has a pinning assertion.
- Files allowed to read:
  - `src/lib.rs` windows: `insert_contact_point` (~3734-3800), branch-A block (~2205-2300),
    F-14 recompute (~2605-2615)
- Files allowed to edit (at most 3):
  - `modules/core-modules/tree-support-planner/src/lib.rs`
  - `modules/core-modules/tree-support-planner/tests/to_buildplate_tdd.rs`
  - `modules/core-modules/tree-support-planner/tests/multi_neighbour_mst_tdd.rs`
- Files explicitly out of bounds: anything touching the move-pass recompute beyond comments
- Expected sub-agent dispatches: none
- Context cost: `S`
- Authoritative docs: plan §12 div 4.6/5.5/5.6 bullets
- OrcaSlicer refs: `move_nodes`, `drop_nodes` (Q8/Q10)
- Verification:
  - `mkdir -p target && cargo test -p tree-support-planner --test to_buildplate_tdd -- --no-fail-fast 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - FACT pass/fail
  - `cargo xtask build-guests --check` - FACT exit code
- Exit condition: inflation split proven (both directions asserted). Falsifying exit: F-14
  behavior change detected ⇒ revert that hunk, it was an out-of-bounds edit.

### Step 7: Miter limit plumbing (AC-6) — cross-crate additive parameter

- Task IDs: `TASK-375`
- Objective: Add miter-limit to host offset path (singular + batch + WIT optional field);
  planner passes 3.0 at TreeVolumes offsets and sample_contact_points erosion (Q6).
- Precondition: Step 6 green.
- Postcondition: WIT builds (`cargo build --tests`), guests rebuilt, planner sites at 3.0,
  other callers unchanged (default preserved).
- Files allowed to read:
  - `crates/slicer-core/src/polygon_ops.rs` - offset/inflate_once (~360-480)
  - `crates/slicer-sdk/src/host.rs` - offset_polygons (~487-520); `host_batch.rs` OffsetRequest
  - `crates/slicer-schema/wit/deps/common.wit` - host-services interface (~46-90)
- Files allowed to edit (at most 3):
  - `common.wit` + SDK host/batch files (counted as one logical surface; keep the edit inside
    two files by adding the param to singular + reusing OffsetRequest struct)
  - `crates/slicer-wasm-host/src/wit_host.rs` (service impl)
  - `modules/core-modules/tree-support-planner/src/lib.rs` (call sites only)
- Files explicitly out of bounds: polygon_ops default value (other callers depend on 2.0)
- Blast-radius discipline (mandatory): dispatch a LOCATIONS worker for every
  `offset_polygons(`/`OffsetRequest {` literal BEFORE editing; add each broken construction
  site to this step's edit list or use a defaulted optional so zero sites break.
- Expected sub-agent dispatches:
  - Question: list all OffsetRequest constructions and offset_polygons callers; scope: crates+modules; return: LOCATIONS ≤20
- Context cost: `M`
- Authoritative docs: plan §12 div 3.3/4.3 bullets
- OrcaSlicer refs: `ClipperUtils.hpp` defaults (Q6)
- Verification:
  - `cargo build --tests` then `cargo xtask build-guests` (rebuild) - FACT pass/fail
  - `mkdir -p target && cargo test -p tree-support-planner --lib sample_contact -- --nocapture 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - FACT pass/fail
  - `cargo xtask build-guests --check` - FACT exit code
- Exit condition: guests fresh; planner sites at 3.0; unrelated callers untouched.
  Falsifying exit: any non-support caller breaks ⇒ use defaulted-optional form instead.

### Step 8: TreeVolumes ctor simplify + union-composing simplify variant (AC-7)

- Task IDs: `TASK-376`
- Objective: Simplify outlines at radius_sample_resolution in ctor before outlines_below;
  add the union-including simplify shape (canonical ExPolygon::simplify, Q9).
- Precondition: Step 7 green.
- Postcondition: AC-7 green; hole-merge/part-split topology covered by a unit test.
- Files allowed to read:
  - `src/lib.rs` windows: `expolygons_simplify` (~1141-1157), `TreeVolumes::new` (~1209-1285)
- Files allowed to edit (at most 3):
  - `modules/core-modules/tree-support-planner/src/lib.rs`
  - in-file `#[cfg(test)]` module additions only (same file)
- Files explicitly out of bounds: host simplify service
- Expected sub-agent dispatches: none
- Context cost: `S`
- Authoritative docs: plan §12 div 4.4/4.5 bullets
- OrcaSlicer refs: `TreeSupportData` ctor, `ExPolygon.cpp::simplify` (Q9)
- Verification:
  - `mkdir -p target && cargo test -p tree-support-planner --lib expolygons_simplify -- --exact 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - FACT pass/fail
  - `cargo xtask build-guests --check` - FACT exit code
- Exit condition: union-topology unit test green. Falsifying exit: outlines_below consumers
  regress ⇒ check whether the union step is fed simplified-or-raw inputs per Q9 order.

### Step 9: Mesh-path shim boundary (AC-10)

- Task IDs: `TASK-377`
- Objective: Prefer host-computed overhang polygons where the contract supplies them; shim
  unreachable whenever analysis contacts exist; boundary recorded precisely otherwise.
- Precondition: Step 8 green.
- Postcondition: gating test proves shim unreachable with analysis contacts present; boundary
  paragraph added to design.md §Mesh-path boundary if a gap remains.
- Files allowed to read:
  - `src/lib.rs` windows: analysis-contact gate (~1735-1800), triangle projection (~1763-1830),
    paint/enforcer collectors (~4030+)
- Files allowed to edit (at most 3):
  - `modules/core-modules/tree-support-planner/src/lib.rs`
  - `modules/core-modules/tree-support-planner/tests/diagnostics_tdd.rs`
  - this packet's `design.md` (boundary record, only if needed)
- Files explicitly out of bounds: host analysis producer (237 surface)
- Expected sub-agent dispatches:
  - Question: does SupportAnalysisView/mesh view carry host overhang polygons reachable by the planner today?; scope: `crates/slicer-sdk/src/views.rs`, `crates/slicer-runtime/src/builtins/support_analysis_producer.rs`; return: FACT
- Context cost: `S`
- Authoritative docs: plan §12 div 3.2 bullet
- OrcaSlicer refs: `generate_contact_points` sampling `layer->loverhangs`
- Verification:
  - `mkdir -p target && cargo test -p tree-support-planner --test diagnostics_tdd -- --no-fail-fast 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - FACT pass/fail
  - `cargo xtask build-guests --check` - FACT exit code
- Exit condition: reachability gate asserted. Falsifying exit: host contract genuinely cannot
  supply polygons for non-coplanar meshes ⇒ boundary record IS the deliverable; mark AC-10
  satisfied-by-recorded-boundary in requirements.

### Step 10: Tree styles — strong movement rule + hybrid Polygon minting (AC-12, AC-N2)

- Task IDs: `TASK-378`
- Objective: Full style enum in from_config (slim unchanged); `is_strong` unweighted sums +
  summed movement with dot-product gate; hybrid mints TreeNodeType::Polygon under large-flat-
  overhang condition with own merge/skip handling; scheduler negative for unknown values.
- Precondition: Step 9 green; 238a declaration confirmed (Step 1 FACT).
- Postcondition: new tree_style_styles_tdd.rs green; AC-N2 scheduler negative green;
  AC-12 green.
- Files allowed to read:
  - `src/lib.rs` windows: `from_config` (~1395-1460 style match), neighbour sum (~2470-2560),
    movement composition (~2550-2570), TreeNodeType uses (~567, ~2226, ~2321)
  - `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` - precedent test
- Files allowed to edit (at most 3):
  - `modules/core-modules/tree-support-planner/src/lib.rs`
  - `modules/core-modules/tree-support-planner/tests/tree_style_styles_tdd.rs` (NEW)
  - `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` (negative)
- Files explicitly out of bounds: 238a manifest declarations; PrintConfig docs
- Blast-radius discipline: style field addition touches `SupportPlanner` construction in tests
  via `from_config` only (no public struct-literal churn expected); verify with one
  `cargo check -p tree-support-planner --tests` before proceeding.
- Expected sub-agent dispatches:
  - Question: bounds-table insertion point for enum-valued string keys; scope: `crates/slicer-scheduler/src/**`; return: LOCATIONS ≤10
- Context cost: `M` (largest step; symbol-windowed reads mandatory)
- Authoritative docs: plan §12 div 5.7 bullet; Ruling 3
- OrcaSlicer refs: `drop_nodes` is_strong branch; `generate_contact_points` hybrid minting
- Verification:
  - `mkdir -p target && cargo test -p tree-support-planner --test tree_style_styles_tdd -- --no-fail-fast 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - FACT pass/fail
  - `mkdir -p target && cargo test -p slicer-scheduler --test scheduler_integration rejects_unknown_support_style_value -- --exact 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - FACT pass/fail
  - `cargo xtask build-guests --check` - FACT exit code
- Exit condition: styles selectable and distinct; unknown rejected/defaulted deterministically.

### Step 11: DEV-144 extra-wall transport (AC-14) — IR/WIT additive bump

- Task IDs: `TASK-379`
- Objective: Per-node wall counts ride the skeleton payload through IR + WIT + both marshal
  legs as ONE additive carrier — `wall-counts: list<u32>` inside `record
  support-plan-skeleton` (same length as its `points: list<point3>`; 0 = plain node, ≥1 =
  extra walls at that node), mirrored as `wall_counts: Vec<u32>` on `SupportPlanSkeleton`
  (`crates/slicer-ir/src/slice_ir.rs`) — NOT a new field on `support-plan-entry`. Both
  marshal mappings name the same field: the wasm dispatch projection in
  `crates/slicer-wasm-host/src/marshal/in_.rs` and the native builder
  `crates/slicer-wasm-host/src/marshal/native.rs`. Capability string demoted to provenance
  note; schema minor bump derived-at-activation.
- Precondition: Step 10 green; blast-radius LOCATIONS complete.
- Postcondition: flag observable end-to-end native+wasm (seam identity holds); AC-14 green.
- Files allowed to read:
  - `crates/slicer-ir/src/slice_ir.rs` - SupportPlanSkeleton (~1318-1322), schema consts (~250-270)
  - `prepass-support-geometry.wit` - skeleton records (~9-31)
  - `crates/slicer-wasm-host/src/marshal/in_.rs`, `native.rs` - skeleton transport legs
- Files allowed to edit (at most 3):
  - `crates/slicer-schema/wit/deps/prepass-support-geometry/prepass-support-geometry.wit`
    (add `wall-counts: list<u32>` to `record support-plan-skeleton`) +
    `crates/slicer-ir/src/slice_ir.rs` (add `wall_counts: Vec<u32>` to `SupportPlanSkeleton`)
    (+ macro derive fallout in `crates/slicer-macros/` if the LOCATIONS sweep demands)
  - `crates/slicer-wasm-host/src/marshal/in_.rs` AND `native.rs` (one logical leg pair; both
    map `points` ↔ `wall_counts` positionally, same length asserted)
  - `modules/core-modules/tree-support-planner/src/lib.rs` (emit site ~3325-3345 fills
    per-node wall counts from the node arena)
- Files explicitly out of bounds: renderer consumption (238c)
- Blast-radius discipline: pre-baked via Step-11 dispatch below; every SupportPlanSkeleton /
  support-plan-skeleton literal found lands in this step's edit list; the schema-version
  constant change and its hard-asserting tests land in THIS step. The parallel-list length
  invariant (len(wall_counts) == len(points)) is asserted at both marshal legs and at emit.
- Expected sub-agent dispatches:
  - Question: all SupportPlanSkeleton / support-plan-skeleton literals + marshal points mappings + tests asserting CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION; scope: crates/** modules/**; return: LOCATIONS ≤20
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md` - SupportPlanIR section (ranged read)
  - AGENTS.md "WIT/Type Changes Checklist"
- OrcaSlicer refs: `smooth_nodes` need_extra_wall producer/consumer
- Verification:
  - `cargo build --tests` then `cargo xtask build-guests` - FACT pass/fail
  - `mkdir -p target && cargo test -p slicer-runtime --test contract native_and_wasm_layer_views_are_field_identical -- --exact 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - FACT pass/fail
  - `cargo xtask build-guests --check` - FACT exit code
- Exit condition: seam identity green; `wall_counts` visible in emitted skeleton entries with
  length matching points; version asserts updated in-step. Falsifying exit: leg skew detected
  ⇒ fix both legs before any further step.

### Step 12: Closure — gates, real-mesh validation, registration

- Task IDs: `TASK-380`
- Objective: Packet completion gate: clippy/literals/freshness green; human-gate artifacts
  produced; TASK rows registered in docs/07 via worker dispatch.
- Precondition: Steps 1-11 complete.
- Postcondition: packet ready for status flip after sign-off.
- Files allowed to read: gate outputs; `target/test-output.log`
- Files allowed to edit (at most 3):
  - `docs/07_implementation_status.md` (via worker; TASK-369..380 rows)
  - `tmp/p238b-*` artifacts (gitignored, not tracked edits)
  - `resources/golden/benchy_tree_support_regression_*` ONLY with classified E3 justification
    recorded in the evidence file
- Files explicitly out of bounds: DEVIATION_LOG content edits beyond what closure rules
  require of the implementing swarm; other packets
- Expected sub-agent dispatches:
  - Question: register TASK-369..380 rows; scope: docs/07 tail; return: FACT
- Context cost: `M` (includes the gated whole-suite run IF the swarm's acceptance ceremony
  requires it: `cargo xtask test --summary --workspace --no-fail-fast`)
- Authoritative docs: plan §8 human gate; §13 T7
- OrcaSlicer refs: reference G-code comparison (human-owned artifacts under tmp/)
- Verification:
  - `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail
  - `cargo xtask check-literals` - FACT pass/fail
  - `cargo xtask build-guests --check` - FACT exit 0
  - Human gate: produce `tmp/p238b-tree-fixture.gcode` + `tmp/p238b-wedge.gcode` +
    visual-debug bundle; checklist signed in `tmp/p238b-human-validation.md`
- Exit condition: sign-off line appended OR packet stays draft pending human review (correct
  terminal state for this queue).

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | forward-dep reconciliation FACT first |
| Step 2 | M | helper rewrite + 3 call sites |
| Step 3 | M | decision record + arg-site split |
| Step 4 | M | three coupled emit-pass changes |
| Step 5 | M | keying swap + carve rule |
| Step 6 | S | two localized site fixes |
| Step 7 | M | cross-crate WIT param + blast radius |
| Step 8 | S | ctor ordering + simplify variant |
| Step 9 | S | reachability gate |
| Step 10 | M | styles + hybrid minting |
| Step 11 | M | IR/WIT transport, both legs |
| Step 12 | M | gates + artifacts + registration |

Split before activation if aggregate exceeds M or any step grows to L (none is L as authored).

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- Human Validation Gate signed (§packet.spec.md) — required before `status: implemented`.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk (largest: golden drift classification; T7 residual).
- Confirm context stayed ≤150k standard, or ≤300k only with a logged swarm ESCALATION;
  otherwise record a packet-authoring lesson.

All gate invocations of `cargo check` and `cargo clippy` MUST use `--all-targets` so the test, bench, and example targets compile. Narrow per-step AC verification commands stay exactly as written (`cargo test -p <crate> --test <file> [name] [--exact]`) — do NOT add `--all-targets` to them; repo Test Discipline keeps verification runs at the narrowest test that proves the change, and feature-gated crates keep their explicit `--features host-algos` flags instead (E6).
