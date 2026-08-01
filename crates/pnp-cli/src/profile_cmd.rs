//! `pnp_cli profile --from <events.jsonl>` — re-summarise a captured
//! progress-event stream without re-slicing (ADR-0055).
//!
//! A profiled slice is expensive; re-running one to look at the numbers a
//! second time is the mistake this command exists to prevent. It is the same
//! ergonomic as `cargo xtask test --summary-from`: the work already happened
//! and landed on disk, so read it.
//!
//! The command is deliberately tolerant of everything a capture picks up
//! besides events — interleaved `warning:` lines from the runtime's stderr
//! diagnostics, `env_logger` output, blank lines. It scans for the one line
//! that is a `profile_summary` event and ignores the rest, so
//! `pnp_cli slice --profile … 2> events.jsonl` can be fed in verbatim.

use std::io::Read;
use std::path::Path;

use slicer_runtime::progress_events::{ProgressEvent, ProgressEventType};
use slicer_runtime::{format_profile_summary, ProfileSummary};

/// Read `path` (or stdin when it is `-`) and render its `profile_summary`.
///
/// With `json`, emits the summary payload as pretty JSON instead of the ranked
/// table — the machine-readable half of the same answer.
///
/// # Errors
///
/// Returns a human-readable message when the capture cannot be read or carries
/// no `profile_summary` event.
pub fn run_profile_from(path: &Path, json: bool) -> Result<String, String> {
    let text = read_capture(path)?;
    let summary = extract_profile_summary(&text)?;
    if json {
        return serde_json::to_string_pretty(&summary)
            .map(|s| format!("{s}\n"))
            .map_err(|e| format!("failed to serialise profile summary: {e}"));
    }
    Ok(format_profile_summary(&summary))
}

fn read_capture(path: &Path) -> Result<String, String> {
    if path.as_os_str() == "-" {
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .map_err(|e| format!("failed to read progress events from stdin: {e}"))?;
        return Ok(buf);
    }
    std::fs::read_to_string(path).map_err(|e| format!("failed to read {}: {e}", path.display()))
}

/// Pull the single `profile_summary` payload out of a JSONL capture.
///
/// Later events win if a capture concatenates several runs: the last summary is
/// the most recent one, which is what a reader asking "how did that go" means.
///
/// # Errors
///
/// Returns a message naming the likely cause when no summary is present —
/// almost always a capture taken without `--profile`, which is worth saying out
/// loud rather than reporting an empty table.
pub fn extract_profile_summary(capture: &str) -> Result<ProfileSummary, String> {
    let mut found: Option<ProfileSummary> = None;
    let mut saw_any_event = false;
    for line in capture.lines() {
        let line = line.trim();
        // Cheap pre-filter: a capture is mostly non-summary lines, and
        // attempting a full parse on each would be both slow and noisy.
        if !line.starts_with('{') {
            continue;
        }
        let Ok(event) = serde_json::from_str::<ProgressEvent>(line) else {
            continue;
        };
        saw_any_event = true;
        if event.event == ProgressEventType::ProfileSummary {
            if let Some(summary) = event.profile {
                found = Some(summary);
            }
        }
    }
    found.ok_or_else(|| {
        if saw_any_event {
            "no profile_summary event in this capture — it was recorded without --profile"
                .to_string()
        } else {
            "no progress events found; expected the JSONL stream from \
             `pnp_cli slice --profile` (stderr)"
                .to_string()
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use slicer_runtime::{MarkEdge, ProfileAggregator, ProfileMark, ProfileUnit};

    fn mark(scope: u32, enter: bool, fuel: u64, wall_ns: u64) -> ProfileMark {
        ProfileMark {
            scope,
            edge: if enter {
                MarkEdge::Enter
            } else {
                MarkEdge::Exit
            },
            fuel,
            wall_ns,
        }
    }

    fn capture_with_summary() -> String {
        let agg = ProfileAggregator::new();
        agg.record_call(
            "com.core.classic-perimeters",
            &[mark(3, true, 0, 0), mark(3, false, 900, 90)],
            1_000,
        );
        let event = ProgressEvent::profile_summary(
            "slice-1".to_string(),
            1_735_843_200_000,
            5_000,
            agg.finish(),
        );
        let layer = ProgressEvent::layer_start(
            "slice-1".to_string(),
            slicer_runtime::progress_events::ProgressPhase::PerLayer,
            0,
            1_735_843_200_000,
        );
        format!(
            "warning: startup DAG advisory (Foo): Bar\n\
             {}\n\
             \n\
             {}\n",
            serde_json::to_string(&layer).unwrap(),
            serde_json::to_string(&event).unwrap(),
        )
    }

    #[test]
    fn extracts_the_summary_from_a_noisy_capture() {
        let summary = extract_profile_summary(&capture_with_summary()).unwrap();
        assert_eq!(summary.fuel_total, 1_000);
        assert_eq!(summary.modules.len(), 1);
        assert_eq!(summary.modules[0].unit, ProfileUnit::Fuel);
        assert_eq!(
            summary.modules[0].scopes[0].scope,
            "polygon_ops::offset2_ex"
        );
    }

    /// A capture from a plain `--instrument-stderr` run is the most likely
    /// mistake, so the error has to name the missing flag rather than render
    /// an empty table that looks like "nothing was slow".
    #[test]
    fn a_capture_without_profile_names_the_missing_flag() {
        let layer = ProgressEvent::layer_start(
            "slice-1".to_string(),
            slicer_runtime::progress_events::ProgressPhase::PerLayer,
            0,
            1,
        );
        let capture = serde_json::to_string(&layer).unwrap();
        let err = extract_profile_summary(&capture).unwrap_err();
        assert!(err.contains("--profile"), "{err}");
        assert!(err.contains("without"), "{err}");
    }

    #[test]
    fn an_empty_capture_says_it_found_no_events_at_all() {
        let err = extract_profile_summary("not json\n\n").unwrap_err();
        assert!(err.contains("no progress events found"), "{err}");
    }

    /// Two runs concatenated into one file: the reader means the latest.
    #[test]
    fn the_last_summary_wins_when_a_capture_holds_several() {
        let first = {
            let agg = ProfileAggregator::new();
            agg.record_call("m", &[mark(1, true, 0, 0), mark(1, false, 10, 1)], 10);
            ProgressEvent::profile_summary("a".to_string(), 1, 1, agg.finish())
        };
        let second = {
            let agg = ProfileAggregator::new();
            agg.record_call("m", &[mark(1, true, 0, 0), mark(1, false, 99, 9)], 99);
            ProgressEvent::profile_summary("b".to_string(), 2, 2, agg.finish())
        };
        let capture = format!(
            "{}\n{}\n",
            serde_json::to_string(&first).unwrap(),
            serde_json::to_string(&second).unwrap()
        );
        assert_eq!(extract_profile_summary(&capture).unwrap().fuel_total, 99);
    }

    #[test]
    fn rendering_from_a_file_matches_the_live_table() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.jsonl");
        let capture = capture_with_summary();
        std::fs::write(&path, &capture).unwrap();

        let rendered = run_profile_from(&path, false).unwrap();
        let expected = format_profile_summary(&extract_profile_summary(&capture).unwrap());
        assert_eq!(rendered, expected);
        assert!(
            rendered.contains("com.core.classic-perimeters"),
            "{rendered}"
        );
    }

    #[test]
    fn json_mode_emits_the_payload_verbatim() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.jsonl");
        std::fs::write(&path, capture_with_summary()).unwrap();

        let rendered = run_profile_from(&path, true).unwrap();
        let back: ProfileSummary = serde_json::from_str(&rendered).unwrap();
        assert_eq!(back.fuel_total, 1_000);
    }

    #[test]
    fn a_missing_file_reports_the_path() {
        let err = run_profile_from(Path::new("does/not/exist.jsonl"), false).unwrap_err();
        assert!(err.contains("does/not/exist.jsonl"), "{err}");
    }
}
