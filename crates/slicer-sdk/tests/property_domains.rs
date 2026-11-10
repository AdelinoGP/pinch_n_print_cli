//! Registry-backed direct-domain properties.

use std::collections::BTreeSet;

use proptest::prelude::*;
use proptest::strategy::ValueTree;
use proptest::test_runner::{Config, RngAlgorithm, RngSeed, TestRunner};
use slicer_ir::{
    ConfigValue, ExPolygon, ExtrusionRole, LayerCollectionIR, Point2, Polygon, PrintEntity,
    TravelMove,
};
use slicer_sdk::test_support::fixtures::print_entity_base;
use slicer_sdk::test_support::property_testing::domains::*;
use slicer_sdk::test_support::property_testing::oracles::*;
use slicer_sdk::test_support::property_testing::PropertyRun;

#[path = "support/property_mesh.rs"]
mod property_mesh;

fn uuid_syntax(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() == 36
        && bytes.iter().enumerate().all(|(index, byte)| {
            if [8, 13, 18, 23].contains(&index) {
                *byte == b'-'
            } else {
                byte.is_ascii_digit() || (b'a'..=b'f').contains(byte)
            }
        })
}

fn fixed_case<S: Strategy>(strategy: &S) -> S::Value {
    let mut runner = TestRunner::new(Config {
        rng_algorithm: RngAlgorithm::ChaCha,
        rng_seed: RngSeed::Fixed(37),
        failure_persistence: None,
        ..Config::default()
    });
    strategy
        .new_tree(&mut runner)
        .expect("structural witness")
        .current()
}

#[test]
fn proptest_direct_scalar_domains() {
    assert!(uuid_syntax("00000000-0000-0000-0000-000000000000"));
    assert!(uuid_syntax("ffffffff-ffff-ffff-ffff-ffffffffffff"));
    assert!(!uuid_syntax("FFFFFFFF-FFFF-FFFF-FFFF-FFFFFFFFFFFF"));
    assert!(!uuid_syntax("é"));
    let positive_subnormal = f32::from_bits(1);
    let negative_subnormal = f64::from_bits(0x8000_0000_0000_0001);
    assert_eq!(
        fixed_case(&finite_f32(positive_subnormal..=positive_subnormal).unwrap()).to_bits(),
        1
    );
    assert_eq!(
        fixed_case(&finite_f64(negative_subnormal..=negative_subnormal).unwrap()).to_bits(),
        0x8000_0000_0000_0001
    );
    assert_eq!(fixed_case(&id_text(IdTextKind::OtherText, 0).unwrap()), "");
    assert_eq!(
        fixed_case(
            &scalar_config_values(ScalarConfigDomain::FloatOrPercent {
                min: 3.5,
                max: 3.5,
                is_percent: true,
            })
            .unwrap()
        ),
        ConfigValue::FloatOrPercent {
            value: 3.5,
            is_percent: true
        }
    );

    let run =
        PropertyRun::new("D-SCALARS", "proptest_direct_scalar_domains").expect("registry context");
    let units = i64::try_from(run.bound("max_scalar_units").expect("scalar bound"))
        .expect("scalar bound fits i64");
    let max_id_bytes =
        usize::try_from(run.bound("max_id_bytes").expect("ID bound")).expect("ID bound fits usize");
    let boundary = rounding_boundary_mm(units).expect("checked boundary limit");
    let extent32 = units as f32;
    let extent64 = units as f64;
    let strategy = (
        finite_f32(-extent32..=extent32).unwrap(),
        finite_f64(-extent64..=extent64).unwrap(),
        nonfinite_f32(),
        nonfinite_f64(),
        boundary,
        scalar_config_values(ScalarConfigDomain::Float {
            min: -extent64,
            max: extent64,
        })
        .unwrap(),
        id_text(IdTextKind::UuidSyntax, max_id_bytes).expect("UUID bound"),
        id_text(IdTextKind::OtherText, max_id_bytes).expect("text bound"),
    );
    run.check(
        &strategy,
        |(a, b, x, y, boundary, config, uuid, other), _| {
            prop_assert!(a.is_finite() && (-extent32..=extent32).contains(&a));
            prop_assert!(b.is_finite() && (-extent64..=extent64).contains(&b));
            prop_assert!(!x.is_finite() && !y.is_finite() && boundary.is_finite());
            match config {
                ConfigValue::Float(value) => {
                    prop_assert!(value.is_finite() && (-extent64..=extent64).contains(&value))
                }
                other => {
                    return Err(proptest::test_runner::TestCaseError::fail(format!(
                        "wrong config variant: {other:?}"
                    )))
                }
            }
            prop_assert!(uuid.len() <= max_id_bytes && uuid_syntax(&uuid));
            prop_assert!(other.len() <= max_id_bytes && !uuid_syntax(&other));
            Ok(())
        },
    )
    .expect("scalar domain contract");
}

#[test]
fn proptest_direct_graph_domains() {
    let dag = SmallGraph {
        node_ids: vec![7, 2, 9],
        edges: vec![(7, 2), (2, 9)],
    };
    assert_eq!(
        graph_reachability(&dag).unwrap(),
        BTreeSet::from([(7, 2), (7, 9), (2, 9)])
    );
    assert!(graph_cycle_remainder(&dag).unwrap().is_empty());
    let witness = SmallGraph {
        node_ids: vec![0, 1, 2],
        edges: vec![(0, 1), (1, 0), (1, 2)],
    };
    assert_eq!(
        graph_reachability(&witness).unwrap(),
        BTreeSet::from([(0, 0), (0, 1), (0, 2), (1, 0), (1, 1), (1, 2)])
    );
    assert_eq!(
        graph_cycle_remainder(&witness).unwrap(),
        BTreeSet::from([0, 1, 2])
    );
    assert!(matches!(
        graph_reachability(&SmallGraph {
            node_ids: vec![0, 0],
            edges: vec![]
        }),
        Err(DomainError::InvalidInput(_))
    ));

    let run =
        PropertyRun::new("D-GRAPHS", "proptest_direct_graph_domains").expect("registry context");
    let max_nodes =
        u16::try_from(run.bound("max_nodes").expect("node bound")).expect("node bound fits u16");
    let strategy = prop_oneof![
        graphs(GraphKind::Dag, max_nodes)
            .expect("DAG bound")
            .prop_map(|g| (false, g)),
        graphs(GraphKind::CycleWithTail, max_nodes)
            .expect("cycle bound")
            .prop_map(|g| (true, g)),
    ];
    run.check(&strategy, |(cycle, graph), _| {
        prop_assert!(graph.node_ids.len() <= usize::from(max_nodes));
        let paths = graph_reachability(&graph)
            .map_err(|e| proptest::test_runner::TestCaseError::fail(e.to_string()))?;
        let cyclic: Vec<_> = graph
            .node_ids
            .iter()
            .copied()
            .filter(|&n| paths.contains(&(n, n)))
            .collect();
        if cycle {
            prop_assert!(cyclic.len() >= 2);
            prop_assert!(graph
                .node_ids
                .iter()
                .any(|&tail| !paths.contains(&(tail, tail))
                    && cyclic.iter().any(|&n| paths.contains(&(n, tail)))));
            let remainder = graph_cycle_remainder(&graph).unwrap();
            prop_assert!(graph
                .node_ids
                .iter()
                .any(|&tail| remainder.contains(&tail) && !paths.contains(&(tail, tail))));
        } else {
            prop_assert!(cyclic.is_empty());
            for &(a, b) in &paths {
                let a_pos = graph.node_ids.iter().position(|&id| id == a).unwrap();
                let b_pos = graph.node_ids.iter().position(|&id| id == b).unwrap();
                prop_assert!(a_pos < b_pos);
            }
        }
        Ok(())
    })
    .expect("graph domain contract");
}

#[test]
fn proptest_direct_ir_domains() {
    let witness = LayerCollectionIR {
        ordered_entities: vec![PrintEntity {
            entity_id: 7,
            ..print_entity_base(ExtrusionRole::OuterWall)
        }],
        travel_moves: vec![
            TravelMove {
                entity_id: 7,
                ..Default::default()
            },
            TravelMove {
                entity_id: 99,
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    assert_eq!(first_missing_anchor(&witness), Some(99));
    let extremes = LayerCollectionIR {
        ordered_entities: vec![
            PrintEntity {
                entity_id: 0,
                ..print_entity_base(ExtrusionRole::OuterWall)
            },
            PrintEntity {
                entity_id: u64::MAX,
                ..print_entity_base(ExtrusionRole::OuterWall)
            },
        ],
        travel_moves: vec![
            TravelMove {
                entity_id: u64::MAX,
                ..Default::default()
            },
            TravelMove {
                entity_id: 0,
                ..Default::default()
            },
            TravelMove {
                entity_id: u64::MAX,
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    assert_eq!(first_missing_anchor(&extremes), None);
    assert!(is_permutation(&[2, 0, 1], 3));
    assert!(!is_permutation(&[0, 0, 2], 3));
    assert!(locked_groups_are_contiguous(&[3, 1, 2], &[vec![3, 1, 2]]));
    assert!(!locked_groups_are_contiguous(&[1, 2, 3], &[vec![3, 1, 2]]));

    let run = PropertyRun::new("D-IR", "proptest_direct_ir_domains").expect("registry context");
    let max_entities = usize::try_from(run.bound("max_entities").expect("entity bound"))
        .expect("entity bound fits usize");
    let max_travels = usize::try_from(run.bound("max_travels").expect("travel bound"))
        .expect("travel bound fits usize");
    let max_points = usize::try_from(run.bound("max_points").expect("point bound"))
        .expect("point bound fits usize");
    let max_layers = usize::try_from(run.bound("max_layers").expect("layer bound"))
        .expect("layer bound fits usize");
    let len = max_entities.min(8);
    let groups = if len >= 3 {
        vec![vec![2, 0]]
    } else {
        Vec::new()
    };
    let strategy = (
        anchor_cases(AnchorDomain::Valid, max_entities, max_travels).expect("valid anchor bounds"),
        anchor_cases(AnchorDomain::Dangling, max_entities, max_travels)
            .expect("dangling anchor bounds"),
        entity_layers(EntityDomain {
            roles: vec![ExtrusionRole::OuterWall, ExtrusionRole::SparseInfill],
            max_entities,
            max_points,
            max_layers,
        })
        .expect("layer bounds"),
        permutations(len),
        locked_orders(len, groups.clone()).expect("lock bounds"),
    );
    run.check(&strategy, |(valid, dangling, layers, order, locked), _| {
        prop_assert_eq!(first_missing_anchor(&valid.layer), None);
        prop_assert!(first_missing_anchor(&dangling.layer).is_some());
        prop_assert!(is_permutation(&order, len) && is_permutation(&locked, len));
        prop_assert!(locked_groups_are_contiguous(&locked, &groups));
        prop_assert!(layers.len() <= max_layers);
        let mut previous_z = None;
        let mut previous_id = 0;
        for (index, layer) in layers.iter().enumerate() {
            prop_assert_eq!(layer.global_layer_index as usize, index);
            prop_assert!(layer.z.is_finite() && previous_z.is_none_or(|z| layer.z > z));
            previous_z = Some(layer.z);
            prop_assert!(layer.ordered_entities.len() <= max_entities);
            for entity in &layer.ordered_entities {
                prop_assert!(entity.entity_id > previous_id);
                previous_id = entity.entity_id;
                prop_assert_eq!(&entity.role, &entity.path.role);
                prop_assert!(
                    entity.path.speed_factor.is_finite() && entity.path.speed_factor > 0.0
                );
                prop_assert!((1..=max_points).contains(&entity.path.points.len()));
                for point in &entity.path.points {
                    prop_assert!(point.x.is_finite() && point.y.is_finite() && point.z.is_finite());
                    prop_assert!(
                        point.width.is_finite()
                            && point.width > 0.0
                            && point.flow_factor.is_finite()
                            && point.flow_factor > 0.0
                    );
                }
            }
        }
        Ok(())
    })
    .expect("IR domain contract");
}

fn geometry_class_witness(case: &GeometryCase, class: TopologyClass) -> bool {
    let turns = |ring: &Polygon| -> Vec<i128> {
        let n = ring.points.len();
        (0..n)
            .map(|i| {
                let a = ring.points[i];
                let b = ring.points[(i + 1) % n];
                let c = ring.points[(i + 2) % n];
                i128::from(b.x - a.x) * i128::from(c.y - a.y)
                    - i128::from(b.y - a.y) * i128::from(c.x - a.x)
            })
            .collect()
    };
    match class {
        TopologyClass::Empty => case.polygons.is_empty(),
        TopologyClass::Convex => case
            .polygons
            .iter()
            .all(|p| turns(&p.contour).iter().all(|&t| t >= 0)),
        TopologyClass::Concave => case
            .polygons
            .iter()
            .any(|p| turns(&p.contour).iter().any(|&t| t < 0)),
        TopologyClass::NonStarShaped => case.polygons.iter().any(|p| {
            let points = &p.contour.points;
            let edges: Vec<_> = points
                .iter()
                .zip(points.iter().cycle().skip(1))
                .take(points.len())
                .collect();
            // The two directed edges require x >= right and x <= left.
            edges.iter().any(|(a, b)| {
                a.x == b.x
                    && a.y > b.y
                    && edges
                        .iter()
                        .any(|(c, d)| c.x == d.x && c.y < d.y && a.x > c.x)
            })
        }),
        TopologyClass::Holed => case.polygons.iter().any(|p| !p.holes.is_empty()),
        TopologyClass::Disconnected => case.polygons.len() >= 2,
        TopologyClass::Thin | TopologyClass::NearDegenerate => case.polygons.iter().any(|p| {
            let points = &p.contour.points;
            let width = points.iter().map(|p| p.x).max().unwrap()
                - points.iter().map(|p| p.x).min().unwrap();
            let height = points.iter().map(|p| p.y).max().unwrap()
                - points.iter().map(|p| p.y).min().unwrap();
            let n = points.len();
            (class != TopologyClass::Thin || width.min(height) >= 16)
                && (0..n).any(|i| {
                    (i + 1..n).any(|j| {
                        if j == i + 1 || (i == 0 && j == n - 1) {
                            return false;
                        }
                        let (a, b, c, d) = (
                            points[i],
                            points[(i + 1) % n],
                            points[j],
                            points[(j + 1) % n],
                        );
                        a.x == b.x
                            && c.x == d.x
                            && (a.x - c.x).abs() == 1
                            && a.y.min(b.y).max(c.y.min(d.y)) < a.y.max(b.y).min(c.y.max(d.y))
                    })
                })
        }),
    }
}

#[test]
fn proptest_direct_geometry_domains() {
    // A U-notch imposes x >= 8 and x <= 6 simultaneously; the rectangle
    // is convex. These explicit witnesses exercise the independent checks.
    let case = |points: &[(i64, i64)]| GeometryCase {
        polygons: vec![ExPolygon {
            contour: Polygon {
                points: points.iter().map(|&(x, y)| Point2 { x, y }).collect(),
            },
            holes: vec![],
        }],
    };
    let rectangle = case(&[(0, 0), (20, 0), (20, 20), (0, 20)]);
    let notch = case(&[
        (0, 0),
        (20, 0),
        (20, 20),
        (8, 20),
        (8, 3),
        (6, 3),
        (6, 20),
        (0, 20),
    ]);
    assert!(geometry_class_witness(&rectangle, TopologyClass::Convex));
    assert!(!geometry_class_witness(&notch, TopologyClass::Convex));
    assert!(geometry_class_witness(&notch, TopologyClass::NonStarShaped));
    assert!(!geometry_class_witness(
        &rectangle,
        TopologyClass::NonStarShaped
    ));

    let run = PropertyRun::new("D-GEOMETRY", "proptest_direct_geometry_domains")
        .expect("registry context");
    let max_abs_units = i64::try_from(run.bound("max_abs_units").expect("coordinate bound"))
        .expect("coordinate bound fits i64");
    let max_vertices = usize::try_from(run.bound("max_vertices").expect("vertex bound"))
        .expect("vertex bound fits usize");
    let max_holes = usize::try_from(run.bound("max_holes").expect("hole bound"))
        .expect("hole bound fits usize");
    let max_components = usize::try_from(run.bound("max_components").expect("component bound"))
        .expect("component bound fits usize");
    let max_shrink_candidates =
        u32::try_from(run.bound("max_shrink_candidates").expect("candidate bound"))
            .expect("candidate bound fits u32");
    let mut bounds = GeometryBounds {
        max_abs_units,
        max_vertices,
        max_holes,
        max_components,
        ..GeometryBounds::default()
    };
    bounds.max_shrink_candidates = max_shrink_candidates;
    let classes = [
        TopologyClass::Empty,
        TopologyClass::Convex,
        TopologyClass::Concave,
        TopologyClass::NonStarShaped,
        TopologyClass::Holed,
        TopologyClass::Disconnected,
        TopologyClass::Thin,
        TopologyClass::NearDegenerate,
    ];
    let branches: Vec<_> = classes
        .into_iter()
        .map(|class| {
            let domain = GeometryDomain {
                class,
                min_components: usize::from(class != TopologyClass::Empty),
                bounds: bounds.clone(),
                ..GeometryDomain::default()
            };
            geometry_cases(domain.clone())
                .expect("geometry profile fits each class")
                .prop_map(move |case| (domain.clone(), case))
                .boxed()
        })
        .collect();
    let strategy = proptest::strategy::Union::new(branches);
    run.check(&strategy, |(domain, case), _| {
        prop_assert!(validate_geometry(&case, &domain).is_ok());
        prop_assert!(
            geometry_class_witness(&case, domain.class),
            "missing {:?} witness: {:?}",
            domain.class,
            case
        );
        Ok(())
    })
    .expect("geometry domain contract");
}

#[test]
fn proptest_direct_mesh_domains() {
    // The rectangular solid is 2*3*4=24 mm³; the prism is (3*4/2)*5=30.
    let box_mesh = property_mesh::worked_box();
    let prism_mesh = property_mesh::worked_prism();
    assert_eq!(validate_indexed_mesh(&box_mesh, true), Ok(()));
    assert_eq!(mesh_signed_volume_mm3(&box_mesh), Ok(24.0));
    assert_eq!(validate_indexed_mesh(&prism_mesh, true), Ok(()));
    assert_eq!(mesh_signed_volume_mm3(&prism_mesh), Ok(30.0));
    let mut missing_triangle = box_mesh;
    missing_triangle
        .indices
        .truncate(missing_triangle.indices.len() - 3);
    assert_eq!(validate_indexed_mesh(&missing_triangle, false), Ok(()));
    assert!(
        matches!(validate_indexed_mesh(&missing_triangle, true), Err(DomainError::InvalidInput(ref s)) if s.contains("closed edge"))
    );

    let run = PropertyRun::new("D-MESH", "proptest_direct_mesh_domains").expect("registry context");
    let extent = run.bound("max_extent_mm").expect("mesh extent bound");
    let translation = run.bound("max_translation_mm").expect("translation bound");
    // u16 conversion is checked, then its entire range converts exactly to f32;
    // the constructors reject values over their actual 10,000 mm limit.
    let extent = f32::from(u16::try_from(extent).expect("mesh extent bound fits u16"));
    let translation = f32::from(u16::try_from(translation).expect("translation bound fits u16"));
    let strategy = (
        analytical_meshes(SolidKind::Box, extent).expect("box extent bound"),
        analytical_meshes(SolidKind::TriangularPrism, extent).expect("prism extent bound"),
        rigid_transforms(translation).expect("translation bound"),
    );
    run.check(&strategy, |(cube, prism, transform), _| {
        property_mesh::check_mesh(&cube, SolidKind::Box, extent)?;
        property_mesh::check_mesh(&prism, SolidKind::TriangularPrism, extent)?;
        property_mesh::check_transform(&transform, translation)?;
        property_mesh::check_transformed_mesh(&cube, SolidKind::Box, &transform, extent)?;
        property_mesh::check_transformed_mesh(
            &prism,
            SolidKind::TriangularPrism,
            &transform,
            extent,
        )?;
        Ok(())
    })
    .expect("mesh domain contract");
}
