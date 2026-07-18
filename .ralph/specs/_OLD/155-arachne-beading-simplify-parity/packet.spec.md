---
status: implemented
packet: 155-arachne-beading-simplify-parity
task_ids:
  - none
backlog_source: docs/18_arachne_parity_audit.md
context_cost_estimate: M
---

# Packet Contract: 155-arachne-beading-simplify-parity

## Goal

Close the round-3 Arachne parity gaps **G15** (BeadingStrategy::getSplitMiddleThreshold absent from the trait surface) and **G20** (ExtrusionLine::simplify missing the dist_greater intersection-distance gate) by faithfully porting the canonical OrcaSlicer algorithms into `slicer-core`.

## Scope Boundaries

Adds two **required** (no-default) trait methods to `BeadingStrategy` — `get_split_middle_threshold(&self) -> f64` and `get_add_middle_threshold(&self) -> f64`, both taking no arguments (OrcaSlicer: `double getSplitMiddleThreshold() const`, `BeadingStrategy.hpp:166`). Every one of the five concrete implementors gains an impl: `DistributedBeadingStrategy` stores the two thresholds as fields and returns them; the four decorators (`Redistribute`, `Widening`, `OuterWallInset`, `Limited`) **forward to `self.parent`**. Forwarding is mandatory, not optional — it is what reproduces OrcaSlicer's `BeadingStrategy(*parent)` copy-construction (`RedistributeBeadingStrategy.cpp:39-48`, `WideningBeadingStrategy.cpp:39-44`, `OuterWallInsetBeadingStrategy.cpp:39-43`, `LimitedBeadingStrategy.cpp:54-58`), which is why a threshold set on the innermost `Distributed` is observable at the top of the stack in Orca. Plumbs the two thresholds through `BeadingFactoryParams` → `DistributedBeadingStrategy::new`, and ports `Distributed::optimal_bead_count` + base `get_transition_thickness` plus the three `RedistributeBeadingStrategy` overrides (`optimal_bead_count`, `get_transition_thickness`, `optimal_thickness`) to consume them.

Restructures `simplify_distance_gated` to track `previous_previous` alongside `last_retained` and ports the `next_length2 > 4 * smallest_line_segment_squared` special case (`ExtrusionLine.cpp:166-220`), including `intersection_infinite`, the `dist_greater` gate (`ExtrusionLine.cpp:180-188`), and the junction-replacement else-branch (`ExtrusionLine.cpp:201-217`). The `use_distance_gates` condition in `simplify_line` is **NOT** changed — see AC-6.

The thresholds are computed in `BeadingStrategyFactory::create_stack` from the already-registered `min_bead_width` / `optimal_width` / `preferred_bead_width_outer` config keys (surfaced on `BeadingFactoryParams` as `min_output_width`, `optimal_width`, `preferred_bead_width_outer`). No WIT/IR/module-WIT changes; no scheduler or path-optimization changes; no new config keys; the arachne-perimeters module TOML is untouched.

## Prerequisites and Blockers

- Depends on: none (no other open packet).
- Unblocks: `156-arachne-region-order` (packet B) — both share the G15/G20/G12 audit cluster but the region-order packet has no read dependency on this packet.
- Activation blockers: none. 

## Acceptance Criteria

Acceptance Criteria are stated **once**, here. `requirements.md` references them by ID.

- **AC-1 (G15 trait surface + stack forwarding). Given** the `BeadingStrategy`
  trait in `crates/slicer-core/src/beading/mod.rs`, **when** the trait is
  extended, **then** it gains two **required** (no-default) methods
  `get_split_middle_threshold(&self) -> f64` and
  `get_add_middle_threshold(&self) -> f64` — both taking **no arguments**,
  matching `double getSplitMiddleThreshold() const` (`BeadingStrategy.hpp:166`);
  `DistributedBeadingStrategy` returns its two stored fields, and **all four
  decorators** (`RedistributeBeadingStrategy`, `WideningBeadingStrategy`,
  `OuterWallInsetBeadingStrategy`, `LimitedBeadingStrategy`) implement both by
  forwarding to `self.parent`. The trait remains object-safe (`Box<dyn
  BeadingStrategy>` still compiles, e.g. at `factory.rs:188` and its other
  `create_stack` usage sites at `:209,219,225,239,248`). *Rationale: these
  decorators do not inherit trait defaults through `parent` — each already
  implements every trait method explicitly and forwards. A default impl would
  be silently picked up at the `Limited` layer and shadow the real value, so
  no default is provided.* |
  `cargo check --workspace --all-targets && cargo test -p slicer-core --test beading_factory -- beading_factory_threshold_propagates_through_full_stack --exact`
- **AC-2 (G15 threshold observable at top of stack). Given** a
  `BeadingStrategyFactory::create_stack` output built from
  `BeadingFactoryParams::default()` with `print_thin_walls = true` and
  `outer_wall_offset = 1.0` (so the stack is
  `Distributed → Redistribute → Widening → OuterWallInset → Limited`, i.e.
  every decorator is present), **when** the test calls
  `stack.get_split_middle_threshold()` and `stack.get_add_middle_threshold()`
  **on the `Limited` top of the stack**, **then** both return the values the
  factory computed for `Distributed` — `0.99` and `0.99` for the shipped
  defaults (see AC-5) — proving forwarding works through all four decorator
  layers. The runtime RED test's `assert!(false)` body is rewritten to make
  these two calls (the audit doc's "closing a gap turns its test green with
  no rewrite" promise is explicitly relaxed for this test per its own doc note
  at `arachne_parity_round2.rs:120-132`). |
  `cargo test -p slicer-runtime --test arachne_parity_round2 -- arachne_parity_beading_split_middle_threshold_exposed --exact`
- **AC-3 (G15 Distributed optimal_bead_count — truncation + parity selection).
  Given** a `DistributedBeadingStrategy` with `optimal_width = 4000` units
  (0.4 mm), **when** `optimal_bead_count(7500)` (0.75 mm) is called, **then**
  the OrcaSlicer formula (`DistributedBeadingStrategy.cpp:132-144`) is applied:
  `naive_count = trunc(7500 / 4000) = 1` (**integer truncation, not
  `.round()`**), `remainder = 7500 - 1*4000 = 3500`,
  `minimum_line_width = 4000 * (naive_count % 2 == 1 ? split : add)`,
  `return naive_count + (remainder >= minimum_line_width)`. The test is
  parameterised over thresholds and **must discriminate the new impl from the
  old `.round()` impl** (`distributed.rs:177-179`):
  - `split = add = 0.99` → `minimum_line_width = 3960`; `3500 >= 3960` is
    false → **1 bead**. (The old `.round()` impl returns `round(1.875) = 2` —
    this input is the falsifying case.)
  - `split = add = 0.5` → `minimum_line_width = 2000`; `3500 >= 2000` is
    true → **2 beads**.
  - `optimal_bead_count(8000)` with `split = add = 0.5` → `naive_count = 2`
    (even) → `add` is selected → `remainder = 0` → **2 beads** (locks the
    parity-based selector). |
  `cargo test -p slicer-core --test beading_distributed -- distributed_optimal_bead_count_uses_split_middle_threshold --exact`
- **AC-4 (G15 Redistribute port). Given** a `RedistributeBeadingStrategy` with
  `optimal_width_outer = W = 4000` units, `minimum_variable_line_ratio = 0.5`,
  wrapping a `Distributed` whose `get_split_middle_threshold()` returns `0.5`,
  **when** the three ported methods are called, **then** they match
  `RedistributeBeadingStrategy.cpp:50-85` exactly:
  - `optimal_bead_count(0.7 * W)` → `1` (`0.7W >= 0.5W` so not 0; `0.7W <= 2W`;
    `0.7W > (1.0 + 0.5) * W = 1.5W` is **false** → 1).
  - `optimal_bead_count(1.6 * W)` → `2` (`1.6W <= 2W`; `1.6W > 1.5W` is
    **true** → 2). *Note: `0.9 * W` also returns `1`, not `2` — the 2-bead
    branch requires `thickness > (1 + split) * W`.*
  - `optimal_bead_count(0.4 * W)` → `0` (`0.4W < minimum_variable_line_ratio * W`).
  - `get_transition_thickness(0)` → `minimum_variable_line_ratio * W = 0.5W`
    (the `case 0` branch — **not** `(1 + split) * W`).
  - `get_transition_thickness(1)` → `(1.0 + 0.5) * W = 1.5W` (the `case 1`
    branch, the one that consults `parent.get_split_middle_threshold()`).
  - `get_transition_thickness(3)` → `parent.get_transition_thickness(1) + 2 * W`
    (the `default` branch).
  - `optimal_thickness(4)` → `parent.optimal_thickness(2) + W * 2`
    (`inner = max(0, 4 - 2) = 2`, `outer = 4 - 2 = 2`). |
  `cargo test -p slicer-core --test beading_redistribute -- redistribute_optimal_bead_count_consults_split_middle --exact`
- **AC-5 (G15 factory plumbing). Given** `BeadingFactoryParams::default()` —
  whose relevant live field values are `min_output_width: 4000.0`,
  `optimal_width: 4000.0`, `preferred_bead_width_outer: 4000.0`
  (`factory.rs:144-160`; **note there is no `min_bead_width` field** — the
  `min_bead_width` config key surfaces as `min_output_width`) — **when**
  `BeadingStrategyFactory::create_stack` builds `DistributedBeadingStrategy`,
  **then** it passes:
  - `wall_split_middle_threshold = clamp(2 * min_output_width / preferred_bead_width_outer - 1, 0.01, 0.99)`
    = `clamp(2*4000/4000 - 1, 0.01, 0.99)` = `clamp(1.0, …)` = **0.99**
  - `wall_add_middle_threshold = clamp(min_output_width / optimal_width, 0.01, 0.99)`
    = `clamp(1.0, …)` = **0.99**

  This mirrors `WallToolPaths.cpp:619-640`, which divides the split threshold
  by the **external** perimeter width and the add threshold by the **inner**
  perimeter width. PnP's nearest analogues are `preferred_bead_width_outer` and
  `optimal_width`. The residual deviation (Orca first converts both widths
  through `Flow::rounded_rectangle_extrusion_width_from_spacing`, which PnP has
  no analogue for at this site) is recorded as **D-155**; the clamp bounds
  `[0.01, 0.99]` are OrcaSlicer's exact constants and **must not be altered**. |
  `cargo test -p slicer-core --test beading_factory -- beading_factory_passes_split_middle_thresholds --exact`
- **AC-6 (G20 RED test reaches the gate). Given** the 4-junction open
  `ExtrusionLine` from `fixtures::simplify_input_intersection_distance_gate()`
  (`[(0,0), (5,0.05), (5.01,0.04), (10,0)]` mm), **when**
  `simplify_toolpaths` is called with `smallest_line_segment_squared = 1e-3`
  (mm²), `allowed_error_distance_squared = 1.0`, and
  `maximum_extrusion_area_deviation = f64::INFINITY`, **then** all 4 junctions
  are retained *because the tier-3 `dist_greater` gate rejects removal*.

  **This AC replaces the RED test's current parameters**
  (`smallest_line_segment_squared = 0.0`, `allowed_error_distance_squared =
  f64::INFINITY`, `arachne_parity_round2.rs:192`), which are **vacuous**: in
  canonical OrcaSlicer the special case is nested inside
  `if (length2 < smallest_line_segment_squared && height_2 <= allowed_error_distance_squared)`
  (`ExtrusionLine.cpp:162-164`), and `length2` is a `squaredNorm()` (always
  ≥ 0), so `smallest_line_segment_squared == 0` makes the guard `length2 < 0` —
  unsatisfiable for every input, leaving `ExtrusionLine.cpp:166-220` dead. The
  old parameters therefore prove nothing about the gate. The new parameters put
  junction 2 (`(5.01,0.04)`) squarely inside the gate: `length2 ≈ 2e-4 < 1e-3`,
  `height_2 ≈ 9.8e-5 <= 1.0`, `next_length2 ≈ 24.9 > 4e-3`, and the
  `(prev_prev,prev)`×`(curr,next)` intersection lands at ≈ `(4.450, 0.0445)`,
  ≈ `0.30 mm²` (squared) from `prev` — far above `smallest_line_segment_squared`
  → `dist_greater` fires → junction **retained**. Under the current impl the
  same input **removes** the junction (3 retained), so the test is genuinely
  RED. Changing the test's parameters (not its assertion strength) is a
  declared, justified exception to the "test bodies are immutable" rule — the
  assertion is *strengthened* from `kept >= 4` to `kept == 4` with the exact
  junction sequence checked.

  **`use_distance_gates` (`simplify.rs:103-104`) is NOT modified** — with
  `smallest_line_segment_squared = 1e-3 > 0` and
  `allowed_error_distance_squared = 1.0 > 0` the existing condition already
  routes to `simplify_distance_gated`. |
  `cargo test -p slicer-runtime --test arachne_parity_round2 -- arachne_parity_simplify_intersection_distance_gate_present --exact`
- **AC-7 (G20 tier-3 dist_greater gate). Given** an open `ExtrusionLine` where,
  for some interior junction `curr`, `length2(prev→curr) < smallest_line_segment_squared`,
  `height_2 <= allowed_error_distance_squared`, and
  `next_length2(curr→next) > 4 * smallest_line_segment_squared`, and the
  intersection of the infinite lines `(prev_prev, prev)` and `(curr, next)` is
  farther than `smallest_line_segment_squared` from `prev` **or** from `curr`,
  **when** `simplify_distance_gated` processes `curr`, **then** `curr` is
  RETAINED (`ExtrusionLine.cpp:189-200` — the reject path). The `dist_greater`
  predicate takes **three** arguments — `(p1, p2, threshold)` — and does the
  overflow-avoiding component-wise fast-reject before the squared-norm compare
  (`ExtrusionLine.cpp:180-188`). |
  `cargo test -p slicer-core --features host-algos --test arachne_simplify_intersection_distance_gate_tdd -- simplify_intersection_distance_gate_preserves_junction --exact`
- **AC-8 (G20 junction replacement). Given** an open `ExtrusionLine` where the
  same tier-3 special case fires but the intersection point lies **within**
  `smallest_line_segment_squared` of both `prev` and `curr` (and passes the
  remaining reject conditions), **when** `simplify_distance_gated` processes
  `curr`, **then** the else-branch at `ExtrusionLine.cpp:201-217` runs:
  a new junction is built at the intersection carrying **`curr`'s** width and
  `curr.perimeter_index` verbatim (no interpolation/averaging); the
  previously-pushed junction is POPPED from the result and `previous` is
  restored to `previous_previous`; then `previous_previous = previous`,
  `previous = new_junction`, the new junction is pushed, and the loop
  `continue`s. |
  `cargo test -p slicer-core --features host-algos --test arachne_simplify_intersection_distance_gate_tdd -- simplify_junction_replacement_moves_to_intersection --exact`
- **AC-9 (G20 Shoelace height_2 + accumulator). Given** the simplify walk,
  **when** the tier-2 and tier-3 gates compute their height, **then** the
  existing `point_line_distance_squared` height is replaced by OrcaSlicer's
  Shoelace-derived height (`ExtrusionLine.cpp:151`):
  `height_2 = (area_removed_so_far)² / base_length_2`, where
  `base_length_2 = |next - previous|²` (`:140`) and
  `area_removed_so_far = accumulated_area_removed + negative_area_closing`
  (`:139`) — **a per-iteration local, NOT the accumulator itself**. The running
  accumulator is a separate variable `accumulated_area_removed` (declared
  `:104`, incremented by `removed_area_next` at `:131`, and **reset** to
  `removed_area_next` at `:211` (replacement branch) and `:223` (retain
  branch)). The test asserts the two formulas diverge for an input with a
  removed upstream junction (i.e. where `accumulated_area_removed != 0`),
  proving the accumulator is threaded, not recomputed per-junction. |
  `cargo test -p slicer-core --features host-algos --test arachne_simplify_intersection_distance_gate_tdd -- simplify_distance_gated_uses_shoelace_height_2 --exact`
- **AC-10 (regression lock). Given** the 14 green `arachne_parity.rs` locks
  + the round-2 G3/G10 closures, **when** the beading threshold + simplify
  changes land, **then** all 14 + G3/G10 still pass (no regressions). The
  `factory_orca_reference.json` golden must not break. **Expected blast
  radius (must be checked, not assumed):** the AC-5 defaults yield
  `0.99 / 0.99`, i.e. `minimum_line_width = 0.99 * optimal_width`, which makes
  the "add a middle bead" branch far *less* likely to fire than the old
  `.round()` heuristic (which behaved like a `0.5` threshold). Any bead-count
  or wall-count fixture that shifts is a **real behavior change**, not a test
  bug — re-record only after confirming the new value matches the OrcaSlicer
  formula by hand, and record the shift in D-155. |
  `cargo test -p slicer-runtime --test arachne_parity && cargo test -p slicer-core`

## Negative Test Cases

- **AC-N1 (G15 clamp bounds are OrcaSlicer's exact constants). Given**
  `BeadingFactoryParams` with `min_output_width = 100.0`,
  `preferred_bead_width_outer = 4000.0`, `optimal_width = 4000.0` (a very thin
  `min_bead_width` relative to the nozzle), **when** the factory computes the
  thresholds, **then** the split threshold clamps to the **lower** bound
  `0.01` (`2*100/4000 - 1 = -0.95` → clamped) and the add threshold clamps to
  `0.025` (unclamped, inside the band); and with
  `min_output_width = 100_000.0` the split threshold clamps to the **upper**
  bound `0.99`. The bounds `[0.01, 0.99]` are OrcaSlicer's literal constants
  (`WallToolPaths.cpp:619-640`) and are NOT to be widened. |
  `cargo test -p slicer-core --test beading_factory -- beading_factory_threshold_clamp_bounds_are_canonical --exact`
- **AC-N2 (G15 beading regression). Given** the existing
  `redistribute.rs::compute` test (`redistribute_outer_consistent`, which
  asserts the symmetric-outer-walls behavior for `bead_count <= 2` per
  `crates/slicer-core/tests/beading/redistribute.rs:123-179`),
  **when** the new `optimal_bead_count`/`get_transition_thickness`/`optimal_thickness`
  overrides are added, **then** the `compute` test still passes — `compute` is
  NOT touched by this packet; only the count/thickness methods change. |
  `cargo test -p slicer-core --test beading_redistribute -- redistribute_outer_consistent --exact`
- **AC-N3 (G20 degenerate input). Given** an open `ExtrusionLine` with
  `junctions.len() == 2`, **when** `simplify_distance_gated` is called,
  **then** it returns the input unchanged (the `n <= 2` early-return at
  `simplify.rs:146-148` is preserved across the restructure; `previous_previous`
  is never dereferenced). This matches OrcaSlicer's
  `min_path_size = is_closed ? 3 : 2; if (junctions.size() <= min_path_size) return;`
  (`ExtrusionLine.cpp:63-65`) for the open case. |
  `cargo test -p slicer-core --features host-algos --test arachne_simplify_intersection_distance_gate_tdd -- simplify_degenerate_two_junctions_unchanged --exact`
- **AC-N4 (G20 closed-line early-return). Given** an `ExtrusionLine` with
  `is_closed = true` and 3 junctions, **when** `simplify_distance_gated` is
  called, **then** the result has exactly 3 junctions. OrcaSlicer's closed-line
  guard is `junctions.size() <= 3` (`ExtrusionLine.cpp:63-65`), so a 3-junction
  closed line is returned untouched. Full closed-line parity (the `spill_over`
  wrap-around at `ExtrusionLine.cpp:121-123`, the mid-loop ≤3 guard at
  `:114-118`, and the `front().p = back().p` copy at `:230-238`) is OUT OF
  SCOPE for this packet (deferred; see Open Questions). |
  `cargo test -p slicer-core --features host-algos --test arachne_simplify_intersection_distance_gate_tdd -- simplify_closed_line_minimum_size_preserved --exact`

## Verification

Gate commands only — full matrix in `requirements.md` §Verification Commands.

- `cargo test -p slicer-runtime --test arachne_parity_round2 -- arachne_parity_beading_split_middle_threshold_exposed arachne_parity_simplify_intersection_distance_gate_present`
- `cargo check --workspace --all-targets --features host-algos`
- `cargo clippy --workspace --all-targets --features host-algos -- -D warnings`

## Authoritative Docs

- `docs/18_arachne_parity_audit.md` — load the G15 and G20 detailed-gap
  sections only (line ~324-388); the summary table is the canonical entry
  point.
- `docs/08_coordinate_system.md` — load directly; short file; the
  `mm_to_units()` helper is used at every threshold-computation site.
- `docs/DEVIATION_LOG.md` — D-105B/C/E entries (close precedents) load
  directly; the new entries for D-155 (beading thresholds) and D-156
  (simplify intersection gate) are added in this packet.
- `docs/02_ir_schemas.md` — delegate a SUMMARY of the
  `ExtrusionJunction`/`Point3WithWidth` field set (the implementer only
  needs the f32-mm coordinate contract and the `width`/`perimeter_index`
  fields touched by junction-replacement).
- `docs/15_config_keys_reference.md` — load the `min_bead_width` and
  `optimal_width` entries directly (the thresholds are derived from these
  two keys; their defaults are the test inputs).

## Doc Impact Statement (Required)

This packet changes a beading trait surface and the simplify algorithm
contract — `none` is not eligible. Sections added/modified:

- `docs/18_arachne_parity_audit.md` §"Gap summary table" — mark G15 + G20
  closed — `rg -q 'G15.*closed' docs/18_arachne_parity_audit.md`
- `docs/18_arachne_parity_audit.md` §"Detailed gaps" — update the
  G15/G20 entries' "PnP status" to "closed (this packet)" —
  `rg -q 'wall_split_middle_threshold' docs/18_arachne_parity_audit.md`
- `docs/DEVIATION_LOG.md` — add D-155 (beading threshold parity) and
  D-156 (simplify intersection gate) — `rg -q 'D-155' docs/DEVIATION_LOG.md`.
  **D-155 must record all three residuals:** (a) `get_add_middle_threshold`
  is a net-new PnP symbol with no OrcaSlicer counterpart (Orca has the field
  but no getter); (b) the clamp inputs use `preferred_bead_width_outer` /
  `optimal_width` in place of Orca's
  `Flow::rounded_rectangle_extrusion_width_from_spacing`-converted
  external/inner perimeter widths; (c) with the shipped defaults
  (`min_output_width == optimal_width == preferred_bead_width_outer == 4000`)
  **both thresholds saturate at the `0.99` clamp ceiling**, which suppresses
  middle-bead addition relative to the old `.round()` behavior — any fixture
  bead-count shift from AC-10 is a consequence of this and must be enumerated
  in the entry. **D-156 must record** that the G20 RED test's parameters were
  changed (see AC-6) because the originals could not reach the gate.
- `CONTEXT.md` — add glossary entries for *split-middle threshold* and
  *intersection-distance gate* — `rg -q 'split-middle threshold' CONTEXT.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

> **All line numbers below were resolved against the real
> `OrcaSlicerDocumented/` tree on 2026-07-14.** The previous draft of this
> packet carried line refs that were wrong by 20–500 lines in every case; do
> not reintroduce them. If a delegated read does not find the claimed content
> at the claimed line, stop and re-resolve — do not guess.

- `OrcaSlicerDocumented/src/libslic3r/Arachne/BeadingStrategy/BeadingStrategy.hpp:166` — `double getSplitMiddleThreshold() const;` — **the only public threshold accessor; it takes NO arguments.** There is **no `getAddMiddleThreshold()` anywhere in OrcaSlicer** (grep of `Arachne/BeadingStrategy/` returns zero hits) — PnP's `get_add_middle_threshold` is a **net-new PnP symbol**, justified because PnP's decorators cannot read a protected base field the way Orca's copy-constructed base can. Record under D-155.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/BeadingStrategy/BeadingStrategy.hpp:74-78, 182, 186` — the base-class constructor (both thresholds are **required parameters**, no sentinel/default) and the two protected fields `wall_split_middle_threshold` / `wall_add_middle_threshold`. This is why the PnP trait methods are **required, not defaulted** (AC-1).
- `OrcaSlicerDocumented/src/libslic3r/Arachne/BeadingStrategy/BeadingStrategy.cpp:90-102` — base `getTransitionThickness`'s parity-based threshold selection (`lower_bead_count % 2 == 1 ? wall_split_middle_threshold : wall_add_middle_threshold`).
- `OrcaSlicerDocumented/src/libslic3r/Arachne/BeadingStrategy/DistributedBeadingStrategy.cpp:132-144` — `getOptimalBeadCount`: **integer-truncating** `naive_count = thickness / optimal_width`, parity-based `minimum_line_width`, and `return naive_count + (remainder >= minimum_line_width)` (a `>=`, not `>`). AC-3's arbiter.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/BeadingStrategy/RedistributeBeadingStrategy.cpp:50-85` — `getOptimalThickness` / `getTransitionThickness` (the `parent->getSplitMiddleThreshold()` call is in the **`case 1`** branch, not `case 0`) / `getOptimalBeadCount` (the 2-bead branch requires `thickness > (1.0 + split) * optimal_width_outer`). AC-4's arbiter.
- Decorator base construction — `RedistributeBeadingStrategy.cpp:39-48`, `WideningBeadingStrategy.cpp:39-44`, `OuterWallInsetBeadingStrategy.cpp:39-43`, `LimitedBeadingStrategy.cpp:54-58` — every decorator copy-constructs `BeadingStrategy(*parent)`, so the thresholds propagate up the whole stack. **This is the mechanism AC-1's mandatory forwarding reproduces.**
- `OrcaSlicerDocumented/src/libslic3r/Arachne/BeadingStrategy/BeadingStrategyFactory.cpp:69-96` — stack order `Distributed → Redistribute → [Widening] → [OuterWallInset] → Limited` (identical to PnP's `factory.rs:187-228`).
- `OrcaSlicerDocumented/src/libslic3r/Arachne/WallToolPaths.cpp:619-640` — the threshold clamp formulas. **Split divides by the external perimeter width; add divides by the inner perimeter width** — two *different* widths, each first passed through `Flow::rounded_rectangle_extrusion_width_from_spacing`. AC-5's arbiter and the source of the D-155 residual.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/utils/ExtrusionLine.cpp:56-243` — the full `ExtrusionLine::simplify` impl. Key inner landmarks: early-return `:63-65`; accumulator declared `:104`; closed-line mid-loop guard `:114-118`; `spill_over` `:121-123`; `length2` `:133`; ultra-short bypass `:134`; `area_removed_so_far` local `:139`; `base_length_2` `:140`; **Shoelace `height_2` `:151`**; colinear tier `:153-159`; **tier-3 main gate `:162-164`**; **special case `:166`**; `dist_greater` lambda **`:180-188`** (three args); reject path `:189-200`; **replacement else-branch `:201-217`**; accumulator resets **`:211` and `:223`**; closed-line front=back copy `:230-238`. Arbiter of AC-6/AC-7/AC-8/AC-9.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
