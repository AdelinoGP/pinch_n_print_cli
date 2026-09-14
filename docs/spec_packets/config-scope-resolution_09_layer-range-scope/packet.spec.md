---
status: draft
packet: config-scope-resolution_09_layer-range-scope
task_ids:
  - TASK-570
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: layer-range-scope

## Goal

Ingest OrcaSlicer's per-object `Metadata/layer_config_ranges.xml` into a first-class typed layer-range scope, compose `layer_height` ranges into the object Z-grid with canonical earlier-starting-wins trimming and gap fill, and apply all other admitted range values through the shared scope resolver in both production setup paths.

## Scope Boundaries

This packet owns the XML adapter and one committed single-range 3MF fixture, the net-new packet-03 layer-range scope variant and interval carrier, packet-05's reserved layer-range precedence slot in `query_z_grid` and `resolve_scope_stack`, packet-07 admission enforcement, runtime/visual-debug wiring, and focused docs/tests. It does not change modifier semantics, introduce a second resolver, or amend the source plan or predecessor packets; `requirements.md` records the approved correction to the plan's inaccurate overlap parenthetical.

## Prerequisites and Blockers

- Depends on: **FORWARD-DEP** packet 05, `config-scope-resolution_05_scope-resolution-module`, for `query_z_grid`, `resolve_scope_stack`, `ResolutionTarget`, and `ResolutionError`.
- Depends on: **FORWARD-DEP** packet 07, `config-scope-resolution_07_scope-eligibility`, for `ConfigSchemaRegistry::admission_set` and `ResolutionError::ScopeDenied`.
- Also consumes packet 03's **FORWARD-DEP** `ConfigScope`, `ScopeDelta`, `ScopedConfig`, and `ConfigIngestor`; `ConfigScope::LayerRange` is net-new in this packet because packet 03 explicitly omits it.
- Activation blockers: packets 03, 05, and 07 must land and their actual exported names/shapes must be reconciled. No unresolved policy blocker remains.

## Compatibility Checklist

- IR schema versions: unchanged; range source and typed scope data remain host-side and are not serialized in MeshIR/SliceIR.
- WIT packages: unchanged; packet 05's resolved per-object planning record remains the layer-planning boundary.
- CLI schema wire: unchanged; no config-schema field is added or removed.
- Manifest schema: unchanged; packet 07's snake_case `denied_scopes` vocabulary already includes `layer_range`.
- Runtime behavior: valid range statements affect Z planning/region resolution; malformed, invalid, denied, and conflicting inputs fail the model/config load atomically.

## Acceptance Criteria

- **AC-1. Given** `resources/layer_range_one_range.3mf` containing `Metadata/layer_config_ranges.xml` with root `objects`, one `<object id="1">`, one `<range min_z="0.4" max_z="0.8">`, and one `<option opt_key="layer_height">0.1</option>`, **when** `read_3mf_layer_config_ranges` reads it, **then** it returns exactly one `RawLayerConfigRange` whose `object_ordinal == 1`, `min_z == 0.4`, `max_z == 0.8`, and `values == {"layer_height": "0.1"}`; the ordinal is mapped to the first MeshIR object independently of that object's 3MF model id. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-model-io --all-targets --test layer_config_ranges_tdd parses_canonical_single_range_fixture_and_maps_one_based_ordinal -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test parses_canonical_single_range_fixture_and_maps_one_based_ordinal .* ok" target/test-output.log'`
- **AC-2. Given** a parsed range for object `obj-a` with world-Z interval `[0.4, 0.8)` and admitted values `layer_height = 0.1` and `infill_density = 0.35`, **when** registry-aware ingestion runs, **then** `ScopedConfig` contains one `LayerConfigRange` identified by `ConfigScope::LayerRange { object_id: "obj-a", range_index: 0 }`, preserves `min_z = 0.4` and `max_z = 0.8` as millimetres, types both values through the registry, and matches a layer top Z of `0.4` but not `0.8`. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-config --all-targets --test layer_range_scope_tdd world_z_half_open_range_is_typed_and_indexed_per_object -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test world_z_half_open_range_is_typed_and_indexed_per_object .* ok" target/test-output.log'`
- **AC-3. Given** base `layer_height = 0.20`, an earlier range `[0.20, 0.70)` stating `0.10`, and a later range `[0.50, 0.90)` stating `0.25`, **when** `query_z_grid` composes the layer-height profile, **then** the earlier range retains `[0.20, 0.70)`, the later range is trimmed rather than replacing it and contributes only `[0.70, 0.90)`, uncovered gaps use `0.20`, the fixed first-layer segment remains first, and the resulting breakpoints/values equal the independently authored expected profile. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-config --all-targets --test layer_range_scope_tdd earlier_starting_layer_height_range_wins_by_trimming_later_overlap -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test earlier_starting_layer_height_range_wins_by_trimming_later_overlap .* ok" target/test-output.log'`
- **AC-4. Given** a global-grid catch-up layer for `obj-a` whose top print Z is inside `[0.4, 0.8)` and whose bottom Z is outside it, **when** `resolve_scope_stack` resolves that layer's region target, **then** it inserts the layer-range delta after object and before modifier, uses the top print Z for membership, and returns the range's `infill_density = 0.35`; removing modifier, range, and object scopes in turn exposes the independently authored lower-precedence values. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-config --all-targets --test layer_range_scope_tdd catch_up_layer_inherits_range_covering_its_top_z_at_reserved_precedence -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test catch_up_layer_inherits_range_covering_its_top_z_at_reserved_precedence .* ok" target/test-output.log'`
- **AC-5. Given** the single-range fixture and equivalent authored configuration, **when** ordinary slicing reaches `run_slice_with_collector` and visual-debug reaches `prepare_prepass_context`, **then** both pass the same typed range set to `query_z_grid` and `resolve_scope_stack`, produce the same object layer plan and resolved in-range value, and neither reparses XML or implements a local range overlay. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-runtime --all-targets --test integration layer_range_scope_tdd::both_production_entry_points_share_range_resolution -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "layer_range_scope_tdd::both_production_entry_points_share_range_resolution .* ok" target/test-output.log'`
- **AC-6. Given** `resources/layer_range_one_range.3mf`, **when** the real `pnp_cli visual-debug` model-source pipeline renders a `Layer::Slice` silhouette bundle spanning layers below, inside, and above the range, **then** it succeeds, writes `manifest.json`, records non-empty rendered layers, and its scheduled print-Z sequence reflects the `0.1` in-range height rather than a uniform `0.2` grid. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p pnp_cli --all-targets --test layer_range_scope_visual_debug_tdd layer_range_fixture_changes_real_visual_debug_z_schedule -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test layer_range_fixture_changes_real_visual_debug_z_schedule .* ok" target/test-output.log'`
- **AC-7. Given** the implemented host-side contract, **when** authoritative docs are inspected, **then** `docs/02_ir_schemas.md` states `global < object < layer range < modifier < paint semantic < tool`, world-Z half-open matching, earlier-starting `layer_height` trimming with gap fill, conflicting non-`layer_height` rejection, and catch-up top-Z inheritance, while `docs/04_host_scheduler.md` names both `query_z_grid` and `resolve_scope_stack` as consumers of the same typed range set. | `python3 -c "from pathlib import Path; a=Path('docs/02_ir_schemas.md').read_text(); b=Path('docs/04_host_scheduler.md').read_text(); req=('global < object < layer range < modifier < paint semantic < tool','half-open','earlier-starting','trim','gap','catch-up','top Z','non-layer_height','load error'); missing=[x for x in req if x not in a]; assert not missing and all(x in b for x in ('query_z_grid','resolve_scope_stack','layer range')),(missing,b[:0])"`

## Negative Test Cases

- **AC-N1. Given** two overlapping ranges on one object that state different `infill_density` values over a non-empty overlap, **when** layer-range ingestion validates them, **then** it returns `LayerRangeLoadError::ConflictingOverlap { object_id, key, first_range, second_range }` before either resolution entry point runs; equal values are accepted and `layer_height` is governed by AC-3 instead. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-config --all-targets --test layer_range_scope_tdd conflicting_non_layer_height_overlap_is_atomic_load_error -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test conflicting_non_layer_height_overlap_is_atomic_load_error .* ok" target/test-output.log'`
- **AC-N2. Given** a layer range stating selector key `wall_generator` or denied machine key `bed_shape`, **when** registry-aware ingestion validates the range, **then** model/config load returns `ResolutionError::ScopeDenied { key, scope: ConfigScope::LayerRange { .. } }` and retains no partial range or resolved config. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-config --all-targets --test layer_range_scope_tdd selector_or_denied_key_in_layer_range_is_load_error -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test selector_or_denied_key_in_layer_range_is_load_error .* ok" target/test-output.log'`
- **AC-N3. Given** malformed XML, missing/non-finite bounds, `min_z >= max_z`, object ordinal `0`, or duplicate object sections, **when** the range part is parsed, **then** a named `LayerRangeParseError` is returned and no valid sibling range is exposed; a missing `Metadata/layer_config_ranges.xml` remains an empty successful result. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-model-io --all-targets --test layer_config_ranges_tdd malformed_or_invalid_ranges_fail_atomically_but_missing_part_is_empty -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test malformed_or_invalid_ranges_fail_atomically_but_missing_part_is_empty .* ok" target/test-output.log'`
- **AC-N4. Given** a syntactically valid range whose one-based object ordinal has no corresponding loaded MeshIR object, **when** registry-aware range ingestion maps ordinals, **then** it returns `LayerRangeLoadError::InvalidObjectOrdinal { object_ordinal }` before typing or committing any sibling range. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-config --all-targets --test layer_range_scope_tdd unmapped_object_ordinal_is_atomic_load_error -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test unmapped_object_ordinal_is_atomic_load_error .* ok" target/test-output.log'`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-runtime --all-targets --test integration layer_range_scope_tdd -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test result: ok" target/test-output.log'`

## Authoritative Docs

- `docs/specs/config-scope-resolution-plan.md` — RC-7, Resolution, Layer range geometry semantics, cross-cutting visual-debug requirement, and queue row 9; read-only and not amended by this packet.
- `docs/adr/0068-config-scope-is-a-wire-encoding.md` — decode-once typed scope boundary.
- `docs/adr/0069-scope-eligibility-is-a-per-key-deny-list.md` — denied statements reject loudly.
- `docs/02_ir_schemas.md` and `docs/04_host_scheduler.md` — scope precedence, Z-grid, and scheduler-resolution contracts.
- `docs/08_coordinate_system.md` — world-Z millimetre/internal-unit boundary.
- `docs/19_visual_debug.md` — deterministic bundle/manifest gate.
- `docs/22_test_quality.md` — independent expectations and non-vacuous fixture assertions.

## Doc Impact Statement (Required)

- `docs/02_ir_schemas.md` section “Config Key Namespaces” — document the first-class layer-range scope and corrected composition semantics; verified by `AC-7`.
- `docs/04_host_scheduler.md` config-resolution/layer-planning sections — document shared typed-range consumption by both resolver queries; verified by `AC-7`.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Format/bbs_3mf.cpp` — `_BBS_3MF_Exporter::_add_layer_config_ranges_file_to_archive` and `_BBS_3MF_Importer::_extract_layer_config_ranges_from_archive`; confirm XML shape, one-based object ordinal linkage, and load failures.
- `OrcaSlicerDocumented/src/libslic3r/Slicing.cpp` — `layer_height_profile_from_ranges`; confirm ascending range iteration, earlier-starting overlap retention by trimming later lows, fixed first-layer precedence, and canonical gap fill.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
