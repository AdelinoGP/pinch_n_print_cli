# ADR-0028 — `run_infill_postprocess` Takes Prior `InfillIR` as Input; `PerimeterRegionView` Carries the Partitioned Fill Polygons

## Status

Proposed (lands with the infill-parity effort; companion to ADR-0025).

## Context

The `Layer::InfillPostProcess` stage exists in `STAGE_ORDER`
(`crates/slicer-scheduler/src/execution_plan.rs:33`) and the
`run_infill_postprocess` trait hook exists
(`crates/slicer-sdk/src/traits.rs:374-393`), but the stage is currently a
no-op for all shipping modules. Two contract gaps make it unusable as the home
for the infill linker (ADR-0025):

**Gap 1 — the builder is empty and the commit replaces.**
`crates/slicer-wasm-host/src/dispatch.rs:435-454` creates a **fresh empty**
`InfillOutputBuilder` for `Layer::InfillPostProcess` — it does not pass the
paths already emitted by `Layer::Infill`. And
`crates/slicer-runtime/src/layer_executor.rs:1151-1156`
(`LayerStageCommit::InfillPostProcess`) **discards** the existing `InfillIR`
(`take_infill()` with `_`) and replaces it wholesale with whatever the
post-process module emits. So a post-process module cannot read what
`Layer::Infill` emitted, and its output replaces the prior stage's work
entirely. A linker that needs to read raw segments and emit linked polylines is
impossible under this contract.

**Gap 2 — `PerimeterRegionView` lacks the partitioned fill polygons.**
`run_infill_postprocess` receives `&[PerimeterRegionView]`
(`crates/slicer-sdk/src/traits.rs:388`). `PerimeterRegionView`
(`crates/slicer-sdk/src/views.rs:490+`) carries `wall_loops`, `infill_areas`
(the raw wall-inset polygon, pre-partition), `seam_candidates`, and
`resolved_seam` — but NOT `sparse_infill_area`, `top_solid_fill`,
`bottom_solid_fill`, or `bridge_areas`. Those four partitioned polygons live
only on `SliceRegionView` (views.rs:19-483), which `run_infill` receives but
`run_infill_postprocess` does not. A linker that needs to re-clip connected
paths against the partitioned boundary cannot see the boundary.

The grilling (2026-07-01) confirmed the project owner wants the cross-module
post-pass linker to be real, not deferred. Both gaps must close.

## Decision

Two coordinated contract changes, both required for the infill-linker to
function:

### Change 1 — `run_infill_postprocess` receives the prior `InfillIR`

The `run_infill_postprocess` signature changes so the hook can read the paths
`Layer::Infill` already emitted. Two implementation options (pick at
implementation time; the WIT change is the same either way):

**Option 1a (pre-populated builder):** The host pre-populates the
`InfillOutputBuilder` passed to `run_infill_postprocess` with the prior
`InfillIR`'s `sparse_paths` / `solid_paths` / `ironing_paths`. The module reads
them from the builder, transforms them, and pushes the result back. The
builder becomes a read-write surface. Smaller WIT change (no new parameter; the
builder just arrives non-empty). Muddies the "builder is write-only" semantics
but is the smallest diff.

**Option 1b (new input parameter):** `run_infill_postprocess` gains a new
`prior_infill: infill-ir` (or `list<extrusion-path3d>`) input parameter,
alongside `output: infill-output-builder`. The builder stays write-only. The
module reads prior paths from the input, writes linked paths to the output.
Cleaner semantics; larger WIT change (new parameter on the export).

The implementation picks one. Both require a WIT schema bump on
`world-layer.wit` and a `SliceIR`/`InfillIR` schema version bump.

### Change 2 — `PerimeterRegionView` carries the four partitioned fill polygons

`PerimeterRegionView` (`crates/slicer-sdk/src/views.rs:490+`) gains four
`Vec<ExPolygon>` fields mirroring `SliceRegionView`:
- `sparse_infill_area`
- `top_solid_fill`
- `bottom_solid_fill`
- `bridge_areas`

The host populates them at dispatch time
(`crates/slicer-wasm-host/src/dispatch.rs:435-454`) by copying from the
corresponding `SliceIR` region. The `perimeter-region-view` WIT resource
(`crates/slicer-schema/wit/deps/ir-types.wit`) gains the four fields. The
`PerimeterRegionViewBuilder` test fixture
(`crates/slicer-sdk/src/test_support/fixtures.rs`) gains setters for them.

### Change 3 — `LayerStageCommit::InfillPostProcess` merges, not replaces

`crates/slicer-runtime/src/layer_executor.rs:1151-1156` changes from
"discard-and-replace" to "merge" — the linker's emitted paths merge into the
existing `InfillIR` (or, under Option 1a where the builder was pre-populated,
the linker's output *is* the merged set, so replace is correct). The exact
merge semantics depend on Option 1a vs 1b and are decided at implementation.

## Consequences

**Positive**:
- The `Layer::InfillPostProcess` stage becomes usable. The infill-linker module
  (ADR-0025) can read prior-stage raw segments and the partitioned fill
  polygons, and emit linked polylines.
- The `run_infill_postprocess` hook graduates from no-op to load-bearing. Any
  future `Layer::InfillPostProcess` module (not just the linker) benefits from
  reading prior output.

**Negative**:
- **WIT schema bump triggers full guest-rebuild** (`cargo xtask
  build-guests`). Every guest's bindgen regenerates. Per `CLAUDE.md` WIT/Type
  Changes Checklist: search all `wit_host.rs`, `dispatch.rs`, and `wit_guest`
  modules for affected types; verify type identity across component boundaries;
  run `cargo build --tests` after WIT changes.
- **Every exhaustive match on `PerimeterRegionView` gains fields.** The blast
  radius is ~30 files (surveyed via grep): `slicer-sdk`, `slicer-wasm-host`,
  `slicer-macros`, `slicer-runtime` tests, `test-guests`, and several
  `core-modules`. Each must add the new fields to construction and (if
  exhaustive) to match arms. This is the standard schema-bump pattern
  (ADR-0002, ADR-0009, ADR-0010 all paid it).
- **`InfillIR` schema version bump** (minor, additive) for the prior-IR input
  path if Option 1b is chosen.
- **Test fixtures gain fields.** `PerimeterRegionViewBuilder` and every test
  that constructs a `PerimeterRegionView` must populate the new fields (or
  default them empty). Existing tests that don't care about fill polygons can
  leave them empty.

**Trade-offs we explicitly accept**:
- The schema bump cost is real (~30 files, full guest rebuild) but is the
  standard pattern. It is not a reason to defer the contract change. Deferring
  means the linker cannot exist, which means Architecture A cannot ship.
- The "builder is write-only" semantics (Option 1a) vs "new input parameter"
  (Option 1b) is an implementation detail. Both are valid; the implementation
  picks based on which produces cleaner host-dispatch code. The WIT change is
  the same shape either way (one new field or one new param).

## Future-Reviewer Notes

- **Do not add fields to `PerimeterRegionView` that are not needed by a concrete
  `Layer::InfillPostProcess` consumer.** The four fill polygons are needed by
  the infill-linker. Future "nice to have" fields (e.g. `overhang_areas` on the
  post-process view) wait for a concrete consumer.
- **Do not split `run_infill_postprocess` into separate "read" and "write"
  hooks.** One hook that reads prior and writes new is the right granularity.
  Splitting introduces ordering complexity with no benefit.
- **The `InfillPostProcess` commit-merge (Change 3) is not the same as the
  `Infill` commit-merge** (`layer_executor.rs:1139-1150`, which merges multiple
  `Layer::Infill` modules' disjoint outputs). The post-process merge is
  "linker output supersedes the raw segments it linked." If this proves
  confusing, a future ADR can separate the two commit semantics explicitly.

## References

- `docs/adr/0025-infill-linker-as-raw-emit-post-pass.md` — Architecture A (why the linker needs this).
- `docs/adr/0026-infill-linking-algorithms-in-linker-module.md` — algorithm home.
- `crates/slicer-scheduler/src/execution_plan.rs:33` — `Layer::InfillPostProcess` in `STAGE_ORDER`.
- `crates/slicer-sdk/src/traits.rs:374-393` — `run_infill_postprocess` trait hook.
- `crates/slicer-wasm-host/src/dispatch.rs:435-454` — current dispatch (empty builder).
- `crates/slicer-runtime/src/layer_executor.rs:1139-1156` — `Infill` merge vs `InfillPostProcess` replace.
- `crates/slicer-sdk/src/views.rs:490+` — `PerimeterRegionView` (lacks fill polygons).
- `crates/slicer-sdk/src/views.rs:19-483` — `SliceRegionView` (has the four fill polygons).
- `crates/slicer-schema/wit/deps/world-layer/world-layer.wit:25` — WIT signature.
- `crates/slicer-schema/wit/deps/ir-types.wit` — `perimeter-region-view` resource (target of the field addition).
- `CLAUDE.md` "WIT/Type Changes Checklist" — rebuild ceremony.
- `docs/adr/0002-wit-marshalling-type-unification.md` — prior WIT schema bump precedent.