//! Transitional bridge from `slicer-runtime::dispatch` to `slicer-wasm-host`.
//!
//! P83 Step 4c+4d: The 7 RUNTIME items that were here have moved:
//!   - `derive_layer_output_envelope` → `layer_executor` (private)
//!   - `commit_layer_outputs_for_test` → `layer_executor` (pub)
//!   - `OrderedEntityView` → `layer_executor` (pub)
//!   - `project_ordered_entities` → `layer_executor` (pub)
//!   - `apply_entity_order_proposal` → `layer_executor` (pub)
//!   - `commit_layer_outputs` → `layer_executor` (private)
//!   - `merge_infill_ir` → `layer_executor` (private)
//!
//! All WIT/WASM dispatch machinery moved to `slicer-wasm-host` in Step 4a-iii.
//! External callers that import `slicer_runtime::dispatch::DispatchError` etc.
//! should migrate to `slicer_runtime::DispatchError` (transitional re-exports in lib.rs).

// Re-export wasm-host dispatch types that external callers depend on (transitional bridge).
pub use slicer_wasm_host::{DispatchError, DispatchPhase, WasmRuntimeDispatcher};
