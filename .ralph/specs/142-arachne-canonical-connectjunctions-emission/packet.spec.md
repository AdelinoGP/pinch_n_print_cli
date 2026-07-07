---
status: implemented
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

## Scope Boundaries

Rewrite the line-assembly layer in `generate_toolpaths.rs:401-758`
(`chain_junctions_for_bead`, `emit_chain_lines`, `generate_toolpaths`) to the
canonical per-quad `connectJunctions` scheme, set `perimeter_index = bead_idx`
at junction generation, delete `assign_perimeter_indices` from `pipeline.rs`,
and update `arachne_pipeline.rs:122` in place to the bead-index semantics.
Rewrite `is_odd` computation to the canonical per-segment rule. **Also
implement 3-or-more-way junction detection in the domain-chain walk itself**
(not just at the `addToolpathSegment` append-decision level) so a walk never
merges two genuinely separate spokes into one chain through a shared branch
vertex — this is the concrete blocker for AC-4 below. Full in/out-of-scope
lists live in `requirements.md`.

## Prerequisites and Blockers

- Depends on: `141-arachne-beading-propagation-and-junction-bands` (A1 — needs
  A1's correct upward-half-edge junction fans for the `perimeter_index`
  pop-back merge to be implementable). **A1's own generate_junctions is now
  fixed and ground-truth-verified (commit `9367d239`) — do not re-derive it;
  the 3 bugs found there (peak-vs-boundary beading anchor, ad hoc per-bead
  width recompute, illegitimate rib exclusion) are pinned by
  `crates/slicer-core/tests/arachne_generate_junctions_canonical_regression.rs`
  and must stay green.**
- **Tightly coupled with A1 in the reverse direction too (discovered
  2026-07-06, not in the original plan):** A1's own AC-1/AC-2
  (`arachne_parity_red_junction_bands.rs`) cannot pass end-to-end until THIS
  packet's 3-way-junction fix lands — see AC-4 below. Treat 141+142 as
  effectively one closure unit, or land this packet immediately after
  confirming A1's `generate_junctions` fix (already committed) rather than as
  a fully independent follow-on.
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
- **AC-4 (added 2026-07-06 — the 3-way-junction / branch-vertex fix; see the
  Goal section's scope correction).** **Given** a plain square (whose medial
  axis is an X, 4 diagonal spokes meeting at the center — a genuine 3+-way
  junction) run through the full `run_arachne_pipeline` with A1's fixed
  (ribs-included, peak-anchored) `generate_junctions`, **when**
  `generate_toolpaths` builds the domain chain, **then** the walk does NOT
  merge two different spokes into one chain through the center vertex — each
  spoke's own outer-wall contribution stays geometrically coherent (no single
  chain segment whose two junctions are both interior spoke points more than
  one spoke length apart, and no junction landing exactly on the shared
  center vertex with an outsized width belonging to a different spoke's
  bead). This is `addToolpathSegment`'s "not a 3-way" check
  (`SkeletalTrapezoidation.cpp:2198-2234`), applied at the domain-walk level,
  not just the per-append level. Concretely, this AC is satisfied when ALL of
  the following (currently failing, for this exact reason) go green without
  weakening any assertion:
  - `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --no-fail-fast 2>&1 | tee target/test-output-a2-ac4-junction-bands.log` (A1's own AC-1/AC-2 — `n1_rectangle_outer_wall_junctions_stay_near_boundary`, `n1_square_outer_wall_junctions_at_outer_bead_radius`)
  - `cargo test -p slicer-core --features host-algos --test generate_toolpaths --no-fail-fast 2>&1 | tee target/test-output-a2-ac4-generate-toolpaths.log` (`outer_wall_closes_for_simple_polygon`, `generate_toolpaths_tapered_wedge`)
  - `cargo test -p slicer-core --features host-algos --test arachne_invariants -- outer_wall_is_closed_ring_for_simple_polygons --nocapture 2>&1 | tee target/test-output-a2-ac4-invariants.log`
  - `cargo test -p slicer-core --features host-algos --test arachne_parity_red_chain_junctions --no-fail-fast 2>&1 | tee target/test-output-a2-ac4-chain-junctions.log` (`constant_radius_chain_to_junction_lands_at_end_vertex_not_start`, `f3_invariant_chain_has_one_junction_per_endpoint_at_shared_vertex` — these currently fail with "expected at least one inset bucket": their hand-built fixtures relied on a flat central edge also emitting to bridge the chain, which canonical's flat-edge skip plus this AC's rib-based connectivity must replace, not merely restore)
  - `cargo test -p slicer-core --features host-algos --test arachne_generate_junctions_canonical_regression --no-fail-fast 2>&1 | tee target/test-output-a2-ac4-junctions-regression.log` (A1's 3 bug-regression locks — must stay green; confirms this packet did not reintroduce A1's fixed bugs while rewriting the chain walk)

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
- `cargo test -p slicer-core --features host-algos --test arachne_parity_red_perimeter_index --test arachne_parity_red_is_odd_semantics --test arachne_parity_red_junction_bands --test arachne_generate_junctions_canonical_regression --no-fail-fast 2>&1 | tee target/test-output-a2-gate.log` (AC-1/AC-2/AC-3 + AC-4's A1-oracle subset + the A1 bug-regression locks, all in one narrow run)

## Authoritative Docs

- `docs/02_ir_schemas.md` — §"Arachne extrusion-line geometry (Packet 112)"
  (lines ~1091-1150) — read directly; purpose: confirm
  `ExtrusionJunction::perimeter_index` (`u32`) and `ExtrusionLine::is_odd`
  (`bool`) field shapes, and confirm NO schema change is needed (the semantic
  change is wire-type-transparent).
- `docs/DEVIATION_LOG.md` `D-113C-FAITHFUL-GRAPH-CONSTRUCTION` entry — read
  full; purpose: substrate A2 builds on (A1 already added the
  `D-141-JUNCTION-BANDS` addendum; A2 adds `D-142-CONNECTJUNCTIONS-EMISSION`).
- `docs/DEVIATION_LOG.md` `D-141-JUNCTION-BANDS`'s **2026-07-06 correction
  note** — read full; purpose: this packet's AC-4 exists because of exactly
  this finding (3-way-junction gap in the chain walk, exposed once A1's
  `generate_junctions` was fixed to be genuinely canonical). Do not
  re-discover this from scratch — the correction note has the full root-cause
  account and the specific failing test names.
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