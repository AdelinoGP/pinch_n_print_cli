# Implementation Plan: 192-infill-linker-anchor-length

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Declare `infill_anchor` / `infill_anchor_max` in the manifest

- Task IDs: `TASK-311`
- Objective: add the two `float_or_percent` anchor keys to `modules/core-modules/infill-linker/infill-linker.toml` with canonical's defaults, bounds and unit; **declare `layer_height` and `line_width` in the same step** so the host's declared-key `ConfigView` filter stops dropping the reads that feed `anchor_base_mm`; and prove the manifest still loads through the real loader.
- Precondition: `modules/core-modules/infill-linker/infill-linker.toml` declares exactly one key, `infill_overlap`; `./target/release/pnp_cli.exe` exists and `module config-schema --module-dir modules/core-modules` emits `{"schema": [...], "schema_version": ...}` where each element is `{"module": <id>, "fields": [...]}`.
- Postcondition: `com.core.infill-linker` reports **five** keys — the pre-existing `infill_overlap`, the two anchor keys, plus `layer_height` and `line_width` declared so the host's `ConfigView::from_declared` filter stops dropping them. `layer_height` copies `classic-perimeters`' block (`float`, default `0.2`, `[0.01, 2.0]`, `unit = "mm"`), the packet-150 dead-read-fix precedent whose manifest comment names this exact mechanism; `line_width` copies `rectilinear-infill`'s block (`float`, default `0.4`, `[0.1, 2.0]`), matching the linker's own `unwrap_or(0.4)` so default-config geometry is unmoved. **Declaring `line_width` nonetheless moves geometry on non-default slices, and that is a disclosed, accepted, filed consequence of this step — not an oversight:** un-deadening the two reads makes `RegionRecord`'s `sparse_spacing_mm` (`line_width / density`) and `solid_spacing_mm` (`line_width`) track the user's value, shifting `remove_short_polylines`' prune threshold, `ExPolygonWithOffset::for_infill_overlap`'s offset and `RegionConfig`'s cross-region `same_config` grouping. It stays in this packet because `anchor_base_mm` is computed from `line_width`, so deferring it would pin the packet's percent base to a constant. See `design.md` §Risks and Tradeoffs for the full chain and the rejected split, and file the dedicated `DEV-###` row in Step 5. `infill_density` is **not** declared — see the packet's Doc Impact row for the fraction-vs-percent conflict between the code and three sibling manifests, and for why declaring it would start honouring a user-set density and move `sparse_spacing_mm` on non-default slices. `infill_anchor` is `float_or_percent` with default `"400%"`, `min = 0.0`, `max = 1000.0`, `unit = "mm"`, `group = "InfillLinker"`; `infill_anchor_max` is `float_or_percent` with default `20.0` and the same bounds/unit/group. Both descriptions name the canonical `ConfigOptionFloatOrPercent` default and record that manifest defaults are never injected into the runtime `ConfigView`, so the module's code fallbacks are what run.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/infill-linker/infill-linker.toml` — whole file; short
  - `modules/core-modules/arachne-perimeters/arachne-perimeters.toml` — the `overhang_reverse_threshold` and `min_width_top_surface` blocks only, as the in-tree `float_or_percent` precedent
  - `crates/slicer-scheduler/src/manifest.rs` — `parse_percent_default` and the `ConfigFieldEntry` declaration only
  - `modules/core-modules/classic-perimeters/classic-perimeters.toml` — the `[config.schema.layer_height]` block and the "Packet 150 Step 6: dead-read fix" comment above `[config.schema.nozzle_diameter]` only; copy the block and mirror the comment's framing
  - `modules/core-modules/rectilinear-infill/rectilinear-infill.toml` — the `[config.schema.line_width]` block only
  - `crates/slicer-ir/src/slice_ir.rs` — `ConfigView::from_declared` only, to see for yourself that an undeclared key is dropped rather than defaulted
- Files allowed to edit (at most 3):
  - `modules/core-modules/infill-linker/infill-linker.toml`
- Files explicitly out of bounds:
  - every other module manifest under `modules/core-modules/`
  - `docs/15_config_keys_reference.md` (regenerated in Step 5, never hand-edited here)
  - `crates/slicer-scheduler/**` (read-only this step)
- Blast-radius discipline: not applicable — no struct field and no schema/version constant is added. `[config.schema.*]` entries are additive and do not move `min-ir-schema` / `max-ir-schema` or any public version constant.
- Expected sub-agent dispatches:
  - Question: does `docs/03_wit_and_manifest.md` restrict the `unit` value set, and is `"mm"` valid for a `float_or_percent` key?; scope: `docs/03_wit_and_manifest.md`; return: `FACT` (≤5 lines)
- Context cost: `S`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` — delegated `FACT` on the `[config.schema.<key>]` field set
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — `PrintConfigDef::init_fff_params`; delegate; never load
- Verification:
  - The AC-1 command (`pnp_cli module config-schema` piped through the key probe) — FACT PASS/FAIL. It must print `FAIL` before the edit and `PASS` after; a `FAIL: infill-linker did not load` result means `parse_percent_default` rejected a default and the manifest is broken, not merely incomplete.
  - The AC-20 command (declared-vs-read guard) — FACT PASS/FAIL. Measured before this step it prints `FAIL: … ['line_width']`; after this step it prints `PASS`, and it will go red again at Step 4 the moment the `layer_height` read is added without its declaration — which is the point.
- Exit condition: AC-1 and AC-20 both print `PASS`. If AC-1 prints `FAIL: infill-linker did not load`, the step is **not** complete — a rejected default takes the whole module out of the schema, which is a louder failure than a missing key. Do **not** silence AC-20 by adding `infill_density` to its allow-list beyond the one entry already there; the allow-list is the deviation row's counterpart, not a mute button.

### Step 2: TDD — author the failing anchor tests

- Task IDs: `TASK-311`
- Objective: create `modules/core-modules/infill-linker/tests/anchor_length_tdd.rs` and add the two orchestrate-level tests, so every change-proving AC has a named, currently-failing driver before any behaviour changes.
- Precondition: `cargo test -p infill-linker --test connect_tdd` reports 6 passed / 0 failed; `anchor_length_tdd.rs` does not exist.
- Postcondition: `anchor_length_tdd.rs` exists with `whole_arc_under_anchor_length_max_merges_into_one_polyline`, `arc_over_anchor_length_max_leaves_two_polylines_each_with_a_stub`, `stub_is_exactly_anchor_length_via_a_lerped_partial_segment`, `shorter_arc_claims_its_endpoints_before_a_longer_arc`, `percent_anchor_resolves_against_flow_spacing_via_get_abs_value`, `zero_anchor_max_dispatches_to_chain_only_never_connect`, `zero_anchor_length_leaves_the_over_max_arc_with_no_stub_at_all`, `stub_is_clamped_at_the_next_boundary_position_and_never_walks_over_it`; `orchestrate_tdd.rs` gains `solid_bucket_forces_unlimited_anchor_while_sparse_obeys_the_key` and `absent_anchor_keys_fall_back_to_four_hundred_percent_of_flow_spacing`. All are written against the **post-change** API and therefore do not compile yet — that is the expected red state for this step. The two `..._flow_spacing` tests must pin `0.3570796` mm as the percent base and `1.4283185` mm as the resolved `anchor_length`, and `absent_anchor_keys_...` must additionally assert that varying `infill_density` alone leaves `anchor_length` unmoved — that assertion is the only one that catches the line-spacing base being reintroduced.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/infill-linker/tests/connect_tdd.rs` — whole file, short; it supplies `square`, `l_shape`, `point`, `segment`, `has_vertex`
  - `modules/core-modules/infill-linker/tests/orchestrate_tdd.rs` — whole file, short; it supplies the `config` / `view` / `run` / `sparse_region` harness
  - `crates/slicer-core/src/flow.rs` — `line_width_to_spacing` only; the percent base the tests must pin
  - `crates/slicer-ir/src/slice_ir.rs` — `ConfigView::from_map` and `ConfigView::get_abs_value` only
- Files allowed to edit (at most 3):
  - `modules/core-modules/infill-linker/tests/anchor_length_tdd.rs` (new)
  - `modules/core-modules/infill-linker/tests/orchestrate_tdd.rs`
- Files explicitly out of bounds:
  - `modules/core-modules/infill-linker/src/**` (no production edit this step)
  - `crates/slicer-sdk/src/test_support/fixtures.rs` — the percent fixture uses `slicer_ir::ConfigView::from_map`, **not** a new builder method; `slicer-sdk` is a universal guest dependency and editing it forces a full guest rebuild
- Blast-radius discipline: not applicable — no struct field or schema constant.
- Expected sub-agent dispatches:
  - Question: what exact ring geometry places two boundary projections 1.0 mm and 8.0 mm apart on a single square ring, with no ring vertex at 2.0 mm from either join?; scope: `modules/core-modules/infill-linker/tests/connect_tdd.rs`; return: `SNIPPETS` (≤2 snippets, ≤30 lines); purpose: AC-6's off-vertex requirement
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0025-infill-linker-as-raw-emit-post-pass.md` §"Regression coverage" — ranged read; names the three containment guards these tests must not weaken
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Fill/FillBase.cpp` — `take_ccw_limited` / `take_cw_limited`, for the exact-length property AC-6 asserts; delegate; never load
- Verification:
  - `cargo check -p infill-linker --all-targets` — expected to **fail** with unresolved `AnchorParams` / `contour_stub`, and the failure must name exactly those symbols. A check that passes means the tests were written against the old API and prove nothing.
- Exit condition: `cargo check -p infill-linker --all-targets` fails, and every named symbol in its error output is one this packet is about to add (`AnchorParams`, `contour_stub`, the new `connect_infill` / `chain_or_connect_infill` arity). Any other unresolved symbol is a test-authoring bug.

### Step 3a: `AnchorParams` + `contour_stub` + the signature change (crate deliberately left non-compiling)

- Task IDs: `TASK-311`
- Objective: add `AnchorParams` to `connect.rs` and `contour_stub` to `graph.rs`, delete `LINK_THRESHOLD_SPACINGS`, and replace `connect_infill` / `chain_or_connect_infill`'s `spacing_mm: f32` parameter with `anchor: AnchorParams`.
- Precondition: Step 2's red state; `LINK_THRESHOLD_SPACINGS` still exists; `connect_infill`'s third parameter is still `spacing_mm: f32`.
- Postcondition: `AnchorParams` exists with `anchor_length_mm` / `anchor_length_max_mm`, `UNLIMITED_MM = 1000.0`, `DONT_CONNECT_MAX_MM = 0.05`, `solid()`, `dont_connect()`, `from_config(Option<&ConfigView>, base_spacing_mm: f32)` resolving through `ConfigView::get_abs_value` with the `4.0 × base_spacing_mm` / `20.0` fallbacks and the `min(anchor_length, anchor_length_max)` clamp. `contour_stub` exists in `graph.rs`, walks in a `RingDirection`, and lerps its terminal point to land exactly at the budget. `LINK_THRESHOLD_SPACINGS` and its dead-citation comment are gone. **The crate does not compile at the end of this step** — `orchestrate.rs` still calls `chain_or_connect_infill` with an `f32` and `connect_tdd.rs` has five stale call sites. That is the declared postcondition, not a failure.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/infill-linker/src/connect.rs` — long; ranged reads only, in two passes: the `connect_infill` / `chain_or_connect_infill` block and the `nearest_pair_candidate` block
  - `modules/core-modules/infill-linker/src/graph.rs` — long; ranged reads only: `BoundaryRing::directed_distance`, `vertices_between`, `contour_connector`, `lerp`
  - `crates/slicer-ir/src/slice_ir.rs` — `ConfigView::get_abs_value` only
  - `crates/slicer-core/src/flow.rs` — `line_width_to_spacing` only
- Files allowed to edit (at most 3):
  - `modules/core-modules/infill-linker/src/connect.rs`
  - `modules/core-modules/infill-linker/src/graph.rs`
- Files explicitly out of bounds:
  - `modules/core-modules/infill-linker/src/orchestrate.rs` and `tests/connect_tdd.rs` — both move in Step 3b
  - `modules/core-modules/infill-linker/src/lib.rs`, `src/offset.rs`
- Blast-radius discipline: `AnchorParams` is a new public struct with no existing literals, so its own blast radius is zero. The **signature** change is the real radius and is discharged in Step 3b; do not attempt it here, because covering it would need four edited files against a three-file limit — which is exactly why this step is split.
- Expected sub-agent dispatches:
  - Question: list every call site of `connect_infill` and `chain_or_connect_infill`; scope: `modules/core-modules/infill-linker/**`; return: `LOCATIONS` (≤20 entries); purpose: hand the exact list to Step 3b
- Context cost: `M`
- Authoritative docs:
  - `docs/08_coordinate_system.md` — delegated `SUMMARY`; confirms `AnchorParams` holds mm and converts once via `mm_to_units`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Fill/FillBase.hpp` — `FillParams`' `1000.f` defaults and `dont_connect()`'s `0.05f`; delegate
  - `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp` — `group_fills`' percent resolution and `std::min` clamp; delegate
  - `OrcaSlicerDocumented/src/libslic3r/Fill/FillBase.cpp` — `take_ccw_limited` / `take_cw_limited`'s lerp; delegate
- Verification:
  - The AC-2 and AC-3 static probes — FACT PASS/FAIL. These are **static greps and do not require a compiling crate**, which is why they are the only gate available at this step.
  - Do **not** run `cargo check`, `cargo test` or `cargo clippy` here; a red result is expected and carries no information.
- Exit condition: AC-2 and AC-3 both print `PASS`, and the `LOCATIONS` dispatch has returned the call-site list Step 3b will consume. Compilation is explicitly **not** an exit criterion for this step.

### Step 3b: move every call site

- Task IDs: `TASK-311`
- Objective: update the `chain_or_connect_infill` call in `orchestrate.rs` and the five `connect_infill` call sites in `connect_tdd.rs` so the crate compiles again, without weakening any containment assertion.
- Precondition: Step 3a complete; `cargo check -p infill-linker --all-targets` is red at exactly those call sites.
- Postcondition: `cargo check -p infill-linker --all-targets` is clean. `connect_tdd.rs`'s five call sites pass `AnchorParams { anchor_length_mm: 0.0, anchor_length_max_mm: <old spacing_mm × 10.0> }` — **10.0** mm for `linked_paths`, `role_and_speed_preserved` and the reflex-corner test, **4.0** mm for the hole-ring test, **50.0** mm for the cross-ring test — reproducing each fixture's original walk budget under the whole-arc branch with stubs off, so the three ADR-0025 containment tests exercise the same geometry as before. `orchestrate.rs`'s call passes a placeholder `AnchorParams::solid()` at this step; Step 4 replaces it with the real per-bucket resolution.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/infill-linker/tests/connect_tdd.rs` — whole file, short
  - `modules/core-modules/infill-linker/src/orchestrate.rs` — `link_paths_without_offset` only
- Files allowed to edit (at most 3):
  - `modules/core-modules/infill-linker/tests/connect_tdd.rs`
  - `modules/core-modules/infill-linker/src/orchestrate.rs`
- Files explicitly out of bounds:
  - `modules/core-modules/infill-linker/src/connect.rs`, `src/graph.rs` — settled in Step 3a
  - `modules/core-modules/infill-linker/src/lib.rs`, `src/offset.rs`
- Blast-radius discipline: consume the `LOCATIONS` list from Step 3a rather than re-deriving it. At authoring time the answer was: one internal call in `chain_or_connect_infill`, one in `orchestrate.rs`'s `link_paths_without_offset`, and **five** in `connect_tdd.rs` (`linked_paths`, `role_and_speed_preserved`, and the three containment tests) — plus doc-comment mentions, which are not call sites.
- Expected sub-agent dispatches:
  - Question: after the edit, does any `connect_infill` or `chain_or_connect_infill` call still pass an `f32` third argument?; scope: `modules/core-modules/infill-linker/**`; return: `FACT` (≤5 lines)
- Context cost: `S`
- Authoritative docs:
  - `docs/adr/0025-infill-linker-as-raw-emit-post-pass.md` §"Regression coverage" — ranged read; names the three guards whose assertions must survive verbatim
- OrcaSlicer refs:
  - none — this step is mechanical call-site movement
- Verification:
  - `cargo check -p infill-linker --all-targets` — FACT pass/fail
  - `bash -c 'cargo test -p infill-linker --test connect_tdd 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: connect_tdd did not run clean"'` — AC-11 do-not-regress; measured green at 6 passed / 0 failed before this step
  - `bash -c 'cargo test -p infill-linker --test anchor_length_tdd -- percent_anchor_resolves_against_flow_spacing_via_get_abs_value --exact 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo FAIL'` — AC-10; the first behavioural test that can run
  - `cargo xtask build-guests --check` — FACT clean/stale (module sources changed in 3a)
- Exit condition: `cargo check -p infill-linker --all-targets` is clean, AC-10 prints `PASS`, and `connect_tdd` still reports 6 passed / 0 failed. If a containment guard went red here, the call-site update changed a fixture's effective budget — fix the `anchor_length_max_mm` value, never the assertion.

### Step 4: The branch, the ordering, and the orchestrate plumbing

- Task IDs: `TASK-311`
- Objective: replace `connect_infill`'s single-gate re-solve loop with the shortest-first, consumed-guarded single pass carrying the whole-arc / stub / nothing branch; add the `dont_connect` fork to `chain_or_connect_infill`; resolve and thread `AnchorParams` per region and per bucket in `orchestrate.rs`, with `PathBucket::Solid` forced unlimited.
- Precondition: Steps 3a and 3b complete; `cargo check -p infill-linker --all-targets` is clean; Step 1 already declared `layer_height` and `line_width`, so the new `layer_height` read this step adds is live rather than dropped; `AnchorParams` and `contour_stub` exist; `nearest_pair_candidate`'s terminal sort still keys on `endpoint_order(first)`.
- Postcondition: `candidates.sort_by` keys on `left.distance.total_cmp(&right.distance)` first with `endpoint_order` tiebreaks; `connect_infill` iterates the sorted list once under a consumed-endpoint guard; a candidate under `mm_to_units(anchor_length_max_mm)` merges (lower index survives), one over it with `anchor_length_mm > 0.0` gets two opposite-direction `contour_stub` runs and stays two paths, and one over it with `anchor_length_mm == 0.0` gets nothing; the stub budget is clamped to the arc reaching the next boundary position in that direction; `chain_or_connect_infill` skips connecting when `AnchorParams::dont_connect()`; `RegionRecord` carries `sparse_anchor`, `RegionConfig` carries the two `u32` bit fields, and a new `anchor(records, index, bucket)` helper returns `AnchorParams::solid()` for `PathBucket::Solid`.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/infill-linker/src/connect.rs` — the `connect_infill`, `chain_or_connect_infill` and `nearest_pair_candidate` blocks
  - `modules/core-modules/infill-linker/src/orchestrate.rs` — lines covering the `RegionConfig` / `RegionRecord` declarations, the `orchestrate_infill` record-construction block, `process_bucket` / `process_bucket_role`'s `same_config` test, `link_paths` / `link_paths_without_offset`, and the `spacing` / `config_float` helpers. **Do not read** `RoleBoundaries`, the wall-grouping helpers, or `majority_owner`.
  - `modules/core-modules/infill-linker/src/graph.rs` — `contour_stub` and `directed_distance` only
- Files allowed to edit (at most 3):
  - `modules/core-modules/infill-linker/src/connect.rs`
  - `modules/core-modules/infill-linker/src/orchestrate.rs`
  - `modules/core-modules/infill-linker/tests/orchestrate_tdd.rs` (only if the two new tests need a fixture adjustment; no assertion may be weakened)
- Files explicitly out of bounds:
  - `modules/core-modules/infill-linker/src/lib.rs` — the anchors are resolved from the per-region `view.config()` in `orchestrate.rs`, exactly as `line_width` / `infill_density` already are; `lib.rs` needs no change
  - `modules/core-modules/infill-linker/src/offset.rs`
  - every host crate
- Blast-radius discipline: `RegionConfig` gains two `u32` fields. Its only struct literal is the `config: RegionConfig { .. }` initialiser inside the `RegionRecord` construction in `orchestrate_infill`; `RegionRecord` gains `sparse_anchor` at the same literal. Dispatch a `LOCATIONS` worker for `RegionConfig {` and `RegionRecord {` across `modules/core-modules/infill-linker/**` **before** editing and cite the count inline; at authoring time it was one literal each, both private to `orchestrate.rs`, neither reachable from a test. Budget that dispatch in this step; do not let a follow-up `cargo check` discover a second literal.
- Expected sub-agent dispatches:
  - Question: how many struct literals of `RegionConfig` and of `RegionRecord` exist?; scope: `modules/core-modules/infill-linker/**`; return: `LOCATIONS` (≤20 entries)
  - Question: in `Fill::connect_infill`, is the arc list built from all same-contour intersection pairs or only from `next_on_contour` neighbours, and does the stub branch mark both endpoints consumed?; scope: `OrcaSlicerDocumented/src/libslic3r/Fill/FillBase.cpp`; return: `SUMMARY` (≤200 words); **only if** the emergent-adjacency argument in `design.md` §Code Change Surface item 3 is challenged
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0026-infill-linking-algorithms-in-linker-module.md` — delegated `SUMMARY`; confirms the algorithm stays in the module
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Fill/FillBase.cpp` — `Fill::connect_infill`'s arc sort, consumed guard, whole-arc and stub branches, and `Fill::chain_or_connect_infill`'s `dont_connect()` fork; delegate; never load
  - `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp` — `group_fills`' `surface.is_solid() || is_bridge` forcing to `1000.f`; delegate
- Verification:
  - `cargo test -p infill-linker --test anchor_length_tdd` — FACT pass/fail; SNIPPETS ≤20 lines on failure (covers AC-4 … AC-7, AC-10, AC-N1 … AC-N3)
  - `cargo test -p infill-linker --test orchestrate_tdd` — FACT pass/fail (covers AC-8, AC-9)
  - `cargo test -p infill-linker --test connect_tdd` — AC-11 do-not-regress; measured green at 6 passed / 0 failed before this step
  - `cargo xtask build-guests --check`, then `cargo test -p slicer-runtime --test integration -- infill_partitioned_input_tdd::` — AC-13 do-not-regress; measured green before this step. **Run the freshness check first**; a stale guest here will look like a containment regression and is not one.
  - `cargo clippy --workspace --all-targets -- -D warnings` — FACT pass/fail
- Exit condition: AC-4 … AC-13 and AC-N1 … AC-N3 all print `PASS`. If `infill_partitioned_input_tdd::` is red **and** `build-guests --check` was clean, the failure is this step's — diagnose it, do not attribute it to staleness or to a separate workstream.

### Step 5: Docs, deviation rows, and provenance corrections

- Task IDs: `TASK-311`
- Objective: regenerate `docs/15`, close and correct the `DEV-089` row, file **every** new `DEV-###` row enumerated in `packet.spec.md` §Doc Impact Statement — including the disclosed `line_width`-declaration behaviour move (see `design.md` §Risks and Tradeoffs) — each with an independently re-derived ID (do not assume they are consecutive, and do not carry a count forward from this line), rewrite ADR-0025's stale section, flip the two `ORCA_CONFIG_REFERENCE` rows, correct the `resolved_config.rs` doc comment, register `TASK-311`, and update the plan's queue row.
- Precondition: Steps 1, 2, 3a, 3b and 4 green. `DEV-089` is `Open`, pins `.../src/graph.rs`, and says "no shortest-first ordering". `docs/07_implementation_status.md` has zero `TASK-311` hits. `crates/slicer-ir/src/resolved_config.rs` still says `OrcaSlicer: infill_anchor_max` on `infill_resolution`. ADR-0025 still carries `### Not yet ported from canonical`.
- Postcondition: every Doc Impact bullet in `packet.spec.md` is satisfied and its verification command prints `PASS`.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/DEVIATION_LOG.md` — **delegated**; the `DEV-089` row and the highest `DEV-###` only
  - `docs/adr/0025-infill-linker-as-raw-emit-post-pass.md` — §"Not yet ported from canonical", §"Regression coverage", §References only
  - `docs/ORCA_CONFIG_REFERENCE.md` — **delegated**; the two `coFloatOrPercent` `infill_anchor*` rows only
  - `crates/slicer-ir/src/resolved_config.rs` — the `infill_resolution` declaration and its doc comment only
  - `docs/specs/deviation-backlog-remediation-plan.md` — the `## Packet Queue` row 10 only
- Files allowed to edit (at most 3 per sub-pass; run this step as three sub-passes so the limit holds):
  - 5a: `docs/DEVIATION_LOG.md`, `docs/adr/0025-infill-linker-as-raw-emit-post-pass.md`, `docs/ORCA_CONFIG_REFERENCE.md`
  - 5b: `crates/slicer-ir/src/resolved_config.rs`, `docs/07_implementation_status.md`, `docs/specs/deviation-backlog-remediation-plan.md`
  - 5c: `docs/15_config_keys_reference.md` — **by generator only** (`cargo xtask gen-config-docs`); never hand-edited
- Files explicitly out of bounds:
  - the generated block between `<!-- BEGIN GENERATED: module-config-keys -->` and `<!-- END GENERATED: module-config-keys -->` in `docs/15_config_keys_reference.md` — regenerate, never hand-edit
  - the generated open-deviations block in `docs/07_implementation_status.md` — regenerate with `cargo xtask check-deviations`; hand-add the `TASK-311` row **outside** it
  - every other packet directory under `.ralph/specs/`
- Blast-radius discipline: the `resolved_config.rs` edit touches a **doc comment only**. Do not change `infill_resolution`'s key name, type or `0.04` default — that would move a live config default and pull in every fixture that depends on it, which is not this packet's scope.
- Expected sub-agent dispatches:
  - Question: what is the highest `DEV-###` in `docs/DEVIATION_LOG.md` **right now**, and what is the exact current text of the `DEV-089` row?; scope: `docs/DEVIATION_LOG.md`; return: `FACT` (≤5 lines). **Re-run this immediately before writing each new row** — sibling packets in this batch are filing rows concurrently and a number captured earlier in the session will collide, which is exactly how a duplicate row reached `main` once already.
  - Question: after `cargo xtask gen-config-docs`, do the two new keys appear in the `module-config-keys` block with `float_or_percent` in the type column?; scope: `docs/15_config_keys_reference.md`; return: `FACT`
- Context cost: `M`
- Authoritative docs:
  - `docs/DEVIATION_LOG.md` — delegated row-level reads only
  - `docs/15_config_keys_reference.md` — generated block boundaries only
  - `docs/07_implementation_status.md` — delegated; hand-add the `TASK-311` row outside the generated block
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Fill/FillBase.cpp` — cite `Fill::connect_infill` / `Fill::chain_or_connect_infill` **by function name only** in every row and ADR sentence written here; the ADR's existing `FillBase.cpp:1497-2300` range is dropped as part of this step
- Verification:
  - The AC-14, AC-15, AC-16, AC-17, AC-18 and AC-19 commands — each FACT PASS/FAIL
  - `bash -c 'rg -q "could_take_prev" docs/DEVIATION_LOG.md && rg -q "trim_next" docs/DEVIATION_LOG.md && echo PASS || echo FAIL'` — the neighbour-trimming residual row exists
  - `bash -c 'rg -q "parse_percent_default" docs/DEVIATION_LOG.md && echo PASS || echo FAIL'` — the percent-transport residual row exists
  - `bash -c 'rg -q "line_width_to_spacing" docs/DEVIATION_LOG.md && rg -q "frInfill" docs/DEVIATION_LOG.md && echo PASS || echo FAIL'` — the percent-base residual row exists (PnP's generic `line_width` vs canonical's per-role `frInfill` flow width; the spacing formula itself matches)
  - `bash -c '! rg -q "FillBase\.cpp:1497-2300" docs/adr/0025-infill-linker-as-raw-emit-post-pass.md && echo PASS || echo FAIL'` — the line-pinned OrcaSlicer citation is gone
  - `bash -c 'rg -q "192-infill-linker-anchor-length" docs/specs/deviation-backlog-remediation-plan.md && echo PASS || echo FAIL'` — queue row 10 updated
  - `cargo check --workspace --all-targets` — the `resolved_config.rs` doc-comment edit did not disturb the macro-generated key table
- Exit condition: every command listed above prints `PASS`, and no new `DEV-###` row duplicates an existing number (re-derive and re-check after writing).

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | One short manifest plus one precedent block; verified by a single CLI probe. |
| Step 2 | M | Two test files, ~10 new tests, deliberately red. Both test files are short; read whole. |
| Step 3a | M | Two source files; branch semantics fully specified in `design.md`, so no exploratory reading. Ends non-compiling by design; gated on static probes only. |
| Step 3b | S | Mechanical: one production call site plus five in `connect_tdd.rs`, consuming Step 3a's `LOCATIONS` list. First step that can gate on `cargo check`. |
| Step 4 | M | The rewrite. Confined to `connect.rs` plus a bounded slice of `orchestrate.rs`; the out-of-bounds list keeps the long file's irrelevant two-thirds unread. |
| Step 5 | M | Six doc/code files across three sub-passes, all row- or section-level, all delegated reads. |

Aggregate `M`. No step is `L`. If Step 4 nevertheless exceeds budget in practice, the packet-level split seam is between the whole-arc half (Steps 1, 2, 3a, 3b plus the `anchor_length_max` gate) and the stub-plus-ordering half (Step 4) — see `design.md` §Risks and Tradeoffs for why that split was not taken up front.

## Packet Completion Gate

- All six steps (1, 2, 3a, 3b, 4, 5) and their exit conditions complete.
- Every pipe-suffixed AC command in `packet.spec.md` returns `PASS` — the 19 positive criteria and the 3 negative ones.
- `cargo check --workspace --all-targets` and `cargo clippy --workspace --all-targets -- -D warnings` are clean.
- `cargo xtask build-guests --check` reports no `STALE:` entry.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- No reopened or superseded packet status to reconcile — this packet supersedes nothing.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and the three packet-level gate commands. Record the PASS/FAIL for each; do not summarise a run that was not re-executed.
- Re-derive the new `DEV-###` numbers one final time and confirm neither duplicates a row filed by a sibling packet since Step 5.
- Re-confirm the three do-not-regress guards (AC-11, AC-12, AC-13) — all three were measured green on the unfixed tree, so any red is this packet's.
- Record remaining packet-local risk: the two filed residuals (absent neighbour-trimming, non-transportable percent form) and the solid-bucket behaviour move.
- `cargo test --workspace` is **not** required for this packet's closure and must not be run as an AC command; the targeted matrix in `requirements.md` §Verification Commands plus the two workspace gates is the closure evidence.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` where the command accepts it (`cargo test --test <binary>` selects a single target by name and does not).
