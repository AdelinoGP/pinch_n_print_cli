//! WIT / wasmtime marshalling and dispatch for ModularSlicer's WASM module host.
//!
//! Extracted from `slicer-runtime` in packet 83. ADR-0002 requires all four
//! `bindgen!` invocations (layer / prepass / finalization / postpass) to be
//! co-located in this crate so the `with:` remap pattern produces shared
//! Rust type identity across the four worlds. The layer world is canonical;
//! see `host.rs` for the ordering rule.

pub mod binding;
pub mod dispatch;
pub mod host;
pub mod instance;
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
