//! Process-global cache of compiled module components.
//!
//! The live loader used to build a fresh [`WasmEngine`] and compile every
//! on-disk component serially on **every** load. In debug builds that costs
//! ~10 s per slice (measured by t114: 11.732 s total, 1.833 s of pipeline), and
//! a test binary runs hundreds of slices in one process, so the same bytes are
//! compiled hundreds of times.
//!
//! This module keeps one engine per `profile` flag — a `Component` is
//! engine-bound — and one compiled component per
//! `(profile, canonical path, mtime, length)`. Invalidation is by source
//! identity: a rewritten `.wasm` changes mtime and/or length and misses.
//! Compilation failures are never cached.
//!
//! Sharing an engine is wasmtime's intended pattern (`Engine` and `Component`
//! are `Send + Sync` and designed for cross-`Store` reuse). Per-run isolation
//! is unchanged: every load still builds its own instance pools, and every
//! dispatch still creates its own `Store`.
//!
//! The cache compiles **outside** the lock, so concurrent first-loads of
//! different modules overlap instead of serialising behind one mutex. A
//! duplicate compile of the same key only wastes one compile; it cannot
//! produce a wrong component, because the key is the exact file identity.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::SystemTime;

use crate::instance::{WasmComponent, WasmEngine, WasmLoadError};

#[derive(Clone, PartialEq, Eq, Hash)]
struct ComponentKey {
    profile: bool,
    path: PathBuf,
    modified: SystemTime,
    len: u64,
}

fn engines() -> &'static Mutex<HashMap<bool, Arc<WasmEngine>>> {
    static ENGINES: OnceLock<Mutex<HashMap<bool, Arc<WasmEngine>>>> = OnceLock::new();
    ENGINES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn components() -> &'static Mutex<HashMap<ComponentKey, Arc<WasmComponent>>> {
    static COMPONENTS: OnceLock<Mutex<HashMap<ComponentKey, Arc<WasmComponent>>>> = OnceLock::new();
    COMPONENTS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// The process-global engine for `profile` (`true` = fuel-metered profiling).
///
/// The live loader hands this engine to every `LiveModuleLoadOutput` so that
/// components returned by [`cached_component`] are instantiable by the same
/// dispatch path that compiled them.
pub fn shared_engine(profile: bool) -> Arc<WasmEngine> {
    let mut engines = engines()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    Arc::clone(
        engines
            .entry(profile)
            .or_insert_with(|| Arc::new(WasmEngine::with_profiling(profile))),
    )
}

/// Compile `bytes` at `path` for `profile`, reusing a previously compiled
/// component when the file identity is unchanged.
///
/// Returns a compile error unchanged; the failure is not cached, so a fixed
/// artifact is picked up on the next call.
pub fn cached_component(
    profile: bool,
    path: &Path,
    bytes: &[u8],
) -> Result<Arc<WasmComponent>, WasmLoadError> {
    // Without a file identity there is nothing safe to key on: compile
    // directly rather than risk serving a stale component.
    let Ok(metadata) = std::fs::metadata(path) else {
        return shared_engine(profile)
            .compile_component(bytes)
            .map(Arc::new);
    };
    let Ok(modified) = metadata.modified() else {
        return shared_engine(profile)
            .compile_component(bytes)
            .map(Arc::new);
    };
    let key = ComponentKey {
        profile,
        path: path.to_path_buf(),
        modified,
        len: metadata.len(),
    };

    if let Some(component) = components()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .get(&key)
        .cloned()
    {
        return Ok(component);
    }

    let component = Arc::new(shared_engine(profile).compile_component(bytes)?);
    components()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .insert(key, Arc::clone(&component));
    Ok(component)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_compile_of_the_same_bytes_returns_the_same_component() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!(
            "pnp-component-cache-{}-{nanos}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("probe.wasm");
        // The smallest valid component is not needed: a compile failure is
        // not cached, so this test exercises the miss path without asserting
        // on a valid artifact.
        std::fs::write(&path, b"not a component").expect("write probe");
        let first = cached_component(false, &path, b"not a component");
        let second = cached_component(false, &path, b"not a component");
        assert!(
            first.is_err() && second.is_err(),
            "invalid bytes must fail both times"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
