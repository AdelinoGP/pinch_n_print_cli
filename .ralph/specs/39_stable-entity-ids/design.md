# Design: stable-entity-ids

## Controlling Code Paths

- Primary code path:
  - `crates/slicer-ir/src/slice_ir.rs` — entity struct (~line 1228 per session memory; Step 0 confirms exact name and line) and `TravelMove` struct. Add `entity_id: u64` to entity struct; replace `TravelMove.entity_idx: u32` (working hypothesis name) with `entity_id: u64`. Bump schema version per `docs/02_ir_schemas.md` versioning rule.
  - `crates/slicer-ir/src/` (new module or appended to existing) — `LayerEntityIdGen` helper plus the dangling-anchor validator function.
  - `crates/slicer-host/src/layer_executor.rs:605-617, 619-636, 638-659` — three producer sites for PerimeterIR / InfillIR / SupportIR. Each must accept (or construct) a `LayerEntityIdGen` and stamp every entity with a fresh ID before pushing into `ordered_entities`. When a producer also pushes a `TravelMove`, the travel anchors by the just-issued ID.
  - `crates/slicer-host/src/dispatch.rs:2861-2877` — finalization-stage merge site. Currently `splice(0..0, fin_entities)`. After this packet: each finalization entity gets an ID stamped from the layer's `LayerEntityIdGen` at merge time. The `splice(0..0, ...)` behavior is preserved bit-for-bit (Packet 40 changes the splice strategy; Packet 39 does not).
  - `crates/slicer-host/src/gcode_emit.rs:182, 285-295` — at the top of the per-layer emit loop, build `let id_to_idx: HashMap<u64, usize> = layer.ordered_entities.iter().enumerate().map(|(i, e)| (e.entity_id, i)).collect();`. Replace travel-resolution `let entity = &layer.ordered_entities[travel.entity_idx as usize];` with `let entity = &layer.ordered_entities[id_to_idx[&travel.entity_id]];`. Behavior identical when anchors are correct; debug builds may add `debug_assert!(id_to_idx.contains_key(&travel.entity_id))`.
- Reference template:
  - None — this is foundational. The `LayerEntityIdGen` is the new pattern other producers will adopt.
- Neighboring tests / fixtures:
  - `crates/slicer-host/tests/benchy_end_to_end_tdd.rs::benchy_gcode_contains_ironing_evidence` — must continue to PASS unchanged. It is the strongest regression test in the workspace for this refactor.
  - Every test fixture across `crates/slicer-host/tests/`, `crates/slicer-ir/tests/`, `modules/core-modules/*/tests/` that constructs a `TravelMove` or accesses `.entity_idx`. Step 0 LOCATIONS dispatch returns the exhaustive list. Estimated count: 10–30 fixtures (most one-line edits).
  - `crates/slicer-host/tests/manifest_ingestion_tdd.rs` and `claim_transition_matrix_tdd.rs` — should not need changes (they don't touch entity-level IR), but Step 0 confirms.
- OrcaSlicer comparison surface:
  - None.

## Architecture Constraints

- **Stable identity is per-layer**, not global. `LayerEntityIdGen` resets at each new `LayerCollectionIR`. Cross-layer references are not supported (and not needed — travels are intra-layer in this slicer).
- **ID width**: `u64`. Even 0.001 mm extrusion segments at 1000 layers won't approach `u64::MAX`. Reserved value `0` MAY be used as "uninitialized" / sentinel; `LayerEntityIdGen::next()` returns IDs starting at `1`.
- **ID uniqueness invariant**: producers must not reuse the same generator across layers. The host owns one per `LayerCollectionIR` it constructs.
- **Concurrency model**: per `docs/04_host_scheduler.md`, layer construction is single-threaded per layer (rayon parallelism is across layers, not within). The generator may be `!Send + !Sync` (a plain `Cell<u64>`), tested via `static_assertions::assert_not_impl_any!`. If Step 0 finds a producer site already shares a layer across threads, the generator switches to `AtomicU64` (no API change for callers).
- **Serialization**: `entity_id` and `TravelMove.entity_id` MUST round-trip through bincode/serde. The schema version bump in `docs/02_ir_schemas.md` records the breaking change for any external IR consumers (none today; future-proofing only).
- **Regression contract**: AC-2 asserts the existing `benchy_end_to_end_tdd` suite continues to PASS unchanged. This is the load-bearing constraint — if any G-code byte changes, the implementation has done extra work outside scope.
- **No FinalizationOutputBuilder API change**: `push_entity_to_layer(layer, path, region)` keeps its current signature in this packet. Internally the host stamps an ID at merge time. Module call sites do not change.

## Code Change Surface

- Selected approach:
  - **Single-counter per-layer ID issuance + ID→index map at emit time.** Simplest workable design; matches the slicer's existing per-layer execution model.
  - Schema migration is one field added per struct (entity gets `entity_id`; `TravelMove` swaps anchor name and type). No alternate-tag dance.
  - Producer migration is mechanical: each entity construction gains one line `let entity_id = id_gen.next();` plus `entity_id` in the struct literal. Each `TravelMove` construction references that same `entity_id` (the producer captured it locally).
  - Validation helper is a free function `pub fn validate_travel_anchors(layer: &LayerCollectionIR) -> Result<(), ValidateError>` returning the diagnostic on the first dangling anchor (Err short-circuits per typical `?` style).
- Rejected alternatives:
  - **Separate "anchor" type wrapping `(entity_id, weak_idx_hint)`** — over-engineered; single field suffices.
  - **Use the entity's content hash as ID** — non-deterministic ordering, not reusable for cross-tool diffing, slower.
  - **Use `Rc<...>`/`Arc<...>` pointer identity** — incompatible with serde and bincode; loses serialization round-trip.
  - **Defer ID issuance to merge time only (no producer participation)** — would require travels constructed at producer time to anchor by something else, breaking the current TravelMove construction sites in `layer_executor.rs`. Producers must participate.
  - **Atomic global counter** — wastes ID space; defeats per-layer reset; forces `Sync` requirement that's not needed.
- Exact functions, traits, manifests, tests expected to change:
  - `crates/slicer-ir/src/slice_ir.rs`:
    - Entity struct (per Step 0) — add `pub entity_id: u64`.
    - `TravelMove` — replace `entity_idx: u32` (or current name) with `entity_id: u64`.
    - Schema-version bump (one constant edit).
  - `crates/slicer-ir/src/entity_id.rs` (new file) — `LayerEntityIdGen` struct + `next()` impl + tests-side `Default::default()`.
  - `crates/slicer-ir/src/validation.rs` (new or appended) — `validate_travel_anchors`.
  - `crates/slicer-ir/src/lib.rs` — `pub mod` re-exports for `LayerEntityIdGen` and `validate_travel_anchors`.
  - `crates/slicer-host/src/layer_executor.rs:605-659` — three producer sites; threads a `&mut LayerEntityIdGen` parameter (or pulls one from the layer struct that already exists in scope; Step 0 confirms which is cleaner).
  - `crates/slicer-host/src/dispatch.rs:2861-2877` — finalization merge: stamp IDs on incoming entities before splice.
  - `crates/slicer-host/src/gcode_emit.rs:182, 285-295` — build the lookup map; resolve travels by ID.
  - Test fixtures across the workspace per Step 0 LOCATIONS (≤ 30 sites).
- Test files added by this packet:
  - `crates/slicer-ir/tests/entity_id_invariants_tdd.rs`
  - `crates/slicer-ir/tests/ir_validation_tdd.rs`
  - `crates/slicer-host/tests/gcode_emit_travel_anchor_tdd.rs`

## Files in Scope (read + edit)

Primary edit targets per step (≤ 3 per step):

- Step 1 ("Failing TDD"): `crates/slicer-ir/tests/entity_id_invariants_tdd.rs` + `crates/slicer-ir/tests/ir_validation_tdd.rs` + `crates/slicer-host/tests/gcode_emit_travel_anchor_tdd.rs` (3 files).
- Step 2 ("IR schema"): `crates/slicer-ir/src/slice_ir.rs` + `crates/slicer-ir/src/entity_id.rs` (new) + `crates/slicer-ir/src/lib.rs` (3 files).
- Step 3 ("Validation helper"): `crates/slicer-ir/src/validation.rs` (1 file; possibly + `lib.rs` re-export — Step 2 already touches `lib.rs`).
- Step 4 ("Producer migration"): `crates/slicer-host/src/layer_executor.rs` + `crates/slicer-host/src/dispatch.rs` (2 files).
- Step 5 ("Emit migration"): `crates/slicer-host/src/gcode_emit.rs` (1 file).
- Step 6 ("Test fixture sweep"): up to 3 fixture files per worker dispatch; if more remain, additional dispatch rounds.
- Step 7 ("Acceptance + docs"): `docs/07_implementation_status.md` (1 file; delegated insertion).

## Read-Only Context

- `docs/02_ir_schemas.md` — `LayerCollectionIR`, `TravelMove`, schema versioning rules. Direct read.
- `docs/01_system_architecture.md` lines 328–363 — direct read.
- `docs/04_host_scheduler.md` lines 309–317 — direct read.
- `crates/slicer-ir/src/slice_ir.rs` — narrow ranges only (entity struct + `TravelMove` ± 40 lines).
- `crates/slicer-host/src/dispatch.rs:2861-2877` — narrow range only.
- `crates/slicer-host/src/gcode_emit.rs:170-200, 280-300` — narrow ranges only.
- `crates/slicer-host/src/layer_executor.rs:600-665` — narrow range only.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/` — never load.
- `target/`, `Cargo.lock`, generated code — never load.
- `crates/slicer-host/src/dispatch.rs` outside the narrow merge-site range.
- `crates/slicer-host/src/gcode_emit.rs` outside the narrow ranges above.
- `crates/slicer-host/src/wit_host.rs`, `manifest.rs`, `validation.rs` (host) — out of scope; this packet does not touch them.
- `crates/slicer-sdk/` — out of scope; FinalizationOutputBuilder API is stable in this packet.
- All core modules (`modules/core-modules/*/`) — out of scope; no module call-site changes.
- All other crates (`slicer-helpers`, `slicer-core`, `slicer-schema`) — out of scope unless Step 0 reveals a producer site there.

## Expected Sub-Agent Dispatches

Per Step 0:

- "FACT: in `crates/slicer-ir/src/slice_ir.rs`, what is the exact struct name + line of the entity stored in `LayerCollectionIR.ordered_entities`? Quote the struct definition (≤ 10 lines). Also locate `TravelMove`: quote its definition and the exact name + type of its `ordered_entities` anchor field (working hypothesis `entity_idx: u32`)."
- "LOCATIONS: every workspace site referencing `entity_idx` (case-sensitive). Use `rg -n 'entity_idx' --type rust`. Return file:line + a 1-line snippet for each. ≤ 30 entries; if more, paginate by crate."
- "FACT: does `LayerCollectionIR` already carry a generator-like field, or do producers receive a per-layer context at construction? Quote the function signature(s) at `layer_executor.rs:605, 619, 638` (≤ 5 lines each) so the producer migration can decide whether to thread `&mut LayerEntityIdGen` or pull from a context struct already in scope."
- "FACT: in `docs/02_ir_schemas.md`, where is the IR schema version constant defined (file:line) and what is the documented bump rule for an additive field? Quote ≤ 3 lines."
- "FACT: in `crates/slicer-host/src/dispatch.rs:2861-2877`, quote the finalization-merge code block exactly (≤ 20 lines). Confirm the `splice(0..0, ...)` is still at the same site after Packet 38-rev1 closure."

Per Step 1: cargo-build/test FACT after authoring (compile-fail expected).

Per Steps 2–5: cargo-build FACT after each change; targeted-test FACT for the directly-affected suite.

Per Step 6: rg-style sweep FACT for any remaining `entity_idx` references; ≤ 3 fixture files migrated per worker dispatch.

Per Step 7: cargo-test FACT for the workspace gate; one delegated insertion of the `TASK-170` row in `docs/07`.

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

## Context Cost Estimate

- Aggregate: `M`.
- Largest single step: `M` (Step 4: producer migration touching three sites + dispatch.rs).
- Highest-risk dispatch: Step 0 LOCATIONS sweep (gates Step 6 sizing directly).

## Open Questions (resolved or punted to Step 0)

- 🔍 Exact name + line of the entity struct in `LayerCollectionIR.ordered_entities` — Step 0 FACT.
- 🔍 Exact name + type of the `TravelMove` anchor field — Step 0 FACT.
- 🔍 Workspace footprint of `entity_idx` references — Step 0 LOCATIONS.
- 🔍 Whether `TravelMove` or its anchor field crosses any WIT boundary — Step 0 FACT.
- 🔍 Schema-version bump location and rule per `docs/02_ir_schemas.md` — Step 0 FACT.
- 🔍 Whether `layer_executor.rs` producer sites already have a context struct that could carry `LayerEntityIdGen` — Step 0 FACT.

The six 🔍 questions are pre-implementation discovery, not packet activation blockers — they are answerable by sub-agents reading code that already exists. The packet remains `draft` so the user can read it before a fresh implementer agent runs Step 0.
