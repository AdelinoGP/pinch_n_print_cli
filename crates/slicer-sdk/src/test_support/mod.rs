#![cfg(any(test, feature = "test"))]
//! Test hooks wired by `#[module_test]`. Wrap the per-thread seams in
//! [`crate::host::test_support`] so generated test harnesses can reset
//! state, install a panic hook that drains captured logs, and set up /
//! tear down a mocked host context.

use crate::host;

/// Drop any captured logs and uninstall any per-thread mesh source.
pub fn reset_global_state() {
    let _ = host::test_support::take_log_messages();
    host::test_support::clear_mesh_source();
}

/// Install a panic hook that drains captured log messages to stderr
/// before delegating to the previously-registered hook. The previous
/// hook is preserved via [`std::panic::take_hook`] and invoked after
/// the drain.
pub fn install_panic_handler() {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let drained = host::test_support::take_log_messages();
        for (level, msg) in drained {
            eprintln!("[captured {}] {}", level.as_str(), msg);
        }
        prev(info);
    }));
}

/// Install the per-thread log capture sink used by mocked test runs.
pub fn mock_host_setup() {
    host::test_support::install_log_capture();
}

/// Tear down the mocked host: drain any captured logs and uninstall
/// the per-thread mesh source.
pub fn mock_host_teardown() {
    let _ = host::test_support::take_log_messages();
    host::test_support::clear_mesh_source();
}

pub mod assert_paths;
pub mod capture;
pub mod fixtures;
pub mod mock_host;
