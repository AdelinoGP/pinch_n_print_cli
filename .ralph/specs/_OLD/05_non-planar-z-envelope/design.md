# Design: non-planar-z-envelope

## Controlling Code Paths

- Primary code path: `crates/slicer-host/src/dispatch.rs` → `dispatch_layer_call` → creates `HostExecutionContext` → calls `ir::HostInfillOutputBuilder::push_sparse_path` etc. in `crates/slicer-host/src/wit_host.rs`
- Neighboring tests or fixtures: `crates/slicer-host/tests/z_envelope_contract_tdd.rs` (new), `crates/slicer-host/tests/core_module_ir_access_contract_tdd.rs` (model for TDD pattern), `crates/slicer-host/tests/claim_transition_matrix_tdd.rs` (model for contract-error TDD)
- OrcaSlicer comparison surface: None — this is an internal contract enforcement task

## Architecture Constraints

- Z envelope validation must happen inside the WIT boundary, not at the IR commit stage, to catch violations as early as possible and before any state is mutated
- `HostExecutionContext` is created per-call and already carries per-call state; it is the natural place to store the envelope bounds
- The envelope check must be in the `push_*` methods that accept `ExtrusionPath3d` or `Point3`, not deferred to a later commit step, because those methods are the actual commit boundary for Tier 2
- Fatal errors from `push_*` methods are already handled as `Result<Result<(), String>, wasmtime::Error>` — the inner `Err(String)` maps to the module error message; this is consistent with the existing error handling pattern
- Catch-up layer adjustment: when `is_catchup_layer = true`, the lower bound is `catchup_z_bottom` (not `layer.z`), but the upper bound remains `catchup_z_bottom + effective_layer_height` (same H, just shifted down)

## Code Change Surface

- Selected approach: Store envelope parameters (`layer_z`, `effective_layer_height`, `catchup_z_bottom: Option<f32>`) on `HostExecutionContext`, add a private `check_z_envelope(z: f32) -> Result<(), String>` helper, call it at the top of every Z-bearing `push_*` method
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - `crates/slicer-host/src/wit_host.rs`:
    - `HostExecutionContext` struct: add `layer_z: f32`, `effective_layer_height: f32`, `catchup_z_bottom: Option<f32>` fields
    - `HostExecutionContext::new`: accept these three parameters
    - Add `fn check_z_envelope(&self, z: f32) -> Result<(), String>`
    - Modify `push_sparse_path`, `push_solid_path`, `push_ironing_path` (InfillOutputBuilder)
    - Modify `push_wall_loop`, `push_seam_candidate` (PerimeterOutputBuilder)
    - Modify `push_support_path`, `push_interface_path`, `push_raft_path` (SupportOutputBuilder)
  - `crates/slicer-host/src/dispatch.rs`:
    - `dispatch_layer_call`: pass `layer.z`, `effective_layer_height`, `catchup_z_bottom` when creating `HostExecutionContext`
    - `LayerParams`: currently stores `layer_z: f32` only — confirm `effective_layer_height` and `is_catchup_layer/catchup_z_bottom` are accessible from the arena or layer being processed
  - New file: `crates/slicer-host/tests/z_envelope_contract_tdd.rs`
  - `crates/slicer-host/src/lib.rs`: if `ZEnvelopeError` or similar new error variant is added to the public API, export it

- Rejected alternatives that were not chosen:
  - Validating in `LayerArena::commit_*` methods — too late (state already mutated), and arena commit is shared across stages making per-module enforcement noisy
  - Validating in `Blackboard::commit_output` — same issue; not all push paths go through a central commit
  - Storing the envelope on each output builder data struct instead of `HostExecutionContext` — each builder (`InfillOutputBuilderData`, etc.) is created fresh per-call and would need the same parameters passed; centralizing on `HostExecutionContext` is simpler

## Data and Contract Notes

- IR or manifest contracts touched: None — envelope enforcement is purely a runtime check at the WIT boundary, not a change to any IR schema or manifest schema
- WIT boundary considerations: The `push_*` methods already return `Result<Result<(), String>, wasmtime::Error>`. The inner `Err(String)` is the module error message surfaced to the progress event system. The envelope violation will be expressed as `Err("Z_ENVELOPE_VIOLATION: Z {z} below layer.z floor {floor}")`. This is consistent with how other fatal contract errors (undeclared reads/writes) are already expressed in the codebase.
- Determinism or scheduler constraints: The check is pure (no side effects, deterministic given the same z value and envelope bounds). Adding it to `push_*` does not affect scheduling or ordering.

## Locked Assumptions and Invariants

- `layer.z < layer.z + effective_layer_height` always holds (envelope always has positive height)
- For catch-up layers: `catchup_z_bottom < layer.z` (catch-up layer's bottom is below the normal layer Z because it skipped a gap)
- `effective_layer_height > 0` always holds
- All Z values in an `ExtrusionPath3d` have the same Z (extrusion paths are planar at one Z height); the envelope check only needs to inspect one Z value per path, not every vertex

## Risks and Tradeoffs

- **Risk**: Adding envelope parameters to `HostExecutionContext::new` changes the call signature, requiring updates to all call sites (prepass dispatch, postpass dispatch, layer dispatch). Mitigation: grep all `HostExecutionContext::new` call sites and update each with the appropriate layer parameters.
- **Risk**: The catch-up layer condition (`is_catchup_layer`) needs to be plumbed from `GlobalLayer` through the dispatch path into `HostExecutionContext`. If `GlobalLayer` does not yet have `catchup_z_bottom` populated reliably for all layers, this needs investigation first. Mitigation: confirm in IR schema (`docs/02_ir_schemas.md` lines 276-278) that `is_catchup_layer` and `catchup_z_bottom` are populated by PrePass.
- **Tradeoff**: The envelope check adds a float comparison to every `push_*` call. This is negligible overhead compared to WASM boundary crossing.

## Open Questions

- Q1: Does `GlobalLayer.catchup_z_bottom` have a defined value for non-catch-up layers (i.e., when `is_catchup_layer = false`)? If so, should we use it as the lower bound always, or only when `is_catchup_layer = true`? **Decision**: When `is_catchup_layer = false`, `catchup_z_bottom` is `None` and lower bound is `layer.z`. When `is_catchup_layer = true`, `catchup_z_bottom` is `Some(B)` and lower bound is `B`. Upper bound is always `lower_bound + effective_layer_height`.
- Q2: Should `push_seam_candidate` (which takes `Point3` directly, not `ExtrusionPath3d`) also be subject to Z envelope validation? **Decision**: Yes — seam candidates are per-layer decisions and Z outside the envelope is equally invalid regardless of the call site. Validate in `push_seam_candidate` as well.
- Q3: Should the error message include the module ID and stage? **Decision**: No — the existing error handling infrastructure (progress events, `handle_module_error`) already annotates errors with module/stage/layer. Adding it here would duplicate context. Keep the message focused: "Z {z} below layer.z floor {floor}".
