# Design: prime-tower-geometry-keys

## Tier Re-derivation

**Tier B.** Map Authoring rule 1 requires a packet that builds a decision point to be re-tiered B or C. This packet builds three (scan-line pitch, per-layer tower depth + framework uniformity, first-layer brim) plus the depth model they rest on. It is **B, not C**, because everything lands inside one existing module at its existing stage (`PostPass::LayerFinalization`), over data the module already receives (`LayerCollectionView::tool_changes`, `z`, `layer_index`), through the builder API it already uses (`FinalizationOutputBuilder::insert_entity_at`). No new stage, no new claim, no new IR carrier, no host crate change.

The former packet 254 was implicitly Tier A ("declare + emit"). That tier is void under rule 1.

## Claims

`wipe-tower`'s manifest declares `holds = []` / `requires = []`; it is stage-scheduled at `PostPass::LayerFinalization`, not claim-resolved. **No behaviour in this packet holds a claim.**

Map Authoring rule 4's claim-holder trigger test asks whether the config selects between *separate implementations that must live in separate modules and be resolved through the claim seam*. It does not fire on any of the three keys:

- `prime_tower_infill_gap` is a scalar pitch factor, not an algorithm selector.
- `prime_tower_brim_width` is a scalar width (with an Auto sentinel), not an algorithm selector.
- `prime_tower_enable_framework` is a boolean the module branches on internally — the rule's own "module branching internally over a mode it implements itself" case.

Introducing a `claim:prime-tower` seam would create a resolution surface with exactly one possible holder. Note that canonical's genuinely cross-implementation split — `WipeTower` vs `WipeTower2`, and within them the cone / rib / normal wall types — *would* be a rule-4 candidate; that is packet 255's surface, flagged here and not taken.

## Which existing mechanism carries the new data

| New data | Carrier | Status |
| --- | --- | --- |
| the three keys | module manifest `[config.schema]` → `bind_module_config_view` (`crates/slicer-scheduler/src/execution_plan.rs`) → `ConfigView` | existing; percent-typed defaults thread into `ResolvedConfig.extensions` via `ConfigBoundsIndex::schema_defaults` (packet-185 path), bool/float defaults stay manifest-side behind the module read fallback |
| per-layer tool-change count (for the depth model) | `LayerCollectionView::tool_changes()` → `&[slicer_ir::ToolChange]`, already read by `run_finalization`; the whole print is in scope as `layers: &[LayerCollectionView]`, and the module already indexes `layers[idx - 1]` for Δz | existing; **no new IR field** |
| tower height (for the brim Auto sentinel) | the top `LayerCollectionView`'s `z()` | existing |
| `layer_height` (for `block_depth` and the brim `spacing`) | `ResolvedConfig::to_config_map` already exports it; `wipe-tower` gains the declaration so `bind_module_config_view` admits it (the `classic-perimeters.toml` precedent). The module's current Δz derivation stays as the fallback when the key is absent | existing |
| brim + purge entities | `FinalizationOutputBuilder::insert_entity_at`, already used; `ExtrusionRole::WipeTower` already exists (as does `PrimeTower`, unused here — canonical tags the brim `erWipeTower`) | existing |

No `PostPass` claim, prepass IR field, `SliceRegionView` metadata, SDK trait method, WIT type, or `ResolvedConfig` field is added.

## Selected Approach

`run_finalization` gains a **plan-then-emit** shape, replacing today's emit-in-place loop:

1. **Plan depths.** One pass over `layers` computes `block_depth = prime_volume / (line_width × layer_height × tower_width)` per layer (`layer_height` from the declared key, falling back to today's Δz derivation) and `layer_depth[i] = n_tool_changes(i) × block_depth[i]`. If `prime_tower_enable_framework`, every tower layer's `layer_depth` is overwritten with the **first tower layer's** — canonical `WipeTower::generate_wipe_tower_blocks`' `block.layer_depths[layer_id] = block.layer_depths[0]`. The first tower layer is the first `view` with a non-empty `tool_changes()`, which is also the layer the brim lands on.
2. **Seat blocks.** `generate_purge_paths` gains a `depth_offset: f32` and a `block_depth: f32` parameter. Block `k` (in ascending `after_entity_index` order, independent of the reverse insertion order the existing code uses for index safety) spans `y ∈ [tower_y + k·block_depth, tower_y + (k+1)·block_depth)`. Under framework, the layer's single or few blocks are stretched so the layer's span reaches the first layer's `layer_depth` — the last block on the layer absorbs the padding, which is where canonical's summed-depth recompute puts it.
3. **Pitch.** The scan-line advance becomes `pitch = (infill_gap_percent / 100) × line_width`, replacing `y += self.line_width`. Read as `ConfigValue::Percent` with the schema default `150%` as fallback (`ConfigValue::Percent` is a live variant — `crates/slicer-gcode/src/serialize.rs` matches it).
4. **Brim.** A new `fn brim_loops(&self, footprint: Rect, top_z: f32) -> Vec<ExtrusionPath3D>` runs only for the first tower layer. `spacing = line_width − layer_height × (1 − π/4)`; `width = if brim_width < 0.0 { auto_brim(top_z) } else { brim_width }` with `auto_brim(h) = if h < 100.0 { h / 100.0 * 8.0 } else { 8.0 }`; `loops_num = floor((width + spacing/2) / spacing)`; loop `i` is the footprint rect offset outward by `(i + 1) × spacing`. Emitted at `ExtrusionRole::WipeTower`, `tool_index = 0`, `RegionKey.object_id = "__wipe_tower__"` (the module's existing marker).
5. **Bed bounds.** The pre-flight corner check moves from the `tower_width`-square approximation to the real planned extent: `x ∈ [tower_x − brim_extent, tower_x + tower_width + brim_extent]`, `y ∈ [tower_y − brim_extent, tower_y + max_i(layer_depth[i]) + brim_extent]`, with `brim_extent = loops_num × spacing`. This runs after the depth plan, so it needs the plan pass to exist — hence the same-commit rule in `requirements.md`.

### Rejected alternative

Deriving per-layer depth from a host-side tower planner (a new prepass IR field carrying planned depths) was rejected: the module already has every input it needs at its own stage, and a host planner would move a module decision into the host, contradicting map Authoring rule 4's "new decision points go where the architecture puts them — not as host-side special cases".

## Code Change Surface (authoritative files-in-scope)

- `modules/core-modules/wipe-tower/wipe-tower.toml` — four new `[config.schema.*]` tables (`prime_tower_infill_gap`, `prime_tower_brim_width`, `prime_tower_enable_framework`, `layer_height`).
- `modules/core-modules/wipe-tower/src/lib.rs` — new `WipeTower` fields + `from_config` arms; new `plan_layer_depths`, `brim_loops`, `auto_brim_width`, `scan_pitch` helpers; `generate_purge_paths` gains depth/offset parameters; `run_finalization` gains the plan pass; the bed-bounds block widens.
- `modules/core-modules/wipe-tower/Cargo.toml` — add `toml = "0.8"` dev-dependency (**verified absent**; the crate's only dev-dependency today is `slicer-sdk` with `features = ["test"]`). Precedent: `arachne-perimeters` and `part-cooling` both use `toml = "0.8"`.
- `modules/core-modules/wipe-tower/tests/wipe_tower_config_schema_tdd.rs` — **new file**, standalone binary (the crate's `tests/` has no aggregator `main.rs`), AC-1 / AC-N3.
- `modules/core-modules/wipe-tower/tests/wipe_tower_tdd.rs` — AC-2, AC-3, AC-4, AC-5, AC-6, plus the pitch-pinned baseline updates.
- `modules/core-modules/wipe-tower/tests/bed_bounds_tdd.rs` — AC-7.
- `modules/core-modules/wipe-tower/tests/finalization_live_tdd.rs` — AC-N2.
- `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` — AC-8 arm (**new arm in an existing, already-registered file**; the file carries no wipe-tower case today).
- `crates/slicer-runtime/tests/contract/config_view_binding_tdd.rs` — AC-N1 arm (**new arm in an existing, already-registered file**).
- `docs/15_config_keys_reference.md` — regenerated only, via `cargo xtask gen-config-docs`.

## Read-only context (allowed reads, no edits)

- `crates/slicer-sdk/src/traits.rs` — `LayerCollectionView::{tool_changes, layer_index, z, ordered_entities}` and `FinalizationOutputBuilder::insert_entity_at`. Located windows only; over the 600-line ceiling.
- `crates/slicer-scheduler/src/config_resolution.rs` — `ConfigBoundsIndex::{check, schema_defaults}`. Located window only.
- `modules/core-modules/part-cooling/Cargo.toml` — the `toml = "0.8"` dev-dependency precedent.
- `modules/core-modules/classic-perimeters/classic-perimeters.toml` — the host-key (`layer_height`) declaration precedent.

## Out of bounds (must not be loaded or edited)

- `crates/slicer-gcode/src/serialize.rs`'s `ORCA_CONFIG_PADDING` and every padding twin (map Authoring rule 2).
- `crates/slicer-ir/src/resolved_config.rs` — no host config change is required or permitted here.
- `docs/spec_packets/254b-prime-tower-interface-and-ramming/` and `docs/spec_packets/255-wipe-tower-geometry-keys/` — sibling packets; never edited from here.
- Every other core module.
- `crates/slicer-schema/wit/` — no WIT change.

## Expected Dispatches

| Question | Scope | Return format |
| --- | --- | --- |
| Confirm the brim spacing / `loops_num` formulas and the outward offset direction (only if disputed) | `OrcaSlicerDocumented/src/libslic3r/GCode/WipeTower2.cpp` `finish_layer` | SUMMARY ≤ 200 words |
| Confirm `generate_wipe_tower_blocks` forces `layer_depths[layer_id] = layer_depths[0]` (only if disputed) | `OrcaSlicerDocumented/src/libslic3r/GCode/WipeTower.cpp` | SUMMARY ≤ 200 words |
| Confirm `get_auto_brim_by_height`'s formula and its height input (only if disputed) | `OrcaSlicerDocumented/src/libslic3r/GCode/WipeTower.cpp`, `Print.cpp::wipe_tower_data` | SUMMARY ≤ 200 words |
| Every `cargo test` / `check` / `clippy` / `xtask` run | — | FACT pass/fail (+ ≤ 20 lines on failure) |

## Architecture Constraints

<!-- snippet: coord-system -->
**Coordinate system:** 1 unit = 100 nm (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Any constant ported from OrcaSlicer must be divided by 100. Use `Point2::from_mm(x, y)` / `mm_to_units()` for conversions. Full porting checklist in `docs/08_coordinate_system.md`.

Applies directly: canonical's brim loop is `offset(poly, scale_(spacing))` and its Auto-brim height is an unscaled mm value. `WipeTower` works in **plain mm floats** throughout (`Point3WithWidth.x/y`, `tower_x`, `line_width`). Port the formulas, never `scale_`/`unscale`, and never a scaled literal.

<!-- snippet: wasm-staleness -->
**Guest WASM staleness:** `wipe-tower.toml` and `wipe-tower/src/lib.rs` are guest-fingerprint inputs — `guest_input_paths` (`xtask/src/build_guests.rs`) covers the guest `Cargo.toml`, every file under the guest `src/`, and for `GuestTree::Core` the parent module's `src/`, its `Cargo.toml`, and every depth-1 `*.toml` under the module dir (which is the module manifest). Every step here dirties it. `cargo xtask build-guests --check` must return exit `0` (`EXIT_FRESH`) before any host-integration or dispatch test result is attributed to this packet. Exit `1` = rebuild and re-run; exit `3` = `wasm-tools` missing, an infrastructure error, **not** clean.

**Config key naming:** snake_case in the manifest and in every `config.get(...)` call (CLAUDE.md).

**Blast radius — `WipeTower` struct fields and the `generate_purge_paths` signature.** The struct gains fields and the helper gains two parameters. `generate_purge_paths` is called from `run_finalization` and from the `#[cfg(test)]` module inside `src/lib.rs` as well as from `tests/wipe_tower_tdd.rs`; the step that changes the signature owns **every** call site in the same step, not a follow-up `cargo check`. `WipeTower` is constructed through `WipeTower::from_config` in the tests, so no `WipeTower { .. }` literal should need a `..` rest — the step confirms that with `cargo xtask check-literals` rather than assuming it (CLAUDE.md struct-literal churn gate, `docs/21_data_defaults_and_fixtures.md`).

**Blast radius — output change at defaults.** Two independent default-path changes land here: the pitch moves `0.4 → 0.6` mm (schema default `150%`) and multi-toolchange layers stop overlapping. Every pitch- or position-pinned assertion in `modules/core-modules/wipe-tower/tests/` and in `src/lib.rs`'s `#[cfg(test)]` module is fallout the owning step must update **to the formula value**, never by loosening the assertion. `crates/slicer-runtime/tests/contract/integrated_parity_wipe_tower_tdd.rs` and `crates/slicer-runtime/tests/executor/finalization_live_tdd.rs` both set `prime_volume` and may be affected; the closure step checks them.

**Test-binary suitability:** AC-2 through AC-6 assert entity geometry produced by `run_finalization` and are homed in `wipe_tower_tdd.rs`, which drives the module directly. AC-7 is homed in `bed_bounds_tdd.rs`, which already builds `printable_area` fixtures. AC-N1 and AC-8 are host-side and are homed in already-registered aggregator files. No AC needs an end-to-end `run_slice` driver.

## Invariants

- **INV-1 (block disjointness).** On any layer, no two purge blocks share a scan-line Y. Pinned by AC-3. This is new — pre-packet they coincided exactly.
- **INV-2 (volume conservation).** Each purge block still delivers `prime_volume` worth of extrusion: raising the pitch reduces the line count and the block depth stays `prime_volume / (line_width × layer_height × tower_width)`. Pinned inside AC-2's half-the-lines assertion.
- **INV-3 (framework uniformity).** With `prime_tower_enable_framework = true`, `max_i(layer_depth[i]) == min_over_tower_layers(layer_depth[i]) == layer_depth[first_tower_layer]`. Pinned by AC-4.
- **INV-4 (brim locality).** Brim loops appear on exactly one layer — the first with a non-empty `tool_changes()` — and never elsewhere. Pinned by AC-5.
- **INV-5 (tool identity).** Purge entities keep `tool_index = tc.to_tool` (the incoming filament flushes the old colour); brim entities use `tool_index = 0`. `region_id` stays a pure identity and is never read as the tool (D-125-TOOL-IDENTITY-SPLIT).
- **INV-6 (insertion-order safety).** The existing reverse-order (`Reverse(tc.after_entity_index)`) insertion loop must be preserved — it exists so inserts at higher indices do not shift lower ones. The depth offset `k` is the block's **ascending** rank, which must be computed before the reverse sort, not from the reversed iteration index.
- **INV-7 (padding untouched).** `ORCA_CONFIG_PADDING` gains no entries and loses none.

## Risks

- **R-1 — reverse-iteration off-by-one.** The insertion loop iterates tool changes in *descending* `after_entity_index`, so naively using the loop index as `k` inverts the depth seating. Mitigation: INV-6; AC-3 asserts the band each block occupies, which a reversed `k` fails.
- **R-2 — baseline churn mistaken for regression.** Two default-path changes land at once; a failing pitch assertion could be either. Mitigation: land the pitch (Step 2) and the depth model (Step 3) as separate steps so the first failing baseline set identifies which change caused it.
- **R-3 — `layer_height` absent at runtime.** If the declared key does not reach the module (profile omits it and the schema default does not thread, as non-percent defaults do not), `block_depth` would use a wrong height. Mitigation: keep the existing Δz derivation as the fallback and prefer it over a hardcoded constant; `DEFAULT_LAYER_HEIGHT` stays the last resort for layer 0 only.
- **R-4 — bed-bounds widening rejects previously-accepted configs.** The old check used a `tower_width`-square; a deep multi-toolchange tower plus brim can now exceed the bed where it previously passed. That is the check becoming correct, not a regression — but it is a behaviour change on the error path. Mitigation: AC-7 asserts both directions (a tower that fits still passes); the fixtures in `bed_bounds_tdd.rs` are updated with measured extents.
- **R-5 — guest staleness masking.** Every step dirties the guest fingerprint. Mitigation: the `build-guests --check` exit-0 gate after each manifest/`lib.rs` step.
- **R-6 — sibling manifest collision.** `254b` and packet 255 add tables to the same manifest and arms to the same `from_config`. Mitigation: land `254a` first; all edits are additive.

## Open Questions

- **[FWD] `prime_tower_skip_points` and travel avoidance.** Returned to the queue (`requirements.md`). Canonical also ANDs it into `m_use_gap_wall`, which gates the interface tower; `254b` records that coupling rather than reproducing it. Forwarded to whichever packet builds travel-avoid-perimeter.
- **[FWD] Canonical's tower planner.** `WipeTower::plan_tower` / `plan_tower_new` re-fit `m_extra_spacing` and plan block depths globally; this packet's depth model is per-layer and local (D-254a-2). A future packet that ports the planner should reconcile the two rather than layer on top.
- **[FWD] Rule-4 candidate not taken.** `WipeTower` vs `WipeTower2` and the cone / rib / normal wall types are genuine cross-implementation alternatives and a legitimate `claim:*` holder shape. That is packet `255-wipe-tower-geometry-keys`' surface; flagged here so 255's re-authoring does not miss it.

**No [BLOCK].** This packet requires no new WIT interface, no IR schema bump, and no new `ResolvedConfig` field.

## Context Cost

Aggregate: **M**. Per-step costs are in `implementation-plan.md`; no step is rated L.
