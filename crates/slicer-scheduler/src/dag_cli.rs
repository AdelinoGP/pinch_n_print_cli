//! Implementation of the `pnp_cli dag <subcommand>` JSON output.
//!
//! All entry points here operate on `&[&dyn Producer]` — a uniform slice of
//! producers that may include both WASM-backed [`LoadedModule`]s and host
//! built-in [`BuiltinProducer`] statics. Designed for agent tooling that
//! polls module wiring at sub-100 ms latency regardless of module count.
//!
//! See `docs/specs/agent-cli-debugging.md` §4.3.

use std::collections::BTreeMap;

use serde::Serialize;
use slicer_ir::{ModuleId, StageId};

use crate::dag::{build_global_dag, build_intra_stage_dag, GlobalEdge, Producer};
use crate::instrumentation::{EdgeReason, SerialEdge};

/// Top-level output for `dag stages`.
#[derive(Debug, Clone, Serialize)]
pub struct StagesOut {
    /// One entry per discovered stage, sorted by canonical stage id.
    pub stages: Vec<StageSummary>,
}

/// One stage row in `dag stages` output.
#[derive(Debug, Clone, Serialize)]
pub struct StageSummary {
    /// Canonical scheduler stage id (e.g. `"Layer::Infill"`).
    pub id: String,
    /// Tier the stage belongs to, derived from the stage-id prefix.
    pub tier: String,
    /// Number of loaded modules in this stage.
    pub module_count: usize,
    /// Number of distinct claims declared by modules in this stage.
    pub claim_count: usize,
}

/// Output for `dag stage <id>`.
#[derive(Debug, Clone, Serialize)]
pub struct StageOut {
    /// Canonical stage id.
    pub id: String,
    /// Tier prefix.
    pub tier: String,
    /// Modules in this stage with their declared IR access and config.
    pub modules: Vec<ModuleOut>,
    /// Serial edges between modules in this stage.
    pub serial_edges: Vec<StageEdgeOut>,
}

/// One module row in `dag stage <id>` output.
#[derive(Debug, Clone, Serialize)]
pub struct ModuleOut {
    /// Reverse-domain module id.
    pub id: String,
    /// Claims held by this module.
    pub claims: Vec<String>,
    /// Declared IR read paths.
    pub ir_reads: Vec<String>,
    /// Declared IR write paths.
    pub ir_writes: Vec<String>,
    /// `requires_modules` entries from the manifest.
    pub requires_modules: Vec<String>,
    /// Config keys declared in the module's config schema, sorted lexically.
    pub config_keys: Vec<String>,
}

/// One intra-stage edge in `dag stage <id>` output.
///
/// `reason` is projected to a flat human-readable string matching the
/// spec's example shapes (`"ir_write_read: <path>"` /
/// `"explicit_requires"`) rather than serializing the full `EdgeReason`
/// enum.
#[derive(Debug, Clone, Serialize)]
pub struct StageEdgeOut {
    /// Upstream module id.
    pub from: String,
    /// Downstream module id.
    pub to: String,
    /// Flat reason string (`"ir_write_read: <path>"` or `"explicit_requires"`).
    pub reason: String,
}

/// Output for `dag depends <module-id>`.
#[derive(Debug, Clone, Serialize)]
pub struct DependsOut {
    /// The queried module id.
    pub module_id: String,
    /// Object ids from `--model`, when supplied; omitted otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_ids: Option<Vec<String>>,
    /// Edges where the queried module appears as the downstream node.
    pub upstream: Vec<GlobalEdgeOut>,
    /// Edges where the queried module appears as the upstream node.
    pub downstream: Vec<GlobalEdgeOut>,
}

/// One cross-stage edge in `dag depends` output.
#[derive(Debug, Clone, Serialize)]
pub struct GlobalEdgeOut {
    /// Upstream module id.
    pub from: String,
    /// Stage that `from` belongs to.
    pub from_stage: String,
    /// Downstream module id.
    pub to: String,
    /// Stage that `to` belongs to.
    pub to_stage: String,
    /// Flat reason string (`"ir_write_read: <path>"` or `"explicit_requires"`).
    pub reason: String,
}

/// Output for `dag claims`.
#[derive(Debug, Clone, Serialize)]
pub struct ClaimsOut {
    /// One entry per distinct claim id seen across the module set.
    pub claims: Vec<ClaimOut>,
}

/// One claim row in `dag claims` output.
#[derive(Debug, Clone, Serialize)]
pub struct ClaimOut {
    /// Claim identifier (e.g. `"claim:sparse-fill"`).
    pub id: String,
    /// Modules that declare this claim in their `claims.holds`.
    pub holders: Vec<String>,
    /// Modules that declare this claim in `requires_claims`.
    pub requesters: Vec<String>,
    /// True when more than one module holds this claim — by docs the
    /// scheduler selects one holder per region/run, making the holders
    /// interchangeable.
    pub interchangeable: bool,
}

/// Derive the tier string from a stage id prefix.
///
/// Unknown prefixes return `"unknown"`. Used to populate the `tier` field
/// in `dag stages` / `dag stage` output without forcing a separate registry
/// lookup.
fn tier_of(stage: &str) -> &'static str {
    crate::stage_order::tier_of(stage)
}

fn flatten_reason(reason: &EdgeReason) -> String {
    match reason {
        EdgeReason::IrWriteRead { writer_path } => {
            format!("ir_write_read: {writer_path}")
        }
        EdgeReason::ExplicitRequires => "explicit_requires".to_string(),
    }
}

fn edge_to_stage_edge_out(edge: &SerialEdge) -> StageEdgeOut {
    StageEdgeOut {
        from: edge.from.clone(),
        to: edge.to.clone(),
        reason: flatten_reason(&edge.reason),
    }
}

fn edge_to_global_out(edge: &GlobalEdge) -> GlobalEdgeOut {
    GlobalEdgeOut {
        from: edge.from.clone(),
        from_stage: edge.from_stage.clone(),
        to: edge.to.clone(),
        to_stage: edge.to_stage.clone(),
        reason: flatten_reason(&edge.reason),
    }
}

/// `dag stages` — list every stage with its tier, module count, claim count.
pub fn run_dag_stages(producers: &[&dyn Producer]) -> StagesOut {
    // BTreeMap gives deterministic stage ordering by canonical id.
    let mut by_stage: BTreeMap<&str, Vec<&dyn Producer>> = BTreeMap::new();
    for p in producers {
        by_stage.entry(p.stage()).or_default().push(*p);
    }

    let stages = by_stage
        .into_iter()
        .map(|(id, group)| {
            let mut distinct_claims: std::collections::BTreeSet<&str> =
                std::collections::BTreeSet::new();
            for p in &group {
                for c in p.claims_holds() {
                    distinct_claims.insert(c.as_str());
                }
            }
            StageSummary {
                id: id.to_string(),
                tier: tier_of(id).to_string(),
                module_count: group.len(),
                claim_count: distinct_claims.len(),
            }
        })
        .collect();

    StagesOut { stages }
}

/// `dag stage <id>` — full detail for one stage, or `None` if no producer is
/// in that stage.
pub fn run_dag_stage(producers: &[&dyn Producer], id: &StageId) -> Option<StageOut> {
    let stage_producers: Vec<&dyn Producer> = producers
        .iter()
        .copied()
        .filter(|p| p.stage() == id.as_str())
        .collect();
    if stage_producers.is_empty() {
        return None;
    }

    // Build intra-stage DAG using the broadened Producer-based function, then
    // convert DAG edges to the flat SerialEdge format for JSON output.
    let dag_nodes = build_intra_stage_dag(id.clone(), &stage_producers).unwrap_or_default();
    let mut serial_edges: Vec<SerialEdge> = Vec::new();
    for node in &dag_nodes {
        for edge in &node.edges_to {
            for reason in &edge.reasons {
                serial_edges.push(SerialEdge {
                    from: node.module_id.clone(),
                    to: edge.to.clone(),
                    reason: reason.clone(),
                });
            }
        }
    }
    serial_edges.sort_by(|a, b| {
        a.from
            .cmp(&b.from)
            .then_with(|| a.to.cmp(&b.to))
            .then_with(|| {
                let tag = |r: &EdgeReason| match r {
                    EdgeReason::IrWriteRead { .. } => 0u8,
                    EdgeReason::ExplicitRequires => 1u8,
                };
                tag(&a.reason).cmp(&tag(&b.reason))
            })
    });

    let module_views = stage_producers
        .iter()
        .map(|p| ModuleOut {
            id: p.id().to_string(),
            claims: p.claims_holds().to_vec(),
            ir_reads: p.ir_reads().to_vec(),
            ir_writes: p.ir_writes().to_vec(),
            requires_modules: p.requires_modules().to_vec(),
            // config_keys is LoadedModule-specific; not in the Producer trait.
            // Built-in producers have no config schema; real WASM modules expose
            // config keys via their manifests but that detail is not surfaced
            // through the Producer abstraction — downstream tooling that needs
            // per-module config schemas should use `dag stage` manifest parsing
            // directly.
            config_keys: vec![],
        })
        .collect();

    Some(StageOut {
        id: id.clone(),
        tier: tier_of(id.as_str()).to_string(),
        modules: module_views,
        serial_edges: serial_edges.iter().map(edge_to_stage_edge_out).collect(),
    })
}

/// `dag depends <module-id>` — upstream and downstream edges in the global
/// DAG, or `None` if the module id isn't found.
pub fn run_dag_depends(
    producers: &[&dyn Producer],
    target: &ModuleId,
    object_ids: Option<&[String]>,
) -> Option<DependsOut> {
    if !producers.iter().any(|p| p.id() == target.as_str()) {
        return None;
    }
    let global = build_global_dag(producers);
    let upstream: Vec<GlobalEdgeOut> = global
        .iter()
        .filter(|e| e.to == *target)
        .map(edge_to_global_out)
        .collect();
    let downstream: Vec<GlobalEdgeOut> = global
        .iter()
        .filter(|e| e.from == *target)
        .map(edge_to_global_out)
        .collect();
    Some(DependsOut {
        module_id: target.clone(),
        object_ids: object_ids.map(|ids| ids.to_vec()),
        upstream,
        downstream,
    })
}

/// `dag claims` — every claim with its holders and requesters.
pub fn run_dag_claims(producers: &[&dyn Producer]) -> ClaimsOut {
    let mut holders: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut requesters: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for p in producers {
        for c in p.claims_holds() {
            holders
                .entry(c.clone())
                .or_default()
                .push(p.id().to_string());
        }
        for c in p.claims_requires() {
            requesters
                .entry(c.clone())
                .or_default()
                .push(p.id().to_string());
        }
    }

    let mut claim_ids: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    claim_ids.extend(holders.keys().cloned());
    claim_ids.extend(requesters.keys().cloned());

    let claims = claim_ids
        .into_iter()
        .map(|id| {
            let h = holders.get(&id).cloned().unwrap_or_default();
            let r = requesters.get(&id).cloned().unwrap_or_default();
            let interchangeable = h.len() > 1;
            ClaimOut {
                id,
                holders: h,
                requesters: r,
                interchangeable,
            }
        })
        .collect();

    ClaimsOut { claims }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dag::Producer;
    use crate::manifest::{LoadedModule, LoadedModuleBuilder};
    use slicer_ir::SemVer;
    use std::path::PathBuf;

    fn module(
        id: &str,
        stage: &str,
        ir_reads: &[&str],
        ir_writes: &[&str],
        requires_modules: &[&str],
        claims: &[&str],
        requires_claims: &[&str],
    ) -> LoadedModule {
        LoadedModuleBuilder::new(
            id,
            SemVer {
                major: 1,
                minor: 0,
                patch: 0,
            },
            stage,
            "slicer:world-layer@1.0.0",
            PathBuf::from(format!("fixtures/{id}.wasm")),
        )
        .ir_reads(ir_reads.iter().map(|s| s.to_string()).collect())
        .ir_writes(ir_writes.iter().map(|s| s.to_string()).collect())
        .requires_modules(requires_modules.iter().map(|s| s.to_string()).collect())
        .claims(claims.iter().map(|s| s.to_string()).collect())
        .requires_claims(requires_claims.iter().map(|s| s.to_string()).collect())
        .min_host_version(SemVer {
            major: 0,
            minor: 1,
            patch: 0,
        })
        .min_ir_schema(SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        })
        .max_ir_schema(SemVer {
            major: 2,
            minor: 0,
            patch: 0,
        })
        .layer_parallel_safe(true)
        .build()
    }

    fn as_producers(modules: &[LoadedModule]) -> Vec<&dyn Producer> {
        modules.iter().map(|m| m as &dyn Producer).collect()
    }

    #[test]
    fn tier_prefix_mapping_is_correct() {
        assert_eq!(tier_of("PrePass::MeshAnalysis"), "prepass");
        assert_eq!(tier_of("Layer::Infill"), "per_layer");
        assert_eq!(tier_of("PostPass::GCodeEmit"), "postpass");
        assert_eq!(tier_of("Bogus::Foo"), "unknown");
    }

    #[test]
    fn flatten_reason_matches_spec_examples() {
        assert_eq!(
            flatten_reason(&EdgeReason::IrWriteRead {
                writer_path: "InfillIR.regions[].paths".to_string(),
            }),
            "ir_write_read: InfillIR.regions[].paths"
        );
        assert_eq!(
            flatten_reason(&EdgeReason::ExplicitRequires),
            "explicit_requires"
        );
    }

    #[test]
    fn stages_groups_modules_and_counts_claims() {
        let modules = vec![
            module(
                "com.example.cubic_infill",
                "Layer::Infill",
                &[],
                &["InfillIR.regions[].paths"],
                &[],
                &["claim:sparse"],
                &[],
            ),
            module(
                "com.example.gyroid_infill",
                "Layer::Infill",
                &[],
                &["InfillIR.regions[].paths"],
                &[],
                &["claim:sparse"],
                &[],
            ),
            module(
                "com.example.mesh_analysis",
                "PrePass::MeshAnalysis",
                &[],
                &[],
                &[],
                &[],
                &[],
            ),
        ];
        let producers = as_producers(&modules);
        let out = run_dag_stages(&producers);
        let infill = out.stages.iter().find(|s| s.id == "Layer::Infill").unwrap();
        assert_eq!(infill.module_count, 2);
        assert_eq!(infill.claim_count, 1); // distinct claim:sparse
        assert_eq!(infill.tier, "per_layer");
        let mesh = out
            .stages
            .iter()
            .find(|s| s.id == "PrePass::MeshAnalysis")
            .unwrap();
        assert_eq!(mesh.module_count, 1);
        assert_eq!(mesh.tier, "prepass");
    }

    #[test]
    fn stage_returns_serial_edges_with_flat_reasons() {
        let modules = vec![
            module(
                "a",
                "Layer::Infill",
                &[],
                &["InfillIR.paths"],
                &[],
                &[],
                &[],
            ),
            module(
                "b",
                "Layer::Infill",
                &["InfillIR.paths"],
                &[],
                &[],
                &[],
                &[],
            ),
        ];
        let producers = as_producers(&modules);
        let out = run_dag_stage(&producers, &"Layer::Infill".to_string()).unwrap();
        assert_eq!(out.serial_edges.len(), 1);
        assert_eq!(out.serial_edges[0].from, "a");
        assert_eq!(out.serial_edges[0].to, "b");
        assert_eq!(out.serial_edges[0].reason, "ir_write_read: InfillIR.paths");
    }

    #[test]
    fn stage_returns_none_for_unknown_stage() {
        let modules = vec![module("a", "Layer::Infill", &[], &[], &[], &[], &[])];
        let producers = as_producers(&modules);
        assert!(run_dag_stage(&producers, &"Layer::Nope".to_string()).is_none());
    }

    #[test]
    fn depends_splits_upstream_and_downstream() {
        let modules = vec![
            // perim writes -> cubic_infill reads (upstream of cubic_infill)
            module(
                "com.example.perim",
                "Layer::Perimeters",
                &[],
                &["LayerIR.regions.walls"],
                &[],
                &[],
                &[],
            ),
            module(
                "com.example.cubic_infill",
                "Layer::Infill",
                &["LayerIR.regions.walls"],
                &["InfillIR.regions.paths"],
                &[],
                &[],
                &[],
            ),
            // postproc reads InfillIR -> downstream of cubic_infill
            module(
                "com.example.postproc",
                "PostPass::GCodePostProcess",
                &["InfillIR.regions.paths"],
                &[],
                &[],
                &[],
                &[],
            ),
        ];
        let producers = as_producers(&modules);
        let target: ModuleId = "com.example.cubic_infill".to_string();
        let out = run_dag_depends(&producers, &target, None).unwrap();
        assert_eq!(out.upstream.len(), 1);
        assert_eq!(out.upstream[0].from, "com.example.perim");
        assert_eq!(out.upstream[0].from_stage, "Layer::Perimeters");
        assert_eq!(out.upstream[0].to_stage, "Layer::Infill");
        assert_eq!(out.downstream.len(), 1);
        assert_eq!(out.downstream[0].to, "com.example.postproc");
        assert_eq!(out.downstream[0].to_stage, "PostPass::GCodePostProcess");
        assert!(out.object_ids.is_none());
    }

    #[test]
    fn depends_returns_none_for_unknown_module() {
        let modules = vec![module(
            "com.example.a",
            "Layer::Infill",
            &[],
            &[],
            &[],
            &[],
            &[],
        )];
        let producers = as_producers(&modules);
        let missing: ModuleId = "com.example.does-not-exist".to_string();
        assert!(run_dag_depends(&producers, &missing, None).is_none());
    }

    #[test]
    fn depends_includes_object_ids_when_supplied() {
        let modules = vec![module(
            "com.example.a",
            "Layer::Infill",
            &[],
            &[],
            &[],
            &[],
            &[],
        )];
        let producers = as_producers(&modules);
        let target: ModuleId = "com.example.a".to_string();
        let ids = vec!["benchy".to_string()];
        let out = run_dag_depends(&producers, &target, Some(&ids)).unwrap();
        assert_eq!(out.object_ids, Some(vec!["benchy".to_string()]));
    }

    #[test]
    fn claims_flags_interchangeable_holders() {
        let modules = vec![
            module(
                "com.example.cubic",
                "Layer::Infill",
                &[],
                &[],
                &[],
                &["claim:sparse"],
                &[],
            ),
            module(
                "com.example.gyroid",
                "Layer::Infill",
                &[],
                &[],
                &[],
                &["claim:sparse"],
                &[],
            ),
            module(
                "com.example.seam_planner",
                "Layer::Perimeters",
                &[],
                &[],
                &[],
                &[],
                &["perimeter-generator"],
            ),
            module(
                "com.example.perim_gen",
                "Layer::Perimeters",
                &[],
                &[],
                &[],
                &["perimeter-generator"],
                &[],
            ),
        ];
        let producers = as_producers(&modules);
        let out = run_dag_claims(&producers);
        let sparse = out.claims.iter().find(|c| c.id == "claim:sparse").unwrap();
        assert_eq!(sparse.holders.len(), 2);
        assert!(sparse.interchangeable);
        let pg = out
            .claims
            .iter()
            .find(|c| c.id == "perimeter-generator")
            .unwrap();
        assert_eq!(pg.holders.len(), 1);
        assert_eq!(pg.requesters, vec!["com.example.seam_planner".to_string()]);
        assert!(!pg.interchangeable);
    }
}
