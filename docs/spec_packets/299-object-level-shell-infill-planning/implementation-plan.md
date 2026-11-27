# Implementation Plan: 299-object-level-shell-infill-planning

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Declare the four P73 `ResolvedConfig` fields

- Task IDs: `TASK-000`
- Objective: Four `declare_resolved_config!` rows at canonical defaults — `ensure_vertical_shell_thickness` (String, `"ensure_all"`, `extract_string`), `extra_solid_infills` (String, `""`, `extract_string`), `infill_combination` (bool, `false`, `extract_bool`), `infill_combination_max_layer_height` (`ResolvedFloatOrPercent`, `100%`-percent-true, `extract_float_or_percent`) — each with its snake_case `cli` binding; the macro auto-emits `Default`, `apply_cli_key`, `to_config_map`, and overlay arms (ticket 126).
- Precondition: none (fields are additive).
- Postcondition: AC-1's grep passes; `apply_cli_key("ensure_vertical_shell_thickness", String("bogus_mode"))` still accepts (mode validation lives in Step 3, AC-N1 — the field is typed String, the enum gate is the prepass's); the whole-struct `PartialEq` drift guard in the `resolved_config.rs` test module covers the new fields with no edit.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/resolved_config.rs` - lines `1975-2060` (neighbouring rows + `@ {}` attribute-arm example) and `1231-1330` (macro shape)
- Files allowed to edit (at most 3):
  - `crates/slicer-ir/src/resolved_config.rs`
  - `docs/15_config_keys_reference.md` (regen only, via `cargo xtask gen-config-docs`)
- Files explicitly out of bounds:
  - `crates/slicer-scheduler/**`, `crates/slicer-runtime/**` (no consumer yet)
  - `OrcaSlicerDocumented/**`
- Expected sub-agent dispatches:
  - Question: did `cargo test -p slicer-ir` pass after the rows land?; scope: `crates/slicer-ir`; return: `FACT`
- Context cost: `S`
- Authoritative docs:
  - `docs/21_data_defaults_and_fixtures.md` - delegated SUMMARY (struct-literal churn gate does not fire — macro rows, not literals)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - delegate; never load (defaults already verified at authoring: enum `ensure_all`, string `""`, bool false, float-or-percent 100%-true)
- Verification:
  - `rg -q 'cli "ensure_vertical_shell_thickness"' crates/slicer-ir/src/resolved_config.rs && rg -q 'cli "infill_combination_max_layer_height"' crates/slicer-ir/src/resolved_config.rs` - FACT pass
  - `cargo test -p slicer-ir 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail
- Exit condition: AC-1 grep green and slicer-ir tests green with zero edits outside the declared row block.

### Step 2: IR field + view accessor + WIT line

- Task IDs: `TASK-000`
- Objective: `SlicedRegion.combined_infill_height: Option<f32>` (serde-defaulted) in `crates/slicer-ir/src/slice_ir.rs`; `combined-infill-height: func() -> option<f32>` on `slice-region-view` in `crates/slicer-schema/wit/deps/ir-types.wit`; field + `combined_infill_height()` getter on `SliceRegionView` in `crates/slicer-sdk/src/views.rs` mirroring `top_shell_index`.
- Precondition: Step 1 landed (no data dependency — parallel-safe, but ordering keeps the emitter step's inputs complete).
- Postcondition: `cargo check --workspace --all-targets` green; old IR fixtures (with `#[serde(default)]`) deserialize unchanged; the schema-version bump is computed at this step from the live `CURRENT_SLICE_IR_SCHEMA_VERSION` (minor +1) with the test fallout in the same step.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/slice_ir.rs` - lines `1935-2010` (SlicedRegion) and `225-235` (version constant)
  - `crates/slicer-sdk/src/views.rs` - lines `21-145` and `370-420`
  - `crates/slicer-schema/wit/deps/ir-types.wit` - lines `125-170` (slice-region-view)
- Files allowed to edit (at most 3):
  - `crates/slicer-ir/src/slice_ir.rs`
  - `crates/slicer-schema/wit/deps/ir-types.wit`
  - `crates/slicer-sdk/src/views.rs`
- Files explicitly out of bounds:
  - `crates/slicer-wasm-host/test-guests/**` (guest regen rides `build-guests --check`, no source edit)
  - `modules/core-modules/**` (no consumer yet)
- Blast-radius discipline (mandatory when adding a new struct field):
  - The `SlicedRegion` addition is serde-defaulted and additive. Dispatch a `LOCATIONS` worker for non-fixture struct-literal sites of `SlicedRegion` across `crates/ modules/` before authoring this step; cite the result inline: expectation is zero non-fixture sites (the `slice_postprocess_prepass.rs` test helpers at lines 1137-1160 construct the struct and must gain the `None` field or a rest-spread per the doc-21 gate — fixture files are the allowed blast radius). The schema-version bump's test fallout (every test hard-asserting the old minor) is dispatched as a `LOCATIONS` sweep and lands in this step's edit list.
- Expected sub-agent dispatches:
  - Question: list non-fixture `SlicedRegion {` struct-literal sites + tests hard-asserting `CURRENT_SLICE_IR_SCHEMA_VERSION`; scope: `crates/ modules/`; return: `LOCATIONS`
  - Question: did `cargo check --workspace --all-targets` pass?; scope: workspace; return: `FACT`
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md` - delegated SUMMARY (SlicedRegion field + version-bump rules)
  - `docs/03_wit_and_manifest.md` - delegated SUMMARY (additive accessor; bindgen/include_str single source)
- OrcaSlicer refs:
  - None (IR-internal; canonical thickness write-back is Step 4's borrow)
- Verification:
  - `rg -q 'combined_infill_height' crates/slicer-ir/src/slice_ir.rs && rg -q 'combined-infill-height' crates/slicer-schema/wit/deps/ir-types.wit && rg -q 'pub fn combined_infill_height' crates/slicer-sdk/src/views.rs` - FACT pass
  - `cargo check --workspace --all-targets 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail
- Exit condition: check green; version bump + its test fallout landed in this step; guests provably fresh or rebuilt via `cargo xtask build-guests --check` exit code.

### Step 3: Vertical-shell + extra-solid prepass stages (TDD)

- Task IDs: `TASK-000`
- Objective: In `crates/slicer-runtime/src/slice_postprocess_prepass.rs`, add `resolve_vertical_shell_mode` (strict-parse: `none`/`ensure_critical_only`/`ensure_moderate`/`ensure_all`, unknown → `TypeMismatch` naming the key), `discover_vertical_solid_shells` (mode-gated; `ensure_all` projects neighbouring top/bottom shell geometry into `internal_solid_fill` on slope-adjacent layers; `none` inert), and `insert_extra_solid_layers` (port of `check_layer_id_pattern`: 1-based `N`, `N#K` intervals, comma lists; matched layers re-type sparse → `internal_solid_fill`); call both inside `commit_shell_classification_builtin` before `convert_small_sparse_islands`; unit tests `vertical_shell_mode_drives_solid_fill`, `extra_solid_pattern_inserts_solid_layers`, `unknown_shell_mode_rejects`.
- Precondition: Step 1 (fields readable via `region_map.config_for`, the `resolve_shell_counts` pattern).
- Postcondition: AC-2 and AC-3 green; `ensure_vertical_shell_thickness = "none"` reproduces the pre-packet baseline byte-for-byte (pinned); default run matches `"ensure_all"` (intended default change, AC-2).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/slice_postprocess_prepass.rs` - lines `100-230` (commit flow), `1028-1106` (islands + shell counts)
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/src/slice_postprocess_prepass.rs`
- Files explicitly out of bounds:
  - `crates/slicer-runtime/src/region_partition.rs` (the partition reads the buckets; the stage writes them per the existing precedence contract — no partition edit)
  - `OrcaSlicerDocumented/**`
- Expected sub-agent dispatches:
  - Question: exact `discover_vertical_shells` projection bounds, regularization radii (0.65×/1.2×/0.2× spacing family), and anchor-area condition; scope: `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp`; return: `SUMMARY`
  - Question: exact `discover_horizontal_shells` margin branches (factor 0.5/0.2, 3× vs 1× solid-width, search-stop conditions) + `extra_solid_infills` insertion; scope: `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp`; return: `SUMMARY`
  - Question: exact `check_layer_id_pattern` edge semantics; scope: `OrcaSlicerDocumented/src/libslic3r/utils.cpp`; return: `SUMMARY`
- Context cost: `M`
- Authoritative docs:
  - `docs/08_coordinate_system.md` - direct range read (mm↔unit at shell margins)
  - `docs/04_host_scheduler.md` - delegated SUMMARY (prepass ordering)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` - delegate; never load
  - `OrcaSlicerDocumented/src/libslic3r/utils.cpp` - delegate; never load
- Verification:
  - `cargo test -p slicer-runtime --lib vertical_shell_mode_drives_solid_fill 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail
  - `cargo test -p slicer-runtime --lib extra_solid_pattern_inserts_solid_layers 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail
  - `cargo test -p slicer-runtime --lib unknown_shell_mode_rejects 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail
- Exit condition: the three tests green; `cargo test -p slicer-runtime --test integration 2>&1 | tee target/test-output.log | tail -5` green or every red attributed (fixture update with measured justification, map test discipline — never a weakened assertion).

### Step 4: Sparse-infill combination prepass stage (TDD)

- Task IDs: `TASK-000`
- Objective: Add `combine_sparse_infill` to `slice_postprocess_prepass.rs`, running after `convert_small_sparse_islands` and before the bridge gates: gated on `infill_combination` + non-zero density; groups consecutive sparse layers so cumulative height < `min(cap, nozzle_diameter)` (cap from the `ResolvedFloatOrPercent` field resolved against the nozzle diameter via the `ConfigView::get_abs_value` percent-resolution shape, `crates/slicer-ir/src/slice_ir.rs` — the prepass stage resolves the raw field, mirroring how `bridge_density`-class percent keys thread; `0`/`100%` → nozzle diameter); uppermost grouped layer gets the summed `combined_infill_height` + the unioned sparse area, lower grouped layers are voided (`sparse_infill_area` → empty); first layer (index 0) skipped; unit test `infill_combination_groups_sparse_layers`.
- Precondition: Steps 1–2 (fields + IR height field); Step 3 (shell stage's `internal_solid_fill` writes are final before grouping).
- Postcondition: AC-4 green; default run (`infill_combination = false`) leaves every `combined_infill_height` `None` — baseline identical.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/slice_postprocess_prepass.rs` - lines `100-230`, `1028-1106`
  - `crates/slicer-ir/src/resolved_config.rs` - lines `24-60` (ResolvedFloatOrPercent shape) and `crates/slicer-ir/src/slice_ir.rs` - lines `955-985` (ConfigView::get_abs_value percent-resolution)
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/src/slice_postprocess_prepass.rs`
- Files explicitly out of bounds:
  - `crates/slicer-runtime/src/region_partition.rs`
  - `modules/core-modules/**` (emitter arm is Step 5)
- Expected sub-agent dispatches:
  - Question: exact `combine_infill` grouping cap, first-layer skip, pattern-dependent clearance offset, and thickness write-back; scope: `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp`; return: `SUMMARY`
  - Question: did `cargo test -p slicer-runtime --lib infill_combination_groups_sparse_layers` pass?; scope: `crates/slicer-runtime`; return: `FACT`
- Context cost: `M`
- Authoritative docs:
  - `docs/04_host_scheduler.md` - delegated SUMMARY (stage ordering: after islands, before bridge gates)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` - delegate; never load
- Verification:
  - `cargo test -p slicer-runtime --lib infill_combination_groups_sparse_layers 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail
- Exit condition: AC-4 green with the cap-`0.2` arm (fewer combined layers than `100%`) included in the same test.

### Step 5: Sparse-emitter grouped-height arm (TDD)

- Task IDs: `TASK-000`
- Objective: In `modules/core-modules/rectilinear-infill/src/lib.rs`'s `run_infill`, read `combined_infill_height()` from the region view; when `Some(h)`, emit the sparse role's paths at the summed height (extrusion Z / E scaling per the module's existing height handling) while wall roles keep `effective_layer_height` (canonical wall exclusion); module test `combined_height_drives_sparse_only` (two-run comparison: combined vs uncombined fixture differing only in sparse paths, walls identical).
- Precondition: Step 2 (accessor) + Step 4 (height populated).
- Postcondition: AC-5 green; default slices emit nothing new (all heights `None`).
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/rectilinear-infill/src/lib.rs` - lines `198-360`, `600-680`
- Files allowed to edit (at most 3):
  - `modules/core-modules/rectilinear-infill/src/lib.rs`
- Files explicitly out of bounds:
  - `modules/core-modules/gyroid-infill/**`, `modules/core-modules/lightning-infill/**` (their sparse arms adopt the height in their own packets if they choose; this packet pins the rectilinear sparse-holder contract only — recorded in requirements' cross-packet impact)
  - `crates/slicer-sdk/src/views.rs` (read-only consumer)
- Expected sub-agent dispatches:
  - Question: did `cargo test -p rectilinear-infill --lib combined_height_drives_sparse_only` pass?; scope: `modules/core-modules/rectilinear-infill`; return: `FACT`
  - Question: did `cargo xtask build-guests --check` exit 0 after the guest-source edit?; scope: guests; return: `FACT`
- Context cost: `S`
- Authoritative docs:
  - `docs/05_module_sdk.md` - delegated SUMMARY (view accessor consumption)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` - delegate; never load (wall exclusion is the pinned borrow)
- Verification:
  - `cargo test -p rectilinear-infill --lib combined_height_drives_sparse_only 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail
  - `cargo xtask build-guests --check` - FACT exit code 0
- Exit condition: AC-5 green; guest freshness proven by `--check` exit code (never `rg 'STALE:'`).

### Step 6: Locked-path bypass proof + DEV-190 + docs regen

- Task IDs: `TASK-000`
- Objective: Unit test `combination_and_shell_bypass_locked_paths` (AC-N2): a sloping fixture with ADR-0062/0063 order locks on the sparse domain, run with `ensure_all` + combination `true` — locked paths byte-identical to the locks-off-shape run's locked subset. Write `DEV-190` (clauses a–c, design.md Locked Assumptions) into `docs/DEVIATION_LOG.md` after re-deriving `max(DEV-*)` over the log + all drafts; regen `docs/15_config_keys_reference.md`.
- Precondition: Steps 3–5 landed.
- Postcondition: AC-6 + AC-N2 green; disposition table (requirements.md) lists 4/4 wired, zero declaration-only.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/DEVIATION_LOG.md` - ID-convention sample rows only (delegated `rg` for `max(DEV-*)`)
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/src/slice_postprocess_prepass.rs` (the AC-N2 test)
  - `docs/DEVIATION_LOG.md`
  - `docs/15_config_keys_reference.md` (regen only)
- Files explicitly out of bounds:
  - `docs/adr/**` (no ADR this packet — S8 sweep found no normative conflict: ADR-0062/0063 conformance is asserted, not amended)
- Expected sub-agent dispatches:
  - Question: what is `max(DEV-*)` over `docs/DEVIATION_LOG.md` + `docs/spec_packets/*/`?; scope: those paths; return: `FACT`
  - Question: did `cargo xtask gen-config-docs --check` pass?; scope: docs regen; return: `FACT`
- Context cost: `S`
- Authoritative docs:
  - `docs/21_data_defaults_and_fixtures.md` - delegated SUMMARY (any fixture updates from Steps 3–5 churn are re-baselined with measured justification)
- OrcaSlicer refs:
  - None (deviation row + docs regen)
- Verification:
  - `rg -q 'DEV-190' docs/DEVIATION_LOG.md && rg -q 'ensure_vertical_shell_thickness' docs/15_config_keys_reference.md && cargo xtask gen-config-docs --check 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail
  - `cargo test -p slicer-runtime --lib combination_and_shell_bypass_locked_paths 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail
- Exit condition: AC-6 + AC-N2 green; DEV-190 row present; no edits outside the three declared files.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Macro rows only; drift guard auto-covers |
| Step 2 | M | Blast-radius dispatch + version-bump fallout |
| Step 3 | M | Three stages + three tests; largest single-file step |
| Step 4 | M | Grouping stage + cap semantics |
| Step 5 | S | One emitter arm; guest freshness gate |
| Step 6 | S | Deviation + docs; re-derive DEV max |

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