# Implementation Plan: infill-angle-and-multiline-keys

## Execution Rules

- Steps are ordered and atomic. Do not start a step until the previous step's exit condition is met.
- Every OrcaSlicer read is delegated (`requirements.md` §OrcaSlicer Reference Obligations). Every cargo/xtask run is delegated with a `FACT pass/fail` return.
- Test output always tees to `target/test-output.log`; read the log rather than re-running (`CLAUDE.md` §Test output).
- `crates/slicer-gcode/src/serialize.rs` is never opened for editing. No padding twin is added or corrected in this packet.
- No step may add a WIT interface, bump an IR schema version, or add a `ResolvedConfig` field. A step that appears to need one stops and reports a `[BLOCK]`.
- Adding a field to a module's config struct obliges the same step to fix every struct literal of that struct in the module's tests (`..` rest or an `// exhaustive: <reason>` waiver) so `cargo xtask check-literals` stays green.

## Steps

### Step 1: Capture the baseline and declare the seven manifest tables

- Task IDs: none (see `task-map.md`).
- Objective: the four keys exist in the two manifests with canonical types/defaults/bounds and correct ownership, and the pre-packet doc/deviation baseline is recorded.
- Preconditions: clean tree; `cargo check --workspace --all-targets` green.
- Allowed reads: `modules/core-modules/{rectilinear-infill,gyroid-infill,lightning-infill}/*.toml`.
- Files edited (2): `modules/core-modules/rectilinear-infill/rectilinear-infill.toml` (4 tables), `modules/core-modules/gyroid-infill/gyroid-infill.toml` (3 tables).
- Out of bounds: `lightning-infill.toml`; everything in `design.md` §Out-of-Bounds Files.
- Dispatches: `FACT` — the deviation-block data-row count in `docs/15_config_keys_reference.md` right now, via `sed -n '/BEGIN GENERATED: orca-deviations/,/END GENERATED: orca-deviations/p' docs/15_config_keys_reference.md | grep -c '^| `'`. Record it in the step log; Step 6 compares against it. Do not copy a count out of any packet document.
- Cost: S.
- Authorities: `docs/03_wit_and_manifest.md` (`[config.schema]` shape).
- Verification: `cargo check --workspace --all-targets`; `cargo xtask build-guests --check; echo "exit=$?"` is expected to report stale at this point — that is correct, not a failure; rebuild at Step 6.
- Exit / falsifying condition: fails if a table's type/default/min/max deviates from AC-1, if any of the four keys appears in `lightning-infill.toml`, or if `fill_multiline` appears in `gyroid-infill.toml`.

### Step 2: Role-scoped base angle in `rectilinear-infill`

- Objective: `solid_infill_direction` drives the solid-role angle (AC-2).
- Preconditions: Step 1 exit met.
- Allowed reads: `modules/core-modules/rectilinear-infill/src/lib.rs` (ranged: the config struct, the `angle_deg` computation, the per-role emit loop), `crates/slicer-sdk/src/views.rs`.
- Files edited (2): `modules/core-modules/rectilinear-infill/src/lib.rs`, `modules/core-modules/rectilinear-infill/tests/rectilinear_raw_emit_tdd.rs`.
- Out of bounds: `design.md` §Out-of-Bounds Files; gyroid (Step 3).
- Dispatches: `SUMMARY` (≤ 200 words) — canonical `Fill.cpp::Layer::make_fills` / `group_fills`: exactly which surface roles take `solid_infill_direction` and which keep `infill_direction`.
- Cost: M.
- Verification: `cargo test -p rectilinear-infill --test rectilinear_raw_emit_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`.
- Exit / falsifying condition: fails if with `infill_direction = 45`, `solid_infill_direction = 90` the sparse direction is not 45° or any solid path's direction is not 90° (± 0.01°); or if the default run's paths are not byte-identical to the pre-step baseline (AC-N4).

### Step 3: Rotate templates in `rectilinear-infill`, then the same two mechanisms in `gyroid-infill`

- Objective: `sparse_infill_rotate_template` and `solid_infill_rotate_template` cycle the per-layer angle in both modules; `solid_infill_direction` works in gyroid (AC-3, AC-4, AC-5, AC-7).
- Preconditions: Step 2 exit met.
- Allowed reads: `modules/core-modules/gyroid-infill/src/lib.rs` (ranged: config struct, angle read, wave generation entry). Note: gyroid's `run_infill` currently binds `_layer_index` (underscore-prefixed, unused). AC-5 requires gyroid to cycle by layer, so this step must un-prefix it — an expected edit, not a surprise.
- Files edited (3): `modules/core-modules/rectilinear-infill/src/lib.rs` (template resolver + both call sites), `modules/core-modules/gyroid-infill/src/lib.rs`, `modules/core-modules/gyroid-infill/tests/gyroid_infill_tdd.rs`. The rectilinear template assertions are added to `tests/rectilinear_raw_emit_tdd.rs` — if that pushes the step past three edits, split the gyroid half into Step 3b with the same exit condition.
- Out of bounds: `design.md` §Out-of-Bounds Files.
- Dispatches: `SUMMARY` (≤ 200 words) — canonical `Fill.cpp::calculate_infill_rotation_angle`: exact list semantics (cycling index base, whether the base angle is added or replaced) and what the unported metalanguage covers.
- Cost: M.
- Verification: `cargo test -p rectilinear-infill --test rectilinear_raw_emit_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` and `cargo test -p gyroid-infill --test gyroid_infill_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`.
- Exit / falsifying condition: fails if `"0,90"` on the sparse template does not yield 0°/90°/0° for `layer_index` 0/1/2 while the solid angle holds; if the solid template does not do the mirror-image thing; if an unsupported template does not produce exactly one warn plus the base angle (AC-7); or — per `design.md` §Risks — if gyroid's solid orientation cannot be asserted, in which case remove `solid_infill_direction` from `gyroid-infill.toml` and record gyroid as unimplemented for that key rather than leaving it declared and unread.

### Step 4: `fill_multiline` in `rectilinear-infill`

- Objective: `fill_multiline` multiplies sparse scan lines (AC-6).
- Preconditions: Step 3 exit met.
- Allowed reads: `modules/core-modules/rectilinear-infill/src/lib.rs` (ranged: the sparse scan-line emission).
- Files edited (2): `modules/core-modules/rectilinear-infill/src/lib.rs`, `modules/core-modules/rectilinear-infill/tests/rectilinear_raw_emit_tdd.rs`.
- Out of bounds: `design.md` §Out-of-Bounds Files; gyroid and lightning.
- Dispatches: `SUMMARY` (≤ 200 words) — canonical `FillBase.cpp::multiline_fill` and `FillRectilinear.cpp::fill_surface_by_multilines`: the offset-list construction, whether base spacing is multiplied by N, the line-width offset, and the `remove_overlapped` de-overlap.
- Cost: M.
- Verification: `cargo test -p rectilinear-infill --test rectilinear_raw_emit_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`.
- Exit / falsifying condition: fails if `fill_multiline = 3` does not yield exactly 3× the sparse path count of `fill_multiline = 1`, if the group period changes (density drift), or if any solid path count changes.

### Step 5: Manifest guard, bounds, and CONFIG_BLOCK arms

- Objective: AC-1, AC-N1, AC-N2, AC-8, AC-9.
- Preconditions: Step 4 exit met.
- Allowed reads: the three fill manifests; `crates/slicer-scheduler/src/config_resolution.rs` (the bounds contract).
- Files edited (3): `modules/core-modules/rectilinear-infill/tests/infill_angle_multiline_config_schema_tdd.rs` (new standalone binary) plus `modules/core-modules/rectilinear-infill/Cargo.toml` (`toml` dev-dependency, add-if-absent), and `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs`. The CONFIG_BLOCK arm in `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` is Step 5b if the edit cap is reached; both members are already registered in their `main.rs` aggregators, so no registration is needed.
- Out of bounds: `crates/slicer-gcode/src/serialize.rs`; `crates/slicer-scheduler/src/**` (no production change).
- Dispatches: none.
- Cost: S.
- Verification: `cargo test -p rectilinear-infill --test infill_angle_multiline_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`; `cargo test -p slicer-scheduler --test scheduler_integration config_bounds_enforcement 2>&1 | tee target/test-output.log | grep -E "^test result"`; `cargo test -p slicer-runtime --test integration gcode_header_thumbnail_config_blocks 2>&1 | tee target/test-output.log | grep -E "^test result"`.
- Exit / falsifying condition: fails if the guard passes with a drifted table, if any of the four keys is found in `lightning-infill.toml` or `fill_multiline` in `gyroid-infill.toml`, if an out-of-range value resolves, or if `; fill_multiline = 3` does not appear exactly once with an explicit raw-config value.

### Step 6: Guest rebuild, generated docs, and closure gates

- Objective: AC-10, AC-N3, and the packet gates.
- Preconditions: Steps 1–5 exit met.
- Allowed reads: `docs/15_config_keys_reference.md` (generated regions only, ranged).
- Files edited: the two rebuilt `.wasm` guests (tool output) and `docs/15_config_keys_reference.md` (tool output, never hand-edited).
- Out of bounds: `crates/slicer-gcode/src/serialize.rs`.
- Dispatches: `FACT` for each command.
- Cost: S.
- Verification, in order:
  1. `cargo xtask build-guests` then `cargo xtask build-guests --check; echo "exit=$?"` (must print `exit=0`)
  2. `cargo xtask gen-config-docs` then the AC-10 command
  3. re-capture the deviation-row count with Step 1's command and assert it equals Step 1's recorded value
  4. `git diff --unified=0 -- crates/slicer-gcode/src/serialize.rs | grep -cE "^[+-][^+-]"` — expect `0` (AC-N3)
  5. `cargo xtask check-literals`
  6. `cargo check --workspace --all-targets` and `cargo clippy --workspace --all-targets -- -D warnings`
- Exit / falsifying condition: fails if `build-guests --check` returns anything but exit 0, if `gen-config-docs --check` is non-zero, if any of the four keys is missing from the generated table or carries a wrong owner, if the deviation-row count moved, if the padding diff is non-zero, or if clippy or `check-literals` reports anything.

## Per-Step Budget Roll-Up

| Step | Cost |
| --- | --- |
| 1 manifests + baseline | S |
| 2 rectilinear solid angle | M |
| 3 rotate templates (both modules) | M |
| 4 multiline | M |
| 5 guard + bounds + CONFIG_BLOCK | S |
| 6 guests + docs + gates | S |

Aggregate: **M**. No single step is L.

## Packet Completion Gate

- All 10 ACs and the four negative cases pass by their own commands.
- `cargo check --workspace --all-targets` and `cargo clippy --workspace --all-targets -- -D warnings` green.
- `cargo xtask check-literals` exit 0.
- `cargo xtask build-guests --check` exit 0.
- Map preflight gates re-checked: (a) zero declaration-only keys in the disposition table; (b) every key has a non-default-value AC.

## Acceptance Ceremony

Closure requires the whole suite through the gated entry point, dispatched to a sub-agent with a `FACT pass/fail` return (never absorbed into the closing agent's context):

`cargo xtask test --summary --workspace`
