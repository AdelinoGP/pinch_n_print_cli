#![allow(missing_docs)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const VERSION: &str = "1.0.0";

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VisualDebugRequest {
    pub schema_version: String,
    pub source: VisualDebugSource,
    pub layers: Vec<LayerSelector>,
    pub taps: Vec<TapSelector>,
    pub visualizations: Vec<VisualizationSpec>,
    #[serde(default = "default_resolution_scale")]
    pub resolution_scale: u32,
    #[serde(default)]
    pub gcode_line_width_mm: Option<f64>,
}

pub fn default_resolution_scale() -> u32 {
    1
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum VisualDebugSource {
    #[serde(rename = "model")]
    Model {
        model: Option<PathBuf>,
        config: Option<PathBuf>,
        #[serde(default)]
        module_dirs: Vec<PathBuf>,
        path: Option<PathBuf>,
    },
    #[serde(rename = "gcode")]
    Gcode {
        path: Option<PathBuf>,
        model: Option<PathBuf>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LayerSelector {
    Index(i64),
    Name(String),
    Detail { index: Option<i64>, z: Option<f64> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TapSelector {
    Name(String),
    Detail { id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum VisualizationSpec {
    Name(String),
    Detail {
        #[serde(rename = "type")]
        kind: String,
        #[serde(default)]
        options: serde_json::Value,
    },
}

impl VisualizationSpec {
    fn kind(&self) -> &str {
        match self {
            Self::Name(name) => name,
            Self::Detail { kind, .. } => kind,
        }
    }
}

#[derive(Debug)]
pub enum ValidationError {
    SchemaVersion,
    MutuallyExclusiveSource,
    MissingSource,
    ResolutionScale,
    GcodeLineWidth,
    MissingField(String),
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SchemaVersion => write!(f, "schema_version must be 1.0.0"),
            Self::MutuallyExclusiveSource => write!(f, "source modes are mutually exclusive"),
            Self::MissingSource => write!(f, "missing source model or path"),
            Self::ResolutionScale => write!(f, "resolution_scale must be 1, 2, 3"),
            Self::GcodeLineWidth => write!(f, "gcode_line_width_mm is required for filled_areas"),
            Self::MissingField(field) => write!(f, "missing required field: {field}"),
        }
    }
}
impl Error for ValidationError {}

impl VisualDebugSource {
    fn validate_model_config(&self) -> Result<(), ValidationError> {
        let Self::Model {
            config,
            module_dirs,
            ..
        } = self
        else {
            return Ok(());
        };
        if config
            .as_ref()
            .is_none_or(|path| path.as_os_str().is_empty())
        {
            return Err(ValidationError::MissingField("config".into()));
        }
        if module_dirs.is_empty() || module_dirs.iter().any(|path| path.as_os_str().is_empty()) {
            return Err(ValidationError::MissingField("module_dirs".into()));
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum VisualDebugError {
    Validation(ValidationError),
    NonEmptyOutputRequiresOverwrite,
    Write(String),
    /// A requested tap is not one of the documented, supported taps
    /// (packet 158). Carries the offending tap name verbatim.
    UnsupportedTap(String),
    /// A tap was selected but no requested layer resolves to a real layer
    /// in the model (packet 158).
    NoApplicableLayer,
    /// Typed tap capture failed for a reason other than an unsupported tap
    /// or an inapplicable layer (model/module load failure, unavailable tap
    /// source, or a fatal executor error) (packet 158).
    CaptureFailed(String),
    /// The intermediate renderer (packet 159) rejected a requested
    /// visualization for a typed capture — an unsupported `resolution_scale`,
    /// a missing/empty documented geometry field, or a `filled_areas`
    /// request over a typed path with no usable width. Carries the
    /// renderer's own typed-error message verbatim.
    RenderFailed(String),
}

impl fmt::Display for VisualDebugError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(e) => write!(f, "validation error: {e}"),
            Self::NonEmptyOutputRequiresOverwrite => {
                write!(f, "output directory is non-empty; use --overwrite")
            }
            Self::Write(e) => write!(f, "write error: {e}"),
            Self::UnsupportedTap(tap) => write!(f, "unsupported visual-debug tap: '{tap}'"),
            Self::NoApplicableLayer => write!(
                f,
                "no requested layer applies to this model; nothing to capture"
            ),
            Self::CaptureFailed(e) => write!(f, "typed tap capture failed: {e}"),
            Self::RenderFailed(e) => write!(f, "intermediate render failed: {e}"),
        }
    }
}
impl Error for VisualDebugError {}

#[derive(Debug, Clone)]
pub struct ValidatedRequest(pub VisualDebugRequest);

pub fn validate_request(req: VisualDebugRequest) -> Result<ValidatedRequest, ValidationError> {
    if req.schema_version != VERSION {
        return Err(ValidationError::SchemaVersion);
    }
    if !(1..=3).contains(&req.resolution_scale) {
        return Err(ValidationError::ResolutionScale);
    }
    match &req.source {
        VisualDebugSource::Model {
            model,
            config: _,
            module_dirs: _,
            path,
        } => {
            if path.is_some() && model.is_some() {
                return Err(ValidationError::MutuallyExclusiveSource);
            }
            if path.is_some() {
                return Err(ValidationError::MutuallyExclusiveSource);
            }
            if model.is_none() {
                return Err(ValidationError::MissingSource);
            }
            req.source.validate_model_config()?;
        }
        VisualDebugSource::Gcode { path, model } => {
            if path.is_some() && model.is_some() {
                return Err(ValidationError::MutuallyExclusiveSource);
            }
            if model.is_some() {
                return Err(ValidationError::MutuallyExclusiveSource);
            }
            if path.is_none() {
                return Err(ValidationError::MissingSource);
            }
            if req
                .visualizations
                .iter()
                .any(|v| v.kind() == "filled_areas")
                && req.gcode_line_width_mm.is_none()
            {
                return Err(ValidationError::GcodeLineWidth);
            }
        }
    }
    Ok(ValidatedRequest(req))
}

#[derive(Debug, Serialize)]
pub struct Manifest {
    pub schema_version: String,
    pub source: ManifestSource,
    pub resolution_scale: u32,
    pub viewport: Viewport,
    pub legend_version: String,
    pub ir_schema_version: Option<String>,
    pub gcode_parser_version: Option<String>,
    pub images: Vec<ImageEntry>,
    pub warnings: Vec<String>,
    /// The per-layer stage closure that actually ran for a typed-tap
    /// capture, in fixed scheduler order (packet 158). Empty when no typed
    /// capture was requested (including the standalone G-code path).
    #[serde(default)]
    pub executed_stage_ids: Vec<String>,
    /// Layers the closure executed for scheduler-fixed-order correctness
    /// but that were not in the request's selected layers (packet 158):
    /// executed, not rendered/retained.
    #[serde(default)]
    pub layer_expansions: Vec<LayerExpansionEntry>,
    /// Global layer indices the closure actually ran the truncated stage
    /// sequence for (follow-up fix: layer-skip). Empty when no typed
    /// capture was requested. Equal to the request's selected layers today
    /// — a non-selected layer is never executed, so it never appears here.
    #[serde(default)]
    pub executed_layer_indices: Vec<i64>,
}

/// One entry in [`Manifest::layer_expansions`].
#[derive(Debug, Serialize)]
pub struct LayerExpansionEntry {
    pub layer_index: i64,
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct ManifestSource {
    pub kind: String,
    pub model: Option<PathBuf>,
    pub path: Option<PathBuf>,
}

#[derive(Debug, Serialize, Clone)]
pub struct Viewport {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Serialize)]
pub struct ImageEntry {
    pub source: String,
    pub tap: String,
    pub layer_index: i64,
    pub layer_z: Option<f64>,
    pub visualization: String,
    pub png_path: String,
    pub viewport: Viewport,
    pub legend_version: String,
    pub ir_schema_version: Option<String>,
    pub gcode_parser_version: Option<String>,
    pub warnings: Vec<String>,
    /// Renderer-owned typed IR payload for a typed-tap capture (packet
    /// 158). `None` for placeholder / standalone G-code entries; a rendered
    /// visualization is out of scope for this packet, so `png_path` stays
    /// empty whenever this is `Some`.
    #[serde(default)]
    pub typed_capture: Option<serde_json::Value>,
    /// The shared, bundle-wide world-space (mm) viewport this entry was
    /// rendered against (follow-up gap fix, AC-4): populated verbatim from
    /// the single `slicer_runtime::compute_viewport_bounds` call
    /// `run_model_source` makes once per bundle, before its per-capture
    /// render loop. Additive alongside `viewport` (pixel raster
    /// width/height) — AC-4 treats "viewport bounds" (world-space) and
    /// "raster dimensions" (pixel) as two distinct properties, and only
    /// `viewport` covered the latter before this field existed. `None` for
    /// an unrendered `typed_ir`-only entry (no visualization requested) or a
    /// standalone G-code-source placeholder entry — both never call the
    /// renderer, so no bounds were computed. Every entry produced by the
    /// per-capture render loop below carries the identical value, byte for
    /// byte, because they all share one `viewport_bounds` binding.
    #[serde(default)]
    pub world_bounds_mm: Option<slicer_runtime::ViewportBoundsMm>,
}

/// Map one requested `VisualizationSpec` to the intermediate renderer's
/// (packet 159) `RenderView`, or `None` for a visualization kind this
/// packet does not render (skipped, not an error — out-of-scope kinds are
/// packet 157/160's concern, not this handoff's).
///
/// `diagnostic_overlay` composes with a base geometry view named via
/// `options.base` (`"filled_areas"` | `"filament_lines"`), defaulting to
/// `"filled_areas"` when `options` omits it or isn't a `Detail` spec.
fn render_view_for_visualization(viz: &VisualizationSpec) -> Option<slicer_runtime::RenderView> {
    use slicer_runtime::{GeometryView, RenderView};
    match viz.kind() {
        "filled_areas" => Some(RenderView::Geometry(GeometryView::FilledAreas)),
        "filament_lines" => Some(RenderView::Geometry(GeometryView::FilamentLines)),
        "diagnostic_overlay" => {
            let base = match viz {
                VisualizationSpec::Detail { options, .. } => {
                    options.get("base").and_then(|v| v.as_str())
                }
                VisualizationSpec::Name(_) => None,
            };
            Some(match base {
                Some("filament_lines") => {
                    RenderView::DiagnosticOverlay(GeometryView::FilamentLines)
                }
                _ => RenderView::DiagnosticOverlay(GeometryView::FilledAreas),
            })
        }
        _ => None,
    }
}

/// Filename disambiguator for a resolved `RenderView`: `Some(base)` only for
/// `DiagnosticOverlay`, naming the composed base geometry view
/// (`"filled_areas"` | `"filament_lines"`) so two `diagnostic_overlay`
/// visualizations with different `options.base` never collide on the same
/// filename (Finding 1). `None` for a plain `Geometry` view — its `viz.kind()`
/// already names the geometry view unambiguously, so no suffix is needed.
fn diagnostic_overlay_base_suffix(view: slicer_runtime::RenderView) -> Option<&'static str> {
    use slicer_runtime::{GeometryView, RenderView};
    match view {
        RenderView::DiagnosticOverlay(GeometryView::FilledAreas) => Some("filled_areas"),
        RenderView::DiagnosticOverlay(GeometryView::FilamentLines) => Some("filament_lines"),
        RenderView::Geometry(_) => None,
    }
}

/// Sanitize a tap/stage id (e.g. `"Layer::Perimeters"`) for use as a
/// filesystem path component: `:` is reserved on Windows (drive letters /
/// alternate data streams), so it is replaced with `_`.
fn sanitize_path_component(s: &str) -> String {
    s.chars().map(|c| if c == ':' { '_' } else { c }).collect()
}

fn tap_name(tap: &TapSelector) -> String {
    match tap {
        TapSelector::Name(s) => s.clone(),
        TapSelector::Detail { id } => id.clone(),
    }
}
fn layer_info(layer: Option<&LayerSelector>) -> (i64, Option<f64>) {
    match layer {
        Some(LayerSelector::Index(i)) => (*i, None),
        Some(LayerSelector::Detail { index, z }) => (index.unwrap_or(0), *z),
        _ => (0, None),
    }
}

/// Resolve `req.layers` selectors to concrete global layer indices for the
/// typed-tap capture executor (packet 158). Only `Index` and
/// `Detail { index: Some(_), .. }` selectors resolve to a layer index —
/// `Name` selectors and z-only `Detail` selectors require a live layer
/// schedule to resolve and are not supported by this packet's closure
/// executor; they contribute no index (surfacing as
/// [`VisualDebugError::NoApplicableLayer`] if they are the only selectors
/// supplied).
fn resolve_requested_layer_indices(layers: &[LayerSelector]) -> Vec<u32> {
    let mut out = Vec::new();
    for layer in layers {
        match layer {
            LayerSelector::Index(i) if *i >= 0 => out.push(*i as u32),
            LayerSelector::Detail { index: Some(i), .. } if *i >= 0 => out.push(*i as u32),
            _ => {}
        }
    }
    out
}

/// Parse the visual-debug request's `source.config` file (the same
/// `--config` JSON format `pnp_cli slice` accepts) into a raw config-source
/// map. `None` (no `config` supplied) yields an empty map — resolved config
/// defaults still apply.
fn load_visual_debug_config(
    config: Option<&Path>,
) -> Result<HashMap<String, slicer_ir::ConfigValue>, VisualDebugError> {
    let Some(path) = config else {
        return Ok(HashMap::new());
    };
    let text = fs::read_to_string(path).map_err(|e| {
        VisualDebugError::CaptureFailed(format!("failed to read config {}: {e}", path.display()))
    })?;
    slicer_runtime::parse_cli_config_source(&text).map_err(|e| {
        VisualDebugError::CaptureFailed(format!("failed to parse config {}: {e}", path.display()))
    })
}

/// The `Model`-source body of [`run_visual_debug`] (packet 158): validates
/// the requested taps and layers, and — only when taps were actually
/// requested — loads the model and modules, runs the scheduler dependency
/// closure through the furthest requested tap, and translates the typed
/// captures into `ImageEntry` rows. An empty `taps` list produces an empty
/// bundle (no model load, no module load, no execution) exactly like the
/// packet-157 placeholder, so requests that only exercise the standalone
/// bundle contract (create/overwrite/atomicity) are unaffected.
#[allow(clippy::type_complexity)]
fn run_model_source(
    model: &Option<PathBuf>,
    config: &Option<PathBuf>,
    module_dirs: &[PathBuf],
    req: &VisualDebugRequest,
    viewport: &Viewport,
) -> Result<
    (
        ManifestSource,
        Option<String>,
        Option<String>,
        Vec<ImageEntry>,
        Vec<String>,
        Vec<LayerExpansionEntry>,
        Vec<i64>,
        Vec<(String, Vec<u8>)>,
    ),
    VisualDebugError,
> {
    let source = ManifestSource {
        kind: "model".into(),
        model: model.clone(),
        path: None,
    };

    let tap_ids: Vec<String> = req.taps.iter().map(tap_name).collect();
    for tap in &tap_ids {
        if !slicer_runtime::SUPPORTED_TAP_STAGE_IDS.contains(&tap.as_str()) {
            return Err(VisualDebugError::UnsupportedTap(tap.clone()));
        }
    }
    if tap_ids.is_empty() {
        // No taps selected: nothing to capture. Model/modules are never
        // touched (AC-N1 — ordinary slicing and no-tap requests must not
        // pay any capture cost).
        return Ok((
            source,
            Some(VERSION.into()),
            None,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        ));
    }

    let layer_indices = resolve_requested_layer_indices(&req.layers);
    if layer_indices.is_empty() {
        return Err(VisualDebugError::NoApplicableLayer);
    }

    let model_path = model
        .clone()
        .expect("validate_request: model source requires `model`");
    let mesh = slicer_model_io::load_model(&model_path).map_err(|e| {
        VisualDebugError::CaptureFailed(format!(
            "failed to load model {}: {e}",
            model_path.display()
        ))
    })?;
    let config_source = load_visual_debug_config(config.as_deref())?;

    let ctx =
        slicer_runtime::prepare_prepass_context(Arc::new(mesh), config_source, module_dirs, false)
            .map_err(|e| VisualDebugError::CaptureFailed(e.to_string()))?;

    let capture_request = slicer_runtime::CaptureRequest {
        stage_ids: tap_ids,
        layer_indices,
    };
    let output = slicer_runtime::execute_captured_stages(
        &ctx.plan,
        &ctx.blackboard,
        &ctx.layer_runner,
        &ctx.wasm_handles,
        &capture_request,
    )
    .map_err(|e| match e {
        slicer_runtime::CaptureExecutionError::UnknownTap { tap } => {
            VisualDebugError::UnsupportedTap(tap)
        }
        slicer_runtime::CaptureExecutionError::NoApplicableLayer => {
            VisualDebugError::NoApplicableLayer
        }
        other => VisualDebugError::CaptureFailed(other.to_string()),
    })?;

    // Renderer-owned typed IR → PNG rendering (packet 159): additive to
    // packet 158's typed capture above. When the request selected no
    // `visualizations`, behavior is unchanged from packet 158 — one
    // unrendered `ImageEntry` per capture, `png_path` empty. When it did,
    // render each requested visualization for each capture (packet 159's
    // pure `slicer_runtime::render_stage_capture`) against one shared,
    // bundle-wide viewport (AC-4) computed once over every selected
    // capture's geometry.
    let mut images: Vec<ImageEntry> = Vec::new();
    let mut rendered_files: Vec<(String, Vec<u8>)> = Vec::new();
    if req.visualizations.is_empty() {
        images.extend(output.captures.iter().map(|capture| ImageEntry {
            source: "model".into(),
            tap: capture.stage_id.clone(),
            layer_index: capture.layer_index as i64,
            layer_z: Some(capture.layer_z as f64),
            visualization: "typed_ir".into(),
            png_path: String::new(),
            viewport: viewport.clone(),
            legend_version: VERSION.into(),
            ir_schema_version: Some(capture.ir.schema_version_string()),
            gcode_parser_version: None,
            warnings: Vec::new(),
            typed_capture: serde_json::to_value(&capture.ir).ok(),
            world_bounds_mm: None,
        }));
    } else {
        let viewport_bounds = slicer_runtime::compute_viewport_bounds(&output.captures);
        for capture in &output.captures {
            for viz in &req.visualizations {
                let Some(render_view) = render_view_for_visualization(viz) else {
                    continue;
                };
                let rendered = slicer_runtime::render_stage_capture(
                    capture,
                    render_view,
                    req.resolution_scale,
                    viewport_bounds,
                )
                .map_err(|e| VisualDebugError::RenderFailed(e.to_string()))?;
                // `viz.kind()` alone collides for two `diagnostic_overlay`
                // visualizations with different `options.base` (both kind
                // "diagnostic_overlay") — append the resolved base geometry
                // view so two different bases never share a filename/manifest
                // row (Finding 1). Non-overlay kinds have no such ambiguity:
                // `viz.kind()` already names the geometry view directly.
                let base_suffix = diagnostic_overlay_base_suffix(render_view);
                let file_name = match base_suffix {
                    Some(base) => format!(
                        "{}_{}_{}_l{}.png",
                        sanitize_path_component(&capture.stage_id),
                        viz.kind(),
                        base,
                        capture.layer_index
                    ),
                    None => format!(
                        "{}_{}_l{}.png",
                        sanitize_path_component(&capture.stage_id),
                        viz.kind(),
                        capture.layer_index
                    ),
                };
                let relative_path = format!("images/{file_name}");
                rendered_files.push((relative_path.clone(), rendered.png_bytes));
                images.push(ImageEntry {
                    source: "model".into(),
                    tap: capture.stage_id.clone(),
                    layer_index: capture.layer_index as i64,
                    layer_z: Some(capture.layer_z as f64),
                    visualization: viz.kind().to_string(),
                    png_path: relative_path,
                    viewport: viewport.clone(),
                    legend_version: VERSION.into(),
                    ir_schema_version: Some(capture.ir.schema_version_string()),
                    gcode_parser_version: None,
                    warnings: Vec::new(),
                    typed_capture: serde_json::to_value(&capture.ir).ok(),
                    world_bounds_mm: Some(viewport_bounds),
                });
            }
        }
    }
    let layer_expansions: Vec<LayerExpansionEntry> = output
        .expansions
        .iter()
        .map(|expansion| LayerExpansionEntry {
            layer_index: expansion.layer_index as i64,
            reason: expansion.reason.clone(),
        })
        .collect();
    let executed_layer_indices: Vec<i64> = output
        .executed_layer_indices
        .iter()
        .map(|i| *i as i64)
        .collect();

    Ok((
        source,
        Some(VERSION.into()),
        None,
        images,
        output.closure_stage_ids,
        layer_expansions,
        executed_layer_indices,
        rendered_files,
    ))
}

pub fn run_visual_debug(
    req: VisualDebugRequest,
    output_dir: &Path,
    overwrite: bool,
) -> Result<PathBuf, VisualDebugError> {
    let ValidatedRequest(req) = validate_request(req).map_err(VisualDebugError::Validation)?;

    // Non-destructive guard only: reject a non-empty `output_dir` without
    // `--overwrite` before doing any other work. This never mutates
    // `output_dir`, so its position relative to the fallible source
    // resolution below is immaterial.
    if output_dir.exists() {
        let mut entries =
            fs::read_dir(output_dir).map_err(|e| VisualDebugError::Write(e.to_string()))?;
        if entries.next().is_some() && !overwrite {
            return Err(VisualDebugError::NonEmptyOutputRequiresOverwrite);
        }
    }

    let scale = req.resolution_scale;
    let viewport = Viewport {
        width: 1024 * scale,
        height: 1024 * scale,
    };
    // Resolve/capture the source BEFORE touching `output_dir` destructively
    // (packet 158 fix): tap/layer validation and typed-tap capture can fail
    // (`UnsupportedTap`, `NoApplicableLayer`, `CaptureFailed`). A rejected
    // request must never wipe or replace an existing bundle — previously the
    // `--overwrite` wipe below ran unconditionally before this fallible
    // step, so an invalid request (e.g. an unknown tap) silently deleted an
    // existing bundle's manifest and left the directory empty.
    let (
        source,
        ir,
        parser,
        images,
        executed_stage_ids,
        layer_expansions,
        executed_layer_indices,
        rendered_files,
    ) = match &req.source {
        VisualDebugSource::Model {
            model,
            config,
            module_dirs,
            ..
        } => run_model_source(model, config, module_dirs, &req, &viewport)?,
        VisualDebugSource::Gcode { path, .. } => {
            let source = ManifestSource {
                kind: "gcode".into(),
                model: None,
                path: path.clone(),
            };
            let taps = if req.taps.is_empty() {
                vec![String::new()]
            } else {
                req.taps.iter().map(tap_name).collect()
            };
            let layer = req.layers.first();
            let (layer_index, layer_z) = layer_info(layer);
            let mut images = Vec::new();
            for visualization in &req.visualizations {
                let name = visualization.kind();
                for tap in &taps {
                    images.push(ImageEntry {
                        source: "gcode".into(),
                        tap: tap.clone(),
                        layer_index,
                        layer_z,
                        visualization: name.to_owned(),
                        png_path: format!("images/{}_{}_l{}.png", tap, name, layer_index),
                        viewport: viewport.clone(),
                        legend_version: VERSION.into(),
                        ir_schema_version: None,
                        gcode_parser_version: Some(VERSION.into()),
                        warnings: Vec::new(),
                        typed_capture: None,
                        world_bounds_mm: None,
                    });
                }
            }
            (
                source,
                None,
                Some(VERSION.to_string()),
                images,
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
            )
        }
    };

    // Only now — after the fallible source resolution above has succeeded —
    // mutate `output_dir`: wipe it (if `--overwrite`'d and non-empty) or
    // create it. A request that failed above never reaches this point, so
    // an existing bundle is left untouched on any validation/capture error.
    if output_dir.exists() {
        if overwrite {
            for entry in
                fs::read_dir(output_dir).map_err(|e| VisualDebugError::Write(e.to_string()))?
            {
                let path = entry
                    .map_err(|e| VisualDebugError::Write(e.to_string()))?
                    .path();
                let result = if path.is_dir() {
                    fs::remove_dir_all(path)
                } else {
                    fs::remove_file(path)
                };
                result.map_err(|e| VisualDebugError::Write(e.to_string()))?;
            }
        }
    } else {
        fs::create_dir_all(output_dir).map_err(|e| VisualDebugError::Write(e.to_string()))?;
    }

    // Write the intermediate renderer's (packet 159) PNG bytes before the
    // manifest that references them by `png_path`, so a write failure here
    // never leaves a manifest pointing at a nonexistent image.
    for (relative_path, bytes) in &rendered_files {
        let file_path = output_dir.join(relative_path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).map_err(|e| VisualDebugError::Write(e.to_string()))?;
        }
        fs::write(&file_path, bytes).map_err(|e| VisualDebugError::Write(e.to_string()))?;
    }

    let manifest = Manifest {
        schema_version: VERSION.into(),
        source,
        resolution_scale: scale,
        viewport,
        legend_version: VERSION.into(),
        ir_schema_version: ir,
        gcode_parser_version: parser,
        images,
        warnings: Vec::new(),
        executed_stage_ids,
        layer_expansions,
        executed_layer_indices,
    };
    let manifest_path = output_dir.join("manifest.json");
    let temp_path = output_dir.join("manifest.json.tmp");
    let result = (|| {
        let json = serde_json::to_vec_pretty(&manifest)
            .map_err(|e| VisualDebugError::Write(e.to_string()))?;
        let mut file =
            fs::File::create(&temp_path).map_err(|e| VisualDebugError::Write(e.to_string()))?;
        file.write_all(&json)
            .map_err(|e| VisualDebugError::Write(e.to_string()))?;
        file.sync_all()
            .map_err(|e| VisualDebugError::Write(e.to_string()))?;
        fs::rename(&temp_path, &manifest_path)
            .map_err(|e| VisualDebugError::Write(e.to_string()))?;
        Ok(manifest_path.clone())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    result
}

pub fn run_cli(
    req_path: &Path,
    output_dir: &Path,
    overwrite: bool,
) -> Result<PathBuf, VisualDebugError> {
    let bytes = fs::read(req_path).map_err(|e| VisualDebugError::Write(e.to_string()))?;
    let request =
        serde_json::from_slice(&bytes).map_err(|e| VisualDebugError::Write(e.to_string()))?;
    run_visual_debug(request, output_dir, overwrite)
}
