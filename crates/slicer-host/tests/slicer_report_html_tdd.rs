//! TDD coverage for the HTML slicer report.
//!
//! Drives `Collector` directly via the `PipelineInstrumentation` trait —
//! no real WASM, no full pipeline — so the test is fast, deterministic,
//! and exercises the bracket → record → HTML pipeline end-to-end.

use std::thread;

use serde_json::Value;
use slicer_host::instrumentation::{
    EdgeReason, Phase, PipelineInstrumentation, SerialEdge, TierKind,
};
use slicer_host::report::Collector;

/// Helper: extract the JSON block between <script type="application/json"
/// id="slicer-report-data"> and </script>, parse it, and return the Value.
fn extract_llm_json(html: &str) -> Value {
    let start_tag = r#"<script type="application/json" id="slicer-report-data">"#;
    let end_tag = "</script>";
    let start = html
        .find(start_tag)
        .expect("missing slicer-report-data script tag");
    let content_start = start + start_tag.len();
    let content = &html[content_start..];
    let end = content
        .find(end_tag)
        .expect("missing closing </script> for slicer-report-data");
    let json_str = content[..end].trim();
    serde_json::from_str(json_str).expect("slicer-report-data JSON must parse")
}

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

    // ── JSON data block assertions ────────────────────────────
    let json = extract_llm_json(&html);
    assert!(
        json.get("total_wallclock_ms").is_some(),
        "JSON: total_wallclock_ms"
    );
    assert!(
        json.get("peak_host_memory_bytes").is_some(),
        "JSON: peak_host_memory_bytes"
    );
    assert_eq!(json.get("layer_count").and_then(|v| v.as_u64()), Some(3));
    assert!(json.get("module_count").and_then(|v| v.as_u64()).unwrap() > 0);
    assert!(json
        .get("threads_observed")
        .and_then(|v| v.as_array())
        .is_some());
    assert!(
        json.get("max_layers_concurrent")
            .and_then(|v| v.as_u64())
            .is_some(),
        "AC-7: max_layers_concurrent must be present and a number"
    );

    let phases = json.get("phases").expect("JSON: phases object");
    for ph in &["prepass", "perlayer", "postpass"] {
        let p = &phases[ph];
        assert!(
            p["wall_ms"].as_f64().is_some(),
            "JSON: phases.{}.wall_ms",
            ph
        );
        assert!(
            p["worker_total_ms"].as_f64().is_some(),
            "JSON: phases.{}.worker_total_ms",
            ph
        );
    }

    // AC-4: PerLayer wall and worker_total must be non-zero for a non-empty run.
    let perlayer_wall_ms = phases["perlayer"]["wall_ms"]
        .as_f64()
        .expect("AC-4: perlayer.wall_ms missing");
    let perlayer_worker_ms = phases["perlayer"]["worker_total_ms"]
        .as_f64()
        .expect("AC-4: perlayer.worker_total_ms missing");
    assert!(
        perlayer_wall_ms > 0.0,
        "AC-4: phases.perlayer.wall_ms must be > 0 for a non-empty run, got {}",
        perlayer_wall_ms
    );
    assert!(
        perlayer_worker_ms > 0.0,
        "AC-4: phases.perlayer.worker_total_ms must be > 0 for a non-empty run, got {}",
        perlayer_worker_ms
    );

    let mods = json["module_aggregates"]
        .as_array()
        .expect("JSON: module_aggregates array");
    assert!(!mods.is_empty());
    for m in mods {
        assert!(
            m["module_id"].as_str().is_some(),
            "module_aggregates[].module_id"
        );
        assert!(m["calls"].as_u64().is_some());
        assert!(m["total_ms"].as_f64().is_some());
        assert!(m["mean_ms"].as_f64().is_some());
        assert!(m["p95_ms"].as_f64().is_some());
        assert!(
            m["peak_host_delta_bytes"].as_u64().is_some(),
            "AC-7: module_aggregates[].peak_host_delta_bytes"
        );
        assert!(
            m["wasm_peak_bytes"].as_u64().is_some(),
            "AC-7: module_aggregates[].wasm_peak_bytes"
        );
    }

    let layers = json["per_layer_summary"]
        .as_array()
        .expect("JSON: per_layer_summary array");
    assert_eq!(layers.len(), 3);
    for l in layers {
        assert!(
            l["layer_index"].as_u64().is_some(),
            "per_layer_summary[].layer_index"
        );
        assert!(l["z_mm"].as_f64().is_some());
        assert!(l["duration_ms"].as_f64().is_some());
        assert!(l["worker"].as_str().is_some());
        assert!(
            l["stages"].as_u64().is_some(),
            "AC-7: per_layer_summary[].stages"
        );
        assert!(
            l["modules"].as_u64().is_some(),
            "AC-7: per_layer_summary[].modules"
        );
        assert!(
            l["host_delta_bytes"].as_i64().is_some(),
            "AC-7: per_layer_summary[].host_delta_bytes"
        );
        assert!(
            l["host_peak_bytes"].as_u64().is_some(),
            "AC-7: per_layer_summary[].host_peak_bytes"
        );
    }

    // AC-4: PerLayer HTML row shows distinct Wall and Worker total cells.
    // Row format: <tr><td class="tier-perlayer">PerLayer</td>
    //               <td>{wall}</td><td>{worker}</td><td>{count}</td></tr>
    let row_marker = r#"<tr><td class="tier-perlayer">PerLayer</td>"#;
    let row_start = html
        .find(row_marker)
        .expect("AC-4: PerLayer row missing from Phase Totals");
    let after_marker = &html[row_start + row_marker.len()..];
    let row_end = after_marker
        .find("</tr>")
        .expect("AC-4: malformed PerLayer row (no </tr>)");
    let row_cells_html = &after_marker[..row_end];
    let mut cells: Vec<&str> = Vec::new();
    let mut cursor = row_cells_html;
    while let Some(td_open) = cursor.find("<td>") {
        let value_start = td_open + "<td>".len();
        let value_end = cursor[value_start..]
            .find("</td>")
            .expect("AC-4: unbalanced <td>");
        cells.push(&cursor[value_start..value_start + value_end]);
        cursor = &cursor[value_start + value_end + "</td>".len()..];
    }
    assert_eq!(
        cells.len(),
        3,
        "AC-4: PerLayer row must have wall, worker, count cells, got {:?}",
        cells
    );
    let wall_cell: f64 = cells[0]
        .parse()
        .expect("AC-4: PerLayer Wall cell must be numeric");
    let worker_cell: f64 = cells[1]
        .parse()
        .expect("AC-4: PerLayer Worker total cell must be numeric");
    assert!(
        wall_cell > 0.0,
        "AC-4: PerLayer Wall (ms) cell must be > 0, got {}",
        wall_cell
    );
    assert!(
        worker_cell > 0.0,
        "AC-4: PerLayer Worker total (ms) cell must be > 0, got {}",
        worker_cell
    );

    // ── Document structure ──────────────────────────────────
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

    // AC-N2: JSON keys must not appear in visible HTML elements.
    // Strip the <script> block, then check remaining HTML for leaked keys.
    let start_tag = r#"<script type="application/json" id="slicer-report-data">"#;
    let end_tag = "</script>";
    let script_start = html.find(start_tag).unwrap();
    let script_end = html[script_start..].find(end_tag).unwrap();
    let visible = format!(
        "{}{}",
        &html[..script_start],
        &html[script_start + script_end + end_tag.len()..]
    );
    for leaked in &[
        "total_wallclock_ms",
        "module_aggregates",
        "per_layer_summary",
    ] {
        assert!(
            !visible.contains(leaked),
            "AC-N2: JSON key '{}' must not leak into visible HTML elements",
            leaked
        );
    }
}

#[test]
fn collector_no_phases_produces_empty_but_valid_html() {
    let c = Collector::new("empty.stl");
    let html = c.finish_and_render_to_string();
    assert!(html.starts_with("<!doctype html>"));

    let json = extract_llm_json(&html);
    assert_eq!(json.get("layer_count").and_then(|v| v.as_u64()), Some(0));
    assert!(
        json["per_layer_summary"]
            .as_array()
            .map(|a| a.is_empty())
            .unwrap_or(false),
        "per_layer_summary must be empty array for no-layer report"
    );
    assert!(
        json["module_aggregates"]
            .as_array()
            .map(|a| a.is_empty())
            .unwrap_or(false),
        "module_aggregates must be empty array for no-layer report"
    );
    assert!(
        json["threads_observed"]
            .as_array()
            .map(|a| a.is_empty())
            .unwrap_or(false),
        "AC-8: threads_observed must be empty array for no-layer report"
    );

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
