# Requirements: wipe-tower-bed-exclude-area

## Packet Metadata

- **Packet directory:** `docs/spec_packets/256-wipe-tower-bed-exclude-area/`
- **Slug:** `wipe-tower-bed-exclude-area`
- **Status:** `draft`
- **Task IDs:** none (queue packet — `task_ids: []`, precedent packets 234a, 253–264)
- **Backlog source:** wayfinder ticket 11 (`docs/specs/orca-feature-gap/issues/11-author-packet-p04-printer-machine-print-volume-wipe-tower.md`), map `docs/specs/orca-feature-gap/map.md` packet P04
- **Tier:** **C** — re-derived. The prior revision was Tier A ("owner and decision point both exist; only the declaration and the check are missing"). That was true only for the tower-corner reading of the key. Building canonical's actual feature means a **new granular module at a seam this tree does not use yet**, which is Tier C by ticket 04's rubric. See `design.md` § Tier Derivation.
- **Re-authoring note:** this directory is overwritten in place (number and slug retained) with explicit user approval, under map Authoring rules 1–6, and in response to the ⚠ correction on the map's ticket-11 entry. **Second re-author (2026-09-05, user request):** the map's ticket-40 entry returned `start_end_points` to the queue as unimplemented, blocked on `bed_exclude_area` being live; this revision folds it in as the packet's second key.

## Problem Statement

OrcaSlicer's `bed_exclude_area` marks a region of the bed the printer cannot use — a filament cutter zone, a purge chute, a clip. `PrintConfig.cpp::get_bed_excluded_area` turns every configured point into **one** polygon, and `Print.cpp::Print::validate` (via `layered_print_cleareance_valid` / `sequential_print_clearance_valid`) intersects each model volume's 2D convex hull with it and **fails the print** with a collision-risk message. It is a validation feature, not a geometry feature.

Pinch 'n Print has neither the key nor the seam. `bed_exclude_area` has zero occurrences in `crates/`, `modules/`, `xtask/` and `resources/`. More importantly, this port has **no pre-slice geometry validation of any kind**: `slicer_scheduler::validation::validate_startup_dag` validates the module DAG and nothing geometric; the only pre-execution gates `slicer_runtime::run::run_slice_with_collector` applies are `ConfigBoundsIndex::check` and `slicer_scheduler::config_resolution::validate_support_layer_heights`. No error variant for "object outside / inside a forbidden area" exists anywhere.

The prior revision of this packet declared the key on the `wipe-tower` manifest and checked the **tower rectangle** against it, recording the object-hull check as a gap. Under map Authoring rule 1 that is defensible (the tower check is a real decision point), but it delivers a different feature from the one canonical attaches to the key, and the map's ticket-11 entry carries a ⚠ correction saying so: *"canonical's feature is object-footprint validation (`Print::validate`) — implement it at the port's validation seam."*

This revision builds that seam. It does **not** build it as a host-side special case: a hardcoded geometry check inside `run_slice_with_collector` would satisfy the letter of the correction while contradicting the project's modular-pipeline and community-extensibility goals (`docs/00_project_overview.md`). Pre-slice validation becomes a **module**, at a stage the schema already defines and no core module occupies.

**The second key.** Canonical's `bed_exclude_area` has a second live consumer: `GCode.cpp::get_path_of_change_filament` computes the three `travel_point_*` placeholders that a user's `change_filament_gcode` template can reference — the travel path from the cutter area to the garbage can, steered around objects. The key that drives it is `start_end_points` (`coPoints`, default `(30,-3),(54,245)`, `readonly`, `comDevelop`), returned to the queue by wayfinder ticket 40 because the path computation needs `bed_exclude_area` live and a transport to the `change_filament_gcode` substitution seam. This revision builds both. The transport is constrained by ADR-0050 §2: the resolvable placeholder set is **exactly** `machine-gcode-emit`'s manifest-declared keys plus the alias table — a computed-value insertion into the substitution lookup would be a third source. So the host computes the path (a pure function in `crates/slicer-runtime/src/travel_path.rs`, reading `start_end_points` + `bed_exclude_area` from `config_source` and per-object XY bounds from `mesh_ir.objects`), injects the six `travel_point_*` values into `config_source` (the `slice_has_paint` host-injected-key pattern, gated on the multi-tool predicate), and `machine-gcode-emit` declares them in its manifest — the existing `config.keys()` sweep then resolves them with **no module code change**. The decision point is the host computation; the architecture's only channel for computed placeholder values is a manifest-declared config key, and the host is the only writer of `config_source`.

## Key Disposition Table

Classification per the map's Authoring rules: **(a)** live behaviour-changing decision point already in tree; **(b)** decision point this packet builds; **(c)** returned to queue; **(d)** dead-in-canonical.

| Key | Class | Owner | Decision point this packet builds | Non-default AC |
| --- | --- | --- | --- | --- |
| `bed_exclude_area` | **(b)** | `print-validator` (new module, primary) + `wipe-tower` (secondary) + `machine-gcode-emit` (travel-path key surface) | pre-slice object-vs-exclusion validation at `PrePass::MeshAnalysis`, fatal on collision; the wipe-tower footprint's own exclusion test at `PostPass::LayerFinalization`; the cutter-area corner of the filament-change travel path (host computation, `crates/slicer-runtime/src/travel_path.rs`) | AC-2, AC-4, AC-5, AC-7, AC-10 |
| `start_end_points` | **(b)** | `machine-gcode-emit` (declares the key + consumes the computed values) with the computation in `slicer-runtime` | the filament-change travel path (`get_path_of_change_filament` equivalent): the host computes `travel_point_1/2/3` from `start_end_points` + `bed_exclude_area` + per-object XY bounds and injects the six values into `config_source`; `machine-gcode-emit` declares them and its substitution resolves them (ADR-0050 §2 conformant) | AC-9, AC-10, AC-11 |

Counts: **(a) 0 · (b) 2 · (c) 0 · (d) 0.** Zero declaration-only keys (map preflight gate (a)); each key carries ACs asserting behaviour changes at non-default values (map preflight gate (b)) — `bed_exclude_area` has no default at all, and `start_end_points`'s non-default ACs (AC-9, AC-10) assert the computed path against canonical's algorithm.

## Returned to Queue — unimplemented

**No keys.** P04 is a two-key packet and both keys are implemented here.

Two canonical **consumers** of `bed_exclude_area` are deliberately not built, and are recorded here rather than silently dropped. Each needs a decision point this port does not have; none is a key of its own:

| Canonical consumer | Needs (missing feature) |
| --- | --- |
| `GCode/GCodeProcessor.cpp::apply_config` | a G-code viewer/analyzer state model. The port's post-process stage has no viewer copy of the bed. |
| `GCode/TimelapsePosPicker.cpp::construct_printable_area_by_printer` | a timelapse camera-park position picker (subtractive use: printable area minus exclusion). The port has no timelapse feature. |

`GCode.cpp::get_path_of_change_filament` was listed here in the prior revision; it is now **built** as the `start_end_points` decision point (see § In Scope, item 5).

## Ruled Dead-in-Canonical

**None.** `bed_exclude_area` has live read sites inside OrcaSlicer's slicing pipeline under `src/libslic3r/` — `PrintConfig.cpp::get_bed_excluded_area` and `Print.cpp::Print::validate` (plus the three consumers above). It is not confined to `ConfigManipulation.cpp`, GUI tooltips, preset plumbing, or an `IGNORE` / legacy-alias set.

## In Scope

1. **A new core module, `print-validator`**, at `PrePass::MeshAnalysis` — the earliest module-hostable stage, and the only one that precedes layer planning. It declares `bed_exclude_area` and `printable_area`, holds no claim, writes no IR, and commits nothing to the blackboard: it exists to reject a print, not to produce one. It is the intended home for the port's later pre-slice validation checks (`printable_height`, extruder clearance — P18/P19).
2. **The probe algorithm.** For each object: read `object-bounds` (`slicer:common/host-services`); skip the object when its XY rectangle does not overlap the exclusion polygon's bounding rectangle; otherwise walk a `1.0` mm grid over the intersection of the two rectangles, keep only points inside the exclusion polygon (even-odd test, boundary-inclusive — the same predicate `wipe-tower`'s `point_in_polygon` uses), and `raycast-z-down(object_id, x, y, start_z)` each one with `start_z` above the object's max Z. The first `Some(_)` is a collision → `ModuleError::fatal`, which the prepass executor converts to `PrepassExecutionError::FatalModule` and the pipeline to `PipelineError::Prepass`.
3. **The wipe-tower half.** `bed_exclude_area` is also declared on `wipe-tower.toml` and tested against the tower footprint inside the existing code-3 bed-bounds site in `run_finalization`. The tower is generated at `PostPass::LayerFinalization`, long after the validator has run, so this is a distinct decision point rather than a duplicate check.
4. **Registration surface.** Workspace member, integrated-module registry, `pnp-cli` passthrough features, and the core-module discovery count (a shared ledger fact — re-derived from disk, never frozen).
5. **The filament-change travel path (`start_end_points`).** The host computes canonical's `get_path_of_change_filament` (safe default `(54,0),(54,0),(54,245)`; cutter area from `bed_exclude_area[2] + 2`; per-object intervals `[min.x-2, max.x+2]` merged and sorted; available intervals over `[0, 255]`; the four-branch `new_path` selection; left-travel vs right-travel point construction) as a pure function in `crates/slicer-runtime/src/travel_path.rs`, reading `start_end_points` + `bed_exclude_area` from `config_source` and per-object XY bounds from `mesh_ir.objects`. `run_slice` injects the six `travel_point_1_x/y` … `travel_point_3_x/y` values into `config_source`, gated on the multi-tool predicate (≥ 2 distinct painted tool indices — the same predicate as the MMU wipe-tower auto-enable), following the `slice_has_paint` / `object_height:<id>` host-injected-key pattern. `machine-gcode-emit` declares `start_end_points`, `bed_exclude_area` and the six `travel_point_*` keys in its manifest — the existing `config.keys()` sweep resolves the placeholders with no module code change, preserving ADR-0050 §2's placeholder domain exactly. Single-tool prints are byte-identical.
6. **Degenerate-value semantics.** Absent, empty, single-point (canonical's own default), fewer than 3 vertices, or an odd float count all mean "nothing is excluded" — never a slice failure. Malformed printer config must not be a fatal error when canonical's own default is degenerate. For the travel path: `start_end_points` absent falls back to canonical's default `(30,-3),(54,245)`; `start_end_points` with ≠ 2 points or `bed_exclude_area` with ≠ 4 points returns the canonical safe-default path — never a failure.
7. **Generated docs**, plus one sentence in `docs/04_host_scheduler.md` recording that `PrePass::MeshAnalysis` now hosts a guest validator beside its host built-in.

## Out of Scope

- **Any WIT change.** The `mesh-analysis` `run` signature (`objects: list<object-id>`, `output`, `config`) is used as-is; `mesh-object-view` (which carries `vertices` / `triangles`) is passed only to `prepass-seam-planning` and `prepass-support-geometry`, and this packet does **not** extend `mesh-analysis` to take it. That constraint is what makes DIV-1 necessary and is the packet's single most consequential design choice. The `gcode-postprocess-module` world's `run(commands, output, config)` is likewise used as-is — the travel path is computed from config keys, not from a new host service.
- **A new `ResolvedConfig` field.** Both keys ride the `extensions` overflow bucket, which `ResolvedConfig::to_config_map` merges through unchanged into the module config map.
- **Canonical's convex-hull footprint** — DIV-1 in `design.md`.
- **The two returned consumers** above.
- **`printable_height` / `extruder_printable_area` / `extruder_printable_height` / `extruder_clearance_*`** — same Orca section, later packets (P18/P19). This packet gives them a home; it does not implement them.
- **`ORCA_CONFIG_PADDING`** — untouched (map Authoring rule 2).
- **Sequential (by-object) print clearance** — canonical's `sequential_print_clearance_valid` also enforces inter-object clearance under by-object sequencing; the port has no by-object mode.
- **A per-object footprint host service** — the `[FWD]` in `design.md`; tightening DIV-1 needs a WIT change and is a separate packet.

## Authoritative Docs

- `docs/04_host_scheduler.md` — stage order, prepass execution, and the claim-resolution rules the new manifest must satisfy.
- `docs/03_wit_and_manifest.md` — manifest contract, stage declaration, host-boundary access enforcement.
- `docs/01_system_architecture.md` § Claim System — the mechanism check in `design.md`.
- `docs/08_coordinate_system.md` — plain-mm boundary for the polygon and probe grid.
- `docs/15_config_keys_reference.md` — generated; regeneration plus grep verification only.

## Parity Evidence Standard

Per map ticket 02: canonical function-read plus described behaviour, pinned by invariant tests; ported OrcaSlicer test assertions acceptable with the attribution header; unverifiable behaviour surfaced to the human before any `DEVIATION_LOG.md` row. This packet consumes no human sign-off and files no deviation row. Its two port-level divergences are recorded in `design.md` § Divergences with rationale, per Authoring rule 4.

## Per-Key Canonical Evidence

| Key | Canonical def | Canonical consumers (file + function) | Behaviour in canonical | Disposition here |
| --- | --- | --- | --- | --- |
| `bed_exclude_area` | `coPoints`, `comAdvanced`, default `{ Vec2d(0, 0) }` (degenerate single point), no bounds | `PrintConfig.cpp::get_bed_excluded_area` (all points → one CCW polygon); `Print.cpp::Print::validate` → `layered_print_cleareance_valid` / `sequential_print_clearance_valid` (per-volume 2D convex hull ∩ polygon → fatal `"<object> is too close to exclusion area, there may be collisions when printing."`); `GCode.cpp::get_path_of_change_filament` (4-point cutter-area form); `GCodeProcessor.cpp::apply_config`; `TimelapsePosPicker.cpp::construct_printable_area_by_printer` | fatal pre-print validation of object footprints against a forbidden bed region; the wipe tower is never tested against it; the cutter-area corner feeds the filament-change travel path | **BUILT** — `print-validator` at `PrePass::MeshAnalysis` rejects an object occupying the excluded region (sampled probe, DIV-1), `wipe-tower` additionally rejects a tower footprint inside it (DIV-2), and `machine-gcode-emit` reads the 4-point cutter form for the travel path. Degenerate values exclude nothing, matching `get_bed_excluded_area`. Two consumers returned to the queue above. |
| `start_end_points` | `coPoints`, `comDevelop`, `readonly`, default `{ Vec2d(30,-3), Vec2d(54,245) }`, no bounds | `GCode.cpp::get_path_of_change_filament` (the only read site in `src/libslic3r/`; called only from `GCode::_do_export` when `m_writer.multiple_extruders`); the computed `travel_point_1/2/3` are injected into the dynamic config as `travel_point_1_x/y` … `travel_point_3_x/y` at `WipeTowerIntegration::append_tcr` and `GCode::process_toolchange`, consumed by the `change_filament_gcode` placeholder substitution | the filament-change travel path from the cutter area to the garbage can, steered around object bounding boxes; safe default `(54,0),(54,0),(54,245)` when `start_end_points.size() != 2` or `bed_exclude_area.size() != 4` | **BUILT** — the host computes the same path from `start_end_points` + `bed_exclude_area` + per-object XY bounds (`crates/slicer-runtime/src/travel_path.rs`), injects the six `travel_point_*` values into `config_source`, and `machine-gcode-emit`'s substitution resolves them (AC-9, AC-10, AC-11, AC-N4). |

## In-Tree Grounding (verified at authoring, 2026-09-02)

- **The stage exists and is free.** `PrePass::MeshAnalysis` is a full `StageSpec` in `crates/slicer-schema/src/lib.rs` (`method: "run_mesh_analysis"`, `trait_name: "PrepassModule"`, `wit_dir: "prepass-mesh-analysis"`, `wit_world: "mesh-analysis-module"`, WIT package `slicer:prepass-mesh-analysis@1.0.0`) and appears in `VALID_STAGES`. **No core module declares it** — the stage runs only its host built-in `host:mesh_analysis` (`crates/slicer-runtime/src/prepass.rs`, gated on `bb.surface_classification().is_none()`), and `required_slots("PrePass::MeshAnalysis")` is empty, so a module there needs no prior prepass output.
- **The WIT contract is sufficient.** `crates/slicer-schema/wit/deps/prepass-mesh-analysis/prepass-mesh-analysis.wit`'s `run` takes `objects: list<object-id>`, a `mesh-analysis-output` resource (`push-facet-annotation` / `push-surface-group` — both optional to call) and a `config-view`; the `mesh-analysis-module` world imports `slicer:common/host-services`, which exposes `object-bounds(object-id) -> bounding-box3`, `raycast-z-down(object-id, x, y, start-z) -> option<f32>` and `surface-normal-at(...)`. Raw triangles are **not** available at this stage: `mesh-object-view` (with `vertices` / `triangles`, `crates/slicer-schema/wit/deps/prepass-types.wit`) is passed only to `prepass-seam-planning` and `prepass-support-geometry`.
- **The SDK trait is a no-op by default.** `PrepassModule::run_mesh_analysis(&self, objects: &[ObjectId], output: &mut MeshAnalysisOutput, config: &ConfigView) -> Result<(), ModuleError>` (`crates/slicer-sdk/src/traits.rs`) ships a default body returning `Ok(())`; `MeshAnalysisOutput` (`crates/slicer-sdk/src/prepass_builders.rs`) is a two-`Vec` accumulator whose `push_facet_annotation` / `push_surface_group` are both optional — there is no commit or finish method, so a module that produces nothing is a supported shape. Host services reach the guest through `slicer_sdk::host::{object_bounds, raycast_z_down}` and the batched `slicer_sdk::host_batch::raycast_z_down_batch`; `crates/slicer-sdk/src/test_support/mock_host.rs` provides the test double the probe-contract AC counts calls on. Note `object_bounds` returns `Result<BoundingBox3, HostUnavailable>`, not `Option`.
- **A fatal module error aborts the slice.** `crates/slicer-wasm-host/src/dispatch.rs` converts a `module-error` with `fatal` set into a `DispatchError`; `crates/slicer-runtime/src/prepass.rs` raises `PrepassExecutionError::FatalModule { stage_id, module_id, message }`, which converts into `PipelineError::Prepass`. Non-fatal errors are logged and execution continues — so the validator MUST set `fatal`.
- **Config transport.** `resolve_global_config` (`crates/slicer-scheduler/src/config_resolution.rs`) routes any key without a declared `ResolvedConfig` field into `cfg.extensions`; `ResolvedConfig::to_config_map` (`crates/slicer-ir/src/resolved_config.rs`) merges `extensions` through unchanged; `bind_module_config_view` (`crates/slicer-scheduler/src/execution_plan.rs`) then filters to the module's declared keys. No host change is needed for `bed_exclude_area` to reach either module.
- **Orca 3MF ingest.** `slicer_ir::parse_orca_point_string` (defined in `crates/slicer-ir/src/resolved_config.rs`, re-exported from `crates/slicer-ir/src/lib.rs`) parses `"250x210"` into `(x, y)` and returns `None` unless exactly two `x`-separated finite floats; `float_list_from_config` in `modules/core-modules/wipe-tower/src/lib.rs` already routes `ConfigValue::List` strings through it. `bed_bounds_tdd.rs::orca_point_string_bed_is_parsed_not_silently_defaulted` pins that path end-to-end for `printable_area`.
- **The wipe-tower check.** `run_finalization` validates four corners with `point_in_polygon` (even-odd, on-edge counts inside) and returns `ModuleError::fatal(3, "wipe-tower corner (x, y) lies outside bed polygon")`; `parse_printable_area` rejects empty / odd / fewer-than-6 raw values with `ModuleError::fatal(2, …)`. Code 4 is entity insertion. The exclusion rejection extends the code-3 site.
- **No footprint helper is exposed.** `compute_xy_footprint` (private) and `bottom_surface_footprint` exist in `crates/slicer-core/src/algos/mesh_analysis.rs`, host-side only; nothing equivalent crosses the WIT boundary. There is no 2D convex-hull helper anywhere in `crates/`.
- **No existing validation error.** No `SchedulerError`, `ConfigResolutionError`, `SliceRunError` or `ModelLoadError` variant covers out-of-bed or excluded-area geometry; the only bed-shape error in the tree is `pnp_cli::visual_debug::VisualDebugError::InvalidBedShape`, which is viewport framing, not printability. This packet adds none either — the module's `ModuleError::fatal` is the whole mechanism.
- **Claim vocabulary.** `RECOGNIZED_CLAIMS` covers `perimeter-generator`, `support-generator`, `seam-placer`, `layer-planner`, `mesh-analyzer`, `slice-postprocessor`, `gcode-postprocessor`, `text-postprocessor`, plus the `claim:*` fill roles. `mesh-analyzer` is recognized but held by no core module. The validator holds **nothing** — see `design.md` § Mechanism Check.
- **Registration and counting.** `modules/core-modules/` holds 23 directories at authoring; `core_modules_directory_is_discoverable_and_all_load` (`crates/slicer-scheduler/tests/integration/manifest_ingestion_tdd.rs`) asserts that exact count. **Re-derive it from disk at implementation time** — draft packet `254b` also adds a module. The runtime's integration bucket is aggregated by `mod` declarations in `crates/slicer-runtime/tests/integration/main.rs`, so a new file there MUST be registered or it silently compiles to zero tests.
- **Guest build.** `discover_guests` / `guest_input_paths` (`xtask/src/build_guests.rs`) pick up a core module by its directory shape (`src/`, `Cargo.toml`, depth-1 `*.toml`, `wit-guest/`), so the new module is fingerprinted automatically once it has a `wit-guest/`.
- **The travel-path seam (verified 2026-09-05).** `machine-gcode-emit` (`modules/core-modules/machine-gcode-emit/`) is a `PostPass::GCodePostProcess` module whose `run_gcode_postprocess` walks the whole command stream and substitutes each injection template's `[key]` placeholders against a lookup built from `config.keys()` (its own `ConfigView`, schema-filtered by `bind_module_config_view`). `change_filament_gcode` splices at `InjectionSite::FilamentChange`, which fires at each `GCodeCommand::ToolChange` — so the template is inert in single-tool prints. An unresolved `[key]` passes through verbatim with an aggregated warning, never a slice error. The module's `from_config` currently returns `Ok(Self)` with an empty struct.
- **ADR-0050 §2 locks the placeholder domain.** `docs/adr/0050-custom-gcode-architecture.md` §2: the resolvable name set is **exactly** `machine-gcode-emit`'s manifest-declared `[config.schema]` keys (as handed to the guest through `ConfigView`) plus the `PLACEHOLDER_ALIASES` table applied after the `config.keys()` sweep. A computed-value insertion into the lookup would be a third source and is **forbidden** — the travel points must be manifest-declared keys whose values the host writes into `config_source`. This packet conforms; no ADR amendment.
- **The host-injected-key pattern.** `run_slice` (`crates/slicer-runtime/src/run.rs`) already seeds `config_source` with host-derived keys: `thumbnail_path`, `object_height:<id>` (per-object world heights), `slice_has_paint` (painted-slice gate, declared on `classic-perimeters.toml` "expecting the host to populate it"), `object_config:<id>:<key>`, and the MMU wipe-tower auto-enable (`enable_prime_tower = true` when the model paints ≥ 2 distinct tool indices). These keys flow into every module's `ConfigView` (schema-filtered) and into the CONFIG_BLOCK via `extensions` → `to_config_map` — `slice_has_paint = true` and `object_height:<id>` are verifiably present in real G-code output today. The travel-point injection follows this exact pattern.
- **Why the host computes.** The `gcode-postprocess-module` world's `run(commands, output, config)` receives **no object ids** and there is no object-enumeration host service — a postpass module cannot read object geometry. The prepass seam can read `object-bounds` but has no arbitrary-data output (`MeshAnalysisOutput` is facet annotations + surface groups only). And ADR-0050 §2 forbids a module-side computed-value insertion. So the host computes the path (it has `mesh_ir.objects` and `config_source`) and the values ride manifest-declared config keys. `ResolvedConfig.extensions` is a `BTreeMap<String, ConfigValue>` merged through `to_config_map` unchanged — the transport for the six `travel_point_*` values.
- **The canonical algorithm (verified 2026-09-05, delegated read).** `get_path_of_change_filament` (`GCode.cpp`): safe default `(54,0),(54,0),(54,245)` returned when `start_end_points.size() != 2` or `bed_exclude_area.size() != 4`; `cutter_area_x/y = excluse_area[2] + 2`; per object, `can_travel_form_left = false` when `bbox.min.x() < start_x && bbox.min.y() < cutter_area_y`, interval `[min.x-2, max.x+2]` merged with overlapping intervals; intervals sorted; available intervals built from `start_position = 0` over `[0, 255]`; `new_path = 255` then the four-branch selection (first > end_x → nearest of `new_path`/`first`, break; second ≥ end_x → `end_x`, break; `!can_travel_form_left && second < start_x` → continue; else `second`); final points `(start_x, cutter_area_y), (new_path, cutter_area_y), (new_path, end_y)` when `new_path < start_x`, else `(new_path, 0), (new_path, 0), (new_path, end_y)`. The `255` is a hardcoded bed-x constant, reproduced verbatim.
- **`start_end_points` canonical declaration.** `coPoints`, `comDevelop`, `readonly`, default `{ Vec2d(30,-3), Vec2d(54,245) }`, no bounds (`PrintConfig.cpp`). The key always carries its default in canonical, so the path is computed whenever `bed_exclude_area` also has 4 points; the safe default fires only on malformed values. The port mirrors this: `start_end_points` absent → module fallback to the canonical default; ≠ 2 points → safe default.
- **Object XY bounds.** `ObjectMesh.mesh` is an `IndexedTriangleSet` (`crates/slicer-ir/src/slice_ir.rs`); `ObjectMesh.transform` is `identity_transform()` for every loaded object (ticket 35's finding — the 3MF transform is baked into the vertices), but the bounds helper applies it anyway for correctness. `crates/slicer-model-io/src/loader.rs` has the pattern to copy: `compute_z_extent_from_mesh` / `object_world_z_extent_from_mesh_and_transform` (private, loader-local). The travel-path bounds are computed in `run_slice` from `mesh_ir.objects` — no IR change.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` (module manifest + count), `AC-2` (fatal rejection), `AC-3` (clear object unaffected), `AC-4` (probe contract), `AC-5` (Orca 3MF form), `AC-6` (degenerate = no exclusion), `AC-7` (wipe-tower half), `AC-8` (docs), `AC-9` (travel-path manifest + substitution), `AC-10` (canonical path algorithm), `AC-11` (host-injected `travel_point_*` gate).
- Negative: `AC-N1` (no cross-module leakage), `AC-N2` (inert with no key, byte-identical G-code), `AC-N3` (manifest guard), `AC-N4` (malformed travel-path inputs → safe default, never a failure).
- Cross-packet impact: the core-module count is shared with draft `254b`; the `wipe-tower` manifest is shared with `254a` / `254b` / `255`; the `machine-gcode-emit` manifest is shared with packets 253, 267 and the P18/P20 work.

## Verification Matrix

This is the authoritative full matrix; `packet.spec.md` § Verification lists the gate commands only.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-scheduler --test scheduler_integration manifest_ingestion_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-1: manifest loads, count incremented | FACT pass/fail |
| `cargo test -p print-validator --test bed_exclusion_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-2, AC-4, AC-5, AC-6, AC-N3 | FACT pass/fail; SNIPPETS ≤ 20 lines on failure |
| `cargo test -p slicer-runtime --test integration bed_exclusion_abort_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-2 (abort path), AC-3, AC-N2 | FACT pass/fail |
| `cargo test -p wipe-tower --test bed_bounds_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-7 | FACT pass/fail |
| `cargo test -p slicer-runtime --lib travel_path 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-10, AC-N4 (host path-computation unit arms) | FACT pass/fail; SNIPPETS ≤ 20 lines on failure |
| `cargo test -p slicer-runtime --test integration travel_path_injection_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-9 (substitution), AC-11 (host-injected `travel_point_*` gate, single-tool absence) | FACT pass/fail |
| `cargo test -p slicer-runtime --test contract config_view_binding_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-N1 | FACT pass/fail |
| `cargo xtask gen-config-docs --check && rg -q 'bed_exclude_area' docs/15_config_keys_reference.md && rg -q 'start_end_points' docs/15_config_keys_reference.md && rg -q 'travel_point_1_x' docs/15_config_keys_reference.md; echo "exit=$?"` | AC-8 | FACT exit=0 |
| `rg -q 'print-validator' docs/04_host_scheduler.md; echo "exit=$?"` | Doc Impact | FACT exit=0 |
| `cargo xtask build-guests --check; echo "exit=$?"` | new guest is discovered and fresh; `wipe-tower` + `machine-gcode-emit` guests rebuilt | FACT exit=0 (exit 3 = `wasm-tools` missing, not clean) |
| `cargo check --workspace --all-targets` | workspace type gate | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail |
| `cargo xtask check-literals 2>&1 \| tail -3` | struct-literal churn gate | FACT pass/fail |

## Step Completion Expectations

- The module crate must exist and be a workspace member before the integrated registry references it; the registry must reference it before the core-module count assertion changes. That ordering is the reason Steps 1–2 are separate.
- The core-module count is a **ledger fact**: re-derive it (`ls -d modules/core-modules/*/ | wc -l`) in the step that changes the assertion, and reconcile with `254b` if that packet landed first.
- The new runtime integration test file MUST be added to `crates/slicer-runtime/tests/integration/main.rs`'s `mod` list in the same step that creates it, or the run reports "0 tests" and looks green.
- The validator must set `fatal` on its `ModuleError`; a non-fatal error is logged and the slice continues, which would make AC-2 silently unenforced.
- The guest freshness gate runs at the end of the module step and again at the acceptance ceremony.
- Implementation is recorded against wayfinder ticket 11; `docs/07_implementation_status.md` holds no TASK row for this queue.

## Context Discipline Notes

- `crates/slicer-runtime/src/prepass.rs` and `crates/slicer-ir/src/resolved_config.rs` are long: the facts this packet needs are in § In-Tree Grounding. Range-read only, one hypothesis per read.
- `docs/15_config_keys_reference.md` and `docs/ORCA_CONFIG_REFERENCE.md`: never read in full.
- Model an existing prepass module rather than inventing structure: `modules/core-modules/layer-planner-default/` is the closest shape (manifest, `wit-guest/`, `src/`, `tests/`).
- `OrcaSlicerDocumented/` is the **sibling** checkout `..\pinch_n_print_cli\OrcaSlicerDocumented`; all reads delegated. Re-derive the absolute path on first use.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet (the checkout is the **sibling** path `..\pinch_n_print_cli\OrcaSlicerDocumented`):

- `src/libslic3r/PrintConfig.cpp` — `PrintConfigDef` (`bed_exclude_area` and `start_end_points` facts) and `get_bed_excluded_area`.
- `src/libslic3r/Print.cpp` — `Print::validate`, `layered_print_cleareance_valid`, `sequential_print_clearance_valid`.
- `src/libslic3r/GCode.cpp` — `get_path_of_change_filament` (the algorithm this packet ports verbatim — safe default, `+2` margins, `[0, 255]` constant, four-branch `new_path` selection, left/right travel) and the two `travel_point_*` dynamic-config injection sites (`WipeTowerIntegration::append_tcr`, `GCode::process_toolchange`); `src/libslic3r/GCode/GCodeProcessor.cpp` — `apply_config`; `src/libslic3r/GCode/TimelapsePosPicker.cpp` — `construct_printable_area_by_printer` (returned-consumer evidence only).
