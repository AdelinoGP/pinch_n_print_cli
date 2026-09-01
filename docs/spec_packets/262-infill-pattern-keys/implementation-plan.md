# Implementation Plan: infill-pattern-keys

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Declare the 7 keys in `rectilinear-infill.toml` + guard test + `toml` dev-dep

- Task IDs: none (queue packet — wayfinder ticket 15)
- Objective: `rectilinear-infill.toml` `[config.schema]` gains the 7 tables with canonical
  type/default/bounds/values and a `description` field per table; the net-new guard test
  pins them; the module's Cargo.toml gains the `toml` dev-dep.
- Precondition: none (manifests are the first edit; the guard is TDD-red before the
  tables exist).
- Postcondition: `cargo test -p rectilinear-infill --test infill_config_schema_tdd`
  passes; the 7 tables are parseable by `crates/slicer-scheduler/src/manifest.rs`.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/part-cooling/tests/cooling_config_schema_tdd.rs` - full (guard pattern)
  - `modules/core-modules/seam-planner-default/seam-planner-default.toml` - lines `27-33` (enum form)
  - `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml` - lines `29-41` (string form)
  - `docs/spec_packets/261-raft-keys/packet.spec.md` - lines `26-37` (AC-1/AC-N1 form)
- Files allowed to edit (at most 3):
  - `modules/core-modules/rectilinear-infill/rectilinear-infill.toml`
  - `modules/core-modules/rectilinear-infill/tests/infill_config_schema_tdd.rs` (net-new)
  - `modules/core-modules/rectilinear-infill/Cargo.toml`
- Files explicitly out of bounds:
  - `modules/core-modules/gyroid-infill/` and `modules/core-modules/lightning-infill/` (Step 2)
  - `modules/core-modules/rectilinear-infill/src/lib.rs` (Step 3)
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
  - `cargo test -p rectilinear-infill --test infill_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail
- Exit condition: the guard passes with the 7 rectilinear tables pinned exactly (AC-1's
  rectilinear half).

### Step 2: Declare in `gyroid-infill.toml` + `lightning-infill.toml` + extend the guard

- Task IDs: none (queue packet — wayfinder ticket 15)
- Objective: `gyroid-infill.toml` gains the same 7 tables; `lightning-infill.toml` gains
  the 3 sparse tables; the guard pins both manifests and the AC-N2 lightning omission.
- Precondition: Step 1 complete (the guard file exists).
- Postcondition: `cargo test -p rectilinear-infill --test infill_config_schema_tdd`
  passes with all 17 tables pinned (AC-1 full) and the lightning omission pinned (AC-N2).
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/gyroid-infill/gyroid-infill.toml` - full (existing tables to append after)
  - `modules/core-modules/lightning-infill/lightning-infill.toml` - full (existing tables to append after)
- Files allowed to edit (at most 3):
  - `modules/core-modules/gyroid-infill/gyroid-infill.toml`
  - `modules/core-modules/lightning-infill/lightning-infill.toml`
  - `modules/core-modules/rectilinear-infill/tests/infill_config_schema_tdd.rs`
- Files explicitly out of bounds:
  - `modules/core-modules/rectilinear-infill/rectilinear-infill.toml` (Step 1, final)
  - `modules/core-modules/gyroid-infill/src/lib.rs` and `modules/core-modules/lightning-infill/src/lib.rs` (Steps 3-4)
  - `OrcaSlicerDocumented/...` - delegate; never load
- Blast-radius discipline: no struct fields or schema constants are added — manifest
  tables only.
- Expected sub-agent dispatches:
  - Question: none.
- Context cost: `S`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` - lines `780-810` (the `[config.schema]` example)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - delegate; never load (enum value lists already captured)
- Verification:
  - `cargo test -p rectilinear-infill --test infill_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail
- Exit condition: the guard passes with all 17 tables pinned and the lightning omission
  pinned (AC-1 full, AC-N2).

### Step 3: Rectilinear wiring — solid-role angle, per-layer templates, sparse multiline

- Task IDs: none (queue packet — wayfinder ticket 15)
- Objective: `RectilinearInfill::from_config` reads the four wired keys
  (`solid_infill_direction` fallback 45.0, `sparse_infill_rotate_template` fallback "",
  `solid_infill_rotate_template` fallback "", `fill_multiline` fallback 1); `run_infill`
  computes per-role per-layer angles (sparse: `infill_direction` + sparse template;
  solid: `solid_infill_direction` + solid template; bridge unchanged) and applies
  `fill_multiline` to the sparse scan (base spacing × N, N copies at perpendicular
  offsets of the sparse line width, clipped to the region polygon); the module-local
  `template_angle` and translate helpers land; the AC-2/3/4/5 arms land in
  `rectilinear_raw_emit_tdd.rs`.
- Precondition: Steps 1-2 complete (the keys are declared, so `ConfigView` carries them).
- Postcondition: `cargo test -p rectilinear-infill --test rectilinear_raw_emit_tdd`
  passes with the AC-2/3/4/5 arms green; at canonical defaults the emitted paths are
  byte-identical to pre-packet (AC-2).
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/rectilinear-infill/src/lib.rs` - lines `40-84` (struct fields), `88-196` (`from_config`), `198-260` (`run_infill` head + angle), `280-400` (sparse/top/bottom/bridge scans), `577-640` (`scan_expolygon` signature)
  - `modules/core-modules/rectilinear-infill/tests/rectilinear_raw_emit_tdd.rs` - full (harness + `angle_45_rotated_output_matches_unrotated_after_inverse` / `pattern_shift_interleaves_layers` patterns)
- Files allowed to edit (at most 3):
  - `modules/core-modules/rectilinear-infill/src/lib.rs`
  - `modules/core-modules/rectilinear-infill/tests/rectilinear_raw_emit_tdd.rs`
- Files explicitly out of bounds:
  - `modules/core-modules/gyroid-infill/` (Step 4)
  - `crates/slicer-gcode/src/serialize.rs` (Step 5)
  - `OrcaSlicerDocumented/...` - delegate; never load
- Blast-radius discipline: no struct fields are added to shared types — the new reads
  are module-private fields on `RectilinearInfill` (the struct-literal sites are the
  module's own tests, which the step edits); no schema/version constants are touched.
- Expected sub-agent dispatches:
  - Question: does the rectilinear module (or `slicer_ir` / `slicer_core` polygon_ops)
    already expose a polygon/path translate utility, or must the step add a module-local
    translate helper (gyroid's `rotate_expolygon` is the precedent)?; scope:
    `modules/core-modules/rectilinear-infill/src/lib.rs` + `crates/slicer-core/src/polygon_ops.rs`; return: `FACT`; purpose: the multiline copy implementation.
- Context cost: `M`
- Authoritative docs:
  - `docs/08_coordinate_system.md` - delegated SUMMARY (the multiline spacing math converts via `mm_to_units`)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp` - delegate; never load (`Layer::make_fills` angle branches, `calculate_infill_rotation_angle` list form)
  - `OrcaSlicerDocumented/src/libslic3r/Fill/FillBase.cpp` - delegate; never load (`multiline_fill` offset lists)
  - `OrcaSlicerDocumented/src/libslic3r/Fill/FillRectilinear.cpp` - delegate; never load (`fill_surface_by_multilines` spacing)
- Verification:
  - `cargo test -p rectilinear-infill --test rectilinear_raw_emit_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail
- Exit condition: AC-2/3/4/5 rectilinear arms green; the pre-packet baseline tests
  (`angle_rotation_45`, `pattern_shift_interleaves_layers`, `solid_spacing_adjusted_for_solid_role`) still pass unchanged.

### Step 4: Gyroid wiring — solid-role angle + per-layer templates

- Task IDs: none (queue packet — wayfinder ticket 15)
- Objective: `GyroidInfill::from_config` reads the three angle keys
  (`solid_infill_direction` fallback 45.0, `sparse_infill_rotate_template` fallback "",
  `solid_infill_rotate_template` fallback ""); `fill_expolygon` takes the per-role angle
  (sparse vs solid; bridge unchanged); the module-local `template_angle` helper lands;
  the AC-2/3/4 arms land in `gyroid_infill_tdd.rs`.
- Precondition: Steps 1-2 complete (the keys are declared).
- Postcondition: `cargo test -p gyroid-infill --test gyroid_infill_tdd` passes with the
  AC-2/3/4 arms green; at canonical defaults the emitted paths are byte-identical to
  pre-packet (AC-2).
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/gyroid-infill/src/lib.rs` - lines `88-160` (`from_config`), `330-360` (`fill_expolygon` angle), `670-700` (`rotate_expolygon` — the module-local helper precedent)
  - `modules/core-modules/gyroid-infill/tests/gyroid_infill_tdd.rs` - full (harness + `from_config_defaults` pattern)
- Files allowed to edit (at most 3):
  - `modules/core-modules/gyroid-infill/src/lib.rs`
  - `modules/core-modules/gyroid-infill/tests/gyroid_infill_tdd.rs`
- Files explicitly out of bounds:
  - `modules/core-modules/rectilinear-infill/` (Step 3, final)
  - `crates/slicer-gcode/src/serialize.rs` (Step 5)
  - `OrcaSlicerDocumented/...` - delegate; never load
- Blast-radius discipline: no struct fields are added to shared types — the new reads
  are module-private fields on `GyroidInfill` (the struct-literal sites are the module's
  own tests, which the step edits); no schema/version constants are touched.
- Expected sub-agent dispatches:
  - Question: none — the `fill_expolygon` call sites (sparse/top/bottom/bridge) and their
    role arguments are visible in the read ranges.
- Context cost: `M`
- Authoritative docs:
  - `docs/08_coordinate_system.md` - delegated SUMMARY (angle conversion)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp` - delegate; never load (`Layer::make_fills` angle branches)
- Verification:
  - `cargo test -p gyroid-infill --test gyroid_infill_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail
- Exit condition: AC-2/3/4 gyroid arms green; the pre-packet baseline tests
  (`from_config_defaults`, `wave_pattern_varies_by_layer`) still pass unchanged.

### Step 5: Padding correction + guest rebuild + CONFIG_BLOCK arm

- Task IDs: none (queue packet — wayfinder ticket 15)
- Objective: `ORCA_CONFIG_PADDING`'s `("sparse_infill_pattern", "grid")` becomes
  `"crosshatch"`; the three infill guests are rebuilt (`cargo xtask build-guests`); the
  AC-7 arms land in `gcode_header_thumbnail_config_blocks_tdd.rs` (defaults: the two
  padding lines present, the other five keys absent; explicit `sparse_infill_pattern =
  "gyroid"` appears exactly once with the padding twin suppressed).
- Precondition: Steps 1-4 complete (manifests + module sources final).
- Postcondition: `cargo xtask build-guests --check` returns exit 0; `cargo test -p
  slicer-runtime --test integration gcode_header_thumbnail_config_blocks_tdd` passes
  with the AC-7 arms green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/src/serialize.rs` - lines `486-560` (`ORCA_CONFIG_PADDING` + `emit_config_line` dedup)
  - `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` - lines `1-120` (setup) + grep for an existing CONFIG_BLOCK assertion to mirror
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/src/serialize.rs`
  - `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs`
- Files explicitly out of bounds:
  - `modules/core-modules/*/src` (Steps 3-4, final)
  - `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` (Step 6)
  - `OrcaSlicerDocumented/...` - delegate; never load
- Blast-radius discipline: no struct fields or schema constants are touched; the padding
  value change is a one-line const edit — the CONFIG_BLOCK arm pins the new value and
  the dedup suppression.
- Expected sub-agent dispatches:
  - Question: does the runtime CONFIG_BLOCK driver thread explicit module-declared keys
    into `raw_config` for `serialize_config_block`, and does the emitted-key dedup
    suppress the padding twin for an explicitly-set key?; scope:
    `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs`
    + `crates/slicer-gcode/src/serialize.rs`; return: `FACT`; purpose: the AC-7 arms.
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md` - delegated SUMMARY (§CONFIG_BLOCK contract)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - delegate; never load (the `crosshatch` default, already captured)
- Verification:
  - `cargo xtask build-guests --check; echo "exit=$?"` - FACT exit code (must be 0)
  - `cargo test -p slicer-runtime --test integration gcode_header_thumbnail_config_blocks_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail
- Exit condition: guests fresh (exit 0) and the AC-7 arms green.

### Step 6: Scheduler bounds/enum arms

- Task IDs: none (queue packet — wayfinder ticket 15)
- Objective: `config_bounds_enforcement_tdd.rs` gains the AC-6 arms: `fill_multiline = 0`
  and `11` → `OutOfRange`; `solid_infill_direction = -1` and `361` → `OutOfRange`;
  `fill_multiline = "abc"` → `TypeMismatch`; `sparse_infill_pattern = "bogus"` → unknown
  enum value rejection — all resolved against the real `rectilinear-infill.toml` via
  `load_module_from_paths` + `ConfigBoundsIndex::from_modules` + `resolve_global_config`.
- Precondition: Steps 1-2 complete (the manifest declares the keys with bounds).
- Postcondition: `cargo test -p slicer-scheduler --test integration
  config_bounds_enforcement_tdd` passes with the AC-6 arms green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` - full (~460 lines; the `rejects_value_below_min` / `rejects_unknown_support_style_value` arms)
  - `crates/slicer-scheduler/src/config_resolution.rs` - grep for `OutOfRange` / `TypeMismatch` / enum rejection
- Files allowed to edit (at most 3):
  - `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` (Step 5, final)
  - `OrcaSlicerDocumented/...` - delegate; never load
- Blast-radius discipline: no struct fields or schema constants are touched — test arms
  only.
- Expected sub-agent dispatches:
  - Question: none — the arm pattern is visible in the read range.
- Context cost: `S`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` - lines `1570-1590` (the bounds index contract)
- OrcaSlicer refs:
  - none (bounds are canonical-declared, already captured)
- Verification:
  - `cargo test -p slicer-scheduler --test integration config_bounds_enforcement_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail
- Exit condition: the AC-6 arms green.

### Step 7: Docs regeneration + closure gates

- Task IDs: none (queue packet — wayfinder ticket 15)
- Objective: `cargo xtask gen-config-docs` regenerates `docs/15_config_keys_reference.md`
  (17 new module-key rows; deviation block unchanged at 26); the workspace gates pass.
- Precondition: Steps 1-6 complete (manifests final).
- Postcondition: `cargo xtask gen-config-docs --check` passes; the AC-8 probe passes;
  `cargo check --workspace --all-targets` and `cargo clippy --workspace --all-targets --
  -D warnings` pass; `cargo xtask build-guests --check` returns exit 0.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/15_config_keys_reference.md` - never load in full; verify via `--check` and the AC-8 `rg`/`sed` probe
- Files allowed to edit (at most 3):
  - `docs/15_config_keys_reference.md` (generated — via `cargo xtask gen-config-docs`, never hand-written)
- Files explicitly out of bounds:
  - `docs/ORCA_CONFIG_REFERENCE.md` (ticket 07 ruling — untouched)
  - `OrcaSlicerDocumented/...` - delegate; never load
- Blast-radius discipline: none (generated doc + gates).
- Expected sub-agent dispatches:
  - Question: does `cargo xtask gen-config-docs --check` pass after regeneration, do the
    7 keys appear in the module-key table under the three owner columns, and does the
    deviations block still count 26?; scope: `docs/15_config_keys_reference.md` + xtask;
    return: `FACT`; purpose: AC-8.
- Context cost: `S`
- Authoritative docs:
  - `docs/15_config_keys_reference.md` - generated; delegated reads only
- OrcaSlicer refs:
  - none
- Verification:
  - `cargo xtask gen-config-docs --check && rg -q 'fill_multiline' docs/15_config_keys_reference.md && rg -q 'solid_infill_direction' docs/15_config_keys_reference.md && [ "$(sed -n '/BEGIN GENERATED: orca-deviations/,/END GENERATED: orca-deviations/p' docs/15_config_keys_reference.md | grep -c '^| \`')" = "26" ]; echo "exit=$?"` - FACT exit code
  - `cargo check --workspace --all-targets` - FACT pass/fail
  - `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail
  - `cargo xtask build-guests --check; echo "exit=$?"` - FACT exit code (must be 0)
- Exit condition: all four gates green.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | manifest + guard + dev-dep |
| Step 2 | S | two manifests + guard arms |
| Step 3 | M | rectilinear wiring + behavior arms |
| Step 4 | M | gyroid wiring + behavior arms |
| Step 5 | M | padding + guest rebuild + CONFIG_BLOCK arm |
| Step 6 | S | scheduler arms |
| Step 7 | S | docs + gates |

Aggregate `M` — no step is L; no split required.

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
