# Implementation Plan: 253-part-cooling-fan-scale-and-cooling-keys

## Execution Rules

- Work one atomic step at a time; every step maps to packet P01 of the OrcaSlicer feature-gap queue (no `TASK-###` rows exist for this queue — packet 234a precedent).
- Use TDD (the invariant test first), then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".
- Guest WASM staleness rule applies from Step 2 onward (see `design.md` Architecture Constraints): after any manifest/source edit, `cargo xtask build-guests --check` precedes any test-failure attribution.

## Steps

### Step 1: Baseline the byte contract and the conversion helper

- Task IDs: none (queue packet P01).
- Objective: pin today's emission bytes as the percent-normalization baseline and add the shared `percent_to_fan_s` helper with its exhaustive formula test (AC-2, AC-N3's 101-value sweep).
- Precondition: clean tree; `cargo xtask build-guests --check` exit 0; `cargo test -p part-cooling` green before any edit.
- Postcondition: `percent_to_fan_s` exists (module-private, `pub(crate)`-style visibility inside the guest crate), exhaustive test red-green complete; no behaviour change yet.
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
  - `GCodeWriter.cpp::set_fan` conversion - delegate `SNIPPETS` re-verification of `255.5 * speed / 100.0`
- Verification:
  - `cargo test -p part-cooling --test cooling_curve_parity_tdd -- percent_to_s_conversion_matches_canonical_formula_exhaustively 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail
  - `cargo test -p part-cooling 2>&1 | tee target/test-output.log | grep -E "^test result"` - pre-existing suite stays green (no behaviour change yet)
- Exit condition: both commands pass with the new conversion helper compiled in and unused-by-decision-path.

### Step 2: Manifest re-key to percent + the 11 new key declarations + co-declaration

- Task IDs: none (queue packet P01).
- Objective: make the `part-cooling` manifest declare all 19 keys with Orca-parity defaults/bounds/enums; co-declare the 4 header/footer keys in `machine-gcode-emit`; update the module's schema test expectations (AC-1's declaration list).
- Precondition: Step 1 complete.
- Postcondition: both manifests parse (`read_config_schema`); schema tests updated to the 19-key list; module source still compiles (it reads 4 keys, two of which changed scale — see Step 3 before running behavioural tests that assert S-values from defaults).
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/part-cooling/part-cooling.toml` - full (89 lines)
  - `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml` - `[config.schema]` section only (delegated LOCATIONS for its line range first)
  - `modules/core-modules/part-cooling/tests/cooling_config_schema_tdd.rs` - full (small)
- Files allowed to edit (at most 3):
  - `modules/core-modules/part-cooling/part-cooling.toml`
  - `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml`
  - `modules/core-modules/part-cooling/tests/cooling_config_schema_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/serialize.rs` (`ORCA_CONFIG_PADDING` is hand-maintained; new keys reach the block via the raw-config passthrough — editing it is not authorized by this packet); any host crate.
- Blast-radius discipline: the schema test file hard-asserts the 8 current keys and their defaults — this step rewrites its expectations for 19 keys; no other test parses the manifest tables (verified by the Step-1 LOCATIONS dispatch: schema tests are the only manifest readers). Manifest `[compatibility]`/`[ir-access]` sections untouched.
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

### Step 6: Auxiliary P2 channel, emission-reachability test, bounds/enum negatives

- Task IDs: none (queue packet P01).
- Objective: implement the `auxiliary_fan`/`additional_cooling_fan_speed` P2 channel (AC-7); write the `machine-gcode-emit` placeholder-reachability test (AC-8); extend the scheduler bounds test for the new keys' rejection paths (AC-N1) and the leakage test (AC-N2).
- Precondition: Steps 2–5 complete.
- Postcondition: every AC from packet.spec.md is either implemented-and-tested or explicitly its docs-regen step; no `P2` line when `auxiliary_fan` false; bounds rejections fire.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/machine-gcode-emit/src/lib.rs` - lines `120-260`, `770-830` only
  - `crates/slicer-scheduler/src/config_resolution.rs` - lines `200-340` (`check`, `check_value`, `check_scalar`)
- Files allowed to edit (at most 3 per execution round; this step runs two sequenced rounds — **Round A**: `part-cooling/src/lib.rs` + `crates/slicer-runtime/tests/integration/gcode_part_cooling_emission_tdd.rs` (P2 channel + AC-N2 leakage test); **Round B**: `machine-gcode-emit/tests/cooling_placeholder_reachability_tdd.rs` (new) + `crates/slicer-scheduler/tests/config_bounds_enforcement_tdd.rs` (verify this file's existing name before extending — if absent, the negative cases live in the nearest existing scheduler test file and the rename is recorded in the step's answer)):
  - `modules/core-modules/part-cooling/src/lib.rs` (P2 channel)
  - `modules/core-modules/machine-gcode-emit/tests/cooling_placeholder_reachability_tdd.rs` (new)
  - `crates/slicer-scheduler/tests/config_bounds_enforcement_tdd.rs` (new negative cases — **verified**: `crates/slicer-scheduler/tests/` is flat at its root, so this file is its own `--test` binary exactly as AC-N1 names; no aggregator registration is needed)
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
  - `cargo test -p machine-gcode-emit 2>&1 | tee target/test-output.log | grep -E "^test result"` - reachability; FACT
  - `cargo test -p slicer-scheduler --test config_bounds_enforcement_tdd -- new_cooling_keys_bounds_enforced 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT
  - `cargo test -p slicer-runtime --test integration -- gcode_part_cooling_emission_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT
  - `cargo xtask build-guests --check; echo "exit=$?"`
- Exit condition: all five green.

### Step 7: Docs regeneration + literal-churn gate + workspace gates + packet close

- Task IDs: none (queue packet P01).
- Objective: regenerate `docs/15_config_keys_reference.md`, run the commit gates, verify AC-9's greps, and reconcile any residual deviation-row fallout.
- Precondition: Steps 1–6 complete.
- Postcondition: all Verification commands green; AC-9's greps pass; the packet is `implemented`-ready (status flip is a swarm-closure act, not this step's).
- Files allowed to read, with ranges when over 300 lines:
  - `docs/15_config_keys_reference.md` - grep only (generated; never reads as prose)
- Files allowed to edit (at most 3):
  - `docs/15_config_keys_reference.md` (via `cargo xtask gen-config-docs`, never by hand)
  - `docs/01_project_overview.md` (only if its fan-scale prose names raw 0–255)
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
| Step 1 | S | baseline bytes + conversion helper |
| Step 2 | S | manifests + schema tests + guest freshness |
| Step 3 | S | percent normalization, output-neutral |
| Step 4 | M | curve port + invariant suite (deepest step) |
| Step 5 | M | threshold + re-timing |
| Step 6 | M | P2 channel + reachability + negatives |
| Step 7 | S | docs regen + gates |

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