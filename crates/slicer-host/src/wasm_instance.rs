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

/// Structured errors for WASM export invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WasmCallError {
    /// The requested export function was not found in the component.
    ExportNotFound {
        /// Module identifier from the manifest.
        module_id: String,
        /// Export function name that was looked up.
        export_name: String,
        /// Human-readable reason.
        reason: String,
    },
    /// The export function call failed at runtime.
    CallFailed {
        /// Module identifier from the manifest.
        module_id: String,
        /// Export function name that was called.
        export_name: String,
        /// Human-readable reason for the call failure.
        reason: String,
    },
}

impl fmt::Display for WasmCallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WasmCallError::ExportNotFound {
                module_id,
                export_name,
                reason,
            } => write!(
                f,
                "export '{export_name}' not found in module '{module_id}': {reason}"
            ),
            WasmCallError::CallFailed {
                module_id,
                export_name,
                reason,
            } => write!(
                f,
                "call to '{export_name}' failed in module '{module_id}': {reason}"
            ),
        }
    }
}

impl std::error::Error for WasmCallError {}

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

    /// Returns a reference to the underlying `wasmtime::Engine`.
    pub fn wasmtime_engine(&self) -> &wasmtime::Engine {
        &self.inner
    }

    /// Compile raw WASM bytes into a reusable component.
    pub fn compile_component(&self, wasm_bytes: &[u8]) -> Result<WasmComponent, WasmLoadError> {
        wasmtime::component::Component::new(&self.inner, wasm_bytes)
            .map(|c| WasmComponent { inner: c })
            .map_err(|e| WasmLoadError::CompilationFailed {
                reason: e.to_string(),
            })
    }

    /// Create a configurable component linker for this engine.
    pub fn new_linker(&self) -> WasmLinker {
        WasmLinker {
            inner: wasmtime::component::Linker::<HostState>::new(&self.inner),
        }
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

/// Configurable component linker used during instantiation.
pub struct WasmLinker {
    inner: wasmtime::component::Linker<HostState>,
}

impl fmt::Debug for WasmLinker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WasmLinker").finish()
    }
}

impl WasmLinker {
    /// Returns mutable access to the underlying wasmtime linker.
    pub fn linker_mut(&mut self) -> &mut wasmtime::component::Linker<HostState> {
        &mut self.inner
    }
}

impl fmt::Debug for WasmComponent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WasmComponent").finish()
    }
}

impl WasmComponent {
    /// Returns a reference to the underlying `wasmtime::component::Component`.
    pub fn wasmtime_component(&self) -> &wasmtime::component::Component {
        &self.inner
    }

    /// Instantiate this component with the given engine and host state.
    pub fn instantiate(
        &self,
        engine: &WasmEngine,
        state: HostState,
    ) -> Result<WasmInstance, WasmLoadError> {
        let linker = engine.new_linker();
        self.instantiate_with_linker(engine, state, &linker)
    }

    /// Instantiate this component with an explicit linker.
    pub fn instantiate_with_linker(
        &self,
        engine: &WasmEngine,
        state: HostState,
        linker: &WasmLinker,
    ) -> Result<WasmInstance, WasmLoadError> {
        let module_id = state.module_id().to_string();
        let mut store = wasmtime::Store::new(&engine.inner, state);
        linker
            .inner
            .instantiate(&mut store, &self.inner)
            .map(|instance| WasmInstance { store, instance })
            .map_err(|e| WasmLoadError::InstantiationFailed {
                module_id,
                reason: e.to_string(),
            })
    }
}

/// Live WASM component instance with an associated store.
pub struct WasmInstance {
    store: wasmtime::Store<HostState>,
    instance: wasmtime::component::Instance,
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

    /// Invoke a named export function that takes no arguments and returns nothing.
    ///
    /// This is the fundamental dispatch primitive. The host calls this with the
    /// stage-appropriate export name (e.g. `"run-infill"`, `"run-mesh-analysis"`).
    /// Data exchange happens through host-provided imports, not through call
    /// arguments (the WIT contract defines the import/export surface).
    pub fn call_void_export(&mut self, export_name: &str) -> Result<(), WasmCallError> {
        let func = self
            .instance
            .get_typed_func::<(), ()>(&mut self.store, export_name)
            .map_err(|e| WasmCallError::ExportNotFound {
                module_id: self.store.data().module_id().to_string(),
                export_name: export_name.to_string(),
                reason: e.to_string(),
            })?;

        func.call(&mut self.store, ())
            .map_err(|e| WasmCallError::CallFailed {
                module_id: self.store.data().module_id().to_string(),
                export_name: export_name.to_string(),
                reason: e.to_string(),
            })?;

        func.post_return(&mut self.store)
            .map_err(|e| WasmCallError::CallFailed {
                module_id: self.store.data().module_id().to_string(),
                export_name: export_name.to_string(),
                reason: format!("post_return failed: {e}"),
            })?;

        Ok(())
    }

    /// Invoke a named export that takes a string argument and returns a string.
    ///
    /// Used for `PostPass::TextPostProcess` where the module receives serialized
    /// G-code text and returns the modified text.
    pub fn call_text_transform(
        &mut self,
        export_name: &str,
        input: &str,
    ) -> Result<String, WasmCallError> {
        let func = self
            .instance
            .get_typed_func::<(&str,), (String,)>(&mut self.store, export_name)
            .map_err(|e| WasmCallError::ExportNotFound {
                module_id: self.store.data().module_id().to_string(),
                export_name: export_name.to_string(),
                reason: e.to_string(),
            })?;

        let (result,) =
            func.call(&mut self.store, (input,))
                .map_err(|e| WasmCallError::CallFailed {
                    module_id: self.store.data().module_id().to_string(),
                    export_name: export_name.to_string(),
                    reason: e.to_string(),
                })?;

        func.post_return(&mut self.store)
            .map_err(|e| WasmCallError::CallFailed {
                module_id: self.store.data().module_id().to_string(),
                export_name: export_name.to_string(),
                reason: format!("post_return failed: {e}"),
            })?;

        Ok(result)
    }
}
