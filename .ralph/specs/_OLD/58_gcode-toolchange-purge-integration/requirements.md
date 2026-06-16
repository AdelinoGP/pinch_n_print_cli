# Requirements: 58_gcode-toolchange-purge-integration

## Packet Metadata

- Grouped task IDs:
  - `TASK-143` — WipeTower live finalization (closed in packet 17; integration gap remains).
  - `TASK-152b` — Mixed-tool ordering / `push-tool-change` (closed in packet 19; emission gap remains).
  - `TASK-120d2` — Retract/unretract pair emission (closed in packet 15; toolchange-specific wrap missing).
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `active`
- Aggregate context cost: `M`

## Problem Statement

Three closed packets each completed their own slice cleanly, but the slices never met. Two refinement audits identified design errors in earlier drafts of this packet and corrected them.

- **Packet 17 (`17_wipe-tower-finalization-live-path`)** ported the `wipe-tower` module onto the live `run_finalization()` path. The module already reads `view.tool_changes()` and emits scan-line tower walls tagged `ExtrusionRole::WipeTower` (`modules/core-modules/wipe-tower/src/lib.rs:259-295`, role at lib.rs:187). **Gap**: `generate_purge_paths` (lib.rs:136-204) emits only rectilinear scan lines — no retract entity before the tower, no travel-to-tower move, no prime entity sized to `wipe_tower_purge_volume`.
- **Packet 19 (`19_path-optimization-tool-order-and-cooling-policy`)** added deterministic mixed-tool ordering and `push-tool-change` via the path-optimization-stage WIT. **Gap**: when `path-optimization-default` populates `LayerCollectionIR.tool_changes`, the downstream `GCodeEmitter` writes a bare `T<n>` line (`crates/slicer-host/src/gcode_emit.rs`, `emit_gcode` near lines 516-525; the bare-line serialization is near lines 1283-1284) with no defensive guard requiring surrounding purge entities.
- **Packet 11 (`11_orca-gcode-emission-contract`)** defined the canonical `;TYPE:<RoleName>` role labeling. **Gap**: `orca_type_label` at `crates/slicer-host/src/gcode_emit.rs:271` maps `ExtrusionRole::WipeTower → ";TYPE:Wipe tower"`. OrcaSlicer's canonical mapping (`OrcaSlicerDocumented/src/libslic3r/ExtrusionEntity.cpp:648`) is `erWipeTower → "Prime tower"`. Downstream tooling expecting `;TYPE:Prime tower` ignores ModularSlicer's section.
- **Positional-insertion gap (newly identified)**: `finalization-output-builder` exposes only `push-entity-to-layer` / `push-entity-with-priority`, both of which APPEND then stable-sort by `(priority, insertion_order)` (`crates/slicer-sdk/src/traits.rs::FinalizationOutputBuilder::apply_to` ≈ lines 918-958). There is **no API by which wipe-tower can place an entity at a specific index relative to `ToolChange.after_entity_index`**. The original packet-58 draft proposed solving the bug by "injecting entities" via `push-entity-with-priority`; that approach was infeasible — wipe-tower entities sort by role priority into a single cluster, not adjacent to the ToolChange. PathOptimization's `set-entity-order` (`wit/deps/ir-types.wit:139-170`) solves this for the layer stage but has no counterpart in finalization. This packet adds the missing primitives.
- **Intra-stage ordering risk (newly identified)**: `wipe-tower` is one of four modules in `PostPass::LayerFinalization` (alongside `skirt-brim`, `part-cooling`, `top-surface-ironing`). Per `docs/04_host_scheduler.md:762-765`, intra-stage order is determined by topological sort over the IR read/write DAG plus explicit `[compatibility].requires` edges; absent either signal, order is stable but undefined. If a sibling module runs after wipe-tower and calls `push-entity-with-priority`, the post-apply stable sort could land a non-wipe-tower entity between index `K` (the `ToolChange`'s reference) and index `K+1` (wipe-tower's retract), breaking the bracketing invariant. **Resolution**: this packet declares `[compatibility].requires` in `wipe-tower.toml` listing the three sibling finalization modules — the documented TOML primitive for intra-stage ordering (`docs/03_wit_and_manifest.md:817-822`).
- **Bed-shape gap (newly identified)**: no `bed_shape` config key exists today (grep across `crates/`, `modules/`, every `.toml` returned zero hits). The wipe-tower module has no way to validate tower placement. An earlier draft proposed a `host-services::print-bed-shape` WIT extension; that was rejected on senior-review as over-engineered. This packet declares `bed_shape` as a `float-list` config key, using the existing `config-value::float-list` variant — zero WIT change.

User reproduction (Slicer A = OrcaSlicer reference, Slicer B = ModularSlicer):

- Slicer A emits one `;TYPE:Prime tower` block per layer of a 292-layer multi-color print.
- Slicer B emits zero `;TYPE:Prime tower` blocks for the same print.
- Slicer B transitions straight from one filament's extrusion to `T<n>` and then to an extruding `G1 ... E+` move on the print model, leaving the previous color smeared on the part.

No prior packet is being superseded — each completed its declared scope. This packet closes the integration gap, adds three additive methods to `finalization-output-builder` (mirroring PathOptimization's capability surface for future packets), declares `bed_shape` as config, and corrects the marker spelling — shipped as a single-release bugfix with a `docs/DEVIATION_LOG.md` entry.

## In Scope

- Extend `wit/world-finalization.wit::finalization-output-builder` with three additive methods:
  - `insert-entity-at(layer-index, position: u32, path, region-key) -> result<_, string>` — positional insert; host-side `apply_to` remaps `ToolChange.after_entity_index` and `ZHop` indices past the insert position.
  - `set-entity-order(layer-index, items: list<tuple<u32, bool>>) -> result<_, string>` — permutation (one entry per existing entity); remaps tool-change/z-hop indices accordingly.
  - `get-ordered-entities(layer-index) -> list<print-entity-view>` — read-back of the layer's staged state.
- Implement the three new builder methods host-side in `crates/slicer-host/src/wit_host.rs` and in the SDK action-recorder **struct** `FinalizationOutputBuilder` at `crates/slicer-sdk/src/traits.rs` (the type is a struct that records builder actions for `apply_to` to drain — not a trait; the prior packet draft used "trait" wording which was incorrect).
- Rebuild every guest's bindgen output via `./modules/core-modules/build-core-modules.sh`; verify `--check` reports fresh.
- Declare `bed_shape: float-list` in `modules/core-modules/wipe-tower/wipe-tower.toml`'s `[config.schema]` block. Format: `[x0, y0, x1, y1, …]` mm. Add the same key to the host-side single source of truth at `crates/slicer-ir/src/resolved_config.rs` (the macro-driven `declare_resolved_config!` invocation introduced by commit `19e5791`), so `ConfigView` carries it for guest modules.
- Declare `[compatibility].requires = ["com.core.skirt-brim", "com.core.part-cooling", "com.core.top-surface-ironing"]` in `modules/core-modules/wipe-tower/wipe-tower.toml` to force wipe-tower last in `PostPass::LayerFinalization` via the documented intra-stage ordering primitive (`docs/03_wit_and_manifest.md:817-822`; DAG edge creation in `crates/slicer-host/src/dag.rs:93-102`). This locks the `K+1` adjacency invariant against later sibling-module reordering. No new manifest key is added — this uses the existing `[compatibility].requires` mechanism.
- Extend `modules/core-modules/wipe-tower/src/lib.rs::generate_purge_paths` to emit, around each `ToolChange`: one retract entity, one travel-to-tower entity, the existing tower wall + rectilinear infill entities, and one prime entity whose cumulative positive E delta equals `wipe_tower_purge_volume` mm.
- Make `run_finalization` use `insert-entity-at(after_entity_index + 1)` to place purge entities adjacent to the `ToolChange`, bracketing the `T<n>` emission. Read `bed_shape` from config and validate tower vertices.
- Change `orca_type_label` at `crates/slicer-host/src/gcode_emit.rs:271` so `ExtrusionRole::WipeTower → ";TYPE:Prime tower"`.
- Add `PostpassError::MissingToolchangePurge { layer_index, tool_change_index }` additively to the enum at `crates/slicer-host/src/postpass.rs:39-59`, plus a defensive guard in `gcode_emit.rs`.
- Fixture: `crates/slicer-host/tests/fixtures/multi_color_cube.stl` + `multi_color_cube.orca.gcode` (committed by the original packet for AC2a/AC2b/AC5/NC2/NC3; retained as historical artifact). Post-review 2026-05-19: the live file-level scripts are retargeted at `resources/benchy_4color.3mf` (committed multi-material 3MF carrying an embedded prime-tower object) + `crates/slicer-host/tests/fixtures/benchy_4color.config.json` (sets `wipe_tower_enabled=true` and wipe-tower coords from the 3MF). See packet.spec.md AC retargeting note.
- New test files:
  - `crates/slicer-host/tests/gcode_toolchange_wrapping.rs` (AC1, AC3, NC1) — TDD-first.
  - `crates/slicer-host/tests/finalization_builder_insert.rs` (AC7, NC5).
  - `crates/slicer-host/tests/finalization_builder_permute.rs` (AC8, NC6).
  - `crates/slicer-host/tests/finalization_builder_readback.rs` (AC9).
  - `crates/slicer-host/tests/wipe_tower_bed_bounds.rs` (AC6).
- New unit tests in `modules/core-modules/wipe-tower/src/lib.rs#tests` for AC4 (`emits_prime_tower_role_marker`) and NC4 (`tower_outside_bed_returns_fatal`).
- One `docs/DEVIATION_LOG.md` entry covering the integration completion, the three new builder methods, the `bed_shape` config addition, and the marker spelling correction.
- One-paragraph addition to `docs/03_wit_and_manifest.md`'s finalization-builder section describing the three new methods and the index-remap invariants.

## Out of Scope

- Any **module-facing** config key beyond `bed_shape`. Reuse `wipe_tower_enabled`, `wipe_tower_purge_volume`, `wipe_tower_x`, `wipe_tower_y`, `wipe_tower_width`, `line_width`.
- Ramming/cooling-tube dynamics from OrcaSlicer (parity-deferred).
- Tree/grid tower interior infill — keep rectilinear from packet 17.
- The 3-release N/N+1/N+2 rollout from `docs/11` — single-release bugfix.
- **Any change to `host-services`** — the prior draft's `print-bed-shape` accessor is rejected; bed shape is config.
- First-classing `extrusion-role::prime-tower` or `extrusion-role::skirt`. Custom-tag round-trip is unchanged.
- Changes to `layer-collection-view` or any other `wit/world-*.wit` resource beyond `finalization-output-builder`.
- Changes to PathOptimization's `layer-collection-builder` — the three new methods are added on the **finalization** builder; PathOptimization's builder is untouched.
- Other `PostPass::LayerFinalization` modules (`skirt-brim`, `part-cooling`, `top-surface-ironing`) — read-only invariant check only (Step 5 dispatch).
- Moving wipe-tower to a different stage. The senior-review audit confirmed three fatal blockers for migrating to `Layer::PathOptimization` (no entity-add; no tool_changes read; no z) — see `design.md` for the full record.

## Authoritative Docs

- `docs/02_ir_schemas.md` — > 600 lines; **delegate**.
- `docs/03_wit_and_manifest.md` — > 300 lines; **delegate** for `finalization-output-builder` exports + `config-value` types + module manifest `[config.schema]` syntax. Step 7 adds the new-methods paragraph.
- `docs/04_host_scheduler.md` — direct read of the LayerFinalization → GCodeEmit transition.
- `docs/08_coordinate_system.md` — direct read.
- `docs/09_progress_events.md` — direct read.
- `docs/11_operational_governance_and_acceptance_gate.md` — range-read §1.

## OrcaSlicer Reference Obligations

All delegated. Never load `OrcaSlicerDocumented/` directly.

- `ExtrusionEntity.cpp:628-654` — canonical role-to-string mapping. `erWipeTower → "Prime tower"` (line 648).
- `WipeTower2.cpp:1557-1640` — Unload/Change/Load/Wipe ordering.
- `Print.cpp:3180-3268` — per-layer toolchange planning.
- `GCode.cpp:7624` — `set_extruder()` retract → toolchange flow.
- `WipeTower2.cpp:2258-2270` — `flush_volumes_matrix` purge-volume reference.

## Acceptance Summary

- **Positive cases (nine)**:
  1. IR-level: `ToolChange` bracketed by retract + travel + prime + wipe entities.
  2a. Retract precedes every `T<n>` within 5 lines.
  2b. Positive-`E` `G1` follows every `T<n>` within 10 lines.
  3. Per-`(from_tool, to_tool)` purge volume within ±20% of OrcaSlicer reference.
  4. `;TYPE:Prime tower` marker emitted exactly once per wipe-tower block.
  5. Marker count ≥ L for L tool-change-containing layers.
  6. Tower polygon within config-supplied bed polygon and outside object footprints.
  7. `insert-entity-at` semantics (index remap on insert).
  8. `set-entity-order` semantics on finalization builder (index remap on permute).
  9. `get-ordered-entities` read-back reflects staged state.

- **Negative cases (six)**:
  1. `ToolChange` without surrounding purge entities → `PostpassError::MissingToolchangePurge`.
  2. Bare `T<n>` followed by extruding `G1 ... E+` → CLI scan exits non-zero.
  3. Multi-tool file with zero `;TYPE:Prime tower` → CLI scan exits non-zero.
  4. Tower placed outside config-supplied bed → fatal `ModuleError`.
  5. `insert-entity-at` with out-of-bounds position → `Err`, layer state unchanged.
  6. `set-entity-order` with malformed proposal → `Err`, layer state unchanged.

- **Measurable outcomes**: zero `MissingToolchangePurge` errors on the fixture; ≥ 1 `;TYPE:Prime tower` marker per tool-change layer; purge volume per `(from, to)` pair within `[0.80, 1.20]` × OrcaSlicer reference; tower vertices inside config bed polygon; index remap invariants hold across insert/permute; no clippy warnings; all guest `.wasm` artifacts fresh.

- **Cross-packet impact**: no packet supersession. The three new `finalization-output-builder` methods unblock any future finalization-stage module that needs positional control or permutation (travel-optimization in finalization, multi-material sort, color-grouping reorder, etc.).

## Verification Commands

- `cargo check --workspace`
- `cargo clippy --workspace -- -D warnings`
- `cargo test -p slicer-host --test gcode_toolchange_wrapping`
- `cargo test -p slicer-host --test finalization_builder_insert`
- `cargo test -p slicer-host --test finalization_builder_permute`
- `cargo test -p slicer-host --test finalization_builder_readback`
- `cargo test -p slicer-host --test wipe_tower_bed_bounds`
- `cargo test -p wipe-tower`
- `./modules/core-modules/build-core-modules.sh` (mandatory after WIT extension)
- `./modules/core-modules/build-core-modules.sh --check` (must report fresh)
- `cargo run --bin slicer-host --release -- run --module modules/core-modules/machine-gcode-emit/machine-gcode-emit.wasm --model resources/benchy_4color.3mf --module-dir modules/core-modules --config crates/slicer-host/tests/fixtures/benchy_4color.config.json --output target/test-output/benchy_4color.gcode` (retargeted post-review 2026-05-19: original line named the non-existent `slicer-cli --slice` binary and ran against an empty single-material STL — the canonical CLI is `slicer-host run`, and the live multi-material verification target is `resources/benchy_4color.3mf` + `benchy_4color.config.json`)
- AC and NC awk/python scripts from `packet.spec.md` against the produced G-code.

All delegation-friendly (exit code, FACT pass/fail).

## Step Completion Expectations

See `implementation-plan.md` for per-step preconditions, postconditions, falsifying checks, files-allowed-to-read/edit, expected sub-agent dispatches, and per-step S/M cost.

## Context Discipline Notes

- **Large files to range-read or delegate**:
  - `docs/02_ir_schemas.md` (> 600 lines) — delegate via SUMMARY.
  - `crates/slicer-ir/src/slice_ir.rs` (~ 1500+ lines) — range-read at `≈ 1318-1350` (`ExtrusionRole`; `WipeTower` at ~1336, `PrimeTower` at ~1338, `Skirt` last); other ranges located by Step 1 dispatch (`ToolChange`, `TravelRetract`, `LayerCollectionIR.tool_changes`, `ConfigValue`).
  - `crates/slicer-host/src/gcode_emit.rs` (~ 1300 lines) — range-read at `259-276` (`orca_type_label`; arm at 271), `385-410`, `516-525`, `1275-1290`.
  - `crates/slicer-host/src/wit_host.rs` — `finalization-output-builder` host impl + `host-services` impl (Step 1 dispatch locates).
  - `crates/slicer-sdk/src/traits.rs` (large) — range-read `apply_to` at ≈ 918-958 and the `FinalizationOutputBuilder` **struct** definition (at ~line 704, NOT a trait; Step 1 dispatch reverifies).
- **Out-of-bounds**: all of `OrcaSlicerDocumented/`, `target/`, `Cargo.lock`, vendored deps, and crates not on the change list. Every `wit/world-*.wit` other than `world-finalization.wit` — read-only invariant check in Step 5.
- **Temptation reads to skip**:
  - The full `crates/slicer-ir/src/slice_ir.rs`. Range-read only.
  - Other core-modules' source. Wipe-tower only.
  - OrcaSlicer source bodies. Delegate.
  - `docs/07_implementation_status.md` in full. Locate three TASK-### ranges via dispatch.
  - `crates/slicer-sdk/src/traits.rs` in full. Range-read only.
- **Sub-agent return-format hints**:
  - `ExtrusionRole` enum query: FACT, ≤ 5 lines.
  - Wipe-tower `run_finalization` + `generate_purge_paths`: SNIPPETS, ≤ 3 of ≤ 30 lines each.
  - OrcaSlicer parity queries: LOCATIONS + one-line role.
  - Host-side `ResolvedConfig` macro invocation locator: FACT — file path + first ≤ 5 lines of the `declare_resolved_config!` block in `crates/slicer-ir/src/resolved_config.rs`. (Per commit `19e5791`, this macro is the SoT for printer-profile config keys; the new `bed_shape` field is added inside this invocation.)
  - Existing retract-distance config key locator: FACT — search `crates/slicer-ir/src/resolved_config.rs` and `modules/core-modules/wipe-tower/wipe-tower.toml` for an existing key like `retract_length`, `retraction_distance`, `retract_distance`, or similar. If one exists, return name + type; if none, FACT "no existing retract-distance config key" so Step 5a can declare a new one (no hand-coded default).
  - `FinalizationOutputBuilder` SDK **struct** location, its action-enum, and existing method list: SNIPPET ≤ 30 lines.
  - `apply_to` index-remap logic touchpoints: LOCATIONS ≤ 5 entries.
