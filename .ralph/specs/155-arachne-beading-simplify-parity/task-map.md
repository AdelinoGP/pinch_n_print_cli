# Task Map: 155-arachne-beading-simplify-parity

## Backlog mapping

This packet's `task_ids` is `none` (declared in `packet.spec.md`
frontmatter). The backlog source is the round-3 Arachne parity audit
appended to `docs/18_arachne_parity_audit.md` (commit `54536d57`).
The packet covers two gaps:

- **G15** — `BeadingStrategy::getSplitMiddleThreshold` (Data Model) —
  `docs/18_arachne_parity_audit.md` lines 324-353 + 421-428
- **G20** — `ExtrusionLine::simplify` `dist_greater` intersection-distance
  gate (Algorithm) — `docs/18_arachne_parity_audit.md` lines 355-386 +
  429-434

Neither gap has a `docs/07` task ID — they were surfaced by the
round-3 orchestrator audit (the `arachne_parity_round2.rs` test file
at `crates/slicer-runtime/tests/` is the only trace). The gaps are
not tracked in `docs/07_implementation_status.md` (the audit doc
is the authoritative source).

## Step ↔ gap mapping

| Step | Gap(s) closed | Verification command |
| --- | --- | --- |
| Step 1 (Extend BeadingStrategy trait + forwarding in all 4 decorators) | G15 (trait surface change) | `cargo check --workspace --all-targets` (a required trait method with no default fails to compile unless all 5 implementors supply it — the compiler IS the gate here) |
| Step 2 (Port DistributedBeadingStrategy fields + overrides + 2 impls) | G15 (Distributed parity-based formula) | `cargo test -p slicer-core --test beading_distributed -- distributed_optimal_bead_count_uses_split_middle_threshold --exact` |
| Step 3 (Port RedistributeBeadingStrategy 3 methods) | G15 (Redistribute three-method port) | `cargo test -p slicer-core --test beading_redistribute -- redistribute_optimal_bead_count_consults_split_middle --exact` + `redistribute_outer_consistent --exact` (AC-N2) |
| Step 4 (Restructure simplify_distance_gated + 2 new helpers + Shoelace) | G20 (intersection-distance gate + junction-replacement else-branch + Shoelace height_2) | `cargo test -p slicer-core --features host-algos --test arachne_simplify_intersection_distance_gate_tdd -- simplify_intersection_distance_gate_preserves_junction --exact simplify_junction_replacement_moves_to_intersection --exact simplify_distance_gated_uses_shoelace_height_2 --exact simplify_degenerate_two_junctions_unchanged --exact simplify_closed_line_minimum_size_preserved --exact` |
| Step 5 (Thread thresholds through BeadingFactoryParams + factory) | G15 (factory plumbing + clamp) | `cargo test -p slicer-core --test beading_factory -- beading_factory_passes_split_middle_thresholds --exact beading_factory_threshold_propagates_through_full_stack --exact beading_factory_threshold_clamp_bounds_are_canonical --exact` |
| Step 6 (Rewrite the two RED test bodies + verify AC-10 regression lock) | G15 + G20 (close RED tests, regression lock) | `cargo test -p slicer-runtime --test arachne_parity_round2 -- arachne_parity_beading_split_middle_threshold_exposed arachne_parity_simplify_intersection_distance_gate_present` + `cargo test -p slicer-runtime --test arachne_parity` |
| Step 7 (Doc updates + final gates) | G15 + G20 (close documentation) | doc greps + `cargo xtask build-guests --check` + `cargo clippy --workspace --all-targets -- -D warnings` |

## Cross-packet relationships

- **Depends on:** none (packet A is the first of the two Arachne
  parity-fix packets).
- **Unblocks:** `156-arachne-region-order` (packet B) — packet B
  records in its prerequisites that packet A should be
  `status: implemented` first to avoid workspace contention,
  but the two packets are technically independent.
- **Adjacent packets (not in this packet's scope):** packets
  148/149/150/151/152/153/154 (the earlier M2 + perimeter-parity
  closure chain). They are all `status: implemented` per
  `docs/07_implementation_status.md:312-330` and their changes
  are already in the tree at `parity/arachne @ 34ce576e`.
- **Audit doc updates (this packet's responsibility):**
  - Mark G15 + G20 closed in the Gap summary table (line 292-293).
  - Update the G15 + G20 detailed-gap "PnP status" entries
    (line 334 + 366) to "closed (this packet)".
  - Drop the G15 + G20 rows from the "Open gaps" list
    (line 48-187).

## Test fixtures (this packet's responsibility)

**Two** test bodies change in this packet, both in
`crates/slicer-runtime/tests/arachne_parity_round2.rs`, and both are
declared exceptions to the audit's "closing a gap turns its test green
with no rewrite" promise:

1. **G15** (`:134-160`) — the `assert!(false)` body fires unconditionally
   and must be replaced with real calls to
   `stack.get_split_middle_threshold()` / `get_add_middle_threshold()`
   (**no arguments**) on a fully-decorated stack, asserting they equal the
   factory-computed value. Sanctioned by the test's own doc note at
   `:120-132`.
2. **G20** (`:192`) — the `simplify_toolpaths` **parameters** must change
   from `smallest_line_segment_squared = 0.0` /
   `allowed_error_distance_squared = f64::INFINITY` to `1e-3` / `1.0`. With
   the threshold at 0 the tier-3 gate (`ExtrusionLine.cpp:162-164`) reduces
   to `length2 < 0` — unsatisfiable for any input, since `length2` is a
   squared norm — so the intersection/`dist_greater` path (`:166-220`) is
   dead and the original parameters could not exercise the gate they name.
   The assertion is **strengthened** (`kept >= 4` → `kept == 4`), never
   weakened.

## OrcaSlicer parity surface

Line numbers below were re-resolved against the real `OrcaSlicerDocumented/`
tree on 2026-07-14. **The original draft's refs were wrong in every case**
(off by 20–500 lines); an implementer who follows a stale ref will read the
wrong function. The canonical list lives in `packet.spec.md` §OrcaSlicer
Reference Obligations.

- `BeadingStrategy.hpp:166` — `getSplitMiddleThreshold()`, **no arguments**.
  (`getAddMiddleThreshold()` does **not exist** in OrcaSlicer — PnP's is a
  net-new symbol; D-155.)
- `BeadingStrategy.hpp:74-78, 182, 186` — thresholds are required ctor
  params + protected fields; no sentinel/default.
- `BeadingStrategy.cpp:90-102` — parity-based threshold selection.
- `DistributedBeadingStrategy.cpp:132-144` — integer-truncating
  `minimum_line_width` selection.
- `RedistributeBeadingStrategy.cpp:50-85` — the three ported methods.
- Decorator ctors (`RedistributeBeadingStrategy.cpp:39-48`,
  `WideningBeadingStrategy.cpp:39-44`,
  `OuterWallInsetBeadingStrategy.cpp:39-43`,
  `LimitedBeadingStrategy.cpp:54-58`) — all copy-construct
  `BeadingStrategy(*parent)`; the reason PnP's decorators must forward.
- `WallToolPaths.cpp:619-640` — clamp formulas (two distinct widths).
- `ExtrusionLine.cpp:56-243` — the full simplify walk; `dist_greater`
  (3 args) at `:180-188`; Shoelace `height_2` at `:151`.

All OrcaSlicer reads MUST be delegated to a sub-agent per the
`orca-delegation` snippet. The implementer must NEVER load
`OrcaSlicerDocumented/` into their own context.
