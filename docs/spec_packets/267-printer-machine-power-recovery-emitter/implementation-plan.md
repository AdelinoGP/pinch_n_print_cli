# Implementation Plan: 267-printer-machine-power-recovery-emitter

## Execution Rules

- Work one atomic step at a time; map every step to [25 - Author packet P18 - Printer / Machine / Power / recovery - emitter](../specs/orca-feature-gap/issues/25-author-packet-p18-printer-machine-power-recovery-emitter.md); this queue packet has `task_ids: []` (the closed `disable_m73` predecessor `TASK-279` is a historical backlog row, not an ownership row).
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".
- Every `cargo` command tees to `target/test-output.log`; when a run fails, read the log, never re-run to see more output.

## Steps

### Step 1: Declare the three owner-manifest keys and add the schema guard

- Task IDs: `[]` (wayfinder ticket 25)
- Objective: declare `disable_m73`, `emit_machine_limits_to_gcode`, and `enable_power_loss_recovery` in `machine-gcode-emit.toml` with the exact AC-1 types/defaults/values, and add a TOML-direct-parse guard that also asserts the P47 keys and `silent_mode` are absent.
- Precondition: `machine-gcode-emit.toml` has the existing injection-point schema; no P18 packet directory or test guard claims these keys.
- Postcondition: the three tables have the exact AC-1 types/defaults/values/display/group; the guard binary is auto-discovered and passes; the manifest declares none of the P47 keys nor `silent_mode`.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml` - full manifest; it is short.
  - `modules/core-modules/machine-gcode-emit/Cargo.toml` - package and dev-dependency sections (add `toml = "0.8"` if absent, per the 266 precedent).
  - `modules/core-modules/top-surface-ironing/Cargo.toml` - TOML dev-dependency precedent only.
- Files allowed to edit (at most 3):
  - `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml`
  - `modules/core-modules/machine-gcode-emit/Cargo.toml`
  - `modules/core-modules/machine-gcode-emit/tests/machine_gcode_emit_config_schema_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-gcode/**` - Steps 2-3.
  - `crates/slicer-scheduler/**` - Step 5.
  - `docs/15_config_keys_reference.md` - generated in Step 5.
  - `OrcaSlicerDocumented/**` - delegate; never load.
- Blast-radius discipline: none; manifest data and a new test target add no Rust struct field or schema constant.
- Expected sub-agent dispatches:
  - Question: is `toml = "0.8"` absent in the module and is the module using Cargo test auto-discovery with no explicit aggregator?; scope: the module `Cargo.toml` and `tests/`; return: `FACT`.
- Context cost: `S`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` - delegated SUMMARY of exact schema forms.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - delegated `LOCATIONS` already captured in `requirements.md`; re-dispatch only if a default/bound is disputed.
- Verification:
  - `cargo test -p machine-gcode-emit --test machine_gcode_emit_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail.
- Exit condition: the schema guard passes, it asserts the three tables and the absence of the P47 keys and `silent_mode`, and a targeted TOML search confirms the tables' exact values.

### Step 2: Add the two `ResolvedConfig` fields and the host-key documentation

- Task IDs: `[]` (wayfinder ticket 25)
- Objective: `emit_machine_limits_to_gcode: bool` (default `true`) and `enable_power_loss_recovery: String` (default `"printer_configuration"`) become typed `ResolvedConfig` fields with `to_config_map`, `PartialEq`, and `Hash` coverage, and both are documented in `docs/config/host-keys.toml` with lock-test arms.
- Precondition: Step 1's manifest and guard pass.
- Postcondition: the two fields resolve from the CLI/JSON config source; `to_config_map` emits them; the manual `PartialEq` and `Hash` impls cover them; the lock test passes with the two new rows.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/resolved_config.rs` - the `cli` macro invocation, `to_config_map`, `impl PartialEq`, and `impl Hash` regions only (the file is long; ranged reads anchored on those symbols).
  - `docs/config/host-keys.toml` - the `[resolved_config]` section only.
  - `crates/slicer-runtime/tests/unit/host_keys_doc_lock_tdd.rs` - `resolved_bool` and `resolved_str` helpers and the `resolved_config_keys_match_default` test.
- Files allowed to edit (at most 3):
  - `crates/slicer-ir/src/resolved_config.rs`
  - `docs/config/host-keys.toml`
  - `crates/slicer-runtime/tests/unit/host_keys_doc_lock_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/serialize.rs` - `ORCA_CONFIG_PADDING` and CONFIG_BLOCK twins (map rule 2; AC-N3).
  - `crates/slicer-gcode/src/estimator.rs` - the estimator's machine-limit reads are unchanged.
  - `OrcaSlicerDocumented/**` - delegate; never load.
- Blast-radius discipline: the `cli` macro generates the struct definition and `Default`, so adding two `cli` lines does not break the tree's `ResolvedConfig { .. }` literals (verified at authoring time: the literal sites use `..ResolvedConfig::default()` functional update). Re-derive the literal count at point of use with a `LOCATIONS` dispatch before editing; if any exhaustive literal exists, add it to this step's edit list. The step's exit condition is `cargo check --workspace --all-targets` plus `cargo xtask check-literals`, both green, in the same step.
- Expected sub-agent dispatches:
  - Question: re-derive every `ResolvedConfig {` literal site and confirm each uses a `..` rest; scope: `crates/**` and `modules/**`, Rust only; return: `LOCATIONS` (≤ 20 entries).
  - Question: confirm the exact `cli` macro field form for a bool and a String with defaults, and the `to_config_map` / `PartialEq` / `Hash` regions to extend; scope: `crates/slicer-ir/src/resolved_config.rs`; return: `LOCATIONS`.
- Context cost: `M`
- Authoritative docs:
  - `docs/21_data_defaults_and_fixtures.md` - the struct-literal churn gate.
- OrcaSlicer refs: none; the canonical defaults are captured in `requirements.md`.
- Verification:
  - `cargo test -p slicer-runtime --test unit host_keys_doc_lock_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail.
  - `cargo check --workspace --all-targets` - FACT pass/fail.
  - `cargo xtask check-literals` - FACT exit code.
- Exit condition: the lock test passes with the two new rows; `to_config_map` emits both keys; `PartialEq`/`Hash` cover them; the workspace compiles and the literal gate is green.

### Step 3: Wire the flavor into the emitter and build the envelope + recovery emission

- Task IDs: `[]` (wayfinder ticket 25)
- Objective: `DefaultGCodeEmitter` gains a `flavor` field (default `GcodeFlavor::Marlin`) and a `with_flavor` builder; `run_slice` passes the resolved flavor; `emit_gcode` prepends the machine envelope and pushes the recovery commands; invariant tests cover AC-3 through AC-6.
- Precondition: Steps 1-2 complete; the two new fields resolve.
- Postcondition: with `emit_machine_limits_to_gcode = true` and the speed/jerk fields set, the GCodeIR stream opens with M203/M204/M205 Raw commands ahead of the M73 pair and `ExtrusionMode` (AC-3); `false` emits none; Klipper/Repetier emit none and the M204 forms match AC-4; the recovery modes and flavor gate match AC-5 and AC-6; the default path (all fields `None`, `"printer_configuration"`) emits no envelope and no recovery line.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/src/emit.rs` - `DefaultGCodeEmitter` struct and builders, `emit_gcode` head (ExtrusionMode open), the layer loop (marker triple, `emitted_layer_count`), and the estimator/M73 tail only.
  - `crates/slicer-gcode/src/flavor.rs` - the `GcodeFlavor` enum and dialect helpers.
  - `crates/slicer-gcode/src/m73.rs` - `inject_m73`'s head-pair insertion.
  - `crates/slicer-gcode/tests/gcode_emit_tdd.rs` - fixture helpers and existing stream assertions.
  - `crates/slicer-runtime/src/run.rs` - the emitter construction and the existing flavor resolution (lines around the `DefaultGCodeEmitter::new_with_config` call).
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/src/emit.rs`
  - `crates/slicer-gcode/tests/gcode_emit_tdd.rs`
  - `crates/slicer-runtime/src/run.rs`
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/serialize.rs` - `ORCA_CONFIG_PADDING` and CONFIG_BLOCK twins (AC-N3).
  - `modules/core-modules/machine-gcode-emit/**` - Step 4.
  - `crates/slicer-ir/src/**` and `crates/slicer-schema/wit/**` - no boundary change.
  - `OrcaSlicerDocumented/**` - delegate; never load.
- Blast-radius discipline: no public struct or schema constant is added; the new `flavor` field is private with a builder, and the existing `DefaultGCodeEmitter::new` / `new_with_config` constructions (including the two in `emit.rs`'s own tests) keep compiling via the default.
- Expected sub-agent dispatches:
  - Question: confirm the exact insertion points — where the command list is complete before `inject_m73`, and where the second-emitted-layer marker triple is pushed; scope: `crates/slicer-gcode/src/emit.rs`; return: `LOCATIONS`.
  - Question: confirm the canonical M204 gating and the RRF × 60 application if a worker disputes `requirements.md`'s evidence table; scope: delegated `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` `print_machine_envelope`; return: `SUMMARY` (≤ 200 words).
- Context cost: `M`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` - delegated config-view reachability summary.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` - `print_machine_envelope` and the `_do_export` recovery call sites, delegated only.
  - `OrcaSlicerDocumented/src/libslic3r/GCodeWriter.cpp` - `enable_power_loss_recovery`, delegated only.
- Verification:
  - `cargo test -p slicer-gcode --test gcode_emit_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail.
- Exit condition: the emission binary passes with named envelope, flavor, and recovery tests; the envelope Raw text carries no trailing newline (the serializer's Raw arm adds one); and `rg -n 'M413|M203|M204|M205' crates/slicer-gcode/src/emit.rs` shows the production emission sites, not just test strings.

### Step 4: Change the postpass PrintStart insertion rule and rebuild the guest

- Task IDs: `[]` (wayfinder ticket 25)
- Objective: `MachineGcodeEmit::run_gcode_postprocess` inserts the resolved `machine_start_gcode` template after the leading run of non-M73 Raw commands, so the host's envelope precedes the start template while the existing ordering contract survives.
- Precondition: Step 3 emits the envelope at the head of the GCodeIR stream.
- Postcondition: with a leading envelope run, the start template lands after it; without one, the template still opens the stream; `machine_start_gcode_precedes_m73_and_extrusion_mode` passes unchanged; the guest artifact is rebuilt and fresh.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/machine-gcode-emit/src/lib.rs` - `run_gcode_postprocess` Step 4 and the injection registry only.
  - `modules/core-modules/machine-gcode-emit/tests/machine_gcode_emit_tdd.rs` - the existing ordering test and fixture helpers.
- Files allowed to edit (at most 3):
  - `modules/core-modules/machine-gcode-emit/src/lib.rs`
  - `modules/core-modules/machine-gcode-emit/tests/machine_gcode_emit_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-gcode/**` - the host side is Step 3's.
  - `OrcaSlicerDocumented/**` - delegate; never load.
- Blast-radius discipline: none; the change is a scan-and-insert in one function plus a test case.
- Expected sub-agent dispatches:
  - Question: confirm the exact Step 4 loop shape and the `GcodeOutputBuilder` push API before editing; scope: `modules/core-modules/machine-gcode-emit/src/lib.rs`; return: `LOCATIONS`.
- Context cost: `S`
- Authoritative docs: none beyond the module's own ordering contract.
- OrcaSlicer refs: none; the ordering is a PnP stream contract, not canonical logic.
- Verification:
  - `cargo test -p machine-gcode-emit --test machine_gcode_emit_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail.
  - `cargo xtask build-guests --check` - FACT exit code; if stale, rebuild without `--check` and re-run the test.
- Exit condition: the module binary passes with the new envelope-ordering case and the unchanged `machine_start_gcode_precedes_m73_and_extrusion_mode`; guest freshness is exit 0.

### Step 5: Add the bounds arm, regenerate docs, and close packet gates

- Task IDs: `[]` (wayfinder ticket 25)
- Objective: prove scheduler enum/bool enforcement for the three declarations (AC-N1), regenerate the generated key reference, and run the packet's closure gates.
- Precondition: Steps 1-4 pass and all manifest/source/config changes are present.
- Postcondition: `enable_power_loss_recovery = "bogus"` and `emit_machine_limits_to_gcode = "yes"` reject with the existing error variants; boundary values resolve; `docs/15_config_keys_reference.md` is generated and its `--check` passes; workspace check/clippy/literals pass.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` - module loading and existing error assertion arms.
  - `crates/slicer-scheduler/src/config_resolution.rs` - `ConfigBoundsIndex::from_modules` and `check`, located by symbol.
  - `docs/15_config_keys_reference.md` - targeted `rg` probes only; never load in full.
- Files allowed to edit (at most 3):
  - `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs`
  - `docs/15_config_keys_reference.md` - generated only through `cargo xtask gen-config-docs`.
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/serialize.rs` - `ORCA_CONFIG_PADDING` and CONFIG_BLOCK twins (AC-N3).
  - `docs/ORCA_CONFIG_REFERENCE.md` - hand-maintained source snapshot.
  - `target/`, `Cargo.lock`, generated code other than doc output - never load.
- Blast-radius discipline: test-only additions to an existing aggregated binary; no new test file or aggregator is needed.
- Expected sub-agent dispatches:
  - Question: does the bounds binary load the real machine-gcode-emit manifest, and what exact error assertion shape should the two arms use?; scope: `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` and `crates/slicer-scheduler/src/config_resolution.rs`; return: `FACT`.
  - Question: after regeneration, do the three machine-gcode-emit rows exist and are there no P18 deviation rows?; scope: generated doc and xtask output; return: `FACT`.
  - Question: run `cargo xtask build-guests --check` and report the exit code; if stale, rebuild without `--check` first; scope: xtask and guest artifacts; return: `FACT`.
- Context cost: `S`
- Authoritative docs: none; generated output and gates are the evidence.
- OrcaSlicer refs: none; canonical evidence is captured in earlier steps.
- Verification:
  - `cargo test -p slicer-scheduler --test scheduler_integration config_bounds_enforcement_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail.
  - `cargo xtask gen-config-docs` - FACT exit code.
  - `cargo xtask gen-config-docs --check` - FACT exit code.
  - `cargo check --workspace --all-targets` - FACT pass/fail.
  - `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail.
  - `cargo xtask check-literals` - FACT exit code.
  - Full matrix from `requirements.md` - FACT pass/fail per command.
- Exit condition: every packet AC and gate command passes; generated docs prove the three machine-gcode-emit keys; guest freshness is exit 0; `git diff --stat -- crates/slicer-gcode/src/serialize.rs` is empty (AC-N3).

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Short manifest plus one direct TOML guard |
| Step 2 | M | Two `ResolvedConfig` fields plus host-key documentation and lock arms |
| Step 3 | M | Emitter flavour wiring, envelope, recovery, and invariant tests |
| Step 4 | S | One postpass insertion rule plus a test case and guest rebuild |
| Step 5 | S | Bounds arm, generated output, and delegated gates |

Aggregate: `M`; no step is `L`, so no split is required before activation. Five atomic steps.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- `docs/07_implementation_status.md` is N-A for this queue packet (`task_ids: []`); implementation/authoring is recorded against wayfinder ticket 25 and the crosswalk is re-derived at completion.
- No reopened or superseded packet transition exists.
- `packet.spec.md` remains `draft` until an explicit activation decision; it is otherwise ready for implementation.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk: the envelope is a partial subset of canonical's (DIV-267-1..4) and the PrintStart rule encodes the host envelope shape in the module; both are bounded, named, and pinned by tests.
- Confirm context stayed within the standard band; no extended-band escalation is permitted for this packet.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands use the required workspace/all-target or tee conventions.
