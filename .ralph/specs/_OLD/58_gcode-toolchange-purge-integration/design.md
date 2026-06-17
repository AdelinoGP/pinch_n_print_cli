# Design: 58_gcode-toolchange-purge-integration

## Controlling Code Paths

- **Primary code path**: `wipe-tower` module's `run_finalization` (`modules/core-modules/wipe-tower/src/lib.rs:249-295`) calls `generate_purge_paths` (lib.rs:136-204) for each `ToolChange`. The new design extends `generate_purge_paths` to return an ordered `Vec<(ExtrusionPath3D, RegionKey)>` containing the retract entity, the travel entity, the existing rectilinear scan-line wall entities, and the prime entity, in that order. `run_finalization` then uses the **new** `FinalizationOutputBuilder::insert_entity_at(layer_index, after_entity_index + 1, …)` to place each entity at a deterministic position adjacent to the `ToolChange`. The host's `GCodeEmitter` (`crates/slicer-host/src/gcode_emit.rs`) emits `T<n>` between entity `after_entity_index` and `after_entity_index + 1`, naturally bracketing the inserted purge entities. A defensive guard near `emit_gcode`'s ToolChange emission (lines 516-525) returns `PostpassError::MissingToolchangePurge` when bracketing fails under `wipe_tower_enabled=true`.
- **Marker spelling fix**: `orca_type_label` at `gcode_emit.rs:271` changes `WipeTower → ";TYPE:Wipe tower"` to `=> ";TYPE:Prime tower"` (OrcaSlicer parity, `ExtrusionEntity.cpp:648`).
- **Bed-shape path**: `bed_shape` declared as `float-list` in `wipe-tower.toml`'s `[config.schema]` and in the host-side printer profile schema. Module reads via `config.get("bed_shape")` and parses the `Vec<f64>` as `[x0, y0, x1, y1, …]` into a `Polygon`. No `host-services` WIT change.
- **Finalization builder extension**: `wit/world-finalization.wit::finalization-output-builder` gains three additive methods (`insert-entity-at`, `set-entity-order`, `get-ordered-entities`) — implemented host-side in `crates/slicer-host/src/wit_host.rs` and exposed through the SDK **action-recorder struct** `FinalizationOutputBuilder` (NOT a trait) in `crates/slicer-sdk/src/traits.rs` (struct definition at ~line 704; `apply_to` impl block at ~lines 918-958). The struct records builder actions; `apply_to` drains them onto `&mut Vec<LayerCollectionIR>`. The three new methods are added as struct `impl` methods (recording a new `BuilderAction` variant each) plus a corresponding `apply_to` arm. Conceptually they mirror PathOptimization's `layer-collection-builder` capability surface (`wit/deps/ir-types.wit:139-170`) but adapted for finalization's multi-layer view (each method takes `layer-index`) and the read-back uses finalization-stage `print-entity-view` (`wit/world-finalization.wit:19-25`) rather than PathOpt's `ordered-entity-view` (`wit/deps/ir-types.wit:149-156`), because finalization sees the richer entity record (entity-id, topo-order).
- **Intra-stage ordering (wipe-tower runs last)**: `modules/core-modules/wipe-tower/wipe-tower.toml` declares `[compatibility].requires = ["com.core.skirt-brim", "com.core.part-cooling", "com.core.top-surface-ironing"]`. Per `docs/03_wit_and_manifest.md:817-822` and `docs/04_host_scheduler.md:762-765`, `[compatibility].requires` is the documented TOML-level primitive for intra-stage ordering; the DAG builder (`crates/slicer-host/src/dag.rs:93-102`) creates an `A → wipe-tower` edge for every declared `A`, forcing wipe-tower last in `PostPass::LayerFinalization`. No new manifest key is added.
- **Neighboring tests/fixtures**:
  - `crates/slicer-host/tests/tool_ordering_tdd.rs` — `LayerCollectionIR` assembly idioms.
  - `crates/slicer-host/tests/finalization_live_tdd.rs:7,50` — load_wipe_tower test (verify it still passes after stage hasn't moved).
  - `crates/slicer-host/tests/finalization_aware_travel_tdd.rs:31,62` — wipe-tower travel reconciliation.
  - `modules/core-modules/wipe-tower/src/lib.rs` and `wipe-tower.toml` — module under change.
- **OrcaSlicer comparison surface**: `WipeTower2.cpp:1557-1640` ordering; `WipeTower2.cpp:2069-2205` finish_layer; `ExtrusionEntity.cpp:628-654` role-to-`;TYPE:` mapping.

## Architecture Constraints

- `wipe-tower` runs in `PostPass::LayerFinalization` and mutates `&mut Vec<LayerCollectionIR>` via the `FinalizationOutputBuilder` action-recorder **struct** (whose `apply_to` impl method drains recorded actions). Senior-review audit confirmed migrating wipe-tower to `Layer::PathOptimization` is infeasible (three fatal blockers — see **Stage-Migration Rejected** below); the module stays in finalization.
- New entities added by wipe-tower use existing IR fields. Adding fields to `ToolChange` or `PrintEntity` is out of scope. `ExtrusionRole::WipeTower` already exists in `crates/slicer-ir/src/slice_ir.rs` (variant at ~line 1336; the surrounding `enum ExtrusionRole` block spans roughly 1318-1350; Step 1 reverifies exact lines) and is first-class in `wit/deps/types.wit:24-29`. `PrimeTower` is at ~line 1338; `Skirt` is the last variant in the same block.
- The `wipe_tower_enabled` config flag is the canonical gate. When `false`, wipe-tower skips emission entirely.
- Per `docs/02_ir_schemas.md` determinism contract, purge entity positions must be deterministic given the same input. Use `wipe_tower_x`, `wipe_tower_y`, `wipe_tower_width`, `line_width` from config — no RNG.
- Coordinate units: `ExtrusionPath3D.points` already in mm; bed_shape config values are mm. No unit conversion at boundaries.
- Per packet 11's emission contract, role labels are `;TYPE:<RoleName>`. **OrcaSlicer's canonical role-name for the wipe-tower extrusion is "Prime tower"** (`ExtrusionEntity.cpp:648`).
- `WipeTower` is first-class at the WIT boundary; host-to-WIT translation at `wit_host.rs:4747-4768` is direct passthrough. `PrimeTower`/`Skirt` use `Custom("slicer.builtin/...")` tags — not affected by this packet.

## Stage-Migration Rejected

A senior-review pass evaluated moving wipe-tower from `PostPass::LayerFinalization` to `Layer::PathOptimization` (where `set-entity-order` already exists). **Rejected** on three fatal blockers:

- **B1**: `layer-collection-builder` in PathOptimization (`wit/deps/ir-types.wit:167-170`) exposes only `set-entity-order` and `get-ordered-entities`. No `push-entity-*`. Wipe-tower couldn't emit its scan lines.
- **B2**: PathOptimization's read-side has no `tool_changes()` accessor. Wipe-tower couldn't see what to react to. Tool changes accumulate in `arena.deferred_tool_changes` (`crates/slicer-host/src/dispatch.rs:2830`) but aren't exposed to sibling modules.
- **B3**: `wit/world-layer.wit::run-path-optimization` exposes `layer-index` only — no `z`, no layer-height. Wipe-tower can't size tower geometry.

Plus the stage order is fixed in `crates/slicer-host/src/execution_plan.rs:27-48` and inserting an intermediate stage is a workspace-wide refactor.

**Conclusion**: keep wipe-tower in finalization; add the missing primitives (positional insert, permutation, read-back) to `finalization-output-builder` instead.

## Code Change Surface

- **Selected approach** — "Three-method WIT extension on `finalization-output-builder` + `bed_shape` config (via `declare_resolved_config!` macro) + `[compatibility].requires` ordering directive + entity-injection inside `generate_purge_paths` + one-line marker spelling fix + defensive guard." The wipe-tower module uses the new `insert-entity-at` to place retract/travel/prime/wipe entities at `after_entity_index + 1`, bracketing the `T<n>`. The host's `apply_to` (the impl block on the `FinalizationOutputBuilder` struct at `crates/slicer-sdk/src/traits.rs` ≈ lines 918-958) is extended to handle the new methods and to remap `ToolChange.after_entity_index` and `ZHop.after_entity_index` on insert/permute (the **Locked Invariants** in `packet.spec.md`).

- **Exact functions, traits, manifests, tests, or fixtures expected to change**:
  - `wit/world-finalization.wit::finalization-output-builder` — add 3 additive methods. The resource definition near lines 62-104 grows.
  - `crates/slicer-sdk/src/traits.rs::FinalizationOutputBuilder` — this type is a **struct** (action-recorder), not a trait; add 3 new `impl` methods that record `BuilderAction` variants and extend the existing `apply_to` impl method (at ≈ lines 918-958) to handle the new actions and remap `ToolChange.after_entity_index` / `ZHop.after_entity_index` on insert/permute.
  - `crates/slicer-host/src/wit_host.rs` — host-side impl of the 3 new builder methods (location confirmed by Step 1 dispatch).
  - `modules/core-modules/wipe-tower/wipe-tower.toml` — add `[config.schema.bed_shape]` entry (type `float-list`, required when `wipe_tower_enabled=true`).
  - `crates/slicer-ir/src/resolved_config.rs` (the macro-driven `declare_resolved_config!` SoT introduced by commit `19e5791`) — add `bed_shape: List<f64>` field with default `[0.0, 0.0, 250.0, 0.0, 250.0, 250.0, 0.0, 250.0]` (250 mm × 250 mm rectangle). Step 1 reverifies the macro accepts a `List<f64>` shape; if it does not, the packet absorbs a small macro extension. The host populates `ConfigView` from `ResolvedConfig`; modules read `config.get("bed_shape")`.
  - `modules/core-modules/wipe-tower/wipe-tower.toml` — also add `[compatibility].requires = ["com.core.skirt-brim", "com.core.part-cooling", "com.core.top-surface-ironing"]` so wipe-tower runs last in `PostPass::LayerFinalization` (documented intra-stage ordering primitive, `docs/03_wit_and_manifest.md:817-822` + `docs/04_host_scheduler.md:762-765`).
  - `modules/core-modules/wipe-tower/src/lib.rs::generate_purge_paths` (lib.rs:136-204) — extend return value with retract, travel, and prime entities. Each new entity is tagged `ExtrusionRole::WipeTower`. The prime entity's cumulative positive E delta equals `wipe_tower_purge_volume / (line_width * layer_height)` mm.
  - `modules/core-modules/wipe-tower/src/lib.rs::run_finalization` (lib.rs:249-295) — call `output.insert_entity_at(layer_index, tc.after_entity_index + 1, path, region_key)` for each generated entity (in order: retract, travel, walls, prime). Read `bed_shape` from `config.get("bed_shape")`; on out-of-bed placement return `ModuleError::fatal` naming the violating coordinate.
  - `crates/slicer-host/src/gcode_emit.rs::orca_type_label` at line 271 — change `WipeTower => ";TYPE:Wipe tower"` to `=> ";TYPE:Prime tower"`. One-line string change.
  - `crates/slicer-host/src/gcode_emit.rs::emit_gcode` (around lines 516-525) — add a defensive check: when `wipe_tower_enabled=true`, each `ToolChange` must be bracketed by at least one retract entity before and at least one `ExtrusionRole::WipeTower` entity after; otherwise return `PostpassError::MissingToolchangePurge { layer_index, tool_change_index }`.
  - `crates/slicer-host/src/postpass.rs::PostpassError` (≈ lines 40-60; existing variants `FatalModule`, `GCodeEmit`, `GCodeSerialization`) — add additive `MissingToolchangePurge { layer_index: u32, tool_change_index: u32 }`. Types are `u32` to match `ToolChange.after_entity_index: u32` and the IR's `layer-idx` convention.
  - **New**: `crates/slicer-host/tests/gcode_toolchange_wrapping.rs` — AC1, AC3, NC1.
  - **New**: `crates/slicer-host/tests/finalization_builder_insert.rs` — AC7, NC5.
  - **New**: `crates/slicer-host/tests/finalization_builder_permute.rs` — AC8, NC6.
  - **New**: `crates/slicer-host/tests/finalization_builder_readback.rs` — AC9.
  - **New**: `crates/slicer-host/tests/wipe_tower_bed_bounds.rs` — AC6.
  - **New**: `modules/core-modules/wipe-tower/src/lib.rs#tests` — AC4 (`emits_prime_tower_role_marker`), NC4 (`tower_outside_bed_returns_fatal`).
  - **New**: `crates/slicer-host/tests/fixtures/multi_color_cube.stl` and `multi_color_cube.orca.gcode` (historical artifact). Post-review 2026-05-19: additionally committed `crates/slicer-host/tests/fixtures/benchy_4color.config.json` to drive the live multi-material verification against `resources/benchy_4color.3mf` — see `packet.spec.md` AC retargeting note.
  - **Docs**: `docs/03_wit_and_manifest.md` — one-paragraph addition under `finalization-output-builder` describing the three new methods and the index-remap invariants.

- **Rejected alternatives** (must choose one):
  1. **"Emitter-level wrapping (host-only)"** — synthesize retract/prime moves at G-code emit time from config alone. Rejected because purge geometry depends on layer-level state owned by the wipe-tower module.
  2. **"`push-entity-with-priority` with role-based priority for bracketing"** — append entities and rely on stable-sort priorities to land them adjacent to `after_entity_index`. Rejected after senior-review audit: priorities cluster entities by role, not by position; wipe-tower entities sort into one block at a position determined by their role priority, not adjacent to the ToolChange.
  3. **"New `PurgeIR` IR struct attached to `ToolChange`"** — add `purge: Option<PurgeSequence>` field. Rejected: bigger blast radius, more migration surface, no behavioral gain over an additive role-tagged entity list.
  4. **"Flip `wipe_tower_enabled` default to true"** — out of scope per the bugfix-only directive.
  5. **"Keep `;TYPE:Wipe tower` and document the divergence"** — rejected. OrcaSlicer's canonical mapping is `erWipeTower → "Prime tower"`; downstream parity tooling looks for that spelling.
  6. **"Add `host-services::print-bed-shape` WIT accessor"** — original draft's choice. Rejected after senior-review: bed shape is a printer-profile property, idiomatically expressed as config. Using `config-value::float-list` requires zero WIT change.
  7. **"Migrate wipe-tower to `Layer::PathOptimization`"** — explored and rejected; three fatal blockers documented in **Stage-Migration Rejected** above.
  8. **"First-class `prime-tower` / `skirt` in `wit/deps/types.wit::extrusion-role`"** — out of scope. Wipe-tower only emits `WipeTower` (first-class).
  9. **"Add only `insert-entity-at`, skip `set-entity-order` and `get-ordered-entities`"** — rejected per user directive: mirror PathOptimization's capability surface so future packets don't need a second WIT pass for permutation/read-back.

## Files in Scope (read + edit)

Primary edit targets (the `≤ 3 per step` rule applies per implementation step, not aggregated):

- `wit/world-finalization.wit` — 3 additive method declarations on `finalization-output-builder`.
- `crates/slicer-sdk/src/traits.rs` — `FinalizationOutputBuilder` **struct** (action-recorder) impl extension + `apply_to` impl-method extension (index remap).
- `crates/slicer-host/src/wit_host.rs` — host-side impl of the 3 new builder methods.
- `modules/core-modules/wipe-tower/src/lib.rs` — extend `generate_purge_paths`; rewrite `run_finalization` to use `insert_entity_at`; add bed-bounds check; add unit tests.
- `modules/core-modules/wipe-tower/wipe-tower.toml` — `[config.schema.bed_shape]` entry.
- Host-side printer-profile config schema (file located by Step 1) — add `bed_shape` key.
- `crates/slicer-host/src/gcode_emit.rs` — one-line spelling fix at 271; guard near 516-525.
- `crates/slicer-host/src/postpass.rs` — additive variant at 39-59.
- Test files: 5 new files (`gcode_toolchange_wrapping.rs`, `finalization_builder_insert.rs`, `finalization_builder_permute.rs`, `finalization_builder_readback.rs`, `wipe_tower_bed_bounds.rs`) plus `#[cfg(test)] mod tests` additions inside `modules/core-modules/wipe-tower/src/lib.rs`.

`crates/slicer-ir/src/slice_ir.rs` is **read-only** — `ExtrusionRole::WipeTower` already exists.

## Read-Only Context

- `docs/02_ir_schemas.md` — delegate via SUMMARY.
- `docs/03_wit_and_manifest.md` — range-read finalization-builder section, config-value types, manifest schema syntax. Step 7 appends a paragraph.
- `docs/04_host_scheduler.md` — range-read LayerFinalization → GCodeEmit boundary.
- `docs/08_coordinate_system.md` — direct read.
- `docs/09_progress_events.md` — direct read.
- `docs/11_operational_governance_and_acceptance_gate.md` — range-read §1.
- `crates/slicer-ir/src/slice_ir.rs` — `ExtrusionRole` block (≈ lines 1318-1350; `WipeTower` at ~1336, `PrimeTower` at ~1338, `Skirt` is the last variant in the same block; Step 1 reverifies exact lines).
- `crates/slicer-ir/src/slice_ir.rs` — `ToolChange`, `TravelRetract`, `LayerCollectionIR.tool_changes`, `ConfigValue` ranges located via Step 1 dispatch.
- `crates/slicer-host/src/gcode_emit.rs:259-276` — `orca_type_label`; the `WipeTower` arm at line 271 is the one-line change.
- `crates/slicer-host/src/gcode_emit.rs:385-410` — per-layer tool-change lookup.
- `crates/slicer-host/src/gcode_emit.rs:516-525` — `GCodeCommand::ToolChange` emission.
- `crates/slicer-host/src/gcode_emit.rs:1275-1290` — bare `T<n>` writeln.
- `crates/slicer-sdk/src/traits.rs` — `FinalizationOutputBuilder` struct at ~line 704; `apply_to` impl method at ~lines 918-958 (gains insert/permute remap logic).
- `crates/slicer-sdk/src/layer_collection_builder.rs:53-71` — PathOptimization's `set-entity-order` contract (mirror reference for the finalization-side equivalent).
- `crates/slicer-host/src/wit_host.rs` — `finalization-output-builder` impl block (Step 1 dispatch locates).
- `wit/host-api.wit` — read-only verification (no edits).
- `wit/world-finalization.wit` — full read (~100 lines).
- `wit/deps/types.wit` — range-read `extrusion-role`, `polygon`, `point2`.
- `wit/deps/config.wit` — full read (small).
- `wit/deps/ir-types.wit:139-170` — `gcode-output-builder.push-tool-change` and PathOptimization `layer-collection-builder` (mirror reference for the new finalization-side methods).
- `wit/world-finalization.wit:19-25` — `print-entity-view` record (return type for the new `get-ordered-entities`).
- `docs/03_wit_and_manifest.md:817-822` — `[compatibility].requires` manifest key documentation.
- `docs/04_host_scheduler.md:762-765` — finalization-stage ordering guarantees.
- `crates/slicer-host/src/dag.rs:93-102` — DAG edge creation for `requires_modules`.
- `modules/core-modules/{skirt-brim,part-cooling,top-surface-ironing}/<name>.toml` — read-only inspection of sibling `[module].id` values to populate `wipe-tower.toml`'s new `[compatibility].requires` list.
- `modules/core-modules/wipe-tower/wipe-tower.toml` — full read.
- `crates/slicer-host/src/layer_finalization.rs:80-110` — orchestration.
- `crates/slicer-host/tests/tool_ordering_tdd.rs` — full read for idioms.

## Out-of-Bounds Files

- All of `OrcaSlicerDocumented/` — delegate every parity check.
- `target/`, `Cargo.lock`, any `.wasm` artifact.
- Crates not on the change list (`slicer-helpers`, `slicer-cli`, other core-modules).
- Other module manifests outside `wipe-tower/`.
- `docs/14_deviation_audit_history.md` — read-only audit trail.
- `docs/07_implementation_status.md` in full — locate line ranges via dispatch.
- Every `wit/world-*.wit` other than `world-finalization.wit` — read-only invariant check in Step 5.
- `crates/slicer-sdk/src/traits.rs` in full — range-read only the `FinalizationOutputBuilder` **struct** (at ~line 704), its action-enum, and `apply_to` (≈ 918-958).

## Expected Sub-Agent Dispatches

- **Step 1** (pure dispatch, confirm landscape):
  - "Confirm `ExtrusionRole::WipeTower` in `crates/slicer-ir/src/slice_ir.rs` (expected variant at ~line 1336; block range ≈ 1318-1350); locate current ranges for `ToolChange`, `TravelRetract`, `LayerCollectionIR.tool_changes`, `ConfigValue`; LOCATIONS ≤ 8 entries."
  - "Confirm `orca_type_label` at `gcode_emit.rs:259-276` currently maps `WipeTower → \";TYPE:Wipe tower\"` at line 271; FACT pass/fail with exact line."
  - "Confirm `PostpassError` at `postpass.rs:39-59` lacks `MissingToolchangePurge`; FACT ≤ 5 lines listing current variants."
  - "Locate `GCodeCommand::ToolChange` emission in `gcode_emit.rs::emit_gcode` (expected ~516-525) and the bare `T<n>` writeln (expected ~1283-1284); LOCATIONS ≤ 4 entries."
  - "Open `crates/slicer-ir/src/resolved_config.rs` (the macro-driven SoT after commit `19e5791`). Locate the `declare_resolved_config!` invocation and confirm whether the macro accepts a `List<f64>` field shape (needed to add `bed_shape`). FACT ≤ 5 lines + SNIPPETS ≤ 15 lines of the macro invocation showing an existing list-typed field if any."
  - "Search `crates/slicer-ir/src/resolved_config.rs` and `modules/core-modules/wipe-tower/wipe-tower.toml` for an existing retract-distance config key (`retract_length`, `retraction_distance`, `retract_distance`, or similar). FACT — key name + type if it exists, or 'no existing retract-distance key' if none."
  - "Locate the `finalization-output-builder` host impl block in `crates/slicer-host/src/wit_host.rs`; LOCATIONS ≤ 3 entries."
  - "Locate the `FinalizationOutputBuilder` **struct** definition in `crates/slicer-sdk/src/traits.rs` (struct at ~line 704, NOT a trait); SNIPPET ≤ 30 lines showing the struct + the variant-enum it records actions into."
  - "Confirm `wit/deps/types.wit` exports `polygon` and `point2` from `geometry`; SNIPPET ≤ 10 lines."
  - "Confirm `wit/deps/config.wit::config-value` includes `float-list(list<f64>)`; SNIPPET ≤ 15 lines."
  - "Summarize OrcaSlicer `WipeTower2.cpp:1557-1640` Unload/Change/Load/Wipe call order; FACT ≤ 5 lines."
- **Step 2** (TDD scaffolding):
  - "Run `cargo test -p slicer-host --test gcode_toolchange_wrapping`; FACT compile success + 3 test failures."
  - "Run `cargo test -p slicer-host --test wipe_tower_bed_bounds`; FACT compile success + 1 ignored test."
  - "Run `cargo test -p slicer-host --test finalization_builder_insert`; FACT compile success + tests ignored until Step 3."
  - "Run `cargo test -p wipe-tower --lib`; FACT compile success + ignored module tests."
- **Step 3** (WIT extension + SDK + host impl):
  - "Run `cargo check --workspace`; FACT pass/fail."
  - "Run `./modules/core-modules/build-core-modules.sh`; FACT exit code + last 5 lines."
  - "Run `./modules/core-modules/build-core-modules.sh --check`; FACT pass/fail."
  - "Run `cargo clippy --workspace -- -D warnings`; FACT pass/fail."
  - "Run `cargo test -p slicer-host --test finalization_builder_insert`; FACT pass/fail (expect AC7 + NC5 green)."
  - "Run `cargo test -p slicer-host --test finalization_builder_permute`; FACT pass/fail (expect AC8 + NC6 green)."
  - "Run `cargo test -p slicer-host --test finalization_builder_readback`; FACT pass/fail (expect AC9 green)."
- **Step 4** (marker fix + guard + variant):
  - "Run `cargo check --workspace`; FACT pass/fail."
  - "Run `cargo clippy --workspace -- -D warnings`; FACT pass/fail."
  - "Run `cargo test -p slicer-host --test gcode_toolchange_wrapping bare_toolchange_rejected -- --nocapture`; FACT pass/fail."
- **Step 5** (module emission + bed-bounds check):
  - "Confirm no other `PostPass::LayerFinalization` module asserts entity-count invariants; LOCATIONS ≤ 10 entries from `modules/core-modules/{skirt-brim,part-cooling,top-surface-ironing}/src/lib.rs`."
  - "Run `./modules/core-modules/build-core-modules.sh`; FACT exit code + last 5 lines."
  - "Run `cargo test -p wipe-tower --lib`; FACT pass/fail (expect AC4 + NC4 green)."
  - "Run `cargo test -p slicer-host --test gcode_toolchange_wrapping`; FACT pass/fail (expect AC1 + AC3 + NC1 green)."
  - "Run `cargo test -p slicer-host --test wipe_tower_bed_bounds`; FACT pass/fail (expect AC6 green)."
- **Step 6** (end-to-end CLI):
  - "Run `cargo run --bin slicer-cli --release --slice --input ... --output ...`; FACT exit code + last 5 lines."
  - "Run each AC and NC pipe-suffixed command from `packet.spec.md` against the produced G-code; FACT pass/fail per command."
- **Step 7** (docs + status):
  - "Locate line ranges for TASK-143, TASK-152b, TASK-120d2 in `docs/07_implementation_status.md`; LOCATIONS ≤ 6 entries."
  - "Show most recent 3 entries of `docs/DEVIATION_LOG.md`; SNIPPETS ≤ 30 lines each."
  - "Locate `finalization-output-builder` section header in `docs/03_wit_and_manifest.md`; LOCATIONS 1 entry."

## Data and Contract Notes

- **IR contracts**: `ExtrusionRole::WipeTower` unchanged. `ToolChange` shape unchanged. `LayerCollectionIR.tool_changes` remains read-only for `gcode_emit.rs`. The `apply_to` function in the SDK grows to remap `ToolChange.after_entity_index` and `ZHop` indices on insert/permute.
- **WIT boundary**: 3 additive methods on `finalization-output-builder`. No change to `host-services`, `layer-collection-view`, `extrusion-role`, or any other resource. Guest bindgen invalidation rebuilds every guest.
- **Marker contract**: Packet 11's `;TYPE:<RoleName>` contract unchanged. `orca_type_label:271` correction aligns Pinch 'n Print's RoleName with OrcaSlicer's canonical "Prime tower".
- **Config contract**: `bed_shape: float-list` is a new printer-profile config key. The host populates it from the active printer profile; modules read via `ConfigView`. Format documented in `wipe-tower.toml`'s schema entry and in the deviation log.
- **Determinism**: tower X/Y from config; spacing from `line_width`; volume from `wipe_tower_purge_volume`; retract/prime E from `wipe_tower_purge_volume`/`line_width`/`layer_height`; insert positions from `after_entity_index + offset` (deterministic). No RNG.
- **Scheduler**: wipe-tower stays in `PostPass::LayerFinalization`. Modules in the stage run sequentially per `crates/slicer-host/src/layer_executor.rs:414-418`. Step 5 dispatch confirms no neighboring finalization module asserts entity-count invariants that adding wipe-tower entities would break.

## Locked Assumptions and Invariants

- `wipe_tower_enabled=false` keeps current behavior. No regression to single-color paths.
- Wipe-tower is the only emitter of `ExtrusionRole::WipeTower` entities and `;TYPE:Prime tower` markers.
- `ToolChange.after_entity_index` semantics are stable across `path-optimization-default` and `wipe-tower`. This packet does not perturb either.
- Purge geometry vertices in mm. `bed_shape` config in mm.
- New fixture < 64 KB STL and < 256 KB OrcaSlicer reference G-code; checked in (not git-lfs).
- No standalone `volume_to_length` helper. Wipe-tower computes inverse inline as `length_mm = volume_mm3 / (line_width_mm * layer_height_mm)`.
- After WIT extension lands, every guest `.wasm` is invalidated. `./modules/core-modules/build-core-modules.sh` must run before any integration test executes; `--check` must report fresh before Step 5.
- **Insert/permute index remap**: see `packet.spec.md` Locked Invariants section. The host's `apply_to` is the sole owner of this remap logic; modules MUST NOT pre-adjust indices themselves.

## Risks and Tradeoffs

- **Risk**: another `PostPass::LayerFinalization` module pushes entities into the same layer after wipe-tower runs, causing the post-apply stable sort to land a non-wipe-tower entity between index `K` and `K+1` and breaking the bracketing invariant. → **Mitigation**: Step 3 adds `[compatibility].requires = ["com.core.skirt-brim", "com.core.part-cooling", "com.core.top-surface-ironing"]` to `wipe-tower.toml`. The DAG builder (`crates/slicer-host/src/dag.rs:93-102`) creates predecessor edges forcing wipe-tower last in its stage. Tradeoff: `[compatibility].requires` enforces presence of the listed modules; if any is removed from the active configuration wipe-tower will refuse to load. Acceptable because all three are core modules shipped together.
- **Risk**: `crates/slicer-ir/src/resolved_config.rs::declare_resolved_config!` macro may not accept a `List<f64>` field shape today. → **Mitigation**: Step 1 dispatch confirms macro support. If it does not support list-typed fields, the packet absorbs a minimal macro extension (documented as a sub-task in Step 3d).
- **Risk**: no existing retract-distance config key for the retract entity's E delta in `generate_purge_paths`. → **Mitigation**: Step 1 dispatch surveys existing keys; Step 3 declares a new `retract_length: f64` (default 2.0 mm) in both `wipe-tower.toml`'s `[config.schema]` and `resolved_config.rs` if none is found, rather than hand-coding a literal in the module.
- **Risk**: ±20% purge-volume parity (AC3) is sensitive to extrusion-width differences. → **Mitigation**: loose tolerance; test reports SNIPPETS diff on first failure.
- **Tradeoff**: WIT extension invalidates every guest `.wasm`. → Acceptable per project workflow.
- **Tradeoff**: adding 3 new builder methods enlarges the `finalization-output-builder` resource from 6 → 9 methods. Acceptable — user explicitly requested mirroring PathOptimization's capability surface for future packets.
- **Tradeoff**: `set-entity-order` and `get-ordered-entities` are not exercised beyond smoke tests in this packet. Listed as YAGNI risk; mitigated by user's explicit forward-looking directive.
- **Tradeoff**: changing `orca_type_label`'s `WipeTower` arm from `";TYPE:Wipe tower"` to `";TYPE:Prime tower"` is a user-visible G-code change. Acceptable — no shipped tooling depends on the old spelling, and OrcaSlicer parity is the project goal. Recorded in Step 7 DEVIATION_LOG.
- **Tradeoff**: `apply_to` index-remap logic is new and the most subtle correctness risk in this packet. Mitigated by AC7/AC8/NC5/NC6 covering all four scenarios (insert in-bounds, insert OOB, permute valid, permute malformed).

## Context Cost Estimate

- Aggregate: **M**.
- Largest single step: **M** (Step 3 — WIT extension + SDK struct-impl extension + host impl + index-remap logic in `apply_to` + 3 builder tests + `[compatibility].requires` ordering directive).
- Highest-risk dispatch: the host-side printer-profile field locator in Step 1 — if the schema indirection is complex, the dispatch should return SUMMARY (≤ 200 words) rather than FACT to give the implementer enough context to add `bed_shape` correctly. Re-dispatch if needed.

## Open Questions

None blocking activation.

**Resolved facts** (pre-confirmed during refinement audits; Step 1 reverifies):

- `ExtrusionRole::WipeTower` in `slice_ir.rs` at ~line 1336 (re-verified after recent IR edits); `PrimeTower` at ~1338; `Skirt` is the last variant in the same block. The `enum ExtrusionRole` block spans roughly lines 1318-1350. Step 1 reverifies exact lines before any edit. `PrimeTower` / `Skirt` are Custom-tagged across WIT and not relevant to this packet.
- `orca_type_label` at `gcode_emit.rs:259-276` currently maps `WipeTower → ";TYPE:Wipe tower"` at line 271 and `PrimeTower → ";TYPE:Prime tower"` at line 272.
- `PostpassError` at `postpass.rs` (≈ lines 40-60) has variants `FatalModule { stage_id, module_id, message }`, `GCodeEmit { message }`, `GCodeSerialization { message }`. The additive variant added by this packet uses `u32` (not `usize`) for `layer_index` and `tool_change_index` to match `ToolChange.after_entity_index: u32`.
- OrcaSlicer canonical mapping at `ExtrusionEntity.cpp:648`: `erWipeTower → "Prime tower"`. No separate `erPrimeTower` exists. Pinch 'n Print's `;TYPE:Prime tower` is the parity-correct spelling.
- `wit/deps/config.wit::config-value` includes `float-list(list<f64>)` — zero-WIT-change path for `bed_shape`.
- `wit/world-finalization.wit::finalization-output-builder` currently has 6 methods (`push-entity-to-layer`, `push-entity-with-priority`, `modify-entity`, `sort-layer-by`, `insert-synthetic-layer-after`, `insert-synthetic-layer`). None provide positional intra-layer insertion or permutation. This packet adds 3.
- `wit/deps/ir-types.wit:139-170` — PathOptimization's `layer-collection-builder` has `set-entity-order(items: list<tuple<u32, bool>>) -> result<_, string>` and `get-ordered-entities() -> list<ordered-entity-view>`. **Mirror reference for the finalization-side additions, with two intentional differences**: (a) finalization methods take an additional `layer-index` parameter because finalization sees `Vec<LayerCollectionIR>`; (b) `get-ordered-entities` returns `list<print-entity-view>` (defined at `wit/world-finalization.wit:19-25`, with `entity-id` and `topo-order` fields) rather than PathOpt's `ordered-entity-view` (`wit/deps/ir-types.wit:149-156`), because finalization sees the richer entity record.
- `crates/slicer-sdk/src/traits.rs` — `FinalizationOutputBuilder` is a **struct** at ~line 704 (NOT a trait); its `apply_to` impl method at ~lines 918-958 is the host-side bookkeeping that grows to handle insert/permute index remap.
- `ToolChange.after_entity_index: u32` and `ZHop.after_entity_index: u32` (in `crates/slicer-ir/src/slice_ir.rs`) — both fields are `u32`; the remap invariant covers `ZHop.after_entity_index` specifically.
- Sibling `PostPass::LayerFinalization` modules: `com.core.skirt-brim`, `com.core.part-cooling`, `com.core.top-surface-ironing` (per their `[module].id` declarations in `modules/core-modules/*/`). None currently use `[compatibility].requires` for ordering.
- Wipe-tower module currently reads `view.tool_changes()` (lib.rs:260,278) and emits `WipeTower`-role scan-line walls (lib.rs:187). Today `generate_purge_paths` (lib.rs:136-204) emits walls only — no retract, no travel, no prime.
- No standalone `volume_to_length` helper.
- Stage migration to `Layer::PathOptimization` is infeasible (three blockers documented above).
- No `bed_shape` config key exists today — grep returned zero hits.
