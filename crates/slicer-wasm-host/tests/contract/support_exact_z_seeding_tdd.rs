//! `ExactZQueryService::with_print_plane_occupancy` (perf-vs-orca t185): the
//! host's own print-plane occupancy may replace re-slicing only where it is the
//! identical cross-section, and the zero-tolerance exact-Z gate must decide
//! every entry exactly as the slicing service does.
use std::sync::Arc;

use slicer_ir::slice_ir::SupportSourceKey;
use slicer_ir::{
    mm_to_units, units_to_mm, ExPolygon, IndexedTriangleSet, MeshIR, ObjectMesh, Point2, Point3,
    SliceIR, SlicedRegion, SupportAnalysisIR, SupportAnalysisPrintPlaneEntry, SupportPlanEntry,
    SupportPlanIR, Transform3d,
};
use slicer_wasm_host::exact_z_query::ExactZQueryService;
use slicer_wasm_host::support_aggregation::{
    aggregate_support_plans, SupportAggregationInput, SupportPlanProducer,
};

const IDENTITY: [f64; 16] = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
];

/// Column-major (`m[12..15]` = translation) affine transform: rotate by
/// `degrees` about Z, scale XY by `scale`, then translate.
fn affine(degrees: f64, scale: f64, tx: f64, ty: f64) -> [f64; 16] {
    let (s, c) = degrees.to_radians().sin_cos();
    [
        c * scale,
        s * scale,
        0.0,
        0.0,
        -s * scale,
        c * scale,
        0.0,
        0.0,
        0.0,
        0.0,
        1.0,
        0.0,
        tx,
        ty,
        0.0,
        1.0,
    ]
}

/// A 10 mm cube with a slanted top (an extra vertex), so cross-sections
/// change with Z.
fn object(id: &str, matrix: [f64; 16]) -> ObjectMesh {
    let p = |x: f32, y: f32, z: f32| Point3 { x, y, z };
    ObjectMesh {
        id: id.into(),
        mesh: IndexedTriangleSet {
            vertices: vec![
                p(0.0, 0.0, 0.0),
                p(10.0, 0.0, 0.0),
                p(10.0, 10.0, 0.0),
                p(0.0, 10.0, 0.0),
                p(0.0, 0.0, 6.0),
                p(10.0, 0.0, 10.0),
                p(10.0, 10.0, 10.0),
                p(0.0, 10.0, 6.0),
            ],
            indices: vec![
                0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 1, 2, 6, 1, 6, 5, 2, 3, 7, 2,
                7, 6, 3, 0, 4, 3, 4, 7,
            ],
        },
        transform: Transform3d { matrix },
        ..Default::default()
    }
}

fn empty_object(id: &str) -> ObjectMesh {
    ObjectMesh {
        id: id.into(),
        transform: Transform3d { matrix: IDENTITY },
        ..Default::default()
    }
}

fn region(object_id: &str) -> SlicedRegion {
    SlicedRegion {
        object_id: object_id.into(),
        region_id: 7,
        polygons: vec![square(0, 0, 10)],
        ..Default::default()
    }
}

fn square(x: i64, y: i64, size: i64) -> ExPolygon {
    ExPolygon {
        contour: slicer_ir::Polygon {
            points: vec![
                Point2 { x, y },
                Point2 { x: x + size, y },
                Point2 {
                    x: x + size,
                    y: y + size,
                },
                Point2 { x, y: y + size },
            ],
        },
        holes: Vec::new(),
    }
}

/// Layers 0..48 at 0.2 mm (through the slanted top), every object present on
/// every layer.
fn slices(object_ids: &[&str]) -> Vec<SliceIR> {
    (0..48u32)
        .map(|index| SliceIR {
            global_layer_index: index,
            z: 0.2 * (index + 1) as f32,
            regions: object_ids.iter().map(|id| region(id)).collect(),
            ..Default::default()
        })
        .collect()
}

/// `print_plane_occupancy` exactly as `collect_print_plane_occupancy`
/// (`crates/slicer-runtime/src/builtins/support_analysis_producer.rs`) builds it,
/// for objects of `mesh` and, additionally, for `ghost_ids` absent from it.
fn analysis(mesh: &MeshIR, slices: &[SliceIR], ghost_ids: &[&str]) -> SupportAnalysisIR {
    let planes: Vec<f32> = slices
        .iter()
        .map(|slice| units_to_mm(mm_to_units(slice.z)))
        .collect();
    let mut print_plane_occupancy = Vec::new();
    for object in &mesh.objects {
        let world = slicer_core::algos::prepass_slice::object_world_mesh(object);
        let sections = slicer_core::slice_mesh_ex(&world, &planes);
        for (slice, polygons) in slices.iter().zip(sections) {
            print_plane_occupancy.push(SupportAnalysisPrintPlaneEntry {
                global_support_layer_index: slice.global_layer_index,
                object_id: object.id.clone(),
                polygons,
            });
        }
    }
    for ghost in ghost_ids {
        print_plane_occupancy.push(SupportAnalysisPrintPlaneEntry {
            global_support_layer_index: 0,
            object_id: (*ghost).into(),
            polygons: vec![square(0, 0, 1)],
        });
    }
    SupportAnalysisIR {
        print_plane_occupancy,
        ..SupportAnalysisIR::default()
    }
}

/// The occupancy production reads: the batched prefetch, then the query.
fn sliced_occupancy(mesh: &Arc<MeshIR>, object_id: &str, z_mm: f32) -> Vec<ExPolygon> {
    let service = ExactZQueryService::new(Arc::clone(mesh));
    service.prefetch_cross_sections(object_id, &[z_mm]);
    service
        .query(object_id, 7, &[], z_mm)
        .expect("sliced query")
        .occupancy
        .clone()
}

#[test]
fn seeded_print_planes_equal_the_sliced_occupancy_without_building_geometry() {
    let mesh = Arc::new(MeshIR {
        objects: vec![
            object("turned", affine(30.0, 1.5, 12.5, -7.25)),
            object("projective", {
                let mut m = IDENTITY;
                m[15] = 2.0;
                m
            }),
            empty_object("hollow"),
        ],
        ..Default::default()
    });
    let slices = slices(&["turned", "projective", "hollow"]);
    let analysis = Arc::new(analysis(&mesh, &slices, &["ghost"]));
    let service =
        ExactZQueryService::with_print_plane_occupancy(Arc::clone(&mesh), analysis, &slices);

    // Only the plainly affine, non-empty object is seeded: the projective
    // transform, the empty mesh and the object missing from the mesh are not.
    assert_eq!(service.seeded_plane_count(), slices.len());

    let planes: Vec<f32> = slices
        .iter()
        .map(|slice| units_to_mm(mm_to_units(slice.z)))
        .collect();
    service.prefetch_cross_sections("turned", &planes);
    let mut non_empty = 0;
    for &z in &planes {
        let seeded = service
            .with_occupancy("turned", z, <[ExPolygon]>::to_vec)
            .expect("seeded plane");
        non_empty += usize::from(!seeded.is_empty());
        assert_eq!(seeded, sliced_occupancy(&mesh, "turned", z), "plane {z}");
    }
    assert!(non_empty > 20, "the cube must intersect most planes");
    assert_eq!(
        service.geometry_build_count(),
        0,
        "seeded planes, prefetch included, never build the object's geometry"
    );

    // A plane that is not a print plane falls through to slicing.
    let between = 0.5 * (planes[10] + planes[11]);
    let sliced = service
        .with_occupancy("turned", between, <[ExPolygon]>::to_vec)
        .expect("unseeded plane");
    assert_eq!(service.geometry_build_count(), 1);
    assert_eq!(sliced, sliced_occupancy(&mesh, "turned", between));

    // Unseeded objects keep the slicing path and its refusals.
    assert_eq!(
        service
            .with_occupancy("projective", planes[5], <[ExPolygon]>::to_vec)
            .expect("projective object is sliced"),
        sliced_occupancy(&mesh, "projective", planes[5])
    );
    for refused in ["hollow", "ghost"] {
        assert!(
            service.with_occupancy(refused, planes[0], |_| ()).is_err(),
            "{refused}: the exact-Z query refuses this object, so must the seeded service"
        );
    }
}

fn entry(body: &str, anchor_z: i64, x: i64) -> SupportPlanEntry {
    // exhaustive: support-plan identity fixture; SupportPlanEntry has no Default impl and FRU would let a new plan field default silently
    SupportPlanEntry {
        global_layer_index: 0,
        object_id: "cube".into(),
        region_id: 7,
        family_id: "tree".into(),
        demand_ids: vec![body.into()],
        body_ids: vec![body.into()],
        anchor_layer_index: 0,
        anchor_z,
        roles: vec![slicer_ir::SupportPlanRoleRegion {
            role: slicer_ir::SupportPlanRole::SupportBody,
            regions: vec![square(x, 20_000, 10_000)],
        }],
        skeleton: None,
        capabilities: vec![],
        provenance: vec!["test".into()],
        decline_reason: None,
        variant_chain: Vec::new(),
    }
}

/// The aggregation verdicts on the same plans, for one service.
fn verdicts(service: &ExactZQueryService, plans: Vec<SupportPlanIR>) -> String {
    let mut owned = SupportAnalysisIR::default();
    for plan in &plans {
        for entry in &plan.entries {
            owned.family_assignments.insert(
                SupportSourceKey {
                    object_id: entry.object_id.clone(),
                    region_id: entry.region_id,
                    variant_chain: entry.variant_chain.clone(),
                },
                entry.family_id.clone(),
            );
        }
    }
    let producers = vec![SupportPlanProducer {
        module_id: "test.support-writer".into(),
        claims: vec!["support-family:tree".into()],
    }];
    let result = aggregate_support_plans(SupportAggregationInput {
        producers,
        plans,
        exact_z: service,
        territory: Some(&owned),
    });
    format!(
        "{:?} | {:?} | {:?} | {}",
        result.retained, result.unmet, result.diagnostics, result.degraded
    )
}

/// Seeded and slicing services must give the gate identical verdicts,
/// including a 1-unit collision on a print plane and a collision on an
/// independent support plane that no print plane covers.
#[test]
fn seeded_and_sliced_services_reject_and_retain_identically() {
    let mesh = Arc::new(MeshIR {
        objects: vec![object("cube", IDENTITY)],
        ..Default::default()
    });
    let slices = slices(&["cube"]);
    let analysis = Arc::new(analysis(&mesh, &slices, &[]));
    let print_plane = mm_to_units(slices[9].z); // 2.0 mm: the cube spans x 0..100_000
    let independent = print_plane + 1_000; // 2.1 mm: not a print plane
    let plans = || {
        vec![SupportPlanIR {
            entries: vec![
                entry("clear", print_plane, 100_005),
                entry("graze", print_plane, 99_999),
                entry("independent", independent, 99_999),
                {
                    let mut far = entry("far", print_plane, 300_000);
                    far.region_id = 8;
                    far
                },
            ],
            ..Default::default()
        }]
    };
    let seeded =
        ExactZQueryService::with_print_plane_occupancy(Arc::clone(&mesh), analysis, &slices);
    let sliced = ExactZQueryService::new(Arc::clone(&mesh));
    let seeded_verdicts = verdicts(&seeded, plans());
    assert_eq!(seeded_verdicts, verdicts(&sliced, plans()));
    // Non-vacuous: both collisions were rejected by the occupancy gate, and
    // only the independent plane needed slicing.
    for body in ["graze", "independent"] {
        assert!(
            seeded_verdicts.contains(&format!(
                "demand_id: \"{body}\", body_id: \"{body}\", reason: \"body rejected: exact-Z occupancy\""
            )),
            "{body} must be rejected: {seeded_verdicts}"
        );
    }
    assert!(seeded_verdicts.contains("\"clear\"") && seeded_verdicts.contains("\"far\""));
    assert_eq!(seeded.geometry_build_count(), 1);
}
