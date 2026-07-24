//! TDD test file for packet 55: gcode-header-thumbnail-config-blocks.
//!
//! Tests compile today but ALL fail because HEADER_BLOCK, THUMBNAIL_BLOCK, and
//! CONFIG_BLOCK emission do not yet exist in gcode_emit.rs. This is the
//! intended red state â€” tests graduate to green as Steps 2â€“5 implement the
//! sentinels and block content.
//!
//! Acceptance criteria sourced from `.ralph/specs/55_gcode-header-thumbnail-config-blocks/packet.spec.md`.

#![allow(missing_docs)]
#![allow(dead_code)]

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use slicer_ir::{
    ConfigKey, ConfigValue, GlobalLayer, LayerCollectionIR, LayerStageCommit, SemVer, StageId,
};
use slicer_model_io::load_model;
use slicer_runtime::pipeline::{
    run_pipeline_with_raw_config, PipelineConfig, PipelineStageRunners,
};
use slicer_runtime::ExecutionPlan;
use slicer_runtime::{
    CompiledModuleLive, DefaultGCodeEmitter, DefaultGCodeSerializer, FinalizationError,
    FinalizationOutput, FinalizationStageInput, FinalizationStageRunner, LayerStageError,
    LayerStageInput, LayerStageRunner, NoopLayerProgressSink, PostpassError, PostpassOutput,
    PostpassStageInput, PostpassStageRunner, PrepassRunnerError, PrepassStageInput,
    PrepassStageOutput, PrepassStageRunner,
};

use base64::Engine as _;

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// Shared fixtures / helpers
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn stl_fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../resources/test_stl/ASCII/20mmbox-LF.stl")
}

fn fake_thumb_path() -> PathBuf {
    Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../resources/fake_thumb.png"
    ))
    .to_path_buf()
}

fn semver(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}

fn empty_plan() -> ExecutionPlan {
    ExecutionPlan {
        prepass_stages: Vec::new(),
        per_layer_stages: Vec::new(),
        layer_finalization_stage: None,
        postpass_stages: Vec::new(),
        global_layers: Arc::new(Vec::new()),
        region_plans: Arc::new(HashMap::new()),
        module_region_index: HashMap::new(),
        aggregated_region_split: BTreeMap::new(),
    }
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// No-op runners (mirrors e2e_integration_tdd pattern)
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

struct NoopPrepassRunner;
impl PrepassStageRunner for NoopPrepassRunner {
    fn run_stage(
        &self,
        _stage_id: &StageId,
        _module: &CompiledModuleLive<'_>,
        _input: PrepassStageInput<'_>,
    ) -> Result<PrepassStageOutput, PrepassRunnerError> {
        Ok(PrepassStageOutput::None)
    }
}

struct NoopLayerRunner;
impl LayerStageRunner for NoopLayerRunner {
    fn run_stage(
        &self,
        _stage_id: &StageId,
        _layer: &GlobalLayer,
        _module: &CompiledModuleLive<'_>,
        _input: LayerStageInput<'_>,
    ) -> Result<Option<LayerStageCommit>, LayerStageError> {
        Ok(None)
    }
}

struct NoopFinalizationRunner;
impl FinalizationStageRunner for NoopFinalizationRunner {
    fn run_stage(
        &self,
        _stage_id: &StageId,
        _module: &CompiledModuleLive<'_>,
        _input: FinalizationStageInput<'_>,
        _layers: &mut Vec<LayerCollectionIR>,
    ) -> Result<FinalizationOutput, FinalizationError> {
        Ok(FinalizationOutput::Success)
    }
}

struct NoopPostpassRunner;
impl PostpassStageRunner for NoopPostpassRunner {
    fn run_gcode_postprocess(
        &self,
        _stage_id: &StageId,
        _module: &CompiledModuleLive<'_>,
        _input: PostpassStageInput<'_>,
        _commands: &mut Vec<slicer_ir::GCodeCommand>,
    ) -> Result<PostpassOutput, PostpassError> {
        Ok(PostpassOutput::GCodeSuccess)
    }

    fn run_text_postprocess(
        &self,
        _stage_id: &StageId,
        _module: &CompiledModuleLive<'_>,
        _input: PostpassStageInput<'_>,
        text: String,
    ) -> Result<PostpassOutput, PostpassError> {
        Ok(PostpassOutput::TextSuccess { text })
    }
}

fn default_runners() -> PipelineStageRunners {
    PipelineStageRunners {
        prepass: Box::new(NoopPrepassRunner),
        layer: Box::new(NoopLayerRunner),
        finalization: Box::new(NoopFinalizationRunner),
        postpass: Box::new(NoopPostpassRunner),
        emitter: Box::new(DefaultGCodeEmitter::new("pnp_cli-test 0.1.0".into())),
        serializer: Box::new(DefaultGCodeSerializer::new()),
    }
}

/// Run the pipeline with an optional thumbnail path injected via raw config.
/// Returns the gcode text string.
fn slice_to_gcode(thumbnail_path: Option<&str>) -> Result<String, String> {
    slice_to_gcode_with_key(thumbnail_path, None)
}

/// Run the pipeline with an optional thumbnail path AND an optional `thumbnails`
/// config key injected via raw config. Returns the gcode text string.
fn slice_to_gcode_with_key(
    thumbnail_path: Option<&str>,
    thumbnails_key: Option<&str>,
) -> Result<String, String> {
    let mesh_ir =
        Arc::new(load_model(&stl_fixture_path()).map_err(|e| format!("load_model failed: {e:?}"))?);

    let mut raw: HashMap<ConfigKey, ConfigValue> = HashMap::new();
    if let Some(path) = thumbnail_path {
        raw.insert(
            "thumbnail_path".to_string(),
            ConfigValue::String(path.to_string()),
        );
    }
    if let Some(key) = thumbnails_key {
        raw.insert(
            "thumbnails".to_string(),
            ConfigValue::String(key.to_string()),
        );
    }

    let config = PipelineConfig {
        mesh_ir,
        plan: empty_plan(),
        runners: default_runners(),
        resolved_configs: Arc::new(std::collections::BTreeMap::new()),
        default_resolved_config: Arc::new(slicer_ir::ResolvedConfig::default()),
        bounds: Arc::new(slicer_runtime::ConfigBoundsIndex::empty()),
        wasm_handles: Default::default(),
        cancel_flag: None,
        support_tools: Default::default(),
    };

    let output = run_pipeline_with_raw_config(config, &raw, &NoopLayerProgressSink)
        .map_err(|e| format!("pipeline error: {e:?}"))?;
    Ok(output.gcode_text)
}

fn slice_no_thumb() -> String {
    slice_to_gcode(None).expect("pipeline should succeed without thumbnail")
}

fn slice_with_thumb() -> String {
    let p = fake_thumb_path();
    let p_str = p.to_str().expect("path to str");
    slice_to_gcode(Some(p_str)).expect("pipeline should succeed with thumbnail")
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// Helper: count exact occurrences of a substring
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
fn count_occurrences(haystack: &str, needle: &str) -> usize {
    let mut count = 0;
    let mut start = 0;
    while let Some(pos) = haystack[start..].find(needle) {
        count += 1;
        start += pos + needle.len();
    }
    count
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// Helper: extract the text region between two sentinel lines
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
fn region_between<'a>(gcode: &'a str, start_sentinel: &str, end_sentinel: &str) -> &'a str {
    let start = gcode.find(start_sentinel).unwrap_or(0) + start_sentinel.len();
    let end = gcode.find(end_sentinel).unwrap_or(gcode.len());
    &gcode[start..end]
}

/// A decoded thumbnail entry parsed out of the THUMBNAIL_BLOCK inner framing.
#[derive(Debug)]
struct ParsedThumbEntry {
    tag: String,
    width: u32,
    height: u32,
    declared_len: usize,
    base64: String,
}

/// Parse the inner-framed THUMBNAIL_BLOCK region into decoded entries.
///
/// Each entry is framed as:
/// ```text
/// ; <tag> begin <W>x<H> <len>
/// ; <base64_chunk>
/// ...
/// ; <tag> end
/// ```
fn parse_thumb_entries(thumb_region: &str) -> Vec<ParsedThumbEntry> {
    let mut entries = Vec::new();
    let mut current: Option<ParsedThumbEntry> = None;
    for raw_line in thumb_region.lines() {
        let line = raw_line.trim_end();
        if line.trim().is_empty() {
            continue;
        }
        let content = line
            .strip_prefix("; ")
            .unwrap_or_else(|| panic!("thumbnail line must start with '; ', got: {line:?}"));
        let tokens: Vec<&str> = content.split_whitespace().collect();
        // begin line: "<tag> begin <W>x<H> <len>"
        if tokens.len() == 4 && tokens[1] == "begin" {
            let tag = tokens[0].to_string();
            let dims: Vec<&str> = tokens[2].splitn(2, 'x').collect();
            let width = dims[0].parse::<u32>().expect("width in begin line");
            let height = dims[1].parse::<u32>().expect("height in begin line");
            let declared_len = tokens[3].parse::<usize>().expect("len in begin line");
            current = Some(ParsedThumbEntry {
                tag,
                width,
                height,
                declared_len,
                base64: String::new(),
            });
            continue;
        }
        // end line: "<tag> end"
        if tokens.len() == 2 && tokens[1] == "end" {
            if let Some(entry) = current.take() {
                entries.push(entry);
            }
            continue;
        }
        // otherwise a base64 chunk line
        if let Some(entry) = current.as_mut() {
            entry.base64.push_str(content);
        }
    }
    entries
}

/// Decode base64 (standard alphabet) via the `base64` dev-dep engine.
fn decode_b64(s: &str) -> Vec<u8> {
    base64::engine::general_purpose::STANDARD
        .decode(s)
        .expect("thumbnail base64 must be valid")
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// POSITIVE TESTS (AC-1 through AC-12)
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// AC-N2: `thumbnails` config key present but no `thumbnail_path` source PNG.
/// With no source image to render, no THUMBNAIL_BLOCK is emitted, but the other
/// sentinels remain present exactly once each.
#[test]
fn sentinels_present_no_thumbnail() {
    let gcode = slice_to_gcode_with_key(None, Some("48x48/PNG"))
        .expect("pipeline should succeed with thumbnails key but no source PNG");

    assert_eq!(
        count_occurrences(&gcode, "; HEADER_BLOCK_START"),
        1,
        "HEADER_BLOCK_START must appear exactly once â€” got:\n{}",
        &gcode[..gcode.len().min(400)]
    );
    assert_eq!(
        count_occurrences(&gcode, "; HEADER_BLOCK_END"),
        1,
        "HEADER_BLOCK_END must appear exactly once"
    );
    assert_eq!(
        count_occurrences(&gcode, "; CONFIG_BLOCK_START"),
        1,
        "CONFIG_BLOCK_START must appear exactly once"
    );
    assert_eq!(
        count_occurrences(&gcode, "; CONFIG_BLOCK_END"),
        1,
        "CONFIG_BLOCK_END must appear exactly once"
    );
    assert!(
        !gcode.contains("; THUMBNAIL_BLOCK_START"),
        "THUMBNAIL_BLOCK_START must NOT appear when no source PNG is provided (AC-N2)"
    );
}

/// AC-2: All six sentinels present when thumbnail is supplied.
#[test]
fn sentinels_present_with_thumbnail() {
    let gcode = slice_with_thumb();

    for sentinel in &[
        "; HEADER_BLOCK_START",
        "; HEADER_BLOCK_END",
        "; THUMBNAIL_BLOCK_START",
        "; THUMBNAIL_BLOCK_END",
        "; CONFIG_BLOCK_START",
        "; CONFIG_BLOCK_END",
    ] {
        assert_eq!(
            count_occurrences(&gcode, sentinel),
            1,
            "sentinel {sentinel:?} must appear exactly once"
        );
    }
}

/// AC-3: Four required header fields are present with non-empty values.
#[test]
fn header_four_required_fields() {
    let gcode = slice_no_thumb();

    for field in &[
        "; total layer number:",
        "; filament_diameter:",
        "; filament_density:",
        "; max_z_height:",
    ] {
        assert_eq!(
            count_occurrences(&gcode, field),
            1,
            "header field {field:?} must appear exactly once"
        );
        // The value after the colon must be non-empty (at least one non-space char on the line)
        let line = gcode
            .lines()
            .find(|l| l.starts_with(field))
            .unwrap_or_else(|| panic!("header field line not found: {field}"));
        let value_part = line[field.len()..].trim();
        assert!(
            !value_part.is_empty(),
            "header field {field:?} must have a non-empty value, got line: {line:?}"
        );
    }
}

/// AC-4: total layer number in header matches actual sliced layer count.
/// Extracts value from header and compares against layer count markers in body.
#[test]
fn header_layer_count_matches_sliced() {
    let gcode = slice_no_thumb();

    // Extract declared layer count from header
    let header_line = gcode
        .lines()
        .find(|l| l.starts_with("; total layer number:"))
        .expect("; total layer number: line must be present in HEADER_BLOCK");
    let declared: u32 = header_line["; total layer number:".len()..]
        .trim()
        .parse()
        .expect("total layer number value must be a valid integer");

    // Count layer markers in the body (OrcaSlicer-style LAYER_CHANGE comments)
    // If none exist because no layers were emitted, declared should be 0.
    let body_layer_count = gcode
        .lines()
        .filter(|l| *l == ";LAYER_CHANGE" || l.starts_with(";LAYER_CHANGE"))
        .count() as u32;

    assert_eq!(
        declared, body_layer_count,
        "header 'total layer number' ({declared}) must match body LAYER_CHANGE count ({body_layer_count})"
    );
}

/// AC-5: max_z_height in header matches the top-layer Z.
/// Extracts value from header and compares against last Z move in body.
#[test]
fn header_max_z_matches_top_layer() {
    let gcode = slice_no_thumb();

    let header_line = gcode
        .lines()
        .find(|l| l.starts_with("; max_z_height:"))
        .expect("; max_z_height: line must be present in HEADER_BLOCK");
    let declared_z: f64 = header_line["; max_z_height:".len()..]
        .trim()
        .parse()
        .expect("max_z_height value must be a valid float");

    // Value must be positive
    assert!(
        declared_z > 0.0,
        "max_z_height must be > 0, got {declared_z}"
    );

    // Find the last Z value emitted in the body (e.g. G1 Z... lines)
    let last_z: Option<f64> = gcode
        .lines()
        .filter_map(|l| {
            if l.starts_with("G1") || l.starts_with("G0") {
                // scan for Z token
                l.split_whitespace()
                    .find(|tok| tok.starts_with('Z'))
                    .and_then(|tok| tok[1..].parse::<f64>().ok())
            } else {
                None
            }
        })
        .next_back();

    if let Some(last_z_body) = last_z {
        assert!(
            (declared_z - last_z_body).abs() < 1e-3,
            "max_z_height ({declared_z}) must match last Z move ({last_z_body}) within 1e-3 mm"
        );
    }
    // If no Z moves exist (empty plan), we only check the value is declared (already done above).
}

/// AC-6: filament line appears and contains at least one tool index.
#[test]
fn header_filament_order_matches_used() {
    let gcode = slice_no_thumb();

    let filament_line = gcode
        .lines()
        .find(|l| l.starts_with("; filament:"))
        .expect("; filament: line must be present in HEADER_BLOCK");

    let value = filament_line["; filament:".len()..].trim();
    assert!(!value.is_empty(), "; filament: must have a non-empty value");
    // Must contain at least one digit (tool index)
    assert!(
        value.chars().any(|c| c.is_ascii_digit()),
        "; filament: value must contain at least one tool index digit, got: {value:?}"
    );
}

/// AC-7: Five extrusion-width comment lines are emitted with numeric values > 0.
#[test]
fn width_comments_emitted() {
    let gcode = slice_no_thumb();

    let width_keys = [
        "; outer_wall_line_width = ",
        "; inner_wall_line_width = ",
        "; sparse_infill_line_width = ",
        "; top_surface_line_width = ",
        "; support_line_width = ",
    ];

    for key in &width_keys {
        assert_eq!(
            count_occurrences(&gcode, key),
            1,
            "width comment {key:?} must appear exactly once"
        );
        let line = gcode
            .lines()
            .find(|l| l.starts_with(key))
            .unwrap_or_else(|| panic!("width comment line not found: {key}"));
        let value_str = line[key.len()..].trim();
        let value: f64 = value_str
            .parse()
            .unwrap_or_else(|_| panic!("width value must be a valid float, got: {value_str:?}"));
        assert!(
            value > 0.0,
            "width comment {key:?} must have value > 0, got {value}"
        );
    }
}

/// AC-8: CONFIG_BLOCK includes user-passed config keys.
/// Slices with explicit layer_height and sparse_infill_density in raw config and
/// asserts those keys appear in the CONFIG_BLOCK region.
#[test]
fn config_block_includes_user_passed() {
    let mesh_ir = Arc::new(load_model(&stl_fixture_path()).expect("fixture load"));

    let mut raw: HashMap<ConfigKey, ConfigValue> = HashMap::new();
    raw.insert(
        "layer_height".to_string(),
        ConfigValue::String("0.16".to_string()),
    );
    raw.insert(
        "sparse_infill_density".to_string(),
        ConfigValue::String("22.0".to_string()),
    );

    let config = PipelineConfig {
        mesh_ir,
        plan: empty_plan(),
        runners: default_runners(),
        resolved_configs: Arc::new(std::collections::BTreeMap::new()),
        default_resolved_config: Arc::new(slicer_ir::ResolvedConfig::default()),
        bounds: Arc::new(slicer_runtime::ConfigBoundsIndex::empty()),
        wasm_handles: Default::default(),
        cancel_flag: None,
        support_tools: Default::default(),
    };

    let output = run_pipeline_with_raw_config(config, &raw, &NoopLayerProgressSink)
        .expect("pipeline should succeed");
    let gcode = output.gcode_text;

    let config_region = region_between(&gcode, "; CONFIG_BLOCK_START", "; CONFIG_BLOCK_END");

    assert!(
        config_region.contains("; layer_height = 0.16"),
        "CONFIG_BLOCK must contain '; layer_height = 0.16', region:\n{config_region}"
    );
    assert!(
        config_region.contains("; sparse_infill_density = 22")
            || config_region.contains("; sparse_infill_density = 22.0"),
        "CONFIG_BLOCK must contain '; sparse_infill_density = 22[.0]', region:\n{config_region}"
    );
}

/// AC-9: CONFIG_BLOCK is non-empty and has no duplicate keys.
#[test]
fn config_block_covers_effective_config() {
    let gcode = slice_no_thumb();

    let config_region = region_between(&gcode, "; CONFIG_BLOCK_START", "; CONFIG_BLOCK_END");

    // Non-empty: at least one "; key = " line
    let key_lines: Vec<&str> = config_region
        .lines()
        .filter(|l| l.starts_with("; ") && l.contains(" = "))
        .collect();

    assert!(
        !key_lines.is_empty(),
        "CONFIG_BLOCK must contain at least one key-value line"
    );

    // No duplicate keys
    let mut seen_keys = std::collections::HashSet::new();
    for line in &key_lines {
        // Extract key part before " = "
        if let Some(key) = line.split(" = ").next() {
            assert!(
                seen_keys.insert(key),
                "CONFIG_BLOCK contains duplicate key: {key:?}"
            );
        }
    }
}

#[test]
fn config_block_meets_orca_minimum_key_gate() {
    let gcode = slice_no_thumb();
    let config_region = region_between(&gcode, "; CONFIG_BLOCK_START", "; CONFIG_BLOCK_END");

    let key_count = config_region
        .lines()
        .filter(|line| line.starts_with("; ") && line.contains(" = "))
        .count();

    assert!(
        key_count >= 80,
        "CONFIG_BLOCK must contain at least 80 key-value lines, found {key_count}"
    );
}

#[test]
fn config_block_synthesizes_non_bbl_printer_model() {
    let gcode = slice_no_thumb();
    let config_region = region_between(&gcode, "; CONFIG_BLOCK_START", "; CONFIG_BLOCK_END");
    let printer_model_lines: Vec<&str> = config_region
        .lines()
        .filter(|line| line.contains("printer_model"))
        .collect();

    assert_eq!(
        printer_model_lines,
        vec!["; printer_model = Generic PNP Printer"]
    );
    assert!(
        printer_model_lines
            .iter()
            .all(|line| !line.contains("Bambu")),
        "CONFIG_BLOCK printer_model lines must not select Bambu behavior: {printer_model_lines:?}"
    );
}

#[test]
fn config_block_fork_keys_never_shadowed() {
    let mesh_ir = Arc::new(load_model(&stl_fixture_path()).expect("fixture load"));
    let mut raw: HashMap<ConfigKey, ConfigValue> = HashMap::new();
    raw.insert(
        "machine_max_acceleration_extruding".to_string(),
        ConfigValue::String("20000".to_string()),
    );
    raw.insert(
        "printer_model".to_string(),
        ConfigValue::String("MyFork Printer".to_string()),
    );

    let config = PipelineConfig {
        mesh_ir,
        plan: empty_plan(),
        runners: default_runners(),
        resolved_configs: Arc::new(std::collections::BTreeMap::new()),
        default_resolved_config: Arc::new(slicer_ir::ResolvedConfig::default()),
        bounds: Arc::new(slicer_runtime::ConfigBoundsIndex::empty()),
        wasm_handles: Default::default(),
        cancel_flag: None,
        support_tools: Default::default(),
    };
    let output = run_pipeline_with_raw_config(config, &raw, &NoopLayerProgressSink)
        .expect("pipeline should succeed");
    let config_region = region_between(
        &output.gcode_text,
        "; CONFIG_BLOCK_START",
        "; CONFIG_BLOCK_END",
    );

    assert_eq!(
        config_region
            .lines()
            .filter(|line| *line == "; machine_max_acceleration_extruding = 20000")
            .count(),
        1
    );
    assert_eq!(
        config_region
            .lines()
            .filter(|line| *line == "; printer_model = MyFork Printer")
            .count(),
        1
    );
}

/// AC-10: THUMBNAIL_BLOCK inner-framed base64 roundtrip matches input file bytes.
/// The default (no `thumbnails` key) emits a single PNG entry at source
/// dimensions whose decoded body is the source PNG bytes verbatim.
#[test]
fn thumbnail_roundtrip_matches_input() {
    let gcode = slice_with_thumb();

    assert!(
        gcode.contains("; THUMBNAIL_BLOCK_START"),
        "THUMBNAIL_BLOCK_START must be present before roundtrip check"
    );

    let thumb_region = region_between(&gcode, "; THUMBNAIL_BLOCK_START", "; THUMBNAIL_BLOCK_END");
    let entries = parse_thumb_entries(thumb_region);
    assert_eq!(
        entries.len(),
        1,
        "default (no thumbnails key) must emit exactly one entry, got: {entries:?}"
    );
    let entry = &entries[0];
    assert_eq!(entry.tag, "thumbnail", "default entry must be a PNG entry");

    let decoded = decode_b64(&entry.base64);
    let expected = std::fs::read(fake_thumb_path()).expect("fake_thumb.png must be readable");

    assert_eq!(
        decoded, expected,
        "decoded thumbnail bytes must match input file bytes"
    );

    // AC-1: the `; thumbnail begin <W>x<H>` framing dimensions must equal the
    // source PNG's actual IHDR dimensions (4-byte big-endian at bytes 16-19 for
    // width, 20-23 for height).
    assert!(
        expected.len() >= 24,
        "source PNG too short to read IHDR dimensions"
    );
    let ihdr_w = u32::from_be_bytes([expected[16], expected[17], expected[18], expected[19]]);
    let ihdr_h = u32::from_be_bytes([expected[20], expected[21], expected[22], expected[23]]);
    assert_eq!(
        (entry.width, entry.height),
        (ihdr_w, ihdr_h),
        "begin-line dimensions must match source PNG IHDR dimensions"
    );
}

/// AC-N1: a malformed `thumbnails` config key is rejected by the pipeline and no
/// THUMBNAIL_BLOCK is emitted. With a valid `thumbnail_path` but `thumbnails =
/// "48x48/BMP"` (unsupported format), `parse_thumbnails_key` fails and the error
/// is propagated as `PostpassError::GCodeSerialization`.
#[test]
fn rejects_malformed_thumbnails_key() {
    let p = fake_thumb_path();
    let p_str = p.to_str().expect("path to str");
    let result = slice_to_gcode_with_key(Some(p_str), Some("48x48/BMP"));

    let err = result.expect_err("pipeline must reject malformed `thumbnails` key");
    assert!(
        err.contains("48x48/BMP"),
        "error message must reference the offending key token, got: {err:?}"
    );
    assert!(
        !err.contains("THUMBNAIL_BLOCK_START"),
        "on rejection no THUMBNAIL_BLOCK emitted; error must not leak sentinel text"
    );
}

/// AC-11: THUMBNAIL_BLOCK inner framing is present and each base64 line is
/// â‰¤ 78 chars after the "; " prefix (Orca row length).
#[test]
fn thumbnail_base64_chunking_orca_parity() {
    let gcode = slice_with_thumb();

    assert!(
        gcode.contains("; THUMBNAIL_BLOCK_START"),
        "THUMBNAIL_BLOCK_START must be present before chunking check"
    );

    let thumb_region = region_between(&gcode, "; THUMBNAIL_BLOCK_START", "; THUMBNAIL_BLOCK_END");

    // Inner framing must be present.
    let has_begin = thumb_region
        .lines()
        .any(|l| l.starts_with("; thumbnail begin ") && l.contains('x'));
    assert!(
        has_begin,
        "block must contain a '; thumbnail begin <W>x<H> <len>' frame line, got:\n{thumb_region}"
    );
    assert!(
        thumb_region.lines().any(|l| l.trim() == "; thumbnail end"),
        "block must contain a '; thumbnail end' frame line"
    );

    // Every base64 chunk line's content after "; " is â‰¤ 78 chars, and the
    // declared <len> equals the total base64 char count.
    let entries = parse_thumb_entries(thumb_region);
    assert!(!entries.is_empty(), "at least one entry must be parsed");
    for entry in &entries {
        assert_eq!(
            entry.declared_len,
            entry.base64.len(),
            "declared <len> must equal total base64 char count (incl. padding)"
        );
    }
    for line in thumb_region.lines().filter(|l| !l.trim().is_empty()) {
        assert!(
            line.starts_with("; "),
            "every thumbnail block line must start with '; ', got: {line:?}"
        );
        let content = &line[2..]; // strip "; "
                                  // Only base64 chunk lines are bounded; frame lines are short anyway.
        assert!(
            content.len() <= 78,
            "line content must be â‰¤ 78 chars, got {} chars: {content:?}",
            content.len()
        );
    }
}

/// AC (multi-entry resize): `thumbnails = "48x48/PNG,300x300/PNG"` renders two
/// PNG entries whose decoded IHDR dimensions match the framing header.
#[test]
fn thumbnail_multi_entry_resized_png() {
    let p = fake_thumb_path();
    let p_str = p.to_str().expect("path to str");
    let gcode = slice_to_gcode_with_key(Some(p_str), Some("48x48/PNG,300x300/PNG"))
        .expect("pipeline should succeed with multi PNG thumbnails key");

    let thumb_region = region_between(&gcode, "; THUMBNAIL_BLOCK_START", "; THUMBNAIL_BLOCK_END");
    let entries = parse_thumb_entries(thumb_region);
    assert_eq!(
        entries.len(),
        2,
        "two PNG specs must produce two entries, got: {entries:?}"
    );

    let expected_dims = [(48u32, 48u32), (300u32, 300u32)];
    for (entry, (ew, eh)) in entries.iter().zip(expected_dims.iter()) {
        assert_eq!(
            entry.tag, "thumbnail",
            "PNG entries carry the 'thumbnail' tag"
        );
        assert_eq!((entry.width, entry.height), (*ew, *eh), "framing dims");

        let bytes = decode_b64(&entry.base64);
        // PNG signature is 8 bytes; IHDR data begins at byte 16: width (4 BE)
        // then height (4 BE). (8 sig + 4 len + 4 type = 16.)
        assert!(bytes.len() >= 24, "decoded PNG too short to read IHDR");
        let ihdr_w = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
        let ihdr_h = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
        assert_eq!(
            (ihdr_w, ihdr_h),
            (*ew, *eh),
            "decoded PNG IHDR dimensions must match the framing header"
        );
    }
}

/// AC (multiformat): `thumbnails = "64x64/JPG,64x64/QOI"` renders a JPG entry
/// (SOI 0xFF 0xD8) and a QOI entry (magic "qoif").
#[test]
fn thumbnail_jpg_qoi_entries() {
    let p = fake_thumb_path();
    let p_str = p.to_str().expect("path to str");
    let gcode = slice_to_gcode_with_key(Some(p_str), Some("64x64/JPG,64x64/QOI"))
        .expect("pipeline should succeed with JPG+QOI thumbnails key");

    let thumb_region = region_between(&gcode, "; THUMBNAIL_BLOCK_START", "; THUMBNAIL_BLOCK_END");
    let entries = parse_thumb_entries(thumb_region);
    assert_eq!(
        entries.len(),
        2,
        "JPG+QOI specs must produce two entries, got: {entries:?}"
    );

    let jpg = &entries[0];
    assert_eq!(jpg.tag, "thumbnail_JPG", "first entry must be JPG");
    let jpg_bytes = decode_b64(&jpg.base64);
    assert!(
        jpg_bytes.len() >= 2 && jpg_bytes[0] == 0xFF && jpg_bytes[1] == 0xD8,
        "JPG entry must begin with SOI 0xFF 0xD8, got: {:?}",
        &jpg_bytes[..jpg_bytes.len().min(4)]
    );

    let qoi = &entries[1];
    assert_eq!(qoi.tag, "thumbnail_QOI", "second entry must be QOI");
    let qoi_bytes = decode_b64(&qoi.base64);
    assert!(
        qoi_bytes.len() >= 4 && &qoi_bytes[..4] == b"qoif",
        "QOI entry must begin with magic 'qoif', got: {:?}",
        &qoi_bytes[..qoi_bytes.len().min(4)]
    );
}

/// AC-12: Block ordering â€” HEADER before first ;TYPE:, CONFIG after last ;TYPE:.
#[test]
fn block_ordering_header_before_body_config_after() {
    let gcode = slice_no_thumb();

    let header_offset = gcode
        .find("; HEADER_BLOCK_START")
        .expect("HEADER_BLOCK_START must be present");
    let config_offset = gcode
        .find("; CONFIG_BLOCK_START")
        .expect("CONFIG_BLOCK_START must be present");

    // Find the first and last ;TYPE: marker
    let first_type_offset = gcode.find(";TYPE:");
    let last_type_offset = {
        let mut last = None;
        let mut search_from = 0;
        while let Some(pos) = gcode[search_from..].find(";TYPE:") {
            last = Some(search_from + pos);
            search_from += pos + 1;
        }
        last
    };

    if let Some(first_type) = first_type_offset {
        assert!(
            header_offset < first_type,
            "HEADER_BLOCK_START offset ({header_offset}) must be < first ;TYPE: offset ({first_type})"
        );
    }

    if let Some(last_type) = last_type_offset {
        assert!(
            config_offset > last_type,
            "CONFIG_BLOCK_START offset ({config_offset}) must be > last ;TYPE: offset ({last_type})"
        );
    }
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// NEGATIVE TESTS (5)
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Negative-1: Verifies our sentinel assertions are meaningful â€” a gcode string
/// without HEADER_BLOCK_START fails the sentinel check.
/// This test itself PASSES (asserting that a crafted string fails our check)
/// only when the assertion logic is correct. Currently it fails because the
/// real pipeline does not emit the sentinel â€” so the assertion in
/// `sentinels_present_no_thumbnail` fails on the real output.
///
/// This test validates that the *absence* of a sentinel is detectable.
/// It uses a fabricated gcode string, so it should always pass once implemented.
#[test]
fn rejects_missing_sentinel_block() {
    // Fabricate a gcode string that is missing HEADER_BLOCK_START
    let fake_gcode = "; this is gcode without any blocks\nG28\nG1 X10 Y10\n";

    assert_eq!(
        count_occurrences(fake_gcode, "; HEADER_BLOCK_START"),
        0,
        "sanity: fabricated gcode must not contain HEADER_BLOCK_START"
    );

    // The real pipeline output must contain the sentinel â€” this fails now (red state)
    let real_gcode = slice_no_thumb();
    assert!(
        real_gcode.contains("; HEADER_BLOCK_START"),
        "HEADER_BLOCK_START sentinel is absent from real pipeline output â€” not yet implemented"
    );
}

/// Negative-2: Pipeline must return an error for a nonexistent thumbnail file.
#[test]
fn rejects_missing_thumbnail_file() {
    let mesh_ir = Arc::new(load_model(&stl_fixture_path()).expect("fixture load"));
    let mut raw: HashMap<ConfigKey, ConfigValue> = HashMap::new();
    raw.insert(
        "thumbnail_path".to_string(),
        ConfigValue::String("nonexistent_path_that_does_not_exist_12345.png".to_string()),
    );

    let config = PipelineConfig {
        mesh_ir,
        plan: empty_plan(),
        runners: default_runners(),
        resolved_configs: Arc::new(std::collections::BTreeMap::new()),
        default_resolved_config: Arc::new(slicer_ir::ResolvedConfig::default()),
        bounds: Arc::new(slicer_runtime::ConfigBoundsIndex::empty()),
        wasm_handles: Default::default(),
        cancel_flag: None,
        support_tools: Default::default(),
    };

    let result = run_pipeline_with_raw_config(config, &raw, &NoopLayerProgressSink);

    assert!(
        result.is_err(),
        "pipeline must return an error when thumbnail file does not exist, got: Ok(..)"
    );
}

/// Negative-3: Pipeline must return an error for a non-PNG thumbnail file.
#[test]
fn rejects_non_png_thumbnail() {
    // Write 64 bytes of non-PNG data to a temp file
    let tmp = tempfile::Builder::new()
        .suffix(".png")
        .tempfile()
        .expect("temp file creation");
    std::fs::write(tmp.path(), vec![0x00u8; 64]).expect("write non-PNG data");

    let tmp_path = tmp.path().to_str().expect("temp path to str").to_string();

    let mesh_ir = Arc::new(load_model(&stl_fixture_path()).expect("fixture load"));
    let mut raw: HashMap<ConfigKey, ConfigValue> = HashMap::new();
    raw.insert("thumbnail_path".to_string(), ConfigValue::String(tmp_path));

    let config = PipelineConfig {
        mesh_ir,
        plan: empty_plan(),
        runners: default_runners(),
        resolved_configs: Arc::new(std::collections::BTreeMap::new()),
        default_resolved_config: Arc::new(slicer_ir::ResolvedConfig::default()),
        bounds: Arc::new(slicer_runtime::ConfigBoundsIndex::empty()),
        wasm_handles: Default::default(),
        cancel_flag: None,
        support_tools: Default::default(),
    };

    let result = run_pipeline_with_raw_config(config, &raw, &NoopLayerProgressSink);

    assert!(
        result.is_err(),
        "pipeline must return an error for non-PNG thumbnail data, got: Ok(..)"
    );
    let err_msg = format!("{:?}", result.unwrap_err());
    assert!(
        err_msg.to_lowercase().contains("png")
            || err_msg.to_lowercase().contains("invalid")
            || err_msg.to_lowercase().contains("magic")
            || err_msg.to_lowercase().contains("thumbnail"),
        "error message must reference PNG / invalid / magic / thumbnail, got: {err_msg}"
    );
}

/// Negative-4: Even with a minimal/default config, CONFIG_BLOCK sentinels appear.
/// (Same assertion as AC-1 for the config sentinels â€” fails now since
/// CONFIG_BLOCK_START is not yet emitted.)
#[test]
fn empty_config_view_still_emits_sentinels() {
    // Run with zero extra config flags â€” defaults only
    let mesh_ir = Arc::new(load_model(&stl_fixture_path()).expect("fixture load"));
    let raw: HashMap<ConfigKey, ConfigValue> = HashMap::new();

    let config = PipelineConfig {
        mesh_ir,
        plan: empty_plan(),
        runners: default_runners(),
        resolved_configs: Arc::new(std::collections::BTreeMap::new()),
        default_resolved_config: Arc::new(slicer_ir::ResolvedConfig::default()),
        bounds: Arc::new(slicer_runtime::ConfigBoundsIndex::empty()),
        wasm_handles: Default::default(),
        cancel_flag: None,
        support_tools: Default::default(),
    };

    let output = run_pipeline_with_raw_config(config, &raw, &NoopLayerProgressSink)
        .expect("pipeline should succeed with default config");

    let gcode = output.gcode_text;

    assert!(
        gcode.contains("; CONFIG_BLOCK_START"),
        "CONFIG_BLOCK_START must appear even with default config"
    );
    assert!(
        gcode.contains("; CONFIG_BLOCK_END"),
        "CONFIG_BLOCK_END must appear even with default config"
    );
}

/// Negative-5: Header layer count matches body layer markers.
/// Fails now because HEADER_BLOCK / total layer number line doesn't exist yet.
#[test]
fn rejects_layer_count_drift() {
    let gcode = slice_no_thumb();

    // Extract declared count from header
    let header_line = gcode
        .lines()
        .find(|l| l.starts_with("; total layer number:"))
        .expect("; total layer number: must be present (fails in red state)");

    let declared: u32 = header_line["; total layer number:".len()..]
        .trim()
        .parse()
        .expect("total layer number must be a valid integer");

    // Count actual layer markers
    let body_count = gcode
        .lines()
        .filter(|l| *l == ";LAYER_CHANGE" || l.starts_with(";LAYER_CHANGE"))
        .count() as u32;

    assert_eq!(
        declared,
        body_count,
        "header layer count ({declared}) must not drift from body LAYER_CHANGE count ({body_count})"
    );
}
