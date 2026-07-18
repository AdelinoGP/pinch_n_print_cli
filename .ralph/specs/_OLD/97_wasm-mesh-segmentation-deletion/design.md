# Design: 97_wasm-mesh-segmentation-deletion

## Controlling Code Paths

- Primary code paths: every file enumerated in `requirements.md` §In Scope. Specifically: `modules/core-modules/mesh-segmentation/`, `crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit`, `crates/slicer-wasm-host/src/host.rs` + `dispatch.rs`, `crates/slicer-macros/src/lib.rs`, `crates/slicer-runtime/src/blackboard.rs` + `prepass.rs`, `crates/slicer-ir/src/slice_ir.rs` + `stage_io.rs`, four tests + the scaffolder + two scheduler-test tables + one bench.
- Neighboring tests or fixtures: the executor test at `crates/slicer-runtime/tests/executor/mesh_segmentation_executor_tdd.rs` was already deleted by P94r (the host stage retirement). The kernel unit test at `crates/slicer-core/tests/algo_mesh_segmentation_tdd.rs` was likewise deleted by P94r along with the kernel itself. P97 has no test-rewire concern; only the WASM-guest scaffolding remains.
- OrcaSlicer comparison surface: none.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test.

- WIT-first invariant: WIT changes land BEFORE host-side handler deletions. The handler references the WIT type; deleting handlers first leaves the WIT type orphaned (still parseable but unused; harmless) but deleting WIT first leaves handlers referencing a non-existent type (compile failure).
- Atomicity invariant: the 97-file blast is one packet, not multiple — intermediate states would either (a) leave the WASM surface declaring a stage the host built-in also claims (DAG validator confusion) or (b) leave handlers referencing missing types.
- Behavior-preservation invariant: the WASM mesh-segmentation surface is dead in the post-P94r world (the host `PrePass::MeshSegmentation` stage was retired by P94r; the WASM guest module's `[stage]` declaration was disabled via `.toml.disabled` rename in the original P94 work and stays that way until P97 deletes the directory). No live producer claims the stage; no live consumer reads the slot. AC-17 byte-identical confirms.
- Allow-list invariant: AC-14's surviving-reference list is the post-packet expected state. Anything outside the list is a missed deletion.

## Code Change Surface

- Selected approach: WIT-first; then host-side handlers; then macro; then runtime Blackboard + prepass; then IR types; then tests + scaffolder + bench; then guest rebuild; then workspace gate. Each step has its own narrow check.
- Exact functions, traits, manifests, tests, or fixtures expected to change: see `requirements.md` §In Scope.
- Rejected alternatives:
  - **Land in pieces across multiple packets**: rejected — intermediate states are not safe.
  - **Keep the WASM surface around as a future-flexibility hedge**: rejected — the user explicitly clarified mesh-segmentation is host-only.

## Files in Scope (read + edit)

Per `requirements.md`. Aggregate: ≤ 30 files of substantive edit + 1 directory delete + 2 test-file deletes + 1 executor-test rewire.

## Read-Only Context

- `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P5a" — file-by-file list with line refs.
- `CLAUDE.md` §"Guest WASM Staleness" — mandatory before any guest-input edit.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — delegate (none expected).
- `target/`, `Cargo.lock`, generated code — never load.
- The deleted module directory — never read; just delete.
- Kernel source at `crates/slicer-core/src/algos/mesh_segmentation.rs` — does NOT exist post-P94r (the kernel was deleted along with the host stage retirement). Confirm via `test ! -f` at Step 1; no read needed.

## Expected Sub-Agent Dispatches

- "Run `rg -nE 'mesh_segmentation|MeshSegmentation|mesh-segmentation-output|run-mesh-segmentation|FacetPaintMark|MeshSegmentationIR|PrepassStageOutput::MeshSegmentation|BlackboardPrepassSlot::MeshSegmentation' crates/ modules/`; return LOCATIONS (cap 100 entries with per-file count summary if truncated)" — purpose: pre-deletion inventory.
- "Run `cargo check --workspace --all-targets`; return FACT pass/fail with first compile error" — purpose: per-step gate.
- "Run `cargo xtask build-guests && cargo xtask build-guests --check`; return FACT pass/fail" — purpose: guest rebuild.
- "Run `cargo test --workspace 2>&1 | tee target/test-output.log | grep '^test result' | head -50`; return FACT per-bucket counts" — purpose: final gate.
- "Run `cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output /tmp/p97-wedge.gcode && sha256sum /tmp/p97-wedge.gcode`; return FACT (sha256)" — purpose: AC-17 wedge.
- "Run the cube_4color slice + sha256sum; return FACT" — purpose: AC-17 cube.
- "Post-deletion: run `rg -n 'mesh_segmentation|MeshSegmentation' crates/ modules/`; return LOCATIONS (cap 100)" — purpose: AC-14 surviving-refs audit. Manual review against the allow-list.

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

## Context Cost Estimate

- Aggregate: `M` (deletion is mechanical but broad).
- Largest single step: `M` (Step 5 — deletion across multiple host-side files).
- Highest-risk dispatch: the pre-deletion LOCATIONS dispatch (Step 1) — must capture every reference so deletion is complete.

## Open Questions

- `[FWD]` — Have line numbers drifted in the deletion sites? Step 1 confirms.
- `[FWD]` — Is there a community module in-tree (outside `modules/core-modules/`) that depends on the deleted surface? Step 1 inventory confirms (expect: no).
- `[BLOCK]` — None.
