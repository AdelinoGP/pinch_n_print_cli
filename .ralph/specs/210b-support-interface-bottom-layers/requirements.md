# Requirements: 210b-support-interface-bottom-layers

## Packet Metadata

- Grouped task IDs: `TASK-327` (**revived** — it was absorbed into `TASK-326` by the 2026-08-07 merge and is restored by the 2026-08-07 re-split; re-derive that the slot is still free at the moment you register it)
- Paired packet: `210a-support-planner-coord-t` carries `TASK-326` and `DEV-128`. This packet depends on it being **implemented and merged**.
- Deviations owned: `DEV-129` (closed here), plus **one new `Open` row** filed for the two permanent divergences the bottom-band approximation introduces.
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Provenance

The history of this slice is non-linear. It is recorded here in full so that the directory names are not mistaken for duplication and so that the fate of packet 211 stays traceable:

1. Packets `210-support-planner-coord-t` (DEV-128) and `211-support-interface-bottom-layers` (DEV-129, TASK-327) were authored separately.
2. **2026-08-07, user decision — merge.** Both rewrote `smooth_branches` (`modules/core-modules/support-planner/src/lib.rs`) and neither planned for the other's edit: 210 retyped the inlined sub-chain gap walk to an integer squared-unit comparison, 211 extracted that same walk into `split_column_into_chains`. Applied in either order the second edit deletes or duplicates the first. `211` was marked `status: superseded` with `superseded_by: 210-support-planner-coord-t`, and `TASK-327` was folded into `TASK-326`.
3. **The merged packet was reviewed and ruled `SIZE: must decompose`.**
4. **2026-08-07, user decision — re-split.** The merged packet became `210a` (DEV-128, the migration plus the `split_column_into_chains` extraction, TASK-326) and **this packet**, `210b` (DEV-129, the bottom-interface bands). **`TASK-327` is revived and belongs here again.**

**`.ralph/specs/211-support-interface-bottom-layers/` stays `status: superseded` and is neither revived nor deleted.** Its `superseded_by` reads `210a-support-planner-coord-t + 210b-support-interface-bottom-layers` — it was updated to name both halves of the re-split, which is the truthful record of what happened to *it*. Its work was absorbed into the merge and then re-split into `210a` and this packet; that is the chain, and this paragraph is where it is written down. Do not implement that directory, do not edit it, do not delete it.

**Why the re-split is safe where the original two-packet arrangement was not.** The collision was always confined to one function. `210a` performs the *only* rewrite of `smooth_branches` and ships `split_column_into_chains` as a finished helper; this packet adds a second **caller** and nothing else. The ordering constraint that makes this work — start only after `210a` is merged, and write against post-migration signatures rather than "whichever signature is on disk" — is precisely the defect that made the original packet 211 unmergeable. `packet.spec.md` §Prerequisites and `design.md` §Prerequisites make it an explicit, verifiable precondition with a first-dispatch check.

**Corrections carried forward from 211's preflight that this packet honours:**

1. `densify_bottom_interface` needs a `z` per emitted band layer, which 211's parameter list did not supply. It is taken from **the target entry's own** first `Point3WithWidth.z` — never the landing entry's — because `branch_points_match_entry_layer_z` (AC-17) asserts exactly that invariant. After `210a`, `first_point_xyw` returns no `z` at all, so the read is explicit: `entry.branch_segments.first()?.first()?.z`.
2. `split_column_into_chains` is called here with **no length filter** — short chains must still receive floor bands. The `e - s < 3` and `column.len() < 3` filters live in `smooth_branches` and stay there.
3. 211's plan closed DEV-129 while recording two live divergences only inside DEV-129's closure text. They are filed as a separate `Open` row instead (see §Deviation Ledger Obligations).

## Problem Statement — DEV-129, a registered, parsed, inert config key

`support-planner.toml` declares `support_interface_bottom_layers` under a `# Not yet implemented` comment with `default = -1`, and `run_support_geometry` reads it only to `push_diagnostic` a code-1003 `Warn` (`"…is not yet implemented (config value={…})"`) whenever the value differs from `-1`. No geometry is produced, so PnP's support interfaces are top-only. Canonical implements it: `number_of_support_interface_bottom_layers` (`SupportParameters.hpp`) applies a `< 0 ⇒ use the top count` fallback, `TreeSupportCommon.hpp` turns the result into `support_floor_enable` / `support_floor_layers`, and `TreeSupport::draw_circles` builds real `floor_areas`.

**The design blocker.** `PlannedSupportNode` carries `x`, `y`, `dist_to_top` and `to_buildplate`. There is no `dist_to_bottom` and no record of where a branch lands on model geometry. That absence is structural: `plan_for_object` walks layers top→bottom in one pass, nodes carry no parent pointers, and a chain's landing layer is unknown until the chain terminates — after every one of its nodes has been emitted. A `dist_to_bottom` field therefore cannot be filled during the walk. The resolution is to move the computation out of node space into a post-pass over the already-emitted `SupportPlanEntry` rows, using the per-layer `LayerCollisionCache` still in scope, and to run it **after** `smooth_branches` so the band is centred on the position that will actually be printed.

**The observability blocker, which shapes three criteria.** Two of the pass's behaviours cannot be observed end-to-end:

- The `support_on_build_plate_only` guard. That flag rejects to-model contacts at creation (`if self.support_on_build_plate_only && !to_buildplate { continue }`, **twice** in `plan_for_object`), so under it every surviving chain reaches global layer 0 and is skipped by the `L_end == 0` early return regardless. "No band appears" holds whether the guard exists or not.
- The `found_contact == false` path. The `L_end == 0` early return fires first, so any chain a planner fixture can produce over open plate is skipped before the collision test runs.

The resolution is to make both observable rather than to ship criteria that cannot fail: the guard is lifted into a pure `should_densify_bottom_interface` predicate (AC-12) with a static wiring check (AC-12b), and the two skip paths are asserted by two separate direct unit tests on `densify_bottom_interface` with hand-built fixtures (AC-11, AC-11b).

## In Scope

- Add `support_interface_bottom_layers: i32` to `SupportPlanner`, parsed in `from_config` with the same `Int`/`Float`/default arm shape as the adjacent `support_interface_top_layers`, defaulting to `-1`.
- Add `pub fn resolve_interface_bottom_layers(bottom_layers: i32, top_layers: i32) -> u32` — the port of canonical `number_of_support_interface_bottom_layers`: `let n = if bottom_layers < 0 { top_layers } else { bottom_layers }; n.max(0) as u32`. Public so AC-9 can pin it without a planner run.
- Add `pub fn should_densify_bottom_interface(support_on_build_plate_only: bool, bottom_n: u32, interface_spacing_mm: f32) -> bool` — the extracted call-site guard, `!support_on_build_plate_only && bottom_n > 0 && interface_spacing_mm > 0.0`. Public solely so the guard is falsifiable (AC-12); the call site must call it rather than re-inline the conditions (AC-12b).
- Add a private `densify_bottom_interface` post-pass over the emitted entries, called in `plan_for_object` **after** `smooth_branches(&mut entries_in_order, 100)` under that guard. For each `(object_id, region_id)` column and each sub-chain from `split_column_into_chains` (no length filter): take the chain's lowest-layer entry `L_end`; skip if `L_end == 0`; classify the landing as *on model* iff the entry's smoothed XY lies inside `collision_cache[L_end - 1].collision_polys` (via **`point_in_any_expoly`** — that field is `Vec<ExPolygon>`, so the `ExPolygon`-level helper is the one that type-checks; the ring-level `point_in_polygon_units` does not, and the `&ex.contour.points` workaround compiles but drops hole handling and would put a band inside a model hole); when on model, walk the chain upward for `bottom_n` entries and call `push_interface_scan_lines` on each with that entry's own `z`, the same `bbox_half = radius + branch_distance * 0.5`, `width`, `spacing`, `parity = global_layer_index.rem_euclid(2)` and the avoidance/collision polys from **that layer's own `LayerCollisionCache` entry** — one cache carries both `collision_polys` and `avoidance_polys`, so there is no second cache parameter. Band radius comes from the entry's own emitted `width / 2.0`.
- Thread `collision_cache` (already a `plan_for_object` parameter) into the post-pass; no new plumbing through `run_support_geometry`.
- Delete the code-1003 `push_diagnostic` block and its `interface_bottom_layers` local from `run_support_geometry`, including the `// ── Packet 118 D11 …` banner. `_config` keeps its leading underscore.
- Rewrite `interface_bottom_layers_emits_one_typed_diagnostic` and `interface_bottom_layers_default_emits_no_typed_diagnostic` in `modules/core-modules/support-planner/tests/diagnostics_tdd.rs` to the new contract (zero code-1003 records in both cases, "exactly N" shape preserved at N = 0, observed-code dump retained, the now-unreachable `let d = ibl_diags[0];` binding and its severity/`layer` assertions deleted, comments stating the code is **retired**), and update the file's `//!` header lines describing its AC-6 / AC-N3.
- Add `modules/core-modules/support-planner/tests/interface_bottom_layers_tdd.rs` with the four planner-level differential cases named by AC-10, AC-13, AC-14 and AC-N5, built on the collision-cache fixture pattern used by `unreachable_buildplate_node_pruned` in `tests/to_buildplate_tdd.rs`.
- Add the four in-file `#[cfg(test)]` cases: `resolve_interface_bottom_layers_applies_canonical_fallback`, `should_densify_bottom_interface_guards`, `densify_bottom_interface_skips_chain_with_no_model_footprint_beneath`, `densify_bottom_interface_skips_chain_landing_on_global_layer_zero`.
- Delete the `# Not yet implemented — see docs/specs/support-modules-orca-port.md` comment above `[config.schema.support_interface_bottom_layers]` in `modules/core-modules/support-planner/support-planner.toml`, leaving `type`/`default`/`min`/`max`/`display`/`group` byte-identical.

### Frozen-golden fixtures (owned, not incidental)

Bands are on by default (`-1` resolves to `support_interface_top_layers = 2`), so **drift is expected here**, unlike in `210a` where it was merely possible. Both self-captured golden pairs are in scope for deliberate regeneration:

- `resources/golden/benchy_tree_support_orca_endpoints.txt` + `..._branch_count.txt`, compared by `benchy_orca_parity_within_tolerance` (`modules/core-modules/support-planner/tests/orca_parity_tdd.rs`), regenerated with `SUPPORT_PLANNER_REGEN_GOLDEN=1`.
- `resources/golden/support_regression_wedge_endpoints.txt` + `..._branch_count.txt`, compared by `current_wedge_output_stays_within_self_capture_tolerance` (`crates/slicer-runtime/tests/integration/support_golden_regression_wedge_tdd.rs`), regenerated with `SUPPORT_WEDGE_REGEN_GOLDEN=1`.

Per `CLAUDE.md` §Test Discipline, canonical-correct output wins and fixtures may be re-recorded to match — but **only as the explicit, owned act of Step 4**, with a written justification, and that justification must name the regenerated files by basename in the `DEV-129` closure row, because **AC-N7b clause (c) greps `docs/DEVIATION_LOG.md` for them**. A silent regeneration is a failing criterion, not an invisible one. The tolerance constants (`let tolerance_mm = 0.5_f32;`, `let tolerance_fraction = 0.10_f32;`, and the `0.10, 0.5` argument pair twice in the wedge comparator) are frozen; widening any of them is prohibited. `detects_intentional_branch_count_drift` in the wedge golden file is a self-test of the comparator and must not be touched.

## Out of Scope

- **Everything `210a` owns**: `PlannedSupportNode`'s field types, `prim_mst`, `euclidean_distance`, `aggregate_neighbour_targets`, `clamp_to_avoidance`, `point_in_polygon_units`, and the bodies of `smooth_branches`, `split_column_into_chains` and `point_in_any_expoly`, plus `tests/multi_neighbour_mst_tdd.rs`, `tests/smooth_nodes_tdd.rs`, and `DEV-128`. This packet **calls** `split_column_into_chains` and `point_in_any_expoly` — `point_in_any_expoly` is a **consumed** export, not an untouched neighbour: it is the model-landing test. It does not modify any of them, and it neither calls nor edits the ring-level `point_in_polygon_units`.
- The WIT/IR wire format. `record point3-with-width` (`crates/slicer-schema/wit/deps/types.wit`) and `slicer_ir::Point3WithWidth` keep `x: f32, y: f32` — pinned by `210a`'s AC-N3. No host marshal, no `crates/slicer-wasm-host/` change.
- The **top**-interface band. Its `dist_to_top`-driven densification inside the layer loop is the model the bottom band mirrors; it is read, not modified. The pre-existing asymmetry (top band emitted before smoothing, bottom band after) is left alone — see `design.md` `[FWD-2]`.
- The bottom **gap** (`bottom_gap_height` / `slicing_params.gap_object_support`); PnP has no corresponding key. Canonical's `num_bottom_base_interface_layers` base/interface split; PnP has no base/interface role inside `SupportPlanEntry.branch_segments`.
- Canonical's requirement that the contact surface be classified `stTop`/`stBottom`, and its search across **all** layers below rather than the one immediately beneath. `SupportGeometryView.outlines` carries no surface classification. Filed as a new deviation row, not silently shipped as parity.
- Codes 1001 (`max_branches_per_layer` cap) and 1002 (`node-clamped-out`), and every other assertion in `diagnostics_tdd.rs`.
- Adding any field to `PlannedSupportNode`. The bottom band deliberately does not need one (see §Problem Statement).

## Deviation Ledger Obligations

- `DEV-129` → `Closed`, referencing this packet, **cross-referencing the new row below** so the closure does not become the only home of a live divergence, and naming any regenerated golden basename (AC-N7b clause (c)).
- `DEV-128` is **not** touched here; `210a` closed it.
- **One new `Open` row** for the two permanent divergences the bottom band introduces: (i) footprint tested at `L_end - 1` only vs canonical `TreeSupport::draw_circles`' full downward `stTop`/`stBottom` search; (ii) the cap-truncation false positive (a chain truncated by `max_branches_per_layer` directly above model geometry gets a band canonical would not draw). It produces more interface, never less, and never places geometry inside the model.
- **Do not pre-allocate the ID.** Re-derive it at the moment of writing: `rg -o '^\| DEV-[0-9]{3}' docs/DEVIATION_LOG.md | sort -u | tail -1`, then take the next. Nothing in this packet may quote a `DEV-###` for that row.
- `design.md` `[FWD-2]` may add a further row if confirmed during implementation, under the same re-derivation rule.

## Authoritative Docs

- `docs/15_config_keys_reference.md` — 875 lines; ranged read of the `support_interface_bottom_layers` note only, located by grep.
- `docs/adr/0010-typed-diagnostic-channel.md` — 125 lines; direct read of §Status **and** of the §Context paragraph beginning "All three shipped via packet 118". §Decision is normative and untouched.
- `docs/DEVIATION_LOG.md` — large; grep `DEV-129` and read that row alone.
- `docs/07_implementation_status.md` — 412 lines; delegate a `LOCATIONS` dispatch for the §"Workstream 3 — Benchy parity and missing OrcaSlicer behavior" insertion point and the `TASK-163b-diagnostic` row. Never read in full.
- `docs/08_coordinate_system.md` — 285 lines; direct ranged read of §"SDK Helpers" only, for `mm_to_units` at the band's half-extent and spacing.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/SupportParameters.hpp` — `number_of_support_interface_bottom_layers`: the `< 0 ⇒ use the top count` fallback and the `std::max(0, …)` clamp.
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` — `TreeSupport::draw_circles`' floor-area block (downward contact search, `found_contact == false` path, `!support_on_build_plate_only` guard, "N interface layers above the contact").
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupportCommon.hpp` — `support_bottom_enable` / `support_bottom_height` / `support_floor_enable` / `support_floor_layers`; establishes that "bottom interface" and "floor" are one band.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp` — how floor areas become interface extrusions; confirms scan-line densification is the right analogue.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-9` … `AC-16`, plus the net-new `AC-11b` and `AC-12b`.
  - `AC-9` pins the canonical fallback on all four branches (`-1`, `-5`, `0`, `3`). The `-5` case matters: `< 0` means "same as top", so any negative resolves to the top count rather than clamping to zero.
  - `AC-10`, `AC-13`, `AC-14` and `AC-N5` are planner-level differential — each compares `branch_segments` counts across runs of the same fixture at different settings. Differential is the only cheap observation available, because `SupportPlanEntry` carries no interface/base role marker.
  - **`AC-14` no longer uses a `support_on_build_plate_only = true` baseline.** That baseline could never pass on a correct implementation: the flag rejects to-model contacts at creation, so the model-landing chain the fixture requires is absent from that run entirely and the entry sets differ for unrelated reasons. It is replaced by an in-run absolute anchor (`L_end` equals a mid-chain non-interface entry when `bottom = 0`) plus a `-1` / `3` differential.
  - **`AC-11` and `AC-11b` are two criteria, not one.** The `L_end == 0` early return fires before the collision test, so a chain reaching global layer 0 never reaches the `found_contact == false` branch the previous single criterion claimed to exercise. Both are direct in-file unit tests on `densify_bottom_interface`, because only a synthetic fixture can hold `L_end >= 1` and "no footprint beneath" simultaneously.
  - **`AC-12` tests a pure predicate, not a planner run.** The guard is unobservable end-to-end (see §Problem Statement); extracting it into `should_densify_bottom_interface` is what makes it falsifiable, and `AC-12b` proves the predicate is actually wired at the call site rather than defined and orphaned.
  - `AC-16` proves the stub is gone rather than merely unreachable.
- Whole-packet: `AC-17` (the wedge invariants with bands on by default; `branch_points_match_entry_layer_z` is the enforcement for the per-layer `z` rule), `AC-18` (every test binary in the crate, with a `>= 8` count clause so a silently-skipped binary cannot pass), `AC-19` (the gap walk still exists exactly once and now has two callers — this packet raises `210a`'s required count from 2 to 3; "exactly once" is checked as a `= 1` declaration-line count, because an existence grep plus an `-ge 3` occurrence count is also satisfied by a *duplicated* declaration, which is the drift the criterion exists to forbid).
- Negative: `AC-N4` (zero code-1003 records at the value that used to warn, with both pinned tests rewritten rather than deleted or ignored — **and with two clauses that actually discriminate the rewrite**, since the substantive assertion is green both before and after: the now-unreachable `ibl_diags[0]` indexing must be gone and the file must say `retired`), `AC-N5` (bands never extend below the landing layer), `AC-N6` (guest freshness, gated on the command's exit code so a broken xtask cannot pass vacuously — its earlier `if …; then … else echo ACN6-FAIL; fi` wrapper exited 0 on the failing branch and defeated its own stated intent), `AC-N7b` (both frozen goldens, with tolerance pins and a silent-regeneration check that is one unbroken `&&` chain and inspects the **working tree**, not just committed history — the earlier `;`-severed, `..HEAD`-scoped form passed with a red suite and with a dirty golden alike).
- Cross-packet impact: consumes `210a`'s exports only — `split_column_into_chains`, `point_in_any_expoly`, `first_point_xyw`, `push_interface_scan_lines`. `resolve_interface_bottom_layers`, `should_densify_bottom_interface` and `densify_bottom_interface` have no consumers outside `modules/core-modules/support-planner/`. No other packet in the 206–212 queue touches this module.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `rg -q 'fn split_column_into_chains' modules/core-modules/support-planner/src/lib.rs && rg -q 'fn point_in_any_expoly\(polygons: &\[ExPolygon\], p: Point2\) -> bool' modules/core-modules/support-planner/src/lib.rs` | **Precondition gate** — `210a` is merged, and the model-landing helper has its post-migration `ExPolygon`/`Point2` shape. Run before anything else. | FACT pass/fail |
| `cargo check -p support-planner --all-targets` | The new field, predicates and pass compile including the new test file | FACT pass/fail; SNIPPETS ≤20 lines of the first error on failure |
| `cargo test -p support-planner --lib` | In-file unit tests: `resolve_interface_bottom_layers_applies_canonical_fallback`, `should_densify_bottom_interface_guards`, `densify_bottom_interface_skips_chain_with_no_model_footprint_beneath`, `densify_bottom_interface_skips_chain_landing_on_global_layer_zero`, plus `210a`'s | FACT pass/fail + failing test names |
| `cargo test -p support-planner --test interface_bottom_layers_tdd` | AC-10, AC-13, AC-14, AC-N5 — the planner-level band contract | FACT pass/fail + failing case names |
| `cargo test -p support-planner --test diagnostics_tdd` | AC-N4 — rewritten 1003 contract, codes 1001/1002 unchanged | FACT pass/fail |
| `cargo test -p support-planner --test smooth_nodes_tdd` | Proof the second `split_column_into_chains` caller did not disturb `210a`'s extraction | FACT pass/fail |
| `cargo test -p support-planner --test to_buildplate_tdd` | Contact admission + code-1002 drop behaviour unchanged | FACT pass/fail |
| `cargo test -p support-planner --test orca_parity_tdd` | AC-N7b half | FACT pass/fail + failing case names |
| `cargo test -p slicer-runtime --test integration support_golden_regression_wedge` | AC-N7b half: the second frozen golden pair | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `cargo test -p support-planner` | AC-18 whole-crate sweep; `support-planner`'s `Cargo.toml` has no `[features]` table and no `required-features` targets, so this compiles every test binary (the `CLAUDE.md` silent-zero-test hazard does not apply). Expect **8** binaries: 7 files under `tests/` plus `--lib` | FACT pass/fail + count of `test result: ok` lines |
| `cargo test -p slicer-runtime --test integration support_invariants_wedge` | AC-17 on the real wedge fixture | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `cargo xtask build-guests --check` | AC-N6; `src/**` **and** the manifest are guest inputs | FACT: exit code + reports `STALE:` yes/no |
| `SUPPORT_PLANNER_REGEN_GOLDEN=1 cargo test -p support-planner --test orca_parity_tdd benchy_orca_parity_within_tolerance` | Deliberate regeneration, Step 4 only, with justification | FACT: regenerated counts |
| `SUPPORT_WEDGE_REGEN_GOLDEN=1 cargo test -p slicer-runtime --test integration support_golden_regression_wedge` | Deliberate regeneration, Step 4 only, with justification | FACT: regenerated counts |
| `cargo check --workspace --all-targets` | Closure gate | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Closure gate; `too_many_arguments` on `densify_bottom_interface` is the expected lint | FACT pass/fail + lint names |

## Step Completion Expectations

- **The precondition gate runs first.** If `split_column_into_chains`, `point_in_any_expoly(&[ExPolygon], Point2)`, the integer `first_point_xyw` or the unit-typed `push_interface_scan_lines` is missing or differently shaped, `210a` has not merged (or drifted). Stop and reconcile `design.md` §Prerequisites; do not adapt silently. In particular, if `point_in_any_expoly` no longer excludes points inside `holes` (`210a` AC-N8), stop: this packet's landing test would then classify a hole as model geometry.
- **`smooth_branches` is not reopened.** This packet adds a caller. If the function needs editing, `210a`'s extraction contract was violated — report it rather than fixing it here.
- `densify_bottom_interface` must run after `smooth_branches` in `plan_for_object`. Running it before means the band is centred on the unsmoothed node and the smoother then moves the structural point away from its own floor band. This ordering is load-bearing and asserted indirectly by AC-17.
- The guard must be the `should_densify_bottom_interface` call, not three inline conditions. Re-inlining makes AC-12 untestable and AC-12b red: AC-12b's first clause requires the invocation **inside `plan_for_object`'s body**, which a declaration plus an in-file unit test cannot satisfy (verified on synthetic wired/re-inlined harnesses).
- AC-11 and AC-11b must remain two cases. Merging them removes the only coverage the `found_contact == false` path has.
- `cargo xtask build-guests --check` must be run after the last `src/lib.rs` **or** `support-planner.toml` edit and before AC-17. Both paths are guest inputs, and the check must exit 0, not merely print nothing.
- `TASK-327`'s availability, the `DEV-129` row text, and the next free `DEV-###` are all ledger facts. Re-derive every one at the moment of the Step 5 edit; do not trust any value quoted in this packet.

## Context Discipline Notes

- `modules/core-modules/support-planner/src/lib.rs` is over 2 000 lines. Read it in ranges: the `SupportPlanner` struct + `from_config`, the tail of `plan_for_object` including the top-interface densification block, the `push_interface_scan_lines` helper, and the `#[cfg(test)] mod tests` block are four separate reads. Never open it in full.
- `crates/slicer-runtime/tests/integration/support_invariants_wedge_tdd.rs` is read-only and only through a delegated FACT on the four named tests.
- `crates/slicer-runtime/tests/integration/support_golden_regression_wedge_tdd.rs` is read-only except for regeneration runs; its tolerance constants are frozen.
- `modules/core-modules/support-planner/tests/diagnostics_tdd.rs` is 506 lines. Read only the `//!` header, the two bottom-layers cases, and the fixture helpers they call; the code-1001/1002 cases are out of bounds.
- `modules/core-modules/support-planner/tests/to_buildplate_tdd.rs` is 570 lines and read-only. Read only `unreachable_buildplate_node_pruned` and the `multi_overhang_grid` / `make_layer_plan` helpers — that is the fixture shape the new test file copies.
- `docs/DEVIATION_LOG.md` rows are single-line and very long. Grep for the row and read it alone.
- Resist reading `crates/slicer-ir/src/slice_ir.rs` for `Point2`: its shape is `{ x: i64, y: i64 }` with `from_mm` / `to_mm`, and `mm_to_units(mm: f32) -> i64` / `units_to_mm(units: i64) -> f32` come from `slicer_sdk::coords` via the prelude. That is the whole fact this packet needs.
