//! Progress event emitter API for runtime event streaming.
//!
//! This module provides the infrastructure for emitting structured runtime events
//! during a slice command execution, as defined in docs/09_progress_events.md.
//!
//! ## Transport
//!
//! - Default transport: JSON Lines (`.jsonl`) on stderr (G-code owns stdout).
//!   Suppressed by `pnp_cli slice --no-progress-events`.
//! - Every event is a single JSON object on one line.
//!
//! ## Event Schema (v1)
//!
//! Events include schema_version, event type, timestamp_ms, slice_id, and
//! context-specific fields as defined in the Required Field Matrix.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::Write;
use std::sync::{Arc, Mutex};

use crate::layer_executor::LayerProgressSink;

/// Schema version for progress events (baseline, without `--instrument-stderr`).
///
/// 1.1.0 (additive): `ProgressError.reason: Option<EventReason>` added. The
/// instrumented schema below shares this struct and therefore the same field;
/// both schemas now sit at 1.1.0. Per `docs/09_progress_events.md` §Compatibility,
/// additive fields are minor version bumps.
///
/// 1.2.0 (additive, packet 169): `slice_stats` event type plus the optional
/// `gcode_prediction_seconds`, `gcode_weight_grams`, `gcode_filament_length_mm`,
/// `layer_count`, `first_layer_height_mm`, `extruded_volume_mm3`, and
/// `toolchange_count` fields.
///
/// 1.3.0 (additive, packet 174): `cancelled` event type.
///
/// Every constructor in this module MUST stamp this constant (or its
/// `_INSTRUMENTED` twin) rather than a version literal. A stream that mixes
/// versions is unparseable by a consumer that keys its field expectations off
/// the first line it sees.
pub const PROGRESS_EVENT_SCHEMA_VERSION: &str = "1.3.0";

/// Schema version emitted when `--instrument-stderr` is active and the
/// additional `stage_*` / `module_*` events plus `wasm_peak_kb` field are in
/// the stream. Additive on top of the baseline — consumers that ignore unknown
/// event types remain compatible.
pub const PROGRESS_EVENT_SCHEMA_VERSION_INSTRUMENTED: &str = "1.3.0";

/// Stable `ProgressError.code` for a `validation_error` raised by intra-stage
/// DAG construction failure during the 14-pass startup validation.
pub const VALIDATION_DAG_CONSTRUCTION_CODE: u32 = 400;

/// Stable `ProgressError.code` for a `validation_error` raised by IR/host
/// version-compatibility failure during the 14-pass startup validation.
pub const VALIDATION_VERSION_COMPAT_CODE: u32 = 401;

/// Stable `ProgressError.code` for a `module_error` raised by a fatal module
/// dispatch or commit failure (prepass, per-layer, or postpass).
pub const MODULE_DISPATCH_FATAL_CODE: u32 = 500;

/// Stable machine-readable reason families for `ProgressError`.
///
/// Additive: new variants are a minor JSONL schema bump per
/// `docs/09_progress_events.md` §Compatibility. Serialized in kebab-case to
/// align with the user-facing message labels (e.g., `"numerical-edge-ambiguity"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EventReason {
    /// Paint-annotation point classification remained numerically unresolved
    /// after polygon edits. Stable code 504; see
    /// `crates/slicer-runtime/src/slice_postprocess.rs` (per-stage reason type
    /// `SlicePostProcessPaintAnnotationWarningReason`).
    NumericalEdgeAmbiguity,
}

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
    /// Emitted when a slice is cancelled before completion.
    Cancelled,
    /// Emitted when the entire slice operation completes.
    SliceComplete,
    /// Emitted exactly once per successful slice (including degraded
    /// success), strictly before `slice_complete`, carrying whole-print
    /// statistics (introduced by packet 169 at schema 1.2.0).
    SliceStats,
    /// Emitted when a stage's module loop begins (instrumented stream only).
    StageStart,
    /// Emitted when a stage's module loop ends (instrumented stream only).
    StageComplete,
    /// Emitted immediately before a module dispatch (instrumented stream only).
    ModuleStart,
    /// Emitted immediately after a module dispatch returns (instrumented stream only).
    ModuleComplete,
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
    /// Stable machine-readable reason family. Additive per JSONL schema 1.1.0.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<EventReason>,
}

/// A structured progress event emitted during slicing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProgressEvent {
    /// Schema version — always `PROGRESS_EVENT_SCHEMA_VERSION`, or its
    /// `_INSTRUMENTED` twin for the `--instrument-stderr` event types.
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
    /// Peak WASM linear memory observed during a module dispatch, in KiB
    /// (ceiling of `wasm_peak_bytes / 1024`). Populated only on
    /// `module_complete` events emitted under `--instrument-stderr`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wasm_peak_kb: Option<u64>,
    /// Estimated print time in whole seconds (for slice_stats).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gcode_prediction_seconds: Option<u64>,
    /// Estimated filament weight in grams (for slice_stats). Present only
    /// when `filament_density` is configured; the key is omitted otherwise
    /// (never 0, never null).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gcode_weight_grams: Option<f64>,
    /// Total filament length consumed across all tools, in mm (for slice_stats).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gcode_filament_length_mm: Option<f64>,
    /// Number of emitted layers (for slice_stats; also used by phase_start in
    /// a later packet-169 step).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layer_count: Option<u32>,
    /// First layer height in mm (for slice_stats).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_layer_height_mm: Option<f32>,
    /// Extruded volume per extruder index in mm³ (for slice_stats).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extruded_volume_mm3: Option<BTreeMap<u32, f64>>,
    /// Number of tool changes in the print (for slice_stats).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub toolchange_count: Option<u32>,
}

impl ProgressEvent {
    /// Create a cancelled event.
    pub fn cancelled(slice_id: String, timestamp_ms: u64) -> Self {
        Self {
            schema_version: PROGRESS_EVENT_SCHEMA_VERSION.to_string(),
            event: ProgressEventType::Cancelled,
            timestamp_ms,
            slice_id,
            phase: None,
            stage: None,
            layer_index: None,
            module_id: None,
            status: ProgressStatus::FatalError,
            elapsed_ms: None,
            degraded: None,
            error: None,
            fatal_error_count: None,
            non_fatal_error_count: None,
            wasm_peak_kb: None,
            gcode_prediction_seconds: None,
            gcode_weight_grams: None,
            gcode_filament_length_mm: None,
            layer_count: None,
            first_layer_height_mm: None,
            extruded_volume_mm3: None,
            toolchange_count: None,
        }
    }

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
            wasm_peak_kb: None,
            gcode_prediction_seconds: None,
            gcode_weight_grams: None,
            gcode_filament_length_mm: None,
            layer_count: None,
            first_layer_height_mm: None,
            extruded_volume_mm3: None,
            toolchange_count: None,
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
            wasm_peak_kb: None,
            gcode_prediction_seconds: None,
            gcode_weight_grams: None,
            gcode_filament_length_mm: None,
            layer_count: None,
            first_layer_height_mm: None,
            extruded_volume_mm3: None,
            toolchange_count: None,
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
            wasm_peak_kb: None,
            gcode_prediction_seconds: None,
            gcode_weight_grams: None,
            gcode_filament_length_mm: None,
            layer_count: None,
            first_layer_height_mm: None,
            extruded_volume_mm3: None,
            toolchange_count: None,
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
            wasm_peak_kb: None,
            gcode_prediction_seconds: None,
            gcode_weight_grams: None,
            gcode_filament_length_mm: None,
            layer_count: None,
            first_layer_height_mm: None,
            extruded_volume_mm3: None,
            toolchange_count: None,
        }
    }

    /// Create a module_error event.
    ///
    /// Required fields: schema_version, event, timestamp_ms, slice_id, phase, stage, module_id, status, error.
    /// `layer_index` is populated for per-layer-phase errors and absent for
    /// prepass/postpass module errors.
    pub fn module_error(
        slice_id: String,
        phase: ProgressPhase,
        stage: String,
        layer_index: Option<u32>,
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
            layer_index,
            module_id: Some(module_id),
            status,
            elapsed_ms: None,
            degraded: None,
            error: Some(error),
            fatal_error_count: None,
            non_fatal_error_count: None,
            wasm_peak_kb: None,
            gcode_prediction_seconds: None,
            gcode_weight_grams: None,
            gcode_filament_length_mm: None,
            layer_count: None,
            first_layer_height_mm: None,
            extruded_volume_mm3: None,
            toolchange_count: None,
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
            wasm_peak_kb: None,
            gcode_prediction_seconds: None,
            gcode_weight_grams: None,
            gcode_filament_length_mm: None,
            layer_count: None,
            first_layer_height_mm: None,
            extruded_volume_mm3: None,
            toolchange_count: None,
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
            wasm_peak_kb: None,
            gcode_prediction_seconds: None,
            gcode_weight_grams: None,
            gcode_filament_length_mm: None,
            layer_count: None,
            first_layer_height_mm: None,
            extruded_volume_mm3: None,
            toolchange_count: None,
        }
    }

    /// Create a slice_stats event (introduced by packet 169 at schema 1.2.0).
    ///
    /// Emitted exactly once per slice that produced a G-code artifact
    /// (including degraded-but-successful runs), strictly before
    /// `slice_complete`. `gcode_weight_grams` is `Some` only when
    /// `filament_density` is configured — the key is omitted otherwise.
    #[allow(clippy::too_many_arguments)]
    pub fn slice_stats(
        slice_id: String,
        timestamp_ms: u64,
        gcode_prediction_seconds: u64,
        gcode_weight_grams: Option<f64>,
        gcode_filament_length_mm: f64,
        layer_count: u32,
        first_layer_height_mm: f32,
        extruded_volume_mm3: BTreeMap<u32, f64>,
        toolchange_count: u32,
    ) -> Self {
        Self {
            schema_version: PROGRESS_EVENT_SCHEMA_VERSION.to_string(),
            event: ProgressEventType::SliceStats,
            timestamp_ms,
            slice_id,
            phase: None,
            stage: None,
            layer_index: None,
            module_id: None,
            status: ProgressStatus::Ok,
            elapsed_ms: None,
            degraded: None,
            error: None,
            fatal_error_count: None,
            non_fatal_error_count: None,
            wasm_peak_kb: None,
            gcode_prediction_seconds: Some(gcode_prediction_seconds),
            gcode_weight_grams,
            gcode_filament_length_mm: Some(gcode_filament_length_mm),
            layer_count: Some(layer_count),
            first_layer_height_mm: Some(first_layer_height_mm),
            extruded_volume_mm3: Some(extruded_volume_mm3),
            toolchange_count: Some(toolchange_count),
        }
    }

    /// Create a stage_start event (emitted only under `--instrument-stderr`).
    ///
    /// Required fields: schema_version, event, timestamp_ms, slice_id, phase, stage, status.
    /// `layer_index` is populated for per-layer stages and absent for prepass/postpass stages.
    pub fn stage_start(
        slice_id: String,
        phase: ProgressPhase,
        stage: String,
        layer_index: Option<u32>,
        timestamp_ms: u64,
    ) -> Self {
        Self {
            schema_version: PROGRESS_EVENT_SCHEMA_VERSION_INSTRUMENTED.to_string(),
            event: ProgressEventType::StageStart,
            timestamp_ms,
            slice_id,
            phase: Some(phase),
            stage: Some(stage),
            layer_index,
            module_id: None,
            status: ProgressStatus::Ok,
            elapsed_ms: None,
            degraded: None,
            error: None,
            fatal_error_count: None,
            non_fatal_error_count: None,
            wasm_peak_kb: None,
            gcode_prediction_seconds: None,
            gcode_weight_grams: None,
            gcode_filament_length_mm: None,
            layer_count: None,
            first_layer_height_mm: None,
            extruded_volume_mm3: None,
            toolchange_count: None,
        }
    }

    /// Create a stage_complete event (emitted only under `--instrument-stderr`).
    ///
    /// Required fields: schema_version, event, timestamp_ms, slice_id, phase, stage, status, elapsed_ms.
    pub fn stage_complete(
        slice_id: String,
        phase: ProgressPhase,
        stage: String,
        layer_index: Option<u32>,
        timestamp_ms: u64,
        elapsed_ms: u64,
    ) -> Self {
        Self {
            schema_version: PROGRESS_EVENT_SCHEMA_VERSION_INSTRUMENTED.to_string(),
            event: ProgressEventType::StageComplete,
            timestamp_ms,
            slice_id,
            phase: Some(phase),
            stage: Some(stage),
            layer_index,
            module_id: None,
            status: ProgressStatus::Ok,
            elapsed_ms: Some(elapsed_ms),
            degraded: None,
            error: None,
            fatal_error_count: None,
            non_fatal_error_count: None,
            wasm_peak_kb: None,
            gcode_prediction_seconds: None,
            gcode_weight_grams: None,
            gcode_filament_length_mm: None,
            layer_count: None,
            first_layer_height_mm: None,
            extruded_volume_mm3: None,
            toolchange_count: None,
        }
    }

    /// Create a module_start event (emitted only under `--instrument-stderr`).
    ///
    /// Required fields: schema_version, event, timestamp_ms, slice_id, phase, stage, module_id, status.
    pub fn module_start(
        slice_id: String,
        phase: ProgressPhase,
        stage: String,
        module_id: String,
        layer_index: Option<u32>,
        timestamp_ms: u64,
    ) -> Self {
        Self {
            schema_version: PROGRESS_EVENT_SCHEMA_VERSION_INSTRUMENTED.to_string(),
            event: ProgressEventType::ModuleStart,
            timestamp_ms,
            slice_id,
            phase: Some(phase),
            stage: Some(stage),
            layer_index,
            module_id: Some(module_id),
            status: ProgressStatus::Ok,
            elapsed_ms: None,
            degraded: None,
            error: None,
            fatal_error_count: None,
            non_fatal_error_count: None,
            wasm_peak_kb: None,
            gcode_prediction_seconds: None,
            gcode_weight_grams: None,
            gcode_filament_length_mm: None,
            layer_count: None,
            first_layer_height_mm: None,
            extruded_volume_mm3: None,
            toolchange_count: None,
        }
    }

    /// Create a module_complete event (emitted only under `--instrument-stderr`).
    ///
    /// Required fields: schema_version, event, timestamp_ms, slice_id, phase, stage, module_id, status, elapsed_ms, wasm_peak_kb.
    /// `wasm_peak_kb` is the ceiling of `wasm_peak_bytes / 1024` and is `0` for host built-ins.
    pub fn module_complete(
        slice_id: String,
        phase: ProgressPhase,
        stage: String,
        module_id: String,
        layer_index: Option<u32>,
        timestamp_ms: u64,
        elapsed_ms: u64,
        wasm_peak_kb: u64,
    ) -> Self {
        Self {
            schema_version: PROGRESS_EVENT_SCHEMA_VERSION_INSTRUMENTED.to_string(),
            event: ProgressEventType::ModuleComplete,
            timestamp_ms,
            slice_id,
            phase: Some(phase),
            stage: Some(stage),
            layer_index,
            module_id: Some(module_id),
            status: ProgressStatus::Ok,
            elapsed_ms: Some(elapsed_ms),
            degraded: None,
            error: None,
            fatal_error_count: None,
            non_fatal_error_count: None,
            wasm_peak_kb: Some(wasm_peak_kb),
            gcode_prediction_seconds: None,
            gcode_weight_grams: None,
            gcode_filament_length_mm: None,
            layer_count: None,
            first_layer_height_mm: None,
            extruded_volume_mm3: None,
            toolchange_count: None,
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

/// Emitter that discards every event. Used when the caller opts out of the
/// JSONL stream (`--no-progress-events`): the `SliceEventCollector` still
/// aggregates counts, but nothing is written to stderr.
pub struct NullEmitter;

impl ProgressEventEmitter for NullEmitter {
    fn emit(&self, _event: ProgressEvent) {}
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

    /// Pins the baseline version so a bump is a deliberate act, not a drift.
    /// The literal here must match the version table in
    /// `docs/09_progress_events.md`.
    #[test]
    fn baseline_schema_version_matches_documented_version_table() {
        assert_eq!(PROGRESS_EVENT_SCHEMA_VERSION, "1.3.0");
    }

    /// The instrumented stream carries the same additive payload as the
    /// baseline stream, so the two constants must not diverge.
    #[test]
    fn instrumented_schema_version_tracks_the_baseline() {
        assert_eq!(
            PROGRESS_EVENT_SCHEMA_VERSION_INSTRUMENTED,
            PROGRESS_EVENT_SCHEMA_VERSION
        );
    }

    /// Structural invariant: one slice emits one schema version.
    ///
    /// Regression guard — `slice_stats` used to hard-code `"1.2.0"` while every
    /// other constructor stamped the constant, so a single stream advertised
    /// two schemas. A consumer that reads the version off the first line and
    /// then keys its field expectations to it is silently mis-parsed by that.
    /// Asserting per-constructor literals would not have caught it; asserting
    /// that the whole set agrees does.
    #[test]
    fn every_constructor_stamps_one_schema_version_per_stream() {
        let slice_id = || "slice-xyz".to_string();
        let ts = 1_735_843_200_000u64;

        let baseline = vec![
            ProgressEvent::phase_start(slice_id(), ProgressPhase::Prepass, ts),
            ProgressEvent::phase_complete(
                slice_id(),
                ProgressPhase::Prepass,
                ts,
                1,
                ProgressStatus::Ok,
            ),
            ProgressEvent::layer_start(slice_id(), ProgressPhase::PerLayer, 0, ts),
            ProgressEvent::layer_complete(
                slice_id(),
                ProgressPhase::PerLayer,
                0,
                ts,
                1,
                ProgressStatus::Ok,
                false,
            ),
            ProgressEvent::cancelled(slice_id(), ts),
            ProgressEvent::slice_complete(slice_id(), ts, 1, ProgressStatus::Ok, false, 0, 0),
            ProgressEvent::slice_stats(slice_id(), ts, 42, None, 1.0, 3, 0.2, BTreeMap::new(), 0),
        ];
        for event in &baseline {
            assert_eq!(
                event.schema_version, PROGRESS_EVENT_SCHEMA_VERSION,
                "{:?} stamped a version that differs from the baseline constant",
                event.event
            );
        }

        let instrumented = vec![
            ProgressEvent::stage_start(
                slice_id(),
                ProgressPhase::Prepass,
                "PrePass::MeshAnalysis".to_string(),
                None,
                ts,
            ),
            ProgressEvent::stage_complete(
                slice_id(),
                ProgressPhase::Prepass,
                "PrePass::MeshAnalysis".to_string(),
                None,
                ts,
                1,
            ),
            ProgressEvent::module_complete(
                slice_id(),
                ProgressPhase::PerLayer,
                "Layer::Perimeters".to_string(),
                "com.example.perimeters".to_string(),
                Some(0),
                ts,
                1,
                0,
            ),
        ];
        for event in &instrumented {
            assert_eq!(
                event.schema_version, PROGRESS_EVENT_SCHEMA_VERSION_INSTRUMENTED,
                "{:?} stamped a version that differs from the instrumented constant",
                event.event
            );
        }
    }

    #[test]
    fn module_complete_event_serializes_with_required_fields() {
        let event = ProgressEvent::module_complete(
            "slice-xyz".to_string(),
            ProgressPhase::PerLayer,
            "Layer::Perimeters".to_string(),
            "com.example.perimeters".to_string(),
            Some(7),
            1_735_843_200_123,
            4_947,
            2_048,
        );
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains(&format!(
            "\"schema_version\":\"{PROGRESS_EVENT_SCHEMA_VERSION_INSTRUMENTED}\""
        )));
        assert!(json.contains("\"event\":\"module_complete\""));
        assert!(json.contains("\"stage\":\"Layer::Perimeters\""));
        assert!(json.contains("\"module_id\":\"com.example.perimeters\""));
        assert!(json.contains("\"layer_index\":7"));
        assert!(json.contains("\"elapsed_ms\":4947"));
        assert!(json.contains("\"wasm_peak_kb\":2048"));
    }

    #[test]
    fn wasm_peak_kb_omitted_when_none_on_other_events() {
        let event = ProgressEvent::layer_start(
            "slice-xyz".to_string(),
            ProgressPhase::PerLayer,
            0,
            1_735_843_200_000,
        );
        let json = serde_json::to_string(&event).unwrap();
        assert!(!json.contains("wasm_peak_kb"));
    }

    #[test]
    fn stage_complete_event_serializes_elapsed_ms() {
        let event = ProgressEvent::stage_complete(
            "slice-xyz".to_string(),
            ProgressPhase::Prepass,
            "PrePass::MeshAnalysis".to_string(),
            None,
            1_735_843_200_451,
            326,
        );
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"event\":\"stage_complete\""));
        assert!(json.contains("\"stage\":\"PrePass::MeshAnalysis\""));
        assert!(json.contains("\"elapsed_ms\":326"));
        // layer_index should be omitted for prepass stages.
        assert!(!json.contains("layer_index"));
    }
}
