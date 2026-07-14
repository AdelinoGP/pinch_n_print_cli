# Design: 156-arachne-region-order

## Controlling Code Paths

- Primary code path:
  - `crates/slicer-core/src/arachne/region_order.rs` (NEW) — contains
    `get_region_order`, `topological_walk`, and `reorder_by_region_order`.
  - `crates/slicer-core/src/arachne/sparse_point_grid.rs` (NEW) — the
    `SparsePointGrid<T, F>` utility.
  - `crates/slicer-core/src/arachne/mod.rs` — register the new sub-module.
  - `crates/slicer-core/src/arachne/pipeline.rs` — add the
    `reorder_by_region_order` call after the `generate_toolpaths`
    flatten at `:383`; add the new `ArachneParams::outer_to_inner`
    field with default **`false`** (see the polarity note below).
  - `modules/core-modules/arachne-perimeters/src/lib.rs` —
    `arachne_params_from_config` reads the already-registered
    `wall_sequence` config key and passes the resolved `bool` into
    `ArachneParams`.
- Neighboring tests/fixtures:
  - `crates/slicer-runtime/tests/arachne_parity_round2.rs:40-106` —
    the G12 RED test.
  - `crates/slicer-runtime/tests/fixtures/arachne_parity/mod.rs:107-109`
    — the `ex_polygons_concentric_islands_mm()` G12 fixture.
  - `crates/slicer-core/tests/region_order_tdd.rs` and
    `crates/slicer-core/tests/sparse_point_grid_tdd.rs` — new top-level
    test files (auto-discovered; there is no `tests/arachne/` directory).
  - `crates/slicer-core/tests/wall_sequence_reorder_tdd.rs` — the existing
    test binary for `perimeter_utils::wall_sequence_reorder`; must stay
    green (AC-8).
- OrcaSlicer comparison surface: see `requirements.md` §OrcaSlicer
  Reference Obligations (delegate; never load).

## Architecture Constraints

- `get_region_order` is a pure function: same input → same output.
  No randomness, no global state. `searching_radius = max_line_w * 1.9`
  (`WallToolPaths.cpp:1019-1020`), where `max_line_w` is the maximum
  junction width across all input lines; if `max_line_w == 0` the function
  returns an empty constraint set (`WallToolPaths.cpp:996-1002`, AC-N5).
- The new `ArachneParams::outer_to_inner` field is set by the pipeline
  caller (the module's `arachne_params_from_config`). The pipeline itself
  does NOT derive it from any other config key. **Default `false`** — see
  the polarity note below.
- **`outer_to_inner` polarity (get this right — an earlier draft had it
  inverted):** `outer_to_inner == true` means *outer walls emitted first*.
  OrcaSlicer's `wall_sequence` defaults to `InnerOuter` (`PrintConfig.cpp:2084`)
  and the call site computes
  `is_outer_wall_first = (ws == OuterInner) || (ws == InnerOuterInner)`, with
  `if (layer_id == 0) is_outer_wall_first = (ws == OuterInner);`
  (`PerimeterGenerator.cpp:2761-2766`). So the **default is `false`**, and
  `OuterInner` (UI label: "Outer/Inner") maps to `true`. The
  `arachne-perimeters` module's existing read at `lib.rs:269-272`
  (`wall_sequence_is_inner_outer`) already uses this polarity; the new
  derivation must agree with it.
- The reorder pass operates on the post-`generate_toolpaths`,
  pre-`stitch_extrusions` `Vec<ExtrusionLine>`. Two neighbouring reorders
  already exist and are **left untouched**:
  - `slicer_core::perimeter_utils::wall_sequence_reorder` (`perimeter_utils.rs:723`,
    tested by `crates/slicer-core/tests/wall_sequence_reorder_tdd.rs`) —
    reorders a region's `Vec<WallLoop>` **after** the module builds them.
  - the module's per-region `sort_by_key(perimeter_index)`
    (`arachne-perimeters/src/lib.rs:614`) — a stable secondary sort on
    `Vec<WallLoop>`.

  Neither is replaced or called by this packet. The implementer **must
  verify they do not double-apply a direction flip** (AC-8): if the module's
  `WallLoop` order derives from the pipeline's now-reordered `ExtrusionLine`
  order, a second flip downstream would cancel the first.
- **`SparsePointGrid`'s cell size IS `searching_radius`**, stored verbatim
  (`WallToolPaths.cpp:1022` passes `searching_radius` straight into the ctor;
  `SparsePointGrid.hpp:31-38` stores it as `cell_size`). **There is no
  `/ sqrt(2)` rule in OrcaSlicer** — an earlier draft of this design asserted
  one and cited `SparsePointGrid.hpp:30-50` for it; that citation contains
  only the ctor doc, which says merely "typical values would be around 0.5–2x
  of expected query radius". Correctness comes from `getNearby` scanning every
  cell the query circle touches and then filtering by exact distance
  (`SparseGrid.hpp:137-146`) — not from shrinking the cell. Port that.
- `topological_walk` is deterministic and follows
  `PerimeterGenerator.cpp:2781-2857`:
  - initial cursor = `lines[0].junctions[0].p` (the first junction of the
    **first input line**), falling back to `(0.0, 0.0)` **only when the input
    is empty** (`:2798-2799`). It is **not** "the line nearest the origin" —
    an earlier draft asserted that; it is wrong.
  - among unblocked candidates, **open lines are iterated before closed ones**
    (`:2815-2818` sorts ascending on the `is_closed` bool). Distance is not a
    sort key; it is evaluated per-candidate within that order (`:2820-2842`).
  - ties broken by `original_index` ascending (PnP determinism addition;
    OrcaSlicer relies on `std::sort` stability). Record in D-157.
- The new module does NOT mutate the input `&[ExtrusionLine]`. The
  `reorder_by_region_order` wrapper takes `&mut Vec<ExtrusionLine>`
  and applies the permutation in place (this is the only mutation).

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- Packet-specific constraint: the exact IR shapes (verified against the
  tree — the earlier draft named the wrong crate and the wrong field):
  - `ExtrusionLine` lives in **`crates/slicer-ir/src/slice_ir.rs:1836-1849`**
    (**not** `crates/slicer-core/src/slice_ir.rs`), with fields
    `junctions: Vec<ExtrusionJunction>`, `inset_idx: u32`, `is_odd: bool`,
    `is_closed: bool`.
  - `ExtrusionJunction` (`slice_ir.rs:1819-1825`) has `p: Point3WithWidth`
    and `perimeter_index: u32`.
  - `Point3WithWidth` (`slice_ir.rs:1618-1632`) carries `x: f32`, `y: f32`,
    `z`, and **`width: f32`** — the width field is `p.width`, **not** `w` as
    in OrcaSlicer. `max_line_w` is therefore
    `max over junctions of j.p.width`.
  All distances are computed in f32 mm space (matching the fixture's mm
  convention); the grid's cell size is `searching_radius` in the same mm
  space.

- Packet-specific constraint: the G12 fixture's two concentric
  20 mm + 10 mm squares produce `ExtrusionLine`s with `max_line_w
  ≈ 0.4 mm`, so `searching_radius ≈ 0.76 mm`. The two squares' walls are
  ~5 mm apart, so **whether the constraint set is non-empty for this fixture
  depends on the wall geometry Arachne actually emits between them** — the
  implementer must confirm empirically in Step 2 (print the constraint count
  for the fixture) rather than assume. If the fixture emits no adjacency
  constraints, the G12 test would pass on the topological walk's ordering
  alone, which would be a **vacuous** close — in that case the fixture must be
  tightened (islands moved within `searching_radius`) before the packet can
  claim G12 closed.

## Code Change Surface

- Selected approach: full faithful port of `getRegionOrder` +
  topological walk from `WallToolPaths.cpp:973-1058` +
  `PerimeterGenerator.cpp:2781-2857`, plus a new
  `SparsePointGrid` utility. The user explicitly chose this over
  the simpler "sort by `(is_odd, inset_idx)`" shortcut so the
  constraint set can be reused by a future path-optimizer
  extension. The `outer_to_inner` parameter is derived from
  the `wall_sequence` config (already registered on
  `arachne-perimeters.toml:249-253`).
- Exact functions, traits, manifests, tests, or fixtures
  expected to change:
  - `crates/slicer-core/src/arachne/region_order.rs` (NEW) —
    `get_region_order`, `topological_walk`, `reorder_by_region_order`.
  - `crates/slicer-core/src/arachne/sparse_point_grid.rs` (NEW) —
    `SparsePointGrid<T, F>`.
  - `crates/slicer-core/src/arachne/mod.rs` — `pub mod region_order;`
    + `pub mod sparse_point_grid;` (the existing list is
    `generate_toolpaths, pipeline, preprocess, remove_small,
    separate_inner_contour, simplify, stitch` at `:71-89`).
  - `crates/slicer-core/src/arachne/pipeline.rs` — add
    `ArachneParams::outer_to_inner: bool` (**default `false`**) to the
    struct (`:75-168`) and its `Default` impl (`:170-219`);
    call `reorder_by_region_order(&mut lines, params.outer_to_inner)`
    after the `:383` flatten and before the `stitch_extrusions` at `:390`.
  - `modules/core-modules/arachne-perimeters/src/lib.rs` —
    `arachne_params_from_config` (`:130`) already reads `wall_sequence`
    (`:269-272`); derive `outer_to_inner` from it with the correct polarity
    (AC-6) and pass it into `ArachneParams`.
  - `crates/slicer-core/tests/region_order_tdd.rs` (NEW) — 6 tests:
    AC-2, AC-4, AC-N1, AC-N2, AC-N3, AC-N5.
  - `crates/slicer-core/tests/sparse_point_grid_tdd.rs` (NEW) — 2 tests:
    AC-3, AC-N4.
  - **Test-path note (the earlier draft contradicted itself):** the test
    files live at the **top level** of `crates/slicer-core/tests/`, i.e.
    `tests/region_order_tdd.rs` and `tests/sparse_point_grid_tdd.rs` — **not**
    under a `tests/arachne/` subdirectory (no such directory exists; every
    existing arachne test is a flat `tests/arachne_*.rs` file). Top-level
    `tests/*.rs` files are **auto-discovered** by Cargo, so **no `Cargo.toml`
    `[[test]]` entry is required** and no per-step edit-cap bust is incurred.
    The `[[test]] path = …` entries in `crates/slicer-core/Cargo.toml` exist
    only for files under the `tests/beading/` subdirectory, which are not
    auto-discovered.
  - `docs/18_arachne_parity_audit.md` — update Gap summary table.
  - `docs/DEVIATION_LOG.md` — add D-157.
  - `CONTEXT.md` — add 2 glossary entries.
- Rejected alternatives:
  - **Sort by `(is_odd, inset_idx)`:** rejected; the user
    explicitly chose the full `getRegionOrder` + topological
    walk port to preserve the constraint-set's reuse
    potential. The simple sort does not emit constraints and
    cannot be consumed by a path-optimizer extension.
  - **Apply the region-order pass in `path-optimization-default`:**
    rejected; the `OrderedEntityView` and `PerimeterRegionView`
    types do not carry `inset_idx` / `is_odd` and adding
    them is a WIT contract change. The `run_arachne_pipeline`
    output is the canonical place for the pass (the
    `ExtrusionLine` struct already has the fields).
  - **Co-locate `SparsePointGrid` in `crates/slicer-helpers`:**
    rejected; the `SparsePointGrid` is only used by
    `region_order` in this packet, and the helpers crate
    should not gain a one-consumer utility. If a future
    packet needs the grid, it can be promoted to helpers.
  - **Generic `SparsePointGrid<T, F>` with a `Locator` trait:**
    rejected for this packet; the implementer may add the
    trait in a follow-up if a second consumer appears. The
    packet's module doc comment notes the generic shape but
    uses the concrete `ExtrusionJunction` payload.

## Files in Scope (read + edit)

- `crates/slicer-core/src/arachne/region_order.rs` (NEW) — role:
  the new sub-module; expected change: 3 public functions +
  the unit tests.
- `crates/slicer-core/src/arachne/sparse_point_grid.rs` (NEW) —
  role: the new grid utility; expected change: the
  `SparsePointGrid<T, F>` struct + 2 methods + the unit tests.
- `crates/slicer-core/src/arachne/mod.rs` — role: sub-module
  registry; expected change: 2 `pub mod` declarations.
- `crates/slicer-core/src/arachne/pipeline.rs` — role: the
  pipeline orchestrator; expected change: 1 new `ArachneParams`
  field + 1 new function call after the `:383` flatten.
- `modules/core-modules/arachne-perimeters/src/lib.rs` — role:
  module config resolution; expected change: 1-line addition in
  `arachne_params_from_config` to read `wall_sequence` and pass
  through to `ArachneParams`.
- `crates/slicer-core/tests/region_order_tdd.rs` (NEW) — role:
  G12 unit tests; expected change: 6 new tests (AC-2, AC-4, AC-N1,
  AC-N2, AC-N3, AC-N5) in one new file. **No `Cargo.toml` edit needed** —
  top-level `tests/*.rs` files are auto-discovered by Cargo.
- `crates/slicer-core/tests/sparse_point_grid_tdd.rs` (NEW) —
  role: G12 grid tests; expected change: 2 new tests (AC-3, AC-N4)
  in one new file. **No `Cargo.toml` edit needed** — same reason.
- `docs/18_arachne_parity_audit.md` — doc update for G12 close.
- `docs/DEVIATION_LOG.md` — add D-157.
- `CONTEXT.md` — 2 glossary entries.

## Read-Only Context

- `crates/slicer-core/src/perimeter_utils.rs:723` —
  `wall_sequence_reorder` — purpose: the pre-existing post-module
  wall-sequence reorder. **Read this before writing any direction logic**;
  the new pass must compose with it, not duplicate or cancel it (AC-8).
- `crates/slicer-ir/src/slice_ir.rs:1618-1632, 1819-1825, 1836-1849` —
  `Point3WithWidth` / `ExtrusionJunction` / `ExtrusionLine` — purpose:
  confirm the real field names (`p.width`, not `w`) and the real crate
  (`slicer-ir`, not `slicer-core`).
- `crates/slicer-core/src/arachne/generate_toolpaths.rs:880-895` —
  `generate_toolpaths` return type — purpose: confirm the
  `Vec<VariableWidthLines>` bucket shape (no change in this
  packet).
- `crates/slicer-core/src/arachne/mod.rs` — the `pub mod`
  registry — purpose: confirm the existing sub-module pattern
  (e.g. `pub mod simplify;` at the top).
- `crates/slicer-core/src/arachne/pipeline.rs:328-411` — the
  full `run_arachne_pipeline` body — purpose: confirm the
  insertion point for the new call (after `:383` flatten, before
  `:390` stitch).
- `crates/slicer-runtime/tests/arachne_parity_round2.rs:40-106` —
  the G12 RED test — purpose: confirm the test's expected
  ordering assertion (`outer_max >= inner_min` becomes false
  after the fix).
- `crates/slicer-runtime/tests/fixtures/arachne_parity/mod.rs:107-109`
  — the G12 fixture — purpose: confirm the two-island input
  shape (`square_mm(20.0)`, `square_mm(10.0)`).
- `docs/15_config_keys_reference.md` — load the `wall_sequence`
  entry directly; purpose: confirm the config key's type and
  default (the module reads it today).

## Out-of-Bounds Files

- `OrcaSlicerDocumented/` — delegate parity checks; never load
  directly.
- `target/`, `Cargo.lock`, generated code under `wit-guest/` —
  never load.
- `crates/slicer-runtime/src/run.rs`,
  `crates/slicer-scheduler/` — outside the G12 change surface.
- `crates/slicer-core/src/beading/` — no changes (packet A's
  scope).
- `crates/slicer-core/src/arachne/simplify.rs` — no changes
  (packet A's scope).
- `crates/slicer-wasm-host/` — no changes.
- `modules/core-modules/path-optimization-default/` — no changes
  (per the user's "fix in slicer-core" decision; the
  `OrderedEntityView` and `PerimeterRegionView` types are NOT
  extended in this packet).

## Expected Sub-Agent Dispatches

- "Run `cargo test -p slicer-runtime --test arachne_parity_round2 --
  arachne_parity_wall_region_order_odd_after_enclosing --exact`;
  return FACT pass/fail or SNIPPETS (fail with assertion + ≤20
  lines)" — purpose: validate AC-1 + AC-5 + AC-6.
- "Run `cargo test -p slicer-core --test region_order_tdd -- region_order_get_emits_adjacent_constraints --exact`; FACT pass/fail" — purpose: validate AC-2.
- "Run `cargo test -p slicer-core --test sparse_point_grid_tdd -- sparse_point_grid_get_nearby_returns_only_nearby_points --exact`; FACT pass/fail" — purpose: validate AC-3.
- "Run `cargo test -p slicer-core --test region_order_tdd -- region_order_topological_walk_respects_constraints --exact`; FACT pass/fail" — purpose: validate AC-4.
- "Run `cargo test -p slicer-core --test region_order_tdd -- region_order_empty_input_returns_empty --exact`; FACT pass/fail" — purpose: validate AC-N1.
- "Run `cargo test -p slicer-core --test region_order_tdd -- region_order_single_line_preserved --exact`; FACT pass/fail" — purpose: validate AC-N2.
- "Run `cargo test -p slicer-core --test region_order_tdd -- region_order_no_adjacency_falls_back_to_nearest_neighbor --exact`; FACT pass/fail" — purpose: validate AC-N3.
- "Run `cargo test -p slicer-core --test sparse_point_grid_tdd -- sparse_point_grid_single_insert_get_nearby_self --exact`; FACT pass/fail" — purpose: validate AC-N4.
- "Run `cargo test -p slicer-runtime --test arachne_parity`;
  return FACT pass/fail (AC-7 14-locks regression lock) or
  SNIPPETS (fail with assertion + ≤20 lines)" — purpose: validate
  AC-7.
- "Delegate OrcaSlicer `WallToolPaths.cpp:973-1058` `getRegionOrder`
  walk; return SUMMARY (≤200 words) + at most three 30-line
  SNIPPETs (the `max_line_w`/`searching_radius` derivation at
  `:996-1020`, the grid construction at `:1003-1022`, and the
  `is_odd` constraint emission at `:1044-1054`)" — purpose:
  arm Step 2's port with the canonical reference.
- "Delegate OrcaSlicer `PerimeterGenerator.cpp:2781-2857`
  topological walk; return SUMMARY (≤200 words) + at most two
  30-line SNIPPETs (the `blocked`/`blocking` adjacency build at
  `:2782-2795`, and the cursor init + candidate sort + greedy
  selection at `:2798-2842`)" — purpose: arm Step 3's port with the
  canonical reference.
- "Delegate OrcaSlicer `SparsePointGrid.hpp:31-58` +
  `SparseGrid.hpp:106,137-146`; return SUMMARY (≤200 words) + at most
  two 30-line SNIPPETs (the ctor's `cell_size` handling and the
  `insert`/`getNearby` signatures)" — purpose: arm Step 1's port.
  **Ask specifically whether any `sqrt(2)` scaling is applied to the
  cell size — it is not; an earlier draft claimed it was.**

## Data and Contract Notes

- `ExtrusionLine` / `ExtrusionJunction` / `Point3WithWidth` are
  UNCHANGED (no IR struct changes).
- `ArachneParams` gains 1 new field: `outer_to_inner: bool`
  (default **`false`** — OrcaSlicer's `wall_sequence` defaults to
  `InnerOuter`, which yields `is_outer_wall_first == false`).
- `OrderedEntityView` / `PerimeterRegionView` / `WallLoop` are
  UNCHANGED (no path-optimization view changes).
- No WIT contract changes (no `slicer-macros` /
  `slicer-schema` / WIT boundary touches).
- No scheduler / host-service changes.
- The `wall_sequence` config key is already registered on
  `arachne-perimeters.toml`; the module's `arachne_params_from_config`
  reads it today (per packet 151 closure) — the G12 packet only
  adds the boolean translation to `ArachneParams`.
- Determinism preserved: `getRegionOrder` is a pure function
  (deterministic spatial hash + deterministic constraint set);
  `topological_walk` is deterministic (greedy nearest-neighbor
  with `original_index` tie-breaking).

## Locked Assumptions and Invariants

- The new `ArachneParams::outer_to_inner` field defaults to **`false`**,
  matching OrcaSlicer: `wall_sequence` defaults to `InnerOuter`
  (`PrintConfig.cpp:2084`), which yields `is_outer_wall_first == false`
  (`PerimeterGenerator.cpp:2761-2766`). `outer_to_inner == true` means
  **outer walls first** and corresponds to `wall_sequence == "OuterInner"`.
- The reorder pass is a permutation: it neither drops nor duplicates lines
  (AC-7). The output has the same length and the same **multiset** (sorted
  `Vec<u32>`, not a `BTreeSet` — a set would hide duplicates) of `inset_idx`
  values as the input.
- **The `SparsePointGrid` cell size IS `searching_radius`**, used verbatim
  (`WallToolPaths.cpp:1022`). There is no `/ sqrt(2)` derivation in
  OrcaSlicer. `get_nearby` achieves correctness by scanning every cell the
  query circle can touch and filtering by exact distance
  (`SparseGrid.hpp:137-146`), which the port must reproduce.
- The topological walk's `current_position` starts at the **first junction of
  the first input line**, falling back to `(0.0, 0.0)` only when the input is
  empty (`PerimeterGenerator.cpp:2798-2799`). After each emit the cursor
  advances to the emitted line's first junction.
- Among unblocked candidates, open lines (`is_closed == false`) are iterated
  before closed ones (`PerimeterGenerator.cpp:2815-2818`); distance is
  evaluated per-candidate within that order, not used as a sort key.
- The new module does NOT mutate the input `&[ExtrusionLine]`
  slice; only `reorder_by_region_order` takes `&mut Vec<ExtrusionLine>`.
- The `wall_sequence` config key is unchanged by this packet — the module's
  `arachne_params_from_config` already reads it (`lib.rs:269-272`) and this
  packet only translates it to the `outer_to_inner` bool.
- The pre-existing `perimeter_utils::wall_sequence_reorder` (`:723`) and the
  module's `sort_by_key(perimeter_index)` (`lib.rs:614`) are **unchanged and
  must not double-apply a direction flip** with the new pass (AC-8).

## Risks and Tradeoffs

- **Direction-flip double-application (highest risk).** Wall-sequence
  direction is now resolved in two places in `slicer-core`: the new
  pre-module `reorder_by_region_order` (`Vec<ExtrusionLine>`) and the
  pre-existing post-module `perimeter_utils::wall_sequence_reorder`
  (`Vec<WallLoop>`, `:723`). If the module's `WallLoop` order derives from
  the pipeline's now-reordered `ExtrusionLine` order, a second flip
  downstream cancels the first and the G12 test would be green while
  production output is unchanged (or doubly-flipped). AC-8 exists to force
  this to be *checked*, not assumed. If they do conflict, the resolution is a
  design decision for the user — do not silently disable either pass.
- The topological-walk port's candidate selection must reproduce
  OrcaSlicer's actual rule (open-before-closed iteration, per-candidate
  distance evaluation — `PerimeterGenerator.cpp:2815-2842`), **not** the
  "sort by distance, then `original_index`" rule an earlier draft of this
  packet described. The `original_index` tie-break is retained only as a
  determinism guarantee where OrcaSlicer leans on `std::sort` stability;
  it is recorded in D-157.
- The new `SparsePointGrid` uses a `HashMap<(i64, i64), Vec<T>>`
  cell map with f32 mm coordinates converted to `i64` cell
  indices via `(p.x as f64 / cell_size).floor() as i64`. Guard the
  `cell_size == 0` case (AC-N5) — `max_line_w == 0` would otherwise divide
  by zero. The `as` casts may lose precision for very small cell sizes
  (< 0.001 mm); the G12 fixture's `cell_size ≈ 0.76 mm` is well above that.
- The `outer_to_inner` field is set by the module; the pipeline does NOT
  derive it from any other config key. Callers that build `ArachneParams`
  literally (e.g. tests) must set it explicitly. `ArachneParams::default()`
  sets it to **`false`**.
- **Fixture may not exercise the constraint set.** If Arachne's emitted walls
  for the 20 mm / 10 mm concentric-square fixture never place two junctions
  within `searching_radius ≈ 0.76 mm` of each other, `get_region_order`
  returns zero constraints and the G12 test would pass on the walk's ordering
  alone — a vacuous close. Step 2 must print the constraint count for the
  fixture and, if it is zero, tighten the fixture before claiming G12 closed.

## Context Cost Estimate

- Aggregate: M. Steps 1–3 are M; Steps 4a/4b/5 are S–M. **No step is L**
  (the previously-L Step 4 has been split into 4a/4b — see
  `implementation-plan.md`), so the packet does not require an extended-band
  run.
- Largest single step: M (Step 2 — the `get_region_order` port).
- Highest-risk dispatch: the OrcaSlicer `getRegionOrder` walk
  (`WallToolPaths.cpp:973-1058`, the heaviest single read in the packet).
  Required return format: SUMMARY (≤200 words) + 3 SNIPPETs (each ≤30
  lines).
- Second-highest-risk dispatch: the topological-walk port (Step 3). The
  greedy selection is the most algorithmically subtle piece; a buggy port
  could still pass the G12 test (which is loose) while failing on complex
  multi-island inputs. The implementer must add an extra test with 4+ lines
  and non-trivial constraints as a safety net.

## Open Questions

- `[BLOCK]` Should packet A (`155-arachne-beading-simplify-parity`)
  be `status: implemented` before packet B is opened? The user has advised
  "G15+G20 first, G12 second" but the packets are technically independent
  (no shared symbol). If packet A is in-flight when B starts, the implementer
  must coordinate — one will see the other's pending changes in the working
  tree. Confirm with the user before activating B.
- `[BLOCK]` **Do the new pre-module region-order pass and the existing
  post-module `perimeter_utils::wall_sequence_reorder` (`:723`) compose, or
  do they double-apply the direction flip?** AC-8 forces the check. If they
  conflict, the resolution (which pass owns direction, and whether ADR-0011
  needs amending) is a user-facing design decision and must be raised, not
  papered over.
- `[FWD]` Should the `SparsePointGrid` be promoted to
  `crates/slicer-helpers` (or a `slicer-core::utils` module) so a future
  packet can reuse it (e.g. a medial-axis spatial lookup)? The packet keeps
  it at `crates/slicer-core/src/arachne/sparse_point_grid.rs`; promotion is a
  future refactor.
- `[FWD]` Should the topological walk consume the `wall_sequence` config
  directly rather than the resolved `outer_to_inner` bool from
  `ArachneParams`? The packet keeps the bool for testability (a unit test can
  pass any bool without building a `ConfigView`).
