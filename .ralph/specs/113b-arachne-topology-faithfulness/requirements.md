# Requirements: 113b-arachne-topology-faithfulness

## Packet Metadata

- Grouped task IDs: **none** (the M2 plan `docs/specs/perimeter-modules-orca-parity-roadmap.md` is the real provenance; no `TASK-###` exists in `docs/07_implementation_status.md` for arachne follow-ups per the packet-112 handoff). Crosswalk: M2 plan Phase 12 items T-220..T-227 + Phase 13 items T-230..T-234 are DONE; this packet re-ports T-220 (centrality), T-221 (bead_count), T-222 (propagation + transitions), T-223 (generate_toolpaths + `connectJunctions`) from the from-first-principles adaptation to the faithful OrcaSlicer port, gated on a new synthetic quad/rib topology pass.
- Backlog source: `docs/specs/perimeter-modules-orca-parity-roadmap.md` §"M2 — Real Arachne" (T-220..T-227 follow-up) + `docs/DEVIATION_LOG.md` D-112-CENTRALITY-ADAPT (line 40), D-112-PROPAGATION-ADAPT (line 42), D-112-SELFCAPTURED-BASELINES (line 48), D-112-MMU-TOPOLOGY (line 50)
- Packet status: `draft` (cannot activate until P113a is `status: implemented`; per `.ralph/specs/README.md:36` exactly one packet is `status: active` at a time, and P113a holds that slot currently)
- Aggregate context cost: `L` (largest single step is the synthetic `makeRib` pass on boostvoronoi output — genuinely L effort; the packet is gated on this step's success). **L-step exception**: the spec-packet-generator skill rule "No step may be L; if it would, split" is OVERRIDDEN for this packet at the user's explicit decision (see `packet.spec.md` §Prerequisites and Blockers). The `makeRib` algorithm is monolithic — partial rib insertion produces incorrect topology that blocks all 4 dependent passes, and there is no natural split point.

## Problem Statement

The packet 112 audit (commit `d9466fd7`) identified that the Arachne pipeline's from-first-principles adaptations cannot be made algorithm-faithful to OrcaSlicer without a synthetic quad/rib topology pass that builds the structural classification OrcaSlicer's code uses. The raw boostvoronoi graph carries no rib/spine edge classification — a vertex at a polygon corner has incident edges radiating in all directions, with no inherent boundary-relationship metadata. OrcaSlicer's `makeRib` + `EXTRA_VD` construction synthesizes this classification. Four Arachne passes depend on it: `updateIsCentral` (centrality predicate), `updateBeadCount` (per-NODE distance_to_boundary), `generateTransitionMids`/`applyTransitions` (transition_ratio-based marking), and `connectJunctions` (per-edge junction fan stitching into full ExtrusionLines). P112's adaptations work around the missing topology with depth-floor centrality, per-EDGE bead count, folded transition marking, and per-edge 2-junction fragment emission — functionally equivalent for tested fixtures but not algorithm-faithful. This packet builds the missing topology pass and re-ports the 4 dependent passes plus the 3 downstream stages (stitch, simplify, remove_small) whose input topology changes as a cascade. P113a ships the 6 independent S/M items first; this packet is the L-effort topology chain.

## In Scope

- **Synthetic quad/rib topology pass** in `crates/slicer-core/src/skeletal_trapezoidation/rib.rs` (NEW): identify corner-to-spine relationships in the raw boostvoronoi half-edge graph, insert synthetic rib edges, build the 4-vertex quadrilateral cell structure. Add `rib_twin: Option<EdgeId>` and `quad_cell: Option<QuadCellId>` fields to `STHalfEdge` in `graph.rs`. Handle boostvoronoi's degenerate zero-length edges at input-segment endpoints (collapse or bridge so the rib pass has clean input).
- **Faithful `filter_central`** in `centrality.rs`: replace depth-floor + whisker-dissolve with OrcaSlicer's `dR < dD * sin(angle/2)` predicate on quad/rib topology. The angle is between two spine edges at a spine vertex.
- **Per-NODE bead_count** in `bead_count.rs`: move `bead_count: Option<u32>` from `STHalfEdge` to the vertex type; assign at Voronoi vertices via quad cell `distance_to_boundary`. Compute `r_avg = (r_min + r_max) / 2.0` per quad cell and call `strategy.optimal_bead_count(2.0 * r_avg)`.
- **Faithful transition marking + propagation re-port** in `propagation.rs`: extract `mark_transitions` from propagation passes. Add new `generate_transition_mids` function (ported from `SkeletalTrapezoidation.cpp:925`) that runs PRE-propagation and reads `transition_ratio` from the beading strategy. Add new `apply_transitions` function (ported from `:1487`) that inserts new half-edge nodes at each `TransitionEnd` position, splitting edges in the quad graph. Re-port `propagate_beadings_upward`/`propagate_beadings_downward` to read quad-decorated graph state.
- **Faithful `connectJunctions`** in `generate_toolpaths.rs`: replace per-edge 2-junction fragment emission with a faithful port of `SkeletalTrapezoidation.cpp:2260`'s `connectJunctions` that stitches per-edge junction fans into full `ExtrusionLine`s across quad rib/non-rib chains. Output becomes multi-junction lines, some closed.
- **Re-validate `stitch_extrusions`** in `stitch.rs`: closed rings pass through untouched; open chains still join within `max_gap`. Primary preservation invariant (`is_closed && inset_idx == 0`) still holds.
- **Re-validate `simplify_toolpaths`** in `simplify.rs`: the Visvalingam-Whyatt port from P113a (A1) now actually exercises vertex removal on multi-junction lines (was a no-op on P112's 2-junction input). The `simplify_toolpaths_vertex_count` test fixture is re-baselined.
- **Re-validate `remove_small_lines`** in `remove_small.rs`: primary preservation invariant still holds. Removal patterns change (longer chains, closed rings immune). The test fixture is re-baselined.
- **Re-baseline 8 self-captured regression fixtures** in `crates/slicer-core/tests/fixtures/arachne/`: centrality 3 + bead_count 1 + propagation 3 + generate_toolpaths 1.
- **Close 2 deviations + register 1 new in `docs/DEVIATION_LOG.md`**: `D-112-CENTRALITY-ADAPT`, `D-112-PROPAGATION-ADAPT` closed; new `D-113B-CONNECTJUNCTIONS` registered and closed same-packet.

## Out of Scope

- The 6 independent S/M items (Visvalingam, config wiring, MMU test fix, loader guard, fixture dir, closure-log) — P113a
- `D-112-SELFCAPTURED-BASELINES` — accepted limitation, no OrcaSlicer binary
- `D-112-HOSTSVC-BRIDGE`, `D-112-WALL-GENERATOR-SELECT`, `D-112-TOOLPATH-WIDTH` — already closed by P112
- `D-112-SIMPLIFY-DP` and `D-112-THIN-WALL-WIDENING` (residual) — P113a
- New per-vertex IR types in `slicer-ir` — no schema bump; topology changes are within existing types
- WIT record changes — no host-service interface changes; the `arachne-params` record from P113a covers all parameter threading
- Spiral-vase + non-planar — orthogonal sibling roadmaps
- Classic-perimeters edits — M1 frozen

## Authoritative Docs

- `docs/02_ir_schemas.md` — range-read §"Point3WithWidth" only (90 lines); purpose: confirm no schema bump needed
- `docs/04_host_scheduler.md` — range-read §"Phase 3 DAG Validation" only (60 lines); purpose: confirm topology changes don't affect scheduler validation
- `docs/08_coordinate_system.md` — range-read §"Constant Conversion Table" only (30 lines); purpose: units-to-mm conversion for `transition_ratio` field
- `docs/specs/orca-mmu-perimeter-investigation.md` (from P105) — read full (35 lines); purpose: per-color Voronoi partition invariants (unaffected by topology changes, but referenced for the MMU re-validation)

All other docs are not authoritative for this packet.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidationGraph.cpp:452` — `makeRib()`: synthetic rib-edge insertion that builds the quad cell decomposition. The implementer needs the exact rib-insertion algorithm to port faithfully.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:672` — `updateIsCentral()`: the `dR < dD * sin(angle/2)` predicate that reads quad/rib topology.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:777` — `updateBeadCount()`: per-NODE bead count assignment reading `distance_to_boundary` from quad cells.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:925` — `generateTransitionMids()`: computes `TransitionMiddle` positions from `transition_ratio` (pre-propagation).
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:1487` — `applyTransitions()`: inserts new half-edge nodes at `TransitionEnd` positions.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:1800,1833` — `propagateBeadingsUpward`/`propagateBeadingsDownward()`: propagation that reads quad state.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:2260` — `connectJunctions()`: stitches per-edge junction fans into full `ExtrusionLine`s.

## Acceptance Summary

Reference Acceptance Criteria by ID; do not copy them.

- Positive cases: `AC-1` (quad/rib pass), `AC-2` (faithful centrality), `AC-3` (per-NODE bead_count), `AC-4` (faithful transitions + propagation re-port), `AC-5` (faithful `connectJunctions`), `AC-6` (stitch re-validation), `AC-7` (simplify re-validation), `AC-8` (remove_small re-validation), `AC-9` (8 re-baselined fixtures), `AC-10` (deviation closures).
- Negative cases: `AC-N1` (square has no ribs), `AC-N2` (bead_count requires centrality), `AC-N3` (remove_small primary preservation), `AC-N4` (rib pass deterministic).
- Refinements not captured in Given/When/Then:
  - The `bead_count: Option<u32>` field moves from `STHalfEdge` to the vertex type. This is a structural type change that propagates to `bead_count.rs`, `graph.rs`, and any consumer that reads `bead_count` from a half-edge. All consumers must be updated atomically.
  - The 4 dependent passes (centrality, bead_count, transitions, connectJunctions) all read the quad/rib topology from B1. If B1 produces incorrect topology, all 4 fail. B1 is the single point of failure for the entire packet.
  - boostvoronoi's degenerate zero-length edges at input-segment endpoints must be handled BEFORE the rib pass runs. OrcaSlicer's richer construction never produces these edges; boostvoronoi does. The implementer must decide whether to collapse (merge zero-length edges) or bridge (insert a synthetic midpoint) — this is a forward-dep on B1's design.
  - The 8 re-baselined fixtures are self-captured regression locks. They do NOT validate against an OrcaSlicer oracle. The re-recording is acceptable per the perimeter-parity harness convention (`D-112-SELFCAPTURED-BASELINES`); the algorithm-faithfulness criterion is asserted via direct OrcaSlicer code references (`OrcaSlicerDocumented/.../SkeletalTrapezoidationGraph.cpp:452` etc.) and code review, not output match.

## Verification Commands

Full verification matrix. `packet.spec.md` §Verification carries only the gate subset.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-core --features host-algos --test skeletal_trapezoidation -- quad_rib_topology_square_has_no_ribs 2>&1 | tee target/test-output-rib-square.log` | AC-1: quad/rib pass on square | FACT pass/fail |
| `cargo test -p slicer-core --features host-algos --test skeletal_trapezoidation -- quad_rib_topology_is_deterministic 2>&1 | tee target/test-output-rib-deterministic.log` | AC-N4: rib pass deterministic | FACT pass/fail |
| `cargo test -p slicer-core --features host-algos --test centrality -- centrality_three_fixtures 2>&1 | tee target/test-output-centrality-faithful.log` | AC-2: faithful centrality on 3 fixtures | FACT pass/fail (fixtures re-baselined) |
| `cargo test -p slicer-core --features host-algos --test bead_count -- bead_count_tapered_wedge 2>&1 | tee target/test-output-bead-faithful.log` | AC-3: per-NODE bead count | FACT pass/fail |
| `cargo test -p slicer-core --features host-algos --test bead_count -- bead_count_requires_centrality 2>&1 | tee target/test-output-bead-neg.log` | AC-N2: bead_count requires centrality | FACT pass/fail |
| `cargo test -p slicer-core --features host-algos --test propagation -- propagation_three_fixtures 2>&1 | tee target/test-output-propagation-faithful.log` | AC-4: faithful propagation on 3 fixtures | FACT pass/fail (fixtures re-baselined) |
| `cargo test -p slicer-core --features host-algos --test generate_toolpaths -- generate_toolpaths_tapered_wedge 2>&1 | tee target/test-output-toolpaths-faithful.log` | AC-5: faithful connectJunctions | FACT pass/fail (fixture re-baselined) |
| `cargo test -p slicer-core --features host-algos --test stitch -- stitch_extrusions_preserves_primary 2>&1 | tee target/test-output-stitch-faithful.log` | AC-6: stitch re-validated | FACT pass/fail |
| `cargo test -p slicer-core --features host-algos --test simplify -- simplify_toolpaths_vertex_count 2>&1 | tee target/test-output-simplify-faithful.log` | AC-7: simplify re-validated (now actually exercises VW) | FACT pass/fail (fixture re-baselined) |
| `cargo test -p slicer-core --features host-algos --test remove_small -- remove_small_lines_preserves_primary 2>&1 | tee target/test-output-remove-faithful.log` | AC-8: remove_small re-validated | FACT pass/fail |
| `cargo test -p slicer-core --features host-algos --test remove_small -- remove_small_lines_all_primary_invariant 2>&1 | tee target/test-output-remove-neg.log` | AC-N3: remove_small primary preservation | FACT pass/fail |
| `for f in centrality_square.json centrality_wedge.json centrality_multi_feature.json bead_count_tapered_wedge.json propagation_varying.json propagation_uniform.json propagation_multi_feature.json toolpaths_tapered_wedge.json; do test -f "crates/slicer-core/tests/fixtures/arachne/$f" && echo "PRESENT $f" || echo "MISSING $f"; done` | AC-9: 8 re-baselined fixtures present | FACT: all 8 PRESENT |
| `rg -q 'D-112-CENTRALITY-ADAPT.*Closed' docs/DEVIATION_LOG.md && rg -q 'D-112-PROPAGATION-ADAPT.*Closed' docs/DEVIATION_LOG.md && rg -q 'D-113B-CONNECTJUNCTIONS.*Closed' docs/DEVIATION_LOG.md` | AC-10: 3 deviation closures | FACT pass/fail (all 3 grep must succeed) |
| `cargo check --workspace --all-targets` | Cross-crate compile | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Clippy gate | FACT pass/fail |
| `cargo xtask build-guests --check` | Guest WASM coherence | FACT clean / STALE list |
| `cargo xtask test --workspace --summary 2>&1 | tee target/test-output.log` | M2 closure gate | FACT pass/fail + summary line + count |

All verification commands are delegation-friendly.

## Step Completion Expectations

Cross-step invariants the per-step blocks in `implementation-plan.md` cannot express:

- **Step 1 (quad/rib) is the single point of failure.** Steps 2-5 (centrality, bead_count, transitions, connectJunctions) all read the quad/rib topology from Step 1. If Step 1 produces incorrect topology, all 4 fail. The implementer MUST run Step 1's tests (AC-1 + AC-N1 + AC-N4) and confirm CLEAN before proceeding to Step 2.
- **boostvoronoi degeneracy handling is a forward-dep on Step 1.** boostvoronoi produces degenerate zero-length edges at input-segment endpoints that OrcaSlicer's richer construction never produces. Step 1 must handle these BEFORE the rib pass runs. The implementer must decide: collapse (merge zero-length edges) or bridge (insert a synthetic midpoint). This decision is made in Step 1's implementation, not deferred.
- **Step 6 (re-baseline 8 fixtures) is atomic.** Once Step 5 (faithful `connectJunctions`) lands, the 8 fixtures re-baseline in one batch. The implementer MUST record the re-baselining rationale in each fixture's own commit message (e.g., "fixture re-baselined for faithful quad/rib topology + per-NODE bead count").
- **Step 7 (workspace gate) is the final gate.** The implementer MUST run `cargo xtask test --workspace --summary` and confirm green before flipping `packet.spec.md` to `status: implemented`.
- **No ADR-0033 dependency.** The original packet draft listed "ADR-0033 (Algorithm Faithfulness as OrcaSlicer Parity Definition)" as a P113b dependency. That ADR does not exist in `docs/adr/` and the user has not asked for it. The acceptance criteria assert algorithm fidelity via OrcaSlicer code references (`OrcaSlicerDocumented/.../SkeletalTrapezoidationGraph.cpp:452` etc.) — code references are sufficient on their own; a formal ADR is not required for this packet.

## Context Discipline Notes

Packet-specific context hazards:

- `crates/slicer-core/src/skeletal_trapezoidation/centrality.rs` (354 LOC) is a primary edit target (Step 2). Can be full-read; it is the primary edit target.
- `crates/slicer-core/src/skeletal_trapezoidation/bead_count.rs` (113 LOC) is a primary edit target (Step 3). Can be full-read.
- `crates/slicer-core/src/skeletal_trapezoidation/propagation.rs` (274 LOC) is a primary edit target (Step 4). Can be full-read.
- `crates/slicer-core/src/skeletal_trapezoidation/graph.rs` (~200 LOC) is the graph type that carries the new `rib_twin` and `quad_cell` fields (Step 1). Can be full-read.
- `crates/slicer-core/src/arachne/generate_toolpaths.rs` (391 LOC) is a primary edit target (Step 5). Can be full-read.
- The OrcaSlicer `makeRib` algorithm at `SkeletalTrapezoidationGraph.cpp:452` MUST be obtained via SUMMARY dispatch — the implementer MUST NOT read the OrcaSlicer source directly. The dispatch should return the rib-insertion algorithm's data structures (what constitutes a "rib" vs "spine" edge), the algorithm's loop body, and the quad cell construction rules.
- The OrcaSlicer `updateIsCentral` predicate at `SkeletalTrapezoidation.cpp:672` MUST be obtained via SUMMARY dispatch. The dispatch should return the predicate's input (r_min, r_max, angle), the threshold formula, and the recursive dissolve loop.
- Tempting reads to skip: `modules/core-modules/arachne-perimeters/src/lib.rs` (not edited by this packet), `crates/slicer-sdk/src/host.rs` (no changes), `crates/slicer-schema/wit/deps/common.wit` (no changes — the `arachne-params` record from P113a covers all parameter threading).
- The 4 OrcaSlicer dispatches (makeRib, updateIsCentral, generateTransitionMids, connectJunctions) are the heaviest dispatches in the packet. Each should return SUMMARY (≤ 200 words, algorithm description, input/output types, edge cases) and the implementer should consult all 4 before designing the rib.rs module to ensure the quad cell structure supports all 4 downstream passes.

If none apply, write `None packet-specific.`
