---
status: draft
packet: 156-arachne-region-order
task_ids:
  - none
backlog_source: docs/18_arachne_parity_audit.md
context_cost_estimate: M
---

# Packet Contract: 156-arachne-region-order

## Goal

Close the round-3 Arachne parity gap **G12** (`WallToolPaths::getRegionOrder` missing from the pipeline) by porting the canonical OrcaSlicer constraint-set + greedy nearest-neighbor topological walk into `slicer-core::arachne` and calling it inside `run_arachne_pipeline` after `generate_toolpaths` flattening.

## Scope Boundaries

Adds a new `crates/slicer-core/src/arachne/region_order.rs` module implementing `get_region_order` (the spatial-adjacency constraint-set builder, ported from `WallToolPaths.cpp:973-1058`) and `topological_walk` (the greedy ordered-emission walk from `PerimeterGenerator.cpp:2781-2857`). Adds a new `SparsePointGrid` utility (no PnP equivalent exists today) used by `get_region_order` to look up nearby junctions within `searching_radius = max_line_w * 1.9`. Calls both inside `run_arachne_pipeline` between the `generate_toolpaths` flatten at `pipeline.rs:383` and `stitch_extrusions` at `pipeline.rs:390`, so the returned `Vec<ExtrusionLine>` has inner (odd) regions following their enclosing even regions.

`ArachneParams` gains an `outer_to_inner: bool` field, **default `false`** — matching OrcaSlicer, where `wall_sequence` defaults to `InnerOuter` (`PrintConfig.cpp:2084`) and the call site computes `is_outer_wall_first = (wall_sequence == OuterInner) || (wall_sequence == InnerOuterInner && layer_id != 0)` (`PerimeterGenerator.cpp:2761-2766`). The `arachne-perimeters` module's `arachne_params_from_config` already reads `wall_sequence` (`lib.rs:269-272`) and derives the bool.

**Relationship to the existing `slicer-core` wall-sequence reorder:** `crates/slicer-core/src/perimeter_utils.rs:723` already exposes `wall_sequence_reorder` (tested by `crates/slicer-core/tests/wall_sequence_reorder_tdd.rs`), which reorders a region's `Vec<WallLoop>` *after* the perimeter module has built them. That helper is **not** replaced, moved, or called by this packet; the two passes operate on different types at different stages (`Vec<ExtrusionLine>` pre-module vs `Vec<WallLoop>` post-module) and must not double-apply a direction flip. AC-8 locks that they compose rather than fight. The module's per-region `sort_by_key(perimeter_index)` (`lib.rs:614`) likewise stays in place.

No WIT/IR/manifest/scheduler changes; no `path-optimization-default` changes; no new config keys.

## Prerequisites and Blockers

- Depends on: `155-arachne-beading-simplify-parity` (packet A) — both share the
  G15/G20/G12 audit cluster, but packet B has no read or compile
  dependency on packet A's beading/simplify changes. The packets are
  independent and can technically land in either order, but the audit
  spec recommends G15+G20 first (the implementer has been advised of
  this).
- Unblocks: none (G11 concentric-infill-via-Arachne is a separate
  pre-existing red; G12 closes a different gap).
- Activation blockers: packet A (`155-arachne-beading-simplify-parity`)
  must be `status: implemented` before packet B is opened, to keep
  the test-suite green during B's development. If A is in-flight,
  B's worker must account for A's pending changes in the
  workspace.

## Acceptance Criteria

Acceptance Criteria are stated **once**, here. `requirements.md` references them by ID.

- **AC-1 (G12 fixture lock). Given** two concentric square islands
  (20 mm and 10 mm, same centre) fed to `run_arachne_pipeline` with
  `ArachneParams::default()`, **when** the returned `Vec<ExtrusionLine>`
  is scanned, **then** the first index of any `inset_idx >= 1` (inner)
  line is greater than the last index of any `inset_idx == 0` (outer)
  line — inner regions follow their enclosing even regions. |
  `cargo test -p slicer-runtime --test arachne_parity_round2 -- arachne_parity_wall_region_order_odd_after_enclosing --exact`
- **AC-2 (getRegionOrder constraint set — exact predicate). Given** the
  `get_region_order(input: &[ExtrusionLine], outer_to_inner: bool) -> Vec<(usize, usize)>`
  function in the new `region_order.rs`, **when** it processes a hand-built
  input of 4 lines (2 even/outer, 2 odd/inner on concentric islands),
  **then** for every pair of lines with a junction within
  `searching_radius = max_line_w * 1.9` of each other it emits exactly the
  constraint `WallToolPaths.cpp:1044-1054` emits, where the pair `(a, b)`
  means **"a must be emitted before b"**:
  ```
  if here.is_odd || nearby.is_odd {
      if  here.is_odd && !nearby.is_odd && nearby.inset_idx < here.inset_idx  → (nearby, here)
      if !here.is_odd &&  nearby.is_odd && here.inset_idx  < nearby.inset_idx → (here, nearby)
  } else if (nearby.inset_idx < here.inset_idx) == outer_to_inner {
      → (nearby, here)
  } else {
      → (here, nearby)
  }
  ```
  Note the **odd branch ignores `outer_to_inner` entirely** — an odd wall is
  *always* preceded by its enclosing lower-`inset_idx` even wall, regardless
  of direction. Only the even/even branch flips on the flag. Isolated lines
  (no junction within `searching_radius` of any other line) emit zero pairs.
  `max_line_w` is the max `junction.p.width` over all junctions of all input
  lines; if it is 0 the function returns an empty constraint set
  (`WallToolPaths.cpp:996-1002`). |
  `cargo test -p slicer-core --test region_order_tdd -- region_order_get_emits_adjacent_constraints --exact`
- **AC-3 (SparsePointGrid lookup). Given** a `SparsePointGrid` built from
  100 deterministic (seeded, not random) point locations within a 10×10 mm
  box with `searching_radius = 1.5 mm`, **when** `get_nearby(query, radius)`
  is called for each input point, **then** every returned point is within
  `radius` of `query` (Euclidean), and no point within `radius` is omitted.
  **The grid's cell size is `searching_radius` itself** — OrcaSlicer
  constructs it as `GridT grid(searching_radius)` (`WallToolPaths.cpp:1022`)
  and `SparsePointGrid`'s ctor stores the value verbatim as the cell size
  (`SparsePointGrid.hpp:31-38`, delegating to `SparseGrid::SparseGrid` at
  `SparseGrid.hpp:106`). **There is no `/ sqrt(2)` derivation anywhere in
  OrcaSlicer** — an earlier draft of this packet asserted one; it was
  fabricated. Correctness comes from `getNearby` scanning every cell the
  query circle can touch and then filtering by true distance
  (`SparseGrid.hpp:137-146`), not from shrinking the cell. The PnP port must
  do the same: scan the 3×3 (or wider, if `radius > cell_size`) cell
  neighborhood, then filter by exact distance. |
  `cargo test -p slicer-core --test sparse_point_grid_tdd -- sparse_point_grid_get_nearby_returns_only_nearby_points --exact`
- **AC-4 (topological_walk greedy order). Given** the
  `topological_walk(lines: &[ExtrusionLine], constraints: &[(usize, usize)]) -> Vec<usize>`
  function in `region_order.rs`, **when** it processes a 4-line input
  with constraints `[(0, 2), (0, 3), (1, 2), (1, 3)]` (lines 0/1 must be
  emitted before lines 2/3), **then**:
  - it builds `blocked: Vec<usize>` (in-degree) and `blocking: Vec<Vec<usize>>`
    (out-edges) from the constraint pairs, exactly as
    `PerimeterGenerator.cpp:2782-2795` does;
  - the returned index order has 0 and 1 before 2 and 3;
  - the **initial cursor** is `lines[0].junctions[0].p` — the first junction
    of the first line in input order — falling back to `(0.0, 0.0)` **only
    when the input is empty** (`PerimeterGenerator.cpp:2798-2799`:
    `all_extrusions.empty() ? Point::Zero() : all_extrusions.front()->junctions.front().p`).
    An earlier draft claimed the cursor always starts at `(0,0)` and that the
    walk begins with the line nearest the origin — **that is wrong** and must
    not be implemented;
  - among the unblocked candidates, **open lines (`is_closed == false`) are
    iterated before closed ones** (`PerimeterGenerator.cpp:2815-2818` sorts
    ascending on the `is_closed` bool); distance is *not* a sort key — it is
    evaluated candidate-by-candidate within that iteration order;
  - after each emission the cursor advances to the emitted line's first
    junction, and every line it was `blocking` has its `blocked` count
    decremented;
  - ties are broken by `original_index` ascending (a PnP determinism
    addition; OrcaSlicer relies on `std::sort` stability here). |
  `cargo test -p slicer-core --test region_order_tdd -- region_order_topological_walk_respects_constraints --exact`
- **AC-5 (pipeline integration). Given** `run_arachne_pipeline` is
  called with the G12 fixture (two concentric islands), **when** the
  function returns, **then** the `Vec<ExtrusionLine>` has been
  reordered by the new region-order pass: the
  `let lines: Vec<ExtrusionLine> = buckets.into_iter().flatten().collect();`
  at `pipeline.rs:383` is followed by
  `reorder_by_region_order(&mut lines, params.outer_to_inner)` before the
  `stitch_extrusions(lines, max_gap)` at `pipeline.rs:390`. |
  `cargo test -p slicer-runtime --test arachne_parity_round2 -- arachne_parity_wall_region_order_odd_after_enclosing --exact`
- **AC-6 (outer_to_inner from wall_sequence — correct polarity). Given**
  the new `ArachneParams::outer_to_inner: bool` field with **default
  `false`**, **when** the module's `arachne_params_from_config` resolves the
  `wall_sequence` config key (`arachne-perimeters.toml:249-253`; type
  `string`, default `"InnerOuter"`, values `InnerOuter` / `OuterInner` /
  `InnerOuterInner`), **then** it derives:
  ```
  outer_to_inner = (wall_sequence == "OuterInner")
                || (wall_sequence == "InnerOuterInner" && !is_initial_layer)
  ```
  matching `PerimeterGenerator.cpp:2761-2766` exactly (Orca disables the
  sandwich mode's outer-first behavior on layer 0). Therefore:
  - `wall_sequence = "InnerOuter"` (**the default**) → `outer_to_inner = false`
    → **inner walls emitted first**;
  - `wall_sequence = "OuterInner"` → `outer_to_inner = true` → **outer walls
    emitted first** (the enum's own OrcaSlicer UI label is literally
    "Outer/Inner");
  - `wall_sequence = "InnerOuterInner"` → `true` on layers > 0, `false` on
    layer 0.

  **An earlier draft of this packet had this mapping exactly inverted**
  (claiming `default true` "matching `InnerOuter`", and describing
  `OuterInner` as "inner walls first"). Implementing the inverted mapping
  would flip wall order on every default slice. The module's existing read at
  `lib.rs:269-272` (`wall_sequence_is_inner_outer`) already uses the correct
  polarity — the new derivation must agree with it, not contradict it.

  Note the odd-wall constraint branch (AC-2) is direction-independent, so the
  G12 fixture's "odd after enclosing" assertion holds under **both** values of
  `outer_to_inner`; the flag only reorders even/even pairs. |
  `cargo test -p slicer-runtime --test arachne_parity_round2 -- arachne_parity_wall_region_order_odd_after_enclosing --exact`
- **AC-7 (regression lock). Given** the 14 round-1 `arachne_parity.rs`
  locks + the G3/G10 closures from packet 152 + the G15/G20 closures
  from packet A, **when** the region-order pass lands, **then** all
  locks still pass. The pass must be a *permutation*, not a *drop or
  duplicate* — the output `Vec<ExtrusionLine>` must have the same length as
  the input and the same multiset of `inset_idx` values (compare sorted
  `Vec<u32>`, not a `BTreeSet`, so duplicates are caught). |
  `cargo test -p slicer-runtime --test arachne_parity && cargo test -p slicer-core`
- **AC-8 (no double-application with the existing wall-sequence reorder).
  Given** the pre-existing `slicer_core::perimeter_utils::wall_sequence_reorder`
  (`perimeter_utils.rs:723`, tested by
  `crates/slicer-core/tests/wall_sequence_reorder_tdd.rs`), which reorders a
  region's `Vec<WallLoop>` *after* the perimeter module builds them, **when**
  the new pre-module `reorder_by_region_order` pass lands, **then** the
  existing helper is unchanged, still called from its existing call sites, and
  its own test binary still passes — and the two passes do **not** both flip
  the outer/inner direction (the new pass orders `ExtrusionLine`s by region
  adjacency; the existing helper orders `WallLoop`s within a region). The
  implementer must confirm this explicitly rather than assume it: if the
  arachne module's `WallLoop` order is derived from the pipeline's now-reordered
  `ExtrusionLine` order, a second direction flip downstream would cancel the
  first. |
  `cargo test -p slicer-core --test wall_sequence_reorder_tdd`

## Negative Test Cases

- **AC-N1 (empty input). Given** an empty `Vec<ExtrusionLine>` input,
  **when** the region-order pass runs, **then** it returns an empty
  `Vec` and emits zero constraints. |
  `cargo test -p slicer-core --test region_order_tdd -- region_order_empty_input_returns_empty --exact`
- **AC-N2 (single line). Given** a single `ExtrusionLine` input, **when**
  the region-order pass runs, **then** the output is a single-element
  `Vec` containing the input line in its original position. The
  `SparsePointGrid` insert + the topological walk must both handle
  the 1-element case without indexing past the end. |
  `cargo test -p slicer-core --test region_order_tdd -- region_order_single_line_preserved --exact`
- **AC-N3 (no spatial adjacency). Given** two islands whose junctions are
  > 100 mm apart, **when** `get_region_order` runs, **then** it emits ZERO
  constraints (`searching_radius = max_line_w * 1.9 ≈ 0.76 mm` for a 0.4 mm
  bead, far below the separation). The topological walk then degrades to the
  unconstrained greedy walk — **starting from the first junction of the first
  input line** (AC-4), *not* from `(0,0)`. |
  `cargo test -p slicer-core --test region_order_tdd -- region_order_no_adjacency_falls_back_to_nearest_neighbor --exact`
- **AC-N4 (SparsePointGrid single-point insert). Given** a
  `SparsePointGrid` with a single inserted point, **when**
  `get_nearby(that_point, any_radius)` is called, **then** the result
  includes that point itself (the query point is in its own vicinity).
  The `insert` API must not deduplicate or drop the point. |
  `cargo test -p slicer-core --test sparse_point_grid_tdd -- sparse_point_grid_single_insert_get_nearby_self --exact`
- **AC-N5 (zero-width input). Given** an input where every junction has
  `p.width == 0.0` (so `max_line_w == 0`), **when** `get_region_order` runs,
  **then** it returns an empty constraint set immediately rather than building
  a grid with a zero cell size (which would divide by zero when computing cell
  indices). This is OrcaSlicer's own guard at `WallToolPaths.cpp:996-1002`. |
  `cargo test -p slicer-core --test region_order_tdd -- region_order_zero_max_line_width_returns_no_constraints --exact`

## Verification

Gate commands only — full matrix in `requirements.md` §Verification Commands.

- `cargo test -p slicer-runtime --test arachne_parity_round2 -- arachne_parity_wall_region_order_odd_after_enclosing`
- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`

## Authoritative Docs

- `docs/18_arachne_parity_audit.md` — load the G12 detailed-gap
  section (lines 296-322) and the "Round-3 fixture additions"
  section (lines 406-413).
- `docs/08_coordinate_system.md` — load directly; the new
  `SparsePointGrid` operates in mm-space (the test fixture is in mm)
  and the pipeline's `generate_toolpaths` output is in
  `ExtrusionJunction.p.x/y` f32-mm space.
- `docs/04_host_scheduler.md` — delegate a SUMMARY of the
  `wall_generator` dispatch section; the G12 fix does not touch the
  scheduler, but the implementer should confirm the
  `path-optimization-default` module is not the right home for the
  fix (it isn't; see Open Questions in `design.md`).
- `docs/03_wit_and_manifest.md` — delegate a SUMMARY of the
  config-key schema (the `wall_sequence` key is already registered
  on `arachne-perimeters.toml`; the new `outer_to_inner` field is
  derived from it, not a new key).

## Doc Impact Statement (Required)

This packet adds a new arachne sub-module and modifies the pipeline
emission order — `none` is not eligible. Sections added/modified:

- `docs/18_arachne_parity_audit.md` §"Gap summary table" — mark G12
  closed — `rg -q 'G12.*closed' docs/18_arachne_parity_audit.md`
- `docs/18_arachne_parity_audit.md` §"Detailed gaps" — update the
  G12 entry's "PnP status" to "closed (this packet)" —
  `rg -q 'getRegionOrder' docs/18_arachne_parity_audit.md`
- `docs/DEVIATION_LOG.md` — add D-157 (region-order port) —
  `rg -q 'D-157' docs/DEVIATION_LOG.md`. **D-157 must record:** (a) that
  wall-sequence direction is now resolved in **two** places in `slicer-core`
  — the new pre-module `reorder_by_region_order` and the pre-existing
  post-module `perimeter_utils::wall_sequence_reorder` (`:723`) — with the
  reasoning for why they compose rather than double-apply (AC-8); (b) the
  `topological_walk` tie-break by `original_index` is a PnP determinism
  addition (OrcaSlicer leans on `std::sort` stability); (c) PnP's
  `SparsePointGrid` is monomorphised on a single payload type rather than
  templated as `SparsePointGrid<T, Locator>`.
- `docs/adr/0011-perimeter-module-owns-wall-sequencing.md` — **no edit
  required, but confirm conformance:** ADR-0011 locks
  "`PerimeterRegion.walls: Vec<WallLoop>` is committed in final print order"
  and that `wall_sequence` is owned by the perimeter module. This packet's
  pass runs *inside* `run_arachne_pipeline`, which the arachne-perimeters
  module itself calls, and it reorders `Vec<ExtrusionLine>` (pre-`WallLoop`)
  — so the module still owns the committed order. Verify with
  `rg -q 'final print order' docs/adr/0011-perimeter-module-owns-wall-sequencing.md`
  and state the conformance argument in D-157.
- `CONTEXT.md` — add glossary entries for *region order* and
  *SparsePointGrid* — `rg -q 'SparsePointGrid' CONTEXT.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

> **All line numbers below were resolved against the real
> `OrcaSlicerDocumented/` tree on 2026-07-14.** The previous draft of this
> packet cited `WallToolPaths.cpp:809-893`, `WallToolPaths.hpp:104` and
> `PerimeterGenerator.cpp:2270-2360` — all wrong, by 100–500 lines. It also
> asserted a `radius / sqrt(2)` cell-size rule and a `Point::Zero()` initial
> cursor, **neither of which exists in OrcaSlicer**. Do not reintroduce any of
> these. If a delegated read does not find the claimed content at the claimed
> line, stop and re-resolve — do not guess.

- `OrcaSlicerDocumented/src/libslic3r/Arachne/WallToolPaths.hpp:211` — `static ExtrusionLineSet getRegionOrder(const std::vector<ExtrusionLine*>& input, bool outer_to_inner);` — AC-2's signature. (PnP takes `&[ExtrusionLine]` and returns `Vec<(usize, usize)>` of indices instead of a pointer set; the semantic is identical.)
- `OrcaSlicerDocumented/src/libslic3r/Arachne/WallToolPaths.cpp:973-1058` — the full `getRegionOrder` impl. Landmarks: `max_line_w` computed as the max `junction.w` over all lines, with an **early return when it is 0** (`:996-1002`); `constexpr float diagonal_extension = 1.9f; searching_radius = max_line_w * diagonal_extension` (`:1019-1020`); the `LineLoc { junction, line }` payload + `Locator` returning `elem.j.p` (`:1003-1012`); `GridT grid(searching_radius)` — **the cell size IS the searching radius** (`:1022`); the constraint emission with its direction-independent `is_odd` branch (`:1044-1054`). Arbiter of AC-2 and AC-3.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/utils/SparsePointGrid.hpp:31-38, 44, 54-58` and `SparseGrid.hpp:106, 137-146` — the canonical grid API: ctor takes a **`cell_size` stored verbatim** (its doc says only "typical values would be around 0.5–2x of expected query radius" — **there is no `/ sqrt(2)`**); `insert(elem)`; `getNearby(query_pt, radius)` which scans the touched cells and filters by true distance. Arbiter of AC-3's API shape.
- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:2781-2857` — the topological-walk consumer. Landmarks: `blocked` / `blocking` adjacency build from the constraint pairs (`:2782-2795`, where a pair is destructured as `for (auto [before, after] : extrusions_constrains)` — confirming `(a, b)` means "a before b"); the initial cursor `all_extrusions.empty() ? Point::Zero() : all_extrusions.front()->junctions.front().p` (`:2798-2799`); the candidate sort, which orders **open before closed** on the `is_closed` bool and does **not** sort by distance (`:2815-2818`); the per-candidate distance evaluation (`:2820-2842`). Arbiter of AC-4.
- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:2761-2766` — `is_outer_wall_first = (wall_sequence == OuterInner) || (wall_sequence == InnerOuterInner)`, with the layer-0 override `if (layer_id == 0) is_outer_wall_first = (wall_sequence == OuterInner);`. Together with `PrintConfig.hpp:187-192` (the `WallSequence` enum) and `PrintConfig.cpp:2084` (`default = InnerOuter`), this is the arbiter of AC-6's polarity.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split). **This packet is M and has no L step** — the original L-rated Step 4 was split into 4a (pipeline) and 4b (module); no extended-band run is required.
