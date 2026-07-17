//! Default-on progress-event stream coverage (docs/09_progress_events.md).
//!
//! A plain `pnp_cli slice` (no event flags) must emit the core JSONL contract
//! on stderr: phase brackets for validation/prepass/per_layer/postpass, layer
//! events, and `slice_complete` exactly once — with NO stage/module timing
//! events (those stay behind `--instrument-stderr`). `--no-progress-events`
//! silences the stream entirely; `--instrument-stderr` is a strict superset
//! of the core stream (the OrcaSlicer fork passes it unconditionally).

use std::path::PathBuf;
use std::process::Output;

use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/pnp-cli has a parent")
        .parent()
        .expect("workspace root above crates/")
        .to_path_buf()
}

fn model_path() -> PathBuf {
    workspace_root()
        .join("resources")
        .join("regression_wedge.stl")
}

fn module_dir() -> PathBuf {
    workspace_root().join("modules").join("core-modules")
}

fn tail(s: &str, n: usize) -> String {
    let lines: Vec<&str> = s.lines().collect();
    let start = lines.len().saturating_sub(n);
    lines[start..].join("\n")
}

/// Run `pnp_cli slice` with the given extra args; return the process output.
fn run_slice(extra_args: &[&str]) -> (Output, TempDir) {
    let tmp = TempDir::new().expect("tempdir");
    let gcode = tmp.path().join("out.gcode");
    let mut cmd = Command::cargo_bin("pnp_cli").expect("pnp_cli binary");
    cmd.arg("slice")
        .arg("--model")
        .arg(model_path())
        .arg("--module-dir")
        .arg(module_dir())
        .arg("--no-default-module-paths")
        .arg("--output")
        .arg(&gcode);
    for a in extra_args {
        cmd.arg(a);
    }
    let output = cmd.output().expect("spawn pnp_cli");
    assert!(
        output.status.success(),
        "pnp_cli slice must succeed; stderr tail:\n{}",
        tail(&String::from_utf8_lossy(&output.stderr), 20)
    );
    (output, tmp)
}

/// Parse every JSONL progress-event line on stderr, in order.
fn parse_events(stderr: &str) -> Vec<Value> {
    stderr
        .lines()
        .filter(|l| l.contains("\"schema_version\""))
        .map(|l| serde_json::from_str::<Value>(l).unwrap_or_else(|e| panic!("bad JSONL: {e}: {l}")))
        .collect()
}

fn events_of<'a>(events: &'a [Value], kind: &str) -> Vec<&'a Value> {
    events
        .iter()
        .filter(|e| e["event"].as_str() == Some(kind))
        .collect()
}

fn assert_core_contract(events: &[Value]) {
    // Phase brackets for every docs/09 phase, complete-after-start.
    for phase in ["validation", "prepass", "per_layer", "postpass"] {
        let starts: Vec<usize> = events
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                e["event"].as_str() == Some("phase_start") && e["phase"].as_str() == Some(phase)
            })
            .map(|(i, _)| i)
            .collect();
        let completes: Vec<usize> = events
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                e["event"].as_str() == Some("phase_complete") && e["phase"].as_str() == Some(phase)
            })
            .map(|(i, _)| i)
            .collect();
        assert_eq!(starts.len(), 1, "exactly one phase_start({phase})");
        assert_eq!(completes.len(), 1, "exactly one phase_complete({phase})");
        assert!(
            starts[0] < completes[0],
            "phase_complete({phase}) must follow phase_start({phase})"
        );
    }

    // Layer events present and paired at least once.
    assert!(
        !events_of(events, "layer_start").is_empty(),
        "layer_start events must be emitted"
    );
    assert!(
        !events_of(events, "layer_complete").is_empty(),
        "layer_complete events must be emitted"
    );

    // slice_complete exactly once, last event, with required aggregates.
    let sc = events_of(events, "slice_complete");
    assert_eq!(sc.len(), 1, "slice_complete must fire exactly once");
    let last = events.last().expect("stream non-empty");
    assert_eq!(
        last["event"].as_str(),
        Some("slice_complete"),
        "slice_complete must be the final event"
    );
    assert_eq!(sc[0]["status"].as_str(), Some("ok"));
    assert!(sc[0]["degraded"].is_boolean(), "degraded field required");
    assert!(sc[0]["elapsed_ms"].is_u64(), "elapsed_ms field required");
    assert!(
        sc[0]["fatal_error_count"].is_u64(),
        "fatal_error_count field required"
    );
    assert!(
        sc[0]["non_fatal_error_count"].is_u64(),
        "non_fatal_error_count field required"
    );
    assert_eq!(
        sc[0]["fatal_error_count"].as_u64(),
        Some(0),
        "successful slice must report zero fatal errors"
    );

    // Uniform schema version and a single slice_id across the stream.
    for e in events {
        assert_eq!(e["schema_version"].as_str(), Some("1.1.0"));
        assert_eq!(
            e["slice_id"].as_str(),
            events[0]["slice_id"].as_str(),
            "all events share one slice_id"
        );
    }
}

#[test]
fn default_slice_emits_core_jsonl_stream() {
    let (output, _tmp) = run_slice(&[]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let events = parse_events(&stderr);
    assert!(
        !events.is_empty(),
        "default slice must emit JSONL progress events on stderr; tail:\n{}",
        tail(&stderr, 20)
    );

    assert_core_contract(&events);

    // Core tier must NOT include the instrumented timing events.
    for kind in [
        "stage_start",
        "stage_complete",
        "module_start",
        "module_complete",
    ] {
        assert!(
            events_of(&events, kind).is_empty(),
            "{kind} events must stay behind --instrument-stderr"
        );
    }
}

#[test]
fn no_progress_events_flag_silences_stream() {
    let (output, tmp) = run_slice(&["--no-progress-events"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("\"schema_version\""),
        "--no-progress-events must suppress all JSONL events; tail:\n{}",
        tail(&stderr, 20)
    );
    let gcode = std::fs::read_to_string(tmp.path().join("out.gcode")).expect("gcode written");
    assert!(!gcode.is_empty(), "slice must still produce G-code");
}

#[test]
fn instrument_stderr_is_superset_of_core() {
    let (output, _tmp) = run_slice(&["--instrument-stderr"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let events = parse_events(&stderr);

    // The full core contract still holds (backward compat for the fork)...
    assert_core_contract(&events);

    // ...plus the instrumented timing events.
    for kind in [
        "stage_start",
        "stage_complete",
        "module_start",
        "module_complete",
    ] {
        assert!(
            !events_of(&events, kind).is_empty(),
            "--instrument-stderr must add {kind} events"
        );
    }
}
