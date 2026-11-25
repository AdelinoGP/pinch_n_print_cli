// Step 3 contract tests, included into `property_scripted_contracts.rs` so the
// acceptance-criterion test names stay top-level in that binary.

use std::sync::Mutex;

use slicer_sdk::test_support::property_testing::PropertyRun;

// ---------------------------------------------------------------------------
// Step 3: the real ValueTree protocol, useful shrinking, truthful failure
// ---------------------------------------------------------------------------

fn step3_bounds(max_evaluations: u32) -> RecipeBounds {
    // exhaustive: pins the shrink-protocol scenario bounds deliberately
    RecipeBounds {
        max_nodes: 48,
        max_depth: 8,
        max_facets: 32,
        max_source_bytes: 32 * 1024,
        max_evaluations,
    }
}

fn step3_options(recipe: &RecipeBounds, units: i64) -> slicer_scad::ScadEvalOptions {
    scripted_support::options(recipe, units, 400_000, 800_000, 20_000_000, 0x5c4d_0007)
}

fn step3_domain(
    class: TopologyClass,
    witness: Option<SolidWitness>,
    units: i64,
    max_evaluations: u32,
) -> ScriptedDomain {
    ScriptedDomain {
        // exhaustive: pins the shrink-protocol scenario domain capacities
        geometry: GeometryDomain {
            class,
            min_holes: 0,
            min_components: usize::from(class != TopologyClass::Empty),
            boundary: BoundaryRule::Strict,
            // exhaustive: pins the shrink-protocol scenario capacities
            bounds: GeometryBounds {
                max_abs_units: units,
                max_vertices: 96,
                max_holes: 8,
                max_components: 8,
                max_shrink_candidates: 64,
            },
        },
        solid_witness: witness,
        bounds: step3_bounds(max_evaluations),
    }
}

fn step3_runner() -> TestRunner {
    TestRunner::new(Config {
        cases: 4,
        rng_algorithm: RngAlgorithm::ChaCha,
        rng_seed: RngSeed::Fixed(0x5c4d_0007),
        failure_persistence: None,
        ..Config::default()
    })
}


fn body_count(case: &ScriptedCase) -> usize {
    contours_of(&case.output).len()
}

fn assert_reproduces(case: &ScriptedCase) {
    let source = emit_scad(&case.recipe, &case.recipe_bounds).expect("re-emit the retained recipe");
    assert_eq!(
        source, case.source,
        "the retained source must be exactly what the retained recipe emits"
    );
    let options = step3_options(&case.recipe_bounds, 40_000);
    let output = evaluate_recipe(&case.recipe, &options, &case.recipe_bounds)
        .expect("the retained recipe must still evaluate");
    assert_eq!(
        output.geometry, case.output.geometry,
        "the retained source must reproduce the retained geometry"
    );
}

#[test]
fn scripted_value_tree_protocol_and_useful_shrinking() {
    let units = 40_000;
    let mut runner = step3_runner();

    // A holed domain: the hole is required, so shrinking may remove ordinary
    // vertices but never the required hole.
    let holed = scripted_cases(
        step3_domain(TopologyClass::Holed, None, units, 128),
        step3_options(&step3_bounds(128), units),
    )
    .expect("a holed strategy");
    let mut tree = holed.new_tree(&mut runner).expect("a holed case");
    let initial = tree.current();
    assert_reproduces(&initial);
    let mut accepted = 0_usize;
    let mut previous = initial.clone();
    loop {
        let changed = tree.simplify();
        let case = tree.current();
        assert_reproduces(&case);
        let contours = match &case.output.geometry {
            slicer_scad::ScadGeometry::Contours(contours) => contours.clone(),
            other => panic!("a holed contour case produced {other:?}"),
        };
        validate_geometry(
            &GeometryCase {
                polygons: contours.clone(),
            },
            &step3_domain(TopologyClass::Holed, None, units, 128).geometry,
        )
        .expect("every exposed value satisfies its domain");
        assert!(
            scripted_support::hole_count(&contours) >= 1,
            "the required hole must survive shrinking"
        );
        if changed {
            accepted += 1;
            let complexity = recipe_complexity(&case.recipe).expect("complexity");
            let before = recipe_complexity(&previous.recipe).expect("complexity");
            assert!(
                complexity < before,
                "an accepted simplification must strictly decrease RecipeComplexity"
            );
        }
        previous = case;
        if !changed {
            break;
        }
    }
    assert!(accepted > 0, "the initial holed case must not be minimal");

    // Repeated complicate restores the exact pre-simplification value.
    let mut restored_steps = 0_usize;
    let mut previous = tree.current();
    while tree.complicate() {
        restored_steps += 1;
        let case = tree.current();
        assert_reproduces(&case);
        let now = recipe_complexity(&case.recipe).expect("complexity");
        let before = recipe_complexity(&previous.recipe).expect("complexity");
        assert!(
            now > before || case == initial,
            "complicate must restore a more complex certified predecessor"
        );
        previous = case;
    }
    assert!(
        restored_steps >= accepted,
        "every accepted simplification must be reversible"
    );
    assert_eq!(
        tree.current(),
        initial,
        "repeated complicate must restore the original certified case"
    );
    assert!(!tree.complicate(), "complicate is false at the starting value");
    assert_reproduces(&tree.current());

    // A disconnected domain keeps at least two components while shrinking, and
    // a false simplify leaves the value untouched.
    let disjoint = scripted_cases(
        step3_domain(TopologyClass::Disconnected, None, units, 128),
        step3_options(&step3_bounds(128), units),
    )
    .expect("a disconnected strategy");
    let mut tree = disjoint
        .new_tree(&mut runner)
        .expect("a disconnected case");
    let mut saw_simplify = false;
    for _ in 0..32 {
        let before = tree.current();
        let changed = tree.simplify();
        saw_simplify |= changed;
        let case = tree.current();
        assert_reproduces(&case);
        let slicer_scad::ScadGeometry::Contours(contours) = &case.output.geometry else {
            panic!("a disconnected contour case must stay contours");
        };
        assert!(contours.len() >= 2, "both bodies must survive");
        if !changed {
            assert_eq!(case, before, "a false simplify must not change the value");
            break;
        }
    }
    assert!(saw_simplify, "a disconnected case must shrink structurally");

    // Optional structure removal: a case with more than the required bodies
    // must lose bodies while keeping the required minimum.
    let mut optional_bodies = None;
    for _ in 0..8 {
        let strategy = scripted_cases(
            step3_domain(TopologyClass::Disconnected, None, units, 128),
            step3_options(&step3_bounds(128), units),
        )
        .expect("a disconnected strategy");
        let tree = strategy
            .new_tree(&mut runner)
            .expect("a disconnected case");
        let count = body_count(&tree.current());
        if count >= 3 {
            optional_bodies = Some(tree);
            break;
        }
    }
    let mut tree = optional_bodies.expect("a bounded catalog must reach three bodies");
    let initial_bodies = body_count(&tree.current());
    let mut removed = false;
    for _ in 0..32 {
        if !tree.simplify() {
            break;
        }
        let case = tree.current();
        assert_reproduces(&case);
        let count = body_count(&case);
        assert!(count >= 2, "required bodies must survive");
        removed |= count < initial_bodies;
        let slicer_scad::ScadGeometry::Contours(contours) = &case.output.geometry else {
            panic!("a disconnected case must stay contours");
        };
        validate_geometry(
            &GeometryCase {
                polygons: contours.clone(),
            },
            &step3_domain(TopologyClass::Disconnected, None, units, 128).geometry,
        )
        .expect("an optional-body shrink stays valid");
    }
    assert!(
        removed,
        "a case with optional bodies must actually lose one ({initial_bodies} initially)"
    );
    assert_eq!(
        body_count(&tree.current()),
        2,
        "the search must stop at the required two bodies, not earlier"
    );
    let terminal = tree.current();
    assert!(!tree.simplify(), "the terminal value must be a fixed point");
    assert_eq!(
        tree.current(),
        terminal,
        "a false simplify must leave the terminal value untouched"
    );

    // Optional hole removal: a case with more than the required hole must lose
    // holes while keeping the required one.
    let mut optional_holes = None;
    for _ in 0..32 {
        let mut domain = step3_domain(TopologyClass::Holed, None, units, 128);
        // The required minimum stays one (the Holed class minimum); the loop
        // only accepts a case with at least one *optional* extra hole so the
        // shrink must remove it.
        domain.geometry.min_holes = 1;
        domain.geometry.bounds.max_holes = 4;
        let strategy = scripted_cases(domain, step3_options(&step3_bounds(128), units))
            .expect("a holed strategy");
        let tree = strategy.new_tree(&mut runner).expect("a holed case");
        if scripted_support::hole_count(&contours_of(&tree.current().output)) >= 2 {
            optional_holes = Some(tree);
            break;
        }
    }
    let mut tree = optional_holes.expect("a bounded catalog must reach two holes");
    let initial_holes = scripted_support::hole_count(&contours_of(&tree.current().output));
    let mut removed = false;
    for _ in 0..32 {
        if !tree.simplify() {
            break;
        }
        let case = tree.current();
        assert_reproduces(&case);
        let holes = scripted_support::hole_count(&contours_of(&case.output));
        assert!(holes >= 1, "the required hole must survive");
        removed |= holes < initial_holes;
    }
    assert!(
        removed,
        "a case with optional holes must actually lose one ({initial_holes} initially)"
    );
    assert_eq!(
        scripted_support::hole_count(&contours_of(&tree.current().output)),
        1,
        "the search must stop at the required hole, not earlier"
    );
    let terminal = tree.current();
    assert!(!tree.simplify(), "the terminal value must be a fixed point");
    assert_eq!(
        tree.current(),
        terminal,
        "a false simplify must leave the terminal value untouched"
    );

    // A 3D CSG scene: structural reduction keeps a closed positive-volume solid
    // and the overhang witness survives.
    for witness in [SolidWitness::Overhang, SolidWitness::Cavity] {
        let solid = scripted_cases(
            step3_domain(TopologyClass::Convex, Some(witness), units, 128),
            step3_options(&step3_bounds(128), units),
        )
        .unwrap_or_else(|error| panic!("{witness:?} strategy: {error}"));
        let mut tree = solid
            .new_tree(&mut runner)
            .unwrap_or_else(|reason| panic!("{witness:?} case: {reason}"));
        let initial_volume = match &tree.current().output.geometry {
            slicer_scad::ScadGeometry::Mesh(mesh) => mesh_signed_volume_mm3(mesh).expect("volume"),
            other => panic!("a solid case produced {other:?}"),
        };
        let mut saw_simplify = false;
        for _ in 0..32 {
            let changed = tree.simplify();
            saw_simplify |= changed;
            let case = tree.current();
            assert_reproduces(&case);
            let slicer_scad::ScadGeometry::Mesh(mesh) = &case.output.geometry else {
                panic!("a solid case must stay a mesh");
            };
            validate_indexed_mesh(mesh, true).expect("every exposed solid stays closed");
            let volume = mesh_signed_volume_mm3(mesh).expect("volume");
            assert!(volume > 0.0, "every exposed solid keeps positive volume");
            if !changed {
                break;
            }
            if witness == SolidWitness::Overhang {
                assert!(
                    scripted_support::has_unsupported_span(mesh),
                    "the overhang witness must survive shrinking"
                );
            }
        }
        assert!(
            saw_simplify,
            "{witness:?} scene must shrink structurally from a nonminimal start"
        );
        let final_volume = match &tree.current().output.geometry {
            slicer_scad::ScadGeometry::Mesh(mesh) => mesh_signed_volume_mm3(mesh).expect("volume"),
            other => panic!("a solid case produced {other:?}"),
        };
        assert!(
            final_volume <= initial_volume,
            "structural shrinking must not grow the solid"
        );
    }
}

// ---------------------------------------------------------------------------
// Real foundation completion path
// ---------------------------------------------------------------------------

static CONTEXT_LOCK: Mutex<()> = Mutex::new(());

fn unique_dir(label: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/property-testing")
        .join(format!(
            "scripted-contracts-{label}-{}",
            std::process::id()
        ))
}

/// Run `run` with a real registry context in the process environment.
///
/// `PNP_PROPERTY_CONTEXT` is process-global, so every scripted completion-path
/// test holds one lock and clears the variable afterwards.
fn with_context<T>(
    label: &str,
    test_name: &str,
    prepare: impl FnOnce(&std::path::Path),
    run: impl FnOnce() -> T,
) -> (T, std::path::PathBuf) {
    let _guard = CONTEXT_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let dir = unique_dir(label);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create the artifact directory");
    prepare(&dir);
    let context = scripted_support::context(&dir, "D-SCAD-2D", test_name, 4, 0x5c4d_0007);
    std::env::set_var("PNP_PROPERTY_CONTEXT", context.to_string());
    let result = run();
    std::env::remove_var("PNP_PROPERTY_CONTEXT");
    (result, dir)
}

#[test]
fn scripted_generation_and_shrink_exhaustion_are_not_success() {
    let units = 40_000;
    // One candidate evaluation per tree: the initial case consumes it, so any
    // shrink step hits the limit instead of silently searching further.
    let bounds = step3_bounds(1);
    let strategy = scripted_cases(
        step3_domain(TopologyClass::Holed, None, units, 1),
        step3_options(&bounds, units),
    )
    .expect("a holed strategy");

    // A real bounded scripted run passes, and every callback value is certified.
    let (passed, _) = with_context(
        "passed",
        "proptest_scripted_contracts_passed",
        |_| {},
        || {
            let run = PropertyRun::new("D-SCAD-2D", "proptest_scripted_contracts_passed")?;
            run.check(&strategy, |case, _guard| {
                let contours = match &case.output.geometry {
                    slicer_scad::ScadGeometry::Contours(contours) => contours.clone(),
                    other => panic!("a contour case produced {other:?}"),
                };
                assert!(scripted_support::hole_count(&contours) >= 1);
                Ok(())
            })
        },
    );
    assert!(
        passed.is_ok(),
        "a real bounded scripted run must pass: {passed:?}"
    );

    // An all-rejected population exhausts the rejection budget and is distinct
    // from a passing or assertion-failing run.
    let (exhausted, _) = with_context(
        "exhausted",
        "proptest_scripted_contracts_exhausted",
        |_| {},
        || {
            let run = PropertyRun::new("D-SCAD-2D", "proptest_scripted_contracts_exhausted")?;
            run.check(&strategy, |_case, _guard| {
                Err(proptest::test_runner::TestCaseError::reject("always reject"))
            })
        },
    );
    let error = exhausted.expect_err("an all-rejected population must not pass");
    assert_eq!(error.kind(), "exhausted", "got {error:?}");

    // A known failing property with a one-evaluation budget cannot claim a
    // completed shrink, and its retained failure record says so.
    let (failed, failed_dir) = with_context(
        "failed",
        "proptest_scripted_contracts_failed",
        |_| {},
        || {
            let run = PropertyRun::new("D-SCAD-2D", "proptest_scripted_contracts_failed")?;
            run.check(&strategy, |_case, _guard| {
                Err(proptest::test_runner::TestCaseError::fail(
                    "known failing probe",
                ))
            })
        },
    );
    let error = failed.expect_err("a failing property must not pass");
    assert_eq!(error.kind(), "sut_failure", "got {error:?}");
    let failure: serde_json::Value = serde_json::from_slice(
        &std::fs::read(failed_dir.join("failure.json")).expect("a retained failure record"),
    )
    .expect("failure.json");
    assert_eq!(failure["classification"], "sut_or_contract");
    assert_eq!(
        failure["shrink_completed"], false,
        "a shrink cut short by the evaluation budget must not claim completion: {failure}"
    );
    assert!(
        failed_dir.join("observations.jsonl").is_file(),
        "the retained attempt keeps its callback observations"
    );

    // A candidate/proposal limit is reported as incomplete shrinking, not as a
    // completed minimization. One proposal is allowed, so the second hits it.
    let mut proposal_domain = step3_domain(TopologyClass::Holed, None, units, 128);
    proposal_domain.geometry.bounds.max_shrink_candidates = 1;
    let proposal_strategy = scripted_cases(
        proposal_domain,
        step3_options(&step3_bounds(128), units),
    )
    .expect("a proposal-limited strategy");
    let (proposal_result, proposal_dir) = with_context(
        "proposal-limit",
        "proptest_scripted_contracts_proposal_limit",
        |_| {},
        || {
            let run =
                PropertyRun::new("D-SCAD-2D", "proptest_scripted_contracts_proposal_limit")?;
            run.check(&proposal_strategy, |_case, _guard| {
                Err(proptest::test_runner::TestCaseError::fail(
                    "known failing proposal-limit probe",
                ))
            })
        },
    );
    let error = proposal_result.expect_err("a failing property must not pass");
    assert_eq!(error.kind(), "sut_failure", "got {error:?}");
    let failure: serde_json::Value = serde_json::from_slice(
        &std::fs::read(proposal_dir.join("failure.json")).expect("a retained failure record"),
    )
    .expect("failure.json");
    assert_eq!(
        failure["shrink_completed"], false,
        "a proposal limit must not claim completed minimization: {failure}"
    );

    // A fatal artifact failure outside any callback cannot end in a pass.
    let (fatal, _) = with_context(
        "blocked",
        "proptest_scripted_contracts_fatal",
        |dir| {
            // A file where the attempts directory must go makes every
            // pre-execution attempt journal unwritable.
            std::fs::write(dir.join("attempts"), b"not a directory")
                .expect("block the attempts path");
        },
        || {
            let run = PropertyRun::new("D-SCAD-2D", "proptest_scripted_contracts_fatal")?;
            run.check(&strategy, |_case, _guard| Ok(()))
        },
    );
    let error = fatal.expect_err("an unwritable attempt journal must fail the run");
    assert_eq!(
        error.kind(),
        "generator_or_oracle",
        "a fatal generator error must be classified, got {error:?}"
    );
}
