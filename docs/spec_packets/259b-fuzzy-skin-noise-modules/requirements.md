# Requirements: fuzzy-skin-noise-modules

## Packet Metadata

- **Packet directory:** `docs/spec_packets/259b-fuzzy-skin-noise-modules/`
- **Slug:** `fuzzy-skin-noise-modules`
- **Status:** `draft`
- **Task IDs:** none (queue packet — `task_ids: []`)
- **Backlog source:** wayfinder ticket 14, map packet P07
- **Tier:** **C** — re-derived. Builds decision points (map Authoring rule 1 forbids Tier A for such a packet) across five new core-module crates plus one new library crate and a generalization of the scheduler's claim-dedup path. Above the Tier B single-module ceiling.
- **Re-authoring note:** this packet plus `259a-fuzzy-skin-gate-and-mode-keys` replace the single `259-fuzzy-skin-keys` draft, with explicit user approval for the split.

## Problem Statement

OrcaSlicer's fuzzy skin can be driven by six different noise sources. Five of them — Perlin, Billow, RidgedMulti, Voronoi, and the arc-length Ripple — are *different algorithms*, not different parameter values, and canonical selects between them in `get_noise_module` (`src/libslic3r/Feature/FuzzySkin/FuzzySkin.cpp`). Three further keys parameterise them: `fuzzy_skin_scale` (as `SetFrequency(1 / scale)`), `fuzzy_skin_octaves`, and `fuzzy_skin_persistence`.

This tree has one noise source: a xorshift32 `Rng::next_f32` returning `[-1.0, 1.0]`, sampled once per resampled vertex as `rng.next_f32() * fuzzy_skin_thickness` inside `apply_fuzzy_skin` (`modules/core-modules/fuzzy-skin/src/lib.rs`). That is a faithful match for canonical's `classic` (`UniformNoise::GetValue`, `random_value() * 2 - 1`) and nothing else. None of the four noise keys is read anywhere.

The prior revision declared all four with-gap. Map Authoring rule 1 prohibits that, and rule 4 names the right shape: an Orca enum whose values are different algorithms is a set of `claim:*` holders, one per shipped value, selected through the existing selection mechanism. This packet builds exactly that.

## Key Disposition Table

Classification per the map's Authoring rules: **(a)** live behaviour-changing decision point already in tree; **(b)** decision point this packet builds; **(c)** returned to queue; **(d)** dead-in-canonical.

| Key | Class | Owner | Decision point this packet builds | Non-default AC |
| --- | --- | --- | --- | --- |
| `fuzzy_skin_noise_type` | **(b)** | host (load-time dedup) | value→`claim:fuzzy-skin-generator` holder mapping over six modules | AC-2, AC-N3 |
| `fuzzy_skin_scale` | **(b)** | the four coherent-noise modules | noise frequency (`1 / scale`) — lower frequency, smoother displacement | AC-3 |
| `fuzzy_skin_octaves` | **(b)** | `perlin`, `billow`, `ridged-multi` | fractal octave count — added high-frequency detail | AC-4, AC-5 |
| `fuzzy_skin_persistence` | **(b)** | `perlin`, `billow` | per-octave amplitude falloff | AC-4 |

Counts for this packet's four keys: **(a) 0 · (b) 4 · (c) 0 · (d) 0.** Zero declaration-only keys (map preflight gate (a)); every key carries at least one AC asserting a behaviour change at a non-default value (map preflight gate (b)).

**Three keys outside ticket 14** are additionally declared and wired here because the user's ruling is to ship the `ripple` value, which is meaningless without them: `fuzzy_skin_ripples_per_layer`, `fuzzy_skin_ripple_offset`, `fuzzy_skin_layers_between_ripple_offset` — all class **(b)**, all covered by AC-7. **Ticket 14's key list must gain these three;** the update is reported in the session handoff and is not applied by this packet.

Ticket 14's remaining three keys are elsewhere: `fuzzy_skin`, `fuzzy_skin_first_layer`, `fuzzy_skin_mode` belong to packet 259a. `fuzzy_skin_thickness` and `fuzzy_skin_point_distance` were already live before either packet and are claimed as coverage by neither.

## Returned to Queue — unimplemented

**None.** All four of this packet's ticket-14 keys are implemented, and all six canonical `fuzzy_skin_noise_type` values ship, so no value is rejected as unimplemented either.

## Ruled Dead-in-Canonical

**None.** Every fuzzy-skin key — including all four here and the three ripple keys — is read inside OrcaSlicer's slicing pipeline at a single site: `group_region_by_fuzzify` (`src/libslic3r/Feature/FuzzySkin/FuzzySkin.cpp`) builds `FuzzySkinConfig` (`PerimeterGenerator.hpp`) from the region config, and that struct is consumed by `get_noise_module`, `fuzzy_polyline`, `fuzzy_extrusion_line`, and the ripple variants. The only non-pipeline mentions are `PrintObject::invalidate_state_by_config_options` (a key-name list) and preset plumbing.

## In Scope

1. **`fuzzy-skin-core`** — new library crate (not a module) holding the resampling walk, `should_fuzzify`, and the `fuzzy_skin_mode` switch that packet 259a builds, re-exported behind a noise trait so each generator module supplies only a sampling function. Also home to the shared manifest guard `tests/noise_config_schema_tdd.rs`.
2. **`perlin-fuzzy-skin`**, **`billow-fuzzy-skin`**, **`ridged-multi-fuzzy-skin`**, **`voronoi-fuzzy-skin`** — four new core-module crates, `LayerModule` at `Layer::PerimetersPostProcess`, each holding `claim:fuzzy-skin-generator`, each porting its libnoise algorithm to Rust and honouring exactly the parameters canonical gives it.
3. **`ripple-fuzzy-skin`** — new core-module crate on the same contract, porting canonical `fuzzy_polyline_ripple` / `fuzzy_extrusion_line_ripple` / `ripple_phase_shift_rad` / `ripple_anchor_arc_mm`: an arc-length-driven sine that consults no noise module and ignores scale, octaves, and persistence.
4. **`fuzzy-skin` becomes the `classic` holder** — it gains `claim:fuzzy-skin-generator` and delegates to `fuzzy-skin-core`, keeping its existing `Rng` as the uniform sampler. Its output for a given seed must not change.
5. **Selection-key claim dedup** — generalize `dedup_same_claim_modules_with_wall_generator` (`crates/slicer-scheduler/src/execution_plan.rs`; its one non-test call site is `load_live_modules_for_plan_with_integrated` in `crates/slicer-wasm-host/src/execution_plan_live.rs`, which holds the raw `config_source` map and does the key extraction) so it resolves any claim registered with a `(claim_id, config_key, default_value, value→module map)` entry, then register both `perimeter-generator`/`wall_generator` (unchanged behaviour) and `claim:fuzzy-skin-generator`/`fuzzy_skin_noise_type`. Unknown values are rejected rather than falling back.
6. **Manifest ownership** — the three noise parameters declared identically on every module that reads them; the three ripple keys on `ripple-fuzzy-skin` only; `fuzzy_skin_noise_type` declared in **no** manifest (host-side selection key, `wall_generator` precedent).
7. **Registration** — workspace members in the root `Cargo.toml`; `crates/slicer-integrated-modules/Cargo.toml` optional deps + features; `crates/slicer-integrated-modules/src/lib.rs` `manifest_const!` + `integrated_registry!` entries and their `#[cfg(not(feature = …))]` arms; `crates/pnp-cli/Cargo.toml` `integrated-<name>` passthrough features.
8. **Docs** — the `claim:fuzzy-skin-generator` row in `docs/03_wit_and_manifest.md` § Known claim IDs; the generalized selection-key subsection in `docs/04_host_scheduler.md` § Claim Resolution; regeneration of `docs/15_config_keys_reference.md`.

## Out of Scope

- **`fuzzy_skin`, `fuzzy_skin_first_layer`, `fuzzy_skin_mode`** — packet 259a's scope. This packet consumes the gate and mode logic 259a builds; it does not re-specify them.
- **Vendoring libnoise.** Canonical links an external CMake dependency; this port implements the four algorithms in Rust. Byte-for-byte equality with libnoise is explicitly not a goal — see `design.md` DIV-2.
- **Painted-region fuzzy promotion** — returned to the queue by packet 259a.
- **`ORCA_CONFIG_PADDING`** — untouched (AC-N2, map Authoring rule 2).
- **Changing `classic`'s output.** `fuzzy-skin`'s existing `Rng` and seeding stay exactly as they are so AC-N1 and 259a's tests remain valid.

## Authoritative Docs

- `docs/01_system_architecture.md` § Claim System — the normative claim concept and the Allowed Claim Transition Matrix.
- `docs/04_host_scheduler.md` § Claim Resolution — the `wall_generator` dedup subsection is the precedent being generalized; the doc gains this packet's subsection.
- `docs/03_wit_and_manifest.md` § Known claim IDs — gains one row.
- `docs/adr/0056-integrated-modules-native-dispatch.md` — registration contract.
- `docs/08_coordinate_system.md` — 1 unit = 100 nm; noise is sampled in millimetres, so conversion happens once at the boundary.
- `docs/15_config_keys_reference.md` — generated.

## Parity Evidence Standard

A key counts as covered only when a non-default value changes emitted geometry, proven by a named test. Default-path identity (AC-N1) is an additional guard and is never sole evidence. Because noise is stochastic, ACs assert **structural** properties (sign-change counts, variance ordering, period counts, byte-identity under a parameter a generator must ignore) rather than golden coordinates — these are falsifiable without a canonical golden file the port cannot produce. Canonical is cited by file + function; in-tree by crate-qualified path + symbol.

## Per-Key Canonical Evidence

Established by delegated reads of the sibling `OrcaSlicerDocumented` checkout during authoring.

- **`fuzzy_skin_noise_type`** — `PrintConfig.cpp` `PrintConfigDef::init_fff_params` with `s_keys_map_NoiseType`: `Classic` (default), `Perlin`, `Billow`, `RidgedMulti`, `Voronoi`, `Ripple`. Consumed by `get_noise_module`, which returns `noise::module::Perlin` / `Billow` / `RidgedMulti` / `Voronoi`, or the file-local `UniformNoise` for `classic`. `ripple` never reaches `get_noise_module` — it short-circuits into `fuzzy_polyline_ripple` / `fuzzy_extrusion_line_ripple`.
- **`fuzzy_skin_scale`** — float, default 1.0, min 0.1, max 500. Applied as `SetFrequency(1 / fuzzy_skin_scale)` to **all four** coherent generators. Ignored by `classic` (`UniformNoise` has no frequency) and by `ripple`.
- **`fuzzy_skin_octaves`** — int, default 4, min 1, max 10. `SetOctaveCount` on **Perlin, Billow, RidgedMulti only**. Voronoi ignores it; `classic` and `ripple` ignore it.
- **`fuzzy_skin_persistence`** — float, default 0.5, min 0.01, max 1. `SetPersistence` on **Perlin and Billow only**. RidgedMulti and Voronoi ignore it; `classic` and `ripple` ignore it.
- **Voronoi** additionally receives a fixed `SetDisplacement(1.0)`.
- **The sample site** — `noise->GetValue(unscale_(pa.x()), unscale_(pa.y()), slice_z) * cfg.thickness` inside `fuzzy_polyline` and `fuzzy_extrusion_line`; the coordinates are millimetres and `slice_z` is the layer's slice Z in millimetres (from `LayerRegion::make_perimeters`).
- **`classic`** — `UniformNoise::GetValue` returns `random_value() * 2 - 1`, where `random_value` is a thread-local mt19937 uniform `[0, 1)`. It takes no frequency, octave, or persistence setter, which is the canonical basis for AC-8's honest-absence assertion.
- **Ripple** — `fuzzy_polyline_ripple` / `fuzzy_extrusion_line_ripple` displace by an arc-length sine driven by `thickness`, `point_distance`, and the three ripple keys; `ripple_phase_shift_rad` and `ripple_anchor_arc_mm` compute the phase. No noise module is consulted.
- **Merging** — `collect_merged_fuzzy_regions` / `same_fuzzy_effect` treat two regions as the same noise when their noise parameters match, ignoring `type` and `first_layer`. Informative for a future region-merging optimisation; not ported here.

## In-Tree Grounding (verified at authoring, 2026-09-01)

- `apply_fuzzy_skin` (a free fn in `modules/core-modules/fuzzy-skin/src/lib.rs`) samples noise at exactly **two** expressions, both spelled `let displacement = rng.next_f32() * fuzzy_skin_thickness;` — one in the `while dist < seg_len` sampling loop, one in the `if !emitted_sample` fallback branch. **Both** are the seam this packet replaces with a generator call; replacing only the first silently leaves short segments on the uniform sampler.
- `Rng::next_f32` returns `[-1.0, 1.0]` via xorshift32 — the same range as canonical `UniformNoise::GetValue`. `classic` is therefore already correct and must not be changed.
- `fuzzy-skin.toml` declares `holds = []` and `[stage] Layer::PerimetersPostProcess`, reads and writes `PerimeterIR`.
- **The selection precedent exists and needs no new `ResolvedConfig` field.** `WALL_GENERATOR_CONFIG_KEY` (`= "wall_generator"`) and `DEFAULT_WALL_GENERATOR` (`= "classic"`) are `pub const`s in `crates/slicer-scheduler/src/execution_plan.rs`, alongside `dedup_same_claim_modules_with_wall_generator`, which resolves the `perimeter-generator` claim by reading the key **directly from the raw config source at module-load time, before `ResolvedConfig` exists** (`docs/04_host_scheduler.md` § "Perimeter-generator selection"). Both consts are re-exported from `crates/slicer-scheduler/src/lib.rs` and `crates/slicer-runtime/src/lib.rs`. This is why the packet needs no new host config field.
- Existing claim IDs in `docs/03_wit_and_manifest.md` § Known claim IDs include `perimeter-generator`, `claim:top-fill` … `claim:sparse-fill`, `claim:ironing`, `claim:infill-link`. `claim:fuzzy-skin-generator` is **net-new**.
- The scheduler's integration test target is **`scheduler_integration`**; its aggregator `tests/integration/main.rs` uses `mod <name>;` declarations. `slicer-runtime`'s `e2e` target is auto-discovered from `tests/e2e/main.rs` and contains `slice_end_to_end_tdd`.
- `load_modules_from_roots` is `pub fn` in `crates/slicer-scheduler/src/manifest.rs`; `crates/slicer-scheduler/tests/integration/manifest_ingestion_tdd.rs` asserts on `report.modules.len()`.

## Acceptance Summary

Authoritative Given/When/Then text lives in `packet.spec.md`. IDs only here.

| AC | Subject | Key(s) covered |
| --- | --- | --- |
| AC-1 | manifest ingestion: five new modules, the shared claim, zero Error diagnostics | `fuzzy_skin_noise_type` |
| AC-2 | six values → six holders; absent → classic; unknown rejected | `fuzzy_skin_noise_type` |
| AC-3 | scale changes frequency (sign-change count falls as scale rises) | `fuzzy_skin_scale` |
| AC-4 | octaves add detail; persistence changes variance | `fuzzy_skin_octaves`, `fuzzy_skin_persistence` |
| AC-5 | billow/ridged distinct from perlin; ridged ignores persistence | `fuzzy_skin_octaves`, `fuzzy_skin_persistence` |
| AC-6 | voronoi responds to scale, ignores octaves and persistence | `fuzzy_skin_scale` |
| AC-7 | ripple period/phase/layer-recurrence; ignores the three noise params | the three ripple keys |
| AC-8 | classic provably ignores all three noise params | honest absence |
| AC-9 | manifest schema; selection key in no manifest | all four |
| AC-10 | bounds enforcement at both ends of all three ranges | the three parameters |
| AC-11 | shared core is actually shared (no per-module copy of the walk) | — |
| AC-12 | claim row + host-scheduler selection subsection | `fuzzy_skin_noise_type` |
| AC-13 | generated config-keys doc; deviation row count unchanged | all four + ripple keys |
| AC-N1 | default path byte-identical (additional guard only) | all four |
| AC-N2 | zero `ORCA_CONFIG_PADDING` diff lines | — |
| AC-N3 | unknown noise type rejected, never silently defaulted | `fuzzy_skin_noise_type` |
| AC-N4 | duplicate-claim safety net still armed behind the dedup | `fuzzy_skin_noise_type` |
| AC-N5 | generator choice never itself enables fuzzing | all four |

## Verification Matrix

| AC | Command |
| --- | --- |
| AC-1 | `cargo test -p slicer-scheduler --test scheduler_integration manifest_ingestion 2>&1 \| tee target/test-output.log \| grep -E "^test result"` |
| AC-2, AC-N3, AC-N4 | `cargo test -p slicer-scheduler --test scheduler_integration fuzzy_skin_generator_selection 2>&1 \| tee target/test-output.log \| grep -E "^test result"` |
| AC-3, AC-4 | `cargo test -p perlin-fuzzy-skin --test perlin_fuzzy_skin_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` |
| AC-5 | `cargo test -p ridged-multi-fuzzy-skin --test ridged_multi_fuzzy_skin_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` plus `cargo test -p billow-fuzzy-skin --test billow_fuzzy_skin_tdd` |
| AC-6 | `cargo test -p voronoi-fuzzy-skin --test voronoi_fuzzy_skin_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` |
| AC-7 | `cargo test -p ripple-fuzzy-skin --test ripple_fuzzy_skin_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` |
| AC-8 | `cargo test -p fuzzy-skin --test fuzzy_skin_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` |
| AC-9, AC-N5 | `cargo test -p fuzzy-skin-core --test noise_config_schema_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` |
| AC-10 | `cargo test -p slicer-scheduler --test scheduler_integration config_bounds_enforcement 2>&1 \| tee target/test-output.log \| grep -E "^test result"` |
| AC-11 | `rg -c 'fuzzy_skin_core::' modules/core-modules/perlin-fuzzy-skin/src/lib.rs modules/core-modules/billow-fuzzy-skin/src/lib.rs modules/core-modules/ridged-multi-fuzzy-skin/src/lib.rs modules/core-modules/voronoi-fuzzy-skin/src/lib.rs modules/core-modules/ripple-fuzzy-skin/src/lib.rs modules/core-modules/fuzzy-skin/src/lib.rs; echo "exit=$?"` |
| AC-12 | `rg -q 'claim:fuzzy-skin-generator' docs/03_wit_and_manifest.md && rg -q 'fuzzy_skin_noise_type' docs/04_host_scheduler.md && rg -q 'ripple-fuzzy-skin' docs/04_host_scheduler.md; echo "exit=$?"` |
| AC-13 | `cargo xtask gen-config-docs --check && rg -q '\`fuzzy_skin_persistence\`' docs/15_config_keys_reference.md && rg -q '\`fuzzy_skin_ripples_per_layer\`' docs/15_config_keys_reference.md; echo "exit=$?"` |
| AC-N1 | `cargo test -p slicer-runtime --test e2e slice_end_to_end 2>&1 \| tee target/test-output.log \| grep -E "^test result"` |
| AC-N2 | `git diff --unified=0 -- crates/slicer-gcode/src/serialize.rs \| grep -cE "^[+-][^+-]"` (expect `0`) |
| Gates | `cargo check --workspace --all-targets`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo xtask check-literals`; `cargo xtask build-guests --check; echo "exit=$?"` |

## Step Completion Expectations

- The five new crates must be registered in all four registration surfaces in the **same** step that creates them, or `cargo check --workspace --all-targets` and `xtask dist`'s per-module passthrough check fail out of band.
- The module-count assertion in `crates/slicer-scheduler/tests/integration/manifest_ingestion_tdd.rs` moves in the same step that lands the five manifests. Re-derive its pre-packet value from disk at that moment.
- Giving six modules the same claim without the dedup generalization in place is a **fatal startup conflict**. The dedup step must land before or with the manifests, never after.
- `fuzzy-skin`'s output for a fixed seed must not change when it is refactored onto `fuzzy-skin-core`. Keep AC-8 and 259a's `fuzzy_skin_tdd` green from that step onward.
- `cargo xtask build-guests --check` must return exit 0 before closure: six guests are affected.

## Context Discipline Notes

- Never load `OrcaSlicerDocumented/` directly. libnoise is not in that tree at all — the four algorithms are ported from their published definitions.
- The five generator ports are independent. Implement and verify one crate at a time; never hold more than one noise algorithm in context at once.
- `docs/15_config_keys_reference.md` is generated; verify with the AC-13 command, never read to author.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Feature/FuzzySkin/FuzzySkin.cpp` — `get_noise_module` (which setter each type receives), `UniformNoise::GetValue`, `random_value`, `fuzzy_polyline` / `fuzzy_extrusion_line` (the sample site and its millimetre coordinates), `fuzzy_polyline_ripple` / `fuzzy_extrusion_line_ripple` / `ripple_phase_shift_rad` / `ripple_anchor_arc_mm`, `collect_merged_fuzzy_regions` / `same_fuzzy_effect`.
- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.hpp` — `struct FuzzySkinConfig`.
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — `PrintConfigDef::init_fff_params`, `s_keys_map_NoiseType`, and the bounds of the three noise parameters and three ripple keys.
- `OrcaSlicerDocumented/src/libslic3r/LayerRegion.cpp` — `LayerRegion::make_perimeters` (the `slice_z` millimetre value used as the sampler's third coordinate).
- libnoise itself is an external CMake dependency (`deps/libnoise/libnoise.cmake`) and is **not** present in the checkout; do not search for its source there.

Note: in this clone the checkout is the sibling `..\pinch_n_print_cli\OrcaSlicerDocumented` (pinned by wayfinder ticket 08's ledger note) — workers must resolve `OrcaSlicerDocumented/` against that absolute sibling path.
