# Implementation Plan: wipe-tower-geometry-keys

Five atomic steps, strictly ordered. Every step names its own falsifying exit condition; a step is not done until that condition is checked and false.

**Blast-radius discipline:** Step 2 adds seven fields to `WipeTower` (`modules/core-modules/wipe-tower/src/lib.rs`). The struct lives under `modules/`, not `crates/*/src`, so it is **not** on the `cargo xtask check-literals` watchlist, and the tree's only construction sites go through `from_config` (including `wipe_tower_from` in `finalization_live_tdd.rs`). Before adding the fields, re-derive that with `rg -n 'WipeTower \{' modules crates` and bring any literal the search finds into the same step's edit list.

---

## Step 1 — Declare the seven keys

- **Task IDs:** none (queue packet; recorded against wayfinder ticket 10).
- **Objective:** the manifest declares exactly the seven keys of AC-1 with canonical types, defaults and bounds, and a guard test fails on any drift or on any returned key reappearing.
- **Preconditions:** re-derive the manifest's current key set from disk (8 today; more if `254a` / `254b` landed). Re-derive whether `modules/core-modules/wipe-tower/tests/wipe_tower_config_schema_tdd.rs` and the `toml` dev-dependency already exist — `254a` authors both, and this step must extend rather than duplicate them.
- **Postconditions:** AC-1 and AC-N3 pass. No module code reads the new keys yet.
- **Allowed reads:** `modules/core-modules/wipe-tower/wipe-tower.toml`, `modules/core-modules/path-optimization-default/path-optimization-default.toml` (enum table shape), `docs/03_wit_and_manifest.md` § `[config.schema]`.
- **Files allowed to edit (≤3):** `modules/core-modules/wipe-tower/wipe-tower.toml`, `modules/core-modules/wipe-tower/Cargo.toml` (dev-dependency only, if absent), `modules/core-modules/wipe-tower/tests/wipe_tower_config_schema_tdd.rs`.
- **Out of bounds:** `modules/core-modules/wipe-tower/src/lib.rs` (Step 2 owns it), `crates/**`.
- **Dispatches:** `FACT` — does `254a` appear landed (its `packet.spec.md` status, plus presence of the schema test and `toml` dev-dep)?
- **Cost:** `S`.
- **Authorities:** `packet.spec.md` AC-1, AC-N3; `requirements.md` § Per-Key Canonical Evidence.
- **Verification:** `cargo test -p wipe-tower --test wipe_tower_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **Falsifying exit condition:** the guard passes while a key's `min`/`max`/`values`/`default` differs from AC-1's table, or while any of the six returned keys is present. If the guard cannot fail on a deliberate drift, the guard is wrong, not the manifest.

---

## Step 2 — Build the wall generator

- **Task IDs:** none.
- **Objective:** `WipeTower` gains the four wall fields (`wall_type`, `cone_angle`, `rib_width`, `extra_rib_length`) plus `fillet_wall`, a `wall_loop(z, layer_depth, tower_top_z)` helper with the three shapes and the fillet pass, and `generate_purge_paths` emits the resulting closed loop ahead of the scan lines.
- **Preconditions:** Step 1 landed (an undeclared key reads as `None`, silently). `tower_top_z` is available: compute it in `run_finalization` as the maximum `z` over layers whose `tool_changes()` are non-empty, and pass it into `generate_purge_paths`.
- **Postconditions:** AC-2 … AC-6 pass. Rotation is **not** applied yet (Step 4); flow is untouched (Step 3).
- **Allowed reads:** `modules/core-modules/wipe-tower/src/lib.rs`, `crates/slicer-ir/src/slice_ir.rs` (`ExtrusionPath3D` / `Point3WithWidth` field lists), `docs/08_coordinate_system.md`.
- **Files allowed to edit (≤3):** `modules/core-modules/wipe-tower/src/lib.rs`, `modules/core-modules/wipe-tower/tests/wipe_tower_wall_tdd.rs` (new).
- **Out of bounds:** everything under `crates/`, the padding table, other packets' directories.
- **Dispatches:** `SUMMARY` ≤ 200 words to the sibling `OrcaSlicerDocumented` if any arc, taper or rounding parameter needs re-checking against `generate_support_cone_wall` / `generate_rib_polygon` / `rounding_polygon`. Never open the checkout directly.
- **Cost:** `M`.
- **Authorities:** `packet.spec.md` AC-2 … AC-6; `design.md` § Selected Approach, DIV-1, DIV-2, DIV-5; `requirements.md` § Per-Key Canonical Evidence.
- **Verification:** `cargo test -p wipe-tower --test wipe_tower_wall_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **Falsifying exit condition:** the rib ring is not a single closed non-self-intersecting ring for `rib_width` at its clamp (`min(layer_depth, tower_width) / 2`), or the cone ring at `cone_angle = 0.0` is not vertex-identical to the rectangle ring. Either means the shape construction is wrong, not the test.

---

## Step 3 — Wire `wipe_tower_extra_flow` as the effective width

- **Task IDs:** none.
- **Objective:** `extra_flow` is read as a percent factor and folded into one `effective_width` driving the scan lines' point `width`, the scan-line pitch and the purge cross-section, with `flow_factor` carrying the factor to the emitter.
- **Preconditions:** Step 2 landed. Re-derive the current pitch and depth expressions from `generate_purge_paths` (they are `line_width` and `purge_volume / cross_section` today; `254a` replaces both) — do not transcribe a formula from the packet without checking the code in front of you.
- **Postconditions:** AC-9 passes; purge volume is invariant across `"100%"` and `"200%"` for the same fixture.
- **Allowed reads:** `modules/core-modules/wipe-tower/src/lib.rs`, `modules/core-modules/wipe-tower/tests/wipe_tower_tdd.rs`.
- **Files allowed to edit (≤3):** `modules/core-modules/wipe-tower/src/lib.rs`, `modules/core-modules/wipe-tower/tests/wipe_tower_tdd.rs`.
- **Out of bounds:** `crates/slicer-gcode/src/emit.rs` (the E computation is read-only context and needs no change), the padding table.
- **Dispatches:** `SNIPPETS` ≤ 1 × 30 lines — the current scan-line loop, to re-derive pitch and cross-section.
- **Cost:** `S`.
- **Authorities:** `packet.spec.md` AC-9; `design.md` DIV-4.
- **Falsifying exit condition:** at `"200%"` the scan-line count is unchanged, or the emitted purge volume differs from the `"100%"` case by more than one line's worth. Either means the factor was applied to `flow_factor` alone — the exact defect the prior revision of this packet shipped as a "wiring".
- **Verification:** `cargo test -p wipe-tower --test wipe_tower_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`

---

## Step 4 — Rotation and the rotated bed check

- **Task IDs:** none.
- **Objective:** every point the module emits passes through `place(p) = origin + R(rotation_angle)·(p − origin)`, and `run_finalization` validates the placed wall ring's vertices against `printable_area` instead of an axis-aligned `tower_width` square.
- **Preconditions:** Steps 2–3 landed, so the wall ring exists and is the geometry the bed check should consume.
- **Postconditions:** AC-7, AC-8 and AC-N2 pass; at `rotation_angle = 0.0` every coordinate is bit-identical to the Step-3 output.
- **Allowed reads:** `modules/core-modules/wipe-tower/src/lib.rs`, `modules/core-modules/wipe-tower/tests/bed_bounds_tdd.rs`.
- **Files allowed to edit (≤3):** `modules/core-modules/wipe-tower/src/lib.rs`, `modules/core-modules/wipe-tower/tests/bed_bounds_tdd.rs`, `modules/core-modules/wipe-tower/tests/finalization_live_tdd.rs`.
- **Out of bounds:** `crates/**`, `ORCA_CONFIG_PADDING`.
- **Dispatches:** `FACT` — `cargo xtask build-guests --check; echo "exit=$?"` at step exit (manifest + `src/` are guest-fingerprint inputs). Exit `3` is `wasm-tools` missing, not clean.
- **Cost:** `M`.
- **Authorities:** `packet.spec.md` AC-7, AC-8, AC-N2; `design.md` DIV-3.
- **Falsifying exit condition:** a rotated tower that visibly leaves the bed still returns `Ok`, or the `0.0` case is not bit-identical. The first means the check still reads the old square; the second means the transform is not the identity at zero.
- **Verification:** `cargo test -p wipe-tower --test bed_bounds_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` then `cargo test -p wipe-tower --test finalization_live_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`

---

## Step 5 — Bounds, leakage, and generated docs

- **Task IDs:** none.
- **Objective:** the scheduler rejects out-of-range and out-of-domain values for the seven keys, the percent default threads into `extensions`, no other module can see the keys, and `docs/15_config_keys_reference.md` is current.
- **Preconditions:** Steps 1–4 landed. `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` and `crates/slicer-runtime/tests/contract/config_view_binding_tdd.rs` both exist and are registered; the scheduler's binary is `scheduler_integration`.
- **Postconditions:** AC-10, AC-11 and AC-N1 pass.
- **Allowed reads:** `crates/slicer-scheduler/src/config_resolution.rs` (`ConfigBoundsIndex` shapes only), the two test files, `crates/slicer-scheduler/tests/integration/config_resolution_tdd.rs` (`LoadedModuleBuilder` fixture shape).
- **Files allowed to edit (≤3):** `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs`, `crates/slicer-runtime/tests/contract/config_view_binding_tdd.rs`, `docs/15_config_keys_reference.md` (**generated only** — via `cargo xtask gen-config-docs`, never by hand).
- **Out of bounds:** any `crates/**/src/**` production file — this packet changes no host logic.
- **Dispatches:** `FACT` for each verification command.
- **Cost:** `M`.
- **Authorities:** `packet.spec.md` AC-10, AC-11, AC-N1.
- **Falsifying exit condition:** `wipe_tower_extra_rib_length = -50.0` is *rejected* (canonical declares no `min`; rejecting it is a manifest bug), or `wipe_tower_wall_type = "hexagon"` is accepted.
- **Verification:** `cargo test -p slicer-scheduler --test scheduler_integration config_bounds_enforcement_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`, `cargo test -p slicer-runtime --test contract config_view_binding_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`, `cargo xtask gen-config-docs --check && rg -q 'wipe_tower_wall_type' docs/15_config_keys_reference.md && rg -q 'wipe_tower_extra_flow' docs/15_config_keys_reference.md && rg -q 'wipe_tower_fillet_wall' docs/15_config_keys_reference.md; echo "exit=$?"`

---

## Closure Gate

Run in this order, all delegated with `FACT` returns:

1. `cargo check --workspace --all-targets`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo xtask check-literals 2>&1 | tail -3`
4. `cargo xtask build-guests --check; echo "exit=$?"` — must be `0`
5. Every AC command in `requirements.md` § Verification Matrix

`cargo test --workspace` runs only if this packet's acceptance ceremony demands it, and then only through `cargo xtask test --summary --workspace`, dispatched to a sub-agent returning `FACT pass/fail`.

## Reporting Obligations (not code changes)

The implementer/reviewer reports these upward; this packet edits neither the map nor the tickets.

- `docs/specs/orca-feature-gap/issues/key-correction-inventory.md` — the Q3(a) rulings row lists `wipe_tower_wall_type` among the holder-only enums. The human ruled at this packet's authoring that the tower wall needs no holder, which moves the key to the Q8 in-module-branching row. The row needs amending before this packet merges.
- The same file's rows for the six returned keys should move from "packet 255" to *unimplemented, returned to queue*, each with the missing feature named in `requirements.md` § Returned to Queue.
- `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` — P03's rows are Tier A; the seven kept keys are Tier B work under this packet.
- Wayfinder ticket 10's "declare in the owner's manifest + wire" line is superseded by Authoring rules 1–6 for this packet.
