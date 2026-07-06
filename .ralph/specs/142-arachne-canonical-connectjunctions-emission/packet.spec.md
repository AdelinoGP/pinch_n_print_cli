---
status: draft
packet: 142-arachne-canonical-connectjunctions-emission
task_ids:
  - none
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 142-arachne-canonical-connectjunctions-emission

## Goal

Port canonical `connectJunctions` emission (N2 — per-quad junction pairing,
`perimeter_index = bead_idx`, pop-back merge, `addToolpathSegment`-style line
growth) and canonical `is_odd` semantics (N4 — odd-count centerline gap-fill
bead, not odd-indexed inset), so `ExtrusionJunction::perimeter_index` carries
the bead/inset index at generation time and `ExtrusionLine::is_odd` marks only
the centerline bead of an odd-bead-count region.

## Scope Boundaries

Rewrite the line-assembly layer in `generate_toolpaths.rs:401-758`
(`chain_junctions_for_bead`, `emit_chain_lines`, `generate_toolpaths`) to the
canonical per-quad `connectJunctions` scheme, set `perimeter_index = bead_idx`
at junction generation, delete `assign_perimeter_indices` from `pipeline.rs`,
and update `arachne_pipeline.rs:122` in place to the bead-index semantics.
Rewrite `is_odd` computation to the canonical per-segment rule. Full in/out-of-
scope lists live in `requirements.md`.

## Prerequisites and Blockers

- Depends on: `141-arachne-beading-propagation-and-junction-bands` (A1 — needs
  A1's correct upward-half-edge junction fans for the `perimeter_index`
  pop-back merge to be implementable).
- Unblocks: `143-arachne-transition-ends-and-extra-ribs` (B — beading
  interpolation reads the canonical junction fans); `144-arachne-angle-fudge-and-noncentral-regions` (C — the π hack is load-bearing for A1's centrality-gated
  scheme until A1 lands, and C removes it strictly after A2).
- Activation blockers: none (A1 is the only prerequisite; the audit is encoded
  in committed red tests at `b2ea52b7`).

## Acceptance Criteria

Acceptance Criteria are stated **once**, here. `requirements.md` references them
by ID, never copies them.

- **AC-1. Given** a minimal single-central-edge graph with `bead_count = 2` at
  the "to" vertex (the fixture from `arachne_parity_red_perimeter_index.rs`),
  **when** `generate_toolpaths(&graph, &FixedBeadingStrategy)` runs, **then**
  every junction of every line carries `perimeter_index == line.inset_idx` —
  canonical `generateJunctions` sets `junction.perimeter_index = junction_idx`
  at generation time (`SkeletalTrapezoidation.cpp:2064-2077`), and
  `connectJunctions`'s pop-back merge keys on it (`:2302-2314`).
  | `cargo test -p slicer-core --features host-algos --test arachne_parity_red_perimeter_index -- n2_junction_perimeter_index_is_bead_index --nocapture 2>&1 | tee target/test-output-a2-ac1.log`
- **AC-2. Given** the same minimal single-central-edge graph with
  `bead_count = 2` (an EVEN count), **when** `generate_toolpaths` runs, **then**
  no emitted line has `is_odd == true` — canonical `is_odd` requires
  `bead_count % 2 == 1` (`ExtrusionLine.hpp:62-70`,
  `SkeletalTrapezoidation.cpp:2344-2354`); PNP's `bead_idx % 2 == 1`
  mislabelling is removed.
  | `cargo test -p slicer-core --features host-algos --test arachne_parity_red_is_odd_semantics -- n4_even_bead_count_lines_are_never_marked_odd --nocapture 2>&1 | tee target/test-output-a2-ac2.log`
- **AC-3. Given** the same fixture's per-bead lines are < 1 mm long (short open
  polylines), **when** `remove_small_lines(lines, 0.5, 4.0)` runs (threshold
  0.5 × 4.0 = 2.0 mm), **then** every inset-1 (second wall) line survives —
  canonical `remove_small_lines` only removes `is_odd && !is_closed` lines
  (`WallToolPaths.cpp:838-856`), and with the N4 fix the inset-1 line is no
  longer mislabelled `is_odd`.
  | `cargo test -p slicer-core --features host-algos --test arachne_parity_red_is_odd_semantics -- n4_even_inner_wall_survives_remove_small_lines --nocapture 2>&1 | tee target/test-output-a2-ac3.log`

## Negative Test Cases

- **AC-N1. Given** `arachne_pipeline.rs:122`
  (`arachne_pipeline_perimeter_index_is_sequential_per_line`) was updated in
  place by A2 to assert `perimeter_index == line.inset_idx`, **when** the test
  runs against a 10 mm square, **then** it passes (the N2 contract holds at the
  pipeline level too, not just the `generate_toolpaths` layer) — the old
  sequence-position assertion is gone.
  | `cargo test -p slicer-core --features host-algos --test arachne_pipeline -- arachne_pipeline_perimeter_index_is_sequential_per_line --nocapture 2>&1 | tee target/test-output-a2-neg1.log`

## Verification

Gate commands only — the 2–3 commands the preflight / closure gate runs. The
full verification matrix lives in `requirements.md` §Verification Commands.

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-core --features host-algos --test arachne_parity_red_perimeter_index --test arachne_parity_red_is_odd_semantics --no-fail-fast 2>&1 | tee target/test-output-a2-gate.log`

## Authoritative Docs

- `docs/02_ir_schemas.md` — §"Arachne extrusion-line geometry (Packet 112)"
  (lines ~1091-1150) — read directly; purpose: confirm
  `ExtrusionJunction::perimeter_index` (`u32`) and `ExtrusionLine::is_odd`
  (`bool`) field shapes, and confirm NO schema change is needed (the semantic
  change is wire-type-transparent).
- `docs/DEVIATION_LOG.md` `D-113C-FAITHFUL-GRAPH-CONSTRUCTION` entry — read
  full; purpose: substrate A2 builds on (A1 already added the
  `D-141-JUNCTION-BANDS` addendum; A2 adds `D-142-CONNECTJUNCTIONS-EMISSION`).
- `docs/specs/arachne-parity-N1-N13-plan.md` — read full; purpose: cross-packet
  policies (the `arachne_pipeline.rs:122` in-place update decision, the e2e
  record-only policy, the fixture re-baseline distributed-per-packet policy).

## Doc Impact Statement

A list of specific doc sections that this packet adds or modifies:

- `docs/DEVIATION_LOG.md` — new entry `D-142-CONNECTJUNCTIONS-EMISSION`
  documenting the N2+N4 fix (canonical `connectJunctions` emission +
  `perimeter_index = bead_idx` + canonical `is_odd`), with an addendum on
  `D-141-JUNCTION-BANDS` noting A2 supersedes A1's junction *metadata* layer
  (A1 owns the junction *geometry*; A2 owns the metadata + emission). Supersession
  pattern (new ID + addendum, no in-place edits).
  - `rg -q 'D-142-CONNECTJUNCTIONS-EMISSION' docs/DEVIATION_LOG.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:2283-2327` — `connectJunctions` per-quad from/to pairing + `perimeter_index` pop-back merge (`:2302-2314`).
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:2198-2234` — `addToolpathSegment` (extend last `ExtrusionLine` if within 10 µm, else new line; `new_domain_start` fresh-line flag).
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:2344-2354` — canonical `is_odd` per-segment rule (`bead_count % 2 == 1`, `transition_ratio == 0`, innermost junction, endpoint proximity 0.005 mm to peak node).
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:2355-2361` — `passed_odd_edges` dedup keyed on the physical edge (not `(bead, edge, twin)` triple).
- `OrcaSlicerDocumented/src/libslic3r/Arachne/utils/ExtrusionLine.hpp:62-70` — `is_odd` semantics ("centerline bead of an odd bead count, gap-fill, no companion, not a closed loop").
- `OrcaSlicerDocumented/src/libslic3r/Arachne/WallToolPaths.cpp:838-856` — `removeSmallLines` eligibility gate (`is_odd && !is_closed` only).

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.