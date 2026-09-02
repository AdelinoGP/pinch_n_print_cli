# Requirements: fuzzy-skin-gate-and-mode-keys

## Packet Metadata

- **Packet directory:** `docs/spec_packets/259a-fuzzy-skin-gate-and-mode-keys/`
- **Slug:** `fuzzy-skin-gate-and-mode-keys`
- **Status:** `draft` (blocked — see `design.md` BLOCK-1)
- **Task IDs:** none (queue packet — `task_ids: []`)
- **Backlog source:** wayfinder ticket 14, map packet P07
- **Tier:** **B** — re-derived. Builds decision points inside one module plus one IR enum variant and its two producers. Above Tier A (which map Authoring rule 1 now forbids for any packet that builds a decision point), below Tier C (no new module crate).
- **Re-authoring note:** this packet plus `259b-fuzzy-skin-noise-modules` replace the single `259-fuzzy-skin-keys` draft, under map Authoring rules 1–6 and with explicit user approval for the split (210a/210b, 262a/262b precedent).

## Problem Statement

`fuzzy-skin` today reads exactly three config keys — `fuzzy_skin_thickness`, `fuzzy_skin_point_distance`, and the PnP-invented `apply_to_all` — verified against `FuzzySkinModule`'s `LayerModule::from_config` impl and `fuzzy-skin.toml`. **None of ticket 14's seven Orca keys is live.** Loop selection is a PnP heuristic (`self.apply_to_all || wall.feature_flags.iter().any(|f| f.fuzzy_skin)`, restricted to `LoopType::Outer`), which is neither canonical's gate nor expressible in canonical's vocabulary.

The prior revision of this packet declared five of the seven keys "with-gap". Map Authoring rule 1 prohibits that disposition. This packet takes the three keys whose decision points can be built inside the existing module and builds them; packet 259b takes the four noise keys.

One canonical input has no port representation: `should_fuzzify` needs to know whether a loop is a **contour or a hole**, and this tree's `LoopType` enum (`Outer`, `Inner`, `ThinWall`, `NonPlanarShell`, `GapFill`) and `WallBoundaryType` enum (`ExteriorSurface`, `MaterialBoundary`, `Interior`) both fail to carry it — verified against `crates/slicer-ir/src/slice_ir.rs`. Supplying it is an IR change and is the packet's one blocker.

## Key Disposition Table

Classification per the map's Authoring rules: **(a)** live behaviour-changing decision point already in tree; **(b)** decision point this packet builds; **(c)** returned to queue; **(d)** dead-in-canonical.

| Key | Class | Owner | Decision point this packet builds | Non-default AC |
| --- | --- | --- | --- | --- |
| `fuzzy_skin` | **(b)** | `fuzzy-skin` | canonical `should_fuzzify`'s six-value loop-selection gate, replacing the `apply_to_all` heuristic | AC-2 |
| `fuzzy_skin_first_layer` | **(b)** | `fuzzy-skin` | the `layer_id <= 0` suppression and its override | AC-3 |
| `fuzzy_skin_mode` | **(b)** | `fuzzy-skin` | displacement vs. extrusion-width vs. combined, on the per-vertex width output `apply_fuzzy_skin` already produces | AC-4, AC-N4 |

Counts: **(a) 0 · (b) 3 · (c) 0 · (d) 0** for this packet's three keys. Zero declaration-only keys (map preflight gate (a)); every key has at least one AC asserting a behaviour change at a non-default value (map preflight gate (b)).

Ticket 14's other four keys (`fuzzy_skin_noise_type`, `fuzzy_skin_octaves`, `fuzzy_skin_persistence`, `fuzzy_skin_scale`) are **not returned to the queue** — they move to packet `259b-fuzzy-skin-noise-modules`, which builds them. `fuzzy_skin_thickness` and `fuzzy_skin_point_distance` are already live and are not claimed as coverage by either packet.

## Returned to Queue — unimplemented

**None from this packet's three keys.**

One canonical *behaviour* is returned rather than built: **painted-region fuzzy promotion**. Canonical `PrintApply.cpp` `generate_print_object_regions` forces `fuzzy_skin = All` for brush-painted regions, segmented by `fuzzy_skin_segmentation_by_painting` (`MultiMaterialSegmentation.cpp`) and applied by `apply_fuzzy_skin_segmentation` (`PrintObjectSlice.cpp`). This port has no fuzzy-skin paint semantic. Returned to the queue as *unimplemented, needs a fuzzy-skin paint semantic and its segmentation pass*. It is a behaviour, not one of ticket 14's keys, so it does not change any key count.

## Ruled Dead-in-Canonical

**None.** All three keys have a live read site inside OrcaSlicer's slicing pipeline: every fuzzy-skin key is read in `group_region_by_fuzzify` (`src/libslic3r/Feature/FuzzySkin/FuzzySkin.cpp`), which builds the `FuzzySkinConfig` consumed by `should_fuzzify`, `fuzzy_polyline`, and `fuzzy_extrusion_line`. `fuzzy_skin` is additionally read in `PrintObject.cpp` `region_config_from_model_volume` (forced to `None` for degenerate thickness/point-distance) and `PrintApply.cpp`. None of the three is confined to `ConfigManipulation.cpp`, GUI tooltips, preset plumbing, or an `IGNORE` set.

## In Scope

1. **The `fuzzy_skin` gate.** Port canonical `should_fuzzify(cfg, layer_id, loop_idx, is_contour)` into `fuzzy-skin`: `None` / `Disabled_fuzzy` → never; `layer_id <= 0 && !fuzzy_first_layer` → never; contours fuzzify when `loop_idx == 0 && type != Hole`, or always under `AllWalls`; holes fuzzify under `Hole` / `All` / `AllWalls`, and only at `loop_idx == 0` unless `AllWalls`. `loop_idx` maps to `WallLoop::perimeter_index`; `is_contour` comes from the new `LoopType` distinction.
2. **The `fuzzy_skin_first_layer` gate**, as the `layer_id <= 0` clause of the same function, driven by `run_wall_postprocess`'s `layer_index` argument.
3. **The `fuzzy_skin_mode` switch** inside `apply_fuzzy_skin`. `Displacement` keeps today's behaviour (perpendicular offset by `noise * thickness`, width unchanged). `Extrusion` leaves every point where it is and sets that vertex's width to `max(input_width + r + 0.01, 0.01)`. `Combined` does both, offsetting by `(perturbed_width - input_width) / 2`. The function already returns `out_widths`, so the carrier exists.
4. **Hole-loop identification in the IR.** Add the variant the gate needs to `LoopType` (`crates/slicer-ir/src/slice_ir.rs`) and populate it in the two perimeter generators. **See `design.md` BLOCK-1 — this is an IR schema change and is the packet's activation blocker.**
5. **Manifest ownership** of the three keys on `fuzzy-skin.toml` with canonical types/defaults/values and a `description` naming the canonical consumer; **removal** of `apply_to_all`, whose canonical equivalent is `fuzzy_skin = "allwalls"`.
6. **Bounds/enum enforcement** arms in `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` and CONFIG_BLOCK reachability arms in the `slicer-runtime` integration binary.

## Out of Scope

- **The four noise keys.** `fuzzy_skin_noise_type`, `fuzzy_skin_octaves`, `fuzzy_skin_persistence`, `fuzzy_skin_scale` belong to packet 259b. This packet leaves the single `rng.next_f32()` noise call in `apply_fuzzy_skin` exactly as it is; 259b replaces it.
- **Painted-region promotion** — returned to the queue above.
- **`ORCA_CONFIG_PADDING`** — untouched (AC-N2, map Authoring rule 2).
- **`fuzzy_skin_thickness` / `fuzzy_skin_point_distance`** — already live; not re-declared, not claimed as coverage.
- **Canonical's degenerate-value forcing** (`region_config_from_model_volume` sets `fuzzy_skin = None` when `point_distance < 0.01` or `thickness < 0.001`). The port's `apply_fuzzy_skin` already returns the input unchanged for non-finite or non-positive thickness/point-distance, which is behaviourally equivalent at the point of use. Recorded in `design.md` DIV-2, not re-implemented at config-resolution time.

## Authoritative Docs

- `docs/03_wit_and_manifest.md` — `[config.schema]` contract; `min-ir-schema` / `max-ir-schema` if the schema version moves.
- `docs/04_host_scheduler.md` — bounds enforcement and enum rejection.
- `docs/08_coordinate_system.md` — 1 unit = 100 nm.
- `docs/21_data_defaults_and_fixtures.md` — the struct-literal churn gate, triggered by the `LoopType` change.
- `docs/15_config_keys_reference.md` — generated.

## Parity Evidence Standard

A key counts as covered only when a non-default value changes emitted geometry or emitted G-code, proven by a named test. Default-path identity (AC-N1) is an additional guard and is never the sole evidence for any key. Canonical is cited by file + function; in-tree by crate-qualified path + symbol.

## Per-Key Canonical Evidence

Established by delegated reads of the sibling `OrcaSlicerDocumented` checkout during authoring.

- **`fuzzy_skin`** — `PrintConfig.cpp` `PrintConfigDef::init_fff_params` with `s_keys_map_FuzzySkinType`: enum `FuzzySkinType` = `None` ("none" / "Painted only"), `External` ("external" / "Contour"), `Hole`, `All` ("Contour and hole"), `AllWalls`, `Disabled_fuzzy` ("Disabled"), **default `Disabled_fuzzy`**. It is an enum, not a bool — the snapshot in `docs/ORCA_CONFIG_REFERENCE.md` is wrong here. Consumed by `should_fuzzify`.
- **`fuzzy_skin_first_layer`** — bool, default false. Consumed by `should_fuzzify`'s `layer_id <= 0 && !fuzzy_first_layer` clause.
- **`fuzzy_skin_mode`** — `s_keys_map_FuzzySkinMode`: `Displacement` (default), `Extrusion`, `Combined`. Switched on **only** in `fuzzy_extrusion_line` (the Arachne junction path); `fuzzy_polyline` (the classic path) has no mode switch and always displaces, which is why Orca's tooltip says "Works only with Arachne!". Semantics: `Displacement` moves the point perpendicular by `r` and keeps width `p1.w`; `Extrusion` leaves the point unmoved and sets junction width to `max(p1.w + r + 0.01, 0.01)`; `Combined` sets `rad = max(p1.w + r + 0.01, 0.01)` and offsets perpendicular by `(rad - p1.w) / 2`.
- **`should_fuzzify(cfg, layer_id, loop_idx, is_contour)`** — returns false for `None` / `Disabled_fuzzy`; false when `layer_id <= 0` and `!fuzzy_first_layer`; contours fuzzify when `loop_idx == 0 && type != Hole`, or always for `AllWalls`; holes fuzzify for `Hole` / `All` / `AllWalls`, and only at `loop_idx == 0` unless `AllWalls`.
- **Single read site** — `group_region_by_fuzzify` (`Feature/FuzzySkin/FuzzySkin.cpp`) is the only place any fuzzy-skin key is read into `FuzzySkinConfig` (`PerimeterGenerator.hpp`). Call sites are `traverse_loops` (classic) and `traverse_extrusions` (Arachne) in `PerimeterGenerator.cpp`, and `is_contour` there comes from the loop's own contour flag / `extrusion->inset_idx`.

## In-Tree Grounding (verified at authoring, 2026-09-01)

- `FuzzySkinModule` (`modules/core-modules/fuzzy-skin/src/lib.rs`) has exactly three fields — `fuzzy_skin_thickness`, `fuzzy_skin_point_distance`, `apply_to_all` — populated in its `LayerModule::from_config` impl under `#[slicer_module]`. `fuzzy-skin.toml` declares exactly those three keys, `holds = []`, `[stage] Layer::PerimetersPostProcess`, reads and writes `PerimeterIR`.
- `run_wall_postprocess` restricts perturbation to `wall.loop_type != LoopType::Outer` → pass-through, then gates on `self.apply_to_all || wall.feature_flags.iter().any(|f| f.fuzzy_skin)`. This is the heuristic `fuzzy_skin` replaces.
- `apply_fuzzy_skin` already returns `(Vec<Point3WithWidth>, Vec<WallFeatureFlags>, Vec<f32>)` — the third element is the per-vertex width profile. **The `extrusion` and `combined` modes need no new carrier.**
- `Rng::next_f32` returns a value in `[-1.0, 1.0]`, matching canonical `UniformNoise::GetValue`'s `random_value() * 2 - 1`. The displacement is `rng.next_f32() * fuzzy_skin_thickness`, matching canonical's `GetValue(...) * cfg.thickness`. No correction is needed here.
- `LoopType` (`crates/slicer-ir/src/slice_ir.rs`) has exactly `Outer`, `Inner`, `ThinWall`, `NonPlanarShell`, `GapFill` — **no `Hole`**. `WallLoop::boundary_type` is `WallBoundaryType` = `ExteriorSurface` / `MaterialBoundary { segments }` / `Interior` — also no contour/hole distinction.
- The scheduler's integration test target is **`scheduler_integration`** (`[[test]] name = "scheduler_integration"`, `crates/slicer-scheduler/Cargo.toml`); its aggregator `tests/integration/main.rs` uses `mod <name>;` declarations. `slicer-runtime`'s `contract`, `e2e`, and `integration` targets exist (`integration` declared; `contract` / `e2e` auto-discovered from `tests/<name>/main.rs`).
- Existing `fuzzy-skin` test files: `closed_loop_tdd.rs`, `fuzzy_skin_tdd.rs`, `slicer_module_binding_tdd.rs`. `fuzzy_config_schema_tdd.rs` is **net-new** in this packet.

## Acceptance Summary

Authoritative Given/When/Then text lives in `packet.spec.md`. IDs only here.

| AC | Subject | Key(s) covered |
| --- | --- | --- |
| AC-1 | manifest schema for the three keys; `apply_to_all` removed | all three |
| AC-2 | the six-value loop-selection gate | `fuzzy_skin` |
| AC-3 | layer-0 suppression and its override | `fuzzy_skin_first_layer` |
| AC-4 | the three modes are geometrically distinguishable | `fuzzy_skin_mode` |
| AC-5 | `LoopType` carries the contour/hole distinction from both generators | `fuzzy_skin` (enabler) |
| AC-6 | bounds/enum rejection | all three |
| AC-7 | CONFIG_BLOCK emits the explicit value once | `fuzzy_skin` |
| AC-8 | generated config-keys doc gains three rows, loses `apply_to_all` | all three |
| AC-N1 | default path byte-identical (additional guard only) | all three |
| AC-N2 | zero `ORCA_CONFIG_PADDING` diff lines | — |
| AC-N3 | removed `apply_to_all` does not silently stop fuzzing | `fuzzy_skin` |
| AC-N4 | extrusion mode inserts no subdivision points | `fuzzy_skin_mode` |

## Verification Matrix

| AC | Command |
| --- | --- |
| AC-1, AC-N3 | `cargo test -p fuzzy-skin --test fuzzy_config_schema_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` |
| AC-2, AC-3, AC-4, AC-N4 | `cargo test -p fuzzy-skin --test fuzzy_skin_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` |
| AC-5 | `cargo test -p slicer-ir --test loop_type_hole_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` |
| AC-6 | `cargo test -p slicer-scheduler --test scheduler_integration config_bounds_enforcement 2>&1 \| tee target/test-output.log \| grep -E "^test result"` |
| AC-7 | `cargo test -p slicer-runtime --test integration gcode_header_thumbnail_config_blocks 2>&1 \| tee target/test-output.log \| grep -E "^test result"` |
| AC-8 | `cargo xtask gen-config-docs --check && [ "$(rg -c '^\| \`fuzzy_skin_mode\`' docs/15_config_keys_reference.md)" = "1" ] && ! rg -q '^\| \`apply_to_all\`' docs/15_config_keys_reference.md; echo "exit=$?"` |
| AC-N1 | `cargo test -p slicer-runtime --test e2e slice_end_to_end 2>&1 \| tee target/test-output.log \| grep -E "^test result"` |
| AC-N2 | `git diff --unified=0 -- crates/slicer-gcode/src/serialize.rs \| grep -cE "^[+-][^+-]"` (expect `0`) |
| Gates | `cargo check --workspace --all-targets`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo xtask check-literals`; `cargo xtask build-guests --check; echo "exit=$?"` |

## Step Completion Expectations

- The `LoopType` variant and both perimeter generators must move in the **same** step. A variant with no producer makes AC-5 vacuous and leaves the gate unable to distinguish holes; a producer change without the variant will not compile.
- Adding a `LoopType` variant is a match-exhaustiveness blast radius across every crate matching on it, plus the struct-literal churn gate on `WallLoop`. The step that adds the variant owns both.
- Removing `apply_to_all` and adding `fuzzy_skin` must land together, or the module briefly has no way to be switched on at all.
- `cargo xtask build-guests --check` must exit 0 before closure: the `LoopType` change is a `slicer-ir` change, which is a fingerprint input for **every** guest.

## Context Discipline Notes

- Never load `OrcaSlicerDocumented/` directly.
- `crates/slicer-ir/src/slice_ir.rs` is very long — locate `LoopType` and `WallLoop` by symbol and range-read; never open the file.
- `docs/15_config_keys_reference.md` is generated; verify with the AC-8 command, never read to author.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Feature/FuzzySkin/FuzzySkin.cpp` — `should_fuzzify`, `fuzzy_extrusion_line` (the mode switch), `fuzzy_polyline`, `group_region_by_fuzzify`.
- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.hpp` — `struct FuzzySkinConfig`.
- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp` — `traverse_loops`, `traverse_extrusions` (the `apply_fuzzy_skin` call sites and the origin of `is_contour`).
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — `PrintConfigDef::init_fff_params`, `s_keys_map_FuzzySkinType`, `s_keys_map_FuzzySkinMode`.

Note: in this clone the checkout is the sibling `..\pinch_n_print_cli\OrcaSlicerDocumented` (pinned by wayfinder ticket 08's ledger note) — workers must resolve `OrcaSlicerDocumented/` against that absolute sibling path.
