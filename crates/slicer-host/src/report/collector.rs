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
use std::time::Instant;

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
    /// Per-stage serial edges recorded at plan freeze.
    edges_by_stage: Mutex<HashMap<String, Vec<SerialEdge>>>,
    prepass: Mutex<Vec<StageRecord>>,
    layers: Mutex<Vec<LayerRecord>>,
    postpass: Mutex<Vec<StageRecord>>,
    current_phase: AtomicU8,
    layer_count: AtomicU8Counter,
    module_count: AtomicU8Counter,
}

/// Atomic u32 counter wrapped to keep field types tidy.
struct AtomicU8Counter(std::sync::atomic::AtomicU32);

impl AtomicU8Counter {
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

impl Collector {
    /// Create a new collector. `model_path` is informational and embedded
    /// in the report header.
    pub fn new(model_path: impl Into<String>) -> Self {
        Self {
            base_instant: Instant::now(),
            started_at: format!("{:?}", std::time::SystemTime::now()),
            model_path: model_path.into(),
            edges_by_stage: Mutex::new(HashMap::new()),
            prepass: Mutex::new(Vec::new()),
            layers: Mutex::new(Vec::new()),
            postpass: Mutex::new(Vec::new()),
            current_phase: AtomicU8::new(PHASE_NONE),
            layer_count: AtomicU8Counter::new(),
            module_count: AtomicU8Counter::new(),
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
            _ => {
                if let Ok(mut v) = self.postpass.lock() {
                    v.push(record);
                }
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
        }
    }

    /// Finalize, render, and write the HTML to `path`.
    pub fn finish_and_render_to(&self, path: impl AsRef<Path>) -> std::io::Result<()> {
        let report = self.finalize();
        let html = render_html(&report);
        std::fs::write(path, html)
    }

    /// Finalize and return the rendered HTML as a string. Useful for tests.
    pub fn finish_and_render_to_string(&self) -> String {
        let report = self.finalize();
        render_html(&report)
    }
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
        _wasm_before: u64,
        _wasm_after: u64,
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
            wasm_delta: 0,
            wasm_peak: 0,
        };
        self.module_count.set_max(self.module_count.get() + 1);
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
