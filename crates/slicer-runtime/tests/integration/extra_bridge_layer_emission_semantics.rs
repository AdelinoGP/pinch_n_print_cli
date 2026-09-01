//! Packet 234a AC-7: host-side carrier-free extra bridge-layer emission.

use std::sync::Arc;

use slicer_ir::{
    ConfigValue, ExPolygon, Point2, Polygon, RegionKey, RegionMapIR, RegionPlan, ResolvedConfig,
    SliceIR, SlicedRegion,
};
use slicer_runtime::{commit_shell_classification_builtin, Blackboard};

fn square() -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(-5.0, -5.0),
                Point2::from_mm(5.0, -5.0),
                Point2::from_mm(5.0, 5.0),
                Point2::from_mm(-5.0, 5.0),
            ],
        },
        holes: Vec::new(),
    }
}

fn run(extra: Option<bool>) -> Vec<SliceIR> {
    let object_id = "extra-bridge-cube".to_string();
    let footprint = square();
    let slices = (0..4)
        .map(|index| SliceIR {
            global_layer_index: index,
            z: 0.2 * (index + 1) as f32,
            regions: vec![SlicedRegion {
                object_id: object_id.clone(),
                region_id: 0,
                polygons: vec![footprint.clone()],
                infill_areas: vec![footprint.clone()],
                ..Default::default()
            }],
            ..Default::default()
        })
        .collect::<Vec<_>>();
    let mut region_map = RegionMapIR::default();
    let mut resolved = ResolvedConfig {
        sparse_infill_density: 20.0,
        top_shell_layers: 3,
        bottom_shell_layers: 0,
        ..Default::default()
    };
    if let Some(enabled) = extra {
        resolved.extensions.insert(
            "enable_extra_bridge_layer".into(),
            ConfigValue::Bool(enabled),
        );
    }
    let config = region_map.intern_config(resolved);
    for index in 0..4 {
        region_map.entries.insert(
            RegionKey {
                global_layer_index: index,
                object_id: object_id.clone(),
                region_id: 0,
                variant_chain: Vec::new(),
            },
            RegionPlan {
                config,
                ..Default::default()
            },
        );
    }
    let mut blackboard = Blackboard::new(Arc::new(Default::default()), 4);
    blackboard
        .commit_region_map(Arc::new(region_map))
        .expect("region map");
    blackboard
        .commit_slice_ir(Arc::new(slices))
        .expect("slice IR");
    commit_shell_classification_builtin(&mut blackboard).expect("shell classification");
    blackboard.slice_ir().expect("classified slices").to_vec()
}

#[test]
fn default_off_is_byte_stable() {
    let absent = serde_json::to_vec(&run(None)).expect("serialize absent result");
    let explicit_off = serde_json::to_vec(&run(Some(false))).expect("serialize off result");
    assert_eq!(absent, explicit_off);
}

#[test]
fn enabled_duplicates_layer_above() {
    let baseline = run(None);
    let enabled = run(Some(true));
    let baseline_count: Vec<usize> = baseline
        .iter()
        .map(|slice| slice.regions[0].internal_bridge_areas.len())
        .collect();
    let enabled_count: Vec<usize> = enabled
        .iter()
        .map(|slice| slice.regions[0].internal_bridge_areas.len())
        .collect();
    let source = baseline_count
        .iter()
        .position(|count| *count > 0)
        .expect("fixture must qualify a bridge layer");
    assert_eq!(enabled_count[source], baseline_count[source]);
    assert_eq!(
        enabled_count[source + 1],
        baseline_count[source + 1] + baseline_count[source]
    );
    assert_eq!(
        enabled[source + 1].regions[0].internal_bridge_areas,
        baseline[source + 1].regions[0]
            .internal_bridge_areas
            .iter()
            .chain(baseline[source].regions[0].internal_bridge_areas.iter(),)
            .cloned()
            .collect::<Vec<_>>(),
        "duplicate must be the dense-interior overlap directly above the source"
    );
    println!(
        "carrier-free angle report: duplicate uses existing anchor-derived construction; canonical perpendicular intent is parent + 90 degrees"
    );
}
