# Requirements: support-support-keys

## Packet Metadata

- **Packet directory:** `docs/spec_packets/265-support-support-keys/`
- **Slug:** `support-support-keys`
- **Status:** `draft`
- **Task IDs:** none (queue packet — `task_ids: []`, precedent packets 234a, 253–264)
- **Backlog source:** wayfinder ticket 20 (`docs/specs/orca-feature-gap/issues/20-author-packet-p13-support-support-support-planner.md`), map `docs/specs/orca-feature-gap/map.md` packet P13
- **Tier:** **B** — re-derived. The prior revision was Tier A on the "declare + wire the cheap keys" reading. Under map Authoring rule 1 a packet that *builds* a decision point is B or C; this packet builds five decision points, all of them new logic inside owners that already exist (two planner modules, one `slicer-core` algo, one `slicer-runtime` builtin) and adds no module and no new seam, which is exactly the ticket-04 rubric's Tier B. See `design.md` § Tier Derivation.
- **Re-authoring note:** this directory is overwritten in place (number and slug retained) with explicit user approval, under map Authoring rules 1–7.

## Problem Statement

The pre-rules packet 265 covered twelve keys by declaring five of them "declared-with-gap" — manifest stubs with canonical types and defaults, zero read sites, and an AC that asserted only non-perturbation. Authoring rule 1 prohibits that disposition outright, and rule 5 removes the plumbing exemption it leaned on. Re-derived from disk at authoring time, the twelve keys split three ways.

Five keys have a live, behaviour-changing decision point today. `support_expansion`, `support_threshold_angle` and `support_threshold_overlap` are consumed by `slicer_core::algos::overhang_annotation`'s contact detection (the XY-expansion step, the angle-derived lower-layer offset, and the zero-angle overlap branch respectively). `support_object_xy_distance` drives the per-layer object trim in `traditional-support-planner` and the collision inflation in `tree-support-planner`. `support_type` is resolved per region by the support-analysis producer's `effective_support_type` and selects the family through `slicer_scheduler::execution_plan::select_support_family`.

Five keys are declared in `docs/config/host-keys.toml`, carried as typed `ResolvedConfig` fields, and read by nothing. Four of those five have a decision point that is genuinely absent, and the fifth — `enforce_support_layers` — is the sharpest case in the whole queue: the decision point *already exists* (`SupportContactParams::enforce_support_layers` feeds the `force_support` branch in `detect_support_contacts`), and `resolve_contact_params` hardcodes the field to `0` behind a comment that says the knob has "no production config source yet". The key is one line from being live.

Two keys must leave the packet. `raft_first_layer_expansion` has zero occurrences in Rust or TOML source under `crates/` and `modules/`, and its owner is already assigned: packet `240-support-raft` lists it by name among the raft keys it wires. `support_style` is live in `traditional-support` (it resolves `smooth_supports`) but its manifest declaration is inconsistent across the tree, and correcting it collides with queued work that has no packet directory yet.

This packet therefore builds five decision points, proves five live ones at non-default values, and returns two keys to the queue with their owners named.

## Key Disposition Table

Classification per the map's Authoring rules: **(a)** live behaviour-changing decision point already in tree; **(b)** decision point this packet builds; **(c)** returned to queue (no decision point, not built here); **(d)** dead-in-canonical.

| Key | Class | Owner | Decision point | Non-default AC |
| --- | --- | --- | --- | --- |
| `support_expansion` | **(a)** | `slicer-core` (`algos::overhang_annotation`) | `SupportContactParams::xy_expansion_mm` — the XY growth applied to the finished contact region | AC-7 |
| `support_threshold_angle` | **(a)** | `slicer-core` (`algos::overhang_annotation`), per-region via `algos::region_mapping` | the angle-derived lower-layer offset, including the canonical `+1` inclusivity bump and 89-degree clamp | AC-8 |
| `support_threshold_overlap` | **(a)** | `slicer-runtime` (`builtins::support_analysis_producer::resolve_contact_params`) → `slicer-core` | `threshold_overlap_mm`, consulted on the zero-angle branch | AC-9 |
| `support_object_xy_distance` | **(a)** | `traditional-support-planner`, `tree-support-planner` | the per-layer object trim offset and `inflate_model_occupancy`'s collision inflation | AC-10 |
| `support_type` | **(a)** | `slicer-scheduler` (`execution_plan::select_support_family`), `slicer-runtime` (`effective_support_type`) | family dispatch and the `is_auto()` gate on angle-based detection | AC-11 |
| `enforce_support_layers` | **(b)** | `slicer-runtime` (`resolve_contact_params`) | source the field from config instead of the hardcoded `0`; the consuming `force_support` branch already exists | AC-1 |
| `support_critical_regions_only` | **(b)** | `slicer-core` (`algos::overhang_annotation`) | new: when set, replace ordinary contacts with the cantilever and sharp-tail sets already computed in the same function | AC-2 |
| `support_remove_small_overhang` | **(b)** | `slicer-core` (`algos::overhang_annotation`) | new: canonical's cluster erode-and-measure filter, with the sharp-tail / cantilever exemption | AC-3 |
| `support_bottom_z_distance` | **(b)** | `traditional-support-planner`, `tree-support-planner` | new: a bottom air gap on model-terminated columns, symmetric with the live `support_top_z_distance` gap | AC-4 |
| `support_object_first_layer_gap` | **(b)** | `traditional-support-planner`, `tree-support-planner` | new: a layer-0 substitution for the XY clearance at the two existing clearance sites | AC-5, AC-6 |

Counts: **(a) 5 · (b) 5 · (c) 2 · (d) 0**, twelve keys accounted for. Zero declaration-only keys (map preflight gate (a)); every in-packet key carries at least one AC asserting a behaviour change at a non-default value (map preflight gate (b)).

## Returned to Queue — unimplemented

### `raft_first_layer_expansion` — needs a raft generator

`coFloat`, canonical default `2.0`, min 0, no max. **Zero occurrences in Rust or TOML source** under `crates/` and `modules/` — re-derived from disk at authoring time (generated artifacts under `target/` are build output, not source, and are never evidence of a read site). It is not even a `ResolvedConfig` field, unlike the other four with-gap keys.

Its canonical decision points are `SupportCommon.cpp` `generate_raft_base` (scaled into `inflate_factor_1st_layer` to grow the raft's first layer), `TreeSupport::generate_toolpaths` (offsets `raft_areas` and the layer-0 support expansion), and `TreeSupport::draw_circles` (inflates hybrid-tree layer-0 areas when raft layers are absent). All three require raft geometry, which this tree does not have.

**Owner: packet `240-support-raft`.** That packet's `requirements.md` already lists this key by name among the "Issue-19/20 raft keys: `raft_contact_distance`, `raft_expansion`, `raft_first_layer_expansion`; wire the existing dead raft keys". The sibling approved plan (`D:\slicerProject\pinch_n_print_cli\docs\specs\support-generation-remediation-plan.md`, row 3 `raft-geometry`, TASK-324) records that row as **absorbed** into `docs/spec_packets/240-support-raft/`. This packet must not declare the key; AC-N2 asserts its absence from the planner manifests so a future worker cannot quietly re-add it as a stub. The map's queue entry for packet 261 reached the same disposition for the sibling raft keys.

### `support_style` — needs the organic tree engine

`coEnum` over `default, grid, snug, organic, tree_slim, tree_strong, tree_hybrid`, canonical default `default`. The key is **live** in one place: `traditional-support`'s `from_config` reads it and derives `smooth_supports` as canonical's `support_params.support_style != smsGrid`. It is declared as a 7-value `enum` in `tree-support-planner.toml` and, inconsistently, as a bare `type = "string"` in `traditional-support.toml`.

The inconsistency is real, but correcting it in isolation would cover nothing that is not already covered: the `smooth_supports` branch is a two-way split on `grid`, so five of the seven values are behaviourally indistinguishable in this tree today. The values that make them distinguishable — `default` and `organic` running the canonical organic engine — are the subject of sibling-plan row 7 (`organic-tree-engine`, TASK-441, status `queued`, **no packet directory exists**), which ports `TreeSupport3D.cpp` and its `TreeModelVolumes` avoidance model and retires the DEV-156 Strong alias.

**Owner: sibling-plan row 7 / TASK-441**, which still needs a packet number derived from disk when it is authored. Returned here as *unimplemented, needs the organic tree engine*. The manifest-type inconsistency is reported to the map (see `design.md` § Map and Ticket Updates Required) and is **not** edited by this packet; AC-N2 asserts `traditional-support.toml`'s `support_style` table is untouched.

## Ruled Dead-in-Canonical

**None.** All twelve of ticket 20's keys have at least one read site inside OrcaSlicer's slicing pipeline under `src/libslic3r/`, verified per key at authoring time by a delegated sweep that explicitly excluded `src/slic3r/GUI/**` (including `ConfigManipulation.cpp`), `PrintConfig.cpp` tooltip and label text, `Preset.cpp` key lists, and `IGNORE`/legacy-alias sets. Authoring rule 3 therefore rules none of them out of scope.

One shared caveat, recorded because it is exactly the trap rule 3 warns about: **all twelve keys also appear in `PrintObject::invalidate_state_by_config_options`**, which is a key-*name* list and not a value read. It was excluded from the evidence below and must not be cited as a read site.

## Per-Key Canonical Evidence

Cited by file and function, never by line number (repo rule).

| Key | Canonical read sites under `libslic3r/` |
| --- | --- |
| `enforce_support_layers` | `Support/SupportMaterial.cpp` `detect_overhangs` (layers below the index get forced contact instead of angle-based detection); `Support/TreeSupport.cpp` `TreeSupport::detect_overhangs` (gates non-auto support, picks a fixed lower-layer offset for enforced layers); `Support/TreeSupport3D.cpp` `generate_overhangs`; `Slicing.cpp` `SlicingParameters::create_from_config` (counts as support-enabled for raft/contact setup) |
| `support_critical_regions_only` | `Support/TreeSupport.cpp` `TreeSupport::detect_overhangs` (clears ordinary overhangs, keeps only critical regions); `PrintObject.cpp` `PrintObject::detect_overhangs_for_lift` (must be false for bottom surfaces to count as fully supported) |
| `support_remove_small_overhang` | `Support/SupportMaterial.cpp` `PrintObjectSupportMaterial::top_contact_layers` (clusters overhangs and erases small ones); `Support/TreeSupport.cpp` `TreeSupport::detect_overhangs` (identical cluster-based removal) |
| `support_bottom_z_distance` | `Slicing.cpp` `SlicingParameters::create_from_config` (becomes `gap_object_support`, feeds `zero_gap_interface_bottom`); `GCode.cpp` `GCode::collect_layers_to_print` (bottom contact distance for the floating-layer gap check) |
| `support_object_first_layer_gap` | `Support/SupportParameters.hpp` `SupportParameters::SupportParameters` (sets `gap_xy_first_layer`); `Support/TreeSupport.cpp` `TreeSupport::draw_circles` (XY offset used on object layer 0 instead of `m_xy_distance`); `Support/TreeSupportCommon.hpp` `TreeSupportSettings::TreeSupportSettings` (`support_xy_distance_1st_layer`) |
| `support_expansion` | `Support/SupportMaterial.cpp` `detect_overhangs` (scaled into the `xy_expansion` that grows detected contact areas) |
| `support_object_xy_distance` | `Support/SupportParameters.hpp` `SupportParameters::SupportParameters` (sets `gap_xy`); `Support/TreeSupportCommon.hpp` `TreeSupportSettings::TreeSupportSettings` (`support_xy_distance`, and clamps `support_xy_distance_overhang`) |
| `support_style` | `Support/SupportParameters.hpp` `SupportParameters::SupportParameters` (resolves `smsDefault` against `support_type` into the effective style consumed by `SupportCommon` and `TreeSupport`) |
| `support_threshold_angle` | `Support/SupportMaterial.cpp` `detect_overhangs` (`thresh_angle` drives the per-layer overhang offset); `Support/TreeSupportCommon.hpp` `TreeSupportSettings::TreeSupportSettings` (`support_angle`) |
| `support_threshold_overlap` | `Support/SupportMaterial.cpp` `detect_overhangs` (the `fw - scale_(get_abs_value(...))` lower-layer offset); `Support/TreeSupport3D.cpp` `generate_overhangs` (same expression against `external_perimeter_width`) |
| `support_type` | `Support/SupportParameters.hpp` `SupportParameters::SupportParameters` (`is_tree()` decides the default style); `Support/TreeSupport.cpp` `TreeSupport::detect_overhangs` / `TreeSupport::generate` (`is_auto`/`is_tree` gate tree generation); `Support/SupportMaterial.cpp` `detect_overhangs` (`auto_normal_support` gates angle-based detection) |
| `raft_first_layer_expansion` | `Support/SupportCommon.cpp` `generate_raft_base`; `Support/TreeSupport.cpp` `TreeSupport::generate_toolpaths` and `TreeSupport::draw_circles` |

### Canonical semantics the port borrows exactly

- **Critical regions.** In `TreeSupport::detect_overhangs`, when the flag is set and support type is auto, the code clears the layer's ordinary overhangs entirely and re-appends only cantilevers; sharp tails are appended later in the same loop unconditionally, and user enforcers are added after the clear. The surviving set is therefore *cantilevers + sharp tails + enforcers*. The port reproduces exactly that ordering.
- **Small-overhang smallness.** Overhangs are grouped into vertical clusters. A cluster is exempt if it overlaps a sharp tail or a cantilever. Otherwise the merged cluster polygon is eroded by one scaled extrusion width and the bounding-box extent of the erosion is measured; the cluster is small when `bbox.x < 2 * fw` **or** `bbox.y < 2 * fw`. It is a bounding-box-width criterion in extrusion widths, **not** an area threshold. `SupportMaterial.cpp::top_contact_layers` and `TreeSupport::detect_overhangs` use the identical expression.
- **First-layer gap.** `TreeSupport::draw_circles` substitutes `support_object_first_layer_gap` for `m_xy_distance` when the object layer index is 0, and uses `m_xy_distance` otherwise. It is a substitution, not an addition.

### Canonical behaviour the port deliberately does not borrow

- `support_bottom_z_distance == 0` falls back to `support_top_z_distance` **only** in `GCode::collect_layers_to_print`'s air-gap sanity check. In `Slicing.cpp` a zero value combined with bottom interface layers instead sets `gap_object_support = 0` through the zero-gap-interface path. The port has no equivalent of the G-code-side sanity check, so importing the fallback there would invent a coupling canonical does not have at the geometry seam. Recorded as **DIV-1** in `design.md`.

## In Scope

1. **`SupportContactParams` gains two fields** — `critical_regions_only: bool` (default `false`) and `remove_small_overhang: bool` (default `true`) — in `crates/slicer-core/src/algos/overhang_annotation.rs`, with the `Default` impl updated to the canonical defaults. This is a watched struct-literal type; the step that adds the fields owns the full blast radius (see `implementation-plan.md`).
2. **Two new filter stages in `detect_support_contacts_with_annotations`** in the same file: the small-overhang cluster filter (erode by one `external_perimeter_width_mm`, measure the bbox extent, drop when either extent is below `2 * fw`, exempt anything overlapping the sharp-tail or cantilever sets), then the critical-regions restriction (replace `contacts` with the union of the cantilever and sharp-tail sets). Ordering matches canonical: filter first, restrict second, enforcers unioned by the caller afterwards.
3. **`resolve_contact_params` sources three fields from config** in `crates/slicer-runtime/src/builtins/support_analysis_producer.rs`: `enforce_support_layers` from `config.enforce_support_layers` with the `extension_int` per-region override path used by the sibling keys, `critical_regions_only` from `config.support_critical_regions_only`, `remove_small_overhang` from `config.support_remove_small_overhang`. The stale comment claiming these knobs have "no production config source yet" is corrected, and `bridge_no_support` / `bridge_polygons` / `support_sharp_tails` keep their current neutral sourcing (they belong to other tickets).
4. **Bottom-Z gap in both planners.** In `traditional-support-planner`, when `model_termination_layer` is `Some(t)`, raise the emit floor above `t` by walking actual layer Z until the accumulated gap reaches `support_bottom_z_distance` — the same Z-walk idiom the live `target_top_z` computation uses, and explicitly *not* a division by `effective_layer_height` (register rows G-09 and RC-11 prohibit it). Build-plate terminations (`None`, collapsing to layer 0) get no gap. The tree planner gets the equivalent at its own descent-termination site.
5. **First-layer XY gap in both planners.** In `traditional-support-planner`, the per-layer trim offset selects `support_object_first_layer_gap` when the layer index is 0 and `support_object_xy_distance` otherwise. In `tree-support-planner`, both `inflate_model_occupancy` call sites make the same selection on the object layer index.
6. **Manifest declarations** for `support_bottom_z_distance` (float, default `0.2`, min 0, no max) and `support_object_first_layer_gap` (float, default `0.2`, `[0.0, 10.0]` canonical bounds) on `traditional-support-planner.toml` and `tree-support-planner.toml`, each with a `description` naming the canonical consumer. No other manifest table is added or edited.
7. **Tests**: one net-new `slicer-core` test file plus its `[[test]]` entry carrying `required-features = ["host-algos"]`; one net-new test file plus `[[test]]` entry in each planner crate; new cases in the producer's own `tests` module; a bounds arm in `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs`.
8. **Docs**: regenerate `docs/15_config_keys_reference.md`.

## Out of Scope

- **`ORCA_CONFIG_PADDING` and every CONFIG_BLOCK twin.** Authoring rule 2: padding is not parity, is never a deliverable, and is not evidence. AC-N3 asserts the file is untouched.
- **Raft geometry of any kind** — packet `240-support-raft`.
- **The organic tree engine and the `support_style` manifest-type correction** — sibling-plan row 7 / TASK-441.
- **`bridge_no_support`, `support_sharp_tails` sourcing, and `bridge_polygons`.** All three are still hardcoded neutral in `resolve_contact_params`. They are not ticket-20 keys; this packet reads the sharp-tail *result* for the critical-regions and exemption logic but does not change how the sharp-tail flag itself is sourced. Reported to the map as a separate gap.
- **The per-object / per-filament config model.** Canonical declares these on `PrintObjectConfig`; the port declares them scalar-global in owner manifests and per-region through `algos::region_mapping`. That model question is Tier-D fog, not this packet (the 260a precedent).
- **`support_top_z_distance`.** Live and correct; not a ticket-20 key.

## Authoritative Docs

- `docs/00_project_overview.md` — project goals the design must satisfy (modular pipeline, community extensibility, config robustness).
- `docs/01_system_architecture.md` § Claim System — read to confirm rule 4's cross-module trigger test does **not** fire here (see `design.md` § Rule 4 Trigger Test).
- `docs/04_host_scheduler.md` § Claim Resolution — support-family dispatch, the mechanism `support_type` rides.
- `docs/08_coordinate_system.md` — every offset in this packet is in millimetres at the module boundary; the 1 unit = 100 nm hazard applies to the `slicer-core` polygon ops.
- `docs/21_data_defaults_and_fixtures.md` — the struct-literal churn gate that governs the two new `SupportContactParams` fields.
- `docs/15_config_keys_reference.md` (generated) — regenerate at close.
- `docs/specs/support-parity-gap-register.md` — row G-05, closed by this packet; rows G-09 and RC-11, whose prohibition on dividing by `effective_layer_height` constrains the bottom-Z build.

## Parity Evidence Standard

Under Authoring rule 5, "default matches and the value reaches the consumer" is **not** sufficient evidence for any key in this packet. Each key's evidence is a behaviour difference measured between two runs that differ only in that key's value, with the non-default value named in the AC. AC-N4 exists solely as a regression guard on the default path and is not evidence for any key.

## Acceptance Summary

| AC | Key | Class | Asserts |
| --- | --- | --- | --- |
| AC-1 | `enforce_support_layers` | b | forced contacts on layers below the index at `3` vs none at `0` |
| AC-2 | `support_critical_regions_only` | b | ordinary overhang dropped, cantilever kept, at `true` vs both at `false` |
| AC-3 | `support_remove_small_overhang` | b | narrow cluster survives at `false` vs erased at `true` |
| AC-4 | `support_bottom_z_distance` | b | model-terminated column floor rises at `0.6` vs `0.2`; plate-terminated unaffected |
| AC-5 | `support_object_first_layer_gap` | b | layer-0 clearance `1.0`, layer-1 clearance `0.35` |
| AC-6 | `support_object_first_layer_gap` | b | tree-family inflation distance differs on object layer 0 only |
| AC-7 | `support_expansion` | a | contact area grows at `0.5` vs `0.0` |
| AC-8 | `support_threshold_angle` | a | contact set shrinks at `60` vs `30` |
| AC-9 | `support_threshold_overlap` | a | `threshold_overlap_mm` equals `fw` at `100%` vs half at `50%` |
| AC-10 | `support_object_xy_distance` | a | trimmed area shrinks at `1.0` vs `0.35` |
| AC-11 | `support_type` | a | `tree(auto)` selects the tree family vs traditional at `normal(auto)` |
| AC-N1 | bounds | — | out-of-range values rejected, not clamped |
| AC-N2 | returned keys | — | `raft_first_layer_expansion` and `support_style` not re-stubbed |
| AC-N3 | padding | — | `ORCA_CONFIG_PADDING` untouched |
| AC-N4 | default path | — | regression guard only |

## Verification Matrix

| Surface | Command |
| --- | --- |
| `slicer-core` new filters (AC-2, AC-3) | `mkdir -p target && cargo test -p slicer-core --features host-algos --test support_critical_and_small_overhang_tdd 2>&1 \| tee target/test-output.log` |
| `slicer-core` live keys (AC-7, AC-8) | `mkdir -p target && cargo test -p slicer-core --features host-algos --test support_overhang_detection_tdd 2>&1 \| tee target/test-output.log` |
| producer wiring (AC-1, AC-9) | `mkdir -p target && cargo test -p slicer-runtime --lib support_analysis_producer 2>&1 \| tee target/test-output.log` |
| traditional planner (AC-4, AC-5) | `mkdir -p target && cargo test -p traditional-support-planner --test support_gap_keys_tdd 2>&1 \| tee target/test-output.log` |
| traditional planner regression (AC-10, AC-N4) | `mkdir -p target && cargo test -p traditional-support-planner --test traditional_family_tdd 2>&1 \| tee target/test-output.log` |
| tree planner (AC-6) | `mkdir -p target && cargo test -p tree-support-planner --test support_gap_keys_tdd 2>&1 \| tee target/test-output.log` |
| scheduler (AC-11, AC-N1) | `mkdir -p target && cargo test -p slicer-scheduler --test scheduler_integration 2>&1 \| tee target/test-output.log` |
| guest freshness | `cargo xtask build-guests --check` (inspect the exit code; never grep for `STALE:`) |
| type gate | `cargo check --workspace --all-targets` |
| lint gate | `cargo clippy --workspace --all-targets -- -D warnings` |
| literal gate | `cargo xtask check-literals` |

**Feature-gate warning (binding).** `slicer-core`'s support test targets carry `required-features = ["host-algos"]` in `crates/slicer-core/Cargo.toml`. A bare `cargo test -p slicer-core` compiles **zero** of them and prints a clean `ok`. Every `slicer-core` command in this packet includes `--features host-algos`, and the new test target must be registered with the same `required-features`. See `CLAUDE.md` § "Feature-gated test files report green when they don't compile".

## Step Completion Expectations

- The step that adds the two `SupportContactParams` fields must land the field additions and every literal fix-up together; `cargo check --workspace --all-targets` is that step's exit condition, not a later step's discovery.
- The two planner steps are independent of each other and of the `slicer-core` steps; either planner may be verified alone.
- No step may edit `crates/slicer-gcode/src/serialize.rs`.
- The manifest step and the module-read step for a given planner must land together, or the manifest declares a key the module does not read — the exact disposition rule 1 prohibits.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp` — `detect_overhangs` (the `enforce_support_layers` forced branch and the `support_expansion` XY growth) and `PrintObjectSupportMaterial::top_contact_layers` (the overhang-cluster erode-and-measure smallness test borrowed by AC-3).
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` — `TreeSupport::detect_overhangs` (the `support_critical_regions_only` clear-and-keep-cantilevers branch borrowed by AC-2) and `TreeSupport::draw_circles` (the `support_object_first_layer_gap` layer-0 substitution borrowed by AC-5 and AC-6).
- `OrcaSlicerDocumented/src/libslic3r/Slicing.cpp` — `SlicingParameters::create_from_config`, for `support_bottom_z_distance` becoming `gap_object_support`. The deliberately **not** borrowed part is `GCode::collect_layers_to_print`'s zero-means-fall-back-to-top rule; see `design.md` DIV-1.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportParameters.hpp` — `SupportParameters::SupportParameters`, for `gap_xy` and `gap_xy_first_layer`, and for the `support_style` resolution this packet deliberately does not port.

## Context Discipline Notes

- `modules/core-modules/tree-support-planner/src/lib.rs` is long; ranged reads only, anchored on `inflate_model_occupancy` and on the descent-termination site, never a full-file read.
- `crates/slicer-core/src/algos/overhang_annotation.rs` is read in two windows: the `SupportContactParams` declaration plus its `Default` impl, and the body of `detect_support_contacts_with_annotations`.
- Every ledger fact in this packet (key counts, the next free packet number, the `docs/07` inventory, deviation IDs) must be re-derived at point of use, never quoted from here.
