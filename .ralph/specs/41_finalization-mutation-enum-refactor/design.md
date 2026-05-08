# Design: finalization-mutation-enum-refactor

## Controlling Code Paths

- Primary code path:
  - `crates/slicer-sdk/src/traits.rs` — `FinalizationOutputBuilder` definition + `MergeOp` enum + `apply_to`. The closure-typed methods `modify_entity` and `sort_layer_by` are replaced; `insert_synthetic_layer_after` switches its parameter from `LayerCollectionIR` to `SyntheticLayerData`. `MergeOp` becomes a plain serializable enum.
  - `crates/slicer-sdk/src/lib.rs` — re-export the new types `EntityMutation`, `SortKey`, `SyntheticLayerData` if they don't already flow through the prelude.
  - `crates/slicer-macros/src/lib.rs` `run_finalization` glue (around lines 1198–1214 post-Packet-40-Step-3b-fix) — extend the drain-back loop to forward `merge_ops` via WIT. The current loop drains `priority_pushes()` only; this packet adds a sibling loop draining `merge_ops()` and dispatching each variant to the corresponding WIT method on the bound output resource.
  - `crates/slicer-macros/src/lib.rs` inline WIT in `build_finalization_world_glue` (around lines 948–974) — confirm the WIT method signatures for `modify-entity`, `sort-layer-by`, `insert-synthetic-layer-after` align with the new SDK enum names. Rename WIT types if drift is found.
  - `wit/world-finalization.wit` — canonical WIT mirror; same alignment check.
  - `crates/slicer-host/src/wit_host.rs` — `HostFinalizationOutputBuilder` impl methods. With the SDK now taking enums directly, these methods become straight forwards (no closure construction).
- Reference template / neighbor:
  - `test-guests/sdk-finalization-guest/` — existing test guest at `test-guests/sdk-finalization-guest/src/lib.rs:14`. Mirror its Cargo.toml and module shape for the new `finalization-mutation-roundtrip-guest`.
  - `crates/slicer-sdk/tests/finalization_builder_tdd.rs` — the 8 tests authored in Packet 40 Step 1B. They are the canonical migration target for closure → enum.
- Neighboring tests / fixtures:
  - `crates/slicer-host/tests/benchy_end_to_end_tdd.rs::benchy_top_surface_precedes_ironing` and `benchy_gcode_contains_ironing_evidence` — must continue to PASS unchanged. Packet 40's print-quality fix is the regression baseline.
  - `modules/core-modules/top-surface-ironing/tests/top_surface_ironing_emission_tdd.rs` — must continue to PASS 8/8 (this packet does not touch the module).
  - `crates/slicer-sdk/tests/finalization_module_tdd.rs` — must continue to PASS 7/7 (existing fixtures use `push_entity_to_layer`; not affected by the closure-method refactor).
- OrcaSlicer comparison surface:
  - None required. If parity is challenged for `EntityMutation::SetSpeedFactor` semantics, delegate one SUMMARY ≤ 200 words on `OrcaSlicerDocumented/src/libslic3r/GCode/CoolingBuffer.cpp` for context only.

## Architecture Constraints

- **Closure-free SDK API for the three mutation methods**. This is the load-bearing invariant of the packet. After Step 2, `modify_entity` / `sort_layer_by` / `insert_synthetic_layer_after` MUST take only types that are `Serialize + Deserialize + Clone + Debug` (or whatever subset the SDK's transport convention requires). No `Box<dyn FnOnce>`, no `Box<dyn Fn>`, no impl-trait closure parameters. NEG-4 grep-asserts this contract.
- **`MergeOp` is plain data**. The internal `Vec<MergeOp>` storage on `FinalizationOutputBuilder` becomes plain serializable data. No type erasure, no boxed closures. Tests that previously used `Box::new(|e| …)` are now constructing `EntityMutation::Set*` variants directly.
- **WIT and SDK shapes are unified**. Packet 40 Step 3b introduced `entity-mutation`, `sort-key`, `synthetic-layer-data` on the WIT side. This packet ensures the SDK uses the SAME shapes (or trivially mappable equivalents). The `wit_host.rs` translation layer becomes a one-liner per method (or vanishes entirely if the bindgen can autoderive).
- **No producer-order or role-priority changes**. The role-priority table from Packet 40 stays put. `apply_to`'s 5-phase merge order stays put: (1) extend + ID-stamp, (2) stable-sort by `(priority, original_index)`, (3) apply `MergeOp::ModifyEntity`, (4) apply `MergeOp::SortLayer`, (5) apply `MergeOp::InsertSynthLayer` at outer Vec.
- **Operation recording, not direct mutation**. Every WIT call still RECORDS an op; the host applies them after the module returns. Same model as Packet 40; only the recording shape changes.
- **Stable identity**. Mutations match by `entity_id` (Packet 39 invariant). NEG-1 and NEG-3 verify the unknown-id error path at SDK and WIT layers respectively.
- **Backwards compatibility for `push_entity_with_priority` and `push_entity_to_layer`**. Both stay closure-free already; this packet does not touch them. AC-8 verifies benchy still passes, which exercises the legacy alias via skirt-brim.
- **Drain-back symmetry**. After Step 5, the drain-back forwards BOTH `priority_pushes` AND `merge_ops`. AC-7 grep-asserts the iteration site exists; the round-trip ACs (5, NEG-3) prove the forwarding works end-to-end.
- **`SyntheticLayerData` defaults**. When `apply_to` constructs a `LayerCollectionIR` from `SyntheticLayerData { z, paths }`, sibling fields (`global_layer_index`, `is_top_layer`, `is_bottom_layer`, `is_first_layer`, `is_last_layer`, `region_membership`, `travel_moves`, `ordered_entities` initial state, etc.) get sensible defaults: `global_layer_index` is the insertion index in the new outer Vec; layer-flag booleans are all `false` (synthetic layers are not first/last/top/bottom by default); `region_membership` is empty; `travel_moves` is empty; `ordered_entities` is built from the supplied `paths` with `entity_id`s stamped from a fresh `LayerEntityIdGen`. Locked at Step 0 audit; documented inline at the construction site.

## Code Change Surface

- Selected approach:
  - **Serializable-enum SDK + thin WIT-host forward**. SDK and WIT share the same enum/record shapes. The drain-back loop forwards `merge_ops` directly. `apply_to` translates `EntityMutation` to a concrete in-place mutation via a match arm per variant.
- Rejected alternatives (briefly):
  - **Hybrid: closure SDK + enum WIT**. Locks in two API surfaces forever; cfg-gating closure path for WASM is fragile; module authors must remember which to call. Rejected.
  - **Shadow-entity diff extraction**. Run the closure on a synthetic in-WASM PrintEntity, extract a diff, forward the diff via WIT. Theoretically clever; fails for any read-then-mutate logic; leaky abstraction. Rejected.
  - **Deprecate-but-keep closure API as `#[deprecated]` aliases**. Adds carry cost and tempts module authors to use the wrong path. Closure callers in this codebase number exactly eight (the existing tests). Migrate them and remove the closure API outright. Rejected.
  - **`EntityMutation::Patch(serde_json::Value)` escape hatch from day one**. Adds runtime cost and parsing surface for no current consumer. Rejected; defer until a real consumer needs it.
  - **Mirror full `LayerCollectionIR` into `SyntheticLayerData`**. Heavy WIT marshalling for fields no current consumer needs. Rejected; minimal `(z, paths)` only.
- Exact functions, traits, manifests, tests expected to change:
  - `crates/slicer-sdk/src/traits.rs` — `FinalizationOutputBuilder` impl block: replace three method bodies + signatures; refactor `MergeOp` enum; rewrite `apply_to`'s match arms for each `MergeOp` variant. Add a `merge_ops()` accessor returning `&[MergeOp]` if not already present (Packet 40 Step 3b-fix added `priority_pushes()`; check for `merge_ops()` symmetry).
  - `crates/slicer-sdk/src/lib.rs` — re-export `EntityMutation`, `SortKey`, `SyntheticLayerData` if not already accessible via prelude.
  - `crates/slicer-sdk/tests/finalization_builder_tdd.rs` — migrate the 8 existing tests; rename `modify_entity_by_id_applies_closure` → `modify_entity_by_id_applies` (or split into per-variant tests); rename `sort_layer_by_applies_comparator` → `sort_layer_by_applies` or per-variant; add `modify_entity_set_speed_factor_applies` and `modify_entity_set_extrusion_width_factor_applies` for AC-1 and AC-2 explicit coverage; add `closure_api_is_fully_removed` for NEG-4.
  - `wit/world-finalization.wit` — alignment check; rename WIT types only if drift requires it. Most likely no edit if Packet 40 Step 3b's names already match this packet's intent.
  - `crates/slicer-host/src/wit_host.rs` — `HostFinalizationOutputBuilder` impl: replace the three closure-construction methods with direct forwards. Confirm `WitEntityMutation` / `WitSortKey` types are aligned with SDK's `EntityMutation` / `SortKey`; if Packet 40 used distinct names, decide whether to converge.
  - `crates/slicer-macros/src/lib.rs` — `run_finalization` glue: extend drain-back to iterate `sdk_output.merge_ops()` after the existing `priority_pushes()` iteration. For each variant, call the WIT-bound method on `output`. Inline WIT in `build_finalization_world_glue` — alignment check, no edit if names match.
  - `test-guests/finalization-mutation-roundtrip-guest/Cargo.toml` (new file) — model on `test-guests/sdk-finalization-guest/Cargo.toml`.
  - `test-guests/finalization-mutation-roundtrip-guest/src/lib.rs` (new file) — implement `FinalizationModule`; in `run_finalization` call `output.modify_entity(layer, 1, EntityMutation::SetSpeedFactor(0.5))`. The guest may also accept a config flag (or have a sibling impl) that calls with unknown id `99` for NEG-3; alternatively two separate guests if a single one can't toggle behavior.
  - `crates/slicer-host/tests/finalization_mutation_roundtrip_tdd.rs` (new file) — host-side end-to-end test using the new guest. At least three test functions per the AC list.
  - `docs/07_implementation_status.md` — one new row for `TASK-172`.
  - `docs/14_deviation_audit_history.md` — append `DEV-041 closed` line in chronology section.
- Test files added by this packet:
  - `crates/slicer-host/tests/finalization_mutation_roundtrip_tdd.rs`
  - (No new SDK test file; the existing `finalization_builder_tdd.rs` is migrated, not duplicated.)

## Files in Scope (read + edit)

Primary edit targets per step (≤ 3 per step):

- Step 0 ("Discovery + audit"): no edits; dispatches only.
- Step 1 ("Failing TDD migration"): `crates/slicer-sdk/tests/finalization_builder_tdd.rs` + `crates/slicer-host/tests/finalization_mutation_roundtrip_tdd.rs` (new, may be stub at this stage) + (NEW) `test-guests/finalization-mutation-roundtrip-guest/{Cargo.toml,src/lib.rs}` (counted as one effective edit target since they're created together; if scope feels tight, splitting the guest authoring into Step 1b is acceptable).
- Step 2 ("Define new types"): `crates/slicer-sdk/src/traits.rs` (1 file).
- Step 3 ("SDK API replacement"): `crates/slicer-sdk/src/traits.rs` + `crates/slicer-sdk/src/lib.rs` (re-exports if needed) (≤ 2 files).
- Step 4 ("WIT alignment + host-impl simplification"): `wit/world-finalization.wit` (alignment) + `crates/slicer-host/src/wit_host.rs` + `crates/slicer-macros/src/lib.rs` inline WIT only (≤ 3 files; keep edits narrow).
- Step 5 ("Drain-back wiring + WASM round-trip test"): `crates/slicer-macros/src/lib.rs` drain-back loop + `test-guests/finalization-mutation-roundtrip-guest/src/lib.rs` + `crates/slicer-host/tests/finalization_mutation_roundtrip_tdd.rs` (≤ 3 files). The macro change is the substantive fix; the guest + host test are the substantive validation.
- Step 6 ("Acceptance + docs"): `docs/07_implementation_status.md` (delegated insertion) + `docs/14_deviation_audit_history.md` (closure note) (≤ 2 files).

## Read-Only Context

- `docs/01_system_architecture.md` lines 328–363 — mutability contract.
- `docs/04_host_scheduler.md` lines 309–317, 680–717.
- `docs/05_module_sdk.md` — relevant `FinalizationOutputBuilder` section only (delegate SUMMARY).
- `docs/02_ir_schemas.md` — `PrintEntity`, `ExtrusionPath3D`, `LayerCollectionIR`, `TravelMove`.
- `docs/03_wit_and_manifest.md` — WIT shape conventions, narrow.
- `crates/slicer-sdk/src/traits.rs` — full read OK if < 600 lines after Packet 40 changes; otherwise narrow.
- `crates/slicer-host/src/wit_host.rs` — narrow (HostFinalizationOutputBuilder impl block + WitEntityMutation/WitSortKey type defs).
- `crates/slicer-macros/src/lib.rs` — narrow (lines 948–974 inline WIT, lines 1198–1214 drain-back).
- `wit/world-finalization.wit` — full read OK (~80 lines after Packet 40).
- `test-guests/sdk-finalization-guest/{Cargo.toml,src/lib.rs}` — full read (small; precedent for the new guest).
- `.ralph/specs/40_finalization-mutation-builder/design.md` — narrow (Open Questions section listing future modules).
- `docs/14_deviation_audit_history.md` — narrow (DEV-041 entry).

## Out-of-Bounds Files

- `OrcaSlicerDocumented/` — never load (delegate only if parity is challenged).
- `target/`, `Cargo.lock`, generated code — never load.
- `crates/slicer-host/src/dispatch.rs` — Packet 40 Step 4 closed this code path. Do not re-open.
- `crates/slicer-host/src/gcode_emit.rs`, `manifest.rs`, `layer_executor.rs` — not touched by this packet.
- `crates/slicer-ir/src/slice_ir.rs` outside narrow reads of `PrintEntity`/`ExtrusionPath3D`/`LayerCollectionIR`/`TravelMove`.
- `modules/core-modules/{skirt-brim,wipe-tower,top-surface-ironing}/src/lib.rs` — none consume `merge_ops`; none change in this packet.
- All other modules under `modules/core-modules/` — out of scope.
- All other crates (`slicer-helpers`, `slicer-core`, `slicer-schema`).

## Expected Sub-Agent Dispatches

Per Step 0:

- "FACT: confirm Packet 40 (`finalization-mutation-builder`) is `implemented`. Read `.ralph/specs/40_finalization-mutation-builder/packet.spec.md` frontmatter; quote the `status:` line. Confirm `TASK-171` exists in `docs/07_implementation_status.md`."
- "FACT: locate `FinalizationOutputBuilder`, `MergeOp`, and `priority_pushes()` / `merge_ops()` accessors in `crates/slicer-sdk/src/traits.rs`. Quote each declaration site (≤ 5 lines each, with file:line). Confirm whether `merge_ops()` accessor exists today; if not, that's a one-line add at Step 3."
- "FACT: in `wit/world-finalization.wit`, quote the existing `entity-mutation`, `sort-key`, and `synthetic-layer-data` definitions (≤ 30 lines total, with file:line). These are this packet's WIT alignment baseline."
- "FACT: in `crates/slicer-macros/src/lib.rs` `build_finalization_world_glue`, quote the inline WIT for the same three types (≤ 30 lines total). Confirm whether names match `wit/world-finalization.wit` or have drifted."
- "SNIPPETS: `crates/slicer-host/src/wit_host.rs` `HostFinalizationOutputBuilder` impl methods for `modify_entity`, `sort_layer_by`, `insert_synthetic_layer_after` — verbatim ≤ 30 lines each. We need to see how Packet 40's translation logic works to plan the simplification."
- "SUMMARY ≤ 200 words: read `.ralph/specs/40_finalization-mutation-builder/design.md` Open Questions and the four future-module references (`SequentialPrintOrder`, `MinLayerTimeEnforcer`, `FlushVolumeCalculator`, `PrimeTower`). For each module, list the `PrintEntity` (or `ExtrusionPath3D`) fields it plausibly mutates. This drives Step 0's `EntityMutation` variant lock."
- "FACT: list every `Cargo.toml` under `test-guests/`. Return file:line of each `[package]` `name` field (so we can model the new test-guest)."
- "FACT: list every existing test under `crates/slicer-host/tests/` (LOCATIONS only, ≤ 30 entries). Confirm `finalization_mutation_roundtrip_tdd.rs` does NOT already exist."

Per Step 1: cargo-build/test FACT after authoring (compile-fail expected on the migrated tests until Steps 2–3 land; assertion-fail expected on the new round-trip test until Steps 4–5 land).

Per Step 2: `cargo build -p slicer-sdk` FACT pass/fail.

Per Step 3: `cargo build -p slicer-sdk` + `cargo test -p slicer-sdk --test finalization_builder_tdd` FACT pass/fail per test.

Per Step 4: `cargo build -p slicer-host` + `cargo build -p slicer-macros` + `./modules/core-modules/build-core-modules.sh` FACT pass/fail.

Per Step 5: `cargo test -p slicer-host --test finalization_mutation_roundtrip_tdd` FACT pass/fail per test + `cargo test -p slicer-host --test benchy_end_to_end_tdd` regression FACT.

Per Step 6: cargo-test FACT for the workspace gate; one delegated insertion of the `TASK-172` row in `docs/07`; one delegated append of the `DEV-041 closed` line in `docs/14`.

## Data and Contract Notes

- IR or manifest contracts touched:
  - No new IR fields.
  - No new manifest entries.
- WIT boundary considerations: the WIT shapes for `entity-mutation`, `sort-key`, `synthetic-layer-data` already exist (Packet 40 Step 3b). Step 4 confirms alignment. If the SDK names diverge from the WIT names, Step 4 chooses one canonical form (recommend SDK adopts WIT names verbatim, or vice-versa, with consistent kebab/snake/camel conventions per Rust/WIT idiom).
- Determinism / scheduler constraints:
  - PostPass is sequential (`docs/04_host_scheduler.md:680–717`). No parallelism inside the merge.
  - Stable-sort is mandatory (Packet 40 invariant; preserved here).
  - Multiple `merge_ops` of the same kind apply in record-order (preserved from Packet 40).

## Locked Assumptions and Invariants

- `PrintEntity` shape is unchanged from Packet 39+40. `entity_id`, `path`, `role`, `region_membership`, `travel_moves` are the relevant fields.
- `ExtrusionPath3D` carries `speed_factor: f32`, `extrusion_width_factor: f32`, and the nested `Point3WithWidth.flow_factor` for per-point flow. `EntityMutation::SetSpeedFactor` and `SetExtrusionWidthFactor` mutate the path-level fields. `SetFlowFactor` semantics: Step 0 audit decides whether this is per-path (apply to every point's `flow_factor`) or per-point (variant carries an index). Recommend per-path for the initial variant set; per-point can be added in a future packet.
- `apply_to`'s 5-phase merge order is preserved.
- The `push_entity_with_priority` method from Packet 40 is unchanged — closure-free already.
- The `push_entity_to_layer` legacy alias is unchanged.
- `LayerEntityIdGen` (from `slicer-ir`) is the canonical source for stamping `entity_id` on synthetic-layer entities; `apply_to`'s synthetic-layer construction uses a fresh generator per inserted layer (Packet 40 invariant; preserved).

## Migration Obligations Inherited from Packet 40

None. Packet 40's follow-up session migrated `skirt-brim` and `wipe-tower` to the builder API (`LayerEntityIdGen` removed from both module sources). Top-surface-ironing was migrated as part of Packet 40 itself. No further module-side migrations are owed.

## Risks and Mitigations

- **`EntityMutation` variant coverage too narrow**. If Step 0's audit produces a list that misses a near-future module's needs, the next packet has to extend `EntityMutation` retroactively. Mitigation: be generous on initial variants; the compiler complains about unused variants only at warn level. Step 0 explicitly audits the four future modules' design intent.
- **WIT name drift between SDK and WIT after Packet 40 Step 3b**. Likely WitEntityMutation / WitSortKey were prefixed `Wit` to avoid namespace collision. Step 4 decides: rename SDK to drop the prefix (and the WIT-host translation layer trivially aligns), or keep the prefix and tolerate one more layer. Recommendation: SDK adopts unprefixed names (`EntityMutation`, `SortKey`); WIT-host translation collapses to `From` impl or direct field map.
- **Test guest scaffolding complexity**. `test-guests/sdk-finalization-guest/` is the precedent; if its build pipeline requires manifest registration or component-model glue, the new guest needs the same. Step 0 confirms via FACT.
- **Existing closure-using test count larger than 8**. If migration touches more than the expected 8 tests, step cost grows. Mitigation: Step 1 first FACT-greps `crates/slicer-sdk/tests/finalization_builder_tdd.rs` for `Box::new(|` and `|e|` closure literals; if count > 10, Step 1 splits into 1a/1b.
- **`SortKey::ByObjectIdThenPriority` requires `object_id` access on PrintEntity**. Verify at Step 0 that `PrintEntity` carries `object_id` (or has a path to it via region membership). If not, drop this variant or defer to a future packet that adds the field.
- **Benchy regression**. The packet should not change G-code output. Step 5 runs benchy as the final canary before acceptance ceremony.
- **`apply_to` error semantics for unknown `entity_id`**. NEG-1 expects an `Err` containing `entity_id` and the offending value. NEG-3 expects the same for the WIT round-trip. Both ACs assume the host returns the diagnostic; verify the WIT method's `result<_, string>` shape carries the message verbatim.

## Context Cost Estimate

- Aggregate: `M`.
- Largest single step: `M` (Step 5: drain-back + WASM round-trip test guest + 3+ host tests).
- Highest-risk dispatch: Step 0's variant-audit SUMMARY — its output dictates the entire `EntityMutation` and `SortKey` shape.

## Open Questions (resolved or punted to Step 0)

- 🔍 Final `EntityMutation` variant list — Step 0 audit decision.
- 🔍 Final `SortKey` variant list — Step 0 audit decision.
- 🔍 `EntityMutation::SetFlowFactor` semantics (per-path vs per-point) — Step 0 decision.
- 🔍 Whether to keep WIT-side `WitEntityMutation` prefix or rename to match SDK — Step 4 decision.
- 🔍 Whether `merge_ops()` accessor exists on the SDK builder today — Step 0 FACT.
- 🔍 Whether the new test guest needs a separate manifest entry — Step 0 FACT (compare to sdk-finalization-guest).
- 🔍 Whether `PrintEntity.object_id` (or equivalent) exists for `SortKey::ByObjectIdThenPriority` — Step 0 FACT; if no, drop the variant or defer.

The seven 🔍 questions are pre-implementation discovery and design choices answerable inside the packet's Step 0. No external blockers.
