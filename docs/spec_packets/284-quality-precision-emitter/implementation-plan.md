# Implementation Plan: 284-quality-precision-emitter

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Declare both scalar-global fields + mirror + schema guard scaffold

- Task IDs: `TASK-000`
- Objective: both P51 keys resolve from raw config at canonical defaults; schema guard proves it; no behaviour change yet.
- Precondition: `enable_arc_fitting` is undeclared and bare `resolution` lives only in `ORCA_CONFIG_PADDING` (dispatched FACT); `declare_resolved_config!` still carries the precision/resolution window and the host-key `to_config_map` insert region.
- Postcondition: `ResolvedConfig` carries `enable_arc_fitting: bool = false` (`extract_bool_or_first`) and `resolution: f32 = 0.01` (`extract_float`, `min 0`, no `max`); `to_config_map` emits `resolution` only (arc omitted host-only with P35-comment); `host-keys.toml` `[resolved_config]` mirrors both; new test file's schema test passes; tolerance and renderer untouched.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/resolved_config.rs` - precision/resolution window only
  - `crates/slicer-ir/src/resolved_config.rs` - `to_config_map` host-key insert region only
  - `docs/config/host-keys.toml` - `[resolved_config]` window only
- Files allowed to edit (at most 3):
  - `crates/slicer-ir/src/resolved_config.rs`
  - `docs/config/host-keys.toml`
  - `crates/slicer-gcode/tests/quality_precision_arc_resolution_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/serialize.rs` (Step 2 owns tolerance)
  - `crates/slicer-gcode/src/emit.rs` (Step 3 owns arc + validation)
  - `crates/slicer-runtime/tests/unit/host_keys_doc_lock_tdd.rs` (Step 4 owns the lock arms with the DEV row)
  - `OrcaSlicerDocumented/...`, `target/`, `modules/...`
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - New `ResolvedConfig` fields ripple into every struct literal of that struct. Dispatch a `LOCATIONS` worker for `ResolvedConfig {` literal sites before editing; cite the result inline below and include every site in this step's edit list (split the step if it exceeds 3 edits — prefer one step-per-blast-radius over a wide edit).
  - LOCATIONS result (pre-authored 2026-09-07, re-derive at implementation): worker not yet run — implementer MUST dispatch `LOCATIONS: rg 'ResolvedConfig\s*\{' --type rust` (≤20 entries) and extend the edit list before writing fields.
- Expected sub-agent dispatches:
  - Question: list every `ResolvedConfig {` struct-literal site that must gain the 2 fields; scope: `crates/ modules/ xtask/`; return: `LOCATIONS`
  - Question: what extractor do neighbouring scalar `bool` / `f32` host fields use, and which 3MF spellings must the bool accept?; scope: `crates/slicer-ir/src/resolved_config.rs`; return: `FACT`
- Context cost: `M` (blast radius unknown until dispatch; split if L)
- Authoritative docs:
  - `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` - P51 rows only
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - delegate; never load
- Verification:
  - `cargo test -p slicer-gcode --test quality_precision_arc_resolution_tdd schema_declares_both_keys 2>&1 | tail -5` - FACT pass/fail or bounded failure SNIPPETS
  - `cargo check --workspace --all-targets 2>&1 | tail -3` - FACT pass/fail (blast-radius proof)
- Exit condition: schema test names both keys with types `bool` / `f32`, effective defaults `false` / `0.01`, and TOML mirror spellings; `cargo check --workspace --all-targets` green.

### Step 2: Wire the effective tolerance + resolution behaviour pins

- Task IDs: `TASK-000`
- Objective: large `resolution` dominates per-role tolerances while defaults stay identity; resolution ACs green.
- Precondition: Step 1 exit holds (fields resolve); `tolerance_for_role` still returns per-role values only (dispatched FACT).
- Postcondition: arc-off effective tolerance is `max(per_role, resolution)` and arc-on is `min(per_role, 0.2 * resolution)` with travel pinned at `0.0`; large-resolution run emits strictly fewer extrusion moves than default; defaults still emit zero arcs with HEAD-identical move counts; pre-existing per-role-tolerance binary green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/src/serialize.rs` - `tolerance_for_role` only
  - `crates/slicer-gcode/tests/quality_precision_arc_resolution_tdd.rs` - full (new file, <600 lines)
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/src/serialize.rs`
  - `crates/slicer-gcode/tests/quality_precision_arc_resolution_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-ir/src/resolved_config.rs` (Step 1 owns it)
  - `crates/slicer-gcode/src/emit.rs` (Step 3 owns arc + validation)
  - `docs/config/host-keys.toml` (Step 1 owns it)
  - `OrcaSlicerDocumented/...`, `target/`, `modules/...`
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - Not triggered (no new field or version constant in this step).
- Expected sub-agent dispatches:
  - Question: did each resolution test pass?; scope: none (run only); return: `FACT` or `SNIPPETS` on failure
- Context cost: `S`
- Authoritative docs:
  - `docs/00_project_overview.md` - delegated SUMMARY (emitter seam only)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp` - delegate; never load (`process_classic` / `process_arachne` `0.2 *` rule quoted in design.md)
- Verification:
  - `cargo test -p slicer-gcode --test quality_precision_arc_resolution_tdd default_path_is_identity 2>&1 | tail -5` - FACT pass/fail
  - `cargo test -p slicer-gcode --test quality_precision_arc_resolution_tdd large_resolution_simplifies_more 2>&1 | tail -5` - FACT pass/fail
  - `cargo test -p slicer-gcode --test gcode_emit_per_role_tolerance_tdd 2>&1 | tail -3` - FACT pass/fail (no regression)
- Exit condition: default test proves zero arcs + HEAD-identical move counts + single intended CONFIG_BLOCK value change; large-resolution test proves strictly fewer moves; pre-existing tolerance binary green.

### Step 3: Wire emitter-side arc fitting + emit-time validation + remaining pins

- Task IDs: `TASK-000`
- Objective: arc-on emits E-conserving `G2`/`G3` for arc-consistent extrusion runs with travel/lock exclusion and tightened tolerance; negatives reject through the real emitter error; arc, interaction, and negative ACs green.
- Precondition: Steps 1–2 exits hold; `emit_gcode`'s error type admits a stable validation code (dispatched SNIPPETS); extrusion still renders `G1`-only when arc is off.
- Postcondition: circle fixture with arc on contains ≥1 `G2`/`G3` line with E conserved to `1e-3` and travel still `G0`; small-arc kept counts exceed arc-off at the same resolution; negative `resolution` rejects via the stable code; travel/locked entities never arc and locked counts ignore `resolution`; pre-existing emit/golden binaries green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/src/emit.rs` - simplify + `Move`-construction window only
  - `crates/slicer-gcode/tests/quality_precision_arc_resolution_tdd.rs` - full (new file, <600 lines)
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/src/emit.rs`
  - `crates/slicer-gcode/tests/quality_precision_arc_resolution_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-ir/src/resolved_config.rs` (Step 1 owns it)
  - `crates/slicer-gcode/src/serialize.rs` (Step 2 owns tolerance; padding table never editable — AC-N3)
  - `docs/config/host-keys.toml` (Step 1 owns it)
  - `OrcaSlicerDocumented/...`, `target/`, `modules/...`
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - Not triggered (no new field or version constant in this step; arcs reuse the existing `Raw` command shape — no `GCodeCommand` variant is added).
- Expected sub-agent dispatches:
  - Question: what stable error code does `emit_gcode` return for config validation, and which existing test asserts one?; scope: `crates/slicer-gcode/src/emit.rs`, `crates/slicer-gcode/tests/`; return: `SNIPPETS` (≤1 snippet, ≤30 lines)
  - Question: did each arc/negative test pass?; scope: none (run only); return: `FACT` or `SNIPPETS` on failure
- Context cost: `M`
- Authoritative docs:
  - `docs/00_project_overview.md` - delegated SUMMARY (emitter seam only)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` - delegate; never load (extrusion `G1`-vs-arc selection quoted in design.md)
  - `OrcaSlicerDocumented/src/libslic3r/GCodeWriter.cpp` - delegate; never load (arc-density paths quoted in design.md)
- Verification:
  - `cargo test -p slicer-gcode --test quality_precision_arc_resolution_tdd 2>&1 | tail -5` - FACT pass/fail (all 8 tests: schema + default + large + arc + tightening + 2 negatives + padding-absence doc check)
  - `cargo test -p slicer-gcode --test gcode_emit_tdd 2>&1 | tail -3` - FACT pass/fail (no regression)
  - `cargo test -p slicer-gcode --test golden_emit_tdd 2>&1 | tail -3` - FACT pass/fail (no regression)
- Exit condition: all 8 new tests pass; both pre-existing binaries green; validation tests assert the stable code through the real `emit_gcode` error (not a helper-level string match); no `GCodeCommand` variant was added.

### Step 4: Docs, deviation, regen, gates

- Task IDs: `TASK-000`
- Objective: DEV-176 row lands collision-free; lock-test arms land with the mirror; `gen-config-docs` regen threads the keys; padding absence pinned; full gates green.
- Precondition: Steps 1–3 exits hold; DEV-176 absent from both `docs/DEVIATION_LOG.md` and every `docs/spec_packets/*/` (re-derive `max(DEV-*)` over both at implementation — drafts already propose DEV-172/173/174/175).
- Postcondition: DEV-176 row present with clauses (a)(b)(c)(d); `docs/15_config_keys_reference.md` contains both keys at effective `false` / `0.01`; `ORCA_CONFIG_PADDING` untouched; lock test green; clippy + literals + check green.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/DEVIATION_LOG.md` - lines `last 25 only`
  - `docs/config/host-keys.toml` - `[resolved_config]` tail only
- Files allowed to edit (at most 3):
  - `docs/DEVIATION_LOG.md`
  - `docs/15_config_keys_reference.md`
  - `crates/slicer-runtime/tests/unit/host_keys_doc_lock_tdd.rs`
- Files explicitly out of bounds:
  - `crates/...`, `modules/...` (Steps 1–3 own them)
  - `crates/slicer-gcode/src/serialize.rs` (AC-N3 forbids touching it)
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - Not triggered.
- Expected sub-agent dispatches:
  - Question: re-derive `max(DEV-*)` over `docs/DEVIATION_LOG.md` + `docs/spec_packets/*/` — is DEV-176 still free, else next free?; scope: `docs/DEVIATION_LOG.md`, `docs/spec_packets/`; return: `FACT`
  - Question: did each gate pass?; scope: none (run only); return: `FACT` or `SNIPPETS` on failure
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/orca-feature-gap/map.md` - Notes §Authoring rules 1–6 (zero declaration-only keys; every key drives a behaviour change at non-default value)
- OrcaSlicer refs:
  - None (no new canonical reads in this step).
- Verification:
  - `rg -q 'DEV-176' docs/DEVIATION_LOG.md && rg -q 'enable_arc_fitting' docs/15_config_keys_reference.md && rg -q 'enable_arc_fitting' docs/config/host-keys.toml` - FACT pass/fail (use `rg -q 'crates\/slicer-gcode\/src\/serialize\.rs::tolerance_for_role' crates/slicer-gcode/src/serialize.rs || rg -q 'tolerance_for_role' crates/slicer-gcode/src/serialize.rs || rg -q 'fn tolerance_for_role' crates/slicer-gcode/src/serialize.rs for any symbol-path static check in this packet)
  - `cargo test -p slicer-runtime --test unit host_keys_doc_lock 2>&1 | tail -3` - FACT pass/fail
  - `cargo xtask check-literals 2>&1 | tail -3` - FACT pass/fail
  - `cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -3` - FACT pass/fail
- Exit condition: the triple-`rg` conjunction passes; lock test + `check-literals` + clippy green; ticket-58 answer records the packet dir + preflight verdict (written by the wayfinder session, not the swarm).

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | field declarations + map arm + TOML mirror + blast-radius dispatch |
| Step 2 | S | effective tolerance + 2 resolution tests + 1 no-regression binary |
| Step 3 | M | arc fitter + validation + 4 remaining tests + 2 no-regression binaries |
| Step 4 | S | DEV row + lock arms + regen + gates |

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
