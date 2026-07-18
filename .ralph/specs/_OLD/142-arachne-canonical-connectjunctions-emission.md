---
status: implemented
packet: 142-arachne-canonical-connectjunctions-emission
task_ids:
  - none
---

# 142-arachne-canonical-connectjunctions-emission

## Goal

Port canonical `connectJunctions` emission (N2 — per-quad junction pairing,
`perimeter_index = bead_idx`, pop-back merge, `addToolpathSegment`-style line
growth **including its 3-or-more-way-junction detection**) and canonical
`is_odd` semantics (N4 — odd-count centerline gap-fill bead, not odd-indexed
inset), so `ExtrusionJunction::perimeter_index` carries the bead/inset index
at generation time, `ExtrusionLine::is_odd` marks only the centerline bead of
an odd-bead-count region, **and the domain-chain walk stops/splits at a
genuine branch vertex (3+ edges meeting) instead of driving straight through
it**.

**2026-07-06 scope correction (read before starting this packet — see
`docs/DEVIATION_LOG.md`'s `D-141-JUNCTION-BANDS` correction note for the full
account):** after A1's `generate_junctions` was fixed to be genuinely
canonical (peak-anchored beading, ribs included, no centrality/type gate —
commit `9367d239`), the *existing* chain walk in `generate_toolpaths`
(`find_quad` + a plain `.twin` hop, `chain_junctions_for_bead`'s width-based
merge) was found to have **no concept of a 3-or-more-way junction at all**.
For a plain square, the medial axis is an X (4 diagonal spokes meeting at the
center); once ribs correctly carry junction data (as canonical requires), the
current walk drives straight through that center vertex, merging two
unrelated spokes into one fragmented, sometimes-6mm-wide-junction chain,
instead of recognizing the center as a branch point and stopping/splitting
there — exactly what canonical's `addToolpathSegment` "not a 3-way" check
exists to prevent (previously masked because A1's original, buggy
implementation excluded ribs, which incidentally also avoided ever routing
through real branch points for these fixtures). **This is not new scope** —
`addToolpathSegment`'s 3-way check was already named in this packet's own
`requirements.md`/`design.md` — but it was not previously reflected in a
concrete, testable acceptance criterion. AC-4 below makes it one.

## Problem Statement

Packet 141 (A1) fixed junction *geometry* — canonical `generateJunctions`
(upward half-edges, in-band beads, single `get_beding` at peak). But the layer
that assembles those junctions into toolpath *lines* remains divergent in two
blocking ways (N2 + N4), and one in-tree test
(`arachne_pipeline.rs:122`) actively asserts the divergent semantics A2 must
correct.

**N2 (line assembly):** PNP's `chain_junctions_for_bead` /
`emit_chain_lines` / `generate_toolpaths` (`generate_toolpaths.rs:401-758`)
collects every central `NORMAL` edge into one `full_chain`, then per bead index
emits one polyline spanning the chain, merging at shared vertices by *wider
width*. `ExtrusionJunction::perimeter_index` is zeroed at generation
(`:299-306`) and later overwritten by `assign_perimeter_indices`
(`pipeline.rs:384-390`) with the junction's *sequence position within its
line*. Canonical `connectJunctions` (`SkeletalTrapezoidation.cpp:2283-2327`)
instead pairs junctions **per quad** (`from_junctions` = junctions of
`edge_to_peak`, `to_junctions` = junctions of `edge_from_peak->twin`), merges
secondary fans by **`perimeter_index` pop-back dedup** (not width), and grows
lines via `addToolpathSegment` (`:2198-2234`) — extend the last `ExtrusionLine`
if the new `from` is within 10 µm of its last junction (same width, not a
3-way), else start a new line. `perimeter_index` on each junction **is the
bead/inset index** (`junction_idx`), which is what the pop-back rule keys on.
PNP's redefinition ("index within the wall sequence at that vertex",
`pipeline.rs:378-390`) breaks any downstream consumer expecting Orca semantics
and makes the pop-back rule unimplementable without re-plumbing.

**N4 (`is_odd`):** PNP sets `is_odd = bead_idx % 2 == 1`
(`generate_toolpaths.rs:632`) — "odd-indexed inset". Canonical
(`ExtrusionLine.hpp:62-70`) is "centerline bead of an odd bead count — a
gap-fill line with no companion on the other side, not a closed loop",
computed per segment in `connectJunctions` (`:2344-2354`): requires
`bead_count % 2 == 1`, `transition_ratio == 0`, the junction being the
innermost of the fan, and endpoint proximity (0.005 mm) to the quad's peak node.
With PNP's definition every 2nd, 4th, … wall is classified as gap-fill:
`remove_small_lines` (`arachne/remove_small.rs:57`, mirroring
`WallToolPaths.cpp:838-856`) only removes `is_odd && !is_closed` lines, so
short open fragments of REAL inner walls get silently deleted; the stitcher
groups by `is_odd` (`stitch.rs:83`), so mislabelled walls can't join their
peers; and the flag is forwarded verbatim across the host boundary
(`slicer-wasm-host/src/host.rs:1818`, `slicer-sdk/src/host.rs:721`).

This packet supersedes `D-141-JUNCTION-BANDS` for the junction-metadata +
emission layer only; A1's junction *geometry* (upward-half-edge, in-band,
no-clamp) remains canonical and untouched. A2 also corrects the in-tree test
`arachne_pipeline.rs:122` (`arachne_pipeline_perimeter_index_is_sequential_per_line`),
which actively asserts the divergent sequence-position semantics — a conflict
the audit didn't flag but grilling surfaced (user decision: update in place to
bead-index semantics, same test name, new assertion).

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- Packet-specific constraint: **`perimeter_index` semantic change is wire-type-transparent.** `ExtrusionJunction::perimeter_index` is `u32` at `slicer-ir::slice_ir.rs:1744,1798`, forwarded verbatim through `slicer-sdk/src/host.rs:717` and `slicer-wasm-host/src/host.rs:1814`. The semantic change (bead index vs sequence position) does NOT change the wire type — NO schema change, NO WIT change. A2 must NOT edit `slicer-sdk/src/host.rs` or `slicer-wasm-host/src/host.rs`; the change is transparent at the boundary. The only in-tree consumer of the old semantics is `arachne_pipeline.rs:122` (updated in place).
- Packet-specific constraint: **A2 must keep N1 red tests GREEN** (`arachne_parity_red_junction_bands.rs`'s AC-1/AC-2 — currently RED, not because A1's `generate_junctions` is wrong, but because THIS packet's own 3-way-junction fix (AC-4) hasn't landed yet) **and must keep `arachne_generate_junctions_canonical_regression.rs`'s 3 tests GREEN throughout** (they pin A1's peak-anchor/own-array-width/rib-inclusion fixes in isolation from the chain walk this packet rewrites). A2 builds on A1's junction geometry; regressing A1's `generate_junctions` rewrite while rewriting the surrounding chain walk means backing out — re-run the regression file after every step that touches `generate_toolpaths.rs`.
- Packet-specific constraint: **the domain-chain walk must detect and correctly handle 3-or-more-way junctions** (a vertex where 3+ edges' own "quads" converge — e.g. a plain square's medial-axis center, where 4 diagonal spokes meet). The current walk (`find_quad` + a plain `.twin` hop off the quad's dead end) has no such detection and will merge unrelated spokes into one fragmented chain once ribs correctly carry junction data (post-A1-fix). This is `addToolpathSegment`'s "not a 3-way" check (`SkeletalTrapezoidation.cpp:2198-2234`) applied at the WALK level, not just the per-append level — see AC-4 in `packet.spec.md` for the concrete, currently-failing tests this must fix.
- Packet-specific constraint: **A2 must NOT remove the π hack (`pipeline.rs:334`) or the 0.1× filter-dist fudge (`pipeline.rs:272-277`).** Those are Packet C's (`144`) scope, strictly after A2.
- Packet-specific constraint: **WASM staleness does NOT apply** — A2's change surface is `slicer-core`-internal; no path feeds the guest WASM build. The `wasm-staleness` snippet is intentionally omitted.

## Data and Contract Notes

- IR or manifest contracts touched: **none**. `ExtrusionJunction::perimeter_index` stays `u32`; `ExtrusionLine::is_odd` stays `bool`. The semantic change is wire-type-transparent at `slicer-sdk/src/host.rs:717` and `slicer-wasm-host/src/host.rs:1814` — both files are NOT edited.
- WIT boundary considerations: **none**. No WIT/IR schema change. The `perimeter_index` semantic change is a `slicer-core`-internal contract change that is transparent at the host boundary (the field's wire type is unchanged).
- Determinism: A2's rewrite preserves determinism (per-quad pairing is index-ordered; the pop-back merge is deterministic given the `perimeter_index` values; `is_odd` is a deterministic per-segment predicate). `passed_odd_edges` is a `BTreeSet`/`HashSet` of physical edge indices (deterministic under ties via index-ascending).

## Locked Assumptions and Invariants

- `perimeter_index = bead_idx` is set at junction *generation* (in A1's rewritten `generate_junctions`), NOT in a post-pass. `assign_perimeter_indices` is deleted.
- `is_odd` is computed per segment during `connectJunctions`, not as a post-pass on `ExtrusionLine`.
- `passed_odd_edges` is keyed on the physical edge index, not `(bead, edge, twin)` triple.
- `arachne_pipeline.rs:122` is updated in place (same test name, new assertion) — explicit in the commit message.
- `slicer-sdk/src/host.rs:717` and `slicer-wasm-host/src/host.rs:1814` are NOT edited — wire-type-transparent.
- A2 keeps N1 red tests GREEN (gated) and N3 red tests RED (gated).
- A2 does NOT remove the π hack or the 0.1× filter-dist fudge (Packet C's scope).
- Fixture re-baseline uses the self-capture pattern; never read the JSONs directly.

## Risks and Tradeoffs

- **The `connectJunctions` per-quad walk is the most complex rewrite in the A1→A2 chain.** It replaces a whole-chain-polyline-per-bead scheme with per-quad pairing + pop-back merge + `addToolpathSegment` line growth. Risk is contained by the N2 red test (the pop-back merge's observable is `perimeter_index == inset_idx`) and the existing `generate_toolpaths`/`stitch`/`remove_small` regression suite.
- **`is_odd` change affects `stitch.rs:83` grouping and `remove_small.rs:57` eligibility.** The consumers are unchanged (A2 changes the producer); the regression suite gates this. The N4 red tests are the parity oracle.
- **`arachne_pipeline.rs:122` in-place update could mask a regression if the new assertion is too weak.** The N2 red test (`arachne_parity_red_perimeter_index.rs`) is the strict oracle; the pipeline-level test is a regression guard, not the primary oracle.
- **Bisect confusion across A1→A2 boundary.** Between A1 and A2, N2/N4 red tests stay red. A2's commit message must record the boundary.
