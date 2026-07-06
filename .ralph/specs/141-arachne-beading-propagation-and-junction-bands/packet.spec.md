---
status: draft
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
  generation walks.
- Unblocks: `142-arachne-canonical-connectjunctions-emission` (N2+N4 — needs
  A1's correct junction fans for the `perimeter_index` pop-back merge); `143-arachne-transition-ends-and-extra-ribs` (N3+N8 — B's beading interpolation reads
  A1's junction fans); `144-arachne-angle-fudge-and-noncentral-regions` (N5+N6 —
  the π hack is load-bearing for the centrality-gated scheme A1 replaces).
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
- **AC-2. Given** a 10 mm square, **when** `run_arachne_pipeline` runs, **then**
  every inset-0 junction deviates ≤ 0.15 mm from the canonical outer-bead radius
  (0.2 mm from the boundary = `preferred_bead_width_outer / 2`) — canonical
  `generateJunctions` resolves one beading at the peak node via `getBeading`
  (`SkeletalTrapezoidation.cpp:2064-2077`, `:2091-2127`) and emits only in-band
  beads, not per-endpoint beadings computed from the endpoint's own domain-max
  `bead_count`.
  | `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands -- n1_square_outer_wall_junctions_at_outer_bead_radius --nocapture 2>&1 | tee target/test-output-a1-ac2.log`

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
- `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --no-fail-fast 2>&1 | tee target/test-output-a1-gate.log`

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
  - `rg -q 'D-141-JUNCTION-BANDS' docs/DEVIATION_LOG.md && rg -q 'inherited.*141\|141.*inherit' docs/DEVIATION_LOG.md`

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