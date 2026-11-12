// Step 4 contract tests: durable artifacts, provenance-checked replay, and an
// engine-independent concrete fixture. Included into `property_scripted_contracts.rs`.

use std::path::{Path, PathBuf};
use std::process::Command;

use slicer_sdk::test_support::property_testing::regression::{
    format_regression_fixture, ConcreteInput,
};

const FIXED_FILES: [&str; 9] = [
    "recipe.json",
    "recipe-bounds.json",
    "source.scad",
    "options.json",
    "engine.json",
    "state.json",
    "geometry.json",
    "diagnostics.json",
    "output.json",
];

fn artifact_dir(label: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/property-testing")
        .join(format!("scripted-artifacts-{label}-{}", std::process::id()))
}

fn write_case(label: &str) -> (PathBuf, ScriptedCase) {
    let _guard = CONTEXT_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    std::env::remove_var("PNP_PROPERTY_CONTEXT");
    let bounds = step3_bounds(64);
    let recipe = ScadRecipe::Contours(Recipe2d::Difference {
        base: Box::new(Recipe2d::Rectangle {
            size_units: [8_000, 6_000],
        }),
        cuts: vec![Recipe2d::Translate {
            offset_units: [3_000, 2_000],
            child: Box::new(Recipe2d::Disk {
                radius_units: 1_500,
                facets: 9,
            }),
        }],
    });
    let options = step3_options(&bounds, 40_000);
    let source = emit_scad(&recipe, &bounds).expect("emit");
    let output = evaluate_recipe(&recipe, &options, &bounds).expect("evaluate");
    let case = ScriptedCase { // exhaustive: complete replay-record contract
        recipe,
        recipe_bounds: bounds,
        source,
        options,
        output,
    };
    let dir = artifact_dir(label);
    let _ = std::fs::remove_dir_all(&dir);
    slicer_sdk::test_support::property_testing::scripted::write_scripted_artifacts(&dir, &case)
        .expect("write a complete evaluated case");
    (dir, case)
}

fn read_state(dir: &Path) -> serde_json::Value {
    serde_json::from_slice(&std::fs::read(dir.join("state.json")).expect("state.json"))
        .expect("state json")
}

fn rewrite_state(dir: &Path, state: &serde_json::Value) {
    std::fs::write(
        dir.join("state.json"),
        serde_json::to_vec_pretty(state).expect("state"),
    )
    .expect("rewrite state.json");
}

fn scripted_artifacts_replay_body() {
    let (dir, case) = write_case("complete");

    // Every fixed file exists, and the evaluated state is honest about its
    // stage and digests.
    for name in FIXED_FILES {
        assert!(dir.join(name).is_file(), "missing artifact {name}");
    }
    let state = read_state(&dir);
    assert_eq!(state["schema_version"], 1);
    assert_eq!(state["stage"], "evaluated");
    assert_eq!(state["generator_version"], 1);
    for digest in ["source_digest", "input_digest"] {
        let value = state[digest].as_str().expect("digest string");
        assert_eq!(value.len(), 64, "{digest} must be a SHA-256 hex");
        assert!(
            value
                .chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
            "{digest} must be lowercase hex"
        );
    }
    // The retained provenance names the active engine, not a guessed pin.
    let engine: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.join("engine.json")).expect("engine"))
            .expect("engine json");
    assert_eq!(engine, serde_json::to_value(slicer_scad::engine_info().expect("engine")).expect("json"));
    let options: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.join("options.json")).expect("options"))
            .expect("options json");
    assert!(
        options["limits"]["evaluation_steps"].as_u64().is_some(),
        "the retained options keep their measured limits"
    );

    // Direct replay is byte-identical and needs no seed regeneration.
    let replayed = slicer_sdk::test_support::property_testing::scripted::replay_scripted_case(&dir)
        .expect("replay the retained case");
    assert_eq!(replayed.recipe, case.recipe);
    assert_eq!(replayed.source, case.source);
    assert_eq!(replayed.options, case.options);
    assert_eq!(replayed.output.geometry, case.output.geometry);
    assert_eq!(replayed.output.provenance, case.output.provenance);

    // The same explicit seed reproduces the geometry exactly, twice.
    let options = step3_options(&case.recipe_bounds, 40_000);
    let first = evaluate_recipe(&case.recipe, &options, &case.recipe_bounds).expect("first");
    let second = evaluate_recipe(&case.recipe, &options, &case.recipe_bounds).expect("second");
    assert_eq!(first.geometry, second.geometry);
    assert_eq!(first.provenance, second.provenance);

    // Concrete fixtures: a contour case maps to Geometry, a mesh to Mesh, and
    // the emitted statements compile and evaluate without engine or Proptest.
    let holes = match &case.output.geometry {
        slicer_scad::ScadGeometry::Contours(contours) => contours.clone(),
        other => panic!("expected contours, got {other:?}"),
    };
    let geometry_block = format_regression_fixture(
        "scripted-contour-retained-case",
        "previously failing scripted contour case",
        &ConcreteInput::Geometry(GeometryCase {
            polygons: holes.clone(),
        }),
    )
    .expect("format a geometry fixture");

    let solid_bounds = step3_bounds(64);
    let solid_recipe = ScadRecipe::Solid(Recipe3d::Difference {
        base: Box::new(Recipe3d::Box {
            size_units: [6_000, 6_000, 6_000],
        }),
        cuts: vec![Recipe3d::Translate {
            offset_units: [1_500, 1_500, 1_500],
            child: Box::new(Recipe3d::Box {
                size_units: [3_000, 3_000, 3_000],
            }),
        }],
    });
    let solid_options = step3_options(&solid_bounds, 40_000);
    let solid_output = evaluate_recipe(&solid_recipe, &solid_options, &solid_bounds)
        .expect("evaluate the solid case");
    let mesh = match &solid_output.geometry {
        slicer_scad::ScadGeometry::Mesh(mesh) => mesh.clone(),
        other => panic!("expected a mesh, got {other:?}"),
    };
    let mesh_block = format_regression_fixture(
        "scripted-solid-retained-case",
        "previously failing scripted solid case",
        &ConcreteInput::Mesh(mesh.clone()),
    )
    .expect("format a mesh fixture");

    for (name, block) in [
        ("geometry", &geometry_block),
        ("mesh", &mesh_block),
    ] {
        for forbidden in [
            "slicer_scad",
            "OpenSCAD",
            "openscad",
            "proptest",
            "PropertyRun",
            "PNP_PROPERTY_CONTEXT",
        ] {
            assert!(
                !block.contains(forbidden),
                "{name} fixture must not reference {forbidden}:\n{block}"
            );
        }
    }

    // Compile and run the fixtures in a probe crate that only depends on the IR
    // and the SDK's `test` feature: no engine, no property harness.
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let token = format!("scripted-fixture-{}", std::process::id());
    let probe = workspace
        .join("target/property-regression-probe")
        .join(&token);
    std::fs::create_dir_all(probe.join("src")).expect("probe directory");
    let to_toml = |path: &Path| path.to_string_lossy().replace('\\', "/");
    let manifest = format!(
        "[package]\nname = \"scripted-fixture-probe-{pid}\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[workspace]\n\n[dependencies]\nslicer-ir = {{ path = \"{}\" }}\nslicer-sdk = {{ path = \"{}\", features = [\"test\"] }}\n",
        to_toml(&workspace.join("crates/slicer-ir")),
        to_toml(&workspace.join("crates/slicer-sdk")),
        pid = std::process::id(),
    );
    assert!(!manifest.contains("proptest"));
    std::fs::write(probe.join("Cargo.toml"), &manifest).expect("probe manifest");
    let encoder = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/support/regression_encode.rs"
    ))
    .expect("shared encoder source");
    let mut main = String::from("use slicer_ir::*;\n\n");
    main.push_str(&encoder);
    main.push_str(
        "\nfn main() {\n    let mut out = String::new();\n    {\n",
    );
    main.push_str(&geometry_block);
    main.push_str("        out.push_str(&encode_geometry(&input));\n        out.push('\\n');\n    }\n    {\n");
    main.push_str(&mesh_block);
    main.push_str("        out.push_str(&encode_mesh(&input));\n        out.push('\\n');\n    }\n    print!(\"{}\", out);\n}\n");
    std::fs::write(probe.join("src/main.rs"), main).expect("probe source");

    // Keep the readable fixture target-local; never a committed seed file.
    let fixture_dir = workspace
        .join("target/property-testing")
        .join(&token)
        .join("regressions");
    std::fs::create_dir_all(&fixture_dir).expect("fixture directory");
    std::fs::write(
        fixture_dir.join("fixture.rs"),
        format!("{geometry_block}\n{mesh_block}"),
    )
    .expect("retained fixture");

    let mut command = Command::new("cargo");
    command
        .args(["run", "--quiet", "--offline"])
        .current_dir(&probe)
        .env("CARGO_TARGET_DIR", workspace.join("target"))
        .env_remove("PNP_PROPERTY_CONTEXT")
        .env_remove("RUSTFLAGS")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .env_remove("_PROPTEST_FORKFILE");
    let output = command
        .output()
        .expect("cargo must be runnable for the fixture probe");
    assert!(
        output.status.success(),
        "fixture probe failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let tree = Command::new("cargo")
        .args(["tree", "--offline", "-e", "normal"])
        .current_dir(&probe)
        .env("CARGO_TARGET_DIR", workspace.join("target"))
        .env_remove("RUSTFLAGS")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .output()
        .expect("cargo tree must be runnable");
    assert!(tree.status.success(), "cargo tree failed");
    let tree_text = String::from_utf8_lossy(&tree.stdout);
    for forbidden in ["proptest", "slicer-scad", "openrscad", "num-bigint"] {
        assert!(
            !tree_text.contains(forbidden),
            "the concrete fixture graph must not contain {forbidden}:\n{tree_text}"
        );
    }
    let expected = [
        regression_encode::encode_geometry(&holes),
        regression_encode::encode_mesh(&mesh),
    ]
    .join("\n");
    let actual = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
    assert_eq!(
        actual.trim_end(),
        expected.trim_end(),
        "the compiled fixture must rebuild exactly the retained concrete input"
    );
}

fn scripted_replay_refusals_body() {
    use slicer_sdk::test_support::property_testing::scripted::{
        replay_scripted_case, write_scripted_artifacts, ScriptedError,
    };

    fn fresh(label: &str) -> (PathBuf, ScriptedCase) {
        write_case(label)
    }

    // Changed source bytes.
    let (dir, _) = fresh("changed-source");
    let mut source = std::fs::read(dir.join("source.scad")).expect("source");
    source.push(b' ');
    std::fs::write(dir.join("source.scad"), &source).expect("tamper source");
    assert!(matches!(
        replay_scripted_case(&dir),
        Err(ScriptedError::ReplayMismatch(_))
    ));

    // Changed recipe, bounds and options each change the input digest.
    for name in ["recipe.json", "recipe-bounds.json", "options.json"] {
        let (dir, _) = fresh(&format!("changed-{name}"));
        let mut bytes = std::fs::read(dir.join(name)).expect(name);
        // A benign JSON edit: whitespace inside the object is still a change.
        bytes.insert(1, b' ');
        std::fs::write(dir.join(name), &bytes).expect("tamper");
        assert!(
            matches!(
                replay_scripted_case(&dir),
                Err(ScriptedError::ReplayMismatch(_))
            ),
            "a changed {name} must not replay"
        );
    }

    // Incompatible generator version.
    let (dir, _) = fresh("generator-version");
    let mut state = read_state(&dir);
    state["generator_version"] = serde_json::json!(2);
    rewrite_state(&dir, &state);
    assert!(matches!(
        replay_scripted_case(&dir),
        Err(ScriptedError::ReplayMismatch(_))
    ));

    // Different engine revision, backend or patch identity.
    for field in ["revision", "backend", "patch_digest"] {
        let (dir, _) = fresh(&format!("engine-{field}"));
        let mut engine: serde_json::Value =
            serde_json::from_slice(&std::fs::read(dir.join("engine.json")).expect("engine"))
                .expect("engine json");
        engine[field] = serde_json::json!("tampered-identity");
        std::fs::write(
            dir.join("engine.json"),
            serde_json::to_vec_pretty(&engine).expect("engine"),
        )
        .expect("tamper engine");
        // The digest check fires first for an engine change.
        assert!(
            matches!(
                replay_scripted_case(&dir),
                Err(ScriptedError::ReplayMismatch(_))
            ),
            "a changed engine {field} must not replay"
        );
    }

    // A consistent engine edit (digest recomputed) must still be refused by the
    // active-identity comparison, not silently accepted.
    let (dir, _case) = fresh("engine-consistent");
    let mut engine: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.join("engine.json")).expect("engine"))
            .expect("engine json");
    engine["revision"] = serde_json::json!("tampered-but-digested");
    let engine_bytes = serde_json::to_vec_pretty(&engine).expect("engine");
    std::fs::write(dir.join("engine.json"), &engine_bytes).expect("tamper engine");
    let mut state = read_state(&dir);
    state["input_digest"] = serde_json::json!(tampered_input_digest(&dir, &engine_bytes));
    rewrite_state(&dir, &state);
    assert!(
        matches!(
            replay_scripted_case(&dir),
            Err(ScriptedError::ReplayMismatch(_))
        ),
        "a self-consistent but different engine record must not replay"
    );

    // Missing and truncated files are artifact damage, not silent defaults.
    let (dir, _) = fresh("missing-file");
    std::fs::remove_file(dir.join("recipe.json")).expect("remove");
    assert!(matches!(
        replay_scripted_case(&dir),
        Err(ScriptedError::Artifact(_))
    ));
    let (dir, _) = fresh("truncated-state");
    std::fs::write(dir.join("state.json"), b"{\"schema_version\": 1").expect("truncate");
    assert!(matches!(
        replay_scripted_case(&dir),
        Err(ScriptedError::Artifact(_))
    ));

    // A directory where a file is expected is refused as not-regular, and a
    // non-directory replay root is refused outright.
    let (dir, _) = fresh("directory-instead-of-file");
    std::fs::remove_file(dir.join("source.scad")).expect("remove");
    std::fs::create_dir(dir.join("source.scad")).expect("directory");
    assert!(matches!(
        replay_scripted_case(&dir),
        Err(ScriptedError::Artifact(_))
    ));
    assert!(matches!(
        replay_scripted_case(&dir.join("state.json")),
        Err(ScriptedError::Artifact(_))
    ));

    // An oversize artifact is refused before loading it.
    let (dir, _) = fresh("oversize");
    let oversize = vec![b'0'; 17 * 1024 * 1024];
    std::fs::write(dir.join("source.scad"), &oversize).expect("write oversize");
    assert!(matches!(
        replay_scripted_case(&dir),
        Err(ScriptedError::Artifact(_))
    ));

    // A Windows directory junction where a file is expected is refused. Junction
    // creation is available without elevation; if the environment denies it the
    // probe says so instead of silently passing as if it had run.
    let (dir, _) = fresh("junction");
    std::fs::remove_file(dir.join("source.scad")).expect("remove source");
    let outside = dir.join("outside-directory");
    std::fs::create_dir_all(&outside).expect("outside directory");
    // `mklink` rejects paths containing `..`, so canonicalize both first.
    let link_path = std::fs::canonicalize(&dir)
        .expect("canonical replay directory")
        .join("source.scad");
    let target_path = std::fs::canonicalize(&outside).expect("canonical outside directory");
    let command = format!(
        "mklink /J \"{}\" \"{}\"",
        link_path.display(),
        target_path.display()
    );
    let junction = Command::new("cmd.exe")
        .args(["/C", &command])
        .output();
    let created = junction
        .as_ref()
        .map(|output| output.status.success())
        .unwrap_or(false);
    if created {
        assert!(
            matches!(
                replay_scripted_case(&dir),
                Err(ScriptedError::Artifact(_))
            ),
            "a junction where a regular file is expected must be refused"
        );
    } else {
        // Report the actual failure rather than passing silently.
        match &junction {
            Ok(output) => println!(
                "junction probe unavailable (status {:?}):
stdout: {}
stderr: {}",
                output.status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ),
            Err(error) => println!("junction probe unavailable: {error}"),
        }
        // A file symlink is the other escape primitive. It needs Developer Mode
        // or the symlink privilege; if it is available, the same refusal applies.
        std::fs::write(outside.join("payload.scad"), b"// escape payload
")
            .expect("outside payload");
        let command = format!(
            "mklink \"{}\" \"{}\"",
            link_path.display(),
            target_path.join("payload.scad").display()
        );
        let symlink = Command::new("cmd.exe").args(["/C", &command]).output();
        let created = symlink
            .as_ref()
            .map(|output| output.status.success())
            .unwrap_or(false);
        if created {
            assert!(
                matches!(
                    replay_scripted_case(&dir),
                    Err(ScriptedError::Artifact(_))
                ),
                "a file symlink where a regular file is expected must be refused"
            );
        } else {
            match &symlink {
                Ok(output) => println!(
                    "symlink probe unavailable (status {:?}): {}",
                    output.status,
                    String::from_utf8_lossy(&output.stderr)
                ),
                Err(error) => println!("symlink probe unavailable: {error}"),
            }
        }
    }

    // A failed attempt keeps a readable input set but is not a replayable
    // evaluated case.
    let (dir, case) = fresh("failed-stage");
    let mut state = read_state(&dir);
    state["stage"] = serde_json::json!("failed");
    rewrite_state(&dir, &state);
    assert!(matches!(
        replay_scripted_case(&dir),
        Err(ScriptedError::Artifact(_))
    ));
    // Re-writing the complete case restores the replayable state.
    write_scripted_artifacts(&dir, &case).expect("rewrite the complete case");
    assert!(replay_scripted_case(&dir).is_ok());
}

/// Recompute the documented input-digest framing for a tampered input set.
fn tampered_input_digest(dir: &Path, engine_bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(b"pnp-scripted-inputs-v1\n");
    for (name, bytes) in [
        ("recipe.json", std::fs::read(dir.join("recipe.json")).expect("recipe")),
        (
            "recipe-bounds.json",
            std::fs::read(dir.join("recipe-bounds.json")).expect("bounds"),
        ),
        ("source.scad", std::fs::read(dir.join("source.scad")).expect("source")),
        ("options.json", std::fs::read(dir.join("options.json")).expect("options")),
        ("engine.json", engine_bytes.to_vec()),
    ] {
        hasher.update(name.as_bytes());
        hasher.update(b"\n");
        hasher.update(bytes.len().to_string().as_bytes());
        hasher.update(b"\n");
        hasher.update(&bytes);
        hasher.update(b"\n");
    }
    hasher.update(b"generator_version\n1\n");
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(64);
    for byte in digest {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

/// Measurement entry point for the evidence document. `#[ignore]` keeps it out
/// of ordinary runs; invoke with `--ignored --nocapture` to record costs.
#[test]
#[ignore = "measurement entry point, not a correctness assertion"]
fn scripted_measure_representative_costs() {
    use std::time::Instant;

    let bounds = step3_bounds(64);
    let options = step3_options(&bounds, 40_000);
    let recipes = vec![
        (
            "rectangle",
            ScadRecipe::Contours(Recipe2d::Rectangle {
                size_units: [20_000, 12_000],
            }),
        ),
        (
            "faceted-disk",
            ScadRecipe::Contours(Recipe2d::Disk {
                radius_units: 5_000,
                facets: 16,
            }),
        ),
        (
            "hole-difference",
            ScadRecipe::Contours(Recipe2d::Difference {
                base: Box::new(Recipe2d::Rectangle {
                    size_units: [20_000, 20_000],
                }),
                cuts: vec![Recipe2d::Translate {
                    offset_units: [10_000, 10_000],
                    child: Box::new(Recipe2d::Disk {
                        radius_units: 4_000,
                        facets: 12,
                    }),
                }],
            }),
        ),
        (
            "cavity-solid",
            ScadRecipe::Solid(Recipe3d::Difference {
                base: Box::new(Recipe3d::Box {
                    size_units: [8_000, 8_000, 8_000],
                }),
                cuts: vec![Recipe3d::Translate {
                    offset_units: [2_000, 2_000, 2_000],
                    child: Box::new(Recipe3d::Box {
                        size_units: [4_000, 4_000, 4_000],
                    }),
                }],
            }),
        ),
        (
            "bridge-solid",
            ScadRecipe::Solid(Recipe3d::Union(vec![
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
            ])),
        ),
    ];
    let iterations = 20;
    let mut emit_total = std::time::Duration::ZERO;
    let mut render_total = std::time::Duration::ZERO;
    for _ in 0..iterations {
        for (_, recipe) in &recipes {
            let start = Instant::now();
            let _ = emit_scad(recipe, &bounds).expect("emit");
            emit_total += start.elapsed();
            let start = Instant::now();
            let _ = evaluate_recipe(recipe, &options, &bounds).expect("evaluate");
            render_total += start.elapsed();
        }
    }
    let samples = iterations * recipes.len();
    println!(
        "generation: {} samples, {:?} total, {:?} per recipe",
        samples,
        emit_total,
        emit_total / samples as u32
    );
    println!(
        "render: {} samples, {:?} total, {:?} per recipe",
        samples,
        render_total,
        render_total / samples as u32
    );

    // Recipe-strategy generation is a distinct phase from emission: time the
    // `new_tree` call that selects a family, builds the recipe and certifies it.
    let mut generation_total = std::time::Duration::ZERO;
    let mut generation_samples = 0_usize;
    for class in [TopologyClass::Holed, TopologyClass::Disconnected] {
        let strategy = scripted_cases(
            step3_domain(class, None, 40_000, 128),
            options.clone(),
        )
        .expect("strategy");
        let mut runner = step3_runner();
        for _ in 0..8 {
            let start = Instant::now();
            let _ = strategy.new_tree(&mut runner).expect("case");
            generation_total += start.elapsed();
            generation_samples += 1;
        }
    }
    println!(
        "strategy-generation: {} samples, {:?} total, {:?} per tree",
        generation_samples,
        generation_total,
        generation_total / generation_samples as u32
    );

    let mut shrink_total = std::time::Duration::ZERO;
    let mut shrink_steps = 0_usize;
    let mut evaluations = 0_usize;
    for class in [TopologyClass::Holed, TopologyClass::Disconnected] {
        let mut runner = step3_runner();
        let strategy = scripted_cases(
            step3_domain(class, None, 40_000, 128),
            options.clone(),
        )
        .expect("strategy");
        for _ in 0..4 {
            let mut tree = strategy.new_tree(&mut runner).expect("case");
            let mut first = true;
            loop {
                let start = Instant::now();
                let changed = tree.simplify();
                shrink_total += start.elapsed();
                if first {
                    evaluations += 1;
                    first = false;
                }
                if changed {
                    shrink_steps += 1;
                } else {
                    break;
                }
            }
        }
    }
    println!(
        "shrink: {} accepted steps across 8 trees, {:?} total, {:?} per accepted step",
        shrink_steps,
        shrink_total,
        shrink_total / shrink_steps.max(1) as u32
    );
    println!("shrink trees: {evaluations}");

    // One concrete structural shrink trace for the evidence document.
    let mut runner = step3_runner();
    let strategy = scripted_cases(
        step3_domain(TopologyClass::Holed, None, 40_000, 128),
        options.clone(),
    )
    .expect("holed strategy");
    let mut tree = strategy.new_tree(&mut runner).expect("holed case");
    let mut trace = Vec::new();
    trace.push(trace_line("initial", &tree.current()));
    while tree.simplify() {
        trace.push(trace_line("simplify", &tree.current()));
    }
    for line in trace {
        println!("{line}");
    }
}

/// One readable step of a structural shrink trace.
fn trace_line(label: &str, case: &ScriptedCase) -> String {
    let complexity = slicer_sdk::test_support::property_testing::scripted::recipe_complexity(&case.recipe)
        .expect("complexity");
    let (components, holes, area) = match &case.output.geometry {
        slicer_scad::ScadGeometry::Contours(polygons) => (
            polygons.len(),
            polygons.iter().map(|p| p.holes.len()).sum::<usize>(),
            scripted_support::forest_area_mm2(polygons),
        ),
        _ => (0, 0, 0.0),
    };
    format!(
        "trace {label}: complexity nodes={} vertices+facets={} sum_abs={} components={components} holes={holes} area_mm2={area:.6}",
        complexity.0, complexity.1, complexity.2
    )
}

/// Child half of the abort probe: build one certified case (which writes the
/// pre-execution attempt journal), announce readiness, then block until killed.
#[test]
fn scripted_attempt_abort_fixture() {
    // test-quality: the env gate is the child-mode selector for the abort probe
    if std::env::var_os("PNP_SCRIPTED_ABORT_FIXTURE").is_some() {
    let bounds = step3_bounds(16);
    let options = step3_options(&bounds, 40_000);
    let domain = ScriptedDomain {
        // exhaustive: pins the abort fixture's domain deliberately
        geometry: GeometryDomain {
            class: TopologyClass::Holed,
            min_holes: 0,
            min_components: 1,
            boundary: BoundaryRule::Strict,
            // exhaustive: pins the abort fixture's geometry capacities
            bounds: GeometryBounds {
                max_abs_units: 40_000,
                max_vertices: 32,
                max_holes: 2,
                max_components: 2,
                max_shrink_candidates: 8,
            },
        },
        solid_witness: None,
        bounds,
    };
    let strategy = scripted_cases(domain, options).expect("abort fixture strategy");
    // A real PropertyRun publishes the artifact directory to the scripted
    // generator (the attempt journal is thread-local, not environment-global).
    let _run = slicer_sdk::test_support::property_testing::PropertyRun::new(
        "D-SCAD-2D",
        "scripted_abort_and_timeout_keep_pre_execution_case",
    )
    .expect("abort fixture context");
    let phase = std::env::var("PNP_SCRIPTED_ABORT_PHASE").unwrap_or_else(|_| "generation".into());
    let mut runner = TestRunner::new(Config {
        cases: 1,
        rng_algorithm: RngAlgorithm::ChaCha,
        rng_seed: RngSeed::Fixed(0x5c4d_0010),
        failure_persistence: None,
        ..Config::default()
    });
    let mut tree = strategy.new_tree(&mut runner).expect("abort fixture case");
    if phase == "shrink" {
        // The first evaluation completed; the second is a shrink candidate and
        // pauses inside its own render window.
        let _ = tree.simplify();
    }
    // test-quality: blocks if the seam is ever removed, so the probe stays live
    std::thread::sleep(std::time::Duration::from_secs(120));
    }
}

#[test]
fn scripted_abort_and_timeout_keep_pre_execution_case() {
    let _guard = CONTEXT_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let dir = unique_dir("abort");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create the abort artifact directory");
    // One kill at each dangerous phase: before a callback and during a shrink
    // candidate render. Both must retain a prepared input set.
    let executable = std::env::current_exe().expect("current test executable");
    for (phase, pause) in [("generation", "1"), ("shrink", "2")] {
        let phase_dir = dir.join(phase);
        let _ = std::fs::remove_dir_all(&phase_dir);
        std::fs::create_dir_all(&phase_dir).expect("phase artifact directory");
        let context = scripted_support::context(
            &phase_dir,
            "D-SCAD-2D",
            "scripted_abort_and_timeout_keep_pre_execution_case",
            1,
            0x5c4d_0010,
        );
        run_abort_phase(&executable, &phase_dir, &context, phase, pause);
    }
}

fn run_abort_phase(
    executable: &std::path::Path,
    dir: &std::path::Path,
    context: &serde_json::Value,
    phase: &str,
    pause: &str,
) {
    let mut child = Command::new(executable)
        .args([
            "--exact",
            "scripted_attempt_abort_fixture",
            "--nocapture",
        ])
        .env("PNP_PROPERTY_CONTEXT", context.to_string())
        .env("PNP_SCRIPTED_ABORT_FIXTURE", "1")
        .env("PNP_SCRIPTED_ABORT_PHASE", phase)
        // Block at the exact window between a complete input set and the adapter
        // call: evaluation 1 is the initial tree, evaluation 2 a shrink candidate.
        .env("PNP_SCRIPTED_PAUSE_AFTER", pause)
        .env_remove("_PROPTEST_FORKFILE")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn the abort fixture");
    // Poll for the prepared attempt rather than a child announcement: the
    // child is paused *inside* its first evaluation, so it never prints one.
    let attempts_dir = dir.join("attempts");
    let mut prepared = None;
    for _ in 0..400 {
        if let Ok(entries) = std::fs::read_dir(&attempts_dir) {
            let mut found: Vec<_> = entries
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.path())
                .filter(|path| path.join("state.json").is_file())
                .collect();
            found.sort();
            if let Some(path) = found.pop() {
                let state = read_state(&path);
                if state["stage"] == "prepared" {
                    prepared = Some(path);
                    break;
                }
            }
        }
        // test-quality: waits for the child process to reach its prepared attempt
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    assert!(
        prepared.is_some(),
        "the child must write a prepared attempt before the kill"
    );
    // Kill the whole tree exactly as the executor does on a deadline.
    #[cfg(windows)]
    {
        let _ = Command::new("taskkill")
            .args(["/T", "/F", "/PID", &child.id().to_string()])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }
    let _ = child.kill();
    let status = child.wait().expect("join the killed child");
    assert!(
        !status.success(),
        "the abort fixture must not exit successfully"
    );

    // The killed process left a readable pre-execution input set behind.
    let attempts = dir.join("attempts");
    let mut attempt_dirs: Vec<_> = std::fs::read_dir(&attempts)
        .expect("attempt directory")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    attempt_dirs.sort();
    let attempt = attempt_dirs
        .pop()
        .expect("the killed child must leave an attempt directory");
    for name in [
        "recipe.json",
        "recipe-bounds.json",
        "source.scad",
        "options.json",
        "engine.json",
        "state.json",
    ] {
        assert!(
            attempt.join(name).is_file(),
            "a killed property child must retain {name}"
        );
    }
    let state = read_state(&attempt);
    assert_eq!(
        state["stage"], "prepared",
        "a kill before the adapter call must leave a prepared attempt, not a fabricated evaluated one: {state}"
    );
    assert_eq!(state["generator_version"], 1);
    // The retained case is still replayable: no seed regeneration, no defaults.
    // The replay renders natively, so it runs in a deadline-supervised child.
    supervised_replay(&attempt).unwrap_or_else(|error| panic!("{error}"));
}


/// Run one native-risk contract body in a supervised child of this binary.
///
/// The child owns every engine call (fixture creation, replay, tamper probes);
/// the parent only waits, kills the whole tree on a deadline, and reports the
/// child's own status. A failure inside the child is never a parent-thread
/// timeout approximation.
fn supervised_body(name: &str, fixture: &str) -> Result<(), String> {
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let mut child = Command::new(executable)
        .args(["--exact", fixture, "--nocapture"])
        .env("PNP_SCRIPTED_CONTRACT_FIXTURE", name)
        .env_remove("_PROPTEST_FORKFILE")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|error| error.to_string())?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let stdout_reader = std::thread::spawn(move || {
        use std::io::Read;
        let mut bytes = Vec::new();
        if let Some(mut pipe) = stdout {
            let _ = pipe.read_to_end(&mut bytes);
        }
        bytes
    });
    let stderr_reader = std::thread::spawn(move || {
        use std::io::Read;
        let mut bytes = Vec::new();
        if let Some(mut pipe) = stderr {
            let _ = pipe.read_to_end(&mut bytes);
        }
        bytes
    });
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(180);
    let mut timed_out = false;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.code(),
            Ok(None) if std::time::Instant::now() >= deadline => {
                timed_out = true;
                let _ = child.kill();
                break child.wait().ok().and_then(|status| status.code());
            }
            // test-quality: polls a supervised child with a real deadline
            Ok(None) => std::thread::sleep(std::time::Duration::from_millis(25)),
            Err(error) => return Err(error.to_string()),
        }
    };
    let out = stdout_reader.join().unwrap_or_default();
    let err = stderr_reader.join().unwrap_or_default();
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&out),
        String::from_utf8_lossy(&err)
    );
    if timed_out {
        return Err(format!("{name} exceeded its supervised deadline:\n{text}"));
    }
    if status != Some(0) {
        return Err(format!("{name} failed in its supervised child:\n{text}"));
    }
    Ok(())
}

/// Replay one retained attempt directory in a supervised child.
fn supervised_replay(directory: &std::path::Path) -> Result<(), String> {
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let mut child = Command::new(executable)
        .args(["--exact", "scripted_replay_dir_fixture", "--nocapture"])
        .env("PNP_SCRIPTED_REPLAY_DIR", directory)
        .env_remove("_PROPTEST_FORKFILE")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|error| error.to_string())?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let stdout_reader = std::thread::spawn(move || {
        use std::io::Read;
        let mut bytes = Vec::new();
        if let Some(mut pipe) = stdout {
            let _ = pipe.read_to_end(&mut bytes);
        }
        bytes
    });
    let stderr_reader = std::thread::spawn(move || {
        use std::io::Read;
        let mut bytes = Vec::new();
        if let Some(mut pipe) = stderr {
            let _ = pipe.read_to_end(&mut bytes);
        }
        bytes
    });
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
    let mut timed_out = false;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.code(),
            Ok(None) if std::time::Instant::now() >= deadline => {
                timed_out = true;
                let _ = child.kill();
                break child.wait().ok().and_then(|status| status.code());
            }
            // test-quality: polls a supervised replay child against its deadline
            Ok(None) => std::thread::sleep(std::time::Duration::from_millis(25)),
            Err(error) => return Err(error.to_string()),
        }
    };
    let out = stdout_reader.join().unwrap_or_default();
    let err = stderr_reader.join().unwrap_or_default();
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&out),
        String::from_utf8_lossy(&err)
    );
    if timed_out {
        return Err(format!("replay exceeded its supervised deadline:\n{text}"));
    }
    if status != Some(0) {
        return Err(format!("replay failed in its supervised child:\n{text}"));
    }
    Ok(())
}

#[test]
fn scripted_replay_dir_fixture() {
    let Ok(directory) = std::env::var("PNP_SCRIPTED_REPLAY_DIR") else {
        return;
    };
    match slicer_sdk::test_support::property_testing::scripted::replay_scripted_case(
        std::path::Path::new(&directory),
    ) {
        Ok(case) => {
            let source = emit_scad(&case.recipe, &case.recipe_bounds).expect("re-emit");
            assert_eq!(
                source, case.source,
                "the replayed case must reproduce its retained source"
            );
        }
        Err(error) => panic!("replay failed: {error}"),
    }
}

#[test]
fn scripted_artifacts_replay_and_concrete_fixture() {
    supervised_body("artifacts", "scripted_artifacts_replay_fixture")
        .unwrap_or_else(|error| panic!("{error}"));
}

#[test]
fn scripted_replay_refuses_changed_provenance_and_damaged_artifacts() {
    supervised_body("refusals", "scripted_replay_refusals_fixture")
        .unwrap_or_else(|error| panic!("{error}"));
}

#[test]
fn scripted_artifacts_replay_fixture() {
    if std::env::var("PNP_SCRIPTED_CONTRACT_FIXTURE").as_deref() == Ok("artifacts") {
        scripted_artifacts_replay_body();
    }
}

#[test]
fn scripted_replay_refusals_fixture() {
    if std::env::var("PNP_SCRIPTED_CONTRACT_FIXTURE").as_deref() == Ok("refusals") {
        scripted_replay_refusals_body();
    }
}
