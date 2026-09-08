# Implementation Plan: 291-slow-down-layers-initial-layer-speed

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Declare the scalar-global field + schema guard

- Task IDs: `TASK-000`
- Objective: Declare `slow_down_layers` in `ResolvedConfig` with canonical default/bounds and pin it with a schema guard; prove the struct-literal blast radius is fully owned.
- Precondition: No `ResolvedConfig` field exists for the key (verified at authoring: zero-occurrence across `crates/` `modules/` `xtask/` outside the ticket/asset/map mentions — re-verify with the Step-1 dispatch before editing).
- Postcondition: `ResolvedConfig` carries 1× `u32 = 0` with CLI ingestion (no `@`-block bounds — none representable post-extraction); `speed_p58_slow_down_layers_emission_tdd::schema_declares_slow_down_layers` (AC-1) passes; `cargo check` is green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/resolved_config.rs` - lines covering the `declare_resolved_config!` int row syntax (`wall_loops` precedent) only
  - `docs/config/host-keys.toml` - `[resolved_config]` section only
- Files allowed to edit (at most 3):
  - `crates/slicer-ir/src/resolved_config.rs`
  - `docs/config/host-keys.toml`
  - `crates/slicer-gcode/tests/speed_p58_slow_down_layers_emission_tdd.rs` (new guard binary: AC-1 schema case only in this step)
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/serialize.rs` (padding table untouched — zero twins for the key, rule 2)
  - `crates/slicer-gcode/src/estimator.rs` (no change — it times whatever the arm leaves)
  - `modules/core-modules/machine-gcode-emit/` (wrong seam)
  - `OrcaSlicerDocumented/` (delegate; never load)
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - When the step adds a field to a struct or a new entry to a schema/version constant, list the **struct-literal blast radius** in "Files allowed to edit" — every test/non-test struct-literal site that compiles against the struct today, plus the test files that hard-assert on the constant's old value. Budget this in the step's context cost; do not let a follow-up "cargo check" discover it.
  - Dispatch a `LOCATIONS` worker for the struct-literal sites before authoring this step; cite the result inline below.
  - Blast-radius dispatch (run first): Question: every `ResolvedConfig { .. }` literal + every test asserting `ResolvedConfig::default()` int values; scope: `crates/ modules/ xtask/`; return: `LOCATIONS` (≤20 entries). Result: [implementer pastes the ≤20-entry return here and adds any listed file to "Files allowed to edit" above before editing].
- Expected sub-agent dispatches:
  - Question: `declare_resolved_config!` int row + CLI arm syntax for the `wall_loops` precedent; scope: `crates/slicer-ir/src/resolved_config.rs`; return: `SNIPPETS` (≤2 snippets, ≤30 lines each)
  - Question: struct-literal blast radius (above); scope: `crates/ modules/ xtask/`; return: `LOCATIONS` (≤20 entries)
- Context cost: `S` (split an L step)
- Authoritative docs:
  - `docs/config/host-keys.toml` - `[resolved_config]` section (direct)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - delegate; never load
- Verification:
  - `cargo test -p slicer-gcode --test speed_p58_slow_down_layers_emission_tdd schema_declares_slow_down_layers 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail or bounded failure SNIPPETS
  - `cargo check --workspace --all-targets 2>&1 | tee target/test-output.log | tail -3` - FACT pass/fail (blast-radius proof)
- Exit condition: AC-1 passes; `cargo check --workspace --all-targets` is green; no other test file changed behaviour (no blend assertion runs yet — AC-2 still unwritten).

### Step 1b: Regen generated host-keys docs

- Task IDs: `TASK-000`
- Objective: Regenerate the derived host-keys reference from the Step-1 schema so the doc-impact greps hold.
- Precondition: Step 1 green (field + host-keys.toml row landed).
- Postcondition: `docs/15_config_keys_reference.md` contains the key; `cargo xtask gen-config-docs --check` is green.
- Files allowed to read, with ranges when over 300 lines:
  - `xtask/src/gen_config_docs.rs` - invocation + `--check` gate spelling only (delegate FACT first)
- Files allowed to edit (at most 3):
  - `docs/15_config_keys_reference.md` (generated only, via the xtask — never hand-edited)
- Files explicitly out of bounds:
  - `docs/DEVIATION_LOG.md` (Step 4)
  - Hand edits to the generated reference (forbidden — regen only)
- Expected sub-agent dispatches:
  - Question: `gen-config-docs` invocation + `--check` spelling; scope: `xtask/src/gen_config_docs.rs`; return: `FACT` (≤5 lines)
- Context cost: `S` (split an L step)
- Authoritative docs:
  - `docs/15_config_keys_reference.md` - generated output (direct, grep-only)
- OrcaSlicer refs:
  - None (no canonical read in this step)
- Verification:
  - `cargo xtask gen-config-docs --check 2>&1 | tee target/test-output.log | tail -3` - FACT pass/fail
  - `rg -q 'slow_down_layers' docs/15_config_keys_reference.md && rg -q 'slow_down_layers' docs/config/host-keys.toml` - FACT pass/fail
- Exit condition: Both greps pass and `--check` is green; no hand edit to the generated file.

### Step 2: Build the blend arm

- Task IDs: `TASK-000`
- Objective: Blend layer-0-adjacent print speeds from the first-layer speeds toward the role speeds (F-only scaling, E untouched), honour the master gate, the never-slow guard, the `BottomSolidInfill` skip and the flat skirt/brim hold; the arm sits at the print-path `F` emission site only (no runtime bound exists — `u32` post-extraction — so no validation arm is built).
- Precondition: Step 1b green (schema + docs landed); per-point F-emission anchor + layer binding + clamp position located via the Step-2 dispatch before editing; no draft packet owns the position (verified at authoring — no FORWARD-DEP to read).
- Postcondition: `emit_gcode` blends through the new helper (F-only, E-conserved, travels/retracts untouched); AC-2 (identity), AC-3 (blend), AC-4 (exemptions), AC-N1 (type rejection) all have runnable pins (assertions may still be red until Step 3 fills fixtures — the arm exists and compiles).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/src/emit.rs` - lines covering the per-point `F` emission site + the per-entity loop's layer binding only
  - `crates/slicer-ir/src/slice_ir.rs` - lines `2373-2419` (`ExtrusionRole` variants) only
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/src/emit.rs`
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/serialize.rs` (no CONFIG_BLOCK change — host-only omitted)
  - `crates/slicer-gcode/src/estimator.rs` (times the blended stream automatically — do not touch)
  - `modules/core-modules/path-optimization-default/src/lib.rs` (travel-shape context — do not touch)
  - `modules/core-modules/machine-gcode-emit/` (wrong seam — do not touch)
- Expected sub-agent dispatches:
  - Question: per-point `F` emission anchor lines (the `resolve_feedrate` call with profile/entity-factor fallback) + the layer binding in scope there + clamp position relative to the call; scope: `crates/slicer-gcode/src/emit.rs`; return: `LOCATIONS` (≤10 entries, one context line each)
  - Question: `is_perimeter`-equivalent predicate derivation (which `ExtrusionRole` variants count as perimeter for the first-layer-speed selection); scope: `crates/ modules/`; return: `LOCATIONS` (≤10 entries) — `ThinWall`/`GapFill` membership decided here, pinned in a code comment
- Context cost: `M` (split an L step)
- Authoritative docs:
  - `docs/01_system_architecture.md` - Claim System section, delegated SUMMARY (arm is in-module parameter, not selection)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` - `GCode::_extrude` blend arms + `BottomSurface` exemption + post-blend `erSkirt` override order - delegate; never load
- Verification:
  - `cargo check -p slicer-gcode --all-targets 2>&1 | tee target/test-output.log | tail -3` - FACT pass/fail
  - `cargo clippy -p slicer-gcode --all-targets -- -D warnings 2>&1 | tee target/test-output.log | tail -3` - FACT pass/fail
- Exit condition: Arm compiles clippy-clean; blending is a pure function of (role, `global_layer_index`, key, first-layer speeds, entity/profile factor) with the master gate as a stage precondition; no `e` touched (F-only diff review passes); no CONFIG_BLOCK edit; no estimator/ordering/travel-path logic touched.

### Step 3: Pin behaviour at non-default values (AC-2–AC-N1)

- Task IDs: `TASK-000`
- Objective: Fill the TDD guard with the identity, blend, exemption, and type-rejection cases; prove the key changes behaviour at a non-default value (map Authoring-rule gate (b)); the identity cases need no re-baseline (defaults are byte-identical by construction — like packet 290's inert default, unlike packet 289's emitting default).
- Precondition: Step 2 green (arm compiles; helper callable from tests).
- Postcondition: `cargo test -p slicer-gcode --test speed_p58_slow_down_layers_emission_tdd` is fully green (AC-1–AC-4, AC-N1); the key has non-default behaviour assertions (AC-3/AC-4).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/src/emit.rs` - helper signatures + call sites only (no re-read of the whole file)
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/tests/speed_p58_slow_down_layers_emission_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-ir/src/resolved_config.rs` (schema frozen after Step 1)
  - Production emission logic (frozen after Step 2 — test-only step; a red that needs production change sends the packet back to Step 2)
  - `docs/` (Step 4 owns docs)
- Expected sub-agent dispatches:
  - None (test-only step; no new authority needed — all semantics pinned in Steps 1–2).
- Context cost: `M` (split an L step)
- Authoritative docs:
  - None new (all authority consumed in Steps 1–2)
- OrcaSlicer refs:
  - None (no new canonical read in this step)
- Verification:
  - `cargo test -p slicer-gcode --test speed_p58_slow_down_layers_emission_tdd 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail or bounded failure SNIPPETS
  - `cargo test -p slicer-gcode --test gcode_emit_tdd 2>&1 | tee target/test-output.log | tail -3` - FACT pass/fail (neighbour-identity proof — defaults unblended, so no churn expected)
- Exit condition: Guard fully green; the key has non-default assertions; no production file changed in this step.

### Step 4: Record deviations + annotate the queue ledger

- Task IDs: `TASK-000`
- Objective: File the DEV-183 row (extractor-contract note, bottom/skirt/brim port divergences, dead-arm + raft-comment record) and annotate the 04 tier table + 05 packet list so P58 reads 1-in at packet 291; prove doc freshness.
- Precondition: Steps 1b–3 green (schema, arm, and pins landed).
- Postcondition: `docs/DEVIATION_LOG.md` carries DEV-183 with (a)+(b)+(c) clauses; 04's Speed/Initial-layer-speed row points at packet 291; 05's P58 section reads 1 key in; `cargo xtask gen-config-docs --check` and `cargo xtask check-deviations --check` (if the repo gates it) are green.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/DEVIATION_LOG.md` - DEV-row format + max-ID region only
  - `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` - Speed/Initial layer speed section only
  - `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md` - P58 section only
- Files allowed to edit (at most 3):
  - `docs/DEVIATION_LOG.md`
  - `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md`
  - `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md`
- Files explicitly out of bounds:
  - `crates/` (production frozen — test/docs-only step)
  - `docs/15_config_keys_reference.md` (regen-only; Step 1b owns it)
- Expected sub-agent dispatches:
  - Question: DEV-row exact format + current max DEV id at implementation time; scope: `docs/DEVIATION_LOG.md`; return: `FACT` (≤5 lines)
- Context cost: `S` (split an L step)
- Authoritative docs:
  - `docs/DEVIATION_LOG.md` - row format (direct, grep-only)
- OrcaSlicer refs:
  - None (deviation text was grounded in Steps 1–2)
- Verification:
  - `rg -q 'DEV-183' docs/DEVIATION_LOG.md` - FACT pass/fail
  - `cargo xtask gen-config-docs --check 2>&1 | tee target/test-output.log | tail -3` - FACT pass/fail
  - `cargo test -p slicer-gcode --test speed_p58_slow_down_layers_emission_tdd 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail (final re-dispatch)
- Exit condition: DEV-183 greps; ledger rows read 1-in at 291; all three verification commands pass; no queue-count change; no new fog graduated, nothing ruled out of scope.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Schema + guard; blast-radius dispatch owned here |
| Step 1b | S | Regen only |
| Step 2 | M | Blend arm + selection/guard/exemption branches |
| Step 3 | M | Behaviour pins, no re-baseline (defaults identical) |
| Step 4 | S | DEV row + ledger annotation |

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
