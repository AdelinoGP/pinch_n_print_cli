# Implementation Plan: 282-resonance-avoidance-emitter

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Declare the three scalar-global fields + mirror + guards scaffold

- Task IDs: `TASK-000`
- Objective: `resonance_avoidance`, `min_resonance_avoidance_speed`, `max_resonance_avoidance_speed` resolve from raw config with canonical defaults and reject negatives/inverted ranges; schema guard proves it.
- Precondition: the 3 names are absent from `crates/slicer-ir/src/resolved_config.rs` and `docs/config/host-keys.toml` (dispatched FACT); `declare_resolved_config!` invocation still at `crates/slicer-ir/src/resolved_config.rs` with `cli`-key field syntax.
- Postcondition: `ResolvedConfig` carries the 3 fields (bool `false`, f32 `70.0`/`120.0`, `extract_bool`/`extract_float`); `host-keys.toml` `[resolved_config]` mirrors them; new test file's schema test passes; no behaviour change yet (adjustment not wired).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/resolved_config.rs` - lines `1821-1850`
  - `docs/config/host-keys.toml` - lines `1-60`
  - `crates/slicer-gcode/tests/gcode_feedrate_emission_tdd.rs` - lines `1-100`
- Files allowed to edit (at most 3):
  - `crates/slicer-ir/src/resolved_config.rs`
  - `docs/config/host-keys.toml`
  - `crates/slicer-gcode/tests/resonance_avoidance_emission_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/emit.rs` (Step 2)
  - `OrcaSlicerDocumented/...`, `target/`, `modules/core-modules/machine-gcode-emit/...`
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - New `ResolvedConfig` fields ripple into every struct literal of that struct. Dispatch a `LOCATIONS` worker for `ResolvedConfig {` literal sites before editing; cite the result inline below and include every site in this step's edit list (split the step if it exceeds 3 edits — prefer oneStep-per-blast-radius over a wide edit).
  - LOCATIONS result (pre-authored 2026-09-07, re-derive at implementation): worker not yet run — implementer MUST dispatch `LOCATIONS: rg 'ResolvedConfig\s*\{' --type rust` (≤20 entries) and extend the edit list before writing fields.
- Expected sub-agent dispatches:
  - Question: list every `ResolvedConfig {` struct-literal site that must gain the 3 fields; scope: `crates/ modules/ xtask/`; return: `LOCATIONS`
  - Question: what extractor do neighbouring bool/f32 `ResolvedConfig` fields use (`extract_bool`/`extract_float` vs alternatives)?; scope: `crates/slicer-ir/src/resolved_config.rs`; return: `FACT`
- Context cost: `M` (blast radius unknown until dispatch; split if L)
- Authoritative docs:
  - `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` - P48 rows only
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - delegate; never load
- Verification:
  - `cargo test -p slicer-gcode --test resonance_avoidance_emission_tdd schema_declares_three_keys 2>&1 | tail -5` - FACT pass/fail or bounded failure SNIPPETS
  - `cargo check --workspace --all-targets 2>&1 | tail -3` - FACT pass/fail (blast-radius proof)
- Exit condition: schema test names all 3 keys with types `bool`/`f32`/`f32`, defaults `false`/`70.0`/`120.0`, and TOML mirror spellings; `cargo check --workspace --all-targets` green.

### Step 2: Wire the OuterWall adjustment + validation + full test pins

- Task IDs: `TASK-000`
- Objective: `resolve_feedrate` adjusts OuterWall factored speeds per the canonical half-range rule; negatives and inverted ranges rejected; all behaviour ACs green.
- Precondition: Step 1 exit holds (fields resolve); `resolve_feedrate` OuterWall arm still selects `outer_wall_speed` with `clamp(0.05, 5.0)` shaping (dispatched FACT).
- Postcondition: avoidance-off identity; lower-half clamp, upper-half boost, above-max bypass, non-OuterWall identity; both negative guards reject; midpoint-boundary (`95.0` → max arm) pinned.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/src/emit.rs` - lines `205-260`
  - `crates/slicer-gcode/src/emit.rs` - lines `670-690`
  - `crates/slicer-gcode/tests/resonance_avoidance_emission_tdd.rs` - full (new file, <600 lines)
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/src/emit.rs`
  - `crates/slicer-gcode/tests/resonance_avoidance_emission_tdd.rs`
  - `crates/slicer-gcode/tests/gcode_feedrate_emission_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-ir/src/resolved_config.rs` (Step 1 owns it)
  - `docs/config/host-keys.toml` (Step 1 owns it)
  - `OrcaSlicerDocumented/...`, `target/`, `modules/...`
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - Not triggered (no new field or version constant in this step).
- Expected sub-agent dispatches:
  - Question: how does `gcode_feedrate_emission_tdd` construct a configured `DefaultGCodeEmitter` today (constructor args, config scaffolding)?; scope: `crates/slicer-gcode/tests/gcode_feedrate_emission_tdd.rs`; return: `SNIPPETS` (≤1 snippet, ≤30 lines)
  - Question: did each behaviour test pass?; scope: none (run only); return: `FACT` or `SNIPPETS` on failure
- Context cost: `M`
- Authoritative docs:
  - `docs/00_project_overview.md` - delegated SUMMARY (emitter seam only)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` - delegate; never load (`GCode::_extrude` block quoted in design.md)
- Verification:
  - `cargo test -p slicer-gcode --test resonance_avoidance_emission_tdd 2>&1 | tail -5` - FACT pass/fail (all 8 tests: schema + default + lower + upper + above-max + non-outer + 2 negatives)
  - `cargo test -p slicer-gcode --test gcode_feedrate_emission_tdd 2>&1 | tail -3` - FACT pass/fail (no regression)
  - `cargo test -p slicer-gcode --test gcode_emit_tdd 2>&1 | tail -3` - FACT pass/fail (no regression)
- Exit condition: all 8 new tests pass; both pre-existing emitter binaries green; no `Custom`/`GapFill`/`RaftInfill` arm adjusted (pinned by non-OuterWall test using SparseInfill + a Custom-spot check where the harness allows).

### Step 3: Docs, deviation, regen, gates

- Task IDs: `TASK-000`
- Objective: DEV-174 row lands collision-free; `gen-config-docs` regen threads the keys; padding absence pinned; full gates green.
- Precondition: Steps 1–2 exits hold; DEV-174 absent from both `docs/DEVIATION_LOG.md` and every `docs/spec_packets/*/` (re-derive `max(DEV-*)` over both at implementation — drafts 276/277/281 already propose DEV-171/172/173).
- Postcondition: DEV-174 row present with clauses (a)(b)(c); `docs/15_config_keys_reference.md` contains the 3 keys at canonical defaults; `ORCA_CONFIG_PADDING` untouched; lock test green; clippy + literals + check green.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/DEVIATION_LOG.md` - lines `last 25 only`
  - `docs/config/host-keys.toml` - `[resolved_config]` tail only
- Files allowed to edit (at most 3):
  - `docs/DEVIATION_LOG.md`
  - `docs/15_config_keys_reference.md`
  - `docs/specs/orca-feature-gap/issues/55-author-packet-p48-printer-machine-resonance-emitter.md`
- Files explicitly out of bounds:
  - `crates/...`, `modules/...` (Steps 1–2 own them)
  - `crates/slicer-gcode/src/serialize.rs` (AC-N3 forbids touching it)
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - Not triggered.
- Expected sub-agent dispatches:
  - Question: re-derive `max(DEV-*)` over `docs/DEVIATION_LOG.md` + `docs/spec_packets/*/` — is DEV-174 still free, else next free?; scope: `docs/DEVIATION_LOG.md`, `docs/spec_packets/`; return: `FACT`
  - Question: did each gate pass?; scope: none (run only); return: `FACT` or `SNIPPETS` on failure
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/orca-feature-gap/map.md` - Notes §Authoring rules 1–6 (zero declaration-only keys; every key drives a behaviour change at non-default value)
- OrcaSlicer refs:
  - None (no new canonical reads in this step).
- Verification:
  - `rg -q 'DEV-174' docs/DEVIATION_LOG.md && rg -q 'resonance_avoidance' docs/15_config_keys_reference.md && rg -q 'min_resonance_avoidance_speed' docs/config/host-keys.toml` - FACT pass/fail (use `rg -q 'crates\/slicer-gcode\/src\/emit\.rs::resolve_feedrate' crates/slicer-gcode/src/emit.rs || rg -q 'resolve_feedrate' crates/slicer-gcode/src/emit.rs || rg -q 'fn resolve_feedrate' crates/slicer-gcode/src/emit.rs for any symbol-path static check in this packet)
  - `cargo xtask check-literals 2>&1 | tail -3` - FACT pass/fail
  - `cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -3` - FACT pass/fail
- Exit condition: the triple-`rg` conjunction passes; `check-literals` + clippy green; ticket-55 answer records the packet dir + preflight verdict (written by the wayfinder session, not the swarm).

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | field declarations + TOML mirror + blast-radius dispatch |
| Step 2 | M | helper + hook + 8 tests + 2 no-regression binaries |
| Step 3 | S | DEV row + regen + gates |

Split before activation if aggregate cost exceeds M or any step is L.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- Reconcile reopened/superseded status transitions.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
