# Requirements: 143-arachne-transition-ends-and-extra-ribs

## Packet Metadata

- Grouped task IDs: **none** (provenanced by the second-pass Arachne parity
  audit findings N3 + N8, encoded as committed red tests at `b2ea52b7`).
- Backlog source: `docs/07_implementation_status.md` (no `TASK-###` for N1–N13).
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

PNP's `apply_transitions` (`propagation.rs:646-740`) converts every
`TransitionMiddle` directly into a single `insert_node` split at the MID
position, with `transition_ratio` hard-set to `0.0` everywhere (`:714`, `:723`).
There is no `filterTransitionMids`, no `generateTransitionEnds`/`generateAllTransitionEnds`,
and `generate_junctions` calls `strategy.compute(2R, bead_count)` directly with
an integer bead count — no interpolation. Canonical
(`SkeletalTrapezoidation.cpp:881-915` `generateTransitioningRibs`) instead runs
`generateTransitionMids` → `filterTransitionMids` (`:1007-1076`) →
`generateAllTransitionEnds` (`:1247-1403`) → `applyTransitions` (`:1487-1543`):
each mid spawns a lower end walking backward on `edge.twin` and an upper end
walking forward, spread over `beading_strategy.getTransitioningLength(lower_bead_count)`
around the anchor `getTransitionAnchorPos`; ends recursively travel onto
successor edges, assigning every traversed node a fractional `transition_ratio`;
`applyTransitions` inserts nodes at END positions with `bead_count = lower` or
`lower + 1` per `is_lower_end` (`:1525-1526`); `generateSegments` (`:1712-1721`)
interpolates the beading of any node with nonzero `transition_ratio` between
`compute(thickness, bead_count)` and `compute(thickness, bead_count + 1)`. Net
effect of the gap: PNP snaps the bead count at a single point (abrupt width step
at every transition — visible bumps), and keeps every raw transition mid (extra
churn on noisy geometry). Separately, `generateExtraRibs` (`:1579-1633`) is
absent — long spine edges get no intermediate width-sampling points at
nonlinear-strategy breakpoints, so widths along long spine edges are linearly
interpolated across nonlinear-strategy breakpoints (visible width error on
wide regions). The `BeadingStrategy` trait (`beading/mod.rs:64-108`) lacks
`getTransitioningLength` / `getTransitionAnchorPos` / `getNonlinearThicknesses`
entirely — N3 (and N8) require a trait extension. (Note: `wall_transition_angle`
already exists on the trait at `mod.rs:93`; B must not add a duplicate.)

This packet supersedes `D-112-PROPAGATION-ADAPT` for the transition machinery;
A1/A2's junction generation and emission remain canonical and untouched.

## In Scope

- **`BeadingStrategy` trait extension** in `crates/slicer-core/src/beading/mod.rs`:
  add `get_transitioning_length` / `get_transition_anchor_pos` /
  `get_nonlinear_thicknesses` with default implementations that delegate to
  `self.parent` for the 4 decorators (`widening.rs`, `redistribute.rs`,
  `outer_wall_inset.rs`, `limited.rs`); `DistributedBeadingStrategy`
  (`distributed.rs`) returns its stored `default_transition_length` (line 43,
  currently `#[allow(dead_code)]`) for `get_transitioning_length`, and
  canonical defaults for the other two. **Do NOT add a duplicate
  `wall_transition_angle`** — it already exists at `mod.rs:93`.
- **`generate_all_transition_ends` pipeline stage** (NEW) in
  `crates/slicer-core/src/skeletal_trapezoidation/propagation.rs`: the
  `generateTransitionMids` → `filterTransitionMids` → `generateAllTransitionEnds`
  → `applyTransitions` sequence. `filterTransitionMids` (`:1007-1076`):
  recursive dissolve of nearby same-`lower_bead_count` transitions within
  `transition_filter_dist`. `generateAllTransitionEnds` (`:1247-1403`): each
  mid spawns a lower end (backward on `edge.twin`) + upper end (forward),
  spread over `get_transitioning_length(lower_bead_count)` around
  `get_transition_anchor_pos`; ends recursively travel onto successor edges
  assigning fractional `transition_ratio`. `applyTransitions` at ends
  (`:1487-1543`): insert nodes at END positions with `bead_count = lower` or
  `lower + 1` per `is_lower_end`.
- **`apply_transitions` rewrite** in `propagation.rs:646-740`: consume
  `TransitionEnd`s (not `TransitionMiddle`s) and insert at END positions with
  `bead_count = lower` or `lower + 1` per `is_lower_end`; write fractional
  `transition_ratio` on traversed nodes (not `0.0`).
- **`generateExtraRibs`** (NEW) in `propagation.rs`: for upward central edges
  ≥ `discretization_step_size`, insert rib nodes at every radius in
  `get_nonlinear_thicknesses()` between the endpoints' radii (via the existing
  `insert_node` machinery). Called after `applyTransitions` at `:645`.
- **Beading interpolation at emission** in
  `crates/slicer-core/src/arachne/generate_toolpaths.rs`: for any node with
  nonzero `transition_ratio`, interpolate the beading between
  `compute(thickness, bead_count)` and `compute(thickness, bead_count + 1)` per
  `generateSegments` (`SkeletalTrapezoidation.cpp:1712-1721`). This reads A2's
  canonical junction fans.
- **`EdgeType::TRANSITION_END`**: a PNP invention, currently unused. B decides
  repurpose (if the new `TransitionEnd` type needs an edge marker) vs delete.
  The audit flags it as unused; prefer delete unless the rewrite needs it.
- **N3 red-test call-site update** (assertions untouched per grilling decision):
  `crates/slicer-core/tests/arachne_parity_red_transition_ends.rs` currently
  calls `apply_transitions(&mut graph)` directly. B updates the call sites to
  invoke `generate_all_transition_ends` before `apply_transitions` (or the new
  combined entry point). The assertions (lower+upper end splits, fractional
  ratio) are NOT weakened — only the call sites gain the preceding stage call.
- **`pipeline.rs` stage wiring**: insert `generate_all_transition_ends` into
  `run_arachne_pipeline` between `generate_transition_mids` and
  `apply_transitions` (currently `:345-346`).
- **Beading-stack audit** (handoff-flagged): `crates/slicer-core/src/beading/`
  was OUT of the audit's read scope. B's author must audit the beading stack's
  readiness for the 3 new trait methods before implementation — confirm
  `DistributedBeadingStrategy`'s `default_transition_length` is the right
  value for `get_transitioning_length`, and that the 4 decorators' delegation
  is correct.
- **Fixture re-baseline (this packet's own stage only)**:
  `crates/slicer-core/tests/fixtures/arachne/propagation_*.json` — re-record
  via self-capture (B changes transition splitting + adds ends).
- **Deviation-log entry**: `D-143-TRANSITION-ENDS` (new ID, addendum on
  `D-112-PROPAGATION-ADAPT`, supersession pattern).

## Out of Scope

- **N1/N7 (junction geometry + BeadingPropagation)** — A1 (`141`). B reads
  A1's `get_beding` for emission interpolation but does not change it.
- **N2/N4 (`perimeter_index` + `is_odd`)** — A2 (`142`). B reads A2's junction
  fans for beading interpolation but does not change them.
- **N5 (π hack) and N6 (`filterNoncentralRegions`)** — Packet C (`144`).
- **N9–N13** — Packets D, E, F.
- **`cube_4color.3mf` e2e closure gate** — record-only across B; Packet F blocks.
- **`cargo test --workspace`** — only at Packet F's closure ceremony.
- **New WIT/IR schema changes** — B's `BeadingStrategy` trait extension is
  `slicer-core`-internal; no WIT/IR change. The trait is not exposed across the
  host boundary.
- **`OrcaSlicerDocumented/` C++ oracle build** — declined.

## Authoritative Docs

- `docs/08_coordinate_system.md` — §"Constant Conversion Table" (~30 lines);
  0.4 mm = 4000 units, 0.1 mm = 1000 units.
- `docs/15_config_keys_reference.md` — §"Arachne beading strategy stack" (lines
  ~479-521); `wall_transition_length`, `wall_transition_filter_deviation`.
- `docs/DEVIATION_LOG.md` `D-112-PROPAGATION-ADAPT` + `D-141-JUNCTION-BANDS` +
  `D-142-CONNECTJUNCTIONS-EMISSION` entries — substrate.
- `docs/specs/arachne-parity-N1-N13-plan.md` — cross-packet policies.
- `.ralph/specs/113c-arachne-faithful-graph-construction/requirements.md`
  §"OrcaSlicer Reference Obligations" (the `orca-delegation` snippet) — B
  carries this contract forward verbatim.

All other docs are not authoritative for this packet.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:881-915` — `generateTransitioningRibs`.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:1007-1076` — `filterTransitionMids`.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:1247-1403` — `generateAllTransitionEnds`.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:1487-1543` — `applyTransitions` at ends.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:1579-1633` — `generateExtraRibs`.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:1712-1721` — `generateSegments` beading interpolation.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/BeadingStrategy/BeadingStrategy.h` — trait surface for the 3 new methods.

## Acceptance Summary

Reference Acceptance Criteria by ID; do not copy them.

- Positive cases: `AC-1` (lower+upper end splits, not single-mid-split), `AC-2`
  (fractional `transition_ratio` on spillover vertex) from `packet.spec.md`.
  Both are red tests committed at `b2ea52b7` and currently FAIL; B is done when
  they pass **without weakened assertions**.
- Negative cases: `AC-N1` (trait extension compiles without caller-side
  breakage — the 5 concrete strategies absorb the 3 new methods via defaults).
- Cross-packet impact: unblocks `144` (C — `filterNoncentralRegions` interacts
  with B's transition regions).
- Refinements not captured in Given/When/Then:
  - N3 red-test call sites are updated (assertions untouched per grilling
    decision) — `apply_transitions(&mut graph)` becomes
    `generate_all_transition_ends(&mut graph, &strategy); apply_transitions(&mut graph);`
    (or a combined entry point).
  - `wall_transition_angle` already exists on the trait (`mod.rs:93`); B does
    NOT add a duplicate — disambiguate during B's grilling.
  - `EdgeType::TRANSITION_END` is a PNP invention, currently unused; B prefers
    delete unless the rewrite needs an edge marker.
  - `DistributedBeadingStrategy`'s `default_transition_length` (line 43,
    currently `#[allow(dead_code)]`) becomes the live value for
    `get_transitioning_length` — the `#[allow(dead_code)]` is removed.
  - B's beading interpolation at emission reads A2's canonical junction fans;
    B does NOT change A2's `connectJunctions` emission.

## Verification Commands

Full verification matrix. `packet.spec.md` §Verification carries only the gate
subset.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-core --features host-algos --test arachne_parity_red_transition_ends -- n3_apply_transitions_creates_lower_and_upper_end_splits --nocapture 2>&1 \| tee target/test-output-b-ac1.log` | AC-1: lower+upper end splits | FACT pass/fail; SNIPPETS ≤ 20 lines on failure |
| `cargo test -p slicer-core --features host-algos --test arachne_parity_red_transition_ends -- n3_transition_spilling_past_vertex_sets_fractional_ratio --nocapture 2>&1 \| tee target/test-output-b-ac2.log` | AC-2: fractional ratio on spillover | FACT pass/fail |
| `cargo check -p slicer-core --all-targets 2>&1 \| tee target/test-output-b-neg1.log` | AC-N1: trait extension compiles | FACT pass/fail |
| `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --test arachne_parity_red_perimeter_index --test arachne_parity_red_is_odd_semantics --no-fail-fast 2>&1 \| tee target/test-output-b-stays-green.log` | N1/N2/N4 stay green (B doesn't regress A1/A2) | FACT pass (expected) |
| `cargo test -p slicer-core --features host-algos --test propagation 2>&1 \| tee target/test-output-b-regression.log` | propagation regression (fixtures re-baselined) | FACT pass/fail |
| `cargo run --bin pnp_cli --release -- slice --model resources/cube_4color.3mf --config resources/test_config/cube_4color-arachne.json --output /tmp/b-cube4color.gcode 2>&1 \| tail -5` then `cargo test -p slicer-runtime --test executor -- cube_4color_arachne_outer_walls_close_end_to_end --nocapture 2>&1 \| tee target/test-output-b-e2e.log` | e2e closure delta (record-only) | FACT + summary line |
| `rg -q 'D-143-TRANSITION-ENDS' docs/DEVIATION_LOG.md` | Deviation log entry present | FACT pass/fail |
| `cargo check --workspace --all-targets` | Cross-crate compile | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Clippy gate | FACT pass/fail |
| `cargo xtask build-guests --check` | Guest WASM coherence (B's surface is `slicer-core`-internal; no guest feed) | FACT clean / STALE list |

All verification commands are delegation-friendly.

## Step Completion Expectations

Cross-step invariants the per-step blocks in `implementation-plan.md` cannot
express:

- **The trait extension (Step 1) must compile before the pipeline stage (Step 2)
  uses it.** `get_transitioning_length` / `get_transition_anchor_pos` /
  `get_nonlinear_thicknesses` must exist on the trait with default
  implementations before `generate_all_transition_ends` calls them.
- **B must keep N1, N2, N4 red tests GREEN.** B builds on A1/A2; regressing them
  means backing out.
- **B must NOT remove the π hack (`pipeline.rs:334`) or the 0.1× filter-dist
  fudge (`pipeline.rs:272-277`).** Those are Packet C's scope.
- **N3 red-test call sites are updated (assertions untouched).** The call
  `apply_transitions(&mut graph)` becomes
  `generate_all_transition_ends(&mut graph, &strategy); apply_transitions(&mut graph);`
  (or a combined entry point). The assertions (lower+upper end splits,
  fractional ratio) are NOT weakened.
- **`wall_transition_angle` already exists** — B does NOT add a duplicate.
- **`EdgeType::TRANSITION_END` is deleted unless the rewrite needs it.**
- **Beading-stack audit is mandatory** — `crates/slicer-core/src/beading/` was
  out of the audit's read scope; B's author must confirm the 5 concrete
  strategies' readiness for the 3 new methods before implementation.
- **Fixture re-baseline is atomic per fixture and records rationale.**
- **Deviation-log correction uses the supersession pattern** — new
  `D-143-TRANSITION-ENDS` + addendum on `D-112-PROPAGATION-ADAPT`.

## Context Discipline Notes

Packet-specific context-budget hazards:

- `crates/slicer-core/src/skeletal_trapezoidation/propagation.rs` (~1107 LOC)
  is the primary edit target for Step 2 — range-read `:640-740`
  (`apply_transitions`) + the new `generate_all_transition_ends`/`filter_transition_mids`/`generate_extra_ribs` insertions. Do NOT full-read
  (the file's `upward_central_edges`/`propagate_beadings_downward` at
  `:120-160`/`:980-1100` are A1's scope).
- `crates/slicer-core/src/beading/mod.rs` (108 LOC) + the 5 concrete strategies
  (`distributed.rs` ~198, `widening.rs`, `redistribute.rs`, `outer_wall_inset.rs`,
  `limited.rs`) are the primary edit targets for Step 1 — full-read `mod.rs`
  and `distributed.rs`; range-read the 4 decorators' `impl BeadingStrategy`
  blocks only (the delegation pattern is `self.parent.method()`, confirmed
  during grilling).
- `crates/slicer-core/src/arachne/generate_toolpaths.rs` — range-read the
  emission interpolation site only (B's change is the interpolation for nonzero
  `transition_ratio`, not the emission rewrite — that's A2).
- Likely temptation reads to skip: `OrcaSlicerDocumented/` (delegate),
  `modules/core-modules/arachne-perimeters/` (B's surface is `slicer-core`-
  internal), `slicer-sdk`/`slicer-wasm-host` (no WIT change — the trait is not
  exposed across the boundary).
- Sub-agent return-format hints for the heaviest dispatches: the
  `generateAllTransitionEnds` SUMMARY (`SkeletalTrapezoidation.cpp:1247-1403`)
  should request the recursive travel structure + the fractional
  `transition_ratio` assignment explicitly. The `filterTransitionMids` SUMMARY
  (`:1007-1076`) should request the recursive dissolve condition
  (same-`lower_bead_count` within `transition_filter_dist`).