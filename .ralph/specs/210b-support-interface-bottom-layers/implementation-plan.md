# Implementation Plan: 210b-support-interface-bottom-layers

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".
- **Step 0 is a gate, not a formality.** This packet is written against `210a-support-planner-coord-t`'s merged output. Verify the four consumed signatures before writing a line of code.
- **`smooth_branches` is not reopened.** `210a` rewrote it once; this packet adds a second caller to `split_column_into_chains` and nothing else.
- Runs as a single swarm session in a fresh context, after `210a` has merged.

## Steps

### Step 0: Verify the `210a` preconditions

- Task IDs: `TASK-327`
- Objective: confirm the four symbols this packet consumes exist with the shapes `design.md` §Prerequisites lists, before any edit. The four are `split_column_into_chains`, **`point_in_any_expoly`** (the model-landing helper — `collision_polys` is `Vec<ExPolygon>`, so this is the one that type-checks; `point_in_polygon_units` is a ring-level primitive this packet does not call), `first_point_xyw` and `push_interface_scan_lines`.
- Precondition: `210a` reported `status: implemented` and its branch is merged; working tree clean; `cargo test -p support-planner` green.
- Postcondition: every row of `design.md` §Prerequisites confirmed, or the packet stops.
- Files allowed to read, with ranges when over 300 lines:
  - none directly — this step is a single delegated `FACT`
- Files allowed to edit (at most 3):
  - `.ralph/specs/210b-support-interface-bottom-layers/design.md` (§Prerequisites only, and only to record a confirmed drift before stopping)
- Files explicitly out of bounds:
  - everything else
- Blast-radius discipline: not applicable.
- Expected sub-agent dispatches:
  - Question: in `modules/core-modules/support-planner/src/lib.rs`, what are the exact signatures of `split_column_into_chains`, `point_in_any_expoly`, `first_point_xyw` and `push_interface_scan_lines`; does `point_in_any_expoly`'s body still exclude points inside `holes`; and does `LayerCollisionCache` carry both `collision_polys` and `avoidance_polys`?; scope: that file; return: `FACT` (≤5 lines)
- Context cost: `S`
- Authoritative docs: none
- OrcaSlicer refs: none for this step
- Verification:
  - `rg -q 'fn split_column_into_chains' modules/core-modules/support-planner/src/lib.rs && rg -q 'fn point_in_any_expoly\(polygons: &\[ExPolygon\], p: Point2\) -> bool' modules/core-modules/support-planner/src/lib.rs && rg -q 'fn first_point_xyw' modules/core-modules/support-planner/src/lib.rs` — FACT pass/fail
- Exit condition: all four signatures match §Prerequisites exactly. **A mismatch stops the packet** — record it in §Prerequisites and report. Do not adapt the plan to whatever is on disk; that is the failure mode that made the original packet 211 unmergeable.

### Step 1: Port the canonical fallback, the guard predicate, and the config field

- Task IDs: `TASK-327`
- Objective: add `pub fn resolve_interface_bottom_layers(bottom_layers: i32, top_layers: i32) -> u32` and `pub fn should_densify_bottom_interface(support_on_build_plate_only: bool, bottom_n: u32, interface_spacing_mm: f32) -> bool` with their in-file tests `resolve_interface_bottom_layers_applies_canonical_fallback` and `should_densify_bottom_interface_guards`, and add `support_interface_bottom_layers: i32` to `SupportPlanner`, parsed in `from_config`. No geometry yet, no stub deletion yet.
- Precondition: Step 0 green.
- Postcondition: `cargo test -p support-planner --lib resolve_interface_bottom_layers` passes all four fallback cases and `--lib should_densify_bottom_interface` passes all five guard cases; the crate compiles; the code-1003 diagnostic still fires exactly as before (its tests are untouched in this step).
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/support-planner/src/lib.rs` — two ranges: the `SupportPlanner` struct + `from_config` block, and the `#[cfg(test)] mod tests` header with `default_planner()`
- Files allowed to edit (at most 3):
  - `modules/core-modules/support-planner/src/lib.rs`
- Files explicitly out of bounds:
  - every `tests/*.rs` in this module (this step touches only the in-file test module)
  - `docs/**`, `crates/**`, `resources/golden/**`, `OrcaSlicerDocumented/**` (delegate)
- Blast-radius discipline (mandatory — this step adds a struct field):
  - `SupportPlanner` gains `support_interface_bottom_layers: i32`. The **complete, grep-verified struct-literal blast radius is two sites, both in `modules/core-modules/support-planner/src/lib.rs`**: `Ok(Self { … })` at the end of `from_config`, and `default_planner()` inside the in-file `#[cfg(test)] mod tests`. `SupportPlanner` is `pub`, but no external test constructs it by literal — all files under `modules/core-modules/support-planner/tests/` go through `SupportPlanner::from_config(&config)`. Both sites are in this step's single edited file; that is why the edit cap is 1.
  - No schema or version constant is bumped, so there is no constant-value test fallout. `support-planner.toml`'s `default = -1` is unchanged (its comment is deleted in Step 3).
- Expected sub-agent dispatches:
  - Question: what exactly does `number_of_support_interface_bottom_layers` return for a negative input, and where is the `std::max(0, …)` clamp applied?; scope: `OrcaSlicerDocumented/src/libslic3r/Support/SupportParameters.hpp`; return: `FACT` (≤5 lines)
- Context cost: `S`
- Authoritative docs:
  - `docs/15_config_keys_reference.md` — the `support_interface_bottom_layers` note paragraph only, located by grep; confirms the key's live default and range before the field is added
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Support/SupportParameters.hpp` — delegate; never load
- Verification:
  - `cargo test -p support-planner --lib resolve_interface_bottom_layers 2>&1 | rg -q 'test result: ok\. [1-9][0-9]* passed; 0 failed'` — FACT pass/fail (AC-9)
  - `cargo test -p support-planner --lib should_densify_bottom_interface 2>&1 | rg -q 'test result: ok\. [1-9][0-9]* passed; 0 failed'` — FACT pass/fail (AC-12)
  - `cargo check -p support-planner --all-targets` — FACT pass/fail
- Exit condition: AC-9 and AC-12 PASS, both struct-literal sites compile, `-5` resolves to the top count rather than to `0`, and `should_densify_bottom_interface` returns `false` for all four disabling inputs including a **negative** spacing. If the canonical dispatch shows a different clamp order, follow canonical and update AC-9's expectations in `packet.spec.md` in the same edit.

### Step 2: Detect the landing and emit the bottom band (RED first)

- Task IDs: `TASK-327`
- Objective: author the four planner-level cases in `modules/core-modules/support-planner/tests/interface_bottom_layers_tdd.rs` plus the two in-file skip-path unit tests, watch them fail, then implement `densify_bottom_interface` and its guarded call after `smooth_branches` in `plan_for_object` until they pass.
- Mechanism, stated because several details are exactly where a plausible wrong choice breaks an AC:
  - Iterate `group_branches_into_columns` → `split_column_into_chains` (`210a`'s helper; **its second caller, completing AC-19**) with **no length filter** — short chains get bands too.
  - Entries are sorted descending by `global_layer_index`, so a range's last index is its lowest layer `L_end`. Skip if `L_end == 0` (AC-11b).
  - Landing test: read that entry's position via `first_point_xyw` and test it against `collision_cache[L_end - 1].collision_polys` with **`point_in_any_expoly`**. That field is `Vec<ExPolygon>`, so the ring-level `point_in_polygon_units(&[Point2], Point2)` does not type-check against it; the workaround that *does* compile — `.iter().any(|ex| point_in_polygon_units(&ex.contour.points, p))` — silently drops hole handling and would classify a landing inside a model hole as "on model", drawing a band into the hole. Use the `ExPolygon`-level helper. Outside ⇒ skip, canonical's `found_contact == false` path (AC-11).
  - Band emission: walk the range upward for `bottom_n` entries; for **each** call `push_interface_scan_lines` with **that entry's own** `z`, read directly from `entry.branch_segments.first()?.first()?.z` (`first_point_xyw` returns no `z`). Using the *landing* entry's `z` for all band layers breaks `branch_points_match_entry_layer_z` (AC-17) immediately.
  - Remaining arguments mirror the top band: `radius = width / 2.0` from the entry's own emitted width, `half_units = mm_to_units(radius + branch_distance_mm * 0.5)`, `spacing_units = mm_to_units(interface_spacing_mm)`, `parity = global_layer_index.rem_euclid(2)`, and `avoidance_polys` / `collision_polys` from **that layer's own** `LayerCollisionCache` entry — one cache, both poly sets; there is no separate avoidance cache parameter.
  - Call site: `let bottom_n = resolve_interface_bottom_layers(…); if should_densify_bottom_interface(self.support_on_build_plate_only, bottom_n, self.tree_support_interface_spacing_mm) { densify_bottom_interface(…); }`. AC-12b greps for the predicate **inside `plan_for_object`'s body**; re-inlining the three conditions fails it.
- Test-authoring notes that are load-bearing:
  - AC-11 and AC-11b are **in-file `#[cfg(test)]` unit tests calling `densify_bottom_interface` directly** with hand-built `entries` / `collision_cache` pairs. The planner cannot produce a chain with `L_end >= 1` and no model footprint beneath — it keeps propagating to layer 0 — so a planner-level fixture would test the `L_end == 0` early return while claiming to test the collision path. Keep them separate: deleting either guard must fail exactly one of the two.
  - AC-14's fixture runs the *same* model-landing fixture three times at `bottom = 0`, `-1` and `3` with `top = 2`, and anchors the `0` run absolutely against a mid-chain non-interface entry of the same chain. It must **not** compare against a `support_on_build_plate_only = true` run: that flag rejects to-model contacts at creation, so the model-landing chain is absent from such a run and the entry sets differ for unrelated reasons.
- Precondition: Step 1 complete and green.
- Postcondition: all four planner cases and both in-file skip-path cases pass; `to_buildplate_tdd` and `smooth_nodes_tdd` remain green; the code-1003 stub is still present (Step 3 retires it).
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/support-planner/tests/to_buildplate_tdd.rs` — `unreachable_buildplate_node_pruned` and the `multi_overhang_grid` / `make_layer_plan` helpers only, located by name — read-only template
  - `modules/core-modules/support-planner/src/lib.rs` — two ranges: the top-interface densification block inside `plan_for_object` (the mirror to follow, including its `bbox_half` and `layer_parity` derivation) and the `push_interface_scan_lines` helper
- Files allowed to edit (at most 3):
  - `modules/core-modules/support-planner/tests/interface_bottom_layers_tdd.rs`
  - `modules/core-modules/support-planner/src/lib.rs`
- Files explicitly out of bounds:
  - `modules/core-modules/support-planner/tests/to_buildplate_tdd.rs` — template only, never edited
  - `modules/core-modules/support-planner/tests/diagnostics_tdd.rs` — Step 3 owns it
  - `modules/core-modules/support-planner/tests/smooth_nodes_tdd.rs` and `tests/multi_neighbour_mst_tdd.rs` — `210a`'s guards; frozen
  - `smooth_branches`, `split_column_into_chains`'s body, `point_in_any_expoly`'s body — call the latter two, do not edit them; `point_in_polygon_units` is neither called nor edited here
  - `resources/golden/**` — Step 4 owns reconciliation
  - `crates/**`, `docs/**`
- Blast-radius discipline: not applicable — no struct field is added here (Step 1 owned that) and no constant's value changes. `densify_bottom_interface` and its call site are additive. The one non-additive touch is adding a second caller to `split_column_into_chains`, whose signature does not change.
- Expected sub-agent dispatches:
  - Question: in `TreeSupport::draw_circles`' floor-area block, (a) what condition triggers a floor band, (b) what happens when no model contact is found below a component, and (c) what disables the block entirely?; scope: `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp`; return: `SUMMARY` (≤200 words, no code)
  - Question: do `support_bottom_enable` and `support_floor_layers` both derive from `number_of_support_interface_bottom_layers`?; scope: `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupportCommon.hpp`; return: `FACT` (≤5 lines)
  - Question: does `cargo test -p support-planner --test interface_bottom_layers_tdd` pass, and which cases fail?; scope: `modules/core-modules/support-planner`; return: `FACT` pass/fail + failing case names
- Context cost: `M`
- Authoritative docs:
  - `docs/08_coordinate_system.md` — §"SDK Helpers" range, for `mm_to_units` at the band's half-extent and spacing
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` — delegate; never load
  - `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupportCommon.hpp` — delegate; never load
- Verification:
  - `cargo test -p support-planner --test interface_bottom_layers_tdd 2>&1 | rg -q 'test result: ok\. [1-9][0-9]* passed; 0 failed'` — FACT pass/fail (AC-10, AC-13, AC-14, AC-N5)
  - `cargo test -p support-planner --lib densify_bottom_interface_skips 2>&1 | rg -q 'test result: ok\. [1-9][0-9]* passed; 0 failed'` — FACT pass/fail (AC-11, AC-11b)
  - `rg -qUP 'fn plan_for_object\((?:(?!\n    \})[\s\S])*?should_densify_bottom_interface\(' modules/core-modules/support-planner/src/lib.rs && rg -qU 'should_densify_bottom_interface\([^)]*\)[^;]*\{' modules/core-modules/support-planner/src/lib.rs && [ "$(rg -c 'should_densify_bottom_interface' modules/core-modules/support-planner/src/lib.rs)" -ge 3 ]` — FACT: AC-12b (first clause is body-scoped to `plan_for_object`; the declaration and the in-file unit test cannot satisfy it)
  - `cargo test -p support-planner --test smooth_nodes_tdd 2>&1 | rg -q 'test result: ok\. [1-9][0-9]* passed; 0 failed'` — FACT pass/fail (the second caller did not disturb the extraction)
  - `[ "$(rg -c '^\s*fn split_column_into_chains' modules/core-modules/support-planner/src/lib.rs)" = "1" ] && [ "$(rg -c 'split_column_into_chains\(' modules/core-modules/support-planner/src/lib.rs)" -ge 3 ]` — FACT: AC-19 complete (exactly one declaration + two callers)
- Exit condition: AC-10, AC-11, AC-11b, AC-12b, AC-13, AC-14, AC-19 and AC-N5 PASS **and** the RED run before implementation was recorded with every case failing. If a case passes before the implementation exists, the fixture is not exercising a model landing — fix the fixture (the layer below `L_end` must carry a `SupportGeometryViewEntry` whose outline covers the chain's XY) rather than accepting the pass.

### Step 3: Retire the code-1003 stub and the manifest comment

- Task IDs: `TASK-327`
- Objective: delete the code-1003 `push_diagnostic` block from `run_support_geometry`, delete the `# Not yet implemented` comment from the manifest, and rewrite the two `diagnostics_tdd.rs` cases to the new contract.
- Precondition: Step 2 green — the geometry exists, so the warning is now false.
- Postcondition: neither `1003` nor `is not yet implemented` appears in `src/lib.rs`; `diagnostics_tdd` passes with the two rewritten cases asserting zero code-1003 records at value 3 and at `-1`/absent, with the code-1001/1002 cases untouched.
- **The rewrite must be discriminating.** Today the file asserts `assert_eq!(ibl_diags.len(), 1, …)` and then binds `let d = ibl_diags[0];` to assert that record's severity and `layer`. Setting the count to 0 alone is *not* enough to prove the rewrite happened, because the file also passes today: AC-N4's substantive assertion is green in both states. Two additional changes make it falsifiable and are required — delete the `ibl_diags[0]` binding and everything that depends on it (unreachable at count 0), and state in the case's comment that code 1003 is **retired**. AC-N4 greps for the absence of `ibl_diags[0]` and the presence of `retired`; both are red today.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/support-planner/tests/diagnostics_tdd.rs` — the `//!` header, `interface_bottom_layers_emits_one_typed_diagnostic`, `interface_bottom_layers_default_emits_no_typed_diagnostic`, and the fixture helpers they call, located by name; the code-1001/1002 cases are out of bounds
  - `modules/core-modules/support-planner/support-planner.toml` — the `[config.schema.support_interface_top_layers]` … `[config.schema.tree_support_interface_spacing_mm]` span only
- Files allowed to edit (at most 3):
  - `modules/core-modules/support-planner/src/lib.rs`
  - `modules/core-modules/support-planner/tests/diagnostics_tdd.rs`
  - `modules/core-modules/support-planner/support-planner.toml`
- Files explicitly out of bounds:
  - `modules/core-modules/support-planner/tests/interface_bottom_layers_tdd.rs` — Step 2 owns it
  - every other module's manifest
  - `docs/**` — Step 5 owns the doc edits
- Blast-radius discipline: not applicable — nothing is added. The deletion's fallout is exactly the two `diagnostics_tdd.rs` cases, both edited in this step, which is why the edit cap is used in full.
- Expected sub-agent dispatches:
  - Question: does `cargo test -p support-planner` pass, and do any binaries other than `diagnostics_tdd` change their result?; scope: `modules/core-modules/support-planner`; return: `FACT` pass/fail + failing test names
- Context cost: `S`
- Authoritative docs:
  - `docs/adr/0010-typed-diagnostic-channel.md` — §Status **and** the §Context paragraph beginning "All three shipped via packet 118", read here to draft both sentences applied in Step 5
- OrcaSlicer refs: none for this step
- Verification:
  - `cargo test -p support-planner --test diagnostics_tdd 2>&1 | rg -q 'test result: ok\. [1-9][0-9]* passed; 0 failed'` — FACT pass/fail
  - `! rg -q 'ibl_diags\[0\]' modules/core-modules/support-planner/tests/diagnostics_tdd.rs && rg -q 'retired' modules/core-modules/support-planner/tests/diagnostics_tdd.rs` — FACT: AC-N4's discriminating clauses
  - `! rg -q '\b1003\b' modules/core-modules/support-planner/src/lib.rs && ! rg -q 'is not yet implemented' modules/core-modules/support-planner/src/lib.rs` — FACT: AC-16
  - `! rg -q 'Not yet implemented' modules/core-modules/support-planner/support-planner.toml` — FACT: AC-15
  - `! rg -q '#\[ignore\]' modules/core-modules/support-planner/tests/diagnostics_tdd.rs` — FACT: neither pinned case was suppressed
- Exit condition: AC-15, AC-16 and AC-N4 PASS. The two rewritten tests must keep their "exactly N code-1003 records" assertion shape with N = 0, keep dumping the observed code list on failure, and keep their original fn names. Deleting either test, `#[ignore]`-ing it, renaming it, or replacing the count assertion with a weaker predicate fails this step.

### Step 4: Rebuild the guest, reconcile the goldens, and clear the whole-packet gates

- Task IDs: `TASK-327`
- Objective: rebuild `support-planner.wasm`, prove the wedge invariants hold with bands enabled by default, reconcile both frozen-golden pairs, run the whole-crate sweep, and clear the workspace gates.
- Precondition: Steps 2 and 3 complete; `cargo test -p support-planner` green.
- Golden reconciliation ladder, in order — stop at the first that applies:
  1. Both suites green against the committed goldens ⇒ done; record "no drift".
  2. A golden comparison exceeds its bound ⇒ the mechanism is already known and must be named explicitly: "bottom-interface bands are on by default (`-1` resolves to `support_interface_top_layers = 2`), so every model-landing chain gains two densified layers." Regenerate that pair, commit it, and **carry the justification and the regenerated file basenames into Step 5's `DEV-129` closure text** — AC-N7b clause (c) greps `docs/DEVIATION_LOG.md` for those basenames. Clause (c) inspects the **working tree** (a merge-base diff taken *without* `..HEAD`, unioned with `git status --porcelain -- resources/golden/`), so it fires as soon as the regenerated file lands on disk — before the commit, not only after. Regeneration is expected in this packet, so plan on the Step 5 deviation-log edit being a prerequisite for AC-N7b, not an afterthought.
- **Prohibited:** widening `let tolerance_mm = 0.5_f32;` or `let tolerance_fraction = 0.10_f32;` in `orca_parity_tdd.rs`, or either `0.10, 0.5` argument pair in the wedge comparator; editing `detects_intentional_branch_count_drift`; regenerating a golden without naming it in the deviation log.
- Postcondition: `cargo xtask build-guests --check` exits 0 with no `STALE:`; `support_invariants_wedge` passes; both golden suites pass with unchanged tolerances; at least 8 `support-planner` test binaries report `ok` with zero failures; `cargo check --workspace --all-targets` and `cargo clippy --workspace --all-targets -- -D warnings` clean.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/integration/support_golden_regression_wedge_tdd.rs` — `current_wedge_output_stays_within_self_capture_tolerance` and the `SUPPORT_WEDGE_REGEN_GOLDEN` gate only, and only if a golden moved
- Files allowed to edit (at most 3):
  - `modules/core-modules/support-planner/src/lib.rs` (lint fixes only — `clippy::too_many_arguments` on `densify_bottom_interface` is the expected one; prefer a context struct over `#[allow]`)
  - `resources/golden/benchy_tree_support_orca_*.txt` (via `SUPPORT_PLANNER_REGEN_GOLDEN=1`, never hand-edited)
  - `resources/golden/support_regression_wedge_*.txt` (via `SUPPORT_WEDGE_REGEN_GOLDEN=1`, never hand-edited)
- Files explicitly out of bounds:
  - `crates/slicer-runtime/tests/**` — the invariant and comparator suites are the oracle; adjusting either to accept the new bands is prohibited
  - `modules/core-modules/support-planner/tests/**` — all test edits are closed by Step 3
  - `target/**`, `modules/core-modules/support-planner/support-planner.wasm` (regenerated, never hand-edited)
- Blast-radius discipline: not applicable.
- Expected sub-agent dispatches:
  - Question: does `cargo xtask build-guests --check` exit 0 and report no `STALE:` line, and after a rebuild does it come back clean?; scope: workspace; return: `FACT` (≤5 lines, include the exit code)
  - Question: do the four named tests in `support_invariants_wedge` pass?; scope: `cargo test -p slicer-runtime --test integration support_invariants_wedge`; return: `FACT` pass/fail + ≤20 lines of the first failure
  - Question: do `benchy_orca_parity_within_tolerance` and `current_wedge_output_stays_within_self_capture_tolerance` pass, and if not what Hausdorff distance and branch counts are reported?; scope: the two suites; return: `FACT` (≤5 lines)
  - Question: how many `test result: ok. N passed; 0 failed` lines does `cargo test -p support-planner` print, and is there any `test result: FAILED` line?; scope: `modules/core-modules/support-planner`; return: `FACT` (≤5 lines)
  - Question: does `cargo clippy --workspace --all-targets -- -D warnings` pass, and which lints fire in `support-planner`?; scope: workspace; return: `FACT` pass/fail + lint names
- Context cost: `S`
- Authoritative docs:
  - `CLAUDE.md` §Test Discipline and §"Guest WASM Staleness" — summarised in `design.md`; no read needed
- OrcaSlicer refs: none for this step
- Verification:
  - `cargo xtask build-guests --check` — FACT: exit 0, no `STALE:` (AC-N6)
  - `cargo test -p slicer-runtime --test integration support_invariants_wedge 2>&1 | rg -q 'test result: ok\. [1-9][0-9]* passed; 0 failed'` — FACT pass/fail (AC-17)
  - `cargo test -p support-planner --test orca_parity_tdd 2>&1 | rg -q 'test result: ok\. [1-9][0-9]* passed; 0 failed'` and `cargo test -p slicer-runtime --test integration support_golden_regression_wedge 2>&1 | rg -q 'test result: ok\. [1-9][0-9]* passed; 0 failed'` — FACT pass/fail (AC-N7b clause (a))
  - `rg -q 'let tolerance_mm = 0\.5_f32;' modules/core-modules/support-planner/tests/orca_parity_tdd.rs && rg -q 'let tolerance_fraction = 0\.10_f32;' modules/core-modules/support-planner/tests/orca_parity_tdd.rs && [ "$(rg -c '^\s+0\.10,$' crates/slicer-runtime/tests/integration/support_golden_regression_wedge_tdd.rs)" = "2" ] && [ "$(rg -c '^\s+0\.5,$' crates/slicer-runtime/tests/integration/support_golden_regression_wedge_tdd.rs)" = "2" ]` — FACT: AC-N7b clause (b)
  - `out=$(cargo test -p support-planner 2>&1); ! printf '%s' "$out" | rg -q '^test result: FAILED' && [ "$(printf '%s' "$out" | rg -c '^test result: ok\. [1-9][0-9]* passed; 0 failed')" -ge 8 ]` — FACT pass/fail (AC-18)
  - `cargo check --workspace --all-targets` and `cargo clippy --workspace --all-targets -- -D warnings` — FACT pass/fail
- Exit condition: AC-17, AC-18, AC-N6 and AC-N7b clauses (a)/(b) PASS, and every regeneration performed is recorded with its file basename, staged for Step 5 so clause (c) can pass. A wedge failure is this packet's bug until `build-guests --check` has been shown clean; the deflections in `CLAUDE.md` §"Guest WASM Staleness" are prohibited.

### Step 5: Close the ledger

- Task IDs: `TASK-327`
- Objective: land every doc edit with re-derived, not quoted, values — close `DEV-129`, **file the new divergence row**, register `TASK-327`, and update the config reference and both stale ADR-0010 paragraphs.
- Precondition: Step 4 green.
- Ledger obligations, in the order they must be performed:
  1. Re-derive the next free deviation ID: `rg -o '^\| DEV-[0-9]{3}' docs/DEVIATION_LOG.md | sort -u | tail -1`, then take the next. **Nothing in this packet quotes that ID; do not trust any number written here or in a prior draft.**
  2. Add that row as **`Open`**, recording the two permanent divergences: (i) PnP tests the model footprint at `L_end - 1` only, where canonical `TreeSupport::draw_circles` searches every layer below for `stTop`/`stBottom` surfaces; (ii) the cap-truncation false positive — a chain truncated by `max_branches_per_layer` directly above model geometry receives a band canonical would not draw (more interface, never less; never inside the model).
  3. `DEV-129` → `Closed`, citing this packet, **cross-referencing the new row from step 2**, and naming any golden basename regenerated in Step 4.
  4. `docs/07_implementation_status.md`: register `TASK-327` under §"Workstream 3 — Benchy parity and missing OrcaSlicer behavior", noting it was folded into `TASK-326` by the 2026-08-07 merge and revived by the same day's re-split; add the code-1003 retirement note to the existing `TASK-163b-diagnostic` row.
  5. `docs/15_config_keys_reference.md`: rewrite the `support_interface_bottom_layers` note to the implemented semantics; drop the dead `docs/specs/_OLD/` pointer and the code-1003 sentence.
  6. `docs/adr/0010-typed-diagnostic-channel.md` §Status: append one sentence that code 1003 is retired and codes 1001/1002 are unaffected.
  7. `docs/adr/0010-typed-diagnostic-channel.md` §Context: the sentence "All three shipped via packet 118 with the codes `1001` / `1002` / `1003` as described in Status above" is now stale and sits in §Context, so the §Status edit does not reach it. Append a clause noting code 1003 was subsequently retired by this packet. **§Decision is not edited** — this is a factual correction to a descriptive paragraph, not an amendment, so no ADR-amendment deviation is required.
- Precondition on `docs/specs/deviation-remediation-206-212-plan.md`: already updated by the split decision (row #5 → `210a`, row #5b → this packet, `depends on: #5`). **Do not edit it again.**
- Files allowed to read, with ranges when over 300 lines:
  - `docs/DEVIATION_LOG.md` — the `DEV-129` row and the last row of the table, located by grep
  - `docs/07_implementation_status.md` — the Workstream 3 heading and the `TASK-163b-diagnostic` row only, located by grep
  - `docs/15_config_keys_reference.md` — the `support_interface_bottom_layers` note paragraph only, located by grep
  - `docs/adr/0010-typed-diagnostic-channel.md` — §Status and the §Context paragraph beginning "All three shipped via packet 118"
- Files allowed to edit (at most 3 per sub-step; split 5a/5b because this step carries four files):
  - **5a (deviations):** `docs/DEVIATION_LOG.md`
  - **5b (backlog + reference + ADR):** `docs/07_implementation_status.md`, `docs/15_config_keys_reference.md`, `docs/adr/0010-typed-diagnostic-channel.md`
- Files explicitly out of bounds:
  - `docs/specs/deviation-remediation-206-212-plan.md` — already reconciled
  - `docs/08_coordinate_system.md`, `docs/02_ir_schemas.md`, `docs/03_wit_and_manifest.md`, `docs/05_module_sdk.md` — no contract changed
  - `.ralph/specs/211-support-interface-bottom-layers/`, `.ralph/specs/210a-support-planner-coord-t/` — never edited from this packet
  - `docs/adr/**` beyond ADR-0010 §Status and §Context — no ADR is created or amended
- Blast-radius discipline: not applicable.
- Expected sub-agent dispatches:
  - Question: what is the current status cell of `DEV-129`, what is the highest `DEV-###` present, and is `TASK-327` still absent from `docs/07_implementation_status.md`?; scope: both files; return: `FACT` (≤5 lines); purpose: re-derive every ledger fact at the moment of edit rather than trusting this packet's text
- Context cost: `S`
- Authoritative docs:
  - `docs/DEVIATION_LOG.md` (`DEV-129`, last row), `docs/07_implementation_status.md` (Workstream 3, `TASK-163b-diagnostic`), `docs/15_config_keys_reference.md` (the note), `docs/adr/0010-typed-diagnostic-channel.md` (§Status, §Context) — all ranged or delegated
- OrcaSlicer refs: none for this step
- Verification:
  - `rg -q '^\| DEV-129 .*Closed' docs/DEVIATION_LOG.md` — FACT pass/fail
  - `rg -q 'L_end - 1' docs/DEVIATION_LOG.md && rg -q 'cap-truncation' docs/DEVIATION_LOG.md` — FACT: the new `Open` row exists with both divergences
  - `rg -q 'TASK-327' docs/07_implementation_status.md` — FACT pass/fail
  - `rg -q 'resolves to .support_interface_top_layers' docs/15_config_keys_reference.md && ! rg -q 'code-.1003' docs/15_config_keys_reference.md` — FACT pass/fail
  - `rg -q '1003.*retired' docs/adr/0010-typed-diagnostic-channel.md` — FACT pass/fail (§Status)
  - `rg -qU 'All three shipped via packet 118[^.]*\.[^#]*retired' docs/adr/0010-typed-diagnostic-channel.md` — FACT pass/fail (§Context)
- Exit condition: every Doc Impact grep in `packet.spec.md` returns PASS, **including the new-deviation-row grep and the §Context grep** — a `DEV-129` closure without a matching `Open` row for the two live divergences fails this step, and so does a §Status-only ADR edit that leaves §Context claiming code 1003 ships. If `TASK-327` turns out to be taken by a parallel packet, take the next free ID, update `packet.spec.md`'s frontmatter and `task-map.md` in the same edit, and report the change — do not double-book.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 0 | S | One delegated FACT; a mismatch stops the packet |
| Step 1 | S | Two `pub fn`s, one field, two struct-literal sites, two in-file tests |
| Step 2 | M | Four-case RED planner suite, two in-file skip-path unit tests, landing detection and band emission |
| Step 3 | S | Three deletions and two discriminating test rewrites |
| Step 4 | S | Verification plus golden reconciliation |
| Step 5 | S | Four ledger files, split 5a/5b |

Aggregate `M`. No individual step is `L`.

## Packet Completion Gate

- Step 0 confirmed the `210a` preconditions; all six steps and exits complete.
- Every pipe-suffixed AC command in `packet.spec.md` returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read; the update registers `TASK-327` (revived) and adds the code-1003 retirement note on `TASK-163b-diagnostic`.
- `DEV-129` is `Closed` and exactly one new `Open` row carries the two permanent divergences. `DEV-128` is untouched — `210a` closed it.
- `.ralph/specs/211-support-interface-bottom-layers/packet.spec.md` remains `status: superseded` and its directory untouched.
- Packet 118's code-1003 work is *retired by implementation*, recorded on `TASK-163b-diagnostic` and in ADR-0010 §Status **and** §Context, rather than by a status flip on packet 118.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC (`AC-9` … `AC-19` including `AC-11b` and `AC-12b`, `AC-N4` … `AC-N7b`) and the three packet-level gate commands.
- Record which frozen goldens were regenerated and the named mechanism for each. A regeneration not named in `docs/DEVIATION_LOG.md` fails AC-N7b clause (c).
- Confirm no tolerance constant moved: `let tolerance_mm = 0.5_f32;` and `let tolerance_fraction = 0.10_f32;` in `orca_parity_tdd.rs`; both `0.10, 0.5` argument pairs in the wedge comparator.
- Confirm both `diagnostics_tdd.rs` cases still exist by name, are not `#[ignore]`d, assert exactly zero code-1003 records, no longer index `ibl_diags[0]`, and say the code is retired.
- Confirm AC-11 and AC-11b are still two separate cases and that `should_densify_bottom_interface` is called at the `plan_for_object` call site rather than re-inlined.
- Confirm `smooth_branches` is byte-unchanged relative to `210a`'s merged state.
- Record remaining packet-local risk: the `L_end - 1` footprint approximation and the cap-truncation false positive, both now carried on their own `Open` deviation row; and the coarseness of the differential segment-count assertions.
- Confirm context stayed at or below 150k, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.
- `cargo test --workspace` is **not** required for this packet's closure. The wedge invariant suite, both golden suites, `cargo test -p support-planner`, and `cargo check/clippy --workspace --all-targets` are the closure bar. Run the full suite only if the user asks — and if you do, use `cargo xtask test --summary --workspace` per `CLAUDE.md`, dispatched to a sub-agent returning `FACT pass/fail`.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
