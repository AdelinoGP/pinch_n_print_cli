//! IR fixture builders for tests.

use std::collections::HashMap;

use slicer_ir::{mm_to_units, ConfigValue, ConfigView, ExPolygon, Polygon};
use slicer_sdk::views::SliceRegionView;

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
    /// use slicer_test::fixtures::ConfigViewBuilder;
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
    /// use slicer_test::fixtures::ConfigViewBuilder;
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
    /// use slicer_test::fixtures::ConfigViewBuilder;
    ///
    /// let _config = ConfigViewBuilder::new().float("density", 0.2).build();
    /// ```
    #[must_use]
    pub fn float(mut self, key: impl Into<String>, value: f64) -> Self {
        self.fields.insert(key.into(), ConfigValue::Float(value));
        self
    }

    /// Add a string key/value pair.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_test::fixtures::ConfigViewBuilder;
    ///
    /// let _config = ConfigViewBuilder::new().string("pattern", "grid").build();
    /// ```
    #[must_use]
    pub fn string(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.fields
            .insert(key.into(), ConfigValue::String(value.into()));
        self
    }

    /// Build a [`ConfigView`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_test::fixtures::ConfigViewBuilder;
    ///
    /// let config = ConfigViewBuilder::new().int("count", 1).build();
    /// assert_eq!(config.fields.len(), 1);
    /// ```
    #[must_use]
    pub fn build(self) -> ConfigView {
        ConfigView {
            fields: self.fields,
        }
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
}

impl SliceRegionViewBuilder {
    /// Create a new slice region view builder with sensible defaults.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_test::fixtures::SliceRegionViewBuilder;
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
        }
    }

    /// Set object id.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_test::fixtures::SliceRegionViewBuilder;
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
    /// use slicer_test::fixtures::SliceRegionViewBuilder;
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
    /// use slicer_test::fixtures::SliceRegionViewBuilder;
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
    /// use slicer_test::fixtures::SliceRegionViewBuilder;
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
    /// use slicer_test::fixtures::SliceRegionViewBuilder;
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
    /// use slicer_test::fixtures::{square_polygon, SliceRegionViewBuilder};
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
    /// use slicer_test::fixtures::{square_polygon, SliceRegionViewBuilder};
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
        SliceRegionView::new(
            self.object_id,
            self.region_id,
            self.polygons,
            infill_areas,
            self.effective_layer_height,
            self.z,
            self.has_nonplanar,
        )
    }
}

/// Build a centered square polygon in millimeters.
///
/// Uses [`mm_to_units`] for coordinate scaling.
///
/// # Examples
///
/// ```rust
/// use slicer_test::fixtures::square_polygon;
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
