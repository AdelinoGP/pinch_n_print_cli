//! Host-side accounting allocator.
//!
//! `AccountingAllocator<A>` wraps any `GlobalAlloc` and, when the `ENABLED`
//! flag is set, records per-thread bytes-in-use deltas around the currently
//! active bracket scope. The collector pushes a scope key (`u32`) onto the
//! thread-local `CTX` slot at bracket start and reads the accumulated delta
//! at bracket end.
//!
//! When disabled (the default), the fast path is a single relaxed atomic
//! load per allocation, then forwarding to the wrapped allocator — there is
//! no locking and no map lookup.
//!
//! WASM linear memory is **not** measured here. wasmtime allocates linear
//! memory via mmap directly, bypassing `GlobalAlloc`. v1 of the report
//! sets `wasm_delta = 0`; see `docs/16_slicer_report.md`.

use std::alloc::{GlobalAlloc, Layout};
use std::cell::Cell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU32, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

/// Global enable flag. Default off; set to `true` by `enable()` before any
/// bracket-pushing happens. When false, all `alloc`/`dealloc` paths reduce
/// to one relaxed atomic load and a forward to `inner`.
static ENABLED: AtomicBool = AtomicBool::new(false);

/// Monotonically increasing scope-key generator.
static NEXT_KEY: AtomicU32 = AtomicU32::new(1);

/// Live total bytes in use across all scopes (debug / smoke metric).
static TOTAL_BYTES: AtomicI64 = AtomicI64::new(0);

/// Process-wide peak bytes observed (informational).
static PEAK_BYTES: AtomicU64 = AtomicU64::new(0);

thread_local! {
    /// Current bracket scope on this thread, or `None` if none is active.
    /// Stored as a stack so nested brackets (stage → module) restore the
    /// parent on pop.
    static CTX_STACK: Cell<Option<Vec<u32>>> = const { Cell::new(None) };
    /// Re-entrancy guard: when account()'s hashmap path itself allocates,
    /// recursive entries must short-circuit so we don't deadlock the
    /// stats mutex or stack-overflow.
    static IN_ACCOUNTING: Cell<bool> = const { Cell::new(false) };
}

/// Per-scope statistics flushed at scope close.
#[derive(Debug, Clone, Copy, Default)]
pub struct MemStats {
    /// Current bytes in use (signed; may dip negative if pre-scope
    /// allocations are freed inside the scope).
    pub current: i64,
    /// Peak bytes in use observed during the scope's lifetime.
    pub peak: u64,
    /// Sum of all allocation requests (gross, including ones later freed).
    pub total_alloc: u64,
}

fn stats_map() -> &'static Mutex<HashMap<u32, MemStats>> {
    static MAP: OnceLock<Mutex<HashMap<u32, MemStats>>> = OnceLock::new();
    MAP.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Allocator wrapper. Set `#[global_allocator]` to an instance of this in
/// each binary that wants accounting.
pub struct AccountingAllocator<A> {
    inner: A,
}

impl<A> AccountingAllocator<A> {
    /// Wrap an underlying allocator.
    pub const fn new(inner: A) -> Self {
        Self { inner }
    }
}

unsafe impl<A: GlobalAlloc> GlobalAlloc for AccountingAllocator<A> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { self.inner.alloc(layout) };
        if !ptr.is_null() && ENABLED.load(Ordering::Relaxed) {
            account(layout.size() as i64);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if ENABLED.load(Ordering::Relaxed) {
            account(-(layout.size() as i64));
        }
        unsafe { self.inner.dealloc(ptr, layout) };
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { self.inner.alloc_zeroed(layout) };
        if !ptr.is_null() && ENABLED.load(Ordering::Relaxed) {
            account(layout.size() as i64);
        }
        ptr
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_ptr = unsafe { self.inner.realloc(ptr, layout, new_size) };
        if !new_ptr.is_null() && ENABLED.load(Ordering::Relaxed) {
            let delta = new_size as i64 - layout.size() as i64;
            if delta != 0 {
                account(delta);
            }
        }
        new_ptr
    }
}

fn account(delta: i64) {
    // Re-entrancy guard: if account() itself allocates (HashMap growth,
    // Mutex internals), recursive entries fall through without touching
    // shared state to avoid deadlock.
    let guarded = IN_ACCOUNTING.with(|cell| {
        if cell.get() {
            return false;
        }
        cell.set(true);
        true
    });
    if !guarded {
        return;
    }

    let new_total = TOTAL_BYTES.fetch_add(delta, Ordering::Relaxed) + delta;
    if delta > 0 {
        let nt = new_total.max(0) as u64;
        PEAK_BYTES.fetch_max(nt, Ordering::Relaxed);
    }

    // Attribute to current scope (if any). Reading the stack via take/set
    // dance avoids holding the Cell while we look up the mutex.
    CTX_STACK.with(|cell| {
        let stack = cell.take();
        if let Some(stack_vec) = stack {
            if let Some(&key) = stack_vec.last() {
                if let Ok(mut map) = stats_map().lock() {
                    let entry = map.entry(key).or_default();
                    entry.current += delta;
                    if delta > 0 {
                        entry.total_alloc += delta as u64;
                        if entry.current > 0 && (entry.current as u64) > entry.peak {
                            entry.peak = entry.current as u64;
                        }
                    }
                }
            }
            cell.set(Some(stack_vec));
        }
    });

    IN_ACCOUNTING.with(|cell| cell.set(false));
}

/// Enable accounting. Idempotent.
pub fn enable() {
    ENABLED.store(true, Ordering::Relaxed);
}

/// Disable accounting. Idempotent. Existing scopes remain in the stats map
/// but no further deltas will be recorded.
pub fn disable() {
    ENABLED.store(false, Ordering::Relaxed);
}

/// Is accounting currently enabled?
pub fn is_enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// Process-wide peak observed bytes in use.
pub fn peak_bytes() -> u64 {
    PEAK_BYTES.load(Ordering::Relaxed)
}

/// Open a new scope. Returns the scope key; pass it to [`pop_scope`] when
/// the bracket closes. Nested scopes are supported (stack semantics).
pub fn push_scope() -> u32 {
    let key = NEXT_KEY.fetch_add(1, Ordering::Relaxed);
    CTX_STACK.with(|cell| {
        let mut stack = cell.take().unwrap_or_default();
        stack.push(key);
        cell.set(Some(stack));
    });
    key
}

/// Close a scope and return its accumulated stats. Returns
/// `MemStats::default()` if no scope was open or if accounting was
/// disabled at scope time.
pub fn pop_scope(key: u32) -> MemStats {
    CTX_STACK.with(|cell| {
        let mut stack = cell.take().unwrap_or_default();
        // Defensive: only pop if the top matches; out-of-order pops are a bug.
        if stack.last() == Some(&key) {
            stack.pop();
        }
        if stack.is_empty() {
            cell.set(None);
        } else {
            cell.set(Some(stack));
        }
    });
    if let Ok(mut map) = stats_map().lock() {
        map.remove(&key).unwrap_or_default()
    } else {
        MemStats::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enable_disable_round_trip() {
        assert!(!is_enabled());
        enable();
        assert!(is_enabled());
        disable();
        assert!(!is_enabled());
    }

    #[test]
    fn push_pop_returns_default_when_disabled() {
        // ENABLED is false by default — even with a scope open, no deltas
        // are recorded.
        let key = push_scope();
        let s = pop_scope(key);
        assert_eq!(s.current, 0);
        assert_eq!(s.peak, 0);
        assert_eq!(s.total_alloc, 0);
    }

    #[test]
    fn nested_scopes_pop_in_lifo_order() {
        let a = push_scope();
        let b = push_scope();
        // Pop in reverse order.
        let _ = pop_scope(b);
        let _ = pop_scope(a);
    }
}
