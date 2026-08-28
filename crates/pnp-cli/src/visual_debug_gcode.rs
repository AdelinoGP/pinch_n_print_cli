//! Packet 160, Step 2 — standalone final-G-code visual-debug parser and
//! renderer.
//!
//! Parses the serialized G-code text written after
//! `PostPass::TextPostProcess` (`docs/01_system_architecture.md` lines
//! 477-497) — the artifact actually handed to a printer, not merely
//! `GCodeIR` — for the documented Pinch 'n Print `G0`/`G1` `X`/`Y`/`Z`/`E`/`F`
//! subset (`docs/specs/visual-pipeline-debug.md`, "Final G-code Path"),
//! tracking `;LAYER_CHANGE`, `;Z:`, `;TYPE:` markers, absolute/relative
//! extrusion-mode markers (`M82`/`M83`), and source line numbers, then
//! rasterizes deterministic PNGs.
//!
//! This module is self-contained: it does not know about `Manifest`,
//! `ImageEntry`, or atomic bundle/file commit (that remains
//! `crate::visual_debug`'s job — see packet 160 Step 3). It exposes a small
//! request/response surface ([`render_gcode_visual_debug`] and
//! [`render_gcode_visual_debug_from_path`]) that a caller supplies resolved
//! layer indices, a resolved pixel canvas size, and an optional
//! `gcode_line_width_mm` to.
//!
//! Coordinate hazard: this module works entirely in plain `f64` millimeters
//! for parsed G-code coordinates and only converts to output pixels — it
//! never touches the crate's internal `1 unit = 100 nm` IR coordinate space
//! (`docs/08_coordinate_system.md`), since it never constructs IR types.
//!
//! Raw macros/commands outside the documented `G0`/`G1` subset are never
//! approximated: they are collected as warnings naming the 1-indexed source
//! line. Role-less extrusion (an extrusion move seen before any `;TYPE:`
//! marker) is retained with role `"unclassified"`, never dropped or guessed,
//! plus one bundle-wide `"unclassified"` warning.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fmt;
use std::fs;
use std::path::Path;

use png::{BitDepth, ColorType, Encoder};
use slicer_runtime::visual_debug_style::{
    self as style, overlay_palette, ColorBy, GlyphKind, OverlayEvent, OverlayKind, ToolColors,
};
use slicer_runtime::{Projector, ViewportBoundsMm};

/// Parser/renderer version string recorded in every bundle produced from a
/// standalone final-G-code source (`Manifest::gcode_parser_version` /
/// `ImageEntry::gcode_parser_version` in `crate::visual_debug`).
pub const GCODE_PARSER_VERSION: &str = "pnp-gcode-visual-debug/1";

// The fixed viewport margin and the mm→pixel projection both live in
// `slicer_runtime::visual_debug_render` now (`VIEWPORT_MARGIN_MM`,
// `Projector`), shared with the typed-IR stage renderer. This module used to
// own a second, independent copy of both; the two drifted (uniform scale here,
// per-axis scale there), so a model rendered from G-code and the same model
// rendered from a pipeline tap were framed differently.

/// Role string used for extrusion moves seen before any `;TYPE:` marker.
/// Never dropped, never guessed as a following role.
const UNCLASSIFIED_ROLE: &str = "unclassified";

// ─────────────────────────────── public API ──────────────────────────────

/// A visualization kind this module knows how to rasterize for a gcode
/// source. Intentionally a small local enum (not `visual_debug`'s
/// `VisualizationSpec`) so this module stays decoupled from the
/// manifest/`ImageEntry` types the caller owns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GcodeVisualization {
    /// Path centerlines colored by semantic role.
    FilamentLines,
    /// Swept extrusion-width shapes using the caller-supplied
    /// `gcode_line_width_mm` stroke width. Bead width is NEVER derived from
    /// `E` values.
    FilledAreas,
    /// One overlay event class rendered in isolation (schema 1.1.0):
    /// extrusion centerlines painted faint gray, this kind's glyphs on top.
    /// Seams are never supported here — final G-code carries no seam marker.
    Overlay(OverlayKind),
}

impl GcodeVisualization {
    pub fn name(&self) -> &'static str {
        match self {
            GcodeVisualization::FilamentLines => "filament_lines",
            GcodeVisualization::FilledAreas => "filled_areas",
            GcodeVisualization::Overlay(_) => "diagnostic_overlay",
        }
    }
}

/// What [`render_gcode_visual_debug`] frames its shared viewport to. The
/// standalone-G-code mirror of `visual_debug`'s `FrameMode`, kept local so
/// this module stays decoupled from the request/manifest types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GcodeFrame {
    /// The parsed geometry's own model-wide extent, plus the fixed margin.
    #[default]
    Model,
    /// The bed from the config block's `printable_area` comment, plus the
    /// fixed margin.
    Plate,
}

/// Failure modes for [`render_gcode_visual_debug`]. All are terminal: no
/// partial PNG/manifest content should be committed by a caller that
/// receives one of these.
pub enum GcodeRenderError {
    /// [`GcodeFrame::Plate`] was requested but the file carries no usable
    /// `printable_area` config comment, so there is no bed to frame to.
    /// Never silently falls back to model framing — that would return an
    /// image other than the one requested.
    NoPrintableArea,
    /// Reading the G-code file from disk failed. Only produced by
    /// [`render_gcode_visual_debug_from_path`], the test-exercised
    /// convenience wrapper (`visual_debug.rs` reads the file itself to share
    /// one read between schedule resolution and rendering).
    #[allow(dead_code)]
    Io(String),
    /// The source contains zero supported, renderable `G0`/`G1` moves
    /// anywhere in the file (only unsupported constructs, or no motion at
    /// all). A caller must fail the whole request, not report a successful
    /// empty/partial bundle.
    NoRenderableMoves,
    /// `filled_areas` was requested without an explicit
    /// `gcode_line_width_mm`. Bead width must never be derived from `E`.
    MissingLineWidth,
    /// A `silhouette` render needed a bead width for an extruding move it
    /// could not derive from flow, and no explicit `gcode_line_width_mm`
    /// fallback was supplied. `detail` names the missing datum (an absent
    /// `; filament_diameter = ...` config comment, or an `M200` volumetric-
    /// extrusion command that makes `E` a volume rather than a length) and
    /// the remedy. Never guesses a width.
    SilhouetteWidthUnderivable { detail: String },
}

impl fmt::Debug for GcodeRenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GcodeRenderError::NoPrintableArea => write!(
                f,
                "GcodeRenderError::NoPrintableArea(frame: \"plate\" needs a `printable_area` \
                 config comment; this gcode carries none)"
            ),
            GcodeRenderError::Io(msg) => write!(f, "GcodeRenderError::Io({msg})"),
            GcodeRenderError::NoRenderableMoves => write!(
                f,
                "GcodeRenderError::NoRenderableMoves: the G-code source contains no \
                 supported G0/G1 X/Y/Z/E/F renderable moves"
            ),
            GcodeRenderError::MissingLineWidth => write!(
                f,
                "GcodeRenderError::MissingLineWidth: filled_areas requires an explicit \
                 gcode_line_width_mm (line width); it must never be derived from E values"
            ),
            GcodeRenderError::SilhouetteWidthUnderivable { detail } => {
                write!(f, "GcodeRenderError::SilhouetteWidthUnderivable({detail})")
            }
        }
    }
}

impl fmt::Display for GcodeRenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl std::error::Error for GcodeRenderError {}

/// One rasterized image produced by [`render_gcode_visual_debug`].
#[derive(Debug)]
pub struct RenderedImage {
    pub layer_index: i64,
    /// The parsed `;Z:` marker value for this layer, mm. `None` if the
    /// layer never saw a `;Z:` comment.
    pub layer_z: Option<f64>,
    pub visualization: GcodeVisualization,
    /// For an `Overlay` visualization: the structured events this image's
    /// glyphs were drawn from, verbatim, for the manifest's
    /// `overlay_events` mirror. Empty for geometry visualizations.
    pub overlay_events: Vec<OverlayEvent>,
    pub png_bytes: Vec<u8>,
    /// Not yet read by any caller until packet 160 Step 3 wires this module
    /// into `visual_debug.rs`'s dispatch; retained for the eventual
    /// `ImageEntry` width/height fields.
    #[allow(dead_code)]
    pub width: u32,
    #[allow(dead_code)]
    pub height: u32,
}

/// The full result of parsing + rendering a standalone final-G-code source.
#[derive(Debug)]
pub struct GcodeVisualDebugOutput {
    pub parser_version: String,
    /// Bundle-wide warnings in stable source order: one per unsupported
    /// construct (naming its source line), followed by the single
    /// unclassified-extrusion summary warning if any occurred.
    pub warnings: Vec<String>,
    /// Rendered images in stable order: ascending layer index (source
    /// order), then requested-visualization order within a layer.
    pub images: Vec<RenderedImage>,
    /// The single model-wide (whole-file) mm viewport every image in this
    /// output was projected through, margin included.
    ///
    /// Returned so `crate::visual_debug` can record it on each manifest entry:
    /// the agent-facing contract is "read the viewport from `manifest.json`",
    /// and G-code entries used to hard-code `world_bounds_mm: None`, leaving
    /// that promise unmet on this path.
    pub world_bounds_mm: ViewportBoundsMm,
}

/// Parse `gcode_text` and rasterize one PNG per (selected layer, requested
/// visualization) pair into `canvas_width` x `canvas_height` pixels.
///
/// `layer_indices` are already-resolved layer indices (a caller resolving a
/// `LayerSelector::All`-style selector must expand it against
/// [`parse_gcode`]'s output first). `canvas_width`/`canvas_height` are the
/// caller-computed pixel viewport (per packet design: viewport pixel
/// dimensions come from `resolution_scale` and are not this module's
/// concern) — this module only computes the model-wide XY bounding box (in
/// mm) used to project geometry into that shared canvas consistently across
/// every emitted image.
#[allow(dead_code)] // convenience wrapper; exercised by this module's tests
pub fn render_gcode_visual_debug(
    gcode_text: &str,
    layer_indices: &[i64],
    visualizations: &[GcodeVisualization],
    canvas_width: u32,
    canvas_height: u32,
    gcode_line_width_mm: Option<f64>,
    frame: GcodeFrame,
) -> Result<GcodeVisualDebugOutput, GcodeRenderError> {
    render_gcode_visual_debug_styled(
        gcode_text,
        layer_indices,
        visualizations,
        canvas_width,
        canvas_height,
        gcode_line_width_mm,
        frame,
        ColorBy::Role,
    )
}

/// [`render_gcode_visual_debug`] plus the schema-1.1.0 `color_by` selection.
/// `ColorBy::Tool` colors extrusion by the tracked active tool (`T<n>`,
/// tool 0 until the first change) via the fixed shared tool palette — a
/// standalone `.gcode` resolves no config, so `tool_color_source:
/// "filament"` has nothing to read on this path and callers resolve it to
/// the palette.
#[allow(clippy::too_many_arguments)]
pub fn render_gcode_visual_debug_styled(
    gcode_text: &str,
    layer_indices: &[i64],
    visualizations: &[GcodeVisualization],
    canvas_width: u32,
    canvas_height: u32,
    gcode_line_width_mm: Option<f64>,
    frame: GcodeFrame,
    color_by: ColorBy,
) -> Result<GcodeVisualDebugOutput, GcodeRenderError> {
    if visualizations.contains(&GcodeVisualization::FilledAreas) && gcode_line_width_mm.is_none() {
        return Err(GcodeRenderError::MissingLineWidth);
    }

    let parsed = parse_gcode(gcode_text);
    if !parsed.has_renderable_moves {
        return Err(GcodeRenderError::NoRenderableMoves);
    }

    let world_bounds = match frame {
        GcodeFrame::Model => viewport_bounds(parsed.bounds_mm.unwrap_or((0.0, 0.0, 1.0, 1.0))),
        // Frame the bed exactly — never widened to the geometry, or "frame to
        // the plate" would stop meaning the plate as soon as anything sat near
        // an edge.
        GcodeFrame::Plate => viewport_bounds(
            parsed
                .printable_area_mm
                .ok_or(GcodeRenderError::NoPrintableArea)?,
        ),
    };
    let projector = Projector::new(world_bounds, canvas_width, canvas_height);
    let selected: BTreeSet<i64> = layer_indices.iter().copied().collect();

    let mut images = Vec::new();
    for layer in &parsed.layers {
        if !selected.contains(&layer.layer_index) {
            continue;
        }
        for viz in visualizations {
            let mut overlay_events = Vec::new();
            let png_bytes = match viz {
                GcodeVisualization::FilamentLines => {
                    render_filament_lines(layer, &projector, canvas_width, canvas_height, color_by)
                }
                GcodeVisualization::FilledAreas => render_filled_areas(
                    layer,
                    &projector,
                    canvas_width,
                    canvas_height,
                    gcode_line_width_mm.expect("checked above"),
                    color_by,
                ),
                GcodeVisualization::Overlay(kind) => {
                    overlay_events = layer_overlay_events(layer, *kind);
                    render_overlay(
                        layer,
                        &projector,
                        canvas_width,
                        canvas_height,
                        &overlay_events,
                    )
                }
            };
            images.push(RenderedImage {
                layer_index: layer.layer_index,
                layer_z: layer.layer_z,
                visualization: *viz,
                overlay_events,
                png_bytes,
                width: canvas_width,
                height: canvas_height,
            });
        }
    }

    Ok(GcodeVisualDebugOutput {
        parser_version: GCODE_PARSER_VERSION.to_string(),
        warnings: parsed.warnings,
        images,
        world_bounds_mm: world_bounds,
    })
}

/// Convenience wrapper reading `path` from disk before calling
/// [`render_gcode_visual_debug`].
#[allow(dead_code)] // convenience wrapper; exercised by this module's tests
pub fn render_gcode_visual_debug_from_path(
    path: &Path,
    layer_indices: &[i64],
    visualizations: &[GcodeVisualization],
    canvas_width: u32,
    canvas_height: u32,
    gcode_line_width_mm: Option<f64>,
    frame: GcodeFrame,
) -> Result<GcodeVisualDebugOutput, GcodeRenderError> {
    let text = fs::read_to_string(path)
        .map_err(|e| GcodeRenderError::Io(format!("{}: {e}", path.display())))?;
    render_gcode_visual_debug(
        &text,
        layer_indices,
        visualizations,
        canvas_width,
        canvas_height,
        gcode_line_width_mm,
        frame,
    )
}

// ─────────────────────────────── parsing ──────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointMm {
    pub x: f64,
    pub y: f64,
}

/// One motion segment (travel or extrusion) in source order.
#[derive(Debug, Clone)]
pub struct Segment {
    pub from: PointMm,
    pub to: PointMm,
    pub is_extrusion: bool,
    /// `"unclassified"` when no `;TYPE:` marker was active yet. Empty for
    /// travel segments (role is meaningless for non-extrusion motion).
    pub role: String,
    /// The active tool when this segment was emitted (`T<n>` tracking;
    /// tool 0 until the first tool change).
    pub tool: u32,
    /// The signed E-axis delta this move commanded, mm of filament. `0.0`
    /// for a move that carries no `E` token; negative for a retraction.
    ///
    /// Retained (rather than being collapsed into `is_extrusion`) so the
    /// `silhouette` visualization can derive a per-segment bead width from
    /// flow — see [`silhouette_segment_width_mm`].
    pub e_delta_mm: f64,
    /// 1-indexed source line the move was parsed from.
    ///
    /// Needed because `M200` (volumetric extrusion) poisons flow-derived
    /// widths **from its line onward**, not for the whole file: a layer
    /// selection that touches only pre-`M200` moves must still render. That
    /// question can only be answered per segment, so each one carries its
    /// own line.
    pub source_line: usize,
}

#[derive(Debug, Clone, Default)]
pub struct ParsedLayer {
    pub layer_index: i64,
    pub layer_z: Option<f64>,
    pub segments: Vec<Segment>,
    /// Point events parsed for this layer in source order: retractions/
    /// unretractions (E-only moves and firmware `G10`/`G11`), z-hops (Z-only
    /// lifts above the layer's base Z), and tool changes. Travel polylines
    /// are NOT stored here — they are derived from `segments` by
    /// [`layer_overlay_events`] so the polyline and the rendered travel
    /// share one source.
    pub events: Vec<OverlayEvent>,
}

/// Structured parse of a full G-code source. Always "succeeds" structurally
/// — unsupported constructs become warnings, not parse failures; callers
/// decide whether `has_renderable_moves == false` is fatal.
#[derive(Debug, Clone)]
pub struct ParsedGcode {
    pub layers: Vec<ParsedLayer>,
    pub warnings: Vec<String>,
    /// Model-wide XY bounding box in mm across every parsed move endpoint
    /// (travel and extrusion), or `None` if the file has no motion at all.
    pub bounds_mm: Option<(f64, f64, f64, f64)>,
    /// True iff at least one supported `G0`/`G1` move with an actual XY
    /// displacement was parsed anywhere in the file (AC-N2: a file with only
    /// unsupported constructs, e.g. G2/G3 arcs, has none).
    pub has_renderable_moves: bool,
    /// The bed's XY bounding box in mm, from the slicer config block's
    /// `printable_area` comment, or `None` if the file carries no usable one.
    ///
    /// This is the only bed definition a standalone `.gcode` has — the
    /// standalone path resolves no printer profile — so it is what
    /// `frame: "plate"` frames to on this source.
    pub printable_area_mm: Option<(f64, f64, f64, f64)>,
    /// Filament diameters in mm from the config block's
    /// `; filament_diameter = …` comment, in declaration (extruder) order.
    /// Empty when the file carries no such comment — the silhouette path
    /// then has no way to convert an E delta into a volume and must say so
    /// rather than assume 1.75.
    pub filament_diameters_mm: Vec<f64>,
    /// 1-indexed source line of the first `M200` (volumetric extrusion), or
    /// `None`. `M200` makes E a *volume*, not a length, which invalidates
    /// every flow-derived width — it is recorded as a poison marker, not a
    /// warning about an unsupported construct.
    pub volumetric_extrusion_line: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExtrusionMode {
    Absolute,
    Relative,
}

/// The text shared by every unsupported-construct warning — the per-line
/// first occurrence AND the collapsed summary — so one grep finds them all.
const UNSUPPORTED_CONSTRUCT_TEXT: &str =
    "unsupported G-code construct outside the documented G0/G1 X/Y/Z/E/F subset";

/// One distinct unsupported construct (`M73`, `G2`, …) and its tally.
struct UnsupportedTally {
    construct: String,
    first_line: usize,
    count: usize,
}

/// Record one unsupported-construct occurrence.
///
/// The FIRST occurrence of a construct emits the full, line-numbered warning
/// verbatim (text unchanged); every later occurrence of the SAME construct
/// is suppressed and only counted, with [`finish_unsupported_tallies`]
/// appending one summary that preserves the true total.
///
/// Without this collapse the warning channel is pure noise on real files: a
/// measured 13,662-line source produced 234 warnings, all 234 of them `M73`
/// progress commands, which would bury any genuine W3 skip-with-warning —
/// this feature's own fail-closed signal — in the same list.
fn record_unsupported_construct(
    warnings: &mut Vec<String>,
    tallies: &mut Vec<UnsupportedTally>,
    construct: &str,
    line_no: usize,
    code_part: &str,
) {
    if let Some(tally) = tallies.iter_mut().find(|t| t.construct == construct) {
        tally.count += 1;
        return;
    }
    tallies.push(UnsupportedTally {
        construct: construct.to_string(),
        first_line: line_no,
        count: 1,
    });
    warnings.push(format!(
        "line {line_no}: {UNSUPPORTED_CONSTRUCT_TEXT}: {code_part}"
    ));
}

/// Append one summary per construct seen more than once, in parse order of
/// first occurrence (the `Vec` push order — no `HashMap` on this path).
fn finish_unsupported_tallies(warnings: &mut Vec<String>, tallies: &[UnsupportedTally]) {
    for tally in tallies {
        if tally.count > 1 {
            warnings.push(format!(
                "{UNSUPPORTED_CONSTRUCT_TEXT}: {} (first at line {}; {} occurrences total, \
                 {} suppressed)",
                tally.construct,
                tally.first_line,
                tally.count,
                tally.count - 1
            ));
        }
    }
}

/// Parse the documented Pinch 'n Print final-G-code subset. Public so
/// callers (and this module's own tests) can inspect structured layer/
/// warning data directly without going through PNG rendering.
pub fn parse_gcode(text: &str) -> ParsedGcode {
    let mut layers: Vec<ParsedLayer> = Vec::new();
    let mut layer_map: BTreeMap<i64, usize> = BTreeMap::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut unclassified_lines: Vec<usize> = Vec::new();
    let mut unsupported_tallies: Vec<UnsupportedTally> = Vec::new();

    let mut current_layer_index: i64 = -1;
    let mut current_role: Option<String> = None;
    let mut mode = ExtrusionMode::Absolute;
    // The toolhead's XY position, per axis, `None` until the G-code actually
    // states it.
    //
    // This must NOT default to the origin. A file's first move is typically
    // `G1 X80 Y90` after a homing/start macro this parser does not model; if
    // the toolhead were assumed to start at (0, 0), that first move would be
    // treated as a real travel *from the bed origin*, dragging (0, 0) into the
    // model-wide bounding box. Every render would then be framed from the bed
    // origin to the model's far corner — the model shrunk into a corner of
    // what looks like a full-plate view — even though no such move exists in
    // the file. Fabricating a start position is exactly the "never approximate
    // what we don't fully understand" rule this module states below.
    let mut pos_x: Option<f64> = None;
    let mut pos_y: Option<f64> = None;
    let mut last_e: f64 = 0.0;
    let mut has_renderable_moves = false;
    // Overlay-event state (schema 1.1.0): the active tool (`T<n>`, 0 until
    // the first change) and the layer's base Z, against which a Z-only lift
    // is classified as a z-hop.
    let mut current_tool: u32 = 0;
    let mut layer_base_z: Option<f64> = None;

    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut printable_area_mm: Option<(f64, f64, f64, f64)> = None;
    let mut filament_diameters_mm: Vec<f64> = Vec::new();
    let mut volumetric_extrusion_line: Option<usize> = None;

    for (idx, raw_line) in text.lines().enumerate() {
        let line_no = idx + 1;
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }

        if line.starts_with(";LAYER_CHANGE") {
            current_layer_index += 1;
            ensure_layer(&mut layers, &mut layer_map, current_layer_index);
            layer_base_z = None;
            continue;
        }
        if let Some(rest) = line.strip_prefix(";Z:") {
            if let Ok(z) = rest.trim().parse::<f64>() {
                let li = ensure_layer(&mut layers, &mut layer_map, current_layer_index);
                layers[li].layer_z = Some(z);
                layer_base_z = Some(z);
            }
            continue;
        }
        if let Some(rest) = line.strip_prefix(";TYPE:") {
            current_role = Some(rest.trim().to_string());
            continue;
        }
        if let Some(rest) = line.strip_prefix(';') {
            // The slicer's own config block, emitted as `; key = value`
            // comments (OrcaSlicer writes it as a trailer, after all motion).
            // `printable_area` is the bed polygon, and it is the only bed
            // definition a standalone `.gcode` carries — there is no printer
            // profile to consult on this path.
            if let Some(area) = parse_printable_area_comment(rest) {
                printable_area_mm = Some(area);
            }
            // `filament_diameter` is what turns an E *length* delta into an
            // extruded *volume*, and so is the only source-carried datum
            // that makes a flow-derived silhouette width possible.
            if let Some(diameters) = parse_filament_diameter_comment(rest) {
                filament_diameters_mm = diameters;
            }
            continue;
        }

        // Strip an inline trailing "; comment" suffix from a code line.
        let code_part = match line.find(';') {
            Some(p) => line[..p].trim(),
            None => line,
        };
        if code_part.is_empty() {
            continue;
        }

        let mut tokens = code_part.split_whitespace();
        let Some(cmd) = tokens.next() else {
            continue;
        };

        match cmd {
            "M82" => mode = ExtrusionMode::Absolute,
            "M83" => mode = ExtrusionMode::Relative,
            // Volumetric extrusion: E states a volume rather than a length,
            // so every flow-derived width downstream is meaningless. Record
            // the first occurrence as a poison marker — this construct IS
            // understood (that is precisely why we can rule the derivation
            // out), so it must not raise an unsupported-construct warning.
            "M200" => {
                if volumetric_extrusion_line.is_none() {
                    volumetric_extrusion_line = Some(line_no);
                }
            }
            // `G92 E<val>` re-defines the current E-axis position without
            // moving the extruder. The parser's carried `last_e` MUST follow
            // it: a mid-print `G92 E0` after E has climbed to, say, 5.0
            // would otherwise make the next absolute-mode extruding move
            // compute a large negative delta and be misclassified as travel.
            // X/Y/Z offsets are a different (unmodelled) thing and stay
            // unsupported.
            "G92" => {
                let mut saw_e = false;
                let mut unsupported = false;
                for tok in tokens {
                    if tok.is_empty() {
                        continue;
                    }
                    let (letter, rest) = tok.split_at(1);
                    match (letter, rest.parse::<f64>()) {
                        ("E", Ok(value)) => {
                            last_e = value;
                            saw_e = true;
                        }
                        _ => unsupported = true,
                    }
                }
                if unsupported || !saw_e {
                    record_unsupported_construct(
                        &mut warnings,
                        &mut unsupported_tallies,
                        cmd,
                        line_no,
                        code_part,
                    );
                }
            }
            "G0" | "G1" => {
                let mut new_x = pos_x;
                let mut new_y = pos_y;
                let mut has_xy = false;
                let mut new_z: Option<f64> = None;
                let mut has_e = false;
                let mut e_delta = 0.0_f64;
                let mut unsupported = false;

                for tok in tokens {
                    if tok.is_empty() {
                        continue;
                    }
                    let (letter, rest) = tok.split_at(1);
                    let Ok(value) = rest.parse::<f64>() else {
                        unsupported = true;
                        continue;
                    };
                    match letter {
                        "X" => {
                            new_x = Some(value);
                            has_xy = true;
                        }
                        "Y" => {
                            new_y = Some(value);
                            has_xy = true;
                        }
                        // A Z value doesn't affect XY segments/viewport, but
                        // is tracked to classify Z-only lifts as z-hops.
                        "Z" => new_z = Some(value),
                        "F" => {} // feed rate; irrelevant to geometry.
                        "E" => {
                            has_e = true;
                            e_delta = match mode {
                                ExtrusionMode::Absolute => value - last_e,
                                ExtrusionMode::Relative => value,
                            };
                            last_e = match mode {
                                ExtrusionMode::Absolute => value,
                                ExtrusionMode::Relative => last_e + value,
                            };
                        }
                        _ => unsupported = true,
                    }
                }

                if unsupported {
                    record_unsupported_construct(
                        &mut warnings,
                        &mut unsupported_tallies,
                        cmd,
                        line_no,
                        code_part,
                    );
                    // Any recognized X/Y on this line are still real,
                    // physically-known state changes (a real printer would
                    // still apply them) even though the move as a whole is
                    // never rendered — so `pos` must still advance to keep
                    // the NEXT supported move's delta correct. `last_e` is
                    // already updated unconditionally above, for the same
                    // reason. Only the render (segment push + bounds
                    // update) is skipped for this partially-unsupported
                    // move — never approximate what we don't fully
                    // understand.
                    pos_x = new_x;
                    pos_y = new_y;
                    continue;
                }

                let from = match (pos_x, pos_y) {
                    (Some(x), Some(y)) => Some(PointMm { x, y }),
                    // The toolhead's position was never stated before this
                    // move — there is no known point to draw *from*.
                    _ => None,
                };
                let to = match (new_x, new_y) {
                    (Some(x), Some(y)) => Some(PointMm { x, y }),
                    // Still only one axis known (e.g. a lone `G1 X80` opener):
                    // no complete XY point exists yet.
                    _ => None,
                };
                pos_x = new_x;
                pos_y = new_y;
                let is_extrusion = has_e && e_delta > 0.0;

                // Overlay events (schema 1.1.0). Positions require a known
                // toolhead XY — an event before the first stated position is
                // skipped, never fabricated at an assumed origin.
                if !has_xy {
                    if let (Some(x), Some(y)) = (pos_x, pos_y) {
                        let (x, y) = (x as f32, y as f32);
                        let li = ensure_layer(&mut layers, &mut layer_map, current_layer_index);
                        if has_e && e_delta < 0.0 {
                            layers[li].events.push(OverlayEvent::Retraction {
                                x,
                                y,
                                length_mm: (-e_delta) as f32,
                            });
                        } else if has_e && e_delta > 0.0 {
                            layers[li].events.push(OverlayEvent::Unretraction {
                                x,
                                y,
                                length_mm: e_delta as f32,
                            });
                        } else if let (Some(z), false) = (new_z, has_e) {
                            // A Z-only move: a lift above the layer's base Z
                            // is a z-hop; the first Z statement of a layer
                            // (no base yet) establishes the base instead.
                            match layer_base_z {
                                Some(base) if z > base + 1e-9 => {
                                    layers[li].events.push(OverlayEvent::ZHop {
                                        x,
                                        y,
                                        height_mm: (z - base) as f32,
                                    });
                                }
                                Some(_) => {}
                                None => layer_base_z = Some(z),
                            }
                        }
                    } else if let (Some(z), false, false) = (new_z, has_e, has_xy) {
                        // Even with no XY yet, a Z statement can establish
                        // the layer base so a later lift classifies.
                        if layer_base_z.is_none() {
                            layer_base_z = Some(z);
                        }
                    }
                }

                // A destination the file actually stated is real geometry and
                // always bounds the viewport, even when we can't draw the
                // travel that reached it.
                if let Some(to) = to {
                    min_x = min_x.min(to.x);
                    min_y = min_y.min(to.y);
                    max_x = max_x.max(to.x);
                    max_y = max_y.max(to.y);
                }

                // A segment needs two known endpoints. When `from` is unknown
                // this is the file's opening move: its destination counts
                // (above), but inventing a line to it from a guessed origin
                // would be fabricated geometry.
                let (Some(from), Some(to)) = (from, to) else {
                    continue;
                };

                if from.x != to.x || from.y != to.y {
                    min_x = min_x.min(from.x);
                    min_y = min_y.min(from.y);
                    max_x = max_x.max(from.x);
                    max_y = max_y.max(from.y);
                    has_renderable_moves = true;

                    let role = if is_extrusion {
                        match &current_role {
                            Some(r) => r.clone(),
                            None => {
                                unclassified_lines.push(line_no);
                                UNCLASSIFIED_ROLE.to_string()
                            }
                        }
                    } else {
                        String::new()
                    };

                    let li = ensure_layer(&mut layers, &mut layer_map, current_layer_index);
                    layers[li].segments.push(Segment {
                        from,
                        to,
                        is_extrusion,
                        role,
                        tool: current_tool,
                        e_delta_mm: if has_e { e_delta } else { 0.0 },
                        source_line: line_no,
                    });
                }
            }
            // Firmware retract/unretract: bare opcodes with no length on the
            // line (the length lives in printer memory) — recorded with
            // length 0.0, never guessed.
            "G10" => {
                if let (Some(x), Some(y)) = (pos_x, pos_y) {
                    let li = ensure_layer(&mut layers, &mut layer_map, current_layer_index);
                    layers[li].events.push(OverlayEvent::Retraction {
                        x: x as f32,
                        y: y as f32,
                        length_mm: 0.0,
                    });
                }
            }
            "G11" => {
                if let (Some(x), Some(y)) = (pos_x, pos_y) {
                    let li = ensure_layer(&mut layers, &mut layer_map, current_layer_index);
                    layers[li].events.push(OverlayEvent::Unretraction {
                        x: x as f32,
                        y: y as f32,
                        length_mm: 0.0,
                    });
                }
            }
            _ => {
                // `T<n>` tool select: track the active tool and record the
                // change event when the toolhead position is known.
                if let Some(rest) = cmd.strip_prefix('T') {
                    if let Ok(tool) = rest.parse::<u32>() {
                        if tool != current_tool {
                            if let (Some(x), Some(y)) = (pos_x, pos_y) {
                                let li =
                                    ensure_layer(&mut layers, &mut layer_map, current_layer_index);
                                layers[li].events.push(OverlayEvent::ToolChange {
                                    x: x as f32,
                                    y: y as f32,
                                    from_tool: Some(current_tool),
                                    to_tool: tool,
                                });
                            }
                            current_tool = tool;
                        }
                        continue;
                    }
                }
                record_unsupported_construct(
                    &mut warnings,
                    &mut unsupported_tallies,
                    cmd,
                    line_no,
                    code_part,
                );
            }
        }
    }

    finish_unsupported_tallies(&mut warnings, &unsupported_tallies);

    if let Some(&first_line) = unclassified_lines.first() {
        warnings.push(format!(
            "{} unclassified extrusion segment(s) retained (extrusion occurred before \
             any ;TYPE: marker was seen), e.g. source line {first_line}",
            unclassified_lines.len()
        ));
    }

    let bounds_mm = if min_x.is_finite() {
        Some((min_x, min_y, max_x, max_y))
    } else {
        None
    };

    ParsedGcode {
        layers,
        warnings,
        bounds_mm,
        has_renderable_moves,
        printable_area_mm,
        filament_diameters_mm,
        volumetric_extrusion_line,
    }
}

fn ensure_layer(
    layers: &mut Vec<ParsedLayer>,
    layer_map: &mut BTreeMap<i64, usize>,
    layer_index: i64,
) -> usize {
    if let Some(&li) = layer_map.get(&layer_index) {
        return li;
    }
    layers.push(ParsedLayer {
        layer_index,
        layer_z: None,
        segments: Vec::new(),
        events: Vec::new(),
    });
    let li = layers.len() - 1;
    layer_map.insert(layer_index, li);
    li
}

// ─────────────────────────────── projection ───────────────────────────────

/// Parse a `printable_area` config comment's value into an
/// `(min_x, min_y, max_x, max_y)` mm bounding box, or `None` if this comment
/// isn't `printable_area` or its value isn't a usable polygon.
///
/// The emitted form is `; printable_area = 0x0,220x0,220x200,0x200` — points
/// separated by `,`, and each point's X and Y separated by a literal `x`.
///
/// `rest` is the comment body with its leading `;` already stripped. The key
/// is matched exactly, which matters more than it looks: this file also
/// contains `extruder_printable_area` (a different key, usually empty) and a
/// `different_settings_to_system = ...;printable_area;...` line that mentions
/// the name in a value. A substring match would pick up either.
fn parse_printable_area_comment(rest: &str) -> Option<(f64, f64, f64, f64)> {
    let (key, value) = rest.split_once('=')?;
    if key.trim() != "printable_area" {
        return None;
    }

    let (mut min_x, mut min_y) = (f64::MAX, f64::MAX);
    let (mut max_x, mut max_y) = (f64::MIN, f64::MIN);
    let mut points = 0usize;
    for point in value.trim().split(',') {
        let point = point.trim();
        if point.is_empty() {
            continue;
        }
        // `split_once` rather than `split`: a malformed `1x2x3` is rejected
        // rather than silently read as its first two components.
        let (x, y) = point.split_once('x')?;
        let x: f64 = x.trim().parse().ok()?;
        let y: f64 = y.trim().parse().ok()?;
        if !x.is_finite() || !y.is_finite() {
            return None;
        }
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
        points += 1;
    }

    // A bed needs at least a triangle, and must enclose real area.
    if points < 3 || max_x <= min_x || max_y <= min_y {
        return None;
    }
    Some((min_x, min_y, max_x, max_y))
}

/// Parse a `filament_diameter` config comment's value into the per-extruder
/// diameters in mm, or `None` if this comment isn't `filament_diameter` or
/// its value isn't a usable list.
///
/// The emitted form is `; filament_diameter = 1.75,1.75` — one positive
/// finite value per extruder, comma separated.
///
/// `rest` is the comment body with its leading `;` already stripped. As with
/// [`parse_printable_area_comment`], the key is matched EXACTLY: this file
/// also carries neighbouring keys that contain this one as a substring
/// (`filament_diameter` appears inside `different_settings_to_system`-style
/// value lists), and a substring match would pick those up.
///
/// A single unusable entry rejects the whole comment rather than yielding a
/// partial list: an extruder-indexed list with a hole would silently
/// misattribute diameters to the wrong extruder.
fn parse_filament_diameter_comment(rest: &str) -> Option<Vec<f64>> {
    let (key, value) = rest.split_once('=')?;
    if key.trim() != "filament_diameter" {
        return None;
    }

    let mut diameters = Vec::new();
    for entry in value.trim().split(',') {
        let d: f64 = entry.trim().parse().ok()?;
        if !d.is_finite() || d <= 0.0 {
            return None;
        }
        diameters.push(d);
    }
    if diameters.is_empty() {
        return None;
    }
    Some(diameters)
}

/// Derive each layer's `(bottom_z, top_z)` slab in mm from the file's `;Z:`
/// markers, plus one W3 warning per layer whose marker is unusable.
///
/// The `;Z:` marker is the ONLY layer-height information a standalone
/// `.gcode` carries, and it states a layer's *top* Z. A slab is therefore the
/// span from the previously ACCEPTED marker to this layer's marker:
///
/// - The carried marker starts at the bed, `0.0`. The first accepted marker
///   therefore yields `(0.0, z)` — the bottom is always the bed, never a
///   guess extrapolated from a later marker delta, since with one marker in
///   hand there is no delta to extrapolate from.
/// - `z <= prev` (a duplicate or non-monotonic marker), a non-finite marker,
///   or a layer with no marker at all yields NO slab and exactly one W3
///   warning. There is NO first-marker exemption: `prev` is `0.0` for layer
///   one, so a first marker of `0` or a negative Z is rejected like any other
///   non-monotonic marker rather than forming a degenerate or inverted slab.
/// - A skipped layer does NOT advance the carried marker, so the next good
///   layer's slab still starts at the last Z the file actually established.
///
/// Warnings come back in layer order, and the map is a [`BTreeMap`], so both
/// outputs are deterministic across runs.
pub fn gcode_silhouette_slabs(parsed: &ParsedGcode) -> (BTreeMap<i64, (f64, f64)>, Vec<String>) {
    let mut slabs: BTreeMap<i64, (f64, f64)> = BTreeMap::new();
    let mut warnings: Vec<String> = Vec::new();
    // The bed. The first layer's marker is measured against it exactly like
    // every later marker is measured against its predecessor — there is no
    // first-marker special case, so one comparison governs all layers.
    let mut prev_z: f64 = 0.0;

    for layer in &parsed.layers {
        let li = layer.layer_index;
        let Some(z) = layer.layer_z else {
            warnings.push(format!(
                "W3: layer {li} has no ;Z: marker; no silhouette slab was derived for it"
            ));
            continue;
        };
        // `str::parse::<f64>` accepts "NaN"/"inf"/"-inf", so a `;Z:` marker
        // CAN carry a non-finite value; reject it before the ordering test,
        // where NaN would compare false against everything.
        if !z.is_finite() {
            warnings.push(format!(
                "W3: layer {li} has a non-finite ;Z: marker (Z {z}); no silhouette slab \
                 was derived for it"
            ));
            continue;
        }
        if z <= prev_z {
            warnings.push(format!(
                "W3: layer {li} has a non-increasing ;Z: marker (Z {z} <= previous \
                 accepted Z {prev_z}); no silhouette slab was derived for it"
            ));
            continue;
        }
        slabs.insert(li, (prev_z, z));
        prev_z = z;
    }

    (slabs, warnings)
}

/// The extrusion width in mm implied by one move's flow, in closed form.
///
/// Inverts the standard authoring relation `Δe = L × w × h / A_filament`
/// (extruded filament volume equals the deposited bead's volume) for `w`:
///
/// ```text
/// w = Δe × π·(d/2)² / (L × h)
/// ```
///
/// `pub` so the closed form can be pinned directly by a test rather than
/// only observed through a rendered raster.
///
/// Returns `0.0` for a degenerate input (`L` or `h` non-positive, or a
/// non-finite result) rather than an infinity or NaN that would poison a
/// downstream polygon offset.
pub fn silhouette_segment_width_mm(
    e_delta_mm: f64,
    length_mm: f64,
    slab_height_mm: f64,
    filament_diameter_mm: f64,
) -> f64 {
    if length_mm <= 0.0 || slab_height_mm <= 0.0 || filament_diameter_mm <= 0.0 {
        return 0.0;
    }
    let filament_area_mm2 = std::f64::consts::PI * (filament_diameter_mm / 2.0).powi(2);
    let width = e_delta_mm * filament_area_mm2 / (length_mm * slab_height_mm);
    if width.is_finite() {
        width
    } else {
        0.0
    }
}

// ───────────────────────── silhouette composite render ────────────────────

/// The result of a composite silhouette render over a standalone
/// final-G-code source (schema 1.2.0).
#[derive(Debug)]
pub struct GcodeSilhouetteOutput {
    pub parser_version: String,
    /// Parse warnings in source order, followed by the W3 slab-derivation
    /// warnings in layer order.
    pub warnings: Vec<String>,
    pub png_bytes: Vec<u8>,
    pub width: u32,
    pub height: u32,
    /// The whole-file (selection-independent) mm viewport the raster was
    /// projected through, margin included.
    pub world_bounds_mm: ViewportBoundsMm,
    /// The selected layers that actually had a derived slab, ascending —
    /// the selection minus everything W3 skipped.
    pub layers_rendered: Vec<i64>,
}

/// The paint class a silhouette rectangle belongs to.
///
/// The derived `Ord` IS the paint order (D15): `Unclassified` sorts before
/// every `Role`, so role-less extrusion is painted first and every role
/// class occludes it; `Role` compares lexicographically and `Tool` by
/// ascending index. A single render only ever produces one of the
/// `Role`/`Tool` families, since the family follows the requested
/// [`ColorBy`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum SilhouetteClassKey {
    Unclassified,
    Role(String),
    Tool(u32),
}

impl SilhouetteClassKey {
    fn of(seg: &Segment, color_by: ColorBy) -> Self {
        match color_by {
            ColorBy::Role if seg.role == UNCLASSIFIED_ROLE => Self::Unclassified,
            ColorBy::Role => Self::Role(seg.role.clone()),
            ColorBy::Tool => Self::Tool(seg.tool),
        }
    }

    fn color(&self) -> [u8; 3] {
        match self {
            Self::Unclassified => style::GCODE_UNCLASSIFIED_COLOR,
            Self::Role(role) => style::gcode_role_color(role, UNCLASSIFIED_ROLE),
            // A standalone `.gcode` resolves no config, so tool colors are
            // always the fixed shared palette.
            Self::Tool(tool) => ToolColors::default().color(*tool),
        }
    }
}

/// The bead width, in mm, this silhouette render must use for `seg`.
///
/// Derived from flow whenever the source carries the data to do so, and
/// otherwise taken from the caller's explicit `fallback_width_mm`. The
/// fallback is never preferred over a derivable width — only used when the
/// derivation is impossible:
///
/// * the file carries no `; filament_diameter = …` comment, so an `E` length
///   cannot become a volume; or
/// * the move sits at or after the file's first `M200`, which redefines `E`
///   as a volume and invalidates the relation this derivation inverts.
///
/// With no fallback either, this fails closed naming the missing datum — it
/// never guesses a width.
/// Why a rendered segment's width had to come from the caller's fallback
/// rather than from the file's own flow data. Ordered so the bundle-level
/// summary warnings (see [`render_gcode_silhouette`]) have a deterministic
/// emission order independent of parse order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum FallbackCause {
    /// The file carries no `; filament_diameter = ...` config comment.
    NoFilamentDiameter,
    /// An `M200` (volumetric extrusion) at the carried source line makes `E`
    /// a volume, so no flow-derived width is meaningful from there onward.
    VolumetricExtrusion(usize),
}

fn silhouette_width_for_segment(
    seg: &Segment,
    parsed: &ParsedGcode,
    slab_height_mm: f64,
    fallback_width_mm: Option<f64>,
) -> Result<(f64, Option<FallbackCause>), GcodeRenderError> {
    let poisoned =
        matches!(parsed.volumetric_extrusion_line, Some(line) if seg.source_line >= line);
    let derivable = !parsed.filament_diameters_mm.is_empty() && !poisoned;

    if derivable {
        // A tool index past the declared list clamps to the last entry
        // rather than falling back: the file DID state a diameter, and the
        // last one is the nearest thing it says about this extruder.
        let idx = (seg.tool as usize).min(parsed.filament_diameters_mm.len() - 1);
        let diameter = parsed.filament_diameters_mm[idx];
        let dx = seg.to.x - seg.from.x;
        let dy = seg.to.y - seg.from.y;
        let length_mm = (dx * dx + dy * dy).sqrt();
        return Ok((
            silhouette_segment_width_mm(seg.e_delta_mm, length_mm, slab_height_mm, diameter),
            None,
        ));
    }

    if let Some(w) = fallback_width_mm {
        // Which of the two modelled causes applies is reported to the caller
        // so ONE bundle-level warning can name it — a silent fallback makes a
        // silhouette whose widths came entirely from the request look exactly
        // as authoritative as a flow-derived one.
        let cause = match parsed.volumetric_extrusion_line {
            Some(line) if poisoned => FallbackCause::VolumetricExtrusion(line),
            _ => FallbackCause::NoFilamentDiameter,
        };
        return Ok((w, Some(cause)));
    }

    let detail = if poisoned {
        let line = parsed
            .volumetric_extrusion_line
            .expect("poisoned implies a recorded M200 line");
        format!(
            "silhouette bead width is underivable for the extruding move on source line {}: \
             `M200` (volumetric extrusion) on source line {line} redefines E as a volume \
             rather than a length, so no flow-derived width is meaningful from that line \
             onward; supply an explicit `gcode_line_width_mm` to render a silhouette from \
             this source",
            seg.source_line
        )
    } else {
        format!(
            "silhouette bead width is underivable for the extruding move on source line {}: \
             this G-code carries no `; filament_diameter = ...` config comment, so an E \
             length cannot be converted to an extruded volume; supply an explicit \
             `gcode_line_width_mm` to render a silhouette from this source",
            seg.source_line
        )
    };
    Err(GcodeRenderError::SilhouetteWidthUnderivable { detail })
}

/// Render one composite silhouette PNG (D12) for a standalone final-G-code
/// source: every selected layer's extrusion, projected onto the view plane
/// and stacked in its own derived Z slab.
///
/// Framing reads whole-file data only — the horizontal extent comes from the
/// parsed model-wide bounds and the vertical extent from every accepted
/// `;Z:` marker — so a layer-subset request and an all-layers request are
/// framed identically (AC-9).
///
/// Determinism: the only iteration sources are parse order and `BTreeMap`
/// key order; no `HashMap` appears anywhere on this path.
pub fn render_gcode_silhouette(
    gcode_text: &str,
    layer_indices: &[i64],
    view: slicer_runtime::SilhouetteView,
    canvas_width: u32,
    canvas_height: u32,
    fallback_width_mm: Option<f64>,
    color_by: ColorBy,
) -> Result<GcodeSilhouetteOutput, GcodeRenderError> {
    let parsed = parse_gcode(gcode_text);
    if !parsed.has_renderable_moves {
        return Err(GcodeRenderError::NoRenderableMoves);
    }

    let (slabs, slab_warnings) = gcode_silhouette_slabs(&parsed);

    // ── framing: whole-file only, never the selection ──────────────────────
    let (min_x, min_y, max_x, max_y) = parsed.bounds_mm.unwrap_or((0.0, 0.0, 1.0, 1.0));
    let (h_min, h_max) = match view {
        slicer_runtime::SilhouetteView::Front => (min_x, max_x),
        slicer_runtime::SilhouetteView::Side => (min_y, max_y),
    };
    let v_min = slabs
        .values()
        .next()
        .map_or(0.0, |(bottom, _)| *bottom)
        .min(0.0);
    let v_max = slabs
        .values()
        .map(|(_, top)| *top)
        .fold(v_min, |acc, top| if top > acc { top } else { acc });
    let world_bounds = viewport_bounds((h_min, v_min, h_max, v_max));
    let projector = Projector::new(world_bounds, canvas_width, canvas_height);

    // ── per (layer, class) horizontal intervals, in parse order ────────────
    let selected: BTreeSet<i64> = layer_indices.iter().copied().collect();
    let mut layers_rendered: Vec<i64> = Vec::new();
    // Keyed by (layer, class): `BTreeMap` order IS the emission order —
    // ascending layer, then the class paint order encoded by
    // `SilhouetteClassKey`'s `Ord`.
    let mut buckets: BTreeMap<(i64, SilhouetteClassKey), Vec<(f32, f32)>> = BTreeMap::new();
    // Fallback-width provenance (finding 1): how many rendered extruding
    // moves took the caller's `gcode_line_width_mm` instead of a flow-derived
    // width, tallied per cause. `BTreeMap` keeps the emission order stable.
    let mut extruding_rendered: usize = 0;
    let mut fallback_counts: BTreeMap<FallbackCause, usize> = BTreeMap::new();

    for layer in &parsed.layers {
        if !selected.contains(&layer.layer_index) {
            continue;
        }
        let Some(&(z_bottom, z_top)) = slabs.get(&layer.layer_index) else {
            // W3 already reported this layer; it contributes nothing.
            continue;
        };
        layers_rendered.push(layer.layer_index);
        let slab_height = z_top - z_bottom;

        for seg in &layer.segments {
            if !seg.is_extrusion {
                continue;
            }
            // Evaluated lazily, per rendered extruding segment, in parse
            // order: a selection that never touches a poisoned move must
            // still succeed.
            let (w, fallback_cause) =
                silhouette_width_for_segment(seg, &parsed, slab_height, fallback_width_mm)?;
            extruding_rendered += 1;
            if let Some(cause) = fallback_cause {
                *fallback_counts.entry(cause).or_insert(0) += 1;
            }
            let (h0, h1) = match view {
                slicer_runtime::SilhouetteView::Front => (seg.from.x, seg.to.x),
                slicer_runtime::SilhouetteView::Side => (seg.from.y, seg.to.y),
            };
            // The move's own extent, inflated by half its width at EACH end
            // — the bead is centered on the path.
            let start = (h0.min(h1) - w / 2.0) as f32;
            let end = (h0.max(h1) + w / 2.0) as f32;
            buckets
                .entry((layer.layer_index, SilhouetteClassKey::of(seg, color_by)))
                .or_default()
                .push((start, end));
        }
    }

    // ── rasterize ──────────────────────────────────────────────────────────
    let mut buf = vec![255u8; canvas_width as usize * canvas_height as usize * 3];
    for ((layer_index, class), intervals) in &buckets {
        let (z_bottom, z_top) = slabs[layer_index];
        let color = class.color();
        // The one shared union implementation — this module owns no copy.
        for (start, end) in slicer_runtime::union_silhouette_intervals(intervals) {
            let p0 = projector.project(f64::from(start), z_bottom);
            let p1 = projector.project(f64::from(end), z_top);
            fill_rect(&mut buf, canvas_width, canvas_height, p0, p1, color);
        }
    }

    let mut warnings = parsed.warnings;
    warnings.extend(slab_warnings);

    // ── finding 1: fallback widths must never be silent ────────────────────
    // One warning per distinct cause (there are only two, and both can occur
    // in the same file: moves before an `M200` in a source that also lacks a
    // `filament_diameter` comment). `BTreeMap` iteration gives the fixed
    // `FallbackCause` order, never parse order or hash order.
    if let Some(w) = fallback_width_mm {
        for (cause, count) in &fallback_counts {
            let reason = match cause {
                FallbackCause::NoFilamentDiameter => {
                    "this file carries no `; filament_diameter = ...` config comment, so an E \
                     length cannot be converted to an extruded volume"
                        .to_string()
                }
                FallbackCause::VolumetricExtrusion(line) => format!(
                    "`M200` (volumetric extrusion) on source line {line} redefines E as a \
                     volume rather than a length"
                ),
            };
            warnings.push(format!(
                "{count} of {extruding_rendered} rendered extruding moves used the \
                 gcode_line_width_mm fallback ({w} mm) because {reason}: their rendered widths \
                 are NOT derived from the file"
            ));
        }
    }

    // ── finding 3: warn on an unreadably flat silhouette, never distort ────
    // Framing stays uniformly scaled and the margin is untouched: squashing
    // one axis would misrepresent deposited bead widths, which is the one
    // thing this image exists to show. So the image stays correct and the
    // reader is told it is unreadable.
    //
    // Threshold: the geometry's vertical extent covering under 1% of the
    // canvas height. At the default 2048-px canvas that is ~20 rows, so any
    // print taller than ~20 layers is guaranteed sub-pixel per-layer slabs
    // and the whole stack reads as a single line. (Measured case: a 239 mm
    // wide x 0.8 mm tall source rendered ~7 rows of 2048, or 0.33%.)
    const MIN_READABLE_VERTICAL_FRACTION: f64 = 0.01;
    let px_top = projector.project(h_min, v_max).1;
    let px_bottom = projector.project(h_min, v_min).1;
    let vertical_px = (px_bottom - px_top).abs();
    let vertical_fraction = vertical_px / f64::from(canvas_height.max(1));
    if vertical_fraction < MIN_READABLE_VERTICAL_FRACTION {
        warnings.push(format!(
            "silhouette geometry is extremely flat: its {:.3} mm vertical extent covers only \
             {:.2}% of the {canvas_height} px canvas height against a {:.3} mm horizontal \
             extent (aspect ratio {:.0}:1). The image is correctly framed and uniformly \
             scaled — no axis is distorted — but is unreadable at this ratio; render the \
             `side` view if the object is much shallower than it is wide, or render a G-code \
             source trimmed to fewer layers.",
            v_max - v_min,
            vertical_fraction * 100.0,
            h_max - h_min,
            if (v_max - v_min) > 0.0 {
                (h_max - h_min) / (v_max - v_min)
            } else {
                f64::INFINITY
            },
        ));
    }

    Ok(GcodeSilhouetteOutput {
        parser_version: GCODE_PARSER_VERSION.to_string(),
        warnings,
        png_bytes: encode_png(canvas_width, canvas_height, &buf),
        width: canvas_width,
        height: canvas_height,
        world_bounds_mm: world_bounds,
        layers_rendered,
    })
}

/// Fill the axis-aligned pixel rectangle spanned by two projected corners.
/// Inclusive on both ends, so a slab thinner than a pixel still paints one
/// row rather than vanishing.
fn fill_rect(
    buf: &mut [u8],
    width: u32,
    height: u32,
    p0: (f64, f64),
    p1: (f64, f64),
    color: [u8; 3],
) {
    let x0 = p0.0.min(p1.0).round() as i64;
    let x1 = p0.0.max(p1.0).round() as i64;
    let y0 = p0.1.min(p1.1).round() as i64;
    let y1 = p0.1.max(p1.1).round() as i64;
    for y in y0..=y1 {
        for x in x0..=x1 {
            set_pixel(buf, width, height, x, y, color);
        }
    }
}

/// Convert this module's parsed `(min_x, min_y, max_x, max_y)` mm bounds into
/// the shared [`ViewportBoundsMm`], margin included.
///
/// The `f64`→`f32` narrowing is immaterial here: over a ≤250 mm bed, `f32`
/// resolves to ~3e-5 mm, four orders of magnitude finer than the ~0.024 mm a
/// single pixel covers at the default 1024 px raster.
fn viewport_bounds(parsed_bounds: (f64, f64, f64, f64)) -> ViewportBoundsMm {
    let (min_x, min_y, max_x, max_y) = parsed_bounds;
    ViewportBoundsMm {
        min_x: min_x as f32,
        min_y: min_y as f32,
        max_x: max_x as f32,
        max_y: max_y as f32,
    }
    .with_margin()
}

/// Project a parsed G-code point through the shared [`Projector`].
fn project(projector: &Projector, p: PointMm) -> (f64, f64) {
    projector.project(p.x, p.y)
}

// ─────────────────────────────── rasterization ────────────────────────────

// The role palette (Solarized accents + gray for `"unclassified"`) lives in
// the shared style module (`slicer_runtime::visual_debug_style`) alongside
// the typed-IR renderer's palette, glyphs, and tool palette — this module
// used to own an independent copy.

/// A segment's color under the requested `color_by`.
fn segment_color(seg: &Segment, color_by: ColorBy) -> [u8; 3] {
    match color_by {
        ColorBy::Role => style::gcode_role_color(&seg.role, UNCLASSIFIED_ROLE),
        // Standalone-gcode has no config, so tool colors are always the
        // fixed shared palette (see `render_gcode_visual_debug_styled` doc).
        ColorBy::Tool => ToolColors::default().color(seg.tool),
    }
}

fn render_filament_lines(
    layer: &ParsedLayer,
    projector: &Projector,
    width: u32,
    height: u32,
    color_by: ColorBy,
) -> Vec<u8> {
    let mut buf = vec![255u8; width as usize * height as usize * 3];
    for seg in &layer.segments {
        if !seg.is_extrusion {
            continue;
        }
        let p0 = project(projector, seg.from);
        let p1 = project(projector, seg.to);
        draw_line(
            &mut buf,
            width,
            height,
            p0,
            p1,
            segment_color(seg, color_by),
        );
    }
    encode_png(width, height, &buf)
}

fn render_filled_areas(
    layer: &ParsedLayer,
    projector: &Projector,
    width: u32,
    height: u32,
    line_width_mm: f64,
    color_by: ColorBy,
) -> Vec<u8> {
    let mut buf = vec![255u8; width as usize * height as usize * 3];
    let width_px = projector.scale_mm(line_width_mm).max(1.0);
    for seg in &layer.segments {
        if !seg.is_extrusion {
            continue;
        }
        let p0 = project(projector, seg.from);
        let p1 = project(projector, seg.to);
        draw_thick_line(
            &mut buf,
            width,
            height,
            p0,
            p1,
            width_px,
            segment_color(seg, color_by),
        );
    }
    encode_png(width, height, &buf)
}

/// Every overlay event of `kind` for one parsed layer. Point events come
/// from [`ParsedLayer::events`]; travel polylines are derived here from the
/// layer's non-extrusion segments (consecutive travel segments merge into
/// one polyline), so the drawn travel and the manifest's mirror share one
/// source.
pub fn layer_overlay_events(layer: &ParsedLayer, kind: OverlayKind) -> Vec<OverlayEvent> {
    if kind == OverlayKind::Travel {
        let mut events = Vec::new();
        let mut run: Vec<[f32; 2]> = Vec::new();
        let flush = |run: &mut Vec<[f32; 2]>, events: &mut Vec<OverlayEvent>| {
            if run.len() >= 2 {
                let points = std::mem::take(run);
                let length_mm = style::polyline_length_mm(&points);
                events.push(OverlayEvent::Travel { points, length_mm });
            } else {
                run.clear();
            }
        };
        for seg in &layer.segments {
            if seg.is_extrusion {
                flush(&mut run, &mut events);
                continue;
            }
            let from = [seg.from.x as f32, seg.from.y as f32];
            let to = [seg.to.x as f32, seg.to.y as f32];
            match run.last() {
                Some(&last) if last == from => run.push(to),
                _ => {
                    flush(&mut run, &mut events);
                    run.push(from);
                    run.push(to);
                }
            }
        }
        flush(&mut run, &mut events);
        return events;
    }
    layer
        .events
        .iter()
        .filter(|e| e.kind() == kind)
        .cloned()
        .collect()
}

/// Rasterize one isolated overlay image: every extrusion centerline in
/// faint gray, then `events`' glyphs (shared legend v1.1.0).
fn render_overlay(
    layer: &ParsedLayer,
    projector: &Projector,
    width: u32,
    height: u32,
    events: &[OverlayEvent],
) -> Vec<u8> {
    let mut buf = vec![255u8; width as usize * height as usize * 3];
    for seg in &layer.segments {
        if !seg.is_extrusion {
            continue;
        }
        let p0 = project(projector, seg.from);
        let p1 = project(projector, seg.to);
        draw_line(&mut buf, width, height, p0, p1, overlay_palette::FAINT_BASE);
    }
    let glyph_half = style::GLYPH_HALF_PX * i64::from((width / 1024).max(1));
    for event in events {
        match event {
            OverlayEvent::Travel { points, .. } => {
                let px: Vec<(f64, f64)> = points
                    .iter()
                    .map(|&[x, y]| projector.project(f64::from(x), f64::from(y)))
                    .collect();
                for pair in px.windows(2) {
                    style::draw_dotted_line_px(pair[0], pair[1], &mut |x, y| {
                        set_pixel(&mut buf, width, height, x, y, overlay_palette::TRAVEL);
                    });
                }
                if let (Some(&first), true) = (px.first(), px.len() >= 2) {
                    style::draw_glyph(
                        GlyphKind::CircleOutline,
                        first.0.round() as i64,
                        first.1.round() as i64,
                        glyph_half,
                        &mut |x, y| {
                            set_pixel(&mut buf, width, height, x, y, overlay_palette::TRAVEL)
                        },
                    );
                }
                if let Some(&last) = px.last() {
                    style::draw_glyph(
                        GlyphKind::Dot,
                        last.0.round() as i64,
                        last.1.round() as i64,
                        glyph_half,
                        &mut |x, y| {
                            set_pixel(&mut buf, width, height, x, y, overlay_palette::TRAVEL)
                        },
                    );
                }
            }
            OverlayEvent::Seam { x, y }
            | OverlayEvent::Retraction { x, y, .. }
            | OverlayEvent::Unretraction { x, y, .. }
            | OverlayEvent::ZHop { x, y, .. }
            | OverlayEvent::ToolChange { x, y, .. } => {
                let (kind, color) = style::event_glyph(event);
                let (px, py) = projector.project(f64::from(*x), f64::from(*y));
                style::draw_glyph(
                    kind,
                    px.round() as i64,
                    py.round() as i64,
                    glyph_half,
                    &mut |gx, gy| set_pixel(&mut buf, width, height, gx, gy, color),
                );
            }
        }
    }
    encode_png(width, height, &buf)
}

fn set_pixel(buf: &mut [u8], width: u32, height: u32, x: i64, y: i64, color: [u8; 3]) {
    if x < 0 || y < 0 || x as u32 >= width || y as u32 >= height {
        return;
    }
    let idx = (y as u32 * width + x as u32) as usize * 3;
    buf[idx] = color[0];
    buf[idx + 1] = color[1];
    buf[idx + 2] = color[2];
}

/// Integer Bresenham line rasterization on rounded pixel coordinates.
/// Deterministic given the same input floats.
fn draw_line(
    buf: &mut [u8],
    width: u32,
    height: u32,
    p0: (f64, f64),
    p1: (f64, f64),
    color: [u8; 3],
) {
    let mut x0 = p0.0.round() as i64;
    let mut y0 = p0.1.round() as i64;
    let x1 = p1.0.round() as i64;
    let y1 = p1.1.round() as i64;
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx: i64 = if x0 < x1 { 1 } else { -1 };
    let sy: i64 = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    loop {
        set_pixel(buf, width, height, x0, y0, color);
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

/// A stroked line of `width_px` (approximated as a set of parallel offset
/// centerlines along the segment's normal), filled solid — the `filled_areas`
/// sausage-shape approximation for a G-code bead of the requested width.
fn draw_thick_line(
    buf: &mut [u8],
    width: u32,
    height: u32,
    p0: (f64, f64),
    p1: (f64, f64),
    width_px: f64,
    color: [u8; 3],
) {
    let dx = p1.0 - p0.0;
    let dy = p1.1 - p0.1;
    let len = (dx * dx + dy * dy).sqrt();
    let (nx, ny) = if len > f64::EPSILON {
        (-dy / len, dx / len)
    } else {
        (0.0, 1.0)
    };

    let half = width_px / 2.0;
    let steps = width_px.round().max(1.0) as i64;
    for i in 0..steps {
        // Offsets spread symmetrically across [-half, half].
        let t = if steps == 1 {
            0.0
        } else {
            -half + (i as f64) * (width_px / (steps - 1) as f64)
        };
        let offset_p0 = (p0.0 + nx * t, p0.1 + ny * t);
        let offset_p1 = (p1.0 + nx * t, p1.1 + ny * t);
        draw_line(buf, width, height, offset_p0, offset_p1, color);
    }
}

fn encode_png(width: u32, height: u32, rgb: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut encoder = Encoder::new(&mut out, width, height);
        encoder.set_color(ColorType::Rgb);
        encoder.set_depth(BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .expect("PNG header write cannot fail for a fixed-size in-memory buffer");
        writer
            .write_image_data(rgb)
            .expect("PNG image data write cannot fail for a correctly sized buffer");
    }
    out
}

// ─────────────────────────────────── tests ────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const SUPPORTED_SINGLE_LAYER_GCODE: &str = "\
;LAYER_CHANGE
;Z:0.2
G1 Z0.2 F600
;TYPE:Outer wall
G1 X0 Y0 F3000
G1 X10 Y0 E1.0 F1200
G1 X10 Y10 E2.0
G1 X0 Y10 E3.0
G1 X0 Y0 E4.0
";

    fn png_dimensions(bytes: &[u8]) -> (u32, u32) {
        const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n'];
        assert_eq!(&bytes[0..8], &SIGNATURE, "not a PNG file");
        assert_eq!(&bytes[12..16], b"IHDR", "IHDR must be the first PNG chunk");
        let width = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
        let height = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
        (width, height)
    }

    // ─────────────────────── AC-1: supported gcode -> manifest+PNG data ───

    #[test]
    fn ac1_supported_gcode_parses_and_renders_one_layer() {
        let out = render_gcode_visual_debug(
            SUPPORTED_SINGLE_LAYER_GCODE,
            &[0],
            &[GcodeVisualization::FilamentLines],
            256,
            256,
            None,
            GcodeFrame::Model,
        )
        .expect("a fully-supported final-gcode request should succeed");

        assert_eq!(out.parser_version, GCODE_PARSER_VERSION);
        assert_eq!(out.images.len(), 1);
        let image = &out.images[0];
        assert_eq!(image.layer_index, 0);
        assert_eq!(
            image.layer_z,
            Some(0.2),
            "the parsed ;Z: marker must populate layer_z"
        );
        assert_eq!(image.visualization, GcodeVisualization::FilamentLines);
        let (w, h) = png_dimensions(&image.png_bytes);
        assert_eq!((w, h), (256, 256));
    }

    // ─────────────────────── AC-2: unclassified extrusion retained ────────

    #[test]
    fn ac2_preserves_unclassified_extrusion_with_warning() {
        let gcode = "\
;LAYER_CHANGE
;Z:0.2
G1 Z0.2 F600
G1 X0 Y0 F3000
G1 X5 Y0 E0.5 F1200
;TYPE:Outer wall
G1 X10 Y0 E1.0
";
        let parsed = parse_gcode(gcode);
        assert_eq!(parsed.layers.len(), 1);
        let segments = &parsed.layers[0].segments;
        let unclassified: Vec<_> = segments
            .iter()
            .filter(|s| s.is_extrusion && s.role == UNCLASSIFIED_ROLE)
            .collect();
        assert_eq!(
            unclassified.len(),
            1,
            "the E-increasing move before ;TYPE: must be retained as unclassified, not dropped"
        );
        assert!(
            parsed
                .warnings
                .iter()
                .any(|w| w.to_lowercase().contains("unclassified")),
            "a warning naming the unclassified extrusion must be recorded; got {:?}",
            parsed.warnings
        );

        let out = render_gcode_visual_debug(
            gcode,
            &[0],
            &[GcodeVisualization::FilamentLines],
            128,
            128,
            None,
            GcodeFrame::Model,
        )
        .expect("role-less extrusion must still render, not fail the whole bundle");
        assert_eq!(out.images.len(), 1);
        assert!(
            out.warnings
                .iter()
                .any(|w| w.to_lowercase().contains("unclassified")),
            "the render output must also carry the unclassified warning"
        );
    }

    // ─────────────────────── AC-3: filled_areas uses requested width ──────

    #[test]
    fn ac3_filled_areas_uses_requested_line_width_not_e() {
        let narrow = render_gcode_visual_debug(
            SUPPORTED_SINGLE_LAYER_GCODE,
            &[0],
            &[GcodeVisualization::FilledAreas],
            256,
            256,
            Some(0.2),
            GcodeFrame::Model,
        )
        .expect("filled_areas with an explicit narrow width should succeed");
        let wide = render_gcode_visual_debug(
            SUPPORTED_SINGLE_LAYER_GCODE,
            &[0],
            &[GcodeVisualization::FilledAreas],
            256,
            256,
            Some(1.2),
            GcodeFrame::Model,
        )
        .expect("filled_areas with an explicit wide width should succeed");

        assert_ne!(
            narrow.images[0].png_bytes, wide.images[0].png_bytes,
            "changing only gcode_line_width_mm (E identical) must change filled_areas \
             output; width must come from the request, not E"
        );
    }

    // ─────────────────────── AC-4: motion state, layers, roles, viewport ──

    #[test]
    fn ac4_tracks_motion_state_across_two_layers_with_shared_viewport() {
        let gcode = "\
;LAYER_CHANGE
;Z:0.2
G1 Z0.2 F600
M82
;TYPE:Outer wall
G1 X0 Y0 F3000
G1 X10 Y0 E1.0 F1200
G0 X10 Y10 F3000
G1 X0 Y10 E2.0
;LAYER_CHANGE
;Z:0.4
G1 Z0.4 F600
;TYPE:Solid infill
G1 X0 Y0 F3000
G1 X10 Y0 E3.0 F1200
";
        let parsed = parse_gcode(gcode);
        assert_eq!(
            parsed.layers.len(),
            2,
            "two ;LAYER_CHANGE markers -> two layers"
        );
        assert_eq!(parsed.layers[0].layer_index, 0);
        assert_eq!(parsed.layers[0].layer_z, Some(0.2));
        assert_eq!(parsed.layers[1].layer_index, 1);
        assert_eq!(parsed.layers[1].layer_z, Some(0.4));

        // The G0 travel move must be recorded but not classified as extrusion.
        let travel_count = parsed.layers[0]
            .segments
            .iter()
            .filter(|s| !s.is_extrusion)
            .count();
        assert_eq!(
            travel_count, 1,
            "the G0 X10 Y10 travel move must be tracked, non-extruding"
        );

        let roles: Vec<&str> = parsed
            .layers
            .iter()
            .flat_map(|l| l.segments.iter())
            .filter(|s| s.is_extrusion)
            .map(|s| s.role.as_str())
            .collect();
        assert!(roles.contains(&"Outer wall"));
        assert!(roles.contains(&"Solid infill"));

        let out = render_gcode_visual_debug(
            gcode,
            &[0, 1],
            &[GcodeVisualization::FilamentLines],
            256,
            256,
            None,
            GcodeFrame::Model,
        )
        .expect("multi-layer gcode render should succeed");
        assert_eq!(
            out.images.len(),
            2,
            "both selected layers must produce their own image"
        );
        let indices: Vec<i64> = out.images.iter().map(|i| i.layer_index).collect();
        assert!(indices.contains(&0) && indices.contains(&1));
        assert_ne!(
            out.images[0].layer_z, out.images[1].layer_z,
            "the two layers' parsed ;Z: markers must differ, not both report the first layer's Z"
        );
        assert_ne!(
            out.images[0].png_bytes, out.images[1].png_bytes,
            "two distinct layers must not render identical PNGs by accident here"
        );
    }

    // ─────────────────────── AC-5: unsupported construct line warning ─────

    #[test]
    fn ac5_records_unsupported_construct_line_number_and_still_renders_rest() {
        let lines: Vec<&str> = vec![
            ";LAYER_CHANGE",
            ";Z:0.2",
            "G1 Z0.2 F600",
            ";TYPE:Outer wall",
            "G1 X0 Y0 F3000",
            "G2 X10 Y0 I5 J0 E1.0 F1200",
            "G1 X10 Y10 E2.0",
        ];
        let unsupported_line_number = 6usize;
        assert_eq!(
            lines[unsupported_line_number - 1],
            "G2 X10 Y0 I5 J0 E1.0 F1200"
        );
        let gcode = format!("{}\n", lines.join("\n"));

        let out = render_gcode_visual_debug(
            &gcode,
            &[0],
            &[GcodeVisualization::FilamentLines],
            128,
            128,
            None,
            GcodeFrame::Model,
        )
        .expect("supported moves elsewhere in the file must let the render complete");

        assert!(
            out.warnings
                .iter()
                .any(|w| w.contains(&unsupported_line_number.to_string())),
            "a warning must name the unsupported construct's source line number \
             ({unsupported_line_number}); got {:?}",
            out.warnings
        );
        assert!(!out.images.is_empty(), "supported moves must still render");
    }

    // ─────────────────────── AC-6: determinism ─────────────────────────────

    #[test]
    fn ac6_render_is_deterministic_across_two_independent_calls() {
        let a = render_gcode_visual_debug(
            SUPPORTED_SINGLE_LAYER_GCODE,
            &[0],
            &[GcodeVisualization::FilamentLines],
            256,
            256,
            None,
            GcodeFrame::Model,
        )
        .expect("first run should succeed");
        let b = render_gcode_visual_debug(
            SUPPORTED_SINGLE_LAYER_GCODE,
            &[0],
            &[GcodeVisualization::FilamentLines],
            256,
            256,
            None,
            GcodeFrame::Model,
        )
        .expect("second run should succeed");

        assert_eq!(a.warnings, b.warnings, "warning ordering must be stable");
        assert_eq!(a.images.len(), b.images.len());
        for (ia, ib) in a.images.iter().zip(b.images.iter()) {
            assert_eq!(ia.layer_index, ib.layer_index);
            assert_eq!(ia.layer_z, ib.layer_z);
            assert_eq!(
                ia.png_bytes, ib.png_bytes,
                "PNG bytes must be byte-identical across two independent calls"
            );
        }
    }

    // ─────────────────────── AC-N1: filled_areas requires line width ──────

    #[test]
    fn ac_n1_rejects_filled_areas_without_line_width() {
        let err = render_gcode_visual_debug(
            SUPPORTED_SINGLE_LAYER_GCODE,
            &[0],
            &[GcodeVisualization::FilledAreas],
            128,
            128,
            None,
            GcodeFrame::Model,
        )
        .expect_err("filled_areas without an explicit gcode_line_width_mm must be rejected");
        let message = format!("{err:?}").to_lowercase();
        assert!(
            message.contains("line_width") || message.contains("line width"),
            "the rejection must explicitly report that a line width is required; got: {message}"
        );
    }

    // ─────────────────────── AC-N2: no renderable moves ────────────────────

    #[test]
    fn ac_n2_rejects_input_with_no_supported_renderable_moves() {
        let gcode = "\
;LAYER_CHANGE
;Z:0.2
G2 X10 Y0 I5 J0
G3 X0 Y0 I-5 J0
";
        let err = render_gcode_visual_debug(
            gcode,
            &[0],
            &[GcodeVisualization::FilamentLines],
            128,
            128,
            None,
            GcodeFrame::Model,
        )
        .expect_err("a file with no supported G0/G1 renderable moves must fail");
        let message = format!("{err:?}");
        assert!(
            !message.is_empty(),
            "the rejection must carry a diagnostic message"
        );
    }

    // ─────────────────────── additional focused unit coverage ─────────────

    #[test]
    fn relative_extrusion_mode_m83_is_tracked() {
        let gcode = "\
;LAYER_CHANGE
;Z:0.2
G1 Z0.2 F600
M83
;TYPE:Outer wall
G1 X0 Y0 F3000
G1 X10 Y0 E1.0 F1200
G1 X10 Y10 E1.0
";
        let parsed = parse_gcode(gcode);
        let extrusions: Vec<_> = parsed.layers[0]
            .segments
            .iter()
            .filter(|s| s.is_extrusion)
            .collect();
        assert_eq!(
            extrusions.len(),
            2,
            "both relative-mode E deltas are positive, so both moves are extrusion"
        );
    }

    #[test]
    fn from_path_wrapper_reads_file_and_matches_text_variant() {
        let tmp = std::env::temp_dir().join(format!(
            "pnp_visual_debug_gcode_test_{}.gcode",
            std::process::id()
        ));
        fs::write(&tmp, SUPPORTED_SINGLE_LAYER_GCODE).expect("write fixture");
        let out = render_gcode_visual_debug_from_path(
            &tmp,
            &[0],
            &[GcodeVisualization::FilamentLines],
            128,
            128,
            None,
            GcodeFrame::Model,
        )
        .expect("from-path variant should succeed for a valid file");
        let _ = fs::remove_file(&tmp);
        assert_eq!(out.images.len(), 1);
    }

    #[test]
    fn missing_file_reports_io_error() {
        let missing = std::env::temp_dir().join("pnp_visual_debug_gcode_definitely_missing.gcode");
        let err = render_gcode_visual_debug_from_path(
            &missing,
            &[0],
            &[GcodeVisualization::FilamentLines],
            64,
            64,
            None,
            GcodeFrame::Model,
        )
        .expect_err("a missing file must be reported as an error, not panic");
        assert!(matches!(err, GcodeRenderError::Io(_)));
    }
}

#[cfg(test)]
mod printable_area_tests {
    use super::*;

    /// OrcaSlicer's emitted form, verbatim from a real Benchy export.
    #[test]
    fn parses_the_emitted_printable_area_form() {
        assert_eq!(
            parse_printable_area_comment(" printable_area = 0x0,220x0,220x200,0x200"),
            Some((0.0, 0.0, 220.0, 200.0))
        );
    }

    /// The key must match exactly. This file also contains
    /// `extruder_printable_area` (a different key, usually empty) and a
    /// `different_settings_to_system = ...;printable_area;...` line that names
    /// it inside a value — a substring match would latch onto either and
    /// frame every plate render to garbage.
    #[test]
    fn ignores_keys_that_merely_contain_the_name() {
        for line in [
            " extruder_printable_area = ",
            " extruder_printable_area = 0x0,100x0,100x100,0x100",
            " different_settings_to_system = brim_type;printable_area;z_hop",
            " printable_area_shape = 0x0,220x0,220x200,0x200",
        ] {
            assert_eq!(
                parse_printable_area_comment(line),
                None,
                "must not match: {line}"
            );
        }
    }

    /// Values that cannot describe a bed yield `None`, so the caller fails
    /// closed instead of framing to a degenerate or half-read box.
    #[test]
    fn rejects_unusable_values() {
        for line in [
            " printable_area = ",
            " printable_area = 0x0",                       // a point
            " printable_area = 0x0,220x0",                 // a line
            " printable_area = 5x5,5x5,5x5",               // zero area
            " printable_area = 0x0,220x0,220x200,0x200x9", // malformed point
            " printable_area = 0x0,220xNaN,220x200",       // unparseable
            " printable_area = 0x0,oops,220x200",          // no `x` separator
        ] {
            assert_eq!(
                parse_printable_area_comment(line),
                None,
                "must reject: {line}"
            );
        }
    }

    /// A bed need not start at the origin, and coordinates may be negative or
    /// fractional.
    #[test]
    fn handles_offset_negative_and_fractional_beds() {
        assert_eq!(
            parse_printable_area_comment(" printable_area = -5x-5,215.5x-5,215.5x195.5,-5x195.5"),
            Some((-5.0, -5.0, 215.5, 195.5))
        );
    }

    /// The config block is a trailer — it appears *after* every move — so the
    /// parser must still pick it up on a whole-file pass.
    #[test]
    fn picks_up_the_config_trailer_after_all_motion() {
        let gcode = "\
;LAYER_CHANGE
;Z:0.2
;TYPE:Outer wall
G1 X100 Y100 F3000
G1 X110 Y100 E1.0
G1 X110 Y110 E2.0
; printable_area = 0x0,220x0,220x200,0x200
";
        assert_eq!(
            parse_gcode(gcode).printable_area_mm,
            Some((0.0, 0.0, 220.0, 200.0))
        );
    }

    /// A file with no config block simply has no bed.
    #[test]
    fn absent_printable_area_is_none() {
        let gcode = "\
;LAYER_CHANGE
;Z:0.2
;TYPE:Outer wall
G1 X100 Y100 F3000
G1 X110 Y100 E1.0
";
        assert_eq!(parse_gcode(gcode).printable_area_mm, None);
    }
}
