// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/GCode/WipeTower.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
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
    printable_area: Vec<(f32, f32)>,
    /// Per-filament-pair purge volumes in mm^3, indexed `[from_tool][to_tool]`.
    /// Empty when the profile supplied no `flush_volumes_matrix`, in which case
    /// every tool change falls back to the flat `purge_volume`.
    flush_volumes: Vec<Vec<f32>>,
    /// Scale applied to every `flush_volumes` entry. Inert while `flush_volumes`
    /// is empty — it never scales the `purge_volume` fallback.
    flush_multiplier: f32,
}

/// Canonical OrcaSlicer default for `flush_multiplier` (`PrintConfig.cpp`).
const DEFAULT_FLUSH_MULTIPLIER: f32 = 0.3;

/// Parse a flat, row-major `N*N` float list into `[from_tool][to_tool]` rows.
///
/// Mirrors canonical `WipeTower2::extract_wipe_volumes`, which derives the
/// extruder count as `sqrt(size)` and slices the flat vector into rows. Unlike
/// canonical this rejects a non-square length instead of truncating it: a
/// mis-sized matrix silently mis-indexes every pair, which is worse than a
/// startup error.
fn parse_flush_volumes_matrix(raw: &[f64]) -> Result<Vec<Vec<f32>>, ModuleError> {
    if raw.is_empty() {
        return Ok(Vec::new());
    }
    let n = (raw.len() as f64).sqrt().round() as usize;
    if n == 0 || n * n != raw.len() {
        return Err(ModuleError::fatal(
            5,
            format!(
                "flush_volumes_matrix has {} entries; expected a perfect square (N*N for N tools)",
                raw.len()
            ),
        ));
    }
    let mut rows = Vec::with_capacity(n);
    for i in 0..n {
        rows.push(raw[i * n..(i + 1) * n].iter().map(|v| *v as f32).collect());
    }
    Ok(rows)
}

/// Parse a flat `[x0, y0, x1, y1, …]` float list into `(x, y)` vertex pairs.
///
/// Returns `Err` if the list is empty, has odd length, or has fewer than 6
/// values (i.e. fewer than 3 vertices — not a polygon).
/// Read a config key as a flat `f64` list, accepting both spellings of a bed
/// polygon.
///
/// This port models `printable_area` as interleaved `[x0, y0, x1, y1, ...]`
/// numbers, but an Orca 3MF plate serialises the same key as point strings
/// (`["0x0", "250x0", "250x210", "0x210"]`), which expand to two entries each
/// via `slicer_ir::parse_orca_point_string`. Entries in neither form are
/// skipped, exactly as before — `parse_printable_area` is what judges whether
/// what survived is a usable polygon.
fn float_list_from_config(config: &ConfigView, key: &str) -> Option<Vec<f64>> {
    let Some(ConfigValue::List(items)) = config.get(key) else {
        return None;
    };
    let mut raw: Vec<f64> = Vec::with_capacity(items.len() * 2);
    for v in items {
        match v {
            ConfigValue::Float(f) => raw.push(*f),
            ConfigValue::Int(i) => raw.push(*i as f64),
            ConfigValue::String(s) => {
                if let Some((x, y)) = slicer_ir::parse_orca_point_string(s) {
                    raw.push(x);
                    raw.push(y);
                }
            }
            _ => {}
        }
    }
    Some(raw)
}

fn parse_printable_area(raw: &[f64]) -> Result<Vec<(f32, f32)>, ModuleError> {
    if raw.is_empty() {
        return Err(ModuleError::fatal(
            2,
            "printable_area config is empty; expected at least 6 values [x0,y0,x1,y1,x2,y2]",
        ));
    }
    if !raw.len().is_multiple_of(2) {
        return Err(ModuleError::fatal(
            2,
            format!(
                "printable_area has odd length {}; must be even (interleaved x,y pairs)",
                raw.len()
            ),
        ));
    }
    if raw.len() < 6 {
        return Err(ModuleError::fatal(
            2,
            format!(
                "printable_area has only {} values; need at least 6 for a 3-vertex polygon",
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
        let enabled = match config.get("enable_prime_tower") {
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

        let tower_width = match config.get("prime_tower_width") {
            Some(ConfigValue::Float(v)) => *v as f32,
            _ => 60.0,
        };

        let purge_volume = match config.get("prime_volume") {
            Some(ConfigValue::Float(v)) => *v as f32,
            // Match the manifest default (wipe-tower.toml: default 45.0, max 50.0),
            // which ticket 100 aligned to OrcaSlicer's `prime_volume` default.
            // The previous 70.0 fallback exceeded the schema max and is reachable
            // now that multi-tool prints auto-enable the wipe tower without
            // necessarily supplying an explicit purge volume.
            _ => 45.0,
        };

        let line_width = match config.get("line_width") {
            Some(ConfigValue::Float(v)) => *v as f32,
            _ => 0.4,
        };

        let retract_length = match config.get("retract_length") {
            Some(ConfigValue::Float(v)) => *v as f32,
            _ => 2.0,
        };

        // Parse printable_area from config (interleaved [x0,y0,x1,y1,...] or
        // Orca point strings — see `float_list_from_config`).
        // Default to a 250×250 mm rectangle if not provided.
        let default_bed = || vec![(0.0, 0.0), (250.0, 0.0), (250.0, 250.0), (0.0, 250.0)];
        let printable_area = match float_list_from_config(config, "printable_area") {
            Some(raw) if raw.len() >= 6 && raw.len().is_multiple_of(2) => {
                parse_printable_area(&raw).unwrap_or_else(|_| default_bed())
            }
            _ => default_bed(),
        };

        // `flush_volumes_matrix` / `flush_multiplier`: the manifest default of a
        // plain float or float-list key never reaches a module (only `percent` /
        // `float_or_percent` schema defaults are threaded into `ResolvedConfig`
        // by `resolve_global_config`), so the effective defaults live here.
        let flush_volumes = match float_list_from_config(config, "flush_volumes_matrix") {
            Some(raw) => parse_flush_volumes_matrix(&raw)?,
            None => Vec::new(),
        };

        let flush_multiplier = match config.get("flush_multiplier") {
            Some(ConfigValue::Float(v)) => *v as f32,
            _ => DEFAULT_FLUSH_MULTIPLIER,
        };

        Ok(Self {
            tower_x,
            tower_y,
            tower_width,
            purge_volume,
            line_width,
            enabled,
            retract_length,
            printable_area,
            flush_volumes,
            flush_multiplier,
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
                    // The purge prints with the DESTINATION filament being changed
                    // to (`tc.to_tool`) — the tower extrudes the incoming material to
                    // flush the old colour. region_id is a pure identity post-split
                    // and is never read as the tool (D-125 invariant).
                    let tool_index = tc.to_tool;
                    // TODO(packet-41): retire this legacy `process()` path;
                    // live path is `run_finalization` which routes through
                    // `push_entity_with_priority(..., WipeTower.default_priority())`.
                    layers[layer_idx].ordered_entities.push(PrintEntity {
                        entity_id: 0,
                        path,
                        role,
                        tool_index,
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
    /// module side (see packet 58 tool-rotation scheduling contract follow-up (i), DEV-054 closed).
    ///
    /// The purge volume is per filament pair: `tc` selects the
    /// `flush_volumes_matrix` entry for this tool change (see
    /// [`WipeTower::purge_volume_for`]).
    fn generate_purge_paths(
        &self,
        z: f32,
        layer_height: f32,
        global_layer_index: u32,
        tc: &slicer_ir::ToolChange,
    ) -> Vec<(ExtrusionPath3D, RegionKey)> {
        let cross_section = self.line_width * layer_height * self.tower_width;
        if cross_section <= 0.0 {
            return Vec::new();
        }

        let purge_volume = self.purge_volume_for(tc.from_tool, tc.to_tool);

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
                    dist_to_top_mm: 0.0,
                    overhang_distance_mm: None,
                },
                Point3WithWidth {
                    x: self.tower_x,
                    y: self.tower_y,
                    z,
                    width: self.line_width,
                    flow_factor: 0.0,
                    overhang_quartile: None,
                    dist_to_top_mm: 0.0,
                    overhang_distance_mm: None,
                },
            ],
            role: ExtrusionRole::WipeTower,
            speed_factor: 1.0,
            tool_index: None,
            order_lock: None,
        };
        pairs.push((travel_path, region_key.clone()));

        // ── 2. Rectilinear scan-line wall entities ───────────────────────────
        let purge_depth = purge_volume / cross_section;
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
                        dist_to_top_mm: 0.0,
                        overhang_distance_mm: None,
                    },
                    Point3WithWidth {
                        x: end_x,
                        y,
                        z,
                        width: self.line_width,
                        flow_factor: 1.0,
                        overhang_quartile: None,
                        dist_to_top_mm: 0.0,
                        overhang_distance_mm: None,
                    },
                ],
                role: ExtrusionRole::WipeTower,
                speed_factor: 1.0,
                tool_index: None,
                order_lock: None,
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
            purge_volume / (self.line_width * layer_height)
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
                    dist_to_top_mm: 0.0,
                    overhang_distance_mm: None,
                },
                Point3WithWidth {
                    x: self.tower_x + prime_length,
                    y: self.tower_y,
                    z,
                    width: self.line_width,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                    dist_to_top_mm: 0.0,
                    overhang_distance_mm: None,
                },
            ],
            role: ExtrusionRole::WipeTower,
            speed_factor: 1.0,
            tool_index: None,
            order_lock: None,
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

    /// Flat purge volume in mm^3 (`prime_volume`), used as the fallback when no
    /// `flush_volumes_matrix` is configured.
    pub fn purge_volume(&self) -> f32 {
        self.purge_volume
    }

    /// Purge volume in mm^3 for a `from_tool` -> `to_tool` change.
    ///
    /// Canonical (`WipeTower2::extract_wipe_volumes`) builds `wipe_volumes[from][to]`
    /// from `flush_volumes_matrix` scaled by `flush_multiplier`, and feeds it to
    /// `plan_toolchange`, where it sets the depth of that toolchange's purge box.
    /// This port routes the same value into the scan-line depth and the prime
    /// entity's extruded length.
    ///
    /// Recorded divergences from canonical:
    /// - **No matrix means `prime_volume`, not zero.** Canonical always has a
    ///   matrix (a preset supplies one) and zeroes it unless
    ///   `purge_in_prime_tower && single_extruder_multi_material` — neither key
    ///   exists in this tree yet (both sit in P02). Falling back to the flat
    ///   `prime_volume` keeps the tower working for prints that configure no
    ///   matrix, instead of silently emitting an empty tower.
    /// - **`flush_multiplier` scales matrix entries only.** Applying it to the
    ///   `prime_volume` fallback would cut every unconfigured print's purge to
    ///   30% of its declared volume, which no key in canonical does.
    /// - **One scalar multiplier, not one per extruder.** Canonical's
    ///   `flush_multiplier` is a `coFloats`; `WipeTower2::extract_wipe_volumes`
    ///   reads `get_at(0)` and `ToolOrdering::prepare_flush_matrices` indexes it
    ///   per nozzle. Per-tool config scoping is inventoried by ticket 118 and
    ///   is not available here; this matches the `WipeTower2` reading.
    /// - **`filament_minimal_purge_on_wipe_tower`** (canonical's per-filament
    ///   lower clamp on each entry) is Tier D per-filament config and is not in
    ///   this tree; no clamp is applied.
    pub fn purge_volume_for(&self, from_tool: u32, to_tool: u32) -> f32 {
        let (from, to) = (from_tool as usize, to_tool as usize);
        match self.flush_volumes.get(from).and_then(|row| row.get(to)) {
            Some(v) => v * self.flush_multiplier,
            None => self.purge_volume,
        }
    }

    /// Scale applied to `flush_volumes_matrix` entries.
    pub fn flush_multiplier(&self) -> f32 {
        self.flush_multiplier
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
// `from_config` delegates to the existing `from_config` constructor.
// `run_finalization` uses `insert_entity_at` to position purge paths
// immediately after each tool change's anchor entity.
#[slicer_module]
impl FinalizationModule for WipeTower {
    fn from_config(config: &ConfigView) -> Result<Self, ModuleError> {
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

        // Parse printable_area from config for bounds checking.
        let bed_polygon: Vec<(f32, f32)> = match float_list_from_config(config, "printable_area") {
            Some(raw) => parse_printable_area(&raw)?,
            None => self.printable_area.clone(),
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
                    // The purge prints with the DESTINATION filament being changed
                    // to (`tc.to_tool`) — the tower extrudes the incoming material to
                    // flush the old colour. region_id is a pure identity post-split
                    // and is never read as the tool (D-125 invariant).
                    let tool_index = tc.to_tool;
                    output
                        .insert_entity_at(layer_index, position, path, tool_index, region_key)
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
            ("enable_prime_tower", ConfigValue::Bool(true)),
            ("wipe_tower_x", ConfigValue::Float(10.0)),
            ("wipe_tower_y", ConfigValue::Float(10.0)),
            ("prime_tower_width", ConfigValue::Float(60.0)),
            ("prime_volume", ConfigValue::Float(70.0)),
            ("line_width", ConfigValue::Float(0.4)),
            ("retract_length", ConfigValue::Float(2.0)),
            (
                "printable_area",
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
                            dist_to_top_mm: 0.0,
                            overhang_distance_mm: None,
                        },
                        Point3WithWidth {
                            x: 6.0,
                            y: 5.0,
                            z: 0.2,
                            width: 0.4,
                            flow_factor: 1.0,
                            overhang_quartile: None,
                            dist_to_top_mm: 0.0,
                            overhang_distance_mm: None,
                        },
                    ],
                    role: ExtrusionRole::OuterWall,
                    speed_factor: 1.0,
                    tool_index: None,
                    order_lock: None,
                },
                role: ExtrusionRole::OuterWall,
                tool_index: 0,
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
    /// naming the violating coordinate. Setup: printable_area=[0,0, 100,0, 100,100, 0,100]
    /// (100×100 mm), wipe_tower_x=150.0, wipe_tower_y=150.0 (outside bed).
    #[test]
    fn tower_outside_bed_returns_fatal() {
        let config = config_from_pairs(&[
            ("enable_prime_tower", ConfigValue::Bool(true)),
            ("wipe_tower_x", ConfigValue::Float(150.0)),
            ("wipe_tower_y", ConfigValue::Float(150.0)),
            ("prime_tower_width", ConfigValue::Float(60.0)),
            ("prime_volume", ConfigValue::Float(70.0)),
            ("line_width", ConfigValue::Float(0.4)),
            ("retract_length", ConfigValue::Float(2.0)),
            (
                "printable_area",
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

    // ── Ticket 30 (P23 — Multimaterial / Flush options): per-pair purge volume ──
    //
    // `flush_volumes_matrix` / `flush_multiplier` replace the flat `prime_volume`
    // with canonical's per-filament-pair volume
    // (`WipeTower2::extract_wipe_volumes` -> `plan_toolchange`). The observable
    // behaviour here is the depth of the purge box: the scan-line count and the
    // prime entity's extruded length both scale with the selected volume.

    /// Config identical to `default_config` plus a 2x2 flush matrix and an
    /// explicit multiplier. `[0][1]` = 480 mm^3, so at multiplier 0.5 the
    /// 0 -> 1 change purges 240 mm^3 against `prime_volume`'s 70 mm^3.
    fn config_with_flush_matrix(multiplier: f64, matrix: &[f64]) -> ConfigView {
        let mut pairs: Vec<(&str, ConfigValue)> = vec![
            ("enable_prime_tower", ConfigValue::Bool(true)),
            ("wipe_tower_x", ConfigValue::Float(10.0)),
            ("wipe_tower_y", ConfigValue::Float(10.0)),
            ("prime_tower_width", ConfigValue::Float(60.0)),
            ("prime_volume", ConfigValue::Float(70.0)),
            ("line_width", ConfigValue::Float(0.4)),
            ("retract_length", ConfigValue::Float(2.0)),
            ("flush_multiplier", ConfigValue::Float(multiplier)),
        ];
        let matrix_value = ConfigValue::List(matrix.iter().map(|v| ConfigValue::Float(*v)).collect());
        pairs.push(("flush_volumes_matrix", matrix_value));
        pairs.push((
            "printable_area",
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
        ));
        config_from_pairs(&pairs)
    }

    /// Number of scan-line entities in a purge burst: total pairs minus the
    /// leading travel entity and the trailing prime entity.
    fn scan_line_count(pairs: &[(ExtrusionPath3D, RegionKey)]) -> usize {
        pairs.len().saturating_sub(2)
    }

    fn tool_change(from_tool: u32, to_tool: u32) -> slicer_ir::ToolChange {
        slicer_ir::ToolChange {
            after_entity_index: 0,
            from_tool,
            to_tool,
        }
    }

    /// With no matrix configured, every pair purges the flat `prime_volume` —
    /// the pre-ticket-30 behaviour, unchanged.
    #[test]
    fn absent_flush_matrix_falls_back_to_prime_volume() {
        let tower = WipeTower::from_config(&default_config()).expect("valid config");
        assert_eq!(tower.purge_volume_for(0, 1), 70.0);
        assert_eq!(tower.purge_volume_for(1, 0), 70.0);
    }

    /// A configured matrix selects per-pair volumes, scaled by the multiplier.
    #[test]
    fn flush_matrix_selects_per_pair_volume() {
        let tower = WipeTower::from_config(&config_with_flush_matrix(
            0.5,
            &[0.0, 480.0, 120.0, 0.0],
        ))
        .expect("valid config");

        assert_eq!(tower.purge_volume_for(0, 1), 240.0);
        assert_eq!(tower.purge_volume_for(1, 0), 60.0);
        assert_eq!(tower.purge_volume_for(0, 0), 0.0);
    }

    /// `flush_multiplier` is the only thing that changes between these two
    /// towers, and it changes the emitted geometry.
    #[test]
    fn flush_multiplier_scales_emitted_purge_geometry() {
        let matrix = [0.0, 480.0, 480.0, 0.0];
        let low = WipeTower::from_config(&config_with_flush_matrix(0.5, &matrix))
            .expect("valid config");
        let high = WipeTower::from_config(&config_with_flush_matrix(1.0, &matrix))
            .expect("valid config");

        let tc = tool_change(0, 1);
        let low_lines = scan_line_count(&low.generate_purge_paths(0.2, 0.2, 0, &tc));
        let high_lines = scan_line_count(&high.generate_purge_paths(0.2, 0.2, 0, &tc));

        // cross_section = line_width * layer_height * tower_width = 0.4*0.2*60 = 4.8 mm^2.
        // depth = volume / cross_section; lines are spaced `line_width` apart
        // starting at line_width/2.
        let expected = |volume: f32| {
            let depth = volume / 4.8;
            let mut y = 0.2 / 2.0;
            let mut n = 0;
            while y < depth {
                n += 1;
                y += 0.4;
            }
            n
        };
        assert_eq!(low_lines, expected(240.0), "multiplier 0.5 -> 240 mm^3");
        assert_eq!(high_lines, expected(480.0), "multiplier 1.0 -> 480 mm^3");
        assert!(
            high_lines > low_lines,
            "raising flush_multiplier must deepen the purge box: {} vs {}",
            high_lines,
            low_lines
        );
    }

    /// The matrix changes geometry against the `prime_volume` baseline — the
    /// non-default-value behaviour assertion the map's authoring rule 1 requires.
    #[test]
    fn flush_matrix_changes_emitted_purge_geometry_against_prime_volume() {
        let baseline = WipeTower::from_config(&default_config()).expect("valid config");
        let matrixed =
            WipeTower::from_config(&config_with_flush_matrix(1.0, &[0.0, 480.0, 480.0, 0.0]))
                .expect("valid config");

        let tc = tool_change(0, 1);
        let baseline_lines = scan_line_count(&baseline.generate_purge_paths(0.2, 0.2, 0, &tc));
        let matrixed_lines = scan_line_count(&matrixed.generate_purge_paths(0.2, 0.2, 0, &tc));

        assert!(
            matrixed_lines > baseline_lines,
            "480 mm^3 from the matrix must out-purge prime_volume's 70 mm^3: {} vs {}",
            matrixed_lines,
            baseline_lines
        );
    }

    /// A tool index outside the matrix falls back rather than panicking — the
    /// matrix is sized for the profile's tool count, which need not match the
    /// tool indices a given print actually uses.
    #[test]
    fn tool_index_outside_matrix_falls_back_to_prime_volume() {
        let tower =
            WipeTower::from_config(&config_with_flush_matrix(1.0, &[0.0, 480.0, 480.0, 0.0]))
                .expect("valid config");
        assert_eq!(tower.purge_volume_for(0, 5), 70.0);
        assert_eq!(tower.purge_volume_for(5, 0), 70.0);
    }

    /// A non-square matrix mis-indexes every pair, so it is rejected at
    /// construction instead of silently truncated.
    #[test]
    fn non_square_flush_matrix_returns_fatal() {
        let err = match WipeTower::from_config(&config_with_flush_matrix(1.0, &[0.0, 480.0, 480.0]))
        {
            Ok(_) => panic!("non-square flush_volumes_matrix must be rejected"),
            Err(e) => e,
        };
        assert!(
            err.message.contains("flush_volumes_matrix"),
            "error must name the offending key, got: {}",
            err.message
        );
    }

    /// The canonical default multiplier applies when the profile supplies none.
    /// The manifest default of a plain float key never reaches a module, so this
    /// pins the in-code fallback.
    #[test]
    fn flush_multiplier_defaults_to_canonical_value() {
        let tower = WipeTower::from_config(&default_config()).expect("valid config");
        assert_eq!(tower.flush_multiplier(), DEFAULT_FLUSH_MULTIPLIER);
        assert_eq!(tower.flush_multiplier(), 0.3);
    }
}
