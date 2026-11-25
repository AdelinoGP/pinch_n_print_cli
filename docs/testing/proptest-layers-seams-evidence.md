# Evidence: independently checked layer, seam and path-ordering properties

Packet: `proptest-openrscad_15_pbt-layers-seams`. Status: Step 6 measured; routine and extended
`P` pass for all 11 IDs after one generator fix (M-SEAM-PLANNER-PICK `RearY`, §9.4). Work is in `wip(...)` checkpoint commits on
`feature/proptest-openrscad`; the ADR-0062 fix in §8a landed in checkpoint `5ea56deb`. The
post-fix `M-PATH-*` rows in §9.4 come from `target/step6-logs/fix2-routine-full.log` and
`target/step6-logs/fix2-extended-full.log`; the closure re-run of all 11 IDs on `5ea56deb`
(routine and extended, both `VERDICT: PASS`) is in `target/close/`.
This file records measured provenance, baseline/after statuses, exact commands,
controls, counters, shrink evidence, manual lint review, dependency-graph evidence and
confirmed defect dispositions. Sections 9.3, 9.4 and 10-12 were filled by the Step 6 run; every number in them comes from a
command executed in that run, and anything not measured is stated as unmeasured.

## 1. Provenance

| Field | Value |
| --- | --- |
| Authoring tree | `9b096111` (branch `feature/proptest-openrscad`), packet `status: draft` |
| Implementation base | `84a026e0` (`origin/master`), reached by `git merge --ff-only origin/master` (pure fast-forward, 0 local commits / 108 behind); `git stash` run first (no-op, clean tree) |
| Merge that resolved AC-1 | PR #10 `7b4c5781` from `feature/perf-vs-orca`, containing `10114830` (perf-vs-orca ticket 139) |
| Toolchain | `rustc 1.96.0 (ac68faa20 2026-05-25)`, `cargo 1.96.0 (30a34c682 2026-05-25)` |
| Platform | Windows, native host build |
| `CARGO_TARGET_DIR` | unset; workspace default `target/` for all recorded commands |
| Workspace Proptest | `proptest = "1.11"`, `default-features = false`, features `std`, `fork`, `timeout` |
| Guest freshness | `cargo xtask build-guests --check` returned exit `1` (STALE, 49 guests) immediately after the 108-commit merge; plain `cargo xtask build-guests` rebuilt `built 49 guest(s)` (EXIT 0); re-check exit `0` (EXIT_FRESH) |
| Dirty paths at B0 capture | none (`git status --porcelain` empty at `84a026e0`) |

Per-command dirty digests and `source_state_digest` are carried by the `PropertyRun`
context (`context.json`) written by `cargo xtask test --properties` under
`target/property-testing/<run_token>/<id>/`; a replay cannot present a stale provenance
line as current. Step-level provenance is re-derived at each measured run below.

## 2. Predecessor handoffs actually in the tree

- `proptest-openrscad_01_pbt-foundation` and `proptest-openrscad_02_pbt-domains`: both
  `status: implemented` on this tree (rechecked on disk). Live exports:
  `PropertyRun::{new, bound, check}` and `PropertyRunError` in
  `crates/slicer-sdk/src/test_support/property_testing/run.rs`; `CaseGuard` in
  `property_testing/mod.rs` (NOT `run.rs`); `take_case_logs`; `pub mod domains`,
  `pub mod oracles`, `pub mod regression`; `oracles::{is_permutation,
  locked_groups_are_contiguous}` and `regression::{ConcreteInput,
  format_regression_fixture}`. No shared `dyadic`/`quanta`/`select_modes` generator
  exists; owner strategies stay private.
- `PropertyRun::check` supplies a **fresh `CaseGuard` per generated case and per shrink
  attempt**; `bound(name)` returns `u64` (error on missing key or zero). Registry schema:
  14 record keys / 11 profile keys; `test_name` last `::` component starts with
  `proptest_`; `charter = "docs/testing/properties.md#<lowercased-id>"`.
- `cargo xtask test --properties` CLI: `--tier routine|extended`, repeatable
  `--property ID`, `--seed U64`, `--with-scad`, `--summary`; executor does `--list` /
  `--list --ignored` discovery and `--exact <test_name> --nocapture`; logs to
  `target/test-output.log`; artifacts to `target/property-testing/<run_token>/<id>/`.

## 3. AC-1 end-rule parity resolved upstream (see packet.spec.md §Deviations items 1–2)

The authoring tree (`9b096111`) terminated the layer sequence on the layer **top**
(`z <= object_height + 1e-9` in `layer_top_zs`, `z_f64 > height + 1e-6` in
`generate_object_layers_f64` / `merge_same_height` / `merge_different_heights`), while
canonical `generate_object_layers` (`Slicing.cpp`, three local checkouts) emits while
the band midpoint `print_z + 0.5*height` is below `object_print_z_height()`. On the
fast-forwarded tree this is **resolved by the merge, not by this packet**: `DEV-201` is
Closed, `docs/02_ir_schemas.md` §"Print top vs sample plane" is normative, and the host
`layer_top_zs` (`crates/slicer-config/src/resolution.rs`) plus all three module end
rules use the midpoint probe. Measured consequence recorded in DEV-201: `base.stl`
495 layers / last Z 99.0 → 496 / 99.2. `DEV-202` remains Open but records only opt-in
`agg`-rasterizer fixture re-measurement and a calicat bridge-site index shift; it does
not reopen sample-plane parity. No WIT/IR/ADR, `docs/02`, or deviation-log edit was
made by this packet. The two `M-LAYERS-*` properties pin the canonical midpoint rule
(independent integer half-quanta oracle), keeping only the fast(f64)/general(f32)
numeric distinction and the catch-up guard as characterizations.

## 4. Native baseline before any edit (B0, 7/7 PASS at `84a026e0`)

All seven exact `B0` commands from `packet.spec.md` §Verification, each `2>&1 | tee
target/baselines/B0/<n>.log | tee target/test-output.log`; each ran exactly 1 test
(`running 1 test`, never `0`). No false greens.

| # | test | result |
| --- | --- | --- |
| 1 | `layer_planning_tdd::test_first_layer_height_respected` | `ok. 1 passed` |
| 2 | `layer_planning_tdd::explicit_first_layer_credit_survives_a_preceding_catch_up` | `ok. 1 passed` |
| 3 | `seam_canonical_spline_tdd::canonical_spline_tests::single_observation_fit_returns_that_observation` | `ok. 1 passed` |
| 4 | `seam_continuous_projection_tdd::projects_onto_nearest_segment_point` | `ok. 1 passed` |
| 5 | `seam_degraded_fallback_tdd::missing_plan_emits_non_fatal_and_preserves_walls` | `ok. 1 passed` |
| 6 | `path-optimization-default` `--lib` `tests::locked_block_never_split_or_reversed` | `ok. 1 passed` |
| 7 | `path-optimization-default` `--lib` `tests::committed_wall_sequence_is_not_reordered_by_role_priority` | `ok. 1 passed` |

t139's edit to `layer_planning_tdd.rs` was purely additive (two new fractional-height
midpoint regressions); all pre-existing fixtures use exact multiples of the layer
height, so B0 tests 1–2 stay green under the midpoint rule.

## 5. Owner seams and dependency-graph isolation (Step 1, checkpoint `522eece7`)

Four owners (`layer-planner-default`, `seam-planner-default`, `seam-placer`,
`path-optimization-default`) each gained a default-off `[features] property_testing =
[]`, workspace `proptest` as a dev-dependency, `slicer-sdk` dev features
`["test","property_testing"]`, and a stacked `#[cfg(test)]` /
`#[cfg(feature = "property_testing")]` / `mod property_tests;` registration in
`src/lib.rs`. `Cargo.lock` gained exactly four `+ "proptest",` dev-dep entries
(unavoidable once the dev edge exists), matching the committed sibling pattern.

- 4/4 `cargo check -p <owner> --all-targets --features property_testing`: `Finished`; 0
  `error[`/`error:` (logs `target/baselines/step1/check{1,4}.log`).
- Negative-control wire probe: temporarily inserting `fn wire_probe() -> u8 {
  "not a u8" }` into each stub made all four checks fail with `error[E0308]`; stubs were
  restored byte-identically. This proves `mod property_tests;` actually reaches each
  file under `--all-targets --features property_testing`.
- Default/guest isolation: `cargo tree -p <owner> -e features --no-dev` shows 0
  `proptest` and 0 `slicer-sdk feature "property_testing"` occurrences; `cargo tree -p
  layer-planner-default --no-default-features -e normal` shows no proptest. No guest
  manifest or wasm32 target block was touched.

## 6. Registry and CI scaffolding (checkpoint `0da17b4a`)

- 11 provisional Windows-pilot records appended to `test-properties.toml`
  (71 → 82 records), all `target_kind = "lib"`, `generator_version = 1`,
  `requires_host_algos = false`, `requires_scad = false`, both tiers, `chacha`,
  `in_process`, `timeout_ms = 0`, zero reject budgets, routine 64 / extended 256 cases,
  `bounds` with the packet's exact bound keys. Validated: `tomllib` 82 records, no
  duplicate IDs, full `routine`+`extended` profiles with non-empty bounds.
- `cargo metadata --locked --no-deps` (the same input `validate_record` uses) confirms
  all four owner lib targets (`layer_planner_default`, `seam_planner_default`,
  `seam_placer`, `path_optimization_default`) and the `property_testing` feature exist.
- Real xtask validation: `cargo xtask test --summary --properties --tier routine
  --property H-IR-ANCHORS` returned **VERDICT: PASS** (`test result: ok. 1 passed`),
  which parses and validates ALL 82 records against Cargo metadata before executing the
  selected property. A `--property M-LAYERS-SEQ` probe correctly reported
  `missing_property_execution` for the two layer records (stubs had no `proptest_*`
  tests yet) — expected pre-implementation discovery behaviour.
- `.github/workflows/ci.yml` `properties-windows` assertion extended 32 → 43 IDs:
  step name, `Write-Host`, comment, and the `@(...)` list now include all 11 new IDs;
  verified 43 unique asserted IDs (isolated to the `properties-windows` job).
- **Profiles were PROVISIONAL pilots** at this checkpoint. Step 6 (§9.4) measured both tiers
  passing with the same values; the registry comment now says measured.

## 7. Charters (checkpoints `e527711c`, `977e8c53`)

11 `## <ID>` sections appended to `docs/testing/properties.md`, each with the fixed
11-subsection shape. A read-only cold audit found and the planner fixed 6 over-claims:
provisional profiles called "measured"; `--seed 42` replay stated as a result; a
frozen "43 IDs" ledger count; the two AC-N2 fixes described as already done; and the
`InternalBridgeInfill` wildcard outcome decided before any investigation. Verified
after the fix: 11 IDs, one heading each, all 11 subsections each, all registry charter
anchors resolve, LF preserved, no stale over-claim strings in the packet-15 block.

## 8. Source-observed candidate witnesses and confirmed defect dispositions

Read-only contract survey (verbatim sources cited by symbol/section, never line numbers):

- **AC-N2 Random empty-candidate**: `select_seam_candidate`'s `SeamMode::Random` arm
  evaluates `(layer_index as usize) % candidates.len()` before `.get()`, so an empty
  slice panics (divide by zero). The empty-candidate guarantee is documented
  MODE-AGNOSTICALLY (`docs/05_module_sdk.md` T-082 + P109 graceful degradation:
  "empty `seam_candidates` and no `resolved_seam` … emits the region's walls pristine";
  wall-preservation invariant "No path in this contract may drop, skip, or fail to emit
  a region's wall loop"). No doc says "Random must not panic" by name, so the fix is
  gated on **in-session reproduction** per AC-N2 and packet §"Seam placer".
- **AC-N2 open-path force-closing**: `project_onto_wall_segment` wraps
  `points[last] -> points[0]` as a real segment for open paths, and
  `insert_projected_point` unconditionally appends a closing point/width (flags
  closing push is `is_closed`-gated), so an open N-point polyline becomes `points =
  widths = N+2`, `flags = N+1`. Applicable live contract: `docs/02_ir_schemas.md` IR 7
  "legitimately open paths … exist. **Nothing may fabricate closure for an open
  line**" + unconditional per-point-array rule + `docs/01_system_architecture.md` /
  `docs/05_module_sdk.md` cardinality postcondition. `docs/01`/`docs/05`'s "re-closes
  the loop" presumes a closed loop and does not authorize closing an open polyline.
  Corroborated by `rotate_wall_loop`'s own "Open walls … are canonical
  `ExtrusionPath`s, not loops" comment.
- **Disposition (confirmed in session; supersedes the "gated"/"suspected" wording above).**
  Commit `ab05d774` reproduced each defect on HEAD `lib.rs` before fixing, and added three
  named regressions in `modules/core-modules/seam-placer/tests/seam_placer_ac_n2_regression_tdd.rs`:
  `random_empty_candidates_degrade_without_panic` (empty candidates returned `None` before
  Random's modulo: divide-by-zero), `open_polyline_projection_preserves_open_geometry`
  (open polylines no longer get an implicit closing chord/repeat) and
  `closed_loop_with_closing_repeat_metadata_mismatch_keeps_closure_and_alignment` (closure
  uses the XY-only `ExtrusionPath3D::is_closed`, matching `rotate_wall_loop`; the
  closure-representation mismatch below was therefore a third confirmed defect, not only a
  characterization). Re-run in Step 6: `cargo test -p seam-placer --features property_testing
  --test seam_placer_ac_n2_regression_tdd` -> 3 passed (`target/step6-logs/ac-n2-regressions.log`).
  Two pre-existing open-fixture baselines were changed to closed loops with assertions
  unchanged (per the commit message).
- **Closure-representation mismatch** (characterization witness, not a separate fix):
  projection/insertion use full `Point3WithWidth` equality `first()==last()` while
  `ExtrusionPath3D::is_closed()` is XY-only; a same-XY/different-metadata closing
  endpoint is treated as open by projection. Classified, not silently treated as an
  extra independent vertex.
- **`role_group` `_ => 9` wildcard** (Step 5, characterization only, NO fix): the
  only statement that new infill roles such as `InternalBridgeInfill` belong with
  infill is the **archived** `docs/spec_packets/_OLD/61_path-optimization-role-sequencing.md`
  (`status: implemented`, a design-only digest per `docs/spec_packets/README.md`,
  "not implementable packet directories"). No live doc (`docs/01`, `docs/02` IR
  4.8.0, ADR-0062, `docs/07`) mandates infill-group membership; `docs/02` gives
  `InternalBridgeInfill` `default_priority() = 3500`, a different mechanism. Variants
  matched by `_ => 9`: `InternalSolidInfill`, `RaftInfill`, `SupportBaseInterface`,
  `Brim`, `InternalBridgeInfill`. Packet §"Path ordering" says "investigate … rather
  than blessing the wildcard fallback or excluding it without evidence" → characterize.
- **ADR-0062 cross-partition locked blocks (review finding, high; FIXED, see §8a).**
- **Dormancy** (Step 3): `comparator::pick_seam_point`, `pick_random_seam_point` and
  `align::fit_cubic_bspline` (transitively via `align_seam_points`) have zero production
  callers; reported as dormant-kernel coverage. `region_candidates` /
  `choose_region_candidate` are live (called by `run_region_planning_entries`) and the
  direct selection test is not credited as end-to-end paint delivery.

### 8a. ADR-0062 cross-role / cross-tool locked blocks (review fix)

**Finding.** `PathOptimizationDefault::group_then_nearest_neighbor`
(`modules/core-modules/path-optimization-default/src/lib.rs`) partitioned entities by
`tool_index_of`, then walls vs `role_group`, and only then called `coalesce_locked_candidates`
inside each partition. A valid ADR-0062 block (same tag, adjacent in staged order) whose members
fell into different tool clusters, role groups, or walls/non-walls was therefore split. The
Step 5 property pinned that split as correct (`LockFailure::Characterized` /
`GroupFailure::Characterized`), which contradicted AC-4 and the packet's "Locks take precedence
when a role/tool grouping would split one".

**Constraint check (no genuine upstream constraint found).**
- `validate_entity_order_locks` (`crates/slicer-runtime/src/layer_executor.rs`) keys only on
  adjacent equal `Some(order_lock)` over the staged `ordered_entities`; it has no role or tool
  condition. It rejects a proposal that reverses, splits or interleaves such a run
  (`"locked block {tag} was reversed|split|interleaved or reordered"`).
- `apply_cross_layer_tool_rotation` (`crates/slicer-gcode/src/emit.rs`, host-side rotation in
  `DefaultGCodeEmitter`) explicitly supports blocks spanning tools: its comment reads "A lock is
  an atomic authored sequence. If the tool cluster touches any member, rotate the complete
  block", and it extends the cluster range to a fixpoint over lock blocks. Its test
  `order_lock_tool_rotation_preserves_block` covers it. It does not reject cross-tool blocks.
- `OrderLockAllocator` (`crates/slicer-sdk/src/order_lock.rs`) only allocates tags; it carries no
  role/tool restriction. ADR-0062 says any layer producer may mint locks.
Conclusion: nothing rules out cross-role or cross-tool blocks reaching this optimizer, so no
partition keeps a characterization.

**Host-contract reproduction.** `crates/slicer-runtime/tests/unit/layer_collection_builder_tdd.rs`
new test `order_lock_cross_role_block_split_by_role_partition_rejected`: a tag-7 block over
`[SparseInfill, Ironing]` plus an unrelated entity; the role-partitioned proposal
`[(0,false),(2,false),(1,false)]` (what the pre-fix module emitted) returns
`set-entity-order: locked block 7 was interleaved or reordered`; the forward proposal is accepted.
(Existing `order_lock_proposal_split_rejected` covers the role-agnostic split.) The module-level
property executes the split on the real SUT as well.

**Fix.** Before partitioning, `coalesce_locked_candidates` is applied to the whole staged input,
yielding units (maximal runs of equal `Some(order_lock)` adjacent in staged order, the host's
own adjacency rule; no region/object comparison, because the host validator has none and tags
are globally remapped before this stage; untagged entities are singleton units). Each unit goes
to the partition (tool cluster; wall vs `role_group`) of its FIRST member; candidate
start = first member start, end = last member end; members are emitted adjacent, in authored
order, reversal `false`. Unlocked same-partition behaviour is unchanged (walls in staged order,
skirt before walls, tool clusters ascending, role groups ascending); tool-change records are
still derived from the final permutation. One behavioural nuance: two same-tag entities that
were adjacent only within a partition (not in staged input) are no longer merged.

**Property changes.** `property_tests.rs`: the `Characterized` pins are gone (variant renamed
`Witness`, used only for "generator witness lost its property" checks). `check_locks` requires
every locked run, cross-partition ones included, to be a contiguous forward window (own
`window_start` plus `oracles::locked_groups_are_contiguous` over all blocks) and unit
first-member tools to ascend; `check_groups` derives units and `(first tool, segment)`
partitions independently (never `role_group`) and requires exact unit membership/order.
Controls: the former split outputs (`[0,4,1,2,5,3,6]` cross-role, `[0,2,1,3]` cross-tool) are
now rejected by the same checker as `LockFailure::Block` / `GroupFailure::Block`; interleaved,
reverse-ordered and flagged-reversal variants are also rejected. The `split` counter must be 0.

**Regression** (`tests::cross_role_and_cross_tool_locked_blocks_stay_adjacent_and_forward` in
`modules/core-modules/path-optimization-default/src/lib.rs`, RNG-independent; doc comment gives
root cause and failing input).
- On the HEAD `lib.rs` logic (`git show HEAD:` body plus the new test), log
  `target/step6-logs/fix2-head-fail.log`: `... cross_role_and_cross_tool_locked_blocks_stay_adjacent_and_forward ... FAILED`,
  `left: [(0, false), (2, false), (1, false)] right: [(0, false), (1, false), (2, false)]`.
- With the fix, `target/step6-logs/fix2-fix-pass.log`: `... ok`, `1 passed; 0 failed`.

**Wildcard producers (review medium; `role_group` NOT changed).** Roles reaching `_ => 9`:
`InternalBridgeInfill`, `InternalSolidInfill`, `RaftInfill`, `SupportBaseInterface`, `Brim`
(plus `Custom(_)` explicitly 9). Production emitters found by grep: `InternalBridgeInfill` by
`rectilinear-infill` (`bridge_role` in `modules/core-modules/rectilinear-infill/src/lib.rs`) and
`wave-overhangs` (`fallback_role`, `modules/core-modules/wave-overhangs/src/lib.rs`);
`SupportBaseInterface` by `traditional-support` and `tree-support` (`SupportPlanRole::BaseInterface`
arm, `path.role = ExtrusionRole::SupportBaseInterface`). `RaftInfill` appears in production only
as the `should_emit` claim selector in `raft-default` (`modules/core-modules/raft-default/src/lib.rs`);
no assignment of that role to a path was found by grep. `InternalSolidInfill` and `Brim`
producers were not grepped in this pass. Consequence: `InternalBridgeInfill` and
`SupportBaseInterface` (live-emitted) sort after wipe/prime towers (group 9) rather than with
infill/support. No non-archived contract states their group (a `docs/` grep for `role_group`
found none outside the archived packet 61), so it is reported and left unchanged.

## 9. Executed verification

### 9.1 Step 2 — layer sequence / merge (checkpoint `9b72a4a8`)

File `modules/core-modules/layer-planner-default/src/property_tests.rs` (2738 lines):
`proptest_layer_sequence`, `proptest_layer_merge`, one combined
`packet15_witnesses_controls_and_shrinking`. Independent **integer half-quanta**
canonical-midpoint oracle (`half_quanta_midpoint`: emit while `T(n) = 2(F+nS) −
(n==0?F:S) < 2H`) cross-checked against a closed form
(`closed_form_native_count`); `f64_native_tops` transcribes the documented rule and
must agree bit-for-bit on the grid; the global `f32` plane `(raft_top + z) as f32` is
lossless for every generated plane (quantum index < 2^17 < 2^24 mantissa), licensing
bit-exact comparison. One shared `check_layer_sequence`; 8 positive `Cell` counters per
property; 9 named mutation controls; real `ValueTree` simplify/complicate.

| Command | Result |
| --- | --- |
| `cargo check -p layer-planner-default --all-targets --features property_testing` | `Finished`, 0 warnings |
| `cargo clippy -p layer-planner-default --all-targets --features property_testing -- -D warnings` | `Finished` |
| `C`: `packet15_witnesses_controls_and_shrinking --exact` (planner re-run) | `test result: ok. 1 passed`; `witnesses=… controls=rejected shrinking=green`; 581 simplifications, complexity e.g. 19576→12 |
| `B`: `test_first_layer_height_respected`, `explicit_first_layer_credit_survives_a_preceding_catch_up` | `ok. 1 passed` each |
| property-body pilot (injected local context, NOT the `P` run) | M-LAYERS-SEQ 32/32, M-LAYERS-MERGE 64/64 accepted, 256 cases × seeds 1–5 (1280 each) green, zero rejects; full `layer_planning_tdd` 17/17 |

### 9.2 Step 3 — planner pick / paint / spline (checkpoint `e4380ec9`)

File `modules/core-modules/seam-planner-default/src/property_tests.rs` (3483 lines):
three properties, three independent oracles (range index membership/priority, paint
annotation→score→selection, restricted weighted-mean fit). Weighted-mean degeneracy
`[[0,0],[10,0],[0,10]]`, weights `[1,2,1]` → `[5,2.5]` (f64 accumulation, measured
`[5.0000005, 2.5000002]`, 1 f32 ulp, inside 1e-3 mm); unweighted-mean and non-finite
controls rejected by the same checker. 20 deterministic witnesses, 10 mutation
controls, 3 ValueTree shrink probes, one combined controls/shrink test.

| Command | Result |
| --- | --- |
| `cargo check -p seam-planner-default --all-targets --features property_testing` | `Finished` |
| `cargo clippy -p seam-planner-default --all-targets --features property_testing -- -D warnings` | `Finished`, 0 warnings |
| `C`: `packet15_witnesses_controls_and_shrinking --exact` (planner re-run) | `test result: ok. 1 passed`; `pick=7 paint=10 spline=3`, three shrink traces |
| `B`: `seam_canonical_spline_tdd::canonical_spline_tests::single_observation_fit_returns_that_observation` | `ok. 1 passed` |
| property-body pilot (injected context, NOT `P`) | PICK / PAINT / SPLINE 64/64 each, all counters positive |
| collateral (feature on) | comparator 12/12, planner 4/4, region-aware 9/9, spline 23/23 |

### 9.3 Steps 4 and 5

**Step 4 — seam-placer select / project / rotate (checkpoint `ab05d774`).** File
`modules/core-modules/seam-placer/src/property_tests.rs`: `proptest_placer_select`,
`proptest_placer_project`, `proptest_placer_rotate` and one combined
`packet15_witnesses_controls_and_shrinking`. Three confirmed production defects (see §8
Disposition), each with a named RNG-independent regression in
`tests/seam_placer_ac_n2_regression_tdd.rs`: empty-candidate Random modulo panic,
open-polyline forced closing, and XY-only closure disagreement between projection/insertion
and `rotate_wall_loop`.

**Step 5 — path nearest / locks / groups (checkpoint `ed0a8c99`).** File
`modules/core-modules/path-optimization-default/src/property_tests.rs`: `proptest_path_nearest`,
`proptest_path_locks`, `proptest_path_groups` and one combined controls/shrink test. Oracles:
greedy first-choice distance + `is_permutation`, adjacent-run derivation +
`locked_groups_are_contiguous`, and an authored role table (never `role_group`).
**Characterized, no production change:** the `role_group` `_ => 9` wildcard takes
`InternalBridgeInfill`, `InternalSolidInfill`, `RaftInfill`, `SupportBaseInterface`, `Brim`
(no live contract mandates infill-group membership; see §8 and §8a). The Step 5 pin that
cross-tool/cross-group locked blocks are split was WRONG against ADR-0062 and was replaced by
the fix and properties in §8a.

### 9.4 Measured Windows profiles, counters and replay (Step 6)

**Provenance.** Base HEAD `ab05d7744f70858cb35d8233cd7e81379b990e15` (branch
`feature/proptest-openrscad`), clean at the start of Step 6. The final (post-fix) `P` and replay
runs carried `dirty_paths` = `docs/testing/properties.md`,
`docs/testing/proptest-layers-seams-evidence.md`,
`modules/core-modules/seam-placer/src/property_tests.rs` (waiver comment, §10),
`modules/core-modules/seam-planner-default/src/property_tests.rs` (generator fix, below) and
`test-properties.toml` (`generator_version` and comment), with
`source_state_digest = d4e13871d0850f66a2f687df51bacf2a21824be2c72b79ec517281a851c8f10f`
for both tiers. `rustc 1.96.0 (ac68faa20 2026-05-25)`, `cargo 1.96.0 (30a34c682 2026-05-25)`,
host Windows 11 Home 10.0.26300, native target, `CARGO_TARGET_DIR` unset. Logs and artifact
copies are under `target/step6-logs/` (untracked build output); the post-fix runs are the
`F-*` files.

**Compile vs execution.** Each `P` child printed `Finished` in under 1 s (the owner test
binaries had just been built by the preceding `C`/`B`/clippy commands), so the runs are
execution-dominated. The post-fix routine xtask command took 122 s wall and the extended one
26 s; the routine figure includes `xtask test: pnp_cli is stale or absent; rebuilding...`
(the compile was not separated further, so cold compile cost is **unmeasured**). Per-test
execution times are the `finished in` values below.

#### Initial extended failure (pre-fix) — retained for the record

First extended run (HEAD `ab05d774`, run token `d3db09a76916ada18bd3cbfabf4ed156`, exit 101):
ten IDs passed, **M-SEAM-PLANNER-PICK failed** at seed `6119898662842419507` after 74 accepted
cases (failure on call 75, then 256 shrink calls). Message `rear-y case: fixture outside the
domain: rear-Y class lost its Y spread`. Failing pre-shrink input: a `RearY` `PickCase` with 2
candidates, `range: 1..2`, both at `y = 8.0` (x 4.899414 / 6.073242); the shrunk input kept
both at `y = 8.0`. **Classification: generator** (the fixture guard in
`validate_pick_case` fired; the SUT `pick_seam_point` was never judged). **Root cause:** in
`build_pick_case`, for `RearY` with `len = 2`, `lo = 1`, `hi = 2` the overlay set
`candidates[lo].y = from_quanta(coord_cap)` (no pair, so the target is `lo`), and then, since
`lo > 0`, `paint_strongest(candidates[0], RearY, coord_cap)` set the out-of-range decoration to
the same `coord_cap`, collapsing the Y spread. More generally the in-range Rear answer and the
out-of-range "strongest" decoration tied at `coord_cap`, weakening the
"range-ignoring kernel returns a different index" intent. Routine (64 cases) did not hit it.
Pre-fix routine run (run token `c8ed548a7ad1ca3b461303d33eb81098`) passed 11/11; logs
`routine.log`, `extended.log`.

**Fix (generator only; no change to `lib.rs`, the oracle, `check_pick` or any checker
assertion).** `build_pick_case`'s `RearY` overlay now reserves one quantum of headroom: all
in-range Y values are clamped to `coord_cap - 1` quanta, the in-range answer is
`coord_cap - 1`, and the out-of-range decoration keeps `coord_cap`, so every decoration is
strictly better under the Rear key than every in-range candidate, the class always has a Y
spread, and all coordinates stay within `max_coord_quanta`. Other classes' generation is
unchanged. **`generator_version` for `M-SEAM-PLANNER-PICK` was bumped 1 -> 2** in
`test-properties.toml` (and its charter) because `docs/testing/properties.md`
("Registry schema": `generator_version` — "bumped when the generator semantics change") requires
it when generator semantics change.

**New deterministic witness** `assert_rear_y_decoration_witness`, called from
`packet15_witnesses_controls_and_shrinking` (so the exact `C` command runs it). For two raw
layouts and every `(len, lo, hi)` with `2 <= len <= max_candidates`, `lo < hi <= len` it asserts
the built case has the requested shape, passes `validate_pick_case`, each decoration slot
(index 0 when `lo > 0`, `len - 1` when `hi < len`) is strictly better than every in-range
candidate, the real `pick_seam_point` result is accepted by `check_pick`, and the range-ignoring
whole-slice scan (an index outside the range) is rejected by the same checker as out-of-range;
and it names the exact failing shape (`RearY`, `len = 2`, range `1..2`) asserting a Y spread.
**Old generator** (overlay hunk temporarily reverted, restored afterwards; log
`fix-C-oldgen.log`): the `C` test FAILED with `rear-y 2/0..1: fixture outside the domain:
rear-Y class lost its Y spread` (`test result: FAILED. 0 passed; 1 failed`). The sweep reaches
`2/0..1` before `2/1..2`; both are the same collision. **New generator** (`F-C.log`):
`test result: ok. 1 passed`. A first draft of the witness over-asserted that every
out-of-range candidate (not only the two decoration slots) is strictly better; that was a
witness bug (non-decoration out-of-range candidates keep the plain layout) and was corrected
before the final run. No bound, profile or checker assertion was changed.

After the fix, `build-guests --check` returned exit 1 (`STALE: seam-planner-default-guest`);
`cargo xtask build-guests` rebuilt it (exit 0) and the re-check returned 0.

#### Post-fix routine tier (`cargo xtask test --summary --properties --tier routine` + 11 IDs)

Exit 0, `VERDICT: PASS`, 122 s wall, run token `21f8952b62e6267b8ad0b11be56402ad`, 22
controller events (begin + end per ID), all 11 `outcome: passed`, `completion_eligible: true`,
`accepted_cases = 64`, `rejected_test_calls = 0`, 0 local/global rejects, `generator_version`
2 for PICK. Logs `F-routine.log`, `F-routine.stdout`, `F-routine-property-testing/`.

| ID | cases | rejects | test wall | positive counters (generated unless `witness_`) |
| --- | --- | --- | --- | --- |
| M-LAYERS-SEQ | 64 | 0 | 0.03 s | native_events=974 catchups=282 raft_entries=94 participating=117 fast=34 general=30 explicit=30 uniform=64 |
| M-LAYERS-MERGE | 64 | 0 | 0.03 s | native_events=1588 catchups=1168 raft_entries=101 participating=184 fast=32 general=32 explicit=32 uniform=64 |
| M-SEAM-PLANNER-PICK | 64 | 0 | 0.06 s | paint=22 central=18 visibility=6 rear_y=9 ties=9 sub_ranges=61 |
| M-SEAM-PLANNER-PAINT | 64 | 0 | 0.30 s | painted=50 blocked=43 holed=25 random=8 singleton=2 candidates=552 |
| M-SEAM-PLANNER-SPLINE | 64 | 0 | 0.05 s | constant=14 weighted_mean=24 single=26 interior_queries=140 queries=344 |
| M-SEAM-PLACER-SELECT | 64 | 0 | 0.04 s | nonempty=64 ties=5 random_cycles=6 per_mode=[15,17,9,9,14] |
| M-SEAM-PLACER-PROJECT | 64 | 0 | 0.05 s | interior=59 closed=31 open=28 endpoints=5 multi_wall=50 |
| M-SEAM-PLACER-ROTATE | 64 | 0 | 0.04 s | closed=21 open_unchanged=43 shifted=17 |
| M-PATH-NEAREST | 64 | 0 | 0.04 s | permutations=49 ties=16 empty=15 singleton=15 modes=[14,12,16,9,13] |
| M-PATH-LOCKS | 64 | 0 | 0.16 s | permutations=55 blocks=27 empty=9 singleton=12 separated=10 cross_tool=7 (post ADR-0062 fix, §8a; `fix2-*-full.log`) |
| M-PATH-GROUPS | 64 | 0 | 0.04 s | permutations=46 tool_blocks=88 group_segments=180 wall_segments=10 tool_changes=64 cross_partition=20 split=0 empty=18 (post ADR-0062 fix, §8a; `fix2-*-full.log`) |

(Unseeded runs draw a fresh seed each time, so generated counters differ between runs. The
deterministic `witness_*` counters printed in the logs are separate from the generated
populations above and are not counted toward them.)

#### Post-fix extended tier (same command with `--tier extended`)

Exit 0, `VERDICT: PASS`, 26 s wall, run token `d013134ebea100ba4b3bd4b7744e380e`, 22
controller events, all 11 `outcome: passed`, `completion_eligible: true`,
`accepted_cases = 256`, `rejected_test_calls = 0`. Logs `F-extended.log`,
`F-extended.stdout`, `F-extended-property-testing/`.

| ID | cases | rejects | test wall | positive counters |
| --- | --- | --- | --- | --- |
| M-LAYERS-SEQ | 256 | 0 | 0.11 s | native_events=4001 catchups=1173 raft_entries=318 participating=475 fast=133 general=123 explicit=123 uniform=256 |
| M-LAYERS-MERGE | 256 | 0 | 0.13 s | native_events=6228 catchups=4717 raft_entries=308 participating=721 fast=131 general=125 explicit=125 uniform=256 |
| M-SEAM-PLANNER-PICK | 256 | 0 | 0.12 s | paint=71 central=81 visibility=30 rear_y=41 ties=33 sub_ranges=236 |
| M-SEAM-PLANNER-PAINT | 256 | 0 | 0.60 s | painted=203 blocked=194 holed=106 random=47 singleton=20 candidates=2056 |
| M-SEAM-PLANNER-SPLINE | 256 | 0 | 0.16 s | constant=74 weighted_mean=88 single=94 interior_queries=554 queries=1364 |
| M-SEAM-PLACER-SELECT | 256 | 0 | 0.12 s | nonempty=256 ties=27 random_cycles=41 per_mode=[46,57,56,51,46] |
| M-SEAM-PLACER-PROJECT | 256 | 0 | 0.17 s | interior=218 closed=127 open=91 endpoints=38 multi_wall=194 |
| M-SEAM-PLACER-ROTATE | 256 | 0 | 0.11 s | closed=132 open_unchanged=124 shifted=91 |
| M-PATH-NEAREST | 256 | 0 | 0.23 s | permutations=201 ties=51 empty=55 singleton=65 modes=[50,58,49,43,56] |
| M-PATH-LOCKS | 256 | 0 | 0.15 s | permutations=208 blocks=101 empty=48 singleton=36 separated=37 cross_tool=44 (post ADR-0062 fix, §8a; `fix2-*-full.log`) |
| M-PATH-GROUPS | 256 | 0 | 0.17 s | permutations=201 tool_blocks=415 group_segments=927 wall_segments=57 tool_changes=316 cross_partition=102 split=0 empty=55 (post ADR-0062 fix, §8a; `fix2-*-full.log`) |

Both tiers pass with the registered values (routine 64 cases / size 32 / shrink 64, extended
256 / 128 / 256, bounds as in `test-properties.toml`); no profile or bound was changed to obtain
the pass (only PICK's `generator_version`, above). Largest measured test time is 0.60 s
(M-SEAM-PLANNER-PAINT extended); no timing concern for the required routine lane (largest
routine test 0.30 s). Pre-fix extended runs of the other ten IDs also passed (previous section).
`singleton=0` for PAINT in the pre-fix routine run is now 2; the packet names no generated
singleton minimum.

Re-running the old failing seed (`--seed 6119898662842419507`, extended, PICK only) passes
(`F-oldseed.log`), but because the generator changed the seed no longer produces the same
inputs, so that pass is not independent evidence; the deterministic witness is.

#### Seed 42 replay of M-SEAM-PLACER-SELECT (routine), run twice, repeated after the fix

`cargo xtask test --summary --properties --property M-SEAM-PLACER-SELECT --tier routine --seed
42` twice (`target/property-testing/` cleared before each). Both exit 0, `VERDICT: PASS`,
`context.json` `seed: 42`, wall 11 s and 13 s (xtask), test `finished in` 0.04 s each.
Artifacts: `target/step6-logs/F-replay{1,2}-property-testing/<run_token>/M-SEAM-PLACER-SELECT/`
(`case.txt`, `observations.jsonl`, `context.json`, `source-state.json`), logs
`F-replay{1,2}.log`. **Concrete input stream:** `case.txt` (64 `--- attempt` blocks, 124084
bytes) is byte-identical (`cmp`; sha256
`c09411e3a16c8f7cdd436edc882e784c69236c3b08b869118abd751100957782` both) and
`observations.jsonl` (128 lines) is identical after stripping the per-process `"pid"` field.
Counters identical: `accepted_calls=64 nonempty=64 ties=9 random_cycles=10
per_mode=[12, 12, 13, 9, 18]`. The same hash was obtained in the pre-fix replay pair
(`replay{1,2}-property-testing`), as expected since SELECT's generator is untouched.

#### Standalone commands

Pre-fix, each executed exactly 1 test and passed: 4 x `C`, 7 x `B`, 4 x `cargo check` and 4 x
package clippy (`C1..C4`, `B1..B7`, `check-*`, `clippy-*` logs). After the generator fix,
re-run: `C` seam-planner-default `1 passed` (0.01 s, `F-C.log`; `pick=7 paint=10 spline=3`),
`B` `single_observation_fit_returns_that_observation` `1 passed` (`F-B.log`), `cargo clippy -p
seam-planner-default --all-targets --features property_testing -- -D warnings` exit 0
(`F-clippy.log`), `cargo xtask check-fmt` `62 package(s) clean`. The seam-placer `C`
(`1 passed`) was re-run after its §10 edit; the other two owners' sources did not change after
their pre-fix `C`/`B`/check/clippy runs.

## 10. Manual literal / test-quality review (scanner blind spot)

`xtask/src/check_literals.rs::collect_enforced_files` and the equivalent in
`check_test_quality.rs` collect only `crates/*/{tests,benches,src}` (`src` under
`ScanMode::CfgTestOnly`) and `modules/core-modules/*/tests`; they do **NOT** scan
`modules/core-modules/*/src/**` (normatively `docs/21_data_defaults_and_fixtures.md`
§7.1). So a green `cargo xtask check-literals` / `check-test-quality --report` is NOT
evidence for the four `src/property_tests.rs` files; each is reviewed by hand:

| Owner file | Watched-type literals | Discipline |
| --- | --- | --- |
| layer-planner-default | `RegionLayerProposal` (in `expected_region`), `LayerPlanningObject` (in `planning_objects`) | `// exhaustive:` waivers; 4 `// test-quality:` determinism pins |
| seam-planner-default | `SeamPlanningRegionInput` | FRU `..Default::default()`; 3 `// test-quality:` pins; no `Point3WithWidth`/`ScoredSeamCandidate` literals |
| seam-placer | `Point3WithWidth` (8 fields), `WallFeatureFlags` (6), `WallLoop` (6), `ExtrusionPath3D` (5) | `plain_point`, `distinct_flags`, `wall_from_records` use `..` FRU; `rich_point` has an `// exhaustive:` waiver; `model_interpolate` lacked one and was fixed in Step 6 (accurate waiver); 5 `// test-quality:` determinism pins; no `SeamCandidate` literal |
| path-optimization-default | `Point3WithWidth` (8), `OrderedEntityView` (8) | all literals use `..` FRU (`..Default::default()` / `..ordered_entity_view_base(role)`); no waiver needed; 4 `// test-quality:` determinism pins |

Method (Step 6): field counts were recomputed from the `pub struct` definitions under
`crates/*/src` (`Point3WithWidth` 8, `WallFeatureFlags` 6, `WallLoop` 6, `ExtrusionPath3D` 5,
`OrderedEntityView` 8), then every `Name { .. }` literal of a watched type in both files was
enumerated with a throwaway script and read. One violation (seam-placer `model_interpolate`,
an unwaived full 8-field `Point3WithWidth` literal) was found and fixed in the test file only
(waiver comment, no logic change); afterwards the seam-placer `C` test (1 passed), package
clippy (`Finished`) and `cargo xtask check-fmt` (clean) were re-run. Because seam-placer is
also a guest, the comment edit made `cargo xtask build-guests --check` report
`STALE: seam-placer-guest` (exit 1); `cargo xtask build-guests` rebuilt it and the re-check
returned exit 0.

Test-quality review (`docs/22_test_quality.md`): grep found no `assert!(true)`, decorative or
success-only checks; every generated population has a positive-counter assertion; the
hand-maintained enum rosters (`MODES`, `REASONS`, `TOPOLOGIES`, `NEAREST_MODES`, `LOCK_MODES`,
`GROUP_MODES`) are iterated with a per-mode "was generated" assertion; `WILDCARD_ROLES` (path)
is a deliberately authored characterization roster. Residual concern: the path
`characterized_group` table is an authored mirror of the current wildcard, not a contract
oracle (lock-block assertions are no longer characterizations; see §8a). This was a bounded manual review, not a
line-by-line audit of the ~7.2k lines in the two files.

Watched-type threshold: `pub` struct under `crates/*/src` with ≥5 named fields. Test
quality obligations (`docs/22_test_quality.md`): no self-referential oracle, no
vacuous loop, no success-only assertion, no hand-maintained roster, no decorative
assert; determinism pins marked `// test-quality: <reason>`.

## 11. Workspace gates (Step 6)

| Gate | Result |
| --- | --- |
| `cargo build --workspace` | exit 0, 333 s wall (before the generator fix; includes compile) |
| `cargo check --workspace --all-targets` | exit 0, 258 s (before the generator fix) |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0, 352 s before the fix; re-run after all source changes: exit 0, 11 s (`F-g-clippy.log`) |
| `cargo xtask check-fmt` | exit 0, `62 package(s) clean` (after the final source change) |
| `cargo xtask check-literals` | exit 0, `0 violation(s) in 0 file(s) (watchlist: 199 types)` (before the fix) |
| `cargo xtask check-test-quality --report` | exit 0, `0 finding(s) in 0 file(s) [report mode]` (before the fix) |
| `cargo xtask build-guests --check` | exit 0 at start; exit 1 after the §10 comment edit (`STALE: seam-placer-guest`) and again after the generator fix (`STALE: seam-planner-default-guest`); each cleared by `cargo xtask build-guests` (exit 0) and a re-check exit 0 |

The workspace `build` and `check` ran before the generator fix and were not repeated; the
test-only edits are covered by the post-fix workspace clippy (all targets), the package
`C`/`B`/clippy runs, and the full post-fix `P` runs. "0 file(s)" in the literal/test-quality
gates confirms the §10 scanner blind spot; the generator-fix literals (`seam_candidate(..)`
calls and a `for` over `candidates[lo..hi]`) contain no watched-type struct literal. No
workspace test suite was run.

**CI discovery.** `.github/workflows/ci.yml` job `properties-windows` runs
`cargo xtask test --summary --properties --tier routine` with **no `--property` filter**; the
executor selects every registry record that has a routine profile and does not set
`requires_scad` (`xtask/src/properties/executor.rs`, record filter
`(selection.with_scad || !record.requires_scad) && record.profile(selection.tier).is_ok()`).
All 11 records have `requires_scad = false` and a routine profile. The step
`Assert the 43 required property records executed` then `Select-String`s each ID (the 11 IDs
are the `'M-LAYERS-SEQ' ... 'M-PATH-GROUPS'` lines) in `target/test-output.log` and fails on
any missing. A real Windows CI run of this lane was not performed here; the local run of the
same command with the 11-ID filter printed each ID's execution line in `target/test-output.log`.

## 12. Coverage ledger (Step 6, post-fix)

| ID | Implemented | Registered | Discovered | Routine (64) | Extended (256) | Replay |
| --- | --- | --- | --- | --- | --- | --- |
| M-LAYERS-SEQ / MERGE | yes | yes | yes | pass | pass | - |
| M-SEAM-PLANNER-PICK (`generator_version = 2`) | yes | yes | yes | pass | pass after generator fix (first extended run failed, generator defect) | - |
| M-SEAM-PLANNER-PAINT / SPLINE | yes | yes | yes | pass | pass | - |
| M-SEAM-PLACER-SELECT | yes | yes | yes | pass | pass | seed 42 x2 identical (twice, pre- and post-fix) |
| M-SEAM-PLACER-PROJECT / ROTATE | yes | yes | yes | pass | pass | - |
| M-PATH-NEAREST / LOCKS / GROUPS | yes | yes | yes | pass | pass | - |

Confirmed SUT regressions: the three seam-placer AC-N2 tests in §8. Confirmed generator
defect (not SUT): the PICK `RearY` decoration collision (§9.4), fixed with a deterministic
witness in `packet15_witnesses_controls_and_shrinking`. Characterized (no fix): `role_group`
wildcard, dormant planner comparator/spline. Confirmed SUT defect FIXED after review: cross-role /
cross-tool ADR-0062 locked blocks were split by the path optimizer (§8a). The registry
profiles are marked measured in `test-properties.toml`; the Windows CI lane itself was not run
in this session.
