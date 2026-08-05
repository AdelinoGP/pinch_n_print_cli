# Implementation Plan: 171-gcode-flavor-writer

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Red tests — dialect matrix

- Task IDs: `TASK-276`
- Objective: author the failing test file `crates/slicer-gcode/tests/gcode_flavor_dialect_tdd.rs` covering: `flavor_parses_five_config_strings`, `rrf_temperature_uses_g10_and_m116`, `acceleration_dialect_per_flavor`, `travel_acceleration_capability_matrix`, jerk/junction-deviation/pressure-advance per-flavor assertions, and `unknown_flavor_falls_back_to_marlin`, pinning the exact strings from `design.md` §Data and Contract Notes.
- Precondition: workspace compiles (`cargo check -p slicer-gcode`).
- Postcondition: test file exists, fails to compile (missing `GcodeFlavor`) — red state confirmed.
- Files allowed to read, with ranges when over 300 lines:
  - `.ralph/specs/171-gcode-flavor-writer/design.md`
  - `crates/slicer-gcode/src/lib.rs`
  - `crates/slicer-gcode/tests/gcode_relative_extrusion_tdd.rs` - lines 1-60 (test-harness style reference)
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/tests/gcode_flavor_dialect_tdd.rs` (new)
- Files explicitly out of bounds:
  - `OrcaSlicerDocumented/**`, `crates/slicer-gcode/src/serialize.rs` (no impl edits yet)
- Expected sub-agent dispatches:
  - Question: verbatim per-flavor branches of `GCodeWriter.cpp::set_temperature`, `set_acceleration_internal`, `set_jerk_xy`, `set_pressure_advance`, `set_junction_deviation`; scope: `OrcaSlicerDocumented/src/libslic3r/GCodeWriter.cpp`; return: `SNIPPETS` (≤3×30 lines)
- Context cost: `S`
- Authoritative docs:
  - `docs/ORCASLICER_ATTRIBUTION.md` - direct read (short)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCodeWriter.cpp` - delegate; never load
- Verification:
  - `cargo check -p slicer-gcode --all-targets 2>&1 | tail -20` - FACT: fails naming missing `GcodeFlavor` symbols only
- Exit condition: red run fails solely on missing dialect symbols; any other failure falsifies the step.

### Step 2: Green — `flavor.rs` dialect layer

- Task IDs: `TASK-276`
- Objective: implement `crates/slicer-gcode/src/flavor.rs` (`GcodeFlavor` enum, `from_config_str`, `config_str`, `set_temperature`, `set_bed_temperature`, `set_acceleration`, `set_travel_acceleration`, `supports_separate_travel_acceleration`, `set_jerk_xy`, `set_junction_deviation`, `set_pressure_advance`) with the OrcaSlicer attribution header, and register `pub mod flavor;` + `GcodeFlavor` re-export in `lib.rs`.
- Precondition: Step 1 red state.
- Postcondition: all dialect tests pass; no line-number citations in `flavor.rs`.
- Files allowed to read, with ranges when over 300 lines:
  - `.ralph/specs/171-gcode-flavor-writer/design.md`
  - `docs/ORCASLICER_ATTRIBUTION.md`
  - `crates/slicer-gcode/tests/gcode_flavor_dialect_tdd.rs`
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/src/flavor.rs` (new)
  - `crates/slicer-gcode/src/lib.rs`
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/serialize.rs`, `crates/slicer-runtime/**`, `OrcaSlicerDocumented/**`
- Expected sub-agent dispatches:
  - none (Step 1's SNIPPETS return is carried in the test file's pinned strings)
- Context cost: `M`
- Authoritative docs:
  - `docs/ORCASLICER_ATTRIBUTION.md` - direct read
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCodeWriter.cpp` - delegate; never load
- Verification:
  - `mkdir -p target && cargo test -p slicer-gcode --test gcode_flavor_dialect_tdd 2>&1 | tee target/test-output.log | grep "^test result"` - FACT pass/fail
  - `cd F:/slicerProject/pinch_n_print && head -20 crates/slicer-gcode/src/flavor.rs | grep -qi "OrcaSlicer" && ! grep -nE "GCodeWriter\.cpp:[0-9]" crates/slicer-gcode/src/flavor.rs && echo PASS || echo FAIL` - FACT
- Exit condition: dialect test binary fully green and attribution grep PASS; any red test falsifies.

### Step 3: Serializer routing + runtime threading

- Task IDs: `TASK-276`
- Objective: add `flavor: GcodeFlavor` + `with_flavor` to `DefaultGCodeSerializer` (default `Marlin` in all constructors); route the `GCodeCommand::Temperature` arm through `self.flavor.set_temperature`; parse `gcode_flavor` from `config_source` in `run.rs` (next to the `use_relative_e_distances` read) and construct the serializer with it.
- Precondition: Step 2 green.
- Postcondition: RRF serializer emits `G10 P.. S..`/`M116`; default path byte-identical (all pre-existing `slicer-gcode` tests green).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/src/serialize.rs` - lines 55-135 and 480-744
  - `crates/slicer-runtime/src/run.rs` - lines 600-660
  - `crates/slicer-runtime/src/pipeline.rs` - lines 450-470
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/src/serialize.rs`
  - `crates/slicer-runtime/src/run.rs`
  - `crates/slicer-runtime/src/pipeline.rs` (only if the constructor change forces an explicit default)
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/emit.rs`, `OrcaSlicerDocumented/**`
- Expected sub-agent dispatches:
  - Question: does `cargo check --workspace --all-targets` pass; scope: workspace; return: `FACT` + ≤20 error lines
- Context cost: `M`
- Authoritative docs:
  - none beyond `design.md`
- OrcaSlicer refs:
  - none (port already landed in Step 2)
- Verification:
  - `mkdir -p target && cargo test -p slicer-gcode 2>&1 | tee target/test-output.log | grep "^test result"` - FACT pass/fail (includes golden byte-identity, AC-6)
- Exit condition: all `slicer-gcode` test binaries green including `golden_emit_tdd`; any golden diff falsifies (default-flavor regression).

### Step 4: CONFIG_BLOCK echo + integration test

- Task IDs: `TASK-276`
- Objective: extend `serialize_config_block` to emit the real resolved `gcode_flavor` key (raw_config wins; resolved default otherwise), remove `("gcode_flavor", "marlin")` from `ORCA_CONFIG_PADDING`, forward the flavor through `ThumbnailAwareSerializer`; add `crates/slicer-runtime/tests/integration/gcode_flavor_config_block_tdd.rs` asserting AC-5 (klipper echo, exactly-one line, no marlin line) and register its `mod` in the integration bucket harness.
- Precondition: Step 3 complete.
- Postcondition: AC-5 test green; existing CONFIG_BLOCK invariant tests still green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/src/serialize.rs` - lines 264-410 and 480-552
  - `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` - read-only harness/style reference
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/src/serialize.rs`
  - `crates/slicer-runtime/tests/integration/gcode_flavor_config_block_tdd.rs` (new)
  - the integration bucket harness file (one `mod` line; locate via dispatch)
- Files explicitly out of bounds:
  - `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` (packet 167 owns edits), `OrcaSlicerDocumented/**`
- Expected sub-agent dispatches:
  - Question: which file declares `mod` entries for the integration test bucket; scope: `crates/slicer-runtime/tests/`; return: `FACT`
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md` - delegated bounded lookup of the CONFIG_BLOCK subsection (for Step 5's doc edit anchor)
- OrcaSlicer refs:
  - none
- Verification:
  - `mkdir -p target && cargo test -p slicer-runtime --test integration -- gcode_flavor_config_block 2>&1 | tee target/test-output.log | grep "^test result"` - FACT pass/fail
  - `mkdir -p target && cargo test -p slicer-runtime --test integration -- gcode_header 2>&1 | tee target/test-output.log | grep "^test result"` - FACT pass/fail (pre-existing block invariants)
- Exit condition: both integration filters green; a duplicated or missing `gcode_flavor` line falsifies.

### Step 5: Docs + closure gates

- Task IDs: `TASK-276`
- Objective: add the `gcode_flavor` honored-key note to `docs/02_ir_schemas.md` (CONFIG_BLOCK / G-code serialization subsection); dispatch the TASK-276 row addition to `docs/07_implementation_status.md` per `task-map.md`; run closure gates.
- Precondition: Steps 1-4 complete.
- Postcondition: doc grep passes; workspace check/clippy green.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/02_ir_schemas.md` - only the subsection located by the Step 4 dispatch
  - `.ralph/specs/171-gcode-flavor-writer/task-map.md`
- Files allowed to edit (at most 3):
  - `docs/02_ir_schemas.md`
  - `.ralph/specs/171-gcode-flavor-writer/packet.spec.md` (status flip at ceremony only)
- Files explicitly out of bounds:
  - `docs/07_implementation_status.md` (worker dispatch only, never a full read)
- Expected sub-agent dispatches:
  - Question: append the TASK-276 row per `task-map.md`; scope: `docs/07_implementation_status.md`; return: `FACT` (row added, line number)
  - Question: run `cargo clippy --workspace --all-targets -- -D warnings`; scope: workspace; return: `FACT` pass/fail
- Context cost: `S`
- Authoritative docs:
  - `docs/02_ir_schemas.md` - bounded subsection only
- OrcaSlicer refs:
  - none
- Verification:
  - `rg -q 'gcode_flavor' docs/02_ir_schemas.md && echo PASS || echo FAIL` - FACT
  - `cargo check --workspace --all-targets` - FACT pass/fail (delegated)
- Exit condition: doc grep PASS and both workspace gates green; a clippy warning falsifies.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | red tests; Orca SNIPPETS dispatch |
| Step 2 | M | dialect port + attribution |
| Step 3 | M | serializer/runtime wiring; golden identity |
| Step 4 | M | CONFIG_BLOCK echo + integration test |
| Step 5 | S | docs + gates |

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- Reconcile reopened/superseded status transitions (none expected).
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk (packet-167 padding-table merge order).
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
