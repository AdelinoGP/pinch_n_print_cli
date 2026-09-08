# Design: 239c-support-layer-height-producer

## Controlling Code Paths

- **Primary code path (Z authority).** `SupportPlanner::plan_for_object`
  (`modules/core-modules/tree-support-planner/src/lib.rs`) and its traditional twin
  (`modules/core-modules/traditional-support-planner/src/lib.rs`) are where object-layer Z
  becomes support Z. Both set `SupportPlanEntry.anchor_z` from grid values today —
  `anchor_z: candidate.z_units` and `anchor_z: mm_to_units(z)` — where the `z` originates in
  `layer_plan.layers[...].z`. `TreeVolumes::new` and `insert_contact_point`
  (same tree file) also read `effective_layer_height`.
- **Primary code path (emission).** `TreeSupport::run_support`
  (`modules/core-modules/tree-support/src/lib.rs`) and `TraditionalSupport::run_support`
  (`modules/core-modules/traditional-support/src/lib.rs`), both opening their region loop with
  `let z = region.z();` and finishing with `SupportOutputBuilder::push_support_path`. Both
  already call `PaintRegionLayerView::support_plan_entries_for(object_id, region_id)`
  (`crates/slicer-sdk/src/traits.rs`), so the plan entry carrying `anchor_z` is already in hand
  at the emission site.
- **Primary code path (flow, measurement only).** `DefaultGCodeEmitter::emit_gcode`
  (`crates/slicer-gcode/src/emit.rs`) — the `height_delta` derivation and the volumetric-E
  application both live inline inside that one function; there is no helper to isolate. The
  per-point fields are `width` and `flow_factor` on `slicer_ir::Point3WithWidth`
  (`crates/slicer-ir/src/slice_ir.rs`).
- **Config path.** `[config.schema]` in the two planner manifests →
  `read_config_schema` / `ConfigFieldEntry` (`crates/slicer-scheduler/src/manifest.rs`) →
  `ConfigBoundsIndex` and `resolve_global_config`
  (`crates/slicer-scheduler/src/config_resolution.rs`) → `bind_module_config_view` and the
  declared-read guard producing `ExecutionPlanError::UndeclaredConfigKey`
  (`crates/slicer-scheduler/src/execution_plan.rs`) → `ConfigView::from_declared` and
  `ConfigView::get_bool` (`crates/slicer-ir/src/slice_ir.rs`) → each module's `from_config`.
- **Neighbouring tests/fixtures.**
  `crates/slicer-runtime/tests/integration/support_family_closure.rs` (real-slice driver
  `run_slice_for_family` → `slicer_runtime::run::run_slice`, tracked fixture
  `crates/slicer-runtime/tests/fixtures/support-family/SupportTest.stl` resolved by
  `support_test_path`, tracked config
  `crates/slicer-runtime/tests/fixtures/support-family/orca-matched-config.json` resolved by
  `matched_config_path`; `final_gcode_roles` already asserts `;TYPE:Support`);
  `crates/slicer-runtime/tests/executor/support_config_surface_tdd.rs`;
  `crates/slicer-runtime/tests/contract/config_view_binding_tdd.rs`
  (`bind_module_config_view_hides_undeclared_keys_entirely` is the neighbour of AC-N3);
  `modules/core-modules/tree-support-planner/tests/tree_family_tdd.rs`;
  `modules/core-modules/traditional-support-planner/tests/traditional_family_tdd.rs`;
  `modules/core-modules/tree-support/tests/tree_family_tdd.rs`;
  `crates/slicer-gcode/tests/gcode_emit_tdd.rs`.
- **OrcaSlicer comparison:** see `requirements.md` §OrcaSlicer Reference Obligations; do not
  repeat delegation rules.

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

## Code Change Surface

**Selected approach — `anchor_z` is the declared support print plane.**

`SupportPlanEntry` already carries `anchor_z: i64` ("anchor height in canonical units") beside
`anchor_layer_index: u32` ("layer on which the support is anchored"). Today both planners set
`anchor_z` to a copy of the object layer's Z and both renderers ignore it in favour of
`region.z()`. The whole decoupling is therefore: **make `anchor_z` mean what its doc comment
says, and make the renderers read it.** `anchor_layer_index` keeps naming the nearest object
layer, which is exactly the anchor an `OrderedEventCollection` needs.

Exact functions, manifests, tests, and fixtures:

1. `[config.schema.independent_support_layer_height]` (`type = "bool"`, `default = true`,
   `display`, `group = "Support"`) appended to
   `modules/core-modules/tree-support-planner/tree-support-planner.toml` and
   `modules/core-modules/traditional-support-planner/traditional-support-planner.toml`.
2. `SupportPlanner::from_config` (tree) and the traditional planner's `from_config` read the
   key with `ConfigView::get_bool`, defaulting to `true` on absence to match the canonical
   default.
3. `SupportPlanner::plan_for_object` and the traditional planner's entry-emitting functions
   gain a support-plane derivation. **Enabled:** planes step by the canonical rule
   (`n_layers_extra = ceil((dist - EPSILON) / max_support_layer_height)`,
   `step = dist / n_layers_extra`, `print_z = bottom_z + k * step`), then groups within
   `EPSILON` collapse to their midpoint `zavg = 0.5 * (first + last)` with the group minimum as
   height — canonical `generate_support_layers`, which is flag-independent and therefore also
   runs on the disabled branch. **Disabled:** the plane is `mm_to_units(layer_plan.layers[i].z)`
   exactly, reproducing `sync_gap_with_object_layer`. `max_support_layer_height` comes from the
   existing `support_layer_height_mm` key when non-zero, else `effective_layer_height`.
4. `TreeSupport::run_support` / `TraditionalSupport::run_support` replace `let z = region.z();`
   with the plan-declared plane taken from the `SupportPlanEntry` they already fetch via
   `PaintRegionLayerView::support_plan_entries_for`. When
   `|entry.anchor_z - mm_to_units(region.z())| <= COORDINATE_TOLERANCE_UNITS` the existing
   `push_support_path` route is used unchanged; otherwise the paths are proposed as an
   `OrderedEventCollection` through 239b's drain, with the collection's declared plane set to
   `entry.anchor_z` and its `anchor_global_layer_index` set to `entry.anchor_layer_index`.
5. Support layer **height** is not transported. The renderer derives it from consecutive
   declared planes of the same body; the emitter derives its own height term from row Z gaps
   (which is precisely what Step 5 measures).
6. Conditional: `DefaultGCodeEmitter::emit_gcode` gains per-entity plane-Z context **only** on
   a recorded `MISSCALE_FIXED` verdict.

**Rejected alternatives.**

- *Add a `support_layer_height` field to `SupportPlanEntry`.* Rejected: it is a WIT-crossing
  prepass type (`crates/slicer-sdk/src/prepass_types.rs` mirrors it), so the change would pull
  in a WIT edit, both marshal legs, and a struct-literal blast radius across at least seven
  test files — for a value both consumers can derive from the plane sequence they already have.
- *Reuse `support_layer_height_mm` alone, without a new key.* Rejected: that key decimates the
  object grid (`build_emit_schedule`, `crates/slicer-core/src/algos/support_geometry.rs`
  selects a subset of object layers) and has no notion of leaving it. Canonical treats the two
  as distinct — `Slicing.cpp` snaps gaps to `layer_height` multiples only when
  `independent_support_layer_height` is FALSE — so collapsing them would misreport parity.
- *Add a new `Layer::AnchoredEvents` core module to do the drain.* Rejected, and **no longer
  retained as a fallback** — §Open Questions `[RESOLVED]` settles the seam by widening the
  `layer-support` world instead. It is a whole new module (Cargo.toml, manifest, `src/lib.rs`,
  tests) for a re-emission hop, and it would have to re-derive what the renderer already knows.
- *Do the off-grid lowering entirely host-side from committed `SupportIR`.* Rejected, and
  **no longer retained as a fallback** for the same reason: it would leave 239b's transport
  unused and put support-family semantics back in the host, which is the architecture 221 moved
  away from.
- *Declare the key on the two renderer manifests as well.* Rejected: the renderers gate on the
  data (`anchor_z` off-grid or not), not on the flag. One authority — the planner — decides.

## Files in Scope (read + edit)

Six primary files, which is above the three-file target; the justification is that the change
is inherently symmetric across two support families and two pipeline roles, and the split is
already expressed as eight single-task steps in which no step edits more than three files.
Splitting the packet further would separate a planner from its own renderer and leave an
intermediate commit where support Z is declared off-grid but emitted on-grid.

- `modules/core-modules/tree-support-planner/tree-support-planner.toml` - role: tree planner
  config surface; expected change: one `[config.schema.independent_support_layer_height]` table.
- `modules/core-modules/traditional-support-planner/traditional-support-planner.toml` - role:
  traditional planner config surface; expected change: the identical table.
- `modules/core-modules/tree-support-planner/src/lib.rs` - role: tree Z authority; expected
  change: `from_config` reads the key; `plan_for_object` derives `anchor_z` free-floating when
  enabled, grid-exact when disabled. **Very large file — ranged reads only.**
- `modules/core-modules/traditional-support-planner/src/lib.rs` - role: traditional Z authority;
  expected change: the same derivation at its three `anchor_z` assignment sites.
- `modules/core-modules/tree-support/src/lib.rs` - role: tree renderer; expected change:
  `run_support` emits at the plan-declared plane and routes off-grid paths through the anchored
  drain.
- `modules/core-modules/traditional-support/src/lib.rs` - role: traditional renderer; expected
  change: the same in its `run_support`.

Conditional seventh file, gated on the Step 5 verdict:

- `crates/slicer-gcode/src/emit.rs` - role: volumetric E and `height_delta`; expected change:
  **none** on a `CONSISTENT` verdict; per-entity plane-Z context on `MISSCALE_FIXED`, with grid
  passes bit-identical.

Test and doc files edited by their owning steps: the four module test files named in
§Controlling Code Paths, `crates/slicer-runtime/tests/integration/support_family_closure.rs`,
`crates/slicer-runtime/tests/executor/support_config_surface_tdd.rs`,
`crates/slicer-runtime/tests/contract/config_view_binding_tdd.rs`,
`crates/slicer-gcode/tests/gcode_emit_tdd.rs`, `tmp/239c-human-validation.md`,
`docs/07_implementation_status.md`, `docs/15_config_keys_reference.md` (generated),
`docs/specs/support-parity-gap-register.md`,
`docs/specs/support-independent-layer-z-split-plan.md`.

## Read-Only Context

Include ranges for files over 300 lines.

- `docs/spec_packets/239a-anchored-host-seams/packet.spec.md` - whole file - purpose: the
  exports this packet consumes (`PipelineConfig.anchored_entities`, the executor switch, row
  synthesis, the capturing `GCodeEmitter` fixture). Its `design.md` and `implementation-plan.md`
  are **out of bounds**.
- `docs/spec_packets/239b-anchored-wit-contract/packet.spec.md` - whole file - purpose: the WIT
  package, the `set-anchored-event-collection` method, and the SDK drain. Its `design.md` and
  `implementation-plan.md` are **out of bounds**.
- `docs/specs/support-independent-layer-z-split-plan.md` - whole file (short) - purpose: F8 is
  this packet's problem statement; the canonical block is its parity ground truth.
- `docs/specs/support-parity-gap-register.md` - the range around row `G-02` only - purpose: the
  registered gap this packet closes and the origin of the `height_delta` "unverified risk".
- `crates/slicer-ir/src/slice_ir.rs` - the `SupportPlanEntry` definition and the `ConfigView`
  `impl` block only - purpose: `anchor_z`/`anchor_layer_index` semantics and the typed getters.
- `crates/slicer-scheduler/src/execution_plan.rs` - the declared-read guard inside the module
  binding loop and `bind_module_config_view` only - purpose: the `UndeclaredConfigKey` behaviour
  AC-N3 asserts.
- `crates/slicer-sdk/src/prepass_builders.rs` - the `SupportGeometryOutput` `impl` block only -
  purpose: `push_support_plan_entry` is the planner's only entry channel.
- `crates/slicer-gcode/src/emit.rs` - only the two bounded ranges the Step 5 dispatch returns -
  purpose: the `height_delta` derivation and the volumetric-E line.
- `crates/slicer-runtime/tests/integration/support_family_closure.rs` - the helper block
  (`support_test_path`, `matched_config_path`, `matched_config_for`, `run_slice_for_family`,
  `interface_block_count`) and `final_gcode_roles` only - purpose: the real-slice driver AC-1
  and AC-N1/AC-N2 reuse, and `assert_no_test_reads_orca_gcode`, which forbids any test from
  reading Orca reference G-code.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load.
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load.
- `docs/spec_packets/239a-anchored-host-seams/{design,implementation-plan,requirements}.md` and
  the same three files under `239b-anchored-wit-contract` - consume the `packet.spec.md`
  contracts only; do not absorb their internals.
- `crates/slicer-runtime/src/layer_executor.rs`, `crates/slicer-runtime/src/pipeline.rs`,
  `crates/pnp-cli/src/visual_debug.rs` - 239a-owned; read-only if unavoidable, never edited here.
- `crates/slicer-schema/wit/**`, `crates/slicer-wasm-host/src/dispatch.rs`,
  `crates/slicer-wasm-host/src/marshal/native.rs`, `crates/slicer-sdk/src/layer_collection_builder.rs`
  - 239b-owned; never edited here.
- `docs/15_config_keys_reference.md` - generated by `cargo xtask gen-config-docs`; never
  hand-edited.
- `docs/config/host-keys.toml` and `crates/slicer-runtime/tests/unit/host_keys_doc_lock_tdd.rs`
  - host-key surface; this packet declares a module key and must not touch them.
- Unrelated crates - delegate symbol lookups; do not browse.

## Expected Sub-Agent Dispatches

- Question: confirm `independent_support_layer_height` is declared in `init_fff_params`
  (`PrintConfig.cpp`) as `coBool` default true, and describe `bottom_contact_layer`'s enabled
  vs disabled `print_z` behaviour including the `sync_gap_with_object_layer` call; scope:
  `OrcaSlicerDocumented/src/libslic3r/`; return: `SUMMARY` (<= 200 words); purpose: Steps 1–2.
- Question: restate `generate_support_layers`' grouping predicate, midpoint Z rule, group-height
  rule, and the `n_layers_extra`/`step`/`print_z` intermediate stepping, and confirm it does not
  reference the flag; scope: `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp`;
  return: `SUMMARY` (<= 200 words); purpose: Step 2.
- Question: every site in `modules/core-modules/tree-support-planner/src/lib.rs` and
  `modules/core-modules/traditional-support-planner/src/lib.rs` that assigns `anchor_z` or reads
  `layer_plan.layers[..].z` / `effective_layer_height`; scope: those two files; return:
  `LOCATIONS` (<= 20 entries); purpose: Step 2, so neither large file is full-read.
- Question: the three measurement numbers for a minimal off-grid case through
  `DefaultGCodeEmitter::emit_gcode` — applied height term, declared plane delta, resulting E —
  plus the same three for the immediately following object pass (observation O-1); scope:
  `crates/slicer-gcode/`; return: `FACT` (six numbers plus the verdict word); purpose: Step 5.
  **Highest-risk dispatch.**
- Question: every test in the workspace that hard-asserts an emitted E value or a `;HEIGHT:`
  value; scope: `crates/`, `modules/`; return: `LOCATIONS` (file + test fn, <= 25 entries);
  purpose: Step 6 blast radius, fix branch only.
- Question: **confirm** (do not re-decide) that 239b landed the two-builder `layer-support` `run`
  — i.e. that `crates/slicer-schema/wit/deps/layer-support/layer-support.wit`'s `run` takes
  `collection: layer-collection-builder` and `LayerModule::run_support`
  (`crates/slicer-sdk/src/traits.rs`) takes `&mut LayerCollectionBuilder`, so
  `set_anchored_event_collection` is reachable from a module whose manifest `stage.id` is
  `Layer::Support`; scope: `crates/slicer-sdk/`, `crates/slicer-schema/wit/`,
  `crates/slicer-wasm-host/src/dispatch.rs`; return: `FACT` yes/no plus the reachable symbol
  name; purpose: an **activation precondition check** before Step 4 — the design decision is
  already made (§Open Questions `[RESOLVED]`). A `no` means 239b is not yet at
  `status: implemented`, not that this packet must choose a different route.

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

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Steps 1, 2, 4, 6, 7)
- Highest-risk dispatch and required return format: the Step 5 flow measurement over
  `crates/slicer-gcode/` — return `FACT` only (six numbers plus the verdict word). A dispatch
  that returns emitter source instead of numbers has failed and must be re-issued; the whole
  falsifiability of the packet rests on that one return.

## Open Questions

- **[RESOLVED] 239b's anchored drain IS reachable from a `Layer::Support` guest context.**
  The question as raised was correct: `crates/slicer-scheduler/src/manifest.rs` requires exactly
  one `stage.id` per manifest (`required_stage`), so `tree-support` and `traditional-support`
  cannot also export `Layer::AnchoredEvents`; 239b places `set-anchored-event-collection` on the
  `layer-collection-builder` resource; and `LayerModule::run_support`
  (`crates/slicer-sdk/src/traits.rs`) received a `SupportOutputBuilder` and no
  `LayerCollectionBuilder`.

  **Approved decision, delivered by `239b-anchored-wit-contract` (its Step 5c), not by this
  packet:** `crates/slicer-schema/wit/deps/layer-support/layer-support.wit`'s `run` gains a second
  builder parameter, `collection: layer-collection-builder`, placed after
  `output: support-output-builder` — exactly the two-builder shape
  `crates/slicer-schema/wit/deps/layer-path-optimization/layer-path-optimization.wit`'s `run`
  already uses (`output: gcode-output-builder, collection: layer-collection-builder`).
  Correspondingly `LayerModule::run_support` gains `collection: &mut LayerCollectionBuilder`.
  Rationale of record: additive to one WIT file; follows an existing in-tree precedent rather
  than inventing a shape; keeps anchored transport generic, matching ADR-0059's "each worker
  returns ordered event collections". Rejected there and not reopened here: moving the drain onto
  `support-output-builder` (would confine anchored events to support stages and narrow the
  generic substrate packets 219-223 built), and a dedicated anchored-events module (the
  one-stage-per-module manifest rule makes it a whole sibling module to author and wire).

  **Consequence for this packet:** the two documented fallbacks — a thin `Layer::AnchoredEvents`
  re-emitter module, and host-side lowering of off-grid `SupportIR` into 239a's
  `PipelineConfig.anchored_entities` — are **withdrawn**. The renderers emit anchored work
  directly from their existing `Layer::Support` context, through the `collection` parameter
  239b's widened `run_support` hands them. This packet **consumes** that two-builder signature and
  re-specifies none of it. No `[BLOCK]` remains; the two `[FWD]` questions below are open and
  implementer-resolvable, and the packet stays `status: draft` pending normal authoring gates.
- **[FWD] Which height feeds the enabled-branch plane derivation when
  `support_layer_height_mm == 0.0`?** The sentinel means "use the object's effective layer
  height", which on the enabled branch would produce planes that coincide with the grid and
  make the feature a no-op for default profiles. Canonical derives the contact plane from the
  *interface flow height* in `bottom_contact_layer`, which PnP does not model. Implementer
  decision at Step 2, to be recorded in the step's exit condition: either treat `0.0` as
  "independent height requested but unspecified → derive from the interface line width", or
  treat it as "grid pitch → no off-grid planes for this object". The second is safer for
  default profiles and keeps AC-N1 trivially true; the first is closer to canonical. Whichever
  is chosen, AC-1's fixture config must set a support pitch that demonstrably demands an
  off-grid plane, or AC-1 is unprovable.
- **[FWD] Does any raft prefix layer (negative `global_layer_index`, per the
  `SupportPlanEntry.global_layer_index` doc comment) need a declared plane too?** Raft geometry
  is packet 240's. Recommendation: leave raft prefix entries on the disabled-branch derivation
  regardless of the flag, and record that as a scoped limitation rather than a silent cut.
