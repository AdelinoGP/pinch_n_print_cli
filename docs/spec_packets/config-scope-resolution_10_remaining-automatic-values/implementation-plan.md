# Implementation Plan: remaining-automatic-values

## Execution Rules

- Work one atomic step at a time; map every step to `TASK-571`.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Reconcile dependencies and derive the remaining sentinel census

- Task IDs: `TASK-571`
- Objective: verify packet 04/05 exports and emitter handoff, name-reconcile packet 04's FORWARD-DEP net-new `automatic_value_expansion_tdd.rs` test target, derive all registry declarations with a negative numeric default or lower bound, and lock canonical `GCode::_extrude` zero-speed behavior before editing code.
- Precondition: packets 04 and 05 are implemented or available for exact export inspection.
- Postcondition: a bounded record confirms export shapes, global/tool delivery, the canonical formula/guards, and either no Phase-C negative candidate or one precisely named in-scope candidate.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/spec_packets/config-scope-resolution_04_automatic-value-expansion/{design.md,task-map.md}` - exports/exclusions only
  - `docs/spec_packets/config-scope-resolution_05_scope-resolution-module/{design.md,packet.spec.md}` - resolution exports/precedence only
  - `crates/slicer-runtime/src/run.rs` - emitter construction and config handoff only
- Files allowed to edit (at most 3):
  - None; discovery step.
- Files explicitly out of bounds:
  - `OrcaSlicerDocumented/...` direct reads, packets 01–09 edits, plan edits, unrelated code
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - Dispatch a `LOCATIONS` inventory for `ResolvedConfig` struct literals and old-value/schema assertions now; carry every non-FRU site into Step 2 before adding the field.
- Expected sub-agent dispatches:
  - Question: do `ExpansionContext`, `expand_automatic_values`, `ExpansionError` and the resolution module match their FORWARD-DEP shapes, did draft packet 04 land its net-new test target as `crates/slicer-config/tests/automatic_value_expansion_tdd.rs` (otherwise report the exact landed name), and do resolved tool maps reach `with_tool_configs`?; scope: packets 04/05 landed code and emitter construction; return: `FACT` ≤5 lines.
  - Question: derive every assembled declaration with a negative numeric default or lower bound and classify its owner/input phase; scope: live registry channels; return: `LOCATIONS` ≤20.
  - Question: verify zero-speed formula, active-extruder selection, and invalid-flow treatment; scope: canonical `GCode.cpp::GCode::_extrude`; return: `SUMMARY` ≤200 words.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/config-scope-resolution-plan.md` - RC-8, Expansion Phases B/C, row 10
  - `docs/22_test_quality.md` - derived roster and independent oracle rules
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` - `GCode::_extrude`, delegated; never load
- Verification:
  - `rg -n "ExpansionContext|expand_automatic_values|ExpansionError" crates/slicer-config/src crates/slicer-config/tests` - bounded LOCATIONS only.
  - Derived census report containing each negative default, declaration source, and owner - FACT pass/fail.
- Exit condition: PASS only if dependencies match, per-tool config reaches the emitter, canonical behavior is decisive, and every negative default has an owner; otherwise stop with the precise mismatched symbol or unowned key.

### Step 2: Add the typed filament maximum and its census guard

- Task IDs: `TASK-571`
- Objective: add `filament_max_volumetric_speed` to the host/per-filament resolved config surface and land the registry-derived negative-default ownership test.
- Precondition: Step 1 confirms the field is absent, the packet-05 tool-resolution path can carry it, packet 04's FORWARD-DEP net-new automatic-value test target is name-reconciled, and the struct-literal inventory is complete.
- Postcondition: global and per-tool configs carry the finite numeric key; generated maps/docs can observe it; the derived census fails on any unclassified negative default.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/resolved_config.rs` - `declare_resolved_config!`, extractors, and field-generation macro only
  - FORWARD-DEP net-new `crates/slicer-config/tests/automatic_value_expansion_tdd.rs` from draft packet 04 (or the exact name reconciled in Step 1) - registry fixture/assembly helpers only
  - Step-1 `ResolvedConfig` literal/assertion inventory only
- Files allowed to edit (at most 3):
  - `crates/slicer-ir/src/resolved_config.rs`
  - FORWARD-DEP net-new `crates/slicer-config/tests/automatic_value_expansion_tdd.rs` from draft packet 04, using the Step-1 name-reconciled landed path
  - Any one non-FRU `ResolvedConfig` literal site identified by Step 1; split additional sites into repeated Step 2 slices before editing.
- Files explicitly out of bounds:
  - Emitter source/tests, runtime precedence, overhang percent declarations, WIT, schema/version constants
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - Use Step 1's complete `ResolvedConfig` literal/assertion inventory. The declaration macro must generate default/map/equality/hash behavior; update every compiling exhaustive literal in bounded ≤3-file slices, using FRU where semantically correct and `// exhaustive:` only with a real contract reason.
- Expected sub-agent dispatches:
  - Question: after each ≤3-file slice, do all inventoried `ResolvedConfig` literals compile and does the new key round-trip globally/per-tool?; scope: touched files and narrow crates; return: `FACT` ≤5 lines.
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md` - `ResolvedConfig` macro authority and hash invariant
  - `docs/21_data_defaults_and_fixtures.md` - watched literal/FRU policy
- OrcaSlicer refs:
  - None; Step 1 already captured delegated behavior.
- Verification:
  - `cargo test -p slicer-config --all-targets --test automatic_value_expansion_tdd registry_negative_sentinel_census_has_no_unowned_phase_c_candidate -- --exact` - FACT pass/fail; the command uses draft packet 04's expected FORWARD-DEP net-new target name and must use Step 1's reconciled target if it landed differently.
  - `cargo check -p slicer-ir --all-targets` - FACT pass/fail.
  - `cargo xtask check-literals` - FACT pass/fail.
- Exit condition: the new key is typed and observable through resolved global/tool configs, the derived census is green without a complete hand-authored registry roster, and every inventoried literal compiles.

### Step 3: Drive and implement move-context volumetric automatic speed

- Task IDs: `TASK-571`
- Objective: write literal emitter tests red, then resolve zero role speed from active-tool volumetric maximum, per-move width/flow, and the layer's height delta at emission.
- Precondition: Step 2 exposes the resolved key and Step 1's canonical summary confirms the formula and guard semantics.
- Postcondition: real `emit_gcode` output has literal `F6000`/`F9000` values for the specified fixtures, explicit `30.0` mm/s remains `F1800`, and invalid inputs return `GCodeEmitError::Emit` without non-finite output.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/src/emit.rs` - `DefaultGCodeEmitter`, `resolve_feedrate`, height/e-delta/move loop only
  - `crates/slicer-gcode/src/error.rs` - `GCodeEmitError::Emit` only
  - `crates/slicer-gcode/tests/gcode_feedrate_emission_tdd.rs` - emitter fixture and F-extraction helpers only
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/src/emit.rs`
  - `crates/slicer-gcode/tests/volumetric_auto_speed_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-ir/src/feedrate.rs` overhang percent logic, `slicer-config` expansion logic, runtime resolution, WIT, unrelated G-code serialization
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - No struct field or schema constant is added in this step; use FRU for all watched test fixtures.
- Expected sub-agent dispatches:
  - Question: run the exact emitter test binary and return failing test/assertion with ≤20 relevant lines, then final verdict; scope: `slicer-gcode`; return: `FACT` plus bounded `SNIPPETS` on failure.
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/config-scope-resolution-plan.md` - Phase-C emitter ownership
  - `docs/adr/0052-per-point-speed-factor-contract.md` - `resolve_feedrate` as the sole factor-to-`F` seam and factor clamp `[0.05, 5.0]`
  - `docs/22_test_quality.md` - real-path oracle and negative controls
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` - Step-1 delegated `GCode::_extrude` summary
- Verification:
  - `cargo test -p slicer-gcode --all-targets --test volumetric_auto_speed_tdd` - FACT pass/fail.
  - `cargo check -p slicer-gcode --all-targets` - FACT pass/fail.
- Exit condition: removing tool selection, width, height, flow factor, or the zero-speed branch makes at least one literal test fail; invalid zero/non-finite inputs cannot produce a `Move.f` value.

### Step 4A: Add the visual request and its parsed config source

- Task IDs: `TASK-571`
- Objective: add the mandated G-code-emission visual request and the companion JSON config file loaded through the request's `source.config` path and `parse_cli_config_source`.
- Precondition: Steps 1–3 pass their narrow tests and no unowned Phase-C negative sentinel remains.
- Postcondition: the committed request references the committed companion config containing `outer_wall_speed = 0` and `filament_max_volumetric_speed = 8.0`, and renders a valid `PostPass::GCodeEmit` bundle.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/19_visual_debug.md` - request, manifest, and `PostPass::GCodeEmit` sections only
  - Existing nearby visual-debug request fixture located by bounded search only
- Files allowed to edit (at most 3):
  - `crates/pnp-cli/tests/fixtures/config_scope_resolution_10/visual-debug.json`
  - `crates/pnp-cli/tests/fixtures/config_scope_resolution_10/visual-debug-config.json` - exact JSON object consumed by `parse_cli_config_source`
- Files explicitly out of bounds:
  - Visual-debug implementation/schema, packet/plan files, WIT, IR schema versions, unrelated docs
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - No new field/constant in this sub-step; Step 2 owns and has already closed the `ResolvedConfig` blast radius.
- Expected sub-agent dispatches:
  - Question: run the visual command and inspect only required manifest paths; scope: committed request/output bundle; return: `FACT` ≤5 lines.
- Context cost: `S`
- Authoritative docs:
  - `docs/19_visual_debug.md` - deterministic bundle contract
- OrcaSlicer refs:
  - None; no further parity read.
- Verification:
  - `cargo run --bin pnp_cli -- visual-debug --request crates/pnp-cli/tests/fixtures/config_scope_resolution_10/visual-debug.json --output target/visual-debug/config-scope-resolution-10` plus AC-4's bounded manifest assertion - FACT pass/fail.
- Exit condition: AC-4 passes and inspection confirms the request's `source.config` references the companion JSON file with exactly the two required config keys.

### Step 4B: Document placement and run closure gates

- Task IDs: `TASK-571`
- Objective: add the host-key mirror entry that feeds the generator, document Phase-C placement, regenerate config docs, and run closure gates after visual evidence is committed.
- Precondition: Step 4A passes AC-4; Step 2's `ResolvedConfig` field exists so the host-key mirror locks to a live default.
- Postcondition: `docs/config/host-keys.toml` carries the entry, docs name the key and placement, generated docs are clean, and freshness and workspace gates return decisive verdicts.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/02_ir_schemas.md` - `ResolvedConfig` section only
  - `docs/11_operational_governance_and_acceptance_gate.md` - compatibility dimensions only
  - `docs/config/host-keys.toml` - `[resolved_config]` table only
- Files allowed to edit (at most 3):
  - `docs/config/host-keys.toml` - add the `filament_max_volumetric_speed` `[resolved_config]` entry mirroring Step 2's default; feeds `cargo xtask gen-config-docs`
  - `docs/02_ir_schemas.md`
  - `docs/15_config_keys_reference.md` - generator output only; regenerate with `cargo xtask gen-config-docs` after the host-key entry, never hand-edit
- Files explicitly out of bounds:
  - Visual-debug implementation/schema and Step-4A fixtures, packet/plan files, WIT, IR schema versions, unrelated docs
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - No new field/constant in this sub-step; Step 2 owns and has already closed the `ResolvedConfig` blast radius.
- Expected sub-agent dispatches:
  - Question: run freshness, generator, check, clippy, literals, and touched-test quality gates; scope: exact commands below; return: one `FACT` verdict per command.
- Context cost: `S`
- Authoritative docs:
  - `docs/02_ir_schemas.md` - config placement contract
  - `docs/11_operational_governance_and_acceptance_gate.md` - compatibility dimensions
- OrcaSlicer refs:
  - None; no further parity read.
- Verification:
  - `cargo xtask gen-config-docs --check` - FACT pass/fail; the `docs/config/host-keys.toml` entry feeds this gate and the entry is regenerated into doc 15 first.
  - `cargo xtask build-guests --check` - FACT with exact exit 0/1/3.
  - `cargo check --workspace --all-targets` - FACT pass/fail.
  - `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail.
  - `cargo xtask check-literals && cargo xtask check-test-quality --report` - FACT pass/fail/findings for touched files.
- Exit condition: AC-4/AC-5 assertions pass, guest freshness exits 0, all workspace/touched-test gates pass, and no schema/version constant changed.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Dependency, sentinel, and canonical grounding |
| Step 2 | M | `ResolvedConfig` field blast radius and derived census |
| Step 3 | M | Real emitter path and negative controls |
| Step 4A | S | Visual request and parsed companion config |
| Step 4B | S | Docs/freshness/closure gates |

Aggregate remains M because steps are sequential and each dispatch returns bounded evidence; no step is L.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- Reconcile packets 04/05 FORWARD-DEPs and record the final negative-sentinel census result.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk, especially whether explicit positive speeds remain intentionally uncapped.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands use `--all-targets` so test, bench, and example targets compile.
