# Requirements: 238b-tree-planner-canonical-fidelity

## Packet Metadata

- Grouped task IDs: `TASK-369`..`TASK-380`
- Backlog source: `docs/specs/support-families-anchored-entities-plan.md` §12 row #4
- Packet status: `draft`
- Aggregate context cost: `M` (per-step roll-up in `implementation-plan.md`; no step rated L)

## Problem Statement

The tree planner (`modules/core-modules/tree-support-planner/src/lib.rs`, ~5.9k lines, port
of canonical `TreeSupport.cpp`) reached packet-224 parity on its main paths, but fourteen
recorded divergences remain open: orca-divergences rows 1.1, 2.1, 2.2, 2.3 (superseded in
part by the 224 carve work — residual is role coexistence + simplify gating), 3.2, 3.3,
4.1, 4.2, 4.3, 4.4/4.5, 4.6/5.6, 5.1, 5.5, 5.7, 7.1, 7.2, 8.1, plus DEV-141 (smoothing kernel
premise), DEV-142 (emit simplify gating), DEV-143 (f64 vs truncating integer smoothing),
DEV-144 (per-node extra-wall transport). Each is small alone; together they are one coherent
slice because they all land inside one planner module and share its guest-WASM test surface.
238a declared the keys (`support_style`, `max_bridge_length`); this packet makes their
behaviors exist (Ruling 3) and sizes DEV-128 for Ruling 4.

Canonical evidence below was pre-verified against the local checkout on 2026-08-22 by a
delegated probe (Q1-Q10); where it contradicts the plan brief, it wins and the contradiction
is recorded as a plan correction (`design.md` §Plan Corrections): DEV-141's premise names a
`clip_narrow_corner` kernel that does not exist inside this checkout's `smooth_nodes`, and
canonical `move_out_expolys` never restores `from0`.

## In Scope

Full scope = every divergence with an explicit disposition. "Canonical" = match the observed
checkout behavior cited in `design.md`.

| Item | Divergence / DEV | Disposition |
| --- | --- | --- |
| Top-Z gap | div 1.1 | Canonical layer-count mechanism ALREADY landed (F-34: `round_up_divide+1`, virtual gap node verified live at `plan_for_object`). This packet ADDS the variable-layer-height regression test pinning it and deletes any residual mm-walk comment debt. AC-1. |
| Smoothing reinstatement | div 2.1, DEV-141, DEV-143 | DECISION POINT (`design.md` §The Smoothing Decision Point). Reinstate node-graph `smooth_nodes` before emit gates OR record reasoned deviation. Either way: resolve DEV-141 against the OBSERVED kernel (no `clip_narrow_corner` in `smooth_nodes` in this checkout — the plan's premise is corrected), and record the f64-vs-truncation arithmetic choice (DEV-143). Negative AC-N1 binds both branches. AC-2. |
| Role coexistence | div 2.2 residual | Replace whole-layer clearing semantics with per-node classification + disjoint diff (verified live: current `build_roles` carves per-role and keeps body remainder; pin with a coexistence regression test). AC-3. |
| Circle fidelity | div 2.3 | Keep per-node fine circles (100-gon class) out of the union; retire the `BRANCH_CIRCLE_SEGMENTS` 16-vertex contour cap on emitted role contours; keep coarse-4 only under square support. AC-4. |
| Collision keying | div 4.2 | Radius-baked volumes + point-in tests on ALL production gates; retire production use of `body_overlaps_occupancy` disc inflation (test-only retention allowed). AC-5. |
| Avoidance keying | div 4.1 | Already per-node-bucketed (verified live: `get_avoidance(next_radius, …)` at branch-A and move-pass sites). Pin with regression test; remove stale constant-radius comments. AC-5. |
| Largest-part carve | div 7.1 | Carve keeps ONLY the largest-area surviving part of each drawn region after collision difference (canonical `avoid_object_remove_extra_small_parts`). AC-5. |
| Miter limits | div 3.3/4.3 | Miter limit 3.0 on TreeVolumes collision/avoidance offsets and `sample_contact_points` erosion; add miter-limit parameter to host offset path (singular + batch; WIT `offset-request` gains an optional field — additive). AC-6. |
| TreeVolumes ctor | div 4.4/4.5 | Simplify outlines at `radius_sample_resolution` in `TreeVolumes::new` before building `layer_outlines_below`; switch to a union-including simplify (canonical `ExPolygon::simplify` shape). AC-7. |
| to_buildplate inflation | div 4.6/5.6 | Contact-seeding + branch-A sites test inflated collision `get_collision(0,l)`; F-14 per-descendant move-pass recompute KEEPS raw outlines (exception preserved). AC-8. |
| move_out_expolys | div 5.1 | Dilated-ring projection, pt_max clamp, bool return; correct the false `from0` comment (PLAN CORRECTION #2). AC-9. |
| STUDIO-4252 retry args | div 7.2 | Fallback site passes `max_move_between_samples` as BOTH args; other sites keep dilation=`radius_sample_resolution+EPSILON` with own budget (both encoded distinctly). AC-9. |
| Mesh-path shim | div 3.2 | Consume host-computed overhang polygons where the contract allows; otherwise record the precise boundary. AC-10. |
| Branch-A roof counter | div 5.5 | Parent-inherited counter minus decrement replaces `max(id,nid)` merge. AC-11. |
| Tree styles | div 5.7, Ruling 3 | Implement `is_strong` movement rule + hybrid `TreeNodeType::Polygon` minting/merge/move, keyed on 238a's `support_style`. AC-12, negative AC-N2. |
| Emit simplify gating | div 8.1, DEV-142 | Gate simplify to base-areas + square-support + `line_width/2`; correct the in-tree comment. AC-13. |
| Extra-wall transport | DEV-144 | Per-node flag through `SupportPlanIR` (+ WIT additive field, schema minor bump derived-at-activation, 237 precedent); capability string demoted to provenance note. AC-14. |
| DEV-128 sizing | Ruling 4 | Sized in `design.md` §DEV-128 Sizing; implemented here only if sized S; otherwise waiver recorded. No AC — outcome recorded. |

## Out of Scope

- Renderer flow/density/interface semantics, hollow-wall rendering, radius caps (G-12),
  roof/floor band counts (G-18), base-interface role (F-37 piece 2), DEV-129/145/146 — 238c.
- AGG rasterizer (G-07) — 241. Raft geometry and raft keys — 240.
- Config-key DECLARATIONS (`support_style`, `max_bridge_length`,
  `support_branch_merge_distance_mm`, `support_max_branches_per_layer`) — done by 238a;
  this packet consumes them.
- `docs/DEVIATION_LOG.md` edits: closing DEV-141..144 rows is implementation-time work;
  dispositions are recorded HERE (requirements + design) and the log is updated when the
  implementing swarm closes them. Same for the gap register: §11 absorbs no G-rows for this
  packet and no stub consumes to 238b.
- DEV-128 implementation beyond the sizing decision (unless sized S).
- Exact Orca toolpath identity (plan §15): behavioral parity is the bar.
- G-14/G-15 pre-existing noise/debt (T10).

## Authoritative Docs

- `docs/specs/support-families-anchored-entities-plan.md` - ~755 lines; §3/§6/§7/§8/§12/§13
  ranged reads only (done at authoring)
- `docs/spec_packets/224-support-family-orca-closure/handoffs/orca-divergences.md` - 339
  lines; divergence rows ranged read (done at authoring)
- `docs/02_ir_schemas.md` - SupportPlanIR section; ranged read at implementation time
- `docs/08_coordinate_system.md` - unit checklist; ranged read

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` — `generate_contact_points`,
  `smooth_nodes`, `draw_circles`, `drop_nodes`, `move_nodes`, `move_out_expolys`,
  `avoid_object_remove_extra_small_parts`, `insert_dropped_node`: the canonical behaviors
  itemized in the scope table and in `packet.spec.md` §OrcaSlicer Reference Obligations.
  Evidence Q1-Q10 was captured from these functions on 2026-08-22; re-delegate ONLY if an
  implementation step finds the recorded evidence insufficient for the edit at hand.
- `OrcaSlicerDocumented/src/libslic3r/ClipperUtils.hpp` — offset defaults
  (`jtMiter`, `DefaultMiterLimit = 3.0`) and the SUPPORT_SURFACES override (not used on the
  tree path).
- `OrcaSlicerDocumented/src/libslic3r/ExPolygon.cpp` — `ExPolygon::simplify(tolerance)`
  composition (`union_ex(simplify_p(tolerance))`).

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1`..`AC-14` (top-Z gap pinning; smoothing decision either-branch; role
  coexistence; circle fidelity; collision/avoidance/largest-part keying; miter limits;
  TreeVolumes ctor; to_buildplate split incl. F-14 exception; move_out_expolys + retry args;
  shim boundary; branch-A counter; styles; simplify gating; extra-wall transport).
- Negative: `AC-N1` (final-geometry validation under EITHER smoothing branch),
  `AC-N2` (unsupported style value rejected/defaulted per 238a declaration),
  `AC-N3` (guest freshness gate exit 0 before attribution).
- Cross-packet impact: consumes 238a declarations (forward dep); unblocks 238c renderer
  consumption of roles/extra-wall transport; 242 closure requires every disposition above
  landed or waived.

## Verification Commands

Authoritative full matrix; `packet.spec.md` lists only the gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `mkdir -p target && cargo test -p tree-support-planner --test smooth_nodes_tdd -- --no-fail-fast 2>&1 \| tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-2/N1 smoothing branch | FACT pass/fail; ≤20 failure lines |
| `cargo test -p tree-support-planner --test tree_family_tdd -- --no-fail-fast` (tee'd) | AC-3 coexistence + roles | FACT pass/fail |
| `cargo test -p tree-support-planner --test multi_neighbour_mst_tdd -- --no-fail-fast` (tee'd) | AC-4/AC-11 circles + branch-A | FACT pass/fail |
| `cargo test -p tree-support-planner --test wall_clearance_tdd -- --no-fail-fast` (tee'd) | AC-5/N1 keying + final-geometry rejection | FACT pass/fail |
| `cargo test -p tree-support-planner --test to_buildplate_tdd -- --no-fail-fast` (tee'd) | AC-8 inflation split | FACT pass/fail |
| `cargo test -p tree-support-planner --lib -- --no-fail-fast` (tee'd) | AC-7/AC-9/AC-13 unit-level (move_out_expolys, build_roles, expolygons_simplify) | FACT pass/fail |
| `rg -q 'wall-counts' crates/slicer-schema/wit/deps/prepass-support-geometry/prepass-support-geometry.wit && rg -q 'wall_counts' crates/slicer-ir/src/slice_ir.rs && rg -q 'wall_counts' crates/slicer-wasm-host/src/marshal/in_.rs && rg -q 'wall_counts' crates/slicer-wasm-host/src/marshal/native.rs` | AC-14 DEV-144 carrier present in WIT + IR + BOTH marshal legs (`wall-counts: list<u32>` in `record support-plan-skeleton`, `wall_counts` in IR and both legs) | FACT pass/fail |
| `cargo test -p slicer-scheduler --test scheduler_integration rejects_unknown_support_style_value --exact` (tee'd) | AC-N2 style knob negative | FACT pass/fail |
| `cargo check --workspace --all-targets` then `cargo clippy --workspace --all-targets -- -D warnings` | compile + lint gate | FACT pass/fail |
| `cargo xtask build-guests --check` | AC-N3 guest freshness (exit codes 0/1/3) | FACT exit code |
| WIT-touching steps: `cargo build --tests` then `cargo xtask build-guests` | rebuild guests after schema/WIT edits | FACT pass/fail |

Every command obeys invariant 16 (non-zero matched tests asserted in the same run).

## Step Completion Expectations

- Guest-affecting steps end with `cargo xtask build-guests --check` exit 0 BEFORE their
  narrow suite is trusted (T4).
- WIT/schema steps run `cargo build --tests` immediately after the WIT edit and rebuild
  guests in the SAME step (never deferred to ceremony).
- The smoothing decision (design point) must be RESOLVED IN WRITING in `design.md`
  §The Smoothing Decision Point before Step 3's implementation lands; the negative AC-N1
  must pass under whichever branch was chosen.
- Red-first discipline: every behavioral step's new test fails for the right reason before
  the fix (E1: no vacuous assertions; assert geometry, not artifact existence).

## Context Discipline Notes

- `modules/core-modules/tree-support-planner/src/lib.rs` is ~5.9k lines: NEVER full-load;
  every step cites symbol ranges and reads ±40-line windows around them.
- `OrcaSlicerDocumented/**` delegated always (T1: gitignored — glob misses it).
- `tmp/` artifacts are disposable; regenerate before relying on them.
- Do not re-run tests to "see more output" — read `target/test-output.log`.
