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

use slicer_host::progress_events::{
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
// Test 1: Event schema version is "1.1.0" (bumped from 1.0.0 — additive
// `ProgressError.reason: Option<EventReason>` field).
// ============================================================================

#[test]
fn event_schema_version_is_1_1_0() {
    assert_eq!(PROGRESS_EVENT_SCHEMA_VERSION, "1.1.0");
}

// ============================================================================
// Test 2: PhaseStart event has required fields
// ============================================================================

#[test]
fn phase_start_event_has_required_fields() {
    let event = ProgressEvent::phase_start(slice_id(), ProgressPhase::Prepass, timestamp_ms());

    // Required fields: schema_version, event, timestamp_ms, slice_id, phase, status
    assert_eq!(event.schema_version, "1.1.0");
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
    assert_eq!(event.schema_version, "1.1.0");
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
    assert_eq!(event.schema_version, "1.1.0");
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
    assert_eq!(event.schema_version, "1.1.0");
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
        42,
        "com.example.perimeters".to_string(),
        timestamp_ms(),
        error.clone(),
    );

    // Required fields: schema_version, event, timestamp_ms, slice_id, phase, stage, layer_index, module_id, status, error
    assert_eq!(event.schema_version, "1.1.0");
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
    assert_eq!(event.schema_version, "1.1.0");
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
    assert_eq!(event.schema_version, "1.1.0");
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
        0,
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
    assert_eq!(json["schema_version"], "1.1.0");
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
        0,
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
        1,
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
        0,
        "com.example.infill".to_string(),
        timestamp_ms(),
        error_fixture(false), // fatal=false → non_fatal
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
