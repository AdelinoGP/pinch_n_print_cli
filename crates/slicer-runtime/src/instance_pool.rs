//! WASM instance pool planning contracts.

use std::sync::{Arc, Condvar, Mutex};

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
}

/// Planned pool of reusable WASM instances for one compiled module.
#[derive(Debug)]
pub struct WasmInstancePool {
    mode: InstancePoolMode,
    size: usize,
    slots: Arc<SlotAvailability>,
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
        let slot_index = self.slots.acquire();

        WasmInstanceLease {
            slot_index,
            slots: Arc::clone(&self.slots),
        }
    }
}

/// RAII lease for one acquired WASM instance slot.
#[derive(Debug)]
pub struct WasmInstanceLease {
    slot_index: usize,
    slots: Arc<SlotAvailability>,
}

impl WasmInstanceLease {
    /// Returns the slot index assigned by the pool.
    pub fn slot_index(&self) -> usize {
        self.slot_index
    }
}

impl Drop for WasmInstanceLease {
    fn drop(&mut self) {
        self.slots.release(self.slot_index);
    }
}

/// Builds the effective WASM instance pool for one loaded module.
pub fn build_wasm_instance_pool(
    module: &LoadedModule,
    host_parallelism: usize,
    artifact: WasmArtifactMetadata,
) -> Result<WasmInstancePool, InstancePoolError> {
    let stage = module.stage.as_str();
    let is_finalization = stage == "PostPass::LayerFinalization";

    if module.layer_parallel_safe && artifact.uses_shared_memory {
        return Err(InstancePoolError::SharedMemoryRejected {
            module_id: module.id.clone(),
            stage: module.stage.clone(),
        });
    }

    let (mode, size) = if !is_finalization && module.layer_parallel_safe {
        (InstancePoolMode::Parallel, host_parallelism.max(1))
    } else {
        (InstancePoolMode::Serialized, 1)
    };

    Ok(WasmInstancePool {
        mode,
        size,
        slots: Arc::new(SlotAvailability::new(size)),
    })
}

#[derive(Debug)]
struct SlotAvailability {
    state: Mutex<SlotAvailabilityState>,
    available: Condvar,
}

impl SlotAvailability {
    fn new(size: usize) -> Self {
        Self {
            state: Mutex::new(SlotAvailabilityState::new(size)),
            available: Condvar::new(),
        }
    }

    fn acquire(&self) -> usize {
        let mut state = match self.state.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        loop {
            if let Some(slot_index) = state.take_first_available() {
                return slot_index;
            }

            state = match self.available.wait(state) {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
        }
    }

    fn release(&self, slot_index: usize) {
        let mut state = match self.state.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        state.make_available(slot_index);
        self.available.notify_one();
    }
}

#[derive(Debug)]
struct SlotAvailabilityState {
    in_use: Vec<bool>,
}

impl SlotAvailabilityState {
    fn new(size: usize) -> Self {
        Self {
            in_use: vec![false; size],
        }
    }

    fn take_first_available(&mut self) -> Option<usize> {
        let slot_index = self.in_use.iter().position(|in_use| !*in_use)?;
        self.in_use[slot_index] = true;
        Some(slot_index)
    }

    fn make_available(&mut self, slot_index: usize) {
        if let Some(in_use) = self.in_use.get_mut(slot_index) {
            *in_use = false;
        }
    }
}
