---
status: draft
packet: fuzzy-skin-gate-and-mode-keys
task_ids: []
backlog_source: docs/specs/orca-feature-gap/issues/14-author-packet-p07-others-fuzzy-skin-fuzzy-skin.md (wayfinder map: Close the OrcaSlicer FFF feature gap — packet P07; re-authored under the map's Authoring rules 1–6 and split 259a/259b, 210a/210b precedent)
context_cost_estimate: M
---

# Packet Contract: fuzzy-skin-gate-and-mode-keys

## Goal

Make OrcaSlicer's three fuzzy-skin *control* keys drive real decision points in the `fuzzy-skin` module. `fuzzy_skin` replaces the PnP-invented `apply_to_all` boolean with canonical's six-value loop-selection gate (`should_fuzzify`), deciding per wall loop whether it is fuzzified from the loop's contour/hole nature and its perimeter index. `fuzzy_skin_first_layer` gates layer 0. `fuzzy_skin_mode` selects between displacing the point (`displacement`), widening the extrusion without moving it (`extrusion`), and both (`combined`) — a switch canonical offers only on its Arachne path, which this port applies uniformly because `apply_fuzzy_skin` already owns the per-vertex width output. The packet also introduces the hole-loop distinction the gate needs, which the IR does not carry today.

## Scope Boundaries

The packet touches the `fuzzy-skin` core module (`modules/core-modules/fuzzy-skin/{fuzzy-skin.toml,src/lib.rs,tests/**}`), the `LoopType` enum and its schema version in `crates/slicer-ir/src/slice_ir.rs` together with its WIT mirror `enum wall-loop-type` in `crates/slicer-schema/wit/deps/ir-types.wit`, the perimeter generators that construct `WallLoop`s (`modules/core-modules/classic-perimeters/src/lib.rs`, `modules/core-modules/arachne-perimeters/src/lib.rs`), and one integration arm each in `slicer-scheduler` (bounds/enum enforcement) and `slicer-runtime` (CONFIG_BLOCK reachability). It does **not** touch `ORCA_CONFIG_PADDING` (`crates/slicer-gcode/src/serialize.rs`) — the previous revision's `fuzzy_skin` padding-value correction is deleted from scope under map Authoring rule 2. It does not implement coherent noise (`fuzzy_skin_noise_type`, `_octaves`, `_persistence`, `_scale` are packet **259b**'s scope), does not port canonical's painted-region promotion (`PrintApply.cpp` `generate_print_object_regions`, `apply_fuzzy_skin_segmentation`), and does not change `fuzzy_skin_thickness` / `fuzzy_skin_point_distance`, which are already live.

## Prerequisites and Blockers

- Depends on: wayfinder ticket 06 (packet numbering — 259a allocated by the approved 259a/259b split this session, re-derived from disk at authoring); ticket 103 (fuzzy-skin rename — resolved; the module already carries the Orca names `fuzzy_skin_thickness` / `fuzzy_skin_point_distance`).
- Unblocks: packet **259b**, which extracts this packet's gate and mode logic into a shared core crate that all six noise-generator modules reuse. 259b must land after this packet.
- Ordering, not gating: packets 253–258 precede this packet in the queue and touch different modules.
- **Activation blocker: one `[BLOCK]` in `design.md` (BLOCK-1 — `LoopType::Hole` is *both* an IR schema change and a WIT interface change; `enum wall-loop-type` in `crates/slicer-schema/wit/deps/ir-types.wit` mirrors `LoopType`, verified at authoring).** The packet stays `draft` and must not be activated until that block is resolved by the user or the architecture owner.

## Acceptance Criteria

- **AC-1. Given** the `fuzzy-skin` module manifest, **when** its `[config.schema]` is parsed, **then** it declares `fuzzy_skin` (`type = "enum"`, `values = ["none", "external", "hole", "all", "allwalls", "disabled_fuzzy"]`, `default = "disabled_fuzzy"`), `fuzzy_skin_first_layer` (`type = "bool"`, `default = false`), and `fuzzy_skin_mode` (`type = "enum"`, `values = ["displacement", "extrusion", "combined"]`, `default = "displacement"`) — each with a `display` name, `group = "Fuzzy Skin"`, and a `description` naming its canonical consumer function; and the PnP-invented `apply_to_all` key is **removed** from the manifest, because `fuzzy_skin = "allwalls"` is its canonical equivalent. | `cargo test -p fuzzy-skin --test fuzzy_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-2. Given** a region carrying an outer contour loop (`perimeter_index` 0), an inner contour loop (`perimeter_index` 1), and a hole loop, **when** `FuzzySkinModule::run_wall_postprocess` executes at layer 1, **then** the set of perturbed loops is exactly canonical `should_fuzzify`'s: `"disabled_fuzzy"` and `"none"` perturb nothing; `"external"` perturbs only the outer contour; `"hole"` perturbs only the hole loop at `perimeter_index` 0; `"all"` perturbs the outer contour and the hole loop; `"allwalls"` perturbs every loop including `perimeter_index` 1. Every non-perturbed loop is byte-identical to its input. | `cargo test -p fuzzy-skin --test fuzzy_skin_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-3. Given** `fuzzy_skin = "external"`, **when** `run_wall_postprocess` executes, **then** with `fuzzy_skin_first_layer` absent (default `false`) every loop at layer 0 passes through byte-identical while the outer loop at layer 1 is perturbed; and with `fuzzy_skin_first_layer = true` the outer loop at layer 0 is perturbed as well (canonical `should_fuzzify` returns false when `layer_id <= 0 && !fuzzy_first_layer`). | `cargo test -p fuzzy-skin --test fuzzy_skin_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-4. Given** `fuzzy_skin = "external"` and a fixed seed, **when** `run_wall_postprocess` runs once per mode over the identical outer loop, **then** the three modes are distinguishable exactly as canonical `fuzzy_extrusion_line` defines them: under `"displacement"` (default) the emitted point coordinates differ from the input and every emitted `width_profile` entry equals its input width; under `"extrusion"` every emitted point coordinate is **unchanged** from the input while at least one `width_profile` entry differs, each perturbed width equal to `max(input_width + r + 0.01, 0.01)` for that vertex's noise sample `r`; under `"combined"` both the coordinates and the widths differ, with the perpendicular offset equal to `(perturbed_width - input_width) / 2`. | `cargo test -p fuzzy-skin --test fuzzy_skin_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-5. Given** a closed wall loop whose vertices wind clockwise (a hole) and one that winds counter-clockwise (a contour), **when** the perimeter generators emit them, **then** each `WallLoop` carries `loop_type` distinguishing the two, and `fuzzy-skin` reads that field rather than re-deriving it — asserted by constructing both windings through `classic-perimeters` and checking the emitted `loop_type`, and separately by `arachne-perimeters`. | `cargo test -p slicer-ir --test loop_type_hole_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-6. Given** the scheduler's config bounds index loaded from the real `fuzzy-skin.toml`, **when** `fuzzy_skin = "bogus"` or `fuzzy_skin_mode = "bogus"` is resolved, **then** each is rejected with the enum `TypeMismatch` error naming the key and its legal values, and each legal value of both enums resolves; `fuzzy_skin_first_layer = 3` is rejected with `TypeMismatch`. | `cargo test -p slicer-scheduler --test scheduler_integration config_bounds_enforcement 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-7. Given** a slice run whose resolved config carries an explicit `fuzzy_skin = "external"`, **when** the G-code CONFIG_BLOCK is emitted, **then** the line `; fuzzy_skin = external` appears exactly once (the pre-existing padding entry is dedup-suppressed by the emitted-key path in `serialize_config_block`), and the packet's diff to `crates/slicer-gcode/src/serialize.rs` is empty. | `cargo test -p slicer-runtime --test integration gcode_header_thumbnail_config_blocks 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-8. Given** `cargo xtask gen-config-docs` has run, **then** `docs/15_config_keys_reference.md`'s generated module-key table carries `fuzzy_skin`, `fuzzy_skin_first_layer`, and `fuzzy_skin_mode` with owner `fuzzy-skin` exactly once each, carries no `apply_to_all` row, and the generated deviations block has the same number of data rows as immediately before the packet's manifest edits (re-derive that number from disk at implementation time). | `cargo xtask gen-config-docs --check && [ "$(rg -c '^\| `fuzzy_skin_mode`' docs/15_config_keys_reference.md)" = "1" ] && ! rg -q '^\| `apply_to_all`' docs/15_config_keys_reference.md; echo "exit=$?"`

## Negative Test Cases

- **AC-N1. Given** default configuration (`fuzzy_skin` absent, therefore `disabled_fuzzy`), **when** a slice runs over the square fixture, **then** no wall loop is perturbed anywhere and the emitted G-code is byte-identical to the pre-packet baseline. This is an *additional* criterion; it is never the sole evidence for any key. | `cargo test -p slicer-runtime --test e2e slice_end_to_end 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N2. Given** the `ORCA_CONFIG_PADDING` table (`crates/slicer-gcode/src/serialize.rs`), **when** the packet's diff is inspected, **then** it contains zero added, removed, or edited lines — in particular the previous revision's `("fuzzy_skin", "none")` → `"disabled_fuzzy"` correction is **not** a deliverable (map Authoring rule 2). | `git diff --unified=0 -- crates/slicer-gcode/src/serialize.rs | grep -cE "^[+-][^+-]"` (expect `0`)
- **AC-N3. Given** the removal of `apply_to_all`, **when** a config still supplies it, **then** resolution does not silently ignore it: the key is either rejected as unknown-to-the-module or mapped to `fuzzy_skin = "allwalls"` with a deprecation diagnostic — whichever the implementer chooses, the choice is asserted, and a run configured the old way never silently stops fuzzing. | `cargo test -p fuzzy-skin --test fuzzy_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N4. Given** `fuzzy_skin_mode = "extrusion"`, **when** a loop is perturbed, **then** the emitted point count equals the input point count for the perturbed span — extrusion mode must not insert the resampled subdivision points that displacement mode inserts, because it does not move anything. | `cargo test -p fuzzy-skin --test fuzzy_skin_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p fuzzy-skin --test fuzzy_skin_tdd` and `cargo test -p fuzzy-skin --test fuzzy_config_schema_tdd` (primary contracts), then `cargo xtask build-guests --check; echo "exit=$?"` — `fuzzy-skin`, `classic-perimeters`, `arachne-perimeters`, and every guest depending on `slicer-ir` are fingerprint-affected by the `LoopType` change, so this must return exit 0 before closure.

## Authoritative Docs

- `docs/03_wit_and_manifest.md` — `[config.schema]` type-table contract.
- `docs/04_host_scheduler.md` — config resolution and bounds enforcement.
- `docs/08_coordinate_system.md` — 1 unit = 100 nm; the displacement arithmetic is geometry.
- `docs/21_data_defaults_and_fixtures.md` — struct-literal churn gate, triggered by the `LoopType` variant addition.
- `docs/15_config_keys_reference.md` — generated.

## Doc Impact Statement (Required)

- `docs/15_config_keys_reference.md` — generated; gains three rows (owner `fuzzy-skin`) and **loses** the `apply_to_all` row. Verification: the AC-8 command.
- `docs/03_wit_and_manifest.md` — if the IR schema version is bumped for `LoopType::Hole`, its `min-ir-schema` / `max-ir-schema` guidance and any enumerated `LoopType` list must be updated in the same step. Verification: `rg -q 'LoopType' docs/03_wit_and_manifest.md` and a manual read of the surrounding paragraph by the implementing step.
- `docs/DEVIATION_LOG.md` — gains one row for the uniform-mode divergence (DIV-1 in `design.md`): canonical applies `fuzzy_skin_mode` only on its Arachne path, this port applies it on both. Re-derive the next free `D-` ID from the log at the moment of writing; never freeze it.
- `docs/07_implementation_status.md` — no new module; no inventory change.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Feature/FuzzySkin/FuzzySkin.cpp` — `should_fuzzify` (the whole gate: the `None`/`Disabled_fuzzy` early false, the `layer_id <= 0 && !fuzzy_first_layer` false, the contour rule `loop_idx == 0 && type != Hole`, the `AllWalls` always-true, and the hole rule for `Hole`/`All`/`AllWalls`), `fuzzy_extrusion_line` (the `switch (cfg.mode)` over Displacement / Extrusion / Combined, including the `max(p1.w + r + 0.01, 0.01)` width and the `(rad - p1.w)/2` combined offset), `fuzzy_polyline` (the classic path, which has **no** mode switch — the divergence this packet records), `group_region_by_fuzzify` (the single site that reads every fuzzy-skin key into `FuzzySkinConfig`).
- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.hpp` — `struct FuzzySkinConfig` (the carrier: type, thickness, point_distance, fuzzy_first_layer, mode, noise fields, layer_id).
- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp` — `traverse_loops` and `traverse_extrusions` (the two `apply_fuzzy_skin` call sites, and where `is_contour` comes from).
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — `PrintConfigDef::init_fff_params` plus `s_keys_map_FuzzySkinType` and `s_keys_map_FuzzySkinMode`: the six-value `fuzzy_skin` enum with default `Disabled_fuzzy`, `fuzzy_skin_first_layer` default false, and the three-value `fuzzy_skin_mode` with default `Displacement`.

Note: in this clone the checkout is the sibling `..\pinch_n_print_cli\OrcaSlicerDocumented` (pinned by wayfinder ticket 08's ledger note) — workers must resolve `OrcaSlicerDocumented/` against that absolute sibling path.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
