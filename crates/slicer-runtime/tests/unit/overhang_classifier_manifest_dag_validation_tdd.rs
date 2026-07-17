//! Packet 107 (iteration 2) regression: nothing previously loaded the real
//! `overhang-classifier-default` manifest through production DAG validation.
//! A manifest defect (declaring only the dotted field-level read
//! `"LayerCollectionIR.overhang_quartile"` and losing the base-IR read
//! `"LayerCollectionIR"`, which is what actually establishes the
//! cross-stage fulfillment edge from `path-optimization-default`) shipped
//! invisibly and was only caught by human review, not by any test.
//!
//! This test drives the exact production path: `load_modules_from_roots`
//! (the manifest loader) -> `build_intra_stage_dag` per stage ->
//! `validate_startup_dag` (mirrors `crates/slicer-runtime/src/run.rs`'s
//! startup-validation block) against the real manifests on disk under
//! `modules/core-modules/`.

#![allow(missing_docs)]

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use slicer_ir::CURRENT_SLICE_IR_SCHEMA_VERSION;
use slicer_runtime::{
    build_intra_stage_dag, load_modules_from_roots, manifest::LoadedModuleBuilder,
    validate_startup_dag, ClaimHolder, ConflictScope, DagValidationPass, DagValidationRequest,
    LoadedModule, Producer, SchedulerError, StageDag,
};

const CLASSIFIER_ID: &str = "com.core.overhang-classifier-default";
const PATH_OPT_ID: &str = "com.core.path-optimization-default";
const BASE_FIELD: &str = "LayerCollectionIR";
const DOTTED_FIELD: &str = "LayerCollectionIR.overhang_quartile";

fn core_modules_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .join("modules")
        .join("core-modules")
}

/// Groups a flat module list into one `StageDag` per distinct declared
/// stage, exactly as `run_slice` does in `src/run.rs` (minus the built-in
/// producers, which don't touch `LayerCollectionIR`).
fn stage_dags_for(modules: &[LoadedModule]) -> Vec<StageDag> {
    let mut by_stage: BTreeMap<&str, Vec<&dyn Producer>> = BTreeMap::new();
    for m in modules {
        by_stage
            .entry(m.stage())
            .or_default()
            .push(m as &dyn Producer);
    }
    by_stage
        .into_iter()
        .map(|(stage, producers)| StageDag {
            stage: stage.to_string(),
            nodes: build_intra_stage_dag(stage.to_string(), &producers)
                .expect("real core-module manifests must form a valid intra-stage DAG"),
        })
        .collect()
}

fn validation_request(modules: Vec<LoadedModule>) -> DagValidationRequest {
    let stage_dags = stage_dags_for(&modules);
    let claim_holders: Vec<ClaimHolder> = modules
        .iter()
        .flat_map(|m| {
            m.claims().iter().map(|claim| ClaimHolder {
                claim: claim.clone(),
                module_id: m.id().to_string(),
                scope: ConflictScope::Global,
            })
        })
        .collect();
    DagValidationRequest {
        modules,
        stage_dags,
        host_ir_schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
        host_version: slicer_runtime::manifest::parse_semver(env!("CARGO_PKG_VERSION"))
            .expect("slicer-runtime CARGO_PKG_VERSION must be valid semver"),
        claim_holders,
        access_audits: Vec::new(),
    }
}

// ============================================================================
// 1+2: load the real manifests, run production DAG validation, assert the
// base "LayerCollectionIR" read for overhang-classifier-default is FULFILLED.
// ============================================================================

#[test]
fn overhang_classifier_base_layer_collection_read_is_fulfilled_against_real_manifests() {
    let root = core_modules_root();
    assert!(
        root.is_dir(),
        "core-modules directory missing at {}",
        root.display()
    );

    let report = load_modules_from_roots(&[root]).expect("all core module manifests should load");

    assert!(
        report.modules.iter().any(|m| m.id() == CLASSIFIER_ID),
        "expected {CLASSIFIER_ID} to be discovered among core modules"
    );
    assert!(
        report.modules.iter().any(|m| m.id() == PATH_OPT_ID),
        "expected {PATH_OPT_ID} to be discovered among core modules"
    );

    let request = validation_request(report.modules);
    let dag_report = validate_startup_dag(&request);

    // The defect under regression: dotted-only reads silently drop the base
    // "LayerCollectionIR" fulfillment edge from path-optimization-default
    // (Layer::PathOptimization) into overhang-classifier-default
    // (PostPass::LayerFinalization). If that edge is lost, the base field
    // read is either absent (no diagnostic either way, see the
    // `defect_class_is_structurally_invisible_to_unfulfilled_reads_pass`
    // test below) or, if somehow re-declared without a real upstream
    // writer, would surface here as an UnfulfilledRead error.
    let base_unfulfilled = dag_report.errors.iter().find(|d| {
        d.pass == DagValidationPass::UnfulfilledReads
            && matches!(
                &d.detail,
                SchedulerError::UnfulfilledRead { module, field, .. }
                    if module == CLASSIFIER_ID && field == BASE_FIELD
            )
    });
    assert!(
        base_unfulfilled.is_none(),
        "expected the base 'LayerCollectionIR' read declared by {CLASSIFIER_ID} to be \
         fulfilled by {PATH_OPT_ID}'s earlier-stage write, but got UnfulfilledRead: \
         {base_unfulfilled:?}"
    );

    // The dotted field-level annotation ("LayerCollectionIR.overhang_quartile") is a
    // documented, currently-inert advisory: DAG validation matches read/write paths by
    // exact string, no module writes that literal dotted string (writers declare the
    // base "LayerCollectionIR"), so this always produces an UnfulfilledRead diagnostic
    // regardless of manifest correctness. We deliberately do NOT assert its absence —
    // see AC-4 / the manifest's own `[ir-access]` comment.
    let dotted_present = dag_report.errors.iter().any(|d| {
        d.pass == DagValidationPass::UnfulfilledReads
            && matches!(
                &d.detail,
                SchedulerError::UnfulfilledRead { module, field, .. }
                    if module == CLASSIFIER_ID && field == DOTTED_FIELD
            )
    });
    assert!(
        dotted_present,
        "expected the documented inert advisory UnfulfilledRead for the dotted \
         '{DOTTED_FIELD}' annotation to still be present; its absence would mean either \
         the annotation was dropped from the manifest, or DAG validation gained \
         field-level matching (in which case this comment and assertion should be updated)"
    );
}

// ============================================================================
// 3: regression tripwire (AC-4) — the manifest on disk must still declare
// BOTH the base and the dotted field-level read.
// ============================================================================

#[test]
fn overhang_classifier_manifest_declares_both_base_and_dotted_reads() {
    let manifest_path = core_modules_root()
        .join("overhang-classifier-default")
        .join("overhang-classifier-default.toml");
    let text = fs::read_to_string(&manifest_path)
        .unwrap_or_else(|e| panic!("failed to read manifest at {manifest_path:?}: {e}"));
    let toml: toml::Value = text.parse().expect("manifest must be valid TOML");

    let reads: Vec<String> = toml["ir-access"]["reads"]
        .as_array()
        .expect("ir-access.reads must be an array")
        .iter()
        .map(|v| {
            v.as_str()
                .expect("reads entries must be strings")
                .to_string()
        })
        .collect();

    assert!(
        reads.iter().any(|r| r == BASE_FIELD),
        "manifest reads {reads:?} must still declare the base '{BASE_FIELD}' read — \
         its loss is exactly the regression this test guards against"
    );
    assert!(
        reads.iter().any(|r| r == DOTTED_FIELD),
        "manifest reads {reads:?} must still declare the field-level '{DOTTED_FIELD}' \
         annotation (AC-4)"
    );
}

// ============================================================================
// 4: negative guard — reconstruct the broken dotted-only reads form and show
// what DAG validation actually does (and does not) detect about it.
// ============================================================================
//
// Finding: `validate_unfulfilled_reads` (crates/slicer-scheduler/src/validation.rs)
// only iterates `module.ir_reads()` and checks each *declared* field for an
// upstream writer. If the base "LayerCollectionIR" read is missing from the
// declaration entirely (the broken dotted-only form), the pass never attempts
// to check it — there is no "missing base read" diagnostic, because there is
// nothing to check. This is precisely why the defect "shipped invisibly": the
// UnfulfilledReads pass is structurally blind to a *missing declaration*, as
// opposed to a *declared-but-unsatisfied* one. We therefore cannot assert that
// validation "DOES emit UnfulfilledRead" for the broken form (it does not, and
// making it do so would require changing validation.rs, which is out of this
// packet's scope). Instead this test proves the detection gap precisely: the
// broken and fixed forms produce IDENTICAL UnfulfilledReads diagnostics for
// the base field (both silent), so the manifest-content tripwire above (test
// `overhang_classifier_manifest_declares_both_base_and_dotted_reads`) is the
// only mechanism that actually catches this defect class today.
#[test]
fn defect_class_is_structurally_invisible_to_unfulfilled_reads_pass() {
    let path_opt = LoadedModuleBuilder::new(
        PATH_OPT_ID.to_string(),
        semver(0, 1, 0),
        "Layer::PathOptimization".to_string(),
        slicer_schema::WORLD_LAYER.to_string(),
        PathBuf::from("fixtures/path-opt.wasm"),
    )
    .ir_reads(vec![])
    .ir_writes(vec![BASE_FIELD.to_string()])
    .min_host_version(semver(0, 1, 0))
    .min_ir_schema(semver(1, 0, 0))
    .max_ir_schema(semver(5, 0, 0))
    .layer_parallel_safe(true)
    .build();

    let broken_classifier = LoadedModuleBuilder::new(
        CLASSIFIER_ID.to_string(),
        semver(0, 1, 0),
        "PostPass::LayerFinalization".to_string(),
        slicer_schema::WORLD_FINALIZATION.to_string(),
        PathBuf::from("fixtures/overhang-classifier.wasm"),
    )
    // The defect: dotted-only, base "LayerCollectionIR" read dropped.
    .ir_reads(vec![DOTTED_FIELD.to_string()])
    .ir_writes(vec![BASE_FIELD.to_string()])
    .min_host_version(semver(0, 1, 0))
    .min_ir_schema(semver(1, 0, 0))
    .max_ir_schema(semver(5, 0, 0))
    .layer_parallel_safe(false)
    .build();

    let fixed_classifier = LoadedModuleBuilder::new(
        CLASSIFIER_ID.to_string(),
        semver(0, 1, 0),
        "PostPass::LayerFinalization".to_string(),
        slicer_schema::WORLD_FINALIZATION.to_string(),
        PathBuf::from("fixtures/overhang-classifier.wasm"),
    )
    .ir_reads(vec![BASE_FIELD.to_string(), DOTTED_FIELD.to_string()])
    .ir_writes(vec![BASE_FIELD.to_string()])
    .min_host_version(semver(0, 1, 0))
    .min_ir_schema(semver(1, 0, 0))
    .max_ir_schema(semver(5, 0, 0))
    .layer_parallel_safe(false)
    .build();

    let base_field_diagnostics = |classifier: LoadedModule| -> usize {
        let request = validation_request(vec![path_opt.clone(), classifier]);
        let report = validate_startup_dag(&request);
        report
            .errors
            .iter()
            .filter(|d| {
                d.pass == DagValidationPass::UnfulfilledReads
                    && matches!(
                        &d.detail,
                        SchedulerError::UnfulfilledRead { module, field, .. }
                            if module == CLASSIFIER_ID && field == BASE_FIELD
                    )
            })
            .count()
    };

    let broken_count = base_field_diagnostics(broken_classifier.clone());
    let fixed_count = base_field_diagnostics(fixed_classifier);

    assert_eq!(
        broken_count, 0,
        "broken (dotted-only) form: UnfulfilledReads pass does not check an undeclared \
         base field, so it emits zero diagnostics for it — confirming the pass cannot \
         detect a missing declaration"
    );
    assert_eq!(
        fixed_count, 0,
        "fixed (base+dotted) form: base field is declared and satisfied by path-optimization's \
         write, so zero diagnostics too"
    );
    assert_eq!(
        broken_count, fixed_count,
        "the UnfulfilledReads pass produces identical (silent) output for both the broken \
         and fixed manifest forms — this is the structural detection gap that let the \
         defect ship invisibly"
    );

    // What DOES differ, and what the manifest-content tripwire test above guards:
    // the declared read set itself.
    assert!(
        !broken_classifier
            .ir_reads()
            .contains(&BASE_FIELD.to_string()),
        "sanity: broken form must not declare the base read"
    );
}

fn semver(major: u32, minor: u32, patch: u32) -> slicer_ir::SemVer {
    slicer_ir::SemVer {
        major,
        minor,
        patch,
    }
}
