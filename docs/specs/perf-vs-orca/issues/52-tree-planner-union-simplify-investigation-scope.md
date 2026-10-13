# Tree-planner union/simplify investigation scope

Type: grilling
Label: wayfinder:grilling
Status: resolved
Assignee: agpen (current OpenCode wayfinder scope session), 2026-10-06
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none

## Question

What bounded investigation should establish whether the tree-planner emit
pass's union/simplify chain admits representation-safe call reduction on the
current matched supports-on jobs?

Set the fixture/mode coverage, diagnostic and exactness boundaries, stop rules
and handback before any captures or candidate implementation. Historical
[Tree-planner substage attribution](22-tree-planner-substage-attribution.md)
motivates this lead; it is not a current-tree speed claim.

## Comments

### Route selection and claim — 2026-10-06

The human selected **Scope union/simplify** in the live resumption exchange.
Claimed before scope investigation. The route-only claim on
[offset2_ex call reduction](19-offset2-ex-call-reduction.md) is released and its
park remains unchanged. This selection authorizes scoping only: no temporary
probes, slices, timing campaign, production candidate, cache retry, adoption
retry, default change or commit has been authorized.

### Verified scope facts

- `build_roles`'s `with_areas` closure
  (`modules/core-modules/tree-support-planner/src/lib.rs`) carves each role's
  input, calls `union_expolys`, conditionally calls `expolygons_simplify`, then
  applies the set-wide collision difference. Body-minus-interface differences
  follow the separate body/roof/base/floor chains. Do not attribute those
  surrounding differences to union or simplify.
- `union_expolys` in that file returns sets with at most one polygon unchanged;
  larger sets use `host::clip_polygons(..., ClipOperation::Union)`.
  `expolygons_simplify` is guest-side ring simplification, not the host
  batch-simplify service. `role_simplify_tolerance` enables it only for the body
  chain above `COARSE_CIRCLE_NODE_THRESHOLD`; actual activation on these jobs
  remains unmeasured.
- `simplify_ring` in that file closes the ring at its first vertex before
  `douglas_peucker_open`. Ring representation therefore matters, independently
  of filled-area equality. The existing carve gate's lone-region repair stays
  intact; see [Emit-pass carve findings](../evidence/t32-carve-gate/FINDINGS.md).
- `t32_gate_chain_gate_matches_unconditional` and
  `build_roles_merges_overlapping_node_cross_sections_into_one_outline` in that
  file provide useful existing regressions, not a ready-made real-job oracle
  for a new union reduction. `integrated_parity_support_planner_native_matches_wasm`
  (`crates/slicer-runtime/tests/contract/integrated_parity_support_planner_tdd.rs`)
  is a nonvacuous structural dispatch check, not an end-to-end exactness or speed
  gate for the matched jobs.
- Both `tmp/3dbenchy.stl` and `tmp/base.stl` were found by main-session directory
  inspection. This corrects the read-only exploration agent's missing-fixture
  report. Presence is verified; model hashes, current output identity and
  artifact freshness have not been checked in this scoping session.

## Answer — resolution comment, 2026-10-06

The human selected **Fresh split**, **Exact current chain**, and then
**Confirm** in the live scope exchange. Authorize a separate diagnostic take:

- Benchy and base with
  `evidence/matched-pair/configs/pnp-classic-supports-on.json`, ordinary and
  accelerated external Developer snapshots. No Arachne or Orca timing matrix,
  candidate implementation or speed acceptance campaign is included.
- Refresh attribution before proposing reduction. Separate host union execution,
  conversion/round-trip overhead, guest simplification and surrounding work
  with non-overlapping accounting, explicit clock/fuel boundaries and role/input
  counters. Instrumented wall is attribution only, not a speed claim; guest
  fuel cannot measure the host boolean work.
- Preserve the exact current chain: role separation, ordered contours/holes,
  simplification enablement/tolerance, carve normalization/repair and subsequent
  differences. Independent-request batching is not geometric merging. Any
  skipping, batching, merging or reuse remains a hypothesis for a later scope.
- Record real intermediate inputs/outputs and unmodified current-chain results
  as the basis for a later independent replay oracle. Area equality or final
  G-code alone is insufficient to certify a future candidate. No schema or
  production contract changes are authorized.
- Derive fresh post-repair baseline output identities. Require both mode-specific
  guest freshness gates, external provenance, clean completion and nonvacuous
  support geometry. Stop on missing inputs, identity/freshness failure or output
  drift; do not silently substitute fixtures, normalize away differences or
  relax checks. Do not reuse retired timing references.
- Remove diagnostic probes, restore production sources/artifacts and verify
  restoration plus both freshness gates. Keep headline evidence and a
  falsifiable reducer/verifier in-repo; heavy models remain uncommitted.
  Return attribution, representation findings, gaps and a recommendation only.
  No automatic retry, optimization, KEEP decision, commit, adoption retry or
  default change is authorized. Other parked leads remain parked.

Execution is graduated to
[Tree-planner union/simplify current-job attribution](53-tree-planner-union-simplify-current-job-attribution.md),
unclaimed for a later session. This scope decision is this session's sole
ticket resolution.

Validation at this boundary: read-only source/document/config inspection and
model-presence verification; local-link, status/dependency, unclaimed-handoff
and map-pointer assertions passed, as did `git diff --check`.
No builds, tests, guest freshness checks, probes,
slices or timing runs. Current cost and potential savings remain unmeasured;
production sources/artifacts and pre-existing working-tree changes preserved.
