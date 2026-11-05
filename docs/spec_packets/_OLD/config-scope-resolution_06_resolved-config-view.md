---
status: implemented
packet: config-scope-resolution_06_resolved-config-view
task_ids:
  - TASK-567
---

# config-scope-resolution_06_resolved-config-view

## Goal

Make every guest `ConfigView` (load-time binding and per-region layer dispatch) a registry-complete projection of resolved config, make `CONFIG_BLOCK` emission registry-driven over the full registry, remove all 87 load-bearing guest config-literal fallbacks, and enforce warn-then-drop handling for genuinely undeclared keys only after every host-consumed key is registered and the no-drop oracle passes.

## Problem Statement

`ConfigView` has two sources today, and neither is complete.

- The load-time binding comes from `bind_module_config_view(module, source)` (`crates/slicer-scheduler/src/execution_plan.rs`) over `expanded_global_source` (`crates/slicer-runtime/src/run.rs`). That map mixes the raw authored source, including unrecognized keys, with a `to_config_map()` overlay.
- Per-region views come from `ConfigView::from_declared` over the region `ResolvedConfig`'s `to_config_map()` (`crates/slicer-wasm-host/src/dispatch.rs`).
- Neither source seeds registry defaults. `resolve_scope_stack` starts from `ResolvedConfig::default()` with empty `extensions`, so an unauthored module-declared key is absent from every view. That is why 87 literal fallbacks across 13 guests carry behavior.

The remaining problems:

1. `ResolvedConfig::to_config_map` hand-lists its keys (RC-4).
2. `apply_delta` (`crates/slicer-config/src/resolution.rs`) writes `extensions` with no registry bounds check.
3. `CONFIG_BLOCK` overlays the raw source (`run_postpass_with_thumbnail`, `crates/slicer-runtime/src/pipeline.rs`) and emits strings unescaped.
4. Several keys are read by the host but declared nowhere: `gcode_flavor`, `printer_model`, `filament_colour`, `extruder_colour`, `filament_cost`, `printable_area`, `support_type`, `support_family`, `thumbnails`, `machine_max_acceleration_retracting`, `extruder`. They survive only because ingestion is warn-and-keep and the raw overlay re-injects them.
5. Unknown keys are still retained after warning, although the approved staging permits dropping them once a no-drop oracle proves no declared value is lost.

## Architecture Constraints

- `slicer-config` remains dependent only on `slicer-ir`. Scheduler, runtime, and wasm-host orchestrate; they do not become config-semantic authorities.
- `ConfigView` means effective resolved values everywhere. No binding API accepts a raw or expanded source map.
- A declared key with a registry default or an authored value is always present with its effective value, and an undeclared key is always absent. `None` from a getter means "not declared, or declared without default and never authored".
- Registry defaults enter below every authored scope and before Phase-B expansion. An explicitly authored value equal to the default still overrides a lower non-default scope, because presence lives in `ScopeDelta`, not in value comparison.
- **Seed set.** A registry key is seeded into `extensions` only if all four hold:
  - its key is exact, not a `prefix:*` wildcard;
  - it has a registry default;
  - it is not `selector` (today only `wall_generator`, whose value travels as a selector value, not a delta);
  - it is not a `declare_resolved_config!` field (`cli`, `cli_opt`, or `plain` row).

  Do not test "no typed field" with `apply_cli_key`, because `plain` rows also return `Ok(false)`. Test membership in `ResolvedConfig::typed_field_keys()` instead. Typed fields already carry their defaults through `ResolvedConfig::default()`. Seeding them into `extensions` would shadow the typed value, because `to_config_map` merges `extensions` last.
- **Host-consumed registration.** Before the drop flip, every key the host reads from a `ScopeDelta`, from `extensions`, from the raw source, or from the CONFIG_BLOCK map is registered as a `HOST_RUNTIME_KEYS` row, as is every key the docs/02 viewer-key contract lists.
  - A key whose absence the serializer handles by synthesis (`filament_colour`, `extruder_colour`, `printer_model`, and `gcode_flavor` via the flavor fallback) is registered with **no** default. It is therefore not seeded, stays absent unless authored, and synthesis still fires.
  - `extruder` is also registered with no default, because a seeded `extruder` would change per-region tool assignment.
  - This requires `HostRuntimeKey.default: Option<&'static str>`. `runtime_declaration` then passes it through as-is, and `build_host_key_entries` renders `None` as JSON `null`.
- **CONFIG_BLOCK population.** A key of `to_config_map()` is emitted if either:
  - it is a registry entry without `omit_from_config_block`; or
  - it is a `typed_field_keys()` key with no registry entry (today `infill_type`).

  Retained unknown `extensions` keys are never emitted, even before the flip.
- **`config_block` metadata:**
  - It is polarity-safe. The Rust field is `omit_from_config_block: bool` (default `false`, meaning emitted) on `HostKeyMeta`, `ConfigFieldEntry`, and `RegistryEntry`, because all three default their `bool`s to `false` (`HostKeyMeta::NONE`, `#[derive(Default)]`). The manifest spelling stays `config_block = false`.
  - Reconciliation follows ADR-0067's deny-union shape for `denied_scopes`: a key is omitted if any host row or declaring module sets `config_block = false`. ADR-0067 does not constrain this field, so this is not an amendment.
- Canonical parity outranks byte stability: string values are escaped per `escape_string_cstyle`.
- No WIT or public IR schema version changes. The `CONFIG_BLOCK` textual change is intentional.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it. Guests depend on `slicer-ir` and `slicer-sdk`, so every guest goes stale in Steps 2a and 2b (`crates/slicer-ir/src/{resolved_config.rs,config_schema.rs}`) and 4b (`crates/slicer-ir/src/slice_ir.rs`, `crates/slicer-sdk/src/error.rs`), as well as in every Step 5 guest batch.

## Data and Contract Notes

- **IR/manifest contracts:**
  - `extensions` keeps its storage and hash semantics, and now carries seeded defaults for the seed set.
  - Memory and interning impact is unmeasured; record the measured distinct-config count in the acceptance ceremony.
  - Manifest `config_block` is optional and defaults to true.
- **WIT boundary:** unchanged. The required accessors are guest-side Rust only.
- **Determinism/scheduler constraints:** seeding iterates `ConfigSchemaRegistry` (`BTreeMap`) order, `config_block_map` returns a `BTreeMap`, and hash-map iteration never defines output order.

## Locked Assumptions and Invariants

- FORWARD-DEP on packet 05 (draft, in progress). The names and shapes in `packet.spec.md` §Prerequisites match both packet 05's `design.md` and the working tree as of this preflight. Step 1 re-verifies them against the landed tree.
- `nozzle_diameter = 0.4` is the registry default in seven-plus manifests and is seeded at resolution, but the fix cannot ride seeding alone: `seed_expansion_context` (`crates/slicer-runtime/src/run.rs`) builds its `nozzle_diameter` lookup from the authored source, and that runs before the `resolve_scope_stack` seeding in `crates/slicer-config/src/resolution.rs` (run-time call order in `run.rs`). So the fallback belongs in the expansion-context builder itself, where an authored value still wins and expansion semantics are otherwise untouched. The stock red that exposes this is the Step-6 pre-existing blocker recorded in Step 6a.
- The seam-planner aligned-mode gap is fixed in the coordinates the guest emits, not in host escalation. `seam-planner-default` emits mesh-corner coordinates on the aligned path; the defect is the emitted value, not the escalation contract, so changing escalation would move every caller and mask the guest defect. The fix edits the guest's planner coordinates and leaves host escalation untouched; the exact files are `modules/core-modules/seam-planner-default/src/{lib.rs,contours.rs,align.rs,comparator.rs,visibility.rs}`, diagnosed in Step 6a before any edit.
- Packet 05's layer-planning seam removes guest `object_height:<id>` / `layer_height:<id>` reads. If `overlay_object_layer_planning` or a guest wildcard-instance read survives packet 05, binding over the resolved config would drop those instance keys, and Step 1 must stop.
- The fallback census is exactly 87 across the 13 named guests under AC-3's chain-aware classification. It was re-verified during refinement against post-packet-05 HEAD `4558519297e9ce4a269d321252b5c721383ea0fc`; the pre-seam `a85421af` baseline was stale because packet 05's layer-planning seam removed layer-planner-default's guest `object_height:<id>` / `layer_height:<id>` reads, leaving that guest with 0 classified sites. A strict "getter immediately followed by `.unwrap_or`" count is lower; do not use it.
- Eleven guest:key reads were verified undeclared at preflight. Today they always return `None`, so the guest's literal or helper default runs:
  - overhang-classifier-default: `outer_wall_line_width`, `line_width`;
  - tree-support and traditional-support: `nozzle_diameter`, `layer_height`, `support_line_width` (via `get_abs_value`);
  - infill-linker: `infill_density`;
  - rectilinear-infill: `infill_shift_step`, which has no registry entry anywhere, so declaring it creates the entry with the guest's current effective fallback as its default;
  - wave-overhangs: `thick_bridges` (via `cfg_bool`, typed as the existing declarers declare it).

  Guest tests that pass an empty or partial `ConfigView` into a classified code path fail once fallbacks become `require_*` reads. They are migrated to `ConfigView::from_map` views holding the keys their path reads, at the guest's manifest-default values. No new test-support surface is added.

  Declaring them moves each read to its effective or registry-default value. The acceptance ceremony records any value delta against the old literal or helper default.
- Keys typed `float_or_percent` but read with `get_float` must be read with `require_abs_value`, using the base the registry `base_key` names, unless Phase-B expansion guarantees a `Float`. This covers arachne-perimeters `outer_wall_line_width`, `inner_wall_line_width`, `overhang_reverse_threshold`, and overhang-classifier-default `overhang_1_4_speed` … `overhang_4_4_speed`.
- Exactly four keys carry `config_block = false`: the three `mmu_segmented_region_*` keys and `thumbnail_path`. The plan says "three", meaning only the `to_config_map` omissions. `thumbnail_path` preserves today's removal of a fork-only runtime key, which would otherwise leak a local path.
- `wall_generator` is a registry runtime `selector` row. `infill_type` is the only typed field without a registry entry after this packet, and it keeps emitting through `typed_field_keys()`.

## Risks and Tradeoffs

- Seeding changes `resolve_scope_stack` output and may break whole-value equality assertions in packet 04/05 tests. Step 1 inventories them, and they are updated to independently derived expectations.
- A full-registry `CONFIG_BLOCK` exceeds 96 keys, so the existing cap suppresses nearly all `ORCA_CONFIG_PADDING`, including padding-only keys like `wall_loops`. The canary's `95` and `wall_loops` expectations are re-derived from the registry, and the ≥80-key Orca loader gate still holds.
- Registering host-consumed keys grows the runtime channel and the `module config-schema` `host` array. Several tests change with it:
  - `registry_census_tdd`'s runtime-row pins;
  - `registry_assembly_tdd`'s `runtime_key` helper;
  - the retained-`support_family` assertion in `typed_selector_claim_selection_tdd`.

  All three are updated in the same substep. The wire version stays the same, because rows are data and no field is added.
- Public pipeline entry points keep the legacy CONFIG_BLOCK composition, so two compositions coexist. Tests that call them exercise the serializer (escaping, flavor, synthesis) but not registry projection. Registry projection is proven only by AC-4 (unit) and AC-11 (production `run_slice`).
- Materializing defaults can expose bad or conflicting manifest defaults formerly hidden by guest literals. Registry assembly, typing, and bounds failures must stay loud.
- Metadata additions have a struct-literal blast radius; Step 1 inventories it and Steps 2a–2c′ edit it atomically.
