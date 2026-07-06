# Arachne Parity N1–N13 — Packet Decomposition Plan

**Date:** 2026-07-06
**Branch:** `parity/arachne`
**Red suite pinned at:** `b2ea52b7` (`test(arachne): add red tests for second-pass parity audit N1-N4`)
**Authoritative audit:** `target/arachne_parity_audit_20260706_020657.md` (findings N1–N13)
**Prior packet:** `.ralph/specs/113c-arachne-faithful-graph-construction/` (status `implemented`) — its `requirements.md` `orca-delegation` snippet is carried verbatim into every new packet.

## State of the world

Known-good (do not re-port): graph construction, `insert_node` twin-splitting, `apply_transitions` own-bucket/ascending mechanics, `updateIsCentral` predicate shape, 9-stage input preprocessing, quad-walk topology.
Broken: junction generation through emission (N1/N2), transition end/ramp machinery (N3), `is_odd` semantics (N4), plus major/minor gaps N5–N13.

## Revised packet decomposition (grilling-validated)

Dependency graph:

```
A1 (N7+N1) -> A2 (N2+N4) -> B (N3+N8) / C (N5+N6) [either order] -> D (N9+N10) -> E (N11-N13) -> F (closure)
```

Linear A1 -> A2 -> rest (B/C strictly after A2; no parallelism with A2).

### Packet A1 — BeadingPropagation + canonical generateJunctions (L, gating)

- **Findings:** N7 + N1
- **Prereqs:** none
- **Acceptance oracle:** 2 N1 red tests (`arachne_parity_red_junction_bands.rs`)
- **Surface:**
  - `BeadingPropagation` side table (full `Beading` + source distances per node) — N7.
  - `getBeading`-equivalent (propagation/nearest lookup, 0.1mm `getNearestBeading` radius) so rib-foot nodes (no `bead_count`) resolve a `Beading`.
  - Canonical `generateJunctions` rewrite (`generate_toolpaths.rs:192-334`): iterate ALL edges (no centrality gate, ribs included), skip non-upward half-edges (`from.R > to.R`), skip flat/same-bead-count edges, single beading at the peak node via `getBeading`, in-band beads only (middle-index start, break on `bead_R < end_R`), no clamping.
  - `upward_central_edges` (`propagation.rs:126`) loses the centrality filter (N7).
  - `propagate_beadings_downward` must skip central edges and interpolate bead widths/locations (not rounded integer counts) — N7.
- **Scope note:** `arachne_pipeline.rs:122` (`arachne_pipeline_perimeter_index_is_sequential_per_line`) asserts the divergent sequence-position semantics — A1 does NOT touch it (A2 owns the `perimeter_index` fix); leave it red until A2.
- **Carries forward:** `orca-delegation` snippet verbatim from 113c `requirements.md`.

### Packet A2 — Canonical connectJunctions emission + is_odd (L)

- **Findings:** N2 + N4
- **Prereqs:** A1 (strict — A2's pop-back merge needs A1's correct junction fans)
- **Acceptance oracle:** 1 N2 red test + 2 N4 red tests
- **Surface:**
  - `perimeter_index = bead_idx` at junction generation (`generate_toolpaths.rs:315,326`).
  - Per-quad `connectJunctions` emission: `from_junctions`/`to_junctions` pairing, secondary-fan `perimeter_index` pop-back dedup (`SkeletalTrapezoidation.cpp:2302-2314`), `addToolpathSegment`-style line growth (extend last `ExtrusionLine` if within 10µm, else new line), `new_domain_start` fresh-line flag.
  - Canonical `is_odd` per segment (`SkeletalTrapezoidation.cpp:2344-2354`): `bead_count % 2 == 1`, `transition_ratio == 0`, innermost junction, endpoint proximity (0.005mm) to peak node.
  - `passed_odd_edges` keyed on the physical edge (not `(bead, edge, twin)` triple).
  - Delete `assign_perimeter_indices` (`pipeline.rs:384-390`) — becomes dead.
  - **Update in place** `arachne_pipeline.rs:122` to assert `perimeter_index == line.inset_idx` (the N2 contract); same test name, new assertion, explicit in commit. [User decision: update in place.]
- **Scope note (NOT a silent absorb):** `ExtrusionJunction::perimeter_index` is `u32` at `slicer-ir::slice_ir.rs:1744,1798`, forwarded verbatim at `slicer-sdk/src/host.rs:717` and `slicer-wasm-host/src/host.rs:1814`. The semantic change (bead index vs sequence position) is wire-type-transparent — NO schema change. The only in-tree consumer of the old sequence semantics is the test at `arachne_pipeline.rs:122` (updated in this packet). Surface this as a scope decision in A2's `requirements.md`.

### Packet B — Transition ends + extra ribs (L)

- **Findings:** N3 + N8
- **Prereqs:** A2
- **Acceptance oracle:** 2 N3 red tests (`arachne_parity_red_transition_ends.rs`)
- **Surface:**
  - `BeadingStrategy` trait extension: `get_transitioning_length`, `get_transition_anchor_pos`, `get_nonlinear_thicknesses`. [Confirmed codebase-ready: `DistributedBeadingStrategy` already stores `default_transition_length` (unused, `distributed.rs:43`); the 4 decorators delegate to `self.parent`. `wall_transition_angle` already exists (`mod.rs:93`) — disambiguate during this packet's grilling, do NOT add a duplicate.]
  - **New pipeline stage** `generate_all_transition_ends` (red test *call sites* updated to invoke it before `apply_transitions`, assertions untouched per user decision).
  - `filterTransitionMids` (`SkeletalTrapezoidation.cpp:1007-1076`): recursive dissolve of nearby same-`lower_bead_count` transitions within `transition_filter_dist`.
  - `generateAllTransitionEnds` (`:1247-1403`): each mid spawns lower end (backward on `edge.twin`) + upper end (forward), spread over `getTransitioningLength(lower_bead_count)` around `getTransitionAnchorPos`; ends recursively travel onto successor edges assigning fractional `transition_ratio`.
  - `applyTransitions` at ends (`:1487-1543`): insert nodes at END positions with `bead_count = lower` or `lower + 1` per `is_lower_end`.
  - `generateExtraRibs` (`:1579-1633`): for upward central edges ≥ `discretization_step_size`, insert rib nodes at every `getNonlinearThicknesses()` radius.
  - Beading interpolation at emission (`generateSegments :1712-1721`): interpolate `compute(thickness, bead_count)` and `compute(thickness, bead_count + 1)` for nonzero `transition_ratio`.
  - `EdgeType::TRANSITION_END` is a PNP invention, currently unused — decide repurpose vs delete.
- **Audit obligation:** `crates/slicer-core/src/beading/` was OUT of the audit's read scope — packet B's author must audit the beading stack's readiness for these APIs before implementation.

### Packet C — Angle/filter fudge removal + filterNoncentralRegions (M)

- **Findings:** N5 + N6
- **Prereqs:** A2 (strict — the π hack is load-bearing for A's centrality-gated scheme, removed only after A2)
- **Acceptance oracle:** N1 red tests still pass with configured angle; N6 suggested dumbbell test (post-fix, needs the rewrite to be observable)
- **Surface:**
  - Delete π workaround (`pipeline.rs:334`) and 0.1× filter-dist fudge (`pipeline.rs:272-277`).
  - Thread configured `wall_transition_angle` through `filter_central`.
  - Port `filterNoncentralRegions` (`SkeletalTrapezoidation.cpp:811-862`): promote non-central gaps between same/±1-bead-count central regions (within hardcoded 0.4mm) back to central; copy bead counts across.
- **Gotcha:** Orca's top-level `filterCentral` whisker-dissolve is DEAD CODE upstream (`:716-730`, self-contradictory) — do NOT wire the dissolve in.

### Packet D — Local maxima + construction epilogue (M)

- **Findings:** N9 + N10
- **Prereqs:** A2, B, C
- **Acceptance oracle:** N6 dumbbell test + existing suite
- **Surface:**
  - `generateLocalMaximaSingleBeads` (`SkeletalTrapezoidation.cpp:2383-2413`): odd bead count, `isLocalMaximum(true)`, not central — emit 6-segment hexagonal micro-loop (radius `width/8`, `is_odd = true`).
  - Construction epilogue (`SkeletalTrapezoidation.cpp:538-546`): `separatePointyQuadEndNodes`, `collapseSmallEdges`, incident-edge normalization (reset each node's `incident_edge` to first `prev`-less edge).

### Packet E — Post-process order + remove_small + simplify (S)

- **Findings:** N11 + N12 + N13
- **Prereqs:** D
- **Acceptance oracle:** existing suite
- **Surface:**
  - Post-process order (`WallToolPaths.cpp:679-699`): `stitch -> removeSmallLines -> separateOutInnerContour -> simplifyToolPaths -> removeEmptyToolPaths` (swap `simplify`/`remove_small` order; add `separateOutInnerContour` + `removeEmptyToolPaths`).
  - Per-line `min_width` in `remove_small_lines` (`WallToolPaths.cpp:838-856`): `min_width` = minimum junction width over the line; divisor `min_width/2` on top/bottom layers, `min_width * min_length_factor` otherwise.
  - Simplify distance gates (`ExtrusionLine.cpp:56-243`): `smallest_line_segment_squared` / `allowed_error_distance_squared` (from `meshfix_maximum_resolution`/`_deviation`) with `calculateExtrusionAreaDeviationError` as extra guard on near-colinear fast path only (not the iterative multi-pass sweep PNP currently does).

### Packet F — Cross-cutting closure (M)

- **Findings:** cross-cutting (no new finding fixes)
- **Prereqs:** A1, A2, B, C, D, E (all)
- **Acceptance oracle:** `cube_4color_arachne_outer_walls_close_end_to_end` green + `cargo xtask test --workspace --summary` green
- **Surface:**
  - All fixture batch re-baselines (stragglers after per-packet re-baselines — see policy below).
  - `D-11X-*` deviation-log supersession entries (addendum pattern per 113c: new ID + addendum, NO in-place edits to `D-112-MMU-TOPOLOGY`/`D-113B-CONNECTJUNCTIONS`).
  - ADR `0035-arachne-faithful-emission-and-transitions.md` (next free number after 0034).
  - `cube_4color_arachne_outer_walls_close_end_to_end` re-greened as permanent test.
- **e2e gate policy [user decision]:** Record-only across A1–E; block in F. Each packet runs the e2e test and records the failure delta in its commit message; F is the packet that blocks on green.

## Cross-cutting policies (decided during grilling)

- **Fixture re-baseline:** Distributed per-packet (each of A1–E re-baselines ONLY the fixtures its own stage touches, in its own commit, with rationale in the commit message); F closes stragglers. Two mechanisms in tree: `slicer-core` self-capture (first-run writes, subsequent compare — no `record_*` fns); `slicer-runtime/perimeter_parity` `#[ignore]`-marked `record_*` functions (11 of them). Never read the big JSONs directly; re-record via `record_*`.
- **e2e closure gate:** Record-only across A1–E (each packet records failure delta in commit msg); block in F.
- **`arachne_pipeline_perimeter_index_is_sequential_per_line`** (`arachne_pipeline.rs:122`): Update in place to bead-index semantics in Packet A2.

## Constraints to encode in every packet (repo law)

- OrcaSlicer reads ONLY via sub-agent delegation (`orca-delegation` snippet from 113c `requirements.md`, carried verbatim).
- 1 unit = 100 nm (`UNITS_PER_MM = 10_000`); divide OrcaSlicer constants by 100.
- Attribution header (`docs/ORCASLICER_ATTRIBUTION.md`) on any newly ported file.
- Test discipline: narrow `cargo test -p slicer-core --features host-algos --test <file>`; tee to `target/test-output.log`; `cargo test --workspace` only at packet-close via `cargo xtask test --workspace --summary`; `cargo clippy --workspace --all-targets -- -D warnings` gate; `cargo xtask build-guests --check` before blaming guest-test failures.
- No WIT/IR schema changes (all `slicer-core` internal). `perimeter_index` semantic change is wire-type-transparent — surface in A2 as scope decision.

## Gotchas (carry forward from audit)

- Orca's `filterCentral` whisker-dissolve is DEAD CODE upstream (`:716-730`, self-contradictory) — do not wire in.
- `EdgeType::TRANSITION_END` is a PNP invention, currently unused — Packet B decides repurpose vs delete.
- Square fixture does NOT show net 2× duplication downstream (merging masks it); observable is wrong outer-wall radius (~0.5mm vs 0.2mm) — don't build acceptance on total-length duplication.
- Use `--no-fail-fast` for the red suite (multi-target `cargo test` aborts at first failing binary).

## Suggested skill sequence (next session)

1. `spec-packet-generator` — one invocation per packet (A1, A2, B, C, D, E, F), feeding the finding sections + red-test paths + this plan.
2. `spec-review` — review each packet against its docs before flipping to `active`.
3. `swarm` — implementation sessions (planner/worker with context budget).

## Pre-work completed this session

- Red suite committed at `b2ea52b7` (7 tests, 4 files, all confirmed FAIL via `cargo test --no-fail-fast`, log `target/test-output.log`).