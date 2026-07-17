# Implementation Plan: 172-mm-e2e-and-object-keys

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Extended object-metadata allowlist (loader)

- Task IDs: `TASK-212`
- Objective: TDD-extend `object_metadata_to_config_data` with the 18 keys (Int: `wall_loops`, `top_shell_layers`, `bottom_shell_layers`, `raft_layers`, `support_interface_top_layers`, `support_interface_bottom_layers`; Int-rebased: `support_filament`, `support_interface_filament`; Float: `layer_height`, `brim_width`, `support_threshold_angle`, `support_top_z_distance`; String: `seam_position`, `sparse_infill_density`, `sparse_infill_pattern`, `brim_type`, `fuzzy_skin`, `support_base_pattern`) plus unknown-key logging (recognized set = 3 existing + 18 new + `name` + `matrix`). `object_metadata_to_config_data` is private: the new tests reach it through the public 3MF load path exactly as the existing allowlist assertions in `threemf_sidecar_classification_tdd.rs` (lines 220-290) do — build an in-memory 3MF whose model-settings sidecar carries the object metadata, load it, and assert on the resulting per-object config map. Do not widen the function's visibility.
- Precondition: workspace compiles; delegated LOCATIONS confirmation of key spellings from `GUI_Factories.cpp` received.
- Postcondition: AC-1, AC-2, AC-N2 tests green; pre-existing sidecar tests green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-model-io/src/loader.rs` - lines 655-700 and 805-870
  - `crates/slicer-model-io/tests/threemf_sidecar_classification_tdd.rs` - lines 220-290 (existing assertion style)
  - `.ralph/specs/172-mm-e2e-and-object-keys/design.md`
- Files allowed to edit (at most 3):
  - `crates/slicer-model-io/src/loader.rs`
  - `crates/slicer-model-io/tests/threemf_sidecar_classification_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-model-io/src/sidecar.rs` (capture side unchanged), `crates/slicer-runtime/**`, `OrcaSlicerDocumented/**`
- Expected sub-agent dispatches:
  - Question: confirm the 18 key spellings against Orca's per-object settable set; scope: `OrcaSlicerDocumented/src/slic3r/GUI/GUI_Factories.cpp`; return: `LOCATIONS` (≤20)
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md` - delegated bounded lookup of the per-object config subsection
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/slic3r/GUI/GUI_Factories.cpp` - delegate; never load
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - delegate; never load
- Verification:
  - `mkdir -p target && cargo test -p slicer-model-io --test threemf_sidecar_classification_tdd 2>&1 | tee target/test-output.log | grep "^test result"` - FACT pass/fail
- Exit condition: all three new tests (`extended_object_allowlist_types`, `support_filament_keys_rebased`, `invalid_and_unknown_object_keys_logged`) green with zero pre-existing failures; a silently-dropped unknown key falsifies.

### Step 2: SupportToolSelection routing (runtime)

- Task IDs: `TASK-210`
- Objective: TDD-add `SupportToolSelection` to `layer_executor.rs`, extend `assemble_ordered_entities` with the parameter (support/raft → support tool; interface/ironing → interface tool), thread it via `PipelineConfig` from a `config_source` parse in `run.rs` (1-indexed rebase, default `{0,0}`), updating all three call sites (`layer_executor.rs:463`, `:573`, test `:2300`) and both threading functions (`execute_single_layer_inner`, `prestage_layer_collection_if_path_optimization`). Because `SupportToolSelection` and `assemble_ordered_entities` are `pub(crate)`, the new tests (`support_tool_selection_assigns_entities`, `support_tool_selection_default_keeps_tool_zero`) go in the existing in-file `#[cfg(test)] mod tests` of `layer_executor.rs` (declared at line 2195) and run via `cargo test -p slicer-runtime --lib` — never via the external `--test unit` binary, which cannot reach them.
- Precondition: Step 1 complete (independent, but fixes rebase convention precedent); delegated FACT on `PrintConfig.cpp` filament-selector semantics received.
- Postcondition: AC-3 unit test green; AC-N1 (`tool_ordering`) and `cube_4color` executor suites green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/layer_executor.rs` - lines 261-300, 430-480, 555-590, 1365-1400, 1600-1670, 2290-2320
  - `crates/slicer-runtime/src/run.rs` - lines 600-660
  - `crates/slicer-runtime/src/pipeline.rs` - the `PipelineConfig` struct definition (locate via grep, read ±20 lines)
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/src/layer_executor.rs`
  - `crates/slicer-runtime/src/run.rs`
  - `crates/slicer-runtime/src/pipeline.rs`
- Files explicitly out of bounds:
  - `crates/slicer-gcode/**` (emit only consumes `tool_index`), `crates/slicer-scheduler/**`, `OrcaSlicerDocumented/**`
- Expected sub-agent dispatches:
  - Question: `support_filament`/`support_interface_filament` option semantics (1-indexed? 0 meaning?); scope: `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp`; return: `FACT`
  - Question: does `cargo check --workspace --all-targets` pass; scope: workspace; return: `FACT` + ≤20 error lines
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md` - delegated bounded lookup of `SupportIR`/`PrintEntity` sections
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - delegate; never load
- Verification:
  - `mkdir -p target && cargo test -p slicer-runtime --lib -- support_tool_selection 2>&1 | tee target/test-output.log | grep "^test result"` - FACT pass/fail (must report ≥2 tests run; 0 tests run is a FAIL)
  - `mkdir -p target && cargo test -p slicer-runtime --test unit -- tool_ordering 2>&1 | tee target/test-output.log | grep "^test result"` - FACT pass/fail (pre-existing external suite)
- Exit condition: both in-file `support_tool_selection` tests green (assignment + default-zero) AND external `tool_ordering` green; any default-path tool_index change, or a `--lib` filter matching 0 tests, falsifies.

### Step 3: Real-fixture MM E2E

- Task IDs: `TASK-211`
- Objective: author `crates/slicer-runtime/tests/e2e/mm_real_fixture_gcode_tdd.rs` with `mm_painted_fixture_t0_t1` (multi_tool_triangle.3mf → both `T0` and `T1` lines in emitted G-code) and `mm_support_filament_real_fixture` (bridge_support_enforcers.3mf + `enable_support=true` + `support_filament=2` → at least one `T1` and one `T0` line), registered in the e2e bucket harness.
- Precondition: Step 2 complete; delegated FACTs on the e2e harness file and the multi_tool_triangle parity-test config received.
- Postcondition: AC-4 and AC-5 green; executor `cube_4color` suite green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/e2e/run_slice_api_tdd.rs` - full (API pattern reference)
  - `.ralph/specs/172-mm-e2e-and-object-keys/design.md`
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/e2e/mm_real_fixture_gcode_tdd.rs` (new)
  - the e2e bucket harness file (one `mod` line; locate via dispatch)
- Files explicitly out of bounds:
  - `*.3mf` fixture binaries (slice via test API only), `crates/slicer-model-io/**`, `OrcaSlicerDocumented/**`
- Expected sub-agent dispatches:
  - Question: which harness file declares the `e2e` bucket `mod` entries; scope: `crates/slicer-runtime/tests/`; return: `FACT`
  - Question: what config does the existing `multi_tool_triangle` parity test pass, and does it produce ≥2 tools; scope: `crates/slicer-runtime/tests/` (grep `multi_tool_triangle`); return: `FACT` + ≤10 lines
- Context cost: `M`
- Authoritative docs:
  - none beyond `design.md` (guest WASM must be fresh: run `cargo xtask build-guests --check` before attributing any e2e failure to this change)
- OrcaSlicer refs:
  - none
- Verification:
  - `mkdir -p target && cargo test -p slicer-runtime --test e2e -- mm_ 2>&1 | tee target/test-output.log | grep "^test result"` - FACT pass/fail
  - `mkdir -p target && cargo test -p slicer-runtime --test executor -- cube_4color 2>&1 | tee target/test-output.log | grep "^test result"` - FACT pass/fail
- Exit condition: both `mm_` e2e tests green on real fixtures; if `multi_tool_triangle` cannot produce two tools, switch AC-5's fixture to `resources/cube_4color.3mf` (fallback locked in `design.md` §Risks) and record the swap — an AC-5 asserting on a single-tool run falsifies.

### Step 4: Docs + closure gates

- Task IDs: `TASK-210`, `TASK-211`, `TASK-212`
- Objective: update `docs/02_ir_schemas.md` (per-object allowlist + rebase semantics, `support_filament` routing note incl. the flat-SupportIR global-selection deviation); dispatch the docs/07 row flips per `task-map.md`; run closure gates.
- Precondition: Steps 1-3 complete.
- Postcondition: doc grep passes; workspace check/clippy green.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/02_ir_schemas.md` - only the per-object config / SupportIR subsections (locate via grep)
  - `.ralph/specs/172-mm-e2e-and-object-keys/task-map.md`
- Files allowed to edit (at most 3):
  - `docs/02_ir_schemas.md`
  - `.ralph/specs/172-mm-e2e-and-object-keys/packet.spec.md` (status flip at ceremony only)
- Files explicitly out of bounds:
  - `docs/07_implementation_status.md` (worker dispatch only, never a full read)
- Expected sub-agent dispatches:
  - Question: flip TASK-210/211/212 rows (lines 137-139) to done with the packet reference and the global-selection deviation note on TASK-210; scope: `docs/07_implementation_status.md`; return: `FACT` (rows updated)
  - Question: run `cargo clippy --workspace --all-targets -- -D warnings`; scope: workspace; return: `FACT` pass/fail
- Context cost: `S`
- Authoritative docs:
  - `docs/02_ir_schemas.md` - bounded subsections only
- OrcaSlicer refs:
  - none
- Verification:
  - `rg -q 'support_filament' docs/02_ir_schemas.md && echo PASS || echo FAIL` - FACT
  - `cargo check --workspace --all-targets` - FACT pass/fail (delegated)
- Exit condition: doc grep PASS, both workspace gates green, all three docs/07 rows updated; an unflipped row falsifies.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | loader allowlist + tests |
| Step 2 | M | SupportToolSelection threading |
| Step 3 | M | real-fixture e2e; guest freshness check before diagnosing failures |
| Step 4 | S | docs + gates |

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- Reconcile reopened/superseded status transitions (none expected).
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk (flat-SupportIR global selection deviation; fixture swap if exercised).
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
