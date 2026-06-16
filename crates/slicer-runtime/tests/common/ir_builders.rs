//! IR builder helpers for test fixtures.
//!
//! Provides fluent constructors for `SliceIR`, `PerimeterIR`, and `WallLoop`
//! with deterministic synthetic data. Two entry points per IR type:
//! `with_count(n)` for cardinality-focused tests and `with_ids(ids)` for
//! identity-preservation tests.

use slicer_ir::WallLoop;

// ── slice_ir module ──────────────────────────────────────────────────────────

pub mod slice_ir {
    use slicer_ir::{
        ExPolygon, Point2, Polygon, SliceIR, SlicedRegion, CURRENT_SLICE_IR_SCHEMA_VERSION,
    };

    /// Entry point: build a `SliceIR` with `n` synthetic regions (`obj-0` .. `obj-{n-1}`).
    pub fn with_count(n: usize) -> SliceIrBuilder {
        let ids: Vec<(String, u64)> = (0..n).map(|i| (format!("obj-{i}"), i as u64)).collect();
        SliceIrBuilder {
            ids,
            z: 0.2,
            layer_index: 0,
        }
    }

    /// Entry point: build a `SliceIR` with explicitly named `(object_id, region_id)` pairs.
    pub fn with_ids(ids: &[(&str, u64)]) -> SliceIrBuilder {
        let ids: Vec<(String, u64)> = ids.iter().map(|(o, r)| (o.to_string(), *r)).collect();
        SliceIrBuilder {
            ids,
            z: 0.2,
            layer_index: 0,
        }
    }

    /// Builder for `SliceIR`.
    pub struct SliceIrBuilder {
        ids: Vec<(String, u64)>,
        z: f32,
        layer_index: u32,
    }

    impl SliceIrBuilder {
        /// Set the Z height (default `0.2`).
        pub fn at_z(mut self, z: f32) -> Self {
            self.z = z;
            self
        }

        /// Set the global layer index (default `0`).
        pub fn at_layer(mut self, idx: u32) -> Self {
            self.layer_index = idx;
            self
        }

        /// Build the `SliceIR`.
        pub fn build(self) -> SliceIR {
            let regions = self
                .ids
                .into_iter()
                .map(|(object_id, region_id)| SlicedRegion {
                    object_id,
                    region_id,
                    polygons: vec![ExPolygon {
                        contour: Polygon {
                            points: vec![
                                Point2 { x: 0, y: 0 },
                                Point2 { x: 10_000, y: 0 },
                                Point2 {
                                    x: 10_000,
                                    y: 10_000,
                                },
                                Point2 { x: 0, y: 10_000 },
                            ],
                        },
                        holes: Vec::new(),
                    }],
                    effective_layer_height: self.z,
                    ..Default::default()
                })
                .collect();
            SliceIR {
                schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
                global_layer_index: self.layer_index,
                z: self.z,
                regions,
            }
        }
    }
}

// ── perimeter_ir module ──────────────────────────────────────

pub mod perimeter_ir {
    use super::make_wall_loop;
    use slicer_ir::{
        ExPolygon, PerimeterIR, PerimeterRegion, Point2, Polygon, WallLoop,
        CURRENT_PERIMETER_IR_SCHEMA_VERSION,
    };

    /// Entry point: build a `PerimeterIR` with `n` synthetic regions (`obj-0` .. `obj-{n-1}`).
    pub fn with_count(n: usize) -> PerimeterIrBuilder {
        let ids: Vec<(String, u64)> = (0..n).map(|i| (format!("obj-{i}"), i as u64)).collect();
        PerimeterIrBuilder {
            ids,
            layer_index: 0,
            walls_per_region: 0,
            infill_polys: 0,
            custom_walls: None,
        }
    }

    /// Entry point: build a `PerimeterIR` with explicitly named `(object_id, region_id)` pairs.
    pub fn with_ids(ids: &[(&str, u64)]) -> PerimeterIrBuilder {
        let ids: Vec<(String, u64)> = ids.iter().map(|(o, r)| (o.to_string(), *r)).collect();
        PerimeterIrBuilder {
            ids,
            layer_index: 0,
            walls_per_region: 0,
            infill_polys: 0,
            custom_walls: None,
        }
    }

    /// Builder for `PerimeterIR`.
    pub struct PerimeterIrBuilder {
        ids: Vec<(String, u64)>,
        layer_index: u32,
        walls_per_region: u32,
        infill_polys: usize,
        custom_walls: Option<Vec<WallLoop>>,
    }

    impl PerimeterIrBuilder {
        /// Set the global layer index (default `0`).
        pub fn at_layer(mut self, idx: u32) -> Self {
            self.layer_index = idx;
            self
        }

        /// Set the number of synthetic wall loops per region (default `0`).
        pub fn walls(mut self, n: u32) -> Self {
            self.walls_per_region = n;
            self
        }

        /// Provide custom `WallLoop` values for each region (overrides `.walls()`).
        pub fn walls_with(mut self, w: Vec<WallLoop>) -> Self {
            self.custom_walls = Some(w);
            self
        }

        /// Set the number of synthetic infill polygons per region (default `0`).
        pub fn infill(mut self, n: usize) -> Self {
            self.infill_polys = n;
            self
        }

        /// Build the `PerimeterIR`.
        pub fn build(self) -> PerimeterIR {
            let wall_z = if self.layer_index == 0 {
                0.2
            } else {
                self.layer_index as f32 * 0.2
            };
            let regions = self
                .ids
                .into_iter()
                .map(|(object_id, region_id)| {
                    let walls = if let Some(ref cw) = self.custom_walls {
                        cw.clone()
                    } else {
                        (0..self.walls_per_region)
                            .map(|w| make_wall_loop(w, wall_z))
                            .collect()
                    };
                    let infill_areas = (0..self.infill_polys)
                        .map(|_| ExPolygon {
                            contour: Polygon {
                                points: vec![
                                    Point2 { x: 0, y: 0 },
                                    Point2 { x: 1000, y: 0 },
                                    Point2 { x: 1000, y: 1000 },
                                ],
                            },
                            holes: Vec::new(),
                        })
                        .collect();
                    PerimeterRegion {
                        object_id,
                        region_id,
                        walls,
                        infill_areas,
                        seam_candidates: Vec::new(),
                        resolved_seam: None,
                    }
                })
                .collect();
            PerimeterIR {
                schema_version: CURRENT_PERIMETER_IR_SCHEMA_VERSION,
                global_layer_index: self.layer_index,
                regions,
            }
        }
    }
}

// ── wall_loop ─────────────────────────────────────────────────────────────────

/// Build a synthetic `WallLoop` with `point_count` points at the given Z.
pub fn wall_loop() -> WallLoopBuilder {
    WallLoopBuilder {
        perimeter_index: 0,
        point_count: 2,
        z: 0.2,
    }
}

/// Builder for `WallLoop`.
pub struct WallLoopBuilder {
    perimeter_index: u32,
    point_count: usize,
    z: f32,
}

impl WallLoopBuilder {
    /// Set the perimeter index (default `0`).
    pub fn outer(mut self) -> Self {
        self.perimeter_index = 0;
        self
    }

    /// Set the point count (default `2`).
    pub fn points(mut self, n: usize) -> Self {
        self.point_count = n;
        self
    }

    /// Set the Z height (default `0.2`).
    pub fn at_z(mut self, z: f32) -> Self {
        self.z = z;
        self
    }

    /// Build the `WallLoop`.
    pub fn build(self) -> WallLoop {
        make_wall_loop_impl(self.perimeter_index, self.z, self.point_count)
    }

    /// Set the loop type to Inner.
    pub fn inner(mut self) -> Self {
        self.perimeter_index = 1;
        self
    }
}

// ── Internal helpers ──────────────────────────────────────────────────────────

fn make_wall_loop(perimeter_index: u32, z: f32) -> WallLoop {
    make_wall_loop_impl(perimeter_index, z, 2)
}

fn make_wall_loop_impl(perimeter_index: u32, z: f32, point_count: usize) -> WallLoop {
    use slicer_ir::{
        ExtrusionPath3D, ExtrusionRole, LoopType, Point3WithWidth, WallBoundaryType,
        WallFeatureFlags, WallLoop, WidthProfile,
    };

    let points: Vec<Point3WithWidth> = (0..point_count)
        .map(|i| Point3WithWidth {
            x: i as f32,
            y: 0.0,
            z,
            width: 0.4,
            flow_factor: 1.0,
            overhang_quartile: None,
        })
        .collect();
    let feature_flags: Vec<WallFeatureFlags> = (0..point_count)
        .map(|_| WallFeatureFlags {
            tool_index: None,
            fuzzy_skin: false,
            is_bridge: false,
            is_thin_wall: false,
            skip_ironing: false,
            custom: std::collections::HashMap::new(),
        })
        .collect();

    WallLoop {
        perimeter_index,
        loop_type: LoopType::Outer,
        path: ExtrusionPath3D {
            points: points.clone(),
            role: ExtrusionRole::OuterWall,
            speed_factor: 1.0,
        },
        width_profile: WidthProfile {
            widths: points.iter().map(|p| p.width).collect(),
        },
        feature_flags,
        boundary_type: WallBoundaryType::Interior,
    }
}
