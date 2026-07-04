---
status: draft
packet: 113b-arachne-topology-faithfulness
task_ids: []
backlog_source: docs/specs/perimeter-modules-orca-parity-roadmap.md (M2 follow-up)
context_cost_estimate: L
---

# Packet Contract: 113b-arachne-topology-faithfulness

## Goal

Replace P112's from-first-principles adaptations in the Arachne pipeline with algorithm-faithful OrcaSlicer ports, gated on a synthetic quad/rib topology pass that builds the structural classification (rib edge vs spine edge) that four OrcaSlicer passes depend on: centrality filtering, per-NODE bead count, transition marking, and junction stitching. Close `D-112-CENTRALITY-ADAPT`, `D-112-PROPAGATION-ADAPT`, and the unregistered `connectJunctions` adaptation. Re-validate downstream stages (stitch, simplify, remove_small) against the topology-changed input shape.

## Scope Boundaries

This packet owns the synthetic quad/rib topology pass (L effort — see Context Cost Estimate for the explicit L-step exception) and the 4 passes that depend on it (centrality, bead_count, transitions + propagation, connectJunctions) plus the 3 downstream re-validations (stitch, simplify, remove_small) that change behavior when the input topology changes. The 6 independent S/M items (Visvalingam, config wiring, MMU test fix, loader guard, fixture dir, closure-log) belong to P113a and ship first. `D-112-SELFCAPTURED-BASELINES` stays as an accepted limitation (no OrcaSlicer binary).

## Prerequisites and Blockers

- Depends on: P113a (must ship first; provides Visvalingam code-ready, config wiring, and MMU test fix). Also depends on P112 (`d9466fd7`) for the existing Arachne pipeline source.
- Unblocks: the M2 closure ceremony (P112's T-234) — after this packet ships and the per-packet narrow tests are green, the M2 closure can be flipped to fully-faithful.
- Activation blockers: P113a must be `status: implemented` before this packet activates. This packet's own `status` is `draft` for that reason — it cannot activate while P113a is `active` (per `.ralph/specs/README.md:36`, exactly one packet is `status: active` at a time).
- **L-step exception:** the spec-packet-generator skill rule "No step may be L; if it would, split" is OVERRIDDEN for this packet at the user's explicit decision. The synthetic `makeRib` pass on boostvoronoi is a genuinely L-effort construction: it does not have a natural split point (the algorithm is monolithic — partial rib insertion produces incorrect topology that blocks all 4 dependent passes), and OrcaSlicer's `vd_t` Voronoi construction is richer than `boostvoronoi` so a from-first-principles port is non-trivial. The exception is documented in `design.md` §Context Cost Estimate and `implementation-plan.md` §Per-Step Budget Roll-Up. If subsequent work surfaces a natural split point, the packet SHOULD be split before activation.

## Acceptance Criteria

- **AC-1.** A synthetic quad/rib topology pass in `crates/slicer-core/src/skeletal_trapezoidation/rib.rs` (NEW) inserts rib edges connecting polygon corners to the medial axis and builds the 4-vertex quadrilateral cell structure that OrcaSlicer's `makeRib` produces. `STHalfEdge` in `crates/slicer-core/src/skeletal_trapezoidation/graph.rs` carries `rib_twin: Option<EdgeId>` and `quad_cell: Option<QuadCellId>` fields. The pass runs after `SkeletalTrapezoidationGraph::from_polygons` and before `filter_central`. | `cargo test -p slicer-core --features host-algos --test skeletal_trapezoidation -- quad_rib_topology_square_has_no_ribs 2>&1 | tee target/test-output-rib-square.log`
- **AC-2.** `filter_central` in `crates/slicer-core/src/skeletal_trapezoidation/centrality.rs` uses OrcaSlicer's `dR < dD * sin(angle/2)` predicate on quad/rib topology, replacing the depth-floor + whisker-dissolve adaptation. The angle is between two spine edges at a spine vertex. | `cargo test -p slicer-core --features host-algos --test centrality -- centrality_three_fixtures 2>&1 | tee target/test-output-centrality-faithful.log`
- **AC-3.** `assign_bead_counts` in `crates/slicer-core/src/skeletal_trapezoidation/bead_count.rs` assigns counts at Voronoi vertices (nodes) via quad cell `distance_to_boundary`, replacing per-EDGE assignment. The `bead_count: Option<u32>` field moves from `STHalfEdge` to the vertex type. | `cargo test -p slicer-core --features host-algos --test bead_count -- bead_count_tapered_wedge 2>&1 | tee target/test-output-bead-faithful.log`
- **AC-4.** `propagate_beadings_upward` and `propagate_beadings_downward` in `crates/slicer-core/src/skeletal_trapezoidation/propagation.rs` are re-ported to read quad-decorated graph state. A new `generate_transition_mids` function (ported from OrcaSlicer's `SkeletalTrapezoidation.cpp:925`) runs PRE-propagation and reads `transition_ratio` from the beading strategy. A new `apply_transitions` function (ported from `SkeletalTrapezoidation.cpp:1487`) inserts new half-edge nodes at each `TransitionEnd` position, splitting edges in the quad graph. The `mark_transitions` function's folded-in logic is removed. | `cargo test -p slicer-core --features host-algos --test propagation -- propagation_three_fixtures 2>&1 | tee target/test-output-propagation-faithful.log`
- **AC-5.** `generate_toolpaths` in `crates/slicer-core/src/arachne/generate_toolpaths.rs` replaces the per-edge 2-junction fragment emission with a faithful `connectJunctions` pass (ported from `SkeletalTrapezoidation.cpp:2260`) that stitches per-edge junction fans into full `ExtrusionLine`s across quad rib/non-rib chains, producing `VariableWidthLines` (multi-junction lines, some closed). | `cargo test -p slicer-core --features host-algos --test generate_toolpaths -- generate_toolpaths_tapered_wedge 2>&1 | tee target/test-output-toolpaths-faithful.log`
- **AC-6.** `stitch_extrusions` in `crates/slicer-core/src/arachne/stitch.rs` is re-validated against the multi-junction input from AC-5. Closed rings pass through untouched; open chains still join within `max_gap`. Primary preservation invariant (`is_closed && inset_idx == 0`) still holds. | `cargo test -p slicer-core --features host-algos --test stitch -- stitch_extrusions_preserves_primary 2>&1 | tee target/test-output-stitch-faithful.log`
- **AC-7.** `simplify_toolpaths` in `crates/slicer-core/src/arachne/simplify.rs` is re-validated against the multi-junction input from AC-5. The Visvalingam-Whyatt port from P113a (A1) now actually exercises vertex removal on the new multi-junction lines. Vertex counts in the `simplify_toolpaths_vertex_count` test decrease (re-baselined to the faithful algorithm's output). | `cargo test -p slicer-core --features host-algos --test simplify -- simplify_toolpaths_vertex_count 2>&1 | tee target/test-output-simplify-faithful.log`
- **AC-8.** `remove_small_lines` in `crates/slicer-core/src/arachne/remove_small.rs` is re-validated against the multi-junction input from AC-5. Primary preservation invariant still holds. Removal patterns change (longer chains, closed rings immune) — the test fixture is re-baselined. | `cargo test -p slicer-core --features host-algos --test remove_small -- remove_small_lines_preserves_primary 2>&1 | tee target/test-output-remove-faithful.log`
- **AC-9.** All 9 self-captured regression fixtures affected by the topology chain (centrality 3, bead_count 1, propagation 3, generate_toolpaths 1) are re-baselined against the faithful algorithm's output. Each fixture is committed; each test passes with the re-baselined golden. | `for f in centrality_square.json centrality_wedge.json centrality_multi_feature.json bead_count_tapered_wedge.json propagation_varying.json propagation_uniform.json propagation_multi_feature.json toolpaths_tapered_wedge.json; do test -f "crates/slicer-core/tests/fixtures/arachne/$f" && echo "PRESENT $f" || echo "MISSING $f"; done`
- **AC-10.** `docs/DEVIATION_LOG.md` closes `D-112-CENTRALITY-ADAPT` and `D-112-PROPAGATION-ADAPT` with Status "Closed — 2026-07-03: quad/rib topology + faithful port landed". The unregistered `connectJunctions` adaptation is recorded as `D-113B-CONNECTJUNCTIONS` (new deviation, closed same packet). `D-112-MMU-TOPOLOGY` STAYS OPEN across this packet (the "tens of mm outside the naive per-face footprint" symptom is re-verified with the faithful `connectJunctions` output, and either the symptom is gone — closing the deviation — or it persists with new evidence — re-targeting the follow-up). The packet documents the result in the deviation's "Verification" column. | `rg -q 'D-112-CENTRALITY-ADAPT.*Closed' docs/DEVIATION_LOG.md && rg -q 'D-112-PROPAGATION-ADAPT.*Closed' docs/DEVIATION_LOG.md && rg -q 'D-113B-CONNECTJUNCTIONS.*Closed' docs/DEVIATION_LOG.md`

## Negative Test Cases

- **AC-N1.** A square input has no ribs: the quad/rib pass does not insert any rib edges because a square has no sharp corners that generate ribs. Every edge is a spine edge. | `cargo test -p slicer-core --features host-algos --test skeletal_trapezoidation -- quad_rib_topology_square_has_no_ribs 2>&1 | tee target/test-output-rib-square-neg.log`
- **AC-N2.** `assign_bead_counts` returns `Err(BeadCountError::CentralityNotRun)` when centrality has not been run, preserving the AC-N1 invariant from P112. | `cargo test -p slicer-core --features host-algos --test bead_count -- bead_count_requires_centrality 2>&1 | tee target/test-output-bead-neg.log`
- **AC-N3.** `remove_small_lines` does NOT remove any `ExtrusionLine` where `is_closed == true && inset_idx == 0`, regardless of length — invariant preserved across the topology change. | `cargo test -p slicer-core --features host-algos --test remove_small -- remove_small_lines_all_primary_invariant 2>&1 | tee target/test-output-remove-neg.log`
- **AC-N4.** The quad/rib pass is deterministic: two runs on the same input produce identical graph structure (rib edges + quad cells). | `cargo test -p slicer-core --features host-algos --test skeletal_trapezoidation -- quad_rib_topology_is_deterministic 2>&1 | tee target/test-output-rib-deterministic.log`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask build-guests --check` (CLEAN — this packet edits `slicer-core/src/skeletal_trapezoidation/*.rs` which feeds the host-side pipeline; no WIT changes)
- `cargo xtask test --workspace --summary` (final closure gate — per CLAUDE.md §"Test Discipline" workspace-test exception)

## Authoritative Docs

- `docs/02_ir_schemas.md` — no schema bump needed; topology changes are within existing types
- `docs/04_host_scheduler.md` — read §"Phase 3 DAG Validation" only (dependency validation, not affected)
- `docs/08_coordinate_system.md` — range-read §"Constant Conversion Table" only (unit conversion for `transition_ratio`)
- `docs/specs/orca-mmu-perimeter-investigation.md` — read full (35 lines; per-color partition invariants)

For each doc, the implementer should range-read or delegate. Do not load any doc > 300 lines in full.

## Doc Impact Statement (Required)

- `docs/DEVIATION_LOG.md` — update `D-112-CENTRALITY-ADAPT` Status to "Closed — 2026-07-03: quad/rib topology + faithful `updateIsCentral`/`filterCentral` port landed; per-NODE bead count via quad cell `distance_to_boundary`" — `rg -q 'D-112-CENTRALITY-ADAPT.*Closed' docs/DEVIATION_LOG.md`
- `docs/DEVIATION_LOG.md` — update `D-112-PROPAGATION-ADAPT` Status to "Closed — 2026-07-03: faithful `generateTransitionMids`/`applyTransitions` port landed; propagation re-ported to read quad graph state" — `rg -q 'D-112-PROPAGATION-ADAPT.*Closed' docs/DEVIATION_LOG.md`
- `docs/DEVIATION_LOG.md` — add new `D-113B-CONNECTJUNCTIONS` entry recording the unregistered `connectJunctions` adaptation that was replaced; Status "Closed — 2026-07-03: faithful `connectJunctions` port landed; per-edge 2-junction fragment emission replaced with full `ExtrusionLine` stitching" — `rg -q 'D-113B-CONNECTJUNCTIONS.*Closed' docs/DEVIATION_LOG.md`
- `docs/DEVIATION_LOG.md` — update `D-112-MMU-TOPOLOGY` Status based on Step 6's re-verification result: if the "tens of mm outside" symptom is gone with the faithful `connectJunctions` output, close the deviation; if the symptom persists, re-target the follow-up to the new evidence and stay open. Either outcome is acceptable.
- `docs/01_system_architecture.md` — update §"Perimeter Modules — OrcaSlicer Parity Roadmap" to mark the M2 topology chain as complete (P110/P111/P112/P113a/P113b all green) — `rg -q 'M2.*complete.*P110.*P111.*P112.*P113a.*P113b' docs/01_system_architecture.md`
- `docs/specs/perimeter-modules-orca-parity-roadmap.md` — T-220..T-234 are already DONE (P112); add a new section noting P113a + P113b as the "M2-faithful" follow-up (P113a = 6 S/M items; P113b = L topology chain). Do NOT re-mark T-220..T-234 — they remain DONE per P112. — `rg -q 'P113a.*complete\|P113b.*complete' docs/specs/perimeter-modules-orca-parity-roadmap.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidationGraph.cpp:452` — `makeRib()`: synthetic rib-edge insertion that builds the quad cell decomposition. The implementer needs the exact rib-insertion algorithm to port faithfully.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:672` — `updateIsCentral()`: the `dR < dD * sin(angle/2)` predicate that reads quad/rib topology.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:777` — `updateBeadCount()`: per-NODE bead count assignment reading `distance_to_boundary` from quad cells.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:925` — `generateTransitionMids()`: computes `TransitionMiddle` positions from `transition_ratio` (pre-propagation).
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:1487` — `applyTransitions()`: inserts new half-edge nodes at `TransitionEnd` positions, splitting edges in the quad graph.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:1800` — `propagateBeadingsUpward()` and `:1833` — `propagateBeadingsDownward()`: propagation re-ported to read quad state.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:2260` — `connectJunctions()`: stitches per-edge junction fans into full `ExtrusionLine`s across quad rib/non-rib chains.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
