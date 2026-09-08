# Requirements: 241b-support-plan-ownership-seam

## Packet Metadata

- Grouped task IDs: `TASK-531` (umbrella row, minted 2026-09-03 from the `TASK-530` high-water mark; re-derive before editing `docs/07`). **Collision repaired at preflight:** the backlog source's Packet Queue rows 7a/7b reserved `TASK-531..TASK-535` for packets 240a/240b, but those ids were never registered in `docs/07_implementation_status.md`, which binds `TASK-531` to this packet. Step 6c shifts the 240a/240b reservations up by one so only the collided id moves; re-derive the free block at edit time rather than trusting these numbers.
- Backlog source: `docs/specs/support-families-anchored-entities-plan.md` (Packet Queue row 10)
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Packet 241 removed the DEV-166 clamp from the `agg` rasterizer and the resulting geometry tripped `SupportPlanIR::duplicate_region_identity` at commit. The instrumented root cause (DEV-167) is older than 241: `SupportPlanner::plan_for_object` publishes one `SupportPlanEntry` per candidate per layer, and the host `union_same_family_entries` had been hiding that by merging on a bbox-centroid grid cell (`ROUTING_CELL_SIZE = 1 << 20`) rather than on `region_id`. Three centroids straddling a cell boundary stopped merging and the invariant fired. The same grid line already caused packet 224's RC-14 defect on the `in_routing_cell` path.

Investigation exposed a wider gap: region ownership at the support-plan seam is not enforced anywhere. `entry.family_id` is self-declared by the guest and never checked against `SupportAnalysisIR::family_assignments` or the producer's manifest claims; cross-family duplicates are resolved by arrival order `(plan_index, entry_index)`; the schedule-time `PerRegionClaimConflicts` pass runs on an empty set because `crates/slicer-runtime/src/run.rs` builds only `ConflictScope::Global` holders and has no region ids at startup. Grilling on 2026-09-03 settled the shape: enforce ownership once, at the host merge point, keyed by declared identity and `family_assignments`; retire arrival order; keep the producer merge as the DEV-167 fix; restore the packet-239 tests against the one-entry-per-triple shape.

Packet 241 stays `implemented`; this packet turns its AC-N2 green rather than reopening it.

## In Scope

- **W2 — declared merge key.** `union_same_family_entries` merges on `family_id`, `global_layer_index`, `object_id`, `region_id`, `anchor_z`. Delete `RoutingCell`, `routing_cell`, `group_cells` (a local `Vec<Option<RoutingCell>>`, not a fn), and `same_body`. Keep `fn in_routing_cell` under its current name — packet 224's RC-14 record and the traditional planner's `merge_region_identity_entries` doc comment both reference it. **`ROUTING_CELL_SIZE` cannot simply be deleted:** `in_routing_cell` reads it twice as its bbox extent bound. Ruling (2026-09-03 preflight): rename the constant to `MAX_BODY_EXTENT_UNITS`, same `1 << 20` value, and update the two uses. `in_routing_cell` is therefore behaviourally identical (same numeric bound), not byte-identical. Add the first test of the non-`same_body` merge branch.
- **W3 — ownership check (default-deny).** New `SupportPlanProducer { module_id, claims }` and `SupportAggregationInput::producers` index-parallel to `plans`. For every entry: owner family = `territory.family_assignments.get(&(object_id, region_id))`; trespass when no row, when owner != `entry.family_id`, or when `producers[plan_index].claims` lacks `support-family:<entry.family_id>`. `Degrade` drops the entry and pushes an ownership diagnostic (new code `1206`; codes `1200`-`1205` are taken; `SupportAggregationResult` gets an `ownership_violations` vector; `AttributedDiagnostic.plan_index` set directly from the entry's plan index rather than the `family_to_plan` guess). `Fail` returns `Err(SupportAggregationError)`, whose fields are replaced: the existing `expected_family_id` / `conflicting_family_id` struct described the arrival-order conflict that no longer exists; it becomes `{ global_layer_index, object_id, region_id, family_id, module_id, reason: OwnershipReason }`. Ruling (2026-09-03 preflight): **`SupportAggregationError` stays a `pub struct`** — it is not converted to an enum, and no `ProducerCountMismatch` variant is introduced. A `plans`/`producers` length mismatch is a host-internal construction invariant: `check_ownership` treats a missing producer entry as `OwnershipReason::MissingClaim` (default-deny), guarded by a `debug_assert_eq!` on the two lengths. This keeps AC-N1's struct-literal pattern valid and avoids breaking every `Fail`-path caller and fixture. A wholly absent `territory` means no owners exist: every entry is a trespass. Delete `arrival_owners` and the cross-family duplicate branch it fed; rewrite `support_plan_aggregation_diagnoses_duplicate_identity` to assert assignment-based ownership in both plan orders.
- **W3b — producer identity at the call site.** `crates/slicer-runtime/src/prepass.rs` pairs each buffered plan with `module.module_id()` and `module.claims()`. Attribution in the prepass uses the exact `plan_index`.
- **W3c — tree self-default removed.** Delete the `assignments_empty` fallback in `modules/core-modules/tree-support-planner/src/lib.rs` (both the inline region loop and `candidate_family`); empty `family_assignments` emits nothing. `support_analysis_producer` already mints one row per RegionMap region, so this only closes the no-region-map path.
- **W4 — anchor_z ↔ layer-index consistency.** `merge_region_identity_entries` returns `Result<(), ModuleError>` (the type `plan_for_object` already returns) and rejects any pair with equal `(global_layer_index, object_id, region_id)` and unequal `anchor_z`, or equal `(object_id, region_id, anchor_z)` and unequal `global_layer_index`.
- **W5 — producer merge kept as the DEV-167 fix.** Reword the `merge_region_identity_entries` doc comment so it no longer describes the host routing cell. (Measured 2026-09-03: the doc comment does **not** contain the word "interim" — zero matches file-wide; that word appears in the DEV-167 row and in packet 241's spec, not in the code. Its final paragraph does describe the `ROUTING_CELL_SIZE` extent bound and must be updated to `MAX_BODY_EXTENT_UNITS` per W2.) Close DEV-167; its current Status text says this packet owns "the removal of `merge_region_identity_entries`", so the closure row must state explicitly that grilling on 2026-09-03 reversed that and the merge is retained as the producer invariant.
- **W6 — packet-239 tests restored.** `coarse_same_region_sources_keep_distinct_body_membership` and `coarse_source_preference_keeps_mixed_source_memberships` assert one entry per plane whose `body_ids` carries both memberships (body + interface-only) and whose contour is the union of the two source contours.
- **W7 — doc defects.** All anchors measured 2026-09-03.
  - `docs/02_ir_schemas.md`: drop the `layer-range override` level — **three** occurrences, in the IR 3 merging sentence and in the §"Config Precedence Rules" ordering list, not two; rename the stale `stamp_modifier_config_deltas` reference to `stamp_modifier_sub_region_configs` (the real symbol is `crates/slicer-core/src/algos/region_mapping.rs::stamp_modifier_sub_region_configs`; no `fn stamp_modifier_config_deltas` exists anywhere in `crates/`); add the ownership sentence to IR 9b **on a single source line**, because `rg` is line-oriented and the neighbouring uniqueness prose is line-wrapped; and replace the routing-cell ownership paragraph in §"IR 9 — SupportIR", which describes both the deleted routing cells and the deleted cross-family arrival-order branch.
  - `docs/01_system_architecture.md` step 4: name the support-plan commit seam and state that production builds only `ConflictScope::Global` holders.
  - `docs/04_host_scheduler.md`: replace routing-cell prose in **both** bullets that carry it — §"Host aggregation as the sole multi-writer merge point" and §"Complete-body validation". The second describes the surviving `in_routing_cell` bound and must be reworded as a max-body-extent bound, not deleted wholesale.
  - ADR-0059: **add** (not restore) a dated `Ruling 2` under `## Amendments`. The phrase "exactly one attributed plan entry" has never appeared in the ADR — it lives in `docs/specs/support-families-anchored-entities-plan.md` §6 invariant 15. Ruling 2 must quote the superseded decision clause "assigns deterministic routing cells", record declared-identity keying, and carry invariant 15 **in full**, including "regions requiring no support carry a structured no-work/declined record"; quoting only the first half would contradict the ADR's own Ruling 1, which defers candidate-less emission semantics.
  - `docs/DEVIATION_LOG.md`: new row (ID re-derived at Step 6) recording the ADR-0059 amendment, per the spec-review S8 rule that an ADR's normative content is never silently rewritten.
  - `docs/07`: open row recording the inert `paint_config:*` / `tool_config:*` axis with evidence.
  - Add a doc comment on `DagValidationPass::PerRegionClaimConflicts` noting production emits only `Global` holders.
- **Test-fixture fallout.** Every existing aggregation test that passes `territory: None` and expects retention gains a `SupportAnalysisIR` with `family_assignments` and a matching `producers` vector: `SupportAggregationInput {` literals measured 2026-09-03: `crates/slicer-wasm-host/tests/contract/support_plan_validation.rs` (5), `tests/unit/support_cross_family_scope_tdd.rs` (6), `crates/slicer-runtime/tests/integration/support_family_routing.rs` (4, its `routing_cells` test renamed to describe declared-identity independence), plus 2 internal sites in `src/support_aggregation.rs`. `support_decline_contract.rs` and `support_family_closure.rs` reach aggregation through the IR-level wrappers and are re-verified, not necessarily edited. Add a shared `family_assignments_for(entries)` helper per test crate. Re-derive the inventory at Step 3.

## Out of Scope

- Reviving the schedule-time `PerRegionClaimConflicts` pass or adding a post-region-assignment claim phase (grilling decision: W3 is the enforcement; the pass is documented, not deleted).
- Fixing the inert paint/tool config axis; record only.
- The AGG rasterizer, `support_area_rasterizer` knob, DEV-166 (packet 241).
- Raft, independent support-layer Z, renderer flow (packets 240a/240b, 239, 238c).
- Restructuring `plan_for_object` to emit one entry per region without a post-hoc merge (rejected at grilling in favour of keeping the merge).
- The tree planner's `covered_regions` dedup key (lacks `anchor_z`); recorded as `[FWD]` in `design.md`.

## Authoritative Docs

- `docs/02_ir_schemas.md` - long; ranged reads of §"IR 9 — SupportIR" (routing-cell ownership paragraph), §"IR 9b — SupportPlanIR", §"Modifier Resolution Contract", §"Config Precedence Rules", §"Config Key Namespaces".
- `docs/04_host_scheduler.md` - long; ranged read of §"Host aggregation as the sole multi-writer merge point" through §"Complete-body validation" — both bullets name routing cells.
- `docs/01_system_architecture.md` - long; ranged read of §"Claim Conflict Resolution (Normative)".
- `docs/specs/support-families-anchored-entities-plan.md` - delegate SUMMARY (§6 invariants 15 **and 16**, AC-8 Ruling 1, Packet Queue rows 7a/7b/10).
- `docs/adr/0059-support-families-and-anchored-entities.md` - short; read in full.
- `docs/DEVIATION_LOG.md` - short; read rows DEV-162, DEV-165, DEV-167 only.

## Acceptance Summary

- Positive: `AC-1` through `AC-11` (`AC-11` added at preflight for the ADR-0059 amendment). Refinement absent from their text: AC-5's rewritten test must keep the original file and test name so packet 241/239 references stay valid — it lives in `crates/slicer-wasm-host/tests/contract/support_plan_validation.rs` and therefore in the `contract` binary, which is what AC-5 targets.
- Negative: `AC-N1` through `AC-N4`.
- Cross-packet impact: packet 241 AC-N2 becomes green without editing packet 241's files; DEV-167 closes; packet 224/242 Orca-closure evidence is unaffected because retained geometry for a correctly-owned region is byte-identical before and after (merge output is the same union, only the key changes).

## Verification Commands

Every `cargo test` command below tees to `target/test-output.log` and asserts a non-zero matched count, per §6 invariant 16 and CLAUDE.md. A bare `grep -q ' 0 failed'` is forbidden — a zero-test run satisfies it.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `mkdir -p target && cargo test -p slicer-wasm-host --test unit support_plan_ownership_tdd 2>&1 \| tee target/test-output.log; grep -q '^test result: ok' target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -eq 4 && echo PASS` | AC-1, AC-N1, AC-N2 plus the both-orders test | FACT PASS/absent |
| `mkdir -p target && cargo test -p slicer-wasm-host --test contract support_plan_ 2>&1 \| tee target/test-output.log; grep -q '^test result: ok' target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS` | AC-5 plus fixture fallout | FACT PASS/absent; SNIPPETS <=20 lines on failure |
| `mkdir -p target && cargo test -p slicer-wasm-host --test unit support_cross_family_scope_tdd 2>&1 \| tee target/test-output.log; grep -q '^test result: ok' target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS` | fixture fallout, territory clipper unchanged | FACT PASS/absent |
| `mkdir -p target && cargo test -p slicer-runtime --test integration 2>&1 \| tee target/test-output.log; grep -q '^test result: ok' target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 30 && echo PASS` | AC-8 — whole binary; **never** filter on `support_family_` | FACT PASS/absent |
| `mkdir -p target && cargo test -p traditional-support-planner --test traditional_family_tdd 2>&1 \| tee target/test-output.log; grep -q '^test result: ok' target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 10 && echo PASS` | AC-6, AC-7, AC-N3 | FACT PASS/absent |
| `mkdir -p target && cargo test -p tree-support-planner --test tree_family_tdd 2>&1 \| tee target/test-output.log; grep -q '^test result: ok' target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS` | AC-N4 plus tree fallout | FACT PASS/absent |
| `cargo xtask build-guests --check; echo EXIT=$?` | guest freshness after planner edits | FACT exit code |
| `cargo check --workspace --all-targets` | gate; also covers the second `support_family_routing` test target | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | gate | FACT pass/fail |
| `cargo xtask check-literals; echo EXIT=$?` | struct-literal gate (new test literals of `SupportPlanEntry` / `SupportAnalysisIR` must use `..`) | FACT exit code |
| AC-2, AC-3, AC-4, AC-9, AC-10, AC-11 static greps as written in `packet.spec.md` | static surface | FACT PASS/absent |

## Step Completion Expectations

- Step 1 (merge key) and Step 2 (ownership check) both edit `support_aggregation.rs`; Step 2 must start from Step 1's committed state so the deleted `RoutingCell` does not reappear in a merge.
- Fixture fallout (Step 3) must land before Step 2's negative tests are declared green, because Step 2 turns every `territory: None` fixture red by design.
- Guest edits (Steps 4, 5) require `cargo xtask build-guests --check` before any host integration test is interpreted.
- DEV-167 closes only in Step 6, after AC-7 is green.

## Context Discipline Notes

Line counts are ledger facts that rot; re-derive any range from a `LOCATIONS` dispatch at the moment of use and navigate by symbol name.

- `crates/slicer-wasm-host/src/support_aggregation.rs` is long: ranged reads only, anchored on `union_same_family_entries`, `try_aggregate_support_plans_with_policy`, `aggregate_support_plan_irs_with_policy_attributed`, and the diagnostics/struct block near the top of the file.
- `modules/core-modules/tree-support-planner/src/lib.rs` is very long (several thousand lines): never open without a line range from a `LOCATIONS` dispatch.
- `modules/core-modules/traditional-support-planner/src/lib.rs` is long: ranged reads anchored on `plan_for_object` and `merge_region_identity_entries`.
- `docs/07_implementation_status.md` is always delegated.
