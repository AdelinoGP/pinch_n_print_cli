//! Native/WASM transport witness for the standalone lightning feasibility kernel.

use lightning_feasibility_kernel::planner::{clip_lightning, plan_lightning};
use lightning_feasibility_kernel::{DomainKind, GroundKind, KernelInput};
use slicer_ir::{ConfigView, ExtrusionPath3D, ExtrusionRole, PaintValue, Point3WithWidth};
use slicer_sdk::builders::InfillOutputBuilder;
use slicer_sdk::error::ModuleError;
use slicer_sdk::slicer_module;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

pub struct LightningPortabilityModule;

#[slicer_module]
impl LayerModule for LightningPortabilityModule {
    fn from_config(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }

    fn run_infill(
        &self,
        _layer_index: u32,
        regions: &[SliceRegionView],
        paint: &PaintRegionLayerView,
        output: &mut InfillOutputBuilder,
        config: &ConfigView,
    ) -> Result<(), ModuleError> {
        // The current SDK has no regions_by_semantic/custom_regions API.
        // Paint is carried by SliceIR segment_annotations; inspect both the
        // attached paint slice and the incoming guest region views.
        let custom_paint = paint.slice_ir().is_some_and(|slice| {
            slice.regions.iter().any(|region| {
                region.segment_annotations.values().any(|perimeters| {
                    perimeters
                        .iter()
                        .flatten()
                        .any(|value| matches!(value, Some(PaintValue::Custom(_))))
                }) || region
                    .variant_chain
                    .iter()
                    .any(|(_, value)| matches!(value, PaintValue::Custom(_)))
            })
        }) || regions.iter().any(|region| {
            region.segment_annotations().values().any(|perimeters| {
                perimeters
                    .iter()
                    .flatten()
                    .any(|value| matches!(value, Some(PaintValue::Custom(_))))
            })
        });
        if custom_paint {
            return Err(ModuleError::fatal(2, "custom paint value rejected"));
        }

        let json = required_string(config, "probe_input_json")?;
        let mode = required_string(config, "probe_output")?;
        if !matches!(mode, "planned" | "clipped") {
            return Err(ModuleError::fatal(3, "probe_output: unknown value"));
        }
        let dispatch = if mode == "clipped" {
            Some(required_string(config, "probe_final_dispatch")?)
        } else {
            None
        };
        let selected_layer = if mode == "planned" {
            let index = config
                .get_int("probe_layer_index")
                .ok_or_else(|| ModuleError::fatal(3, "probe_layer_index: missing or not an Int"))?;
            Some(
                u32::try_from(index)
                    .map_err(|_| ModuleError::fatal(3, "probe_layer_index: outside u32 range"))?,
            )
        } else {
            None
        };
        let input: KernelInput =
            serde_json::from_str(json).map_err(|e| ModuleError::fatal(4, e.to_string()))?;
        let plan = plan_lightning(&input).map_err(|e| ModuleError::fatal(5, e.to_string()))?;
        let width = input.parameters.extrusion_width_mm as f32;
        // Witness paths require a positive finite width even for invalid input.
        // Only a non-finite or non-positive width falls back to 1.0.
        let width = if width.is_finite() && width > 0.0 {
            width
        } else {
            1.0
        };

        if let Some(index) = selected_layer {
            let Some(layer) = plan.layers.iter().find(|l| l.global_layer_index == index) else {
                return Ok(());
            };
            output.begin_region(&input.object_id, 0, &[]);
            let z = layer.z_mm as f32;
            for node in &layer.nodes {
                let (ground, surface, ring, segment, domain) = match &node.grounding {
                    None => (0, 0, 0, 0, 0),
                    Some(witness) => (
                        match witness.kind {
                            GroundKind::Boundary => 1,
                            GroundKind::TemporaryAnchor => 2,
                        },
                        witness.boundary.surface_id,
                        witness.boundary.ring_id,
                        witness.boundary.segment_id,
                        match witness.boundary.domain {
                            DomainKind::Internal => 1,
                            DomainKind::Void => 2,
                            DomainKind::TemporaryAnchor => 3,
                        },
                    ),
                };
                emit(
                    output,
                    [
                        (1.0, node.node_id as f32),
                        (node.point.x as f32, node.point.y as f32),
                        (node.parent_id.unwrap_or(0) as f32, node.root_id as f32),
                        (node.origin_region_index as f32, ground as f32),
                        (surface as f32, ring as f32),
                        (segment as f32, domain as f32),
                    ],
                    z,
                    width,
                )?;
            }
            for edge in &layer.edges {
                let parent = layer
                    .nodes
                    .iter()
                    .find(|n| n.node_id == edge.parent_id)
                    .ok_or_else(|| ModuleError::fatal(6, "edge.parent_id: missing node"))?;
                let child = layer
                    .nodes
                    .iter()
                    .find(|n| n.node_id == edge.child_id)
                    .ok_or_else(|| ModuleError::fatal(6, "edge.child_id: missing node"))?;
                emit(
                    output,
                    [
                        (2.0, edge.edge_id as f32),
                        (edge.parent_id as f32, edge.child_id as f32),
                        (parent.point.x as f32, parent.point.y as f32),
                        (child.point.x as f32, child.point.y as f32),
                        (edge.root_id as f32, edge.origin_region_index as f32),
                        (0.0, 0.0),
                    ],
                    z,
                    width,
                )?;
            }
            let bytes = serde_json::to_vec(&layer.provenance)
                .map_err(|e| ModuleError::fatal(4, e.to_string()))?;
            let chunk_count = bytes.len().div_ceil(8);
            for (index, chunk) in bytes.chunks(8).enumerate() {
                let mut padded = [0_u8; 8];
                padded[..chunk.len()].copy_from_slice(chunk);
                emit(
                    output,
                    [
                        (3.0, index as f32),
                        (bytes.len() as f32, chunk_count as f32),
                        (padded[0] as f32, padded[1] as f32),
                        (padded[2] as f32, padded[3] as f32),
                        (padded[4] as f32, padded[5] as f32),
                        (padded[6] as f32, padded[7] as f32),
                    ],
                    z,
                    width,
                )?;
            }
        } else if let Some(dispatch) = dispatch {
            let clipped =
                clip_lightning(&plan, &input).map_err(|e| ModuleError::fatal(5, e.to_string()))?;
            let mut matches = clipped
                .iter()
                .flat_map(|layer| layer.fills.iter().map(move |fill| (layer, fill)))
                .filter(|(_, fill)| fill.dispatch_token == dispatch);
            let (layer, fill) = matches
                .next()
                .ok_or_else(|| ModuleError::fatal(7, "missing dispatch"))?;
            if matches.next().is_some() {
                return Err(ModuleError::fatal(7, "ambiguous dispatch"));
            }
            let identity = &fill.caller_identity;
            if identity
                .variant_chain
                .iter()
                .any(|(_, value)| matches!(value, PaintValue::Custom(_)))
            {
                return Err(ModuleError::fatal(2, "custom paint value rejected"));
            }
            output.begin_region(
                &identity.object_id,
                identity.region_id,
                &identity.variant_chain,
            );
            for [a, b] in &fill.segments {
                emit(
                    output,
                    [(a.x as f32, a.y as f32), (b.x as f32, b.y as f32)],
                    layer.z_mm as f32,
                    width,
                )?;
            }
        }
        Ok(())
    }
}

fn required_string<'a>(config: &'a ConfigView, key: &str) -> Result<&'a str, ModuleError> {
    config
        .get_string(key)
        .ok_or_else(|| ModuleError::fatal(3, format!("{key}: missing or not a String")))
}

fn emit<const N: usize>(
    output: &mut InfillOutputBuilder,
    coordinates: [(f32, f32); N],
    z: f32,
    width: f32,
) -> Result<(), ModuleError> {
    let path = ExtrusionPath3D {
        points: coordinates
            .into_iter()
            .map(|(x, y)| Point3WithWidth {
                x,
                y,
                z,
                width,
                flow_factor: 1.0,
                overhang_quartile: None,
                dist_to_top_mm: 0.0,
                overhang_distance_mm: None,
            })
            .collect(),
        role: ExtrusionRole::SparseInfill,
        speed_factor: 1.0,
        tool_index: None,
        order_lock: None,
    };
    output
        .push_sparse_path(path)
        .map_err(|e| ModuleError::fatal(1, e))?;
    if output
        .sparse_path_origins()
        .last()
        .is_none_or(Option::is_none)
    {
        return Err(ModuleError::fatal(6, "origin: missing explicit origin"));
    }
    Ok(())
}
