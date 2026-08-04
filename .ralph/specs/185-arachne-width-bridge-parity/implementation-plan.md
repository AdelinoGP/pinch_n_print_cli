# Implementation Plan: 185-arachne-width-bridge-parity

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Scheduler percent retention and transport

- Task IDs: `TASK-303`
- Objective: make `parse_config_field_entry`'s call sites in `crates/slicer-scheduler/src/manifest.rs` retain the `ConfigValue::Percent` / `ConfigValue::FloatOrPercent` returned by `parse_percent_default` (today invoked as a bare validation statement and discarded, per the wall-width/percent-transport residual), and thread the retained value through `crates/slicer-scheduler/src/config_resolution.rs` into `ResolvedConfig.extensions` (`crates/slicer-ir/src/resolved_config.rs`, `extensions` at :650). Do NOT touch `ResolvedConfig::to_config_map` — the wall-width/percent-transport residual verified its extensions pass-through is already transparent.
- Precondition: packet activated; `cargo xtask build-guests --check` clean on the baseline tree.
- Postcondition: a manifest `[config.schema.*]` entry of type `percent`/`float_or_percent` with a percent default round-trips end-to-end into `ResolvedConfig.extensions` as the same `ConfigValue` variant (no coercion to `Float`); the percent round-trip AC is testable in later steps.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-scheduler/src/manifest.rs` - lines `[1030-1180]` (`read_config_schema` at :1036, `parse_config_field_entry`, `parse_percent_default`)
  - `crates/slicer-scheduler/src/config_resolution.rs` - full if under 300 lines, else resolution-entry functions only
  - `crates/slicer-ir/src/slice_ir.rs` - lines `[686-720]` (`ConfigValue` at :691)
  - `docs/DEVIATION_LOG.md` - the wall-width/percent-transport residual only
- Files allowed to edit (at most 3):
  - `crates/slicer-scheduler/src/manifest.rs`
  - `crates/slicer-scheduler/src/config_resolution.rs`
   - `crates/slicer-scheduler/tests/integration/config_resolution_tdd.rs` (existing registered integration module)
- Files explicitly out of bounds:
  - `crates/slicer-ir/src/resolved_config.rs` (`to_config_map` is not the barrier)
  - `crates/slicer-schema/wit/**` (no WIT change)
  - `OrcaSlicerDocumented/**`, `target/**`
- Expected sub-agent dispatches:
  - Question: list every existing test that asserts percent defaults are rejected or coerced; scope: `crates/slicer-scheduler/tests/**`; return: `LOCATIONS`
- Context cost: `M`
- Authoritative docs:
  - `docs/DEVIATION_LOG.md` - the wall-width/percent-transport residual only (ranged read)
- OrcaSlicer refs:
  - none
- Verification:
   - `set -o pipefail; cargo test -p slicer-scheduler --all-targets --test scheduler_integration -- percent_round_trip 2>&1 | tee target/test-output.log | rg '^test result: ok'` - FACT pass/fail
  - `cargo check -p slicer-scheduler --all-targets` - FACT pass/fail
- Exit condition: new percent round-trip tests green; no existing scheduler test weakened; check clean.

### Step 2: ResolvedConfig field rename + legacy alias + blast radius

- Task IDs: `TASK-303`
- Objective: rename the `ResolvedConfig` field declared at `crates/slicer-ir/src/resolved_config.rs:829` from `first_layer_line_width` to `initial_layer_line_width` (canonical rename); update `to_config_map`, `PartialEq`, `Hash`, and `crates/slicer-core/src/algos/region_mapping.rs` field accesses; retain the scheduler alias and both-keys rejection introduced in Step 1; defaults move to auto (`0`) for the width keys per brief decision 4.
- Precondition: Step 1 green; the blast-radius `LOCATIONS` dispatch (below) has returned and its sites are baked into 'Files allowed to edit' before authoring begins.
- Postcondition: tree compiles with zero remaining `first_layer_line_width` Rust-field references; old-key profiles still resolve (alias); both-keys profiles produce a hard validation error at config resolution; `cargo check --workspace --all-targets` clean.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/resolved_config.rs` - lines `[807-860]` (`declare_resolved_config!` invocation; rename line :829) and `[630-670]` (macro emit)
  - `crates/slicer-scheduler/src/config_resolution.rs` - resolution-entry functions only
- Files allowed to edit (at most 3):
   - `crates/slicer-ir/src/resolved_config.rs`
   - `crates/slicer-core/src/algos/region_mapping.rs`
   - `crates/slicer-ir/tests/resolved_config_defaults_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-schema/wit/**` (key strings are not WIT records; confirm via dispatch)
  - Any module `src/lib.rs` (module migration is Steps 4-8)
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
   - This is a public-struct field rename: the verified production blast radius is `crates/slicer-ir/src/resolved_config.rs` (`to_config_map`, `PartialEq`, `Hash`) plus `crates/slicer-core/src/algos/region_mapping.rs` (comparison and assignment). The scheduler's legacy string remains only in its alias/rejection path from Step 1. No WIT identifier carries this key.
- Expected sub-agent dispatches:
  - Question: enumerate all `first_layer_line_width` references (struct literals, accessors, test assertions, CLI strings); scope: `crates/** modules/**`; return: `LOCATIONS`
  - Question: does any WIT record or dispatch path carry the string `first_layer_line_width`?; scope: `crates/slicer-schema/wit/** crates/slicer-runtime/src/**`; return: `FACT`
- Context cost: `M`
- Authoritative docs:
  - `docs/07_implementation_status.md` - `TASK-302` row (:172) only — serde shape is in-process, no schema-version bump
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - delegate confirmation of the canonical key name `initial_layer_line_width`; never load
- Verification:
   - `rg -n "first_layer_line_width" crates/slicer-ir/src/resolved_config.rs crates/slicer-core/src/algos/region_mapping.rs` returns no field/accessor references - FACT
   - `cargo check --workspace --all-targets` - FACT pass/fail
   - `set -o pipefail; cargo test -p slicer-ir --all-targets --test resolved_config_defaults_tdd 2>&1 | tee target/test-output.log | rg '^test result: ok'` - FACT pass/fail
  - `cargo xtask build-guests --check` (slicer-ir edit stales guests) - FACT clean-or-rebuilt
- Exit condition: rename complete, alias + both-keys rejection tested, workspace check clean, guest freshness confirmed.

### Step 3: `resolve_role_width` in flow.rs

- Task IDs: `TASK-303`
- Objective: add `resolve_role_width` to `crates/slicer-core/src/flow.rs`, keyed by explicit canonical role + first-layer/bridge context, implementing the locked precedence chain: configured `bridge_line_width`; else positive `initial_layer_line_width` on first layer; else role width; zero role width → `line_width` → auto (`0` sentinel = `1.125 × nozzle_diameter`, canonical `Flow.cpp::auto_extrusion_width`). `BottomSolidInfill` maps to `internal_solid_infill_line_width` except the first-layer/bridge overrides. Existing `line_width_to_spacing` (:86), `flow_to_width`, `bridging_flow` unchanged.
- Precondition: Step 2 green (canonical key name exists); `cargo xtask build-guests --check` clean after Step 2's slicer-ir rebuild.
- Postcondition: parameterized unit tests cover every role × {first layer, non-first} × {bridge override, bridge fallback} × {zero, positive, absent} width, all green; resolver is a pure mm-domain function.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/src/flow.rs` - full (284 lines)
  - `crates/slicer-core/tests/flow_tdd.rs` - full if under 300 lines, else the spacing-test section
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/flow.rs`
   - `crates/slicer-core/tests/flow_tdd.rs`
- Files explicitly out of bounds:
  - All module sources (consumers land Steps 4-8)
  - Flow-ratio/bridge-flow plumbing (deferred, `DEV-102`)
- Expected sub-agent dispatches:
  - Question: quote canonical `Flow.cpp::new_from_config_width` / `auto_extrusion_width` role-to-key mapping including bridge and first-layer branches; scope: `OrcaSlicerDocumented/src/libslic3r/Flow.cpp`; return: `SNIPPETS`
- Context cost: `S`
- Authoritative docs:
  - `docs/08_coordinate_system.md` - conversion checklist section only
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Flow.cpp` - delegate; never load
- Verification:
   - `set -o pipefail; cargo test -p slicer-core --all-targets --test flow_tdd 2>&1 | tee target/test-output.log | rg '^test result: ok'` - FACT pass/fail
  - `cargo xtask build-guests --check` (slicer-core is a universal guest dep) - FACT clean-or-rebuilt
- Exit condition: precedence-matrix unit tests green; no change to existing flow fns; guests fresh.

### Step 4: classic-perimeters resolver migration + D-105 infill-boundary formula

- Task IDs: `TASK-303`
- Objective: migrate `classic-perimeters` to `resolve_role_width` for outer/inner wall widths, and port canonical `PerimeterGenerator.cpp::process_classic`'s final-infill-boundary formula, replacing the raw `-inner_wall_line_width` offset in `ClassicPerimeters::emit_walls` (`modules/core-modules/classic-perimeters/src/lib.rs:1104`) — closes the `classic final-infill-boundary gap` surviving residual. Manifest declares the flow keys the module consumes.
- Precondition: Step 3 green; canonical formula `SNIPPETS` dispatch returned; guests rebuilt after Step 3.
- Postcondition: infill-boundary inset derives from the canonical formula (verified against the dispatch snippets); module reads widths exclusively via the resolver; manifest keys declared in snake_case.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/classic-perimeters/src/lib.rs` - lines `[570-620]` (`line_width_for` :591) and `[1095-1115]` (infill inset :1104)
  - `modules/core-modules/classic-perimeters/classic-perimeters.toml` - full
- Files allowed to edit (at most 3):
  - `modules/core-modules/classic-perimeters/src/lib.rs`
  - `modules/core-modules/classic-perimeters/classic-perimeters.toml`
   - `modules/core-modules/classic-perimeters/tests/classic_perimeters_tdd.rs`
- Files explicitly out of bounds:
   - `crates/slicer-core/src/top_surface_split.rs` (generic splitter untouched)
  - One-wall-top and overlap behavior (Step 9)
  - Other modules
- Expected sub-agent dispatches:
  - Question: quote canonical `process_classic`'s final infill-boundary inset (spacing vs raw width); scope: `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp`; return: `SNIPPETS`
- Context cost: `M`
- Authoritative docs:
   - `docs/DEVIATION_LOG.md` - `classic final-infill-boundary gap` row (:24) only
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp` - delegate; never load
- Verification:
  - `cargo xtask build-guests --check` then rebuild - FACT
   - `set -o pipefail; cargo test -p classic-perimeters --all-targets --test classic_perimeters_tdd 2>&1 | tee target/test-output.log | rg '^test result: ok'` - FACT pass/fail (goldens may drift; drift is recorded for Step 11, NOT re-blessed here)
- Exit condition: resolver wired, canonical formula in place, module tests green except recorded golden drift; `classic final-infill-boundary gap` residual code path removed.

### Step 5: arachne-perimeters resolver migration

- Task IDs: `TASK-303`
- Objective: migrate `arachne-perimeters` wall-width resolution to `resolve_role_width` (shared wall-width/percent-transport residual, Arachne half); manifest declares consumed flow keys. Do not alter beading, second-pass, or top-area logic.
- Precondition: Step 3 green; guests rebuilt after Step 4.
- Postcondition: width reads route through the shared resolver; arachne parity tests unchanged in outcome except recorded drift.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/arachne-perimeters/src/lib.rs` - the `ArachneParams`/width-read section only (locate via grep for `line_width`)
  - `modules/core-modules/arachne-perimeters/arachne-perimeters.toml` - full
- Files allowed to edit (at most 3):
  - `modules/core-modules/arachne-perimeters/src/lib.rs`
  - `modules/core-modules/arachne-perimeters/arachne-perimeters.toml`
   - `modules/core-modules/arachne-perimeters/tests/precise_outer_wall_tdd.rs`
- Files explicitly out of bounds:
  - `emit_only_one_wall_top_second_pass` (:923) and everything it calls
  - `crates/slicer-core/src/arachne/**` (ADR-0035 scope; no algorithm change)
- Expected sub-agent dispatches:
  - Question: list current width-key read sites in arachne-perimeters; scope: `modules/core-modules/arachne-perimeters/src/**`; return: `LOCATIONS`
- Context cost: `S`
- Authoritative docs:
   - `docs/DEVIATION_LOG.md` - the wall-width/percent-transport residual and `DEV-101` rows only
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Flow.cpp` - delegate; never load
- Verification:
  - `cargo xtask build-guests --check` then rebuild - FACT
   - `set -o pipefail; cargo test -p arachne-perimeters --all-targets --test precise_outer_wall_tdd 2>&1 | tee target/test-output.log | rg '^test result: ok'` - FACT pass/fail
   - `set -o pipefail; cargo test -p slicer-core --all-targets --features host-algos --test arachne_d5_taper_coverage 2>&1 | tee target/test-output.log | rg '^test result: ok'` - FACT no new failures
- Exit condition: resolver wired; no arachne algorithm change; recorded drift only.

### Step 6: rectilinear-infill resolver migration

- Task IDs: `TASK-303`
- Objective: replace the raw `line_width` read (`modules/core-modules/rectilinear-infill/src/lib.rs:80`) with `resolve_role_width` for the sparse-infill role (`sparse_infill_line_width`); manifest declares consumed keys.
- Precondition: Step 3 green; guests rebuilt after Step 5.
- Postcondition: infill width honors role key → `line_width` → auto fallback; percent forms resolve via `ConfigView::get_abs_value`.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/rectilinear-infill/src/lib.rs` - lines `[60-110]`
  - `modules/core-modules/rectilinear-infill/rectilinear-infill.toml` - full
- Files allowed to edit (at most 3):
  - `modules/core-modules/rectilinear-infill/src/lib.rs`
  - `modules/core-modules/rectilinear-infill/rectilinear-infill.toml`
- Files explicitly out of bounds:
  - gyroid/lightning modules (Steps 7-8)
  - Fill-pattern geometry
- Expected sub-agent dispatches:
  - none
- Context cost: `S`
- Authoritative docs:
  - `docs/08_coordinate_system.md` - conversion checklist section only
- OrcaSlicer refs:
  - none
- Verification:
  - `cargo xtask build-guests --check` then rebuild - FACT
   - `set -o pipefail; cargo test -p rectilinear-infill --all-targets 2>&1 | tee target/test-output.log | rg '^test result: ok'` - FACT pass/fail
- Exit condition: resolver wired; tests green.

### Step 7: gyroid-infill resolver migration

- Task IDs: `TASK-303`
- Objective: replace the raw `line_width` read (`modules/core-modules/gyroid-infill/src/lib.rs:125`) with `resolve_role_width` for the sparse-infill role; manifest declares consumed keys.
- Precondition: Step 3 green; guests rebuilt after Step 6.
- Postcondition: identical semantics to Step 6 for the gyroid module.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/gyroid-infill/src/lib.rs` - lines `[110-150]`
  - `modules/core-modules/gyroid-infill/gyroid-infill.toml` - full
- Files allowed to edit (at most 3):
  - `modules/core-modules/gyroid-infill/src/lib.rs`
  - `modules/core-modules/gyroid-infill/gyroid-infill.toml`
- Files explicitly out of bounds:
  - rectilinear/lightning modules
  - Gyroid field/geometry math
- Expected sub-agent dispatches:
  - none
- Context cost: `S`
- Authoritative docs:
  - `docs/08_coordinate_system.md` - conversion checklist section only
- OrcaSlicer refs:
  - none
- Verification:
  - `cargo xtask build-guests --check` then rebuild - FACT
   - `set -o pipefail; cargo test -p gyroid-infill --all-targets 2>&1 | tee target/test-output.log | rg '^test result: ok'` - FACT pass/fail
- Exit condition: resolver wired; tests green.

### Step 8: lightning-infill resolver migration

- Task IDs: `TASK-303`
- Objective: replace the raw `line_width` read (`modules/core-modules/lightning-infill/src/lib.rs:66`) with `resolve_role_width` for the sparse-infill role; manifest declares consumed keys.
- Precondition: Step 3 green; guests rebuilt after Step 7.
- Postcondition: identical semantics to Step 6 for the lightning module; `crates/slicer-core` lightning algos untouched.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/lightning-infill/src/lib.rs` - lines `[50-95]`
  - `modules/core-modules/lightning-infill/lightning-infill.toml` - full
- Files allowed to edit (at most 3):
  - `modules/core-modules/lightning-infill/src/lib.rs`
  - `modules/core-modules/lightning-infill/lightning-infill.toml`
- Files explicitly out of bounds:
  - `crates/slicer-core/src/lightning/**` (tree generation algorithm)
  - rectilinear/gyroid modules
- Expected sub-agent dispatches:
  - none
- Context cost: `S`
- Authoritative docs:
  - `docs/08_coordinate_system.md` - conversion checklist section only
- OrcaSlicer refs:
  - none
- Verification:
  - `cargo xtask build-guests --check` then rebuild - FACT
   - `set -o pipefail; cargo test -p lightning-infill --all-targets 2>&1 | tee target/test-output.log | rg '^test result: ok'` - FACT pass/fail
- Exit condition: resolver wired; tests green.

### Step 9: classic overlap keys + only_one_wall_top behavior

- Task IDs: `TASK-303`
- Objective: classic-perimeters declares `infill_wall_overlap` (`percent`, default `15`, ratio base `inner_wall_line_width`) and `top_bottom_infill_wall_overlap` (`percent`, default `25`, same ratio base), then gains (a) overlap selection: `top_bottom_infill_wall_overlap` for layer 0 and for `top_shell_index == Some(0)` (`crates/slicer-sdk/src/views.rs:41`), `infill_wall_overlap` otherwise; (b) module-local `only_one_wall_top`: topmost (`top_shell_index == Some(0)`) unconditional one wall; `min_width_top_surface` threshold on non-topmost top sub-areas, modeled on `emit_only_one_wall_top_second_pass` (`modules/core-modules/arachne-perimeters/src/lib.rs:923`). `top_surface_split.rs::split_top_surfaces` stays untouched; `D-152-TOP-AREA-SOURCE` stays open (no `upper_slices` access).
- Precondition: Step 4 green; canonical one-wall-top `SNIPPETS` dispatch returned.
- Postcondition: both overlap contexts and both one-wall-top contexts covered by parameterized tests; generic splitter byte-identical.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/arachne-perimeters/src/lib.rs` - lines `[900-960]` (model implementation)
  - `modules/core-modules/classic-perimeters/src/lib.rs` - the region-emit driver section (locate via grep for `top_shell_index`)
  - `crates/slicer-sdk/src/views.rs` - lines `[30-55]`
- Files allowed to edit (at most 3):
   - `modules/core-modules/classic-perimeters/src/lib.rs`
   - `modules/core-modules/classic-perimeters/classic-perimeters.toml`
   - `modules/core-modules/classic-perimeters/tests/classic_perimeters_tdd.rs`
- Files explicitly out of bounds:
   - `crates/slicer-core/src/top_surface_split.rs`
   - Any IR/WIT addition for `upper_slices` (`D-152-TOP-AREA-SOURCE` open)
  - arachne-perimeters source
- Expected sub-agent dispatches:
  - Question: quote canonical `only_one_wall_top` topmost vs non-topmost branches and the `min_width_top_surface` threshold in `process_classic`; scope: `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp`; return: `SNIPPETS`
- Context cost: `M`
- Authoritative docs:
   - `docs/DEVIATION_LOG.md` - `D-152-TOP-AREA-SOURCE` (:126) and `top-surface min-width threshold gap` (:105) rows only
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp` - delegate; never load
- Verification:
  - `cargo xtask build-guests --check` then rebuild - FACT
   - `set -o pipefail; cargo test -p classic-perimeters --all-targets --test classic_perimeters_tdd 2>&1 | tee target/test-output.log | rg '^test result: ok'` - FACT pass/fail (record golden drift; no re-bless here)
- Exit condition: both behaviors implemented and matrix-covered; splitter untouched; drift recorded.

### Step 10: Part-level width metadata allowlist

- Task IDs: `TASK-212b`
- Objective: ensure modifier-part 3MF metadata preserves `inner_wall_line_width`, `outer_wall_line_width`, and `sparse_infill_line_width` in `ModifierVolume.config_delta.fields` on the live path — part metadata is ingested generically into `config_delta.fields` in `crates/slicer-model-io/src/loader.rs`; `object_metadata_to_config_data` additionally allows the same three keys at object scope; add a production-loader-path regression test (`part_width_keys_survive_in_config_delta_fields`) asserting the three keys' exact values in `config_delta.fields`.
- Precondition: the canonical key names are fixed by Step 2; no WIT change is needed because the existing sidecar metadata path already carries arbitrary fields.
- Postcondition: the three width keys are asserted in `ModifierVolume.config_delta.fields` with their exact fixture values through the production `load_model` path; the raw sidecar regression remains green; unrelated unknown keys remain dropped at object scope.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-model-io/src/loader.rs` - locate `object_metadata_to_config_data` and the generic part-metadata ingestion loop (`config_delta.fields`)
  - `crates/slicer-model-io/tests/threemf_sidecar_classification_tdd.rs` - lines `[235-312]` (`parses_cube_cilindrical_modifier_sidecar` + `part_width_keys_survive_in_config_delta_fields`)
- Files allowed to edit (at most 3):
  - `crates/slicer-model-io/src/loader.rs`
  - `crates/slicer-model-io/tests/threemf_sidecar_classification_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-model-io/src/sidecar.rs` (raw metadata preservation already exists)
  - `crates/slicer-schema/wit/**` (no boundary change)
  - `target/**`, generated code, and unrelated loaders
- Expected sub-agent dispatches:
  - Question: verify the three width keys reach `config_delta.fields` via the generic part ingestion loop and unknown keys remain dropped at object scope; scope: `crates/slicer-model-io/src/loader.rs crates/slicer-model-io/tests/threemf_sidecar_classification_tdd.rs`; return: `SNIPPETS`
- Context cost: `S`
- Authoritative docs:
  - `docs/07_implementation_status.md` - `TASK-212b` row only
- OrcaSlicer refs:
  - none
- Verification:
   - `set -o pipefail; cargo test -p slicer-model-io --all-targets --test threemf_sidecar_classification_tdd -- part_width_keys_survive_in_config_delta_fields 2>&1 | tee target/test-output.log | rg '^test result: ok'` - FACT pass/fail
- Exit condition: all three width keys are asserted in `config_delta.fields` with exact values via the production loader; `TASK-212b` has a concrete passing regression.

### Step 11: precedence-matrix integration tests + deviation log + task crosswalk

- Task IDs: `TASK-303`, `TASK-212b`
- Objective: land the full parameterized precedence-matrix suite (every role × first-layer × bridge override/fallback × percent transport × module parity × both top-overlap contexts) BEFORE any golden re-bless; then update `docs/DEVIATION_LOG.md` (close the classic final-infill-boundary residual and the wall-width/percent-transport ingestion residual; add `Arachne wall-width ADR amendment` quoting ADR-0043's contested wall-width clause; add `guest-freshness ADR amendment` quoting ADR-0014's stale `slicer-core` freshness exclusion; create `DEV-102` for deferred flow-ratio controls — re-derive the next free DEV number at execution time with `rg -o '^\| DEV-[0-9]{3}' docs/DEVIATION_LOG.md | sort -u | tail -1`; leave `D-152-TOP-AREA-SOURCE` untouched) and the `docs/07_implementation_status.md` `TASK-303`/`TASK-212b` rows via worker dispatch (never a full backlog read).
- Precondition: Steps 1-10 green; drift inventory from Steps 4/5/9 collected.
- Postcondition: matrix suite green against canonical-correct semantics; deviation rows and task rows reflect reality; re-bless scope enumerated for Step 11.
- Files allowed to read, with ranges when over 300 lines:
   - `docs/DEVIATION_LOG.md` - surviving width residuals, `D-152-TOP-AREA-SOURCE`, and `DEV-102`; the wall-width and freshness amendment records are read from their ADR files
   - `docs/07_implementation_status.md` - lines `[150-175]` only
   - `docs/adr/0043-derive-arachne-bead-widths-from-wall-flows.md` - lines `[29-44]` only for the quoted Decision item 2 clause
   - `docs/adr/0014-xtask-guest-discovery-via-validated-filesystem-walk.md` - lines `[17-35]` only for the quoted freshness clause
- Files allowed to edit (at most 3):
   - `crates/slicer-runtime/tests/integration/per_object_config_override_tdd.rs` (existing registered integration module)
  - `docs/DEVIATION_LOG.md`
  - `docs/07_implementation_status.md`
- Files explicitly out of bounds:
   - Golden fixture JSONs (re-bless is Step 13)
- Expected sub-agent dispatches:
  - Question: apply the DEVIATION_LOG and docs/07 row edits; scope: the two files, cited rows only; return: `FACT`
  - Question: enumerate golden fixtures drifting from Steps 4/5/9; scope: `crates/slicer-runtime/tests/fixtures/** modules/core-modules/**/tests/**`; return: `LOCATIONS`
- Context cost: `M`
- Authoritative docs:
  - `docs/DEVIATION_LOG.md` - cited rows only
- OrcaSlicer refs:
  - none
- Verification:
  - `cargo xtask build-guests --check` - FACT clean
   - `set -o pipefail; cargo test -p slicer-runtime --all-targets --test integration -- precedence_matrix 2>&1 | tee target/test-output.log | rg '^test result: ok'` - FACT pass/fail
- Exit condition: matrix suite green; log + task rows updated; `D-152-TOP-AREA-SOURCE` untouched; ADR amendment row present; re-bless scope listed.

### Step 12: ADR amendment decision records

- Task IDs: `TASK-303`, `TASK-212b`
- Objective: append explicit amendment records to ADR-0043 and ADR-0014. Preserve each original decision text, name its `D-185-ADR-*` deviation, state the canonical-parity or current-freshness rationale, and do not rewrite unrelated ADR decisions.
- Precondition: Step 11 matrix and deviation/task ledger updates are green; the exact ADR clauses have been read through the delegated/ranged authority paths.
- Postcondition: both ADR files contain append-only amendment records for the exact deviation IDs; no new ADR slot is created and no original decision text is deleted.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/adr/0043-derive-arachne-bead-widths-from-wall-flows.md` - lines `[29-68]` only
  - `docs/adr/0014-xtask-guest-discovery-via-validated-filesystem-walk.md` - lines `[17-35]` only
- Files allowed to edit (at most 3):
  - `docs/adr/0043-derive-arachne-bead-widths-from-wall-flows.md`
  - `docs/adr/0014-xtask-guest-discovery-via-validated-filesystem-walk.md`
- Files explicitly out of bounds:
  - Any other ADR file or a new ADR filename/slot
  - Implementation code, `docs/DEVIATION_LOG.md`, and `docs/07_implementation_status.md` (owned by Step 11)
- Expected sub-agent dispatches:
  - Question: verify both amendment records preserve the original clauses and name the exact deviation IDs; scope: the two ADR files; return: `FACT`
- Context cost: `S`
- Authoritative docs:
  - `docs/adr/0043-derive-arachne-bead-widths-from-wall-flows.md` - Decision item 2
  - `docs/adr/0014-xtask-guest-discovery-via-validated-filesystem-walk.md` - freshness list
- OrcaSlicer refs:
  - none
- Verification:
  - `rg -q 'Arachne wall-width ADR amendment' docs/adr/0043-derive-arachne-bead-widths-from-wall-flows.md && rg -q 'guest-freshness ADR amendment' docs/adr/0014-xtask-guest-discovery-via-validated-filesystem-walk.md` - FACT pass/fail
- Exit condition: both amendment records exist, quote their contested clauses, and no unrelated ADR content changed.

### Step 13: Golden re-bless + acceptance

- Task IDs: `TASK-303`, `TASK-212b`
- Objective: re-bless only the golden fixtures enumerated in Step 11 whose drift is canonical-correct (per CLAUDE.md Test Discipline: update fixtures to match canonical-correct output; never weaken the canonical implementation to make baselines pass); run packet-level gates.
- Precondition: Steps 11-12 green; matrix suite pins intended semantics and ADR records are present.
- Postcondition: all pipe-suffixed AC commands PASS; `cargo clippy --workspace --all-targets -- -D warnings` clean; guests fresh; packet ready for `status: implemented`.
- Files allowed to read, with ranges when over 300 lines:
  - `target/test-output.log` - failure sections only
- Files allowed to edit (at most 3):
   - The enumerated golden fixture JSONs (batch via one worker dispatch)
   - `.ralph/specs/185-arachne-width-bridge-parity/packet.spec.md` (status flip only, at closure)
- Files explicitly out of bounds:
  - Any canonical implementation file (no weakening to satisfy baselines)
  - `.ralph/specs/184-*/**` (superseded packet's files are read-only)
- Expected sub-agent dispatches:
  - Question: re-record enumerated fixtures and confirm diff direction is canonical-correct; scope: enumerated fixture paths; return: `FACT`
- Context cost: `M`
- Authoritative docs:
   - `docs/DEVIATION_LOG.md` - rows amended in Step 11 only
   - `docs/adr/0043-derive-arachne-bead-widths-from-wall-flows.md` - quoted read-only clause only
- OrcaSlicer refs:
  - none
- Verification:
  - `cargo xtask build-guests --check` - FACT clean
  - `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail
  - Every pipe-suffixed AC command from `packet.spec.md`, re-dispatched - FACT pass/fail each
  - Full-suite acceptance run only if the packet-close ceremony requires it: `cargo xtask test --summary --workspace` (guest-freshness-gated entry point per CLAUDE.md) - FACT pass/fail
- Exit condition: re-blessed fixtures green, clippy clean, all ACs PASS, no assertion weakened anywhere in the packet.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | Parser retention + transport; wall-width/percent-transport ingestion residual |
| Step 2 | M | Rename + blast radius; survey dispatch mandatory before authoring |
| Step 3 | S | Pure-function resolver + unit matrix |
| Step 4 | M | Classic resolver + classic final-infill-boundary gap residual formula |
| Step 5 | S | Arachne resolver swap; no algorithm change |
| Step 6 | S | Rectilinear resolver swap |
| Step 7 | S | Gyroid resolver swap |
| Step 8 | S | Lightning resolver swap |
| Step 9 | M | Classic overlap + one-wall-top |
| Step 10 | S | Part-level width metadata allowlist + regression |
| Step 11 | M | Matrix suite + deviation/task crosswalk |
| Step 12 | S | ADR-0043 and ADR-0014 amendment records |
| Step 13 | M | Re-bless + acceptance gates |

Aggregate M; no step is L. Steps 6-8 are separate S steps (rather than one L) to keep each within the 3-edit cap.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS (including the percent round-trip AC and the precedence-matrix AC).
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- Check TASK-212b only after Step 10's loader allowlist regression passes; the task is not satisfied by a documentation checkbox alone.
- Reconcile reopened/superseded status transitions: packet 184's infill-boundary and wall-width/percent-transport residuals close here; `D-152-TOP-AREA-SOURCE` stays open; `Arachne wall-width ADR amendment` and `guest-freshness ADR amendment` record the ADR changes; `DEV-102` is created for deferred flow-ratio controls.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk: percent transport now live — downstream consumers that silently coerce `Percent` to `Float` would surface as new parity gaps; the matrix suite is the guard.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
