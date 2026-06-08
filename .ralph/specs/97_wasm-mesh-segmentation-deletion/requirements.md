# Requirements: 97_wasm-mesh-segmentation-deletion

## Packet Metadata

- Grouped task IDs:
  - `TASK-247` — Delete the WASM guest-module surface for mesh-segmentation (97-file blast radius); the host built-in from P94 is the live path.
- Backlog source: `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P5a — WASM mesh-segmentation surface deletion"
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

The pinch_n_print architecture historically supported guest WASM modules overriding the mesh-segmentation stage via a `mesh-segmentation-output` WIT resource. P94 wired the host kernel as `host:mesh_segmentation` because, per the user's directive, mesh-segmentation is a host responsibility (performance + non-interesting modularity surface). With the host built-in claiming the stage, the WASM surface is dead code — but it spans ≈ 97 files across the workspace and tests. Leaving it as-is creates:

- DAG-validator confusion: two producers (one host, one WASM-declared) could both claim the stage, leading to silently-resolved conflicts.
- Maintenance debt: every WIT, macro, dispatch, blackboard, and test surface for mesh-segmentation needs to stay coherent with no consumer.
- Scaffolder rot: `pnp_cli module_new` still offers `PrePass::MeshSegmentation` as a template stage, misleading future module authors.

This packet performs the surgical deletion. The roadmap's P5a section enumerates each deletion site with file path + approximate line range; the packet executes the deletions, verifies no surviving consumer, forces a guest rebuild, and confirms the workspace tests stay green.

The KEPT artifacts are: the host kernel at `crates/slicer-core/src/algos/mesh_segmentation.rs`, the kernel unit test at `crates/slicer-core/tests/algo_mesh_segmentation_tdd.rs`, the host BuiltinProducer constant at `crates/slicer-runtime/src/builtins/mesh_segmentation_producer.rs` (from P94), and the driver wiring at `crates/slicer-runtime/src/prepass.rs` (from P94). The executor test at `crates/slicer-runtime/tests/executor/mesh_segmentation_executor_tdd.rs` is REWIRED to test the host built-in path (no WASM roundtrip).

## In Scope

(Exhaustive deletion list. Line numbers may drift; the implementer's Step 1 dispatch confirms current locations.)

- `modules/core-modules/mesh-segmentation/` — entire directory.
- `crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit:46-54` — `mesh-segmentation-output` resource + `run-mesh-segmentation` export.
- `crates/slicer-wasm-host/src/host.rs:3588-3622, 767, 1042-1043` — resource impl + `mesh_segmentation_marks` field + accessor.
- `crates/slicer-wasm-host/src/dispatch.rs:1700-1727, 818, 1906-1908` — harvest + dispatch arm.
- `crates/slicer-macros/src/lib.rs:452, 1439-1480` — macro arm (forces guest rebuild on edit).
- `crates/slicer-runtime/src/blackboard.rs:159-172` — `commit_mesh_segmentation` + `mesh_segmentation()` accessor.
- `crates/slicer-runtime/src/prepass.rs:280, 656, 730` — dispatcher-output handling + `BlackboardPrepassSlot::MeshSegmentation`.
- `crates/slicer-ir/src/slice_ir.rs:1053-1086, 238-…` — `FacetPaintMark`, `MeshSegmentationIR`, schema constant.
- `crates/slicer-ir/src/stage_io.rs:30-31, 262-…` — `PrepassStageOutput::MeshSegmentation`.
- `crates/slicer-runtime/tests/contract/macro_mesh_segmentation_output_roundtrip_tdd.rs` — DELETE.
- `crates/slicer-runtime/tests/integration/macro_mesh_segmentation_geometry_tdd.rs` — DELETE.
- `crates/slicer-runtime/tests/executor/mesh_segmentation_executor_tdd.rs` — REWIRE to host path (drop WASM roundtrip; keep file).
- `crates/slicer-runtime/tests/contract/dispatch_tdd.rs:282, 4771-5074, 6187-…` — drop dispatch contract arms.
- `crates/pnp-cli/src/module_new.rs:388, 521, 569, 571, 681` — scaffolder template arm.
- `crates/pnp-cli/tests/module_new_tdd.rs:136` — scaffolder test.
- `crates/slicer-scheduler/tests/contract/core_module_ir_access_contract_tdd.rs:43, 233` — canonical-stages drop.
- `crates/slicer-scheduler/tests/integration/manifest_ingestion_tdd.rs:653` — canonical-stages drop.
- `crates/slicer-runtime/benches/wasm_modules.rs:89` — bench entry.

## Out of Scope

- The host kernel + producer + driver wiring (P94 territory; KEPT).
- The kernel unit test at `crates/slicer-core/tests/algo_mesh_segmentation_tdd.rs` (KEPT).
- Paint-segmentation kernel — P3/P4.
- Doc updates to `docs/02`, `docs/03`, `docs/04` — P5c (99) — except where the deletion forces a doc update inline (the host stage_id `host:mesh_segmentation` remains documented).
- Loader symmetry — P5b (98).

## Authoritative Docs

- `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P5a" — primary checklist (≤ 90 lines).
- `CLAUDE.md` §"Guest WASM Staleness" — must read before any path under the guest-WASM input list is edited.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent.

Files to inspect for this packet:

- None. Pure in-house deletion.

## Acceptance Summary

- Positive cases: `AC-1` through `AC-17`. Refinements:
  - The line numbers in the roadmap reference list may have drifted since roadmap-write. Step 1 dispatch confirms current sites.
  - AC-14's surviving-references list is allow-list — the implementer reviews each surviving reference against the list and escalates if any reference doesn't match.
  - AC-N1's producer count check is closure-log evidence, not a machine gate (depends on commit N).
- Negative cases: `AC-N1` (producer count unchanged), `AC-N2` (no WIT consumer).
- Cross-packet impact: completes the architectural cleanup. P5b and P5c are independent.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo check --workspace --all-targets` | Compiles | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | No lint warnings | FACT pass/fail |
| `cargo xtask build-guests && cargo xtask build-guests --check` | AC-5, AC-16 — guest rebuild after macro arm deletion | FACT pass/fail |
| `cargo test --workspace 2>&1 \| tee target/test-output.log` | AC-15 — workspace gate (deletion blast wide; workspace gate required per `CLAUDE.md` §Test Discipline rule 2) | FACT per-bucket counts |
| `! rg -q 'mesh-segmentation-output\|run-mesh-segmentation' crates/ modules/` | AC-2, AC-N2 — WIT/dispatch deletion | FACT pass/fail |
| `cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output /tmp/p97-wedge.gcode && sha256sum /tmp/p97-wedge.gcode` | AC-17 — wedge byte-identical | FACT (sha256) |
| `cargo run --bin pnp_cli --release -- slice --model resources/cube_4color.3mf --module-dir modules/core-modules --output /tmp/p97-cube.gcode && sha256sum /tmp/p97-cube.gcode` | AC-17 — cube byte-identical | FACT (sha256) |
| `rg -n 'mesh_segmentation\|MeshSegmentation' crates/ modules/` | AC-14 — surviving references audit (manual review) | LOCATIONS (≤ 60 entries) |

## Step Completion Expectations

- WIT changes (Step 2) MUST land BEFORE host-side handler deletions (Step 3) — otherwise the host references a non-existent WIT type and the workspace doesn't compile.
- The macro arm deletion (Step 4) triggers a full guest rebuild on the next `cargo xtask build-guests`. The rebuild MUST happen before any test runs that depend on guest behavior.
- The executor-test rewire (Step 9) tests the SAME kernel functionality (sub-facet stroke normalization on a painted cube) but via the host built-in path; the assertion content can be preserved.
- AC-17 byte-identical g-code is the regression contract: the deleted WASM path was dead, so deletion should not change runtime behavior. Any g-code diff is investigated, not waved off.

## Context Discipline Notes

- The 97-file blast radius is the largest test of context discipline. NEVER load any deleted file's body before deletion; just delete via `Bash rm` or `Edit` with `replace_all`. Sub-agent dispatches return file paths only.
- `crates/slicer-runtime/src/prepass.rs` is large (> 700 lines). The deletion sites at lines 280 / 656 / 730 are narrow; range-read each (40-line window).
- `crates/slicer-runtime/tests/contract/dispatch_tdd.rs` lines 282 / 4771-5074 / 6187 — the middle range (4771-5074) is sizeable (~300 LOC of test arms); range-read.
- `crates/slicer-wasm-host/src/host.rs` is large (likely > 5000 LOC); range-read at each deletion site.
- The deleted module directory `modules/core-modules/mesh-segmentation/` — DO NOT read its contents. Just `rm -rf` (or Windows equivalent).
