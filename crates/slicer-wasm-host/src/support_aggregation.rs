//! Host-owned support-family aggregation and degraded validation.

use slicer_ir::{
    units_to_mm, ExPolygon, RaftPlan, SupportPlanDeclineReason, SupportPlanEntry, SupportPlanIR,
};
use std::collections::{HashMap, HashSet};

use crate::exact_z_query::ExactZQueryService;

/// Edge length of one deterministic host routing cell, in canonical
/// coordinate units. Routing cells partition feasible space into a fixed grid
/// and bound the territory a single support body may claim: a body must fit
/// within one cell-sized envelope, and the grid cell containing its centroid
/// is its stable routing identity for same-family union.
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
}

/// Inputs to the single host multi-writer support merge point.
pub struct SupportAggregationInput<'a> {
    /// Plans collected from support-family writers.
    pub plans: Vec<SupportPlanIR>,
    /// Host-owned exact-Z query service used for validation.
    pub exact_z: &'a ExactZQueryService,
}

/// What aggregation does when two *different* families claim one
/// `(global_layer_index, object_id, region_id)` identity.
///
/// Packet 223 made this unconditionally fatal, which every infallible caller
/// then turned into a total loss of the aggregate (`unwrap_or_else` yields an
/// empty result, and the prepass mapped it to a fatal module error). Callers
/// that must keep printing choose [`FamilyConflictPolicy::Degrade`]; the one
/// caller that must refuse to publish keeps [`FamilyConflictPolicy::Fail`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FamilyConflictPolicy {
    /// Retain the first-*arriving* claimant, record a
    /// [`DuplicateSupportPlanEntry`] (surfaced as diagnostic code 1202), and
    /// mark the aggregate degraded.
    #[default]
    Degrade,
    /// Abort with [`SupportAggregationError`], publishing nothing.
    Fail,
}

// ORDERING ASYMMETRY (deliberate; no OrcaSlicer basis for either half).
//
// Two different orders are live in this function and they disagree:
//
//   * `Fail` computes its `SupportAggregationError` payload from the SORTED
//     entry list, so `expected_family_id` is whichever family sorts first
//     under `compare_entries` (which keys on `body_ids.iter().min()`, so e.g.
//     "traditional-body" < "tree-body").
//   * `Degrade` retains the first entry in PLAN-ARRIVAL order -- the
//     `(plan_index, entry_index)` ordinal captured before the sort -- because
//     "the first writer to claim a region owns it" is the only rule that is
//     stable against a body being renamed.
//
// Neither order is derived from canonical OrcaSlicer behaviour; canonical has
// no multi-family merge point at all. They are kept apart because
// `mismatched_family_fatal` (slicer-runtime `tests/integration/
// support_family_routing.rs`) pins the sorted-order payload while the two
// degrade tests pin arrival-order retention. Do NOT unify them without first
// amending `mismatched_family_fatal`.
//
// The sort itself remains, and is now purely for output determinism.

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

/// Structured host-owned routing diagnostic identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportRoutingDiagnostics {
    /// Family that produced the rejected body.
    pub family_id: String,
    /// Rejected complete body identity.
    pub body_id: String,
    /// Demand made unmet by the rejection.
    pub demand_id: String,
    /// Stable routing rejection reason.
    pub reason: String,
}

const SUPPORT_OVERLAP_TOLERANCE: i64 = 0;

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
    /// Structured diagnostics for rejected bodies and declined demands.
    pub diagnostics: Vec<SupportRoutingDiagnostics>,
}

/// Fatal identity conflict between support families.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportAggregationError {
    /// Colliding layer identity.
    pub global_layer_index: i32,
    /// Colliding object identity.
    pub object_id: String,
    /// Colliding region identity.
    pub region_id: u64,
    /// Family selected by the first writer.
    pub expected_family_id: String,
    /// Family attempting the conflicting write.
    pub conflicting_family_id: String,
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

/// One aggregate diagnostic plus the index of the input plan it originated
/// from.
///
/// Aggregation is a *multi-writer* merge point: the diagnostics it mints come
/// from several family writers at once, so a flat `Vec<Diagnostic>` loses the
/// only information a caller needs to name the module at fault. The prepass
/// used to attach the whole flat vector to the LAST support-plan writer's
/// audit, which reported e.g. a traditional-planner `NoRoute` decline against
/// `com.core.tree-support-planner` — a module in which `NoRoute` does not
/// appear at all.
#[derive(Debug, Clone)]
pub struct AttributedDiagnostic {
    /// Index into the `plans` slice passed to aggregation, when the producing
    /// plan is recoverable. `None` means "not attributable" — callers must
    /// then avoid naming any specific module rather than guessing one.
    pub plan_index: Option<usize>,
    /// The diagnostic itself.
    pub diagnostic: slicer_ir::Diagnostic,
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
    try_aggregate_support_plans_with_policy(input, FamilyConflictPolicy::Degrade).unwrap_or_else(
        |error| {
            // Unreachable under `Degrade`, which never returns `Err`; kept so
            // the signature stays total.
            let mut result = SupportAggregationResult {
                degraded: true,
                ..SupportAggregationResult::default()
            };
            result.diagnostics.push(SupportRoutingDiagnostics {
                family_id: String::new(),
                body_id: String::new(),
                demand_id: String::new(),
                reason: format!("fatal support family routing mismatch: {error:?}"),
            });
            result
        },
    )
}

/// Fallible aggregate used by the prepass commit seam, which must not publish
/// a plan when two families claim the same source region.
pub fn try_aggregate_support_plans(
    input: SupportAggregationInput<'_>,
) -> Result<SupportAggregationResult, SupportAggregationError> {
    try_aggregate_support_plans_with_policy(input, FamilyConflictPolicy::Fail)
}

/// Aggregate family plans under an explicit [`FamilyConflictPolicy`].
///
/// See the ORDERING ASYMMETRY note above: the `Fail` error payload is computed
/// from the sorted entry list, while `Degrade` retention follows plan-arrival
/// order.
pub fn try_aggregate_support_plans_with_policy(
    input: SupportAggregationInput<'_>,
    conflict_policy: FamilyConflictPolicy,
) -> Result<SupportAggregationResult, SupportAggregationError> {
    let mut result = SupportAggregationResult::default();
    let mut identities = HashMap::new();
    // Arrival ordinal `(plan_index, entry_index)`, captured BEFORE the total
    // sort below. Under `Degrade` this -- not sort position -- decides which
    // family owns a contested identity.
    let mut entries = input
        .plans
        .iter()
        .enumerate()
        .flat_map(|(plan_index, plan)| {
            plan.entries
                .iter()
                .enumerate()
                .map(move |(entry_index, entry)| ((plan_index, entry_index), entry.clone()))
        })
        .collect::<Vec<_>>();
    let mut arrival_owners: HashMap<(i32, String, u64), ((usize, usize), String)> = HashMap::new();
    for (arrival, entry) in &entries {
        let identity = (
            entry.global_layer_index,
            entry.object_id.clone(),
            entry.region_id,
        );
        match arrival_owners.get(&identity) {
            Some((incumbent, _)) if incumbent <= arrival => {}
            _ => {
                arrival_owners.insert(identity, (*arrival, entry.family_id.clone()));
            }
        }
    }
    entries.sort_by(|left, right| compare_entries(&left.1, &right.1));
    for plan in input.plans {
        result.raft_plan = merge_raft_plans(result.raft_plan.take(), plan.raft_plan);
    }
    for (_arrival, entry) in entries {
        let identity = (
            entry.global_layer_index,
            entry.object_id.clone(),
            entry.region_id,
        );
        match conflict_policy {
            FamilyConflictPolicy::Fail => {
                if let Some(first_family_id) = identities.get(&identity).cloned() {
                    if first_family_id != entry.family_id {
                        return Err(SupportAggregationError {
                            global_layer_index: identity.0,
                            object_id: identity.1.clone(),
                            region_id: identity.2,
                            expected_family_id: first_family_id,
                            conflicting_family_id: entry.family_id.clone(),
                        });
                    }
                }
            }
            FamilyConflictPolicy::Degrade => {
                if let Some((_, owner_family_id)) = arrival_owners.get(&identity) {
                    if owner_family_id != &entry.family_id {
                        result.degraded = true;
                        result.duplicates.push(DuplicateSupportPlanEntry {
                            global_layer_index: identity.0,
                            object_id: identity.1.clone(),
                            region_id: identity.2,
                            first_family_id: owner_family_id.clone(),
                            duplicate_family_id: entry.family_id.clone(),
                        });
                        continue;
                    }
                }
            }
        }
        // A repeated identity within ONE family is not a conflict: it is two
        // entries for one region from one writer (body + interface candidates,
        // or two candidates at the same layer). It is combined by
        // `union_same_family_entries` below and carries no diagnostic. Dropping
        // it made whichever role sorted first authoritative for the identity,
        // losing interfaces or body geometry from the other entry.
        identities
            .entry(identity.clone())
            .or_insert_with(|| entry.family_id.clone());
        if let Some(decline_reason) = entry.decline_reason {
            let reason = format!("declined: {decline_reason:?}");
            for body_id in &entry.body_ids {
                for demand_id in &entry.demand_ids {
                    result.diagnostics.push(SupportRoutingDiagnostics {
                        family_id: entry.family_id.clone(),
                        body_id: body_id.clone(),
                        demand_id: demand_id.clone(),
                        reason: reason.clone(),
                    });
                }
            }
            continue;
        }
        match validate_entry(&entry, input.exact_z) {
            None => result.retained.push(entry),
            Some(reason) => {
                result.degraded = true;
                record_rejection(&mut result, &entry, reason);
            }
        }
    }
    // Validation is a *per-body* gate and runs only here, before union.
    // Re-running it on merged groups was a category error: a merged group is by
    // construction not one planner-emitted body, so the per-body routing-cell
    // territory bound does not apply to it, and two legitimately same-`body_id`
    // entries far apart on the plate were dropped wholesale once their union
    // envelope exceeded one cell. Canonical support-island merging (`union_` in
    // OrcaSlicer's `SupportCommon.cpp` / `SupportMaterial.cpp`) imposes no size
    // cap on the merged result. The occupancy predicate is set-monotone -- a
    // union cannot introduce an overlap that was absent from every input -- so
    // re-checking it after merging would be redundant as well.
    union_same_family_entries(&mut result.retained);
    // Cross-family overlap is a FAMILY-ARBITRATION guard, not a plate-wide
    // collision check: it exists because two families that both claim positive
    // area for the same slice of the same object would double-extrude there,
    // and the host cannot decide which one is right. Its domain is therefore
    // the same identity domain as the `(global_layer_index, object_id,
    // region_id)` conflict logic above -- minus `region_id`, since two regions
    // of one object at one layer really can be claimed by two families.
    //
    // It was scoped by NEITHER object nor layer, and it rejects BOTH sides. So
    // two genuinely different print objects that legitimately select different
    // per-object `support_type` values annihilated each other's support the
    // moment their bodies overlapped in XY -- and `entries_overlap` compares
    // only XY polygons, so entries on different layers, which cannot physically
    // collide at all, annihilated each other too. Measured on
    // `resources/bridge_support_enforcers.3mf`: one tree object plus two
    // traditional objects yielded a slice with no `;TYPE:Support` whatsoever.
    let mut rejected = vec![false; result.retained.len()];
    for left in 0..result.retained.len() {
        for right in (left + 1)..result.retained.len() {
            let a = &result.retained[left];
            let b = &result.retained[right];
            if a.object_id == b.object_id
                && a.global_layer_index == b.global_layer_index
                && a.family_id != b.family_id
                && entries_overlap(a, b)
            {
                rejected[left] = true;
                rejected[right] = true;
            }
        }
    }
    if rejected.iter().any(|value| *value) {
        let retained = std::mem::take(&mut result.retained);
        for (index, entry) in retained.into_iter().enumerate() {
            if rejected[index] {
                result.degraded = true;
                for body_id in &entry.body_ids {
                    for demand_id in &entry.demand_ids {
                        result.unmet.push(UnmetSupportDemand {
                            demand_id: demand_id.clone(),
                            body_id: body_id.clone(),
                            reason: "body rejected: cross-family positive-area overlap".into(),
                        });
                        result.diagnostics.push(SupportRoutingDiagnostics {
                            family_id: entry.family_id.clone(),
                            body_id: body_id.clone(),
                            demand_id: demand_id.clone(),
                            reason: "body rejected: cross-family positive-area overlap".into(),
                        });
                    }
                }
            } else {
                result.retained.push(entry);
            }
        }
    }
    Ok(result)
}

fn compare_entries(left: &SupportPlanEntry, right: &SupportPlanEntry) -> std::cmp::Ordering {
    let left_candidate = left
        .body_ids
        .iter()
        .min()
        .or_else(|| left.demand_ids.iter().min());
    let right_candidate = right
        .body_ids
        .iter()
        .min()
        .or_else(|| right.demand_ids.iter().min());
    (
        left.global_layer_index,
        &left.object_id,
        left.region_id,
        left_candidate,
        &left.family_id,
        &left.demand_ids,
        &left.body_ids,
    )
        .cmp(&(
            right.global_layer_index,
            &right.object_id,
            right.region_id,
            right_candidate,
            &right.family_id,
            &right.demand_ids,
            &right.body_ids,
        ))
}

fn validate_entry(entry: &SupportPlanEntry, exact_z: &ExactZQueryService) -> Option<&'static str> {
    exact_z
        .query(
            &entry.object_id,
            entry.region_id,
            units_to_mm(entry.anchor_z),
        )
        .map(|query| {
            if !in_routing_cell(entry) {
                Some("body rejected: routing-cell collision")
            } else if entry.roles.iter().any(|role| {
                role.regions
                    .iter()
                    .any(|body| overlaps_any(body, &query.occupancy))
            }) {
                Some("body rejected: exact-Z occupancy")
            } else {
                None
            }
        })
        .unwrap_or(Some("body rejected: exact-Z query unavailable"))
}

fn record_rejection(result: &mut SupportAggregationResult, entry: &SupportPlanEntry, reason: &str) {
    for body_id in &entry.body_ids {
        for demand_id in &entry.demand_ids {
            result.unmet.push(UnmetSupportDemand {
                demand_id: demand_id.clone(),
                body_id: body_id.clone(),
                reason: reason.into(),
            });
            result.diagnostics.push(SupportRoutingDiagnostics {
                family_id: entry.family_id.clone(),
                body_id: body_id.clone(),
                demand_id: demand_id.clone(),
                reason: reason.into(),
            });
        }
    }
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

/// Combine validated entries that are owned by one family and route through
/// the same body/cell. The first entry supplies scalar attribution; geometry,
/// body identities, and demands are accumulated without duplicates.
fn union_same_family_entries(entries: &mut Vec<SupportPlanEntry>) {
    let mut merged: Vec<SupportPlanEntry> = Vec::new();
    // Routing identity of each merged group, snapshotted when the group is
    // created. Recomputing it from `merged[index]` mid-loop let a group's cell
    // drift as it absorbed members, making the result order-sensitive.
    let mut group_cells: Vec<Option<RoutingCell>> = Vec::new();
    for entry in entries.drain(..) {
        let entry_cell = routing_cell(&entry);
        let matching = merged.iter().enumerate().position(|(index, existing)| {
            existing.family_id == entry.family_id
                && existing.global_layer_index == entry.global_layer_index
                && existing.object_id == entry.object_id
                // Ruling 1 assigns support per source region. Same-family
                // union may combine duplicate writes for one region, but it
                // must not erase distinct tree-assigned regions that share a
                // routing cell.
                && existing.region_id == entry.region_id
                && (same_body(existing, &entry) || group_cells[index] == entry_cell)
        });
        let Some(index) = matching else {
            merged.push(entry);
            group_cells.push(entry_cell);
            continue;
        };
        let existing = &mut merged[index];
        existing.demand_ids.extend(entry.demand_ids);
        existing.body_ids.extend(entry.body_ids);
        for incoming_role in entry.roles {
            if let Some(role) = existing
                .roles
                .iter_mut()
                .find(|role| role.role == incoming_role.role)
            {
                role.regions.extend(incoming_role.regions);
            } else {
                existing.roles.push(incoming_role);
            }
        }
        existing.capabilities.extend(entry.capabilities);
        existing.provenance.extend(entry.provenance);
        dedup_sorted(&mut existing.demand_ids);
        dedup_sorted(&mut existing.body_ids);
        dedup_sorted(&mut existing.capabilities);
        dedup_sorted(&mut existing.provenance);
    }
    *entries = merged;
}

fn same_body(left: &SupportPlanEntry, right: &SupportPlanEntry) -> bool {
    left.body_ids
        .iter()
        .any(|body| right.body_ids.contains(body))
}

fn routing_cell(entry: &SupportPlanEntry) -> Option<RoutingCell> {
    let regions: Vec<&ExPolygon> = entry
        .roles
        .iter()
        .flat_map(|role| role.regions.iter())
        .collect();
    body_bounds(&regions).map(|(minx, maxx, miny, maxy)| {
        RoutingCell::from_centroid((minx + maxx) / 2, (miny + maxy) / 2)
    })
}

fn dedup_sorted(values: &mut Vec<String>) {
    let mut unique = HashSet::new();
    values.retain(|value| unique.insert(value.clone()));
    values.sort();
}

/// Validate one harvested writer result before it is handed to the runtime
/// blackboard. Declined entries are diagnostics only, never renderer input.
pub fn aggregate_support_plan_ir(
    plan: SupportPlanIR,
    exact_z: &ExactZQueryService,
) -> Result<SupportPlanIR, SupportAggregationError> {
    let aggregate = try_aggregate_support_plans(SupportAggregationInput {
        plans: vec![plan.clone()],
        exact_z,
    })?;
    Ok(SupportPlanIR {
        schema_version: plan.schema_version,
        entries: aggregate.retained,
        raft_plan: aggregate.raft_plan,
    })
}

/// Production support harvest result, including host-owned degraded diagnostics.
pub fn aggregate_support_plan_ir_with_diagnostics(
    plan: SupportPlanIR,
    exact_z: &ExactZQueryService,
) -> (SupportPlanIR, Vec<slicer_ir::Diagnostic>) {
    try_aggregate_support_plan_ir_with_diagnostics(plan, exact_z).unwrap_or_else(|error| {
        (
            SupportPlanIR::default(),
            vec![slicer_ir::Diagnostic {
                severity: slicer_ir::DiagnosticSeverity::Error,
                code: 1204,
                layer: None,
                object_id: None,
                message: format!("support family routing mismatch: {error:?}"),
            }],
        )
    })
}

/// Fallible form used by the runtime prepass commit seam.
pub fn try_aggregate_support_plan_ir_with_diagnostics(
    plan: SupportPlanIR,
    exact_z: &ExactZQueryService,
) -> Result<(SupportPlanIR, Vec<slicer_ir::Diagnostic>), SupportAggregationError> {
    try_aggregate_support_plan_irs_with_diagnostics(vec![plan], exact_z)
}

/// Aggregate all harvested family plans at the host multi-writer seam.
pub fn aggregate_support_plan_irs_with_diagnostics(
    plans: Vec<SupportPlanIR>,
    exact_z: &ExactZQueryService,
) -> (SupportPlanIR, Vec<slicer_ir::Diagnostic>) {
    try_aggregate_support_plan_irs_with_diagnostics(plans, exact_z).unwrap_or_else(|error| {
        (
            SupportPlanIR::default(),
            vec![slicer_ir::Diagnostic {
                severity: slicer_ir::DiagnosticSeverity::Error,
                code: 1204,
                layer: None,
                object_id: None,
                message: format!("support family routing mismatch: {error:?}"),
            }],
        )
    })
}

/// Fallible aggregation used when a caller must prevent publication on error.
///
/// Keeps [`FamilyConflictPolicy::Fail`]: a cross-family identity conflict is
/// reported as `Err` and nothing is published.
pub fn try_aggregate_support_plan_irs_with_diagnostics(
    plans: Vec<SupportPlanIR>,
    exact_z: &ExactZQueryService,
) -> Result<(SupportPlanIR, Vec<slicer_ir::Diagnostic>), SupportAggregationError> {
    aggregate_support_plan_irs_with_policy(plans, exact_z, FamilyConflictPolicy::Fail)
}

/// Degrading aggregation used by the runtime prepass commit seam.
///
/// A cross-family identity conflict retains the first-arriving family, emits
/// diagnostic code 1202, and still publishes every other entry, instead of
/// discarding the whole aggregate.
pub fn aggregate_support_plan_irs_degrading_with_diagnostics(
    plans: Vec<SupportPlanIR>,
    exact_z: &ExactZQueryService,
) -> (SupportPlanIR, Vec<slicer_ir::Diagnostic>) {
    aggregate_support_plan_irs_with_policy(plans, exact_z, FamilyConflictPolicy::Degrade)
        .unwrap_or_else(|error| {
            // Unreachable under `Degrade`, which never returns `Err`.
            (
                SupportPlanIR::default(),
                vec![slicer_ir::Diagnostic {
                    severity: slicer_ir::DiagnosticSeverity::Error,
                    code: 1204,
                    layer: None,
                    object_id: None,
                    message: format!("support family routing mismatch: {error:?}"),
                }],
            )
        })
}

/// Degrading aggregation that preserves per-plan attribution for every
/// diagnostic it mints.
///
/// This is the form the prepass uses: it needs to attach each diagnostic to
/// the audit of the module that actually produced the offending plan, not to
/// whichever family writer happened to run last.
pub fn aggregate_support_plan_irs_degrading_with_attributed_diagnostics(
    plans: Vec<SupportPlanIR>,
    exact_z: &ExactZQueryService,
) -> (SupportPlanIR, Vec<AttributedDiagnostic>) {
    aggregate_support_plan_irs_with_policy_attributed(plans, exact_z, FamilyConflictPolicy::Degrade)
        .unwrap_or_else(|error| {
            // Unreachable under `Degrade`, which never returns `Err`.
            (
                SupportPlanIR::default(),
                vec![AttributedDiagnostic {
                    plan_index: None,
                    diagnostic: slicer_ir::Diagnostic {
                        severity: slicer_ir::DiagnosticSeverity::Error,
                        code: 1204,
                        layer: None,
                        object_id: None,
                        message: format!("support family routing mismatch: {error:?}"),
                    },
                }],
            )
        })
}

fn aggregate_support_plan_irs_with_policy(
    plans: Vec<SupportPlanIR>,
    exact_z: &ExactZQueryService,
    conflict_policy: FamilyConflictPolicy,
) -> Result<(SupportPlanIR, Vec<slicer_ir::Diagnostic>), SupportAggregationError> {
    let (plan, attributed) =
        aggregate_support_plan_irs_with_policy_attributed(plans, exact_z, conflict_policy)?;
    Ok((
        plan,
        attributed
            .into_iter()
            .map(|entry| entry.diagnostic)
            .collect(),
    ))
}

fn aggregate_support_plan_irs_with_policy_attributed(
    plans: Vec<SupportPlanIR>,
    exact_z: &ExactZQueryService,
    conflict_policy: FamilyConflictPolicy,
) -> Result<(SupportPlanIR, Vec<AttributedDiagnostic>), SupportAggregationError> {
    let schema_version = plans
        .first()
        .map(|plan| plan.schema_version)
        .unwrap_or_default();
    let aggregate = try_aggregate_support_plans_with_policy(
        SupportAggregationInput {
            plans: plans.clone(),
            exact_z,
        },
        conflict_policy,
    )?;
    // Attribution indices. Aggregation flattens every family's entries into
    // one list, so the post-hoc diagnostics carry only `family_id` / `body_id`
    // rather than a plan ordinal; these maps invert that back to the producing
    // plan. First plan wins for a given key, which is exact as long as a family
    // (and a body identity) is written by a single module -- the property the
    // `(layer, object, region)` conflict logic above already enforces.
    let mut family_to_plan: HashMap<&str, usize> = HashMap::new();
    let mut body_to_plan: HashMap<&str, usize> = HashMap::new();
    for (plan_index, plan) in plans.iter().enumerate() {
        for entry in &plan.entries {
            family_to_plan
                .entry(entry.family_id.as_str())
                .or_insert(plan_index);
            for body_id in &entry.body_ids {
                body_to_plan.entry(body_id.as_str()).or_insert(plan_index);
            }
        }
    }

    let mut diagnostics = aggregate
        .unmet
        .iter()
        .map(|demand| AttributedDiagnostic {
            plan_index: body_to_plan.get(demand.body_id.as_str()).copied(),
            diagnostic: slicer_ir::Diagnostic {
                severity: slicer_ir::DiagnosticSeverity::Warn,
                code: 1200,
                layer: None,
                object_id: None,
                message: format!(
                    "support demand '{}' unmet for body '{}': {}",
                    demand.demand_id, demand.body_id, demand.reason
                ),
            },
        })
        .collect::<Vec<_>>();
    diagnostics.extend(aggregate.diagnostics.iter().map(|d| AttributedDiagnostic {
        plan_index: family_to_plan.get(d.family_id.as_str()).copied(),
        diagnostic: slicer_ir::Diagnostic {
            severity: slicer_ir::DiagnosticSeverity::Warn,
            code: 1203,
            layer: None,
            object_id: None,
            message: format!(
                "support routing: family='{}', body='{}', demand='{}': {}",
                d.family_id, d.body_id, d.demand_id, d.reason
            ),
        },
    }));
    diagnostics.extend(aggregate.duplicates.iter().map(|duplicate| AttributedDiagnostic {
        // The *duplicate* is the rejected write, so the diagnostic belongs to
        // the family that lost the arbitration, not to the incumbent.
        plan_index: family_to_plan.get(duplicate.duplicate_family_id.as_str()).copied(),
        diagnostic: slicer_ir::Diagnostic {
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
        },
    }));
    // Declines are minted straight from the input plans, so the producing plan
    // index is exact here -- no inversion needed. The family is also named in
    // the message so the diagnostic stays self-describing once it is detached
    // from its `AttributedDiagnostic` wrapper (e.g. in a log line).
    for (plan_index, entry) in plans
        .iter()
        .enumerate()
        .flat_map(|(plan_index, plan)| plan.entries.iter().map(move |e| (plan_index, e)))
    {
        if let Some(reason) = entry.decline_reason {
            for demand_id in &entry.demand_ids {
                diagnostics.push(AttributedDiagnostic {
                    plan_index: Some(plan_index),
                    diagnostic: slicer_ir::Diagnostic {
                        severity: slicer_ir::DiagnosticSeverity::Warn,
                        code: 1201,
                        layer: Some(entry.global_layer_index),
                        object_id: Some(entry.object_id.clone()),
                        message: format!(
                            "support demand '{}' declined by family '{}': {:?}",
                            demand_id, entry.family_id, reason
                        ),
                    },
                });
            }
        }
    }
    Ok((
        SupportPlanIR {
            schema_version,
            entries: aggregate.retained,
            raft_plan: aggregate.raft_plan,
        },
        diagnostics,
    ))
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

/// True when a body fits inside *some* routing-cell-sized territory, i.e. its
/// envelope is no larger than one cell on either axis. Routing cells bound how
/// much territory a single body may claim; the body is assigned to the cell
/// that contains it rather than being measured against the absolute grid, so a
/// small body that merely straddles a grid line (notably x = 0 or y = 0, which
/// are cell boundaries) keeps its territory. Only bodies genuinely larger than
/// one cell exceed their permitted territory and are rejected.
///
/// `saturating_sub` is deliberate: a malformed guest plan can place `minx`
/// near `i64::MIN`, and a plain subtraction would panic in debug builds.
fn in_routing_cell(entry: &SupportPlanEntry) -> bool {
    let regions: Vec<&ExPolygon> = entry
        .roles
        .iter()
        .flat_map(|role| role.regions.iter())
        .collect();
    let Some((minx, maxx, miny, maxy)) = body_bounds(&regions) else {
        return true;
    };
    maxx.saturating_sub(minx) <= ROUTING_CELL_SIZE && maxy.saturating_sub(miny) <= ROUTING_CELL_SIZE
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
    others.iter().any(|other| {
        let overlap = slicer_core::polygon_ops::intersection(
            std::slice::from_ref(a),
            std::slice::from_ref(other),
        );
        overlap.iter().map(expolygon_area).sum::<f64>() > SUPPORT_OVERLAP_TOLERANCE as f64
    })
}

fn entries_overlap(a: &SupportPlanEntry, b: &SupportPlanEntry) -> bool {
    let a_regions = a.roles.iter().flat_map(|role| role.regions.iter());
    let b_regions = b.roles.iter().flat_map(|role| role.regions.iter());
    a_regions.clone().any(|left| {
        b_regions.clone().any(|right| {
            let overlap = slicer_core::polygon_ops::intersection(
                std::slice::from_ref(left),
                std::slice::from_ref(right),
            );
            overlap.iter().map(expolygon_area).sum::<f64>() > SUPPORT_OVERLAP_TOLERANCE as f64
        })
    })
}

fn expolygon_area(poly: &ExPolygon) -> f64 {
    fn ring_area(points: &[slicer_ir::Point2]) -> f64 {
        if points.len() < 3 {
            return 0.0;
        }
        points
            .iter()
            .enumerate()
            .map(|(i, a)| {
                let b = &points[(i + 1) % points.len()];
                (a.x as f64) * (b.y as f64) - (b.x as f64) * (a.y as f64)
            })
            .sum::<f64>()
            .abs()
            * 0.5
    }
    ring_area(&poly.contour.points) - poly.holes.iter().map(|h| ring_area(&h.points)).sum::<f64>()
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
