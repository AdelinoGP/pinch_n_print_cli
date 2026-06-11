# Design: 94_host-mesh-segmentation-wiring

## Controlling Code Paths

- Primary code paths to delete: `crates/slicer-core/src/algos/mesh_segmentation.rs` (host kernel; 545 lines), `crates/slicer-core/tests/algo_mesh_segmentation_tdd.rs` (kernel unit tests; 79 lines), `crates/slicer-runtime/tests/executor/mesh_segmentation_executor_tdd.rs` (executor-scope direct-call test; 185 lines).
- Primary code paths to edit: `crates/slicer-core/src/algos/mod.rs` (drop `pub mod mesh_segmentation;` on line 4), `crates/slicer-runtime/src/lib.rs:193-195` (drop the 3-line `mesh_segmentation` re-export block; surrounding `mesh_analysis` and `paint_segmentation` re-exports stay), `crates/slicer-runtime/tests/executor/main.rs:28` (drop `mod mesh_segmentation_executor_tdd;`).
- Neighboring tests or fixtures: `cube_4color.3mf` and `cube_fuzzyPainted.3mf` survive untouched as P95's input fixtures. The cherry-pick 5c272ef RED tests in `cube_4color_paint_tdd.rs` and `cube_fuzzy_painted_tdd.rs` stay RED awaiting P95's paint-segmentation port. The loader's `split_triangle_strokes` path at `crates/slicer-model-io/src/loader.rs:1900-1961` is the canonical TriangleSelector normalization site post-P94 — verify intact via AC-N3 (do not edit).
- OrcaSlicer comparison surface: see `requirements.md` §OrcaSlicer Reference Obligations. The TASK-250 investigation already encoded the parity findings; this packet executes the architectural verdict.

## Architecture Constraints

- The retirement is purely subtractive. No new code lands; only deletions + re-export removal. The post-packet workspace builds, passes clippy + tests, and the wedge SHA matches the recorded P93 baseline.
- The host kernel is fully orphaned. The pre-implementation inventory (Step 1) confirmed zero callers in production source: no prepass driver invocation, no blackboard handoff, no `Blackboard::replace_mesh` (does not exist), no `MESH_SEGMENTATION_PRODUCER` constant (does not exist). The kernel is dead code.
- The loader's `split_triangle_strokes` + `walk_triangle_selector_strokes` (loader.rs:1900-1961) is the canonical TriangleSelector normalization site. Do not touch it. P95 will consume `PaintLayer.strokes` directly per the parity doc's Phase 3 `collect_facets()` design.
- The cube_4color SHA captured in Step 6 (`P94R_POST_CUBE_SHA`) becomes P95's input baseline — P95 will compare against this for its byte-identical regression contract.
- The WASM-guest `mesh-segmentation` core-module infrastructure stays untouched: `modules/core-modules/mesh-segmentation/mesh-segmentation.toml` (active), `mesh-segmentation.wasm` artifact, `PrepassStageOutput::MeshSegmentation` in `crates/slicer-runtime/src/prepass.rs:280`, `BlackboardPrepassSlot::MeshSegmentation` + `commit_mesh_segmentation` in `crates/slicer-runtime/src/blackboard.rs:158-174`, `MeshSegmentationIR` in `crates/slicer-ir/src/stage_io.rs:285`. P97 handles the full directory deletion; do NOT touch any of this.

## Code Change Surface

- Selected approach: surgical deletion in dependency order — kernel + its two test files first (the kernel is the only source of the symbols), then the `pub use` re-export in `slicer-runtime/src/lib.rs`, then the `mod` declarations in `mod.rs` and `tests/executor/main.rs`. Doc edits follow.
- Exact files, manifests, tests, or fixtures expected to change:
  - **`crates/slicer-core/src/algos/mesh_segmentation.rs`** — DELETE entire file (host kernel; 545 lines; 12+ `TangentToFacetEdge` raise sites prove structural incompatibility with OrcaSlicer-pattern leaves).
  - **`crates/slicer-core/tests/algo_mesh_segmentation_tdd.rs`** — DELETE entire file (kernel unit tests; 79 lines).
  - **`crates/slicer-core/Cargo.toml`** — drop the `[[test]] name = "algo_mesh_segmentation_tdd"` block (required consequence of the test-file deletion; cargo errors on a missing test target otherwise).
  - **`crates/slicer-runtime/tests/executor/mesh_segmentation_executor_tdd.rs`** — DELETE entire file (executor-scope direct-call test; 185 lines; imports `execute_mesh_segmentation`, `DegenerateStrokeReason`, `MeshSegmentationError` from `slicer_runtime`).
  - **`crates/slicer-core/src/algos/mod.rs`** — drop the `pub mod mesh_segmentation;` line (line 4).
  - **`crates/slicer-runtime/src/lib.rs`** — DELETE the 3-line `pub use slicer_core::algos::mesh_segmentation::{...}` block at lines 193-195. Keep the `mesh_analysis` re-export (lines 190-192) and the `paint_segmentation` re-export (lines 196-198) untouched.
  - **`crates/slicer-runtime/tests/executor/main.rs`** — drop the `mod mesh_segmentation_executor_tdd;` line (line 28).
  - **`docs/07_implementation_status.md`** — update the TASK-244 row to reflect retirement (delegated edit).
  - **`docs/specs/paint-pipeline-orca-parity-roadmap.md`** — append a one-paragraph TASK-250 supersession note at the end of §P2.
  - **`.ralph/specs/94_host-mesh-segmentation-wiring/closure-log.md`** (already created in Step 0) — append `P94R_POST_CUBE_SHA=<hex>` in Step 6.
- Rejected alternatives that were considered and why they were not chosen:
  - **Keep `execute_mesh_segmentation` as a free function under `slicer-helpers` for future use**: dead-code-rot anti-pattern. The codebase has discipline against this (compare P88's overhang classifier: when needed, ported into a guest module; when dead, deleted). If a future consumer materializes with a precise contract, write a fit-for-purpose kernel then. Rejected.
  - **Move the WASM module deletion into this packet** (absorb P97): P97 has its own scope (97 files), its own AC matrix, and its own coordination with the WIT/dispatch surface. Don't bundle. Rejected.
  - **Wire the kernel into a new prepass stage before deleting**: the TASK-250 finding established that the kernel is structurally incompatible with OrcaSlicer-pattern leaves. Wiring it up just to delete it later is wasted work. Rejected.
  - **Delete the WASM-guest infrastructure alongside the host kernel**: out of scope; P97 owns the WASM deletion with its own AC matrix. Rejected.

## Files in Scope (read + edit)

- `crates/slicer-core/src/algos/mesh_segmentation.rs` — role: DELETE entire file (the kernel). Never load — context waste.
- `crates/slicer-core/tests/algo_mesh_segmentation_tdd.rs` — role: DELETE entire file. Never load.
- `crates/slicer-core/Cargo.toml` — role: drop the `[[test]] name = "algo_mesh_segmentation_tdd"` block (4 lines); required consequence of the test-file deletion. File is small; read in full OK.
- `crates/slicer-runtime/tests/executor/mesh_segmentation_executor_tdd.rs` — role: DELETE entire file. Never load.
- `crates/slicer-core/src/algos/mod.rs` — role: drop the `mod` declaration (line 4); one-line change. File is 12 lines; read in full OK.
- `crates/slicer-runtime/src/lib.rs` — role: drop the `mesh_segmentation` re-export block (lines 193-195); 3-line change. Range-read at lines 185-200 only.
- `crates/slicer-runtime/tests/executor/main.rs` — role: drop the `mod` declaration (line 28); one-line change. File is 40 lines; read in full OK.
- `docs/07_implementation_status.md` — role: TASK-244 row update; delegated edit.
- `docs/specs/paint-pipeline-orca-parity-roadmap.md` — role: append §P2 supersession paragraph.
- `.ralph/specs/94_host-mesh-segmentation-wiring/closure-log.md` (NEW, already created in Step 0) — role: archival record; Step 6 appends `P94R_POST_CUBE_SHA`.

Total: 6 primary production edits (3 file deletions + 3 line drops) + 2 doc edits + 1 closure-log update.

## Read-Only Context

- `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P2" — historical roadmap intent (NOT the current direction; informational only).
- `docs/specs/orca-paint-segmentation-parity.md` §Phase 3 (lines 140-141) — `collect_facets()` design that locks in P95's input contract.
- `crates/slicer-model-io/src/loader.rs:1900-1961` — read only if AC-N3 verification requires confirming the path is intact.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — delegate (no parity check expected for this packet; TASK-250 already established the surface).
- `target/`, `Cargo.lock`, generated code — never load.
- `modules/core-modules/mesh-segmentation/**` — P97 territory; this packet does NOT touch the WASM-guest module directory, the active `mesh-segmentation.toml` manifest, or the `mesh-segmentation.wasm` artifact.
- `crates/slicer-model-io/src/loader.rs:1900-1961` (the loader's stroke path) — read-only via AC-N3 only; never edit. This is the canonical normalization site post-P94.
- `crates/slicer-runtime/src/wasm_host.rs`, `dispatch.rs`, `crates/slicer-wasm-host/**` — P97 territory.
- WASM-guest host infrastructure that P97 owns: `crates/slicer-runtime/src/prepass.rs:280` (`PrepassStageOutput::MeshSegmentation`), `crates/slicer-runtime/src/prepass.rs:656` (`BlackboardPrepassSlot::MeshSegmentation`), `crates/slicer-runtime/src/prepass.rs:730` (`commit_mesh_segmentation` dispatch), `crates/slicer-runtime/src/blackboard.rs:11` (use of `MeshSegmentationIR`), `crates/slicer-runtime/src/blackboard.rs:61` (`mesh_segmentation` field), `crates/slicer-runtime/src/blackboard.rs:158-174` (`commit_mesh_segmentation` + `mesh_segmentation()`), `crates/slicer-ir/src/stage_io.rs:285` (`Self::MeshSegmentation => "mesh-segmentation"`).
- Cherry-pick 5c272ef tests at `crates/slicer-runtime/tests/executor/cube_4color_paint_tdd.rs` and `cube_fuzzy_painted_tdd.rs` — RED tests awaiting P95; do NOT modify.

## Expected Sub-Agent Dispatches

- "Run `rg -nE 'execute_mesh_segmentation|MeshSegmentationError|DegenerateStrokeReason|mesh_segmentation_executor' crates/ modules/ docs/`; return LOCATIONS (≤ 40 entries) per-file count summary" — purpose: pre-deletion inventory. **(DONE in initial run; only the kernel + re-export + 2 test files reference the symbols; no production source uses them.)**
- "Run `cargo check -p slicer-core --all-targets 2>&1 | tee target/test-output.log`; return FACT pass/fail with first error" — purpose: per-step gate after kernel deletion.
- "Run `cargo check -p slicer-runtime --all-targets 2>&1 | tee target/test-output.log`; return FACT pass/fail with first error" — purpose: per-step gate after re-export removal.
- "Run `cargo check --workspace --all-targets 2>&1 | tee target/test-output.log`; return FACT pass/fail" — purpose: workspace gate (AC-3).
- "Run `cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tee target/test-output.log`; return FACT pass/fail" — purpose: lint gate (AC-2).
- "Run `cargo test --workspace 2>&1 | tee target/test-output.log | grep '^test result' | head -50`; return FACT per-bucket counts" — purpose: AC-4 final gate.
- "Run `mkdir -p target && cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output target/p94-wedge.gcode && sha256sum target/p94-wedge.gcode | awk '{print $1}'`; return FACT (sha256)" — purpose: AC-5 baseline compare.
- "Run `mkdir -p target && cargo run --bin pnp_cli --release -- slice --model resources/cube_4color.3mf --module-dir modules/core-modules --output target/p94-cube.gcode && test -s target/p94-cube.gcode && sha256sum target/p94-cube.gcode | awk '{print $1}'`; return FACT (sha256)" — purpose: AC-6 cube SHA capture.
- "Run `cargo xtask build-guests --check`; return FACT pass/fail" — purpose: AC-7.
- "Run the AC-N1 sweep command from `packet.spec.md`; return FACT pass/fail (exit 1 == no matches == pass)" — purpose: surviving-references gate.
- "Run the AC-N2 + AC-N3 verification commands from `packet.spec.md`; return FACT pass/fail" — purpose: negative-case gates.
- "Locate the TASK-244 row in `docs/07_implementation_status.md` and rewrite it to: `[x] TASK-244 — Retired. The execute_mesh_segmentation host kernel was orphaned (zero callers in production source per pre-implementation inventory); deleted in packet 94. The loader's split_triangle_strokes (loader.rs:1900-1961) is the canonical TriangleSelector normalization site; P95 will consume PaintLayer.strokes directly. Closed 2026-06-10.`; return FACT pass/fail" — purpose: AC-8 delegated edit.

## Data and Contract Notes

- IR contracts touched: none. The deleted kernel was internal to `slicer-core`; the deleted re-export was an internal `slicer-runtime` re-export (3 lines). No public IR shape changes.
- WIT boundary considerations: none. The WASM-guest `mesh-segmentation-output` resource still exists (P97 deletes it); this packet does not touch WIT.
- Determinism or scheduler constraints: the wedge byte-identical contract (AC-5) confirms the deletion is purely subtractive. The cube_4color SHA (AC-6) captures the new baseline for P95.

## Locked Assumptions and Invariants

- **Wedge byte-identical contract**: `P93_BASELINE_SHA` (captured at Step 0; value = `AA4DA2FAECA139F2C17909051497D6998F71BFB8A2DD9856D286296252EF1E3B`) equals the post-P94 wedge SHA (computed in AC-5's verification command). Any drift is investigated, not waved off — the wedge has no painted strokes, so the deleted kernel was a no-op for it.
- **The host kernel is fully orphaned**: pre-implementation inventory confirmed zero callers in production source. The kernel, its re-export, and its two test files are the only places these symbols exist. Deletion cannot break a production code path.
- **The loader is the normalization site**: `split_triangle_strokes` + `walk_triangle_selector_strokes` at `loader.rs:1900-1961` reproduce OrcaSlicer's TriangleSelector recursive subdivision and emit `PaintLayer.strokes` in OrcaSlicer's flat-leaf form. P95 consumes this directly. NO host-side post-load normalization stage exists post-packet.
- **WASM-guest infrastructure stays**: `mesh-segmentation.toml`, `mesh-segmentation.wasm`, `PrepassStageOutput::MeshSegmentation`, `BlackboardPrepassSlot::MeshSegmentation`, `commit_mesh_segmentation`, `MeshSegmentationIR`, and the `modules/core-modules/mesh-segmentation/` directory are P97's territory. This packet does not touch any of them.

## Risks and Tradeoffs

- **Risk: a downstream consumer references the deleted kernel that the pre-deletion grep missed.** Mitigation: AC-N1 sweeps the workspace post-deletion; `cargo check --workspace --all-targets` is the cheapest compile-time falsifier; AC-4's workspace test gate catches runtime consumers.
- **Risk: the kernel re-export was depended on by an external consumer (downstream `pnp_cli` or test).** Mitigation: pre-implementation inventory confirmed only the two test files use the symbols; AC-N1 sweeps post-deletion. If a downstream user surfaces, they'd be re-adding the symbol to their own tree.
- **Risk: cube_4color slicing still fails for an unrelated reason (a downstream stage that the kernel was inadvertently masking).** Mitigation: AC-6 asserts the slice completes to a non-empty g-code. If the slice fails post-deletion, the failure is informative — surface the new error to the user before completing the packet.
- **Tradeoff: archival vs. fresh-start closure-log.** The closure-log records `P93_BASELINE_SHA` (kept from prior baseline) + `P94R_POST_CUBE_SHA` (new) + the TASK-250 rationale paragraph. The honest record of the supersession lives in the rationale paragraph appended to the closure-log + the §P2 roadmap addendum + the AC-8 docs/07 row update.

## Context Cost Estimate

- Aggregate: `S` (deletion is mechanical; the inventory + gate runs dominate).
- Largest single step: `S` (the kernel deletion touches one 545-line file via DELETE; no read needed; the test files are smaller; the re-export removal is a 3-line edit).
- Highest-risk dispatch: the pre-deletion LOCATIONS dispatch (must catch every reference so post-deletion sweep is meaningful). The initial-run inventory completed cleanly: only the kernel + 2 test files + 1 re-export line reference the symbols.

## Open Questions

- None. The TASK-250 architectural verdict is the activation gate; the pre-implementation inventory confirmed the host path is fully orphaned; this packet executes the retirement.
