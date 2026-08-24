//! Support-family closure tests and their acceptance-criteria mapping:
//!
//! - `fixture_invariants`: AC-1, fixture identity and non-empty evidence.
//! - `invalid_geometry_fails`: AC-2, invalid geometry is rejected.
//! - `matched_height_evidence`: AC-2, matched physical-layer evidence.
//! - `differential_evidence`: AC-3, PnP-side two-family invariants (no Orca claim).
//! - `final_gcode_roles`: AC-4, final G-code role evidence.
//! - `supersedes_packet_213_and_task_329`: AC-5, closure supersession.
//! - `task_163b_disposition`: AC-6, PnP-side decline-reason invariants (no Orca claim).
//! - `support_never_intersects_model_at_exact_z`: invariant 1, over tracked `resources/` models.
//! - `accepted_demands_terminate_on_plate_or_model`: invariant 2.
//! - `interface_is_topmost_and_carved_out`: invariant 3.
//! - `no_overhang_mesh_produces_zero_support`: invariant 4, negative guard.
//!
//! The `missing_fixture_is_blocking` gate was removed: it asserted that
//! `std::fs::read` returns `NotFound` for a path it had just constructed to not
//! exist, which tests `std::fs` and nothing about this closure. The real gate is
//! the panic in [`support_test_path`].

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use slicer_core::polygon_ops::intersection_ex;
use slicer_ir::{
    ConfigValue, ExPolygon, MeshIR, Point2, Point3, Polygon, SupportPlanDeclineReason,
    SupportPlanEntry, SupportPlanIR, SupportPlanRole, SupportPlanRoleRegion, SupportPlanSkeleton,
};
use slicer_wasm_host::exact_z_query::ExactZQueryService;
use slicer_wasm_host::support_aggregation::try_aggregate_support_plan_irs_with_diagnostics;

use crate::common::model_cache::cached_load_model;

fn support_test_path() -> std::path::PathBuf {
    let tracked =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/support-family/SupportTest.stl");
    if tracked.exists() {
        return tracked;
    }
    panic!(
        "required support-family fixture is missing at {} (tracked authoritative path); tmp/* is not authoritative",
        tracked.display()
    );
}

/// Tracked path of the Orca-matched config fixture. Mirrors [`support_test_path`]:
/// a missing fixture panics naming the authoritative tracked path rather than
/// silently degrading to an in-process default config.
fn matched_config_path() -> std::path::PathBuf {
    let tracked = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/support-family/orca-matched-config.json");
    if tracked.exists() {
        return tracked;
    }
    panic!(
        "required support-family config fixture is missing at {} (tracked authoritative path); tmp/* is not authoritative",
        tracked.display()
    );
}

/// Loads the tracked Orca-matched config fixture as the config base.
///
/// Before this existed, the closure suite built a two-key `HashMap` in-process
/// (`enable_support` + `support_type`) and left every support tuning key at its
/// module default, so the fixture was tracked but never read by anything.
fn matched_config_base() -> HashMap<String, ConfigValue> {
    let path = matched_config_path();
    let raw = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    let parsed: serde_json::Value = serde_json::from_str(&raw)
        .unwrap_or_else(|error| panic!("parse {}: {error}", path.display()));
    let object = parsed
        .as_object()
        .unwrap_or_else(|| panic!("{} is not a JSON object", path.display()));
    object
        .iter()
        .map(|(key, value)| {
            let converted = json_to_config_value(value).unwrap_or_else(|| {
                panic!(
                    "{}: key `{key}` has unsupported JSON value {value}",
                    path.display()
                )
            });
            (key.clone(), converted)
        })
        .collect()
}

fn json_to_config_value(value: &serde_json::Value) -> Option<ConfigValue> {
    match value {
        serde_json::Value::Bool(flag) => Some(ConfigValue::Bool(*flag)),
        serde_json::Value::String(text) => Some(ConfigValue::String(text.clone())),
        serde_json::Value::Number(number) => {
            if let Some(integer) = number.as_i64() {
                Some(ConfigValue::Int(integer))
            } else {
                number.as_f64().map(ConfigValue::Float)
            }
        }
        serde_json::Value::Array(items) => items
            .iter()
            .map(json_to_config_value)
            .collect::<Option<Vec<_>>>()
            .map(ConfigValue::List),
        _ => None,
    }
}

/// The matched config with the family selection applied as an override on top.
fn matched_config_for(support_enabled: bool, support_type: &str) -> HashMap<String, ConfigValue> {
    let mut config = matched_config_base();
    config.insert(
        "enable_support".to_string(),
        ConfigValue::Bool(support_enabled),
    );
    config.insert(
        "support_type".to_string(),
        ConfigValue::String(support_type.to_string()),
    );
    config
}

fn core_module_dirs() -> Vec<std::path::PathBuf> {
    vec![Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("modules/core-modules")]
}

/// Shared prepass driver: runs the real prepass over `model` with `config`.
fn prepare_model_support(
    model: &Path,
    config: HashMap<String, ConfigValue>,
) -> Result<slicer_runtime::run::PrepassContext, String> {
    if !model.exists() {
        return Err(format!("model is missing at {}", model.display()));
    }
    let mesh = cached_load_model(model);
    slicer_runtime::run::prepare_prepass_context(mesh, config, &core_module_dirs(), true, false)
        .map_err(|error| format!("{} prepass failed: {error:?}", model.display()))
}

fn prepare_support_test(
    support_enabled: bool,
    support_type: &str,
) -> Result<slicer_runtime::run::PrepassContext, String> {
    let mut config = HashMap::new();
    config.insert(
        "enable_support".to_string(),
        ConfigValue::Bool(support_enabled),
    );
    config.insert(
        "support_type".to_string(),
        ConfigValue::String(support_type.to_string()),
    );
    prepare_model_support(&support_test_path(), config)
}

fn run_slice_for_family(support_type: &str) -> Result<String, String> {
    run_slice_for_family_with_interface_layers(support_type, 2, 2)
}

fn run_slice_for_family_with_interface_layers(
    support_type: &str,
    top_layers: i64,
    bottom_layers: i64,
) -> Result<String, String> {
    let mesh = cached_load_model(&support_test_path());
    // Tracked Orca-matched config is the base; the family selection is an override.
    let overrides = matched_config_for(true, support_type);
    let mut overrides = overrides;
    overrides.insert(
        "support_interface_top_layers".to_string(),
        ConfigValue::Int(top_layers),
    );
    overrides.insert(
        "support_interface_bottom_layers".to_string(),
        ConfigValue::Int(bottom_layers),
    );
    let opts = slicer_runtime::run::SliceRunOptions {
        mesh,
        config_overrides: overrides,
        module_dirs: core_module_dirs(),
        ..Default::default()
    };
    let outcome = slicer_runtime::run::run_slice(opts)
        .map_err(|e| format!("run_slice({support_type}) failed: {e}"))?;
    Ok(outcome.gcode_text)
}

fn interface_block_count(gcode: &str) -> usize {
    gcode
        .lines()
        .filter(|line| line.trim_end() == ";TYPE:Support interface")
        .count()
}

/// AC-1: runs both real support families on SupportTest.stl and records
/// positive closure, termination, exact-Z exclusion, and disabled-output evidence.
/// Plate tolerance is derived from layer plan's effective_layer_height, not hardcoded.
pub fn fixture_invariants() -> Result<(), String> {
    let mesh = cached_load_model(&support_test_path());
    let exact_z = ExactZQueryService::new(Arc::clone(&mesh));
    let query_object_id = mesh
        .objects
        .first()
        .map(|object| object.id.as_str())
        .ok_or_else(|| "SupportTest.stl contains no objects".to_string())?;
    for (support_type, family) in [("tree(auto)", "tree"), ("normal(auto)", "traditional")] {
        let context = prepare_support_test(true, support_type)?;
        let analysis = context
            .blackboard
            .support_analysis()
            .ok_or_else(|| format!("{family}: SupportAnalysisIR missing"))?;
        if analysis.candidates.is_empty() {
            return Err(format!("{family}: SupportAnalysisIR has no candidates"));
        }
        let has_family = analysis.family_assignments.values().any(|v| v == family);
        if !has_family {
            return Err(format!(
                "{family}: SupportAnalysisIR missing family assignment for {family}"
            ));
        }
        let plan = context
            .blackboard
            .support_plan()
            .ok_or_else(|| format!("{family}: SupportPlanIR missing"))?;
        if plan.entries.is_empty() {
            let candidate_count = analysis.candidates.len();
            return Err(format!(
                "{family}: SupportPlanIR has no entries; support candidates={candidate_count}"
            ));
        }
        let global_layers = context
            .blackboard
            .layer_plan()
            .ok_or_else(|| format!("{family}: LayerPlanIR missing"))?;
        let effective_height_for = |layer_idx: u32| -> f32 {
            if let Some(layer) = global_layers
                .global_layers
                .iter()
                .find(|l| l.index == layer_idx)
            {
                let prev_z = global_layers
                    .global_layers
                    .iter()
                    .find(|l| l.index == layer_idx.saturating_sub(1))
                    .map(|l| l.z)
                    .unwrap_or(0.0);
                if layer.index == 0 {
                    layer.z
                } else {
                    layer.z - prev_z
                }
            } else {
                0.2
            }
        };
        let mut plate_terminated = false;
        for entry in &plan.entries {
            if entry.decline_reason.is_some()
                || entry.body_ids.is_empty()
                || entry.family_id != family
            {
                return Err(format!("{family}: invalid entry {:?}", entry));
            }
            if entry.anchor_z < 0 || entry.demand_ids.is_empty() {
                return Err(format!(
                    "{family}: invalid anchor/demand in entry {:?}",
                    entry
                ));
            }
            let eff_h = effective_height_for(entry.global_layer_index as u32);
            let tolerance_mm = eff_h * 3.0;
            let skeleton_at_plate = entry
                .skeleton
                .as_ref()
                .is_some_and(|s| s.points.iter().any(|point| point.z <= tolerance_mm));
            let anchor_at_plate = entry.anchor_z <= slicer_ir::mm_to_units(tolerance_mm);
            if skeleton_at_plate || anchor_at_plate {
                plate_terminated = true;
            }
            let query = exact_z.query(
                query_object_id,
                entry.region_id,
                slicer_ir::units_to_mm(entry.anchor_z),
            );
            let query = query.map_err(|error| {
                format!(
                    "{family}: exact-Z query failed for entry {:?}: {error}",
                    entry
                )
            })?;
            for role in &entry.roles {
                if !intersection_ex(&role.regions, &query.occupancy).is_empty() {
                    return Err(format!(
                        "{family}: role {:?} overlaps exact-Z occupancy",
                        role.role
                    ));
                }
            }
        }
        if !plate_terminated {
            let anchor_zs_mm = plan
                .entries
                .iter()
                .map(|entry| slicer_ir::units_to_mm(entry.anchor_z))
                .collect::<Vec<_>>();
            return Err(format!(
                "{family}: no plate-terminated entry; anchor_zs_mm={anchor_zs_mm:?}"
            ));
        }
    }
    let disabled = prepare_support_test(false, "normal(auto)")?;
    if disabled
        .blackboard
        .support_plan()
        .is_some_and(|plan| !plan.entries.is_empty())
    {
        return Err("support-disabled prepass published support entries".into());
    }
    if !disabled
        .blackboard
        .support_analysis()
        .is_some_and(|analysis| analysis.candidates.is_empty())
    {
        return Err("support-disabled prepass has support candidates".into());
    }
    Ok(())
}

/// AC-N1: proves host aggregation drops invalid bodies with structured diagnostics
/// for both exact-Z occupancy and cross-family positive-area overlap.
pub fn invalid_geometry_fails() -> Result<(), String> {
    // exhaustive: synthetic negative-test entry; every field is pinned so the aggregation gate is exercised against a fully-specified body
    let invalid = SupportPlanEntry {
        global_layer_index: 0,
        object_id: "invalid-object".into(),
        region_id: 0,
        family_id: "tree".into(),
        demand_ids: vec!["invalid-demand".into()],
        body_ids: vec!["invalid-body".into()],
        anchor_layer_index: 0,
        anchor_z: 0,
        roles: Vec::new(),
        skeleton: None,
        capabilities: Vec::new(),
        provenance: vec!["synthetic-invalid-geometry".into()],
        decline_reason: Some(SupportPlanDeclineReason::Blocked),
    };
    // exhaustive: synthetic negative-test entry; every field is pinned so the aggregation gate is exercised against a fully-specified body
    let invalid_body = SupportPlanEntry {
        global_layer_index: 2,
        body_ids: vec!["invalid-body".into()],
        demand_ids: vec!["invalid-demand".into()],
        object_id: "invalid-object".into(),
        region_id: 0,
        family_id: "tree".into(),
        anchor_layer_index: 0,
        anchor_z: 0,
        roles: Vec::new(),
        skeleton: None,
        capabilities: Vec::new(),
        provenance: vec!["synthetic-invalid-body".into()],
        decline_reason: None,
    };
    // exhaustive: synthetic negative-test entry; every field is pinned so the aggregation gate is exercised against a fully-specified body
    let valid = SupportPlanEntry {
        global_layer_index: 1,
        object_id: "valid-object".into(),
        region_id: 0,
        family_id: "tree".into(),
        demand_ids: vec!["valid-demand".into()],
        body_ids: vec!["valid-body".into()],
        anchor_layer_index: 0,
        anchor_z: 0,
        roles: vec![SupportPlanRoleRegion {
            role: SupportPlanRole::SupportBody,
            regions: vec![square()],
        }],
        skeleton: Some(SupportPlanSkeleton {
            points: vec![Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            }],
        }),
        capabilities: Vec::new(),
        provenance: vec!["synthetic-valid".into()],
        decline_reason: None,
    };
    let mesh = Arc::new(MeshIR {
        objects: vec![slicer_ir::ObjectMesh {
            id: "valid-object".into(),
            ..Default::default()
        }],
        ..Default::default()
    });
    let (_, diagnostics) = try_aggregate_support_plan_irs_with_diagnostics(
        vec![SupportPlanIR {
            entries: vec![invalid, invalid_body, valid],
            ..Default::default()
        }],
        &ExactZQueryService::new(Arc::clone(&mesh)),
    )
    .map_err(|error| format!("synthetic aggregation failed: {error:?}"))?;
    let diagnostic = diagnostics.iter().find(|diagnostic| {
        diagnostic.code == 1200
            && diagnostic.message.contains("invalid-body")
            && diagnostic.message.contains("invalid-demand")
    });
    if diagnostic.is_none() {
        return Err(format!(
            "diagnostic does not identify invalid body/demand: {diagnostics:?}"
        ));
    }
    // Cross-family positive-area overlap: tree vs traditional bodies overlapping -> both dropped, code 1200 or routing diagnostic.
    // exhaustive: synthetic negative-test entry; every field is pinned so the aggregation gate is exercised against a fully-specified body
    let tree_body = SupportPlanEntry {
        global_layer_index: 5,
        object_id: "overlap-obj".into(),
        region_id: 0,
        family_id: "tree".into(),
        demand_ids: vec!["tree-demand".into()],
        body_ids: vec!["tree-body".into()],
        anchor_layer_index: 5,
        anchor_z: slicer_ir::mm_to_units(1.0),
        roles: vec![SupportPlanRoleRegion {
            role: SupportPlanRole::SupportBody,
            regions: vec![square()],
        }],
        skeleton: None,
        capabilities: Vec::new(),
        provenance: vec!["synthetic-overlap-tree".into()],
        decline_reason: None,
    };
    // exhaustive: synthetic negative-test entry; every field is pinned so the aggregation gate is exercised against a fully-specified body
    let trad_body = SupportPlanEntry {
        global_layer_index: 5,
        object_id: "overlap-obj".into(),
        region_id: 0,
        family_id: "traditional".into(),
        demand_ids: vec!["trad-demand".into()],
        body_ids: vec!["trad-body".into()],
        anchor_layer_index: 5,
        anchor_z: slicer_ir::mm_to_units(1.0),
        roles: vec![SupportPlanRoleRegion {
            role: SupportPlanRole::SupportBody,
            regions: vec![square()],
        }],
        skeleton: None,
        capabilities: Vec::new(),
        provenance: vec!["synthetic-overlap-trad".into()],
        decline_reason: None,
    };
    // Cross-family overlap on same (layer, object, region) is a fatal family conflict per support_aggregation.
    // Prove degraded handling via try_aggregate that must error on conflicting families at same identity.
    let overlap_mesh = Arc::new(MeshIR {
        objects: vec![slicer_ir::ObjectMesh {
            id: "overlap-obj".into(),
            ..Default::default()
        }],
        ..Default::default()
    });
    let overlap_result = try_aggregate_support_plan_irs_with_diagnostics(
        vec![SupportPlanIR {
            entries: vec![tree_body, trad_body],
            ..Default::default()
        }],
        &ExactZQueryService::new(overlap_mesh),
    );
    match overlap_result {
        Ok((retained, overlap_diagnostics)) => {
            let has_overlap_diag = overlap_diagnostics.iter().any(|d| {
                d.message.contains("cross-family") || d.message.contains("positive-area overlap")
            });
            if !has_overlap_diag && !retained.entries.is_empty() {
                return Err(format!(
                    "cross-family overlap not rejected: retained={:?}, diagnostics={:?}",
                    retained
                        .entries
                        .iter()
                        .map(|e| &e.body_ids)
                        .collect::<Vec<_>>(),
                    overlap_diagnostics
                ));
            }
        }
        Err(err) => {
            // Fatal family conflict at same (layer, object, region) is correct degraded behavior.
            if !format!("{err:?}").contains("conflicting_family_id") {
                return Err(format!("unexpected overlap error: {err:?}"));
            }
        }
    }
    Ok(())
}

fn square() -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 { x: 0, y: 0 },
                Point2 { x: 10, y: 0 },
                Point2 { x: 10, y: 10 },
                Point2 { x: 0, y: 10 },
            ],
        },
        holes: Vec::new(),
    }
}

fn workspace_path(relative: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

/// AC-1 / RC-4: the selected support family must survive into region routing.
///
/// `PrePass::LayerPlanning` runs before `PrePass::RegionMapping`, so
/// `restore_layer_plan_configs` has neither a region map nor a prior layer plan
/// to read and every `ActiveRegion.resolved_config` is committed as
/// `ResolvedConfig::default()`. `module_claims_match_active_region` then
/// resolves `select_support_family(None, None)` == `"traditional"` for every
/// region, so a `support-family:tree` module is dispatched on every layer and
/// handed zero regions — silently, because a renderer with nothing to render is
/// not an error.
///
/// This test reads the promoted `ExecutionPlan.global_layers` (the exact value
/// `module_receives_slice_region` and `module_region_index` consume) and
/// requires the routing predicate to select the configured family.
pub fn family_reaches_region_routing() -> Result<(), String> {
    for (family, support_type) in [("tree", "tree(auto)"), ("traditional", "normal(auto)")] {
        let claims = vec![
            "support-generator".to_string(),
            format!("support-family:{family}"),
        ];
        let context = prepare_support_test(true, support_type)?;
        let layers = &context.plan.global_layers;
        if layers.is_empty() {
            return Err(format!(
                "{support_type}: promoted execution plan has no global layers"
            ));
        }
        let matched = layers
            .iter()
            .filter(|layer| {
                layer.active_regions.iter().any(|region| {
                    slicer_scheduler::execution_plan::module_claims_match_active_region(
                        &claims, region,
                    )
                })
            })
            .count();
        if matched == 0 {
            let sample = layers
                .iter()
                .find(|layer| !layer.active_regions.is_empty())
                .and_then(|layer| layer.active_regions.first())
                .map(|region| {
                    format!(
                        "object={} region={} support_type_enum={:?} ext.support_type={:?} ext.support_family={:?}",
                        region.object_id,
                        region.region_id,
                        region.resolved_config.support_type,
                        region.resolved_config.extensions.get("support_type"),
                        region.resolved_config.extensions.get("support_family"),
                    )
                })
                .unwrap_or_else(|| "no active regions on any layer".to_string());
            return Err(format!(
                "{support_type}: no global layer routes any region to `support-family:{family}`; \
                 layers={} first_active_region[{sample}]",
                layers.len(),
            ));
        }
    }
    Ok(())
}

fn pnp_support_evidence(
    family: &str,
    support_type: &str,
) -> Result<slicer_ir::SupportPlanIR, String> {
    let context = prepare_support_test(true, support_type)?;
    let analysis = context
        .blackboard
        .support_analysis()
        .ok_or_else(|| format!("{family}: SupportAnalysisIR missing"))?;
    if analysis.candidates.is_empty() {
        return Err(format!("{family}: SupportAnalysisIR has no candidates"));
    }
    let plan = context
        .blackboard
        .support_plan()
        .ok_or_else(|| format!("{family}: SupportPlanIR missing"))?
        .as_ref()
        .clone();
    if plan.entries.is_empty() {
        return Err(format!("{family}: SupportPlanIR has no entries"));
    }
    for entry in &plan.entries {
        if entry.decline_reason.is_some() {
            return Err(format!("{family}: declined entry {:?}", entry));
        }
        if entry.family_id != family {
            return Err(format!("{family}: wrong family_id {:?}", entry.family_id));
        }
        if entry.body_ids.is_empty() || entry.demand_ids.is_empty() {
            return Err(format!("{family}: missing body/demand {:?}", entry));
        }
        // Interface is carved OUT of the body rather than layered on top of
        // it, so an interface layer legitimately carries no `SupportBody`
        // role. Requiring one on every entry encoded the pre-224 additive
        // model, where body and interface held identical regions and both
        // were extruded over the same area.
        let has_printable = entry.roles.iter().any(|role| !role.regions.is_empty());
        let is_tip =
            entry.skeleton.is_some() && entry.roles.iter().all(|role| role.regions.is_empty());
        if !has_printable && !is_tip {
            return Err(format!(
                "{family}: entry carries no printable role {:?}",
                entry
            ));
        }
    }
    // A column is not all interface: below the interface band there must be
    // body geometry, or the plan is interface-only and something has gone
    // wrong with the band width.
    if !plan.entries.iter().any(|entry| {
        entry
            .roles
            .iter()
            .any(|role| role.role == SupportPlanRole::SupportBody && !role.regions.is_empty())
    }) {
        return Err(format!(
            "{family}: plan carries no SupportBody geometry at any layer"
        ));
    }
    Ok(plan)
}

pub fn matched_height_evidence() -> Result<(), String> {
    let tree_plan = pnp_support_evidence("tree", "tree(auto)")?;
    let trad_plan = pnp_support_evidence("traditional", "normal(auto)")?;
    // Physical height overlap (mm), not global_layer_index overlap.
    let tree_zs: Vec<f32> = tree_plan
        .entries
        .iter()
        .map(|e| slicer_ir::units_to_mm(e.anchor_z))
        .collect();
    let trad_zs: Vec<f32> = trad_plan
        .entries
        .iter()
        .map(|e| slicer_ir::units_to_mm(e.anchor_z))
        .collect();
    if tree_zs.is_empty() || trad_zs.is_empty() {
        return Err("one family has no height evidence".into());
    }
    let tree_min = tree_zs.iter().cloned().fold(f32::INFINITY, f32::min);
    let tree_max = tree_zs.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let trad_min = trad_zs.iter().cloned().fold(f32::INFINITY, f32::min);
    let trad_max = trad_zs.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let overlap_min = tree_min.max(trad_min);
    let overlap_max = tree_max.min(trad_max);
    // Need overlapping physical window within one layer height tolerance (0.3mm).
    if overlap_min > overlap_max + 0.3 {
        return Err(format!(
            "no overlapping physical heights: tree [{tree_min:.3},{tree_max:.3}] vs traditional [{trad_min:.3},{trad_max:.3}]"
        ));
    }
    // PrePass::SupportGeometry coverage at matched heights is validated via SupportPlanIR entries at those heights.
    Ok(())
}

/// AC-3 (amended to invariant-plus-recorded-inspection): asserts only what this
/// process can honestly observe — that BOTH families produce a non-empty plan
/// whose every entry is attributed to at least one body and one demand, and that
/// every declined entry records a reason.
///
/// It asserts NOTHING about OrcaSlicer. The previous body computed
/// `has_orca` from the presence of two `tmp/*.gcode` files and then ran an empty
/// `if` block, so it could not fail on any Orca-side condition; Orca comparison
/// is a recorded manual inspection, not an assertion in this suite.
pub fn differential_evidence() -> Result<(), String> {
    let mut per_family_heights = Vec::new();
    for (family, support_type) in [("tree", "tree(auto)"), ("traditional", "normal(auto)")] {
        let plan = pnp_support_evidence(family, support_type)?;
        assert_attribution_and_decline_reasons(family, &plan)?;
        let heights: std::collections::BTreeSet<_> =
            plan.entries.iter().map(|entry| entry.anchor_z).collect();
        if heights.is_empty() {
            return Err(format!("{family}: plan carries no anchor heights"));
        }
        per_family_heights.push((family, heights.len(), plan.entries.len()));
    }
    if per_family_heights.len() != 2 {
        return Err(format!(
            "expected evidence from both families, got {per_family_heights:?}"
        ));
    }
    Ok(())
}

/// Shared PnP-side attribution invariant used by AC-3 and AC-6.
///
/// Every entry is either accepted — in which case it must name at least one body
/// and one demand — or declined with a recorded reason. An entry that is empty,
/// unattributed AND undeclined is the silent-drop failure mode this guards.
fn assert_attribution_and_decline_reasons(
    family: &str,
    plan: &slicer_ir::SupportPlanIR,
) -> Result<(), String> {
    if plan.entries.is_empty() {
        return Err(format!("{family}: plan has no entries"));
    }
    for entry in &plan.entries {
        match &entry.decline_reason {
            Some(_) => {
                let has_geometry = entry.roles.iter().any(|role| !role.regions.is_empty());
                if has_geometry {
                    return Err(format!(
                        "{family}: declined entry still carries printable geometry: layer={} obj={} region={} reason={:?}",
                        entry.global_layer_index,
                        entry.object_id,
                        entry.region_id,
                        entry.decline_reason
                    ));
                }
            }
            None => {
                if entry.body_ids.is_empty() || entry.demand_ids.is_empty() {
                    return Err(format!(
                        "{family}: accepted entry is unattributed and records no decline reason: layer={} obj={} region={} bodies={:?} demands={:?}",
                        entry.global_layer_index,
                        entry.object_id,
                        entry.region_id,
                        entry.body_ids,
                        entry.demand_ids
                    ));
                }
            }
        }
    }
    Ok(())
}

/// AC-4: both family selections must carry `SupportBody` and an interface role
/// through `SupportPlanIR`, and must emit the exact markers `;TYPE:Support` and
/// `;TYPE:Support interface` in final G-code produced by the real `run_slice`
/// pipeline.
///
/// Neither family is exempt. An earlier revision of this test checked one
/// marker, for one family, and explicitly exempted tree from the interface
/// requirement — which is how the tree family shipped a plan with only
/// `SupportBody` roles and emitted no support G-code at all while the suite
/// reported green.
pub fn final_gcode_roles() -> Result<(), String> {
    for (family, support_type) in [("tree", "tree(auto)"), ("traditional", "normal(auto)")] {
        let plan = pnp_support_evidence(family, support_type)?;

        let has_body = plan.entries.iter().any(|entry| {
            entry
                .roles
                .iter()
                .any(|role| role.role == SupportPlanRole::SupportBody && !role.regions.is_empty())
        });
        if !has_body {
            return Err(format!("{family}: no SupportBody role in SupportPlanIR"));
        }

        let has_interface = plan.entries.iter().any(|entry| {
            entry.roles.iter().any(|role| {
                matches!(
                    role.role,
                    SupportPlanRole::TopInterface | SupportPlanRole::BottomInterface
                ) && !role.regions.is_empty()
            })
        });
        if !has_interface {
            let mut roles_seen = plan
                .entries
                .iter()
                .flat_map(|entry| entry.roles.iter().map(|role| format!("{:?}", role.role)))
                .collect::<Vec<_>>();
            roles_seen.sort();
            roles_seen.dedup();
            return Err(format!(
                "{family}: no interface role in SupportPlanIR (TopInterface or BottomInterface); roles seen: {roles_seen:?}"
            ));
        }

        // Roles must survive the whole pipeline, not merely exist in the plan.
        let gcode = run_slice_for_family(support_type)?;
        let types_seen = gcode
            .lines()
            .filter(|line| line.starts_with(";TYPE:"))
            .collect::<std::collections::BTreeSet<_>>();
        if !gcode.contains(";TYPE:Support") {
            // Dump enough plan identity to tell "the planner produced nothing"
            // apart from "the renderer never matched what the planner produced",
            // which are the two failure modes this marker has actually had.
            let mut sample = plan
                .entries
                .iter()
                .take(6)
                .map(|entry| {
                    format!(
                        "(layer={} obj={} region={} roles={})",
                        entry.global_layer_index,
                        entry.object_id,
                        entry.region_id,
                        entry.roles.len()
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            if plan.entries.len() > 6 {
                sample.push_str(&format!(", .. ({} entries total)", plan.entries.len()));
            }
            return Err(format!(
                "{family}: PNP G-code missing ;TYPE:Support; gcode_len={} types={types_seen:?} plan={sample}",
                gcode.len()
            ));
        }
        // `;TYPE:Support` is a prefix of `;TYPE:Support interface`, so the
        // interface marker must be matched as its own whole line.
        if !gcode
            .lines()
            .any(|line| line.trim_end() == ";TYPE:Support interface")
        {
            return Err(format!(
                "{family}: PNP G-code missing ;TYPE:Support interface; gcode_len={} types={types_seen:?}",
                gcode.len()
            ));
        }
    }
    interface_layer_count_follows_config()
}

/// Interface coverage must respond to the configured top band rather than to a
/// renderer default. SupportTest terminates on the plate, so its bottom band is
/// intentionally not expected to add interface blocks.
pub fn interface_layer_count_follows_config() -> Result<(), String> {
    for (family, support_type) in [("tree", "tree(auto)"), ("traditional", "normal(auto)")] {
        let one = interface_block_count(&run_slice_for_family_with_interface_layers(
            support_type,
            1,
            0,
        )?);
        let two = interface_block_count(&run_slice_for_family_with_interface_layers(
            support_type,
            2,
            0,
        )?);
        let three = interface_block_count(&run_slice_for_family_with_interface_layers(
            support_type,
            3,
            0,
        )?);
        if (one, two, three) != (1, 2, 3) {
            return Err(format!(
                "{family}: interface block count must equal configured top_layers: 1->{one}, 2->{two}, 3->{three}"
            ));
        }

        let fallback = interface_block_count(&run_slice_for_family_with_interface_layers(
            support_type,
            2,
            -1,
        )?);
        if fallback != two {
            return Err(format!(
                "{family}: negative bottom_layers did not fall back to top_layers: top=2,bottom=0->{two}, top=2,bottom=-1->{fallback}"
            ));
        }
    }
    Ok(())
}

pub fn supersedes_packet_213_and_task_329() -> Result<(), String> {
    let status = fs::read_to_string(workspace_path("docs/07_implementation_status.md"))
        .map_err(|error| format!("read implementation status: {error}"))?;
    let status_lines = status.lines().collect::<Vec<_>>();
    let task_index = status_lines
        .iter()
        .position(|line| line.contains("TASK-329"))
        .ok_or_else(|| "TASK-329 is absent from implementation status".to_string())?;
    let nearby = status_lines
        [task_index.saturating_sub(1)..=(task_index + 1).min(status_lines.len() - 1)]
        .join(" ");
    let recorded_superseded = status_lines.iter().any(|line| {
        line.contains("TASK-329") && (line.contains("SUPERSEDED") || line.contains("supersedes"))
    });
    if !recorded_superseded && !nearby.contains("SUPERSEDED") && !nearby.contains("supersedes") {
        return Err("TASK-329 status does not record supersession".into());
    }
    let packet = workspace_path("docs/spec_packets/213-support-planner-defect-fix/packet.spec.md");
    if packet.exists() {
        let packet_text =
            fs::read_to_string(packet).map_err(|error| format!("read packet: {error}"))?;
        if !packet_text
            .lines()
            .take(10)
            .any(|line| line.trim() == "status: superseded")
        {
            return Err("packet 213 is not marked superseded".into());
        }
    }
    Ok(())
}

/// Recursively collect Rust source file paths under `dir`.
///
/// Only `.rs` files are collected: they are the test sources that could
/// reference an Orca-derived G-code path, and they are the only files under
/// `tests/` guaranteed to be UTF-8 (binary fixtures like `.stl` are not).
fn collect_test_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) -> Result<(), String> {
    let entries =
        fs::read_dir(dir).map_err(|error| format!("read_dir {}: {error}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("read_dir entry {}: {error}", dir.display()))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|error| format!("file_type {}: {error}", path.display()))?;
        if file_type.is_dir() {
            collect_test_files(&path, out)?;
        } else if file_type.is_file() && path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
    Ok(())
}

/// AC-6 amendment, second half: no test may read the Orca-derived G-code
/// references. Orca comparison is a recorded manual inspection, not something
/// the automated suite may depend on, so no test source under `tests/` may
/// reference `SupportTest_*_Orca.gcode` (nor the bare `Orca.gcode`).
///
/// The scan uses a recursive `read_dir` over `tests/` rather than `include_str!`
/// of known files so that a new test file reading an Orca reference is caught
/// without this assertion being updated. The assertion's own file is excluded:
/// it must name the literals it searches for, so it would otherwise trip its own
/// gate.
fn assert_no_test_reads_orca_gcode() -> Result<(), String> {
    const FORBIDDEN: &[&str] = &[
        "SupportTest_Tree_Orca.gcode",
        "SupportTest_Normal_Orca.gcode",
        "Orca.gcode",
    ];
    let tests_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut files = Vec::new();
    collect_test_files(&tests_dir, &mut files)?;
    let mut offenders: Vec<String> = Vec::new();
    for path in files {
        // The assertion's own source necessarily contains the literals it scans
        // for; exclude it from the scan.
        if path.file_name().and_then(|name| name.to_str()) == Some("support_family_closure.rs") {
            continue;
        }
        let contents = fs::read_to_string(&path)
            .map_err(|error| format!("read {}: {error}", path.display()))?;
        for needle in FORBIDDEN {
            if contents.contains(needle) {
                offenders.push(format!("{} references `{needle}`", path.display()));
                break;
            }
        }
    }
    if !offenders.is_empty() {
        return Err(format!(
            "AC-6: a test reads an Orca-derived G-code reference — no test may read \
             SupportTest_*_Orca.gcode: {}",
            offenders.join("; ")
        ));
    }
    Ok(())
}

/// AC-6 (amended to invariant-plus-recorded-inspection): asserts the PnP-side
/// disposition invariants only — both families plan geometry, every entry is
/// attributed, and every declined entry records a reason.
///
/// The previous body probed two `tmp/*.gcode` paths and ran an empty `if`, so no
/// Orca-side condition could fail it. Exact-path parity with OrcaSlicer is never
/// claimed here; termination, coverage, collision and interface structure are
/// covered by the invariant tests below. The amendment's second half is the
/// static self-check [`assert_no_test_reads_orca_gcode`]: no test source under
/// `tests/` may read the Orca-derived G-code references.
pub fn task_163b_disposition() -> Result<(), String> {
    for (family, support_type) in [("tree", "tree(auto)"), ("traditional", "normal(auto)")] {
        let plan = pnp_support_evidence(family, support_type)?;
        assert_attribution_and_decline_reasons(family, &plan)?;
    }
    assert_no_test_reads_orca_gcode()?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Structural support invariants over tracked `resources/` models.
//
// Each test runs the real prepass pipeline (`prepare_model_support`) with the
// tracked Orca-matched config as its base, for BOTH families, and fails on
// violation. Models are chosen for the geometric hazard each exercises.
// ---------------------------------------------------------------------------

/// `(relative path, hazard exercised)`. All four paths are tracked in `resources/`.
const INVARIANT_MODELS: &[(&str, &str)] = &[
    (
        "resources/cube_with_concave_hole_enlarged_standing.obj",
        "wall leakage",
    ),
    ("resources/two_hollow_squares.obj", "multi-island"),
    ("resources/V_standing.obj", "branch merging"),
    ("resources/A_upsidedown.obj", "sharp tail"),
];

const FAMILIES: &[(&str, &str)] = &[("tree", "tree(auto)"), ("traditional", "normal(auto)")];

/// Real-pipeline evidence for one (model, family) pair.
struct InvariantEvidence {
    mesh: Arc<MeshIR>,
    plan: SupportPlanIR,
    /// `global_layer_index` -> physical Z (mm), read from the committed `LayerPlanIR`.
    layer_z_mm: std::collections::BTreeMap<i32, f32>,
    /// First-layer height (mm); the basis for the build-plate tolerance.
    first_layer_height_mm: f32,
}

fn invariant_evidence(model_rel: &str, support_type: &str) -> Result<InvariantEvidence, String> {
    let model = workspace_path(model_rel);
    let mesh = cached_load_model(&model);
    let context = prepare_model_support(&model, matched_config_for(true, support_type))?;
    let layer_plan = context
        .blackboard
        .layer_plan()
        .ok_or_else(|| format!("{model_rel}/{support_type}: LayerPlanIR missing"))?;
    let layer_z_mm: std::collections::BTreeMap<i32, f32> = layer_plan
        .global_layers
        .iter()
        .map(|layer| (layer.index as i32, layer.z))
        .collect();
    let first_layer_height_mm = layer_z_mm.values().cloned().fold(f32::INFINITY, f32::min);
    if !first_layer_height_mm.is_finite() {
        return Err(format!(
            "{model_rel}/{support_type}: layer plan has no layers"
        ));
    }
    let plan = context
        .blackboard
        .support_plan()
        .map(|plan| plan.as_ref().clone())
        .unwrap_or_default();
    Ok(InvariantEvidence {
        mesh,
        plan,
        layer_z_mm,
        first_layer_height_mm,
    })
}

fn accepted_entries(plan: &SupportPlanIR) -> impl Iterator<Item = &SupportPlanEntry> {
    plan.entries
        .iter()
        .filter(|entry| entry.decline_reason.is_none())
}

fn is_interface(role: SupportPlanRole) -> bool {
    matches!(
        role,
        SupportPlanRole::TopInterface | SupportPlanRole::BottomInterface
    )
}

/// Invariant 1: no support geometry may intersect model occupancy at the exact
/// physical Z of the layer the geometry is printed on.
///
/// The Z used is the entry's own `global_layer_index` Z from the committed
/// `LayerPlanIR` — not `anchor_z`, which is where the column *lands*, one or
/// more layers below the geometry being checked.
pub fn support_never_intersects_model_at_exact_z() -> Result<(), String> {
    let mut families_with_geometry = 0usize;
    for (family, support_type) in FAMILIES {
        let mut geometry_seen = false;
        for (model_rel, hazard) in INVARIANT_MODELS {
            let evidence = invariant_evidence(model_rel, support_type)?;
            let exact_z = ExactZQueryService::new(Arc::clone(&evidence.mesh));
            for entry in accepted_entries(&evidence.plan) {
                let z_mm = *evidence
                    .layer_z_mm
                    .get(&entry.global_layer_index)
                    .ok_or_else(|| {
                        format!(
                            "{model_rel} [{hazard}] {family}: entry references global layer {} absent from LayerPlanIR",
                            entry.global_layer_index
                        )
                    })?;
                let query = exact_z
                    .query(entry.object_id.as_str(), entry.region_id, z_mm)
                    .map_err(|error| {
                        format!(
                            "{model_rel} [{hazard}] {family}: exact-Z query at z={z_mm:.4}mm failed: {error}"
                        )
                    })?;
                for role in &entry.roles {
                    if role.regions.is_empty() {
                        continue;
                    }
                    geometry_seen = true;
                    let overlap = intersection_ex(&role.regions, &query.occupancy);
                    if !overlap.is_empty() {
                        return Err(format!(
                            "{model_rel} [{hazard}] {family}: role {:?} at layer {} (z={z_mm:.4}mm) intersects model occupancy \
                             ({} overlapping region(s)); obj={} region={}",
                            role.role,
                            entry.global_layer_index,
                            overlap.len(),
                            entry.object_id,
                            entry.region_id
                        ));
                    }
                }
            }
        }
        if geometry_seen {
            families_with_geometry += 1;
        }
    }
    if families_with_geometry == 0 {
        return Err(
            "no family produced any support geometry on any invariant model; the exact-Z \
             non-intersection check was vacuous"
                .into(),
        );
    }
    Ok(())
}

/// Invariant 2: every accepted demand must terminate on the build plate or on
/// the model — never in mid-air.
///
/// "On the model" is witnessed by non-empty model occupancy at the anchor Z.
/// This is the weaker of the two possible forms (it does not require the
/// occupancy to sit under the column footprint, because a tree branch's XY
/// footprint at its anchor is not its footprint at its tip); it still fails on
/// an anchor floating in empty space, which is the failure mode being guarded.
pub fn accepted_demands_terminate_on_plate_or_model() -> Result<(), String> {
    let mut checked_entries = 0usize;
    for (family, support_type) in FAMILIES {
        for (model_rel, hazard) in INVARIANT_MODELS {
            let evidence = invariant_evidence(model_rel, support_type)?;
            let exact_z = ExactZQueryService::new(Arc::clone(&evidence.mesh));
            // Three first-layer heights of slack, matching `fixture_invariants`.
            let plate_tolerance_mm = evidence.first_layer_height_mm * 3.0;
            for entry in accepted_entries(&evidence.plan) {
                if entry.demand_ids.is_empty() {
                    return Err(format!(
                        "{model_rel} [{hazard}] {family}: accepted entry at layer {} carries no demand",
                        entry.global_layer_index
                    ));
                }
                checked_entries += 1;
                let anchor_z_mm = slicer_ir::units_to_mm(entry.anchor_z);
                if anchor_z_mm <= plate_tolerance_mm {
                    continue; // terminates on the build plate
                }
                let query = exact_z
                    .query(entry.object_id.as_str(), entry.region_id, anchor_z_mm)
                    .map_err(|error| {
                        format!(
                            "{model_rel} [{hazard}] {family}: exact-Z query at anchor z={anchor_z_mm:.4}mm failed: {error}"
                        )
                    })?;
                if query.occupancy.is_empty() {
                    return Err(format!(
                        "{model_rel} [{hazard}] {family}: demand {:?} terminates in mid-air — anchor z={anchor_z_mm:.4}mm \
                         is above the plate tolerance {plate_tolerance_mm:.4}mm and the model has no occupancy there; \
                         layer={} obj={} region={}",
                        entry.demand_ids,
                        entry.global_layer_index,
                        entry.object_id,
                        entry.region_id
                    ));
                }
            }
        }
    }
    if checked_entries == 0 {
        return Err(
            "no accepted support entry on any invariant model for either family; the termination \
             check was vacuous"
                .into(),
        );
    }
    Ok(())
}

/// Invariant 3: the interface is topmost and is carved OUT of the body.
///
/// Assertions per support column, keyed by `(object_id, region_id)`:
/// 1. wherever a layer carries BOTH body and interface, the two MUST be
///    disjoint — canonical `TreeSupport.cpp::draw_circles` does
///    `base_areas = diff_ex(base_areas, roofs)`, so the interface is carved
///    OUT of the body and never layered additively on top of it (the pre-224
///    model). This used to `continue` whenever either set was empty, which
///    made the disjointness check unreachable on a planner that never emits
///    both. The first geometry-bearing layer strictly BELOW a contiguous
///    interface run, on a column that continues below that run, MUST also
///    carry `SupportBody`: that is where `diff_ex(base_areas, roofs)`
///    provably leaves a remainder. It is NOT asserted inside the run —
///    canonical's node dispatch is exclusive, so an all-roof layer has an
///    empty `base_areas` before the carve even runs (see the in-body
///    comment);
/// 2. the topmost geometry-bearing layer of a column carries a `TopInterface`;
/// 3. no interface layer floats above a gap — an interface layer above the
///    column's own bottom must have support geometry on the layer below it.
pub fn interface_is_topmost_and_carved_out() -> Result<(), String> {
    let mut columns_checked = 0usize;
    let mut interface_layers_checked = 0usize;
    for (family, support_type) in FAMILIES {
        for (model_rel, hazard) in INVARIANT_MODELS {
            let evidence = invariant_evidence(model_rel, support_type)?;
            // Body/interface geometry, aggregated per column per layer. Body
            // and interface may arrive on separate entries for the same
            // (object, region, layer), so aggregating first is what makes the
            // carve-out check total instead of per-entry.
            let mut column_layers: std::collections::BTreeMap<
                (String, u64),
                std::collections::BTreeMap<i32, (Vec<ExPolygon>, Vec<ExPolygon>)>,
            > = std::collections::BTreeMap::new();
            for entry in accepted_entries(&evidence.plan) {
                let slot = column_layers
                    .entry((entry.object_id.to_string(), entry.region_id))
                    .or_default()
                    .entry(entry.global_layer_index)
                    .or_default();
                for role in &entry.roles {
                    if role.regions.is_empty() {
                        continue;
                    }
                    if role.role == SupportPlanRole::SupportBody {
                        slot.0.extend(role.regions.iter().cloned());
                    } else if is_interface(role.role) {
                        slot.1.extend(role.regions.iter().cloned());
                    }
                }
            }
            for ((object_id, region_id), layers) in &column_layers {
                let geometry_layers: Vec<i32> = layers
                    .iter()
                    .filter(|(_, (body, interface))| !body.is_empty() || !interface.is_empty())
                    .map(|(&layer, _)| layer)
                    .collect();
                for (&layer, (body, interface)) in layers.iter() {
                    if interface.is_empty() {
                        continue;
                    }
                    // Walk down the contiguous interface run this layer sits in;
                    // the column "continues below" only if it still carries
                    // geometry underneath that whole run.
                    let mut run_bottom = layer;
                    while layers
                        .get(&(run_bottom - 1))
                        .is_some_and(|(_, lower_interface)| !lower_interface.is_empty())
                    {
                        run_bottom -= 1;
                    }
                    // CANONICAL SCOPE: the body-survival check belongs BELOW
                    // the interface run, never inside it.
                    //
                    // This block used to require `SupportBody` on the
                    // interface layer itself whenever the column continued
                    // below the run. Canonical does not guarantee that.
                    // `draw_circles` (`TreeSupport.cpp`) dispatches each node
                    // through an EXCLUSIVE if/else-if/else chain —
                    // `roof_gap_areas` (`distance_to_top < 0`), else
                    // `roof_1st_layer` (`support_roof_layers_below == 1`), else
                    // `roof_areas` / `roof_base_areas`
                    // (`support_roof_layers_below > 1`), else `base_areas` —
                    // so a node that lands in any roof bucket appends NOTHING
                    // to `base_areas`. On a layer whose surviving nodes are
                    // all roof nodes, `base_areas` is already empty when
                    // `base_areas = diff_ex(base_areas, roofs)` runs: there is
                    // no remainder to keep. Requiring a body there asserts the
                    // pre-224 additive model, in which the interface was
                    // layered on top of a body that was still drawn.
                    //
                    // This is the same ruling `54a98e22` applied to the four
                    // sibling assertions in `orca_parity_tdd` AC-4, tree
                    // `distributed_contacts`, and traditional
                    // `contact_area_planning` / `anchored_termination`; this
                    // site was written before that ruling and was missed by
                    // its sweep. What canonical DOES guarantee — and what is
                    // asserted here instead — is that the column keeps
                    // printing a body BELOW the roof band, which is precisely
                    // where `diff_ex(base_areas, roofs)` leaves a remainder.
                    //
                    // NOT the F-3 gate. `54a98e22` established that
                    // `tree_family_tdd::anchored_heights_and_termination` is
                    // the only fixture in either crate producing MIXED
                    // (body + interface) layers, and it is deliberately kept
                    // un-narrowed there. Every fixture reached from here is
                    // uniform, so no `carved.clear()` regression is observable
                    // in it either way.
                    let continues_below = geometry_layers.iter().any(|&lower| lower < run_bottom);
                    if continues_below && layer == run_bottom {
                        let below = geometry_layers
                            .iter()
                            .copied()
                            .filter(|&lower| lower < run_bottom)
                            .max()
                            .expect("`continues_below` proves one exists");
                        let (below_body, _) = layers
                            .get(&below)
                            .expect("`geometry_layers` is derived from `layers`");
                        interface_layers_checked += 1;
                        if below_body.is_empty() {
                            return Err(format!(
                                "{model_rel} [{hazard}] {family}: layer {below} of column obj={object_id} region={region_id} is the first layer BELOW the interface run ending at {run_bottom} and carries no SupportBody, though it carries geometry. Canonical `draw_circles` carves the roof out of the body (`base_areas = diff_ex(base_areas, roofs)`) and keeps the remainder; below the roof band the body is the only thing left to print."
                            ));
                        }
                    }
                    if body.is_empty() {
                        continue;
                    }
                    let overlap = intersection_ex(interface, body);
                    if !overlap.is_empty() {
                        return Err(format!(
                            "{model_rel} [{hazard}] {family}: interface is not carved out of the body — {} overlapping region(s) at layer {layer} obj={object_id} region={region_id}",
                            overlap.len(),
                        ));
                    }
                }
            }

            // Topmost / no-floating-interface are per column.
            let mut columns: std::collections::BTreeMap<
                (String, u64),
                (
                    std::collections::BTreeSet<i32>,
                    std::collections::BTreeSet<i32>,
                ),
            > = std::collections::BTreeMap::new();
            for entry in accepted_entries(&evidence.plan) {
                let has_geometry = entry.roles.iter().any(|role| !role.regions.is_empty());
                if !has_geometry {
                    continue;
                }
                let has_top_interface = entry.roles.iter().any(|role| {
                    role.role == SupportPlanRole::TopInterface && !role.regions.is_empty()
                });
                let column = columns
                    .entry((entry.object_id.to_string(), entry.region_id))
                    .or_default();
                column.0.insert(entry.global_layer_index);
                if has_top_interface {
                    column.1.insert(entry.global_layer_index);
                }
            }
            for ((object_id, region_id), (geometry_layers, top_interface_layers)) in &columns {
                columns_checked += 1;
                let top_layer = *geometry_layers
                    .iter()
                    .next_back()
                    .expect("column has at least one geometry layer");
                if !top_interface_layers.contains(&top_layer) {
                    return Err(format!(
                        "{model_rel} [{hazard}] {family}: topmost support layer {top_layer} of column \
                         obj={object_id} region={region_id} carries no TopInterface; interface layers={top_interface_layers:?}"
                    ));
                }
                let bottom_layer = *geometry_layers
                    .iter()
                    .next()
                    .expect("column has at least one geometry layer");
                for layer in top_interface_layers {
                    if *layer == bottom_layer {
                        continue;
                    }
                    if !geometry_layers.contains(&layer.saturating_sub(1)) {
                        return Err(format!(
                            "{model_rel} [{hazard}] {family}: interface layer {layer} of column \
                             obj={object_id} region={region_id} sits above a layer with no support \
                             (layer {} absent); geometry layers={geometry_layers:?}",
                            layer.saturating_sub(1)
                        ));
                    }
                }
            }
        }
    }
    if columns_checked == 0 {
        return Err(
            "no support column with geometry on any invariant model for either family; the \
             interface-topology check was vacuous"
                .into(),
        );
    }
    if interface_layers_checked == 0 {
        return Err(
            "no interface run had a geometry-bearing layer below it, on any invariant model for either family; the body-survives-the-carve check (canonical `draw_circles`' `base_areas = diff_ex(base_areas, roofs)`) was vacuous"
                .into(),
        );
    }
    Ok(())
}

/// Invariant 4 (negative guard): a NON-EMPTY mesh with no overhang must produce
/// ZERO support entries, for both families.
///
/// `resources/20mm_cube.obj` is the substitution here: none of the four hazard
/// models is overhang-free, and a cube's walls are vertical, so no facet can
/// exceed the 30 degree `support_threshold_angle` in the matched config.
///
/// This guards a specific retracted regression: a planner fallback that
/// fabricated support for every candidate layer of any non-empty mesh. Its own
/// regression test passed an EMPTY mesh, so it exercised only the `is_empty`
/// guard and never the fallback. The mesh non-emptiness assertion below is
/// therefore load-bearing, not a sanity check.
pub fn no_overhang_mesh_produces_zero_support() -> Result<(), String> {
    let model_rel = "resources/20mm_cube.obj";
    for (family, support_type) in FAMILIES {
        let evidence = invariant_evidence(model_rel, support_type)?;
        let triangle_count: usize = evidence
            .mesh
            .objects
            .iter()
            .map(|object| object.mesh.indices.len() / 3)
            .sum();
        if triangle_count == 0 {
            return Err(format!(
                "{model_rel}: mesh is empty ({} object(s)); this test must exercise a NON-EMPTY \
                 overhang-free mesh, not the is_empty guard",
                evidence.mesh.objects.len()
            ));
        }
        let offending: Vec<String> = accepted_entries(&evidence.plan)
            .filter(|entry| entry.roles.iter().any(|role| !role.regions.is_empty()))
            .take(8)
            .map(|entry| {
                format!(
                    "(layer={} obj={} region={} roles={})",
                    entry.global_layer_index,
                    entry.object_id,
                    entry.region_id,
                    entry.roles.len()
                )
            })
            .collect();
        if !offending.is_empty() {
            return Err(format!(
                "{model_rel} {family}: overhang-free mesh ({triangle_count} triangles) produced \
                 support geometry; entries={} sample={offending:?}",
                evidence.plan.entries.len()
            ));
        }
    }
    Ok(())
}
