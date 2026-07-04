# Implementation Plan: 113b-arachne-topology-faithfulness

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first, then implementation, then the narrowest falsifying validation.
- Each step honors the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. The fields below are not optional metadata — they are the budget contract for this step.

## Steps

### Step 1: Quad/rib topology pass (L, single point of failure — L exception per user)

- Task IDs:
  - none (M2 follow-up; crosswalk to T-220 in the M2 plan)
- Objective: Build the synthetic `makeRib` pass on boostvoronoi output. Insert rib edges connecting polygon corners to the medial axis; build the 4-vertex quadrilateral cell structure. Add `rib_twin: Option<EdgeId>` and `quad_cell: Option<QuadCellId>` fields to `STHalfEdge`. Handle boostvoronoi's degenerate zero-length edges at input-segment endpoints (collapse or bridge — design decision made in this step).
- Precondition: OrcaSlicer `makeRib` algorithm obtained via SUMMARY dispatch. P113a is `status: implemented` (this packet's own activation blocker).
- Postcondition: AC-1 + AC-N1 + AC-N4 green. Step 1 is the gating dependency for Steps 2-5. **If AC-1 or AC-N1 or AC-N4 fails, STOP. Do not proceed to Step 2.** Report the failure to the user and re-plan the rib pass.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-core/src/skeletal_trapezoidation/graph.rs` (~200 LOC) — read `STHalfEdge` struct (lines 90-129) only
  - `crates/slicer-core/src/skeletal_trapezoidation/mod.rs` — full (small file)
  - `crates/slicer-core/src/skeletal_trapezoidation/centrality.rs` (354 LOC) — read module doc (lines 1-60) only
  - `crates/slicer-core/src/voronoi.rs` — read public API only
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/skeletal_trapezoidation/rib.rs` — NEW; primary edit target
  - `crates/slicer-core/src/skeletal_trapezoidation/graph.rs` — add 2 fields
  - `crates/slicer-core/src/skeletal_trapezoidation/mod.rs` — add `pub mod rib;`
- Files explicitly out-of-bounds for this step:
  - `OrcaSlicerDocumented/` — delegate; never load
  - `crates/slicer-core/src/skeletal_trapezoidation/{bead_count,propagation}.rs` — Steps 3-4
  - `crates/slicer-core/src/arachne/*.rs` — Steps 5-8
- Expected sub-agent dispatches:
  - "Summarize `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidationGraph.cpp:452` `makeRib()`; return SUMMARY (≤ 200 words: rib-insertion algorithm, data structures for rib vs spine edge classification, quad cell construction rules, how degenerate zero-length edges at input-segment endpoints are handled). No code." — purpose: design the `rib.rs` module
- Context cost: **L** (this is the genuinely L step — synthetic construction on a different Voronoi library than OrcaSlicer's `vd_t`). **L-step exception documented**: the spec-packet-generator skill rule "No step may be L; if it would, split" is OVERRIDDEN at the user's explicit decision during packet refinement. The `makeRib` algorithm is monolithic (no natural split point; partial rib insertion produces incorrect topology that blocks all 4 dependent passes). See `design.md` §Context Cost Estimate for the full justification.
- Authoritative docs:
  - `docs/02_ir_schemas.md` — read §"Point3WithWidth" only — purpose: confirm no schema bump needed
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidationGraph.cpp:452` — delegate; never load
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidationGraph.h` — delegate a struct field SUMMARY if needed
- Verification:
  - `cargo test -p slicer-core --features host-algos --test skeletal_trapezoidation -- quad_rib_topology_square_has_no_ribs 2>&1 | tee target/test-output-rib-square.log` — dispatch as FACT pass/fail (AC-1 + AC-N1)
  - `cargo test -p slicer-core --features host-algos --test skeletal_trapezoidation -- quad_rib_topology_is_deterministic 2>&1 | tee target/test-output-rib-deterministic.log` — dispatch as FACT pass/fail (AC-N4)
- Exit condition: AC-1 + AC-N1 + AC-N4 green. **DO NOT proceed to Step 2 until this is green.** If any of AC-1/AC-N1/AC-N4 fails, STOP and report.

### Step 2: Faithful `filter_central` on quad/rib topology

- Task IDs:
  - none (M2 follow-up; crosswalk to T-220 in the M2 plan)
- Objective: Replace the depth-floor + whisker-dissolve predicate in `centrality.rs` with OrcaSlicer's `dR < dD * sin(angle/2)` predicate that reads the quad/rib topology from Step 1. The angle is between two spine edges at a spine vertex.
- Precondition: Step 1 (quad/rib pass) green.
- Postcondition: AC-2 green. The 3 centrality fixtures are re-baselined to the faithful predicate's output.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-core/src/skeletal_trapezoidation/centrality.rs` (354 LOC) — read as primary edit target
  - `crates/slicer-core/tests/centrality.rs` — read as test file
  - `crates/slicer-core/tests/fixtures/arachne/centrality_*.json` — read as fixtures to re-baseline
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/skeletal_trapezoidation/centrality.rs` — primary edit target
  - `crates/slicer-core/tests/centrality.rs` — re-baseline fixture path constant if needed
  - `crates/slicer-core/tests/fixtures/arachne/centrality_*.json` — re-baseline goldens (3 files)
- Files explicitly out-of-bounds for this step:
  - `OrcaSlicerDocumented/` — delegate; never load
  - `crates/slicer-core/src/skeletal_trapezoidation/{bead_count,propagation}.rs` — Steps 3-4
- Expected sub-agent dispatches:
  - "Summarize `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:672` `updateIsCentral()`; return SUMMARY (≤ 200 words: `dR < dD * sin(angle/2)` predicate, recursive dissolve loop, exit conditions). No code." — purpose: design the faithful `filter_central`
  - "Run the 3 centrality test cases; capture the output; write new golden files. This is a one-time re-baseline, not regenerated." — purpose: re-baseline the 3 fixtures
- Context cost: M
- Authoritative docs:
  - None (the centrality algorithm is in the source itself)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:672` — delegate; never load
- Verification:
  - `cargo test -p slicer-core --features host-algos --test centrality -- centrality_three_fixtures 2>&1 | tee target/test-output-centrality-faithful.log` — dispatch as FACT pass/fail (AC-2, with re-baselined fixtures)
- Exit condition: AC-2 green; 3 fixtures re-baselined.

### Step 3: Per-NODE bead_count via quad cell `distance_to_boundary`

- Task IDs:
  - none (M2 follow-up; crosswalk to T-221 in the M2 plan)
- Objective: Move `bead_count: Option<u32>` from `STHalfEdge` to the vertex type. Re-port `assign_bead_counts` to assign at Voronoi vertices via quad cell `distance_to_boundary`, reading from the quad/rib topology from Step 1.
- Precondition: Step 2 green.
- Postcondition: AC-3 + AC-N2 green. The 1 bead_count fixture is re-baselined.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-core/src/skeletal_trapezoidation/bead_count.rs` (113 LOC) — read as primary edit target
  - `crates/slicer-core/src/skeletal_trapezoidation/graph.rs` (~200 LOC) — read vertex type
  - `crates/slicer-core/tests/bead_count.rs` — read as test file
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/skeletal_trapezoidation/bead_count.rs` — primary edit target
  - `crates/slicer-core/src/skeletal_trapezoidation/graph.rs` — move `bead_count` field from `STHalfEdge` to vertex type
  - `crates/slicer-core/tests/fixtures/arachne/bead_count_tapered_wedge.json` — re-baseline
- Files explicitly out-of-bounds for this step:
  - `OrcaSlicerDocumented/` — delegate; never load
  - `crates/slicer-core/src/skeletal_trapezoidation/propagation.rs` — Step 4
- Expected sub-agent dispatches:
  - "Find the vertex type in `crates/slicer-core/src/skeletal_trapezoidation/graph.rs`; return its current field list and the location of the `STHalfEdge` struct. Identify which fields need to be added to the vertex type to support per-NODE bead count." — purpose: plan the type change
  - "Summarize `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:777` `updateBeadCount()`; return SUMMARY (≤ 200 words: per-NODE assignment, `distance_to_boundary` computation, beading strategy call). No code." — purpose: design the faithful per-NODE assignment
- Context cost: M
- Authoritative docs:
  - None
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:777` — delegate; never load
- Verification:
  - `cargo test -p slicer-core --features host-algos --test bead_count -- bead_count_tapered_wedge 2>&1 | tee target/test-output-bead-faithful.log` — dispatch as FACT pass/fail (AC-3)
  - `cargo test -p slicer-core --features host-algos --test bead_count -- bead_count_requires_centrality 2>&1 | tee target/test-output-bead-neg.log` — dispatch as FACT pass/fail (AC-N2)
- Exit condition: AC-3 + AC-N2 green; bead_count fixture re-baselined.

### Step 4: Faithful transition marking + propagation re-port

- Task IDs:
  - none (M2 follow-up; crosswalk to T-222 in the M2 plan)
- Objective: Extract `mark_transitions` from the propagation passes. Add new `generate_transition_mids` (pre-propagation) + `apply_transitions` (edge splitting) functions. Re-port `propagate_beadings_upward`/`propagate_beadings_downward` to read quad-decorated graph state. Add `transition_ratio: f64` field to the vertex type.
- Precondition: Step 3 green.
- Postcondition: AC-4 green. The 3 propagation fixtures are re-baselined.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-core/src/skeletal_trapezoidation/propagation.rs` (274 LOC) — read as primary edit target
  - `crates/slicer-core/src/skeletal_trapezoidation/graph.rs` (~200 LOC) — read vertex type
  - `crates/slicer-core/tests/propagation.rs` — read as test file
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/skeletal_trapezoidation/propagation.rs` — primary edit target
  - `crates/slicer-core/src/skeletal_trapezoidation/graph.rs` — add `transition_ratio` field
  - `crates/slicer-core/tests/fixtures/arachne/propagation_*.json` — re-baseline (3 files)
- Files explicitly out-of-bounds for this step:
  - `OrcaSlicerDocumented/` — delegate; never load
  - `crates/slicer-core/src/arachne/generate_toolpaths.rs` — Step 5
- Expected sub-agent dispatches:
  - "Summarize `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:925` `generateTransitionMids()` + `:1487` `applyTransitions()`; return SUMMARY (≤ 200 words: `transition_ratio` computation, `TransitionMiddle`/`TransitionEnd` marking rules, edge-splitting algorithm, ordering relative to propagation). No code." — purpose: design the transition split
  - "Summarize `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:1800,1833` `propagateBeadingsUpward`/`propagateBeadingsDownward()`; return SUMMARY (≤ 200 words: how propagation reads quad state, when it marks transitions, ordering of upward vs downward passes). No code." — purpose: design the propagation re-port
- Context cost: M
- Authoritative docs:
  - `docs/08_coordinate_system.md` — read §"Constant Conversion Table" only — purpose: `transition_ratio` unit
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:925,1487,1800,1833` — delegate; never load
- Verification:
  - `cargo test -p slicer-core --features host-algos --test propagation -- propagation_three_fixtures 2>&1 | tee target/test-output-propagation-faithful.log` — dispatch as FACT pass/fail (AC-4, with re-baselined fixtures)
- Exit condition: AC-4 green; 3 propagation fixtures re-baselined.

### Step 5: Faithful `connectJunctions` in `generate_toolpaths`

- Task IDs:
  - none (M2 follow-up; crosswalk to T-223 in the M2 plan)
- Objective: Replace per-edge 2-junction fragment emission in `generate_toolpaths.rs` with a faithful port of OrcaSlicer's `connectJunctions` that stitches per-edge junction fans into full `ExtrusionLine`s across quad rib/non-rib chains. Output becomes multi-junction lines, some closed. Update `pipeline.rs` call order.
- Precondition: Step 4 green.
- Postcondition: AC-5 green. The 1 generate_toolpaths fixture is re-baselined. The pipeline call order is updated.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-core/src/arachne/generate_toolpaths.rs` (391 LOC) — read as primary edit target
  - `crates/slicer-core/src/arachne/pipeline.rs` — read `run_arachne_pipeline` (lines 244-310) only
  - `crates/slicer-core/tests/generate_toolpaths.rs` — read as test file
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/arachne/generate_toolpaths.rs` — primary edit target
  - `crates/slicer-core/src/arachne/pipeline.rs` — update call order
  - `crates/slicer-core/tests/fixtures/arachne/toolpaths_tapered_wedge.json` — re-baseline
- Files explicitly out-of-bounds for this step:
  - `OrcaSlicerDocumented/` — delegate; never load
  - `crates/slicer-core/src/arachne/{stitch,simplify,remove_small}.rs` — Steps 6-8
- Expected sub-agent dispatches:
  - "Summarize `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:2260` `connectJunctions()`; return SUMMARY (≤ 200 words: per-edge junction fan walking, quad rib/non-rib chain stitch, `ExtrusionLine` emission). No code." — purpose: design the faithful `connectJunctions`
- Context cost: M
- Authoritative docs:
  - None
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:2260` — delegate; never load
- Verification:
  - `cargo test -p slicer-core --features host-algos --test generate_toolpaths -- generate_toolpaths_tapered_wedge 2>&1 | tee target/test-output-toolpaths-faithful.log` — dispatch as FACT pass/fail (AC-5, with re-baselined fixture)
- Exit condition: AC-5 green; toolpaths fixture re-baselined; pipeline call order updated.

### Step 6: Re-validate stitch + simplify + remove_small against multi-junction input

- Task IDs:
  - none (M2 follow-up; crosswalk to T-225..T-227 in the M2 plan)
- Objective: Run the 3 downstream stage tests against the new multi-junction input from Step 5. Confirm invariants hold. Re-baseline fixtures if needed (simplify and remove_small will have different outputs).
- Precondition: Step 5 green.
- Postcondition: AC-6 + AC-7 + AC-8 + AC-N3 green. simplify and remove_small fixtures re-baselined.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-core/src/arachne/stitch.rs` (249 LOC) — read as re-validation target
  - `crates/slicer-core/src/arachne/simplify.rs` (140 LOC) — read as re-validation target
  - `crates/slicer-core/src/arachne/remove_small.rs` (77 LOC) — read as re-validation target
  - `crates/slicer-core/tests/{stitch,simplify,remove_small}.rs` — read as test files
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/arachne/{stitch,simplify,remove_small}.rs` — minimal edits only if invariants need adjustment
  - `crates/slicer-core/tests/fixtures/arachne/{simplify,remove_small}_tapered_wedge.json` — re-baseline if needed
- Files explicitly out-of-bounds for this step:
  - `OrcaSlicerDocumented/` — delegate; never load
- Expected sub-agent dispatches:
  - "Run the 3 stage tests; identify which tests pass without fixture changes vs which need fixture re-baselining; return LOCATIONS (file:line + 1-line summary of each test's result)." — purpose: determine re-baselining scope
- Context cost: S
- Authoritative docs:
  - None
- OrcaSlicer refs:
  - None
- Verification:
  - `cargo test -p slicer-core --features host-algos --test stitch -- stitch_extrusions_preserves_primary 2>&1 | tee target/test-output-stitch-faithful.log` — dispatch as FACT pass/fail (AC-6)
  - `cargo test -p slicer-core --features host-algos --test simplify -- simplify_toolpaths_vertex_count 2>&1 | tee target/test-output-simplify-faithful.log` — dispatch as FACT pass/fail (AC-7, with re-baselined fixture)
  - `cargo test -p slicer-core --features host-algos --test remove_small -- remove_small_lines_preserves_primary 2>&1 | tee target/test-output-remove-faithful.log` — dispatch as FACT pass/fail (AC-8)
  - `cargo test -p slicer-core --features host-algos --test remove_small -- remove_small_lines_all_primary_invariant 2>&1 | tee target/test-output-remove-neg.log` — dispatch as FACT pass/fail (AC-N3)
- Exit condition: AC-6 + AC-7 + AC-8 + AC-N3 green; simplify and remove_small fixtures re-baselined if needed.

### Step 7: Close 2 deviations + register+close 1 + re-verify D-112-MMU-TOPOLOGY + workspace gate

- Task IDs:
  - none (M2 follow-up)
- Objective: Close `D-112-CENTRALITY-ADAPT`, `D-112-PROPAGATION-ADAPT` in `docs/DEVIATION_LOG.md`. Register + close `D-113B-CONNECTJUNCTIONS`. Re-verify `D-112-MMU-TOPOLOGY` against the faithful `connectJunctions` output (close it if the symptom is gone; re-target the follow-up if the symptom persists). Update `docs/01_system_architecture.md` and `docs/specs/perimeter-modules-orca-parity-roadmap.md` to mark P113a + P113b complete. Run the workspace gate.
- Precondition: Steps 1-6 all green.
- Postcondition: 2 deviations closed; 1 new deviation registered + closed; `D-112-MMU-TOPOLOGY` either closed (if symptom gone) or re-targeted (if symptom persists); M2-faithful roadmap update; workspace gate green; packet ready for `status: implemented`.
- Files allowed to read (with line-range hints when > 300 lines):
  - `docs/DEVIATION_LOG.md` (50 lines) — read full
  - `docs/01_system_architecture.md` — range-read §"Perimeter Modules" only
  - `docs/specs/perimeter-modules-orca-parity-roadmap.md` — read full
- Files allowed to edit (≤ 3):
  - `docs/DEVIATION_LOG.md` — close 2 + register+close 1 + re-verify D-112-MMU-TOPOLOGY
  - `docs/01_system_architecture.md` — update M2 marker
  - `docs/specs/perimeter-modules-orca-parity-roadmap.md` — add P113a+P113b section
- Files explicitly out-of-bounds for this step:
  - `OrcaSlicerDocumented/` — delegate; never load
  - All other source files — not edited
- Expected sub-agent dispatches:
  - "Run `cargo xtask test --workspace --summary 2>&1 | tee target/test-output.log`; return FACT pass/fail + summary line + count." — purpose: workspace gate (per CLAUDE.md §"Test Discipline" workspace-test exception)
  - "Re-run `crates/slicer-runtime/tests/executor/cube_4color_arachne.rs` against the new faithful `connectJunctions` output; check whether the 'tens of mm outside the naive per-face footprint' symptom persists. Return FACT: symptom gone / symptom persists with new evidence." — purpose: re-verify D-112-MMU-TOPOLOGY
- Context cost: S
- Authoritative docs:
  - None (this step is administrative)
- OrcaSlicer refs:
  - None
- Verification:
  - `rg -q 'D-112-CENTRALITY-ADAPT.*Closed' docs/DEVIATION_LOG.md && rg -q 'D-112-PROPAGATION-ADAPT.*Closed' docs/DEVIATION_LOG.md && rg -q 'D-113B-CONNECTJUNCTIONS.*Closed' docs/DEVIATION_LOG.md` — dispatch as FACT (AC-10, all 3 grep must succeed)
  - `for f in centrality_square.json centrality_wedge.json centrality_multi_feature.json bead_count_tapered_wedge.json propagation_varying.json propagation_uniform.json propagation_multi_feature.json toolpaths_tapered_wedge.json; do test -f "crates/slicer-core/tests/fixtures/arachne/$f" && echo "PRESENT $f" || echo "MISSING $f"; done` — dispatch as FACT (AC-9, all 8 PRESENT)
  - `rg -q 'M2.*complete.*P110.*P111.*P112.*P113a.*P113b' docs/01_system_architecture.md` — dispatch as FACT
  - `rg -q 'P113a.*complete\|P113b.*complete' docs/specs/perimeter-modules-orca-parity-roadmap.md` — dispatch as FACT
  - `cargo xtask test --workspace --summary 2>&1 | tee target/test-output.log` — dispatch as FACT pass/fail
  - `cargo xtask build-guests --check` — dispatch as FACT clean / STALE list (precaution)
- Exit condition: 2 deviation closures verified; 1 new deviation registered + closed; 8 fixtures PRESENT; M2-faithful roadmap updated; workspace gate green.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | **L** | Quad/rib synthetic edge insertion (single point of failure). **L-step exception documented per user decision** — see `design.md` §Context Cost Estimate and `packet.spec.md` §Prerequisites and Blockers. |
| Step 2 | M | Faithful centrality predicate + 3 fixture re-baselines |
| Step 3 | M | Type change (bead_count from edge to node) + 1 fixture re-baseline |
| Step 4 | M | Transition split + propagation re-port + 3 fixture re-baselines |
| Step 5 | M | Faithful connectJunctions + 1 fixture re-baseline + pipeline call order |
| Step 6 | S | Re-validate 3 downstream stages + 2 fixture re-baselines |
| Step 7 | S | Close 3 deviations (2 + register/close 1) + re-verify D-112-MMU-TOPOLOGY + workspace gate |

Sum: L aggregate; Step 1 is the only L step (genuinely L, the synthetic construction). L-step exception documented; see design.md §Context Cost Estimate for the full justification.

## Packet Completion Gate

- All 7 steps complete.
- Every step exit condition is met.
- Packet acceptance criteria green (AC-1 through AC-10, AC-N1 through AC-N4 each verified by their pipe-suffixed command).
- 3 deviations touched in `docs/DEVIATION_LOG.md`: 2 closed (`D-112-CENTRALITY-ADAPT`, `D-112-PROPAGATION-ADAPT`); 1 new registered + closed (`D-113B-CONNECTJUNCTIONS`); `D-112-MMU-TOPOLOGY` re-verified (closed if symptom gone, re-targeted if symptom persists).
- 8 re-baselined fixtures committed.
- M2-faithful roadmap updated.
- `cargo xtask test --workspace --summary` green.
- `packet.spec.md` ready to move from `status: draft` to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md`.
- Confirm packet-level verification commands are green.
- Record any remaining packet-local risk explicitly before moving to `status: implemented`.
- Confirm the implementer's peak context usage stayed under 70%; if not, log it as a packet-authoring lesson for future spec-packet-generator runs.
- No ADR-0033 dependency. The original packet draft listed "ADR-0033 (Algorithm Faithfulness as OrcaSlicer Parity Definition)" as a P113b dependency. That ADR does not exist and the user has not asked for it. Removed.
