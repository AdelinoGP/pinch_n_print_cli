---
status: implemented
packet: 141-arachne-beading-propagation-and-junction-bands
task_ids:
  - none
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 141-arachne-beading-propagation-and-junction-bands

## Goal

Port `BeadingPropagation` + `getBeading` (N7) and rewrite `generate_junctions` to
the canonical upward-half-edge/in-band/no-clamp scheme (N1), so junctions ride
the carrier edge whose radius band contains the bead's target radius (ribs
included, no centrality gate, single beading at the peak node), making the
outer wall land at `preferred_bead_width_outer / 2` from the boundary.

## Known Implementation Hazard — read before starting or resuming this packet

**2026-07-06:** a first implementation of this packet's Step 2 (commit
`798bb324`) deviated from this packet's OWN already-correct brief (this Goal
section) in three concrete, ground-truth-verified ways, then got AC-1/AC-2
green anyway because the two test fixtures (a rectangle, a square) happen to
only ever touch bead index 0 within their tolerance bands — masking all
three bugs. Fixed in commit `9367d239`; see `docs/DEVIATION_LOG.md`'s
`D-141-JUNCTION-BANDS` correction note for the full account. **If you are
re-implementing or reverting this packet's Step 2 from scratch, do not
repeat these:**

1. **Resolve the beading at `edge.from` (boundary-side) as the primary path,
   falling back to `edge.to` (peak) only if empty.** Canonical always
   resolves at the peak (`getOrCreateBeading(edge->to, ...)`,
   `SkeletalTrapezoidation.cpp:2029`) — never the boundary side, and never as
   a fallback-only case. `crates/slicer-core/tests/arachne_generate_junctions_canonical_regression.rs::generate_junctions_resolves_beading_at_peak_not_boundary_side`
   pins this directly.
2. **Recompute each junction's width via a fresh `strategy.compute(2 * r,
   bead_count).bead_widths.first()` call per bead**, instead of reading
   `beading.bead_widths[idx]` directly from the ONE resolved (peak) beading.
   The `.first()` call always returns index 0 regardless of the bead's own
   index, so every bead on the same edge collapses to (at most) two
   identical values. Canonical writes `beading->bead_widths[junction_idx]`
   directly (`:2076`), no recompute.
   `arachne_generate_junctions_canonical_regression.rs::generate_junctions_reads_width_from_beadings_own_array_per_bead_index`
   pins this directly.
3. **Keep an `if !edge.central { continue }` / `if edge.edge_type ==
   EdgeType::EXTRA_VD { continue }` gate in `generate_junctions`, or add a
   matching centrality/type filter to `generate_toolpaths`'s domain-start
   seeding.** Canonical has ZERO edge-type/centrality checks anywhere in
   `generateJunctions` or in the `unprocessed_quad_starts` seed (confirmed by
   a direct ground-truth re-read of `SkeletalTrapezoidation.cpp:2015` and
   `:2265-2269`) — ribs are the primary near-boundary junction carrier, not
   an exclusion. `arachne_generate_junctions_canonical_regression.rs::generate_junctions_does_not_exclude_ribs`
   pins this directly.

**Run `cargo test -p slicer-core --features host-algos --test arachne_generate_junctions_canonical_regression --no-fail-fast` before considering any change to `generate_junctions` (or the domain-seeding in `generate_toolpaths`) complete — all 3 must stay green.**

**Separately, note that fixing these three bugs correctly exposes a real,
pre-existing gap ONE LAYER DOWN**, in `generate_toolpaths`'s chain walk (no
3-or-more-way junction detection, no flat-edge-connectivity replacement now
that ribs carry it) — this is Packet 142's scope, not this packet's, and
Packet 141's own AC-1/AC-2 cannot go fully green until Packet 142 lands (see
that packet's `packet.spec.md` AC-4). Do not attempt to "fix" AC-1/AC-2 by
re-introducing bug 3 above (excluding ribs) to keep the old, simpler chain
walk's assumptions intact — that is exactly the mistake this section exists
to prevent.

## Scope Boundaries

Rewrite `generate_junctions` (`generate_toolpaths.rs:192-334`) to the canonical
scheme and add the `BeadingPropagation` side table + `getBeading`-equivalent that
feeds it. Touch `upward_central_edges` / `propagate_beadings_downward` /
`interpolate_bead_counts` (`propagation.rs`) to drop the centrality gate and
interpolate bead widths/locations rather than rounded integer counts. Full
in/out-of-scope lists live in `requirements.md`.

## Prerequisites and Blockers

- Depends on: `113c-arachne-faithful-graph-construction` (`status: implemented`)
  — its interleaved-rib graph topology is the substrate this packet's junction
  generation walks. This is a hard technical prerequisite that happens to
  already be satisfied; `docs/specs/arachne-parity-N1-N13-plan.md`'s "Prereqs:
  none" for A1 means "no *new* packet in this N1-N13 chain must land first,"
  not "no dependency at all" — the two statements are consistent, not
  conflicting.
- Unblocks: `142-arachne-canonical-connectjunctions-emission` (N2+N4 — needs
  A1's correct junction fans for the `perimeter_index` pop-back merge); `143-arachne-transition-ends-and-extra-ribs` (N3+N8 — B's beading interpolation reads
  A1's junction fans); `144-arachne-angle-fudge-and-noncentral-regions` (N5+N6 —
  the π hack is load-bearing for the centrality-gated scheme A1 replaces).
- **Reverse coupling (discovered 2026-07-06, not in the original plan):** A1's
  own AC-1/AC-2 below cannot pass end-to-end until 142's 3-way-junction fix
  lands (142's `packet.spec.md` AC-4) — 142 is not merely unblocked by A1, it
  is also a hard prerequisite for A1's own closure. Treat 141+142 as
  effectively one closure unit.
- Activation blockers: none (audit is encoded in committed red tests at
  `b2ea52b7`; canonical OrcaSlicer refs are delegated).

## Acceptance Criteria

Acceptance Criteria are stated **once**, here. `requirements.md` references them
by ID, never copies them.

- **AC-1. Given** a 20×4 mm rectangle whose medial axis has a long
  constant-radius (R = 2 mm) horizontal spine, **when**
  `run_arachne_pipeline(&[rect], &ArachneParams::default(), false)` runs, **then**
  every inset-0 (outer wall) junction lies within 0.6 mm of the rectangle
  boundary (3× the canonical ~0.2 mm placement, generous slack for corner
  geometry and preprocessing offsets) — canonical `generateJunctions` skips flat
  edges (`SkeletalTrapezoidation.cpp:2024-2027`) and places outer-wall junctions
  on the rib edge at the outer bead's target radius, never on the medial axis.
  | `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands -- n1_rectangle_outer_wall_junctions_stay_near_boundary --nocapture 2>&1 | tee target/test-output-a1-ac1.log`
  **Status 2026-07-06: currently RED, not because `generate_junctions` is wrong
  (it is now genuinely canonical, commit `9367d239`) but because Packet 142's
  chain-walk 3-way-junction fix hasn't landed — see the Known Implementation
  Hazard section above and 142's AC-4. Do not attempt to make this green by
  changing `generate_junctions` further; the fix belongs in 142.**
- **AC-2. Given** a 10 mm square, **when** `run_arachne_pipeline` runs, **then**
  every inset-0 junction deviates ≤ 0.15 mm from the canonical outer-bead radius
  (0.2 mm from the boundary = `preferred_bead_width_outer / 2`) — canonical
  `generateJunctions` resolves one beading at the peak node via `getBeading`
  (`SkeletalTrapezoidation.cpp:2064-2077`, `:2091-2127`) and emits only in-band
  beads, not per-endpoint beadings computed from the endpoint's own domain-max
  `bead_count`.
  | `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands -- n1_square_outer_wall_junctions_at_outer_bead_radius --nocapture 2>&1 | tee target/test-output-a1-ac2.log`
  **Status 2026-07-06: same as AC-1 — currently RED pending Packet 142.**
- **AC-3 (added 2026-07-06). Given** the three isolated-`generate_junctions`
  bug regressions found in the first Step 2 implementation (peak-vs-boundary
  beading anchor, ad hoc per-bead width recompute, illegitimate rib
  exclusion — see the Known Implementation Hazard section above), **when**
  `generate_junctions` is called directly on small hand-built graphs (no
  chain walk involved), **then** all three bugs stay fixed. Unlike AC-1/AC-2,
  this AC is isolated from Packet 142's still-open chain-walk gap and must be
  green NOW.
  | `cargo test -p slicer-core --features host-algos --test arachne_generate_junctions_canonical_regression --no-fail-fast 2>&1 | tee target/test-output-a1-ac3.log`

## Negative Test Cases

- **AC-N1. Given** a single central twin-pair edge graph with a `TransitionMiddle`
  at pos 0.5 (the same fixture as `arachne_parity_red_transition_ends.rs`), **when**
  `generate_toolpaths(&graph, &FixedBeadingStrategy)` runs (A1's surface only — no
  `apply_transitions`), **then** the edge emits junctions only on the upward
  half-edge (the one with `from.R < to.R`), never on both halves — the half-fan
  double-emission that forced the odd-bead dedup gymnastics is removed by
  construction. Asserted via a new structural test in A1's own implementation.
  | `cargo test -p slicer-core --features host-algos --test arachne_junction_upward_half_edge_only --nocapture 2>&1 | tee target/test-output-a1-neg1.log`

## Verification

Gate commands only — the 2–3 commands the preflight / closure gate runs. The
full verification matrix lives in `requirements.md` §Verification Commands.

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-core --features host-algos --test arachne_generate_junctions_canonical_regression --no-fail-fast 2>&1 | tee target/test-output-a1-gate.log` (AC-3 — MUST be green; isolated from Packet 142's chain-walk gap)
- `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --no-fail-fast 2>&1 | tee target/test-output-a1-gate-ac12.log` (AC-1/AC-2 — currently RED pending Packet 142; do not block finalizing THIS packet's own `generate_junctions`/N7 work on this, but do not flip `packet.spec.md` to `status: implemented` until it's green either — see the reverse-coupling note under Prerequisites and Blockers)

## Authoritative Docs

- `docs/08_coordinate_system.md` — read §"Constant Conversion Table" only (~30
  lines); purpose: unit conversion for any new per-vertex/per-cell fields.
  Delegate if > 300 lines.
- `docs/02_ir_schemas.md` — §"Arachne extrusion-line geometry (Packet 112)"
  (lines ~1091-1150) — read directly; purpose: confirm `ExtrusionJunction` /
  `ExtrusionLine` field shapes A1's `generate_junctions` emits into.
- `docs/DEVIATION_LOG.md` `D-113C-FAITHFUL-GRAPH-CONSTRUCTION` entry — read full;
  purpose: understand the substrate A1 builds on (Steps 1-8b done; Steps 9-10
  fixture re-baseline + e2e closure are A1's inheritance, not silently absorbed).
- `docs/specs/arachne-parity-N1-N13-plan.md` — read full; purpose: this packet's
  grilling-validated decomposition and cross-packet policies.

## Doc Impact Statement

A list of specific doc sections that this packet adds or modifies:

- `docs/DEVIATION_LOG.md` — new entry `D-141-JUNCTION-BANDS` documenting the
  N1+N7 fix (junction generation rewrite + `BeadingPropagation` side table),
  with an addendum on `D-113C-FAITHFUL-GRAPH-CONSTRUCTION` noting Steps 9-10
  (fixture re-baseline + e2e closure) are inherited by this packet chain, not
  silently absorbed. Supersession pattern (new ID + addendum, no in-place edits).
  - `rg -q 'D-141-JUNCTION-BANDS' docs/DEVIATION_LOG.md`
  - `rg -q 'D-141-JUNCTION-BANDS' docs/DEVIATION_LOG.md && rg -q -e 'inherited.*141' -e '141.*inherit' docs/DEVIATION_LOG.md`
  - **Already done (2026-07-06):** the same `D-141-JUNCTION-BANDS` row also
    carries a `**Correction 2026-07-06**` paragraph (in-place addition to
    this packet's own still-open entry, not a supersession — this is the
    same packet's own draft-state correction, not a different historical
    packet's closed entry) documenting the 3 bugs and the AC-1/AC-2 ↔
    Packet-142 coupling. Do not duplicate it; extend it if this packet's
    understanding changes further.
    - `rg -q 'Correction 2026-07-06' docs/DEVIATION_LOG.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:2013-2079` — `generateJunctions` — the upward-half-edge / in-band / no-clamp / single-beading-at-peak scheme A1 ports.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:2091-2127` — `getBeading` / `getNearestBeading` (0.1 mm radius) — the propagation/nearest-lookup A1 ports as the `BeadingPropagation` side table.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:1669-1672` — `upward_quad_mids` (no centrality filter) — A1 drops the centrality gate from `upward_central_edges`.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:1805-1814` — `propagateBeadingsUpward` skip guards (bead-count/hasBeading, not centrality).
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:1833-1899` — `propagateBeadingsDownward` — skips central edges, routes non-central equidistant via twin, interpolates `ratio_of_top` over bead widths/locations (not rounded integer counts).
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:1883-1885` — `ratio_of_top` blend (the smooth width falloff A1's `interpolate_bead_counts` replacement produces).

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.