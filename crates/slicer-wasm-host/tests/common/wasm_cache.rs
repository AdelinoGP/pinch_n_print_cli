//! Process-local cache for wasmtime engine + compiled components, used by
//! `slicer-wasm-host` test binaries.
//!
//! Duplicated from `slicer-runtime/tests/common/wasm_cache.rs` minus the
//! `cached_live_modules` helper (orchestrator-only). P83's AC-N3 forbids a
//! `slicer-wasm-host[dev] → slicer-runtime` back-edge; the engine /
//! component / WAT helpers below carry no orchestrator coupling and live
//! on the wasm-host side of the boundary.

#![allow(dead_code)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use slicer_wasm_host::{WasmComponent, WasmEngine};

static ENGINE: OnceLock<Arc<WasmEngine>> = OnceLock::new();

pub fn shared_engine() -> Arc<WasmEngine> {
    ENGINE.get_or_init(|| Arc::new(WasmEngine::new())).clone()
}

type ComponentCell = Arc<OnceLock<Arc<WasmComponent>>>;
type ComponentMap = HashMap<PathBuf, ComponentCell>;
static COMPONENT_CACHE: OnceLock<Mutex<ComponentMap>> = OnceLock::new();

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

pub fn compiled_guest(name: &str) -> Arc<WasmComponent> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test-guests")
        .join(format!("{name}.component.wasm"));
    compiled_component_at(&path)
}

type WatCell = Arc<OnceLock<Arc<WasmComponent>>>;
type WatMap = HashMap<String, WatCell>;
static WAT_CACHE: OnceLock<Mutex<WatMap>> = OnceLock::new();

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
