# Requirements: 155-arachne-beading-simplify-parity

## Packet Metadata

- Grouped task IDs:
  - `none` (audit-driven; backlog source is `docs/18_arachne_parity_audit.md`,
    following the precedent of packets 148/149/150/151/152/153/154 which also
    declare `task_ids: none`)
- Backlog source: `docs/18_arachne_parity_audit.md`
- Packet status: `draft`
- Aggregate context cost: `M`

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

## In Scope

- Add `get_split_middle_threshold(&self) -> f64` and
  `get_add_middle_threshold(&self) -> f64` to the `BeadingStrategy` trait
  in `crates/slicer-core/src/beading/mod.rs` as **required methods with no
  default impl** and **no arguments** (Orca: `double getSplitMiddleThreshold()
  const`, `BeadingStrategy.hpp:166`). Trait remains object-safe.
- Implement both on **all five** concrete strategies. `Distributed` returns
  two stored fields; `Redistribute`, `Widening`, `OuterWallInset` and
  `Limited` **forward to `self.parent`**. Forwarding is mandatory — it is
  PnP's equivalent of OrcaSlicer's `BeadingStrategy(*parent)`
  copy-construction in all four decorator ctors, which is what makes the
  threshold observable at the top of a factory-built stack. (A default impl
  would be silently picked up at the `Limited` layer and shadow the real
  value.)
- Add `wall_split_middle_threshold: f64` and `wall_add_middle_threshold: f64`
  fields to `DistributedBeadingStrategy` (`distributed.rs`). Port
  `optimal_bead_count` from `DistributedBeadingStrategy.cpp:132-144` —
  **integer truncation** for `naive_count` (replacing today's `.round()` at
  `distributed.rs:177-179`), parity-based `minimum_line_width`, and a `>=`
  comparison. Port `get_transition_thickness` using the same parity-based
  selection (`BeadingStrategy.cpp:90-102`).
- Add the same two `f64` ratio fields to `BeadingFactoryParams`
  (`factory.rs`). Compute them in `create_stack` from the **real live field
  names** (`factory.rs:144-160` — there is **no `min_bead_width` field**; the
  `min_bead_width` config key surfaces as `min_output_width`) via OrcaSlicer's
  clamp formulas (`WallToolPaths.cpp:619-640`):
  `split = clamp(2 * min_output_width / preferred_bead_width_outer - 1, 0.01, 0.99)`,
  `add = clamp(min_output_width / optimal_width, 0.01, 0.99)`. Pass both to
  `DistributedBeadingStrategy::new`. Clamp bounds are OrcaSlicer's literal
  constants and must not be altered.
- Port `RedistributeBeadingStrategy::optimal_thickness`,
  `get_transition_thickness`, and `optimal_bead_count` from
  `RedistributeBeadingStrategy.cpp:50-85`. Only `get_transition_thickness`'s
  **`case 1`** branch and `optimal_bead_count`'s `<= 2W` branch consult
  `parent.get_split_middle_threshold()`; `case 0` returns
  `minimum_variable_line_ratio * optimal_width_outer`. The `compute` method
  is UNCHANGED.
- Restructure `simplify_distance_gated` in
  `crates/slicer-core/src/arachne/simplify.rs` to track `previous` and
  `previous_previous` as `ExtrusionJunction` **value copies**
  (`ExtrusionLine.cpp:75,79`). **`use_distance_gates` (`simplify.rs:103-104`)
  is NOT changed** — see the G20 test-parameter correction below.
- Add two new helpers in `simplify.rs`:
  `line_intersection_infinite(a, b, c, d) -> Option<(f64, f64)>` (the
  intersection of the infinite lines through a-b and c-d) and
  `dist_greater(p1, p2, threshold) -> bool` (**three** args — the
  overflow-avoiding component-wise fast-reject + squared-norm comparison,
  `ExtrusionLine.cpp:180-188`).
- Port the tier-3 special case from `ExtrusionLine.cpp:166-220`:
  `next_length2 > 4 * smallest_line_segment_squared` → compute intersection
  → `dist_greater` reject path (`:189-200`) → else-branch replacement
  (`:201-217`: pop the previously-pushed junction, restore
  `previous = previous_previous`, push the intersection carrying **`curr`'s**
  width and `perimeter_index`).
- Replace the height calc at both gate sites with the OrcaSlicer Shoelace
  formula `height_2 = (area_removed_so_far)² / base_length_2`
  (`ExtrusionLine.cpp:151`), where `area_removed_so_far` is a per-iteration
  local (`accumulated_area_removed + negative_area_closing`, `:139`) and the
  **running accumulator is `accumulated_area_removed`** (declared `:104`,
  incremented `:131`, reset to `removed_area_next` at `:211` and `:223`).
- Correct the G20 RED test's `simplify_toolpaths` parameters
  (`arachne_parity_round2.rs:192`) from `smallest_line_segment_squared = 0.0`
  / `allowed_error_distance_squared = f64::INFINITY` to `1e-3` / `1.0`. At
  `smallest_line_segment_squared = 0` the tier-3 gate
  (`ExtrusionLine.cpp:162-164`) becomes `length2 < 0` — unsatisfiable for any
  input, since `length2` is a squared norm — so the entire intersection path
  is dead and the old parameters could not exercise the gate. The assertion is
  simultaneously **strengthened** from `kept >= 4` to `kept == 4`.
- Close gaps G15 and G20; update `docs/18_arachne_parity_audit.md`
  Gap summary table; add `D-155` (beading threshold parity) and `D-156`
  (simplify intersection gate) to `docs/DEVIATION_LOG.md`.
- Add `CONTEXT.md` glossary entries for *split-middle threshold* and
  *intersection-distance gate*.

## Out of Scope

- G12 (`WallToolPaths::getRegionOrder` + path-optimizer walk) — packet B
  (`156-arachne-region-order`); separated because it introduces a new
  `SparsePointGrid` utility and a path-optimizer concept not in this slice.
- G11 (concentric infill via Arachne) — pre-existing red in
  `arachne_parity.rs`; tracked separately per the audit doc.
- `is_closed = true` full OrcaSlicer parity (loop bounds, ≤3-junction
  guard, spill-over, front=back copy at `ExtrusionLine.cpp:87-97,211-218`).
  The simplify restructure in this packet preserves the existing PnP
  open-line behavior + the `n <= 2` early-return; closed-line special
  handling is deferred and tracked as an Open Question in `design.md`.
- New config keys `wall_split_middle_threshold` / `wall_add_middle_threshold`
  in `arachne-perimeters.toml`. The thresholds are internal Arachne
  parameters derived from already-registered `min_bead_width` and
  `optimal_width` keys (per the audit doc §"Porting reminders for the
  fixing agent (additive)").
- WIT contract changes (the trait surface change is in `slicer-core`, not
  on the WASM boundary). No `slicer-macros` / `slicer-schema` / `slicer-ir`
  IR changes.
- Host-side threshold pre-resolution (rejected per the packet-150 precedent
  in `docs/18_arachne_parity_audit.md` §"Round-3 test categorization"; the
  module computes them at `arachne_params_from_config` time, not the host).
- The `BeadingStrategyFactoryParams` rename to match OrcaSlicer's
  factory-internal name (cosmetic; deferred).

## Authoritative Docs

- `docs/18_arachne_parity_audit.md` — load only the G15 and G20
  detailed-gap sections (lines 324-388); the Gap summary table is the
  canonical entry point.
- `docs/08_coordinate_system.md` — load directly (short file); the
  `mm_to_units()` helper is used at the threshold-computation site.
- `docs/DEVIATION_LOG.md` — load only the D-105B/C/E entries (close
  precedents for trait-surface changes) and the new D-155/D-156 entries
  this packet adds.
- `docs/02_ir_schemas.md` — delegate a SUMMARY of the
  `ExtrusionJunction` / `Point3WithWidth` field set (the implementer
  only needs the f32-mm coordinate contract and `width` /
  `perimeter_index` for junction-replacement).
- `docs/15_config_keys_reference.md` — load the `min_bead_width` and
  `optimal_width` entries directly; thresholds are derived from these.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

> Line numbers re-resolved against the real `OrcaSlicerDocumented/` tree on
> 2026-07-14. The canonical list lives in `packet.spec.md` §OrcaSlicer
> Reference Obligations — this is a summary of it.

- `BeadingStrategy.hpp:166` — `double getSplitMiddleThreshold() const;` (**no arguments**). **`getAddMiddleThreshold()` does not exist in OrcaSlicer** — PnP's is a net-new symbol (D-155).
- `BeadingStrategy.hpp:74-78, 182, 186` — thresholds are required ctor params + protected fields; no sentinel. Justifies AC-1's "required, no default" trait methods.
- `BeadingStrategy.cpp:90-102` — base `getTransitionThickness` parity-based selection.
- `DistributedBeadingStrategy.cpp:132-144` — `getOptimalBeadCount`: integer truncation + parity-based `minimum_line_width` + `>=` (AC-3).
- `RedistributeBeadingStrategy.cpp:50-85` — the three overrides (AC-4). `parent->getSplitMiddleThreshold()` is consulted in `getTransitionThickness`'s **`case 1`** and in `getOptimalBeadCount`'s `<= 2W` branch.
- Decorator ctors — `RedistributeBeadingStrategy.cpp:39-48`, `WideningBeadingStrategy.cpp:39-44`, `OuterWallInsetBeadingStrategy.cpp:39-43`, `LimitedBeadingStrategy.cpp:54-58` — all copy-construct `BeadingStrategy(*parent)`; the mechanism AC-1's forwarding reproduces.
- `BeadingStrategyFactory.cpp:69-96` — stack order (matches PnP's `factory.rs:187-228`).
- `WallToolPaths.cpp:619-640` — threshold clamp formulas; split ÷ external perimeter width, add ÷ inner perimeter width, both Flow-converted (AC-5, D-155 residual).
- `ExtrusionLine.cpp:56-243` — the full `simplify` impl (AC-6/7/8/9). Landmarks: early-return `:63-65`; accumulator `:104`; `length2` `:133`; Shoelace `height_2` **`:151`**; tier-3 gate **`:162-164`**; special case `:166`; `dist_greater` (3 args) **`:180-188`**; replacement else-branch **`:201-217`**; accumulator resets **`:211`, `:223`**.

## Acceptance Summary

- Positive cases: `AC-1` (G15 trait surface — required methods + mandatory
  decorator forwarding), `AC-2` (threshold observable at the top of a
  fully-decorated stack), `AC-3` (G15 `Distributed::optimal_bead_count`
  truncation + parity selection, with a falsifying case that discriminates
  it from the old `.round()`), `AC-4` (G15 `Redistribute` three-method port),
  `AC-5` (G15 factory clamp), `AC-6` (G20 RED test reaches the gate after its
  parameters are corrected), `AC-7` (G20 tier-3 `dist_greater` preserves
  junction), `AC-8` (G20 junction-replacement else-branch), `AC-9` (G20
  Shoelace `height_2` + threaded accumulator), `AC-10` (14 locks + G3/G10
  stay green, with fixture shifts adjudicated not rubber-stamped).
  Refinements: AC-3's lock is parameterised across threshold values, and
  **must include an input where the new and old impls disagree** (thresholds
  `0.99`, `thickness = 7500`, `optimal_width = 4000` → new `1`, old `2`).
  AC-5's shipped-default result is `0.99 / 0.99` — both clamps saturate at
  the ceiling because PnP's `min_bead_width` default equals `optimal_width`;
  this is a faithful evaluation on an unrepresentative input and is recorded
  in D-155.
- Negative cases: `AC-N1` (clamp bounds `[0.01, 0.99]` are OrcaSlicer's
  literal constants and are honored at both ends), `AC-N2` (existing
  `redistribute::compute` tests still pass), `AC-N3` (2-junction open-line
  early-return preserved), `AC-N4` (3-junction closed-line minimum-size
  guard preserved).
- Cross-packet impact: does not unblock any other packet. Packet B
  (`156-arachne-region-order`) has no read dependency on this packet.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-runtime --test arachne_parity_round2 -- arachne_parity_beading_split_middle_threshold_exposed --exact` | AC-2 (G15 flips green; threshold observable on the `Limited` top of a fully-decorated stack) | FACT pass/fail; SNIPPETS ≤20 on fail |
| `cargo test -p slicer-core --test beading_factory -- beading_factory_threshold_propagates_through_full_stack --exact` | AC-1 (decorator forwarding through all 4 layers) | FACT pass/fail |
| `cargo test -p slicer-core --test beading_distributed -- distributed_optimal_bead_count_uses_split_middle_threshold --exact` | AC-3 (G15 Distributed truncation + parity selection) | FACT pass/fail |
| `cargo test -p slicer-core --test beading_redistribute -- redistribute_optimal_bead_count_consults_split_middle --exact` | AC-4 (G15 Redistribute port) | FACT pass/fail |
| `cargo test -p slicer-core --test beading_factory -- beading_factory_passes_split_middle_thresholds --exact` | AC-5 (G15 factory clamp) | FACT pass/fail |
| `cargo test -p slicer-core --test beading_factory -- beading_factory_threshold_clamp_bounds_are_canonical --exact` | AC-N1 (clamp bounds honored at both ends) | FACT pass/fail |
| `cargo test -p slicer-core --test beading_redistribute -- redistribute_compute --exact` | AC-N2 (no compute regression) | FACT pass/fail |
| `cargo test -p slicer-runtime --test arachne_parity_round2 -- arachne_parity_simplify_intersection_distance_gate_present --exact` | AC-6 (G20 flips green) | FACT pass/fail; SNIPPETS ≤20 on fail |
| `cargo test -p slicer-core --test arachne_simplify_intersection_distance_gate_tdd -- simplify_intersection_distance_gate_preserves_junction --exact` | AC-7 (G20 dist_greater gate) | FACT pass/fail |
| `cargo test -p slicer-core --test arachne_simplify_intersection_distance_gate_tdd -- simplify_junction_replacement_moves_to_intersection --exact` | AC-8 (G20 replacement else-branch) | FACT pass/fail |
| `cargo test -p slicer-core --test arachne_simplify_intersection_distance_gate_tdd -- simplify_distance_gated_uses_shoelace_height_2 --exact` | AC-9 (G20 Shoelace formula) | FACT pass/fail |
| `cargo test -p slicer-core --test arachne_simplify_intersection_distance_gate_tdd -- simplify_degenerate_two_junctions_unchanged --exact` | AC-N3 (degenerate input) | FACT pass/fail |
| `cargo test -p slicer-core --test arachne_simplify_intersection_distance_gate_tdd -- simplify_closed_line_minimum_size_preserved --exact` | AC-N4 (closed-line guard) | FACT pass/fail |
| `cargo test -p slicer-runtime --test arachne_parity` | AC-10 (14 round-1 locks stay green) | FACT pass/fail |
| `cargo test -p slicer-core` | full slicer-core test sweep | FACT pass/fail |
| `cargo check --workspace --all-targets` | compiles incl. test/bench targets | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail |
| `cargo xtask build-guests --check` | guest WASM fresh after beading/factory edits | FACT clean/STALE |

## Step Completion Expectations

- Cross-step invariant: no step may regress the 14 round-1
  `arachne_parity.rs` locks or the `factory_orca_reference.json` golden
  (AC-10). Step 1 (trait surface change) and Step 4 (simplify
  restructure) are the steps most likely to cause collateral damage —
  verify AC-10 immediately after each, not only at packet close.
- Ordering rationale: the trait-surface step (Step 1) precedes every
  other G15 step (it unblocks them). The simplify restructure (Step 4)
  is independent of G15 and may land in either order relative to G15
  steps 2/3/5. Step 6 (AC-10 verification) must run last.
- Shared scratch state: none. The beading and simplify changes touch
  different files; no cross-step state.

## Context Discipline Notes

- Large files in the read-only path that MUST be ranged or delegated:
  `crates/slicer-core/src/beading/mod.rs` (~150 lines; load directly,
  but only the trait def + new methods), `crates/slicer-core/src/arachne/simplify.rs`
  (~350 lines; load directly, the whole file is in-scope for Step 4),
  `crates/slicer-core/src/arachne/pipeline.rs` (~470 lines; read only
  the `to_beading_factory_params` fn at `:269-292` for Step 5's
  factory-plumbing pass).
- Likely temptation reads: the `BeadingFactoryParams` default in
  `factory.rs:144-161` is NOT changing in this packet (the new fields
  carry the same 4000/4000-unit defaults the existing fields use; the
  threshold defaults are derived from those). Skip re-reading the
  existing default body unless Step 5's FACT dispatch fails.
- Heaviest dispatch return-format hint: the OrcaSlicer
  `ExtrusionLine.cpp:56-243` simplify walk must be returned as
  `SUMMARY` (≤200 words) plus at most two 30-line `SNIPPET`s (the
  tier-3 block at `:162-220` + the `dist_greater` lambda at `:180-188`) —
  never the full file.
- **Line-reference hygiene (mandatory):** every OrcaSlicer line number in
  this packet was re-resolved against the real tree on 2026-07-14 after an
  audit found the original draft's refs wrong by 20–500 lines in every
  single case. If a delegated read does not find the claimed content at the
  claimed line, **stop and re-resolve** — do not proceed from a guess, and
  do not "fix" the ref by trusting memory.
