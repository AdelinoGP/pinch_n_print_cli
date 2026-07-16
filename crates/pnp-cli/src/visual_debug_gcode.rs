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
}

impl GcodeVisualization {
    pub fn name(&self) -> &'static str {
        match self {
            GcodeVisualization::FilamentLines => "filament_lines",
            GcodeVisualization::FilledAreas => "filled_areas",
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
    /// Reading the G-code file from disk failed.
    Io(String),
    /// The source contains zero supported, renderable `G0`/`G1` moves
    /// anywhere in the file (only unsupported constructs, or no motion at
    /// all). A caller must fail the whole request, not report a successful
    /// empty/partial bundle.
    NoRenderableMoves,
    /// `filled_areas` was requested without an explicit
    /// `gcode_line_width_mm`. Bead width must never be derived from `E`.
    MissingLineWidth,
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
pub fn render_gcode_visual_debug(
    gcode_text: &str,
    layer_indices: &[i64],
    visualizations: &[GcodeVisualization],
    canvas_width: u32,
    canvas_height: u32,
    gcode_line_width_mm: Option<f64>,
    frame: GcodeFrame,
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
            let png_bytes = match viz {
                GcodeVisualization::FilamentLines => {
                    render_filament_lines(layer, &projector, canvas_width, canvas_height)
                }
                GcodeVisualization::FilledAreas => render_filled_areas(
                    layer,
                    &projector,
                    canvas_width,
                    canvas_height,
                    gcode_line_width_mm.expect("checked above"),
                ),
            };
            images.push(RenderedImage {
                layer_index: layer.layer_index,
                layer_z: layer.layer_z,
                visualization: *viz,
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
}

#[derive(Debug, Clone, Default)]
pub struct ParsedLayer {
    pub layer_index: i64,
    pub layer_z: Option<f64>,
    pub segments: Vec<Segment>,
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExtrusionMode {
    Absolute,
    Relative,
}

/// Parse the documented Pinch 'n Print final-G-code subset. Public so
/// callers (and this module's own tests) can inspect structured layer/
/// warning data directly without going through PNG rendering.
pub fn parse_gcode(text: &str) -> ParsedGcode {
    let mut layers: Vec<ParsedLayer> = Vec::new();
    let mut layer_map: BTreeMap<i64, usize> = BTreeMap::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut unclassified_lines: Vec<usize> = Vec::new();

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

    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut printable_area_mm: Option<(f64, f64, f64, f64)> = None;

    for (idx, raw_line) in text.lines().enumerate() {
        let line_no = idx + 1;
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }

        if line.starts_with(";LAYER_CHANGE") {
            current_layer_index += 1;
            ensure_layer(&mut layers, &mut layer_map, current_layer_index);
            continue;
        }
        if let Some(rest) = line.strip_prefix(";Z:") {
            if let Ok(z) = rest.trim().parse::<f64>() {
                let li = ensure_layer(&mut layers, &mut layer_map, current_layer_index);
                layers[li].layer_z = Some(z);
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
            "G0" | "G1" => {
                let mut new_x = pos_x;
                let mut new_y = pos_y;
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
                        "X" => new_x = Some(value),
                        "Y" => new_y = Some(value),
                        "Z" => {} // Z lift moves don't affect the XY viewport/segments.
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
                    warnings.push(format!(
                        "line {line_no}: unsupported G-code construct outside the \
                         documented G0/G1 X/Y/Z/E/F subset: {code_part}"
                    ));
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
                    });
                }
            }
            _ => {
                warnings.push(format!(
                    "line {line_no}: unsupported G-code construct outside the documented \
                     G0/G1 X/Y/Z/E/F subset: {code_part}"
                ));
            }
        }
    }

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

/// Fixed, deterministic role color palette (Solarized accents). The special
/// role `"unclassified"` always maps to a neutral gray outside this palette.
const ROLE_PALETTE: [[u8; 3]; 6] = [
    [220, 50, 47],
    [38, 139, 210],
    [133, 153, 0],
    [203, 75, 22],
    [108, 113, 196],
    [42, 161, 152],
];
const UNCLASSIFIED_COLOR: [u8; 3] = [128, 128, 128];

fn role_color(role: &str) -> [u8; 3] {
    if role == UNCLASSIFIED_ROLE {
        return UNCLASSIFIED_COLOR;
    }
    let hash = fnv1a(role.as_bytes());
    ROLE_PALETTE[(hash as usize) % ROLE_PALETTE.len()]
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn render_filament_lines(
    layer: &ParsedLayer,
    projector: &Projector,
    width: u32,
    height: u32,
) -> Vec<u8> {
    let mut buf = vec![255u8; width as usize * height as usize * 3];
    for seg in &layer.segments {
        if !seg.is_extrusion {
            continue;
        }
        let p0 = project(projector, seg.from);
        let p1 = project(projector, seg.to);
        draw_line(&mut buf, width, height, p0, p1, role_color(&seg.role));
    }
    encode_png(width, height, &buf)
}

fn render_filled_areas(
    layer: &ParsedLayer,
    projector: &Projector,
    width: u32,
    height: u32,
    line_width_mm: f64,
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
            role_color(&seg.role),
        );
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
