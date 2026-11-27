# Implementation Plan: 294-flush-into-purge-reuse-wipe-tower

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Declare the three keys with schema guards

- Task IDs: `TASK-000`
- Objective: Declare `flush_into_infill` / `flush_into_objects` / `flush_into_support` in `wipe-tower.toml` with canonical defaults, read all three in `WipeTower::from_config` with matching code defaults and accessors, and pin all three spellings with schema guards; prove the struct-literal blast radius is fully owned.
- Precondition: No `wipe-tower.toml` row and no `WipeTower` field exists for any of the three spellings (verified at authoring: zero-occurrence in `crates/`/`modules/`/`xtask/`, zero `ORCA_CONFIG_PADDING` rows, no prior packet — re-verify with the Step-1 blast-radius dispatch before editing).
- Postcondition: Manifest carries three bool rows (support default `true`, others `false`); `WipeTower::from_config` carries three matching fields + accessors; `flush_into_purge_reuse_tdd::schema_declares_flush_into_keys` (AC-1) passes; `cargo check` is green.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/wipe-tower/src/lib.rs` - `WipeTower` struct + `from_config` bool-row region only
  - `modules/core-modules/wipe-tower/wipe-tower.toml` - `[config.schema.enable_prime_tower]` block + file end only
  - `modules/core-modules/wipe-tower/Cargo.toml` - full (23 lines; `toml` dev-dep check, add-if-absent per packet-260/261 precedent)
- Files allowed to edit (at most 3):
  - `modules/core-modules/wipe-tower/wipe-tower.toml`
  - `modules/core-modules/wipe-tower/src/lib.rs`
  - `modules/core-modules/wipe-tower/Cargo.toml` (`toml` dev-dep add-if-absent for the manifest guard; the schema case itself lands in the new guard file owned by this step's file budget)
  - `modules/core-modules/wipe-tower/tests/flush_into_purge_reuse_tdd.rs` (new guard binary: AC-1 schema case only in this step)
- Files explicitly out of bounds:
  - `modules/core-modules/wipe-tower/src/lib.rs` depth path (`purge_depth_for` / `run_finalization` / `max_purge_depth` — Step 2 owns the subtraction)
  - `crates/slicer-gcode/src/serialize.rs` (no CONFIG_BLOCK change — honest absence, AC-N1)
  - `crates/slicer-scheduler/src/config_resolution.rs` (no per-object threading — DEV-186(a) future)
  - `OrcaSlicerDocumented/` (delegate; never load)
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - When the step adds a field to a struct or a new entry to a schema/version constant, list the **struct-literal blast radius** in "Files allowed to edit" — every test/non-test struct-literal site that compiles against the struct today, plus the test files that hard-assert on the constant's old value. Budget this in the step's context cost; do not let a follow-up "cargo check" discover it.
  - Dispatch a `LOCATIONS` worker for the struct-literal sites before authoring this step; cite the result inline below.
  - Blast-radius dispatch (run first): Question: every `WipeTower { .. }` literal + every test asserting tower defaults (`from_config_defaults`, bed-bounds fixtures) + every `printable_area`-style manifest-count assertion over `wipe-tower.toml`; scope: `modules/ crates/`; return: `LOCATIONS` (≤20 entries). Result: [implementer pastes the ≤20-entry return here and adds any listed file to "Files allowed to edit" above before editing].
- Expected sub-agent dispatches:
  - Question: `WipeTower` struct fields + `from_config` bool-read syntax + accessor shape for one existing bool (`enable_prime_tower`); scope: `modules/core-modules/wipe-tower/src/lib.rs`; return: `SNIPPETS` (≤2 snippets, ≤30 lines each)
  - Question: struct-literal blast radius (above); scope: `modules/ crates/`; return: `LOCATIONS` (≤20 entries)
  - Question: `toml` dev-dep presence in the wipe-tower manifest; scope: `modules/core-modules/wipe-tower/Cargo.toml`; return: `FACT` (≤5 lines)
- Context cost: `S` (split an L step)
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` - manifest `[config.schema]` bool-row shape + `from_declared` whitelist (delegated SUMMARY)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - delegate; never load
- Verification:
  - `cargo test -p wipe-tower --test flush_into_purge_reuse_tdd schema_declares_flush_into_keys 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail or bounded failure SNIPPETS
  - `cargo check --workspace --all-targets 2>&1 | tee target/test-output.log | tail -3` - FACT pass/fail (blast-radius proof)
- Exit condition: AC-1 passes; `cargo check --workspace --all-targets` is green; no other test file changed behaviour (no depth assertion runs yet — AC-2 still unwritten).

### Step 2: Build the wiping-volume helper + role gate + per-toolchange subtraction

- Task IDs: `TASK-000`
- Objective: Compute per-toolchange wiping volume from the layer's own post-anchor new-tool entities through the three-flag role gate, and subtract it per toolchange in the depth path behind the grab-length clamp, with bed-bounds validation following automatically.
- Precondition: Step 1 green (fields + reads landed); `purge_volume_for` / `purge_depth_for` / `max_purge_depth` / `run_finalization` signatures + `LayerCollectionView::ordered_entities` / `PrintEntity.tool_index` / `ToolChange.after_entity_index` spellings located via the Step-2 dispatch before editing.
- Postcondition: `wiping_volume_for` + `counts_for_role` + `entity_volume` exist beside `purge_volume_for`; the depth path subtracts per toolchange floored at zero; AC-2 (subject-gating), AC-3 (infill-only), AC-4 (objects walls+solids), AC-5 (support flip), AC-N2 (bridge exclusion) all have runnable pins (assertions may still be red until Step 3 fills fixtures — the subtraction exists and compiles).
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/wipe-tower/src/lib.rs` - lines covering `purge_volume_for` + `purge_depth_for` + `max_purge_depth` + `run_finalization` insert loop only
  - `crates/slicer-sdk/src/traits.rs` - lines covering `LayerCollectionView::ordered_entities` + `tool_changes` + `FinalizationOutputBuilder::insert_entity_at` only
  - `crates/slicer-ir/src/slice_ir.rs` - lines `2373-2419` only (`ExtrusionRole` variants) + `PrintEntity.tool_index` + `ToolChange.after_entity_index`
- Files allowed to edit (at most 3):
  - `modules/core-modules/wipe-tower/src/lib.rs`
- Files explicitly out of bounds:
  - `modules/core-modules/wipe-tower/src/lib.rs` `generate_purge_paths` body (keeps its signature and un-subtracted prime-length computation — the subtracted depth reaches it through the caller's loop bound, not a rewrite)
  - `modules/core-modules/path-optimization-default/` (ordering untouched — no entity moves, DEV-186(d))
  - `modules/core-modules/wipe-tower/wipe-tower.toml` (schema frozen after Step 1)
  - `crates/slicer-gcode/src/serialize.rs` (no CONFIG_BLOCK change — honest absence)
  - `OrcaSlicerDocumented/` (delegate; never load)
- Expected sub-agent dispatches:
  - Question: `purge_volume_for` / `purge_depth_for` / `max_purge_depth` / `run_finalization` exact signatures + call edges + the `insert_entity_at` anchor contract; scope: `modules/core-modules/wipe-tower/src/lib.rs`; return: `SNIPPETS` (≤3 snippets, ≤30 lines each)
  - Question: `ordered_entities` element type + `tool_index`/`role`/`path` spelling + `after_entity_index` positional contract; scope: `crates/slicer-sdk/src/traits.rs`, `crates/slicer-ir/src/slice_ir.rs`; return: `FACT` (≤5 lines)
- Context cost: `M` (split an L step)
- Authoritative docs:
  - `docs/01_system_architecture.md` - Claim System section, delegated SUMMARY (subtraction is an in-module parameter, not selection)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/ToolOrdering.cpp` - `is_overriddable` + `is_support_overriddable` + `mark_wiping_extrusions` role table and walk shape - delegate; never load
- Verification:
  - `cargo check -p wipe-tower --all-targets 2>&1 | tee target/test-output.log | tail -3` - FACT pass/fail
  - `cargo clippy -p wipe-tower --all-targets -- -D warnings 2>&1 | tee target/test-output.log | tail -3` - FACT pass/fail
- Exit condition: Subtraction compiles clippy-clean; depth is a pure function of (layer entities, toolchange, three flags, purge volume) floored at zero; `generate_purge_paths` signature unchanged; no entity moved; no CONFIG_BLOCK edit; no path-optimization file touched.

### Step 3: Pin behaviour at non-default values (AC-2–AC-N2)

- Task IDs: `TASK-000`
- Objective: Fill the TDD guard with the subject-gating, three flag, flip, and bridge-exclusion cases; prove each key changes behaviour at a non-default value (map Authoring-rule gate (b)); confirm the default tower carries no new values on walls+sparse-only layers and the CONFIG_BLOCK carries no new line (AC-N1 pins the absence — no re-baseline is owed).
- Precondition: Step 2 green (subtraction compiles; helpers callable from tests).
- Postcondition: `cargo test -p wipe-tower --test flush_into_purge_reuse_tdd` is fully green (AC-1–AC-5, AC-N2); each of the three bool keys has at least one non-default behaviour assertion (infill `true` in AC-3, objects `true` in AC-4, support `false` in AC-5 — the support `true` default's subject-gating rides AC-2).
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/wipe-tower/src/lib.rs` - helper signatures + call sites only (no re-read of the whole file)
  - `modules/core-modules/wipe-tower/tests/finalization_live_tdd.rs` - `printable_area_250` + `wipe_tower_inserts_for_layer` helpers only (fixture/counting models, read-only)
- Files allowed to edit (at most 3):
  - `modules/core-modules/wipe-tower/tests/flush_into_purge_reuse_tdd.rs`
  - `modules/core-modules/wipe-tower/tests/wipe_tower_tdd.rs` (only if a default pins — and a break is a packet defect, not churn: walls+sparse-only layers are subject-identical, so any depth diff fails review; the only honest diff class is a support-co-occurring default shrink, and a second file breaking sends the packet back for a split, not a third edit)
- Files explicitly out of bounds:
  - `modules/core-modules/wipe-tower/wipe-tower.toml` (schema frozen after Step 1)
  - Production subtraction logic (frozen after Step 2 — test-only step; a red that needs production change sends the packet back to Step 2)
  - `docs/` (Step 4 owns docs)
- Expected sub-agent dispatches:
  - None (test-only step; no new authority needed — all semantics pinned in Steps 1–2).
- Context cost: `M` (split an L step)
- Authoritative docs:
  - None new (all authority consumed in Steps 1–2)
- OrcaSlicer refs:
  - None (no new canonical read in this step)
- Verification:
  - `cargo test -p wipe-tower --test flush_into_purge_reuse_tdd 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail or bounded failure SNIPPETS
  - `cargo test -p wipe-tower 2>&1 | tee target/test-output.log | tail -3` - FACT pass/fail (no-regression proof — existing purge/matrix/grab/bed-bounds suite green)
- Exit condition: Guard fully green; every bool key has a non-default assertion; walls+sparse-only default depth identical; no production file changed in this step.

### Step 4: Record deviations + annotate the queue ledger

- Task IDs: `TASK-000`
- Objective: File the DEV-186 row (scalar-not-per-object, filament-gating non-borrow, role-table porting, order-untouched) and annotate the 04 tier table + 05 packet list so P65 reads 3-in at packet 294 with the corrected owner; prove doc freshness.
- Precondition: Steps 1–3 green (schema, subtraction, and pins landed).
- Postcondition: `docs/DEVIATION_LOG.md` carries DEV-186 with (a)+(b)+(c)+(d) clauses; 04's three Multimaterial/Flush-options rows point at packet 294 with owner `wipe-tower`; 05's P65 section reads 3 keys in at 294; `cargo xtask check-deviations --check` (if the repo gates it) is green.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/DEVIATION_LOG.md` - DEV-row format + max-ID region only
  - `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` - Multimaterial/Flush options section only
  - `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md` - P65 section only
- Files allowed to edit (at most 3):
  - `docs/DEVIATION_LOG.md`
  - `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md`
  - `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md`
- Files explicitly out of bounds:
  - `modules/` (production frozen — docs-only step)
  - `crates/` (production frozen — docs-only step)
  - `docs/spec_packets/293-speed-other-layers-emitter/` (never edit another packet)
  - `docs/15_config_keys_reference.md` (module-manifest keys do not regen host docs — no touch)
- Expected sub-agent dispatches:
  - Question: DEV-row exact format + current max DEV id at implementation time; scope: `docs/DEVIATION_LOG.md`; return: `FACT` (≤5 lines)
- Context cost: `S` (split an L step)
- Authoritative docs:
  - `docs/DEVIATION_LOG.md` - row format (direct, grep-only)
- OrcaSlicer refs:
  - None (deviation text was grounded in Steps 1–2)
- Verification:
  - `rg -q 'DEV-186' docs/DEVIATION_LOG.md` - FACT pass/fail
  - `cargo test -p wipe-tower --test flush_into_purge_reuse_tdd 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail (final re-dispatch)
- Exit condition: DEV-186 greps; ledger rows read 3-in at 294 with corrected owner; both verification commands pass; no queue-count change; no new fog graduated, nothing ruled out of scope.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Schema + guards; struct-literal + toml-dep dispatches owned here |
| Step 2 | M | Volume helper + gate + subtraction threading |
| Step 3 | M | Behaviour pins + subject-identity proof |
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
