# Requirements: 31a-REV2_revert-prepass-support-generation

## Packet Metadata

- Grouped task IDs:
  - `TASK-161` — rewritten in place under this packet (consolidated outcome: `SupportPlanIR` + cross-layer support planning produced inside `PrePass::SupportGeometry`).
  - `TASK-162` — already `[x]`; this packet preserves the underlying work (planner walks real `LayerPlanView` and emits one entry per `(layer, object, region)`) but absorbs it under the `PrePass::SupportGeometry` stage with the renamed export.
  - `TASK-163-foundation` — the still-correct portion of TASK-163 originally scoped under packet `31a` (IR `SupportGeometryIR`, blackboard slot, host built-in, config keys, WIT view records, intermediate model-resolution outline logic). Algorithmic completion of TASK-163 (avoidance/collision cache, radius taper, raft + interface layers, wall-count-aware move scaling, OrcaSlicer config keys) stays in packet `31b` and is explicitly out of scope here.
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M` (no step is `L`; the WIT/host-binding step and the test rename + rewrite step are the largest single steps, both `M`).

## Problem Statement

Packet `31a` and its REV1 amendment introduced two distinct prepass stages — `PrePass::SupportGeneration` (a guest stage producing `SupportPlanIR`) and `PrePass::SupportGeometry` (a host built-in producing `SupportGeometryIR`) — and threaded the planner module against the former. The user has subsequently directed that **all** coarse prepass support planning, including coarse geometry, must collapse into a single prepass slot, and **all** support generation must remain in `Layer::Support`. `PrePass::SupportGeneration` therefore must not exist as a stage.

This leaves the working tree in a partial state: the IR types, blackboard slot, host built-in, config keys, and WIT view records introduced by `31a` are correct and must be carried forward; the execution-order fix from `31a-REV1` is correct and must be carried forward; but the stage routing, WIT export `run-support-generation`, SDK builder `SupportGenerationOutput`, schema entry, macro arm, manifest stage id on `support-planner`, and a long list of doc / spec / comment references are wrong and must be reverted or normalized.

The "supersede" of packets `31a` and `31a-REV1` is a **runnable-lineage signal**, not abandonment: this packet absorbs every still-correct AC from those packets verbatim. A fresh agent running `31a-REV2` alone produces the full intended end state — `SupportGeometryIR`, the blackboard slot, the host built-in, the config keys, the WIT view records, the corrected execution order, **plus** the consolidation that `31a` and `31a-REV1` did not yet contain.

Spec packets `28` and `30` shipped (`status: implemented`) under the now-removed stage name. Their text references a stage that no longer exists, so future readers would be misled. By user directive — explicitly overriding the standing Cross-Packet Mutation Rule — those packets receive a HEAD admonition and a body rewrite of stage-name references so the architectural prose stays coherent. Packet `31b` (`status: draft`, forward-looking algorithmic work) gets a HEAD note rebasing its dependency from `31a` onto `31a-REV2` and a normalization of ~6 stage references; its algorithmic content is untouched.

## In Scope

- Removal of every `PrePass::SupportGeneration` reference across the workspace: stage id, WIT export `run-support-generation`, SDK builder `SupportGenerationOutput`, trait method `run_support_generation`, schema entry, macro arm, host stage routing, `STAGE_ORDER` entry, `required_slots` row, blackboard `SupportPlan` slot policy attribution.
- Introduction of the unified WIT export `run-support-geometry` with merged signature `(list<mesh-object-view>, layer-plan-view, region-segmentation-view, support-geometry-view)` returning `support-geometry-output { geometry-output fields, support-plan-entry list }`.
- Repurpose of `support-planner` core module: manifest `stage.id` flips to `PrePass::SupportGeometry`; trait method renames to `run_support_geometry`; doc comment and crate description rewritten.
- Carry-forward of all `31a` foundation work as ACs of this packet (see "Predecessor AC absorption" below).
- Carry-forward of `31a-REV1` execution-order semantics as ACs of this packet.
- Tests renamed via `git mv` to preserve git history, then bodies rewritten.
- SDK builder rename `SupportGenerationOutput` → `SupportGeometryOutput`.
- Documentation rewrite across `docs/01`, `docs/02`, `docs/03`, `docs/04`, `docs/05`, `docs/10` for stage I/O, required-slots, glossary, scenario traces, and `SupportPlanIR` producer attribution.
- `docs/07_implementation_status.md` reconciliation: TASK-161 rewritten in place; TASK-162 unchanged.
- Cross-packet edits (explicit override of Cross-Packet Mutation Rule):
  - `28`, `30` (status `implemented` preserved): HEAD admonition + body rewrite of stage-name references.
  - `31a`, `31a-REV1` (flip `status: superseded`): HEAD admonition with explicit AC absorption mapping.
  - `31b` (status `draft` preserved): HEAD note about dep rebase + reference normalization.
- Codebase comment sweep across the ~17 source files identified in the discovery scan (`crates/slicer-ir/src/slice_ir.rs`, `crates/slicer-sdk/src/{prepass_builders,traits}.rs`, `crates/slicer-host/tests/{benchy_end_to_end,live_support_generation,prepass_support_generation,prepass_support_generation_layer_plan}_tdd.rs`, `crates/slicer-macros/src/lib.rs`, `crates/slicer-host/src/{dispatch,blackboard,support_geometry,wit_host}.rs`, `modules/core-modules/{tree-support,traditional-support,paint-segmentation,support-planner}/...`, `wit/deps/ir-types.wit`, `wit/world-prepass.wit`).

## Out of Scope

- TASK-163 algorithmic work (avoidance/collision cache, radius taper, raft + interface layers, wall-count-aware move scaling, OrcaSlicer config keys). Stays in packet `31b`.
- Any new algorithmic content. This packet is structurally a revert + consolidation.
- OrcaSlicer parity reads. No new behavior is being ported.
- Any change to `Layer::Support` consumer logic (`tree-support`, `traditional-support`) beyond comment normalization. The consumer-side `SupportPlanIR` read path via the SDK accessor is unchanged.
- GUI wiring of `support_layer_height_mm` / `support_top_z_distance_mm`.
- Per-region layer heights, soluble materials, raft, interface layers (all 31b territory).

## Authoritative Docs

- `docs/01_system_architecture.md` — > 500 lines; range-read lines 100–230, 370–410, 525–540 only. Sections: Tier 1 stage definitions, Stage I/O Contract table, Cross-Stage Dependency Matrix, Claim System.
- `docs/02_ir_schemas.md` — > 700 lines; range-read lines 75–85, 680–700 only. Sections: `MeshIR.ObjectMesh` doc, `SupportPlanIR` definition + producer attribution.
- `docs/03_wit_and_manifest.md` — range-read lines 540–565. Section: WIT prepass export catalogue.
- `docs/04_host_scheduler.md` — range-read lines 95–110, 660–680, 905–920. Sections: `STAGE_ORDER`, `required_slots()` table, lifecycle diagram.
- `docs/05_module_sdk.md` — range-read lines 130–220. Section: `PrepassModule` trait + manifest examples.
- `docs/07_implementation_status.md` — range-read lines 95–105 only (TASK-161 / TASK-162 / TASK-163 rows). Edit TASK-161 in place.
- `docs/10_glossary_and_scenario_traces.md` — range-read lines 25–35, 125–160.

Default rule: delegate any doc > 300 lines for full SUMMARY when a ranged read is not sufficient.

## OrcaSlicer Reference Obligations

None. This packet is a structural revert; no parity reads are required. The implementer must not load anything from `OrcaSlicerDocumented/`.

## Acceptance Summary

### Positive cases

The full pipe-suffixed AC list lives in `packet.spec.md`. Measurable outcomes the implementer must satisfy:

- Workspace contains zero references to `PrePass::SupportGeneration`, `run-support-generation`, `run_support_generation`, or `SupportGenerationOutput` outside the HEAD-admonition lines of packets `28`, `30`, `31a`, `31a-REV1`.
- `wit/world-prepass.wit` declares `export run-support-geometry: func(...)` whose signature includes `list<mesh-object-view>`, `layer-plan-view`, `region-segmentation-view`, and `support-geometry-view`.
- `SupportGeometryIR` keys `(global_support_layer_index: u32, object_id, region_id) → Vec<ExPolygon>` with `u32::MAX` sentinel for intermediate model-resolution outlines.
- `BlackboardPrepassSlot::SupportGeometry` is committed via `commit_support_geometry()` and read via `support_geometry()`.
- Within `PrePass::SupportGeometry`, the host built-in commits `SupportGeometryIR` strictly before the guest's `run-support-geometry` is invoked. The guest receives a non-empty `SupportGeometryView` and emits `SupportPlanIR`.
- `support-planner.toml` declares `stage.id = "PrePass::SupportGeometry"` and is described in language that does not mention `PrePass::SupportGeneration`.
- Both `support-planner.toml` and `tree-support.toml` expose `support_layer_height_mm` (default 0.0, min 0.05, max 1.0) and `support_top_z_distance_mm` (default 0.0, min 0.0, max 5.0).
- Tree-support plans without `PrePass::LayerPlanning` succeed without `LayerPlanIR`-missing errors (execution-order fix carried forward from `31a-REV1`).
- `Layer::Support` consumes `SupportPlanIR` committed by `PrePass::SupportGeometry` and emits matching branch segments.
- `docs/07_implementation_status.md` TASK-161 text reads "SupportPlanIR + cross-layer support planning produced inside `PrePass::SupportGeometry`"; checkbox `[ ]` until this packet closes it.
- Predecessor packet edits applied per the cross-packet ledger below.

### Negative cases

- A module manifest declaring `stage.id = "PrePass::SupportGeneration"` fails to load with `PrepassExecutionError::UnknownStage` (or named variant) referencing the removed stage id.
- A guest module exporting the obsolete `run-support-generation` symbol fails WIT instantiation rather than silently routing to a host stub.
- Workspace `rg` sweep across `crates/`, `modules/`, `wit/`, `docs/` returns zero matches for `PrePass::SupportGeneration|run-support-generation|run_support_generation|SupportGenerationOutput`.

### Predecessor AC absorption (mapping into this packet's ACs)

| Origin packet | Origin AC (substance) | Absorbed into 31a-REV2 AC |
| --- | --- | --- |
| `31a` | `SupportGeometryIR` defined in `slicer-ir`; key `(global_support_layer_index, object_id, region_id) → Vec<ExPolygon>`; `u32::MAX` sentinel for intermediate outlines | AC: "SupportGeometryIR shape and sentinel" |
| `31a` | `BlackboardPrepassSlot::SupportGeometry`, `commit_support_geometry()`, `support_geometry()` accessor | AC: "Blackboard SupportGeometry slot roundtrip" |
| `31a` | Host built-in `crates/slicer-host/src/support_geometry.rs` computes coarse polygons via plane-triangle intersection at support layer boundaries | AC: "Host built-in runs first within PrePass::SupportGeometry" + AC: "SupportGeometryView non-empty when guest invoked" |
| `31a` | WIT `support-geometry-view-entry` / `support-geometry-view` records | AC: "run-support-geometry export carries support-geometry-view parameter" |
| `31a` | `support_layer_height_mm` (default 0.0, min 0.05, max 1.0) and `support_top_z_distance_mm` (default 0.0, min 0.0, max 5.0) on both manifests | AC: "Manifest config keys preserved on both planner and tree-support" |
| `31a-REV1` | `execute_prepass()` runs before built-in commitment so `LayerPlanIR` is always present | AC: "Execution order: LayerPlanIR committed before built-ins observe blackboard" |
| `31a-REV1` | `stage_requires_region_map` two-phase helper removed | Same AC; falsifying check is the absence of that helper in the file. |
| `31a-REV1` | Tree-support plan (no `PrePass::LayerPlanning`) does not error about missing `LayerPlanIR` | AC: "Tree-support plan succeeds without LayerPlanning stage" |

Anything in `31a` or `31a-REV1` referencing `PrePass::SupportGeneration` (e.g., `required_slots("PrePass::SupportGeneration")` returning `[SurfaceClassification, LayerPlan, RegionMap, SupportGeometry]`) is **not** absorbed; it is replaced by the corresponding `PrePass::SupportGeometry` row in the new `required_slots` table.

### Cross-packet impact ledger

| Packet | Status before | Status after | Edit applied |
| --- | --- | --- | --- |
| `28_tree-support-multi-layer-propagation` | implemented | implemented | HEAD admonition + body rewrite of ~28 stage-name references |
| `30_support-planner-prepass-wit-plumbing` | implemented | implemented | HEAD admonition + body rewrite of ~18 stage-name references |
| `31a_support-geometry-prepass-and-layer-height` | draft | superseded | Frontmatter flip + HEAD admonition with explicit AC absorption map |
| `31a-REV1_support-geometry-prepass-and-layer-height` | draft | superseded | Frontmatter flip + HEAD admonition with explicit AC absorption map |
| `31b_support-planner-algorithmic-parity` | draft | draft | HEAD note: "Dependency rebased onto 31a-REV2" + normalization of ~6 stage references; algorithmic content untouched |

## Verification Commands

- `cargo build --workspace` — top-level compile gate.
- `cargo test --workspace` — full test suite.
- `cargo clippy --workspace -- -D warnings` — lint gate (required before close).
- `./modules/core-modules/build-core-modules.sh` — WASM core-module build gate.
- `! rg -q "PrePass::SupportGeneration|run-support-generation|run_support_generation|SupportGenerationOutput" crates modules wit docs` — global zero-hit sweep across non-spec trees.
- `rg "PrePass::SupportGeneration|run-support-generation" .ralph/specs/` — spec-tree sweep; all hits must lie inside HEAD-admonition lines of `28`, `30`, `31a`, `31a-REV1`.
- `cargo test -p slicer-host --test prepass_support_geometry_tdd -- --exact host_builtin_runs_before_guest` — intra-stage ordering check.
- `cargo test -p slicer-host --test live_layer_support_tdd -- --exact tree_support_consumes_support_plan_ir_from_support_geometry_stage` — end-to-end consumer check.

All commands are delegation-friendly: small, parseable output (exit code or named-test pass/fail).

## Step Completion Expectations

For each step in `implementation-plan.md`, the implementer captures:

- Precondition (what state must hold before starting).
- Postcondition (what state must hold after).
- Falsifying check (the cheapest command that would prove the step is wrong).
- Files allowed to read (with line-range hints for files > 300 lines).
- Files allowed to edit (≤ 3).
- Expected sub-agent dispatches (with required return format).
- Step context cost: `S | M` (never `L`; if a step would be `L`, split it).

The full per-step contract is in `implementation-plan.md`.

## Context Discipline Notes

- **Large files in the read-only path that MUST be ranged or delegated:**
  - `docs/01_system_architecture.md` (> 500 lines) — range-read 100–230, 370–410, 525–540.
  - `docs/02_ir_schemas.md` (> 700 lines) — range-read 75–85, 680–700.
  - `docs/04_host_scheduler.md` (> 900 lines) — range-read 95–110, 660–680, 905–920.
  - `crates/slicer-macros/src/lib.rs` (> 1700 lines) — locate-then-read; use `rg` to find the affected arms (lines 1342, 1772 from the scan) and ±40 lines around each.
  - `crates/slicer-host/src/wit_host.rs` (> 1500 lines) — locate-then-read; affected lines 673, 1256, 1552, 1553 from the scan.
  - `crates/slicer-ir/src/slice_ir.rs` (> 1000 lines) — locate-then-read; affected lines 163, 165, 617, 786, 805, 1065, 1067 from the scan.
- **OrcaSlicer trees the implementer must NOT load directly:** all of `OrcaSlicerDocumented/`. This packet is a revert; nothing to port.
- **Likely temptation reads:**
  - Full read of `docs/01_system_architecture.md` ("just to understand the architecture") — forbidden; range-read or SUMMARY only. The architecture is already locked at `PrePass::SupportGeometry` host-first → guest reads.
  - Full read of any predecessor packet (28/30/31a/31a-REV1/31b) — forbidden; delegate per-packet edits.
  - `OrcaSlicerDocumented/` "to confirm tree-support semantics" — forbidden; consumer-side semantics are unchanged by this packet.
  - `crates/slicer-host/tests/benchy_end_to_end_tdd.rs` (> 1000 lines) — only one comment line on line 688 needs editing; `Edit` it directly without loading the file in full.
- **Sub-agent return-format hints for the heaviest dispatches:**
  - Comment sweep per file: dispatch as `LOCATIONS` first to confirm line numbers, then `Edit` directly. Never request the full file contents.
  - Spec-packet sweep per packet: dispatch as `SNIPPETS` (≤ 3 snippets, ≤ 30 lines each) showing only the lines to edit, plus a `FACT` confirming the HEAD admonition is in place after edit.
  - `cargo test --workspace`: dispatch as `FACT` pass/fail with on-failure `SNIPPETS` carrying only the failing test name + assertion + ≤ 20 lines.
  - `rg` sweeps: dispatch as `FACT` (count) when verifying zero hits; as `LOCATIONS` only when verifying that surviving hits are all inside HEAD-admonition lines.
