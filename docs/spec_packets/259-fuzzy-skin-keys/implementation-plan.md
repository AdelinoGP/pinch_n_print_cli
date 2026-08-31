# Implementation Plan: fuzzy-skin-keys

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs. This packet carries no `docs/07` task IDs (queue precedent, `task_ids: []`); implementation is recorded against wayfinder ticket 14.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Declare keys + create manifest guard

- Task IDs: none (wayfinder ticket 14)
- Objective: add the seven `[config.schema]` tables to `fuzzy-skin.toml` exactly as AC-1 pins them (types/defaults/bounds/enum order/display/group) and author the net-new guard test `fuzzy_config_schema_tdd.rs` asserting that exact table (including the drift-fail naming, AC-N2).
- Precondition: tree green; `cargo xtask build-guests --check` exit 0 before starting (manifest edits are guest-fingerprint inputs).
- Postcondition: manifest parses; guard test passes; no source change yet — module behavior unchanged.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/fuzzy-skin/fuzzy-skin.toml` - full (58 lines)
  - `modules/core-modules/part-cooling/tests/cooling_config_schema_tdd.rs` - full (guard pattern; ~110 lines)
  - `modules/core-modules/seam-planner-default/seam-planner-default.toml` - lines `27-33` (enum-table form)
- Files allowed to edit (at most 3):
  - `modules/core-modules/fuzzy-skin/fuzzy-skin.toml`
  - `modules/core-modules/fuzzy-skin/Cargo.toml` (add `toml = "0.8"` to `[dev-dependencies]` — add-if-absent, packet 257/258 Step 1 precedent)
  - `modules/core-modules/fuzzy-skin/tests/fuzzy_config_schema_tdd.rs` (net-new)
- Files explicitly out of bounds:
  - `modules/core-modules/fuzzy-skin/src/lib.rs` (Step 2's surface)
  - `crates/slicer-gcode/src/serialize.rs` (padding is read-only, AC-5)
  - everything outside `modules/core-modules/fuzzy-skin/`
- Blast-radius discipline: TOML manifest tables are additive; the guard test is net-new — no struct literals or constants change, so no blast radius beyond the dev-dep add. If a prior packet already added the `toml` dev-dep, skip the edit (verify, don't assume).
- Expected sub-agent dispatches:
  - Question: after the manifest edit, is the module still loadable with its schema intact (load-or-none check over the real manifest)?; scope: `modules/core-modules/fuzzy-skin/`; return: `FACT`; purpose: guard against TOML/table-form mistakes.
- Context cost: `S`
- Authoritative docs:
  - `docs/15_config_keys_reference.md` - delegated; verify with `cargo xtask gen-config-docs --check` at packet close only (doc impact lands in Step 4).
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - delegate; never load (evidence captured in `requirements.md` §Per-Key Canonical Evidence).
- Verification:
  - `cargo test -p fuzzy-skin --test fuzzy_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail
  - `cargo xtask build-guests --check; echo "exit=$?"` - FACT exit 0 (manifest fingerprint changed; rebuild if stale, then re-run)
- Exit condition: guard green + guests fresh.

### Step 2: Wire both gates + full test fallout

- Task IDs: none (wayfinder ticket 14)
- Objective: teach `FuzzySkinModule` two gates with default identity — (a) the `fuzzy_skin` loop-selection gate: `disabled_fuzzy`/`hole` → no candidates, `external`/`all` → `LoopType::Outer` (perimeter_index 0), `allwalls` → every wall loop, `none` → `LoopType::Outer` with the per-vertex flag gate; (b) the `fuzzy_skin_first_layer` layer gate: `!fuzzy_skin_first_layer && layer_index == 0` → pass every wall through unchanged. Implement in `from_config` (two new fields with fallback-to-default reads, tree-support-planner enum pattern for `fuzzy_skin`) and in `run_wall_postprocess` (layer gate at the top, loop-selection gate per wall, existing `apply_to_all || flags.any(fuzzy_skin)` unchanged inside candidates). Update the existing `fuzzy_skin_tdd.rs` and `closed_loop_tdd.rs` tests in the same step: per-vertex-flag tests set `fuzzy_skin = "none"` (painted-only — the flag path's faithful enum value), apply-to-all tests set `fuzzy_skin = "all"` (preserving their intent: all outer walls perturbed regardless of flags), and all perturbation tests run at layer 1 (or set `fuzzy_skin_first_layer = true`), with measured justification — the gates are canonical-alignment behavior changes (default `disabled_fuzzy` is inert; layer 0 passes through at default).
- Precondition: Step 1 exit met (manifest declares the keys, guard green, guests fresh).
- Postcondition: AC-2 and AC-3 pass; AC-N1's inertness and default-path identity hold — absent keys leave output byte-identical; both existing suites green.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/fuzzy-skin/src/lib.rs` - full (~336 lines, the change surface)
  - `modules/core-modules/tree-support-planner/src/lib.rs` - lines `226-234` (enum read fallback pattern)
  - `modules/core-modules/fuzzy-skin/tests/fuzzy_skin_tdd.rs` - full (~413 lines, existing assertions to update)
  - `modules/core-modules/fuzzy-skin/tests/closed_loop_tdd.rs` - full (~175 lines)
- Files allowed to edit (at most 3):
  - `modules/core-modules/fuzzy-skin/src/lib.rs`
  - `modules/core-modules/fuzzy-skin/tests/fuzzy_skin_tdd.rs` (+enum-selection tests, +first-layer tests, +gap-keys-inert test, +default-identity test; existing layer-0/apply-to-all tests updated)
  - `modules/core-modules/fuzzy-skin/tests/closed_loop_tdd.rs` (layer-0 fixtures updated to layer 1)
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/serialize.rs` (AC-5)
  - `crates/slicer-model-io/src/loader.rs` (sidecar allowlist is read-only context)
  - `OrcaSlicerDocumented/` (delegate any further canonical reads)
- Blast-radius discipline: `FuzzySkinModule` gains two private fields — literal-construction sites are `from_config` only (`run_wall_postprocess` consumes `&self`); the two test files listed above are the only consumers whose assertions can wobble, and both are in the edit list. The layer gate changes every perturbation test that runs at layer 0 — pre-baked into this step's edit list, never deferred to a follow-up `cargo check`.
- Expected sub-agent dispatches:
  - Question: which existing tests construct `FuzzySkinModule` literally or assert exact perturbation at layer 0, and do they still pass after the two gates?; scope: `modules/core-modules/fuzzy-skin/tests/`; return: `LOCATIONS` (≤20); purpose: confirm blast radius is exactly the two test files.
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md` §CONFIG_BLOCK contract - delegated SUMMARY only if the worker needs the padding rule restated (it is already pinned in AC-5).
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Feature/FuzzySkin/FuzzySkin.cpp` `should_fuzzify` - delegate (the type/first-layer gate semantics this wiring mirrors).
- Verification:
  - `cargo test -p fuzzy-skin --test fuzzy_skin_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail (AC-2/3/N1)
  - `cargo test -p fuzzy-skin --test closed_loop_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail (updated fixtures)
  - `cargo xtask build-guests --check; echo "exit=$?"` - FACT exit 0 (src edits are fingerprint inputs)
- Exit condition: AC-2 + AC-3 + AC-N1 command-green; both suites green; guests fresh.

### Step 3: Integration arms — bounds/enum + CONFIG_BLOCK + padding correction

- Task IDs: none (wayfinder ticket 14)
- Objective: add the scheduler enum/bounds rejection tests (AC-4: `fuzzy_skin = "bogus"` → enum `TypeMismatch`; `fuzzy_skin_octaves = 0` → `OutOfRange`; `fuzzy_skin_scale = 600.0` → `OutOfRange`) against the real `fuzzy-skin.toml` manifest, and the runtime CONFIG_BLOCK tests (AC-5: explicit `fuzzy_skin = "external"` → exactly one `; fuzzy_skin = external`; defaults → `; fuzzy_skin = disabled_fuzzy` and `; fuzzy_skin_mode = displacement` present, the other five absent). Apply the one-line `ORCA_CONFIG_PADDING` value correction in `crates/slicer-gcode/src/serialize.rs` (`("fuzzy_skin", "none")` → `("fuzzy_skin", "disabled_fuzzy")`) and check the runtime binary's existing CONFIG_BLOCK tests for fallout (any test asserting the old `; fuzzy_skin = none` line is updated with measured justification).
- Precondition: Steps 1–2 exit met.
- Postcondition: AC-4 and AC-5 pass against the real manifest and the real pipeline driver; the padding table has the same entry count as before.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` - full (460-ish lines — bounded read)
  - `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` - lines `1-120` (setup) + grep for an existing CONFIG_BLOCK assertion to mirror (do not read all 1040 lines)
  - `crates/slicer-gcode/src/serialize.rs` - lines `490-560` only (the `ORCA_CONFIG_PADDING` table; the one editable entry is `fuzzy_skin`)
- Files allowed to edit (at most 3):
  - `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs`
  - `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs`
  - `crates/slicer-gcode/src/serialize.rs` (the single `fuzzy_skin` padding value; no other line)
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/serialize.rs` beyond the single `fuzzy_skin` padding value (AC-5 pins the entry count)
  - `docs/15_config_keys_reference.md` (generated; Step 4 only)
- Blast-radius discipline: test-only additions to existing binaries plus one padding value; no production surface beyond the padding line. If the runtime binary's driver needs a config key it asserts on, use the same per-test config mechanism its current tests use (verified present at authoring: per-test config injection exists in `gcode_header_thumbnail_config_blocks_tdd.rs`, exercised for packet 258's keys). Any existing test asserting the old `; fuzzy_skin = none` padding line is in this step's edit list.
- Expected sub-agent dispatches: none — the tests are the direct verification.
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md` §CONFIG_BLOCK - delegated SUMMARY (padding rule), already applied.
- OrcaSlicer refs: none — integration arms are port-side only.
- Verification:
  - `cargo test -p slicer-scheduler --test integration config_bounds_enforcement_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail (AC-4)
  - `cargo test -p slicer-runtime --test integration gcode_header_thumbnail_config_blocks_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail (AC-5)
- Exit condition: AC-4 + AC-5 green; padding entry count unchanged.

### Step 4: Regenerate docs + close gates

- Task IDs: none (wayfinder ticket 14)
- Objective: run `cargo xtask gen-config-docs` to regenerate `docs/15_config_keys_reference.md` with the seven keys; verify `--check` passes and the keys appear (AC-6); then the packet completion gate.
- Precondition: Steps 1–3 exit met.
- Postcondition: AC-6 exit 0 and keys present; all AC commands green.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/15_config_keys_reference.md` - NEVER load in full; verify via `rg` only (key-presence; the doc has no per-module subheadings — rows carry the owner column, so `rg -n 'fuzzy_skin_scale' docs/15_config_keys_reference.md` is the AC-6 check)
- Files allowed to edit (at only via the generator):
  - `docs/15_config_keys_reference.md` - through `cargo xtask gen-config-docs` only
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/serialize.rs` (padding)
  - any hand-edit of the generated doc
- Blast-radius discipline: none (generated doc).
- Expected sub-agent dispatches:
  - Question: does `gen-config-docs --check` pass and do the seven keys appear in the generated module-key table (owner column `fuzzy-skin`)?; scope: `docs/15_config_keys_reference.md`; return: `FACT`; purpose: AC-6.
- Context cost: `S`
- Authoritative docs:
  - `docs/15_config_keys_reference.md` - generated; key-presence rg-verified.
- OrcaSlicer refs: none.
- Verification:
  - `cargo xtask gen-config-docs --check && rg -q 'fuzzy_skin_scale' docs/15_config_keys_reference.md && rg -q 'fuzzy_skin_first_layer' docs/15_config_keys_reference.md; echo "exit=$?"` - FACT
  - `cargo check --workspace --all-targets` - FACT pass/fail
  - `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail
- Exit condition: AC-6 green; workspace gates green.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | manifest + guard test only |
| Step 2 | M | two gates, dual test-file fallout, blast-radius confirmed by dispatch |
| Step 3 | M | two test-file additions + one padding value correction |
| Step 4 | S | generator + gates |

Split before activation if aggregate cost exceeds M or any step is L.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read — **re-derive the crosswalk question at completion time**, not from this frozen note (ledger-fact rule). The feature-gap queue's packets carry no TASK row (survey precedent at 234a/253/254/255/256/257/258 authoring time); implementation is recorded against wayfinder ticket 14.
- Reconcile reopened/superseded status transitions.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
