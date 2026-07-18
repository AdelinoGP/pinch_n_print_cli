# Requirements: 113c-arachne-faithful-graph-construction

## Packet Metadata

- Grouped task IDs: **none** (this is un-packeted remediation continuing past packet 113b,
  provenanced by a `/diagnose` session against a live user-reported bug, not a `docs/07_
  implementation_status.md` `TASK-###`; the crosswalk is `docs/DEVIATION_LOG.md`'s
  `D-112-MMU-TOPOLOGY` and `D-113B-CONNECTJUNCTIONS` entries).
- Backlog source: `docs/DEVIATION_LOG.md` `D-112-MMU-TOPOLOGY` (line 50) and
  `D-113B-CONNECTJUNCTIONS` (line 56) — both currently `Closed`, both re-opened by this packet's
  root-cause finding; `/diagnose` session 2026-07-05 against `resources/cube_4color.3mf`.
- Packet status: `active` (confirmed during grilling — no other packet currently holds the
  active slot in `.ralph/specs/`).
- Aggregate context cost: `L` (Step 3, the faithful per-cell graph construction, is the gating
  L-effort step — see `packet.spec.md` §Prerequisites and Blockers for the exception,
  re-confirmed against packet 113b's own precedent during this packet's grilling session).

## Problem Statement

Packet 113b's `build_quad_rib_topology` (`crates/slicer-core/src/skeletal_trapezoidation/rib.rs`)
only inserts rib edges at reflex/sharp polygon corners — an admitted "Step 1 minimal"
implementation that was never upgraded to the full algorithm. Real OrcaSlicer's
`constructFromPolygons`/`makeRib` inserts a rib edge pair after **every** transferred edge,
interleaved directly into the `next`/`prev` chain; that interleaving is what lets
`getNextUnconnected()` traverse junctions of any degree. Because this codebase's
`SkeletalTrapezoidationGraph::from_polygons` instead copies `next`/`prev`/`twin` verbatim from
the raw `boostvoronoi` per-cell DCEL (which encodes "walk around one Voronoi cell's own
boundary," not "continue along the medial-axis spine"), `generate_toolpaths.rs`'s domain-walk
breaks at every junction/branch vertex — present in every non-trivial polygon, including a
plain square's single central X-junction. A `/diagnose` session (2026-07-05) confirmed this is
not an edge case: 100% of outer-wall gcode segments for `resources/cube_4color.3mf` fail to
close (283/283, mean gap 18.7mm), reproduced in isolation via `run_arachne_pipeline` on a single
polygon, and reproduced even for a bare 10mm square (the existing
`arachne_perimeters_simple_square` test's own doc comment admits 26 fragmented lines). This
mis-diagnosis traces to packet 113b's OrcaSlicer-read-delegation protocol: its `makeRib`
dispatch asked for a SUMMARY of the callee's ~30-line body in isolation, never
`constructFromPolygons`'s caller loop — the single fact (called after every edge, not just at
corners) that would have caught this. `docs/DEVIATION_LOG.md`'s `D-112-MMU-TOPOLOGY` and
`D-113B-CONNECTJUNCTIONS` entries both show `Closed`, but neither closure touched graph
construction: `D-112-MMU-TOPOLOGY`'s 11th-pass closure was a test-harness realignment (gcode
header re-pairing to eliminate a sampling-aliasing artifact); `D-113B-CONNECTJUNCTIONS`'s
closure was the central-only domain-walk generalization now proven insufficient (breaks at
every rib). This packet replaces the graph-construction and junction-connection layer with a
faithful port of the real algorithm, re-validates every downstream Arachne stage against the
new graph shape, and corrects the deviation-log record.

## In Scope

- **Per-cell Voronoi metadata** in `crates/slicer-core/src/voronoi.rs` (NEW `VCell` type,
  `HalfEdgeGraph::cells`): `contains_point`, `contains_segment`, `contains_segment_startpoint`,
  `contains_segment_endpoint`, `source_index`, `source_category`, `get_incident_edge`,
  `is_degenerate`, mirroring `boostvoronoi::Cell` (verified present in the vendored crate
  during this packet's grilling session — no crate patching needed).
- **Cell-range-walk research spike**: a written note in `design.md` resolving (a) whether a raw
  `incident_edge → next → …` cycle walk suffices for cell-range determination or extra
  angle/category logic is needed, matching `compute_point_cell_range`/`compute_segment_cell_range`;
  (b) whether the `source_index()` shared-vertex dedup ambiguity (a point-cell's surviving index
  may resolve to either of two adjacent segments, confirmed via direct `builder.rs` read during
  grilling) breaks provenance resolution.
- **Faithful per-cell graph construction** in `crates/slicer-core/src/skeletal_trapezoidation/
  graph.rs::from_polygons`: per-cell `transferEdge` walks building fresh spine chains,
  interleaved with `makeRib` insertions after every transferred edge (cursor reassigned to
  `back_edge`), cross-cell twin-mirroring, provenance via `source_index()` + a flatten-time side
  table (no `Segment` struct changes — verified safe during grilling), degenerate zero-length
  edge handling (same hazard packet 113b flagged), curved-edge discretization re-integration.
  `rib.rs`'s reflex-corner-only pass is superseded.
- **Faithful `connectJunctions`** in `crates/slicer-core/src/arachne/generate_toolpaths.rs`:
  replace the central-only `walk_domain_chain` gate with the real quad-by-quad stitch
  (`getNextUnconnected`, `getQuadMaxRedgeTo`, `new_domain_start`-flagged progressive splice).
- **Re-validation of `centrality.rs`/`bead_count.rs`** against the new graph shape (ribs now
  ubiquitous, not corner-only); fixtures re-recorded.
- **Dedicated re-audit of `propagation.rs::insert_node`**: rewiring assumptions re-checked
  against the new interleaved-rib chain shape, given its 3-compounding-bug history under the
  old topology (`D-112-MMU-TOPOLOGY`'s 6th pass, the "busy-hub" investigation).
- **Re-validation of `stitch.rs`/`simplify.rs`/`remove_small.rs`** against now-correctly-closing
  multi-junction lines.
- **Faithfulness invariant suite**: closed-ring outer wall for simple input; quad-chain length
  (2-3 edges); `getNextUnconnected` termination bound; junction-count-delta bound (`<= 1`).
  Selective `test_voronoi.cpp` triage for `voronoi.rs`/`preprocess.rs` fixtures (NOT for
  connectJunctions faithfulness — that layer has zero OrcaSlicer unit tests, confirmed by
  direct search of `OrcaSlicerDocumented/tests/`).
- **Fixture re-baseline** across `crates/slicer-core/tests/fixtures/arachne/*.json` and
  `crates/slicer-runtime/tests/fixtures/perimeter_parity/*` (at minimum: `tapered_wedge`,
  `narrow_strip_widening`, `max_bead_count_cap`, `complex_multi_feature`, `cube_4color_arachne`).
- **`cube_4color_arachne_per_color_footprint_within_bbox` strengthened in place**: keep its
  still-valid structural assertions, replace the weakened bbox-with-tolerance check with a hard
  closure assertion.
- **Deviation log correction**: register `D-113C-FAITHFUL-GRAPH-CONSTRUCTION`, superseding
  `D-112-MMU-TOPOLOGY` and `D-113B-CONNECTJUNCTIONS` via an addendum on each (not an in-place
  edit — per grilling decision, preserves their historical narrative).
- **`docs/adr/0034-arachne-faithful-graph-construction.md`** (NEW): records the architectural
  decision (faithful per-cell + rib port, not an approximation) and the process lesson about
  OrcaSlicer-read delegation losing caller-loop context. Authored alongside packet authoring,
  per grilling decision — not deferred to an implementation step.
- **`CONTEXT.md` glossary additions** (once Step 3/4's Rust shapes settle): central/spine edge,
  rib edge, quad (Arachne), junction fan, domain-start, `getNextUnconnected`.
- **End-to-end re-verification** of `resources/cube_4color.3mf`: the `;TYPE:Outer wall`
  start≈end gcode closure check used to diagnose this bug, formalized as a permanent test.

## Out of Scope

- Building the real `OrcaSlicerDocumented` C++ checkout (CMake+vcpkg+MSVC) to generate true
  oracle golden fixtures — considered and explicitly declined during grilling. Self-captured
  fixtures + invariant tests only, matching every prior arachne packet's precedent.
- Splitting this work into two packets (e.g. 113c core-fix + 113d validation) — considered and
  declined; every step cascades sequentially from Step 3 (unlike 113a/113b's independent-items
  split), so a split adds packet-management overhead without reducing risk.
- Classic-perimeters edits — M1 frozen.
- Spiral-vase and non-planar — orthogonal sibling roadmaps.
- New WIT record changes — no host-service interface changes; this packet's surface is entirely
  `slicer-core` internals (`voronoi.rs`, `skeletal_trapezoidation/*`, `arachne/*`).
- New IR schema types in `slicer-ir` — topology changes are internal to `skeletal_trapezoidation`.

## Authoritative Docs

- `docs/02_ir_schemas.md` — not authoritative; no schema bump needed.
- `docs/08_coordinate_system.md` — range-read §"Constant Conversion Table" only (30 lines);
  purpose: unit conversion for any new per-vertex/per-cell fields.
- `docs/adr/0023-arachne-port-strategy.md` — read full (short); purpose: this packet's Step 3
  must keep honoring the existing degeneracy-handling contract (T-junctions, duplicate
  vertices, near-collinear segments) — it is not being relaxed, only the topology built on top
  of it is changing.
- `docs/adr/0034-arachne-faithful-graph-construction.md` (NEW, authored alongside this packet)
  — read full; purpose: the architectural decision this packet implements.
- `docs/DEVIATION_LOG.md` `D-112-MMU-TOPOLOGY` + `D-113B-CONNECTJUNCTIONS` entries — read full;
  purpose: understand what the prior "Closed" status actually covered (and didn't).

All other docs are not authoritative for this packet.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into
the implementer's own context. Default dispatch contract: return `LOCATIONS` (file:line + 1-line
context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns
are capped at 30 lines.

**Scoped exception for Steps 2 and 3 only** (see `packet.spec.md` for the full rationale): allow
dispatches to return up to 30-line code excerpts of caller-side loop structure in addition to
prose, explicitly asking about calling-loop frequency/structure, not just callee summaries.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:431-560` —
  `constructFromPolygons()` — Steps 2-3 (relaxed dispatch: request caller loop structure)
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:157-257` —
  `transferEdge()` — Step 3
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidationGraph.cpp:452-482` —
  `makeRib()` — Step 3
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidationGraph.cpp:183-193` —
  `getNextUnconnected()` — Step 4
- `OrcaSlicerDocumented/src/libslic3r/Geometry/VoronoiUtils.cpp`
  (`compute_segment_cell_range`/`compute_point_cell_range`) — Steps 2-3
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:2260-2368` —
  `connectJunctions()` — Step 4
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidationGraph.cpp:310-431` —
  `insertRib()`/`insertNode()` — Step 6

## Acceptance Summary

Reference Acceptance Criteria by ID; do not copy them.

- Positive cases: `AC-1` (per-cell metadata), `AC-2` (cell-range spike note), `AC-3` (faithful
  graph construction), `AC-4` (faithful connectJunctions), `AC-5` (centrality/bead_count
  revalidation), `AC-6` (insert_node re-audit), `AC-7` (stitch/simplify/remove_small
  revalidation), `AC-8` (invariant suite + test_voronoi triage), `AC-9` (fixture re-baseline +
  strengthened cube_4color test), `AC-10` (end-to-end cube_4color closure).
- Negative cases: `AC-N1` (square produces multiple ribs, corrects 113b's wrong expectation),
  `AC-N2` (remove_small primary preservation, carried from 113b), `AC-N3` (graph construction
  deterministic, carried from 113b).
- Refinements not captured in Given/When/Then:
  - `rib.rs`'s `build_quad_rib_topology` and its `QuadCell`/`RibData` types are superseded by
    Step 3's construction; the implementer must decide what (if anything) survives as a shared
    type vs. gets deleted. Likely only `EdgeType`/`RibData` type shapes are reused.
  - Step 3 is the single point of failure for Steps 4-10 (all depend on its graph shape). The
    implementer MUST green-gate Step 3's own tests (AC-3, AC-N1, AC-N3) before proceeding.
  - `propagation.rs::insert_node`'s re-audit (Step 6/AC-6) is deliberately a dedicated step, not
    folded into Step 5, because of its documented bug history — treat it with Step-3-level
    rigor even though it's sized M, not L.
  - The deviation-log correction (Doc Impact Statement) uses a supersession pattern (new ID +
    addendum), not in-place edits to `D-112-MMU-TOPOLOGY`/`D-113B-CONNECTJUNCTIONS` — this was
    an explicit user decision during grilling, not a default convention choice.
  - No new ADR-numbering conflict: `docs/adr/0033-host-service-bridge-for-host-only-algorithms.md`
    already exists (created after packet 113b, which had explicitly declined an "ADR-0033" that
    didn't yet exist at the time); this packet's new ADR is `0034`, confirmed as the next free
    number.

## Verification Commands

Full verification matrix. `packet.spec.md` §Verification carries only the gate subset.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-core --features host-algos --test voronoi -- voronoi_cells_square_metadata 2>&1 \| tee target/test-output-voronoi-cells.log` | AC-1: per-cell metadata | FACT pass/fail |
| `rg -q 'Step 2 Spike Findings' .ralph/specs/113c-arachne-faithful-graph-construction/design.md` | AC-2: spike note present | FACT pass/fail |
| `cargo test -p slicer-core --features host-algos --test skeletal_trapezoidation -- square_domain_closes_into_one_ring 2>&1 \| tee target/test-output-graph-construction.log` | AC-3: faithful graph construction | FACT pass/fail |
| `cargo test -p slicer-core --features host-algos --test skeletal_trapezoidation -- square_produces_multiple_ribs 2>&1 \| tee target/test-output-rib-square-neg.log` | AC-N1: square has ribs | FACT pass/fail |
| `cargo test -p slicer-core --features host-algos --test skeletal_trapezoidation -- graph_construction_is_deterministic 2>&1 \| tee target/test-output-deterministic-neg.log` | AC-N3: deterministic | FACT pass/fail |
| `cargo test -p slicer-core --features host-algos --test generate_toolpaths -- outer_wall_closes_for_simple_polygon 2>&1 \| tee target/test-output-connectjunctions.log` | AC-4: faithful connectJunctions | FACT pass/fail |
| `cargo test -p slicer-core --features host-algos --test centrality --test bead_count 2>&1 \| tee target/test-output-centrality-beadcount.log` | AC-5: centrality/bead_count revalidation | FACT pass/fail (fixtures re-baselined) |
| `cargo test -p slicer-core --features host-algos --test propagation -- same_edge_splits_near_rib_insertion 2>&1 \| tee target/test-output-insert-node.log` | AC-6: insert_node re-audit | FACT pass/fail |
| `cargo test -p slicer-core --features host-algos --test stitch --test simplify --test remove_small 2>&1 \| tee target/test-output-downstream.log` | AC-7: downstream revalidation | FACT pass/fail (fixtures re-baselined) |
| `cargo test -p slicer-core --features host-algos --test remove_small -- remove_small_lines_all_primary_invariant 2>&1 \| tee target/test-output-remove-neg.log` | AC-N2: primary preservation | FACT pass/fail |
| `cargo test -p slicer-core --features host-algos --test arachne_invariants 2>&1 \| tee target/test-output-invariants.log` | AC-8: invariant suite + triage | FACT pass/fail |
| `cargo test -p slicer-runtime --test executor -- cube_4color_arachne 2>&1 \| tee target/test-output-cube4color-strengthened.log` | AC-9: strengthened test + fixtures | FACT pass/fail |
| `cargo test -p slicer-runtime --test executor -- cube_4color_arachne_outer_walls_close_end_to_end 2>&1 \| tee target/test-output-e2e-closure.log` | AC-10: end-to-end closure | FACT pass/fail |
| `rg -q 'D-113C-FAITHFUL-GRAPH-CONSTRUCTION' docs/DEVIATION_LOG.md && rg -q 'Superseded.*D-113C-FAITHFUL-GRAPH-CONSTRUCTION' docs/DEVIATION_LOG.md` | Deviation log correction | FACT pass/fail |
| `rg -q '### Rib edge' CONTEXT.md` | Glossary update | FACT pass/fail |
| `cargo check --workspace --all-targets` | Cross-crate compile | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Clippy gate | FACT pass/fail |
| `cargo xtask build-guests --check` | Guest WASM coherence | FACT clean / STALE list |
| `cargo xtask test --workspace --summary 2>&1 \| tee target/test-output.log` | Final closure gate | FACT pass/fail + summary line + count |

All verification commands are delegation-friendly.

## Step Completion Expectations

Cross-step invariants the per-step blocks in `implementation-plan.md` cannot express:

- **Step 2 (spike) gates Step 3's design.** The written note must exist and answer both open
  questions before Step 3's implementation begins — this is a design gate, not a code gate.
- **Step 3 (faithful graph construction) is the single point of failure for Steps 4-10.** The
  implementer MUST run Step 3's own tests (AC-3, AC-N1, AC-N3) and confirm CLEAN before
  proceeding to Step 4. If Step 3 produces incorrect topology, every downstream step fails.
- **Step 6 (`insert_node` re-audit) is dedicated, not folded into Step 5**, per explicit user
  decision during grilling — its bug history (3 compounding defects under the old topology)
  warrants its own gated exit condition separate from `centrality.rs`/`bead_count.rs`'s
  lower-risk revalidation.
- **Step 9's fixture re-baseline is atomic and must record rationale.** Once Step 4 (faithful
  `connectJunctions`) lands, every affected fixture re-baselines in one batch; the implementer
  MUST record the re-baselining rationale in each fixture's own commit message.
- **Step 9's deviation-log correction uses the supersession pattern, not in-place edits** — a
  new `D-113C-FAITHFUL-GRAPH-CONSTRUCTION` entry plus a one-line addendum on each of the two old
  entries. Do not rewrite `D-112-MMU-TOPOLOGY`'s or `D-113B-CONNECTJUNCTIONS`'s existing
  narrative text.
- **Step 10 (workspace gate + end-to-end verification) is the final gate.** The implementer
  MUST run `cargo xtask test --workspace --summary` and confirm green, AND confirm the
  `cube_4color.3mf` end-to-end closure check passes, before flipping `packet.spec.md` to
  `status: implemented`.
- **The ADR (`docs/adr/0034-...md`) is authored alongside packet authoring**, not as an
  implementation step — it is a guardrail that should exist before Step 1 begins, per explicit
  user decision during grilling.

## Context Discipline Notes

Packet-specific context hazards:

- `crates/slicer-core/src/skeletal_trapezoidation/graph.rs` is the primary edit target for
  Steps 1 and 3 — can be full-read; it is small (~700 lines) and central to this packet.
- `crates/slicer-core/src/skeletal_trapezoidation/propagation.rs` (632 LOC) is the primary edit
  target for Step 6 — can be full-read for that step only; out-of-bounds for Steps 1-5.
- `crates/slicer-core/src/arachne/generate_toolpaths.rs` (~975 LOC) is the primary edit target
  for Step 4 — can be full-read for that step only.
- The 6 OrcaSlicer dispatches (`constructFromPolygons`, `transferEdge`, `makeRib`,
  `getNextUnconnected`, `compute_segment_cell_range`/`compute_point_cell_range`,
  `connectJunctions`, `insertRib`/`insertNode`) are the heaviest dispatches in the packet. Steps
  2-3's dispatches use the relaxed (code-excerpt-permitted) contract; Steps 4/6/8 use the
  default SUMMARY-only contract since Step 3 will have already surfaced the caller structure.
- Tempting reads to skip: `modules/core-modules/arachne-perimeters/src/lib.rs` (not edited by
  this packet — the per-region call structure is unaffected), `crates/slicer-sdk/src/host.rs`
  (no WIT changes), `crates/slicer-schema/wit/deps/common.wit` (no changes).
- `crates/slicer-runtime/tests/fixtures/perimeter_parity/*` fixtures are large JSON files
  (`expected_perimeter_ir.json` can exceed 10MB per the cube_4color fixture) — never read these
  directly; always re-record via their documented `#[ignore]`d `record_*` functions.

If none apply, write `None packet-specific.`
