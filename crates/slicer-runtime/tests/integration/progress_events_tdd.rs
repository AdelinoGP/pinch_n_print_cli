#![allow(missing_docs)]

//! TDD red tests for TASK-036: Progress event emitter.
//!
//! These tests define the contract for progress event emission per docs/09_progress_events.md
//! and must fail only on the explicit todo! stub until the green implementation is completed.
//!
//! Acceptance criteria:
//! - [x] API covers `ProgressEvent` struct with all required fields
//! - [x] API covers `ProgressEventType`, `ProgressPhase`, `ProgressStatus`, `ProgressError` enums/structs
//! - [x] API covers helper constructors for all event types
//! - [x] API covers `JsonLinesEmitter` for JSON serialization
//! - [x] API covers `SliceEventCollector` for aggregate statistics
//! - [x] Tests lock down required fields per event type
//! - [x] Tests lock down error tracking behavior
//! - [x] Tests lock down event ordering constraints
//!
//! Reference: docs/09_progress_events.md

use slicer_runtime::progress_events::{
    JsonLinesEmitter, ProgressError, ProgressEvent, ProgressEventType, ProgressPhase,
    ProgressStatus, SliceEventCollector, PROGRESS_EVENT_SCHEMA_VERSION,
};
use std::io::Cursor;

// ============================================================================
// Test fixtures
// ============================================================================

fn slice_id() -> String {
    "9f9075ad-2bd8-4e9a-a2f5-3b9055d2f239".to_string()
}

fn timestamp_ms() -> u64 {
    1735843200123
}

fn error_fixture(fatal: bool) -> ProgressError {
    ProgressError {
        code: 12014,
        message: "feature_flags length mismatch".to_string(),
        fatal,
        suggestion: Some("Verify wall-loop feature flag cardinality".to_string()),
        reason: None,
    }
}

// ============================================================================
// Test 1: Event schema version is "1.3.0" (bumped from 1.2.0 â€” additive
// `ProgressError.reason: Option<EventReason>` field).
// ============================================================================

#[test]
fn event_schema_version_is_1_2_0() {
    assert_eq!(PROGRESS_EVENT_SCHEMA_VERSION, "1.3.0");
}

// ============================================================================
// Test 2: PhaseStart event has required fields
// ============================================================================

#[test]
fn phase_start_event_has_required_fields() {
    let event = ProgressEvent::phase_start(slice_id(), ProgressPhase::Prepass, timestamp_ms());

    // Required fields: schema_version, event, timestamp_ms, slice_id, phase, status
    assert_eq!(event.schema_version, "1.3.0");
    assert_eq!(event.event, ProgressEventType::PhaseStart);
    assert_eq!(event.timestamp_ms, timestamp_ms());
    assert_eq!(event.slice_id, slice_id());
    assert_eq!(event.phase, Some(ProgressPhase::Prepass));
    assert_eq!(event.status, ProgressStatus::Ok);
}

// ============================================================================
// Test 3: PhaseComplete event has required fields including elapsed_ms
// ============================================================================

#[test]
fn phase_complete_event_has_required_fields() {
    let event = ProgressEvent::phase_complete(
        slice_id(),
        ProgressPhase::Prepass,
        timestamp_ms(),
        1500,
        ProgressStatus::Ok,
    );

    // Required fields: schema_version, event, timestamp_ms, slice_id, phase, status, elapsed_ms
    assert_eq!(event.schema_version, "1.3.0");
    assert_eq!(event.event, ProgressEventType::PhaseComplete);
    assert_eq!(event.timestamp_ms, timestamp_ms());
    assert_eq!(event.slice_id, slice_id());
    assert_eq!(event.phase, Some(ProgressPhase::Prepass));
    assert_eq!(event.status, ProgressStatus::Ok);
    assert_eq!(event.elapsed_ms, Some(1500));
}

// ============================================================================
// Test 4: LayerStart event has required fields including layer_index
// ============================================================================

#[test]
fn layer_start_event_has_required_fields() {
    let event = ProgressEvent::layer_start(slice_id(), ProgressPhase::PerLayer, 42, timestamp_ms());

    // Required fields: schema_version, event, timestamp_ms, slice_id, phase, layer_index, status
    assert_eq!(event.schema_version, "1.3.0");
    assert_eq!(event.event, ProgressEventType::LayerStart);
    assert_eq!(event.timestamp_ms, timestamp_ms());
    assert_eq!(event.slice_id, slice_id());
    assert_eq!(event.phase, Some(ProgressPhase::PerLayer));
    assert_eq!(event.layer_index, Some(42));
    assert_eq!(event.status, ProgressStatus::Ok);
}

// ============================================================================
// Test 5: LayerComplete event has required fields including elapsed_ms, degraded
// ============================================================================

#[test]
fn layer_complete_event_has_required_fields() {
    let event = ProgressEvent::layer_complete(
        slice_id(),
        ProgressPhase::PerLayer,
        42,
        timestamp_ms(),
        18,
        ProgressStatus::Ok,
        false,
    );

    // Required fields: schema_version, event, timestamp_ms, slice_id, phase, layer_index, status, elapsed_ms, degraded
    assert_eq!(event.schema_version, "1.3.0");
    assert_eq!(event.event, ProgressEventType::LayerComplete);
    assert_eq!(event.timestamp_ms, timestamp_ms());
    assert_eq!(event.slice_id, slice_id());
    assert_eq!(event.phase, Some(ProgressPhase::PerLayer));
    assert_eq!(event.layer_index, Some(42));
    assert_eq!(event.status, ProgressStatus::Ok);
    assert_eq!(event.elapsed_ms, Some(18));
    assert_eq!(event.degraded, Some(false));
}

// ============================================================================
// Test 6: ModuleError event has required fields including stage, module_id, error
// ============================================================================

#[test]
fn module_error_event_has_required_fields() {
    let error = error_fixture(true);
    let event = ProgressEvent::module_error(
        slice_id(),
        ProgressPhase::PerLayer,
        "Layer::Perimeters".to_string(),
        Some(42),
        "com.example.perimeters".to_string(),
        timestamp_ms(),
        error.clone(),
    );

    // Required fields: schema_version, event, timestamp_ms, slice_id, phase, stage, layer_index, module_id, status, error
    assert_eq!(event.schema_version, "1.3.0");
    assert_eq!(event.event, ProgressEventType::ModuleError);
    assert_eq!(event.timestamp_ms, timestamp_ms());
    assert_eq!(event.slice_id, slice_id());
    assert_eq!(event.phase, Some(ProgressPhase::PerLayer));
    assert_eq!(event.stage, Some("Layer::Perimeters".to_string()));
    assert_eq!(event.layer_index, Some(42));
    assert_eq!(event.module_id, Some("com.example.perimeters".to_string()));
    // Status should be FatalError for fatal errors
    assert_eq!(event.status, ProgressStatus::FatalError);
    assert!(event.error.is_some());
    let err = event.error.unwrap();
    assert_eq!(err.code, 12014);
    assert!(err.fatal);
}

// ============================================================================
// Test 7: ValidationError event has required fields including error
// ============================================================================

#[test]
fn validation_error_event_has_required_fields() {
    let error = error_fixture(true);
    let event = ProgressEvent::validation_error(slice_id(), timestamp_ms(), error.clone());

    // Required fields: schema_version, event, timestamp_ms, slice_id, phase, status, error
    assert_eq!(event.schema_version, "1.3.0");
    assert_eq!(event.event, ProgressEventType::ValidationError);
    assert_eq!(event.timestamp_ms, timestamp_ms());
    assert_eq!(event.slice_id, slice_id());
    assert_eq!(event.phase, Some(ProgressPhase::Validation));
    assert_eq!(event.status, ProgressStatus::FatalError);
    assert!(event.error.is_some());
}

// ============================================================================
// Test 8: SliceComplete event has required fields including error counts
// ============================================================================

#[test]
fn slice_complete_event_has_required_fields() {
    let event = ProgressEvent::slice_complete(
        slice_id(),
        timestamp_ms(),
        5000,
        ProgressStatus::Ok,
        false,
        0,
        0,
    );

    // Required fields: schema_version, event, timestamp_ms, slice_id, status, degraded, elapsed_ms, fatal_error_count, non_fatal_error_count
    assert_eq!(event.schema_version, "1.3.0");
    assert_eq!(event.event, ProgressEventType::SliceComplete);
    assert_eq!(event.timestamp_ms, timestamp_ms());
    assert_eq!(event.slice_id, slice_id());
    assert_eq!(event.status, ProgressStatus::Ok);
    assert_eq!(event.degraded, Some(false));
    assert_eq!(event.elapsed_ms, Some(5000));
    assert_eq!(event.fatal_error_count, Some(0));
    assert_eq!(event.non_fatal_error_count, Some(0));
}

// ============================================================================
// Test 9: Helper constructors produce correctly typed events
// ============================================================================

#[test]
fn helper_constructors_produce_correct_event_types() {
    let phase_start =
        ProgressEvent::phase_start(slice_id(), ProgressPhase::Prepass, timestamp_ms());
    assert_eq!(phase_start.event, ProgressEventType::PhaseStart);

    let phase_complete = ProgressEvent::phase_complete(
        slice_id(),
        ProgressPhase::Prepass,
        timestamp_ms(),
        100,
        ProgressStatus::Ok,
    );
    assert_eq!(phase_complete.event, ProgressEventType::PhaseComplete);

    let layer_start =
        ProgressEvent::layer_start(slice_id(), ProgressPhase::PerLayer, 0, timestamp_ms());
    assert_eq!(layer_start.event, ProgressEventType::LayerStart);

    let layer_complete = ProgressEvent::layer_complete(
        slice_id(),
        ProgressPhase::PerLayer,
        0,
        timestamp_ms(),
        50,
        ProgressStatus::Ok,
        false,
    );
    assert_eq!(layer_complete.event, ProgressEventType::LayerComplete);

    let module_error = ProgressEvent::module_error(
        slice_id(),
        ProgressPhase::PerLayer,
        "Layer::Infill".to_string(),
        Some(0),
        "com.example.infill".to_string(),
        timestamp_ms(),
        error_fixture(false),
    );
    assert_eq!(module_error.event, ProgressEventType::ModuleError);

    let validation_error =
        ProgressEvent::validation_error(slice_id(), timestamp_ms(), error_fixture(true));
    assert_eq!(validation_error.event, ProgressEventType::ValidationError);

    let slice_complete = ProgressEvent::slice_complete(
        slice_id(),
        timestamp_ms(),
        1000,
        ProgressStatus::Ok,
        false,
        0,
        0,
    );
    assert_eq!(slice_complete.event, ProgressEventType::SliceComplete);
}

// ============================================================================
// Test 10: JsonLinesEmitter serializes to valid JSON
// ============================================================================

#[test]
fn json_lines_emitter_serializes_to_valid_json() {
    let buffer = Cursor::new(Vec::new());
    let emitter = JsonLinesEmitter::new(buffer);

    let event = ProgressEvent::phase_start(slice_id(), ProgressPhase::Prepass, timestamp_ms());

    // emit_event should succeed
    let result = emitter.emit_event(&event);
    assert!(result.is_ok(), "emit_event should succeed: {:?}", result);

    // Get the written data
    let data = emitter.writer.lock().unwrap();
    let json_str = String::from_utf8(data.get_ref().clone()).expect("should be valid UTF-8");

    // Should be valid JSON
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(json_str.trim());
    assert!(parsed.is_ok(), "output should be valid JSON: {}", json_str);

    // Check that required fields are present
    let json = parsed.unwrap();
    assert_eq!(json["schema_version"], "1.3.0");
    assert_eq!(json["event"], "phase_start");
    assert!(json["timestamp_ms"].is_number());
    assert_eq!(json["slice_id"], slice_id());
}

// ============================================================================
// Test 11: JsonLinesEmitter outputs one event per line
// ============================================================================

#[test]
fn json_lines_emitter_outputs_one_event_per_line() {
    let buffer = Cursor::new(Vec::new());
    let emitter = JsonLinesEmitter::new(buffer);

    let event1 = ProgressEvent::phase_start(slice_id(), ProgressPhase::Validation, timestamp_ms());
    let event2 = ProgressEvent::phase_complete(
        slice_id(),
        ProgressPhase::Validation,
        timestamp_ms() + 100,
        100,
        ProgressStatus::Ok,
    );

    emitter.emit_event(&event1).expect("emit should succeed");
    emitter.emit_event(&event2).expect("emit should succeed");

    let data = emitter.writer.lock().unwrap();
    let output = String::from_utf8(data.get_ref().clone()).expect("should be valid UTF-8");

    // Count newlines - should have 2 lines
    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(lines.len(), 2, "should have 2 lines, got: {}", lines.len());

    // Each line should be valid JSON
    for (i, line) in lines.iter().enumerate() {
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(line);
        assert!(parsed.is_ok(), "line {} should be valid JSON: {}", i, line);
    }
}

// ============================================================================
// Test 12: SliceEventCollector tracks fatal/non-fatal counts
// ============================================================================

#[test]
fn slice_event_collector_tracks_error_counts() {
    let mut collector = SliceEventCollector::new();

    // Initially zero
    assert_eq!(collector.fatal_count(), 0);
    assert_eq!(collector.non_fatal_count(), 0);

    // Record a fatal error
    let fatal_event = ProgressEvent::module_error(
        slice_id(),
        ProgressPhase::PerLayer,
        "Layer::Perimeters".to_string(),
        Some(0),
        "com.example.perimeters".to_string(),
        timestamp_ms(),
        error_fixture(true), // fatal=true
    );
    collector.record(fatal_event);

    assert_eq!(collector.fatal_count(), 1);
    assert_eq!(collector.non_fatal_count(), 0);

    // Record a non-fatal error
    let non_fatal_event = ProgressEvent::module_error(
        slice_id(),
        ProgressPhase::PerLayer,
        "Layer::Infill".to_string(),
        Some(1),
        "com.example.infill".to_string(),
        timestamp_ms() + 100,
        error_fixture(false), // fatal=false
    );
    collector.record(non_fatal_event);

    assert_eq!(collector.fatal_count(), 1);
    assert_eq!(collector.non_fatal_count(), 1);
}

// ============================================================================
// Test 13: SliceEventCollector sets degraded=true on any non-fatal error
// ============================================================================

#[test]
fn slice_event_collector_sets_degraded_on_non_fatal_error() {
    let mut collector = SliceEventCollector::new();

    // Initially not degraded
    assert!(!collector.is_degraded());

    // Record a non-fatal error
    let event = ProgressEvent::module_error(
        slice_id(),
        ProgressPhase::PerLayer,
        "Layer::Infill".to_string(),
        Some(0),
        "com.example.infill".to_string(),
        timestamp_ms(),
        error_fixture(false), // fatal=false â†’ non_fatal
    );
    collector.record(event);

    // Now degraded
    assert!(collector.is_degraded());
}

// ============================================================================
// Test 14: Event ordering - layer_start before layer_complete for same layer_index
// ============================================================================

#[test]
fn event_ordering_layer_start_before_layer_complete() {
    let mut collector = SliceEventCollector::new();

    // Record layer_start for layer 42
    let layer_start =
        ProgressEvent::layer_start(slice_id(), ProgressPhase::PerLayer, 42, timestamp_ms());
    collector.record(layer_start);

    // Record layer_complete for layer 42
    let layer_complete = ProgressEvent::layer_complete(
        slice_id(),
        ProgressPhase::PerLayer,
        42,
        timestamp_ms() + 100,
        100,
        ProgressStatus::Ok,
        false,
    );
    collector.record(layer_complete);

    // Verify ordering: layer_start must come before layer_complete
    let events = collector.events();
    assert_eq!(events.len(), 2);

    // Find indices of events for layer 42
    let start_idx = events
        .iter()
        .position(|e| e.event == ProgressEventType::LayerStart && e.layer_index == Some(42))
        .expect("layer_start should exist");
    let complete_idx = events
        .iter()
        .position(|e| e.event == ProgressEventType::LayerComplete && e.layer_index == Some(42))
        .expect("layer_complete should exist");

    assert!(
        start_idx < complete_idx,
        "layer_start (idx={}) must come before layer_complete (idx={})",
        start_idx,
        complete_idx
    );
}

// ============================================================================
// Test 15: Event ordering - phase_start before phase_complete for same phase
// ============================================================================

#[test]
fn event_ordering_phase_start_before_phase_complete() {
    let mut collector = SliceEventCollector::new();

    // Record phase_start for prepass
    let phase_start =
        ProgressEvent::phase_start(slice_id(), ProgressPhase::Prepass, timestamp_ms());
    collector.record(phase_start);

    // Record phase_complete for prepass
    let phase_complete = ProgressEvent::phase_complete(
        slice_id(),
        ProgressPhase::Prepass,
        timestamp_ms() + 500,
        500,
        ProgressStatus::Ok,
    );
    collector.record(phase_complete);

    // Verify ordering: phase_start must come before phase_complete
    let events = collector.events();
    assert_eq!(events.len(), 2);

    // Find indices of events for prepass
    let start_idx = events
        .iter()
        .position(|e| {
            e.event == ProgressEventType::PhaseStart && e.phase == Some(ProgressPhase::Prepass)
        })
        .expect("phase_start should exist");
    let complete_idx = events
        .iter()
        .position(|e| {
            e.event == ProgressEventType::PhaseComplete && e.phase == Some(ProgressPhase::Prepass)
        })
        .expect("phase_complete should exist");

    assert!(
        start_idx < complete_idx,
        "phase_start (idx={}) must come before phase_complete (idx={})",
        start_idx,
        complete_idx
    );
}

// ============================================================================
// Packet 169 Step 3: slice_stats event (schema 1.2.0).
// ============================================================================

/// Shared, cloneable in-memory writer so the JSONL stream produced by the
/// production emission path (`slicer_runtime::run::emit_end_of_slice_events`)
/// can be inspected after the emitter takes ownership of its writer.
#[derive(Clone, Default)]
struct SharedBuf(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);

impl std::io::Write for SharedBuf {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .expect("shared buf poisoned")
            .extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn stats_inputs_fixture(weight: Option<f64>) -> slicer_runtime::run::SliceStatsInputs {
    let mut volumes = std::collections::BTreeMap::new();
    volumes.insert(0u32, 1234.5);
    volumes.insert(1u32, 67.8);
    slicer_runtime::run::SliceStatsInputs {
        gcode_prediction_seconds: 4321,
        gcode_weight_grams: weight,
        gcode_filament_length_mm: 5432.1,
        layer_count: 42,
        first_layer_height_mm: 0.25,
        extruded_volume_mm3: volumes,
        toolchange_count: 3,
    }
}

/// Drive the production end-of-slice emission path and return the parsed
/// JSONL lines it wrote.
fn run_end_of_slice_emission(
    stats: Option<slicer_runtime::run::SliceStatsInputs>,
) -> Vec<serde_json::Value> {
    let buf = SharedBuf::default();
    let emitter = std::sync::Arc::new(JsonLinesEmitter::new(buf.clone()));
    let collector = std::sync::Arc::new(std::sync::Mutex::new(SliceEventCollector::new()));
    let sink = slicer_runtime::progress_events::RuntimeProgressSink::new(emitter, collector);

    slicer_runtime::run::emit_end_of_slice_events(&sink, &slice_id(), 999, stats);

    let bytes = buf.0.lock().expect("shared buf poisoned").clone();
    let text = String::from_utf8(bytes).expect("stream must be UTF-8");
    text.lines()
        .map(|l| serde_json::from_str(l).expect("each line must be valid JSON"))
        .collect()
}

/// Recursively assert no key named `cost` or `gcode_cost` exists anywhere.
fn assert_no_cost_keys(value: &serde_json::Value) {
    if let Some(map) = value.as_object() {
        for (k, v) in map {
            assert!(
                k != "cost" && k != "gcode_cost",
                "forbidden key {k:?} present in event"
            );
            assert_no_cost_keys(v);
        }
    }
}

#[test]
fn slice_stats_event_shape_and_ordering() {
    let lines = run_end_of_slice_emission(Some(stats_inputs_fixture(Some(15.5))));

    let stats_indices: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, v)| v["event"] == "slice_stats")
        .map(|(i, _)| i)
        .collect();
    let complete_indices: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, v)| v["event"] == "slice_complete")
        .map(|(i, _)| i)
        .collect();

    assert_eq!(stats_indices.len(), 1, "exactly one slice_stats line");
    assert_eq!(complete_indices.len(), 1, "exactly one slice_complete line");
    assert!(
        stats_indices[0] < complete_indices[0],
        "slice_stats must precede slice_complete"
    );

    let stats = &lines[stats_indices[0]];
    assert_eq!(stats["schema_version"], "1.2.0");
    assert_eq!(stats["gcode_prediction_seconds"], 4321);
    assert_eq!(stats["gcode_weight_grams"], 15.5);
    assert_eq!(stats["gcode_filament_length_mm"], 5432.1);
    assert_eq!(stats["layer_count"], 42);
    assert!(
        (stats["first_layer_height_mm"].as_f64().unwrap() - 0.25).abs() < 1e-6,
        "first_layer_height_mm must be present and 0.25"
    );
    let volumes = stats["extruded_volume_mm3"]
        .as_object()
        .expect("extruded_volume_mm3 must be a map keyed by extruder index");
    assert_eq!(volumes["0"], 1234.5);
    assert_eq!(volumes["1"], 67.8);
    assert_eq!(stats["toolchange_count"], 3);

    for line in &lines {
        assert_no_cost_keys(line);
    }
}

#[test]
fn slice_stats_omits_weight_without_density() {
    let lines = run_end_of_slice_emission(Some(stats_inputs_fixture(None)));
    let stats = lines
        .iter()
        .find(|v| v["event"] == "slice_stats")
        .expect("slice_stats line must exist");

    let obj = stats.as_object().unwrap();
    assert!(
        !obj.contains_key("gcode_weight_grams"),
        "gcode_weight_grams key must be absent (not null, not 0) without filament_density"
    );
    for key in [
        "gcode_prediction_seconds",
        "gcode_filament_length_mm",
        "layer_count",
        "first_layer_height_mm",
        "extruded_volume_mm3",
        "toolchange_count",
    ] {
        assert!(obj.contains_key(key), "field {key:?} must be present");
    }
}

#[test]
fn progress_event_1_1_0_roundtrip_unchanged() {
    // Every pre-existing (1.1.0-era) event variant must serialize without any
    // of the new slice_stats fields and round-trip unchanged (additive bump).
    let events = vec![
        ProgressEvent::phase_start(slice_id(), ProgressPhase::Prepass, timestamp_ms()),
        ProgressEvent::phase_complete(
            slice_id(),
            ProgressPhase::Prepass,
            timestamp_ms(),
            500,
            ProgressStatus::Ok,
        ),
        ProgressEvent::layer_start(slice_id(), ProgressPhase::PerLayer, 7, timestamp_ms()),
        ProgressEvent::layer_complete(
            slice_id(),
            ProgressPhase::PerLayer,
            7,
            timestamp_ms(),
            12,
            ProgressStatus::Ok,
            false,
        ),
        ProgressEvent::module_error(
            slice_id(),
            ProgressPhase::PerLayer,
            "Layer::Perimeters".to_string(),
            Some(7),
            "org.example.walls".to_string(),
            timestamp_ms(),
            error_fixture(false),
        ),
        ProgressEvent::validation_error(slice_id(), timestamp_ms(), error_fixture(true)),
        ProgressEvent::slice_complete(
            slice_id(),
            timestamp_ms(),
            9000,
            ProgressStatus::Ok,
            false,
            0,
            0,
        ),
        ProgressEvent::stage_start(
            slice_id(),
            ProgressPhase::PerLayer,
            "Layer::Perimeters".to_string(),
            Some(7),
            timestamp_ms(),
        ),
        ProgressEvent::stage_complete(
            slice_id(),
            ProgressPhase::PerLayer,
            "Layer::Perimeters".to_string(),
            Some(7),
            timestamp_ms(),
            33,
        ),
        ProgressEvent::module_start(
            slice_id(),
            ProgressPhase::PerLayer,
            "Layer::Perimeters".to_string(),
            "org.example.walls".to_string(),
            Some(7),
            timestamp_ms(),
        ),
        ProgressEvent::module_complete(
            slice_id(),
            ProgressPhase::PerLayer,
            "Layer::Perimeters".to_string(),
            "org.example.walls".to_string(),
            Some(7),
            timestamp_ms(),
            21,
            64,
        ),
    ];

    let new_keys = [
        "gcode_prediction_seconds",
        "gcode_weight_grams",
        "gcode_filament_length_mm",
        "layer_count",
        "first_layer_height_mm",
        "extruded_volume_mm3",
        "toolchange_count",
    ];

    for event in events {
        let json = serde_json::to_string(&event).expect("serialize");
        let value: serde_json::Value = serde_json::from_str(&json).expect("parse");
        let obj = value.as_object().unwrap();
        for key in new_keys {
            assert!(
                !obj.contains_key(key),
                "1.1.0-era event {:?} must not serialize new key {key:?}",
                event.event
            );
        }
        let back: ProgressEvent = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, event, "round-trip must be lossless");
    }

    // A literal 1.1.0-era JSONL line (no new fields) must still deserialize.
    let legacy = "{\"schema_version\":\"1.1.0\",\"event\":\"slice_complete\",\"timestamp_ms\":1,\"slice_id\":\"s\",\"status\":\"ok\",\"elapsed_ms\":2,\"degraded\":false,\"fatal_error_count\":0,\"non_fatal_error_count\":0}";
    let parsed: ProgressEvent = serde_json::from_str(legacy).expect("legacy line must parse");
    assert_eq!(parsed.event, ProgressEventType::SliceComplete);
    assert_eq!(parsed.gcode_prediction_seconds, None);
    assert_eq!(parsed.layer_count, None);
}

// ============================================================================
// Packet 169 Step 4: per-layer phase_start carries layer_count
// ============================================================================

#[test]
fn phase_start_per_layer_carries_layer_count() {
    use slicer_runtime::{
        LayerProgressSink, Phase, PipelineInstrumentation, ProgressPipelineInstrumentation,
    };
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct RecordingSink {
        events: Mutex<Vec<ProgressEvent>>,
    }
    impl LayerProgressSink for RecordingSink {
        fn record(&self, event: ProgressEvent) {
            self.events.lock().unwrap().push(event);
        }
    }

    let sink = Arc::new(RecordingSink::default());
    let pi = ProgressPipelineInstrumentation::new(
        sink.clone() as Arc<dyn LayerProgressSink + Send + Sync>,
        slice_id(),
    );

    let global_layer_count: u32 = 42;
    pi.on_phase_start(Phase::PrePass);
    pi.on_phase_start_with_layer_count(Phase::PerLayer, Some(global_layer_count));
    pi.on_phase_start(Phase::PostPass);

    let events = sink.events.lock().unwrap();
    assert_eq!(events.len(), 3);
    for event in events.iter() {
        assert_eq!(event.event, ProgressEventType::PhaseStart);
        let json = serde_json::to_string(event).expect("serialize");
        let value: serde_json::Value = serde_json::from_str(&json).expect("parse");
        let obj = value.as_object().unwrap();
        match event.phase {
            Some(ProgressPhase::PerLayer) => {
                assert_eq!(
                    obj.get("layer_count").and_then(|v| v.as_u64()),
                    Some(u64::from(global_layer_count)),
                    "per-layer phase_start must carry layer_count == global layer count"
                );
            }
            other => {
                assert!(
                    !obj.contains_key("layer_count"),
                    "phase_start for {other:?} must omit the layer_count key entirely"
                );
            }
        }
    }
}

#[test]
fn phase_start_per_layer_carries_layer_count_through_composite() {
    use slicer_runtime::instrumentation::NoopInstrumentation;
    use slicer_runtime::{
        CompositeInstrumentation, LayerProgressSink, Phase, PipelineInstrumentation,
        ProgressPipelineInstrumentation,
    };
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct RecordingSink {
        events: Mutex<Vec<ProgressEvent>>,
    }
    impl LayerProgressSink for RecordingSink {
        fn record(&self, event: ProgressEvent) {
            self.events.lock().unwrap().push(event);
        }
    }

    let sink = Arc::new(RecordingSink::default());
    let pi = ProgressPipelineInstrumentation::new(
        sink.clone() as Arc<dyn LayerProgressSink + Send + Sync>,
        slice_id(),
    );
    let other = NoopInstrumentation;
    let composite = CompositeInstrumentation::new(&other, &pi);

    let global_layer_count: u32 = 42;
    composite.on_phase_start_with_layer_count(Phase::PerLayer, Some(global_layer_count));

    let events = sink.events.lock().unwrap();
    assert_eq!(events.len(), 1);
    let event = &events[0];
    assert_eq!(event.event, ProgressEventType::PhaseStart);
    assert_eq!(event.phase, Some(ProgressPhase::PerLayer));
    let json = serde_json::to_string(event).expect("serialize");
    let value: serde_json::Value = serde_json::from_str(&json).expect("parse");
    assert_eq!(
        value
            .as_object()
            .unwrap()
            .get("layer_count")
            .and_then(|v| v.as_u64()),
        Some(u64::from(global_layer_count)),
        "composite must forward layer_count to its delegates, not drop it via the default method"
    );
}
