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
        infill_density: 0.2,
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

    // Canonical second internal bridge (`PrintObject::bridge_over_infill`'s
    // extra-layer pass): the layer's cached bridge angle (last internal bridge
    // on the layer) plus 90 degrees, so the second layer crosses the first.
    let parent = *enabled[source].regions[0]
        .internal_bridge_angles_deg
        .last()
        .expect("the source layer carries one angle per qualified polygon");
    let expected = (parent + 90.0).rem_euclid(180.0);
    let upper = &enabled[source + 1].regions[0];
    assert_eq!(
        upper.internal_bridge_angles_deg.len(),
        upper.internal_bridge_areas.len(),
        "one angle per internal-bridge polygon on the duplicate layer"
    );
    let duplicate_angles = &upper.internal_bridge_angles_deg[baseline_count[source + 1]..];
    assert!(
        duplicate_angles
            .iter()
            .all(|angle| (angle - expected).abs() < 1e-4),
        "duplicates must cross the parent at {expected} deg (parent {parent}), got {duplicate_angles:?}"
    );
    // The duplicates are bridge fill, so they join `bridge_areas`; the
    // partition then removes them from sparse infill and the fill module
    // emits them.
    for duplicate in &upper.internal_bridge_areas[baseline_count[source + 1]..] {
        assert!(
            upper.bridge_areas.contains(duplicate),
            "duplicate internal bridge polygon missing from bridge_areas"
        );
    }
}

/// Canonical `PrintObject::bridge_over_infill`'s extra-layer pass REPLACES the
/// converted surface: the overlap leaves `stInternal`/`stInternalSolid` and
/// becomes the second internal bridge, while the non-overlapping leftover keeps
/// its original type. This IR has no separate second-bridge surface type, so
/// the duplicate joins `bridge_areas` (canonical's own interim workaround
/// reclassifies `stSecondInternalBridge` back to `stInternalBridge`); the
/// equivalent of "leaves the original class" is that the area must no longer
/// be claimed as dense interior (`internal_solid_fill`, this IR's
/// `stInternalSolid` carrier).
#[test]
fn converted_second_bridge_leaves_the_dense_interior_claim() {
    let baseline = run(None);
    let enabled = run(Some(true));
    let source = baseline
        .iter()
        .map(|slice| slice.regions[0].internal_bridge_areas.len())
        .position(|count| count > 0)
        .expect("fixture must qualify a bridge layer");
    let upper = &enabled[source + 1].regions[0];
    let baseline_upper_count = baseline[source + 1].regions[0].internal_bridge_areas.len();
    let duplicates = &upper.internal_bridge_areas[baseline_upper_count..];
    assert!(
        !duplicates.is_empty(),
        "fixture must produce at least one second-bridge duplicate"
    );
    for duplicate in duplicates {
        for dense in &upper.internal_solid_fill {
            let overlap = slicer_core::polygon_ops::intersection(
                std::slice::from_ref(duplicate),
                std::slice::from_ref(dense),
            );
            assert!(
                overlap.is_empty(),
                "a converted second bridge must not remain in \
                 internal_solid_fill; overlap area: {overlap:?}"
            );
        }
        // `internal_solid_fill` is a subset of `top_solid_fill`, and
        // `only_one_wall_top` derives the exposed top as their difference, so
        // keeping the converted area in the top claim would make it read as
        // exposed (one wall) instead of the second bridge.
        for top in &upper.top_solid_fill {
            let overlap = slicer_core::polygon_ops::intersection(
                std::slice::from_ref(duplicate),
                std::slice::from_ref(top),
            );
            assert!(
                overlap.is_empty(),
                "a converted second bridge must not remain in the top-solid \
                 claim; overlap area: {overlap:?}"
            );
        }
    }
    // The fixture's overlap sits inside the dense band, so this test is
    // non-vacuous only if the duplicate actually meets it — the assertion
    // above is the falsifier, and this checks the premise.
    assert!(
        !upper.internal_solid_fill.is_empty(),
        "fixture must have a dense interior on the duplicate layer"
    );
}

