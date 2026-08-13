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
pub struct HostState {
    module_id: String,
    table: wasmtime::component::ResourceTable,
    /// Default-deny WASI execution state for component instantiation.
    pub wasi: wasmtime_wasi::WasiCtx,
}

impl fmt::Debug for HostState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HostState")
            .field("module_id", &self.module_id)
            .finish()
    }
}

impl HostState {
    /// Create a new host state with the given module identifier.
    pub fn new(module_id: String) -> Self {
        Self {
            module_id,
            table: wasmtime::component::ResourceTable::new(),
            wasi: wasmtime_wasi::WasiCtxBuilder::new().build(),
        }
    }

    /// Returns the module identifier.
    pub fn module_id(&self) -> &str {
        &self.module_id
    }
}

impl wasmtime_wasi::WasiView for HostState {
    fn ctx(&mut self) -> wasmtime_wasi::WasiCtxView<'_> {
        wasmtime_wasi::WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

// `HostState` backs the bare `WasmInstance` path, which has no WIT `profiling`
// import to report through — it takes the no-op default so it still routes
// through `new_store` and therefore still gets a fuel budget.
impl FuelSampleSink for HostState {}

/// Fuel handed to every store built by [`new_store`] when profiling is on.
///
/// Enabling `Config::consume_fuel` makes a store with no fuel trap on its very
/// first instruction, so a budget is mandatory rather than optional. This one is
/// effectively unlimited — at a billion instructions per second it would take
/// over a century to exhaust — and it stays at or below `i64::MAX` so wasmtime
/// injects the whole amount into the VM instead of splitting it into a reserve,
/// which keeps `budget - get_fuel()` an exact consumed-fuel reading.
pub const FUEL_BUDGET: u64 = 1 << 62;

/// Store data that can accept the fuel sample taken at a guest→host boundary.
///
/// This exists because bindgen-generated host methods receive `&mut T`, never
/// the `Store`, and `get_fuel` lives on the store. The store's `call_hook` sees
/// both, so it pushes the reading *into* `T` on the way in and the host method
/// reads it back out. Implementors that do not profile inherit the no-op
/// default and pay nothing.
pub trait FuelSampleSink: 'static {
    /// Called on every `CallHook::CallingHost` transition with the fuel the
    /// guest has consumed so far in this call. Only fires when profiling is on.
    fn record_host_entry_fuel(&mut self, consumed: u64) {
        let _ = consumed;
    }
}

/// Builds the `wasmtime::Store` for a dispatch call, applying the fuel budget
/// and profiling call hook that [`WasmEngine::with_profiling`] implies.
///
/// **Every** store in this crate goes through here. With `consume_fuel` on, a
/// store that skipped the budget would trap on its first instruction, and that
/// failure would look like a broken module rather than a missed call site — so
/// the budget is not something individual call sites are trusted to remember.
pub fn new_store<T: FuelSampleSink>(engine: &WasmEngine, data: T) -> wasmtime::Store<T> {
    let mut store = wasmtime::Store::new(&engine.inner, data);
    if engine.profiling_enabled() {
        store
            .set_fuel(FUEL_BUDGET)
            .expect("consume_fuel is enabled whenever profiling is");
        store.call_hook(|mut cx, hook| {
            if matches!(hook, wasmtime::CallHook::CallingHost) {
                // Sample here rather than inside the host method: this is the
                // last point at which the store is reachable. A mark is a host
                // call, so this fires immediately before `profile_mark` runs and
                // the reading it stashes is the one that mark reports.
                let remaining = cx.get_fuel().unwrap_or(FUEL_BUDGET);
                cx.data_mut()
                    .record_host_entry_fuel(FUEL_BUDGET.saturating_sub(remaining));
            }
            Ok(())
        });
    }
    store
}

/// Wrapper around a [`wasmtime::Engine`] with the component model enabled.
pub struct WasmEngine {
    inner: wasmtime::Engine,
    profiling: bool,
}

impl fmt::Debug for WasmEngine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WasmEngine").finish()
    }
}

impl WasmEngine {
    /// Create a new engine with the component model enabled and profiling off.
    ///
    /// This is the default for every non-profiling call site: fuel metering
    /// costs throughput, so it rides an explicit opt-in (ADR-0055).
    pub fn new() -> Self {
        Self::with_profiling(false)
    }

    /// Create a new engine, optionally with fuel metering enabled.
    ///
    /// With `profiling` on, `Config::consume_fuel` makes wasmtime count executed
    /// instructions, which is what gives per-scope attribution a deterministic,
    /// machine-independent signal. It also means every store needs a fuel
    /// budget — see [`new_store`], which is the only place stores are built.
    pub fn with_profiling(profiling: bool) -> Self {
        let mut config = wasmtime::Config::new();
        config.wasm_component_model(true);
        config.consume_fuel(profiling);
        let engine = wasmtime::Engine::new(&config).expect("failed to create wasmtime engine");
        Self {
            inner: engine,
            profiling,
        }
    }

    /// Whether this engine meters fuel and installs the profiling call hook.
    pub fn profiling_enabled(&self) -> bool {
        self.profiling
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
        let mut store = new_store(engine, state);
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

    /// Total fuel this instance's guest code has consumed so far.
    ///
    /// `0` when the engine does not meter fuel, which is the default — see
    /// [`WasmEngine::with_profiling`]. Cumulative across every export call made
    /// on this instance, because they share one store.
    pub fn fuel_consumed(&self) -> u64 {
        FUEL_BUDGET.saturating_sub(self.store.get_fuel().unwrap_or(FUEL_BUDGET))
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

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    /// A component with one export that burns a known-nonzero amount of fuel
    /// and then calls a host import, so a test can observe both the fuel meter
    /// and the `CallHook::CallingHost` transition the profiler rides on.
    const TICKING_COMPONENT_WAT: &str = r#"
    (component
      (import "host-tick" (func $tick))
      (core func $tick_core (canon lower (func $tick)))
      (core module $m
        (import "host" "tick" (func $tick))
        (func (export "run")
          (local $i i32)
          (loop $l
            (local.set $i (i32.add (local.get $i) (i32.const 1)))
            (br_if $l (i32.lt_s (local.get $i) (i32.const 1000))))
          (call $tick)))
      (core instance $shim (export "tick" (func $tick_core)))
      (core instance $i (instantiate $m (with "host" (instance $shim))))
      (func (export "run") (canon lift (core func $i "run")))
    )
    "#;

    /// Store data that records every fuel sample the call hook pushes at it.
    #[derive(Default)]
    struct RecordingState {
        samples: RefCell<Vec<u64>>,
    }

    impl FuelSampleSink for RecordingState {
        fn record_host_entry_fuel(&mut self, consumed: u64) {
            self.samples.borrow_mut().push(consumed);
        }
    }

    fn run_ticking_component(engine: &WasmEngine) -> (u64, Vec<u64>) {
        let wasm = wat::parse_str(TICKING_COMPONENT_WAT).expect("WAT parse");
        let component = wasmtime::component::Component::new(engine.wasmtime_engine(), &wasm)
            .expect("component compile");

        let mut linker =
            wasmtime::component::Linker::<RecordingState>::new(engine.wasmtime_engine());
        linker
            .root()
            .func_wrap("host-tick", |_store, (): ()| Ok(()))
            .expect("wire host-tick");

        let mut store = new_store(engine, RecordingState::default());
        let instance = linker
            .instantiate(&mut store, &component)
            .expect("instantiate");
        let run = instance
            .get_typed_func::<(), ()>(&mut store, "run")
            .expect("run export");
        run.call(&mut store, ()).expect("call must not trap");

        let consumed = FUEL_BUDGET.saturating_sub(store.get_fuel().unwrap_or(FUEL_BUDGET));
        let samples = store.data().samples.borrow().clone();
        (consumed, samples)
    }

    /// The regression this guards: turning on `consume_fuel` makes a store with
    /// no fuel trap on its first instruction. If any construction site skipped
    /// the budget, `run.call` would fail here rather than at some distant
    /// module dispatch that looks like a broken guest.
    #[test]
    fn profiling_engine_budgets_fuel_and_reports_consumption() {
        let engine = WasmEngine::with_profiling(true);
        assert!(engine.profiling_enabled());

        let (consumed, samples) = run_ticking_component(&engine);

        assert!(
            consumed > 1000,
            "the 1000-iteration loop must register on the fuel meter, saw {consumed}"
        );

        // The call hook fired on the guest→host transition, which is the whole
        // mechanism `profile_mark` uses to see fuel it cannot otherwise reach.
        assert_eq!(
            samples.len(),
            1,
            "exactly one CallingHost transition expected, saw {samples:?}"
        );
        assert!(
            samples[0] > 1000 && samples[0] <= consumed,
            "the sample must be the fuel burned before the host call, \
             saw {} against a call total of {consumed}",
            samples[0]
        );
    }

    /// The default engine must stay exactly as it was: no metering, no hook,
    /// and no fuel-related failure on a call.
    #[test]
    fn default_engine_does_not_meter_fuel() {
        let engine = WasmEngine::new();
        assert!(!engine.profiling_enabled());

        let (consumed, samples) = run_ticking_component(&engine);

        assert_eq!(
            consumed, 0,
            "fuel must read as 'no sample' when metering off"
        );
        assert!(
            samples.is_empty(),
            "no call hook may be installed when profiling is off"
        );
    }

    /// `WasmInstance` goes through the same shared store constructor, so the
    /// bare instantiate/call path must survive a profiling engine too.
    #[test]
    fn wasm_instance_path_survives_profiling_and_exposes_fuel() {
        let engine = WasmEngine::with_profiling(true);
        // No imports here: `WasmInstance`'s linker is `Linker<HostState>` and
        // carries no host functions, so the component must be self-contained.
        let wat = r#"
        (component
          (core module $m
            (func (export "run")
              (local $i i32)
              (loop $l
                (local.set $i (i32.add (local.get $i) (i32.const 1)))
                (br_if $l (i32.lt_s (local.get $i) (i32.const 1000))))))
          (core instance $i (instantiate $m))
          (func (export "run") (canon lift (core func $i "run")))
        )
        "#;
        let wasm = wat::parse_str(wat).expect("WAT parse");
        let component = engine.compile_component(&wasm).expect("compile");
        let mut instance = component
            .instantiate(&engine, HostState::new("fuel-probe".to_string()))
            .expect("instantiate must not trap for lack of fuel");

        assert_eq!(instance.fuel_consumed(), 0, "no export called yet");
        instance
            .call_void_export("run")
            .expect("call must not trap");
        assert!(
            instance.fuel_consumed() > 1000,
            "fuel must accumulate on the instance's store"
        );
    }
}
