//! Pipeline instrumentation surface.
//!
//! Provides a bracket-shaped trait that the scheduler calls at every phase,
//! stage, layer, and module boundary. The default `NoopInstrumentation` is
//! zero-cost; a real implementation (e.g. `slicer_report::Collector`) records
//! timing, memory, and DAG metadata for the HTML slicer report.
//!
//! This module also owns the report-domain types (`Phase`, `TierKind`,
//! `EdgeReason`, `SerialEdge`) and a helper that derives in-stage serial-edge
//! reasons from a stage's `LoadedModule` list without disturbing `dag.rs`.

use serde::Serialize;
use slicer_ir::{ModuleId, StageId};

use crate::execution_plan::CompiledModule;
use crate::manifest::LoadedModule;

/// Top-level execution phase. Layers are reported separately within
/// `Phase::PerLayer`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Phase {
    /// Sequential pre-pass tier (MeshSegmentation, MeshAnalysis, …).
    PrePass,
    /// Per-layer tier executed in parallel across layers, sequential within.
    PerLayer,
    /// Sequential post-pass tier (LayerFinalization, GCodeEmit, …).
    PostPass,
}

/// Pipeline tier a stage belongs to. Used by the report to annotate stages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TierKind {
    /// Pre-pass: sequential by construction.
    PrePass,
    /// Per-layer: parallel across layers, sequential within each layer.
    PerLayer,
    /// Post-pass: sequential by construction.
    PostPass,
}

/// Reason a serial edge exists between two modules in the same stage.
///
/// Only the variants that the actual DAG builder emits are represented —
/// claim conflicts block plan validation entirely (they never appear in a
/// runnable plan), and `layer_parallel_safe = false` constrains parallelism
/// *across layers* rather than between modules in a stage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EdgeReason {
    /// The `from` module writes an IR path that the `to` module reads.
    IrWriteRead {
        /// The shared IR path (e.g. `"PerimeterIR.regions.walls"`).
        writer_path: String,
    },
    /// The `to` module's manifest lists the `from` module in `requires_modules`.
    ExplicitRequires,
}

/// One serial edge between two modules in the same stage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SerialEdge {
    /// Upstream module — runs first.
    pub from: ModuleId,
    /// Downstream module — runs after `from`.
    pub to: ModuleId,
    /// Why the scheduler placed `from` before `to`.
    pub reason: EdgeReason,
}

/// Compute every in-stage serial edge for a single stage's modules.
///
/// Returns one `SerialEdge` per (writer, reader, reason) tuple — so a pair
/// connected by both an IR write/read overlap and an `ExplicitRequires` will
/// appear twice (once per reason). Callers that want a per-pair grouping can
/// fold by `(from, to)` themselves.
///
/// Order is deterministic: edges are sorted by `(from, to, reason_tag)`.
pub fn compute_serial_edges_for_stage(modules: &[LoadedModule]) -> Vec<SerialEdge> {
    let mut edges: Vec<SerialEdge> = Vec::new();

    for writer in modules {
        for reader in modules {
            if writer.id == reader.id {
                continue;
            }
            for written_path in &writer.ir_writes {
                if reader.ir_reads.contains(written_path) {
                    edges.push(SerialEdge {
                        from: writer.id.clone(),
                        to: reader.id.clone(),
                        reason: EdgeReason::IrWriteRead {
                            writer_path: written_path.clone(),
                        },
                    });
                }
            }
        }
    }

    for module in modules {
        for required in &module.requires_modules {
            if modules.iter().any(|m| &m.id == required) {
                edges.push(SerialEdge {
                    from: required.clone(),
                    to: module.id.clone(),
                    reason: EdgeReason::ExplicitRequires,
                });
            }
        }
    }

    edges.sort_by(|a, b| {
        a.from
            .cmp(&b.from)
            .then_with(|| a.to.cmp(&b.to))
            .then_with(|| reason_tag(&a.reason).cmp(&reason_tag(&b.reason)))
            .then_with(|| match (&a.reason, &b.reason) {
                (
                    EdgeReason::IrWriteRead { writer_path: ap },
                    EdgeReason::IrWriteRead { writer_path: bp },
                ) => ap.cmp(bp),
                _ => std::cmp::Ordering::Equal,
            })
    });
    edges
}

fn reason_tag(r: &EdgeReason) -> u8 {
    match r {
        EdgeReason::IrWriteRead { .. } => 0,
        EdgeReason::ExplicitRequires => 1,
    }
}

/// Runtime-equivalent of [`compute_serial_edges_for_stage`] that works on the
/// frozen [`CompiledModule`] view available inside `ExecutionPlan`.
///
/// Emits both `IrWriteRead` (overlapping write/read paths) and
/// `ExplicitRequires` (manifest `requires_modules`) reasons, matching the
/// `LoadedModule`-side helper's coverage.
pub fn compute_serial_edges_from_compiled(modules: &[CompiledModule]) -> Vec<SerialEdge> {
    let mut edges: Vec<SerialEdge> = Vec::new();
    for writer in modules {
        for reader in modules {
            if writer.module_id == reader.module_id {
                continue;
            }
            for written_path in &writer.ir_write_mask.paths {
                if reader.ir_read_mask.paths.contains(written_path) {
                    edges.push(SerialEdge {
                        from: writer.module_id.clone(),
                        to: reader.module_id.clone(),
                        reason: EdgeReason::IrWriteRead {
                            writer_path: written_path.clone(),
                        },
                    });
                }
            }
        }
    }
    for module in modules {
        for required in &module.requires_modules {
            if modules.iter().any(|m| &m.module_id == required) {
                edges.push(SerialEdge {
                    from: required.clone(),
                    to: module.module_id.clone(),
                    reason: EdgeReason::ExplicitRequires,
                });
            }
        }
    }
    edges.sort_by(|a, b| {
        a.from
            .cmp(&b.from)
            .then_with(|| a.to.cmp(&b.to))
            .then_with(|| reason_tag(&a.reason).cmp(&reason_tag(&b.reason)))
            .then_with(|| match (&a.reason, &b.reason) {
                (
                    EdgeReason::IrWriteRead { writer_path: ap },
                    EdgeReason::IrWriteRead { writer_path: bp },
                ) => ap.cmp(bp),
                _ => std::cmp::Ordering::Equal,
            })
    });
    edges
}

/// Bracket-shaped instrumentation surface called by the scheduler.
///
/// All methods take `&self` and must be safe to call from rayon worker
/// threads. Implementations should be lock-cheap on the hot path — the
/// scheduler invokes `on_module_start`/`on_module_end` once per module call
/// per layer.
pub trait PipelineInstrumentation: Send + Sync {
    /// Called when entering a top-level phase.
    fn on_phase_start(&self, phase: Phase);
    /// Called when leaving a top-level phase.
    fn on_phase_end(&self, phase: Phase);

    /// Called when a stage's module loop is about to begin. `layer` is `None`
    /// for prepass/postpass stages.
    fn on_stage_start(&self, stage: &StageId, layer: Option<u32>);
    /// Called after every module in a stage has run (or after an abort).
    fn on_stage_end(&self, stage: &StageId, layer: Option<u32>);

    /// Called immediately before a module dispatch.
    fn on_module_start(&self, stage: &StageId, layer: Option<u32>, module: &ModuleId);
    /// Called immediately after a module dispatch returns.
    ///
    /// `wasm_initial_bytes` is the guest's linear-memory size right after
    /// instantiation but before the export call (the module's static
    /// baseline). `wasm_peak_bytes` is the highwater observed across the
    /// whole dispatch (≥ initial; equal when the call did not grow memory).
    /// Both are `0` if the runner does not sample wasm memory (test mocks,
    /// host built-ins).
    fn on_module_end(
        &self,
        stage: &StageId,
        layer: Option<u32>,
        module: &ModuleId,
        wasm_initial_bytes: u64,
        wasm_peak_bytes: u64,
    );

    /// Called before a layer's stage loop begins. `z_mm` is the layer's
    /// nominal Z height in millimetres (matches `GlobalLayer.z: f32`).
    fn on_layer_start(&self, layer: u32, z_mm: f32);
    /// Called after a layer's stage loop completes (or aborts).
    fn on_layer_end(&self, layer: u32);

    /// Called once per stage during plan freeze, after the DAG has been
    /// built. Carries every in-stage serial edge so the report can show
    /// "module A → module B (reason)" rows even for stages where no modules
    /// happened to fire on a given run.
    fn record_edges(&self, stage: &StageId, tier: TierKind, edges: &[SerialEdge]);

    /// Called immediately after `on_module_end` for **host built-ins** to
    /// report blackboard byte footprint deltas attributable to the call.
    ///
    /// `host_initial_bytes` is `Blackboard::estimated_size()` snapshotted
    /// before the built-in ran; `host_peak_bytes` is the same snapshot
    /// taken after the built-in's commit. The delta
    /// (`host_peak_bytes - host_initial_bytes`) approximates the IR growth
    /// caused by this stage.
    ///
    /// Distinct from `wasm_initial_bytes` / `wasm_peak_bytes`, which name
    /// the guest's linear-memory footprint and are zero for host built-ins
    /// by design. Default impl is no-op so existing implementors stay valid.
    fn on_host_builtin_bytes(
        &self,
        _stage: &StageId,
        _layer: Option<u32>,
        _module: &ModuleId,
        _host_initial_bytes: u64,
        _host_peak_bytes: u64,
    ) {
    }
}

/// Zero-overhead default. All methods are empty; the compiler inlines the
/// call sites to nothing.
pub struct NoopInstrumentation;

impl PipelineInstrumentation for NoopInstrumentation {
    fn on_phase_start(&self, _phase: Phase) {}
    fn on_phase_end(&self, _phase: Phase) {}
    fn on_stage_start(&self, _stage: &StageId, _layer: Option<u32>) {}
    fn on_stage_end(&self, _stage: &StageId, _layer: Option<u32>) {}
    fn on_module_start(&self, _stage: &StageId, _layer: Option<u32>, _module: &ModuleId) {}
    fn on_module_end(
        &self,
        _stage: &StageId,
        _layer: Option<u32>,
        _module: &ModuleId,
        _wasm_initial_bytes: u64,
        _wasm_peak_bytes: u64,
    ) {
    }
    fn on_layer_start(&self, _layer: u32, _z_mm: f32) {}
    fn on_layer_end(&self, _layer: u32) {}
    fn record_edges(&self, _stage: &StageId, _tier: TierKind, _edges: &[SerialEdge]) {}
}

// ============================================================================
// StageInstrumentationGuard
// ============================================================================

/// Panic-safe RAII wrapper around `on_stage_start` / `on_module_start` /
/// `on_module_end` / `on_stage_end`. Replaces hand-rolled instrumentation
/// boilerplate at each host built-in dispatch site.
///
/// Construction emits the two `start` callbacks; calling `finish` emits the
/// two `end` callbacks plus `on_host_builtin_bytes` with the captured
/// before/after blackboard sizes. If the guard is dropped without `finish`
/// being called (e.g. a panic or early `?` propagation in the closure that
/// owned it), `Drop` emits the `end` callbacks with `0/0` bytes so the
/// slicer-report never sees a dangling start-without-end pair.
pub struct StageInstrumentationGuard<'a> {
    instrumentation: &'a dyn PipelineInstrumentation,
    stage_id: StageId,
    layer: Option<u32>,
    module_id: ModuleId,
    host_initial_bytes: u64,
    finished: bool,
}

impl<'a> StageInstrumentationGuard<'a> {
    /// Open the bracket: emit `on_stage_start` + `on_module_start`, and
    /// snapshot the blackboard's byte footprint as the initial baseline.
    pub fn start(
        instrumentation: &'a dyn PipelineInstrumentation,
        stage_id: impl Into<StageId>,
        layer: Option<u32>,
        module_id: impl Into<ModuleId>,
        host_initial_bytes: u64,
    ) -> Self {
        let stage_id = stage_id.into();
        let module_id = module_id.into();
        instrumentation.on_stage_start(&stage_id, layer);
        instrumentation.on_module_start(&stage_id, layer, &module_id);
        Self {
            instrumentation,
            stage_id,
            layer,
            module_id,
            host_initial_bytes,
            finished: false,
        }
    }

    /// Close the bracket on the success path: emit `on_module_end` (0/0
    /// wasm bytes — host built-ins by contract), then
    /// `on_host_builtin_bytes(initial, peak)`, then `on_stage_end`.
    /// Consumes the guard so Drop becomes a no-op.
    pub fn finish(mut self, host_peak_bytes: u64) {
        self.finished = true;
        self.instrumentation
            .on_module_end(&self.stage_id, self.layer, &self.module_id, 0, 0);
        self.instrumentation.on_host_builtin_bytes(
            &self.stage_id,
            self.layer,
            &self.module_id,
            self.host_initial_bytes,
            host_peak_bytes,
        );
        self.instrumentation
            .on_stage_end(&self.stage_id, self.layer);
    }
}

impl Drop for StageInstrumentationGuard<'_> {
    fn drop(&mut self) {
        if self.finished {
            return;
        }
        // Panic or early-return path: ensure the report sees end events so
        // dangling start-without-end pairs never reach the slicer-report.
        // We do NOT emit on_host_builtin_bytes here — the peak is unknown
        // on the failure path.
        self.instrumentation
            .on_module_end(&self.stage_id, self.layer, &self.module_id, 0, 0);
        self.instrumentation
            .on_stage_end(&self.stage_id, self.layer);
    }
}

// ============================================================================
// CompositeInstrumentation
// ============================================================================

/// Fans every `PipelineInstrumentation` callback out to two delegates in order.
///
/// Used when `--instrument-stderr` and `--report` are both active on a single
/// `slicer-host run` invocation, so the JSONL stream and the HTML report
/// `Collector` both see every bracket without either consumer interfering with
/// the other.
pub struct CompositeInstrumentation<'a> {
    a: &'a (dyn PipelineInstrumentation + 'a),
    b: &'a (dyn PipelineInstrumentation + 'a),
}

impl<'a> CompositeInstrumentation<'a> {
    /// Build a composite that fans calls to `a` first, then `b`.
    pub fn new(
        a: &'a (dyn PipelineInstrumentation + 'a),
        b: &'a (dyn PipelineInstrumentation + 'a),
    ) -> Self {
        Self { a, b }
    }
}

impl PipelineInstrumentation for CompositeInstrumentation<'_> {
    fn on_phase_start(&self, phase: Phase) {
        self.a.on_phase_start(phase);
        self.b.on_phase_start(phase);
    }
    fn on_phase_end(&self, phase: Phase) {
        self.a.on_phase_end(phase);
        self.b.on_phase_end(phase);
    }
    fn on_stage_start(&self, stage: &StageId, layer: Option<u32>) {
        self.a.on_stage_start(stage, layer);
        self.b.on_stage_start(stage, layer);
    }
    fn on_stage_end(&self, stage: &StageId, layer: Option<u32>) {
        self.a.on_stage_end(stage, layer);
        self.b.on_stage_end(stage, layer);
    }
    fn on_module_start(&self, stage: &StageId, layer: Option<u32>, module: &ModuleId) {
        self.a.on_module_start(stage, layer, module);
        self.b.on_module_start(stage, layer, module);
    }
    fn on_module_end(
        &self,
        stage: &StageId,
        layer: Option<u32>,
        module: &ModuleId,
        wasm_initial_bytes: u64,
        wasm_peak_bytes: u64,
    ) {
        self.a
            .on_module_end(stage, layer, module, wasm_initial_bytes, wasm_peak_bytes);
        self.b
            .on_module_end(stage, layer, module, wasm_initial_bytes, wasm_peak_bytes);
    }
    fn on_layer_start(&self, layer: u32, z_mm: f32) {
        self.a.on_layer_start(layer, z_mm);
        self.b.on_layer_start(layer, z_mm);
    }
    fn on_layer_end(&self, layer: u32) {
        self.a.on_layer_end(layer);
        self.b.on_layer_end(layer);
    }
    fn record_edges(&self, stage: &StageId, tier: TierKind, edges: &[SerialEdge]) {
        self.a.record_edges(stage, tier, edges);
        self.b.record_edges(stage, tier, edges);
    }
    fn on_host_builtin_bytes(
        &self,
        stage: &StageId,
        layer: Option<u32>,
        module: &ModuleId,
        host_initial_bytes: u64,
        host_peak_bytes: u64,
    ) {
        self.a
            .on_host_builtin_bytes(stage, layer, module, host_initial_bytes, host_peak_bytes);
        self.b
            .on_host_builtin_bytes(stage, layer, module, host_initial_bytes, host_peak_bytes);
    }
}

#[cfg(test)]
mod composite_tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Default)]
    struct Counting {
        calls: Mutex<Vec<&'static str>>,
    }

    impl PipelineInstrumentation for Counting {
        fn on_phase_start(&self, _phase: Phase) {
            self.calls.lock().unwrap().push("phase_start");
        }
        fn on_phase_end(&self, _phase: Phase) {
            self.calls.lock().unwrap().push("phase_end");
        }
        fn on_stage_start(&self, _stage: &StageId, _layer: Option<u32>) {
            self.calls.lock().unwrap().push("stage_start");
        }
        fn on_stage_end(&self, _stage: &StageId, _layer: Option<u32>) {
            self.calls.lock().unwrap().push("stage_end");
        }
        fn on_module_start(&self, _stage: &StageId, _layer: Option<u32>, _module: &ModuleId) {
            self.calls.lock().unwrap().push("module_start");
        }
        fn on_module_end(
            &self,
            _stage: &StageId,
            _layer: Option<u32>,
            _module: &ModuleId,
            _wasm_initial_bytes: u64,
            _wasm_peak_bytes: u64,
        ) {
            self.calls.lock().unwrap().push("module_end");
        }
        fn on_layer_start(&self, _layer: u32, _z_mm: f32) {
            self.calls.lock().unwrap().push("layer_start");
        }
        fn on_layer_end(&self, _layer: u32) {
            self.calls.lock().unwrap().push("layer_end");
        }
        fn record_edges(&self, _stage: &StageId, _tier: TierKind, _edges: &[SerialEdge]) {
            self.calls.lock().unwrap().push("record_edges");
        }
        fn on_host_builtin_bytes(
            &self,
            _stage: &StageId,
            _layer: Option<u32>,
            _module: &ModuleId,
            _host_initial_bytes: u64,
            _host_peak_bytes: u64,
        ) {
            self.calls.lock().unwrap().push("host_builtin_bytes");
        }
    }

    #[test]
    fn composite_fans_every_callback_to_both_delegates() {
        let a = Counting::default();
        let b = Counting::default();
        let composite = CompositeInstrumentation::new(&a, &b);
        let stage: StageId = "Layer::Perimeters".to_string();
        let module: ModuleId = "m".to_string();

        composite.on_phase_start(Phase::PrePass);
        composite.on_phase_end(Phase::PrePass);
        composite.on_stage_start(&stage, Some(0));
        composite.on_stage_end(&stage, Some(0));
        composite.on_module_start(&stage, Some(0), &module);
        composite.on_module_end(&stage, Some(0), &module, 1, 2);
        composite.on_layer_start(0, 0.2);
        composite.on_layer_end(0);
        composite.record_edges(&stage, TierKind::PerLayer, &[]);
        composite.on_host_builtin_bytes(&stage, None, &module, 0, 0);

        let a_calls = a.calls.lock().unwrap().clone();
        let b_calls = b.calls.lock().unwrap().clone();
        assert_eq!(
            a_calls,
            vec![
                "phase_start",
                "phase_end",
                "stage_start",
                "stage_end",
                "module_start",
                "module_end",
                "layer_start",
                "layer_end",
                "record_edges",
                "host_builtin_bytes",
            ]
        );
        assert_eq!(a_calls, b_calls, "both delegates must see every callback");
    }
}

#[cfg(test)]
mod guard_tests {
    use super::*;
    use std::panic::{catch_unwind, AssertUnwindSafe};
    use std::sync::Mutex;

    #[derive(Default)]
    struct RecordingInstrumentation {
        events: Mutex<Vec<String>>,
    }

    impl PipelineInstrumentation for RecordingInstrumentation {
        fn on_phase_start(&self, _phase: Phase) {}
        fn on_phase_end(&self, _phase: Phase) {}
        fn on_stage_start(&self, stage: &StageId, _layer: Option<u32>) {
            self.events
                .lock()
                .unwrap()
                .push(format!("stage_start:{stage}"));
        }
        fn on_stage_end(&self, stage: &StageId, _layer: Option<u32>) {
            self.events
                .lock()
                .unwrap()
                .push(format!("stage_end:{stage}"));
        }
        fn on_module_start(&self, _stage: &StageId, _layer: Option<u32>, module: &ModuleId) {
            self.events
                .lock()
                .unwrap()
                .push(format!("module_start:{module}"));
        }
        fn on_module_end(
            &self,
            _stage: &StageId,
            _layer: Option<u32>,
            module: &ModuleId,
            _wasm_initial_bytes: u64,
            _wasm_peak_bytes: u64,
        ) {
            self.events
                .lock()
                .unwrap()
                .push(format!("module_end:{module}"));
        }
        fn on_layer_start(&self, _layer: u32, _z_mm: f32) {}
        fn on_layer_end(&self, _layer: u32) {}
        fn record_edges(&self, _stage: &StageId, _tier: TierKind, _edges: &[SerialEdge]) {}
        fn on_host_builtin_bytes(
            &self,
            _stage: &StageId,
            _layer: Option<u32>,
            module: &ModuleId,
            initial: u64,
            peak: u64,
        ) {
            self.events
                .lock()
                .unwrap()
                .push(format!("bytes:{module}:{initial}:{peak}"));
        }
    }

    #[test]
    fn guard_finish_emits_full_event_sequence_with_bytes() {
        let instr = RecordingInstrumentation::default();
        let guard = StageInstrumentationGuard::start(
            &instr,
            "PrePass::MeshAnalysis",
            None,
            "host:mesh_analysis",
            100,
        );
        guard.finish(250);

        let events = instr.events.lock().unwrap().clone();
        assert_eq!(
            events,
            vec![
                "stage_start:PrePass::MeshAnalysis".to_string(),
                "module_start:host:mesh_analysis".to_string(),
                "module_end:host:mesh_analysis".to_string(),
                "bytes:host:mesh_analysis:100:250".to_string(),
                "stage_end:PrePass::MeshAnalysis".to_string(),
            ]
        );
    }

    #[test]
    fn guard_drop_after_panic_still_emits_end_events_without_bytes() {
        let instr = RecordingInstrumentation::default();

        let result = catch_unwind(AssertUnwindSafe(|| {
            let _guard =
                StageInstrumentationGuard::start(&instr, "PrePass::Slice", None, "host:slice", 100);
            panic!("simulated built-in failure");
        }));

        assert!(result.is_err(), "panic must propagate out of guard scope");

        let events = instr.events.lock().unwrap().clone();
        // The bytes event MUST NOT fire on the failure path (peak is unknown).
        // The end events MUST fire so the report never sees a dangling
        // start-without-end pair.
        assert_eq!(
            events,
            vec![
                "stage_start:PrePass::Slice".to_string(),
                "module_start:host:slice".to_string(),
                "module_end:host:slice".to_string(),
                "stage_end:PrePass::Slice".to_string(),
            ],
            "panic-safety: expected start, module_end (0/0), stage_end \
             — no bytes event on failure path"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::LoadedModuleBuilder;
    use slicer_ir::SemVer;
    use std::path::PathBuf;

    fn module(
        id: &str,
        ir_reads: &[&str],
        ir_writes: &[&str],
        requires_modules: &[&str],
    ) -> LoadedModule {
        LoadedModuleBuilder::new(
            id,
            SemVer {
                major: 1,
                minor: 0,
                patch: 0,
            },
            "Layer::Perimeters",
            "slicer:world-layer@1.0.0",
            PathBuf::from(format!("fixtures/{id}.wasm")),
        )
        .ir_reads(ir_reads.iter().map(|s| s.to_string()).collect())
        .ir_writes(ir_writes.iter().map(|s| s.to_string()).collect())
        .requires_modules(requires_modules.iter().map(|s| s.to_string()).collect())
        .min_host_version(SemVer {
            major: 0,
            minor: 1,
            patch: 0,
        })
        .min_ir_schema(SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        })
        .max_ir_schema(SemVer {
            major: 2,
            minor: 0,
            patch: 0,
        })
        .layer_parallel_safe(true)
        .build()
    }

    #[test]
    fn ir_write_read_overlap_emits_edge_with_path() {
        let modules = vec![
            module("a", &[], &["PerimeterIR.regions.walls"], &[]),
            module("b", &["PerimeterIR.regions.walls"], &[], &[]),
        ];
        let edges = compute_serial_edges_for_stage(&modules);
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].from, "a");
        assert_eq!(edges[0].to, "b");
        match &edges[0].reason {
            EdgeReason::IrWriteRead { writer_path } => {
                assert_eq!(writer_path, "PerimeterIR.regions.walls");
            }
            other => panic!("expected IrWriteRead, got {other:?}"),
        }
    }

    #[test]
    fn explicit_requires_emits_edge_with_explicit_reason() {
        let modules = vec![module("a", &[], &[], &[]), module("b", &[], &[], &["a"])];
        let edges = compute_serial_edges_for_stage(&modules);
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].from, "a");
        assert_eq!(edges[0].to, "b");
        assert_eq!(edges[0].reason, EdgeReason::ExplicitRequires);
    }

    #[test]
    fn dual_reason_pair_emits_two_edges() {
        let modules = vec![
            module("a", &[], &["P"], &[]),
            module("b", &["P"], &[], &["a"]),
        ];
        let edges = compute_serial_edges_for_stage(&modules);
        assert_eq!(edges.len(), 2, "expected one edge per reason");
        assert!(edges.iter().any(|e| matches!(
            (&e.from[..], &e.to[..], &e.reason),
            ("a", "b", EdgeReason::IrWriteRead { .. })
        )));
        assert!(edges.iter().any(|e| matches!(
            (&e.from[..], &e.to[..], &e.reason),
            ("a", "b", EdgeReason::ExplicitRequires)
        )));
    }

    #[test]
    fn requires_pointing_at_missing_module_is_dropped() {
        let modules = vec![module("b", &[], &[], &["does-not-exist"])];
        let edges = compute_serial_edges_for_stage(&modules);
        assert!(edges.is_empty());
    }

    fn compiled_module(
        id: &str,
        ir_reads: &[&str],
        ir_writes: &[&str],
        requires_modules: &[&str],
    ) -> CompiledModule {
        use crate::execution_plan::IrAccessMask;
        use crate::instance_pool::{build_wasm_instance_pool, WasmArtifactMetadata};
        use std::sync::Arc;
        let loaded = module(id, ir_reads, ir_writes, requires_modules);
        let pool = Arc::new(
            build_wasm_instance_pool(
                &loaded,
                1,
                WasmArtifactMetadata {
                    uses_shared_memory: false,
                },
            )
            .expect("pool should build for synthetic fixture"),
        );
        crate::execution_plan::CompiledModuleBuilder::new(id.to_string(), pool)
            .ir_read_mask(IrAccessMask {
                paths: ir_reads.iter().map(|s| s.to_string()).collect(),
            })
            .ir_write_mask(IrAccessMask {
                paths: ir_writes.iter().map(|s| s.to_string()).collect(),
            })
            .requires_modules(requires_modules.iter().map(|s| s.to_string()).collect())
            .build()
    }

    #[test]
    fn compiled_explicit_requires_emits_explicit_reason() {
        let modules = vec![
            compiled_module("a", &[], &[], &[]),
            compiled_module("b", &[], &[], &["a"]),
        ];
        let edges = compute_serial_edges_from_compiled(&modules);
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].from, "a");
        assert_eq!(edges[0].to, "b");
        assert_eq!(edges[0].reason, EdgeReason::ExplicitRequires);
    }

    #[test]
    fn compiled_dual_reason_pair_emits_two_edges() {
        let modules = vec![
            compiled_module("a", &[], &["P"], &[]),
            compiled_module("b", &["P"], &[], &["a"]),
        ];
        let edges = compute_serial_edges_from_compiled(&modules);
        assert_eq!(edges.len(), 2);
        assert!(edges.iter().any(|e| matches!(
            (&e.from[..], &e.to[..], &e.reason),
            ("a", "b", EdgeReason::IrWriteRead { .. })
        )));
        assert!(edges.iter().any(|e| matches!(
            (&e.from[..], &e.to[..], &e.reason),
            ("a", "b", EdgeReason::ExplicitRequires)
        )));
    }

    #[test]
    fn noop_instrumentation_compiles_and_runs() {
        let n = NoopInstrumentation;
        n.on_phase_start(Phase::PrePass);
        n.on_stage_start(&"s".to_string(), Some(0));
        n.on_module_start(&"s".to_string(), Some(0), &"m".to_string());
        n.on_module_end(&"s".to_string(), Some(0), &"m".to_string(), 0, 0);
        n.on_layer_start(0, 0.2f32);
        n.on_layer_end(0);
        n.on_stage_end(&"s".to_string(), Some(0));
        n.on_phase_end(Phase::PrePass);
        n.record_edges(&"s".to_string(), TierKind::PerLayer, &[]);
    }
}
