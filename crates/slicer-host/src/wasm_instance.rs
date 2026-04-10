//! Wasmtime-backed WASM component wrapper for module instantiation.
//!
//! Provides [`WasmEngine`], [`WasmComponent`], and [`WasmInstance`] as thin
//! wrappers over `wasmtime` types with the component model enabled.

use std::fmt;

/// Structured errors for WASM component compilation and instantiation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WasmLoadError {
    /// The provided bytes could not be compiled as a WASM component.
    CompilationFailed {
        /// Human-readable reason for the compilation failure.
        reason: String,
    },
    /// A compiled component could not be instantiated in a store.
    InstantiationFailed {
        /// Module identifier from the manifest.
        module_id: String,
        /// Human-readable reason for the instantiation failure.
        reason: String,
    },
}

impl fmt::Display for WasmLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WasmLoadError::CompilationFailed { reason } => {
                write!(f, "WASM compilation failed: {reason}")
            }
            WasmLoadError::InstantiationFailed { module_id, reason } => {
                write!(
                    f,
                    "WASM instantiation failed for module '{module_id}': {reason}"
                )
            }
        }
    }
}

impl std::error::Error for WasmLoadError {}

/// Host-side state passed into the WASM store.
///
/// Holds per-instance metadata such as the module identifier and (in future
/// tasks) logger handles and configuration snapshots.
#[derive(Debug, Clone)]
pub struct HostState {
    module_id: String,
}

impl HostState {
    /// Create a new host state with the given module identifier.
    pub fn new(module_id: String) -> Self {
        Self { module_id }
    }

    /// Returns the module identifier.
    pub fn module_id(&self) -> &str {
        &self.module_id
    }
}

/// Wrapper around a [`wasmtime::Engine`] with the component model enabled.
pub struct WasmEngine {
    inner: wasmtime::Engine,
}

impl fmt::Debug for WasmEngine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WasmEngine").finish()
    }
}

impl WasmEngine {
    /// Create a new engine with the component model enabled.
    pub fn new() -> Self {
        let mut config = wasmtime::Config::new();
        config.wasm_component_model(true);
        let engine = wasmtime::Engine::new(&config).expect("failed to create wasmtime engine");
        Self { inner: engine }
    }

    /// Compile raw WASM bytes into a reusable component.
    pub fn compile_component(&self, wasm_bytes: &[u8]) -> Result<WasmComponent, WasmLoadError> {
        wasmtime::component::Component::new(&self.inner, wasm_bytes)
            .map(|c| WasmComponent { inner: c })
            .map_err(|e| WasmLoadError::CompilationFailed {
                reason: e.to_string(),
            })
    }
}

impl Default for WasmEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Compiled WASM component ready for instantiation.
pub struct WasmComponent {
    inner: wasmtime::component::Component,
}

impl fmt::Debug for WasmComponent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WasmComponent").finish()
    }
}

impl WasmComponent {
    /// Instantiate this component with the given engine and host state.
    pub fn instantiate(
        &self,
        engine: &WasmEngine,
        state: HostState,
    ) -> Result<WasmInstance, WasmLoadError> {
        let module_id = state.module_id().to_string();
        let mut store = wasmtime::Store::new(&engine.inner, state);
        let linker = wasmtime::component::Linker::<HostState>::new(&engine.inner);
        linker
            .instantiate(&mut store, &self.inner)
            .map(|instance| WasmInstance {
                store,
                _instance: instance,
            })
            .map_err(|e| WasmLoadError::InstantiationFailed {
                module_id,
                reason: e.to_string(),
            })
    }
}

/// Live WASM component instance with an associated store.
pub struct WasmInstance {
    store: wasmtime::Store<HostState>,
    _instance: wasmtime::component::Instance,
}

impl fmt::Debug for WasmInstance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WasmInstance")
            .field("module_id", &self.store.data().module_id)
            .finish()
    }
}

impl WasmInstance {
    /// Returns the module identifier for this instance.
    pub fn module_id(&self) -> &str {
        self.store.data().module_id()
    }
}
