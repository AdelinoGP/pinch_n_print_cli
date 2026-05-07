# Design: top-surface-ironing-rev1

## Controlling Code Paths

- Primary code path:
  - `modules/core-modules/top-surface-ironing/` (existing directory; predecessor packet 38 left a wrong-stage implementation here — rev1 rewrites in place):
    - `Cargo.toml` (rewrite to mirror `skirt-brim`)
    - `top-surface-ironing.toml` (rewrite manifest: stage `PostPass::LayerFinalization`, ir-access for `LayerCollectionIR`, claims empty, hints `layer-parallel-safe = false`, Orca-aligned config defaults)
    - `src/lib.rs` (rewrite to implement `FinalizationModule` with `run_finalization` callback)
    - `tests/top_surface_ironing_emission_tdd.rs` (rewrite for object-scope fixtures)
    - `wit-guest/Cargo.toml` and `wit-guest/src/lib.rs` (adjust the cdylib re-export to match the new trait shape if needed; `skirt-brim/wit-guest/` is the precedent)
- Reference template:
  - `modules/core-modules/skirt-brim/` — read as the authoritative PostPass::LayerFinalization template. The predecessor packet incorrectly used `support-surface-ironing/` (a `Layer::SupportPostProcess` module) as its template; that mismatch is part of why the predecessor went wrong.
- Neighboring tests / fixtures:
  - `crates/slicer-host/tests/benchy_end_to_end_tdd.rs::benchy_gcode_contains_ironing_evidence` (already authored by predecessor packet; verify still compiles + check whether the assertion needs adjustment after stage relocation — the assertion is on G-code text only, so likely no change)
  - `crates/slicer-host/tests/manifest_ingestion_tdd.rs` — `core_modules_directory_is_discoverable_and_all_load` hardcodes a count (currently expecting 19 but the directory has 20 with `top-surface-ironing` present); also `core_modules_all_have_placeholder_wasm_flag_set` expects every manifest to declare `placeholder_wasm = true` while NO existing core-module manifest declares this field (predecessor Step 0 confirmed). Step 0 must determine whether this test is fixture-bug (test wrongly asserts a default), schema-bug (host-side default differs from test expectation), or genuine convention (every new module must declare the flag — meaning all 19 existing manifests are out of compliance, which is unlikely).
  - `crates/slicer-host/tests/claim_transition_matrix_tdd.rs::stable_holder_across_layers_is_valid_for_non_transitionable_claim` — predecessor pass observed `MissingDependency { module: "fill-role-claim:claim:top-fill", requires: "no module holds claim:top-fill" }`. The new module's `[claims].holds = []`, so this should not be a real claim collision. Most likely root causes: (a) test fixture hardcodes the set of expected claim holders and the new module's presence breaks an unrelated count invariant, (b) packet 37's recently-shipped fill-role-claim wiring expects an exact module set. Step 0 dispatches a worker to read the failing test source and identify the root cause before assuming a fix.
- OrcaSlicer comparison surface:
  - `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp::make_ironing` (~line 1530) — algorithm shape and defaults
  - `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp::ironing()` (~line 838) — phase ordering
  - `OrcaSlicerDocumented/src/libslic3r/Layer.hpp::LayerRegion::make_ironing` — declaration

## Architecture Constraints

- **Stage**: `PostPass::LayerFinalization`. Sequential, single-threaded, runs after all per-layer stages drain via the rayon join (`docs/04_host_scheduler.md:680-717`). The module sees the FULL `Vec<LayerCollectionIR>` for all objects in the print; cross-layer look-ahead is therefore native, not synthesized.
- **Trait**: `FinalizationModule` with `run_finalization(&self, layers: &[LayerCollectionView], output: &mut FinalizationOutputBuilder, _config: &ConfigView) -> Result<(), ModuleError>` (signature from `skirt-brim/src/lib.rs:300-305`). NOT `LayerModule::run_infill_postprocess` — that was the predecessor's mistake.
- **Output channel**: `output.push_entity_to_layer(layer_index, path, region_key)` (precedent `skirt-brim/src/lib.rs:347-349`). Module does NOT mutate the input `&[LayerCollectionView]`. Host's dispatch code at `crates/slicer-host/src/dispatch.rs:2877` collects pushes and merges them into `layer.ordered_entities` via `splice(0..0, ...)` (prepend). **The prepend behavior is a known concern** — see "Risks and Tradeoffs" — Step 0 must verify whether the splice actually targets the front or whether the index is computed per push, AND whether the host has any role-based ordering that would correctly place Ironing entities after fill entities at G-code emit time. If pure prepend is the only behavior, Step 0a extends the SDK to support an APPEND mode before Step 3 implementation.
- **IR-access transform chain**: `reads = ["LayerCollectionIR"]`, `writes = ["LayerCollectionIR.ironing"]`. The kebab-case sub-field write target is the canonical pattern from `skirt-brim` (`"LayerCollectionIR.skirt-brim"`); Step 0 confirms the exact ironing field name from the IR schema.
- **Detection mechanism**: object-scope direct lookup. For each `(object_id, region_key)` derivable from the `LayerCollectionView` slice, scan `0..layers.len()` from highest index downward; the first index whose region carries any `TopSolidInfill` paths is "the topmost top-solid layer" for that region. Emit ironing only on that layer. This requires no `is_top_surface` flag, no SDK extension to per-region views, and no proxy.
- **Coordinate system**: 1 unit = 100 nm (`docs/08_coordinate_system.md`). All zigzag generation must use `Point2::from_mm` / `mm_to_units()`; never assume Orca's 1 unit = 1 nm.
- **Append-only contract**: ironing entities are emitted as ADDITIONAL entities on the topmost layer; existing `TopSolidInfill` paths in input layers are not touched (the module reads `&[LayerCollectionView]` — read-only by signature; mutations are scoped to the output builder).
- **Determinism**: PostPass is sequential with pool size 1 per `docs/04_host_scheduler.md:680-717`. No parallelism inside the module is needed or allowed.

## Code Change Surface

- Selected approach:
  - Rewrite `top-surface-ironing/{Cargo.toml, top-surface-ironing.toml, src/lib.rs, tests/top_surface_ironing_emission_tdd.rs, wit-guest/Cargo.toml, wit-guest/src/lib.rs}` to mirror `skirt-brim` exactly in skeleton; insert ironing-specific algorithm in `run_finalization`. The implementation algorithm:
    1. Read all five config keys via `ConfigView::get_*`.
    2. Validate (`ironing_flow > 0.0`; `ironing_pattern == "rectilinear"`); reject via `ModuleError::fatal(code, message)` whose message names the offending key.
    3. If `ironing == false`, return `Ok(())` without emitting (preserves zero-output semantics for AC-4).
    4. Build a map `(object_id, region_key) → Option<usize>` recording the highest layer index whose region carries any `TopSolidInfill` paths.
    5. For each entry where the index is `Some(idx)`:
       - Collect that region's `TopSolidInfill` paths from `layers[idx]`.
       - Compute the bounding ExPolygon (or union — `slicer-helpers` polygon utilities; Step 0 FACT confirms availability).
       - Generate a rectilinear zigzag at `ironing_spacing` mm — horizontal strokes at Y intervals, alternating direction, snake-style. For a 10 mm × 10 mm region at `ironing_spacing = 0.1`, expect ≥ 100 stroke points.
       - For each stroke, build an `ExtrusionPath3D` with `role == ExtrusionRole::Ironing`, `flow_factor == ironing_flow`, `speed_factor` derived from `ironing_speed` per the `skirt-brim` precedent.
       - Push via `output.push_entity_to_layer(idx as u32, path, region_key.clone())`.
    6. Return `Ok(())`.
- Rejected alternatives:
  - **Keep `Layer::InfillPostProcess` and extend SDK to expose `is_top_surface()` on `PerimeterRegionView`** — rejected: requires synthesizing cross-layer information into a per-layer-parallel view, which is architecturally backwards and adds an SDK surface that propagates the fundamental confusion.
  - **Add a new `PostPass::Ironing` stage** — rejected: needlessly expands host scope (dispatch.rs, scheduler doc, STAGE_ORDER) for behavior that fits cleanly into the existing `PostPass::LayerFinalization` slot. `skirt-brim` proves the pattern.
  - **Inline ironing inside `rectilinear-infill`** — rejected: ironing is a distinct print operation with separate config, timing, and stage scope; mixing pollutes the fill module's responsibility and re-introduces the per-layer detection problem.
- Exact functions, traits, manifests, tests expected to change:
  - `modules/core-modules/top-surface-ironing/Cargo.toml` — mirror `skirt-brim/Cargo.toml`; package name stays `top-surface-ironing`; deps stay `slicer-sdk`, `slicer-schema`, `slicer-ir`; wasm32 dep `wit-bindgen = "0.24"`
  - `modules/core-modules/top-surface-ironing/top-surface-ironing.toml` — manifest rewrite as described above
  - `modules/core-modules/top-surface-ironing/src/lib.rs` — full body rewrite implementing `FinalizationModule`
  - `modules/core-modules/top-surface-ironing/tests/top_surface_ironing_emission_tdd.rs` — full rewrite with object-scope fixtures (8 tests; the AC-TSI-3 fixture must carry real `top_shell_layers = 3` interior+topmost geometry, not an empty region as in the predecessor)
  - `modules/core-modules/top-surface-ironing/wit-guest/Cargo.toml` and `wit-guest/src/lib.rs` — adjust to match `skirt-brim/wit-guest/` (re-exports `TopSurfaceIroning` as a `FinalizationModule` cdylib)
  - `crates/slicer-host/tests/manifest_ingestion_tdd.rs` — count adjustment + `placeholder_wasm` reconciliation (Step 4)
  - `crates/slicer-host/tests/claim_transition_matrix_tdd.rs` — only if Step 0 finds a real fixture-side fix (otherwise out of scope and the failure is escalated to a separate packet)
  - `docs/07_implementation_status.md` — insert TASK-169 row at acceptance ceremony

## Files in Scope (read + edit)

Primary edit targets per step (≤ 3 per step):

- Step 1 ("Failing TDD"): `tests/top_surface_ironing_emission_tdd.rs` (1 file).
- Step 2 ("Module skeleton"): `Cargo.toml` + `top-surface-ironing.toml` + `src/lib.rs` (3 files).
- Step 2a ("wit-guest sync"): `wit-guest/Cargo.toml` + `wit-guest/src/lib.rs` (2 files).
- Step 3 ("Implementation"): `src/lib.rs` body (1 file).
- Step 4 ("Workspace test reconciliation"): `manifest_ingestion_tdd.rs` (+ optionally `claim_transition_matrix_tdd.rs` if mechanical) (≤ 2 files).
- Step 5 ("Acceptance + docs"): `docs/07_implementation_status.md` (1 file).

## Read-Only Context

- `modules/core-modules/skirt-brim/` — full read; this is the authoritative template. Files are small.
- `modules/core-modules/skirt-brim/wit-guest/` — full read; mirrors a PostPass module's cdylib pattern.
- `docs/04_host_scheduler.md` lines 680–717, 309–317, 57–63 — direct narrow reads.
- `docs/05_module_sdk.md` — `FinalizationModule` trait section + `FinalizationOutputBuilder` API (delegate SUMMARY for sections > 100 lines).
- `docs/02_ir_schemas.md` — `LayerCollectionIR`, `InfillRegion`, `ExtrusionPath3D`, `ExtrusionRole` (one section).
- `docs/03_wit_and_manifest.md` — `[ir-access]` rules.
- `docs/08_coordinate_system.md` — small; full read.
- `crates/slicer-sdk/src/views.rs` — only `LayerCollectionView`, `FinalizationOutputBuilder`, `ConfigView`. Symbol search; no full read.
- `crates/slicer-helpers/src/lib.rs` — symbol search only for polygon-union / bounding utilities.
- `crates/slicer-host/src/dispatch.rs` — only the `PostPass::LayerFinalization` lines (2815, 1124-1130, 2877). Already FACT-narrowed.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` — delegate only.
- `target/`, `Cargo.lock`, generated code — never load.
- `crates/slicer-host/src/dispatch.rs` beyond the four narrow line ranges above — out of scope.
- `crates/slicer-host/src/gcode_emit.rs` — out of scope; predecessor confirmed mapping at line 91.
- `wit/` — no WIT changes.
- All other crates not listed above.
- All other core modules beyond `skirt-brim` and the existing `top-surface-ironing` directory (do NOT re-read `support-surface-ironing` — it is the wrong template and is what misled the predecessor).

## Expected Sub-Agent Dispatches

Per Step 0:

- "FACT: in `crates/slicer-host/src/dispatch.rs` line 2877, does `splice(0..0, …)` always prepend, or is the index parameterized? Quote the exact line. If parameterized, what determines the insertion point? Cite file:line."
- "FACT: in the IR schema (`crates/slicer-ir/src/slice_ir.rs`), what is the canonical kebab-case field name for ironing on `LayerCollectionIR`? Is it `LayerCollectionIR.ironing`, `LayerCollectionIR.regions.ironing`, or something else? Search for `#[serde(rename = ...)]` or schema-export macros. Cite file:line."
- "FACT: does `slicer-helpers` expose a polygon-union or bounding-box utility for a slice of paths? Symbol search; return the function name and file:line."
- "FACT: in `crates/slicer-host/tests/manifest_ingestion_tdd.rs`, what is the test fixture for `core_modules_all_have_placeholder_wasm_flag_set` checking? Specifically, does it expect every manifest to declare `placeholder_wasm = true`, or does it allowlist a subset? Quote the assertion. Cite file:line."
- "SUMMARY ≤ 200 words: in `crates/slicer-host/tests/claim_transition_matrix_tdd.rs::stable_holder_across_layers_is_valid_for_non_transitionable_claim`, what does the test assert and how is the new top-surface-ironing module breaking it? Read the test fixture and the failing assertion path. Cite file:line. Distinguish: (a) hardcoded module-set count, (b) claim-graph derivation involving the new module, (c) packet 37 fill-role-claim wiring expecting an exact module set."

Per Step 1: none beyond the standard cargo-build/test FACT after authoring.

Per Step 3: cargo-test FACT per assertion-bound iteration.

Per Step 5: cargo-test FACT for the workspace gate; one delegated insertion of the TASK-169 row in `docs/07`.

## Data and Contract Notes

- IR or manifest contracts touched:
  - `LayerCollectionIR` — read (full input visibility) + write (ironing strokes appended to a designated sub-field, exact kebab-case path TBD by Step 0).
  - `InfillRegion.solid_infill` and `InfillRegion.ironing` — both exist (predecessor's Step 0 dispatch confirmed `slicer-ir/src/slice_ir.rs:1354-1365`). The `ironing` sub-field is the natural target; whether the host merges `ironing` paths into G-code or whether the module's emitted entities flow through `ordered_entities` is the open question Step 0 resolves.
- WIT boundary considerations: none new — the `FinalizationModule` trait is already host-supported (precedent: `skirt-brim`).
- Determinism / scheduler constraints:
  - Transform-chain edge fill→ironing established by `reads/writes` declarations.
  - PostPass is sequential; no internal parallelism allowed.

## Locked Assumptions and Invariants

- `ExtrusionRole::Ironing` enum variant exists (predecessor confirmed at `crates/slicer-host/src/wit_host.rs:2572`).
- `ExtrusionRole::Ironing => ";TYPE:Ironing"` mapping exists at `crates/slicer-host/src/gcode_emit.rs:91` (predecessor confirmed).
- `skirt-brim` module is a working `PostPass::LayerFinalization` module and its skeleton is the canonical template.
- `FinalizationOutputBuilder::push_entity_to_layer(layer_index, path, region_key)` is the only emission API for finalization modules (per `skirt-brim/src/lib.rs:347-349`).
- The host's per-layer entity merge at `dispatch.rs:2877` integrates finalization pushes into `ordered_entities` so that G-code emit picks up the role marker. Pending confirmation of insertion order — see Risks.

## Risks and Tradeoffs

- **Insertion-order risk (highest implementation risk).** If the host's `splice(0..0, ...)` at `dispatch.rs:2877` literally prepends, ironing entities will appear BEFORE fill entities in G-code, which is wrong (ironing must follow fill within a layer). Mitigations in priority order:
  1. Step 0 FACT confirms whether the index parameter is actually `0` or computed per push. If computed, the issue is moot.
  2. If the splice is purely prepend, Step 0a extends the SDK / host to provide an APPEND or AFTER-region insertion mode. Scope expansion stays inside `crates/slicer-sdk/` and `crates/slicer-host/src/` finalization paths; no stage-graph changes.
  3. Worst case: if (1) and (2) are both blocked, the packet stays `draft` and a follow-up packet is opened for the SDK extension before reactivating this one. The module-level tests CAN still verify the entity-push contract (assertion is on `output.entity_pushes()`, which records pushes regardless of host merge order); only the Benchy E2E AC-6 is sensitive to the merge order.
- **`claim_transition_matrix_tdd` regression**. May or may not be packet-attributable. Step 0 SUMMARY determines whether the fix is mechanical or substantive; a substantive fix may exceed packet scope and would be carved out into a separate packet.
- **`placeholder_wasm` convention**. Predecessor's Step 0 found NO existing core-module manifest declares the field. Either every existing manifest is non-compliant (unlikely) or the test fixture is wrong. Step 0 FACT determines the correct fix.
- **Bounding ExPolygon vs union**. For complex top surfaces (donuts, multi-island regions), a bounding-box approach over-irons empty space. Orca uses union (`Fill.cpp:1719`). The packet's algorithm calls for union via `slicer-helpers`; Step 0 FACT confirms helper availability. If absent, fall back to per-island bounding boxes.

## Context Cost Estimate

- Aggregate: `M`.
- Largest single step: `M` (Step 3: implementation body).
- Highest-risk dispatch: Step 0 insertion-order FACT (gates Step 0a vs Step 3 directly).

## Open Questions (resolved or punted to Step 0)

- ✅ Stage choice → `PostPass::LayerFinalization` (user-confirmed).
- ✅ Defaults → Orca-aligned `0.1 / 0.10 / 20.0` (user-confirmed).
- ✅ Task ID → `TASK-169` (user-confirmed).
- 🔍 Exact kebab-case `[ir-access].writes` path string for ironing — Step 0 FACT.
- 🔍 `FinalizationOutputBuilder` insertion order (prepend vs append) — Step 0 FACT.
- 🔍 `slicer-helpers` polygon-union availability — Step 0 FACT.
- 🔍 `manifest_ingestion_tdd::core_modules_all_have_placeholder_wasm_flag_set` — convention or fixture bug — Step 0 FACT.
- 🔍 `claim_transition_matrix_tdd` regression — fixture-driven or substantive — Step 0 SUMMARY.

The five 🔍 questions are pre-implementation discovery, not packet activation blockers — they are answerable by sub-agents reading code that already exists. The packet remains `draft` so the user can read it before a fresh implementer agent runs Step 0.
