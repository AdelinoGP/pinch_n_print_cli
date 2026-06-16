# Implementation Plan: 102_perimeter-modules-foundations

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first (write the failing test before the production change), then implementation, then the narrowest falsifying validation.
- Each step honors the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. The fields below are not optional metadata — they are the budget contract for this step.

## Steps

### Step 1: Extract shared helpers to `slicer-helpers::perimeter_utils` and migrate `classic-perimeters`

- Task IDs:
  - `T-010` — Create shared helpers module
  - `T-011` — Migrate `classic-perimeters` to consume it
- Objective: create `crates/slicer-helpers/src/perimeter_utils.rs` exporting the seven helpers + `BASE_SPEED`, delete the local definitions from `classic-perimeters/src/lib.rs`, replace with `use` imports.
- Precondition: workspace builds clean (`cargo check --workspace` green before any edit).
- Postcondition: AC-1 verification command passes (symbols exported); AC-2 partial — `classic-perimeters/src/lib.rs` has no local `fn` defs for the seven helpers; `arachne-perimeters/src/lib.rs` still does (migration in Step 1b).
- Files allowed to read (with line-range hints when > 300 lines):
  - `modules/core-modules/classic-perimeters/src/lib.rs` — full file (≤ 420 lines) — verify exact helper locations.
  - `crates/slicer-helpers/src/lib.rs` — current `pub mod …` declarations.
  - `crates/slicer-helpers/Cargo.toml` — confirm `slicer-ir` dep is already present.
- Files allowed to edit (≤ 3):
  - `crates/slicer-helpers/src/perimeter_utils.rs` (NEW)
  - `crates/slicer-helpers/src/lib.rs`
  - `modules/core-modules/classic-perimeters/src/lib.rs`
- Files explicitly out-of-bounds for this step:
  - `modules/core-modules/arachne-perimeters/src/lib.rs` — Step 1b
  - Any IR / WIT file — Step 2
- Expected sub-agent dispatches:
  - `Run cargo check -p slicer-helpers --all-targets after the helper extraction; return FACT (pass/fail) + SNIPPETS (≤ 20 lines on fail).`
  - `Run cargo test -p classic-perimeters --test boundary_paint_tdd after the classic migration; return FACT (pass/fail).`
- Context cost: `M` (one new file, one migration; ~170 LOC moved)
- Authoritative docs:
  - `docs/specs/perimeter-modules-orca-parity-roadmap.md` — read T-010 + T-011 rows only.
- OrcaSlicer refs:
  - None (helpers are local; no parity check needed for the extraction itself).
- Verification:
  - `cargo test -p classic-perimeters --test boundary_paint_tdd 2>&1 | tee target/test-output.log` — dispatch as FACT pass/fail.
  - `! rg -q '^fn (build_outer_wall_flags|has_adjacent_material_change|find_adjacent_tool|extract_tool_index|default_feature_flags|expolygon_to_path3d|generate_seam_candidates)' modules/core-modules/classic-perimeters/src/lib.rs` — exit code 0 means no local defs remain.
- Exit condition: `classic-perimeters` `boundary_paint_tdd` test passes AND no local helper `fn` defs remain in its `lib.rs` AND `slicer-helpers::perimeter_utils` exports all eight named symbols.

### Step 1b: Migrate `arachne-perimeters` to the shared helpers

- Task IDs:
  - `T-012` — Migrate `arachne-perimeters` to consume shared utils
- Objective: same as Step 1 but for `arachne-perimeters`. Mirror the `use` imports and delete the duplicated local definitions.
- Precondition: Step 1 exit condition met; `cargo check --workspace --all-targets` clean.
- Postcondition: AC-2 fully met (neither module has local helper defs).
- Files allowed to read (with line-range hints when > 300 lines):
  - `modules/core-modules/arachne-perimeters/src/lib.rs` — full file (≤ 670 lines).
  - `modules/core-modules/classic-perimeters/src/lib.rs` — already migrated; reference for the `use`-import shape.
- Files allowed to edit (≤ 3):
  - `modules/core-modules/arachne-perimeters/src/lib.rs`
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-helpers/src/perimeter_utils.rs` — already done; only consume.
  - Any IR / WIT file.
- Expected sub-agent dispatches:
  - `Run cargo test -p arachne-perimeters --test boundary_paint_tdd after the migration; return FACT (pass/fail).`
- Context cost: `S` (one file edit; mirror Step 1's shape)
- Authoritative docs:
  - `docs/specs/perimeter-modules-orca-parity-roadmap.md` — read T-012 row only.
- OrcaSlicer refs:
  - None.
- Verification:
  - `cargo test -p arachne-perimeters --test boundary_paint_tdd 2>&1 | tee target/test-output.log` — FACT pass/fail.
  - `! rg -q '^fn (build_outer_wall_flags|has_adjacent_material_change|find_adjacent_tool|extract_tool_index|default_feature_flags|expolygon_to_path3d|generate_seam_candidates)' modules/core-modules/arachne-perimeters/src/lib.rs` — exit 0 = clean.
- Exit condition: AC-2 verification command passes (both module files clean), `boundary_paint_tdd` green for `arachne-perimeters`.

### Step 2: Widen `WallBoundaryType::MaterialBoundary` (IR + WIT + adapter)

- Task IDs:
  - `T-013` — Widen MaterialBoundary to `Vec<MaterialBoundarySegment>`
  - `T-014` — Update `build_outer_wall_flags` in shared utils to emit full transition list
- Objective: widen the IR + WIT type, bump `CURRENT_SLICE_IR_SCHEMA_VERSION` to `4.2.0`, add `serde` migration adapter for the old single-tool shape, update `build_outer_wall_flags` in `perimeter_utils` to compute and emit all transitions on a multi-tool polygon.
- Precondition: Step 1b exit condition met; `cargo check --workspace --all-targets` clean.
- Postcondition: AC-3 and AC-N2 verification commands pass.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-ir/src/slice_ir.rs` — range-read by `rg -n 'WallBoundaryType|MaterialBoundary|CURRENT_SLICE_IR_SCHEMA_VERSION'`, then `Read` ±40 lines around each hit. Do NOT load the full file.
  - `crates/slicer-schema/wit/deps/ir-types.wit` — full file (≤ 200 lines).
  - `crates/slicer-helpers/src/perimeter_utils.rs` — full file (recently created; small).
- Files allowed to edit (≤ 3):
  - `crates/slicer-ir/src/slice_ir.rs`
  - `crates/slicer-schema/wit/deps/ir-types.wit`
  - `crates/slicer-helpers/src/perimeter_utils.rs`
- Files explicitly out-of-bounds for this step:
  - Both perimeter modules' `lib.rs` files — they consume the change via `perimeter_utils`; no direct edit needed (unless a `match` arm exists, which delegation must confirm).
  - `docs/02_ir_schemas.md` — doc impact handled in Step 5.
- Expected sub-agent dispatches:
  - `Find all match arms or constructors of WallBoundaryType::MaterialBoundary across the workspace; return LOCATIONS (≤ 20 entries).`
  - `Run cargo test -p slicer-ir --test material_boundary_widening_tdd; return FACT (pass/fail) + assertion text on fail.`
  - `Run cargo test -p slicer-helpers --test perimeter_utils_three_tool_boundary_tdd; return FACT (pass/fail).`
  - `Run cargo xtask build-guests --check; return FACT (clean / STALE list ≤ 5 lines).`
- Context cost: `M` (three-crate edit; new tests + migration adapter)
- Authoritative docs:
  - `docs/02_ir_schemas.md` — delegate SUMMARY for `WallBoundaryType`.
  - `docs/03_wit_and_manifest.md` — read §"WIT/Type Changes Checklist" (≈ 30 lines).
  - `CLAUDE.md` — §"WIT/Type Changes Checklist" and §"Guest WASM Staleness".
- OrcaSlicer refs:
  - None for this step (the widening shape is local-design per ADR-0011 / D-13 closure).
- Verification:
  - `cargo test -p slicer-ir --test material_boundary_widening_tdd 2>&1 | tee target/test-output.log` — FACT.
  - `cargo test -p slicer-helpers --test perimeter_utils_three_tool_boundary_tdd 2>&1 | tee target/test-output.log` — FACT.
  - `cargo build --tests --workspace 2>&1 | tee target/test-output.log` — FACT (catches WIT type identity break before runtime).
  - `cargo xtask build-guests --check` — must report no STALE entries after rebuild.
- Exit condition: AC-3 + AC-N2 green, no STALE guests reported, all `MaterialBoundary` call sites adjusted, `CURRENT_SLICE_IR_SCHEMA_VERSION` reads `4.2.0`.

### Step 3: Plumb per-layer config + replace `let _ = output\.` with `?` propagation

- Task IDs:
  - `T-015` — Plumb `LayerOverrides` per-layer config via `_config: &ConfigView`
  - `T-016` — Replace `let _ = output.…` with `?` in both modules
- Objective: read `_config.get*` per-layer in `run_perimeters` so `LayerOverrides` take effect; rewrite every `let _ = output\.` to `?` propagation so capacity/contract errors surface as `ModuleError`.
- Precondition: Step 2 exit condition met; `cargo check --workspace --all-targets` clean.
- Postcondition: AC-4 and AC-5 verification commands pass.
- Files allowed to read (with line-range hints when > 300 lines):
  - `modules/core-modules/classic-perimeters/src/lib.rs` — range-read `run_perimeters` body and `on_print_start`.
  - `modules/core-modules/arachne-perimeters/src/lib.rs` — same.
  - `crates/slicer-sdk/src/views.rs` — read `ConfigView::get_int / get_float` method docs only.
- Files allowed to edit (≤ 3):
  - `modules/core-modules/classic-perimeters/src/lib.rs`
  - `modules/core-modules/arachne-perimeters/src/lib.rs`
  - `crates/slicer-runtime/tests/contract/per_layer_config_override_tdd.rs` (NEW)
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-sdk/src/traits.rs` — the `LayerModule::run_perimeters` signature is unchanged; only the parameter usage changes.
- Expected sub-agent dispatches:
  - `Find all occurrences of "let _ = output\." in modules/core-modules/{classic,arachne}-perimeters/src/lib.rs; return LOCATIONS.`
  - `Run cargo test -p slicer-runtime --test contract per_layer_config_override_tdd; return FACT (pass/fail).`
- Context cost: `M` (two-file rewrite + new test fixture)
- Authoritative docs:
  - `docs/05_module_sdk.md` — delegate SUMMARY for `ConfigView` and `LayerOverrides`.
- OrcaSlicer refs:
  - None.
- Verification:
  - `cargo test -p slicer-runtime --test contract per_layer_config_override_tdd 2>&1 | tee target/test-output.log` — FACT.
  - `! rg -q 'let _ = output\.' modules/core-modules/classic-perimeters/src/lib.rs modules/core-modules/arachne-perimeters/src/lib.rs` — exit 0 = no swallowed Results.
- Exit condition: AC-4 + AC-5 green; no remaining `let _ = output\.` in either module file.

### Step 4: Document `PerimeterOutputBuilder` failure modes + negative-path TDD

- Task IDs:
  - `T-017` — Document failure-mode contract; add negative test
- Objective: add a `## Failure Modes` subsection to `PerimeterOutputBuilder`'s doc-comment (matches the SDK-doc convention); write a negative TDD that constructs a capacity-rejecting mock builder and asserts the perimeter module returns `Err(ModuleError)` rather than swallowing.
- Precondition: Step 3 exit condition met.
- Postcondition: AC-N1 verification command passes.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-sdk/src/builders.rs` — range-read the `PerimeterOutputBuilder` struct and `impl` block.
  - `docs/05_module_sdk.md` — delegate SUMMARY for §"PerimeterOutputBuilder".
- Files allowed to edit (≤ 3):
  - `crates/slicer-sdk/src/builders.rs` — add doc-comment subsection.
  - `docs/05_module_sdk.md` — add `## PerimeterOutputBuilder failure modes` section.
  - `crates/slicer-runtime/tests/contract/perimeter_builder_capacity_error_tdd.rs` (NEW)
- Files explicitly out-of-bounds for this step:
  - Module `lib.rs` files — they were rewritten in Step 3 to propagate `?`; no edit needed here.
- Expected sub-agent dispatches:
  - `Run cargo test -p slicer-runtime --test contract perimeter_builder_capacity_error_tdd; return FACT (pass/fail).`
- Context cost: `S` (doc edit + small negative test)
- Authoritative docs:
  - `docs/05_module_sdk.md` — read §"PerimeterOutputBuilder" via SUMMARY before editing.
- OrcaSlicer refs:
  - None.
- Verification:
  - `cargo test -p slicer-runtime --test contract perimeter_builder_capacity_error_tdd 2>&1 | tee target/test-output.log` — FACT.
  - `rg -q 'PerimeterOutputBuilder failure modes' docs/05_module_sdk.md` — confirms doc section landed.
- Exit condition: AC-N1 green; doc grep passes.

### Step 5: Reconcile manifest defaults + document `_paint` disuse + doc edits

- Task IDs:
  - `T-018` — Reconcile manifest defaults with code fallbacks
  - `T-019` — Read `_paint` or document intentional disuse
  - Doc impact: `docs/02_ir_schemas.md`, `docs/15_config_keys_reference.md`, `docs/05_module_sdk.md` per Doc Impact Statement.
- Objective: align manifest defaults with code fallbacks (manifest wins per `[FWD]` open question default); update the module doc-comments to record `_paint`'s intentional disuse; land the three doc-impact edits.
- Precondition: Step 4 exit condition met.
- Postcondition: AC-6 verification command passes; all four Doc Impact Statement grep checks return hits.
- Files allowed to read (with line-range hints when > 300 lines):
  - `modules/core-modules/classic-perimeters/classic-perimeters.toml` — full.
  - `modules/core-modules/arachne-perimeters/arachne-perimeters.toml` — full.
- Files allowed to edit (≤ 3, per-substep):
  - Sub-step 5a (manifest reconcile): `modules/core-modules/classic-perimeters/classic-perimeters.toml`, `modules/core-modules/arachne-perimeters/arachne-perimeters.toml`, `crates/slicer-runtime/tests/integration/manifest_default_reconcile_tdd.rs` (NEW).
  - Sub-step 5b (`_paint` doc-comment + module doc): `modules/core-modules/classic-perimeters/src/lib.rs`, `modules/core-modules/arachne-perimeters/src/lib.rs`. (Already touched in earlier steps; this is comment-only.)
  - Sub-step 5c (doc edits): `docs/02_ir_schemas.md`, `docs/05_module_sdk.md`, `docs/15_config_keys_reference.md`.
- Files explicitly out-of-bounds for this step:
  - Any new test crate.
- Expected sub-agent dispatches:
  - `Run cargo test -p slicer-runtime --test integration manifest_default_reconcile_tdd; return FACT (pass/fail).`
  - `For each grep in the Doc Impact Statement, run rg -q on the listed path and confirm exit 0; return FACT pass/fail per grep.`
- Context cost: `M` (three doc edits + small new test + comment-only changes; per-sub-step it would be S each, aggregated to M)
- Authoritative docs:
  - `docs/15_config_keys_reference.md` — locate the `wall_count`, `outer_wall_speed`, `inner_wall_speed` rows (delegate LOCATIONS).
  - `docs/02_ir_schemas.md` — section for `WallBoundaryType` (delegate SUMMARY before editing).
- OrcaSlicer refs:
  - None directly; if the manifest defaults need OrcaSlicer-cross-check (`outer_wall_speed = 30.0` matching), delegate a FACT (`grep for outer_wall_speed default in OrcaSlicerDocumented/resources/profiles/`).
- Verification:
  - `cargo test -p slicer-runtime --test integration manifest_default_reconcile_tdd 2>&1 | tee target/test-output.log` — FACT.
  - `rg -q 'MaterialBoundarySegment' docs/02_ir_schemas.md` — confirms IR doc section landed.
  - `rg -q '4\.2\.0.*MaterialBoundary' docs/02_ir_schemas.md` — confirms schema-bump rationale documented.
  - `rg -q 'PerimeterOutputBuilder failure modes' docs/05_module_sdk.md` — confirms (also touched in Step 4) section present.
  - `rg -q 'wall_count.*default: 2' docs/15_config_keys_reference.md` — confirms reconciled default.
- Exit condition: AC-6 green AND all four Doc Impact Statement greps return hits AND module doc-comments explicitly state `_paint`'s intentional disuse with reference to packet 102.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | One new file + one migration; ~170 LOC moved. |
| Step 1b | S | Mirror of Step 1 in arachne-perimeters. |
| Step 2 | M | Three-crate edit (IR + WIT + helpers); new tests + migration adapter; guest-WASM rebuild gate. |
| Step 3 | M | Two-module rewrite for `_config` + `?` propagation; new TDD. |
| Step 4 | S | Doc-comment + negative TDD. |
| Step 5 | M | Manifest reconcile + three doc edits + comment-only changes. |

Aggregate context cost: `M`. No single step is `L`. Per-step file edit count never exceeds 3 (Step 5 splits into sub-steps a/b/c to honor this).

## Packet Completion Gate

- All five steps complete; each step's exit condition met.
- AC-1, AC-2, AC-3, AC-4, AC-5, AC-6, AC-N1, AC-N2 verification commands all return PASS via worker dispatch.
- `cargo check --workspace --all-targets` clean.
- `cargo clippy --workspace --all-targets -- -D warnings` clean.
- `cargo xtask build-guests --check` reports no STALE guests.
- `docs/07_implementation_status.md` updated for each T-010..T-019 entry — via worker dispatch (`echo '<line>' >> ...` or `sed -i` patches), never by loading the full backlog.
- `packet.spec.md` ready to move from `status: draft` → `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md` and confirm each returns PASS.
- Confirm the three gate commands in `packet.spec.md` §Verification are green.
- Record any remaining packet-local risk in the closure log under `.ralph/specs/102_perimeter-modules-foundations/closure-log.md` (e.g., if the manifest-vs-code reconcile direction had to flip from the `[FWD]` default).
- Confirm the implementer's peak context usage stayed under 70% during the run; if it exceeded 70% at any step, log it as a packet-authoring lesson for future spec-packet-generator runs (likely indicates Step 2 or Step 5 needs further subdivision in similar future packets).
