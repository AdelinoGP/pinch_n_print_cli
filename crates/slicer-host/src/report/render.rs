//! HTML rendering for the slicer report.
//!
//! Single function `render_html(&Report) -> String`. No template engine —
//! plain `format!`/`write!` against a `String` with inline CSS and a small
//! amount of inline SVG for the parallelism gantt. The output is a single
//! self-contained file with no external assets.

use std::fmt::Write;

use crate::instrumentation::{EdgeReason, SerialEdge, TierKind};

use super::model::{ModuleRecord, ParallelismRecord, Report, StageRecord};

// ── LLM-readable JSON summary structures ──────────────────────────────

#[derive(serde::Serialize)]
struct LlmReport<'a> {
    total_wallclock_ms: f64,
    peak_host_memory_bytes: u64,
    layer_count: u32,
    module_count: u32,
    threads_observed: &'a [String],
    max_layers_concurrent: usize,
    phases: LlmPhases,
    module_aggregates: Vec<LlmModuleAggregate>,
    per_layer_summary: Vec<LlmPerLayerSummary>,
}

#[derive(serde::Serialize)]
struct LlmPhases {
    prepass: LlmPhaseEntry,
    perlayer: LlmPhaseEntry,
    postpass: LlmPhaseEntry,
}

#[derive(serde::Serialize)]
struct LlmPhaseEntry {
    wall_ms: f64,
    worker_total_ms: f64,
}

#[derive(serde::Serialize)]
struct LlmModuleAggregate {
    module_id: String,
    calls: usize,
    total_ms: f64,
    mean_ms: f64,
    p95_ms: f64,
    #[serde(rename = "peak_host_delta_bytes")]
    peak_host_bytes: u64,
    wasm_peak_bytes: u64,
}

#[derive(serde::Serialize)]
struct LlmPerLayerSummary {
    layer_index: u32,
    z_mm: f32,
    duration_ms: f64,
    worker: String,
    stages: usize,
    modules: usize,
    host_delta_bytes: i64,
    host_peak_bytes: u64,
}

fn ns_to_ms(ns: u64) -> f64 {
    ns as f64 / 1_000_000.0
}

const STYLE: &str = r#"
body { font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
       margin: 1.5rem; color: #1a1a1a; background: #fafafa; }
h1 { font-size: 1.4rem; margin-bottom: 0.25rem; }
h2 { font-size: 1.05rem; margin-top: 2rem; margin-bottom: 0.5rem;
     border-bottom: 1px solid #ddd; padding-bottom: 0.2rem; }
table { border-collapse: collapse; margin-bottom: 1rem; font-size: 0.85rem; }
th, td { padding: 0.25rem 0.6rem; text-align: right; border-bottom: 1px solid #eee; }
th:first-child, td:first-child { text-align: left; }
th { background: #f0f0f0; font-weight: 600; }
tr:hover td { background: #f6f6f6; }
.meta { color: #666; font-size: 0.85rem; margin-bottom: 1rem; }
.meta span { margin-right: 1.2rem; }
.gantt { background: #fff; border: 1px solid #ddd; padding: 0.5rem; }
.tier-prepass { color: #2a6; }
.tier-perlayer { color: #36b; }
.tier-postpass { color: #a52; }
details { margin-bottom: 0.3rem; }
summary { cursor: pointer; padding: 0.2rem 0; }
.edge-row { font-size: 0.8rem; color: #555; padding-left: 1.5rem; }
.note { color: #888; font-size: 0.8rem; font-style: italic; margin-top: 0.4rem; }
"#;

fn render_llm_data(out: &mut String, r: &Report) {
    let mut by_module: std::collections::BTreeMap<String, Vec<&ModuleRecord>> =
        std::collections::BTreeMap::new();
    for layer in &r.layers {
        for stage in &layer.stages {
            for module in &stage.modules {
                by_module
                    .entry(module.module_id.clone())
                    .or_default()
                    .push(module);
            }
        }
    }

    let module_aggregates: Vec<LlmModuleAggregate> = by_module
        .iter()
        .map(|(id, calls)| {
            let durations_ns: Vec<u64> = calls.iter().map(|c| c.duration_ns()).collect();
            let total_ns: u64 = durations_ns.iter().sum();
            let mean_ns = if calls.is_empty() {
                0
            } else {
                total_ns / calls.len() as u64
            };
            let p95_ns = percentile_ns(&durations_ns, 0.95);
            let peak_host_bytes = calls.iter().map(|c| c.mem.host_peak).max().unwrap_or(0);
            let wasm_peak_bytes = calls.iter().map(|c| c.mem.wasm_peak).max().unwrap_or(0);
            LlmModuleAggregate {
                module_id: id.clone(),
                calls: calls.len(),
                total_ms: ns_to_ms(total_ns),
                mean_ms: ns_to_ms(mean_ns),
                p95_ms: ns_to_ms(p95_ns),
                peak_host_bytes,
                wasm_peak_bytes,
            }
        })
        .collect();

    let per_layer_summary: Vec<LlmPerLayerSummary> = r
        .layers
        .iter()
        .map(|l| {
            let modules: usize = l.stages.iter().map(|s| s.modules.len()).sum();
            LlmPerLayerSummary {
                layer_index: l.layer_index,
                z_mm: l.z_mm,
                duration_ms: ns_to_ms(l.duration_ns()),
                worker: l.worker_thread.clone(),
                stages: l.stages.len(),
                modules,
                host_delta_bytes: l.mem.host_delta,
                host_peak_bytes: l.mem.host_peak,
            }
        })
        .collect();

    let prepass_worker_ns: u64 = r.prepass.iter().map(|s| s.duration_ns()).sum();
    let perlayer_worker_ns: u64 = r.layers.iter().map(|l| l.duration_ns()).sum();
    let postpass_worker_ns: u64 = r.postpass.iter().map(|s| s.duration_ns()).sum();

    let llm = LlmReport {
        total_wallclock_ms: ns_to_ms(r.slice_meta.total_ns),
        peak_host_memory_bytes: r.slice_meta.peak_host_bytes,
        layer_count: r.slice_meta.layer_count,
        module_count: r.slice_meta.module_count,
        threads_observed: &r.parallelism.threads_observed,
        max_layers_concurrent: r.parallelism.max_layers_concurrent,
        phases: LlmPhases {
            prepass: LlmPhaseEntry {
                wall_ms: ns_to_ms(r.slice_meta.phase_times.prepass_ns),
                worker_total_ms: ns_to_ms(prepass_worker_ns),
            },
            perlayer: LlmPhaseEntry {
                wall_ms: ns_to_ms(r.slice_meta.phase_times.perlayer_ns),
                worker_total_ms: ns_to_ms(perlayer_worker_ns),
            },
            postpass: LlmPhaseEntry {
                wall_ms: ns_to_ms(r.slice_meta.phase_times.postpass_ns),
                worker_total_ms: ns_to_ms(postpass_worker_ns),
            },
        },
        module_aggregates,
        per_layer_summary,
    };

    let json = serde_json::to_string_pretty(&llm).unwrap_or_else(|_| "{}".to_string());
    let _ = write!(
        out,
        "<script type=\"application/json\" id=\"slicer-report-data\">\n{json}\n</script>\n"
    );
}

/// Render the report to a self-contained HTML document.
pub fn render_html(r: &Report) -> String {
    let mut out = String::with_capacity(64 * 1024);
    let _ = write!(out, "<!doctype html><html><head><meta charset=\"utf-8\">");
    let _ = write!(out, "<title>Slicer Report</title><style>{STYLE}</style>");
    let _ = write!(out, "</head><body>");

    render_header(&mut out, r);
    render_phase_summary(&mut out, r);
    render_module_summary(&mut out, r);
    render_per_layer_table(&mut out, r);
    render_per_stage_breakdown(&mut out, r);
    if r.verbose {
        render_per_layer_per_module_detail(&mut out, r);
    }
    render_parallelism(&mut out, &r.parallelism);
    render_serial_edges(&mut out, r);
    render_llm_data(&mut out, r);

    let _ = write!(out, "</body></html>");
    out
}

fn render_per_layer_per_module_detail(out: &mut String, r: &Report) {
    if r.layers.is_empty() {
        return;
    }
    let _ = write!(out, "<h2>Per-Layer-Per-Module (verbose)</h2>");
    let _ = write!(out, "<table><thead><tr><th>Layer</th><th>Stage</th><th>Module</th><th>Duration (ms)</th><th>Host peak</th><th>WASM peak</th></tr></thead><tbody>");
    for layer in &r.layers {
        for stage in &layer.stages {
            for module in &stage.modules {
                let _ = write!(
                    out,
                    "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                    layer.layer_index,
                    escape_html(&stage.stage_id),
                    escape_html(&module.module_id),
                    fmt_ms(module.duration_ns()),
                    fmt_bytes(module.mem.host_peak),
                    fmt_bytes(module.mem.wasm_peak),
                );
            }
        }
    }
    let _ = write!(out, "</tbody></table>");
}

fn fmt_ms(ns: u64) -> String {
    if ns == 0 {
        return "0".into();
    }
    let ms = ns as f64 / 1_000_000.0;
    if ms < 1.0 {
        format!("{:.3}", ms)
    } else if ms < 100.0 {
        format!("{:.2}", ms)
    } else {
        format!("{:.0}", ms)
    }
}

fn fmt_bytes(b: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if b >= GB {
        format!("{:.2} GB", b as f64 / GB as f64)
    } else if b >= MB {
        format!("{:.2} MB", b as f64 / MB as f64)
    } else if b >= KB {
        format!("{:.1} KB", b as f64 / KB as f64)
    } else {
        format!("{} B", b)
    }
}

fn fmt_delta(b: i64) -> String {
    let sign = if b < 0 { "-" } else { "+" };
    let mag = b.unsigned_abs();
    format!("{}{}", sign, fmt_bytes(mag))
}

fn tier_class(t: TierKind) -> &'static str {
    match t {
        TierKind::PrePass => "tier-prepass",
        TierKind::PerLayer => "tier-perlayer",
        TierKind::PostPass => "tier-postpass",
    }
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn render_header(out: &mut String, r: &Report) {
    let m = &r.slice_meta;
    let _ = write!(out, "<h1>Slicer Report</h1>");
    let _ = write!(out, "<div class=\"meta\">");
    let _ = write!(out, "<span>model: {}</span>", escape_html(&m.model_path));
    let _ = write!(out, "<span>started: {}</span>", escape_html(&m.started_at));
    let _ = write!(out, "<span>total: {} ms</span>", fmt_ms(m.total_ns));
    let _ = write!(out, "<span>layers: {}</span>", m.layer_count);
    let _ = write!(out, "<span>module calls: {}</span>", m.module_count);
    let _ = write!(
        out,
        "<span>peak host mem: {}</span>",
        fmt_bytes(m.peak_host_bytes)
    );
    let _ = write!(
        out,
        "<span>threads: {}</span>",
        r.parallelism.threads_observed.len()
    );
    let _ = write!(
        out,
        "<span>max layers concurrent: {}</span>",
        r.parallelism.max_layers_concurrent
    );
    let _ = write!(out, "</div>");
    let _ = write!(
        out,
        "<div class=\"note\">Host built-ins inside prepass (MeshAnalysis, SupportGeometry, RegionMapping) and postpass (GCode emit / serialize) are not bracketed — only user modules appear in the tables.</div>"
    );
}

fn render_phase_summary(out: &mut String, r: &Report) {
    let wall = &r.slice_meta.phase_times;
    let prepass_worker_ns: u64 = r.prepass.iter().map(|s| s.duration_ns()).sum();
    let perlayer_worker_ns: u64 = r.layers.iter().map(|l| l.duration_ns()).sum();
    let postpass_worker_ns: u64 = r.postpass.iter().map(|s| s.duration_ns()).sum();

    let _ = write!(out, "<h2>Phase Totals</h2>");
    let _ = write!(
        out,
        "<table><thead><tr><th>Phase</th><th>Wall (ms)</th><th>Worker total (ms)</th><th>Count</th></tr></thead><tbody>"
    );
    let _ = write!(
        out,
        "<tr><td class=\"tier-prepass\">PrePass</td><td>{}</td><td>{}</td><td>{}</td></tr>",
        fmt_ms(wall.prepass_ns),
        fmt_ms(prepass_worker_ns),
        r.prepass.len()
    );
    let _ = write!(
        out,
        "<tr><td class=\"tier-perlayer\">PerLayer</td><td>{}</td><td>{}</td><td>{}</td></tr>",
        fmt_ms(wall.perlayer_ns),
        fmt_ms(perlayer_worker_ns),
        r.layers.len()
    );
    let _ = write!(
        out,
        "<tr><td class=\"tier-postpass\">PostPass</td><td>{}</td><td>{}</td><td>{}</td></tr>",
        fmt_ms(wall.postpass_ns),
        fmt_ms(postpass_worker_ns),
        r.postpass.len()
    );
    let _ = write!(out, "</tbody></table>");
    let _ = write!(
        out,
        "<div class=\"note\">PerLayer is parallel (rayon). Wall time = elapsed clock; worker total = sum of per-layer durations across all threads. For sequential phases (PrePass, PostPass), the two are identical.</div>"
    );
}

fn render_module_summary(out: &mut String, r: &Report) {
    // Aggregate ModuleRecords across all layers.
    let mut by_module: std::collections::BTreeMap<String, Vec<&ModuleRecord>> =
        std::collections::BTreeMap::new();
    for layer in &r.layers {
        for stage in &layer.stages {
            for module in &stage.modules {
                by_module
                    .entry(module.module_id.clone())
                    .or_default()
                    .push(module);
            }
        }
    }
    if by_module.is_empty() {
        return;
    }
    let _ = write!(out, "<h2>Per-Module Aggregate (per-layer tier)</h2>");
    let _ = write!(out, "<table><thead><tr><th>Module</th><th>Calls</th><th>Total (ms)</th><th>Mean (ms)</th><th>p95 (ms)</th><th>Peak host Δ</th><th>WASM peak</th></tr></thead><tbody>");
    for (id, calls) in by_module {
        let durations_ns: Vec<u64> = calls.iter().map(|c| c.duration_ns()).collect();
        let total_ns: u64 = durations_ns.iter().sum();
        let mean_ns = if calls.is_empty() {
            0
        } else {
            total_ns / calls.len() as u64
        };
        let p95_ns = percentile_ns(&durations_ns, 0.95);
        let peak_host = calls.iter().map(|c| c.mem.host_peak).max().unwrap_or(0);
        let peak_wasm = calls.iter().map(|c| c.mem.wasm_peak).max().unwrap_or(0);
        let _ = write!(
            out,
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            escape_html(&id),
            calls.len(),
            fmt_ms(total_ns),
            fmt_ms(mean_ns),
            fmt_ms(p95_ns),
            fmt_bytes(peak_host),
            fmt_bytes(peak_wasm)
        );
    }
    let _ = write!(out, "</tbody></table>");
}

fn percentile_ns(ns: &[u64], p: f64) -> u64 {
    if ns.is_empty() {
        return 0;
    }
    let mut sorted: Vec<u64> = ns.to_vec();
    sorted.sort();
    let idx = ((sorted.len() as f64) * p).floor() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

fn render_per_layer_table(out: &mut String, r: &Report) {
    if r.layers.is_empty() {
        return;
    }
    let _ = write!(out, "<h2>Per-Layer</h2>");
    let _ = write!(out, "<table><thead><tr><th>Layer</th><th>Z (mm)</th><th>Duration (ms)</th><th>Worker</th><th>Stages</th><th>Modules</th><th>Host Δ</th><th>Host peak</th></tr></thead><tbody>");
    for layer in &r.layers {
        let modules: usize = layer.stages.iter().map(|s| s.modules.len()).sum();
        let _ = write!(
            out,
            "<tr><td>{}</td><td>{:.3}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            layer.layer_index,
            layer.z_mm,
            fmt_ms(layer.duration_ns()),
            escape_html(&layer.worker_thread),
            layer.stages.len(),
            modules,
            fmt_delta(layer.mem.host_delta),
            fmt_bytes(layer.mem.host_peak)
        );
    }
    let _ = write!(out, "</tbody></table>");
}

fn render_per_stage_breakdown(out: &mut String, r: &Report) {
    // Aggregate StageRecord across layers by stage_id.
    let mut by_stage: std::collections::BTreeMap<String, (TierKind, Vec<&StageRecord>)> =
        std::collections::BTreeMap::new();
    for stage in &r.prepass {
        by_stage
            .entry(stage.stage_id.clone())
            .or_insert_with(|| (stage.tier, Vec::new()))
            .1
            .push(stage);
    }
    for layer in &r.layers {
        for stage in &layer.stages {
            by_stage
                .entry(stage.stage_id.clone())
                .or_insert_with(|| (stage.tier, Vec::new()))
                .1
                .push(stage);
        }
    }
    for stage in &r.postpass {
        by_stage
            .entry(stage.stage_id.clone())
            .or_insert_with(|| (stage.tier, Vec::new()))
            .1
            .push(stage);
    }

    if by_stage.is_empty() {
        return;
    }
    let _ = write!(out, "<h2>Per-Stage Aggregate</h2>");
    let _ = write!(out, "<table><thead><tr><th>Stage</th><th>Tier</th><th>Calls</th><th>Total (ms)</th><th>Mean (ms)</th><th>Peak host</th></tr></thead><tbody>");
    for (id, (tier, calls)) in by_stage {
        let total_ns: u64 = calls.iter().map(|s| s.duration_ns()).sum();
        let mean_ns = if calls.is_empty() {
            0
        } else {
            total_ns / calls.len() as u64
        };
        let peak_host = calls.iter().map(|s| s.mem.host_peak).max().unwrap_or(0);
        let _ = write!(
            out,
            "<tr><td>{}</td><td class=\"{}\">{:?}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            escape_html(&id),
            tier_class(tier),
            tier,
            calls.len(),
            fmt_ms(total_ns),
            fmt_ms(mean_ns),
            fmt_bytes(peak_host)
        );
    }
    let _ = write!(out, "</tbody></table>");
}

fn render_parallelism(out: &mut String, p: &ParallelismRecord) {
    if p.per_thread.is_empty() {
        return;
    }
    let _ = write!(out, "<h2>Parallelism (per-layer Gantt)</h2>");

    // Compute scale: total time range across all threads.
    let mut t_min: u64 = u64::MAX;
    let mut t_max: u64 = 0;
    for rows in p.per_thread.values() {
        for &(_, s, e) in rows {
            t_min = t_min.min(s);
            t_max = t_max.max(e);
        }
    }
    if t_max <= t_min {
        return;
    }
    let span = t_max - t_min;
    const WIDTH: u32 = 900;
    const ROW_HEIGHT: u32 = 18;
    let height = (p.per_thread.len() as u32) * ROW_HEIGHT + 30;
    let _ = write!(
        out,
        "<div class=\"gantt\"><svg width=\"{WIDTH}\" height=\"{height}\" xmlns=\"http://www.w3.org/2000/svg\">"
    );
    let colors = [
        "#36b", "#2a6", "#a52", "#b3a", "#6a3", "#a36", "#3ba", "#a63",
    ];
    for (i, (thread, rows)) in p.per_thread.iter().enumerate() {
        let y = 20 + i as u32 * ROW_HEIGHT;
        let color = colors[i % colors.len()];
        // thread label
        let _ = write!(
            out,
            "<text x=\"0\" y=\"{}\" font-size=\"11\">{}</text>",
            y + 12,
            escape_html(thread)
        );
        for &(layer_idx, s, e) in rows {
            let x = 120 + ((s - t_min) as u128 * (WIDTH as u128 - 120) / span as u128) as u32;
            let w = ((e - s).max(1) as u128 * (WIDTH as u128 - 120) / span as u128).max(1) as u32;
            let _ = write!(
                out,
                "<rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{}\" fill=\"{color}\" opacity=\"0.7\"><title>layer {layer_idx}</title></rect>",
                ROW_HEIGHT - 4
            );
        }
    }
    let _ = write!(out, "</svg></div>");
    let _ = write!(
        out,
        "<div class=\"note\">Threads observed: {} · max layers concurrent: {} (sweep-line over per-layer intervals)</div>",
        p.threads_observed.len(),
        p.max_layers_concurrent
    );
}

fn render_serial_edges(out: &mut String, r: &Report) {
    // Collect serial edges by stage from all stage records.
    let mut by_stage: std::collections::BTreeMap<String, Vec<SerialEdge>> =
        std::collections::BTreeMap::new();
    let mut absorb = |stage: &StageRecord| {
        if !stage.serial_edges.is_empty() {
            let entry = by_stage.entry(stage.stage_id.clone()).or_default();
            for edge in &stage.serial_edges {
                if !entry.iter().any(|e| {
                    e.from == edge.from
                        && e.to == edge.to
                        && fmt_reason(&e.reason) == fmt_reason(&edge.reason)
                }) {
                    entry.push(edge.clone());
                }
            }
        }
    };
    for stage in &r.prepass {
        absorb(stage);
    }
    for layer in &r.layers {
        for stage in &layer.stages {
            absorb(stage);
        }
    }
    for stage in &r.postpass {
        absorb(stage);
    }

    if by_stage.is_empty() {
        return;
    }
    let _ = write!(out, "<h2>Serial Edges (why modules ran in order)</h2>");
    // Auto-collapse <details> blocks when there are many stages to keep
    // the initial scroll length compact.
    let details_attr = if by_stage.len() > 3 {
        "details"
    } else {
        "details open"
    };
    for (stage_id, edges) in by_stage {
        let _ = write!(
            out,
            "<{details_attr}><summary><b>{}</b> · {} edge{}</summary>",
            escape_html(&stage_id),
            edges.len(),
            if edges.len() == 1 { "" } else { "s" }
        );
        for edge in edges {
            let _ = write!(
                out,
                "<div class=\"edge-row\">{} → {} &nbsp;<i>({})</i></div>",
                escape_html(&edge.from),
                escape_html(&edge.to),
                escape_html(&fmt_reason(&edge.reason))
            );
        }
        let _ = write!(out, "</details>");
    }
}

fn fmt_reason(r: &EdgeReason) -> String {
    match r {
        EdgeReason::IrWriteRead { writer_path } => format!("IrWriteRead: {writer_path}"),
        EdgeReason::ExplicitRequires => "ExplicitRequires".to_string(),
    }
}
