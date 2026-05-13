# Design: finalization-mutation-builder

## Controlling Code Paths

- Primary code path:
  - `crates/slicer-ir/src/slice_ir.rs` (or wherever `ExtrusionRole` is canonically defined post-Packet-39 — Step 0 confirms exact path; session memory shows the enum near line 1228) — add `pub const fn default_priority(&self) -> u32` impl block on `ExtrusionRole`.
  - `crates/slicer-sdk/src/builders.rs` (or located path; Step 0 LOCATIONS dispatch returns the exact file housing `FinalizationOutputBuilder`) — add four new methods plus internal storage for the recorded operations.
  - `crates/slicer-host/src/dispatch.rs:2877` (or the post-Packet-39 location of the finalization-merge site — Step 0 confirms; line numbers may have shifted by ≤ 20 lines after Packet 39 changed `splice` parameters) — replace the prepend with the new merge sequence.
  - `modules/core-modules/top-surface-ironing/src/lib.rs` — single-line change at the existing `output.push_entity_to_layer(...)` site; uses `ExtrusionRole::Ironing.default_priority()` as the explicit priority arg.
- Reference template:
  - `modules/core-modules/skirt-brim/src/lib.rs` — must continue to compile and run unchanged. It is the canary for the legacy `push_entity_to_layer` alias.
  - The post-Packet-39 `LayerEntityIdGen` helper in `slicer-ir` — referenced (not edited) by the new merge code at the dispatch site to stamp IDs on finalization-pushed entities.
- Neighboring tests / fixtures:
  - `crates/slicer-host/tests/benchy_end_to_end_tdd.rs::benchy_gcode_contains_ironing_evidence` — must continue to PASS. New test `benchy_top_surface_precedes_ironing` is added in the same file.
  - `modules/core-modules/top-surface-ironing/tests/top_surface_ironing_emission_tdd.rs` — must continue to PASS (8/8) post-migration. The 8 module-level tests assert on `output.entity_pushes()` / push tuples, which are unaffected by the `push_entity_with_priority` API change because the test harness's `FinalizationOutputBuilder` records pushes regardless of method variant. Step 0 FACT confirms.
  - `crates/slicer-host/tests/manifest_ingestion_tdd.rs` and `claim_transition_matrix_tdd.rs` — should not need changes; this packet does not add or remove modules.
- OrcaSlicer comparison surface:
  - None required. If parity is challenged, delegate one SUMMARY ≤ 200 words on OrcaSlicer's per-layer extrusion ordering site.

## Architecture Constraints

- **Trait signature stability for FinalizationModule**: the existing `run_finalization(&self, layers: &[LayerCollectionView], output: &mut FinalizationOutputBuilder, _config: &ConfigView)` shape stays unchanged. Modules opt into new methods by calling them; legacy modules continue to work via the `push_entity_to_layer` alias.
- **Builder operation recording, not direct mutation**: `modify_entity`, `sort_layer_by`, and `insert_synthetic_layer_after` RECORD operations on the builder. The host applies them deterministically AFTER the module returns. This preserves the existing dispatch-site invariant that modules do not directly mutate the IR; mutations are reified as data and applied by the host.
- **Stable identity**: relies on Packet 39's `entity_id`. `modify_entity(layer, entity_id, ...)` matches by ID; `sort_layer_by` and `insert_synthetic_layer_after` do not depend on identity but rely on the existing `entity_id` invariant for travel anchors to survive reorder.
- **Deterministic merge order**: when the host applies recorded operations, the order is:
  1. Extend `ordered_entities` with new pushes (new pushes carry `Option<u32>` explicit priority; `None` defaults to role priority).
  2. Stamp `entity_id` on every newly-pushed entity from the layer's `LayerEntityIdGen`.
  3. Stable-sort the entire `ordered_entities` by `(effective_priority(entry), original_index_within_layer_at_post-extend)`. The "original index" tiebreaker keeps producer-emit order stable for ties.
  4. Apply each `modify_entity` op by `entity_id` lookup; surface diagnostic on dangling ID.
  5. Apply each `sort_layer_by` closure (rare but documented; runs after `modify_entity`).
  6. After all per-layer merges, apply `insert_synthetic_layer_after` ops to the outer `Vec<LayerCollectionIR>`. Multiple inserts at the same `idx` apply in module-call order (stable insertion).
- **Backwards compatibility**: `push_entity_to_layer(layer, path, region)` is `#[inline]` to `push_entity_with_priority(layer, path, region, 0)`. Skirt-brim's behavior is preserved bit-for-bit (priority 0 places its entities first; ties preserve producer order).
- **Producer (non-finalization) entities are NOT re-sorted**: only the post-extend layer state is sorted. Wait — that's incorrect: if the producer-emitted entries should keep their producer order, they MUST have priorities matching their emit order. The role-priority table is designed so producer-emit order matches priority order:
  - Producers emit in order: walls (OuterWall < InnerWall) → fill (Sparse → Solid → Top → Bottom) → support → ironing-not-yet-emitted-by-producer.
  - Role priority table mirrors this: `OuterWall=1000 < InnerWall=1500 < ThinWall=1700 < SparseInfill=3000 < BridgeInfill=3500 < BottomSolidInfill=4000 < TopSolidInfill=4500 < SupportMaterial=5000 < SupportInterface=5500 < Ironing=6000 < WipeTower=8000 < PrimeTower=8500 < Skirt=0` (skirt is special; lowest, which preserves prepend).
  - Therefore stable-sorting the entire vec by priority preserves producer order for non-finalization entries, AND lands finalization entries at their correct slot. **This is the load-bearing invariant.** Step 1 explicitly verifies it via a unit test that constructs all roles in producer order, sorts by `default_priority`, and asserts the result equals the producer-order vec.
- **Tie-stable**: stable-sort is mandatory. Two entities with the same priority preserve their post-extend insertion order. Critical for two finalization modules pushing at the same priority and for producer pairs (e.g., two `OuterWall` entities in the same layer).
- **Travel anchor survival**: travels anchor by `entity_id` (Packet 39 invariant). Reorder via stable-sort preserves all `entity_id` values; lookup at emit time still resolves correctly. AC-3 explicitly verifies.

## Code Change Surface

- Selected approach:
  - **Operation-recording builder + post-merge applier**. Modules call `modify_entity` / `sort_layer_by` / `insert_synthetic_layer_after`; the builder stores them as `Vec<MergeOp>`. Host applies ops in deterministic order after the module returns.
  - **Numeric priority u32 with role-default fallback**. Existing pushes default to `role.default_priority()`; modules can override per-push.
  - **Single host-merge sequence at `dispatch.rs:2877`** (post-Packet-39 location). All four new ops are applied here.
  - **One module migration only** (`top-surface-ironing` → one-line change). Skirt-brim and any other existing finalization modules stay on the legacy alias.
- Rejected alternatives:
  - **Role-anchor API instead of numeric priority** (e.g., `push_entity_after_role(TopSolidInfill)`). Rejected because the user explicitly wants flexibility for "anywhere in the printing priority" — a numeric priority allows insertion BETWEEN roles (e.g., priority 4250 lands between BottomSolidInfill=4000 and TopSolidInfill=4500). Role-anchor is constrained to declared variants.
  - **Direct mutation of `&mut Vec<LayerCollectionIR>` passed to modules**. Closer to `docs/01:332`'s literal contract but loses the deterministic-ordering and dependency-graph guarantees that the dispatch-site invariant provides. The recorded-ops approach is a strict improvement over direct mutation: still mutable in effect; deterministic in application.
  - **Manifest-driven priority**. Adds config surface; modules with the same role would need per-instance priority resolution. Out of scope; numeric priority arg covers all use cases this packet anticipates.
  - **Sort only finalization-pushed entries (skip producer entries)**. Requires marking which entries came from finalization vs. producers — adds bookkeeping. Rejected because the role-priority table is designed so producer-emit order ALREADY matches priority order; full-vec sort is safe and simpler.
  - **Schedule the priority sort at G-code emit time instead of at dispatch merge time**. Rejected because emit-time sort would require remapping `travel_moves` anchors a second time (already correct post-Packet-39 by ID, but the lookup map would have to be rebuilt after sort — minor performance hit but more importantly mixes responsibilities). Merge-time sort is cleaner.
- Exact functions, traits, manifests, tests expected to change:
  - `crates/slicer-ir/src/slice_ir.rs` — add `impl ExtrusionRole { pub const fn default_priority(&self) -> u32 { ... } }` block.
  - `crates/slicer-sdk/src/builders.rs` (path per Step 0) — add four methods + internal `Vec<MergeOp>` storage; add `enum MergeOp { ModifyEntity { layer, entity_id, op_id }, SortLayer { layer, key_fn_id }, InsertSynthLayer { idx, new_layer } }`. Closures are stored either as `Box<dyn FnOnce(...)>` or via an op-id registry; Step 1 chooses based on object-safety and serde constraints.
  - `crates/slicer-sdk/src/builders.rs` — `push_entity_to_layer` becomes `#[inline] fn ... { self.push_entity_with_priority(layer, path, region, 0) }`.
  - `crates/slicer-host/src/dispatch.rs:2877` (or post-Packet-39 location) — replace single `splice(0..0, ...)` with the merge-sequence helper. Helper lives in `dispatch.rs` or a new `merge.rs` module if size warrants (Step 4 decides).
  - `modules/core-modules/top-surface-ironing/src/lib.rs` — single-line edit at the existing push site.
  - `crates/slicer-host/tests/benchy_end_to_end_tdd.rs` — add `benchy_top_surface_precedes_ironing`.
- Test files added by this packet:
  - `crates/slicer-ir/tests/extrusion_role_priority_tdd.rs`
  - `crates/slicer-sdk/tests/finalization_builder_tdd.rs`

## Files in Scope (read + edit)

Primary edit targets per step (≤ 3 per step):

- Step 1 ("Failing TDD"): `crates/slicer-ir/tests/extrusion_role_priority_tdd.rs` + `crates/slicer-sdk/tests/finalization_builder_tdd.rs` + new test in `benchy_end_to_end_tdd.rs` (3 files).
- Step 2 ("Role priority table"): `crates/slicer-ir/src/slice_ir.rs` (1 file).
- Step 3 ("Builder API"): `crates/slicer-sdk/src/builders.rs` (1 file; ≤ 2 if helper modules needed).
- Step 4 ("Host merge replacement"): `crates/slicer-host/src/dispatch.rs` (1 file).
- Step 5 ("Top-surface-ironing migration"): `modules/core-modules/top-surface-ironing/src/lib.rs` (1 file).
- Step 6 ("Acceptance + docs"): `docs/07_implementation_status.md` (1 file; delegated insertion).

## Read-Only Context

- `docs/01_system_architecture.md` lines 328–363 — mutability contract.
- `docs/04_host_scheduler.md` lines 309–317, 680–717.
- `docs/05_module_sdk.md` — relevant `FinalizationOutputBuilder` section only (delegate SUMMARY).
- `docs/02_ir_schemas.md` — `ExtrusionRole` enum + entity struct.
- `crates/slicer-sdk/src/builders.rs` — full read only if < 300 lines; otherwise SUMMARY.
- `crates/slicer-host/src/dispatch.rs` — narrow range only (≤ 30 lines around the post-Packet-39 finalization-merge site).
- `modules/core-modules/skirt-brim/src/lib.rs` — full read (small; precedent for legacy alias).
- `modules/core-modules/top-surface-ironing/src/lib.rs` — full read (small; the one migration target).
- `crates/slicer-host/tests/benchy_end_to_end_tdd.rs::benchy_gcode_contains_ironing_evidence` — narrow range (existing test that must keep passing).

## Out-of-Bounds Files

- `OrcaSlicerDocumented/` — never load (delegate only if parity needed).
- `target/`, `Cargo.lock`, generated code — never load.
- `crates/slicer-host/src/dispatch.rs` outside the narrow merge-site range.
- `crates/slicer-host/src/gcode_emit.rs`, `wit_host.rs`, `manifest.rs` — out of scope; this packet does not touch them.
- `crates/slicer-ir/src/slice_ir.rs` outside the `ExtrusionRole` enum and its impl block.
- All core modules beyond `skirt-brim` (read-only canary) and `top-surface-ironing` (one-line migration).
- All other crates (`slicer-helpers`, `slicer-core`, `slicer-schema`).
- Any `wit/` files — this packet does not touch the WIT boundary (Step 0 confirms `FinalizationOutputBuilder` is host-side / SDK-side only; if Step 0 finds a WIT mirror, packet escalates to user before proceeding).

## Expected Sub-Agent Dispatches

Per Step 0:

- "FACT: confirm Packet 39 (`stable-entity-ids`) is `implemented`. Read `.ralph/specs/39_stable-entity-ids/packet.spec.md` frontmatter; quote the `status:` line. Confirm `TASK-170` exists in `docs/07_implementation_status.md` (FACT-narrowed; do not load full file)."
- "FACT: locate `FinalizationOutputBuilder` definition. Use ripgrep across `crates/slicer-sdk/` for the struct definition. Return file:line. If the file is < 300 lines, return its size; if > 300 lines, return SUMMARY ≤ 200 words of the impl block (method names + signatures only)."
- "FACT: at the post-Packet-39 finalization-merge site (working hypothesis: still near `crates/slicer-host/src/dispatch.rs:2877`), quote the merge code block ≤ 20 lines. Has the call shape changed (e.g., does it now stamp `entity_id` via `LayerEntityIdGen`)? Cite file:line."
- "LOCATIONS: every `FinalizationModule` impl across `modules/core-modules/`. Use ripgrep for `impl FinalizationModule` (case-sensitive). Return file:line for each. Also list each module's manifest stage if grep can fetch it."
- "FACT: in `modules/core-modules/top-surface-ironing/src/lib.rs`, quote the existing `output.push_entity_to_layer(...)` call ≤ 5 lines (file:line). Confirm there is exactly one such call site."
- "FACT: search `wit/` and `crates/slicer-host/src/wit_host.rs` for any reference to `FinalizationOutputBuilder` or its method names. If positive, the WIT boundary is involved and packet scope expands; report. If negative, return `no WIT exposure`."
- "FACT: in `docs/05_module_sdk.md`, what does the existing `FinalizationOutputBuilder` doc section say about mutation / reorder? Quote the relevant paragraph (≤ 10 lines)."
- "LOCATIONS: enumerate every direct `PrintEntity { ... }` struct-literal construction across `modules/core-modules/`. Use ripgrep `'PrintEntity \\{'` with `--type rust` restricted to `modules/core-modules/`. For each match: file:line + ≤ 1-line snippet + whether the surrounding code instantiates a local `LayerEntityIdGen` (Packet 39 carry-forward — see `## Migration Obligations Inherited from Packet 39`) or already uses a builder method. These sites are this packet's Step 5+ migration targets."

Per Step 1: cargo-build/test FACT after authoring (compile-fail expected on the new test files until Steps 2–4 land).

Per Step 2: `cargo build -p slicer-ir` + `cargo test -p slicer-ir --test extrusion_role_priority_tdd` FACT pass/fail.

Per Step 3: `cargo build -p slicer-sdk` + `cargo test -p slicer-sdk --test finalization_builder_tdd` FACT pass/fail per test (8 tests including negatives).

Per Step 4: `cargo build -p slicer-host` + `cargo test -p slicer-host --test benchy_end_to_end_tdd` FACT pass/fail.

Per Step 5: `cargo build -p top-surface-ironing` + `./modules/core-modules/build-core-modules.sh` + `cargo test -p top-surface-ironing --test top_surface_ironing_emission_tdd` (regression check — should still 8/8 pass) + `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_top_surface_precedes_ironing` FACT pass/fail.

Per Step 6: cargo-test FACT for the workspace gate; one delegated insertion of the `TASK-171` row in `docs/07`.

## Data and Contract Notes

- IR or manifest contracts touched:
  - `ExtrusionRole` — new const fn `default_priority()`. Additive; no breaking change.
  - No new IR fields. Entity struct already carries `entity_id` and `path.role` post-Packet-39.
- WIT boundary considerations: `FinalizationOutputBuilder` is host-side / SDK-side only per session memory. Step 0 confirms by grep. If WIT mirror exists, packet escalates.
- Determinism / scheduler constraints:
  - PostPass is sequential (`docs/04_host_scheduler.md:680-717`). No parallelism inside the merge.
  - Stable-sort is mandatory (Rust's `sort_by` is unstable; use `sort_by_key` with a tuple `(priority, original_index)` OR use `sort_by` with a stable comparator from the `slicer-helpers` crate if one exists).
  - Multiple finalization modules pushing at the same priority preserve producer-call order via the stable-sort tiebreaker.

## Locked Assumptions and Invariants

- `ExtrusionRole` enum has the variants enumerated by Packet 38-rev1's Step 0 (13 + `Custom(String)`): `OuterWall, InnerWall, ThinWall, TopSolidInfill, BottomSolidInfill, SparseInfill, SupportMaterial, SupportInterface, WipeTower, PrimeTower, Ironing, BridgeInfill, Skirt`.
- Producers emit in role order: `OuterWall < InnerWall < ThinWall < SparseInfill < BridgeInfill < BottomSolidInfill < TopSolidInfill < SupportMaterial < SupportInterface < Ironing`. The default-priority table is designed so this order is preserved by the stable-sort.
- Skirt has the lowest priority (0); legacy `push_entity_to_layer` maps to priority 0 to preserve skirt-brim's prepend semantics.
- `Custom(String)` defaults to a fixed priority (e.g., 9000) representing "after most things"; this is a deliberate choice — modules using `Custom` are typically ad-hoc additions.
- Packet 39 is `implemented` and `LayerEntityIdGen` is available for the host merge to stamp IDs on finalization-pushed entities.
- `FinalizationOutputBuilder` lives host-side / SDK-side only (no WIT mirror).
- `gcode_emit.rs` does NOT re-sort `ordered_entities`; the order at emit time is the order produced by the merge sequence.

## Migration Obligations Inherited from Packet 39

Packet `39_stable-entity-ids` made `entity_id: u64` a mandatory field on `PrintEntity`. Two core modules construct `PrintEntity` directly (bypassing `FinalizationOutputBuilder::push_entity_to_layer`) and were updated as out-of-scope deviations: each instantiates a local `slicer_ir::LayerEntityIdGen` and stamps IDs itself. These provisional inserts (logged retroactively in `docs/14_deviation_audit_history.md` DEV-047; originally registered as DEV-039 and renumbered 2026-05-13 to resolve a collision with the canonical DEV-039 packet-35 bbox-fallback entry) are not intended to be permanent.

- `modules/core-modules/wipe-tower/src/lib.rs`
- `modules/core-modules/skirt-brim/src/lib.rs`

When this packet lands `push_entity_with_priority`, the two sites above should be migrated so that:

1. Module code no longer reaches into `slicer-ir` for `LayerEntityIdGen`.
2. `entity_id`s are issued by the host-side merge sequence (using the per-layer `LayerEntityIdGen` already owned by `dispatch.rs` post-Packet-39).
3. The direct `PrintEntity { ... }` struct literals are replaced with builder method calls (`push_entity_to_layer` for skirt-brim's prepend-priority-0 semantics; `push_entity_with_priority` for wipe-tower if it needs a non-default priority).

This is a **Step 5+ obligation** — it cannot start before Step 3 lands the builder API. Track as part of Step 5 ("Top-surface-ironing migration") or a new Step 5b. If migrating the two sites threatens regression (skirt-brim is a documented canary), defer to a follow-up packet and leave a TODO comment in the module sources referencing DEV-047 (originally DEV-039; see renumbering note above).

- **Producer-order vs priority-order mismatch**. If any producer in `layer_executor.rs` emits in an order that does NOT match `default_priority` ordering, the post-merge stable-sort will reorder them, changing G-code output. **Mitigation**: Step 1's `default_priority_orders_correctly` test explicitly validates the table matches producer-emit order. Step 4's `cargo test -p slicer-host --test benchy_end_to_end_tdd` regression catches any divergence (existing assertions stay green).
- **Closure storage in `MergeOp`**. `Box<dyn FnOnce(...)>` is the natural shape for `modify_entity` and `sort_layer_by`, but FnOnce is not object-safe in stable Rust without workarounds (`Box<dyn FnOnce(...) + Send>` works on stable; otherwise `FnMut` or an enum dispatch). Step 1 chooses based on stable Rust's current capabilities and the type's `Send`/`Sync` requirements (PostPass is sequential, so `Send`/`Sync` are not required, but library hygiene may want them).
- **`Custom(String)` priority**. Modules using `Custom` get a default priority of 9000. If a module needs a `Custom` to land mid-stack, it must use `push_entity_with_priority` with an explicit number. Documented in `docs/05_module_sdk.md` update (out of scope for this packet — defer to documentation packet).
- **Builder operation visibility**. `modify_entity` and `sort_layer_by` operate on the *post-merge* state, which means a module cannot read an entity's pre-merge state to decide whether to mutate it. **Mitigation**: the `&[LayerCollectionView]` input the module already receives provides pre-merge read access. Modules query input → record op based on query → host applies op deterministically.
- **Skirt-brim regression**. AC-8 verifies the legacy alias preserves prepend. If skirt-brim somehow regresses (e.g., its role's default priority is wrong, or the alias isn't wired correctly), the regression is caught by both AC-8 and benchy.
- **Out-of-bounds `insert_synthetic_layer_after`**. NEG-2 explicitly verifies the error path. The host MUST validate `idx <= layers.len()` before applying.

## Context Cost Estimate

- Aggregate: `M`.
- Largest single step: `M` (Step 3: builder API surface + 8 tests).
- Highest-risk dispatch: Step 1's `default_priority_orders_correctly` test — its passage is the load-bearing invariant for the merge sequence's correctness.

## Open Questions (resolved or punted to Step 0)

- 🔍 Exact path of `FinalizationOutputBuilder` definition — Step 0 LOCATIONS.
- 🔍 Post-Packet-39 line of the finalization-merge site — Step 0 FACT.
- 🔍 Existing FinalizationModule impls inventory — Step 0 LOCATIONS.
- 🔍 Whether `FinalizationOutputBuilder` crosses any WIT boundary — Step 0 FACT.
- 🔍 Final `default_priority` numeric values (the table in `packet.spec.md` is the working draft; Step 1 may adjust based on producer-emit order verification) — Step 1 decision.
- 🔍 Closure storage shape in `MergeOp` — Step 1 design choice (`Box<dyn FnOnce>` vs op-id registry).

The six 🔍 questions are pre-implementation discovery and design choices answerable inside the packet. No external blockers.
