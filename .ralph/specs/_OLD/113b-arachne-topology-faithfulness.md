---
status: implemented
packet: 113b-arachne-topology-faithfulness
task_ids: []
---

# 113b-arachne-topology-faithfulness

## Goal

Replace P112's from-first-principles adaptations in the Arachne pipeline with algorithm-faithful OrcaSlicer ports, gated on a synthetic quad/rib topology pass that builds the structural classification (rib edge vs spine edge) that four OrcaSlicer passes depend on: centrality filtering, per-NODE bead count, transition marking, and junction stitching. Close `D-112-CENTRALITY-ADAPT`, `D-112-PROPAGATION-ADAPT`, and the unregistered `connectJunctions` adaptation. Re-validate downstream stages (stitch, simplify, remove_small) against the topology-changed input shape.

## Problem Statement

The packet 112 audit (commit `d9466fd7`) identified that the Arachne pipeline's from-first-principles adaptations cannot be made algorithm-faithful to OrcaSlicer without a synthetic quad/rib topology pass that builds the structural classification OrcaSlicer's code uses. The raw boostvoronoi graph carries no rib/spine edge classification — a vertex at a polygon corner has incident edges radiating in all directions, with no inherent boundary-relationship metadata. OrcaSlicer's `makeRib` + `EXTRA_VD` construction synthesizes this classification. Four Arachne passes depend on it: `updateIsCentral` (centrality predicate), `updateBeadCount` (per-NODE distance_to_boundary), `generateTransitionMids`/`applyTransitions` (transition_ratio-based marking), and `connectJunctions` (per-edge junction fan stitching into full ExtrusionLines). P112's adaptations work around the missing topology with depth-floor centrality, per-EDGE bead count, folded transition marking, and per-edge 2-junction fragment emission — functionally equivalent for tested fixtures but not algorithm-faithful. This packet builds the missing topology pass and re-ports the 4 dependent passes plus the 3 downstream stages (stitch, simplify, remove_small) whose input topology changes as a cascade. P113a ships the 6 independent S/M items first; this packet is the L-effort topology chain.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it. Note: this packet's change surface is entirely host-side (slicer-core); no WIT or module edits, so guest staleness is not expected. The freshness check is run as a precaution.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- **Single point of failure:** Step 1 (quad/rib pass) is the only structural dependency for Steps 2-5. If Step 1 produces incorrect topology, all 4 dependent passes fail. The implementer MUST run Step 1's tests (AC-1 + AC-N1 + AC-N4) and confirm CLEAN before proceeding to Step 2.

- **boostvoronoi degeneracy handling:** boostvoronoi produces degenerate zero-length edges at input-segment endpoints that OrcaSlicer's richer construction never produces. Step 1 must handle these BEFORE the rib pass runs. Decision: collapse (merge zero-length edges) or bridge (insert synthetic midpoint). This is a forward-dep on Step 1's design.

- **Type structure change:** `bead_count: Option<u32>` moves from `STHalfEdge` to the vertex type. All consumers (bead_count.rs, graph.rs, propagation.rs, any test) must be updated atomically.

- **No schema bump:** Topology changes are within existing IR types. `CURRENT_SLICE_IR_SCHEMA_VERSION` stays at 4.7.0 (P112's bump).

## Data and Contract Notes

- **Graph type change:** `STHalfEdge` gains `rib_twin: Option<EdgeId>` and `quad_cell: Option<QuadCellId>`; loses `bead_count: Option<u32>`. A new vertex type gains `bead_count: Option<u32>`. The change is structural and propagates to all consumers.
- **Pipeline call order change:** `run_arachne_pipeline` now: `preprocess → voronoi → from_polygons → rib::build_quad_rib_topology → filter_central → assign_bead_counts (per-NODE) → generate_transition_mids → propagate_beadings_upward → propagate_beadings_downward → apply_transitions → generate_toolpaths (with connectJunctions) → stitch → simplify → remove_small`.
- **No IR schema bump:** `ExtrusionLine`/`ExtrusionJunction` from P112 are unchanged. `bead_count` field moves between `STHalfEdge` and vertex type within the internal `skeletal_trapezoidation` module, not in `slicer-ir`.
- **No WIT changes:** the `arachne-params` record from P113a already covers all parameter threading. This packet does not add or rename WIT fields.
- **Determinism:** The rib pass must be deterministic (AC-N4). Two runs on the same input must produce identical rib edges + quad cells.

## Locked Assumptions and Invariants

- The `STHalfEdge` rib/spine classification is a faithful port of OrcaSlicer's `makeRib` — verified by SUMMARY dispatch against `SkeletalTrapezoidationGraph.cpp:452` and code review.
- The `filter_central` predicate `dR < dD * sin(angle/2)` is computed between two SPINE edges at a SPINE vertex. The angle is measured on the quad-decorated graph, not the raw boostvoronoi graph.
- The `bead_count: Option<u32>` field moves atomically from `STHalfEdge` to the vertex type. No intermediate state where the field exists in both.
- The 8 re-baselined fixtures are self-captured regression locks. They are accepted as such per the perimeter-parity harness convention established by `D-112-SELFCAPTURED-BASELINES` (same root cause as `D-109-SELF-CAPTURED-FIXTURES` for M1). They do NOT validate against an OrcaSlicer oracle. The re-recording is acceptable because the algorithm-faithfulness criterion is asserted via direct OrcaSlicer code references (`OrcaSlicerDocumented/.../SkeletalTrapezoidationGraph.cpp:452` etc.) — code review, not output match.
- `remove_small_lines` primary preservation invariant (`is_closed && inset_idx == 0` lines never removed) is preserved across the topology change. The re-validation test (AC-8 + AC-N3) proves this.

## Risks and Tradeoffs

- **Single point of failure (Step 1 quad/rib):** If the rib pass produces incorrect topology, all 4 dependent passes fail. boostvoronoi's degenerate zero-length edges at input-segment endpoints are a known hazard — OrcaSlicer's richer construction never produces them. The implementer must decide how to handle them. Mitigation: AC-N1 (square has no ribs) + AC-N4 (deterministic) are the cheapest early failure checks; if they fail, the rib pass is broken and Steps 2-5 cannot proceed.
- **ConnectJunctions cascade:** Faithful `connectJunctions` changes the input shape to stitch/simplify/remove_small from per-edge 2-junction fragments to multi-junction lines. Simplify goes from no-op to active (it was a no-op on the per-edge 2-junction input). The Visvalingam port from P113a now actually exercises vertex removal. Mitigation: AC-7 re-baselines the simplify fixture; the cascade is expected and documented.
- **Fixture re-baselining as regression-locks:** Re-baselining goldens locks in output from an algorithm we ported but can't validate against the reference (no OrcaSlicer binary). If a subtle port bug exists, the re-baselined golden locks it in. Mitigation: the algorithm-faithfulness criterion is asserted via direct OrcaSlicer code references (`OrcaSlicerDocumented/.../SkeletalTrapezoidationGraph.cpp:452` etc.) and code review, not output match. Each re-baselined fixture is committed with a commit message explaining what changed.
- **`transition_ratio` field on quad graph:** OrcaSlicer's `transition_ratio` is a fractional field on the quad-decorated graph that this codebase does not currently carry. The implementer must add it to the graph type and compute it from the beading strategy. This is a new field, not a renumbered existing field.
- **Topological edge case handling:** A square input has no sharp corners that generate ribs (AC-N1). A wedge has one corner. A multi-feature polygon has many. The rib pass must handle all three without producing malformed quad cells.
