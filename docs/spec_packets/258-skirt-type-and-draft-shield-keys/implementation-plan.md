# Implementation Plan: skirt-type-and-draft-shield-keys

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs. This packet carries no `docs/07` task IDs (queue precedent, `task_ids: []`); implementation is recorded against wayfinder ticket 13.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Declare keys + create manifest guard

- Task IDs: none (wayfinder ticket 13)
- Objective: add the five `[config.schema]` tables to `skirt-brim.toml` exactly as AC-1 pins them (types/defaults/bounds/enum order/display/group) and author the net-new guard test `skirt_config_schema_tdd.rs` asserting that exact table (including the drift-fail naming, AC-N2).
- Precondition: tree green; `cargo xtask build-guests --check` exit 0 before starting (manifest edits are guest-fingerprint inputs).
- Postcondition: manifest parses; guard test passes; no source change yet — module behavior unchanged.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/skirt-brim/skirt-brim.toml` - full (82 lines)
  - `modules/core-modules/part-cooling/tests/cooling_config_schema_tdd.rs` - full (guard pattern; ~120 lines)
  - `modules/core-modules/seam-planner-default/seam-planner-default.toml` - lines `27-33` (enum-table form)
- Files allowed to edit (at most 3):
  - `modules/core-modules/skirt-brim/skirt-brim.toml`
  - `modules/core-modules/skirt-brim/Cargo.toml` (add `toml = "0.8"` to `[dev-dependencies]` — add-if-absent, packet 257 Step 1 precedent)
  - `modules/core-modules/skirt-brim/tests/skirt_config_schema_tdd.rs` (net-new)
- Files explicitly out of bounds:
  - `modules/core-modules/skirt-brim/src/lib.rs` (Step 2's surface)
  - `crates/slicer-gcode/src/serialize.rs` (padding is read-only, AC-6)
  - everything outside `modules/core-modules/skirt-brim/`
- Blast-radius discipline: TOML manifest tables are additive; the guard test is net-new — no struct literals or constants change, so no blast radius beyond the dev-dep add. If packet 257 already added the `toml` dev-dep, skip the edit (verify, don't assume).
- Expected sub-agent dispatches:
  - Question: after the manifest edit, is the module still loadable with its schema intact (load-or-none check over the real manifest)?; scope: `modules/core-modules/skirt-brim/`; return: `FACT`; purpose: guard against TOML/table-form mistakes.
- Context cost: `S`
- Authoritative docs:
  - `docs/15_config_keys_reference.md` - delegated; verify with `cargo xtask gen-config-docs --check` at packet close only (doc impact lands in Step 5).
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - delegate; never load (evidence captured in `requirements.md` §Per-Key Canonical Evidence).
- Verification:
  - `cargo test -p skirt-brim --test skirt_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail
  - `cargo xtask build-guests --check; echo "exit=$?"` - FACT exit 0 (manifest fingerprint changed; rebuild if stale, then re-run)
- Exit condition: guard green + guests fresh.

### Step 2: Wire `draft_shield` + `single_loop_draft_shield`

- Task IDs: none (wayfinder ticket 13)
- Objective: teach `SkirtBrim` two gates with default identity — (a) `draft_shield = "enabled"` extends the skirt layer span to the full layer set; (b) `single_loop_draft_shield = true` limits every `global_layer_index > 0` layer to the innermost skirt loop. Implement in `from_config` (two new fields with fallback-to-default reads, tree-support-planner enum pattern for `draft_shield`), in `run_finalization` (span selection + per-layer count), and in `process()` (same gates).
- Precondition: Step 1 exit met (manifest declares the keys, guard green, guests fresh).
- Postcondition: AC-2 and AC-3 pass; AC-2's disabled arm, AC-N1's inertness, and default-path identity hold — absent keys leave output byte-identical.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/skirt-brim/src/lib.rs` - full (~420 lines, the change surface)
  - `modules/core-modules/tree-support-planner/src/lib.rs` - lines `226-234` (enum read fallback pattern)
  - `modules/core-modules/skirt-brim/tests/finalization_live_tdd.rs` - full (live-path driver setup)
  - `modules/core-modules/skirt-brim/tests/skirt_brim_tdd.rs` - full (existing generator assertions to mirror)
- Files allowed to edit (at most 3):
  - `modules/core-modules/skirt-brim/src/lib.rs`
  - `modules/core-modules/skirt-brim/tests/finalization_live_tdd.rs` (+2 tests: span-enabled, span-disabled identity)
  - `modules/core-modules/skirt-brim/tests/skirt_brim_tdd.rs` (+2 tests: single-wall upper layers, default loop count)
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/serialize.rs` (AC-6)
  - `OrcaSlicerDocumented/` (delegate any further canonical reads)
- Blast-radius discipline: `SkirtBrim` gains two private fields — literal-construction sites are `from_config` only (`run_finalization`/`process` consume `&self`); the two test files listed above are the only consumers whose assertions can wobble; both are in the edit list.
- Expected sub-agent dispatches:
  - Question: which existing tests construct `SkirtBrim` literally or assert exact loop counts, and do they still pass?; scope: `modules/core-modules/skirt-brim/tests/`; return: `LOCATIONS` (≤20); purpose: confirm blast radius is exactly the two test files.
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md` §CONFIG_BLOCK contract - delegated SUMMARY only if the worker needs the padding rule restated (it is already pinned in AC-6).
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` `GCode::generate_skirt` - delegate (the `!first_layer` single-wall condition and layer-keying evidence).
  - `OrcaSlicerDocumented/src/libslic3r/Print.cpp` `Print::has_infinite_skirt` - delegate (span semantics).
- Verification:
  - `cargo test -p skirt-brim --test finalization_live_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail (AC-2)
  - `cargo test -p skirt-brim --test skirt_brim_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail (AC-3, plus the loop-count default arm)
  - `cargo xtask build-guests --check; echo "exit=$?"` - FACT exit 0 (src edits are fingerprint inputs)
- Exit condition: AC-2 + AC-3 command-green; guests fresh.

### Step 3: Wire `skirt_start_angle` start corner

- Task IDs: none (wayfinder ticket 13)
- Objective: when `global_layer_index == 0`, rotate the first (innermost) skirt loop's point ring so it begins at the corner angularly nearest the canonical desired start point: center = the loop's own bbox center, `r` = half-diagonal, desired = center + r·(cos θ, sin θ), corner-nearest selection with a total tie-break (lower corner index wins). `skirt_start_angle` default −135° must select the existing start corner so the default path is byte-identical (AC-4 identity clause). Keep the closed-ring invariant (first point re-appended as closing point).
- Precondition: Step 2 exit met.
- Postcondition: AC-4 passes including its ±180°/wrap-around/tie arms and default identity; `order_lock` stays `None`; no other loop touched.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/skirt-brim/src/lib.rs` - full
  - `modules/core-modules/skirt-brim/tests/skirt_brim_tdd.rs` - full
- Files allowed to edit (at most 3):
  - `modules/core-modules/skirt-brim/src/lib.rs`
  - `modules/core-modules/skirt-brim/tests/skirt_brim_tdd.rs` (+1 start-corner test with tie arm)
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/serialize.rs`; `crates/slicer-gcode/src/emit.rs` (emitter must stay untouched — authoring-verified start-point preservation is what makes the wiring observable)
  - `OrcaSlicerDocumented/` (delegate)
- Blast-radius discipline: the rotation happens inside `generate_skirt_entities`/`make_rect_loop` on the final point list; `make_rect_loop`'s point-list construction is the only literal-constructing site — no other producer constructs skirt rings. `process()` gets the same corner gate (shared generator).
- Expected sub-agent dispatches:
  - Question: does any other crate or module produce or consume skirt loop point order (`ExtrusionRole::Skirt` ring start) in a way that would observe or undo the rotation?; scope: `crates/`, `modules/`; return: `FACT`; purpose: pin AC-4's observability chain.
- Context cost: `M`
- Authoritative docs:
  - `docs/08_coordinate_system.md` - delegated SUMMARY; confirm degrees (not the 100 nm unit hazard) are the only unit here and are converted to radians locally.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` `Skirt::find_start_point` - delegate (the angle formula this wiring mirrors).
- Verification:
  - `cargo test -p skirt-brim --test skirt_brim_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail (AC-4, AC-N1)
  - `cargo xtask build-guests --check; echo "exit=$?"` - FACT exit 0
- Exit condition: AC-4 command-green; guests fresh.

### Step 4: Integration arms — bounds/enum + CONFIG_BLOCK

- Task IDs: none (wayfinder ticket 13)
- Objective: add the scheduler enum/bounds rejection tests (AC-5: `skirt_type = "outer_only"` → enum `TypeMismatch`; `skirt_start_angle = 200.0` → `OutOfRange`) against the real manifest, and the runtime CONFIG_BLOCK tests (AC-6: explicit `skirt_type = "perobject"` → exactly one `; skirt_type = perobject`; explicit `min_skirt_length = 5.0` → exactly one line; defaults → none of the five lines present).
- Precondition: Steps 1–3 exit met.
- Postcondition: AC-5 and AC-6 pass against the real manifests and the real pipeline driver.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` - full (460-ish lines — bounded read)
  - `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` - lines `1-120` (setup) + grep for an existing CONFIG_BLOCK assertion to mirror (do not read all 1040 lines)
- Files allowed to edit (at most 3):
  - `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs`
  - `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/serialize.rs` (AC-6 pins padding untouched)
  - `docs/15_config_keys_reference.md` (generated; Step 5 only)
- Blast-radius discipline: test-only additions to existing binaries; no production surface. If the runtime binary's driver needs a config key it asserts on, use the same per-test config mechanism its current tests use (verified present at authoring: per-test config injection exists in `gcode_header_thumbnail_config_blocks_tdd.rs`).
- Expected sub-agent dispatches: none — the tests are the direct verification.
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md` §CONFIG_BLOCK - delegated SUMMARY (padding rule), already applied.
- OrcaSlicer refs: none — integration arms are port-side only.
- Verification:
  - `cargo test -p slicer-scheduler --test integration config_bounds_enforcement_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail (AC-5)
  - `cargo test -p slicer-runtime --test integration gcode_header_thumbnail_config_blocks_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail (AC-6)
- Exit condition: AC-5 + AC-6 green.

### Step 5: Regenerate docs + close gates

- Task IDs: none (wayfinder ticket 13)
- Objective: run `cargo xtask gen-config-docs` to regenerate `docs/15_config_keys_reference.md` with the five keys; verify `--check` passes and the keys appear (AC-7); then the packet completion gate.
- Precondition: Steps 1–4 exit met.
- Postcondition: AC-7 exit 0 and keys present; all AC commands green.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/15_config_keys_reference.md` - NEVER load in full; verify via `rg` only (key-presence; the doc has no per-module subheadings — rows carry the owner column, so `rg -n 'single_loop_draft_shield' docs/15_config_keys_reference.md` is the AC-7 check)
- Files allowed to edit (at only via the generator):
  - `docs/15_config_keys_reference.md` - through `cargo xtask gen-config-docs` only
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/serialize.rs` (padding)
  - any hand-edit of the generated doc
- Blast-radius discipline: none (generated doc).
- Expected sub-agent dispatches:
  - Question: does `gen-config-docs --check` pass and do the five keys appear in the generated module-key table (owner column `skirt-brim`)?; scope: `docs/15_config_keys_reference.md`; return: `FACT`; purpose: AC-7.
- Context cost: `S`
- Authoritative docs:
  - `docs/15_config_keys_reference.md` - generated; key-presence rg-verified.
- OrcaSlicer refs: none.
- Verification:
  - `cargo xtask gen-config-docs --check && rg -q 'single_loop_draft_shield' docs/15_config_keys_reference.md; echo "exit=$?"` - FACT
  - `cargo check --workspace --all-targets` - FACT pass/fail
  - `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail
- Exit condition: AC-7 green; workspace gates green.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | manifest + guard test only |
| Step 2 | M | two gates, dual-path, blast-radius confirmed by dispatch |
| Step 3 | M | rotation helper + wrap/tie arms |
| Step 4 | S/M | two test-file additions to existing binaries |
| Step 5 | S | generator + gates |

Split before activation if aggregate cost exceeds M or any step is L.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read — **re-derive the crosswalk question at completion time**, not from this frozen note (ledger-fact rule). The feature-gap queue's packets carry no TASK row (survey precedent at 234a/253/254/255/256/257 authoring time); implementation is recorded against wayfinder ticket 13.
- Reconcile reopened/superseded status transitions.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.