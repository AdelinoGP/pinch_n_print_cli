//! Process-local cache for `load_model` outputs, keyed by canonical fixture
//! path. First caller through pays the parse cost; later callers receive a
//! clone of the cached `Arc<MeshIR>`.
//!
//! 3MF fixtures in the e2e bucket (`benchy_4color.3mf` at 2.6 MB,
//! `benchy_painted.3mf` at 2.5 MB) are otherwise parsed once per test that
//! touches them. Sharing the parsed `MeshIR` across tests in the same process
//! removes that redundant work.
//!
//! Failure-asserting tests (e.g. `missing_fixture_returns_error`) MUST NOT
//! route through this cache — they exercise the loader's error path and need
//! the real `load_model` invocation.

#![allow(dead_code)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use slicer_ir::MeshIR;
use slicer_runtime::model_loader::load_model;

type Cell = Arc<OnceLock<Arc<MeshIR>>>;
static CACHE: OnceLock<Mutex<HashMap<PathBuf, Cell>>> = OnceLock::new();

/// Returns the cached `Arc<MeshIR>` for `path`, parsing on first call.
/// Panics if `load_model` fails — symmetrical to the per-test `.expect(...)`
/// in today's call sites.
pub fn cached_load_model(path: &Path) -> Arc<MeshIR> {
    let key = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let cell: Cell = {
        let mut map = CACHE
            .get_or_init(|| Mutex::new(HashMap::new()))
            .lock()
            .expect("model_cache mutex");
        map.entry(key.clone())
            .or_insert_with(|| Arc::new(OnceLock::new()))
            .clone()
    };
    cell.get_or_init(|| {
        Arc::new(
            load_model(&key)
                .unwrap_or_else(|e| panic!("cached_load_model({}) failed: {e:?}", key.display())),
        )
    })
    .clone()
}
