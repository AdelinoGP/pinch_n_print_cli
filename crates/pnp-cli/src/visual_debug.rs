#![allow(missing_docs)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[path = "visual_debug_gcode.rs"]
pub mod visual_debug_gcode;

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
    /// What the bundle's shared viewport is framed to. Defaults to
    /// [`FrameMode::Model`].
    ///
    /// `#[serde(default)]` is required, not cosmetic: this struct is
    /// `deny_unknown_fields`, and without a default every request written
    /// before this field existed would fail to deserialize.
    #[serde(default)]
    pub frame: FrameMode,
}

/// What the bundle-wide viewport is framed to.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FrameMode {
    /// The model's own world-space XY extent (unioned with the captured
    /// geometry, so brim/skirt/support are never clipped), plus a fixed
    /// margin. The default, and the only mode a standalone-G-code source
    /// supports.
    #[default]
    Model,
    /// The printer's `bed_shape` polygon extent, plus the same fixed margin.
    /// Frames every render to the whole plate, so a small part renders small
    /// — useful for judging placement, not for inspecting a feature.
    ///
    /// Model-source only: a standalone `.gcode` loads no config, so there is
    /// no bed to read.
    Plate,
}

impl FrameMode {
    /// Stable lowercase name, as accepted in a request and recorded in the
    /// manifest.
    pub fn name(self) -> &'static str {
        match self {
            Self::Model => "model",
            Self::Plate => "plate",
        }
    }
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

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum LayerSelector {
    Index(i64),
    Name(String),
    /// An explicit `{start, end}` layer-index range (inclusive), resolved
    /// against the real layer schedule in Phase 2 (ADR-0041).
    Range {
        start: i64,
        end: i64,
    },
    Detail {
        index: Option<i64>,
        z: Option<f64>,
    },
}

/// Manual `Deserialize` for [`LayerSelector`]: serde's derive has no
/// per-variant `deny_unknown_fields` for an untagged enum (it is only a
/// container attribute), so a naive `#[serde(untagged)]` derive lets an
/// object with unrecognized fields (e.g. a malformed `{start, end, ...}`)
/// silently fall through to `Detail` with every known field defaulted to
/// `None` — the exact bug ADR-0041 fixes. This impl enforces each object
/// variant's field set exactly: `{start, end}` only for `Range`, and a
/// subset of `{index, z}` only for `Detail`; anything else is a hard
/// deserialization error, never a silent empty `Detail`.
impl<'de> Deserialize<'de> for LayerSelector {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error as _;
        let value = serde_json::Value::deserialize(deserializer)?;
        match value {
            serde_json::Value::Number(n) => n
                .as_i64()
                .map(LayerSelector::Index)
                .ok_or_else(|| D::Error::custom("layer selector index must be an integer")),
            serde_json::Value::String(s) => Ok(LayerSelector::Name(s)),
            serde_json::Value::Object(map) => {
                let keys: std::collections::BTreeSet<&str> =
                    map.keys().map(String::as_str).collect();
                let range_keys: std::collections::BTreeSet<&str> =
                    ["start", "end"].into_iter().collect();
                let detail_keys: [&str; 2] = ["index", "z"];
                if keys == range_keys {
                    let start = map["start"].as_i64().ok_or_else(|| {
                        D::Error::custom("layer selector range 'start' must be an integer")
                    })?;
                    let end = map["end"].as_i64().ok_or_else(|| {
                        D::Error::custom("layer selector range 'end' must be an integer")
                    })?;
                    Ok(LayerSelector::Range { start, end })
                } else if keys.iter().all(|k| detail_keys.contains(k)) {
                    let index = match map.get("index") {
                        None | Some(serde_json::Value::Null) => None,
                        Some(v) => Some(v.as_i64().ok_or_else(|| {
                            D::Error::custom("layer selector 'index' must be an integer")
                        })?),
                    };
                    let z = match map.get("z") {
                        None | Some(serde_json::Value::Null) => None,
                        Some(v) => Some(v.as_f64().ok_or_else(|| {
                            D::Error::custom("layer selector 'z' must be a number")
                        })?),
                    };
                    Ok(LayerSelector::Detail { index, z })
                } else {
                    Err(D::Error::custom(format!(
                        "unrecognized layer selector object fields {:?}; expected an integer \
                         index, a {{start, end}} range, or an {{index, z}} detail",
                        map.keys().collect::<Vec<_>>()
                    )))
                }
            }
            other => Err(D::Error::custom(format!("invalid layer selector: {other}"))),
        }
    }
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
    /// A gcode source's `layers` selector contains a variant that isn't
    /// index-based (`LayerSelector::Name` or a z-only `Detail`). The
    /// standalone final-G-code source has no named-layer table — only
    /// `;LAYER_CHANGE`/`;Z:` markers indexed by parse order — so such a
    /// selector is meaningless, not a valid alias for layer 0.
    GcodeUnsupportedLayerSelector,
    MissingField(String),
    /// Phase 1 (ADR-0041): the request names a visualization kind this
    /// packet does not recognize for any source (`filled_areas`,
    /// `filament_lines`, `diagnostic_overlay` are the only supported
    /// kinds). Previously silently skipped by `render_view_for_visualization`'s
    /// dispatch loop (a `None`/`continue`) — now rejected before any render
    /// or bundle write.
    UnknownVisualizationKind {
        kind: String,
    },
    /// Phase 1 (ADR-0041): `diagnostic_overlay` requires a `Model` source —
    /// the standalone final-G-code source has no `PrepassContext`/blackboard
    /// to source the overlay's LayerPlanning flags from. Previously silently
    /// dropped by the gcode branch's visualization filter; now a named
    /// source/visualization mismatch.
    DiagnosticOverlayRequiresModelSource,
    /// Phase 1 (ADR-0041, Model source): a `LayerSelector::Name` selector
    /// has no resolution target — `GlobalLayer` carries `index`/`z`/flags
    /// but no name — so it is rejected rather than silently discarded (the
    /// prior behavior of `resolve_requested_layer_indices`'s catch-all arm).
    /// The standalone G-code source's equivalent rejection is
    /// `GcodeUnsupportedLayerSelector` (preserved separately: it long
    /// predates this packet and its exact variant is pinned by an existing
    /// test).
    AnonymousLayerSelector,
    /// Phase 2 (ADR-0041): a selector (`Index`/`Range`/z-only `Detail`)
    /// resolved against the real layer schedule (model:
    /// `LayerPlanIR.global_layers`; gcode: parsed `;Z:` layers) matched no
    /// layer. Fails closed before any bundle write rather than silently
    /// contributing zero layers.
    LayerSelectorResolvesToNoLayer {
        selector: String,
    },
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SchemaVersion => write!(f, "schema_version must be 1.0.0"),
            Self::MutuallyExclusiveSource => write!(f, "source modes are mutually exclusive"),
            Self::MissingSource => write!(f, "missing source model or path"),
            Self::ResolutionScale => write!(f, "resolution_scale must be 1, 2, 3"),
            Self::GcodeLineWidth => write!(f, "gcode_line_width_mm is required for filled_areas"),
            Self::GcodeUnsupportedLayerSelector => write!(
                f,
                "gcode source layer selectors must be index-based (LayerSelector::Index or \
                 Detail with an index); Name/z-only selectors are not supported"
            ),
            Self::MissingField(field) => write!(f, "missing required field: {field}"),
            Self::UnknownVisualizationKind { kind } => {
                write!(f, "unknown visualization kind: '{kind}'")
            }
            Self::DiagnosticOverlayRequiresModelSource => write!(
                f,
                "diagnostic_overlay requires a model source; the standalone gcode source has \
                 no PrepassContext/blackboard to source it from"
            ),
            Self::AnonymousLayerSelector => write!(
                f,
                "layers are anonymous; LayerSelector::Name has no resolution target"
            ),
            Self::LayerSelectorResolvesToNoLayer { selector } => {
                write!(f, "layer selector {selector} matched no scheduled layer")
            }
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
    /// `frame: "plate"` was requested but the resolved `bed_shape` does not
    /// describe a usable bed polygon. Only reachable after config resolution,
    /// so it cannot be a `ValidationError` (which runs before any config is
    /// loaded). Carries the specific defect.
    InvalidBedShape(String),
    /// The intermediate renderer (packet 159) rejected a requested
    /// visualization for a typed capture — an unsupported `resolution_scale`,
    /// a missing/empty documented geometry field, or a `filled_areas`
    /// request over a typed path with no usable width. Carries the
    /// renderer's own typed-error message verbatim.
    RenderFailed(String),
    /// The standalone final-G-code source (packet 160) contains zero
    /// supported, renderable `G0`/`G1` moves anywhere in the file. A caller
    /// must fail the whole request rather than report a successful
    /// empty/partial bundle — see `visual_debug_gcode::GcodeRenderError::NoRenderableMoves`.
    NoRenderableGcodeMoves(String),
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
            Self::InvalidBedShape(e) => write!(f, "frame: \"plate\" is unusable: {e}"),
            Self::RenderFailed(e) => write!(f, "intermediate render failed: {e}"),
            Self::NoRenderableGcodeMoves(e) => write!(f, "gcode render failed: {e}"),
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
    // Phase 1 (ADR-0041): visualization-kind and source/visualization
    // mismatch checks, before any render or bundle write. Unrecognized
    // kinds and a `diagnostic_overlay` request against a G-code source were
    // previously silently dropped downstream (the model dispatch loop's
    // `None`/`continue`, and the gcode branch's visualization filter); both
    // are now named, fail-closed rejections.
    for viz in &req.visualizations {
        let kind = viz.kind();
        if !matches!(
            kind,
            "filled_areas" | "filament_lines" | "diagnostic_overlay"
        ) {
            return Err(ValidationError::UnknownVisualizationKind {
                kind: kind.to_string(),
            });
        }
        if kind == "diagnostic_overlay" && matches!(req.source, VisualDebugSource::Gcode { .. }) {
            return Err(ValidationError::DiagnosticOverlayRequiresModelSource);
        }
    }
    // NOTE: `frame: "plate"` is supported on BOTH sources. A standalone
    // `.gcode` resolves no printer profile, but it carries the slicer's own
    // config block, whose `printable_area` comment is the bed polygon. That
    // request can only fail once the file is parsed (no such comment), so it
    // is a render-time `NoPrintableArea`, not a validation-time rejection.
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
            // Phase 1 (ADR-0041, Model source): layers are anonymous —
            // `GlobalLayer` carries `index`/`z`/flags but no name — so
            // `LayerSelector::Name` has no resolution target and is
            // rejected here rather than silently discarded by
            // `resolve_requested_layer_indices`'s old catch-all arm. (The
            // standalone G-code source rejects `Name` separately, via the
            // pre-existing `GcodeUnsupportedLayerSelector` check in its own
            // branch of `run_visual_debug`.)
            if req
                .layers
                .iter()
                .any(|l| matches!(l, LayerSelector::Name(_)))
            {
                return Err(ValidationError::AnonymousLayerSelector);
            }
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
    /// What the bundle's shared viewport was framed to (`"model"` /
    /// `"plate"`). Recorded so a consumer can tell a plate-framed render from
    /// a model-framed one without inferring it from the bounds.
    pub frame: String,
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
/// (packet 159) `RenderView`. `validate_request`'s Phase 1 (ADR-0041)
/// rejects any visualization kind this dispatch does not recognize before a
/// `ValidatedRequest` can exist, so every kind reaching this function is
/// guaranteed to be one of the three below — the former silent
/// `None`/`continue` drop is unreachable.
///
/// `diagnostic_overlay` composes with a base geometry view named via
/// `options.base` (`"filled_areas"` | `"filament_lines"`), defaulting to
/// `"filled_areas"` when `options` omits it or isn't a `Detail` spec.
fn render_view_for_visualization(viz: &VisualizationSpec) -> slicer_runtime::RenderView {
    use slicer_runtime::{GeometryView, RenderView};
    match viz.kind() {
        "filled_areas" => RenderView::Geometry(GeometryView::FilledAreas),
        "filament_lines" => RenderView::Geometry(GeometryView::FilamentLines),
        "diagnostic_overlay" => {
            let base = match viz {
                VisualizationSpec::Detail { options, .. } => {
                    options.get("base").and_then(|v| v.as_str())
                }
                VisualizationSpec::Name(_) => None,
            };
            match base {
                Some("filament_lines") => RenderView::DiagnosticOverlay(GeometryView::FilamentLines),
                _ => RenderView::DiagnosticOverlay(GeometryView::FilledAreas),
            }
        }
        other => unreachable!(
            "validate_request rejects unknown visualization kind {other:?} before this dispatch runs"
        ),
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
/// One real, scheduled layer against which `LayerSelector::Index` / `Range`
/// / a z-only `Detail` resolve in Phase 2 (ADR-0041). Source-agnostic: the
/// `Model` source builds this from `LayerPlanIR.global_layers`; the
/// standalone G-code source builds it from parsed `;Z:` layers
/// (`visual_debug_gcode::ParsedLayer`).
#[derive(Debug, Clone, Copy)]
struct ScheduledLayer {
    index: i64,
    z: Option<f64>,
}

/// Resolve every selector in `layers` against the real `schedule` (Phase 2,
/// ADR-0041), failing closed the instant any individual selector matches no
/// scheduled layer — a request is never silently satisfied by a subset of
/// its selectors, and no partial bundle is ever written for the rest.
/// `LayerSelector::Name` must already be rejected by `validate_request`'s
/// Phase 1 (Model source) or the gcode branch's own pre-check (G-code
/// source) by the time this runs; there is no schedule-based resolution for
/// an anonymous name in either universe.
fn resolve_layers_against_schedule(
    layers: &[LayerSelector],
    schedule: &[ScheduledLayer],
) -> Result<Vec<i64>, ValidationError> {
    let mut resolved = std::collections::BTreeSet::new();
    for selector in layers {
        let matches: Vec<i64> = match selector {
            LayerSelector::Index(i) => schedule
                .iter()
                .filter(|l| l.index == *i)
                .map(|l| l.index)
                .collect(),
            LayerSelector::Range { start, end } => schedule
                .iter()
                .filter(|l| l.index >= *start && l.index <= *end)
                .map(|l| l.index)
                .collect(),
            LayerSelector::Detail { index: Some(i), .. } => schedule
                .iter()
                .filter(|l| l.index == *i)
                .map(|l| l.index)
                .collect(),
            LayerSelector::Detail {
                index: None,
                z: Some(z),
            } => schedule
                .iter()
                .filter(|l| l.z.is_some_and(|lz| (lz - z).abs() < 1e-6))
                .map(|l| l.index)
                .collect(),
            LayerSelector::Detail {
                index: None,
                z: None,
            } => Vec::new(),
            LayerSelector::Name(_) => unreachable!(
                "LayerSelector::Name must already be rejected before schedule resolution runs"
            ),
        };
        if matches.is_empty() {
            return Err(ValidationError::LayerSelectorResolvesToNoLayer {
                selector: format!("{selector:?}"),
            });
        }
        resolved.extend(matches);
    }
    Ok(resolved.into_iter().collect())
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

/// The loaded model's world-space XY extent in millimeters, margin included,
/// or `None` if the mesh carries no usable extent (no objects, or a degenerate
/// box) — in which case the caller falls back to the captured geometry's own
/// bounds.
///
/// Reads `MeshIR::build_volume`, which `slicer_model_io::load_model` already
/// computes as the union AABB over every object's vertices. Those vertices are
/// the exact ones the prepass slices (every `ObjectMesh.transform` is identity
/// out of `assemble_object`), so this box and the downstream `SliceIR`
/// polygons share one origin and one mm scale.
///
/// The Z extent is deliberately ignored: this is the model's silhouette across
/// *all* layers, which is precisely why the viewport it produces is stable no
/// matter which layers a request selects.
fn mesh_xy_bounds(mesh: &slicer_ir::MeshIR) -> Option<slicer_runtime::ViewportBoundsMm> {
    if mesh.objects.is_empty() {
        return None;
    }
    let bv = mesh.build_volume;
    let bounds = slicer_runtime::ViewportBoundsMm {
        min_x: bv.min.x,
        min_y: bv.min.y,
        max_x: bv.max.x,
        max_y: bv.max.y,
    };
    let degenerate = !(bounds.min_x.is_finite()
        && bounds.min_y.is_finite()
        && bounds.max_x.is_finite()
        && bounds.max_y.is_finite())
        || bounds.max_x <= bounds.min_x
        || bounds.max_y <= bounds.min_y;
    if degenerate {
        return None;
    }
    Some(bounds.with_margin())
}

/// The printer bed's XY extent in millimeters, margin included, for
/// `frame: "plate"`.
///
/// `bed_shape` is an interleaved `[x0, y0, x1, y1, ...]` polygon in mm
/// (`slicer-ir/src/resolved_config.rs`), so this takes its bounding box —
/// correct for the rectangular beds the default describes, and a sane
/// enclosing frame for a delta's circular bed.
///
/// Fails closed rather than silently falling back to model framing: a request
/// that asked for the plate and quietly got the model would be a misleading
/// image, which is the one thing a debugging tool must never produce.
fn plate_xy_bounds(
    config: &slicer_ir::ResolvedConfig,
) -> Result<slicer_runtime::ViewportBoundsMm, VisualDebugError> {
    let pts = &config.bed_shape;
    if pts.len() < 6 || !pts.len().is_multiple_of(2) {
        return Err(VisualDebugError::InvalidBedShape(format!(
            "bed_shape must be at least 3 points as interleaved [x0, y0, x1, y1, ...] mm; \
             got {} value(s)",
            pts.len()
        )));
    }
    let (mut min_x, mut min_y) = (f64::MAX, f64::MAX);
    let (mut max_x, mut max_y) = (f64::MIN, f64::MIN);
    for xy in pts.chunks_exact(2) {
        if !xy[0].is_finite() || !xy[1].is_finite() {
            return Err(VisualDebugError::InvalidBedShape(
                "bed_shape has a non-finite coordinate".into(),
            ));
        }
        min_x = min_x.min(xy[0]);
        max_x = max_x.max(xy[0]);
        min_y = min_y.min(xy[1]);
        max_y = max_y.max(xy[1]);
    }
    if max_x <= min_x || max_y <= min_y {
        return Err(VisualDebugError::InvalidBedShape(format!(
            "bed_shape encloses no area; got x [{min_x}..{max_x}], y [{min_y}..{max_y}]"
        )));
    }
    Ok(slicer_runtime::ViewportBoundsMm {
        min_x: min_x as f32,
        min_y: min_y as f32,
        max_x: max_x as f32,
        max_y: max_y as f32,
    }
    .with_margin())
}

/// Shared error mapping for both capture closures (arena `execute_captured_stages`
/// and Blackboard-read `execute_blackboard_taps`, packet 161 Step 3): both
/// return `slicer_runtime::CaptureExecutionError`, so one mapping keeps the
/// two call sites in [`run_model_source`] from drifting.
fn map_capture_error(e: slicer_runtime::CaptureExecutionError) -> VisualDebugError {
    match e {
        slicer_runtime::CaptureExecutionError::UnknownTap { tap } => {
            VisualDebugError::UnsupportedTap(tap)
        }
        slicer_runtime::CaptureExecutionError::NoApplicableLayer => {
            VisualDebugError::NoApplicableLayer
        }
        other => VisualDebugError::CaptureFailed(other.to_string()),
    }
}

/// Drives the whole-print pipeline prefix (all layers -> finalization ->
/// postpass) so the two PostPass taps (`PostPass::LayerFinalization`,
/// `PostPass::GCodeEmit` — packet 161, Step 5, the third tap class per
/// ADR-0040) can read IR that only exists after the whole print's per-layer
/// and finalization tiers complete — unlike the arena
/// (`execute_captured_stages`) and Blackboard-read (`execute_blackboard_taps`)
/// closures, there is no bounded per-layer truncation available here: the
/// finalized `Vec<LayerCollectionIR>` and the emitted `GCodeIR` are each a
/// single whole-print artifact.
///
/// This is the one documented minimal-closure deviation: the returned
/// [`slicer_runtime::CaptureOutput`]'s `closure_stage_ids` and
/// `executed_layer_indices` report every stage and every real layer in the
/// print, not just the request's selected subset — even though only the
/// request's selected layers end up as [`slicer_runtime::StageCapture`] rows
/// (the renderer, packet 161 Step 6, further restricts
/// `CapturedIr::LayerFinalization` to just the requested layer when it
/// renders it; `CapturedIr::GCodeEmit` has no per-layer marker to restrict
/// on at all — see `gcode_shapes`'s doc comment in
/// `visual_debug_render.rs` — so it renders the whole captured `GCodeIR`
/// unfiltered).
///
/// The `DefaultGCodeEmitter`/`DefaultGCodeSerializer` instances constructed
/// here are deliberately minimal — no per-object/per-tool resolved-config
/// threading, no thumbnail/CONFIG_BLOCK wiring — mirroring
/// `prepare_prepass_context`'s own "deliberately narrower than `run_slice`"
/// scope note: none of that machinery changes the finalized
/// `LayerCollectionIR`/`GCodeIR` shape this capture needs, only cosmetic
/// G-code formatting choices that belong to `pnp_cli slice`'s production
/// emission path, not visual-debug capture.
fn run_postpass_taps(
    ctx: &mut slicer_runtime::PrepassContext,
    request: &slicer_runtime::CaptureRequest,
) -> Result<slicer_runtime::CaptureOutput, VisualDebugError> {
    for tap in &request.stage_ids {
        if !slicer_runtime::layer_executor::POSTPASS_TAP_STAGE_IDS.contains(&tap.as_str()) {
            return Err(VisualDebugError::UnsupportedTap(tap.clone()));
        }
    }
    if request.stage_ids.is_empty() {
        return Ok(slicer_runtime::CaptureOutput::default());
    }

    // Tier 2: run every per-layer stage for every layer (no truncation —
    // the finalization/emission tiers below need every layer's committed
    // LayerCollectionIR).
    let (mut layer_irs, _layer_audits) = slicer_runtime::execute_per_layer_with_events(
        &ctx.plan,
        &ctx.blackboard,
        &ctx.layer_runner,
        &slicer_runtime::NoopLayerProgressSink,
        &ctx.wasm_handles,
    )
    .map_err(|e| VisualDebugError::CaptureFailed(e.to_string()))?;

    // Tier 3: layer finalization (module-based, if any is bound in this plan).
    slicer_runtime::execute_layer_finalization(
        &ctx.plan,
        &ctx.blackboard,
        &ctx.layer_runner,
        &mut layer_irs,
        &ctx.wasm_handles,
    )
    .map_err(|e| VisualDebugError::CaptureFailed(e.to_string()))?;

    // Tier 4: postpass, with the read-only capture sink enabled so we get
    // back the finalized (travel-reconciled) layers and the initially
    // emitted GCodeIR without altering what would ordinarily be emitted.
    let emitter = slicer_runtime::DefaultGCodeEmitter::new("pnp_cli visual-debug".to_string());
    let serializer = slicer_runtime::DefaultGCodeSerializer::new();
    let mut capture = slicer_runtime::postpass::PostPassCapture::default();
    slicer_runtime::postpass::execute_postpass_with_capture(
        &ctx.plan,
        &layer_irs,
        &ctx.blackboard,
        &emitter,
        &serializer,
        &mut ctx.layer_runner,
        &slicer_runtime::NoopInstrumentation,
        &ctx.wasm_handles,
        Some(&mut capture),
    )
    .map_err(|e| VisualDebugError::CaptureFailed(e.to_string()))?;

    let real_layer_indices: std::collections::BTreeSet<u32> = capture
        .finalized_layers
        .iter()
        .map(|l| l.global_layer_index)
        .collect();
    let applicable: std::collections::BTreeSet<u32> = request
        .layer_indices
        .iter()
        .copied()
        .filter(|i| real_layer_indices.contains(i))
        .collect();
    if applicable.is_empty() {
        return Err(VisualDebugError::NoApplicableLayer);
    }

    let mut captures = Vec::new();
    for &layer_index in &applicable {
        let layer_z = capture
            .finalized_layers
            .iter()
            .find(|l| l.global_layer_index == layer_index)
            .map(|l| l.z)
            .unwrap_or(0.0);
        for tap in &request.stage_ids {
            let ir = match tap.as_str() {
                "PostPass::LayerFinalization" => {
                    slicer_runtime::CapturedIr::LayerFinalization(capture.finalized_layers.clone())
                }
                "PostPass::GCodeEmit" => {
                    slicer_runtime::CapturedIr::GCodeEmit(capture.gcode_ir.clone())
                }
                _ => unreachable!("tap validated against POSTPASS_TAP_STAGE_IDS at function entry"),
            };
            captures.push(slicer_runtime::StageCapture {
                stage_id: tap.clone(),
                layer_index,
                layer_z,
                ir,
            });
        }
    }
    captures.sort_by_key(|c| {
        (
            slicer_runtime::STAGE_ORDER
                .iter()
                .position(|s| *s == c.stage_id.as_str())
                .unwrap_or(usize::MAX),
            c.layer_index,
        )
    });

    // Whole-print closure (this function's documented minimal-closure
    // deviation): every per-layer / finalization / postpass stage, and
    // every real layer in the print — not just the request's selected
    // subset — because neither tap's source IR exists until the whole
    // print has run through postpass.
    let mut closure_stage_ids: Vec<String> = ctx
        .plan
        .per_layer_stages
        .iter()
        .map(|s| s.stage_id.clone())
        .collect();
    closure_stage_ids.push("PostPass::LayerFinalization".to_string());
    closure_stage_ids.push("PostPass::GCodeEmit".to_string());
    closure_stage_ids.extend(ctx.plan.postpass_stages.iter().map(|s| s.stage_id.clone()));
    closure_stage_ids.push("PostPass::GCodeSerialize".to_string());

    Ok(slicer_runtime::CaptureOutput {
        captures,
        expansions: Vec::new(),
        closure_stage_ids,
        executed_layer_indices: real_layer_indices.into_iter().collect(),
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
        if !slicer_runtime::SUPPORTED_TAP_STAGE_IDS.contains(&tap.as_str())
            && !slicer_runtime::layer_executor::BLACKBOARD_TAP_STAGE_IDS.contains(&tap.as_str())
            && !slicer_runtime::layer_executor::POSTPASS_TAP_STAGE_IDS.contains(&tap.as_str())
        {
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

    // A literal empty `layers` list has no selector to resolve at all — fail
    // before ever touching the model/config/modules (AC-N4's coarse guard,
    // preserved from the prior implementation). A non-empty list may still
    // contain a selector that resolves to no layer once the real schedule
    // is known (Phase 2, below) — that is a distinct, later failure mode.
    if req.layers.is_empty() {
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

    // The model-wide XY extent, captured before `mesh` is moved into the
    // prepass context below.
    //
    // `load_model` already computed this as the union AABB over every object
    // (`slicer-model-io/src/loader.rs`, `compute_bounding_box_union`), and
    // `assemble_object` leaves every `ObjectMesh.transform` as identity — a
    // 3MF's authored world transform is baked into its vertices at load. The
    // prepass slices those same vertices through that same (identity)
    // transform, so `build_volume` and the resulting `SliceIR` polygons share
    // one mm origin: there is no centering, auto-arrange, or instance
    // placement anywhere in this pipeline to offset them.
    //
    // This is what makes the viewport *model-wide* rather than
    // selection-wide: it does not vary with which layers a request selected,
    // so two bundles over one model can be compared stage-to-stage.
    let model_bounds = mesh_xy_bounds(&mesh);

    let mut ctx =
        slicer_runtime::prepare_prepass_context(Arc::new(mesh), config_source, module_dirs, false)
            .map_err(|e| VisualDebugError::CaptureFailed(e.to_string()))?;

    // Phase 2 (ADR-0041): resolve every requested layer selector
    // (`Index`/`Range`/z-only `Detail`) against the real, just-committed
    // layer schedule (`LayerPlanIR.global_layers`, a prepass-committed
    // Blackboard slot — available immediately after `prepare_prepass_context`
    // returns, before Tier 2 per-layer execution). Fails closed before any
    // capture/render/bundle write the moment any selector matches no layer.
    let schedule: Vec<ScheduledLayer> = ctx
        .blackboard
        .layer_plan()
        .map(|lp| {
            lp.global_layers
                .iter()
                .map(|gl| ScheduledLayer {
                    index: i64::from(gl.index),
                    z: Some(f64::from(gl.z)),
                })
                .collect()
        })
        .unwrap_or_default();
    let layer_indices: Vec<u32> = resolve_layers_against_schedule(&req.layers, &schedule)
        .map_err(VisualDebugError::Validation)?
        .into_iter()
        .map(|i| i as u32)
        .collect();

    // Split the requested taps into the three closures that source them
    // (ADR-0040 "three tap classes"): the seven arena taps still run
    // `execute_captured_stages`'s truncated per-layer closure; the
    // SliceIR-family/composite taps (packet 161, Steps 3-4) route to the
    // Blackboard-read closure instead — prepass only, no arena, no module
    // dispatch; the two PostPass taps (packet 161, Step 5) route to
    // `run_postpass_taps`, which drives the whole-print pipeline prefix
    // (all layers -> finalization -> postpass) since their source IR does
    // not exist until that whole prefix has run. Tap membership across the
    // three `*_TAP_STAGE_IDS` sets is disjoint (validated above).
    let (arena_tap_ids, rest): (Vec<String>, Vec<String>) = tap_ids
        .into_iter()
        .partition(|t| slicer_runtime::SUPPORTED_TAP_STAGE_IDS.contains(&t.as_str()));
    let (blackboard_tap_ids, postpass_tap_ids): (Vec<String>, Vec<String>) =
        rest.into_iter().partition(|t| {
            slicer_runtime::layer_executor::BLACKBOARD_TAP_STAGE_IDS.contains(&t.as_str())
        });

    let mut arena_output = slicer_runtime::CaptureOutput::default();
    if !arena_tap_ids.is_empty() {
        let capture_request = slicer_runtime::CaptureRequest {
            stage_ids: arena_tap_ids,
            layer_indices: layer_indices.clone(),
        };
        arena_output = slicer_runtime::execute_captured_stages(
            &ctx.plan,
            &ctx.blackboard,
            &ctx.layer_runner,
            &ctx.wasm_handles,
            &capture_request,
        )
        .map_err(map_capture_error)?;
    }

    let mut blackboard_output = slicer_runtime::CaptureOutput::default();
    if !blackboard_tap_ids.is_empty() {
        let capture_request = slicer_runtime::CaptureRequest {
            stage_ids: blackboard_tap_ids,
            layer_indices: layer_indices.clone(),
        };
        blackboard_output = slicer_runtime::layer_executor::execute_blackboard_taps(
            &ctx.blackboard,
            &capture_request,
        )
        .map_err(map_capture_error)?;
    }

    let mut postpass_output = slicer_runtime::CaptureOutput::default();
    if !postpass_tap_ids.is_empty() {
        let capture_request = slicer_runtime::CaptureRequest {
            stage_ids: postpass_tap_ids,
            layer_indices: layer_indices.clone(),
        };
        postpass_output = run_postpass_taps(&mut ctx, &capture_request)?;
    }

    // Merge the three closures' outputs into one, deterministically ordered
    // the same way each closure orders its own captures (STAGE_ORDER
    // position, then layer index) so a bundle mixing arena, Blackboard-read,
    // and PostPass taps still produces one stable manifest order.
    let mut captures = arena_output.captures;
    captures.extend(blackboard_output.captures);
    captures.extend(postpass_output.captures);
    captures.sort_by_key(|c| {
        (
            slicer_runtime::STAGE_ORDER
                .iter()
                .position(|s| *s == c.stage_id.as_str())
                .unwrap_or(usize::MAX),
            c.layer_index,
        )
    });
    let mut expansions = arena_output.expansions;
    expansions.extend(blackboard_output.expansions);
    expansions.extend(postpass_output.expansions);
    // Blackboard-read taps contribute no arena closure (ADR-0040: prepass
    // only). PostPass taps contribute their own whole-print closure
    // (`run_postpass_taps`'s documented minimal-closure deviation — every
    // stage, not just the arena path's truncated per-layer sequence) when
    // requested.
    let mut closure_stage_ids = arena_output.closure_stage_ids;
    closure_stage_ids.extend(postpass_output.closure_stage_ids);
    let mut executed_layer_indices: std::collections::BTreeSet<u32> =
        arena_output.executed_layer_indices.into_iter().collect();
    executed_layer_indices.extend(blackboard_output.executed_layer_indices);
    executed_layer_indices.extend(postpass_output.executed_layer_indices);
    let executed_layer_indices: Vec<u32> = executed_layer_indices.into_iter().collect();

    let output = slicer_runtime::CaptureOutput {
        captures,
        expansions,
        closure_stage_ids,
        executed_layer_indices,
    };

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
        let viewport_bounds = match req.frame {
            // Plate framing is exactly the bed: it must NOT widen to the
            // geometry, or "frame to the plate" would stop meaning the plate
            // the moment anything sat near an edge.
            FrameMode::Plate => plate_xy_bounds(&ctx.default_resolved_config)?,
            // Model-wide extent, widened to cover anything the captures put
            // outside the mesh footprint. The union is load-bearing, not
            // defensive: brim, skirt, and support all extrude beyond the
            // model's own XY silhouette, and a mesh-only viewport would
            // silently clip them off the edge of the raster. It only ever
            // grows, so framing stays mesh-dominated and stable across
            // requests.
            FrameMode::Model => {
                let captured = slicer_runtime::compute_viewport_bounds(&output.captures);
                model_bounds.map_or(captured, |m| m.union(captured))
            }
        };
        for capture in &output.captures {
            for viz in &req.visualizations {
                let render_view = render_view_for_visualization(viz);
                // `LayerPlanIR` diagnostic overlay (packet 161, Step 7):
                // `LayerPlanning` has no standalone tap/`CapturedIr`
                // variant, so its sync/non-planar/active-region flags only
                // ever reach a rendered image here, threaded from the
                // Model-source `PrepassContext::blackboard` opt-in — never
                // drawn for a plain `RenderView::Geometry` request (see
                // `render_stage_capture`'s doc comment), and never available
                // at all for the standalone G-code source (no
                // `PrepassContext`/blackboard exists on that path).
                let layer_plan_layer = if matches!(
                    render_view,
                    slicer_runtime::RenderView::DiagnosticOverlay(_)
                ) {
                    ctx.blackboard.layer_plan().and_then(|lp| {
                        lp.global_layers
                            .iter()
                            .find(|gl| gl.index == capture.layer_index)
                    })
                } else {
                    None
                };
                let rendered =
                    slicer_runtime::visual_debug_render::render_stage_capture_with_layer_plan(
                        capture,
                        render_view,
                        req.resolution_scale,
                        viewport_bounds,
                        layer_plan_layer,
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
            let gcode_path = path
                .clone()
                .expect("validate_request: gcode source requires `path`");
            let source = ManifestSource {
                kind: "gcode".into(),
                model: None,
                path: Some(gcode_path.clone()),
            };
            // `validate_request`'s Phase 1 (ADR-0041) already rejected any
            // unrecognized visualization kind and any `diagnostic_overlay`
            // request against this G-code source, so only `filament_lines`/
            // `filled_areas` can reach this dispatch — the former silent
            // `filter_map`/`_ => None` drop is unreachable.
            let visualizations: Vec<visual_debug_gcode::GcodeVisualization> = req
                .visualizations
                .iter()
                .map(|v| match v.kind() {
                    "filament_lines" => visual_debug_gcode::GcodeVisualization::FilamentLines,
                    "filled_areas" => visual_debug_gcode::GcodeVisualization::FilledAreas,
                    other => unreachable!(
                        "validate_request rejects unknown/mismatched visualization kind \
                         {other:?} before this dispatch runs"
                    ),
                })
                .collect();

            if visualizations.is_empty() {
                // Nothing to render: skip opening/parsing the gcode file
                // entirely, mirroring `run_model_source`'s "no taps -> no
                // capture" short-circuit. A request that only exercises the
                // standalone bundle/exclusive-source contract (no
                // visualizations) must never require the referenced gcode
                // path to actually exist on disk.
                (
                    source,
                    None,
                    None,
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                )
            } else {
                let taps: Vec<String> = if req.taps.is_empty() {
                    vec![String::new()]
                } else {
                    req.taps.iter().map(tap_name).collect()
                };

                // An empty `layers` list with non-empty `visualizations`
                // must fail the whole command, not silently succeed with an
                // empty bundle (mirrors `run_model_source`'s
                // `NoApplicableLayer` guard for the Model source).
                if req.layers.is_empty() {
                    return Err(VisualDebugError::NoApplicableLayer);
                }
                // The standalone final-G-code source has no named-layer
                // table (only `;LAYER_CHANGE`/`;Z:` markers indexed by parse
                // order): a `LayerSelector::Name` is meaningless here, not a
                // valid alias for layer 0 — reject it explicitly rather than
                // ever falling through to a silent default.
                for layer in &req.layers {
                    if matches!(layer, LayerSelector::Name(_)) {
                        return Err(VisualDebugError::Validation(
                            ValidationError::GcodeUnsupportedLayerSelector,
                        ));
                    }
                }
                // Phase 2 (ADR-0041): resolve every requested layer selector
                // (`Index`/`Range`/z-only `Detail`) against the real, parsed
                // `;Z:` layer schedule. Fails closed before any render/bundle
                // write the moment any selector matches no layer.
                let gcode_text = fs::read_to_string(&gcode_path).map_err(|e| {
                    VisualDebugError::CaptureFailed(format!(
                        "failed to read gcode file {}: {e}",
                        gcode_path.display()
                    ))
                })?;
                let parsed_for_schedule = visual_debug_gcode::parse_gcode(&gcode_text);
                let schedule: Vec<ScheduledLayer> = parsed_for_schedule
                    .layers
                    .iter()
                    .map(|l| ScheduledLayer {
                        index: l.layer_index,
                        z: l.layer_z,
                    })
                    .collect();
                let layer_indices: Vec<i64> =
                    resolve_layers_against_schedule(&req.layers, &schedule)
                        .map_err(VisualDebugError::Validation)?;

                let output = visual_debug_gcode::render_gcode_visual_debug_from_path(
                    &gcode_path,
                    &layer_indices,
                    &visualizations,
                    viewport.width,
                    viewport.height,
                    req.gcode_line_width_mm,
                    match req.frame {
                        FrameMode::Model => visual_debug_gcode::GcodeFrame::Model,
                        FrameMode::Plate => visual_debug_gcode::GcodeFrame::Plate,
                    },
                )
                .map_err(|e| match e {
                    visual_debug_gcode::GcodeRenderError::NoPrintableArea => {
                        VisualDebugError::InvalidBedShape(format!(
                            "{} carries no usable `printable_area` config comment to frame to",
                            gcode_path.display()
                        ))
                    }
                    visual_debug_gcode::GcodeRenderError::Io(msg) => {
                        VisualDebugError::CaptureFailed(format!(
                            "failed to read gcode file {}: {msg}",
                            gcode_path.display()
                        ))
                    }
                    visual_debug_gcode::GcodeRenderError::NoRenderableMoves => {
                        VisualDebugError::NoRenderableGcodeMoves(format!(
                            "{} contains no supported G0/G1 X/Y/Z/E/F renderable moves",
                            gcode_path.display()
                        ))
                    }
                    visual_debug_gcode::GcodeRenderError::MissingLineWidth => {
                        // Defensive only: packet-157 request validation
                        // (`ValidationError::GcodeLineWidth`) already rejects
                        // a `filled_areas` request with no
                        // `gcode_line_width_mm` before this arm ever runs
                        // (AC-N1), so this branch should be unreachable in
                        // practice.
                        VisualDebugError::CaptureFailed(
                            "filled_areas requires an explicit gcode_line_width_mm".into(),
                        )
                    }
                })?;

                let mut images = Vec::new();
                let mut rendered_files: Vec<(String, Vec<u8>)> = Vec::new();
                for image in &output.images {
                    for tap in &taps {
                        let file_name = format!(
                            "{}_{}_l{}.png",
                            sanitize_path_component(tap),
                            image.visualization.name(),
                            image.layer_index
                        );
                        let relative_path = format!("images/{file_name}");
                        rendered_files.push((relative_path.clone(), image.png_bytes.clone()));
                        images.push(ImageEntry {
                            source: "gcode".into(),
                            tap: tap.clone(),
                            layer_index: image.layer_index,
                            layer_z: image.layer_z,
                            visualization: image.visualization.name().to_string(),
                            png_path: relative_path,
                            viewport: viewport.clone(),
                            legend_version: VERSION.into(),
                            ir_schema_version: None,
                            gcode_parser_version: Some(output.parser_version.clone()),
                            warnings: output.warnings.clone(),
                            typed_capture: None,
                            // The whole-file mm viewport every image in this
                            // bundle was projected through. Identical across
                            // entries, like the model path's.
                            world_bounds_mm: Some(output.world_bounds_mm),
                        });
                    }
                }

                (
                    source,
                    None,
                    Some(output.parser_version.clone()),
                    images,
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                    rendered_files,
                )
            }
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
        frame: req.frame.name().into(),
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

#[cfg(test)]
mod framing_tests {
    use super::*;

    fn config_with_bed(bed_shape: Vec<f64>) -> slicer_ir::ResolvedConfig {
        slicer_ir::ResolvedConfig {
            bed_shape,
            ..Default::default()
        }
    }

    /// The default 250x250 bed frames to its own extent plus the shared fixed
    /// margin — the same margin the model path uses, so the two framing modes
    /// are directly comparable.
    #[test]
    fn plate_bounds_are_the_bed_extent_plus_the_fixed_margin() {
        let cfg = config_with_bed(vec![0.0, 0.0, 250.0, 0.0, 250.0, 250.0, 0.0, 250.0]);
        let b = plate_xy_bounds(&cfg).expect("the default bed is usable");
        let m = slicer_runtime::VIEWPORT_MARGIN_MM;
        assert_eq!((b.min_x, b.min_y), (-m, -m));
        assert_eq!((b.max_x, b.max_y), (250.0 + m, 250.0 + m));
    }

    /// A non-rectangular bed (e.g. a delta's circular plate, approximated as a
    /// polygon) frames to its enclosing box.
    #[test]
    fn plate_bounds_take_the_bounding_box_of_a_non_rectangular_bed() {
        // A diamond inscribed in [0, 200]^2.
        let cfg = config_with_bed(vec![100.0, 0.0, 200.0, 100.0, 100.0, 200.0, 0.0, 100.0]);
        let b = plate_xy_bounds(&cfg).expect("a diamond bed is usable");
        let m = slicer_runtime::VIEWPORT_MARGIN_MM;
        assert_eq!((b.min_x, b.max_x), (-m, 200.0 + m));
        assert_eq!((b.min_y, b.max_y), (-m, 200.0 + m));
    }

    /// A bed_shape that cannot describe a plate fails closed. Falling back to
    /// model framing would silently hand back an image that is not the one the
    /// request asked for.
    #[test]
    fn unusable_bed_shapes_are_rejected_rather_than_silently_ignored() {
        for (name, pts) in [
            ("empty", vec![]),
            (
                "two points (a line, not a polygon)",
                vec![0.0, 0.0, 10.0, 10.0],
            ),
            (
                "odd length (a dangling coordinate)",
                vec![0.0, 0.0, 10.0, 10.0, 5.0],
            ),
            ("zero area", vec![5.0, 5.0, 5.0, 5.0, 5.0, 5.0]),
            ("non-finite", vec![0.0, 0.0, f64::NAN, 0.0, 10.0, 10.0]),
        ] {
            let err = plate_xy_bounds(&config_with_bed(pts))
                .expect_err(&format!("{name} bed_shape must be rejected"));
            assert!(
                matches!(err, VisualDebugError::InvalidBedShape(_)),
                "{name}: expected InvalidBedShape, got {err:?}"
            );
        }
    }

    /// A mesh whose loader-computed extent is unusable yields `None`, so the
    /// caller falls back to the captured geometry's own bounds instead of
    /// framing to a degenerate box.
    #[test]
    fn mesh_bounds_are_none_for_a_mesh_with_no_objects() {
        let mesh = slicer_ir::MeshIR {
            objects: vec![],
            ..Default::default()
        };
        assert!(mesh_xy_bounds(&mesh).is_none());
    }
}
