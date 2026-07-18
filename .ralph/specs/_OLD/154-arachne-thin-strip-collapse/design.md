# Design: 154-arachne-thin-strip-collapse

> **Investigation packet.** No fix is prescribed. The candidate set below is grounded — every
> OrcaSlicer claim carries a verified `file:line`, and every PnP symbol named has been confirmed
> to exist. Do not implement until `§Step 1 Findings` names the responsible mechanism.

## Canonical Facts (VERIFIED — do not re-derive)

These were established by delegated reads of `OrcaSlicerDocumented/` and by grepping the PnP tree.
They **replace** the earlier draft's candidate framing, which rested on three false premises
(recorded in §Falsified Premises). Treat these as settled; do not spend Step-1 budget re-checking
them.

**C-1 — `discretize()` case analysis** (`SkeletalTrapezoidation.cpp:220`, branches at `:236`,
`:240`, `:246`):

| Branch | Condition (verbatim shape) | Returns |
| --- | --- | --- |
| 1 | `(!point_left && !point_right) \|\| vd_edge.is_secondary()` | `Points({start, end})` |
| 2 | `point_left != point_right` | parabola via `discretize_parabola` |
| 3 | `else` (both cells are point-cells) | straight edge, **still subdivided** by `discretization_step_size` + marking vertices |

Note branch 1 also fires on `is_secondary()` regardless of cell type — the earlier draft omitted
this disjunct.

**C-2 — A thin rectangle's spine is branch 1.** Its spine is the bisector of the two long
*segment* cells, so `!point_left && !point_right` holds → `{start, end}`, i.e. **a two-node edge
with no interior nodes**. **This means the "single two-node spine" that D-105D calls the collapse
is exactly what OrcaSlicer itself produces.** The two-node spine is not the defect.

**C-3 — `generateJunctions()` skips flat edges** (`SkeletalTrapezoidation.cpp:1727`, guard at
`:1740`): it processes only the upward half-edge (`from.R <= to.R`, `:1732`) and then
`continue`s on `end_R >= start_R` ("No beads to generate"). A thin strip's spine has **constant
R** (= half-width) along its whole length, so `end_R == start_R` and **OrcaSlicer emits zero
junctions on the spine edge too**.

**C-4 — PnP already implements C-3 faithfully.** `crates/slicer-core/src/arachne/
generate_toolpaths.rs:258` `generate_junctions` contains `if from_r >= to_r { continue; }` with a
comment that explicitly names the flat-edge case. (Line was `:210` when this fact was first
recorded; drift from later edits. The guard itself is untouched by packet 154 — AC-N2.) So the collapse is **not** a missing equal-R
guard, and "distribute junctions along the spine" is **not** a faithful goal — canonical never
does that.

**C-5 — Therefore, canonically, a thin strip's wall comes from the RIBS, not the spine.
VERIFIED** (`connectJunctions`, `SkeletalTrapezoidation.cpp:1949-2052`). The mechanism:

- Junctions are read off **`edge_to_peak`** and **`edge_from_peak->twin`** (`:1973`, `:1981`) — the
  quad's two **rib** edges, running boundary-node → spine-node. Their R strictly decreases
  (`start_R > end_R`), so `generateJunctions`' flat-edge guard (C-3) never skips them. **Junctions
  are never read off the flat spine edge itself.**
- The peak is selected by **`getQuadMaxRedgeTo(quad_start)`** (`:1963`) — the canonical function
  PnP's `quad_peak_position` claims to mirror.
- The wall is built by **`addToolpathSegment`** (`:1887-1932`), which appends each quad's
  `from`/`to` pair onto the **tail of the same `ExtrusionLine`** whenever the new point is within
  **0.01 mm** of the previous one (`:1906-1925`). `force_new_path` is set only on a new domain
  start or a junction-list break (`:1899-1905`).
- The outer `do…while` processes **one rib-quad per iteration**, advancing `quad_start` via
  `getNextUnconnected()` (`:2052`) — walking *along the length of the strip*.

So the spine-length wall is produced by **stitching many short rib-quad segments end-to-end**, not
by any single edge's junction list. A degenerate point/zero-length loop is **not** canonical
behavior. (The exact rib count for a given rectangle depends on boundary discretization during
graph construction — outside these two functions — but the stitching mechanism itself guarantees a
spine-spanning line.)

**C-6 — Only one canonical stage merges topology.** Delegated stage-order read of the constructor
+ `generateToolpaths()` (`SkeletalTrapezoidation.cpp:332`, `:496`) confirms `collapseSmallEdges()`
(`:450`) is the *only* stage that merges vertices / erases edges; `separatePointyQuadEndNodes()`
(`:448`) only *adds* nodes; every `filter*` stage flips flags without changing topology. Its
default `snap_dist = 5` (`SkeletalTrapezoidationGraph.hpp:94`) is **5 nm** — a coincident-point
tolerance, not a feature-scale one. PnP's `collapse_small_edges`
(`crates/slicer-core/src/skeletal_trapezoidation/graph.rs:353`) uses `SNAP_DIST_SQ = 0.05 * 0.05`
PnP units = 5 nm — **correctly unit-converted; this hazard is already closed.**

**C-7 — `generateLocalMaximaSingleBeads` does NOT fire on a thin strip.** Its guard
(`SkeletalTrapezoidation.cpp:2067`) requires `bead_widths.size() % 2 == 1 && node.isLocalMaximum(true)
&& !node.isCentral()`. A thin strip's spine is a **flat, constant-R run of *central* nodes** — not
an isolated local maximum, and central nodes are explicitly excluded. This demotes Candidate C′.

**C-8 — OrcaSlicer's `removeSmallLines` would NOT drop a thin strip's wall.**
(`WallToolPaths.cpp:684-700`, drop condition `:693`): it drops only lines where
`is_odd && !is_closed && shorterThan(...)`. A real spine-length strip wall is not short. So
PnP's observed "`remove_small_lines` drops it" (per D-105D) is a **downstream consequence** of the
zero-length loop, not the cause — and it is *not* what canonical does. **This effectively kills
Candidate D′**: canonical has a concrete mechanism (C-5) producing a real wall, and no thin-strip
drop path.

## Falsified Premises (what the earlier draft got wrong)

Recorded so the implementer does not chase them, and so the D-105D row can be corrected.

- **F-1 — `connectJunctions` is NOT in `SkeletalTrapezoidationGraph.cpp`.** It is defined at
  `SkeletalTrapezoidation.cpp:1934`. The draft (and the D-105D row) sent the reader to the wrong
  file.
- **F-2 — `getNextUnconnected()` is not a "single-edge-domain traversal".** It is
  `STHalfEdge::getNextUnconnected()` (`SkeletalTrapezoidationGraph.cpp:115-127`): it walks `next`
  pointers to the end of a chain and returns `result->twin` (`nullptr` if the chain cycles back to
  `this`). Its **real** role, per C-5, is to advance `quad_start` from one rib-quad to the next
  along the boundary (`SkeletalTrapezoidation.cpp:2052`) — chain advancement, not domain traversal.
  It *is* on the hot path for a thin strip, but not for the reason the draft gave, and it is a
  four-line pointer walk with no single-edge special case to get wrong.
- **F-3 — Neither `connect_junctions` nor `get_next_unconnected` nor `BeadingPropagation` exists
  in the PnP tree.** They appear only in OrcaSlicer-name doc comments. The draft's Candidates A
  and B named non-existent PnP symbols as edit targets. (PnP's `connectJunctions` equivalent is
  **inlined and unnamed** inside `generate_toolpaths.rs`; PnP's beading side-table is the
  `beading` field on `SkeletalTrapezoidationGraph`, `graph.rs:239-249`, populated by
  `propagation::populate_beading_propagation`.)
- **F-4 — Candidate C is real but off-target.** PnP's `discretize_edge`
  (`graph.rs:1280`) returns `{start, end}` for **all** `!is_curved` edges, which conflates
  canonical branch 1 and branch 3 — a genuine parity gap for **point-point** edges. But per C-2 a
  rectangle's spine is branch 1, so this gap **cannot** be the thin-strip root cause. It is a
  separate defect; see §Open Questions.

## Controlling Code Paths (revised candidate set)

Per C-5, the search moves to the rib/quad chain — PnP's inlined `connectJunctions`:

- **Candidate A′ — the quad chain walk** in `crates/slicer-core/src/arachne/generate_toolpaths.rs`:
  `find_quad` (`:466`), `quad_peak_position` (`:491`), `resolve_to_vertex` (`:149`),
  `chain_junctions_for_bead` (`:536`), `emit_chain_lines` (`:693`). D-105D's symptom — *"every
  emitted edge shares the same `to` peak vertex, so every junction snaps to one point"* — is
  literally a statement about `resolve_to_vertex` / `quad_peak_position`. **Prime suspect.**
- **Candidate B′ — rib topology** in `crates/slicer-core/src/skeletal_trapezoidation/rib.rs`
  (`build_quad_rib_topology`, `:101`) vs canonical `graph.makeRib()`. If ribs are not built (or
  their `prev`/`next` linkage is wrong), the R-varying edges that C-5 says carry *all* of a thin
  strip's junctions never exist or never chain.
- **Candidate C′ — `generate_local_maxima_single_beads`** (`generate_toolpaths.rs:803`) vs
  canonical `generateLocalMaximaSingleBeads`. **Demoted by C-7**: canonical's guard excludes
  *central* nodes and requires an isolated local maximum, which a flat central spine ridge is not.
  A thin strip should never enter this path. Check only whether PnP's guard wrongly *admits* it.
- **Candidate D′ — "OrcaSlicer behaves identically" (tests are wrong). Effectively KILLED by
  C-5 + C-8.** Canonical has a concrete spine-spanning mechanism, and its `removeSmallLines` drops
  only `is_odd && !is_closed && short` lines. Do **not** re-bless a golden to a zero-length loop.
  Retained only as a formal escape hatch requiring positive contradicting `file:line` evidence.

Ordering for Step 1: **A′ first** — it is the symptom restated, and C-5 puts the canonical
mechanism (`getQuadMaxRedgeTo` peak selection + `addToolpathSegment` end-to-end stitching with a
0.01 mm join tolerance) squarely inside PnP's `quad_peak_position` / `emit_chain_lines`. Then B′
(do the ribs that must carry *all* the junctions even exist?). C′ and D′ are now near-exonerated by
C-7/C-8 and need only a confirming glance.

**Highest-value single check** (do this first): per C-5, PnP's `emit_chain_lines` must append
successive rib-quad segments onto the **tail of the same `ExtrusionLine`** using a **0.01 mm
proximity join** (canonical `addToolpathSegment`, `:1906-1925`). If PnP instead starts a new line
per quad, or lacks that join tolerance, every quad collapses to its own degenerate segment — which
is exactly the reported symptom.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this
  packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"),
  the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported,
  rebuild without `--check` before re-running the failing test.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer
  constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary.
  Full porting checklist in `docs/08_coordinate_system.md`.

- **ADR-0034 faithfulness is non-negotiable.** No fabricated spine subdivision (the reverted
  `from_polygons_with_beading` mechanism subdividing all `!is_curved` edges > `2 * optimal_width`)
  may be reintroduced. C-2/C-3/C-4 now give the *reason* it was unfaithful: canonical emits **no**
  junctions on a flat spine, so subdividing the spine to create some is exactly backwards.
  Enforced by AC-N1, AC-N2.

- **Single point of failure: Step 1.** Steps 2-4 MUST NOT begin until §Step 1 Findings names the
  responsible mechanism.

- **No schema bump / no WIT changes expected.** The collapse is internal to
  `skeletal_trapezoidation`/`arachne`; `ExtrusionLine`/`ExtrusionJunction` shapes are unchanged.

## Code Change Surface

- **Selected approach:** TBD pending §Step 1 Findings — exactly one of A′ / B′ / C′ / D′.
- **Exact functions expected to change:** TBD pending Step 1. The acceptance surface is fixed: the
  4 thin-strip tests + the G4 test, all currently RED. No new tests required beyond the Step-1
  regression pin.
- **Rejected alternatives:**
  - **The reverted fabricated spine subdivision.** Rejected on canonical evidence C-2/C-3:
    OrcaSlicer returns `{start,end}` for a seg-seg spine *and* emits zero junctions on it.
    Violates ADR-0034. Forbidden by AC-N1.
  - **Porting `discretize` branch 3 (point-point subdivision) as the thin-strip fix.** Rejected
    per F-4/C-2: the gap is real but a rectangle's spine is branch 1, so branch 3 cannot be the
    root cause. Tracked separately (§Open Questions), not fixed here.
  - **Adding an equal-R junction path to `generate_junctions`.** Rejected per C-3/C-4: canonical
    skips flat edges and PnP already matches. Any change here would *introduce* a deviation.

## Files in Scope (read + edit)

- `crates/slicer-core/src/arachne/generate_toolpaths.rs` — Candidates A′/C′ (Step 1 read; Step 2-3
  edit only if implicated)
- `crates/slicer-core/src/skeletal_trapezoidation/rib.rs` — Candidate B′ (Step 1 read; Step 2-3
  edit only if implicated)
- `crates/slicer-runtime/tests/arachne_parity.rs` + `arachne_parity_gaps.rs` — golden re-blessing
  (Step 5)
- `modules/core-modules/arachne-perimeters/tests/arachne_parity_is_thin_wall_flag_tdd.rs` +
  `arachne_parity_thin_wall_loop_type_tdd.rs` — golden re-blessing (Step 5)
- `docs/DEVIATION_LOG.md` + `docs/18_arachne_parity_audit.md` — D-105D closure + G4 note (Step 5)

## Read-Only Context

- `docs/DEVIATION_LOG.md` D-105D (line 27) — the open entry this packet closes. **Note: its
  symbol list is wrong** (see F-1/F-2/F-3); correcting it is part of the Step-5 closure.
- `docs/adr/0034-arachne-faithful-graph-construction.md` — the faithfulness constraint.
- `docs/18_arachne_parity_audit.md` §G4 (lines 87-101) — the G4 closure the collapse masks.
- `crates/slicer-core/src/skeletal_trapezoidation/graph.rs:239-249` — the `beading` side-table
  (the real PnP analogue of `BeadingPropagation`).
- `OrcaSlicerDocumented/...` — delegate only. **Most canonical questions are already answered in
  §Canonical Facts; do not re-dispatch them.**

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` — delegate parity checks; never load directly.
- `target/`, `Cargo.lock`, generated code — never load.
- `crates/slicer-runtime/tests/fixtures/perimeter_parity/*/expected_perimeter_ir.json` — never
  read directly (can exceed 10MB); always re-record via the documented `#[ignore]`d `record_*`
  functions.
- `crates/slicer-core/src/skeletal_trapezoidation/propagation.rs` — the draft's Candidate B rested
  on a non-existent type (F-3); out of bounds unless Step 1 produces new evidence implicating it.
- `crates/slicer-core/src/skeletal_trapezoidation/graph.rs` `discretize_edge` — F-4 gap, tracked
  separately; do not edit in this packet.
- Classic-perimeters, spiral-vase, non-planar, and the D-105/D-105B/C/E fixes — out of scope.

## Expected Sub-Agent Dispatches

- (Step 1) "Run `cargo test -p arachne-perimeters --test arachne_parity_is_thin_wall_flag_tdd`,
  `cargo test -p slicer-runtime --test arachne_parity`, and `cargo test -p slicer-runtime --test
  arachne_parity_gaps -- arachne_parity_pipeline_wall_gap_uses_flow_spacing_not_width --exact`;
  return SNIPPETS (≤ 20 lines each) of the failing assertion plus the wall-loop state (length,
  junction count, `is_closed`)." — purpose: establish the failure shape.
- (Step 1, local) "In `crates/slicer-core/src/arachne/generate_toolpaths.rs`, for a thin-strip
  graph: do `resolve_to_vertex` (`:149`) and `quad_peak_position` (`:491`) resolve **every** quad's
  peak to the same vertex? Instrument or unit-test the single-edge-domain fixture at `:1112`.
  Return FACT: the distinct peak-vertex count for a thin-strip graph, and the junction positions
  emitted per bead." — purpose: confirm/deny Candidate A′ (the prime suspect).
- (Step 1, local) "In `crates/slicer-core/src/skeletal_trapezoidation/rib.rs`
  (`build_quad_rib_topology`, `:101`): for a thin rectangle, how many rib edges are created, and
  what are their endpoints' `distance_to_boundary` values? Return FACT (rib count + R range)." —
  purpose: confirm/deny Candidate B′ (do the R-varying edges that must carry all junctions per C-5
  actually exist?).
- **(No OrcaSlicer dispatch is required for Step 1.)** The `connectJunctions` chaining rule was
  pinned during packet refinement and is recorded verbatim in C-5/C-7/C-8. Re-dispatching it is
  wasted budget. A delegated read is warranted only if a Step-1 finding directly contradicts C-5.

## Data and Contract Notes

- The two-node spine is **canonical, not a bug** (C-2). Any fix that adds spine nodes is a
  deviation and must be rejected.
- A thin strip's junctions live on ribs and pointy-end edges, not the spine (C-3/C-5). The wall's
  length therefore comes from *chaining*, not from *junction placement*.
- G4 observability depends on the collapse being fixed: the D-105 beading fix changed the wall gap
  from `thickness/max_bead_count` to `optimal_width` (Flow spacing), correct for the over-cap
  branch, but the topology-level collapse prevents the gap from being observable on thin strips.

## Locked Assumptions and Invariants

- C-1 … C-6 are verified canonical/tree facts and are **locked**; a Step-1 finding that contradicts
  one must cite the contradicting `file:line` explicitly and update this section.
- ADR-0034 prohibits fabricated subdivisions; `from_polygons_with_beading` must not return.
- The D-105 beading fix is faithful and out of scope (per D-105D).

## Risks and Tradeoffs

- **Chasing a falsified candidate.** Mitigated: F-1…F-4 retire the draft's candidate set; the
  revised set A′/B′/C′ is ordered by evidence strength.
- **Golden re-blessing masks a real defect (D′ wrong).** Mitigation: C-5 makes D′ unlikely;
  re-blessing to a zero-length loop is forbidden without positive `file:line` evidence from
  `connectJunctions` that canonical also degenerates.
- **Fix regresses classic perimeters.** Mitigation: narrow per-AC runs + the Step-4 workspace gate.

## Context Cost Estimate

- **Aggregate:** M. Step 1 is the heaviest but is now **cheaper than the draft's** — three of the
  four OrcaSlicer dispatches — plus the `connectJunctions` chaining rule — are pre-answered in
  §Canonical Facts. Step 1 needs **zero** OrcaSlicer delegations; only two local traces.
- **Largest single step:** Step 1 — M.
- **Highest-risk dispatch:** the Candidate A′ peak-vertex trace — it decides the packet's direction.

## Step 1 Findings

**Completed 2026-07-14.** The diagnosis falsified the packet's own candidate set. The root cause is
**upstream of every candidate A′/B′/C′/D′**: the outline is corrupted by preprocessing *before* the
medial-axis graph is ever built. A fifth candidate — **E′** — is added below and carries the verdict.

**None of C-1 … C-8 is contradicted.** Every locked canonical fact survives. What was falsified is
the candidate set's *completeness*: A′/B′/C′/D′ all silently assumed the skeletal graph is built
from the actual 0.25 mm × 5 mm rectangle. It is not.

### Candidate E′ — outline preprocessing collapses the strip before graph construction

`crates/slicer-core/src/arachne/preprocess.rs` — `merge_short_segments` (`:240-260`), reached via
`simplify_stage` (`:216-235`), stage 2 of the nine-stage `preprocess_input_outline` pipeline
(`:139-146`, `:176-194`), called from exactly one production site, `arachne/pipeline.rs:336`, with
`PreprocessParams::default()`.

### Empirical trace (fixture: `rect_polygon(0.0, 0.0, 0.25, 5.0)`, `optimal_width` 0.4 mm)

| Stage | Output |
| --- | --- |
| stage 1 `triple_offset` | clean 4-point rectangle `[(0.125,-2.5), (0.125,2.5), (-0.125,2.5), (-0.125,-2.5)]` |
| stage 2 `simplify_stage` | **3-point triangle** `[(0.125,-2.5), (0.125,2.5), (-0.125,-2.5)]` — a corner is gone |

Everything downstream is then *correct behavior on corrupted input*:

- The medial axis of that triangle has exactly **one** meaningful vertex: `v[1]` at
  (0.0031, −2.3781) mm, R = 0.1219 mm, `bead_count = 1`.
- `generate_junctions` fans 6 edges, each with 1 junction — **all 6 land at the identical point**
  (0.00312, −2.37812) mm. There is no second spine node 5 mm away, because the spine does not exist.
- `emit_chain_lines` emits one `ExtrusionLine` with 7 coincident junctions; polyline length =
  **0.0000 mm**.
- `remove_small_lines` (`arachne/remove_small.rs:66-96`) then correctly drops it: `is_odd = true`,
  `is_closed = false`, threshold `0.4 * 0.5 = 0.2 mm`, and `0.0 < 0.2` → removed. `wall_loops()` is
  empty, which is the `assert!(!walls.is_empty())` all 4 thin-strip tests fail on.

### Why preprocessing drops the corner (the actual defect)

Canonical `simplify()` (`OrcaSlicerDocumented/src/libslic3r/Arachne/WallToolPaths.cpp:140-197`)
gates vertex removal on an **AND** of *two* conditions (`WallToolPaths.cpp:161-162`):

```cpp
if (length2 < smallest_line_segment_squared
    && height_2 <= allowed_error_distance_squared) // removing the vertex doesn't introduce too much error
```

where `height_2` is the squared perpendicular **deviation** the removal would introduce
(`WallToolPaths.cpp:155`, via accumulated Shoelace area → triangle height).

PnP ported **only the length half**. `merge_short_segments` (`preprocess.rs:246-255`) drops a vertex
purely on Euclidean distance to the last *kept* point, with no deviation bound — its sole safety net
is a "never below 3 points" floor (`:242`, `:256-258`), which is precisely what permits a 4-point
rectangle to become a 3-point triangle:

```rust
if (dx * dx + dy * dy).sqrt() < min_len_units {
    continue;                      // preprocess.rs:250 — length-only; no deviation guard
}
```

The constants are **not** the bug. PnP's `smallest_segment_mm = 0.5` (`preprocess.rs:100`) correctly
matches canonical `meshfix_maximum_resolution()` = 0.5 mm (`WallToolPaths.hpp:19-21`), and
`PreprocessParams` **already carries the correct `allowed_distance_mm = 0.025`**
(`preprocess.rs:101`) matching canonical `meshfix_maximum_deviation()` = 0.025 mm. That field is
simply never read by `merge_short_segments`. A deviation-bounded RDP pass does run afterwards
(`expolygons_simplify`, `:234`) — but it runs *after* the length-merge has already destroyed the
points, and RDP cannot restore a vertex that no longer exists.

For the 0.25 mm strip: the end-cap edge is 0.25 mm < 0.5 mm, so the length trigger fires. Removing
that corner deviates the outline by ≈0.25 mm — **10× the 0.025 mm allowance** — so canonical's AND
fails and the vertex is **kept** (`WallToolPaths.cpp:193-197`, `//Don't remove the vertex.`). All 4
corners survive in OrcaSlicer. PnP, checking only length, drops one.

The file's attribution header (`preprocess.rs:1-11`) also cites the wrong port source:
`WallToolPaths.cpp:565-604`. Canonical `simplify()` is at `WallToolPaths.cpp:140-197`.

### Candidates exonerated

- **A′ (quad chain walk) — EXONERATED.** `find_quad` / `quad_peak_position` / `resolve_to_vertex` /
  `chain_junctions_for_bead` / `emit_chain_lines` faithfully process the graph they are given. The
  7 coincident junctions are a *consequence* of a one-vertex medial axis, not a chaining defect.
- **B′ (rib topology) — EXONERATED.** `build_quad_rib_topology` (`rib.rs:101-106`) is a documented
  vestigial no-op (`rib.rs:14-42`), but the real `makeRib` port — `Builder::make_rib`
  (`graph.rs:992-1021`, invoked `graph.rs:850/857/935` from `Builder::build`) — does create both
  R-varying ribs and links them via `prev`/`next`/`twin` (`graph.rs:1013-1020`). For a genuine thin
  rectangle it yields 2 quads, each spanning the full spine — ample for a spine-length wall.
- **C′ (`generate_local_maxima_single_beads`) — EXONERATED**, as C-7 predicted.
- **D′ ("OrcaSlicer behaves identically") — KILLED.** Canonical provably preserves all 4 corners
  (`WallToolPaths.cpp:161-162`). The goldens are right; the code is wrong. No golden may be
  re-blessed to a zero-length loop.

### Verdict

```
verdict: E′   — outline preprocessing collapses the 0.25mm strip to a triangle before graph construction: merge_short_segments drops vertices on segment LENGTH alone, omitting canonical simplify()'s AND-gated deviation bound (crates/slicer-core/src/arachne/preprocess.rs:246-255 vs OrcaSlicerDocumented/src/libslic3r/Arachne/WallToolPaths.cpp:161-162)
```

### Zero goldens were re-recorded (and that is the correct outcome)

The packet planned to "re-bless the 6 stale goldens" (Step 5). **No fixture or golden file needed any
change, and none was made.** That framing presupposed the tests were encoding stale expectations. They
were not: killing D′ established that the 4 thin-strip tests asserted *correct* OrcaSlicer-parity
behavior all along — `assert!(!walls.is_empty())` is exactly right, and PnP was simply failing to
produce the wall. The production code was wrong; the goldens were right. They went green on the fix
with every assertion untouched.

Verified: `git status --porcelain -- crates/slicer-runtime/tests/fixtures/` is empty, and there is no
diff against `arachne_parity.rs`, `arachne_parity_gaps.rs`, or the two `arachne-perimeters` TDD test
files. Re-blessing a golden here would have been the failure mode, not the deliverable.

### Faithful fix (authorized by this verdict)

Port canonical `simplify()` (`WallToolPaths.cpp:140-197`) into `preprocess.rs` so vertex removal is
gated on `length2 < smallest_line_segment_squared && height_2 <= allowed_error_distance_squared`,
using the `allowed_distance_mm` field that already exists. This satisfies the faithfulness gates by
construction: it adds no spine subdivision (AC-N1), no junction emission on flat/equal-R edges
(AC-N2), and no interior nodes to the two-node seg-seg spine (AC-N3) — it touches only outline
preprocessing, upstream of the graph.

## Open Questions

- [FWD] Which of A′ / B′ / C′ / D′ is responsible? Resolved by Step 1.
- [FWD] **New deviation to file (not fixed here):** PnP's `discretize_edge` (`graph.rs:1280`)
  returns `{start, end}` for all `!is_curved` edges, conflating canonical branch 1 and branch 3
  (F-4). Point-point VD edges are therefore never subdivided, unlike canonical. This is a real
  parity gap with a real geometric consequence (point-point channels that narrow between two
  vertices get no intermediate beading samples) — but it is **not** the thin-strip root cause. Step
  5 should open a new deviation-log row for it rather than silently absorbing it.
- [FWD] PnP's `discretize_edge` also omits canonical branch 1's `is_secondary()` disjunct (C-1).
  Confirm whether PnP's `is_curved` flag already encodes secondary-edge status; if not, that is a
  second sub-gap of the same deviation row.
- [BLOCK] None — Step 1 is the gate, not a blocker.
