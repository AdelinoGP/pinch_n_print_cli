# Design: 94_host-mesh-segmentation-wiring

## Controlling Code Paths

- Primary code paths: `crates/slicer-runtime/src/blackboard.rs` (revert `replace_mesh`), `crates/slicer-runtime/src/builtins/mesh_segmentation_producer.rs` (DELETE entire file), `crates/slicer-runtime/src/builtins/mod.rs` (drop `mod` declaration), `crates/slicer-runtime/src/prepass.rs` (revert driver insertion + table entry + error variant), `crates/slicer-core/src/algos/mesh_segmentation.rs` (DELETE entire file), `crates/slicer-core/src/algos/mod.rs` (drop `mod` declaration), `crates/slicer-core/tests/algo_mesh_segmentation_tdd.rs` (DELETE entire file), six P94-introduced contract / integration / executor test files (DELETE), test-harness `main.rs` files (drop `mod` lines).
- Neighboring tests or fixtures: `cube_4color.3mf` and `cube_fuzzyPainted.3mf` survive untouched as P95's input fixtures. The cherry-pick 5c272ef RED tests in `cube_4color_paint_tdd.rs` and `cube_fuzzy_painted_tdd.rs` stay RED awaiting P95's paint-segmentation port. The loader's `split_triangle_strokes` path at `crates/slicer-model-io/src/loader.rs:1900-1961` is the canonical TriangleSelector normalization site post-P94 — verify intact via AC-N3 (do not edit).
- OrcaSlicer comparison surface: see `requirements.md` §OrcaSlicer Reference Obligations. The TASK-250 investigation already encoded the parity findings; this packet executes the architectural verdict.

## Architecture Constraints

- The retirement is purely subtractive. No new code lands; only deletions. The post-packet workspace builds, passes clippy + tests, and the wedge SHA matches the recorded P93 baseline.
- The loader's `split_triangle_strokes` + `walk_triangle_selector_strokes` (loader.rs:1900-1961) is the canonical TriangleSelector normalization site. Do not touch it. P95 will consume `PaintLayer.strokes` directly per the parity doc's Phase 3 `collect_facets()` design.
- The cube_4color SHA captured in Step 7 (`P94R_POST_CUBE_SHA`) becomes P95's input baseline — P95 will compare against this for its byte-identical regression contract.
- The WASM `mesh-segmentation` core-module manifest stays disabled (`.toml.disabled` from the original P94 work). P97 handles the full directory deletion; do NOT re-activate the manifest or alter the disabled state.

## Code Change Surface

- Selected approach: surgical deletion in dependency order — kernel + tests first (smallest blast radius), then producer constant + `mod` declaration, then prepass driver insertion + `required_slots` entry + error variant, then `Blackboard::replace_mesh` + doc-comment, then the six P94-introduced test files + their `mod` declarations. Final step: `docs/07` TASK-244 row update + `.ralph/specs/94/closure-log.md` (NEW) + roadmap §P2 supersession appendix.
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - **`crates/slicer-core/src/algos/mesh_segmentation.rs`** — DELETE entire file (the kernel that 12+ `TangentToFacetEdge` raise sites prove is structurally incompatible with OrcaSlicer-pattern leaves).
  - **`crates/slicer-core/tests/algo_mesh_segmentation_tdd.rs`** — DELETE entire file (kernel unit tests).
  - **`crates/slicer-core/src/algos/mod.rs`** — drop the `pub mod mesh_segmentation;` line.
  - **`crates/slicer-runtime/src/builtins/mesh_segmentation_producer.rs`** — DELETE entire file (host built-in producer constant + cache slots).
  - **`crates/slicer-runtime/src/builtins/mod.rs`** — drop the `pub mod mesh_segmentation_producer;` line.
  - **`crates/slicer-runtime/src/blackboard.rs`** — DELETE the `pub fn replace_mesh(&mut self, new_mesh: Arc<MeshIR>) -> Result<(), BlackboardError>` method + its doc-comment block (returns to the pre-P94 shape with `replace_slice_ir` as the sole `replace_*` method).
  - **`crates/slicer-runtime/src/prepass.rs`** — revert (a) the `run_builtin_stage(..., "PrePass::MeshSegmentation", "host:mesh_segmentation", ...)` invocation block in `execute_prepass_for_object` (or equivalent driver entry); (b) the `"PrePass::MeshSegmentation" => &[]` entry in the `required_slots` table; (c) the `MeshSegmentation { source: MeshSegmentationError }` variant on `PrepassExecutionError`; (d) the `has_subfacet_strokes` helper if it was added in P94 and has no other caller (verify via grep at Step 4).
  - **Six P94-introduced test files** — DELETE entirely:
    - `crates/slicer-runtime/tests/contract/blackboard_replace_mesh_tdd.rs`
    - `crates/slicer-runtime/tests/contract/prepass_execution_error_mesh_segmentation_variant_tdd.rs`
    - `crates/slicer-runtime/tests/executor/cube_4color_mesh_segmentation_strokes_consumed_tdd.rs`
    - `crates/slicer-runtime/tests/executor/cube_fuzzy_painted_mesh_segmentation_strokes_consumed_tdd.rs`
    - `crates/slicer-runtime/tests/executor/mesh_segmentation_determinism_tdd.rs`
    - `crates/slicer-runtime/tests/executor/mesh_segmentation_short_circuit_no_strokes_tdd.rs`
  - **`crates/slicer-runtime/tests/contract/main.rs`** — drop the matching `mod` declarations.
  - **`crates/slicer-runtime/tests/executor/main.rs`** — drop the matching `mod` declarations.
  - **`docs/07_implementation_status.md`** — update the TASK-244 row to reflect retirement supersession (delegated edit).
  - **`docs/specs/paint-pipeline-orca-parity-roadmap.md`** — append a one-paragraph TASK-250 supersession note at the end of §P2.
  - **`.ralph/specs/94_host-mesh-segmentation-wiring/closure-log.md`** (NEW) — captures `P93_BASELINE_SHA`, `P94R_POST_CUBE_SHA`, and a rationale paragraph documenting the TASK-250 architectural verdict.
- Rejected alternatives that were considered and why they were not chosen:
  - **Option B from TASK-250 (full IR flatten)**: requires subdivision-depth inference + unpainted-leaf reconstruction + multi-layer `facet_values` co-update. Makes the IR more "flat-mesh" than OrcaSlicer's (which keeps the bitstream + transient flat-list); diverges from parity in the wrong direction. Rejected.
  - **Option C from TASK-250 (narrow contract — bisectable strokes only)**: keeps the kernel + adds a categorization step. The data-model fork (facet_values + strokes) becomes permanent, every downstream stage handles both representations, and the classification rule (what counts as "cleanly bisectable") is ambiguous. Rejected.
  - **Keep `execute_mesh_segmentation` as a free function under `slicer-helpers` for future use**: dead-code-rot anti-pattern. The codebase already has discipline against this (compare P88's overhang classifier: when needed, ported into a guest module; when dead, deleted). If a future consumer materializes with a precise contract, write a fit-for-purpose kernel then. Rejected.
  - **Move the WASM module deletion into this packet** (absorb P97): P97 has its own scope (97 files), its own AC matrix, and its own coordination with the WIT/dispatch surface. Don't bundle. Rejected.

## Files in Scope (read + edit)

- `crates/slicer-runtime/src/blackboard.rs` — role: revert `replace_mesh`; expected change: delete the method + doc-comment block.
- `crates/slicer-runtime/src/prepass.rs` — role: revert driver insertion + table entry + error variant; expected change: four narrow blocks deleted.
- `crates/slicer-runtime/src/builtins/mod.rs` — role: drop `mod` declaration; one-line change.
- `crates/slicer-core/src/algos/mod.rs` — role: drop `mod` declaration; one-line change.
- `crates/slicer-runtime/tests/contract/main.rs`, `crates/slicer-runtime/tests/executor/main.rs` — role: drop `mod` declarations; one-line changes each.
- `crates/slicer-runtime/src/builtins/mesh_segmentation_producer.rs` — DELETE entire file.
- `crates/slicer-core/src/algos/mesh_segmentation.rs` — DELETE entire file.
- `crates/slicer-core/tests/algo_mesh_segmentation_tdd.rs` — DELETE entire file.
- Six P94-introduced test files under `crates/slicer-runtime/tests/{contract,executor}/` — DELETE entirely (listed above).
- `docs/07_implementation_status.md` — role: TASK-244 row update; delegated edit.
- `docs/specs/paint-pipeline-orca-parity-roadmap.md` — role: append §P2 supersession paragraph.
- `.ralph/specs/94_host-mesh-segmentation-wiring/closure-log.md` (NEW) — role: archival record.

Total: ≤ 8 primary edit targets per step. Multi-commit-safe: kernel + producer + prepass + tests can be deleted in independent commits, or batched into one. Implementer's choice; the per-step plan in `implementation-plan.md` orders them by dependency.

## Read-Only Context

- `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P2" — historical roadmap intent (NOT the current direction; informational only).
- `docs/specs/orca-paint-segmentation-parity.md` §Phase 3 (lines 140-141) — `collect_facets()` design that locks in P95's input contract.
- `crates/slicer-model-io/src/loader.rs:1900-1961` — read only if AC-N3 verification requires confirming the path is intact.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — delegate (no parity check expected for this packet; TASK-250 already established the surface).
- `target/`, `Cargo.lock`, generated code — never load.
- `modules/core-modules/mesh-segmentation/**` — P97 territory; this packet does NOT touch the WASM-guest module directory or its disabled manifest.
- `crates/slicer-model-io/src/loader.rs:1900-1961` (the loader's stroke path) — read-only via AC-N3 only; never edit. This is the canonical normalization site post-P94.
- `crates/slicer-runtime/src/wasm_host.rs`, `dispatch.rs` — P97 territory.
- Cherry-pick 5c272ef tests at `crates/slicer-runtime/tests/executor/cube_4color_paint_tdd.rs` and `cube_fuzzy_painted_tdd.rs` — RED tests awaiting P95; do NOT modify.

## Expected Sub-Agent Dispatches

- "Run `rg -nE 'execute_mesh_segmentation\|MESH_SEGMENTATION_PRODUCER\|MeshSegmentationError\|host:mesh_segmentation\|PrePass::MeshSegmentation\|replace_mesh\|has_subfacet_strokes' crates/ modules/ docs/`; return LOCATIONS (≤ 40 entries) per-file count summary" — purpose: pre-deletion inventory.
- "Run `cargo check --workspace --all-targets`; return FACT pass/fail with first error" — purpose: per-step gate.
- "Run `cargo test --workspace 2>&1 | tee target/test-output.log | grep '^test result' | head -50`; return FACT per-bucket counts" — purpose: AC-5 final gate.
- "Run `mkdir -p target && cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output target/p94-wedge.gcode && sha256sum target/p94-wedge.gcode | awk '{print $1}'`; return FACT (sha256)" — purpose: AC-6 baseline compare.
- "Run `mkdir -p target && cargo run --bin pnp_cli --release -- slice --model resources/cube_4color.3mf --module-dir modules/core-modules --output target/p94-cube.gcode && test -s target/p94-cube.gcode && sha256sum target/p94-cube.gcode | awk '{print $1}'`; return FACT (sha256)" — purpose: AC-7 cube SHA capture.
- "Run `cargo xtask build-guests --check`; return FACT pass/fail" — purpose: AC-8.
- "Locate the TASK-244 row in `docs/07_implementation_status.md` and rewrite it to: `[x] TASK-244 — Retired by TASK-250 architectural finding. The PrePass::MeshSegmentation host stage was wired but the kernel structurally fails on OrcaSlicer-pattern subdivision leaves; the loader's split_triangle_strokes is the canonical normalization site. Closed YYYY-MM-DD — packet 94.`; return FACT pass/fail" — purpose: AC-9 delegated edit.

## Data and Contract Notes

- IR contracts touched: none. The deleted kernel was internal to `slicer-core`; the deleted producer constant was internal to `slicer-runtime`. No public IR shape changes.
- WIT boundary considerations: none. The WASM-guest `mesh-segmentation-output` resource still exists (P97 deletes it); this packet does not touch WIT.
- Determinism or scheduler constraints: the wedge byte-identical contract (AC-6) confirms the deletion is purely subtractive. The cube_4color SHA (AC-7) captures the new baseline for P95.

## Locked Assumptions and Invariants

- **Wedge byte-identical contract**: `P93_BASELINE_SHA` (captured at Step 0) equals the post-P94r wedge SHA (computed in AC-6's verification command). Any drift is investigated, not waved off — the wedge has no painted strokes, so the deleted kernel was a no-op for it.
- **The loader is the normalization site**: `split_triangle_strokes` + `walk_triangle_selector_strokes` at `loader.rs:1900-1961` reproduce OrcaSlicer's TriangleSelector recursive subdivision and emit `PaintLayer.strokes` in OrcaSlicer's flat-leaf form. P95 consumes this directly. NO host-side post-load normalization stage exists post-packet.
- **`mesh-segmentation.toml.disabled` stays**: the rename from the original P94 work persists. P97 deletes the full module directory.
- **No `Blackboard::replace_*` method survives except `replace_slice_ir`**: the precedent the original P94 mirrored stays in place; the mirror is removed.

## Risks and Tradeoffs

- **Risk: a downstream consumer references the deleted kernel that the pre-deletion grep missed.** Mitigation: AC-N1 sweeps the workspace post-deletion; `cargo check --workspace --all-targets` is the cheapest compile-time falsifier; AC-5's workspace test gate catches runtime consumers.
- **Risk: cube_4color slicing still fails for a different reason (a downstream stage that the kernel was inadvertently masking).** Mitigation: AC-7 asserts the slice completes to a non-empty g-code. If the slice fails post-deletion, the failure is informative — surface the new error to the user before completing the packet.
- **Risk: the loader's `split_triangle_strokes` has its own latent bugs that the kernel was masking.** Mitigation: AC-N3 confirms the loader path is intact (no edits in this packet); P95 will exercise the path properly via `collect_facets()`. If a loader bug surfaces later, it gets its own packet.
- **Tradeoff: archival vs. fresh-start closure-log.** The original P94 closure-log (committed under the prior framing) recorded `P94_POST_PAINTED_CUBE_SHA=N/A`. This packet's closure-log records `P93_BASELINE_SHA` (kept from prior) + `P94R_POST_CUBE_SHA` (new). The honest record of the supersession lives in the rationale paragraph appended to the closure-log + the §P2 roadmap addendum.

## Context Cost Estimate

- Aggregate: `S` (deletion is mechanical; the inventory + gate runs dominate).
- Largest single step: `S` (the prepass.rs revert touches four narrow blocks in one file; ≤ 30 line delta).
- Highest-risk dispatch: the pre-deletion LOCATIONS dispatch (must catch every reference so post-deletion sweep is meaningful).

## Open Questions

- `[FWD]` — Does the `has_subfacet_strokes` helper added in P94 have any caller outside the deleted driver block? Step 1 dispatch confirms; if yes, decide whether to keep it (move to `slicer-helpers` as a public utility) or delete. Default: delete unless a caller exists.
- `[FWD]` — Does the workspace contain any test that references `cube_4color.3mf` and asserts on mesh-segmentation-specific output (e.g., `facet_values.len() > N`)? Step 1 grep confirms; if yes, those tests need separate disposition (likely all are the six P94 test files this packet deletes, but verify).
- `[BLOCK]` — None. The TASK-250 architectural verdict is the activation gate; this packet executes it.
