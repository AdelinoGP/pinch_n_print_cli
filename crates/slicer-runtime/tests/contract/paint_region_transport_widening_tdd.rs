//! TDD anchoring tests for Packet 42 — paint-region transport widening (host side).
//!
//! Module layout:
//!   - `doc_grep_tests` — file-string-grep tests; no new WIT/IR types needed;
//!     compile and run RIGHT NOW.
//!   - `transport_round_trip_tests` — end-to-end tests that reference
//!     `pm::PaintValueInput`, `PaintValue::Custom`, and the typed
//!     `paint-value-input` WIT variant on `pm::PaintRegionEntry`.
//!
//! The round-trip module is gated behind `#[cfg(feature = "transport_widened")]`
//! so the grep tests compile and run even when the new types don't exist yet.
//! The compile failure on `--features slicer-runtime/transport_widened` IS the RED
//! state for the unimplemented parts.
//!
//! `doc_grep_tests` must be GREEN after Step 1 docs are committed.

// ── doc_grep_tests ────────────────────────────────────────────────────────────
mod doc_grep_tests {
    use std::path::Path;

    // ── AC-host-7 ─────────────────────────────────────────────────────────────
    /// `docs/07_implementation_status.md` must contain a row for TASK-130c
    /// titled "Widen paint-region transport" and must list TASK-130c in the
    /// blocker list.
    ///
    /// GREEN after Step 1.
    #[test]
    fn docs_07_registers_task_130c() {
        let path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/07_implementation_status.md");
        let src = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read 07_implementation_status.md: {e}"));

        // (a) A line must contain both TASK-130c and "Widen paint-region transport"
        let has_task_row = src
            .lines()
            .any(|l| l.contains("TASK-130c") && l.contains("Widen paint-region transport"));
        assert!(
            has_task_row,
            "07_implementation_status.md must have a line with TASK-130c and \
             'Widen paint-region transport'"
        );

        // (b) A line must reference TASK-130c's relationship to a blocker.
        // The task can be registered as either an open blocker
        // (`Blocking`/`blocker`) OR as the closure of a blocker
        // (`Closed`/`closed`/`Covers DEV-025`). After packet 42 the task
        // was closed; either form still satisfies the registration contract.
        let has_blocker_or_closure = src.lines().any(|l| {
            l.contains("TASK-130c")
                && (l.contains("Blocking")
                    || l.contains("blocker")
                    || l.contains("Closed")
                    || l.contains("closed")
                    || l.contains("DEV-025"))
        });
        assert!(
            has_blocker_or_closure,
            "07_implementation_status.md must reference TASK-130c as a blocker or its closure"
        );
    }

    // AC-host-8 (`dev_log_extends_dev025_with_4_and_5`) asserted DEVIATION_LOG.md's
    // DEV-025 row content directly; removed 2026-07-02 when DEV-025 was deleted
    // from the log as a closed entry (history preserved in git).
}
