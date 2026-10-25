// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/Fill/Lightning/Generator.cpp
// Original code owner: Ultimaker B.V. (Copyright (c) 2021 Ultimaker B.V.)
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Lightning sparse infill sampler module.
//!
//! The canonical lightning algorithm commits tree-edge segments to the layer
//! paint view. This module samples those segments into raw sparse-infill paths.

#![warn(missing_docs)]
#![warn(unused_imports)]

use slicer_ir::{ConfigValue, ConfigView, ExtrusionPath3D, ExtrusionRole, Point3WithWidth};
use slicer_sdk::builders::InfillOutputBuilder;
use slicer_sdk::error::ModuleError;
use slicer_sdk::preparation::PreparedPlanView;
use slicer_sdk::slicer_module;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

use crate::kernel::KernelPlan;
use crate::private_plan::{decode_layer_piece, piece_name};

// The relocated bodies keep the host copy's surface verbatim — planning logic is
// frozen — including members the entry point does not call: the two
// written-only `Generator` fields, `Generator::{get_trees_for_layer,
// take_committed_segments}`, `DistanceField::unsupported_count`, `Layer::new`
// and `KernelError::Unimplemented`. In `crates/slicer-core/src/algos/lightning/`
// those were `pub` items of a public crate, so they were never dead; inside this
// private module the same items are unreachable. Deleting them would be a change
// to the moved code that this migration step may not make, so the allowance is
// scoped here and the residue is recorded in the packet ledger.
#[allow(dead_code)]
mod kernel;
mod planning_input;
mod preparation;
mod private_plan;
mod projection;

/// Default base speed used for normalizing speed factors (mm/s).
const BASE_SPEED: f32 = 50.0;

/// Lightning sparse infill sampler.
pub struct LightningInfill {
    /// Infill density (0.0 to 1.0).
    density: f32,
    /// Infill print speed in mm/s.
    infill_speed: f32,
    /// Extrusion line width in millimeters.
    line_width: f32,
}

impl LightningInfill {
    /// Returns the configured infill density.
    pub fn density(&self) -> f32 {
        self.density
    }

    /// Returns the configured line width.
    pub fn line_width(&self) -> f32 {
        self.line_width
    }
}

#[slicer_module(preparation)]
impl LayerModule for LightningInfill {
    fn from_config(config: &ConfigView) -> Result<Self, ModuleError> {
        // Required reads: the registry seeds every declared key's default into
        // a bound view, so an absent key is a contract violation, not a
        // configurable fallback (packet 06 fail-closed semantics; mirrors
        // arachne-perimeters).
        let density = config.require_float("infill_density")? as f32;

        let infill_speed = config.require_float("infill_speed")? as f32;

        // The `width` closure below keeps its literal `0.0` arms: they are the
        // documented canonical auto-0 sentinel (packet 185 AC-5), not
        // configurable defaults — `resolve_role_width` routes a zero role
        // width through the already-expanded base width.
        let width = |key: &str, fallback: f32| match config.get(key) {
            Some(ConfigValue::Float(w)) => *w as f32,
            Some(ConfigValue::Int(w)) => *w as f32,
            _ => fallback,
        };

        let line_width = slicer_core::flow::resolve_role_width(
            ExtrusionRole::SparseInfill,
            false,
            false,
            &slicer_core::flow::RoleWidthContext {
                // Packet 185 (AC-5): absent `line_width` is the canonical
                // auto-0 sentinel (1.125 × nozzle), not the legacy 0.4 mm.
                line_width: width("line_width", 0.0),
                nozzle_diameter: 0.4,
                // Packet 06 (AC-3): both keys are declared
                // `float_or_percent` with `base_key = "nozzle_diameter"`
                // (0.4) in the manifest, so a bound view always holds them;
                // a missing value is a contract violation, not a fallback.
                bridge_line_width: config.require_abs_value("bridge_line_width", 0.4)? as f32,
                initial_layer_line_width: config
                    .require_abs_value("initial_layer_line_width", 0.4)?
                    as f32,
                sparse_infill_line_width: width("sparse_infill_line_width", 0.0),
                ..Default::default()
            },
        );

        Ok(Self {
            density,
            infill_speed,
            line_width,
        })
    }

    fn run_infill(
        &self,
        layer_index: u32,
        regions: &[SliceRegionView],
        _paint: &PaintRegionLayerView,
        output: &mut InfillOutputBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        let speed_factor = self.infill_speed / BASE_SPEED;
        // The plan arrives as this layer's private piece (D3). No paint-view
        // segment read remains: the piece is the only plan source, so a print
        // without a bound plan fails loudly instead of degrading to an empty
        // layer through a host fallback.
        let plan = read_layer_plan(layer_index)?;

        for region in regions {
            output.begin_region(
                region.object_id(),
                *region.region_id(),
                region.variant_chain(),
            );
            if !region.should_emit(ExtrusionRole::SparseInfill) {
                continue;
            }

            let z = region.z();
            // Exact source query: the full `(object_id, region_id, ordered
            // variant_chain)` must match the committed plan entry. A pair-only
            // lookup is never a fallback; an empty chain is the exact identity
            // for a genuinely chain-less source, never a wildcard.
            let segments = plan
                .entries
                .iter()
                .filter(|entry| {
                    entry.global_layer_index == layer_index as i32
                        && entry.object_id == *region.object_id()
                        && entry.region_id == *region.region_id()
                        && entry.variant_chain.as_slice() == region.variant_chain()
                })
                .flat_map(|entry| entry.tree_edge_segments.iter());

            for segment in segments {
                let points = segment
                    .iter()
                    .map(|point| Point3WithWidth {
                        x: slicer_ir::units_to_mm(point.x),
                        y: slicer_ir::units_to_mm(point.y),
                        z,
                        width: self.line_width,
                        flow_factor: 1.0,
                        overhang_quartile: None,
                        overhang_distance_mm: None,
                        dist_to_top_mm: 0.0,
                    })
                    .collect();

                output
                    .push_sparse_path(ExtrusionPath3D {
                        points,
                        role: ExtrusionRole::SparseInfill,
                        speed_factor,
                        tool_index: None,
                        order_lock: None,
                    })
                    .map_err(|error| {
                        ModuleError::fatal(
                            57413,
                            format!("lightning sparse path emission: {error}"),
                        )
                    })?;
            }
        }

        Ok(())
    }
}

/// Read and decode the prepared piece for one layer.
///
/// `PreparedPlanView::current()` is the call-local view the ordinary stage runs
/// under; the piece name is the module's own `lightning-layer-{index}` spelling,
/// and the decoder is this module's, so neither the host nor the SDK interprets
/// the bytes.
fn read_layer_plan(layer_index: u32) -> Result<KernelPlan, ModuleError> {
    let name = piece_name(layer_index as i32);
    let view = PreparedPlanView::current().map_err(ModuleError::from)?;
    let piece = view.require(&name)?;
    let len = piece.len().map_err(ModuleError::from)?;
    let bytes = piece.read(0, len).map_err(ModuleError::from)?;
    decode_layer_piece(&name, &bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_config_defaults() {
        // Packet 06 (AC-3): `bridge_line_width` / `initial_layer_line_width`
        // are contract-required reads (`require_abs_value` over the fixed
        // 0.4 mm nozzle base); a bound view always holds them at their
        // resolved default (0.4 at the module's nozzle base).
        let mut values = std::collections::HashMap::new();
        values.insert("bridge_line_width".into(), ConfigValue::Float(0.4));
        values.insert("initial_layer_line_width".into(), ConfigValue::Float(0.4));
        // Packet 04 (TASK-565): the host expands the line_width auto-0
        // sentinel (1.125 × nozzle_diameter) before guests see the view, so
        // a bound view already holds the expanded width (0.45 at the
        // module's fixed 0.4 mm nozzle). Seed the bound value; the
        // production auto-expansion lives host-side now.
        values.insert("line_width".into(), ConfigValue::Float(0.45));
        // Declared keys (lightning-infill.toml) seeded at their manifest
        // defaults, mirroring the production `seed_registry_defaults` step.
        values.insert("infill_density".into(), ConfigValue::Float(0.2));
        values.insert("infill_angle".into(), ConfigValue::Float(45.0));
        values.insert("infill_speed".into(), ConfigValue::Float(60.0));
        let config = ConfigView::from_map(values);
        let module = LightningInfill::from_config(&config).unwrap();
        assert!((module.density - 0.2).abs() < 0.001);
        // Packet 185 (AC-5): a bound view's line_width resolves to the
        // canonical auto width 1.125 × nozzle_diameter (0.45 at the module's
        // fixed 0.4 mm nozzle), not the legacy 0.4 mm default.
        assert!((module.line_width - 0.45).abs() < 0.001);
    }
}
