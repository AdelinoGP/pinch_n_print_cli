//! AC-7 — `slicer_sdk::test_support::install_panic_handler` chains: on a
//! panic inside a `catch_unwind` it (a) drains the captured log buffer to
//! stderr, and (b) delegates to the previously-registered panic hook.
//!
//! The panic hook is a process-wide global in Rust, so this test is the only
//! `#[test]` in its binary — cargo runs it single-threaded by default, so
//! parallel tests cannot clobber the hook chain.

use std::cell::Cell;
use std::panic::{catch_unwind, set_hook, take_hook, AssertUnwindSafe};

thread_local! {
    static PRIOR_HOOK_CALLED: Cell<bool> = const { Cell::new(false) };
}

#[test]
fn panic_inside_module_test_drains_log_buffer_to_stderr() {
    // Reset SDK state at entry (mirrors #[module_test]).
    slicer_sdk::test_support::reset_global_state();

    // Install log capture so log_warn lands in the buffer.
    slicer_sdk::host::test_support::install_log_capture();

    // 1. Install a "prior" panic hook that flips a thread-local flag. The
    //    flag verifies that install_panic_handler chains through to us.
    PRIOR_HOOK_CALLED.with(|f| f.set(false));
    // Swap out the default hook (discarded — we install our own prior hook).
    let _discarded_default = take_hook();
    set_hook(Box::new(|_info| {
        PRIOR_HOOK_CALLED.with(|f| f.set(true));
    }));

    // 2. Install the chained panic handler from test_support. It captures
    //    the prior hook we just installed and must delegate to it.
    slicer_sdk::test_support::install_panic_handler();

    // 3. Inside catch_unwind, emit a captured log then panic.
    let result = catch_unwind(AssertUnwindSafe(|| {
        slicer_sdk::host::log_warn("BUG_MARKER_42");
        panic!("expected-panic-for-test");
    }));

    // 4a. Panic was caught.
    assert!(result.is_err(), "panic should have been caught");

    // 4b. The prior hook ran — proves install_panic_handler chained
    //     through, not replaced.
    assert!(
        PRIOR_HOOK_CALLED.with(|f| f.get()),
        "prior panic hook must have been called (chain preserved)"
    );

    // 4c. The capture buffer was drained by the panic hook. A fresh
    //     take_log_messages must return empty (nothing left over).
    let remaining = slicer_sdk::host::test_support::take_log_messages();
    assert!(
        remaining.is_empty(),
        "panic hook should have drained the log buffer; got {:?}",
        remaining
    );

    // Restore default hook so we don't leak a custom hook into any later
    // test binaries (defense in depth — this binary has only one test).
    let _ = take_hook();
}
