//! Acceptance contracts for the SCAD adapter (AC-2, AC-3, AC-4, AC-N1, AC-N2,
//! AC-N4).
//!
//! Every fixture here is a real source evaluated by the pinned engine through the
//! public API. Expected topology and numerics are enumerated independently: areas
//! and volumes come from the declared dimensions, never from the adapter's own
//! grouping routine.
//!
//! Raw-ring counterexamples (shuffled/reversed/quantized damage) cannot be driven
//! through this API, because the engine owns ring production. Those live in the
//! private conversion tests under the `U` lane.

use slicer_ir::slice_ir::{IndexedTriangleSet, Point2, Polygon};
use slicer_scad::{
    evaluate, ScadDiagnosticClass, ScadErrorKind, ScadEvalOptions, ScadEvalOutput, ScadGeometry,
    ScadLimits, ScadParameter, ScadSource,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn params(pairs: &[(&str, &str)]) -> Vec<ScadParameter> {
    pairs
        .iter()
        .map(|(name, expression)| ScadParameter {
            name: (*name).to_string(),
            expression: (*expression).to_string(),
        })
        .collect()
}

fn options_with(pairs: &[(&str, &str)]) -> ScadEvalOptions {
    ScadEvalOptions {
        parameters: params(pairs),
        ..ScadEvalOptions::default()
    }
}

fn eval(label: &str, text: &str) -> Result<ScadEvalOutput, slicer_scad::ScadError> {
    evaluate(
        &ScadSource {
            label: label.to_string(),
            text: text.to_string(),
        },
        &ScadEvalOptions::default(),
    )
}

fn eval_with(
    label: &str,
    text: &str,
    options: &ScadEvalOptions,
) -> Result<ScadEvalOutput, slicer_scad::ScadError> {
    evaluate(
        &ScadSource {
            label: label.to_string(),
            text: text.to_string(),
        },
        options,
    )
}

/// Twice the signed area in scaled-unit² (positive = CCW).
fn shoelace12(points: &[Point2]) -> i128 {
    let mut sum = 0i128;
    for index in 0..points.len() {
        let a = &points[index];
        let b = &points[(index + 1) % points.len()];
        sum += a.x as i128 * b.y as i128 - b.x as i128 * a.y as i128;
    }
    sum
}

/// Area in square millimetres.
fn area_mm2(points: &[Point2]) -> f64 {
    shoelace12(points) as f64 / 2.0 / (10_000.0 * 10_000.0)
}

fn polygon_area_mm2(polygon: &Polygon) -> f64 {
    area_mm2(&polygon.points)
}

/// Net signed area of one region: outer contour minus its holes.
fn region_area_mm2(region: &slicer_ir::slice_ir::ExPolygon) -> f64 {
    polygon_area_mm2(&region.contour) + region.holes.iter().map(polygon_area_mm2).sum::<f64>()
}

/// Signed mesh volume in mm³ (six times the closed-surface integral sums to the
/// exact volume in these fixtures).
fn mesh_volume_mm3(mesh: &IndexedTriangleSet) -> f64 {
    let mut sum = 0.0f64;
    for triangle in mesh.indices.chunks(3) {
        let a = &mesh.vertices[triangle[0] as usize];
        let b = &mesh.vertices[triangle[1] as usize];
        let c = &mesh.vertices[triangle[2] as usize];
        let (ax, ay, az) = (a.x as f64, a.y as f64, a.z as f64);
        let (bx, by, bz) = (b.x as f64, b.y as f64, b.z as f64);
        let (cx, cy, cz) = (c.x as f64, c.y as f64, c.z as f64);
        sum += ax * (by * cz - bz * cy) - ay * (bx * cz - bz * cx) + az * (bx * cy - by * cx);
    }
    sum / 6.0
}

fn z_bounds(mesh: &IndexedTriangleSet) -> (f32, f32) {
    let mut lo = f32::INFINITY;
    let mut hi = f32::NEG_INFINITY;
    for vertex in &mesh.vertices {
        lo = lo.min(vertex.z);
        hi = hi.max(vertex.z);
    }
    (lo, hi)
}

fn assert_close(actual: f64, expected: f64, what: &str) {
    let tolerance = 1e-6 * expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= tolerance,
        "{what}: expected {expected}, got {actual}"
    );
}

/// Assert a successful result, returning it.
fn expect_ok(result: Result<ScadEvalOutput, slicer_scad::ScadError>, what: &str) -> ScadEvalOutput {
    match result {
        Ok(output) => output,
        Err(error) => panic!("{what} must succeed, got {:?}: {error}", error.kind),
    }
}

/// Assert a failure kind, returning the error.
fn expect_err(
    result: Result<ScadEvalOutput, slicer_scad::ScadError>,
    kind: ScadErrorKind,
    what: &str,
) -> slicer_scad::ScadError {
    match result {
        Ok(output) => panic!(
            "{what} must fail with {kind:?}, but it succeeded with {:?}",
            output.geometry
        ),
        Err(error) => {
            assert_eq!(error.kind, kind, "{what}: {error}");
            error
        }
    }
}

fn contours<'a>(output: &'a ScadEvalOutput, what: &str) -> &'a Vec<slicer_ir::slice_ir::ExPolygon> {
    match &output.geometry {
        ScadGeometry::Contours(regions) => regions,
        other => panic!("{what} must be contours, got {other:?}"),
    }
}

fn mesh<'a>(output: &'a ScadEvalOutput, what: &str) -> &'a IndexedTriangleSet {
    match &output.geometry {
        ScadGeometry::Mesh(mesh) => mesh,
        other => panic!("{what} must be a mesh, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// AC-2
// ---------------------------------------------------------------------------

#[test]
fn scad_parameters_budget_and_exact_mode() {
    // --- arithmetic and nested vectors -----------------------------------
    let arithmetic = expect_ok(
        eval_with(
            "arith",
            "x = 1 + 2 * 3; cube(x);",
            &options_with(&[("x", "4")]),
        ),
        "an arithmetic override",
    );
    assert_close(
        mesh_volume_mm3(mesh(&arithmetic, "arith")),
        64.0,
        "cube(4) volume",
    );

    // `m[0][0]=10, m[1][1]=40, m[0][1]=20` -> 10 x 40 x 20 mm.
    let nested = expect_ok(
        eval_with(
            "nested-vector",
            "m = [[1,2],[3,4]]; cube([m[0][0], m[1][1], m[0][1]]);",
            &options_with(&[("m", "[[10,20],[30,40]]")]),
        ),
        "a nested-vector override",
    );
    assert_close(
        mesh_volume_mm3(mesh(&nested, "nested-vector")),
        8_000.0,
        "10 x 40 x 20 volume",
    );

    // --- duplicate last-wins, and the raw list is preserved --------------
    let duplicate = expect_ok(
        eval_with(
            "dup",
            "x = 1; cube(x);",
            &options_with(&[("x", "2"), ("x", "5")]),
        ),
        "a duplicate override",
    );
    assert_close(
        mesh_volume_mm3(mesh(&duplicate, "dup")),
        125.0,
        "the last occurrence must win",
    );
    assert_eq!(
        duplicate.provenance.parameters.len(),
        2,
        "provenance must preserve duplicates, not collapse them"
    );
    assert_eq!(duplicate.provenance.parameters[0].expression, "2");
    assert_eq!(duplicate.provenance.parameters[1].expression, "5");

    // --- case sensitivity ------------------------------------------------
    let wrong_case = expect_ok(
        eval_with("case", "x = 1; cube(x);", &options_with(&[("X", "9")])),
        "an override whose case does not match",
    );
    assert_close(
        mesh_volume_mm3(mesh(&wrong_case, "case")),
        1.0,
        "`X` must not override `x`",
    );
    let right_case = expect_ok(
        eval_with("case2", "X = 9; cube(X);", &options_with(&[("x", "2")])),
        "an override whose case does not match the source",
    );
    assert_close(
        mesh_volume_mm3(mesh(&right_case, "case2")),
        729.0,
        "`x` must not override `X`",
    );

    // --- `$` specials ----------------------------------------------------
    let four = expect_ok(
        eval_with(
            "fn4",
            "cylinder(h = 1, r = 1);",
            &options_with(&[("$fn", "4")]),
        ),
        "a `$fn` override",
    );
    assert_eq!(
        mesh(&four, "fn4").vertices.len(),
        8,
        "a 4-sided prism has 8 vertices"
    );
    let eight = expect_ok(
        eval_with(
            "fn8",
            "cylinder(h = 1, r = 1);",
            &options_with(&[("$fn", "8")]),
        ),
        "a larger `$fn` override",
    );
    assert!(
        mesh(&eight, "fn8").vertices.len() > mesh(&four, "fn4").vertices.len(),
        "the `$` override must actually reach the engine"
    );

    // --- source diagnostics ----------------------------------------------
    let parse = expect_err(
        eval("parse", "cube(;"),
        ScadErrorKind::Parse,
        "a syntax error",
    );
    assert_eq!(parse.source_label, "parse");
    let span_diagnostic = parse
        .diagnostics
        .iter()
        .find(|d| d.stage == slicer_scad::ScadDiagnosticStage::Parse)
        .expect("a parse failure must retain a stage-tagged diagnostic");
    assert!(
        span_diagnostic.span.is_some(),
        "a syntax error must retain its byte span"
    );

    // --- `$preview` selects a branch; it never selects the preview renderer
    let preview_true = expect_ok(
        eval_with(
            "preview-true",
            "if ($preview) cube(2); else cube(1);",
            &options_with(&[("$preview", "true")]),
        ),
        "an explicit $preview=true",
    );
    assert_close(
        mesh_volume_mm3(mesh(&preview_true, "preview-true")),
        8.0,
        "the $preview branch must be taken",
    );
    let preview_default = expect_ok(
        eval_with(
            "preview-default",
            "if ($preview) cube(2); else cube(1);",
            &ScadEvalOptions::default(),
        ),
        "exact mode seeds $preview=false",
    );
    assert_close(
        mesh_volume_mm3(mesh(&preview_default, "preview-default")),
        1.0,
        "exact evaluation must seed $preview=false",
    );

    // Exact union geometry has its analytic volume even when `$preview=true` is
    // supplied: preview concatenation would keep both interiors and report 16.
    let exact_union = expect_ok(
        eval_with(
            "exact-union-preview",
            "union(){ cube(2); translate([1,0,0]) cube(2); }",
            &options_with(&[("$preview", "true")]),
        ),
        "an exact union under $preview=true",
    );
    assert_close(
        mesh_volume_mm3(mesh(&exact_union, "exact-union-preview")),
        12.0,
        "2 + 2 cubes overlapping by 1 mm in x occupy 12 mm3, not 16",
    );

    // --- one shared finite allowance -------------------------------------
    let budget = |steps: u64| ScadEvalOptions {
        limits: ScadLimits {
            evaluation_steps: steps,
            ..ScadLimits::default()
        },
        ..ScadEvalOptions::default()
    };

    // The program alone fits the allowance ...
    expect_ok(
        eval_with("budget-baseline", "cube(1);", &budget(100_000)),
        "a trivial program must fit a 100k allowance",
    );

    // ... but a single expensive override does not, and the failure names it.
    let expensive_parameter = ScadEvalOptions {
        parameters: params(&[("x", "[for (i = [0:300000]) i]")]),
        ..budget(100_000)
    };
    let parameter_error = expect_err(
        eval_with("budget-param", "cube(1);", &expensive_parameter),
        ScadErrorKind::EvaluationBudgetExceeded,
        "a runaway parameter expression",
    );
    assert_eq!(
        parameter_error.parameter.as_deref(),
        Some("x"),
        "the failed parameter must be identified"
    );

    // The allowance is not reset per override: two overrides that each fit alone
    // exhaust it together, and the second one is the one that fails.
    let two_expensive = ScadEvalOptions {
        parameters: params(&[
            ("a", "[for (i = [0:70000]) i]"),
            ("b", "[for (i = [0:70000]) i]"),
        ]),
        ..budget(100_000)
    };
    let second_error = expect_err(
        eval_with("budget-shared", "cube(1);", &two_expensive),
        ScadErrorKind::EvaluationBudgetExceeded,
        "two overrides under one allowance",
    );
    assert_eq!(
        second_error.parameter.as_deref(),
        Some("b"),
        "the shared allowance must be consumed in input order"
    );

    // A nested-loop program exhausts the same allowance in the program phase.
    let nested_loop = expect_err(
        eval_with(
            "budget-program",
            "for (a = [0:100000]) for (b = [0:100000]) translate([a, b, 0]) cube(1);",
            &budget(100_000),
        ),
        ScadErrorKind::EvaluationBudgetExceeded,
        "a nested-loop program",
    );
    assert_eq!(
        nested_loop.parameter, None,
        "a program budget failure is not a parameter failure"
    );
}

// ---------------------------------------------------------------------------
// AC-3
// ---------------------------------------------------------------------------

#[test]
fn scad_parameter_evaluation_obeys_resource_and_diagnostic_boundary() {
    for (expression, kind) in [
        ("not_defined", ScadErrorKind::InvalidParameter),
        ("undef", ScadErrorKind::InvalidParameter),
        ("let(a=unknown_fn()) 1", ScadErrorKind::UnknownDiagnostic),
        (
            "dxf_dim(file=\"tempting.dxf\")",
            ScadErrorKind::ExternalResourceDenied,
        ),
    ] {
        let error = expect_err(
            eval_with(
                "parameter-boundary",
                "cube(1);",
                &options_with(&[("x", expression)]),
            ),
            kind,
            expression,
        );
        assert_eq!(error.parameter.as_deref(), Some("x"), "{expression}");
        assert!(!error.diagnostics.is_empty(), "{expression}");
        assert!(error
            .diagnostics
            .iter()
            .all(|d| d.stage == slicer_scad::ScadDiagnosticStage::Parameter));
        match kind {
            ScadErrorKind::InvalidParameter => {
                assert!(error
                    .message
                    .contains("an override expression evaluated to `undef`"));
            }
            ScadErrorKind::ExternalResourceDenied => {
                assert!(error.message.contains("tempting.dxf"));
                assert!(error
                    .diagnostics
                    .iter()
                    .any(|d| d.code == "external_resource_denied"));
            }
            ScadErrorKind::UnknownDiagnostic => {
                assert!(error
                    .diagnostics
                    .iter()
                    .any(|d| d.class == ScadDiagnosticClass::Unknown
                        && d.message.contains("unknown_fn")));
            }
            _ => unreachable!("the fixture enumerates only these failure kinds"),
        }
    }
    let echoed = expect_ok(
        eval_with(
            "parameter-echo",
            "cube(x);",
            &options_with(&[("x", "echo(7) 1")]),
        ),
        "an informational override echo",
    );
    assert_close(
        mesh_volume_mm3(mesh(&echoed, "parameter-echo")),
        1.0,
        "override value",
    );
    let echo = echoed
        .diagnostics
        .iter()
        .find(|d| d.code == "scad_echo")
        .expect("override echo must not disappear");
    assert_eq!(echo.class, ScadDiagnosticClass::Information);
    assert_eq!(echo.stage, slicer_scad::ScadDiagnosticStage::Parameter);
    assert!(echo.message.contains('7'));
}

#[test]
fn scad_complexity_limits_cover_overrides_projection_and_derived_facets() {
    let aggregate = ScadEvalOptions {
        parameters: params(&[("a", "1"), ("b", "2"), ("c", "3")]),
        limits: ScadLimits {
            max_ast_nodes: 2,
            ..ScadLimits::default()
        },
        ..ScadEvalOptions::default()
    };
    let error = expect_err(
        eval_with("aggregate-ast", "cube(1);", &aggregate),
        ScadErrorKind::ComplexityLimit,
        "program and overrides share an AST allowance",
    );
    assert_eq!(error.parameter.as_deref(), Some("a"));
    assert!(error.message.contains("max_ast_nodes"));

    for source in [
        "union(){ cube(1); translate([3,0,0]) cube(1); }",
        "projection() union(){ cube(1); translate([3,0,0]) cube(1); }",
    ] {
        let error = expect_err(
            eval_with(
                "projection-nodes",
                source,
                &ScadEvalOptions {
                    limits: ScadLimits {
                        max_geometry_nodes: 2,
                        ..ScadLimits::default()
                    },
                    ..ScadEvalOptions::default()
                },
            ),
            ScadErrorKind::ComplexityLimit,
            source,
        );
        assert!(error.message.contains("max_geometry_nodes"));
    }
    for source in [
        "circle(r=100, $fa=0.5, $fs=0.1);",
        "sphere(r=100, $fa=0.5, $fs=0.1);",
        "cylinder(h=1, r1=1, r2=100, $fa=0.5, $fs=0.1);",
        "projection() sphere(r=1, $fn=500);",
    ] {
        let error = expect_err(
            eval("derived-facets", source),
            ScadErrorKind::ComplexityLimit,
            source,
        );
        assert!(error.message.contains("max_facets"));
    }
}

#[test]
fn scad_even_odd_contours_and_explicit_extrusion() {
    // --- a single hole with an independently enumerated area --------------
    let annulus = expect_ok(
        eval(
            "annulus",
            "difference(){ square(100); translate([10,10]) square(80); }",
        ),
        "a holed profile",
    );
    let holed = contours(&annulus, "annulus");
    assert_eq!(holed.len(), 1, "one outer component");
    assert_eq!(holed[0].holes.len(), 1, "one hole");
    assert_close(polygon_area_mm2(&holed[0].contour), 10_000.0, "outer area");
    assert_close(polygon_area_mm2(&holed[0].holes[0]), -6_400.0, "hole area");
    assert_close(region_area_mm2(&holed[0]), 3_600.0, "net region area");
    assert!(
        shoelace12(&holed[0].contour.points) > 0,
        "an outer contour must be CCW"
    );
    assert!(
        shoelace12(&holed[0].holes[0].points) < 0,
        "a hole must be CW"
    );

    // --- two holes in one outer ------------------------------------------
    let two_holes = expect_ok(
        eval(
            "two-holes",
            "difference(){ square(100); translate([10,10]) square(20); \
             translate([60,60]) square(20); }",
        ),
        "a two-hole profile",
    );
    let two = contours(&two_holes, "two-holes");
    assert_eq!(two.len(), 1);
    assert_eq!(two[0].holes.len(), 2, "both holes must survive");
    for hole in &two[0].holes {
        assert_close(polygon_area_mm2(hole), -400.0, "hole area");
    }
    assert_close(region_area_mm2(&two[0]), 10_000.0 - 800.0, "net area");

    // --- nested island, and a hole inside the island (depth 3) -----------
    let island = expect_ok(
        eval(
            "island",
            "difference(){ square(100); translate([10,10]) \
             difference(){ square(80); translate([10,10]) square(60); } }",
        ),
        "a nested island",
    );
    let nested = contours(&island, "island");
    assert_eq!(nested.len(), 2, "the island is its own component");
    assert_eq!(nested[0].holes.len(), 1);
    assert_close(polygon_area_mm2(&nested[0].contour), 10_000.0, "outer");
    assert_close(polygon_area_mm2(&nested[0].holes[0]), -6_400.0, "hole");
    assert_close(polygon_area_mm2(&nested[1].contour), 3_600.0, "island");
    assert!(nested[1].holes.is_empty(), "this island has no hole");

    let deeper = expect_ok(
        eval(
            "deeper",
            "difference(){ square(100); translate([10,10]) difference(){ square(80); \
             translate([10,10]) difference(){ square(60); translate([10,10]) square(40); } } }",
        ),
        "a depth-3 nesting",
    );
    let three = contours(&deeper, "deeper");
    assert_eq!(three.len(), 2);
    assert_eq!(three[1].holes.len(), 1, "the island has its own hole");
    assert_close(polygon_area_mm2(&three[1].contour), 3_600.0, "island");
    assert_close(
        polygon_area_mm2(&three[1].holes[0]),
        -1_600.0,
        "island hole",
    );

    // --- disjoint outers, one of them holed ------------------------------
    let disjoint = expect_ok(
        eval(
            "disjoint",
            "union(){ difference(){ square(100); translate([10,10]) square(80); } \
             translate([200,0]) square(40); }",
        ),
        "disjoint outers",
    );
    let outers = contours(&disjoint, "disjoint");
    assert_eq!(outers.len(), 2);
    let holed_component = outers
        .iter()
        .find(|r| !r.holes.is_empty())
        .expect("one component keeps its hole");
    assert_eq!(holed_component.holes.len(), 1);
    let plain_component = outers
        .iter()
        .find(|r| r.holes.is_empty())
        .expect("one component has no holes");
    assert_close(polygon_area_mm2(&plain_component.contour), 1_600.0, "plain");

    // --- no implicit thickness -------------------------------------------
    assert!(
        matches!(annulus.geometry, ScadGeometry::Contours(_)),
        "a 2D result without a height must stay contours"
    );

    // --- explicit extrusion: closed mesh, Z from 0, exact volume ---------
    let extruded = expect_ok(
        eval_with(
            "extruded",
            "difference(){ square(100); translate([10,10]) square(80); }",
            &ScadEvalOptions {
                extrude_height_mm: Some(2.0),
                ..ScadEvalOptions::default()
            },
        ),
        "an explicit extrusion",
    );
    let extruded_mesh = mesh(&extruded, "extruded");
    assert_eq!(
        extruded_mesh.indices.len() % 3,
        0,
        "the index stream must be whole triangles"
    );
    let (lo, hi) = z_bounds(extruded_mesh);
    assert_close(f64::from(lo), 0.0, "extrusion starts at z=0");
    assert_close(f64::from(hi), 2.0, "extrusion ends at z=h");
    assert_close(
        mesh_volume_mm3(extruded_mesh),
        3_600.0 * 2.0,
        "extruded volume is profile area x height",
    );

    // --- pure 3D keeps separated bodies in one indexed mesh ---------------
    let separated = expect_ok(
        eval(
            "separated",
            "union(){ cube(1); translate([5,0,0]) cube(1); }",
        ),
        "two separated bodies",
    );
    let bodies = mesh(&separated, "separated");
    assert_eq!(
        bodies.vertices.len(),
        16,
        "two disjoint cubes keep both vertex sets"
    );
    assert_eq!(
        bodies.indices.len(),
        24 * 3,
        "two disjoint cubes keep both triangle sets"
    );
    assert_close(mesh_volume_mm3(bodies), 2.0, "two unit cubes");

    let drilled = expect_ok(
        eval(
            "drilled",
            "difference(){ cube([10,10,2]); translate([2,2,-1]) cube([2,2,4]); \
             translate([6,6,-1]) cube([2,2,4]); }",
        ),
        "a drilled 3D solid",
    );
    assert_close(
        mesh_volume_mm3(mesh(&drilled, "drilled")),
        184.0,
        "200 mm3 minus two 8 mm3 bores",
    );
}

// ---------------------------------------------------------------------------
// AC-4
// ---------------------------------------------------------------------------

#[test]
fn scad_informational_diagnostics_and_state_isolation() {
    // --- informational diagnostics are preserved, not rejected -----------
    let echoed = expect_ok(eval("echo", "echo(\"hi\"); cube(1);"), "an echo");
    let echo = echoed
        .diagnostics
        .iter()
        .find(|d| d.code == "scad_echo")
        .expect("an echo must be retained as scad_echo");
    assert_eq!(echo.class, ScadDiagnosticClass::Information);
    assert!(
        echo.message.contains("hi"),
        "the echo message must be preserved: {}",
        echo.message
    );

    let overwritten = expect_ok(
        eval("overwritten", "z = 1; z = 2; cube(z);"),
        "an overwritten assignment",
    );
    let lint = overwritten
        .diagnostics
        .iter()
        .find(|d| d.code == "assignment_overwritten")
        .expect("the verified overwritten-assignment lint must be retained");
    assert_eq!(lint.class, ScadDiagnosticClass::Information);
    assert!(
        lint.span.is_some(),
        "the dead assignment's byte span must be retained"
    );
    assert_close(
        mesh_volume_mm3(mesh(&overwritten, "overwritten")),
        8.0,
        "the last assignment wins",
    );

    // --- seeded evaluation repeats before and after another source -------
    let seeded_source = "echo(rands(0,1,3)); cube([1,1,1 + rands(0,3,1)[0]]);";
    let seeded_options = ScadEvalOptions {
        random_seed: Some(7),
        ..ScadEvalOptions::default()
    };
    let first = expect_ok(
        eval_with("seed-a", seeded_source, &seeded_options),
        "a seeded evaluation",
    );
    let first_echoes: Vec<String> = first
        .diagnostics
        .iter()
        .map(|d| d.message.clone())
        .collect();
    assert!(
        !first_echoes.is_empty(),
        "the seeded fixture echoes its stream"
    );

    // Interleave a different source *and* a different seed.
    expect_ok(
        eval_with(
            "seed-other",
            "echo(rands(0,1,3)); cube(2);",
            &ScadEvalOptions {
                random_seed: Some(99),
                ..ScadEvalOptions::default()
            },
        ),
        "an interleaved evaluation",
    );

    let repeat = expect_ok(
        eval_with("seed-b", seeded_source, &seeded_options),
        "a repeated seeded evaluation",
    );
    let repeat_echoes: Vec<String> = repeat
        .diagnostics
        .iter()
        .map(|d| d.message.clone())
        .collect();
    assert_eq!(
        first_echoes, repeat_echoes,
        "a seeded stream must repeat independently of prior calls"
    );
    // test-quality: determinism pins cover the full geometry and accepted
    // diagnostics, not only the rendered echo text.
    assert_eq!(first.geometry, repeat.geometry);
    assert_eq!(first.diagnostics, repeat.diagnostics);

    // A different seed must actually change the stream, or the fixture proves
    // nothing about seeding.
    let other_seed = expect_ok(
        eval_with(
            "seed-c",
            seeded_source,
            &ScadEvalOptions {
                random_seed: Some(8),
                ..ScadEvalOptions::default()
            },
        ),
        "a differently seeded evaluation",
    );
    let other_echoes: Vec<String> = other_seed
        .diagnostics
        .iter()
        .map(|d| d.message.clone())
        .collect();
    assert_ne!(
        first_echoes, other_echoes,
        "a different seed must produce a different stream"
    );
    assert_ne!(
        first.geometry, other_seed.geometry,
        "the fixture geometry must consume the seeded stream"
    );

    // --- concurrent seeded callers agree ---------------------------------
    let concurrent: Vec<ScadEvalOutput> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..4)
            .map(|_| {
                scope.spawn(|| {
                    expect_ok(
                        eval_with("seed-concurrent", seeded_source, &seeded_options),
                        "a concurrent seeded evaluation",
                    )
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().expect("join a seeded caller"))
            .collect()
    });
    for (index, output) in concurrent.iter().enumerate() {
        let echoes: Vec<String> = output
            .diagnostics
            .iter()
            .map(|d| d.message.clone())
            .collect();
        assert_eq!(
            echoes, first_echoes,
            "concurrent caller {index} must observe the same seeded stream"
        );
        assert_eq!(
            output.geometry, first.geometry,
            "concurrent geometry {index}"
        );
        assert_eq!(
            output.diagnostics, first.diagnostics,
            "concurrent diagnostics {index}"
        );
    }

    let literal_seed_source = "echo(rands(0,1,3,42)); cube([1,1,1 + rands(0,3,1,42)[0]]);";
    let literal_seed = expect_ok(
        eval("literal-seed", literal_seed_source),
        "literal seed without an option seed",
    );
    for random_seed in [None, Some(7), Some(99)] {
        let repeated = expect_ok(
            eval_with(
                "literal-seed-repeat",
                literal_seed_source,
                &ScadEvalOptions {
                    random_seed,
                    ..ScadEvalOptions::default()
                },
            ),
            "literal seed is independent of the option seed",
        );
        // test-quality: determinism pin for explicit rands(..., seed).
        assert_eq!(literal_seed.geometry, repeated.geometry);
        assert_eq!(literal_seed.diagnostics, repeated.diagnostics);
    }

    // --- no resolver or cache state survives a denied evaluation ----------
    expect_err(
        eval("denied-before", "include <missing.scad>; cube(1);"),
        ScadErrorKind::ExternalResourceDenied,
        "a denied include",
    );
    let after_denial = expect_ok(
        eval("clean-after-denial", "cube(3);"),
        "a clean evaluation after a denied one",
    );
    assert_close(
        mesh_volume_mm3(mesh(&after_denial, "clean-after-denial")),
        27.0,
        "the denial must not poison later calls",
    );

    // --- fonts come from bundled data, never from disk -------------------
    let text = expect_ok(
        eval("text", "linear_extrude(1) text(\"A\");"),
        "a text fixture",
    );
    let text_mesh = mesh(&text, "text");
    assert!(
        text_mesh.vertices.len() > 8,
        "a glyph is not a box: got {} vertices",
        text_mesh.vertices.len()
    );
    assert!(
        text.diagnostics.is_empty(),
        "bundled-font text must not report a diagnostic: {:?}",
        text.diagnostics
    );
}

// ---------------------------------------------------------------------------
// AC-N1
// ---------------------------------------------------------------------------

#[test]
fn scad_rejects_resources_even_with_remaining_geometry() {
    // A file that *does* exist at the exact path the source asks for, whose
    // content would visibly change the model. Denial must not depend on the file
    // being absent.
    let tempting_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/scad-adapter/tempting");
    std::fs::create_dir_all(&tempting_dir).expect("create tempting directory");
    let tempting_scad = tempting_dir.join("tempting.scad");
    std::fs::write(&tempting_scad, "cube(99);\n").expect("write tempting include");
    let tempting_mesh = tempting_dir.join("tempting.stl");
    std::fs::write(&tempting_mesh, b"solid tempting\nendsolid tempting\n")
        .expect("write tempting mesh");
    let tempting_path = tempting_scad.to_string_lossy().replace('\\', "/");
    let tempting_mesh_path = tempting_mesh.to_string_lossy().replace('\\', "/");

    let cases: [(&str, String); 7] = [
        (
            "include with a readable sibling primitive",
            format!("include <{tempting_path}>; cube(1);"),
        ),
        (
            "use with a readable sibling primitive",
            format!("use <{tempting_path}>; cube(1);"),
        ),
        (
            "mesh import with a readable sibling primitive",
            format!("import(\"{tempting_mesh_path}\"); cube(1);"),
        ),
        (
            "2D import with a readable sibling primitive",
            "import(\"drawing.svg\"); cube(1);".to_string(),
        ),
        (
            "surface with a readable sibling primitive",
            "surface(\"heightmap.dat\"); cube(1);".to_string(),
        ),
        (
            // `dxf_dim`/`dxf_cross` read through `load_bytes` from the PROGRAM
            // phase, not only from an override expression.
            "program dxf_dim with a readable sibling primitive",
            "echo(dxf_dim(file = \"tempting.dxf\", name = \"w\")); cube(1);".to_string(),
        ),
        (
            "program dxf_cross with a readable sibling primitive",
            "echo(dxf_cross(file = \"tempting.dxf\")); cube(1);".to_string(),
        ),
    ];

    for (label, source) in cases {
        let error = expect_err(
            eval(label, &source),
            ScadErrorKind::ExternalResourceDenied,
            label,
        );
        assert!(
            !error.message.is_empty(),
            "{label}: the denial must explain what was requested"
        );
        let request = error
            .diagnostics
            .iter()
            .find(|d| d.code == "external_resource_denied")
            .unwrap_or_else(|| panic!("{label}: the request must be retained as a diagnostic"));
        assert_eq!(
            request.class,
            ScadDiagnosticClass::Error,
            "{label}: a request is a hard error, not a warning"
        );
    }

    // A denial must not discard what the evaluation already reported: the request
    // is the primary error, but the echo and the unclassified warning are still
    // reproduction context.
    let retained = expect_err(
        eval(
            "denial-retains-diagnostics",
            "echo(\"preserve-me\");\nnonsense_module();\ninclude <missing.scad>;\ncube(1);",
        ),
        ScadErrorKind::ExternalResourceDenied,
        "a denial after an echo and an unknown construct",
    );
    assert!(
        retained
            .diagnostics
            .iter()
            .any(|d| d.code == "scad_echo" && d.message.contains("preserve-me")),
        "the echo must survive the denial: {:?}",
        retained
            .diagnostics
            .iter()
            .map(|d| d.code.clone())
            .collect::<Vec<_>>()
    );
    assert!(
        retained
            .diagnostics
            .iter()
            .any(|d| d.class == ScadDiagnosticClass::Unknown),
        "the unclassified warning must survive the denial"
    );

    // The tempting files live in `target/scad-adapter/tempting/`, not in this
    // process's working directory, so a working-directory fallback would have to
    // reach across directories to find them. The recorded request shows resolution
    // started at "." instead.
    assert!(
        std::env::current_dir()
            .expect("current directory")
            .join("tempting.scad")
            != tempting_scad,
        "the tempting include must not sit in the working directory"
    );

    // The include really would have changed the model had it been read.
    assert!(
        std::fs::read_to_string(&tempting_scad)
            .expect("read back tempting include")
            .contains("cube(99)"),
        "the tempting include must be a visible model change"
    );

    // A request that is never reached is not a request: scanning the source text
    // for the word `import` would be a false positive.
    let unreached = expect_ok(
        eval(
            "unreached-import",
            "if (false) import(\"never.stl\");\n// import(\"commented.stl\")\ncube(1);",
        ),
        "an unreached import and a commented import",
    );
    assert_close(
        mesh_volume_mm3(mesh(&unreached, "unreached-import")),
        1.0,
        "the sibling cube is the only geometry",
    );
}

// ---------------------------------------------------------------------------
// AC-N2
// ---------------------------------------------------------------------------

#[test]
fn scad_rejects_partial_approximate_degraded_and_unknown() {
    for source in [
        "union(){ cube(10); polyhedron(points=[[0,0,0],[1,0,0],[0,1,0]], faces=[[0,1,2]]); }",
        "projection() union(){ cube(10); polyhedron(points=[[0,0,0],[1,0,0],[0,1,0]], faces=[[0,1,2]]); }",
    ] {
        let error = expect_err(eval("boolean-fallback", source), ScadErrorKind::DegradedOutput, source);
        assert!(error.diagnostics.iter().any(|d| d.code == "boolean_fallback" && d.class == ScadDiagnosticClass::Degraded));
    }

    // --- root-recursion partial output -----------------------------------
    let partial = expect_err(
        eval(
            "partial",
            "function cum(list, k) = (k == 0) ? 0 : cum(list, k - 1) + list[k - 1];\
             cube(3); cube(cum([5], -1)); cube(7);",
        ),
        ScadErrorKind::PartialOutput,
        "a root-recursion abort",
    );
    assert!(
        partial
            .diagnostics
            .iter()
            .any(|d| d.code == "recursion_aborted"),
        "the partial result must retain its structured cause"
    );
    assert!(
        partial
            .diagnostics
            .iter()
            .any(|d| d.message.contains("Recursion")),
        "the engine message must be retained"
    );

    // --- concave/holed roof omission, on the 3D path ---------------------
    let roof = expect_err(
        eval(
            "roof",
            "roof(){ difference(){ square(10); translate([3,3]) square(4); } }",
        ),
        ScadErrorKind::ApproximateOutput,
        "a holed roof",
    );
    assert!(roof.diagnostics.iter().any(|d| d.code == "roof_omitted"));

    // --- concave-leaf Minkowski approximation, on the 3D path ------------
    let minkowski = expect_err(
        eval(
            "minkowski",
            "minkowski(){ linear_extrude(1) \
             polygon([[0,0],[6,0],[6,2],[2,2],[2,6],[0,6]]); sphere(1); }",
        ),
        ScadErrorKind::ApproximateOutput,
        "a concave 3D Minkowski",
    );
    assert!(minkowski
        .diagnostics
        .iter()
        .any(|d| d.code == "minkowski_approximation"));

    // --- the contour path reports its own diagnostics --------------------
    // A deliberately failing projected 3D child must not be accepted as clean 2D
    // contours: re-rendering it through the 3D path would prove nothing about the
    // projection/lowering traversal.
    let projected_minkowski = expect_err(
        eval(
            "projection-minkowski",
            "projection() minkowski(){ linear_extrude(1) \
             polygon([[0,0],[6,0],[6,2],[2,2],[2,6],[0,6]]); sphere(1); }",
        ),
        ScadErrorKind::ApproximateOutput,
        "a projected concave Minkowski",
    );
    assert!(projected_minkowski
        .diagnostics
        .iter()
        .any(|d| d.code == "minkowski_approximation"));

    let projected_roof = expect_err(
        eval(
            "projection-roof",
            "projection() roof(){ difference(){ square(10); translate([3,3]) square(4); } }",
        ),
        ScadErrorKind::ApproximateOutput,
        "a projected holed roof",
    );
    assert!(projected_roof
        .diagnostics
        .iter()
        .any(|d| d.code == "roof_omitted"));

    // --- an unclassified diagnostic channel ------------------------------
    let unknown = expect_err(
        eval("unknown", "nonsense_module(); cube(1);"),
        ScadErrorKind::UnknownDiagnostic,
        "an unclassified engine warning",
    );
    assert_eq!(
        unknown.diagnostics[0].class,
        ScadDiagnosticClass::Unknown,
        "the engine catch-all code must stay unclassified"
    );
    assert!(
        unknown.diagnostics[0].message.contains("nonsense_module"),
        "the retained message must identify the construct"
    );
}

// ---------------------------------------------------------------------------
// AC-N4
// ---------------------------------------------------------------------------

#[test]
fn scad_rejects_invalid_options_and_dimensions() {
    // --- malformed and statement-injecting parameters --------------------
    let bad_name = expect_err(
        eval_with("bad-name", "cube(1);", &options_with(&[("1x", "2")])),
        ScadErrorKind::InvalidParameter,
        "a parameter name that is not an identifier",
    );
    assert_eq!(bad_name.parameter.as_deref(), Some("1x"));

    // Reserved keywords are distinct token kinds in the adopted grammar, not
    // identifiers, so they are rejected as override names.
    for keyword in ["true", "false", "undef", "for", "module", "function", "let"] {
        let error = expect_err(
            eval_with("keyword", "cube(1);", &options_with(&[(keyword, "1")])),
            ScadErrorKind::InvalidParameter,
            keyword,
        );
        assert_eq!(
            error.parameter.as_deref(),
            Some(keyword),
            "{keyword} must be named as the offending parameter"
        );
    }

    let injected = expect_err(
        eval_with(
            "injected",
            "cube(1);",
            &options_with(&[("x", "1); cube(9); y = (2")]),
        ),
        ScadErrorKind::InvalidParameter,
        "a statement-injecting parameter expression",
    );
    assert_eq!(injected.parameter.as_deref(), Some("x"));
    let injection = "1); cube(9); y = (2";
    for (expression, expected_span) in [
        ("1 +", None),
        ("1 + @", Some(4..5)),
        (injection, Some(0..injection.len())),
        ("", None),
    ] {
        let error = expect_err(
            eval_with(
                "parameter-span",
                "cube(1);",
                &options_with(&[("x", expression)]),
            ),
            ScadErrorKind::InvalidParameter,
            expression,
        );
        assert_eq!(error.parameter.as_deref(), Some("x"));
        let diagnostic = error
            .diagnostics
            .iter()
            .find(|d| d.stage == slicer_scad::ScadDiagnosticStage::Parameter)
            .expect("parameter parse failure retains its diagnostic");
        // A synthetic closing-parenthesis span clamps to empty (None), while
        // an invalid byte inside the expression keeps its expression-local span.
        assert_eq!(diagnostic.span, expected_span, "{expression:?}");
        if let Some(span) = &diagnostic.span {
            assert!(
                span.start < span.end && span.end <= expression.len(),
                "{expression:?}: {span:?}"
            );
        }
    }

    // --- invalid limits ---------------------------------------------------
    let invalid_limits: [(&str, ScadLimits); 5] = [
        (
            "zero evaluation steps",
            ScadLimits {
                evaluation_steps: 0,
                ..ScadLimits::default()
            },
        ),
        (
            "unlimited evaluation steps",
            ScadLimits {
                evaluation_steps: u64::MAX,
                ..ScadLimits::default()
            },
        ),
        (
            "non-finite coordinate bound",
            ScadLimits {
                max_abs_coordinate_mm: f64::NAN,
                ..ScadLimits::default()
            },
        ),
        (
            "unsafe worker stack",
            ScadLimits {
                worker_stack_bytes: 1024 * 1024,
                ..ScadLimits::default()
            },
        ),
        (
            "zero source budget",
            ScadLimits {
                max_source_bytes: 0,
                ..ScadLimits::default()
            },
        ),
    ];
    for (label, limits) in invalid_limits {
        expect_err(
            eval_with(
                "invalid-limits",
                "cube(1);",
                &ScadEvalOptions {
                    limits,
                    ..ScadEvalOptions::default()
                },
            ),
            ScadErrorKind::InvalidOptions,
            label,
        );
    }

    // --- invalid extrusion heights ---------------------------------------
    for (label, height) in [
        ("negative height", -1.0),
        ("zero height", 0.0),
        ("non-finite height", f64::INFINITY),
        ("height beyond the coordinate bound", f64::INFINITY),
    ] {
        expect_err(
            eval_with(
                "invalid-height",
                "square(1);",
                &ScadEvalOptions {
                    extrude_height_mm: Some(height),
                    ..ScadEvalOptions::default()
                },
            ),
            ScadErrorKind::InvalidOptions,
            label,
        );
    }

    // --- a height on a 3D result, and mixed dimensions --------------------
    expect_err(
        eval_with(
            "height-on-3d",
            "cube(1);",
            &ScadEvalOptions {
                extrude_height_mm: Some(1.0),
                ..ScadEvalOptions::default()
            },
        ),
        ScadErrorKind::ExtrusionHeightOn3d,
        "a height on a 3D result",
    );

    expect_err(
        eval("mixed", "union(){ cube(1); square(1); }"),
        ScadErrorKind::MixedDimensions,
        "a mixed 2D/3D union",
    );
    for source in [
        "linear_extrude(1){ square(1); cube(2); }",
        "rotate_extrude(){ square(1); cube(2); }",
        "roof(){ square(1); cube(2); }",
        "projection() square(1);",
        "offset(1) cube(1);",
    ] {
        expect_err(
            eval("invalid-wrapper-child", source),
            ScadErrorKind::MixedDimensions,
            source,
        );
    }
    for source in [
        "polyhedron(points=[[0,0,0],[1,0,0],[0,1,0]], faces=[[0,1,9]]);",
        "union(){ cube(10); polyhedron(points=[[0,0,0],[1,0,0],[0,1,0]], faces=[[0,1,9]]); }",
        "projection() union(){ cube(10); polyhedron(points=[[0,0,0],[1,0,0],[0,1,0]], faces=[[0,1,9]]); }",
        "polyhedron(points=[[0,0,0],[1,0,0],[0,1,0]], faces=[[0,1]]);",
    ] {
        let error = expect_err(eval("invalid-polyhedron-face", source), ScadErrorKind::InvalidMesh, source);
        assert!(error.message.contains("polyhedron face"));
    }

    // --- declared complexity bounds ---------------------------------------
    expect_err(
        eval_with(
            "source-bytes",
            "cube(1);",
            &ScadEvalOptions {
                limits: ScadLimits {
                    max_source_bytes: 7,
                    ..ScadLimits::default()
                },
                ..ScadEvalOptions::default()
            },
        ),
        ScadErrorKind::ComplexityLimit,
        "a source above the byte budget",
    );

    // Parentheses are not AST nodes, so the nesting must be real operators:
    // `(1 + (1 + (1 + ...)))` produces one `Binary` node per level.
    let mut nested_expression = "1".to_string();
    for _ in 0..12 {
        nested_expression = format!("({nested_expression} + 1)");
    }
    let deep_source = format!("cube({nested_expression});");
    expect_err(
        eval_with(
            "ast-depth",
            &deep_source,
            &ScadEvalOptions {
                limits: ScadLimits {
                    max_ast_depth: 8,
                    ..ScadLimits::default()
                },
                ..ScadEvalOptions::default()
            },
        ),
        ScadErrorKind::ComplexityLimit,
        "an AST deeper than the depth budget",
    );

    expect_err(
        eval_with(
            "ast-nodes",
            "cube(1); cube(2); cube(3); translate([1,1,1]) cube(4);",
            &ScadEvalOptions {
                limits: ScadLimits {
                    max_ast_nodes: 4,
                    ..ScadLimits::default()
                },
                ..ScadEvalOptions::default()
            },
        ),
        ScadErrorKind::ComplexityLimit,
        "an AST larger than the node budget",
    );

    expect_err(
        eval_with(
            "geometry-nodes",
            "union(){ cube(1); translate([2,0,0]) cube(1); }",
            &ScadEvalOptions {
                limits: ScadLimits {
                    max_geometry_nodes: 2,
                    ..ScadLimits::default()
                },
                ..ScadEvalOptions::default()
            },
        ),
        ScadErrorKind::ComplexityLimit,
        "a CSG tree above the node budget",
    );

    expect_err(
        eval_with(
            "facets",
            "sphere(r = 1, $fn = 500);",
            &ScadEvalOptions {
                limits: ScadLimits {
                    max_facets: 360,
                    ..ScadLimits::default()
                },
                ..ScadEvalOptions::default()
            },
        ),
        ScadErrorKind::ComplexityLimit,
        "an explicit facet count above the facet budget",
    );

    expect_err(
        eval_with(
            "vertices",
            "cube(1);",
            &ScadEvalOptions {
                limits: ScadLimits {
                    max_vertices: 7,
                    ..ScadLimits::default()
                },
                ..ScadEvalOptions::default()
            },
        ),
        ScadErrorKind::ComplexityLimit,
        "a mesh with more vertices than allowed",
    );

    expect_err(
        eval_with(
            "triangles",
            "cube(1);",
            &ScadEvalOptions {
                limits: ScadLimits {
                    max_triangles: 11,
                    ..ScadLimits::default()
                },
                ..ScadEvalOptions::default()
            },
        ),
        ScadErrorKind::ComplexityLimit,
        "a mesh with more triangles than allowed",
    );

    // --- empty is empty, never fabricated thickness -----------------------
    for (label, source) in [
        ("background-only", "%cube(1);"),
        ("cancelled-difference", "difference(){ cube(1); cube(1); }"),
    ] {
        let empty = expect_ok(eval(label, source), label);
        assert!(
            matches!(empty.geometry, ScadGeometry::Empty),
            "{label} must be Empty, got {:?}",
            empty.geometry
        );
    }
    // A dimension-changing wrapper around empty geometry is still empty, alone and
    // next to valid sibling geometry.
    for (label, source) in [
        ("empty-extrude", "linear_extrude(1){}"),
        ("empty-projection", "projection(){}"),
        ("empty-roof", "roof(){}"),
    ] {
        let empty = expect_ok(eval(label, source), label);
        assert!(
            matches!(empty.geometry, ScadGeometry::Empty),
            "{label} must be Empty, got {:?}",
            empty.geometry
        );
    }
    let empty_sibling = expect_ok(
        eval(
            "empty-extrude-sibling",
            "union(){ linear_extrude(1){}; translate([3,0,0]) cube(1); }",
        ),
        "an empty extrusion beside a sibling cube",
    );
    assert_close(
        mesh_volume_mm3(mesh(&empty_sibling, "empty-extrude-sibling")),
        1.0,
        "the sibling cube is the only geometry",
    );

    // A 2D wrapper around nothing must not make its container look 2D: the engine's
    // 2D path would drop the 3D sibling entirely. Both wrappers must keep the cube.
    for (label, source) in [
        (
            "empty-projection-sibling",
            "union(){ projection(){}; translate([3,0,0]) cube(1); }",
        ),
        (
            "empty-offset-sibling",
            "union(){ offset(1){}; translate([3,0,0]) cube(1); }",
        ),
    ] {
        let sibling = expect_ok(eval(label, source), label);
        assert_close(
            mesh_volume_mm3(mesh(&sibling, label)),
            1.0,
            &format!("{label}: the cube must survive an empty 2D wrapper"),
        );
    }

    let empty_with_height = expect_ok(
        eval_with(
            "background-with-height",
            "%cube(1);",
            &ScadEvalOptions {
                extrude_height_mm: Some(2.0),
                ..ScadEvalOptions::default()
            },
        ),
        "an empty result with a valid height",
    );
    assert!(
        matches!(empty_with_height.geometry, ScadGeometry::Empty),
        "a valid height must not turn an empty result into a solid"
    );
}

// ---------------------------------------------------------------------------
// AC-N3 (public-path half): post-quantization damage reachable from a real source
// ---------------------------------------------------------------------------

#[test]
fn scad_public_path_rejects_quantization_damage() {
    // Two unit squares 2e-5 mm apart. In raw `f64` they are disjoint, so the exact
    // pre-quantization pass accepts them; at the 1e-4 mm output grid the gap rounds
    // away and the rings touch. This is a genuine public-path *post-quantization*
    // failure, and it must be rejected rather than repaired into a merged outline.
    let error = expect_err(
        eval(
            "quantization-contact",
            "union(){ square(1); translate([1.00002, 0]) square(1); }",
        ),
        ScadErrorKind::InvalidContours,
        "a gap that quantization closes",
    );
    assert!(
        error.message.contains("quantized grid"),
        "the rejection must name the quantized grid as the cause: {}",
        error.message
    );

    // The same source one grid step wider is accepted: the gap survives rounding,
    // so this is a boundary in the quantization, not a blanket rejection of unions.
    let accepted = expect_ok(
        eval(
            "quantization-gap-survives",
            "union(){ square(1); translate([1.00006, 0]) square(1); }",
        ),
        "a gap wider than half a grid unit",
    );
    assert_eq!(
        contours(&accepted, "quantization-gap-survives").len(),
        2,
        "two separated squares stay two components"
    );

    let conversion = error
        .diagnostics
        .iter()
        .find(|d| d.stage == slicer_scad::ScadDiagnosticStage::Conversion)
        .expect("a conversion rejection must be retained as a Conversion diagnostic");
    assert_eq!(conversion.class, ScadDiagnosticClass::Error);
}

// ---------------------------------------------------------------------------
// Measurements for the Windows feasibility record
// ---------------------------------------------------------------------------

/// Representative evaluation + exact-render costs for AC-3's fixtures, printed for
/// `docs/scad/windows-feasibility.md`.
///
/// Ignored by default: it is a measurement entry point, not an assertion about
/// wall-clock time. Run explicitly with
/// `cargo test -p slicer-scad --features openrscad --test scad_adapter_contracts -- --ignored --nocapture --exact scad_measure_representative_costs`.
#[test]
#[ignore = "measurement entry point, not a correctness assertion"]
fn scad_measure_representative_costs() {
    use std::time::Instant;

    let fixtures: [(&str, &str, Option<f64>); 3] = [
        (
            "holed-2d-contours",
            "difference(){ square(100); translate([10,10]) square(80); }",
            None,
        ),
        (
            "holed-2d-extruded",
            "difference(){ square(100); translate([10,10]) square(80); }",
            Some(2.0),
        ),
        (
            "indexed-3d",
            "difference(){ cube([10,10,2]); translate([2,2,-1]) cube([2,2,4]); \
             translate([6,6,-1]) cube([2,2,4]); }",
            None,
        ),
    ];

    for (label, source, height) in fixtures {
        // One warm-up so the measured run is not paying first-call costs.
        let options = ScadEvalOptions {
            extrude_height_mm: height,
            ..ScadEvalOptions::default()
        };
        let _ = eval_with(label, source, &options);

        let runs = 10;
        let mut durations = Vec::with_capacity(runs);
        for _ in 0..runs {
            let started = Instant::now();
            let output = expect_ok(eval_with(label, source, &options), label);
            durations.push(started.elapsed());
            match &output.geometry {
                ScadGeometry::Contours(regions) => {
                    assert!(!regions.is_empty(), "{label} must produce contours")
                }
                ScadGeometry::Mesh(mesh) => {
                    assert!(!mesh.indices.is_empty(), "{label} must produce triangles")
                }
                ScadGeometry::Empty => panic!("{label} must not be empty"),
            }
        }
        durations.sort();
        let median = durations[runs / 2];
        let p95 = durations[(runs * 95) / 100];
        println!(
            "MEASURE {label}: median={median:?} p95={p95:?} over {runs} runs (evaluate+render+convert)"
        );
    }
}
