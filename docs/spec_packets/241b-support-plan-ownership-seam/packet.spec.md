---
status: draft
packet: 241b-support-plan-ownership-seam
task_ids: []
depends_on: 241-support-agg-rasterizer
backlog_source: docs/specs/support-families-anchored-entities-plan.md
context_cost_estimate: TBD
---

# Packet Contract: 241b-support-plan-ownership-seam

> **STUB ONLY.** This file records scope and the evidence behind it. `requirements.md`,
> `design.md`, `implementation-plan.md`, and `task-map.md` are NOT yet written — generate them
> with `/spec-packet-generator` and grill the result before activation. Every measured value
> below was taken during the packet-241 session (2026-09-03); re-derive anything load-bearing
> before you rely on it.

## Goal

Make region ownership real at the support-plan seam: one module owns a region, a module cannot
publish output for a region it does not own, and entry identity is enforced by declared
identifiers rather than by where geometry happens to sit.

## Why this packet exists

Packet 241 removed the DEV-166 clamp from the `agg` rasterizer arm. The resulting geometry
tripped a commit-time invariant and exposed a class of defect that predates 241 entirely:

> `SupportPlanIR contains duplicate entries for support region (layer=0, object=..., region=0)`

Root cause, verified by instrumented probe (not inferred). `SupportPlanner::plan_for_object`
(`modules/core-modules/traditional-support-planner/src/lib.rs`) publishes one
`SupportPlanEntry` per CANDIDATE per layer, so several entries can share one
`(global_layer_index, object_id, region_id)`. That contradicts
`docs/02_ir_schemas.md` section "IR 9b — SupportPlanIR" ("Each `SupportPlanEntry` is produced
once per `(global_layer_index, object_id, region_id)` triple") and
`docs/specs/support-families-anchored-entities-plan.md` section 6 invariant 15 ("Every RegionMap
region has exactly one attributed plan entry"). Ruling 1 of that same plan diagnosed this exact
class once already — "assignments were minted per candidate".

It survived because host `union_same_family_entries`
(`crates/slicer-wasm-host/src/support_aggregation.rs`) merges on `family_id` + layer +
`object_id` + `anchor_z` plus (same body **OR** equal `routing_cell`) — and `region_id` is
absent from that key. `routing_cell` is the entry bbox centroid floored onto a
`ROUTING_CELL_SIZE` = `1<<20` unit grid (~104.86 mm). Measured at the failure: centroids
(503750, 250000) and (250000, 541750) both land in cell (0, 0) and merge; the third, at
(250004, **-15250**), is floored by `div_euclid` into cell (0, -1) and never merges. The
duplicates had been merging on centroid coincidence.

**Prior art — the same bug, the same file, the same grid line.** Packet 224 section RC-14
records that `in_routing_cell` once demanded absolute-grid containment: a 0.4 mm interface tip
reaching y = -0.4 mm crossed the y = 0 cell boundary, producing 528 rejections at
`support_top_z_distance = 0.2` and zero at `0.0`, destroying both `TopInterface` layers. It was
fixed by switching that check to a pure extent bound. The merge path never got the same
treatment.

## Scope

### W1 — Revive region-scoped claim enforcement

`ConflictScope::Region { object_id, region_id, global_layer_index }`
(`crates/slicer-scheduler/src/validation.rs`) exists, and `validate_claim_conflicts` has a
region pass. But the only production construction site — the `claim_holders` build in
`crates/slicer-runtime/src/run.rs` — emits `ConflictScope::Global` for every claim, so the
`PerRegionClaimConflicts` pass runs on an empty set every time. `docs/01_system_architecture.md`
section "Claim Conflict Resolution (Normative)" step 4, "Validate uniqueness for every
`(layer, object, region, claim)`", is a **dead pass**. Construct region-scoped holders so it
becomes live.

Note before designing: support claims are deliberately exempted from the global exclusivity
check — `if global_only && FAMILY_SCOPED_SUPPORT_CLAIMS.contains(&claim.as_str()) { continue; }`,
covering `support-generator`, `support-planner`, `support-family:traditional`,
`support-family:tree`. Both planners hold `support-planner` simultaneously by design. Reviving
region scope must not break that; the exemption is about global co-existence, not about
per-region exclusivity.

### W2 — Key the union on declared identity, not on geometry

Replace the centroid `routing_cell` term in `union_same_family_entries`'s merge predicate with
`region_id`. Blast radius measured SMALL: `RoutingCell` is a private struct in one file with
~8 call sites, derives no `Serialize`, and has no IR field, WIT type, or persisted form. **No
test exercises the cell-merge branch** — every union test shares a `body_id` and merges via
`same_body`.

Do NOT confuse the two mechanisms that share the name: `in_routing_cell` (used by
`validate_entry`) is a pure bbox-**extent** bound since packet 224 and computes no cell at all.
It is independent of the merge key and must be left alone.

The documented determinism justification in `docs/04_host_scheduler.md` does not depend on
centroids: ordering comes from `entries.sort_by(compare_entries)`. The `group_cells` snapshot
exists only because recomputing the centroid mid-loop let the key drift as a group absorbed
members — an order-sensitivity that centroid keying itself introduced. A declared key removes it.

### W3 — Verify self-declared family ownership

`entry.family_id` is written by the guest about itself (hardcoded `"traditional"` / `"tree"`)
and is never cross-checked against the producing module's manifest claims or against
`SupportAnalysisIR::family_assignments`. Cross-family arbitration currently trusts it.

Thread producing-module identity into aggregation and verify. The seam already exists:
`aggregate_support_plan_irs_degrading_with_attributed_diagnostics`
(`crates/slicer-runtime/src/prepass.rs`) sees every plan, holds per-plan indices for
attribution, already drops and clips entries, and already receives `family_assignments`. What
it lacks is module identity — `SupportAggregationInput` carries `plans` with no module id,
though the id is available at the `live_module` construction site in the same loop. This is a
new parameter, not a new stage.

**Follow the shipping precedent**, do not invent one: `enforce_authored_coloring` /
`AuthoredColoringContext::allows`, called from `convert_infill_output`
(`crates/slicer-wasm-host/src/marshal/out.rs`), is default-deny per `(object_id, region_id)`
and strips ungranted output at the commit boundary. It is the only producer-output-vs-ownership
check in the tree.

### W4 — Make the `anchor_z` coincidence an enforced invariant

`SupportPlanIR::duplicate_region_identity` (`crates/slicer-ir/src/slice_ir.rs`) keys on
`(global_layer_index, object_id, region_id)` and ignores `anchor_z`, while the union key
**includes** `anchor_z` — deliberately, per DEV-162, because merging across distinct declared
planes broke off-grid support. The two agree today only because synthetic negative layer
indices are minted per `anchor_z` (`intermediate_plane_indices`), making `global_layer_index` a
function of `anchor_z` by construction. Assert that relationship at the producer so a future
change to plane minting fails loudly there instead of mysteriously at commit.

### W5 — Retire the interim producer-side merge

Packet 241 landed `merge_region_identity_entries`
(`modules/core-modules/traditional-support-planner/src/lib.rs`) as a temporary unblock so `agg`
does not abort real-mesh slices. It unions on
`(global_layer_index, object_id, region_id, anchor_z)` before publish and preserves geometry
exactly. Once W1-W3 make ownership real at the seam, decide whether the producer-side merge is
still wanted as defence in depth or should be removed. See DEV-167.

### W6 — Restore the intent of two packet-239 tests

`coarse_same_region_sources_keep_distinct_body_membership` and
`coarse_source_preference_keeps_mixed_source_memberships`
(`modules/core-modules/traditional-support-planner/tests/traditional_family_tdd.rs`) assert that
TWO entries share one identity triple — the shape the IR forbids. They fail under 241's interim
merge and were left RED by human decision, with AC-N2 red, rather than rewritten under a packet
that did not own the semantics. The behaviour they protect is real and must survive: both source
body memberships surviving, and the body-only / interface-only preference of
`select_coarse_source_entries` remaining observable. Restore that intent against whatever shape
this packet settles on.

### W7 — Documentation defects found while investigating

- `docs/02_ir_schemas.md` section "Config Precedence Rules" (IR 3) states
  `layer-range override > modifier > object config > global default`, while section "Config Key
  Namespaces" (IR 5) plus the Modifier Resolution Contract give
  `global < object < modifier < paint < tool`. **The two chains contradict each other**, and no
  implementation of a `layer-range override` level was found.
- The Modifier Resolution Contract cites `stamp_modifier_config_deltas`; the live symbol is
  `stamp_modifier_sub_region_configs` (`crates/slicer-core/src/algos/region_mapping.rs`).
- **The paint and tool levels of the hierarchy are inert.** No shipped module manifest declares
  `[[region_split]]` (only scheduler test fixtures do), so `aggregated_region_split` is empty in
  a stock slice, `enumerate_canonical_chains` yields only the empty chain, and
  `paint_config:*` / `tool_config:*` overlays never fire — for any feature, not just support.
  Determine whether that is intentional staging or an unnoticed regression before filing it as a
  bug; it is out of scope to FIX here beyond recording it.
- `docs/adr/0059-support-families-and-anchored-entities.md` folded in Ruling 1's
  "`family_assignments` are minted per RegionMap region" but dropped its exclusivity half
  ("exactly one attributed plan entry"). That omission is why this invariant was easy to violate
  unnoticed.

## Out of Scope

- The AGG rasterizer port itself, the `support_area_rasterizer` knob, and the DEV-166
  block-snapping divergence — all owned by `241-support-agg-rasterizer`.
- Fixing the inert paint/tool axis (W7) — record and diagnose only; the fix belongs to a packet
  that owns config resolution.
- Raft, independent support-layer Z, renderer flow/density — packets 240a/240b, 239, 238c.

## Open Questions for Grilling

1. Should region-scoped claim holders be constructed for **every** claim, or only where a
   per-region owner is meaningful? Building them for all claims makes a currently-empty pass
   suddenly load-bearing across every stage at once.
2. Does W3's verification **reject**, **clip**, or **degrade-and-diagnose** a trespassing entry?
   `AuthoredColoringContext` strips silently; support aggregation already has a
   `FamilyConflictPolicy` with `Fail` / `Degrade` arms and a territory-clipping path (DEV-165).
3. Is `region_id` alone the right union key, or `(region_id, anchor_z)`? W4's invariant makes
   them equivalent today; stating which one is authoritative decides whether off-grid planes
   are identity-distinct.
4. Cross-family competition for one region is currently resolved by **arrival order**
   (`arrival_owners`, keyed `(plan_index, entry_index)`) — an accident of plan indexing, not a
   design. Config resolution already assigns family per region through
   `global < object < modifier`, so genuine two-family competition for one region may be
   unreachable. If so, is arrival order dead code to delete rather than a hierarchy to build?
5. Does W1 subsume W3, or are they independent? A live region-scoped claim pass may already
   make a self-declared `family_id` unforgeable, or may not — claims are validated at schedule
   time, `family_id` is written at publish time.

## Acceptance Criteria

TBD — none written. Every criterion must end with a runnable pipe-suffixed verification command,
and this packet changes enforcement behaviour, so it needs at least one negative/rejection
criterion (a trespassing entry must be provably refused, not silently accepted).

## Prerequisites

- `241-support-agg-rasterizer` reaching a settled state (it closes `draft`, AC-N2 red, with
  DEV-167 filed against this packet).
- Re-derive DEV-167's status and the `241` AC-N2 result from disk before activation; both are
  ledger facts and will have moved.
