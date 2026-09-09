// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/SupportCommon.cpp::generate_raft_base
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Default raft footprint module.

#![warn(missing_docs)]
#![warn(unused_imports)]

use slicer_ir::{ConfigValue, ConfigView};
use slicer_sdk::builders::InfillOutputBuilder;
use slicer_sdk::error::ModuleError;
use slicer_sdk::host::{hatch_areas, offset_polygons, OffsetJoinType};
use slicer_sdk::slicer_module;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

/// Default raft fill generator scaffold.
pub struct RaftDefault {
    raft_expansion: f32,
    first_layer_expansion: f32,
    contact_distance: f32,
    line_spacing: f32,
}

#[slicer_module]
impl LayerModule for RaftDefault {
    fn from_config(config: &ConfigView) -> Result<Self, ModuleError> {
        // The filtered view supplies these values from raft-default.toml (and
        // later from the global config surface).
        let value = |key: &str, default: f32| match config.get(key) {
            Some(ConfigValue::Float(v)) => *v as f32,
            Some(ConfigValue::Int(v)) => *v as f32,
            _ => default,
        };
        Ok(Self {
            raft_expansion: value("raft_expansion", 1.5),
            first_layer_expansion: value("raft_first_layer_expansion", 2.0),
            contact_distance: value("raft_contact_distance", 0.1),
            line_spacing: value("raft_line_spacing", 0.5).max(0.000001),
        })
    }

    fn run_infill(
        &self,
        layer_index: u32,
        regions: &[SliceRegionView],
        paint: &PaintRegionLayerView,
        output: &mut InfillOutputBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        let Some(plan) = paint.raft_plan() else {
            return Ok(());
        };
        if !paint.is_raft() || plan.raft_layers == 0 {
            return Ok(());
        }
        if layer_index >= plan.raft_layers {
            return Ok(());
        }
        // RaftPlan maps to one first layer, an explicit base band, then the
        // interface band. Consume base_raft_layers directly rather than
        // inferring it from the upper-band count.
        let expected_layers = 1u32
            .checked_add(plan.base_raft_layers)
            .and_then(|layers| layers.checked_add(plan.interface_raft_layers));
        if expected_layers != Some(plan.raft_layers) {
            return Err(ModuleError::fatal(
                2,
                "raft plan must satisfy raft_layers == 1 + base_raft_layers + interface_raft_layers",
            ));
        }
        let interface_start = 1 + plan.base_raft_layers;
        for region in regions {
            let source = region.polygons();
            if source.is_empty() || !region.should_emit(slicer_ir::ExtrusionRole::RaftInfill) {
                continue;
            }
            output.begin_region(region.object_id(), *region.region_id());
            // The footprint is object-independent: it is a pure function of
            // harvested region geometry plus the raft config keys.
            let is_base_layer = layer_index > 0 && layer_index < interface_start;
            let expansion = if layer_index == 0 {
                self.first_layer_expansion
            } else if is_base_layer {
                self.raft_expansion
            } else {
                self.raft_expansion
                    + if layer_index >= interface_start {
                        // AC-4 interface spacing mapping: each upper interface-band
                        // footprint receives one raft_contact_distance margin on top
                        // of raft_expansion.  The first printed layer is the only one
                        // using raft_first_layer_expansion, preserving strict ordering.
                        self.contact_distance
                    } else {
                        0.0
                    }
            };
            let expanded = iterated_offset(source, expansion);
            if !expanded.is_empty() {
                let mut fill = expanded.clone();
                // A fixed 45° angle matches the raft's canonical diagonal pattern.
                for line in hatch_areas(&expanded, self.line_spacing, 45.0) {
                    fill.push(slicer_ir::ExPolygon {
                        contour: line,
                        holes: Vec::new(),
                    });
                }
                output
                    .push_raft_fill(fill)
                    .map_err(|e| ModuleError::fatal(1, e))?;
            }
        }
        Ok(())
    }
}

/// Apply offsets in deterministic steps, as in canonical raft inflation.
fn iterated_offset(
    source: &[slicer_ir::ExPolygon],
    expansion_mm: f32,
) -> Vec<slicer_ir::ExPolygon> {
    let steps = (expansion_mm.max(0.0) / 0.5).ceil() as usize;
    if steps == 0 {
        return source.to_vec();
    }
    let step = expansion_mm / steps as f32;
    let mut current = source.to_vec();
    for _ in 0..steps {
        current = offset_polygons(&current, step, OffsetJoinType::Round, 0.01);
    }
    current
}
