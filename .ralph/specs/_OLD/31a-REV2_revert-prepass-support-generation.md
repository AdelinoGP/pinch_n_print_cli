---
status: implemented
packet: 31a-REV2_revert-prepass-support-generation
task_ids:
  - TASK-161
  - TASK-162
  - TASK-163-foundation
supersedes:
  - 31a_support-geometry-prepass-and-layer-height
  - 31a-REV1_support-geometry-prepass-and-layer-height
---

# 31a-REV2_revert-prepass-support-generation

## Goal

Revert the prepass-stage notion of `PrePass::SupportGeneration` everywhere it appears — host code, SDK, WIT, schema, manifests, tests, docs, predecessor spec packets (`28`, `30`, `31a`, `31a-REV1`), the in-flight successor spec packet (`31b`), and source-code comments — and consolidate all coarse prepass support planning into a single `PrePass::SupportGeometry` slot. Within that slot the host built-in runs first and commits `SupportGeometryIR`; guest modules then run via the unified WIT export `run-support-geometry` and emit `SupportPlanIR`. All actual support generation (paths, extrusions, tree branches, walls) remains constrained to `Layer::Support`. The substantive foundation work from `31a` (IR shape, blackboard slot, host built-in, config keys, WIT view records) and the execution-order fix from `31a-REV1` are absorbed verbatim as ACs of this packet so a fresh agent running this packet alone produces the full intended end state.

## Problem Statement

Packet `31a` and its REV1 amendment introduced two distinct prepass stages — `PrePass::SupportGeneration` (a guest stage producing `SupportPlanIR`) and `PrePass::SupportGeometry` (a host built-in producing `SupportGeometryIR`) — and threaded the planner module against the former. The user has subsequently directed that **all** coarse prepass support planning, including coarse geometry, must collapse into a single prepass slot, and **all** support generation must remain in `Layer::Support`. `PrePass::SupportGeneration` therefore must not exist as a stage.

This leaves the working tree in a partial state: the IR types, blackboard slot, host built-in, config keys, and WIT view records introduced by `31a` are correct and must be carried forward; the execution-order fix from `31a-REV1` is correct and must be carried forward; but the stage routing, WIT export `run-support-generation`, SDK builder `SupportGenerationOutput`, schema entry, macro arm, manifest stage id on `support-planner`, and a long list of doc / spec / comment references are wrong and must be reverted or normalized.

The "supersede" of packets `31a` and `31a-REV1` is a **runnable-lineage signal**, not abandonment: this packet absorbs every still-correct AC from those packets verbatim. A fresh agent running `31a-REV2` alone produces the full intended end state — `SupportGeometryIR`, the blackboard slot, the host built-in, the config keys, the WIT view records, the corrected execution order, **plus** the consolidation that `31a` and `31a-REV1` did not yet contain.

Spec packets `28` and `30` shipped (`status: implemented`) under the now-removed stage name. Their text references a stage that no longer exists, so future readers would be misled. By user directive — explicitly overriding the standing Cross-Packet Mutation Rule — those packets receive a HEAD admonition and a body rewrite of stage-name references so the architectural prose stays coherent. Packet `31b` (`status: draft`, forward-looking algorithmic work) gets a HEAD note rebasing its dependency from `31a` onto `31a-REV2` and a normalization of ~6 stage references; its algorithmic content is untouched.

## Architecture Constraints

- `PrePass::SupportGeometry` is the only prepass slot for coarse support planning.
- Within the slot, the host built-in always runs first and commits `SupportGeometryIR` before any guest is invoked. This is enforced in `prepass.rs` via the existing built-in invocation path; the guest's `run-support-geometry` then receives `SupportGeometryView` as one of its parameters.
- `SupportPlanIR` survives as a blackboard slot (`BlackboardPrepassSlot::SupportPlan`) but is now produced by guests of `PrePass::SupportGeometry`, not by a separate `PrePass::SupportGeneration` stage.
- All actual support generation (extrusion paths, tree branches, walls) happens in `Layer::Support`. The consumer-side contract for tree-support and traditional-support is unchanged.
- The `31a-REV1` execution-order invariant is preserved: `execute_prepass()` runs before built-in commitment so `LayerPlanIR` is always present when built-ins observe the blackboard. No two-phase `stage_requires_region_map` helper is reintroduced.
- The unit-system invariant (1 unit = 100 nm) and Z-axis convention from `docs/08_coordinate_system.md` are unchanged and must be preserved by the implementer if any geometry calculation is touched. The host built-in's plane-triangle intersection logic from `31a` already respects this; do not re-derive it.

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
