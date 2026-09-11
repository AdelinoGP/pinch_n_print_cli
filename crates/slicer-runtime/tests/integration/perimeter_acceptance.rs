const SAMPLES_PER_CELL: usize = 4;
const THREADS: u32 = 12;
const WARMUP_PER_CELL: usize = 1;

#[derive(Clone, Debug)]
struct ConfigProvenance {
    path: String,
    config_sha256: String,
}

fn synthetic_config_provenance() -> ConfigProvenance {
    ConfigProvenance {
        path: "synthetic://perimeter-acceptance/config.json".to_owned(),
        config_sha256: String::new(),
    }
}

#[derive(Clone, Debug)]
struct SampleRow {
    cpu_seconds: f64,
    wall_seconds: f64,
    generator_marker: String,
    status: String,
    degraded: i64,
    nonfatal_errors: i64,
    fatal_errors: i64,
    config_provenance: ConfigProvenance,
}

#[derive(Clone, Debug)]
struct VariantSummary {
    cpu_samples: Vec<f64>,
    wall_samples: Vec<f64>,
    rows: Vec<SampleRow>,
}

#[derive(Clone, Debug)]
struct GeneratorMarkerSummary {
    baseline: String,
    candidate: String,
}

#[derive(Clone, Debug)]
struct CellSummary {
    workload: String,
    generator: String,
    expected_generator: String,
    exactness_passed: bool,
    cpu_separated: bool,
    wall_separated: bool,
    baseline: VariantSummary,
    candidate: VariantSummary,
    status: String,
    status_baseline: String,
    status_candidate: String,
    config_provenance: ConfigProvenance,
    cpu_ratio: Option<f64>,
    wall_ratio: Option<f64>,
    generator_marker: GeneratorMarkerSummary,
    generator_marker_match: bool,
    degraded_baseline: i64,
    degraded_candidate: i64,
    nonfatal_errors_baseline: i64,
    nonfatal_errors_candidate: i64,
    fatal_errors_baseline: i64,
    fatal_errors_candidate: i64,
    decision: String,
}

#[derive(Debug)]
struct AcceptanceSummary {
    status: String,
    automatic_commit: bool,
    threads: u32,
    samples_per_cell: usize,
    warmup_per_cell: usize,
    config_provenance: Vec<ConfigProvenance>,
    cells: Vec<CellSummary>,
}

fn sample_row(
    cpu_seconds: f64,
    wall_seconds: f64,
    generator_marker: &str,
    degraded: i64,
    nonfatal_errors: i64,
    fatal_errors: i64,
) -> SampleRow {
    sample_row_with_status(
        cpu_seconds,
        wall_seconds,
        generator_marker,
        "ok",
        degraded,
        nonfatal_errors,
        fatal_errors,
        synthetic_config_provenance(),
    )
}

fn sample_row_with_status(
    cpu_seconds: f64,
    wall_seconds: f64,
    generator_marker: &str,
    status: &str,
    degraded: i64,
    nonfatal_errors: i64,
    fatal_errors: i64,
    config_provenance: ConfigProvenance,
) -> SampleRow {
    SampleRow {
        cpu_seconds,
        wall_seconds,
        generator_marker: generator_marker.to_owned(),
        status: status.to_owned(),
        degraded,
        nonfatal_errors,
        fatal_errors,
        config_provenance,
    }
}

fn rows(
    cpu_seconds: [f64; SAMPLES_PER_CELL],
    wall_seconds: [f64; SAMPLES_PER_CELL],
    generator_marker: &str,
) -> Vec<SampleRow> {
    cpu_seconds
        .into_iter()
        .zip(wall_seconds)
        .map(|(cpu, wall)| sample_row(cpu, wall, generator_marker, 0, 0, 0))
        .collect()
}

fn rows_with_counts(
    cpu_seconds: [f64; SAMPLES_PER_CELL],
    wall_seconds: [f64; SAMPLES_PER_CELL],
    generator_marker: &str,
    degraded: [i64; SAMPLES_PER_CELL],
    nonfatal_errors: [i64; SAMPLES_PER_CELL],
    fatal_errors: [i64; SAMPLES_PER_CELL],
) -> Vec<SampleRow> {
    cpu_seconds
        .into_iter()
        .zip(wall_seconds)
        .zip(degraded)
        .zip(nonfatal_errors)
        .zip(fatal_errors)
        .map(
            |((((cpu, wall), degraded), nonfatal_errors), fatal_errors)| {
                sample_row(
                    cpu,
                    wall,
                    generator_marker,
                    degraded,
                    nonfatal_errors,
                    fatal_errors,
                )
            },
        )
        .collect()
}

fn rows_with_status(
    cpu_seconds: [f64; SAMPLES_PER_CELL],
    wall_seconds: [f64; SAMPLES_PER_CELL],
    generator_marker: &str,
    status: &str,
    config_provenance: &ConfigProvenance,
) -> Vec<SampleRow> {
    cpu_seconds
        .into_iter()
        .zip(wall_seconds)
        .map(|(cpu, wall)| {
            sample_row_with_status(
                cpu,
                wall,
                generator_marker,
                status,
                0,
                0,
                0,
                config_provenance.clone(),
            )
        })
        .collect()
}

fn median(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }

    let mut sorted = values.to_vec();
    sorted.sort_by(|left, right| left.total_cmp(right));
    let middle = sorted.len() / 2;
    if sorted.len().is_multiple_of(2) {
        Some((sorted[middle - 1] + sorted[middle]) / 2.0)
    } else {
        Some(sorted[middle])
    }
}

fn ratio(baseline: Option<f64>, candidate: Option<f64>) -> Option<f64> {
    match (baseline, candidate) {
        (Some(baseline), Some(candidate)) if baseline != 0.0 => Some(candidate / baseline),
        _ => None,
    }
}

fn max_count(rows: &[SampleRow], property: fn(&SampleRow) -> i64) -> i64 {
    rows.iter()
        .fold(0, |maximum, row| maximum.max(property(row)))
}

fn sum_count(rows: &[SampleRow], property: fn(&SampleRow) -> i64) -> i64 {
    rows.iter().map(property).sum()
}

fn markers_match(rows: &[SampleRow], expected: &str) -> bool {
    !rows.is_empty()
        && rows.iter().all(|row| {
            !row.generator_marker.trim().is_empty()
                && row.generator_marker.eq_ignore_ascii_case(expected)
        })
}

fn range_separated(
    baseline_rows: &[SampleRow],
    candidate_rows: &[SampleRow],
    property: fn(&SampleRow) -> f64,
) -> bool {
    if baseline_rows.len() != SAMPLES_PER_CELL || candidate_rows.len() != SAMPLES_PER_CELL {
        return false;
    }

    let baseline_min = baseline_rows
        .iter()
        .map(property)
        .fold(f64::INFINITY, f64::min);
    let candidate_max = candidate_rows
        .iter()
        .map(property)
        .fold(f64::NEG_INFINITY, f64::max);
    candidate_max < baseline_min
}

fn ranges_overlap(
    baseline_rows: &[SampleRow],
    candidate_rows: &[SampleRow],
    property: fn(&SampleRow) -> f64,
) -> bool {
    if baseline_rows.is_empty() || candidate_rows.is_empty() {
        return false;
    }

    let baseline_values: Vec<f64> = baseline_rows.iter().map(property).collect();
    let candidate_values: Vec<f64> = candidate_rows.iter().map(property).collect();
    let baseline_min = baseline_values
        .iter()
        .copied()
        .fold(f64::INFINITY, f64::min);
    let baseline_max = baseline_values
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    let candidate_min = candidate_values
        .iter()
        .copied()
        .fold(f64::INFINITY, f64::min);
    let candidate_max = candidate_values
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);

    !(candidate_max < baseline_min || baseline_max < candidate_min)
}

fn get_cell_decision(cell: &CellSummary) -> &'static str {
    if !cell.exactness_passed || !cell.generator_marker_match {
        return "DROP";
    }
    // Empty/empty is an identical status; empty/set and differing non-empty
    // statuses are generator disagreements and can never be retained.
    if cell.status_baseline != cell.status_candidate {
        return "DROP";
    }
    if cell.degraded_baseline != cell.degraded_candidate
        || cell.nonfatal_errors_baseline != cell.nonfatal_errors_candidate
    {
        return "DROP";
    }
    if cell.fatal_errors_baseline > 0 || cell.fatal_errors_candidate > 0 {
        return "DROP";
    }
    if cell.cpu_separated && cell.wall_separated {
        return "KEEP";
    }
    if ranges_overlap(&cell.baseline.rows, &cell.candidate.rows, |row| {
        row.cpu_seconds
    }) || ranges_overlap(&cell.baseline.rows, &cell.candidate.rows, |row| {
        row.wall_seconds
    }) {
        return "inconclusive";
    }
    "DROP"
}

fn cell_summary(
    workload: &str,
    generator: &str,
    expected_generator: &str,
    exactness_passed: bool,
    baseline_rows: Vec<SampleRow>,
    candidate_rows: Vec<SampleRow>,
    baseline_marker: &str,
    candidate_marker: &str,
) -> CellSummary {
    cell_summary_with_provenance(
        workload,
        generator,
        expected_generator,
        exactness_passed,
        baseline_rows,
        candidate_rows,
        baseline_marker,
        candidate_marker,
        synthetic_config_provenance(),
    )
}

fn cell_summary_with_provenance(
    workload: &str,
    generator: &str,
    expected_generator: &str,
    exactness_passed: bool,
    baseline_rows: Vec<SampleRow>,
    candidate_rows: Vec<SampleRow>,
    baseline_marker: &str,
    candidate_marker: &str,
    fallback_config_provenance: ConfigProvenance,
) -> CellSummary {
    let baseline_marker = if baseline_marker.trim().is_empty() {
        baseline_rows
            .first()
            .map(|row| row.generator_marker.clone())
            .unwrap_or_default()
    } else {
        baseline_marker.to_owned()
    };
    let candidate_marker = if candidate_marker.trim().is_empty() {
        candidate_rows
            .first()
            .map(|row| row.generator_marker.clone())
            .unwrap_or_default()
    } else {
        candidate_marker.to_owned()
    };
    let status_baseline = baseline_rows
        .first()
        .map(|row| row.status.clone())
        .unwrap_or_default();
    let status_candidate = candidate_rows
        .first()
        .map(|row| row.status.clone())
        .unwrap_or_default();
    let status = if !status_candidate.trim().is_empty() {
        status_candidate.clone()
    } else {
        status_baseline.clone()
    };
    let config_provenance = baseline_rows
        .first()
        .map(|row| row.config_provenance.clone())
        .or_else(|| {
            candidate_rows
                .first()
                .map(|row| row.config_provenance.clone())
        })
        .unwrap_or(fallback_config_provenance);

    let baseline_cpu_samples: Vec<f64> = baseline_rows.iter().map(|row| row.cpu_seconds).collect();
    let candidate_cpu_samples: Vec<f64> =
        candidate_rows.iter().map(|row| row.cpu_seconds).collect();
    let baseline_wall_samples: Vec<f64> =
        baseline_rows.iter().map(|row| row.wall_seconds).collect();
    let candidate_wall_samples: Vec<f64> =
        candidate_rows.iter().map(|row| row.wall_seconds).collect();
    let baseline_cpu = median(&baseline_cpu_samples);
    let candidate_cpu = median(&candidate_cpu_samples);
    let baseline_wall = median(&baseline_wall_samples);
    let candidate_wall = median(&candidate_wall_samples);
    let cpu_separated = range_separated(&baseline_rows, &candidate_rows, |row| row.cpu_seconds);
    let wall_separated = range_separated(&baseline_rows, &candidate_rows, |row| row.wall_seconds);
    let generator_marker_match = markers_match(&baseline_rows, expected_generator)
        && markers_match(&candidate_rows, expected_generator)
        && baseline_marker.eq_ignore_ascii_case(&candidate_marker);

    let mut cell = CellSummary {
        workload: workload.to_owned(),
        generator: generator.to_owned(),
        expected_generator: expected_generator.to_owned(),
        exactness_passed,
        cpu_separated,
        wall_separated,
        baseline: VariantSummary {
            cpu_samples: baseline_cpu_samples,
            wall_samples: baseline_wall_samples,
            rows: baseline_rows,
        },
        candidate: VariantSummary {
            cpu_samples: candidate_cpu_samples,
            wall_samples: candidate_wall_samples,
            rows: candidate_rows,
        },
        status,
        status_baseline,
        status_candidate,
        config_provenance,
        cpu_ratio: ratio(baseline_cpu, candidate_cpu),
        wall_ratio: ratio(baseline_wall, candidate_wall),
        generator_marker: GeneratorMarkerSummary {
            baseline: baseline_marker.clone(),
            candidate: candidate_marker.clone(),
        },
        generator_marker_match,
        degraded_baseline: 0,
        degraded_candidate: 0,
        nonfatal_errors_baseline: 0,
        nonfatal_errors_candidate: 0,
        fatal_errors_baseline: 0,
        fatal_errors_candidate: 0,
        decision: String::new(),
    };

    cell.degraded_baseline = max_count(&cell.baseline.rows, |row| row.degraded);
    cell.degraded_candidate = max_count(&cell.candidate.rows, |row| row.degraded);
    cell.nonfatal_errors_baseline = sum_count(&cell.baseline.rows, |row| row.nonfatal_errors);
    cell.nonfatal_errors_candidate = sum_count(&cell.candidate.rows, |row| row.nonfatal_errors);
    cell.fatal_errors_baseline = sum_count(&cell.baseline.rows, |row| row.fatal_errors);
    cell.fatal_errors_candidate = sum_count(&cell.candidate.rows, |row| row.fatal_errors);
    cell.decision = get_cell_decision(&cell).to_owned();
    cell
}

fn get_overall_decision(cells: &[CellSummary]) -> &'static str {
    if cells.iter().any(|cell| cell.decision == "DROP") {
        return "DROP";
    }
    if cells.iter().any(|cell| cell.decision == "inconclusive") {
        return "inconclusive";
    }
    if !cells.is_empty() && cells.iter().all(|cell| cell.decision == "KEEP") {
        return "KEEP";
    }
    "DROP"
}

fn acceptance_summary(cells: Vec<CellSummary>) -> AcceptanceSummary {
    let config_provenance = cells
        .iter()
        .map(|cell| cell.config_provenance.clone())
        .collect();
    AcceptanceSummary {
        status: get_overall_decision(&cells).to_owned(),
        automatic_commit: false,
        threads: THREADS,
        samples_per_cell: SAMPLES_PER_CELL,
        warmup_per_cell: WARMUP_PER_CELL,
        config_provenance,
        cells,
    }
}

fn separated_cell(workload: &str, generator: &str) -> CellSummary {
    cell_summary(
        workload,
        generator,
        generator,
        true,
        rows(
            [10.0, 11.0, 12.0, 13.0],
            [20.0, 21.0, 22.0, 23.0],
            generator,
        ),
        rows([1.0, 2.0, 3.0, 4.0], [11.0, 12.0, 13.0, 14.0], generator),
        generator,
        generator,
    )
}

pub(crate) fn overlap_is_inconclusive_and_never_keep() {
    let overlap_only_wall = cell_summary(
        "overlap-only-wall",
        "classic",
        "classic",
        true,
        rows(
            [10.0, 11.0, 12.0, 13.0],
            [20.0, 21.0, 22.0, 23.0],
            "classic",
        ),
        rows([1.0, 2.0, 3.0, 4.0], [19.0, 21.0, 22.0, 24.0], "classic"),
        "classic",
        "classic",
    );
    assert!(overlap_only_wall.cpu_separated);
    assert!(!overlap_only_wall.wall_separated);
    assert_eq!(overlap_only_wall.decision, "inconclusive");

    let overlap_only_cpu = cell_summary(
        "overlap-only-cpu",
        "classic",
        "classic",
        true,
        rows(
            [10.0, 11.0, 12.0, 13.0],
            [20.0, 21.0, 22.0, 23.0],
            "classic",
        ),
        rows([9.0, 11.0, 12.0, 13.0], [1.0, 2.0, 3.0, 4.0], "classic"),
        "classic",
        "classic",
    );
    assert!(!overlap_only_cpu.cpu_separated);
    assert!(overlap_only_cpu.wall_separated);
    assert_eq!(overlap_only_cpu.decision, "inconclusive");

    let exactness_drop = cell_summary(
        "exactness-drop",
        "classic",
        "classic",
        false,
        rows(
            [10.0, 11.0, 12.0, 13.0],
            [20.0, 21.0, 22.0, 23.0],
            "classic",
        ),
        rows([1.0, 2.0, 3.0, 4.0], [11.0, 12.0, 13.0, 14.0], "classic"),
        "classic",
        "classic",
    );
    assert!(!exactness_drop.exactness_passed);
    assert!(exactness_drop.cpu_separated && exactness_drop.wall_separated);
    assert_eq!(exactness_drop.decision, "DROP");

    let marker_disagreement = cell_summary(
        "marker-disagreement",
        "classic",
        "classic",
        true,
        rows(
            [10.0, 11.0, 12.0, 13.0],
            [20.0, 21.0, 22.0, 23.0],
            "classic",
        ),
        rows([1.0, 2.0, 3.0, 4.0], [11.0, 12.0, 13.0, 14.0], "classic"),
        "classic",
        "arachne",
    );
    assert_eq!(marker_disagreement.workload, "marker-disagreement");
    assert_eq!(marker_disagreement.generator, "classic");
    assert_eq!(marker_disagreement.expected_generator, "classic");
    assert_eq!(
        marker_disagreement.baseline.cpu_samples.len(),
        SAMPLES_PER_CELL
    );
    assert_eq!(
        marker_disagreement.baseline.wall_samples.len(),
        SAMPLES_PER_CELL
    );
    assert_eq!(
        marker_disagreement.candidate.cpu_samples.len(),
        SAMPLES_PER_CELL
    );
    assert_eq!(
        marker_disagreement.candidate.wall_samples.len(),
        SAMPLES_PER_CELL
    );
    assert!(marker_disagreement.cpu_ratio.is_some());
    assert!(marker_disagreement.wall_ratio.is_some());
    assert_eq!(marker_disagreement.generator_marker.baseline, "classic");
    assert_eq!(marker_disagreement.generator_marker.candidate, "arachne");
    assert!(!marker_disagreement.generator_marker_match);
    assert_eq!(marker_disagreement.decision, "DROP");

    let degraded_count_changed = cell_summary(
        "degraded-count-changed",
        "classic",
        "classic",
        true,
        rows_with_counts(
            [10.0, 11.0, 12.0, 13.0],
            [20.0, 21.0, 22.0, 23.0],
            "classic",
            [1, 1, 1, 1],
            [2, 2, 2, 2],
            [0, 0, 0, 0],
        ),
        rows_with_counts(
            [1.0, 2.0, 3.0, 4.0],
            [11.0, 12.0, 13.0, 14.0],
            "classic",
            [0, 0, 0, 0],
            [2, 2, 2, 2],
            [0, 0, 0, 0],
        ),
        "classic",
        "classic",
    );
    assert_eq!(degraded_count_changed.degraded_baseline, 1);
    assert_eq!(degraded_count_changed.degraded_candidate, 0);
    assert_eq!(
        degraded_count_changed.nonfatal_errors_baseline,
        degraded_count_changed.nonfatal_errors_candidate
    );
    assert_eq!(degraded_count_changed.decision, "DROP");

    let nonfatal_count_changed = cell_summary(
        "nonfatal-count-changed",
        "classic",
        "classic",
        true,
        rows_with_counts(
            [10.0, 11.0, 12.0, 13.0],
            [20.0, 21.0, 22.0, 23.0],
            "classic",
            [0, 0, 0, 0],
            [2, 2, 2, 2],
            [0, 0, 0, 0],
        ),
        rows_with_counts(
            [1.0, 2.0, 3.0, 4.0],
            [11.0, 12.0, 13.0, 14.0],
            "classic",
            [0, 0, 0, 0],
            [3, 3, 3, 3],
            [0, 0, 0, 0],
        ),
        "classic",
        "classic",
    );
    assert_eq!(nonfatal_count_changed.nonfatal_errors_baseline, 8);
    assert_eq!(nonfatal_count_changed.nonfatal_errors_candidate, 12);
    assert_eq!(nonfatal_count_changed.decision, "DROP");

    let fatal_count_changed = cell_summary(
        "fatal-count-changed",
        "classic",
        "classic",
        true,
        rows_with_counts(
            [10.0, 11.0, 12.0, 13.0],
            [20.0, 21.0, 22.0, 23.0],
            "classic",
            [0, 0, 0, 0],
            [0, 0, 0, 0],
            [0, 0, 0, 0],
        ),
        rows_with_counts(
            [1.0, 2.0, 3.0, 4.0],
            [11.0, 12.0, 13.0, 14.0],
            "classic",
            [0, 0, 0, 0],
            [0, 0, 0, 0],
            [1, 0, 0, 0],
        ),
        "classic",
        "classic",
    );
    assert_eq!(fatal_count_changed.fatal_errors_baseline, 0);
    assert_eq!(fatal_count_changed.fatal_errors_candidate, 1);
    assert_eq!(fatal_count_changed.decision, "DROP");

    let degraded_count_identical = cell_summary(
        "degraded-count-identical",
        "classic",
        "classic",
        true,
        rows_with_counts(
            [10.0, 11.0, 12.0, 13.0],
            [20.0, 21.0, 22.0, 23.0],
            "classic",
            [1, 1, 1, 1],
            [2, 2, 2, 2],
            [0, 0, 0, 0],
        ),
        rows_with_counts(
            [1.0, 2.0, 3.0, 4.0],
            [11.0, 12.0, 13.0, 14.0],
            "classic",
            [1, 1, 1, 1],
            [2, 2, 2, 2],
            [0, 0, 0, 0],
        ),
        "classic",
        "classic",
    );
    assert_eq!(degraded_count_identical.degraded_baseline, 1);
    assert_eq!(degraded_count_identical.degraded_candidate, 1);
    assert_eq!(degraded_count_identical.nonfatal_errors_baseline, 8);
    assert_eq!(degraded_count_identical.nonfatal_errors_candidate, 8);
    assert_eq!(degraded_count_identical.status_baseline, "ok");
    assert_eq!(degraded_count_identical.status_candidate, "ok");
    assert_eq!(degraded_count_identical.status, "ok");
    assert_eq!(degraded_count_identical.decision, "KEEP");

    let config_provenance = synthetic_config_provenance();
    let status_changed = cell_summary_with_provenance(
        "status-changed",
        "classic",
        "classic",
        true,
        rows_with_status(
            [10.0, 11.0, 12.0, 13.0],
            [20.0, 21.0, 22.0, 23.0],
            "classic",
            "ok",
            &config_provenance,
        ),
        rows_with_status(
            [1.0, 2.0, 3.0, 4.0],
            [11.0, 12.0, 13.0, 14.0],
            "classic",
            "degraded",
            &config_provenance,
        ),
        "classic",
        "classic",
        config_provenance.clone(),
    );
    assert!(status_changed.cpu_separated && status_changed.wall_separated);
    assert_eq!(status_changed.status_baseline, "ok");
    assert_eq!(status_changed.status_candidate, "degraded");
    assert_eq!(status_changed.status, "degraded");
    assert_eq!(status_changed.decision, "DROP");

    let empty_status_discrepancy = cell_summary_with_provenance(
        "empty-status-discrepancy",
        "classic",
        "classic",
        true,
        rows_with_status(
            [10.0, 11.0, 12.0, 13.0],
            [20.0, 21.0, 22.0, 23.0],
            "classic",
            "",
            &config_provenance,
        ),
        rows_with_status(
            [1.0, 2.0, 3.0, 4.0],
            [11.0, 12.0, 13.0, 14.0],
            "classic",
            "ok",
            &config_provenance,
        ),
        "classic",
        "classic",
        config_provenance,
    );
    assert!(empty_status_discrepancy.cpu_separated && empty_status_discrepancy.wall_separated);
    assert!(empty_status_discrepancy.status_baseline.is_empty());
    assert_eq!(empty_status_discrepancy.status_candidate, "ok");
    assert_eq!(empty_status_discrepancy.decision, "DROP");

    let six_cells = vec![
        separated_cell("supports-off-benchy", "classic"),
        separated_cell("supports-off-benchy", "arachne"),
        separated_cell("tree-support-benchy", "classic"),
        separated_cell("tree-support-benchy", "arachne"),
        separated_cell("original-tree-support-base", "classic"),
        separated_cell("original-tree-support-base", "arachne"),
    ];
    assert_eq!(six_cells.len(), 6);
    assert!(six_cells
        .iter()
        .all(|cell| { cell.cpu_separated && cell.wall_separated && cell.decision == "KEEP" }));

    let summary = acceptance_summary(six_cells);
    assert_eq!(summary.status, "KEEP");
    assert!(!summary.automatic_commit);
    assert_eq!(summary.threads, THREADS);
    assert_eq!(summary.samples_per_cell, SAMPLES_PER_CELL);
    assert_eq!(summary.warmup_per_cell, WARMUP_PER_CELL);
    assert_eq!(summary.config_provenance.len(), 6);
    assert!(summary.config_provenance.iter().all(|provenance| {
        provenance.path.starts_with("synthetic://") && provenance.config_sha256.is_empty()
    }));
    assert_eq!(summary.cells.len(), 6);
}
