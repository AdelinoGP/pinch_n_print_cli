//! TDD tests for the paint-only / region-split decision at plan-build time.
//!
//! Locks the ADR-0071 contract end-to-end through `build_execution_plan`:
//! - `aggregated_region_split` is exactly the union of loaded module
//!   declarations; no core semantics are seeded implicitly.
//! - `CompiledModuleStatic.region_split_semantics` is empty unless the
//!   manifest set `paint_only = true`; the per-layer dispatch filter
//!   (`module_invocation_allowed_on_layer` in `crates/slicer-runtime`) therefore
//!   allows any non-paint-only module on unpainted layers.
//! - A `paint_only = true` module with a declaration carries exactly its
//!   declared semantics and IS filtered on unpainted layers.
//!
//! The dispatch outcomes are pinned here by evaluating the same predicate the
//! runtime uses (`declared.is_empty() → true`, else require a matching
//! `variant_chain` semantic) over the compiled plan's semantics, so this test
//! file has no runtime dependency (scheduler is wasmtime-free).

#![allow(missing_docs)]

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use slicer_ir::{ConfigView, GlobalLayer, PaintValue, RegionKey, RegionPlan, SemVer, SlicedRegion};
use slicer_scheduler::{
    build_execution_plan, CompiledModuleStatic, DiagnosticLevel, ExecutionModuleBinding,
    ExecutionPlanRequest, LoadedModule, LoadedModuleBuilder, SortedStageModules,
};

// -- fixture helpers ----------------------------------------------------------

fn semver(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}

fn module(id: &str) -> LoadedModuleBuilder {
    LoadedModuleBuilder::new(
        id,
        semver(0, 1, 0),
        "Layer::Infill",
        String::new(),
        PathBuf::from(format!("fixtures/{id}.wasm")),
    )
    .min_host_version(semver(0, 1, 0))
    .min_ir_schema(semver(1, 0, 0))
    .max_ir_schema(semver(5, 0, 0))
    .layer_parallel_safe(true)
}

/// A paint_only community module declaring `com.example.paint-only` at 1500.
fn paint_only_module(id: &str) -> LoadedModule {
    module(id)
        .paint_only(true)
        .region_splits(vec![slicer_scheduler::RegionSplitDeclaration {
            semantic: "com.example.paint-only".to_owned(),
            priority: 1500,
            value_type: slicer_scheduler::RegionSplitValueType::CustomString,
        }])
        .build()
}

/// A dispatch-transparent module that still declares a semantic (the
/// classic/arachne/fuzzy-skin shape): declaration is present, paint_only is
/// false, so it must run on every layer.
fn declaring_but_transparent_module(id: &str) -> LoadedModule {
    module(id)
        .region_splits(vec![slicer_scheduler::RegionSplitDeclaration {
            semantic: "material".to_owned(),
            priority: 100,
            value_type: slicer_scheduler::RegionSplitValueType::ToolIndex,
        }])
        .build()
}

/// A module with no `[[region_split]]` declaration at all.
fn plain_module(id: &str) -> LoadedModule {
    module(id).build()
}

fn bound(module: LoadedModule) -> ExecutionModuleBinding {
    ExecutionModuleBinding {
        module,
        config_view: Arc::new(ConfigView::from_map(HashMap::new())),
    }
}

fn plan_for(modules: Vec<LoadedModule>) -> slicer_scheduler::ExecutionPlan {
    let module_ids: Vec<String> = modules.iter().map(|m| m.id().to_owned()).collect();
    let bindings: Vec<ExecutionModuleBinding> = modules.into_iter().map(bound).collect();
    let request = ExecutionPlanRequest {
        sorted_stages: vec![SortedStageModules {
            stage_id: "Layer::Infill".to_owned(),
            module_ids,
        }],
        module_bindings: bindings,
        global_layers: Arc::new(Vec::<GlobalLayer>::new()),
        region_plans: Arc::new(HashMap::<RegionKey, RegionPlan>::new()),
    };
    build_execution_plan(&request, &mut Vec::new()).expect("plan must build")
}

/// The runtime dispatch predicate, replicated over compiled semantics:
/// empty ⇒ allowed; otherwise allowed iff some region carries a matching
/// `variant_chain` semantic. Mirrors
/// `module_invocation_allowed_on_layer` in `crates/slicer-runtime/src/layer_executor.rs`.
fn dispatch_allowed(declared: &HashSet<String>, regions: &[SlicedRegion]) -> bool {
    if declared.is_empty() {
        return true;
    }
    regions.iter().any(|region| {
        region
            .variant_chain
            .iter()
            .any(|(semantic, _value)| declared.contains(semantic))
    })
}

fn unpainted_region() -> SlicedRegion {
    SlicedRegion {
        variant_chain: vec![],
        ..Default::default()
    }
}

fn painted_region(semantic: &str) -> SlicedRegion {
    SlicedRegion {
        variant_chain: vec![(semantic.to_owned(), PaintValue::Flag(true))],
        ..Default::default()
    }
}

fn compiled_modules(plan: &slicer_scheduler::ExecutionPlan) -> Vec<&CompiledModuleStatic> {
    plan.per_layer_stages
        .iter()
        .flat_map(|stage| stage.modules.iter())
        .collect()
}

fn compiled<'a>(plan: &'a slicer_scheduler::ExecutionPlan, id: &str) -> &'a CompiledModuleStatic {
    compiled_modules(plan)
        .into_iter()
        .find(|module| module.module_id() == id)
        .unwrap_or_else(|| panic!("compiled plan must contain module {id}"))
}

// -- aggregate is declaration-only -------------------------------------------

/// Empty aggregate: with no declaring modules the plan's aggregate must be
/// empty — no implicit core seeding (ADR-0071).
#[test]
fn plan_aggregate_is_empty_without_declarations() {
    let plan = plan_for(vec![plain_module("com.test.plain")]);
    assert!(
        plan.aggregated_region_split.is_empty(),
        "aggregate must contain only declared semantics; got {:?}",
        plan.aggregated_region_split
    );
    assert!(!plan.aggregated_region_split.contains_key("material"));
    assert!(!plan.aggregated_region_split.contains_key("fuzzy_skin"));
}

/// Painted material/fuzzy aggregate: the classic/arachne (material) and
/// fuzzy-skin shapes aggregate their semantics with the canonical priorities
/// and declaring-module lists, while neither module is dispatch-filtered.
#[test]
fn plan_aggregate_holds_painted_material_and_fuzzy_semantics() {
    let classic = declaring_but_transparent_module("com.core.classic-perimeters");
    let fuzzy = module("com.core.fuzzy-skin")
        .region_splits(vec![slicer_scheduler::RegionSplitDeclaration {
            semantic: "fuzzy_skin".to_owned(),
            priority: 200,
            value_type: slicer_scheduler::RegionSplitValueType::Flag,
        }])
        .build();

    let plan = plan_for(vec![classic, fuzzy]);

    let material = plan
        .aggregated_region_split
        .get("material")
        .expect("material must be aggregated from the classic declaration");
    assert_eq!(material.priority, 100);
    assert_eq!(
        material.value_type,
        slicer_scheduler::RegionSplitValueType::ToolIndex
    );
    assert_eq!(
        material.declaring_modules,
        vec!["com.core.classic-perimeters".to_owned()]
    );

    let fuzzy_skin = plan
        .aggregated_region_split
        .get("fuzzy_skin")
        .expect("fuzzy_skin must be aggregated from the fuzzy-skin declaration");
    assert_eq!(fuzzy_skin.priority, 200);
    assert_eq!(
        fuzzy_skin.value_type,
        slicer_scheduler::RegionSplitValueType::Flag
    );
    assert_eq!(
        fuzzy_skin.declaring_modules,
        vec!["com.core.fuzzy-skin".to_owned()]
    );

    // Neither module opted into paint_only: both compiled dispatch sets are
    // empty and both are allowed on an unpainted layer.
    let classic_compiled = compiled(&plan, "com.core.classic-perimeters");
    let fuzzy_compiled = compiled(&plan, "com.core.fuzzy-skin");
    assert!(classic_compiled.region_split_semantics().is_empty());
    assert!(fuzzy_compiled.region_split_semantics().is_empty());
    let unpainted = vec![unpainted_region()];
    assert!(dispatch_allowed(
        classic_compiled.region_split_semantics(),
        &unpainted
    ));
    assert!(dispatch_allowed(
        fuzzy_compiled.region_split_semantics(),
        &unpainted
    ));
}

/// Aggregate deduplicates a semantic declared by two modules at the same
/// contract, listing both declaring modules.
#[test]
fn plan_aggregate_merges_same_semantic_from_two_modules() {
    let a = declaring_but_transparent_module("com.test.a");
    let b = declaring_but_transparent_module("com.test.b");

    let plan = plan_for(vec![a, b]);

    let material = plan
        .aggregated_region_split
        .get("material")
        .expect("material must be aggregated once");
    assert_eq!(
        material.declaring_modules,
        vec!["com.test.a".to_owned(), "com.test.b".to_owned()],
        "both declaring modules must be recorded, sorted"
    );
}

// -- paint_only false on unpainted layers ------------------------------------

/// A module with no declaration is always allowed (classic default).
#[test]
fn plan_paint_transparent_module_allowed_on_unpainted_layer() {
    let plan = plan_for(vec![plain_module("com.test.plain")]);
    let compiled_module = compiled(&plan, "com.test.plain");
    assert!(compiled_module.region_split_semantics().is_empty());
    assert!(dispatch_allowed(
        compiled_module.region_split_semantics(),
        &[unpainted_region()]
    ));
}

/// A module that DECLARES a semantic but does not set `paint_only` compiles to
/// an empty dispatch set and is allowed on a completely unpainted layer. This
/// is the case the correction requires: fuzzy-skin's `apply_to_all` mode and
/// classic/arachne perimeters all run unpainted.
#[test]
fn plan_declaring_but_not_paint_only_module_allowed_on_unpainted_layer() {
    let plan = plan_for(vec![declaring_but_transparent_module("com.test.declares")]);
    let compiled_module = compiled(&plan, "com.test.declares");
    assert!(
        compiled_module.region_split_semantics().is_empty(),
        "declaration alone must not filter dispatch"
    );
    assert!(dispatch_allowed(
        compiled_module.region_split_semantics(),
        &[unpainted_region()]
    ));

    // It declares material, so the aggregate still holds it.
    assert!(plan.aggregated_region_split.contains_key("material"));
}

// -- paint_only true behaviour through compiled semantics --------------------

/// A `paint_only = true` module compiles to exactly its declared semantics:
/// filtered on an unpainted layer, allowed once a matching chain appears.
#[test]
fn plan_paint_only_module_filters_on_unpainted_layer_and_allows_painted() {
    let plan = plan_for(vec![paint_only_module("com.test.paint-required")]);
    let compiled_module = compiled(&plan, "com.test.paint-required");

    let semantics = compiled_module.region_split_semantics();
    assert_eq!(
        semantics.len(),
        1,
        "paint_only module must carry exactly its declared semantic"
    );
    assert!(semantics.contains("com.example.paint-only"));

    assert!(
        !dispatch_allowed(semantics, &[unpainted_region()]),
        "paint_only module must be filtered on a layer with no matching chain"
    );
    assert!(
        !dispatch_allowed(semantics, &[painted_region("material")]),
        "a different semantic must not satisfy the filter"
    );
    assert!(
        dispatch_allowed(semantics, &[painted_region("com.example.paint-only")]),
        "the declared semantic must satisfy the filter"
    );
}

/// Mixed plan: on a single unpainted layer the invocation set is exactly the
/// transparent modules — the paint_only module is skipped.
#[test]
fn plan_mixed_modules_invocation_set_on_unpainted_layer() {
    let plan = plan_for(vec![
        plain_module("com.test.plain"),
        declaring_but_transparent_module("com.test.declares"),
        paint_only_module("com.test.paint-required"),
    ]);

    let unpainted = vec![unpainted_region()];
    let mut invoked: Vec<&str> = compiled_modules(&plan)
        .into_iter()
        .filter(|module| dispatch_allowed(module.region_split_semantics(), &unpainted))
        .map(|module| module.module_id().as_str())
        .collect();
    invoked.sort_unstable();

    assert_eq!(
        invoked,
        vec!["com.test.declares", "com.test.plain"],
        "only the paint_only module may be skipped on an unpainted layer"
    );
}

/// A tied community priority across two paint_only modules produces the
/// non-fatal tied-priority warning while both semantics remain aggregated.
#[test]
fn plan_tied_community_priority_warns_and_keeps_both_semantics() {
    let alpha = module("com.test.alpha")
        .paint_only(true)
        .region_splits(vec![slicer_scheduler::RegionSplitDeclaration {
            semantic: "com.example.alpha".to_owned(),
            priority: 1500,
            value_type: slicer_scheduler::RegionSplitValueType::CustomString,
        }])
        .build();
    let beta = module("com.test.beta")
        .paint_only(true)
        .region_splits(vec![slicer_scheduler::RegionSplitDeclaration {
            semantic: "com.example.beta".to_owned(),
            priority: 1500,
            value_type: slicer_scheduler::RegionSplitValueType::CustomString,
        }])
        .build();

    let bindings: Vec<ExecutionModuleBinding> = vec![bound(alpha), bound(beta)];
    let request = ExecutionPlanRequest {
        sorted_stages: vec![SortedStageModules {
            stage_id: "Layer::Infill".to_owned(),
            module_ids: vec!["com.test.alpha".to_owned(), "com.test.beta".to_owned()],
        }],
        module_bindings: bindings,
        global_layers: Arc::new(Vec::<GlobalLayer>::new()),
        region_plans: Arc::new(HashMap::<RegionKey, RegionPlan>::new()),
    };
    let mut diagnostics = Vec::new();
    let plan = build_execution_plan(&request, &mut diagnostics).expect("plan must build");

    assert_eq!(plan.aggregated_region_split.len(), 2);
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.level == DiagnosticLevel::Warning
            && diagnostic.message.contains("Tied")
            && diagnostic.message.contains("com.example.alpha")
            && diagnostic.message.contains("com.example.beta")
    }));
}

/// Sanity: no diagnostics are emitted for a plain plan.
#[test]
fn plan_with_no_declarations_emits_no_diagnostics() {
    let bindings = vec![bound(plain_module("com.test.plain"))];
    let request = ExecutionPlanRequest {
        sorted_stages: vec![SortedStageModules {
            stage_id: "Layer::Infill".to_owned(),
            module_ids: vec!["com.test.plain".to_owned()],
        }],
        module_bindings: bindings,
        global_layers: Arc::new(Vec::<GlobalLayer>::new()),
        region_plans: Arc::new(HashMap::<RegionKey, RegionPlan>::new()),
    };
    let mut diagnostics = Vec::new();
    build_execution_plan(&request, &mut diagnostics).expect("plan must build");
    assert!(diagnostics.is_empty(), "got {diagnostics:?}");
}
