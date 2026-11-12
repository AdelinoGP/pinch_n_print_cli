//! Registry-backed scripted SCAD properties (`D-SCAD-2D`, `D-SCAD-3D`).
//!
//! The `proptest_*` entry points below are executed only through
//! `cargo xtask test --properties --with-scad`, which supplies the measured
//! profile context. The rest of the file carries direct contract tests so the
//! target cannot build as a green zero-test binary when the feature is on.
//!
//! These properties validate the *helper* contracts and their witness
//! preservation. They deliberately do not credit host or module SUT coverage.

use std::iter::once;

use proptest::prelude::*;
use proptest::test_runner::TestCaseResult;

use slicer_sdk::test_support::property_testing::domains::{
    BoundaryRule, GeometryBounds, GeometryCase, GeometryDomain, TopologyClass,
};
use slicer_sdk::test_support::property_testing::oracles::{validate_indexed_mesh, validate_ring};
use slicer_sdk::test_support::property_testing::regression::{
    format_regression_fixture, ConcreteInput,
};
use slicer_sdk::test_support::property_testing::scripted::{
    emit_scad, evaluate_recipe, scripted_cases, Recipe2d, RecipeBounds, ScadRecipe, ScriptedCase,
    ScriptedDomain, SolidWitness,
};
use slicer_sdk::test_support::property_testing::PropertyRun;

#[path = "support/scripted.rs"]
mod scripted_support;

type BoxError = Box<dyn std::error::Error>;

fn recipe_bounds(run: &PropertyRun) -> Result<RecipeBounds, BoxError> {
    // exhaustive: pins the registry bound mapping deliberately
    Ok(RecipeBounds {
        max_nodes: usize::try_from(run.bound("max_recipe_nodes")?)?,
        max_depth: usize::try_from(run.bound("max_recipe_depth")?)?,
        max_facets: u16::try_from(run.bound("max_facets")?)?,
        max_source_bytes: usize::try_from(run.bound("max_source_bytes")?)?,
        max_evaluations: u32::try_from(run.bound("max_evaluations")?)?,
    })
}

fn geometry_bounds(run: &PropertyRun) -> Result<GeometryBounds, BoxError> {
    // exhaustive: pins the registry bound mapping deliberately
    Ok(GeometryBounds {
        max_abs_units: i64::try_from(run.bound("max_abs_units")?)?,
        max_vertices: usize::try_from(run.bound("max_vertices")?)?,
        max_holes: usize::try_from(run.bound("max_holes")?)?,
        max_components: usize::try_from(run.bound("max_components")?)?,
        max_shrink_candidates: u32::try_from(run.bound("max_shrink_candidates")?)?,
    })
}

fn options(
    run: &PropertyRun,
    bounds: &RecipeBounds,
) -> Result<slicer_scad::ScadEvalOptions, BoxError> {
    let units = i64::try_from(run.bound("max_abs_units")?)?;
    Ok(scripted_support::options_with_stack(
        bounds,
        units,
        usize::try_from(run.bound("max_scad_vertices")?)?,
        usize::try_from(run.bound("max_scad_triangles")?)?,
        run.bound("evaluation_steps")?,
        usize::try_from(run.bound("worker_stack_bytes")?)?,
        run.bound("max_evaluations")?,
    ))
}

fn domain(
    run: &PropertyRun,
    class: TopologyClass,
    witness: Option<SolidWitness>,
    recipe_bounds: RecipeBounds,
) -> Result<ScriptedDomain, BoxError> {
    Ok(ScriptedDomain {
        // exhaustive: pins the registry geometry capacities deliberately
        geometry: GeometryDomain {
            class,
            min_holes: 0,
            min_components: usize::from(class != TopologyClass::Empty),
            boundary: BoundaryRule::Strict,
            bounds: geometry_bounds(run)?,
        },
        solid_witness: witness,
        bounds: recipe_bounds,
    })
}

/// Area of the converted contour forest in square millimetres.
fn converted_area_mm2(case: &ScriptedCase) -> Option<f64> {
    let slicer_scad::ScadGeometry::Contours(polygons) = &case.output.geometry else {
        return None;
    };
    let mut total: i128 = 0;
    for polygon in polygons {
        for ring in once(&polygon.contour).chain(polygon.holes.iter()) {
            total += scripted_support::signed_area2(ring);
        }
    }
    Some(total as f64 / (2.0 * 10_000.0 * 10_000.0))
}

/// Area of an explicit intended polygon leaf in square millimetres.
fn intended_area_mm2(recipe: &ScadRecipe) -> Option<f64> {
    let ScadRecipe::Contours(Recipe2d::Polygon { polygons }) = recipe else {
        return None;
    };
    Some(scripted_support::forest_area_mm2(polygons))
}

fn is_explicit_empty(recipe: &ScadRecipe) -> bool {
    matches!(
        recipe,
        ScadRecipe::Contours(Recipe2d::Union(children)) if children.is_empty()
    )
}

fn concrete_input(case: &ScriptedCase) -> Option<ConcreteInput> {
    match &case.output.geometry {
        slicer_scad::ScadGeometry::Contours(polygons) if !polygons.is_empty() => {
            Some(ConcreteInput::Geometry(GeometryCase {
                polygons: polygons.clone(),
            }))
        }
        slicer_scad::ScadGeometry::Mesh(mesh) => Some(ConcreteInput::Mesh(mesh.clone())),
        _ => None,
    }
}

/// Independent per-case assertions shared by both scripted properties.
fn check_case(case: &ScriptedCase) -> TestCaseResult {
    let source = emit_scad(&case.recipe, &case.recipe_bounds).expect("the retained recipe emits");
    prop_assert_eq!(
        &source,
        &case.source,
        "a certified case must reproduce its retained source exactly"
    );
    if let Some(input) = concrete_input(case) {
        // The concrete regression formatter must be able to carry this input
        // without engine or Proptest context.
        let block = format_regression_fixture(
            "scripted retained case",
            "certified scripted case",
            &input,
        )
        .expect("a certified case formats as a concrete fixture");
        prop_assert!(!block.is_empty());
    }
    match &case.output.geometry {
        slicer_scad::ScadGeometry::Contours(polygons) => {
            for polygon in polygons {
                for ring in once(&polygon.contour).chain(polygon.holes.iter()) {
                    validate_ring(ring).expect("a certified contour ring is simple");
                }
            }
            let converted = converted_area_mm2(case).expect("contour area");
            prop_assert!(
                converted > 0.0,
                "a non-empty contour case must have positive area"
            );
            if let Some(intended) = intended_area_mm2(&case.recipe) {
                prop_assert!(
                    (converted - intended).abs() <= 1e-9 * intended.abs().max(1.0),
                    "an explicit polygon leaf must keep its exact intended area: {} vs {}",
                    converted,
                    intended
                );
            }
        }
        slicer_scad::ScadGeometry::Mesh(mesh) => {
            validate_indexed_mesh(mesh, true).expect("a certified mesh is closed");
            prop_assert!(
                mesh.indices.len() >= 3,
                "a certified mesh keeps at least one triangle"
            );
        }
        slicer_scad::ScadGeometry::Empty => {
            prop_assert!(
                is_explicit_empty(&case.recipe),
                "only an explicitly empty recipe may evaluate to no geometry"
            );
        }
    }
    Ok(())
}

fn contour_strategy(run: &PropertyRun) -> Result<impl Strategy<Value = ScriptedCase>, BoxError> {
    let bounds = recipe_bounds(run)?;
    let options = options(run, &bounds)?;
    Ok(prop_oneof![
        scripted_cases(domain(run, TopologyClass::Convex, None, bounds)?, options.clone())?,
        scripted_cases(domain(run, TopologyClass::Concave, None, bounds)?, options.clone())?,
        scripted_cases(
            domain(run, TopologyClass::NonStarShaped, None, bounds)?,
            options.clone()
        )?,
        scripted_cases(domain(run, TopologyClass::Holed, None, bounds)?, options.clone())?,
        scripted_cases(
            domain(run, TopologyClass::Disconnected, None, bounds)?,
            options.clone()
        )?,
        scripted_cases(domain(run, TopologyClass::Thin, None, bounds)?, options.clone())?,
        scripted_cases(
            domain(run, TopologyClass::NearDegenerate, None, bounds)?,
            options.clone()
        )?,
        scripted_cases(domain(run, TopologyClass::Empty, None, bounds)?, options)?,
    ])
}

fn solid_strategy(run: &PropertyRun) -> Result<impl Strategy<Value = ScriptedCase>, BoxError> {
    let bounds = recipe_bounds(run)?;
    let options = options(run, &bounds)?;
    Ok(prop_oneof![
        scripted_cases(
            domain(run, TopologyClass::Convex, Some(SolidWitness::ClosedSolid), bounds)?,
            options.clone()
        )?,
        scripted_cases(
            domain(run, TopologyClass::Convex, Some(SolidWitness::Cavity), bounds)?,
            options.clone()
        )?,
        scripted_cases(
            domain(run, TopologyClass::Convex, Some(SolidWitness::Bridge), bounds)?,
            options.clone()
        )?,
        scripted_cases(
            domain(run, TopologyClass::Convex, Some(SolidWitness::Overhang), bounds)?,
            options.clone()
        )?,
        scripted_cases(
            domain(
                run,
                TopologyClass::Convex,
                Some(SolidWitness::SeparatedBodies),
                bounds
            )?,
            options
        )?,
    ])
}

#[test]
fn proptest_scripted_contour_domains() -> Result<(), BoxError> {
    let run = PropertyRun::new("D-SCAD-2D", "proptest_scripted_contour_domains")?;
    let strategy = contour_strategy(&run)?;
    run.check(&strategy, |case, _guard| check_case(&case))?;
    Ok(())
}

#[test]
fn proptest_scripted_solid_domains() -> Result<(), BoxError> {
    let run = PropertyRun::new("D-SCAD-3D", "proptest_scripted_solid_domains")?;
    let strategy = solid_strategy(&run)?;
    run.check(&strategy, |case, _guard| check_case(&case))?;
    Ok(())
}

/// Controlled process-tree fixture for the executor's whole-process supervision.
///
/// It does nothing in an ordinary run. With `PNP_SCRIPTED_SUPERVISION_FIXTURE`
/// set it spawns a long-sleeping descendant, prints its PID, and blocks *before*
/// any property callback, so the executor's timeout must cover generation.
// The descendant is deliberately not waited on: the executor's deadline kill
// must terminate the whole tree, which is exactly what this fixture proves.
#[allow(clippy::zombie_processes)]
#[test]
fn scripted_supervision_fixture() {
    // test-quality: this is the executor's supervised process-tree fixture; the
    // gated block is its whole purpose and the sleep is what supervision kills.
    if std::env::var_os("PNP_SCRIPTED_SUPERVISION_FIXTURE").is_some() {
        // test-quality: the descendant branch exists only to be killed
        if std::env::var_os("PNP_SCRIPTED_SLEEP_SECONDS").is_some() {
            // test-quality: blocks so the executor's deadline has something to end
            std::thread::sleep(std::time::Duration::from_secs(120));
        } else {
            let executable = std::env::current_exe().expect("current test executable");
            let child = std::process::Command::new(executable)
                .env("PNP_SCRIPTED_SUPERVISION_FIXTURE", "descendant")
                .env("PNP_SCRIPTED_SLEEP_SECONDS", "120")
                .arg("--exact")
                .arg("scripted_supervision_fixture")
                .arg("--nocapture")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .expect("spawn a sleeping descendant");
            println!("PNP_SCRIPTED_DESCENDANT {}", child.id());
            // test-quality: blocks so supervision must terminate the whole tree
            std::thread::sleep(std::time::Duration::from_secs(120));
        }
    }
}

#[test]
fn scripted_target_reaches_the_public_adapter() {
    let bounds = RecipeBounds::default();
    let recipe = ScadRecipe::Contours(Recipe2d::Rectangle {
        size_units: [10_000, 10_000],
    });
    let source = emit_scad(&recipe, &bounds).expect("emit a 1 mm square");
    assert!(source.text.contains("square([1.0000, 1.0000]);"));
    let options = scripted_support::options(&bounds, 20_000, 200_000, 400_000, 10_000_000, 7);
    let output = evaluate_recipe(&recipe, &options, &bounds).expect("evaluate a 1 mm square");
    assert!(
        matches!(output.geometry, slicer_scad::ScadGeometry::Contours(_)),
        "a square must evaluate to contours"
    );
}
