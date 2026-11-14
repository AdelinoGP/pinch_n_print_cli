//! Deterministic controls for charter `H-REGISTRY` (AC-6).
//!
//! These controls drive the real `slicer_scheduler::region_split::
//! aggregate_region_splits` and `slicer_scheduler::region_split::
//! canonical_variant_chain_order` on uniquely bound loaded modules, and the real
//! plan-assembly caller `slicer_scheduler::execution_plan::build_execution_plan`
//! (which stores the aggregate in `ExecutionPlan.aggregated_region_split`).
//!
//! This proves **aggregator + plan-aggregation integration** and the helper's
//! own contract. It does **not** prove helper integration with `RegionMapping`:
//! `canonical_variant_chain_order` has no production caller in this tree, and
//! `commit_region_mapping_builtin` currently supplies the map's name-ordered keys
//! to `enumerate_canonical_chains` (see ADR-0018's amendment). Nothing here
//! reads `PNP_PROPERTY_CONTEXT` or builds a `PropertyRun`.
//!
//! `models` re-mounts the target-private model file relative to this directory
//! (`tests/common/property_scheduler_contracts/`).

// The target-private model file is mounted once by this binary's root; this
// alias keeps the control body readable.
use crate::registry_models as models;

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::PathBuf;
use std::sync::Arc;

use models::{DeclarationSpec, ModuleSpec};
use slicer_ir::{GlobalLayer, RegionKey, RegionPlan, ResolvedConfig};
use slicer_scheduler::region_split::{aggregate_region_splits, canonical_variant_chain_order};
use slicer_scheduler::{
    bind_module_config_view, build_execution_plan, AggregatedRegionSplitEntry, DiagnosticLevel,
    ExecutionModuleBinding, ExecutionPlanRequest, LoadDiagnostic, LoadedModule,
    RegionSplitValueType, SortedStageModules,
};

/// The spec's exact tied-priority message for the four-module witness.
const WITNESS_TIE_MESSAGE: &str = "Tied region-split priority 1000: com.example.alpha (from com.example.axis, com.example.copy), com.example.beta (from com.example.axis). Lex tiebreaker order: com.example.alpha, com.example.beta.";

fn sentinel() -> LoadDiagnostic {
    LoadDiagnostic {
        level: DiagnosticLevel::Error,
        path: PathBuf::from("fixtures/preexisting-diagnostic.wasm"),
        field: Some("preexisting_control".to_owned()),
        message: String::from(
            "preexisting scheduler diagnostic that region-split aggregation must never clear",
        ),
    }
}

fn entry(
    priority: u32,
    value_type: RegionSplitValueType,
    declaring_modules: &[&str],
) -> AggregatedRegionSplitEntry {
    AggregatedRegionSplitEntry {
        priority,
        value_type,
        declaring_modules: declaring_modules
            .iter()
            .map(|module_id| (*module_id).to_owned())
            .collect(),
    }
}

fn witness_expectations() -> BTreeMap<String, AggregatedRegionSplitEntry> {
    let mut expected = BTreeMap::new();
    expected.insert(
        String::from("com.example.alpha"),
        entry(
            1000,
            RegionSplitValueType::Flag,
            &["com.example.axis", "com.example.copy"],
        ),
    );
    expected.insert(
        String::from("com.example.beta"),
        entry(1000, RegionSplitValueType::Flag, &["com.example.axis"]),
    );
    expected.insert(
        String::from("fuzzy_skin"),
        entry(200, RegionSplitValueType::Flag, &["com.example.fuzzy"]),
    );
    expected.insert(
        String::from("material"),
        entry(
            100,
            RegionSplitValueType::ToolIndex,
            &["com.example.material"],
        ),
    );
    expected
}

/// The spec's exact four-module witness.
fn witness_specs() -> Vec<ModuleSpec> {
    vec![
        ModuleSpec::new(
            "com.example.material",
            "fixtures/material.wasm",
            vec![DeclarationSpec::new(
                "material",
                100,
                RegionSplitValueType::ToolIndex,
            )],
        ),
        ModuleSpec::new(
            "com.example.fuzzy",
            "fixtures/fuzzy.wasm",
            vec![DeclarationSpec::new(
                "fuzzy_skin",
                200,
                RegionSplitValueType::Flag,
            )],
        ),
        ModuleSpec::new(
            "com.example.axis",
            "fixtures/axis.wasm",
            vec![
                DeclarationSpec::new("com.example.alpha", 1000, RegionSplitValueType::Flag),
                DeclarationSpec::new("com.example.beta", 1000, RegionSplitValueType::Flag),
            ],
        ),
        ModuleSpec::new(
            "com.example.copy",
            "fixtures/copy.wasm",
            vec![DeclarationSpec::new(
                "com.example.alpha",
                1000,
                RegionSplitValueType::Flag,
            )],
        ),
    ]
}

fn loaded(specs: &[ModuleSpec]) -> Vec<LoadedModule> {
    specs.iter().map(models::loaded_module).collect()
}

fn first_seen_specs() -> (ModuleSpec, ModuleSpec) {
    let axis_only = ModuleSpec::new(
        "com.example.axis",
        "fixtures/axis.wasm",
        vec![DeclarationSpec::new(
            "com.example.alpha",
            1000,
            RegionSplitValueType::Flag,
        )],
    );
    let redecl = ModuleSpec::new(
        "com.example.redecl",
        "fixtures/redecl.wasm",
        vec![DeclarationSpec::new(
            "com.example.alpha",
            1001,
            RegionSplitValueType::CustomString,
        )],
    );
    (axis_only, redecl)
}

#[test]
fn scheduler_registry_oracle_and_warning_controls() {
    models::assert_normative_catalog().expect("charter priority values must match the catalog");

    // ── Four-module witness: priority/name versus semantic-name order ────────
    let specs = witness_specs();
    let modules = loaded(&specs);
    let sentinel = sentinel();
    let mut diagnostics = vec![sentinel.clone()];
    let aggregate = aggregate_region_splits(&modules, &mut diagnostics);

    let raw_keys: Vec<&str> = aggregate.keys().map(String::as_str).collect();
    assert_eq!(
        raw_keys,
        vec![
            "com.example.alpha",
            "com.example.beta",
            "fuzzy_skin",
            "material"
        ],
        "raw BTreeMap iteration must be semantic-name ordered, not priority ordered"
    );

    let helper = canonical_variant_chain_order(&aggregate);
    assert_eq!(
        helper,
        vec![
            "material".to_owned(),
            "fuzzy_skin".to_owned(),
            "com.example.alpha".to_owned(),
            "com.example.beta".to_owned()
        ],
        "the helper must return the separate (priority, name) sequence"
    );

    let expected = witness_expectations();
    assert_eq!(
        aggregate.len(),
        expected.len(),
        "the witness aggregate must contain exactly the four declared semantics"
    );
    for (semantic, expected_entry) in &expected {
        let actual = aggregate
            .get(semantic)
            .unwrap_or_else(|| panic!("witness must aggregate {semantic}"));
        assert_eq!(actual, expected_entry, "entry for {semantic}");
    }

    assert_eq!(
        diagnostics.len(),
        2,
        "the witness must append exactly one tied-priority warning: {diagnostics:?}"
    );
    assert_eq!(
        diagnostics[0], sentinel,
        "the preexisting diagnostic must be unchanged"
    );
    let expected_warning = LoadDiagnostic {
        level: DiagnosticLevel::Warning,
        path: PathBuf::new(),
        field: Some("region_split".to_owned()),
        message: WITNESS_TIE_MESSAGE.to_owned(),
    };
    assert_eq!(
        diagnostics[1], expected_warning,
        "the tied-priority payload must be exactly Warning/empty path/Some(region_split)/verbatim message"
    );

    // The independent model must agree with the hand-computed witness.
    let model = models::expected_model(&specs);
    assert_eq!(model.entries, expected);
    assert_eq!(
        model.raw_keys,
        raw_keys
            .iter()
            .map(|semantic| (*semantic).to_owned())
            .collect::<Vec<_>>()
    );
    assert_eq!(model.helper_order, helper);
    assert_eq!(model.tie_messages, vec![WITNESS_TIE_MESSAGE.to_owned()]);
    models::check_registry_observation(
        &aggregate,
        &helper,
        &diagnostics,
        &sentinel,
        &model,
        "four-module witness",
    )
    .expect("the witness must satisfy the independent observation model");

    // ── No-seed controls ─────────────────────────────────────────────────────
    let mut empty_diagnostics = vec![sentinel.clone()];
    let empty = aggregate_region_splits(&[], &mut empty_diagnostics);
    assert!(
        empty.is_empty(),
        "no modules means no aggregated keys; core semantics must not be seeded"
    );
    assert!(!empty.contains_key("material") && !empty.contains_key("fuzzy_skin"));
    assert_eq!(
        empty_diagnostics,
        vec![sentinel.clone()],
        "empty aggregation must neither emit nor clear diagnostics"
    );

    let material_only = vec![specs[0].clone()];
    let mut material_diagnostics = vec![sentinel.clone()];
    let material_aggregate =
        aggregate_region_splits(&loaded(&material_only), &mut material_diagnostics);
    assert_eq!(
        material_aggregate
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        vec!["material"],
        "a material-only manifest must aggregate exactly material"
    );
    assert_eq!(
        material_aggregate.get("material"),
        Some(&entry(
            100,
            RegionSplitValueType::ToolIndex,
            &["com.example.material"]
        )),
        "material must keep its normative priority and value type"
    );
    assert!(!material_aggregate.contains_key("fuzzy_skin"));
    assert_eq!(
        material_diagnostics,
        vec![sentinel.clone()],
        "material-only aggregation must not warn"
    );

    // ── Duplicate declarer at the aggregation seam ───────────────────────────
    let mut repeated_specs = specs.clone();
    repeated_specs.push(specs[2].clone());
    let mut repeated_diagnostics = vec![sentinel.clone()];
    let repeated_aggregate =
        aggregate_region_splits(&loaded(&repeated_specs), &mut repeated_diagnostics);
    let alpha = repeated_aggregate
        .get("com.example.alpha")
        .expect("alpha must remain aggregated after the seam repetition");
    assert_eq!(
        alpha.declaring_modules,
        vec!["com.example.axis".to_owned(), "com.example.copy".to_owned()],
        "repeating the axis module must not duplicate its declaring id"
    );
    let unique_declarers: BTreeSet<&String> = alpha.declaring_modules.iter().collect();
    assert_eq!(unique_declarers.len(), alpha.declaring_modules.len());
    assert_eq!(
        repeated_aggregate, aggregate,
        "repeating a compatible declarer must not change the aggregate"
    );
    assert_eq!(
        repeated_diagnostics, diagnostics,
        "repeating a compatible declarer must not add or remove warnings"
    );

    // ── Separate cross-module tie (different modules, different semantics) ───
    let cross_specs = vec![
        ModuleSpec::new(
            "com.example.gamma-src",
            "fixtures/gamma.wasm",
            vec![DeclarationSpec::new(
                "com.example.gamma",
                1000,
                RegionSplitValueType::Flag,
            )],
        ),
        ModuleSpec::new(
            "com.example.delta-src",
            "fixtures/delta.wasm",
            vec![DeclarationSpec::new(
                "com.example.delta",
                1000,
                RegionSplitValueType::Flag,
            )],
        ),
    ];
    let mut cross_diagnostics = vec![sentinel.clone()];
    let cross_aggregate = aggregate_region_splits(&loaded(&cross_specs), &mut cross_diagnostics);
    assert_eq!(cross_aggregate.len(), 2);
    let cross_message = "Tied region-split priority 1000: com.example.delta (from com.example.delta-src), com.example.gamma (from com.example.gamma-src). Lex tiebreaker order: com.example.delta, com.example.gamma.";
    assert_eq!(
        cross_diagnostics,
        vec![
            sentinel.clone(),
            LoadDiagnostic {
                level: DiagnosticLevel::Warning,
                path: PathBuf::new(),
                field: Some("region_split".to_owned()),
                message: cross_message.to_owned(),
            }
        ],
        "a cross-module tie must name both semantics and both declaring modules"
    );

    // ── First-seen conflict fixture ──────────────────────────────────────────
    let (axis_only, redecl) = first_seen_specs();
    let mut forward_diagnostics = vec![sentinel.clone()];
    let forward = aggregate_region_splits(
        &loaded(&[axis_only.clone(), redecl.clone()]),
        &mut forward_diagnostics,
    );
    assert_eq!(
        forward.keys().map(String::as_str).collect::<Vec<_>>(),
        vec!["com.example.alpha"]
    );
    assert_eq!(
        forward.get("com.example.alpha"),
        Some(&entry(
            1000,
            RegionSplitValueType::Flag,
            &["com.example.axis"]
        )),
        "the first-seen priority/value type and declaring module must survive"
    );
    assert_eq!(
        forward_diagnostics.len(),
        2,
        "exactly one conflict warning is appended: {forward_diagnostics:?}"
    );
    assert_eq!(forward_diagnostics[0], sentinel);
    let forward_warning = &forward_diagnostics[1];
    assert_eq!(forward_warning.level, DiagnosticLevel::Warning);
    assert_eq!(forward_warning.path, PathBuf::from("fixtures/redecl.wasm"));
    assert_eq!(forward_warning.field.as_deref(), Some("region_split"));
    assert!(
        forward_warning.message.contains("com.example.redecl"),
        "the warning must name the later module: {}",
        forward_warning.message
    );
    assert!(
        forward_warning
            .message
            .contains("priority=1001/value_type=CustomString"),
        "the warning must name the rejected contract: {}",
        forward_warning.message
    );
    assert!(
        forward_warning
            .message
            .contains("priority=1000/value_type=Flag"),
        "the warning must name the kept contract: {}",
        forward_warning.message
    );
    assert!(
        !forward_warning.message.contains("Tied"),
        "a single-semantic conflict must not also warn about a tied priority"
    );
    let forward_model = models::expected_model(&[axis_only.clone(), redecl.clone()]);
    models::check_registry_observation(
        &forward,
        &canonical_variant_chain_order(&forward),
        &forward_diagnostics,
        &sentinel,
        &forward_model,
        "first-seen conflict (axis first)",
    )
    .expect("the forward first-seen conflict must satisfy the independent model");

    let mut reverse_diagnostics = vec![sentinel.clone()];
    let reverse = aggregate_region_splits(
        &loaded(&[redecl.clone(), axis_only.clone()]),
        &mut reverse_diagnostics,
    );
    assert_eq!(
        reverse.get("com.example.alpha"),
        Some(&entry(
            1001,
            RegionSplitValueType::CustomString,
            &["com.example.redecl"]
        )),
        "reversing the declaration order must make redecl the first-seen contract"
    );
    assert_eq!(reverse_diagnostics.len(), 2);
    assert_eq!(reverse_diagnostics[0], sentinel);
    let reverse_warning = &reverse_diagnostics[1];
    assert_eq!(reverse_warning.level, DiagnosticLevel::Warning);
    assert_eq!(reverse_warning.path, PathBuf::from("fixtures/axis.wasm"));
    assert_eq!(reverse_warning.field.as_deref(), Some("region_split"));
    assert!(
        reverse_warning.message.contains("com.example.axis"),
        "the reversed warning must name the later module: {}",
        reverse_warning.message
    );
    assert!(
        reverse_warning
            .message
            .contains("priority=1000/value_type=Flag"),
        "the reversed warning must name the rejected contract: {}",
        reverse_warning.message
    );
    assert!(
        reverse_warning
            .message
            .contains("priority=1001/value_type=CustomString"),
        "the reversed warning must name the kept contract: {}",
        reverse_warning.message
    );
    assert!(
        !reverse_warning.message.contains("Tied"),
        "no tie warning arises in either single-semantic conflict case"
    );
    let reverse_model = models::expected_model(&[redecl, axis_only]);
    models::check_registry_observation(
        &reverse,
        &canonical_variant_chain_order(&reverse),
        &reverse_diagnostics,
        &sentinel,
        &reverse_model,
        "first-seen conflict (redecl first)",
    )
    .expect("the reversed first-seen conflict must satisfy the independent model");

    // ── Mutation controls ────────────────────────────────────────────────────
    // Substituting raw name-ordered keys for the helper result.
    let raw_key_helper: Vec<String> = aggregate.keys().cloned().collect();
    let rejection = models::check_registry_observation(
        &aggregate,
        &raw_key_helper,
        &diagnostics,
        &sentinel,
        &model,
        "raw-key helper mutation",
    )
    .expect_err("substituting raw keys for the helper result must be rejected");
    assert!(
        rejection.contains("canonical_variant_chain_order"),
        "the rejection must name the helper comparison, got: {rejection}"
    );

    // Reversing a tied pair in the helper sequence.
    let mut reversed_helper = helper.clone();
    reversed_helper.swap(2, 3);
    let rejection = models::check_registry_observation(
        &aggregate,
        &reversed_helper,
        &diagnostics,
        &sentinel,
        &model,
        "reversed tied pair mutation",
    )
    .expect_err("reversing a tied pair must be rejected");
    assert!(
        rejection.contains("canonical_variant_chain_order"),
        "the rejection must name the helper comparison, got: {rejection}"
    );

    // Reversing a tied pair's declaring-module list.
    let mut reversed_entry = aggregate.clone();
    if let Some(alpha_entry) = reversed_entry.get_mut("com.example.alpha") {
        alpha_entry.declaring_modules.reverse();
    }
    let rejection = models::check_registry_observation(
        &reversed_entry,
        &helper,
        &diagnostics,
        &sentinel,
        &model,
        "unsorted declaring modules mutation",
    )
    .expect_err("an unsorted declaring-module list must be rejected");
    assert!(
        rejection.contains("entry for") && rejection.contains("com.example.alpha"),
        "the rejection must identify the reversed entry, got: {rejection}"
    );

    // Adding an implicit seed the modules never declared.
    let mut seeded = material_aggregate.clone();
    seeded.insert(
        String::from("fuzzy_skin"),
        entry(200, RegionSplitValueType::Flag, &["com.example.fuzzy"]),
    );
    let material_model = models::expected_model(&material_only);
    let rejection = models::check_registry_observation(
        &seeded,
        &canonical_variant_chain_order(&seeded),
        &material_diagnostics,
        &sentinel,
        &material_model,
        "implicit-seed mutation",
    )
    .expect_err("an implicitly seeded semantic must be rejected");
    assert!(
        rejection.contains("undeclared semantic"),
        "the rejection must name the implicit seed, got: {rejection}"
    );

    // Omitting a declared semantic.
    let mut omitted = aggregate.clone();
    omitted.remove("com.example.beta");
    let rejection = models::check_registry_observation(
        &omitted,
        &canonical_variant_chain_order(&omitted),
        &diagnostics,
        &sentinel,
        &model,
        "omitted-declaration mutation",
    )
    .expect_err("omitting a declaration must be rejected");
    assert!(
        rejection.contains("omits declared semantic") && rejection.contains("com.example.beta"),
        "the rejection must name the omitted semantic, got: {rejection}"
    );

    // Keeping the wrong first-seen contract in a conflict case.
    let mut wrong_first_seen = forward.clone();
    wrong_first_seen.insert(
        String::from("com.example.alpha"),
        entry(
            1001,
            RegionSplitValueType::CustomString,
            &["com.example.redecl"],
        ),
    );
    let rejection = models::check_registry_observation(
        &wrong_first_seen,
        &canonical_variant_chain_order(&wrong_first_seen),
        &forward_diagnostics,
        &sentinel,
        &forward_model,
        "wrong first-seen mutation",
    )
    .expect_err("keeping the later conflicting declaration must be rejected");
    assert!(
        rejection.contains("entry for") && rejection.contains("com.example.alpha"),
        "the rejection must identify the wrong first-seen entry, got: {rejection}"
    );

    // Losing the appended warning.
    let sentinel_only = vec![sentinel.clone()];
    let rejection = models::check_registry_observation(
        &aggregate,
        &helper,
        &sentinel_only,
        &sentinel,
        &model,
        "dropped-warning mutation",
    )
    .expect_err("losing an appended warning must be rejected");
    assert!(
        rejection.contains("diagnostics"),
        "the rejection must name the diagnostics count, got: {rejection}"
    );

    // Clearing the preexisting sentinel.
    let tie_only = vec![diagnostics[1].clone()];
    let rejection = models::check_registry_observation(
        &aggregate,
        &helper,
        &tie_only,
        &sentinel,
        &model,
        "cleared-sentinel mutation",
    )
    .expect_err("clearing the sentinel diagnostic must be rejected");
    assert!(
        rejection.contains("sentinel"),
        "the rejection must name the sentinel, got: {rejection}"
    );

    // ── Real plan assembly ───────────────────────────────────────────────────
    let resolved = ResolvedConfig::default();
    let bindings: Vec<ExecutionModuleBinding> = modules
        .iter()
        .map(|module| ExecutionModuleBinding {
            module: module.clone(),
            config_view: bind_module_config_view(module, &resolved),
        })
        .collect();
    let module_ids: Vec<String> = specs.iter().map(|spec| spec.module_id.clone()).collect();
    let request = ExecutionPlanRequest {
        sorted_stages: vec![SortedStageModules {
            stage_id: models::PROPERTY_STAGE.to_owned(),
            module_ids,
        }],
        module_bindings: bindings,
        global_layers: Arc::new(Vec::<GlobalLayer>::new()),
        region_plans: Arc::new(HashMap::<RegionKey, RegionPlan>::new()),
    };
    let mut plan_diagnostics = vec![sentinel.clone()];
    let plan = build_execution_plan(&request, &mut plan_diagnostics)
        .expect("a plan with four valid unique bindings must build");
    assert!(
        !plan.aggregated_region_split.is_empty(),
        "plan assembly must not omit aggregation for a non-empty binding set"
    );
    assert_eq!(
        plan.aggregated_region_split, expected,
        "ExecutionPlan.aggregated_region_split must match the independently specified witness"
    );
    assert_eq!(
        plan_diagnostics, diagnostics,
        "plan assembly must append the same warning to the caller's diagnostics"
    );
    models::check_registry_observation(
        &plan.aggregated_region_split,
        &canonical_variant_chain_order(&plan.aggregated_region_split),
        &plan_diagnostics,
        &sentinel,
        &model,
        "plan-assembled aggregation",
    )
    .expect("the plan aggregate must satisfy the independent observation model");

    // If assembly silently omitted aggregation, the same comparison must fail.
    let omitted_plan: BTreeMap<String, AggregatedRegionSplitEntry> = BTreeMap::new();
    let no_helper: Vec<String> = Vec::new();
    let rejection = models::check_registry_observation(
        &omitted_plan,
        &no_helper,
        &plan_diagnostics,
        &sentinel,
        &model,
        "omitted-plan-aggregation mutation",
    )
    .expect_err("an empty plan aggregate must be rejected for a non-empty binding set");
    assert!(
        rejection.contains("omits declared semantic"),
        "the rejection must name the omitted semantics, got: {rejection}"
    );

    let empty_request = ExecutionPlanRequest {
        sorted_stages: Vec::new(),
        module_bindings: Vec::new(),
        global_layers: Arc::new(Vec::<GlobalLayer>::new()),
        region_plans: Arc::new(HashMap::<RegionKey, RegionPlan>::new()),
    };
    let mut empty_plan_diagnostics = vec![sentinel.clone()];
    let empty_plan = build_execution_plan(&empty_request, &mut empty_plan_diagnostics)
        .expect("an empty-binding plan must build");
    assert!(
        empty_plan.aggregated_region_split.is_empty(),
        "an empty binding set must produce an empty plan aggregate"
    );
    assert_eq!(
        empty_plan_diagnostics,
        vec![sentinel.clone()],
        "empty plan aggregation must neither emit nor clear diagnostics"
    );
}
