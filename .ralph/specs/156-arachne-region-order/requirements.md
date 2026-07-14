# Requirements: 156-arachne-region-order

## Packet Metadata

- Grouped task IDs:
  - `none` (audit-driven; backlog source is `docs/18_arachne_parity_audit.md`,
    following the precedent of packets 148/149/150/151/152/153/154 which
    also declare `task_ids: none`)
- Backlog source: `docs/18_arachne_parity_audit.md`
- Packet status: `draft`
- Aggregate context cost: `M` (no step is L — the original L-rated Step 4
  was split into 4a (pipeline) and 4b (module); no extended-band run is
  required)

## Problem Statement

The Arachne beading-strategy pipeline ends with
`run_arachne_pipeline` returning a flat `Vec<ExtrusionLine>` in
source-polygon / inset-index order
(`crates/slicer-core/src/arachne/pipeline.rs:382-383`:
`let lines: Vec<ExtrusionLine> = buckets.into_iter().flatten().collect();`).
OrcaSlicer pairs a `getRegionOrder` constraint set
(`OrcaSlicerDocumented/src/libslic3r/Arachne/WallToolPaths.cpp:973-1058`)
with a greedy topological walk in the perimeter generator
(`OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:2781-2857`)
that reorders the emitted extrusions so an inner (odd) region follows
its enclosing even region. PnP's source-order flatten means an inner
region can be emitted before its enclosing outer region — the
`arachne_parity_wall_region_order_odd_after_enclosing` RED test in
`arachne_parity_round2.rs:41-106` locks the gap.

This is a one-gap packet because the SparsePointGrid utility and the
topological-walk concept are net-new to PnP (no equivalent exists in
`crates/slicer-core` or `crates/slicer-helpers` today) and they pull
in a path-optimizer-flavored concept that warrants its own review pass
distinct from the beading/simplify changes in packet A. The
`arachne-perimeters` module's existing `sort_by_key(perimeter_index)`
at `modules/core-modules/arachne-perimeters/src/lib.rs:614` is left
in place — it sorts the per-region `Vec<WallLoop>` after the module
converts the pipeline's `Vec<ExtrusionLine>` to `WallLoop`s; the G12
fix operates one level earlier on the `Vec<ExtrusionLine>`.

The user explicitly chose to scope this as a separate packet (not
fold it into packet A) so the `arachne-perimeters` module's
wall-tree ownership (ADR-0011) is not disturbed and so the
topological-walk port gets its own acceptance ceremony. The
user also chose to port the full `getRegionOrder` + topological
walk faithfully from OrcaSlicer (not the simpler
"sort by `(is_odd, inset_idx)`" shortcut) so the constraint set can
be reused later by a path-optimizer extension that
`path-optimization-default` may grow.

## In Scope

- Add a new module `crates/slicer-core/src/arachne/region_order.rs`
  containing:
  - `pub fn get_region_order(input: &[ExtrusionLine], outer_to_inner: bool) -> Vec<(usize, usize)>`
    — a faithful Rust port of `WallToolPaths::getRegionOrder`
    (`WallToolPaths.cpp:973-1058`; decl at `WallToolPaths.hpp:211`).
    Returns `(before_idx, after_idx)` index pairs meaning "emit
    `before_idx` before `after_idx`". `searching_radius = max_line_w * 1.9`;
    returns empty when `max_line_w == 0`. The constraint predicate
    (`:1044-1054`) has a **direction-independent `is_odd` branch** — an odd
    wall is always preceded by its enclosing lower-`inset_idx` even wall,
    regardless of `outer_to_inner`; only the even/even branch flips on the
    flag.
  - `pub fn topological_walk(lines: &[ExtrusionLine], constraints: &[(usize, usize)]) -> Vec<usize>`
    — a faithful Rust port of `PerimeterGenerator.cpp:2781-2857`. Builds
    `blocked`/`blocking` adjacency, starts the cursor at **the first junction
    of the first input line** (`(0,0)` only when the input is empty,
    `:2798-2799`), iterates **open lines before closed ones** (`:2815-2818`),
    evaluates distance per-candidate within that order, and breaks ties by
    `original_index` ascending (a PnP determinism addition). Returns a
    permutation of `0..lines.len()`.
  - `pub fn reorder_by_region_order(lines: &mut Vec<ExtrusionLine>, outer_to_inner: bool)`
    — convenience wrapper: calls `get_region_order` then
    `topological_walk` then permutes `lines` in place.
- Add a new utility `crates/slicer-core/src/arachne/sparse_point_grid.rs`
  containing:
  - `pub struct SparsePointGrid<T, F>` parameterised by a `T: Clone`
    payload and a `F: Fn(&T) -> Point2` locator. Uses a
    `HashMap<(i64, i64), Vec<T>>` cell map. **The cell size is the
    `searching_radius` itself**, stored verbatim — OrcaSlicer passes
    `searching_radius` straight into the ctor (`WallToolPaths.cpp:1022`;
    `SparsePointGrid.hpp:31-38` → `SparseGrid.hpp:106`). **There is no
    `/ sqrt(2)` derivation anywhere in OrcaSlicer.**
  - `pub fn insert(&mut self, item: T)` — adds the item to the cell
    containing its `locator(item)`.
  - `pub fn get_nearby(&self, query: Point2, radius: f32) -> Vec<T>`
    — scans every cell the query circle can touch, then filters by exact
    Euclidean distance (`SparseGrid.hpp:137-146`). Guards `cell_size == 0`.
- Wire the new `reorder_by_region_order` pass into
  `crates/slicer-core/src/arachne/pipeline.rs` between the
  `generate_toolpaths` flatten at `:383` and `stitch_extrusions` at
  `:390`. Add `ArachneParams::outer_to_inner: bool`, **default `false`** —
  OrcaSlicer's `wall_sequence` defaults to `InnerOuter`
  (`PrintConfig.cpp:2084`), which yields `is_outer_wall_first == false`
  (`PerimeterGenerator.cpp:2761-2766`). `outer_to_inner == true` means
  **outer walls first** and corresponds to `wall_sequence == "OuterInner"`.
  The module's `arachne_params_from_config` (which already reads
  `wall_sequence` at `arachne-perimeters/src/lib.rs:269-272`) derives it as
  `(ws == "OuterInner") || (ws == "InnerOuterInner" && !is_initial_layer)`.
- Register both new sub-modules in `crates/slicer-core/src/arachne/mod.rs`.
- Add 8 unit tests across two new **top-level** test files:
  `crates/slicer-core/tests/region_order_tdd.rs` (AC-2, AC-4, AC-N1, AC-N2,
  AC-N3, AC-N5) and `crates/slicer-core/tests/sparse_point_grid_tdd.rs`
  (AC-3, AC-N4). **No `Cargo.toml` `[[test]]` entry is required** — top-level
  `tests/*.rs` files are auto-discovered by Cargo (as every existing
  `crates/slicer-core/tests/arachne_*.rs` file demonstrates). There is no
  `tests/arachne/` subdirectory; earlier drafts named one.
- Leave the pre-existing `slicer_core::perimeter_utils::wall_sequence_reorder`
  (`perimeter_utils.rs:723`, tested by `tests/wall_sequence_reorder_tdd.rs`)
  and the module's `sort_by_key(perimeter_index)` (`lib.rs:614`) **unchanged**,
  and verify the new pass does not double-apply a direction flip with them
  (AC-8).
- Close gap G12; update `docs/18_arachne_parity_audit.md` Gap
  summary table; add `D-157` (region-order port) to
  `docs/DEVIATION_LOG.md`; add *region order* and *SparsePointGrid*
  glossary entries to `CONTEXT.md`.

## Out of Scope

- G15 (BeadingStrategy::getSplitMiddleThreshold) and G20 (simplify
  intersection gate) — packet A (`155-arachne-beading-simplify-parity`).
- G11 (concentric infill via Arachne) — pre-existing red in
  `arachne_parity.rs`; tracked separately per the audit doc.
- Path-optimizer extension to `path-optimization-default`. The
  `reorder_by_region_order` helper is consumed only by
  `run_arachne_pipeline` in this packet; the `OrderedEntityView` and
  `PerimeterRegionView` types do not gain `inset_idx` / `is_odd`
  fields. A future packet can plumb the constraint set into
  `path-optimization-default`'s nearest-neighbor walk.
- `arachne-perimeters` module changes beyond reading
  `wall_sequence` and passing it through. The module's
  `sort_by_key(perimeter_index)` stays.
- WIT/IR/manifest/scheduler changes. The `outer_to_inner` field
  is derived from the already-registered `wall_sequence` config
  key; no new key.
- `SparsePointGrid` generalisation (templated on `T` and `Locator`).
  The `SparsePointGrid` in this packet uses a concrete
  `ExtrusionJunction` payload + a `Point2` locator; the generic
  `T, F` API is documented in the module's doc comment but only
  one monomorphisation is used.
- Special-casing the initial layer inside `get_region_order` itself.
  `WallToolPaths::getRegionOrder` does NOT special-case layer 0 — the
  constraint emission is identical on every layer, and the PnP port follows
  suit. **However, the initial layer IS special-cased one level up**, in the
  `outer_to_inner` derivation: OrcaSlicer disables `InnerOuterInner`'s
  outer-first behavior on layer 0 (`PerimeterGenerator.cpp:2764-2766`), which
  AC-6's formula reproduces via the module's existing `is_initial_layer` flag.
- Changing `perimeter_utils::wall_sequence_reorder` (`:723`) or the
  `arachne-perimeters` module's `sort_by_key(perimeter_index)` (`lib.rs:614`).
  Both stay as they are; AC-8 only requires the implementer to *verify* the
  new pass composes with them rather than double-flipping direction.

## Authoritative Docs

- `docs/18_arachne_parity_audit.md` — load only the G12 detailed-gap
  section (lines 296-322) and the "Round-3 fixture additions"
  section (lines 406-413).
- `docs/08_coordinate_system.md` — load directly (short file); the
  G12 fixture and the `SparsePointGrid` operate in mm-space.
- `docs/04_host_scheduler.md` — delegate a SUMMARY of the
  `wall_generator` dispatch section; the G12 fix does NOT touch the
  scheduler (the fix is in `slicer-core::arachne`, not the
  scheduler's `dedup_same_claim_modules_with_wall_generator`).
- `docs/03_wit_and_manifest.md` — delegate a SUMMARY of the
  config-key schema; the `wall_sequence` key is already registered
  on `arachne-perimeters.toml` (per packet 151 closure).
- `docs/15_config_keys_reference.md` — load the `wall_sequence`
  entry directly; the module reads this key today and the
  `outer_to_inner` bool is derived from it.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

> Line numbers re-resolved against the real `OrcaSlicerDocumented/` tree on
> 2026-07-14. The canonical list lives in `packet.spec.md` §OrcaSlicer
> Reference Obligations — this is a summary. The earlier draft's refs
> (`WallToolPaths.cpp:809-893`, `WallToolPaths.hpp:104`,
> `PerimeterGenerator.cpp:2270-2360`) were **all wrong**, as were its claims
> of a `radius / sqrt(2)` cell size and a `Point::Zero()` initial cursor.

- `WallToolPaths.hpp:211` — `static ExtrusionLineSet getRegionOrder(const std::vector<ExtrusionLine*>& input, bool outer_to_inner);` (AC-2's signature).
- `WallToolPaths.cpp:973-1058` — the full `getRegionOrder` impl: `max_line_w` + zero-guard (`:996-1002`), `searching_radius = max_line_w * 1.9` (`:1019-1020`), grid built with **cell size = `searching_radius`** (`:1022`), constraint emission with the direction-independent `is_odd` branch (`:1044-1054`). Arbiter of AC-2 and AC-3.
- `SparsePointGrid.hpp:31-38, 44, 54-58` + `SparseGrid.hpp:106, 137-146` — the grid API: ctor stores `cell_size` verbatim (**no `/ sqrt(2)`**); `insert`; `getNearby` scans touched cells then filters by true distance. Arbiter of AC-3.
- `PerimeterGenerator.cpp:2781-2857` — the topological-walk consumer: `blocked`/`blocking` build (`:2782-2795`), initial cursor = first junction of the first line, `(0,0)` only if empty (`:2798-2799`), open-before-closed candidate iteration (`:2815-2818`), per-candidate distance evaluation (`:2820-2842`). Arbiter of AC-4.
- `PerimeterGenerator.cpp:2761-2766` + `PrintConfig.hpp:187-192` + `PrintConfig.cpp:2084` — `is_outer_wall_first` derivation, the `WallSequence` enum, and the `InnerOuter` default. Arbiter of AC-6's polarity (`outer_to_inner` defaults to **false**).

## Acceptance Summary

- Positive cases: `AC-1` (G12 fixture lock), `AC-2` (`getRegionOrder`
  emits the exact canonical constraint predicate, incl. the
  direction-independent `is_odd` branch), `AC-3` (`SparsePointGrid`
  correctness with cell size = `searching_radius`), `AC-4`
  (`topological_walk` — `blocked`/`blocking` adjacency, cursor starting at
  the first line's first junction, open-before-closed iteration), `AC-5`
  (pipeline integration point), `AC-6` (`outer_to_inner` derived from
  `wall_sequence` with the **correct polarity** — default `false`), `AC-7`
  (regression lock — permutation only, no drop/duplicate), `AC-8` (no
  double-application with the pre-existing
  `perimeter_utils::wall_sequence_reorder`). Refinements: AC-7's lock
  compares the sorted `Vec<u32>` of `inset_idx` values (a **multiset**, not a
  `BTreeSet` — a set would hide duplicates) plus the output length.
- Negative cases: `AC-N1` (empty input → empty output, zero
  constraints), `AC-N2` (single line → single-element output), `AC-N3`
  (no spatial adjacency → zero constraints; the walk falls back to the
  unconstrained greedy order **starting from the first input line**, not
  from the origin), `AC-N4` (single-point grid → self in own vicinity),
  `AC-N5` (`max_line_w == 0` → empty constraint set, no divide-by-zero on
  the cell size).
- Cross-packet impact: does not unblock any other packet. Packet A
  (`155-arachne-beading-simplify-parity`) has no read dependency on this
  packet; both are independent.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-runtime --test arachne_parity_round2 -- arachne_parity_wall_region_order_odd_after_enclosing --exact` | AC-1 + AC-5 + AC-6 (G12 flips green) | FACT pass/fail; SNIPPETS ≤20 on fail |
| `cargo test -p slicer-core --test region_order_tdd -- region_order_get_emits_adjacent_constraints --exact` | AC-2 (getRegionOrder constraints) | FACT pass/fail |
| `cargo test -p slicer-core --test sparse_point_grid_tdd -- sparse_point_grid_get_nearby_returns_only_nearby_points --exact` | AC-3 (grid correctness) | FACT pass/fail |
| `cargo test -p slicer-core --test region_order_tdd -- region_order_topological_walk_respects_constraints --exact` | AC-4 (topological walk) | FACT pass/fail |
| `cargo test -p slicer-core --test region_order_tdd -- region_order_empty_input_returns_empty --exact` | AC-N1 (empty) | FACT pass/fail |
| `cargo test -p slicer-core --test region_order_tdd -- region_order_single_line_preserved --exact` | AC-N2 (single line) | FACT pass/fail |
| `cargo test -p slicer-core --test region_order_tdd -- region_order_no_adjacency_falls_back_to_nearest_neighbor --exact` | AC-N3 (no adjacency) | FACT pass/fail |
| `cargo test -p slicer-core --test sparse_point_grid_tdd -- sparse_point_grid_single_insert_get_nearby_self --exact` | AC-N4 (single insert) | FACT pass/fail |
| `cargo test -p slicer-core --test region_order_tdd -- region_order_zero_max_line_width_returns_no_constraints --exact` | AC-N5 (zero max_line_w guard) | FACT pass/fail |
| `cargo test -p slicer-core --test wall_sequence_reorder_tdd` | AC-8 (pre-existing reorder stays green; no double flip) | FACT pass/fail |
| `cargo test -p slicer-runtime --test arachne_parity` | AC-7 (14 round-1 locks stay green) | FACT pass/fail; SNIPPETS ≤20 on fail |
| `cargo test -p slicer-core` | full slicer-core test sweep | FACT pass/fail |
| `cargo check --workspace --all-targets` | compiles incl. test/bench targets | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail |
| `cargo xtask build-guests --check` | guest WASM fresh after slicer-core edits | FACT clean/STALE |

## Step Completion Expectations

- Cross-step invariant: no step may regress the 14 round-1
  `arachne_parity.rs` locks (AC-7), nor the pre-existing
  `wall_sequence_reorder_tdd` binary (AC-8). Step 4b (module integration)
  is the one most likely to cause collateral damage — verify AC-7 and AC-8
  immediately after it, not only at packet close.
- Ordering rationale: the `SparsePointGrid` step (Step 1) is the
  foundation for `get_region_order` (Step 2) and `topological_walk`
  (Step 3). Pipeline integration (Step 4a) depends on all three; module
  integration (Step 4b) depends on 4a. The doc + final-gate step (Step 5)
  runs last.
- Step 4b edits a guest module (`arachne-perimeters`), so
  `cargo xtask build-guests --check` **will** report STALE until the guests
  are rebuilt. Rebuild before attributing any component/dispatch test
  failure to the change.
- Shared scratch state: none. The new module and the new
  `ArachneParams` field are independent of packet A's changes.

## Context Discipline Notes

- Large files in the read-only path that MUST be ranged or
  delegated: `OrcaSlicerDocumented/src/libslic3r/Arachne/WallToolPaths.cpp`
  (~895 lines; delegate SUMMARY + SNIPPETs for the `getRegionOrder`
  block), `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp`
  (~100+ lines around the topological walk; delegate SUMMARY +
  SNIPPETs).
- Likely temptation reads: `path-optimization-default` — NOT needed;
  the G12 fix is in slicer-core, not the path-optimization module.
  Skip unless the implementer wants to verify the user's "fix in
  slicer-core" decision.
- Heaviest dispatch return-format hint: the OrcaSlicer
  `getRegionOrder` walk must be returned as `SUMMARY` (≤200
  words) + at most three 30-line `SNIPPET`s (the `searching_radius`
  derivation, the `is_odd` constraint emission, and the
  `shorter_then` predicate) — never the full file.
