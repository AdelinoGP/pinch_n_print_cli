# Implementation Plan: infill-pattern-specific-keys

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Declare the 10 keys in `rectilinear-infill.toml` + guard test + `toml` dev-dep

- Task IDs: none (queue packet — wayfinder ticket 16)
- Objective: `rectilinear-infill.toml` `[config.schema]` gains the 10 tables with canonical
  type/default/bounds, the canonical-title `display`, `group = "Infill"`, and a `description`
  field per table recording the disposition (AC-1); the net-new guard test pins the 10
  tables and the AC-N2 gyroid/lightning omission; the module's Cargo.toml gains the `toml`
  dev-dep (add-if-absent — verify first; packet 262 may have added it).
- Precondition: none (manifests are the first edit; the guard is TDD-red before the tables exist).
- Postcondition: `cargo test -p rectilinear-infill --test infill_pattern_specific_config_schema_tdd`
  passes; the 10 tables are parseable by `crates/slicer-scheduler/src/manifest.rs`.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/rectilinear-infill/rectilinear-infill.toml` - full (~200 lines; the append point and the `sparse_infill_density`/width table forms to mirror)
  - `modules/core-modules/wipe-tower/wipe-tower.toml` - lines `66-73` (the `bool` table form)
  - `modules/core-modules/part-cooling/tests/cooling_config_schema_tdd.rs` - full (guard pattern)
  - `modules/core-modules/rectilinear-infill/Cargo.toml` - full (dev-dep state)
- Files allowed to edit (at most 3):
  - `modules/core-modules/rectilinear-infill/rectilinear-infill.toml`
  - `modules/core-modules/rectilinear-infill/tests/infill_pattern_specific_config_schema_tdd.rs` (net-new)
  - `modules/core-modules/rectilinear-infill/Cargo.toml`
- Files explicitly out of bounds:
  - `modules/core-modules/rectilinear-infill/src/lib.rs` and all other module sources (read-free pins for the 10 keys — AC-2's no-reads grep is the evidence; never open them for reads)
  - `modules/core-modules/gyroid-infill/gyroid-infill.toml` and `modules/core-modules/lightning-infill/lightning-infill.toml` (omission pins — never edit)
  - `OrcaSlicerDocumented/...` - delegate; never load
- Blast-radius discipline: no struct fields or schema constants are added — manifest
  tables only; the guard test is the only new compile surface.
- Expected sub-agent dispatches:
  - Question: none — the manifest forms are grounded in the read list.
- Context cost: `S`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` - lines `780-810` (the `[config.schema]` example with `enum`/`values`/`description`) and `1530-1545` (the type table)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - delegate; never load (declarations already captured in `requirements.md` §Per-Key Canonical Evidence)
- Verification:
  - `cargo test -p rectilinear-infill --test infill_pattern_specific_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail
- Exit condition: the guard passes with the 10 rectilinear tables pinned exactly (AC-1) and
  the gyroid/lightning omissions pinned (AC-N2).

### Step 2: Inertness arm in the module suite

- Task IDs: none (queue packet — wayfinder ticket 16)
- Objective: `rectilinear_raw_emit_tdd.rs` gains the AC-2 arm: a rectilinear run over the
  square fixture with the 10 keys set explicitly to their canonical defaults emits
  byte-identical `InfillIR` to the same run with the keys absent (the keys are unread at
  any value); the no-reads grep over the four infill module src dirs returns no matches.
- Precondition: Step 1 complete (the keys are declared, so `ConfigView` carries them).
- Postcondition: `cargo test -p rectilinear-infill --test rectilinear_raw_emit_tdd`
  passes with the AC-2 arm green, and the no-reads compound command exits 0 (no matches).
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/rectilinear-infill/tests/rectilinear_raw_emit_tdd.rs` - full (harness + an existing absent-vs-explicit comparison to mirror, e.g. packet 262's AC-2 arm style if landed)
- Files allowed to edit (at most 3):
  - `modules/core-modules/rectilinear-infill/tests/rectilinear_raw_emit_tdd.rs`
- Files explicitly out of bounds:
  - `modules/core-modules/rectilinear-infill/src/lib.rs` and all other module sources (read-free pins)
  - `OrcaSlicerDocumented/...` - delegate; never load
- Blast-radius discipline: no struct fields or schema constants are touched — test arm only.
- Expected sub-agent dispatches:
  - Question: none — the harness and comparison pattern are visible in the read range.
- Context cost: `S`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` - lines `780-810` (how `ConfigView` carries declared keys)
- OrcaSlicer refs:
  - none (inertness is the with-gap contract, not a canonical behaviour)
- Verification:
  - `cargo test -p rectilinear-infill --test rectilinear_raw_emit_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail
  - `rg -n 'infill_lock_depth|infill_overhang_angle|lateral_lattice_angle_1|lateral_lattice_angle_2|skeleton_infill_density|skeleton_infill_line_width|skin_infill_density|skin_infill_depth|skin_infill_line_width|symmetric_infill_y_axis' modules/core-modules/rectilinear-infill/src modules/core-modules/gyroid-infill/src modules/core-modules/lightning-infill/src modules/core-modules/infill-linker/src; [ "$?" = "1" ]; echo "exit=$?"` - FACT exit code (must be 0)
- Exit condition: the AC-2 arm green and the no-reads grep exits 0.

### Step 3: Scheduler bounds/type arms

- Task IDs: none (queue packet — wayfinder ticket 16)
- Objective: `config_bounds_enforcement_tdd.rs` gains the AC-3 arms: `lateral_lattice_angle_1 = -80`
  and `lateral_lattice_angle_2 = 80` and `infill_overhang_angle = 10` and `skin_infill_density = 101`
  and `skin_infill_depth = -1` → each rejected with `OutOfRange`; `symmetric_infill_y_axis = "abc"`
  → `TypeMismatch`; `symmetric_infill_y_axis = true` resolves — all resolved against the real
  `rectilinear-infill.toml` via `load_module_from_paths` + `ConfigBoundsIndex::from_modules` +
  `resolve_global_config`.
- Precondition: Step 1 complete (the manifest declares the keys with bounds).
- Postcondition: `cargo test -p slicer-scheduler --test integration
  config_bounds_enforcement_tdd` passes with the AC-3 arms green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` - full (~460 lines; the `rejects_value_below_min` arm is the pattern)
  - `crates/slicer-scheduler/src/config_resolution.rs` - grep for `OutOfRange` / `TypeMismatch`
- Files allowed to edit (at most 3):
  - `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` (Step 4)
  - `OrcaSlicerDocumented/...` - delegate; never load
- Blast-radius discipline: no struct fields or schema constants are touched — test arms only.
- Expected sub-agent dispatches:
  - Question: does the bounds harness already cover float min/max and bool `TypeMismatch`
    arms (precedent from packet 259/260/261/262's keys), and can AC-3's six arms mirror them
    directly?; scope: `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs`; return: `FACT`; purpose: the arm shapes.
- Context cost: `S`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` - lines `1570-1590` (the bounds index contract)
- OrcaSlicer refs:
  - none (bounds are canonical-declared, already captured)
- Verification:
  - `cargo test -p slicer-scheduler --test integration config_bounds_enforcement_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail
- Exit condition: the AC-3 arms green.

### Step 4: Guest rebuild + CONFIG_BLOCK arm

- Task IDs: none (queue packet — wayfinder ticket 16)
- Objective: the rectilinear guest is rebuilt against the enlarged manifest
  (`cargo xtask build-guests`); the AC-4 arms land in `gcode_header_thumbnail_config_blocks_tdd.rs`
  (defaults: zero `infill_lock_depth` / `infill_overhang_angle` / `lateral_lattice_angle_1` /
  `lateral_lattice_angle_2` / `skeleton_infill_density` / `skeleton_infill_line_width` /
  `skin_infill_density` / `skin_infill_depth` / `skin_infill_line_width` /
  `symmetric_infill_y_axis` lines in the CONFIG_BLOCK — no padding twins; explicit raw-config
  `skin_infill_density = 30.0` → the line `; skin_infill_density = 30` appears exactly once).
- Precondition: Steps 1-3 complete (manifest final; no module-source edits so the rebuild
  is manifest-fingerprint-driven).
- Postcondition: `cargo xtask build-guests --check` returns exit 0; `cargo test -p
  slicer-runtime --test integration gcode_header_thumbnail_config_blocks_tdd` passes
  with the AC-4 arms green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/src/serialize.rs` - lines `490-560` (`ORCA_CONFIG_PADDING` + `emit_config_kv` dedup — read-only context proving the absence pin)
  - `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` - lines `1-120` (setup) + grep for an existing CONFIG_BLOCK assertion to mirror
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/serialize.rs` (read-only — zero padding edits; AC-4 pins honest absence)
  - `modules/core-modules/*/src` (all module sources)
  - `OrcaSlicerDocumented/...` - delegate; never load
- Blast-radius discipline: no struct fields or schema constants are touched — test arms only; the guest rebuild is fingerprint-mandatory, not a code change.
- Expected sub-agent dispatches:
  - Question: does the runtime CONFIG_BLOCK driver thread an explicit raw-config float into
    `serialize_config_block` for a key with NO padding twin, and does the emitted line appear
    exactly once (packet-257 AC-5 precedent)?; scope:
    `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs`
    + `crates/slicer-gcode/src/serialize.rs`; return: `FACT`; purpose: the AC-4 arms.
- Context cost: `S`
- Authoritative docs:
  - `docs/02_ir_schemas.md` - delegated SUMMARY (§CONFIG_BLOCK contract)
- OrcaSlicer refs:
  - none
- Verification:
  - `cargo xtask build-guests --check; echo "exit=$?"` - FACT exit code (must be 0)
  - `cargo test -p slicer-runtime --test integration gcode_header_thumbnail_config_blocks_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail
- Exit condition: guests fresh (exit 0) and the AC-4 arms green.

### Step 5: Docs regeneration + closure gates

- Task IDs: none (queue packet — wayfinder ticket 16)
- Objective: `cargo xtask gen-config-docs` regenerates `docs/15_config_keys_reference.md`
  (10 new module-key rows under the `rectilinear-infill` owner column; deviation block
  unchanged at 26 — re-measured at implementation time per the ledger-fact rule; 26
  measured at 263 authoring); the workspace gates pass.
- Precondition: Steps 1-4 complete (manifest final).
- Postcondition: `cargo xtask gen-config-docs --check` passes; the AC-5 probe passes;
  `cargo check --workspace --all-targets` and `cargo clippy --workspace --all-targets --
  -D warnings` pass; `cargo xtask build-guests --check` returns exit 0.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/15_config_keys_reference.md` - never load in full; verify via `--check` and the AC-5 `rg`/`sed` probe
- Files allowed to edit (at most 3):
  - `docs/15_config_keys_reference.md` (generated — via `cargo xtask gen-config-docs`, never hand-written)
- Files explicitly out of bounds:
  - `docs/ORCA_CONFIG_REFERENCE.md` (ticket 07 ruling — untouched)
  - `OrcaSlicerDocumented/...` - delegate; never load
- Blast-radius discipline: none (generated doc + gates).
- Expected sub-agent dispatches:
  - Question: does `cargo xtask gen-config-docs --check` pass after regeneration, do the
    10 keys appear in the module-key table under the `rectilinear-infill` owner column, and
    does the deviations block still count 26 data rows?; scope: `docs/15_config_keys_reference.md`
    + xtask; return: `FACT`; purpose: AC-5.
- Context cost: `S`
- Authoritative docs:
  - `docs/15_config_keys_reference.md` - generated; delegated reads only
- OrcaSlicer refs:
  - none
- Verification:
  - `cargo xtask gen-config-docs --check && for k in infill_lock_depth infill_overhang_angle lateral_lattice_angle_1 lateral_lattice_angle_2 skeleton_infill_density skeleton_infill_line_width skin_infill_density skin_infill_depth skin_infill_line_width symmetric_infill_y_axis; do rg -q "$k" docs/15_config_keys_reference.md || exit 9; done && [ "$(sed -n '/BEGIN GENERATED: orca-deviations/,/END GENERATED: orca-deviations/p' docs/15_config_keys_reference.md | grep -c '^| \`')" = "26" ]; echo "exit=$?"` - FACT exit code
  - `cargo check --workspace --all-targets` - FACT pass/fail
  - `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail
  - `cargo xtask build-guests --check; echo "exit=$?"` - FACT exit code (must be 0)
- Exit condition: all four gates green.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | manifest + guard + dev-dep |
| Step 2 | S | inertness arm + no-reads grep |
| Step 3 | S | scheduler arms |
| Step 4 | S | guest rebuild + CONFIG_BLOCK arm |
| Step 5 | S | docs + gates |

Aggregate `S` — no step is L; no split required.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read (re-derive the crosswalk question at completion time — queue precedent at 234a/253/254/255/256/257/258/259/260/261/262: no TASK row; implementation is recorded against wayfinder ticket 16).
- Reconcile reopened/superseded status transitions.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
