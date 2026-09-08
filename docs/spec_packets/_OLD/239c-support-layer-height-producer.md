---
status: implemented
packet: 239c-support-layer-height-producer
supersedes: 239-support-independent-layer-z
task_ids:
  - TASK-515
  - TASK-516
  - TASK-517
  - TASK-518
  - TASK-519
  - TASK-520
  - TASK-521
  - TASK-522
---

# 239c-support-layer-height-producer

## Goal

Declare `independent_support_layer_height`, decouple support-layer Z from the object layer
grid inside `tree-support-planner` / `traditional-support-planner` and their two renderers,
emit the resulting off-grid support work through the anchored path established by
`239a-anchored-host-seams` and `239b-anchored-wit-contract`, and settle the measure-first
`height_delta` flow verdict — so a real slice of `SupportTest.stl` produces support print
rows at Z values the object layer plan does not contain.

## Problem Statement

PnP has no support-layer Z independent of object-layer Z. Gap-register row `G-02` has named
this since the support-family audit, and the superseded packet `239-support-independent-layer-z`
tried to close it host-side only. A `/swarm` run on 2026-08-28 measured 239's central premise
as false and split it into three packets
(`docs/specs/support-independent-layer-z-split-plan.md`). Two of its nine findings are this
packet's problem statement:

- **F8 — support Z is structurally grid-bound.** `modules/core-modules/tree-support/src/lib.rs`
  and `modules/core-modules/traditional-support/src/lib.rs` both emit via `let z = region.z()`
  inside `run_support`. `modules/core-modules/tree-support-planner` reads
  `layer_plan.layers[layer_rev].z` in `SupportPlanner::plan_for_object` and takes heights from
  `layer.effective_layer_height` / `layer_plan.layers[0].effective_layer_height` as
  `nominal_layer_height`. `LayerPlanView` is the single Z authority and **no module has any
  concept of a support-specific layer height**.
- **The consequence for 239a and 239b.** Both dependency packets state honestly that no real
  slice exercises their paths, because nothing constructs anchored work. Their closure rests on
  hand-built plans and purpose-built test guests. Without a producer, slicing
  `crates/slicer-runtime/tests/fixtures/support-family/SupportTest.stl` yields structurally
  identical output to today's — vacuous evidence (E1).

The exact gap the superseded 239 missed: it put the module surfaces out of bounds, so it could
never make a real slice behave differently, and its AC-1 was unprovable. This packet moves the
boundary to the modules, where the Z authority actually lives.

Two further facts shape the slice. First, `independent_support_layer_height` does not exist
anywhere in the tree (verified: zero matches across `*.rs`, `*.toml`, `*.md`). Second, a
neighbouring key **does** exist and is not the same thing: `support_layer_height_mm` is
declared on both `*-support-planner` manifests and typed in
`declare_resolved_config!` (`crates/slicer-ir/src/resolved_config.rs`), with `0.0` meaning
"use the object's effective layer height". It **decimates the object grid** — a 0.4 mm support
height over 0.2 mm model layers emits support at object layers `{1,3,5}` — so it selects a
*subset* of grid planes and never produces an off-grid Z. `build_emit_schedule`
(`crates/slicer-core/src/algos/support_geometry.rs`) is that decimation. The new key is the
gate that lets the planner leave the grid entirely; the existing key remains the height value.

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

- **`AnchoredGeometryContract::COORDINATE_TOLERANCE_UNITS` = 10 units = 1e-3 mm** is the single
  on-grid/off-grid discriminator used by both the planner (deciding whether a derived plane is
  off-grid) and the renderer (deciding whether to take the anchored route). Do not introduce a
  second epsilon.
- **Config keys are snake_case in Rust, always.** `config.get_bool("independent_support_layer_height")`,
  never `"independent-support-layer-height"`. Manifest section headers are already snake_case.
- **One module, one stage.** `crates/slicer-scheduler/src/manifest.rs` reads a single required
  `stage.id` per manifest (`required_stage`, validated against `known_stage_ids`). A module
  cannot serve two stages, which is why the anchored drain must be reachable from the
  renderers' existing `Layer::Support` context rather than by adding a second stage export to
  the same crate. **This is settled, not open:** `239b-anchored-wit-contract` widens
  `crates/slicer-schema/wit/deps/layer-support/layer-support.wit`'s `run` with
  `collection: layer-collection-builder`, so a `Layer::Support` guest receives the builder
  directly and `LayerModule::run_support` (`crates/slicer-sdk/src/traits.rs`) carries a
  `&mut LayerCollectionBuilder`. See §Open Questions `[RESOLVED]`.
- **Schema/version constants.** No version constant is bumped by this packet.
  `CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION` (`crates/slicer-ir/src/slice_ir.rs`) **is not
  bumped by this packet** — it keeps whatever value the live constant carries at activation, and
  `docs/02_ir_schemas.md` must continue to agree with it — and no `SupportPlanIR` version moves.
  No version literal is frozen here on purpose: re-derive it from the constant at the moment you
  need it. If a later reviewer
  believes a bump is needed, that is a scope change, not a detail: the bump and its full
  test fallout would have to land in one step, and this packet's design deliberately avoids
  the situation by transporting no new field.
- **`GCodeEmitter::emit_gcode` signature is frozen.** It has many impls and many call sites
  spread across the test crates, with a single production impl (`DefaultGCodeEmitter`) and a
  single production call site (`slicer_runtime::postpass::execute_postpass_with_capture`). No
  count is quoted: the totals are mutable shared state that rot as test crates grow. If you need
  a count, re-derive it (`rg -n 'impl GCodeEmitter for' crates/`,
  `rg -c '\.emit_gcode\(' crates/ -g '*.rs'`) — the signature being frozen is the constraint, not
  any particular number. Off-grid rows arrive as ordinary `LayerCollectionIR` per 239a.

## Data and Contract Notes

- **IR/manifest contracts.** No IR shape changes. `SupportPlanEntry.anchor_z` changes
  *meaning-in-practice* from "a copy of the object layer Z" to "the declared support print
  plane", which its existing doc comment already permits; the doc comment is tightened in the
  same step so the semantics are written down. Two manifest `[config.schema]` tables are added.
  `ConfigBoundsIndex::from_modules` intersects bounds across every module declaring a key —
  both declarations must therefore be byte-identical in `type` and `default`, or the
  intersection is a silent behaviour difference between families.
- **WIT boundary.** None crossed by this packet. The anchored transport is 239b's; the anchored
  host seam is 239a's. Editing either is out of bounds.
- **Determinism/scheduler constraints.** Declared planes must be deterministic and strictly
  increasing per object, independent of module execution order; the plane derivation must be a
  pure function of `LayerPlanView` plus config, never of iteration order or of a hash map's
  traversal. 239a's row-synthesis ordering and its serial/parallel identity guarantee are what
  make the off-grid rows deterministic downstream; this packet must not introduce a second
  ordering authority.
- **Support-family claims.** Both planners hold `support-family:*` claims, and
  `crates/slicer-scheduler/src/execution_plan.rs` injects `SUPPORT_GENERATOR_CONFIG_KEY` /
  `SUPPORT_FAMILY_CONFIG_KEY` into their views regardless of declaration. The new key is **not**
  one of those and must be declared explicitly on both manifests, which is precisely what AC-N3
  guards.

## Locked Assumptions and Invariants

- **Locked:** `anchor_z` is the declared support print plane, in canonical units, and is the
  only Z authority a support renderer may consult. `region.z()` must not be used to place
  support extrusions after this packet.
- **Locked:** the on-grid/off-grid discriminator is
  `AnchoredGeometryContract::COORDINATE_TOLERANCE_UNITS` (10 units = 1e-3 mm), in both planner
  and renderer.
- **Locked:** the disabled branch is bit-for-bit the pre-change behaviour. AC-N1 is the
  falsifier; it compares against a baseline captured **before** any planner edit.
- **Locked by measurement, not by assumption:** whether `DefaultGCodeEmitter::emit_gcode`
  mis-scales an off-grid pass. Nothing in this packet may state a flow figure or a verdict that
  the Step 5 record does not contain.
- **Not locked:** the support layer *height* representation. It is derived from consecutive
  declared planes and can be promoted to a transported field by a later packet without
  contradicting anything asserted here.

## Risks and Tradeoffs

- **Both dependencies must land first.** This packet has the deepest dependency footprint of
  the three-packet split. If either 239a or 239b changes its exported surface during
  implementation, Step 4 is the step that breaks; the confirmation dispatch above exists to catch
  that before any module edit. The specific surface Step 4 depends on is 239b's **two-builder**
  `layer-support` `run` and the matching `LayerModule::run_support`
  (`crates/slicer-sdk/src/traits.rs`) signature. A narrowing of that signature after 239b closes
  would be a cross-packet regression to raise against 239b, not a local fallback to take here —
  there are no fallbacks left (§Open Questions `[RESOLVED]`).
- **Changing `anchor_z`'s effective value is a behaviour change for existing consumers.** Any
  code that today treats `anchor_z` as interchangeable with the object layer Z becomes wrong on
  the enabled branch. Step 2 must sweep for `anchor_z` readers before editing; at authoring
  time the only readers found were the planners' own writers, which is exactly why this seam is
  cheap — but that is a ledger fact and must be re-derived.
- **The human gate cannot close today.** Both reference files are verified absent
  (`REFS-ABSENT-GATE-OPEN`), and only a human can produce them. The packet reaches "all steps
  complete, sign-off pending" and stops there; that is the designed outcome, not a failure.
- **Trap T11.** The pre-existing references were sliced with the feature disabled and cannot
  measure this gap. The temptation to reuse them — and to requote the VOID "205 vs 150" figure —
  is the single most likely way this packet produces a false parity claim.
- **Guest staleness.** Every step here edits guest-feeding paths, so essentially every failure
  in this packet is a stale-guest suspect until `cargo xtask build-guests --check` returns
  exit `0`.
- **Emitter blast radius on the fix branch.** Two tests bind the current E formula tightly.
  Widening either tolerance to make a change pass would be gaming the gate and is forbidden.
