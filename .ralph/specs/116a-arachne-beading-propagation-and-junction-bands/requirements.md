# Requirements: 116a-arachne-beading-propagation-and-junction-bands

## Packet Metadata

- Grouped task IDs: **none** (this is un-packeted remediation continuing past
  packet 113c, provenanced by the second-pass Arachne parity audit
  `target/arachne_parity_audit_20260706_020657.md` findings N7 and N1, encoded
  as committed red tests at `b2ea52b7`; the crosswalk is
  `docs/DEVIATION_LOG.md`'s `D-113C-FAITHFUL-GRAPH-CONSTRUCTION` entry, which
  this packet supersedes for the junction-generation layer).
- Backlog source: `docs/07_implementation_status.md` (no `TASK-###` for N1–N13 —
  matching 113c's `none` precedent; provenanced by the audit + red tests).
- Packet status: `draft`
- Aggregate context cost: `M`

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

## In Scope

- **`BeadingPropagation` side table** (NEW) in
  `crates/slicer-core/src/skeletal_trapezoidation/`: a per-node table holding a
  full `Beading` + source distances, mirroring upstream's
  `SkeletalTrapezoidation::beading_propagation` (`SkeletalTrapezoidation.cpp`
  side-table referenced at `:2091-2127`). Stored alongside
  `SkeletalTrapezoidationGraph` (not on `STVertex` — keeps the vertex struct
  small and matches upstream's side-table-not-vertex-field layout).
- **`getBeading`-equivalent** (NEW): resolve a `Beading` for any node —
  primary source vertices return their own `compute(2R, bead_count)`; rib-foot
  nodes (no `bead_count`) resolve via the `BeadingPropagation` side table's
  nearest lookup (0.1 mm `getNearestBeading` radius). Used by `generate_junctions`
  to compute ONE beading at the peak node (`edge.to`), not per-endpoint.
- **Canonical `generateJunctions` rewrite** in
  `crates/slicer-core/src/arachne/generate_toolpaths.rs:192-334`:
  - iterate ALL graph edges with **no centrality gate** (ribs included — ribs
    are the main junction carriers in constant-bead-count regions);
  - skip non-upward half-edges (`from.R > to.R` → continue,
    `SkeletalTrapezoidation.cpp:2017`) so each physical edge emits from exactly
    one side;
  - skip flat edges and edges whose endpoints share the same resolved bead count
    (`(from.bead_count == to.bead_count && bead_count >= 0) || end_R >= start_R`
    → continue, `:2024-2027`) — constant-radius central spine edges carry no
    junctions;
  - compute ONE beading at `edge.to` (the peak node) via `getBeading`
    (`:2038` area);
  - emit ONLY beads whose `toolpath_locations[idx]` lies within
    `[end_R, start_R]` — loop starts at the middle bead index
    (`(max(1,n)-1)/2`, `:2046`) and `break`s once `bead_R < end_R` (`:2068`);
    out-of-band beads are skipped (never clamped);
  - near-`start_R` beads snap to the start node (`:2072`).
- **`upward_central_edges` centrality gate removal** in
  `crates/slicer-core/src/skeletal_trapezoidation/propagation.rs:126-159`:
  drop the `.filter(|(_, e)| e.central)` so the set matches upstream's
  `upward_quad_mids = edges with prev && next && isUpward()` (no centrality
  filter, `SkeletalTrapezoidation.cpp:1669-1672`).
- **`propagate_beadings_downward` interpolation fix** in
  `crates/slicer-core/src/skeletal_trapezoidation/propagation.rs:982-1095`:
  skip central edges, route non-central equidistant edges via the twin
  (`:1836-1846`), and interpolate **bead widths/locations** (`ratio_of_top`,
  `:1883-1885`), not the rounded integer bead count. `interpolate_bead_counts`
  (`:818-822`) is replaced by a width/location blend that writes into the
  `BeadingPropagation` side table.
- **`primary_source_vertices` centrality gate relaxation** in
  `crates/slicer-core/src/skeletal_trapezoidation/propagation.rs:844-856`:
  the "to vertices of central edges" gate must align with the
  `upward_central_edges` change — primary sources are now "to vertices of
  upward edges", not centrality-gated.
- **`Beading` invariant** (`bead_widths.len() == toolpath_locations.len()`)
  preserved on every entry written into the side table; debug-assert in
  `getBeading`'s hot path.
- **Fixture re-baseline (this packet's own stage only)**:
  `crates/slicer-core/tests/fixtures/arachne/centrality_*.json`,
  `propagation_*.json`, `bead_count_tapered_wedge.json`,
  `toolpaths_tapered_wedge.json` — re-record via the self-capture pattern
  (first-run writes if missing, subsequent runs compare). Each re-baseline
  records rationale in its own commit message. Never read the JSONs directly.
- **Deviation-log entry**: `D-116A-JUNCTION-BANDS` (new ID, addendum on
  `D-113C-FAITHFUL-GRAPH-CONSTRUCTION`, supersession pattern — no in-place edits
  to 113c's narrative).
- **Structural test** for AC-N1: `arachne_junction_upward_half_edge_only.rs`
  (NEW) — a single central twin-pair edge graph asserting junctions emit only
  on the upward half-edge.
- **Scope decision (NOT a silent absorb)**: `ExtrusionJunction::perimeter_index`
  is left at `0` by A1 (A2 owns the `perimeter_index = bead_idx` fix); A1 does
  NOT touch `arachne_pipeline.rs:122`
  (`arachne_pipeline_perimeter_index_is_sequential_per_line`) — leave it red
  until A2. `assign_perimeter_indices` (`pipeline.rs:384-390`) stays in place
  for A1; A2 deletes it.

## Out of Scope

- **N2 (`perimeter_index = bead_idx`) and N4 (`is_odd` semantics)** — owned by
  Packet A2 (`116b-arachne-canonical-connectjunctions-emission`). A1 leaves
  both red.
- **N3 (transition ends) and N8 (`generateExtraRibs`)** — Packet B. A1 does not
  touch `apply_transitions` or add `generateAllTransitionEnds`.
- **N5 (π hack) and N6 (`filterNoncentralRegions`)** — Packet C. A1 does NOT
  remove the π workaround (`pipeline.rs:334`); it is load-bearing for the
  centrality-gated scheme until A1's rewrite lands, and Packet C removes it
  strictly after A2.
- **N9–N13** — Packets D, E, F.
- **`cube_4color.3mf` e2e closure gate** — record-only across A1 (per
  `docs/specs/arachne-parity-N1-N13-plan.md` cross-cutting policy); Packet F
  blocks on green. A1 records the failure delta in its commit message.
- **`cargo test --workspace`** — only at Packet F's closure ceremony; A1 uses
  targeted per-crate commands.
- **Classic-perimeters edits** — M1 frozen.
- **Spiral-vase, non-planar** — orthogonal sibling roadmaps.
- **New WIT/IR schema changes** — `ExtrusionJunction::perimeter_index` stays
  `u32` (wire-type-transparent; the semantic change is A2's scope decision, not
  A1's).
- **`OrcaSlicerDocumented/` C++ oracle build** — declined (matching 113c
  precedent); self-captured fixtures + red tests only.

## Authoritative Docs

- `docs/08_coordinate_system.md` — §"Constant Conversion Table" (~30 lines);
  purpose: unit conversion for any new per-vertex/per-cell fields. Delegate if
  > 300 lines.
- `docs/02_ir_schemas.md` — §"Arachne extrusion-line geometry (Packet 112)"
  (lines ~1091-1150) — read directly (small section); purpose: confirm
  `ExtrusionJunction` / `ExtrusionLine` field shapes A1 emits into.
- `docs/DEVIATION_LOG.md` `D-113C-FAITHFUL-GRAPH-CONSTRUCTION` entry — read
  full; purpose: the substrate A1 builds on.
- `docs/specs/arachne-parity-N1-N13-plan.md` — read full; purpose: this
  packet's grilling-validated decomposition and cross-packet policies.
- `docs/adr/0034-arachne-faithful-graph-construction.md` — read full (short);
  purpose: the architectural decision A1 inherits (faithful port, not
  approximation; OrcaSlicer-read delegation losing caller-loop context).
- `.ralph/specs/113c-arachne-faithful-graph-construction/requirements.md`
  §"OrcaSlicer Reference Obligations" (the `orca-delegation` snippet) — read
  the snippet only; A1 carries this contract forward verbatim.

All other docs are not authoritative for this packet.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:2013-2079` — `generateJunctions` — the upward-half-edge / in-band / no-clamp / single-beading-at-peak scheme A1 ports.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:2091-2127` — `getBeading` / `getNearestBeading` (0.1 mm radius) — the propagation/nearest-lookup A1 ports as the `BeadingPropagation` side table.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:1669-1672` — `upward_quad_mids` (no centrality filter) — A1 drops the centrality gate from `upward_central_edges`.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:1805-1814` — `propagateBeadingsUpward` skip guards.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:1833-1899` — `propagateBeadingsDownward` — skips central edges, interpolates `ratio_of_top` over bead widths/locations.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:1883-1885` — `ratio_of_top` blend.

## Acceptance Summary

Reference Acceptance Criteria by ID; do not copy them.

- Positive cases: `AC-1` (rectangle outer-wall junctions near boundary),
  `AC-2` (square outer-wall junctions at outer-bead radius) from `packet.spec.md`.
  Both are red tests committed at `b2ea52b7` and currently FAIL; A1 is done
  when they pass **without weakened assertions**.
- Negative cases: `AC-N1` (upward-half-edge-only emission — structural test
  for the half-fan double-emission removal).
- Cross-packet impact: unblocks `116b` (A2 — needs A1's junction fans),
  `117` (B — needs A1's fans for beading interpolation),
  `118` (C — the π hack A1 leaves in place is load-bearing until C removes it
  strictly after A2).
- Refinements not captured in Given/When/Then:
  - `BeadingPropagation` is a side table, NOT a field on `STVertex` — keeps the
    vertex struct small and matches upstream's side-table layout. A1's
    implementer decides the concrete Rust type (likely `Vec<Beading>` indexed by
    vertex, or a `HashMap<usize, Beading>` if sparse — the audit confirmed
    upstream keeps a full `Beading` per node, so `Vec<Beading>` is the expected
    shape, with a sentinel for "no beading yet").
  - `getBeading`'s 0.1 mm `getNearestBeading` radius is in slicer units (1000
    units = 0.1 mm = 100 µm) — divide OrcaSlicer's 0.1 mm by the unit factor
    (1 unit = 100 nm) per `docs/08_coordinate_system.md`.
  - A1 does NOT remove the π hack — it is load-bearing for the centrality-gated
    scheme until A1's rewrite lands. Packet C (`118`) removes it strictly after
    A2.
  - A1 does NOT touch `arachne_pipeline.rs:122`
    (`arachne_pipeline_perimeter_index_is_sequential_per_line`) — A2 owns that
    in-place update.
  - A1 leaves `assign_perimeter_indices` (`pipeline.rs:384-390`) in place; A2
    deletes it.

## Verification Commands

Full verification matrix. `packet.spec.md` §Verification carries only the gate
subset.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands -- n1_rectangle_outer_wall_junctions_stay_near_boundary --nocapture 2>&1 \| tee target/test-output-a1-ac1.log` | AC-1: rectangle outer-wall near boundary | FACT pass/fail; SNIPPETS ≤ 20 lines on failure |
| `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands -- n1_square_outer_wall_junctions_at_outer_bead_radius --nocapture 2>&1 \| tee target/test-output-a1-ac2.log` | AC-2: square outer-wall at outer-bead radius | FACT pass/fail; SNIPPETS ≤ 20 lines on failure |
| `cargo test -p slicer-core --features host-algos --test arachne_junction_upward_half_edge_only --nocapture 2>&1 \| tee target/test-output-a1-neg1.log` | AC-N1: upward-half-edge-only emission | FACT pass/fail |
| `cargo test -p slicer-core --features host-algos --test arachne_parity_red_perimeter_index --no-fail-fast 2>&1 \| tee target/test-output-a1-n2-still-red.log` | N2 stays red (A1 doesn't own it) | FACT fail (expected — confirms A1 didn't accidentally fix N2) |
| `cargo test -p slicer-core --features host-algos --test arachne_parity_red_is_odd_semantics --no-fail-fast 2>&1 \| tee target/test-output-a1-n4-still-red.log` | N4 stays red (A1 doesn't own it) | FACT fail (expected) |
| `cargo test -p slicer-core --features host-algos --test arachne_parity_red_transition_ends --no-fail-fast 2>&1 \| tee target/test-output-a1-n3-still-red.log` | N3 stays red (A1 doesn't own it) | FACT fail (expected) |
| `cargo test -p slicer-core --features host-algos --test centrality --test bead_count --test propagation 2>&1 \| tee target/test-output-a1-regression.log` | centrality/bead_count/propagation regression (fixtures re-baselined) | FACT pass/fail |
| `cargo test -p slicer-core --features host-algos --test generate_toolpaths 2>&1 \| tee target/test-output-a1-toolpaths.log` | generate_toolpaths regression (toolpaths_tapered_wedge re-baselined) | FACT pass/fail |
| `cargo run --bin pnp_cli --release -- slice --model resources/cube_4color.3mf --config resources/test_config/cube_4color-arachne.json --output /tmp/a1-cube4color.gcode 2>&1 \| tail -5` then `cargo test -p slicer-runtime --test executor -- cube_4color_arachne_outer_walls_close_end_to_end --nocapture 2>&1 \| tee target/test-output-a1-e2e.log` | e2e closure delta (record-only per cross-cutting policy; A1 records the failure count in its commit msg, does NOT block on green) | FACT pass/fail + summary line (record-only) |
| `rg -q 'D-116A-JUNCTION-BANDS' docs/DEVIATION_LOG.md` | Deviation log entry present | FACT pass/fail |
| `cargo check --workspace --all-targets` | Cross-crate compile | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Clippy gate | FACT pass/fail |
| `cargo xtask build-guests --check` | Guest WASM coherence (A1's change surface is `slicer-core`-internal; no guest feed expected, but the gate must run clean before blaming any guest-test failure) | FACT clean / STALE list |

All verification commands are delegation-friendly.

## Step Completion Expectations

Cross-step invariants the per-step blocks in `implementation-plan.md` cannot
express:

- **N7's `BeadingPropagation` side table is a prerequisite for N1's
  `generate_junctions` rewrite.** The implementer MUST land N7 first (Step 1)
  and green-gate it with a structural test (the side table is populated for
  rib-foot nodes, `getBeading` returns the correct `Beading` for known
  vertices) before touching `generate_junctions` (Step 2). N7 has no dedicated
  red test — the structural test is its only oracle before N1's red tests
  validate the combined system.
- **A1 must keep N2, N3, N4 red tests RED.** A1 owns only N1+N7; if any of N2
  (perimeter_index), N3 (transition ends), or N4 (is_odd) accidentally turns
  green during A1, the implementer has crossed scope into A2/B and must back
  out. The "stays red" verification commands above gate this.
- **A1 must NOT remove the π hack (`pipeline.rs:334`) or the 0.1× filter-dist
  fudge (`pipeline.rs:272-277`).** Those are Packet C's (`118`) scope, strictly
  after A2. A1's rewrite is gated on the centrality scheme the π hack sustains;
  removing it here would break A1's own green path.
- **A1 must NOT touch `arachne_pipeline.rs:122`
  (`arachne_pipeline_perimeter_index_is_sequential_per_line`).** That test
  asserts the divergent sequence-position semantics A2 fixes in-place; A1
  leaves it red.
- **Fixture re-baseline is atomic per fixture and records rationale.** Each
  re-baselined fixture gets its own commit message explaining what shifted and
  why (the `BeadingPropagation` side table changes bead widths/locations, so
  `toolpaths_tapered_wedge.json` and the centrality/propagation fixtures will
  drift). Never read the JSONs directly — re-record via the self-capture
  pattern.
- **Deviation-log correction uses the supersession pattern** — a new
  `D-116A-JUNCTION-BANDS` entry plus a one-line addendum on
  `D-113C-FAITHFUL-GRAPH-CONSTRUCTION`. Do not rewrite 113c's existing narrative
  text.

## Context Discipline Notes

Packet-specific context-budget hazards:

- `crates/slicer-core/src/arachne/generate_toolpaths.rs` (~953 LOC) is the
  primary edit target for Step 2 — can be full-read for that step only. Out of
  bounds for Step 1 (N7's side table lives in `skeletal_trapezoidation/`).
- `crates/slicer-core/src/skeletal_trapezoidation/propagation.rs` (~1107 LOC)
  is the primary edit target for Step 1's `upward_central_edges` /
  `propagate_beadings_downward` / `interpolate_bead_counts` changes — range-read
  `:120-160`, `:810-860`, `:980-1100`; do NOT full-read (the file's
  `apply_transitions` body at `:640-740` is Packet B's scope).
- `crates/slicer-core/src/beading/mod.rs` is read-only for A1 (the `Beading`
  struct shape); the 5 concrete strategies are out of bounds — A1 does not
  extend the trait (that's Packet B).
- Likely temptation reads to skip: `OrcaSlicerDocumented/` (delegate via the
  contract above), `modules/core-modules/arachne-perimeters/src/lib.rs` (A1's
  change surface is `slicer-core`-internal; the per-region call structure is
  unaffected), `crates/slicer-sdk/src/host.rs` (no WIT changes).
- Sub-agent return-format hints for the heaviest dispatches: the
  `generateJunctions` dispatch (`SkeletalTrapezoidation.cpp:2013-2079`) should
  request `SUMMARY` with explicit ask for the upward-skip / in-band-break /
  middle-index-start loop structure, NOT just a callee summary — the prior
  audit's core insight is that the caller-loop structure (which beads are
  skipped, where the loop breaks) is what PNP diverged on.