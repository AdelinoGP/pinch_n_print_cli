#![allow(missing_docs)]

mod common;

use std::path::PathBuf;

use common::perimeter_harness::{
    align_coverage_measurements, run_pipeline_capturing_perimeters, AlignedCoverageMeasurement,
    PerimeterHarnessError, WallGenerator,
};
use slicer_ir::PerimeterIR;

struct CoverageSubject {
    name: &'static str,
    source_name: &'static str,
}

const SUBJECTS: &[CoverageSubject] = &[
    CoverageSubject {
        name: "tapered_wedge",
        source_name: "tapered_wedge.stl",
    },
    CoverageSubject {
        name: "narrow_strip_widening",
        source_name: "narrow_strip_widening.stl",
    },
    CoverageSubject {
        name: "max_bead_count_cap",
        source_name: "max_bead_count_cap.stl",
    },
    CoverageSubject {
        name: "complex_multi_feature",
        source_name: "complex_multi_feature.stl",
    },
    CoverageSubject {
        name: "cube_4color_arachne",
        // The checked-in coverage fixture is a painted 3MF despite the packet
        // table's stale .stl label.
        source_name: "cube_4color.3mf",
    },
];

// Pinned in design.md "Measured Coverage Baseline" — X-extent metric is too coarse to encode the D5 discriminator by itself, so the threshold is set to 0.99 to satisfy AC-4 (0.668 fails, 0.990 passes).
const COVERAGE_THRESHOLD: f64 = 0.99;

fn fixture_dir(subject: &CoverageSubject) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/perimeter_parity")
        .join(subject.name)
}

fn core_modules_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../modules/core-modules")
}

fn measure_subject(
    subject: &CoverageSubject,
) -> Result<
    (
        Vec<AlignedCoverageMeasurement>,
        Vec<AlignedCoverageMeasurement>,
    ),
    PerimeterHarnessError,
> {
    let directory = fixture_dir(subject);
    let source = directory.join(subject.source_name);
    let config = directory.join("config.json");
    let modules = [core_modules_dir()];

    let classic =
        run_pipeline_capturing_perimeters(&source, &config, &modules, WallGenerator::Classic)?;
    let arachne =
        run_pipeline_capturing_perimeters(&source, &config, &modules, WallGenerator::Arachne)?;
    let first = align_coverage_measurements(&classic, &arachne)?;

    let classic_repeat =
        run_pipeline_capturing_perimeters(&source, &config, &modules, WallGenerator::Classic)?;
    let arachne_repeat =
        run_pipeline_capturing_perimeters(&source, &config, &modules, WallGenerator::Arachne)?;
    let repeat = align_coverage_measurements(&classic_repeat, &arachne_repeat)?;
    Ok((first, repeat))
}

fn symmetric_coverage_ratio(measurement: &AlignedCoverageMeasurement) -> f32 {
    measurement
        .arachne_extent_mm
        .min(measurement.classic_extent_mm)
        / measurement
            .arachne_extent_mm
            .max(measurement.classic_extent_mm)
}

fn coverage_predicate(ratio: f64) -> Result<(), String> {
    if ratio < COVERAGE_THRESHOLD {
        Err(format!(
            "coverage ratio {ratio:.3} < threshold {COVERAGE_THRESHOLD:.2}"
        ))
    } else {
        Ok(())
    }
}

fn assert_capture_is_structural(fixture: &str, generator: WallGenerator, capture: &[PerimeterIR]) {
    let generator_name = match generator {
        WallGenerator::Classic => "classic",
        WallGenerator::Arachne => "arachne",
    };
    assert!(
        !capture.is_empty(),
        "fixture {fixture}, generator {generator_name}: capture must be nonempty"
    );
    let mut wall_count = 0;
    for perimeter in capture {
        for region in &perimeter.regions {
            for wall in &region.walls {
                wall_count += 1;
                assert!(
                    wall.path.points.len() >= 2,
                    "fixture {fixture}, generator {generator_name}, layer {}: wall must have at least two points",
                    perimeter.global_layer_index
                );
                for point in &wall.path.points {
                    assert!(
                        point.x.is_finite()
                            && point.y.is_finite()
                            && point.z.is_finite()
                            && point.width.is_finite(),
                        "fixture {fixture}, generator {generator_name}, layer {}: coordinates and width must be finite",
                        perimeter.global_layer_index
                    );
                }
            }
        }
    }
    assert!(
        wall_count > 0,
        "fixture {fixture}, generator {generator_name}: capture must contain a wall"
    );
}

#[test]
fn tapered_wedge_parity_is_structural() {
    let subject = SUBJECTS
        .iter()
        .find(|subject| subject.name == "tapered_wedge")
        .expect("tapered_wedge must be in the structural source corpus");
    let directory = fixture_dir(subject);
    let source = directory.join(subject.source_name);
    let config = directory.join("config.json");
    let modules = [core_modules_dir()];
    let classic =
        run_pipeline_capturing_perimeters(&source, &config, &modules, WallGenerator::Classic)
            .unwrap_or_else(|error| {
                panic!("fixture {} classic capture failed: {error}", subject.name)
            });
    let arachne =
        run_pipeline_capturing_perimeters(&source, &config, &modules, WallGenerator::Arachne)
            .unwrap_or_else(|error| {
                panic!("fixture {} Arachne capture failed: {error}", subject.name)
            });
    assert_capture_is_structural(subject.name, WallGenerator::Classic, &classic);
    assert_capture_is_structural(subject.name, WallGenerator::Arachne, &arachne);

    let aligned = align_coverage_measurements(&classic, &arachne).unwrap_or_else(|error| {
        panic!("fixture {} paired alignment failed: {error}", subject.name)
    });
    assert!(
        !aligned.is_empty(),
        "fixture {}: paired capture must have an aligned layer",
        subject.name
    );
    for measurement in &aligned {
        let ratio = symmetric_coverage_ratio(measurement) as f64;
        if ratio < COVERAGE_THRESHOLD {
            panic!(
                "fixture {} at Z {:.6} mm: Arachne extent {:.6} mm, Classic extent {:.6} mm, coverage ratio {:.6} < threshold {:.2}",
                subject.name,
                measurement.z_plane_mm,
                measurement.arachne_extent_mm,
                measurement.classic_extent_mm,
                ratio,
                COVERAGE_THRESHOLD
            );
        }
    }
}

#[test]
fn coverage_subjects_repeat_and_report_ratios() {
    let mut rows = Vec::with_capacity(SUBJECTS.len());
    for subject in SUBJECTS {
        let (first, repeat) = measure_subject(subject)
            .unwrap_or_else(|error| panic!("{} measurement failed: {error}", subject.name));
        let selected = first
            .iter()
            .min_by(|left, right| left.ratio.total_cmp(&right.ratio))
            .expect("subject must have at least one aligned layer");
        let repeated = repeat
            .iter()
            .find(|measurement| measurement.global_layer_index == selected.global_layer_index)
            .expect("repeat must contain the selected global layer");
        let ratio = symmetric_coverage_ratio(selected);
        let repeat_ratio = symmetric_coverage_ratio(repeated);
        let repeat_delta = (ratio - repeat_ratio).abs();
        rows.push((subject.name, selected.clone(), ratio, repeat_delta));
    }

    let observed_min = rows
        .iter()
        .map(|(_, _, ratio, _)| *ratio)
        .fold(f32::INFINITY, f32::min);
    let margin = rows
        .iter()
        .map(|(_, _, _, repeat_delta)| *repeat_delta)
        .fold(0.0, f32::max);
    let threshold = observed_min - margin;

    println!("| Fixture | Arachne X extent (mm) | Classic X extent (mm) | Coverage ratio | Z plane (mm) | Repeat delta | Notes |");
    println!("| --- | ---: | ---: | ---: | ---: | ---: | --- |");
    for (name, measurement, ratio, repeat_delta) in &rows {
        println!(
            "| `{name}` | {:.6} | {:.6} | {:.6} | {:.6} | {:.6} | minimum aligned ratio at global layer {} |",
            measurement.arachne_extent_mm,
            measurement.classic_extent_mm,
            ratio,
            measurement.z_plane_mm,
            repeat_delta,
            measurement.global_layer_index,
        );
    }
    println!("observed_min={observed_min:.6}, margin={margin:.6}, threshold={threshold:.6}");

    assert!(
        margin <= 0.02,
        "repeatability margin exceeded 0.02: {margin}"
    );
    assert!(
        threshold > 0.668,
        "derived threshold admits broken D5 ratio: {threshold}"
    );
}

#[test]
fn coverage_threshold_rejects_d5_broken_ratio() {
    let error = coverage_predicate(0.668).expect_err("broken D5 ratio must be rejected");
    assert!(error.contains("0.668"), "diagnostic omitted ratio: {error}");
    assert!(
        error.contains("0.99"),
        "diagnostic omitted threshold: {error}"
    );
}

#[test]
fn coverage_threshold_accepts_d5_fixed_ratio() {
    coverage_predicate(0.990).expect("fixed D5 ratio must be admitted");
}

#[test]
fn coverage_invariant_rejects_synthetic_d5_regression() {
    let error = coverage_predicate(0.668).expect_err("synthetic D5 regression must be rejected");
    assert!(
        error.contains("coverage ratio 0.668") && error.contains("threshold 0.99"),
        "diagnostic must name ratio and threshold: {error}"
    );
}

#[test]
fn arachne_coverage_floor_over_source_corpus() {
    for subject in SUBJECTS {
        let (first, _) = measure_subject(subject)
            .unwrap_or_else(|error| panic!("{} measurement failed: {error}", subject.name));
        let selected = first
            .iter()
            .min_by(|left, right| left.ratio.total_cmp(&right.ratio))
            .expect("subject must have at least one aligned layer");
        let ratio = symmetric_coverage_ratio(selected) as f64;
        if let Err(error) = coverage_predicate(ratio) {
            panic!(
                "{error} for fixture {} at Z {:.6} mm (Arachne X {:.6} mm, Classic X {:.6} mm)",
                subject.name,
                selected.z_plane_mm,
                selected.arachne_extent_mm,
                selected.classic_extent_mm,
            );
        }
    }
}

#[test]
fn wall_generator_arg_overrides_config_arachne_config_to_classic_run() {
    // Regression test for the precedence rule: the test arg is the single source of truth for
    // the wall_generator selector. A previous harness version panicked on config-vs-arg
    // divergence, breaking paired coverage with Classic and Arachne on the same input. This
    // guards the rule by using an arachne-configured fixture with a Classic test arg.
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/perimeter_parity/tapered_wedge");
    let mesh_path = fixture.join("tapered_wedge.stl");
    let config_path = fixture.join("config.json");
    let module_dirs = [core_modules_dir()];
    let result = run_pipeline_capturing_perimeters(
        &mesh_path,
        &config_path,
        &module_dirs,
        WallGenerator::Classic,
    );

    assert!(result.is_ok(), "Classic capture must succeed: {result:?}");
    let perimeters = result.expect("Classic capture must return perimeters");
    assert!(!perimeters.is_empty(), "Classic capture must be non-empty");
}
