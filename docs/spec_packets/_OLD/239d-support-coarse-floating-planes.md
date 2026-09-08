---
status: implemented
packet: 239d-support-coarse-floating-planes
task_ids:
  - TASK-523
  - TASK-524
  - TASK-525
  - TASK-526
  - TASK-527
  - TASK-528
  - TASK-529
  - TASK-530
---

# 239d-support-coarse-floating-planes

## Goal

Deliver free-floating support **stacks** in the coarse direction: when the support pitch
(`support_layer_height_mm`) is >= the object layer pitch, both support planners generate the
support stack at pitch spacing between the brackets of each `(object_id, region_id)`
contiguous run — brackets selected by the measured Q1 rule (interface-role planes across
body-bearing interface spans only; otherwise the run's surviving support-bearing endpoint
fallback) — the **traditional** family following `raft_and_intermediate_support_layers`
(`Support/SupportMaterial.cpp`) stepping (`ceil((dist - EPSILON) / pitch)`, `step = dist / n`,
last plane aligned to the upper bracket) and the **tree** family following
`plan_layer_heights` (`TreeSupport.cpp`) stepping (`ceil(dist / pitch)` for main-body
spacing, **no** EPSILON bias, `step = dist / n`) — plus the `generate_support_layers`
(`Support/SupportCommon.cpp`) EPSILON candidate-grouping/midpoint rule, replacing the 239c
grid-bound degeneration — so a real nominal 0.45-pitch slice of `SupportTest.stl` emits off-grid
support rows that each extrude, with the disabled flag reproducing the baseline exactly and
the finer direction unregressed.

## Problem Statement

Packet 239c (implemented) made support Z independent of the object grid, but only in the
FINER direction. Measured 2026-08-31 on
`crates/slicer-runtime/tests/fixtures/support-family/SupportTest.stl` with the tracked
config, flag true:

- `support_layer_height_mm = 0.1` over `layer_height` 0.2 → 273 distinct `;Z:`, 123
  extruding off-grid rows (finer direction works).
- `support_layer_height_mm = 0.3` over `layer_height` 0.2 → 150 distinct `;Z:`, **0**
  off-grid rows (coarse direction degenerates to the object grid).
- `support_layer_height_mm = 0.3` over `layer_height` 0.1 → 299 distinct `;Z:`, 0 off-grid
  rows; family-labeled support rows (per the TASK-523 record): normal(auto) 85 of 299 rows,
  tree(auto) exploratory run 248 rows — in both families the grid decimation did NOT cut
  support to ~every 3rd row.

Root cause: the 239c intermediate-plane derivation (`packet239c_intermediate_planes` in both
planners) brackets consecutive support rows, which sit at object-grid spacing; when the
support pitch >= the object gap, `n_layers_extra == 1` and the stack stays grid-bound.
The 239d bracket selection measured 2026-09-02 against the real slice adds a second
degeneration: interface roles are per-layer flags, not spanning markers, and on this fixture
they appear only in adjacent top-row clusters — tree interface roles at exactly
`anchor_z` 246000/248000, traditional at 244000/246000/248000 (`[DEBUG-239D-BRACKETS]`
records in `target/test-output.log`; all surviving-row gaps 2000 units; every consecutive
interface pair encloses zero strictly interior non-interface rows). A rule that uses a bare
interface-plane count as the bracket set therefore brackets intervals with no
body-bearing support rows, and the stack has nothing between its brackets to replace.
Canonical OrcaSlicer does not degenerate: `raft_and_intermediate_support_layers`
(`Support/SupportMaterial.cpp`) brackets the sorted `extremes` — the top/bottom contact
layers, which span many object layers — and fills between consecutive ones at
`step = dist / n_layers_extra ≈ pitch`, free-floating relative to the object grid. That is
the user-visible speed purpose of the toggle ("supports are waste material; print them
coarser"). 239c delivered free-floating planes; 239d must deliver free-floating STACKS in
the coarse direction, bracketing intervals that actually carry support body: interface
planes only across body-bearing interface spans, with the run's surviving support-bearing
endpoints as the fallback brackets when no such span exists.

The original 0.3 mm slice is retained as historical baseline evidence. The binding nominal
acceptance is `support_layer_height_mm = 0.45`: the supplied human-generated Orca references
use a 0.5 mm nozzle with `max_layer_height = 0.45` and measure approximately 0.447273 mm
effective coarse spacing; the packet-specific PnP evidence uses the explicit 0.45 override.

The decimation question is answered by measurement: `build_emit_schedule`
(`crates/slicer-core/src/algos/support_geometry.rs`) gates the host-side `SupportGeometryIR`
only, and both planners ignore `SupportGeometryView` on the meshed-object planner path (the
tree's sole read is the mesh-less legacy contact fallback in the tree planner's
`SupportPlanner::plan_for_object` — a genuinely mesh-less object with no contacts; the
traditional planner's `_support_geometry` parameter at its `run_support_geometry_with_analysis`
entry is never read) — so the host decimation never reaches the meshed-object planner path
(family-labeled: normal(auto) support on 85/299 rows, tree(auto) exploratory run 248 rows,
despite the decimation). Only the traditional
planner decimates, via `support_step = round(pitch / gap)`, which is on-grid. The coarse
rows must therefore come from a new floating stack, not from either decimation mechanism.

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

- **`AnchoredGeometryContract::COORDINATE_TOLERANCE_UNITS` = 10 units = 1e-3 mm** is the
  single on-grid/off-grid discriminator used by both the planner (deciding whether a derived
  plane is off-grid) and the renderer (deciding whether to take the anchored route). Do not
  introduce a second epsilon.
- **Config keys are snake_case in Rust, always.** `config.get("support_layer_height_mm")`,
  never `"support-layer-height-mm"`. Manifest section headers are already snake_case.
- **No schema/version constant moves.** `CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION`
  (`crates/slicer-ir/src/slice_ir.rs`) is not bumped; no `SupportPlanIR` version moves; no
  field is added to `SupportPlanEntry` — the coarse stack is expressed entirely through
  `anchor_z` values. No version literal is frozen here on purpose: re-derive it from the
  constant at the moment you need it.
- **239c's locked invariants carry over.** `anchor_z` is the declared support print plane and
  the only Z authority a support renderer may consult; the disabled branch is bit-for-bit
  the pre-change behaviour; the on-grid/off-grid discriminator is
  `COORDINATE_TOLERANCE_UNITS`; the `support_layer_height_mm == 0.0` sentinel means "object
  pitch" (239c [FWD] option b). This packet extends the enabled branch only.

## Data and Contract Notes

- **IR/manifest contracts.** No IR shape changes; no manifest changes. `SupportPlanEntry`
  keeps its live field set (body membership carried by `body_ids: Vec<String>`; no entry
  `id` field); the coarse stack is expressed through `anchor_z` values and the
  existing `anchor_layer_index` (the nearest object layer). The keys
  (`independent_support_layer_height`, `support_layer_height_mm`) are already declared on
  both planner manifests with byte-identical `type`/`default` (the
  `ConfigBoundsIndex::from_modules` intersection requires it).
- **WIT boundary.** None crossed. The anchored transport is 239b's; the host seam is 239a's;
  the renderer emission is 239c's. Editing any of them is out of bounds.
- **Determinism/scheduler constraints.** The coarse derivation must be a pure function of
  the entries plus config, never of iteration order or hash-map traversal. Per object, the
  emitted entry sequence must be **nondecreasing in `anchor_z`** in the planner's original
  output order, and the **distinct** `anchor_z` planes must be **strictly increasing** —
  equal-`anchor_z` entries within an object (different `region_id` or different source
  entry, distinguishable by the identity key's `ordered body_ids` component) are the only
  repetition allowed, and duplicate identity keys
  `(source global_layer_index, object_id, region_id, ordered body_ids, anchor_z)` are
  prevented at insertion (see the
  duplicate-key rule above). A strict per-entry increase is **not** required or asserted.
  The synthetic `global_layer_index` scheme (tree `i32::MIN + ordinal`, deduped per plane via
  `intermediate_plane_indices`) is inherited from 239c and must not introduce a second
  ordering authority.
- **Decimation facts (measured, not assumed).** `build_emit_schedule` gates only the
  host-side `SupportGeometryIR`; both planners ignore `SupportGeometryView` on the
  meshed-object planner path (the tree's only read is the mesh-less legacy contact fallback,
  tree `lib.rs` ~2169-2173; the traditional parameter, `lib.rs` ~174, is never read). The
  traditional `support_step` decimation is on-grid. Neither can produce free-floating rows;
  the floating stack is the coarse-row mechanism.

## Locked Assumptions and Invariants

- **Locked:** `anchor_z` is the declared support print plane, in canonical units, and is the
  only Z authority a support renderer may consult (239c).
- **Locked:** the disabled branch is bit-for-bit the pre-change behaviour. AC-N1 is the
  falsifier; it compares against a baseline captured **before** any planner edit (Step 1).
- **Locked:** the `support_layer_height_mm == 0.0` sentinel means "object pitch" (239c [FWD]
  option b). AC-N3 is the falsifier.
- **Locked:** the finer direction (configured nonzero pitch < `local_support_gap`) is
  unchanged. AC-N2 (the 239c AC-1 test) is
  the falsifier.
- **Locked (coarse/finer selection is bracket-local, with one binding predicate).** For
  each consecutive demanded bracket pair, `local_support_gap` is the maximum positive
  anchor-Z difference between consecutive surviving support-bearing rows of that same
  `(object_id, region_id)` contiguous run covered by the bracket (rows already available
  to both planner callers). The bracket takes the coarse path iff the configured nonzero
  pitch >= `local_support_gap`, compared in exact canonical units with
  `AnchoredGeometryContract::COORDINATE_TOLERANCE_UNITS` as the only
  tolerance if one is needed (no new epsilon); otherwise the bracket retains the 239c
  finer derivation. The decision is made **per bracket pair**. It is never
  decided from the first or contact layer height alone: two runs of the same object can
  resolve differently (one coarse, one finer) when their covered surviving-row gaps differ,
  and a bracket pair whose `local_support_gap` exceeds the configured pitch (e.g. pitch 0.2
  over covered surviving-row gaps of 0.3, even when the object's first/base layer gap is
  0.2) keeps the
  239c
  finer behaviour even when the global pitch >= the object's base layer pitch.
- **Locked:** the coarse stack follows the canonical `dist/n` stepping between consecutive
  bracket planes (interface brackets across body-bearing interface spans, or the D1
  endpoint fallback), with the canonical grouping/midpoint rule applied
  (user decision 2026-08-31).
- **Locked:** the on-grid/off-grid discriminator is
  `AnchoredGeometryContract::COORDINATE_TOLERANCE_UNITS` (10 units = 1e-3 mm), in both
  planner and renderer (239c).
- **Locked (Q1-Q3, binding):** bracket selection is per `(object_id, region_id)` contiguous
  run: interface-role planes bracket only when consecutive ones enclose a body-bearing
  interface span (at least one strictly interior surviving support-bearing row with no
  interface role); otherwise the run's first/last surviving support-bearing rows are the
  fallback brackets, sorted/deduped by `anchor_z` (a run with no body-bearing interface
  span — including zero/one interface plane or an adjacent interface cluster confined to a
  run boundary — uses the endpoint fallback); stack planes select the nearest surviving
  at-or-below source row within the run, prefer non-interface geometry per `body_ids`
  membership, retain otherwise-unmatched memberships, and rewrite cloned roles to
  `SupportBody`, capturing the source `global_layer_index` into the local duplicate key and
  clone-source provenance decision only, assigning the emitted entry's final
  `global_layer_index` from the per-plane DEV-163 synthetic identity map
  (`BTreeMap<i64, i32>` — one plane identity shared by all entries at a synthesized plane),
  and preserving other provenance fields;
  `support_step` neutralization lets entries inside the computed coarse bracket-range
  membership bypass the modulo/removal gate per coarse bracket
  pair only. See §Code Change Surface.
- **Locked (family-specific stepping):** the traditional stack uses
  `raft_and_intermediate_support_layers` (`Support/SupportMaterial.cpp`)
  `ceil((dist - EPSILON) / pitch)` with the EPSILON bias; the tree stack uses
  `plan_layer_heights` (`TreeSupport.cpp`) `ceil(dist / pitch)` without it. One shared
  formula for both families is a spec defect, not a simplification.
- **Locked (binding coarse/finer predicate):** for each consecutive demanded bracket
  pair, `local_support_gap` = the maximum positive anchor-Z difference between consecutive
  surviving support-bearing rows of that same `(object_id, region_id)` contiguous run
  covered by the bracket; coarse path iff configured nonzero pitch >= `local_support_gap`
  (exact canonical units, `AnchoredGeometryContract::COORDINATE_TOLERANCE_UNITS` as the
  only tolerance if needed — no new epsilon); otherwise the 239c finer derivation is
  retained for that bracket. AC-N4 pins the concrete case: configured pitch 0.2 over
  covered surviving-row gaps 0.3 stays finer even when the object's first/base layer gap is
  0.2.
- **Locked (ordering):** entries are nondecreasing in `anchor_z` per object in original
  output order; distinct planes strictly increasing; identity key
  `(source global_layer_index, object_id, region_id, ordered body_ids, anchor_z)` unique.
- **Locked (anchoring):** each synthesized plane's `anchor_layer_index` is the true-nearest
  object layer by absolute Z distance to the plane's `anchor_z`, with the lower
  `anchor_layer_index` winning a tie deterministically.
- **Locked by measurement, not by assumption:** whether `DefaultGCodeEmitter::emit_gcode`
  mis-scales a nominal coarse 0.45-pitch off-grid pass. Nothing in this packet may state a
  flow figure or a verdict that the Step 5 record does not contain.

## Risks and Tradeoffs

- **The bracket selection is the design's crux.** The tree planner's interface roles are
  per-node/per-layer (Roof/Floor/Base counters seeded at contact creation), not per
  contiguous run; the measured real slice's interface roles sit only in adjacent top-row
  clusters (tree 246000/248000, traditional 244000/246000/248000) that span no
  body-bearing interval. The Q1 binding decision resolves this: partition by
  `(object_id, region_id)` contiguous run; interface-role planes bracket only when
  consecutive ones enclose a body-bearing interface span; a run with no such span —
  including an adjacent interface cluster confined to a run boundary — falls back to the
  run's first/last surviving support-bearing rows. The fallback is specified from measured
  evidence, so the stack can no longer silently stay grid-bound for cluster-only or
  interface-less regions, and genuine interface entries are protected real entries —
  never removed or demoted to body.
- **The body-row replacement touches the source geometry and membership.** The Q2 binding
   decision selects the nearest at-or-below source row within the current run, prefers
   non-interface geometry per `body_ids` membership, retains memberships with no body-only
   alternative, rewrites roles to `SupportBody`, and assigns the emitted final
   `global_layer_index` from the per-plane DEV-163 synthetic identity map. A wrong
   implementation would clone the lower bracket at every height, drop a distinct membership
   on a mixed interface/body source row, or leave interface roles on a body plane;
   AC-2/AC-3 assert source geometry and role rewrite directly.
- **The `support_step` neutralization has a blast radius.** The traditional planner's tests
  pin the decimation behaviour; the Step 3 dispatch inventories them before editing. Widening
  a tolerance to make a change pass is gaming the gate and is forbidden.
- **The human gate is signed.** The supplied reference files use a 0.5 mm nozzle with
   `max_layer_height = 0.45` and measure approximately 0.447273 mm effective spacing; the
   existence command is `REFS-PRESENT`. The human gate owner approved the nominal artifacts
   and all six checklist items on 2026-09-02.
- **Guest staleness.** Every step here edits guest-feeding paths, so essentially every
  failure in this packet is a stale-guest suspect until `cargo xtask build-guests --check`
  returns exit `0`.
- **The E=0 defect class is a test assertion, not a gate finding.** AC-1 asserts extrusion
  presence on every off-grid support row; the 239c artifact regression ("it renders
  nothing") was caught only by human-gate inspection, and this packet must not repeat that.
