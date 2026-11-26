# Implementation Plan: 276-retraction-toolchange-restart-lift-emitter

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Pin canonical strings, comparisons, and the blast radius (read-only discovery)

- Task IDs: none (queue packet, ticket 43)
- Objective: return binding inputs for Steps 2-4: exact `retract_lift_enforce` enum key strings + the five numeric defaults, the `lazy_lift`/`eager_lift` comparison shape, the `_cut` placeholder name/shape, the `TravelRetract` speed-unit finding, and the exhaustive `ResolvedConfig` struct-literal list. (Render home confirmed at authoring: `machine_gcode_emit_tdd`; schema home: new `retraction_keys_schema_tdd` guard binary.)
- Precondition: packet files exist at `docs/spec_packets/276-retraction-toolchange-restart-lift-emitter/`.
- Postcondition: a `STEP1-FINDINGS` note (posted in-session, not a file) records: (a) enforce strings + port mapping, (b) bound comparison (strict/inclusive, zero sentinel) or the divergence Step 3 takes, (c) placeholder name + bool spelling precedent, (d) TravelRetract unit-path verdict (follow-up filed or no bug), (e) struct-literal site list for Step 2.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/machine-gcode-emit/src/lib.rs` - placeholder lookup + `format_placeholder_value` + one bool arm only
  - `crates/slicer-gcode/src/emit.rs` - toolchange synthesis sites + `retract_length_for_tool` only
  - `crates/slicer-gcode/tests/gcode_toolchange_wrapping.rs` - harness shape only (first 80 lines)
- Files allowed to edit (at most 3):
  - none (read-only step)
- Files explicitly out of bounds:
  - `OrcaSlicerDocumented/` (delegate only), `target/`, P37 wipe sites, `region_mapping.rs`
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - This step IS the blast-radius survey: dispatch the struct-literal `LOCATIONS` worker before Step 2 edits anything; cite its result in the `STEP1-FINDINGS` note inline.
- Expected sub-agent dispatches:
  - Question: enforce enum strings + five numeric defaults; scope: `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp`; return: `LOCATIONS` (<=10 entries)
  - Question: bound-comparison shape; scope: `OrcaSlicerDocumented/src/libslic3r/GCodeWriter.cpp`; return: `SUMMARY` (<=150 words)
  - Question: `_cut` placeholder publication; scope: `OrcaSlicerDocumented/src/libslic3r/GCode.cpp`; return: `SUMMARY` (<=150 words)
  - Question: every `ResolvedConfig` struct-literal site; scope: `crates/ modules/ xtask/`; return: `LOCATIONS` (<=20 entries)
- Context cost: `S` (four bounded dispatches + three ranged reads, no edits)
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` - schema-key section only
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - delegate; never load
  - `OrcaSlicerDocumented/src/libslic3r/GCodeWriter.cpp` - delegate; never load
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` - delegate; never load
- Verification:
  - `STEP1-FINDINGS` note exists with all five items (a)-(e) - in-session check, no cargo
- Exit condition: every (a)-(e) item is pinned or explicitly recorded as a divergence with rationale; any contradiction with `design.md` assumptions redesigns the affected step before Step 2 starts.

### Step 2: Declare the eight host fields (TDD schema first)

- Task IDs: none (queue packet, ticket 43)
- Objective: eight scalar-global `ResolvedConfig` fields with `to_config_map` inserts, `host-keys.toml` + lock-test entries, DEV-171 row, and a schema-shape test proving each declaration; every struct literal from Step 1(e) updated in this step.
- Precondition: `STEP1-FINDINGS` (a)-(e) recorded.
- Postcondition: `cargo check --workspace --all-targets` green; new unit test asserts all eight declarations (key, type, default, min-where-canonical) and their `to_config_map` spellings; DEV-171 present in `docs/DEVIATION_LOG.md`.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/resolved_config.rs` - `retract_length` declaration neighbourhood + `to_config_map` insert neighbourhood only
  - `docs/DEVIATION_LOG.md` - last 20 lines only
  - `docs/config/host-keys.toml` - `retract_length` entry neighbourhood only
- Files allowed to edit (at most 3):
  - `crates/slicer-ir/src/resolved_config.rs`
  - `docs/config/host-keys.toml`
  - `docs/DEVIATION_LOG.md`
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/emit.rs` (Step 3), module manifests (Step 4), `ORCA_CONFIG_PADDING` (never), `region_mapping.rs` (ticket 126)
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - Step 1(e)'s struct-literal list lands here: every site is edited in this step alongside the declarations (test + non-test literals), plus the `host_keys_doc_lock_tdd` lock test update (packet-267 precedent). The `PartialEq`/`Hash` arms beside `retract_length` (`to_bits` pattern) gain the new float fields in the same edit.
- Expected sub-agent dispatches:
  - none (all inputs pinned by Step 1)
- Context cost: `M` (macro arms + inserts + literals + docs in one step; split if the literal list exceeds 12 sites)
- Authoritative docs:
  - `docs/DEVIATION_LOG.md` - last 20 lines (row format)
- OrcaSlicer refs:
  - none (Step 1 findings are binding; no new reads)
- Verification:
  - `cargo check --workspace --all-targets` - FACT pass/fail (blast radius closed)
  - `cargo test -p slicer-ir --test resolved_config_defaults_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail (the packet authors the eight-key case there, never a new binary for schema alone)
  - `rg -q 'DEV-171' docs/DEVIATION_LOG.md && rg -q 'retract_length_toolchange' docs/config/host-keys.toml` - FACT pass/fail
- Exit condition: check green on all targets; schema test asserts all eight keys with canonical defaults (`10.0`, `0.0` x4, `0.0` deretraction, `"all_surfaces"`, `false`); DEV-171 names the scalar-vs-vector divergence and the ticket-125 owner.

### Step 3: Wire the emitter decisions + eight behaviour tests

- Task IDs: none (queue packet, ticket 43)
- Objective: toolchange-length resolver with per-tool override, unretract length/speed overrides, ZHop gating helper + strict enforce parse, and the AC-1..AC-7 + AC-N1 tests (TDD: tests first, then logic).
- Precondition: Step 2 green (fields + map inserts exist).
- Postcondition: AC-1 through AC-7 and AC-N1 pass; default-path identity arms hold except the intended toolchange-length change (2.0 -> 10.0), which its own test pins.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/src/emit.rs` - entity retract loop, the single unretract site, both toolchange synthesis sites, both ZHop arms, `retract_length_for_tool` only
  - `crates/slicer-gcode/src/serialize.rs` - lines 760-811 only (render shape the tests assert against)
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/src/emit.rs`
  - `crates/slicer-gcode/tests/gcode_emit_tdd.rs`
  - `crates/slicer-gcode/tests/gcode_toolchange_wrapping.rs`
- Files explicitly out of bounds:
  - `crates/slicer-ir/src/resolved_config.rs` (Step 2 owns it), module files (Step 4), `estimator.rs`, `ORCA_CONFIG_PADDING`
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - not triggered (no new field or constant; new tests only)
- Expected sub-agent dispatches:
  - none (implementer works the four bounded sites directly)
- Context cost: `M` (largest step: emitter logic + 8 tests)
- Authoritative docs:
  - none beyond packet files (all authority pinned in Steps 1-2)
- OrcaSlicer refs:
  - none (Step 1 findings are binding)
- Verification:
  - `cargo test -p slicer-gcode --test gcode_emit_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail (AC-2/AC-4/AC-5/AC-6/AC-7/AC-N1 single-filter pipes in `packet.spec.md` are the falsifiers; libtest takes one filter per invocation)
  - `cargo test -p slicer-gcode --test gcode_toolchange_wrapping 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail (AC-1/AC-3 single-filter pipes likewise)
- Exit condition: all eight pipe commands behind AC-1..AC-7 + AC-N1 return PASS, including every default-identity arm; bound tests use far-from-boundary values per the Step 1 comparison finding.

### Step 4: Publish the long-retraction placeholder (module seam)

- Task IDs: none (queue packet, ticket 43)
- Objective: `long_retractions_when_cut` declared in `machine-gcode-emit.toml`, published through the existing placeholder lookup with canonical `1`/`0` spelling, new schema-guard binary + render case green, stale guest rebuilt.
- Precondition: Steps 2-3 green; Step 1(c) placeholder name pinned (render home confirmed at authoring: `machine_gcode_emit_tdd`).
- Postcondition: AC-8 passes; `cargo xtask build-guests --check` returns fresh (exit 0) after rebuilding the touched guest.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/machine-gcode-emit/src/lib.rs` - placeholder lookup construction + `format_placeholder_value` (generic word-form `Bool` rendering) only
- Files allowed to edit (at most 3):
  - `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml`
  - `modules/core-modules/machine-gcode-emit/src/lib.rs`
  - `modules/core-modules/machine-gcode-emit/tests/retraction_keys_schema_tdd.rs` (new guard binary) + render case in `tests/machine_gcode_emit_tdd.rs` (one step, two test files: the guard is a new file, the render case joins the proven harness)
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/emit.rs` (Step 3 owns it), `ORCA_CONFIG_PADDING` (never), any other module's manifest
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - manifest-schema addition: the schema-guard test binary is extended in the same step; guest rebuild rides this step (freshness gate below), not a later one.
- Expected sub-agent dispatches:
  - none
- Context cost: `S` (one schema row + one lookup arm + tests)
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` - schema-key section (conformance re-check only)
- OrcaSlicer refs:
  - none (Step 1(c) is binding)
- Verification:
  - `cargo test -p machine-gcode-emit --test machine_gcode_emit_tdd -- long_retraction_placeholder 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail
  - `cargo test -p machine-gcode-emit --test retraction_keys_schema_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail
  - `cargo xtask build-guests --check` - FACT exit code (rebuild without `--check` on stale, then re-run both test binaries)
- Exit condition: AC-8 PASS on a rebuilt-fresh tree; no other guest affected (only `machine-gcode-emit` stales by construction).

### Step 5: Regenerate docs, run the gates, close the packet

- Task IDs: none (queue packet, ticket 43)
- Objective: generated config reference regenerated with zero unexpected diffs, full targeted suites + workspace gates green, packet ready for `status: implemented` (swarm-time activation stays separate).
- Precondition: Steps 2-4 green.
- Postcondition: `docs/15_config_keys_reference.md` contains the eight host keys; deviation gate shows no new default-deviation row (DEV-171 is a behaviour record, not a default mismatch); all gate commands PASS.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/15_config_keys_reference.md` - new-key entries only (diff review)
- Files allowed to edit (at most 3):
  - `docs/15_config_keys_reference.md` (via the generator, never by hand)
  - `docs/spec_packets/276-retraction-toolchange-restart-lift-emitter/packet.spec.md` (status line only, and only on explicit activation request — otherwise stays `draft`)
- Files explicitly out of bounds:
  - everything else (no code changes in the gate step)
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - not triggered (no new field or constant)
- Expected sub-agent dispatches:
  - Question: regenerate + diff the config reference; scope: `cargo xtask gen-config-docs` output; return: `FACT` (eight keys present, no other diffs) — cargo runs always delegate
  - Question: full e2e suite (default-output audit for the intended toolchange-length change); scope: runtime e2e binary; return: `FACT` pass/fail + `SNIPPETS` (<=20 lines) naming any fixture whose diff is NOT the toolchange-length line
- Context cost: `S` (gates + diff review via dispatches)
- Authoritative docs:
  - `docs/11_operational_governance_and_acceptance_gate.md` - gate criteria only (delegated range)
- OrcaSlicer refs:
  - none
- Verification:
  - `cargo check --workspace --all-targets` - FACT pass/fail
  - `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail
  - `cargo test -p slicer-gcode --test gcode_emit_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail
  - `rg -q 'retract_length_toolchange' docs/15_config_keys_reference.md` - FACT pass/fail
- Exit condition: every pipe-suffixed AC command (AC-1..AC-8, AC-N1) re-dispatched PASS; e2e diffs limited to the intended toolchange-length lines; packet stays `draft` unless activation was explicitly requested with no open `[BLOCK]`.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Discovery only; four bounded dispatches |
| Step 2 | M | Fields + inserts + literals + docs; split if >12 literal sites |
| Step 3 | M | Largest step: emitter logic + 8 tests |
| Step 4 | S | One manifest row + one lookup arm |
| Step 5 | S | Delegated gates + diff review |

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
