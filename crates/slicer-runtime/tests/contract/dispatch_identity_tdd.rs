// dispatch_identity_tdd.rs — Region-identity preservation across dispatch
// (bucket-by-origin for infill/perimeter/support/slice-postprocess)

#![allow(missing_docs, dead_code, unused_imports)]

use crate::common::{dispatch_fixture, ir_builders};
use slicer_ir::GlobalLayer;

fn default_layer() -> GlobalLayer {
    GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    }
}

#[test]
fn slice_postprocess_merge_replaces_polygons_preserving_identity() {
    use slicer_runtime::wit_host::{
        merge_slice_postprocess_into, ExPolygon, Point2, Polygon, RegionKey,
        SlicePostprocessCollected,
    };

    let existing = ir_builders::slice_ir::with_count(2)
        .at_layer(5)
        .at_z(0.2)
        .build();
    let target_key = RegionKey {
        layer_index: 5,
        object_id: existing.regions[1].object_id.clone(),
        region_id: existing.regions[1].region_id.to_string(),
    };
    let output = SlicePostprocessCollected {
        polygon_updates: vec![(
            target_key,
            vec![ExPolygon {
                contour: Polygon {
                    points: vec![
                        Point2 { x: 0, y: 0 },
                        Point2 { x: 100, y: 0 },
                        Point2 { x: 100, y: 100 },
                    ],
                },
                holes: Vec::new(),
            }],
        )],
        path_z_updates: Vec::new(),
    };

    let merged =
        merge_slice_postprocess_into(existing.clone(), &output).expect("merge should succeed");
    assert_eq!(
        merged.regions.len(),
        2,
        "all regions preserved (not flattened)"
    );
    assert_eq!(
        merged.regions[0], existing.regions[0],
        "untouched region unchanged"
    );
    assert_eq!(merged.regions[1].object_id, existing.regions[1].object_id);
    assert_eq!(merged.regions[1].region_id, existing.regions[1].region_id);
    assert_eq!(merged.regions[1].polygons[0].contour.points.len(), 3);
}

#[test]
fn perimeter_postprocess_commit_preserves_distinct_region_identities() {
    let ids = [("alpha", 11u64), ("beta", 22u64), ("gamma", 33u64)];
    let mut fx = dispatch_fixture::for_stage("Layer::PerimetersPostProcess")
        .with_slice(ir_builders::slice_ir::with_ids(&ids).build())
        .with_perimeter(
            ir_builders::perimeter_ir::with_ids(&ids)
                .walls(2)
                .infill(1)
                .build(),
        )
        .build();

    fx.run_layer(&default_layer()).unwrap();

    let perim = fx.arena.perimeter().expect("perimeter populated");
    assert_eq!(
        perim.regions.len(),
        3,
        "3 distinct regions preserved (not flattened)"
    );
    let observed: Vec<(String, u64)> = perim
        .regions
        .iter()
        .map(|r| (r.object_id.clone(), r.region_id))
        .collect();
    let expected: Vec<(String, u64)> = ids.iter().map(|(o, r)| (o.to_string(), *r)).collect();
    assert_eq!(observed, expected, "identities preserved in input order");
    for r in &perim.regions {
        assert_eq!(
            r.walls.len(),
            1,
            "each committed region got its own wall-loop"
        );
    }
}

#[test]
fn infill_postprocess_commit_preserves_distinct_region_identities() {
    let ids = [("part-A", 7u64), ("part-B", 9u64)];
    let mut fx = dispatch_fixture::for_stage("Layer::InfillPostProcess")
        .with_perimeter(
            ir_builders::perimeter_ir::with_ids(&ids)
                .walls(1)
                .infill(1)
                .build(),
        )
        .build();

    fx.run_layer(&default_layer()).unwrap();

    let infill = fx.arena.infill().expect("infill populated");
    assert_eq!(infill.regions.len(), 2, "2 distinct regions preserved");
    let observed: Vec<(String, u64)> = infill
        .regions
        .iter()
        .map(|r| (r.object_id.clone(), r.region_id))
        .collect();
    let expected: Vec<(String, u64)> = ids.iter().map(|(o, r)| (o.to_string(), *r)).collect();
    assert_eq!(observed, expected, "identities preserved in input order");
}

#[test]
fn perimeter_postprocess_identity_preservation_deterministic() {
    let ids = [("x", 1u64), ("y", 2u64), ("z", 3u64), ("w", 4u64)];
    let mut results = Vec::new();
    for _ in 0..3 {
        let mut fx = dispatch_fixture::for_stage("Layer::PerimetersPostProcess")
            .with_slice(ir_builders::slice_ir::with_ids(&ids).build())
            .with_perimeter(
                ir_builders::perimeter_ir::with_ids(&ids)
                    .walls(2)
                    .infill(0)
                    .build(),
            )
            .build();
        fx.run_layer(&default_layer()).unwrap();
        results.push(fx.arena.take_perimeter().unwrap());
    }
    assert_eq!(results[0], results[1]);
    assert_eq!(results[1], results[2]);
}

#[test]
fn perimeter_postprocess_identity_isolation_across_dispatches() {
    let ids1 = [("first", 100u64), ("second", 200u64)];
    let mut fx1 = dispatch_fixture::for_stage("Layer::PerimetersPostProcess")
        .with_slice(ir_builders::slice_ir::with_ids(&ids1).build())
        .with_perimeter(
            ir_builders::perimeter_ir::with_ids(&ids1)
                .walls(1)
                .infill(0)
                .build(),
        )
        .build();
    fx1.run_layer(&default_layer()).unwrap();

    let ids2 = [("alt", 999u64)];
    let mut fx2 = dispatch_fixture::for_stage("Layer::PerimetersPostProcess")
        .with_slice(ir_builders::slice_ir::with_ids(&ids2).build())
        .with_perimeter(
            ir_builders::perimeter_ir::with_ids(&ids2)
                .walls(1)
                .infill(0)
                .build(),
        )
        .build();
    fx2.run_layer(&default_layer()).unwrap();

    let p1 = fx1.arena.perimeter().unwrap();
    let p2 = fx2.arena.perimeter().unwrap();
    assert_eq!(
        p1.regions
            .iter()
            .map(|r| (r.object_id.clone(), r.region_id))
            .collect::<Vec<_>>(),
        vec![("first".to_string(), 100), ("second".to_string(), 200)]
    );
    assert_eq!(
        p2.regions
            .iter()
            .map(|r| (r.object_id.clone(), r.region_id))
            .collect::<Vec<_>>(),
        vec![("alt".to_string(), 999)],
        "no leak from prior dispatch's identities"
    );
}

#[test]
fn slice_postprocess_commit_preserves_distinct_region_identities() {
    let mut fx = dispatch_fixture::for_stage("Layer::SlicePostProcess")
        .with_slice(ir_builders::slice_ir::with_count(3).at_z(0.2).build())
        .build();

    fx.run_layer(&default_layer()).unwrap();

    let slice = fx
        .arena
        .slice()
        .expect("slice populated after post-process merge");
    assert_eq!(
        slice.regions.len(),
        3,
        "all three source regions preserved (not flattened)"
    );
    let observed: Vec<(String, u64)> = slice
        .regions
        .iter()
        .map(|r| (r.object_id.clone(), r.region_id))
        .collect();
    let expected: Vec<(String, u64)> = vec![
        ("obj-0".into(), 0),
        ("obj-1".into(), 1),
        ("obj-2".into(), 2),
    ];
    assert_eq!(
        observed, expected,
        "identities preserved in input order after merge"
    );
    for r in &slice.regions {
        assert_eq!(r.polygons.len(), 1);
        assert_eq!(
            r.polygons[0].contour.points.len(),
            3,
            "guest polygon replacement applied per region"
        );
    }
}

#[test]
fn support_postprocess_commit_preserves_distinct_region_identities() {
    let mut fx = dispatch_fixture::for_stage("Layer::SupportPostProcess")
        .with_slice(ir_builders::slice_ir::with_count(2).at_z(0.2).build())
        .build();

    fx.run_layer(&default_layer()).unwrap();

    let support = fx
        .arena
        .support()
        .expect("support populated after post-process");
    assert_eq!(
        support.support_paths.len(),
        2,
        "two origin-tagged paths preserved"
    );
    assert_eq!(
        support.support_paths[0].points[0].x, 1.0,
        "region 0 has 1 polygon"
    );
    assert_eq!(
        support.support_paths[1].points[0].x, 1.0,
        "region 1 has 1 polygon"
    );
}

#[test]
fn slice_postprocess_identity_preservation_deterministic() {
    let mut results = Vec::new();
    for _ in 0..3 {
        let mut fx = dispatch_fixture::for_stage("Layer::SlicePostProcess")
            .with_slice(ir_builders::slice_ir::with_count(4).at_z(0.2).build())
            .build();
        fx.run_layer(&default_layer()).unwrap();
        results.push(fx.arena.take_slice().unwrap());
    }
    assert_eq!(results[0], results[1]);
    assert_eq!(results[1], results[2]);
}

#[test]
fn support_postprocess_identity_isolation_across_dispatches() {
    let mut fx1 = dispatch_fixture::for_stage("Layer::SupportPostProcess")
        .with_slice(ir_builders::slice_ir::with_count(3).at_z(0.2).build())
        .build();
    fx1.run_layer(&default_layer()).unwrap();

    let mut fx2 = dispatch_fixture::for_stage("Layer::SupportPostProcess")
        .with_slice(ir_builders::slice_ir::with_count(1).at_z(0.2).build())
        .build();
    fx2.run_layer(&default_layer()).unwrap();

    assert_eq!(
        fx1.arena.support().unwrap().support_paths.len(),
        3,
        "dispatch 1 kept its 3 regions"
    );
    assert_eq!(
        fx2.arena.support().unwrap().support_paths.len(),
        1,
        "dispatch 2 kept its 1 region (no leak)"
    );
}

#[test]
fn support_output_rejects_untagged_push_in_identity_mode() {
    use slicer_runtime::wit_host::{
        convert_support_output, ExtrusionPath3d, ExtrusionRole, Point3WithWidth,
        SupportOutputCollected,
    };
    let mk_path = || ExtrusionPath3d {
        points: vec![Point3WithWidth {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            width: 0.4,
            flow_factor: 1.0,
            overhang_quartile: None,
        }],
        role: ExtrusionRole::SupportMaterial,
        speed_factor: 1.0,
    };
    let output = SupportOutputCollected {
        support_paths: vec![mk_path(), mk_path()],
        interface_paths: Vec::new(),
        raft_paths: Vec::new(),
        support_path_origins: vec![Some(("obj-0".into(), 0)), None],
        interface_path_origins: Vec::new(),
        raft_path_origins: Vec::new(),
    };
    let result = convert_support_output(&output, 0);
    assert!(result.is_err(), "untagged push in identity mode must fail");
    let msg = result.unwrap_err();
    assert!(
        msg.contains("active slice source region") || msg.contains("without an active"),
        "diagnostic should explain missing region context: {msg}"
    );
}
