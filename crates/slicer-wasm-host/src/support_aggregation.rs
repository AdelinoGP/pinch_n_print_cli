//! Host-owned support-family aggregation and degraded validation.

use slicer_ir::{
    units_to_mm, ExPolygon, RaftPlan, SupportPlanDeclineReason, SupportPlanEntry, SupportPlanIR,
};
use std::collections::HashMap;

use crate::exact_z_query::ExactZQueryService;

/// Edge length of one deterministic host routing cell, in canonical
/// coordinate units. Routing cells partition feasible space into a fixed grid;
/// the cell containing a body's centroid is its permitted territory.
const ROUTING_CELL_SIZE: i64 = 1 << 20;

/// Deterministic routing cell territory assigned to one support body. The
/// cell is derived purely from body geometry (the grid cell containing the
/// body centroid), so assignment is stable regardless of plan ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RoutingCell {
    x: i64,
    y: i64,
}

impl RoutingCell {
    fn from_centroid(cx: i64, cy: i64) -> Self {
        Self {
            x: cx.div_euclid(ROUTING_CELL_SIZE),
            y: cy.div_euclid(ROUTING_CELL_SIZE),
        }
    }

    fn min_x(&self) -> i64 {
        self.x * ROUTING_CELL_SIZE
    }

    fn max_x(&self) -> i64 {
        self.x * ROUTING_CELL_SIZE + ROUTING_CELL_SIZE
    }

    fn min_y(&self) -> i64 {
        self.y * ROUTING_CELL_SIZE
    }

    fn max_y(&self) -> i64 {
        self.y * ROUTING_CELL_SIZE + ROUTING_CELL_SIZE
    }
}

/// Inputs to the single host multi-writer support merge point.
pub struct SupportAggregationInput<'a> {
    /// Plans collected from support-family writers.
    pub plans: Vec<SupportPlanIR>,
    /// Host-owned exact-Z query service used for validation.
    pub exact_z: &'a ExactZQueryService,
}

/// Structured unmet demand diagnostic emitted when a body is dropped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnmetSupportDemand {
    /// Demand identifier that could not be retained.
    pub demand_id: String,
    /// Support body identifier that was rejected.
    pub body_id: String,
    /// Stable rejection explanation.
    pub reason: String,
}

/// Validated aggregate. Invalid bodies are removed as complete entries.
#[derive(Debug, Default)]
pub struct SupportAggregationResult {
    /// Entries that passed complete-body validation.
    pub retained: Vec<SupportPlanEntry>,
    /// Demand diagnostics for rejected bodies.
    pub unmet: Vec<UnmetSupportDemand>,
    /// Whether at least one body was rejected.
    pub degraded: bool,
    /// Duplicate identities rejected in deterministic input order.
    pub duplicates: Vec<DuplicateSupportPlanEntry>,
    /// Raft metadata merged from all family plans.
    pub raft_plan: Option<RaftPlan>,
}

/// A duplicate `(layer, object, region)` identity found during aggregation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateSupportPlanEntry {
    /// Identity of the support region.
    pub global_layer_index: i32,
    /// Object owning the support region.
    pub object_id: String,
    /// Region inside the object.
    pub region_id: u64,
    /// Family of the first entry.
    pub first_family_id: String,
    /// Family of the rejected duplicate.
    pub duplicate_family_id: String,
}

/// Declined candidates retained as diagnostics, with no renderer/filler output.
#[derive(Debug, Clone)]
pub struct DeclinedSupport {
    /// Demand identifiers associated with the declined candidate.
    pub demand_ids: Vec<String>,
    /// Planner-provided decline reason.
    pub reason: SupportPlanDeclineReason,
}

/// Result of recording planner-declined support candidates.
#[derive(Debug, Default)]
pub struct DeclinedSupportResult {
    /// Structured declined candidates.
    pub declined: Vec<DeclinedSupport>,
    /// Always empty: declined candidates must not become paths.
    pub support_paths: Vec<()>,
}

/// Aggregate all family plans, preserving family attribution and validating
/// every body against exact-Z occupancy before it can reach a renderer.
pub fn aggregate_support_plans(input: SupportAggregationInput<'_>) -> SupportAggregationResult {
    let mut result = SupportAggregationResult::default();
    let mut identities = HashMap::new();
    for plan in input.plans {
        result.raft_plan = merge_raft_plans(result.raft_plan.take(), plan.raft_plan);
        for entry in plan.entries {
            let identity = (
                entry.global_layer_index,
                entry.object_id.clone(),
                entry.region_id,
            );
            if let Some(first_family_id) = identities.get(&identity).cloned() {
                result.degraded = true;
                result.duplicates.push(DuplicateSupportPlanEntry {
                    global_layer_index: identity.0,
                    object_id: identity.1,
                    region_id: identity.2,
                    first_family_id,
                    duplicate_family_id: entry.family_id.clone(),
                });
                continue;
            }
            identities.insert(identity.clone(), entry.family_id.clone());
            if entry.decline_reason.is_some() {
                continue;
            }
            let valid = input
                .exact_z
                .query(
                    &entry.object_id,
                    entry.region_id,
                    units_to_mm(entry.anchor_z),
                )
                .map(|query| {
                    // The whole body must stay inside the deterministic routing
                    // cell derived from its own geometry, and must not collide
                    // with exact-Z model occupancy.
                    in_routing_cell(&entry) && {
                        entry.roles.iter().all(|role| {
                            role.regions
                                .iter()
                                .all(|body| !overlaps_any(body, &query.occupancy))
                        })
                    }
                })
                .unwrap_or(false);
            if valid {
                result.retained.push(entry);
            } else {
                result.degraded = true;
                for body_id in &entry.body_ids {
                    for demand_id in &entry.demand_ids {
                        result.unmet.push(UnmetSupportDemand {
                            demand_id: demand_id.clone(),
                            body_id: body_id.clone(),
                            reason: "body rejected: exact-Z occupancy or routing cell collision"
                                .into(),
                        });
                    }
                }
            }
        }
    }
    result
}

fn merge_raft_plans(current: Option<RaftPlan>, incoming: Option<RaftPlan>) -> Option<RaftPlan> {
    match (current, incoming) {
        (None, None) => None,
        (Some(plan), None) | (None, Some(plan)) => Some(plan),
        (Some(current), Some(incoming)) => Some(RaftPlan {
            raft_layers: current.raft_layers.min(incoming.raft_layers),
            raft_first_layer_density: current
                .raft_first_layer_density
                .min(incoming.raft_first_layer_density),
            base_raft_layers: current.base_raft_layers.min(incoming.base_raft_layers),
            interface_raft_layers: current
                .interface_raft_layers
                .min(incoming.interface_raft_layers),
        }),
    }
}

/// Validate one harvested writer result before it is handed to the runtime
/// blackboard. Declined entries are diagnostics only, never renderer input.
pub fn aggregate_support_plan_ir(
    plan: SupportPlanIR,
    exact_z: &ExactZQueryService,
) -> SupportPlanIR {
    let aggregate = aggregate_support_plans(SupportAggregationInput {
        plans: vec![plan.clone()],
        exact_z,
    });
    SupportPlanIR {
        schema_version: plan.schema_version,
        entries: aggregate.retained,
        raft_plan: aggregate.raft_plan,
    }
}

/// Production support harvest result, including host-owned degraded diagnostics.
pub fn aggregate_support_plan_ir_with_diagnostics(
    plan: SupportPlanIR,
    exact_z: &ExactZQueryService,
) -> (SupportPlanIR, Vec<slicer_ir::Diagnostic>) {
    aggregate_support_plan_irs_with_diagnostics(vec![plan], exact_z)
}

/// Aggregate all harvested family plans at the host multi-writer seam.
pub fn aggregate_support_plan_irs_with_diagnostics(
    plans: Vec<SupportPlanIR>,
    exact_z: &ExactZQueryService,
) -> (SupportPlanIR, Vec<slicer_ir::Diagnostic>) {
    let schema_version = plans
        .first()
        .map(|plan| plan.schema_version)
        .unwrap_or_default();
    let aggregate = aggregate_support_plans(SupportAggregationInput {
        plans: plans.clone(),
        exact_z,
    });
    let mut diagnostics = aggregate
        .unmet
        .into_iter()
        .map(|demand| slicer_ir::Diagnostic {
            severity: slicer_ir::DiagnosticSeverity::Warn,
            code: 1200,
            layer: None,
            object_id: None,
            message: format!(
                "support demand '{}' unmet for body '{}': {}",
                demand.demand_id, demand.body_id, demand.reason
            ),
        })
        .collect::<Vec<_>>();
    diagnostics.extend(aggregate.duplicates.iter().map(|duplicate| slicer_ir::Diagnostic {
        severity: slicer_ir::DiagnosticSeverity::Warn,
        code: 1202,
        layer: Some(duplicate.global_layer_index),
        object_id: Some(duplicate.object_id.clone()),
        message: format!(
            "duplicate support region rejected: layer={}, object='{}', region={}, families '{}' and '{}'",
            duplicate.global_layer_index,
            duplicate.object_id,
            duplicate.region_id,
            duplicate.first_family_id,
            duplicate.duplicate_family_id
        ),
    }));
    for entry in plans.iter().flat_map(|plan| &plan.entries) {
        if let Some(reason) = entry.decline_reason {
            for demand_id in &entry.demand_ids {
                diagnostics.push(slicer_ir::Diagnostic {
                    severity: slicer_ir::DiagnosticSeverity::Warn,
                    code: 1201,
                    layer: Some(entry.global_layer_index),
                    object_id: Some(entry.object_id.clone()),
                    message: format!("support demand '{}' declined: {:?}", demand_id, reason),
                });
            }
        }
    }
    (
        SupportPlanIR {
            schema_version,
            entries: aggregate.retained,
            raft_plan: aggregate.raft_plan,
        },
        diagnostics,
    )
}

/// Record planner declines without synthesizing fallback support geometry.
pub fn aggregate_declined_support_plans(plans: &[SupportPlanIR]) -> DeclinedSupportResult {
    let mut result = DeclinedSupportResult::default();
    for entry in plans.iter().flat_map(|plan| &plan.entries) {
        if let Some(reason) = entry.decline_reason {
            result.declined.push(DeclinedSupport {
                demand_ids: entry.demand_ids.clone(),
                reason,
            });
        }
    }
    result
}

/// True when a body's full geometry stays inside the routing cell derived from
/// its centroid. Bodies spanning more than one cell exceed their permitted
/// territory and are rejected as a routing-cell violation.
fn in_routing_cell(entry: &SupportPlanEntry) -> bool {
    let regions: Vec<&ExPolygon> = entry
        .roles
        .iter()
        .flat_map(|role| role.regions.iter())
        .collect();
    let Some((minx, maxx, miny, maxy)) = body_bounds(&regions) else {
        return true;
    };
    let cell = RoutingCell::from_centroid((minx + maxx) / 2, (miny + maxy) / 2);
    minx >= cell.min_x() && maxx <= cell.max_x() && miny >= cell.min_y() && maxy <= cell.max_y()
}

/// Envelope union across all role regions of a support body.
fn body_bounds(polys: &[&ExPolygon]) -> Option<(i64, i64, i64, i64)> {
    let mut acc: Option<(i64, i64, i64, i64)> = None;
    for poly in polys {
        let Some(b) = bounds(poly) else { continue };
        acc = Some(match acc {
            None => b,
            Some((aminx, amaxx, aminy, amaxy)) => (
                aminx.min(b.0),
                amaxx.max(b.1),
                aminy.min(b.2),
                amaxy.max(b.3),
            ),
        });
    }
    acc
}

fn overlaps_any(a: &ExPolygon, others: &[ExPolygon]) -> bool {
    let Some((aminx, amaxx, aminy, amaxy)) = bounds(a) else {
        return false;
    };
    others
        .iter()
        .filter_map(bounds)
        .any(|(bminx, bmaxx, bminy, bmaxy)| {
            aminx < bmaxx && amaxx > bminx && aminy < bmaxy && amaxy > bminy
        })
}

fn bounds(poly: &ExPolygon) -> Option<(i64, i64, i64, i64)> {
    let first = poly.contour.points.first()?;
    let mut out = (first.x, first.x, first.y, first.y);
    for point in &poly.contour.points[1..] {
        out.0 = out.0.min(point.x);
        out.1 = out.1.max(point.x);
        out.2 = out.2.min(point.y);
        out.3 = out.3.max(point.y);
    }
    Some(out)
}
