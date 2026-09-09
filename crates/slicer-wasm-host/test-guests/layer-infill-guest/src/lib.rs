wit_bindgen::generate!({
    path: "../../../slicer-schema/wit",
    world: "slicer:layer-infill/infill-module",
    generate_all,
});

use exports::slicer::layer_infill::infill::Guest;
use slicer::common::module_errors::ModuleError;
use slicer::config::config_types::ConfigView;
use slicer::ir_handles::ir_handles::{
    InfillOutputBuilder, LayerIdx, PaintRegionLayerView, SliceRegionView,
};

struct Component;

impl Guest for Component {
    fn run(
        layer_index: LayerIdx,
        regions: Vec<SliceRegionView>,
        paint: PaintRegionLayerView,
        output: InfillOutputBuilder,
        config: ConfigView,
    ) -> Result<(), ModuleError> {
        let spacing = config.get_float("infill-spacing").unwrap_or(2.0);
        slicer::common::host_services::log(
            slicer::common::host_services::LogLevel::Info,
            &format!(
                "run-infill: layer={}, spacing={}, regions={}",
                layer_index,
                spacing,
                regions.len()
            ),
        );

        let Some(z) = regions.first().map(|region| region.z()) else {
            return Ok(());
        };
        let region_count = regions.len() as f32;
        let total_polys: f32 = regions.iter().map(|r| r.polygons().len() as f32).sum();
        let lightning_segment_count: usize = regions
            .iter()
            .map(|region| {
                paint
                    .lightning_tree_segments(
                        region.object_id().as_str(),
                        region.region_id().as_str(),
                    )
                    .len()
            })
            .sum();

        let path = slicer::types::geometry::ExtrusionPath3d {
            points: vec![
                slicer::types::geometry::Point3WithWidth {
                    overhang_distance_mm: None,
                    x: 0.0,
                    y: 0.0,
                    z,
                    width: total_polys,
                    flow_factor: region_count,
                    overhang_quartile: None,
                    dist_to_top_mm: 0.0,
                },
                slicer::types::geometry::Point3WithWidth {
                    overhang_distance_mm: None,
                    x: spacing as f32 * 10.0,
                    y: 0.0,
                    z,
                    width: 0.4,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                    dist_to_top_mm: 0.0,
                },
            ],
            role: slicer::types::geometry::ExtrusionRole::SparseInfill,
            speed_factor: 1.0,
            tool_index: None,
            order_lock: None,
        };
        output.push_sparse_path(&path).expect("push failed");
        if lightning_segment_count > 0 {
            let witness = slicer::types::geometry::ExtrusionPath3d {
                points: vec![slicer::types::geometry::Point3WithWidth {
                    overhang_distance_mm: None,
                    x: lightning_segment_count as f32,
                    y: 0.0,
                    z,
                    width: 137.0,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                    dist_to_top_mm: 0.0,
                }],
                role: slicer::types::geometry::ExtrusionRole::SparseInfill,
                speed_factor: 1.0,
                tool_index: None,
                order_lock: None,
            };
            output
                .push_sparse_path(&witness)
                .expect("push lightning witness failed");
        }
        // Raft reads (packet 240a AC-7). Both witnesses are emitted only when
        // the accessor reports raft state, so every non-raft fixture keeps its
        // existing sparse-path shape.
        if paint.is_raft() {
            output
                .push_sparse_path(&witness_path(240.0, 1.0, 0.0, 1.0, z))
                .expect("push is-raft witness failed");
        }
        if let Some(raft) = paint.raft_plan() {
            output
                .push_sparse_path(&witness_path(
                    241.0,
                    raft.raft_layers as f32,
                    raft.raft_first_layer_density,
                    raft.base_raft_layers as f32,
                    z,
                ))
                .expect("push raft-plan witness failed");
            output
                .push_sparse_path(&witness_path(
                    242.0,
                    raft.interface_raft_layers as f32,
                    0.0,
                    1.0,
                    z,
                ))
                .expect("push raft-plan interface witness failed");
        }
        Ok(())
    }
}

/// Single-point witness path: `width` tags the witness, `x`/`y`/`flow_factor`
/// carry the payload back to the host-side assertion.
fn witness_path(
    width: f32,
    x: f32,
    y: f32,
    flow_factor: f32,
    z: f32,
) -> slicer::types::geometry::ExtrusionPath3d {
    slicer::types::geometry::ExtrusionPath3d {
        points: vec![slicer::types::geometry::Point3WithWidth {
            overhang_distance_mm: None,
            x,
            y,
            z,
            width,
            flow_factor,
            overhang_quartile: None,
            dist_to_top_mm: 0.0,
        }],
        role: slicer::types::geometry::ExtrusionRole::SparseInfill,
        speed_factor: 1.0,
        tool_index: None,
        order_lock: None,
    }
}

export!(Component);
