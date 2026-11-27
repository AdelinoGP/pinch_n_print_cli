# Implementation Plan: prime-tower-geometry-keys

Steps are ordered and atomic. Steps 3 and 6 must land in one commit (see `requirements.md` §Step Completion Expectations). No step is rated L.

---

## Step 1 — Declare the four keys in the `wipe-tower` manifest and guard them

- Task IDs: none (the backlog slice is the wayfinder map's P02, not a `docs/07` `TASK-###`).
- Objective: make the three prime-tower keys plus `layer_height` visible to the module through `bind_module_config_view`, and pin their canonical shape against drift.
- Precondition: `modules/core-modules/wipe-tower/wipe-tower.toml` declares exactly 8 keys (`enable_prime_tower`, `wipe_tower_x`, `wipe_tower_y`, `prime_tower_width`, `prime_volume`, `line_width`, `printable_area`, `retract_length`). `modules/core-modules/wipe-tower/Cargo.toml` has exactly one dev-dependency (`slicer-sdk` with `features = ["test"]`) — **no `toml`**. `modules/core-modules/wipe-tower/tests/` has no aggregator `main.rs`.
- Postcondition: the manifest declares 12 keys — the 8 plus `prime_tower_infill_gap`, `prime_tower_brim_width`, `prime_tower_enable_framework`, `layer_height` — at the exact types/defaults/bounds in AC-1; `tests/wipe_tower_config_schema_tdd.rs` exists as a standalone binary and passes; `toml = "0.8"` is a dev-dependency.
- Allowed reads: `modules/core-modules/wipe-tower/wipe-tower.toml`, `modules/core-modules/wipe-tower/Cargo.toml`, `modules/core-modules/part-cooling/Cargo.toml` (dev-dep precedent), `modules/core-modules/classic-perimeters/classic-perimeters.toml` (host-key declaration precedent).
- Files allowed to edit (3): `modules/core-modules/wipe-tower/wipe-tower.toml`, `modules/core-modules/wipe-tower/tests/wipe_tower_config_schema_tdd.rs` (new), `modules/core-modules/wipe-tower/Cargo.toml`.
- Out of bounds: `src/lib.rs` (no wiring in this step), every other module, `ORCA_CONFIG_PADDING`, both sibling packet directories.
- Dispatches: `cargo test -p wipe-tower --test wipe_tower_config_schema_tdd` → FACT pass/fail.
- Context cost: **S**.
- Authorities: `requirements.md` §Per-Key Canonical Evidence; `docs/03_wit_and_manifest.md` §Host-Boundary Access Enforcement (Normative).
- Verification: `cargo test -p wipe-tower --test wipe_tower_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` (AC-1, AC-N3), then `cargo xtask build-guests --check; echo "exit=$?"` → 0.
- Falsifying exit: the guard fails; any declared default differs from `requirements.md`'s table; any of the nine `254b` keys or `prime_tower_skip_points` appears in this manifest.

---

## Step 2 — Wire `prime_tower_infill_gap` to the scan-line pitch

- Objective: make the pitch configurable and own the resulting default-path baseline change.
- Precondition: Step 1 landed. `generate_purge_paths` advances with `y += self.line_width`.
- Postcondition: `WipeTower` carries `infill_gap_percent: f32` from `from_config` (read as `ConfigValue::Percent`, fallback `150.0`); the advance becomes `y += (infill_gap_percent / 100.0) * self.line_width`; every pitch-pinned assertion in `tests/wipe_tower_tdd.rs` and in `src/lib.rs`'s `#[cfg(test)]` module is updated **to the formula value**, never loosened.
- Allowed reads: `modules/core-modules/wipe-tower/src/lib.rs` — **772 lines at authoring, over the 600-line ceiling**; read located windows around `from_config`, `generate_purge_paths` and `mod tests` only. `modules/core-modules/wipe-tower/tests/wipe_tower_tdd.rs`.
- Files allowed to edit (2): `modules/core-modules/wipe-tower/src/lib.rs`, `modules/core-modules/wipe-tower/tests/wipe_tower_tdd.rs`.
- Out of bounds: every crate under `crates/`, every other module, the manifest (Step 1 owns it).
- Dispatches: `cargo test -p wipe-tower` → FACT pass/fail; on failure SNIPPETS ≤ 20 lines.
- Context cost: **S**.
- Authorities: `requirements.md` §Per-Key Canonical Evidence `prime_tower_infill_gap` row and D-254a-2.
- Verification: `cargo test -p wipe-tower --test wipe_tower_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` (AC-2), then `cargo xtask build-guests --check; echo "exit=$?"` → 0.
- Falsifying exit: a baseline was made to pass by loosening an assertion rather than by computing the new expected value — that is a gate-gaming stop; or the pitch drops below `line_width` for any value the bounds accept (`min = 100`).

---

## Step 3 — Build the per-layer tower depth model

- Objective: give each purge block its own depth band so `prime_tower_enable_framework` has something to act on, and fix the overlapping-blocks defect.
- Precondition: Step 2 landed. Every block starts at `y = tower_y`; `generate_purge_paths` takes `_tc` unused; `run_finalization` sorts tool changes by `Reverse(tc.after_entity_index)` before inserting.
- Postcondition: a private `plan_layer_depths(layers) -> Vec<f32>` computes `block_depth` per layer and `layer_depth[i] = n_tool_changes(i) × block_depth[i]`; `generate_purge_paths` takes `depth_offset: f32` and `block_depth: f32`; block `k` (the block's **ascending** `after_entity_index` rank, computed **before** the reverse sort) spans `[tower_y + k·block_depth, tower_y + (k+1)·block_depth)`. `layer_height` comes from the declared key, falling back to the existing Δz derivation, with `DEFAULT_LAYER_HEIGHT` only for layer 0.
- Allowed reads: `modules/core-modules/wipe-tower/src/lib.rs` (located windows: `run_finalization`, `generate_purge_paths`, `mod tests`), `modules/core-modules/wipe-tower/tests/wipe_tower_tdd.rs`, `crates/slicer-sdk/src/traits.rs` located window around `LayerCollectionView::tool_changes`.
- Files allowed to edit (2): `modules/core-modules/wipe-tower/src/lib.rs`, `modules/core-modules/wipe-tower/tests/wipe_tower_tdd.rs`.
- Out of bounds: `crates/slicer-sdk/src/traits.rs` (read-only — no SDK change is needed or permitted); as Step 2 otherwise.
- Dispatches: `cargo test -p wipe-tower` → FACT pass/fail.
- Context cost: **M**.
- Authorities: `design.md` §Selected Approach steps 1–2, INV-1, INV-2, INV-6, R-1; `requirements.md` D-254a-1.
- Verification: `cargo test -p wipe-tower --test wipe_tower_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` (AC-3), then `cargo xtask build-guests --check; echo "exit=$?"` → 0.
- Falsifying exit: two blocks on one layer share a scan-line Y (INV-1); the depth rank was taken from the reversed iteration index (INV-6, R-1); or the single-tool-change case changes position at all.
- **Commit note:** land with Step 6. The depth model is exactly what makes the old `tower_width`-square bed bound wrong.

---

## Step 4 — Wire `prime_tower_enable_framework` (uniform depth)

- Objective: make the framework flag change emitted geometry.
- Precondition: Step 3 landed.
- Postcondition: `WipeTower` carries `enable_framework: bool`; when set, `plan_layer_depths` overwrites every tower layer's `layer_depth` with the **first tower layer's** (the first `view` with a non-empty `tool_changes()`), and the layer's last block absorbs the padding so the layer's span reaches that depth.
- Allowed reads: as Step 3.
- Files allowed to edit (2): `modules/core-modules/wipe-tower/src/lib.rs`, `modules/core-modules/wipe-tower/tests/wipe_tower_tdd.rs`.
- Out of bounds: as Step 3.
- Dispatches: `cargo test -p wipe-tower` → FACT pass/fail. If the forcing semantics are disputed, dispatch a SUMMARY read of `WipeTower::generate_wipe_tower_blocks` (`OrcaSlicerDocumented/src/libslic3r/GCode/WipeTower.cpp`) — never read it in-context.
- Context cost: **S**.
- Authorities: `requirements.md` §Per-Key Canonical Evidence `prime_tower_enable_framework` row; `design.md` INV-3.
- Verification: `cargo test -p wipe-tower --test wipe_tower_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` (AC-4), then `cargo xtask build-guests --check; echo "exit=$?"` → 0.
- Falsifying exit: `false` (the default) changes any output relative to Step 3; or `true` leaves any tower layer's depth unequal to the first tower layer's (INV-3).

---

## Step 5 — Build `prime_tower_brim_width` (first-layer brim + Auto sentinel)

- Objective: emit brim rings around the tower footprint on the first tower layer.
- Precondition: Step 4 landed.
- Postcondition: `WipeTower` carries `brim_width: f32`; private `auto_brim_width(top_z)` returns `if top_z < 100.0 { top_z / 100.0 * 8.0 } else { 8.0 }`; private `brim_loops(footprint, top_z)` computes `spacing = line_width − layer_height × (1 − π/4)`, `loops_num = floor((width + spacing/2) / spacing)`, and emits loop `i` as the footprint rect offset outward by `(i + 1) × spacing`, at `ExtrusionRole::WipeTower`, `tool_index = 0`, `RegionKey.object_id = "__wipe_tower__"`. Runs only on the first layer with a non-empty `tool_changes()`.
- Allowed reads: as Step 3.
- Files allowed to edit (2): `modules/core-modules/wipe-tower/src/lib.rs`, `modules/core-modules/wipe-tower/tests/wipe_tower_tdd.rs`.
- Out of bounds: as Step 3.
- Dispatches: `cargo test -p wipe-tower` → FACT pass/fail. If the spacing or `loops_num` formula is disputed, dispatch a SUMMARY read of `WipeTower2::finish_layer` (`OrcaSlicerDocumented/src/libslic3r/GCode/WipeTower2.cpp`).
- Context cost: **M**.
- Authorities: `requirements.md` §Per-Key Canonical Evidence `prime_tower_brim_width` row, D-254a-3, D-254a-4, D-254a-5; `design.md` INV-4, and the coord-system constraint (**port the formulas, never `scale_`**).
- Verification: `cargo test -p wipe-tower --test wipe_tower_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` (AC-5, AC-6), then `cargo xtask build-guests --check; echo "exit=$?"` → 0.
- Falsifying exit: brim loops appear on more than one layer (INV-4); loops offset inward; `brim_width = 0.0` still emits a loop; or a `scale_`/`unscale` conversion was ported into a mm-float module.

---

## Step 6 — Widen the bed-bounds check to the planned tower extent

- Objective: validate the real footprint the tower now occupies, instead of the `tower_width`-square approximation.
- Precondition: Steps 3 and 5 landed (the check needs both `max_i(layer_depth[i])` and `brim_extent`).
- Postcondition: the corner check in `run_finalization` validates `x ∈ [tower_x − brim_extent, tower_x + tower_width + brim_extent]` and `y ∈ [tower_y − brim_extent, tower_y + max_i(layer_depth[i]) + brim_extent]`, with `brim_extent = loops_num × spacing`; the failure path still returns `ModuleError::fatal(3, ...)` naming the offending corner.
- Allowed reads: `modules/core-modules/wipe-tower/src/lib.rs` located window around the bed-bounds block, `modules/core-modules/wipe-tower/tests/bed_bounds_tdd.rs`.
- Files allowed to edit (2): `modules/core-modules/wipe-tower/src/lib.rs`, `modules/core-modules/wipe-tower/tests/bed_bounds_tdd.rs`.
- Out of bounds: as Step 3.
- Dispatches: `cargo test -p wipe-tower --test bed_bounds_tdd` → FACT pass/fail.
- Context cost: **S**.
- Authorities: `design.md` §Selected Approach step 5, R-4.
- Verification: `cargo test -p wipe-tower --test bed_bounds_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` (AC-7), then `cargo xtask build-guests --check; echo "exit=$?"` → 0.
- Falsifying exit: the existing `bed_bounds_tdd` fixtures (`tower_geometry_within_config_bed_only`, `orca_point_string_bed_is_parsed_not_silently_defaulted`, `orca_point_string_bed_accepts_a_tower_that_fits`) were made to pass by shrinking the asserted extent rather than by recomputing it from the plan.
- **Commit note:** land with Step 3.

---

## Step 7 — Author the scheduler bounds arm and the module-visibility arm

- Objective: prove the manifest bounds are enforced before values reach `ConfigView`, and that the new declarations leak into no other module.
- Precondition: Steps 1–6 landed.
- Postcondition: `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` gains AC-8's cases (`"99%"` and `-2.0` rejected; `"110%"` and `-1.0` accepted; the `"150%"` schema default threading into `extensions`); `crates/slicer-runtime/tests/contract/config_view_binding_tdd.rs` gains AC-N1's case. **Both files already exist and are already registered** in `crates/slicer-scheduler/tests/integration/main.rs` and `crates/slicer-runtime/tests/contract/main.rs` respectively, so no aggregator edit is needed — verify that before writing, because the former packet 254 cited two test targets (`wipe_tower_config_bounds_tdd.rs`, `undeclared_prime_tower_keys_stay_hidden_from_other_modules`) that **do not exist anywhere in the tree**.
- Allowed reads: those two test files; `crates/slicer-scheduler/src/config_resolution.rs` located window around `ConfigBoundsIndex::{check, schema_defaults}`.
- Files allowed to edit (2): `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs`, `crates/slicer-runtime/tests/contract/config_view_binding_tdd.rs`.
- Out of bounds: `crates/slicer-scheduler/src/`, `crates/slicer-runtime/src/` (test-only step), `ORCA_CONFIG_PADDING`.
- Dispatches: both test commands → FACT pass/fail.
- Context cost: **S**.
- Authorities: `requirements.md` §Verification Commands; `design.md` §Code Change Surface.
- Verification: `cargo test -p slicer-scheduler --test integration config_bounds_enforcement_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` and `cargo test -p slicer-runtime --test contract config_view_binding_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`.
- Falsifying exit: either run reports `0 tests` for the new case (an unregistered or mis-filtered arm reads as a false pass); or an out-of-range value passes through to `ConfigView`.

---

## Step 8 — Register deviations, regenerate config docs, run the closure gates

- Objective: register the output-affecting divergences, bring the generated key reference in line, and prove the tree is green.
- Precondition: Steps 1–7 landed.
- Postcondition: `docs/DEVIATION_LOG.md` carries one row each for `requirements.md`'s D-254a-1 through D-254a-5, using `DEV-###` IDs **re-derived from the log at write time** (`rg -o '^\| DEV-[0-9]{3}' docs/DEVIATION_LOG.md | sort -u | tail -1`, take the next) — never an ID frozen at authoring (CLAUDE.md ledger-fact rule); `docs/15_config_keys_reference.md` regenerated; all gates green.
- Allowed reads: `docs/DEVIATION_LOG.md` (tail only, for the ID convention and the highest ID); otherwise command output only.
- Files allowed to edit (2): `docs/DEVIATION_LOG.md` (appended rows), `docs/15_config_keys_reference.md` — the latter **only** as the output of `cargo xtask gen-config-docs`, never by hand.
- Out of bounds: every source file (this is a gate step).
- Dispatches: each gate command → FACT exit code / pass-fail; additionally `cargo test -p slicer-runtime --test contract integrated_parity_wipe_tower_tdd` and `cargo test -p slicer-runtime --test executor finalization_live_tdd` → FACT pass/fail, because both set `prime_volume` and are the most likely default-path fallout outside the module.
- Context cost: **S**.
- Authorities: CLAUDE.md §Build & Test Commands, §Test Discipline, §Guest WASM Staleness; `design.md` §Architecture Constraints "Blast radius — output change at defaults".
- Verification: `cargo xtask gen-config-docs --check` (AC-9) · `cargo xtask check-deviations` · `cargo xtask check-literals` · `cargo check --workspace --all-targets` · `cargo clippy --workspace --all-targets -- -D warnings` · `cargo xtask build-guests --check; echo "exit=$?"`.
- Falsifying exit: any gate non-zero; `gen-config-docs --check` reports drift after regeneration; or a `DEV-###` ID collides because it was frozen at authoring instead of re-derived.

---

## Blast-radius discipline

- Steps 2–5 add fields to `WipeTower` and change `generate_purge_paths`' signature. The owning step updates **every** call site — `run_finalization`, `src/lib.rs`'s `#[cfg(test)]` module, and `tests/wipe_tower_tdd.rs` — in the same step, not via a follow-up `cargo check`.
- `WipeTower` is constructed through `WipeTower::from_config` in the tests, so no `WipeTower { .. }` literal should need a `..` rest. The step that adds the fields confirms this with `cargo xtask check-literals` rather than assuming it. Any new test fixture for a watched type carries a `..` rest or an `// exhaustive: <reason>` waiver.
- Two default-path changes land (pitch `0.4 → 0.6`; blocks stop overlapping). Their assertion fallout is owned by Steps 2 and 3 respectively, split precisely so a failing baseline identifies which change caused it (R-2). Cross-crate fallout in `slicer-runtime`'s two `prime_volume` tests is checked in Step 8.
- No public schema constant or version is bumped, so there is no "hard-asserts the old constant value" fallout.
