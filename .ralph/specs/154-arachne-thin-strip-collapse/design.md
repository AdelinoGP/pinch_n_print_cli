# Design: 154-arachne-thin-strip-collapse

> **TBD — investigation pending.** This packet is an investigation first. No solution is
> prescribed. The controlling code path, code-change surface, and selected approach below are
> placeholders that Step 1's diagnosis MUST resolve. Do not implement against this file until
> `§Step 1 Findings` names the responsible mechanism.

## Controlling Code Paths

The responsible location is one of three candidates (verified present in the tree via D-105D's
own references; full-read only once Step 1 implicates it):

- **Candidate A — `connectJunctions` / `getNextUnconnected` traversal** in
  `crates/slicer-core/src/arachne/generate_toolpaths.rs`. The single-edge spine domain may not be
  walked correctly: a two-node edge whose `to` peak vertex every emitted edge shares yields a
  degenerate traversal where every junction snaps to one point.
- **Candidate B — `BeadingPropagation`** in
  `crates/slicer-core/src/skeletal_trapezoidation/propagation.rs`. A single-edge domain may be
  assigned a degenerate bead count, collapsing all junctions to one vertex before emission.
- **Candidate C — `discretize_edge` in
  `crates/slicer-core/src/skeletal_trapezoidation/graph.rs`.** Currently returns `{start, end}`
  for ALL `!is_curved` edges. OrcaSlicer `discretize` distinguishes three cases: seg-seg →
  `{start,end}`; point-segment → parabola; point-point → subdivided by `discretization_step_size`
  with marking vertices. The missing faithful port of case 3 (point-point) *may* be needed — BUT a
  rectangle's medial-axis spine is seg-seg (case 1), NOT point-point, so this is **unlikely** to
  be the root cause for a thin strip. Must be verified during Step 1.
- **Candidate D — "OrcaSlicer behaves identically" (tests are wrong).** OrcaSlicer's
  `WallToolPaths.cpp` may itself drop/zero-length-loop a thin strip. If so, no code fix is needed;
  the goldens are re-blessed to that documented behavior (CR-4).

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this
  packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"),
  the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported,
  rebuild without `--check` before re-running the failing test. This packet's surface is
  host-side (`slicer-core` internals and/or `arachne-perimeters`); no WIT/module edits expected,
  but the check is run as a precaution.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer
  constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary.
  Full porting checklist in `docs/08_coordinate_system.md`.

- **ADR-0034 faithfulness is non-negotiable.** No fabricated spine subdivision (the reverted
  `from_polygons_with_beading` mechanism subdividing all `!is_curved` edges > `2 * optimal_width`)
  may be reintroduced. Any fix MUST be traceable to a specific OrcaSlicer
  `discretize`/`WallToolPaths`/`connectJunctions` case (AC-N1, AC-N2).

- **Single point of failure: Step 1 (diagnosis).** Steps 2-4 MUST NOT begin until Step 1's
  findings exist and name the responsible mechanism. If Step 1 concludes Candidate D, Steps 2-4
  collapse into golden re-blessing (Step 5) with no code change beyond the goldens.

- **No schema bump / no WIT changes expected.** The collapse is internal to
  `skeletal_trapezoidation`/`arachne`; `ExtrusionLine`/`ExtrusionJunction` shapes are unchanged.

## Code Change Surface

- **Selected approach:** TBD pending `§Step 1 Findings`. Exactly one of: (A) faithful
  `connectJunctions`/`getNextUnconnected` single-edge-domain fix; (B) faithful
  `BeadingPropagation` degenerate-bead-count fix; (C) faithful `discretize_edge` case-3 port (only
  if Step 1 proves case 3 is genuinely required for the thin strip); or (D) golden re-blessing
  with no code change.
- **Exact functions / tests / fixtures expected to change:** TBD pending Step 1. The acceptance
  surface is fixed: the 4 thin-strip tests + the G4 test (6 goldens total), all currently RED
  under the reverted fabrication. No new tests are required.
- **Rejected alternatives:**
  - **The reverted fabricated spine subdivision** (`from_polygons_with_beading` subdividing all
    `!is_curved` edges > `2 * optimal_width`). Rejected: verified NOT a faithful port — OrcaSlicer
    `discretize` returns `{start, end}` for the seg-seg edges making up a thin strip's spine.
    Violates ADR-0034. Forbidden by AC-N1.
  - **Assuming Candidate C (case-3 subdivision) is the root cause without verifying.** Rejected: a
    rectangle's medial-axis spine is seg-seg (case 1), not point-point (case 3), so case 3 is
    unlikely to be the thin-strip root cause — must be confirmed by Step 1 evidence, not assumed.

## Files in Scope (read + edit)

- `crates/slicer-core/src/arachne/generate_toolpaths.rs` — Candidate A (Step 1 read; Step 2-3 edit
  only if implicated)
- `crates/slicer-core/src/skeletal_trapezoidation/propagation.rs` — Candidate B (Step 1 read;
  Step 2-3 edit only if implicated)
- `crates/slicer-core/src/skeletal_trapezoidation/graph.rs` — Candidate C (Step 1 read; Step 2-3
  edit only if implicated)
- `crates/slicer-runtime/tests/arachne_parity.rs` + `arachne_parity_gaps.rs` — golden re-blessing
  (Step 5)
- `modules/core-modules/arachne-perimeters/tests/arachne_parity_is_thin_wall_flag_tdd.rs` +
  `arachne_parity_thin_wall_loop_type_tdd.rs` — golden re-blessing (Step 5)
- `docs/DEVIATION_LOG.md` + `docs/18_arachne_parity_audit.md` — D-105D closure + G4 note (Step 5)

## Read-Only Context

- `docs/DEVIATION_LOG.md` `D-105D` (line 27) — the open entry this packet closes.
- `docs/adr/0034-arachne-faithful-graph-construction.md` — the faithfulness constraint.
- `docs/18_arachne_parity_audit.md` §G4 (lines 87-101) — G4 closure the collapse masks.
- D-105B / D-105C / D-105E rows — confirm out-of-scope fixes.
- `OrcaSlicerDocumented/...` — delegate only (see `requirements.md` §OrcaSlicer Reference
  Obligations): `SkeletalTrapezoidation.cpp` (`discretize`/`connectJunctions`),
  `WallToolPaths.cpp` (thin-strip special cases), `SkeletalTrapezoidationGraph.cpp`
  (`getNextUnconnected`).

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` — delegate parity checks; never load directly.
- `target/`, `Cargo.lock`, generated code — never load.
- `crates/slicer-runtime/tests/fixtures/perimeter_parity/*/expected_perimeter_ir.json` — never
  read directly (can exceed 10MB); always re-record via documented `#[ignore]`d `record_*`
  functions.
- Classic-perimeters, spiral-vase, non-planar, and the D-105/D-105B/C/E fixes — out of scope.

## Expected Sub-Agent Dispatches

- (Step 1) "Reproduce the thin-strip collapse: run `cargo test -p arachne-perimeters --test
  arachne_parity_is_thin_wall_flag_tdd` and `cargo test -p slicer-runtime --test arachne_parity`
  and `cargo test -p slicer-runtime --test arachne_parity_gaps -- arachne_parity_pipeline_wall_gap_uses_flow_spacing_not_width --exact`; return SNIPPETS (≤ 20 lines) of the failure for each — the assertion that fails and the wall-loop state (length, junction count, `is_closed`)." — purpose: establish the exact failure shape.
- (Step 1, delegated) "Summarize `OrcaSlicerDocumented/src/libslic3r/Arachne/
  SkeletalTrapezoidation.cpp`'s `discretize()`/`discretize_edge()` case analysis: for a thin
  rectangle strip, is the medial-axis spine seg-seg (case 1, `{start,end}`) or point-point (case
  3, subdivided)? Confirm from the C++ which case a rectangle's spine falls under. Return SUMMARY
  (≤ 200 words) + one 30-line excerpt if it clarifies the case selection. No other code." —
  purpose: resolve Candidate C's likelihood.
- (Step 1, delegated) "Summarize `OrcaSlicerDocumented/src/libslic3r/Arachne/WallToolPaths.cpp`
  for thin-strip special cases: does OrcaSlicer itself emit a zero-length loop / drop a thin
  strip, or emit a real (non-zero-length) wall? Return SUMMARY (≤ 200 words). No code." — purpose:
  resolve Candidate D (are the tests wrong?).
- (Step 1, delegated) "Summarize `OrcaSlicerDocumented/src/libslic3r/Arachne/
  SkeletalTrapezoidationGraph.cpp`'s `connectJunctions()`/`getNextUnconnected()` for a single-edge
  (two-node) spine domain: how does the traversal handle a domain that is exactly one edge? Return
  SUMMARY (≤ 200 words). No code." — purpose: resolve Candidate A.
- (Step 1) "Trace `BeadingPropagation` in `crates/slicer-core/src/skeletal_trapezoidation/
  propagation.rs` for a single-edge domain: does it assign a degenerate bead count that collapses
  all junctions to one vertex? Return LOCATIONS (file:line + 1-line note) for the assignment
  logic." — purpose: resolve Candidate B.

## Data and Contract Notes

- The collapsed spine is a single two-node edge; its `to` peak vertex is shared by every emitted
  edge. Any faithful fix must break that single-vertex sharing so junctions distribute along the
  spine (or accept zero-length per OrcaSlicer — Candidate D).
- G4 observability depends on the collapse being fixed: the D-105 beading fix changed the wall gap
  from `thickness/max_bead_count` to `optimal_width` (Flow spacing), correct for the over-cap
  branch, but the topology-level collapse prevents the gap from being observable on thin strips.
  Fixing the collapse makes G4 observable and GREEN.

## Locked Assumptions and Invariants

- The D-105 beading fix is faithful and out of scope (per D-105D).
- ADR-0034 prohibits fabricated subdivisions; the reverted `from_polygons_with_beading` mechanism
  must not return.
- A rectangle's thin-strip medial-axis spine is geometrically seg-seg (case 1), so `discretize`
  case 3 is unlikely to be the root cause — but this is a hypothesis to be confirmed by Step 1,
  not a locked fact.

## Risks and Tradeoffs

- **Wrong candidate fixed.** Mitigation: Step 1 diagnosis gates all fix work; AC-N1/AC-N2 enforce
  faithfulness regardless of which candidate wins.
- **Case 3 rabbit hole.** Mitigation: Step 1 must explicitly confirm or deny case 3's relevance
  for a seg-seg spine before any `graph.rs` change.
- **Golden re-blessing masks a real defect (Candidate D wrong).** Mitigation: Step 1's delegated
  `WallToolPaths.cpp` read must produce OrcaSlicer file:line evidence that OrcaSlicer truly drops
  the strip; re-blessing without that evidence is forbidden.
- **Fix regresses classic perimeters.** Mitigation: `cargo test` narrow runs per AC; full
  workspace gate in `packet.spec.md` §Verification.

## Context Cost Estimate

- **Aggregate (sum across 5 steps):** M. Step 1 (diagnosis + 4 delegated OrcaSlicer reads) is the
  heaviest; the fix itself (Steps 2-4) is S/M pending the diagnosis; Step 5 (re-bless + close) is
  S.
- **Largest single step:** Step 1 (diagnosis) — M.
- **Highest-risk dispatches:** the 4 Step 1 OrcaSlicer delegations (`discretize` case analysis,
  `WallToolPaths` thin-strip behavior, `connectJunctions` single-edge handling, plus the local
  `BeadingPropagation` trace) — these decide the entire packet's direction.

## Step 1 Findings

**Pending Step 1 (diagnosis).** This section is populated by Step 1 with: the exact failing
assertion shape for the 4 thin-strip tests + G4; the delegated OrcaSlicer `discretize` case
analysis (seg-seg vs point-point for a thin rectangle); the `WallToolPaths.cpp` thin-strip
behavior (does OrcaSlicer drop or emit?); and the `connectJunctions`/`getNextUnconnected` vs
`BeadingPropagation` vs `discretize_edge` verdict naming the single responsible mechanism. Until
then, no approach is selected and no code changes are authorized.

## Open Questions

- [FWD] Which candidate (A/B/C/D) is the responsible mechanism? Resolved by Step 1's findings.
- [FWD] If Candidate C (case 3), is the thin-strip spine genuinely point-point in some fixture
  despite being seg-seg for a plain rectangle? Confirmed against the actual fixtures in Step 1.
- [BLOCK] None — this packet activates immediately; Step 1 is the gate, not a blocker.
