use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::Serialize;
use slicer_ir::{ConfigValue, ExPolygon, GlobalLayer, SupportGeometryIR};

#[derive(Debug, Serialize)]
pub struct SupportPreviewDoc {
    pub schema_version: String,
    pub units: String,
    pub layer_count: u32,
    pub skipped_intermediate_entries: u32,
    pub layers: Vec<SupportPreviewLayer>,
}

#[derive(Debug, Serialize)]
pub struct SupportPreviewLayer {
    pub layer_index: u32,
    pub z_mm: f64,
    pub support: Vec<SupportPreviewExPolygon>,
}

#[derive(Debug, Serialize)]
pub struct SupportPreviewExPolygon {
    pub contour: Vec<[f64; 2]>,
    pub holes: Vec<Vec<[f64; 2]>>,
}

pub fn run_support_preview(
    input: &Path,
    output: &Path,
    config: Option<&Path>,
    module_dirs: &[PathBuf],
    no_default_module_paths: bool,
) -> Result<(), String> {
    if !input.exists() {
        return Err(format!("input path does not exist: {}", input.display()));
    }

    let mesh = slicer_model_io::load_model(input)
        .map_err(|e| format!("failed to load model {}: {e}", input.display()))?;
    let config_source = match config {
        Some(path) => {
            let text = fs::read_to_string(path)
                .map_err(|e| format!("failed to read config {}: {e}", path.display()))?;
            slicer_runtime::parse_cli_config_source(&text)
                .map_err(|e| format!("failed to parse config {}: {e}", path.display()))?
        }
        None => HashMap::new(),
    };
    let support_enabled = !matches!(
        config_source.get("enable_support"),
        Some(ConfigValue::Bool(false))
    );

    let ctx = slicer_runtime::prepare_prepass_context(
        Arc::new(mesh),
        config_source,
        module_dirs,
        no_default_module_paths,
    )
    .map_err(|e| e.to_string())?;

    let doc = match (support_enabled, ctx.blackboard.support_geometry()) {
        (false, _) => SupportPreviewDoc {
            schema_version: "1.0.0".to_owned(),
            units: "mm".to_owned(),
            layer_count: ctx.plan.global_layers.len() as u32,
            skipped_intermediate_entries: 0,
            layers: Vec::new(),
        },
        (true, Some(geometry)) => build_preview_doc(geometry, ctx.plan.global_layers.as_ref()),
        (true, None) => SupportPreviewDoc {
            schema_version: "1.0.0".to_owned(),
            units: "mm".to_owned(),
            layer_count: ctx.plan.global_layers.len() as u32,
            skipped_intermediate_entries: 0,
            layers: Vec::new(),
        },
    };

    let serialized = serde_json::to_string_pretty(&doc)
        .map_err(|e| format!("failed to serialize preview: {e}"))?;
    let temporary = output.with_extension("tmp");
    fs::write(&temporary, serialized.as_bytes()).map_err(|e| {
        format!(
            "failed to write temporary output {}: {e}",
            temporary.display()
        )
    })?;
    fs::rename(&temporary, output).map_err(|e| {
        format!(
            "failed to finalize output {} from {}: {e}",
            output.display(),
            temporary.display()
        )
    })?;

    Ok(())
}

pub fn build_preview_doc(
    geometry: &SupportGeometryIR,
    global_layers: &[GlobalLayer],
) -> SupportPreviewDoc {
    let mut entries: Vec<_> = geometry.entries.iter().collect();
    entries.sort_by(|(left, _), (right, _)| {
        left.global_support_layer_index
            .cmp(&right.global_support_layer_index)
            .then_with(|| left.object_id.cmp(&right.object_id))
            .then_with(|| left.region_id.cmp(&right.region_id))
    });

    let mut layers = Vec::new();
    let mut skipped_intermediate_entries = 0;

    for (key, polygons) in entries {
        if key.global_support_layer_index == u32::MAX {
            skipped_intermediate_entries += 1;
            continue;
        }

        let layer_index = key.global_support_layer_index;
        let Some(global_layer) = global_layers.get(layer_index as usize) else {
            eprintln!(
                "warning: skipping support geometry entry for out-of-range layer index {layer_index}"
            );
            continue;
        };

        if layers
            .last()
            .is_none_or(|layer: &SupportPreviewLayer| layer.layer_index != layer_index)
        {
            layers.push(SupportPreviewLayer {
                layer_index,
                z_mm: global_layer.z as f64,
                support: Vec::new(),
            });
        }

        let layer = layers
            .last_mut()
            .expect("a support preview layer was just created or already exists");
        layer
            .support
            .extend(polygons.iter().map(|polygon| preview_expolygon(polygon)));
    }

    layers.sort_by_key(|layer| layer.layer_index);

    SupportPreviewDoc {
        schema_version: "1.0.0".to_owned(),
        units: "mm".to_owned(),
        layer_count: global_layers.len() as u32,
        skipped_intermediate_entries,
        layers,
    }
}

fn preview_expolygon(polygon: &ExPolygon) -> SupportPreviewExPolygon {
    let to_mm = |points: &[slicer_ir::Point2]| {
        points
            .iter()
            .map(|point| {
                let (x, y) = point.to_mm();
                [x as f64, y as f64]
            })
            .collect()
    };

    SupportPreviewExPolygon {
        contour: to_mm(&polygon.contour.points),
        holes: polygon
            .holes
            .iter()
            .map(|hole| to_mm(&hole.points))
            .collect(),
    }
}
