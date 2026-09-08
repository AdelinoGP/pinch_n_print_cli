---
status: implemented
packet: 241b-support-plan-ownership-seam
task_ids:
  - TASK-531
depends_on: 241-support-agg-rasterizer
backlog_source: docs/specs/support-families-anchored-entities-plan.md
context_cost_estimate: M
---

# Packet Contract: 241b-support-plan-ownership-seam

## Goal

Make support-region ownership enforced at the single host merge point: `union_same_family_entries` keys on declared `(family_id, global_layer_index, object_id, region_id, anchor_z)` instead of a bbox-centroid grid cell, every `SupportPlanEntry` is checked against `SupportAnalysisIR::family_assignments` and its producing module's `support-family:<id>` claim (default-deny), arrival-order arbitration is deleted, the traditional planner's per-triple merge becomes the DEV-167 fix with an `anchor_z`↔`global_layer_index` consistency check, and the two packet-239 tests are restored against the one-entry-per-triple shape.

## Scope Boundaries

Host side: `crates/slicer-wasm-host/src/support_aggregation.rs` (merge key, ownership check, `SupportAggregationInput` producer identity) and its call site in `crates/slicer-runtime/src/prepass.rs`. Guest side: `modules/core-modules/traditional-support-planner` (merge invariant, two tests) and the `assignments_empty` self-default in `modules/core-modules/tree-support-planner`. Docs: the W7 text defects, the routing-cell prose in **both** the `docs/04_host_scheduler.md` aggregation and complete-body-validation bullets, the routing-cell ownership paragraph in `docs/02_ir_schemas.md` §"IR 9 — SupportIR", and an ADR-0059 `Ruling 2` amendment for the deleted routing-cell mechanism. Everything else, including the schedule-time claim pass and the inert paint/tool config axis, is recorded, not changed. Full lists live in `requirements.md`.

## Prerequisites and Blockers

- Depends on: `241-support-agg-rasterizer` (frontmatter `status: implemented`, closed by human override with AC-N2 red; DEV-167 `Open — target close: packet 241b`). Note that packet 241's own body and the DEV-166/DEV-167 rows describe it as closing "NARROW and NOT GREEN with `status: draft`"; the frontmatter is authoritative for the dependency check, and this packet does not reopen or edit packet 241.
- Unblocks: packet 241 AC-N2 turning green; `TASK-352` green gate for the support family.
- Activation blockers: none recorded. Grilling decisions (2026-09-03) and the two preflight design rulings (2026-09-03: `MAX_BODY_EXTENT_UNITS` rename; `SupportAggregationError` stays a struct) are locked in `design.md` §Locked Assumptions.
- Task-ID note: `docs/07_implementation_status.md` binds `TASK-531` to this packet. The Packet Queue rows 7a/7b of the backlog source carried an unregistered forward reservation of `TASK-531..TASK-535` for packets 240a/240b; Step 6c repairs those rows so only the collided id moves.

## Acceptance Criteria

- **AC-1. Given** two same-family entries with the same `(global_layer_index, object_id, region_id, anchor_z)`, disjoint `body_ids`, and bounding boxes more than `2_000_000` units apart, **when** `try_aggregate_support_plans_with_policy` runs with `family_assignments` granting that region to the family, **then** `retained.len() == 1` and the retained `body_ids` contains both source ids. | `mkdir -p target && cargo test -p slicer-wasm-host --test unit -- --exact support_plan_ownership_tdd::union_merges_same_region_entries_regardless_of_distance 2>&1 | tee target/test-output.log; grep -q '^test result: ok' target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -eq 1 && echo PASS`
- **AC-2. Given** the host aggregation source, **when** the merge key is inspected, **then** `RoutingCell`, `ROUTING_CELL_SIZE`, `fn routing_cell`, `group_cells`, and `arrival_owners` are absent, while `fn in_routing_cell` remains and its extent bound reads from the renamed constant `MAX_BODY_EXTENT_UNITS`. | `! rg -q 'RoutingCell|ROUTING_CELL_SIZE|fn routing_cell|group_cells|arrival_owners' crates/slicer-wasm-host/src/support_aggregation.rs && rg -q 'fn in_routing_cell' crates/slicer-wasm-host/src/support_aggregation.rs && rg -q 'const MAX_BODY_EXTENT_UNITS' crates/slicer-wasm-host/src/support_aggregation.rs && echo PASS`
- **AC-3. Given** `SupportAggregationInput`, **when** its definition is inspected, **then** it carries a `producers: Vec<SupportPlanProducer>` field index-parallel to `plans`, and `SupportPlanProducer` has `module_id: String` and `claims: Vec<String>`. | `rg -q 'pub producers: Vec<SupportPlanProducer>' crates/slicer-wasm-host/src/support_aggregation.rs && rg -q 'pub struct SupportPlanProducer' crates/slicer-wasm-host/src/support_aggregation.rs && rg -q 'pub module_id: String' crates/slicer-wasm-host/src/support_aggregation.rs && rg -q 'pub claims: Vec<String>' crates/slicer-wasm-host/src/support_aggregation.rs && echo PASS`
- **AC-4. Given** the prepass support-plan collection loop, **when** plans are buffered, **then** each buffered plan is paired with a `SupportPlanProducer` built from `module.module_id()` and `module.claims()`. | `rg -q 'SupportPlanProducer' crates/slicer-runtime/src/prepass.rs && rg -q 'module\.claims\(\)' crates/slicer-runtime/src/prepass.rs && echo PASS`
- **AC-5. Given** a tree entry and a traditional entry sharing one identity triple where `family_assignments` grants the region to `traditional`, **when** the plans are aggregated in both plan orders, **then** the retained entry is `traditional` in both orders and one ownership diagnostic names `tree` as the trespasser. | `mkdir -p target && cargo test -p slicer-wasm-host --test contract -- --exact support_plan_validation::support_plan_aggregation_diagnoses_duplicate_identity 2>&1 | tee target/test-output.log; grep -q '^test result: ok' target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -eq 1 && echo PASS`
- **AC-6. Given** the traditional planner emits two candidates in one region on one plane, **when** `merge_region_identity_entries` runs, **then** exactly one entry exists at that `anchor_z` whose `body_ids` holds both memberships and whose summed `roles[].regions` area (shoelace, computed by a local test helper) equals the union area of both source contours. | `mkdir -p target && cargo test -p traditional-support-planner --test traditional_family_tdd -- --exact coarse_same_region_sources_keep_distinct_body_membership coarse_source_preference_keeps_mixed_source_memberships 2>&1 | tee target/test-output.log; grep -q '^test result: ok' target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -eq 2 && echo PASS`
- **AC-7. Given** the traditional planner test binary after W6, **when** the whole binary runs, **then** it reports `0 failed` over more than ten passing tests (packet 241 AC-N2 turns green under that packet's own command shape). | `mkdir -p target && cargo test -p traditional-support-planner --test traditional_family_tdd 2>&1 | tee target/test-output.log; grep -q '^test result: ok' target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 10 && echo PASS`
- **AC-8. Given** the routing and closure integration suites updated to supply `family_assignments`, **when** the whole `integration` binary runs, **then** it reports `0 failed`, more than thirty passing tests, and the run names both the renamed routing test and a closure wrapper. | `mkdir -p target && cargo test -p slicer-runtime --test integration 2>&1 | tee target/test-output.log; grep -q '^test result: ok' target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 30 && grep -q '^test support_family_routing::declared_identity_is_input_order_independent' target/test-output.log && grep -q '^test fixture_invariants' target/test-output.log && echo PASS`
- **AC-9. Given** the doc edits, **when** the anchors are grepped, **then** each edited section carries its new text and no doc in the edit surface still describes routing-cell ownership. | `rg -q 'support-family:<family_id>' docs/02_ir_schemas.md && ! rg -q 'stamp_modifier_config_deltas' docs/02_ir_schemas.md && ! rg -q 'layer-range override' docs/02_ir_schemas.md && ! rg -qi 'routing[ -]cell' docs/02_ir_schemas.md && ! rg -qi 'routing[ -]cell' docs/04_host_scheduler.md && rg -q 'support-plan commit seam' docs/01_system_architecture.md && rg -q 'exactly one attributed plan entry' docs/adr/0059-support-families-and-anchored-entities.md && echo PASS`
- **AC-10. Given** `docs/DEVIATION_LOG.md`, **when** DEV-167 is read, **then** its Status column reads `Closed` and names this packet. | `rg -q '^\| DEV-167 .*\| Closed[^|]*241b' docs/DEVIATION_LOG.md && echo PASS`
- **AC-11. Given** the packet deletes the routing-cell mechanism that ADR-0059's decision text names, **when** the ADR and the deviation log are read, **then** ADR-0059 carries a dated `Ruling 2` amendment quoting the superseded clause, and the deviation log carries a row naming ADR-0059. | `rg -q 'Ruling 2:.*assigns deterministic routing cells' docs/adr/0059-support-families-and-anchored-entities.md && rg -q 'ADR-0059' docs/DEVIATION_LOG.md && echo PASS`

Every AC names exact fields, paths, counts, errors, variants, or output fragments and ends with its own runnable command. Repeat shared commands; never write "see AC-N". Commands that dump more than 200 successful output lines must be wrapped or filtered so a subagent can return a FACT.

**Test-command rules (mandatory — `docs/specs/support-families-anchored-entities-plan.md` §6 invariant 16, and CLAUDE.md §"Test output must always tee").**

1. **No acceptance command may match zero tests.** Every `cargo test` AC above passes `--exact` test names or asserts a matched count in the same run (`-eq N` / `-gt N`). A bare `grep -q ' 0 failed'` is forbidden — `test result: ok. 0 passed; 0 failed` satisfies it and reads green.
2. **Every `cargo test` invocation tees to `target/test-output.log`** and results are read from that file. Never re-run a test to see more output.
3. **Never filter the `integration` binary on `support_family_`.** `support_family_closure`'s cases are plain fns wrapped by top-level `#[test]` fns in `crates/slicer-runtime/tests/integration/main.rs` that carry no module prefix, so that filter matches only 1 of ~28 closure tests. This is the packet-224 lesson that produced invariant 16; AC-8 therefore runs the whole binary.

AC verification command rule (mandatory): each pipe-suffixed command's `--test` binary must be one that can actually drive the asserted behavior. `slicer-wasm-host --test unit` and `--test contract` already drive `try_aggregate_support_plans` / `aggregate_support_plans` with inline `SupportAnalysisIR` fixtures (`support_cross_family_scope_tdd.rs`, `support_plan_validation.rs`); `traditional-support-planner --test traditional_family_tdd` already drives `plan_for_object` natively (it has no polygon-area helper today; AC-6 adds a local shoelace helper in the same step); `slicer-runtime --test integration` already drives host aggregation in `support_family_routing.rs`. No new driver is required. None of these targets carries `required-features`, so a bare `-p <crate> --test <bin>` run compiles them. Note that `crates/slicer-runtime/Cargo.toml` also declares a second `[[test]] name = "support_family_routing"` over the same file, so Step 3c's edits compile into two binaries; AC-8 verifies the `integration` one and `cargo check --workspace --all-targets` covers the other.

## Negative Test Cases

- **AC-N1. Given** `family_assignments` has no row for `(object_id, region_id)`, **when** an entry for that region is aggregated under `FamilyConflictPolicy::Degrade`, **then** it is dropped, `degraded == true`, and a diagnostic with code `1206` names the entry's `family_id` and region; under `FamilyConflictPolicy::Fail` the call returns `Err(SupportAggregationError { reason: OwnershipReason::NoAssignment, .. })`. | `mkdir -p target && cargo test -p slicer-wasm-host --test unit -- --exact support_plan_ownership_tdd::unassigned_region_entry_is_a_trespass 2>&1 | tee target/test-output.log; grep -q '^test result: ok' target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -eq 1 && echo PASS`
- **AC-N2. Given** the producer of a `family_id == "tree"` entry has `claims` lacking `support-family:tree`, **when** aggregated under `Degrade` with the region assigned to `tree`, **then** the entry is dropped with code `1206` and the diagnostic's `plan_index` resolves to that producer. | `mkdir -p target && cargo test -p slicer-wasm-host --test unit -- --exact support_plan_ownership_tdd::producer_without_family_claim_is_a_trespass 2>&1 | tee target/test-output.log; grep -q '^test result: ok' target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -eq 1 && echo PASS`
- **AC-N3. Given** the traditional planner would publish two entries with equal `(global_layer_index, object_id, region_id)` but different `anchor_z`, or equal `(object_id, region_id, anchor_z)` but different `global_layer_index`, **when** `merge_region_identity_entries` runs, **then** it returns the module's error path instead of publishing. | `mkdir -p target && cargo test -p traditional-support-planner --test traditional_family_tdd -- --exact merge_rejects_anchor_z_layer_index_disagreement 2>&1 | tee target/test-output.log; grep -q '^test result: ok' target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -eq 1 && echo PASS`
- **AC-N4. Given** `family_assignments` is empty, **when** the tree planner plans, **then** it emits zero entries and sets no `fallback_family_emitted` self-default. | `mkdir -p target && cargo test -p tree-support-planner --test tree_family_tdd -- --exact empty_family_assignments_emit_nothing 2>&1 | tee target/test-output.log; grep -q '^test result: ok' target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -eq 1 && echo PASS`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask check-literals; echo EXIT=$?`
- `cargo xtask build-guests --check; echo EXIT=$?`
- `mkdir -p target && cargo test -p slicer-wasm-host --test unit support_plan_ownership_tdd 2>&1 | tee target/test-output.log; grep -q '^test result: ok' target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -eq 4 && echo PASS`

## Authoritative Docs

- `docs/02_ir_schemas.md` - long; ranged reads only: §"IR 9 — SupportIR" (the routing-cell ownership paragraph), §"IR 9b — SupportPlanIR", §"Modifier Resolution Contract", §"Config Precedence Rules".
- `docs/04_host_scheduler.md` - long; ranged read of the §"Host aggregation as the sole multi-writer merge point" bullet **and** the following §"Complete-body validation" bullet — both name routing cells.
- `docs/01_system_architecture.md` - long; ranged read of §"Claim Conflict Resolution (Normative)".
- `docs/specs/support-families-anchored-entities-plan.md` - delegate SUMMARY of §6 invariants 15 **and 16**, the AC-8 Ruling 1, and the Packet Queue.
- `docs/adr/0059-support-families-and-anchored-entities.md` - short; read in full.

## Doc Impact Statement (Required)

- `docs/02_ir_schemas.md` §"IR 9b — SupportPlanIR" - add the ownership sentence (owner = `family_assignments`; producer must hold `support-family:<family_id>`), written on a single source line so a line-oriented `rg` can match it - `rg -q 'support-family:<family_id>' docs/02_ir_schemas.md`
- `docs/02_ir_schemas.md` §"IR 9 — SupportIR" - replace the routing-cell ownership paragraph; it describes both the deleted routing cells and the deleted cross-family arrival-order branch - `! rg -qi 'routing[ -]cell' docs/02_ir_schemas.md`
- `docs/02_ir_schemas.md` §"Config Precedence Rules" and the IR 3 merging sentence - remove the `layer-range override` level (all three occurrences) and point to §"Modifier Resolution Contract" - `! rg -q 'layer-range override' docs/02_ir_schemas.md`
- `docs/02_ir_schemas.md` §"Modifier Resolution Contract" - rename the stale `stamp_modifier_config_deltas` reference to the real symbol `stamp_modifier_sub_region_configs` (`crates/slicer-core/src/algos/region_mapping.rs`) - `! rg -q 'stamp_modifier_config_deltas' docs/02_ir_schemas.md`
- `docs/01_system_architecture.md` §"Claim Conflict Resolution (Normative)" step 4 - state that per-region uniqueness for support is enforced at the support-plan commit seam and that production constructs only `ConflictScope::Global` holders - `rg -q 'support-plan commit seam' docs/01_system_architecture.md`
- `docs/04_host_scheduler.md` §"Host aggregation as the sole multi-writer merge point" **and** §"Complete-body validation" - replace routing-cell prose in both bullets; the second must describe `in_routing_cell` as a max-body-extent bound, not a cell assignment - `! rg -qi 'routing[ -]cell' docs/04_host_scheduler.md`
- `docs/adr/0059-support-families-and-anchored-entities.md` `## Amendments` - **add** (not restore) a dated `Ruling 2`, written as a **single source line** beginning `Ruling 2:` and matching Ruling 1's one-line style, that quotes the superseded decision clause "assigns deterministic routing cells" on that same line, records that aggregation now keys on declared identity, and states that every RegionMap region has exactly one attributed plan entry with candidate-less regions carrying a structured no-work/declined record per Ruling 1. The line must be unwrapped because the AC grep is line-oriented. Existing decision paragraphs and Ruling 1 stay byte-identical - `rg -q 'Ruling 2:.*assigns deterministic routing cells' docs/adr/0059-support-families-and-anchored-entities.md && rg -q 'exactly one attributed plan entry' docs/adr/0059-support-families-and-anchored-entities.md`
- `docs/DEVIATION_LOG.md` DEV-167 - Status `Closed`, this packet - `rg -q '^\| DEV-167 .*\| Closed[^|]*241b' docs/DEVIATION_LOG.md`
- `docs/DEVIATION_LOG.md` - new row (ID re-derived at Step 6 via `rg -o '^\| DEV-[0-9]{3}' docs/DEVIATION_LOG.md | sort -u | tail -1`) recording the ADR-0059 amendment - `rg -q 'ADR-0059' docs/DEVIATION_LOG.md`
- `docs/07_implementation_status.md` - close `TASK-531`; add one open row recording the inert paint/tool config axis (evidence: no shipped manifest declares `[[region_split]]` — the only TOML hits are scheduler test fixtures under `crates/slicer-scheduler/tests/fixtures/region_split_manifests/` — so `paint_config:*` / `tool_config:*` overlays never fire) - `rg -q 'region_split.*paint_config' docs/07_implementation_status.md`
- `docs/specs/support-families-anchored-entities-plan.md` Packet Queue - set row 10 status, and repair the `TASK-531` double-allocation in rows 7a/7b - `! rg -q 'TASK-531\.\.TASK-534' docs/specs/support-families-anchored-entities-plan.md`

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
