#![allow(missing_docs)]

use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

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
}

impl fmt::Display for VisualDebugError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(e) => write!(f, "validation error: {e}"),
            Self::NonEmptyOutputRequiresOverwrite => {
                write!(f, "output directory is non-empty; use --overwrite")
            }
            Self::Write(e) => write!(f, "write error: {e}"),
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

pub fn run_visual_debug(
    req: VisualDebugRequest,
    output_dir: &Path,
    overwrite: bool,
) -> Result<PathBuf, VisualDebugError> {
    let ValidatedRequest(req) = validate_request(req).map_err(VisualDebugError::Validation)?;
    if output_dir.exists() {
        let mut entries =
            fs::read_dir(output_dir).map_err(|e| VisualDebugError::Write(e.to_string()))?;
        if entries.next().is_some() && !overwrite {
            return Err(VisualDebugError::NonEmptyOutputRequiresOverwrite);
        }
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

    let scale = req.resolution_scale;
    let viewport = Viewport {
        width: 1024 * scale,
        height: 1024 * scale,
    };
    let (source, ir, parser) = match &req.source {
        VisualDebugSource::Model { model, .. } => (
            ManifestSource {
                kind: "model".into(),
                model: model.clone(),
                path: None,
            },
            Some(VERSION.into()),
            None,
        ),
        VisualDebugSource::Gcode { path, .. } => (
            ManifestSource {
                kind: "gcode".into(),
                model: None,
                path: path.clone(),
            },
            None,
            Some(VERSION.into()),
        ),
    };
    let source_name = source.kind.clone();
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
                source: source_name.clone(),
                tap: tap.clone(),
                layer_index,
                layer_z,
                visualization: name.to_owned(),
                png_path: format!("images/{}_{}_l{}.png", tap, name, layer_index),
                viewport: viewport.clone(),
                legend_version: VERSION.into(),
                ir_schema_version: ir.clone(),
                gcode_parser_version: parser.clone(),
                warnings: Vec::new(),
            });
        }
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
