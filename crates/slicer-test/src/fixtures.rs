//! IR fixture builders for tests.

use std::collections::HashMap;

use slicer_ir::{mm_to_units, ConfigValue, ConfigView, ExPolygon, Polygon, SlicedRegion};

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

/// Minimal builder for [`SlicedRegion`] fixtures.
#[derive(Debug, Default)]
pub struct SliceRegionViewBuilder {
    object_id: String,
    region_id: u64,
    effective_layer_height: f32,
    polygons: Vec<ExPolygon>,
}

impl SliceRegionViewBuilder {
    /// Create a new slice region builder.
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
            effective_layer_height: 0.2,
            polygons: Vec::new(),
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

    /// Add one polygon to both region polygon collections.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_test::fixtures::{square_polygon, SliceRegionViewBuilder};
    ///
    /// let _region = SliceRegionViewBuilder::new()
    ///     .add_polygon(square_polygon(0.0, 0.0, 10.0))
    ///     .build();
    /// ```
    #[must_use]
    pub fn add_polygon(mut self, polygon: ExPolygon) -> Self {
        self.polygons.push(polygon);
        self
    }

    /// Build a [`SlicedRegion`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_test::fixtures::{square_polygon, SliceRegionViewBuilder};
    ///
    /// let region = SliceRegionViewBuilder::new()
    ///     .add_polygon(square_polygon(0.0, 0.0, 4.0))
    ///     .build();
    /// assert_eq!(region.polygons.len(), 1);
    /// ```
    #[must_use]
    pub fn build(self) -> SlicedRegion {
        let polygons = self.polygons;
        SlicedRegion {
            object_id: self.object_id,
            region_id: self.region_id,
            polygons: polygons.clone(),
            infill_areas: polygons,
            nonplanar_surface: None,
            effective_layer_height: self.effective_layer_height,
            boundary_paint: HashMap::new(),
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
