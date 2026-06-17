// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/GCode/WipeTower.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the ModularSlicer architecture.
// -----------------------------------------------------------------------------
//! Wipe tower module for multi-material tool change purge/prime paths.
//!
//! Runs in the `PostPass::LayerFinalization` stage, operating on the full
//! set of `LayerCollectionIR` outputs after per-layer processing completes.
//! For each tool change, generates rectilinear purge scan lines within a
//! configurable rectangular region.

#![warn(missing_docs)]
#![warn(unused_imports)]

use slicer_ir::{
    ConfigValue, ConfigView, ExtrusionPath3D, ExtrusionRole, LayerCollectionIR, Point3WithWidth,
    PrintEntity, RegionKey,
};
use slicer_sdk::error::ModuleError;
use slicer_sdk::slicer_module;
use slicer_sdk::traits::{FinalizationModule, FinalizationOutputBuilder, LayerCollectionView};

/// Default layer height used when layer height cannot be inferred from
/// adjacent layers.
const DEFAULT_LAYER_HEIGHT: f32 = 0.2;

/// Wipe tower purge/prime path generator.
///
/// Generates rectangular rectilinear scan-line purge extrusions at each
/// tool change location across all layers.
pub struct WipeTower {
    tower_x: f32,
    tower_y: f32,
    tower_width: f32,
    purge_volume: f32,
    line_width: f32,
    enabled: bool,
    retract_length: f32,
    bed_shape: Vec<(f32, f32)>,
}

/// Parse a flat `[x0, y0, x1, y1, …]` float list into `(x, y)` vertex pairs.
///
/// Returns `Err` if the list is empty, has odd length, or has fewer than 6
/// values (i.e. fewer than 3 vertices — not a polygon).
fn parse_bed_shape(raw: &[f64]) -> Result<Vec<(f32, f32)>, ModuleError> {
    if raw.is_empty() {
        return Err(ModuleError::fatal(
            2,
            "bed_shape config is empty; expected at least 6 values [x0,y0,x1,y1,x2,y2]",
        ));
    }
    if !raw.len().is_multiple_of(2) {
        return Err(ModuleError::fatal(
            2,
            format!(
                "bed_shape has odd length {}; must be even (interleaved x,y pairs)",
                raw.len()
            ),
        ));
    }
    if raw.len() < 6 {
        return Err(ModuleError::fatal(
            2,
            format!(
                "bed_shape has only {} values; need at least 6 for a 3-vertex polygon",
                raw.len()
            ),
        ));
    }
    Ok(raw.chunks(2).map(|c| (c[0] as f32, c[1] as f32)).collect())
}

/// Point-in-polygon test using ray casting (even-odd rule).
///
/// Returns `true` if `(px, py)` is strictly inside or on the boundary of the polygon.
fn point_in_polygon(px: f32, py: f32, polygon: &[(f32, f32)]) -> bool {
    let n = polygon.len();
    if n < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let (xi, yi) = polygon[i];
        let (xj, yj) = polygon[j];
        if ((yi > py) != (yj > py)) && (px < (xj - xi) * (py - yi) / (yj - yi) + xi) {
            inside = !inside;
        }
        // On-edge: if the point lies on the segment, treat as inside.
        let on_edge = {
            let cross = (xj - xi) * (py - yi) - (yj - yi) * (px - xi);
            let dot = (px - xi) * (xj - xi) + (py - yi) * (yj - yi);
            let len2 = (xj - xi) * (xj - xi) + (yj - yi) * (yj - yi);
            cross.abs() < 1e-4 && dot >= 0.0 && dot <= len2
        };
        if on_edge {
            return true;
        }
        j = i;
    }
    inside
}

impl WipeTower {
    /// Construct from a config view, reading wipe tower settings with defaults.
    pub fn from_config(config: &ConfigView) -> Result<Self, ModuleError> {
        let enabled = match config.get("wipe_tower_enabled") {
            Some(ConfigValue::Bool(b)) => *b,
            _ => false,
        };

        let tower_x = match config.get("wipe_tower_x") {
            Some(ConfigValue::Float(v)) => *v as f32,
            _ => 0.0,
        };

        let tower_y = match config.get("wipe_tower_y") {
            Some(ConfigValue::Float(v)) => *v as f32,
            _ => 0.0,
        };

        let tower_width = match config.get("wipe_tower_width") {
            Some(ConfigValue::Float(v)) => *v as f32,
            _ => 60.0,
        };

        let purge_volume = match config.get("wipe_tower_purge_volume") {
            Some(ConfigValue::Float(v)) => *v as f32,
            _ => 70.0,
        };

        let line_width = match config.get("line_width") {
            Some(ConfigValue::Float(v)) => *v as f32,
            _ => 0.4,
        };

        let retract_length = match config.get("retract_length") {
            Some(ConfigValue::Float(v)) => *v as f32,
            _ => 2.0,
        };

        // Parse bed_shape from config (interleaved [x0,y0,x1,y1,...]).
        // Default to a 250×250 mm rectangle if not provided.
        let bed_shape = match config.get("bed_shape") {
            Some(ConfigValue::List(items)) => {
                let raw: Vec<f64> = items
                    .iter()
                    .filter_map(|v| match v {
                        ConfigValue::Float(f) => Some(*f),
                        ConfigValue::Int(i) => Some(*i as f64),
                        _ => None,
                    })
                    .collect();
                if raw.len() >= 6 && raw.len().is_multiple_of(2) {
                    parse_bed_shape(&raw).unwrap_or_else(|_| {
                        vec![(0.0, 0.0), (250.0, 0.0), (250.0, 250.0), (0.0, 250.0)]
                    })
                } else {
                    vec![(0.0, 0.0), (250.0, 0.0), (250.0, 250.0), (0.0, 250.0)]
                }
            }
            _ => vec![(0.0, 0.0), (250.0, 0.0), (250.0, 250.0), (0.0, 250.0)],
        };

        Ok(Self {
            tower_x,
            tower_y,
            tower_width,
            purge_volume,
            line_width,
            enabled,
            retract_length,
            bed_shape,
        })
    }

    /// Process all layers, inserting wipe tower purge paths at tool changes.
    ///
    /// If the tower is disabled, returns immediately without modification.
    #[allow(clippy::ptr_arg)]
    pub fn process(&self, layers: &mut Vec<LayerCollectionIR>) -> Result<(), ModuleError> {
        if !self.enabled {
            return Ok(());
        }

        for layer_idx in 0..layers.len() {
            if layers[layer_idx].tool_changes.is_empty() {
                continue;
            }

            let z = layers[layer_idx].z;

            // Estimate layer height from adjacent layers
            let layer_height = if layer_idx > 0 {
                let dz = z - layers[layer_idx - 1].z;
                if dz > 0.0 {
                    dz
                } else {
                    DEFAULT_LAYER_HEIGHT
                }
            } else {
                DEFAULT_LAYER_HEIGHT
            };

            // Clone tool_changes so we don't borrow layers while mutating
            let tool_changes = layers[layer_idx].tool_changes.clone();

            let global_layer_index = layers[layer_idx].global_layer_index;
            for tc in &tool_changes {
                let pairs = self.generate_purge_paths(z, layer_height, global_layer_index, tc);
                for (path, region_key) in pairs {
                    let role = path.role.clone();
                    // TODO(packet-41/DEV-047): retire this legacy `process()` path;
                    // live path is `run_finalization` which routes through
                    // `push_entity_with_priority(..., WipeTower.default_priority())`.
                    layers[layer_idx].ordered_entities.push(PrintEntity {
                        entity_id: 0,
                        path,
                        role,
                        region_key,
                        topo_order: 0,
                    });
                }
            }
        }

        Ok(())
    }

    /// Generate purge paths for a single tool change.
    ///
    /// Returns `(ExtrusionPath3D, RegionKey)` pairs in the order:
    /// 1. Travel-to-tower entity (zero E)
    /// 2. Rectilinear scan-line wall entities
    /// 3. Prime entity (positive E equal to purge volume)
    ///
    /// The retract that physically must precede `T<n>` is synthesized
    /// host-side in `crates/slicer-runtime/src/gcode_emit.rs` because
    /// `insert_entity_at` positions module entities AFTER the tool-change
    /// reference index, while a correct retract must come BEFORE it. The host
    /// emitter consults `resolved_config.retract_length` for the negative-E
    /// amount; this module's `retract_length` field is retained for future
    /// builder primitives that can place a real `TravelRetract` from the
    /// module side (see DEV-054 follow-up (i)).
    ///
    /// The `tc` parameter is used to contextualise which tool change this purge serves.
    fn generate_purge_paths(
        &self,
        z: f32,
        layer_height: f32,
        global_layer_index: u32,
        _tc: &slicer_ir::ToolChange,
    ) -> Vec<(ExtrusionPath3D, RegionKey)> {
        let cross_section = self.line_width * layer_height * self.tower_width;
        if cross_section <= 0.0 {
            return Vec::new();
        }

        let region_key = RegionKey {
            global_layer_index,
            object_id: "__wipe_tower__".to_string(),
            region_id: 0,
            variant_chain: Vec::new(),
        };

        let mut pairs: Vec<(ExtrusionPath3D, RegionKey)> = Vec::new();

        // ── 1. Travel-to-tower entity ────────────────────────────────────────
        // A zero-E move to the tower start position (flow_factor = 0.0).
        let travel_path = ExtrusionPath3D {
            points: vec![
                Point3WithWidth {
                    x: self.tower_x,
                    y: self.tower_y,
                    z,
                    width: self.line_width,
                    flow_factor: 0.0,
                    overhang_quartile: None,
                },
                Point3WithWidth {
                    x: self.tower_x,
                    y: self.tower_y,
                    z,
                    width: self.line_width,
                    flow_factor: 0.0,
                    overhang_quartile: None,
                },
            ],
            role: ExtrusionRole::WipeTower,
            speed_factor: 1.0,
        };
        pairs.push((travel_path, region_key.clone()));

        // ── 2. Rectilinear scan-line wall entities ───────────────────────────
        let purge_depth = self.purge_volume / cross_section;
        let x_min = self.tower_x;
        let x_max = self.tower_x + self.tower_width;
        let y_min = self.tower_y;
        let y_max = self.tower_y + purge_depth;

        let mut y = y_min + self.line_width / 2.0;
        let mut forward = true;

        while y < y_max {
            let (start_x, end_x) = if forward {
                (x_min, x_max)
            } else {
                (x_max, x_min)
            };

            let path = ExtrusionPath3D {
                points: vec![
                    Point3WithWidth {
                        x: start_x,
                        y,
                        z,
                        width: self.line_width,
                        flow_factor: 1.0,
                        overhang_quartile: None,
                    },
                    Point3WithWidth {
                        x: end_x,
                        y,
                        z,
                        width: self.line_width,
                        flow_factor: 1.0,
                        overhang_quartile: None,
                    },
                ],
                role: ExtrusionRole::WipeTower,
                speed_factor: 1.0,
            };

            pairs.push((path, region_key.clone()));

            forward = !forward;
            y += self.line_width;
        }

        // ── 3. Prime entity ──────────────────────────────────────────────────
        // A single straight-line entity that fits within the tower width, whose
        // cumulative positive E delta contributes to the purge volume budget.
        // The path is capped at tower_width to stay within the bed footprint.
        // E = length * line_width * flow; length = purge_volume / (line_width * layer_height),
        // but capped at tower_width so the geometry stays within the tower rectangle.
        let prime_length_full = if layer_height > 0.0 {
            self.purge_volume / (self.line_width * layer_height)
        } else {
            0.0
        };
        // Clamp to tower width so prime entity stays within the tower footprint.
        let prime_length = prime_length_full.min(self.tower_width);
        let prime_path = ExtrusionPath3D {
            points: vec![
                Point3WithWidth {
                    x: self.tower_x,
                    y: self.tower_y,
                    z,
                    width: self.line_width,
                    flow_factor: 0.0, // first point: no extrusion
                    overhang_quartile: None,
                },
                Point3WithWidth {
                    x: self.tower_x + prime_length,
                    y: self.tower_y,
                    z,
                    width: self.line_width,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                },
            ],
            role: ExtrusionRole::WipeTower,
            speed_factor: 1.0,
        };
        pairs.push((prime_path, region_key.clone()));

        pairs
    }

    /// Whether the wipe tower is enabled.
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// Tower X position in mm.
    pub fn tower_x(&self) -> f32 {
        self.tower_x
    }

    /// Tower Y position in mm.
    pub fn tower_y(&self) -> f32 {
        self.tower_y
    }

    /// Tower width in mm.
    pub fn tower_width(&self) -> f32 {
        self.tower_width
    }

    /// Purge volume in mm^3.
    pub fn purge_volume(&self) -> f32 {
        self.purge_volume
    }

    /// Line width in mm.
    pub fn line_width(&self) -> f32 {
        self.line_width
    }

    /// Retract length in mm.
    pub fn retract_length(&self) -> f32 {
        self.retract_length
    }
}

// ── SDK authoring-path adoption (TASK-111 / packet-17) ─────────────────
//
// `on_print_start` delegates to the existing `from_config` constructor.
// `run_finalization` uses `insert_entity_at` to position purge paths
// immediately after each tool change's anchor entity.
#[slicer_module]
impl FinalizationModule for WipeTower {
    fn on_print_start(config: &ConfigView) -> Result<Self, ModuleError> {
        Self::from_config(config)
    }

    fn run_finalization(
        &self,
        layers: &[LayerCollectionView],
        output: &mut FinalizationOutputBuilder,
        config: &ConfigView,
    ) -> Result<(), ModuleError> {
        if !self.enabled {
            return Ok(());
        }

        // Parse bed_shape from config for bounds checking.
        let bed_polygon: Vec<(f32, f32)> = match config.get("bed_shape") {
            Some(ConfigValue::List(items)) => {
                let raw: Vec<f64> = items
                    .iter()
                    .filter_map(|v| match v {
                        ConfigValue::Float(f) => Some(*f),
                        ConfigValue::Int(i) => Some(*i as f64),
                        _ => None,
                    })
                    .collect();
                parse_bed_shape(&raw)?
            }
            _ => self.bed_shape.clone(),
        };

        // Validate all 4 corners of the tower bounding rectangle against the bed polygon.
        // Corners: (x, y), (x+w, y), (x+w, y+purge_depth_max), (x, y+purge_depth_max).
        // Use tower_width for a conservative bound; purge_depth varies per layer.
        let tower_corners = [
            (self.tower_x, self.tower_y),
            (self.tower_x + self.tower_width, self.tower_y),
            (
                self.tower_x + self.tower_width,
                self.tower_y + self.tower_width,
            ),
            (self.tower_x, self.tower_y + self.tower_width),
        ];
        for (cx, cy) in &tower_corners {
            if !point_in_polygon(*cx, *cy, &bed_polygon) {
                return Err(ModuleError::fatal(
                    3,
                    format!(
                        "wipe-tower corner ({:.3}, {:.3}) lies outside bed polygon",
                        cx, cy
                    ),
                ));
            }
        }

        for (idx, view) in layers.iter().enumerate() {
            if view.tool_changes().is_empty() {
                continue;
            }

            let z = view.z();
            let layer_index = view.layer_index();

            let layer_height = if idx > 0 {
                let dz = z - layers[idx - 1].z();
                if dz > 0.0 {
                    dz
                } else {
                    DEFAULT_LAYER_HEIGHT
                }
            } else {
                DEFAULT_LAYER_HEIGHT
            };

            // Snapshot tool_changes BEFORE any insertions to avoid index-remap
            // confusion. Process in REVERSE order (highest after_entity_index first)
            // so that insertions at higher indices do not shift lower indices.
            let mut tool_changes = view.tool_changes().to_vec();
            tool_changes.sort_by_key(|tc| std::cmp::Reverse(tc.after_entity_index));

            for tc in &tool_changes {
                let pairs = self.generate_purge_paths(z, layer_height, layer_index, tc);
                // Insert entities starting at tc.after_entity_index + 1, in order.
                // Each insert shifts later entities right by 1, so offset 0 → position K+1,
                // offset 1 → K+2, etc. The SDK's apply_to handles remap for other
                // ToolChange references with after_entity_index >= position.
                let base_position = tc.after_entity_index + 1;
                for (offset, (path, region_key)) in pairs.into_iter().enumerate() {
                    let position = base_position + offset as u32;
                    output
                        .insert_entity_at(layer_index, position, path, region_key)
                        .map_err(|e| ModuleError::fatal(4, e))?;
                }
            }
        }

        Ok(())
    }
}

// ── Unit tests (packet-58 TDD scaffolding) ───────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use slicer_ir::{ConfigValue, ConfigView, ToolChange};
    use slicer_sdk::traits::{FinalizationOutputBuilder, LayerCollectionView};
    use std::collections::HashMap;

    /// Build a minimal ConfigView with the given key-value pairs.
    fn config_from_pairs(pairs: &[(&str, ConfigValue)]) -> ConfigView {
        let mut map = HashMap::new();
        for (k, v) in pairs {
            map.insert(k.to_string(), v.clone());
        }
        ConfigView::from_map(map)
    }

    /// Build a ConfigView with basic wipe-tower defaults.
    fn default_config() -> ConfigView {
        config_from_pairs(&[
            ("wipe_tower_enabled", ConfigValue::Bool(true)),
            ("wipe_tower_x", ConfigValue::Float(10.0)),
            ("wipe_tower_y", ConfigValue::Float(10.0)),
            ("wipe_tower_width", ConfigValue::Float(60.0)),
            ("wipe_tower_purge_volume", ConfigValue::Float(70.0)),
            ("line_width", ConfigValue::Float(0.4)),
            ("retract_length", ConfigValue::Float(2.0)),
            (
                "bed_shape",
                ConfigValue::List(vec![
                    ConfigValue::Float(0.0),
                    ConfigValue::Float(0.0),
                    ConfigValue::Float(250.0),
                    ConfigValue::Float(0.0),
                    ConfigValue::Float(250.0),
                    ConfigValue::Float(250.0),
                    ConfigValue::Float(0.0),
                    ConfigValue::Float(250.0),
                ]),
            ),
        ])
    }

    /// Build a minimal single-layer LayerCollectionIR with one ToolChange.
    fn layer_with_tool_change(after_entity_index: u32) -> slicer_ir::LayerCollectionIR {
        use slicer_ir::{ExtrusionPath3D, ExtrusionRole, Point3WithWidth, PrintEntity, RegionKey};
        slicer_ir::LayerCollectionIR {
            global_layer_index: 0,
            z: 0.2,
            ordered_entities: vec![PrintEntity {
                entity_id: 1,
                path: ExtrusionPath3D {
                    points: vec![
                        Point3WithWidth {
                            x: 5.0,
                            y: 5.0,
                            z: 0.2,
                            width: 0.4,
                            flow_factor: 1.0,
                            overhang_quartile: None,
                        },
                        Point3WithWidth {
                            x: 6.0,
                            y: 5.0,
                            z: 0.2,
                            width: 0.4,
                            flow_factor: 1.0,
                            overhang_quartile: None,
                        },
                    ],
                    role: ExtrusionRole::OuterWall,
                    speed_factor: 1.0,
                },
                role: ExtrusionRole::OuterWall,
                region_key: RegionKey {
                    global_layer_index: 0,
                    object_id: "cube".to_string(),
                    region_id: 0,
                    variant_chain: Vec::new(),
                },
                topo_order: 0,
            }],
            tool_changes: vec![ToolChange {
                after_entity_index,
                from_tool: 0,
                to_tool: 1,
            }],
            ..Default::default()
        }
    }

    /// AC4 — wipe-tower emits entities tagged `ExtrusionRole::WipeTower`.
    ///
    /// Expected behaviour:
    ///   Given a wipe-tower enabled config and a layer with one ToolChange,
    ///   the module's generate_purge_paths returns at least one entity with
    ///   ExtrusionRole::WipeTower. This verifies that when the gcode emitter
    ///   sees a WipeTower entity it will emit `;TYPE:Prime tower`
    ///   (verified separately in gcode_emit.rs unit tests).
    #[test]
    fn emits_prime_tower_role_marker() {
        let config = default_config();
        let tower =
            WipeTower::from_config(&config).expect("from_config must succeed with valid config");

        let tc = ToolChange {
            after_entity_index: 0,
            from_tool: 0,
            to_tool: 1,
        };

        // generate_purge_paths returns (ExtrusionPath3D, RegionKey) pairs.
        let pairs = tower.generate_purge_paths(0.2, 0.2, 0, &tc);

        assert!(
            !pairs.is_empty(),
            "AC4 FAIL: generate_purge_paths returned no entities"
        );

        // Every emitted entity must be tagged WipeTower.
        for (i, (path, _rk)) in pairs.iter().enumerate() {
            assert!(
                matches!(path.role, ExtrusionRole::WipeTower),
                "AC4 FAIL: entity {} has role {:?}, expected WipeTower",
                i,
                path.role
            );
        }

        // At least 3 entities: travel, ≥1 scan line, prime.
        // (The retract is synthesized host-side by the gcode emitter; the
        // module no longer emits a marker retract entity — packet-58 Fix G.)
        assert!(
            pairs.len() >= 3,
            "AC4 FAIL: expected at least 3 entities (travel + scan lines + prime), got {}",
            pairs.len()
        );
    }

    /// NC4 — tower placed outside config-supplied bed returns a fatal ModuleError
    /// naming the violating coordinate. Setup: bed_shape=[0,0, 100,0, 100,100, 0,100]
    /// (100×100 mm), wipe_tower_x=150.0, wipe_tower_y=150.0 (outside bed).
    #[test]
    fn tower_outside_bed_returns_fatal() {
        let config = config_from_pairs(&[
            ("wipe_tower_enabled", ConfigValue::Bool(true)),
            ("wipe_tower_x", ConfigValue::Float(150.0)),
            ("wipe_tower_y", ConfigValue::Float(150.0)),
            ("wipe_tower_width", ConfigValue::Float(60.0)),
            ("wipe_tower_purge_volume", ConfigValue::Float(70.0)),
            ("line_width", ConfigValue::Float(0.4)),
            ("retract_length", ConfigValue::Float(2.0)),
            (
                "bed_shape",
                ConfigValue::List(vec![
                    ConfigValue::Float(0.0),
                    ConfigValue::Float(0.0),
                    ConfigValue::Float(100.0),
                    ConfigValue::Float(0.0),
                    ConfigValue::Float(100.0),
                    ConfigValue::Float(100.0),
                    ConfigValue::Float(0.0),
                    ConfigValue::Float(100.0),
                ]),
            ),
        ]);

        let tower =
            WipeTower::from_config(&config).expect("from_config must succeed with valid config");

        let ir_layer = layer_with_tool_change(0);
        let sdk_layers = vec![LayerCollectionView::new(ir_layer)];
        let mut output = FinalizationOutputBuilder::new();

        let result = tower.run_finalization(&sdk_layers, &mut output, &config);

        assert!(
            result.is_err(),
            "NC4 FAIL: expected run_finalization to return Err for tower outside bed, got Ok"
        );
        let err = result.unwrap_err();
        // The error message must name the violating coordinate (contains "150").
        assert!(
            err.message.contains("150"),
            "NC4 FAIL: error message does not name the violating coordinate (150). Got: {}",
            err.message
        );
        assert!(
            err.fatal,
            "NC4 FAIL: expected fatal error, got non-fatal: {}",
            err.message
        );
    }
}
