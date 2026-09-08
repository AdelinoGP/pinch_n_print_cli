# Design: 241b-support-plan-ownership-seam

## Controlling Code Paths

- Primary code path: `try_aggregate_support_plans_with_policy` → `union_same_family_entries` (`crates/slicer-wasm-host/src/support_aggregation.rs`), reached from `aggregate_support_plan_irs_degrading_with_attributed_diagnostics` at the `PrePass::SupportGeometry` buffer in `crates/slicer-runtime/src/prepass.rs` (the `for module in &stage.modules` loop that fills `support_plans` / `support_plan_audits`).
- Producer path: `SupportPlanner::plan_for_object` → `merge_region_identity_entries` (`modules/core-modules/traditional-support-planner/src/lib.rs`); tree planner region loop and `candidate_family` (`modules/core-modules/tree-support-planner/src/lib.rs`).
- Neighboring tests/fixtures: `crates/slicer-wasm-host/tests/contract/support_plan_validation.rs`, `tests/unit/support_cross_family_scope_tdd.rs` (inline `SupportAnalysisIR` with `family_assignments`), `crates/slicer-runtime/tests/integration/support_family_routing.rs`, `modules/core-modules/traditional-support-planner/tests/traditional_family_tdd.rs`.
- OrcaSlicer comparison: none. OrcaSlicer has a single support generator and no multi-writer merge; there is no canonical behaviour to borrow. No `orca-delegation` snippet applies.

## Architecture Constraints

- Ownership is decided by `SupportAnalysisIR::family_assignments: BTreeMap<(ObjectId, RegionId), String>`, minted per RegionMap region by `slicer_runtime::builtins::support_analysis_producer`. The aggregation seam is the only enforcement point; the schedule-time `PerRegionClaimConflicts` pass stays as-is (production constructs only `ConflictScope::Global` holders and has no region ids at startup).
- Default-deny mirrors `enforce_authored_coloring` / `AuthoredColoringContext::allows` (`crates/slicer-wasm-host/src/marshal/out.rs`) in policy but not in silence: every dropped entry produces a diagnostic.
- Merge key keeps `anchor_z` (DEV-162): distinct declared planes never merge. `duplicate_region_identity` ignores `anchor_z`; the W4 producer check guarantees the two keys cannot disagree.
- ADR-0059's existing decision paragraphs and Ruling 1 stay byte-identical, but the ADR **is** amended, because its decision text says the host "assigns deterministic routing cells" and this packet deletes that mechanism from aggregation (routing cells survive only as `in_routing_cell`'s max-body-extent bound in complete-body validation). Per the spec-review S8 rule, a packet may not silently contradict an ADR's normative content: Step 6 appends a dated `Ruling 2` under `## Amendments` that (a) quotes the superseded clause "assigns deterministic routing cells", (b) records declared-identity keying as the replacement, and (c) carries §6 invariant 15 in full — "Every RegionMap region has exactly one attributed plan entry; regions requiring no support carry a structured no-work/declined record" — quoting only the first half would contradict Ruling 1's deferral of candidate-less emission semantics. The phrase "exactly one attributed plan entry" is **added**, not restored; it has never appeared in this ADR. A deviation row naming ADR-0059 is filed in the same step.
- Guest-visible geometry must be unchanged for correctly-owned regions: the union output for a same-region group is the same polygon union as before, only the grouping predicate changes.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- No public schema/version constant is bumped. `SupportAggregationInput` is a host-internal `pub struct` with two construction sites in `src/` plus the test fixtures listed in `requirements.md`; the struct-literal blast radius is owned by Step 3.

## Code Change Surface

- Selected approach: enforce ownership at the host merge point using declared identity plus `family_assignments`; thread producer identity as a new index-parallel input; keep the producer-side merge as the DEV-167 fix and harden it with the anchor/layer check.
- Exact functions, traits, manifests, tests, and fixtures:
  - `crates/slicer-wasm-host/src/support_aggregation.rs`
    - `union_same_family_entries`: predicate becomes `family_id && global_layer_index && object_id && region_id && anchor_z`; delete `group_cells` (a local `Vec<Option<RoutingCell>>`, not a fn), `RoutingCell`, and `routing_cell`. `same_body` becomes unused; delete it unless another caller exists (dispatch confirms).
    - `ROUTING_CELL_SIZE` **is not deletable on its own**: `in_routing_cell` reads it twice as the bbox extent bound (`maxx - minx <= ROUTING_CELL_SIZE && maxy - miny <= ROUTING_CELL_SIZE`). Ruling (2026-09-03 preflight): rename it to `MAX_BODY_EXTENT_UNITS`, same `1 << 20` value, and update those two uses. `fn in_routing_cell` keeps its name — packet 224's RC-14 record and the traditional planner's `merge_region_identity_entries` doc comment both reference it — so it is behaviourally identical, **not** byte-identical.
    - New `pub struct SupportPlanProducer { pub module_id: String, pub claims: Vec<String> }`; `SupportAggregationInput` (before this packet: `pub struct SupportAggregationInput<'a>` with exactly `plans`, `exact_z`, `territory`) gains `pub producers: Vec<SupportPlanProducer>`, index-parallel to `plans`. A length mismatch is a host-internal construction invariant, **not** a new error variant: `check_ownership` treats a missing producer entry as `OwnershipReason::MissingClaim` (default-deny), guarded by `debug_assert_eq!(input.plans.len(), input.producers.len())`.
    - `try_aggregate_support_plans_with_policy`: delete `arrival_owners` and the `Degrade`/`Fail` cross-family duplicate branch; insert `check_ownership(entry, plan_index, &input) -> Result<(), OwnershipViolation>` before validation. New `OwnershipViolation { global_layer_index, object_id, region_id, family_id, module_id, reason: OwnershipReason }` — **as implemented this carries a seventh field, `plan_index: usize`** (six were planned here). It was added so the code-`1206` diagnostic's `plan_index` comes directly from the violation rather than from the `family_to_plan` guess, which is what lets AC-N2 assert `plan_index == Some(1)` with `OwnershipReason::{NoAssignment, WrongFamily { owner }, MissingClaim { required }}`. `SupportAggregationResult` gains `ownership_violations: Vec<OwnershipViolation>`. `SupportAggregationError` was, before this packet, a `pub struct { global_layer_index, object_id, region_id, expected_family_id, conflicting_family_id }` describing the arrival-order conflict; its fields are replaced by the `OwnershipViolation` shape (`family_id`, `module_id`, `reason` in place of the two family fields). Ruling (2026-09-03 preflight): **it stays a struct.** It is not converted to an enum and gains no variants, so AC-N1's `Err(SupportAggregationError { reason: OwnershipReason::NoAssignment, .. })` pattern stays valid and no `Fail`-path caller or fixture breaks on the shape change beyond the renamed fields. A `LOCATIONS` dispatch for `expected_family_id|conflicting_family_id` readers precedes the edit.
    - Attributed entry `aggregate_support_plan_irs_with_policy_attributed`: emit code `1206` (Warn; `1200`-`1205` are in use, `1205` is the clipped-body Info) per violation with `plan_index: Some(plan_index)` taken from the violation, not from `family_to_plan`. `DuplicateSupportPlanEntry` and code `1202` remain for same-family duplicates that survive union (should be unreachable; keep as a tripwire).
  - `crates/slicer-runtime/src/prepass.rs`: the buffer push becomes `support_plans.push(plan); support_plan_producers.push(SupportPlanProducer { module_id: module.module_id().to_string(), claims: module.claims().to_vec() })`; pass `producers` into the aggregation input.
  - `crates/slicer-scheduler/src/validation.rs`: doc comment only on `DagValidationPass::PerRegionClaimConflicts`.
  - `modules/core-modules/traditional-support-planner/src/lib.rs`: `merge_region_identity_entries` returned unit and mutated in place before this packet; it becomes `Result<(), ModuleError>`. That is the type `plan_for_object` already returns, so the `?` is ergonomic with no signature churn above it. (Correction, verified against the tree: the merge call is **not** directly after the `emit_coarse_entries(...)?` call — `emit_coarse_entries` sits inside the `if` arm of a `let mut emitted = if … else …;`, with the `else` branch, the closing `};` and a two-line comment between it and the merge call. The adjacency claim was wrong; the `Result`-return conclusion it was offered in support of stands on `plan_for_object`'s own signature.) (Correction: `next_intermediate_plane_index` is **not** used inside `plan_for_object` — it is defined and `?`-used elsewhere in the file; do not rely on that as the precedent.) Adds the two-direction anchor/layer disagreement check; doc comment rewritten to drop the host routing-cell description and to name `MAX_BODY_EXTENT_UNITS` for the surviving extent bound.
  - `modules/core-modules/tree-support-planner/src/lib.rs`: delete `assignments_empty` / `fallback_family_emitted` self-default in the inline region loop and in `candidate_family`.
  - Tests: new `crates/slicer-wasm-host/tests/unit/support_plan_ownership_tdd.rs` (registered via `mod support_plan_ownership_tdd;` in `crates/slicer-wasm-host/tests/unit/main.rs`) with `union_merges_same_region_entries_regardless_of_distance`, `unassigned_region_entry_is_a_trespass`, `producer_without_family_claim_is_a_trespass`, `wrong_family_entry_is_a_trespass_in_both_plan_orders`; rewritten `support_plan_aggregation_diagnoses_duplicate_identity`; fixture fallout files per `requirements.md`; `merge_rejects_anchor_z_layer_index_disagreement` and the two rewritten `coarse_*` tests in `traditional_family_tdd.rs`; `empty_family_assignments_emit_nothing` in `tree_family_tdd.rs`.
- Rejected alternatives and reasons:
  - Post-region-assignment claim phase to make `PerRegionClaimConflicts` live: needs a second `DagValidationRequest` after region mapping; an L-sized seam duplicating what the commit-time check does.
  - Arrival order as tiebreak when no row exists: keeps dead code alive for a case the producer already prevents; strict no-row = trespass chosen.
  - Silent strip like `AuthoredColoringContext`: hides producer defects; `FamilyConflictPolicy` already has the `Fail`/`Degrade` vocabulary.
  - Relaxing identity to include `body_ids`: contradicts `docs/02` IR 9b and invariant 15.
  - Dropping `anchor_z` from the merge key: reverses DEV-162.

## Files in Scope (read + edit)

- `crates/slicer-wasm-host/src/support_aggregation.rs` - role: sole merge point; expected change: key, ownership check, producer input, diagnostics.
- `crates/slicer-runtime/src/prepass.rs` - role: only production construction site; expected change: producer vector alongside `support_plans`.
- `modules/core-modules/traditional-support-planner/src/lib.rs` - role: DEV-167 producer; expected change: `merge_region_identity_entries` hardening.
- Extras, justified: `modules/core-modules/tree-support-planner/src/lib.rs` (self-default removal across **three** sites, not two — the `let mut fallback_family_emitted = false;` declaration, the `assignments_empty` region-loop block that ORs into it, and the trailing `if fallback_family_emitted { push_diagnostic(code 1004) }` block, all inside one long function and separated by roughly two thousand lines; re-derive every range from a `LOCATIONS` dispatch); `crates/slicer-scheduler/src/validation.rs` (doc comment only); test files and docs enumerated in `requirements.md`. Splitting the tree edit into another packet would leave AC-N4's no-row rule unenforceable for the no-region-map path.

## Read-Only Context

Navigate these by symbol, not by line; any range below is a hint to re-derive at read time.

- `crates/slicer-ir/src/slice_ir.rs` - `SupportPlanIR::duplicate_region_identity` (keys the `(global_layer_index, object_id, region_id)` triple and deliberately ignores both `anchor_z` and `family_id`) and `SupportAnalysisIR::family_assignments` only. Note `ObjectId = String` and `RegionId = u64` here, while the guest-side `slicer-sdk` aliases `RegionId = String`.
- `crates/slicer-runtime/src/blackboard.rs` - `commit_support_plan`'s two `duplicate_region_identity` checks only.
- `crates/slicer-runtime/src/builtins/support_analysis_producer.rs` - the per-RegionMap-region assignment minting loop only (it also has a candidate-based fallback for fixtures lacking a `RegionMapIR`).
- `crates/slicer-scheduler/src/execution_plan.rs` - `CompiledModuleStatic::module_id()` and `CompiledModuleStatic::claims()` accessors only; `claims()` returns `&[String]`.
- `crates/slicer-wasm-host/src/marshal/out.rs` - `enforce_authored_coloring` only, as the precedent. Its `AuthoredColoringContext::allows` is private, so it is a policy model, not a reusable API.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - not consulted; no parity surface.
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load.
- `crates/slicer-runtime/src/run.rs` - claim-holder construction is unchanged; do not open.
- `docs/spec_packets/241-support-agg-rasterizer/**` - never edit; packet 241 stays `implemented`.
- Other crates - delegate symbol lookups.

## Expected Sub-Agent Dispatches

- Question: does anything outside `union_same_family_entries` call `same_body`, `routing_cell`, or read `RoutingCell`?; scope: `crates/slicer-wasm-host/src/`; return: `LOCATIONS`; purpose: Step 1.
- Question: every construction site of `SupportAggregationInput { .. }` in `crates/` (src and tests) and `modules/`; scope: `crates/ modules/`; return: `LOCATIONS`; purpose: Step 3 blast radius.
- Question: what error type does `plan_for_object` return and how does a guest surface it?; scope: `modules/core-modules/traditional-support-planner/src/lib.rs`; return: `FACT`; purpose: Step 4.
- Question: exact line ranges of the tree planner `assignments_empty` fallback and `fallback_family_emitted` uses; scope: `modules/core-modules/tree-support-planner/src/lib.rs`; return: `LOCATIONS`; purpose: Step 5.
- Question: which tree planner tests construct an empty `family_assignments` and expect entries?; scope: `modules/core-modules/tree-support-planner/tests/`; return: `LOCATIONS`; purpose: Step 5.
- Question: current highest `TASK-###` and `DEV-###`; scope: `docs/07_implementation_status.md`, `docs/DEVIATION_LOG.md`; return: `FACT`; purpose: Step 6 (re-derive at edit time).

## Data and Contract Notes

- IR/manifest contracts: no IR field changes. `family_assignments` becomes normative for ownership (docs/02 IR 9b sentence). Producer claim strings are the existing `support-family:traditional` / `support-family:tree` manifest claims. Note that `FAMILY_SCOPED_SUPPORT_CLAIMS` is **not** an importable shared item: it exists as two identical function-local `const`s inside `crates/slicer-scheduler/src/validation.rs`, so `slicer-wasm-host` cannot reference it. The ownership check compares against the `support-family:<family_id>` string it composes from the entry, not against that list; do not plan an import.
- WIT boundary: untouched. `SupportPlanProducer` is host-internal.
- `SupportPlanEntry` has **no** contour or polygon field: geometry lives in `roles: Vec<SupportPlanRoleRegion>` (each with its own `regions`) and optionally `skeleton`. AC-6's "contour area" therefore means the shoelace area summed over `roles[].regions` for the merged entry, compared against the same sum over the two source entries' regions. `union_same_family_entries` merges roles by matching `role`, so the test helper must sum across all roles rather than assuming a single one.
- `crates/slicer-runtime/Cargo.toml` declares a second `[[test]] name = "support_family_routing"` over the same file that `tests/integration/main.rs` already registers as a module. Step 3c's edits to that file compile into two binaries; `cargo check --workspace --all-targets` is what covers the second one.
- Determinism/scheduler constraints: ordering still comes from `entries.sort_by(compare_entries)`; removing `group_cells` removes the only order-sensitive term. Ownership is a pure function of `(entry, family_assignments, producer claims)`, so results are plan-order independent by construction; AC-5 asserts both orders.

## Locked Assumptions and Invariants

- Grilling 2026-09-03: W1 dropped, W3 is the enforcement; assignment decides and arrival order is deleted; no assignment row means no owner; union key keeps `anchor_z`; producer merge kept as the DEV-167 fix; one entry per triple wins over the packet-239 test shape; W7 fixes the text defects and records the inert paint/tool axis.
- Preflight ruling 2026-09-03 (blocker 1): `ROUTING_CELL_SIZE` is renamed to `MAX_BODY_EXTENT_UNITS` rather than deleted or inlined; `fn in_routing_cell` keeps its name. AC-2 and Step 1's exit condition are written against that ruling — "byte-identical" was unsatisfiable because the surviving function reads the constant.
- Preflight ruling 2026-09-03 (blocker 2): `SupportAggregationError` stays a `pub struct` with replaced fields; no enum conversion, no `ProducerCountMismatch` variant. Producer/plan length mismatch is a `debug_assert_eq!` plus default-deny `MissingClaim`.
- Preflight ruling 2026-09-03 (S8): ADR-0059 gains a dated `Ruling 2` amendment plus a deviation row, rather than being silently contradicted.
- Test-command invariant (§6 invariant 16): no acceptance command may match zero tests, and every `cargo test` tees to `target/test-output.log`. The `integration` binary is never filtered on `support_family_`.
- Invariant (new, enforced at producer): within one plan, `(global_layer_index, object_id, region_id)` determines `anchor_z` and `(object_id, region_id, anchor_z)` determines `global_layer_index`.
- Invariant (new, enforced at host): a retained `SupportPlanEntry` always has `family_id == family_assignments[(object_id, region_id)]` and its producer holds `support-family:<family_id>`.

## Risks and Tradeoffs

- Fixture churn: 15 existing `SupportAggregationInput` literals across three test files must supply `family_assignments` and `producers`. Mitigated by one helper per test crate; budgeted in Step 3.
- A wholly absent `SupportAnalysisIR` now drops every plan. In production planners cannot run without it (they read `family_assignments`), so this is unreachable; a diagnostic per dropped entry makes any surprise loud.
- Tree `covered_regions` dedup lacks `anchor_z`. This design assumed off-grid interpolation mints distinct synthetic layer indices per plane so the triple cannot collide; **that assumption is refuted** — the `coarse_used` branch can emit two entries on one triple, masked today only by the host's `union_same_family_entries` fold. See DEV-170 in `docs/DEVIATION_LOG.md`.
- The AGG rasterizer path (packet 241) produced the geometry that exposed DEV-167; AC-7 proves the planner binary is green but does not re-run 241's `agg` real-mesh slice. Step 6 re-dispatches packet 241's AC-N2 command as recorded in its `packet.spec.md`.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 3 fixture fallout)
- Highest-risk dispatch and required return format: `SupportAggregationInput { .. }` construction-site inventory, `LOCATIONS` (≤ 20).

## Open Questions

- **RESOLVED** — `same_body` had no caller outside `union_same_family_entries`; deleted in Step 1 and verified absent from `crates/slicer-wasm-host/src/`.
- **RESOLVED, positive** — Tree planner `covered_regions` key without `anchor_z`: the 239c off-grid path **can** emit two entries with equal `(global_layer_index, object_id, region_id)` (two candidates from different source layers grouped into one `CANONICAL_EPSILON_MM` plane get distinct `synthesized_seen` keys but the same emitted identity). This packet first recorded the opposite, negative answer; that was wrong. The duplicate is currently masked by the host's `union_same_family_entries` fold, and the W4 anchor/layer check was **not** ported to the tree planner. Tracked as DEV-170 (`Open`) in `docs/DEVIATION_LOG.md`.
- **RESOLVED — inert by construction** — `DuplicateSupportPlanEntry` / code `1202`: `SupportAggregationResult::duplicates` is declared and read at the code-`1202` mapping, but is pushed to nowhere; the only writer was the cross-family arrival-order arbitration branch this packet deleted. The field and code are retained as an inert tripwire for a future writer, and no fix is pending. Repopulating it is not possible as stated: `union_same_family_entries` merges on `(family_id, global_layer_index, object_id, region_id, anchor_z)`, so post-union survivors are unique on any key that CONTAINS `anchor_z` — such a key can never collide, by construction — while a key that OMITS `anchor_z` false-positives on independent support rows that legitimately share `(layer, object, region)` at distinct physical planes. Cross-family collision on the same tuple is now reported as `OwnershipViolation` / diagnostic code `1206`, not `1202`. Note: `duplicates.is_empty()` proves nothing today — no test may rely on it as evidence that duplicates were checked.
- None `[BLOCK]`.
