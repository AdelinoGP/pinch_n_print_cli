//! IR fixture builders for tests.

use std::collections::HashMap;

use crate::views::{PerimeterRegionView, SliceRegionView};
use slicer_ir::{
    mm_to_units, ConfigValue, ConfigView, ExPolygon, ExtrusionPath3D, ExtrusionRole,
    LayerCollectionIR, LoopType, Point3WithWidth, Polygon, PrintEntity, RegionKey, SeamCandidate,
    SeamReason, ToolChange, WallBoundaryType, WallFeatureFlags, WallLoop, WidthProfile,
};

/// Builder for creating [`ConfigView`] fixtures.
#[derive(Debug, Default)]
pub struct ConfigViewBuilder {
    fields: HashMap<String, ConfigValue>,
}

impl ConfigViewBuilder {
    /// Create a new empty config view builder.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_sdk::test_support::fixtures::ConfigViewBuilder;
    ///
    /// let _builder = ConfigViewBuilder::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an integer key/value pair.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_sdk::test_support::fixtures::ConfigViewBuilder;
    ///
    /// let _config = ConfigViewBuilder::new().int("count", 2).build();
    /// ```
    #[must_use]
    pub fn int(mut self, key: impl Into<String>, value: i64) -> Self {
        self.fields.insert(key.into(), ConfigValue::Int(value));
        self
    }

    /// Add a float key/value pair.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_sdk::test_support::fixtures::ConfigViewBuilder;
    ///
    /// let _config = ConfigViewBuilder::new().float("density", 0.2).build();
    /// ```
    #[must_use]
    pub fn float(mut self, key: impl Into<String>, value: f64) -> Self {
        self.fields.insert(key.into(), ConfigValue::Float(value));
        self
    }

    /// Add a boolean key/value pair.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_sdk::test_support::fixtures::ConfigViewBuilder;
    ///
    /// let _config = ConfigViewBuilder::new().bool("enabled", true).build();
    /// ```
    #[must_use]
    pub fn bool(mut self, key: impl Into<String>, value: bool) -> Self {
        self.fields.insert(key.into(), ConfigValue::Bool(value));
        self
    }

    /// Add a string key/value pair.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_sdk::test_support::fixtures::ConfigViewBuilder;
    ///
    /// let _config = ConfigViewBuilder::new().string("pattern", "grid").build();
    /// ```
    #[must_use]
    pub fn string(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.fields
            .insert(key.into(), ConfigValue::String(value.into()));
        self
    }

    /// Add a list key/value pair.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_ir::ConfigValue;
    /// use slicer_sdk::test_support::fixtures::ConfigViewBuilder;
    ///
    /// let _config = ConfigViewBuilder::new()
    ///     .list("speeds", vec![ConfigValue::Float(1.0), ConfigValue::Float(2.0)])
    ///     .build();
    /// ```
    #[must_use]
    pub fn list(mut self, key: impl Into<String>, values: Vec<ConfigValue>) -> Self {
        self.fields.insert(key.into(), ConfigValue::List(values));
        self
    }

    /// Build a [`ConfigView`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_sdk::test_support::fixtures::ConfigViewBuilder;
    ///
    /// let config = ConfigViewBuilder::new().int("count", 1).build();
    /// assert_eq!(config.len(), 1);
    /// ```
    #[must_use]
    pub fn build(self) -> ConfigView {
        ConfigView::from_map(self.fields)
    }
}

/// Builder for creating [`SliceRegionView`] fixtures.
///
/// Produces a read-only `SliceRegionView` (from slicer-sdk) suitable for
/// module testing. When no explicit infill areas are added, the builder
/// auto-clones polygons into infill areas for convenience.
#[derive(Debug, Default)]
pub struct SliceRegionViewBuilder {
    object_id: String,
    region_id: u64,
    z: f32,
    effective_layer_height: f32,
    has_nonplanar: bool,
    polygons: Vec<ExPolygon>,
    infill_areas: Vec<ExPolygon>,
    infill_areas_explicit: bool,
    top_shell_index: Option<u8>,
    top_solid_fill: Vec<ExPolygon>,
    bottom_shell_index: Option<u8>,
    bottom_solid_fill: Vec<ExPolygon>,
    is_bridge: bool,
    bridge_areas: Vec<ExPolygon>,
    bridge_orientation_deg: f32,
    sparse_infill_area: Vec<ExPolygon>,
}

impl SliceRegionViewBuilder {
    /// Create a new slice region view builder with sensible defaults.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_sdk::test_support::fixtures::SliceRegionViewBuilder;
    ///
    /// let _builder = SliceRegionViewBuilder::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            object_id: "obj-0".to_string(),
            region_id: 0,
            z: 0.0,
            effective_layer_height: 0.2,
            has_nonplanar: false,
            polygons: Vec::new(),
            infill_areas: Vec::new(),
            infill_areas_explicit: false,
            top_shell_index: None,
            top_solid_fill: Vec::new(),
            bottom_shell_index: None,
            bottom_solid_fill: Vec::new(),
            is_bridge: false,
            bridge_areas: Vec::new(),
            bridge_orientation_deg: 0.0,
            sparse_infill_area: Vec::new(),
        }
    }

    /// Set object id.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_sdk::test_support::fixtures::SliceRegionViewBuilder;
    ///
    /// let _builder = SliceRegionViewBuilder::new().object_id("obj-1");
    /// ```
    #[must_use]
    pub fn object_id(mut self, object_id: impl Into<String>) -> Self {
        self.object_id = object_id.into();
        self
    }

    /// Set region id.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_sdk::test_support::fixtures::SliceRegionViewBuilder;
    ///
    /// let _builder = SliceRegionViewBuilder::new().region_id(5);
    /// ```
    #[must_use]
    pub fn region_id(mut self, region_id: u64) -> Self {
        self.region_id = region_id;
        self
    }

    /// Set the Z height in millimeters.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_sdk::test_support::fixtures::SliceRegionViewBuilder;
    ///
    /// let view = SliceRegionViewBuilder::new().z(1.2).build();
    /// assert!((view.z() - 1.2).abs() < f32::EPSILON);
    /// ```
    #[must_use]
    pub fn z(mut self, z_mm: f32) -> Self {
        self.z = z_mm;
        self
    }

    /// Set the effective layer height in millimeters.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_sdk::test_support::fixtures::SliceRegionViewBuilder;
    ///
    /// let _builder = SliceRegionViewBuilder::new().effective_layer_height(0.24);
    /// ```
    #[must_use]
    pub fn effective_layer_height(mut self, value_mm: f32) -> Self {
        self.effective_layer_height = value_mm;
        self
    }

    /// Set whether this region has non-planar surfaces.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_sdk::test_support::fixtures::SliceRegionViewBuilder;
    ///
    /// let view = SliceRegionViewBuilder::new().has_nonplanar(true).build();
    /// assert!(view.has_nonplanar());
    /// ```
    #[must_use]
    pub fn has_nonplanar(mut self, value: bool) -> Self {
        self.has_nonplanar = value;
        self
    }

    /// Add a polygon to the region's polygon collection.
    ///
    /// When no explicit infill areas are added via [`add_infill_area`](Self::add_infill_area),
    /// polygons are auto-cloned into infill areas on build.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_sdk::test_support::fixtures::{square_polygon, SliceRegionViewBuilder};
    ///
    /// let view = SliceRegionViewBuilder::new()
    ///     .add_polygon(square_polygon(0.0, 0.0, 10.0))
    ///     .build();
    /// assert_eq!(view.polygons().len(), 1);
    /// ```
    #[must_use]
    pub fn add_polygon(mut self, polygon: ExPolygon) -> Self {
        self.polygons.push(polygon);
        self
    }

    /// Add an infill area independently from polygons.
    ///
    /// Once called, the auto-clone from polygons is disabled.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_sdk::test_support::fixtures::{square_polygon, SliceRegionViewBuilder};
    ///
    /// let view = SliceRegionViewBuilder::new()
    ///     .add_polygon(square_polygon(0.0, 0.0, 20.0))
    ///     .add_infill_area(square_polygon(5.0, 5.0, 10.0))
    ///     .build();
    /// assert_eq!(view.polygons().len(), 1);
    /// assert_eq!(view.infill_areas().len(), 1);
    /// ```
    #[must_use]
    pub fn add_infill_area(mut self, polygon: ExPolygon) -> Self {
        self.infill_areas.push(polygon);
        self.infill_areas_explicit = true;
        self
    }

    /// Set the top-shell index (`Some(0)` = exposed top; `None` = outside any
    /// top shell). Mirrors [`SliceRegionView::set_top_shell_index`].
    #[must_use]
    pub fn top_shell_index(mut self, idx: Option<u8>) -> Self {
        self.top_shell_index = idx;
        self
    }

    /// Set the polygon-precise top solid-fill area for this region.
    /// Mirrors [`SliceRegionView::set_top_solid_fill`].
    #[must_use]
    pub fn top_solid_fill(mut self, fills: Vec<ExPolygon>) -> Self {
        self.top_solid_fill = fills;
        self
    }

    /// Set the bottom-shell index (`Some(0)` = exposed bottom; `None` =
    /// outside any bottom shell). Mirrors
    /// [`SliceRegionView::set_bottom_shell_index`].
    #[must_use]
    pub fn bottom_shell_index(mut self, idx: Option<u8>) -> Self {
        self.bottom_shell_index = idx;
        self
    }

    /// Set the polygon-precise bottom solid-fill area for this region.
    /// Mirrors [`SliceRegionView::set_bottom_solid_fill`].
    #[must_use]
    pub fn bottom_solid_fill(mut self, fills: Vec<ExPolygon>) -> Self {
        self.bottom_solid_fill = fills;
        self
    }

    /// Set the bridge classification flag for this region.
    /// Mirrors [`SliceRegionView::set_is_bridge`].
    #[must_use]
    pub fn is_bridge(mut self, on: bool) -> Self {
        self.is_bridge = on;
        self
    }

    /// Set the per-layer expanded bridge polygons.
    /// Mirrors [`SliceRegionView::set_bridge_areas`].
    #[must_use]
    pub fn bridge_areas(mut self, areas: Vec<ExPolygon>) -> Self {
        self.bridge_areas = areas;
        self
    }

    /// Set the best bridge direction across all valid bridge regions (degrees).
    /// Mirrors [`SliceRegionView::set_bridge_orientation_deg`].
    #[must_use]
    pub fn bridge_orientation_deg(mut self, deg: f32) -> Self {
        self.bridge_orientation_deg = deg;
        self
    }

    /// Set the host-partitioned sparse-only infill polygon.
    /// Mirrors [`SliceRegionView::set_sparse_infill_area`].
    #[must_use]
    pub fn sparse_infill_area(mut self, polygons: Vec<ExPolygon>) -> Self {
        self.sparse_infill_area = polygons;
        self
    }

    /// Build a [`SliceRegionView`].
    ///
    /// If no infill areas were explicitly added, polygons are cloned
    /// into infill areas for convenience.
    #[must_use]
    pub fn build(self) -> SliceRegionView {
        let infill_areas = if self.infill_areas_explicit {
            self.infill_areas
        } else {
            self.polygons.clone()
        };
        {
            let mut tmp = SliceRegionView::default();
            tmp.set_object_id(self.object_id);
            tmp.set_region_id(self.region_id);
            tmp.set_polygons(self.polygons);
            tmp.set_infill_areas(infill_areas);
            tmp.set_effective_layer_height(self.effective_layer_height);
            tmp.set_z(self.z);
            tmp.set_has_nonplanar(self.has_nonplanar);
            tmp.set_top_shell_index(self.top_shell_index);
            tmp.set_top_solid_fill(self.top_solid_fill);
            tmp.set_bottom_shell_index(self.bottom_shell_index);
            tmp.set_bottom_solid_fill(self.bottom_solid_fill);
            tmp.set_is_bridge(self.is_bridge);
            tmp.set_bridge_areas(self.bridge_areas);
            tmp.set_bridge_orientation_deg(self.bridge_orientation_deg);
            tmp.set_sparse_infill_area(self.sparse_infill_area);
            tmp
        }
    }
}

/// Build a centered square polygon in millimeters.
///
/// Uses [`mm_to_units`] for coordinate scaling.
///
/// # Examples
///
/// ```rust
/// use slicer_sdk::test_support::fixtures::square_polygon;
///
/// let square = square_polygon(0.0, 0.0, 2.0);
/// assert_eq!(square.contour.points.len(), 4);
/// ```
#[must_use]
pub fn square_polygon(cx_mm: f32, cy_mm: f32, side_mm: f32) -> ExPolygon {
    let half = side_mm / 2.0;
    let x0 = mm_to_units(cx_mm - half);
    let y0 = mm_to_units(cy_mm - half);
    let x1 = mm_to_units(cx_mm + half);
    let y1 = mm_to_units(cy_mm + half);

    ExPolygon {
        contour: Polygon {
            points: vec![
                slicer_ir::Point2 { x: x0, y: y0 },
                slicer_ir::Point2 { x: x1, y: y0 },
                slicer_ir::Point2 { x: x1, y: y1 },
                slicer_ir::Point2 { x: x0, y: y1 },
            ],
        },
        holes: Vec::new(),
    }
}

/// Build a centered axis-aligned rectangle in millimeters.
///
/// Mirrors [`square_polygon`] but accepts independent `width_mm` and
/// `height_mm`. Corners are emitted CCW (signed area > 0) with `holes`
/// empty. Uses [`mm_to_units`] for coordinate scaling.
///
/// # Examples
///
/// ```rust
/// use slicer_sdk::test_support::fixtures::rect_polygon;
///
/// let rect = rect_polygon(0.0, 0.0, 4.0, 6.0);
/// assert_eq!(rect.contour.points.len(), 4);
/// assert!(rect.holes.is_empty());
/// ```
#[must_use]
pub fn rect_polygon(cx_mm: f32, cy_mm: f32, width_mm: f32, height_mm: f32) -> ExPolygon {
    let half_w = width_mm / 2.0;
    let half_h = height_mm / 2.0;
    let x0 = mm_to_units(cx_mm - half_w);
    let y0 = mm_to_units(cy_mm - half_h);
    let x1 = mm_to_units(cx_mm + half_w);
    let y1 = mm_to_units(cy_mm + half_h);

    ExPolygon {
        contour: Polygon {
            points: vec![
                slicer_ir::Point2 { x: x0, y: y0 },
                slicer_ir::Point2 { x: x1, y: y0 },
                slicer_ir::Point2 { x: x1, y: y1 },
                slicer_ir::Point2 { x: x0, y: y1 },
            ],
        },
        holes: Vec::new(),
    }
}

/// Build a rectangular [`ExtrusionPath3D`] in millimeters.
///
/// Creates a 4-point rectangle centered at `(cx_mm, cy_mm)` with the given
/// `side_mm` and extrusion `width_mm`. Z is set to 0, role to
/// [`ExtrusionRole::OuterWall`], and speed factor to 1.0.
///
/// # Examples
///
/// ```rust
/// use slicer_sdk::test_support::fixtures::rect_path;
///
/// let path = rect_path(0.0, 0.0, 10.0, 0.4);
/// assert_eq!(path.points.len(), 4);
/// ```
#[must_use]
pub fn rect_path(cx_mm: f32, cy_mm: f32, side_mm: f32, width_mm: f32) -> ExtrusionPath3D {
    let half = side_mm / 2.0;
    let corners = [
        (cx_mm - half, cy_mm - half),
        (cx_mm + half, cy_mm - half),
        (cx_mm + half, cy_mm + half),
        (cx_mm - half, cy_mm + half),
    ];
    let points = corners
        .iter()
        .map(|&(x, y)| Point3WithWidth {
            x,
            y,
            z: 0.0,
            width: width_mm,
            flow_factor: 1.0,
            overhang_quartile: None,
        })
        .collect();
    ExtrusionPath3D {
        points,
        role: ExtrusionRole::OuterWall,
        speed_factor: 1.0,
    }
}

/// Builder for creating [`PerimeterRegionView`] fixtures.
///
/// Produces a read-only `PerimeterRegionView` (from slicer-sdk) suitable for
/// module testing. Outer walls get [`LoopType::Outer`] with `perimeter_index=0`.
/// Inner walls get [`LoopType::Inner`] with auto-incrementing `perimeter_index`
/// starting at 1.
#[derive(Debug, Default)]
pub struct PerimeterRegionViewBuilder {
    object_id: String,
    region_id: u64,
    wall_loops: Vec<WallLoop>,
    infill_areas: Vec<ExPolygon>,
    seam_candidates: Vec<SeamCandidate>,
    next_inner_index: u32,
}

impl PerimeterRegionViewBuilder {
    /// Create a new perimeter region view builder with sensible defaults.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_sdk::test_support::fixtures::PerimeterRegionViewBuilder;
    ///
    /// let _builder = PerimeterRegionViewBuilder::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            object_id: "obj-0".to_string(),
            region_id: 0,
            wall_loops: Vec::new(),
            infill_areas: Vec::new(),
            seam_candidates: Vec::new(),
            next_inner_index: 1,
        }
    }

    /// Set object id.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_sdk::test_support::fixtures::PerimeterRegionViewBuilder;
    ///
    /// let _builder = PerimeterRegionViewBuilder::new().object_id("obj-1");
    /// ```
    #[must_use]
    pub fn object_id(mut self, object_id: impl Into<String>) -> Self {
        self.object_id = object_id.into();
        self
    }

    /// Set region id.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_sdk::test_support::fixtures::PerimeterRegionViewBuilder;
    ///
    /// let _builder = PerimeterRegionViewBuilder::new().region_id(5);
    /// ```
    #[must_use]
    pub fn region_id(mut self, region_id: u64) -> Self {
        self.region_id = region_id;
        self
    }

    /// Add an outer wall from an [`ExtrusionPath3D`].
    ///
    /// Creates a [`WallLoop`] with [`LoopType::Outer`], `perimeter_index=0`,
    /// [`WallBoundaryType::Interior`], and a uniform [`WidthProfile`] derived
    /// from the first point's width.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_sdk::test_support::fixtures::{rect_path, PerimeterRegionViewBuilder};
    ///
    /// let view = PerimeterRegionViewBuilder::new()
    ///     .add_outer_wall(rect_path(0.0, 0.0, 10.0, 0.4))
    ///     .build();
    /// assert_eq!(view.wall_loops().len(), 1);
    /// ```
    #[must_use]
    pub fn add_outer_wall(mut self, path: ExtrusionPath3D) -> Self {
        let width = path.points.first().map_or(0.4, |p| p.width);
        let widths = vec![width; path.points.len()];
        self.wall_loops.push(WallLoop {
            perimeter_index: 0,
            loop_type: LoopType::Outer,
            path,
            width_profile: WidthProfile { widths },
            feature_flags: vec![],
            boundary_type: WallBoundaryType::Interior,
        });
        self
    }

    /// Add an inner wall from an [`ExtrusionPath3D`].
    ///
    /// Creates a [`WallLoop`] with [`LoopType::Inner`] and an auto-incrementing
    /// `perimeter_index` starting at 1.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_sdk::test_support::fixtures::{rect_path, PerimeterRegionViewBuilder};
    ///
    /// let view = PerimeterRegionViewBuilder::new()
    ///     .add_inner_wall(rect_path(0.0, 0.0, 8.0, 0.4))
    ///     .build();
    /// assert_eq!(view.wall_loops().len(), 1);
    /// ```
    #[must_use]
    pub fn add_inner_wall(mut self, path: ExtrusionPath3D) -> Self {
        let width = path.points.first().map_or(0.4, |p| p.width);
        let widths = vec![width; path.points.len()];
        let index = self.next_inner_index;
        self.next_inner_index += 1;
        self.wall_loops.push(WallLoop {
            perimeter_index: index,
            loop_type: LoopType::Inner,
            path,
            width_profile: WidthProfile { widths },
            feature_flags: vec![],
            boundary_type: WallBoundaryType::Interior,
        });
        self
    }

    /// Add a custom [`WallLoop`] directly.
    ///
    /// The loop is added as-is with no modifications.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_ir::{WallLoop, LoopType, WallBoundaryType, WidthProfile};
    /// use slicer_sdk::test_support::fixtures::{rect_path, PerimeterRegionViewBuilder};
    ///
    /// let wl = WallLoop {
    ///     perimeter_index: 0,
    ///     loop_type: LoopType::ThinWall,
    ///     path: rect_path(0.0, 0.0, 4.0, 0.3),
    ///     width_profile: WidthProfile { widths: vec![0.3; 4] },
    ///     feature_flags: vec![],
    ///     boundary_type: WallBoundaryType::Interior,
    /// };
    /// let _view = PerimeterRegionViewBuilder::new().add_wall_loop(wl).build();
    /// ```
    #[must_use]
    pub fn add_wall_loop(mut self, wall_loop: WallLoop) -> Self {
        self.wall_loops.push(wall_loop);
        self
    }

    /// Add an infill area polygon.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_sdk::test_support::fixtures::{square_polygon, PerimeterRegionViewBuilder};
    ///
    /// let view = PerimeterRegionViewBuilder::new()
    ///     .add_infill_area(square_polygon(0.0, 0.0, 10.0))
    ///     .build();
    /// assert_eq!(view.infill_areas().len(), 1);
    /// ```
    #[must_use]
    pub fn add_infill_area(mut self, polygon: ExPolygon) -> Self {
        self.infill_areas.push(polygon);
        self
    }

    /// Add a seam candidate.
    #[must_use]
    pub fn add_seam_candidate(mut self, candidate: SeamCandidate) -> Self {
        self.seam_candidates.push(candidate);
        self
    }

    /// Build a [`PerimeterRegionView`].
    #[must_use]
    pub fn build(self) -> PerimeterRegionView {
        let mut view = PerimeterRegionView::default();
        view.set_object_id(self.object_id);
        view.set_region_id(self.region_id);
        view.set_wall_loops(self.wall_loops);
        view.set_infill_areas(self.infill_areas);
        view.set_seam_candidates(self.seam_candidates);
        view.set_resolved_seam(None);
        view
    }

    /// Add an outer wall with explicit `feature_flags` and `boundary_type`.
    ///
    /// Mirrors [`add_outer_wall`](Self::add_outer_wall) but lets callers thread
    /// per-vertex [`WallFeatureFlags`] and a non-default [`WallBoundaryType`]
    /// (e.g., [`WallBoundaryType::ExteriorSurface`]) — required by the
    /// seam-placer module's `wall_at_z` shape.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_ir::{WallBoundaryType, WallFeatureFlags};
    /// use slicer_sdk::test_support::fixtures::{rect_path, PerimeterRegionViewBuilder};
    ///
    /// let path = rect_path(0.0, 0.0, 10.0, 0.4);
    /// let flags = vec![WallFeatureFlags::default(); path.points.len()];
    /// let view = PerimeterRegionViewBuilder::new()
    ///     .add_outer_wall_with_flags(path, flags, WallBoundaryType::ExteriorSurface)
    ///     .build();
    /// assert_eq!(view.wall_loops().len(), 1);
    /// ```
    #[must_use]
    pub fn add_outer_wall_with_flags(
        mut self,
        path: ExtrusionPath3D,
        feature_flags: Vec<WallFeatureFlags>,
        boundary_type: WallBoundaryType,
    ) -> Self {
        let width = path.points.first().map_or(0.4, |p| p.width);
        let widths = vec![width; path.points.len()];
        self.wall_loops.push(WallLoop {
            perimeter_index: 0,
            loop_type: LoopType::Outer,
            path,
            width_profile: WidthProfile { widths },
            feature_flags,
            boundary_type,
        });
        self
    }
}

// ============================================================================
// Freestanding IR fixture helpers
// ============================================================================

/// Build a [`PrintEntity`] from explicit identity, role, geometry, and ordering
/// inputs.
///
/// Constructs the inner [`ExtrusionPath3D`] from `points` and `role` with
/// `speed_factor = 1.0`. The returned `PrintEntity` carries the inputs verbatim
/// (entity id, role, region key, topo order). Names are passed via named-struct
/// construction to keep the signature stable as new IR fields land.
///
/// `tool_index` defaults to the base tool `0`; `region_id` is a pure region
/// identity post region_id↔tool split and is never read as the tool selector
/// (D-125-TOOL-IDENTITY-SPLIT invariant). A fixture needing a specific tool
/// should construct [`PrintEntity`] directly.
///
/// # Examples
///
/// ```rust
/// use slicer_ir::{ExtrusionRole, Point3WithWidth, RegionKey};
/// use slicer_sdk::test_support::fixtures::print_entity;
///
/// let points = vec![Point3WithWidth {
///     x: 0.0,
///     y: 0.0,
///     z: 0.2,
///     width: 0.4,
///     flow_factor: 1.0,
///     overhang_quartile: None,
/// }];
/// let entity = print_entity(
///     1,
///     ExtrusionRole::OuterWall,
///     points,
///     RegionKey::default(),
///     0,
/// );
/// assert_eq!(entity.entity_id, 1);
/// assert_eq!(entity.topo_order, 0);
/// ```
#[must_use]
pub fn print_entity(
    entity_id: u64,
    role: ExtrusionRole,
    points: Vec<Point3WithWidth>,
    region_key: RegionKey,
    topo_order: u32,
) -> PrintEntity {
    PrintEntity {
        entity_id,
        path: ExtrusionPath3D {
            points,
            role: role.clone(),
            speed_factor: 1.0,
        },
        role,
        tool_index: 0,
        region_key,
        topo_order,
    }
}

/// Build a [`ToolChange`] anchored after the entity at `after_entity_index`
/// targeting `tool_index`.
///
/// Uses named-struct construction so the helper survives additions of new
/// optional `ToolChange` fields. Callers pass the previous tool (`from_tool`)
/// and target tool (`to_tool`) explicitly — wipe-tower's multi-tool layer
/// fixtures need non-zero `from_tool` (e.g. transitioning T1 → T0 on layer 1),
/// which the original single-tool signature could not express.
///
/// # Examples
///
/// ```rust
/// use slicer_sdk::test_support::fixtures::tool_change;
///
/// let tc = tool_change(3, 0, 2);
/// assert_eq!(tc.after_entity_index, 3);
/// assert_eq!(tc.from_tool, 0);
/// assert_eq!(tc.to_tool, 2);
/// ```
#[must_use]
pub fn tool_change(
    after_entity_index: u32,
    from_tool: u32,
    to_tool: u32,
) -> ToolChange {
    ToolChange {
        after_entity_index,
        from_tool,
        to_tool,
    }
}

/// Build a [`SeamCandidate`] with explicit `position`, `score`, and `reason`.
///
/// Uses named-struct construction so the helper survives additions of new
/// optional `SeamCandidate` fields.
///
/// # Examples
///
/// ```rust
/// use slicer_ir::{Point3WithWidth, SeamReason};
/// use slicer_sdk::test_support::fixtures::seam_candidate;
///
/// let pos = Point3WithWidth {
///     x: 1.0,
///     y: 2.0,
///     z: 0.2,
///     width: 0.4,
///     flow_factor: 1.0,
///     overhang_quartile: None,
/// };
/// let sc = seam_candidate(pos, 0.5, SeamReason::Sharp);
/// assert!((sc.score - 0.5).abs() < f32::EPSILON);
/// ```
#[must_use]
pub fn seam_candidate(
    position: Point3WithWidth,
    score: f32,
    reason: SeamReason,
) -> SeamCandidate {
    SeamCandidate {
        position,
        score,
        reason,
    }
}

/// Builder for assembling [`LayerCollectionIR`] fixtures with entities and
/// tool changes.
///
/// Distinct from the production
/// [`crate::layer_collection_builder::LayerCollectionBuilder`] (a WIT-resource
/// proposal builder used at runtime); this is a test-only IR assembler that
/// lets callers stage entities, tool changes, and basic header fields into a
/// concrete [`LayerCollectionIR`].
#[derive(Debug, Default)]
pub struct LayerCollectionFixtureBuilder {
    global_layer_index: u32,
    z: f32,
    entities: Vec<PrintEntity>,
    tool_changes: Vec<ToolChange>,
}

impl LayerCollectionFixtureBuilder {
    /// Create a new fixture builder with all fields defaulted.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_sdk::test_support::fixtures::LayerCollectionFixtureBuilder;
    ///
    /// let _builder = LayerCollectionFixtureBuilder::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the global layer index for the assembled IR.
    #[must_use]
    pub fn global_layer_index(mut self, idx: u32) -> Self {
        self.global_layer_index = idx;
        self
    }

    /// Set the Z height (millimeters) for the assembled IR.
    #[must_use]
    pub fn z(mut self, z: f32) -> Self {
        self.z = z;
        self
    }

    /// Append a [`PrintEntity`] to the layer's ordered entities.
    #[must_use]
    pub fn add_entity(mut self, e: PrintEntity) -> Self {
        self.entities.push(e);
        self
    }

    /// Append a [`ToolChange`] to the layer's tool changes.
    #[must_use]
    pub fn add_tool_change(mut self, tc: ToolChange) -> Self {
        self.tool_changes.push(tc);
        self
    }

    /// Build a [`LayerCollectionIR`].
    ///
    /// Threads the four staged fields onto the IR; remaining fields
    /// (schema version, z hops, annotations, retracts, travel moves) come from
    /// [`LayerCollectionIR::default`].
    #[must_use]
    pub fn build(self) -> LayerCollectionIR {
        LayerCollectionIR {
            global_layer_index: self.global_layer_index,
            z: self.z,
            ordered_entities: self.entities,
            tool_changes: self.tool_changes,
            ..Default::default()
        }
    }
}

/// Build a [`ConfigView`] from a slice of `(key, value)` pairs.
///
/// Convenience over [`ConfigViewBuilder`] for the common case of a flat,
/// pre-typed key set. Replaces the per-module `config_with` / `config_view`
/// helpers that were duplicated across module test files.
///
/// # Examples
///
/// ```rust
/// use slicer_ir::ConfigValue;
/// use slicer_sdk::test_support::fixtures::config_with;
///
/// let cfg = config_with(&[
///     ("fan_speed_max", ConfigValue::Int(255)),
///     ("enable_overhang_fan", ConfigValue::Bool(false)),
/// ]);
/// assert_eq!(cfg.len(), 2);
/// ```
#[must_use]
pub fn config_with(pairs: &[(&str, ConfigValue)]) -> ConfigView {
    let mut fields = HashMap::new();
    for (k, v) in pairs {
        fields.insert((*k).to_string(), v.clone());
    }
    ConfigView::from_map(fields)
}

/// Build a minimal [`SliceRegionView`] carrying only `object_id` and
/// `region_id`, with all geometry empty.
///
/// Removes the repeated 5–7 setter boilerplate when a test needs a region that
/// is present but carries no polygons (e.g. negative / gating assertions). Use
/// [`SliceRegionViewBuilder`] directly when geometry is needed.
///
/// # Examples
///
/// ```rust
/// use slicer_sdk::test_support::fixtures::empty_region;
///
/// let region = empty_region("obj-1", 0);
/// assert!(region.top_solid_fill().is_empty());
/// ```
#[must_use]
pub fn empty_region(object_id: &str, region_id: u64) -> SliceRegionView {
    SliceRegionViewBuilder::new()
        .object_id(object_id)
        .region_id(region_id)
        .build()
}

/// Build a minimal [`PerimeterRegionView`] carrying only `object_id` and
/// `region_id`, with no walls, infill areas, or seam candidates.
///
/// # Examples
///
/// ```rust
/// use slicer_sdk::test_support::fixtures::empty_perimeter_region;
///
/// let region = empty_perimeter_region("obj-1", 0);
/// assert!(region.wall_loops().is_empty());
/// ```
#[must_use]
pub fn empty_perimeter_region(object_id: &str, region_id: u64) -> PerimeterRegionView {
    PerimeterRegionViewBuilder::new()
        .object_id(object_id)
        .region_id(region_id)
        .build()
}
