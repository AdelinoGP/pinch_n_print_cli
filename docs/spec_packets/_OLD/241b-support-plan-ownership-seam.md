---
status: implemented
packet: 241b-support-plan-ownership-seam
task_ids:
  - TASK-531
---

# 241b-support-plan-ownership-seam

## Goal

Make support-region ownership enforced at the single host merge point: `union_same_family_entries` keys on declared `(family_id, global_layer_index, object_id, region_id, anchor_z)` instead of a bbox-centroid grid cell, every `SupportPlanEntry` is checked against `SupportAnalysisIR::family_assignments` and its producing module's `support-family:<id>` claim (default-deny), arrival-order arbitration is deleted, the traditional planner's per-triple merge becomes the DEV-167 fix with an `anchor_z`↔`global_layer_index` consistency check, and the two packet-239 tests are restored against the one-entry-per-triple shape.

## Problem Statement

Packet 241 removed the DEV-166 clamp from the `agg` rasterizer and the resulting geometry tripped `SupportPlanIR::duplicate_region_identity` at commit. The instrumented root cause (DEV-167) is older than 241: `SupportPlanner::plan_for_object` publishes one `SupportPlanEntry` per candidate per layer, and the host `union_same_family_entries` had been hiding that by merging on a bbox-centroid grid cell (`ROUTING_CELL_SIZE = 1 << 20`) rather than on `region_id`. Three centroids straddling a cell boundary stopped merging and the invariant fired. The same grid line already caused packet 224's RC-14 defect on the `in_routing_cell` path.

Investigation exposed a wider gap: region ownership at the support-plan seam is not enforced anywhere. `entry.family_id` is self-declared by the guest and never checked against `SupportAnalysisIR::family_assignments` or the producer's manifest claims; cross-family duplicates are resolved by arrival order `(plan_index, entry_index)`; the schedule-time `PerRegionClaimConflicts` pass runs on an empty set because `crates/slicer-runtime/src/run.rs` builds only `ConflictScope::Global` holders and has no region ids at startup. Grilling on 2026-09-03 settled the shape: enforce ownership once, at the host merge point, keyed by declared identity and `family_assignments`; retire arrival order; keep the producer merge as the DEV-167 fix; restore the packet-239 tests against the one-entry-per-triple shape.

Packet 241 stays `implemented`; this packet turns its AC-N2 green rather than reopening it.

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
