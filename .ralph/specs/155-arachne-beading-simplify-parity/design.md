# Design: 155-arachne-beading-simplify-parity

## Controlling Code Paths

- Primary code path (G15):
  - `crates/slicer-core/src/beading/mod.rs` — `BeadingStrategy` trait
    (adds two **required**, no-default, no-argument methods)
  - `crates/slicer-core/src/beading/distributed.rs` — new struct fields
    + impls + ported `optimal_bead_count` / `get_transition_thickness`
  - `crates/slicer-core/src/beading/redistribute.rs` — ported three
    `optimal_bead_count` / `get_transition_thickness` / `optimal_thickness`
    overrides + the two threshold accessors forwarding to `self.parent`
  - `crates/slicer-core/src/beading/{widening,outer_wall_inset,limited}.rs`
    — each gains the two threshold accessors forwarding to `self.parent`
    (**mandatory** — see the constraint below)
  - `crates/slicer-core/src/beading/factory.rs` — `BeadingFactoryParams`
    gains the two new fields; `BeadingStrategyFactory::create_stack`
    computes them from `min_output_width`, `preferred_bead_width_outer`
    and `optimal_width` and threads them to `DistributedBeadingStrategy::new`
- Primary code path (G20):
  - `crates/slicer-core/src/arachne/simplify.rs` — `simplify_distance_gated`
    restructured to track `previous_previous`; new `line_intersection_infinite`
    and `dist_greater` helpers; tier-3 special case + Shoelace `height_2`
  - `use_distance_gates` in `simplify_line` (`simplify.rs:103-104`) is
    **UNCHANGED**
- OrcaSlicer comparison surface: see `packet.spec.md` §OrcaSlicer Reference
  Obligations (delegate; never load). All line refs there were re-resolved
  against the real tree on 2026-07-14.

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

## Code Change Surface

- Selected approach (G15): add the two trait methods as **required**
  (no-default, no-argument) members; give `Distributed` the two stored
  fields and give all four decorators explicit `self.parent`-forwarding
  impls; thread the thresholds through the factory; port the three
  `Redistribute` methods + two `Distributed` methods faithfully from the
  OrcaSlicer source. The thresholds are computed in the factory from
  already-registered config keys (no new keys).
- Selected approach (G20): restructure `simplify_distance_gated` to
  use OrcaSlicer's two-back cursor pattern (track `previous` and
  `previous_previous` as full `ExtrusionJunction` **value copies**, not
  indices — `ExtrusionLine.cpp:75,79`); add the new helpers; port the full
  tier-3 block. The Shoelace `height_2` formula (`ExtrusionLine.cpp:151`)
  replaces the existing `point_line_distance_squared` height calc at both
  the tier-2 and tier-3 gate sites. **`use_distance_gates` is left
  unchanged** — the RED test's *parameters* are corrected instead (AC-6),
  which is the honest fix: the old parameters
  (`smallest_line_segment_squared = 0.0`) cannot reach the gate in
  OrcaSlicer either, so widening the PnP condition to accommodate them
  would have been a production behavior change invented purely to turn a
  test green.
- Exact functions, traits, manifests, tests, or fixtures expected to
  change:
  - `crates/slicer-core/src/beading/mod.rs` — add 2 required trait methods
  - `crates/slicer-core/src/beading/distributed.rs` — add 2 struct fields
    + 2 trailing `new()` args, implement 2 trait methods, port 2 impls
  - `crates/slicer-core/src/beading/redistribute.rs` — port 3 impls +
    2 forwarding accessors
  - `crates/slicer-core/src/beading/widening.rs` — 2 forwarding accessors
  - `crates/slicer-core/src/beading/outer_wall_inset.rs` — 2 forwarding accessors
  - `crates/slicer-core/src/beading/limited.rs` — 2 forwarding accessors
  - `crates/slicer-core/src/beading/factory.rs` — add 2 `BeadingFactoryParams`
    fields + their `Default` values; compute the clamps in `create_stack`;
    thread 2 args to `DistributedBeadingStrategy::new`
  - `crates/slicer-core/src/arachne/simplify.rs` — restructure
    `simplify_distance_gated`; add 2 helpers; replace the tier-2/tier-3
    `height_2` with the Shoelace formula. **`use_distance_gates` untouched.**
  - `crates/slicer-runtime/tests/arachne_parity_round2.rs` — rewrite the G15
    test body (`:134-160`) to call `stack.get_split_middle_threshold()` /
    `get_add_middle_threshold()` (no args) on a fully-decorated stack; and
    correct the G20 test's `simplify_toolpaths` parameters (`:192`) so the
    tier-3 gate is reachable, strengthening `kept >= 4` to `kept == 4`
  - `crates/slicer-core/tests/beading/distributed.rs` — add parameterised
    `distributed_optimal_bead_count_uses_split_middle_threshold`
  - `crates/slicer-core/tests/beading/redistribute.rs` — add
    `redistribute_optimal_bead_count_consults_split_middle`
  - `crates/slicer-core/tests/beading/factory.rs` — add
    `beading_factory_passes_split_middle_thresholds`,
    `beading_factory_threshold_propagates_through_full_stack`,
    `beading_factory_threshold_clamp_bounds_are_canonical`
  - `crates/slicer-core/src/arachne/generate_toolpaths.rs` — a
    `#[cfg(test)]` fixture (`FixedTestStrategy`) implements
    `BeadingStrategy`; it needs the 2 required trait methods
    (returning `0.99_f64`) to keep compiling. Test-only, mechanical
    ripple from AC-1's required-method addition; no production code
    in this file changes.
  - `crates/slicer-core/tests/beading/{limited,outer_wall_inset,widening}.rs`
    — companion unit-test updates for the corresponding decorator's
    2 new forwarding accessors.
  - New test file
    `crates/slicer-core/tests/arachne_simplify_intersection_distance_gate_tdd.rs`
    — 6 G20 tests (AC-7/AC-8/AC-9/AC-N3/AC-N4, plus a second AC-N4
    falsifying case using a near-colinear closed triangle). **No `Cargo.toml` edit is
    required**: files directly under `crates/slicer-core/tests/*.rs` are
    auto-discovered by Cargo as integration-test binaries (this is how every
    existing `arachne_*.rs` test in that directory is registered — none of
    them has a `[[test]]` entry). The `[[test]] path = …` entries in
    `Cargo.toml` exist only for the `tests/beading/` **subdirectory** files,
    which are not auto-discovered. The earlier draft's claim that "without
    this registration cargo does not pick the file up" was false for a
    top-level test file.
  - `docs/18_arachne_parity_audit.md` — update Gap summary table
  - `docs/DEVIATION_LOG.md` — add D-155 + D-156 entries
  - `CONTEXT.md` — add 2 glossary entries
- Rejected alternatives:
  - **Give the two trait methods a default impl (`f64::INFINITY` or any
    other value):** rejected — see the Architecture Constraint above. The
    decorators would shadow the parent's real value with the default, making
    AC-2 unsatisfiable and, worse, silently feeding `INFINITY` into
    `Redistribute::get_transition_thickness`'s `(1.0 + split) * W`
    expression, which would collapse `optimal_bead_count` to 1 for every
    thickness ≤ 2W.
  - **Add `get_split_middle_threshold` only (no `get_add_middle_threshold`):**
    rejected; the base `getTransitionThickness` and
    `Distributed::getOptimalBeadCount` both select between the two by parity
    of the bead count, so the add threshold is load-bearing for even counts.
    (Orca reads it from the protected base field rather than a getter; PnP
    needs the accessor because its decorators cannot see a parent's private
    fields. This is a recognised PnP-only symbol — D-155.)
  - **Hardcode 0.5/0.5 constants:** rejected; the thresholds must rescale
    with `min_bead_width` / nozzle width, per `WallToolPaths.cpp:619-640`.
  - **Port only the `dist_greater` gate (not the junction-replacement
    else-branch):** rejected; the full tier-3 port is in-scope per the audit
    doc and the user's decision ("Full port: gate + replacement"). The
    else-branch actively improves the polyline shape, not just preserves it.
  - **Widen `use_distance_gates` to fire when only
    `allowed_error_distance_squared > 0.0`:** rejected. It has no OrcaSlicer
    counterpart (Orca has no such switch — the gates are inline and the
    parameters simply make them inert), and it would change production
    behavior for every caller passing `smallest_line_segment_squared = 0.0`
    solely to accommodate a test fixture whose parameters were themselves
    wrong. The fixture parameters are fixed instead (AC-6).

## Files in Scope (read + edit)

- `crates/slicer-core/src/beading/mod.rs` — role: trait def;
  expected change: add 2 default-implemented methods.
- `crates/slicer-core/src/beading/distributed.rs` — role: `Distributed`
  impl; expected change: add fields, override methods, port 2 impls.
- `crates/slicer-core/src/beading/redistribute.rs` — role:
  `Redistribute` impl; expected change: port 3 impls.
- `crates/slicer-core/src/beading/factory.rs` — role: factory
  assembly; expected change: add 2 `BeadingFactoryParams` fields,
  thread to `Distributed::new`.
- `crates/slicer-core/src/arachne/simplify.rs` — role: simplify
  dist-gated path; expected change: restructure (two-back cursor) + 2 new
  helpers + tier-3 special case + Shoelace `height_2`.
  **`use_distance_gates` is NOT touched.**
- `crates/slicer-runtime/tests/arachne_parity_round2.rs` — role: G15
  fixture caller; expected change: rewrite the `assert!(false)` body
  to call the new trait method.
- `crates/slicer-core/src/beading/{widening,outer_wall_inset,limited}.rs`
  — role: decorators; expected change: 2 forwarding accessors each
  (mandatory — see Architecture Constraints).
- `crates/slicer-core/tests/beading/{distributed,redistribute,factory,limited,outer_wall_inset,widening}.rs`
  — role: unit-test extensions for the G15 methods (the latter three are
  companion updates for each decorator's 2 new forwarding accessors).
  Already registered as separate binaries via
  `[[test]] path = "tests/beading/*.rs"` in
  `crates/slicer-core/Cargo.toml:75-97` (they live in a subdirectory, so
  the explicit entry is required); no new registration needed for in-file
  additions.
- `crates/slicer-core/src/arachne/generate_toolpaths.rs` — role: a
  `#[cfg(test)]`-only `BeadingStrategy` fixture (`FixedTestStrategy`)
  needs the 2 required trait methods to keep compiling; expected change:
  2 trait-method stubs in the test module only, no production code.
- New test file `crates/slicer-core/tests/arachne_simplify_intersection_distance_gate_tdd.rs`
  — role: G20 unit tests for AC-7/AC-8/AC-9/AC-N3/AC-N4. **No `Cargo.toml`
  edit needed** — top-level `tests/*.rs` files are auto-discovered by Cargo
  (as every existing `crates/slicer-core/tests/arachne_*.rs` file already
  demonstrates).
- `docs/18_arachne_parity_audit.md` — doc update for G15 + G20 close.
- `docs/DEVIATION_LOG.md` — add D-155 + D-156.
- `CONTEXT.md` — 2 glossary entries.

## Read-Only Context

- `crates/slicer-core/src/arachne/pipeline.rs:269-292` —
  `to_beading_factory_params` — purpose: confirm the `BeadingFactoryParams`
  field set the pipeline currently fills in (the new fields must be
  added there too).
- `crates/slicer-core/src/arachne/generate_toolpaths.rs:880-895` —
  `generate_toolpaths` return type — purpose: confirm
  `Vec<VariableWidthLines>` is the bucket shape (no change needed in
  this packet, but G12 will touch this).
- `crates/slicer-core/src/arachne/remove_small.rs` (~150 lines) — purpose:
  confirm `remove_small_lines` still calls `simplify_toolpaths` in the
  canonical post-process order (no change needed).
- `crates/slicer-core/tests/beading/redistribute.rs` (~150 lines) —
  purpose: confirm the existing `compute` test set the new
  `optimal_bead_count` / `get_transition_thickness` / `optimal_thickness`
  must preserve.
- `crates/slicer-runtime/tests/fixtures/arachne_parity/mod.rs:135-158` —
  `simplify_input_intersection_distance_gate` fixture — purpose:
  confirm the f32 mm coordinates the G20 test feeds in.
- `docs/15_config_keys_reference.md` — read only the `min_bead_width`
  and `optimal_width` entries — purpose: confirm the default values the
  threshold formulas consume.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/` — delegate parity checks; never load directly.
- `target/`, `Cargo.lock`, generated code under `wit-guest/` — never load.
- `crates/slicer-runtime/src/run.rs`, `crates/slicer-scheduler/` —
  outside the G15/G20 change surface.
- `modules/core-modules/arachne-perimeters/src/lib.rs` — the G15 factory
  plumbing happens in `BeadingFactoryParams` (slicer-core), not in the
  module. The `arachne_params_from_config` function in the module is
  NOT in this packet's scope (the thresholds are derived in the
  factory, not at the module call site).
- `crates/slicer-core/src/arachne/pipeline.rs` (full file) — only the
  `to_beading_factory_params` body is in read-only context; no edits
  in this packet (G12 will add the region-order pass here).

## Expected Sub-Agent Dispatches

- "Run `cargo test -p slicer-runtime --test arachne_parity_round2 --
  arachne_parity_beading_split_middle_threshold_exposed --exact`; return
  FACT pass/fail or SNIPPETS (fail with assertion + ≤20 lines)" —
  purpose: validate AC-1 + AC-2.
- "Run `cargo test -p slicer-core --test beading_distributed -- distributed_optimal_bead_count_uses_split_middle_threshold --exact`;
  FACT pass/fail" — purpose: validate AC-3.
- "Run `cargo test -p slicer-core --test beading_redistribute -- redistribute_optimal_bead_count_consults_split_middle --exact`;
  FACT pass/fail" — purpose: validate AC-4.
- "Run `cargo test -p slicer-core --test beading_factory -- beading_factory_passes_split_middle_thresholds --exact`;
  FACT pass/fail" — purpose: validate AC-5.
- "Run `cargo test -p slicer-runtime --test arachne_parity_round2 --
  arachne_parity_simplify_intersection_distance_gate_present --exact`;
  FACT pass/fail or SNIPPETS (fail with assertion + ≤20 lines)" —
  purpose: validate AC-6.
- "Run `cargo test -p slicer-core --features host-algos --test arachne_simplify_intersection_distance_gate_tdd -- simplify_intersection_distance_gate_preserves_junction --exact`; FACT pass/fail" — purpose: validate AC-7.
- "Run `cargo test -p slicer-core --features host-algos --test arachne_simplify_intersection_distance_gate_tdd -- simplify_junction_replacement_moves_to_intersection --exact`; FACT pass/fail" — purpose: validate AC-8.
- "Run `cargo test -p slicer-core --features host-algos --test arachne_simplify_intersection_distance_gate_tdd -- simplify_distance_gated_uses_shoelace_height_2 --exact`; FACT pass/fail" — purpose: validate AC-9.
- "Run `cargo test -p slicer-core --features host-algos --test arachne_simplify_intersection_distance_gate_tdd -- simplify_degenerate_two_junctions_unchanged --exact`; FACT pass/fail" — purpose: validate AC-N3.
- "Run `cargo test -p slicer-core --features host-algos --test arachne_simplify_intersection_distance_gate_tdd -- simplify_closed_line_minimum_size_preserved --exact`; FACT pass/fail" — purpose: validate AC-N4.
- "Run `cargo test -p slicer-runtime --test arachne_parity`; return
  FACT pass/fail or SNIPPETS (fail with assertion + ≤20 lines)" —
  purpose: validate AC-10.
- "Run `cargo xtask build-guests --check`; return FACT clean / STALE" —
  purpose: confirm the beading trait-surface extension does not break
  guest builds.
- "Confirm the `BeadingStrategy` trait remains object-safe after
  adding `get_split_middle_threshold` and `get_add_middle_threshold`;
  return FACT (yes/no) with ≤ 5 lines of evidence" — purpose: AC-1's
  object-safety pre-flight check.
- "Delegate OrcaSlicer `ExtrusionLine.cpp:56-243` simplify walk;
  return SUMMARY (≤200 words) + at most two 30-line SNIPPETs (the tier-3
  block at `:162-220` + the `dist_greater` lambda at `:180-188`)" —
  purpose: arm Step 4's restructure with the canonical reference.

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

## Context Cost Estimate

- Aggregate: M (6 implementation steps × S, plus 1 final verification step = M).
- Largest single step: M (Step 4 — the simplify restructure with
  `previous_previous` tracking + tier-3 port + 2 new helpers + Shoelace
  formula).
- Highest-risk dispatch: the AC-10 regression sweep after Step 5. The
  `0.99/0.99` threshold saturation (see Risks) makes bead-count fixture
  shifts likely. Required return format: per-fixture pass/fail with the
  actual-vs-expected bead counts on any fail — never a blanket "re-record
  and move on".
- Second-highest-risk dispatch: the simplify restructure's Shoelace
  `height_2` change (AC-9). Existing tests may regress; required return
  format: per-test pass/fail with assertion line on any fail.

## Open Questions

- None `[BLOCK]`. The packet is implementation-ready.
- `[FWD]` Should the `is_closed = true` OrcaSlicer special case
  (`spill_over` at `ExtrusionLine.cpp:121-123`, the mid-loop ≤3 guard at
  `:114-118`, and the `front().p = back().p` copy at `:230-238`) be ported
  in a follow-up packet? The G20 test uses `is_closed = false` and AC-N4
  only checks the minimum-size guard is preserved. Full closed-line parity
  is not required for any RED test here but is needed for full OrcaSlicer
  parity. The implementer may resolve mid-flight (the change surface is
  local to `simplify.rs`).
- `[FWD]` Should PnP's `min_bead_width` **config default** (currently
  `4000`, i.e. equal to `optimal_width`) be lowered to match OrcaSlicer's
  convention (min bead width materially below the perimeter width)? At the
  present default both thresholds saturate at the `0.99` clamp ceiling (see
  Risks), which is a faithful evaluation of the OrcaSlicer formula on an
  unrepresentative input. Changing the config default is a **separate
  packet** — it is a user-facing behavior change and is explicitly out of
  scope here.
- `[FWD]` Should `BeadingFactoryParams` gain a Flow-derived
  `external_perimeter_extrusion_width` / `perimeter_extrusion_width` pair so
  the clamp inputs match `WallToolPaths.cpp:619-640` exactly (Orca passes
  both widths through `Flow::rounded_rectangle_extrusion_width_from_spacing`
  first)? This packet uses `preferred_bead_width_outer` and `optimal_width`
  as the nearest available analogues and records the residual as D-155.
  Closing it fully likely requires extending packet 150's Flow-spacing work
  into the beading factory.
