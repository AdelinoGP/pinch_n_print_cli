---
status: implemented
packet: config-scope-resolution_09_layer-range-scope
task_ids:
  - TASK-570
backlog_source: docs/07_implementation_status.md
context_cost_estimate: L
---

# Packet Contract: layer-range-scope

## Goal

Ingest OrcaSlicer's per-object `Metadata/layer_config_ranges.xml` into a first-class typed layer-range scope; compose `layer_height` ranges into a host-side per-object layer-height profile (canonical earlier-starting-wins trimming with gap fill) and derive each object's explicit top-Z schedule; carry that schedule across the `PrePass::LayerPlanning` WIT boundary so the guest layer planner emits variable-step layers; apply all other admitted range values through the shared scope resolver per layer in both production setup paths; and expose the scheduled Z sequence in the visual-debug manifest.

## Amendment 1 (scope expansion, owner-approved)

The original draft deferred WIT/guest changes. Reconciliation proved the layer Z schedule is produced by guest `layer-planner-default` from scalar per-object heights, so `layer_height` ranges cannot affect any schedule without a WIT change; AC-3's "breakpoints/values" also required a net-new host profile API. The owner approved expanding this packet in place to own:

- WIT package `slicer:prepass-layer-planning` bump `@2.0.0 → @3.0.0` and `object-layer-config.layer-zs: list<f64>`.
- Guest `layer-planner-default` consumption of explicit per-object top-Z schedules (variable step), preserving uniform fallback when `layer-zs` is empty.
- Host `slicer-config` profile/top-Z derivation API (`query_layer_height_profile`, `layer_top_zs`).
- Per-layer application of non-`layer_height` ranges in the runtime region-mapping builtin via `resolve_scope_stack` with a layer-top-Z target.
- Additive `manifest.json` field `scheduled_layer_zs` (visual-debug schema 1.3).

The original `layer_height`-overlap correction stands: canonical `layer_height_profile_from_ranges` iterates ascending and clips later ranges' low to the last pushed high, so earlier-starting wins.

## Scope Boundaries

This packet owns the XML adapter and one committed single-range 3MF fixture; the net-new `ConfigScope::LayerRange` variant, `LayerConfigRange` interval carrier, `LayerRangeInput`, ingestion/validation, and profile/top-Z API in `slicer-config`; the `ResolutionTarget.layer_top_z` extension and range insertion in `resolve_scope_stack`; the `scheduled_layer_zs` manifest field; the `prepass-layer-planning@3.0.0` WIT bump and its host/guest/macro/doc reconciliation; the guest variable-schedule algorithm; the runtime carrier and per-layer region-config application; pnp-cli ordinary/visual-debug adapters; and focused docs/tests. It does not change modifier semantics, introduce a second resolver, amend the source plan or predecessor packets, or change serialized MeshIR/SliceIR.

## Prerequisites and Blockers

- Packet 03 (`ConfigScope`, `ScopeDelta`, `ScopedConfig`, `ConfigIngestor`), packet 05 (`query_z_grid`, `resolve_scope_stack`, `ResolutionTarget`, `ResolutionError`), and packet 07 (`ConfigSchemaRegistry::admission_set`, `ResolutionError::ScopeDenied`) are landed and reconciled: `ConfigScope` has no `LayerRange`; `ResolutionTarget` has no Z field; registry assembly and `ConfigIngestor` live in `slicer-wasm-host::execution_plan_live::load_live_modules_for_plan_manifest_first`; `pnp-cli` has no `slicer-config` dependency yet; the guest emits a uniform-step schedule only.
- No unresolved policy blocker remains.

## Compatibility Checklist

- IR schema versions: unchanged; MeshIR/SliceIR gain no field and no version bump.
- WIT packages: `slicer:prepass-layer-planning` **bumps 2.0.0 → 3.0.0** (record field addition is breaking); `StageSpec.wit_package`, the schema README, `docs/03_wit_and_manifest.md`, `slicer-macros` binding-surface fixture, and the guest binding test update together; guests rebuild via `cargo xtask build-guests`.
- CLI schema wire: unchanged; layer ranges come from the model file, not CLI flags.
- Manifest schema: additive `schema_version` 1.3 with `scheduled_layer_zs`; older schema versions remain readable.
- Runtime behavior: valid range statements affect Z planning and per-layer region resolution; malformed, invalid, denied, conflicting, and unmapped inputs fail the model/config load atomically.

## Acceptance Criteria

- **AC-1. Given** `resources/layer_range_one_range.3mf` containing `Metadata/layer_config_ranges.xml` with root `objects`, one `<object id="1">`, one `<range min_z="0.4" max_z="0.8">`, and one `<option opt_key="layer_height">0.1</option>`, **when** `read_3mf_layer_config_ranges` reads it, **then** it returns exactly one `RawLayerConfigRange` whose `object_ordinal == 1`, `min_z == 0.4`, `max_z == 0.8`, and `values == {"layer_height": "0.1"}`; and `map_layer_config_ranges` maps that ordinal to the first loaded `ObjectId` independently of the 3MF model XML id. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-model-io --all-targets --test layer_config_ranges_tdd parses_canonical_single_range_fixture_and_maps_one_based_ordinal -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test parses_canonical_single_range_fixture_and_maps_one_based_ordinal .* ok" target/test-output.log'`
- **AC-2. Given** a parsed range for object `obj-a` with world-Z interval `[0.4, 0.8)` and admitted values `layer_height = 0.1` and `infill_density = 0.35`, **when** registry-aware ingestion runs, **then** `ScopedConfig` contains one `LayerConfigRange` identified by `ConfigScope::LayerRange { object_id: "obj-a", range_index: 0 }`, preserves `min_z = 0.4` and `max_z = 0.8` as millimetres, types both values through the registry, and matches a layer top Z of `0.4` but not `0.8`. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-config --all-targets --test layer_range_scope_tdd world_z_half_open_range_is_typed_and_indexed_per_object -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test world_z_half_open_range_is_typed_and_indexed_per_object .* ok" target/test-output.log'`
- **AC-3. Given** base `layer_height = 0.20`, `first_layer_height = 0.20`, object height `1.0`, an earlier range `[0.20, 0.70)` stating `0.10`, and a later range `[0.50, 0.90)` stating `0.25`, **when** `query_layer_height_profile` composes the layer-height profile, **then** the returned literal segments are exactly `[(0.00, 0.20, 0.20), (0.20, 0.70, 0.10), (0.70, 0.90, 0.25), (0.90, 1.00, 0.20)]` (earlier range retained, later trimmed), and `layer_top_zs` returns `[0.20, 0.30, 0.40, 0.50, 0.60, 0.70, 0.95]` (the 0.95 top starts in the trimmed 0.25 segment; the next step 1.15 exceeds the 1.0 object height). A later-wins implementation fails the segment assertion. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-config --all-targets --test layer_range_scope_tdd earlier_starting_layer_height_range_retains_overlap_and_gap_fills -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test earlier_starting_layer_height_range_retains_overlap_and_gap_fills .* ok" target/test-output.log'`
- **AC-4. Given** `ResolutionTarget { object_id: "obj-a", layer_top_z: Some(0.65), .. }` and scopes object `infill_density = 0.20`, layer range `[0.4, 0.8)` `infill_density = 0.35`, modifier `infill_density = 0.45`, **when** `resolve_scope_stack` resolves, **then** it returns `0.45` (modifier above range); removing the modifier yields `0.35`; a target with `layer_top_z: Some(0.85)` or `Some(0.80)` excludes the range and yields the object value; and removing range, modifier, then object scopes in turn exposes each independently authored lower-precedence value. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-config --all-targets --test layer_range_scope_tdd catch_up_layer_inherits_range_covering_its_top_z_at_reserved_precedence -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test catch_up_layer_inherits_range_covering_its_top_z_at_reserved_precedence .* ok" target/test-output.log'`
- **AC-5. Given** the single-range fixture's equivalent typed input (`layer_height = 0.1` over `[0.4, 0.8)`, base `0.2`, height `1.0`), **when** ordinary slicing reaches `run_slice_with_collector` and visual-debug reaches `prepare_prepass_context`, **then** both produce the identical `LayerPlanIR.global_layers` Z sequence `[0.2, 0.4, 0.5, 0.6, 0.7, 0.8, 1.0]`, both resolve the in-range `infill_density = 0.35` at layer top `0.6` through the same `resolve_scope_stack` target, and neither reparses XML or implements a local range overlay. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-runtime --all-targets --test integration layer_range_scope_tdd::both_production_entry_points_share_range_resolution -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "layer_range_scope_tdd::both_production_entry_points_share_range_resolution .* ok" target/test-output.log'`
- **AC-6. Given** `resources/layer_range_one_range.3mf`, **when** the real `pnp_cli visual-debug` model-source pipeline renders a `Layer::Slice` silhouette bundle spanning layers below, inside, and above the range, **then** it succeeds, writes `manifest.json`, records non-empty rendered layers, and `scheduled_layer_zs` equals the mixed `[0.2, 0.4, 0.5, 0.6, 0.7, 0.8, 1.0]` sequence rather than the uniform `[0.2, 0.4, 0.6, 0.8, 1.0]` grid. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p pnp-cli --all-targets --test layer_range_scope_visual_debug_tdd layer_range_fixture_changes_real_visual_debug_z_schedule -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test layer_range_fixture_changes_real_visual_debug_z_schedule .* ok" target/test-output.log'`
- **AC-7. Given** the guest layer planner and WIT contract, **when** `object-layer-config.layer-zs` is non-empty, **then** the planner emits exactly those top Zs (raft-offset applied) with variable-step `effective_layer_height` (first layer credited `first_layer_height`, later native layers `z - previous top`), still routes any object lacking `layer-zs` through the uniform formula, and host `layer_zs_round_trip_through_wit_record` proves the field survives the `@3.0.0` boundary. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test --manifest-path modules/core-modules/layer-planner-default/Cargo.toml --test layer_planning_tdd explicit_layer_zs_drive_variable_schedule_and_catch_up -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test explicit_layer_zs_drive_variable_schedule_and_catch_up .* ok" target/test-output.log && cargo test -p slicer-wasm-host --all-targets --test contract layer_zs_round_trip_through_wit_record -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null && rg -q "test layer_zs_round_trip_through_wit_record .* ok" target/test-output.log'`
- **AC-8. Given** the implemented host-side contract, **when** authoritative docs are inspected, **then** `docs/02_ir_schemas.md` states `global < object < layer range < modifier < paint semantic < tool`, world-Z half-open matching, earlier-starting `layer_height` trimming with gap fill, conflicting non-`layer_height` rejection, catch-up top-Z inheritance, and explicit top-Z schedules; `docs/04_host_scheduler.md` names `query_z_grid`, `query_layer_height_profile`, `layer_top_zs`, and `resolve_scope_stack` as consumers/producers of the same typed range set; `docs/03_wit_and_manifest.md` records the `@3.0.0` `layer-zs` field; and `docs/19_visual_debug.md` documents `scheduled_layer_zs`. | `python3 -c "from pathlib import Path; enc='utf-8'; a=Path('docs/02_ir_schemas.md').read_text(encoding=enc); b=Path('docs/04_host_scheduler.md').read_text(encoding=enc); c=Path('docs/03_wit_and_manifest.md').read_text(encoding=enc); d=Path('docs/19_visual_debug.md').read_text(encoding=enc); req=('global < object < layer range < modifier < paint semantic < tool','half-open','earlier-starting','trim','gap','catch-up','top Z','non-layer_height','load error'); missing=[x for x in req if x not in a]; assert not missing and all(x in b for x in ('query_z_grid','query_layer_height_profile','layer_top_zs','resolve_scope_stack','layer range')) and 'prepass-layer-planning@3.0.0' in c and 'layer-zs' in c and 'scheduled_layer_zs' in d, (missing, [x for x in ('query_z_grid','query_layer_height_profile','layer_top_zs','resolve_scope_stack') if x not in b])"`

## Negative Test Cases

- **AC-N1. Given** two overlapping ranges on one object that state different `infill_density` values over a non-empty overlap, **when** layer-range ingestion validates them, **then** it returns `LayerRangeLoadError::ConflictingOverlap { object_id, key, first_range, second_range }` before either resolution entry point runs; equal values are accepted and `layer_height` is governed by AC-3 instead. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-config --all-targets --test layer_range_scope_tdd conflicting_non_layer_height_overlap_is_atomic_load_error -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test conflicting_non_layer_height_overlap_is_atomic_load_error .* ok" target/test-output.log'`
- **AC-N2. Given** a layer range stating selector key `wall_generator` or denied machine key `bed_shape`, **when** registry-aware ingestion validates the range, **then** model/config load returns `LayerRangeLoadError::Denied(ResolutionError::ScopeDenied { scope: ConfigScope::LayerRange { .. }, .. })` and retains no partial range or resolved config. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-config --all-targets --test layer_range_scope_tdd selector_or_denied_key_in_layer_range_is_load_error -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test selector_or_denied_key_in_layer_range_is_load_error .* ok" target/test-output.log'`
- **AC-N3. Given** malformed XML, missing/non-finite bounds, `min_z >= max_z`, object ordinal `0`, or duplicate object sections, **when** the range part is parsed, **then** a named `LayerRangeParseError` is returned and no valid sibling range is exposed; a missing `Metadata/layer_config_ranges.xml` remains an empty successful result. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-model-io --all-targets --test layer_config_ranges_tdd malformed_or_invalid_ranges_fail_atomically_but_missing_part_is_empty -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test malformed_or_invalid_ranges_fail_atomically_but_missing_part_is_empty .* ok" target/test-output.log'`
- **AC-N4. Given** a syntactically valid range whose one-based object ordinal has no corresponding loaded MeshIR object, **when** `map_layer_config_ranges` maps ordinals, **then** it returns `LayerRangeParseError::InvalidObjectOrdinal { object_ordinal }` before any sibling range is exposed for typing. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-model-io --all-targets --test layer_config_ranges_tdd unmapped_object_ordinal_is_atomic_load_error -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test unmapped_object_ordinal_is_atomic_load_error .* ok" target/test-output.log'`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask check-literals`
- `cargo xtask check-test-quality --report`
- `cargo xtask build-guests --check`
- `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-runtime --all-targets --test integration layer_range_scope_tdd -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test result: ok" target/test-output.log'`

## Authoritative Docs

- `docs/specs/config-scope-resolution-plan.md` — RC-7, Resolution, Layer range geometry semantics, cross-cutting visual-debug requirement, and queue row 9; read-only and not amended by this packet.
- `docs/adr/0068-config-scope-is-a-wire-encoding.md` — decode-once typed scope boundary.
- `docs/adr/0069-scope-eligibility-is-a-per-key-deny-list.md` — denied statements reject loudly.
- `docs/02_ir_schemas.md`, `docs/04_host_scheduler.md`, `docs/03_wit_and_manifest.md`, `docs/19_visual_debug.md` — scope precedence, Z-grid, scheduler, WIT, and manifest contracts.
- `docs/08_coordinate_system.md` — world-Z millimetre/internal-unit boundary.
- `docs/22_test_quality.md` — independent expectations and non-vacuous fixture assertions.

## Doc Impact Statement (Required)

- `docs/02_ir_schemas.md` — first-class layer-range scope, precedence, composition semantics, top-Z schedules; verified by `AC-8`.
- `docs/04_host_scheduler.md` — shared typed-range consumption by profile/top-Z and both resolver queries; verified by `AC-8`.
- `docs/03_wit_and_manifest.md` — `prepass-layer-planning@3.0.0` `layer-zs` field; verified by `AC-8`.
- `docs/19_visual_debug.md` — additive `scheduled_layer_zs`; verified by `AC-8`.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Format/bbs_3mf.cpp` — `_BBS_3MF_Exporter::_add_layer_config_ranges_file_to_archive` and `_BBS_3MF_Importer::_extract_layer_config_ranges_from_archive`; confirmed XML shape, one-based object ordinal linkage, and deferred load errors (already delegated; reuse).
- `OrcaSlicerDocumented/src/libslic3r/Slicing.cpp` — `layer_height_profile_from_ranges`; confirmed ascending iteration, earlier-starting overlap retention by trimming later lows, fixed first-layer precedence, and canonical gap fill (already delegated; reuse).

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
