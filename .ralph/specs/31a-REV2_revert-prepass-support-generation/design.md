# Design: 31a-REV2_revert-prepass-support-generation

## Controlling Code Paths

- Primary code paths:
  - `wit/world-prepass.wit` — the WIT export catalogue. The single most consequential change: `export run-support-generation` is removed; `export run-support-geometry` is introduced (or the existing `run-support-geometry` is extended) with the merged signature.
  - `crates/slicer-host/src/prepass.rs` — stage routing, `required_slots()` table, host-built-in invocation, intra-stage ordering between built-in and guest.
  - `crates/slicer-host/src/dispatch.rs` — stage dispatcher; routes stage ids to WIT exports and converts builder outputs into IR commitments.
  - `crates/slicer-host/src/wit_host.rs` — host-side `wit-bindgen` impls; the host stub `HostSupportGenerationOutput` is renamed to `HostSupportGeometryOutput` and extended to absorb support-plan-entry pushes.
  - `crates/slicer-host/src/execution_plan.rs` — `STAGE_ORDER` constant.
  - `crates/slicer-host/src/blackboard.rs` — `BlackboardPrepassSlot` enum and accessor pair already exists for `SupportGeometry` and `SupportPlan` slots; only doc comments and slot-policy attribution change.
  - `crates/slicer-sdk/src/{prelude,traits,prepass_builders}.rs` — `PrepassModule` trait method rename (`run_support_generation` → `run_support_geometry`); SDK builder rename (`SupportGenerationOutput` → `SupportGeometryOutput`).
  - `crates/slicer-schema/src/lib.rs` — StageSpec entry list.
  - `crates/slicer-macros/src/lib.rs` — `#[slicer_module]` macro arms that route stage ids to exports.
  - `modules/core-modules/support-planner/{src/lib.rs,support-planner.toml,Cargo.toml}` — manifest stage id flips; trait impl renames.
- Neighboring tests / fixtures:
  - `crates/slicer-host/tests/prepass_support_generation_tdd.rs` (rename + rewrite).
  - `crates/slicer-host/tests/prepass_support_generation_layer_plan_tdd.rs` (rename + rewrite).
  - `crates/slicer-host/tests/live_support_generation_tdd.rs` (rename + rewrite — note: this is a `Layer::Support` integration test; new name `live_layer_support_tdd.rs`).
  - `crates/slicer-host/tests/benchy_end_to_end_tdd.rs` (single-comment edit, do not load full file).
  - `modules/core-modules/tree-support/tests/enforcer_blocker_tdd.rs` (single-comment edit).
- OrcaSlicer comparison surface: **none**. No parity check required.

## Architecture Constraints

- `PrePass::SupportGeometry` is the only prepass slot for coarse support planning.
- Within the slot, the host built-in always runs first and commits `SupportGeometryIR` before any guest is invoked. This is enforced in `prepass.rs` via the existing built-in invocation path; the guest's `run-support-geometry` then receives `SupportGeometryView` as one of its parameters.
- `SupportPlanIR` survives as a blackboard slot (`BlackboardPrepassSlot::SupportPlan`) but is now produced by guests of `PrePass::SupportGeometry`, not by a separate `PrePass::SupportGeneration` stage.
- All actual support generation (extrusion paths, tree branches, walls) happens in `Layer::Support`. The consumer-side contract for tree-support and traditional-support is unchanged.
- The `31a-REV1` execution-order invariant is preserved: `execute_prepass()` runs before built-in commitment so `LayerPlanIR` is always present when built-ins observe the blackboard. No two-phase `stage_requires_region_map` helper is reintroduced.
- The unit-system invariant (1 unit = 100 nm) and Z-axis convention from `docs/08_coordinate_system.md` are unchanged and must be preserved by the implementer if any geometry calculation is touched. The host built-in's plane-triangle intersection logic from `31a` already respects this; do not re-derive it.

## Code Change Surface

- **Selected approach:** Host built-in runs first within `PrePass::SupportGeometry` and commits `SupportGeometryIR`; guest modules of the same stage then run via `export run-support-geometry`. The guest's WIT entrypoint receives `(list<mesh-object-view>, layer-plan-view, region-segmentation-view, support-geometry-view)` and returns a record carrying both the prior geometry-output fields and a `list<support-plan-entry>`. The host commits `SupportPlanIR` from the returned support-plan-entry list to `BlackboardPrepassSlot::SupportPlan`.

- **Rejected alternatives:**
  - *Guest replaces host built-in entirely.* Rejected: the host built-in (`crates/slicer-host/src/support_geometry.rs`) implements coarse polygon production via plane-triangle intersection at support layer boundaries — a coordinate-system-sensitive operation already validated under `31a`. Pushing this into a guest module loses the host's deterministic execution and forces every prepass guest implementer to re-validate plane-triangle intersection against `docs/08_coordinate_system.md`.
  - *Guest is optional; host always runs.* Rejected: would leave `SupportPlanIR` absent in the no-guest case, forcing every `Layer::Support` consumer to handle absence. The user wants a single canonical path for the v1 architecture.
  - *Two separate exports `run-support-geometry` + `run-support-planning`.* Rejected: doubles the WIT surface and re-introduces an intra-stage ordering contract between two guest exports. The merged-signature single-export path keeps the WIT minimal.

- **Exact functions, traits, manifests, tests, or fixtures expected to change:**
  - `wit/world-prepass.wit`: remove `export run-support-generation`; add (or rename to) `export run-support-geometry: func(objects: list<mesh-object-view>, layer-plan: layer-plan-view, region-segmentation: region-segmentation-view, support-geometry: support-geometry-view) -> support-geometry-output;` where `support-geometry-output` is a record `{ support-plan-entries: list<support-plan-entry>, ... }`. The implementer confirms the exact return record shape against the existing `SupportGeometryView`-bearing path (lines 129–140 of `wit/world-prepass.wit` per the discovery scan).
  - `wit/deps/ir-types.wit`: doc-comment normalization at line 179.
  - `crates/slicer-host/src/prepass.rs`: remove the arm at line 36 (`"PrePass::SupportGeneration" => Some("run-support-generation")`); remove the `required_slots` arm at line 390; ensure the host built-in invocation runs before any `PrePass::SupportGeometry` guest; doc comment at line 298 already references `PrePass::SupportGeometry` correctly. **Phase-2 built-in ordering fix (applied during Step 12 fixup):** guard `commit_region_mapping_builtin` on LayerPlan presence; call `commit_support_geometry_builtin` after phase-1 LayerPlanning so `PrePass::SupportGeometry` guests observe SupportGeometryIR. This is the same class of carry-forward as 31a-REV1's LayerPlanIR ordering invariant, extended to RegionMapIR and SupportGeometryIR.
  - `crates/slicer-host/src/dispatch.rs`: remove the routing arm at line 36; rename the comment at lines 1745, 2064; ensure the dispatcher's conversion path produces both `SupportGeometryIR` (from host built-in) and `SupportPlanIR` (from guest entries).
  - `crates/slicer-host/src/execution_plan.rs`: remove `"PrePass::SupportGeneration"` from `STAGE_ORDER` (line 32 from scan).
  - `crates/slicer-host/src/wit_host.rs`: rename `HostSupportGenerationOutput` → `HostSupportGeometryOutput`; extend it to accept `push_support_plan_entry`; rewrite comments at lines 673, 1256, 1552, 1553.
  - `crates/slicer-host/src/blackboard.rs`: doc comment at line 141 (`"Support plan produced by PrePass::SupportGeneration."` → `"Support plan produced by PrePass::SupportGeometry."`).
  - `crates/slicer-host/src/support_geometry.rs`: doc comment at line 5 normalized.
  - `crates/slicer-sdk/src/prelude.rs`: rename `SupportGenerationOutput` re-export.
  - `crates/slicer-sdk/src/traits.rs`: rename `run_support_generation` → `run_support_geometry`; update method signature to receive `support_geometry: SupportGeometryView`; update doc comments at lines 278, 464.
  - `crates/slicer-sdk/src/prepass_builders.rs`: rename builder; doc comments at lines 388, 390 normalized.
  - `crates/slicer-schema/src/lib.rs`: remove the `PrePass::SupportGeneration` `StageSpec` entry; ensure `PrePass::SupportGeometry` `StageSpec` carries the merged signature.
  - `crates/slicer-macros/src/lib.rs`: remove the macro arm at line 1342 ("SupportGeneration stage"); update comment at line 1772; consolidate the dispatch arm under `PrePass::SupportGeometry`.
  - `crates/slicer-host/tests/prepass_support_generation_tdd.rs` → `prepass_support_geometry_tdd.rs` via `git mv`; rewrite test bodies to reference the new stage and the unified entrypoint; preserve the integration intent.
  - `crates/slicer-host/tests/prepass_support_generation_layer_plan_tdd.rs` → `prepass_support_geometry_layer_plan_tdd.rs` via `git mv`; same rewrite.
  - `crates/slicer-host/tests/live_support_generation_tdd.rs` → `live_layer_support_tdd.rs` via `git mv`; this test exercises `Layer::Support`, not the prepass stage, so the new name reflects the actual scope; rewrite ensures it asserts `Layer::Support` consumes `SupportPlanIR` committed by `PrePass::SupportGeometry`.
  - `crates/slicer-host/tests/benchy_end_to_end_tdd.rs`: single comment edit at line 688; do not load full file.
  - `modules/core-modules/support-planner/support-planner.toml`: `id = "PrePass::SupportGeneration"` → `id = "PrePass::SupportGeometry"`; description rewritten.
  - `modules/core-modules/support-planner/Cargo.toml`: description at line 6 normalized.
  - `modules/core-modules/support-planner/src/lib.rs`: doc comment at line 1 normalized; trait impl method renamed to `run_support_geometry`; method signature updated to receive `support_geometry: SupportGeometryView`.
  - `modules/core-modules/tree-support/{tree-support.toml,src/lib.rs,tests/enforcer_blocker_tdd.rs}`: comment normalization only; consumer-side `SupportPlanIR` reads unchanged.
  - `modules/core-modules/traditional-support/src/lib.rs`: comment lines 13, 41 normalized.
  - `modules/core-modules/paint-segmentation/wit-guest/src/lib.rs`: comment line 211 normalized; **inline WIT world block (resource → record + export rename) — discovered during Step 12 fixup; the Step 8 sweep had partial coverage but missed the inline WIT**.
  - `crates/slicer-ir/src/slice_ir.rs`: `SupportPlanIR` and `SupportPlanEntry` retained verbatim; doc comments at lines 163, 165, 617, 786, 805, 1065, 1067 normalized to attribute production to `PrePass::SupportGeometry`.
  - `docs/01`, `docs/02`, `docs/03`, `docs/04`, `docs/05`, `docs/10`: stage I/O table, required-slots table, glossary, scenario traces, `SupportPlanIR` producer attribution.
  - `docs/07_implementation_status.md`: TASK-161 rewritten in place; checkbox stays `[ ]`.
  - `.ralph/specs/{28,30}_*/`: HEAD admonition + body rewrite.
  - `.ralph/specs/{31a,31a-REV1}_*/packet.spec.md`: frontmatter `status: superseded`; HEAD admonition with explicit AC absorption mapping.
  - `.ralph/specs/31b_*/{packet.spec.md,design.md}`: HEAD note about dep rebase + reference normalization.

## Files in Scope (read + edit)

This packet's surface is wider than the typical ≤ 3-file rule, justified because the work is a coordinated revert / rename across the workspace. Per-step the rule is enforced — no individual implementation step touches more than 3 files. The packet-level surface is:

- `wit/world-prepass.wit` — primary WIT contract; remove `run-support-generation`, introduce/extend `run-support-geometry` with merged signature.
- `wit/deps/ir-types.wit` — doc-comment normalization.
- `crates/slicer-host/src/prepass.rs` — stage routing, required-slots, intra-stage ordering.
- `crates/slicer-host/src/dispatch.rs` — stage dispatcher / output conversion.
- `crates/slicer-host/src/execution_plan.rs` — `STAGE_ORDER` constant.
- `crates/slicer-host/src/wit_host.rs` — host-side WIT impls; rename + extend host stubs.
- `crates/slicer-host/src/blackboard.rs` — doc-comment + slot policy attribution.
- `crates/slicer-host/src/support_geometry.rs` — doc-comment.
- `crates/slicer-sdk/src/prelude.rs` — re-export rename.
- `crates/slicer-sdk/src/traits.rs` — `PrepassModule` trait method rename + signature update.
- `crates/slicer-sdk/src/prepass_builders.rs` — builder rename.
- `crates/slicer-schema/src/lib.rs` — StageSpec entry list.
- `crates/slicer-macros/src/lib.rs` — `#[slicer_module]` macro arms.
- `crates/slicer-ir/src/slice_ir.rs` — doc-comment normalization (struct definitions retained).
- `crates/slicer-host/tests/prepass_support_geometry_tdd.rs` — `git mv` from old name + rewrite.
- `crates/slicer-host/tests/prepass_support_geometry_layer_plan_tdd.rs` — `git mv` + rewrite.
- `crates/slicer-host/tests/live_layer_support_tdd.rs` — `git mv` + rewrite.
- `crates/slicer-host/tests/benchy_end_to_end_tdd.rs` — single comment edit; do not load full file.
- `modules/core-modules/support-planner/{src/lib.rs,support-planner.toml,Cargo.toml}` — manifest + trait impl.
- `modules/core-modules/tree-support/{src/lib.rs,tree-support.toml,tests/enforcer_blocker_tdd.rs}` — comment normalization.
- `modules/core-modules/traditional-support/src/lib.rs` — comment normalization.
- `modules/core-modules/paint-segmentation/wit-guest/src/lib.rs` — comment normalization.
- `docs/01_system_architecture.md` — ranged edits at 100–230, 370–410, 525–540.
- `docs/02_ir_schemas.md` — ranged edits at 75–85, 680–700.
- `docs/03_wit_and_manifest.md` — ranged edits at 540–565.
- `docs/04_host_scheduler.md` — ranged edits at 95–110, 660–680, 905–920.
- `docs/05_module_sdk.md` — ranged edits at 130–220.
- `docs/07_implementation_status.md` — TASK-161 line rewrite.
- `docs/10_glossary_and_scenario_traces.md` — ranged edits at 25–35, 125–160.
- `.ralph/specs/28_tree-support-multi-layer-propagation/{packet.spec.md,design.md,task-map.md}` — HEAD admonition + body rewrite.
- `.ralph/specs/30_support-planner-prepass-wit-plumbing/{packet.spec.md,requirements.md,design.md}` — HEAD admonition + body rewrite.
- `.ralph/specs/31a_support-geometry-prepass-and-layer-height/packet.spec.md` — frontmatter flip + HEAD admonition.
- `.ralph/specs/31a-REV1_support-geometry-prepass-and-layer-height/packet.spec.md` — frontmatter flip + HEAD admonition.
- `.ralph/specs/31b_support-planner-algorithmic-parity/{packet.spec.md,design.md}` — HEAD note + reference normalization.

Each individual step in `implementation-plan.md` lists files-to-edit ≤ 3.

## Read-Only Context

- `docs/01_system_architecture.md` — read lines `100–230`, `370–410`, `525–540` only — purpose: confirm Stage I/O table and Cross-Stage Dependency Matrix shape before editing.
- `docs/04_host_scheduler.md` — read lines `95–110`, `660–680`, `905–920` only — purpose: confirm `STAGE_ORDER` and `required_slots()` table layout before editing.
- `docs/02_ir_schemas.md` — read lines `680–700` only — purpose: confirm `SupportPlanIR` producer attribution wording.
- `docs/08_coordinate_system.md` — read only if a coordinate-system question arises while reviewing the host built-in; do not load proactively.
- `docs/05_module_sdk.md` — read lines `130–220` — purpose: confirm trait/method examples that show `run_support_generation`.
- The current contents of `crates/slicer-host/src/support_geometry.rs` — read in full (small file, < 200 lines) once to confirm the host built-in's commit path.
- The current frontmatter of each predecessor packet (`28`, `30`, `31a`, `31a-REV1`, `31b`) — read only the frontmatter and lines flagged in the discovery scan; do not load packet bodies in full.

## Out-of-Bounds Files

The implementer must NOT load these directly:

- `OrcaSlicerDocumented/` — entirely out of bounds. This packet is a revert; no parity check is required. Delegate any rare fact-check via SUMMARY.
- `target/`, `Cargo.lock`, generated `wit-bindgen` output — never load.
- Vendored deps under `vendor/` (if any) — never load.
- `crates/slicer-macros/src/lib.rs` (> 1700 lines) — locate-then-read with ±40 lines around the affected arms. Do not load in full.
- `crates/slicer-host/src/wit_host.rs` (> 1500 lines) — locate-then-read with ±40 lines around lines 673, 1256, 1552, 1553. Do not load in full.
- `crates/slicer-ir/src/slice_ir.rs` (> 1000 lines) — locate-then-read with ±20 lines around each affected line; the comment edits are tiny.
- `crates/slicer-host/tests/benchy_end_to_end_tdd.rs` (> 1000 lines) — `Edit` the single affected comment line directly without loading.
- The full body of any predecessor packet directory (`28`, `30`, `31a`, `31a-REV1`, `31b`) — delegate per-packet edits via per-file dispatches.
- Crates outside the change surface (`slicer-config`, `slicer-rendering`, etc., if present) — do not browse.

## Expected Sub-Agent Dispatches

The implementer is expected to make the following classes of dispatches. The list is not exhaustive but covers the predictable ones.

- **WIT verification:** "Verify `wit/world-prepass.wit` declares `export run-support-geometry` with parameters `list<mesh-object-view>`, `layer-plan-view`, `region-segmentation-view`, `support-geometry-view`. Return FACT pass/fail." — purpose: confirm Step 1.
- **Compile gate per step:** "Run `cargo build --tests --workspace`; return FACT (pass) or SNIPPETS (fail with first compile error file:line + ≤ 20 lines)." — purpose: validate each non-doc step.
- **Test pass gate per step:** "Run `cargo test -p slicer-host --test prepass_support_geometry_tdd`; return FACT pass/fail with on-failure SNIPPETS (failing test name + assertion + ≤ 20 lines)." — purpose: validate Step 7.
- **Workspace zero-hit sweep:** "Run `rg -c \"PrePass::SupportGeneration|run-support-generation|run_support_generation|SupportGenerationOutput\" crates modules wit docs`; return FACT (count or empty)." — purpose: validate the comment / docs / spec sweep at packet completion.
- **Comment sweep per file:** "In `<file>`, replace each occurrence of `PrePass::SupportGeneration` with `PrePass::SupportGeometry` and `run-support-generation` with `run-support-geometry` inside comments only (do not edit code identifiers); return SNIPPETS of the diff." — purpose: keep the implementer from loading large source files.
- **Spec-packet edit per packet:** "In `.ralph/specs/<packet>/<file>`, prepend the HEAD admonition `<text>` and rewrite stage-name references in body. Return SNIPPETS showing the first 30 lines after edit." — purpose: validate cross-packet edits without loading full packet bodies.
- **Authoritative-doc range read:** "From `docs/04_host_scheduler.md` lines 660–680, return SNIPPETS of the `required_slots()` table." — purpose: confirm the table shape before editing.
- **OrcaSlicer fact check:** **none expected**. If the implementer thinks they need one, they have misread the scope.

## Data and Contract Notes

- IR contracts touched:
  - `SupportGeometryIR` — shape preserved verbatim from `31a`. Key: `(global_support_layer_index: u32, object_id, region_id) → Vec<ExPolygon>`. `u32::MAX` sentinel for intermediate model-resolution outline layers.
  - `SupportPlanIR` and `SupportPlanEntry` — preserved verbatim from `28`. The struct definitions in `crates/slicer-ir/src/slice_ir.rs` do not change; only the doc-comment producer attribution is normalized.
- WIT boundary considerations:
  - The merged `run-support-geometry` signature must match between `wit/world-prepass.wit` (host's exported import surface) and the `crates/slicer-host/src/wit_host.rs` host-side bindings, AND between any `wit-guest` modules in `modules/core-modules/*/wit-guest/`. After WIT changes the implementer must run `cargo build --tests` to surface any binding mismatch.
  - The WIT `support-geometry-output` return record must carry `support-plan-entries: list<support-plan-entry>`. The host then commits both `SupportGeometryIR` (from the built-in's prior commit) and `SupportPlanIR` (from the returned entries).
- Determinism / scheduler constraints:
  - Intra-stage ordering: host built-in always before guest within `PrePass::SupportGeometry`. Any ordering between multiple guests of the same stage follows the existing dispatch / claim system rules — unchanged by this packet.
  - The execution-order invariant from `31a-REV1` (LayerPlanIR committed inside `execute_prepass()`) must continue to hold.

## Locked Assumptions and Invariants

- The unit system (1 unit = 100 nm) and Z-axis convention from `docs/08_coordinate_system.md` are unchanged. The implementer must not re-derive plane-triangle intersection logic; the `31a` host built-in is already correct.
- The `Layer::Support` consumer contract is unchanged: tree-support and traditional-support read `SupportPlanIR` via the SDK accessor; this read path is preserved.
- The blackboard slot policy survives: `BlackboardPrepassSlot::SupportGeometry` and `BlackboardPrepassSlot::SupportPlan` both exist; only the producer attribution shifts so `SupportPlanIR` is now produced by guests of `PrePass::SupportGeometry`.
- The `support_layer_height_mm` and `support_top_z_distance_mm` config keys retain their bounds (default 0.0, min 0.05, max 1.0; default 0.0, min 0.0, max 5.0).
- The HEAD admonitions are the only allowed edits inside packets `31a` and `31a-REV1` beyond the frontmatter `status` flip; the body of those packets is preserved verbatim so historical record stays intact.
- Implemented packets (`28`, `30`) keep their `status: implemented` frontmatter; only HEAD admonition + stage-name reference normalization in the body.

## Risks and Tradeoffs

- **Cross-packet edit risk:** modifying packets `28` and `30` (status `implemented`) actively rewrites historical record. Mitigation: the HEAD admonition explicitly states the rewrite is normalization, not contract change; original implementation evidence is preserved in `git log`.
- **Test rename via `git mv`:** loses tracked rename history if Git does not detect the rename heuristically. Mitigation: `git mv` is explicit; subsequent body edits will appear as modifications to the new path.
- **Macro-arm consolidation risk:** `crates/slicer-macros/src/lib.rs` is large and the dispatch arms are sensitive. If the implementer accidentally collapses two distinct arms or changes argument-threading semantics, the failure surface is silent (wrong code generated). Mitigation: per-arm edits, with `cargo build --tests` between each, and a unit test asserting the macro's expansion shape (if one exists; if not, the failing integration test is the safety net).
- **WIT contract churn:** any guest module not rebuilt after the WIT change will silently mismatch at runtime. Mitigation: `./modules/core-modules/build-core-modules.sh` is a packet-level acceptance gate.
- **HostSupportGenerationOutput rename:** if there are any string-based references to the renamed Rust type (e.g., in doc tests, error messages), they must be normalized. Mitigation: `rg "HostSupportGenerationOutput"` zero-hit check after Step 5.

## Context Cost Estimate

- Aggregate (sum across all steps): **M**.
- Largest single step: **M** (Step 4: host stage routing — touches `prepass.rs`, `dispatch.rs`, `execution_plan.rs` in lockstep; or Step 5: `wit_host.rs` + `blackboard.rs` rename and stub extension).
- Highest-risk dispatch: the `cargo test --workspace` gate at Step 12. Mis-shaped return (full test output) would blow budget. Required return format: FACT pass/fail with on-failure SNIPPETS limited to failing test name + assertion + ≤ 20 lines.

## Open Questions

- **None.** All four design questions are resolved (host-first ordering, `run-support-geometry` merged signature, packet 28/30 admonition + body rewrite, TASK-161 rewritten in place). Mechanical decisions confirmed: `git mv` for test renames, `SupportGenerationOutput` → `SupportGeometryOutput` for the SDK builder, `rg` zero-hit checks as the falsifying validation for sweep steps.

If the implementer encounters new ambiguity during execution (e.g., the existing `wit/world-prepass.wit` already has a `run-support-geometry` export with a different signature than this packet specifies), they must stop and surface it rather than silently expanding scope.
