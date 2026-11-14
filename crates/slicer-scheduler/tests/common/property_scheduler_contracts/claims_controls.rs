//! Deterministic controls for charter `H-CLAIMS`: typed claim scope/identity,
//! the reviewed exception table, write ordering and effective-universe
//! dependency rules.
//!
//! Every witness here drives the real public
//! `slicer_scheduler::validation::validate_startup_dag` surface and compares
//! the exact `SchedulerError` payload, not just `report.is_valid()`. The models
//! in `../property_scheduler_claims.rs` supply the case builders and the typed
//! scope representation; this file deliberately re-derives the expected
//! diagnostics for each concrete input.

#![allow(missing_docs)]

// The target-private model file is mounted once by this binary's root; this
// alias keeps the control body readable.
use crate::claim_models as models;

use std::collections::BTreeSet;

use slicer_scheduler::{
    validate_startup_dag, ClaimHolder, ConflictScope, DagValidationPass, DagValidationReport,
    DagValidationRequest, LoadedModule, SchedulerError, StageDag, FILL_CLAIM_IDS,
};

use proptest::strategy::{Strategy, ValueTree};
use proptest::test_runner::{Config, RngAlgorithm, RngSeed, TestRunner};

use models::ScopeModel;

fn control_ids(count: usize) -> Vec<String> {
    (0..count)
        .map(|index| format!("com.control.claim-{index}"))
        .collect()
}

fn plain_modules(ids: &[String]) -> Vec<LoadedModule> {
    ids.iter()
        .map(|id| models::plain_module(id, models::STAGE_INFILL))
        .collect()
}

fn request(modules: Vec<LoadedModule>, claim_holders: Vec<ClaimHolder>) -> DagValidationRequest {
    let stage_dags = models::stage_dags_for(&modules);
    models::request_from(modules, stage_dags, claim_holders)
}

fn scope_view(scope: &ConflictScope) -> ScopeModel {
    match scope {
        ConflictScope::Global => ScopeModel::Global,
        ConflictScope::Region {
            object_id,
            region_id,
            global_layer_index,
        } => ScopeModel::Region {
            object_id: object_id.clone(),
            region_id: region_id.clone(),
            layer: *global_layer_index,
        },
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ClaimView {
    pass: &'static str,
    claim: String,
    module_a: String,
    module_b: String,
    scope: ScopeModel,
}

fn claim_conflicts(report: &DagValidationReport) -> Vec<ClaimView> {
    let mut view: Vec<ClaimView> = report
        .errors
        .iter()
        .filter_map(|diagnostic| {
            let pass = match diagnostic.pass {
                DagValidationPass::GlobalClaimConflicts => "global",
                DagValidationPass::PerRegionClaimConflicts => "per-region",
                _ => return None,
            };
            match &diagnostic.detail {
                SchedulerError::ClaimConflict {
                    claim,
                    module_a,
                    module_b,
                    scope,
                } => Some(ClaimView {
                    pass,
                    claim: claim.clone(),
                    module_a: module_a.clone(),
                    module_b: module_b.clone(),
                    scope: scope_view(scope),
                }),
                // The fill-role coverage check lives in the global claim pass.
                SchedulerError::MissingDependency { .. } => None,
                other => panic!("a claim pass emitted an unexpected payload: {other:?}"),
            }
        })
        .collect();
    view.sort();
    view
}

fn missing_dependencies(report: &DagValidationReport) -> Vec<(String, String)> {
    let mut view: Vec<(String, String)> = report
        .errors
        .iter()
        .filter(|diagnostic| {
            diagnostic.pass == DagValidationPass::MissingDependencies
                || diagnostic.pass == DagValidationPass::GlobalClaimConflicts
        })
        .filter_map(|diagnostic| match &diagnostic.detail {
            SchedulerError::MissingDependency { module, requires } => {
                Some((module.clone(), requires.clone()))
            }
            _ => None,
        })
        .collect();
    view.sort();
    view
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct WriteView {
    field: String,
    module_a: String,
    module_b: String,
    stage: String,
    orderable: bool,
}

fn write_conflicts(report: &DagValidationReport) -> Vec<WriteView> {
    let mut view: Vec<WriteView> = report
        .errors
        .iter()
        .filter(|diagnostic| diagnostic.pass == DagValidationPass::WriteConflicts)
        .map(|diagnostic| match &diagnostic.detail {
            SchedulerError::WriteConflict {
                field,
                module_a,
                module_b,
                stage,
                orderable,
            } => {
                let (module_a, module_b) = if module_a <= module_b {
                    (module_a.clone(), module_b.clone())
                } else {
                    (module_b.clone(), module_a.clone())
                };
                WriteView {
                    field: field.clone(),
                    module_a,
                    module_b,
                    stage: stage.clone(),
                    orderable: *orderable,
                }
            }
            other => panic!("the write-conflict pass emitted an unexpected payload: {other:?}"),
        })
        .collect();
    view.sort();
    view
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct CrossView {
    module: String,
    requires: String,
    module_stage: String,
    required_stage: String,
}

fn cross_stage_dependencies(report: &DagValidationReport) -> Vec<CrossView> {
    let mut view: Vec<CrossView> = report
        .errors
        .iter()
        .filter(|diagnostic| diagnostic.pass == DagValidationPass::CrossStageDependencyLegality)
        .map(|diagnostic| match &diagnostic.detail {
            SchedulerError::CrossStageDependency {
                module,
                requires,
                module_stage,
                required_stage,
            } => CrossView {
                module: module.clone(),
                requires: requires.clone(),
                module_stage: module_stage.clone(),
                required_stage: required_stage.clone(),
            },
            other => panic!("the cross-stage pass emitted an unexpected payload: {other:?}"),
        })
        .collect();
    view.sort();
    view
}

fn transitive_dependencies(
    report: &DagValidationReport,
) -> Vec<(String, String, String, Vec<String>)> {
    let mut view: Vec<(String, String, String, Vec<String>)> = report
        .errors
        .iter()
        .filter(|diagnostic| diagnostic.pass == DagValidationPass::TransitiveDependencyLegality)
        .map(|diagnostic| match &diagnostic.detail {
            SchedulerError::TransitiveStageDependency {
                module,
                path,
                module_stage,
                later_stage,
            } => (
                module.clone(),
                module_stage.clone(),
                later_stage.clone(),
                path.clone(),
            ),
            other => panic!("the transitive pass emitted an unexpected payload: {other:?}"),
        })
        .collect();
    view.sort();
    view
}

#[test]
fn scheduler_claim_scope_and_exception_controls() {
    let ids = control_ids(3);
    let modules = plain_modules(&ids);

    // The configured fill catalog is exactly the four documented ids. Deriving
    // expectations from the production constant is only sound with this check.
    let mut production: Vec<&str> = FILL_CLAIM_IDS.to_vec();
    production.sort_unstable();
    let mut documented: Vec<&str> = models::EXPECTED_FILL_CLAIM_IDS.to_vec();
    documented.sort_unstable();
    assert_eq!(
        production, documented,
        "FILL_CLAIM_IDS must be exactly the four documented configured fill ids"
    );
    assert!(
        !FILL_CLAIM_IDS.contains(&models::RAFT_CLAIM),
        "claim:raft-fill is an ordinary claim, never a configured fill exception"
    );

    // ── Global ordinary: distinct holders conflict pairwise; a duplicate
    // observation of one holder adds no pair. ────────────────────────────────
    let mut holders = models::fixed_fill_holders(&ids[0]);
    holders.push(models::global_holder(models::ORDINARY_CLAIM, &ids[0]));
    holders.push(models::global_holder(models::ORDINARY_CLAIM, &ids[0]));
    holders.push(models::global_holder(models::ORDINARY_CLAIM, &ids[1]));
    let report = validate_startup_dag(&request(modules.clone(), holders));
    assert_eq!(
        claim_conflicts(&report),
        vec![ClaimView {
            pass: "global",
            claim: models::ORDINARY_CLAIM.to_string(),
            module_a: ids[0].clone(),
            module_b: ids[1].clone(),
            scope: ScopeModel::Global,
        }],
        "a duplicate observation of the same module must not be treated as a second holder"
    );

    // ── claim:raft-fill is ordinary at this API: two global holders conflict. ─
    let mut holders = models::fixed_fill_holders(&ids[0]);
    holders.push(models::global_holder(models::RAFT_CLAIM, &ids[0]));
    holders.push(models::global_holder(models::RAFT_CLAIM, &ids[1]));
    let report = validate_startup_dag(&request(modules.clone(), holders));
    assert_eq!(
        claim_conflicts(&report),
        vec![ClaimView {
            pass: "global",
            claim: models::RAFT_CLAIM.to_string(),
            module_a: ids[0].clone(),
            module_b: ids[1].clone(),
            scope: ScopeModel::Global,
        }],
        "claim:raft-fill must still be rejected when two modules hold it globally"
    );

    // ── Every configured fill id allows global multiplicity ... ──────────────
    for fill in FILL_CLAIM_IDS {
        let mut holders = models::fixed_fill_holders(&ids[0]);
        holders.push(models::global_holder(fill, &ids[1]));
        let report = validate_startup_dag(&request(modules.clone(), holders));
        assert!(
            claim_conflicts(&report).is_empty(),
            "configured fill id {fill} must allow multiple global holders, got {:?}",
            claim_conflicts(&report)
        );
        assert!(
            missing_dependencies(&report).is_empty(),
            "the four-role fill setup must be complete in the {fill} multiplicity witness"
        );
    }

    // ── ... and every configured fill id reports its exact absence. ──────────
    for fill in FILL_CLAIM_IDS {
        let holders: Vec<ClaimHolder> = models::fixed_fill_holders(&ids[0])
            .into_iter()
            .filter(|holder| holder.claim != *fill)
            .collect();
        let report = validate_startup_dag(&request(modules.clone(), holders));
        assert_eq!(
            missing_dependencies(&report),
            vec![(
                format!("fill-role-claim:{fill}"),
                format!("no module holds {fill}")
            )],
            "an absent configured fill id must produce its exact MissingDependency"
        );
        assert!(
            claim_conflicts(&report).is_empty(),
            "a single absent fill role must not also fabricate a claim conflict"
        );
    }

    // ── Global fill multiplicity does not exempt a per-region duplicate. ─────
    for fill in FILL_CLAIM_IDS {
        let mut holders = models::fixed_fill_holders(&ids[0]);
        holders.push(models::region_holder(fill, &ids[1], "obj", "r0", Some(0)));
        holders.push(models::region_holder(fill, &ids[2], "obj", "r0", Some(0)));
        let report = validate_startup_dag(&request(modules.clone(), holders));
        assert_eq!(
            claim_conflicts(&report),
            vec![ClaimView {
                pass: "per-region",
                claim: (*fill).to_string(),
                module_a: ids[1].clone(),
                module_b: ids[2].clone(),
                scope: ScopeModel::Region {
                    object_id: String::from("obj"),
                    region_id: String::from("r0"),
                    layer: Some(0),
                },
            }],
            "per-region duplicate holders of {fill} must still conflict"
        );
    }

    // ── The four finite family-support globals are exempt; an arbitrary
    // support-family:<suffix> is not. ────────────────────────────────────────
    for family in models::FAMILY_SUPPORT_GLOBAL_CLAIMS {
        let mut holders = models::fixed_fill_holders(&ids[0]);
        holders.push(models::global_holder(family, &ids[0]));
        holders.push(models::global_holder(family, &ids[1]));
        let report = validate_startup_dag(&request(modules.clone(), holders));
        assert!(
            claim_conflicts(&report).is_empty(),
            "family-support global {family} is a documented global exception, got {:?}",
            claim_conflicts(&report)
        );
    }
    let mut holders = models::fixed_fill_holders(&ids[0]);
    holders.push(models::global_holder(
        "support-family:tree-extended",
        &ids[0],
    ));
    holders.push(models::global_holder(
        "support-family:tree-extended",
        &ids[1],
    ));
    let report = validate_startup_dag(&request(modules.clone(), holders));
    assert_eq!(
        claim_conflicts(&report),
        vec![ClaimView {
            pass: "global",
            claim: String::from("support-family:tree-extended"),
            module_a: ids[0].clone(),
            module_b: ids[1].clone(),
            scope: ScopeModel::Global,
        }],
        "the family exception table is finite; support-family:<other> is not exempt"
    );

    // ── Delimiter-rich ids must remain distinct typed scopes. Two modules, one
    // holder each, in `(a:b, c)` and `(a, b:c)` never conflict. ──────────────
    let mut holders = models::fixed_fill_holders(&ids[0]);
    holders.push(models::region_holder(
        models::ORDINARY_CLAIM,
        &ids[0],
        "a:b",
        "c",
        Some(0),
    ));
    holders.push(models::region_holder(
        models::ORDINARY_CLAIM,
        &ids[1],
        "a",
        "b:c",
        Some(0),
    ));
    let report = validate_startup_dag(&request(modules.clone(), holders));
    assert!(
        claim_conflicts(&report).is_empty(),
        "`(a:b, c)` and `(a, b:c)` are distinct typed scopes and must not alias, got {:?}",
        claim_conflicts(&report)
    );

    // ── A real same-scope collision keeps the original object/region identity:
    // two holders at `(a:b, c, layer 3)` and one at `(a, b:c, layer 3)`. ─────
    let mut holders = models::fixed_fill_holders(&ids[0]);
    holders.push(models::region_holder(
        models::ORDINARY_CLAIM,
        &ids[0],
        "a:b",
        "c",
        Some(3),
    ));
    holders.push(models::region_holder(
        models::ORDINARY_CLAIM,
        &ids[1],
        "a:b",
        "c",
        Some(3),
    ));
    holders.push(models::region_holder(
        models::ORDINARY_CLAIM,
        &ids[2],
        "a",
        "b:c",
        Some(3),
    ));
    let report = validate_startup_dag(&request(modules.clone(), holders));
    assert_eq!(
        claim_conflicts(&report),
        vec![ClaimView {
            pass: "per-region",
            claim: models::ORDINARY_CLAIM.to_string(),
            module_a: ids[0].clone(),
            module_b: ids[1].clone(),
            scope: ScopeModel::Region {
                object_id: String::from("a:b"),
                region_id: String::from("c"),
                layer: Some(3),
            },
        }],
        "the diagnostic must retain the original object and region identity"
    );

    // ── Stable claims are object-wide: a holder change across *regions* of one
    // object at different layers is rejected. Same-region cross-layer retains
    // the original region and pins global_layer_index = None. ────────────────
    for claim in models::STABLE_CLAIM_IDS {
        let mut holders = models::fixed_fill_holders(&ids[0]);
        holders.push(models::region_holder(claim, &ids[0], "cube", "r0", Some(0)));
        holders.push(models::region_holder(claim, &ids[1], "cube", "r0", Some(1)));
        let report = validate_startup_dag(&request(modules.clone(), holders));
        assert_eq!(
            claim_conflicts(&report),
            vec![ClaimView {
                pass: "per-region",
                claim: (*claim).to_string(),
                module_a: ids[0].clone(),
                module_b: ids[1].clone(),
                scope: ScopeModel::Region {
                    object_id: String::from("cube"),
                    region_id: String::from("r0"),
                    layer: None,
                },
            }],
            "stable claim {claim} must reject a same-region cross-layer holder change"
        );

        let mut holders = models::fixed_fill_holders(&ids[0]);
        holders.push(models::region_holder(claim, &ids[0], "cube", "r0", Some(0)));
        holders.push(models::region_holder(claim, &ids[1], "cube", "r1", Some(1)));
        let report = validate_startup_dag(&request(modules.clone(), holders));
        assert_eq!(
            claim_conflicts(&report),
            vec![ClaimView {
                pass: "per-region",
                claim: (*claim).to_string(),
                module_a: ids[0].clone(),
                module_b: ids[1].clone(),
                scope: ScopeModel::Region {
                    object_id: String::from("cube"),
                    region_id: String::from("r0"),
                    layer: None,
                },
            }],
            "stable claim {claim} must not change holder across regions of one object"
        );
    }

    // ── One holder across layers, and different objects, stay valid. ─────────
    let mut holders = models::fixed_fill_holders(&ids[0]);
    holders.push(models::region_holder(
        models::STABLE_CLAIM_IDS[0],
        &ids[0],
        "cube",
        "r0",
        Some(0),
    ));
    holders.push(models::region_holder(
        models::STABLE_CLAIM_IDS[0],
        &ids[0],
        "cube",
        "r0",
        Some(5),
    ));
    let report = validate_startup_dag(&request(modules.clone(), holders));
    assert!(
        claim_conflicts(&report).is_empty(),
        "one stable holder across layers must remain valid, got {:?}",
        claim_conflicts(&report)
    );

    let mut holders = models::fixed_fill_holders(&ids[0]);
    holders.push(models::region_holder(
        models::STABLE_CLAIM_IDS[0],
        &ids[0],
        "cube-a",
        "r0",
        Some(0),
    ));
    holders.push(models::region_holder(
        models::STABLE_CLAIM_IDS[0],
        &ids[1],
        "cube-b",
        "r0",
        Some(1),
    ));
    let report = validate_startup_dag(&request(modules.clone(), holders));
    assert!(
        claim_conflicts(&report).is_empty(),
        "different objects must stay independent, got {:?}",
        claim_conflicts(&report)
    );

    // ── Declared transition-capable claims may change holder across layers. ──
    for claim in models::TRANSITION_CAPABLE_CLAIM_IDS {
        let mut holders = models::fixed_fill_holders(&ids[0]);
        holders.push(models::region_holder(claim, &ids[0], "cube", "r0", Some(0)));
        holders.push(models::region_holder(claim, &ids[1], "cube", "r0", Some(1)));
        let report = validate_startup_dag(&request(modules.clone(), holders));
        assert!(
            claim_conflicts(&report).is_empty(),
            "transition-capable claim {claim} must allow a per-layer holder change, got {:?}",
            claim_conflicts(&report)
        );
    }

    // ── ... but the same claim in ONE typed scope still rejects two holders.
    // The `support-generator` half is named explicitly by the exception table:
    // different layers are allowed, the same `(object, region, layer)` is not.
    let claim = models::TRANSITION_CAPABLE_CLAIM_IDS[1];
    assert_eq!(
        claim, "support-generator",
        "the same-scope witness must name the support-generator exception"
    );
    let mut holders = models::fixed_fill_holders(&ids[0]);
    holders.push(models::region_holder(claim, &ids[0], "cube", "r0", Some(2)));
    holders.push(models::region_holder(claim, &ids[1], "cube", "r0", Some(2)));
    let report = validate_startup_dag(&request(modules.clone(), holders));
    assert_eq!(
        claim_conflicts(&report),
        vec![ClaimView {
            pass: "per-region",
            claim: claim.to_string(),
            module_a: ids[0].clone(),
            module_b: ids[1].clone(),
            scope: ScopeModel::Region {
                object_id: String::from("cube"),
                region_id: String::from("r0"),
                layer: Some(2),
            },
        }],
        "two distinct {claim} holders in the same typed scope must still conflict"
    );
}

/// Root cause: `validate_claim_conflicts` grouped region holders by the
/// concatenated string `region:{object}:{region}:{layer}` and re-parsed that
/// string to rebuild the reported scope. Object `a:b` with region `c` and
/// object `a` with region `b:c` therefore produced the same key
/// (`region:a:b:c:0`), aliasing two independent scopes and corrupting the
/// diagnostic identity. Previous input: modules `com.control.claim-0` at
/// `(a:b, c)` and `com.control.claim-1` at `(a, b:c)`, both at layer 0; then a
/// second holder `com.control.claim-2` inside `(a:b, c)`.
#[test]
fn delimiter_rich_object_and_region_ids_never_share_a_claim_scope() {
    let ids = control_ids(3);
    let modules = plain_modules(&ids);

    let mut holders = models::fixed_fill_holders(&ids[0]);
    holders.push(models::region_holder(
        models::ORDINARY_CLAIM,
        &ids[0],
        "a:b",
        "c",
        Some(0),
    ));
    holders.push(models::region_holder(
        models::ORDINARY_CLAIM,
        &ids[1],
        "a",
        "b:c",
        Some(0),
    ));
    let report = validate_startup_dag(&request(modules.clone(), holders));
    assert!(
        claim_conflicts(&report).is_empty(),
        "distinct typed scopes must not alias through delimiter concatenation, got {:?}",
        claim_conflicts(&report)
    );

    let mut holders = models::fixed_fill_holders(&ids[0]);
    holders.push(models::region_holder(
        models::ORDINARY_CLAIM,
        &ids[0],
        "a:b",
        "c",
        Some(0),
    ));
    holders.push(models::region_holder(
        models::ORDINARY_CLAIM,
        &ids[1],
        "a:b",
        "c",
        Some(0),
    ));
    holders.push(models::region_holder(
        models::ORDINARY_CLAIM,
        &ids[2],
        "a",
        "b:c",
        Some(0),
    ));
    let report = validate_startup_dag(&request(modules, holders));
    assert_eq!(
        claim_conflicts(&report),
        vec![ClaimView {
            pass: "per-region",
            claim: models::ORDINARY_CLAIM.to_string(),
            module_a: ids[0].clone(),
            module_b: ids[1].clone(),
            scope: ScopeModel::Region {
                object_id: String::from("a:b"),
                region_id: String::from("c"),
                layer: Some(0),
            },
        }],
        "only the genuinely shared typed scope conflicts, with its original identities"
    );
}

#[test]
fn scheduler_write_dependency_and_disabled_controls() {
    let ids = control_ids(3);

    // ── Unordered writers of one exact path conflict. ────────────────────────
    let modules = vec![
        models::module_with(
            &ids[0],
            models::STAGE_INFILL,
            Vec::new(),
            Vec::new(),
            vec![String::from("MeshIR.layers")],
            Vec::new(),
            Vec::new(),
        ),
        models::module_with(
            &ids[1],
            models::STAGE_INFILL,
            Vec::new(),
            Vec::new(),
            vec![String::from("MeshIR.layers")],
            Vec::new(),
            Vec::new(),
        ),
    ];
    let report = validate_startup_dag(&request(modules, Vec::new()));
    assert_eq!(
        write_conflicts(&report),
        vec![WriteView {
            field: String::from("MeshIR.layers"),
            module_a: ids[0].clone(),
            module_b: ids[1].clone(),
            stage: models::STAGE_INFILL.to_string(),
            orderable: false,
        }],
        "two unordered writers of the same exact path must be rejected"
    );

    // ── Explicit ordering without consuming the shared field is not a
    // transform exemption. ──────────────────────────────────────────────────
    let modules = vec![
        models::module_with(
            &ids[0],
            models::STAGE_INFILL,
            Vec::new(),
            Vec::new(),
            vec![String::from("MeshIR.layers")],
            Vec::new(),
            Vec::new(),
        ),
        models::module_with(
            &ids[1],
            models::STAGE_INFILL,
            Vec::new(),
            Vec::new(),
            vec![String::from("MeshIR.layers")],
            vec![ids[0].clone()],
            Vec::new(),
        ),
    ];
    let report = validate_startup_dag(&request(modules, Vec::new()));
    assert_eq!(
        write_conflicts(&report),
        vec![WriteView {
            field: String::from("MeshIR.layers"),
            module_a: ids[0].clone(),
            module_b: ids[1].clone(),
            stage: models::STAGE_INFILL.to_string(),
            orderable: false,
        }],
        "a bare explicit ordering is not write ordering; the shared field must be consumed"
    );

    // ── A real transform (downstream reads the shared path and a path exists)
    // is retained. ───────────────────────────────────────────────────────────
    let modules = vec![
        models::module_with(
            &ids[0],
            models::STAGE_INFILL,
            Vec::new(),
            Vec::new(),
            vec![String::from("MeshIR.layers")],
            Vec::new(),
            Vec::new(),
        ),
        models::module_with(
            &ids[1],
            models::STAGE_INFILL,
            Vec::new(),
            vec![String::from("MeshIR.layers")],
            vec![String::from("MeshIR.layers")],
            Vec::new(),
            Vec::new(),
        ),
    ];
    let report = validate_startup_dag(&request(modules, Vec::new()));
    assert!(
        write_conflicts(&report).is_empty(),
        "a downstream writer that consumes the shared exact field is an orderable \
         transform, got {:?}",
        write_conflicts(&report)
    );

    // ── Family-support roots: SupportIR and SupportPlanIR descendants are
    // exempt; SupportIRx and an unrelated field are not. ─────────────────────
    let shared: Vec<String> = [
        "SupportIR",
        "SupportPlanIR.regions",
        "SupportIRx",
        "MeshIR.regions",
    ]
    .iter()
    .map(|field| (*field).to_string())
    .collect();
    let modules = vec![
        models::module_with(
            &ids[0],
            models::STAGE_INFILL,
            vec![
                String::from("support-generator"),
                String::from("support-family:traditional"),
            ],
            Vec::new(),
            shared.clone(),
            Vec::new(),
            Vec::new(),
        ),
        models::module_with(
            &ids[1],
            models::STAGE_INFILL,
            vec![
                String::from("support-planner"),
                String::from("support-family:traditional"),
            ],
            Vec::new(),
            shared,
            Vec::new(),
            Vec::new(),
        ),
    ];
    let mut holders = models::fixed_fill_holders(&ids[0]);
    holders.push(models::global_holder("support-generator", &ids[0]));
    holders.push(models::global_holder("support-planner", &ids[1]));
    let report = validate_startup_dag(&request(modules, holders));
    assert_eq!(
        write_conflicts(&report),
        vec![
            WriteView {
                field: String::from("MeshIR.regions"),
                module_a: ids[0].clone(),
                module_b: ids[1].clone(),
                stage: models::STAGE_INFILL.to_string(),
                orderable: false,
            },
            WriteView {
                field: String::from("SupportIRx"),
                module_a: ids[0].clone(),
                module_b: ids[1].clone(),
                stage: models::STAGE_INFILL.to_string(),
                orderable: false,
            },
        ],
        "only SupportIR/SupportPlanIR roots are family-orderable, and an unrelated \
         shared field stays rejected"
    );

    // ── Effective-universe membership: absent module and capability reject. ──
    let absent_module = String::from("com.control.absent-module");
    let absent_claim = String::from("claim:absent-capability");
    let modules = vec![
        models::module_with(
            &ids[0],
            models::STAGE_INFILL,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            vec![absent_module.clone()],
            vec![absent_claim.clone()],
        ),
        models::plain_module(&ids[1], models::STAGE_INFILL),
        models::plain_module(&ids[2], models::STAGE_INFILL),
    ];
    let report = validate_startup_dag(&request(modules, models::fixed_fill_holders(&ids[0])));
    assert_eq!(
        missing_dependencies(&report),
        vec![
            (ids[0].clone(), absent_claim),
            (ids[0].clone(), absent_module),
        ],
        "an absent module and an absent capability each reject with their exact ids"
    );

    // ── Direct and transitive cross-stage dependencies reject by STAGE_ORDER
    // rank, and every reported path is a real requires_modules path. ────────
    let modules = vec![
        models::module_with(
            &ids[0],
            models::STAGE_MESH_ANALYSIS,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            vec![ids[1].clone()],
            Vec::new(),
        ),
        models::module_with(
            &ids[1],
            models::STAGE_LAYER_PLANNING,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            vec![ids[2].clone()],
            Vec::new(),
        ),
        models::plain_module(&ids[2], models::STAGE_GCODE_EMIT),
    ];
    let stage_dags: Vec<StageDag> = models::stage_dags_for(&modules);
    let report = validate_startup_dag(&models::request_from(
        modules,
        stage_dags,
        models::fixed_fill_holders(&ids[0]),
    ));
    assert_eq!(
        cross_stage_dependencies(&report),
        vec![
            CrossView {
                module: ids[0].clone(),
                requires: ids[1].clone(),
                module_stage: models::STAGE_MESH_ANALYSIS.to_string(),
                required_stage: models::STAGE_LAYER_PLANNING.to_string(),
            },
            CrossView {
                module: ids[1].clone(),
                requires: ids[2].clone(),
                module_stage: models::STAGE_LAYER_PLANNING.to_string(),
                required_stage: models::STAGE_GCODE_EMIT.to_string(),
            },
        ],
        "every direct requirement of a later stage must reject with exact stage fields"
    );
    let transitive = transitive_dependencies(&report);
    let transitive_modules: BTreeSet<String> =
        transitive.iter().map(|entry| entry.0.clone()).collect();
    assert_eq!(
        transitive_modules,
        BTreeSet::from([ids[0].clone(), ids[1].clone()]),
        "only modules with a real path to a later stage receive a transitive diagnostic"
    );
    for (module, module_stage, later_stage, path) in &transitive {
        assert_eq!(path.first(), Some(module), "path must start at the module");
        let stage_of = |id: &str| -> &str {
            if id == ids[0] {
                models::STAGE_MESH_ANALYSIS
            } else if id == ids[1] {
                models::STAGE_LAYER_PLANNING
            } else {
                models::STAGE_GCODE_EMIT
            }
        };
        assert_eq!(
            stage_of(path.last().expect("nonempty path")),
            later_stage,
            "later_stage must be the stage of the path's last module"
        );
        assert!(
            path.windows(2).all(|pair| {
                match pair {
                    [source, target] => match source.as_str() {
                        source if source == ids[0].as_str() => target == &ids[1],
                        source if source == ids[1].as_str() => target == &ids[2],
                        _ => false,
                    },
                    _ => false,
                }
            }),
            "every reported path edge must be a declared requires_modules edge: {path:?}"
        );
        assert_eq!(
            module_stage,
            if module == &ids[0] {
                models::STAGE_MESH_ANALYSIS
            } else {
                models::STAGE_LAYER_PLANNING
            },
            "the requesting stage must be reported"
        );
    }

    // ── Enabled-only effective universe: removing the disabled module removes
    // only its conflict while the retained consumer must reject. ────────────
    let modules = vec![
        models::module_with(
            &ids[0],
            models::STAGE_INFILL,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            vec![ids[1].clone()],
            vec![String::from(models::CAPABILITY_CLAIM)],
        ),
        models::plain_module(&ids[1], models::STAGE_INFILL),
        models::plain_module(&ids[2], models::STAGE_INFILL),
    ];
    let mut holders = models::fixed_fill_holders(&ids[2]);
    holders.push(models::global_holder(models::CAPABILITY_CLAIM, &ids[1]));
    holders.push(models::global_holder(models::ORDINARY_CLAIM, &ids[1]));
    holders.push(models::global_holder(models::ORDINARY_CLAIM, &ids[2]));

    let unfiltered = validate_startup_dag(&request(modules.clone(), holders.clone()));
    assert_eq!(
        claim_conflicts(&unfiltered),
        vec![ClaimView {
            pass: "global",
            claim: models::ORDINARY_CLAIM.to_string(),
            module_a: ids[1].clone(),
            module_b: ids[2].clone(),
            scope: ScopeModel::Global,
        }],
        "the competing disabled holder conflicts while it is still effective"
    );
    assert!(
        missing_dependencies(&unfiltered).is_empty(),
        "the unfiltered universe satisfies the consumer"
    );

    let enabled_modules = vec![modules[0].clone(), modules[2].clone()];
    let enabled_holders: Vec<ClaimHolder> = holders
        .into_iter()
        .filter(|holder| holder.module_id != ids[1])
        .collect();
    let filtered = validate_startup_dag(&request(enabled_modules, enabled_holders));
    assert!(
        claim_conflicts(&filtered).is_empty(),
        "removing the disabled competing holder must eliminate only its conflict, got {:?}",
        claim_conflicts(&filtered)
    );
    assert_eq!(
        missing_dependencies(&filtered),
        vec![
            (ids[0].clone(), String::from(models::CAPABILITY_CLAIM)),
            (ids[0].clone(), ids[1].clone()),
        ],
        "the retained consumer must reject the removed module and capability"
    );
    assert!(
        missing_dependencies(&filtered)
            .iter()
            .all(|(module, _)| !module.starts_with("fill-role-claim:")),
        "the fixed fill setup survives the disabled removal"
    );
}

/// The direct-only false green for the transitive stage rule.
///
/// A and B share `PrePass::LayerPlanning`, B requires C in the later
/// `PostPass::GCodeEmit`, and A requires only B. A direct-only replacement of
/// the transitive pass would reject B -> C and silently accept A, so the exact
/// three-node path for A is the witness that keeps the transitive pass required.
#[test]
fn shared_stage_transitive_only_cross_stage_witness() {
    let ids = control_ids(3);
    let modules = vec![
        models::module_with(
            &ids[0],
            models::STAGE_LAYER_PLANNING,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            vec![ids[1].clone()],
            Vec::new(),
        ),
        models::module_with(
            &ids[1],
            models::STAGE_LAYER_PLANNING,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            vec![ids[2].clone()],
            Vec::new(),
        ),
        models::plain_module(&ids[2], models::STAGE_GCODE_EMIT),
    ];
    let report = validate_startup_dag(&request(modules, models::fixed_fill_holders(&ids[0])));

    assert_eq!(
        cross_stage_dependencies(&report),
        vec![CrossView {
            module: ids[1].clone(),
            requires: ids[2].clone(),
            module_stage: models::STAGE_LAYER_PLANNING.to_string(),
            required_stage: models::STAGE_GCODE_EMIT.to_string(),
        }],
        "A requires B in its own stage, so A must have no direct cross-stage violation"
    );
    assert!(
        cross_stage_dependencies(&report)
            .iter()
            .all(|view| view.module != ids[0]),
        "a direct-only rule would be enough if A also violated the direct rule"
    );

    assert_eq!(
        transitive_dependencies(&report),
        vec![
            (
                ids[0].clone(),
                models::STAGE_LAYER_PLANNING.to_string(),
                models::STAGE_GCODE_EMIT.to_string(),
                vec![ids[0].clone(), ids[1].clone(), ids[2].clone()],
            ),
            (
                ids[1].clone(),
                models::STAGE_LAYER_PLANNING.to_string(),
                models::STAGE_GCODE_EMIT.to_string(),
                vec![ids[1].clone(), ids[2].clone()],
            ),
        ],
        "the transitive pass must report A's exact three-node [A, B, C] path and B's [B, C] path"
    );
}

/// Stable-claim scope is a property of each conflicting *pair*, not of the
/// object group.
///
/// Root cause: the H-CLAIMS model derived one object-wide region from the
/// lexically first holder and applied it to every pair. Production reports the
/// pair's own first holder's region with `global_layer_index = None`. Previous
/// input: three `perimeter-generator` holders in `(cube, r0, layer 0)`,
/// `(cube, r1, layer 1)` and `(cube, r2, layer 2)`.
#[test]
fn stable_claim_scope_is_reported_per_holder_pair() {
    let ids = control_ids(3);
    let modules = plain_modules(&ids);
    let mut holders = models::fixed_fill_holders(&ids[0]);
    holders.push(models::region_holder(
        models::STABLE_CLAIM_IDS[0],
        &ids[0],
        "cube",
        "r0",
        Some(0),
    ));
    holders.push(models::region_holder(
        models::STABLE_CLAIM_IDS[0],
        &ids[1],
        "cube",
        "r1",
        Some(1),
    ));
    holders.push(models::region_holder(
        models::STABLE_CLAIM_IDS[0],
        &ids[2],
        "cube",
        "r2",
        Some(2),
    ));

    let report = validate_startup_dag(&request(modules, holders));
    let pair = |left: &String, right: &String, region: &str| ClaimView {
        pass: "per-region",
        claim: models::STABLE_CLAIM_IDS[0].to_string(),
        module_a: left.clone(),
        module_b: right.clone(),
        scope: ScopeModel::Region {
            object_id: String::from("cube"),
            region_id: String::from(region),
            layer: None,
        },
    };
    assert_eq!(
        claim_conflicts(&report),
        vec![
            pair(&ids[0], &ids[1], "r0"),
            pair(&ids[0], &ids[2], "r0"),
            pair(&ids[1], &ids[2], "r1"),
        ],
        "each stable-claim pair must retain its own first holder's region"
    );
}

/// The AC-N3 exposure of the claims shrink audit: the same population and
/// witness-preservation code `run_property` runs must be callable from a bare
/// bounds profile, without `PNP_PROPERTY_CONTEXT` or a `PropertyRun`.
///
/// The first call proves the checked-bound rejection still fires before any
/// strategy allocates; the second runs the real audit at the registry's routine
/// bounds (4 modules, 4 claims, 8 observations, 3 layers, 36 text bytes).
#[test]
fn claims_shrink_audit_runs_without_property_context() {
    let error = models::run_shrink_audit(2, 4, 8, 3, 36)
        .expect_err("an unsafe module ceiling must be rejected before allocation");
    assert!(
        error.contains("max_modules"),
        "the rejection must name the offending bound, got {error:?}"
    );

    models::run_shrink_audit(4, 4, 8, 3, 36)
        .expect("the routine H-CLAIMS audit must run without a property context");
}

/// The packet's safety-ceiling extremes plus the live registry's routine
/// profile. Every point must produce every available tagged mode, agree with the
/// real validator and respect the accepted bounds; a measured registry profile
/// anywhere inside the ceiling cannot be narrower than these.
///
/// This is a generator/model compatibility walk, not a registry execution and
/// not a random hit-rate assertion: the case count is fixed, the seed is fixed
/// and every failure names the exact case.
#[test]
fn scheduler_claim_model_ceiling_walk_controls() {
    const WALK_CASES: usize = 192;

    for (max_modules, max_claims, max_observations, max_layers, max_text_bytes) in
        [(3, 1, 2, 2, 36), (4, 4, 8, 3, 36), (7, 8, 16, 4, 64)]
    {
        let limits = models::checked_claim_limits(
            max_modules,
            max_claims,
            max_observations,
            max_layers,
            max_text_bytes,
        )
        .expect("both extremes are inside the packet safety ceilings");
        assert_eq!(
            models::checked_claim_limits(
                max_modules,
                max_claims,
                max_observations,
                max_layers,
                max_text_bytes
            ),
            Ok(limits.clone()),
            "the ceiling check must be deterministic"
        );
        let strategy = models::claim_case_strategy(&limits);
        models::assert_mode_and_verdict_population(&strategy, &limits);
        models::assert_shrink_preserves_witnesses(&strategy, &limits);

        let mut runner = TestRunner::new(Config {
            cases: u32::try_from(WALK_CASES).expect("bounded"),
            max_local_rejects: 0,
            rng_algorithm: RngAlgorithm::ChaCha,
            rng_seed: RngSeed::Fixed(0x0c1a_1155_0000_0008 ^ max_modules),
            failure_persistence: None,
            ..Config::default()
        });
        for _ in 0..WALK_CASES {
            let case = strategy
                .new_tree(&mut runner)
                .expect("the bounded H-CLAIMS strategy must not reject")
                .current();
            if let Err(error) = models::check_claim_case(&case, &limits) {
                panic!(
                    "case {:?} disagrees with the typed model at ceiling \
                     ({max_modules}, {max_claims}, {max_observations}, \
                     {max_layers}, {max_text_bytes}): {error:?}",
                    case.mode
                );
            }
        }
    }
}
