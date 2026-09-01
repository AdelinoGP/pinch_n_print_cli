# Implementation Plan: top-bottom-surface-keys

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Declare the 4 keys in `rectilinear-infill.toml` + guard test + `toml` dev-dep

- Task IDs: none (queue packet — wayfinder ticket 17)
- Objective: `rectilinear-infill.toml` `[config.schema]` gains the 4 tables with canonical
  type/default/bounds/values, the canonical-title `display`, `group = "Infill"`, and a
  `description` field per table recording the disposition (AC-1); the net-new guard test
  pins the 4 tables and the AC-N2 gyroid/lightning omission; the module's Cargo.toml
  gains the `toml` dev-dep (add-if-absent — verify first; packets 262/263 may have added
  it).
- Precondition: none (manifests are the first edit; the guard is TDD-red before the tables exist).
- Postcondition: `cargo test -p rectilinear-infill --test top_bottom_surface_config_schema_tdd`
  passes; the 4 tables are parseable by `crates/slicer-scheduler/src/manifest.rs`.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/rectilinear-infill/rectilinear-infill.toml` - full (~210 lines; the append point and the `sparse_infill_density` float-percent form to mirror)
  - `modules/core-modules/seam-planner-default/seam-planner-default.toml` - the `[config.schema.seam_position]` table (the `enum` + `values` form)
  - `modules/core-modules/part-cooling/tests/cooling_config_schema_tdd.rs` - full (guard pattern)
  - `modules/core-modules/rectilinear-infill/Cargo.toml` - full (dev-dep state)
- Files allowed to edit (at most 3):
  - `modules/core-modules/rectilinear-infill/rectilinear-infill.toml`
  - `modules/core-modules/rectilinear-infill/tests/top_bottom_surface_config_schema_tdd.rs` (net-new)
  - `modules/core-modules/rectilinear-infill/Cargo.toml`
- Files explicitly out of bounds:
  - `modules/core-modules/rectilinear-infill/src/lib.rs` (Step 2 — the wire lands after the declarations)
  - `modules/core-modules/gyroid-infill/gyroid-infill.toml` and `modules/core-modules/lightning-infill/lightning-infill.toml` (omission pins — never edit)
  - `OrcaSlicerDocumented/...` - delegate; never load
- Blast-radius discipline: no struct fields or schema constants are added in this step —
  manifest tables only; the guard test is the only new compile surface.
- Expected sub-agent dispatches:
  - Question: none — the manifest forms are grounded in the read list.
- Context cost: `S`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` - lines `780-810` (the `[config.schema]` example with `enum`/`values`/`description`) and `1530-1545` (the type table)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - delegate; never load (declarations already captured in `requirements.md` §Per-Key Canonical Evidence)
- Verification:
  - `cargo test -p rectilinear-infill --test top_bottom_surface_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail
- Exit condition: the guard passes with the 4 rectilinear tables pinned exactly (AC-1) and
  the gyroid/lightning omissions pinned (AC-N2).

### Step 2: Module wire + identity/reachability/skip arms

- Task IDs: none (queue packet — wayfinder ticket 17)
- Objective: `rectilinear-infill/src/lib.rs` gains the density wire — 2 new
  `RectilinearInfill` fields (`top_surface_density`, `bottom_surface_density`, percent/100
  fractions read in `from_config` via `config.get_abs_value(key, 100.0).map(|d| d as f32
  / 100.0).unwrap_or(1.0)`, the `sparse_infill_density` read pattern); the top solid
  block's spacing switches from `SOLID_DENSITY` to the per-region density
  (`match region.top_shell_index() { Some(0) => self.top_surface_density, _ =>
  SOLID_DENSITY }`) with a `density > 0` gate (canonical `density <= 0` skip); the bottom
  solid block mirrors it with `bottom_surface_density` (gate provably inert under min 10).
  `top_bottom_fill_tdd.rs` gains the AC-2 arm (explicit canonical defaults vs absent →
  byte-identical paths), the AC-3 arm (density 50 → top/bottom solid path count
  approximately halves), and the AC-N3 arm (density 0 → zero `TopSolidInfill` paths for
  the exposed region; an internal-solid region with top_shell_index ≥ 1 still emits).
- Precondition: Step 1 complete (the keys are declared, so `ConfigView` carries them).
- Postcondition: `cargo test -p rectilinear-infill --test top_bottom_fill_tdd` passes
  with the three arms green, and the pattern keys' no-reads compound command exits 0 (no
  matches).
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/rectilinear-infill/src/lib.rs` - the struct (lines `40-90`), `from_config` (lines `87-160`), and the top/bottom solid blocks (lines `310-380`)
  - `modules/core-modules/rectilinear-infill/tests/top_bottom_fill_tdd.rs` - full (~390 lines; the `make_test_region` harness + `has_path_with_role` helpers)
  - `modules/core-modules/rectilinear-infill/tests/rectilinear_infill_tdd.rs` - lines `17-22` (the `ConfigViewBuilder` config form)
- Files allowed to edit (at most 3):
  - `modules/core-modules/rectilinear-infill/src/lib.rs`
  - `modules/core-modules/rectilinear-infill/tests/top_bottom_fill_tdd.rs`
- Files explicitly out of bounds:
  - `modules/core-modules/gyroid-infill/src/lib.rs`, `modules/core-modules/lightning-infill/src/lib.rs`, `modules/core-modules/infill-linker/src/*` (read-free pins for the 4 keys)
  - `OrcaSlicerDocumented/...` - delegate; never load
- Blast-radius discipline: the `RectilinearInfill` struct gains 2 fields — zero
  struct-literal sites exist (all 39 construction sites in the tree use
  `RectilinearInfill::from_config`, verified by grep at authoring), so no test/non-test
  literal fallout; the struct-literal churn gate (docs/21) is not engaged (no test
  literals of the watched struct).
- Expected sub-agent dispatches:
  - Question: none — the harness, config form, and block shapes are visible in the read ranges.
- Context cost: `S`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` - lines `780-810` (how `ConfigView` carries declared keys)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp` - delegate; never load (`group_fills` per-surface-type assignments already captured in `requirements.md` §Per-Key Canonical Evidence)
- Verification:
  - `cargo test -p rectilinear-infill --test top_bottom_fill_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail
  - `rg -n 'top_surface_pattern|bottom_surface_pattern' modules/core-modules/rectilinear-infill/src modules/core-modules/gyroid-infill/src modules/core-modules/lightning-infill/src modules/core-modules/infill-linker/src; [ "$?" = "1" ]; echo "exit=$?"` - FACT exit code (must be 0)
- Exit condition: the three arms green and the no-reads grep exits 0.

### Step 3: Scheduler bounds/type arms

- Task IDs: none (queue packet — wayfinder ticket 17)
- Objective: `config_bounds_enforcement_tdd.rs` gains the AC-4 arms: `top_surface_density = 101`
  and `-1` and `bottom_surface_density = 5` and `101` → each rejected with `OutOfRange`;
  `top_surface_pattern = "bogus"` and `bottom_surface_pattern = "bogus"` → each rejected
  with `TypeMismatch` ("unsupported enum value"); `top_surface_density = 0` and
  `bottom_surface_density = 10` resolve — all resolved against the real
  `rectilinear-infill.toml` via `load_module_from_paths` + `ConfigBoundsIndex::from_modules` +
  `resolve_global_config`.
- Precondition: Step 1 complete (the manifest declares the keys with bounds).
- Postcondition: `cargo test -p slicer-scheduler --test integration
  config_bounds_enforcement_tdd` passes with the AC-4 arms green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` - full (~460 lines; the `rejects_value_below_min` arm is the pattern)
  - `crates/slicer-scheduler/src/config_resolution.rs` - grep for `OutOfRange` / `TypeMismatch` / `unsupported enum value`
- Files allowed to edit (at most 3):
  - `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` (Step 4)
  - `OrcaSlicerDocumented/...` - delegate; never load
- Blast-radius discipline: no struct fields or schema constants are touched — test arms only.
- Expected sub-agent dispatches:
  - Question: does the bounds harness already cover float min/max and enum `TypeMismatch`
    arms (precedent from packet 259/260/261/262/263's keys), and can AC-4's six arms mirror
    them directly?; scope: `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs`; return: `FACT`; purpose: the arm shapes.
- Context cost: `S`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` - lines `1570-1590` (the bounds index contract)
- OrcaSlicer refs:
  - none (bounds are canonical-declared, already captured)
- Verification:
  - `cargo test -p slicer-scheduler --test integration config_bounds_enforcement_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail
- Exit condition: the AC-4 arms green.

### Step 4: Padding correction + guest rebuild + CONFIG_BLOCK arm

- Task IDs: none (queue packet — wayfinder ticket 17)
- Objective: `crates/slicer-gcode/src/serialize.rs`'s `ORCA_CONFIG_PADDING` entry
  `("top_surface_pattern", "monotonic")` is corrected to `("top_surface_pattern",
  "monotonicline")` (canonical default — ticket-14/262 precedent); the rectilinear guest
  is rebuilt against the enlarged manifest and wired module (`cargo xtask build-guests`);
  the AC-5 arms land in `gcode_header_thumbnail_config_blocks_tdd.rs` (defaults: the block
  carries `; top_surface_pattern = monotonicline` and `; bottom_surface_pattern =
  monotonic` and zero `top_surface_density` / `bottom_surface_density` lines; explicit
  raw-config `top_surface_density = 50.0` → `; top_surface_density = 50` exactly once;
  explicit `top_surface_pattern = "concentric"` → `; top_surface_pattern = concentric`
  exactly once, padding twin suppressed by the `emit_config_kv` dedup).
- Precondition: Steps 1-3 complete (manifest final; module wire landed).
- Postcondition: `cargo xtask build-guests --check` returns exit 0; `cargo test -p
  slicer-runtime --test integration gcode_header_thumbnail_config_blocks_tdd` passes
  with the AC-5 arms green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/src/serialize.rs` - lines `455-470` (the padding loop + `emit_config_kv` dedup — read-only context; the only edit is the value at line 505)
  - `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` - lines `1-120` (setup) + grep for an existing CONFIG_BLOCK assertion to mirror
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/src/serialize.rs` (the single padding value)
  - `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/serialize.rs` beyond the single padding value (no twins added or removed; AC-5 pins honest absence for the density keys)
  - `modules/core-modules/*/src` (all module sources — Step 2 is done)
  - `OrcaSlicerDocumented/...` - delegate; never load
- Blast-radius discipline: no struct fields or schema constants are touched — one padding
  value + test arms; the guest rebuild is fingerprint-mandatory, not a code change.
- Expected sub-agent dispatches:
  - Question: does the runtime CONFIG_BLOCK driver thread an explicit raw-config float
    into `serialize_config_block` for a key with NO padding twin, and does an explicit
    enum value suppress its padding twin via the `emit_config_kv` dedup (packet-257 AC-5
    and packet-262 AC-7 precedent)?; scope:
    `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs`
    + `crates/slicer-gcode/src/serialize.rs`; return: `FACT`; purpose: the AC-5 arms.
- Context cost: `S`
- Authoritative docs:
  - `docs/02_ir_schemas.md` - delegated SUMMARY (§CONFIG_BLOCK contract)
- OrcaSlicer refs:
  - none
- Verification:
  - `cargo xtask build-guests --check; echo "exit=$?"` - FACT exit code (must be 0)
  - `cargo test -p slicer-runtime --test integration gcode_header_thumbnail_config_blocks_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail
- Exit condition: guests fresh (exit 0) and the AC-5 arms green.

### Step 5: Docs regeneration + closure gates

- Task IDs: none (queue packet — wayfinder ticket 17)
- Objective: `cargo xtask gen-config-docs` regenerates `docs/15_config_keys_reference.md`
  (4 new module-key rows under the `rectilinear-infill` owner column; deviation block
  unchanged at 26 — re-measured at implementation time per the ledger-fact rule; 26
  measured at 264 authoring); the workspace gates pass.
- Precondition: Steps 1-4 complete (manifest final).
- Postcondition: `cargo xtask gen-config-docs --check` passes; the AC-6 probe passes;
  `cargo check --workspace --all-targets` and `cargo clippy --workspace --all-targets --
  -D warnings` pass; `cargo xtask build-guests --check` returns exit 0.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/15_config_keys_reference.md` - never load in full; verify via `--check` and the AC-6 `rg`/`sed` probe
- Files allowed to edit (at most 3):
  - `docs/15_config_keys_reference.md` (generated — via `cargo xtask gen-config-docs`, never hand-written)
- Files explicitly out of bounds:
  - `docs/ORCA_CONFIG_REFERENCE.md` (ticket 07 ruling — untouched)
  - `OrcaSlicerDocumented/...` - delegate; never load
- Blast-radius discipline: none (generated doc + gates).
- Expected sub-agent dispatches:
  - Question: does `cargo xtask gen-config-docs --check` pass after regeneration, do the
    4 keys appear in the module-key table under the `rectilinear-infill` owner column, and
    does the deviations block still count 26 data rows?; scope: `docs/15_config_keys_reference.md`
    + xtask; return: `FACT`; purpose: AC-6.
- Context cost: `S`
- Authoritative docs:
  - `docs/15_config_keys_reference.md` - generated; delegated reads only
- OrcaSlicer refs:
  - none
- Verification:
  - `cargo xtask gen-config-docs --check && for k in top_surface_density bottom_surface_density top_surface_pattern bottom_surface_pattern; do rg -q "$k" docs/15_config_keys_reference.md || exit 9; done && [ "$(sed -n '/BEGIN GENERATED: orca-deviations/,/END GENERATED: orca-deviations/p' docs/15_config_keys_reference.md | grep -c "^| \`")" = "26" ]; echo "exit=$?"` - FACT exit code
  - `cargo check --workspace --all-targets` - FACT pass/fail
  - `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail
  - `cargo xtask build-guests --check; echo "exit=$?"` - FACT exit code (must be 0)
- Exit condition: all four gates green.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | manifest + guard + dev-dep |
| Step 2 | S | module wire + three test arms + no-reads grep |
| Step 3 | S | scheduler arms |
| Step 4 | S | padding correction + guest rebuild + CONFIG_BLOCK arm |
| Step 5 | S | docs + gates |

Aggregate `S` — no step is L; no split required.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read (re-derive the crosswalk question at completion time — queue precedent at 234a/253/254/255/256/257/258/259/260/261/262/263: no TASK row; implementation is recorded against wayfinder ticket 17).
- Reconcile reopened/superseded status transitions.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
