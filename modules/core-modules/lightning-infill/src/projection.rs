//! Typed `planning_geometry` projection for the lightning module (AC-4).
//!
//! The scene is the module's public diagnostic view of the same in-memory plan
//! the private pieces encode: it is published through the row-06 typed scene
//! API, never as serialized piece bytes, and it is built from the plan plus the
//! committed planning domains rather than from a second planning pass.
//!
//! The committed plan stores clipped, directed two-point tree edges and no
//! parent/child topology (the same limitation the equivalence recorder's
//! "grounding/root witnesses" document). The projection therefore recovers:
//!
//! * `planning_domain` — the exact committed `ExPolygon`s the planner ran on,
//!   holes preserved;
//! * `tree_branch` — one open polyline per recovered branch (contiguous
//!   committed edges are rejoined by [`crate::chain_tree_segments`]);
//! * `tree_root` — the root side of each directed run, i.e. a vertex whose
//!   directed incidence imbalance (`out - in`) is positive;
//! * `grounding` — that root's closest point on its planning domain, the wall
//!   attachment canonical `attach` uses for a seeded tree. The plan carries no
//!   separate grounding point, so this is the recoverable witness rather than
//!   the canonical `GroundingLocation` kind.

use std::collections::BTreeMap;

use slicer_core::geometry::closest_point_on_polygons;
use slicer_ir::{ExPolygon, PaintValue, Point2};
use slicer_sdk::error::ModuleError;
use slicer_sdk::preparation::{
    PreparationProjectionGeometry, PreparationProjectionPrimitive, PreparationProjectionScene,
    PreparationRegion, PreparedPlanOutput,
};

use crate::chain_tree_segments;
use crate::kernel::{KernelPlan, PlanEntry};

/// Declared diagnostic view this module serves.
pub(crate) const VIEW: &str = "planning_geometry";

/// Owner-local scene identity (`lightning-plan`).
pub(crate) const SCENE_ID: &str = "lightning-plan";

/// Exact diagnostic class identifiers the scene declares.
///
/// Classes are diagnostic identities, not extrusion roles; they carry no width.
pub(crate) const CLASSES: [&str; 4] = ["planning_domain", "tree_branch", "tree_root", "grounding"];

/// Map a rejected projection call to this module's generic failure code.
///
/// Deliberately not `57401`/`57410`/`57411`/`57412`: those four name plan-content
/// and decoder failures AC-N2 requires to stay distinct, while a refused sink or
/// a host-rejected scene is a projection contract failure.
fn projection_error(error: impl std::fmt::Display) -> ModuleError {
    ModuleError::fatal(1, format!("lightning projection: {error}"))
}

/// Publish the `lightning-plan` scene when the host requested it.
///
/// `empty` is the `prepared_plan_diagnostic = empty_plan` mode: the scene still
/// declares its four classes and publishes `primitives: []`, because "no
/// primitives" is a successful empty result, not a missing scene. Nothing is
/// derived and no diagnostic collection is allocated when the view was not
/// requested, so capture-off runs pay nothing.
pub(crate) fn publish_projection(
    output: &PreparedPlanOutput,
    plan: &KernelPlan,
    regions: &[PreparationRegion],
    empty: bool,
) -> Result<(), ModuleError> {
    let Some(sink) = output.projection().map_err(projection_error)? else {
        return Ok(());
    };
    let requested = sink.requested_views().map_err(projection_error)?;
    if !requested.iter().any(|view| view == VIEW) {
        return Ok(());
    }
    let primitives = if empty {
        Vec::new()
    } else {
        derive_primitives(plan, regions)
    };
    let scene = PreparationProjectionScene {
        scene_id: SCENE_ID.to_string(),
        view: VIEW.to_string(),
        classes: declared_classes(),
        primitives,
    };
    sink.publish(&scene).map_err(projection_error)
}

/// The scene's declared class set.
///
/// Kept as one function so the empty scene's classes cannot drift from the
/// classes the nonempty derivation emits.
fn declared_classes() -> Vec<String> {
    CLASSES.iter().map(|class| (*class).to_string()).collect()
}

/// Build the scene's primitives from the plan and the committed domains.
fn derive_primitives(
    plan: &KernelPlan,
    regions: &[PreparationRegion],
) -> Vec<PreparationProjectionPrimitive> {
    // Exact full identity -> the committed region outline, so an entry's
    // grounding is projected against the region the planner was invoked for.
    // (`sparse_infill_area` is written by `region_partition` after whole-print
    // preparation, so it is empty here; the outline is the committed region
    // ExPolygon the row-08 contract names.) Domains are emitted for every
    // committed region, including tree-less ones: they are planning input, not
    // a claim about the plan.
    let domains: BTreeMap<String, &Vec<ExPolygon>> = regions
        .iter()
        .map(|region| (region_identity(region), &region.polygons))
        .collect();

    let mut primitives = Vec::new();
    for region in regions {
        let identity = region_identity(region);
        let Some(area) = domains.get(&identity) else {
            continue;
        };
        for (polygon_index, polygon) in area.iter().enumerate() {
            primitives.push(PreparationProjectionPrimitive {
                id: format!("domain:{identity}:p{polygon_index}"),
                global_layer_index: region.global_layer_index,
                class: "planning_domain".to_string(),
                label: Some(source_label(&region.object_id, region.region_id)),
                // Filled with holes preserved: a planning domain is a solid
                // region, and a hole is not "no data".
                geometry: PreparationProjectionGeometry::Polygon {
                    polygon: polygon.clone(),
                    filled: true,
                },
            });
        }
    }

    for entry in &plan.entries {
        let identity = entry_identity(entry);
        let source = source_label(&entry.object_id, entry.region_id);
        // The kernel stores a `u32` schedule index cast to `i32`; the same cast
        // back preserves it. A value that was never a schedule index becomes an
        // out-of-schedule reference, which the host rejects by name.
        let layer = entry.global_layer_index as u32;

        for (branch_index, polyline) in chain_tree_segments(&entry.tree_edge_segments)
            .into_iter()
            .enumerate()
        {
            primitives.push(PreparationProjectionPrimitive {
                id: format!("branch:{identity}:b{branch_index}"),
                global_layer_index: layer,
                class: "tree_branch".to_string(),
                label: Some(source.clone()),
                geometry: PreparationProjectionGeometry::Polyline(polyline),
            });
        }

        let domain = domains.get(&identity);
        for (node_index, root) in root_points(entry).into_iter().enumerate() {
            primitives.push(PreparationProjectionPrimitive {
                id: format!("root:{identity}:n{node_index}"),
                global_layer_index: layer,
                class: "tree_root".to_string(),
                label: Some(format!("{source} root")),
                geometry: PreparationProjectionGeometry::Point(root),
            });
            if let Some(closest) = domain.and_then(|area| closest_point_on_polygons(root, area)) {
                primitives.push(PreparationProjectionPrimitive {
                    id: format!("ground:{identity}:n{node_index}"),
                    global_layer_index: layer,
                    class: "grounding".to_string(),
                    label: Some(format!("{source} grounding")),
                    geometry: PreparationProjectionGeometry::Point(closest.point),
                });
            }
        }
    }

    primitives
}

/// The root side of each directed run: distinct vertices with a positive
/// directed incidence imbalance (`out - in > 0`), in first-appearance order.
///
/// A run start has no incoming committed edge, so it is the only root witness a
/// topology-free edge multiset can recover. Clipping can split a run, in which
/// case the clipped fragment's own start is recovered the same way.
fn root_points(entry: &PlanEntry) -> Vec<Point2> {
    let mut balance: BTreeMap<(i64, i64), i64> = BTreeMap::new();
    let mut first_seen: Vec<(i64, i64)> = Vec::new();
    for [from, to] in &entry.tree_edge_segments {
        let from_key = (from.x, from.y);
        let to_key = (to.x, to.y);
        if !balance.contains_key(&from_key) {
            first_seen.push(from_key);
        }
        *balance.entry(from_key).or_insert(0) += 1;
        if !balance.contains_key(&to_key) {
            first_seen.push(to_key);
        }
        *balance.entry(to_key).or_insert(0) -= 1;
    }
    first_seen
        .into_iter()
        .filter(|key| balance[key] > 0)
        .map(|(x, y)| Point2 { x, y })
        .collect()
}

/// Display identity of an entry, used only for scene-local primitive ids.
fn entry_identity(entry: &PlanEntry) -> String {
    identity(
        entry.global_layer_index,
        &entry.object_id,
        entry.region_id,
        &entry.variant_chain,
    )
}

/// Display identity of a committed region, matching [`entry_identity`].
fn region_identity(region: &PreparationRegion) -> String {
    let chain: Vec<(String, PaintValue)> = region
        .variant_chain
        .iter()
        .map(|(key, value)| (key.clone(), PaintValue::from(value)))
        .collect();
    identity(
        region.global_layer_index as i32,
        &region.object_id,
        region.region_id,
        &chain,
    )
}

/// `layer:object:region:chain` — a stable, human-readable identity.
fn identity(
    global_layer_index: i32,
    object_id: &str,
    region_id: u64,
    chain: &[(String, PaintValue)],
) -> String {
    let mut token = String::new();
    for (index, (key, value)) in chain.iter().enumerate() {
        if index > 0 {
            token.push(',');
        }
        token.push_str(key);
        token.push('=');
        match value {
            PaintValue::Flag(flag) => token.push_str(&format!("flag:{flag}")),
            PaintValue::Scalar(scalar) => {
                token.push_str(&format!("scalar:{:08x}", scalar.to_bits()));
            }
            PaintValue::ToolIndex(index) => token.push_str(&format!("tool:{index}")),
            PaintValue::Custom(text) => token.push_str(&format!("custom:{text}")),
        }
    }
    if token.is_empty() {
        token.push('-');
    }
    format!("{global_layer_index}:{object_id}:{region_id}:{token}")
}

/// Source identity carried on every primitive of one entry.
fn source_label(object_id: &str, region_id: u64) -> String {
    format!("{object_id}/{region_id}")
}
