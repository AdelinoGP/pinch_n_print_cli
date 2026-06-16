# Task Map: 58_gcode-toolchange-purge-integration

This file is the explicit bridge from packet steps to `docs/07_implementation_status.md` task IDs. It also names the prior packets whose integration gap this packet closes (no supersession — each prior packet stays `implemented`).

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-143` (WipeTower live finalization) | Steps 1, 2, 3, 5, 6, 7 | `docs/02_ir_schemas.md`, `docs/03_wit_and_manifest.md`, `docs/04_host_scheduler.md`, `docs/08_coordinate_system.md` | `modules/core-modules/wipe-tower/src/lib.rs` (extend `generate_purge_paths` with retract/travel/prime + bed-bounds check + tests); `modules/core-modules/wipe-tower/wipe-tower.toml` (`[config.schema.bed_shape]` + `[compatibility].requires` for run-last ordering); `wit/world-finalization.wit` (3 additive builder methods); `crates/slicer-sdk/src/traits.rs` (`FinalizationOutputBuilder` **struct** impl extension + `apply_to` index remap — NOT a trait); `crates/slicer-host/src/wit_host.rs` (host-side impl); `crates/slicer-ir/src/resolved_config.rs` (`bed_shape` field in the `declare_resolved_config!` macro invocation; also `retract_length` if not already present) | `WipeTower2.cpp:1557-1640`, `WipeTower2.cpp:2069-2205`, `ExtrusionEntity.cpp:628-654` | M | Wipe-tower already emits scan-line walls; this packet adds the surrounding retract/travel/prime entities, uses the new `insert-entity-at` to position them adjacent to `ToolChange.after_entity_index`, declares `[compatibility].requires` listing the three sibling finalization modules to force wipe-tower last in its stage (locking the `K+1` adjacency invariant), validates against the config-supplied bed polygon, and corrects the `;TYPE:` marker for OrcaSlicer parity. |
| `TASK-152b` (Mixed-tool ordering / push-tool-change) | Steps 2, 4, 6, 7 | `docs/02_ir_schemas.md`, `docs/04_host_scheduler.md` | `crates/slicer-host/src/gcode_emit.rs` (`orca_type_label` line 271 spelling fix; defensive guard near 516-525); new `crates/slicer-host/tests/gcode_toolchange_wrapping.rs` | `Print.cpp:3180-3268`, `GCode.cpp:7624` | S | `push-tool-change` (packet 19) deposits `ToolChange` entries; this packet ensures the emitter labels them with the canonical `;TYPE:Prime tower` marker and refuses to write a bare `T<n>` without surrounding purge entities. |
| `TASK-120d2` (Retract/unretract pair emission) | Steps 2, 4, 6, 7 | `docs/02_ir_schemas.md` | `crates/slicer-host/src/gcode_emit.rs` (missing-purge guard); `crates/slicer-host/src/postpass.rs::PostpassError::MissingToolchangePurge { layer_index: u32, tool_change_index: u32 }` additive variant (u32 to match IR `after_entity_index`) | `WipeTower2.cpp:1619-1640` | S | Travel retract (packet 15) covers cross-position moves; this packet adds the toolchange-specific guard via additive `MissingToolchangePurge`. |

Aggregate context cost (sum of per-step costs): **M**. No step is L.

## Prior-packet relationships

This packet does **not** supersede any prior packet. The five prior packets each closed their declared scope:

- `17_wipe-tower-finalization-live-path` — wipe-tower module ported to `PostPass::LayerFinalization`. ✓ Implemented.
- `19_path-optimization-tool-order-and-cooling-policy` — mixed-tool ordering closed; `push-tool-change` shipped. ✓ Implemented.
- `11_orca-gcode-emission-contract` — `;TYPE:` role labeling contract defined. ✓ Implemented.
- `15_live-travel-retraction-policy` — travel retract/no-retract decision in `path-optimization-default`. ✓ Implemented.
- `34_retraction-mode-firmware-vs-gcode` — `retract_mode` toggle. ✓ Implemented.

Two refinement audits revealed additional in-scope work that this packet absorbs:

1. **Positional-insertion primitive missing in finalization** — `finalization-output-builder` had no `insert-entity-at` / `set-entity-order` / `get-ordered-entities`. PathOptimization's `layer-collection-builder` (`wit/deps/ir-types.wit:139-170`) has the permute/read-back contract but no counterpart in finalization. This packet adds all three.
2. **No `bed_shape` config** — grep across the workspace returned zero hits. A prior draft proposed a `host-services::print-bed-shape` WIT extension; rejected on senior review as over-engineered. This packet uses `config-value::float-list` (zero WIT change) to declare `bed_shape` in `crates/slicer-ir/src/resolved_config.rs` (the macro-driven SoT after commit `19e5791`) and in `wipe-tower.toml`'s `[config.schema]`.
3. **Wipe-tower's `K+1` adjacency invariant vulnerable to sibling reordering** — refinement review revealed that `wipe-tower` shares `PostPass::LayerFinalization` with `skirt-brim`, `part-cooling`, `top-surface-ironing`; intra-stage order is topo-sort over the IR DAG plus `[compatibility].requires` edges and is otherwise stable but undefined. If a sibling runs after wipe-tower and pushes entities, the post-apply stable sort could land a non-wipe-tower entity between index `K` (the `ToolChange`'s reference) and `K+1` (wipe-tower's retract). **Resolution**: declare `[compatibility].requires` in `wipe-tower.toml` listing the three siblings — using the existing documented TOML primitive (`docs/03_wit_and_manifest.md:817-822`) instead of inventing a new ordering key.
4. **`FinalizationOutputBuilder` is a struct, not a trait** — an earlier draft repeatedly wrote "SDK trait `FinalizationOutputBuilder`"; the actual type is an action-recorder struct at `crates/slicer-sdk/src/traits.rs:~704`. All references corrected.

Per the cross-packet mutation rule, this packet does NOT modify files inside any prior packet directories.

## Step → Acceptance Criterion coverage

| Step | Lands ACs | Lands NCs |
| --- | --- | --- |
| Step 1 | — (pure dispatch; unblocks all subsequent steps) | — |
| Step 2 | (compile + failing scaffolding for AC1, AC3, AC4, AC6, AC7, AC8, AC9, NC1, NC4, NC5, NC6) | NC1 (scaffolded, failing); NC4, NC5, NC6 (scaffolded, `#[ignore]`) |
| Step 3 | AC7 (`insert-entity-at` semantics), AC8 (`set-entity-order` semantics), AC9 (`get-ordered-entities` read-back) | NC5 (`insert-entity-at` OOB rejected); NC6 (`set-entity-order` malformed rejected) |
| Step 4 | — (clippy/check gate; marker spelling now `;TYPE:Prime tower`) | NC1 (passes after guard added) |
| Step 5 | AC1, AC3, AC4, AC6 (all green after module emission + bed-bounds check) | NC4 (`tower_outside_bed_returns_fatal` green) |
| Step 6 | AC2a, AC2b, AC5 (file-level awk/python) | NC2, NC3 |
| Step 7 | — (docs only; finalizes packet status) | — |

Every AC and NC traces to at least one step.

## WIT delta summary

This is the only packet section that itemizes WIT changes.

| WIT file | Change | Step | Why |
| --- | --- | --- | --- |
| `wit/world-finalization.wit::finalization-output-builder` | **ADD** `insert-entity-at: func(layer-index, position: u32, path, region-key) -> result<_, string>` | Step 3 | Required for wipe-tower to bracket `T<n>` at `after_entity_index + 1`. `push-entity-with-priority` can only append + role-priority sort; it cannot guarantee positional adjacency to a specific `after_entity_index`. |
| `wit/world-finalization.wit::finalization-output-builder` | **ADD** `set-entity-order: func(layer-index, items: list<tuple<u32, bool>>) -> result<_, string>` | Step 3 | Mirrors PathOptimization's permutation contract for finalization. Likely needed by future packets (travel optimization, multi-material sort). Smoke-tested by AC8 + NC6. |
| `wit/world-finalization.wit::finalization-output-builder` | **ADD** `get-ordered-entities: func(layer-index) -> list<print-entity-view>` | Step 3 | Mirrors PathOptimization's read-back contract for finalization. Lets modules validate proposed inserts/permutes before committing. Smoke-tested by AC9. |
| `wit/deps/types.wit::extrusion-role` | **NONE** | — | `wipe-tower` is first-class (line 27). `PrimeTower` / `Skirt` continue to round-trip via `Custom("slicer.builtin/...")` per `wit_host.rs:4747-4768`. |
| `wit/world-finalization.wit::layer-collection-view` | **NONE** | — | `tool-changes` accessor already exists (lines 13-40). |
| `wit/host-api.wit::host-services` | **NONE** | — | Earlier draft's `print-bed-shape` accessor explicitly **rejected** on senior review. Bed shape is config. |
| `wit/deps/config.wit::config-value` | **NONE** | — | `float-list(list<f64>)` already exists; `bed_shape` encodes as `[x0, y0, x1, y1, …]`. |
| `wit/deps/ir-types.wit` (PathOptimization's `layer-collection-builder`) | **NONE** | — | This packet adds the three methods to the **finalization** builder. PathOptimization's builder is the mirror reference, not a target. |
| Guest bindgen output for every core-module `.wasm` | **REBUILT** | Step 3 | Adding methods to a WIT resource invalidates every guest. `./modules/core-modules/build-core-modules.sh --check` must report fresh before Step 5. |

## Config delta summary

| Config surface | Change | Step | Notes |
| --- | --- | --- | --- |
| `modules/core-modules/wipe-tower/wipe-tower.toml` | **ADD** `[config.schema.bed_shape]` entry, type `float-list`, required when `wipe_tower_enabled=true`. Also **ADD** (conditionally, per Step 1) `[config.schema.retract_length]` type `float`, default 2.0 mm | Step 3 | Format: `[x0, y0, x1, y1, …]` mm. Closed polygon convention. |
| `crates/slicer-ir/src/resolved_config.rs` (macro-driven SoT after commit `19e5791`) | **ADD** `bed_shape: List<f64>` field with default `[0.0, 0.0, 250.0, 0.0, 250.0, 250.0, 0.0, 250.0]` to the `declare_resolved_config!` invocation; also **ADD** `retract_length: f64` (default 2.0) if no existing retract-distance key is found in Step 1 | Step 3 | 250 mm × 250 mm rectangle default. Step 1 reverifies the macro accepts list-typed fields; if not, packet absorbs a minimal macro extension. |
| `wit/deps/config.wit` | **NONE** | — | `float-list` already exists; no WIT change. |

## Manifest ordering delta

| Manifest | Change | Step | Mechanism |
| --- | --- | --- | --- |
| `modules/core-modules/wipe-tower/wipe-tower.toml` | **ADD** `[compatibility].requires = ["com.core.skirt-brim", "com.core.part-cooling", "com.core.top-surface-ironing"]` | Step 3d2 | Uses the existing documented intra-stage ordering primitive (`docs/03_wit_and_manifest.md:817-822`); DAG edge creation in `crates/slicer-host/src/dag.rs:93-102` produces `<sibling> → wipe-tower` edges forcing wipe-tower last in `PostPass::LayerFinalization`. No new manifest key is introduced. Tradeoff: this also imposes a presence requirement on the three siblings; acceptable because all are core modules shipped together. |

## Marker spelling correction summary

| File:line | Before | After | Source of truth |
| --- | --- | --- | --- |
| `crates/slicer-host/src/gcode_emit.rs:271` | `ExtrusionRole::WipeTower => ";TYPE:Wipe tower"` | `ExtrusionRole::WipeTower => ";TYPE:Prime tower"` | `OrcaSlicerDocumented/src/libslic3r/ExtrusionEntity.cpp:648` — `erWipeTower → "Prime tower"`. OrcaSlicer has no `erPrimeTower` variant; the wipe tower's prime-load and color-purge passes share the same enum and the same canonical marker. |

User-visible in produced `.gcode` output. Recorded in Step 7 DEVIATION_LOG entry.

## Index-remap invariants summary

The three new builder methods carry correctness contracts that the host's `apply_to` (`crates/slicer-sdk/src/traits.rs:918-956`) must honor. Tested by AC7, AC8, NC5, NC6.

| Operation | Invariant | Failure mode |
| --- | --- | --- |
| `insert-entity-at(layer, position, path, rk)` | New entity occupies `position`; entities at `position..N` shift to `position+1..N+1`; `ToolChange.after_entity_index >= position` increment by 1; `ZHop` indices `>= position` increment by 1. | OOB position → `Err`, layer state unchanged. |
| `set-entity-order(layer, items)` | Permutation: each `(old_index, reverse)` defines the new position; `ToolChange.after_entity_index` remapped via the permutation; `ZHop` indices remapped accordingly. | Length mismatch / duplicate / OOB index → `Err`, layer state unchanged. |
| `get-ordered-entities(layer)` | Returns staged state (pre-existing + module-pushed + module-inserted entities, post-`apply_to` for completed operations and current proposal state for in-flight). | — (read-only). |

## Rejected alternatives summary (for future reviewers)

| Alternative | Why rejected | Audit ref |
| --- | --- | --- |
| `host-services::print-bed-shape` WIT accessor | Bed shape is a printer-profile property; `config-value::float-list` already exists. Senior review caught over-engineering. | Refinement round 2 |
| Migrate wipe-tower to `Layer::PathOptimization` | Three fatal blockers: no entity-add in PathOpt; no tool_changes read; no `z`/layer-height. Plus 7+ tests break. | Senior-review feasibility audit |
| `push-entity-with-priority` with role-based priority for bracketing | Priorities cluster by role, not by position; wipe-tower entities sort into one block, never adjacent to `after_entity_index`. | Senior-review priority-sort audit |
| Keep `;TYPE:Wipe tower` spelling | OrcaSlicer parity mandates `;TYPE:Prime tower` per `ExtrusionEntity.cpp:648`; downstream tooling looks for that. | OrcaSlicer parity audit |
| First-class `extrusion-role::prime-tower` / `extrusion-role::skirt` | Wipe-tower only emits `WipeTower` (first-class). Custom-tag round-trip is out of scope. | WIT extrusion-role audit |
| Add only `insert-entity-at`, skip the other two | User directive: mirror PathOptimization's capability surface to avoid a future WIT pass for permutation/read-back. | User decision after refinement round 2 |
