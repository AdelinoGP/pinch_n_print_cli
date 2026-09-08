# Implementation Plan: 241b-support-plan-ownership-seam

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Declared merge key in `union_same_family_entries`

- Task IDs: `TASK-531`
- Objective: replace the centroid `routing_cell` term with `region_id`; delete `RoutingCell`, `routing_cell`, `group_cells`, `same_body`; **rename** `ROUTING_CELL_SIZE` to `MAX_BODY_EXTENT_UNITS` (same `1 << 20` value) and update its two uses inside `in_routing_cell`, which keeps its name; add the first test of the non-`same_body` merge branch.
- Precondition: `mkdir -p target && cargo test -p slicer-wasm-host --test unit support_cross_family_scope_tdd 2>&1 | tee target/test-output.log; grep -q '^test result: ok' target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS` on the starting tree.
- Postcondition: AC-1 test exists and passes; AC-2 grep passes except for `arrival_owners` (removed in Step 2).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-wasm-host/src/support_aggregation.rs` - the const/`RoutingCell`/`SupportAggregationInput` block at the top of the file, the `union_same_family_entries` body, and the `in_routing_cell` body. Re-derive line ranges from a `LOCATIONS` dispatch; do not trust ranges recorded here.
  - `crates/slicer-wasm-host/tests/unit/support_cross_family_scope_tdd.rs` - fixture-helper region only
- Files allowed to edit (at most 3):
  - `crates/slicer-wasm-host/src/support_aggregation.rs`
  - `crates/slicer-wasm-host/tests/unit/support_plan_ownership_tdd.rs` (new)
  - `crates/slicer-wasm-host/tests/unit/main.rs` (add `mod support_plan_ownership_tdd;`)
- Files explicitly out of bounds:
  - `crates/slicer-runtime/**`, `modules/**`, `docs/**`
- Blast-radius discipline: no struct field or constant added in this step.
- Expected sub-agent dispatches:
  - Question: callers of `same_body`, `routing_cell`, `RoutingCell` outside `union_same_family_entries`; scope: `crates/slicer-wasm-host/src/`; return: `LOCATIONS`
- Context cost: `S`
- Authoritative docs:
  - `docs/04_host_scheduler.md` - §"Host aggregation as the sole multi-writer merge point" (ranged)
- OrcaSlicer refs: none.
- Verification:
  - `mkdir -p target && cargo test -p slicer-wasm-host --test unit -- --exact support_plan_ownership_tdd::union_merges_same_region_entries_regardless_of_distance 2>&1 | tee target/test-output.log; grep -q '^test result: ok' target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -eq 1 && echo PASS` - FACT PASS/absent
  - `! rg -q 'RoutingCell|ROUTING_CELL_SIZE|fn routing_cell|group_cells' crates/slicer-wasm-host/src/support_aggregation.rs && rg -q 'fn in_routing_cell' crates/slicer-wasm-host/src/support_aggregation.rs && rg -q 'const MAX_BODY_EXTENT_UNITS' crates/slicer-wasm-host/src/support_aggregation.rs && echo PASS` - FACT PASS/absent
- Exit condition: the AC-1 test fails on the pre-step tree (entries far apart do not merge) and passes after; `in_routing_cell` is **behaviourally** identical — same `1 << 20` bound on both axes, now read from `MAX_BODY_EXTENT_UNITS` — and keeps its name so packet 224's RC-14 record stays resolvable. It is not byte-identical, because the surviving function reads the renamed constant.

### Step 2: Ownership check and producer identity in the host

- Task IDs: `TASK-531`
- Objective: add `SupportPlanProducer`, `SupportAggregationInput::producers`, `check_ownership`, `OwnershipViolation` / `OwnershipReason`, code `1206` (verified free; `1200`-`1205` are in use); replace `SupportAggregationError`'s fields with the ownership shape **keeping it a `pub struct`** — no enum conversion, no `ProducerCountMismatch` variant, length mismatch handled by `debug_assert_eq!` plus default-deny `MissingClaim`; delete `arrival_owners` and the cross-family duplicate branch; direct `plan_index` attribution.
- Precondition: Step 1 committed.
- Postcondition: AC-3, AC-N1, AC-N2 and `wrong_family_entry_is_a_trespass_in_both_plan_orders` pass; `arrival_owners` absent. Existing `territory: None` fixtures are expected red until Step 3.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-wasm-host/src/support_aggregation.rs` - the struct/diagnostic block near the top, the `try_aggregate_support_plans_with_policy` body, and the `aggregate_support_plan_irs_with_policy_attributed` body. Re-derive ranges from a `LOCATIONS` dispatch.
  - `crates/slicer-ir/src/slice_ir.rs` - the `SupportAnalysisIR` definition only
  - `crates/slicer-wasm-host/src/marshal/out.rs` - `enforce_authored_coloring` only
- Files allowed to edit (at most 3):
  - `crates/slicer-wasm-host/src/support_aggregation.rs`
  - `crates/slicer-wasm-host/tests/unit/support_plan_ownership_tdd.rs`
  - `crates/slicer-wasm-host/src/lib.rs` — slot reserved but expected unused: verified 2026-09-03 that `lib.rs` carries only `pub mod support_aggregation;` with no `pub use` re-export of aggregation types, so callers path through the module and `SupportPlanProducer` needs no re-export.
- Files explicitly out of bounds:
  - `crates/slicer-runtime/**`, `modules/**`, other test files (Step 3)
- Blast-radius discipline (new struct field `producers`):
  - Construction sites of `SupportAggregationInput { .. }` are inventoried by the Step 3 dispatch and edited in Step 3; this step edits only `src/support_aggregation.rs` sites. `cargo check -p slicer-wasm-host --all-targets` is expected red between Step 2 and Step 3 only on test targets.
- Expected sub-agent dispatches:
  - Question: readers of `expected_family_id` / `conflicting_family_id` outside `support_aggregation.rs`; scope: `crates/`; return: `LOCATIONS` (verified 2026-09-03: `src/lib.rs` has `pub mod support_aggregation;` and no re-export, so the third edit slot below is normally unused)
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md` - §"IR 9b — SupportPlanIR" (ranged)
  - `docs/01_system_architecture.md` - §"Claim Conflict Resolution (Normative)" (ranged)
- OrcaSlicer refs: none.
- Verification:
  - `mkdir -p target && cargo test -p slicer-wasm-host --test unit support_plan_ownership_tdd 2>&1 | tee target/test-output.log; grep -q '^test result: ok' target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -eq 4 && echo PASS` - FACT PASS/absent
  - `rg -q 'pub producers: Vec<SupportPlanProducer>' crates/slicer-wasm-host/src/support_aggregation.rs && ! rg -q 'arrival_owners' crates/slicer-wasm-host/src/support_aggregation.rs && rg -q 'pub struct SupportAggregationError' crates/slicer-wasm-host/src/support_aggregation.rs && echo PASS` - FACT PASS/absent
- Exit condition: `unassigned_region_entry_is_a_trespass` asserts `degraded == true`, one `ownership_violations` element with `reason == NoAssignment`, and `Err(SupportAggregationError { reason: OwnershipReason::NoAssignment, .. })` under `Fail`; `producer_without_family_claim_is_a_trespass` asserts `reason == MissingClaim { required: "support-family:tree" }` and `plan_index == Some(1)`; both fail to compile or fail on the Step 1 tree.

### Step 3: Fixture fallout and prepass call site

- Task IDs: `TASK-531`
- Objective: give every existing aggregation fixture `family_assignments` plus `producers`; rewrite `support_plan_aggregation_diagnoses_duplicate_identity` to assignment-based ownership in both orders; thread `SupportPlanProducer` through `prepass.rs`.
- Precondition: Step 2 committed; `LOCATIONS` inventory of `SupportAggregationInput { .. }` sites in hand.
- Postcondition: AC-4, AC-5, AC-8 pass; `cargo check --workspace --all-targets` green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/prepass.rs` - the `support_plans` / `support_plan_audits` declarations, the `for module in &stage.modules` loop that pushes into them, and the aggregation call site. Re-derive ranges from a `LOCATIONS` dispatch.
  - `crates/slicer-scheduler/src/execution_plan.rs` - `CompiledModuleStatic::module_id()` and `CompiledModuleStatic::claims()` only (`claims()` returns `&[String]`, so the producer needs `.to_vec()`)
  - each fixture file from the inventory, helper region only
- Files allowed to edit (at most 3 per sub-step; execute as 3a/3b/3c):
  - 3a: `crates/slicer-runtime/src/prepass.rs`
  - 3b: `crates/slicer-wasm-host/tests/contract/support_plan_validation.rs`, `crates/slicer-wasm-host/tests/contract/support_decline_contract.rs`, `crates/slicer-wasm-host/tests/unit/support_cross_family_scope_tdd.rs`
  - 3c: `crates/slicer-runtime/tests/integration/support_family_routing.rs`, `crates/slicer-runtime/tests/integration/support_family_closure.rs`
- Files explicitly out of bounds:
  - `crates/slicer-runtime/src/run.rs`, `modules/**`, `docs/**`
- Blast-radius discipline (struct field added in Step 2):
  - Dispatch before authoring: Question: every `SupportAggregationInput {` literal under `crates/` and `modules/`; scope: `crates/ modules/`; return: `LOCATIONS`. Measured 2026-09-03: `src/support_aggregation.rs` (2 internal), `tests/contract/support_plan_validation.rs` (5), `tests/unit/support_cross_family_scope_tdd.rs` (6), `crates/slicer-runtime/tests/integration/support_family_routing.rs` (4); no other files. `support_decline_contract.rs` and `support_family_closure.rs` are run, not necessarily edited. Test literals must use `..` per `cargo xtask check-literals`.
- Expected sub-agent dispatches:
  - the inventory above; return: `LOCATIONS`
- Context cost: `M`
- Authoritative docs:
  - `docs/21_data_defaults_and_fixtures.md` - struct-literal rule (ranged)
- OrcaSlicer refs: none.
- Verification:
  - `mkdir -p target && cargo test -p slicer-wasm-host --test contract support_plan_ 2>&1 | tee target/test-output.log; grep -q '^test result: ok' target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS` - FACT PASS/absent
  - `mkdir -p target && cargo test -p slicer-wasm-host --test unit support_cross_family_scope_tdd 2>&1 | tee target/test-output.log; grep -q '^test result: ok' target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS` - FACT PASS/absent
  - `mkdir -p target && cargo test -p slicer-runtime --test integration 2>&1 | tee target/test-output.log; grep -q '^test result: ok' target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 30 && echo PASS` - FACT PASS/absent. **Never** filter this binary on `support_family_`: the closure cases are top-level wrappers in `tests/integration/main.rs` with no module prefix, so that filter silently skips ~27 of them.
  - `cargo check --workspace --all-targets` - FACT pass/fail (also compiles the second `support_family_routing` test target)
  - `cargo xtask check-literals; echo EXIT=$?` - FACT exit code
- Exit condition: `support_plan_aggregation_diagnoses_duplicate_identity` asserts `retained[0].family_id == "traditional"` for both `[tree, traditional]` and `[traditional, tree]` plan orders with the region assigned to `traditional`, and one `ownership_violations` element with `family_id == "tree"`; the routing test formerly named `routing_cells` is renamed `declared_identity_is_input_order_independent` and keeps its equality assertion.

### Step 4: Producer invariant in the traditional planner (W4, W5, W6)

- Task IDs: `TASK-531`
- Objective: make `merge_region_identity_entries` return an error on anchor/layer disagreement, rewrite its doc comment as the DEV-167 fix, and restore the two packet-239 tests against the one-entry-per-plane shape.
- Precondition: Step 3 committed.
- Postcondition: AC-6, AC-7, AC-N3 pass; `cargo xtask build-guests --check` exit 0.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/traditional-support-planner/src/lib.rs` - the `plan_for_object` body and the `merge_region_identity_entries` doc comment plus body. Re-derive ranges from a `LOCATIONS` dispatch. Note the merge is a free fn returning unit today, called from `plan_for_object` immediately after an `emit_coarse_entries(...)?`; its doc comment's closing paragraph describes the `ROUTING_CELL_SIZE` extent bound and must be updated to `MAX_BODY_EXTENT_UNITS`.
  - `modules/core-modules/traditional-support-planner/tests/traditional_family_tdd.rs` - the two `coarse_*` target tests only; re-derive their ranges.
- Files allowed to edit (at most 3):
  - `modules/core-modules/traditional-support-planner/src/lib.rs`
  - `modules/core-modules/traditional-support-planner/tests/traditional_family_tdd.rs`
- Files explicitly out of bounds:
  - `crates/**`, `modules/core-modules/tree-support-planner/**`, `docs/**`
- Blast-radius discipline: no struct field added.
- Expected sub-agent dispatches:
  - Question: current line ranges of `plan_for_object`, `merge_region_identity_entries` (definition + doc comment), and the two `coarse_*` tests; scope: `modules/core-modules/traditional-support-planner/`; return: `LOCATIONS`
  - (`plan_for_object -> Result<(), ModuleError>` verified 2026-09-03; do **not** cite `next_intermediate_plane_index` as the in-function `?` precedent — it is used elsewhere in the file.)
- Context cost: `S`
- Authoritative docs:
  - `docs/02_ir_schemas.md` - §"IR 9b — SupportPlanIR" (ranged)
- OrcaSlicer refs: none.
- Verification:
  - `mkdir -p target && cargo test -p traditional-support-planner --test traditional_family_tdd 2>&1 | tee target/test-output.log; grep -q '^test result: ok' target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 10 && echo PASS` - FACT PASS/absent
  - `mkdir -p target && cargo test -p traditional-support-planner --test traditional_family_tdd -- --exact merge_rejects_anchor_z_layer_index_disagreement 2>&1 | tee target/test-output.log; grep -q '^test result: ok' target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -eq 1 && echo PASS` - FACT PASS/absent
  - `cargo xtask build-guests --check; echo EXIT=$?` - FACT exit code
- Exit condition: `coarse_same_region_sources_keep_distinct_body_membership` asserts `same_plane.len() == 1`, `body_ids` sorted equals both source ids, and that the shoelace area summed over the merged entry's `roles[].regions` equals the union area of the two source entries' regions (`SupportPlanEntry` has no contour field; geometry lives under `roles`, and roles merge by matching `role`, so the helper must sum across all roles). The local shoelace `polygon_area` helper is added to the test file in this step; none exists there today. `coarse_source_preference_keeps_mixed_source_memberships` asserts `synthesized.len() == 1` with `body_ids == [body_id, interface_only_id]` sorted; `merge_rejects_anchor_z_layer_index_disagreement` feeds two hand-built entries with equal triple and `anchor_z` 10000 vs 12000 and asserts `Err`. Both `coarse_*` tests already exist unignored in the file — this step rewrites their bodies, not their names.

### Step 5: Tree planner self-default removal (W3c)

- Task IDs: `TASK-531`
- Objective: delete the `assignments_empty` / `fallback_family_emitted` self-default so empty `family_assignments` emits nothing. **Three** edit sites, not two, all inside one long function and separated by roughly two thousand lines: the `let mut fallback_family_emitted = false;` declaration, the `assignments_empty` region-loop block that ORs into it, and the trailing `if fallback_family_emitted { push_diagnostic(code 1004, "no family assignments; using configured support family") }` block. The step's own `! rg -q 'assignments_empty|fallback_family_emitted'` gate requires all three to go.
- Precondition: Step 4 committed; `LOCATIONS` for the two fallback sites and any tree tests relying on them in hand.
- Postcondition: AC-N4 passes; `tree_family_tdd` binary `0 failed`; guests fresh.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/tree-support-planner/src/lib.rs` - only the ranges returned by the dispatch. There are three `assignments_empty` / `fallback_family_emitted` sites, widely separated; the earlier draft of this plan named a single window plus a second window that actually belongs to `candidate_family`, which would have missed two of the three. Take every range from the dispatch, never from this file.
  - `modules/core-modules/tree-support-planner/tests/tree_family_tdd.rs` - helper region and any test named by the dispatch. At least one existing test (`staggered_region_runs_do_not_pair_across_region_boundaries`) builds `family_assignments: vec![]` and relies on the native fallback, so it will need updating.
- Files allowed to edit (at most 3):
  - `modules/core-modules/tree-support-planner/src/lib.rs`
  - `modules/core-modules/tree-support-planner/tests/tree_family_tdd.rs`
- Files explicitly out of bounds:
  - `crates/**`, `modules/core-modules/traditional-support-planner/**`, `docs/**`
- Blast-radius discipline: no struct field added.
- Expected sub-agent dispatches:
  - Question: **every** line occurrence of `assignments_empty` and `fallback_family_emitted` (expect three sites, including the `let mut` declaration and the trailing code-1004 diagnostic block) plus the enclosing function's bounds; scope: `modules/core-modules/tree-support-planner/src/lib.rs`; return: `LOCATIONS`
  - Question: tests constructing empty `family_assignments` that expect entries; scope: `modules/core-modules/tree-support-planner/tests/`; return: `LOCATIONS`
  - Question: can the 239c off-grid path emit two entries with equal `(global_layer_index, object_id, region_id)` given `covered_regions` omits `anchor_z`?; scope: `modules/core-modules/tree-support-planner/src/lib.rs` lines `3690-3800`; return: `FACT`
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-families-anchored-entities-plan.md` - delegated SUMMARY of Ruling 1
- OrcaSlicer refs: none.
- Verification:
  - `mkdir -p target && cargo test -p tree-support-planner --test tree_family_tdd 2>&1 | tee target/test-output.log; grep -q '^test result: ok' target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS` - FACT PASS/absent
  - `mkdir -p target && cargo test -p tree-support-planner --test tree_family_tdd -- --exact empty_family_assignments_emit_nothing 2>&1 | tee target/test-output.log; grep -q '^test result: ok' target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -eq 1 && echo PASS` - FACT PASS/absent
  - `cargo xtask build-guests --check; echo EXIT=$?` - FACT exit code
  - `! rg -q 'assignments_empty|fallback_family_emitted' modules/core-modules/tree-support-planner/src/lib.rs && echo PASS` - FACT PASS/absent
- Exit condition: `empty_family_assignments_emit_nothing` builds a planner input with a populated region map and `family_assignments` empty and asserts `output.entries().is_empty()`; it fails on the Step 4 tree.

### Step 6: Docs, deviation, registration (W7)

- Task IDs: `TASK-531`
- Objective: apply the doc edits in `packet.spec.md` §Doc Impact, close DEV-167, file the ADR-0059 amendment deviation, add the ADR-0059 `Ruling 2`, add the `PerRegionClaimConflicts` doc comment, record the inert paint/tool axis in `docs/07`, close `TASK-531`, and repair the `TASK-531..TASK-535` double-allocation carried by Packet Queue rows 7a/7b and the 240a/240b frontmatter.
- Precondition: Steps 1-5 committed and green; packet 241 AC-N2 command re-derived from `docs/spec_packets/241-support-agg-rasterizer/packet.spec.md` via a `SNIPPETS` dispatch.
- Postcondition: AC-9, AC-10 pass; packet 241 AC-N2 command returns green.
- Files allowed to read, with ranges when over 300 lines (re-derive every range at read time; navigate by section heading, not by the numbers here):
  - `docs/02_ir_schemas.md` - §"Modifier Resolution Contract", §"Config Precedence Rules" and the IR 3 merging sentence, §"IR 9 — SupportIR" routing-cell ownership paragraph, §"IR 9b — SupportPlanIR"
  - `docs/01_system_architecture.md` - §"Claim Conflict Resolution (Normative)" (step 4 reads "Validate uniqueness for every `(layer, object, region, claim)`")
  - `docs/04_host_scheduler.md` - the §"Host aggregation as the sole multi-writer merge point" bullet **and** the following §"Complete-body validation" bullet
  - `docs/adr/0059-support-families-and-anchored-entities.md` - short; read in full
  - `docs/DEVIATION_LOG.md` - DEV-167 row only, plus the tail needed to re-derive the next free `DEV-###`
- Files allowed to edit (execute as 6a/6b/6c/6d, at most 3 each):
  - 6a: `docs/02_ir_schemas.md`, `docs/01_system_architecture.md`, `docs/04_host_scheduler.md`
  - 6b: `docs/adr/0059-support-families-and-anchored-entities.md` (append `Ruling 2` under `## Amendments`; decision paragraphs and Ruling 1 stay byte-identical), `docs/DEVIATION_LOG.md` (close DEV-167 **and** add the ADR-0059 amendment row), `crates/slicer-scheduler/src/validation.rs` (doc comment only)
  - 6c: `docs/07_implementation_status.md` (via worker dispatch), `docs/specs/support-families-anchored-entities-plan.md` (queue row 10 status **and** the rows 7a/7b `TASK-531..TASK-535` reservation repair)
  - 6d: `docs/spec_packets/240a-support-raft-substrate/packet.spec.md`, `docs/spec_packets/240b-support-raft-module/packet.spec.md` — shift their unregistered `task_ids` reservations off `TASK-531` so only the collided id moves. Both are `draft`; re-derive the free block from `docs/07_implementation_status.md` at edit time rather than trusting any number written here. This is the sole exception to the "never edit another packet's files" rule and exists only because those reservations collide with this packet's registered id.
- Files explicitly out of bounds:
  - `docs/spec_packets/241-support-agg-rasterizer/**`, all `src/` other than the one doc comment
- Blast-radius discipline: none.
- Expected sub-agent dispatches:
  - Question: highest `TASK-###` and `DEV-###` now (`rg -o '^\| DEV-[0-9]{3}' docs/DEVIATION_LOG.md | sort -u | tail -1`); scope: `docs/07_implementation_status.md`, `docs/DEVIATION_LOG.md`; return: `FACT`
  - Question: the exact AC-N2 verification command in packet 241; scope: `docs/spec_packets/241-support-agg-rasterizer/packet.spec.md`; return: `SNIPPETS`
  - Question: every remaining occurrence of `routing[ -]cell` (case-insensitive) in `docs/02_ir_schemas.md` and `docs/04_host_scheduler.md` after the 6a edits; scope: those two files; return: `LOCATIONS` — AC-9 bans all of them, and one hit in each file sits outside the primary section being rewritten.
- Context cost: `S`
- Authoritative docs: the five files above, ranged.
- OrcaSlicer refs: none.
- Verification:
  - the AC-9, AC-10 and AC-11 commands verbatim from `packet.spec.md` - FACT PASS/absent
  - packet 241's AC-N2 command as returned by the dispatch - FACT pass/fail
  - `! rg -q 'TASK-531\.\.TASK-534' docs/specs/support-families-anchored-entities-plan.md && ! rg -q 'TASK-531' docs/spec_packets/240a-support-raft-substrate/packet.spec.md && echo PASS` - FACT PASS/absent
  - `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail
- Exit condition: all greps PASS; DEV-167 Status reads `Closed — packet 241b-support-plan-ownership-seam` and states that `merge_region_identity_entries` is retained (reversing the row's earlier "removal" wording); a new deviation row names the ADR-0059 amendment; ADR-0059 carries a dated `Ruling 2` that quotes "assigns deterministic routing cells" and carries invariant 15 in full, with its decision paragraphs and Ruling 1 byte-identical; the new `docs/07` row for the paint/tool axis cites "no shipped manifest declares `[[region_split]]`"; and `TASK-531` is claimed by this packet alone.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | one function, one new test file |
| Step 2 | M | new types, diagnostics, arrival-order removal |
| Step 3 | M | ~22 fixture edits across 5 files plus prepass; split 3a/3b/3c |
| Step 4 | S | guest edit; freshness check mandatory |
| Step 5 | S | guest edit; freshness check mandatory; three edit sites in one long fn |
| Step 6 | M | docs, ADR amendment, two deviation rows, ledgers, task-id repair; split 6a/6b/6c/6d |

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- Reconcile reopened/superseded status transitions (none; packet 241 stays `implemented`).
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check` and `cargo clippy` gate invocations must use `--all-targets` so the test, bench, and example targets compile. `cargo test --test <bin>` invocations deliberately do not — `--all-targets` and `--test` are mutually exclusive in intent; the `--all-targets` gates are what prove the narrow test binaries still build. Every `cargo test` invocation tees to `target/test-output.log` and asserts a non-zero matched test count.
