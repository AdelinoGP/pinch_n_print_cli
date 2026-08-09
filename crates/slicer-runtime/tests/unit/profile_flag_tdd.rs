//! `--profile` must be strictly opt-in (ADR-0055).
//!
//! The plumbing added for fuel profiling touches every executor drop-site and
//! the shared `WasmEngine` construction, so the failure mode worth guarding is
//! not "profiling is broken" but "profiling leaked into a run that did not ask
//! for it". Fuel metering costs throughput and the extra fields would widen the
//! JSONL stream for every consumer, so a run with `profile: false` must be
//! observably identical to one from before the feature existed.
//!
//! This is an end-to-end assertion against a real slice rather than a
//! seam-level one: the seams are unit-tested next to their own code, and what
//! they cannot show is that no *other* path (the loader, the native
//! `slicer-core` sink, the report fork) switched profiling on behind the flag's
//! back.

#![allow(missing_docs)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use slicer_runtime::progress_events::{ProgressEventType, SliceEventCollector};
use slicer_runtime::run::run_slice_with_collector;
use slicer_runtime::SliceRunOptions;

fn workspace_root() -> PathBuf {
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set by cargo test");
    PathBuf::from(manifest_dir)
        .join("..")
        .join("..")
        .canonicalize()
        .expect("workspace root must be resolvable")
}

fn unprofiled_options() -> SliceRunOptions {
    let root = workspace_root();
    let model = root.join("resources").join("regression_wedge.stl");
    let mesh = Arc::new(slicer_model_io::load_model(&model).expect("wedge fixture must load"));
    // exhaustive: SliceRunOptions fixture intentionally supplies explicit profiling options
    SliceRunOptions {
        mesh,
        model_label: model.to_string_lossy().into_owned(),
        config_path: None,
        output_path: None,
        module_dirs: vec![root.join("modules").join("core-modules")],
        no_default_module_paths: true,
        thumbnail: None,
        report: None,
        report_verbose: false,
        // The instrumented tier is the widest stream a non-profiled run can
        // produce, so it is the strongest place to assert the absence of
        // profiling output.
        instrument_stderr: true,
        profile: false,
        profile_verbose: false,
        progress_events: true,
        cancel_flag: None,
        config_overrides: HashMap::new(),
    }
}

/// The positive half: `--profile` must reach both guests and host built-ins.
///
/// Deliberately does not assert that guest rows have a *scope* breakdown: those
/// come from marks the guest emits, which depends on whether the SDK it was
/// built against installs the profiling sink. Module-granular fuel does not —
/// it is metered by the store, so it is available the moment the flag exists.
/// Asserting the part that holds regardless keeps this test from going red on
/// an unrelated SDK change.
///
/// The host-built-in half is where the unit split matters: native code is not
/// metered, so those rows must be tagged wall-clock and kept out of the fuel
/// denominator.
#[test]
fn profile_on_reports_host_builtins_in_wall_clock_units() {
    let mut opts = unprofiled_options();
    opts.profile = true;

    let collector = Arc::new(Mutex::new(SliceEventCollector::new()));
    let outcome = run_slice_with_collector(opts, Some(Arc::clone(&collector)))
        .expect("the wedge fixture must slice successfully");

    let summary = outcome
        .profile
        .as_ref()
        .expect("--profile must produce a summary");

    let events = collector
        .lock()
        .expect("progress collector must not be poisoned")
        .events()
        .to_vec();
    let emitted: Vec<_> = events
        .iter()
        .filter(|e| e.event == ProgressEventType::ProfileSummary)
        .collect();
    assert_eq!(
        emitted.len(),
        1,
        "exactly one profile_summary per profiled slice"
    );
    assert_eq!(
        emitted[0].profile.as_ref(),
        Some(summary),
        "the emitted event and the returned summary must be the same fold"
    );

    // `profile_summary` must precede the terminal events, so a consumer that
    // stops at `slice_complete` never misses it.
    let summary_at = events
        .iter()
        .position(|e| e.event == ProgressEventType::ProfileSummary)
        .expect("profile_summary present");
    let complete_at = events
        .iter()
        .position(|e| e.event == ProgressEventType::SliceComplete)
        .expect("slice_complete present");
    assert!(summary_at < complete_at, "{summary_at} !< {complete_at}");

    let native: Vec<&slicer_runtime::ProfileModuleRow> = summary
        .modules
        .iter()
        .filter(|m| m.unit == slicer_runtime::ProfileUnit::WallNs)
        .collect();
    assert!(
        !native.is_empty(),
        "host built-ins call marked polygon_ops; the native sink must see them: {summary:?}"
    );
    for row in &native {
        assert_eq!(
            row.total_fuel, 0,
            "native code is not fuel-metered — a non-zero fuel figure would be fabricated"
        );
        assert!(
            row.total_wall_ns > 0,
            "a wall row with no time is not a row"
        );
        assert!(
            row.scopes
                .iter()
                .all(|s| s.scope.starts_with("polygon_ops::")),
            "host built-ins must report under the same vocabulary as guests: {:?}",
            row.scopes
        );
    }
    // Guest rows: metered by the store, so present even with no scope marks.
    let fuel_rows: Vec<&slicer_runtime::ProfileModuleRow> = summary
        .modules
        .iter()
        .filter(|m| m.unit == slicer_runtime::ProfileUnit::Fuel)
        .collect();
    assert!(
        !fuel_rows.is_empty(),
        "enabling fuel metering must yield per-module fuel totals: {summary:?}"
    );
    for row in &fuel_rows {
        assert!(row.total_fuel > 0, "a fuel row with no fuel is not a row");
        assert!(row.calls > 0);
    }

    // The two denominators must stay disjoint: a wall row must never inflate
    // the fuel base, and vice versa. That is the whole reason rows are tagged.
    assert_eq!(
        summary.fuel_total,
        fuel_rows.iter().map(|r| r.total_fuel).sum::<u64>()
    );
    assert_eq!(
        summary.wall_total_ns,
        native.iter().map(|r| r.total_wall_ns).sum::<u64>()
    );
    assert!(summary.wall_total_ns > 0);

    // The rendered table is what a human reads; it must name both units.
    let rendered = slicer_runtime::format_profile_summary(summary);
    assert!(rendered.contains("units: fuel"), "{rendered}");
    assert!(rendered.contains("NOT fuel-metered"), "{rendered}");
    assert!(rendered.contains("indicative only"), "{rendered}");
}

#[test]
fn profile_off_emits_no_profile_summary_and_returns_no_summary() {
    let collector = Arc::new(Mutex::new(SliceEventCollector::new()));
    let outcome = run_slice_with_collector(unprofiled_options(), Some(Arc::clone(&collector)))
        .expect("the wedge fixture must slice successfully");

    assert!(
        outcome.profile.is_none(),
        "a run without --profile must not synthesise a summary"
    );

    let events = collector
        .lock()
        .expect("progress collector must not be poisoned")
        .events()
        .to_vec();
    assert!(
        !events.is_empty(),
        "the instrumented stream must still carry its usual events"
    );
    assert!(
        events
            .iter()
            .any(|e| e.event == ProgressEventType::ModuleComplete),
        "guard integrity: without module_complete events the absence checks below prove nothing"
    );
    assert!(
        events
            .iter()
            .all(|e| e.event != ProgressEventType::ProfileSummary),
        "no profile_summary event may appear without --profile"
    );
    assert!(
        events.iter().all(|e| e.profile.is_none()),
        "no event may carry a profile payload without --profile"
    );
    assert!(
        events.iter().all(|e| e.profile_scopes.is_none()),
        "no module_complete may carry per-call scopes without --profile-verbose"
    );

    // Serialised form is what consumers actually read: the two new keys must be
    // absent from the wire, not merely `null`.
    for event in &events {
        let json = serde_json::to_string(event).expect("every event must serialize");
        assert!(!json.contains("\"profile\""), "{json}");
        assert!(!json.contains("\"profile_scopes\""), "{json}");
    }
}
