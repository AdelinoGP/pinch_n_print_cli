# Implementation Plan: infill-pattern-holder-mapping

## Execution Rules

- Steps are ordered and atomic. Do not start a step until the previous step's exit condition is met.
- Every OrcaSlicer read is delegated (`requirements.md` §OrcaSlicer Reference Obligations). Every cargo/xtask run is delegated with a `FACT pass/fail` return.
- Test output always tees to `target/test-output.log`; read the log rather than re-running.
- `crates/slicer-gcode/src/serialize.rs` is never opened for editing.
- No step may add a WIT interface, bump an IR schema version, or add a `ResolvedConfig` field. A step that appears to need one stops and reports a `[BLOCK]`.
- A new test file under `crates/slicer-scheduler/tests/integration/` is registered in that directory's `main.rs` **in the same step**; an unregistered file compiles to zero tests and reports a false pass.

## Steps

### Step 1: Scaffold the three module crates and put them in the workspace

- Task IDs: none (see `task-map.md`).
- Objective: three loadable, buildable, claim-correct modules that do nothing yet.
- Preconditions: clean tree; `cargo check --workspace --all-targets` green.
- Allowed reads: `modules/core-modules/gyroid-infill/**` and `modules/core-modules/infill-linker/**` (crate layout, manifest, guest wrapper, post-pass shape).
- Files created — three trees, one per module `M` in {`crosshatch-infill`, `monotonic-infill`, `infill-gap-fill`}: `modules/core-modules/M/{Cargo.toml, src/lib.rs, M.toml, wit-guest/Cargo.toml, wit-guest/src/lib.rs}`. `crosshatch-infill.toml`: `Layer::Infill`, `reads = ["SliceIR"]`, `writes = ["InfillIR"]`, `holds = ["claim:sparse-fill"]`, base tables (`sparse_infill_density`, `infill_direction`, `line_width`, `sparse_infill_speed`). `monotonic-infill.toml`: same stage/IR, `holds = ["claim:top-fill"]`, base tables plus `solid_infill_speed`. `infill-gap-fill.toml`: `Layer::InfillPostProcess`, `reads = ["InfillIR", "PerimeterIR", "RegionMapIR"]`, `writes = ["InfillIR"]`, `holds = ["claim:infill-gap-fill"]`, `requires = []`, and the `gap_fill_target` table (AC-8). Each `run_*` body returns `Ok(())` without emitting.
- Files edited (2): root `Cargo.toml` (three members), `crates/pnp-cli/Cargo.toml` (three `integrated-<name>` passthrough features).
- Out of bounds: `design.md` §Out-of-Bounds Files.
- Dispatches: `FACT` — the deviation-block data-row count in `docs/15_config_keys_reference.md` right now (`sed -n '/BEGIN GENERATED: orca-deviations/,/END GENERATED: orca-deviations/p' docs/15_config_keys_reference.md | grep -c '^| `'`). Record it; Step 8 compares.
- Cost: M.
- Verification: `cargo check --workspace --all-targets`.
- Exit / falsifying condition: fails if `cargo check --workspace --all-targets` is not green, or if any manifest declares a claim other than the one listed above.

### Step 2: Register the three modules and move the module count

- Objective: AC-1.
- Preconditions: Step 1 exit met.
- Allowed reads: `crates/slicer-integrated-modules/src/lib.rs`, `crates/slicer-scheduler/tests/integration/manifest_ingestion_tdd.rs`.
- Files edited (3): `crates/slicer-integrated-modules/Cargo.toml`, `crates/slicer-integrated-modules/src/lib.rs` (three `manifest_const!` + three `integrated_registry!` rows + the `#[cfg(not(feature = …))]` arms), `crates/slicer-scheduler/tests/integration/manifest_ingestion_tdd.rs`.
- Out of bounds: as `design.md`.
- Dispatches: `FACT` — the count the test asserts *today*, read from the test file at this moment; the new value is that + 3. Never carry a number in from a packet document.
- Cost: S.
- Verification: `cargo test -p slicer-scheduler --test scheduler_integration manifest_ingestion 2>&1 | tee target/test-output.log | grep -E "^test result"`.
- Exit / falsifying condition: fails on any `Error`-level diagnostic, if the count does not land at old + 3, or if `infill-gap-fill`'s stage is not `Layer::InfillPostProcess`.

### Step 3: `crosshatch-infill` algorithm

- Objective: AC-4.
- Preconditions: Step 2 exit met.
- Allowed reads: `modules/core-modules/rectilinear-infill/src/lib.rs` (scan-line emission, the `sparse_infill_area` partition contract), `crates/slicer-sdk/src/{views.rs,builders.rs,host.rs}`.
- Files edited (2): `modules/core-modules/crosshatch-infill/src/lib.rs`, `modules/core-modules/crosshatch-infill/tests/crosshatch_infill_tdd.rs` (new standalone binary).
- Out of bounds: the other two new module trees; `design.md` §Out-of-Bounds Files.
- Dispatches: `SUMMARY` (≤ 200 words) — canonical `FillCrossHatch::_fill_surface_single`, `generate_infill_layers`, `generate_repeat_pattern`, `generate_transform_pattern`, `generate_one_cycle`.
- Cost: M.
- Authorities: `docs/08_coordinate_system.md` (the z→grid unit boundary).
- Verification: `cargo test -p crosshatch-infill --test crosshatch_infill_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`.
- Exit / falsifying condition: fails if the direction does not flip once per period across a two-period z sweep, if repeat-band layers are not straight parallel lines at the grid spacing, if transition-band amplitude does not rise to the band midpoint and fall after it, or if two runs at the same z with different `layer_index` differ.

### Step 4: `monotonic-infill` algorithm

- Objective: AC-5.
- Preconditions: Step 3 exit met.
- Allowed reads: `modules/core-modules/rectilinear-infill/src/lib.rs` (the boustrophedon baseline AC-5 compares against).
- Files edited (2): `modules/core-modules/monotonic-infill/src/lib.rs`, `modules/core-modules/monotonic-infill/tests/monotonic_infill_tdd.rs` (new).
- Out of bounds: the other two new module trees; `design.md` §Out-of-Bounds Files.
- Dispatches: `SUMMARY` (≤ 200 words) — canonical `FillMonotonic::fill_surface` and the `params.monotonic` branch of `fill_surface_by_lines`: the observable ordering contract and the `anchor_length_max` distinction between `ipMonotonic` and `ipMonotonicLine`.
- Cost: M.
- Verification: `cargo test -p monotonic-infill --test monotonic_infill_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`.
- Exit / falsifying condition: fails if any successive polyline's sweep coordinate decreases, if the direction-alternation count is non-zero for monotonic, or if the same fixture through `rectilinear-infill` does not show a non-zero alternation count (a zero there would mean the comparison proves nothing).

### Step 5: `infill-gap-fill` pass

- Objective: AC-6, AC-7, AC-8.
- Preconditions: Step 4 exit met.
- Allowed reads: `modules/core-modules/infill-linker/src/lib.rs` (post-pass shape and the prior-IR contract), `crates/slicer-sdk/src/host.rs` (`medial_axis`, `clip_polygons`, `offset_polygons`), `crates/slicer-ir/src/slice_ir.rs` (`ThickPolyline`, `variable_width`, `ExtrusionRole::GapFill`).
- Files edited (3): `modules/core-modules/infill-gap-fill/src/lib.rs`, `modules/core-modules/infill-gap-fill/tests/infill_gap_fill_tdd.rs` (new), `modules/core-modules/infill-gap-fill/tests/infill_gap_fill_config_schema_tdd.rs` (new) — plus `modules/core-modules/infill-gap-fill/Cargo.toml` for the `toml` dev-dependency; if that busts the edit cap, the schema guard and its dev-dependency become Step 5a with the same exit condition (Step 5b below is the linker passthrough).
- Out of bounds: `design.md` §Out-of-Bounds Files; `modules/core-modules/infill-linker/**` is read-only.
- Dispatches: `SUMMARY` (≤ 200 words) — canonical `Fill::_create_gap_fill`: the band formula, the `density >= 1` guard, the per-`GapFillTarget` surface-type check, and the ordering/simplification steps before `medial_axis`.
- Cost: M.
- Authorities: `docs/adr/0028-infill-postprocess-contract-prior-ir-and-partitioned-polygons.md`.
- Verification: `cargo test -p infill-gap-fill --test infill_gap_fill_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` and `cargo test -p infill-gap-fill --test infill_gap_fill_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`.
- Exit / falsifying condition: fails if `"nowhere"` does not re-emit `prior_infill` verbatim (emitting nothing under `"nowhere"` deletes every infill path on the layer — the ADR-0028 Option 1b complete-replacement contract, `run_infill_postprocess` in `crates/slicer-sdk/src/traits.rs`), if `"nowhere"` adds any path, if `"topbottom"` emits on an internal-solid surface, if `"everywhere"` emits no `GapFill` path on the wedge fixture, if any emitted width falls outside the band, or if the linked and unlinked fixtures produce different geometry (AC-7).

### Step 5b: `GapFill` verbatim passthrough in `infill-linker`

- Objective: AC-12 — the linker must not clip or short-filter gap-fill paths, so the two `Layer::InfillPostProcess` modules are order-independent in both directions.
- Preconditions: Step 5 exit met.
- Allowed reads: `modules/core-modules/infill-linker/src/lib.rs` (the `copy_ironing` passthrough this mirrors), `modules/core-modules/infill-linker/src/orchestrate.rs` (`RoleBoundaries::for_role`'s catch-all arm), `modules/core-modules/infill-linker/src/offset.rs` (`remove_short_polylines`), `modules/core-modules/infill-linker/tests/ironing_passthrough_tdd.rs` (the test shape to copy).
- Files edited (2): `modules/core-modules/infill-linker/src/lib.rs`, `modules/core-modules/infill-linker/tests/gap_fill_passthrough_tdd.rs` (new standalone binary).
- Out of bounds: every other file under `modules/core-modules/infill-linker/**` — `src/orchestrate.rs`, `src/offset.rs`, `src/connect.rs`, and `src/graph.rs` are read-only. `claim:infill-link` stays solely the linker's (ADR-0025); this step adds no claim and changes no linking behaviour for any other role.
- Dispatches: `SUMMARY` (≤ 200 words) — canonical: confirm gap fill is emitted by `variable_width(..., erGapFill, ...)` straight into the extrusion collection and is never fed through the infill connect/chain path, so a verbatim passthrough is the parity-correct behaviour rather than a port convenience.
- Cost: S.
- Authorities: `docs/adr/0025-infill-linker-as-raw-emit-post-pass.md`.
- Verification: `cargo test -p infill-linker --test gap_fill_passthrough_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`, then the pre-existing linker suites to prove nothing else moved: `cargo test -p infill-linker 2>&1 | tee target/test-output.log | grep -E "^test result"`.
- Exit / falsifying condition: fails if a `GapFill` path's point count, coordinates, per-vertex widths, `speed_factor`, or ordering changes across the linker; if any non-`GapFill` role's linked output changes; or if any pre-existing `infill-linker` test regresses.

### Step 6: Pattern→holder derivation in `resolve_global_config`

- Objective: AC-2, AC-3, AC-N3.
- Preconditions: Step 5 exit met (the mapping table may only name modules that exist).
- Allowed reads: `crates/slicer-scheduler/src/config_resolution.rs` (ranged, around `resolve_global_config` and `ConfigResolutionError`), `crates/slicer-ir/src/resolved_config.rs` (ranged, the holder fields).
- Files edited (3): `crates/slicer-scheduler/src/config_resolution.rs`, `crates/slicer-scheduler/tests/integration/config_resolution_pattern_holder.rs` (new), `crates/slicer-scheduler/tests/integration/main.rs` (`mod config_resolution_pattern_holder;` — mandatory, see Execution Rules).
- Out of bounds: no new `ResolvedConfig` field in `crates/slicer-ir/src/resolved_config.rs`. Reuse `ConfigResolutionError::TypeMismatch` for the unshipped-value error and leave that file untouched; only if reuse cannot carry the key + value + shipped-list message may this step add a **variant** to `ConfigResolutionError` (it lives in that file, is re-exported by the scheduler, and is not `#[non_exhaustive]`), in which case the step also owns every exhaustive `match` on the enum and its `Display` impl.
- Dispatches: none.
- Cost: M.
- Verification: `cargo test -p slicer-scheduler --test scheduler_integration config_resolution_pattern_holder 2>&1 | tee target/test-output.log | grep -E "^test result"` — and confirm the run reports a non-zero test count, not `0 passed` (the false-pass signature of an unregistered file).
- Exit / falsifying condition: fails if any shipped value does not map to its module, if an explicit holder does not win, if absence changes the default, if an unshipped value is accepted, or if the error message omits the key, the value, or the shipped list.

### Step 7: Bounds, claim-resolution, and hand-maintained docs

- Objective: AC-9, AC-11, AC-N4.
- Preconditions: Step 6 exit met.
- Allowed reads: `docs/04_host_scheduler.md` §Claim Resolution and `docs/03_wit_and_manifest.md` §Known claim IDs (ranged).
- Files edited (4): `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs`, `crates/slicer-runtime/tests/contract/native_infill_claim_resolution_tdd.rs`, `docs/04_host_scheduler.md`, `docs/03_wit_and_manifest.md`. If the edit cap binds, the two doc edits become Step 7b with the AC-11 command as its exit.
- Out of bounds: `crates/slicer-scheduler/src/**` (no further production change), `crates/slicer-gcode/src/serialize.rs`.
- Dispatches: none.
- Cost: S.
- Verification: `cargo test -p slicer-scheduler --test scheduler_integration config_bounds_enforcement 2>&1 | tee target/test-output.log | grep -E "^test result"`; `cargo test -p slicer-runtime --test contract native_infill_claim_resolution 2>&1 | tee target/test-output.log | grep -E "^test result"`; the AC-11 `rg` chain.
- Exit / falsifying condition: fails if `gap_fill_target = "bogus"` resolves, if `monotonic-infill` emits a sparse or bridge path, if `crosshatch-infill` emits a solid or bridge path, or if either doc lacks its required content.

### Step 8: Guest rebuild, generated docs, and closure gates

- Objective: AC-10, AC-N1, AC-N2, and the packet gates.
- Preconditions: Steps 1–7 exit met.
- Allowed reads: `docs/15_config_keys_reference.md` (generated regions only, ranged).
- Files edited: three rebuilt guest `.wasm` artifacts and `docs/15_config_keys_reference.md` — both tool output, never hand-edited.
- Out of bounds: `crates/slicer-gcode/src/serialize.rs`.
- Dispatches: `FACT` for each command.
- Cost: S.
- Verification, in order:
  1. `cargo xtask build-guests` then `cargo xtask build-guests --check; echo "exit=$?"` (must print `exit=0`)
  2. `cargo xtask gen-config-docs` then the AC-10 command
  3. re-capture the deviation-row count with Step 1's command and assert it equals Step 1's recorded value
  4. `cargo test -p slicer-runtime --test e2e slice_end_to_end 2>&1 | tee target/test-output.log | grep -E "^test result"` (AC-N1)
  5. `git diff --unified=0 -- crates/slicer-gcode/src/serialize.rs | grep -cE "^[+-][^+-]"` — expect `0` (AC-N2)
  6. `cargo xtask check-literals`
  7. `cargo check --workspace --all-targets` and `cargo clippy --workspace --all-targets -- -D warnings`
- Exit / falsifying condition: fails if `build-guests --check` returns anything but exit 0, if `gen-config-docs --check` is non-zero, if `gap_fill_target` is absent or duplicated in the generated table, if the deviation-row count moved, if the default-print e2e is not byte-identical, if the padding diff is non-zero, or if clippy or `check-literals` reports anything.

## Per-Step Budget Roll-Up

| Step | Cost |
| --- | --- |
| 1 scaffold + workspace | M |
| 2 registry + count | S |
| 3 crosshatch | M |
| 4 monotonic | M |
| 5 gap-fill pass | M |
| 5b linker `GapFill` passthrough | S |
| 6 pattern→holder derivation | M |
| 7 bounds + claims + docs | S |
| 8 guests + docs + gates | S |

Aggregate: **L**. No single step is L, so the packet does not require a further split.

## Packet Completion Gate

- All 12 ACs and the four negative cases pass by their own commands.
- `cargo check --workspace --all-targets` and `cargo clippy --workspace --all-targets -- -D warnings` green.
- `cargo xtask check-literals` exit 0.
- `cargo xtask build-guests --check` exit 0 with all three new guests present.
- Map preflight gates re-checked: (a) zero declaration-only keys in the disposition table; (b) every key has a non-default-value AC.

## Acceptance Ceremony

Closure requires the whole suite through the gated entry point, dispatched to a sub-agent with a `FACT pass/fail` return (never absorbed into the closing agent's context):

`cargo xtask test --summary --workspace`
