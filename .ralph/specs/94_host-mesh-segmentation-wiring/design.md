# Design: 94_host-mesh-segmentation-wiring

## Controlling Code Paths

- Primary code paths: `crates/slicer-runtime/src/blackboard.rs` (replace_mesh), `crates/slicer-runtime/src/builtins/mesh_segmentation_producer.rs` (NEW, producer constant), `crates/slicer-runtime/src/builtins/mod.rs` (one-line registration), `crates/slicer-runtime/src/prepass.rs` (driver insertion + required_slots table + error variant). The kernel itself at `crates/slicer-core/src/algos/mesh_segmentation.rs:39-109` is **read-only** for this packet.
- Neighboring tests or fixtures: `crates/slicer-runtime/tests/executor/` gains new integration tests; `crates/slicer-runtime/tests/contract/` likely gains a Blackboard `replace_mesh` test.
- OrcaSlicer comparison surface: none directly — kernel parity was established in the kernel's own unit tests.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

- Stage-ordering invariant: `PrePass::MeshSegmentation` runs FIRST in the prepass sequence, before `PrePass::MeshAnalysis`. Reversing the order means mesh-analysis would see un-normalized strokes and produce wrong surface classifications. The required_slots table entry (`PrePass::MeshSegmentation => &[]`) makes this explicit in the DAG validator.
- Blackboard tier invariant: `replace_mesh` is callable ONLY before any Tier 2 output is committed (matches `replace_slice_ir`'s contract). Calling after a `commit_slice_ir` or `commit_layer_plan` is a contract violation surfaced via debug_assert + structured error.
- Short-circuit invariant: `has_subfacet_strokes(mesh)` returns true iff at least one object has `paint_data.layers[*].strokes` non-empty. When false, `execute_mesh_segmentation` is NOT called and `replace_mesh` is NOT called — zero overhead vs. pre-packet.
- WASM mesh-segmentation core-module continues to exist (P5a deletes it). Its `MESH_SEGMENTATION_OUTPUT_STAGE_ID` (or equivalent) is `PrePass::MeshSegmentation`-ish but mismatches the new host stage name by design — no module reaches dispatch for this stage anymore because the host built-in claims it.

## Code Change Surface

- Selected approach: land each piece in dependency order — Blackboard method (consumed by driver), producer constant (consumed by registry), prepass driver (consumes both), error variant, required_slots entry, integration tests.
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - **`crates/slicer-runtime/src/blackboard.rs`** (additive). Field shape is verified: `Blackboard::mesh_ir: Arc<MeshIR>` (non-`Option`). The body mirrors `replace_slice_ir:276-290` exactly, except its assertions target the mesh-swap precondition rather than the slice-IR-swap precondition:
    ```rust
    /// Replace the mesh IR. Symmetric with `replace_slice_ir`: panics in debug
    /// builds if any Tier 2 output has landed.
    pub fn replace_mesh(&mut self, new_mesh: Arc<MeshIR>) -> Result<(), BlackboardError> {
        debug_assert!(
            self.slice_ir.is_none(),
            "replace_mesh called after Tier 2 wrote slice_ir"
        );
        debug_assert!(
            self.layer_outputs
                .as_ref()
                .is_some_and(|v| v.iter().all(Option::is_none)),
            "replace_mesh called after Tier 2 wrote a layer slot"
        );
        self.mesh_ir = new_mesh;
        Ok(())
    }
    ```
    The `Result` return is kept for symmetry; no error path actually fires in this contract (and `BlackboardError` has no `TierViolation` variant — adding one would widen the type and is out of P94 scope). `BlackboardPrepassSlot` has no `MeshIR` variant, so a `MissingRequiredPrepass` guard on the mesh is not expressible and is not needed (the field is non-`Option`).
  - **`crates/slicer-runtime/src/builtins/mesh_segmentation_producer.rs`** (NEW):
    Mirror `mesh_analysis_producer.rs` exactly except for: `id = "host:mesh_segmentation"`, `stage = "PrePass::MeshSegmentation"`, `ir_writes = &["MeshIR"]`. All other fields identical including the seven `OnceLock::new()` cache slots.
  - **`crates/slicer-runtime/src/builtins/mod.rs`** (one-line addition):
    `pub mod mesh_segmentation_producer;` next to `pub mod mesh_analysis_producer;`.
  - **`crates/slicer-runtime/src/prepass.rs`** (driver insertion + table + error variant):
    - Add helper `fn has_subfacet_strokes(mesh: &MeshIR) -> bool { mesh.objects.iter().any(|o| o.paint_data.as_ref().is_some_and(|pd| pd.layers.iter().any(|l| !l.strokes.is_empty()))) }`.
    - Insert before existing `host:mesh_analysis` invocation (around line 374):
      ```rust
      run_builtin_stage(
          blackboard, instrumentation,
          "PrePass::MeshSegmentation", "host:mesh_segmentation",
          |bb| has_subfacet_strokes(bb.mesh()),
          |bb| {
              let normalized = execute_mesh_segmentation(bb.mesh().clone())
                  .map_err(PrepassExecutionError::MeshSegmentation)?;
              bb.replace_mesh(normalized).map_err(|source| PrepassExecutionError::Blackboard {
                  stage_id: "PrePass::MeshSegmentation".to_string(),
                  module_id: "host:mesh_segmentation".to_string(),
                  source,
              })
          },
      )?;
      ```
      (Adjust to whatever the existing `run_builtin_stage` signature actually is; the helper exists per the roadmap reference.)
    - Add `PrepassExecutionError::MeshSegmentation { source: MeshSegmentationError }` to the existing error enum. Add `#[from]` if the enum uses `thiserror`'s `#[error]`/`#[from]` pattern.
    - Add `"PrePass::MeshSegmentation" => &[]` to the `required_slots(StageId)` table at lines 680-708.
  - **`modules/core-modules/mesh-segmentation/mesh-segmentation.toml`** (one-line minimal edit; AC-3.5): disable the manifest's `stage = "PrePass::MeshSegmentation"` claim so the DAG validator sees only the new host built-in as producer for that stage. Mechanism is implementer's choice (comment out the line, rename the manifest to `.disabled`, or use the loader's documented "disabled" pathway). The directory remains in place — P5a (97) still owns its full deletion. This edit triggers a guest rebuild before AC-13's `--check` reports clean (per `CLAUDE.md` §"Guest WASM Staleness").
  - **`crates/slicer-runtime/tests/executor/mesh_segmentation_short_circuit_no_strokes_tdd.rs`** (NEW) — loads `regression_wedge.stl`, runs prepass, asserts no `replace_mesh` invocation.
  - **`crates/slicer-runtime/tests/executor/cube_4color_mesh_segmentation_strokes_consumed_tdd.rs`** (NEW) — loads `cube_4color.3mf`, runs prepass, asserts `strokes.is_empty()` and a deterministic post-normalization facet count.
  - **`crates/slicer-runtime/tests/executor/cube_fuzzyPainted_mesh_segmentation_strokes_consumed_tdd.rs`** (NEW) — same shape for the fuzzy_skin fixture.
  - **`crates/slicer-runtime/tests/executor/mesh_segmentation_determinism_tdd.rs`** (NEW) — runs prepass twice on the same painted mesh, byte-compares the normalized `MeshIR`.
  - **`crates/slicer-runtime/tests/contract/blackboard_replace_mesh_tdd.rs`** (NEW) — unit tests: happy-path (AC-1) and `#[should_panic(expected = "replace_mesh called after Tier 2")]` for the Tier-2-violation guard (AC-N1).
  - **`crates/slicer-runtime/tests/contract/prepass_execution_error_mesh_segmentation_variant_tdd.rs`** (NEW; AC-10) — (a) constructs `PrepassExecutionError::MeshSegmentation { source: MeshSegmentationError::<some-real-variant>(...) }` directly; (b) exercises a `fn() -> Result<(), PrepassExecutionError>` that invokes a function returning `MeshSegmentationError` via `?`. The test passes by compiling; runtime assertions are minimal.
- Rejected alternatives that were considered and why they were not chosen:
  - **Run mesh-segmentation as part of mesh-commit** (in the constructor): violates the "stages are explicit" design; mesh-segmentation would be hidden and its short-circuit observability invisible to the instrumentation harness.
  - **Use `commit_mesh` instead of `replace_mesh`**: `commit_mesh` is the initial-commit path; `replace_mesh` is the post-init swap path. Re-using `commit_mesh` would conflate them. Mirror `replace_slice_ir`/`commit_slice_ir` distinction.
  - **Add a DAG-validator precedence rule** (host-builtins beat WASM modules on the same stage): out of P94's wiring-only scope; the manifest-disable in AC-3.5 is a one-line surgical edit with the same observable effect and no validator-semantics churn.
  - **Skip the short-circuit guard** and always call `execute_mesh_segmentation`: the kernel is a no-op on unpainted meshes (returns the input mesh unchanged per AC-N2), so technically calling it is harmless. But the `Arc` clone has a cost and the kernel iterates objects looking for strokes anyway — guarding once at the driver is cheaper.
  - **Defer the WASM-manifest stage-claim to P5a** (i.e., do not include AC-3.5 in P94): rejected. The verified fact that `modules/core-modules/mesh-segmentation/mesh-segmentation.toml` *currently* declares `stage = "PrePass::MeshSegmentation"` means without this packet's edit, AC-4 cannot pass (the DAG validator sees two producers for the same stage). The one-line manifest edit is the minimum needed to unblock the rest of P94.

## Files in Scope (read + edit)

- `crates/slicer-runtime/src/blackboard.rs` — role: add `replace_mesh`; expected change: ~20 LOC.
- `crates/slicer-runtime/src/builtins/mesh_segmentation_producer.rs` (NEW) — role: producer constant; expected change: ~35 LOC mirroring mesh_analysis_producer.rs.
- `crates/slicer-runtime/src/builtins/mod.rs` — role: module declaration; expected change: one line.
- `crates/slicer-runtime/src/prepass.rs` — role: driver insertion + error variant + table entry + `has_subfacet_strokes` helper; expected change: ~25 LOC.
- `modules/core-modules/mesh-segmentation/mesh-segmentation.toml` — role: disable the `stage = "PrePass::MeshSegmentation"` claim (AC-3.5); expected change: one line (comment-out or equivalent disable mechanism).
- `crates/slicer-runtime/tests/executor/*.rs` (4 new files) — role: integration tests; expected change: 4 new files (each ~80 LOC).
- `crates/slicer-runtime/tests/contract/blackboard_replace_mesh_tdd.rs` (NEW) — role: Blackboard contract test; expected change: 1 new file (~60 LOC).
- `crates/slicer-runtime/tests/contract/prepass_execution_error_mesh_segmentation_variant_tdd.rs` (NEW; AC-10) — role: variant construction + `?`-propagation; expected change: 1 new file (~40 LOC).
- `.ralph/specs/94_host-mesh-segmentation-wiring/closure-log.md` — role: capture `P93_BASELINE_SHA` (Step 0), `P94_PRE_PAINTED_CUBE_SHA` (Step 0), `P94_POST_PAINTED_CUBE_SHA` (Step 7), and one-paragraph normalization rationale; created in Step 0 if absent.

All edits ≤ 3 files per implementation-plan step.

## Read-Only Context

- `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P2".
- `crates/slicer-runtime/src/blackboard.rs` — lines 270-310 (the `replace_slice_ir` template).
- `crates/slicer-runtime/src/builtins/mesh_analysis_producer.rs` — full (47 LOC).
- `crates/slicer-core/src/algos/mesh_segmentation.rs` — lines 1-50 (signature + error type only). DO NOT edit.
- `crates/slicer-runtime/src/prepass.rs` — lines 360-410 (driver insertion site) and lines 680-720 (table). Ranged reads.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — delegate; no parity check is expected for this packet.
- `target/`, `Cargo.lock`, generated code — never load.
- Binary 3MF / STL fixtures — never `Read`.
- `crates/slicer-core/src/algos/mesh_segmentation.rs` lines 50-onwards (the kernel body) — read-only; not edited.
- `modules/core-modules/mesh-segmentation/**` EXCEPT `mesh-segmentation.toml` — P5a's deletion target; only the single manifest's stage-claim line is edited here for AC-3.5.
- `crates/slicer-wasm-host/**` — not in scope.
- `crates/slicer-runtime/src/dispatch.rs` — not in scope.

## Expected Sub-Agent Dispatches

- "Open `crates/slicer-runtime/src/blackboard.rs` lines 270-310 and return SNIPPETS of `replace_slice_ir` (≤ 30 lines)" — purpose: replicate its shape.
- "Open `crates/slicer-runtime/src/builtins/mesh_analysis_producer.rs` and return SNIPPETS (≤ 30 lines)" — purpose: template for `mesh_segmentation_producer.rs`.
- "Open `crates/slicer-runtime/src/prepass.rs` lines 360-410; return SNIPPETS showing how `host:mesh_analysis` is invoked (≤ 30 lines)" — purpose: pattern for `host:mesh_segmentation` insertion.
- "Open `crates/slicer-runtime/src/prepass.rs` lines 680-720; return SNIPPETS of `required_slots` (≤ 25 lines)" — purpose: locate insertion point in the table.
- "Locate `PrepassExecutionError` enum definition; return FILE:LINE" — purpose: error-variant addition.
- "Locate `MeshSegmentationError` definition in `crates/slicer-core/src/algos/mesh_segmentation.rs`; return FILE:LINE + the variant list" — purpose: ensure `From` derive compatibility.
- "Run `cargo test -p slicer-core --test algo_mesh_segmentation_tdd 2>&1 | tee target/test-output.log`; return FACT pass/fail" — purpose: kernel-tests pre-check.
- "Run `cargo test -p slicer-runtime --test executor mesh_segmentation 2>&1 | tee target/test-output.log`; return FACT pass/fail per test" — purpose: integration tests.
- "Run `cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output /tmp/p94-wedge.gcode && sha256sum /tmp/p94-wedge.gcode`; return FACT (sha256)" — purpose: AC-11.

## Data and Contract Notes

- IR or manifest contracts touched: none new. The `MeshIR` shape is unchanged. The `BuiltinProducer` constant is one more registered producer.
- WIT boundary considerations: none. No WIT change.
- Determinism or scheduler constraints: the new stage runs first; its short-circuit makes it observably no-op on unpainted meshes.

## Locked Assumptions and Invariants

- **`PrePass::MeshSegmentation` runs FIRST**: reversing would corrupt mesh-analysis output. Sealed by the empty `required_slots` entry + DAG validator.
- **Short-circuit on unpainted meshes**: zero overhead vs pre-packet for the unpainted case (AC-11 baseline-compare against `P93_BASELINE_SHA` is the gate).
- **Kernel is dead code → kernel is live code**: the post-packet workspace must contain at least one reference to `execute_mesh_segmentation` from `crates/slicer-runtime/src/`. AC-N3 asserts this.
- **`replace_mesh` is a Tier-1-only operation, enforced by `debug_assert!` only.** Release-mode behavior on Tier-2 violation is undefined; this matches `replace_slice_ir`'s established contract. Adding a runtime `BlackboardError::TierViolation` variant is explicitly out of scope.
- **Host built-in is the sole producer for `PrePass::MeshSegmentation`** after P94. The WASM manifest's stage claim is disabled (AC-3.5). P5a (97) removes the directory entirely; until then, the `modules/core-modules/mesh-segmentation/` source tree exists but produces no stage.

## Risks and Tradeoffs

- **Risk: a downstream stage's behavior changes because it now sees normalized facet_values instead of un-normalized strokes.** Mitigation: AC-6/AC-7 confirm strokes are consumed correctly; AC-11 confirms unpainted behavior unchanged; AC-12 captures the painted SHA for traceability. The diff is expected and correct.
- **Risk: `replace_slice_ir`'s tier-guard pattern was a one-off, not a recipe**, and replicating it for `replace_mesh` introduces a subtly wrong guard. Mitigation: the implementer reads `replace_slice_ir` first (Step 1 dispatch) and mirrors its shape exactly.
- **Risk: a test guest somewhere ELSE still declares `PrePass::MeshSegmentation` as a guest-output stage.** The known offender — `modules/core-modules/mesh-segmentation/mesh-segmentation.toml` — is handled by AC-3.5. But the workspace may contain additional declarations (e.g., under `crates/slicer-runtime/test-guests/`, fixtures, or harness manifests). Mitigation: in Step 3.5, dispatch `rg -nE '^stage\s*=\s*"PrePass::MeshSegmentation"' modules/ crates/slicer-runtime/test-guests/ | rg -v 'mesh-segmentation\.toml'`. If any non-host source declares it, treat as an activation-blocking finding (open a new packet or widen the AC-3.5 disable mechanism to cover those files).
- **Tradeoff: `has_subfacet_strokes` is a free helper, not a kernel method.** Adding it to the kernel would couple kernel I/O knowledge to the driver; keeping it driver-side preserves layer separation.

## Context Cost Estimate

- Aggregate: `M`.
- Largest single step: `M` (Step 3 — driver insertion + error variant + table entry in one file with three concerns).
- Highest-risk dispatch: the prepass-driver SNIPPETS dispatch (must surface the existing `host:mesh_analysis` invocation pattern without loading the whole `prepass.rs` file).

## Open Questions

- `[FWD]` — `has_subfacet_strokes` helper: verified. A **private** function `mesh_has_subfacet_strokes(mesh_ir: &MeshIR) -> bool` exists at `crates/slicer-core/src/algos/mesh_segmentation.rs:57`. Since P94's scope forbids edits to the kernel file, the implementer adds an equivalent driver-local helper `has_subfacet_strokes(mesh: &MeshIR) -> bool` in `prepass.rs` per the body shown in §Code Change Surface. Promoting the kernel's helper to `pub` is rejected here (touches the read-only kernel file); the driver-local duplicate is ~5 lines and stays in `slicer-runtime` per the layer-separation invariant in §Locked Assumptions.
- `[FWD]` — `run_builtin_stage` signature is verified (it's the existing private helper at `prepass.rs:627-633`: `fn run_builtin_stage(blackboard: &mut Blackboard, instrumentation: &(dyn PipelineInstrumentation + Sync), stage_id: &'static str, module_id: &'static str, should_run: impl FnOnce(&Blackboard) -> bool, execute: impl FnOnce(&mut Blackboard) -> Result<(), PrepassExecutionError>) -> Result<(), PrepassExecutionError>`). The pseudo-code in §Code Change Surface already matches this shape; no rediscovery needed.
- `[BLOCK]` — None. The stage-ownership conflict that would have been a blocker is owned by AC-3.5.
