//! WASM instance pool planning contracts.

use slicer_ir::{ModuleId, StageId};

use crate::LoadedModule;

/// Effective scheduling mode for a module's WASM instance pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstancePoolMode {
    /// Pool may hand out multiple distinct instances concurrently.
    Parallel,
    /// Pool serializes all access through a single instance.
    Serialized,
}

/// Lightweight artifact metadata used during pool planning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WasmArtifactMetadata {
    /// Whether the compiled artifact declares or imports shared WASM memory.
    pub uses_shared_memory: bool,
}

/// Structured planning/load errors for WASM instance pools.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstancePoolError {
    /// Module declared parallel safety but the artifact uses shared WASM memory.
    SharedMemoryRejected {
        /// Module identifier from manifest ingestion.
        module_id: ModuleId,
        /// Scheduler stage for the rejected module.
        stage: StageId,
    },
    /// Placeholder variant while TASK-024 remains unimplemented.
    NotImplemented,
}

/// Planned pool of reusable WASM instances for one compiled module.
#[derive(Debug)]
pub struct WasmInstancePool {
    mode: InstancePoolMode,
    size: usize,
}

impl WasmInstancePool {
    /// Returns the effective scheduling mode for this pool.
    pub fn mode(&self) -> InstancePoolMode {
        self.mode
    }

    /// Returns the effective number of pooled instance slots.
    pub fn size(&self) -> usize {
        self.size
    }

    /// Acquires one instance slot and returns a lease exposing its slot index.
    pub fn acquire(&self) -> WasmInstanceLease {
        todo!("TASK-024: implement WASM instance acquisition")
    }
}

/// RAII lease for one acquired WASM instance slot.
#[derive(Debug)]
pub struct WasmInstanceLease {
    slot_index: usize,
}

impl WasmInstanceLease {
    /// Returns the slot index assigned by the pool.
    pub fn slot_index(&self) -> usize {
        self.slot_index
    }
}

/// Builds the effective WASM instance pool for one loaded module.
pub fn build_wasm_instance_pool(
    module: &LoadedModule,
    host_parallelism: usize,
    artifact: WasmArtifactMetadata,
) -> Result<WasmInstancePool, InstancePoolError> {
    let _ = (module, host_parallelism, artifact);
    todo!("TASK-024: implement WASM instance pool planning")
}
