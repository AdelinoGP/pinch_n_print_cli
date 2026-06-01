//! `slicer_sdk::host::log_warn` must round-trip through the per-thread log
//! capture sink when one is installed: the message lands in
//! `take_log_messages()`, and `take_log_messages()` drains the buffer so a
//! second call returns empty.

fn reset() {
    slicer_sdk::test_support::reset_global_state();
}

#[test]
fn log_warn_round_trips_through_capture_when_installed() {
    reset();

    // Install the per-thread log capture sink (same call #[module_test]'s
    // mock_host_setup makes).
    slicer_sdk::host::test_support::install_log_capture();

    // Emit via the real host wrapper.
    slicer_sdk::host::log_warn("hello-marker");

    // Drain — must contain our marker message at Warn level.
    let drained = slicer_sdk::host::test_support::take_log_messages();
    assert!(
        drained
            .iter()
            .any(|(level, msg)| *level == slicer_sdk::host::LogLevel::Warn
                && msg.contains("hello-marker")),
        "expected captured warn containing 'hello-marker'; got {:?}",
        drained
    );

    // Re-install capture and demonstrate drain semantics: a fresh
    // take_log_messages with no intervening log_warn returns empty.
    slicer_sdk::host::test_support::install_log_capture();
    let drained_again = slicer_sdk::host::test_support::take_log_messages();
    assert!(
        drained_again.is_empty(),
        "fresh capture sink with no logs must drain empty; got {:?}",
        drained_again
    );
}
