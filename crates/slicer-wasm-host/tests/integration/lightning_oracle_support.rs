//! Gate A oracle support: fixture loader, projector/extractor, comparator,
//! evidence hasher, and outcome writer for the Lightning portability gate.
//!
//! Everything here is test-only. The oracle is the recorded canonical fixture
//! (`crates/slicer-wasm-host/tests/fixtures/lightning_gate/`), never the kernel's
//! own output: expectations are read from the fixture, the construction is
//! derived from the restricted earlier-source subtree, and the comparator never
//! feeds a reference value into construction.
//!
//! Unit discipline: `a_source` and `GateAConstruction` are PnP IR units
//! (`1 unit = 100 nm`). The recorded reference is canonical `coord_t` units
//! (1 nm). Reference geometry is divided by 100 with the recorder's documented
//! `round-half-away-from-zero` transport rule before comparison.
//!
//! `serde` is not a direct dependency of this test binary, so the fixture
//! envelopes and the outcome record are decoded/encoded explicitly against
//! `serde_json::Value`. The kernel's own DTOs keep their derives; they are
//! decoded through `serde_json::from_value` without naming the trait here.

#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use lightning_feasibility_kernel::domain::build_planning_domain;
use lightning_feasibility_kernel::{
    BoundaryAddress, BoundaryContribution, BoundaryProvenance, ContributionRelation,
    DerivedRegionPolicy, DomainKind, FillPolicyInput, GateAConstruction, GateASourceInput,
    GeometryOperation, KernelError, LightningParameters, PlanningLayer, RegionIdentity,
    SourceSurface,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use slicer_ir::{ExPolygon, Point2};

pub const FIXTURE_DIR_REL: &str = "tests/fixtures/lightning_gate";
pub const INPUTS_FILE: &str = "inputs.json";
pub const CANONICAL_FILE: &str = "canonical.json";
pub const MANIFEST_FILE: &str = "manifest.json";

pub const PNP_UNITS_PER_CANONICAL_UNIT: i64 = 100;
/// The recorder's transport tolerance (`manifest.units.transport`): one PnP
/// unit per converted canonical coordinate.
pub const GEOMETRY_TOLERANCE_PNP_UNITS: i64 = 1;
/// Plain millimetre/degree/fraction fields are derived from the same recorded
/// raw config, so only float-formatting noise separates them (1e-9 mm).
pub const SCALAR_TOLERANCE_MM: f64 = 1e-9;
pub const ANGLE_TOLERANCE_RAD: f64 = 1e-9;
/// `prune_length_mm` and `straightening_max_distance_mm` are recorded as
/// canonical integer `coord_t` nanometres (`17497 nm` = `0.017497 mm`), while
/// the kernel derives the exact real value (`0.0174977... mm`). One canonical
/// `coord_t` is 1 nm = 1e-6 mm, so that is the honest comparison resolution.
pub const COORD_T_TOLERANCE_MM: f64 = 1e-6;
/// The replay's aligned symmetric-difference area is recorded to 1e-11 mm².
pub const REPLAY_AREA_TOLERANCE_MM2: f64 = 1e-9;
/// Recorded on the current capture for `anchor_bridge_classic`.
pub const ANCHOR_BRIDGE_SYMMETRIC_DIFFERENCE_MM2: f64 = 142.559_129_272_76;

/// The five Classic case families in fixture order.
pub const EXPECTED_CASE_IDS: [&str; 5] = [
    "anchor_bridge_classic",
    "pooled_regions_classic",
    "hole_void_classic",
    "island_split_merge_classic",
    "paint_modifier_config_classic",
];

// -----------------------------------------------------------------------------
// Strict JSON envelope helpers
// -----------------------------------------------------------------------------

fn strict_fields<'a>(
    value: &'a Value,
    expected: &[&'a str],
    label: &str,
) -> Result<BTreeMap<&'a str, &'a Value>, String> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("{label} is not a JSON object"))?;
    for key in object.keys() {
        if !expected.contains(&key.as_str()) {
            return Err(format!("{label} has unknown field `{key}`"));
        }
    }
    let mut fields = BTreeMap::new();
    for key in expected {
        let field = object
            .get(*key)
            .ok_or_else(|| format!("{label} is missing required field `{key}`"))?;
        fields.insert(*key, field);
    }
    Ok(fields)
}

fn field_str(fields: &BTreeMap<&str, &Value>, key: &str, label: &str) -> Result<String, String> {
    fields
        .get(key)
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .ok_or_else(|| format!("{label}.{key} is not a string"))
}

fn field_bool(fields: &BTreeMap<&str, &Value>, key: &str, label: &str) -> Result<bool, String> {
    fields
        .get(key)
        .and_then(|value| value.as_bool())
        .ok_or_else(|| format!("{label}.{key} is not a boolean"))
}

fn field_u64(fields: &BTreeMap<&str, &Value>, key: &str, label: &str) -> Result<u64, String> {
    fields
        .get(key)
        .and_then(|value| value.as_u64())
        .ok_or_else(|| format!("{label}.{key} is not an unsigned integer"))
}

fn field_f64(fields: &BTreeMap<&str, &Value>, key: &str, label: &str) -> Result<f64, String> {
    fields
        .get(key)
        .and_then(|value| value.as_f64())
        .ok_or_else(|| format!("{label}.{key} is not a number"))
}

fn field_array<'a>(
    fields: &BTreeMap<&'a str, &'a Value>,
    key: &str,
    label: &str,
) -> Result<&'a Vec<Value>, String> {
    fields
        .get(key)
        .and_then(|value| value.as_array())
        .ok_or_else(|| format!("{label}.{key} is not an array"))
}

fn field_string_array(
    fields: &BTreeMap<&str, &Value>,
    key: &str,
    label: &str,
) -> Result<Vec<String>, String> {
    field_array(fields, key, label)?
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_string)
                .ok_or_else(|| format!("{label}.{key} contains a non-string"))
        })
        .collect()
}

// -----------------------------------------------------------------------------
// Fixture schema
// -----------------------------------------------------------------------------

/// Closed wall strategy. Only the recorded Classic capture is a Gate A case.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WallStrategy {
    Classic,
}

impl WallStrategy {
    fn parse(value: &Value, label: &str) -> Result<Self, String> {
        match value.as_str() {
            Some("Classic") => Ok(Self::Classic),
            other => Err(format!(
                "{label}.wall_strategy must be the closed value `Classic`, got {other:?}"
            )),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Classic => "Classic",
        }
    }
}

#[derive(Debug, Clone)]
pub struct GateInputsFile {
    pub schema_version: u64,
    pub cases: Vec<GateInputCase>,
}

impl GateInputsFile {
    /// Strict decode: unknown fields, wrong types and a missing `b_input` key all
    /// fail. `b_input` is transported but never parsed by A, so it stays opaque.
    pub fn from_value(value: &Value) -> Result<Self, String> {
        let fields = strict_fields(value, &["schema_version", "cases"], INPUTS_FILE)?;
        let schema_version = field_u64(&fields, "schema_version", INPUTS_FILE)?;
        let cases = field_array(&fields, "cases", INPUTS_FILE)?
            .iter()
            .enumerate()
            .map(|(index, case)| GateInputCase::from_value(case, index))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            schema_version,
            cases,
        })
    }
}

/// One portable input case.
#[derive(Debug, Clone)]
pub struct GateInputCase {
    pub case_id: String,
    pub wall_strategy: WallStrategy,
    pub a_source: GateASourceInput,
    pub b_input: Value,
}

impl GateInputCase {
    pub fn from_value(value: &Value, index: usize) -> Result<Self, String> {
        let label = format!("{INPUTS_FILE}.cases[{index}]");
        let fields = strict_fields(
            value,
            &["case_id", "wall_strategy", "a_source", "b_input"],
            &label,
        )?;
        let case_id = field_str(&fields, "case_id", &label)?;
        let wall_strategy = WallStrategy::parse(fields["wall_strategy"], &label)?;
        let a_source: GateASourceInput = serde_json::from_value(fields["a_source"].clone())
            .map_err(|error| format!("{label}.a_source is not a strict GateASourceInput: {error}"))?;
        Ok(Self {
            case_id,
            wall_strategy,
            a_source,
            b_input: fields["b_input"].clone(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct GateOracleFile {
    pub schema_version: u64,
    pub cases: Vec<GateOracleCase>,
}

impl GateOracleFile {
    pub fn from_value(value: &Value) -> Result<Self, String> {
        let fields = strict_fields(value, &["schema_version", "cases"], CANONICAL_FILE)?;
        let schema_version = field_u64(&fields, "schema_version", CANONICAL_FILE)?;
        let cases = field_array(&fields, "cases", CANONICAL_FILE)?
            .iter()
            .enumerate()
            .map(|(index, case)| GateOracleCase::from_value(case, index))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            schema_version,
            cases,
        })
    }
}

/// One recorded oracle case. The A reference fields are typed; the B reference
/// fields (`graph`, `clipped`, `final_fills`) stay opaque because Gate A never
/// reads them.
#[derive(Debug, Clone)]
pub struct GateOracleCase {
    pub case_id: String,
    pub wall_strategy: WallStrategy,
    pub reference: GateReference,
}

impl GateOracleCase {
    pub fn from_value(value: &Value, index: usize) -> Result<Self, String> {
        let label = format!("{CANONICAL_FILE}.cases[{index}]");
        let fields = strict_fields(value, &["case_id", "wall_strategy", "reference"], &label)?;
        let case_id = field_str(&fields, "case_id", &label)?;
        let wall_strategy = WallStrategy::parse(fields["wall_strategy"], &label)?;
        let reference = GateReference::from_value(&fields["reference"], &format!("{label}.reference"))?;
        Ok(Self {
            case_id,
            wall_strategy,
            reference,
        })
    }
}

#[derive(Debug, Clone)]
pub struct GateReference {
    pub planning_layers: Vec<PlanningLayer>,
    pub policies: Option<Vec<DerivedRegionPolicy>>,
    pub parameters: LightningParameters,
    pub scalar_source_region: RegionIdentity,
    pub replay: GateReplay,
    pub graph: Value,
    pub clipped: Value,
    pub final_fills: Value,
}

impl GateReference {
    fn from_value(value: &Value, label: &str) -> Result<Self, String> {
        let fields = strict_fields(
            value,
            &[
                "planning_layers",
                "policies",
                "parameters",
                "scalar_source_region",
                "replay",
                "graph",
                "clipped",
                "final_fills",
            ],
            label,
        )?;
        let planning_layers = serde_json::from_value::<Vec<PlanningLayer>>(
            fields["planning_layers"].clone(),
        )
        .map_err(|error| format!("{label}.planning_layers is not a strict layer list: {error}"))?;
        let policies = if fields["policies"].is_null() {
            None
        } else {
            Some(
                serde_json::from_value::<Vec<DerivedRegionPolicy>>(fields["policies"].clone())
                    .map_err(|error| {
                        format!("{label}.policies is not a strict policy list: {error}")
                    })?,
            )
        };
        let parameters = serde_json::from_value::<LightningParameters>(fields["parameters"].clone())
            .map_err(|error| format!("{label}.parameters is not strict: {error}"))?;
        let scalar_source_region =
            serde_json::from_value::<RegionIdentity>(fields["scalar_source_region"].clone())
                .map_err(|error| format!("{label}.scalar_source_region is not strict: {error}"))?;
        Ok(Self {
            planning_layers,
            policies,
            parameters,
            scalar_source_region,
            replay: GateReplay::from_value(&fields["replay"], &format!("{label}.replay"))?,
            graph: fields["graph"].clone(),
            clipped: fields["clipped"].clone(),
            final_fills: fields["final_fills"].clone(),
        })
    }
}

/// Re-encode the typed reference exactly as recorded. Step 1's test asserts this
/// equals the raw fixture subtree before the reference digest is trusted.
pub fn reference_to_value(reference: &GateReference) -> Value {
    let policies = match &reference.policies {
        None => Value::Null,
        Some(policies) => serde_json::to_value(policies).expect("policies serialize"),
    };
    json!({
        "planning_layers": serde_json::to_value(&reference.planning_layers).expect("layers serialize"),
        "policies": policies,
        "parameters": serde_json::to_value(&reference.parameters).expect("parameters serialize"),
        "scalar_source_region": serde_json::to_value(&reference.scalar_source_region).expect("identity serializes"),
        "replay": replay_to_value(&reference.replay),
        "graph": reference.graph,
        "clipped": reference.clipped,
        "final_fills": reference.final_fills,
    })
}

#[derive(Debug, Clone)]
pub struct GateReplay {
    pub domain_symmetric_difference_area: f64,
    pub faithful_domain_polygon_count: u64,
    pub faithful_matches_production: bool,
    pub production: ReplayGraph,
    pub scalars_resolved: bool,
    pub seed_policy: String,
    pub snapshot_after: String,
    pub snapshot_before: String,
    pub snapshot_unchanged: bool,
    pub topology_differs: bool,
    pub wrong: ReplayGraph,
    pub wrong_domain_polygon_count: u64,
    pub wrong_nonempty: bool,
    pub faithful: ReplayGraph,
}

impl GateReplay {
    fn from_value(value: &Value, label: &str) -> Result<Self, String> {
        let fields = strict_fields(
            value,
            &[
                "domain_symmetric_difference_area",
                "faithful_domain_polygon_count",
                "faithful_matches_production",
                "production",
                "scalars_resolved",
                "seed_policy",
                "snapshot_after",
                "snapshot_before",
                "snapshot_unchanged",
                "topology_differs",
                "wrong",
                "wrong_domain_polygon_count",
                "wrong_nonempty",
                "faithful",
            ],
            label,
        )?;
        Ok(Self {
            domain_symmetric_difference_area: field_f64(
                &fields,
                "domain_symmetric_difference_area",
                label,
            )?,
            faithful_domain_polygon_count: field_u64(
                &fields,
                "faithful_domain_polygon_count",
                label,
            )?,
            faithful_matches_production: field_bool(
                &fields,
                "faithful_matches_production",
                label,
            )?,
            production: ReplayGraph::from_value(
                fields["production"],
                &format!("{label}.production"),
            )?,
            scalars_resolved: field_bool(&fields, "scalars_resolved", label)?,
            seed_policy: field_str(&fields, "seed_policy", label)?,
            snapshot_after: field_str(&fields, "snapshot_after", label)?,
            snapshot_before: field_str(&fields, "snapshot_before", label)?,
            snapshot_unchanged: field_bool(&fields, "snapshot_unchanged", label)?,
            topology_differs: field_bool(&fields, "topology_differs", label)?,
            wrong: ReplayGraph::from_value(fields["wrong"], &format!("{label}.wrong"))?,
            wrong_domain_polygon_count: field_u64(&fields, "wrong_domain_polygon_count", label)?,
            wrong_nonempty: field_bool(&fields, "wrong_nonempty", label)?,
            faithful: ReplayGraph::from_value(fields["faithful"], &format!("{label}.faithful"))?,
        })
    }
}

fn replay_to_value(replay: &GateReplay) -> Value {
    json!({
        "domain_symmetric_difference_area": replay.domain_symmetric_difference_area,
        "faithful_domain_polygon_count": replay.faithful_domain_polygon_count,
        "faithful_matches_production": replay.faithful_matches_production,
        "production": replay_graph_to_value(&replay.production),
        "scalars_resolved": replay.scalars_resolved,
        "seed_policy": replay.seed_policy,
        "snapshot_after": replay.snapshot_after,
        "snapshot_before": replay.snapshot_before,
        "snapshot_unchanged": replay.snapshot_unchanged,
        "topology_differs": replay.topology_differs,
        "wrong": replay_graph_to_value(&replay.wrong),
        "wrong_domain_polygon_count": replay.wrong_domain_polygon_count,
        "wrong_nonempty": replay.wrong_nonempty,
        "faithful": replay_graph_to_value(&replay.faithful),
    })
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReplayGraph {
    pub graph: Vec<ReplayLayer>,
    pub node_count: u64,
    pub edge_count: u64,
    pub root_count: u64,
}

impl ReplayGraph {
    fn from_value(value: &Value, label: &str) -> Result<Self, String> {
        let fields = strict_fields(value, &["graph", "node_count", "edge_count", "root_count"], label)?;
        let graph = field_array(&fields, "graph", label)?
            .iter()
            .enumerate()
            .map(|(index, layer)| ReplayLayer::from_value(layer, &format!("{label}.graph[{index}]")))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            graph,
            node_count: field_u64(&fields, "node_count", label)?,
            edge_count: field_u64(&fields, "edge_count", label)?,
            root_count: field_u64(&fields, "root_count", label)?,
        })
    }
}

fn replay_graph_to_value(graph: &ReplayGraph) -> Value {
    json!({
        "graph": graph.graph.iter().map(replay_layer_to_value).collect::<Vec<_>>(),
        "node_count": graph.node_count,
        "edge_count": graph.edge_count,
        "root_count": graph.root_count,
    })
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReplayLayer {
    pub layer_index: u64,
    pub nodes: Vec<ReplayNode>,
    pub edges: Vec<ReplayEdge>,
    pub root_count: u64,
}

impl ReplayLayer {
    fn from_value(value: &Value, label: &str) -> Result<Self, String> {
        let fields = strict_fields(value, &["layer_index", "nodes", "edges", "root_count"], label)?;
        let nodes = field_array(&fields, "nodes", label)?
            .iter()
            .enumerate()
            .map(|(index, node)| ReplayNode::from_value(node, &format!("{label}.nodes[{index}]")))
            .collect::<Result<Vec<_>, _>>()?;
        let edges = field_array(&fields, "edges", label)?
            .iter()
            .enumerate()
            .map(|(index, edge)| ReplayEdge::from_value(edge, &format!("{label}.edges[{index}]")))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            layer_index: field_u64(&fields, "layer_index", label)?,
            nodes,
            edges,
            root_count: field_u64(&fields, "root_count", label)?,
        })
    }
}

fn replay_layer_to_value(layer: &ReplayLayer) -> Value {
    json!({
        "layer_index": layer.layer_index,
        "nodes": layer.nodes.iter().map(replay_node_to_value).collect::<Vec<_>>(),
        "edges": layer.edges.iter().map(replay_edge_to_value).collect::<Vec<_>>(),
        "root_count": layer.root_count,
    })
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReplayNode {
    pub is_root: bool,
    pub last_grounding: Option<[i64; 2]>,
    pub node_id: u64,
    /// `-1` is the recorder's sentinel for "no parent".
    pub parent_id: i64,
    pub point: [i64; 2],
    pub point_mm: [f64; 2],
    pub root_id: u64,
}

impl ReplayNode {
    fn from_value(value: &Value, label: &str) -> Result<Self, String> {
        let fields = strict_fields(
            value,
            &[
                "is_root",
                "last_grounding",
                "node_id",
                "parent_id",
                "point",
                "point_mm",
                "root_id",
            ],
            label,
        )?;
        let last_grounding = if fields["last_grounding"].is_null() {
            None
        } else {
            Some(point_array(fields["last_grounding"], &format!("{label}.last_grounding"))?)
        };
        let point = point_array(fields["point"], &format!("{label}.point"))?;
        let point_mm = f64_pair_array(fields["point_mm"], &format!("{label}.point_mm"))?;
        Ok(Self {
            is_root: field_bool(&fields, "is_root", label)?,
            last_grounding,
            node_id: field_u64(&fields, "node_id", label)?,
            parent_id: fields["parent_id"]
                .as_i64()
                .ok_or_else(|| format!("{label}.parent_id is not a signed integer"))?,
            point,
            point_mm,
            root_id: field_u64(&fields, "root_id", label)?,
        })
    }
}

fn replay_node_to_value(node: &ReplayNode) -> Value {
    json!({
        "is_root": node.is_root,
        "last_grounding": node.last_grounding.map(|point| json!([point[0], point[1]])),
        "node_id": node.node_id,
        "parent_id": node.parent_id,
        "point": node.point,
        "point_mm": node.point_mm,
        "root_id": node.root_id,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayEdge {
    pub child_id: u64,
    pub parent_id: u64,
    pub root_id: u64,
}

impl ReplayEdge {
    fn from_value(value: &Value, label: &str) -> Result<Self, String> {
        let fields = strict_fields(value, &["child_id", "parent_id", "root_id"], label)?;
        Ok(Self {
            child_id: field_u64(&fields, "child_id", label)?,
            parent_id: field_u64(&fields, "parent_id", label)?,
            root_id: field_u64(&fields, "root_id", label)?,
        })
    }
}

fn replay_edge_to_value(edge: &ReplayEdge) -> Value {
    json!({
        "child_id": edge.child_id,
        "parent_id": edge.parent_id,
        "root_id": edge.root_id,
    })
}

fn point_array(value: &Value, label: &str) -> Result<[i64; 2], String> {
    let array = value
        .as_array()
        .ok_or_else(|| format!("{label} is not an array"))?;
    if array.len() != 2 {
        return Err(format!("{label} is not a two-element array"));
    }
    let x = array[0]
        .as_i64()
        .ok_or_else(|| format!("{label}[0] is not an integer"))?;
    let y = array[1]
        .as_i64()
        .ok_or_else(|| format!("{label}[1] is not an integer"))?;
    Ok([x, y])
}

fn f64_pair_array(value: &Value, label: &str) -> Result<[f64; 2], String> {
    let array = value
        .as_array()
        .ok_or_else(|| format!("{label} is not an array"))?;
    if array.len() != 2 {
        return Err(format!("{label} is not a two-element array"));
    }
    let x = array[0]
        .as_f64()
        .ok_or_else(|| format!("{label}[0] is not a number"))?;
    let y = array[1]
        .as_f64()
        .ok_or_else(|| format!("{label}[1] is not a number"))?;
    Ok([x, y])
}

#[derive(Debug, Clone)]
pub struct GateManifest {
    pub schema_version: u64,
    pub gate: String,
    pub status: String,
    pub case_ids: Vec<String>,
    pub capture: ManifestCapture,
    pub units: ManifestUnits,
    pub cases: BTreeMap<String, ManifestCase>,
}

impl GateManifest {
    pub fn from_value(value: &Value) -> Result<Self, String> {
        let fields = strict_fields(
            value,
            &[
                "schema_version",
                "gate",
                "status",
                "case_ids",
                "capture",
                "units",
                "cases",
            ],
            MANIFEST_FILE,
        )?;
        let cases_object = fields["cases"]
            .as_object()
            .ok_or_else(|| format!("{MANIFEST_FILE}.cases is not an object"))?;
        let mut cases = BTreeMap::new();
        for (case_id, case_value) in cases_object {
            cases.insert(
                case_id.clone(),
                ManifestCase::from_value(case_value, &format!("{MANIFEST_FILE}.cases[{case_id}]"))?,
            );
        }
        Ok(Self {
            schema_version: field_u64(&fields, "schema_version", MANIFEST_FILE)?,
            gate: field_str(&fields, "gate", MANIFEST_FILE)?,
            status: field_str(&fields, "status", MANIFEST_FILE)?,
            case_ids: field_string_array(&fields, "case_ids", MANIFEST_FILE)?,
            capture: ManifestCapture::from_value(fields["capture"], "manifest.capture")?,
            units: ManifestUnits::from_value(fields["units"], "manifest.units")?,
            cases,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ManifestCapture {
    pub manifest_sha256: String,
    pub canonical_revision: String,
    pub canonical_build_revision: String,
    pub canonical_build_dirty: bool,
    pub recorder_patch_sha256: String,
    pub recorder_patch_applied: bool,
    pub tools: ManifestTools,
}

impl ManifestCapture {
    fn from_value(value: &Value, label: &str) -> Result<Self, String> {
        let fields = strict_fields(
            value,
            &[
                "manifest_sha256",
                "canonical_revision",
                "canonical_build_revision",
                "canonical_build_dirty",
                "recorder_patch_sha256",
                "recorder_patch_applied",
                "tools",
            ],
            label,
        )?;
        Ok(Self {
            manifest_sha256: field_str(&fields, "manifest_sha256", label)?,
            canonical_revision: field_str(&fields, "canonical_revision", label)?,
            canonical_build_revision: field_str(&fields, "canonical_build_revision", label)?,
            canonical_build_dirty: field_bool(&fields, "canonical_build_dirty", label)?,
            recorder_patch_sha256: field_str(&fields, "recorder_patch_sha256", label)?,
            recorder_patch_applied: field_bool(&fields, "recorder_patch_applied", label)?,
            tools: ManifestTools::from_value(fields["tools"], &format!("{label}.tools"))?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ManifestTools {
    pub platform: String,
    pub python: String,
    pub slicer_sha256: String,
}

impl ManifestTools {
    fn from_value(value: &Value, label: &str) -> Result<Self, String> {
        let fields = strict_fields(value, &["platform", "python", "slicer_sha256"], label)?;
        Ok(Self {
            platform: field_str(&fields, "platform", label)?,
            python: field_str(&fields, "python", label)?,
            slicer_sha256: field_str(&fields, "slicer_sha256", label)?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ManifestUnits {
    pub canonical: String,
    pub pnp: String,
    pub pnp_units_per_canonical_unit: u64,
    pub reference: String,
    pub transport: ManifestTransport,
}

impl ManifestUnits {
    fn from_value(value: &Value, label: &str) -> Result<Self, String> {
        let fields = strict_fields(
            value,
            &[
                "canonical",
                "pnp",
                "pnp_units_per_canonical_unit",
                "reference",
                "transport",
            ],
            label,
        )?;
        Ok(Self {
            canonical: field_str(&fields, "canonical", label)?,
            pnp: field_str(&fields, "pnp", label)?,
            pnp_units_per_canonical_unit: field_u64(
                &fields,
                "pnp_units_per_canonical_unit",
                label,
            )?,
            reference: field_str(&fields, "reference", label)?,
            transport: ManifestTransport::from_value(
                fields["transport"],
                &format!("{label}.transport"),
            )?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ManifestTransport {
    pub applies_to: Vec<String>,
    pub conversion: String,
    pub tolerance_pnp_units: i64,
}

impl ManifestTransport {
    fn from_value(value: &Value, label: &str) -> Result<Self, String> {
        let fields = strict_fields(
            value,
            &["applies_to", "conversion", "tolerance_pnp_units"],
            label,
        )?;
        Ok(Self {
            applies_to: field_string_array(&fields, "applies_to", label)?,
            conversion: field_str(&fields, "conversion", label)?,
            tolerance_pnp_units: fields["tolerance_pnp_units"]
                .as_i64()
                .ok_or_else(|| format!("{label}.tolerance_pnp_units is not an integer"))?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ManifestCase {
    pub a_source_sha256: String,
    pub a_raw_config_sha256: String,
    pub a_earlier_provenance_sha256: String,
    pub b_input_sha256: String,
    pub reference_sha256: String,
    pub contributor_evidence: ContributorEvidenceSummary,
    /// Named absent-reason table, e.g. `reference.policies`.
    pub absent: BTreeMap<String, String>,
}

impl ManifestCase {
    fn from_value(value: &Value, label: &str) -> Result<Self, String> {
        let fields = strict_fields(
            value,
            &[
                "a_source_sha256",
                "a_raw_config_sha256",
                "a_earlier_provenance_sha256",
                "b_input_sha256",
                "reference_sha256",
                "contributor_evidence",
                "absent",
            ],
            label,
        )?;
        let absent_object = fields["absent"]
            .as_object()
            .ok_or_else(|| format!("{label}.absent is not an object"))?;
        let mut absent = BTreeMap::new();
        for (key, reason) in absent_object {
            let reason = reason
                .as_str()
                .ok_or_else(|| format!("{label}.absent[{key}] is not a string"))?;
            absent.insert(key.clone(), reason.to_string());
        }
        Ok(Self {
            a_source_sha256: field_str(&fields, "a_source_sha256", label)?,
            a_raw_config_sha256: field_str(&fields, "a_raw_config_sha256", label)?,
            a_earlier_provenance_sha256: field_str(&fields, "a_earlier_provenance_sha256", label)?,
            b_input_sha256: field_str(&fields, "b_input_sha256", label)?,
            reference_sha256: field_str(&fields, "reference_sha256", label)?,
            contributor_evidence: ContributorEvidenceSummary::from_value(
                fields["contributor_evidence"],
                &format!("{label}.contributor_evidence"),
            )?,
            absent,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ContributorEvidenceSummary {
    pub complete: u64,
    pub unavailable: u64,
    pub unavailable_dispatches: Vec<String>,
}

impl ContributorEvidenceSummary {
    fn from_value(value: &Value, label: &str) -> Result<Self, String> {
        let fields = strict_fields(
            value,
            &["complete", "unavailable", "unavailable_dispatches"],
            label,
        )?;
        Ok(Self {
            complete: field_u64(&fields, "complete", label)?,
            unavailable: field_u64(&fields, "unavailable", label)?,
            unavailable_dispatches: field_string_array(&fields, "unavailable_dispatches", label)?,
        })
    }
}

/// Everything loaded from the fixture directory.
pub struct Fixtures {
    pub inputs: GateInputsFile,
    pub oracle: GateOracleFile,
    pub manifest: GateManifest,
    pub inputs_raw: Value,
    pub canonical_raw: Value,
    pub inputs_text: String,
    pub canonical_text: String,
}

pub fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(FIXTURE_DIR_REL)
}

pub fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn outcome_path() -> PathBuf {
    workspace_root().join("target/lightning-oracle/outcome.json")
}

pub fn load_fixtures() -> Result<Fixtures, String> {
    load_fixtures_from(&fixture_dir())
}

/// Strict load of the three portable files. A missing file is an error, never
/// a skip, and no sibling OrcaSlicer checkout is consulted.
pub fn load_fixtures_from(dir: &Path) -> Result<Fixtures, String> {
    let inputs_text = read_text(&dir.join(INPUTS_FILE))?;
    let canonical_text = read_text(&dir.join(CANONICAL_FILE))?;
    let manifest_text = read_text(&dir.join(MANIFEST_FILE))?;
    let inputs_raw = parse_json(&inputs_text, &dir.join(INPUTS_FILE))?;
    let canonical_raw = parse_json(&canonical_text, &dir.join(CANONICAL_FILE))?;
    let manifest_raw = parse_json(&manifest_text, &dir.join(MANIFEST_FILE))?;
    let inputs = GateInputsFile::from_value(&inputs_raw)?;
    let oracle = GateOracleFile::from_value(&canonical_raw)?;
    let manifest = GateManifest::from_value(&manifest_raw)?;
    Ok(Fixtures {
        inputs,
        oracle,
        manifest,
        inputs_raw,
        canonical_raw,
        inputs_text,
        canonical_text,
    })
}

fn read_text(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path)
        .map_err(|error| format!("fixture file {} is unreadable: {error}", path.display()))
}

fn parse_json(text: &str, path: &Path) -> Result<Value, String> {
    serde_json::from_str(text)
        .map_err(|error| format!("fixture file {} is not valid JSON: {error}", path.display()))
}

fn read_json(path: &Path) -> Result<Value, String> {
    parse_json(&read_text(path)?, path)
}

/// The manifest, inputs and canonical files must name exactly the same cases.
/// A missing case is a named error, never a silently smaller population.
pub fn validate_population(fixtures: &Fixtures) -> Result<(), String> {
    validate_population_values(&fixtures.inputs, &fixtures.oracle, &fixtures.manifest)
}

pub fn validate_population_values(
    inputs: &GateInputsFile,
    oracle: &GateOracleFile,
    manifest: &GateManifest,
) -> Result<(), String> {
    let mut problems = Vec::new();
    let input_ids: BTreeSet<&str> = inputs.cases.iter().map(|case| case.case_id.as_str()).collect();
    let oracle_ids: BTreeSet<&str> = oracle.cases.iter().map(|case| case.case_id.as_str()).collect();
    let manifest_ids: BTreeSet<&str> = manifest.case_ids.iter().map(String::as_str).collect();

    if input_ids.len() != inputs.cases.len() {
        problems.push("inputs.json carries a duplicate case_id".to_string());
    }
    if oracle_ids.len() != oracle.cases.len() {
        problems.push("canonical.json carries a duplicate case_id".to_string());
    }
    for missing in manifest_ids.difference(&input_ids) {
        problems.push(format!("inputs.json is missing manifest case `{missing}`"));
    }
    for extra in input_ids.difference(&manifest_ids) {
        problems.push(format!("inputs.json carries unmanifested case `{extra}`"));
    }
    for missing in manifest_ids.difference(&oracle_ids) {
        problems.push(format!("canonical.json is missing manifest case `{missing}`"));
    }
    for extra in oracle_ids.difference(&manifest_ids) {
        problems.push(format!("canonical.json carries unmanifested case `{extra}`"));
    }
    for id in &manifest_ids {
        if !manifest.cases.contains_key(*id) {
            problems.push(format!("manifest.cases has no evidence row for `{id}`"));
        }
    }
    for expected in EXPECTED_CASE_IDS {
        if !manifest_ids.contains(expected) {
            problems.push(format!("manifest is missing Classic case `{expected}`"));
        }
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("; "))
    }
}

pub fn input_case<'a>(fixtures: &'a Fixtures, case_id: &str) -> Result<&'a GateInputCase, String> {
    fixtures
        .inputs
        .cases
        .iter()
        .find(|case| case.case_id == case_id)
        .ok_or_else(|| format!("inputs.json has no case `{case_id}`"))
}

pub fn oracle_case<'a>(fixtures: &'a Fixtures, case_id: &str) -> Result<&'a GateOracleCase, String> {
    fixtures
        .oracle
        .cases
        .iter()
        .find(|case| case.case_id == case_id)
        .ok_or_else(|| format!("canonical.json has no case `{case_id}`"))
}

pub fn manifest_case<'a>(fixtures: &'a Fixtures, case_id: &str) -> Result<&'a ManifestCase, String> {
    fixtures
        .manifest
        .cases
        .get(case_id)
        .ok_or_else(|| format!("manifest.json has no evidence row for `{case_id}`"))
}

// -----------------------------------------------------------------------------
// Projector / extractor: the restricted source-only path
// -----------------------------------------------------------------------------

/// Select `a_source` from a case view, serialize it unchanged, and strictly
/// deserialize the restricted subtree. Reference-like keys inside `a_source`
/// fail the closed DTO before any construction runs.
pub fn project_gate_a(view: &Value) -> Result<GateASourceInput, KernelError> {
    let bytes = project_gate_a_bytes(view)?;
    extract_gate_a_source(&bytes)
}

/// The exact bytes the projector hands to the extractor. Holding them fixed is
/// AC-4's isolation assertion; the bytes are the typed A subtree only.
pub fn project_gate_a_bytes(view: &Value) -> Result<Vec<u8>, KernelError> {
    let source = view
        .get("a_source")
        .ok_or_else(|| KernelError::RequiredSourceInputUnavailable {
            context: empty_location(),
            field: "a_source".to_string(),
            reason:
                "the case view has no `a_source` key; only the restricted source subtree is a Gate A input"
                    .to_string(),
        })?;
    serde_json::to_vec(source).map_err(|error| KernelError::UnrepresentableTransport {
        context: empty_location(),
        reason: format!("the selected `a_source` subtree cannot be serialized: {error}"),
    })
}

pub fn extract_gate_a_source(source_bytes: &[u8]) -> Result<GateASourceInput, KernelError> {
    serde_json::from_slice(source_bytes).map_err(|error| KernelError::InvalidInput {
        context: empty_location(),
        reason: format!("the restricted `a_source` subtree is not a strict GateASourceInput: {error}"),
    })
}

fn empty_location() -> lightning_feasibility_kernel::EvidenceLocation {
    lightning_feasibility_kernel::EvidenceLocation {
        object_id: "gate-a-oracle".to_string(),
        global_layer_index: 0,
        region: None,
        source: None,
        checkpoint: None,
    }
}

/// Build the case view the projector sees. This is the only input handed to
/// construction; `b_input` is present in the view (the projector must ignore
/// it) and no reference is included.
pub fn case_view(case: &GateInputCase) -> Result<Value, String> {
    Ok(json!({
        "case_id": case.case_id,
        "wall_strategy": case.wall_strategy.as_str(),
        "a_source": serde_json::to_value(&case.a_source)
            .map_err(|error| format!("case {} a_source cannot be serialized: {error}", case.case_id))?,
        "b_input": case.b_input,
    }))
}

pub fn oracle_case_to_value(case: &GateOracleCase) -> Value {
    json!({
        "case_id": case.case_id,
        "wall_strategy": case.wall_strategy.as_str(),
        "reference": reference_to_value(&case.reference),
    })
}

pub struct CaseExecution {
    pub projected: Vec<u8>,
    pub construction: Option<GateAConstruction>,
    pub error: Option<KernelError>,
}

/// Run the three-stage source-only path: project -> extract -> construct.
pub fn execute_case(view: &Value) -> CaseExecution {
    let projected = match project_gate_a_bytes(view) {
        Ok(bytes) => bytes,
        Err(error) => {
            return CaseExecution {
                projected: Vec::new(),
                construction: None,
                error: Some(error),
            }
        }
    };
    let source = match extract_gate_a_source(&projected) {
        Ok(source) => source,
        Err(error) => {
            return CaseExecution {
                projected,
                construction: None,
                error: Some(error),
            }
        }
    };
    match build_planning_domain(&source) {
        Ok(construction) => CaseExecution {
            projected,
            construction: Some(construction),
            error: None,
        },
        Err(error) => CaseExecution {
            projected,
            construction: None,
            error: Some(error),
        },
    }
}

// -----------------------------------------------------------------------------
// Canonical JSON and evidence hashes
// -----------------------------------------------------------------------------

/// SHA-256 over canonical JSON is the recorder's evidence digest:
/// `json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)`.
pub fn canonical_json(value: &Value) -> String {
    let mut out = String::new();
    write_canonical(value, &mut out);
    out
}

fn write_canonical(value: &Value, out: &mut String) {
    match value {
        Value::Object(map) => {
            out.push('{');
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            for (index, key) in keys.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                out.push_str(&serde_json::to_string(key).expect("string key serializes"));
                out.push(':');
                write_canonical(&map[*key], out);
            }
            out.push('}');
        }
        Value::Array(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_canonical(item, out);
            }
            out.push(']');
        }
        scalar => out.push_str(&serde_json::to_string(scalar).expect("JSON scalar serializes")),
    }
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

pub fn sha256_canonical(value: &Value) -> String {
    sha256_hex(canonical_json(value).as_bytes())
}

pub fn sha256_file(path: &Path) -> Result<String, String> {
    let bytes =
        std::fs::read(path).map_err(|error| format!("cannot hash {}: {error}", path.display()))?;
    Ok(sha256_hex(&bytes))
}

pub fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub fn is_hash_or_absent(value: &str) -> bool {
    value == "absent" || is_sha256_hex(value)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseEvidenceHashes {
    pub a_source_sha256: String,
    pub a_raw_config_sha256: String,
    pub a_earlier_provenance_sha256: String,
    pub b_input_sha256: String,
    pub reference_sha256: String,
}

/// The three source/config/provenance digests derived only from typed A fields.
///
/// `typed_a_source_sha256` is the digest of a **typed re-encode**, and is
/// deliberately not named `a_source_sha256`: `CaseEvidenceHashes::a_source_sha256`
/// is the digest of the recorded raw subtree text, which is the value written to
/// the manifest and to `outcome.json`. For the same case the two digests differ
/// (typed re-encoding is not byte-identical to the published subtree), so giving
/// them the same name would read as a phantom digest change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedSourceHashes {
    pub typed_a_source_sha256: String,
    pub a_raw_config_sha256: String,
    pub a_earlier_provenance_sha256: String,
}

pub fn source_hashes(source: &GateASourceInput) -> TypedSourceHashes {
    let a_source = serde_json::to_value(source).expect("GateASourceInput serializes");
    let mut regions = Vec::new();
    for layer in &source.layers {
        for region in &layer.regions {
            regions.push(json!({
                "identity": &region.identity,
                "config": &region.config,
            }));
        }
    }
    let raw_config = json!({
        "object_config": &source.object_config,
        "regions": regions,
    });
    let provenance = serde_json::to_value(&source.provenance).expect("provenance serializes");
    TypedSourceHashes {
        typed_a_source_sha256: sha256_canonical(&a_source),
        a_raw_config_sha256: sha256_canonical(&raw_config),
        a_earlier_provenance_sha256: sha256_canonical(&provenance),
    }
}

/// Recompute every evidence hash from its recorded subtree. The A hashes derive
/// only from the typed `GateASourceInput` fields (whole envelope never hashed).
///
/// `a_source` and `reference` are hashed from the exact canonical value text in
/// the checked-in file rather than from a typed re-encode: `serde_json`'s
/// default float parser is not round-trip exact (`1.5999999999999999` parses to
/// `1.6` and re-encodes as `1.6`), and the recorder's digest is over the
/// published bytes. Both files are written as canonical compact JSON, so the
/// value text is byte-identical to the recorder's canonical encoding. The typed
/// decode is asserted separately (strict envelope decode and the comparator).
pub fn recompute_hashes_for(fixtures: &Fixtures, case_id: &str) -> Result<CaseEvidenceHashes, String> {
    let input_index = fixtures
        .inputs
        .cases
        .iter()
        .position(|case| case.case_id == case_id)
        .ok_or_else(|| format!("inputs.json has no case `{case_id}`"))?;
    let oracle_index = fixtures
        .oracle
        .cases
        .iter()
        .position(|case| case.case_id == case_id)
        .ok_or_else(|| format!("canonical.json has no case `{case_id}`"))?;
    let input = &fixtures.inputs.cases[input_index];
    let typed_hashes = source_hashes(&input.a_source);
    let a_source_text = value_text_after(&fixtures.inputs_text, "a_source", input_index)?;
    let reference_text = value_text_after(&fixtures.canonical_text, "reference", oracle_index)?;
    // Cross-check: the extracted text is a JSON value, and its typed decode has
    // the same shape the envelope loader accepted.
    let _: Value = serde_json::from_str(a_source_text)
        .map_err(|error| format!("extracted a_source text is not JSON: {error}"))?;
    let _: Value = serde_json::from_str(reference_text)
        .map_err(|error| format!("extracted reference text is not JSON: {error}"))?;
    Ok(CaseEvidenceHashes {
        // The recorded slot is the raw-subtree digest, not `typed_hashes.typed_a_source_sha256`.
        a_source_sha256: sha256_hex(a_source_text.as_bytes()),
        a_raw_config_sha256: typed_hashes.a_raw_config_sha256,
        a_earlier_provenance_sha256: typed_hashes.a_earlier_provenance_sha256,
        b_input_sha256: sha256_canonical(&input.b_input),
        reference_sha256: sha256_hex(reference_text.as_bytes()),
    })
}

/// Return the exact source text of the `occurrence`-th value of `key`. The
/// fixture files are canonical compact JSON, so this is the recorder's hash
/// preimage for that subtree.
fn value_text_after<'a>(document: &'a str, key: &str, occurrence: usize) -> Result<&'a str, String> {
    let needle = format!("\"{key}\":");
    let start = document
        .match_indices(&needle)
        .nth(occurrence)
        .map(|(index, _)| index + needle.len())
        .ok_or_else(|| format!("fixture text has no occurrence {occurrence} of key `{key}`"))?;
    scan_json_value(&document[start..])
}

fn scan_json_value(text: &str) -> Result<&str, String> {
    let bytes = text.as_bytes();
    let first = *bytes.first().ok_or_else(|| "empty JSON value".to_string())?;
    match first {
        b'{' | b'[' => {
            let mut depth = 0usize;
            let mut in_string = false;
            let mut escaped = false;
            for (index, byte) in bytes.iter().enumerate() {
                if in_string {
                    if escaped {
                        escaped = false;
                    } else if *byte == b'\\' {
                        escaped = true;
                    } else if *byte == b'"' {
                        in_string = false;
                    }
                    continue;
                }
                match byte {
                    b'"' => in_string = true,
                    b'{' | b'[' => depth += 1,
                    b'}' | b']' => {
                        depth = depth
                            .checked_sub(1)
                            .ok_or_else(|| "unbalanced JSON value".to_string())?;
                        if depth == 0 {
                            return Ok(&text[..=index]);
                        }
                    }
                    _ => {}
                }
            }
            Err("unterminated JSON object/array".to_string())
        }
        b'"' => {
            let mut escaped = false;
            for (index, byte) in bytes.iter().enumerate().skip(1) {
                if escaped {
                    escaped = false;
                } else if *byte == b'\\' {
                    escaped = true;
                } else if *byte == b'"' {
                    return Ok(&text[..=index]);
                }
            }
            Err("unterminated JSON string".to_string())
        }
        _ => {
            let end = bytes
                .iter()
                .position(|byte| matches!(byte, b',' | b'}' | b']' | b'\n' | b'\r' | b' ' | b'\t'))
                .unwrap_or(bytes.len());
            Ok(&text[..end])
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct HashVerification {
    pub verified: Vec<&'static str>,
    pub absent: Vec<&'static str>,
    pub mismatches: Vec<String>,
}

pub fn verify_case_hashes(recorded: &ManifestCase, recomputed: &CaseEvidenceHashes) -> HashVerification {
    let mut verification = HashVerification::default();
    let slots: [(&'static str, &str, &str); 5] = [
        ("a_source_sha256", &recorded.a_source_sha256, &recomputed.a_source_sha256),
        (
            "a_raw_config_sha256",
            &recorded.a_raw_config_sha256,
            &recomputed.a_raw_config_sha256,
        ),
        (
            "a_earlier_provenance_sha256",
            &recorded.a_earlier_provenance_sha256,
            &recomputed.a_earlier_provenance_sha256,
        ),
        ("b_input_sha256", &recorded.b_input_sha256, &recomputed.b_input_sha256),
        ("reference_sha256", &recorded.reference_sha256, &recomputed.reference_sha256),
    ];
    for (name, recorded_value, recomputed_value) in slots {
        if !is_hash_or_absent(recorded_value) {
            verification.mismatches.push(format!(
                "manifest.{name} is neither a 64-hex digest nor the literal \"absent\": {recorded_value:?}"
            ));
        } else if recorded_value == "absent" {
            verification.absent.push(name);
        } else if recorded_value != recomputed_value {
            verification.mismatches.push(format!(
                "manifest.{name} does not match its subtree: recorded {recorded_value}, recomputed {recomputed_value}"
            ));
        } else {
            verification.verified.push(name);
        }
    }
    verification
}

// -----------------------------------------------------------------------------
// Comparator: independent recomputation from construction + recorded reference
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseStatus {
    Pass,
    Fail,
    Inconclusive,
}

pub fn parse_case_status(value: &Value, label: &str) -> Result<CaseStatus, String> {
    match value.as_str() {
        Some("pass") => Ok(CaseStatus::Pass),
        Some("fail") => Ok(CaseStatus::Fail),
        Some("inconclusive") => Ok(CaseStatus::Inconclusive),
        other => Err(format!(
            "{label} must be one of pass|fail|inconclusive, got {other:?}"
        )),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupVerdict {
    Pass,
    Fail,
    Inconclusive,
}

#[derive(Debug, Clone)]
pub struct GroupOutcome {
    pub group: String,
    pub verdict: GroupVerdict,
    pub mismatches: Vec<String>,
    pub unavailable: Vec<String>,
    pub detail: Vec<String>,
}

impl GroupOutcome {
    fn pass(group: &str) -> Self {
        Self {
            group: group.to_string(),
            verdict: GroupVerdict::Pass,
            mismatches: Vec::new(),
            unavailable: Vec::new(),
            detail: Vec::new(),
        }
    }

    fn finish(mut self) -> Self {
        self.verdict = if !self.mismatches.is_empty() {
            GroupVerdict::Fail
        } else if !self.unavailable.is_empty() {
            GroupVerdict::Inconclusive
        } else {
            GroupVerdict::Pass
        };
        self
    }

    fn named(&self) -> Vec<String> {
        let mut named = Vec::new();
        for mismatch in &self.mismatches {
            named.push(format!("{}: {mismatch}", self.group));
        }
        for reason in &self.unavailable {
            named.push(format!("{}: {reason}", self.group));
        }
        named
    }
}

#[derive(Debug, Clone)]
pub struct CaseDecision {
    pub case_id: String,
    pub status: CaseStatus,
    pub groups: Vec<GroupOutcome>,
    pub mismatches: Vec<String>,
    pub unavailable: Vec<String>,
    pub construction_available: bool,
}

impl CaseDecision {
    pub fn group(&self, name: &str) -> Option<&GroupOutcome> {
        self.groups.iter().find(|group| group.group == name)
    }

    pub fn named_evidence(&self) -> Vec<String> {
        self.groups.iter().flat_map(GroupOutcome::named).collect()
    }
}

/// Derive the case verdict from the per-group verdicts only. This is the
/// independent recomputation the recorded status must equal.
///
/// Precedence: a measured difference is `fail`; otherwise missing evidence is
/// `inconclusive`; only a fully compared case is `pass`.
pub fn recompute_status(groups: &[GroupOutcome]) -> CaseStatus {
    if groups.iter().any(|group| group.verdict == GroupVerdict::Fail) {
        CaseStatus::Fail
    } else if groups
        .iter()
        .any(|group| group.verdict == GroupVerdict::Inconclusive)
    {
        CaseStatus::Inconclusive
    } else {
        CaseStatus::Pass
    }
}

pub fn aggregate_status(decisions: &[CaseDecision]) -> CaseStatus {
    if decisions.iter().any(|case| case.status == CaseStatus::Fail) {
        CaseStatus::Fail
    } else if decisions
        .iter()
        .any(|case| case.status == CaseStatus::Inconclusive)
    {
        CaseStatus::Inconclusive
    } else {
        CaseStatus::Pass
    }
}

/// Compare one case's construction to the recorded reference. The construction
/// is read-only; no reference value is ever returned into it.
pub fn compare_case(
    input: &GateInputCase,
    oracle: &GateOracleCase,
    manifest_case: &ManifestCase,
    construction: Option<&GateAConstruction>,
    error: Option<&KernelError>,
) -> CaseDecision {
    if let Some(error) = error {
        let group = construction_error_group(error);
        let status = recompute_status(std::slice::from_ref(&group));
        return CaseDecision {
            case_id: input.case_id.clone(),
            status,
            mismatches: group.named(),
            unavailable: group.unavailable.clone(),
            groups: vec![group],
            construction_available: false,
        };
    }
    let Some(construction) = construction else {
        let mut group = GroupOutcome::pass("construction");
        group.unavailable.push(
            "the case produced neither a GateAConstruction nor a KernelError; no population is comparable"
                .to_string(),
        );
        let group = group.finish();
        let status = recompute_status(std::slice::from_ref(&group));
        return CaseDecision {
            case_id: input.case_id.clone(),
            status,
            mismatches: Vec::new(),
            unavailable: group.unavailable.clone(),
            groups: vec![group],
            construction_available: false,
        };
    };

    let groups = vec![
        compare_layer_identity(construction, &oracle.reference),
        compare_layer_population(construction, &oracle.reference),
        compare_domain_group("planning_layers.domains", construction, &oracle.reference),
        compare_source_regions(construction, &oracle.reference),
        compare_source_catalog(construction, &oracle.reference),
        compare_boundary_provenance_group(construction, &oracle.reference),
        compare_operations(construction, &oracle.reference),
        compare_parameters(construction, &oracle.reference),
        compare_scalar_source(construction, &oracle.reference),
        compare_policies(construction, &oracle.reference, manifest_case),
    ];

    let status = recompute_status(&groups);
    let mismatches: Vec<String> = groups
        .iter()
        .flat_map(|group| {
            group
                .mismatches
                .iter()
                .map(|mismatch| format!("{}: {mismatch}", group.group))
        })
        .collect();
    let unavailable: Vec<String> = groups
        .iter()
        .flat_map(|group| {
            group
                .unavailable
                .iter()
                .map(|reason| format!("{}: {reason}", group.group))
        })
        .collect();
    CaseDecision {
        case_id: input.case_id.clone(),
        status,
        groups,
        mismatches,
        unavailable,
        construction_available: true,
    }
}

fn construction_error_group(error: &KernelError) -> GroupOutcome {
    let mut group = GroupOutcome::pass("construction");
    let (fail, reason) = match error {
        KernelError::SourceProvenanceUnavailable { reason, .. }
        | KernelError::RequiredSourceInputUnavailable { reason, .. }
        | KernelError::UnrepresentableTransport { reason, .. } => (false, reason.clone()),
        KernelError::SourceProvenanceMismatch { .. }
        | KernelError::RequiredSourceInputMismatch { .. }
        | KernelError::InvalidInput { .. } => (true, error.to_string()),
    };
    if fail {
        group.mismatches.push(reason);
    } else {
        group.unavailable.push(format!("{error} ({reason})"));
    }
    group.finish()
}

fn recorded_layers(reference: &GateReference) -> BTreeMap<u32, &PlanningLayer> {
    let mut layers = BTreeMap::new();
    for layer in &reference.planning_layers {
        layers.insert(layer.global_layer_index, layer);
    }
    layers
}

fn constructed_layers(construction: &GateAConstruction) -> BTreeMap<u32, &PlanningLayer> {
    let mut layers = BTreeMap::new();
    for layer in &construction.planning_layers {
        layers.insert(layer.global_layer_index, layer);
    }
    layers
}

fn compare_layer_identity(construction: &GateAConstruction, reference: &GateReference) -> GroupOutcome {
    let mut group = GroupOutcome::pass("planning_layers.identity");
    let constructed = constructed_layers(construction);
    for (index, recorded) in recorded_layers(reference) {
        let Some(derived) = constructed.get(&index) else {
            continue;
        };
        if derived.object_id != recorded.object_id {
            group.mismatches.push(format!(
                "layer g{index} object_id: recorded {}, derived {}",
                recorded.object_id, derived.object_id
            ));
        }
        if derived.local_layer_index != recorded.local_layer_index {
            group.mismatches.push(format!(
                "layer g{index} local_layer_index: recorded {}, derived {}",
                recorded.local_layer_index, derived.local_layer_index
            ));
        }
        if (derived.z_mm - recorded.z_mm).abs() > SCALAR_TOLERANCE_MM {
            group.mismatches.push(format!(
                "layer g{index} z_mm: recorded {}, derived {} (tol {SCALAR_TOLERANCE_MM})",
                recorded.z_mm, derived.z_mm
            ));
        }
    }
    group.finish()
}

/// The recorded planning population is the reference's layer set. Construction
/// layers outside it are not comparable layer-for-layer, but they must carry no
/// planning activity; a planned layer the reference never recorded is a named
/// population mismatch, never a silent skip.
fn compare_layer_population(construction: &GateAConstruction, reference: &GateReference) -> GroupOutcome {
    let mut group = GroupOutcome::pass("planning_layers.population");
    let recorded = recorded_layers(reference);
    let constructed = constructed_layers(construction);
    if recorded.is_empty() {
        group
            .unavailable
            .push("the reference records no planning layer".to_string());
        return group.finish();
    }
    for index in recorded.keys() {
        match constructed.get(index) {
            None => group.mismatches.push(format!(
                "the reference records planning layer g{index} but the construction has none"
            )),
            Some(layer) => {
                if !layer.temporary_anchors.is_empty() || !layer.operations.is_empty() {
                    group.detail.push(format!(
                        "g{index}: {} anchors / {} operations compared",
                        layer.temporary_anchors.len(),
                        layer.operations.len()
                    ));
                }
            }
        }
    }
    let mut unrecorded_quiescent = Vec::new();
    for (index, layer) in &constructed {
        if recorded.contains_key(index) {
            continue;
        }
        if !layer.temporary_anchors.is_empty() || !layer.operations.is_empty() {
            group.mismatches.push(format!(
                "the construction planned unrecorded layer g{index} ({} temporary anchors, {} operations); the reference records no planning layer there",
                layer.temporary_anchors.len(),
                layer.operations.len()
            ));
        } else {
            unrecorded_quiescent.push(format!("g{index}"));
        }
    }
    if !unrecorded_quiescent.is_empty() {
        group.detail.push(format!(
            "unrecorded construction layers with no planning activity: {}",
            unrecorded_quiescent.join(", ")
        ));
    }
    group.finish()
}

fn compare_domain_group(
    name: &str,
    construction: &GateAConstruction,
    reference: &GateReference,
) -> GroupOutcome {
    let mut group = GroupOutcome::pass(name);
    let recorded = recorded_layers(reference);
    let constructed = constructed_layers(construction);
    if recorded.is_empty() {
        group
            .unavailable
            .push("the reference records no planning layer".to_string());
        return group.finish();
    }
    for (index, recorded_layer) in &recorded {
        let Some(derived_layer) = constructed.get(index) else {
            group.unavailable.push(format!(
                "planning layer g{index} is recorded but absent from the construction, so its domains are not comparable"
            ));
            continue;
        };
        let converted = convert_reference_layer(recorded_layer);
        compare_domain_set(
            &mut group,
            &format!("g{index}.internal"),
            &derived_layer.internal_domains,
            &converted.internal_domains,
        );
        compare_domain_set(
            &mut group,
            &format!("g{index}.void"),
            &derived_layer.void_domains,
            &converted.void_domains,
        );
        compare_domain_set(
            &mut group,
            &format!("g{index}.temporary_anchor"),
            &derived_layer.temporary_anchors,
            &converted.temporary_anchors,
        );
    }
    group.finish()
}

fn compare_domain_set(
    group: &mut GroupOutcome,
    label: &str,
    derived: &[ExPolygon],
    recorded: &[ExPolygon],
) {
    let (matched, unmatched_recorded, unmatched_derived) =
        match_polygon_multiset(derived, recorded, GEOMETRY_TOLERANCE_PNP_UNITS);
    if unmatched_recorded.is_empty() && unmatched_derived.is_empty() {
        group
            .detail
            .push(format!("{label}: {matched}/{} polygons matched", recorded.len()));
        return;
    }
    group.mismatches.push(format!(
        "{label}: {} derived / {} recorded polygons, {matched} matched; unmatched recorded {:?}, unmatched derived {:?}",
        derived.len(),
        recorded.len(),
        unmatched_recorded
            .iter()
            .map(|index| format!("#{index}:{:.6}mm2", polygon_area_mm2(&recorded[*index])))
            .collect::<Vec<_>>(),
        unmatched_derived
            .iter()
            .map(|index| format!("#{index}:{:.6}mm2", polygon_area_mm2(&derived[*index])))
            .collect::<Vec<_>>()
    ));
}

fn compare_source_regions(construction: &GateAConstruction, reference: &GateReference) -> GroupOutcome {
    let mut group = GroupOutcome::pass("planning_layers.source_regions");
    let recorded = recorded_layers(reference);
    let constructed = constructed_layers(construction);
    for (index, recorded_layer) in &recorded {
        let Some(derived_layer) = constructed.get(index) else {
            continue;
        };
        if derived_layer.source_regions != recorded_layer.source_regions {
            group.mismatches.push(format!(
                "g{index} source_regions: recorded {:?}, derived {:?}",
                recorded_layer.source_regions, derived_layer.source_regions
            ));
        }
    }
    group.finish()
}

fn compare_source_catalog(construction: &GateAConstruction, reference: &GateReference) -> GroupOutcome {
    let mut group = GroupOutcome::pass("planning_layers.source_catalog");
    let recorded = recorded_layers(reference);
    let constructed = constructed_layers(construction);
    for (index, recorded_layer) in &recorded {
        let Some(derived_layer) = constructed.get(index) else {
            continue;
        };
        let recorded_catalog = convert_source_catalog(&recorded_layer.source_catalog);
        let derived_by_token = index_surfaces(&derived_layer.source_catalog);
        let recorded_by_token = index_surfaces(&recorded_catalog);
        for token in recorded_by_token.keys() {
            if !derived_by_token.contains_key(token) {
                group.mismatches.push(format!(
                    "g{index} source_catalog is missing recorded surface `{token}`"
                ));
            }
        }
        for token in derived_by_token.keys() {
            if !recorded_by_token.contains_key(token) {
                group.mismatches.push(format!(
                    "g{index} source_catalog carries unrecorded surface `{token}`"
                ));
            }
        }
        for (token, recorded_surface) in &recorded_by_token {
            let Some(derived_surface) = derived_by_token.get(token) else {
                continue;
            };
            compare_surface(
                &mut group.mismatches,
                &format!("g{index}.catalog[{token}]"),
                derived_surface,
                recorded_surface,
            );
        }
    }
    group.finish()
}

fn compare_surface(
    mismatches: &mut Vec<String>,
    label: &str,
    derived: &SourceSurface,
    recorded: &SourceSurface,
) {
    if derived.surface_kind != recorded.surface_kind {
        mismatches.push(format!(
            "{label} surface_kind: recorded {}, derived {}",
            recorded.surface_kind, derived.surface_kind
        ));
    }
    compare_option_angle(
        mismatches,
        &format!("{label} solid_angle_rad"),
        derived.solid_angle_rad,
        recorded.solid_angle_rad,
    );
    compare_option_angle(
        mismatches,
        &format!("{label} bridge_angle_rad"),
        derived.bridge_angle_rad,
        recorded.bridge_angle_rad,
    );
    let geometry_derived = vec![derived.geometry.clone()];
    let geometry_recorded = vec![recorded.geometry.clone()];
    let (matched, unmatched_recorded, unmatched_derived) =
        match_polygon_multiset(&geometry_derived, &geometry_recorded, GEOMETRY_TOLERANCE_PNP_UNITS);
    if matched != 1 || !unmatched_recorded.is_empty() || !unmatched_derived.is_empty() {
        mismatches.push(format!("{label} geometry does not match the recorded surface"));
    }
}

fn compare_option_angle(mismatches: &mut Vec<String>, label: &str, derived: Option<f64>, recorded: Option<f64>) {
    match (derived, recorded) {
        (None, None) => {}
        (Some(a), Some(b)) if (a - b).abs() <= ANGLE_TOLERANCE_RAD => {}
        _ => mismatches.push(format!(
            "{label}: recorded {recorded:?}, derived {derived:?} (tol {ANGLE_TOLERANCE_RAD})"
        )),
    }
}

fn compare_boundary_provenance_group(
    construction: &GateAConstruction,
    reference: &GateReference,
) -> GroupOutcome {
    let mut group = GroupOutcome::pass("planning_layers.boundary_provenance");
    let recorded = recorded_layers(reference);
    let constructed = constructed_layers(construction);
    for (index, recorded_layer) in &recorded {
        let Some(derived_layer) = constructed.get(index) else {
            group.unavailable.push(format!(
                "planning layer g{index} is recorded but absent from the construction, so its boundary provenance is not comparable"
            ));
            continue;
        };
        let converted = convert_reference_layer(recorded_layer);
        compare_boundary_layers(&mut group, *index, derived_layer, &converted);
    }
    group.finish()
}

fn compare_boundary_layers(
    group: &mut GroupOutcome,
    layer_index: u32,
    derived: &PlanningLayer,
    recorded: &PlanningLayer,
) {
    // Output `surface_id`/`ring_id`/`segment_id` are locators into the produced
    // polygon list, and an independent reconstruction may order equivalent
    // polygons differently (the domain comparison is a multiset for the same
    // reason). Match edges by domain + geometry within the transport tolerance,
    // then compare the semantic content: the exact multiset of source
    // ring/segment identities, overlap intervals, and generated operation
    // lineage. The comparison is two-way: a recorded edge without a derived
    // counterpart and a derived edge without a recorded counterpart are both
    // named mismatches, never a silent count.
    let mut used = vec![false; derived.boundary_provenance.len()];
    let mut claimed = vec![false; derived.boundary_provenance.len()];
    let mut matched = 0usize;
    let mut unmatched_recorded = Vec::new();
    let mut contributor_differences = Vec::new();
    for recorded_entry in &recorded.boundary_provenance {
        let recorded_contributors = contributor_multiset(&recorded_entry.contributors);
        let candidates: Vec<usize> = derived
            .boundary_provenance
            .iter()
            .enumerate()
            .filter(|(index, derived_entry)| {
                !used[*index]
                    && !claimed[*index]
                    && derived_entry.address.domain == recorded_entry.address.domain
                    && point_close(derived_entry.endpoints[0], recorded_entry.endpoints[0])
                    && point_close(derived_entry.endpoints[1], recorded_entry.endpoints[1])
            })
            .map(|(index, _)| index)
            .collect();
        let exact_match = candidates.iter().copied().find(|index| {
            contributor_multiset(&derived.boundary_provenance[*index].contributors)
                == recorded_contributors
        });
        match (exact_match, candidates.first().copied()) {
            (Some(index), _) => {
                used[index] = true;
                matched += 1;
            }
            (None, Some(index)) => {
                // Same domain and endpoints, different contributor content: the
                // edge is claimed (so it is not also reported as unrecorded),
                // and the contributor difference is the named mismatch.
                claimed[index] = true;
                let derived_entry = &derived.boundary_provenance[index];
                contributor_differences.push(format!(
                    "g{layer_index} {:?} edge {:?}->{:?} contributors: recorded {:?}, derived {:?}",
                    recorded_entry.address.domain,
                    recorded_entry.endpoints[0],
                    recorded_entry.endpoints[1],
                    contributor_summary(&recorded_entry.contributors),
                    contributor_summary(&derived_entry.contributors)
                ));
            }
            (None, None) => unmatched_recorded.push(format!(
                "g{layer_index} {:?} edge {:?}->{:?} ({} recorded contributors)",
                recorded_entry.address.domain,
                recorded_entry.endpoints[0],
                recorded_entry.endpoints[1],
                recorded_entry.contributors.len()
            )),
        }
    }
    let unmatched_derived: Vec<usize> = used
        .iter()
        .zip(claimed.iter())
        .enumerate()
        .filter_map(|(index, (was_used, was_claimed))| (!was_used && !was_claimed).then_some(index))
        .collect();
    for difference in contributor_differences.iter().take(3) {
        group.mismatches.push(difference.clone());
    }
    for missing in unmatched_recorded.iter().take(3) {
        group.mismatches.push(format!(
            "recorded boundary edge has no derived counterpart: {missing}"
        ));
    }
    for extra in unmatched_derived.iter().take(3) {
        let derived_entry = &derived.boundary_provenance[*extra];
        group.mismatches.push(format!(
            "derived boundary edge has no recorded counterpart: g{layer_index} {:?} edge {:?}->{:?} ({} derived contributors: {:?})",
            derived_entry.address.domain,
            derived_entry.endpoints[0],
            derived_entry.endpoints[1],
            derived_entry.contributors.len(),
            contributor_summary(&derived_entry.contributors)
        ));
    }
    if contributor_differences.len() > 3
        || unmatched_recorded.len() > 3
        || unmatched_derived.len() > 3
    {
        group.mismatches.push(format!(
            "g{layer_index} boundary comparison truncated: {} contributor differences, {} unmatched recorded edges, {} unmatched derived edges",
            contributor_differences.len(),
            unmatched_recorded.len(),
            unmatched_derived.len()
        ));
    }
    group.detail.push(format!(
        "g{layer_index}: {matched}/{} recorded boundary edges matched ({} derived edges unmatched)",
        recorded.boundary_provenance.len(),
        unmatched_derived.len()
    ));
}

fn boundary_by_address(
    provenance: &[BoundaryProvenance],
) -> BTreeMap<(u8, u32, u32, u32), Vec<&BoundaryProvenance>> {
    let mut map: BTreeMap<(u8, u32, u32, u32), Vec<&BoundaryProvenance>> = BTreeMap::new();
    for entry in provenance {
        map.entry(address_key(&entry.address)).or_default().push(entry);
    }
    map
}

fn address_key(address: &BoundaryAddress) -> (u8, u32, u32, u32) {
    (
        domain_ordinal(&address.domain),
        address.surface_id,
        address.ring_id,
        address.segment_id,
    )
}

fn domain_ordinal(domain: &DomainKind) -> u8 {
    match domain {
        DomainKind::Internal => 0,
        DomainKind::Void => 1,
        DomainKind::TemporaryAnchor => 2,
    }
}

fn contributor_multiset(contributors: &[BoundaryContribution]) -> BTreeMap<String, usize> {
    let mut multiset = BTreeMap::new();
    for contributor in contributors {
        let value = serde_json::to_value(contributor).expect("contributor serializes");
        *multiset.entry(canonical_json(&value)).or_insert(0) += 1;
    }
    multiset
}

fn contributor_summary(contributors: &[BoundaryContribution]) -> Vec<String> {
    contributors
        .iter()
        .map(|contributor| {
            let source = &contributor.source.surface;
            match &contributor.relation {
                ContributionRelation::Overlap { start, end } => format!(
                    "overlap({} ring {:?} seg {:?} {}/{}-{}/{})",
                    source.surface_token,
                    contributor.source.ring_id,
                    contributor.source.segment_id,
                    start.numerator,
                    start.denominator,
                    end.numerator,
                    end.denominator
                ),
                ContributionRelation::Generated { operation_id } => {
                    format!("generated({} op {operation_id})", source.surface_token)
                }
            }
        })
        .collect()
}

fn compare_operations(construction: &GateAConstruction, reference: &GateReference) -> GroupOutcome {
    let mut group = GroupOutcome::pass("planning_layers.operations");
    let recorded = recorded_layers(reference);
    let constructed = constructed_layers(construction);
    for (index, recorded_layer) in &recorded {
        let Some(derived_layer) = constructed.get(index) else {
            group.unavailable.push(format!(
                "planning layer g{index} is recorded but absent from the construction, so its operations are not comparable"
            ));
            continue;
        };
        compare_operation_layers(&mut group, *index, derived_layer, recorded_layer);
    }
    group.finish()
}

fn compare_operation_layers(
    group: &mut GroupOutcome,
    layer_index: u32,
    derived: &PlanningLayer,
    recorded: &PlanningLayer,
) {
    let derived_by_id = operation_by_id(&derived.operations);
    let recorded_by_id = operation_by_id(&recorded.operations);
    for (id, recorded_op) in &recorded_by_id {
        let Some(derived_op) = derived_by_id.get(id) else {
            group.mismatches.push(format!(
                "g{layer_index} recorded operation {id} ({:?}) is absent from the construction",
                recorded_op.kind
            ));
            continue;
        };
        compare_operation(group, layer_index, derived_op, recorded_op);
    }
    for (id, derived_op) in &derived_by_id {
        if !recorded_by_id.contains_key(id) {
            group.mismatches.push(format!(
                "g{layer_index} derived operation {id} ({:?}) has no recorded counterpart",
                derived_op.kind
            ));
        }
    }
    group.detail.push(format!(
        "g{layer_index}: {} operations compared ({} recorded)",
        recorded_by_id.len(),
        recorded.operations.len()
    ));
}

fn operation_by_id(operations: &[GeometryOperation]) -> BTreeMap<u32, &GeometryOperation> {
    let mut map = BTreeMap::new();
    for operation in operations {
        map.insert(operation.operation_id, operation);
    }
    map
}

fn compare_operation(
    group: &mut GroupOutcome,
    layer_index: u32,
    derived: &GeometryOperation,
    recorded: &GeometryOperation,
) {
    let id = recorded.operation_id;
    let label = format!("g{layer_index} operation {id}");
    if derived.kind != recorded.kind {
        group.mismatches.push(format!(
            "{label} kind: recorded {:?}, derived {:?}",
            recorded.kind, derived.kind
        ));
    }
    if derived.checkpoint != recorded.checkpoint {
        group.mismatches.push(format!(
            "{label} checkpoint: recorded {:?}, derived {:?}",
            recorded.checkpoint, derived.checkpoint
        ));
    }
    if derived.inputs != recorded.inputs {
        group.mismatches.push(format!("{label} inputs differ from the recorded source segments"));
    }
    if derived.parent_operations != recorded.parent_operations {
        group.mismatches.push(format!(
            "{label} parent_operations: recorded {:?}, derived {:?}",
            recorded.parent_operations, derived.parent_operations
        ));
    }
    if derived.operands != recorded.operands {
        group.mismatches.push(format!("{label} operands differ from the recorded lineage"));
    }
    if derived.join != recorded.join {
        group.mismatches.push(format!(
            "{label} join: recorded {:?}, derived {:?}",
            recorded.join, derived.join
        ));
    }
    let recorded_offset = recorded.offset_units.map(canonical_to_pnp);
    if derived.offset_units != recorded_offset {
        group.mismatches.push(format!(
            "{label} offset_units: recorded {:?} (canonical), derived {:?} (PnP)",
            recorded.offset_units, derived.offset_units
        ));
    }
    let recorded_outputs: BTreeSet<(u8, u32, u32, u32)> =
        recorded.outputs.iter().map(address_key).collect();
    let derived_outputs: BTreeSet<(u8, u32, u32, u32)> = derived.outputs.iter().map(address_key).collect();
    if derived_outputs != recorded_outputs {
        group.mismatches.push(format!(
            "{label} outputs: recorded {} addresses, derived {} addresses",
            recorded_outputs.len(),
            derived_outputs.len()
        ));
    }
    let recorded_geometry = vec_of_converted_polygons(&recorded.result_geometry);
    let (matched, unmatched_recorded, unmatched_derived) = match_polygon_multiset(
        &derived.result_geometry,
        &recorded_geometry,
        GEOMETRY_TOLERANCE_PNP_UNITS,
    );
    if !unmatched_recorded.is_empty() || !unmatched_derived.is_empty() {
        group.mismatches.push(format!(
            "{label} result_geometry: {matched} of {} recorded polygons matched",
            recorded.result_geometry.len()
        ));
    }
}

fn compare_parameters(construction: &GateAConstruction, reference: &GateReference) -> GroupOutcome {
    let mut group = GroupOutcome::pass("parameters");
    let derived = &construction.parameters;
    let recorded = &reference.parameters;
    compare_scalar(
        &mut group,
        "nozzle_diameter_mm",
        derived.nozzle_diameter_mm,
        recorded.nozzle_diameter_mm,
        SCALAR_TOLERANCE_MM,
    );
    compare_scalar(
        &mut group,
        "extrusion_width_mm",
        derived.extrusion_width_mm,
        recorded.extrusion_width_mm,
        SCALAR_TOLERANCE_MM,
    );
    compare_scalar(
        &mut group,
        "density_fraction",
        derived.density_fraction,
        recorded.density_fraction,
        SCALAR_TOLERANCE_MM,
    );
    if derived.multiline != recorded.multiline {
        group.mismatches.push(format!(
            "multiline: recorded {}, derived {}",
            recorded.multiline, derived.multiline
        ));
    }
    compare_scalar(
        &mut group,
        "layer_height_mm",
        derived.layer_height_mm,
        recorded.layer_height_mm,
        SCALAR_TOLERANCE_MM,
    );
    compare_scalar(
        &mut group,
        "overhang_angle_deg",
        derived.overhang_angle_deg,
        recorded.overhang_angle_deg,
        SCALAR_TOLERANCE_MM,
    );
    compare_scalar(
        &mut group,
        "prune_angle_deg",
        derived.prune_angle_deg,
        recorded.prune_angle_deg,
        SCALAR_TOLERANCE_MM,
    );
    compare_scalar(
        &mut group,
        "straightening_angle_deg",
        derived.straightening_angle_deg,
        recorded.straightening_angle_deg,
        SCALAR_TOLERANCE_MM,
    );
    match (derived.supporting_radius_mm, recorded.supporting_radius_mm) {
        (Some(a), Some(b)) if (a - b).abs() <= SCALAR_TOLERANCE_MM => {}
        (None, None) => {}
        _ => group.mismatches.push(format!(
            "supporting_radius_mm: recorded {:?}, derived {:?} (tol {SCALAR_TOLERANCE_MM})",
            recorded.supporting_radius_mm, derived.supporting_radius_mm
        )),
    }
    compare_scalar(
        &mut group,
        "wall_supporting_radius_mm",
        derived.wall_supporting_radius_mm,
        recorded.wall_supporting_radius_mm,
        SCALAR_TOLERANCE_MM,
    );
    // Canonical integer `coord_t` nanometres vs the derived exact real value:
    // one canonical coord_t = 1 nm = 1e-6 mm.
    compare_scalar(
        &mut group,
        "prune_length_mm",
        derived.prune_length_mm,
        recorded.prune_length_mm,
        COORD_T_TOLERANCE_MM,
    );
    compare_scalar(
        &mut group,
        "straightening_max_distance_mm",
        derived.straightening_max_distance_mm,
        recorded.straightening_max_distance_mm,
        COORD_T_TOLERANCE_MM,
    );
    group.finish()
}

fn compare_scalar(group: &mut GroupOutcome, field: &str, derived: f64, recorded: f64, tolerance: f64) {
    if !(derived - recorded).abs().is_finite() || (derived - recorded).abs() > tolerance {
        group.mismatches.push(format!(
            "{field}: recorded {recorded}, derived {derived} (tol {tolerance})"
        ));
    }
}

fn compare_scalar_source(construction: &GateAConstruction, reference: &GateReference) -> GroupOutcome {
    let mut group = GroupOutcome::pass("scalar_source_region");
    if construction.scalar_source_region != reference.scalar_source_region {
        group.mismatches.push(format!(
            "scalar_source_region: recorded {:?}, derived {:?}",
            reference.scalar_source_region, construction.scalar_source_region
        ));
    }
    group.finish()
}

fn compare_policies(
    construction: &GateAConstruction,
    reference: &GateReference,
    manifest_case: &ManifestCase,
) -> GroupOutcome {
    let mut group = GroupOutcome::pass("policies");
    let Some(recorded_policies) = &reference.policies else {
        let reason = manifest_case
            .absent
            .get("reference.policies")
            .cloned()
            .unwrap_or_else(|| {
                "the reference records no `policies` array and the manifest names no absent reason"
                    .to_string()
            });
        group.unavailable.push(format!(
            "reference.policies is null: {reason}; `FillPolicyInput.bridge_angle_rad`/`is_bridge`/`fixed_angle` are therefore not comparable from recorded evidence"
        ));
        return group.finish();
    };
    let derived_by_key = policy_by_key(&construction.policies);
    let recorded_by_key = policy_by_key(recorded_policies);
    for (key, recorded_policy) in &recorded_by_key {
        let Some(derived_policy) = derived_by_key.get(key) else {
            group.mismatches.push(format!("recorded policy {key} has no derived occurrence"));
            continue;
        };
        compare_fill_policy(&mut group, key, &derived_policy.policy, &recorded_policy.policy);
    }
    for key in derived_by_key.keys() {
        if !recorded_by_key.contains_key(key) {
            group
                .mismatches
                .push(format!("derived policy {key} has no recorded occurrence"));
        }
    }
    group.finish()
}

fn policy_by_key(policies: &[DerivedRegionPolicy]) -> BTreeMap<String, &DerivedRegionPolicy> {
    let mut map = BTreeMap::new();
    for policy in policies {
        let identity = canonical_json(&serde_json::to_value(&policy.identity).expect("identity serializes"));
        map.insert(format!("{}@g{}", identity, policy.global_layer_index), policy);
    }
    map
}

fn compare_fill_policy(
    group: &mut GroupOutcome,
    key: &str,
    derived: &FillPolicyInput,
    recorded: &FillPolicyInput,
) {
    compare_scalar(
        group,
        &format!("{key}.infill_direction_deg"),
        derived.infill_direction_deg,
        recorded.infill_direction_deg,
        SCALAR_TOLERANCE_MM,
    );
    if derived.sparse_rotate_template != recorded.sparse_rotate_template {
        group.mismatches.push(format!(
            "{key}.sparse_rotate_template: recorded {:?}, derived {:?}",
            recorded.sparse_rotate_template, derived.sparse_rotate_template
        ));
    }
    compare_scalar(
        group,
        &format!("{key}.solid_infill_direction_deg"),
        derived.solid_infill_direction_deg,
        recorded.solid_infill_direction_deg,
        SCALAR_TOLERANCE_MM,
    );
    if derived.solid_rotate_template != recorded.solid_rotate_template {
        group.mismatches.push(format!(
            "{key}.solid_rotate_template: recorded {:?}, derived {:?}",
            recorded.solid_rotate_template, derived.solid_rotate_template
        ));
    }
    compare_scalar(
        group,
        &format!("{key}.top_layer_direction_deg"),
        derived.top_layer_direction_deg,
        recorded.top_layer_direction_deg,
        SCALAR_TOLERANCE_MM,
    );
    compare_scalar(
        group,
        &format!("{key}.bottom_layer_direction_deg"),
        derived.bottom_layer_direction_deg,
        recorded.bottom_layer_direction_deg,
        SCALAR_TOLERANCE_MM,
    );
    if derived.align_to_model != recorded.align_to_model {
        group.mismatches.push(format!(
            "{key}.align_to_model: recorded {}, derived {}",
            recorded.align_to_model, derived.align_to_model
        ));
    }
    compare_scalar(
        group,
        &format!("{key}.model_alignment_offset_rad"),
        derived.model_alignment_offset_rad,
        recorded.model_alignment_offset_rad,
        ANGLE_TOLERANCE_RAD,
    );
    compare_scalar(
        group,
        &format!("{key}.resolved_sparse_angle_rad"),
        derived.resolved_sparse_angle_rad,
        recorded.resolved_sparse_angle_rad,
        ANGLE_TOLERANCE_RAD,
    );
    compare_scalar(
        group,
        &format!("{key}.resolved_solid_angle_rad"),
        derived.resolved_solid_angle_rad,
        recorded.resolved_solid_angle_rad,
        ANGLE_TOLERANCE_RAD,
    );
    compare_option_angle(
        &mut group.mismatches,
        &format!("{key}.resolved_top_angle_rad"),
        derived.resolved_top_angle_rad,
        recorded.resolved_top_angle_rad,
    );
    compare_option_angle(
        &mut group.mismatches,
        &format!("{key}.resolved_bottom_angle_rad"),
        derived.resolved_bottom_angle_rad,
        recorded.resolved_bottom_angle_rad,
    );
    if derived.fixed_angle != recorded.fixed_angle {
        group.mismatches.push(format!(
            "{key}.fixed_angle: recorded {}, derived {}",
            recorded.fixed_angle, derived.fixed_angle
        ));
    }
    if derived.is_bridge != recorded.is_bridge {
        group.mismatches.push(format!(
            "{key}.is_bridge: recorded {}, derived {}",
            recorded.is_bridge, derived.is_bridge
        ));
    }
    compare_option_angle(
        &mut group.mismatches,
        &format!("{key}.bridge_angle_rad"),
        derived.bridge_angle_rad,
        recorded.bridge_angle_rad,
    );
}

// -----------------------------------------------------------------------------
// Geometry normalization (canonical nm -> PnP IR units, holes preserved)
// -----------------------------------------------------------------------------

pub fn canonical_to_pnp(value: i64) -> i64 {
    if value >= 0 {
        (value + PNP_UNITS_PER_CANONICAL_UNIT / 2) / PNP_UNITS_PER_CANONICAL_UNIT
    } else {
        -((-value + PNP_UNITS_PER_CANONICAL_UNIT / 2) / PNP_UNITS_PER_CANONICAL_UNIT)
    }
}

fn convert_point(point: Point2) -> Point2 {
    Point2 {
        x: canonical_to_pnp(point.x),
        y: canonical_to_pnp(point.y),
    }
}

fn convert_expolygon(polygon: &ExPolygon) -> ExPolygon {
    ExPolygon {
        contour: slicer_ir::Polygon {
            points: polygon.contour.points.iter().copied().map(convert_point).collect(),
        },
        holes: polygon
            .holes
            .iter()
            .map(|hole| slicer_ir::Polygon {
                points: hole.points.iter().copied().map(convert_point).collect(),
            })
            .collect(),
    }
}

fn vec_of_converted_polygons(polygons: &[ExPolygon]) -> Vec<ExPolygon> {
    polygons.iter().map(convert_expolygon).collect()
}

/// Convert a recorded canonical-unit layer into PnP IR units without mutating
/// the recorded reference (this is a clone used only for comparison).
pub fn convert_reference_layer(layer: &PlanningLayer) -> PlanningLayer {
    PlanningLayer {
        object_id: layer.object_id.clone(),
        local_layer_index: layer.local_layer_index,
        global_layer_index: layer.global_layer_index,
        z_mm: layer.z_mm,
        internal_domains: vec_of_converted_polygons(&layer.internal_domains),
        void_domains: vec_of_converted_polygons(&layer.void_domains),
        temporary_anchors: vec_of_converted_polygons(&layer.temporary_anchors),
        source_regions: layer.source_regions.clone(),
        source_catalog: convert_source_catalog(&layer.source_catalog),
        boundary_provenance: layer
            .boundary_provenance
            .iter()
            .map(|entry| BoundaryProvenance {
                address: entry.address.clone(),
                endpoints: [convert_point(entry.endpoints[0]), convert_point(entry.endpoints[1])],
                contributors: entry.contributors.clone(),
            })
            .collect(),
        operations: layer
            .operations
            .iter()
            .map(|operation| GeometryOperation {
                operation_id: operation.operation_id,
                kind: operation.kind,
                checkpoint: operation.checkpoint,
                inputs: operation.inputs.clone(),
                parent_operations: operation.parent_operations.clone(),
                operands: operation.operands.clone(),
                offset_units: operation.offset_units.map(canonical_to_pnp),
                join: operation.join.clone(),
                result_geometry: vec_of_converted_polygons(&operation.result_geometry),
                outputs: operation.outputs.clone(),
            })
            .collect(),
    }
}

pub fn convert_source_catalog(catalog: &[SourceSurface]) -> Vec<SourceSurface> {
    catalog
        .iter()
        .map(|surface| SourceSurface {
            id: surface.id.clone(),
            geometry: convert_expolygon(&surface.geometry),
            surface_kind: surface.surface_kind.clone(),
            solid_angle_rad: surface.solid_angle_rad,
            bridge_angle_rad: surface.bridge_angle_rad,
        })
        .collect()
}

fn index_surfaces(catalog: &[SourceSurface]) -> BTreeMap<String, &SourceSurface> {
    let mut map = BTreeMap::new();
    for surface in catalog {
        map.insert(surface.id.surface_token.clone(), surface);
    }
    map
}

fn point_close(a: Point2, b: Point2) -> bool {
    (a.x - b.x).abs() <= GEOMETRY_TOLERANCE_PNP_UNITS
        && (a.y - b.y).abs() <= GEOMETRY_TOLERANCE_PNP_UNITS
}

fn ring_matches(derived: &[Point2], recorded: &[Point2], tolerance: i64) -> bool {
    if derived.len() != recorded.len() {
        return false;
    }
    let count = derived.len();
    if count == 0 {
        return true;
    }
    (0..count).any(|shift| {
        (0..count).all(|index| {
            let a = derived[index];
            let b = recorded[(index + shift) % count];
            (a.x - b.x).abs() <= tolerance && (a.y - b.y).abs() <= tolerance
        })
    })
}

fn ex_polygon_matches(derived: &ExPolygon, recorded: &ExPolygon, tolerance: i64) -> bool {
    if !ring_matches(&derived.contour.points, &recorded.contour.points, tolerance) {
        return false;
    }
    if derived.holes.len() != recorded.holes.len() {
        return false;
    }
    let mut used = vec![false; recorded.holes.len()];
    for hole in &derived.holes {
        let Some(position) = recorded.holes.iter().enumerate().position(|(index, candidate)| {
            !used[index] && ring_matches(&hole.points, &candidate.points, tolerance)
        }) else {
            return false;
        };
        used[position] = true;
    }
    true
}

/// Multiset polygon matching with rotation-tolerant rings and preserved holes.
/// Returns matched count plus unmatched indices for each side.
pub fn match_polygon_multiset(
    derived: &[ExPolygon],
    recorded: &[ExPolygon],
    tolerance: i64,
) -> (usize, Vec<usize>, Vec<usize>) {
    let mut recorded_order: Vec<usize> = (0..recorded.len()).collect();
    recorded_order.sort_by(|a, b| {
        polygon_area_units(&recorded[*b])
            .partial_cmp(&polygon_area_units(&recorded[*a]))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut used = vec![false; derived.len()];
    let mut matched = 0usize;
    let mut unmatched_recorded = Vec::new();
    for recorded_index in recorded_order {
        let found = derived.iter().enumerate().position(|(index, candidate)| {
            !used[index] && ex_polygon_matches(candidate, &recorded[recorded_index], tolerance)
        });
        match found {
            Some(index) => {
                used[index] = true;
                matched += 1;
            }
            None => unmatched_recorded.push(recorded_index),
        }
    }
    let unmatched_derived = used
        .iter()
        .enumerate()
        .filter_map(|(index, was_used)| (!was_used).then_some(index))
        .collect();
    (matched, unmatched_recorded, unmatched_derived)
}

pub fn polygon_area_units(polygon: &ExPolygon) -> f64 {
    ring_area_units(&polygon.contour.points)
        - polygon
            .holes
            .iter()
            .map(|hole| ring_area_units(&hole.points).abs())
            .sum::<f64>()
}

pub fn polygon_area_mm2(polygon: &ExPolygon) -> f64 {
    polygon_area_units(polygon) / (10_000.0 * 10_000.0)
}

fn ring_area_units(points: &[Point2]) -> f64 {
    if points.len() < 3 {
        return 0.0;
    }
    let mut sum = 0.0f64;
    for index in 0..points.len() {
        let a = points[index];
        let b = points[(index + 1) % points.len()];
        sum += (a.x as f64) * (b.y as f64) - (b.x as f64) * (a.y as f64);
    }
    sum / 2.0
}

// -----------------------------------------------------------------------------
// Outcome writer (Step 6 runs this through the outcome test)
// -----------------------------------------------------------------------------

pub const OUTCOME_SCHEMA_VERSION: u64 = 1;
pub const GATE_B_BLOCKER: &str = "MIGRATION_BLOCKED_GATE_B";
pub const FRAMEWORK_BLOCKER: &str = "MIGRATION_BLOCKED_FRAMEWORK";
pub const GATE_A_BLOCKER: &str = "MIGRATION_BLOCKED_GATE_A";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandRecord {
    /// The argv actually spawned: `<test-binary> <filter> --exact --nocapture`.
    /// Cargo is never invoked by the outcome test, so this is never a Cargo line.
    pub command: String,
    /// The Cargo invocation a developer would run to reproduce `command`. It is
    /// recorded as documentation only; nothing in this test executes it.
    pub equivalent: String,
    /// `None` only for the still-running invocation, which names why in
    /// `exit_reason`.
    pub exit: Option<i32>,
    pub exit_reason: Option<String>,
    pub observed_by: String,
}

fn command_to_value(command: &CommandRecord) -> Value {
    json!({
        "command": command.command,
        "equivalent": command.equivalent,
        "exit": command.exit,
        "exit_reason": command.exit_reason,
        "observed_by": command.observed_by,
    })
}

/// `executed` is derived from exits that were actually observed, never
/// asserted: every child-observed sibling command must have exited zero, and
/// the one invocation that cannot observe itself must carry a non-empty reason
/// instead of an exit. A record with no observed child execution proves nothing,
/// so it is not `executed`.
pub fn commands_executed(commands: &[CommandRecord]) -> bool {
    let mut observed_children = 0usize;
    for command in commands {
        if command.observed_by.contains("child") {
            observed_children += 1;
            if command.exit != Some(0) {
                return false;
            }
        } else {
            let documented_self = command.exit.is_none()
                && command
                    .exit_reason
                    .as_ref()
                    .is_some_and(|reason| !reason.trim().is_empty());
            if !documented_self {
                return false;
            }
        }
    }
    observed_children > 0
}

/// Build the outcome record from the independently recomputed decisions and the
/// observed command records. `executed` is derived from the observed exits and
/// `migration_eligible` from the named blocker list: A alone never permits
/// migration, and a non-pass A names its own blocker, so eligibility is only
/// true when no blocker is named.
pub fn build_outcome(
    fixtures: &Fixtures,
    decisions: &[CaseDecision],
    hashes: &BTreeMap<String, CaseEvidenceHashes>,
    commands: Vec<CommandRecord>,
) -> Result<Value, String> {
    let dir = fixture_dir();
    let manifest_path = dir.join(MANIFEST_FILE);
    let mut cases = Vec::new();
    for decision in decisions {
        let case_hashes = hashes
            .get(&decision.case_id)
            .ok_or_else(|| format!("no evidence hashes for case `{}`", decision.case_id))?;
        cases.push(json!({
            "case_id": decision.case_id,
            "status": status_label(decision.status),
            "mismatches": decision.mismatches,
            "unavailable": decision.unavailable,
            "a_source_sha256": case_hashes.a_source_sha256,
            "a_raw_config_sha256": case_hashes.a_raw_config_sha256,
            "a_earlier_provenance_sha256": case_hashes.a_earlier_provenance_sha256,
            "reference_sha256": case_hashes.reference_sha256,
        }));
    }
    let mut blockers = vec![
        json!({
            "code": GATE_B_BLOCKER,
            "detail": "Gate B (the same owned kernel through native and WASM adapters, packet layer-module-preparation_09_lightning-kernel-portability) has not executed; Gate A alone never permits migration (ADR-0066).",
        }),
        json!({
            "code": FRAMEWORK_BLOCKER,
            "detail": "ADR-0066: no layer-preparation migration before the canonical strategy and portability proof complete.",
        }),
    ];
    let not_passed: Vec<String> = decisions
        .iter()
        .filter(|case| case.status != CaseStatus::Pass)
        .map(|case| format!("{}={}", case.case_id, status_label(case.status)))
        .collect();
    if !not_passed.is_empty() {
        blockers.push(json!({
            "code": GATE_A_BLOCKER,
            "detail": format!(
                "Gate A did not pass for: {}. Recorded statuses and named mismatches/unavailable populations are in `cases`; no approximation or defaulted parameter repairs a case.",
                not_passed.join(", ")
            ),
        }));
    }
    let executed = commands_executed(&commands);
    let migration_eligible = blockers.is_empty();
    Ok(json!({
        "schema_version": OUTCOME_SCHEMA_VERSION,
        "gate": "A",
        "packet": "layer-module-preparation_07_lightning-geometry-portability-gate",
        "task_id": "TASK-576",
        "executed": executed,
        "generator": "lightning_oracle_tdd::outcome_records_gate_a",
        "fixtures": {
            "inputs_sha256": sha256_file(&dir.join(INPUTS_FILE))?,
            "canonical_sha256": sha256_file(&dir.join(CANONICAL_FILE))?,
            "manifest_sha256": sha256_file(&manifest_path)?,
            "manifest_recorded_sha256": fixtures.manifest.capture.manifest_sha256,
        },
        "capture": {
            "canonical_revision": fixtures.manifest.capture.canonical_revision,
            "canonical_build_revision": fixtures.manifest.capture.canonical_build_revision,
            "canonical_build_dirty": fixtures.manifest.capture.canonical_build_dirty,
            "recorder_patch_sha256": fixtures.manifest.capture.recorder_patch_sha256,
            "recorder_patch_applied": fixtures.manifest.capture.recorder_patch_applied,
        },
        "cases": cases,
        "commands": commands.iter().map(command_to_value).collect::<Vec<_>>(),
        "blockers": blockers,
        "migration_eligible": migration_eligible,
    }))
}

pub fn write_outcome(outcome: &Value) -> Result<PathBuf, String> {
    let path = outcome_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    }
    let text = serde_json::to_string_pretty(outcome)
        .map_err(|error| format!("outcome cannot be serialized: {error}"))?;
    std::fs::write(&path, format!("{text}\n"))
        .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
    Ok(path)
}

pub fn read_outcome() -> Result<Value, String> {
    read_json(&outcome_path())
}

/// Run one sibling test filter in a child process of the current test binary.
/// This is how the outcome test observes real exits without spawning Cargo, so
/// the recorded `command` is the argv that actually ran and the Cargo line is
/// recorded separately as `equivalent`.
pub fn run_sibling_test(filter: &str) -> CommandRecord {
    let equivalent =
        format!("cargo test -p slicer-wasm-host --test integration {filter} -- --exact --nocapture");
    let exe = std::env::current_exe();
    let command = match &exe {
        Ok(exe) => format!("{} {filter} --exact --nocapture", exe.display()),
        Err(error) => format!("<unresolved test binary: {error}> {filter} --exact --nocapture"),
    };
    let status = exe.ok().and_then(|exe| {
        std::process::Command::new(exe)
            .arg(filter)
            .arg("--exact")
            .arg("--nocapture")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .ok()
    });
    match status {
        Some(status) => CommandRecord {
            command,
            equivalent,
            exit: status.code(),
            exit_reason: status
                .code()
                .is_none()
                .then(|| "the child was terminated by a signal".to_string()),
            observed_by: "child process of the integration test binary".to_string(),
        },
        None => CommandRecord {
            command,
            equivalent,
            exit: None,
            exit_reason: Some("the child process could not be spawned or waited for".to_string()),
            observed_by: "child process of the integration test binary".to_string(),
        },
    }
}

pub fn self_command_record(filter: &str) -> CommandRecord {
    let equivalent =
        format!("cargo test -p slicer-wasm-host --test integration {filter} -- --exact --nocapture");
    let command = match std::env::current_exe() {
        Ok(exe) => format!("{} {filter} --exact --nocapture", exe.display()),
        Err(error) => format!("<unresolved test binary: {error}> {filter} --exact --nocapture"),
    };
    CommandRecord {
        command,
        equivalent,
        exit: None,
        exit_reason: Some(
            "this invocation is still running; it cannot observe its own exit code, so the harness records it"
                .to_string(),
        ),
        observed_by: "in-process".to_string(),
    }
}

pub fn status_label(status: CaseStatus) -> &'static str {
    match status {
        CaseStatus::Pass => "pass",
        CaseStatus::Fail => "fail",
        CaseStatus::Inconclusive => "inconclusive",
    }
}

