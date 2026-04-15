//! Progress event emitter API for runtime event streaming.
//!
//! This module provides the infrastructure for emitting structured runtime events
//! during a slice command execution, as defined in docs/09_progress_events.md.
//!
//! ## Transport
//!
//! - Default transport: JSON Lines (`.jsonl`) on stdout.
//! - Optional transport: explicit event file via `--log-events <path>`.
//! - Every event is a single JSON object on one line.
//!
//! ## Event Schema (v1)
//!
//! Events include schema_version, event type, timestamp_ms, slice_id, and
//! context-specific fields as defined in the Required Field Matrix.

use serde::{Deserialize, Serialize};
use std::io::Write;
use std::sync::{Arc, Mutex};

use crate::layer_executor::LayerProgressSink;

/// Schema version for progress events.
pub const PROGRESS_EVENT_SCHEMA_VERSION: &str = "1.0.0";

/// Type of progress event emitted during slicing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProgressEventType {
    /// Emitted when a phase starts.
    PhaseStart,
    /// Emitted when a phase completes.
    PhaseComplete,
    /// Emitted when a layer starts processing.
    LayerStart,
    /// Emitted when a layer completes processing.
    LayerComplete,
    /// Emitted when a module encounters an error.
    ModuleError,
    /// Emitted when validation fails.
    ValidationError,
    /// Emitted when the entire slice operation completes.
    SliceComplete,
}

/// Phase of the slicing pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProgressPhase {
    /// Validation phase.
    Validation,
    /// Prepass phase (surface classification, layer planning, etc.).
    Prepass,
    /// Per-layer processing phase.
    PerLayer,
    /// Postpass phase (G-code emission).
    Postpass,
}

/// Status of an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProgressStatus {
    /// Operation completed successfully.
    Ok,
    /// Operation completed with non-fatal errors (degraded success).
    NonFatalError,
    /// Operation failed fatally.
    FatalError,
}

/// Error details included in error events.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProgressError {
    /// Numeric error code.
    pub code: u32,
    /// Human-readable error message.
    pub message: String,
    /// Whether this error is fatal (aborts the slice).
    pub fatal: bool,
    /// Optional suggestion for resolving the error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

/// A structured progress event emitted during slicing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProgressEvent {
    /// Schema version (always "1.0.0" for v1).
    pub schema_version: String,
    /// Type of event.
    pub event: ProgressEventType,
    /// Unix epoch time in milliseconds.
    pub timestamp_ms: u64,
    /// Unique identifier for this slice operation.
    pub slice_id: String,
    /// Phase of the slicing pipeline.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase: Option<ProgressPhase>,
    /// Stage identifier (e.g., "Layer::Perimeters").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage: Option<String>,
    /// Global layer index (0-based).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layer_index: Option<u32>,
    /// Module identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module_id: Option<String>,
    /// Status of the operation.
    pub status: ProgressStatus,
    /// Elapsed time in milliseconds (relative to event scope).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elapsed_ms: Option<u64>,
    /// Whether any non-fatal error occurred (degraded success).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub degraded: Option<bool>,
    /// Error details (required for module_error and validation_error).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ProgressError>,
    /// Count of fatal errors (for slice_complete).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fatal_error_count: Option<u32>,
    /// Count of non-fatal errors (for slice_complete).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub non_fatal_error_count: Option<u32>,
}

impl ProgressEvent {
    /// Create a phase_start event.
    ///
    /// Required fields: schema_version, event, timestamp_ms, slice_id, phase, status
    pub fn phase_start(slice_id: String, phase: ProgressPhase, timestamp_ms: u64) -> Self {
        Self {
            schema_version: PROGRESS_EVENT_SCHEMA_VERSION.to_string(),
            event: ProgressEventType::PhaseStart,
            timestamp_ms,
            slice_id,
            phase: Some(phase),
            stage: None,
            layer_index: None,
            module_id: None,
            status: ProgressStatus::Ok,
            elapsed_ms: None,
            degraded: None,
            error: None,
            fatal_error_count: None,
            non_fatal_error_count: None,
        }
    }

    /// Create a phase_complete event.
    ///
    /// Required fields: schema_version, event, timestamp_ms, slice_id, phase, status, elapsed_ms
    pub fn phase_complete(
        slice_id: String,
        phase: ProgressPhase,
        timestamp_ms: u64,
        elapsed_ms: u64,
        status: ProgressStatus,
    ) -> Self {
        Self {
            schema_version: PROGRESS_EVENT_SCHEMA_VERSION.to_string(),
            event: ProgressEventType::PhaseComplete,
            timestamp_ms,
            slice_id,
            phase: Some(phase),
            stage: None,
            layer_index: None,
            module_id: None,
            status,
            elapsed_ms: Some(elapsed_ms),
            degraded: None,
            error: None,
            fatal_error_count: None,
            non_fatal_error_count: None,
        }
    }

    /// Create a layer_start event.
    ///
    /// Required fields: schema_version, event, timestamp_ms, slice_id, phase, layer_index, status
    pub fn layer_start(
        slice_id: String,
        phase: ProgressPhase,
        layer_index: u32,
        timestamp_ms: u64,
    ) -> Self {
        Self {
            schema_version: PROGRESS_EVENT_SCHEMA_VERSION.to_string(),
            event: ProgressEventType::LayerStart,
            timestamp_ms,
            slice_id,
            phase: Some(phase),
            stage: None,
            layer_index: Some(layer_index),
            module_id: None,
            status: ProgressStatus::Ok,
            elapsed_ms: None,
            degraded: None,
            error: None,
            fatal_error_count: None,
            non_fatal_error_count: None,
        }
    }

    /// Create a layer_complete event.
    ///
    /// Required fields: schema_version, event, timestamp_ms, slice_id, phase, layer_index, status, elapsed_ms, degraded
    pub fn layer_complete(
        slice_id: String,
        phase: ProgressPhase,
        layer_index: u32,
        timestamp_ms: u64,
        elapsed_ms: u64,
        status: ProgressStatus,
        degraded: bool,
    ) -> Self {
        Self {
            schema_version: PROGRESS_EVENT_SCHEMA_VERSION.to_string(),
            event: ProgressEventType::LayerComplete,
            timestamp_ms,
            slice_id,
            phase: Some(phase),
            stage: None,
            layer_index: Some(layer_index),
            module_id: None,
            status,
            elapsed_ms: Some(elapsed_ms),
            degraded: Some(degraded),
            error: None,
            fatal_error_count: None,
            non_fatal_error_count: None,
        }
    }

    /// Create a module_error event.
    ///
    /// Required fields: schema_version, event, timestamp_ms, slice_id, phase, stage, layer_index, module_id, status, error
    pub fn module_error(
        slice_id: String,
        phase: ProgressPhase,
        stage: String,
        layer_index: u32,
        module_id: String,
        timestamp_ms: u64,
        error: ProgressError,
    ) -> Self {
        // Determine status based on error.fatal
        let status = if error.fatal {
            ProgressStatus::FatalError
        } else {
            ProgressStatus::NonFatalError
        };

        Self {
            schema_version: PROGRESS_EVENT_SCHEMA_VERSION.to_string(),
            event: ProgressEventType::ModuleError,
            timestamp_ms,
            slice_id,
            phase: Some(phase),
            stage: Some(stage),
            layer_index: Some(layer_index),
            module_id: Some(module_id),
            status,
            elapsed_ms: None,
            degraded: None,
            error: Some(error),
            fatal_error_count: None,
            non_fatal_error_count: None,
        }
    }

    /// Create a validation_error event.
    ///
    /// Required fields: schema_version, event, timestamp_ms, slice_id, phase, status, error
    pub fn validation_error(slice_id: String, timestamp_ms: u64, error: ProgressError) -> Self {
        Self {
            schema_version: PROGRESS_EVENT_SCHEMA_VERSION.to_string(),
            event: ProgressEventType::ValidationError,
            timestamp_ms,
            slice_id,
            phase: Some(ProgressPhase::Validation),
            stage: None,
            layer_index: None,
            module_id: None,
            status: ProgressStatus::FatalError,
            elapsed_ms: None,
            degraded: None,
            error: Some(error),
            fatal_error_count: None,
            non_fatal_error_count: None,
        }
    }

    /// Create a slice_complete event.
    ///
    /// Required fields: schema_version, event, timestamp_ms, slice_id, status, degraded, elapsed_ms, fatal_error_count, non_fatal_error_count
    pub fn slice_complete(
        slice_id: String,
        timestamp_ms: u64,
        elapsed_ms: u64,
        status: ProgressStatus,
        degraded: bool,
        fatal_error_count: u32,
        non_fatal_error_count: u32,
    ) -> Self {
        Self {
            schema_version: PROGRESS_EVENT_SCHEMA_VERSION.to_string(),
            event: ProgressEventType::SliceComplete,
            timestamp_ms,
            slice_id,
            phase: None,
            stage: None,
            layer_index: None,
            module_id: None,
            status,
            elapsed_ms: Some(elapsed_ms),
            degraded: Some(degraded),
            error: None,
            fatal_error_count: Some(fatal_error_count),
            non_fatal_error_count: Some(non_fatal_error_count),
        }
    }
}

/// Trait for emitting progress events.
///
/// Implementations must be non-blocking to per-layer compute threads.
/// Events should be queued to a dedicated emitter thread/process.
pub trait ProgressEventEmitter: Send + Sync {
    /// Emit a single progress event.
    ///
    /// This method should be non-blocking and must never drop events.
    fn emit(&self, event: ProgressEvent);
}

/// JSON Lines emitter that writes events to a writer (stdout or file).
///
/// Each event is serialized as a single JSON object on one line.
pub struct JsonLinesEmitter<W: Write + Send> {
    /// The underlying writer wrapped in a mutex for thread-safe access.
    pub writer: std::sync::Mutex<W>,
}

impl<W: Write + Send> JsonLinesEmitter<W> {
    /// Create a new JSON Lines emitter with the given writer.
    pub fn new(writer: W) -> Self {
        Self {
            writer: std::sync::Mutex::new(writer),
        }
    }

    /// Serialize an event to JSON and write it to the underlying writer.
    ///
    /// Each event is written as a single JSON object followed by a newline.
    pub fn emit_event(&self, event: &ProgressEvent) -> std::io::Result<()> {
        let json = serde_json::to_string(event)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        let mut writer = self.writer.lock().expect("writer mutex poisoned");
        writeln!(writer, "{}", json)?;
        Ok(())
    }
}

impl<W: Write + Send + Sync> ProgressEventEmitter for JsonLinesEmitter<W> {
    fn emit(&self, event: ProgressEvent) {
        // Trait method is non-blocking and ignores errors
        // (events are queued for emission)
        let _ = self.emit_event(&event);
    }
}

/// Collector for tracking aggregate statistics during a slice operation.
///
/// Used to build the final slice_complete event with accurate error counts.
#[derive(Debug, Clone, Default)]
pub struct SliceEventCollector {
    /// Count of fatal errors encountered.
    pub fatal_error_count: u32,
    /// Count of non-fatal errors encountered.
    pub non_fatal_error_count: u32,
    /// Whether any non-fatal error occurred (slice is degraded).
    pub degraded: bool,
    /// Events collected in order.
    events: Vec<ProgressEvent>,
}

impl SliceEventCollector {
    /// Create a new collector.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record an event and update aggregate statistics.
    ///
    /// For events with an error field, tracks fatal vs non-fatal counts.
    /// Sets degraded=true on any non-fatal error.
    pub fn record(&mut self, event: ProgressEvent) {
        // Track error counts based on the error field
        if let Some(ref error) = event.error {
            if error.fatal {
                self.fatal_error_count += 1;
            } else {
                self.non_fatal_error_count += 1;
                // Any non-fatal error sets degraded=true
                self.degraded = true;
            }
        }

        // Store the event
        self.events.push(event);
    }

    /// Get the current fatal error count.
    pub fn fatal_count(&self) -> u32 {
        self.fatal_error_count
    }

    /// Get the current non-fatal error count.
    pub fn non_fatal_count(&self) -> u32 {
        self.non_fatal_error_count
    }

    /// Check if the slice is degraded (any non-fatal error occurred).
    pub fn is_degraded(&self) -> bool {
        self.degraded
    }

    /// Get all recorded events in order.
    pub fn events(&self) -> &[ProgressEvent] {
        &self.events
    }

    /// Drain all recorded events.
    pub fn drain(&mut self) -> Vec<ProgressEvent> {
        std::mem::take(&mut self.events)
    }
}

/// Production `LayerProgressSink` that forwards each recorded event to both
/// a `ProgressEventEmitter` (e.g. the JSONL transport on stderr/stdout) and
/// a shared `SliceEventCollector` so aggregate degraded/fatal counts remain
/// visible to the slice driver (docs/09 progress events; docs/11 §73-75).
///
/// Recording is deterministic per call: the collector is updated before the
/// event is handed to the emitter so the collector's view of the slice state
/// is consistent with any observer reading the JSONL stream after a given
/// event has been written.
pub struct RuntimeProgressSink {
    emitter: Arc<dyn ProgressEventEmitter>,
    collector: Arc<Mutex<SliceEventCollector>>,
}

impl RuntimeProgressSink {
    /// Construct a sink that fans out events to `emitter` and `collector`.
    #[must_use]
    pub fn new(
        emitter: Arc<dyn ProgressEventEmitter>,
        collector: Arc<Mutex<SliceEventCollector>>,
    ) -> Self {
        Self { emitter, collector }
    }

    /// Access the shared `SliceEventCollector` (e.g. to read final counts
    /// after the pipeline completes).
    #[must_use]
    pub fn collector(&self) -> Arc<Mutex<SliceEventCollector>> {
        Arc::clone(&self.collector)
    }
}

impl LayerProgressSink for RuntimeProgressSink {
    fn record(&self, event: ProgressEvent) {
        self.collector
            .lock()
            .expect("slice event collector mutex poisoned")
            .record(event.clone());
        self.emitter.emit(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_version_is_1_0_0() {
        assert_eq!(PROGRESS_EVENT_SCHEMA_VERSION, "1.0.0");
    }
}
