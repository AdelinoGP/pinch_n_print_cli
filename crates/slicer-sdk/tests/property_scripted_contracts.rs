//! AC-1/AC-N1: the scripted feature boundary and its typed contract.
//!
//! This target requires `property_testing_scad`, so there is no feature-off
//! configuration to assert here; the engine-free configurations are asserted
//! from the graph side in `xtask`'s `scripted_feature_isolation_and_union`
//! (default, `test`, `property_testing`, product CLI and discovered guests).
//! What this file proves is the enabled half: the engine edge is really active,
//! a typed recipe really reaches the adapter, and invalid capacities are
//! rejected before any render happens.

use proptest::strategy::{Strategy, ValueTree};
use proptest::test_runner::{Config, RngAlgorithm, RngSeed, TestRunner};

use slicer_ir::{ExPolygon, Point2, Polygon};
use slicer_sdk::test_support::property_testing::domains::{
    BoundaryRule, DomainError, GeometryBounds, GeometryCase, GeometryDomain, TopologyClass,
};
use slicer_sdk::test_support::property_testing::oracles::{
    mesh_signed_volume_mm3, point_in_region, validate_geometry, validate_indexed_mesh, PointLocation,
};
use slicer_sdk::test_support::property_testing::scripted::{
    emit_scad, evaluate_recipe, recipe_complexity, scripted_cases, Recipe2d, Recipe3d, RecipeBounds,
    ScadRecipe, ScriptedCase, ScriptedDomain, ScriptedError, SolidWitness,
};

#[path = "support/scripted.rs"]
mod scripted_support;

// The shared encoder is compiled for two targets; this one uses two helpers.
#[allow(dead_code)]
#[path = "support/regression_encode.rs"]
mod regression_encode;

fn bounds() -> RecipeBounds {
    RecipeBounds::default()
}

fn options(recipe: &RecipeBounds) -> slicer_scad::ScadEvalOptions {
    scripted_support::options(recipe, 20_000, 200_000, 400_000, 10_000_000, 0x5c4d_0004)
}

#[test]
fn scripted_feature_isolation_and_union() {
    let info = slicer_scad::engine_info()
        .expect("property_testing_scad must enable the native adapter and its engine");
    assert_eq!(
        info.backend, "manifold_cpp",
        "the scripted feature must reach the measured native backend"
    );

    let recipe_bounds = bounds();
    let recipe = ScadRecipe::Contours(Recipe2d::Rectangle {
        size_units: [20_000, 12_345],
    });
    let source = emit_scad(&recipe, &recipe_bounds).expect("emit");
    assert_eq!(source.label, "recipe.scad");
    assert!(
        source.text.contains("square([2.0000, 1.2345]);"),
        "canonical units must reach the source as exact millimetres, got:\n{}",
        source.text
    );
    let output = evaluate_recipe(&recipe, &options(&recipe_bounds), &recipe_bounds)
        .expect("a bounded rectangle must evaluate");
    let slicer_scad::ScadGeometry::Contours(contours) = &output.geometry else {
        panic!("a square is 2D geometry, got {:?}", output.geometry);
    };
    assert_eq!(contours.len(), 1);
    assert_eq!(
        contours[0].contour.points,
        vec![
            Point2 { x: 0, y: 0 },
            Point2 { x: 20_000, y: 0 },
            Point2 { x: 20_000, y: 12_345 },
            Point2 { x: 0, y: 12_345 },
        ],
        "the adapter must return the exact canonical-unit rectangle"
    );
    assert_eq!(output.provenance.engine, info);

    // The complete replay-record contract: a new field on ScriptedCase must
    // fail here rather than be silently dropped from the artifact round trip.
    let case = ScriptedCase { // exhaustive: complete replay-record contract
        recipe: recipe.clone(),
        recipe_bounds,
        source,
        options: options(&recipe_bounds),
        output,
    };
    let json = serde_json::to_string(&case).expect("serialize a scripted case");
    let restored: ScriptedCase = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored, case, "the replay record is lossless");
}

#[test]
fn scripted_rejects_invalid_recipe_bounds_before_sampling() {
    let recipe_bounds = bounds();

    let mut bad_bounds = recipe_bounds;
    bad_bounds.max_nodes = 0;
    assert!(matches!(
        emit_scad(
            &ScadRecipe::Contours(Recipe2d::Rectangle {
                size_units: [10, 10]
            }),
            &bad_bounds
        ),
        Err(ScriptedError::Domain(DomainError::InvalidBounds(_)))
    ));

    let zero_dimension = ScadRecipe::Solid(Recipe3d::Box {
        size_units: [10, 0, 10],
    });
    assert!(matches!(
        emit_scad(&zero_dimension, &recipe_bounds),
        Err(ScriptedError::Domain(DomainError::InvalidInput(_)))
    ));

    let too_many_turns = ScadRecipe::Contours(Recipe2d::QuarterTurn {
        turns: 4,
        child: Box::new(Recipe2d::Rectangle {
            size_units: [10, 10],
        }),
    });
    assert!(emit_scad(&too_many_turns, &recipe_bounds).is_err());

    let too_few_facets = ScadRecipe::Contours(Recipe2d::Disk {
        radius_units: 100,
        facets: 2,
    });
    assert!(emit_scad(&too_few_facets, &recipe_bounds).is_err());

    // A recipe that is fine on its own but needs more adapter AST capacity than
    // the supplied options allow must be rejected before the render.
    let rectangle = ScadRecipe::Contours(Recipe2d::Rectangle {
        size_units: [10, 10],
    });
    let mut starved = options(&recipe_bounds);
    starved.limits.max_ast_nodes = 1;
    match evaluate_recipe(&rectangle, &starved, &recipe_bounds) {
        Err(ScriptedError::Domain(DomainError::InvalidBounds(message))) => {
            assert!(message.contains("max_ast_nodes"), "got {message}");
        }
        other => panic!("expected a pre-render capacity rejection, got {other:?}"),
    }

    let mut starved_facets = options(&recipe_bounds);
    starved_facets.limits.max_facets = 2;
    assert!(evaluate_recipe(&rectangle, &starved_facets, &recipe_bounds).is_err());

    // Extrusion height belongs to 2D-only programs; a solid recipe must reject
    // it through the adapter rather than silently render a mixed scene.
    let solid = ScadRecipe::Solid(Recipe3d::Box {
        size_units: [10, 10, 10],
    });
    let mut extrude = options(&recipe_bounds);
    extrude.extrude_height_mm = Some(1.0);
    match evaluate_recipe(&solid, &extrude, &recipe_bounds) {
        Err(ScriptedError::Adapter(error)) => {
            assert_eq!(error.kind, slicer_scad::ScadErrorKind::ExtrusionHeightOn3d)
        }
        other => panic!("expected an adapter rejection, got {other:?}"),
    }
}

#[test]
fn scripted_recipe_complexity_counts_explicit_leaves() {
    let rectangle = ScadRecipe::Contours(Recipe2d::Rectangle {
        size_units: [4, 3],
    });
    assert_eq!(recipe_complexity(&rectangle).unwrap(), (1, 0, 7));

    let polygons = vec![ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 { x: 0, y: 0 },
                Point2 { x: 4, y: 0 },
                Point2 { x: 4, y: 4 },
                Point2 { x: 0, y: 4 },
            ],
        },
        holes: vec![Polygon {
            points: vec![
                Point2 { x: 1, y: 1 },
                Point2 { x: 1, y: 3 },
                Point2 { x: 3, y: 3 },
                Point2 { x: 3, y: 1 },
            ],
        }],
    }];
    let leaf = ScadRecipe::Contours(Recipe2d::Polygon { polygons });
    // Eight explicit vertices, and the absolute coordinate sum of both rings
    // (16 + 16) plus the vertex count itself, which is an integral parameter.
    assert_eq!(recipe_complexity(&leaf).unwrap(), (1, 8, 40));
}

// ---------------------------------------------------------------------------
// Step 2: emission, independent references, topology and solid witnesses
// ---------------------------------------------------------------------------

fn scenario_bounds() -> RecipeBounds {
    // exhaustive: pins the contract-test scenario bounds deliberately
    RecipeBounds {
        max_nodes: 24,
        max_depth: 6,
        max_facets: 32,
        max_source_bytes: 16 * 1024,
        max_evaluations: 64,
    }
}

fn scenario_options(recipe: &RecipeBounds, units: i64) -> slicer_scad::ScadEvalOptions {
    scripted_support::options(recipe, units, 400_000, 800_000, 20_000_000, 0x5c4d_0005)
}

fn geometry_domain(class: TopologyClass, units: i64) -> GeometryDomain {
    // exhaustive: pins the contract-test scenario bounds deliberately
    GeometryDomain {
        class,
        min_holes: 0,
        min_components: usize::from(class != TopologyClass::Empty),
        boundary: BoundaryRule::Strict,
        // exhaustive: pins the scenario geometry capacities deliberately
        bounds: GeometryBounds {
            max_abs_units: units,
            max_vertices: 96,
            max_holes: 8,
            max_components: 8,
            max_shrink_candidates: 64,
        },
    }
}

fn evaluate(
    recipe: &ScadRecipe,
    recipe_bounds: &RecipeBounds,
    units: i64,
) -> slicer_scad::ScadEvalOutput {
    evaluate_recipe(recipe, &scenario_options(recipe_bounds, units), recipe_bounds)
        .unwrap_or_else(|error| panic!("evaluate {recipe:?}: {error}"))
}

fn contours_of(output: &slicer_scad::ScadEvalOutput) -> Vec<ExPolygon> {
    match &output.geometry {
        slicer_scad::ScadGeometry::Contours(contours) => contours.clone(),
        other => panic!("expected contours, got {other:?}"),
    }
}

fn mesh_of(output: &slicer_scad::ScadEvalOutput) -> slicer_ir::IndexedTriangleSet {
    match &output.geometry {
        slicer_scad::ScadGeometry::Mesh(mesh) => mesh.clone(),
        other => panic!("expected a mesh, got {other:?}"),
    }
}

fn ring(points: &[(i64, i64)]) -> Polygon {
    Polygon {
        points: points
            .iter()
            .map(|&(x, y)| Point2 { x, y })
            .collect(),
    }
}

#[test]
fn scripted_emission_and_independent_recipe_references() {
    let bounds = scenario_bounds();
    let units = 40_000;

    // Typed primitives: exact analytical rectangle area and bounds.
    let rectangle = ScadRecipe::Contours(Recipe2d::Rectangle {
        size_units: [12_000, 7_500],
    });
    let source = emit_scad(&rectangle, &bounds).expect("emit");
    scripted_support::assert_self_contained(&source.text);
    assert_eq!(
        source.text,
        emit_scad(&rectangle, &bounds).expect("re-emit").text,
        "emission must be deterministic"
    );
    let contours = contours_of(&evaluate(&rectangle, &bounds, units));
    assert_eq!(contours.len(), 1);
    let rectangle_area = scripted_support::forest_area_mm2(&contours);
    assert!(
        (rectangle_area - 1.2 * 0.75).abs() < 1e-12,
        "a rectangle keeps its exact analytical area, got {rectangle_area}"
    );
    assert_eq!(
        scripted_support::ring_bbox(&contours[0].contour),
        (0, 0, 12_000, 7_500)
    );

    // A faceted disk is a regular polygon, never an ideal circle.
    let disk = ScadRecipe::Contours(Recipe2d::Disk {
        radius_units: 4_000,
        facets: 12,
    });
    let contours = contours_of(&evaluate(&disk, &bounds, units));
    assert_eq!(contours.len(), 1);
    assert_eq!(
        contours[0].contour.points.len(),
        12,
        "an explicit facet count survives emission and conversion"
    );
    let analytic = scripted_support::regular_polygon_area_mm2(4_000.0, 12);
    let observed = scripted_support::forest_area_mm2(&contours);
    // The adapter quantizes vertices to canonical units (1e-4 mm), so the
    // faceted area can differ from the analytic regular polygon by roughly
    // perimeter x half a unit -- about 3e-4 relative at this radius.
    assert!(
        (observed - analytic).abs() <= 1e-3 * analytic,
        "faceted disk area {observed} must match the regular-polygon reference {analytic}"
    );
    let ideal = std::f64::consts::PI * 0.4 * 0.4;
    assert!(
        observed < ideal * 0.98,
        "a 12-facet disk must not be an ideal circle: {observed} vs {ideal}"
    );

    // An explicit non-star-shaped, holed leaf: exact area, hole count and
    // exact point observations of the intended region.
    let notched = ring(&[
        (0, 0),
        (9_000, 0),
        (9_000, 6_000),
        (5_000, 6_000),
        (5_000, 3_000),
        (4_000, 3_000),
        (4_000, 6_000),
        (0, 6_000),
    ]);
    assert!(
        scripted_support::empty_kernel_wall_pair(&notched),
        "the explicit leaf must be a proven non-star-shaped witness"
    );
    let hole = ring(&[
        (1_000, 1_000),
        (1_000, 2_000),
        (2_000, 2_000),
        (2_000, 1_000),
    ]);
    let intended = vec![ExPolygon {
        contour: notched.clone(),
        holes: vec![hole.clone()],
    }];
    let polygon = ScadRecipe::Contours(Recipe2d::Polygon {
        polygons: intended.clone(),
    });
    let contours = contours_of(&evaluate(&polygon, &bounds, units));
    assert_eq!(scripted_support::hole_count(&contours), 1);
    assert_eq!(
        scripted_support::forest_area_mm2(&contours),
        scripted_support::forest_area_mm2(&intended),
        "an explicit leaf keeps its exact intended area"
    );
    for (probe, expected) in [
        (Point2 { x: 500, y: 500 }, PointLocation::Inside),
        (Point2 { x: 1_500, y: 1_500 }, PointLocation::Outside),
        (Point2 { x: 4_500, y: 4_500 }, PointLocation::Outside),
        (Point2 { x: 6_000, y: 4_500 }, PointLocation::Inside),
    ] {
        assert_eq!(
            point_in_region(probe, &contours).expect("point probe"),
            expected,
            "probe ({}, {}) must keep its intended location",
            probe.x,
            probe.y
        );
    }

    // Transforms: a quarter turn maps the rectangle exactly.
    let turned = ScadRecipe::Contours(Recipe2d::QuarterTurn {
        turns: 1,
        child: Box::new(Recipe2d::Rectangle {
            size_units: [4_000, 2_000],
        }),
    });
    let contours = contours_of(&evaluate(&turned, &bounds, units));
    assert_eq!(
        scripted_support::ring_bbox(&contours[0].contour),
        (-2_000, 0, 0, 4_000),
        "a quarter turn around Z maps +x to +y"
    );

    // Union of disjoint rectangles.
    let disjoint = ScadRecipe::Contours(Recipe2d::Union(vec![
        Recipe2d::Rectangle {
            size_units: [3_000, 3_000],
        },
        Recipe2d::Translate {
            offset_units: [5_000, 0],
            child: Box::new(Recipe2d::Rectangle {
                size_units: [3_000, 3_000],
            }),
        },
    ]));
    let contours = contours_of(&evaluate(&disjoint, &bounds, units));
    assert_eq!(contours.len(), 2);
    assert!((scripted_support::forest_area_mm2(&contours) - 2.0 * 0.3 * 0.3).abs() < 1e-9);

    // Difference: an inner rectangle becomes exactly one hole.
    let holed = ScadRecipe::Contours(Recipe2d::Difference {
        base: Box::new(Recipe2d::Rectangle {
            size_units: [8_000, 8_000],
        }),
        cuts: vec![Recipe2d::Translate {
            offset_units: [2_000, 2_000],
            child: Box::new(Recipe2d::Rectangle {
                size_units: [4_000, 4_000],
            }),
        }],
    });
    let contours = contours_of(&evaluate(&holed, &bounds, units));
    assert_eq!(scripted_support::hole_count(&contours), 1);
    let expected = 0.8 * 0.8 - 0.4 * 0.4;
    assert!(
        (scripted_support::forest_area_mm2(&contours) - expected).abs() < 1e-9,
        "difference area must be the analytical difference"
    );

    // Intersection: two overlapping rectangles keep their overlap.
    let overlap = ScadRecipe::Contours(Recipe2d::Intersection(vec![
        Recipe2d::Rectangle {
            size_units: [6_000, 6_000],
        },
        Recipe2d::Translate {
            offset_units: [2_000, 2_000],
            child: Box::new(Recipe2d::Rectangle {
                size_units: [6_000, 6_000],
            }),
        },
    ]));
    let contours = contours_of(&evaluate(&overlap, &bounds, units));
    assert_eq!(contours.len(), 1);
    assert!((scripted_support::forest_area_mm2(&contours) - 0.4 * 0.4).abs() < 1e-9);

    // Extrusion: volume is the planar area times the height.
    let extruded = ScadRecipe::Solid(Recipe3d::LinearExtrude {
        height_units: 5_000,
        profile: Box::new(Recipe2d::Rectangle {
            size_units: [8_000, 4_000],
        }),
    });
    let mesh = mesh_of(&evaluate(&extruded, &bounds, units));
    validate_indexed_mesh(&mesh, true).expect("a closed extruded solid");
    let volume = mesh_signed_volume_mm3(&mesh).expect("signed volume");
    let expected = 0.8 * 0.4 * 0.5;
    assert!(
        (volume - expected).abs() <= 1e-5,
        "extrusion volume {volume} must match the analytical {expected}"
    );
    let (min, max) = scripted_support::mesh_bounds(&mesh);
    assert!((min[2] - 0.0).abs() < 1e-6 && (max[2] - 0.5).abs() < 1e-6);
}

#[test]
fn scripted_topology_and_solid_witnesses() {
    let bounds = scenario_bounds();
    let units = 40_000;
    let mut runner = TestRunner::new(Config {
        cases: 8,
        rng_algorithm: RngAlgorithm::ChaCha,
        rng_seed: RngSeed::Fixed(0x5c4d_0006),
        failure_persistence: None,
        ..Config::default()
    });

    for class in [
        TopologyClass::Convex,
        TopologyClass::Concave,
        TopologyClass::NonStarShaped,
        TopologyClass::Holed,
        TopologyClass::Disconnected,
        TopologyClass::Thin,
        TopologyClass::NearDegenerate,
    ] {
        let domain = ScriptedDomain {
            geometry: geometry_domain(class, units),
            solid_witness: None,
            bounds,
        };
        let strategy = scripted_cases(domain.clone(), scenario_options(&bounds, units))
            .expect("a scripted contour strategy");
        let mut tree = strategy
            .new_tree(&mut runner)
            .expect("a certified contour case");
        let case = tree.current();
        let contours = contours_of(&case.output);
        validate_geometry(
            &GeometryCase {
                polygons: contours.clone(),
            },
            &domain.geometry,
        )
        .expect("the converted case satisfies its domain");
        match class {
            TopologyClass::Convex => assert!(!contours
                .iter()
                .any(|p| scripted_support::has_reflex_turn(&p.contour))),
            TopologyClass::Concave => assert!(contours
                .iter()
                .any(|p| scripted_support::has_reflex_turn(&p.contour))),
            TopologyClass::NonStarShaped => assert!(
                contours
                    .iter()
                    .any(|p| scripted_support::empty_kernel_wall_pair(&p.contour)),
                "the non-star witness must have a proven empty visibility kernel"
            ),
            TopologyClass::Holed => {
                assert!(scripted_support::hole_count(&contours) >= 1);
                for component in &contours {
                    for hole in &component.holes {
                        for point in &hole.points {
                            assert_eq!(
                                point_in_region(*point, std::slice::from_ref(component))
                                    .expect("hole probe"),
                                PointLocation::Boundary,
                                "a hole vertex lies on its own hole boundary"
                            );
                        }
                    }
                }
            }
            TopologyClass::Disconnected => assert!(contours.len() >= 2),
            TopologyClass::Thin => {
                let any = contours.iter().any(|component| {
                    let (min_x, min_y, max_x, max_y) =
                        scripted_support::ring_bbox(&component.contour);
                    let min_dim = (max_x - min_x).min(max_y - min_y);
                    min_dim >= 16
                        && scripted_support::nonadjacent_edge_clearance(&component.contour)
                            <= (min_dim as f64) / 16.0
                });
                assert!(any, "the thin witness needs a measured edge clearance");
            }
            TopologyClass::NearDegenerate => {
                let unit_gap = contours.iter().any(|component| {
                    (scripted_support::nonadjacent_edge_clearance(&component.contour) - 1.0).abs()
                        < 1e-9
                });
                assert!(
                    unit_gap,
                    "the near-degenerate witness needs a one-unit clearance"
                );
                // Almost-collinearity is a distinct obligation from the one-unit
                // neck, so it is proven over the family's reachable population
                // rather than required on one tree.
                let mut almost_collinear = contours.iter().any(|component| {
                    scripted_support::has_almost_collinear_vertex(&component.contour)
                });
                for _ in 0..16 {
                    if almost_collinear {
                        break;
                    }
                    let candidate = strategy
                        .new_tree(&mut runner)
                        .expect("a near-degenerate case");
                    almost_collinear = match &candidate.current().output.geometry {
                        slicer_scad::ScadGeometry::Contours(polygons) => polygons.iter().any(
                            |component| {
                                scripted_support::has_almost_collinear_vertex(&component.contour)
                            },
                        ),
                        _ => false,
                    };
                }
                assert!(
                    almost_collinear,
                    "the near-degenerate family must also reach an almost-collinear edge"
                );
            }
            TopologyClass::Empty => unreachable!("not in this loop"),
        }

        // Shrinking must keep the witness: every exposed value re-validates.
        for _ in 0..16 {
            let changed = tree.simplify();
            let case = tree.current();
            validate_geometry(
                &GeometryCase {
                    polygons: contours_of(&case.output),
                },
                &domain.geometry,
            )
            .expect("every exposed shrink value keeps its domain witness");
            if !changed {
                break;
            }
        }
    }

    // Explicit Empty: a recipe that really evaluates to no geometry.
    let empty_domain = ScriptedDomain {
        geometry: geometry_domain(TopologyClass::Empty, units),
        solid_witness: None,
        bounds,
    };
    let strategy = scripted_cases(empty_domain, scenario_options(&bounds, units))
        .expect("an empty strategy");
    let tree = strategy
        .new_tree(&mut runner)
        .expect("a certified empty case");
    assert!(matches!(
        tree.current().output.geometry,
        slicer_scad::ScadGeometry::Empty
    ));

    // Negative controls on the required topology observations.
    let holed_polygons = contours_of(&evaluate(
        &ScadRecipe::Contours(Recipe2d::Difference {
            base: Box::new(Recipe2d::Rectangle {
                size_units: [8_000, 8_000],
            }),
            cuts: vec![Recipe2d::Translate {
                offset_units: [2_000, 2_000],
                child: Box::new(Recipe2d::Rectangle {
                    size_units: [4_000, 4_000],
                }),
            }],
        }),
        &bounds,
        units,
    ));
    let mut lost_hole = holed_polygons.clone();
    lost_hole[0].holes.clear();
    assert!(
        validate_geometry(
            &GeometryCase {
                polygons: lost_hole
            },
            &geometry_domain(TopologyClass::Holed, units)
        )
        .is_err(),
        "losing the hole must be rejected, not silently accepted"
    );
    assert!(validate_geometry(
        &GeometryCase {
            polygons: holed_polygons
        },
        &geometry_domain(TopologyClass::Holed, units)
    )
    .is_ok());
    let disjoint = contours_of(&evaluate(
        &ScadRecipe::Contours(Recipe2d::Union(vec![
            Recipe2d::Rectangle {
                size_units: [3_000, 3_000],
            },
            Recipe2d::Translate {
                offset_units: [5_000, 0],
                child: Box::new(Recipe2d::Rectangle {
                    size_units: [3_000, 3_000],
                }),
            },
        ])),
        &bounds,
        units,
    ));
    let mut lost_body = disjoint.clone();
    lost_body.pop();
    assert!(
        validate_geometry(
            &GeometryCase {
                polygons: lost_body
            },
            &geometry_domain(TopologyClass::Disconnected, units)
        )
        .is_err(),
        "losing a body must be rejected"
    );

    // Solid witnesses: fixed controlled scenes with independent mesh probes.
    let cavity = ScadRecipe::Solid(Recipe3d::Difference {
        base: Box::new(Recipe3d::Box {
            size_units: [8_000, 8_000, 8_000],
        }),
        cuts: vec![Recipe3d::Translate {
            offset_units: [2_000, 2_000, 2_000],
            child: Box::new(Recipe3d::Box {
                size_units: [4_000, 4_000, 4_000],
            }),
        }],
    });
    let cavity_mesh = mesh_of(&evaluate(&cavity, &bounds, units));
    validate_indexed_mesh(&cavity_mesh, true).expect("a closed cavity scene");
    assert!(
        !scripted_support::point_inside_mesh(
            &cavity_mesh,
            scripted_support::units_point([4_000, 4_000, 4_000])
        ),
        "the enclosed void centre must be outside the solid"
    );
    let solid_mesh = mesh_of(&evaluate(
        &ScadRecipe::Solid(Recipe3d::Box {
            size_units: [8_000, 8_000, 8_000],
        }),
        &bounds,
        units,
    ));
    assert!(
        scripted_support::point_inside_mesh(
            &solid_mesh,
            scripted_support::units_point([4_000, 4_000, 4_000])
        ),
        "the cavity-lost control would be inside"
    );
    let cavity_volume = mesh_signed_volume_mm3(&cavity_mesh).expect("cavity volume");
    let solid_volume = mesh_signed_volume_mm3(&solid_mesh).expect("solid volume");
    assert!(
        (solid_volume - cavity_volume - 0.4 * 0.4 * 0.4).abs() <= 1e-5,
        "the cavity must remove exactly its analytical volume"
    );

    let bridge = ScadRecipe::Solid(Recipe3d::Union(vec![
        Recipe3d::Box {
            size_units: [2_000, 2_000, 2_000],
        },
        Recipe3d::Translate {
            offset_units: [4_000, 0, 0],
            child: Box::new(Recipe3d::Box {
                size_units: [2_000, 2_000, 2_000],
            }),
        },
        Recipe3d::Translate {
            offset_units: [0, 0, 2_000],
            child: Box::new(Recipe3d::Box {
                size_units: [6_000, 2_000, 1_000],
            }),
        },
    ]));
    let bridge_mesh = mesh_of(&evaluate(&bridge, &bounds, units));
    validate_indexed_mesh(&bridge_mesh, true).expect("a closed bridge scene");
    let inside = |point: [i64; 3]| {
        scripted_support::point_inside_mesh(&bridge_mesh, scripted_support::units_point(point))
    };
    assert!(inside([3_000, 1_000, 2_500]), "the bridge span is solid");
    assert!(
        !inside([3_000, 1_000, 1_500]),
        "the bridge span has no material below"
    );
    assert!(
        inside([1_000, 1_000, 2_500]) && inside([5_000, 1_000, 2_500]),
        "the span is bridged on both sides"
    );
    assert!(
        inside([1_000, 1_000, 1_500]) && inside([5_000, 1_000, 1_500]),
        "both pillars support the beam"
    );

    let no_bridge = ScadRecipe::Solid(Recipe3d::Union(vec![
        Recipe3d::Box {
            size_units: [2_000, 2_000, 2_000],
        },
        Recipe3d::Translate {
            offset_units: [4_000, 0, 0],
            child: Box::new(Recipe3d::Box {
                size_units: [2_000, 2_000, 2_000],
            }),
        },
    ]));
    let no_bridge_mesh = mesh_of(&evaluate(&no_bridge, &bounds, units));
    assert!(
        !scripted_support::point_inside_mesh(
            &no_bridge_mesh,
            scripted_support::units_point([3_000, 1_000, 2_500])
        ),
        "losing the beam must remove the unsupported span"
    );

    let overhang = ScadRecipe::Solid(Recipe3d::Union(vec![
        Recipe3d::Box {
            size_units: [2_000, 2_000, 2_000],
        },
        Recipe3d::Translate {
            offset_units: [0, 0, 2_000],
            child: Box::new(Recipe3d::Box {
                size_units: [4_000, 2_000, 1_000],
            }),
        },
    ]));
    let overhang_mesh = mesh_of(&evaluate(&overhang, &bounds, units));
    validate_indexed_mesh(&overhang_mesh, true).expect("a closed overhang scene");
    let inside_overhang = |point: [i64; 3]| {
        scripted_support::point_inside_mesh(&overhang_mesh, scripted_support::units_point(point))
    };
    assert!(
        inside_overhang([3_000, 1_000, 2_500]),
        "the plate overhangs its pillar"
    );
    assert!(
        !inside_overhang([3_000, 1_000, 1_500]),
        "nothing supports the overhanging plate"
    );
    assert!(
        !inside_overhang([4_500, 1_000, 2_500]),
        "the plate ends before 4.5 mm"
    );
    assert!(
        inside_overhang([1_000, 1_000, 1_500]),
        "the pillar reaches z=2 mm"
    );

    let no_overhang = ScadRecipe::Solid(Recipe3d::Union(vec![
        Recipe3d::Box {
            size_units: [2_000, 2_000, 2_000],
        },
        Recipe3d::Translate {
            offset_units: [0, 0, 2_000],
            child: Box::new(Recipe3d::Box {
                size_units: [2_000, 2_000, 1_000],
            }),
        },
    ]));
    let no_overhang_mesh = mesh_of(&evaluate(&no_overhang, &bounds, units));
    assert!(
        !scripted_support::point_inside_mesh(
            &no_overhang_mesh,
            scripted_support::units_point([3_000, 1_000, 2_500])
        ),
        "shrinking the plate to the pillar footprint must remove the overhang"
    );

    let separated = ScadRecipe::Solid(Recipe3d::Union(vec![
        Recipe3d::Box {
            size_units: [2_000, 2_000, 2_000],
        },
        Recipe3d::Translate {
            offset_units: [4_000, 0, 0],
            child: Box::new(Recipe3d::Box {
                size_units: [2_000, 2_000, 2_000],
            }),
        },
    ]));
    let separated_mesh = mesh_of(&evaluate(&separated, &bounds, units));
    assert!(
        scripted_support::point_inside_mesh(
            &separated_mesh,
            scripted_support::units_point([1_000, 1_000, 1_000])
        ) && scripted_support::point_inside_mesh(
            &separated_mesh,
            scripted_support::units_point([5_000, 1_000, 1_000])
        ) && !scripted_support::point_inside_mesh(
            &separated_mesh,
            scripted_support::units_point([3_000, 1_000, 1_000])
        ),
        "two separated bodies must be separated by real empty space"
    );

    // The strategy path certifies every witness it is asked for.
    for witness in [
        SolidWitness::ClosedSolid,
        SolidWitness::Cavity,
        SolidWitness::Bridge,
        SolidWitness::Overhang,
        SolidWitness::SeparatedBodies,
    ] {
        let domain = ScriptedDomain {
            geometry: geometry_domain(TopologyClass::Convex, units),
            solid_witness: Some(witness),
            bounds,
        };
        let strategy = scripted_cases(domain, scenario_options(&bounds, units))
            .unwrap_or_else(|error| panic!("{witness:?} strategy: {error}"));
        let tree = strategy
            .new_tree(&mut runner)
            .unwrap_or_else(|reason| panic!("{witness:?} case: {reason}"));
        let mesh = mesh_of(&tree.current().output);
        validate_indexed_mesh(&mesh, true).expect("every witness scene is a closed mesh");
        assert!(mesh_signed_volume_mm3(&mesh).expect("volume") > 0.0);
    }
}

include!("support/step3_contracts.rs");

include!("support/step4_contracts.rs");


// ---------------------------------------------------------------------------
// AC-N1 and generator-reachability contracts
// ---------------------------------------------------------------------------

#[test]
fn scripted_rejects_bounds_invalid_output_and_fatal_adapter_diagnostics() {
    let bounds = scenario_bounds();
    let units = 40_000;
    let options = scenario_options(&bounds, units);

    // Post-conversion damage is refused by the adapter itself. A disk cut whose
    // extent reaches the base's edges produces raw rings that touch, which the
    // adapter classifies as `invalid_contours` rather than repairing.
    let touching = ScadRecipe::Contours(Recipe2d::Difference {
        base: Box::new(Recipe2d::Rectangle {
            size_units: [8_000, 8_000],
        }),
        cuts: vec![Recipe2d::Translate {
            offset_units: [4_000, 4_000],
            child: Box::new(Recipe2d::Disk {
                radius_units: 4_000,
                facets: 8,
            }),
        }],
    });
    match evaluate_recipe(&touching, &options, &bounds) {
        Err(ScriptedError::Adapter(error)) => assert_eq!(
            error.kind,
            slicer_scad::ScadErrorKind::InvalidContours,
            "a touching converted ring must be a hard conversion rejection"
        ),
        other => panic!("a touching cut must not be accepted, got {other:?}"),
    }

    // A difference that exactly cancels its base evaluates to the explicit empty
    // geometry; no non-empty domain may accept it.
    let cancelled = ScadRecipe::Contours(Recipe2d::Difference {
        base: Box::new(Recipe2d::Rectangle {
            size_units: [4_000, 4_000],
        }),
        cuts: vec![Recipe2d::Rectangle {
            size_units: [4_000, 4_000],
        }],
    });
    let cancelled_output = evaluate(&cancelled, &bounds, units);
    assert!(matches!(
        cancelled_output.geometry,
        slicer_scad::ScadGeometry::Empty
    ));
    assert!(
        validate_geometry(
            &GeometryCase {
                polygons: Vec::new()
            },
            &geometry_domain(TopologyClass::Convex, units)
        )
        .is_err(),
        "a collapsed difference must not satisfy a non-empty domain"
    );

    // Invalid converted meshes are rejected before they can be exposed.
    let solid = ScadRecipe::Solid(Recipe3d::Box {
        size_units: [6_000, 6_000, 6_000],
    });
    let mesh = mesh_of(&evaluate(&solid, &bounds, units));
    validate_indexed_mesh(&mesh, true).expect("the undamaged box is closed");
    let mut damaged = mesh;
    damaged.indices.truncate(damaged.indices.len() - 3);
    assert!(
        validate_indexed_mesh(&damaged, true).is_err(),
        "dropping a face must be rejected, not accepted as a closed solid"
    );

    // Constructor prevalidation: an adapter capacity below the emitter bound is
    // rejected before generation rather than becoming an all-rejected population.
    let mut starved_geometry = options.clone();
    starved_geometry.limits.max_geometry_nodes = 1;
    assert!(matches!(
        scripted_cases(
            ScriptedDomain {
                geometry: geometry_domain(TopologyClass::Holed, units),
                solid_witness: None,
                bounds,
            },
            starved_geometry
        ),
        Err(ScriptedError::Domain(DomainError::InvalidBounds(_)))
    ));
    let mut starved_depth = options.clone();
    starved_depth.limits.max_ast_depth = 1;
    assert!(matches!(
        scripted_cases(
            ScriptedDomain {
                geometry: geometry_domain(TopologyClass::Holed, units),
                solid_witness: None,
                bounds,
            },
            starved_depth
        ),
        Err(ScriptedError::Domain(DomainError::InvalidBounds(_)))
    ));

    // Invalid adapter limits are rejected before sampling, including a zero fuel
    // budget (0 is not "unlimited").
    let mut zero_fuel = options.clone();
    zero_fuel.limits.evaluation_steps = 0;
    assert!(matches!(
        scripted_cases(
            ScriptedDomain {
                geometry: geometry_domain(TopologyClass::Holed, units),
                solid_witness: None,
                bounds,
            },
            zero_fuel
        ),
        Err(ScriptedError::Domain(DomainError::InvalidBounds(_)))
    ));

    // A solid domain whose constituent planar topology is not what the bounded
    // solid families build is refused instead of silently ignored.
    let unsupported_constituent = ScriptedDomain {
        geometry: geometry_domain(TopologyClass::Holed, units),
        solid_witness: Some(SolidWitness::ClosedSolid),
        bounds,
    };
    assert!(matches!(
        scripted_cases(unsupported_constituent, options.clone()),
        Err(ScriptedError::Domain(DomainError::UnsupportedDomain(_)))
    ));

    // The solid branch validates its constituent planar domain too.
    let impossible = ScriptedDomain {
        // exhaustive: pins the impossible-capacity control deliberately
        geometry: GeometryDomain {
            class: TopologyClass::Convex,
            min_holes: 0,
            min_components: 1,
            boundary: BoundaryRule::Strict,
            // exhaustive: pins the impossible-capacity control deliberately
            bounds: GeometryBounds {
                max_abs_units: units,
                max_vertices: 96,
                max_holes: 8,
                max_components: 0,
                max_shrink_candidates: 16,
            },
        },
        solid_witness: Some(SolidWitness::ClosedSolid),
        bounds,
    };
    assert!(
        scripted_cases(impossible, options).is_err(),
        "an impossible planar capacity must be rejected for solid domains too"
    );
}

#[test]
fn scripted_shrink_explores_dimensions_after_a_passing_candidate() {
    // A property that fails only while the first dimension is large. Halving that
    // dimension makes the case pass, so Proptest complicates back and the search
    // must keep going with the other dimensions instead of re-proposing the same
    // rejected transition until its iteration budget runs out.
    let units = 40_000;
    let bounds = scenario_bounds();
    let strategy = scripted_cases(
        ScriptedDomain {
            geometry: geometry_domain(TopologyClass::Convex, units),
            solid_witness: Some(SolidWitness::ClosedSolid),
            bounds,
        },
        scenario_options(&bounds, units),
    )
    .expect("a closed-solid strategy");

    let mut runner = TestRunner::new(Config {
        cases: 32,
        rng_algorithm: RngAlgorithm::ChaCha,
        rng_seed: RngSeed::Fixed(0x5c4d_0012),
        failure_persistence: None,
        max_shrink_iters: 256,
        ..Config::default()
    });
    let first_failing = std::cell::RefCell::new(None);
    let failure = runner
        .run(&strategy, |case| {
            let ScadRecipe::Solid(Recipe3d::Box { size_units }) = &case.recipe else {
                return Err(proptest::test_runner::TestCaseError::reject(
                    "only the box family is under test",
                ));
            };
            if size_units[0] > 12_000 {
                first_failing.borrow_mut().get_or_insert(*size_units);
                Err(proptest::test_runner::TestCaseError::fail(
                    "first dimension above the threshold",
                ))
            } else {
                Ok(())
            }
        })
        .expect_err("the threshold probe must fail");
    let initial = first_failing
        .into_inner()
        .expect("the probe recorded the first failing case");
    let proptest::test_runner::TestError::Fail(_, value) = failure else {
        panic!("expected a failing value, got {failure:?}");
    };
    let ScadRecipe::Solid(Recipe3d::Box { size_units }) = value.recipe else {
        panic!("the retained value must stay in the box family");
    };
    assert_eq!(
        size_units[0], initial[0],
        "the failing dimension must be preserved: shrinking it would pass the property"
    );
    assert!(
        size_units[1] < initial[1] || size_units[2] < initial[2],
        "the search must reduce a non-failing dimension after the first candidate passed: {size_units:?} vs {initial:?}"
    );
}

#[test]
fn scripted_generator_reaches_every_advertised_family() {
    let bounds = scenario_bounds();
    let units = 40_000;
    let mut runner = TestRunner::new(Config {
        cases: 48,
        rng_algorithm: RngAlgorithm::ChaCha,
        rng_seed: RngSeed::Fixed(0x5c4d_0011),
        failure_persistence: None,
        ..Config::default()
    });

    fn collect_2d(recipe: &Recipe2d, kinds: &mut std::collections::BTreeSet<&'static str>) {
        match recipe {
            Recipe2d::Polygon { .. } => {
                kinds.insert("polygon-leaf");
            }
            Recipe2d::Rectangle { .. } => {
                kinds.insert("rectangle");
            }
            Recipe2d::Disk { .. } => {
                kinds.insert("disk");
            }
            Recipe2d::Translate { child, .. } => collect_2d(child, kinds),
            Recipe2d::QuarterTurn { child, .. } => collect_2d(child, kinds),
            Recipe2d::Union(children) => {
                kinds.insert("union");
                for child in children {
                    collect_2d(child, kinds);
                }
            }
            Recipe2d::Difference { base, cuts } => {
                kinds.insert("difference");
                collect_2d(base, kinds);
                for cut in cuts {
                    collect_2d(cut, kinds);
                }
            }
            Recipe2d::Intersection(children) => {
                kinds.insert("intersection");
                for child in children {
                    collect_2d(child, kinds);
                }
            }
        }
    }
    fn collect_3d(recipe: &Recipe3d, kinds: &mut std::collections::BTreeSet<&'static str>) {
        match recipe {
            Recipe3d::Box { .. } => {
                kinds.insert("box");
            }
            Recipe3d::Cylinder { .. } => {
                kinds.insert("cylinder");
            }
            Recipe3d::LinearExtrude { profile, .. } => {
                kinds.insert("extrude");
                collect_2d(profile, kinds);
            }
            Recipe3d::Translate { child, .. } => collect_3d(child, kinds),
            Recipe3d::QuarterTurnZ { child, .. } => {
                kinds.insert("turn");
                collect_3d(child, kinds);
            }
            Recipe3d::Union(children) => {
                kinds.insert("union");
                for child in children {
                    collect_3d(child, kinds);
                }
            }
            Recipe3d::Difference { base, cuts } => {
                kinds.insert("difference");
                collect_3d(base, kinds);
                for cut in cuts {
                    collect_3d(cut, kinds);
                }
            }
            Recipe3d::Intersection(children) => {
                kinds.insert("intersection");
                for child in children {
                    collect_3d(child, kinds);
                }
            }
        }
    }
    fn recipe_kinds(recipe: &ScadRecipe) -> std::collections::BTreeSet<&'static str> {
        let mut kinds = std::collections::BTreeSet::new();
        match recipe {
            ScadRecipe::Contours(recipe) => collect_2d(recipe, &mut kinds),
            ScadRecipe::Solid(recipe) => collect_3d(recipe, &mut kinds),
        }
        kinds
    }

    // Planar families: the CSG templates must actually be generated, not only
    // reachable through a retry after a rejection.
    let holed = scripted_cases(
        ScriptedDomain {
            geometry: geometry_domain(TopologyClass::Holed, units),
            solid_witness: None,
            bounds,
        },
        scenario_options(&bounds, units),
    )
    .expect("holed strategy");
    let mut kinds = std::collections::BTreeSet::new();
    for _ in 0..24 {
        let tree = holed.new_tree(&mut runner).expect("holed case");
        kinds.extend(recipe_kinds(&tree.current().recipe));
    }
    assert!(
        kinds.contains("difference"),
        "the holed strategy must generate its CSG difference, saw {kinds:?}"
    );
    assert!(
        kinds.contains("polygon-leaf"),
        "the holed strategy must also generate explicit leaves, saw {kinds:?}"
    );

    let disconnected = scripted_cases(
        ScriptedDomain {
            geometry: geometry_domain(TopologyClass::Disconnected, units),
            solid_witness: None,
            bounds,
        },
        scenario_options(&bounds, units),
    )
    .expect("disconnected strategy");
    let mut kinds = std::collections::BTreeSet::new();
    for _ in 0..24 {
        let tree = disconnected
            .new_tree(&mut runner)
            .expect("disconnected case");
        kinds.extend(recipe_kinds(&tree.current().recipe));
    }
    assert!(
        kinds.contains("union"),
        "the disconnected strategy must generate its CSG union, saw {kinds:?}"
    );

    // Solid families: primitives, extrusions, turns and intersections are all
    // part of the generator, not only of the fixed emission tests.
    let closed = scripted_cases(
        ScriptedDomain {
            geometry: geometry_domain(TopologyClass::Convex, units),
            solid_witness: Some(SolidWitness::ClosedSolid),
            bounds,
        },
        scenario_options(&bounds, units),
    )
    .expect("closed solid strategy");
    let mut kinds = std::collections::BTreeSet::new();
    for _ in 0..32 {
        let tree = closed.new_tree(&mut runner).expect("closed solid case");
        kinds.extend(recipe_kinds(&tree.current().recipe));
    }
    for required in ["box", "cylinder", "extrude"] {
        assert!(
            kinds.contains(required),
            "the closed-solid strategy must generate {required}, saw {kinds:?}"
        );
    }
    assert!(
        kinds.contains("intersection"),
        "the closed-solid strategy must generate an intersection, saw {kinds:?}"
    );

    let cavity = scripted_cases(
        ScriptedDomain {
            geometry: geometry_domain(TopologyClass::Convex, units),
            solid_witness: Some(SolidWitness::Cavity),
            bounds,
        },
        scenario_options(&bounds, units),
    )
    .expect("cavity strategy");
    let mut kinds = std::collections::BTreeSet::new();
    for _ in 0..24 {
        let tree = cavity.new_tree(&mut runner).expect("cavity case");
        kinds.extend(recipe_kinds(&tree.current().recipe));
    }
    assert!(
        kinds.contains("difference") && kinds.contains("turn") && kinds.contains("extrude"),
        "the cavity strategy must reach its difference, extrusion and turn variants, saw {kinds:?}"
    );

    let separated = scripted_cases(
        ScriptedDomain {
            geometry: geometry_domain(TopologyClass::Convex, units),
            solid_witness: Some(SolidWitness::SeparatedBodies),
            bounds,
        },
        scenario_options(&bounds, units),
    )
    .expect("separated strategy");
    let mut kinds = std::collections::BTreeSet::new();
    for _ in 0..24 {
        let tree = separated.new_tree(&mut runner).expect("separated case");
        kinds.extend(recipe_kinds(&tree.current().recipe));
    }
    assert!(
        kinds.contains("union") && kinds.contains("turn"),
        "the separated strategy must reach its union and turn variants, saw {kinds:?}"
    );

}
