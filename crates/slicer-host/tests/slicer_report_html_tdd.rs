//! TDD coverage for the HTML slicer report.
//!
//! Drives `Collector` directly via the `PipelineInstrumentation` trait —
//! no real WASM, no full pipeline — so the test is fast, deterministic,
//! and exercises the bracket → record → HTML pipeline end-to-end.

use std::thread;

use slicer_host::instrumentation::{
    EdgeReason, Phase, PipelineInstrumentation, SerialEdge, TierKind,
};
use slicer_host::report::Collector;

/// Simulate one full run with prepass / per-layer / postpass phases and
/// verify the rendered HTML contains all expected sections and values.
#[test]
fn collector_full_run_produces_well_formed_html() {
    let c = Collector::new("test-model.stl");

    // Plan freeze: record edges for two stages.
    c.record_edges(
        &"Layer::Perimeters".to_string(),
        TierKind::PerLayer,
        &[SerialEdge {
            from: "com.example.module-a".to_string(),
            to: "com.example.module-b".to_string(),
            reason: EdgeReason::IrWriteRead {
                writer_path: "PerimeterIR.regions.walls".to_string(),
            },
        }],
    );
    c.record_edges(&"PrePass::MeshAnalysis".to_string(), TierKind::PrePass, &[]);

    // PrePass phase with one stage and one module.
    c.on_phase_start(Phase::PrePass);
    c.on_stage_start(&"PrePass::MeshAnalysis".to_string(), None);
    c.on_module_start(
        &"PrePass::MeshAnalysis".to_string(),
        None,
        &"com.example.analyzer".to_string(),
    );
    busy_work();
    c.on_module_end(
        &"PrePass::MeshAnalysis".to_string(),
        None,
        &"com.example.analyzer".to_string(),
        0,
        0,
    );
    c.on_stage_end(&"PrePass::MeshAnalysis".to_string(), None);
    c.on_phase_end(Phase::PrePass);

    // PerLayer phase: 3 layers, 1 stage each with 2 modules.
    c.on_phase_start(Phase::PerLayer);
    for layer_idx in 0u32..3 {
        c.on_layer_start(layer_idx, 0.2_f32 * (layer_idx + 1) as f32);
        c.on_stage_start(&"Layer::Perimeters".to_string(), Some(layer_idx));
        for mod_name in &["com.example.module-a", "com.example.module-b"] {
            c.on_module_start(
                &"Layer::Perimeters".to_string(),
                Some(layer_idx),
                &(*mod_name).to_string(),
            );
            busy_work();
            c.on_module_end(
                &"Layer::Perimeters".to_string(),
                Some(layer_idx),
                &(*mod_name).to_string(),
                0,
                0,
            );
        }
        c.on_stage_end(&"Layer::Perimeters".to_string(), Some(layer_idx));
        c.on_layer_end(layer_idx);
    }
    c.on_phase_end(Phase::PerLayer);

    // PostPass: one stage, no modules (just bracket).
    c.on_phase_start(Phase::PostPass);
    c.on_stage_start(&"PostPass::GCodeEmit".to_string(), None);
    c.on_stage_end(&"PostPass::GCodeEmit".to_string(), None);
    c.on_phase_end(Phase::PostPass);

    let html = c.finish_and_render_to_string();

    // Document structure
    assert!(html.starts_with("<!doctype html>"), "should be an HTML doc");
    assert!(html.contains("<title>Slicer Report</title>"));
    assert!(html.ends_with("</body></html>"));

    // Header metadata
    assert!(html.contains("test-model.stl"));
    assert!(html.contains("model:"));
    assert!(html.contains("layers:"));
    assert!(html.contains("module calls:"));

    // Per-layer table
    assert!(html.contains("<h2>Per-Layer</h2>"));
    assert!(html.contains("<th>Layer</th>"));
    assert!(html.contains("<th>Z (mm)</th>"));
    assert!(html.contains("<th>Duration (ms)</th>"));
    assert!(html.contains("<th>Worker</th>"));

    // Per-stage aggregate includes all three stage IDs we exercised
    assert!(html.contains("<h2>Per-Stage Aggregate</h2>"));
    assert!(html.contains("Layer::Perimeters"));
    assert!(html.contains("PrePass::MeshAnalysis"));
    assert!(html.contains("PostPass::GCodeEmit"));

    // Per-module aggregate (per-layer tier) groups by id
    assert!(html.contains("<h2>Per-Module Aggregate (per-layer tier)</h2>"));
    assert!(html.contains("com.example.module-a"));
    assert!(html.contains("com.example.module-b"));

    // Parallelism gantt rendered (we ran serially but the section still shows up)
    assert!(html.contains("<h2>Parallelism (per-layer Gantt)</h2>"));
    assert!(html.contains("<svg"));

    // Serial-edge explainer with IrWriteRead reason
    assert!(html.contains("<h2>Serial Edges (why modules ran in order)</h2>"));
    assert!(
        html.contains("com.example.module-a")
            && html.contains("com.example.module-b")
            && html.contains("IrWriteRead: PerimeterIR.regions.walls"),
        "serial-edge section must label the IrWriteRead writer path"
    );
}

#[test]
fn collector_no_phases_produces_empty_but_valid_html() {
    let c = Collector::new("empty.stl");
    let html = c.finish_and_render_to_string();
    assert!(html.starts_with("<!doctype html>"));
    assert!(html.contains("<title>Slicer Report</title>"));
    assert!(html.ends_with("</body></html>"));
    // No layers means no per-layer table; per-module aggregate also absent.
    assert!(!html.contains("<h2>Per-Layer</h2>"));
    assert!(!html.contains("<h2>Per-Module Aggregate"));
}

#[test]
fn collector_layer_duration_is_non_zero_after_busy_work() {
    let c = Collector::new("dur.stl");
    c.on_phase_start(Phase::PerLayer);
    c.on_layer_start(0, 0.2);
    busy_work_long();
    c.on_layer_end(0);
    c.on_phase_end(Phase::PerLayer);
    let report = c.finalize();
    assert_eq!(report.layers.len(), 1);
    assert!(
        report.layers[0].duration_ns() > 0,
        "layer duration must be > 0 ns after spinning for ~1ms"
    );
}

#[test]
fn collector_worker_thread_is_recorded() {
    let c = Collector::new("worker.stl");
    c.on_phase_start(Phase::PerLayer);
    c.on_layer_start(7, 1.4);
    c.on_layer_end(7);
    c.on_phase_end(Phase::PerLayer);
    let report = c.finalize();
    assert_eq!(report.layers.len(), 1);
    let name = &report.layers[0].worker_thread;
    assert!(!name.is_empty(), "worker_thread must be populated");
}

fn busy_work() {
    // Tiny sleep so start_ns != end_ns even on fast machines.
    thread::sleep(std::time::Duration::from_micros(50));
}

fn busy_work_long() {
    thread::sleep(std::time::Duration::from_millis(1));
}
