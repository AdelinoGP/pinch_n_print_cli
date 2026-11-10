//! Opt-in native property-test execution and per-invocation SDK isolation.

use std::cell::Cell;
use std::marker::PhantomData;
use std::panic::{catch_unwind, AssertUnwindSafe};

use slicer_ir::LayerEntityIdGen;

use crate::host::{self, LogLevel};

use super::capture::{InfillOutputCapture, PerimeterOutputCapture, SupportOutputCapture};
use super::mock_host::MockHost;
use super::{mock_host_setup, mock_host_teardown, reset_global_state};

pub mod domains;
mod oracle;
pub mod oracles;
pub mod regression;
mod run;

pub use oracle::{check_travel_anchors_charter, travel_anchor_oracle, AnchorOracleError};
pub use run::{PropertyRun, PropertyRunError};

thread_local! {
    static CASE_ACTIVE: Cell<bool> = const { Cell::new(false) };
    static RETAINED_LOGS: Cell<Vec<(LogLevel, String)>> = const { Cell::new(Vec::new()) };
}

/// Owned fixtures and thread-local SDK state for one property invocation.
///
/// Dropping the guard retains available logs before clearing the real host
/// seams, including during unwinding. Guards cannot move or be shared across
/// threads; each thread owns its own active guard and diagnostic slot.
pub struct CaseGuard {
    entity_ids: LayerEntityIdGen,
    infill: InfillOutputCapture,
    perimeters: PerimeterOutputCapture,
    support: SupportOutputCapture,
    _not_send_sync: PhantomData<*mut ()>,
}

impl CaseGuard {
    /// Reset this thread's host state and allocate fresh case-local fixtures.
    ///
    /// # Panics
    /// Panics on same-thread nesting, before changing the outer case's state.
    #[must_use]
    pub fn new() -> Self {
        CASE_ACTIVE.with(|active| {
            assert!(!active.get(), "nested CaseGuard on the same thread");
            active.set(true);
        });

        let guard = Self {
            entity_ids: LayerEntityIdGen::new(),
            infill: InfillOutputCapture::new(),
            perimeters: PerimeterOutputCapture::new(),
            support: SupportOutputCapture::new(),
            _not_send_sync: PhantomData,
        };
        // Construct the guard first so even a panicking old mesh-source
        // destructor releases the active-case slot through Drop.
        reset_global_state();
        mock_host_setup();
        guard
    }

    /// Install a mock into the real thread-local host mesh-source seam.
    pub fn install_mock(&mut self, host: MockHost) {
        host.install();
    }

    /// Borrow the fresh, case-owned entity-ID allocator.
    #[must_use]
    pub fn entity_ids(&self) -> &LayerEntityIdGen {
        &self.entity_ids
    }

    /// Borrow this invocation's infill capture.
    pub fn infill(&mut self) -> &mut InfillOutputCapture {
        &mut self.infill
    }

    /// Borrow this invocation's perimeter capture.
    pub fn perimeters(&mut self) -> &mut PerimeterOutputCapture {
        &mut self.perimeters
    }

    /// Borrow this invocation's support capture.
    pub fn support(&mut self) -> &mut SupportOutputCapture {
        &mut self.support
    }
}

impl Default for CaseGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for CaseGuard {
    fn drop(&mut self) {
        // Isolate each teardown operation: no cleanup panic may escape Drop,
        // including when a case is already unwinding. No panic hooks change.
        let _ = catch_unwind(AssertUnwindSafe(|| {
            let logs = host::test_support::take_log_messages();
            let _ = RETAINED_LOGS.try_with(|slot| {
                let mut retained = slot.take();
                retained.extend(logs);
                slot.set(retained);
            });
        }));
        let _ = catch_unwind(AssertUnwindSafe(mock_host_teardown));
        let _ = CASE_ACTIVE.try_with(|active| active.set(false));
    }
}

/// Return and clear this thread's retained case logs, including unwound cases.
///
/// Logs remain available across case entry until consumed. Draining the live
/// host sink yourself removes those messages from what teardown can retain.
#[must_use]
pub fn take_case_logs() -> Vec<(LogLevel, String)> {
    RETAINED_LOGS.with(Cell::take)
}
