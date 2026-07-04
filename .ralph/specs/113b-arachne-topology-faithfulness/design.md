# Design: 113b-arachne-topology-faithfulness

## Controlling Code Paths

- **Primary code path 1:** `crates/slicer-core/src/skeletal_trapezoidation/rib.rs` (NEW) — synthetic `makeRib` pass on boostvoronoi output. Builds the 4-vertex quadrilateral cell structure. The single point of failure for the entire packet.
- **Primary code path 2:** `crates/slicer-core/src/skeletal_trapezoidation/centrality.rs` (354 LOC) — replace depth-floor + whisker-dissolve with `dR < dD * sin(angle/2)` on quad/rib topology.
- **Primary code path 3:** `crates/slicer-core/src/skeletal_trapezoidation/bead_count.rs` (113 LOC) — move `bead_count: Option<u32>` from `STHalfEdge` to the vertex type; per-NODE assignment via quad cell `distance_to_boundary`.
- **Primary code path 4:** `crates/slicer-core/src/skeletal_trapezoidation/propagation.rs` (274 LOC) — extract `mark_transitions`; add `generate_transition_mids` (pre-propagation) + `apply_transitions` (edge splitting); re-port propagation to read quad state.
- **Primary code path 5:** `crates/slicer-core/src/arachne/generate_toolpaths.rs` (391 LOC) — replace per-edge 2-junction fragment emission with faithful `connectJunctions` port.
- **Re-validation targets:** `crates/slicer-core/src/arachne/{stitch,simplify,remove_small}.rs` — re-baseline against multi-junction input from path 5.
- **OrcaSlicer comparison surface:** see `requirements.md` §OrcaSlicer Reference Obligations (delegate; never load).

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it. Note: this packet's change surface is entirely host-side (slicer-core); no WIT or module edits, so guest staleness is not expected. The freshness check is run as a precaution.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- **Single point of failure:** Step 1 (quad/rib pass) is the only structural dependency for Steps 2-5. If Step 1 produces incorrect topology, all 4 dependent passes fail. The implementer MUST run Step 1's tests (AC-1 + AC-N1 + AC-N4) and confirm CLEAN before proceeding to Step 2.

- **boostvoronoi degeneracy handling:** boostvoronoi produces degenerate zero-length edges at input-segment endpoints that OrcaSlicer's richer construction never produces. Step 1 must handle these BEFORE the rib pass runs. Decision: collapse (merge zero-length edges) or bridge (insert synthetic midpoint). This is a forward-dep on Step 1's design.

- **Type structure change:** `bead_count: Option<u32>` moves from `STHalfEdge` to the vertex type. All consumers (bead_count.rs, graph.rs, propagation.rs, any test) must be updated atomically.

- **No schema bump:** Topology changes are within existing IR types. `CURRENT_SLICE_IR_SCHEMA_VERSION` stays at 4.7.0 (P112's bump).

## Code Change Surface

- **Selected approach:** Build quad/rib topology first, then re-port 4 dependent passes, then re-validate 3 downstream stages, then re-baseline 8 fixtures. Each step is a localized edit; the topology chain is the gating dependency.
- **Exact functions, traits, manifests, tests, or fixtures expected to change:**
  - `crates/slicer-core/src/skeletal_trapezoidation/rib.rs` (NEW: `build_quad_rib_topology` function)
  - `crates/slicer-core/src/skeletal_trapezoidation/graph.rs` (ADD `rib_twin: Option<EdgeId>`, `quad_cell: Option<QuadCellId>` fields to `STHalfEdge`; ADD `bead_count: Option<u32>` to vertex type; REMOVE `bead_count` from `STHalfEdge`)
  - `crates/slicer-core/src/skeletal_trapezoidation/centrality.rs` (REPLACE depth-floor predicate with `dR < dD * sin(angle/2)` on quad/rib)
  - `crates/slicer-core/src/skeletal_trapezoidation/bead_count.rs` (REPLACE per-EDGE with per-NODE assignment)
  - `crates/slicer-core/src/skeletal_trapezoidation/propagation.rs` (REMOVE `mark_transitions` from propagation; ADD `generate_transition_mids` + `apply_transitions`; re-port `propagate_beadings_upward`/`downward`)
  - `crates/slicer-core/src/skeletal_trapezoidation/mod.rs` (ADD `pub mod rib;`)
  - `crates/slicer-core/src/arachne/generate_toolpaths.rs` (REPLACE per-edge 2-junction fragment emission with `connectJunctions` port)
  - `crates/slicer-core/src/arachne/pipeline.rs` (UPDATE call order: `run_arachne_pipeline` now calls `rib::build_quad_rib_topology` after `from_polygons` and before `filter_central`; calls `generate_transition_mids` before `propagate_*`; calls `apply_transitions` between transitions and propagation)
  - `crates/slicer-core/src/arachne/stitch.rs` (RE-VALIDATE against multi-junction input; no code change if invariants hold)
  - `crates/slicer-core/src/arachne/simplify.rs` (RE-VALIDATE; the Visvalingam port from P113a now actually exercises vertex removal)
  - `crates/slicer-core/src/arachne/remove_small.rs` (RE-VALIDATE; removal patterns change)
  - `crates/slicer-core/tests/fixtures/arachne/{centrality_square,centrality_wedge,centrality_multi_feature,bead_count_tapered_wedge,propagation_varying,propagation_uniform,propagation_multi_feature,toolpaths_tapered_wedge}.json` (RE-BASELINE all 8)
  - `crates/slicer-core/tests/{centrality,bead_count,propagation,generate_toolpaths}.rs` (UPDATE assertions if needed; the 3-fixture centralities now read re-baselined goldens)
  - `docs/DEVIATION_LOG.md` (CLOSE `D-112-CENTRALITY-ADAPT`, `D-112-PROPAGATION-ADAPT`; REGISTER + CLOSE `D-113B-CONNECTJUNCTIONS`)

- **Rejected alternatives:**
  - **Vertex-aggregation of per-EDGE counts to approximate per-NODE.** Rejected: a from-first-principles approximation of per-NODE assignment that doesn't read quad cell `distance_to_boundary`. The approximation would be functionally equivalent for tested fixtures but not algorithm-faithful.
  - **Defer the topology pass to a future packet.** Rejected: the M2 closure gate cannot be flipped to "fully-faithful" until this lands. The roadmap's M2 goal is algorithm faithfulness.
  - **Run P113a + P113b as a single mega-packet.** Rejected: the user's grilling session chose the split (P113a first, then P113b) precisely to ship the 6 independent items without gating on the L topology pass.

## Files in Scope (read + edit)

- `crates/slicer-core/src/skeletal_trapezoidation/rib.rs` — NEW; primary edit target
- `crates/slicer-core/src/skeletal_trapezoidation/centrality.rs` — primary edit target; faithful predicate
- `crates/slicer-core/src/skeletal_trapezoidation/bead_count.rs` — primary edit target; per-NODE assignment
- `crates/slicer-core/src/skeletal_trapezoidation/propagation.rs` — primary edit target; transition split + propagation re-port

## Read-Only Context

- `crates/slicer-core/src/skeletal_trapezoidation/graph.rs` — read `STHalfEdge` struct (lines 90-129) only
- `crates/slicer-core/src/arachne/pipeline.rs` — read `run_arachne_pipeline` (lines 244-310) only
- `crates/slicer-core/src/arachne/generate_toolpaths.rs` (391 LOC) — read as primary edit target
- `crates/slicer-core/src/arachne/{stitch,simplify,remove_small}.rs` — read as re-validation targets
- `crates/slicer-core/src/beading/mod.rs` — read `BeadingStrategy` trait only
- `crates/slicer-core/tests/fixtures/arachne/` — read 8 existing JSON fixtures as re-baseline source
- `docs/02_ir_schemas.md` — range-read §"Point3WithWidth" only
- `docs/04_host_scheduler.md` — range-read §"Phase 3 DAG Validation" only
- `docs/08_coordinate_system.md` — range-read §"Constant Conversion Table" only

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` — delegate parity checks; never load directly
- `target/`, `Cargo.lock`, generated code — never load
- `crates/slicer-core/src/arachne/stitch.rs` and `remove_small.rs` — re-validate only; no structural edits
- `modules/core-modules/arachne-perimeters/src/lib.rs` — not edited by this packet
- `crates/slicer-sdk/src/host.rs` — not edited (no WIT changes)
- `crates/slicer-schema/wit/deps/common.wit` — not edited (P113a covers the `arachne-params` record)

## Expected Sub-Agent Dispatches

- "Summarize `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidationGraph.cpp:452` `makeRib()`; return SUMMARY (≤ 200 words: rib-insertion algorithm, data structures for rib vs spine edge, quad cell construction rules, how degenerate zero-length edges are handled). No code." — purpose: design the `rib.rs` module
- "Summarize `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:672` `updateIsCentral()`; return SUMMARY (≤ 200 words: `dR < dD * sin(angle/2)` predicate, recursive dissolve loop, exit conditions). No code." — purpose: design the `centrality.rs` re-port
- "Summarize `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:925` `generateTransitionMids()` + `:1487` `applyTransitions()`; return SUMMARY (≤ 200 words: `transition_ratio` computation, `TransitionMiddle`/`TransitionEnd` marking rules, edge-splitting algorithm, ordering relative to propagation). No code." — purpose: design the `propagation.rs` re-port
- "Summarize `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:2260` `connectJunctions()`; return SUMMARY (≤ 200 words: per-edge junction fan walking, quad rib/non-rib chain stitch, `ExtrusionLine` emission). No code." — purpose: design the `generate_toolpaths.rs` re-port
- "Run `cargo test -p slicer-core --features host-algos --test centrality -- centrality_three_fixtures`; return FACT pass/fail." — purpose: validate AC-2
- "Run `cargo test -p slicer-core --features host-algos --test bead_count -- bead_count_tapered_wedge`; return FACT pass/fail." — purpose: validate AC-3
- "Run `cargo test -p slicer-core --features host-algos --test propagation -- propagation_three_fixtures`; return FACT pass/fail." — purpose: validate AC-4
- "Run `cargo test -p slicer-core --features host-algos --test generate_toolpaths -- generate_toolpaths_tapered_wedge`; return FACT pass/fail." — purpose: validate AC-5
- "Run `cargo test -p slicer-core --features host-algos --test {stitch,simplify,remove_small}`; return FACT pass/fail." — purpose: validate AC-6/7/8
- "Run `cargo xtask build-guests --check`; return FACT clean / STALE list." — purpose: guest WASM coherence (precaution)
- "Run `cargo xtask test --workspace --summary 2>&1 | tee target/test-output.log`; return FACT pass/fail + summary line + count." — purpose: workspace gate (M2 closure)

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

## Context Cost Estimate

- **Aggregate (sum across all 7 steps):** L. This is documented honestly — the synthetic `makeRib` pass on boostvoronoi is genuinely L effort. The implementer should plan to allocate 1-2 days of focused time to Step 1, with a clear go/no-go decision point at the end (AC-1 + AC-N1 + AC-N4 green → continue to Step 2; any fail → stop and report; do NOT continue past a broken Step 1).
- **Largest single step:** Step 1 (quad/rib pass) — L. This is the riskiest step in the entire packet and the single point of failure for Steps 2-5.
- **L-step exception (per spec-packet-generator skill rule "No step may be L; if it would, split"):** OVERRIDDEN at the user's explicit decision during packet refinement. The justification is that the `makeRib` algorithm is monolithic — partial rib insertion produces incorrect topology that blocks all 4 dependent passes, and there is no natural split point. The override is documented here, in `packet.spec.md` §Prerequisites and Blockers, and in `implementation-plan.md` §Per-Step Budget Roll-Up. If subsequent design work surfaces a natural split point (e.g., a separate "rib classification" pass that doesn't need quad cells), the packet SHOULD be split before activation. The override is a one-time exception to the skill rule, not a precedent for future L-step packets.
- **Highest-risk dispatch:** The 4 OrcaSlicer SUMMARY dispatches (makeRib, updateIsCentral, generateTransitionMids, connectJunctions) — each needs to return the algorithm's data structures, loop body, and edge cases. The `makeRib` SUMMARY is the most critical (its output drives the design of `rib.rs` which gates everything else). Required return format: SUMMARY (≤ 200 words, algorithm description, input/output types, edge cases, no code).

## Open Questions

- [FWD] How does OrcaSlicer's `makeRib` handle degenerate zero-length edges? Resolve via SUMMARY dispatch against `SkeletalTrapezoidationGraph.cpp:452` before Step 1 implementation. If the SUMMARY is unclear, dispatch a follow-up SUMMARY targeting the degeneracy handling specifically.
- [FWD] Where in `STHalfEdge` and the vertex type do `transition_ratio` and `quad_cell` fields live? Resolve by reading OrcaSlicer's `Edge` and `Node` types via SUMMARY dispatch against `SkeletalTrapezoidation.h` before Step 1's design finalizes.
- [FWD] What is the exact formula for `transition_ratio` in OrcaSlicer? Resolve via SUMMARY dispatch against `SkeletalTrapezoidation.cpp:925` (the `generateTransitionMids` function reads it) before Step 4 implementation.
- [FWD] Does the faithful `connectJunctions` output from Step 5 resolve the `D-112-MMU-TOPOLOGY` "tens of mm outside the naive per-face footprint" symptom? Resolve by re-running `cube_4color_arachne.rs` against the new output and checking the footprint bounds. If the symptom is gone, close the deviation. If the symptom persists, re-target the follow-up to the new evidence.
- [BLOCK] P113a must be `status: implemented` before this packet activates. This is an activation blocker, not an open design question — the packet's own `status` is `draft` for this reason.
