---
status: implemented
packet: 97
task_ids: [TASK-247]
---

# 97_wasm-mesh-segmentation-deletion

## Goal

Delete the entire infrastructure for a "guest WASM module can override mesh-segmentation" path — already dead in the post-P94r world (the host `PrePass::MeshSegmentation` stage and its kernel were retired in P94r per the TASK-250 architectural finding; the loader's `split_triangle_strokes` is the canonical TriangleSelector normalization site). This packet completes the cleanup: remove the directory `modules/core-modules/mesh-segmentation/` in full; drop the `mesh-segmentation-output` resource + `run-mesh-segmentation` export from `crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit:46-54`; drop the resource implementation + `mesh_segmentation_marks` field + accessor at `crates/slicer-wasm-host/src/host.rs:3588-3622, 767, 1042-1043`; drop the harvest + dispatch arm at `crates/slicer-wasm-host/src/dispatch.rs:1700-1727, 818, 1906-1908`; drop the macro arm at `crates/slicer-macros/src/lib.rs:452, 1439-1480` (triggers guest rebuild); drop `Blackboard::commit_mesh_segmentation` + `mesh_segmentation()` accessor at `crates/slicer-runtime/src/blackboard.rs:159-172` (no consumer remains — P94r retired `replace_mesh`; no consumer ever materialized for the guest-output `commit_mesh_segmentation` either); drop dispatcher-output handling + `BlackboardPrepassSlot::MeshSegmentation` at `crates/slicer-runtime/src/prepass.rs:280, 656, 730`; drop `FacetPaintMark`, `MeshSegmentationIR`, schema constant at `crates/slicer-ir/src/slice_ir.rs:1053-1086, 238-…`; drop `PrepassStageOutput::MeshSegmentation` at `crates/slicer-ir/src/stage_io.rs:30-31, 262-…`; delete the contract roundtrip test `crates/slicer-runtime/tests/contract/macro_mesh_segmentation_output_roundtrip_tdd.rs` and the integration geometry test `crates/slicer-runtime/tests/integration/macro_mesh_segmentation_geometry_tdd.rs`; delete the dispatch contract arms at `crates/slicer-runtime/tests/contract/dispatch_tdd.rs:282, 4771-5074, 6187-…`; drop the scaffolder template at `crates/pnp-cli/src/module_new.rs:388, 521, 569, 571, 681` + the scaffolder test at `crates/pnp-cli/tests/module_new_tdd.rs:136`; drop the stage entry from canonical-stages tables at `crates/slicer-scheduler/tests/contract/core_module_ir_access_contract_tdd.rs:43, 233` and `crates/slicer-scheduler/tests/integration/manifest_ingestion_tdd.rs:653`; drop the bench entry at `crates/slicer-runtime/benches/wasm_modules.rs:89`; force a guest rebuild (`cargo xtask build-guests` no flag, then `--check`); confirm via final grep that ZERO `MeshSegmentation`-style references survive (the kernel, producer constant, host stage, and unit/executor tests were already deleted in P94r — the only surviving references should be historical narrative under `.ralph/specs/`, `docs/specs/paint-pipeline-orca-parity-roadmap.md` §P2, and `docs/07_implementation_status.md` TASK-244 entry).

## Problem Statement

The pinch_n_print architecture historically supported guest WASM modules overriding the mesh-segmentation stage via a `mesh-segmentation-output` WIT resource. P94 wired the host kernel as `host:mesh_segmentation` because, per the user's directive, mesh-segmentation is a host responsibility (performance + non-interesting modularity surface). With the host built-in claiming the stage, the WASM surface is dead code — but it spans ≈ 97 files across the workspace and tests. Leaving it as-is creates:

- DAG-validator confusion: two producers (one host, one WASM-declared) could both claim the stage, leading to silently-resolved conflicts.
- Maintenance debt: every WIT, macro, dispatch, blackboard, and test surface for mesh-segmentation needs to stay coherent with no consumer.
- Scaffolder rot: `pnp_cli module_new` still offers `PrePass::MeshSegmentation` as a template stage, misleading future module authors.

This packet performs the surgical deletion. The roadmap's P5a section enumerates each deletion site with file path + approximate line range; the packet executes the deletions, verifies no surviving consumer, forces a guest rebuild, and confirms the workspace tests stay green.

**Amended (P94r reconciliation):** the artifacts the original framing called "KEPT" no longer exist. P94r retired the mesh-segmentation stage in full — the host kernel (`crates/slicer-core/src/algos/mesh_segmentation.rs`), the kernel unit test (`algo_mesh_segmentation_tdd.rs`), the host producer (`crates/slicer-runtime/src/builtins/mesh_segmentation_producer.rs`), and the executor test (`mesh_segmentation_executor_tdd.rs`) were ALL deleted by P94r. `crates/slicer-runtime/src/lib.rs` registers no mesh-seg producer, and `builtin_producers_tdd.rs:60` records "P94r removed mesh_segmentation; expected 7 host built-in producers." P97 therefore preserves and rewires NOTHING — it is pure deletion of the residual WASM-guest + IR + SDK + dispatch + scheduler + test surface.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test.

- WIT-first invariant: WIT changes land BEFORE host-side handler deletions. The handler references the WIT type; deleting handlers first leaves the WIT type orphaned (still parseable but unused; harmless) but deleting WIT first leaves handlers referencing a non-existent type (compile failure).
- Atomicity invariant: the 97-file blast is one packet, not multiple — intermediate states would either (a) leave the WASM surface declaring a stage the host built-in also claims (DAG validator confusion) or (b) leave handlers referencing missing types.
- Behavior-preservation invariant: the WASM mesh-segmentation surface is dead in the post-P94r world (the host `PrePass::MeshSegmentation` stage was retired by P94r; the WASM guest module's `[stage]` declaration was disabled via `.toml.disabled` rename in the original P94 work and stays that way until P97 deletes the directory). No live producer claims the stage; no live consumer reads the slot. AC-17 byte-identical confirms.
- Allow-list invariant: AC-14's surviving-reference list is the post-packet expected state. Anything outside the list is a missed deletion.

## Data and Contract Notes

- IR contracts: `MeshSegmentationIR`, `FacetPaintMark`, `PrepassStageOutput::MeshSegmentation` DELETED. Guest WASMs rebuild without the corresponding bindgen output.
- WIT boundary: `mesh-segmentation-output` resource + `run-mesh-segmentation` export removed. Any community module that depended on this surface (none exist in this workspace) would break.
- Determinism: unchanged. The deletion removes a dead code path.

## Locked Assumptions and Invariants

- **No module declares `PrePass::MeshSegmentation` post-P94r**: confirmed by grepping all `modules/core-modules/*/<name>.toml` for the stage name. The mesh-segmentation guest module's manifest was disabled (`.toml.disabled`) in the original P94 work; P94r retired the host stage entirely. No declarer survives.
- **The host built-in is the only mesh-segmentation surface after this packet**: AC-7 + AC-14.
- **Byte-identical g-code on wedge and cube_4color**: AC-17.

## Risks and Tradeoffs

- **Risk: a community module (outside the in-tree core-modules) depends on the deleted WASM surface.** Mitigation: scan the workspace + this packet's documentation. None exists in-tree. External community modules would break; that's the documented breaking change accompanying this packet.
- **Risk: the deletion misses a hidden reference.** Mitigation: AC-14's allow-list check + workspace tests + clippy gate.
- **Risk: line numbers in the roadmap reference list have drifted.** Mitigation: Step 1 dispatch is the authoritative inventory; the roadmap is a starting point.
