//! Human-channel de-duplication for module logs.
//!
//! `forward_module_logs` (`crates/slicer-wasm-host/src/dispatch.rs`) collapses
//! identical `(level, message)` pairs to a single `log`-facade emission and
//! defers the occurrence count to `emit_module_log_repeat_summary`, which
//! `run_slice` calls once per slice. Without that, a module that warns once per
//! call can put tens of thousands of identical lines on stderr and bury every
//! other diagnostic.
//!
//! This lives in its own test binary because it installs a process-global
//! `log` implementation (`log::set_logger` succeeds at most once per process),
//! which must not leak into the shared `contract` / `integration` / `unit`
//! harnesses.

use std::sync::Mutex;

use slicer_wasm_host::dispatch::{emit_module_log_repeat_summary, forward_module_logs};

static CAPTURED: Mutex<Vec<String>> = Mutex::new(Vec::new());

struct CaptureLogger;

impl log::Log for CaptureLogger {
    fn enabled(&self, _metadata: &log::Metadata<'_>) -> bool {
        true
    }
    fn log(&self, record: &log::Record<'_>) {
        CAPTURED
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .push(format!(
                "{}|{}|{}",
                record.level(),
                record.target(),
                record.args()
            ));
    }
    fn flush(&self) {}
}

static CAPTURE_LOGGER: CaptureLogger = CaptureLogger;

fn install_capture_logger() {
    log::set_logger(&CAPTURE_LOGGER).expect("no other logger in this test binary");
    log::set_max_level(log::LevelFilter::Trace);
}

fn captured() -> Vec<String> {
    CAPTURED
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .iter()
        .cloned()
        .collect()
}

#[test]
fn identical_records_emit_once_and_report_their_count_at_slice_end() {
    install_capture_logger();

    let hot = "clipper retry on degenerate contour".to_string();
    let unique_a = "first-of-a-kind A".to_string();
    let unique_b = "first-of-a-kind B".to_string();

    // 5 identical warns interleaved with two one-off records, exactly as a hot
    // per-layer module would produce them.
    let mut batch = Vec::new();
    for _ in 0..5 {
        batch.push(("warn".to_string(), hot.clone()));
    }
    batch.push(("info".to_string(), unique_a.clone()));
    batch.push(("error".to_string(), unique_b.clone()));
    forward_module_logs("com.core.hot-module", &batch);

    let during_slice = captured();
    assert_eq!(
        during_slice
            .iter()
            .filter(|line| line.contains(&hot))
            .count(),
        1,
        "the human channel must collapse the 5 identical warns to one line: {during_slice:?}"
    );
    assert_eq!(
        during_slice
            .iter()
            .filter(|line| line.contains(&unique_a))
            .count(),
        1,
        "a record seen once is emitted verbatim"
    );
    assert_eq!(
        during_slice
            .iter()
            .filter(|line| line.contains(&unique_b))
            .count(),
        1
    );
    assert!(
        during_slice
            .iter()
            .any(|line| line.starts_with("WARN|slicer_module::com.core.hot-module|")),
        "level and per-module target are preserved: {during_slice:?}"
    );
    assert!(
        !during_slice.iter().any(|line| line.contains("repeated")),
        "the count is reported at slice end, not inline"
    );

    // Slice end: the 4 suppressed repeats are accounted for, at the level the
    // original record carried so RUST_LOG filtering behaves identically.
    emit_module_log_repeat_summary();

    let summary: Vec<String> = captured()
        .into_iter()
        .filter(|line| line.contains("repeated"))
        .collect();
    assert_eq!(
        summary.len(),
        1,
        "only the repeated pair is summarized: {summary:?}"
    );
    assert!(summary[0].starts_with("WARN|slicer_module::com.core.hot-module|"));
    assert!(summary[0].contains(&hot));
    assert!(
        summary[0].contains("repeated 4 more time(s)"),
        "5 occurrences, 1 emitted, 4 suppressed: {}",
        summary[0]
    );

    // The table is drained, so a second slice in the same process starts clean.
    emit_module_log_repeat_summary();
    assert_eq!(
        captured()
            .into_iter()
            .filter(|line| line.contains("repeated"))
            .count(),
        1,
        "draining the table must not re-report the same repeat"
    );
}
