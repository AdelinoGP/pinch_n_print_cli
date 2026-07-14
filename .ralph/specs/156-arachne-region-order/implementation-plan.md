# Implementation Plan: 156-arachne-region-order

## Execution Rules

- One atomic step at a time.
- Each step maps back to the audit gap G12 (backlog source
  `docs/18_arachne_parity_audit.md`; no `docs/07` task IDs).
- TDD first (the red gap test already exists; the new unit
  tests are added alongside each module), then implementation,
  then the narrowest falsifying validation.
- Each step honors the context-discipline preamble shared by
  `spec-packet-generator`, `swarm`, and `spec-review`. The
  fields below are the budget contract for the step.

## Steps

### Step 1: Add `SparsePointGrid` utility (with unit tests)

- Gaps: G12.
- Objective: create
  `crates/slicer-core/src/arachne/sparse_point_grid.rs` with
  the `SparsePointGrid<T, F>` struct, `insert`, and
  `get_nearby` methods. **The grid's cell size is the `searching_radius`
  passed to the constructor, stored verbatim** — OrcaSlicer does
  `GridT grid(searching_radius)` (`WallToolPaths.cpp:1022`) and
  `SparsePointGrid`'s ctor keeps it as the cell size
  (`SparsePointGrid.hpp:31-38` → `SparseGrid.hpp:106`). **There is NO
  `/ sqrt(2)` derivation in OrcaSlicer** — an earlier draft of this packet
  asserted one and cited a line range that does not contain it. Correctness
  comes from `get_nearby` scanning every cell the query circle can touch
  (a 3×3 neighborhood when `radius <= cell_size`, wider otherwise) and then
  filtering by exact Euclidean distance, exactly as `SparseGrid.hpp:137-146`
  does. Guard `cell_size == 0` (AC-N5) so cell-index computation cannot
  divide by zero. Register the module in
  `crates/slicer-core/src/arachne/mod.rs`. Add AC-3 + AC-N4 unit tests in
  `crates/slicer-core/tests/sparse_point_grid_tdd.rs`.
- Precondition: packet active. No other packet in flight.
- Postcondition: AC-3 + AC-N4 green; module compiles;
  `cargo test -p slicer-core` clean.
- Files allowed to read: `crates/slicer-core/src/arachne/mod.rs`
  (whole file, load directly); `crates/slicer-ir/src/slice_ir.rs:1618-1632`
  (`Point3WithWidth` — the width field is `width`, not `w`); any existing
  `HashMap<(i64, i64), ...>` usage in
  `crates/slicer-core/src/skeletal_trapezoidation/` for pattern reference
  (delegate SUMMARY of one example).
- Files allowed to edit (≤3):
  `crates/slicer-core/src/arachne/sparse_point_grid.rs` (NEW),
  `crates/slicer-core/src/arachne/mod.rs`,
  `crates/slicer-core/tests/sparse_point_grid_tdd.rs` (NEW).
  **No `Cargo.toml` edit** — top-level `crates/slicer-core/tests/*.rs` files
  are auto-discovered by Cargo as integration-test binaries (every existing
  `arachne_*.rs` test there is registered this way, with no `[[test]]`
  entry). The earlier draft's cap-bust for a registration entry was based on
  a false premise and is withdrawn.
- Files out-of-bounds: the new `region_order.rs` (Step 2);
  the pipeline (Step 4); the module (Step 4).
- Expected sub-agent dispatches:
  - "Delegate OrcaSlicer `SparsePointGrid.hpp` API; return
    SUMMARY (≤200 words) + at most two 30-line SNIPPETs
    (the cell-radius derivation, the `insert`/`get_nearby`
    signatures)."
  - "Run `cargo test -p slicer-core --test sparse_point_grid_tdd -- sparse_point_grid_get_nearby_returns_only_nearby_points --exact`; FACT pass/fail."
  - "Run `cargo test -p slicer-core --test sparse_point_grid_tdd -- sparse_point_grid_single_insert_get_nearby_self --exact`; FACT pass/fail."
- Context cost: `M` (the new struct + 2 methods + 2 tests +
  module registration is substantial).
- Authoritative docs: none new.
- OrcaSlicer refs: `SparsePointGrid.hpp:31-38, 44, 54-58` and
  `SparseGrid.hpp:106, 137-146` — delegate SUMMARY; never load.
- Verification: the 3 dispatches above.
- Exit condition: AC-3 + AC-N4 green.

### Step 2: Port `get_region_order` (with unit tests)

- Gaps: G12.
- Objective: create
  `crates/slicer-core/src/arachne/region_order.rs` with the
  `get_region_order(input: &[ExtrusionLine], outer_to_inner: bool) -> Vec<(usize, usize)>`
  function. Faithful Rust port of `WallToolPaths.cpp:973-1058`:
  - compute `max_line_w = max over all junctions of j.p.width`; **return an
    empty constraint set if it is 0** (`:996-1002`, AC-N5);
  - `searching_radius = max_line_w * 1.9` (`:1019-1020`);
  - build the grid with **cell size = `searching_radius`** (`:1022`),
    payload = `(junction, line_index)`, locator = `j.p`;
  - for every junction, query `get_nearby` and emit constraints per the
    exact predicate at `:1044-1054` — the `is_odd` branch is
    **direction-independent** (an odd wall is always preceded by its
    enclosing lower-`inset_idx` even wall); only the even/even branch flips
    on `outer_to_inner`. A pair `(a, b)` means "**a before b**" (confirmed by
    the consumer's `for (auto [before, after] : ...)` at
    `PerimeterGenerator.cpp:2789`).

  **Then print the constraint count for the G12 fixture.** If it is zero, the
  fixture does not exercise the constraint set and the G12 test would close
  vacuously — stop and tighten the fixture before proceeding (see
  `design.md` §Risks).

  Register the module in `crates/slicer-core/src/arachne/mod.rs`. Add AC-2 +
  AC-N1 + AC-N2 + AC-N3 + AC-N5 unit tests in
  `crates/slicer-core/tests/region_order_tdd.rs`.
- Precondition: Step 1 landed (`SparsePointGrid` exists).
- Postcondition: AC-2 + AC-N1 + AC-N2 + AC-N3 + AC-N5 green.
- Files allowed to read:
  `crates/slicer-core/src/arachne/sparse_point_grid.rs`
  (the new file from Step 1);
  **`crates/slicer-ir/src/slice_ir.rs:1618-1632, 1819-1825, 1836-1849`** —
  `Point3WithWidth` / `ExtrusionJunction` / `ExtrusionLine`. Note this is the
  **`slicer-ir`** crate, not `slicer-core` (an earlier draft named
  `crates/slicer-core/src/slice_ir.rs`, which does not exist), and the width
  field is **`p.width`**, not `w`.
- Files allowed to edit (≤3):
  `crates/slicer-core/src/arachne/region_order.rs` (NEW),
  `crates/slicer-core/src/arachne/mod.rs`,
  `crates/slicer-core/tests/region_order_tdd.rs` (NEW).
  **No `Cargo.toml` edit** — see Step 1.
- Files out-of-bounds: the topological walk (Step 3); the
  pipeline (Step 4); the module (Step 4).
- Expected sub-agent dispatches:
  - "Delegate OrcaSlicer `WallToolPaths.cpp:973-1058`
    `getRegionOrder` walk; return SUMMARY (≤200 words) +
    at most three 30-line SNIPPETs (the `max_line_w`/`searching_radius`
    derivation at `:996-1020`, the grid construction at `:1003-1022`, and
    the `is_odd` constraint emission at `:1044-1054`)."
  - "Run `cargo test -p slicer-core --test region_order_tdd -- region_order_get_emits_adjacent_constraints --exact`; FACT pass/fail."
  - "Run `cargo test -p slicer-core --test region_order_tdd -- region_order_empty_input_returns_empty --exact`; FACT pass/fail (AC-N1)."
  - "Run `cargo test -p slicer-core --test region_order_tdd -- region_order_single_line_preserved --exact`; FACT pass/fail (AC-N2)."
  - "Run `cargo test -p slicer-core --test region_order_tdd -- region_order_no_adjacency_falls_back_to_nearest_neighbor --exact`; FACT pass/fail (AC-N3)."
  - "Run `cargo test -p slicer-core --test region_order_tdd -- region_order_zero_max_line_width_returns_no_constraints --exact`; FACT pass/fail (AC-N5)."
- Context cost: `M` (the port is substantial; the spatial
  adjacency logic + the `is_odd` special-casing + 5 tests).
- Authoritative docs: none new.
- OrcaSlicer refs: `WallToolPaths.cpp:973-1058` and `WallToolPaths.hpp:211`
  — delegate SUMMARY; never load.
- Verification: the 6 dispatches above.
- Exit condition: AC-2 + AC-N1 + AC-N2 + AC-N3 + AC-N5 green, **and** the
  G12 fixture's constraint count is confirmed non-zero.

### Step 3: Port `topological_walk` + `reorder_by_region_order` (with unit tests)

- Gaps: G12.
- Objective: extend
  `crates/slicer-core/src/arachne/region_order.rs` with
  `topological_walk(lines: &[ExtrusionLine], constraints: &[(usize, usize)]) -> Vec<usize>`
  and the convenience wrapper
  `reorder_by_region_order(lines: &mut Vec<ExtrusionLine>, outer_to_inner: bool)`.
  Faithful Rust port of `PerimeterGenerator.cpp:2781-2857`:
  - build `blocked: Vec<usize>` (in-degree) and `blocking: Vec<Vec<usize>>`
    (out-edges) from the constraint pairs (`:2782-2795`);
  - **initial cursor = the first junction of the FIRST INPUT LINE**
    (`lines[0].junctions[0].p`), falling back to `(0.0, 0.0)` **only when the
    input is empty** (`:2798-2799`:
    `all_extrusions.empty() ? Point::Zero() : all_extrusions.front()->junctions.front().p`).
    An earlier draft claimed the walk always starts at `(0,0)` and picks the
    line nearest the origin — **that is wrong; do not implement it**;
  - among unblocked candidates, iterate **open lines (`is_closed == false`)
    before closed ones** (`:2815-2818` sorts ascending on the `is_closed`
    bool). Distance is **not** a sort key — it is evaluated per-candidate
    inside that iteration order (`:2820-2842`);
  - after each emission, advance the cursor to the emitted line's first
    junction and decrement `blocked` for everything it was `blocking`;
  - break remaining ties by `original_index` ascending (a PnP determinism
    addition where OrcaSlicer leans on `std::sort` stability — record in
    D-157).

  Add the AC-4 unit test **plus** a safety-net test with 4+ lines and
  non-trivial constraints (per `design.md` §Context Cost Estimate).
- Precondition: Step 2 landed (`get_region_order` exists).
- Postcondition: AC-4 green; `reorder_by_region_order` is
  callable.
- Files allowed to read:
  `crates/slicer-core/src/arachne/region_order.rs` (the
  file from Step 2, load directly).
- Files allowed to edit (≤3):
  `crates/slicer-core/src/arachne/region_order.rs`,
  `crates/slicer-core/tests/region_order_tdd.rs`.
- Files out-of-bounds: the pipeline (Step 4a); the module (Step 4b).
- Expected sub-agent dispatches:
  - "Delegate OrcaSlicer `PerimeterGenerator.cpp:2781-2857`
    topological walk; return SUMMARY (≤200 words) + at most two 30-line
    SNIPPETs (the `blocked`/`blocking` adjacency build at `:2782-2795`, and
    the cursor init + candidate sort + greedy selection at `:2798-2842`)."
  - "Run `cargo test -p slicer-core --test region_order_tdd -- region_order_topological_walk_respects_constraints --exact`; FACT pass/fail."
- Context cost: `M` (the greedy walk + adjacency build +
  open-before-closed preference + 2 tests).
- Authoritative docs: none new.
- OrcaSlicer refs: `PerimeterGenerator.cpp:2781-2857` —
  delegate SUMMARY; never load.
- Verification: the 2 dispatches above.
- Exit condition: AC-4 green.

### Step 4a: Wire the region-order pass into the pipeline

- Gaps: G12.
- Objective: extend `crates/slicer-core/src/arachne/pipeline.rs`:
  - Add `ArachneParams::outer_to_inner: bool` to the struct def
    (`:75-168`) and the `Default` impl (`:170-219`), **default `false`**
    (OrcaSlicer's `wall_sequence` defaults to `InnerOuter`, which yields
    `is_outer_wall_first == false` — `PrintConfig.cpp:2084`,
    `PerimeterGenerator.cpp:2761-2766`).
  - Call `reorder_by_region_order(&mut lines, params.outer_to_inner)` after
    the `let lines: Vec<ExtrusionLine> = buckets.into_iter().flatten().collect();`
    at `:383` and before the `stitch_extrusions(lines, max_gap)` at `:390`.
  - Fix up any in-repo `ArachneParams { .. }` literal constructions that now
    miss the field (the compiler enumerates them).
- Precondition: Step 3 landed (`reorder_by_region_order` exists).
- Postcondition: `cargo check --workspace --all-targets` clean; AC-5's call
  ordering is in place. (AC-1/AC-6 close in Step 4b, once the module supplies
  the real `outer_to_inner`.)
- Files allowed to read:
  `crates/slicer-core/src/arachne/pipeline.rs:328-411` (the
  `run_arachne_pipeline` body, range-read).
- Files allowed to edit (≤3):
  `crates/slicer-core/src/arachne/pipeline.rs`,
  `crates/slicer-core/src/arachne/region_order.rs` (to add
  `pub use` re-exports if needed).
- Files out-of-bounds: the module (Step 4b), `path-optimization-default`,
  the scheduler, the IR/WIT boundary.
- Expected sub-agent dispatches:
  - "Run `cargo check --workspace --all-targets`; FACT pass/fail or SNIPPETS
    (compile error) on fail."
- Context cost: `M`.
- Authoritative docs: none new.
- OrcaSlicer refs: `PerimeterGenerator.cpp:2761-2766` (the polarity of
  `is_outer_wall_first`) — delegate; never load.
- Verification: the dispatch above.
- Exit condition: pipeline compiles with the new field + call in place.

### Step 4b: Derive `outer_to_inner` in the module + verify AC-6/AC-7/AC-8

- Gaps: G12.
- Objective: extend `modules/core-modules/arachne-perimeters/src/lib.rs`:
  - In `arachne_params_from_config` (`:130`), the `wall_sequence` config key
    is **already read** at `:269-272`. Derive the bool with the **correct
    polarity**:
    ```
    outer_to_inner = (wall_sequence == "OuterInner")
                  || (wall_sequence == "InnerOuterInner" && !is_initial_layer)
    ```
    matching `PerimeterGenerator.cpp:2761-2766` (Orca disables the sandwich
    mode's outer-first behavior on layer 0). **The earlier draft of this
    packet had this inverted** — it claimed default `true` "matching
    `InnerOuter`" and described `OuterInner` as "inner walls first". Both are
    backwards. The module's existing `wall_sequence_is_inner_outer` at
    `:269-272` already uses the correct polarity; the new derivation must
    agree with it, not contradict it.
  - Pass the resolved bool into `ArachneParams::outer_to_inner`.
  - **Then verify AC-8 explicitly**: confirm the new pre-module reorder and
    the pre-existing post-module
    `slicer_core::perimeter_utils::wall_sequence_reorder`
    (`perimeter_utils.rs:723`) do **not** both flip the outer/inner
    direction. If they do, STOP — raise it to the user; do not silently
    disable either pass (see `design.md` §Open Questions `[BLOCK]`).
- Precondition: Step 4a landed.
- Postcondition: AC-1 + AC-5 + AC-6 + AC-7 + AC-8 green.
- Files allowed to read:
  `modules/core-modules/arachne-perimeters/src/lib.rs:108-290`
  (the `arachne_params_from_config` body incl. the existing `wall_sequence`
  read, range-read); `crates/slicer-core/src/perimeter_utils.rs:723` +
  its surrounding fn (range-read, for the AC-8 check).
- Files allowed to edit (≤3):
  `modules/core-modules/arachne-perimeters/src/lib.rs`.
- Files out-of-bounds: the pipeline (Step 4a), `path-optimization-default`,
  the scheduler, the IR/WIT boundary, `perimeter_utils.rs` (read-only — AC-8
  requires it to stay unchanged).
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-runtime --test arachne_parity_round2 --
    arachne_parity_wall_region_order_odd_after_enclosing --exact`;
    return FACT pass/fail or SNIPPETS (fail with assertion + ≤20 lines)."
  - "Run `cargo test -p slicer-core --test wall_sequence_reorder_tdd`;
    FACT pass/fail (AC-8 — the pre-existing reorder must stay green)."
  - "Run `cargo test -p slicer-runtime --test arachne_parity`;
    return FACT pass/fail (AC-7 14-locks regression lock) or SNIPPETS
    (fail with assertion + ≤20 lines)."
  - "Run `cargo xtask build-guests --check`; FACT clean/STALE — the module
    source changed, so the guest WASM **will** be stale until rebuilt."
- Context cost: `M`.
- Authoritative docs: `docs/15_config_keys_reference.md` (load the
  `wall_sequence` entry directly).
- OrcaSlicer refs: `PerimeterGenerator.cpp:2761-2766`, `PrintConfig.hpp:187-192`
  (the `WallSequence` enum), `PrintConfig.cpp:2084` (the `InnerOuter` default)
  — delegate SUMMARY; never load.
- Verification: the 4 dispatches above.
- Exit condition: AC-1 + AC-5 + AC-6 + AC-7 + AC-8 green; guests rebuilt.

### Step 5: Doc updates + `cargo xtask build-guests --check` + final gates

- Gaps: G12.
- Objective: update
  `docs/18_arachne_parity_audit.md` Gap summary table to
  mark G12 closed; update the detailed-gap "PnP status"
  entry to "closed (this packet)"; add D-157 (region-order
  port) to `docs/DEVIATION_LOG.md`; add *region order* and
  *SparsePointGrid* glossary entries to `CONTEXT.md`; run
  `cargo xtask build-guests --check` to confirm the
  slicer-core changes did not break guest builds; run the
  final `cargo check --workspace --all-targets`,
  `cargo clippy --workspace --all-targets -- -D warnings`,
  and the 4 AC-grep checks from `packet.spec.md` §Doc
  Impact.
- Precondition: Step 4 landed.
- Postcondition: packet acceptance ceremony green; every
  AC green; every doc grep returns a hit; guest WASM
  fresh.
- Files allowed to read: `docs/18_arachne_parity_audit.md`,
  `docs/DEVIATION_LOG.md` (the D-105B/C/E entries only),
  `CONTEXT.md` (the current glossary section only).
- Files allowed to edit (≤3):
  `docs/18_arachne_parity_audit.md`,
  `docs/DEVIATION_LOG.md`, `CONTEXT.md`.
- Files out-of-bounds: source code (no changes in this
  step).
- Expected sub-agent dispatches:
  - "Run `cargo xtask build-guests --check`; return FACT
    clean / STALE."
  - "Run `cargo check --workspace --all-targets`; FACT
    pass/fail."
  - "Run `cargo clippy --workspace --all-targets -- -D
    warnings`; FACT pass/fail."
  - "Run each of the 4 doc-grep checks from
    `packet.spec.md` §Doc Impact; return FACT hit/no-hit
    for each."
  - "Run `cargo test -p slicer-core`; FACT pass/fail
    (final unit sweep)."
- Context cost: `S`.
- Authoritative docs: `docs/07_implementation_status.md`
  (delegate SUMMARY of the current M2 chain status; the
  implementer updates the M2 entry to mark this packet
  complete).
- OrcaSlicer refs: none.
- Verification: all 5 dispatches above.
- Exit condition: every AC green, every doc grep hits,
  clippy clean, guests fresh, `docs/07` updated.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | New `SparsePointGrid` struct + 2 methods + 2 tests + module registration. |
| Step 2 | M | `get_region_order` port + 5 tests + the fixture constraint-count check. |
| Step 3 | M | `topological_walk` + `reorder_by_region_order` wrapper + 2 tests. |
| Step 4a | M | Pipeline: new `ArachneParams` field + the reorder call. |
| Step 4b | M | Module: `outer_to_inner` derivation + AC-6/AC-7/AC-8 verification + guest rebuild. |
| Step 5 | S | Doc updates + final gates. |

Aggregate: **M**. Largest single step: M. **No step is L.** The original
plan's L-rated Step 4 has been split into 4a (pipeline) and 4b (module),
which have a clean seam: 4a's exit is a compiling pipeline with the field and
call in place; 4b supplies the real config-derived value and runs the
regression sweep. The claim that "this step cannot be split without losing
the pipeline + module coupling" was wrong — the coupling is a single `bool`.
**No extended-band run is required.**

## Packet Completion Gate

- All 6 steps complete (1, 2, 3, 4a, 4b, 5).
- Every step exit condition is met.
- Packet acceptance criteria green (each verification
  command dispatched and returned PASS).
- `docs/07_implementation_status.md` updated for the M2
  chain (via worker dispatch — never edited by loading
  the full backlog into the implementer's context).
- `docs/18_arachne_parity_audit.md` Gap summary table
  updated.
- `docs/DEVIATION_LOG.md` D-157 entry added.
- `CONTEXT.md` glossary entries added.
- `packet.spec.md` ready to move to
  `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion
  command from `packet.spec.md` (AC-1 through AC-8 +
  AC-N1 through AC-N5 = 13 commands).
- Confirm packet-level verification commands are green
  (the 3 gate commands in `packet.spec.md` §Verification).
- Record any remaining packet-local risk explicitly before
  moving to `status: implemented`. Known residuals (all must appear in
  D-157): the `original_index` tie-break is a PnP determinism addition
  (OrcaSlicer relies on `std::sort` stability); the `SparsePointGrid` stays
  in `crates/slicer-core::arachne` rather than being promoted to
  `crates/slicer-helpers`, and is monomorphised on one payload type rather
  than templated; wall-sequence direction is now resolved in two places in
  `slicer-core` (the new pre-module pass and the existing post-module
  `perimeter_utils::wall_sequence_reorder`) with the composition argument
  recorded per AC-8.
- Confirm the implementer's peak context usage stayed within the **standard**
  band (≤150k). The packet is M and no step is L, so **no extended-band
  escalation is expected**; if one occurs, log it as a packet-authoring
  lesson.
