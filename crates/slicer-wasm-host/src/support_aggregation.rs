//! Host-owned support-family aggregation and degraded validation.

use slicer_ir::{
    units_to_mm, ExPolygon, RaftPlan, SupportAnalysisIR, SupportPlanDeclineReason,
    SupportPlanEntry, SupportPlanIR,
};
use std::collections::{HashMap, HashSet};

use crate::exact_z_query::ExactZQueryService;
use crate::support_territory::TerritoryClipper;

/// Maximum envelope a single support body may span on either axis, in
/// canonical coordinate units. A body wider or taller than this has escaped
/// the territory one body is permitted to claim and is rejected by
/// [`in_routing_cell`]. It is purely an extent bound: it does not partition
/// space, and it takes no part in deciding which entries merge.
const MAX_BODY_EXTENT_UNITS: i64 = 1 << 20;

/// Inputs to the single host multi-writer support merge point.
pub struct SupportAggregationInput<'a> {
    /// Plans collected from support-family writers.
    pub plans: Vec<SupportPlanIR>,
    /// Host-owned exact-Z query service used for validation.
    pub exact_z: &'a ExactZQueryService,
    /// Committed support analysis, read for its `support_territory` map
    /// (ticket 19). `None`, or an analysis without territory, keeps the
    /// territory-free cross-family guard for every layer.
    pub territory: Option<&'a SupportAnalysisIR>,
    /// Identity of the module that produced each plan, index-parallel to
    /// `plans`. Ownership is a declared property, so the merge point needs to
    /// know *who* wrote a plan, not merely what the plan says about itself.
    pub producers: Vec<SupportPlanProducer>,
}

/// The module behind one entry of [`SupportAggregationInput::plans`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SupportPlanProducer {
    /// Manifest module id, used to name the trespasser in diagnostics.
    pub module_id: String,
    /// Claim strings declared by that module's manifest. A support family may
    /// only be written by a module holding `support-family:<family_id>`.
    pub claims: Vec<String>,
}

/// What aggregation does when an entry trespasses on a region it does not own.
///
/// Packet 223 made conflicts unconditionally fatal, which every infallible
/// caller then turned into a total loss of the aggregate (`unwrap_or_else`
/// yields an empty result, and the prepass mapped it to a fatal module error).
/// Callers that must keep printing choose [`FamilyConflictPolicy::Degrade`];
/// the one caller that must refuse to publish keeps
/// [`FamilyConflictPolicy::Fail`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FamilyConflictPolicy {
    /// Drop the trespassing entry, record an [`OwnershipViolation`] (surfaced
    /// as diagnostic code 1206), and mark the aggregate degraded.
    #[default]
    Degrade,
    /// Abort with [`SupportAggregationError`], publishing nothing.
    Fail,
}

/// Why one entry was refused ownership of the region it wrote.
///
/// Default-deny: an entry is published only when a `family_assignments` row
/// names its family as the region's owner AND its producer holds that family's
/// claim. Everything else — including a host-internal producer/plan length
/// mismatch — denies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnershipReason {
    /// No `family_assignments` row exists for `(object_id, region_id)`, so the
    /// region has no owner and nobody may write it.
    NoAssignment,
    /// The region is owned by a different family than the entry declares.
    WrongFamily {
        /// Family the host assigned the region to.
        owner: String,
    },
    /// The producing module never declared the family's claim.
    MissingClaim {
        /// Claim string the producer would have had to hold.
        required: String,
    },
}

/// One entry refused publication because it does not own its region.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnershipViolation {
    /// Layer the trespassing entry was written for.
    pub global_layer_index: i32,
    /// Object the trespassing entry was written for.
    pub object_id: String,
    /// Region the trespassing entry was written for.
    pub region_id: u64,
    /// Family the trespassing entry declared.
    pub family_id: String,
    /// Module that produced the plan carrying the entry.
    pub module_id: String,
    /// Why ownership was refused.
    pub reason: OwnershipReason,
    /// Index into `plans` of the producing plan.
    pub plan_index: usize,
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

/// One body trimmed to its family's support territory (ticket 19). Trimming
/// is expected behaviour where two families meet across a modifier boundary,
/// so it is reported as Info code 1205 — never as `unmet`, never `degraded`.
#[derive(Debug, Clone, PartialEq)]
pub struct ClippedSupportBody {
    /// Family that owned the trimmed body.
    pub family_id: String,
    /// Body identities carried by the trimmed entry.
    pub body_ids: Vec<String>,
    /// Object the entry belongs to.
    pub object_id: String,
    /// Layer the entry belongs to.
    pub global_layer_index: i32,
    /// Region the entry was planned for.
    pub region_id: u64,
    /// Area removed, in canonical units squared.
    pub removed_area: f64,
    /// Whether nothing of the body survived and the entry was dropped.
    pub dropped: bool,
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
    /// Bodies trimmed to their family's support territory (ticket 19).
    pub clipped: Vec<ClippedSupportBody>,
    /// Duplicate identities rejected in deterministic input order.
    ///
    /// **Currently unreachable by construction — nothing in this crate pushes
    /// to it.** The only site that populated it was the cross-family
    /// arrival-order arbitration branch removed by packet 241b. It is retained
    /// (with diagnostic code `1202`) as a tripwire for a future writer, not as
    /// a live signal: an empty `duplicates` proves nothing today, so do not
    /// read `duplicates.is_empty()` as evidence that duplicates were checked.
    ///
    /// It is deliberately NOT repopulated from a post-union
    /// `(global_layer_index, object_id, region_id)` scan: `region_id` is not
    /// unique per plane, and `union_same_family_entries` keys on `anchor_z` as
    /// well, so same-family entries that legitimately share a dispatch layer
    /// while carrying distinct physical planes would all read as duplicates.
    /// No key over the post-union survivors distinguishes that legitimate case
    /// from a real duplicate without reintroducing the cross-family
    /// arbitration this packet deleted.
    pub duplicates: Vec<DuplicateSupportPlanEntry>,
    /// Entries dropped because they do not own the region they wrote.
    pub ownership_violations: Vec<OwnershipViolation>,
    /// Raft metadata merged from all family plans.
    pub raft_plan: Option<RaftPlan>,
    /// Structured diagnostics for rejected bodies and declined demands.
    pub diagnostics: Vec<SupportRoutingDiagnostics>,
}

/// Fatal support-region ownership violation.
///
/// Under [`FamilyConflictPolicy::Fail`] the first trespass aborts the merge and
/// nothing is published. The payload is the trespass itself: which entry wrote
/// which region, which module produced it, and why it was refused. It no longer
/// describes an arrival-order collision between two writers, because arrival
/// order no longer decides anything — ownership is declared up front.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportAggregationError {
    /// Layer the trespassing entry was written for.
    pub global_layer_index: i32,
    /// Object the trespassing entry was written for.
    pub object_id: String,
    /// Region the trespassing entry was written for.
    pub region_id: u64,
    /// Family the trespassing entry declared.
    pub family_id: String,
    /// Module that produced the plan carrying the entry.
    pub module_id: String,
    /// Why ownership was refused.
    pub reason: OwnershipReason,
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

/// Decide whether one entry is allowed to write the region it names.
///
/// Ownership is declared, never raced for: the host's `family_assignments` map
/// names the owning family of every `(object_id, region_id)`, and only a module
/// holding that family's `support-family:<id>` claim may act for it. There is
/// no fallback — a region with no assignment row has no owner, so a wholly
/// absent `territory` denies every entry.
///
/// This mirrors the policy shape of `enforce_authored_coloring`
/// (`crates/slicer-wasm-host/src/marshal/out.rs`): default-deny in policy, but
/// never in silence — every refusal is reported by the caller.
// The `Err` payload is the violation report itself; boxing it would buy
// nothing on a path taken once per rejected entry and would obscure the
// refusal at every call site.
#[allow(clippy::result_large_err)]
fn check_ownership(
    entry: &SupportPlanEntry,
    plan_index: usize,
    input: &SupportAggregationInput<'_>,
) -> Result<(), OwnershipViolation> {
    let producer = input.producers.get(plan_index);
    let module_id = producer
        .map(|producer| producer.module_id.clone())
        .unwrap_or_default();
    let required_claim = format!("support-family:{}", entry.family_id);
    let owner = input
        .territory
        .and_then(|analysis| {
            analysis
                .family_assignments
                .get(&(entry.object_id.clone(), entry.region_id))
        })
        .cloned();
    let reason = match owner {
        None => Some(OwnershipReason::NoAssignment),
        Some(owner) if owner != entry.family_id => Some(OwnershipReason::WrongFamily { owner }),
        Some(_) => match producer {
            // A missing producer is a host-side construction invariant, not a
            // module fault; it denies like any unclaimed write rather than
            // publishing something whose author is unknown.
            Some(producer) if producer.claims.contains(&required_claim) => None,
            _ => Some(OwnershipReason::MissingClaim {
                required: required_claim,
            }),
        },
    };
    match reason {
        None => Ok(()),
        Some(reason) => Err(OwnershipViolation {
            global_layer_index: entry.global_layer_index,
            object_id: entry.object_id.clone(),
            region_id: entry.region_id,
            family_id: entry.family_id.clone(),
            module_id,
            reason,
            plan_index,
        }),
    }
}

/// Aggregate family plans under an explicit [`FamilyConflictPolicy`].
///
/// Ownership is checked first, before any validation: an entry that does not
/// own its region never reaches the rest of the pipeline. The check is a pure
/// function of `(entry, family_assignments, producer claims)`, so its outcome
/// does not depend on the order the plans arrive in.
pub fn try_aggregate_support_plans_with_policy(
    input: SupportAggregationInput<'_>,
    conflict_policy: FamilyConflictPolicy,
) -> Result<SupportAggregationResult, SupportAggregationError> {
    // Producers are index-parallel to plans by construction at every call site;
    // a mismatch is a host bug, and it denies rather than publishing anything
    // unowned (see `check_ownership`).
    debug_assert_eq!(
        input.plans.len(),
        input.producers.len(),
        "every support plan must carry its producer identity"
    );
    let mut result = SupportAggregationResult::default();
    let mut identities = HashMap::new();
    let mut entries = input
        .plans
        .iter()
        .enumerate()
        .flat_map(|(plan_index, plan)| {
            plan.entries
                .iter()
                .map(move |entry| (plan_index, entry.clone()))
        })
        .collect::<Vec<_>>();
    // Purely for output determinism: ownership no longer depends on order.
    entries.sort_by(|left, right| compare_entries(&left.1, &right.1));
    for plan in &input.plans {
        result.raft_plan = merge_raft_plans(result.raft_plan.take(), plan.raft_plan.clone());
    }
    for (plan_index, entry) in entries {
        let identity = (
            entry.global_layer_index,
            entry.object_id.clone(),
            entry.region_id,
        );
        if let Err(violation) = check_ownership(&entry, plan_index, &input) {
            match conflict_policy {
                FamilyConflictPolicy::Fail => {
                    return Err(SupportAggregationError {
                        global_layer_index: violation.global_layer_index,
                        object_id: violation.object_id,
                        region_id: violation.region_id,
                        family_id: violation.family_id,
                        module_id: violation.module_id,
                        reason: violation.reason,
                    })
                }
                FamilyConflictPolicy::Degrade => {
                    result.degraded = true;
                    result.ownership_violations.push(violation);
                    continue;
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
    // construction not one planner-emitted body, so the per-body max-body-extent
    // territory bound does not apply to it, and two legitimately same-`body_id`
    // entries far apart on the plate were dropped wholesale once their union
    // envelope exceeded one cell. Canonical support-island merging (`union_` in
    // OrcaSlicer's `SupportCommon.cpp` / `SupportMaterial.cpp`) imposes no size
    // cap on the merged result. The occupancy predicate is set-monotone -- a
    // union cannot introduce an overlap that was absent from every input -- so
    // re-checking it after merging would be redundant as well.
    union_same_family_entries(&mut result.retained);
    // Ticket 19: where the host published support territory for an
    // `(object, layer)`, each retained body is clipped to the side its family
    // owns (sub-region: `∩ own`; base: `- inflate(foreign, clearance)`).
    // Two families meeting across a modifier boundary is the *intended*
    // outcome there, so the clip is recorded as Info 1205 and the reject-both
    // guard below is skipped for those layers. Layers without territory keep
    // the guard unchanged.
    let clipper = input.territory.and_then(TerritoryClipper::from_ir);
    let has_territory = |entry: &SupportPlanEntry| {
        entry.global_layer_index >= 0
            && clipper.as_ref().is_some_and(|clipper| {
                clipper.has_territory(&entry.object_id, entry.global_layer_index as u32)
            })
    };
    if let Some(clipper) = clipper.as_ref() {
        let retained = std::mem::take(&mut result.retained);
        for mut entry in retained {
            if !has_territory(&entry) {
                result.retained.push(entry);
                continue;
            }
            let layer = entry.global_layer_index as u32;
            let region_id = entry.region_id.to_string();
            let before = roles_area(&entry);
            for role in &mut entry.roles {
                if let Some(kept) = clipper.clip(
                    &entry.object_id,
                    layer,
                    &region_id,
                    &entry.family_id,
                    &role.regions,
                ) {
                    role.regions = kept;
                }
            }
            entry.roles.retain(|role| !role.regions.is_empty());
            let after = roles_area(&entry);
            let dropped = entry.roles.is_empty();
            if dropped || after < before {
                result.clipped.push(ClippedSupportBody {
                    family_id: entry.family_id.clone(),
                    body_ids: entry.body_ids.clone(),
                    object_id: entry.object_id.clone(),
                    global_layer_index: entry.global_layer_index,
                    region_id: entry.region_id,
                    removed_area: before - after,
                    dropped,
                });
            }
            if !dropped {
                result.retained.push(entry);
            }
        }
    }
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
                && !has_territory(a)
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

fn roles_area(entry: &SupportPlanEntry) -> f64 {
    entry
        .roles
        .iter()
        .flat_map(|role| role.regions.iter())
        .map(expolygon_area)
        .sum()
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
                // Per-body extent-bound violation: the body's own bbox spans
                // more than `MAX_BODY_EXTENT_UNITS` on an axis. This is not a
                // cell assignment and nothing collides. The string is pinned
                // verbatim by assertions in
                // `crates/slicer-runtime/tests/integration/support_family_routing.rs`,
                // so renaming it must land together with those.
                Some("body rejected: max-body-extent violation")
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

/// Combine validated entries that one family contributed to a single DECLARED
/// support region. The merge key is the declared identity
/// `(family_id, global_layer_index, object_id, region_id, anchor_z)` -- never
/// where the geometry happens to sit, so two contributions to one region merge
/// no matter how far apart their bodies are. The first entry supplies scalar
/// attribution; geometry, body identities, and demands are accumulated without
/// duplicates.
fn union_same_family_entries(entries: &mut Vec<SupportPlanEntry>) {
    let mut merged: Vec<SupportPlanEntry> = Vec::new();
    for entry in entries.drain(..) {
        let matching = merged.iter().position(|existing| {
            existing.family_id == entry.family_id
                && existing.global_layer_index == entry.global_layer_index
                && existing.object_id == entry.object_id
                && existing.region_id == entry.region_id
                // Independent support rows share the dispatch layer but
                // intentionally carry distinct physical planes.
                && existing.anchor_z == entry.anchor_z
        });
        let Some(index) = matching else {
            merged.push(entry);
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

fn dedup_sorted(values: &mut Vec<String>) {
    let mut unique = HashSet::new();
    values.retain(|value| unique.insert(value.clone()));
    values.sort();
}

/// Human-readable form of an ownership refusal, for diagnostic messages.
fn describe_ownership_reason(reason: &OwnershipReason) -> String {
    match reason {
        OwnershipReason::NoAssignment => {
            "no support family owns this region (no family_assignments row)".to_string()
        }
        OwnershipReason::WrongFamily { owner } => {
            format!("region is owned by family '{owner}'")
        }
        OwnershipReason::MissingClaim { required } => {
            format!("producer does not hold claim '{required}'")
        }
    }
}

/// Degrading aggregation that preserves per-plan attribution for every
/// diagnostic it mints.
///
/// This is the form the prepass uses: it needs to attach each diagnostic to
/// the audit of the module that actually produced the offending plan, not to
/// whichever family writer happened to run last. It is also the only wrapper
/// that takes the committed support analysis, whose `support_territory`
/// (ticket 19) clips cross-family bodies instead of rejecting them; the
/// other wrappers serve territory-free callers and pass `None`.
///
/// `producers` is index-parallel to `plans` and carries the manifest identity
/// of the module behind each plan. It is a required argument rather than a
/// defaulted one on purpose: ownership is default-deny, so a caller that
/// cannot name its producers would silently publish nothing.
pub fn aggregate_support_plan_irs_degrading_with_attributed_diagnostics(
    plans: Vec<SupportPlanIR>,
    producers: Vec<SupportPlanProducer>,
    exact_z: &ExactZQueryService,
    territory: Option<&SupportAnalysisIR>,
) -> (SupportPlanIR, Vec<AttributedDiagnostic>) {
    {
        aggregate_support_plan_irs_with_policy_attributed(
            plans,
            producers,
            exact_z,
            territory,
            FamilyConflictPolicy::Degrade,
        )
    }
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

/// Attributed aggregation with explicit producer identities.
///
/// `producers` is index-parallel to `plans`; it is what lets the merge point
/// enforce support-region ownership and what lets an ownership diagnostic name
/// the offending module directly instead of guessing it from a family id.
pub fn aggregate_support_plan_irs_with_policy_attributed(
    plans: Vec<SupportPlanIR>,
    producers: Vec<SupportPlanProducer>,
    exact_z: &ExactZQueryService,
    territory: Option<&SupportAnalysisIR>,
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
            territory,
            producers,
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
    // Ownership refusals carry their producing plan index with them, so this is
    // the one diagnostic family that never has to invert a family or body id
    // back to a plan: the trespasser is named exactly.
    diagnostics.extend(
        aggregate
            .ownership_violations
            .iter()
            .map(|violation| AttributedDiagnostic {
                plan_index: Some(violation.plan_index),
                diagnostic: slicer_ir::Diagnostic {
                    severity: slicer_ir::DiagnosticSeverity::Warn,
                    code: 1206,
                    layer: Some(violation.global_layer_index),
                    object_id: Some(violation.object_id.clone()),
                    message: format!(
                        "support region not owned by writer: family='{}', object='{}', region={}, layer={}, module='{}': {}",
                        violation.family_id,
                        violation.object_id,
                        violation.region_id,
                        violation.global_layer_index,
                        violation.module_id,
                        describe_ownership_reason(&violation.reason)
                    ),
                },
            }),
    );
    // Ticket 19: territory clips are expected where two families meet across
    // a modifier boundary. Info, not Warn: nothing is unmet.
    diagnostics.extend(aggregate.clipped.iter().map(|clipped| AttributedDiagnostic {
        plan_index: family_to_plan.get(clipped.family_id.as_str()).copied(),
        diagnostic: slicer_ir::Diagnostic {
            severity: slicer_ir::DiagnosticSeverity::Info,
            code: 1205,
            layer: Some(clipped.global_layer_index),
            object_id: Some(clipped.object_id.clone()),
            message: format!(
                "support body clipped to family territory: family='{}', bodies={:?}, region={}, removed_area={:.0}{}",
                clipped.family_id,
                clipped.body_ids,
                clipped.region_id,
                clipped.removed_area,
                if clipped.dropped { ", entry dropped" } else { "" }
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

/// True when a single body's own envelope spans no more than
/// [`MAX_BODY_EXTENT_UNITS`] (`1 << 20` units) on each axis.
///
/// This is a **pure per-body extent bound**, not a partitioning scheme. It
/// assigns the body to nothing, it compares the body against no grid and
/// against no other body, and it takes no part in deciding which entries
/// merge. Only the width and height of this one body's own bounding box are
/// measured, so absolute position is irrelevant: a body straddling x = 0 or
/// y = 0 is treated exactly like the same body translated anywhere else. A
/// body is rejected only when it is genuinely larger than the maximum extent
/// one support body is permitted to claim.
///
/// The name is retained deliberately (packet 224's RC-14 record and the
/// traditional planner's `merge_region_identity_entries` doc comment both
/// refer to it) even though no routing cell exists.
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
    maxx.saturating_sub(minx) <= MAX_BODY_EXTENT_UNITS
        && maxy.saturating_sub(miny) <= MAX_BODY_EXTENT_UNITS
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
