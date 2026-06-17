//! WIT / wasmtime marshalling and dispatch for Pinch 'n Print's WASM module host.
//!
//! Extracted from `slicer-runtime` in packet 83. ADR-0002 requires all four
//! `bindgen!` invocations (layer / prepass / finalization / postpass) to be
//! co-located in this crate so the `with:` remap pattern produces shared
//! Rust type identity across the four worlds. The layer world is canonical;
//! see `host.rs` for the ordering rule.

pub mod binding;
pub mod dispatch;
pub mod execution_plan_live;
pub mod host;
pub mod instance;
/// IR-to-WIT and WIT-to-IR marshalling helpers for the WASM stage boundary.
pub mod marshal;
pub mod pool;
pub mod traits;

// ---------------------------------------------------------------------------
// Public surface re-exports
// ---------------------------------------------------------------------------

pub use binding::{
    CompiledModuleLive, FinalizationStageInput, LayerStageInput, PostpassStageInput,
    PrepassStageInput,
};

pub use instance::{
    HostState, WasmCallError, WasmComponent, WasmEngine, WasmInstance, WasmLinker, WasmLoadError,
};

pub use pool::{
    build_wasm_instance_pool, InstancePoolError, InstancePoolMode, WasmArtifactMetadata,
    WasmInstanceLease, WasmInstancePool,
};

pub use traits::{
    FinalizationStageRunner, LayerStageRunner, PostpassStageRunner, PrepassStageRunner,
};

pub use dispatch::{DispatchError, DispatchPhase, WasmRuntimeDispatcher};

pub use execution_plan_live::{
    build_live_execution_plan, load_live_modules_for_plan, LiveModuleBinding, LiveModuleLoadError,
    LiveModuleLoadOutput,
};
