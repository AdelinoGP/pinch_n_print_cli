//! Packet 159 — intermediate visual-debug renderer.
//!
//! Pure function: a packet-158 typed [`StageCapture`] (plus a requested
//! [`RenderView`], a `resolution_scale`, and a bundle-wide, already-computed
//! [`ViewportBoundsMm`]) in, deterministic PNG bytes + raster metadata out.
//! Never imports a `pnp-cli` type — `crates/pnp-cli/src/visual_debug.rs` is
//! the only caller, and owns request/bundle/manifest lifecycle (packet 157)
//! and typed capture (packet 158). This module owns none of that; it only
//! turns an already-captured typed IR into pixels.
//!
//! ## Coordinate handling
//!
//! - [`slicer_ir::Point3WithWidth`] (used by every wall/infill/support/
//!   layer-collection extrusion path) is documented in millimeters — no
//!   conversion needed.
//! - [`slicer_ir::TravelMove`]'s doc comment claims "module coordinate units
//!   (100 nm)", but every real construction site
//!   (`crates/slicer-gcode/src/emit.rs:798`) assigns a raw
//!   `Point3WithWidth.x`/`.y` (millimeters) straight through with no
//!   `units_to_mm` conversion. This module follows the real construction
//!   sites, not the stale doc comment, and treats `TravelMove` x/y/z as
//!   millimeters.
//! - [`slicer_ir::Polygon`]/[`slicer_ir::ExPolygon`] use [`slicer_ir::Point2`]
//!   (scaled integer units, 1 unit = 100 nm) — converted via `Point2::to_mm`.
//!
//! ## Fixed v1 semantic palette
//!
//! [`palette`] is a fixed, request-independent set of RGB colors keyed by
//! extrusion role / overlay kind. Never derived from request input (AC-4).

use std::fmt;

use png::{BitDepth, ColorType, Encoder};

use crate::layer_executor::{CapturedIr, StageCapture};
use crate::visual_debug_style::{
    self as style, ColorBy, GlyphKind, OverlayEvent, OverlayKind, ToolColors,
};
use slicer_ir::{
    ExPolygon, ExtrusionPath3D, ExtrusionRole, GCodeCommand, GlobalLayer, LayerAnnotationKind,
    LayerCollectionIR, PerimeterRegion, Point2, Point3WithWidth, Polygon,
};

/// Base raster dimension (px) at `resolution_scale: 1`. Actual canvas is
/// `BASE_DIMENSION_PX * resolution_scale` square, matching the
/// `pnp-cli` bundle-wide `Viewport` (`docs/19_visual_debug.md`).
pub const BASE_DIMENSION_PX: u32 = 1024;

/// Fixed margin (mm) added on each side of the shared viewport (AC-4:
/// "fixed margin", request-independent).
///
/// An absolute distance, applied equally to both axes and shared with
/// `pnp-cli`'s standalone-G-code renderer so the two paths frame identically.
/// This replaced a 5%-of-extent fraction, which scaled each axis by its own
/// extent and so distorted a non-square viewport before projection even began.
pub const VIEWPORT_MARGIN_MM: f32 = 2.0;

/// Half-width (px) of a diagnostic-overlay marker square.
const OVERLAY_MARKER_HALF_PX: i64 = 5;

/// Fixed v1 semantic palette. RGB, request-independent (AC-4).
pub mod palette {
    /// Canvas background.
    pub const BACKGROUND: [u8; 3] = [255, 255, 255];
    /// Outer wall / skirt / brim centerlines and swept bands.
    pub const OUTER_WALL: [u8; 3] = [20, 20, 20];
    /// Inner wall / thin wall / gap-fill.
    pub const INNER_WALL: [u8; 3] = [90, 90, 90];
    /// Direct `PerimeterRegion.infill_areas` polygon fill.
    pub const INFILL_AREA: [u8; 3] = [255, 196, 0];
    /// Sparse infill swept bands.
    pub const SPARSE_INFILL: [u8; 3] = [255, 140, 0];
    /// Solid infill swept bands (top/bottom/internal solid + bridge).
    pub const SOLID_INFILL: [u8; 3] = [255, 90, 0];
    /// Ironing swept bands.
    pub const IRONING: [u8; 3] = [255, 220, 130];
    /// Support material swept bands.
    pub const SUPPORT: [u8; 3] = [0, 160, 220];
    /// Support interface swept bands.
    pub const SUPPORT_INTERFACE: [u8; 3] = [0, 200, 255];
    /// Wipe/prime tower and unclassified entities.
    pub const ENTITY: [u8; 3] = [40, 120, 40];
    /// Diagnostic overlay: resolved seam position marker.
    pub const OVERLAY_SEAM: [u8; 3] = [220, 0, 0];
    /// Diagnostic overlay: travel-move anchor marker.
    pub const OVERLAY_TRAVEL: [u8; 3] = [0, 90, 220];
    /// Diagnostic overlay: guest-emitted annotation anchor marker.
    pub const OVERLAY_ANNOTATION: [u8; 3] = [200, 130, 0];
    /// Diagnostic overlay: this capture's own geometry bounding-box outline
    /// (derived, not a capture field — see `docs` grounded facts).
    pub const OVERLAY_BOUNDS: [u8; 3] = [120, 0, 180];
    /// `SurfaceClassificationIR` bridge-region `xy_footprint` polygons
    /// (packet 161, Step 6).
    pub const SURFACE_BRIDGE: [u8; 3] = [0, 120, 200];
    /// `SurfaceClassificationIR` overhang-region `xy_footprint` polygons and
    /// `overhang_quartile_polygons` bands (packet 161, Step 6).
    pub const SURFACE_OVERHANG: [u8; 3] = [200, 60, 200];
    /// Diagnostic overlay: `LayerPlanIR`'s `GlobalLayer.is_sync_layer` flag,
    /// threaded in as an opt-in annotation on a geometry tap (packet 161,
    /// Step 7) — never a standalone tap/`CapturedIr` variant.
    pub const OVERLAY_LAYERPLAN_SYNC: [u8; 3] = [255, 0, 255];
    /// Diagnostic overlay: `GlobalLayer.has_nonplanar` (packet 161, Step 7).
    pub const OVERLAY_LAYERPLAN_NONPLANAR: [u8; 3] = [0, 220, 120];
    /// Diagnostic overlay: one marker per `GlobalLayer.active_regions` entry
    /// (packet 161, Step 7).
    pub const OVERLAY_LAYERPLAN_ACTIVE_REGION: [u8; 3] = [255, 230, 0];
    /// `SupportGeometryIR.entries` coarse outline polygons (packet 161,
    /// Step 6) — distinct from `SUPPORT`/`SUPPORT_INTERFACE` (per-layer
    /// `SupportIR` swept paths) since this is the coarser prepass artifact.
    pub const SUPPORT_OUTLINE: [u8; 3] = [0, 90, 140];
    /// `SupportPlanIR.entries[].branch_segments` planned branch geometry
    /// (packet 161, Step 6).
    pub const SUPPORT_BRANCH: [u8; 3] = [0, 200, 160];
    /// `SliceIR.regions[].polygons` closed-island outline fill (packet 161,
    /// Step 6) — distinct from `INFILL_AREA` (`infill_areas`).
    pub const SLICE_REGION: [u8; 3] = [140, 140, 140];
}

/// A renderable geometry view, selected by the visual-debug request's
/// `visualizations` entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GeometryView {
    /// Swept/filled area rendering: direct `ExPolygon` geometry where
    /// available, else the deterministic width-swept shape of every
    /// extrusion path (never a zero-width centerline, never an inferred
    /// width — AC-2/AC-N2).
    FilledAreas,
    /// Zero-width centerline rendering of every extrusion path.
    FilamentLines,
}

/// The full render request for one `(tap, layer)` capture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RenderView {
    /// Plain geometry, no overlay.
    Geometry(GeometryView),
    /// The same geometry view, composited with the stable diagnostic
    /// overlay (AC-3).
    DiagnosticOverlay(GeometryView),
    /// One overlay event class rendered in isolation (schema 1.1.0): the base
    /// geometry view painted uniformly in `overlay_palette::FAINT_BASE` gray,
    /// with ONLY this overlay kind's glyphs on top. One image per enabled
    /// overlay keeps each event class legible instead of composited clutter.
    OverlayIsolated(GeometryView, OverlayKind),
}

/// How geometry is colored (schema 1.1.0 `color_by` / `tool_color_source`).
/// `RenderStyle::default()` is the v1 role-palette behavior.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RenderStyle {
    /// Role palette (default) or per-tool coloring.
    pub color_by: ColorBy,
    /// Resolved per-tool colors; only consulted when `color_by` is `Tool`.
    pub tool_colors: ToolColors,
}

/// A shared, bundle-wide world-space (mm) viewport. Computed once per
/// bundle by [`compute_viewport_bounds`] over every selected capture's
/// geometry and reused for every render call so every image entry in one
/// bundle shares byte-identical bounds (AC-4) regardless of any individual
/// capture's own extent.
///
/// `Serialize` (additive, follow-up gap fix): `pnp-cli` records this
/// verbatim on every rendered image's manifest entry so a consumer of
/// `manifest.json` — or a test — can assert byte/value-identical
/// world-space bounds across every entry in a bundle, not just the pixel
/// `Viewport{width,height}` the manifest already carried. Raw `f32` fields
/// (not a lossy formatted string) so cross-entry comparison is exact
/// equality, not an approximation.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
pub struct ViewportBoundsMm {
    /// Minimum X, millimeters.
    pub min_x: f32,
    /// Minimum Y, millimeters.
    pub min_y: f32,
    /// Maximum X, millimeters.
    pub max_x: f32,
    /// Maximum Y, millimeters.
    pub max_y: f32,
}

impl ViewportBoundsMm {
    fn width(self) -> f32 {
        (self.max_x - self.min_x).max(1e-6)
    }
    fn height(self) -> f32 {
        (self.max_y - self.min_y).max(1e-6)
    }

    /// The smallest bounds containing both `self` and `other`.
    ///
    /// Used to guarantee the model-wide viewport never clips geometry that
    /// lies outside the mesh footprint — brim, skirt, and support all can.
    #[must_use]
    pub fn union(self, other: Self) -> Self {
        Self {
            min_x: self.min_x.min(other.min_x),
            min_y: self.min_y.min(other.min_y),
            max_x: self.max_x.max(other.max_x),
            max_y: self.max_y.max(other.max_y),
        }
    }

    /// These bounds grown by [`VIEWPORT_MARGIN_MM`] on all four sides.
    ///
    /// The margin is a fixed millimeter distance applied equally to both
    /// axes: a margin expressed as a fraction of each axis' own extent is
    /// itself anisotropic, which is what this renderer used to do.
    #[must_use]
    pub fn with_margin(self) -> Self {
        Self {
            min_x: self.min_x - VIEWPORT_MARGIN_MM,
            min_y: self.min_y - VIEWPORT_MARGIN_MM,
            max_x: self.max_x + VIEWPORT_MARGIN_MM,
            max_y: self.max_y + VIEWPORT_MARGIN_MM,
        }
    }
}

/// The single world(mm)→pixel transform for **every** visual-debug render
/// path — both the typed-IR stage renderer in this module and `pnp-cli`'s
/// standalone-G-code renderer (`visual_debug_gcode.rs`).
///
/// One shared owner is the point. Packets 159 and 160 each wrote their own
/// transform: this one (ported from the G-code path, which had it right)
/// scales **uniformly** by `min(width_ratio, height_ratio)` and centers the
/// result, so a shape's aspect ratio survives projection and the unused axis
/// becomes a letterbox band. The stage renderer previously normalized X and
/// Y independently against an always-square canvas, which stretched any
/// non-square model — a Benchy footprint (~2:1) rendered visibly squashed.
///
/// `bounds` must already include any margin ([`ViewportBoundsMm::with_margin`]);
/// this type never adds one.
///
/// Y is flipped: larger mm-Y renders toward the top of the canvas (smaller
/// row index), matching the print-bed convention.
#[derive(Debug, Clone, Copy)]
pub struct Projector {
    scale: f64,
    offset_x: f64,
    offset_y: f64,
    canvas_height: f64,
}

impl Projector {
    /// Fit `bounds` inside a `canvas_width` x `canvas_height` raster,
    /// aspect-preserved and centered.
    #[must_use]
    pub fn new(bounds: ViewportBoundsMm, canvas_width: u32, canvas_height: u32) -> Self {
        let world_w = f64::from(bounds.width());
        let world_h = f64::from(bounds.height());
        let cw = f64::from(canvas_width);
        let ch = f64::from(canvas_height);
        // The single uniform scale: fit the larger relative axis, letterbox
        // the other. This `min` is the whole aspect-ratio fix.
        let scale = (cw / world_w).min(ch / world_h);
        let pad_x = (cw - world_w * scale) / 2.0;
        let pad_y = (ch - world_h * scale) / 2.0;
        Self {
            scale,
            offset_x: pad_x - f64::from(bounds.min_x) * scale,
            offset_y: pad_y - f64::from(bounds.min_y) * scale,
            canvas_height: ch,
        }
    }

    /// Project a world-space (mm) point to pixel coordinates.
    #[must_use]
    pub fn project(&self, x: f64, y: f64) -> (f64, f64) {
        (
            x * self.scale + self.offset_x,
            self.canvas_height - (y * self.scale + self.offset_y),
        )
    }

    /// Convert a world-space (mm) length to pixels. Uniform across both axes.
    #[must_use]
    pub fn scale_mm(&self, mm: f64) -> f64 {
        mm * self.scale
    }
}

/// A rendered PNG plus its raster metadata.
#[derive(Debug, Clone)]
pub struct RenderedImage {
    /// Encoded PNG bytes (RGB, 8-bit).
    pub png_bytes: Vec<u8>,
    /// Raster width in pixels (`BASE_DIMENSION_PX * resolution_scale`).
    pub width: u32,
    /// Raster height in pixels (`BASE_DIMENSION_PX * resolution_scale`).
    pub height: u32,
}

/// Typed renderer failure modes. Every variant fails outright — never a
/// partial PNG or a successful image entry (AC-N1/AC-N2/AC-N3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderError {
    /// `resolution_scale` was outside the documented `{1, 2, 3}` set.
    UnsupportedResolutionScale {
        /// The offending scale.
        scale: u32,
    },
    /// The requested visualization's documented geometry field was missing
    /// or empty for this capture — never silently rendered as a blank
    /// image.
    MissingGeometryField {
        /// The tap (stage id) this capture was taken at.
        tap: String,
        /// The layer this capture belongs to.
        layer_index: u32,
        /// The documented field name that was missing/empty.
        field: &'static str,
    },
    /// A `filled_areas` request selected a typed path whose points carry no
    /// usable `Point3WithWidth.width` (every width `<= 0.0` or non-finite).
    /// Never inferred — rejected (AC-N2).
    MissingWidth {
        /// The tap (stage id) this capture was taken at.
        tap: String,
        /// The layer this capture belongs to.
        layer_index: u32,
    },
    /// `color_by: "tool"` was requested for a tap whose captured IR carries
    /// no tool assignment (`PrintEntity.tool_index` / `GCodeCommand::ToolChange`
    /// exist only on `LayerCollection`-family and `GCodeEmit` captures).
    /// Never guessed — rejected.
    ToolColorUnavailable {
        /// The tap (stage id) this capture was taken at.
        tap: String,
        /// The layer this capture belongs to.
        layer_index: u32,
    },
    /// An isolated overlay was requested for a tap whose captured IR has no
    /// source field for that event class at all (distinct from a present but
    /// empty field, which renders a valid zero-event image).
    OverlayUnsupportedForTap {
        /// The tap (stage id) this capture was taken at.
        tap: String,
        /// The layer this capture belongs to.
        layer_index: u32,
        /// The unsupported overlay's stable name.
        overlay: &'static str,
    },
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedResolutionScale { scale } => {
                write!(f, "unsupported resolution_scale: {scale} (must be 1, 2, 3)")
            }
            Self::MissingGeometryField {
                tap,
                layer_index,
                field,
            } => write!(
                f,
                "tap '{tap}' layer {layer_index}: missing or empty required geometry field '{field}'"
            ),
            Self::MissingWidth { tap, layer_index } => write!(
                f,
                "tap '{tap}' layer {layer_index}: no usable Point3WithWidth.width for filled_areas; refusing to infer a bead width"
            ),
            Self::ToolColorUnavailable { tap, layer_index } => write!(
                f,
                "tap '{tap}' layer {layer_index}: color_by \"tool\" is unavailable — this tap's \
                 captured IR carries no tool assignment; refusing to guess one"
            ),
            Self::OverlayUnsupportedForTap {
                tap,
                layer_index,
                overlay,
            } => write!(
                f,
                "tap '{tap}' layer {layer_index}: overlay '{overlay}' has no source field on this \
                 tap's captured IR"
            ),
        }
    }
}
impl std::error::Error for RenderError {}

/// One fillable/strokeable shape in millimeter space, tagged with its fixed
/// palette color.
enum Shape {
    /// A polygon-with-holes fill (even-odd rule over contour + hole edges).
    Fill {
        contour: Vec<(f32, f32)>,
        holes: Vec<Vec<(f32, f32)>>,
        color: [u8; 3],
    },
    /// A zero-width polyline stroke.
    Line {
        points: Vec<(f32, f32)>,
        color: [u8; 3],
    },
}

/// Compute the shared, bundle-wide viewport (AC-4) as the fixed-margin XY
/// bounding box of every selected capture's geometry. Pure aggregation —
/// never reads request/config; the caller (`pnp-cli`) owns selecting which
/// captures feed the bundle.
///
/// Falls back to a unit square around the origin when no capture carries
/// any geometry (never panics, never produces a zero-size viewport).
pub fn compute_viewport_bounds(captures: &[StageCapture]) -> ViewportBoundsMm {
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;
    let mut touched = false;
    for capture in captures {
        for (x, y) in geometry_points_mm(&capture.ir) {
            touched = true;
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }
    }
    if !touched {
        min_x = 0.0;
        min_y = 0.0;
        max_x = 1.0;
        max_y = 1.0;
    }
    ViewportBoundsMm {
        min_x,
        min_y,
        max_x,
        max_y,
    }
    .with_margin()
}

/// Every XY point (millimeters) touched by this capture's geometry, across
/// every documented source field for its variant. Used both by
/// [`compute_viewport_bounds`] (aggregated across a bundle) and by the
/// diagnostic-overlay `layer_bounds` marker (this capture's own extent).
fn geometry_points_mm(ir: &CapturedIr) -> Vec<(f32, f32)> {
    let mut pts = Vec::new();
    match ir {
        CapturedIr::Perimeter(p) => {
            for region in &p.regions {
                for poly in &region.infill_areas {
                    push_expolygon_points(poly, &mut pts);
                }
                for wall in &region.walls {
                    push_path_points(&wall.path, &mut pts);
                }
            }
        }
        CapturedIr::Infill(i) => {
            for region in &i.regions {
                for path in region
                    .sparse_infill
                    .iter()
                    .chain(region.solid_infill.iter())
                    .chain(region.ironing.iter())
                {
                    push_path_points(path, &mut pts);
                }
            }
        }
        CapturedIr::Support(s) => {
            for path in s
                .support_paths
                .iter()
                .chain(s.interface_paths.iter())
                .chain(s.raft_paths.iter())
                .chain(s.ironing_paths.iter())
            {
                push_path_points(path, &mut pts);
            }
        }
        CapturedIr::LayerCollection(l) => {
            for entity in &l.ordered_entities {
                push_path_points(&entity.path, &mut pts);
            }
        }
        // SliceIR-family and composite Blackboard-read taps (packet 161,
        // Steps 3-4) plus the two whole-print PostPass taps (packet 161,
        // Step 5): geometry contribution wired in packet 161 Step 6. Every
        // Point2/ExPolygon source below converts via the same
        // `push_expolygon_points`/`point2_to_mm` (100 nm -> mm) helper used
        // above; every `Point3WithWidth`/`ExtrusionPath3D`/`GCodeCommand::Move`
        // source is already millimeters and is pushed directly, with no
        // rescale — this is what keeps a bundle mixing both kinds of source
        // on one correct shared viewport.
        CapturedIr::Slice(s) => {
            for region in &s.regions {
                for poly in region.polygons.iter().chain(region.infill_areas.iter()) {
                    push_expolygon_points(poly, &mut pts);
                }
            }
        }
        CapturedIr::SurfaceClassification(sc) => {
            for obj in sc.per_object.values() {
                for bridge in &obj.bridge_regions {
                    for poly in &bridge.xy_footprint {
                        push_expolygon_points(poly, &mut pts);
                    }
                }
                for overhang in &obj.overhang_regions {
                    for poly in &overhang.xy_footprint {
                        push_expolygon_points(poly, &mut pts);
                    }
                }
            }
            for bands in sc.overhang_quartile_polygons.values() {
                for band in bands {
                    for poly in &band.polygons {
                        push_expolygon_points(poly, &mut pts);
                    }
                }
            }
        }
        CapturedIr::SeamPlan(sp) => {
            // `Point3WithWidth` — millimeters, no conversion (see module doc).
            for entry in &sp.entries {
                let p = entry.chosen_candidate.point;
                pts.push((p.x, p.y));
            }
        }
        CapturedIr::SupportGeometry { geometry, plan } => {
            for polys in geometry.entries.values() {
                for poly in polys {
                    push_expolygon_points(poly, &mut pts);
                }
            }
            for entry in &plan.entries {
                for path in &entry.branch_segments {
                    push_path_points(path, &mut pts);
                }
            }
        }
        CapturedIr::RegionMapping { slice_ir, .. } => {
            for s in slice_ir {
                for region in &s.regions {
                    for poly in region.polygons.iter().chain(region.infill_areas.iter()) {
                        push_expolygon_points(poly, &mut pts);
                    }
                }
            }
        }
        CapturedIr::LayerFinalization(layers) => {
            for l in layers {
                for entity in &l.ordered_entities {
                    push_path_points(&entity.path, &mut pts);
                }
                for tm in &l.travel_moves {
                    if let (Some(x), Some(y)) = (tm.x, tm.y) {
                        pts.push((x, y));
                    }
                }
            }
        }
        CapturedIr::GCodeEmit(g) => {
            for cmd in &g.commands {
                if let GCodeCommand::Move {
                    x: Some(x),
                    y: Some(y),
                    ..
                } = cmd
                {
                    pts.push((*x, *y));
                }
            }
        }
    }
    pts
}

fn push_path_points(path: &ExtrusionPath3D, out: &mut Vec<(f32, f32)>) {
    for p in &path.points {
        out.push((p.x, p.y));
    }
}

fn push_expolygon_points(poly: &ExPolygon, out: &mut Vec<(f32, f32)>) {
    push_polygon_points(&poly.contour, out);
    for hole in &poly.holes {
        push_polygon_points(hole, out);
    }
}

fn push_polygon_points(poly: &Polygon, out: &mut Vec<(f32, f32)>) {
    for p in &poly.points {
        out.push(point2_to_mm(*p));
    }
}

fn point2_to_mm(p: Point2) -> (f32, f32) {
    p.to_mm()
}

fn role_color(role: &ExtrusionRole) -> [u8; 3] {
    match role {
        ExtrusionRole::OuterWall | ExtrusionRole::Skirt | ExtrusionRole::Brim => {
            palette::OUTER_WALL
        }
        ExtrusionRole::InnerWall | ExtrusionRole::ThinWall | ExtrusionRole::GapFill => {
            palette::INNER_WALL
        }
        ExtrusionRole::SparseInfill => palette::SPARSE_INFILL,
        ExtrusionRole::TopSolidInfill
        | ExtrusionRole::BottomSolidInfill
        | ExtrusionRole::InternalSolidInfill
        | ExtrusionRole::BridgeInfill => palette::SOLID_INFILL,
        ExtrusionRole::Ironing => palette::IRONING,
        ExtrusionRole::SupportMaterial => palette::SUPPORT,
        ExtrusionRole::SupportInterface => palette::SUPPORT_INTERFACE,
        _ => palette::ENTITY,
    }
}

fn usable_width(points: &[Point3WithWidth]) -> bool {
    points.iter().any(|p| p.width.is_finite() && p.width > 0.0)
}

/// Build the deterministic swept-width polygon set for one path (one `Fill`
/// shape aggregating one quad per segment). Returns `Ok(None)` for a
/// degenerate (<2-point) path — contributes nothing, not an error. Returns
/// `Err(MissingWidth)` when the path has real segments but no usable width
/// anywhere on it (AC-N2) — never infers a width.
fn swept_fill_shape(
    path: &ExtrusionPath3D,
    color: [u8; 3],
    tap: &str,
    layer_index: u32,
) -> Result<Option<Shape>, RenderError> {
    if path.points.len() < 2 {
        return Ok(None);
    }
    if !usable_width(&path.points) {
        return Err(RenderError::MissingWidth {
            tap: tap.to_string(),
            layer_index,
        });
    }
    let mut polygons: Vec<Vec<(f32, f32)>> = Vec::new();
    for pair in path.points.windows(2) {
        let (p0, p1) = (pair[0], pair[1]);
        let dx = p1.x - p0.x;
        let dy = p1.y - p0.y;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 1e-6 {
            continue;
        }
        let nx = -dy / len;
        let ny = dx / len;
        let h0 = (p0.width.max(0.0)) / 2.0;
        let h1 = (p1.width.max(0.0)) / 2.0;
        polygons.push(vec![
            (p0.x + nx * h0, p0.y + ny * h0),
            (p1.x + nx * h1, p1.y + ny * h1),
            (p1.x - nx * h1, p1.y - ny * h1),
            (p0.x - nx * h0, p0.y - ny * h0),
        ]);
    }
    if polygons.is_empty() {
        return Ok(None);
    }
    // Multiple quads sharing one color are represented as independent
    // `Shape::Fill`s so callers can treat the whole path uniformly; the
    // caller flattens them.
    Ok(Some(Shape::Fill {
        // Encode all quads as one "contour" list-of-lists via holes=[]:
        // handled specially by `shapes_from_quads` below.
        contour: polygons.remove(0),
        holes: polygons,
        color,
    }))
}

/// `swept_fill_shape` packs every quad of one path into a single `Shape` by
/// (ab)using `holes` to carry the remaining quads (each independently
/// filled, not subtracted) — expand back into one `Shape::Fill` per quad for
/// rasterization.
fn expand_swept_shape(shape: Shape) -> Vec<Shape> {
    match shape {
        Shape::Fill {
            contour,
            holes,
            color,
        } => {
            let mut out = vec![Shape::Fill {
                contour,
                holes: Vec::new(),
                color,
            }];
            for quad in holes {
                out.push(Shape::Fill {
                    contour: quad,
                    holes: Vec::new(),
                    color,
                });
            }
            out
        }
        other => vec![other],
    }
}

fn filament_lines_from_path(path: &ExtrusionPath3D) -> Option<Shape> {
    if path.points.len() < 2 {
        return None;
    }
    Some(Shape::Line {
        points: path.points.iter().map(|p| (p.x, p.y)).collect(),
        color: role_color(&path.role),
    })
}

fn perimeter_shapes(
    p: &slicer_ir::PerimeterIR,
    view: GeometryView,
    tap: &str,
    layer_index: u32,
) -> Result<Vec<Shape>, RenderError> {
    let mut shapes = Vec::new();
    match view {
        GeometryView::FilledAreas => {
            for region in &p.regions {
                for poly in &region.infill_areas {
                    shapes.push(expolygon_fill_shape(poly, palette::INFILL_AREA));
                }
                for wall in &region.walls {
                    if let Some(shape) =
                        swept_fill_shape(&wall.path, role_color(&wall.path.role), tap, layer_index)?
                    {
                        shapes.extend(expand_swept_shape(shape));
                    }
                }
            }
        }
        GeometryView::FilamentLines => {
            for region in &p.regions {
                for wall in &region.walls {
                    if let Some(shape) = filament_lines_from_path(&wall.path) {
                        shapes.push(shape);
                    }
                }
            }
        }
    }
    if shapes.is_empty() {
        return Err(RenderError::MissingGeometryField {
            tap: tap.to_string(),
            layer_index,
            field: "regions[].infill_areas/walls",
        });
    }
    Ok(shapes)
}

fn region_paths(region: &slicer_ir::InfillRegion) -> impl Iterator<Item = &ExtrusionPath3D> {
    region
        .sparse_infill
        .iter()
        .chain(region.solid_infill.iter())
        .chain(region.ironing.iter())
}

fn infill_shapes(
    i: &slicer_ir::InfillIR,
    view: GeometryView,
    tap: &str,
    layer_index: u32,
) -> Result<Vec<Shape>, RenderError> {
    let mut shapes = Vec::new();
    match view {
        GeometryView::FilledAreas => {
            for region in &i.regions {
                for path in region_paths(region) {
                    if let Some(shape) =
                        swept_fill_shape(path, role_color(&path.role), tap, layer_index)?
                    {
                        shapes.extend(expand_swept_shape(shape));
                    }
                }
            }
        }
        GeometryView::FilamentLines => {
            for region in &i.regions {
                for path in region_paths(region) {
                    if let Some(shape) = filament_lines_from_path(path) {
                        shapes.push(shape);
                    }
                }
            }
        }
    }
    if shapes.is_empty() {
        return Err(RenderError::MissingGeometryField {
            tap: tap.to_string(),
            layer_index,
            field: "regions[].sparse_infill/solid_infill/ironing",
        });
    }
    Ok(shapes)
}

fn support_paths(s: &slicer_ir::SupportIR) -> impl Iterator<Item = &ExtrusionPath3D> {
    s.support_paths
        .iter()
        .chain(s.interface_paths.iter())
        .chain(s.raft_paths.iter())
        .chain(s.ironing_paths.iter())
}

fn support_shapes(
    s: &slicer_ir::SupportIR,
    view: GeometryView,
    tap: &str,
    layer_index: u32,
) -> Result<Vec<Shape>, RenderError> {
    let mut shapes = Vec::new();
    match view {
        GeometryView::FilledAreas => {
            for path in support_paths(s) {
                if let Some(shape) =
                    swept_fill_shape(path, role_color(&path.role), tap, layer_index)?
                {
                    shapes.extend(expand_swept_shape(shape));
                }
            }
        }
        GeometryView::FilamentLines => {
            for path in support_paths(s) {
                if let Some(shape) = filament_lines_from_path(path) {
                    shapes.push(shape);
                }
            }
        }
    }
    if shapes.is_empty() {
        return Err(RenderError::MissingGeometryField {
            tap: tap.to_string(),
            layer_index,
            field: "support_paths/interface_paths/raft_paths/ironing_paths",
        });
    }
    Ok(shapes)
}

fn layer_collection_shapes(
    l: &slicer_ir::LayerCollectionIR,
    view: GeometryView,
    tap: &str,
    layer_index: u32,
) -> Result<Vec<Shape>, RenderError> {
    let mut shapes = Vec::new();
    match view {
        GeometryView::FilledAreas => {
            for entity in &l.ordered_entities {
                if let Some(shape) = swept_fill_shape(
                    &entity.path,
                    role_color(&entity.path.role),
                    tap,
                    layer_index,
                )? {
                    shapes.extend(expand_swept_shape(shape));
                }
            }
        }
        GeometryView::FilamentLines => {
            for entity in &l.ordered_entities {
                if let Some(shape) = filament_lines_from_path(&entity.path) {
                    shapes.push(shape);
                }
            }
        }
    }
    if shapes.is_empty() {
        return Err(RenderError::MissingGeometryField {
            tap: tap.to_string(),
            layer_index,
            field: "ordered_entities",
        });
    }
    Ok(shapes)
}

fn expolygon_fill_shape(poly: &ExPolygon, color: [u8; 3]) -> Shape {
    let mut contour = Vec::with_capacity(poly.contour.points.len());
    push_polygon_points(&poly.contour, &mut contour);
    let holes = poly
        .holes
        .iter()
        .map(|h| {
            let mut pts = Vec::with_capacity(h.points.len());
            push_polygon_points(h, &mut pts);
            pts
        })
        .collect();
    Shape::Fill {
        contour,
        holes,
        color,
    }
}

/// Zero-width closed-loop outline strokes (contour + each hole) for one
/// `ExPolygon`, used by `filament_lines`-style views over sources that carry
/// no extrusion path/width (`SliceIR`, `SurfaceClassificationIR`,
/// `SupportGeometryIR`, the `RegionMapping` join) — packet 161, Step 6.
fn expolygon_outline_shapes(poly: &ExPolygon, color: [u8; 3]) -> Vec<Shape> {
    let mut out = Vec::with_capacity(1 + poly.holes.len());
    let mut push_closed = |ring: &Polygon| {
        if ring.points.len() < 2 {
            return;
        }
        let mut points = Vec::with_capacity(ring.points.len() + 1);
        push_polygon_points(ring, &mut points);
        if let Some(&first) = points.first() {
            points.push(first);
        }
        out.push(Shape::Line { points, color });
    };
    push_closed(&poly.contour);
    for hole in &poly.holes {
        push_closed(hole);
    }
    out
}

/// Deterministic FNV-1a hash — used by [`config_tint`] instead of
/// `std::collections::hash_map::DefaultHasher` so the same `ResolvedConfig`
/// always tints identically across processes/builds (AC-5 purity), not just
/// within one process's randomized `HashMap` seed.
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// Deterministic per-config tint for the `RegionMapping` join (packet 161,
/// Step 6): hashes the resolved config's `Debug` representation (its only
/// generically-comparable, fully-populated form) so two `RegionPlan`s
/// resolving to the *same* `ResolvedConfig` content always render the same
/// color, and two resolving to *different* content render differently — a
/// real function of the joined config, never the fixed v1 palette (AC-4
/// governs the *request*-independent palette; this is deliberately
/// config-content-dependent instead, per this tap's own documented join
/// contract).
fn config_tint(config: &slicer_ir::ResolvedConfig) -> [u8; 3] {
    let hash = fnv1a(format!("{config:?}").as_bytes());
    let r = 60 + (hash & 0xFF) % 180;
    let g = 60 + ((hash >> 8) & 0xFF) % 180;
    let b = 60 + ((hash >> 16) & 0xFF) % 180;
    [r as u8, g as u8, b as u8]
}

fn slice_shapes(
    s: &slicer_ir::SliceIR,
    view: GeometryView,
    tap: &str,
    layer_index: u32,
) -> Result<Vec<Shape>, RenderError> {
    let mut shapes = Vec::new();
    match view {
        GeometryView::FilledAreas => {
            for region in &s.regions {
                for poly in &region.polygons {
                    shapes.push(expolygon_fill_shape(poly, palette::SLICE_REGION));
                }
                for poly in &region.infill_areas {
                    shapes.push(expolygon_fill_shape(poly, palette::INFILL_AREA));
                }
            }
        }
        GeometryView::FilamentLines => {
            for region in &s.regions {
                for poly in region.polygons.iter().chain(region.infill_areas.iter()) {
                    shapes.extend(expolygon_outline_shapes(poly, palette::SLICE_REGION));
                }
            }
        }
    }
    if shapes.is_empty() {
        return Err(RenderError::MissingGeometryField {
            tap: tap.to_string(),
            layer_index,
            field: "regions[].polygons/infill_areas",
        });
    }
    Ok(shapes)
}

fn surface_classification_shapes(
    sc: &slicer_ir::SurfaceClassificationIR,
    view: GeometryView,
    tap: &str,
    layer_index: u32,
) -> Result<Vec<Shape>, RenderError> {
    let mut shapes = Vec::new();
    // `per_object` is a whole-print `HashMap`, not per-layer — iterate every
    // object's bridge/overhang footprints every render (matches the
    // documented "whole-print, unfiltered" capture semantics), but sorted by
    // `ObjectId` for deterministic (AC-5) shape ordering.
    let mut objects: Vec<(&String, &slicer_ir::ObjectSurfaceData)> = sc.per_object.iter().collect();
    objects.sort_by(|a, b| a.0.cmp(b.0));
    for (_, obj) in objects {
        for bridge in &obj.bridge_regions {
            for poly in &bridge.xy_footprint {
                match view {
                    GeometryView::FilledAreas => {
                        shapes.push(expolygon_fill_shape(poly, palette::SURFACE_BRIDGE));
                    }
                    GeometryView::FilamentLines => {
                        shapes.extend(expolygon_outline_shapes(poly, palette::SURFACE_BRIDGE));
                    }
                }
            }
        }
        for overhang in &obj.overhang_regions {
            for poly in &overhang.xy_footprint {
                match view {
                    GeometryView::FilledAreas => {
                        shapes.push(expolygon_fill_shape(poly, palette::SURFACE_OVERHANG));
                    }
                    GeometryView::FilamentLines => {
                        shapes.extend(expolygon_outline_shapes(poly, palette::SURFACE_OVERHANG));
                    }
                }
            }
        }
    }
    // `overhang_quartile_polygons` IS per-layer keyed (its doc-pinned
    // exception) — a direct keyed lookup, so no additional sort is needed.
    if let Some(bands) = sc.overhang_quartile_polygons.get(&layer_index) {
        for band in bands {
            for poly in &band.polygons {
                match view {
                    GeometryView::FilledAreas => {
                        shapes.push(expolygon_fill_shape(poly, palette::SURFACE_OVERHANG));
                    }
                    GeometryView::FilamentLines => {
                        shapes.extend(expolygon_outline_shapes(poly, palette::SURFACE_OVERHANG));
                    }
                }
            }
        }
    }
    if shapes.is_empty() {
        return Err(RenderError::MissingGeometryField {
            tap: tap.to_string(),
            layer_index,
            field: "per_object[].bridge_regions/overhang_regions xy_footprint or overhang_quartile_polygons",
        });
    }
    Ok(shapes)
}

fn support_geometry_shapes(
    geometry: &slicer_ir::SupportGeometryIR,
    plan: &slicer_ir::SupportPlanIR,
    view: GeometryView,
    tap: &str,
    layer_index: u32,
) -> Result<Vec<Shape>, RenderError> {
    let mut shapes = Vec::new();
    // Both `entries` maps are whole-print, keyed composites (documented
    // "unfiltered by layer" capture) — the renderer restricts to the
    // requested layer here, sorted by (object_id, region_id) for
    // deterministic (AC-5) ordering within that layer.
    let mut geometry_entries: Vec<(&slicer_ir::SupportGeometryKey, &Vec<ExPolygon>)> = geometry
        .entries
        .iter()
        .filter(|(k, _)| k.global_support_layer_index == layer_index)
        .collect();
    geometry_entries
        .sort_by(|a, b| (&a.0.object_id, a.0.region_id).cmp(&(&b.0.object_id, b.0.region_id)));
    for (_, polys) in geometry_entries {
        for poly in polys {
            match view {
                GeometryView::FilledAreas => {
                    shapes.push(expolygon_fill_shape(poly, palette::SUPPORT_OUTLINE));
                }
                GeometryView::FilamentLines => {
                    shapes.extend(expolygon_outline_shapes(poly, palette::SUPPORT_OUTLINE));
                }
            }
        }
    }
    let mut plan_entries: Vec<&slicer_ir::SupportPlanEntry> = plan
        .entries
        .iter()
        .filter(|e| e.global_layer_index >= 0 && e.global_layer_index as u32 == layer_index)
        .collect();
    plan_entries.sort_by(|a, b| (&a.object_id, a.region_id).cmp(&(&b.object_id, b.region_id)));
    for entry in plan_entries {
        for path in &entry.branch_segments {
            match view {
                GeometryView::FilledAreas => {
                    if let Some(shape) =
                        swept_fill_shape(path, palette::SUPPORT_BRANCH, tap, layer_index)?
                    {
                        shapes.extend(expand_swept_shape(shape));
                    }
                }
                GeometryView::FilamentLines => {
                    if let Some(shape) = filament_lines_from_path(path) {
                        shapes.push(shape);
                    }
                }
            }
        }
    }
    if shapes.is_empty() {
        return Err(RenderError::MissingGeometryField {
            tap: tap.to_string(),
            layer_index,
            field: "SupportGeometryIR.entries / SupportPlanIR.entries[].branch_segments",
        });
    }
    Ok(shapes)
}

/// Join `RegionMapIR.entries` (this layer's `RegionKey -> RegionPlan` rows)
/// against the retained whole-print `Vec<SliceIR>` by the full
/// `(global_layer_index, object_id, region_id, variant_chain)` tuple, and
/// draw each matched `SlicedRegion.polygons` tinted by its `RegionPlan`'s
/// resolved config (packet 161, Step 6). A key with no matching `SliceIR`
/// region is skipped (not an error) — real pipelines never commit a
/// `RegionMapIR` entry without a corresponding sliced region, so this only
/// guards a same-print consistency gap without hiding a genuinely absent
/// tap-wide result (that still fails closed below via the empty-shapes
/// check).
fn region_mapping_shapes(
    region_map: &slicer_ir::RegionMapIR,
    slice_ir: &[slicer_ir::SliceIR],
    view: GeometryView,
    tap: &str,
    layer_index: u32,
) -> Result<Vec<Shape>, RenderError> {
    let mut entries: Vec<(&slicer_ir::RegionKey, &slicer_ir::RegionPlan)> = region_map
        .entries
        .iter()
        .filter(|(k, _)| k.global_layer_index == layer_index)
        .collect();
    // HashMap iteration order is not guaranteed stable across processes —
    // sort by the full join key so shape order (and therefore rendered
    // output) is deterministic (AC-5).
    entries.sort_by(|a, b| {
        (&a.0.object_id, a.0.region_id, &a.0.variant_chain).cmp(&(
            &b.0.object_id,
            b.0.region_id,
            &b.0.variant_chain,
        ))
    });

    let mut shapes = Vec::new();
    for (key, _plan) in entries {
        let Some(slice) = slice_ir
            .iter()
            .find(|s| s.global_layer_index == key.global_layer_index)
        else {
            continue;
        };
        let Some(region) = slice.regions.iter().find(|r| {
            r.object_id == key.object_id
                && r.region_id == key.region_id
                && r.variant_chain == key.variant_chain
        }) else {
            continue;
        };
        let tint = config_tint(region_map.config_for(key));
        for poly in &region.polygons {
            match view {
                GeometryView::FilledAreas => shapes.push(expolygon_fill_shape(poly, tint)),
                GeometryView::FilamentLines => {
                    shapes.extend(expolygon_outline_shapes(poly, tint));
                }
            }
        }
    }
    if shapes.is_empty() {
        return Err(RenderError::MissingGeometryField {
            tap: tap.to_string(),
            layer_index,
            field: "RegionMapIR.entries joined against Vec<SliceIR> regions[].polygons",
        });
    }
    Ok(shapes)
}

/// Whole-print `GCodeIR.commands` rendering (packet 161, Step 6). Unlike
/// every other tap, `GCodeCommand::Move` carries no per-layer marker the
/// renderer can filter on (no `global_layer_index`, no `;LAYER_CHANGE`
/// structure in the typed IR) — so this draws every `Move` in the captured
/// whole-print `GCodeIR`, not just `layer_index`'s slice. `filled_areas` is
/// never satisfiable: `GCodeCommand::Move` has no width field at all (unlike
/// `Point3WithWidth`), so a real bead width can never be recovered here —
/// this fails closed via `MissingWidth` (AC-N2's "never infer a width"),
/// exactly like a typed path with no usable width.
fn gcode_shapes(
    g: &slicer_ir::GCodeIR,
    view: GeometryView,
    tap: &str,
    layer_index: u32,
) -> Result<Vec<Shape>, RenderError> {
    if matches!(view, GeometryView::FilledAreas) {
        return Err(RenderError::MissingWidth {
            tap: tap.to_string(),
            layer_index,
        });
    }
    let mut shapes = Vec::new();
    let mut current: Vec<(f32, f32)> = Vec::new();
    let mut current_role: Option<&ExtrusionRole> = None;
    // Flush the in-progress run into a `Shape::Line` (dropped if it never
    // reached 2 points — a lone Move contributes no visible segment).
    fn flush(shapes: &mut Vec<Shape>, current: &mut Vec<(f32, f32)>, role: Option<&ExtrusionRole>) {
        if current.len() >= 2 {
            shapes.push(Shape::Line {
                points: std::mem::take(current),
                color: role_color(role.expect("len >= 2 implies a role was set for every push")),
            });
        } else {
            current.clear();
        }
    }
    for cmd in &g.commands {
        let this_move = match cmd {
            GCodeCommand::Move {
                x: Some(x),
                y: Some(y),
                role,
                ..
            } => Some((*x, *y, role)),
            _ => None,
        };
        match this_move {
            Some((x, y, role)) if current_role == Some(role) => {
                current.push((x, y));
            }
            Some((x, y, role)) => {
                flush(&mut shapes, &mut current, current_role);
                current.push((x, y));
                current_role = Some(role);
            }
            None => {
                flush(&mut shapes, &mut current, current_role);
                current_role = None;
            }
        }
    }
    flush(&mut shapes, &mut current, current_role);
    if shapes.is_empty() {
        return Err(RenderError::MissingGeometryField {
            tap: tap.to_string(),
            layer_index,
            field: "commands: no run of >= 2 consecutive Move{x, y} with a shared role",
        });
    }
    Ok(shapes)
}

fn shapes_for(
    ir: &CapturedIr,
    view: GeometryView,
    tap: &str,
    layer_index: u32,
) -> Result<Vec<Shape>, RenderError> {
    match ir {
        CapturedIr::Perimeter(p) => perimeter_shapes(p, view, tap, layer_index),
        CapturedIr::Infill(i) => infill_shapes(i, view, tap, layer_index),
        CapturedIr::Support(s) => support_shapes(s, view, tap, layer_index),
        CapturedIr::LayerCollection(l) => layer_collection_shapes(l, view, tap, layer_index),
        // SliceIR-family Blackboard-read tap (packet 161, Step 3): whole
        // region geometry (`polygons`/`infill_areas`), packet 161 Step 6.
        CapturedIr::Slice(s) => slice_shapes(s, view, tap, layer_index),
        // Composite Blackboard-read tap (packet 161, Step 4): bridge/overhang
        // footprints + per-layer overhang-quartile bands, packet 161 Step 6.
        CapturedIr::SurfaceClassification(sc) => {
            surface_classification_shapes(sc, view, tap, layer_index)
        }
        // The base geometry view has no seam-plan-native shape to draw (a
        // seam is a point, not an area/path), so this renders as an empty
        // (background-only) base image rather than erroring: the shared
        // viewport still includes every `SeamPlan` seam position via
        // `geometry_points_mm`, so the bundle-wide bounds already account
        // for it (mixed-unit AC — see `visual_debug_render_tap_tdd.rs`). The
        // seam position itself renders as a `draw_overlay` marker (packet
        // 161, Step 7) — mirroring the existing `Perimeter` seam-marker arm
        // — not as a `shapes_for` area/line shape.
        CapturedIr::SeamPlan(_) => Ok(Vec::new()),
        // Composite Blackboard-read tap (packet 161, Step 4): coarse support
        // outlines + planned branch geometry, packet 161 Step 6.
        CapturedIr::SupportGeometry { geometry, plan } => {
            support_geometry_shapes(geometry, plan, view, tap, layer_index)
        }
        // Composite Blackboard-read tap (packet 161, Step 4): the
        // `RegionKey` join against the retained whole-print `Vec<SliceIR>`,
        // tinted by each matched `RegionPlan`'s resolved config, packet 161
        // Step 6.
        CapturedIr::RegionMapping {
            region_map,
            slice_ir,
        } => region_mapping_shapes(region_map, slice_ir, view, tap, layer_index),
        // Whole-print PostPass tap (packet 161, Step 5): restrict to the
        // requested layer, then reuse the existing per-layer renderer.
        CapturedIr::LayerFinalization(layers) => {
            let Some(layer) = layers.iter().find(|l| l.global_layer_index == layer_index) else {
                return Err(RenderError::MissingGeometryField {
                    tap: tap.to_string(),
                    layer_index,
                    field: "LayerFinalization: no LayerCollectionIR for the requested layer_index",
                });
            };
            layer_collection_shapes(layer, view, tap, layer_index)
        }
        // Whole-print PostPass tap (packet 161, Step 5): `GCodeCommand::Move`
        // has no per-layer marker to filter on — see `gcode_shapes`'s doc
        // comment.
        CapturedIr::GCodeEmit(g) => gcode_shapes(g, view, tap, layer_index),
    }
}

// ───────────────────── Styled (1.1.0) shape selection ─────────────────────

/// A `Shape::Line` stroke with an explicit color (the tool-colored analogue
/// of [`filament_lines_from_path`]).
fn line_shape_with_color(path: &ExtrusionPath3D, color: [u8; 3]) -> Option<Shape> {
    if path.points.len() < 2 {
        return None;
    }
    Some(Shape::Line {
        points: path.points.iter().map(|p| (p.x, p.y)).collect(),
        color,
    })
}

/// `LayerCollectionIR.ordered_entities` colored per `PrintEntity.tool_index`
/// — the `color_by: "tool"` analogue of [`layer_collection_shapes`].
fn layer_collection_shapes_tool(
    l: &LayerCollectionIR,
    view: GeometryView,
    tap: &str,
    layer_index: u32,
    tool_colors: &ToolColors,
) -> Result<Vec<Shape>, RenderError> {
    let mut shapes = Vec::new();
    for entity in &l.ordered_entities {
        let color = tool_colors.color(entity.tool_index);
        match view {
            GeometryView::FilledAreas => {
                if let Some(shape) = swept_fill_shape(&entity.path, color, tap, layer_index)? {
                    shapes.extend(expand_swept_shape(shape));
                }
            }
            GeometryView::FilamentLines => {
                if let Some(shape) = line_shape_with_color(&entity.path, color) {
                    shapes.push(shape);
                }
            }
        }
    }
    if shapes.is_empty() {
        return Err(RenderError::MissingGeometryField {
            tap: tap.to_string(),
            layer_index,
            field: "ordered_entities",
        });
    }
    Ok(shapes)
}

/// Whole-print `GCodeIR` moves colored by the active tool, tracked through
/// `GCodeCommand::ToolChange` (tool 0 until the first change, matching the
/// emitter's initial state). `filled_areas` stays unsatisfiable exactly like
/// [`gcode_shapes`] — a `Move` has no width.
fn gcode_shapes_tool(
    g: &slicer_ir::GCodeIR,
    view: GeometryView,
    tap: &str,
    layer_index: u32,
    tool_colors: &ToolColors,
) -> Result<Vec<Shape>, RenderError> {
    if matches!(view, GeometryView::FilledAreas) {
        return Err(RenderError::MissingWidth {
            tap: tap.to_string(),
            layer_index,
        });
    }
    let mut shapes = Vec::new();
    let mut current: Vec<(f32, f32)> = Vec::new();
    let mut tool: u32 = 0;
    let flush = |shapes: &mut Vec<Shape>, current: &mut Vec<(f32, f32)>, tool: u32| {
        if current.len() >= 2 {
            shapes.push(Shape::Line {
                points: std::mem::take(current),
                color: tool_colors.color(tool),
            });
        } else {
            current.clear();
        }
    };
    for cmd in &g.commands {
        match cmd {
            GCodeCommand::Move {
                x: Some(x),
                y: Some(y),
                ..
            } => current.push((*x, *y)),
            GCodeCommand::ToolChange { to, .. } => {
                flush(&mut shapes, &mut current, tool);
                tool = *to;
            }
            _ => flush(&mut shapes, &mut current, tool),
        }
    }
    flush(&mut shapes, &mut current, tool);
    if shapes.is_empty() {
        return Err(RenderError::MissingGeometryField {
            tap: tap.to_string(),
            layer_index,
            field: "commands: no run of >= 2 consecutive Move{x, y}",
        });
    }
    Ok(shapes)
}

/// [`shapes_for`] with a [`RenderStyle`]: role coloring delegates to the v1
/// builders unchanged; `color_by: "tool"` recolors the tool-carrying captures
/// (`LayerCollection`, `LayerFinalization`, `GCodeEmit`) and fails closed
/// ([`RenderError::ToolColorUnavailable`]) for every capture whose IR carries
/// no tool assignment — never guessed.
fn shapes_for_styled(
    ir: &CapturedIr,
    view: GeometryView,
    tap: &str,
    layer_index: u32,
    render_style: &RenderStyle,
) -> Result<Vec<Shape>, RenderError> {
    match render_style.color_by {
        ColorBy::Role => shapes_for(ir, view, tap, layer_index),
        ColorBy::Tool => match ir {
            CapturedIr::LayerCollection(l) => {
                layer_collection_shapes_tool(l, view, tap, layer_index, &render_style.tool_colors)
            }
            CapturedIr::LayerFinalization(layers) => {
                let Some(layer) = layers.iter().find(|l| l.global_layer_index == layer_index)
                else {
                    return Err(RenderError::MissingGeometryField {
                        tap: tap.to_string(),
                        layer_index,
                        field:
                            "LayerFinalization: no LayerCollectionIR for the requested layer_index",
                    });
                };
                layer_collection_shapes_tool(
                    layer,
                    view,
                    tap,
                    layer_index,
                    &render_style.tool_colors,
                )
            }
            CapturedIr::GCodeEmit(g) => {
                gcode_shapes_tool(g, view, tap, layer_index, &render_style.tool_colors)
            }
            _ => Err(RenderError::ToolColorUnavailable {
                tap: tap.to_string(),
                layer_index,
            }),
        },
    }
}

/// Repaint every shape a single uniform color — the faint gray base under an
/// isolated overlay.
fn recolor_shapes(shapes: &mut [Shape], color: [u8; 3]) {
    for shape in shapes {
        match shape {
            Shape::Fill { color: c, .. } | Shape::Line { color: c, .. } => *c = color,
        }
    }
}

// ───────────────────── Overlay event collection (1.1.0) ────────────────────

/// Last XY point of the entity at `ordered_entities[index]` — the anchor
/// every `after_entity_index`-keyed event (retract, z-hop, tool change) is
/// positioned at.
fn entity_last_point(l: &LayerCollectionIR, index: u32) -> Option<(f32, f32)> {
    l.ordered_entities
        .get(index as usize)
        .and_then(|e| e.path.points.last())
        .map(|p| (p.x, p.y))
}

/// Last XY point of the entity with `entity_id` — the anchor a
/// `TravelMove` departs from.
fn entity_last_point_by_id(l: &LayerCollectionIR, entity_id: u64) -> Option<(f32, f32)> {
    l.ordered_entities
        .iter()
        .find(|e| e.entity_id == entity_id)
        .and_then(|e| e.path.points.last())
        .map(|p| (p.x, p.y))
}

/// Every event of `kind` in one `LayerCollectionIR`, in source order. An
/// event whose anchor entity is missing/degenerate is skipped (there is no
/// position to report or draw), never approximated.
fn layer_collection_events(l: &LayerCollectionIR, kind: OverlayKind) -> Vec<OverlayEvent> {
    let mut events = Vec::new();
    match kind {
        OverlayKind::Travel => {
            for tm in &l.travel_moves {
                let (Some(x), Some(y)) = (tm.x, tm.y) else {
                    continue;
                };
                let mut points: Vec<[f32; 2]> = Vec::with_capacity(2);
                if let Some((fx, fy)) = entity_last_point_by_id(l, tm.entity_id) {
                    points.push([fx, fy]);
                }
                points.push([x, y]);
                let length_mm = style::polyline_length_mm(&points);
                events.push(OverlayEvent::Travel { points, length_mm });
            }
        }
        OverlayKind::Seams => {
            // A LayerCollection carries no seam field of its own; seams come
            // from Perimeter/SeamPlan captures. Handled by the caller's
            // support matrix — this arm is unreachable there.
        }
        OverlayKind::Retractions => {
            for r in &l.retracts {
                let Some((x, y)) = entity_last_point(l, r.after_entity_index) else {
                    continue;
                };
                events.push(if r.is_unretract {
                    OverlayEvent::Unretraction {
                        x,
                        y,
                        length_mm: r.length,
                    }
                } else {
                    OverlayEvent::Retraction {
                        x,
                        y,
                        length_mm: r.length,
                    }
                });
            }
        }
        OverlayKind::ZHops => {
            for hop in &l.z_hops {
                let Some((x, y)) = entity_last_point(l, hop.after_entity_index) else {
                    continue;
                };
                events.push(OverlayEvent::ZHop {
                    x,
                    y,
                    height_mm: hop.hop_height,
                });
            }
        }
        OverlayKind::ToolChanges => {
            for tc in &l.tool_changes {
                let Some((x, y)) = entity_last_point(l, tc.after_entity_index) else {
                    continue;
                };
                events.push(OverlayEvent::ToolChange {
                    x,
                    y,
                    from_tool: Some(tc.from_tool),
                    to_tool: tc.to_tool,
                });
            }
        }
    }
    events
}

/// Events extractable from a whole-print `GCodeIR` command stream: travels
/// (runs of non-extruding `Move`s), retract/unretract commands, and tool
/// changes — each positioned at the last known XY toolhead position.
/// Z-hops and seams have no `GCodeCommand` representation and are handled by
/// the caller's support matrix.
fn gcode_events(g: &slicer_ir::GCodeIR, kind: OverlayKind) -> Vec<OverlayEvent> {
    let mut events = Vec::new();
    let mut pos: Option<(f32, f32)> = None;
    let mut travel_run: Vec<[f32; 2]> = Vec::new();
    let flush_travel = |run: &mut Vec<[f32; 2]>, events: &mut Vec<OverlayEvent>| {
        if run.len() >= 2 {
            let points = std::mem::take(run);
            let length_mm = style::polyline_length_mm(&points);
            events.push(OverlayEvent::Travel { points, length_mm });
        } else {
            run.clear();
        }
    };
    for cmd in &g.commands {
        match cmd {
            GCodeCommand::Move { x, y, e, .. } => {
                let next = match (x, y) {
                    (Some(x), Some(y)) => Some((*x, *y)),
                    _ => pos,
                };
                let extruding = e.is_some_and(|e| e > 0.0);
                if kind == OverlayKind::Travel {
                    if extruding {
                        flush_travel(&mut travel_run, &mut events);
                    } else if let Some((nx, ny)) = next {
                        if travel_run.is_empty() {
                            if let Some((px, py)) = pos {
                                travel_run.push([px, py]);
                            }
                        }
                        travel_run.push([nx, ny]);
                    }
                }
                pos = next;
            }
            GCodeCommand::Retract { length, .. } if kind == OverlayKind::Retractions => {
                if let Some((x, y)) = pos {
                    events.push(OverlayEvent::Retraction {
                        x,
                        y,
                        length_mm: *length,
                    });
                }
            }
            GCodeCommand::Unretract { length, .. } if kind == OverlayKind::Retractions => {
                if let Some((x, y)) = pos {
                    events.push(OverlayEvent::Unretraction {
                        x,
                        y,
                        length_mm: *length,
                    });
                }
            }
            GCodeCommand::ToolChange { from, to, .. } if kind == OverlayKind::ToolChanges => {
                if let Some((x, y)) = pos {
                    events.push(OverlayEvent::ToolChange {
                        x,
                        y,
                        from_tool: Some(*from),
                        to_tool: *to,
                    });
                }
            }
            _ => {
                if kind == OverlayKind::Travel {
                    flush_travel(&mut travel_run, &mut events);
                }
            }
        }
    }
    flush_travel(&mut travel_run, &mut events);
    events
}

/// Collect every overlay event of `kind` for one capture — the single source
/// both the isolated-overlay PNG glyphs and the manifest's `overlay_events`
/// JSON are produced from, so image and data can never disagree.
///
/// Support matrix (fails closed with
/// [`RenderError::OverlayUnsupportedForTap`] outside it — a tap whose IR has
/// no source *field* for the event class; an empty field is a valid
/// zero-event result):
///
/// - `LayerCollection` / `LayerFinalization`: travel, seams via
///   `Perimeter`/`SeamPlan` only — retractions, z-hops, tool changes.
/// - `Perimeter`: seams (`resolved_seam`).
/// - `SeamPlan`: seams (`entries[].chosen_candidate`).
/// - `GCodeEmit`: travel, retractions, tool changes.
pub fn collect_overlay_events(
    ir: &CapturedIr,
    kind: OverlayKind,
    tap: &str,
    layer_index: u32,
) -> Result<Vec<OverlayEvent>, RenderError> {
    let unsupported = || RenderError::OverlayUnsupportedForTap {
        tap: tap.to_string(),
        layer_index,
        overlay: kind.name(),
    };
    match ir {
        CapturedIr::LayerCollection(l) => match kind {
            OverlayKind::Seams => Err(unsupported()),
            _ => Ok(layer_collection_events(l, kind)),
        },
        CapturedIr::LayerFinalization(layers) => {
            let Some(layer) = layers.iter().find(|l| l.global_layer_index == layer_index) else {
                return Err(RenderError::MissingGeometryField {
                    tap: tap.to_string(),
                    layer_index,
                    field: "LayerFinalization: no LayerCollectionIR for the requested layer_index",
                });
            };
            match kind {
                OverlayKind::Seams => Err(unsupported()),
                _ => Ok(layer_collection_events(layer, kind)),
            }
        }
        CapturedIr::Perimeter(p) if kind == OverlayKind::Seams => Ok(p
            .regions
            .iter()
            .filter_map(seam_marker_point)
            .map(|(x, y)| OverlayEvent::Seam { x, y })
            .collect()),
        CapturedIr::SeamPlan(sp) if kind == OverlayKind::Seams => Ok(sp
            .entries
            .iter()
            .map(|entry| {
                let p = entry.chosen_candidate.point;
                OverlayEvent::Seam { x: p.x, y: p.y }
            })
            .collect()),
        CapturedIr::GCodeEmit(g) => match kind {
            OverlayKind::Travel | OverlayKind::Retractions | OverlayKind::ToolChanges => {
                Ok(gcode_events(g, kind))
            }
            _ => Err(unsupported()),
        },
        _ => Err(unsupported()),
    }
}

// ─────────────────────────────── Rasterization ────────────────────────────

struct Canvas {
    width: u32,
    height: u32,
    buf: Vec<u8>,
    /// This canvas' world(mm)→pixel transform, fixed for its whole lifetime.
    /// Owning it here is why no drawing helper takes a `ViewportBoundsMm`:
    /// there is exactly one transform per canvas, built once.
    projector: Projector,
}

impl Canvas {
    fn new(width: u32, height: u32, bounds: ViewportBoundsMm) -> Self {
        let mut buf = vec![0u8; (width as usize) * (height as usize) * 3];
        for px in buf.chunks_exact_mut(3) {
            px.copy_from_slice(&palette::BACKGROUND);
        }
        Self {
            width,
            height,
            buf,
            projector: Projector::new(bounds, width, height),
        }
    }

    fn set(&mut self, x: i64, y: i64, color: [u8; 3]) {
        if x < 0 || y < 0 || x >= self.width as i64 || y >= self.height as i64 {
            return;
        }
        let idx = (y as usize * self.width as usize + x as usize) * 3;
        self.buf[idx..idx + 3].copy_from_slice(&color);
    }

    /// Project a world-space (mm) point onto this canvas via the shared,
    /// aspect-preserving [`Projector`] — the same transform `pnp-cli`'s
    /// standalone-G-code renderer uses, so both paths frame identically.
    fn to_px(&self, x: f32, y: f32) -> (f32, f32) {
        let (px, py) = self.projector.project(f64::from(x), f64::from(y));
        (px as f32, py as f32)
    }

    fn fill_polygon(&mut self, contour: &[(f32, f32)], holes: &[Vec<(f32, f32)>], color: [u8; 3]) {
        if contour.len() < 3 {
            return;
        }
        let contour_px: Vec<(f32, f32)> = contour.iter().map(|&(x, y)| self.to_px(x, y)).collect();
        let holes_px: Vec<Vec<(f32, f32)>> = holes
            .iter()
            .map(|h| h.iter().map(|&(x, y)| self.to_px(x, y)).collect())
            .collect();
        let mut min_x = f32::MAX;
        let mut max_x = f32::MIN;
        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;
        for &(x, y) in &contour_px {
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }
        let x0 = min_x.floor().max(0.0) as i64;
        let x1 = max_x.ceil().min(self.width as f32) as i64;
        let y0 = min_y.floor().max(0.0) as i64;
        let y1 = max_y.ceil().min(self.height as f32) as i64;
        for y in y0..y1 {
            for x in x0..x1 {
                let px = x as f32 + 0.5;
                let py = y as f32 + 0.5;
                if point_in_rings(px, py, &contour_px, &holes_px) {
                    self.set(x, y, color);
                }
            }
        }
    }

    fn stroke_line(&mut self, points: &[(f32, f32)], color: [u8; 3]) {
        let px_points: Vec<(f32, f32)> = points.iter().map(|&(x, y)| self.to_px(x, y)).collect();
        for pair in px_points.windows(2) {
            self.line(pair[0], pair[1], color);
        }
    }
}

// Bresenham line rasterization (separate impl block to keep `draw_line_px`
// signature simple above).
impl Canvas {
    fn line(&mut self, a: (f32, f32), b: (f32, f32), color: [u8; 3]) {
        let (mut x0, mut y0) = (a.0.round() as i64, a.1.round() as i64);
        let (x1, y1) = (b.0.round() as i64, b.1.round() as i64);
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        loop {
            self.set(x0, y0, color);
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

    fn marker(&mut self, center: (f32, f32), color: [u8; 3]) {
        let cx = center.0.round() as i64;
        let cy = center.1.round() as i64;
        for dy in -OVERLAY_MARKER_HALF_PX..=OVERLAY_MARKER_HALF_PX {
            for dx in -OVERLAY_MARKER_HALF_PX..=OVERLAY_MARKER_HALF_PX {
                self.set(cx + dx, cy + dy, color);
            }
        }
    }

    /// Rasterize one shared-style glyph centered at a world-space (mm) point.
    fn glyph(&mut self, kind: GlyphKind, mm: (f32, f32), half: i64, color: [u8; 3]) {
        let (px, py) = self.to_px(mm.0, mm.1);
        let (cx, cy) = (px.round() as i64, py.round() as i64);
        style::draw_glyph(kind, cx, cy, half, &mut |x, y| self.set(x, y, color));
    }

    /// Stroke a world-space polyline with the shared dotted pattern.
    fn dotted_polyline(&mut self, points: &[[f32; 2]], color: [u8; 3]) {
        let px_points: Vec<(f64, f64)> = points
            .iter()
            .map(|&[x, y]| {
                let (px, py) = self.to_px(x, y);
                (f64::from(px), f64::from(py))
            })
            .collect();
        for pair in px_points.windows(2) {
            style::draw_dotted_line_px(pair[0], pair[1], &mut |x, y| self.set(x, y, color));
        }
    }

    fn rect_outline(&mut self, mm_rect: (f32, f32, f32, f32), color: [u8; 3]) {
        let (min_x, min_y, max_x, max_y) = mm_rect;
        let tl = self.to_px(min_x, max_y);
        let tr = self.to_px(max_x, max_y);
        let bl = self.to_px(min_x, min_y);
        let br = self.to_px(max_x, min_y);
        self.line(tl, tr, color);
        self.line(tr, br, color);
        self.line(br, bl, color);
        self.line(bl, tl, color);
    }
}

fn point_in_rings(px: f32, py: f32, contour: &[(f32, f32)], holes: &[Vec<(f32, f32)>]) -> bool {
    let mut inside = ray_cast(px, py, contour);
    for hole in holes {
        if ray_cast(px, py, hole) {
            inside = !inside;
        }
    }
    inside
}

fn ray_cast(px: f32, py: f32, ring: &[(f32, f32)]) -> bool {
    if ring.len() < 3 {
        return false;
    }
    let mut inside = false;
    let n = ring.len();
    for i in 0..n {
        let (x0, y0) = ring[i];
        let (x1, y1) = ring[(i + 1) % n];
        if (y0 > py) != (y1 > py) {
            let x_intersect = x0 + (py - y0) / (y1 - y0) * (x1 - x0);
            if px < x_intersect {
                inside = !inside;
            }
        }
    }
    inside
}

fn draw_shapes(canvas: &mut Canvas, shapes: &[Shape]) {
    for shape in shapes {
        match shape {
            Shape::Fill {
                contour,
                holes,
                color,
            } => canvas.fill_polygon(contour, holes, *color),
            Shape::Line { points, color } => canvas.stroke_line(points, *color),
        }
    }
}

/// Draw the stable v1 diagnostic overlay for one capture on top of an
/// already-rendered base geometry canvas. Only touches pixels within a
/// bounded marker/outline footprint — never repaints the base geometry
/// (AC-3).
///
/// `layer_plan` is the opt-in `LayerPlanIR` annotation (packet 161, Step 7):
/// `pnp-cli`'s `visual_debug.rs` looks up the requested capture's
/// `GlobalLayer` from `PrepassContext::blackboard.layer_plan()` and threads
/// it in only when the request asked for the `diagnostic_overlay`
/// visualization on a geometry tap — `LayerPlanning` has no standalone tap
/// or `CapturedIr` variant of its own, so this is the only place its
/// sync/non-planar/active-region flags ever reach a rendered image.
fn draw_overlay(canvas: &mut Canvas, ir: &CapturedIr, layer_plan: Option<&GlobalLayer>) {
    // `layer_bounds`: this capture's own geometry extent (derived, not a
    // capture field), for every variant.
    let pts = geometry_points_mm(ir);
    if let Some((min_x, min_y, max_x, max_y)) = bbox_of(&pts) {
        canvas.rect_outline((min_x, min_y, max_x, max_y), palette::OVERLAY_BOUNDS);
    }

    match ir {
        CapturedIr::Perimeter(p) => {
            for region in &p.regions {
                if let Some(seam) = seam_marker_point(region) {
                    canvas.marker(canvas.to_px(seam.0, seam.1), palette::OVERLAY_SEAM);
                }
            }
        }
        CapturedIr::LayerCollection(l) => {
            for tm in &l.travel_moves {
                if let (Some(x), Some(y)) = (tm.x, tm.y) {
                    canvas.marker(canvas.to_px(x, y), palette::OVERLAY_TRAVEL);
                }
            }
            for ann in &l.annotations {
                let _ = matches!(
                    ann.kind,
                    LayerAnnotationKind::Comment(_) | LayerAnnotationKind::Raw(_)
                );
                if let Some(entity) = l.ordered_entities.get(ann.after_entity_index as usize) {
                    if let Some(p) = entity.path.points.last() {
                        canvas.marker(canvas.to_px(p.x, p.y), palette::OVERLAY_ANNOTATION);
                    }
                }
            }
        }
        CapturedIr::SeamPlan(sp) => {
            // Mirrors the `Perimeter` seam-marker arm above: one marker per
            // `Point3WithWidth` seam position (millimeters, no rescale — see
            // module doc), rather than a `shapes_for` area/line shape.
            for entry in &sp.entries {
                let p = entry.chosen_candidate.point;
                canvas.marker(canvas.to_px(p.x, p.y), palette::OVERLAY_SEAM);
            }
        }
        _ => {}
    }

    // `LayerPlanIR` flags (packet 161, Step 7): fixed canvas-space markers,
    // independent of world-space `bounds` — these flags describe the whole
    // global layer, not an XY position, so there is no mm coordinate to
    // project. Positioned near the raster's top-left corner, well inside
    // every supported `resolution_scale` (canvas only grows with scale).
    if let Some(gl) = layer_plan {
        if gl.is_sync_layer {
            canvas.marker((10.0, 10.0), palette::OVERLAY_LAYERPLAN_SYNC);
        }
        if gl.has_nonplanar {
            canvas.marker((10.0, 30.0), palette::OVERLAY_LAYERPLAN_NONPLANAR);
        }
        for (i, _region) in gl.active_regions.iter().enumerate() {
            canvas.marker(
                (10.0 + i as f32 * 20.0, 50.0),
                palette::OVERLAY_LAYERPLAN_ACTIVE_REGION,
            );
        }
    }
}

/// Draw one overlay event class's glyphs (legend v1.1.0, shared style
/// module) over an already-painted faint base. `glyph_half` is
/// [`style::GLYPH_HALF_PX`] scaled by the raster's `resolution_scale` so
/// glyphs stay proportionate at 2x/3x.
fn draw_overlay_events(canvas: &mut Canvas, events: &[OverlayEvent], glyph_half: i64) {
    for event in events {
        match event {
            OverlayEvent::Travel { points, .. } => {
                canvas.dotted_polyline(points, style::overlay_palette::TRAVEL);
                if let Some(&[x, y]) = points.first() {
                    if points.len() >= 2 {
                        canvas.glyph(
                            GlyphKind::CircleOutline,
                            (x, y),
                            glyph_half,
                            style::overlay_palette::TRAVEL,
                        );
                    }
                }
                if let Some(&[x, y]) = points.last() {
                    canvas.glyph(
                        GlyphKind::Dot,
                        (x, y),
                        glyph_half,
                        style::overlay_palette::TRAVEL,
                    );
                }
            }
            OverlayEvent::Seam { x, y }
            | OverlayEvent::Retraction { x, y, .. }
            | OverlayEvent::Unretraction { x, y, .. }
            | OverlayEvent::ZHop { x, y, .. }
            | OverlayEvent::ToolChange { x, y, .. } => {
                let (kind, color) = style::event_glyph(event);
                canvas.glyph(kind, (*x, *y), glyph_half, color);
            }
        }
    }
}

fn seam_marker_point(region: &PerimeterRegion) -> Option<(f32, f32)> {
    region
        .resolved_seam
        .as_ref()
        .map(|s| (s.point.x, s.point.y))
}

fn bbox_of(pts: &[(f32, f32)]) -> Option<(f32, f32, f32, f32)> {
    if pts.is_empty() {
        return None;
    }
    let mut min_x = f32::MAX;
    let mut max_x = f32::MIN;
    let mut min_y = f32::MAX;
    let mut max_y = f32::MIN;
    for &(x, y) in pts {
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }
    Some((min_x, min_y, max_x, max_y))
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
            .expect("PNG image-data write cannot fail for a correctly-sized RGB buffer");
    }
    out
}

/// Render one typed capture's requested view into a deterministic PNG.
///
/// Pure function: same `(capture, view, resolution_scale, viewport)` always
/// produces byte-identical output (AC-5). Fails closed — never a partial
/// PNG — on an unsupported scale (AC-N3), a missing/empty documented
/// geometry field (AC-N1), or a `filled_areas` request over a typed path
/// with no usable width (AC-N2).
///
/// Thin wrapper over [`render_stage_capture_with_layer_plan`] with
/// `layer_plan_overlay: None` — preserves this function's original 4-arg
/// signature so existing callers (`crates/pnp-cli/tests/
/// visual_debug_intermediate_renderer_tdd.rs`, out of packet 161 Step 7's
/// edit scope) are unaffected by the Step 7 `LayerPlanIR`-overlay addition.
pub fn render_stage_capture(
    capture: &StageCapture,
    view: RenderView,
    resolution_scale: u32,
    viewport: ViewportBoundsMm,
) -> Result<RenderedImage, RenderError> {
    render_stage_capture_with_layer_plan(capture, view, resolution_scale, viewport, None)
}

/// Same contract as [`render_stage_capture`], plus an opt-in `LayerPlanIR`
/// annotation (packet 161, Step 7 — see [`draw_overlay`]'s doc comment):
/// pure over `(capture, view, resolution_scale, viewport,
/// layer_plan_overlay)` (AC-5). `layer_plan_overlay` is ignored entirely
/// unless `view` is `RenderView::DiagnosticOverlay`, so passing `Some(..)`
/// alongside a plain `RenderView::Geometry` request never draws it — the
/// flags only ever surface composited onto the `diagnostic_overlay`
/// visualization of a geometry tap, never as a standalone render.
pub fn render_stage_capture_with_layer_plan(
    capture: &StageCapture,
    view: RenderView,
    resolution_scale: u32,
    viewport: ViewportBoundsMm,
    layer_plan_overlay: Option<&GlobalLayer>,
) -> Result<RenderedImage, RenderError> {
    render_stage_capture_styled(
        capture,
        view,
        resolution_scale,
        viewport,
        layer_plan_overlay,
        &RenderStyle::default(),
    )
    .map(|(image, _events)| image)
}

/// Full 1.1.0 entry point: [`render_stage_capture_with_layer_plan`]'s
/// contract plus a [`RenderStyle`] (`color_by` / tool colors) and, for a
/// [`RenderView::OverlayIsolated`] request, the overlay's structured
/// [`OverlayEvent`]s — the exact events the returned PNG's glyphs were drawn
/// from, for the manifest's `overlay_events` mirror. Non-overlay views
/// return an empty event list. Pure over all six inputs (AC-5).
pub fn render_stage_capture_styled(
    capture: &StageCapture,
    view: RenderView,
    resolution_scale: u32,
    viewport: ViewportBoundsMm,
    layer_plan_overlay: Option<&GlobalLayer>,
    render_style: &RenderStyle,
) -> Result<(RenderedImage, Vec<OverlayEvent>), RenderError> {
    if !(1..=3).contains(&resolution_scale) {
        return Err(RenderError::UnsupportedResolutionScale {
            scale: resolution_scale,
        });
    }
    let geometry_view = match view {
        RenderView::Geometry(g) => g,
        RenderView::DiagnosticOverlay(g) => g,
        RenderView::OverlayIsolated(g, _) => g,
    };

    let width = BASE_DIMENSION_PX * resolution_scale;
    let height = BASE_DIMENSION_PX * resolution_scale;
    let mut canvas = Canvas::new(width, height, viewport);
    let mut events = Vec::new();

    match view {
        RenderView::Geometry(_) | RenderView::DiagnosticOverlay(_) => {
            let shapes = shapes_for_styled(
                &capture.ir,
                geometry_view,
                &capture.stage_id,
                capture.layer_index,
                render_style,
            )?;
            draw_shapes(&mut canvas, &shapes);
            if matches!(view, RenderView::DiagnosticOverlay(_)) {
                draw_overlay(&mut canvas, &capture.ir, layer_plan_overlay);
            }
        }
        RenderView::OverlayIsolated(_, overlay_kind) => {
            // Collect events FIRST: an unsupported (tap, overlay) pairing
            // must fail closed before any pixels are produced.
            events = collect_overlay_events(
                &capture.ir,
                overlay_kind,
                &capture.stage_id,
                capture.layer_index,
            )?;
            // Faint gray base geometry: the v1 role-colored shape set,
            // repainted uniformly so the overlay glyphs dominate. A capture
            // with no base shape at all (e.g. SeamPlan) renders glyphs over
            // plain background, matching `shapes_for`'s existing contract.
            let mut shapes = shapes_for(
                &capture.ir,
                geometry_view,
                &capture.stage_id,
                capture.layer_index,
            )?;
            recolor_shapes(&mut shapes, style::overlay_palette::FAINT_BASE);
            draw_shapes(&mut canvas, &shapes);
            let glyph_half = style::GLYPH_HALF_PX * i64::from(resolution_scale);
            draw_overlay_events(&mut canvas, &events, glyph_half);
        }
    }

    let png_bytes = encode_png(width, height, &canvas.buf);
    Ok((
        RenderedImage {
            png_bytes,
            width,
            height,
        },
        events,
    ))
}
