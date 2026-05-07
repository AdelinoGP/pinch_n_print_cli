//! Per-layer entity identifier generator.

use std::cell::Cell;
use std::marker::PhantomData;

/// Per-layer monotonic identifier generator. Single-threaded by design
/// (`!Send + !Sync` via `Cell<u64>` and `PhantomData<*const ()>`) since layer
/// construction is per-layer single-threaded per the host scheduler model.
#[derive(Debug)]
pub struct LayerEntityIdGen {
    next: Cell<u64>,
    /// Marker that makes this type `!Send + !Sync` unconditionally.
    _not_send_sync: PhantomData<*const ()>,
}

impl Default for LayerEntityIdGen {
    fn default() -> Self {
        Self::new()
    }
}

impl LayerEntityIdGen {
    /// Creates a new generator. The first ID issued will be `1`.
    pub fn new() -> Self {
        Self {
            next: Cell::new(1),
            _not_send_sync: PhantomData,
        }
    }

    /// Returns a fresh strictly-monotonic ID starting at 1.
    pub fn next(&self) -> u64 {
        let id = self.next.get().max(1);
        self.next.set(id + 1);
        id
    }
}
