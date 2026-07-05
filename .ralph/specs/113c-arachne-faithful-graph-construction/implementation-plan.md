# Implementation Plan: 113c-arachne-faithful-graph-construction

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs (none — see `requirements.md`).
- TDD first, then implementation, then the narrowest falsifying validation.
- Each step honors the context-discipline preamble shared by `spec-packet-generator`, `swarm`,
  and `spec-review`. The fields below are not optional metadata — they are the budget contract
  for this step.

## Steps

### Step 1: Expose per-cell Voronoi metadata

- Task IDs: none (un-packeted remediation continuing packet 113b)
- Objective: Add `VCell` (mirroring `boostvoronoi::Cell`: `contains_point`, `contains_segment`,
  `contains_segment_startpoint`, `contains_segment_endpoint`, `source_index`,
  `source_category`, `get_incident_edge`, `is_degenerate`) and `HalfEdgeGraph::cells: Vec<VCell>`
  to `crates/slicer-core/src/voronoi.rs`.
- Precondition: none (first step).
- Postcondition: AC-1 green.
- Files allowed to read: `crates/slicer-core/src/voronoi.rs` (full; small file) — read as
  primary edit target.
- Files allowed to edit (≤ 3): `crates/slicer-core/src/voronoi.rs`.
- Files explicitly out-of-bounds: `crates/slicer-core/src/skeletal_trapezoidation/*.rs` (Steps
  2-6); `crates/slicer-core/src/arachne/*.rs` (Steps 4/7); `OrcaSlicerDocumented/` (not needed —
  this step only mirrors `boostvoronoi`'s own public API, already verified present).
- Expected sub-agent dispatches: none needed — `boostvoronoi::Cell`'s public API was already
  verified directly against the vendored crate source during this packet's design (see
  `design.md` §Verified Algorithm Mechanics); mirror it 1:1.
- Context cost: S
- Authoritative docs: none.
- OrcaSlicer refs: none (this step is `boostvoronoi`-facing, not OrcaSlicer-facing).
- Verification: `cargo test -p slicer-core --features host-algos --test voronoi -- voronoi_cells_square_metadata 2>&1 | tee target/test-output-voronoi-cells.log` — dispatch as FACT pass/fail (AC-1).
- Exit condition: AC-1 green.

### Step 2: Cell-range-walk research spike

- Task IDs: none
- Objective: Resolve, in writing (not code), whether a raw `incident_edge → next → …` cycle
  walk on Step 1's new `VCell`/`get_incident_edge` suffices for cell-range determination, or
  whether `compute_point_cell_range`/`compute_segment_cell_range`'s extra angle/category logic
  is genuinely required; and whether the `source_index()` shared-vertex dedup ambiguity
  (`design.md` §Verified Algorithm Mechanics) breaks provenance resolution in practice on the
  square fixture. Write the findings into `design.md` under a new heading (do not reuse the
  literal string "Step 2 Spike Findings" verbatim before this step actually runs — AC-2's check
  greps for exactly that string, so it must only appear once this step's real findings exist).
- Precondition: Step 1 green.
- Postcondition: AC-2 green (the findings section exists in `design.md` with both questions
  answered).
- Files allowed to read: `crates/slicer-core/src/voronoi.rs` (Step 1's new API);
  `crates/slicer-core/src/skeletal_trapezoidation/graph.rs` (read `from_polygons`'s current
  segment-flattening logic, lines covering `ring_segments`, to understand today's polygon→
  segment order).
- Files allowed to edit (≤ 3): `.ralph/specs/113c-arachne-faithful-graph-construction/
  design.md` (append findings); a throwaway test module (discarded after the spike, not
  committed).
- Files explicitly out-of-bounds: `crates/slicer-core/src/skeletal_trapezoidation/rib.rs`,
  `generate_toolpaths.rs`, `centrality.rs`, `bead_count.rs`, `propagation.rs` (Steps 3-6, not
  touched by a research spike).
- Expected sub-agent dispatches:
  - (relaxed contract) "Summarize `OrcaSlicerDocumented/src/libslic3r/Geometry/
    VoronoiUtils.cpp`'s `compute_segment_cell_range`/`compute_point_cell_range`, with up to a
    30-line excerpt of the range-finding loop body. Does a raw `incident_edge → next → …` cycle
    walk give an equivalent range without this additional logic?" — purpose: answer spike
    question (a)
  - "Write a throwaway Rust test module against a plain 10mm square input: dump per-cell cycle
    lengths, `is_primary`/`is_curved` composition, and `source_index()` values for point-cells
    at shared vertices. Report the raw data, then discard the test module." — purpose: answer
    spike question (b) empirically
- Context cost: S
- Authoritative docs: none.
- OrcaSlicer refs: `OrcaSlicerDocumented/src/libslic3r/Geometry/VoronoiUtils.cpp` — relaxed
  contract (code excerpts permitted).
- Verification: `rg -q 'Step 2 Spike Findings' .ralph/specs/113c-arachne-faithful-graph-construction/design.md` — dispatch as FACT pass/fail (AC-2).
- Exit condition: AC-2 green; both open questions in `design.md` §Open Questions answered and
  moved out of that list.

### Step 3: Faithful per-cell graph construction (L, single point of failure — L exception per user, same precedent as packet 113b)

- Task IDs: none
- Objective: Rewrite `SkeletalTrapezoidationGraph::from_polygons` to build the real per-cell
  chain + interleaved-rib topology per `design.md`'s §Verified Algorithm Mechanics: per-cell
  `transferEdge` walks building fresh spine chains, `makeRib` insertions after every transferred
  edge (cursor reassigned to `back_edge`), cross-cell twin-mirroring, provenance via
  `source_index()` + a flatten-time side table (informed by Step 2's spike findings), degenerate
  zero-length edge handling, curved-edge discretization re-integration. Supersede `rib.rs`'s
  reflex-corner-only pass.
- Precondition: Step 2 green (spike findings inform this step's design).
- Postcondition: AC-3 + AC-N1 + AC-N3 green. **If any of AC-3/AC-N1/AC-N3 fails, STOP. Do not
  proceed to Step 4.** Report the failure to the user and re-plan the construction.
- Files allowed to read: `crates/slicer-core/src/skeletal_trapezoidation/graph.rs` (full;
  primary edit target) — `crates/slicer-core/src/skeletal_trapezoidation/rib.rs` (full; being
  superseded) — `crates/slicer-core/src/skeletal_trapezoidation/discretize.rs` (full; small,
  curved-edge reuse) — `crates/slicer-core/src/voronoi.rs` (Step 1's new API) —
  `docs/adr/0023-arachne-port-strategy.md` (full; degeneracy contract this step must keep
  honoring).
- Files allowed to edit (≤ 3): `crates/slicer-core/src/skeletal_trapezoidation/graph.rs`;
  `crates/slicer-core/src/skeletal_trapezoidation/rib.rs` (delete `build_quad_rib_topology`/
  `QuadCell`, keep/relocate `EdgeType`/`RibData` if still needed); `crates/slicer-core/src/
  skeletal_trapezoidation/mod.rs` (module surface update).
- Files explicitly out-of-bounds: `OrcaSlicerDocumented/` — delegate (relaxed contract, code
  excerpts permitted this step); `crates/slicer-core/src/arachne/*.rs` (Step 4);
  `crates/slicer-core/src/skeletal_trapezoidation/{centrality,bead_count,propagation}.rs`
  (Steps 5-6).
- Expected sub-agent dispatches:
  - (relaxed contract) "Summarize `OrcaSlicerDocumented/src/libslic3r/Arachne/
    SkeletalTrapezoidation.cpp:431-560` `constructFromPolygons()`, including up to a 30-line
    code excerpt of the per-cell loop and its `makeRib` call sites. Explicitly describe how
    often `makeRib` is called relative to `transferEdge` and whether that frequency is
    unconditional." — purpose: confirm the per-cell loop structure this step ports
  - (relaxed contract) "Summarize `OrcaSlicerDocumented/src/libslic3r/Arachne/
    SkeletalTrapezoidation.cpp:157-257` `transferEdge()` with up to a 30-line excerpt of the
    'twin already exists' mirrored-construction branch (the `for (edge_t* twin = source_twin;;
    twin = twin->prev->twin->prev)` loop)." — purpose: design cross-cell twin-mirroring
  - (relaxed contract) "Summarize `OrcaSlicerDocumented/src/libslic3r/Arachne/
    SkeletalTrapezoidationGraph.cpp:452-482` `makeRib()` with up to a 30-line excerpt, confirming
    the `prev_edge` cursor reassignment to `back_edge` and how degenerate zero-length edges are
    handled." — purpose: design rib-insertion logic
- Context cost: **L** (this is the genuinely L step — synthetic per-cell construction with
  interleaved ribs, superseding both 113b's minimal rib pass and the current verbatim-DCEL-copy
  approach). **L-step exception documented**: see `packet.spec.md` §Prerequisites and Blockers
  — no natural split point, same category and justification as packet 113b's own L-exception
  for its `makeRib` step.
- Authoritative docs: `docs/adr/0023-arachne-port-strategy.md` — read full (degeneracy contract
  this step must keep honoring, not relax).
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:431-560` — relaxed
    contract
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:157-257` — relaxed
    contract
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidationGraph.cpp:452-482` —
    relaxed contract
- Verification:
  - `cargo test -p slicer-core --features host-algos --test skeletal_trapezoidation -- square_domain_closes_into_one_ring 2>&1 | tee target/test-output-graph-construction.log` — FACT pass/fail (AC-3)
  - `cargo test -p slicer-core --features host-algos --test skeletal_trapezoidation -- square_produces_multiple_ribs 2>&1 | tee target/test-output-rib-square-neg.log` — FACT pass/fail (AC-N1)
  - `cargo test -p slicer-core --features host-algos --test skeletal_trapezoidation -- graph_construction_is_deterministic 2>&1 | tee target/test-output-deterministic-neg.log` — FACT pass/fail (AC-N3)
- Exit condition: AC-3 + AC-N1 + AC-N3 green. **DO NOT proceed to Step 4 until this is green.**
  Recommended internal (non-gated) checkpoints while implementing, per `design.md`: (a)
  single-cell walk length-checked against the raw incident-edge cycle; (b) add rib insertion +
  cursor reassignment; (c) add cross-cell twin-mirroring; (d) validate the square closes.

### Step 4: Faithful `connectJunctions` domain-walk

- Task IDs: none
- Objective: Replace `is_domain_start`/`is_domain_edge`/`walk_domain_chain`/
  `process_central_domain`'s central-only-hop gate in `generate_toolpaths.rs` with the real
  quad-by-quad stitch: seed starts from edges with no valid `.prev` (now naturally true for
  every rib `back_edge` from Step 3), find each 2-3-edge quad via `getNextUnconnected`, pick the
  max-R edge via `getQuadMaxRedgeTo`, progressively splice onto the running `ExtrusionLine` via
  a `new_domain_start` flag.
- Precondition: Step 3 green.
- Postcondition: AC-4 green.
- Files allowed to read: `crates/slicer-core/src/arachne/generate_toolpaths.rs` (full; primary
  edit target) — `crates/slicer-core/src/arachne/pipeline.rs` (read `run_arachne_pipeline` call
  order only).
- Files allowed to edit (≤ 3): `crates/slicer-core/src/arachne/generate_toolpaths.rs`.
- Files explicitly out-of-bounds: `OrcaSlicerDocumented/` — delegate (default SUMMARY-only
  contract this step); `crates/slicer-core/src/skeletal_trapezoidation/*.rs` (Steps 1-3, 5-6);
  `crates/slicer-core/src/arachne/{stitch,simplify,remove_small}.rs` (Step 7).
- Expected sub-agent dispatches:
  - "Summarize `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:2260-2368`
    `connectJunctions()`; return SUMMARY (≤ 200 words: `unprocessed_quad_starts` seeding,
    `getQuadMaxRedgeTo`, the `new_domain_start`-flagged progressive stitch, odd-single-bead
    suppression via `passed_odd_edges`). No code." — purpose: confirm `design.md`'s pre-seeded
    mechanics before implementation
- Context cost: M
- Authoritative docs: none.
- OrcaSlicer refs: `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:2260-2368` — default SUMMARY-only contract.
- Verification: `cargo test -p slicer-core --features host-algos --test generate_toolpaths -- outer_wall_closes_for_simple_polygon 2>&1 | tee target/test-output-connectjunctions.log` — dispatch as FACT pass/fail (AC-4).
- Exit condition: AC-4 green.

### Step 5: Re-validate centrality / bead_count against the new graph

- Task IDs: none
- Objective: Re-validate (and adjust if needed) `centrality.rs`'s `dR < dD·sin(angle/2)`
  predicate and its `EdgeType::EXTRA_VD` exclusion against the new graph shape (ribs are now
  ubiquitous, not corner-only); re-validate `bead_count.rs`'s per-NODE assignment.
- Precondition: Step 4 green.
- Postcondition: AC-5 green. Fixtures re-recorded if behavior changed.
- Files allowed to read: `crates/slicer-core/src/skeletal_trapezoidation/centrality.rs` (full;
  primary edit target) — `crates/slicer-core/src/skeletal_trapezoidation/bead_count.rs` (full;
  primary edit target) — their test files and fixtures.
- Files allowed to edit (≤ 3): `crates/slicer-core/src/skeletal_trapezoidation/centrality.rs`;
  `crates/slicer-core/src/skeletal_trapezoidation/bead_count.rs`; their fixture JSON files.
- Files explicitly out-of-bounds: `OrcaSlicerDocumented/` — delegate; `crates/slicer-core/src/
  skeletal_trapezoidation/propagation.rs` (Step 6, deliberately not folded in here).
- Expected sub-agent dispatches:
  - "Run the existing `centrality`/`bead_count` test suites against the new graph from Step 3;
    identify which fixtures need re-baselining vs. which tests need logic changes. Return
    LOCATIONS (file:line + 1-line summary of each failure)." — purpose: scope this step's work
- Context cost: M
- Authoritative docs: none.
- OrcaSlicer refs: none needed (no new algorithm being ported here — re-validation only; if a
  genuine logic gap surfaces, dispatch `SkeletalTrapezoidation.cpp:672` `updateIsCentral()` at
  default SUMMARY-only contract).
- Verification: `cargo test -p slicer-core --features host-algos --test centrality --test bead_count 2>&1 | tee target/test-output-centrality-beadcount.log` — dispatch as FACT pass/fail (AC-5).
- Exit condition: AC-5 green; fixtures re-recorded.

### Step 6: Re-audit `propagation.rs::insert_node` (dedicated step per user decision during grilling)

- Task IDs: none
- Objective: Re-audit `insert_node`'s `next`/`prev`/`twin` rewiring during mid-edge transition
  splits against the new interleaved-rib chain shape from Step 3. This function was the site of
  3 compounding DCEL bugs under the OLD topology (`docs/DEVIATION_LOG.md` `D-112-MMU-TOPOLOGY`
  6th pass: stale next/prev on repeated same-edge splits, `twin` overwritten to the wrong
  endpoint, twin-mirroring pushed onto the wrong edge's list) — do not assume the prior fix
  generalizes; re-derive correctness against the new chain shape from first principles.
- Precondition: Step 5 green.
- Postcondition: AC-6 green. A dedicated regression test covers ≥2 same-edge splits near a rib
  insertion, distinct from Step 5's fixtures.
- Files allowed to read: `crates/slicer-core/src/skeletal_trapezoidation/propagation.rs` (full;
  primary edit target) — `docs/DEVIATION_LOG.md` `D-112-MMU-TOPOLOGY` entry (the 6th-pass
  root-cause narrative, for the exact prior bug shape).
- Files allowed to edit (≤ 3): `crates/slicer-core/src/skeletal_trapezoidation/propagation.rs`;
  its fixture JSON files.
- Files explicitly out-of-bounds: `OrcaSlicerDocumented/` — delegate; `crates/slicer-core/src/
  skeletal_trapezoidation/{centrality,bead_count}.rs` (Step 5, already closed).
- Expected sub-agent dispatches:
  - "Summarize `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidationGraph.cpp:
    310-431` `insertRib()`/`insertNode()`; return SUMMARY (≤ 200 words: twin-severing before
    split, cross-twin patching order, `transition_ratio` initialization on the mid-node). No
    code." — purpose: ground the re-audit against the real algorithm, not just the prior bug
    report
- Context cost: M (elevated rigor per its bug history, per `design.md` §Risks — treat with
  Step-3-level care despite the S/M sizing).
- Authoritative docs: `docs/DEVIATION_LOG.md` `D-112-MMU-TOPOLOGY` entry — read the 6th-pass
  section in full for the exact prior defect shape.
- OrcaSlicer refs: `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidationGraph.cpp:310-431` — default SUMMARY-only contract.
- Verification: `cargo test -p slicer-core --features host-algos --test propagation -- same_edge_splits_near_rib_insertion 2>&1 | tee target/test-output-insert-node.log` — dispatch as FACT pass/fail (AC-6).
- Exit condition: AC-6 green; dedicated regression test present and passing.

### Step 7: Re-validate stitch / simplify / remove_small

- Task IDs: none
- Objective: Confirm `stitch_extrusions`'s proximity-bridge path is now provably unreached on
  the square and tapered-wedge fixtures (rings arrive pre-closed from Step 4); confirm
  Visvalingam-Whyatt and length-based odd-line removal behave sanely on the now-longer closed
  lines; confirm the primary preservation invariant still holds.
- Precondition: Step 6 green.
- Postcondition: AC-7 + AC-N2 green. Fixtures re-baselined if needed.
- Files allowed to read: `crates/slicer-core/src/arachne/{stitch,simplify,remove_small}.rs`
  (full; re-validation targets) — their test files and fixtures.
- Files allowed to edit (≤ 3): `crates/slicer-core/src/arachne/{stitch,simplify,
  remove_small}.rs` (minimal edits only if invariants need adjustment); their fixture JSON
  files.
- Files explicitly out-of-bounds: `OrcaSlicerDocumented/` — not needed (no new algorithm being
  ported; this step is pure re-validation).
- Expected sub-agent dispatches:
  - "Run the 3 stage tests against Step 4's output; identify which pass without fixture changes
    vs. which need re-baselining. Return LOCATIONS (file:line + 1-line summary of each test's
    result)." — purpose: determine re-baselining scope
- Context cost: S
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p slicer-core --features host-algos --test stitch --test simplify --test remove_small 2>&1 | tee target/test-output-downstream.log` — FACT pass/fail (AC-7)
  - `cargo test -p slicer-core --features host-algos --test remove_small -- remove_small_lines_all_primary_invariant 2>&1 | tee target/test-output-remove-neg.log` — FACT pass/fail (AC-N2)
- Exit condition: AC-7 + AC-N2 green; fixtures re-baselined if needed.

### Step 8: Faithfulness invariant suite + selective `test_voronoi.cpp` triage

- Task IDs: none
- Objective: Add invariant tests derivable from the C++ source's own documented properties
  (closed-ring outer wall for simple input; quad spans 2-3 edges; `getNextUnconnected`
  termination bounded by edge count; `|from_junctions.len() - to_junctions.len()| <= 1` per
  stitch). Separately triage `test_voronoi.cpp`'s degenerate-input cases (self-intersection,
  missing vertices/edges, NaN) as candidates for `voronoi.rs`/`preprocess.rs` fixtures only —
  this triage does NOT bear on connectJunctions faithfulness (confirmed: zero OrcaSlicer unit
  tests exist for that layer).
- Precondition: Step 7 green.
- Postcondition: AC-8 green. A triage note with file:line provenance for any ported
  `test_voronoi.cpp` cases exists.
- Files allowed to read: `OrcaSlicerDocumented/tests/libslic3r/test_voronoi.cpp` (delegate —
  see dispatch below; the implementer should not load this 2163-line file directly) —
  `crates/slicer-core/src/arachne/preprocess.rs` (full; where any ported cases would land) —
  every module touched by Steps 3-7 (read-only, to write invariants against their real shapes).
- Files allowed to edit (≤ 3): new test file `crates/slicer-core/tests/arachne_invariants.rs`;
  `crates/slicer-core/src/arachne/preprocess.rs` fixtures (only if triage finds portable cases).
- Files explicitly out-of-bounds: none beyond the standing `OrcaSlicerDocumented/` direct-load
  rule.
- Expected sub-agent dispatches:
  - "Read `OrcaSlicerDocumented/tests/libslic3r/test_voronoi.cpp` (2163 lines, Catch2). Return
    LOCATIONS (≤ 20 entries, file:line + 1-line description) for every `TEST_CASE` covering
    degenerate Voronoi input (missing edges/vertices, duplicate vertices, self-intersection,
    NaN coordinates) that could plausibly reproduce against this crate's `voronoi.rs`/
    `preprocess.rs`. Do not attempt to map these to `connectJunctions` — that layer has no
    upstream tests." — purpose: AC-8's triage requirement
- Context cost: M
- Authoritative docs: none.
- OrcaSlicer refs: `OrcaSlicerDocumented/tests/libslic3r/test_voronoi.cpp` — delegate, LOCATIONS
  contract.
- Verification: `cargo test -p slicer-core --features host-algos --test arachne_invariants 2>&1 | tee target/test-output-invariants.log` — dispatch as FACT pass/fail (AC-8).
- Exit condition: AC-8 green; triage note recorded (even if it concludes zero cases are worth
  porting — the note itself, with reasoning, satisfies the requirement).

### Step 9: Re-baseline fixtures + correct deviation log + glossary

- Task IDs: none
- Objective: Re-record every self-captured Arachne fixture invalidated by Steps 3-8 (at
  minimum: `crates/slicer-core/tests/fixtures/arachne/*.json` touched by Steps 5-8;
  `crates/slicer-runtime/tests/fixtures/perimeter_parity/{tapered_wedge,
  narrow_strip_widening,max_bead_count_cap,complex_multi_feature,cube_4color_arachne}`).
  Strengthen `cube_4color_arachne_per_color_footprint_within_bbox` in place. Register
  `D-113C-FAITHFUL-GRAPH-CONSTRUCTION` in `docs/DEVIATION_LOG.md`, superseding
  `D-112-MMU-TOPOLOGY` and `D-113B-CONNECTJUNCTIONS` via a one-line addendum on each (not an
  in-place edit). Add `CONTEXT.md` glossary entries. Update `docs/01_system_architecture.md`/
  `docs/specs/perimeter-modules-orca-parity-roadmap.md`.
- Precondition: Step 8 green.
- Postcondition: AC-9 green. Deviation log and docs updated per `packet.spec.md`'s Doc Impact
  Statement.
- Files allowed to read: `docs/DEVIATION_LOG.md` (full) — `CONTEXT.md` (full; short) —
  `docs/01_system_architecture.md` (range-read §"Perimeter Modules" only) —
  `docs/specs/perimeter-modules-orca-parity-roadmap.md` (full) —
  `crates/slicer-runtime/tests/executor/cube_4color_arachne.rs` (full; primary edit target).
- Files allowed to edit (≤ 3 per sub-pass; this step is naturally multi-file, treat each
  sub-pass — fixtures, deviation log, docs — as its own bounded edit set): fixture JSON files;
  `docs/DEVIATION_LOG.md`; `CONTEXT.md`; `docs/01_system_architecture.md`;
  `docs/specs/perimeter-modules-orca-parity-roadmap.md`;
  `crates/slicer-runtime/tests/executor/cube_4color_arachne.rs`.
- Files explicitly out-of-bounds: `OrcaSlicerDocumented/` — not needed (documentation/fixture
  step, no new algorithm porting).
- Expected sub-agent dispatches:
  - "Re-record fixture X via its documented `#[ignore]`d `record_*` function; confirm the
    fixture's own doc-comment expectations (e.g. wall count) against the new output; report
    old-vs-new values." — purpose: per-fixture re-baseline, repeated for each affected fixture
  - "Draft the `D-113C-FAITHFUL-GRAPH-CONSTRUCTION` deviation-log entry and the two addendum
    lines for `D-112-MMU-TOPOLOGY`/`D-113B-CONNECTJUNCTIONS`; return the exact markdown to
    insert, without modifying either old entry's existing narrative text." — purpose: the
    supersession-pattern correction
- Context cost: S
- Authoritative docs: `docs/DEVIATION_LOG.md`, `CONTEXT.md` (see Files allowed to read).
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p slicer-runtime --test executor -- cube_4color_arachne 2>&1 | tee target/test-output-cube4color-strengthened.log` — FACT pass/fail (AC-9)
  - `rg -q 'D-113C-FAITHFUL-GRAPH-CONSTRUCTION' docs/DEVIATION_LOG.md && rg -q 'Superseded.*D-113C-FAITHFUL-GRAPH-CONSTRUCTION' docs/DEVIATION_LOG.md` — FACT pass/fail
  - `rg -q '### Rib edge' CONTEXT.md` — FACT pass/fail
- Exit condition: AC-9 green; deviation log corrected via supersession (not in-place edit);
  glossary and roadmap docs updated.

### Step 10: End-to-end verification on `cube_4color.3mf` + workspace gate

- Task IDs: none
- Objective: Re-slice `resources/cube_4color.3mf` with `wall_generator=arachne`, run the
  `;TYPE:Outer wall` start≈end gcode closure check used to diagnose this bug (formalized as a
  permanent test), confirm 0% closure failures (down from the pre-packet 100%/283 documented in
  this packet's provenance). Run the final workspace gate.
- Precondition: Steps 1-9 all green.
- Postcondition: AC-10 green. Full workspace gate green. Packet ready for `status:
  implemented`.
- Files allowed to read: `crates/slicer-runtime/tests/executor/cube_4color_arachne.rs` (full;
  primary edit target, adding the new end-to-end test alongside Step 9's strengthened one).
- Files allowed to edit (≤ 3): `crates/slicer-runtime/tests/executor/cube_4color_arachne.rs`.
- Files explicitly out-of-bounds: all other source files — not edited by this step.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-runtime --test executor -- cube_4color_arachne_outer_walls_close_end_to_end`; return FACT pass/fail." — purpose: validate AC-10
  - "Run `cargo xtask test --workspace --summary 2>&1 | tee target/test-output.log`; return
    FACT pass/fail + summary line + count." — purpose: final workspace gate (per CLAUDE.md
    §"Test Discipline" workspace-test exception)
  - "Run `cargo xtask build-guests --check`; return FACT clean / STALE list." — purpose: guest
    WASM coherence precaution
- Context cost: S
- Authoritative docs: none (this step is administrative/verification).
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p slicer-runtime --test executor -- cube_4color_arachne_outer_walls_close_end_to_end 2>&1 | tee target/test-output-e2e-closure.log` — FACT pass/fail (AC-10)
  - `cargo check --workspace --all-targets` — FACT pass/fail
  - `cargo clippy --workspace --all-targets -- -D warnings` — FACT pass/fail
  - `cargo xtask build-guests --check` — FACT clean / STALE list
  - `cargo xtask test --workspace --summary 2>&1 | tee target/test-output.log` — FACT pass/fail
- Exit condition: AC-10 green; full workspace gate green; `packet.spec.md` ready to move from
  `status: active` to `status: implemented`.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Per-cell metadata plumbing, additive only |
| Step 2 | S | Written spike note, throwaway test, no committed code |
| Step 3 | **L** | Faithful per-cell + interleaved-rib graph construction (single point of failure). **L-step exception documented per user decision** — see `packet.spec.md` §Prerequisites and Blockers and `design.md` §Context Cost Estimate. |
| Step 4 | M | Faithful `connectJunctions` domain-walk |
| Step 5 | M | Centrality/bead_count re-validation + fixture re-baselines |
| Step 6 | M (elevated rigor) | Dedicated `insert_node` re-audit given its bug history |
| Step 7 | S | Stitch/simplify/remove_small re-validation |
| Step 8 | M | Invariant suite + `test_voronoi.cpp` triage |
| Step 9 | S | Fixture re-baseline + deviation log + glossary |
| Step 10 | S | End-to-end verification + workspace gate |

Sum: L aggregate; Step 3 is the only L step (genuinely L, the synthetic construction). L-step
exception documented; see `design.md` §Context Cost Estimate for the full justification.

## Packet Completion Gate

- All 10 steps complete.
- Every step exit condition is met.
- Packet acceptance criteria green (AC-1 through AC-10, AC-N1 through AC-N3, each verified by
  their pipe-suffixed command).
- Deviation log corrected via the supersession pattern: `D-113C-FAITHFUL-GRAPH-CONSTRUCTION`
  registered; one-line addenda present on `D-112-MMU-TOPOLOGY` and `D-113B-CONNECTJUNCTIONS`
  (their existing narrative text untouched).
- All invalidated fixtures re-baselined and committed.
- `docs/adr/0034-arachne-faithful-graph-construction.md` present (authored alongside packet
  authoring, already done — confirm it is not stale against the final implementation).
- `CONTEXT.md` glossary entries present.
- M2-faithful roadmap docs updated.
- `cargo xtask test --workspace --summary` green.
- `packet.spec.md` ready to move from `status: active` to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md`.
- Confirm packet-level verification commands are green.
- Record any remaining packet-local risk explicitly before moving to `status: implemented`.
- Confirm the implementer's peak context usage stayed under 70%; if not, log it as a
  packet-authoring lesson (same discipline packet 113b's own acceptance ceremony required).
- Confirm `docs/adr/0034-arachne-faithful-graph-construction.md` still accurately describes what
  was actually built — if Step 3's real implementation diverged from the ADR's description
  (e.g. Step 2's spike changed the provenance approach), update the ADR before packet closure.
