# Design: 113c-arachne-faithful-graph-construction

## Controlling Code Paths

- **Primary code path 1:** `crates/slicer-core/src/voronoi.rs` — expose per-cell metadata
  (`VCell`, `HalfEdgeGraph::cells`). Step 1.
- **Primary code path 2:** `crates/slicer-core/src/skeletal_trapezoidation/graph.rs` —
  `SkeletalTrapezoidationGraph::from_polygons`, rewritten to build the real per-cell chain +
  interleaved-rib topology. Step 3. **The single point of failure for the entire packet.**
- **Primary code path 3:** `crates/slicer-core/src/skeletal_trapezoidation/rib.rs` — superseded;
  `build_quad_rib_topology`'s reflex-corner-only pass is deleted, `EdgeType`/`RibData` type
  shapes likely reused by path 2's new constructor. Step 3.
- **Primary code path 4:** `crates/slicer-core/src/arachne/generate_toolpaths.rs` — faithful
  `connectJunctions` port replacing the central-only `walk_domain_chain`. Step 4.
- **Primary code path 5:** `crates/slicer-core/src/skeletal_trapezoidation/centrality.rs` +
  `bead_count.rs` — re-validation against the new graph shape. Step 5.
- **Primary code path 6:** `crates/slicer-core/src/skeletal_trapezoidation/propagation.rs`
  (specifically `insert_node`) — dedicated re-audit given its bug history. Step 6.
- **Re-validation targets:** `crates/slicer-core/src/arachne/{stitch,simplify,remove_small}.rs`
  — Step 7.
- **Test/fixture targets:** `crates/slicer-core/tests/fixtures/arachne/*.json`,
  `crates/slicer-runtime/tests/fixtures/perimeter_parity/*`,
  `crates/slicer-runtime/tests/executor/cube_4color_arachne.rs`. Steps 8-10.
- **OrcaSlicer comparison surface:** see `requirements.md` §OrcaSlicer Reference Obligations
  (delegate; Steps 2-3 use the relaxed code-excerpt-permitted contract, others stay
  SUMMARY-only).

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this
  packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"),
  the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported,
  rebuild without `--check` before re-running the failing test. Note: this packet's change
  surface is entirely host-side (`slicer-core`); no WIT or module edits, so guest staleness is
  not expected. The freshness check is run as a precaution, same as packet 113b.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer
  constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary.
  Full porting checklist in `docs/08_coordinate_system.md`.

- **Degeneracy-handling contract stays in force.** `docs/adr/0023-arachne-port-strategy.md`'s
  degeneracy table (collinear input, T-junctions, duplicate vertices, near-collinear-within-
  `epsilon_offset` segments) is a `preprocess.rs`-level contract that Step 3's new construction
  must keep honoring — this packet changes how the graph is built ON TOP of already-preprocessed
  segments, not the preprocessing contract itself.

- **Single point of failure:** Step 3 (faithful graph construction) is the only structural
  dependency for Steps 4-10. If Step 3 produces incorrect topology, every downstream step fails.
  The implementer MUST run Step 3's own tests (AC-3, AC-N1, AC-N3) and confirm CLEAN before
  proceeding to Step 4.

- **No schema bump:** topology changes are internal to `skeletal_trapezoidation`/`arachne`, not
  `slicer-ir`. `ExtrusionLine`/`ExtrusionJunction` shapes from packet 112 are unchanged.

## Verified Algorithm Mechanics (pre-seeded — do not re-derive from scratch)

Extracted via direct source reads during the `/diagnose` session that produced this packet
(justified there as root-cause diagnosis, not packet-authoring delegation). Implementers should
treat this section as a trusted SUMMARY-equivalent and only re-dispatch against
`OrcaSlicerDocumented/` for details this section doesn't cover.

**Graph construction (`constructFromPolygons` / `transferEdge` / `makeRib`):**

Real OrcaSlicer iterates the raw Voronoi diagram's **cells** (not its raw DCEL edges/next/prev
directly) — both point-cells (at polygon vertices) and segment-cells (at polygon edges). For
each cell, it determines a `starting_voronoi_edge..ending_voronoi_edge` range (via
`compute_point_cell_range`/`compute_segment_cell_range` — the open question for Step 2's spike
is how much of this range-computation logic is actually needed versus a raw incident-edge cycle
walk) and calls `transferEdge()` once per raw Voronoi edge in that range.

`transferEdge` creates a **fresh** `edge_t` (this crate's `STHalfEdge`) per raw Voronoi edge,
chaining it via `edge->prev = prev_edge; prev_edge->next = edge;` — i.e. it builds a NEW local
chain, not a copy of `boostvoronoi`'s own next/prev. Critically, `makeRib(prev_edge, ...)` is
called after **every** transferred edge (not just at reflex/sharp corners, which is what this
codebase's current `rib.rs` assumes). `makeRib`:

```
forth_edge->from = prev_edge->to;   // spine node
forth_edge->to   = node;            // new boundary node (perpendicular foot)
forth_edge->twin = back_edge;
back_edge->from  = node;
back_edge->to    = prev_edge->to;   // back to same spine node
prev_edge = back_edge;              // caller's cursor is reassigned to back_edge
```

Because `prev_edge` is reassigned to `back_edge` (not left on the spine edge), the actual built
chain **interleaves**: `spine1 -> forth_rib1` (forth_rib1.next is never set — a dead end) as one
sub-chain, and a separate sub-chain `back_rib1 -> spine2 -> forth_rib2` (dead end), etc.
`forth_rib1.twin == back_rib1`. When `transferEdge` encounters a raw Voronoi edge whose twin was
already transferred by a neighboring cell, it walks `twin->prev->twin->prev` to mirror-construct
the matching interleaved chain on this side — this is how adjacent cells' chains get cross-linked
via `.twin` on the shared raw Voronoi edge.

**`getNextUnconnected()`** (`SkeletalTrapezoidationGraph.cpp:183-193`): walk `.next` until
reaching an edge whose `.next` is null (a dead end), then return **that edge's `.twin`**.
Because of the rib-interleaving above, this naturally continues from `spine1`'s dead-end
(`forth_rib1`) to `back_rib1` (its twin), which continues to `spine2` — the interleaved-rib
construction is exactly what makes this correctly traverse real N-way branch/junction vertices:
adjacent cells' chains are cross-linked via `.twin`, so there is never an ambiguous "which
sibling edge continues" choice — it is always a simple twin-hop.

**`connectJunctions()`** (`SkeletalTrapezoidation.cpp:2260-2368`): seeds
`unprocessed_quad_starts` = every edge with `!edge.prev` (this naturally includes every
`back_rib` edge, since `back_edge->prev` is never assigned by `makeRib`). For each unprocessed
start, walks `.next` to find `quad_end` — a **short 2-3 edge run**: `back_ribₖ -> spineₖ₊₁ ->
forth_ribₖ₊₁`. Finds the max-R edge within that quad (`getQuadMaxRedgeTo`), stitches that quad's
junctions onto the currently-running `ExtrusionLine` (via `addToolpathSegment`, using a
`new_domain_start` flag to know whether to start fresh or continue), then advances
`quad_start = quad_start->getNextUnconnected()` and repeats until back at `poly_domain_start`
(full ring closure) or exhausted. This is a **fine-grained, quad-by-quad progressive stitch** —
NOT "grab the whole domain via next/prev then emit once," which is what this codebase's current
`process_central_domain`/`walk_domain_chain` attempt (and why it breaks: it filters each hop by
requiring the target be central/non-`EXTRA_VD`, which fails immediately once ribs are properly
interleaved after every edge).

**`insertRib`/`insertNode`** (`SkeletalTrapezoidationGraph.cpp:310-431`, relevant to Step 6):
splits an existing edge at a transition position by inserting a rib pair, patching
`next`/`prev`/`twin` across both the original edge and its twin. This is the exact mechanism
whose incorrect rewiring (stale next/prev pointers not rewired on repeated same-edge splits,
`twin` overwritten to point at the wrong endpoint, twin-mirroring pushed onto the wrong edge's
list) caused the 3 compounding "busy-hub" bugs found under the OLD topology
(`docs/DEVIATION_LOG.md` `D-112-MMU-TOPOLOGY`'s 6th pass). Step 6 must re-verify this logic
against the NEW interleaved-rib chain shape, not assume the prior fix generalizes.

**`boostvoronoi` cell API** (verified directly against the vendored crate source,
`~/.cargo/registry/src/.../boostvoronoi-0.12.1/src/diagram/cell_impl.rs`, v0.12.1 — matches
`crates/slicer-core/Cargo.toml`'s pin): `Cell::contains_point()`/`contains_segment()`/
`contains_segment_startpoint()`/`contains_segment_endpoint()`/`source_category()`/
`source_index()`/`get_incident_edge()`/`is_degenerate()` are all present and public. `Diagram`
exposes `cells()`. No crate patching or vendoring needed for Step 1.

**Provenance without a `Segment` struct change** (verified directly against
`boostvoronoi::Builder::with_segments`, `src/builder.rs`): `source_index()` reflects a monotonic
counter (`self.index_`) assigned in **original input iteration order**, incremented once per
input segment, BEFORE `init_sites_queue()`'s internal sweep-line sort. This means a side table
built at flatten-time (mapping flattened-segment-index → ring id / point id, since
`ring_segments` already pushes segments in polygon-ring order) can supply provenance without
touching `voronoi.rs`'s public `Segment{a,b}` type. **Caveat:** each input segment pushes 3 site
events (start-point, end-point, segment) sharing that one index, and `init_sites_queue()`'s
sort+`dedup()` can collapse two adjacent segments' shared-vertex point-site events into one — so
a point-cell's surviving `source_index()` isn't deterministically "always segment i" vs "segment
i+1." This is not believed to be a blocker (OrcaSlicer's own point-cell-range logic resolves
point cells by coordinate + polygon-neighbor navigation, not by trusting a specific segment
index) but Step 2's spike must confirm this empirically before Step 3 relies on it.

## Code Change Surface

- **Selected approach:** Extend `voronoi.rs` with cell metadata, spike the cell-range-walk
  question, rewrite graph construction faithfully (the L step), rework `connectJunctions`,
  re-validate the three downstream passes (centrality/bead_count, then a dedicated
  `insert_node` re-audit, then stitch/simplify/remove_small), build a faithfulness invariant
  suite, re-baseline fixtures + correct the deviation log + glossary, then verify end-to-end.
- **Exact functions, traits, manifests, tests, or fixtures expected to change:**
  - `crates/slicer-core/src/voronoi.rs` (ADD `VCell` struct, `HalfEdgeGraph::cells: Vec<VCell>`)
  - `crates/slicer-core/src/skeletal_trapezoidation/graph.rs` (REWRITE `from_polygons`; likely
    ADD a provenance side-table type and a per-cell walk helper)
  - `crates/slicer-core/src/skeletal_trapezoidation/rib.rs` (DELETE `build_quad_rib_topology`
    and `QuadCell`; KEEP/relocate `EdgeType`/`RibData` type shapes if still needed by the new
    constructor — decide during Step 3)
  - `crates/slicer-core/src/skeletal_trapezoidation/mod.rs` (update `pub use`/`pub mod` as
    `rib.rs`'s surface shrinks)
  - `crates/slicer-core/src/arachne/generate_toolpaths.rs` (REPLACE
    `walk_domain_chain`/`process_central_domain`/`is_domain_start`/`is_domain_edge` with a
    faithful `connectJunctions`/`getNextUnconnected`/`getQuadMaxRedgeTo` port)
  - `crates/slicer-core/src/skeletal_trapezoidation/centrality.rs` (re-validate `EdgeType::
    EXTRA_VD` exclusion against ubiquitous ribs; adjust if needed)
  - `crates/slicer-core/src/skeletal_trapezoidation/bead_count.rs` (re-validate; likely minimal
    change — no direct rib/quad field references per a grep)
  - `crates/slicer-core/src/skeletal_trapezoidation/propagation.rs` (re-audit `insert_node`
    specifically; targeted fixes only)
  - `crates/slicer-core/src/arachne/{stitch,simplify,remove_small}.rs` (re-validate; minimal
    code change expected, mostly fixture re-baseline)
  - `crates/slicer-core/tests/{voronoi,skeletal_trapezoidation,centrality,bead_count,
    propagation,generate_toolpaths,stitch,simplify,remove_small,arachne_invariants}.rs` (NEW
    tests per step; existing test files updated for re-baselined fixtures)
  - `crates/slicer-core/tests/fixtures/arachne/*.json` (RE-BASELINE, at minimum the files
    packet 113b's own AC-9 already tracked, plus any new ones this rewrite touches)
  - `crates/slicer-runtime/tests/fixtures/perimeter_parity/*` (RE-BASELINE: `tapered_wedge`,
    `narrow_strip_widening`, `max_bead_count_cap`, `complex_multi_feature`, `cube_4color_arachne`
    — per `docs/DEVIATION_LOG.md`'s "12th pass" note on packet 113b's own closure)
  - `crates/slicer-runtime/tests/executor/cube_4color_arachne.rs` (STRENGTHEN
    `cube_4color_arachne_per_color_footprint_within_bbox` in place; ADD a new
    `cube_4color_arachne_outer_walls_close_end_to_end` test)
  - `docs/DEVIATION_LOG.md` (REGISTER `D-113C-FAITHFUL-GRAPH-CONSTRUCTION`; APPEND addendum to
    `D-112-MMU-TOPOLOGY` and `D-113B-CONNECTJUNCTIONS`)
  - `docs/adr/0034-arachne-faithful-graph-construction.md` (NEW — authored alongside packet
    authoring, not an implementation step)
  - `CONTEXT.md` (ADD glossary entries, once Step 3/4's Rust shapes settle)
  - `docs/01_system_architecture.md`, `docs/specs/perimeter-modules-orca-parity-roadmap.md`
    (update M2-faithful marker)

- **Rejected alternatives:**
  - **Build the real OrcaSlicer C++ checkout to generate oracle golden fixtures.** Rejected
    (explicit user decision during grilling): a multi-hour CMake+vcpkg+MSVC infra lift with no
    precedent in this project's prior arachne packets; self-captured fixtures + invariant tests
    derived from the C++ source's own documented asserts are accepted as sufficient, matching
    every prior packet.
  - **Split into 113c (core fix) + 113d (validation/cleanup).** Rejected (explicit user
    decision during grilling): every step cascades sequentially from Step 3 — unlike packets
    113a/113b, there are no independent items that could ship in parallel — so a split adds
    packet-management overhead without reducing risk.
  - **Edit `D-112-MMU-TOPOLOGY`/`D-113B-CONNECTJUNCTIONS` in place instead of superseding.**
    Rejected (explicit user decision during grilling): a new ID + addendum preserves the
    existing historical narrative rather than rewriting it.
  - **Fold `propagation.rs::insert_node`'s re-audit into Step 5.** Rejected (explicit user
    decision during grilling): its 3-compounding-bug history under the old topology warrants a
    dedicated gated step, not being merged into the broader (lower-risk)
    centrality/bead_count revalidation.
  - **Add explicit `ring_id`/`point_idx` fields to `Segment`.** Rejected: `source_index()` +
    a flatten-time side table supplies the same provenance with zero changes to `voronoi.rs`'s
    public `Segment` type — a narrower, less invasive change (verified safe during grilling; see
    the dedup-ambiguity caveat above, which Step 2's spike must still confirm empirically).

## Files in Scope (read + edit)

- `crates/slicer-core/src/voronoi.rs` — Step 1 primary edit target
- `crates/slicer-core/src/skeletal_trapezoidation/graph.rs` — Steps 1 + 3 primary edit target
- `crates/slicer-core/src/skeletal_trapezoidation/rib.rs` — Step 3 (superseded; delete/relocate)
- `crates/slicer-core/src/skeletal_trapezoidation/mod.rs` — Step 3 (module surface update)
- `crates/slicer-core/src/arachne/generate_toolpaths.rs` — Step 4 primary edit target
- `crates/slicer-core/src/skeletal_trapezoidation/centrality.rs` — Step 5 primary edit target
- `crates/slicer-core/src/skeletal_trapezoidation/bead_count.rs` — Step 5 primary edit target
- `crates/slicer-core/src/skeletal_trapezoidation/propagation.rs` — Step 6 primary edit target
- `crates/slicer-core/src/arachne/{stitch,simplify,remove_small}.rs` — Step 7 re-validation
- `crates/slicer-runtime/tests/executor/cube_4color_arachne.rs` — Steps 9-10

## Read-Only Context

- `crates/slicer-core/src/arachne/pipeline.rs` — read `run_arachne_pipeline` (the orchestrator
  call order) to confirm no changes needed there beyond what Steps 3-4 already imply
- `crates/slicer-core/src/skeletal_trapezoidation/discretize.rs` — read in full (small); Step 3
  needs to confirm this curved-edge-discretization piece is reusable inside the new per-cell walk
- `crates/slicer-core/src/beading/mod.rs` — read `BeadingStrategy` trait only (unaffected by
  this packet, but Steps 5-6 call into it)
- `.ralph/specs/113b-arachne-topology-faithfulness/{packet.spec.md,requirements.md,design.md,
  implementation-plan.md,task-map.md}` — read in full; this packet directly supersedes 113b's
  Step 1 (quad/rib pass) and Step 5 (`connectJunctions`)
- `docs/DEVIATION_LOG.md` `D-112-MMU-TOPOLOGY` + `D-113B-CONNECTJUNCTIONS` entries — read in
  full before drafting the Step 9 addendum
- `CONTEXT.md` — read in full (short) before adding Step 9's glossary entries, to match its
  existing "definitions only" format

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` — delegate parity checks (relaxed contract for Steps 2-3, default
  SUMMARY-only otherwise); never load directly
- `target/`, `Cargo.lock`, generated code — never load
- `modules/core-modules/arachne-perimeters/src/lib.rs` — not edited; the per-region call
  structure (`output.begin_region(...)`) is unaffected by this packet
- `crates/slicer-sdk/src/host.rs` — not edited (no WIT changes)
- `crates/slicer-schema/wit/deps/common.wit` — not edited (no WIT changes)
- `crates/slicer-runtime/tests/fixtures/perimeter_parity/*/expected_perimeter_ir.json` — never
  read directly (can exceed 10MB); always re-record via documented `#[ignore]`d `record_*`
  functions

## Expected Sub-Agent Dispatches

- (Steps 2-3, relaxed contract) "Summarize `OrcaSlicerDocumented/src/libslic3r/Arachne/
  SkeletalTrapezoidation.cpp:431-560` `constructFromPolygons()`, including up to a 30-line code
  excerpt of the per-cell loop and its `makeRib` call sites. Explicitly describe how often
  `makeRib` is called relative to `transferEdge` and whether that frequency is unconditional.
  ≤ 200 words prose + the excerpt." — purpose: confirm/extend this design's pre-seeded mechanics
  before Step 3 begins
- (Steps 2-3, relaxed contract) "Summarize `OrcaSlicerDocumented/src/libslic3r/Geometry/
  VoronoiUtils.cpp`'s `compute_segment_cell_range`/`compute_point_cell_range`, with up to a
  30-line excerpt of the range-finding loop body. Does a raw `incident_edge → next → …` cycle
  walk on this crate's own `boostvoronoi` wrapper give an equivalent range without this
  additional logic?" — purpose: resolve Step 2's spike question (a)
- (Step 3, relaxed contract) "Summarize `OrcaSlicerDocumented/src/libslic3r/Arachne/
  SkeletalTrapezoidationGraph.cpp:452-482` `makeRib()` with up to a 30-line excerpt, confirming
  the `prev_edge` cursor reassignment to `back_edge` and how degenerate zero-length edges (at
  input-segment endpoints) are handled." — purpose: design Step 3's rib-insertion logic
- (Step 4, default SUMMARY-only) "Summarize `OrcaSlicerDocumented/src/libslic3r/Arachne/
  SkeletalTrapezoidation.cpp:2260-2368` `connectJunctions()`; return SUMMARY (≤ 200 words:
  `unprocessed_quad_starts` seeding, `getQuadMaxRedgeTo`, the `new_domain_start`-flagged
  progressive stitch, odd-single-bead suppression via `passed_odd_edges`). No code." — purpose:
  confirm this design's pre-seeded mechanics before Step 4 begins
- (Step 6, default SUMMARY-only) "Summarize `OrcaSlicerDocumented/src/libslic3r/Arachne/
  SkeletalTrapezoidationGraph.cpp:310-431` `insertRib()`/`insertNode()`; return SUMMARY (≤ 200
  words: twin-severing before split, cross-twin patching, `transition_ratio` initialization).
  No code." — purpose: ground Step 6's re-audit
- "Run `cargo test -p slicer-core --features host-algos --test <name>`; return FACT pass/fail."
  — purpose: validate each step's narrow gate (repeated per step, per `packet.spec.md`'s
  Verification commands)
- "Run `cargo xtask build-guests --check`; return FACT clean / STALE list." — purpose: guest
  WASM coherence (precaution; this packet's surface is host-only)
- "Run `cargo xtask test --workspace --summary 2>&1 | tee target/test-output.log`; return FACT
  pass/fail + summary line + count." — purpose: final workspace gate

## Data and Contract Notes

- **`VCell` is additive**, mirroring `boostvoronoi::Cell` — no existing `HalfEdge`/`Vertex`
  fields change shape in Step 1.
- **`STHalfEdge`'s `next`/`prev`/`twin` semantics change meaning in Step 3**: they no longer
  mirror the raw `boostvoronoi` DCEL 1:1 (as the current doc comment claims) — they encode the
  freshly-constructed per-cell-chain-with-interleaved-ribs topology instead. Any code outside
  this packet's scope that assumed the old "1:1 mirror" semantics (there should be none, since
  `graph.rs` is the sole consumer of the raw `voronoi.rs` output) must be checked during Step 3.
- **No IR schema bump**: `ExtrusionLine`/`ExtrusionJunction` from packet 112 are unchanged by
  this packet — only their upstream construction path changes.
- **No WIT changes**: this packet's entire surface is `slicer-core` internals.
- **Determinism**: the new graph construction must remain deterministic (AC-N3, carried forward
  from packet 113b's AC-N4) — iterate cells/edges in stable index order, no `HashMap` iteration
  order dependence.

## Locked Assumptions and Invariants

- The per-cell chain + interleaved-rib construction is a faithful port of OrcaSlicer's
  `constructFromPolygons`/`transferEdge`/`makeRib` — verified via the relaxed-contract dispatches
  in Steps 2-3 (which MAY include code excerpts) plus this design's own pre-seeded mechanics
  (see §Verified Algorithm Mechanics), and via code review.
- `getNextUnconnected`'s dead-end-then-twin mechanism requires no per-hop centrality filtering —
  unlike the current (incorrect) `walk_domain_chain`, which filters every hop by
  `edge_junctions.contains_key` and therefore breaks at every rib.
- The 2-3 edge "quad" granularity in `connectJunctions` is real, not an implementation
  convenience — `getQuadMaxRedgeTo` operates within one such short quad per iteration, not
  across a whole domain.
- Fixture re-baselining across Steps 5-9 is accepted as self-captured regression-locking, per
  the established `D-112-SELFCAPTURED-BASELINES`/`D-109-SELF-CAPTURED-FIXTURES` precedent — no
  OrcaSlicer binary exists to validate output against. The algorithm-faithfulness criterion is
  asserted via direct OrcaSlicer code references and the invariant suite (AC-8), not output
  match.
- `remove_small_lines`'s primary preservation invariant (`is_closed && inset_idx == 0` never
  removed) must survive the topology change unchanged (AC-N2, carried forward from packet 113b).

## Risks and Tradeoffs

- **Single point of failure (Step 3):** if the per-cell + rib construction is wrong, every
  downstream step fails. Mitigation: AC-3 + AC-N1 + AC-N3 are the cheapest early failure
  checks (does a square close into one ring? does it have ribs? is it deterministic?) — if any
  fails, Step 3 is broken and Step 4 cannot proceed. Internal (non-gated) checkpoints inside
  Step 3 further de-risk this (see `implementation-plan.md` Step 3).
- **Cell-range-walk complexity underestimated:** if Step 2's spike finds that a raw
  incident-edge cycle walk does NOT suffice (i.e. `compute_point_cell_range`/
  `compute_segment_cell_range`'s extra logic is genuinely required), Step 3's scope grows.
  Mitigation: absorb this inside Step 3's internal checkpoints rather than adding a new packet
  step (per this packet's own grilling decision).
- **`insert_node`'s bug history:** three compounding DCEL bugs were found here under the OLD
  topology. Mitigation: Step 6 is a dedicated, gated step with its own regression test (≥2
  same-edge splits near a rib insertion), not folded into Step 5's lower-risk revalidation.
- **Fixture re-baselining locks in undetected port bugs:** with no OrcaSlicer oracle, a subtle
  faithfulness bug could get re-baselined as the new "expected" output. Mitigation: the
  invariant suite (AC-8) checks properties that hold regardless of the specific geometry
  (closure, quad length, junction-count delta) — these cannot be satisfied by a re-baselined
  bug the way a plain snapshot-diff could be.
- **`propagation.rs`/`centrality.rs` coupling to the old rib model:** `centrality.rs` directly
  references `EdgeType::EXTRA_VD`; if its exclusion logic assumed ribs were rare (corner-only),
  it may need adjustment now that ribs are ubiquitous. Mitigation: Step 5's re-validation is
  explicitly scoped to check this, not just re-run existing tests.
- **Deviation log supersession pattern is new to this project** (prior packets always closed
  in place). Mitigation: documented explicitly in `packet.spec.md`'s Doc Impact Statement and
  this design.md, so a future reader understands why `D-112-MMU-TOPOLOGY`/
  `D-113B-CONNECTJUNCTIONS` still show `Closed` with a pointer forward, rather than being
  edited to `Reopened`.

## Context Cost Estimate

- **Aggregate (sum across all 10 steps):** L. The synthetic per-cell graph construction (Step
  3) is genuinely L effort — the implementer should plan to allocate significant focused time,
  with a clear go/no-go decision point at the end (AC-3 + AC-N1 + AC-N3 green → continue to
  Step 4; any fail → stop and report; do NOT continue past a broken Step 3).
- **Largest single step:** Step 3 — L. The riskiest step in the entire packet and the single
  point of failure for Steps 4-10.
- **L-step exception:** documented in `packet.spec.md` §Prerequisites and Blockers, re-confirmed
  against packet 113b's own precedent during this packet's grilling session. If subsequent
  design work surfaces a natural split point, the packet SHOULD be split before Step 3 begins.
- **Highest-risk dispatches:** the Steps 2-3 relaxed-contract dispatches (`constructFromPolygons`,
  `makeRib`, `compute_segment_cell_range`/`compute_point_cell_range`) — their code excerpts
  drive the design of the new `from_polygons`, which gates everything else.

## Step 2 Spike Findings

**(a) Does a raw `incident_edge → next → …` cycle walk suffice for cell-range determination?**

No. Confirmed via direct read of `OrcaSlicerDocumented/src/libslic3r/Geometry/VoronoiUtils.cpp`'s
`compute_segment_cell_range`/`compute_point_cell_range` (`VoronoiUtils.cpp:292-317`). Both
functions internally perform the same do-while ring traversal a naive cycle walk would use, but
layer three things on top a bare walk lacks:

1. **Filtering**: infinite edges are skipped (segment-cell case) or reject the whole cell
   (point-cell case, if the first edge is infinite or has out-of-range coords).
2. **Polygon-membership gate (point-cells only)**: `is_point_inside_polygon_corner()`
   geometrically tests whether the cell is inside the input polygon using the source point's
   polygon-neighbor vertices; cells outside the polygon return an empty range — a raw walk has
   no such concept.
3. **Boundary sub-arc selection, not full enumeration**: the loop's actual job is finding two
   specific edges (`edge_begin`/`edge_end`) via vertex-coordinate comparison against the source
   segment's endpoints, with `seen_possible_start`/`after_start`/`ending_edge_is_set_before_start`
   bookkeeping to disambiguate duplicate/degenerate vertex matches. The returned range is a
   strict sub-arc excluding the side edges coincident with the input segment/point, not the
   cell's whole edge cycle.

```cpp
bool                 seen_possible_start             = false;
bool                 after_start                     = false;
bool                 ending_edge_is_set_before_start = false;
const VD::edge_type* edge                            = cell.incident_edge();
do {
    if (edge->is_infinite())
        continue;
    Vec2i64 v0 = Geometry::VoronoiUtils::to_point(edge->vertex0());
    Vec2i64 v1 = Geometry::VoronoiUtils::to_point(edge->vertex1());
    if (v0 == to_i64 && !after_start) {
        cell_range.edge_begin = edge;
        seen_possible_start   = true;
    } else if (seen_possible_start) {
        after_start = true;
    }
    if (v1 == from_i64 && (!cell_range.edge_end || ending_edge_is_set_before_start)) {
        ending_edge_is_set_before_start = !after_start;
        cell_range.edge_end             = edge;
    }
} while (edge = edge->next(), edge != cell.incident_edge());
```

`constructFromPolygons` calls one of these functions once per cell (each doing its own full
internal ring traversal), then calls `transferEdge()` only across the narrowed
`edge_begin..edge_end` sub-range — not the full ring.

**Design consequence for Step 3:** Step 3's per-cell walk MUST replicate this narrowing logic
(vertex-coordinate boundary matching against the source segment/point's own endpoints, plus the
point-cell polygon-membership gate), not a bare `get_incident_edge()`-cycle enumeration. A raw
cycle walk over-includes edges (the "side" edges of the cell coincident with the input geometry)
that real OrcaSlicer deliberately excludes from the transferred range.

**(b) Does the `source_index()` shared-vertex dedup ambiguity break provenance resolution?**

Confirmed empirically on the 10mm square fixture (4 segments, 4 shared vertices), and the
ambiguity is non-uniform in a specific, predictable way: `init_sites_queue()`'s dedup collapses
each shared-vertex pair of point-site events into one surviving cell (4 point-cells observed for
4 vertices, not 8), and the surviving `source_index()`/`source_category()` always resolves to the
**lower of the two adjacent segment indices** — NOT "always the previous segment" or "always the
next segment" in ring-adjacency terms. For 3 of 4 vertices this coincides with "the previous
(ending) segment wins" (`SegmentEnd`), but at the wraparound vertex `(0,0)`, the "previous"
segment (seg3, higher index) loses to the "next" segment (seg0, lower index, `SegmentStart`) —
because raw input-array index, not ring position, determines the winner. Code assuming a uniform
"previous segment always wins at a shared vertex" rule would silently misresolve exactly at the
polygon's wraparound seam.

Per-cell cycle shape also confirmed on this fixture: point-cells have cycle length 2 (100%
curved edges), segment-cells have cycle length 4 (2 primary + 2 curved), totaling 24 edges across
8 cells with zero double-counting.

**Design consequence for Step 3:** the flatten-time provenance side table (mapping
flattened-segment-index → ring id / point id) MUST NOT assume "point-cell `source_index()` ==
previous-segment-index-in-ring-order" as a blanket rule. It must either (i) special-case the
wraparound seam explicitly (compare `source_index()` against both ring-adjacent segment indices
and accept whichever matches), or (ii) resolve point-cell provenance by coordinate match against
ring vertices (matching real OrcaSlicer's own point-cell-range resolution strategy, which
navigates by coordinate + polygon-neighbor, not by trusting a specific segment index) rather than
by `source_index()` alone. **Option (ii) is recommended** since it sidesteps the ambiguity
entirely rather than special-casing it.

Both open questions below are resolved by the above; Step 3 may proceed.

## Open Questions

- [RESOLVED — see §Step 2 Spike Findings (a)] Does a raw `incident_edge → next → …` cycle walk on
  this crate's `boostvoronoi` wrapper give an equivalent cell-range to OrcaSlicer's
  `compute_point_cell_range`/`compute_segment_cell_range`, or is additional logic required?
- [RESOLVED — see §Step 2 Spike Findings (b)] Does the `source_index()` shared-vertex dedup
  ambiguity (documented in §Verified Algorithm Mechanics) actually surface in practice for this
  crate's fixtures, and does the side-table design need an explicit multi-valued lookup to
  tolerate it?
- [FWD] Which (if any) of `test_voronoi.cpp`'s degenerate-input Catch2 cases are worth porting
  as `voronoi.rs`/`preprocess.rs` regression fixtures? Resolve during Step 8's triage — this is
  explicitly NOT required for connectJunctions faithfulness (that layer has zero OrcaSlicer unit
  tests), only for hardening the layer below.
- [FWD] Does `centrality.rs`'s `EdgeType::EXTRA_VD` exclusion logic need adjustment now that
  ribs are ubiquitous (not corner-only)? Resolve during Step 5.
- [BLOCK] None — no other packet currently holds `status: active`; this packet activates
  immediately.
