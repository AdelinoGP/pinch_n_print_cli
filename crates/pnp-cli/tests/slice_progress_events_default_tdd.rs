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
        // Schema 1.2.0 (packet 169): the per_layer phase_start carries the
        // planned layer_count; other phases omit the key.
        let start_event = &events[starts[0]];
        if phase == "per_layer" {
            let lc = start_event["layer_count"]
                .as_u64()
                .expect("phase_start(per_layer) must carry layer_count");
            assert!(lc > 0, "phase_start(per_layer) layer_count must be > 0");
        } else {
            assert!(
                start_event["layer_count"].is_null(),
                "phase_start({phase}) must not carry layer_count"
            );
        }
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

    // slice_stats (schema 1.2.0, packet 169): exactly once, strictly before
    // slice_complete, with the required aggregate fields.
    let stats_idx: Vec<usize> = events
        .iter()
        .enumerate()
        .filter(|(_, e)| e["event"].as_str() == Some("slice_stats"))
        .map(|(i, _)| i)
        .collect();
    assert_eq!(stats_idx.len(), 1, "slice_stats must fire exactly once");
    let sc_idx = events
        .iter()
        .position(|e| e["event"].as_str() == Some("slice_complete"))
        .expect("slice_complete present");
    assert!(
        stats_idx[0] < sc_idx,
        "slice_stats must precede slice_complete"
    );
    let stats = &events[stats_idx[0]];
    assert!(
        stats["gcode_prediction_seconds"].is_u64(),
        "slice_stats.gcode_prediction_seconds required"
    );
    assert!(
        stats["gcode_filament_length_mm"].is_number(),
        "slice_stats.gcode_filament_length_mm required"
    );
    assert!(
        stats["layer_count"].as_u64().is_some_and(|n| n > 0),
        "slice_stats.layer_count required and > 0"
    );
    assert!(
        stats["first_layer_height_mm"].is_number(),
        "slice_stats.first_layer_height_mm required"
    );
    assert!(
        stats["extruded_volume_mm3"].is_object(),
        "slice_stats.extruded_volume_mm3 required"
    );
    assert!(
        stats["toolchange_count"].is_u64(),
        "slice_stats.toolchange_count required"
    );

    // Uniform schema version (slice_stats is on 1.2.0 per packet 169; all
    // other events are on 1.3.0). The test asserts each event's schema
    // matches its event type, not that the stream is single-versioned.
    for e in events {
        let event_name = e["event"].as_str().unwrap_or("");
        let expected_schema = match event_name {
            "slice_stats" => "1.2.0",
            _ => "1.3.0",
        };
        assert_eq!(
            e["schema_version"].as_str(),
            Some(expected_schema),
            "event {event_name} has unexpected schema_version"
        );
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
