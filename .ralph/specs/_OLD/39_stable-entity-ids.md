---
status: implemented
packet: stable-entity-ids
task_ids:
  - TASK-170
---

# 39_stable-entity-ids

## Goal

Decouple `LayerCollectionIR.travel_moves` anchors from positional indices into `ordered_entities`. Add a per-layer-monotonic `entity_id: u64` to every entity in `ordered_entities`; change `TravelMove.entity_idx: u32 → TravelMove.entity_id: u64`; producers issue IDs at construction time; `gcode_emit.rs` resolves travel anchors via a per-layer `entity_id → index` map built once at emit time. **Pure refactor — zero G-code behavioral change.** Foundation packet for future PostPass mutators (`SequentialPrintOrder`, `MinLayerTimeEnforcer`, `FlushVolumeCalculator`, `PrimeTower`, finalization-priority sort) which all need stable identity to mutate, reorder, or insert entities without rewriting positional anchors.

## Problem Statement

`LayerCollectionIR.travel_moves` currently anchors each travel by an integer index into `ordered_entities` (working hypothesis: `TravelMove.entity_idx: u32`). This makes the index a fragile foreign-key reference: any operation that reorders, inserts into, or removes from `ordered_entities` must hand-rewrite every dependent travel anchor, otherwise G-code emits travels at wrong positions.

This bit Packet `38-rev1_top-surface-ironing` indirectly: the host's finalization-merge site (`crates/slicer-host/src/dispatch.rs:2877`) uses `splice(0..0, fin_entities)` — literal prepend — partly because any other insertion site would have to walk the layer's travel anchors and increment them. The result: top-surface-ironing entities currently print BEFORE top-fill (semantically wrong) because the host has no cheap way to position them after fill.

The architecture document `docs/01_system_architecture.md:328-363` declares `PostPass::LayerFinalization` semantics as *"Vec<LayerCollectionIR> (all layers, mutable — may append entities or insert synthetic layers)"*, but the current implementation enforces a strictly weaker push-only contract because the data structure cannot tolerate mutation cheaply. Several upcoming modules listed in that same doc — `SequentialPrintOrder`, `MinLayerTimeEnforcer`, `FlushVolumeCalculator`, `PrimeTower` — need richer mutation primitives (reorder, in-place edit, synthetic layer insert). All of those primitives are easy with stable IDs and combinatorially expensive without.

This packet introduces `entity_id: u64` as a per-layer-monotonic stable identifier on every entity in `ordered_entities`, and migrates `TravelMove` to anchor by `entity_id` instead of position. Producers issue IDs at construction. `gcode_emit` builds a one-shot `entity_id → index` map per layer. **No G-code byte-level behavioral change** — it is a pure structural refactor whose payoff is unblocking the next packet (`40_finalization-mutation-builder`) and every PostPass mutator downstream.

## Architecture Constraints

- **Stable identity is per-layer**, not global. `LayerEntityIdGen` resets at each new `LayerCollectionIR`. Cross-layer references are not supported (and not needed — travels are intra-layer in this slicer).
- **ID width**: `u64`. Even 0.001 mm extrusion segments at 1000 layers won't approach `u64::MAX`. Reserved value `0` MAY be used as "uninitialized" / sentinel; `LayerEntityIdGen::next()` returns IDs starting at `1`.
- **ID uniqueness invariant**: producers must not reuse the same generator across layers. The host owns one per `LayerCollectionIR` it constructs.
- **Concurrency model**: per `docs/04_host_scheduler.md`, layer construction is single-threaded per layer (rayon parallelism is across layers, not within). The generator may be `!Send + !Sync` (a plain `Cell<u64>`), tested via `static_assertions::assert_not_impl_any!`. If Step 0 finds a producer site already shares a layer across threads, the generator switches to `AtomicU64` (no API change for callers).
- **Serialization**: `entity_id` and `TravelMove.entity_id` MUST round-trip through bincode/serde. The schema version bump in `docs/02_ir_schemas.md` records the breaking change for any external IR consumers (none today; future-proofing only).
- **Regression contract**: AC-2 asserts the existing `benchy_end_to_end_tdd` suite continues to PASS unchanged. This is the load-bearing constraint — if any G-code byte changes, the implementation has done extra work outside scope.
- **No FinalizationOutputBuilder API change**: `push_entity_to_layer(layer, path, region)` keeps its current signature in this packet. Internally the host stamps an ID at merge time. Module call sites do not change.

## Data and Contract Notes

- IR or manifest contracts touched:
  - `LayerCollectionIR.ordered_entities[*].entity_id` — new field, additive.
  - `LayerCollectionIR.travel_moves[*].entity_id` — replaces `entity_idx`. **Breaking** for any external consumer; none exist today.
  - Schema-version bump records the breaking change.
- WIT boundary considerations: WIT does not currently encode `TravelMove` or `entity_idx` per session memory. Step 0 confirms by grep across `wit/` and `crates/slicer-host/src/wit_host.rs`. If a WIT type does carry the field, this packet's scope expands to include the WIT update; if so, escalate before proceeding.
- Determinism / scheduler constraints:
  - ID issuance is deterministic given producer call order.
  - `gcode_emit` lookup is `O(N)` to build the map and `O(1)` per lookup; matches current `O(1)` index access at parse time. No measurable runtime cost.

## Locked Assumptions and Invariants

- The entity struct in `LayerCollectionIR.ordered_entities` exists (Step 0 confirms exact name; working hypothesis from session memory: `PrintEntity` near `crates/slicer-ir/src/slice_ir.rs:1228`).
- `TravelMove` exists and currently anchors by an index field (Step 0 confirms exact name; working hypothesis `entity_idx: u32`).
- `gcode_emit.rs` does not currently sort or reorder `ordered_entities` post-construction (confirmed by Packet 38-rev1 Step 0 at line 182).
- Layer construction is single-threaded per layer (per `docs/04_host_scheduler.md`); the generator can be `!Send + !Sync` unless Step 0 reveals otherwise.
- No external persisted IR consumers exist (no on-disk IR snapshots that pre-date this schema bump).
- The benchy STL pipeline currently produces deterministic G-code byte output for the existing assertions in `benchy_end_to_end_tdd`.

## Risks and Tradeoffs

- **Test-fixture sweep volume**. Estimated 10–30 sites referencing `entity_idx`. If Step 0 LOCATIONS returns > 30, the sweep step is split into multiple worker dispatches (≤ 10 fixtures each) to keep individual dispatches at S cost. Hard cap: if Step 0 returns > 50, escalate before authorizing Step 6.
- **WIT boundary leakage**. If `TravelMove` or its anchor field is currently exposed across a WIT boundary, the packet's scope crosses crate boundaries (`wit_host.rs`, `wit-guest` re-exports). Step 0 explicitly searches `wit/` and `wit_host.rs`; if positive, escalate to user (the packet may need a WIT-update sub-step or split into a separate packet).
- **Hidden positional assumptions in producers**. If `layer_executor.rs` builds travels by capturing an index BEFORE the entity push (i.e., `let idx = layer.ordered_entities.len()`), the migration is straightforward (capture the just-issued `entity_id` instead). If any site captures by post-push index arithmetic in a way that is not 1:1 with the most recently pushed entity, the migration is more involved. Step 0 inspects each producer site's pattern (≤ 5 lines per site).
- **Regression risk on Benchy**. AC-2 is the canary. If it fails, the implementation has changed G-code byte output — investigate before declaring DONE.
- **Schema-version bump**. If `docs/02_ir_schemas.md` policy requires registering the bump in `docs/14_deviation_audit_history.md`, that adds a small docs edit (Step 7 absorbs it).
