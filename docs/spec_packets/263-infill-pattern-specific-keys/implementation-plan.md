# Implementation Plan: infill-pattern-specific-keys

## Execution Rules

- Steps are ordered and atomic. Do not start a step until the previous step's exit condition is met.
- Every OrcaSlicer read is delegated (`requirements.md` §OrcaSlicer Reference Obligations). Every cargo/xtask run is delegated with a `FACT pass/fail` return.
- Test output always tees to `target/test-output.log`; inspect the log rather than re-running (`CLAUDE.md` §Test output).
- Files listed as out-of-bounds in `design.md` must not be opened for editing under any circumstance, including "just to make it compile".
- No step may add a WIT interface, bump an IR schema version, or add a `ResolvedConfig` field. A step that appears to need one stops and reports a `[BLOCK]`.

## Steps

### Step 1: Scaffold the three pattern-module crates and put them in the workspace

- Task IDs: none (see `task-map.md`).
- Objective: three loadable, buildable, claim-correct core modules that emit nothing yet.
- Preconditions: clean tree; `cargo check --workspace --all-targets` green.
- Allowed reads: `modules/core-modules/gyroid-infill/**`, `modules/core-modules/lightning-infill/**` (crate layout, manifest, guest wrapper), `crates/slicer-sdk/src/{traits.rs,views.rs,builders.rs}`.
- Files created — three parallel trees, one per module `M` in {`lateral-lattice-infill`, `lateral-honeycomb-infill`, `locked-zag-infill`}: `modules/core-modules/M/Cargo.toml`, `modules/core-modules/M/src/lib.rs` (struct + `#[slicer_module] impl LayerModule` whose `run_infill` returns `Ok(())` without pushing a path), `modules/core-modules/M/M.toml` (module/stage/ir-access/claims/compatibility + only the four shared base `[config.schema]` tables `sparse_infill_density`, `infill_direction`, `line_width`, `sparse_infill_speed`), `modules/core-modules/M/wit-guest/Cargo.toml`, `modules/core-modules/M/wit-guest/src/lib.rs`.
- Files edited (2): root `Cargo.toml` (three `members` entries), `crates/pnp-cli/Cargo.toml` (three `integrated-<name>` passthrough features).
- Out of bounds: everything in `design.md` §Out-of-Bounds Files; the pattern-specific `[config.schema]` tables (they land with their algorithm in Steps 3–5).
- Dispatches: none.
- Cost: M.
- Authorities: `docs/03_wit_and_manifest.md` (manifest shape), `docs/adr/0056-integrated-modules-native-dispatch.md`.
- Verification: `cargo check --workspace --all-targets` (delegated, FACT pass/fail).
- Exit / falsifying condition: fails if `cargo check --workspace --all-targets` is not green, or if any new manifest declares a claim other than `claim:sparse-fill`.

### Step 2: Register the three modules in the integrated registry and move the module count

- Objective: the three modules are discovered by manifest ingestion and dispatchable natively (AC-1).
- Preconditions: Step 1 exit met.
- Allowed reads: `crates/slicer-integrated-modules/src/lib.rs` (the `manifest_const!` / `integrated_registry!` blocks and the `#[cfg(not(feature = …))]` arms), `crates/slicer-scheduler/tests/integration/manifest_ingestion_tdd.rs`.
- Files edited (3): `crates/slicer-integrated-modules/Cargo.toml` (three optional path deps + three features), `crates/slicer-integrated-modules/src/lib.rs` (three `manifest_const!` + three registry rows + the matching cfg arms), `crates/slicer-scheduler/tests/integration/manifest_ingestion_tdd.rs` (count assertion and its explanatory comment).
- Out of bounds: as `design.md`.
- Dispatches: `FACT` — the count the test asserts *today*, read from the test file at this moment; the new value is that value + 3. Do not carry a number in from any packet document.
- Cost: S.
- Authorities: `docs/adr/0056-integrated-modules-native-dispatch.md`.
- Verification: `cargo test -p slicer-scheduler --test scheduler_integration manifest_ingestion 2>&1 | tee target/test-output.log | grep -E "^test result"` (AC-1) and `cargo check --workspace --all-targets`.
- Exit / falsifying condition: fails if ingestion reports any `Error`-level diagnostic, if the count does not land at old + 3, or if `cargo xtask dist --edition integrated` would reject a missing passthrough feature (spot-check the `integrated-` feature names against the module directory names).

### Step 3: Implement `lateral-lattice-infill`

- Objective: `lateral_lattice_angle_1` / `lateral_lattice_angle_2` drive real geometry (AC-3, AC-4).
- Preconditions: Step 2 exit met.
- Allowed reads: `modules/core-modules/rectilinear-infill/src/lib.rs` (scan-line emission and the `sparse_infill_area` partition contract), `crates/slicer-sdk/src/{host.rs,views.rs,builders.rs}`.
- Files edited (3): `modules/core-modules/lateral-lattice-infill/src/lib.rs`, `modules/core-modules/lateral-lattice-infill/lateral-lattice-infill.toml` (add the two `[config.schema]` tables per AC-13), `modules/core-modules/lateral-lattice-infill/tests/lateral_lattice_infill_tdd.rs` (new).
- Out of bounds: the other two module trees; everything in `design.md` §Out-of-Bounds Files.
- Dispatches: `SUMMARY` (≤ 200 words) — canonical `FillLateralLattice::fill_surface`: exact `dx1`/`dx2` shift arithmetic, the fixed π/2 line angle, `_layer_angle` returning 0, the odd-`layer_id` polyline reversal, and what `fill_surface_by_multilines` does in the single-line case.
- Cost: M.
- Authorities: `docs/08_coordinate_system.md` (mm → units at the shift boundary).
- Verification: `cargo test -p lateral-lattice-infill --test lateral_lattice_infill_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`.
- Exit / falsifying condition: fails if the family-1 shift at `lateral_lattice_angle_1 = -45`, `z = 2.0` is not exactly `slicer_ir::mm_to_units(-2.0)` units from the `0` baseline, if family 2 moves when only family 1's angle changes, or if the odd-layer reversal is absent.

### Step 4: Implement `lateral-honeycomb-infill`

- Objective: `infill_overhang_angle` drives the honeycomb vertical period (AC-5).
- Preconditions: Step 3 exit met.
- Allowed reads: `modules/core-modules/lateral-lattice-infill/src/lib.rs` (the scan-line helper just written), `crates/slicer-sdk/src/{host.rs,views.rs,builders.rs}`.
- Files edited (3): `modules/core-modules/lateral-honeycomb-infill/src/lib.rs`, `modules/core-modules/lateral-honeycomb-infill/lateral-honeycomb-infill.toml` (the `infill_overhang_angle` table), `modules/core-modules/lateral-honeycomb-infill/tests/lateral_honeycomb_infill_tdd.rs` (new).
- Out of bounds: the other two module trees; `design.md` §Out-of-Bounds Files.
- Dispatches: `SUMMARY` (≤ 200 words) — canonical `FillLateralHoneycomb::fill_surface`: `half_horizontal_period` derivation, `vertical_period = 3·half_horizontal_period / tan(infill_overhang_angle)`, the one-third double-line / two-thirds single-line split, the linear `horizontal_position` interpolation across the double-line third, the per-case density rescale, the alternate-period half stagger, and the odd-layer reversal.
- Cost: M.
- Verification: `cargo test -p lateral-honeycomb-infill --test lateral_honeycomb_infill_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`.
- Exit / falsifying condition: fails if the double-line band count at `infill_overhang_angle = 30` is not strictly greater than at `75`, if their ratio is outside 10 % of `tan(75°)/tan(30°)`, or if the count at the default `60` does not fall strictly between them.

### Step 5: Implement `locked-zag-infill` region algebra (depth, lock, densities)

- Objective: `skin_infill_depth`, `infill_lock_depth`, `skin_infill_density`, `skeleton_infill_density` drive real geometry (AC-6 … AC-9).
- Preconditions: Step 4 exit met.
- Allowed reads: `modules/core-modules/lateral-lattice-infill/src/lib.rs`, `crates/slicer-sdk/src/host.rs` (`offset_polygons`, `clip_polygons`), `crates/slicer-ir/src/slice_ir.rs` (`ExtrusionPath3D`, `Point3WithWidth`).
- Files edited (3): `modules/core-modules/locked-zag-infill/src/lib.rs`, `modules/core-modules/locked-zag-infill/locked-zag-infill.toml` (all seven `[config.schema]` tables per AC-13), `modules/core-modules/locked-zag-infill/tests/locked_zag_infill_tdd.rs` (new).
- Out of bounds: the other two module trees; `design.md` §Out-of-Bounds Files.
- Dispatches: `SUMMARY` (≤ 200 words) — canonical `FillLockedZag::fill_surface_locked_zag`: the `zig_expas = offset_ex(surface, -skin_infill_depth)` core, the `cross_expas = diff_ex(surface, zig_expas)` skin band, the `infill_lock_depth` dilation and re-clip, and which density applies to which zone.
- Cost: M.
- Verification: `cargo test -p locked-zag-infill --test locked_zag_infill_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`.
- Exit / falsifying condition: fails if the skeleton-path bounding box is not 16.0 mm at (`skin_infill_depth = 2.0`, `infill_lock_depth = 0.0`), 18.0 mm at lock `1.0`, and 12.0 mm at depth `4.0` (± 0.05 mm); or if doubling either density does not double that zone's path count (± 1) while leaving the other zone's count unchanged.

### Step 6: `locked-zag-infill` dual width and the symmetric-Y mirror

- Objective: `skin_infill_line_width`, `skeleton_infill_line_width`, `symmetric_infill_y_axis` drive real geometry (AC-10, AC-11, AC-12).
- Preconditions: Step 5 exit met.
- Allowed reads: `crates/slicer-sdk/src/host.rs` (`object_bounds`), `crates/slicer-ir/src/slice_ir.rs`.
- Files edited (2): `modules/core-modules/locked-zag-infill/src/lib.rs`, `modules/core-modules/locked-zag-infill/tests/locked_zag_infill_tdd.rs`.
- Out of bounds: `design.md` §Out-of-Bounds Files; the manifest (its tables were finalised in Step 5).
- Dispatches: `SUMMARY` (≤ 200 words) — canonical `Layer::make_fills`' `symmetric_infill_y_axis` activation and mirror axis (`extended_object_bounding_box().center().x()`), `MultiPoint::symmetric_y` arithmetic, and where `FillRectilinear::fill_surface_by_lines` mirrors the output polylines back.
- Cost: M.
- Verification: `cargo test -p locked-zag-infill --test locked_zag_infill_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`.
- Exit / falsifying condition: fails if a non-zero width key does not set every affected path's `Point3WithWidth.width` to `mm_to_units(value)` while leaving the other zone untouched; or if the `symmetric_infill_y_axis = true` output over the L-shaped fixture is not the `x -> 2·cx - x` mirror of the `false` run over the x-mirrored region.

### Step 7: Manifest schema guard (ownership is the pattern gate)

- Objective: AC-13 and AC-N1.
- Preconditions: Step 6 exit met.
- Allowed reads: the three new manifests; `modules/core-modules/{rectilinear,gyroid,lightning}-infill/*.toml` (read-only).
- Files edited (2): `modules/core-modules/locked-zag-infill/tests/infill_pattern_specific_config_schema_tdd.rs` (new), `modules/core-modules/locked-zag-infill/Cargo.toml` (add `toml = "0.8"` dev-dependency if absent).
- Out of bounds: every existing fill module's manifest — the guard reads them, never edits them.
- Dispatches: none.
- Cost: S.
- Verification: `cargo test -p locked-zag-infill --test infill_pattern_specific_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`.
- Exit / falsifying condition: fails if any of the 10 tables deviates from AC-13's exact type/default/min/max/display/group, if a `description` does not name the canonical consumer function, or if any of the 10 keys appears in `rectilinear-infill.toml`, `gyroid-infill.toml`, or `lightning-infill.toml`.

### Step 8: Scheduler bounds and type rejection

- Objective: AC-14.
- Preconditions: Step 7 exit met.
- Allowed reads: `crates/slicer-scheduler/src/config_resolution.rs` (the `check_value` / `apply_cli_key` contract).
- Files edited (1): `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs`.
- Out of bounds: `crates/slicer-scheduler/src/**` — no production change is needed; the bounds come from the manifests.
- Dispatches: none.
- Cost: S.
- Verification: `cargo test -p slicer-scheduler --test scheduler_integration config_bounds_enforcement 2>&1 | tee target/test-output.log | grep -E "^test result"`.
- Exit / falsifying condition: fails if any of the six out-of-range values resolves, if `symmetric_infill_y_axis = "abc"` is not a `TypeMismatch`, or if `symmetric_infill_y_axis = true` is rejected.

### Step 9: Claim resolution and role scope

- Objective: AC-2 and AC-N3.
- Preconditions: Step 8 exit met.
- Allowed reads: `crates/slicer-runtime/tests/contract/native_infill_claim_resolution_tdd.rs`, `docs/04_host_scheduler.md` §Claim Resolution (delegated SUMMARY if needed).
- Files edited (1): `crates/slicer-runtime/tests/contract/native_infill_claim_resolution_tdd.rs`. If the arms require the module crates as dev-dependencies, `crates/slicer-runtime/Cargo.toml` joins this step's edit list (second edit) — that is the only permitted addition.
- Out of bounds: `crates/slicer-runtime/src/**`.
- Dispatches: none.
- Cost: M.
- Verification: `cargo test -p slicer-runtime --test contract native_infill_claim_resolution 2>&1 | tee target/test-output.log | grep -E "^test result"`.
- Exit / falsifying condition: fails if `sparse_fill_holder` pointing at a new module does not transfer `claim:sparse-fill` away from `rectilinear-infill`, if the top/bottom/bridge holders move, or if a new module emits any non-sparse path.

### Step 10: Guest rebuild, generated docs, and closure gates

- Objective: AC-15, AC-N2, and the packet gates.
- Preconditions: Steps 1–9 exit met.
- Allowed reads: `docs/15_config_keys_reference.md` (generated regions only, ranged reads).
- Files edited: the three new `modules/core-modules/M/M.wasm` artifacts (produced by the tool, not hand-edited) and `docs/15_config_keys_reference.md` (regenerated by the tool, never hand-edited).
- Out of bounds: `crates/slicer-gcode/src/serialize.rs` — AC-N2 asserts a zero-line diff there.
- Dispatches: `FACT` for each command below.
- Cost: S.
- Verification, in order:
  1. capture the pre-edit deviation-row count: `sed -n '/BEGIN GENERATED: orca-deviations/,/END GENERATED: orca-deviations/p' docs/15_config_keys_reference.md | grep -c '^| `'`
  2. `cargo xtask build-guests` then `cargo xtask build-guests --check; echo "exit=$?"` (must print `exit=0`)
  3. `cargo xtask gen-config-docs` then the AC-15 command
  4. re-capture the deviation-row count with the command from (1) and assert it is unchanged
  5. `git diff --unified=0 -- crates/slicer-gcode/src/serialize.rs | grep -E "^[+-]" | grep -cE "infill_lock_depth|infill_overhang_angle|lateral_lattice_angle|skeleton_infill|skin_infill|symmetric_infill_y_axis"` — expect `0` (AC-N2)
  6. `cargo xtask check-literals`
  7. `cargo check --workspace --all-targets` and `cargo clippy --workspace --all-targets -- -D warnings`
- Exit / falsifying condition: fails if `build-guests --check` returns anything but exit 0, if `gen-config-docs --check` is non-zero, if any of the 10 keys is missing from the generated table, if the deviation-row count moved, if the padding grep is non-zero, or if clippy reports any warning.

## Per-Step Budget Roll-Up

| Step | Cost |
| --- | --- |
| 1 scaffold + workspace | M |
| 2 integrated registry + count | S |
| 3 lateral lattice | M |
| 4 lateral honeycomb | M |
| 5 locked-zag region algebra | M |
| 6 locked-zag widths + mirror | M |
| 7 schema guard | S |
| 8 bounds | S |
| 9 claim resolution | M |
| 10 guests + docs + gates | S |

Aggregate: **L**. No single step is L, so the packet does not require a split.

## Packet Completion Gate

- All 15 ACs and the three negative cases pass by their own commands.
- `cargo check --workspace --all-targets` and `cargo clippy --workspace --all-targets -- -D warnings` green.
- `cargo xtask check-literals` exit 0.
- `cargo xtask build-guests --check` exit 0 with all three new guests present.
- Map preflight gates re-checked: (a) the disposition table lists zero declaration-only keys; (b) every key has a non-default-value AC.

## Acceptance Ceremony

Closure requires the whole suite through the gated entry point, dispatched to a sub-agent with a `FACT pass/fail` return (never absorbed into the closing agent's context):

`cargo xtask test --summary --workspace`
