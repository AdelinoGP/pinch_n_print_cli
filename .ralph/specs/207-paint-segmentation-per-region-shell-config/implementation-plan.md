# Implementation Plan: 207-paint-segmentation-per-region-shell-config

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".
- **Never run `cargo test -p slicer-core` without `--features host-algos` in this packet.** The new integration file is gated by an inner `#![cfg(feature = "host-algos")]`, and the new in-crate test module lives under `pub mod algos`, which `crates/slicer-core/src/lib.rs` gates on the same feature. A bare run compiles *both* to nothing and prints `ok`. Every verification command below carries the feature flag and an **exact** count assertion.
- **Ten tests, two homes.** Six ACs (AC-1, AC-4, AC-5, AC-N1, AC-N3, AC-N4) unit-call module-private items and therefore run in-crate via `--lib`; four (AC-2, AC-6, AC-9, AC-N2) drive the public `execute_paint_segmentation` and run in the integration binary via `--test`. `crates/slicer-core/tests/*.rs` is a separate crate and cannot see private items — this is a compile constraint, not a preference. Full table and rationale: `design.md` §Test Homing.

## Steps

### Step 1: Red tests for per-`variant_chain` shell-config resolution

- Task IDs: `TASK-323`
- Objective: Write all ten failing tests, **in the two homes `design.md` §Test Homing assigns**, plus the minimum production scaffolding that lets the crate compile:
  1. **New integration binary** `crates/slicer-core/tests/paint_segmentation_per_region_shell_config_tdd.rs` with the `#![cfg(feature = "host-algos")]` preamble and the four end-to-end tests that drive the public `execute_paint_segmentation`: `top_shell_layers_changes_projection_depth` (AC-2), `multi_object_shell_counts_are_independent` (AC-6), `per_variant_chain_shell_counts_are_independent` (AC-9), `zero_shell_counts_keep_contact_layer` (AC-N2).
  2. **New in-crate `#[cfg(test)] mod shell_config_resolver_tests`** in `crates/slicer-core/src/algos/paint_segmentation/mod.rs`, placed *after* the existing `#[cfg(test)] mod driver_v2_tests` and opening with `use super::*;`, carrying the six unit tests that call the module-private items: `resolver_reads_painted_chain_config_not_base_or_placeholder` (AC-1), `outer_wall_line_width_is_honoured` (AC-4), `nozzle_diameter_comes_from_config` (AC-5), `missing_region_key_uses_single_documented_fallback` (AC-N1), `percent_widths_resolve_against_nozzle_base` (AC-N3), `missing_chain_falls_back_to_base_chain_not_default` (AC-N4).
  3. **Placeholder-behaviour stubs** of the five module-private items (`ShellParams`, `region_key_for_chain`, `ext_abs_mm`, `shell_params_from_config`, `resolve_shell_params`) with the exact signatures in `design.md` §Code Change Surface item 2, bodies reproducing **today's** behaviour: `resolve_shell_params` returns `ShellParams { top: 3, bottom: 3, width_mm: 0.45, layer_height_mm: 0.2 }`; `region_key_for_chain` returns `None`; `ext_abs_mm` returns `None`; `shell_params_from_config` ignores `cfg` and returns the same placeholder. **Do not touch the `execute_paint_segmentation` call site in this step** — the `configs.first()` block stays exactly as it is, so the four end-to-end tests fail on real current behaviour.
- **Why the stubs are in the red step, and why they are not cheating.** Rust unit tests for private items must live in the same crate, and a test that names a function which does not exist is a *compile* error, not a red test — the step would be unbuildable, and no `test result:` line would ever print. The stubs make the crate compile while returning precisely the values the packet exists to replace, so all six unit ACs fail on an assertion (a wrong number), which is what "red" means. Any stub that returns a *correct* value, or that reads `region_map`, is a violation of this step.
- Precondition: packet 206 is `implemented` and its seam writer is present in `execute_paint_segmentation`.
- Postcondition: both runs compile under `--features host-algos`; the `--lib` filter reports exactly 6 tests with ≥5 failing (AC-N1 may pass against the stub since its expected answer *is* the terminal fallback — that is expected and is not evidence of a working resolver), and the `--test` binary reports exactly 4 tests, all failing, because every config variant produces identical output today.
- Fixture note: AC-1, AC-6, AC-9 and AC-N4 each need a `RegionMapIR` carrying **two `entries` rows whose `RegionKey`s differ only in `variant_chain`** (and, for AC-6, in `object_id`), pointing at **genuinely different `ResolvedConfig` values**. `intern_config` is a linear-scan dedup on `ResolvedConfig`'s `PartialEq`: interning two *equal* configs returns the same `ConfigId`, and the fixture then cannot distinguish a correct resolver from a BASE-reading one. Call `intern_config` once per distinct config and insert one `entries` row per `RegionKey`; assert in the fixture helper that the two ids differ if you want the guarantee to be mechanical.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/tests/paint_segmentation_mmu_partition_tdd.rs` - the header, `build_region_map`, `run_paint_segmentation` - purpose: the gated preamble and fixture shape for the integration binary
  - `crates/slicer-ir/src/slice_ir.rs` - `RegionMapIR`, `RegionPlan`, `ConfigId`, `RegionKey`, `ConfigValue`, `intern_config`, `config_for` - locate by symbol
  - `crates/slicer-core/src/algos/paint_segmentation/mod.rs` - three symbol-located windows ONLY: the `match region_map.configs.first()` block (the behaviour being falsified), the Phase 6/7 `chain_key` / `region.variant_chain == chain_key` filter (the `variant_chain` shape the fixtures must reproduce), and `mod driver_v2_tests`' `empty_region_map()` / `region_map_with_base_entry()` fixtures (shape to copy for the in-crate module)
- Files allowed to edit (at most 3):
  - `crates/slicer-core/tests/paint_segmentation_per_region_shell_config_tdd.rs` (new)
  - `crates/slicer-core/src/algos/paint_segmentation/mod.rs` — **restricted**: only (a) the five stub items at module scope and (b) the new `#[cfg(test)] mod shell_config_resolver_tests` at the tail. `execute_paint_segmentation`'s body, `mod driver_v2_tests`, and every other item in the file are out of bounds in this step.
- Files explicitly out of bounds:
  - `crates/slicer-core/src/**` other than the two restricted regions above (no behavioural production code in a red step)
  - `crates/slicer-core/src/algos/paint_segmentation/mod.rs`'s `#[cfg(test)] mod driver_v2_tests` — AC-7's subject; read-only for the whole packet
  - `crates/slicer-core/Cargo.toml` (no `[[test]]` entry is added; the sibling file has none either)
  - `crates/slicer-ir/**`, `crates/slicer-runtime/**`
- Blast-radius discipline: not applicable — no struct field and no schema/version constant is added. The tests construct `ResolvedConfig` values through `..ResolvedConfig::default()` rest syntax, never exhaustive literals, per the struct-literal churn rules.
- Expected sub-agent dispatches:
  - Question: does `crates/slicer-core/Cargo.toml` declare a `[[test]]` target for `paint_segmentation_mmu_partition_tdd`, and does that file gate itself with an inner `#![cfg(feature = "host-algos")]`?; scope: `crates/slicer-core/Cargo.toml`, the test file's first 15 lines; return: `FACT` (≤5 lines)
  - Question: do any of the ten new test-function names already exist anywhere under `crates/slicer-core/`?; scope: `rg -n '<name>' crates/slicer-core/`; return: `FACT` pass/fail — a collision would silently widen the bare `--lib` filters
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md` §"IR 4 — RegionMapIR" → "Config Interner Contract (Normative — Packet 91)" - direct ranged read
  - `CLAUDE.md` §"Feature-gated test files report green when they don't compile" - direct read
- OrcaSlicer refs:
  - none for this step
- Verification:
  - `mkdir -p target && cargo test -p slicer-core --features host-algos --test paint_segmentation_per_region_shell_config_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'; grep -qE '^test result: FAILED\. 0 passed; 4 failed' target/test-output.log && echo RED-OK-INTEGRATION` - FACT: `RED-OK-INTEGRATION` (exactly 4 tests, all red)
  - `mkdir -p target && cargo test -p slicer-core --features host-algos --lib shell_config_resolver_tests 2>&1 | tee target/test-output.log | grep -E '^test result'; grep -qE '^test result: .*; [1-9][0-9]* failed' target/test-output.log && echo RED-OK-LIB` - FACT: `RED-OK-LIB`, and the `test result:` line must show `6` total (`passed + failed == 6`)
  - `cargo check -p slicer-core --features host-algos --all-targets` - FACT pass/fail: the stubs must make the crate compile; a compile error here means the step is unbuildable, not red
- Exit condition: **4 tests in the integration binary, all failing**, and **6 tests under the `--lib` filter with at least 5 failing** (AC-N1 may pass against the stub — it asserts the terminal fallback, which is what the stub returns; that is expected and proves nothing on its own). A total of `0` on either side means the feature flag, the inner `cfg`, or the `--lib` gate is wrong — treat that as a failure of this step, never as a pass. Reconcile any count that is not exactly 4 / exactly 6 before proceeding.

### Step 2: Re-key `painted_subsets` and add the resolver

- Task IDs: `TASK-323`
- Objective: Re-key `painted_subsets` to `(ObjectId, sem_name, PaintValue)`; replace Step 1's five **stub** bodies with the real implementations of `ShellParams`, `region_key_for_chain`, `ext_abs_mm`, `shell_params_from_config`, `resolve_shell_params` (signatures unchanged from Step 1); delete the `match region_map.configs.first()` expression, its `None => (3, 3, 0.4, 0.2)` arm and the `TODO:` comment; **hoist the Phase-6/7 `chain_key` binding** above the `propagate_top_bottom` call and call `resolve_shell_params(&region_map, object_id, &chain_key)` once per subset, passing its four fields to `propagate_top_bottom`. The resolver's ladder is painted chain → the object's BASE chain (`&[]`) → `ResolvedConfig::default()`; both lookup tiers go through `region_key_for_chain`. Repair the `RoleWidthContext` in the same edit, per `design.md` §Code Change Surface items 1-3.
- **Which `chain_key` to hoist — read this before editing.** The function contains **three** `chain_key`-family bindings, and only one of them is the target:
  - `base_chain_key` (empty `Vec`) in Phase 4's BASE-emission block — scans `region_map.entries` on `rk.global_layer_index == global_layer_index && rk.variant_chain == base_chain_key`. **Do not touch.**
  - the Phase-4 `chain_key`, built from the per-semantic `sem_name` and a `polys_by_color` colour — scans `region_map.entries` on `rk.global_layer_index == global_layer_index && rk.variant_chain == chain_key` to build `matching_keys`. **Do not touch, and do not reuse it here**: it lives in a different loop and is pinned to one layer.
  - the **Phase-6/7 `chain_key`**, built from the `painted_subsets` loop's own `sname` / `value`, sitting *between* the `propagate_top_bottom` call and the `for (l, polys) in phase6.per_layer.iter().enumerate()` loop. It filters **SliceIR regions** (`region.variant_chain == chain_key`, `.position(|r| r.variant_chain == chain_key)`, `variant_chain: chain_key.clone()`) and never touches `region_map`. **This is the hoist target.** Move it above the `propagate_top_bottom` call; it depends only on `sname` / `value`, so the move is trivially valid.

  Do **not** add a fourth binding inside the Phase-6/7 loop — the resolver's chain and the region filter's chain must be the same value or they can drift. Note that `region_key_for_chain` is a *new* scan over `region_map.entries`, not a reuse of the Phase-4 one: it drops the `global_layer_index` equality, adds an `object_id` equality, and reduces with `min_by_key`.
- Precondition: Step 1's exits hold — 4 integration tests all red, 6 `--lib` tests with ≥5 red, crate compiling on the stubs.
- Postcondition: all ten pass (4 + 6); packet 128's six shell-index invariants pass; `--check` clean.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/src/algos/paint_segmentation/mod.rs` - symbol-located windows ONLY: the `painted_subsets` declaration, both `entry(key)` accumulation arms, the `match region_map.configs.first()` block, the Phase 6/7 loop with its `chain_key` binding, its `region.variant_chain == chain_key` filter and its `propagate_top_bottom` call, Step 1's five stub bodies, the Phase-4 `matching_base` / `matching_keys` scans (read-only — to identify which `chain_key` is which, and as the `entries`-filter idiom to widen), and (read-only) the shell-index propagation block plus `assert_per_object_shell_index_invariant`
  - `crates/slicer-runtime/src/slice_postprocess_prepass.rs` - `resolve_shell_counts` only - purpose: the `config_for` + `(3, 3)`-fallback **shape** to mirror. Do NOT copy its empty `variant_chain`; this resolver keys on the painted chain (see `design.md` §Locked Assumptions "Granularity lock")
  - `modules/core-modules/classic-perimeters/src/lib.rs` - the `width_context` construction only - purpose: the **PRIMARY and correct** `RoleWidthContext` exemplar. It reads `bridge_line_width`, `initial_layer_line_width`, `outer_wall_line_width` and `inner_wall_line_width` with `get_abs_value(key, nozzle_diameter)`, which is exactly the percent-aware behaviour `ext_abs_mm` must mirror. Copy this one
  - `modules/core-modules/arachne-perimeters/src/lib.rs` - the `width_context` construction only - purpose: **SECONDARY, field-coverage reference ONLY.** It reads the same fields with `get_float`, which silently drops `Percent` and percent-form `FloatOrPercent` values — the exact pattern AC-N3 exists to catch. Do NOT copy its access pattern
  - `crates/slicer-core/src/flow.rs` - `RoleWidthContext`'s fields and `resolve_role_width`'s branch order
  - `crates/slicer-ir/src/slice_ir.rs` - `ConfigView::get_abs_value` only - purpose: the normative clause list `ext_abs_mm` mirrors
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/algos/paint_segmentation/mod.rs`
- Files explicitly out of bounds:
  - `crates/slicer-core/src/algos/paint_segmentation/top_bottom.rs` (shell math is out of scope; its signature does not change)
  - `crates/slicer-runtime/src/slice_postprocess_prepass.rs`, `crates/slicer-runtime/src/run.rs`
  - `crates/slicer-ir/**` — out of bounds for **editing**; no typed field is added to `ResolvedConfig`. The `get_abs_value` read listed above is read-only
  - the integration test file from Step 1 (do not adjust tests to fit the implementation)
  - **`crates/slicer-core/src/algos/paint_segmentation/mod.rs`'s two `#[cfg(test)]` modules — `driver_v2_tests` AND the `shell_config_resolver_tests` written in Step 1.** This step edits that file, so the prohibition is region-scoped rather than file-scoped: production items and `execute_paint_segmentation`'s body are in bounds, everything from the first `#[cfg(test)]` attribute to end-of-file is not. If a Step-1 unit test looks wrong, stop and re-diagnose — do not edit the assertion into agreement with the implementation
- Blast-radius discipline: this step changes the **key type** of `painted_subsets`. Dispatch the `LOCATIONS` inventory below before editing and update every site in this same step; the expected inventory as authored is the declaration, two `entry(key)` calls in the accumulation arms, the `is_empty()` guard, and the Phase 6/7 `for ((sname, value), (semantic, painted_mesh, source_objects))` destructuring. No struct field and no schema/version constant is added, so there is no external struct-literal fallout.
- Expected sub-agent dispatches:
  - Question: list every read of `painted_subsets` (declaration, `entry(` calls, `is_empty`, iteration) in `crates/slicer-core/src/algos/paint_segmentation/mod.rs`; scope: that file; return: `LOCATIONS` (≤20 entries)
  - Question: `cargo test -p slicer-core --features host-algos shell_index` — test-result line and pass count (must be exactly `6 passed`); scope: cargo; return: `FACT` (≤5 lines)
  - Question: `cargo xtask build-guests --check` — any `STALE:` line?; scope: cargo; return: `FACT` pass/fail
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md` §"Config Interner Contract (Normative — Packet 91)" - direct ranged read; `config_for` is the only supported read path
  - `docs/08_coordinate_system.md` - direct read; every resolved value stays millimetres
  - `CLAUDE.md` §"Guest WASM Staleness" - direct read
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/MultiMaterialSegmentation.cpp` - `layer_color_stat` inside `multi_material_segmentation_by_painting`; delegate, never load
- Verification:
  - `mkdir -p target && cargo test -p slicer-core --features host-algos --test paint_segmentation_per_region_shell_config_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'; grep -qE '^test result: ok\. 4 passed; 0 failed' target/test-output.log && echo PASS` - FACT: `PASS` (AC-2, AC-6, AC-9, AC-N2)
  - `mkdir -p target && cargo test -p slicer-core --features host-algos --lib shell_config_resolver_tests 2>&1 | tee target/test-output.log | grep -E '^test result'; grep -qE '^test result: ok\. 6 passed; 0 failed' target/test-output.log && echo PASS` - FACT: `PASS` (AC-1, AC-4, AC-5, AC-N1, AC-N3, AC-N4). Dropping `--features host-algos` here prints `ok` with **zero** tests, because `pub mod algos` is feature-gated in `crates/slicer-core/src/lib.rs`
  - `mkdir -p target && cargo test -p slicer-core --features host-algos shell_index 2>&1 | tee target/test-output.log | grep -E '^test result'; grep -qE '^test result: ok\. 6 passed; 0 failed' target/test-output.log && echo PASS` - FACT: `PASS` (AC-7)
  - AC-3's grep command from `packet.spec.md` - FACT: `PASS`
  - `cargo xtask build-guests --check` - FACT: clean, or the list of stale guests
- Exit condition: 4 integration tests and 6 `--lib` tests pass (exact counts — a lower number means a test failed to compile into its binary, never that the run was clean), all six packet-128 invariants pass, AC-3's static check passes, and `--check` is clean. If any packet-128 test fails, the `source_objects` singleton assumption is wrong — stop and re-diagnose; do not relax the invariant or its assert.

### Step 3: Regression sweep on the painted fixtures

- Task IDs: `TASK-323`
- Objective: Prove the change is a no-op for single-object default-config fixtures and that the paint-channel consumer paths — including packet 206's seam writer — are unregressed. No source edits unless a regression is found, in which case the fix lands here.
- Precondition: Step 2's exits hold.
- Postcondition: the MMU partition suite, the `cube_4color` executor tests, the paint-channel consumer tests and the `cube_4color_modifier_part` e2e all report `0 failed`.
- Files allowed to read, with ranges when over 300 lines:
  - `target/test-output.log` - the failing-test sections only, if any command fails
  - `crates/slicer-core/src/algos/paint_segmentation/mod.rs` - only the windows already listed in Step 2, and only if a regression is being diagnosed
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/algos/paint_segmentation/mod.rs` (only if a regression is found)
- Files explicitly out of bounds:
  - every fixture and every existing test file — a regression is fixed in production code, never by adjusting the assertion
  - `crates/slicer-core/tests/paint_segmentation_per_region_shell_config_tdd.rs`
  - the two `#[cfg(test)]` modules in `crates/slicer-core/src/algos/paint_segmentation/mod.rs` (`driver_v2_tests`, `shell_config_resolver_tests`) — the file is editable here only for a production-code regression fix
- Blast-radius discipline: not applicable.
- Expected sub-agent dispatches:
  - Question: `cargo test -p slicer-core --features host-algos --test paint_segmentation_mmu_partition_tdd` — test-result line and pass count; scope: cargo; return: `FACT` (≤5 lines)
  - Question: `cargo test -p slicer-runtime --test executor cube_4color` and `... --test executor paint_channel` — test-result lines; scope: cargo; return: `FACT` (≤5 lines each)
  - Question: `cargo test -p slicer-runtime --test e2e cube_4color_modifier_part` — test-result line; scope: cargo; return: `FACT` (≤5 lines)
- Context cost: `S`
- Authoritative docs:
  - `CLAUDE.md` §"Test output must always tee to `target/test-output.log`" - direct read; failures are read from the log, never re-run for more output
- OrcaSlicer refs:
  - none for this step
- Verification:
  - `mkdir -p target && cargo test -p slicer-core --features host-algos --test paint_segmentation_mmu_partition_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'; grep -qE '^test result: ok\. [1-9][0-9]* passed; 0 failed' target/test-output.log && echo PASS` - FACT: `PASS`
  - `mkdir -p target && cargo test -p slicer-runtime --test executor cube_4color 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT: expect `0 failed`
  - `mkdir -p target && cargo test -p slicer-runtime --test executor paint_channel 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT: expect `0 failed`
  - `mkdir -p target && cargo test -p slicer-runtime --test e2e cube_4color_modifier_part 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT: expect `0 failed`
- Exit condition: all four commands report `0 failed`. A failure on a single-object fixture means the re-key was not the no-op it should be for one object — diagnose that before proceeding; it is not an expected consequence of this packet.

### Step 4: Doc and deviation-row closure

- Task IDs: `TASK-323`
- Objective: Flip DEV-122 to `Closed — packet 207 …` naming the per-`variant_chain` (per painted semantic, per object) resolution, the `painted_subsets` re-key, the `RoleWidthContext` repair, the deleted dead `None` arm and the two-tier fallback ladder; add the interner-contract sharpening bullet to `docs/02_ir_schemas.md`; regenerate the generated views with `cargo xtask check-deviations` — note this one command regenerates **both** doc 07's Open Deviation Map and doc 15's generated tables, because the arm runs `check_deviations::run` then `gen_config_docs::run`.
- Precondition: Steps 1-3 all green.
- Postcondition: AC-8 passes and `cargo xtask check-deviations --check` exits 0.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/DEVIATION_LOG.md` - the DEV-122 row only; **delegate**, do not load the file
  - `docs/02_ir_schemas.md` - §"IR 4 — RegionMapIR" → "Config Interner Contract (Normative — Packet 91)" only
- Files allowed to edit (at most 3):
  - `docs/DEVIATION_LOG.md`
  - `docs/02_ir_schemas.md`
- Files explicitly out of bounds:
  - `docs/07_implementation_status.md` Open Deviation Map - **generated**; regenerate with `cargo xtask check-deviations`, never hand-edit
  - `docs/15_config_keys_reference.md` - **generated**; never hand-edit. It is regenerated by `cargo xtask check-deviations` too (that arm chains `gen_config_docs::run` after `check_deviations::run`), so its drift is **inside** AC-8's gate. This packet adds no config key, so it should be untouched — but if `check-deviations --check` reports doc-15 staleness, run `cargo xtask check-deviations` (no flag) here and commit the regenerated tables; do not treat it as pre-existing drift outside this packet
  - all production and test source
- Blast-radius discipline: not applicable.
- Expected sub-agent dispatches:
  - Question: return the current Status cell of DEV-122 verbatim; scope: `docs/DEVIATION_LOG.md`; return: `SNIPPETS` (1, the Status cell only)
  - Question: `cargo xtask check-deviations --check` — exit code?; scope: cargo; return: `FACT` pass/fail
- Context cost: `S`
- Authoritative docs:
  - `docs/DEVIATION_LOG.md` header - direct read of the "Single source of truth" note; it forbids hand-editing the generated views
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/MultiMaterialSegmentation.cpp` - delegate a `SUMMARY` confirming canonical reads shell counts and width per `PrintRegion` with no default-substitution path, for the closure note's parity sentence
- Verification:
  - AC-8's python + `check-deviations --check` command from `packet.spec.md` - FACT: `PASS`
  - `rg -qU 'never\s+a\s+production\s+config\s+source' docs/02_ir_schemas.md && rg -q 'slice_has_paint' docs/02_ir_schemas.md && echo PASS` - FACT: `PASS`. **`-U` is required**: the Packet-91 contract bullets in that file are hard-wrapped at ~70 columns, so a line-scoped match on the 33-character anchor fails whenever the new bullet is wrapped the same way as its neighbours — a false FAIL on a correct edit. `\s+` absorbs the newline plus the continuation indent
- Exit condition: AC-8 returns `PASS`, the doc-02 anchors resolve, and no line of `docs/07_implementation_status.md` or `docs/15_config_keys_reference.md` was edited by hand (regenerated lines are fine). **`cargo xtask check-deviations --check` validates doc 07 AND doc 15** — its `USAGE` text in `xtask/src/main.rs` reads "Exit 1 if doc 07 or doc 15 generated sections are stale", and the arm calls `check_deviations::run` then `gen_config_docs::run`. If it exits nonzero on doc-15 content, that is an AC-8 failure this step must clear by regenerating, not a drift to defer.

### Step 5: Packet gates

- Task IDs: `TASK-323`
- Objective: Run the closure gates and re-dispatch every AC; no source edits.
- Precondition: Steps 1-4 all green.
- Postcondition: `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo xtask build-guests --check` all pass; every pipe-suffixed AC returns `PASS`.
- Files allowed to read, with ranges when over 300 lines:
  - `.ralph/specs/207-paint-segmentation-per-region-shell-config/packet.spec.md` - the AC list
- Files allowed to edit (at most 3):
  - `.ralph/specs/207-paint-segmentation-per-region-shell-config/packet.spec.md` (status transition only, at closure)
- Files explicitly out of bounds:
  - all production and test source
  - `.ralph/specs/206-seam-paint-delivery/**` - another packet's directory
- Blast-radius discipline: not applicable.
- Expected sub-agent dispatches:
  - Question: `cargo clippy --workspace --all-targets -- -D warnings` — pass/fail; scope: cargo; return: `FACT` pass/fail with ≤20 lines on failure
  - Question: `cargo check --workspace --all-targets` — pass/fail; scope: cargo; return: `FACT` pass/fail with ≤20 lines on failure
- Context cost: `S`
- Authoritative docs:
  - `CLAUDE.md` §"Test Discipline" - direct read; governs whether the workspace suite runs and how it is dispatched
- OrcaSlicer refs:
  - none for this step
- Verification:
  - `cargo check --workspace --all-targets` - FACT pass/fail
  - `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail
  - `cargo xtask build-guests --check` - FACT: clean
- Exit condition: all three gates green and all nine positive (AC-1 … AC-9) plus four negative (AC-N1 … AC-N4) ACs re-dispatched `PASS`.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | Ten red tests in two homes (4 integration + 6 in-crate) plus five placeholder stubs so the crate compiles; fixture construction for the two-chain, multi-object and percent-width cases is the bulk |
| Step 2 | M | Re-key + per-chain resolver + `RoleWidthContext`; must land atomically |
| Step 3 | S | Regression sweep; read-only unless a regression appears |
| Step 4 | S | Docs and the DEV-122 row |
| Step 5 | S | Gates only |

Split before activation if aggregate cost exceeds M or any step is L. No step is rated L.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS, each run with `--features host-algos` and a nonzero-pass assertion where it targets `slicer-core`.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read: append the `TASK-323` entry naming packet 207, the DEV-122 closure, and the explicit per-`variant_chain` (per painted semantic, per object) granularity boundary. `TASK-323` does not exist in the backlog today — the dispatch adds it. Do not hand-edit the generated Open Deviation Map in the same file.
- Reconcile reopened/superseded status transitions: none. Packet 128 / `TASK-253` is neither reopened nor superseded — this packet extends its per-object keying from the shell-index accumulator to the config the shell math reads, and AC-7 proves its invariants survive. Note that `TASK-253`'s ledger checkbox in `docs/07_implementation_status.md` is currently unchecked even though its archived spec is marked `implemented`; do not silently flip it as a side effect of this packet.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC (AC-1 … AC-9, AC-N1 … AC-N4) and the three packet-level gate commands.
- Record remaining packet-local risk: painted-model output moves for every user whose shell/width/layer-height settings — or `paint_config:<semantic>:*` overlays — differ from the placeholder; multi-object scenes now run one `propagate_top_bottom` pass per object, and no fixture exercises the case where two objects' painted projections overlap in XY.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
