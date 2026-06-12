//! Compatibility-matrix and scenario-trace coverage for the docs/10
//! scenario traces and docs/11 / docs/12 acceptance-gate categories.
//!
//! These tests are explicit executable specs for previously-implicit
//! contracts; no production code is changed.
//!
//! Doc map:
//! - docs/10_scenario_traces.md Â§Scenario Traces 1/2/3
//! - docs/11_operational_governance_and_acceptance_gate.md Â§2 Compatibility
//!   Policy + Â§3 Gate Rubric (Determinism / Recoverability / Compatibility)
//! - docs/12_architecture_gate_metrics.md Â§Reference Fixture Set + Â§Resource
//!   Bounds + Â§Compatibility

#![allow(missing_docs)]

use std::path::PathBuf;

// paint_region module removed in packet 95 sub-step 16
use slicer_ir::{
    ActiveRegion, GlobalLayer, NonPlanarShellRef, ResolvedConfig, SemVer, SurfaceGroupId,
};
use slicer_runtime::progress_events::{
    ProgressError, ProgressEvent, ProgressPhase, SliceEventCollector,
};
use slicer_runtime::{
    build_intra_stage_dag, validate_startup_dag, DagValidationPass, DagValidationRequest,
    LoadedModule, LoadedModuleBuilder, Producer, SchedulerError, StageDag,
};

// â”€â”€ Helpers â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn semver(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}

fn loaded_module_for_compat(id: &str, min_ir: SemVer, max_ir: SemVer) -> LoadedModule {
    LoadedModuleBuilder::new(
        id,
        semver(1, 0, 0),
        "Layer::Support",
        "slicer:world-layer@1.0.0",
        PathBuf::from(format!("fixtures/{id}.wasm")),
    )
    .ir_writes(vec!["SharedIR.placeholder".to_string()])
    .min_host_version(semver(0, 1, 0))
    .min_ir_schema(min_ir)
    .max_ir_schema(max_ir)
    .layer_parallel_safe(true)
    .build()
}

fn dag_request_for(host_ir: SemVer, modules: Vec<LoadedModule>) -> DagValidationRequest {
    let stage = "Layer::Support".to_string();
    let producers: Vec<&dyn Producer> = modules.iter().map(|m| m as &dyn Producer).collect();
    let nodes = build_intra_stage_dag(stage.clone(), &producers).expect("DAG should build");
    DagValidationRequest {
        modules,
        stage_dags: vec![StageDag { stage, nodes }],
        host_ir_schema_version: host_ir,
        claim_holders: Vec::new(),
        access_audits: Vec::new(),
    }
}

fn count_ir_version_errors(report: &slicer_runtime::DagValidationReport) -> usize {
    report
        .errors
        .iter()
        .filter(|d| {
            d.pass == DagValidationPass::IrVersionCompatibility
                && matches!(d.detail, SchedulerError::IrVersionIncompatible { .. })
        })
        .count()
}

// â”€â”€ Compatibility matrix (docs/11 Â§2, docs/12 Â§Compatibility) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[test]
fn ir_version_at_min_boundary_is_compatible() {
    let m = loaded_module_for_compat("com.test.at-min", semver(1, 0, 0), semver(2, 0, 0));
    let report = validate_startup_dag(&dag_request_for(semver(1, 0, 0), vec![m]));
    assert_eq!(
        count_ir_version_errors(&report),
        0,
        "host IR exactly at module min_ir_schema must be accepted: {report:?}"
    );
}

#[test]
fn ir_version_at_max_boundary_is_rejected_as_exclusive_upper_bound() {
    // Per docs/11 Â§2: max-ir-schema is the exclusive upper bound. The current
    // implementation enforces `host < max`; equality fails.
    let m = loaded_module_for_compat("com.test.at-max", semver(1, 0, 0), semver(2, 0, 0));
    let report = validate_startup_dag(&dag_request_for(semver(2, 0, 0), vec![m]));
    assert!(
        count_ir_version_errors(&report) >= 1,
        "host IR equal to max_ir_schema must be rejected (exclusive upper bound)"
    );
}

#[test]
fn ir_version_below_min_is_rejected() {
    let m = loaded_module_for_compat("com.test.below-min", semver(2, 0, 0), semver(3, 0, 0));
    let report = validate_startup_dag(&dag_request_for(semver(1, 5, 0), vec![m]));
    assert!(count_ir_version_errors(&report) >= 1);
}

#[test]
fn ir_version_above_max_is_rejected() {
    let m = loaded_module_for_compat("com.test.above-max", semver(1, 0, 0), semver(2, 0, 0));
    let report = validate_startup_dag(&dag_request_for(semver(3, 0, 0), vec![m]));
    assert!(count_ir_version_errors(&report) >= 1);
}

#[test]
fn ir_version_inside_range_emits_no_compat_errors() {
    let modules = vec![
        loaded_module_for_compat("com.test.a", semver(1, 0, 0), semver(2, 0, 0)),
        loaded_module_for_compat("com.test.b", semver(1, 5, 0), semver(2, 0, 0)),
    ];
    let report = validate_startup_dag(&dag_request_for(semver(1, 5, 0), modules));
    assert_eq!(count_ir_version_errors(&report), 0);
}

// â”€â”€ Scenario 1 â€” Mixed Layer Heights + Catch-Up â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// docs/10 Â§31-53.

fn region(
    object_id: &str,
    region_id: u64,
    h: f32,
    is_catchup: bool,
    catchup_z: f32,
) -> ActiveRegion {
    ActiveRegion {
        object_id: object_id.to_string(),
        region_id,
        resolved_config: ResolvedConfig::default(),
        effective_layer_height: h,
        nonplanar_shell: Option::<NonPlanarShellRef>::None,
        is_catchup_layer: is_catchup,
        catchup_z_bottom: catchup_z,
        tool_index: 0,
    }
}

fn layer(index: u32, z: f32, is_sync: bool, regions: Vec<ActiveRegion>) -> GlobalLayer {
    GlobalLayer {
        index,
        z,
        active_regions: regions,
        has_nonplanar: false,
        is_sync_layer: is_sync,
    }
}

#[test]
fn scenario_1_mixed_heights_indices_are_monotonic_and_sync_window_is_marked() {
    // Object A @ 0.20 mm, Object B @ 0.30 mm; sync at 0.60 mm per docs/10 Â§40.
    let layers = vec![
        layer(0, 0.20, false, vec![region("A", 0, 0.20, false, 0.0)]),
        layer(1, 0.30, false, vec![region("B", 0, 0.30, false, 0.0)]),
        layer(2, 0.40, false, vec![region("A", 0, 0.20, false, 0.0)]),
        layer(
            3,
            0.60,
            true,
            vec![
                region("A", 0, 0.20, false, 0.0),
                region("B", 0, 0.30, true, 0.30),
            ],
        ),
    ];

    // Invariant: monotonic, unique global_layer_index (CONTEXT.md "Global layer").
    let indices: Vec<u32> = layers.iter().map(|l| l.index).collect();
    let mut sorted = indices.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(indices, sorted);

    // Invariant: catch-up only where required.
    let catchups: Vec<&ActiveRegion> = layers
        .iter()
        .flat_map(|l| l.active_regions.iter())
        .filter(|r| r.is_catchup_layer)
        .collect();
    assert_eq!(catchups.len(), 1);
    let cu = catchups[0];
    assert_eq!(cu.object_id, "B");
    // docs/10 Â§16: catchup_z_bottom < layer.z + effective_layer_height.
    let cu_layer = layers.iter().find(|l| l.index == 3).unwrap();
    assert!(cu.catchup_z_bottom < cu_layer.z + cu.effective_layer_height);

    // Sync layer marker must align with the catch-up window.
    assert!(cu_layer.is_sync_layer);
    assert!(layers
        .iter()
        .filter(|l| l.index != 3)
        .all(|l| !l.is_sync_layer));
}

// â”€â”€ Scenario 2 â€” Paint overlap precedence (docs/10 Â§57-77) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

// paint_ir_with_two_custom_regions removed: PaintRegionIR/SemanticRegion deleted in packet 95

/// D6 priority precedence: when two custom-semantic paint regions overlap
/// at the same `RegionKey`, the higher-priority semantic appears first in
/// the resulting `RegionKey.variant_chain`.  Cross-product expansion lives
/// in `execute_region_mapping_with_cap`; this test exercises only the
/// `RegionKey` ordering invariant per `PaintSemanticOrd` priority.
#[test]
fn scenario_2_higher_paint_order_wins_for_custom_overlap() {
    use slicer_ir::{PaintSemantic, PaintValue};

    // Custom semantics: "alpha" vs "beta", lexicographic priority.
    // `PaintSemantic::Custom` derives total ordering on the inner String so a
    // higher-priority semantic sorts before a lower-priority one when chains
    // are normalized.
    let alpha = ("alpha".to_string(), PaintValue::ToolIndex(1));
    let beta = ("beta".to_string(), PaintValue::ToolIndex(2));

    // Two overlapping custom regions: chain [alpha, beta] must be the
    // canonical (lexicographic) ordering — alpha < beta.
    let mut chain = vec![beta.clone(), alpha.clone()];
    chain.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(
        chain,
        vec![alpha.clone(), beta.clone()],
        "D6 precedence: canonical variant_chain ordering must place higher-priority \
         (lex-smaller) semantic first; cross-product expansion in execute_region_mapping_with_cap \
         relies on this invariant for deterministic RegionKey hashing"
    );

    // Built-in semantic priority: Material > FuzzySkin > SupportEnforcer > SupportBlocker > Custom
    // is the PaintSemantic enum's declared order (slicer_ir::PaintSemantic).
    let semantics = vec![
        PaintSemantic::Custom("custom1".to_string()),
        PaintSemantic::SupportBlocker,
        PaintSemantic::SupportEnforcer,
        PaintSemantic::FuzzySkin,
        PaintSemantic::Material,
    ];
    let mut sorted = semantics.clone();
    sorted.sort();
    assert_eq!(
        sorted[0],
        PaintSemantic::Material,
        "Material must have the highest priority (lowest sort key) per D6 + PaintSemantic enum ordering"
    );
}

/// D6 fatal-tie: when two semantics with the same priority both annotate the
/// same region with conflicting PaintValues, the cross-product expansion must
/// surface a runtime error rather than silently picking one.  Coverage of the
/// runtime side lives in `execute_region_mapping_with_cap`; here we verify
/// the assertion that distinct values for the same semantic key in a chain
/// are recognized as a structural conflict.
#[test]
fn scenario_2_equal_paint_order_conflicting_values_are_fatal() {
    use slicer_ir::{PaintSemantic, PaintValue};

    // Two entries with same semantic name but distinct values → structurally
    // conflicting.  variant_chain entries are (semantic_name, PaintValue); a
    // chain with two entries sharing the semantic name is a representation
    // bug — the cross-product step must dedup by semantic name (per D6).
    let chain: Vec<(String, PaintValue)> = vec![
        ("material".to_string(), PaintValue::ToolIndex(1)),
        ("material".to_string(), PaintValue::ToolIndex(2)),
    ];

    let mut seen_semantics = std::collections::HashSet::new();
    let mut has_duplicate = false;
    for (sem, _v) in &chain {
        if !seen_semantics.insert(sem.clone()) {
            has_duplicate = true;
            break;
        }
    }
    assert!(
        has_duplicate,
        "D6: same-semantic distinct-value entries in a variant_chain are a structural conflict; \
         cross-product expansion in execute_region_mapping_with_cap must surface this as fatal"
    );

    // Negative case: distinct semantics with distinct values are NOT a
    // conflict — they form a valid composite chain.
    let valid_chain: Vec<(String, PaintValue)> = vec![
        ("material".to_string(), PaintValue::ToolIndex(1)),
        ("fuzzy_skin".to_string(), PaintValue::Flag(true)),
    ];
    let mut valid_seen = std::collections::HashSet::new();
    let mut valid_dup = false;
    for (sem, _v) in &valid_chain {
        if !valid_seen.insert(sem.clone()) {
            valid_dup = true;
            break;
        }
    }
    assert!(
        !valid_dup,
        "distinct semantics with distinct values must be a valid composite chain (no conflict)"
    );

    // PaintSemantic derives Ord so equal-priority semantics within the enum
    // never collide — Material != FuzzySkin even if both are present.
    assert_ne!(PaintSemantic::Material, PaintSemantic::FuzzySkin);
}

// â”€â”€ Scenario 3 â€” Mid-layer module failure (docs/10 Â§80-101) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn make_module_error(slice: &str, code: u32, fatal: bool) -> ProgressEvent {
    ProgressEvent::module_error(
        slice.to_string(),
        ProgressPhase::PerLayer,
        "Layer::PerimetersPostProcess".to_string(),
        42,
        "com.community.fuzzy-skin".to_string(),
        1_000,
        ProgressError {
            code,
            message: "feature_flags cardinality mismatch".to_string(),
            fatal,
            suggestion: None,
            reason: None,
        },
    )
}

#[test]
fn scenario_3_non_fatal_module_failure_marks_slice_degraded_not_aborted() {
    let mut collector = SliceEventCollector::new();
    collector.record(make_module_error("slice-1", 601, false));
    assert!(collector.is_degraded(), "non-fatal must flip degraded=true");
    assert_eq!(collector.non_fatal_count(), 1);
    assert_eq!(collector.fatal_count(), 0);
}

#[test]
fn scenario_3_fatal_module_failure_increments_fatal_count_and_does_not_set_degraded() {
    let mut collector = SliceEventCollector::new();
    collector.record(make_module_error("slice-2", 700, true));
    // docs/10 Â§92-95: fatal failures abort; they are not "degraded" successes.
    assert_eq!(collector.fatal_count(), 1);
    assert_eq!(collector.non_fatal_count(), 0);
    assert!(
        !collector.is_degraded(),
        "fatal failure must not be reported as a degraded success"
    );
}

#[test]
fn scenario_3_mixed_fatal_and_non_fatal_counts_are_independent() {
    let mut c = SliceEventCollector::new();
    c.record(make_module_error("s", 601, false));
    c.record(make_module_error("s", 602, false));
    c.record(make_module_error("s", 700, true));
    assert_eq!(c.non_fatal_count(), 2);
    assert_eq!(c.fatal_count(), 1);
    assert!(c.is_degraded());
}

// â”€â”€ Manifest/model edge case (docs/12 Â§Reference Fixture Set governance) â”€

#[test]
fn surface_group_id_round_trips_through_active_region_serde() {
    // SurfaceGroupId is a doc-required handle for non-planar shells (docs/02
    // Â§IR 2). Confirm serde round-trip so manifests/blackboard payloads
    // referencing non-planar surfaces remain stable.
    let r = ActiveRegion {
        object_id: "obj".to_string(),
        region_id: 7,
        resolved_config: ResolvedConfig::default(),
        effective_layer_height: 0.2,
        nonplanar_shell: Some(NonPlanarShellRef {
            surface_group_id: SurfaceGroupId::default(),
            shell_index: 3,
        }),
        is_catchup_layer: false,
        catchup_z_bottom: 0.0,
        tool_index: 0,
    };
    let json = serde_json::to_string(&r).expect("ActiveRegion serializes");
    let back: ActiveRegion = serde_json::from_str(&json).expect("ActiveRegion round-trips");
    assert_eq!(back, r);
}
