# Implementation Plan: 253-part-cooling-fan-scale-and-cooling-keys

## Execution Rules

- Work one atomic step at a time; every step maps to packet P01 of the OrcaSlicer feature-gap queue (no `TASK-###` rows exist for this queue — packet 234a precedent).
- Use TDD (the invariant test first), then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".
- Guest WASM staleness rule applies from Step 2 onward (see `design.md` Architecture Constraints): after any manifest/source edit, `cargo xtask build-guests --check` precedes any test-failure attribution.

## Steps

### Step 1: Baseline the byte contract and the conversion helper

- Task IDs: none (queue packet P01).
- Objective: pin today's emission bytes as the percent-normalization baseline and add the **three** canonical converters — `percent_to_fan_s` = `floor(255.5 * p / 100)` (`GCodeWriter::set_fan`), `percent_to_additional_fan_s` = `trunc(255.0 * p / 100)` (`set_additional_fan`), `percent_to_exhaust_fan_s` = `trunc(p / 100.0 * 255)` (`set_exhaust_fan`) — each with its exhaustive 101-value formula test, plus a test asserting they disagree for at least one percent so a future collapse into one helper fails loudly (AC-2, AC-N3).
- Precondition: clean tree; `cargo xtask build-guests --check` exit 0; `cargo test -p part-cooling` green before any edit.
- Postcondition: all three converters exist (module-private inside their guest crates; the exhaust one lives in `machine-gcode-emit`, the other two in `part-cooling` — they are not shared across crates and must not be extracted into a common crate by this packet), exhaustive tests red-green complete; no behaviour change yet.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/part-cooling/src/lib.rs` - full (176 lines)
  - `crates/slicer-runtime/tests/integration/gcode_part_cooling_emission_tdd.rs` - full (~240 lines) — the byte contract to preserve
- Files allowed to edit (at most 3):
  - `modules/core-modules/part-cooling/src/lib.rs`
  - `modules/core-modules/part-cooling/tests/cooling_curve_parity_tdd.rs` (new)
- Files explicitly out of bounds:
  - All host crates; the other test fixtures (touched later, by the steps that change their inputs); `OrcaSlicerDocumented/**` (delegated).
- Blast-radius discipline: no struct field or schema constant changes in this step (helper + tests only) — the struct-field blast radius for `PartCooling` is Step 4's and is pre-enumerated there.
- Expected sub-agent dispatches:
  - Question: list every test asserting `M106`/`M107` byte sequences tied to part-cooling output; scope: `crates/slicer-runtime/tests/**` + `modules/core-modules/part-cooling/tests/**`; return: `LOCATIONS`; purpose: confirm the fixture blast-radius list matches `requirements.md` §Step Completion Expectations before any edit.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/orca-feature-gap/issues/02-parity-evidence-standard.md` - full (~80 lines)
- OrcaSlicer refs:
  - `GCodeWriter.cpp::set_fan`, `GCodeWriter.cpp::set_additional_fan`, `GCodeWriter.cpp::set_exhaust_fan` - delegate one `SNIPPETS` re-verification covering all three conversions; they use three different constants and must not be unified (DIV-B in `design.md`)
- Verification:
  - `cargo test -p part-cooling --test cooling_curve_parity_tdd -- percent_to_s_conversion_matches_canonical_formula_exhaustively 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail
  - `cargo test -p part-cooling 2>&1 | tee target/test-output.log | grep -E "^test result"` - pre-existing suite stays green (no behaviour change yet)
- Exit condition: both commands pass with the three converters compiled in and unused-by-decision-path.

### Step 2: Manifest re-key to percent + the new key declarations in both owners

- Task IDs: none (queue packet P01).
- Objective: make `part-cooling` declare its 15 P01 keys (2 re-keyed to percent, 13 new) plus the role base-speed keys Step 7 reads, and make `machine-gcode-emit` declare the 4 header/footer keys plus the supporting `chamber_temperature`. The four header/footer keys are declared **only** in `machine-gcode-emit` — `part-cooling` never reads them, and declaring them there would be the declaration-only disposition the map prohibits. Update both modules' schema test expectations (AC-1, AC-1b).
- Precondition: Step 1 complete.
- Postcondition: both manifests parse (`read_config_schema`); `part-cooling` declares 21 cooling keys plus its speed block and `machine-gcode-emit` declares 19; schema tests updated to those lists; module source still compiles (it reads 4 keys, two of which changed scale — see Step 3 before running behavioural tests that assert S-values from defaults).
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/part-cooling/part-cooling.toml` - full (89 lines)
  - `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml` - `[config.schema]` section only (delegated LOCATIONS for its line range first)
  - `modules/core-modules/part-cooling/tests/cooling_config_schema_tdd.rs` - full (small)
- Files allowed to edit (at most 3):
  - `modules/core-modules/part-cooling/part-cooling.toml`
  - `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml`
  - `modules/core-modules/part-cooling/tests/cooling_config_schema_tdd.rs`
  (plus `modules/core-modules/machine-gcode-emit/tests/machine_gcode_emit_tdd.rs` if and only if its declaration-count assertion fails — a mechanical count update, no behaviour)
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/serialize.rs` (`ORCA_CONFIG_PADDING` is hand-maintained; new keys reach the block via the raw-config passthrough — editing it is not authorized by this packet); any host crate.
- Blast-radius discipline: `cooling_config_schema_tdd.rs` hard-asserts the 8 current keys and their defaults — this step rewrites its expectations; `machine-gcode-emit`'s own schema assertions (in `machine_gcode_emit_tdd.rs`) hard-assert its 14 and must be re-derived to 19 in the same step. Re-derive both counts from the manifests on disk rather than trusting these numbers. Any other test that parses either manifest is found by the Step-1 LOCATIONS dispatch before editing. Manifest `[compatibility]`/`[ir-access]`/`[claims]` sections untouched — neither module gains or loses a claim.
- Expected sub-agent dispatches: none (all inputs already grounded).
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md` - ranged read around the P01 heading (~15 lines)
- OrcaSlicer refs:
  - `PrintConfig.cpp::PrintConfigDef` declarations - delegate `FACT` for the 11 new keys' defaults/min/max if restating from `requirements.md`'s table is insufficient
- Verification:
  - `cargo test -p part-cooling --test cooling_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - AC-1 declarations; FACT
  - `cargo xtask gen-config-docs --check 2>&1 | tail -3` - FACT (must PASS after regeneration)
  - `cargo xtask build-guests --check; echo "exit=$?"` - fresh-guest gate; rebuild (no `--check`) if stale, then re-verify
- Exit condition: all three green; `machine-gcode-emit` still builds its own tests (`cargo test -p machine-gcode-emit 2>&1 | tee target/test-output.log | grep -E "^test result"`).

### Step 3: Percent normalization of the module's live reads (defaults byte-identical)

- Task IDs: none (queue packet P01).
- Objective: switch `PartCooling::from_config` and `layer_fan_speed`/`cooling_decision_for_event` to percent-domain fields, converting to S-values only at `push_fan_speed`/`push_annotation` time via `percent_to_fan_s`; the default-config emission bytes are the Step-1 baseline.
- Precondition: Steps 1–2 complete; guest fresh.
- Postcondition: default-config output byte-identical (`M106 S255`, `M107`, overhang bump S under 100% × 100%); config carrying Orca percent presets no longer mis-scaled; `fan_min_speed` still unread (Step 4 wires it — do not partially wire here).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-sdk/src/traits.rs` - lines `1280-1300` (`push_fan_speed` semantics)
  - `modules/core-modules/part-cooling/tests/part_cooling_tdd.rs` - full
- Files allowed to edit (at most 3):
  - `modules/core-modules/part-cooling/src/lib.rs`
  - `modules/core-modules/part-cooling/tests/part_cooling_tdd.rs` (restate any fixture whose intent was raw-typed into percent terms; expectations for default config must NOT change)
- Files explicitly out of bounds:
  - `crates/slicer-runtime/tests/**` (their fixtures must pass unmodified — that is the point); `crates/slicer-gcode/**`.
- Blast-radius discipline: `PartCooling`'s struct fields change type (`fan_max_speed: u8` → percent domain) — but the struct is crate-private with a two-constructor surface (`from_config`, the `#[slicer_module]` delegate), so the blast radius is this crate's own three test files and the binding test (`slicer_module_binding_tdd.rs` asserts the trait surface, not the fields — no edit expected).
- Expected sub-agent dispatches: none.
- Context cost: `S`
- Authoritative docs: none new.
- OrcaSlicer refs:
  - `GCodeWriter.cpp::set_fan` - already re-verified in Step 1.
- Verification:
  - `cargo test -p part-cooling 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail
  - `cargo test -p slicer-runtime --test integration -- gcode_part_cooling_emission_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - byte-contract regression pin; FACT
  - `cargo test -p slicer-runtime --test contract -- integrated_parity_part_cooling 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT
  - `cargo xtask build-guests --check; echo "exit=$?"` - fresh after source edit
- Exit condition: all four green — absolute proof the re-scale is output-neutral at defaults.

### Step 4: Fan-curve port (layer-time interpolation, ramp, stop/start, role fans)

- Task IDs: none (queue packet P01).
- Objective: port `CoolingBuffer.cpp::apply_layer_cooldown`'s `change_extruder_set_fan` branch chain into the module with a per-layer time estimate from IR geometry; wire `fan_min_speed`, `reduce_fan_stop_start_freq`, `full_fan_speed_layer`, `fan_cooling_layer_time` (plus read `slow_down_layer_time`/`slow_down_min_speed` as inputs); replace the `BridgeInfill`-only bump with role-fan selection + `-1` fallbacks (AC-3, AC-4).
- Precondition: Step 3 complete (percent domain available).
- Postcondition: the branch matrix of AC-3 passes as named invariant tests; role-fan precedence and fallbacks pinned; `fan_min_speed` is read for the first time.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/slice_ir.rs` - lines `2216-2255`, `2340-2420`, `2748-2875` only
  - `crates/slicer-sdk/src/traits.rs` - lines `780-830` (`LayerCollectionView`)
- Files allowed to edit (at most 3):
  - `modules/core-modules/part-cooling/src/lib.rs`
  - `modules/core-modules/part-cooling/tests/cooling_curve_parity_tdd.rs`
- Files explicitly out of bounds:
  - Host crates (no emitter changes — annotations carry the decision); `machine-gcode-emit`.
- Blast-radius discipline: fields added to `PartCooling` (crate-private struct, two constructors, three test files — same radius as Step 3; no external construction site exists; the wasm guest rebuilds from the same source).
- Expected sub-agent dispatches:
  - Question: quote `CoolingBuffer.cpp` `change_extruder_set_fan`'s branch chain verbatim (≤3 snippets × 30 lines); scope: sibling `OrcaSlicerDocumented/src/libslic3r/GCode/CoolingBuffer.cpp`; return: `SNIPPETS`; purpose: implementation-time re-verification of the curve before writing it.
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/orca-feature-gap/issues/08-author-packet-p01-cooling-notes-part-cooling.md` - the 99-amendment note (lines 15-20)
- OrcaSlicer refs:
  - `CoolingBuffer.cpp::apply_layer_cooldown` - the dispatch above; `parse_layer_gcode` external-perimeter gating is NOT ported (recorded gap).
- Verification:
  - `cargo test -p part-cooling --test cooling_curve_parity_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - AC-3/AC-4 suite; FACT
  - `cargo test -p part-cooling 2>&1 | tee target/test-output.log | grep -E "^test result"` - whole module; FACT
  - `cargo xtask build-guests --check; echo "exit=$?"`
- Exit condition: curve suite green; `run_finalization` still returns `Ok` on every layer set (no new error paths); default-config bytes from Step 3 unchanged (`T ≥ F` default layers keep base/max behaviour per fixtures).

### Step 5: Overhang-threshold classifier + fan speedup/kickstart re-timing

- Task IDs: none (queue packet P01).
- Objective: implement `overhang_fan_threshold` classification over `overhang_quartile` bands + roles, and the `fan_kickstart`/`fan_speedup_time`/`fan_speedup_overhangs` re-timing of rising fan commands (AC-5, AC-6).
- Precondition: Step 4 complete (role-fan selection exists to gate).
- Postcondition: threshold maps quartile bands exactly as AC-5 states; rising-fan re-timing honours the overhang gate and the `close_fan` boundary; both flags default-zero/true leaves the Step-4 bytes unchanged.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/slice_ir.rs` - lines `2340-2420` (only if re-checking `overhang_quartile` typing)
- Files allowed to edit (at most 3):
  - `modules/core-modules/part-cooling/src/lib.rs`
  - `modules/core-modules/part-cooling/tests/cooling_curve_parity_tdd.rs`
- Files explicitly out of bounds:
  - Host crates; textual G-code processing (no FanMover-style line rewriter — IR annotations only).
- Blast-radius discipline: same crate-private struct; no schema change (threshold key exists since Step 2).
- Expected sub-agent dispatches:
  - Question: quote `check_overhang_fan`'s threshold comparisons + FanMover re-timing gate (≤2 snippets); scope: sibling `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` + `GCode/FanMover.cpp`; return: `SNIPPETS`; purpose: re-verify the mapping implemented in AC-5/AC-6.
- Context cost: `M`
- Authoritative docs: none new.
- OrcaSlicer refs:
  - `GCode.cpp` (`check_overhang_fan`, `process_layers` FanMover construction); `FanMover.cpp::_process_gcode_line` - via the dispatch.
- Verification:
  - `cargo test -p part-cooling --test cooling_curve_parity_tdd -- overhang_threshold_maps_quartile_bands 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT
  - `cargo test -p part-cooling --test cooling_curve_parity_tdd -- fan_kickstart_and_speedup_retime_rising_fan 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT
  - `cargo test -p part-cooling 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT
  - `cargo xtask build-guests --check; echo "exit=$?"`
- Exit condition: new tests + whole module green; re-timing provably absent under default flags (Step-4 bytes unchanged).

### Step 6: Auxiliary P2 channel + bounds/enum negatives

- Task IDs: none (queue packet P01).
- Objective: implement the `auxiliary_fan`/`additional_cooling_fan_speed` P2 channel using `percent_to_additional_fan_s` (AC-7); extend the scheduler bounds test for the new keys' rejection paths (AC-N1) and the leakage test (AC-N2). The header/footer emission is Step 8, not this step.
- Precondition: Steps 2–5 complete.
- Postcondition: the `M106 P2` channel emits `trunc(255.0 * p / 100)` and nothing at all when `auxiliary_fan` is false; bounds and enum rejections fire for the new keys in both owners.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-scheduler/src/config_resolution.rs` - located by symbol (`ConfigBoundsIndex::check` and its `check_value` / `check_scalar` helpers), not by line pin
- Files allowed to edit (at most 3):
  - `modules/core-modules/part-cooling/src/lib.rs` (P2 channel)
  - `crates/slicer-scheduler/tests/config_bounds_enforcement_tdd.rs` (new negative cases — verify this file's existing name before extending; if absent, the negative cases go in the nearest existing scheduler test file and the substitution is recorded in the step's answer. `crates/slicer-scheduler/tests/` is flat at its root, so the file is its own `--test` binary exactly as AC-N1 names it; no aggregator registration is needed)
  - `crates/slicer-runtime/tests/integration/gcode_part_cooling_emission_tdd.rs` (AC-N2 leakage test appended)
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/serialize.rs` (still read-only); host config-resolution *logic* (tests prove existing enforcement; if enforcement is genuinely missing for a declared enum, STOP and record `[FWD]` in the packet answer rather than patching the host — that is a contract change needing its own scope).
- Blast-radius discipline: no struct fields change; scheduler test extension asserts existing `ConfigResolutionError` variants — no literal churn beyond new test fixtures (`..Default::default()` or the `// exhaustive:` waiver per the churn gate).
- Expected sub-agent dispatches:
  - Question: does `ConfigBoundsIndex::check` enforce manifest-declared enum string values for a newly declared enum key, and with what error variant; scope: `crates/slicer-scheduler/src/config_resolution.rs`; return: `FACT`; purpose: AC-N1 plumbing (resolves the design's first `[FWD]` open question).
- Context cost: `M`
- Authoritative docs:
  - `docs/15_config_keys_reference.md` - regenerated artifact only; verify by grep, never hand-edit.
- OrcaSlicer refs:
  - `GCode.cpp::_do_export` header/footer structure - already summarized in `requirements.md`; no re-read needed for this step.
- Verification:
  - `cargo test -p part-cooling --test cooling_curve_parity_tdd -- auxiliary_fan_emits_p2_channel 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT
  - `cargo test -p slicer-scheduler --test config_bounds_enforcement_tdd -- new_cooling_keys_bounds_enforced 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT
  - `cargo test -p slicer-runtime --test integration -- gcode_part_cooling_emission_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT
  - `cargo xtask build-guests --check; echo "exit=$?"`
- Exit condition: all four green.

### Step 7: Layer-time slowdown stage + `dont_slow_down_outer_wall`

- Task IDs: none (queue packet P01).
- Objective: port canonical `CoolingBuffer::calculate_layer_slowdown` (fed by `CoolingBuffer::parse_layer_gcode`'s adjustability classification) into `PartCooling::run_finalization`, writing speeds through `FinalizationOutputBuilder::modify_entity` + `EntityMutation::SetSpeedFactor` (AC-10, AC-11). This is the step that makes `dont_slow_down_outer_wall`, `slow_down_for_layer_cooling`, `slow_down_layer_time`, and `slow_down_min_speed` read for the first time.
- Precondition: Steps 1-6 complete. In particular Step 4's layer-time estimator exists as a named helper — this step **reuses** it and must not write a second one.
- Postcondition: a layer whose estimate is below `slow_down_layer_time` receives `SetSpeedFactor` mutations reaching the target time; a layer above it receives zero mutations; `slow_down_for_layer_cooling = false` produces zero mutations at any layer time; `OuterWall` entities are excluded exactly when `dont_slow_down_outer_wall` is true; no emitted factor is below `(slow_down_min_speed / base_speed(role)).max(0.05)`.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/overhang-classifier-default/src/lib.rs` - the private `base_speed` and `speed` helpers and the absolute-to-factor conversion feeding `EntityMutation::SetPointSpeedFactors`; read for shape only, they are crate-private and cannot be imported.
  - `crates/slicer-ir/src/feedrate.rs` - `SPEED_KEYS` (the authoritative `*_speed` name list the manifest copied from in Step 2).
  - `crates/slicer-gcode/src/emit.rs` - `DefaultGCodeEmitter::resolve_feedrate`: the role-to-base-speed match the guest helper mirrors, and the `0.05..=5.0` clamp.
  - `docs/adr/0052-per-point-speed-factor-contract.md` - the upsert and clamp contract (short).
- Files allowed to edit (at most 3):
  - `modules/core-modules/part-cooling/src/lib.rs`
  - `modules/core-modules/part-cooling/tests/layer_slowdown_parity_tdd.rs` (new)
- Files explicitly out of bounds:
  - `crates/slicer-gcode/**` and `crates/slicer-ir/**` — the clamp and the emitter's speed table are read, never changed. If the guest table and `SPEED_KEYS` genuinely disagree, STOP and record `[FWD]`; do not "fix" the host.
  - `modules/core-modules/overhang-classifier-default/**` — read-only precedent; this packet does not touch the other finalization writer.
- Blast-radius discipline: `PartCooling` gains config fields for the slowdown keys and the speed block; the struct is crate-private with a two-constructor surface, so the radius is this crate's test files. New test fixtures constructing IR structs must satisfy the struct-literal churn gate (`..` rest or an `// exhaustive: <reason>` waiver) — `cargo xtask check-literals` is run in Step 9 but a violation introduced here is this step's to fix.
- Expected sub-agent dispatches:
  - Question: quote canonical `CoolingBuffer::calculate_layer_slowdown`'s scaling loop and the `parse_layer_gcode` line that clears `adjust_external`; scope: sibling `OrcaSlicerDocumented/src/libslic3r/GCode/CoolingBuffer.cpp`; return: `SNIPPETS` (at most 2, 30 lines each); purpose: the scaling and exclusion semantics, re-verified rather than restated from this packet.
  - Question: does any other module write `EntityMutation::SetSpeedFactor` or `SetPointSpeedFactors` at `PostPass::LayerFinalization`, and in what scheduled order relative to `part-cooling`; scope: `modules/core-modules/**`, `crates/slicer-scheduler/**`; return: `LOCATIONS`; purpose: the composition risk in `design.md` §Risks — a per-point profile written by another module shadows a whole-entity factor for that entity.
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0052-per-point-speed-factor-contract.md` - upsert semantics and the host-side clamp.
- OrcaSlicer refs:
  - `CoolingBuffer.cpp::calculate_layer_slowdown` and `CoolingBuffer.cpp::parse_layer_gcode` - delegated per the dispatch above.
- Verification:
  - `cargo test -p part-cooling --test layer_slowdown_parity_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - AC-10 + AC-11; FACT
  - `cargo test -p part-cooling 2>&1 | tee target/test-output.log | grep -E "^test result"` - no regression in the fan suite; FACT
  - `cargo test -p slicer-runtime --test integration -- gcode_part_cooling_emission_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - default-config emission unchanged (both slowdown bools default off); FACT
  - `cargo xtask build-guests --check; echo "exit=$?"` - fresh after source edit
- Exit condition: all four green, and the composition dispatch has returned — if another module writes per-point profiles to the same entities, the step's answer records how the two carriers compose before the step closes.

### Step 8: Header/footer emission in `machine-gcode-emit`

- Task IDs: none (queue packet P01).
- Objective: synthesize the canonical chamber-temperature and exhaust-fan lines at the `PrintStart` and `PrintEnd` injection sites (AC-1b, AC-8, AC-8b, AC-8c), including the `custom_gcode_sets_temperature` suppression check.
- Precondition: Step 2 complete (the five keys are declared in `machine-gcode-emit.toml`). Independent of Steps 3-7 — it touches a different crate and may run in parallel with them.
- Postcondition: with both bools false (the defaults) the emitted stream is byte-identical to today's; with them on, the four lines appear in canonical order — `M191` before the rendered start template, `M106 P3` last in the start group, `M141 S0` then `M106 P3` after the rendered end template.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/machine-gcode-emit/src/lib.rs` - located by symbol: `INJECTION_POINTS`, `InjectionSite`, `run_gcode_postprocess`, `substitute_placeholders`, `site_lookup`, `reemit_command` (the existing non-template `Raw` write, the `ExtrusionMode` to `M82`/`M83` bridge).
  - `crates/slicer-sdk/src/postpass_builders.rs` - `GcodeOutputBuilder` push methods available for a synthesized `Raw` line.
- Files allowed to edit (at most 3):
  - `modules/core-modules/machine-gcode-emit/src/lib.rs`
  - `modules/core-modules/machine-gcode-emit/tests/exhaust_and_chamber_emission_tdd.rs` (new)
  - `modules/core-modules/machine-gcode-emit/tests/machine_gcode_emit_tdd.rs` (only if its stream assertions need the default-path identity restated; expectations for default config must NOT change)
- Files explicitly out of bounds:
  - `modules/core-modules/part-cooling/**` (the four keys are not its business), every host crate, `crates/slicer-gcode/src/serialize.rs`.
- Blast-radius discipline: the module re-emits the whole command stream today, so an omitted command is a dropped command. Any new emission must be additive at a named site; the existing tests that assert the full default stream are the regression pin and must pass unmodified except for a declaration count.
- Expected sub-agent dispatches:
  - Question: quote canonical `GCodeWriter::set_exhaust_fan` and `GCodeWriter::set_chamber_temperature` verbatim, plus the static helper `custom_gcode_sets_temperature`; scope: sibling `OrcaSlicerDocumented/src/libslic3r/GCodeWriter.cpp` and `GCode.cpp`; return: `SNIPPETS` (at most 3, 30 lines each); purpose: exact command text, scaling, and the suppression scan, re-verified at implementation time.
- Context cost: `S`
- Authoritative docs: none new.
- OrcaSlicer refs:
  - `GCodeWriter.cpp::set_exhaust_fan`, `GCodeWriter.cpp::set_chamber_temperature`, `GCode.cpp::_do_export`, `GCode.cpp::custom_gcode_sets_temperature` - delegated per the dispatch above.
- Verification:
  - `cargo test -p machine-gcode-emit --test exhaust_and_chamber_emission_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - AC-1b, AC-8, AC-8b, AC-8c; FACT
  - `cargo test -p machine-gcode-emit 2>&1 | tee target/test-output.log | grep -E "^test result"` - default-stream regression pin; FACT
  - `cargo xtask build-guests --check; echo "exit=$?"` - fresh after source edit
- Exit condition: all three green, and the default-config stream is provably unchanged (the negative half of AC-8/AC-8b).

### Step 9: Docs regeneration + literal-churn gate + workspace gates + packet close

- Task IDs: none (queue packet P01).
- Objective: regenerate `docs/15_config_keys_reference.md`, run the commit gates, verify AC-9's greps, and reconcile any residual deviation-row fallout.
- Precondition: Steps 1-8 complete.
- Postcondition: all Verification commands green; AC-9's greps pass; the packet is `implemented`-ready (status flip is a swarm-closure act, not this step's).
- Files allowed to read, with ranges when over 300 lines:
  - `docs/15_config_keys_reference.md` - grep only (generated; never reads as prose)
- Files allowed to edit (at most 3):
  - `docs/15_config_keys_reference.md` (via `cargo xtask gen-config-docs`, never by hand)
  - any `docs/*.md` whose fan-scale prose names raw 0–255 (none does today — re-derive before editing)
- Files explicitly out of bounds:
  - `docs/ORCA_CONFIG_REFERENCE.md` (snapshot reference; the map's Notes forbid editing/sizing off it); `docs/DEVIATION_LOG.md` (a row is filed only after the human signs off per ticket 02 — surface, never file, in this packet); every other `docs/spec_packets/**`.
- Expected sub-agent dispatches:
  - Question: run `cargo test -p part-cooling`, `-p machine-gcode-emit`, `-p slicer-scheduler --test config_bounds_enforcement_tdd`, `-p slicer-runtime --test integration -- gcode_part_cooling_emission_tdd`, `-p slicer-runtime --test contract -- integrated_parity_part_cooling` and `cargo xtask gen-config-docs --check`; scope: whole workspace commands as listed; return: `FACT` per command + failing-test names only; purpose: acceptance re-dispatch.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/orca-feature-gap/issues/02-parity-evidence-standard.md` - the sign-off rule for unverifiable findings (~80 lines)
- OrcaSlicer refs: none new.
- Verification (the packet's Verification list, re-dispatched):
  - `cargo check --workspace --all-targets` - FACT pass/fail
  - `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail
  - `cargo xtask gen-config-docs --check 2>&1 | tail -5` - FACT
  - `cargo xtask check-literals 2>&1 | tail -3` - FACT
  - AC-9 grep: `rg -q 'overhang_fan_threshold' docs/15_config_keys_reference.md && rg -q 'support_material_interface_fan_speed' docs/15_config_keys_reference.md` — exit 0
- Exit condition: every command exits 0; deviations (if any surfaced) are presented to the human, never self-filed.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | baseline bytes + the three canonical converters |
| Step 2 | S | manifests + schema tests + guest freshness |
| Step 3 | S | percent normalization, output-neutral |
| Step 4 | M | curve port + invariant suite (deepest step) |
| Step 5 | M | threshold + re-timing |
| Step 6 | M | P2 channel + bounds/enum negatives |
| Step 7 | M | layer-time slowdown stage + outer-wall exclusion |
| Step 8 | S | header/footer emission in machine-gcode-emit |
| Step 9 | S | docs regen + gates |

Aggregate: `M` (within the `M` estimate in `packet.spec.md`; no step is `L`).

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read (note: this queue's packets carry no TASK row; record the implementation against the wayfinder ticket, which is the workflow established by packet 234a).
- Reconcile reopened/superseded status transitions: none — no prior packet is reopened or superseded by this one.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk: the per-entity time model's estimate semantics (acceleration ignored) and the per-layer (not per-point) threshold approximation — both documented in `design.md` §Risks.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` where a workspace gate is named; per-crate test gates name their `--test` binaries explicitly.