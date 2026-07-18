---
status: implemented
packet: 155-arachne-beading-simplify-parity
task_ids:
  - none
---

# 155-arachne-beading-simplify-parity

## Goal

Close the round-3 Arachne parity gaps **G15** (BeadingStrategy::getSplitMiddleThreshold absent from the trait surface) and **G20** (ExtrusionLine::simplify missing the dist_greater intersection-distance gate) by faithfully porting the canonical OrcaSlicer algorithms into `slicer-core`.

## Problem Statement

The Arachne beading-strategy stack and the `simplify_toolpaths` post-processor
each diverge from OrcaSlicer on a physically or algorithmically load-bearing
point, each an accepted-but-open gap:

- **G15 (BeadingStrategy::getSplitMiddleThreshold absent):** OrcaSlicer's
  `BeadingStrategy` base class stores two internal thresholds
  (`wall_split_middle_threshold` and `wall_add_middle_threshold`) and exposes
  them via `getSplitMiddleThreshold()`. The base `getTransitionThickness`
  uses a parity-based selection (`lower_bead_count % 2 == 1 ?
  wall_split_middle_threshold : wall_add_middle_threshold`);
  `DistributedBeadingStrategy::getOptimalBeadCount` uses the same formula;
  `RedistributeBeadingStrategy::getOptimalBeadCount` and
  `getTransitionThickness` call `parent->getSplitMiddleThreshold()` to
  decide whether the middle bead splits (1 ↔ 2 transition). PnP's
  `BeadingStrategy` trait does not expose either method;
  `DistributedBeadingStrategy` uses a `.round()` heuristic;
  `RedistributeBeadingStrategy` delegates to parent unchanged. The result is
  that PnP's bead-count selection diverges from Orca for odd/middle-bead
  regimes (`thickness` in `(optimal_width_outer, 2*optimal_width_outer)`).
- **G20 (ExtrusionLine::simplify missing intersection-distance gate):**
  OrcaSlicer's tier-3 simplify block has a special case when the *next*
  segment is much longer than the *current* one (`next_length2 > 4 *
  smallest_line_segment_squared`): it tries to move the current junction to
  the intersection of the `(prev_prev, prev)` and `(curr, next)` extended
  lines, and if that intersection is more than `smallest_line_segment_squared`
  from either `prev` or `curr` (the `dist_greater` gate), it REJECTS removal
  and preserves the junction. PnP's `simplify_distance_gated` has no
  `previous_previous` cursor and no intersection-distance predicate, so it
  drops near-colinear middle junctions that OrcaSlicer preserves — visible
  as "Z-shape polylines" getting flattened to 2 junctions.

These form one coherent slice because (a) both gaps live in `slicer-core`
arachne code with no WIT/IR/module impact, (b) both have RED TDD tests
already in place (`arachne_parity_round2.rs`), (c) the G15 trait surface
change is contained to the beading module + factory and the G20 simplify
restructure is contained to `simplify.rs`, and (d) closing them together
avoids landing the `BeadingFactoryParams` plumbing twice.

## Architecture Constraints

- `BeadingStrategy` must remain object-safe (the new methods take no
  arguments and no generics, so the existing `Send + Sync` bound and
  `Box<dyn BeadingStrategy>` usage at `factory.rs:174` are preserved).
- **The two new trait methods have NO default impl, and every one of the
  five implementors must implement them.** This is a hard constraint, not a
  style preference. In this tree the four decorators (`Redistribute`,
  `Widening`, `OuterWallInset`, `Limited`) each hold
  `parent: Box<dyn BeadingStrategy>` and *already explicitly implement every
  trait method*, forwarding the non-`compute` ones to `parent` — none of them
  relies on a trait default. If the threshold methods were given a default,
  the decorators would silently pick up **that default** rather than the
  parent's real value, and `stack.get_split_middle_threshold()` on a
  factory-built stack (whose top is always `LimitedBeadingStrategy`) would
  return the default, never `Distributed`'s value. OrcaSlicer sidesteps this
  entirely by copy-constructing the base (`BeadingStrategy(*parent)` in all
  four decorator ctors), which propagates the fields structurally. Required
  methods + explicit forwarding is PnP's equivalent. AC-2 tests exactly this
  propagation through a fully-populated stack.
- The simplify restructure must preserve the existing `n <= 2`
  early-return (AC-N3) and the closed-line minimum-size guard (AC-N4).
  Full `is_closed = true` parity (the `spill_over` wrap-around at
  `ExtrusionLine.cpp:121-123`, the mid-loop ≤3 guard at `:114-118`, and the
  `front().p = back().p` copy at `:230-238`) is OUT OF SCOPE.
- The `simplify_input_intersection_distance_gate` fixture
  (`crates/slicer-runtime/tests/fixtures/arachne_parity/mod.rs:135`) uses
  mm-coordinate f32 fields on the nested `Point3WithWidth`
  (`ExtrusionJunction { p: Point3WithWidth { x: f32, y: f32, width: f32, .. },
  perimeter_index: u32 }` — `crates/slicer-ir/src/slice_ir.rs:1618-1632,
  1819-1825`; note the width field is `p.width`, **not** `w` as in OrcaSlicer).
  These must round-trip through the new `line_intersection_infinite` helper
  (f64 mm space) without precision loss greater than the existing
  `point_line_distance_squared` helper (which already uses the same
  conversion).
- The fixture's *junction data* is unchanged by this packet. Only the
  *parameters* the RED test passes to `simplify_toolpaths` change (AC-6).

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- Packet-specific constraint: the two thresholds are **dimensionless
  ratios**, not lengths. `BeadingFactoryParams`'s width fields
  (`min_output_width`, `optimal_width`, `preferred_bead_width_outer`) are
  already in slicer units (f64), so the clamp formulas divide unit-by-unit
  and the result is unit-free — **no `mm_to_units()` / `UNITS_PER_MM`
  conversion appears anywhere in the threshold computation.** Both
  `BeadingFactoryParams` and `DistributedBeadingStrategy` store them as bare
  `f64` ratios in `[0.01, 0.99]`.

- Packet-specific constraint: the `ExtrusionJunction.p.x/y` fields are
  f32 (mm). The new `line_intersection_infinite` helper accepts f64
  inputs and returns f64; call sites must cast the f32 inputs to f64
  (the same pattern `calculate_extrusion_area_deviation_error` already
  uses at `simplify.rs:291-293`).

## Data and Contract Notes

- `BeadingStrategy` trait gains 2 methods, exact signature
  `fn get_split_middle_threshold(&self) -> f64` /
  `fn get_add_middle_threshold(&self) -> f64` — **no arguments**, matching
  OrcaSlicer's `double getSplitMiddleThreshold() const`
  (`BeadingStrategy.hpp:166`). **No default impls** (see Architecture
  Constraints). The trait remains object-safe.
- `BeadingFactoryParams` gains 2 dimensionless `f64` ratio fields
  (`wall_split_middle_threshold`, `wall_add_middle_threshold`). Their
  `Default` values are derived from the existing live defaults
  `min_output_width: 4000.0`, `preferred_bead_width_outer: 4000.0`,
  `optimal_width: 4000.0` (`factory.rs:144-160`) via the OrcaSlicer clamp
  formulas → `0.99 / 0.99` (both saturate at the ceiling; see Risks).
  **There is no `min_bead_width` field on this struct** — the
  `min_bead_width` config key surfaces as `min_output_width`.
- `ExtrusionLine` / `ExtrusionJunction` / `Point3WithWidth` are
  UNCHANGED (no IR struct changes).
- `ArachneParams` is UNCHANGED (no `wall_split_middle_threshold` /
  `wall_add_middle_threshold` host-facing fields; the thresholds are
  internal Arachne parameters derived in the factory).
- No WIT contract changes (no `slicer-macros` / `slicer-schema` /
  `slicer-ir` IR / WIT boundary touches).
- No scheduler / host-service / manifest TOML changes.
- Determinism preserved: the G15 trait methods return constants
  (the thresholds are computed once at factory time, not per-call);
  the G20 simplify restructure uses a deterministic topological
  walk.

## Locked Assumptions and Invariants

- `BeadingStrategy` must remain object-safe (no generic method params,
  no `where Self: Sized` bounds added to the new methods).
- The two new trait methods are **required**, and all five implementors
  implement them; the four decorators forward to `self.parent`. No default
  impl exists (see Architecture Constraints).
- `compute` on every `BeadingStrategy` impl is UNCHANGED by this packet
  (G15 only changes the count/thickness methods, not the bead-width
  algorithm; AC-N2 locks this).
- The `n <= 2` simplify early-return is preserved (AC-N3).
- The closed-line minimum-size guard is preserved (AC-N4).
- **`use_distance_gates` (`simplify.rs:103-104`) is NOT modified by this
  packet.** The dist-gated path is reached because the corrected RED-test
  parameters supply `smallest_line_segment_squared = 1e-3 > 0` and
  `allowed_error_distance_squared = 1.0 > 0`, which the *existing*
  condition already accepts.
- The G20 simplify path uses the Shoelace `height_2` formula
  (`ExtrusionLine.cpp:151`) at both gate sites (AC-9). This is a behavior
  change from the existing `point_line_distance_squared` height calc, but
  the new formula matches OrcaSlicer exactly.
- Any caller constructing a `BeadingFactoryParams` literally (rather than
  via `..Default::default()`) must set the two new fields. Step 1's audit
  checklist enumerates these call sites.
- The clamp bounds `[0.01, 0.99]` are OrcaSlicer's literal constants and
  **must not be widened or otherwise altered** (AC-N1 locks them).

## Risks and Tradeoffs

- **Threshold saturation at the shipped defaults (highest-impact risk).**
  With `min_output_width == optimal_width == preferred_bead_width_outer ==
  4000`, both clamp formulas evaluate to `1.0` and saturate at the `0.99`
  ceiling. That makes `minimum_line_width = 0.99 * optimal_width`, so the
  "add a middle bead" branch almost never fires — materially *less* often
  than the old `.round()` heuristic, which behaved like a `0.5` threshold.
  This is a genuine, intended parity change (OrcaSlicer's own defaults have
  `min_bead_width` well below the perimeter width, e.g. 85% of nozzle, so
  its thresholds land mid-band), but it means **AC-10 fixture shifts are
  expected and must be adjudicated, not rubber-stamped**. The implementer
  must, for every shifted fixture: (1) recompute the expected bead count by
  hand from the OrcaSlicer formula, (2) confirm the new value matches, (3)
  re-record via the `#[ignore]` recorder, (4) enumerate the shift in D-155.
  **Do NOT widen the clamp bounds to make fixtures pass** — that would be a
  deviation from the very source this packet is porting. If the saturation
  is judged undesirable in production, the correct follow-up is to fix
  PnP's `min_bead_width` **config default** (currently equal to
  `optimal_width`, which is unlike OrcaSlicer), in a separate packet.
- The Shoelace `height_2` formula may change junction-removal decisions for
  existing simplify tests. Run the full `slicer-core` sweep after Step 4 and
  adjudicate each affected golden the same way.
- Two test bodies change in this packet, both in
  `crates/slicer-runtime/tests/arachne_parity_round2.rs`, and both are
  declared exceptions to the "test bodies are immutable" rule:
  (a) the G15 test's `assert!(false)` body is replaced with real calls
  (per its own doc note at `:120-132`); (b) the G20 test's
  `simplify_toolpaths` *parameters* at `:192` are corrected because
  `smallest_line_segment_squared = 0.0` cannot reach the gate under
  OrcaSlicer's own control flow (AC-6). Neither change may weaken an
  assertion — the G20 assertion is *strengthened* from `kept >= 4` to
  `kept == 4`.
