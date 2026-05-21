//! In-process collector that aggregates bracket events into a `Report`.
//!
//! Used as `Arc<Collector>` so rayon workers can share it across threads.
//! Each worker thread maintains its own pending-scope stack via
//! `thread_local!`; only finalized records cross the `Mutex` boundary,
//! keeping per-event contention low.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Mutex;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use slicer_ir::{ModuleId, StageId};

use crate::instrumentation::{Phase, PipelineInstrumentation, SerialEdge, TierKind};

use super::allocator;
use super::model::{
    LayerRecord, MemDelta, ModuleRecord, ParallelismRecord, Report, SliceMeta, StageRecord,
};
use super::render::render_html;

const PHASE_NONE: u8 = 0;
const PHASE_PREPASS: u8 = 1;
const PHASE_PERLAYER: u8 = 2;
const PHASE_POSTPASS: u8 = 3;

enum PendingScope {
    Layer {
        record: LayerRecord,
        alloc_key: u32,
    },
    Stage {
        record: StageRecord,
        alloc_key: u32,
    },
    Module {
        record: ModuleRecord,
        alloc_key: u32,
    },
}

thread_local! {
    static SCOPE_STACK: RefCell<Vec<PendingScope>> = const { RefCell::new(Vec::new()) };
}

/// In-process collector implementing [`PipelineInstrumentation`].
///
/// Construct with [`Collector::new`], wrap in `Arc`, hand to the pipeline
/// via `run_pipeline_with_instrumentation`. After the pipeline returns,
/// call [`Collector::finish_and_render_to`] to write the HTML file.
pub struct Collector {
    base_instant: Instant,
    started_at: String,
    model_path: String,
    verbose: bool,
    /// Per-stage serial edges recorded at plan freeze.
    edges_by_stage: Mutex<HashMap<String, Vec<SerialEdge>>>,
    prepass: Mutex<Vec<StageRecord>>,
    layers: Mutex<Vec<LayerRecord>>,
    postpass: Mutex<Vec<StageRecord>>,
    current_phase: AtomicU8,
    /// Tracks the largest observed `layer_index + 1` — idempotent peak,
    /// safe to race because `set_max` only ratchets upward.
    layer_count: PeakCounter,
    /// True running tally of module-call brackets — incremented once per
    /// `on_module_end`. Uses `fetch_add` so concurrent rayon workers can't
    /// silently undercount the way a load-then-set-max RMW would.
    module_count: AtomicCounter,
}

/// Idempotent peak: only stores `v` if greater than current. Safe to race.
struct PeakCounter(std::sync::atomic::AtomicU32);

impl PeakCounter {
    fn new() -> Self {
        Self(std::sync::atomic::AtomicU32::new(0))
    }
    fn set_max(&self, v: u32) {
        let mut cur = self.0.load(Ordering::Relaxed);
        while v > cur {
            match self
                .0
                .compare_exchange(cur, v, Ordering::Relaxed, Ordering::Relaxed)
            {
                Ok(_) => break,
                Err(prev) => cur = prev,
            }
        }
    }
    fn get(&self) -> u32 {
        self.0.load(Ordering::Relaxed)
    }
}

/// Monotonic tally: each `inc()` is a single atomic add, race-free.
struct AtomicCounter(std::sync::atomic::AtomicU32);

impl AtomicCounter {
    fn new() -> Self {
        Self(std::sync::atomic::AtomicU32::new(0))
    }
    fn inc(&self) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
    fn get(&self) -> u32 {
        self.0.load(Ordering::Relaxed)
    }
}

impl Collector {
    /// Create a new collector. `model_path` is informational and embedded
    /// in the report header.
    pub fn new(model_path: impl Into<String>) -> Self {
        Self::new_with_verbose(model_path, false)
    }

    /// Create a new collector with verbose rendering enabled, which adds a
    /// per-layer-per-module detail table to the HTML output. Off by default
    /// because the detail table scales as O(layers × stages × modules) —
    /// a 1000-layer slice can produce ~10⁴ rows.
    pub fn new_with_verbose(model_path: impl Into<String>, verbose: bool) -> Self {
        Self {
            base_instant: Instant::now(),
            started_at: format_rfc3339_utc(SystemTime::now()),
            model_path: model_path.into(),
            verbose,
            edges_by_stage: Mutex::new(HashMap::new()),
            prepass: Mutex::new(Vec::new()),
            layers: Mutex::new(Vec::new()),
            postpass: Mutex::new(Vec::new()),
            current_phase: AtomicU8::new(PHASE_NONE),
            layer_count: PeakCounter::new(),
            module_count: AtomicCounter::new(),
        }
    }

    fn now_ns(&self) -> u64 {
        self.base_instant.elapsed().as_nanos() as u64
    }

    fn current_thread_name() -> String {
        std::thread::current()
            .name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("thread-{:?}", std::thread::current().id()))
    }

    fn take_edges_for(&self, stage_id: &str) -> Vec<SerialEdge> {
        self.edges_by_stage
            .lock()
            .ok()
            .and_then(|mut m| m.remove(stage_id))
            .unwrap_or_default()
    }

    fn route_completed_stage(&self, record: StageRecord) {
        // If there's still a parent on the stack (we're inside a Layer),
        // attach to it. Otherwise route to prepass/postpass by current phase.
        let attached = SCOPE_STACK.with(|cell| {
            let mut stack = cell.borrow_mut();
            if let Some(PendingScope::Layer {
                record: layer_rec, ..
            }) = stack.last_mut()
            {
                layer_rec.stages.push(record.clone());
                return true;
            }
            false
        });
        if attached {
            return;
        }
        match self.current_phase.load(Ordering::Relaxed) {
            PHASE_PREPASS => {
                if let Ok(mut v) = self.prepass.lock() {
                    v.push(record);
                }
            }
            PHASE_POSTPASS => {
                if let Ok(mut v) = self.postpass.lock() {
                    v.push(record);
                }
            }
            PHASE_PERLAYER => {
                // A stage_end fired with no enclosing Layer scope while
                // PerLayer was active — bracket bug upstream. Drop the
                // record rather than misattribute it to postpass.
                let _ = record;
            }
            _ => {
                // Phase is NONE: stage_end fired with no phase open at all.
                // Same reasoning — drop rather than silently route.
                let _ = record;
            }
        }
    }

    /// Stop accepting events and produce the final `Report`.
    pub fn finalize(&self) -> Report {
        let total_ns = self.now_ns();
        let prepass = self.prepass.lock().map(|v| v.clone()).unwrap_or_default();
        let layers = self.layers.lock().map(|v| v.clone()).unwrap_or_default();
        let postpass = self.postpass.lock().map(|v| v.clone()).unwrap_or_default();
        let mut layers_sorted = layers;
        layers_sorted.sort_by_key(|l| l.layer_index);

        let parallelism = derive_parallelism(&layers_sorted);

        let slice_meta = SliceMeta {
            model_path: self.model_path.clone(),
            total_ns,
            layer_count: self.layer_count.get(),
            module_count: self.module_count.get(),
            peak_host_bytes: allocator::peak_bytes(),
            started_at: self.started_at.clone(),
        };

        Report {
            slice_meta,
            prepass,
            layers: layers_sorted,
            postpass,
            parallelism,
            verbose: self.verbose,
        }
    }

    /// Finalize, render, and write the HTML to `path`.
    pub fn finish_and_render_to(&self, path: impl AsRef<Path>) -> std::io::Result<()> {
        let report = self.finalize();
        let html = render_html(&report);
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, html)
    }

    /// Finalize and return the rendered HTML as a string. Useful for tests.
    pub fn finish_and_render_to_string(&self) -> String {
        let report = self.finalize();
        render_html(&report)
    }
}

/// Format a `SystemTime` as an RFC 3339 timestamp in UTC, e.g.
/// `2026-05-16T23:38:05Z`. Hand-rolled to avoid pulling in a date crate —
/// the value is purely human-display in the report header.
fn format_rfc3339_utc(t: SystemTime) -> String {
    let secs = t
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Civil-from-days (Howard Hinnant, public domain) — converts a Unix
    // day count into (year, month, day) for the proleptic Gregorian calendar.
    let days = (secs / 86_400) as i64;
    let secs_of_day = (secs % 86_400) as u32;
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = y + if month <= 2 { 1 } else { 0 };
    let hour = secs_of_day / 3600;
    let minute = (secs_of_day % 3600) / 60;
    let second = secs_of_day % 60;
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hour, minute, second
    )
}

fn derive_parallelism(layers: &[LayerRecord]) -> ParallelismRecord {
    let mut per_thread: std::collections::BTreeMap<String, Vec<(u32, u64, u64)>> =
        std::collections::BTreeMap::new();
    for layer in layers {
        per_thread
            .entry(layer.worker_thread.clone())
            .or_default()
            .push((layer.layer_index, layer.start_ns, layer.end_ns));
    }
    let threads_observed: Vec<String> = per_thread.keys().cloned().collect();

    // Sweep-line over (event_ns, kind) where +1 on start, -1 on end.
    let mut events: Vec<(u64, i32)> = Vec::with_capacity(layers.len() * 2);
    for layer in layers {
        events.push((layer.start_ns, 1));
        events.push((layer.end_ns, -1));
    }
    events.sort();
    let mut cur = 0i32;
    let mut peak = 0i32;
    for (_, delta) in events {
        cur += delta;
        if cur > peak {
            peak = cur;
        }
    }

    ParallelismRecord {
        threads_observed,
        max_layers_concurrent: peak.max(0) as usize,
        per_thread,
    }
}

impl PipelineInstrumentation for Collector {
    fn on_phase_start(&self, phase: Phase) {
        let tag = match phase {
            Phase::PrePass => PHASE_PREPASS,
            Phase::PerLayer => PHASE_PERLAYER,
            Phase::PostPass => PHASE_POSTPASS,
        };
        self.current_phase.store(tag, Ordering::Relaxed);
    }

    fn on_phase_end(&self, _phase: Phase) {
        self.current_phase.store(PHASE_NONE, Ordering::Relaxed);
    }

    fn on_stage_start(&self, stage: &StageId, layer: Option<u32>) {
        let key = allocator::push_scope();
        let tier = match self.current_phase.load(Ordering::Relaxed) {
            PHASE_PREPASS => TierKind::PrePass,
            PHASE_PERLAYER => TierKind::PerLayer,
            PHASE_POSTPASS => TierKind::PostPass,
            _ => TierKind::PerLayer,
        };
        let edges = self.take_edges_for(stage);
        let record = StageRecord {
            stage_id: stage.clone(),
            tier,
            layer_index: layer,
            start_ns: self.now_ns(),
            end_ns: 0,
            mem: MemDelta::default(),
            modules: Vec::new(),
            serial_edges: edges,
        };
        SCOPE_STACK.with(|cell| {
            cell.borrow_mut().push(PendingScope::Stage {
                record,
                alloc_key: key,
            });
        });
    }

    fn on_stage_end(&self, stage: &StageId, _layer: Option<u32>) {
        let popped = SCOPE_STACK.with(|cell| cell.borrow_mut().pop());
        let Some(PendingScope::Stage {
            mut record,
            alloc_key,
        }) = popped
        else {
            // Mismatched bracket — restore whatever was popped (if any) and
            // drop this end event silently. Shouldn't happen in practice.
            if let Some(other) = popped {
                SCOPE_STACK.with(|cell| cell.borrow_mut().push(other));
            }
            return;
        };
        if record.stage_id != *stage {
            // Recovery: push back, ignore. Indicates upstream bug.
            SCOPE_STACK.with(|cell| {
                cell.borrow_mut()
                    .push(PendingScope::Stage { record, alloc_key });
            });
            return;
        }
        record.end_ns = self.now_ns();
        let stats = allocator::pop_scope(alloc_key);
        record.mem = MemDelta {
            host_delta: stats.current,
            host_peak: stats.peak,
            wasm_delta: 0,
            wasm_peak: 0,
        };
        self.route_completed_stage(record);
    }

    fn on_module_start(&self, stage: &StageId, layer: Option<u32>, module: &ModuleId) {
        let key = allocator::push_scope();
        let record = ModuleRecord {
            module_id: module.clone(),
            stage_id: stage.clone(),
            layer_index: layer,
            start_ns: self.now_ns(),
            end_ns: 0,
            mem: MemDelta::default(),
            worker_thread: Self::current_thread_name(),
        };
        SCOPE_STACK.with(|cell| {
            cell.borrow_mut().push(PendingScope::Module {
                record,
                alloc_key: key,
            });
        });
    }

    fn on_module_end(
        &self,
        _stage: &StageId,
        _layer: Option<u32>,
        _module: &ModuleId,
        wasm_initial_bytes: u64,
        wasm_peak_bytes: u64,
    ) {
        let popped = SCOPE_STACK.with(|cell| cell.borrow_mut().pop());
        let Some(PendingScope::Module {
            mut record,
            alloc_key,
        }) = popped
        else {
            if let Some(other) = popped {
                SCOPE_STACK.with(|cell| cell.borrow_mut().push(other));
            }
            return;
        };
        record.end_ns = self.now_ns();
        let stats = allocator::pop_scope(alloc_key);
        record.mem = MemDelta {
            host_delta: stats.current,
            host_peak: stats.peak,
            // `wasm_delta` records how much linear memory the export call
            // grew beyond the post-instantiation baseline. `wasm_peak`
            // is the absolute highwater (initial + any growth). Equal
            // to `wasm_initial_bytes` when the call did not grow memory.
            wasm_delta: (wasm_peak_bytes as i64).saturating_sub(wasm_initial_bytes as i64),
            wasm_peak: wasm_peak_bytes,
        };
        self.module_count.inc();
        SCOPE_STACK.with(|cell| {
            let mut stack = cell.borrow_mut();
            if let Some(PendingScope::Stage {
                record: stage_rec, ..
            }) = stack.last_mut()
            {
                stage_rec.modules.push(record);
            }
        });
    }

    fn on_layer_start(&self, layer: u32, z_mm: f32) {
        let key = allocator::push_scope();
        let record = LayerRecord {
            layer_index: layer,
            z_mm,
            start_ns: self.now_ns(),
            end_ns: 0,
            worker_thread: Self::current_thread_name(),
            mem: MemDelta::default(),
            stages: Vec::new(),
        };
        SCOPE_STACK.with(|cell| {
            cell.borrow_mut().push(PendingScope::Layer {
                record,
                alloc_key: key,
            });
        });
    }

    fn on_layer_end(&self, layer: u32) {
        let popped = SCOPE_STACK.with(|cell| cell.borrow_mut().pop());
        let Some(PendingScope::Layer {
            mut record,
            alloc_key,
        }) = popped
        else {
            if let Some(other) = popped {
                SCOPE_STACK.with(|cell| cell.borrow_mut().push(other));
            }
            return;
        };
        if record.layer_index != layer {
            // recovery
            SCOPE_STACK.with(|cell| {
                cell.borrow_mut()
                    .push(PendingScope::Layer { record, alloc_key });
            });
            return;
        }
        record.end_ns = self.now_ns();
        let stats = allocator::pop_scope(alloc_key);
        record.mem = MemDelta {
            host_delta: stats.current,
            host_peak: stats.peak,
            wasm_delta: 0,
            wasm_peak: 0,
        };
        self.layer_count.set_max(layer + 1);
        if let Ok(mut v) = self.layers.lock() {
            v.push(record);
        }
    }

    fn record_edges(&self, stage: &StageId, _tier: TierKind, edges: &[SerialEdge]) {
        if let Ok(mut m) = self.edges_by_stage.lock() {
            m.insert(stage.clone(), edges.to_vec());
        }
    }
}
