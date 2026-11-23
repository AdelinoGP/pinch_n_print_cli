//! Gate B uses independently decoded native and component evidence, never arm equality.
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use lightning_feasibility_kernel::{BoundaryAddress, DomainKind, GraphProvenance, KernelInput};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use slicer_ir::{ConfigKey, ConfigValue, ConfigView};
use slicer_sdk::native::{NativeLayerRequest, NativeStageEntry};
use slicer_wasm_host::host::{
    layer_infill, ConfigValueStorage, ConfigViewData, HostExecutionContext,
    HostExecutionContextBuilder, PaintRegionLayerData, SupportQueryViewData,
};
use wasmtime::component::Resource;

pub const PACKET: &str = "layer-module-preparation_09_lightning-kernel-portability";
pub const FILTERS: [&str; 3] = [
    "lightning_portability_tdd::planned_graph_native_and_wasm",
    "lightning_portability_tdd::gate_b_decision_is_truthful",
    "lightning_portability_tdd::rejects_canned_echo_and_fallback",
];
pub const NON_COMPARABLE: [&str; 7] = [
    "origin_region_index",
    "per_node_provenance",
    "sampled_segments",
    "grounding.kind",
    "reference.final_fills",
    "reference.policies",
    "final_fill_contributor_attribution",
];

pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
pub fn read_json(path: &Path) -> Value {
    serde_json::from_slice(
        &std::fs::read(path).unwrap_or_else(|e| panic!("{}: {e}", path.display())),
    )
    .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}
pub fn outcome_path() -> PathBuf {
    root().join("target/lightning-portability/outcome.json")
}

pub struct Case {
    pub id: String,
    pub input: KernelInput,
    pub reference: Value,
    pub absent: Value,
    pub clipped_count: usize,
    pub contributor_unavailable: Vec<Value>,
}

/// The canonical fixture records 1 nm points; KernelInput requires 100 nm points.
/// Walk point objects only, leaving identities, rational intervals, and mm fields intact.
fn convert_planning_geometry(value: &mut Value, case_id: &str) {
    match value {
        Value::Array(values) => {
            for value in values {
                convert_planning_geometry(value, case_id);
            }
        }
        Value::Object(fields) if fields.contains_key("x") && fields.contains_key("y") => {
            for axis in ["x", "y"] {
                let canonical = fields[axis].as_i64().expect("canonical integer coordinate");
                let converted = canonical_units_to_pnp(canonical) as i64;
                assert!(
                    converted.unsigned_abs() < 10_000_000,
                    "{case_id}: planning geometry {axis}={converted} exceeds 1000 mm; check canonical 1 nm to PnP 100 nm conversion"
                );
                fields.insert(axis.into(), json!(converted));
            }
        }
        Value::Object(fields) => {
            for value in fields.values_mut() {
                convert_planning_geometry(value, case_id);
            }
        }
        _ => (),
    }
}

fn assert_planning_scale(layers: &[lightning_feasibility_kernel::PlanningLayer], case_id: &str) {
    for layer in layers {
        let mut min = [i64::MAX; 2];
        let mut max = [i64::MIN; 2];
        for point in layer.internal_domains.iter().flat_map(|polygon| {
            std::iter::once(&polygon.contour)
                .chain(polygon.holes.iter())
                .flat_map(|ring| &ring.points)
        }) {
            for (axis, coordinate) in [point.x, point.y].into_iter().enumerate() {
                min[axis] = min[axis].min(coordinate);
                max[axis] = max[axis].max(coordinate);
            }
        }
        for axis in 0..2 {
            if min[axis] <= max[axis] {
                assert!(
                    max[axis] - min[axis] < 1_000_000,
                    "{case_id}: planning layer {} internal-domain span on axis {axis} must be <100 mm; unconverted canonical 1 nm fixture input previously produced 515 mm / 1642 mm spans",
                    layer.global_layer_index
                );
            }
        }
    }
}

pub fn cases() -> Vec<Case> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/lightning_gate");
    let manifest = read_json(&dir.join("manifest.json"));
    assert_eq!(manifest["units"]["reference"], "canonical_units_1nm");
    let canonical = read_json(&dir.join("canonical.json"));
    let inputs = read_json(&dir.join("inputs.json"));
    manifest["case_ids"]
        .as_array()
        .expect("manifest case_ids")
        .iter()
        .enumerate()
        .map(|(i, id)| {
            let id = id.as_str().expect("case id").to_owned();
            let c = &canonical["cases"][i];
            let source = &inputs["cases"][i]["a_source"];
            assert_eq!(c["case_id"], id);
            let reference = c["reference"].clone();
            let mut planning_geometry = reference["planning_layers"].clone();
            convert_planning_geometry(&mut planning_geometry, &id);
            let planning_layers: Vec<lightning_feasibility_kernel::PlanningLayer> =
                serde_json::from_value(planning_geometry).expect("planning_layers decode");
            assert_planning_scale(&planning_layers, &id);
            let input = KernelInput {
                object_id: planning_layers
                    .first()
                    .expect("planning layer")
                    .object_id
                    .clone(),
                layers: Vec::new(),
                planning_layers,
                canonical_region_order: serde_json::from_value(
                    source["canonical_region_order"].clone(),
                )
                .expect("region order decode"),
                scalar_source_region: serde_json::from_value(
                    reference["scalar_source_region"].clone(),
                )
                .expect("scalar region decode"),
                parameters: serde_json::from_value(reference["parameters"].clone())
                    .expect("parameters decode"),
            };
            assert!(serde_json::to_string(&input)
                .unwrap()
                .contains("\"layers\":[]"));
            let clipped_count = reference["clipped"]
                .as_array()
                .expect("clipped layers")
                .iter()
                .map(|layer| layer["fills"].as_array().expect("clipped fills").len())
                .sum();
            Case {
                id: id.clone(),
                input,
                reference,
                absent: manifest["cases"][&id]["absent"].clone(),
                clipped_count,
                contributor_unavailable: inputs["cases"][i]["b_input"]["contributor_evidence"]
                    .as_array()
                    .expect("contributor evidence")
                    .iter()
                    .filter(|entry| entry["status"] == "Unavailable")
                    .cloned()
                    .collect(),
            }
        })
        .collect()
}

#[derive(Clone, Copy, Debug)]
pub enum Producer {
    Native,
    Wasm,
}
impl Producer {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "Native" => Ok(Self::Native),
            "Wasm" => Ok(Self::Wasm),
            _ => Err(format!(
                "missing producer: {value}; no host fallback installed"
            )),
        }
    }
}
pub type Record = Vec<[f64; 2]>;
#[derive(Clone, Debug)]
pub struct Payload {
    pub producer: Producer,
    pub records: Vec<Record>,
}
#[derive(Clone, Debug)]
pub struct Node {
    pub id: u32,
    pub point: [f64; 2],
    pub parent: Option<u32>,
    pub root: u32,
    pub origin: u32,
    pub grounding: Option<BoundaryAddress>,
    pub ground_kind: u32,
}
#[derive(Clone, Debug)]
pub struct Edge {
    pub id: u32,
    pub parent: u32,
    pub child: u32,
    pub root: u32,
    pub origin: u32,
    pub parent_point: [f64; 2],
    pub child_point: [f64; 2],
}
#[derive(Clone, Debug)]
pub struct DecodedLayer {
    pub global_layer_index: u32,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub provenance: GraphProvenance,
}
pub struct DecodedArm {
    pub payload: Payload,
    pub layer: DecodedLayer,
}
fn r<T, U>(value: Resource<T>) -> Resource<U> {
    Resource::new_own(value.rep())
}

pub fn drive_source(case: &Case, source: &str, layer: u32) -> Result<DecodedArm, String> {
    Ok(drive(case, Producer::parse(source)?, layer))
}

pub fn drive(case: &Case, producer: Producer, layer: u32) -> DecodedArm {
    eprintln!("Gate B {} {producer:?}: starting producer", case.id);
    let input = serde_json::to_string(&case.input).unwrap();
    let records = match producer {
        Producer::Native => {
            // exhaustive: NativeLayerRequest has no defaults; all inputs are deliberately explicit.
            let request = NativeLayerRequest {
                layer_index: layer,
                regions: Vec::new(),
                perimeter_regions: None,
                paint: Some(slicer_sdk::traits::PaintRegionLayerView::new(layer)),
                prior_infill: None,
                stage_export: "Layer::Infill",
                ordered_entities: Vec::new(),
                config: ConfigView::from_map(HashMap::from([
                    (
                        ConfigKey::from("probe_input_json"),
                        ConfigValue::String(input),
                    ),
                    (
                        ConfigKey::from("probe_output"),
                        ConfigValue::String("planned".into()),
                    ),
                    (
                        ConfigKey::from("probe_layer_index"),
                        ConfigValue::Int(i64::from(layer)),
                    ),
                ])),
            };
            let NativeStageEntry::Layer(entry) =
                lightning_portability_guest::LightningPortabilityModule::__slicer_native_entry()
            else {
                panic!("not Layer")
            };
            let response = entry(&request).expect("native run");
            let output = response.infill.expect("native infill");
            assert_eq!(
                output.sparse_paths().len(),
                output.sparse_path_origins().len()
            );
            assert!(
                output.sparse_path_origins().iter().all(Option::is_some),
                "missing native origin"
            );
            output
                .sparse_paths()
                .iter()
                .map(|path| {
                    path.points
                        .iter()
                        .map(|p| [f64::from(p.x), f64::from(p.y)])
                        .collect()
                })
                .collect()
        }
        Producer::Wasm => {
            let engine = crate::common::wasm_cache::shared_engine()
                .wasmtime_engine()
                .clone();
            let component =
                crate::common::wasm_cache::compiled_guest("lightning-portability-guest");
            let mut linker = wasmtime::component::Linker::<HostExecutionContext>::new(&engine);
            layer_infill::LayerModule::add_to_linker::<_, wasmtime::component::HasSelf<_>>(
                &mut linker,
                |ctx| ctx,
            )
            .unwrap();
            let mut ctx =
                HostExecutionContextBuilder::new("lightning-portability", 0.0, 100.0).build();
            let cfg = ctx
                .push_config_view(ConfigViewData {
                    fields: HashMap::from([
                        ("probe_input_json".into(), ConfigValueStorage::Str(input)),
                        (
                            "probe_output".into(),
                            ConfigValueStorage::Str("planned".into()),
                        ),
                        (
                            "probe_layer_index".into(),
                            ConfigValueStorage::Int(i64::from(layer)),
                        ),
                    ]),
                })
                .unwrap();
            let out = ctx.push_infill_output_builder().unwrap();
            let paint = ctx
                .push_paint_region_layer_view(PaintRegionLayerData {
                    layer_index: layer,
                    ..Default::default()
                })
                .unwrap();
            let sq = ctx
                .push_support_query_view(SupportQueryViewData::default())
                .unwrap();
            let mut store = wasmtime::Store::new(&engine, ctx);
            let bindings = layer_infill::LayerModule::instantiate(
                &mut store,
                component.wasmtime_component(),
                &linker,
            )
            .unwrap();
            bindings
                .slicer_layer_infill_infill()
                .call_run(
                    &mut store,
                    layer as i32,
                    &[],
                    r(paint),
                    r(sq),
                    r(out),
                    r(cfg),
                )
                .expect("WASM call")
                .expect("WASM run");
            let ctx = store.into_data();
            let output = ctx.infill_output();
            assert_eq!(output.sparse_paths.len(), output.sparse_path_origins.len());
            assert!(
                output.sparse_path_origins.iter().all(Option::is_some),
                "missing WASM origin"
            );
            output
                .sparse_paths
                .iter()
                .map(|path| {
                    path.points
                        .iter()
                        .map(|p| [f64::from(p.x), f64::from(p.y)])
                        .collect()
                })
                .collect()
        }
    };
    eprintln!("Gate B {} {producer:?}: decoding records", case.id);
    let payload = Payload { producer, records };
    let decoded = decode(&payload, layer).expect("strict payload decode");
    DecodedArm {
        payload,
        layer: decoded,
    }
}

fn uint(v: f64, field: &str) -> Result<u32, String> {
    if !v.is_finite() || v < 0.0 || v.fract() != 0.0 || v >= f64::from(1_u32 << 24) {
        Err(format!(
            "{field}: non-integral, negative, non-finite or inexact metadata {v}"
        ))
    } else {
        Ok(v as u32)
    }
}
pub fn decode(payload: &Payload, global_layer_index: u32) -> Result<DecodedLayer, String> {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut chunks = BTreeMap::new();
    let mut stream = None;
    for record in &payload.records {
        if record.len() != 6 {
            return Err("wrong path length: expected exactly 6 points".into());
        }
        if record.iter().flatten().any(|v| !v.is_finite()) {
            return Err("non-finite coordinate".into());
        }
        let p = record;
        match uint(p[0][0], "tag")? {
            1 => {
                let id = uint(p[0][1], "node_id")?;
                if id == 0 || nodes.iter().any(|n: &Node| n.id == id) {
                    return Err("duplicate or zero node_id".into());
                }
                let parent = uint(p[2][0], "parent_id")?;
                let kind = uint(p[3][1], "ground_code")?;
                let surface_id = uint(p[4][0], "surface_id")?;
                let ring_id = uint(p[4][1], "ring_id")?;
                let segment_id = uint(p[5][0], "segment_id")?;
                let domain = uint(p[5][1], "domain_code")?;
                let grounding = match (kind, domain) {
                    (0, 0) if surface_id == 0 && ring_id == 0 && segment_id == 0 => None,
                    (1 | 2, 1..=3) => Some(BoundaryAddress {
                        surface_id,
                        ring_id,
                        segment_id,
                        domain: match domain {
                            1 => DomainKind::Internal,
                            2 => DomainKind::Void,
                            _ => DomainKind::TemporaryAnchor,
                        },
                    }),
                    _ => return Err("invalid grounding kind/domain/address".into()),
                };
                nodes.push(Node {
                    id,
                    point: p[1],
                    parent: (parent != 0).then_some(parent),
                    root: uint(p[2][1], "root_id")?,
                    origin: uint(p[3][0], "origin_region_index")?,
                    grounding,
                    ground_kind: kind,
                });
            }
            2 => {
                let id = uint(p[0][1], "edge_id")?;
                if id == 0 || edges.iter().any(|e: &Edge| e.id == id) {
                    return Err("duplicate or zero edge_id".into());
                }
                if p[5] != [0.0, 0.0] {
                    return Err("edge reserved fields not zero".into());
                }
                edges.push(Edge {
                    id,
                    parent: uint(p[1][0], "edge parent")?,
                    child: uint(p[1][1], "edge child")?,
                    parent_point: p[2],
                    child_point: p[3],
                    root: uint(p[4][0], "edge root")?,
                    origin: uint(p[4][1], "edge origin")?,
                });
            }
            3 => {
                let index = uint(p[0][1], "chunk_index")? as usize;
                let len = uint(p[1][0], "byte_len")? as usize;
                let count = uint(p[1][1], "chunk_count")? as usize;
                if len == 0 || count != len.div_ceil(8) || index >= count {
                    return Err("tag-3 inconsistent byte_len/chunk_count or extra chunk".into());
                }
                if stream.is_some_and(|s| s != (len, count)) {
                    return Err("tag-3 inconsistent stream metadata".into());
                }
                stream = Some((len, count));
                let mut bytes = Vec::new();
                for value in p[2..].iter().flatten() {
                    let byte = uint(*value, "tag-3 byte")?;
                    if byte > 255 {
                        return Err("tag-3 byte outside u8".into());
                    }
                    bytes.push(byte as u8);
                }
                if chunks.insert(index, bytes).is_some() {
                    return Err("tag-3 duplicate chunk".into());
                }
            }
            tag => return Err(format!("unknown tag {tag}")),
        }
    }
    let (len, count) = stream.ok_or("missing tag-3 provenance stream")?;
    if chunks.len() != count {
        return Err("tag-3 missing chunk".into());
    }
    let mut bytes = Vec::new();
    for i in 0..count {
        bytes.extend(chunks.get(&i).ok_or("tag-3 noncontiguous chunk indices")?);
    }
    if bytes[len..].iter().any(|b| *b != 0) {
        return Err("tag-3 nonzero final padding".into());
    }
    bytes.truncate(len);
    let text = std::str::from_utf8(&bytes).map_err(|e| format!("tag-3 invalid UTF-8: {e}"))?;
    let provenance: GraphProvenance =
        serde_json::from_str(text).map_err(|e| format!("tag-3 invalid GraphProvenance: {e}"))?;
    let layer = DecodedLayer {
        global_layer_index,
        nodes,
        edges,
        provenance,
    };
    validate(&layer, usize::MAX)?;
    Ok(layer)
}

pub fn validate(layer: &DecodedLayer, region_count: usize) -> Result<(), String> {
    let table: BTreeMap<_, _> = layer.nodes.iter().map(|n| (n.id, n)).collect();
    if layer.nodes.is_empty() && layer.edges.is_empty() {
        return Ok(()); // Input-aware empty-layer validation is in compare.
    }
    if layer.edges.is_empty() {
        return Err("missing edges/roots: empty edge population".into());
    }
    if !layer.nodes.iter().any(|n| n.parent.is_none()) {
        return Err("missing root population".into());
    }
    for n in &layer.nodes {
        if n.origin as usize >= region_count {
            return Err("origin_region_index out of range".into());
        }
        let root = table.get(&n.root).ok_or("disconnected root_id")?;
        if root.parent.is_some() {
            return Err("root_id resolves to non-root".into());
        }
        if let Some(parent) = n.parent {
            if !table.contains_key(&parent) {
                return Err("disconnected parent_id".into());
            }
            if !layer
                .edges
                .iter()
                .any(|e| e.parent == parent && e.child == n.id)
            {
                if layer
                    .edges
                    .iter()
                    .any(|e| e.child == parent && e.parent == n.id)
                {
                    return Err("reversed edge".into());
                }
                return Err("missing parent edge".into());
            }
        } else if n.root != n.id {
            return Err("root witness points to different root".into());
        }
        if let Some(address) = &n.grounding {
            if address.surface_id as usize >= layer.provenance.source_catalog.len() {
                return Err("grounding surface_id outside source_catalog".into());
            }
            if !layer
                .provenance
                .boundary_provenance
                .iter()
                .any(|b| &b.address == address)
            {
                return Err("grounding address resolves to no boundary".into());
            }
        }
        let mut seen = BTreeSet::new();
        let mut cursor = n;
        while let Some(parent) = cursor.parent {
            if !seen.insert(cursor.id) {
                return Err("disconnected cyclic parent chain".into());
            }
            cursor = table.get(&parent).ok_or("disconnected parent chain")?;
        }
        if cursor.id != n.root {
            return Err("disconnected root/parent chain".into());
        }
    }
    for e in &layer.edges {
        let p = table.get(&e.parent).ok_or_else(|| {
            format!(
                "edge {} parent_id: expected existing node, actual {}",
                e.id, e.parent
            )
        })?;
        let c = table.get(&e.child).ok_or_else(|| {
            format!(
                "edge {} child_id: expected existing node, actual {}",
                e.id, e.child
            )
        })?;
        if p.point != e.parent_point || c.point != e.child_point {
            return Err("edge endpoint witness differs from node point".into());
        }
        let mut ancestor = *c;
        while let Some(parent) = ancestor.parent {
            // Node validation above has already rejected missing parents and cycles.
            ancestor = table[&parent];
        }
        if e.root != ancestor.id {
            return Err(format!(
                "EDGE_ROOT_ANCESTOR_MISMATCH edge {} root_id: expected {}, actual {}",
                e.id, ancestor.id, e.root
            ));
        }
        if ancestor.parent.is_some() || ancestor.root != ancestor.id {
            return Err(format!(
                "EDGE_ROOT_WITNESS_MISMATCH edge {} root.parent_id/root_id: expected None/{}, actual {:?}/{}",
                e.id, ancestor.id, ancestor.parent, ancestor.root
            ));
        }
        if e.origin as usize >= region_count {
            return Err("edge origin_region_index out of range".into());
        }
    }
    Ok(())
}

fn canonical_units_to_pnp(value: i64) -> f64 {
    // f64::round implements the manifest's round-half-away-from-zero rule.
    (value as f64 / 100.0).round()
}
fn canonical_point(value: &Value) -> [f64; 2] {
    [0, 1].map(|i| canonical_units_to_pnp(value[i].as_i64().expect("canonical integer point")))
}
fn close(a: [f64; 2], b: [f64; 2]) -> bool {
    (a[0] - b[0]).abs() <= 1.0 && (a[1] - b[1]).abs() <= 1.0
}
fn segment_distance(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let d = [b[0] - a[0], b[1] - a[1]];
    let norm = d[0] * d[0] + d[1] * d[1];
    let t = if norm == 0.0 {
        0.0
    } else {
        (((p[0] - a[0]) * d[0] + (p[1] - a[1]) * d[1]) / norm).clamp(0.0, 1.0)
    };
    (p[0] - a[0] - t * d[0]).hypot(p[1] - a[1] - t * d[1])
}

/// A consuming bipartite match retains coincident but distinct occurrences.
fn match_multiset(
    adjacency: &[Vec<usize>],
    count: usize,
    label: &str,
) -> Result<Vec<usize>, String> {
    fn augment(
        i: usize,
        adjacency: &[Vec<usize>],
        seen: &mut [bool],
        owner: &mut [Option<usize>],
    ) -> bool {
        for &j in &adjacency[i] {
            if seen[j] {
                continue;
            }
            seen[j] = true;
            if owner[j].is_none_or(|old| augment(old, adjacency, seen, owner)) {
                owner[j] = Some(i);
                return true;
            }
        }
        false
    }
    let mut owner = vec![None; count];
    for i in 0..adjacency.len() {
        if !augment(i, adjacency, &mut vec![false; count], &mut owner) {
            return Err(format!("missing/reversed/disconnected {label} entry {i}"));
        }
    }
    if adjacency.len() != count {
        return Err(format!(
            "extra {label} entry: expected {}, got {count}",
            adjacency.len()
        ));
    }
    let mut result = vec![0; adjacency.len()];
    for (j, i) in owner.into_iter().enumerate() {
        result[i.expect("complete matching")] = j;
    }
    Ok(result)
}

pub fn compare(case: &Case, layer: &DecodedLayer) -> Result<(), String> {
    let planning = case
        .input
        .planning_layers
        .iter()
        .find(|p| p.global_layer_index == layer.global_layer_index)
        .ok_or("case-identity/layer-provenance mismatch: unrecorded layer")?;
    let expected = GraphProvenance {
        source_catalog: planning.source_catalog.clone(),
        boundary_provenance: planning.boundary_provenance.clone(),
        operations: planning.operations.clone(),
    };
    if layer.provenance != expected {
        return Err("case-identity/layer-provenance mismatch".into());
    }
    if !planning.internal_domains.is_empty() && layer.nodes.is_empty() {
        return Err("missing node population for internal domains".into());
    }
    if layer.nodes.is_empty()
        && !planning.temporary_anchors.is_empty()
        && !(case.id == "anchor_bridge_classic" && layer.global_layer_index == 2)
    {
        return Err("missing node population for temporary anchors".into());
    }
    validate(layer, case.input.canonical_region_order.len())?;
    let reference = case.reference["replay"]["production"]["graph"]
        .as_array()
        .expect("reference graph")
        .iter()
        .find(|l| l["layer_index"].as_u64() == Some(u64::from(layer.global_layer_index)))
        .expect("reference layer");
    let cn = reference["nodes"].as_array().expect("reference nodes");
    let ce = reference["edges"].as_array().expect("reference edges");
    let ct: BTreeMap<_, _> = cn
        .iter()
        .map(|n| (n["node_id"].as_i64().unwrap(), n))
        .collect();
    let nt: BTreeMap<_, _> = layer.nodes.iter().map(|n| (n.id, n)).collect();
    let cp = |id: &Value| canonical_point(&ct[&id.as_i64().unwrap()]["point"]);
    let adjacency: Vec<Vec<usize>> = cn
        .iter()
        .map(|c| {
            layer
                .nodes
                .iter()
                .enumerate()
                .filter_map(|(j, n)| {
                    let parent = c["parent_id"].as_i64().unwrap();
                    let same_parent = match n.parent {
                        None => parent == -1,
                        Some(p) => parent != -1 && close(nt[&p].point, cp(&c["parent_id"])),
                    };
                    (close(n.point, canonical_point(&c["point"]))
                        && same_parent
                        && close(nt[&n.root].point, cp(&c["root_id"]))
                        && n.parent.is_none() == c["is_root"].as_bool().unwrap())
                    .then_some(j)
                })
                .collect()
        })
        .collect();
    let matched = match_multiset(
        &adjacency,
        layer.nodes.len(),
        "node (point,parent point,root point,is_root)",
    )?;
    let edge_adjacency: Vec<Vec<usize>> = ce
        .iter()
        .map(|c| {
            layer
                .edges
                .iter()
                .enumerate()
                .filter_map(|(j, e)| {
                    (close(nt[&e.parent].point, cp(&c["parent_id"]))
                        && close(nt[&e.child].point, cp(&c["child_id"])))
                    .then_some(j)
                })
                .collect()
        })
        .collect();
    match_multiset(&edge_adjacency, layer.edges.len(), "directed edge")?;
    let isolated_c: Vec<_> = cn
        .iter()
        .filter(|n| n["is_root"] == true && !ce.iter().any(|e| e["parent_id"] == n["node_id"]))
        .collect();
    let isolated_n: Vec<_> = layer
        .nodes
        .iter()
        .filter(|n| n.parent.is_none() && !layer.edges.iter().any(|e| e.parent == n.id))
        .collect();
    let adjacency: Vec<Vec<usize>> = isolated_c
        .iter()
        .map(|c| {
            isolated_n
                .iter()
                .enumerate()
                .filter_map(|(j, n)| close(n.point, canonical_point(&c["point"])).then_some(j))
                .collect()
        })
        .collect();
    match_multiset(&adjacency, isolated_n.len(), "isolated root")?;
    for (i, c) in cn.iter().enumerate() {
        let n = &layer.nodes[matched[i]];
        match (c["last_grounding"].is_null(), &n.grounding) {
            (true, None) => (),
            (false, Some(address)) => {
                let b = layer
                    .provenance
                    .boundary_provenance
                    .iter()
                    .find(|b| &b.address == address)
                    .ok_or("grounding address resolves to no boundary")?;
                let [a, b] = b.endpoints.map(|p| [p.x as f64, p.y as f64]);
                if segment_distance(canonical_point(&c["last_grounding"]), a, b) > 1.0 {
                    return Err(format!(
                        "grounding boundary differs at canonical node occurrence {i}"
                    ));
                }
            }
            _ => {
                return Err(format!(
                    "grounding witness presence differs at canonical node occurrence {i}"
                ))
            }
        }
    }
    Ok(())
}

pub fn counts(layer: &DecodedLayer) -> Value {
    json!({"nodes":layer.nodes.len(),"edges":layer.edges.len(),"roots":layer.nodes.iter().filter(|n| n.parent.is_none()).count()})
}
fn non_comparable_node_records(layer: &DecodedLayer) -> Value {
    json!({"per_node_provenance":layer.nodes.iter().map(|n| json!({"node_id":n.id,
        "origin_region_index":n.origin,"grounding.kind":n.ground_kind,"boundary":n.grounding})).collect::<Vec<_>>(),
        "sampled_segments":{"status":"unavailable","reason":"not transported by the graph witness protocol"}})
}
pub struct Run {
    pub case: Case,
    pub native: Vec<DecodedArm>,
    pub wasm: Vec<DecodedArm>,
    pub decision: Value,
}
pub fn run_all() -> Vec<Run> {
    cases().into_iter().map(|case| {
        let native: Vec<_> = case.input.planning_layers.iter().map(|l| drive(&case, Producer::Native, l.global_layer_index)).collect();
        let wasm: Vec<_> = case.input.planning_layers.iter().map(|l| drive(&case, Producer::Wasm, l.global_layer_index)).collect();
        let mut mismatches = Vec::new();
        for (name,arms) in [("native", &native), ("wasm", &wasm)] {
            for arm in arms {
                if let Err(e) = compare_arm(&case, &arm.payload, arm.layer.global_layer_index) { mismatches.push(format!("{} {name}: {e}",case.id)); }
            }
        }
        let status = if mismatches.is_empty() { "pass" } else { "fail" };
        let decision = json!({"case_id":case.id,"status":status,"planned":{"status":status},"mismatches":mismatches,
            "unavailable":NON_COMPARABLE,"native":arm_counts(&native),"wasm":arm_counts(&wasm),
            "non_comparable_records":{"native":native.iter().map(|a| non_comparable_node_records(&a.layer)).collect::<Vec<_>>(),"wasm":wasm.iter().map(|a| non_comparable_node_records(&a.layer)).collect::<Vec<_>>(),
                "final_fill_contributor_attribution":contributor_record(&case)},
            "layers":native.iter().zip(&wasm).map(|(n,w)| json!({"global_layer_index":n.layer.global_layer_index,"native":counts(&n.layer),"wasm":counts(&w.layer),"provenance":n.layer.provenance})).collect::<Vec<_>>(),
            "clipped":{"status":"inconclusive","reason":case.absent["reference.final_fills"],"fill_counts":case.clipped_count,"absent_population":"reference.final_fills"}});
        Run { case,native,wasm,decision }
    }).collect()
}
pub fn compare_arm(case: &Case, payload: &Payload, layer: u32) -> Result<(), String> {
    let decoded =
        decode(payload, layer).map_err(|error| format!("{:?}: {error}", payload.producer))?;
    compare(case, &decoded)
}
pub fn arm_counts(arms: &[DecodedArm]) -> Value {
    let mut total = [0; 3];
    for arm in arms {
        let c = counts(&arm.layer);
        for (i, key) in ["nodes", "edges", "roots"].iter().enumerate() {
            total[i] += c[key].as_u64().unwrap();
        }
    }
    json!({"nodes":total[0],"edges":total[1],"roots":total[2]})
}
pub fn contributor_record(case: &Case) -> Value {
    json!({"unavailable_dispatch_count":case.contributor_unavailable.len(),
        "reasons":case.contributor_unavailable.iter().map(|e| e["reason"].clone()).collect::<Vec<_>>(),
        "dispatches":case.contributor_unavailable})
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum GateOutcome {
    Pass,
    Fail,
    Inconclusive,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FrameworkAcceptance {
    Verified,
    Unverified,
}
pub fn migration_eligible(
    gate_a: GateOutcome,
    gate_b: GateOutcome,
    framework: FrameworkAcceptance,
    executed: bool,
) -> bool {
    gate_a == GateOutcome::Pass
        && gate_b == GateOutcome::Pass
        && framework == FrameworkAcceptance::Verified
        && executed
}
pub fn aggregate(runs: &[Run]) -> &'static str {
    if runs.iter().any(|r| r.decision["status"] == "fail") {
        "fail"
    } else if runs.iter().all(|r| r.decision["status"] == "pass") {
        "pass"
    } else {
        "inconclusive"
    }
}

pub fn sha256_file(path: &Path) -> String {
    format!(
        "{:x}",
        Sha256::digest(
            std::fs::read(path).unwrap_or_else(|e| panic!("hash {}: {e}", path.display()))
        )
    )
}
pub fn run_sibling_test(filter: &str) -> Value {
    let exe = std::env::current_exe().unwrap();
    run_test_binary(
        &exe,
        filter,
        &format!(
            "cargo test -p slicer-wasm-host --test integration {filter} -- --exact --nocapture"
        ),
    )
}
fn run_test_binary(exe: &Path, filter: &str, equivalent: &str) -> Value {
    let output = std::process::Command::new(&exe)
        .args([filter, "--exact", "--nocapture"])
        .output()
        .expect("spawn sibling");
    let mut bytes = output.stdout;
    bytes.extend(output.stderr);
    // Zero-test filters are not accepted as execution evidence.
    let text = String::from_utf8_lossy(&bytes);
    let observed = text.contains("1 passed; 0 failed;");
    json!({"command":format!("{} {filter} --exact --nocapture",exe.display()),
        "equivalent":equivalent,
        "exit":output.status.code(),"exit_reason":if observed { Value::Null } else { json!("sibling did not report one passing test") },
        "observed_by":"child process of the integration test binary","log_sha256":format!("{:x}",Sha256::digest(&bytes))})
}
fn kernel_clip_sibling() -> Value {
    let filter = "planner::tests::clip_lightning_semantics_tdd";
    let manifest = "modules/core-modules/lightning-infill/feasibility-kernel/Cargo.toml";
    let command = format!("cargo test --manifest-path {manifest} {filter} -- --exact");
    let output = std::process::Command::new("cargo")
        .current_dir(root())
        .args(["test", "--manifest-path", manifest, filter, "--", "--exact"])
        .output()
        .expect("direct cargo kernel clip invocation");
    let mut bytes = output.stdout;
    bytes.extend(output.stderr);
    let observed = String::from_utf8_lossy(&bytes).contains("1 passed; 0 failed;");
    json!({"command":command,"equivalent":command,"exit":output.status.code(),
        "exit_reason":if observed { Value::Null } else { json!("direct cargo invocation did not report one passing test") },
        "observed_by":"direct cargo test --manifest-path kernel clip invocation",
        "log_sha256":format!("{:x}",Sha256::digest(&bytes))})
}
pub fn write_outcome(runs: &[Run], observe_commands: bool) -> Value {
    let mut commands = Vec::new();
    if observe_commands {
        for filter in FILTERS {
            commands.push(run_sibling_test(filter));
        }
        commands.push(kernel_clip_sibling());
    }
    commands.push(json!({"command":format!("{} lightning_portability_tdd::outcome_records_gate_b --exact",std::env::current_exe().unwrap().display()),
        "equivalent":"cargo test -p slicer-wasm-host --test integration lightning_portability_tdd::outcome_records_gate_b -- --exact",
        "exit":null,"exit_reason":"this invocation is still running; harness must record its exit","observed_by":"in-process"}));
    let freshness = std::process::Command::new("cargo")
        .current_dir(root())
        .args(["xtask", "build-guests", "--check"])
        .output()
        .expect("freshness check");
    let includes = [
        "generator.rs",
        "layer.rs",
        "tree_node.rs",
        "distance_field.rs",
    ]
    .map(|name| {
        let path = format!("modules/core-modules/lightning-infill/src/kernel/{name}");
        json!({"path":path,"sha256":sha256_file(&root().join(&path))})
    });
    let artifact = "crates/slicer-wasm-host/test-guests/lightning-portability-guest.component.wasm";
    let a_path = root().join("target/lightning-oracle/outcome.json");
    let a = if a_path.exists() {
        read_json(&a_path)["status"].clone()
    } else {
        Value::Null
    };
    let docs = std::fs::read_to_string(root().join("docs/07_implementation_status.md"))
        .expect("status doc");
    let a_line = docs
        .lines()
        .find(|l| l.contains("TASK-576 Gate A outcome:"))
        .map(str::to_owned);
    // Gate A's generated artifact is disposable; retain the durable recorded verdict.
    let a = if a.is_null() {
        a_line
            .as_deref()
            .and_then(|line| line.split_once("TASK-576 Gate A outcome:"))
            .map(|(_, status)| status.trim())
            .filter(|status| matches!(*status, "pass" | "fail" | "inconclusive"))
            .map_or(Value::Null, |status| json!(status))
    } else {
        a
    };
    let mut blockers = vec!["MIGRATION_BLOCKED_FRAMEWORK"];
    if a != "pass" || !a_line.as_ref().is_some_and(|l| l.contains("pass")) {
        blockers.push("MIGRATION_BLOCKED_GATE_A");
    }
    let status = aggregate(runs);
    let gate_a = if a == "pass" && a_line.as_ref().is_some_and(|l| l.contains("pass")) {
        GateOutcome::Pass
    } else {
        GateOutcome::Fail
    };
    let gate_b = if status == "pass" {
        GateOutcome::Pass
    } else {
        GateOutcome::Fail
    };
    if status != "pass" {
        blockers.push("MIGRATION_BLOCKED_GATE_B");
    }
    let non_comparable: Vec<_> = NON_COMPARABLE.iter().map(|population| {
        let reasons: Vec<_> = runs.iter().map(|r| json!({"case_id":r.case.id,"reason":match *population {
            "origin_region_index" | "sampled_segments" => r.case.absent["reference.graph"].clone(),
            "reference.final_fills" | "reference.policies" => r.case.absent[*population].clone(),
            "final_fill_contributor_attribution" => contributor_record(&r.case),
            _ => json!("canonical replay does not record this population"),
        }})).collect(); json!({"population":population,"status":"non_comparable","reasons":reasons})
    }).collect();
    let executed = observe_commands
        && commands
            .iter()
            .filter(|c| c["observed_by"] != "in-process")
            .all(|c| c["exit"] == 0 && c["exit_reason"].is_null());
    if !executed {
        blockers.push("MIGRATION_BLOCKED_UNEXECUTED");
    }
    let eligible = migration_eligible(gate_a, gate_b, FrameworkAcceptance::Unverified, executed);
    let outcome = json!({"gate":"B","packet":PACKET,"task_id":"TASK-576","status":status,"executed":executed,
        "includes":includes,"guest_artifact":{"path":artifact,"sha256":sha256_file(&root().join(artifact))},
        "build_guests_check_exit":freshness.status.code(),"cases":runs.iter().map(|r|r.decision.clone()).collect::<Vec<_>>(),
        "non_comparable_populations":non_comparable,"log_hashes":commands.iter().filter_map(|c|c.get("log_sha256").cloned()).collect::<Vec<_>>(),
        "commands":commands,"gate_a_status":a,"gate_a_record":a_line,"framework_acceptance":"Unverified",
        "framework_acceptance_reason":"not independently verified by this packet",
        "blockers":blockers,"migration_eligible":eligible});
    let path = outcome_path();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let temp = path.with_extension(format!("{}.tmp", std::process::id()));
    std::fs::write(&temp, serde_json::to_vec_pretty(&outcome).unwrap()).unwrap();
    // Windows rename cannot replace an existing file. Only this generated outcome is removed.
    if path.exists() {
        std::fs::remove_file(&path).unwrap();
    }
    std::fs::rename(temp, path).unwrap();
    outcome
}

pub fn corruption_records(arm: &DecodedArm, case: &Case) -> Vec<(&'static str, Payload)> {
    let original = &arm.payload;
    let mut result = Vec::new();
    let node = original
        .records
        .iter()
        .position(|r| r[0][0] == 1.0)
        .unwrap();
    let edge = original
        .records
        .iter()
        .position(|r| r[0][0] == 2.0)
        .unwrap();
    let chunk = original
        .records
        .iter()
        .position(|r| r[0][0] == 3.0)
        .unwrap();
    for name in [
        "missing node",
        "extra node",
        "duplicate node",
        "missing edge",
        "extra edge",
        "duplicate edge",
        "reversed edge",
        "disconnected parent",
        "corrupt root witness",
        "edge_root_set_to_nonroot_child",
        "tag-3 invalid UTF-8",
        "tag-3 nonzero final padding",
        "tag-3 missing chunk",
        "tag-3 inconsistent byte_len/chunk_count or extra chunk",
        "tag-3 duplicate chunk",
        "echoed input",
        "region-isolated payload",
    ] {
        let mut p = original.clone();
        match name {
            "missing node" => {
                let leaf = arm
                    .layer
                    .nodes
                    .iter()
                    .find(|n| !arm.layer.edges.iter().any(|e| e.parent == n.id))
                    .unwrap()
                    .id;
                p.records.retain(|r| {
                    !(r[0][0] == 1.0 && r[0][1] == f64::from(leaf))
                        && !(r[0][0] == 2.0 && r[1][1] == f64::from(leaf))
                });
            }
            "extra node" => {
                let mut n = p.records[node].clone();
                n[0][1] = 16_000_000.0;
                n[2] = [0.0, 16_000_000.0];
                p.records.push(n);
            }
            "duplicate node" => p.records.push(p.records[node].clone()),
            "missing edge" => {
                p.records.remove(edge);
            }
            "extra edge" => {
                let mut e = p.records[edge].clone();
                e[0][1] = 16_000_000.0;
                p.records.push(e);
            }
            "duplicate edge" => p.records.push(p.records[edge].clone()),
            "edge_root_set_to_nonroot_child" => {
                let child = p.records[edge][1][1];
                assert!(arm
                    .layer
                    .nodes
                    .iter()
                    .any(|n| f64::from(n.id) == child && n.parent.is_some()));
                p.records[edge][4][0] = child;
            }
            "reversed edge" => {
                p.records[edge][1].swap(0, 1);
                p.records[edge].swap(2, 3);
            }
            "disconnected parent" => {
                let child = p
                    .records
                    .iter_mut()
                    .find(|r| r[0][0] == 1.0 && r[2][0] != 0.0)
                    .unwrap();
                child[2][0] = 16_000_000.0;
            }
            "corrupt root witness" => {
                let i = p
                    .records
                    .iter()
                    .position(|r| r[0][0] == 1.0 && r[3][1] != 0.0)
                    .unwrap();
                p.records[i][5][0] = 16_000_000.0;
            }
            "tag-3 invalid UTF-8" => p.records[chunk][2][0] = 255.0,
            "tag-3 nonzero final padding" => {
                let last = p.records.iter_mut().rev().find(|r| r[0][0] == 3.0).unwrap();
                let used = last[1][0] as usize % 8;
                assert_ne!(used, 0, "synthetic provenance must exercise final padding");
                last[2 + used / 2][used % 2] = 1.0;
            }
            "tag-3 missing chunk" => {
                p.records.remove(chunk);
            }
            "tag-3 inconsistent byte_len/chunk_count or extra chunk" => {
                let mut extra = p.records[chunk].clone();
                extra[0][1] = extra[1][1];
                p.records.push(extra);
            }
            "tag-3 duplicate chunk" => p.records.push(p.records[chunk].clone()),
            "echoed input" => {
                p.records.retain(|r| r[0][0] == 3.0);
                for polygon in &case.input.planning_layers[0].internal_domains {
                    for point in &polygon.contour.points {
                        let id = p.records.len() as f64 + 1.0;
                        p.records.push(vec![
                            [1.0, id],
                            [point.x as f64, point.y as f64],
                            [0.0, id],
                            [0.0, 0.0],
                            [0.0, 0.0],
                            [0.0, 0.0],
                        ]);
                    }
                }
            }
            "region-isolated payload" => {
                // Remove a whole independently planned region, preserving valid transport and chains.
                let origin = arm.layer.nodes[0].origin;
                let ids: BTreeSet<_> = arm
                    .layer
                    .nodes
                    .iter()
                    .filter(|n| n.origin != origin)
                    .map(|n| n.id)
                    .collect();
                p.records.retain(|r| match r[0][0] as u32 {
                    1 => ids.contains(&(r[0][1] as u32)),
                    2 => ids.contains(&(r[1][0] as u32)) && ids.contains(&(r[1][1] as u32)),
                    _ => true,
                });
                assert!(!ids.is_empty());
                assert!(p.records.len() < original.records.len());
            }
            _ => unreachable!(),
        }
        let expected = match name {
            "missing node" | "region-isolated payload" => "missing/reversed/disconnected node",
            "extra node" => "extra node (point,parent point,root point,is_root) entry",
            "duplicate node" => "duplicate or zero node_id",
            "missing edge" => "missing parent edge",
            "extra edge" => "extra directed edge entry",
            "duplicate edge" => "duplicate or zero edge_id",
            "disconnected parent" => "disconnected parent_id",
            "corrupt root witness" => "grounding address resolves to no boundary",
            "edge_root_set_to_nonroot_child" => "EDGE_ROOT_ANCESTOR_MISMATCH",
            "echoed input" => "missing edges/roots",
            other => other,
        };
        result.push((expected, p));
    }
    result
}

/// SYNTHETIC baseline: a comparator/transport control, not a canonical parity oracle.
// test-quality: encode_plan/decode roundtrip and comparator negative controls only.
pub fn synthetic_baseline(offset: i64) -> (Case, lightning_feasibility_kernel::GraphLayer) {
    let parameters = cases().remove(0).input.parameters;
    let identities: Vec<_> = (0..2).map(|region| json!({"object_id":"SYNTHETIC baseline","region_id":region,"variant_chain":[]})).collect();
    let domains: Vec<_> = (0..2).map(|region| {
        let x = offset + region * 400_000;
        json!({"contour":{"points":[{"x":x,"y":0},{"x":x+200_000,"y":0},{"x":x+200_000,"y":200_000},{"x":x,"y":200_000}]},"holes":[]})
    }).collect();
    let sources: Vec<_> = (0..2).map(|i| json!({"id":{"identity":identities[i],"local_layer_index":0,"global_layer_index":1,
        "checkpoint":"during_planning","surface_token":format!("synthetic-{i}-{offset}")},"geometry":domains[i],"surface_kind":"internal","solid_angle_rad":null,"bridge_angle_rad":null})).collect();
    let mut boundaries = Vec::new();
    for i in 0..2 {
        for segment in 0..4 {
            boundaries.push(json!({"address":{"domain":"internal","surface_id":i,"ring_id":0,"segment_id":segment},
                "endpoints":[domains[i]["contour"]["points"][segment], domains[i]["contour"]["points"][(segment+1)%4]],
                "contributors":[{"source":{"surface":sources[i]["id"],"ring_id":segment/4,"segment_id":segment},
                    "relation":{"overlap":{"start":{"numerator":0,"denominator":1},"end":{"numerator":1,"denominator":1}}}}]}));
        }
    }
    let input: KernelInput = serde_json::from_value(json!({"object_id":"SYNTHETIC baseline","layers":[],
        "planning_layers":[{"object_id":"SYNTHETIC baseline","local_layer_index":0,"global_layer_index":1,"z_mm":0.4,
            "internal_domains":domains,"void_domains":[],"temporary_anchors":[],"source_regions":identities,
            "source_catalog":sources,"boundary_provenance":boundaries,"operations":[]}],
        "canonical_region_order":identities,"scalar_source_region":identities[0],"parameters":parameters})).unwrap();
    let graph = lightning_feasibility_kernel::planner::plan_lightning(&input)
        .unwrap()
        .layers
        .remove(0);
    assert!(!graph.nodes.is_empty() && !graph.edges.is_empty());
    let point = |p: slicer_ir::Point2| json!([p.x * 100, p.y * 100]);
    let nodes: Vec<_> = graph.nodes.iter().map(|n| {
        let grounding = n.grounding.as_ref().map(|g| {
            let boundary = graph.provenance.boundary_provenance.iter().find(|b| b.address == g.boundary).unwrap();
            point(boundary.endpoints[0])
        });
        json!({"node_id":n.node_id,"point":point(n.point),"parent_id":n.parent_id.map_or(-1,i64::from),
            "root_id":n.root_id,"is_root":n.parent_id.is_none(),"last_grounding":grounding})
    }).collect();
    let edges: Vec<_> = graph
        .edges
        .iter()
        .map(|e| json!({"edge_id":e.edge_id,"parent_id":e.parent_id,"child_id":e.child_id}))
        .collect();
    (
        Case {
            id: format!("SYNTHETIC baseline {offset}"),
            input,
            reference: json!({"replay":{"production":{"graph":[{"layer_index":1,"nodes":nodes,"edges":edges}]}}}),
            absent: json!({}),
            clipped_count: 0,
            contributor_unavailable: Vec::new(),
        },
        graph,
    )
}

pub fn encode_plan(
    graph: &lightning_feasibility_kernel::GraphLayer,
    producer: Producer,
) -> Payload {
    use lightning_feasibility_kernel::GroundKind;
    let mut records = Vec::new();
    for n in &graph.nodes {
        let (kind, surface, ring, segment, domain) =
            n.grounding.as_ref().map_or((0, 0, 0, 0, 0), |g| {
                (
                    match g.kind {
                        GroundKind::Boundary => 1,
                        GroundKind::TemporaryAnchor => 2,
                    },
                    g.boundary.surface_id,
                    g.boundary.ring_id,
                    g.boundary.segment_id,
                    match g.boundary.domain {
                        DomainKind::Internal => 1,
                        DomainKind::Void => 2,
                        DomainKind::TemporaryAnchor => 3,
                    },
                )
            });
        records.push(vec![
            [1.0, f64::from(n.node_id)],
            [n.point.x as f64, n.point.y as f64],
            [f64::from(n.parent_id.unwrap_or(0)), f64::from(n.root_id)],
            [f64::from(n.origin_region_index), f64::from(kind)],
            [f64::from(surface), f64::from(ring)],
            [f64::from(segment), f64::from(domain)],
        ]);
    }
    for e in &graph.edges {
        let parent = graph
            .nodes
            .iter()
            .find(|n| n.node_id == e.parent_id)
            .unwrap();
        let child = graph
            .nodes
            .iter()
            .find(|n| n.node_id == e.child_id)
            .unwrap();
        records.push(vec![
            [2.0, f64::from(e.edge_id)],
            [f64::from(e.parent_id), f64::from(e.child_id)],
            [parent.point.x as f64, parent.point.y as f64],
            [child.point.x as f64, child.point.y as f64],
            [f64::from(e.root_id), f64::from(e.origin_region_index)],
            [0.0, 0.0],
        ]);
    }
    let bytes = serde_json::to_vec(&graph.provenance).unwrap();
    for (i, chunk) in bytes.chunks(8).enumerate() {
        let mut padded = [0; 8];
        padded[..chunk.len()].copy_from_slice(chunk);
        let mut record = vec![
            [3.0, i as f64],
            [bytes.len() as f64, bytes.len().div_ceil(8) as f64],
        ];
        record.extend(
            padded
                .chunks_exact(2)
                .map(|p| [f64::from(p[0]), f64::from(p[1])]),
        );
        records.push(record);
    }
    // Both SDK arms transport f32 coordinates; model that conversion independently here.
    let records = match producer {
        Producer::Native => records
            .into_iter()
            .map(|r| {
                r.into_iter()
                    .map(|p| p.map(|v| f64::from(v as f32)))
                    .collect()
            })
            .collect(),
        Producer::Wasm => records
            .into_iter()
            .map(|r| {
                r.into_iter()
                    .map(|[x, y]| [f64::from(x as f32), f64::from(y as f32)])
                    .collect()
            })
            .collect(),
    };
    Payload { producer, records }
}
