//! Deterministic domain and independent-oracle counterexamples.

use std::collections::BTreeSet;
use std::fmt::Debug;

use proptest::prelude::*;
use proptest::strategy::ValueTree;
use proptest::test_runner::{Config, RngAlgorithm, RngSeed, TestRunner};
use slicer_ir::{
    mm_to_units, BoundingBox3, ConfigValue, ExPolygon, ExtrusionPath3D, ExtrusionRole,
    IndexedTriangleSet, LayerCollectionIR, PaintValue, Point2, Point3, Point3WithWidth, Polygon,
    PrintEntity, RegionKey, SemVer, TravelMove,
};
use slicer_sdk::test_support::fixtures::print_entity_base;
use slicer_sdk::test_support::property_testing::domains::*;
use slicer_sdk::test_support::property_testing::oracles::*;
use slicer_sdk::test_support::property_testing::regression::{
    format_regression_fixture, ConcreteInput,
};

#[path = "support/property_mesh.rs"]
mod property_mesh;

#[path = "support/regression_encode.rs"]
mod regression_encode;

fn runner() -> TestRunner {
    TestRunner::new(Config {
        cases: 128,
        rng_algorithm: RngAlgorithm::ChaCha,
        rng_seed: RngSeed::Fixed(0x728f_3ca1),
        failure_persistence: None,
        ..Config::default()
    })
}

fn visit<S: Strategy>(strategy: &S, cases: usize, mut check: impl FnMut(&S::Value)) {
    let mut runner = runner();
    for _ in 0..cases {
        let mut tree = strategy.new_tree(&mut runner).expect("case generation");
        check(&tree.current());
        for _ in 0..64 {
            let changed = tree.simplify();
            check(&tree.current());
            if !changed {
                break;
            }
        }
        for _ in 0..64 {
            let changed = tree.complicate();
            check(&tree.current());
            if !changed {
                break;
            }
        }
    }
}

fn sample<S: Strategy>(strategy: &S, cases: usize) -> Vec<S::Value> {
    let mut runner = runner();
    (0..cases)
        .map(|_| {
            strategy
                .new_tree(&mut runner)
                .expect("case generation")
                .current()
        })
        .collect()
}

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

fn assert_invalid_input<T: Debug>(result: Result<T, DomainError>) {
    assert!(matches!(result, Err(DomainError::InvalidInput(_))));
}

fn assert_invalid_bounds<T: Debug>(result: Result<T, DomainError>) {
    assert!(matches!(result, Err(DomainError::InvalidBounds(_))));
}

fn fixture_layer(entities: &[u64], anchors: &[u64]) -> LayerCollectionIR {
    LayerCollectionIR {
        ordered_entities: entities
            .iter()
            .map(|&entity_id| PrintEntity {
                entity_id,
                ..print_entity_base(ExtrusionRole::OuterWall)
            })
            .collect(),
        travel_moves: anchors
            .iter()
            .map(|&entity_id| TravelMove {
                entity_id,
                ..Default::default()
            })
            .collect(),
        ..Default::default()
    }
}

#[test]
fn domain_scalar_edges_and_config_types() {
    let f32_range = -1.0e-40_f32..=1.0e-40_f32;
    let f32_strategy = finite_f32(f32_range.clone()).unwrap();
    let mut bits32 = BTreeSet::new();
    visit(&f32_strategy, 256, |value| {
        assert!(value.is_finite() && f32_range.contains(value));
        bits32.insert(value.to_bits());
    });
    for bits in [
        0.0_f32.to_bits(),
        (-0.0_f32).to_bits(),
        1,
        0x8000_0001,
        f32_range.start().to_bits(),
        f32_range.end().to_bits(),
    ] {
        assert!(bits32.contains(&bits), "unreachable f32 witness {bits:x}");
    }
    let f64_range = -1.0e-310_f64..=1.0e-310_f64;
    let f64_strategy = finite_f64(f64_range.clone()).unwrap();
    let mut bits64 = BTreeSet::new();
    visit(&f64_strategy, 256, |value| {
        assert!(value.is_finite() && f64_range.contains(value));
        bits64.insert(value.to_bits());
    });
    for bits in [
        0.0_f64.to_bits(),
        (-0.0_f64).to_bits(),
        1,
        0x8000_0000_0000_0001,
        f64_range.start().to_bits(),
        f64_range.end().to_bits(),
    ] {
        assert!(bits64.contains(&bits), "unreachable f64 witness {bits:x}");
    }
    for range in [-f32::MAX..=f32::MAX, 12.0..=12.0, 3.0..=9.0] {
        visit(&finite_f32(range.clone()).unwrap(), 48, |v| {
            assert!(v.is_finite() && range.contains(v))
        });
    }
    for range in [-f64::MAX..=f64::MAX, -12.0..=-12.0, -9.0..=-3.0] {
        visit(&finite_f64(range.clone()).unwrap(), 48, |v| {
            assert!(v.is_finite() && range.contains(v))
        });
    }

    let exceptional32 = sample(&nonfinite_f32(), 256);
    let exceptional64 = sample(&nonfinite_f64(), 256);
    assert!(exceptional32.iter().all(|v| !v.is_finite()));
    assert!(exceptional64.iter().all(|v| !v.is_finite()));
    assert!(exceptional32.contains(&f32::INFINITY) && exceptional32.contains(&f32::NEG_INFINITY));
    assert!(exceptional64.contains(&f64::INFINITY) && exceptional64.contains(&f64::NEG_INFINITY));
    assert!(
        exceptional32
            .iter()
            .filter(|v| v.is_nan())
            .map(|v| v.to_bits())
            .collect::<BTreeSet<_>>()
            .len()
            >= 2
    );
    assert!(
        exceptional64
            .iter()
            .filter(|v| v.is_nan())
            .map(|v| v.to_bits())
            .collect::<BTreeSet<_>>()
            .len()
            >= 2
    );

    let boundary = (0.5_f64 / 10_000.0) as f32;
    let boundary_bits: BTreeSet<_> = sample(&rounding_boundary_mm(1).unwrap(), 256)
        .into_iter()
        .map(f32::to_bits)
        .collect();
    for center in [boundary, -boundary] {
        for value in [center.next_down(), center, center.next_up()] {
            assert!(
                boundary_bits.contains(&value.to_bits()),
                "missing boundary neighbor {value:?}"
            );
        }
    }
    assert_eq!(mm_to_units(boundary.next_down()), 0);
    assert_eq!(mm_to_units(boundary.next_up()), 1);
    assert_eq!(mm_to_units((-boundary).next_down()), -1);
    assert_eq!(mm_to_units((-boundary).next_up()), 0);

    for domain in [
        ScalarConfigDomain::Bool,
        ScalarConfigDomain::Int { min: -2, max: 3 },
        ScalarConfigDomain::Float {
            min: -1.0,
            max: 2.0,
        },
        ScalarConfigDomain::Percent {
            min: -1.0,
            max: 2.0,
        },
        ScalarConfigDomain::FloatOrPercent {
            min: -1.0,
            max: 2.0,
            is_percent: true,
        },
        ScalarConfigDomain::FloatOrPercent {
            min: -1.0,
            max: 2.0,
            is_percent: false,
        },
        ScalarConfigDomain::StringChoices(vec!["grid".into(), "lines".into()]),
    ] {
        let strategy = scalar_config_values(domain.clone()).unwrap();
        visit(&strategy, 64, |value| match (&domain, value) {
            (ScalarConfigDomain::Bool, ConfigValue::Bool(_)) => (),
            (ScalarConfigDomain::Int { min, max }, ConfigValue::Int(v)) => {
                assert!((*min..=*max).contains(v))
            }
            (ScalarConfigDomain::Float { min, max }, ConfigValue::Float(v))
            | (ScalarConfigDomain::Percent { min, max }, ConfigValue::Percent(v)) => {
                assert!(v.is_finite() && (*min..=*max).contains(v))
            }
            (
                ScalarConfigDomain::FloatOrPercent {
                    min,
                    max,
                    is_percent,
                },
                ConfigValue::FloatOrPercent {
                    value,
                    is_percent: actual,
                },
            ) => {
                assert_eq!(is_percent, actual);
                assert!(value.is_finite() && (*min..=*max).contains(value));
            }
            (ScalarConfigDomain::StringChoices(choices), ConfigValue::String(s)) => {
                assert!(choices.contains(s))
            }
            _ => panic!("wrong config variant: {domain:?} / {value:?}"),
        });
    }
    for (domain, positive, negative) in [
        (
            ScalarConfigDomain::Float { min: 0.0, max: 0.0 },
            ConfigValue::Float(0.0),
            ConfigValue::Float(-0.0),
        ),
        (
            ScalarConfigDomain::Percent { min: 0.0, max: 0.0 },
            ConfigValue::Percent(0.0),
            ConfigValue::Percent(-0.0),
        ),
        (
            ScalarConfigDomain::FloatOrPercent {
                min: 0.0,
                max: 0.0,
                is_percent: false,
            },
            ConfigValue::FloatOrPercent {
                value: 0.0,
                is_percent: false,
            },
            ConfigValue::FloatOrPercent {
                value: -0.0,
                is_percent: false,
            },
        ),
        (
            ScalarConfigDomain::FloatOrPercent {
                min: 0.0,
                max: 0.0,
                is_percent: true,
            },
            ConfigValue::FloatOrPercent {
                value: 0.0,
                is_percent: true,
            },
            ConfigValue::FloatOrPercent {
                value: -0.0,
                is_percent: true,
            },
        ),
    ] {
        let zeros = sample(&scalar_config_values(domain).unwrap(), 128);
        assert!(zeros.contains(&positive) && zeros.contains(&negative));
    }

    let uuids = sample(&id_text(IdTextKind::UuidSyntax, 36).unwrap(), 256);
    assert!(uuids.iter().all(|s| uuid_syntax(s)));
    assert!(uuids
        .iter()
        .any(|s| s == "00000000-0000-0000-0000-000000000000"));
    assert!(uuids
        .iter()
        .any(|s| s == "ffffffff-ffff-ffff-ffff-ffffffffffff"));
    let others = sample(&id_text(IdTextKind::OtherText, 40).unwrap(), 256);
    assert!(others.iter().all(|s| s.len() <= 40 && !uuid_syntax(s)));
    assert!(others.iter().any(String::is_empty));
    assert!(others.iter().any(|s| !s.is_ascii()));
    assert!(others.iter().any(|s| s == "!"));
    assert!(others
        .iter()
        .any(|s| s == "FFFFFFFF-FFFF-FFFF-FFFF-FFFFFFFFFFFF"));
    assert!(others
        .iter()
        .any(|s| s == "00000000_0000-0000-0000-000000000000"));
    visit(&id_text(IdTextKind::OtherText, 36).unwrap(), 64, |s| {
        assert!(s.len() <= 36 && !uuid_syntax(s))
    });
    visit(&id_text(IdTextKind::OtherText, 0).unwrap(), 8, |s| {
        assert!(s.is_empty())
    });
    visit(&id_text(IdTextKind::OtherText, 1).unwrap(), 32, |s| {
        assert!(s.len() <= 1 && !uuid_syntax(s))
    });
}

#[test]
fn domain_graph_models_and_cycle_tail() {
    let dag = SmallGraph {
        node_ids: vec![0, 1, 2, 3],
        edges: vec![(0, 1), (0, 2), (1, 2), (2, 3)],
    };
    assert_eq!(
        graph_reachability(&dag).unwrap(),
        BTreeSet::from([(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)])
    );
    assert!(graph_cycle_remainder(&dag).unwrap().is_empty());
    let cycle = SmallGraph {
        node_ids: vec![0, 1, 2, 3, 4],
        edges: vec![(0, 1), (1, 0), (1, 2), (2, 3)],
    };
    assert_eq!(
        graph_reachability(&cycle).unwrap(),
        BTreeSet::from([
            (0, 0),
            (0, 1),
            (0, 2),
            (0, 3),
            (1, 0),
            (1, 1),
            (1, 2),
            (1, 3),
            (2, 3)
        ])
    );
    assert_eq!(
        graph_cycle_remainder(&cycle).unwrap(),
        BTreeSet::from([0, 1, 2, 3])
    );
    assert_invalid_input(graph_reachability(&SmallGraph {
        node_ids: vec![1, 1],
        edges: vec![],
    }));
    assert_invalid_input(graph_cycle_remainder(&SmallGraph {
        node_ids: vec![1],
        edges: vec![(1, 2)],
    }));

    visit(&graphs(GraphKind::Dag, 9).unwrap(), 64, |g| {
        let nodes: BTreeSet<_> = g.node_ids.iter().copied().collect();
        assert_eq!(nodes.len(), g.node_ids.len());
        let rank = |id| {
            g.node_ids
                .iter()
                .position(|&n| n == id)
                .expect("known endpoint")
        };
        for &(a, b) in &g.edges {
            assert!(rank(a) < rank(b));
        }
        for &(a, b) in &graph_reachability(g).unwrap() {
            assert!(rank(a) < rank(b));
        }
        assert!(graph_cycle_remainder(g).unwrap().is_empty());
    });
    visit(&graphs(GraphKind::CycleWithTail, 9).unwrap(), 64, |g| {
        let paths = graph_reachability(g).unwrap();
        let cycle_nodes: Vec<_> = g
            .node_ids
            .iter()
            .copied()
            .filter(|&n| paths.contains(&(n, n)))
            .collect();
        assert!(cycle_nodes.len() >= 2);
        assert!(g.node_ids.iter().any(|&tail| !paths.contains(&(tail, tail))
            && cycle_nodes.iter().any(|&n| paths.contains(&(n, tail)))));
    });
    for kind in [
        MalformedGraphKind::DuplicateNode,
        MalformedGraphKind::UnknownEndpoint,
    ] {
        let strategy = malformed_graphs(kind, 8).unwrap();
        let mut runner = runner();
        let mut reductions = 0;
        let assert_fault = |graph: &SmallGraph| {
            match kind {
                MalformedGraphKind::DuplicateNode => assert!(graph.node_ids
                    [..graph.node_ids.len() - 1]
                    .contains(graph.node_ids.last().unwrap())),
                MalformedGraphKind::UnknownEndpoint => {
                    assert!(!graph.node_ids.contains(&graph.edges.last().unwrap().1))
                }
            }
            assert_invalid_input(graph_reachability(graph));
        };
        for _ in 0..32 {
            let mut tree = strategy.new_tree(&mut runner).unwrap();
            loop {
                assert_fault(&tree.current());
                if !tree.simplify() {
                    break;
                }
                reductions += 1;
            }
            loop {
                let changed = tree.complicate();
                assert_fault(&tree.current());
                if !changed {
                    break;
                }
            }
        }
        assert!(
            reductions > 0,
            "the malformed witness must be tested through actual shrinks"
        );
    }
}

#[test]
fn domain_ir_anchors_permutations_and_layers() {
    assert_eq!(
        first_missing_anchor(&fixture_layer(&[0, u64::MAX], &[u64::MAX, 0, u64::MAX])),
        None
    );
    assert_eq!(
        first_missing_anchor(&fixture_layer(&[7], &[7, 99, 2])),
        Some(99)
    );
    assert_eq!(
        first_missing_anchor(&fixture_layer(&[], &[u64::MAX])),
        Some(u64::MAX)
    );
    assert!(!is_permutation(&[0, 0, 2], 3));
    assert!(!is_permutation(&[0, 1, 3], 3));
    assert!(locked_groups_are_contiguous(
        &[4, 3, 1, 2, 0],
        &[vec![3, 1, 2]]
    ));
    assert!(!locked_groups_are_contiguous(
        &[4, 1, 2, 3, 0],
        &[vec![3, 1, 2]]
    ));

    let mut saw_extremes = BTreeSet::new();
    let mut saw_repeated = false;
    let mut saw_empty = false;
    for mode in [AnchorDomain::Valid, AnchorDomain::Dangling] {
        visit(&anchor_cases(mode, 6, 6).unwrap(), 128, |case| {
            assert_eq!(
                first_missing_anchor(&case.layer).is_some(),
                mode == AnchorDomain::Dangling
            );
            assert!(case.layer.ordered_entities.len() <= 6 && case.layer.travel_moves.len() <= 6);
            saw_empty |=
                case.layer.ordered_entities.is_empty() && case.layer.travel_moves.is_empty();
            for entity in &case.layer.ordered_entities {
                if entity.entity_id == 0 || entity.entity_id == u64::MAX {
                    saw_extremes.insert(entity.entity_id);
                }
            }
            saw_repeated |= case
                .layer
                .travel_moves
                .windows(2)
                .any(|pair| pair[0].entity_id == pair[1].entity_id);
        });
    }
    assert_eq!(saw_extremes, BTreeSet::from([0, u64::MAX]));
    assert!(saw_repeated && saw_empty);
    for len in [0, 1, 5, 12] {
        visit(&permutations(len), 32, |order| {
            assert!(is_permutation(order, len))
        });
    }
    drop(permutations(usize::MAX));
    let groups = vec![vec![3, 1, 2], vec![6, 4]];
    visit(&locked_orders(8, groups.clone()).unwrap(), 64, |order| {
        assert!(is_permutation(order, 8));
        assert!(locked_groups_are_contiguous(order, &groups));
    });
    let domain = EntityDomain {
        roles: vec![ExtrusionRole::OuterWall, ExtrusionRole::SparseInfill],
        max_entities: 5,
        max_points: 4,
        max_layers: 4,
    };
    let mut nonempty_entities = 0;
    let mut multilayer = 0;
    visit(&entity_layers(domain.clone()).unwrap(), 128, |layers| {
        assert!(layers.len() <= domain.max_layers);
        if layers.len() >= 2 {
            multilayer += 1;
        }
        let mut previous_z = None;
        let mut previous_id = 0;
        for (index, layer) in layers.iter().enumerate() {
            assert_eq!(layer.global_layer_index as usize, index);
            assert!(layer.z.is_finite() && previous_z.is_none_or(|z| layer.z > z));
            previous_z = Some(layer.z);
            assert!(layer.ordered_entities.len() <= domain.max_entities);
            for entity in &layer.ordered_entities {
                nonempty_entities += 1;
                assert!(entity.entity_id > previous_id);
                previous_id = entity.entity_id;
                assert!(domain.roles.contains(&entity.role));
                assert_eq!(entity.path.role, entity.role);
                assert!(entity.path.speed_factor.is_finite() && entity.path.speed_factor > 0.0);
                assert!((1..=domain.max_points).contains(&entity.path.points.len()));
                for p in &entity.path.points {
                    assert!(p.x.is_finite() && p.y.is_finite() && p.z.is_finite());
                    assert!(
                        p.width.is_finite()
                            && p.width > 0.0
                            && p.flow_factor.is_finite()
                            && p.flow_factor > 0.0
                    );
                    assert_eq!((p.x * 8.0).fract(), 0.0);
                    assert_eq!((p.y * 8.0).fract(), 0.0);
                    assert_eq!((p.z * 8.0).fract(), 0.0);
                }
            }
        }
    });
    assert!(nonempty_entities > 0 && multilayer > 0);
    let empty = EntityDomain {
        max_points: 0,
        ..domain
    };
    visit(&entity_layers(empty).unwrap(), 32, |layers| {
        assert!(layers.iter().all(|layer| layer.ordered_entities.is_empty()))
    });
}

#[test]
fn domain_rejects_invalid_bounds_and_witness_loss() {
    for range in [f32::NAN..=1.0, 0.0..=f32::INFINITY, 2.0..=1.0] {
        assert_invalid_bounds(finite_f32(range));
    }
    for range in [f64::NEG_INFINITY..=1.0, 0.0..=f64::NAN, 2.0..=1.0] {
        assert_invalid_bounds(finite_f64(range));
    }
    assert_invalid_bounds(rounding_boundary_mm(0));
    assert_invalid_bounds(rounding_boundary_mm(i64::MAX));
    assert_invalid_bounds(scalar_config_values(ScalarConfigDomain::Int {
        min: 2,
        max: 1,
    }));
    assert_invalid_bounds(scalar_config_values(ScalarConfigDomain::Float {
        min: f64::NAN,
        max: 1.0,
    }));
    assert_invalid_bounds(scalar_config_values(ScalarConfigDomain::Percent {
        min: 2.0,
        max: 1.0,
    }));
    assert_invalid_bounds(scalar_config_values(ScalarConfigDomain::FloatOrPercent {
        min: 0.0,
        max: f64::INFINITY,
        is_percent: false,
    }));
    assert_invalid_input(scalar_config_values(ScalarConfigDomain::StringChoices(
        vec![],
    )));
    assert_invalid_bounds(id_text(IdTextKind::UuidSyntax, 35));
    assert_invalid_bounds(id_text(IdTextKind::OtherText, usize::MAX));
    assert_invalid_bounds(graphs(GraphKind::Dag, 0));
    assert_invalid_bounds(graphs(GraphKind::Dag, u16::MAX));
    assert_invalid_bounds(graphs(GraphKind::CycleWithTail, 2));
    assert_invalid_bounds(malformed_graphs(MalformedGraphKind::DuplicateNode, 1));
    assert_invalid_bounds(anchor_cases(AnchorDomain::Dangling, 4, 0));
    assert_invalid_bounds(anchor_cases(AnchorDomain::Valid, usize::MAX, 1));
    assert_invalid_input(entity_layers(EntityDomain {
        roles: vec![],
        max_entities: 1,
        max_points: 1,
        max_layers: 1,
    }));
    assert_invalid_bounds(entity_layers(EntityDomain {
        roles: vec![ExtrusionRole::OuterWall],
        max_entities: 1,
        max_points: 1,
        max_layers: usize::MAX,
    }));
    assert_invalid_input(locked_orders(2, vec![vec![]]));
    assert_invalid_input(locked_orders(3, vec![vec![0, 1], vec![1, 2]]));
    assert_invalid_input(locked_orders(2, vec![vec![0, 2]]));
    assert_invalid_input(locked_orders(1, vec![vec![0], vec![0]]));

    // (MAX - MIN)^2 exceeds i128::MAX; the square's double area does too.
    let min = i64::MIN;
    let max = i64::MAX;
    assert_eq!(
        orientation(point(min, min), point(max, min), point(min, max)),
        Err(DomainError::ArithmeticOverflow)
    );
    assert_eq!(
        signed_area2(&ring(&[(min, min), (max, min), (max, max), (min, max)])),
        Err(DomainError::ArithmeticOverflow)
    );
    let empty_case = GeometryCase { polygons: vec![] };
    // Convex excludes a required hole; Empty excludes any component minimum.
    let bad_domains = [
        GeometryDomain {
            min_holes: 1,
            ..GeometryDomain::default()
        },
        GeometryDomain {
            class: TopologyClass::Empty,
            min_components: 1,
            ..GeometryDomain::default()
        },
        GeometryDomain {
            min_components: 0,
            ..GeometryDomain::default()
        },
        GeometryDomain {
            bounds: GeometryBounds {
                max_abs_units: 1_000_001,
                ..GeometryBounds::default()
            },
            ..GeometryDomain::default()
        },
        GeometryDomain {
            bounds: GeometryBounds {
                max_vertices: 0,
                ..GeometryBounds::default()
            },
            ..GeometryDomain::default()
        },
        GeometryDomain {
            bounds: GeometryBounds {
                max_holes: 0,
                ..GeometryBounds::default()
            },
            ..GeometryDomain::default()
        },
        GeometryDomain {
            bounds: GeometryBounds {
                max_components: 0,
                ..GeometryBounds::default()
            },
            ..GeometryDomain::default()
        },
        GeometryDomain {
            class: TopologyClass::Holed,
            min_holes: 2,
            bounds: GeometryBounds {
                max_holes: 1,
                ..GeometryBounds::default()
            },
            ..GeometryDomain::default()
        },
        GeometryDomain {
            class: TopologyClass::Disconnected,
            bounds: GeometryBounds {
                max_components: 1,
                ..GeometryBounds::default()
            },
            ..GeometryDomain::default()
        },
        // A triangle plus a hole needs at least six total vertices.
        GeometryDomain {
            class: TopologyClass::Holed,
            bounds: GeometryBounds {
                max_vertices: 5,
                ..GeometryBounds::default()
            },
            ..GeometryDomain::default()
        },
        // Every simple ring of at most five vertices has a nonempty kernel.
        GeometryDomain {
            class: TopologyClass::NonStarShaped,
            bounds: GeometryBounds {
                max_vertices: 5,
                ..GeometryBounds::default()
            },
            ..GeometryDomain::default()
        },
        // At |x|,|y| <= 7 the bounding-box dimension cannot reach 16.
        GeometryDomain {
            class: TopologyClass::Thin,
            bounds: GeometryBounds {
                max_abs_units: 7,
                ..GeometryBounds::default()
            },
            ..GeometryDomain::default()
        },
        // Four components need 12 distinct vertices; a [-1,1]^2 grid has 9 points.
        GeometryDomain {
            class: TopologyClass::Disconnected,
            min_components: 4,
            bounds: GeometryBounds {
                max_abs_units: 1,
                ..GeometryBounds::default()
            },
            ..GeometryDomain::default()
        },
    ];
    for domain in bad_domains {
        assert_invalid_bounds(validate_geometry(&empty_case, &domain));
        assert_invalid_bounds(geometry_cases(domain));
    }
    if let Ok(excessive) = usize::try_from(u64::from(u32::MAX) + 1) {
        // This bound cannot be narrowed portably to the documented u32 limit.
        let domain = GeometryDomain {
            bounds: GeometryBounds {
                max_vertices: excessive,
                ..GeometryBounds::default()
            },
            ..GeometryDomain::default()
        };
        assert_invalid_bounds(validate_geometry(&empty_case, &domain));
        assert_invalid_bounds(geometry_cases(domain));
    }
    // The catalog needs eight vertices for a U, two rectangles, or one
    // rectangular hole, even when an unrelated smaller polygon could fit.
    for class in [
        TopologyClass::NonStarShaped,
        TopologyClass::Holed,
        TopologyClass::Disconnected,
    ] {
        assert_invalid_bounds(geometry_cases(GeometryDomain {
            class,
            bounds: GeometryBounds {
                max_vertices: 7,
                ..GeometryBounds::default()
            },
            ..GeometryDomain::default()
        }));
    }
    assert_invalid_bounds(geometry_cases(GeometryDomain {
        class: TopologyClass::Concave,
        bounds: GeometryBounds {
            max_vertices: 5,
            ..GeometryBounds::default()
        },
        ..GeometryDomain::default()
    }));
    assert_invalid_bounds(geometry_cases(GeometryDomain {
        class: TopologyClass::Disconnected,
        min_components: 3,
        bounds: GeometryBounds {
            max_abs_units: 1,
            ..GeometryBounds::default()
        },
        ..GeometryDomain::default()
    }));
    for limit in [0, 1_000_001] {
        let bounds = GeometryBounds {
            max_abs_units: limit,
            ..GeometryBounds::default()
        };
        assert_invalid_bounds(line_segments(bounds.clone()));
        assert_invalid_bounds(invalid_geometry_cases(
            InvalidGeometryKind::Crossing,
            bounds,
        ));
    }
    assert_invalid_bounds(invalid_geometry_cases(
        InvalidGeometryKind::TouchingHole,
        GeometryBounds {
            max_abs_units: 1,
            ..GeometryBounds::default()
        },
    ));
    // Widen before abs: i64::MIN must contribute 2^63, not wrap.
    let extreme = geometry(vec![component(
        ring(&[(i64::MIN, 0), (0, i64::MIN), (i64::MAX, 1)]),
        vec![],
    )]);
    assert_eq!(
        geometry_complexity(&extreme),
        Ok((1, 0, 3, (1_u128 << 63) * 3))
    );
}

fn point(x: i64, y: i64) -> Point2 {
    Point2 { x, y }
}

fn ring(points: &[(i64, i64)]) -> Polygon {
    Polygon {
        points: points.iter().map(|&(x, y)| point(x, y)).collect(),
    }
}

fn component(contour: Polygon, holes: Vec<Polygon>) -> ExPolygon {
    ExPolygon { contour, holes }
}

fn geometry(polygons: Vec<ExPolygon>) -> GeometryCase {
    GeometryCase { polygons }
}

fn class(class: TopologyClass) -> GeometryDomain {
    GeometryDomain {
        class,
        ..GeometryDomain::default()
    }
}

#[test]
fn domain_exact_predicates_counterexamples() {
    use BoundaryRule::{Inclusive, Strict};
    use PointLocation::{Boundary, Inside, Outside};
    use SegmentRelation::{CollinearOverlap, Disjoint, EndpointTouch, ProperCrossing};

    // (4, 0) × (1, 3) = 12, the reverse is -12, and (4, 0) × (2, 0) = 0.
    assert_eq!(orientation(point(0, 0), point(4, 0), point(1, 3)), Ok(1));
    assert_eq!(orientation(point(0, 0), point(1, 3), point(4, 0)), Ok(-1));
    assert_eq!(orientation(point(0, 0), point(4, 0), point(2, 0)), Ok(0));
    // The extreme determinant is (2^64 - 1)^2, too large for i128.
    assert_eq!(
        orientation(
            point(i64::MIN, i64::MIN),
            point(i64::MAX, i64::MIN),
            point(i64::MIN, i64::MAX)
        ),
        Err(DomainError::ArithmeticOverflow)
    );

    let square = ring(&[(0, 0), (10, 0), (10, 10), (0, 10)]);
    let reversed_square = ring(&[(0, 10), (10, 10), (10, 0), (0, 0)]);
    // Shoelace sums: 100 + 100 = 200; reversing changes the sign.
    assert_eq!(signed_area2(&square), Ok(200));
    assert_eq!(signed_area2(&reversed_square), Ok(-200));
    // The right triangle has double area 4 * 6 = 24; short sums are zero.
    assert_eq!(signed_area2(&ring(&[(0, 0), (4, 0), (0, 6)])), Ok(24));
    assert_eq!(signed_area2(&ring(&[])), Ok(0));
    assert_eq!(signed_area2(&ring(&[(2, 3), (4, 5)])), Ok(0));
    // Four extreme square terms overflow their checked accumulated sum.
    assert_eq!(
        signed_area2(&ring(&[
            (i64::MIN, i64::MIN),
            (i64::MAX, i64::MIN),
            (i64::MAX, i64::MAX),
            (i64::MIN, i64::MAX)
        ])),
        Err(DomainError::ArithmeticOverflow)
    );

    let horizontal = [point(0, 0), point(10, 0)];
    // A vertical interior crossing, a separated line, and a positive overlap.
    assert_eq!(
        segment_relation(horizontal, [point(4, -2), point(4, 2)]),
        Ok(ProperCrossing)
    );
    assert_eq!(
        segment_relation(horizontal, [point(0, 1), point(10, 1)]),
        Ok(Disjoint)
    );
    assert_eq!(
        segment_relation(horizontal, [point(3, 0), point(12, 0)]),
        Ok(CollinearOverlap)
    );
    // One shared collinear endpoint and one non-collinear T-junction.
    assert_eq!(
        segment_relation(horizontal, [point(10, 0), point(12, 0)]),
        Ok(EndpointTouch)
    );
    assert_eq!(
        segment_relation(horizontal, [point(5, 0), point(5, 5)]),
        Ok(EndpointTouch)
    );
    // A point on the segment is a touch, otherwise disjoint, including point/point.
    assert_eq!(
        segment_relation(horizontal, [point(5, 0), point(5, 0)]),
        Ok(EndpointTouch)
    );
    assert_eq!(
        segment_relation(horizontal, [point(5, 1), point(5, 1)]),
        Ok(Disjoint)
    );
    assert_eq!(
        segment_relation([point(2, 3); 2], [point(2, 3); 2]),
        Ok(EndpointTouch)
    );
    assert_eq!(
        segment_relation([point(2, 3); 2], [point(3, 3); 2]),
        Ok(Disjoint)
    );
    // Diagonals across the full i64 range cross, even though i128 orientation overflows.
    assert_eq!(
        segment_relation(
            [point(i64::MIN, i64::MIN), point(i64::MAX, i64::MAX)],
            [point(i64::MIN, i64::MAX), point(i64::MAX, i64::MIN)]
        ),
        Ok(ProperCrossing)
    );

    // Rectangle and triangle adjacent edges share just their cyclic endpoint.
    assert_eq!(validate_ring(&square), Ok(()));
    assert_eq!(validate_ring(&ring(&[(0, 0), (4, 0), (0, 6)])), Ok(()));
    // Consecutive collinear forward edges are permitted; a backtrack overlaps.
    assert_eq!(
        validate_ring(&ring(&[(0, 0), (5, 0), (10, 0), (10, 10), (0, 10)])),
        Ok(())
    );
    assert_invalid_input(validate_ring(&ring(&[
        (0, 0),
        (6, 0),
        (2, 0),
        (2, 4),
        (0, 4),
    ])));
    // Repeated closure, collapsed area, fewer than three, and separate crossings.
    assert_invalid_input(validate_ring(&ring(&[(0, 0), (10, 0), (10, 10), (0, 0)])));
    assert_invalid_input(validate_ring(&ring(&[(0, 0), (1, 0), (2, 0)])));
    assert_invalid_input(validate_ring(&ring(&[(0, 0), (1, 0)])));
    // Bow-tie edges (0,0)-(6,6) and (0,6)-(6,0) cross; area2 = 12.
    let bow = ring(&[(0, 0), (6, 6), (0, 6), (6, 0), (9, 2)]);
    assert_eq!(signed_area2(&bow), Ok(12));
    assert_invalid_input(validate_ring(&bow));
    // Nonadjacent edges (6,0)-(2,6) and (6,6)-(0,2) cross internally; area2 = 24.
    let crossing = ring(&[(0, 0), (6, 0), (2, 6), (6, 6), (0, 2)]);
    assert_eq!(signed_area2(&crossing), Ok(24));
    assert_invalid_input(validate_ring(&crossing));
    // A simple extreme square cannot be certified after its area sum overflows.
    assert_eq!(
        validate_ring(&ring(&[
            (i64::MIN, i64::MIN),
            (i64::MAX, i64::MIN),
            (i64::MAX, i64::MAX),
            (i64::MIN, i64::MAX)
        ])),
        Err(DomainError::ArithmeticOverflow)
    );

    let outer = ring(&[(0, 0), (20, 0), (20, 20), (0, 20)]);
    // The clockwise hole is [3,5] × [4,6].
    let hole = ring(&[(3, 4), (3, 6), (5, 6), (5, 4)]);
    let holed = component(outer.clone(), vec![hole.clone()]);
    let region = vec![holed.clone()];
    assert_eq!(point_in_region(point(10, 10), &region), Ok(Inside));
    assert_eq!(point_in_region(point(21, 10), &region), Ok(Outside));
    assert_eq!(point_in_region(point(0, 10), &region), Ok(Boundary));
    assert_eq!(point_in_region(point(3, 5), &region), Ok(Boundary));
    assert_eq!(point_in_region(point(4, 5), &region), Ok(Outside));
    assert_eq!(point_in_region(point(10, 10), &[]), Ok(Outside));
    // Boundary-first means a second component's edge wins over another's interior.
    let overlapping_queries = [
        component(outer.clone(), vec![]),
        component(ring(&[(8, 8), (12, 8), (12, 12), (8, 12)]), vec![]),
    ];
    assert_eq!(
        point_in_region(point(8, 10), &overlapping_queries),
        Ok(Boundary)
    );

    // Interior segment (10,10)-(18,10) never touches any ring.
    for boundary in [Strict, Inclusive] {
        assert_eq!(
            segment_in_region([point(10, 10), point(18, 10)], &region, boundary),
            Ok(true)
        );
    }
    // The notch removes 6 < x < 8 at y=5; endpoints x=2,18 and midpoint x=10 remain inside.
    let notched = component(
        ring(&[
            (0, 0),
            (20, 0),
            (20, 10),
            (8, 10),
            (8, 3),
            (6, 3),
            (6, 10),
            (0, 10),
        ]),
        vec![],
    );
    let concave = vec![notched.clone()];
    for p in [point(2, 5), point(10, 5), point(18, 5)] {
        assert_eq!(point_in_region(p, &concave), Ok(Inside));
    }
    assert_eq!(point_in_region(point(7, 5), &concave), Ok(Outside));
    for boundary in [Strict, Inclusive] {
        assert_eq!(
            segment_in_region([point(2, 5), point(18, 5)], &concave, boundary),
            Ok(false)
        );
    }
    // The hole removes 3 < x < 5 at y=5; endpoints x=1,19 and midpoint x=10 are inside.
    for p in [point(1, 5), point(10, 5), point(19, 5)] {
        assert_eq!(point_in_region(p, &region), Ok(Inside));
    }
    for boundary in [Strict, Inclusive] {
        assert_eq!(
            segment_in_region([point(1, 5), point(19, 5)], &region, boundary),
            Ok(false)
        );
    }
    // At this i64-scale span the two-unit-wide hole's crossings are closer than
    // f64 can distinguish near t=1/2; exact rational partitioning still sees it.
    let extreme_region = vec![component(
        ring(&[
            (i64::MIN, -10),
            (i64::MAX, -10),
            (i64::MAX, 10),
            (i64::MIN, 10),
        ]),
        vec![ring(&[(1, -1), (1, 1), (3, 1), (3, -1)])],
    )];
    for p in [point(i64::MIN + 1, 0), point(0, 0), point(i64::MAX - 1, 0)] {
        assert_eq!(point_in_region(p, &extreme_region), Ok(Inside));
    }
    // The point (2,0) is in the hole, whereas both endpoints and t=1/2 are inside.
    assert_eq!(point_in_region(point(2, 0), &extreme_region), Ok(Outside));
    for boundary in [Strict, Inclusive] {
        assert_eq!(
            segment_in_region(
                [point(i64::MIN + 1, 0), point(i64::MAX - 1, 0)],
                &extreme_region,
                boundary,
            ),
            Ok(false)
        );
    }
    // The whole first segment lies on y=0; the second grazes hole corner (5,5) only.
    let tangent_hole = component(outer.clone(), vec![ring(&[(5, 5), (5, 7), (7, 7), (7, 5)])]);
    for (segment, polygons) in [
        ([point(1, 0), point(19, 0)], region.clone()),
        ([point(2, 8), point(8, 2)], vec![tangent_hole]),
    ] {
        assert_eq!(segment_in_region(segment, &polygons, Inclusive), Ok(true));
        assert_eq!(segment_in_region(segment, &polygons, Strict), Ok(false));
    }
    // An interior singleton is always included; a boundary singleton only inclusively.
    for boundary in [Strict, Inclusive] {
        assert_eq!(
            segment_in_region([point(10, 10); 2], &region, boundary),
            Ok(true)
        );
        assert_eq!(
            segment_in_region([point(21, 10); 2], &region, boundary),
            Ok(false)
        );
    }
    assert_eq!(
        segment_in_region([point(0, 10); 2], &region, Strict),
        Ok(false)
    );
    assert_eq!(
        segment_in_region([point(0, 10); 2], &region, Inclusive),
        Ok(true)
    );
    assert_eq!(
        segment_in_region([point(0, 0), point(1, 1)], &[], Inclusive),
        Ok(false)
    );

    // 0.00001 mm differs from zero by 0.1 unit: both convert to (0,0).
    // Pre-conversion separation proves nothing about the post-quantization ring.
    let collapsed = Polygon {
        points: vec![
            Point2::from_mm(0.0, 0.0),
            Point2::from_mm(0.00001, 0.0),
            Point2::from_mm(0.001, 0.001),
        ],
    };
    assert_invalid_input(validate_ring(&collapsed));

    let convex = geometry(vec![component(square.clone(), vec![])]);
    let holed_case = geometry(vec![holed.clone()]);
    // The square is CCW and the contained square hole is CW, with positive clearance.
    assert_eq!(
        validate_geometry(&convex, &class(TopologyClass::Convex)),
        Ok(())
    );
    assert_eq!(
        validate_geometry(&holed_case, &class(TopologyClass::Holed)),
        Ok(())
    );
    assert_invalid_input(validate_geometry(
        &geometry(vec![component(reversed_square, vec![])]),
        &class(TopologyClass::Convex),
    ));
    // Reversing the hole yields CCW; touching or putting a hole outside is invalid.
    let ccw_hole = ring(&[(3, 4), (5, 4), (5, 6), (3, 6)]);
    assert_invalid_input(validate_geometry(
        &geometry(vec![component(outer.clone(), vec![ccw_hole])]),
        &class(TopologyClass::Holed),
    ));
    let touching_hole = ring(&[(0, 4), (0, 6), (2, 6), (2, 4)]);
    assert_invalid_input(validate_geometry(
        &geometry(vec![component(outer.clone(), vec![touching_hole])]),
        &class(TopologyClass::Holed),
    ));
    let outside_hole = ring(&[(22, 4), (22, 6), (24, 6), (24, 4)]);
    assert_invalid_input(validate_geometry(
        &geometry(vec![component(outer.clone(), vec![outside_hole])]),
        &class(TopologyClass::Holed),
    ));
    // A second separated hole is valid; touching, overlapping, and nested holes are not.
    let second_hole = ring(&[(8, 4), (8, 6), (10, 6), (10, 4)]);
    assert_eq!(
        validate_geometry(
            &geometry(vec![component(
                outer.clone(),
                vec![hole.clone(), second_hole]
            )]),
            &class(TopologyClass::Holed),
        ),
        Ok(())
    );
    let corner_touch = ring(&[(5, 6), (5, 8), (7, 8), (7, 6)]);
    let overlap_hole = ring(&[(4, 5), (4, 7), (6, 7), (6, 5)]);
    for other in [corner_touch, overlap_hole] {
        assert_invalid_input(validate_geometry(
            &geometry(vec![component(outer.clone(), vec![hole.clone(), other])]),
            &class(TopologyClass::Holed),
        ));
    }
    let large_hole = ring(&[(2, 2), (2, 12), (12, 12), (12, 2)]);
    assert_invalid_input(validate_geometry(
        &geometry(vec![component(
            outer.clone(),
            vec![large_hole, hole.clone()],
        )]),
        &class(TopologyClass::Holed),
    ));
    // Each hole vertex lies inside the U, but its horizontal edges cross the
    // removed notch 6 < x < 8, so vertex-only containment is insufficient.
    let bridge_hole = ring(&[(5, 4), (5, 6), (9, 6), (9, 4)]);
    for &p in &bridge_hole.points {
        assert_eq!(point_in_region(p, &concave), Ok(Inside));
    }
    assert_invalid_input(validate_geometry(
        &geometry(vec![component(notched.contour.clone(), vec![bridge_hole])]),
        &class(TopologyClass::Holed),
    ));
    // Overlapping and touching outer closures are both forbidden.
    let overlap = component(ring(&[(9, 0), (15, 0), (15, 10), (9, 10)]), vec![]);
    let touching = component(ring(&[(10, 0), (15, 0), (15, 10), (10, 10)]), vec![]);
    // A contained outer also overlaps the first component even without edge crossings.
    let contained = component(ring(&[(2, 2), (4, 2), (4, 4), (2, 4)]), vec![]);
    for other in [overlap, touching, contained] {
        assert_invalid_input(validate_geometry(
            &geometry(vec![component(square.clone(), vec![]), other]),
            &class(TopologyClass::Disconnected),
        ));
    }
    let empty_domain = GeometryDomain {
        class: TopologyClass::Empty,
        min_components: 0,
        ..GeometryDomain::default()
    };
    assert_eq!(validate_geometry(&geometry(vec![]), &empty_domain), Ok(()));
    assert_invalid_input(validate_geometry(&convex, &empty_domain));
    assert_invalid_input(validate_geometry(&convex, &class(TopologyClass::Holed)));
    assert_invalid_input(validate_geometry(
        &convex,
        &class(TopologyClass::Disconnected),
    ));
    // Two separate outer squares have disjoint closures.
    let separated = component(ring(&[(12, 0), (18, 0), (18, 6), (12, 6)]), vec![]);
    assert_eq!(
        validate_geometry(
            &geometry(vec![component(square.clone(), vec![]), separated]),
            &class(TopologyClass::Disconnected)
        ),
        Ok(())
    );
    // A four-vertex square exceeds a valid three-vertex cap, and coordinate
    // 10 exceeds a valid absolute coordinate cap of 9.
    let three_vertices = GeometryDomain {
        bounds: GeometryBounds {
            max_vertices: 3,
            ..GeometryBounds::default()
        },
        ..GeometryDomain::default()
    };
    let small_coordinates = GeometryDomain {
        bounds: GeometryBounds {
            max_abs_units: 9,
            ..GeometryBounds::default()
        },
        ..GeometryDomain::default()
    };
    assert_invalid_input(validate_geometry(&convex, &three_vertices));
    assert_invalid_input(validate_geometry(&convex, &small_coordinates));
    // A domain requiring two holes cannot accept the one-hole fixture.
    let two_holes = GeometryDomain {
        class: TopologyClass::Holed,
        min_holes: 2,
        ..GeometryDomain::default()
    };
    assert_invalid_input(validate_geometry(&holed_case, &two_holes));

    // The U-notch imposes kernel constraints x >= 8 and x <= 6: intersection empty.
    assert_invalid_input(validate_geometry(
        &geometry(vec![notched.clone()]),
        &class(TopologyClass::Convex),
    ));
    assert_eq!(
        validate_geometry(
            &geometry(vec![notched.clone()]),
            &class(TopologyClass::Concave)
        ),
        Ok(())
    );
    assert_eq!(
        validate_geometry(
            &geometry(vec![notched]),
            &class(TopologyClass::NonStarShaped)
        ),
        Ok(())
    );
    assert_invalid_input(validate_geometry(&convex, &class(TopologyClass::Concave)));
    // This L's kernel includes [0,3] × [0,3], so it is concave but star-shaped.
    let l_shape = geometry(vec![component(
        ring(&[(0, 0), (8, 0), (8, 3), (3, 3), (3, 8), (0, 8)]),
        vec![],
    )]);
    assert_eq!(
        validate_geometry(&l_shape, &class(TopologyClass::Concave)),
        Ok(())
    );
    assert_invalid_input(validate_geometry(
        &l_shape,
        &class(TopologyClass::NonStarShaped),
    ));
    // Left half-planes of the directed edges reduce to y <= 0, y >= -4,
    // -2 <= 2x + y <= 2 and -10 <= x <= 10, so the kernel contains (0,-1)
    // with a neighbourhood: it is nonempty and the class witness must reject
    // this star-shaped polygon. A degenerate point/line kernel would also be
    // nonempty because only an empty clipped vertex list counts as empty.
    let star_shaped = geometry(vec![component(
        ring(&[
            (-10, 0),
            (-1, 0),
            (1, -4),
            (10, -4),
            (10, 0),
            (1, 0),
            (-1, 4),
            (-10, 4),
        ]),
        vec![],
    )]);
    assert_eq!(
        validate_geometry(&star_shaped, &class(TopologyClass::Concave)),
        Ok(())
    );
    assert_invalid_input(validate_geometry(
        &star_shaped,
        &class(TopologyClass::NonStarShaped),
    ));
    // Parallel notch edges are 1 unit apart; bbox min dimension 40 gives threshold 5/2.
    let narrow = geometry(vec![component(
        ring(&[
            (0, 0),
            (40, 0),
            (40, 40),
            (21, 40),
            (21, 1),
            (20, 1),
            (20, 40),
            (0, 40),
        ]),
        vec![],
    )]);
    assert_eq!(
        validate_geometry(&narrow, &class(TopologyClass::Thin)),
        Ok(())
    );
    assert_eq!(
        validate_geometry(&narrow, &class(TopologyClass::NearDegenerate)),
        Ok(())
    );
    // This similar notch has bbox min dimension 15: excluded from Thin, but clearance = 1.
    let small = geometry(vec![component(
        ring(&[
            (0, 0),
            (15, 0),
            (15, 15),
            (8, 15),
            (8, 1),
            (7, 1),
            (7, 15),
            (0, 15),
        ]),
        vec![],
    )]);
    assert_invalid_input(validate_geometry(&small, &class(TopologyClass::Thin)));
    assert_eq!(
        validate_geometry(&small, &class(TopologyClass::NearDegenerate)),
        Ok(())
    );
    // Slanted long-edge clearance squared is 39^2/(40^2+39^2) = 1521/3121, not 1.
    let slanted = geometry(vec![component(
        ring(&[(0, 0), (40, 39), (39, 39), (-1, 0)]),
        vec![],
    )]);
    assert_eq!(
        validate_geometry(&slanted, &class(TopologyClass::Thin)),
        Ok(())
    );
    assert_invalid_input(validate_geometry(
        &slanted,
        &class(TopologyClass::NearDegenerate),
    ));
    // Opposite long edges are sqrt(2) units apart, bbox min dimension = 31.
    // Their squared gap 2 is <= (31/16)^2 = 961/256, but not <= 1^2.
    let fractional_threshold = geometry(vec![component(
        ring(&[(0, 0), (30, 30), (29, 31), (-1, 1)]),
        vec![],
    )]);
    assert_eq!(
        validate_geometry(&fractional_threshold, &class(TopologyClass::Thin)),
        Ok(())
    );
    assert_invalid_input(validate_geometry(
        &fractional_threshold,
        &class(TopologyClass::NearDegenerate),
    ));
}

fn geometry_domain(class: TopologyClass) -> GeometryDomain {
    let mut domain = GeometryDomain {
        class,
        min_components: usize::from(class != TopologyClass::Empty),
        bounds: GeometryBounds {
            max_abs_units: 64,
            max_vertices: 32,
            max_holes: 3,
            max_components: 4,
            ..GeometryBounds::default()
        },
        ..GeometryDomain::default()
    };
    domain.bounds.max_shrink_candidates = 128;
    domain
}

fn ring_turns(ring: &Polygon) -> Vec<i128> {
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
}

// A downward vertical edge's left half-plane is x >= edge.x; an upward
// edge's is x <= edge.x. Strictly reversed constraints prove empty kernel.
fn opposing_notch_walls(ring: &Polygon) -> bool {
    let points = &ring.points;
    let edges: Vec<_> = points
        .iter()
        .copied()
        .zip(points.iter().copied().cycle().skip(1))
        .take(points.len())
        .collect();
    edges.iter().any(|(a, b)| {
        a.x == b.x
            && a.y > b.y
            && edges
                .iter()
                .any(|(c, d)| c.x == d.x && c.y < d.y && a.x > c.x)
    })
}

fn unit_parallel_gap(ring: &Polygon) -> bool {
    let points = &ring.points;
    let n = points.len();
    (0..n).any(|i| {
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
}

fn ring_bbox(ring: &Polygon) -> (i64, i64, i64, i64) {
    (
        ring.points.iter().map(|p| p.x).min().unwrap(),
        ring.points.iter().map(|p| p.x).max().unwrap(),
        ring.points.iter().map(|p| p.y).min().unwrap(),
        ring.points.iter().map(|p| p.y).max().unwrap(),
    )
}

fn boxes_separated_by_unit(a: (i64, i64, i64, i64), b: (i64, i64, i64, i64)) -> bool {
    a.1 < b.0 || b.1 < a.0 || a.3 < b.2 || b.3 < a.2
}

fn assert_independent_witness(case: &GeometryCase, class: TopologyClass) {
    match class {
        TopologyClass::Empty => assert!(case.polygons.is_empty()),
        TopologyClass::Convex => assert!(case
            .polygons
            .iter()
            .all(|p| ring_turns(&p.contour).iter().all(|&turn| turn >= 0))),
        TopologyClass::Concave => assert!(case
            .polygons
            .iter()
            .any(|p| ring_turns(&p.contour).iter().any(|&turn| turn < 0))),
        TopologyClass::NonStarShaped => assert!(case
            .polygons
            .iter()
            .any(|p| opposing_notch_walls(&p.contour))),
        TopologyClass::Holed => assert!(case.polygons.iter().any(|p| !p.holes.is_empty())),
        TopologyClass::Disconnected => assert!(case.polygons.len() >= 2),
        TopologyClass::Thin | TopologyClass::NearDegenerate => {
            assert!(case.polygons.iter().any(|p| {
                let points = &p.contour.points;
                let width = points.iter().map(|p| p.x).max().unwrap()
                    - points.iter().map(|p| p.x).min().unwrap();
                let height = points.iter().map(|p| p.y).max().unwrap()
                    - points.iter().map(|p| p.y).min().unwrap();
                unit_parallel_gap(&p.contour)
                    && (class != TopologyClass::Thin || width.min(height) >= 16)
            }));
        }
    }
}

#[test]
fn domain_geometry_topology_witnesses() {
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
    let mut saw_multiple_holes = false;
    let mut saw_multiple_components = false;
    let mut saw_short_edge = false;
    let mut saw_one_chamfer = false;
    let mut saw_two_chamfers = false;
    let mut saw_collinear = false;
    for class in classes {
        let domain = geometry_domain(class);
        let cases = sample(&geometry_cases(domain.clone()).unwrap(), 128);
        assert_eq!(cases.len(), 128);
        let mut vertex_counts = BTreeSet::new();
        for case in &cases {
            assert_eq!(
                validate_geometry(case, &domain),
                Ok(()),
                "{class:?}: {case:?}"
            );
            assert_independent_witness(case, class);
            if class == TopologyClass::Convex {
                assert!(
                    case.polygons.iter().all(|p| p.contour.points.len() >= 4),
                    "roomy convex domains start from rectangles or chamfers"
                );
                let ring = &case.polygons[0].contour;
                let slanted_edges = ring
                    .points
                    .iter()
                    .zip(ring.points.iter().cycle().skip(1))
                    .take(ring.points.len())
                    .filter(|(a, b)| a.x != b.x && a.y != b.y)
                    .count();
                saw_one_chamfer |= slanted_edges == 1;
                saw_two_chamfers |= slanted_edges == 2;
                saw_collinear |= ring_turns(ring).contains(&0);
            }
            let complexity = geometry_complexity(case).unwrap();
            vertex_counts.insert(complexity.2);
            saw_multiple_holes |= complexity.1 >= 2;
            saw_multiple_components |= complexity.0 >= 3;
            for (index, poly) in case.polygons.iter().enumerate() {
                let outer = ring_bbox(&poly.contour);
                for other in &case.polygons[..index] {
                    assert!(boxes_separated_by_unit(outer, ring_bbox(&other.contour)));
                }
                if class == TopologyClass::Holed {
                    for (hole_index, hole) in poly.holes.iter().enumerate() {
                        let inner = ring_bbox(hole);
                        assert!(
                            outer.0 < inner.0
                                && inner.1 < outer.1
                                && outer.2 < inner.2
                                && inner.3 < outer.3
                        );
                        for other in &poly.holes[..hole_index] {
                            assert!(boxes_separated_by_unit(inner, ring_bbox(other)));
                        }
                    }
                }
            }
            for poly in &case.polygons {
                let points = &poly.contour.points;
                saw_short_edge |= points
                    .iter()
                    .zip(points.iter().cycle().skip(1))
                    .take(points.len())
                    .any(|(a, b)| (a.x - b.x).abs() + (a.y - b.y).abs() == 1);
            }
        }
        if class == TopologyClass::Empty {
            assert_eq!(vertex_counts, BTreeSet::from([0]));
        } else {
            assert!(vertex_counts.len() >= 2, "{class:?}: {vertex_counts:?}");
        }
    }
    assert!(saw_multiple_holes && saw_multiple_components && saw_short_edge);
    assert!(saw_one_chamfer && saw_two_chamfers && saw_collinear);

    // Every catalog minimum must generate without rejection even at tight
    // coordinate and vertex caps, including simultaneous required structures.
    for (class, units, vertices, components, holes) in [
        (TopologyClass::Convex, 1, 3, 1, 0),
        (TopologyClass::Concave, 1, 6, 1, 0),
        (TopologyClass::NonStarShaped, 2, 8, 1, 0),
        (TopologyClass::Holed, 2, 8, 1, 1),
        (TopologyClass::Disconnected, 2, 8, 2, 0),
        (TopologyClass::Thin, 8, 8, 1, 0),
        (TopologyClass::NearDegenerate, 2, 8, 1, 0),
        (TopologyClass::Holed, 3, 12, 1, 2),
        (TopologyClass::Disconnected, 3, 12, 3, 0),
        (TopologyClass::NonStarShaped, 4, 16, 2, 1),
    ] {
        let domain = GeometryDomain {
            class,
            min_components: components,
            min_holes: holes,
            bounds: GeometryBounds {
                max_abs_units: units,
                max_vertices: vertices,
                max_components: components,
                max_holes: holes.max(1),
                ..GeometryBounds::default()
            },
            ..GeometryDomain::default()
        };
        let cases = sample(
            &geometry_cases(domain.clone()).expect("tight catalog domain"),
            32,
        );
        assert_eq!(cases.len(), 32);
        for case in cases {
            assert_eq!(
                validate_geometry(&case, &domain),
                Ok(()),
                "{class:?}: {case:?}"
            );
            assert_independent_witness(&case, class);
        }
    }

    let segments = sample(
        &line_segments(GeometryBounds {
            max_abs_units: 8,
            ..GeometryBounds::default()
        })
        .unwrap(),
        256,
    );
    assert!(segments.contains(&[point(0, 0), point(0, 0)]));
    assert!(segments.contains(&[point(-8, 0), point(8, 0)]));
    assert!(segments.contains(&[point(0, -8), point(0, 8)]));
    assert!(segments.contains(&[point(-8, -8), point(8, 8)]));
    assert!(segments.contains(&[point(0, 0), point(8, 0)]));
    visit(
        &line_segments(GeometryBounds {
            max_abs_units: 8,
            ..GeometryBounds::default()
        })
        .unwrap(),
        32,
        |segment| {
            assert!(segment.iter().all(|p| p.x.abs() <= 8 && p.y.abs() <= 8));
        },
    );
}

#[test]
fn domain_shrink_protocol_preserves_witnesses() {
    for class in [
        TopologyClass::Convex,
        TopologyClass::Concave,
        TopologyClass::NonStarShaped,
        TopologyClass::Holed,
        TopologyClass::Disconnected,
        TopologyClass::Thin,
        TopologyClass::NearDegenerate,
    ] {
        let mut domain = geometry_domain(class);
        domain.bounds.max_components = 2;
        domain.bounds.max_holes = 1;
        match class {
            TopologyClass::Holed => {
                domain.min_holes = 2;
                domain.bounds.max_holes = 3;
            }
            TopologyClass::Disconnected => {
                domain.min_components = 3;
                domain.bounds.max_components = 4;
            }
            TopologyClass::NonStarShaped => domain.min_holes = 1,
            _ => {}
        }
        let strategy = geometry_cases(domain.clone()).unwrap();
        let mut tree = strategy.new_tree(&mut runner()).unwrap();
        let initial = tree.current();
        let mut successful = 0;
        loop {
            let before = geometry_complexity(&tree.current()).unwrap();
            let changed = tree.simplify();
            let current = tree.current();
            assert_eq!(validate_geometry(&current, &domain), Ok(()));
            if class == TopologyClass::NonStarShaped {
                assert!(current
                    .polygons
                    .iter()
                    .any(|p| ring_turns(&p.contour).iter().any(|&turn| turn < 0)));
            }
            if !changed {
                break;
            }
            assert!(geometry_complexity(&current).unwrap() < before);
            successful += 1;
            assert!(successful < 512, "nonterminating shrink for {class:?}");
        }
        assert!(successful > 0, "no shrinks for {class:?}");
        let mut restored = 0;
        loop {
            let changed = tree.complicate();
            assert_eq!(validate_geometry(&tree.current(), &domain), Ok(()));
            if !changed {
                break;
            }
            restored += 1;
        }
        assert_eq!(restored, successful);
        assert_eq!(tree.current(), initial);
        assert!(!tree.complicate());
        assert_eq!(tree.current(), initial);
    }
    // This three-vertex right triangle has absolute-coordinate sum two,
    // the least possible nonzero integer-area triangle sum.
    let minimal_domain = GeometryDomain {
        bounds: GeometryBounds {
            max_abs_units: 1,
            max_vertices: 3,
            max_components: 1,
            max_holes: 1,
            ..GeometryBounds::default()
        },
        ..GeometryDomain::default()
    };
    let strategy = geometry_cases(minimal_domain.clone()).unwrap();
    let mut rng = runner();
    let mut minimal = (0..512)
        .map(|_| strategy.new_tree(&mut rng).unwrap())
        .find(|tree| {
            tree.current().polygons[0].contour.points == vec![point(0, 0), point(1, 0), point(0, 1)]
        })
        .expect("explicit minimal triangle is reachable");
    assert_eq!(
        geometry_complexity(&minimal.current()).unwrap(),
        (1, 0, 3, 2)
    );
    let before = minimal.current();
    assert!(!minimal.simplify());
    assert_eq!(minimal.current(), before);
    assert_eq!(
        validate_geometry(&minimal.current(), &minimal_domain),
        Ok(())
    );

    // A rejected simplification must not be offered again immediately after
    // rollback; Proptest alternates simplify and complicate when probing.
    let mut alternating_domain = geometry_domain(TopologyClass::Convex);
    alternating_domain.bounds.max_components = 1;
    let strategy = geometry_cases(alternating_domain.clone()).unwrap();
    let mut rng = runner();
    let mut alternating = (0..128)
        .map(|_| strategy.new_tree(&mut rng).unwrap())
        .find(|tree| tree.current().polygons[0].contour.points.len() >= 5)
        .expect("nonminimal convex case");
    let initial = alternating.current();
    assert!(alternating.simplify());
    let rejected = alternating.current();
    assert!(alternating.complicate());
    assert_eq!(alternating.current(), initial);
    assert!(alternating.simplify());
    assert_ne!(alternating.current(), rejected);
    assert_eq!(
        validate_geometry(&alternating.current(), &alternating_domain),
        Ok(())
    );
    assert!(alternating.complicate());
    assert_eq!(alternating.current(), initial);
    assert!(!alternating.complicate());
}

#[test]
fn domain_exhaustion_is_not_success() {
    use proptest::test_runner::TestCaseError;
    use serde_json::{json, Value};
    use slicer_sdk::test_support::property_testing::PropertyRun;
    use std::process::Command;

    const NAME: &str = "domain_exhaustion_is_not_success";
    const MODE: &str = "PNP_DOMAIN_EXHAUSTION_CHILD";
    const REJECT: &str = "reject";
    const CANDIDATE: &str = "candidate";

    if let Ok(mode) = std::env::var(MODE) {
        let run = PropertyRun::new("D-EXHAUST", NAME).expect("child context");
        match mode.as_str() {
            REJECT => {
                let error = run
                    .check(&Just(1234_u32), |_, _| {
                        Err(TestCaseError::reject("controlled rejection"))
                    })
                    .expect_err("exhausted property must not pass");
                assert_eq!(error.kind(), "exhausted");
            }
            CANDIDATE => {
                // Geometry reaching its candidate limit during a real harness shrink.
                let domain = GeometryDomain {
                    bounds: GeometryBounds {
                        max_components: 1,
                        max_shrink_candidates: 0,
                        ..GeometryBounds::default()
                    },
                    ..GeometryDomain::default()
                };
                let strategy = geometry_cases(domain.clone()).expect("valid geometry domain");
                let error = run
                    .check(&strategy, |case, _| {
                        assert_eq!(validate_geometry(&case, &domain), Ok(()));
                        Err(TestCaseError::fail("controlled geometry failure"))
                    })
                    .expect_err("failing geometry property must not pass");
                assert_eq!(error.kind(), "sut_failure");
            }
            other => panic!("unexpected exhaustion child mode {other}"),
        }
        return;
    }

    let token = format!("domain-exhaust-{}", std::process::id());
    let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/property-testing")
        .join(&token);
    let mut outputs = Vec::new();
    for mode in [REJECT, CANDIDATE] {
        let dir = base.join(mode).join("D-EXHAUST");
        let context = json!({
            "schema_version": 1, "run_token": format!("{token}-{mode}"), "property_id": "D-EXHAUST",
            "tier": "routine", "seed": 37, "package": "slicer-sdk", "target_kind": "test",
            "target_name": "property_domain_contracts", "test_name": NAME,
            "features": ["slicer-sdk/property_testing"],
            "profile": {"cases": 8, "max_local_rejects": 2, "max_global_rejects": 3,
                "max_flat_map_regens": 100, "max_default_size_range": 32,
                "max_shrink_iters": 64, "max_shrink_time_ms": 0,
                "rng_algorithm": "chacha", "isolation": "in_process", "timeout_ms": 0,
                "bounds": {}},
            "artifact_dir": dir.to_str().unwrap(), "proptest_version": "1.11.0", "generator_version": 1,
            "source_revision": "test-revision", "dirty_paths": [], "source_state_digest": "test-digest",
            "cargo_target_dir": null, "completion_eligible": true
        });
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args(["--exact", NAME, "--nocapture"])
            .env(MODE, mode)
            .env("PNP_PROPERTY_CONTEXT", context.to_string())
            .env_remove("_PROPTEST_FORKFILE")
            .env_remove("PROPTEST_VERBOSE");
        for name in [
            "PROPTEST_CASES",
            "PROPTEST_MAX_LOCAL_REJECTS",
            "PROPTEST_MAX_GLOBAL_REJECTS",
            "PROPTEST_MAX_FLAT_MAP_REGENS",
            "PROPTEST_MAX_SHRINK_TIME",
            "PROPTEST_MAX_SHRINK_ITERS",
            "PROPTEST_MAX_DEFAULT_SIZE_RANGE",
            "PROPTEST_FORK",
            "PROPTEST_TIMEOUT",
            "PROPTEST_RNG_ALGORITHM",
            "PROPTEST_RNG_SEED",
            "PROPTEST_DISABLE_FAILURE_PERSISTENCE",
        ] {
            command.env_remove(name);
        }
        let output = command.output().expect("launch exhaustion probe");
        assert!(
            output.status.success(),
            "child {mode} failed: stdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        outputs.push((mode, dir, output));
    }

    let controller_end = |output: &std::process::Output| -> Value {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let events: Vec<Value> = stdout
            .lines()
            .filter_map(|line| line.strip_prefix("PNP_PROPERTY_EVENT "))
            .map(|line| serde_json::from_str(line).expect("valid property event"))
            .collect();
        let ends: Vec<_> = events
            .iter()
            .filter(|event| event["process_role"] == "controller" && event["phase"] == "end")
            .collect();
        assert_eq!(ends.len(), 1, "controller end event missing: {events:?}");
        ends[0].clone()
    };

    // Rejection exhaustion is reported as `exhausted`, with a retained input and
    // never as a pass.
    let (_, reject_dir, reject_output) = &outputs[0];
    let end = controller_end(reject_output);
    assert_eq!(end["outcome"], "exhausted");
    assert_eq!(end["error_kind"], "exhausted");
    assert_eq!(end["accepted_test_calls"], 0);
    assert!(end["rejected_test_calls"].as_u64().unwrap() > 0);
    let report: Value = serde_json::from_slice(
        &std::fs::read(reject_dir.join("failure.json")).expect("failure artifact"),
    )
    .unwrap();
    assert_eq!(report["classification"], "exhausted");
    assert_eq!(report["concrete_input"], "1234");

    // Candidate exhaustion during a real harness shrink is reported as a failure
    // with a retained concrete geometry, and the candidate-limit diagnostic proves
    // the shrink budget was the binding limit rather than a claimed minimum.
    let (_, candidate_dir, candidate_output) = &outputs[1];
    let diagnostics = String::from_utf8_lossy(&candidate_output.stderr);
    let diagnostic: Vec<&str> = diagnostics
        .lines()
        .filter_map(|line| line.strip_prefix("candidate_limit_reached "))
        .collect();
    assert!(
        !diagnostic.is_empty(),
        "candidate limit must be reported while the harness shrinks"
    );
    let tuple = diagnostic[0]
        .strip_prefix('(')
        .and_then(|text| text.strip_suffix(')'))
        .expect("candidate diagnostic contains the complexity tuple");
    let complexity: Vec<u128> = tuple
        .split(", ")
        .map(|part| part.parse().unwrap())
        .collect();
    assert_eq!(complexity.len(), 4);
    assert_eq!(&complexity[..2], &[1, 0]);
    assert!(complexity[2] >= 4);
    let end = controller_end(candidate_output);
    assert_eq!(end["outcome"], "failed");
    assert_eq!(end["error_kind"], "sut_failure");
    assert!(end["accepted_test_calls"].as_u64().unwrap() >= 1);
    assert_eq!(end["shrink_completed"], Value::Null);
    let report: Value = serde_json::from_slice(
        &std::fs::read(candidate_dir.join("failure.json")).expect("failure artifact"),
    )
    .unwrap();
    assert_eq!(report["classification"], "sut_or_contract");
    assert!(
        report["concrete_input"]
            .as_str()
            .is_some_and(|input| input.contains("GeometryCase")),
        "retained geometry input: {}",
        report["concrete_input"]
    );
    assert_eq!(report["shrink_completed"], Value::Null);
}

#[test]
fn domain_known_failures_shrink_structurally() {
    let scenarios = [
        TopologyClass::Convex,
        TopologyClass::Holed,
        TopologyClass::Disconnected,
    ];
    let mut reductions = [0_usize; 3];
    for (index, class) in scenarios.into_iter().enumerate() {
        let mut domain = geometry_domain(class);
        match class {
            TopologyClass::Convex => {
                domain.bounds.max_abs_units = 2;
                domain.bounds.max_vertices = 5;
                domain.bounds.max_components = 1;
            }
            TopologyClass::Holed => {
                domain.bounds.max_abs_units = 4;
                domain.bounds.max_vertices = 16;
                domain.bounds.max_components = 1;
                domain.min_holes = 1;
            }
            TopologyClass::Disconnected => {
                domain.bounds.max_abs_units = 4;
                domain.min_components = 2;
                domain.bounds.max_components = 4;
            }
            _ => unreachable!(),
        }
        // Concrete nonminimal fixtures, not a search for any vaguely large
        // case: the seeded strategy must reach these exact integer witnesses.
        let witness = match class {
            TopologyClass::Convex => geometry(vec![component(
                ring(&[(-2, 0), (2, 0), (2, 1), (1, 2), (-2, 2)]),
                vec![],
            )]),
            TopologyClass::Holed => geometry(vec![component(
                ring(&[(-3, -2), (4, -2), (4, 3), (-3, 3)]),
                vec![
                    ring(&[(-2, -1), (-2, 0), (-1, 0), (-1, -1)]),
                    ring(&[(0, -1), (0, 0), (1, 0), (1, -1)]),
                    ring(&[(2, -1), (2, 0), (3, 0), (3, -1)]),
                ],
            )]),
            TopologyClass::Disconnected => geometry(vec![
                component(ring(&[(-4, -4), (-2, -4), (-2, 4), (-4, 4)]), vec![]),
                component(ring(&[(-1, -4), (0, -4), (0, -3), (-1, -3)]), vec![]),
                component(ring(&[(1, -4), (2, -4), (2, -3), (1, -3)]), vec![]),
                component(ring(&[(3, -4), (4, -4), (4, -3), (3, -3)]), vec![]),
            ]),
            _ => unreachable!(),
        };
        assert_eq!(validate_geometry(&witness, &domain), Ok(()));
        let strategy = geometry_cases(domain.clone()).unwrap();
        let mut rng = runner();
        let mut tree = (0..512)
            .map(|_| strategy.new_tree(&mut rng).unwrap())
            .find(|tree| tree.current() == witness)
            .expect("explicit nonminimal structural witness is reachable");
        let initial = tree.current();
        let before = geometry_complexity(&initial).unwrap();
        let mut steps = 0;
        while tree.simplify() {
            let current = tree.current();
            assert_eq!(validate_geometry(&current, &domain), Ok(()));
            let c = geometry_complexity(&current).unwrap();
            assert!(c.0 >= domain.min_components && c.1 >= domain.min_holes);
            steps += 1;
            assert!(steps < 512);
        }
        let after = geometry_complexity(&tree.current()).unwrap();
        reductions[index] = match class {
            TopologyClass::Convex => before.2 - after.2,
            TopologyClass::Holed => before.1 - after.1,
            TopologyClass::Disconnected => before.0 - after.0,
            _ => unreachable!(),
        };
        assert!(
            reductions[index] > 0,
            "no {class:?} structural reduction: {before:?} -> {after:?}"
        );
        if class == TopologyClass::Holed {
            assert_eq!(after.1, 1);
        }
        if class == TopologyClass::Disconnected {
            assert_eq!(after.0, 2);
        }
        while tree.complicate() {
            assert_eq!(validate_geometry(&tree.current(), &domain), Ok(()));
        }
        assert_eq!(tree.current(), initial);
    }
    eprintln!(
        "observed_structural_reductions vertices={} holes={} components={}",
        reductions[0], reductions[1], reductions[2]
    );

    for kind in [
        InvalidGeometryKind::Crossing,
        InvalidGeometryKind::DuplicateVertex,
        InvalidGeometryKind::Collapsed,
        InvalidGeometryKind::TouchingHole,
        InvalidGeometryKind::MisnestedHole,
    ] {
        let strategy = invalid_geometry_cases(kind, GeometryBounds::default()).unwrap();
        let mut observed = 0;
        visit(&strategy, 16, |case| {
            observed += 1;
            assert_eq!(case.kind, kind);
            let outer = &case.polygons[0].contour;
            let domain = class(
                if matches!(
                    kind,
                    InvalidGeometryKind::TouchingHole | InvalidGeometryKind::MisnestedHole
                ) {
                    TopologyClass::Holed
                } else {
                    TopologyClass::Convex
                },
            );
            assert_invalid_input(validate_geometry(
                &GeometryCase {
                    polygons: case.polygons.clone(),
                },
                &domain,
            ));
            if matches!(
                kind,
                InvalidGeometryKind::TouchingHole | InvalidGeometryKind::MisnestedHole
            ) {
                assert_eq!(validate_ring(outer), Ok(()));
                assert!(signed_area2(outer).unwrap() > 0);
                let hole = &case.polygons[0].holes[0];
                assert_eq!(validate_ring(hole), Ok(()));
                assert!(signed_area2(hole).unwrap() < 0);
            }
            match kind {
                InvalidGeometryKind::Crossing => assert_eq!(
                    segment_relation(
                        [outer.points[0], outer.points[1]],
                        [outer.points[2], outer.points[3]]
                    ),
                    Ok(SegmentRelation::ProperCrossing)
                ),
                InvalidGeometryKind::DuplicateVertex => assert!(
                    outer.points.iter().copied().collect::<BTreeSet<_>>().len()
                        < outer.points.len()
                ),
                InvalidGeometryKind::Collapsed => assert_eq!(signed_area2(outer), Ok(0)),
                InvalidGeometryKind::TouchingHole => assert_eq!(
                    point_in_region(
                        case.polygons[0].holes[0].points[0],
                        &[component(outer.clone(), vec![])]
                    ),
                    Ok(PointLocation::Boundary)
                ),
                InvalidGeometryKind::MisnestedHole => assert_eq!(
                    point_in_region(
                        case.polygons[0].holes[0].points[0],
                        &[component(outer.clone(), vec![])]
                    ),
                    Ok(PointLocation::Outside)
                ),
            }
        });
        assert!(
            observed > 16,
            "fault must be checked during actual shrinking: {kind:?}"
        );
    }

    // The same families driven through a real Proptest runner: a known failing
    // predicate must start structurally nonminimal and shrink to the failure
    // boundary, while an always-failing predicate must stop at the domain minimum.
    fn shrink_known_failure<S, F>(strategy: S, fails: F) -> (GeometryComplexity, GeometryCase)
    where
        S: Strategy<Value = GeometryCase>,
        F: Fn(&GeometryCase) -> bool,
    {
        let mut rng = TestRunner::new(Config {
            cases: 32,
            max_shrink_iters: 512,
            failure_persistence: None,
            rng_seed: RngSeed::Fixed(37),
            ..Config::default()
        });
        let first = std::cell::Cell::new(None);
        let error = rng
            .run(&strategy, |case| {
                if fails(&case) {
                    let complexity = geometry_complexity(&case).expect("bounded catalog");
                    if first.get().is_none() {
                        first.set(Some(complexity));
                    }
                    Err(proptest::test_runner::TestCaseError::fail(
                        "known failing geometry",
                    ))
                } else {
                    Ok(())
                }
            })
            .expect_err("known failing property must fail");
        let retained = match error {
            proptest::test_runner::TestError::Fail(_, value) => value,
            other => panic!("unexpected shrink outcome: {other:?}"),
        };
        (first.get().expect("at least one failing case"), retained)
    }

    // The threshold predicate fails for any case with more than one hole, so the
    // retained case must have two; the assertion below additionally requires the
    // first failing case to start with more, so a boundary-sized start or a
    // shrink that removes no hole fails the probe.
    let mut holed = geometry_domain(TopologyClass::Holed);
    holed.bounds.max_abs_units = 8;
    holed.bounds.max_vertices = 24;
    holed.bounds.max_components = 1;
    holed.bounds.max_holes = 3;
    holed.min_holes = 1;
    let (first, retained_case) =
        shrink_known_failure(geometry_cases(holed.clone()).unwrap(), |case| {
            geometry_complexity(case).unwrap().1 > 1
        });
    let retained = geometry_complexity(&retained_case).unwrap();
    assert!(
        first.1 > retained.1,
        "hole failure must start structurally nonminimal: {first:?} -> {retained:?}"
    );
    assert_eq!(retained.1, 2, "holes {first:?} -> {retained:?}");
    assert_eq!(validate_geometry(&retained_case, &holed), Ok(()));
    eprintln!("known_failure_trace holes {first:?} -> {retained:?}");
    let mut holed_required = holed.clone();
    holed_required.min_holes = 3;
    let (first, retained_case) =
        shrink_known_failure(geometry_cases(holed_required.clone()).unwrap(), |_| true);
    let retained = geometry_complexity(&retained_case).unwrap();
    assert_eq!(retained.1, holed_required.min_holes);
    assert_eq!(retained.0, 1);
    assert_eq!(validate_geometry(&retained_case, &holed_required), Ok(()));
    eprintln!("known_failure_trace holes_retained {first:?} -> {retained:?}");

    // As with holes, the probe fails unless the first failing case starts above the
    // retained boundary, so a boundary-sized start or a shrink that removes no
    // component fails.
    let mut disconnected = geometry_domain(TopologyClass::Disconnected);
    disconnected.bounds.max_abs_units = 8;
    disconnected.min_components = 2;
    disconnected.bounds.max_components = 4;
    let (first, retained_case) =
        shrink_known_failure(geometry_cases(disconnected.clone()).unwrap(), |case| {
            geometry_complexity(case).unwrap().0 > 2
        });
    let retained = geometry_complexity(&retained_case).unwrap();
    assert!(
        first.0 > retained.0,
        "component failure must start structurally nonminimal: {first:?} -> {retained:?}"
    );
    assert_eq!(retained.0, 3, "components {first:?} -> {retained:?}");
    assert_eq!(validate_geometry(&retained_case, &disconnected), Ok(()));
    eprintln!("known_failure_trace components {first:?} -> {retained:?}");
    let mut disconnected_required = disconnected.clone();
    disconnected_required.min_components = 4;
    disconnected_required.bounds.max_components = 4;
    let (first, retained_case) = shrink_known_failure(
        geometry_cases(disconnected_required.clone()).unwrap(),
        |_| true,
    );
    let retained = geometry_complexity(&retained_case).unwrap();
    assert_eq!(retained.0, disconnected_required.min_components);
    assert_eq!(
        validate_geometry(&retained_case, &disconnected_required),
        Ok(())
    );
    eprintln!("known_failure_trace components_retained {first:?} -> {retained:?}");

    // Ring-vertex reduction is asserted by the explicit nonminimal convex witness
    // above (`reductions[0]`), which starts from a concrete five-vertex case. The
    // Convex generator cannot guarantee a nonminimal vertex count on the first
    // failing case, so a runner-driven vertex probe would either be boundary-sized
    // or vacuous; the holes/components probes below cover the runner path instead.
}

fn assert_mesh_fault(mesh: &IndexedTriangleSet, kind: InvalidMeshKind, token: &str) {
    match kind {
        InvalidMeshKind::BadIndex => assert!(mesh
            .indices
            .iter()
            .any(|&i| i as usize >= mesh.vertices.len())),
        InvalidMeshKind::TruncatedTriangle => assert!(!mesh.indices.len().is_multiple_of(3)),
        InvalidMeshKind::NonFiniteVertex => assert!(mesh
            .vertices
            .iter()
            .any(|p| !p.x.is_finite() || !p.y.is_finite() || !p.z.is_finite())),
        InvalidMeshKind::DegenerateTriangle => {
            // Its three distinct points lie on the X axis.
            assert_eq!(mesh.indices, [0, 1, 2]);
            assert!(mesh.vertices.iter().all(|p| p.y == 0.0 && p.z == 0.0));
        }
    }
    for result in [
        validate_indexed_mesh(mesh, true),
        validate_indexed_mesh(mesh, false),
        mesh_signed_volume_mm3(mesh).map(|_| ()),
    ] {
        assert!(
            matches!(result, Err(DomainError::InvalidInput(ref message)) if message.contains(token)),
            "{kind:?}: {result:?}"
        );
    }
}

#[test]
fn domain_analytical_mesh_oracles() {
    use property_mesh::{check_mesh, check_transform, check_transformed_mesh, vertex_bounds};

    let box_mesh = property_mesh::worked_box();
    let prism = property_mesh::worked_prism();
    // 2 * 3 * 4 = 24 mm³; the extrema are (0,0,0) and (2,3,4).
    assert_eq!(validate_indexed_mesh(&box_mesh, true), Ok(()));
    assert_eq!(mesh_signed_volume_mm3(&box_mesh), Ok(24.0));
    assert_eq!(
        vertex_bounds(&box_mesh),
        BoundingBox3 {
            min: Point3::default(),
            max: Point3 {
                x: 2.0,
                y: 3.0,
                z: 4.0
            },
        }
    );
    // The right-triangle area is 3 * 4 / 2 = 6 mm²; 6 * 5 = 30 mm³.
    assert_eq!(validate_indexed_mesh(&prism, true), Ok(()));
    assert_eq!(mesh_signed_volume_mm3(&prism), Ok(30.0));
    assert_eq!(
        vertex_bounds(&prism),
        BoundingBox3 {
            min: Point3::default(),
            max: Point3 {
                x: 3.0,
                y: 4.0,
                z: 5.0
            },
        }
    );

    // An open surface is valid in non-closed mode; missing edges, reversed
    // incidence, and a third occurrence are independently rejected in closed mode.
    let mut open = box_mesh.clone();
    open.indices.truncate(open.indices.len() - 3);
    assert_eq!(validate_indexed_mesh(&open, false), Ok(()));
    assert!(mesh_signed_volume_mm3(&open).is_ok());
    assert!(
        matches!(validate_indexed_mesh(&open, true), Err(DomainError::InvalidInput(ref s)) if s.contains("closed edge"))
    );
    let mut flipped_face = box_mesh.clone();
    flipped_face.indices.swap(0, 1);
    assert!(
        matches!(validate_indexed_mesh(&flipped_face, true), Err(DomainError::InvalidInput(ref s)) if s.contains("closed edge"))
    );
    let mut tripled_edge = box_mesh.clone();
    tripled_edge
        .indices
        .extend_from_slice(&box_mesh.indices[..3]);
    assert!(
        matches!(validate_indexed_mesh(&tripled_edge, true), Err(DomainError::InvalidInput(ref s)) if s.contains("closed edge"))
    );
    let mut reversed = box_mesh.clone();
    for triangle in reversed.indices.chunks_exact_mut(3) {
        triangle.swap(1, 2);
    }
    assert_eq!(validate_indexed_mesh(&reversed, true), Ok(()));
    assert_eq!(mesh_signed_volume_mm3(&reversed), Ok(-24.0));

    let mut repeated = prism.clone();
    repeated.indices[1] = repeated.indices[0];
    assert!(
        matches!(validate_indexed_mesh(&repeated, false), Err(DomainError::InvalidInput(ref s)) if s.contains("repeated index"))
    );
    assert_invalid_input(mesh_signed_volume_mm3(&repeated));
    let mut coincident = prism.clone();
    coincident.vertices[2] = coincident.vertices[0];
    assert!(
        matches!(validate_indexed_mesh(&coincident, false), Err(DomainError::InvalidInput(ref s)) if s.contains("degenerate"))
    );
    assert_invalid_input(mesh_signed_volume_mm3(&coincident));
    for axis in 0..3 {
        let mut nonfinite = prism.clone();
        match axis {
            0 => nonfinite.vertices[0].x = f32::INFINITY,
            1 => nonfinite.vertices[0].y = f32::NEG_INFINITY,
            _ => nonfinite.vertices[0].z = f32::NAN,
        }
        assert!(
            matches!(validate_indexed_mesh(&nonfinite, false), Err(DomainError::InvalidInput(ref s)) if s.contains("non-finite"))
        );
    }
    // Finite f32 extremes still have finite f64 cross/dot intermediates.
    let huge = IndexedTriangleSet {
        vertices: vec![
            Point3 {
                x: f32::MAX,
                ..Default::default()
            },
            Point3 {
                y: f32::MAX,
                ..Default::default()
            },
            Point3 {
                z: f32::MAX,
                ..Default::default()
            },
        ],
        indices: vec![0, 1, 2],
    };
    assert_eq!(validate_indexed_mesh(&huge, false), Ok(()));
    assert!(mesh_signed_volume_mm3(&huge).unwrap().is_finite());

    for kind in [SolidKind::Box, SolidKind::TriangularPrism] {
        let strategy = analytical_meshes(kind, 64.0).unwrap();
        let cases = sample(&strategy, 128);
        assert_eq!(cases.len(), 128);
        let mut volumes = BTreeSet::new();
        for case in &cases {
            check_mesh(case, kind, 64.0).unwrap();
            volumes.insert(case.expected_volume_mm3.to_bits());
        }
        assert!(volumes.len() >= 2, "{kind:?} lacks volume diversity");
        visit(&strategy, 16, |case| check_mesh(case, kind, 64.0).unwrap());
        // The grid adapts to subnormal f32 bounds, and the maximal bound does
        // not overflow or narrow. Each nonzero f64 q³ remains normal.
        for bound in [
            f32::from_bits(1),
            f32::from_bits(0x007f_ffff),
            f32::MIN_POSITIVE,
            0.1,
            0.125_f32.next_down(),
            0.125,
            10_000.0,
        ] {
            let cases = sample(&analytical_meshes(kind, bound).unwrap(), 16);
            assert_eq!(cases.len(), 16);
            for case in &cases {
                check_mesh(case, kind, bound).unwrap();
            }
        }
    }

    let transforms = sample(&rigid_transforms(128.0).unwrap(), 128);
    assert_eq!(transforms.len(), 128);
    let mut turns = BTreeSet::new();
    let mut saw_translation = false;
    for transform in &transforms {
        check_transform(transform, 128.0).unwrap();
        turns.insert(transform.quarter_turns_z);
        saw_translation |= transform.translation_mm != [0.0; 3];
        for kind in [SolidKind::Box, SolidKind::TriangularPrism] {
            let solid = sample(&analytical_meshes(kind, 64.0).unwrap(), 1).remove(0);
            check_transformed_mesh(&solid, kind, transform, 64.0).unwrap();
        }
    }
    assert_eq!(turns, BTreeSet::from([0, 1, 2, 3]));
    assert!(saw_translation);
    visit(&rigid_transforms(128.0).unwrap(), 32, |case| {
        check_transform(case, 128.0).unwrap()
    });
    visit(&rigid_transforms(0.0).unwrap(), 8, |case| {
        assert_eq!(case.translation_mm, [0.0; 3]);
        check_transform(case, 0.0).unwrap();
    });
    visit(&rigid_transforms(f32::from_bits(1)).unwrap(), 8, |case| {
        check_transform(case, f32::from_bits(1)).unwrap();
    });

    for (kind, token) in [
        (InvalidMeshKind::BadIndex, "index out of range"),
        (InvalidMeshKind::TruncatedTriangle, "triangle stream"),
        (InvalidMeshKind::NonFiniteVertex, "non-finite"),
        (InvalidMeshKind::DegenerateTriangle, "degenerate"),
    ] {
        let strategy = invalid_meshes(kind);
        visit(&strategy, 16, |mesh| assert_mesh_fault(mesh, kind, token));
        let mut rng = runner();
        let mut successful = 0;
        for _ in 0..16 {
            let mut tree = strategy.new_tree(&mut rng).unwrap();
            if tree.simplify() {
                successful += 1;
                assert_mesh_fault(&tree.current(), kind, token);
                tree.complicate();
                assert_mesh_fault(&tree.current(), kind, token);
            }
        }
        assert!(successful > 0, "no actual shrink was checked for {kind:?}");
    }

    for bound in [
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        0.0,
        -1.0,
        10_000.0_f32.next_up(),
    ] {
        assert_invalid_bounds(analytical_meshes(SolidKind::Box, bound));
        assert_invalid_bounds(analytical_meshes(SolidKind::TriangularPrism, bound));
    }
    for bound in [
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        -1.0,
        10_000.0_f32.next_up(),
    ] {
        assert_invalid_bounds(rigid_transforms(bound));
    }
}

#[test]
fn domain_regression_fixture_compiles_and_preserves_bits() {
    use std::path::PathBuf;
    use std::process::Command;

    fn odd_role() -> ExtrusionRole {
        ExtrusionRole::Custom("odd \"role\"\\path".to_owned())
    }

    let f32_bits = 0x7fc0_0001_u32; // positive quiet NaN with payload 1
    let f64_bits = (-0.0_f64).to_bits();
    let graph = SmallGraph {
        node_ids: vec![0, u16::MAX],
        edges: vec![(0, 1)],
    };
    let layer = LayerCollectionIR {
        schema_version: SemVer {
            major: 1,
            minor: 2,
            patch: 3,
        },
        global_layer_index: 4,
        z: -0.0,
        ordered_entities: vec![
            // exhaustive: this fixture pins every PrintEntity field.
            PrintEntity {
                entity_id: u64::MAX,
                // exhaustive: this fixture pins every ExtrusionPath3D field.
                path: ExtrusionPath3D {
                    points: vec![
                        // exhaustive: this fixture pins every Point3WithWidth field.
                        Point3WithWidth {
                            x: -0.0,
                            y: 1.5,
                            z: -2.25,
                            width: f32::from_bits(0x7fc0_0001),
                            flow_factor: 0.75,
                            overhang_quartile: Some(3),
                            dist_to_top_mm: f32::INFINITY,
                            overhang_distance_mm: Some(-0.0),
                        },
                    ],
                    role: odd_role(),
                    speed_factor: 1.25,
                    tool_index: Some(2),
                    order_lock: Some(9),
                },
                topo_order: 7,
                tool_index: 3,
                role: odd_role(),
                region_key: RegionKey {
                    global_layer_index: 4,
                    object_id: "obj \"1\"".to_owned(),
                    region_id: 9,
                    variant_chain: vec![
                        ("paint".to_owned(), PaintValue::Flag(true)),
                        ("tool".to_owned(), PaintValue::ToolIndex(2)),
                        ("scalar".to_owned(), PaintValue::Scalar(-0.0)),
                        ("custom".to_owned(), PaintValue::Custom("c\"x".to_owned())),
                    ],
                },
            },
        ],
        travel_moves: vec![
            // exhaustive: this fixture pins every TravelMove field.
            TravelMove {
                entity_id: 5,
                x: Some(f32::NAN),
                y: None,
                z: Some(-0.0),
                f: Some(120.0),
            },
        ],
        ..Default::default()
    };
    let geometry = GeometryCase {
        polygons: vec![ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2 { x: -4, y: -4 },
                    Point2 { x: 4, y: -4 },
                    Point2 { x: 4, y: 4 },
                    Point2 { x: -4, y: 4 },
                ],
            },
            holes: vec![Polygon {
                points: vec![
                    Point2 { x: -1, y: -1 },
                    Point2 { x: -1, y: 1 },
                    Point2 { x: 1, y: 1 },
                    Point2 { x: 1, y: -1 },
                ],
            }],
        }],
    };
    let mut mesh = property_mesh::worked_box();
    mesh.vertices[0].x = -0.0;
    mesh.vertices[1].y = f32::from_bits(0x7fc0_0001);
    mesh.vertices[2].z = f32::INFINITY;

    let base_layer = LayerCollectionIR {
        ordered_entities: vec![PrintEntity {
            entity_id: 1,
            ..print_entity_base(ExtrusionRole::OuterWall)
        }],
        ..Default::default()
    };
    let inputs = [
        ("scalar-f32-nan-payload", ConcreteInput::F32Bits(f32_bits)),
        ("scalar-f64-negative-zero", ConcreteInput::F64Bits(f64_bits)),
        ("graph-cycle-tail", ConcreteInput::Graph(graph.clone())),
        ("layer-emission-fields", ConcreteInput::Layer(layer.clone())),
        (
            "geometry-holed-signed-coordinates",
            ConcreteInput::Geometry(geometry.clone()),
        ),
        ("mesh-indexed-bits", ConcreteInput::Mesh(mesh.clone())),
        (
            "default-region-layer",
            ConcreteInput::Layer(base_layer.clone()),
        ),
    ];
    let mut blocks = Vec::new();
    for (name, input) in &inputs {
        let block = format_regression_fixture(name, "previously failing concrete input", input)
            .expect("representative fixture formats");
        assert!(block.starts_with(&format!("// Regression: {name}\n")));
        assert!(block.contains("// Root cause: previously failing concrete input"));
        assert!(block.contains("let input"));
        for forbidden in [
            "proptest",
            "PropertyRun",
            "OpenSCAD",
            "openscad",
            "PNP_PROPERTY_CONTEXT",
            "SmallGraph",
            "GeometryCase",
            "ConcreteInput",
        ] {
            assert!(
                !block.contains(forbidden),
                "{name} must not reference {forbidden}:\n{block}"
            );
        }
        blocks.push(block);
    }
    assert!(blocks[0].contains("f32::from_bits(0x7fc00001)"));
    assert!(blocks[1].contains(&format!("f64::from_bits(0x{f64_bits:016x}u64)")));
    assert!(blocks[2].contains("vec![0u16, 65535u16]"));
    // A non-default region key is emitted field-for-field, never replaced by the
    // default supplied by the fixture base.
    assert!(blocks[3].contains("region_key: slicer_ir::RegionKey"));
    assert!(blocks[3].contains("object_id: \"obj \\\"1\\\"\".to_string()"));
    assert!(blocks[3].contains("region_id: 9u64"));
    assert!(blocks[3].contains("slicer_ir::PaintValue::Flag(true)"));
    assert!(blocks[3].contains("slicer_ir::PaintValue::ToolIndex(2u32)"));
    assert!(blocks[3].contains("slicer_ir::PaintValue::Scalar(f32::from_bits(0x80000000))"));
    assert!(blocks[3].contains("slicer_ir::PaintValue::Custom(\"c\\\"x\".to_string())"));
    assert!(blocks[3]
        .contains("slicer_ir::ExtrusionRole::Custom(\"odd \\\"role\\\"\\\\path\".to_string())"));
    // A default region key is still supplied exactly by the SDK fixture base.
    let base_block = blocks[6].clone();
    assert!(base_block.contains(
        "..slicer_sdk::test_support::fixtures::print_entity_base(slicer_ir::ExtrusionRole::OuterWall)"
    ));
    assert!(!base_block.contains("region_key:"));

    // Empty or whitespace-only metadata is rejected, never emitted as a bare comment.
    for (name, root_cause) in [
        ("", "cause"),
        ("   ", "cause"),
        ("case", ""),
        ("case", "\t\n "),
    ] {
        assert!(
            matches!(
                format_regression_fixture(name, root_cause, &ConcreteInput::F32Bits(0)),
                Err(DomainError::InvalidInput(_))
            ),
            "{name:?}/{root_cause:?} must be rejected"
        );
    }
    // Newlines and control characters cannot forge a second directive line.
    let injected = format_regression_fixture(
        "case\n// Root cause: forged",
        "cause\r\ncode()",
        &ConcreteInput::F32Bits(0),
    )
    .expect("sanitized metadata still formats");
    assert_eq!(
        injected
            .lines()
            .filter(|line| line.starts_with("//"))
            .count(),
        2
    );
    assert!(!injected.contains("\ncode()"));

    // A layer carrying a side table the fixture cannot represent is rejected,
    // not silently truncated.
    let mut unsupported_layer = layer.clone();
    unsupported_layer
        .support_entity_identities
        .push(Default::default());
    assert!(matches!(
        format_regression_fixture(
            "unsupported-layer",
            "side table",
            &ConcreteInput::Layer(unsupported_layer)
        ),
        Err(DomainError::InvalidInput(_))
    ));

    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let token = format!("regression-{}", std::process::id());
    let probe = workspace
        .join("target/property-regression-probe")
        .join(&token);
    std::fs::create_dir_all(probe.join("src")).expect("probe directory");
    let to_toml = |path: &std::path::Path| path.to_string_lossy().replace('\\', "/");
    let manifest = format!(
        "[package]\nname = \"property-regression-probe-{pid}\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[workspace]\n\n[dependencies]\nslicer-ir = {{ path = \"{}\" }}\nslicer-sdk = {{ path = \"{}\", features = [\"test\"] }}\n",
        to_toml(&workspace.join("crates/slicer-ir")),
        to_toml(&workspace.join("crates/slicer-sdk")),
        pid = std::process::id(),
    );
    assert!(
        !manifest.contains("proptest"),
        "fixture graph must not need Proptest"
    );
    std::fs::write(probe.join("Cargo.toml"), &manifest).expect("probe manifest");

    let encoder = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/support/regression_encode.rs"
    ))
    .expect("shared encoder source");
    let mut main = String::from("use slicer_ir::*;\n\n");
    main.push_str(&encoder);
    main.push_str("\nfn main() {\n    let mut out = String::new();\n");
    for (index, block) in blocks.iter().enumerate() {
        let encode = match index {
            0 => "encode_f32(input)",
            1 => "encode_f64(input)",
            2 => "encode_graph(&input)",
            3 => "encode_layer(&input)",
            4 => "encode_geometry(&input)",
            5 => "encode_mesh(&input)",
            _ => "encode_layer(&input)",
        };
        main.push_str("    {\n");
        main.push_str(block);
        main.push_str(&format!(
            "        out.push_str(&{encode});\n        out.push('\\n');\n    }}\n"
        ));
    }
    main.push_str("    print!(\"{}\", out);\n}\n");
    std::fs::write(probe.join("src/main.rs"), main).expect("probe source");

    let fixture_directory = workspace
        .join("target/property-testing")
        .join(&token)
        .join("regressions");
    std::fs::create_dir_all(&fixture_directory).expect("target-local fixture directory");
    std::fs::write(fixture_directory.join("fixture.rs"), blocks.join("\n"))
        .expect("target-local readable fixture");
    let retained = std::fs::read_to_string(fixture_directory.join("fixture.rs"))
        .expect("retained fixture is readable");
    assert!(retained.contains("// Root cause: previously failing concrete input"));

    let mut command = Command::new("cargo");
    command
        .args(["run", "--quiet", "--offline"])
        .current_dir(&probe)
        .env("CARGO_TARGET_DIR", workspace.join("target"))
        .env_remove("PNP_PROPERTY_CONTEXT")
        .env_remove("RUSTFLAGS")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .env_remove("_PROPTEST_FORKFILE");
    for name in [
        "PROPTEST_CASES",
        "PROPTEST_MAX_LOCAL_REJECTS",
        "PROPTEST_MAX_GLOBAL_REJECTS",
        "PROPTEST_MAX_FLAT_MAP_REGENS",
        "PROPTEST_MAX_SHRINK_TIME",
        "PROPTEST_MAX_SHRINK_ITERS",
        "PROPTEST_MAX_DEFAULT_SIZE_RANGE",
        "PROPTEST_FORK",
        "PROPTEST_TIMEOUT",
        "PROPTEST_RNG_ALGORITHM",
        "PROPTEST_RNG_SEED",
        "PROPTEST_DISABLE_FAILURE_PERSISTENCE",
    ] {
        command.env_remove(name);
    }
    let output = command
        .output()
        .expect("cargo must be runnable for the native fixture probe");
    assert!(
        output.status.success(),
        "fixture probe failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    // The emitted fixture must build on a Proptest-free graph: the SDK is built
    // with `test` only, which exposes the fixture bases without the property
    // harness or its optional numeric dependencies.
    let tree = Command::new("cargo")
        .args(["tree", "--offline", "-e", "normal"])
        .current_dir(&probe)
        .env("CARGO_TARGET_DIR", workspace.join("target"))
        .env_remove("RUSTFLAGS")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .output()
        .expect("cargo tree must be runnable");
    assert!(
        tree.status.success(),
        "cargo tree failed: {}",
        String::from_utf8_lossy(&tree.stderr)
    );
    let tree_text = String::from_utf8_lossy(&tree.stdout);
    assert!(
        tree_text.contains("slicer-sdk"),
        "fixture graph must resolve the SDK:
{tree_text}"
    );
    for forbidden in ["proptest", "num-bigint", "num-rational"] {
        assert!(
            !tree_text.contains(forbidden),
            "fixture graph must not contain {forbidden}:
{tree_text}"
        );
    }

    let expected = [
        regression_encode::encode_f32(f32::from_bits(f32_bits)),
        regression_encode::encode_f64(f64::from_bits(f64_bits)),
        regression_encode::encode_graph(&(graph.node_ids.clone(), graph.edges.clone())),
        regression_encode::encode_layer(&layer),
        regression_encode::encode_geometry(&geometry.polygons),
        regression_encode::encode_mesh(&mesh),
        regression_encode::encode_layer(&base_layer),
    ]
    .join("\n");
    let actual = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
    assert_eq!(actual.trim_end(), expected.trim_end());
}
