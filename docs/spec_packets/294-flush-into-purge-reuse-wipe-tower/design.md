# Design: 294-flush-into-purge-reuse-wipe-tower

## Controlling Code Paths

- Primary code path: `WipeTower::purge_volume_for` (`modules/core-modules/wipe-tower/src/lib.rs`) — the pair-purge decision point ticket 30 built (matrix entry × multiplier, else `prime_volume`, minus grab volume, floored at zero); the flush-into subtraction sits behind that clamp. `purge_depth_for` (same file) converts to depth; `generate_purge_paths` (same file) lays scan lines; `run_finalization` (same file) inserts after `tc.after_entity_index + 1` via `FinalizationOutputBuilder::insert_entity_at` (`crates/slicer-sdk/src/traits.rs`); `max_purge_depth` (same file) validates the bed footprint from the same helper.
- Neighboring tests/fixtures: `modules/core-modules/wipe-tower/tests/wipe_tower_tdd.rs` owns the `from_config` default/custom pins (`from_config_defaults`, `from_config_custom`) and the `print_entity` / `LayerCollectionFixtureBuilder` fixture shape (`slicer_sdk::test_prelude`); `modules/core-modules/wipe-tower/tests/finalization_live_tdd.rs` owns the `run_finalization` + `MergeOp::InsertEntityAt` counting pattern (`wipe_tower_inserts_for_layer`, `printable_area_250` bed fixture) the new guard clones; `wipe-tower.toml` `[config.schema]` bool-row shape (`enable_prime_tower` is the model).
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

- The subtraction lives in the existing owner (`wipe-tower`) at the existing finalization seam — not as a host-side special case, not as a path-optimization rule, and not in `machine-gcode-emit` (its generic `[key]` sweep publishes placeholders; purge geometry is wipe-tower's job — ticket-27 hazard checked, owner corrected from the tier table's `tool-ordering`).
- Rule-4 trigger test does not fire: the three keys parameterise one module over roles it already sees (`PrintEntity.role`, points, `tool_index`); they do not select across alternative algorithm implementations, so no `claim:*` holders and no `*_fill_holder` selection.
- Canonical per-object shape is deliberately NOT held: one global value governs every layer (DEV-186(a)). The `resolve_per_object_configs` mechanism (`crates/slicer-scheduler/src/config_resolution.rs`) exists but is unthreaded on this path — noted as the future, not claimed as a dep.
- Determinism: the subtraction is a pure function of (layer entities, toolchange, three flags, purge volume) — no ordering dependence, no cross-layer state, no cross-entity state beyond the per-toolchange sum.
- Lengths are measured in mm straight from entity points (`Point3WithWidth.x/y`), summing planar segment lengths — no mm↔unit conversion, no IR unit-system touch.
- Schema/version constants and event-specific locking: none bumped (no `PROGRESS_EVENT_SCHEMA_VERSION`, wire-version, or IR-version touch; additive bool fields with subject-gated defaults are inert on prints without their subjects, pinned by AC-2).
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

## Code Change Surface

- Selected approach: three manifest bool rows + three struct fields read in `from_config` (ticket-30 precedent — effective defaults live in code because only percent schema defaults thread into `ResolvedConfig`, and wipe-tower reads its own manifest directly); one volume helper beside `purge_volume_for` sharing the formula; one subtraction in the depth path threaded through `purge_depth_for`'s callers; new TDD guard. No host twin, no `ResolvedConfig`, no padding edit.
- Exact functions, traits, manifests, tests, and fixtures:
  - `wipe-tower.toml` `[config.schema]` (manifest, `modules/core-modules/wipe-tower/wipe-tower.toml`): three rows — `flush_into_infill` (bool, `false`), `flush_into_objects` (bool, `false`), `flush_into_support` (bool, `true`) — each with `display` + `group = "Wipe Tower"` + a one-line `description` naming the canonical key. Bool rows carry no `min`/`max`.
  - `WipeTower` struct + `from_config` (`modules/core-modules/wipe-tower/src/lib.rs`, `WipeTower` struct + `from_config`): three fields `flush_into_infill: bool`, `flush_into_objects: bool`, `flush_into_support: bool`; three reads `match config.get("flush_into_*") { Some(ConfigValue::Bool(b)) => *b, _ => <canonical default> }` (support default `true`, others `false`); three accessors. Effective defaults live here (ticket-30 comment precedent).
  - Wiping-volume helper (new, same file, beside `purge_volume_for`): `fn wiping_volume_for(&self, entities: &[PrintEntity], after_entity_index: u32, to_tool: u32, layer_height: f32) -> f32` — sums `entity_volume` over entities with positional index `> after_entity_index` and `tool_index == to_tool` passing `fn counts_for_role(&self, role: &ExtrusionRole) -> bool` (objects-flag opens all roles except `SupportMaterial`/`SupportInterface`/`SupportBaseInterface`/`WipeTower`/`PrimeTower`; else infill-flag opens `SparseInfill` only; support-flag opens the three support roles; bridges never). `fn entity_volume(path: &ExtrusionPath3D, layer_height: f32) -> f32` = `planar_length × width × layer_height` (planar length sums `hypot(dx, dy)` over consecutive points; width is the entity's own point width — the layer_height × width cross-section both prior artifact-uses share). Positional index (not `entity_id`, not `topo_order`) matches the `after_entity_index` contract (`LayerCollectionIR.tool_changes` + `insert_entity_at` remap semantics).
  - Subtraction (same file): `purge_depth_for` gains the layer-view + flags context (or a `purged_depth_for(layer_view, layer_height, tc)` wrapper calling `purge_volume_for` then subtracting `wiping_volume_for`, floored `.max(0.0)` before dividing by `cross_section`); `run_finalization` + `max_purge_depth` thread the view through. `generate_purge_paths` is UNCHANGED — it keeps calling the un-subtracted volume (its prime-entity length is per-path, not per-layer; the depth gate in its `while y < y_max` loop already follows the subtracted depth via the caller).
  - Bounds: none — bools admit no representable violation (the 291-packet precedent: no runtime bound exists to build). Non-bool spellings fall to the canonical default via the `_ =>` arm (declared type `bool` rejects at schema validation on the config path that enforces it).
  - Tests: new `modules/core-modules/wipe-tower/tests/flush_into_purge_reuse_tdd.rs` (schema guard AC-1, subject-gating AC-2, three flag pins AC-3–AC-5, bridge exclusion AC-N2; AC-N1 is a static grep, no test).
- Rejected alternatives and reasons:
  - Building the subtraction in `path-optimization-default` (the tier table's owner): rejected — ordering ignores config (`run_path_optimization` takes `_config`), owns sequence only, and flush-into changes no order (DEV-186(d)); the purge decision point is wipe-tower's. Owner corrected per the ticket-27/39/40 precedent.
  - Threading the flags through `ResolvedConfig` + `to_config_map` + host-keys rows: rejected — wipe-tower reads its own manifest directly (ticket-30 precedent); a host twin would add a second declaration with no consumer (rule 1 forbids declaration-only keys).
  - Reassigning entity `tool_index` to the incoming tool (canonical's marking): rejected — this packet emits no entity and moves none; emitted G-code is identical either way (DEV-186(d) non-borrow).
  - Per-object config threading via `resolve_per_object_configs`: rejected for this packet — the finalization seam takes one global `ConfigView`; threading object configs there is new seam work larger than the subtraction (DEV-186(a) future).
  - Consulting `support_filament` assignments / solubility at the seam (canonical's vetoes): rejected — invisible at the finalization seam (roles + tool indices only); roles are the subject signal (DEV-186(b) non-borrow).

## Files in Scope (read + edit)

Target at most 3 primary files; justify extras and consider splitting.

- `modules/core-modules/wipe-tower/wipe-tower.toml` - role: three `[config.schema]` bool rows; expected change: three blocks (extra justified: schema source of truth, one block per key).
- `modules/core-modules/wipe-tower/src/lib.rs` - role: three fields + reads + accessors + volume helper + role gate + subtraction threading; expected change: ~60 lines staged config + pure helpers + call-site threading (ranges: struct + `from_config` + `purge_volume_for` + `purge_depth_for` + `max_purge_depth` + `run_finalization` only — never read the file whole).
- `modules/core-modules/wipe-tower/tests/flush_into_purge_reuse_tdd.rs` (new) - role: AC-1–AC-5 + AC-N2 pins; expected change: net-new guard binary, no registration needed (auto-discovered `tests/` dir).
- `docs/DEVIATION_LOG.md` - role: DEV-186 row; expected change: one row with (a)+(b)+(c)+(d) clauses (extra justified: preflight-visible obligation).

## Read-Only Context

Include ranges for files over 300 lines.

- `modules/core-modules/wipe-tower/src/lib.rs` - lines covering the `WipeTower` struct + `from_config` + `purge_volume_for` + `purge_depth_for` + `generate_purge_paths` signature + `run_finalization` insert loop + `max_purge_depth` only - purpose: subtraction anchors + call-site threading (delegate a LOCATIONS fix before reading).
- `crates/slicer-sdk/src/traits.rs` - lines covering `LayerCollectionView::ordered_entities` + `tool_changes` + `FinalizationOutputBuilder::insert_entity_at` only - purpose: layer-view accessor + insert contract spelling.
- `crates/slicer-ir/src/slice_ir.rs` - lines `2373-2419` only (`ExtrusionRole` variants) + `PrintEntity.tool_index` + `ToolChange.after_entity_index` - purpose: role-gate + position-contract spelling.
- `modules/core-modules/wipe-tower/tests/finalization_live_tdd.rs` - `printable_area_250` + `wipe_tower_inserts_for_layer` helpers only - purpose: fixture/counting pattern models (read, never edit).
- `modules/core-modules/wipe-tower/wipe-tower.toml` - `[config.schema.enable_prime_tower]` block only - purpose: bool-row syntax model.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load
- `modules/core-modules/path-optimization-default/` - out of bounds (ordering untouched — no entity moves, no ToolChange rewrite; the tier-table owner is corrected, not built in)
- `modules/core-modules/machine-gcode-emit/` - out of bounds (wrong seam for purge geometry; generic sweep untouched)
- `crates/slicer-scheduler/src/config_resolution.rs` - read-only reference at most (`resolve_per_object_configs` exists-but-unthreaded — the DEV-186(a) future; no change)
- `crates/slicer-gcode/src/serialize.rs` (`ORCA_CONFIG_PADDING`) - read-only reference at most (zero twins for all three spellings; table untouched — rule 2; AC-N1 pins the absence)
- `crates/slicer-gcode/src/estimator.rs` - read-only context at most (it consumes whatever depth the tower leaves; no change)
- `docs/spec_packets/122-*/` (prime tower body, if landed) - reference only, never edit (no sequencing edge — this subtraction composes with any body)
- Unrelated crates - delegate symbol lookups; do not browse

## Expected Sub-Agent Dispatches

- Question: `WipeTower` struct fields + `from_config` bool-row syntax + `purge_volume_for`/`purge_depth_for`/`max_purge_depth`/`run_finalization` exact signatures and call edges; scope: `modules/core-modules/wipe-tower/src/lib.rs`; return: `SNIPPETS` (≤3 snippets, ≤30 lines each); purpose: Step 1–2 anchors.
- Question: struct-literal blast radius — every `WipeTower { .. }` construction site + every test asserting tower defaults; scope: `modules/ crates/`; return: `LOCATIONS` (≤20 entries); purpose: Step 1 edit-list completeness.
- Question: `LayerCollectionView::ordered_entities` element type + `PrintEntity.tool_index`/`role`/`path` field spelling + `ToolChange.after_entity_index` positional contract; scope: `crates/slicer-sdk/src/traits.rs`, `crates/slicer-ir/src/slice_ir.rs`; return: `FACT` (≤5 lines); purpose: Step 2 helper signatures.
- Question: `toml` dev-dep presence in `wipe-tower/Cargo.toml` (guard tests need it — packet-260/261 precedent: add-if-absent); scope: `modules/core-modules/wipe-tower/Cargo.toml`; return: `FACT` (≤5 lines); purpose: Step 1 guard setup.

## Data and Contract Notes

- IR/manifest contracts: three manifest `[config.schema]` bool tables for wipe-tower (canonical defaults); no IR field, no WIT accessor, no `ResolvedConfig`, no host-keys row. `wipe-tower.toml` is the schema source of truth for all three spellings.
- WIT boundary: untouched (no guest-visible key beyond the manifest the guest already reads; guest rebuild rides the standard staleness check below).
- Determinism/scheduler constraints: the subtraction is per-toolchange pure; no claim, priority, or ordering interaction; `default_priority` untouched; `insert_entity_at` positions unchanged (fewer scan lines per box, same anchors).

## Locked Assumptions and Invariants

- At defaults the tower is subject-identical: walls + sparse carry no `true` flag, so only layers where support entities co-occur with a toolchange shrink — intended, pinned by AC-2's both-halves assertion (inert-baseline + measured shrink).
- Canonical per-object shape is deliberately NOT held: one global value governs every layer (DEV-186(a)); the object-config axis is the named future, and ticket 125 (per-tool/extruder vectors) is explicitly not that axis.
- Bridge roles never count at any flag combination (DEV-186(c)); `WipeTower`/`PrimeTower` self-roles never count (no self-absorption); over-subscription floors at zero depth (no negative tower).
- The volume formula (`length × width × layer_height`) is shared with ticket-30's artifact and the estimator by construction — the helper lives beside `purge_volume_for`, not as a second formula.
- `generate_purge_paths` is depth-driven, not volume-driven: it keeps its signature and its un-subtracted prime-length computation; the subtracted depth reaches it through the caller's loop bound.

## Risks and Tradeoffs

- Support-`true` default moves existing output: any multi-tool print whose toolchange layers carry support entities gets a shallower tower at defaults — intended (canonical default is `true`), but it is a default-path geometry change, unlike packets 289/291's identity defaults. AC-2 pins both halves so the movement is measured, not silent.
- Global-not-per-object: a print wanting flush-into on one object but not another cannot express it — accepted (DEV-186(a)); the alternative (threading object configs to finalization) is larger than the packet.
- Role-only subject signal: a support entity the user assigned to a non-zero filament still counts when the flag is set (canonical would veto via `support_filament`); accepted (DEV-186(b)) — the seam cannot see assignments.
- Bed-bounds coupling: shrinking depth can only shrink the validated footprint (same helper), so validation never newly rejects — but a print that previously failed bounds may now pass; that is the correct consequence of emitting less tower, not a validation hole.
- Formula coupling: sharing the estimator's formula means an estimator-formula fix moves tower purge too — intended (one formula, one fix), but flag it at implementation so the coupling is explicit.

## Context Cost Estimate

- Aggregate: `M` (never L)
- Largest step: `M` (Step 2 subtraction + gate + volume helper)
- Highest-risk dispatch and required return format: struct + `from_config` + depth-path signatures (`SNIPPETS`, ≤3×30) — mis-threading the subtraction (inside `purge_volume_for` itself, or after serialization, or per-layer instead of per-toolchange) silently subtracts the wrong volume or breaks the bed-bounds coupling.

## Open Questions

- `[FWD]` Per-object flag threading if the queue ever carries it: `resolve_per_object_configs` onto the finalization path, keyed by `RegionKey.object_id`. Not in P65 — do not "complete" this packet by building the seam.
- `[FWD]` Filament-assignment vetoes if the finalization seam ever sees assignments: `support_filament`/`support_interface_filament` conditions ported from `is_support_overriddable`. Not in P65 — roles are the subject signal here.
- None blocking: no `[BLOCK]` — scope, seams, and verification are decidable from the tree as cited.
