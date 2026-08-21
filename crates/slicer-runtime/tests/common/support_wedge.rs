#![allow(dead_code)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use slicer_ir::ConfigValue;
use slicer_runtime::run::PrepassContext;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("workspace root must be resolvable")
}

fn wedge_path() -> PathBuf {
    workspace_root()
        .join("resources")
        .join("regression_wedge.stl")
}

fn core_modules_dir() -> PathBuf {
    workspace_root().join("modules").join("core-modules")
}

pub fn prepare_wedge_context(support_enabled: bool) -> PrepassContext {
    prepare_wedge_context_with_overrides(support_enabled, &[])
}

/// Wedge context pinned to the **tree** support family.
///
/// `prepare_wedge_context` sets no `support_type`, so
/// `slicer_ir::canonical_support_family(None)` resolves to `"traditional"` and
/// `traditional-support-planner` emits `skeleton: None` on every entry by
/// construction. Any test that asserts on `SupportPlanEntry::skeleton` — or
/// that dispatches `com.core.tree-support-planner`, whose
/// `if support_family != "tree" { continue; }` guard drops every candidate
/// otherwise — MUST build its fixture through this variant.
pub fn prepare_wedge_context_tree(support_enabled: bool) -> PrepassContext {
    prepare_wedge_context_with_overrides(support_enabled, &tree_family_override())
}

/// The config override that selects the tree family, spelled as
/// `canonical_support_family` accepts it (`tree(auto)`).
pub fn tree_family_override() -> Vec<(&'static str, ConfigValue)> {
    vec![(
        "support_type",
        ConfigValue::String("tree(auto)".to_string()),
    )]
}

pub fn prepare_wedge_context_with_overrides(
    support_enabled: bool,
    overrides: &[(&str, ConfigValue)],
) -> PrepassContext {
    let model = wedge_path();
    assert!(
        model.exists(),
        "regression_wedge.stl must exist at {}",
        model.display()
    );

    let mesh = Arc::new(
        slicer_model_io::load_model(&model).expect("load regression_wedge.stl must succeed"),
    );

    let mut config: HashMap<String, ConfigValue> = HashMap::new();
    config.insert(
        "enable_support".to_string(),
        ConfigValue::Bool(support_enabled),
    );
    for (key, value) in overrides {
        config.insert((*key).to_string(), value.clone());
    }

    let module_dirs = vec![core_modules_dir()];

    let ctx = slicer_runtime::run::prepare_prepass_context(mesh, config, &module_dirs, true, false)
        .expect("prepare_prepass_context must succeed");

    if support_enabled {
        let plan = ctx
            .blackboard
            .support_plan()
            .expect("support_plan must be committed when enable_support=true");
        assert!(
            !plan.entries.is_empty(),
            "enable_support=true but SupportPlanIR.entries is empty (len={}) for fixture {}",
            plan.entries.len(),
            model.display()
        );
    }

    ctx
}

/// A minimal single-entry `SupportPlanIR` covering one `(layer, object,
/// region)` demand with one `SupportBody` role region.
///
/// Packets 220/222 removed the renderers' missing-plan fallback fillers:
/// `traditional-support` and `tree-support` both `continue` when
/// `support_plan_entries_for` returns nothing, so a fixture that dispatches
/// `Layer::Support` against a bare `Blackboard` now commits no `SupportIR` at
/// all. Renderer fixtures must commit a plan first.
///
/// `family_id` must match the renderer under test — both modules raise a
/// family-attribution `ModuleError` on a mismatch.
pub fn single_region_support_plan(
    family_id: &str,
    object_id: &str,
    region_id: u64,
    layer_index: u32,
    layer_z: f32,
    body_region: slicer_ir::ExPolygon,
) -> std::sync::Arc<slicer_ir::SupportPlanIR> {
    std::sync::Arc::new(slicer_ir::SupportPlanIR {
        // exhaustive: support-plan identity fixture; SupportPlanEntry has no Default impl and FRU would let a new plan field default silently
        entries: vec![slicer_ir::SupportPlanEntry {
            global_layer_index: layer_index as i32,
            object_id: object_id.to_string(),
            region_id,
            family_id: family_id.to_string(),
            demand_ids: vec![],
            body_ids: vec![format!("{family_id}-body-{object_id}-{layer_index}")],
            anchor_layer_index: layer_index,
            anchor_z: slicer_ir::mm_to_units(layer_z),
            roles: vec![slicer_ir::SupportPlanRoleRegion {
                role: slicer_ir::SupportPlanRole::SupportBody,
                regions: vec![body_region],
            }],
            skeleton: Some(slicer_ir::SupportPlanSkeleton {
                points: vec![
                    slicer_ir::Point3 {
                        x: 1.0,
                        y: 2.0,
                        z: layer_z,
                    },
                    slicer_ir::Point3 {
                        x: 7.0,
                        y: 8.0,
                        z: layer_z,
                    },
                ],
            }),
            capabilities: vec![],
            provenance: vec![],
            decline_reason: None,
        }],
        ..Default::default()
    })
}

/// A square `ExPolygon` of `size_mm` anchored at the origin, in canonical units.
pub fn square_expolygon(size_mm: f32) -> slicer_ir::ExPolygon {
    let extent = slicer_ir::mm_to_units(size_mm);
    slicer_ir::ExPolygon {
        contour: slicer_ir::Polygon {
            points: vec![
                slicer_ir::Point2 { x: 0, y: 0 },
                slicer_ir::Point2 { x: extent, y: 0 },
                slicer_ir::Point2 {
                    x: extent,
                    y: extent,
                },
                slicer_ir::Point2 { x: 0, y: extent },
            ],
        },
        holes: Vec::new(),
    }
}
