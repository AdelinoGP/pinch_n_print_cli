# Design: skirt-type-and-draft-shield-keys

## Tier Re-derivation

**Tier B.** Map Authoring rule 1 requires a packet that builds a decision point to be re-tiered B or C. This packet builds five: shield span, per-layer loop count, start-corner rotation, per-object grouping with envelope merging, and extruded-length loop expansion. It is B rather than C because every new decision point lands inside one existing module's existing stage (`PostPass::LayerFinalization`) over data the module already receives; there is no new geometry service, no new stage, no new claim, and no new IR carrier.

## Claims

`skirt-brim`'s manifest declares `holds = []` / `requires = []`; it is stage-scheduled at `PostPass::LayerFinalization`, not claim-resolved. **No behaviour in this packet holds a claim, and none should.**

Map Authoring rule 4's claim-holder trigger test asks whether the Orca enum selects between *separate implementations that must live in separate modules and be resolved through the claim seam*. It does not fire here:

- `skirt_type` selects between two groupings of the *same* ring generator inside one module — the rule's own "module branching internally over a mode it implements itself" case, alongside `seam_position` / `support_style` / `wall_sequence`.
- `draft_shield` and `single_loop_draft_shield` are span/count gates on that same generator.
- `skirt_start_angle` and `min_skirt_length` are scalars, not algorithm selectors.

There is no alternative skirt *algorithm* here for a community module to supply, so introducing a `claim:skirt` seam would add a resolution surface with exactly one possible holder — the opposite of what rule 4 is for.

## Which existing mechanism carries the new data

Every input this packet needs already reaches the module, or reaches it through a mechanism that already exists:

| New data | Carrier | Status |
| --- | --- | --- |
| the five skirt keys | module manifest `[config.schema]` → `bind_module_config_view` (`crates/slicer-scheduler/src/execution_plan.rs`) → `ConfigView` | existing; the keys are module-owned, so they enter the source map through `ResolvedConfig`'s `extensions` merge |
| per-object partitioning | `PrintEntity::region_key.object_id`, already on every entity returned by `LayerCollectionView::ordered_entities` (`crates/slicer-sdk/src/traits.rs`) | existing; **no new IR field** |
| `layer_height`, `line_width` for `mm3_per_mm` | `ResolvedConfig::to_config_map` already exports both; `skirt-brim` already declares `line_width` and gains a `layer_height` declaration | existing |
| `filament_diameter` for the filament cross-section | `ResolvedConfig::filament_diameter` **exists as a field** but is not exported by `ResolvedConfig::to_config_map`; this packet adds the export line, mirroring the existing `filament_density` export in the same function | one added map entry, **not** a new `ResolvedConfig` field, not an IR schema bump, not a WIT change |

No `PostPass` claim, prepass IR field, `SliceRegionView` metadata field, or SDK trait method is added.

## Selected Approach

`SkirtBrim` grows five config-derived fields and its skirt emission is restructured into three composable stages, all inside `modules/core-modules/skirt-brim/src/lib.rs`:

1. **Group.** A new `fn skirt_groups(&self, layers, span) -> Vec<BBox2D>`. Under `skirt_type = "combined"` it returns the single all-entity bbox `compute_bbox` produces today. Under `"perobject"` it buckets entities by `region_key.object_id` into per-object `BBox2D`s, then runs canonical's fixed point: grow each bbox by `grouping_offset = skirt_distance + skirt_loops * line_width`, union any two grown rects that intersect, repeat until no union occurs, and return the un-grown union bbox of each surviving group. Deterministic order: groups sorted by `(x_min, y_min)` so entity push order is stable regardless of `HashMap` iteration.
2. **Expand.** A new `fn ring_count(&self, bbox) -> u32`. Returns `skirt_loops` when `min_skirt_length <= 0.0`. Otherwise accumulates `perimeter_i * self.e_per_mm()` over successive rings and returns the smallest count reaching the target, capped at `MAX_MIN_LENGTH_LOOPS`. `perimeter_i = 2 * ((w + 2*off_i) + (h + 2*off_i))` with `off_i = skirt_distance + i * line_width` — closed-form on the rect, so no polygon offsetting is needed. `e_per_mm` is a pure function of `line_width`, `layer_height`, `filament_diameter`.
3. **Emit.** `generate_skirt_entities` takes the group bbox and the ring count, and additionally takes the layer's position in the span so it can apply `single_loop_draft_shield` (upper layers emit ring index 0 only) and `skirt_start_angle` (first layer, ring index 0 only). Start-corner rotation is applied by `fn rotate_rect_start(points, corner)`, which re-orders the four distinct rect vertices to begin at the selected corner and re-closes the loop — the point count stays 5 and the loop stays closed.

The span itself is computed once in both `process` and `run_finalization`: `let span = if draft_shield_enabled && skirt_loops > 0 { layers.len() } else { (skirt_height as usize).min(layers.len()) };` — the port's direct analogue of `Print::has_infinite_skirt`.

### Rejected alternative

Emitting per-object skirts from the host (a new `PostPass` arm that re-runs `skirt-brim` per object) was rejected: it would put a per-object loop in the host for a decision the module can make from data it already holds, contradicting map Authoring rule 4's "new decision points go where the architecture puts them — not as host-side special cases".

## Code Change Surface (authoritative files-in-scope)

- `modules/core-modules/skirt-brim/skirt-brim.toml` — seven new `[config.schema.*]` tables (`skirt_type`, `min_skirt_length`, `skirt_start_angle`, `draft_shield`, `single_loop_draft_shield`, `filament_diameter`, `layer_height`).
- `modules/core-modules/skirt-brim/src/lib.rs` — new struct fields + `from_config` arms; new `skirt_groups`, `ring_count`, `e_per_mm`, `rotate_rect_start`, `MAX_MIN_LENGTH_LOOPS`; span computation and group/ring threading in both `process` and `run_finalization`.
- `modules/core-modules/skirt-brim/tests/skirt_config_schema_tdd.rs` — **new file** (manifest guard; parses the real `skirt-brim.toml`).
- `modules/core-modules/skirt-brim/tests/skirt_brim_tdd.rs` — AC-4, AC-5, AC-6, AC-N1, AC-N2.
- `modules/core-modules/skirt-brim/tests/finalization_live_tdd.rs` — AC-2, AC-3.
- `modules/core-modules/skirt-brim/Cargo.toml` — add `toml = "0.8"` as a dev-dependency **if absent** (needed by the new schema guard).
- `crates/slicer-ir/src/resolved_config.rs` — one `m.insert("filament_diameter", ConfigValue::Float(...))` line inside `ResolvedConfig::to_config_map`, placed next to the existing `filament_density` export.
- `crates/slicer-gcode/src/serialize.rs` — the synthetic-`filament_diameter` branch in `serialize_config_block`: fire when the raw value is absent **or** is a non-`List` scalar, and build the comma-joined array from that scalar; plus its unit test.
- `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` — AC-7 arm.
- `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` — AC-9 arm.
- `docs/15_config_keys_reference.md` — regenerated only, via `cargo xtask gen-config-docs`.

## Read-only context (allowed reads, no edits)

- `crates/slicer-sdk/src/traits.rs` — `LayerCollectionView::ordered_entities` / `layer_index` / `z`, and `FinalizationOutputBuilder::push_entity_to_layer`. Located windows only; the file is over the 600-line ceiling.
- `crates/slicer-scheduler/src/execution_plan.rs` — `bind_module_config_view` and `source_key_matches_declared`, to confirm a module-declared host key resolves. Located window only.
- `modules/core-modules/classic-perimeters/classic-perimeters.toml` — the existing precedent for a module declaring host-owned `layer_height` / `nozzle_diameter`.

## Out of bounds (must not be loaded or edited)

- `crates/slicer-gcode/src/serialize.rs`'s `ORCA_CONFIG_PADDING` table and every padding twin (map Authoring rule 2).
- Any other packet directory under `docs/spec_packets/`, in particular `257a-brim-type-and-object-gap` and `257b-brim-ears`.
- Any other core module under `modules/core-modules/`.
- `crates/slicer-schema/wit/` — no WIT change is required and none is permitted here.
- Generated `target/` artifacts, `docs/ORCA_CONFIG_REFERENCE.md`'s hand-maintained ✅/❌ column (never sized off; map Notes).

## Expected Dispatches

| Question | Scope | Return format |
| --- | --- | --- |
| Confirm `Print::_make_skirt`'s union-find merge criterion and the `grouping_offset` expression (only if a worker disputes `requirements.md`'s row) | `OrcaSlicerDocumented/src/libslic3r/Print.cpp` | SUMMARY ≤ 200 words |
| Confirm `Skirt::find_start_point`'s target-point formula | `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` | SUMMARY ≤ 200 words |
| `docs/02_ir_schemas.md` §CONFIG_BLOCK — what the viewer requires of `filament_diameter` | that section only | SUMMARY ≤ 200 words |
| Every `cargo test` / `cargo check` / `cargo clippy` / `cargo xtask` run in this packet | — | FACT pass/fail (+ ≤ 20 lines on failure) |

## Architecture Constraints

<!-- snippet: coord-system -->
**Coordinate system:** 1 unit = 100 nm (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Any constant ported from OrcaSlicer must be divided by 100. Use `Point2::from_mm(x, y)` / `mm_to_units()` for conversions. Full porting checklist in `docs/08_coordinate_system.md`.

Applies here twice: canonical's `grouping_offset` and loop `distance` are `scale_()`d, whereas `SkirtBrim` works in **plain mm floats** (`Point3WithWidth.x/y` are mm, as `generate_skirt_entities` already shows). Do not port `scale_` or any scaled literal into this module — every offset in the new code is mm. Likewise `unscale(loop.length())` in `append_skirt_loops_for_hull` has no analogue: the port's perimeter is already mm.

<!-- snippet: wasm-staleness -->
**Guest WASM staleness:** `skirt-brim.toml` and `skirt-brim/src/lib.rs` are guest-fingerprint inputs (`guest_input_paths`, `xtask/src/build_guests.rs`), so the guest artifact goes stale on every step in this packet. `cargo xtask build-guests --check` must return exit `0` (`EXIT_FRESH`) before any host-integration, CONFIG_BLOCK, or dispatch test result is attributed to this packet's code. Exit `1` means rebuild (drop `--check`) and re-run; exit `3` is a `wasm-tools`-missing infrastructure error and is **not** clean.

**Config key naming:** all new keys are snake_case in both the manifest and every `config.get(...)` call (CLAUDE.md).

**Blast radius — `to_config_map`:** adding a key to `ResolvedConfig::to_config_map` widens *two* consumers at once, because `crates/slicer-gcode/src/serialize.rs::resolved_config_to_map` and `crates/slicer-wasm-host/src/dispatch.rs::resolved_config_to_map` both delegate to it. The module-visibility widening is the intent; the CONFIG_BLOCK widening is the hazard AC-8 pins. The step that adds the export line owns both, and must run the `slicer-gcode` serialize tests and `gcode_header_thumbnail_config_blocks_tdd` in the same step — not leave them for a later `cargo check`.

**Blast radius — new struct fields:** `SkirtBrim` gains fields, so every `SkirtBrim { .. }` literal in the three test files must use a `..` rest or carry an `// exhaustive: <reason>` waiver (CLAUDE.md struct-literal churn gate, `docs/21_data_defaults_and_fixtures.md`). The tests construct through `SkirtBrim::from_config`, so this should be vacuous — the step that adds the fields must confirm it with `cargo xtask check-literals`, not assume it.

**Test-binary suitability:** AC-2 and AC-3 assert behaviour of the *live* `run_finalization` path and are homed in `tests/finalization_live_tdd.rs`, which already drives that path through `LayerCollectionFixtureBuilder` + `FinalizationOutputBuilder`. AC-4/5/6 assert ring geometry and are homed in `tests/skirt_brim_tdd.rs`. Neither needs an end-to-end `run_slice` driver; AC-9 does, and is homed in the runtime integration binary that already has that setup.

## Invariants

- **INV-1 (default identity).** With all five keys absent, the emitted entity sequence is byte-identical to pre-packet output. Pinned by AC-N1. This is an *additional* guarantee, never the sole evidence for any key (map Authoring rule 1).
- **INV-2 (loop shape).** Every emitted skirt ring has exactly 5 points and `points.first() == points.last()`, before and after start-corner rotation. Pinned by AC-4.
- **INV-3 (termination).** `ring_count` returns at most `MAX_MIN_LENGTH_LOOPS` regardless of `min_skirt_length`. Pinned by AC-N2. Canonical terminates instead on `offset` returning an empty polygon, which a rect grown outward never does — hence the explicit cap (D-258-3's sibling rationale, recorded in `requirements.md`).
- **INV-4 (grouping determinism).** `skirt_groups` output order is independent of `HashMap` iteration order, so entity push order is reproducible across runs. Pinned inside AC-5's per-object assertion, which asserts the two ring sets by bbox identity in sorted order.
- **INV-5 (tool identity).** Skirt and brim keep `tool_index = 0` and `region_id` stays a pure identity, never a tool selector (D-125-TOOL-IDENTITY-SPLIT). Per-object skirts keep the `"__skirt__"` object_id marker on their `RegionKey`; the grouping partition reads the *source* entities' `object_id` and never writes it back onto skirt entities.
- **INV-6 (padding untouched).** `ORCA_CONFIG_PADDING` gains no entries and loses none. Pinned by AC-9.

## Risks

- **R-1 — CONFIG_BLOCK filament-count regression.** Exporting `filament_diameter` as a scalar without the serializer correction emits `; filament_diameter = 1.75` (one value), breaking OrcaSlicer's array-length filament-count inference for multi-tool G-code. Mitigation: the two edits are one commit; AC-8 asserts the comma-joined per-tool array explicitly and asserts the pre-packet hardcoded `1.75,1.75` is gone.
- **R-2 — silent default-branch pass.** If a key is wired in `src/lib.rs` but missing from the manifest, `bind_module_config_view` filters it out, `config.get` returns `None`, and the non-default AC passes on the default branch while asserting nothing. Mitigation: Step 1 (manifest) precedes all wiring steps, and AC-1/AC-N3 guard the manifest against drift.
- **R-3 — 257a/257b manifest collision.** Three packets now add tables to the same `skirt-brim.toml` and arms to the same `from_config`. Mitigation: land 257a/257b first (`packet.spec.md` §Prerequisites); all edits here are additive tables and additive arms, so the conflict is textual, not semantic.
- **R-4 — guest staleness masking.** Every step here dirties the guest fingerprint; a stale guest fails typed instantiation and looks like an unrelated integration failure. Mitigation: `cargo xtask build-guests --check` exit-0 gate after every manifest/`lib.rs` step (Architecture Constraints).
- **R-5 — perimeter closed form vs polygon length.** `ring_count` uses the rect perimeter closed form rather than summing segment lengths. It is exact for the port's rect loops and would silently diverge if D-258-1 is ever resolved to hull-offset geometry. Mitigation: recorded here; the closed form is a private helper, so a future hull packet replaces one function.

## Open Questions

- **[FWD] Wipe-tower grouping obstacle.** Canonical adds the wipe tower's first-layer corners as a non-emitting grouping item so a per-object group touching the tower merges with it. Whether wipe-tower entities carry a stable `region_key.object_id` at `PostPass::LayerFinalization` was not verified at authoring. Forwarded to whichever packet establishes a wipe-tower object-identity contract; it does not gate any AC here, and `requirements.md` §Returned to Queue names it.
- **[FWD] `filament_diameter` as a per-filament array.** Canonical's key is coFloats; this port flattens it (`extract_float_or_first`). Widening `ResolvedConfig` to a `Vec<f32>` would make D-258-3's per-extruder rotation portable, but it is a `ResolvedConfig` *shape* change and therefore out of this packet by its own constraint. Forwarded to the Tier-D per-filament model work the map already parks.

**No [BLOCK].** This packet requires no new WIT interface, no IR schema version bump, and no new `ResolvedConfig` field.

## Context Cost

Aggregate: **M**. Per-step costs are in `implementation-plan.md`; no step is rated L.
