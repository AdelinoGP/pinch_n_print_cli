# Requirements: 189-per-point-speed-factor-carrier

## Packet Metadata

- Grouped task IDs: `TASK-308`
- Backlog source: `docs/specs/deviation-backlog-remediation-plan.md` — the Packet Queue rows for DEV-009 in tranche T3, which the orchestrator split into 9a/9b/9c; this packet is row 9a. **Do not quote that row’s text or any TASK-ID hit count here or anywhere else.** The queue is mutable shared state amended while packets are in flight: this line previously froze a row rendering (`<tbd>-gcode-smoothed-speed-add-intersections`) that is now 0 hits in the plan file, and `packet.spec.md` was corrected a round before this file was — the fix landed in one file and not the other, which is the same propagation failure the drift checker now closes. Re-derive at the moment of use with `rg -n '^\| 9[abc] ' docs/specs/deviation-backlog-remediation-plan.md` (the rows begin at column 1, so `^` must anchor directly on the leading `|`; earlier revisions of this line wrote `^.\\|`, which both consumes that `|` and over-escapes the backslash — measured, 0 hits and exit 1, i.e. a silently-empty re-derivation of exactly the kind this paragraph exists to prevent).
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

### The plan's premise for DEV-009 is falsified in three ways; this packet exists because of the third

`docs/specs/deviation-backlog-remediation-plan.md` describes DEV-009 as "two features in `crates/slicer-gcode/src/emit.rs` (`resolve_feedrate`): (a) smoothed-speed interpolation replacing the flat quantized lookup; (b) `ADD_INTERSECTIONS` mid-segment vertex insertion" and lists them as independent. Grounded against the tree, each clause is wrong:

1. **`resolve_feedrate` contains no quantized lookup.** `DefaultGCodeEmitter::resolve_feedrate` (`crates/slicer-gcode/src/emit.rs`) is a flat per-role `match` selecting a base speed, then `let clamped_factor = speed_factor.clamp(0.05, 5.0); let f_value = base_speed * 60.0 * clamped_factor;` rounded to three decimals. There is no band table in it.
2. **The quantization lives in a guest module.** `modules/core-modules/overhang-classifier-default/src/lib.rs` takes `entity.path.points.iter().filter_map(|p| p.overhang_quartile).max()` — a **whole-entity maximum** — and emits exactly one `EntityMutation::SetSpeedFactor(overhang_speed(q, config) / base)` per entity. `quartile_for_distance` buckets through `BAND_BOUNDARY_MULTIPLIERS: [f32; 3] = [0.5, 1.0, 1.5]`.
3. **PnP therefore carries one scalar speed factor per *entity*, and has no per-point speed resolution at all.** `ExtrusionPath3D::speed_factor` is a single `f32`; the emitter's per-point loop calls `self.resolve_feedrate(role, entity.path.speed_factor)` with the same constant for every point of the entity. Smoothed speed is not merely unimplemented — it is *inexpressible*. That missing prerequisite is what this packet supplies, and it is why queue row 9 is three packets rather than the plan's two independent features.

### The two features are not independent either

Canonical `ExtrusionQualityEstimator::estimate_extrusion_quality` (`GCode/ExtrusionProcessor.hpp`) derives `smallest_distance_with_lower_speed` from the `speed_sections` table — feature (a)'s data — and passes it as the `min_distance` argument to `estimate_points_properties`; the returned `extended_points`, including every vertex feature (b) inserted, is exactly the list feature (a)'s `calculate_speed` is evaluated over. Verified against the checkout, including the `if (!found) smallest_distance_with_lower_speed = -1.f;` fallback that makes (a) degrade gracefully at the original vertex density. The coupling is a resolution amplifier rather than a correctness dependency, but the order is fixed: **carrier (189) → smoothed speed (190) → mid-segment insertion (191)**.

### Why this packet is deliberately a no-behaviour-change packet

The only live producer of speed mutations is `overhang-classifier-default`, and it emits one factor per entity. Until packet 190 changes that producer, every point of every entity resolves the same factor and no emitted G-code moves. That is intentional: it lets the carrier land against the full existing golden/regression wall with the "absent profile ⇒ identical output" rule as its safety net, instead of landing the carrier and a behaviour change together. Most of this packet's criteria are consequently do-not-regress guards, and `packet.spec.md` labels each one as such; AC-1, AC-2, AC-3, AC-4, AC-5, AC-6, AC-11, AC-N1, AC-N2 and **AC-N3** are the change-proving set. (AC-N3 belongs in that list and an earlier revision of this line omitted it, contradicting §Acceptance Summary below and `design.md` §Locked Assumptions, which names AC-N3 as the criterion that actually defends the absent-profile compatibility lock.)

## In Scope

- `crates/slicer-ir/src/slice_ir.rs`: new `pub struct EntitySpeedProfile { pub entity_id: u64, pub factors: Vec<f32> }`; new `#[serde(default)] pub speed_profiles: Vec<EntitySpeedProfile>` on `LayerCollectionIR`; `speed_profiles: Vec::new()` in `LayerCollectionIR`'s explicit `Default` impl; `CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION` takes an **additive minor bump** — same major, minor + 1, patch 0, computed at activation from Step 0’s `target/pin-layer-collection-schema-before.txt` pin and written as a literal nowhere in this packet (the constant is mutable shared state; a frozen target silently becomes a wrong instruction). Precedent: `docs/02_ir_schemas.md` records the precedent verbatim as "packet 125 added the additive `PrintEntity.tool_index: u32` field (region_id↔tool split), bumping 1.0.0→1.1.0").
- `crates/slicer-ir/src/lib.rs`: re-export `EntitySpeedProfile` next to the existing `TravelMove` re-export.
- The struct-literal blast radius of that field: the `implementation-plan.md` Step 3 re-derivation command reports **50 hits across 27 files** that do not already end in `..Default::default()` (enumerated in Steps 3 and 4). **That raw count is an over-count and the file list is a strict superset.** Its regex `LayerCollectionIR\s*\{` also matches the struct definition, the `impl Default for LayerCollectionIR {` header, and every `-> [path::]LayerCollectionIR {` return-type brace — none of which is a struct literal needing an inserted field. Treat both numbers as ledger facts and re-derive them; treat the file list as an over-broad worklist and `cargo check --workspace --all-targets` (the `E0063` set) as the authority on which sites actually need the edit.
- `crates/slicer-schema/wit/deps/world-finalization/world-finalization.wit`: additive `set-point-speed-factors(list<f32>)` case on `variant entity-mutation`.
- `crates/slicer-sdk/src/traits.rs`: additive `EntityMutation::SetPointSpeedFactors(Vec<f32>)`; the `MergeOp::ModifyEntity` arm of `FinalizationOutputBuilder::apply_to` gains a branch that upserts an `EntitySpeedProfile` row into the owning layer's `speed_profiles`, keyed by `entity_id`, and rejects a length mismatch with an `Err` naming both lengths.
- `crates/slicer-wasm-host/src/host.rs`: `WitEntityMutation::SetPointSpeedFactors(Vec<f32>)` plus the `fm::EntityMutation` → `WitEntityMutation` match arm.
- `crates/slicer-wasm-host/src/dispatch.rs`: the `WitEntityMutation` → `slicer_sdk::traits::EntityMutation` translation arm.
- `crates/slicer-macros/src/lib.rs`: the `#[slicer_module]` guest-side `slicer_sdk::traits::EntityMutation` → WIT `EntityMutation` translation arm.
- `crates/slicer-gcode/src/emit.rs`: a `speed_profiles_by_entity: HashMap<u64, &Vec<f32>>` lookup built alongside the existing `travel_moves_by_entity`; the `kept` remap in the simplification block changed to carry each surviving point's **original index**; the per-point `f:` field resolved as `self.resolve_feedrate(role, factor_for(original_index))` with fallback to `entity.path.speed_factor`.
- New tests: `per_point_speed_profile_varies_f_within_one_entity`, `per_point_speed_profile_indexes_original_points_after_simplification` and `unprofiled_entity_in_a_profiled_layer_keeps_whole_entity_speed` (`crates/slicer-gcode/tests/gcode_feedrate_emission_tdd.rs`); `modify_entity_set_point_speed_factors_applies` and `modify_entity_set_point_speed_factors_length_mismatch_errors` (`crates/slicer-sdk/tests/finalization_builder_tdd.rs`); a `speed_profiles` assertion added to the existing `modify_entity_set_speed_factor_applies`.
- Doc edits enumerated in `packet.spec.md` §Doc Impact Statement.

## Out of Scope

- **Any change to `modules/core-modules/overhang-classifier-default/src/lib.rs`.** It keeps emitting one `SetSpeedFactor` per entity. Packet 190 (TASK-309) replaces that.
- **Mid-segment vertex insertion and any path-geometry mutation channel.** `EntityMutation` gains no geometry case here; packet 191 (TASK-310) adds one.
- **A per-point speed field on `Point3WithWidth`.** Rejected on a blast radius in the hundreds of exhaustive struct literals across well over a hundred files, plus a `point3-with-width` WIT record change. **Treat the size as a ledger fact and re-derive it** with `rg -c 'dist_to_top_mm:' --glob '*.rs' crates modules xtask` (the proxy count, since every exhaustive literal names that field once). See `design.md` §Code Change Surface.
- **Any new config key.** `enable_overhang_speed` and `slowdown_for_curled_perimeters` belong to packet 190.
- **The four-band `overhang_quartile` schedule** (`BAND_BOUNDARY_MULTIPLIERS` in `crates/slicer-core/src/algos/overhang_annotation.rs`). Recorded in the `DEV-009` row as an accepted permanent deviation; untouched here and in 190.
- **Whole-output G-code byte comparison as a verification technique.** `DEV-093` records that two runs of the same unmodified release binary on the same input already differ by ~100-160 lines, so no criterion in this packet uses it.

## Authoritative Docs

- `docs/02_ir_schemas.md` — 2157 lines; **ranged reads only** (§"IR 10 — LayerCollectionIR" and its normative `LayerCollectionIR::default()` contract paragraph). Delegate anything wider.
- `docs/05_module_sdk.md` — large; delegated grep only. The `modify_entity` variant table is the only section in scope.
- `docs/03_wit_and_manifest.md` — delegated grep only; the `entity-mutation (variant)` bullet.
- `docs/07_implementation_status.md` — **always delegate.** Only needed to register `TASK-308` outside the `open-deviations` generated block.
- `docs/DEVIATION_LOG.md` — very large rows; delegated grep only. The `DEV-009` row is read for scope and is **not** edited by this packet.
- `CLAUDE.md` §"Guest WASM Staleness" — the WIT/`slicer-ir`/`slicer-sdk`/`slicer-macros` edits are all guest-build inputs.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/GCode/ExtrusionProcessor.hpp` — `ExtrusionQualityEstimator::estimate_extrusion_quality`, borrowed only for the shape of the contract this carrier must be able to express: canonical computes **one speed per point pair** and writes it into a per-point structure, which is why a whole-entity `speed_factor` scalar cannot be the target representation. The interpolation itself is deliberately **not** borrowed here — that is packet 190.
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `GCode::_extrude`, for the fact that canonical emits a fresh `F` token per extrusion sub-segment rather than one per path, confirming that a per-point `F` is the canonical emission shape and not a PnP invention.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-11`.
  - Change-proving (measured FAIL on the unfixed tree): `AC-1`, `AC-2`, `AC-3`, `AC-5`, `AC-6`, `AC-11`. `AC-4` is change-proving by construction (its test does not exist yet).
  - Do-not-regress (measured PASS today, must stay PASS): `AC-7` (`gcode_feedrate_emission_tdd` 9 passed, `golden_emit_tdd` 1 passed), `AC-8` (whole `slicer-gcode` crate, 16 summary lines), `AC-9` (`ir_tests` 44 passed), `AC-10` (`build-guests --check` clean).
- Negative: `AC-N1` (length-mismatch rejection at the SDK contract boundary), `AC-N2` (`SetSpeedFactor` must not be re-implemented as an expanded per-point profile), `AC-N3` (a layer holding one profiled and one un-profiled entity — the mixed state packet 190 creates on every slice, and the only case where the `speed_profiles_by_entity` lookup miss is exercised).
- Cross-packet impact: packets 190 and 191 both consume `EntityMutation::SetPointSpeedFactors` and `LayerCollectionIR.speed_profiles`. AC-N1's asymmetry note — `dispatch.rs` swallows `modify_entity`'s `Err` with `log::warn!`, so a length mismatch is a logged no-op at the WASM boundary — is a constraint on packet 190's producer, which must size its profile to `path.points.len()` exactly.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo check --workspace --all-targets` | Whole struct-literal blast radius compiles; the E0063 list is the authoritative sweep exit | FACT pass/fail; on failure, SNIPPETS of the E0063 file list only |
| `cargo clippy --workspace --all-targets -- -D warnings` | Lint gate over all targets including tests | FACT pass/fail |
| `bash -c 'cargo test -p slicer-ir --test ir_tests 2>&1 \| rg "^test result:" \| rg -q "^test result: ok\. [1-9]" && echo PASS \|\| echo FAIL'` | Schema-bump test fallout (`Default().schema_version` vs the constant) | FACT PASS/FAIL |
| `bash -c 'cargo test -p slicer-sdk --test finalization_builder_tdd 2>&1 \| rg "^test result:" \| rg -q "^test result: ok\. [1-9]" && echo PASS \|\| echo FAIL'` | AC-4, AC-N1, AC-N2 home binary | FACT PASS/FAIL |
| `bash -c 'cargo test -p slicer-gcode --test gcode_feedrate_emission_tdd 2>&1 \| rg "^test result:" \| rg -q "^test result: ok\. [1-9]" && echo PASS \|\| echo FAIL'` | AC-5, AC-6 plus the nine pre-existing feedrate tests | FACT PASS/FAIL |
| `bash -c 'cargo test -p slicer-gcode --test golden_emit_tdd 2>&1 \| rg "^test result:" \| rg -q "^test result: ok\. [1-9]" && echo PASS \|\| echo FAIL'` | Golden must not move (absent-profile compatibility rule) | FACT PASS/FAIL |
| `bash -c 'cargo test -p slicer-gcode 2>&1 \| tee target/test-output.log \| rg "^test result:" > target/guard-ac8-gcode.txt; rg -q "[1-9][0-9]* failed\|^test result: FAILED" target/guard-ac8-gcode.txt && echo "FAIL: see target/test-output.log" \|\| (rg -q "^test result: ok\. [1-9]" target/guard-ac8-gcode.txt && echo PASS \|\| echo "FAIL: zero tests ran")'` | Whole-crate emit regression sweep (multi-result-line form) | FACT PASS/FAIL; on FAIL read `target/test-output.log`, never re-run |
| `bash -c 'cargo xtask build-guests --check 2>&1 \| rg -q "STALE:" && echo "FAIL: stale guests" \|\| echo PASS'` | WIT/SDK/IR/macros edits are guest-build inputs | FACT PASS/FAIL |
| `bash -c 'cargo test -p slicer-runtime --test executor 2>&1 \| rg "^test result:" \| rg -q "^test result: ok\. [1-9]" && echo PASS \|\| echo FAIL'` | Finalization deep-copy / mutation-roundtrip bucket, the largest slice of the blast radius | FACT PASS/FAIL |

## Step Completion Expectations

- Steps 3 and 4 (the struct-literal sweep) are purely mechanical and must land **before** any narrow test run is trusted: until every `LayerCollectionIR` literal compiles, every test binary in the workspace is a build failure, and a "test failure" attributed to the carrier design would in fact be an unswept literal.
- The WIT edit (Step 5) and the guest rebuild are a single unit: `cargo xtask build-guests --check` must be run — and, if it reports `STALE:`, the rebuild performed — before any component, dispatch, or module-dispatch test result from a later step is interpreted. `CLAUDE.md` forbids attributing such a failure to anything else until `--check` returns clean.
- The emit-side change (Step 6) depends on Step 5 only for the enum variant's existence, not for behaviour; it can be validated with a hand-constructed `speed_profiles` row and no guest involvement.
- `target/guard-ac8-gcode.txt` is the only shared scratch file; its key is unique to AC-8 and the identical §Verification command so no other criterion can overwrite it.

## Context Discipline Notes

- `docs/02_ir_schemas.md` is 2157 lines — read only §"IR 10 — LayerCollectionIR" through the start of §"IR 11 — GCodeIR". Never load it whole.
- `docs/DEVIATION_LOG.md`'s `DEV-009` row is a single multi-thousand-word table cell. Delegate any question about it; do not read the row into the implementer's context.
- `crates/slicer-gcode/src/emit.rs` is long (over 1200 lines). The only regions in scope are `DefaultGCodeEmitter::resolve_feedrate`, the `travel_moves_by_entity` map construction, the `kept`/`simplified_points` remap block, and the `GCodeCommand::Move { … f: … }` push. Locate by symbol, then open a ±40-line window.
- `crates/slicer-sdk/src/traits.rs` is long. The two regions in scope are the `pub enum EntityMutation` declaration and the `MergeOp::ModifyEntity` arm inside `apply_to`.
- The struct-literal sweep must **not** be done by reading the files. Run the re-derivation command in `implementation-plan.md` Step 3 to get the file:count list — remembering it over-counts (definition, `impl Default` header, `-> LayerCollectionIR {` return types) — then edit each real literal blind (one inserted line) and let `cargo check --workspace --all-targets` be the oracle.
