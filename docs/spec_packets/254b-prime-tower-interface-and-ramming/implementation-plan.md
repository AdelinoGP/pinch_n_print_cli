# Implementation Plan: prime-tower-interface-and-ramming

Steps are ordered and atomic. **Step 0 is a hard gate: `254a-prime-tower-geometry-keys` must be implemented and merged first.** No step is rated L; Steps 5 and 6 are the finest split of the new-module work that keeps the tree compiling and the scheduler's core-module count true at every commit boundary.

---

## Step 0 — Confirm the `254a` FORWARD-DEP is satisfied

- Task IDs: none (the backlog slice is the wayfinder map's P02, not a `docs/07` `TASK-###`).
- Objective: prove the attachment points this packet composes with actually exist before touching anything.
- Precondition: none.
- Postcondition: `modules/core-modules/wipe-tower/src/lib.rs` contains a `plan_layer_depths`-equivalent and `generate_purge_paths` takes `depth_offset` and `block_depth` parameters; `docs/spec_packets/254a-prime-tower-geometry-keys/packet.spec.md` frontmatter reads `status: implemented`.
- Allowed reads: `modules/core-modules/wipe-tower/src/lib.rs` located window around `generate_purge_paths`; `254a`'s `packet.spec.md` frontmatter line only.
- Files allowed to edit (0): none — this is a gate.
- Out of bounds: everything.
- Dispatches: `FACT: does generate_purge_paths take depth_offset and block_depth? what is 254a's status: line?`
- Context cost: **S**.
- Authorities: `packet.spec.md` §Prerequisites and Blockers.
- Verification: `rg -q 'depth_offset' modules/core-modules/wipe-tower/src/lib.rs && rg -q '^status: implemented' docs/spec_packets/254a-prime-tower-geometry-keys/packet.spec.md; echo "exit=$?"`
- Falsifying exit: either check fails → **stop**; the packet is not ready and forking `generate_purge_paths` a second time is prohibited.

---

## Step 1 — Declare the seven interface/ramming keys in the `wipe-tower` manifest

- Objective: make the seven keys visible to `wipe-tower` and pin their canonical shape.
- Precondition: Step 0 passed. The manifest carries `254a`'s 12 keys.
- Postcondition: 19 keys declared, the seven new ones at exactly AC-1's types/defaults/bounds; `tests/wipe_tower_config_schema_tdd.rs` (created by `254a`) is extended and passes.
- Allowed reads: `modules/core-modules/wipe-tower/wipe-tower.toml`, `modules/core-modules/wipe-tower/tests/wipe_tower_config_schema_tdd.rs`.
- Files allowed to edit (2): those two.
- Out of bounds: `src/lib.rs` (no wiring here), the new module, every other module, `ORCA_CONFIG_PADDING`, both sibling packet directories.
- Dispatches: `cargo test -p wipe-tower --test wipe_tower_config_schema_tdd` → FACT pass/fail.
- Context cost: **S**.
- Authorities: `requirements.md` §Per-Key Canonical Evidence; `docs/03_wit_and_manifest.md` §Host-Boundary Access Enforcement (Normative).
- Verification: `cargo test -p wipe-tower --test wipe_tower_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` (AC-1, AC-N3), then `cargo xtask build-guests --check; echo "exit=$?"` → 0.
- Falsifying exit: the guard fails, or any declared default differs from `requirements.md`'s table — in particular `enable_filament_ramming` must default `true`, not `false`.

---

## Step 2 — Build the interface block: gate, purge volume, lead-in travel and lead-in extrusion

- Objective: make four keys change emitted geometry.
- Precondition: Step 1 landed.
- Postcondition: `WipeTower` carries `interface_features: bool`, `interface_purge_volume: f32`, `pre_extrusion_dist: f32`, `pre_extrusion_length: f32`; `generate_purge_paths` uses `effective_volume = if interface_features { interface_purge_volume } else { purge_volume }` as `254a`'s block-depth numerator, emits a `pre_extrusion_dist`-long leading travel when the gate is on, and emits one extruding lead-in entity of `pre_extrusion_length` path length (clamped to `tower_width`) when that length is `> 0.0` and the gate is on. With the gate off, output is identical to `254a`'s.
- Allowed reads: `modules/core-modules/wipe-tower/src/lib.rs` — **772 lines before `254a`'s edits, over the 600-line ceiling**; located windows around `from_config`, `generate_purge_paths` and `mod tests` only. `modules/core-modules/wipe-tower/tests/wipe_tower_tdd.rs`.
- Files allowed to edit (2): `modules/core-modules/wipe-tower/src/lib.rs`, `modules/core-modules/wipe-tower/tests/wipe_tower_tdd.rs`.
- Out of bounds: every crate under `crates/`, the new module, the manifest (Step 1 owns it).
- Dispatches: `cargo test -p wipe-tower` → FACT pass/fail; on failure SNIPPETS ≤ 20 lines.
- Context cost: **M**.
- Authorities: `requirements.md` rows for `enable_tower_interface_features`, `filament_tower_interface_purge_volume`, `_pre_extrusion_dist`, `_pre_extrusion_length`, and D-254b-1; `design.md` §Selected Approach Part 1 steps 1–3, INV-1, INV-2.
- Verification: `cargo test -p wipe-tower --test wipe_tower_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` (AC-3, AC-4, AC-5), then `cargo xtask build-guests --check; echo "exit=$?"` → 0.
- Falsifying exit: any entity escapes the block's Y band or the tower's X span (INV-2); the gate-off path differs from `254a`'s output by a single vertex; or `pre_extrusion_length = 0.0` still emits a lead-in entity.

---

## Step 3 — Build the ramming zigzag

- Objective: make `enable_filament_ramming` change emitted geometry, and own the resulting default-path change.
- Precondition: Step 2 landed.
- Postcondition: `WipeTower` carries `enable_filament_ramming: bool` (default `true`); when set, one ramming zigzag entity covers the block's leading `y_step = (infill_gap_percent / 100) × line_width` band at `flow_factor = 1.0`, placed after the lead-in and before the scan lines. Every entity-count assertion in `tests/` and in `src/lib.rs`'s `#[cfg(test)]` module is updated **to the new expected count**, never loosened.
- Allowed reads: as Step 2.
- Files allowed to edit (2): `modules/core-modules/wipe-tower/src/lib.rs`, `modules/core-modules/wipe-tower/tests/wipe_tower_tdd.rs`.
- Out of bounds: as Step 2.
- Dispatches: `cargo test -p wipe-tower` → FACT pass/fail. If the `y_step` basis is disputed, dispatch a SUMMARY read of `WipeTower::toolchange_Unload` (`OrcaSlicerDocumented/src/libslic3r/GCode/WipeTower.cpp`) — never read it in-context.
- Context cost: **S**.
- Authorities: `requirements.md` `enable_filament_ramming` row and D-254b-4; `design.md` §Architecture Constraints "Blast radius — default-path change from `enable_filament_ramming`".
- Verification: `cargo test -p wipe-tower --test wipe_tower_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` (AC-7, AC-N1), then `cargo xtask build-guests --check; echo "exit=$?"` → 0.
- Falsifying exit: a baseline was made to pass by loosening an assertion rather than by computing the new expected count — a gate-gaming stop; or `false` leaves any entity that `254a` did not emit.

---

## Step 4 — Build the flat-ironing pass

- Objective: make `prime_tower_flat_ironing` and `filament_tower_ironing_area` change emitted geometry, under canonical's conjunction shape.
- Precondition: Step 3 landed.
- Postcondition: `WipeTower` carries `flat_ironing: bool` and `ironing_area: f32`; when `interface_features && flat_ironing`, a trailing `ExtrusionRole::Ironing` boustrophedon pass covers `ironing_span = (ironing_area / tower_width).min(block_depth)` of the block's depth at the block's pitch; either flag off emits nothing.
- Allowed reads: as Step 2, plus `crates/slicer-ir/src/slice_ir.rs` located window around `ExtrusionRole` (read-only, to confirm the `Ironing` variant).
- Files allowed to edit (2): `modules/core-modules/wipe-tower/src/lib.rs`, `modules/core-modules/wipe-tower/tests/wipe_tower_tdd.rs`.
- Out of bounds: as Step 2; also `crates/slicer-ir/src/slice_ir.rs` (read-only).
- Dispatches: `cargo test -p wipe-tower` → FACT pass/fail.
- Context cost: **M**.
- Authorities: `requirements.md` `prime_tower_flat_ironing` / `filament_tower_ironing_area` rows and D-254b-3; `design.md` INV-3, and the coord-system note that `ironing_area` is **mm²** converted to depth by dividing by `tower_width`.
- Verification: `cargo test -p wipe-tower --test wipe_tower_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` (AC-6), then `cargo xtask build-guests --check; echo "exit=$?"` → 0.
- Falsifying exit: an `Ironing` entity appears with either flag off (INV-3); the ironing pass escapes the block's band (INV-2); or the area→depth conversion omits the `tower_width` divisor.

---

## Step 5 — Scaffold the `prime-tower-interface` module (crate, guest, manifest, discovery, count)

- Objective: create the new core module so it is discovered and the tree stays green — without yet wiring it into the integrated registry.
- Precondition: Steps 1–4 landed. `crates/slicer-scheduler/tests/integration/manifest_ingestion_tdd.rs` asserts exactly **23** core modules, with a dated comment and a failure message naming the number.
- Postcondition: `modules/core-modules/prime-tower-interface/` exists with `Cargo.toml`, `prime-tower-interface.toml` (stage `PostPass::GCodePostProcess`, `[ir-access] reads = ["GCodeIR"]`, `holds = []` / `requires = []`, the two `[config.schema.*]` tables from AC-2), `src/lib.rs` (a `run_gcode_postprocess` that returns `Ok(())` — behaviour is Step 7), and `wit-guest/{Cargo.toml,src/lib.rs}`; the workspace member list includes it; the core-module count assertion reads **24** with its comment re-dated.
- Allowed reads: `modules/core-modules/machine-gcode-emit/` (the `PostPass::GCodePostProcess` structural template — manifest, `Cargo.toml`, `wit-guest/`, and the `run_gcode_postprocess` override shape), `crates/slicer-scheduler/tests/integration/manifest_ingestion_tdd.rs` located window around the count assertion, the root workspace `Cargo.toml` member list.
- Files allowed to edit (3 logical units — the new module directory counts as one, since it is created whole): `modules/core-modules/prime-tower-interface/**` (new), the root workspace `Cargo.toml`, `crates/slicer-scheduler/tests/integration/manifest_ingestion_tdd.rs`.
- Out of bounds: `crates/slicer-integrated-modules/`, `crates/slicer-runtime/`, `crates/pnp-cli/` (Step 6 owns them); `crates/slicer-schema/wit/` — **if the scaffold appears to need a WIT change, stop and report a `[BLOCK]`; do not edit WIT**.
- Dispatches: `cargo check --workspace --all-targets` → FACT pass/fail; `cargo xtask build-guests --check` → FACT exit code; `cargo test -p slicer-scheduler --test integration manifest_ingestion_tdd` → FACT pass/fail.
- Context cost: **M**.
- Authorities: `docs/03_wit_and_manifest.md` (manifest + stage declaration); `design.md` §Code Change Surface and §Architecture Constraints "Blast radius — new core module".
- Verification: `cargo test -p slicer-scheduler --test integration manifest_ingestion_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` (AC-2), then `cargo xtask build-guests --check; echo "exit=$?"` → 0, then `cargo check --workspace --all-targets`.
- Falsifying exit: `build-guests --check` returns exit `3` (`wasm-tools` missing) and is read as clean — it prints no `STALE:` line and is **not** a pass; or the count assertion was changed to a range/inequality instead of `24`.

---

## Step 6 — Register the module in the integrated edition

- Objective: make the new module available on every edition, not just the wasm path.
- Precondition: Step 5 landed and the tree is green.
- Postcondition: `crates/slicer-integrated-modules/{Cargo.toml,src/lib.rs}`, `crates/slicer-runtime/{Cargo.toml,src/lib.rs}` and `crates/pnp-cli/Cargo.toml` carry the same entries they carry for `skirt-brim`.
- Allowed reads: those five files, plus a `LOCATIONS` dispatch enumerating every `skirt-brim` / `skirt_brim` reference across them (the packet's authoritative list of what registration touches).
- Files allowed to edit (5): the five named above. This exceeds the usual 3-edit cap; it is justified because the five are one indivisible edge — a dependency added in a `Cargo.toml` without the matching registry entry does not compile, and vice versa.
- Out of bounds: `modules/core-modules/prime-tower-interface/**` (Step 5 owns it), `crates/slicer-schema/wit/`.
- Dispatches: the `LOCATIONS` sweep above; `cargo check --workspace --all-targets` → FACT pass/fail.
- Context cost: **M**.
- Authorities: `design.md` §Architecture Constraints "Blast radius — new core module".
- Verification: `cargo check --workspace --all-targets` and `cargo clippy --workspace --all-targets -- -D warnings`, then `cargo test -p slicer-runtime --test contract integrated_parity_wipe_tower_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`.
- Falsifying exit: the integrated edition builds but the module is absent from its registry (a silent no-op module is worse than a compile error) — confirm by grepping the registry for the module id, not by the build succeeding.

---

## Step 7 — Build the interface temperature emitter

- Objective: make the two temperature keys change the emitted command stream.
- Precondition: Steps 5 and 6 landed.
- Postcondition: `prime-tower-interface`'s `run_gcode_postprocess` returns immediately when `filament_tower_interface_print_temp < 0`; otherwise it identifies maximal `ExtrusionRole::WipeTower` move runs (treating a non-extruding move between two `WipeTower` moves as part of the run) with their nearest preceding `GCodeCommand::ToolChange`, and pushes exactly one `Temperature { tool, celsius, wait: false }` per run — before the first `WipeTower` move when `enable_tower_interface_cooldown_during_tower` is `true`, before the `ToolChange` when it is `false`; `tool` is the `ToolChange`'s `to`, falling back to `0`.
- Allowed reads: `modules/core-modules/prime-tower-interface/src/lib.rs`, `crates/slicer-sdk/src/postpass_builders.rs` located window around `push_temperature`, `crates/slicer-sdk/src/traits.rs` located window around `run_gcode_postprocess`, `crates/slicer-ir/src/slice_ir.rs` located window around `GCodeCommand`. **All three crate files are far over the 600-line ceiling — located windows only.**
- Files allowed to edit (2): `modules/core-modules/prime-tower-interface/src/lib.rs`, `modules/core-modules/prime-tower-interface/tests/interface_temp_tdd.rs` (new; a fresh `tests/` dir has no aggregator, so it is a standalone binary needing no `mod` registration).
- Out of bounds: `crates/**` (read-only in this step), `modules/core-modules/wipe-tower/`.
- Dispatches: `cargo test -p prime-tower-interface --test interface_temp_tdd` → FACT pass/fail. If the cooldown branch's meaning is disputed, dispatch a SUMMARY read of `WipeTower2::tool_change`.
- Context cost: **M**.
- Authorities: `requirements.md` rows for `filament_tower_interface_print_temp` / `enable_tower_interface_cooldown_during_tower`, D-254b-2, D-254b-5; `design.md` §Selected Approach Part 2, INV-4, INV-5, R-4.
- Verification: `cargo test -p prime-tower-interface --test interface_temp_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` (AC-8, AC-9, AC-N2), then `cargo xtask build-guests --check; echo "exit=$?"` → 0.
- Falsifying exit: more than one `Temperature` per tower run (INV-4, R-4); any command pushed at `-1` (INV-5); or the run test reports `0 tests` (a false pass).

---

## Step 8 — Bounds arm, deviations, generated docs, closure gates

- Objective: prove bounds enforcement, register the divergences, refresh generated docs, and show the tree green.
- Precondition: Steps 1–7 landed.
- Postcondition: `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` carries AC-10's cases (the file already exists and is already registered in that directory's `main.rs`); `docs/DEVIATION_LOG.md` carries one row each for D-254b-1 … D-254b-5 with `DEV-###` IDs **re-derived from the log at write time** (`rg -o '^\| DEV-[0-9]{3}' docs/DEVIATION_LOG.md | sort -u | tail -1`, take the next) — never an ID frozen at authoring (CLAUDE.md ledger-fact rule); `docs/15_config_keys_reference.md` regenerated.
- Allowed reads: `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs`; `docs/DEVIATION_LOG.md` tail only.
- Files allowed to edit (3): `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs`, `docs/DEVIATION_LOG.md`, `docs/15_config_keys_reference.md` — the last **only** as the output of `cargo xtask gen-config-docs`, never by hand.
- Out of bounds: every source file under `modules/` and `crates/*/src/` (this is a test-and-gate step); `ORCA_CONFIG_PADDING`.
- Dispatches: each gate command → FACT exit code / pass-fail; additionally `cargo test -p slicer-runtime --test executor finalization_live_tdd` → FACT pass/fail, because it sets `prime_volume` and is the most likely default-path fallout from Step 3's ramming default.
- Context cost: **S**.
- Authorities: CLAUDE.md §Build & Test Commands, §Test Discipline, §Guest WASM Staleness.
- Verification: `cargo test -p slicer-scheduler --test integration config_bounds_enforcement_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` (AC-10) · `cargo xtask gen-config-docs --check` (AC-11) · `cargo xtask check-deviations --check` · `cargo xtask check-literals` · `cargo check --workspace --all-targets` · `cargo clippy --workspace --all-targets -- -D warnings` · `cargo xtask build-guests --check; echo "exit=$?"`.
- Falsifying exit: any gate non-zero; `gen-config-docs --check` reports drift after regeneration; or a `DEV-###` ID collides because it was frozen at authoring instead of re-derived.

---

## Blast-radius discipline

- **New core module (Steps 5–6).** The registration edge spans `modules/core-modules/prime-tower-interface/**`, the root workspace `Cargo.toml`, `crates/slicer-integrated-modules/{Cargo.toml,src/lib.rs}`, `crates/slicer-runtime/{Cargo.toml,src/lib.rs}`, `crates/pnp-cli/Cargo.toml`, and the **hard-asserted core-module count** in `crates/slicer-scheduler/tests/integration/manifest_ingestion_tdd.rs` (currently `23`, becomes `24`). That count is test-assertion fallout pre-baked into Step 5's edit list, not left for a follow-up `cargo check`.
- **`WipeTower` struct fields (Steps 2–4).** Seven new fields. Construction goes through `WipeTower::from_config` in the tests, so no `WipeTower { .. }` literal should need a `..` rest — confirm with `cargo xtask check-literals` rather than assuming. New fixtures for watched types carry a `..` rest or an `// exhaustive: <reason>` waiver.
- **Default-path change (Step 3 only).** `enable_filament_ramming` defaults `true`, so every purge block gains an entity. Its assertion fallout — in `modules/core-modules/wipe-tower/tests/`, in `src/lib.rs`'s `#[cfg(test)]` module, and in `crates/slicer-runtime/tests/{contract/integrated_parity_wipe_tower_tdd.rs, executor/finalization_live_tdd.rs}` — is owned by Step 3 for the module-local files and checked in Steps 6 and 8 for the cross-crate ones. Steps 2 and 4 are default-identity by construction, which is what makes Step 3 the single suspect if a baseline moves.
- No public schema constant or version is bumped, so there is no "hard-asserts the old constant value" fallout beyond the core-module count.
