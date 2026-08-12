# Support Families and Anchored Entities - Approved Remediation Plan

Status: approved (2026-08-12, grill-with-docs session)

Sources:

- `docs/adr/0059-support-families-and-anchored-entities.md`
- `docs/specs/support-generation-defect-verified-findings.md`
- `docs/specs/support-generation-remediation-plan.md`
- OrcaSlicer comparison bundles under `target/vd-orca-tree-compare/` and
  `target/vd-orca-normal-compare/` (disposable evidence; regenerate before use)

## Problem

The current support repair cannot be closed safely as a local lone-node or
renderer fix. On `tmp/SupportTest.stl`, the tree path reaches the plate only as
one oversized body centered near `(3.0, 9.808)` whose swept radius intersects
the model. OrcaSlicer produces distributed tree bodies or broad traditional
support beneath the overhang instead. The divergence begins before final path
rendering.

The defect exposed a deeper contract mismatch:

- `com.core.support-planner` is named generically but implements a tree-specific
  centroid-contact, MST, tapering, and smoothing algorithm.
- It runs when support is enabled even when traditional support is selected;
  `support_type` selects only the `Layer::Support` renderer.
- Traditional support is treated as a per-layer filler even though contact
  detection, downward propagation, interfaces, and termination are cross-layer
  planning concerns.
- A third support mechanism cannot register and atomically select a matching
  planner and renderer without scheduler source changes.
- `SupportPlanIR` carries tree branch extrusion paths rather than universal
  structural support geometry. A trunk diameter is consequently interpreted as
  one extrusion width.
- The current global-layer output shape cannot represent independent support
  heights as ordered work associated with one global layer.

## Feedback Loop

The existing typed capture supplies a deterministic red-capable replay check:

```bash
node -e "const fs=require('fs'); const m=JSON.parse(fs.readFileSync('target/vd-tree-fixed/manifest.json','utf8')); const e=m.images.find(x=>x.tap==='Layer::Support'&&x.layer_index===0); const p=e.typed_capture.value.support_paths; const bad=p.length===1&&p.some(path=>path.points.some(q=>q.x-q.width/2<0)); if(bad){console.error('FAIL: layer 0 collapsed to 1 support path and its swept bead overlaps the pillar footprint');process.exit(1)} console.log('PASS')"
```

This replay is evidence, not the final regression seam. The implementation
sequence must replace it with a fixture-driven plan test that runs the real
planner and checks complete support-body polygons against exact-Z model
occupancy.

## Domain Model

The canonical terms are defined in `CONTEXT.md`:

- **Support candidate**: host-identified unsupported or enforced surface.
- **Support demand**: a candidate accepted by its assigned family.
- **Feasible support envelope**: baseline space left after shared hard
  constraints; family planners must tighten it for their geometry.
- **Support routing cell**: host-assigned feasible territory for one family and
  demand group.
- **Support family**: one strategy planner and matching layer renderer selected
  as a unit.
- **Support body**: one connected cross-layer structure serving one or more
  demands.
- **Anchored entity**: printable work assigned to a global layer for execution
  ordering while declaring its own planar Z or continuous Z span.
- **Sublayer**: a discrete anchored execution subdivision that does not create a
  model slice at its physical Z.
- **Z-spanning print entity**: one atomic anchored entity whose continuous path
  crosses multiple global-layer height intervals.

## Architecture Decisions

### 1. Preserve the global layer as the parallel work unit

Global layers remain the authoritative worker and ordering barriers. Work for
different global layers may still be generated in parallel from immutable
prepass state.

Each global-layer worker returns an ordered list of event collections rather
than exactly one flat `LayerCollectionIR`:

1. Planar anchored entities below the model plane, ordered by physical Z.
2. The ordinary same-Z model event, including same-Z support.
3. Future locally ordered anchored entities, including atomic Z-spanning
   entities where their feature contract requires them.

No cross-layer scheduler is introduced. A Z-spanning entity executes at its
anchor global layer's normal position after all prior global layers, remains
atomic, and may contain path points outside the anchor layer's model Z envelope.

### 2. Add generic anchored entities before changing support contracts

An anchored entity carries at least:

- a stable ID local to its anchor;
- `anchor_global_layer_index`;
- a declared geometry contract: planar `z`, or atomic Z-spanning
  `min_z`/`max_z`;
- available input capabilities and requested output capabilities;
- provenance identifying the requesting feature and source plan entry.

The scheduler derives the applicable stage closure from capabilities rather
than a hardcoded event-kind table or an explicit feature-owned stage list. The
existing `layer-parallel-safe` hint keeps its name and governs concurrent
invocations across anchored work as well as ordinary model layers.

Path commit validation follows the entity contract:

- planar output must lie on the declared plane within coordinate tolerance;
- Z-spanning output must remain within its declared range and retain atomic
  continuity;
- the old assumption that every output point lies in the model layer's
  `[layer.z, layer.z + effective_layer_height]` envelope no longer applies to
  declared Z-spanning entities.

Each planar support event runs path optimization independently and contributes
normal event-level time and cooling accounting. Same-Z support joins the
ordinary model event and uses its existing ordering.

Rafts are not anchored entities. They retain signed negative global-layer
prefix entries (`-1..=-raft_layers`) and execute before non-negative model
layers, preserving ADR-0009 and packet 215's scheduling contract.

### 3. Split host analysis from support-family strategy planning

Add host-owned `PrePass::SupportAnalysis`, producing a new
`SupportAnalysisIR`. It contains strategy-neutral inputs:

- conservative support candidates and source evidence;
- explicit enforcer and blocker annotations;
- model occupancy and eligible model/plate termination surfaces;
- shared support settings and candidate assignment metadata;
- a baseline feasible support envelope containing only shared hard
  constraints;
- deterministic per-region family assignments.

Candidates are optional inputs to a family. An enforcer guarantees candidate
creation, not printed support. A family may decline a candidate or fail to find
a route, but must return a structured reason such as `declined-policy`,
`no-route`, `blocked`, or `unsupported-mode`. Such omissions are degraded
success, not fatal slicing errors.

The host analysis does not propagate final support bodies. Traditional and tree
strategies occupy different subsets of feasible space and apply different
clearance, merging, interface, and termination policies. Making the host emit a
mandatory downward body would embed one family algorithm in the generic stage.

### 4. Provide normalized exact-Z host geometry queries

Family planners may select support planes between model planes. While planning,
they query a host service for exact-Z model occupancy and baseline envelope by
object, region, and physical Z.

The host normalizes requested Z to canonical coordinate units and caches
immutable results so multiple families do not repeat mesh cross-section and
shared envelope work. The service returns occupancy, blockers, eligible
termination geometry, and the baseline envelope. The family remains responsible
for tightening those results for its actual branch radius, extrusion geometry,
XY gap, and routing policy.

This contract directly prevents recurrence of the current fixed-inflation bug,
where avoidance is computed from one base radius while a lower tapered branch
has a larger physical radius.

### 5. Make `SupportPlanIR` a universal structural geometry carrier

Retain the `SupportPlanIR` name but replace its tree-extrusion-path semantics.
One complete family-planner invocation emits plan entries containing:

- family ID and source object/region attribution;
- stable support-demand IDs;
- stable cross-layer support-body IDs;
- anchor global-layer index and physical Z;
- semantic `ExPolygon` regions by standard role, initially support body, top
  interface, bottom interface, and raft-related roles where applicable;
- optional structural centerline graphs with local radii and body references;
- anchored-entity capabilities and provenance;
- declined-candidate records with structured reasons.

The skeleton is structural, not a printable extrusion path. Renderers may use
or ignore it when body polygons are sufficient. No planner emits nozzle-width
toolpaths into `SupportPlanIR`.

The fixed universal geometry vocabulary is the extension boundary. An external
family that cannot express its output using the host's supported polygon,
skeleton, role, and metadata schema requires a future additive host/schema
version and must fail compatibility checks on an older host. It must not rely on
opaque family bytes or private role IDs.

### 6. Select planner and renderer as one support family

Support mechanisms remain modules in two pipeline phases:

- `tree-support-planner` and `traditional-support-planner` run at modular
  `PrePass::SupportGeometry`.
- `tree-support` and `traditional-support` render validated family plan entries
  during anchored `Layer::Support` execution.

Manifests use both role and family claims:

- planners hold `support-planner` and `support-family:<id>`;
- renderers hold `support-generator` and the same `support-family:<id>`;
- the loader retains all per-region candidates rather than globally dropping
  all but one planner and renderer;
- one region-overridable `support_family` selector resolves both roles
  atomically;
- existing `support_type` values are compatibility aliases:
  `normal*`/`classic*` map to the traditional family and
  `tree*`/`hybrid*` map to the tree family;
- auto/manual/hybrid behavior is family configuration, not a separate family.

A missing or mismatched planner-renderer pair is a startup error. The current
per-layer fallback fillers are removed; absence of a plan cannot silently select
another algorithm.

### 7. Dispatch family planning per region without planner negotiation

The host invokes each selected family planner once per object. Every planner
sees the whole object, all candidate/family assignments, and resolved region
config views, but may emit support bodies only for demands assigned to its own
family.

The host owns multi-writer aggregation into one immutable `SupportPlanIR` and
validates family attribution. Planners do not read and transform each other's
plans and do not negotiate shared structures in the first implementation.

The host partitions baseline feasible space into deterministic, non-overlapping
support routing cells using candidate assignment and proximity. Cells assigned
to several regions of the same family may be unioned. One same-family body may
serve demands from several source regions while preserving every demand ID.
Crossing another family's routing territory requires a future explicit sharing
contract and is not permitted initially.

### 8. Validate and degrade before rendering

After all family plans are aggregated, the host validates every body against:

- exact-Z model occupancy and the planner's permitted routing cells;
- declared body and anchor identity;
- complete body polygons, not only skeleton center points;
- positive-area overlap with bodies from another family.

Invalid bodies are dropped as complete cross-layer structures, and their
attached demands become unmet with structured diagnostics. Positive-area
cross-family overlap drops both bodies rather than selecting a winner. Boundary
touching within coordinate tolerance is allowed. If routing cells cannot
separate mixed-family demands, those candidates remain unmet and slicing
continues degraded.

Module crashes, malformed schemas, invalid family pairing, and impossible host
contract states remain fatal. Unroutable support does not abort a slice merely
because no valid body can be generated.

### 9. Render attributed support per anchored event

For each anchored event, the host groups validated plan entries by family and
invokes each active family renderer once. The renderer converts semantic body
and interface polygons into nozzle-width walls, infill, and printable paths.

`SupportIR` becomes structured rather than flat. One entry per body/role carries
family ID, body ID, source demand IDs, object/region attribution, role, and
extrusion paths. This identity survives tool selection, diagnostics,
visual-debug, path optimization, and G-code emission.

After all family renderers finish one physical event, a host commit hook checks
cross-family swept-path overlap. Conflicting rendered bodies are dropped and
their demands marked unmet. The host does not silently clip paths or allow
renderers to reorder across planar event boundaries.

### 10. Implement real tree and traditional family planners

The tree planner must replace triangle-centroid-only contacts with distributed
overhang-area sampling, including corner, contour, and interior coverage. It
must use radius-aware exact-Z collision/avoidance, validate full body circles or
polygons, route to eligible plate/model termination surfaces, produce body and
interface polygons, and preserve same-family demand merging. The tree renderer
generates printable trunk walls/fill from polygons; it does not emit trunk
diameter as one extrusion width.

The traditional planner must perform cross-layer contact-area detection,
downward base propagation, interface generation, obstacle handling, and
termination planning. The traditional renderer scan-fills only the planned
body/interface polygons. It never fills `region.polygons()` or derives support
eligibility independently during the parallel layer stage.

Both families target behavioral OrcaSlicer parity: demand coverage, collision
freedom, valid reachability, interfaces, independent support heights, and
printable body construction. Exact path identity is not required.

## Required Invariants

The superseding packet sequence must establish automated checks for all of the
following:

1. Every support body polygon and rendered nozzle sweep is disjoint from
   exact-Z model occupancy, except an explicitly modeled contact tolerance.
2. Accepted demands are connected through one or more attributed bodies to an
   eligible build-plate or model termination surface.
3. Declined or unroutable candidates have structured reasons and produce no
   colliding fallback geometry.
4. Every family plan entry is emitted only for demands assigned to that family.
5. Missing or mismatched planner-renderer families fail before slicing.
6. Same-family bodies may merge while preserving all source demand IDs.
7. Different-family bodies have no positive-area overlap; invalid bodies are
   dropped completely and slicing is marked degraded.
8. Planar anchored outputs lie on their declared Z and are ordered before their
   upper anchor's model event.
9. A future Z-spanning entity can remain atomic outside its anchor's model-layer
   Z envelope and still execute at the anchor's normal layer position.
10. Same-Z support participates in ordinary model-event path ordering.
11. Every event is path-optimized and cooling/time-accounted without reordering
    across physical event boundaries.
12. Forced-serial and forced-parallel generation produce identical ordered
    event collections and support geometry.
13. Disabling support produces no support candidates, plans, anchored support
    events, or support paths.
14. Traditional and tree output for `tmp/SupportTest.stl` reaches the build
    plate, remains beneath the overhang, and does not enter the pillar.

## Visual And Differential Gates

Every geometry-changing packet must include model-backed `visual-debug` taps
for its own new boundary. The complete closure comparison must regenerate and
inspect, not merely grep, matched-height views for:

- host support analysis candidates, occupancy, envelope, and routing cells;
- aggregated family `SupportPlanIR` body/interface polygons and skeletons;
- each anchored `Layer::Support` event and structured `SupportIR` output;
- final PNP G-code;
- standalone Orca tree and normal G-code references.

The decisive fixture remains `tmp/SupportTest.stl`, with Orca references
`tmp/SupportTest_Tree_Orca.gcode` and
`tmp/SupportTest_Normal_Orca.gcode`. Packet authors must verify fixture
availability and regenerate disposable bundles. PNG existence, byte size,
manifest greps, and self-captured goldens are not sufficient evidence.

## Supersession And Compatibility

- Packet `213-support-planner-defect-fix` and reopened `TASK-329` are superseded
  by this sequence. Its lone-node and radius-floor work may be retained only
  where it remains valid inside the new tree family; its degenerate-disk visual
  result is not closure evidence.
- Packet `214-support-fallback-overhang-clip` is superseded by this sequence.
  The old remediation plan's fallback-filler fix is superseded. Removing the
  fallback and adding a traditional family planner replaces clipping
  `region.overhang_areas()` inside a per-layer filler; the marshalling
  `needs_support` derivation moves into `PrePass::SupportAnalysis`.
- Packets `210a-support-planner-coord-t` (DEV-128) and
  `210b-support-interface-bottom-layers` (DEV-129) are superseded by this
  sequence. DEV-128's scaled-integer geometry work is absorbed into the
  `tree-support-planner` rewrite (packet 221) only where it remains valid
  inside the new tree family; DEV-129's bottom-interface bands become a
  standard semantic role in the structural `SupportPlanIR` (packet 220) and
  both family planners (packets 221/222) emit interface polygons, retiring the
  code-1003 stub through the family-contract migration rather than a post-pass
  over the old `SupportPlanEntry` rows. Their directories are retained intact
  for provenance; do not implement as-is.
- Existing `SupportPlanIR` branch-path consumers, schema versions, WIT views,
  macros, host marshal code, tests, visual-debug captures, and documentation
  require an explicit additive-or-breaking migration decision during contract
  packet authoring. Do not preserve path semantics under the same schema
  version.
- Existing Orca profile and 3MF `support_type` values remain compatible through
  aliases to `support_family`.
- Raft negative-prefix scheduling and `claim:raft-fill` remain governed by
  ADR-0009 and packet 215. Anchored entities must not silently absorb or replace
  those contracts.
- `TASK-163b-orca-ref` remains the backlog owner for replacing stale
  self-captured support goldens with authoritative Orca references. The final
  differential packet must either close it with real evidence or document its
  remaining external blocker without claiming exact parity.

## Packet Authoring Rules

The next session must use the spec-packet generator's Batch Protocol and author
the queue in dependency order. Before generating the first packet it must:

1. Allocate new canonical task IDs in `docs/07_implementation_status.md` for
   rows 1-6 below. No existing task except reopened `TASK-329` covers these
   architectural slices, and unrelated closed IDs must not be reused.
2. Mark packet 213 and `TASK-329` superseded by the generated sequence using the
   packet-safety rules; do not overwrite or delete packet 213.
3. Re-ground every load-bearing IR, WIT, stage, scheduler, raft, and visual-debug
   symbol against the live tree. Names below describe approved contracts, not a
   claim that the symbols already exist.
4. Delegate all OrcaSlicer source inspection and preserve returned file/line
   locations in parity packets.
5. Split any packet whose implementation plan contains an L-sized step. In
   particular, anchored entities may require internal decomposition after the
   struct-literal and stage-commit blast radius is measured.

## Packet Queue

Task IDs are allocated (2026-08-12): rows 1-6 map to `TASK-330`..`TASK-335`, registered in `docs/07_implementation_status.md`. Row 1 = TASK-330, row 2 = TASK-331, row 3 = TASK-332, row 4 = TASK-333, row 5 = TASK-334, row 6 = TASK-335.

| # | packet slug | goal (one sentence) | task ids | depends on | status | packet dir |
|---|-------------|---------------------|----------|------------|--------|------------|
| 1 | anchored-entity-execution | Add generic planar and atomic Z-spanning anchored entities, capability-derived per-anchor event execution, ordered event collections, contract-aware Z validation, per-event optimization, and cooling/time accounting while retaining global layers as parallel workers. | TASK-330 | - | generated | `docs/spec_packets/219-anchored-entity-execution/` |
| 2 | support-analysis-family-contracts | Add `PrePass::SupportAnalysis`, normalized exact-Z occupancy/envelope queries, universal structural `SupportPlanIR`, structured `SupportIR`, per-region atomic family claims/selection, host-owned plan aggregation, and remove missing-plan fallback semantics. | TASK-331 | #1 | generated | `docs/spec_packets/220-support-analysis-family-contracts/` |
| 3 | tree-support-family | Split and rename the current planner into `tree-support-planner`, implement distributed contacts plus radius-aware collision-safe body/interface polygons and anchored support heights, and render printable tree bodies through the paired `tree-support` renderer. | TASK-332 | #2 | generated | `docs/spec_packets/221-tree-support-family/` |
| 4 | traditional-support-family | Add `traditional-support-planner` for contact detection, downward base/interface propagation, obstacle-safe termination, and anchored support heights, then make `traditional-support` render only its planned polygons. | TASK-333 | #2 | generated | `docs/spec_packets/222-traditional-support-family/` |
| 5 | mixed-support-family-routing | Implement deterministic host routing cells, same-family cell/body unions, cross-family plan and nozzle-sweep overlap rejection, structured degraded diagnostics, and per-region mixed-family validation. | TASK-334 | #3,#4 | generated | `docs/spec_packets/223-mixed-support-family-routing/` |
| 6 | support-family-orca-closure | Add fixture-driven invariants and regenerate visually inspected model/G-code differential evidence proving both built-in families reach valid termination surfaces without model collision and preserve correct roles through final G-code. | TASK-335 | #3,#4,#5 | generated | `docs/spec_packets/224-support-family-orca-closure/` |

## Queue Exports

Packet authors must treat these as dependency contracts:

- **#1 exports:** anchored-entity IR, planar/Z-spanning geometry contracts,
  capability-derived event closure, ordered event collections, and runtime hooks
  for per-event optimization/accounting.
- **#2 exports:** `SupportAnalysisIR`, exact-Z host query service, universal
  `SupportPlanIR`, structured `SupportIR`, family claim/selection rules,
  planner/renderer pairing validation, family planner WIT/SDK contracts, and
  host aggregation/dispatch seams.
- **#3 exports:** complete tree planner-renderer family and tree-specific
  behavioral invariants.
- **#4 exports:** complete traditional planner-renderer family and
  traditional-specific behavioral invariants.
- **#5 exports:** deterministic multi-family routing/ownership and degraded
  conflict handling.
- **#6 exports:** authoritative closure evidence and supersession completion for
  packet 213 / `TASK-329`, plus disposition of `TASK-163b-orca-ref`.

## Out Of Scope

- Implementing a nonlinear perimeter, non-planar wall, milling, inspection, or
  other future anchored-entity producer. The generic substrate supports them;
  this plan implements only support producers.
- A global cross-layer entity scheduler. Global-layer ordering remains the
  execution barrier.
- Planner-to-planner negotiation or cross-family structural sharing.
- Opaque family payloads, private role IDs, or host execution of family-owned
  algorithms.
- Exact Orca toolpath identity. Behavioral parity and collision-safe printable
  geometry are required.
- Replacing signed negative raft-prefix global layers with anchored entities.
- Silent clipping of invalid family geometry, emergency fallback fillers, or
  allowing model collisions to preserve support coverage.
