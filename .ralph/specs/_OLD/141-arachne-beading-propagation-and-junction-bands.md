---
status: implemented
packet: 141-arachne-beading-propagation-and-junction-bands
task_ids:
  - none
---

# 141-arachne-beading-propagation-and-junction-bands

## Goal

Port `BeadingPropagation` + `getBeading` (N7) and rewrite `generate_junctions` to
the canonical upward-half-edge/in-band/no-clamp scheme (N1), so junctions ride
the carrier edge whose radius band contains the bead's target radius (ribs
included, no centrality gate, single beading at the peak node), making the
outer wall land at `preferred_bead_width_outer / 2` from the boundary.

## Problem Statement

Packet 113c's faithful per-cell graph construction + interleaved-rib topology
(Steps 1-8b, `D-113C-FAITHFUL-GRAPH-CONSTRUCTION`) fixed the substrate but
explicitly left the layer that turns the graph into toolpaths as an
acknowledged "ADAPTATION" — and the second-pass parity audit
(`target/arachne_parity_audit_20260706_020657.md`, findings N1 + N7) proved
that adaptation diverges from the canonical algorithm at its core, with a
user-visible symptom: "output still visibly worse than canonical Orca and worse
than classic walls" (measured: outer-wall junctions 2.0 mm from the boundary of
a 20×4 mm rectangle — exactly the medial axis — and ~0.5 mm inset on a 10 mm
square vs canonical 0.2 mm, so the part prints undersized everywhere).

The single deepest divergence (N1): canonical Arachne places extrusion junctions
on **upward half-edges whose radius band contains the bead's target radius —
primarily the rib (`EXTRA_VD`) edges — and skips beads outside the band**; PNP
instead places junctions **only on `central` spine edges, excludes ribs
entirely, emits *every* bead index on *both* half-edge directions, clamps
out-of-band beads onto the edge endpoints, and resolves the beading from each
endpoint's own (domain-max) bead count**. The prerequisite for the fix (N7):
canonical keeps a **`BeadingPropagation` side table (a full `Beading` + source
distances) per node** and resolves rib-foot nodes (which have no `bead_count`)
via `getBeading`'s propagation/nearest lookup (0.1 mm `getNearestBeading`
radius); PNP stores only `STVertex::bead_count` and blends rounded integers, so
the smooth width falloff near region boundaries (the visible hallmark of
Arachne) cannot be produced even with correct gating. This packet closes both
gaps in one atomic rewrite — N7 is bundled with N1 because N7 has no dedicated
red test (only N1's tests validate it), so green-gating N7 in isolation would
be a weak oracle.

This packet supersedes `D-113C-FAITHFUL-GRAPH-CONSTRUCTION` for the
junction-generation layer only; 113c's graph construction (Steps 1-3) and
`insert_node` re-audit (Step 6) remain canonical and untouched. 113c's
deferred Steps 9-10 (fixture re-baseline + e2e closure) are inherited by this
packet chain (per `docs/specs/arachne-parity-N1-N13-plan.md` — distributed
per-packet, A1 re-baselines only the fixtures its own stage touches), not
silently absorbed.

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- Packet-specific constraint: **`BeadingPropagation` is a side table, not a field on `STVertex`.** Upstream keeps the full `Beading` per node in a side structure (`SkeletalTrapezoidation.cpp:2091-2127`), not on the vertex itself. A1 must match this layout — putting a `Beading` on `STVertex` would bloat the struct and break the existing `STVertex: PartialEq` derive across centrality/bead_count/propagation fixtures. The side table is owned by `SkeletalTrapezoidationGraph` (a `Vec<Beading>` indexed by vertex, with a sentinel for "no beading yet", or `HashMap<usize, Beading>` if sparse — the implementer decides based on upstream's actual density, which the audit summary says is full per-node).
- Packet-specific constraint: **A1 must not remove the π hack (`pipeline.rs:334`) or the 0.1× filter-dist fudge (`pipeline.rs:272-277`).** Those are Packet C's (`144`) scope, strictly after A2. A1's rewrite is gated on the centrality scheme the π hack sustains.
- Packet-specific constraint: **A1 must not touch `arachne_pipeline.rs:122` or delete `assign_perimeter_indices`.** Both are A2's scope. A1 leaves `perimeter_index = 0` at junction generation.
- Packet-specific constraint: **WASM staleness does NOT apply** — A1's change surface is `slicer-core`-internal (`arachne/`, `skeletal_trapezoidation/`); no path feeds the guest WASM build (`wit/`, `slicer-macros`, `slicer-sdk`, `slicer-ir`, `slicer-schema`, core-modules). The `wasm-staleness` snippet is intentionally omitted.
- Packet-specific constraint (added 2026-07-06, see `packet.spec.md`'s "Known
  Implementation Hazard"): **the beading MUST be resolved at the peak
  (`edge.to`) always — never at `edge.from` as a primary path with the peak
  as a fallback — and each junction's width MUST come from that ONE resolved
  beading's own `bead_widths[idx]`, never a fresh per-bead
  `strategy.compute()` call.** `generate_junctions` must have NO
  `edge.central`/`edge.edge_type == EdgeType::EXTRA_VD` gate, and the domain
  seeding in `generate_toolpaths` must NOT gain a matching filter either —
  canonical has zero such checks. Run
  `cargo test -p slicer-core --features host-algos --test arachne_generate_junctions_canonical_regression`
  after any change to `generate_junctions` to confirm.

## Data and Contract Notes

- IR or manifest contracts touched: **none**. `ExtrusionJunction::perimeter_index` stays `u32` and stays `0` at A1's layer (A2 sets it to `bead_idx`). `ExtrusionLine`/`ExtrusionJunction` field shapes unchanged.
- WIT boundary considerations: **none**. A1's change surface is `slicer-core`-internal; no WIT/IR schema change. The `perimeter_index` semantic change is A2's scope decision (wire-type-transparent).
- Determinism: A1's rewrite preserves determinism (index-ordered traversal; the upward-half-edge skip and in-band bead filter are deterministic given the graph). The `getNearestBeading` 0.1 mm radius lookup must be deterministic under ties (index-ascending tiebreak, matching upstream's `BTreeSet`/`std::map` ordering).

## Locked Assumptions and Invariants

- `BeadingPropagation` is a side table on `SkeletalTrapezoidationGraph`, not a field on `STVertex` — keeps the vertex struct small, matches upstream, preserves `STVertex: PartialEq` derives.
- `getBeading`'s `getNearestBeading` radius is 0.1 mm = 1000 slicer units (1 unit = 100 nm per `docs/08_coordinate_system.md`).
- A1 leaves the π hack (`pipeline.rs:334`), the 0.1× filter-dist fudge (`pipeline.rs:272-277`), `arachne_pipeline.rs:122`, and `assign_perimeter_indices` (`pipeline.rs:384-390`) untouched — all are downstream packets' scope.
- A1 keeps N2, N3, N4 red tests RED (gated by the "stays red" verification commands).
- `Beading` invariant `bead_widths.len() == toolpath_locations.len()` preserved on every side-table entry; debug-assert in `get_beding`'s hot path.
- Fixture re-baseline uses the self-capture pattern (first-run writes if missing, subsequent compare) — never read the JSONs directly.

## Risks and Tradeoffs

- **N7's structural test is a weak oracle.** The side table's correctness is only fully validated by N1's red tests (the combined system). If Step 1's structural test passes but Step 2's N1 tests fail, the bug could be in either N7 or N1 — the implementer must bisect via the structural test's invariants. This is the accepted tradeoff of bundling (user decision).
- **`upward_central_edges` signature change ripples into `propagate_beadings_upward`/`downward` and `compute_dist_to_bottom_source`.** The implementer must find all callers (dispatch listed) and update call sites. Risk is contained (the function is private to `propagation.rs`).
- **Fixture re-baseline may mask regressions.** The self-capture pattern locks in *this* implementation's behavior, not OrcaSlicer ground truth. The N1 red tests are the real parity oracle; the fixtures guard self-regression only.
- **Bisect confusion across A1→A2 boundary.** Between A1 and A2, N2/N4 red tests stay red. The "stays red" verification commands gate this, but a future bisect across the boundary will see red tests that are "expected red" — the implementer must record the A1/A2 boundary in commit messages.
