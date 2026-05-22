//! Data model for the HTML slicer report.
//!
//! All structures are populated by `collector::Collector` as the
//! `PipelineInstrumentation` trait fires bracket events during a pipeline
//! run, then handed to `render::render_html` at the end.

use std::collections::BTreeMap;

use crate::instrumentation::{SerialEdge, TierKind};

/// Nanosecond timestamp captured from `std::time::Instant`.
pub type Nanos = u64;

/// Byte delta — signed because a stage may net-free memory.
pub type Bytes = i64;

/// Memory bracketing for one bracket scope.
#[derive(Debug, Clone, Copy, Default)]
pub struct MemDelta {
    /// Net host bytes-in-use change between bracket start and end.
    pub host_delta: Bytes,
    /// Peak host bytes-in-use observed during the bracket scope.
    pub host_peak: u64,
    /// WASM linear-memory growth during the bracket, in bytes
    /// (`wasm_peak - wasm_initial`). Zero when the call did not grow memory
    /// or when the runner does not sample wasm (test mocks, host built-ins).
    pub wasm_delta: Bytes,
    /// WASM linear-memory peak observed during the bracket, in bytes.
    /// Zero when the runner does not sample wasm.
    pub wasm_peak: u64,
}

/// One module dispatch record.
#[derive(Debug, Clone)]
pub struct ModuleRecord {
    /// Module identifier (e.g. `"com.example.classic-perimeters"`).
    pub module_id: String,
    /// Stage identifier this call was part of.
    pub stage_id: String,
    /// Layer index for per-layer stages; `None` for prepass/postpass.
    pub layer_index: Option<u32>,
    /// Wall-clock start (monotonic).
    pub start_ns: Nanos,
    /// Wall-clock end (monotonic).
    pub end_ns: Nanos,
    /// Memory bracket for this call.
    pub mem: MemDelta,
    /// Rayon / OS thread name that executed this call.
    pub worker_thread: String,
}

impl ModuleRecord {
    /// Duration of this call in nanoseconds.
    pub fn duration_ns(&self) -> u64 {
        self.end_ns.saturating_sub(self.start_ns)
    }
}

/// One stage execution record (modules within may have run sequentially).
#[derive(Debug, Clone)]
pub struct StageRecord {
    /// Stage identifier.
    pub stage_id: String,
    /// Tier annotation.
    pub tier: TierKind,
    /// Layer index for per-layer stages; `None` for prepass/postpass.
    pub layer_index: Option<u32>,
    /// Wall-clock start.
    pub start_ns: Nanos,
    /// Wall-clock end.
    pub end_ns: Nanos,
    /// Aggregated memory bracket.
    pub mem: MemDelta,
    /// Per-module records observed during this stage scope.
    pub modules: Vec<ModuleRecord>,
    /// Serial edges contributing to this stage's intra-stage ordering.
    /// Populated by `record_edges` once per stage at plan freeze (so it
    /// shows up even when no modules fired on a given run).
    pub serial_edges: Vec<SerialEdge>,
}

impl StageRecord {
    /// Duration of this stage in nanoseconds.
    pub fn duration_ns(&self) -> u64 {
        self.end_ns.saturating_sub(self.start_ns)
    }
}

/// One layer execution record.
#[derive(Debug, Clone)]
pub struct LayerRecord {
    /// Layer index (zero-based).
    pub layer_index: u32,
    /// Z height in millimetres.
    pub z_mm: f32,
    /// Wall-clock start.
    pub start_ns: Nanos,
    /// Wall-clock end.
    pub end_ns: Nanos,
    /// Rayon worker thread that processed this layer.
    pub worker_thread: String,
    /// Aggregated memory bracket for the whole layer.
    pub mem: MemDelta,
    /// Per-stage records, in execution order.
    pub stages: Vec<StageRecord>,
}

impl LayerRecord {
    /// Duration of this layer in nanoseconds.
    pub fn duration_ns(&self) -> u64 {
        self.end_ns.saturating_sub(self.start_ns)
    }
}

/// Observed parallelism characteristics across the run.
#[derive(Debug, Clone, Default)]
pub struct ParallelismRecord {
    /// Distinct rayon worker thread names observed during per-layer execution.
    pub threads_observed: Vec<String>,
    /// Peak number of layers being processed simultaneously, computed by
    /// sweep-line across `LayerRecord.start_ns/end_ns`.
    pub max_layers_concurrent: usize,
    /// Per-thread gantt rows: thread → list of `(layer_index, start_ns, end_ns)`.
    pub per_thread: BTreeMap<String, Vec<(u32, Nanos, Nanos)>>,
}

/// Per-phase wall-clock elapsed times recorded from the pipeline's
/// `on_phase_start` / `on_phase_end` bracket callbacks.  All values are
/// monotonic nanoseconds relative to the same base `Instant`.
#[derive(Debug, Clone, Copy, Default, serde::Serialize)]
pub struct PhaseWallTimes {
    /// Wall-clock elapsed for the PrePass phase, in nanoseconds.
    pub prepass_ns: u64,
    /// Wall-clock elapsed for the PerLayer phase, in nanoseconds.
    pub perlayer_ns: u64,
    /// Wall-clock elapsed for the PostPass phase, in nanoseconds.
    pub postpass_ns: u64,
}

/// Metadata about the slice this report describes.
#[derive(Debug, Clone, Default)]
pub struct SliceMeta {
    /// Per-phase wall-clock durations.
    pub phase_times: PhaseWallTimes,
    /// Path of the model that was sliced (informational).
    pub model_path: String,
    /// Total slice wall-clock in nanoseconds.
    pub total_ns: Nanos,
    /// Number of layers in the global layer schedule.
    pub layer_count: u32,
    /// Number of modules loaded into the pipeline.
    pub module_count: u32,
    /// Peak host-side bytes observed across the entire run.
    pub peak_host_bytes: u64,
    /// ISO-8601 timestamp of when slicing started.
    pub started_at: String,
}

/// Top-level report. Rendered by `render::render_html`.
#[derive(Debug, Clone, Default)]
pub struct Report {
    /// Run metadata.
    pub slice_meta: SliceMeta,
    /// Prepass stage records (no layer index).
    pub prepass: Vec<StageRecord>,
    /// Per-layer records.
    pub layers: Vec<LayerRecord>,
    /// Postpass stage records (no layer index).
    pub postpass: Vec<StageRecord>,
    /// Parallelism observation.
    pub parallelism: ParallelismRecord,
    /// When `true`, the render includes a per-layer-per-module detail table
    /// (one row per module call). Off by default to keep the HTML compact —
    /// a 1000-layer slice can easily reach 10⁴ module rows.
    pub verbose: bool,
}
