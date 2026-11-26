# Implementation Plan: 277-retraction-wipe-travel-firmware-emitter

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Pin canonical defaults, hop strings, slope math, placeholders, and the blast radius (read-only discovery)

- Task IDs: none (queue packet, ticket 44)
- Objective: return binding inputs for Steps 2-6: the eleven P37 defaults/types, the `z_hop_types` enum strings + port snake-case mapping, the `retract_before_wipe` percent clamp, the `travel_slope` degrees-to-radians conversion + slope-lift math, the `_cut`/`_ec` placeholder names + float spelling, the G10/G11 render shape, the `z_offset` preamble/layer sites, the `TravelRetract` speed-unit verdict, and the exhaustive `ResolvedConfig` struct-literal list. (Render home confirmed at authoring: `gcode_emit_tdd` for AC-1–AC-7/AC-N1, `machine_gcode_emit_tdd` for AC-8/AC-9; schema home: existing `resolved_config_defaults_tdd` case (Step 3) + shared `retraction_keys_schema_tdd` guard binary (Step 6).)
- Precondition: packet files exist at `docs/spec_packets/277-retraction-wipe-travel-firmware-emitter/`.
- Postcondition: a `STEP1-FINDINGS` note (posted in-session, not a file) records: (a) eleven defaults/types + port mapping, (b) hop strings + snake mapping + strict-parse set, (c) percent clamp + slope math (`dz / tan`, radians) or the divergence Step 5 takes, (d) placeholder names + float spelling, (e) G10/G11 shape + `z_offset` sites, (f) TravelRetract unit-path verdict (follow-up filed or no bug), (g) struct-literal site list for Step 2.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/machine-gcode-emit/src/lib.rs` - placeholder lookup + float formatting arms only
  - `crates/slicer-gcode/src/emit.rs` - entity retract loop + ZHop arms + preamble Z neighbourhood only
  - `crates/slicer-gcode/tests/gcode_emit_tdd.rs` - harness shape only (first 80 lines)
- Files allowed to edit (at most 3):
  - none (read-only step)
- Files explicitly out of bounds:
  - `OrcaSlicerDocumented/` (delegate only), `target/`, `ORCA_CONFIG_PADDING`, `region_mapping.rs`
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - This step IS the blast-radius survey: dispatch the struct-literal `LOCATIONS` worker before Step 2 edits anything; cite its result in the `STEP1-FINDINGS` note inline.
- Expected sub-agent dispatches:
  - Question: eleven P37 defaults/types + hop enum strings; scope: `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp`; return: `LOCATIONS` (<=10 entries)
  - Question: wipe split + slope math + G10/G11 shape; scope: `OrcaSlicerDocumented/src/libslic3r/GCodeWriter.cpp`; return: `SUMMARY` (<=150 words)
  - Question: travel gate + layer-change + Z-offset + placeholder publication; scope: `OrcaSlicerDocumented/src/libslic3r/GCode.cpp`; return: `SUMMARY` (<=150 words)
  - Question: percent clamp + slope conversion; scope: `OrcaSlicerDocumented/src/libslic3r/Extruder.cpp`; return: `SUMMARY` (<=100 words)
  - Question: every `ResolvedConfig` struct-literal site; scope: `crates/ modules/ xtask/`; return: `LOCATIONS` (<=20 entries)
- Context cost: `S` (five bounded dispatches + three ranged reads, no edits)
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` - schema-key section only
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - delegate; never load
  - `OrcaSlicerDocumented/src/libslic3r/GCodeWriter.cpp` - delegate; never load
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` - delegate; never load
  - `OrcaSlicerDocumented/src/libslic3r/Extruder.cpp` - delegate; never load
- Verification:
  - `STEP1-FINDINGS` note exists with all seven items (a)-(g) - in-session check, no cargo
- Exit condition: every (a)-(g) item is pinned or explicitly recorded as a divergence with rationale; any contradiction with `design.md` assumptions redesigns the affected step before Step 2 starts.

### Step 2: Declare the eleven host fields + struct-literal blast radius

- Task IDs: none (queue packet, ticket 44)
- Objective: eleven scalar-global `ResolvedConfig` fields with `to_config_map` inserts; every struct literal from Step 1(g) updated in this step.
- Precondition: `STEP1-FINDINGS` (a)-(g) recorded.
- Postcondition: `cargo check --workspace --all-targets` green with all eleven fields declared and reachable via `to_config_map`.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/resolved_config.rs` - `retract_length` declaration neighbourhood + `to_config_map` insert neighbourhood only
- Files allowed to edit (at most 3):
  - `crates/slicer-ir/src/resolved_config.rs`
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/emit.rs` (Steps 4-5), module manifests (Step 6), `ORCA_CONFIG_PADDING` (never), `region_mapping.rs` (ticket 126)
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - Step 1(g)'s struct-literal list lands here: every site is edited in this step alongside the declarations (test + non-test literals). Float fields gain the `to_bits`-pattern arms beside `retract_length` where that pattern applies. (The eleven-key declaration case, `host-keys.toml` entries, and DEV row land in Step 3 — this step stays at one named file plus the bullet-authorized literal sites.)
- Expected sub-agent dispatches:
  - none (all inputs pinned by Step 1)
- Context cost: `M` (macro arms + inserts + literals in one step; split if the literal list exceeds 12 sites)
- Authoritative docs:
  - none (Step 1 findings are binding)
- OrcaSlicer refs:
  - none (Step 1 findings are binding; no new reads)
- Verification:
  - `cargo check --workspace --all-targets` - FACT pass/fail (declarations compile, blast radius closed)
- Exit condition: check green on all targets; all eleven fields exist with canonical defaults (`2.0`, `false`, `100.0` percent, `18.0`, `10.0`, `3.0`, `false`, `false`, `1.0`, `"slope"`, `0.0`).

### Step 3: Prove the declarations (guard case + host-keys + DEV row)

- Task IDs: none (queue packet, ticket 44)
- Objective: the eleven-key declaration case in the existing `resolved_config_defaults_tdd` binary, `host-keys.toml` entries, and the DEV-172 row (number re-derived at implementation time).
- Precondition: Step 2 green (fields + map inserts exist and compile).
- Postcondition: guard case asserts all eleven declarations (key, type, default) and their `to_config_map` spellings; DEV-172 present in `docs/DEVIATION_LOG.md`.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/DEVIATION_LOG.md` - last 20 lines only
  - `docs/config/host-keys.toml` - `retract_length` entry neighbourhood only
- Files allowed to edit (at most 3):
  - `crates/slicer-ir/tests/resolved_config_defaults_tdd.rs`
  - `docs/config/host-keys.toml`
  - `docs/DEVIATION_LOG.md`
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/emit.rs` (Steps 4-5), module manifests (Step 6), `ORCA_CONFIG_PADDING` (never), `region_mapping.rs` (ticket 126)
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - No new field or constant in this step (Step 2 closed the literals). The next free `DEV-###` is re-derived in this step (`rg -o '^\| DEV-[0-9]{3}' docs/DEVIATION_LOG.md | sort -u | tail -1`); DEV-172 is the authoring-time expectation.
- Expected sub-agent dispatches:
  - none (all inputs pinned by Steps 1-2)
- Context cost: `S` (one test case + eleven doc entries + one DEV row)
- Authoritative docs:
  - `docs/DEVIATION_LOG.md` - last 20 lines (row format)
- OrcaSlicer refs:
  - none (Step 1 findings are binding; no new reads)
- Verification:
  - `cargo test -p slicer-ir --test resolved_config_defaults_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail (the packet authors the eleven-key case there, never a new binary for schema alone)
  - `rg -q 'retract_before_wipe' docs/config/host-keys.toml` - FACT pass/fail (use the bare key form; accept the `slicer_ir::resolved_config::ResolvedConfig::retract_before_wipe` fully-qualified form OR the in-scope `retract_before_wipe` short form OR any name-resolution-equivalent spelling — never fail on path-prefix differences alone)
- Exit condition: guard case asserts all eleven keys with canonical defaults; DEV row names the scalar-vs-vector divergence and the ticket-125 owner.

### Step 4: Wire travel gate, layer-change gate, wipe emission + split (AC-1–AC-4)

- Task IDs: none (queue packet, ticket 44)
- Objective: `retraction_minimum_travel` travel gate, `retract_when_changing_layer` layer-change gate, wipe `Move` emission (`wipe` + `wipe_distance`), and the `retract_before_wipe` split helper at the entity-retract site, with the AC-1..AC-4 tests (TDD: tests first, then logic).
- Precondition: Steps 2-3 green (fields declared, guard case + map inserts exist).
- Postcondition: AC-1 through AC-4 pass; default-path identity arms hold except the intended `retraction_minimum_travel` output change (short travels newly skip retracts), which AC-1 pins.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/src/emit.rs` - entity retract loop + layer-change path + travel-length helper neighbourhood only
  - `crates/slicer-gcode/tests/gcode_emit_tdd.rs` - travel-fixture helper neighbourhood only
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/src/emit.rs`
  - `crates/slicer-gcode/tests/gcode_emit_tdd.rs`
  - `crates/slicer-ir/src/resolved_config.rs`
- Files explicitly out of bounds:
  - `modules/core-modules/machine-gcode-emit/` (Step 6), `ORCA_CONFIG_PADDING` (never), hop/firmware/offset arms (Step 5)
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - No new field or constant in this step; Step 2's literals are not re-opened. If a helper needs a new shared constant, its test-assertion fallout lands in this step.
- Expected sub-agent dispatches:
  - none (Step 1 math is binding)
- Context cost: `M` (new emission site + split arithmetic + four behaviour tests)
- Authoritative docs:
  - none (Step 1 findings are binding)
- OrcaSlicer refs:
  - none (Step 1 findings are binding; no new reads)
- Verification:
  - `cargo test -p slicer-gcode --test gcode_emit_tdd -- minimum_travel_gates_retract 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail (AC-1)
  - `cargo test -p slicer-gcode --test gcode_emit_tdd -- retract_on_layer_change 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail (AC-2)
  - `cargo test -p slicer-gcode --test gcode_emit_tdd -- wipe_emits_move 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail (AC-3)
  - `cargo test -p slicer-gcode --test gcode_emit_tdd -- before_wipe_splits_retract 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail (AC-4)
- Exit condition: all four AC commands PASS including default-identity arms; wipe direction follows the locked invariant (reverse of last extrusion vector, exact `wipe_distance` extent; no wipe when no prior extrusion exists).

### Step 5: Wire firmware mode, hop style + slope, Z offset (AC-5–AC-7, AC-N1)

- Task IDs: none (queue packet, ticket 44)
- Objective: `use_firmware_retraction` mode selection, `z_hop_types` style arms + `travel_slope` diagonal math + strict-parse rejection, and `z_offset` Z-target shift, with the AC-5..AC-7 + AC-N1 tests (TDD: tests first, then logic).
- Precondition: Step 4 green (travel/layer/wipe arms exist).
- Postcondition: AC-5 through AC-7 and AC-N1 pass; `slope` default lift shape change is pinned by AC-6 (intended canonical alignment, not a regression).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/src/emit.rs` - ZHop arms + preamble Z + retract/unretract render neighbourhood only
  - `crates/slicer-gcode/src/serialize.rs` - lines 760-811 only (G10/G11 render proof, read-only)
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/src/emit.rs`
  - `crates/slicer-gcode/tests/gcode_emit_tdd.rs`
  - `crates/slicer-ir/src/resolved_config.rs`
- Files explicitly out of bounds:
  - `modules/core-modules/machine-gcode-emit/` (Step 6), `ORCA_CONFIG_PADDING` (never), Step 4 arms (no re-design; additive edits only)
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - No new field or constant in this step. If the hop enum needs a shared parser, its rejection-test fallout lands here.
- Expected sub-agent dispatches:
  - none (Step 1 strings and math are binding)
- Context cost: `M` (three arms + strict parse + four tests)
- Authoritative docs:
  - none (Step 1 findings are binding)
- OrcaSlicer refs:
  - none (Step 1 findings are binding; no new reads)
- Verification:
  - `cargo test -p slicer-gcode --test gcode_emit_tdd -- firmware_mode_selects_g10_g11 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail (AC-5)
  - `cargo test -p slicer-gcode --test gcode_emit_tdd -- hop_type_selects_shape 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail (AC-6)
  - `cargo test -p slicer-gcode --test gcode_emit_tdd -- z_offset_shifts_targets 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail (AC-7)
  - `cargo test -p slicer-gcode --test gcode_emit_tdd -- hop_type_rejects_unknown 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail (AC-N1)
- Exit condition: all four AC commands PASS; unknown hop strings fail fatal naming the key + four legal values; `auto` resolves per the Step 1 pin-down (slope when `travel_slope > 0`, else normal) or the recorded divergence.

### Step 6: Publish the cut/EC placeholders + docs + guests (AC-8, AC-9)

- Task IDs: none (queue packet, ticket 44)
- Objective: `machine-gcode-emit.toml` float declarations + substitution arms for `retraction_distances_when_cut`/`_ec`, the AC-8/AC-9 placeholder tests, generated reference + lock-test regen, and guest rebuild.
- Precondition: Steps 2-5 green (all emitter arms exist).
- Postcondition: AC-8 and AC-9 pass; `cargo xtask build-guests --check` exits `0` (or `1` triggers a rebuild then re-check to `0`); generated docs contain all eleven keys.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/machine-gcode-emit/src/lib.rs` - placeholder lookup neighbourhood only
  - `docs/DEVIATION_LOG.md` - last 20 lines only (DEV number confirmation)
- Files allowed to edit (at most 3):
  - `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml`
  - `modules/core-modules/machine-gcode-emit/src/lib.rs`
  - `modules/core-modules/machine-gcode-emit/tests/retraction_keys_schema_tdd.rs` (guard binary 276 authors — extend with the two distance cases, or create with them if 276 hasn't merged; queue-order merge churn) + render cases in `tests/machine_gcode_emit_tdd.rs` (one step, two test files: the guard is shared, the render cases join the proven harness)
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/emit.rs` (Steps 3-4, no re-open), `ORCA_CONFIG_PADDING` (never), `region_mapping.rs` (ticket 126)
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - The two new manifest schema rows are the schema-constant addition: guest artifacts embed them, so the freshness gate + rebuild land in this step, plus the generated-reference regen. No `ResolvedConfig` literals re-open (Step 2 closed them).
- Expected sub-agent dispatches:
  - Question: run the full gate trio and guest check; scope: workspace; return: `FACT` pass/fail per command
- Context cost: `S` (two schema rows + two arms + two tests + docs + guests)
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` - schema-key section only
  - `docs/DEVIATION_LOG.md` - last 20 lines (number confirmation)
- OrcaSlicer refs:
  - none (Step 1 placeholder pin-down is binding; no new reads)
- Verification:
  - `cargo test -p machine-gcode-emit --test machine_gcode_emit_tdd -- cut_distance_placeholder 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail (AC-8)
  - `cargo test -p machine-gcode-emit --test machine_gcode_emit_tdd -- ec_distance_placeholder 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail (AC-9)
  - `cargo test -p machine-gcode-emit --test retraction_keys_schema_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail (shared guard binary: two distance declaration cases alongside 276's bool case)
  - `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail (lint gate)
  - `cargo xtask build-guests --check` - FACT exit code (0 fresh / 1 stale-then-rebuild / 3 infra)
- Exit condition: both placeholder ACs PASS with float spelling (`25.0`/`12.0` at explicit values, `18.0`/`10.0` at defaults); generated reference and `host-keys.toml` contain all eleven keys; guests fresh.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Discovery only, no edits |
| Step 2 | M | Eleven fields + literals |
| Step 3 | S | Guard case + host-keys + DEV row |
| Step 4 | M | New wipe emission site (largest) |
| Step 5 | M | Three arms + strict parse |
| Step 6 | S | Two schema rows + guests |

Split before activation if aggregate cost exceeds M or any step is L. Aggregate is M (three M steps are sequential slices, each independently bounded; no single step is L).

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
