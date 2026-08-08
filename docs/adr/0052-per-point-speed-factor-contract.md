# ADR-0052 — Per-point speed is a `Vec<f32>` of *factors* carried in an `entity_id`-keyed side table on `LayerCollectionIR`

<!-- filename: 0052-per-point-speed-factor-contract -->

## Status

Accepted (2026-07-25). Authored for
`docs/spec_packets/_OLD/189-per-point-speed-factor-carrier.md`, which takes two contract
decisions and stands in unacknowledged tension with three accepted ADRs. The
packet mentions none of the three; reconciling them is the substance of this
record.

## Context

`crates/slicer-gcode/src/emit.rs`'s `DefaultGCodeEmitter::resolve_feedrate` is
the single place a speed factor becomes an `F` token. It takes
`(&ExtrusionRole, f32)` — a role and a factor — selects the per-role base speed,
applies a `clamp(0.05, 5.0)` to the factor, and returns the feedrate. Today the
factor is `entity.path.speed_factor`: one value for a whole entity, written by
`EntityMutation::SetSpeedFactor` through
`FinalizationOutputBuilder::apply_to`'s `MergeOp::ModifyEntity` arm.

Packet 190 needs a *continuous* overhang speed that varies **along** a single
wall loop. A whole-entity factor cannot express that. Two questions follow, and
they are contract questions, not implementation details:

1. What does a per-point speed value *mean* — a factor, or an absolute mm/s?
2. Where does the per-point vector *live* — on `Point3WithWidth`, or in a side
   table?

Both cross the finalization→emit tier boundary: the value is produced by a
`FinalizationModule` running in a WASM guest and consumed by the native
serializer.

## Decision

### 1. `EntitySpeedProfile.factors` is `Vec<f32>` of FACTORS, not absolute mm/s

```rust
pub struct EntitySpeedProfile { pub entity_id: u64, pub factors: Vec<f32> }
```

Each element is a multiplier against the same per-role base speed
`resolve_feedrate` already selects, on the same scale as
`path.speed_factor`. The emitter resolves each point as
`self.resolve_feedrate(role, profile.and_then(|p| p.get(original_index).copied()).unwrap_or(entity.path.speed_factor))`.
`resolve_feedrate`'s signature and body are **unchanged**.

This keeps `resolve_feedrate` the single place speed is resolved across the tier
boundary: base-speed selection and the `clamp(0.05, 5.0)` stay host-side, in one
function, for both the whole-entity and per-point paths. A producer that has
computed an absolute mm/s must divide by the same role base speed it already
uses.

**Changing this to absolute mm/s is a contract change, not a refactor.** It
requires a new acceptance criterion and a `resolve_feedrate` signature change,
and it moves clamping and base-speed policy into every producing module. Do not
make it silently.

`EntitySpeedProfile.factors.len()` MUST equal the target entity's
`path.points.len()` at the moment the mutation is applied; `apply_to` enforces
it with an `Err` and nothing downstream re-checks. A second
`SetPointSpeedFactors` for the same `entity_id` **replaces** the row rather than
appending, so `speed_profiles` holds at most one row per entity — this is what
lets packet 191 submit a geometry mutation followed by a resized profile for the
same entity.

### 2. The carrier is an `entity_id`-keyed side table on `LayerCollectionIR`

`#[serde(default)] pub speed_profiles: Vec<EntitySpeedProfile>` on
`LayerCollectionIR`, `speed_profiles: Vec::new()` in its explicit `Default` impl,
`CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION` bumped additively.
**Not** a per-point field on `Point3WithWidth`.

The precedent is `TravelMove` on the same struct, whose first field is
`pub entity_id: u64` precisely so that finalization-stage sorting and insertion
cannot dangle the anchor. (`TravelRetract` is deliberately *not* cited as a
second precedent: its anchor is `pub after_entity_index: u32`, a positional
index that was never converted. One true example is worth more than two of which
one is false.) The emitter already builds `travel_moves_by_entity` in the same
loop, so `speed_profiles_by_entity` is a two-line addition beside an identical
one.

An absent profile means uniform speed: an entity with no row emits exactly the
`F` values it emits today. That compatibility lock is the load-bearing property
of the whole design.

## Reconciliation with prior decisions

This is the part packet 189 omits, and the reason this ADR exists.

### ADR-0032 rejected exactly this WIT/IR churn — the distinguishing principle

[ADR-0032](./0032-curled-edge-slowdown-shares-overhang-speed-table.md)'s second
decision reads, verbatim:

> **Compute `curled_height` transiently, not as a persisted field.** Unlike `overhang_quartile`
> (a genuine cross-module IR/WIT field, since it's produced by a PrePass stage and consumed by a
> different Layer-tier module across the WASM guest boundary), curl estimation and its consumption are
> both computed inside a single `FinalizationModule::run_finalization` call … Nothing else in this codebase reads `curled_height` back out, so
> adding a WIT field, an IR schema-version bump, and marshal fan-out across every `Point3WithWidth`
> construction site would be speculative work against a need that doesn't exist.

Packet 189 adds a WIT variant case, an IR schema-version bump, and a fan-out
across the tree. On its face that is the rejected shape.

**The distinguishing principle is the boundary, and ADR-0032 states it itself.**
ADR-0032's own criterion is not "avoid IR fields"; it is that curl was computed
**and** consumed "inside a single `FinalizationModule::run_finalization` call" —
"there is no boundary to cross, so persistence would be unused surface area", as
its Alternatives put it. Per-point speed is the opposite case on precisely that
test: it is produced in a WASM guest at `PostPass::LayerFinalization` and
consumed by `DefaultGCodeEmitter` in native code. **The WIT actually separates
those two tiers** — the guest's only channel out is
`FinalizationOutputBuilder`, and a value that does not travel through it does
not exist for the emitter. ADR-0032's word for the qualifying case is "a genuine
cross-module IR/WIT field", which is what this is.

So ADR-0032 is **not** overturned, contradicted, or narrowed by this ADR. It is
applied. Its test — *does the value cross a boundary the WIT enforces?* —
returns "no" for `curled_height` and "yes" for a per-point speed factor. Anyone
citing ADR-0032 against a future IR field should apply that test rather than the
conclusion.

### ADR-0048 chose a per-point field for `dist_to_top_mm`; this chooses a side table

[ADR-0048](./0048-packet-119-closure-dist-to-top-and-raft-plan.md) added
`dist_to_top_mm: f32` directly to `Point3WithWidth`, on the ground that
"Per-point is the shape needed by downstream consumers and the current wedge
harness", and its Future-Reviewer Notes instruct: "**Do not re-suggest per-entry
`dist_to_top_mm`.** Per-point is the Orca-aligned shape and the only one that
captures chain monotonicity."

**This is not a supersession.** ADR-0048 is scoped to `dist_to_top_mm`, and that
field stays exactly where it is. But the divergence is real and must be recorded
rather than left for someone to discover as an apparent inconsistency:

- **A per-point field on `Point3WithWidth` is correct when the value is a
  property of the point itself and every point has one** — geometry-intrinsic
  data that must survive interpolation, insertion, and resampling, because any
  code synthesising a new point must be forced to supply it. `dist_to_top_mm`
  and `overhang_quartile` are that shape. Its cost is an exhaustive struct-literal
  sweep across the tree, paid once per field.
- **An `entity_id`-keyed side table is correct when the value is sparse,
  entity-scoped, and optional** — present for some entities and absent for most,
  meaningless without an owning entity, and required to round-trip unchanged for
  every entity that has no value. `speed_profiles` and `TravelMove` are that
  shape. Its cost is a lookup and a staleness question; its benefit is that
  entities without a profile are untouched, which is what makes the
  byte-compatibility lock provable.

Note that [ADR-0053](./0053-overhang-emission-time-speed-sections.md)'s ruled
option (C) goes back to the per-point shape for `overhang_distance_mm` — under
the first rule above, correctly: it is a geometry-intrinsic distance stamped for
every point by a prepass. The two decisions are not in conflict; they are the two
rules applied to two different kinds of value. State which rule you are applying
before choosing a shape.

### ADR-0008 recorded "No WIT contract change"; packet 189 makes one

[ADR-0008](./0008-overhang-as-finalization-module.md)'s Consequences open with:

> - **No WIT contract change**: existing 20 core-modules are unaffected (no rebuild churn).

Packet 189 adds `set-point-speed-factors(list<f32>)` to `variant entity-mutation`
in `crates/slicer-schema/wit/deps/world-finalization/world-finalization.wit`.

**Addressed, and narrower than it looks.** ADR-0008's clause is a statement about
what *that packet* cost, in support of its "no new stage, no new WIT export"
decision — it is a consequence, not a prohibition, and its stated concern is
rebuild churn rather than the WIT surface as such. This ADR does not overturn
ADR-0008's decision: overhang speed application still happens in a
`FinalizationModule`, still through `FinalizationOutputBuilder`, still with no
new stage and no new WIT *export*. What changes is one additive case on an
existing `variant`, which is a `world-finalization` wire-format change only.

The churn half of the clause does now apply, and is the honest cost: bindgen
re-reads `crates/slicer-schema/wit/**/*.wit`, so **every** guest artifact is
invalidated even though only finalization guests are semantically affected. The
`cargo xtask build-guests --check` gate is mandatory before interpreting any
component test on a tree carrying this change. ADR-0008's Consequences bullet
should be read as historical rather than current from this ADR forward.

[ADR-0053](./0053-overhang-emission-time-speed-sections.md) separately retires
ADR-0008's `set-speed-factor` decision for the overhang module; that is its
amendment to make, not this one's.

## Consequences

- **The finalization→emit speed contract is now two-valued, and the emitter must
  agree on precedence.** `SetPointSpeedFactors` *replaces* the whole-entity
  mutation for a given entity rather than supplementing it; a consumer must never
  receive both and have to guess which wins. The producing module is responsible
  for emitting one or the other.
- **The `kept`-remap change touches emission for every entity in every slice**,
  including entities with no profile, because each surviving point after
  simplification must now carry its **original** index in order to index the
  profile. The fallback path must resolve to the identical
  `resolve_feedrate(role, entity.path.speed_factor)` call. This cannot be
  verified by whole-output G-code byte comparison — `DEV-093` records that the
  pipeline is not byte-deterministic run-to-run — so the guard is the feedrate
  and golden test suites, including a **mixed** layer holding both profiled and
  un-profiled entities. An all-absent-profiles test cannot see a lookup miss.
- **The schema bump is additive.** `#[serde(default)]` keeps prior payloads
  deserializable, matching the `tool_index` precedent.
- **A guest can never read back a speed profile — its own or a sibling's.**
  `layer-collection-view` in
  `crates/slicer-schema/wit/deps/world-finalization/world-finalization.wit`
  exposes exactly six methods — `layer-index`, `z`, `entity-count`,
  `ordered-entities`, `tool-changes`, `z-hops` — and **no `travel-moves`
  accessor**, hence no accessor of that family for `speed_profiles` either. The
  channel is write-only from the guest's side: `apply_to` runs host-side after
  finalization dispatch, in `merge_ops` submission order, and nothing hands the
  result back. Two finalization modules therefore cannot compose per-point
  speeds, and a module cannot inspect what it wrote. This is an accepted
  limitation of the current world, not an oversight; lifting it is a
  `world-finalization` WIT change and a rebuild of every guest.
- **Stale rows are possible in principle and inert in practice.** No finalization
  primitive removes an entity today (`push_entity_to_layer`, `insert_entity_at`,
  `set_entity_order`, `sort_layer_by` only add or reorder), so a row for a
  vanished `entity_id` is unreachable; the emitter's lookup would simply miss.
  Pruning may be left unimplemented. If it is implemented, it must not change
  emission for any reachable input.

## Alternatives considered

- **A per-point `speed_factor` field on `Point3WithWidth`**, mirroring
  `dist_to_top_mm`. Rejected on blast radius: it forces an exhaustive
  struct-literal sweep across the tree plus a `point3-with-width` WIT record
  field, for a value that is absent on the overwhelming majority of entities and
  is not geometry-intrinsic. See the ADR-0048 reconciliation above for when the
  opposite answer is correct.
- **Absolute mm/s in `factors`.** Rejected: it moves the `clamp(0.05, 5.0)` and
  per-role base-speed selection out of `resolve_feedrate` and into every
  producing guest, giving two places where speed policy lives and no single
  place to change it.
- **A positional (index-keyed) side table**, i.e. a `Vec<Vec<f32>>` parallel to
  the entity list. Rejected: packet 39 already converted `TravelMove` off a
  positional anchor for exactly this reason — finalization-stage entity
  insertion invalidates positional indices, and a dangling speed anchor would
  mis-speed a different entity rather than fail.
- **Keeping `SetSpeedFactor` and adding a second mutation alongside it.**
  Rejected: two mutation kinds reaching the emitter for one entity requires a
  precedence rule that no consumer can be trusted to apply consistently.

## Cross-references

- ADR-0032 (curled-edge slowdown, computed transiently) — applied, not
  overturned; its boundary test is the distinguishing principle above.
- ADR-0048 (`dist_to_top_mm` per-point) — divergent shape for a different kind
  of value; not superseded.
- ADR-0008 (overhang as a `FinalizationModule`) — its "No WIT contract change"
  consequence is addressed above and should be read as historical.
- ADR-0053 (overhang emission-time speed sections) — the consumer of this
  contract, and the ADR that retires ADR-0008's `set-speed-factor` decision.
- `docs/spec_packets/_OLD/189-per-point-speed-factor-carrier.md` — the implementing packet.
- `crates/slicer-gcode/src/emit.rs` — `DefaultGCodeEmitter::resolve_feedrate`,
  `travel_moves_by_entity`, and the simplification `kept` remap.
- `crates/slicer-ir/src/slice_ir.rs` — `Point3WithWidth`, `LayerCollectionIR`,
  `TravelMove`, `TravelRetract`.

## Amendment — 2026-08-05 (packet 189)

### Retired clause (verbatim, Consequences)

> - **Stale rows are possible in principle and inert in practice.** No finalization
>   primitive removes an entity today (`push_entity_to_layer`, `insert_entity_at`,
>   `set_entity_order`, `sort_layer_by` only add or reorder), so a row for a
>   vanished `entity_id` is unreachable; the emitter's lookup would simply miss.
>   Pruning may be left unimplemented. If it is implemented, it must not change
>   emission for any reachable input.

That clause is retired in the quoted form and replaced with the exhaustive
`MergeOp` basis below.

### Replacement

- **Stale rows are possible in principle and inert in practice.** The basis for
  inertness is the exhaustive `MergeOp` set in `crates/slicer-sdk/src/traits.rs`:
  `pub enum MergeOp` has exactly five variants — `ModifyEntity`, `SortLayer`,
  `InsertSynthLayer`, `InsertEntityAt`, `SetEntityOrder` — and none removes an
  entity; they modify, reorder, insert, or permute. Orphaned rows are therefore
  unreachable under the current set, and the emitter's `entity_id`-keyed lookup
  would simply miss if one arose, so it is inert. Pruning may be left
  unimplemented. If it is implemented, it must not change emission for any
  reachable input. Verify against the enum, not against a list of builder
  methods: the retired clause's four-name enumeration was incomplete
  (`push_entity_with_priority`, `insert_synthetic_layer`,
  `insert_synthetic_layer_after`, `push_annotation` and `push_fan_speed` also
  exist on `FinalizationOutputBuilder`) and would have rotted again with the
  next added method; the enum is exhaustive by construction.
