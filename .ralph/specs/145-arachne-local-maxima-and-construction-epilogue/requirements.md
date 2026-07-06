# Requirements: 145-arachne-local-maxima-and-construction-epilogue

## Packet Metadata

- Grouped task IDs: **none** (provenanced by the second-pass Arachne parity
  audit `target/arachne_parity_audit_20260706_020657.md` findings N9 and N10;
  no `docs/07` `TASK-###` exists for N1–N13).
- Backlog source: `docs/07_implementation_status.md` (no `TASK-###` for N1–N13).
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Two canonical passes are absent (N9 + N10). **N9 (`generateLocalMaximaSingleBeads`
absent):** `SkeletalTrapezoidation.cpp:2383-2413` is the final step of
`generateSegments`: for nodes with odd `beading.bead_widths.size()`,
`isLocalMaximum(true)`, and not central, it emits a 6-segment hexagonal
micro-loop (radius `width/8`, `is_odd = true`) so isolated thick spots get their
center dot. Without it, local maxima that never join a domain chain simply
vanish (pinholes at e.g. the center of near-square regions with odd bead
counts). `grep local_maxima` in PNP finds no hits — the pass is entirely
missing. **N10 (`constructFromPolygons` epilogue missing):** PNP's
`SkeletalTrapezoidationGraph::from_polygons` (`graph.rs:269-327`) ends after
per-edge radius bounds; none of the three canonical epilogue passes
(`SkeletalTrapezoidation.cpp:538-546`) exists: (1) `separatePointyQuadEndNodes`
duplicates shared boundary start-nodes so each quad traversal has a unique
start; (2) `graph.collapseSmallEdges()` removes degenerate zero-length edges
produced by integer rounding; (3) each node's `incident_edge` is reset to the
first `prev`-less edge. Consequences in PNP: zero-length spine fragments
survive into centrality/junction math (degenerate `edge_length` guards paper
over them: `centrality.rs:167`, `propagation.rs:1042-1044`), and pointy-corner
cells share quad-start nodes, which the `connectJunctions` walk then has to
survive by its defensive "already claimed" break (`generate_toolpaths.rs:699-705`)
instead of by construction. This packet ports both passes — N9 as the final
step of `generate_toolpaths`, N10 as the epilogue of `from_polygons`.

This packet extends `D-113C-FAITHFUL-GRAPH-CONSTRUCTION`'s `from_polygons` with
the canonical epilogue; 113c's per-cell graph construction (Steps 1-3) remains
canonical and untouched. D's epilogue is additive (three passes appended after
113c's existing per-edge radius bounds).

## In Scope

- **`generateLocalMaximaSingleBeads`** (NEW) in
  `crates/slicer-core/src/arachne/generate_toolpaths.rs`: the final step of
  `generate_toolpaths` (after `connectJunctions` emission). For nodes with odd
  `beading.bead_widths.size()`, `isLocalMaximum(true)`, and not central, emit
  a 6-segment hexagonal micro-loop (radius `width/8`, `is_odd = true`)
  mirroring `SkeletalTrapezoidation.cpp:2383-2413`. The micro-loop is a closed
  `ExtrusionLine` with `is_odd = true` (canonical N4 semantics — the centerline
  bead of an odd-bead-count region).
- **`separatePointyQuadEndNodes`** (NEW) in
  `crates/slicer-core/src/skeletal_trapezoidation/graph.rs`: duplicate shared
  boundary start-nodes so each quad traversal has a unique start, mirroring
  `SkeletalTrapezoidation.cpp:538-542` / `SkeletalTrapezoidationGraph.cpp`.
- **`collapseSmallEdges`** (NEW) in `graph.rs`: remove degenerate zero-length
  edges produced by integer rounding, mirroring `SkeletalTrapezoidation.cpp:543`
  / `SkeletalTrapezoidationGraph.cpp`. Edges with `edge_length < ε` (in slicer
  units; the canonical ε is a small constant — delegate for the exact value)
  are collapsed: their endpoints are merged, and incident edges repointed.
- **Incident-edge normalization** (NEW) in `graph.rs`: each node's
  `incident_edge` is reset to the first `prev`-less edge, mirroring
  `SkeletalTrapezoidation.cpp:545-546`.
- **Epilogue wiring** in `graph.rs::from_polygons`: append the three passes
  (`separatePointyQuadEndNodes` → `collapseSmallEdges` → incident-edge
  normalization) after 113c's existing per-edge radius bounds (`:269-327`).
- **`isLocalMaximum` predicate for N9's `generateLocalMaximaSingleBeads`
  gate**: a node is a local maximum if all its neighbors have
  `distance_to_boundary <=` its own. **Not a fresh symbol** — a private,
  currently-`#[allow(dead_code)]` function with matching semantics already
  exists at `crates/slicer-core/src/skeletal_trapezoidation/centrality.rs:264`
  (`fn is_local_maximum`, used only by the unwired `try_dissolve` whisker-
  dissolve helper the packet-113c/144 gotcha already forbids wiring up — see
  `docs/specs/arachne-parity-N1-N13-plan.md`'s "Gotchas" section). D's
  implementer MUST decide, before Step 1 begins, between:
  (a) reuse `centrality.rs`'s existing `is_local_maximum` directly (drop its
  `#[allow(dead_code)]`, keep it private, call it from `generate_toolpaths.rs`
  via a `pub(crate)` re-export or a thin wrapper), or
  (b) add a distinctly-named new predicate (e.g. `is_local_max_for_odd_bead`)
  in `graph.rs` if the two checks are not actually semantically identical
  (N9's gate needs `isLocalMaximum(true)` — the canonical bool argument's
  exact meaning must be confirmed via OrcaSlicer delegation before assuming
  reuse is safe).
  Adding a second, same-named `is_local_maximum` in the same module
  (`centrality.rs`) is a compile error; the decision must be made and recorded
  in `design.md` before implementation, not discovered mid-Step-1.
- **New tests**: `arachne_local_maxima_single_beads.rs` (AC-1 — near-square
  odd-bead-count region emits hexagonal micro-loop), `arachne_construction_epilogue.rs`
  (AC-2 — no zero-length edges, normalized incident edges, unique quad-start
  nodes).
- **Fixture re-baseline (this packet's own stage only)**:
  `crates/slicer-core/tests/fixtures/arachne/centrality_*.json`,
  `toolpaths_tapered_wedge.json` — re-record via self-capture (D's epilogue
  changes the graph shape, which ripples into centrality + toolpaths fixtures).
  Coordinate with prior packets' re-baselines via commit order.
- **Deviation-log entry**: `D-145-LOCAL-MAXIMA-EPILOGUE` (new ID, addendum on
  `D-113C-FAITHFUL-GRAPH-CONSTRUCTION`, supersession pattern — D extends 113c's
  `from_polygons`).

## Out of Scope

- **N1–N8** — Packets A1, A2, B, C. D reads their output but does not change
  them.
- **N11–N13** — Packets E (`146`), F (`147`).
- **`cube_4color.3mf` e2e closure gate** — record-only across D; Packet F blocks.
- **`cargo test --workspace`** — only at Packet F's closure ceremony.
- **New WIT/IR schema changes** — D's surface is `slicer-core`-internal; no
  WIT/IR change.
- **`OrcaSlicerDocumented/` C++ oracle build** — declined.

## Authoritative Docs

- `docs/02_ir_schemas.md` — §"Arachne extrusion-line geometry (Packet 112)"
  (lines ~1091-1150); `ExtrusionLine::is_odd` (D's micro-loops are `is_odd =
  true`).
- `docs/08_coordinate_system.md` — §"Constant Conversion Table" (~30 lines);
  `width/8` radius conversion.
- `docs/DEVIATION_LOG.md` `D-113C-FAITHFUL-GRAPH-CONSTRUCTION` entry — read
  full; D's epilogue extends 113c's `from_polygons`.
- `docs/specs/arachne-parity-N1-N13-plan.md` — read full; cross-packet policies.
- `.ralph/specs/113c-arachne-faithful-graph-construction/requirements.md`
  §"OrcaSlicer Reference Obligations" (the `orca-delegation` snippet) — D
  carries this contract forward verbatim.

All other docs are not authoritative for this packet.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:2383-2413` — `generateLocalMaximaSingleBeads` (6-segment hexagonal micro-loop, radius `width/8`, `is_odd = true`, for odd-bead-count local maxima that are `isLocalMaximum(true)` and not central).
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:538-546` — `constructFromPolygons` epilogue: `separatePointyQuadEndNodes`, `collapseSmallEdges`, incident-edge normalization.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidationGraph.cpp` — `collapseSmallEdges` + `separatePointyQuadEndNodes` implementations (delegate for exact signatures + the zero-length ε constant).

## Acceptance Summary

Reference Acceptance Criteria by ID; do not copy them.

- Positive cases: `AC-1` (hexagonal micro-loop at local maximum), `AC-2` (no
  zero-length edges, normalized incident edges, unique quad-start nodes) from
  `packet.spec.md`.
- Negative cases: `AC-N1` (N1 red tests stay green — epilogue doesn't regress
  A1).
- Cross-packet impact: unblocks `146` (E — E's `removeSmallLines` interacts
  with D's micro-loops, which are `is_odd = true` closed lines).
- Refinements not captured in Given/When/Then:
  - D's micro-loops are `is_odd = true` closed `ExtrusionLine`s (canonical N4
    semantics — the centerline bead of an odd-bead-count region). This is
    consistent with A2's `is_odd` fix (A2 owns the per-segment `is_odd` rule
    for the main walls; D's micro-loops are a separate emission with
    `is_odd = true` by construction).
  - `collapseSmallEdges`'s zero-length ε is a small constant in slicer units;
    the implementer confirms the exact value via a delegated SUMMARY of
    `SkeletalTrapezoidationGraph.cpp`'s `collapseSmallEdges`.
  - D's epilogue is additive — three passes appended after 113c's existing
    per-edge radius bounds. 113c's `from_polygons` Steps 1-3 remain
    canonical.
  - D's `isLocalMaximum` predicate is a new graph predicate; the implementer
    decides its location (`centrality.rs` alongside `updateIsCentral`, or
    `graph.rs` as a standalone).

## Verification Commands

Full verification matrix. `packet.spec.md` §Verification carries only the gate
subset.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-core --features host-algos --test arachne_local_maxima_single_beads --nocapture 2>&1 \| tee target/test-output-d-ac1.log` | AC-1: hexagonal micro-loop | FACT pass/fail; SNIPPETS ≤ 20 lines on failure |
| `cargo test -p slicer-core --features host-algos --test arachne_construction_epilogue --nocapture 2>&1 \| tee target/test-output-d-ac2.log` | AC-2: epilogue (no zero-length, normalized incident, unique quad-start) | FACT pass/fail |
| `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --test arachne_parity_red_perimeter_index --test arachne_parity_red_is_odd_semantics --test arachne_parity_red_transition_ends --no-fail-fast 2>&1 \| tee target/test-output-d-stays-green.log` | N1/N2/N4/N3 stay green (D doesn't regress A1/A2/B) | FACT pass (expected) |
| `cargo test -p slicer-core --features host-algos --test centrality --test bead_count --test propagation --test generate_toolpaths 2>&1 \| tee target/test-output-d-regression.log` | regression (fixtures re-baselined) | FACT pass/fail |
| `cargo run --bin pnp_cli --release -- slice --model resources/cube_4color.3mf --config resources/test_config/cube_4color-arachne.json --output /tmp/d-cube4color.gcode 2>&1 \| tail -5` then `cargo test -p slicer-runtime --test executor -- cube_4color_arachne_outer_walls_close_end_to_end --nocapture 2>&1 \| tee target/test-output-d-e2e.log` | e2e closure delta (record-only per cross-cutting policy; D records the failure count in its commit msg, does NOT block on green) | FACT pass/fail + summary line (record-only) |
| `rg -q 'D-145-LOCAL-MAXIMA-EPILOGUE' docs/DEVIATION_LOG.md` | Deviation log entry present | FACT pass/fail |
| `cargo check --workspace --all-targets` | Cross-crate compile | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Clippy gate | FACT pass/fail |
| `cargo xtask build-guests --check` | Guest WASM coherence (D's surface is `slicer-core`-internal; no guest feed) | FACT clean / STALE list |

All verification commands are delegation-friendly.

## Step Completion Expectations

Cross-step invariants the per-step blocks in `implementation-plan.md` cannot
express:

- **D must keep N1, N2, N3, N4 red tests GREEN.** D's epilogue changes the
  graph shape; regressing A1/A2/B means backing out.
- **D's micro-loops are `is_odd = true` closed `ExtrusionLine`s** (canonical N4
  semantics). This is consistent with A2's `is_odd` fix; D's micro-loops are a
  separate emission with `is_odd = true` by construction, not a per-segment
  computation.
- **`collapseSmallEdges`'s zero-length ε is a small constant in slicer units.**
  The implementer confirms the exact value via a delegated SUMMARY.
- **D's epilogue is additive** — three passes appended after 113c's existing
  per-edge radius bounds. 113c's `from_polygons` Steps 1-3 remain canonical.
- **Fixture re-baseline is atomic per fixture and records rationale.**
  `centrality_*.json` + `toolpaths_tapered_wedge.json` drift because D's
  epilogue changes the graph shape. Coordinate with prior packets' re-baselines
  via commit order.
- **Deviation-log correction uses the supersession pattern** — new
  `D-145-LOCAL-MAXIMA-EPILOGUE` + addendum on `D-113C-FAITHFUL-GRAPH-CONSTRUCTION`.

## Context Discipline Notes

Packet-specific context-budget hazards:

- `crates/slicer-core/src/skeletal_trapezoidation/graph.rs` (~700 LOC per 113c)
  is the primary edit target for Step 2 — range-read `:269-327` (the current
  `from_polygons` end) + the `STHalfEdge`/`STVertex` struct defs; do NOT
  full-read (113c's per-cell construction is canonical, not D's scope to
  re-derive).
- `crates/slicer-core/src/arachne/generate_toolpaths.rs` (~953 LOC) is the
  primary edit target for Step 1 — range-read the end of `generate_toolpaths`
  (where `generateLocalMaximaSingleBeads` is appended as the final step).
- `crates/slicer-core/src/skeletal_trapezoidation/centrality.rs` — read-only
  for D (the `isLocalMaximum` predicate may live here alongside
  `updateIsCentral`, or in `graph.rs`); range-read `:100-200` for the predicate
  convention.
- Likely temptation reads to skip: `OrcaSlicerDocumented/` (delegate),
  `modules/core-modules/arachne-perimeters/` (D's surface is `slicer-core`-
  internal), `slicer-sdk`/`slicer-wasm-host` (no WIT change).
- Sub-agent return-format hints for the heaviest dispatches: the
  `generateLocalMaximaSingleBeads` SUMMARY (`SkeletalTrapezoidation.cpp:2383-2413`)
  should request the hexagonal micro-loop geometry (6 segments, radius
  `width/8`, `is_odd = true`) + the `isLocalMaximum(true)` + not-central +
  odd-bead-count conditions explicitly. The `collapseSmallEdges` SUMMARY
  (`SkeletalTrapezoidationGraph.cpp`) should request the zero-length ε
  constant + the endpoint-merge rule.