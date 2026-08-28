//! Allocation of invocation-local extrusion order-lock tags.

use slicer_ir::ExtrusionPath3D;

const GLOBAL_BIT: u64 = 1 << 63;

/// Allocates deterministic local order-lock tags.
pub struct OrderLockAllocator {
    next: u64,
}

impl OrderLockAllocator {
    /// Creates an allocator whose first tag is one.
    pub fn new() -> Self {
        Self { next: 1 }
    }

    /// Creates an allocator at a specific next tag for boundary testing.
    #[doc(hidden)]
    pub fn from_next(next: u64) -> Self {
        Self { next }
    }

    /// Allocates the next local tag, or `None` when the local space is exhausted.
    pub fn allocate(&mut self) -> Option<u64> {
        if self.next >= 1 << 63 {
            return None;
        }
        let tag = self.next;
        self.next += 1;
        Some(tag)
    }
}

impl Default for OrderLockAllocator {
    fn default() -> Self {
        Self::new()
    }
}

/// Rewrites local order-lock tags to global tags for a single output boundary.
pub fn remap_order_locks_to_global(
    paths: &mut [ExtrusionPath3D],
    next_global: &mut u64,
) -> Result<(), String> {
    for path in paths {
        let Some(tag) = path.order_lock else {
            continue;
        };

        if tag & GLOBAL_BIT == 0 {
            if tag == 0 {
                return Err("order-lock tag 0 is invalid".to_string());
            }
            path.order_lock = Some(GLOBAL_BIT | *next_global);
            *next_global += 1;
        } else if tag >= GLOBAL_BIT | *next_global {
            return Err(format!("unknown global tag {tag}"));
        }
    }
    Ok(())
}
