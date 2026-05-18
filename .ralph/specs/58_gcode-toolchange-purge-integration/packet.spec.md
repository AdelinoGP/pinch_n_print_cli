---
status: draft
packet: 58_gcode-toolchange-purge-integration
task_ids:
  - TASK-143
  - TASK-152b
  - TASK-120d2
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
copy_note: This packet closes an integration gap between five prior implemented packets (17, 19, 11, 15, 34). It does not supersede any of them. The `task_ids:` listed in this frontmatter were each previously closed by their owning packet (TASK-143 by 17, TASK-152b by 19, TASK-120d2 by 15); this packet reopens them at the integration layer and re-closes them via Step 7's `docs/07_implementation_status.md` update. A senior-reviewer critique of an earlier draft revealed two design errors that this version corrects — (a) a proposed `host-services::print-bed-shape` WIT extension was over-engineered; bed shape is now declared as a `bed_shape: float-list` config key (zero WIT change); (b) the original "inject retract/travel/prime entities via push-entity-with-priority" approach was infeasible because `finalization-output-builder` has no positional insertion — this packet adds three additive methods (`insert-entity-at`, `set-entity-order`, `get-ordered-entities`) to that builder, mirroring PathOptimization's `layer-collection-builder` capability surface.
---

# Packet Contract: 58_gcode-toolchange-purge-integration

## Goal

Wire the existing `wipe-tower` module's per-layer output through the live G-code emission path so every `T<n>` tool-change token in the final `.gcode` is bracketed by a retract → travel → load/prime → wipe sequence, and so every layer that contains at least one tool change emits a `;TYPE:Prime tower` block matching OrcaSlicer's canonical marker (`OrcaSlicerDocumented/src/libslic3r/ExtrusionEntity.cpp:648` — `erWipeTower → "Prime tower"`).

Today three problems stack:

1. **No positional insertion in finalization.** `finalization-output-builder` only has `push-entity-to-layer` / `push-entity-with-priority`, which APPEND and then stable-sort by `(priority, insertion_order)` (`crates/slicer-sdk/src/traits.rs:918-956`). Wipe-tower cannot place an entity at a specific index relative to `ToolChange.after_entity_index`. The `T<n>` is emitted between entities `K` and `K+1`; today there is no API by which wipe-tower can guarantee its tower entities land at position `K+1`. The `set-entity-order` method that solves this in `Layer::PathOptimization` (`wit/deps/ir-types.wit:167-170`) has no counterpart in finalization.
2. **`generate_purge_paths` emits only scan-line walls** (`modules/core-modules/wipe-tower/src/lib.rs:136-204`) — no retract, no travel-to-tower, no prime entity sized to `wipe_tower_purge_volume`.
3. **`orca_type_label` emits the wrong marker spelling.** At `crates/slicer-host/src/gcode_emit.rs:271`, `ExtrusionRole::WipeTower → ";TYPE:Wipe tower"` diverges from OrcaSlicer's canonical `";TYPE:Prime tower"`.

This packet closes all three.

## Scope Boundaries

- **In scope**:
  - Extend `wit/world-finalization.wit::finalization-output-builder` with three additive methods (full WIT delta tabulated in **WIT Impact** below):
    - `insert-entity-at: func(layer-index, position: u32, path, region-key) -> result<_, string>` — positional insert; shifts later entities right and remaps `ToolChange.after_entity_index` / `ZHop` indices past the insert point.
    - `set-entity-order: func(layer-index, items: list<tuple<u32, bool>>) -> result<_, string>` — permutation (mirrors PathOptimization's contract: exactly one entry per existing entity in the staged ordering). Remaps tool-change/z-hop indices accordingly.
    - `get-ordered-entities: func(layer-index) -> list<print-entity-view>` — read-back of the layer's currently staged entity list (useful for any module wanting to validate before mutating; mirrors PathOptimization's `get-ordered-entities`).
  - Declare `bed_shape: float-list` in `modules/core-modules/wipe-tower/wipe-tower.toml`'s `[config.schema]` block. Format: `[x0, y0, x1, y1, …]` mm. Wipe-tower reads via `config.get("bed_shape")`. No `host-services` WIT change. The same key is added to the host-side macro-driven `declare_resolved_config!` invocation in `crates/slicer-ir/src/resolved_config.rs` (the SoT after commit `19e5791`) so the host can populate `ConfigView` for guest modules.
  - **Force wipe-tower to run last in `PostPass::LayerFinalization`** via its TOML manifest's `[compatibility].requires`, listing the three sibling finalization modules. Per `docs/03_wit_and_manifest.md:817-822` and `docs/04_host_scheduler.md:762-765`, this is the documented intra-stage ordering primitive: the DAG builder (`crates/slicer-host/src/dag.rs:93-102`) creates an `A → wipe-tower` edge for every `A` listed, forcing wipe-tower after all of them. This locks the `K+1` adjacency invariant against later priority-sort interleaving by sibling finalization modules.
  - Extend `modules/core-modules/wipe-tower/src/lib.rs::generate_purge_paths` to emit, around each `ToolChange`: (a) one retract entity (negative E delta) tagged `ExtrusionRole::WipeTower`; (b) one travel entity to `(wipe_tower_x, wipe_tower_y)` at zero E delta; (c) the existing tower wall + rectilinear scan-line entities; (d) one prime entity whose cumulative positive E delta equals `wipe_tower_purge_volume` mm via `length_mm = volume_mm3 / (line_width_mm * layer_height_mm)`.
  - Make `run_finalization` use the new `insert-entity-at(layer_index, after_entity_index + 1, …)` to place purge entities adjacent to the `ToolChange`'s `after_entity_index` — bracketing the `T<n>` emission.
  - Change `orca_type_label` at `crates/slicer-host/src/gcode_emit.rs:271` so `ExtrusionRole::WipeTower → ";TYPE:Prime tower"` (one-line string change driven by OrcaSlicer parity).
  - Add `PostpassError::MissingToolchangePurge { layer_index, tool_change_index }` additively to the enum at `crates/slicer-host/src/postpass.rs:39-59`, plus a defensive guard in `gcode_emit.rs` that returns this when a `ToolChange` is not bracketed by purge entities under `wipe_tower_enabled=true`.
  - Validate tower placement against the config-supplied `bed_shape` polygon. AC6 / NC4 run against the real polygon.
  - Check in one synthetic multi-material STL at `crates/slicer-host/tests/fixtures/multi_color_cube.stl` and one OrcaSlicer reference G-code at `crates/slicer-host/tests/fixtures/multi_color_cube.orca.gcode`.
- **Out of scope**:
  - Any new **module-facing** config key beyond `bed_shape`. Reuse `wipe_tower_enabled`, `wipe_tower_purge_volume`, `wipe_tower_x`, `wipe_tower_y`, `wipe_tower_width`, `line_width`.
  - Ramming and cooling-tube load-dynamics modeling (deferred — borrow call ordering, not the velocity profile).
  - Tree/grid tower interior infill beyond the rectilinear pattern from packet 17.
  - The 3-release N/N+1/N+2 rollout from `docs/11`. Single-release bugfix with DEVIATION_LOG entry.
  - **Any change to `host-services`** — the prior draft's `print-bed-shape` accessor is rejected; bed shape is now config.
  - First-classing `extrusion-role::prime-tower` or `extrusion-role::skirt` in `wit/deps/types.wit`. Custom-tag round-trip (`crates/slicer-host/src/wit_host.rs:4747-4768`) is unchanged. Wipe-tower only emits `WipeTower` (first-class).
  - Changes to `layer-collection-view` or any other `wit/world-*.wit` resource beyond `finalization-output-builder`.
  - Changes to PathOptimization's `layer-collection-builder` — the three new methods are added on the **finalization** builder; the PathOptimization builder is untouched.

## WIT Impact (explicit)

| Surface | Change | Rationale |
| --- | --- | --- |
| `wit/world-finalization.wit::finalization-output-builder` | **ADD** `insert-entity-at(layer-index, position: u32, path, region-key) -> result<_, string>` | The actual ordering primitive wipe-tower needs. Solves the "T<n> bracketing" problem the prior draft's `push-entity-with-priority` could not. |
| `wit/world-finalization.wit::finalization-output-builder` | **ADD** `set-entity-order(layer-index, items: list<tuple<u32, bool>>) -> result<_, string>` | Mirrors PathOptimization's permutation contract for finalization. Likely needed by future packets (travel optimization in finalization, multi-material sort, …). Not exercised by this packet beyond a smoke test. |
| `wit/world-finalization.wit::finalization-output-builder` | **ADD** `get-ordered-entities(layer-index) -> list<print-entity-view>` | Mirrors PathOptimization's read-back contract for finalization. Lets modules validate their proposed inserts/permutes before committing. Smoke-tested by this packet. |
| `wit/host-api.wit::host-services` | **NONE** | Prior draft's `print-bed-shape` rejected — bed shape is config. |
| `wit/deps/types.wit::extrusion-role` | **NONE** | `wipe-tower` is first-class (line 27). |
| `wit/world-finalization.wit::layer-collection-view` | **NONE** | `tool-changes` accessor already exists (lines 13-40). |
| `wit/deps/config.wit::config-value` | **NONE** | `float-list(list<f64>)` already exists; `bed_shape` encodes as `[x0, y0, x1, y1, …]`. |
| Module manifest: `modules/core-modules/wipe-tower/wipe-tower.toml` | **ADD** `[config.schema.bed_shape]` entry of type `float-list`, required; **ADD** `[compatibility].requires = ["com.core.skirt-brim", "com.core.part-cooling", "com.core.top-surface-ironing"]` to force wipe-tower last in `PostPass::LayerFinalization` | Declares the new config key the module reads; uses the documented `[compatibility].requires` ordering primitive (`docs/03_wit_and_manifest.md:817-822`) to lock `K+1` adjacency against sibling reordering. |
| `crates/slicer-ir/src/resolved_config.rs` (macro-driven SoT after commit `19e5791`) | **ADD** `bed_shape: List<f64>` field to the `declare_resolved_config!` invocation; default `[0.0, 0.0, 250.0, 0.0, 250.0, 250.0, 0.0, 250.0]` | Lets the host populate `ConfigView` for guest modules through the post-refactor single source of truth. |
| `crates/slicer-host/src/wit_host.rs` | **ADD** host-side impl of three new builder methods; **ADD** index-remap bookkeeping for `ToolChange.after_entity_index` and `ZHop` indices on insert/permute | Required to satisfy the new WIT exports at runtime. The remap invariant is a locked correctness contract — see **Locked Invariants** below. |
| Guest bindings (every core-module's `wit-bindgen` output) | **REBUILD** via `./modules/core-modules/build-core-modules.sh` | Per `CLAUDE.md` Guest WASM Staleness rules — adding methods to a WIT resource invalidates every guest. |

## Prerequisites and Blockers

- **Depends on**: packets 17, 19, 11, 15, 34 — all `implemented`.
- **Unblocks**: any downstream multi-material end-to-end correctness packet; any future finalization-stage module that needs positional control or permutation (now possible via the three new builder methods).
- **Activation blockers**: none. No other packet is currently `active`.

## Acceptance Criteria

- **AC1 — IR-level bracketing**: **Given** a `LayerCollectionIR` containing one `ToolChange { from_tool: 0, to_tool: 1, after_entity_index: K }` and `wipe_tower_enabled=true`, **when** `GCodeSerializer` serializes the layer, **then** the produced text contains, in order: at least one entity emitting negative `E` delta (retract), at least one `G1` travel move to the tower X/Y, the literal line `T1`, at least one entity emitting cumulative positive `E` delta ≥ `wipe_tower_purge_volume` mm (the prime+wipe), and the literal line `;TYPE:Prime tower` appears before the first of these new entities; the next print-role extrusion appears only after the wipe block ends. | `cargo test -p slicer-host --test gcode_toolchange_wrapping toolchange_emits_retract_prime_wipe -- --nocapture`

- **AC2a — retract precedes `T<n>`**: as before. | `awk '/^T[0-9]/{ok=0; for(i=NR-5;i<NR;i++) if(i>0 && prev[i]~/E-/) ok=1; if(!ok){print "no retract before line "NR": "$0; bad=1}} {prev[NR]=$0} END{exit bad+0}' target/test-output/multi_color_cube.gcode`

- **AC2b — positive-`E` `G1` follows `T<n>` within 10 lines**: as before. | `awk '{lines[NR]=$0} END{bad=0; for(i=1;i<=NR;i++) if(lines[i]~/^T[0-9]/){ok=0; for(j=i+1;j<=i+10 && j<=NR;j++) if(lines[j]~/^G1.*E[0-9]/ && lines[j]!~/E-/){ok=1; break} if(!ok){print "no prime after line "i": "lines[i]; bad=1}} exit bad+0}' target/test-output/multi_color_cube.gcode`

- **AC3 — purge volume parity ±20% vs OrcaSlicer**: as before. | `cargo test -p slicer-host --test gcode_toolchange_wrapping purge_volume_within_tolerance -- --nocapture`

- **AC4 — `;TYPE:Prime tower` marker emitted**: as before, asserting the exact spelling `;TYPE:Prime tower`. | `cargo test -p wipe-tower --lib emits_prime_tower_role_marker -- --nocapture`

- **AC5 — marker count ≥ tool-change layers**: as before. | `python -c "import re,sys; lines=open('target/test-output/multi_color_cube.gcode').readlines(); tc_layers=len({i for i,l in enumerate(lines) if re.match(r'T[0-9]+',l)}); pt=sum(1 for l in lines if ';TYPE:Prime tower' in l); sys.exit(0 if pt>=tc_layers else 1)"`

- **AC6 — tower placement against config-supplied bed polygon**: **Given** `wipe_tower_enabled=true` and `bed_shape=[0.0, 0.0, 250.0, 0.0, 250.0, 250.0, 0.0, 250.0]` set in the printer profile, **when** the wipe-tower module emits a tower polygon for the first layer of the multi-material fixture, **then** every tower vertex is inside the bed polygon AND the tower polygon does not intersect any object's footprint bounding box (via existing `host-services::object-bounds`). | `cargo test -p slicer-host --test wipe_tower_bed_bounds tower_geometry_within_config_bed_outside_objects -- --nocapture`

- **AC7 — `insert-entity-at` semantics**: **Given** a layer with `N` staged entities and a `ToolChange` at `after_entity_index=K` (`0 ≤ K < N`), **when** a module calls `insert-entity-at(layer_index, position=K+1, path, region_key)`, **then** after `apply_to`: (a) the new entity occupies index `K+1` in `ordered_entities`; (b) the original entities at indices `K+1..N` shift to `K+2..N+1`; (c) the `ToolChange.after_entity_index` is still `K` (unchanged because the insert is AFTER the tool change's reference entity); (d) any other `ToolChange` with `after_entity_index >= K+1` is incremented by 1; (e) any `ZHop` index `>= K+1` is incremented by 1. | `cargo test -p slicer-host --test finalization_builder_insert insert_at_position_remaps_indices -- --nocapture`

- **AC8 — `set-entity-order` semantics on finalization builder**: **Given** a layer with 3 staged entities (indices 0, 1, 2), **when** a module calls `set-entity-order(layer_index, [(2, false), (0, false), (1, false)])`, **then** after `apply_to` the entities occupy indices in the order `[original[2], original[0], original[1]]` and any `ToolChange.after_entity_index` is remapped through the same permutation. | `cargo test -p slicer-host --test finalization_builder_permute set_entity_order_remaps_indices -- --nocapture`

- **AC9 — `get-ordered-entities` read-back**: **Given** a module that has pushed 2 entities and called `insert-entity-at` once, **when** the module calls `get-ordered-entities(layer_index)`, **then** the returned list reflects the staged state including the inserted entity at its declared position. | `cargo test -p slicer-host --test finalization_builder_readback get_ordered_entities_reflects_staged_state -- --nocapture`

## Negative Test Cases

- **NC1 — bare `T<n>` without surrounding purge rejected**: as before. | `cargo test -p slicer-host --test gcode_toolchange_wrapping bare_toolchange_rejected -- --nocapture`

- **NC2 — produced gcode has no bare `T<n>` → extruding `G1`**: as before. | `awk '/^T[0-9]/{getline n; while(n~/^;/||n==""){getline n} if(n ~ /G1.*E[0-9]/ && n !~ /E-/){print "no prime: "$0" then "n; exit 1}}' target/test-output/multi_color_cube.gcode`

- **NC3 — multi-tool gcode without any `;TYPE:Prime tower` marker rejected**: as before. | `python -c "import sys,re; lines=open('target/test-output/multi_color_cube.gcode').readlines(); tools=set(re.match(r'(T[0-9]+)',l).group(1) for l in lines if re.match(r'T[0-9]+',l)); markers=sum(1 for l in lines if ';TYPE:Prime tower' in l); sys.exit(1 if len(tools)>1 and markers==0 else 0)"`

- **NC4 — tower placed outside config-supplied bed rejected**: **Given** `wipe_tower_enabled=true`, `bed_shape=[0.0, 0.0, 100.0, 0.0, 100.0, 100.0, 0.0, 100.0]` (100 mm × 100 mm), and `wipe_tower_x=150.0` / `wipe_tower_y=150.0`, **when** the wipe-tower module emits geometry, **then** the module returns a fatal `ModuleError` naming the violating coordinate. | `cargo test -p wipe-tower --lib tower_outside_bed_returns_fatal -- --nocapture`

- **NC5 — `insert-entity-at` with out-of-bounds position rejected**: **Given** a layer with 3 staged entities, **when** a module calls `insert-entity-at(layer_index, position=99, …)`, **then** the call returns `Err("position 99 out of bounds; layer has 3 entities")` (or equivalent), and the layer's staged state is unchanged. | `cargo test -p slicer-host --test finalization_builder_insert insert_at_oob_position_rejected -- --nocapture`

- **NC6 — `set-entity-order` malformed proposal rejected**: **Given** a layer with 3 entities, **when** a module calls `set-entity-order` with `[(0, false), (0, false), (2, false)]` (duplicate index 0, missing 1), **then** the call returns `Err` and the layer's staged state is unchanged. | `cargo test -p slicer-host --test finalization_builder_permute set_entity_order_malformed_rejected -- --nocapture`

## Verification

Supplemental packet-level commands (not per-criterion):

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
- `cargo run --bin slicer-cli --release --slice --input crates/slicer-host/tests/fixtures/multi_color_cube.stl --output target/test-output/multi_color_cube.gcode`

(`cargo test --workspace` is invoked exactly once at the acceptance ceremony in `implementation-plan.md`, per Test Discipline.)

## Locked Invariants

- **Wipe-tower runs last in finalization stage**: `[compatibility].requires` in `wipe-tower.toml` lists `com.core.skirt-brim`, `com.core.part-cooling`, `com.core.top-surface-ironing`. The DAG builder forces wipe-tower after each declared module, so no sibling finalization module can perturb the `K+1` adjacency of inserted retract/travel/prime/wipe entities via subsequent `push-entity-with-priority` calls. Tradeoff: per `[compatibility].requires` semantics, wipe-tower will refuse to load if any listed module is missing from the active configuration — acceptable because all three are core modules shipped together as the default finalization roster.
- **Index remap on insert**: when `insert-entity-at(layer, position, …)` is applied, every existing `ToolChange.after_entity_index >= position` is incremented by 1, and every `ZHop.after_entity_index >= position` is incremented by 1. A `ToolChange.after_entity_index < position` is unchanged.
- **Index remap on permute**: when `set-entity-order(layer, items)` is applied, every `ToolChange.after_entity_index` is remapped via the permutation (if entity at old index `K` moves to new index `K'`, the corresponding tool change references update to `K'`). Same for `ZHop.after_entity_index`.
- **Atomicity**: a malformed `insert-entity-at` (out-of-bounds position) or `set-entity-order` (length mismatch, duplicate, or out-of-range index) returns an `Err` and leaves the layer's staged state unmodified. No partial application.
- **Single permutation per layer per `run_finalization`**: `set-entity-order` can be called at most once per `(layer, module, run_finalization invocation)`, matching PathOptimization's single-permutation contract at `crates/slicer-sdk/src/layer_collection_builder.rs:53-71`.
- **`get-ordered-entities` reflects staged state**: the read-back includes both pre-existing entities (from prior stages) and any entities the calling module has pushed or inserted during this `run_finalization`.

## Authoritative Docs

- `docs/02_ir_schemas.md` — > 600 lines; **delegate via SUMMARY**.
- `docs/03_wit_and_manifest.md` — **range-read** wipe-tower manifest schema, `finalization-output-builder` exports, `config-value` types, and the module manifest's `[config.schema]` syntax. The new three builder methods need a one-paragraph addition (Step 7 covers).
- `docs/04_host_scheduler.md` — **direct read** of the LayerFinalization → GCodeEmit transition only.
- `docs/08_coordinate_system.md` — **direct read** (units math for tower geometry).
- `docs/09_progress_events.md` — **direct read**; confirm no progress event is being violated.
- `docs/11_operational_governance_and_acceptance_gate.md` — **range-read §1** (DEVIATION_LOG entry format).

## OrcaSlicer Reference Obligations

All reads delegated; never load into the implementer's context.

- `OrcaSlicerDocumented/src/libslic3r/ExtrusionEntity.cpp:628-654` — **canonical role-to-string mapping**. `erWipeTower → "Prime tower"` (line 648).
- `OrcaSlicerDocumented/src/libslic3r/GCode/WipeTower2.cpp:1557-1640` — Unload/Change/Load/Wipe ordering.
- `OrcaSlicerDocumented/src/libslic3r/Print.cpp:3180-3268` — per-layer toolchange planning.
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp:7624` — `set_extruder()` retract → toolchange flow.
- `OrcaSlicerDocumented/src/libslic3r/GCode/WipeTower2.cpp:2258-2270` — `flush_volumes_matrix` purge-volume reference.

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`

## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents must treat `design.md`'s code change surface as authoritative, honor the out-of-bounds list, delegate every cargo run and authoritative-doc fact-check, stop reading at 60% context, and hand off at 85%.

Aggregate context cost is the sum of per-step costs in `implementation-plan.md`. No single step is L.
