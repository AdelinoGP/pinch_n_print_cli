---
status: implemented
packet: 58_gcode-toolchange-purge-integration
task_ids:
  - TASK-143
  - TASK-152b
  - TASK-120d2
---

# 58_gcode-toolchange-purge-integration

## Goal

Wire the existing `wipe-tower` module's per-layer output through the live G-code emission path so every `T<n>` tool-change token in the final `.gcode` is bracketed by a retract → travel → load/prime → wipe sequence, and so every layer that contains at least one tool change emits a `;TYPE:Prime tower` block matching OrcaSlicer's canonical marker (`OrcaSlicerDocumented/src/libslic3r/ExtrusionEntity.cpp:648` — `erWipeTower → "Prime tower"`).

Today three problems stack:

1. **No positional insertion in finalization.** `finalization-output-builder` only has `push-entity-to-layer` / `push-entity-with-priority`, which APPEND and then stable-sort by `(priority, insertion_order)` (`crates/slicer-sdk/src/traits.rs:918-956`). Wipe-tower cannot place an entity at a specific index relative to `ToolChange.after_entity_index`. The `T<n>` is emitted between entities `K` and `K+1`; today there is no API by which wipe-tower can guarantee its tower entities land at position `K+1`. The `set-entity-order` method that solves this in `Layer::PathOptimization` (`wit/deps/ir-types.wit:167-170`) has no counterpart in finalization.
2. **`generate_purge_paths` emits only scan-line walls** (`modules/core-modules/wipe-tower/src/lib.rs:136-204`) — no retract, no travel-to-tower, no prime entity sized to `wipe_tower_purge_volume`.
3. **`orca_type_label` emits the wrong marker spelling.** At `crates/slicer-host/src/gcode_emit.rs:271`, `ExtrusionRole::WipeTower → ";TYPE:Wipe tower"` diverges from OrcaSlicer's canonical `";TYPE:Prime tower"`.

This packet closes all three.

## Problem Statement

Three closed packets each completed their own slice cleanly, but the slices never met. Two refinement audits identified design errors in earlier drafts of this packet and corrected them.

- **Packet 17 (`17_wipe-tower-finalization-live-path`)** ported the `wipe-tower` module onto the live `run_finalization()` path. The module already reads `view.tool_changes()` and emits scan-line tower walls tagged `ExtrusionRole::WipeTower` (`modules/core-modules/wipe-tower/src/lib.rs:259-295`, role at lib.rs:187). **Gap**: `generate_purge_paths` (lib.rs:136-204) emits only rectilinear scan lines — no retract entity before the tower, no travel-to-tower move, no prime entity sized to `wipe_tower_purge_volume`.
- **Packet 19 (`19_path-optimization-tool-order-and-cooling-policy`)** added deterministic mixed-tool ordering and `push-tool-change` via the path-optimization-stage WIT. **Gap**: when `path-optimization-default` populates `LayerCollectionIR.tool_changes`, the downstream `GCodeEmitter` writes a bare `T<n>` line (`crates/slicer-host/src/gcode_emit.rs`, `emit_gcode` near lines 516-525; the bare-line serialization is near lines 1283-1284) with no defensive guard requiring surrounding purge entities.
- **Packet 11 (`11_orca-gcode-emission-contract`)** defined the canonical `;TYPE:<RoleName>` role labeling. **Gap**: `orca_type_label` at `crates/slicer-host/src/gcode_emit.rs:271` maps `ExtrusionRole::WipeTower → ";TYPE:Wipe tower"`. OrcaSlicer's canonical mapping (`OrcaSlicerDocumented/src/libslic3r/ExtrusionEntity.cpp:648`) is `erWipeTower → "Prime tower"`. Downstream tooling expecting `;TYPE:Prime tower` ignores Pinch 'n Print's section.
- **Positional-insertion gap (newly identified)**: `finalization-output-builder` exposes only `push-entity-to-layer` / `push-entity-with-priority`, both of which APPEND then stable-sort by `(priority, insertion_order)` (`crates/slicer-sdk/src/traits.rs::FinalizationOutputBuilder::apply_to` ≈ lines 918-958). There is **no API by which wipe-tower can place an entity at a specific index relative to `ToolChange.after_entity_index`**. The original packet-58 draft proposed solving the bug by "injecting entities" via `push-entity-with-priority`; that approach was infeasible — wipe-tower entities sort by role priority into a single cluster, not adjacent to the ToolChange. PathOptimization's `set-entity-order` (`wit/deps/ir-types.wit:139-170`) solves this for the layer stage but has no counterpart in finalization. This packet adds the missing primitives.
- **Intra-stage ordering risk (newly identified)**: `wipe-tower` is one of four modules in `PostPass::LayerFinalization` (alongside `skirt-brim`, `part-cooling`, `top-surface-ironing`). Per `docs/04_host_scheduler.md:762-765`, intra-stage order is determined by topological sort over the IR read/write DAG plus explicit `[compatibility].requires` edges; absent either signal, order is stable but undefined. If a sibling module runs after wipe-tower and calls `push-entity-with-priority`, the post-apply stable sort could land a non-wipe-tower entity between index `K` (the `ToolChange`'s reference) and index `K+1` (wipe-tower's retract), breaking the bracketing invariant. **Resolution**: this packet declares `[compatibility].requires` in `wipe-tower.toml` listing the three sibling finalization modules — the documented TOML primitive for intra-stage ordering (`docs/03_wit_and_manifest.md:817-822`).
- **Bed-shape gap (newly identified)**: no `bed_shape` config key exists today (grep across `crates/`, `modules/`, every `.toml` returned zero hits). The wipe-tower module has no way to validate tower placement. An earlier draft proposed a `host-services::print-bed-shape` WIT extension; that was rejected on senior-review as over-engineered. This packet declares `bed_shape` as a `float-list` config key, using the existing `config-value::float-list` variant — zero WIT change.

User reproduction (Slicer A = OrcaSlicer reference, Slicer B = Pinch 'n Print):

- Slicer A emits one `;TYPE:Prime tower` block per layer of a 292-layer multi-color print.
- Slicer B emits zero `;TYPE:Prime tower` blocks for the same print.
- Slicer B transitions straight from one filament's extrusion to `T<n>` and then to an extruding `G1 ... E+` move on the print model, leaving the previous color smeared on the part.

No prior packet is being superseded — each completed its declared scope. This packet closes the integration gap, adds three additive methods to `finalization-output-builder` (mirroring PathOptimization's capability surface for future packets), declares `bed_shape` as config, and corrects the marker spelling — shipped as a single-release bugfix with a `docs/DEVIATION_LOG.md` entry.

## Architecture Constraints

- `wipe-tower` runs in `PostPass::LayerFinalization` and mutates `&mut Vec<LayerCollectionIR>` via the `FinalizationOutputBuilder` action-recorder **struct** (whose `apply_to` impl method drains recorded actions). Senior-review audit confirmed migrating wipe-tower to `Layer::PathOptimization` is infeasible (three fatal blockers — see **Stage-Migration Rejected** below); the module stays in finalization.
- New entities added by wipe-tower use existing IR fields. Adding fields to `ToolChange` or `PrintEntity` is out of scope. `ExtrusionRole::WipeTower` already exists in `crates/slicer-ir/src/slice_ir.rs` (variant at ~line 1336; the surrounding `enum ExtrusionRole` block spans roughly 1318-1350; Step 1 reverifies exact lines) and is first-class in `wit/deps/types.wit:24-29`. `PrimeTower` is at ~line 1338; `Skirt` is the last variant in the same block.
- The `wipe_tower_enabled` config flag is the canonical gate. When `false`, wipe-tower skips emission entirely.
- Per `docs/02_ir_schemas.md` determinism contract, purge entity positions must be deterministic given the same input. Use `wipe_tower_x`, `wipe_tower_y`, `wipe_tower_width`, `line_width` from config — no RNG.
- Coordinate units: `ExtrusionPath3D.points` already in mm; bed_shape config values are mm. No unit conversion at boundaries.
- Per packet 11's emission contract, role labels are `;TYPE:<RoleName>`. **OrcaSlicer's canonical role-name for the wipe-tower extrusion is "Prime tower"** (`ExtrusionEntity.cpp:648`).
- `WipeTower` is first-class at the WIT boundary; host-to-WIT translation at `wit_host.rs:4747-4768` is direct passthrough. `PrimeTower`/`Skirt` use `Custom("slicer.builtin/...")` tags — not affected by this packet.

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
