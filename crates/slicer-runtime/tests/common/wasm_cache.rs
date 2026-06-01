//! Process-local cache for wasmtime engine, compiled components, and
//! production-loader plans, used by the slicer-runtime test binaries.
//!
//! Each `cargo test --test <bucket>` runs as one process; without caching,
//! every test that needs a `WasmEngine` pays a fresh Cranelift JIT cold
//! start (~100-300 ms on Windows), and every test that needs a compiled
//! guest or core-module component pays a full `Component::new` validate +
//! compile (50-200 ms per component, ×20 production core modules). With
//! hundreds of tests per bucket this dominates test wall-clock.
//!
//! `wasmtime::Engine` and `wasmtime::component::Component` are `Send +
//! Sync` and explicitly designed for cross-`Store` sharing, so process-
//! wide caching is the intended pattern. Each test still constructs its
//! own `Store`, preserving per-test runtime isolation.
//!
//! # Engine-discipline rule
//!
//! `load_live_modules_for_plan` builds its own internal `WasmEngine` (see
//! `execution_plan.rs:380`). The `Arc<WasmEngine>` carried inside a
//! [`cached_live_modules`] result is therefore *a different engine* from
//! [`shared_engine`]. wasmtime requires that a `Component` and the
//! `Store` instantiating it share the same engine — mixing the two
//! engine sources in one test produces a non-obvious "engine mismatch"
//! instantiation error.
//!
//! Rules:
//! - A test that uses [`cached_live_modules`] and then dispatches must
//!   instantiate via `out.engine.as_ref()` (carried on the cached
//!   output), **not** [`shared_engine`].
//! - A test that uses only [`shared_engine`] + [`compiled_guest`] /
//!   [`compiled_wat`] / [`compiled_component_at`] uses [`shared_engine`]
//!   throughout.
//! - A single test must not mix bindings from [`cached_live_modules`]
//!   with components from [`shared_engine`].
//!
//! # Carve-outs
//!
//! Tests whose purpose is to exercise the raw `WasmEngine::new()` /
//! `compile_component` / `load_live_modules_for_plan` APIs must not use
//! these helpers (a cache hit would no longer exercise a real
//! compile/load and a stale artifact could silently mask a regression).
//! Failure-path tests that intentionally feed invalid bytes likewise
//! bypass the component cache — they may use [`shared_engine`] but the
//! `compile_component(bad_bytes)` call stays direct.

#![allow(dead_code)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use slicer_runtime::{load_live_modules_for_plan, LiveModuleLoadOutput, WasmComponent, WasmEngine};

// ── Shared engine ─────────────────────────────────────────────────────

static ENGINE: OnceLock<Arc<WasmEngine>> = OnceLock::new();

/// Process-wide shared `WasmEngine`. Constructed once on first call,
/// returned via cheap `Arc` clones thereafter.
pub fn shared_engine() -> Arc<WasmEngine> {
    ENGINE.get_or_init(|| Arc::new(WasmEngine::new())).clone()
}

// ── Compiled-component cache (by canonical file path) ─────────────────

type ComponentCell = Arc<OnceLock<Arc<WasmComponent>>>;
type ComponentMap = HashMap<PathBuf, ComponentCell>;
static COMPONENT_CACHE: OnceLock<Mutex<ComponentMap>> = OnceLock::new();

/// Compile any `.component.wasm` / `.wasm` file once per test binary,
/// keyed by canonical absolute path. Reused for both test-guests and
/// production core-modules.
///
/// The outer `Mutex` is held only across the per-key cell insert; the
/// compile happens against a per-key inner `OnceLock` outside the
/// mutex, so a panic during `Component::new` cannot poison the cache.
pub fn compiled_component_at(path: &Path) -> Arc<WasmComponent> {
    let key = path
        .canonicalize()
        .unwrap_or_else(|e| panic!("canonicalize {} for cache key: {}", path.display(), e));

    let cell: ComponentCell = {
        let mut map = COMPONENT_CACHE
            .get_or_init(|| Mutex::new(HashMap::new()))
            .lock()
            .expect("wasm component cache mutex");
        map.entry(key.clone())
            .or_insert_with(|| Arc::new(OnceLock::new()))
            .clone()
    };

    cell.get_or_init(|| {
        let bytes = std::fs::read(&key)
            .unwrap_or_else(|e| panic!("read wasm file {}: {}", key.display(), e));
        let component = shared_engine()
            .compile_component(&bytes)
            .unwrap_or_else(|e| panic!("compile component {}: {}", key.display(), e));
        Arc::new(component)
    })
    .clone()
}

/// Convenience: resolve `<CARGO_MANIFEST_DIR>/test-guests/<name>.component.wasm`
/// and delegate to [`compiled_component_at`].
pub fn compiled_guest(name: &str) -> Arc<WasmComponent> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test-guests")
        .join(format!("{name}.component.wasm"));
    compiled_component_at(&path)
}

// ── Compiled-WAT cache (by source string) ─────────────────────────────

type WatCell = Arc<OnceLock<Arc<WasmComponent>>>;
type WatMap = HashMap<String, WatCell>;
static WAT_CACHE: OnceLock<Mutex<WatMap>> = OnceLock::new();

/// Compile a WAT / component-text string once per test binary, keyed by
/// the source string. ONLY for inputs the caller knows compile cleanly;
/// failure-path tests bypass this and call
/// `shared_engine().compile_component(bytes)` directly.
pub fn compiled_wat(wat: &str) -> Arc<WasmComponent> {
    let cell: WatCell = {
        let mut map = WAT_CACHE
            .get_or_init(|| Mutex::new(HashMap::new()))
            .lock()
            .expect("wat cache mutex");
        map.entry(wat.to_string())
            .or_insert_with(|| Arc::new(OnceLock::new()))
            .clone()
    };

    cell.get_or_init(|| {
        let component = shared_engine()
            .compile_component(wat.as_bytes())
            .unwrap_or_else(|e| panic!("compile WAT component: {e}"));
        Arc::new(component)
    })
    .clone()
}

// ── Cached LiveModuleLoadOutput (by canonical roots + parallelism) ────

type PlanKey = (Vec<PathBuf>, usize);
type PlanCell = Arc<OnceLock<Arc<LiveModuleLoadOutput>>>;
type PlanMap = HashMap<PlanKey, PlanCell>;
static PLAN_CACHE: OnceLock<Mutex<PlanMap>> = OnceLock::new();

/// Memoise the production loader's output keyed by canonicalised search
/// roots + `host_parallelism`. First call invokes
/// [`load_live_modules_for_plan`] directly; subsequent calls return the
/// cached `Arc<LiveModuleLoadOutput>`.
///
/// **Carries its own internal `Arc<WasmEngine>`** (built by the loader),
/// which is *different* from [`shared_engine`]. See the
/// engine-discipline rule in the module docstring.
pub fn cached_live_modules(roots: &[PathBuf], parallelism: usize) -> Arc<LiveModuleLoadOutput> {
    let canonical_roots: Vec<PathBuf> = roots
        .iter()
        .map(|p| p.canonicalize().unwrap_or_else(|_| p.clone()))
        .collect();
    let key: PlanKey = (canonical_roots, parallelism);

    let cell: PlanCell = {
        let mut map = PLAN_CACHE
            .get_or_init(|| Mutex::new(HashMap::new()))
            .lock()
            .expect("live-modules plan cache mutex");
        map.entry(key.clone())
            .or_insert_with(|| Arc::new(OnceLock::new()))
            .clone()
    };

    cell.get_or_init(|| {
        let out = load_live_modules_for_plan(&key.0, key.1)
            .unwrap_or_else(|e| panic!("load_live_modules_for_plan: {e}"));
        Arc::new(out)
    })
    .clone()
}
