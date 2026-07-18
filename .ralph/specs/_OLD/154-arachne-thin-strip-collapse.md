---
status: implemented
packet: 154-arachne-thin-strip-collapse
task_ids: []
---

# 154-arachne-thin-strip-collapse

## Goal

Determine and fix the root cause of the thin-strip medial-axis collapse so the 4 thin-strip
parity tests plus the G4 Flow-spacing test go GREEN with a *faithful* mechanism — never with a
fabricated spine subdivision that violates ADR-0034. This packet is an **investigation first**:
it prescribes no solution; its sole deliverable is a verified root cause and a faithful fix that
survives the OrcaSlicer `discretize` case analysis.

## Problem Statement

A faithful-port audit of the D-105 beading fix verified one fix and surfaced one open root-cause
question: the **thin-strip medial-axis collapse**. Symptom (per D-105D): a thin strip's medial
axis collapses to a single two-node edge whose `to` peak vertex every emitted edge shares, so
every junction snaps to that one point and the wall loop has zero length, then
`remove_small_lines` drops it. The D-105 beading fix is faithful but does NOT address this — it
is a graph-construction / topology issue governed by ADR-0034 (faithful port, no fabricated
subdivisions). A prior session tried to mask the symptom with a fabricated
`from_polygons_with_beading` spine subdivision in `graph.rs` (subdividing all `!is_curved` edges
longer than `2 * optimal_width`). That was verified as NOT a faithful port — OrcaSlicer's
`discretize` returns `{start, end}` for the seg-seg edges that make up a thin strip's spine. The
fabrication was reverted; the thin-strip parity tests are now RED. This packet investigates the
true root cause and fixes it faithfully, closing D-105D.

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
