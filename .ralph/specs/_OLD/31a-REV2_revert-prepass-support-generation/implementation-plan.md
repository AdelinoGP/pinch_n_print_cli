# Implementation Plan: 31a-REV2_revert-prepass-support-generation

## Execution Rules

- One atomic step at a time.
- Each step maps back to the packet's grouped task IDs (`TASK-161`, `TASK-162`, `TASK-163-foundation`).
- TDD-first ordering is awkward in a revert: the existing tests reference the removed stage and will not compile mid-revert. The plan therefore orders steps by *compile-time dependency*: WIT → SDK → schema/macros → host routing → host WIT impl → manifest → tests. Each step ends with a narrow falsifying check (compile or rg sweep) so breakage is localized.
- Each step honors the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. The fields below are the budget contract for this step.
- No step is rated `L`. If a step would become `L`, split it before proceeding.

## Steps

### Step 1: WIT contracts — remove run-support-generation, define run-support-geometry

- Task IDs:
  - `TASK-161`
  - `TASK-162`
  - `TASK-163-foundation`
- Objective: Replace `export run-support-generation` with the unified `export run-support-geometry: func(objects: list<mesh-object-view>, layer-plan: layer-plan-view, region-segmentation: region-segmentation-view, support-geometry: support-geometry-view) -> support-geometry-output`. Extend `support-geometry-output` to carry `support-plan-entries: list<support-plan-entry>`. Normalize the doc comment in `wit/deps/ir-types.wit` line 179 to attribute `SupportPlanIR` production to `PrePass::SupportGeometry`.
- Precondition: `wit/world-prepass.wit` currently declares `export run-support-generation` (per discovery scan, line 101+). `wit/deps/ir-types.wit` line 179 currently reads `SupportPlanIR committed by PrePass::SupportGeneration`.
- Postcondition: `wit/world-prepass.wit` declares only `run-support-geometry` (no `run-support-generation`); `support-geometry-output` includes the support-plan-entries field; `wit/deps/ir-types.wit` doc comment normalized.
- Files allowed to read:
  - `wit/world-prepass.wit` (full)
  - `wit/deps/ir-types.wit` (full)
- Files allowed to edit (≤ 3):
  - `wit/world-prepass.wit`
  - `wit/deps/ir-types.wit`
- Files explicitly out-of-bounds for this step:
  - All `crates/`, `modules/` source — those steps come later
  - `OrcaSlicerDocumented/` — not relevant
- Expected sub-agent dispatches:
  - "After edit, return SNIPPETS of `wit/world-prepass.wit` lines around `run-support-geometry`; confirm parameter list contains `list<mesh-object-view>`, `layer-plan-view`, `region-segmentation-view`, `support-geometry-view`." — return format: SNIPPETS ≤ 30 lines.
- Context cost: **S**
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` — read lines `540–565` only (or delegate SUMMARY).
- OrcaSlicer refs: none.
- Verification:
  - `! rg -q "run-support-generation" wit/` — dispatch as FACT pass/fail.
  - `rg -q "run-support-geometry" wit/world-prepass.wit && rg -q "support-plan-entries" wit/world-prepass.wit` — dispatch as FACT pass/fail.
- Exit condition: both verification commands return success.

### Step 2: SDK rename — SupportGenerationOutput → SupportGeometryOutput; trait method rename

- Task IDs:
  - `TASK-161`
  - `TASK-162`
- Objective: Rename `SupportGenerationOutput` → `SupportGeometryOutput` in the SDK builder; rename `PrepassModule::run_support_generation` → `run_support_geometry`; update method signature to receive `support_geometry: SupportGeometryView`; normalize doc comments at `traits.rs` lines 278, 464 and `prepass_builders.rs` lines 388, 390.
- Precondition: WIT changes from Step 1 are in place. Workspace does not yet compile (callers in slicer-host still use old names).
- Postcondition: SDK declares `SupportGeometryOutput` and `run_support_geometry`; no remaining `SupportGenerationOutput` or `run_support_generation` symbols in `crates/slicer-sdk/`.
- Files allowed to read:
  - `crates/slicer-sdk/src/prelude.rs` (small)
  - `crates/slicer-sdk/src/traits.rs` (locate-then-read: lines 278, 464 ±40)
  - `crates/slicer-sdk/src/prepass_builders.rs` (locate-then-read: lines 388, 390 ±40)
- Files allowed to edit (≤ 3):
  - `crates/slicer-sdk/src/prelude.rs`
  - `crates/slicer-sdk/src/traits.rs`
  - `crates/slicer-sdk/src/prepass_builders.rs`
- Files explicitly out-of-bounds for this step:
  - All `crates/slicer-host/`, `crates/slicer-schema/`, `crates/slicer-macros/`, `modules/` — handled in later steps
- Expected sub-agent dispatches:
  - "Run `cargo build -p slicer-sdk`; return FACT pass/fail with on-failure SNIPPETS ≤ 20 lines." — purpose: confirm slicer-sdk crate compiles standalone after rename.
- Context cost: **S**
- Authoritative docs:
  - `docs/05_module_sdk.md` — read lines `130–220` only.
- OrcaSlicer refs: none.
- Verification:
  - `! rg -q "SupportGenerationOutput|run_support_generation" crates/slicer-sdk/` — dispatch as FACT pass/fail.
  - `cargo build -p slicer-sdk` — dispatch as FACT pass/fail.
- Exit condition: both verification commands succeed.

### Step 3: Schema + macros — strip SupportGeneration entry; consolidate dispatch arm

- Task IDs:
  - `TASK-161`
- Objective: Remove the `PrePass::SupportGeneration` `StageSpec` entry from `crates/slicer-schema/src/lib.rs`. Remove the macro arm at `crates/slicer-macros/src/lib.rs` line 1342 ("SupportGeneration stage") and update the comment at line 1772; ensure the `PrePass::SupportGeometry` arm carries the merged-signature dispatch (host built-in + guest with support-geometry-view threading).
- Precondition: SDK rename from Step 2 in place.
- Postcondition: schema lists only `PrePass::SupportGeometry` for the consolidated slot; macro generates dispatch only for `PrePass::SupportGeometry`.
- Files allowed to read:
  - `crates/slicer-schema/src/lib.rs` (locate-then-read; affected near line 100+ per scan)
  - `crates/slicer-macros/src/lib.rs` (locate-then-read; lines 1342, 1772 ±40)
- Files allowed to edit (≤ 3):
  - `crates/slicer-schema/src/lib.rs`
  - `crates/slicer-macros/src/lib.rs`
- Files explicitly out-of-bounds for this step:
  - All `crates/slicer-host/`, `modules/` — later steps
- Expected sub-agent dispatches:
  - "Run `cargo build -p slicer-schema -p slicer-macros`; return FACT pass/fail with on-failure SNIPPETS ≤ 20 lines."
- Context cost: **S**
- Authoritative docs: none for this step.
- OrcaSlicer refs: none.
- Verification:
  - `! rg -q "PrePass::SupportGeneration|SupportGeneration stage" crates/slicer-schema/ crates/slicer-macros/` — FACT pass/fail.
  - `cargo build -p slicer-schema -p slicer-macros` — FACT pass/fail.
- Exit condition: both succeed.

### Step 4: Host stage routing — prepass.rs, dispatch.rs, execution_plan.rs

- Task IDs:
  - `TASK-161`
  - `TASK-162`
- Objective: Remove `"PrePass::SupportGeneration" => Some("run-support-generation")` arm at `crates/slicer-host/src/prepass.rs` line 36; remove the `required_slots` arm at line 390. Remove the analogous arm at `crates/slicer-host/src/dispatch.rs` line 36. Remove `"PrePass::SupportGeneration"` from `STAGE_ORDER` at `crates/slicer-host/src/execution_plan.rs` line 32. Confirm the host built-in invocation in `prepass.rs` (around line 298) still runs before any `PrePass::SupportGeometry` guest. Update the comment at `dispatch.rs` line 1745 to reference `PrePass::SupportGeometry`.
- Precondition: SDK rename (Step 2) and schema/macros (Step 3) in place.
- Postcondition: no `PrePass::SupportGeneration` references in the three files; intra-stage ordering preserved (host built-in commits `SupportGeometryIR` before guest invocation).
- Files allowed to read:
  - `crates/slicer-host/src/prepass.rs` — locate-then-read; affected lines 36, 159, 194-195, 298, 390 ±40
  - `crates/slicer-host/src/dispatch.rs` — locate-then-read; lines 36, 1745, 2064 ±40
  - `crates/slicer-host/src/execution_plan.rs` — small file; full read OK
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/src/prepass.rs`
  - `crates/slicer-host/src/dispatch.rs`
  - `crates/slicer-host/src/execution_plan.rs`
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-host/src/wit_host.rs` — Step 5
  - `crates/slicer-host/src/blackboard.rs` — Step 5 (still mostly comment edits)
- Expected sub-agent dispatches:
  - "Run `cargo build -p slicer-host`; return FACT pass/fail with on-failure SNIPPETS ≤ 20 lines." — slicer-host will not yet compile (wit_host.rs still references old types); the dispatched return is expected to be SNIPPETS naming the wit_host.rs blockers, which become Step 5's input.
- Context cost: **M**
- Authoritative docs:
  - `docs/04_host_scheduler.md` — read lines `95–110`, `660–680` only.
- OrcaSlicer refs: none.
- Verification:
  - `! rg -q "PrePass::SupportGeneration" crates/slicer-host/src/{prepass,dispatch,execution_plan}.rs` — FACT pass/fail.
- Exit condition: rg sweep returns 0 hits across the three files; remaining compile errors are isolated to `wit_host.rs` (Step 5 input).

### Step 5: Host WIT impl + blackboard — wit_host.rs rename + extend; blackboard.rs comment

- Task IDs:
  - `TASK-161`
  - `TASK-162`
- Objective: Rename `HostSupportGenerationOutput` → `HostSupportGeometryOutput` in `crates/slicer-host/src/wit_host.rs`; extend it to absorb `push_support_plan_entry` calls (which previously lived on the SupportGeneration host stub). Wire the renamed `run-support-geometry` import. Update comments at `wit_host.rs` lines 673, 1256, 1552, 1553. Update doc comment at `crates/slicer-host/src/blackboard.rs` line 141 (`"Support plan produced by PrePass::SupportGeneration."` → `"Support plan produced by PrePass::SupportGeometry."`). Update doc comment at `crates/slicer-host/src/support_geometry.rs` line 5.
  - Audit `execute_prepass_with_builtins` phase-2 logic so each host built-in is guarded by its preconditions (RegionMap requires LayerPlan; SupportGeometry requires LayerPlan).
- Precondition: Step 4 complete; remaining workspace compile errors localized to `wit_host.rs`.
- Postcondition: `cargo build --workspace` compiles cleanly; no `HostSupportGenerationOutput` symbol remains; blackboard slot policy attribution updated.
- Files allowed to read:
  - `crates/slicer-host/src/wit_host.rs` — locate-then-read; lines 673, 1256, 1552, 1553 ±40
  - `crates/slicer-host/src/blackboard.rs` — locate-then-read; line 141 ±40 (and slot definitions at line 60)
  - `crates/slicer-host/src/support_geometry.rs` — full read (small file)
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/src/wit_host.rs`
  - `crates/slicer-host/src/blackboard.rs`
  - `crates/slicer-host/src/support_geometry.rs`
- Files explicitly out-of-bounds for this step:
  - `modules/` — Step 6
- Expected sub-agent dispatches:
  - "Run `cargo build --workspace`; return FACT pass/fail with on-failure SNIPPETS ≤ 30 lines." — purpose: workspace-wide compile gate after host changes.
- Context cost: **M**
- Authoritative docs: none for this step.
- OrcaSlicer refs: none.
- Verification:
  - `! rg -q "HostSupportGenerationOutput" crates/slicer-host/` — FACT pass/fail.
  - `cargo build --workspace` — FACT pass/fail.
- Exit condition: workspace compiles cleanly.

### Step 6: support-planner module repurpose

- Task IDs:
  - `TASK-161`
  - `TASK-162`
- Objective: Flip `support-planner.toml` `id` from `"PrePass::SupportGeneration"` → `"PrePass::SupportGeometry"`; rewrite `description` field. Update `Cargo.toml` description at line 6. In `src/lib.rs`, normalize doc comment at line 1; rename trait impl method `fn run_support_generation(&self, ...)` → `fn run_support_geometry(&self, ...)`; update method signature to receive `support_geometry: SupportGeometryView` and to push entries into `SupportGeometryOutput` (renamed from `SupportGenerationOutput`).
- Precondition: Steps 1–5 complete; SDK trait now declares `run_support_geometry`.
- Postcondition: support-planner manifest declares `PrePass::SupportGeometry`; module trait impl matches the new SDK trait; module compiles.
- Files allowed to read:
  - `modules/core-modules/support-planner/support-planner.toml` (small)
  - `modules/core-modules/support-planner/Cargo.toml` (small)
  - `modules/core-modules/support-planner/src/lib.rs` (full)
- Files allowed to edit (≤ 3):
  - `modules/core-modules/support-planner/support-planner.toml`
  - `modules/core-modules/support-planner/Cargo.toml`
  - `modules/core-modules/support-planner/src/lib.rs`
- Files explicitly out-of-bounds for this step:
  - All other module crates — Step 8 (comment sweep) handles them.
- Expected sub-agent dispatches:
  - "Run `cargo build -p support_planner_guest --target wasm32-wasip1` (or the project's actual target); return FACT pass/fail." — confirms guest builds against new SDK trait.
- Context cost: **S**
- Authoritative docs: none for this step.
- OrcaSlicer refs: none.
- Verification:
  - `rg -q '^id\s*=\s*"PrePass::SupportGeometry"' modules/core-modules/support-planner/support-planner.toml` — FACT pass/fail.
  - `! rg -q "PrePass::SupportGeneration|run_support_generation|SupportGenerationOutput" modules/core-modules/support-planner/` — FACT pass/fail.
- Exit condition: both succeed.

### Step 7: Tests rename + rewrite

- Task IDs:
  - `TASK-161`
  - `TASK-162`
- Objective: `git mv` test files to preserve history:
  - `crates/slicer-host/tests/prepass_support_generation_tdd.rs` → `prepass_support_geometry_tdd.rs`
  - `crates/slicer-host/tests/prepass_support_generation_layer_plan_tdd.rs` → `prepass_support_geometry_layer_plan_tdd.rs`
  - `crates/slicer-host/tests/live_support_generation_tdd.rs` → `live_layer_support_tdd.rs`
  
  Then rewrite the test bodies to assert against `PrePass::SupportGeometry` and the unified `run-support-geometry` entrypoint. Add (or update) the named tests required by the packet ACs:
  - `host_builtin_runs_before_guest` (in `prepass_support_geometry_tdd.rs`)
  - `tree_support_plan_succeeds_without_layer_planning_stage` (in `prepass_execution_order_tdd.rs` — create or repurpose; this is the carry-forward AC from `31a-REV1`)
  - `tree_support_consumes_support_plan_ir_from_support_geometry_stage` (in `live_layer_support_tdd.rs`)
  - `pre_pass_support_generation_manifest_rejected` (in `manifest_unknown_stage_tdd.rs` — create)
  - `obsolete_run_support_generation_export_rejected` (in `wit_instantiation_tdd.rs` — create)
  - `support_geometry_ir_keys_and_sentinel` (in `crates/slicer-ir`'s `tests/support_geometry_ir_shape_tdd.rs` — create)
  - `support_geometry_slot_roundtrip` (in `crates/slicer-host`'s `tests/blackboard_support_geometry_slot_tdd.rs` — create)
- Precondition: Steps 1–6 complete; workspace compiles cleanly.
- Postcondition: all named tests exist and pass; old test file paths no longer exist.
- Files allowed to read:
  - The three renamed test files in their new paths.
  - `crates/slicer-host/tests/benchy_end_to_end_tdd.rs` line 688 ±5 only (single comment line; `Edit` directly without loading).
- Files allowed to edit (≤ 3 per sub-step):
  - Sub-step 7a: `prepass_support_geometry_tdd.rs`, `prepass_support_geometry_layer_plan_tdd.rs`, `live_layer_support_tdd.rs` (after `git mv`).
  - Sub-step 7b: new test files for negative cases (`manifest_unknown_stage_tdd.rs`, `wit_instantiation_tdd.rs`, `prepass_execution_order_tdd.rs` if not present, `support_geometry_ir_shape_tdd.rs`, `blackboard_support_geometry_slot_tdd.rs`) — added one or two per ≤ 3-file edit window.
  - Sub-step 7c: single-comment edits (`benchy_end_to_end_tdd.rs:688`, `tree-support/tests/enforcer_blocker_tdd.rs:5`).
- Files explicitly out-of-bounds for this step:
  - Source files in `src/` (no implementation changes here; only test bodies).
- Expected sub-agent dispatches:
  - "Run `cargo test --workspace`; return FACT pass/fail with on-failure SNIPPETS (failing test name + assertion + ≤ 20 lines)." — full test gate.
- Context cost: **M**
- Authoritative docs: none for this step.
- OrcaSlicer refs: none.
- Verification:
  - `! ls crates/slicer-host/tests/prepass_support_generation*.rs 2>/dev/null` — old paths gone.
  - `cargo test --workspace` — all tests pass.
- Exit condition: full test suite green; old test paths absent.

### Step 8: Codebase comment sweep — delegated per file

- Task IDs:
  - `TASK-161`
- Objective: Normalize every remaining doc-comment / line-comment / block-comment reference to `PrePass::SupportGeneration`, `run-support-generation`, or stage-level "support generation" prose, across the ~17 source files identified in the discovery scan. Implementer dispatches per-file edits; do NOT load any of these files in full.
- Precondition: Steps 1–7 complete; workspace compiles and tests pass.
- Postcondition: zero comment-level references to the obsolete vocabulary across `crates/`, `modules/`, `wit/`.
- Files allowed to read:
  - None directly. All sweeps are dispatched.
- Files allowed to edit (≤ 3 per dispatched sub-step):
  - The per-file edits dispatched to sub-agents. Each dispatch covers ≤ 3 files.
- Expected sub-agent dispatches (one per file or small group):
  - "In `<file>`, replace each comment-level reference to `PrePass::SupportGeneration` → `PrePass::SupportGeometry` and `run-support-generation` → `run-support-geometry`. Do NOT edit code identifiers or string literals. Return SNIPPETS of the diff." — return format: SNIPPETS ≤ 30 lines per file.
  - Files in scope (one dispatch each):
    - `crates/slicer-ir/src/slice_ir.rs` (lines 786, 805 — primary; lines 163, 165, 617, 1065, 1067 — verify each is stage-level prose, not generic "support generation" descriptive text; if generic descriptive, leave alone).
    - `crates/slicer-sdk/src/prepass_builders.rs` (388, 390).
    - `crates/slicer-sdk/src/traits.rs` (278, 464).
    - `crates/slicer-host/src/dispatch.rs` (1745, 2064 — already partially handled in Step 4; verify zero hits).
    - `crates/slicer-host/src/wit_host.rs` (673, 1256, 1552, 1553 — already partially handled in Step 5; verify zero hits).
    - `crates/slicer-host/src/blackboard.rs` (141 — already handled in Step 5; verify zero hits).
    - `crates/slicer-host/src/support_geometry.rs` (5 — already handled in Step 5; verify zero hits).
    - `crates/slicer-macros/src/lib.rs` (1342, 1772 — already partially handled in Step 3; verify zero hits).
    - `modules/core-modules/support-planner/src/lib.rs` (1 — already handled in Step 6; verify zero hits).
    - `modules/core-modules/tree-support/src/lib.rs` (45) and `tree-support/tests/enforcer_blocker_tdd.rs` (5).
    - `modules/core-modules/traditional-support/src/lib.rs` (13, 41).
    - `modules/core-modules/paint-segmentation/wit-guest/src/lib.rs` (211 — comment normalization). **Note: this file also contains an inline WIT world block that must be edited as a structured WIT change (resource → record conversion, `run-support-geometry` export rename, `support-geometry-output` field) — NOT as a comment edit. The Step 8 sweep has partial coverage; the inline WIT block was completed during Step 12 fixup.**
    - `wit/deps/ir-types.wit` (179 — already handled in Step 1; verify zero hits).
    - `wit/world-prepass.wit` (111, 129 — already handled in Step 1; verify zero hits).
- Context cost: **S** (no direct reads by the implementer).
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification:
  - `! rg -q "PrePass::SupportGeneration|run-support-generation" crates modules wit` — FACT pass/fail.
- Exit condition: rg sweep returns 0 hits across `crates/`, `modules/`, `wit/`.

### Step 9: Docs rewrite — 01, 02, 03, 04, 05, 10

- Task IDs:
  - `TASK-161`
  - `TASK-162`
  - `TASK-163-foundation`
- Objective: Update Stage I/O table, Cross-Stage Dependency Matrix, `STAGE_ORDER`, `required_slots()` table, glossary, and scenario traces so the documents describe `PrePass::SupportGeometry` as the only prepass support slot and attribute `SupportPlanIR` to it as producer. Implementer dispatches per-doc-section edits; do NOT load any of these large docs in full.
- Precondition: Step 8 complete (codebase normalized).
- Postcondition: zero `PrePass::SupportGeneration` references across `docs/01–10`; producer attribution table consistent.
- Files allowed to read:
  - None directly. All ranged reads are dispatched.
- Files allowed to edit (≤ 3 per dispatched sub-step):
  - `docs/01_system_architecture.md` — sub-step 9a (lines 100–230, 370–410, 525–540).
  - `docs/02_ir_schemas.md` — sub-step 9b (lines 75–85, 680–700).
  - `docs/03_wit_and_manifest.md` — sub-step 9c (lines 540–565).
  - `docs/04_host_scheduler.md` — sub-step 9d (lines 95–110, 660–680, 905–920).
  - `docs/05_module_sdk.md` — sub-step 9e (lines 130–220).
  - `docs/10_glossary_and_scenario_traces.md` — sub-step 9f (lines 25–35, 125–160).
- Files explicitly out-of-bounds for this step:
  - `docs/07_implementation_status.md` — Step 10.
  - `docs/06`, `docs/08`, `docs/09`, `docs/11`, `docs/12`, `docs/13`, `docs/14` — not in this packet's scope.
- Expected sub-agent dispatches (one per doc):
  - "From `<doc>` lines `<range>`, return SNIPPETS of any sentence/table-row mentioning `PrePass::SupportGeneration` / `run-support-generation` / `SupportPlanIR producer attribution`. Then apply the replacement: `PrePass::SupportGeneration` → `PrePass::SupportGeometry`, `run-support-generation` → `run-support-geometry`, producer of `SupportPlanIR` → `PrePass::SupportGeometry` (guest-emitted)." — return format: SNIPPETS of post-edit lines.
- Context cost: **M** (six doc files; per-doc dispatches are S each but aggregate to M).
- Authoritative docs (the docs being edited):
  - `docs/01_system_architecture.md`, `docs/02_ir_schemas.md`, `docs/03_wit_and_manifest.md`, `docs/04_host_scheduler.md`, `docs/05_module_sdk.md`, `docs/10_glossary_and_scenario_traces.md`.
- OrcaSlicer refs: none.
- Verification:
  - `! rg -q "PrePass::SupportGeneration|run-support-generation" docs/{01,02,03,04,05,10}_*.md` — FACT pass/fail.
- Exit condition: rg sweep returns 0 hits across the six docs.

### Step 10: docs/07 reconciliation — TASK-161 rewritten in place

- Task IDs:
  - `TASK-161`
  - `TASK-162`
- Objective: Rewrite `docs/07_implementation_status.md` line 98 (TASK-161). New text describes the consolidated outcome: "Establish `SupportPlanIR` and cross-layer support planning produced inside `PrePass::SupportGeometry` (host built-in commits `SupportGeometryIR`; `support-planner` guest emits `SupportPlanIR` from coarse geometry)." Checkbox stays `[ ]` until 31a-REV2 closes it. TASK-162 (line 99) stays `[x]` unchanged. TASK-163 (lines 100, 101) unchanged.
- Precondition: Step 9 complete.
- Postcondition: `docs/07_implementation_status.md` TASK-161 line carries the rewritten text; no `PrePass::SupportGeneration` reference in the file.
- Files allowed to read:
  - `docs/07_implementation_status.md` lines 95–105 only.
- Files allowed to edit (≤ 3):
  - `docs/07_implementation_status.md`
- Files explicitly out-of-bounds for this step:
  - Any other doc.
- Expected sub-agent dispatches:
  - "From `docs/07_implementation_status.md` lines 95–105, return SNIPPETS confirming TASK-161 rewritten with new text and `[ ]` checkbox; TASK-162 still `[x]`." — return format: SNIPPETS ≤ 15 lines.
- Context cost: **S**
- Authoritative docs:
  - `docs/07_implementation_status.md` lines 95–105.
- OrcaSlicer refs: none.
- Verification:
  - `rg -q "TASK-161.*PrePass::SupportGeometry" docs/07_implementation_status.md` — FACT pass/fail.
  - `! rg -q "TASK-161.*PrePass::SupportGeneration" docs/07_implementation_status.md` — FACT pass/fail.
- Exit condition: both succeed.

### Step 11: Spec-packet sweep — 28, 30, 31a, 31a-REV1, 31b

- Task IDs:
  - `TASK-161`
  - `TASK-162`
  - `TASK-163-foundation`
- Objective: Apply cross-packet edits per the cross-packet impact ledger. Implementer dispatches per-packet edits; do NOT load any predecessor packet body in full.
  - **Packet 28** (`status: implemented`, preserved): prepend HEAD admonition `> Superseded by 31a-REV2 — historical contract preserved below; current architectural authority is .ralph/specs/31a-REV2_revert-prepass-support-generation/. Stage-name references in body normalized to PrePass::SupportGeometry / run-support-geometry.` to `packet.spec.md`. Rewrite ~28 stage-name references across `packet.spec.md`, `design.md`, `task-map.md`.
  - **Packet 30** (`status: implemented`, preserved): same admonition pattern. Rewrite ~18 stage-name references across `packet.spec.md`, `requirements.md`, `design.md`.
  - **Packet 31a** (flip `status: superseded`): prepend HEAD admonition `> Superseded by 31a-REV2. Substantive ACs absorbed: SupportGeometryIR shape, BlackboardPrepassSlot::SupportGeometry roundtrip, host built-in coarse polygons, support-geometry-view-entry/support-geometry-view WIT records, support_layer_height_mm and support_top_z_distance_mm config keys. The PrePass::SupportGeneration stage references in this packet do not survive; see 31a-REV2 ACs for the consolidated stage routing. No substantive work lost.` Body preserved verbatim.
  - **Packet 31a-REV1** (flip `status: superseded`): prepend HEAD admonition `> Superseded by 31a-REV2. Substantive ACs absorbed: execute_prepass() runs before built-in commitment; stage_requires_region_map two-phase helper removed; tree-support plan without PrePass::LayerPlanning succeeds. No substantive work lost.` Body preserved verbatim.
  - **Packet 31b** (`status: draft`, preserved): prepend HEAD note `> Dependency rebased onto 31a-REV2 (which superseded 31a/31a-REV1). Stage references normalized to PrePass::SupportGeometry; algorithmic content unchanged.` to `packet.spec.md`. Normalize ~6 references across `packet.spec.md` and `design.md`.
- Precondition: Steps 1–10 complete.
- Postcondition: cross-packet impact ledger satisfied; the only remaining `PrePass::SupportGeneration` / `run-support-generation` strings in the workspace are inside the HEAD admonitions of packets 28, 30, 31a, 31a-REV1.
- Files allowed to read:
  - None directly. All edits dispatched per-packet.
- Files allowed to edit (≤ 3 per dispatched sub-step):
  - Sub-step 11a: `.ralph/specs/28_tree-support-multi-layer-propagation/{packet.spec.md,design.md,task-map.md}`.
  - Sub-step 11b: `.ralph/specs/30_support-planner-prepass-wit-plumbing/{packet.spec.md,requirements.md,design.md}`.
  - Sub-step 11c: `.ralph/specs/31a_*/packet.spec.md` and `.ralph/specs/31a-REV1_*/packet.spec.md` (frontmatter flip + admonition).
  - Sub-step 11d: `.ralph/specs/31b_support-planner-algorithmic-parity/{packet.spec.md,design.md}`.
- Expected sub-agent dispatches (one per packet):
  - "In `.ralph/specs/<packet>/packet.spec.md`, prepend the HEAD admonition `<exact text>` immediately after the YAML frontmatter; normalize stage-name references in the body. Return SNIPPETS of the first 30 lines after edit." — return format: SNIPPETS ≤ 30 lines per file.
- Context cost: **M**
- Authoritative docs: none for this step.
- OrcaSlicer refs: none.
- Verification:
  - `rg -q "^status: superseded" .ralph/specs/31a_*/packet.spec.md` — FACT pass/fail.
  - `rg -q "^status: superseded" .ralph/specs/31a-REV1_*/packet.spec.md` — FACT pass/fail.
  - `rg -q "Superseded by 31a-REV2" .ralph/specs/{28_*,30_*,31a_*,31a-REV1_*}/packet.spec.md` — FACT pass/fail (4 hits).
  - `rg -q "Dependency rebased onto 31a-REV2" .ralph/specs/31b_*/packet.spec.md` — FACT pass/fail.
  - `! rg -q "PrePass::SupportGeneration" .ralph/specs/{28,30,31b}_*/` — FACT pass/fail (the only surviving hits in `31a` and `31a-REV1` are inside the HEAD admonitions, which is acceptable per the AC).
- Exit condition: all rg checks pass.

### Step 12: Backpressure gates + global zero-hit sweep

- Task IDs:
  - `TASK-161`
  - `TASK-162`
  - `TASK-163-foundation`
- Objective: Run all backpressure gates and the global zero-hit rg sweep. Confirm all packet ACs return PASS. Update `docs/07_implementation_status.md` TASK-161 checkbox to `[x]` if and only if all gates are green.
- Precondition: Steps 1–11 complete.
- Postcondition: workspace is in the "31a-REV2 implemented" state. TASK-161 closed.
- Files allowed to read:
  - None directly. All gate runs are dispatched.
- Files allowed to edit (≤ 3):
  - `docs/07_implementation_status.md` (TASK-161 checkbox flip only).
- Expected sub-agent dispatches:
  - "Run `cargo build --workspace`; return FACT pass/fail." 
  - "Run `cargo test --workspace`; return FACT pass/fail with on-failure SNIPPETS (failing test name + assertion + ≤ 20 lines)."
  - "Run `cargo clippy --workspace -- -D warnings`; return FACT pass/fail."
  - "Run `./modules/core-modules/build-core-modules.sh`; return FACT pass/fail."
  - "Run `rg -c \"PrePass::SupportGeneration|run-support-generation|run_support_generation|SupportGenerationOutput\" crates modules wit docs`; return FACT (count, must be 0)."
  - "Run `rg \"PrePass::SupportGeneration|run-support-generation\" .ralph/specs/`; return LOCATIONS — every hit must be inside a HEAD admonition line. Reject if any hit is in body content."
  - "Re-dispatch every pipe-suffixed AC verification command from `packet.spec.md`; return FACT pass/fail per AC."
- Context cost: **S** (the implementer is purely orchestrating dispatched gates).
- Authoritative docs: none for this step.
- OrcaSlicer refs: none.
- Verification:
  - All backpressure gates green.
  - All AC commands green.
- Exit condition: all gates green; TASK-161 flipped to `[x]`; packet ready for `status: implemented`.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | WIT contracts; small files. |
| Step 2 | S | SDK rename; locate-then-read with line-range hints. |
| Step 3 | S | Schema + macros; locate-then-read inside large macros file. |
| Step 4 | M | Host stage routing; three files in lockstep; mid-stream compile is expected to fail with isolated wit_host.rs blockers. |
| Step 5 | M | wit_host.rs rename + extend; large file via locate-then-read. Workspace compiles after this step. |
| Step 6 | S | support-planner module; small. |
| Step 7 | M | Three test renames + several new test files; bodies must assert against new stage. |
| Step 8 | S | Comment sweep; fully delegated per-file; implementer reads nothing directly. |
| Step 9 | M | Six doc files; ranged dispatches per file. |
| Step 10 | S | Single-line rewrite in docs/07. |
| Step 11 | M | Five spec packets; per-packet dispatches. |
| Step 12 | S | Backpressure gates; orchestration only. |

Aggregate: **M** (sum of 4 × M + 8 × S). No single step is `L`. If during execution any step is reclassified to `L`, split it before proceeding.

## Packet Completion Gate

- All steps complete.
- Every step exit condition is met.
- Packet acceptance criteria green (each verification command from `packet.spec.md` dispatched and returned PASS).
- `docs/07_implementation_status.md` TASK-161 transitioned to `[x]` (via worker dispatch — never edited by loading the full backlog into the implementer's context).
- Predecessor packets `31a` and `31a-REV1` carry `status: superseded` with HEAD admonition.
- Implemented predecessor packets `28` and `30` carry HEAD admonition + body normalization.
- Sibling packet `31b` carries HEAD note with dep rebase.
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md`.
- Confirm packet-level verification commands are green:
  - `cargo build --workspace`
  - `cargo test --workspace`
  - `cargo clippy --workspace -- -D warnings`
  - `./modules/core-modules/build-core-modules.sh`
  - `! rg -q "PrePass::SupportGeneration|run-support-generation|run_support_generation|SupportGenerationOutput" crates modules wit docs`
  - `rg "PrePass::SupportGeneration|run-support-generation" .ralph/specs/` — every hit must be a HEAD admonition line.
- Record any remaining packet-local risk explicitly before moving to `status: implemented`.
- Confirm the implementer's peak context usage stayed under 70%; if not, log it as a packet-authoring lesson for future spec-packet-generator runs (likely candidate: the macro file or wit_host.rs read window was wider than necessary).
