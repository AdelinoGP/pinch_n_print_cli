//! Shared visual-debug style: overlay glyphs, tool palette, and overlay-event
//! types used by BOTH visual-debug renderers — the typed-IR stage renderer
//! (`visual_debug_render.rs`, this crate) and `pnp-cli`'s standalone-G-code
//! renderer (`visual_debug_gcode.rs`).
//!
//! One owner is the point (the same reason `Projector` moved here earlier):
//! the two renderers previously kept independent palettes and marker code and
//! drifted. Everything in this module is deterministic and request-independent
//! except [`ToolColors`], which is an explicit, caller-resolved input (the
//! `tool_color_source: "filament"` option) — never inferred.
//!
//! ## Glyph legend (legend v1.1.0)
//!
//! Event kinds are distinguished by SHAPE, not color alone, so a defect is
//! legible even where hues are ambiguous at marker size:
//!
//! | event        | glyph              |
//! |--------------|--------------------|
//! | seam         | filled circle      |
//! | retraction   | down-triangle      |
//! | unretraction | up-triangle        |
//! | z-hop        | diamond            |
//! | tool change  | filled square      |
//! | travel       | dotted polyline, open-circle origin, filled-dot destination |

use serde::Serialize;

/// Legend version recorded in bundle manifests once the v1.1 overlay/glyph
/// set exists. The v1.1 legend is a strict superset of v1.0 (no existing
/// color changed meaning).
pub const LEGEND_VERSION: &str = "1.1.0";

/// Overlay glyph half-size (px) at `resolution_scale: 1`; scaled by the
/// caller for larger rasters.
pub const GLYPH_HALF_PX: i64 = 6;

/// Dotted-travel-line pattern: pixels drawn per dash.
pub const DOT_ON_PX: u32 = 2;
/// Dotted-travel-line pattern: pixels skipped per gap.
pub const DOT_OFF_PX: u32 = 4;

/// Overlay event colors (fixed, request-independent). Seam and travel reuse
/// the pre-existing v1 palette values from
/// `visual_debug_render::palette` verbatim.
pub mod overlay_palette {
    /// Seam glyph (filled circle) — same red as v1 `OVERLAY_SEAM`.
    pub const SEAM: [u8; 3] = [220, 0, 0];
    /// Travel polyline + endpoint glyphs — same blue as v1 `OVERLAY_TRAVEL`.
    pub const TRAVEL: [u8; 3] = [0, 90, 220];
    /// Retraction glyph (down-triangle).
    pub const RETRACT: [u8; 3] = [200, 0, 200];
    /// Unretraction glyph (up-triangle).
    pub const UNRETRACT: [u8; 3] = [0, 160, 0];
    /// Z-hop glyph (diamond).
    pub const Z_HOP: [u8; 3] = [130, 60, 255];
    /// Tool-change glyph (filled square).
    pub const TOOL_CHANGE: [u8; 3] = [10, 10, 10];
    /// Faint base geometry under an isolated overlay render: light gray so
    /// the overlay glyphs dominate while context stays visible.
    pub const FAINT_BASE: [u8; 3] = [210, 210, 210];
}

/// Fixed, deterministic high-contrast per-tool palette for
/// `color_by: "tool"`. Indexed by `tool_index % TOOL_PALETTE.len()`.
/// Deliberately NOT real filament colors (those can be white/low-contrast);
/// `tool_color_source: "filament"` opts into config colors via [`ToolColors`].
pub const TOOL_PALETTE: [[u8; 3]; 8] = [
    [31, 119, 180],  // T0 blue
    [255, 127, 14],  // T1 orange
    [44, 160, 44],   // T2 green
    [214, 39, 40],   // T3 red
    [148, 103, 189], // T4 purple
    [140, 86, 75],   // T5 brown
    [227, 119, 194], // T6 pink
    [23, 190, 207],  // T7 cyan
];

/// The fixed per-index tool color.
#[must_use]
pub fn tool_palette_color(tool_index: u32) -> [u8; 3] {
    TOOL_PALETTE[(tool_index as usize) % TOOL_PALETTE.len()]
}

/// How geometry shapes are colored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorBy {
    /// Semantic extrusion-role palette (the v1 default).
    #[default]
    Role,
    /// Per-tool color via [`ToolColors`].
    Tool,
}

/// Resolved per-tool colors for `color_by: "tool"`.
///
/// `filament` holds caller-parsed config `filament_colour` values (index =
/// tool index); any tool without one falls back to the fixed
/// [`TOOL_PALETTE`]. `ToolColors::default()` is pure-palette.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ToolColors {
    /// Per-tool filament colors, when `tool_color_source: "filament"`.
    pub filament: Vec<Option<[u8; 3]>>,
}

impl ToolColors {
    /// The color for `tool_index`: the caller-resolved filament color when
    /// present, else the fixed palette entry.
    #[must_use]
    pub fn color(&self, tool_index: u32) -> [u8; 3] {
        self.filament
            .get(tool_index as usize)
            .copied()
            .flatten()
            .unwrap_or_else(|| tool_palette_color(tool_index))
    }
}

/// Parse a `#RRGGBB` (or `RRGGBB`) hex color, as used by the config's
/// `filament_colour` semicolon-separated list. Anything else is `None` —
/// never approximated.
#[must_use]
pub fn parse_hex_color(s: &str) -> Option<[u8; 3]> {
    let hex = s.trim().strip_prefix('#').unwrap_or_else(|| s.trim());
    if hex.len() != 6 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some([r, g, b])
}

/// One toggleable overlay event class. Each enabled kind renders as its own
/// isolated image (faint base + this kind's glyphs only).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum OverlayKind {
    /// Travel moves: dotted polylines with endpoint glyphs.
    Travel,
    /// Resolved seam positions.
    Seams,
    /// Retraction and unretraction events.
    Retractions,
    /// Z-hop events.
    ZHops,
    /// Tool-change events.
    ToolChanges,
}

impl OverlayKind {
    /// Every kind, in stable render/manifest order.
    pub const ALL: [OverlayKind; 5] = [
        OverlayKind::Travel,
        OverlayKind::Seams,
        OverlayKind::Retractions,
        OverlayKind::ZHops,
        OverlayKind::ToolChanges,
    ];

    /// Stable snake_case name, as accepted in a request's
    /// `options.overlays` list and used in filenames / manifest rows.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Travel => "travel",
            Self::Seams => "seams",
            Self::Retractions => "retractions",
            Self::ZHops => "z_hops",
            Self::ToolChanges => "tool_changes",
        }
    }

    /// Parse a request-supplied overlay name. `None` for anything unknown —
    /// the caller fails closed.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.name() == name)
    }
}

/// One rendered overlay event, mirrored into the manifest as structured JSON
/// so a consumer can reason about seams/retractions/travels numerically
/// without reading pixels. Coordinates are world-space millimeters.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum OverlayEvent {
    /// A resolved seam position.
    Seam {
        /// X, mm.
        x: f32,
        /// Y, mm.
        y: f32,
    },
    /// A retraction at a point.
    Retraction {
        /// X, mm.
        x: f32,
        /// Y, mm.
        y: f32,
        /// Retraction length, mm of filament.
        length_mm: f32,
    },
    /// An unretraction (prime) at a point.
    Unretraction {
        /// X, mm.
        x: f32,
        /// Y, mm.
        y: f32,
        /// Unretraction length, mm of filament.
        length_mm: f32,
    },
    /// A Z-hop at a point.
    ZHop {
        /// X, mm.
        x: f32,
        /// Y, mm.
        y: f32,
        /// Hop height, mm.
        height_mm: f32,
    },
    /// A tool change at a point.
    ToolChange {
        /// X, mm.
        x: f32,
        /// Y, mm.
        y: f32,
        /// Previous tool index, when known.
        from_tool: Option<u32>,
        /// New tool index.
        to_tool: u32,
    },
    /// One travel move's polyline.
    Travel {
        /// Polyline points, mm. May be a single destination point when the
        /// departure position is unknown.
        points: Vec<[f32; 2]>,
        /// Total XY path length, mm.
        length_mm: f32,
    },
}

impl OverlayEvent {
    /// The overlay kind this event belongs to.
    #[must_use]
    pub fn kind(&self) -> OverlayKind {
        match self {
            Self::Seam { .. } => OverlayKind::Seams,
            Self::Retraction { .. } | Self::Unretraction { .. } => OverlayKind::Retractions,
            Self::ZHop { .. } => OverlayKind::ZHops,
            Self::ToolChange { .. } => OverlayKind::ToolChanges,
            Self::Travel { .. } => OverlayKind::Travel,
        }
    }
}

/// Total XY polyline length in mm.
#[must_use]
pub fn polyline_length_mm(points: &[[f32; 2]]) -> f32 {
    points
        .windows(2)
        .map(|w| {
            let dx = w[1][0] - w[0][0];
            let dy = w[1][1] - w[0][1];
            (dx * dx + dy * dy).sqrt()
        })
        .sum()
}

// ────────────────────────── glyph rasterization ───────────────────────────

/// A glyph shape. Distinguishable without color (see module doc legend).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlyphKind {
    /// Seam: filled circle.
    Circle,
    /// Travel origin: circle outline.
    CircleOutline,
    /// Retraction: filled triangle pointing down.
    TriangleDown,
    /// Unretraction: filled triangle pointing up.
    TriangleUp,
    /// Z-hop: filled diamond.
    Diamond,
    /// Tool change: filled square.
    Square,
    /// Travel destination: small filled dot.
    Dot,
}

/// Rasterize one glyph centered at pixel `(cx, cy)` with half-size `half`,
/// invoking `set(x, y)` for every covered pixel. Deterministic; the caller
/// owns clipping and color.
pub fn draw_glyph<F: FnMut(i64, i64)>(kind: GlyphKind, cx: i64, cy: i64, half: i64, set: &mut F) {
    let h = half.max(1);
    match kind {
        GlyphKind::Circle => {
            for dy in -h..=h {
                for dx in -h..=h {
                    if dx * dx + dy * dy <= h * h {
                        set(cx + dx, cy + dy);
                    }
                }
            }
        }
        GlyphKind::CircleOutline => {
            let inner = (h - 2).max(0);
            for dy in -h..=h {
                for dx in -h..=h {
                    let d2 = dx * dx + dy * dy;
                    if d2 <= h * h && d2 > inner * inner {
                        set(cx + dx, cy + dy);
                    }
                }
            }
        }
        GlyphKind::TriangleDown => {
            // Apex at the bottom: row width shrinks as dy grows.
            for dy in -h..=h {
                let w = (h - dy) / 2 + (h - dy) % 2;
                let w = w.min(h);
                for dx in -w..=w {
                    set(cx + dx, cy + dy);
                }
            }
        }
        GlyphKind::TriangleUp => {
            // Apex at the top: row width shrinks as dy shrinks.
            for dy in -h..=h {
                let w = (h + dy) / 2 + (h + dy) % 2;
                let w = w.min(h);
                for dx in -w..=w {
                    set(cx + dx, cy + dy);
                }
            }
        }
        GlyphKind::Diamond => {
            for dy in -h..=h {
                let w = h - dy.abs();
                for dx in -w..=w {
                    set(cx + dx, cy + dy);
                }
            }
        }
        GlyphKind::Square => {
            for dy in -h..=h {
                for dx in -h..=h {
                    set(cx + dx, cy + dy);
                }
            }
        }
        GlyphKind::Dot => {
            let d = (h / 2).max(1);
            for dy in -d..=d {
                for dx in -d..=d {
                    if dx * dx + dy * dy <= d * d {
                        set(cx + dx, cy + dy);
                    }
                }
            }
        }
    }
}

/// The glyph for an [`OverlayEvent`]'s point marker, plus its fixed color.
/// (`Travel` returns the destination-dot pairing; its origin glyph and the
/// dotted polyline are drawn separately by the renderer.)
#[must_use]
pub fn event_glyph(event: &OverlayEvent) -> (GlyphKind, [u8; 3]) {
    match event {
        OverlayEvent::Seam { .. } => (GlyphKind::Circle, overlay_palette::SEAM),
        OverlayEvent::Retraction { .. } => (GlyphKind::TriangleDown, overlay_palette::RETRACT),
        OverlayEvent::Unretraction { .. } => (GlyphKind::TriangleUp, overlay_palette::UNRETRACT),
        OverlayEvent::ZHop { .. } => (GlyphKind::Diamond, overlay_palette::Z_HOP),
        OverlayEvent::ToolChange { .. } => (GlyphKind::Square, overlay_palette::TOOL_CHANGE),
        OverlayEvent::Travel { .. } => (GlyphKind::Dot, overlay_palette::TRAVEL),
    }
}

/// Walk the Bresenham line from `(x0, y0)` to `(x1, y1)` in pixel space,
/// invoking `set(x, y)` only for pixels inside the ON phase of a
/// [`DOT_ON_PX`]/[`DOT_OFF_PX`] dash pattern. The phase counter runs
/// continuously across the segment so the dotting is even.
pub fn draw_dotted_line_px<F: FnMut(i64, i64)>(a: (f64, f64), b: (f64, f64), set: &mut F) {
    let (mut x0, mut y0) = (a.0.round() as i64, a.1.round() as i64);
    let (x1, y1) = (b.0.round() as i64, b.1.round() as i64);
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx: i64 = if x0 < x1 { 1 } else { -1 };
    let sy: i64 = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    let period = DOT_ON_PX + DOT_OFF_PX;
    let mut phase: u32 = 0;
    loop {
        if phase % period < DOT_ON_PX {
            set(x0, y0);
        }
        phase += 1;
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

// ─────────────── shared G-code role palette (standalone path) ──────────────

/// Fixed, deterministic role color palette for the standalone G-code
/// renderer (Solarized accents), keyed by FNV-1a hash of the role string.
/// Moved here from `pnp-cli`'s `visual_debug_gcode.rs` so both renderers
/// draw from one style module.
pub const GCODE_ROLE_PALETTE: [[u8; 3]; 6] = [
    [220, 50, 47],
    [38, 139, 210],
    [133, 153, 0],
    [203, 75, 22],
    [108, 113, 196],
    [42, 161, 152],
];

/// Neutral gray for extrusion seen before any `;TYPE:` marker.
pub const GCODE_UNCLASSIFIED_COLOR: [u8; 3] = [128, 128, 128];

/// Deterministic FNV-1a hash (shared by role hashing and config tinting).
#[must_use]
pub fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// Role-string → color for the standalone G-code path.
#[must_use]
pub fn gcode_role_color(role: &str, unclassified_role: &str) -> [u8; 3] {
    if role == unclassified_role {
        return GCODE_UNCLASSIFIED_COLOR;
    }
    let hash = fnv1a(role.as_bytes());
    GCODE_ROLE_PALETTE[(hash as usize) % GCODE_ROLE_PALETTE.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_palette_cycles_deterministically() {
        assert_eq!(tool_palette_color(0), TOOL_PALETTE[0]);
        assert_eq!(tool_palette_color(8), TOOL_PALETTE[0]);
        assert_eq!(tool_palette_color(9), TOOL_PALETTE[1]);
    }

    #[test]
    fn filament_colors_override_palette_with_fallback() {
        let tc = ToolColors {
            filament: vec![Some([1, 2, 3]), None],
        };
        assert_eq!(tc.color(0), [1, 2, 3]);
        assert_eq!(tc.color(1), tool_palette_color(1));
        assert_eq!(tc.color(7), tool_palette_color(7));
    }

    #[test]
    fn hex_color_parsing_accepts_only_rrggbb() {
        assert_eq!(parse_hex_color("#26A69A"), Some([0x26, 0xA6, 0x9A]));
        assert_eq!(parse_hex_color("26A69A"), Some([0x26, 0xA6, 0x9A]));
        assert_eq!(parse_hex_color("#FFF"), None);
        assert_eq!(parse_hex_color("#GGGGGG"), None);
        assert_eq!(parse_hex_color(""), None);
    }

    #[test]
    fn overlay_kind_names_round_trip() {
        for kind in OverlayKind::ALL {
            assert_eq!(OverlayKind::parse(kind.name()), Some(kind));
        }
        assert_eq!(OverlayKind::parse("wipe"), None);
    }

    #[test]
    fn glyph_shapes_are_distinct() {
        // Each glyph's covered pixel set (same center/half) must differ from
        // every other's — shape is the primary channel, not color.
        let kinds = [
            GlyphKind::Circle,
            GlyphKind::CircleOutline,
            GlyphKind::TriangleDown,
            GlyphKind::TriangleUp,
            GlyphKind::Diamond,
            GlyphKind::Square,
            GlyphKind::Dot,
        ];
        let mut sets: Vec<std::collections::BTreeSet<(i64, i64)>> = Vec::new();
        for kind in kinds {
            let mut px = std::collections::BTreeSet::new();
            draw_glyph(kind, 0, 0, GLYPH_HALF_PX, &mut |x, y| {
                px.insert((x, y));
            });
            assert!(!px.is_empty(), "{kind:?} draws nothing");
            sets.push(px);
        }
        for i in 0..sets.len() {
            for j in (i + 1)..sets.len() {
                assert_ne!(
                    sets[i], sets[j],
                    "{:?} vs {:?} identical",
                    kinds[i], kinds[j]
                );
            }
        }
    }

    #[test]
    fn dotted_line_has_gaps() {
        let mut px = Vec::new();
        draw_dotted_line_px((0.0, 0.0), (60.0, 0.0), &mut |x, y| px.push((x, y)));
        // A 61-pixel run with a 2-on/4-off pattern draws ~1/3 of pixels.
        assert!(px.len() < 40, "dotted line drew {} of 61 pixels", px.len());
        assert!(!px.is_empty());
    }

    #[test]
    fn travel_polyline_length_is_summed() {
        let pts = [[0.0_f32, 0.0], [3.0, 4.0], [3.0, 4.0]];
        assert!((polyline_length_mm(&pts) - 5.0).abs() < 1e-6);
    }
}
