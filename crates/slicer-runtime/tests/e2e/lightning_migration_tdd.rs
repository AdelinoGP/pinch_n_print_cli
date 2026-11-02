//! Step 2 of the lightning-migration packet: the acceptance harness (AC-1..AC-5,
//! AC-N1..AC-N4) for `docs/spec_packets/layer-module-preparation_08_lightning-migration`.
//!
//! # What each test protects
//!
//! | Test | Contract |
//! | --- | --- |
//! | `baseline_is_deterministic_and_strict` | The committed oracle decodes strictly, holds nonempty per-case populations and its recorded digests are the artifact's own |
//! | `module_plan_matches_recorded_baseline` | Native and fresh-WASM preparation each reproduce the recorded plan population (keys, edge multiplicity, witnesses) and report input-vs-plan divergence first |
//! | `fresh_consumers_emit_baseline_segments` | Ordinary `Layer::Infill` consumers emit the recorded per-caller segment multiset with `SparseInfill` role, recorded width and truthful lifecycle accounting |
//! | `typed_projection_is_atomic_and_neutral` | `planning_geometry`/`lightning-plan` is built from the same plan, commits with the pieces, is capture-neutral and fails closed by name |
//! | `legacy_contract_is_retired` | The live-contract scope (code + WIT) carries no retired symbol, the six live docs describe the retirement, and output still matches the baseline |
//! | `equivalence_rejects_corrupted_baseline` | The comparator rejects dropped/reversed/duplicated/swapped/length-only populations and fails its own empty/length-only control |
//! | `failed_or_malformed_plan_never_falls_back` | This module's `57401`/`57410`/`57411`/`57412` stay four distinct failures, publication is atomic and cleanup is truthful |
//! | `selection_override_empty_and_foreign_owner` | `invoke && !targets.is_empty()` gates preparation/consumption, external selection suppresses the native pair, Ready-empty is not missing, foreign/disposed owners are `AccessDenied` |
//! | `retirement_leaves_no_host_fallback` | Each removed symbol fails to compile on its own name and the slice still emits baseline-equivalent infill through the module |
//!
//! # Drivers (no adapter compares itself)
//!
//! Every population here is produced by the production slice/preparation/visual
//! capture facade over the same committed case material the Step 1 recorder used:
//! `prepare_prepass_context` for the recorded inputs, and
//! `run_slice_preparation_visual_capture` for real preparation plus the real
//! `Layer::Infill`/`Layer::Slice` taps. The oracle is always the committed
//! baseline fixture read back through [`load_baseline`]; no test re-derives the
//! plan from the baseline and calls the result equivalence, and native and WASM
//! arms are each compared to the baseline, never to each other.
//!
//! # Named blockers (RED evidence, reported rather than stubbed)
//!
//! * `B1` (AC-2 native leg) — **retired by Step 7c.** `slicer-integrated-modules::prepared_native_entries()`
//!   returns the module's generated `NativePreparedStageEntry`/`PreparationExportSchema`
//!   pair, and the native arm feeds it through `SliceRuntimeExtensions::prepared_native`
//!   with the lightning guest removed from the search roots so the integrated
//!   registration is the only provider for that module id.
//! * `B2` (AC-3 emitted segments): handled below — the per-layer `Layer::Infill`
//!   tap on `run_slice_preparation_visual_capture` is the observation surface.
//! * `B3` (AC-N2) — **retired by Step 7d.** The module-owned, default-`none`
//!   `prepared_plan_diagnostic` key publishes a missing piece, a decoder-rejected
//!   body and an unsupported-version header after planning, so 57401/57410/57411
//!   are reachable through the real consumer. The 57412 leg stays a real
//!   per-object config disagreement.
//! * `B4` (AC-N4): implemented with `rustc` compile witnesses, so no `trybuild`
//!   dev-dependency is needed (see [`compile_witness`]).
//! * `B5` (AC-N3) — **retired by Step 7d/8b.** `prepared_plan_diagnostic =
//!   empty_plan` is the module-owned Ready/no-work publication (empty pieces plus
//!   the empty `lightning-plan` scene), and the foreign/disposed `AccessDenied`
//!   leg runs through the already-public `PreparedOwnerStore`/`freeze_native`
//!   surface with the module's real frozen pair.

#![allow(clippy::too_many_lines)]

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use slicer_ir::{
    ConfigValue, ConfigView, ExtrusionRole, FacetPaintData, GlobalLayer, IndexedTriangleSet,
    InfillIR, MeshIR, ModuleId, ObjectConfig, ObjectMesh, PaintLayer, PaintSemantic, PaintValue,
    Point3, SliceIR, Transform3d,
};
use slicer_runtime::{
    run_slice_preparation_capture, run_slice_preparation_visual_capture,
    run_slice_with_runtime_extensions, CapturedIr, PreparationCapture,
    PreparationLifecycleObserver, PreparationLifecyclePhase, PreparationLifecycleRecord,
    PreparationLifecycleStatus, PreparationViewRequest, PreparationVisualCapture,
    PreparationVisualCaptureError, SliceRunOptions, SliceRuntimeExtensions,
};
use slicer_schema::preparation::{
    PreparationDeclaration, PreparationProjectionError, PreparationTransportError,
};
use slicer_schema::PreparationExportSchema;
use slicer_sdk::native::NativePreparedStageEntry;
use slicer_sdk::preparation::PreparationProjectionGeometry;
use slicer_wasm_host::preparation::{
    freeze_native, NativePreparationArtifact, PreparationFailure, PreparedOwnerStore,
};
use slicer_wasm_host::selection::{
    build_layer_selection_index, LayerSelectionIndexInput, LayerSelectionModuleInput,
};
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Committed artifact locations
// ---------------------------------------------------------------------------

/// Row-07 gate fixture: case **input material only** (never modified here).
const GATE_FIXTURE_REL: &str = "crates/slicer-wasm-host/tests/fixtures/lightning_gate";
/// Committed Step 1 baseline (the oracle).
const BASELINE_REL: &str = "crates/slicer-runtime/tests/fixtures/lightning_baseline";
const BASELINE_FILE: &str = "baseline_classic.json";
const MANIFEST_FILE: &str = "manifest.json";

/// The migrated owner under test.
const MODULE_ID: &str = "com.core.lightning-infill";
/// The projection view AC-4 requires the owner to declare.
const VIEW: &str = "planning_geometry";
const INFILL_TAP: &str = "Layer::Infill";
const SLICE_TAP: &str = "Layer::Slice";

/// Fixture geometry is canonical `coord_t` nanometres; PnP units are 100 nm.
const CANONICAL_UNITS_PER_PNP_UNIT: f64 = 100.0;
/// PnP units per millimetre (`docs/08`: 1 unit = 100 nm).
const UNITS_PER_MM: f64 = 10_000.0;

/// Case material's mesh scale, read from the committed baseline's `derivation`
/// block so the recorder and this harness cannot drift on it.
const BASELINE_DERIVATION_KEY: &str = "geometry_scale";
/// Must match the recorder's `PAINTED_CASE`: the one case whose committed
/// material carries the synthetic paint patch that exercises ordered
/// `variant_chain` identity. The baseline's `derivation.synthetic_paint` block
/// records the same derivation; the input digest is what proves the two
/// derivations agree.
const PAINTED_CASE: &str = "hole_void_classic";
/// Must match the recorder's `PAINT_TOOL_INDEX`.
const PAINT_TOOL_INDEX: u32 = 1;

// ---------------------------------------------------------------------------
// AC-1: strict decode of the committed oracle
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("workspace root must be resolvable")
}

fn read_text(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn is_digest(text: &str) -> bool {
    text.len() == 64 && text.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// One directed edge of a recorded edge multiset, in PnP units.
type Edge = (i64, i64, i64, i64);

/// Full plan identity: `(global_layer_index, object_id, region_id, ordered
/// variant_chain)`. The chain is kept in file order and its elements are the
/// canonical JSON text of the recorded `PaintValue`, so ordering is part of the
/// key and the text round-trips into the digest surface.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct PlanKey {
    global_layer_index: i32,
    object_id: String,
    region_id: u64,
    variant_chain: Vec<(String, String)>,
}

impl PlanKey {
    fn describe(&self) -> String {
        let chain: Vec<String> = self
            .variant_chain
            .iter()
            .map(|(semantic, value)| format!("{semantic}={value}"))
            .collect();
        format!(
            "(layer {}, object '{}', region {}, chain [{}])",
            self.global_layer_index,
            self.object_id,
            self.region_id,
            chain.join(", ")
        )
    }

    fn to_value(&self) -> Value {
        json!({
            "global_layer_index": self.global_layer_index,
            "object_id": self.object_id,
            "region_id": self.region_id,
            "variant_chain": self
                .variant_chain
                .iter()
                .map(|(semantic, value)| json!({
                    "semantic": semantic,
                    "value": serde_json::from_str::<Value>(value).expect("recorded chain value is JSON"),
                }))
                .collect::<Vec<Value>>(),
        })
    }

    fn from_infill(key_layer: u32, region: &slicer_ir::InfillRegion) -> Self {
        Self {
            global_layer_index: key_layer as i32,
            object_id: region.object_id.clone(),
            region_id: region.region_id,
            variant_chain: region
                .variant_chain
                .iter()
                .map(|(semantic, value)| {
                    (
                        semantic.clone(),
                        serde_json::to_string(value).expect("PaintValue serializes"),
                    )
                })
                .collect(),
        }
    }

    /// The same identity read from the committed `SliceIR` region, which is the
    /// caller population the recorder writes alongside the plan population.
    fn from_slice_region(key_layer: u32, region: &slicer_ir::SlicedRegion) -> Self {
        Self {
            global_layer_index: key_layer as i32,
            object_id: region.object_id.clone(),
            region_id: region.region_id,
            variant_chain: region
                .variant_chain
                .iter()
                .map(|(semantic, value)| {
                    (
                        semantic.clone(),
                        serde_json::to_string(value).expect("PaintValue serializes"),
                    )
                })
                .collect(),
        }
    }
}

#[derive(Clone, Debug)]
struct RecordedEntry {
    key: PlanKey,
    edge_count: u64,
    edges: Vec<(Edge, u64)>,
    witnesses: Vec<((i64, i64), i64)>,
}

#[derive(Clone, Debug)]
struct RecordedCaller {
    key: PlanKey,
    segment_count: u64,
    segments: Vec<(Edge, u64)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CaseDigests {
    case_source_sha256: String,
    inputs_sha256: String,
    plan_sha256: String,
}

#[derive(Clone, Debug)]
struct CaseConfig {
    sparse_fill_holder: String,
    infill_density: f64,
    layer_height: f64,
    infill_angle: f64,
    line_width: f64,
}

#[derive(Clone, Debug)]
struct BaselineCase {
    case_id: String,
    fixture_object_id: String,
    mesh_objects: Vec<String>,
    config: CaseConfig,
    layer_count: u64,
    plan_entry_count: u64,
    plan_edge_count: u64,
    caller_count: u64,
    caller_segment_count: u64,
    plan_entries: Vec<RecordedEntry>,
    callers: Vec<RecordedCaller>,
    digests: CaseDigests,
}

/// Decoded baseline: the case material plus the raw `derivation` block, which
/// the recorder carries forward so a re-record cannot silently drop it.
#[derive(Clone, Debug)]
struct Baseline {
    case_source: String,
    geometry_scale: f64,
    derivation: Value,
    cases: Vec<BaselineCase>,
}

#[derive(Clone, Debug)]
struct CaseSummary {
    case_id: String,
    fixture_object_id: String,
    mesh_objects: Vec<String>,
    layer_count: u64,
    plan_entry_count: u64,
    plan_edge_count: u64,
    caller_count: u64,
    caller_segment_count: u64,
    digests: CaseDigests,
}

#[derive(Clone, Debug)]
struct BaselineManifest {
    baseline_file: String,
    baseline_sha256: String,
    schema_version: u64,
    fixture_dir: String,
    fixture_inputs_sha256: String,
    fixture_manifest_sha256: String,
    fixture_case_count: u64,
    fixture_case_ids: Vec<String>,
    case_summaries: Vec<CaseSummary>,
}

type DecodeResult<T> = Result<T, String>;

fn expect_object<'a>(path: &str, value: &'a Value) -> DecodeResult<&'a Map<String, Value>> {
    value
        .as_object()
        .ok_or_else(|| format!("{path} must be a JSON object"))
}

/// Reject any key that is neither required nor optional, and any missing
/// required key. A baseline that grows a key must be re-recorded, never
/// silently tolerated.
fn expect_keys(
    path: &str,
    map: &Map<String, Value>,
    required: &[&str],
    optional: &[&str],
) -> DecodeResult<()> {
    for key in map.keys() {
        if !required.contains(&key.as_str()) && !optional.contains(&key.as_str()) {
            return Err(format!("{path} carries unknown key '{key}'"));
        }
    }
    for key in required {
        if !map.contains_key(*key) {
            return Err(format!("{path} is missing required key '{key}'"));
        }
    }
    Ok(())
}

fn expect_array<'a>(path: &str, value: &'a Value) -> DecodeResult<&'a Vec<Value>> {
    value
        .as_array()
        .ok_or_else(|| format!("{path} must be a JSON array"))
}

fn expect_str(path: &str, value: &Value) -> DecodeResult<String> {
    value
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| format!("{path} must be a string"))
}

fn expect_u64(path: &str, value: &Value) -> DecodeResult<u64> {
    value
        .as_u64()
        .ok_or_else(|| format!("{path} must be a non-negative integer"))
}

fn expect_i64(path: &str, value: &Value) -> DecodeResult<i64> {
    value
        .as_i64()
        .ok_or_else(|| format!("{path} must be an integer"))
}

fn expect_f64(path: &str, value: &Value) -> DecodeResult<f64> {
    value
        .as_f64()
        .ok_or_else(|| format!("{path} must be a number"))
}

fn expect_digest(path: &str, value: &Value) -> DecodeResult<String> {
    let digest = expect_str(path, value)?;
    if !is_digest(&digest) {
        return Err(format!("{path} is not a sha256 digest: {digest:?}"));
    }
    Ok(digest)
}

fn decode_chain(path: &str, value: &Value) -> DecodeResult<Vec<(String, String)>> {
    let mut chain = Vec::new();
    for (index, element) in expect_array(path, value)?.iter().enumerate() {
        let element_path = format!("{path}[{index}]");
        let map = expect_object(&element_path, element)?;
        expect_keys(&element_path, map, &["semantic", "value"], &[])?;
        let semantic = expect_str(&format!("{element_path}.semantic"), &map["semantic"])?;
        let canonical = canonical_json_text(&map["value"]);
        chain.push((semantic, canonical));
    }
    Ok(chain)
}

fn decode_key(path: &str, value: &Value) -> DecodeResult<PlanKey> {
    let map = expect_object(path, value)?;
    expect_keys(
        path,
        map,
        &[
            "global_layer_index",
            "object_id",
            "region_id",
            "variant_chain",
        ],
        &[],
    )?;
    Ok(PlanKey {
        global_layer_index: expect_i64(
            &format!("{path}.global_layer_index"),
            &map["global_layer_index"],
        )? as i32,
        object_id: expect_str(&format!("{path}.object_id"), &map["object_id"])?,
        region_id: expect_u64(&format!("{path}.region_id"), &map["region_id"])?,
        variant_chain: decode_chain(&format!("{path}.variant_chain"), &map["variant_chain"])?,
    })
}

fn decode_edges(path: &str, value: &Value) -> DecodeResult<Vec<(Edge, u64)>> {
    let mut edges = Vec::new();
    for (index, tuple) in expect_array(path, value)?.iter().enumerate() {
        let tuple_path = format!("{path}[{index}]");
        let items = expect_array(&tuple_path, tuple)?;
        if items.len() != 5 {
            return Err(format!(
                "{tuple_path} must be [from_x, from_y, to_x, to_y, count], got {items:?}"
            ));
        }
        let count = expect_u64(&format!("{tuple_path}[4]"), &items[4])?;
        if count == 0 {
            return Err(format!("{tuple_path} records a zero-multiplicity edge"));
        }
        edges.push((
            (
                expect_i64(&format!("{tuple_path}[0]"), &items[0])?,
                expect_i64(&format!("{tuple_path}[1]"), &items[1])?,
                expect_i64(&format!("{tuple_path}[2]"), &items[2])?,
                expect_i64(&format!("{tuple_path}[3]"), &items[3])?,
            ),
            count,
        ));
    }
    Ok(edges)
}

fn decode_witnesses(path: &str, value: &Value) -> DecodeResult<Vec<((i64, i64), i64)>> {
    let mut witnesses = Vec::new();
    for (index, tuple) in expect_array(path, value)?.iter().enumerate() {
        let tuple_path = format!("{path}[{index}]");
        let items = expect_array(&tuple_path, tuple)?;
        if items.len() != 3 {
            return Err(format!(
                "{tuple_path} must be [x, y, out_minus_in], got {items:?}"
            ));
        }
        let balance = expect_i64(&format!("{tuple_path}[2]"), &items[2])?;
        if balance == 0 {
            return Err(format!(
                "{tuple_path} records a zero incidence imbalance; witnesses are nonzero by construction"
            ));
        }
        witnesses.push((
            (
                expect_i64(&format!("{tuple_path}[0]"), &items[0])?,
                expect_i64(&format!("{tuple_path}[1]"), &items[1])?,
            ),
            balance,
        ));
    }
    Ok(witnesses)
}

fn decode_case(path: &str, value: &Value) -> DecodeResult<BaselineCase> {
    let map = expect_object(path, value)?;
    expect_keys(
        path,
        map,
        &[
            "case_id",
            "wall_strategy",
            "fixture_object_id",
            "fixture_region_ids",
            "mesh_objects",
            "config",
            "layer_count",
            "plan_entry_count",
            "plan_edge_count",
            "plan_entries",
            "caller_count",
            "caller_segment_count",
            "callers",
            "digests",
        ],
        &[],
    )?;

    let config_map = expect_object(&format!("{path}.config"), &map["config"])?;
    expect_keys(
        &format!("{path}.config"),
        config_map,
        &[
            "sparse_fill_holder",
            "infill_density",
            "layer_height",
            "infill_angle",
            "line_width",
        ],
        &[],
    )?;
    let config = CaseConfig {
        sparse_fill_holder: expect_str(
            &format!("{path}.config.sparse_fill_holder"),
            &config_map["sparse_fill_holder"],
        )?,
        infill_density: expect_f64(
            &format!("{path}.config.infill_density"),
            &config_map["infill_density"],
        )?,
        layer_height: expect_f64(
            &format!("{path}.config.layer_height"),
            &config_map["layer_height"],
        )?,
        infill_angle: expect_f64(
            &format!("{path}.config.infill_angle"),
            &config_map["infill_angle"],
        )?,
        line_width: expect_f64(
            &format!("{path}.config.line_width"),
            &config_map["line_width"],
        )?,
    };

    let mut plan_entries = Vec::new();
    for (index, entry) in expect_array(&format!("{path}.plan_entries"), &map["plan_entries"])?
        .iter()
        .enumerate()
    {
        let entry_path = format!("{path}.plan_entries[{index}]");
        let entry_map = expect_object(&entry_path, entry)?;
        expect_keys(
            &entry_path,
            entry_map,
            &["key", "edge_count", "edges", "witnesses"],
            &[],
        )?;
        plan_entries.push(RecordedEntry {
            key: decode_key(&format!("{entry_path}.key"), &entry_map["key"])?,
            edge_count: expect_u64(
                &format!("{entry_path}.edge_count"),
                &entry_map["edge_count"],
            )?,
            edges: decode_edges(&format!("{entry_path}.edges"), &entry_map["edges"])?,
            witnesses: decode_witnesses(
                &format!("{entry_path}.witnesses"),
                &entry_map["witnesses"],
            )?,
        });
    }

    let mut callers = Vec::new();
    for (index, caller) in expect_array(&format!("{path}.callers"), &map["callers"])?
        .iter()
        .enumerate()
    {
        let caller_path = format!("{path}.callers[{index}]");
        let caller_map = expect_object(&caller_path, caller)?;
        expect_keys(
            &caller_path,
            caller_map,
            &["key", "segment_count", "segments"],
            &[],
        )?;
        callers.push(RecordedCaller {
            key: decode_key(&format!("{caller_path}.key"), &caller_map["key"])?,
            segment_count: expect_u64(
                &format!("{caller_path}.segment_count"),
                &caller_map["segment_count"],
            )?,
            segments: decode_edges(&format!("{caller_path}.segments"), &caller_map["segments"])?,
        });
    }

    let digests_map = expect_object(&format!("{path}.digests"), &map["digests"])?;
    expect_keys(
        &format!("{path}.digests"),
        digests_map,
        &["case_source_sha256", "inputs_sha256", "plan_sha256"],
        &[],
    )?;
    let digests = CaseDigests {
        case_source_sha256: expect_digest(
            &format!("{path}.digests.case_source_sha256"),
            &digests_map["case_source_sha256"],
        )?,
        inputs_sha256: expect_digest(
            &format!("{path}.digests.inputs_sha256"),
            &digests_map["inputs_sha256"],
        )?,
        plan_sha256: expect_digest(
            &format!("{path}.digests.plan_sha256"),
            &digests_map["plan_sha256"],
        )?,
    };

    let mut mesh_objects = Vec::new();
    for (index, object) in expect_array(&format!("{path}.mesh_objects"), &map["mesh_objects"])?
        .iter()
        .enumerate()
    {
        mesh_objects.push(expect_str(
            &format!("{path}.mesh_objects[{index}]"),
            object,
        )?);
    }

    Ok(BaselineCase {
        case_id: expect_str(&format!("{path}.case_id"), &map["case_id"])?,
        fixture_object_id: expect_str(
            &format!("{path}.fixture_object_id"),
            &map["fixture_object_id"],
        )?,
        mesh_objects,
        config,
        layer_count: expect_u64(&format!("{path}.layer_count"), &map["layer_count"])?,
        plan_entry_count: expect_u64(
            &format!("{path}.plan_entry_count"),
            &map["plan_entry_count"],
        )?,
        plan_edge_count: expect_u64(&format!("{path}.plan_edge_count"), &map["plan_edge_count"])?,
        caller_count: expect_u64(&format!("{path}.caller_count"), &map["caller_count"])?,
        caller_segment_count: expect_u64(
            &format!("{path}.caller_segment_count"),
            &map["caller_segment_count"],
        )?,
        plan_entries,
        callers,
        digests,
    })
}

/// Decode `baseline_classic.json` with an explicit key allowlist at every level.
fn decode_baseline(text: &str) -> DecodeResult<Baseline> {
    let root: Value = serde_json::from_str(text)
        .map_err(|error| format!("baseline is not valid JSON: {error}"))?;
    let map = expect_object("baseline", &root)?;
    expect_keys(
        "baseline",
        map,
        &[
            "schema_version",
            "recorder",
            "producer",
            "case_source",
            "units",
            "derivation",
            "cases",
        ],
        &[],
    )?;
    if expect_u64("baseline.schema_version", &map["schema_version"])? != 1 {
        return Err("baseline.schema_version must be 1".to_owned());
    }
    let units = expect_object("baseline.units", &map["units"])?;
    expect_keys(
        "baseline.units",
        units,
        &["plan", "canonical", "pnp_units_per_canonical_unit"],
        &[],
    )?;
    let derivation = expect_object("baseline.derivation", &map["derivation"])?;
    let geometry_scale = expect_f64(
        &format!("baseline.derivation.{BASELINE_DERIVATION_KEY}"),
        derivation
            .get(BASELINE_DERIVATION_KEY)
            .ok_or_else(|| format!("baseline.derivation is missing '{BASELINE_DERIVATION_KEY}'"))?,
    )?;
    if !(geometry_scale.is_finite() && geometry_scale > 0.0) {
        return Err(format!(
            "baseline.derivation.{BASELINE_DERIVATION_KEY} must be positive, got {geometry_scale}"
        ));
    }

    let mut cases = Vec::new();
    for (index, case) in expect_array("baseline.cases", &map["cases"])?
        .iter()
        .enumerate()
    {
        cases.push(decode_case(&format!("baseline.cases[{index}]"), case)?);
    }
    Ok(Baseline {
        case_source: expect_str("baseline.case_source", &map["case_source"])?,
        geometry_scale,
        derivation: map["derivation"].clone(),
        cases,
    })
}

fn decode_manifest(text: &str) -> DecodeResult<BaselineManifest> {
    let root: Value = serde_json::from_str(text)
        .map_err(|error| format!("manifest is not valid JSON: {error}"))?;
    let map = expect_object("manifest", &root)?;
    expect_keys(
        "manifest",
        map,
        &[
            "schema_version",
            "recorder",
            "baseline_file",
            "baseline_sha256",
            "fixture",
            "case_summaries",
        ],
        &[],
    )?;
    let fixture_map = expect_object("manifest.fixture", &map["fixture"])?;
    expect_keys(
        "manifest.fixture",
        fixture_map,
        &[
            "dir",
            "inputs_sha256",
            "manifest_sha256",
            "case_count",
            "case_ids",
        ],
        &[],
    )?;
    let mut case_ids = Vec::new();
    for (index, id) in expect_array("manifest.fixture.case_ids", &fixture_map["case_ids"])?
        .iter()
        .enumerate()
    {
        case_ids.push(expect_str(
            &format!("manifest.fixture.case_ids[{index}]"),
            id,
        )?);
    }

    let mut case_summaries = Vec::new();
    for (index, summary) in expect_array("manifest.case_summaries", &map["case_summaries"])?
        .iter()
        .enumerate()
    {
        let path = format!("manifest.case_summaries[{index}]");
        let summary_map = expect_object(&path, summary)?;
        expect_keys(
            &path,
            summary_map,
            &[
                "case_id",
                "fixture_object_id",
                "mesh_objects",
                "layer_count",
                "plan_entry_count",
                "plan_edge_count",
                "caller_count",
                "caller_segment_count",
                "digests",
            ],
            &[],
        )?;
        let digests_map = expect_object(&format!("{path}.digests"), &summary_map["digests"])?;
        expect_keys(
            &format!("{path}.digests"),
            digests_map,
            &["case_source_sha256", "inputs_sha256", "plan_sha256"],
            &[],
        )?;
        let mut mesh_objects = Vec::new();
        for (object_index, object) in expect_array(
            &format!("{path}.mesh_objects"),
            &summary_map["mesh_objects"],
        )?
        .iter()
        .enumerate()
        {
            mesh_objects.push(expect_str(
                &format!("{path}.mesh_objects[{object_index}]"),
                object,
            )?);
        }
        case_summaries.push(CaseSummary {
            case_id: expect_str(&format!("{path}.case_id"), &summary_map["case_id"])?,
            fixture_object_id: expect_str(
                &format!("{path}.fixture_object_id"),
                &summary_map["fixture_object_id"],
            )?,
            mesh_objects,
            layer_count: expect_u64(&format!("{path}.layer_count"), &summary_map["layer_count"])?,
            plan_entry_count: expect_u64(
                &format!("{path}.plan_entry_count"),
                &summary_map["plan_entry_count"],
            )?,
            plan_edge_count: expect_u64(
                &format!("{path}.plan_edge_count"),
                &summary_map["plan_edge_count"],
            )?,
            caller_count: expect_u64(
                &format!("{path}.caller_count"),
                &summary_map["caller_count"],
            )?,
            caller_segment_count: expect_u64(
                &format!("{path}.caller_segment_count"),
                &summary_map["caller_segment_count"],
            )?,
            digests: CaseDigests {
                case_source_sha256: expect_digest(
                    &format!("{path}.digests.case_source_sha256"),
                    &digests_map["case_source_sha256"],
                )?,
                inputs_sha256: expect_digest(
                    &format!("{path}.digests.inputs_sha256"),
                    &digests_map["inputs_sha256"],
                )?,
                plan_sha256: expect_digest(
                    &format!("{path}.digests.plan_sha256"),
                    &digests_map["plan_sha256"],
                )?,
            },
        });
    }

    Ok(BaselineManifest {
        baseline_file: expect_str("manifest.baseline_file", &map["baseline_file"])?,
        baseline_sha256: expect_digest("manifest.baseline_sha256", &map["baseline_sha256"])?,
        schema_version: expect_u64("manifest.schema_version", &map["schema_version"])?,
        fixture_dir: expect_str("manifest.fixture.dir", &fixture_map["dir"])?,
        fixture_inputs_sha256: expect_digest(
            "manifest.fixture.inputs_sha256",
            &fixture_map["inputs_sha256"],
        )?,
        fixture_manifest_sha256: expect_digest(
            "manifest.fixture.manifest_sha256",
            &fixture_map["manifest_sha256"],
        )?,
        fixture_case_count: expect_u64("manifest.fixture.case_count", &fixture_map["case_count"])?,
        fixture_case_ids: case_ids,
        case_summaries,
    })
}

// ---------------------------------------------------------------------------
// Canonical hashing (identical to the Step 1 recorder's `sha256_canonical`)
// ---------------------------------------------------------------------------

fn canonical_json_text(value: &Value) -> String {
    let mut text = String::new();
    write_canonical(value, &mut text);
    text
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
                out.push_str(&serde_json::to_string(key).expect("JSON string key serializes"));
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

fn sha256_canonical(value: &Value) -> String {
    sha256_hex(canonical_json_text(value).as_bytes())
}

/// Canonical directed edge multiset of one recorded population.
fn edge_multiset(edges: &[(Edge, u64)]) -> BTreeMap<Edge, u64> {
    let mut multiset = BTreeMap::new();
    for (edge, count) in edges {
        *multiset.entry(*edge).or_insert(0) += *count;
    }
    multiset
}

/// Witness projection recorded by the oracle: directed incidence imbalance
/// (`out - in`) per vertex, derived from the edge multiset and never invented.
fn witness_set(edges: &BTreeMap<Edge, u64>) -> BTreeMap<(i64, i64), i64> {
    let mut balance: BTreeMap<(i64, i64), i64> = BTreeMap::new();
    for ((from_x, from_y, to_x, to_y), count) in edges {
        *balance.entry((*from_x, *from_y)).or_insert(0) += *count as i64;
        *balance.entry((*to_x, *to_y)).or_insert(0) -= *count as i64;
    }
    balance.retain(|_, out_minus_in| *out_minus_in != 0);
    balance
}

fn entry_to_value(entry: &RecordedEntry) -> Value {
    let edges = edge_multiset(&entry.edges);
    json!({
        "key": entry.key.to_value(),
        "edge_count": entry.edge_count,
        "edges": edges
            .iter()
            .map(|((from_x, from_y, to_x, to_y), count)| json!([from_x, from_y, to_x, to_y, count]))
            .collect::<Vec<Value>>(),
        "witnesses": entry
            .witnesses
            .iter()
            .map(|((x, y), balance)| json!([x, y, balance]))
            .collect::<Vec<Value>>(),
    })
}

fn plan_digest(case: &BaselineCase) -> String {
    sha256_canonical(&Value::Array(
        case.plan_entries.iter().map(entry_to_value).collect(),
    ))
}

// ---------------------------------------------------------------------------
// AC-1: artifact validation (strict, fail-closed, no producer re-run)
// ---------------------------------------------------------------------------

/// Read both committed files and validate the artifact as an artifact: strict
/// decode, internal consistency, cross-file agreement, recorded digests against
/// the bytes and recomputed plan/case-source hashes.
fn load_baseline(root: &Path) -> Result<(Baseline, BaselineManifest, String), String> {
    let dir = root.join(BASELINE_REL);
    let baseline_path = dir.join(BASELINE_FILE);
    let manifest_path = dir.join(MANIFEST_FILE);
    if !baseline_path.is_file() {
        return Err(format!(
            "committed baseline is absent: {} (an absent baseline is an error, never a skip)",
            baseline_path.display()
        ));
    }
    if !manifest_path.is_file() {
        return Err(format!(
            "committed baseline manifest is absent: {}",
            manifest_path.display()
        ));
    }
    let baseline_text = read_text(&baseline_path)?;
    let manifest_text = read_text(&manifest_path)?;
    let baseline = decode_baseline(&baseline_text)?;
    let manifest = decode_manifest(&manifest_text)?;
    validate_artifact(root, &baseline, &manifest, &baseline_text)?;
    Ok((baseline, manifest, baseline_text))
}

fn validate_artifact(
    root: &Path,
    baseline: &Baseline,
    manifest: &BaselineManifest,
    baseline_text: &str,
) -> Result<(), String> {
    let mut errors: Vec<String> = Vec::new();
    let mut record = |result: Result<(), String>| {
        if let Err(error) = result {
            errors.push(error);
        }
    };

    record(validate_manifest_envelope(root, manifest, baseline_text));
    record(validate_cases_against_manifest(baseline, manifest));
    record(validate_case_source_digests(root, baseline));

    for case in &baseline.cases {
        record(validate_case(case));
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

fn validate_manifest_envelope(
    root: &Path,
    manifest: &BaselineManifest,
    baseline_text: &str,
) -> Result<(), String> {
    if manifest.schema_version != 1 {
        return Err(format!(
            "manifest.schema_version must be 1, got {}",
            manifest.schema_version
        ));
    }
    if manifest.baseline_file != BASELINE_FILE {
        return Err(format!(
            "manifest.baseline_file names '{}', expected '{BASELINE_FILE}'",
            manifest.baseline_file
        ));
    }
    let actual = sha256_hex(baseline_text.as_bytes());
    if manifest.baseline_sha256 != actual {
        return Err(format!(
            "tampered digest: manifest.baseline_sha256 is {} but the committed bytes hash to {actual}",
            manifest.baseline_sha256
        ));
    }
    let fixture_dir = root.join(&manifest.fixture_dir);
    if !fixture_dir.is_dir() {
        return Err(format!(
            "manifest.fixture.dir is absent: {}",
            fixture_dir.display()
        ));
    }
    let inputs_text = read_text(&fixture_dir.join("inputs.json"))?;
    let fixture_manifest_text = read_text(&fixture_dir.join("manifest.json"))?;
    if sha256_hex(inputs_text.as_bytes()) != manifest.fixture_inputs_sha256 {
        return Err(
            "manifest.fixture.inputs_sha256 does not match the committed fixture".to_owned(),
        );
    }
    if sha256_hex(fixture_manifest_text.as_bytes()) != manifest.fixture_manifest_sha256 {
        return Err(
            "manifest.fixture.manifest_sha256 does not match the committed fixture".to_owned(),
        );
    }
    if manifest.fixture_case_count != manifest.fixture_case_ids.len() as u64 {
        return Err(format!(
            "manifest.fixture.case_count is {} but case_ids lists {}",
            manifest.fixture_case_count,
            manifest.fixture_case_ids.len()
        ));
    }
    if manifest.fixture_case_count == 0 {
        return Err(
            "manifest.fixture lists no cases; an empty case population is not a baseline"
                .to_owned(),
        );
    }
    Ok(())
}

fn validate_cases_against_manifest(
    baseline: &Baseline,
    manifest: &BaselineManifest,
) -> Result<(), String> {
    let baseline_ids: Vec<&str> = baseline
        .cases
        .iter()
        .map(|case| case.case_id.as_str())
        .collect();
    if baseline_ids.is_empty() {
        return Err(
            "baseline carries no cases; an empty case population is not a baseline".to_owned(),
        );
    }
    if baseline_ids != manifest.fixture_case_ids {
        return Err(format!(
            "baseline case ids {baseline_ids:?} do not match manifest case_ids {:?}",
            manifest.fixture_case_ids
        ));
    }
    if manifest.case_summaries.len() != baseline.cases.len() {
        return Err(format!(
            "manifest carries {} case summaries but the baseline carries {} cases",
            manifest.case_summaries.len(),
            baseline.cases.len()
        ));
    }
    if baseline.case_source != manifest.fixture_dir {
        return Err(format!(
            "baseline.case_source '{}' does not match manifest.fixture.dir '{}'",
            baseline.case_source, manifest.fixture_dir
        ));
    }

    for (case, summary) in baseline.cases.iter().zip(&manifest.case_summaries) {
        let label = &case.case_id;
        if summary.case_id != *label {
            return Err(format!(
                "manifest summary '{}' is out of order with baseline case '{label}'",
                summary.case_id
            ));
        }
        if summary.fixture_object_id != case.fixture_object_id
            || summary.mesh_objects != case.mesh_objects
        {
            return Err(format!(
                "case `{label}`: manifest fixture identity disagrees with the baseline"
            ));
        }
        if summary.layer_count != case.layer_count
            || summary.plan_entry_count != case.plan_entry_count
            || summary.plan_edge_count != case.plan_edge_count
            || summary.caller_count != case.caller_count
            || summary.caller_segment_count != case.caller_segment_count
            || summary.digests != case.digests
        {
            return Err(format!(
                "case `{label}`: manifest summary disagrees with the recorded counts or digests"
            ));
        }
    }
    Ok(())
}

fn validate_case_source_digests(root: &Path, baseline: &Baseline) -> Result<(), String> {
    let inputs_text = read_text(&root.join(&baseline.case_source).join("inputs.json"))?;
    let inputs: Value = serde_json::from_str(&inputs_text)
        .map_err(|error| format!("fixture inputs.json is not valid JSON: {error}"))?;
    let cases = inputs
        .get("cases")
        .and_then(Value::as_array)
        .ok_or_else(|| "fixture inputs.json carries no cases array".to_owned())?;
    for case in &baseline.cases {
        let source = cases
            .iter()
            .find(|candidate| {
                candidate.get("case_id").and_then(Value::as_str) == Some(&case.case_id)
            })
            .and_then(|candidate| candidate.get("a_source"))
            .ok_or_else(|| {
                format!(
                    "fixture inputs.json has no a_source for baseline case '{}'",
                    case.case_id
                )
            })?;
        let actual = sha256_canonical(source);
        if actual != case.digests.case_source_sha256 {
            return Err(format!(
                "tampered digest: case `{}` records case_source_sha256 {} but the committed case material hashes to {actual}",
                case.case_id, case.digests.case_source_sha256
            ));
        }
    }
    Ok(())
}

fn validate_case(case: &BaselineCase) -> Result<(), String> {
    let label = &case.case_id;
    if case.layer_count == 0 {
        return Err(format!("case `{label}`: layer_count is zero"));
    }
    if !(case.config.infill_density.is_finite()
        && case.config.infill_density > 0.0
        && case.config.line_width.is_finite()
        && case.config.line_width > 0.0)
    {
        return Err(format!(
            "case `{label}`: recorded planner config is not usable (density {}, width {})",
            case.config.infill_density, case.config.line_width
        ));
    }

    if case.plan_entries.is_empty() {
        return Err(format!(
            "case `{label}`: the plan population is empty; an empty population is not a baseline"
        ));
    }
    if case.plan_entry_count != case.plan_entries.len() as u64 {
        return Err(format!(
            "case `{label}`: plan_entry_count {} disagrees with {} recorded entries",
            case.plan_entry_count,
            case.plan_entries.len()
        ));
    }
    let mut plan_keys: BTreeSet<PlanKey> = BTreeSet::new();
    let mut total_edges: u64 = 0;
    for entry in &case.plan_entries {
        if !plan_keys.insert(entry.key.clone()) {
            return Err(format!(
                "case `{label}`: duplicated plan entry for key {}",
                entry.key.describe()
            ));
        }
        let edges = edge_multiset(&entry.edges);
        let count: u64 = edges.values().sum();
        if count == 0 {
            return Err(format!(
                "case `{label}`: plan entry {} records no edges",
                entry.key.describe()
            ));
        }
        if entry.edge_count != count {
            return Err(format!(
                "case `{label}`: plan entry {} records edge_count {} but its multiset totals {count}",
                entry.key.describe(),
                entry.edge_count
            ));
        }
        let derived = witness_set(&edges);
        if derived.is_empty() {
            return Err(format!(
                "case `{label}`: plan entry {} has no witness population",
                entry.key.describe()
            ));
        }
        let recorded: BTreeMap<(i64, i64), i64> = entry.witnesses.iter().copied().collect();
        if recorded != derived {
            return Err(format!(
                "case `{label}`: plan entry {} records witnesses that its edge multiset does not imply",
                entry.key.describe()
            ));
        }
        total_edges += count;
    }
    if case.plan_edge_count != total_edges {
        return Err(format!(
            "case `{label}`: plan_edge_count {} disagrees with the recorded multiset total {total_edges}",
            case.plan_edge_count
        ));
    }
    let expected_plan_digest = plan_digest(case);
    if expected_plan_digest != case.digests.plan_sha256 {
        return Err(format!(
            "tampered digest: case `{label}` records plan_sha256 {} but its recorded entries hash to {expected_plan_digest}",
            case.digests.plan_sha256
        ));
    }

    if case.callers.is_empty() {
        return Err(format!(
            "case `{label}`: the caller population is empty; an empty population is not a baseline"
        ));
    }
    if case.caller_count != case.callers.len() as u64 {
        return Err(format!(
            "case `{label}`: caller_count {} disagrees with {} recorded callers",
            case.caller_count,
            case.callers.len()
        ));
    }
    let mut caller_keys: BTreeSet<PlanKey> = BTreeSet::new();
    let mut caller_segments: u64 = 0;
    for caller in &case.callers {
        if !caller_keys.insert(caller.key.clone()) {
            return Err(format!(
                "case `{label}`: duplicated caller for key {}",
                caller.key.describe()
            ));
        }
        let segments = edge_multiset(&caller.segments);
        let count: u64 = segments.values().sum();
        if caller.segment_count != count {
            return Err(format!(
                "case `{label}`: caller {} records segment_count {} but its multiset totals {count}",
                caller.key.describe(),
                caller.segment_count
            ));
        }
        caller_segments += count;
    }
    if case.caller_segment_count != caller_segments {
        return Err(format!(
            "case `{label}`: caller_segment_count {} disagrees with the recorded multiset total {caller_segments}",
            case.caller_segment_count
        ));
    }
    if caller_segments == 0 {
        return Err(format!(
            "case `{label}`: every caller received zero segments; the caller population is vacuous"
        ));
    }

    // The plan's host-visible partition is the caller multiset. AC-2/AC-3 compare
    // the module's emitted output against the plan; that is only sound if the
    // committed artifact itself records the two populations as the same edges.
    let callers_by_key: BTreeMap<&PlanKey, &RecordedCaller> = case
        .callers
        .iter()
        .map(|caller| (&caller.key, caller))
        .collect();
    for entry in &case.plan_entries {
        let caller = callers_by_key.get(&entry.key).ok_or_else(|| {
            format!(
                "case `{label}`: plan entry {} has no caller record",
                entry.key.describe()
            )
        })?;
        if edge_multiset(&caller.segments) != edge_multiset(&entry.edges) {
            return Err(format!(
                "case `{label}`: plan entry {} and its caller record disagree on the edge multiset",
                entry.key.describe()
            ));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Population comparison (AC-2, AC-3, AC-N1)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
struct EntryPopulation {
    edges: BTreeMap<Edge, u64>,
}

type Population = BTreeMap<PlanKey, EntryPopulation>;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Mismatch {
    case_id: String,
    key: Option<PlanKey>,
    detail: String,
}

impl std::fmt::Display for Mismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.key {
            Some(key) => write!(
                f,
                "case `{}` key {}: {}",
                self.case_id,
                key.describe(),
                self.detail
            ),
            None => write!(f, "case `{}`: {}", self.case_id, self.detail),
        }
    }
}

fn mismatch(case_id: &str, key: Option<&PlanKey>, detail: impl Into<String>) -> Mismatch {
    Mismatch {
        case_id: case_id.to_owned(),
        key: key.cloned(),
        detail: detail.into(),
    }
}

/// Build a population, rejecting a duplicated key instead of silently merging
/// it (a duplicate is an identity error, not a merge).
fn try_population<'a>(
    case_id: &str,
    entries: impl IntoIterator<Item = (&'a PlanKey, BTreeMap<Edge, u64>)>,
) -> Result<Population, Mismatch> {
    let mut population: Population = BTreeMap::new();
    for (key, edges) in entries {
        if population
            .insert(key.clone(), EntryPopulation { edges })
            .is_some()
        {
            return Err(mismatch(
                case_id,
                Some(key),
                "duplicated entry; identity keys are unique by contract",
            ));
        }
    }
    Ok(population)
}

fn population_from_plan_entries(case: &BaselineCase) -> Result<Population, Mismatch> {
    try_population(
        &case.case_id,
        case.plan_entries
            .iter()
            .map(|entry| (&entry.key, edge_multiset(&entry.edges))),
    )
}

fn population_from_callers(case: &BaselineCase) -> Result<Population, Mismatch> {
    try_population(
        &case.case_id,
        case.callers
            .iter()
            .map(|caller| (&caller.key, edge_multiset(&caller.segments))),
    )
}

/// The plan's identity population is the *nonempty* subset of the emitted
/// population: the caller population also carries regions with no trees, and an
/// empty region is not a plan entry.
fn nonempty(population: Population) -> Population {
    population
        .into_iter()
        .filter(|(_, entry)| !entry.edges.is_empty())
        .collect()
}

/// The recordable plan population of one real module run: every emitted
/// `Layer::Infill` region, keyed by its full identity, with the directed edge
/// multiset recovered from the emitted sparse paths.
fn population_from_infill(
    case_id: &str,
    layer_index: u32,
    infill: &InfillIR,
) -> Result<Population, Mismatch> {
    let mut entries: Vec<(PlanKey, BTreeMap<Edge, u64>)> = Vec::new();
    for region in &infill.regions {
        let mut edges: BTreeMap<Edge, u64> = BTreeMap::new();
        for path in &region.sparse_infill {
            for pair in path.points.windows(2) {
                let edge = (
                    mm_to_units(pair[0].x),
                    mm_to_units(pair[0].y),
                    mm_to_units(pair[1].x),
                    mm_to_units(pair[1].y),
                );
                *edges.entry(edge).or_insert(0) += 1;
            }
        }
        entries.push((PlanKey::from_infill(layer_index, region), edges));
    }
    try_population(
        case_id,
        entries.iter().map(|(key, edges)| (key, edges.clone())),
    )
}

/// Exact recovery of PnP units from the module's mm output. The module writes
/// `units_to_mm(unit)` into an `f32`; at the fixture's ~300 mm extent that
/// quantisation is ~0.15 units, well inside the 0.5 needed for exact rounding.
fn mm_to_units(mm: f32) -> i64 {
    let scaled = f64::from(mm) * UNITS_PER_MM;
    scaled.round() as i64
}

/// Assert full plan identity: key set (including ordered `variant_chain`), edge
/// **multiset** with multiplicity, and the derived grounding/root witness set.
fn compare_populations(
    case_id: &str,
    expected: &Population,
    actual: &Population,
) -> Result<(), Mismatch> {
    if expected.is_empty() {
        return Err(mismatch(
            case_id,
            None,
            "baseline population is empty; an empty population is never equivalent",
        ));
    }
    if actual.is_empty() {
        return Err(mismatch(
            case_id,
            None,
            "candidate population is empty; an empty population is never equivalent",
        ));
    }

    for (key, expected_entry) in expected {
        let Some(actual_entry) = actual.get(key) else {
            return Err(mismatch(case_id, Some(key), "missing entry"));
        };
        let expected_witnesses = witness_set(&expected_entry.edges);
        let actual_witnesses = witness_set(&actual_entry.edges);
        if expected_witnesses != actual_witnesses {
            return Err(mismatch(
                case_id,
                Some(key),
                format!(
                    "grounding/root witness sets differ ({} vs {} witnesses)",
                    expected_witnesses.len(),
                    actual_witnesses.len()
                ),
            ));
        }
        for (edge, expected_count) in &expected_entry.edges {
            match actual_entry.edges.get(edge) {
                Some(actual_count) if actual_count == expected_count => {}
                Some(actual_count) => {
                    return Err(mismatch(
                        case_id,
                        Some(key),
                        format!(
                            "edge ({}, {}, {}, {}) multiplicity {expected_count} vs {actual_count}",
                            edge.0, edge.1, edge.2, edge.3
                        ),
                    ))
                }
                None => {
                    return Err(mismatch(
                        case_id,
                        Some(key),
                        format!(
                            "missing edge ({}, {}, {}, {}) with multiplicity {expected_count}",
                            edge.0, edge.1, edge.2, edge.3
                        ),
                    ))
                }
            }
        }
        if let Some(edge) = actual_entry
            .edges
            .keys()
            .find(|edge| !expected_entry.edges.contains_key(*edge))
        {
            return Err(mismatch(
                case_id,
                Some(key),
                format!(
                    "unexpected edge ({}, {}, {}, {})",
                    edge.0, edge.1, edge.2, edge.3
                ),
            ));
        }
    }
    for key in actual.keys() {
        if !expected.contains_key(key) {
            return Err(mismatch(case_id, Some(key), "unexpected entry"));
        }
    }
    Ok(())
}

/// Deliberately weak reference oracle: key set plus per-key edge totals. Used
/// only as a control to prove [`compare_populations`] is strictly stronger.
fn compare_lengths_only(
    case_id: &str,
    expected: &Population,
    actual: &Population,
) -> Result<(), Mismatch> {
    if expected.is_empty() || actual.is_empty() {
        return Err(mismatch(case_id, None, "empty population"));
    }
    if expected.keys().ne(actual.keys()) {
        return Err(mismatch(case_id, None, "key sets differ"));
    }
    for (key, expected_entry) in expected {
        let expected_total: u64 = expected_entry.edges.values().sum();
        let actual_total: u64 = actual[key].edges.values().sum();
        if expected_total != actual_total {
            return Err(mismatch(
                case_id,
                Some(key),
                format!("segment totals {expected_total} vs {actual_total}"),
            ));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Case material (the recorder's documented derivation, scale pinned by the
// committed baseline's `derivation` block)
// ---------------------------------------------------------------------------

/// One recorded case's geometry, decoded from the fixture's `a_source`.
struct CaseGeometry {
    case_id: String,
    layer_height_mm: f64,
    /// Layers in recorded order: print Z of the layer top and its height.
    layers: Vec<(f64, f64)>,
    regions: Vec<RegionGeometry>,
}

struct RegionGeometry {
    region_id: u64,
    object_id: String,
    /// Aligned with `CaseGeometry::layers`; `None` when the region is absent on
    /// that layer.
    footprints: Vec<Option<Vec<Rect>>>,
}

/// One recorded surface: closed contour plus holes, in PnP units.
struct Surface {
    contour: Vec<[i64; 2]>,
    holes: Vec<Vec<[i64; 2]>>,
}

/// An axis-aligned rectangle in PnP units.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Rect {
    x0: i64,
    y0: i64,
    x1: i64,
    y1: i64,
}

impl CaseGeometry {
    fn parse(case_id: &str, source: &Value, scale: f64) -> Self {
        let object_id = source
            .get("object_id")
            .and_then(Value::as_str)
            .unwrap_or_else(|| panic!("case `{case_id}`: a_source has no object_id"))
            .to_string();
        let object_config = source
            .get("object_config")
            .unwrap_or_else(|| panic!("case `{case_id}`: a_source has no object_config"));
        let layer_height_mm = object_config
            .get("layer_height_mm")
            .and_then(Value::as_f64)
            .unwrap_or_else(|| panic!("case `{case_id}`: object_config has no layer_height_mm"));
        let layer_values = source
            .get("layers")
            .and_then(Value::as_array)
            .unwrap_or_else(|| panic!("case `{case_id}`: a_source has no layers"));

        let mut layers: Vec<(f64, f64)> = Vec::with_capacity(layer_values.len());
        let mut regions: BTreeMap<u64, RegionGeometry> = BTreeMap::new();

        for layer in layer_values {
            let top_z = layer
                .get("z_mm")
                .and_then(Value::as_f64)
                .unwrap_or_else(|| panic!("case `{case_id}`: a layer has no z_mm"));
            let height = layer
                .get("effective_layer_height_mm")
                .and_then(Value::as_f64)
                .unwrap_or(layer_height_mm);
            let height = if height > 0.0 {
                height
            } else {
                layer_height_mm
            };
            layers.push((top_z, height));
            let layer_slot = layers.len() - 1;

            let layer_regions = layer
                .get("regions")
                .and_then(Value::as_array)
                .unwrap_or_else(|| panic!("case `{case_id}`: a layer has no regions"));
            for region in layer_regions {
                let identity = region
                    .get("identity")
                    .unwrap_or_else(|| panic!("case `{case_id}`: a region has no identity"));
                let region_id = identity
                    .get("region_id")
                    .and_then(Value::as_u64)
                    .unwrap_or_else(|| {
                        panic!("case `{case_id}`: a region identity has no region_id")
                    });
                let region_object = identity
                    .get("object_id")
                    .and_then(Value::as_str)
                    .unwrap_or(&object_id)
                    .to_string();
                let chain = identity
                    .get("variant_chain")
                    .and_then(Value::as_array)
                    .map(Vec::len)
                    .unwrap_or(0);
                assert_eq!(
                    chain, 0,
                    "case `{case_id}`: region {region_id} carries a paint variant chain; the \
                     recorder only materialises chain-less regions and this harness must use the \
                     same case material"
                );

                let mut surfaces = Vec::new();
                for surface in region
                    .get("surfaces")
                    .and_then(Value::as_array)
                    .unwrap_or_else(|| panic!("case `{case_id}`: a region has no surfaces"))
                {
                    let geometry = surface
                        .get("geometry")
                        .unwrap_or_else(|| panic!("case `{case_id}`: a surface has no geometry"));
                    surfaces.push(parse_surface(case_id, geometry, scale));
                }
                let cells = footprint_cells(&surfaces);

                let entry = regions.entry(region_id).or_insert_with(|| RegionGeometry {
                    region_id,
                    object_id: format!("{region_object}-region-{region_id}"),
                    footprints: vec![None; layers.len().max(layer_slot + 1)],
                });
                if entry.footprints.len() <= layer_slot {
                    entry.footprints.resize(layer_slot + 1, None);
                }
                entry.footprints[layer_slot] = Some(cells);
            }
        }

        for region in regions.values_mut() {
            region.footprints.resize(layers.len(), None);
        }
        assert!(
            !regions.is_empty(),
            "case `{case_id}`: the fixture records no region geometry"
        );

        Self {
            case_id: case_id.to_string(),
            layer_height_mm,
            layers,
            regions: regions.into_values().collect(),
        }
    }

    fn top_z_mm(&self) -> f64 {
        self.layers
            .last()
            .map(|(top, height)| top + height * 0.25)
            .unwrap_or(self.layer_height_mm)
    }

    /// Band bounds for layer `index`; every boundary sits a quarter layer-height
    /// above a print Z, so no slice plane coincides with a band boundary.
    fn band(&self, index: usize) -> (f64, f64) {
        let (top, height) = self.layers[index];
        let lo = if index == 0 {
            0.0
        } else {
            let (previous_top, previous_height) = self.layers[index - 1];
            previous_top + previous_height * 0.25
        };
        (lo.max(0.0), top + height * 0.25)
    }

    fn mesh(&self) -> MeshIR {
        let top = self.top_z_mm() as f32;
        let mut painted = false;
        let objects: Vec<ObjectMesh> = self
            .regions
            .iter()
            .map(|region| {
                let mut vertices: Vec<Point3> = Vec::new();
                let mut indices: Vec<u32> = Vec::new();
                let mut paint_facet: Option<usize> = None;
                let top_layer = region.footprints.iter().rposition(Option::is_some);
                for (index, footprint) in region.footprints.iter().enumerate() {
                    let Some(cells) = footprint else { continue };
                    let (lo, hi) = self.band(index);
                    for cell in cells {
                        let box_index = vertices.len() / 8;
                        push_box(&mut vertices, &mut indices, cell, lo as f32, hi as f32);
                        if paint_facet.is_none() && Some(index) == top_layer {
                            paint_facet = Some(box_index * 12 + 2);
                        }
                    }
                }
                assert!(
                    !indices.is_empty(),
                    "case `{}`: region {} extruded to an empty mesh",
                    self.case_id,
                    region.region_id
                );
                let paint_data = if !painted && self.case_id == PAINTED_CASE {
                    painted = true;
                    let facet = paint_facet.unwrap_or_else(|| {
                        panic!(
                            "case `{}`: the synthetic paint patch has no top-face triangle to target",
                            self.case_id
                        )
                    });
                    let mut facet_values = vec![None; indices.len() / 3];
                    facet_values[facet] = Some(PaintValue::ToolIndex(PAINT_TOOL_INDEX));
                    Some(FacetPaintData {
                        layers: vec![PaintLayer {
                            semantic: PaintSemantic::Material,
                            facet_values,
                            strokes: Vec::new(),
                        }],
                    })
                } else {
                    None
                };
                // exhaustive: the case material must state every `ObjectMesh` field (identity, geometry, placement, paint) so a new field is a compile error here rather than a silently defaulted mesh property.
                ObjectMesh {
                    id: region.object_id.clone(),
                    mesh: IndexedTriangleSet { vertices, indices },
                    transform: identity_transform(),
                    config: ObjectConfig {
                        data: HashMap::new(),
                    },
                    modifier_volumes: Vec::new(),
                    paint_data,
                    world_z_extent: Some((0.0, top)),
                }
            })
            .collect();
        MeshIR {
            objects,
            ..Default::default()
        }
    }
}

fn parse_surface(case_id: &str, geometry: &Value, scale: f64) -> Surface {
    let contour = geometry
        .get("contour")
        .and_then(|value| value.get("points"))
        .and_then(Value::as_array)
        .unwrap_or_else(|| panic!("case `{case_id}`: a surface contour has no points"));
    let holes = geometry
        .get("holes")
        .and_then(Value::as_array)
        .map(|holes| {
            holes
                .iter()
                .map(|hole| {
                    hole.get("points")
                        .and_then(Value::as_array)
                        .map(|points| parse_points(points, scale))
                        .unwrap_or_else(|| panic!("case `{case_id}`: a surface hole has no points"))
                })
                .collect()
        })
        .unwrap_or_default();
    Surface {
        contour: parse_points(contour, scale),
        holes,
    }
}

fn parse_points(points: &[Value], scale: f64) -> Vec<[i64; 2]> {
    points
        .iter()
        .map(|point| {
            let x = point
                .get("x")
                .and_then(Value::as_f64)
                .expect("fixture point has an x coordinate");
            let y = point
                .get("y")
                .and_then(Value::as_f64)
                .expect("fixture point has a y coordinate");
            [to_pnp_units(x * scale), to_pnp_units(y * scale)]
        })
        .collect()
}

fn to_pnp_units(canonical_units: f64) -> i64 {
    let scaled = canonical_units / CANONICAL_UNITS_PER_PNP_UNIT;
    if scaled >= 0.0 {
        (scaled + 0.5).floor() as i64
    } else {
        (scaled - 0.5).ceil() as i64
    }
}

/// Exact rectilinear footprint of a union-of-surfaces stack, in PnP units.
fn footprint_cells(surfaces: &[Surface]) -> Vec<Rect> {
    let mut xs: BTreeSet<i64> = BTreeSet::new();
    let mut ys: BTreeSet<i64> = BTreeSet::new();
    for surface in surfaces {
        for [x, y] in &surface.contour {
            xs.insert(*x);
            ys.insert(*y);
        }
        for hole in &surface.holes {
            for [x, y] in hole {
                xs.insert(*x);
                ys.insert(*y);
            }
        }
    }
    let xs: Vec<i64> = xs.into_iter().collect();
    let ys: Vec<i64> = ys.into_iter().collect();
    if xs.len() < 2 || ys.len() < 2 {
        return Vec::new();
    }

    let mut open: Vec<Rect> = Vec::new();
    for row in 0..ys.len() - 1 {
        let y0 = ys[row];
        let y1 = ys[row + 1];
        let center_y = (y0 as f64 + y1 as f64) / 2.0;
        let mut run_start: Option<usize> = None;
        for column in 0..xs.len() {
            let covered = column + 1 < xs.len() && {
                let center_x = (xs[column] as f64 + xs[column + 1] as f64) / 2.0;
                surfaces.iter().any(|surface| {
                    point_in_polygon(&surface.contour, center_x, center_y)
                        && !surface
                            .holes
                            .iter()
                            .any(|hole| point_in_polygon(hole, center_x, center_y))
                })
            };
            match (covered, run_start) {
                (true, None) => run_start = Some(column),
                (false, Some(start)) => {
                    extend_run(&mut open, xs[start], xs[column], y0, y1);
                    run_start = None;
                }
                _ => {}
            }
        }
    }
    open
}

fn extend_run(open: &mut Vec<Rect>, x0: i64, x1: i64, y0: i64, y1: i64) {
    if let Some(existing) = open
        .iter_mut()
        .find(|rect| rect.x0 == x0 && rect.x1 == x1 && rect.y1 == y0)
    {
        existing.y1 = y1;
    } else {
        open.push(Rect { x0, y0, x1, y1 });
    }
}

fn point_in_polygon(points: &[[i64; 2]], px: f64, py: f64) -> bool {
    let mut inside = false;
    let count = points.len();
    if count < 3 {
        return false;
    }
    for index in 0..count {
        let [x1, y1] = points[index];
        let [x2, y2] = points[(index + 1) % count];
        let (y1, y2) = (y1 as f64, y2 as f64);
        if (y1 > py) != (y2 > py) {
            let t = (py - y1) / (y2 - y1);
            let crossing_x = x1 as f64 + t * (x2 as f64 - x1 as f64);
            if crossing_x > px {
                inside = !inside;
            }
        }
    }
    inside
}

fn push_box(vertices: &mut Vec<Point3>, indices: &mut Vec<u32>, rect: &Rect, z0: f32, z1: f32) {
    // Historical case material: `Rect` is in PnP units (1 unit = 100 nm) and the
    // value is written straight into the millimetre-valued `Point3`, so the
    // planned domain is 100x the canonical region. The recorded baseline and the
    // tests share this derivation; `derivation.mesh` states it explicitly. Do
    // not "fix" it here without re-recording and re-checking the per-layer tap
    // contract.
    let (x0, y0, x1, y1) = (
        rect.x0 as f32,
        rect.y0 as f32,
        rect.x1 as f32,
        rect.y1 as f32,
    );
    let base = vertices.len() as u32;
    vertices.extend([
        Point3 {
            x: x0,
            y: y0,
            z: z0,
        },
        Point3 {
            x: x1,
            y: y0,
            z: z0,
        },
        Point3 {
            x: x1,
            y: y1,
            z: z0,
        },
        Point3 {
            x: x0,
            y: y1,
            z: z0,
        },
        Point3 {
            x: x0,
            y: y0,
            z: z1,
        },
        Point3 {
            x: x1,
            y: y0,
            z: z1,
        },
        Point3 {
            x: x1,
            y: y1,
            z: z1,
        },
        Point3 {
            x: x0,
            y: y1,
            z: z1,
        },
    ]);
    let faces: [[u32; 3]; 12] = [
        [0, 2, 1],
        [0, 3, 2],
        [4, 5, 6],
        [4, 6, 7],
        [0, 1, 5],
        [0, 5, 4],
        [3, 7, 6],
        [3, 6, 2],
        [0, 4, 7],
        [0, 7, 3],
        [1, 6, 5],
        [1, 2, 6],
    ];
    for face in faces {
        indices.extend(face.iter().map(|index| base + index));
    }
}

fn identity_transform() -> Transform3d {
    Transform3d {
        matrix: [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ],
    }
}

/// The committed case material: the oracle's own mesh derivation, at the scale
/// the baseline's `derivation` block pins.
fn case_mesh(root: &Path, baseline: &Baseline, case: &BaselineCase) -> MeshIR {
    assert_eq!(
        baseline.case_source, GATE_FIXTURE_REL,
        "the committed oracle must be derived from the row-07 case material"
    );
    let inputs_text = read_text(&root.join(&baseline.case_source).join("inputs.json"))
        .unwrap_or_else(|error| panic!("{error}"));
    let inputs: Value = serde_json::from_str(&inputs_text)
        .unwrap_or_else(|error| panic!("fixture inputs.json is not valid JSON: {error}"));
    let source = inputs
        .get("cases")
        .and_then(Value::as_array)
        .and_then(|cases| {
            cases.iter().find(|candidate| {
                candidate.get("case_id").and_then(Value::as_str) == Some(case.case_id.as_str())
            })
        })
        .and_then(|candidate| candidate.get("a_source"))
        .unwrap_or_else(|| {
            panic!(
                "fixture inputs.json has no a_source for baseline case '{}'",
                case.case_id
            )
        });
    CaseGeometry::parse(&case.case_id, source, baseline.geometry_scale).mesh()
}

// ---------------------------------------------------------------------------
// Real drivers
// ---------------------------------------------------------------------------

#[derive(Default)]
struct Trace {
    records: Mutex<Vec<PreparationLifecycleRecord>>,
}

impl Trace {
    fn records(&self) -> Vec<PreparationLifecycleRecord> {
        self.records
            .lock()
            .expect("lifecycle trace poisoned")
            .clone()
    }

    fn count(
        &self,
        module_id: &str,
        phase: PreparationLifecyclePhase,
        status: PreparationLifecycleStatus,
    ) -> usize {
        self.records()
            .iter()
            .filter(|record| {
                record.module_id.as_str() == module_id
                    && record.lifecycle_phase == phase
                    && record.status == status
            })
            .count()
    }

    fn failures(&self, module_id: &str) -> Vec<PreparationLifecycleRecord> {
        self.records()
            .iter()
            .filter(|record| {
                record.module_id.as_str() == module_id
                    && record.status == PreparationLifecycleStatus::Failure
            })
            .cloned()
            .collect()
    }

    fn max_retained_after_disposal(&self) -> u64 {
        self.records()
            .iter()
            .filter(|record| record.status == PreparationLifecycleStatus::Disposed)
            .map(|record| record.retained_bytes)
            .max()
            .unwrap_or(0)
    }
}

impl PreparationLifecycleObserver for Trace {
    fn on_preparation_lifecycle(&self, record: &PreparationLifecycleRecord) {
        self.records
            .lock()
            .expect("lifecycle trace poisoned")
            .push(record.clone());
    }
}

/// Records lifecycle events **and** requests cancellation the moment the
/// module's preparation reports `Ready` while holding pieces.
///
/// AC-3 requires cleanup accounting after cancellation. A flag set before the
/// run starts never allocates a plan, so it proves nothing about disposal; this
/// observer raises the flag only after publication has committed bytes.
struct CancelAfterPublication {
    trace: Arc<Trace>,
    flag: Arc<std::sync::atomic::AtomicBool>,
}

impl PreparationLifecycleObserver for CancelAfterPublication {
    fn on_preparation_lifecycle(&self, record: &PreparationLifecycleRecord) {
        if record.module_id.as_str() == MODULE_ID
            && record.lifecycle_phase == PreparationLifecyclePhase::Preparation
            && record.status == PreparationLifecycleStatus::Ready
            && !record.pieces.is_empty()
        {
            self.flag.store(true, std::sync::atomic::Ordering::SeqCst);
        }
        PreparationLifecycleObserver::on_preparation_lifecycle(&*self.trace, record);
    }
}

fn extensions(
    views: Vec<PreparationViewRequest>,
    prepared_native: Vec<(String, NativePreparedStageEntry, PreparationExportSchema)>,
    trace: Arc<Trace>,
) -> SliceRuntimeExtensions {
    SliceRuntimeExtensions::new(Vec::new(), prepared_native, Vec::new(), Some(trace))
        .with_preparation_views(views)
}

fn case_options(
    root: &Path,
    mesh: MeshIR,
    case: &BaselineCase,
    integrated: bool,
) -> SliceRunOptions {
    let module_dir = root.join("modules").join("core-modules");
    assert!(
        module_dir.is_dir(),
        "core module root is absent: {}",
        module_dir.display()
    );
    case_options_with_dirs(root, mesh, case, integrated, vec![module_dir])
}

/// The native arm's search roots: every core module except the lightning guest.
///
/// Integrated registrations are the loader's last tier and lose to any on-disk
/// manifest with the same id, so leaving `lightning-infill` in the roots would
/// silently run the WASM artifact instead of the module's native prepared pair.
fn native_module_dirs(root: &Path) -> Vec<PathBuf> {
    let core = root.join("modules").join("core-modules");
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(&core)
        .unwrap_or_else(|error| panic!("read {}: {error}", core.display()))
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .filter(|path| path.file_name().and_then(|name| name.to_str()) != Some("lightning-infill"))
        .collect();
    dirs.sort();
    assert!(
        dirs.len() > 1,
        "native arm needs the other core modules as guest roots"
    );
    dirs
}

fn case_options_with_dirs(
    root: &Path,
    mesh: MeshIR,
    case: &BaselineCase,
    integrated: bool,
    module_dirs: Vec<PathBuf>,
) -> SliceRunOptions {
    case_options_with_config(root, mesh, case, integrated, module_dirs, &[])
}

/// `case_options_with_dirs` plus explicit config overrides.
///
/// Used by the diagnostic runs: the module's advanced `prepared_plan_diagnostic`
/// key is the only real-adapter trigger for the failure paths D6 documents.
fn case_options_with_config(
    root: &Path,
    mesh: MeshIR,
    case: &BaselineCase,
    integrated: bool,
    module_dirs: Vec<PathBuf>,
    extra: &[(&str, ConfigValue)],
) -> SliceRunOptions {
    let _ = root;
    let mut config_overrides = HashMap::new();
    config_overrides.insert(
        "sparse_fill_holder".to_owned(),
        ConfigValue::String(case.config.sparse_fill_holder.clone()),
    );
    config_overrides.insert(
        "infill_density".to_owned(),
        ConfigValue::Float(case.config.infill_density),
    );
    config_overrides.insert(
        "layer_height".to_owned(),
        ConfigValue::Float(case.config.layer_height),
    );
    config_overrides.insert(
        "infill_angle".to_owned(),
        ConfigValue::Float(case.config.infill_angle),
    );
    config_overrides.insert(
        "line_width".to_owned(),
        ConfigValue::Float(case.config.line_width),
    );
    for (key, value) in extra {
        config_overrides.insert((*key).to_owned(), value.clone());
    }
    SliceRunOptions {
        mesh: Arc::new(mesh),
        model_label: case.case_id.clone(),
        no_default_module_paths: true,
        no_integrated_modules: !integrated,
        module_dirs,
        config_overrides,
        ..Default::default()
    }
}

/// One case's ordinary module roots plus a `prepared_plan_diagnostic` mode.
fn diagnostic_options(
    root: &Path,
    baseline: &Baseline,
    case: &BaselineCase,
    mode: &str,
) -> SliceRunOptions {
    case_options_with_config(
        root,
        case_input(root, baseline, case),
        case,
        false,
        vec![root.join("modules").join("core-modules")],
        &[(
            "prepared_plan_diagnostic",
            ConfigValue::String(mode.to_owned()),
        )],
    )
}

/// Run one real consumer-side diagnostic and return the layer-stage failure
/// text **and** the lifecycle trace.
///
/// The mode affects publication only, so preparation succeeds; the ordinary
/// `Layer::Infill` stage then fails when it reads its own layer's piece. A
/// failure that is not a slice (layer-stage) failure would prove the wrong
/// boundary, so it panics instead of being normalized. The trace is returned
/// because a consumer failure happens *after* allocation, so its cleanup is an
/// assertion rather than an assumption.
fn consumer_failure(
    root: &Path,
    baseline: &Baseline,
    case: &BaselineCase,
    mode: &str,
) -> (String, Arc<Trace>) {
    let options = diagnostic_options(root, baseline, case, mode);
    let trace = Arc::new(Trace::default());
    let error = match run_visual(
        options,
        Vec::new(),
        vec![INFILL_TAP.to_owned()],
        layer_indices(case),
        Arc::clone(&trace),
    ) {
        Ok(capture) => panic!(
            "case `{}`, mode `{mode}`: the run completed with {} owner(s); the diagnostic \
             piece never reached a consumer",
            case.case_id,
            capture.metadata().owners.len()
        ),
        Err(error) => error,
    };
    let text = match error {
        PreparationVisualCaptureError::Slice(inner) => inner.0,
        other => panic!(
            "case `{}`, mode `{mode}`: the failure must be a real layer-stage failure, got {other:?}",
            case.case_id
        ),
    };
    (text, trace)
}

/// A consumer failure occurs after a fully allocated plan, so cleanup must
/// dispose real bytes.
///
/// `max_retained_after_disposal()` returns zero when *no* disposal record
/// exists, so this asserts the population first: allocation observed, at least
/// one disposal, and every disposal at zero. Without that ordering a
/// failure-only retention regression would read as clean.
fn assert_failure_cleanup(trace: &Trace, case_id: &str, mode: &str) {
    let records = trace.records();
    let ready = records
        .iter()
        .find(|record| {
            record.module_id.as_str() == MODULE_ID
                && record.lifecycle_phase == PreparationLifecyclePhase::Preparation
                && record.status == PreparationLifecycleStatus::Ready
                && !record.pieces.is_empty()
        })
        .unwrap_or_else(|| {
            panic!(
                "case `{case_id}`, mode `{mode}`: no Ready preparation holding pieces was \
                 observed, so the failure never exercised an allocated plan"
            )
        });
    assert!(
        ready.retained_bytes > 0,
        "case `{case_id}`, mode `{mode}`: the failed consumption must have held bytes"
    );
    let disposals: Vec<_> = records
        .iter()
        .filter(|record| {
            record.module_id.as_str() == MODULE_ID
                && record.status == PreparationLifecycleStatus::Disposed
        })
        .collect();
    assert!(
        !disposals.is_empty(),
        "case `{case_id}`, mode `{mode}`: a failed consumption must record actual disposal, \
         not merely an absent one"
    );
    assert!(
        disposals.iter().all(|record| record.retained_bytes == 0),
        "case `{case_id}`, mode `{mode}`: a failed consumption releases every piece"
    );
}

/// The recorded case's mesh plus its committed print config.
fn case_input(root: &Path, baseline: &Baseline, case: &BaselineCase) -> MeshIR {
    case_mesh(root, baseline, case)
}

fn layer_indices(case: &BaselineCase) -> Vec<u32> {
    (0..case.layer_count as u32).collect()
}

fn run_visual(
    options: SliceRunOptions,
    views: Vec<PreparationViewRequest>,
    taps: Vec<String>,
    layers: Vec<u32>,
    trace: Arc<Trace>,
) -> Result<PreparationVisualCapture, PreparationVisualCaptureError> {
    run_slice_preparation_visual_capture(
        options,
        extensions(views, Vec::new(), trace),
        layers,
        taps,
    )
}

/// Run the same capture with the integrated modules' **real prepared pairs**
/// supplied to the runtime. This is the native arm: the module's own generated
/// `NativePreparedStageEntry`/`PreparationExportSchema`, never a hand-built
/// surrogate.
fn run_visual_native(
    options: SliceRunOptions,
    taps: Vec<String>,
    layers: Vec<u32>,
    trace: Arc<Trace>,
) -> Result<PreparationVisualCapture, PreparationVisualCaptureError> {
    run_slice_preparation_visual_capture(
        options,
        extensions(
            Vec::new(),
            slicer_integrated_modules::prepared_native_entries(),
            trace,
        ),
        layers,
        taps,
    )
}

fn captured_infill(capture: &PreparationVisualCapture) -> BTreeMap<u32, InfillIR> {
    let mut by_layer = BTreeMap::new();
    for stage in capture.captures() {
        if let CapturedIr::Infill(infill) = &stage.ir {
            assert!(
                by_layer
                    .insert(infill.global_layer_index, infill.clone())
                    .is_none(),
                "two Layer::Infill captures for layer {}",
                infill.global_layer_index
            );
        }
    }
    by_layer
}

fn captured_slices(capture: &PreparationVisualCapture) -> BTreeMap<u32, SliceIR> {
    let mut by_layer = BTreeMap::new();
    for stage in capture.captures() {
        if let CapturedIr::Slice(slice) = &stage.ir {
            assert!(
                by_layer
                    .insert(slice.global_layer_index, slice.clone())
                    .is_none(),
                "two Layer::Slice captures for layer {}",
                slice.global_layer_index
            );
        }
    }
    by_layer
}

/// The recorder's exact input-digest surface: the committed `SliceIR` identities
/// and polygons plus the five planner scalars **as supplied to the run**.
///
/// The scalars are read back from the options the caller actually handed the
/// pipeline (`SliceRunOptions::config_overrides`), never from the baseline
/// record. A plumbing defect that drops or rewrites an override therefore moves
/// this digest, which is what lets AC-2 report an input defect rather than an
/// algorithm difference. `module_plan_matches_recorded_baseline` carries the
/// control that perturbs one supplied scalar and requires the digest to move.
/// It cannot see the scalars the module resolves from those overrides; that
/// would need the module to publish them (`docs/07` TASK-577 records the limit).
fn inputs_digest(config: &HashMap<String, ConfigValue>, slices: &BTreeMap<u32, SliceIR>) -> String {
    let float = |key: &str| -> f64 {
        match config.get(key) {
            Some(ConfigValue::Float(value)) => *value,
            Some(other) => panic!("supplied config override `{key}` must be a Float: {other:?}"),
            None => panic!("supplied config override `{key}` is missing"),
        }
    };
    let sparse_fill_holder = match config.get("sparse_fill_holder") {
        Some(ConfigValue::String(value)) => value.clone(),
        Some(other) => {
            panic!("supplied config override `sparse_fill_holder` must be a String: {other:?}")
        }
        None => panic!("supplied config override `sparse_fill_holder` is missing"),
    };
    let mut layers = Vec::new();
    for slice in slices.values() {
        let regions: Vec<Value> = slice
            .regions
            .iter()
            .map(|region| {
                json!({
                    "object_id": region.object_id,
                    "region_id": region.region_id,
                    "variant_chain": region
                        .variant_chain
                        .iter()
                        .map(|(semantic, value)| json!({
                            "semantic": semantic,
                            "value": value,
                        }))
                        .collect::<Vec<Value>>(),
                    "polygons": region
                        .polygons
                        .iter()
                        .map(|polygon| json!({
                            "contour": polygon.contour.points.iter().map(|p| [p.x, p.y]).collect::<Vec<_>>(),
                            "holes": polygon
                                .holes
                                .iter()
                                .map(|hole| hole.points.iter().map(|p| [p.x, p.y]).collect::<Vec<_>>())
                                .collect::<Vec<Vec<[i64; 2]>>>(),
                        }))
                        .collect::<Vec<Value>>(),
                })
            })
            .collect();
        layers.push(json!({
            "global_layer_index": slice.global_layer_index,
            "z": f64::from(slice.z),
            "regions": regions,
        }));
    }
    sha256_canonical(&json!({
        "config": {
            "sparse_fill_holder": sparse_fill_holder,
            "infill_density": float("infill_density"),
            "layer_height": float("layer_height"),
            "infill_angle": float("infill_angle"),
            "line_width": float("line_width"),
        },
        "layers": layers,
    }))
}

/// Assert the owner published one Ready preparation with pieces, and return its
/// capture. The module declares the capability (Step 7b), so a missing owner is a
/// real failure rather than a skip.
fn assert_prepared_owner(capture: &PreparationCapture, case_id: &str) {
    assert_eq!(
        capture.owners.len(),
        1,
        "case `{case_id}`: expected exactly one eligible prepared owner for '{MODULE_ID}'. \
         Owners: {:?}",
        capture
            .owners
            .iter()
            .map(|owner| owner.module_id.as_str())
            .collect::<Vec<_>>()
    );
    let owner = &capture.owners[0];
    assert_eq!(owner.module_id.as_str(), MODULE_ID);
    assert!(
        !owner.pieces.is_empty(),
        "case `{case_id}`: a Ready owner must publish at least one private piece"
    );
    assert!(
        owner
            .pieces
            .iter()
            .all(|piece| piece.name.starts_with("lightning-layer-")),
        "case `{case_id}`: piece names must be the module's own `lightning-layer-*`: {:?}",
        owner.pieces
    );
    assert_eq!(
        capture.retained_bytes, 0,
        "case `{case_id}`: preparation-only capture must retain no piece bytes after disposal"
    );
}

/// Assert one acceptance arm reproduces the recorded baseline.
///
/// Both arms are compared to the baseline, never to each other: the WASM arm
/// stages the artifact with integrated modules disabled, the native arm runs the
/// module's generated prepared pair with them enabled. Because the ordinary stage
/// now reads its pieces (Step 8a), the infill-derived population below measures
/// the **module's** plan on both arms.
fn assert_arm_matches_baseline(
    case: &BaselineCase,
    capture: &PreparationVisualCapture,
    arm: &str,
    config: &HashMap<String, ConfigValue>,
) {
    let slices = captured_slices(capture);
    assert_eq!(
        slices.len() as u64,
        case.layer_count,
        "case `{}` ({arm}): the committed schedule must expose every recorded layer to the tap",
        case.case_id
    );
    let actual_inputs = inputs_digest(config, &slices);
    assert_eq!(
        actual_inputs, case.digests.inputs_sha256,
        "case `{}` ({arm}): input digest diverged first (recorded {}, actual {actual_inputs}); \
         this is an input-plumbing defect, not an algorithm difference",
        case.case_id, case.digests.inputs_sha256
    );

    assert_prepared_owner(&capture.metadata(), &case.case_id);

    let infill = captured_infill(capture);
    assert_eq!(
        infill.len() as u64,
        case.layer_count,
        "case `{}` ({arm}): every committed layer must produce a Layer::Infill capture",
        case.case_id
    );
    let expected = population_from_plan_entries(case).expect("baseline population is unique");
    let actual = nonempty(
        infill
            .iter()
            .try_fold(Population::new(), |mut population, (layer, ir)| {
                let layer_population = population_from_infill(&case.case_id, *layer, ir)
                    .expect("the module's emitted region keys are unique");
                for (key, entry) in layer_population {
                    if population.insert(key.clone(), entry).is_some() {
                        panic!(
                            "case `{}` ({arm}): layer {layer} emitted a duplicated key {}",
                            case.case_id,
                            key.describe()
                        );
                    }
                }
                Ok::<Population, Mismatch>(population)
            })
            .expect("no duplicate keys"),
    );
    compare_populations(&case.case_id, &expected, &actual).unwrap_or_else(|error| {
        panic!(
            "case `{}` ({arm}): the module plan diverged from the recorded baseline: {error}",
            case.case_id
        )
    });
}

// ---------------------------------------------------------------------------
// AC-1
// ---------------------------------------------------------------------------

/// AC-1 — the committed oracle is deterministic **as an artifact**: strict
/// decode, nonempty per-case populations, cross-file agreement and recorded
/// digests that its own bytes and entries reproduce. Mutation controls prove
/// every rejection is real.
///
/// What this does *not* prove: the recorder's two-run byte-identity. That run
/// happened against the pre-migration producer and is not reproducible from
/// this tree — the recorder was deleted with the producer (implementation-plan
/// Step 14f, recorded limitation). The evidence for pre-deletion determinism is
/// the committed fixture plus its `derivation`/digests, not a re-runnable
/// capture.
#[test]
fn baseline_is_deterministic_and_strict() {
    let root = workspace_root();
    let (baseline, manifest, baseline_text) = load_baseline(&root)
        .unwrap_or_else(|error| panic!("committed lightning baseline is unusable: {error}"));

    assert!(
        !baseline.cases.is_empty(),
        "an empty case population is not a baseline"
    );
    assert_eq!(
        baseline.cases.len() as u64,
        manifest.fixture_case_count,
        "the manifest's case count must equal the decoded case population"
    );
    let mut input_digests: BTreeSet<&str> = BTreeSet::new();
    for case in &baseline.cases {
        assert!(
            case.plan_entries
                .iter()
                .all(|entry| !entry.edges.is_empty()),
            "case `{}`: every recorded plan entry must carry a nonempty edge multiset",
            case.case_id
        );
        assert!(
            case.callers
                .iter()
                .any(|caller| !caller.segments.is_empty()),
            "case `{}`: at least one caller must carry segments",
            case.case_id
        );
        assert!(
            input_digests.insert(case.digests.inputs_sha256.as_str()),
            "case `{}`: two cases must not share one input digest; distinct case material is the oracle's premise",
            case.case_id
        );
    }

    // Negative controls: each corruption must be rejected *for its own reason*.
    let absent = TempDir::new().expect("temp dir for the absent-baseline control");
    let absent_error = load_baseline(absent.path())
        .expect_err("an absent baseline must be an error, never an empty baseline or a skip");
    assert!(
        absent_error.contains("absent"),
        "absent baseline must name the absence: {absent_error}"
    );

    let mut unknown_key: Value = serde_json::from_str(&baseline_text).expect("baseline JSON");
    unknown_key["cases"][0]
        .as_object_mut()
        .expect("case object")
        .insert("unrecorded_key".to_owned(), json!(1));
    let error = decode_baseline(&unknown_key.to_string())
        .expect_err("an unknown key must be rejected, never ignored");
    assert!(error.contains("unrecorded_key"), "unknown key: {error}");

    let mut missing_case: Value = serde_json::from_str(&baseline_text).expect("baseline JSON");
    missing_case["cases"]
        .as_array_mut()
        .expect("cases array")
        .pop()
        .expect("at least one case");
    let decoded_missing = decode_baseline(&missing_case.to_string()).expect("still decodes");
    let error = validate_cases_against_manifest(&decoded_missing, &manifest)
        .expect_err("a missing case must be rejected");
    assert!(
        error.contains("do not match manifest case_ids"),
        "missing case must name the manifest disagreement: {error}"
    );

    let mut tampered_plan_digest: Value =
        serde_json::from_str(&baseline_text).expect("baseline JSON");
    let digest = tampered_plan_digest["cases"][0]["digests"]["plan_sha256"]
        .as_str()
        .expect("plan digest")
        .to_owned();
    let flipped = format!(
        "{}{}",
        if &digest[..1] == "0" { "1" } else { "0" },
        &digest[1..]
    );
    tampered_plan_digest["cases"][0]["digests"]["plan_sha256"] = json!(flipped);
    let tampered = decode_baseline(&tampered_plan_digest.to_string()).expect("still decodes");
    let error =
        validate_case(&tampered.cases[0]).expect_err("a tampered plan digest must be rejected");
    assert!(error.contains("plan_sha256"), "tampered digest: {error}");

    let mut tampered_edge: Value = serde_json::from_str(&baseline_text).expect("baseline JSON");
    let count = tampered_edge["cases"][0]["plan_entries"][0]["edges"][0][4]
        .as_u64()
        .expect("edge multiplicity");
    tampered_edge["cases"][0]["plan_entries"][0]["edges"][0][4] = json!(count + 1);
    let tampered = decode_baseline(&tampered_edge.to_string()).expect("still decodes");
    let error =
        validate_case(&tampered.cases[0]).expect_err("an edited edge multiset must be rejected");
    assert!(
        error.contains("edge_count") || error.contains("plan_sha256"),
        "edited edge: {error}"
    );

    let mut tampered_file = baseline_text.clone();
    tampered_file.push('\n');
    let error = validate_manifest_envelope(&root, &manifest, &tampered_file)
        .expect_err("bytes that do not match manifest.baseline_sha256 must be rejected");
    assert!(
        error.contains("tampered digest"),
        "tampered baseline bytes: {error}"
    );
}

// ---------------------------------------------------------------------------
// AC-2
// ---------------------------------------------------------------------------

/// AC-2 — each adapter independently reproduces the recorded plan population.
///
/// Both arms are real: the WASM arm stages the module artifact from
/// `modules/core-modules` with integrated modules disabled; the native arm
/// registers the module's generated prepared pair and enables integrated modules.
/// Both are compared to the baseline, never to each other. The input digest is
/// checked first, so a plumbing defect is reported as such rather than as an
/// algorithm difference.
#[test]
fn module_plan_matches_recorded_baseline() {
    let root = workspace_root();
    let (baseline, _manifest, _text) = load_baseline(&root)
        .unwrap_or_else(|error| panic!("committed lightning baseline is unusable: {error}"));

    for case in &baseline.cases {
        let wasm_trace = Arc::new(Trace::default());
        let options = case_options(&root, case_input(&root, &baseline, case), case, false);
        // The digest hashes the overrides this run is handed, not the baseline
        // record: a plumbing defect has to move it.
        let wasm_config = options.config_overrides.clone();
        let wasm_capture = run_visual(
            options,
            Vec::new(),
            vec![SLICE_TAP.to_owned(), INFILL_TAP.to_owned()],
            layer_indices(case),
            Arc::clone(&wasm_trace),
        )
        .unwrap_or_else(|error| panic!("case `{}`: the WASM arm failed: {error}", case.case_id));
        assert_arm_matches_baseline(case, &wasm_capture, "wasm", &wasm_config);

        // Control: a changed supplied scalar must move the digest. Deriving the
        // digest from `case.config` instead would leave this unchanged, which is
        // the self-referential-oracle failure AC-2 exists to prevent.
        let mut perturbed = wasm_config.clone();
        perturbed.insert(
            "infill_density".to_owned(),
            ConfigValue::Float(case.config.infill_density + 0.01),
        );
        assert_ne!(
            inputs_digest(&perturbed, &captured_slices(&wasm_capture)),
            case.digests.inputs_sha256,
            "case `{}`: the input digest ignored a changed supplied scalar",
            case.case_id
        );

        // Native arm: the module's real prepared pair. The lightning guest is
        // absent from the search roots so the integrated registration (and its
        // pair) is the only provider for this module id.
        let native_options = case_options_with_dirs(
            &root,
            case_input(&root, &baseline, case),
            case,
            true,
            native_module_dirs(&root),
        );
        let native_config = native_options.config_overrides.clone();
        let native_trace = Arc::new(Trace::default());
        let native_capture = run_visual_native(
            native_options,
            vec![SLICE_TAP.to_owned(), INFILL_TAP.to_owned()],
            layer_indices(case),
            Arc::clone(&native_trace),
        )
        .unwrap_or_else(|error| panic!("case `{}`: the native arm failed: {error}", case.case_id));
        assert_arm_matches_baseline(case, &native_capture, "native", &native_config);
    }
}

// ---------------------------------------------------------------------------
// AC-3
// ---------------------------------------------------------------------------

/// AC-3 — ordinary consumers emit the recorded per-caller segment multiset with
/// `SparseInfill` identity and truthful lifecycle accounting.
#[test]
fn fresh_consumers_emit_baseline_segments() {
    let root = workspace_root();
    let (baseline, _manifest, _text) = load_baseline(&root)
        .unwrap_or_else(|error| panic!("committed lightning baseline is unusable: {error}"));

    for case in &baseline.cases {
        let prep_trace = Arc::new(Trace::default());
        let options = case_options(&root, case_input(&root, &baseline, case), case, false);
        let prep_only = run_slice_preparation_capture(
            options,
            extensions(Vec::new(), Vec::new(), Arc::clone(&prep_trace)),
        )
        .unwrap_or_else(|error| {
            panic!(
                "case `{}`: the preparation-only path failed: {error}",
                case.case_id
            )
        });
        assert_prepared_owner(&prep_only, &case.case_id);
        assert_eq!(
            prep_trace.count(
                MODULE_ID,
                PreparationLifecyclePhase::Consumption,
                PreparationLifecycleStatus::Start
            ),
            0,
            "case `{}`: the preparation-only path constructs no consumer",
            case.case_id
        );
        assert_eq!(
            prep_only.transferred_bytes, 0,
            "case `{}`: the preparation-only path transfers no piece bytes",
            case.case_id
        );

        let trace = Arc::new(Trace::default());
        let options = case_options(&root, case_input(&root, &baseline, case), case, false);
        let capture = run_visual(
            options,
            Vec::new(),
            vec![INFILL_TAP.to_owned()],
            layer_indices(case),
            Arc::clone(&trace),
        )
        .unwrap_or_else(|error| {
            panic!("case `{}`: the consumer path failed: {error}", case.case_id)
        });
        assert_prepared_owner(&capture.metadata(), &case.case_id);
        let consumption_starts = trace.count(
            MODULE_ID,
            PreparationLifecyclePhase::Consumption,
            PreparationLifecycleStatus::Start,
        );
        assert!(
            consumption_starts > 0,
            "case `{}`: ordinary consumers must actually start",
            case.case_id
        );

        let infill = captured_infill(&capture);
        assert_eq!(
            infill.len() as u64,
            case.layer_count,
            "case `{}`: every committed layer must produce a Layer::Infill capture",
            case.case_id
        );
        let mut population = Population::new();
        let width = case.config.line_width;
        for (layer, ir) in &infill {
            let layer_population = population_from_infill(&case.case_id, *layer, ir)
                .expect("the module's emitted region keys are unique");
            for (key, entry) in layer_population {
                assert!(
                    population.insert(key.clone(), entry).is_none(),
                    "case `{}`: duplicated emitted key {}",
                    case.case_id,
                    key.describe()
                );
            }
            for region in &ir.regions {
                for path in &region.sparse_infill {
                    assert_eq!(
                        path.role,
                        ExtrusionRole::SparseInfill,
                        "case `{}`: emitted sparse paths keep the SparseInfill role",
                        case.case_id
                    );
                    assert!(
                        path.points.len() >= 2,
                        "case `{}`: an emitted sparse path carries a segment",
                        case.case_id
                    );
                    for point in &path.points {
                        assert!(
                            (f64::from(point.width) - width).abs() < 1.0e-6,
                            "case `{}`: emitted width {} does not match the recorded {}",
                            case.case_id,
                            point.width,
                            width
                        );
                    }
                }
                // Full identity, including the ordered chain: the painted case has
                // two callers sharing `(layer, object, region)`, so a three-field
                // lookup would silently compare the painted region to the
                // chain-less sibling — the exact identity collapse Step 1a extended
                // the baseline to catch.
                let emitted_chain: Vec<(String, String)> = region
                    .variant_chain
                    .iter()
                    .map(|(semantic, value)| {
                        (
                            semantic.clone(),
                            serde_json::to_string(value).expect("PaintValue serializes"),
                        )
                    })
                    .collect();
                let _recorded_caller = case
                    .callers
                    .iter()
                    .find(|caller| {
                        caller.key.global_layer_index == ir.global_layer_index as i32
                            && caller.key.object_id == region.object_id
                            && caller.key.region_id == region.region_id
                            && caller.key.variant_chain == emitted_chain
                    })
                    .unwrap_or_else(|| {
                        panic!(
                            "case `{}`: emitted region {}:{} with chain {emitted_chain:?} has no recorded caller",
                            case.case_id, region.object_id, region.region_id
                        )
                    });
            }
        }

        // A caller with zero segments is recorded — the baseline enumerates every
        // committed slice region — but carries no segment identity, and whether
        // such a region reaches the stage depends on the host's region partition.
        // Compare the nonempty subsets on both sides: a key that should emit
        // nothing but emits something is still an unexpected entry, and a key that
        // should emit something but emits nothing is still a missing entry.
        let expected =
            nonempty(population_from_callers(case).expect("caller population is unique"));
        let actual = nonempty(population);
        compare_populations(&case.case_id, &expected, &actual).unwrap_or_else(|error| {
            panic!(
                "case `{}`: emitted segments diverged from the recorded caller multiset: {error}",
                case.case_id
            )
        });
        assert_eq!(
            capture.metadata().retained_bytes,
            0,
            "case `{}`: no piece bytes survive disposal",
            case.case_id
        );
        assert!(
            capture.metadata().transferred_bytes > 0,
            "case `{}`: full consumption returns nonzero piece bytes",
            case.case_id
        );
        assert_eq!(
            trace.max_retained_after_disposal(),
            0,
            "case `{}`: every disposal reports zero retained bytes",
            case.case_id
        );

        // Cancellation after allocation: the flag is raised only once the module
        // reports a Ready preparation that holds pieces, so this leg exercises
        // disposal of an already-committed plan. A pre-set flag would cancel
        // before anything was allocated and prove nothing about cleanup.
        let cancel = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let mut cancel_options =
            case_options(&root, case_input(&root, &baseline, case), case, false);
        cancel_options.cancel_flag = Some(Arc::clone(&cancel));
        let cancel_trace = Arc::new(Trace::default());
        let cancel_observer = Arc::new(CancelAfterPublication {
            trace: Arc::clone(&cancel_trace),
            flag: Arc::clone(&cancel),
        });
        let error = run_slice_with_runtime_extensions(
            cancel_options,
            SliceRuntimeExtensions::new(Vec::new(), Vec::new(), Vec::new(), Some(cancel_observer)),
        )
        .expect_err("cancellation after publication must abort the print");
        assert!(
            cancel.load(std::sync::atomic::Ordering::SeqCst),
            "case `{}`: the cancellation trigger never fired, so this leg would be vacuous",
            case.case_id
        );
        assert!(
            format!("{error}").to_lowercase().contains("cancel"),
            "case `{}`: cancellation must be reported as cancellation: {error}",
            case.case_id
        );
        let allocation = cancel_trace
            .records()
            .into_iter()
            .find(|record| {
                record.module_id.as_str() == MODULE_ID
                    && record.lifecycle_phase == PreparationLifecyclePhase::Preparation
                    && record.status == PreparationLifecycleStatus::Ready
                    && !record.pieces.is_empty()
            })
            .unwrap_or_else(|| {
                panic!(
                    "case `{}`: no Ready preparation holding pieces was observed, so cancellation \
                     was never exercised against an allocated plan",
                    case.case_id
                )
            });
        assert!(
            allocation.retained_bytes > 0,
            "case `{}`: the cancelled preparation must have held bytes",
            case.case_id
        );
        let disposals: Vec<_> = cancel_trace
            .records()
            .into_iter()
            .filter(|record| record.status == PreparationLifecycleStatus::Disposed)
            .collect();
        assert!(
            !disposals.is_empty(),
            "case `{}`: cancellation must record actual disposal, not merely an absent one",
            case.case_id
        );
        assert!(
            disposals.iter().all(|record| record.retained_bytes == 0),
            "case `{}`: cancellation releases every piece",
            case.case_id
        );
        assert_eq!(
            cancel_trace.max_retained_after_disposal(),
            0,
            "case `{}`: cancellation releases every piece",
            case.case_id
        );
    }
}

// ---------------------------------------------------------------------------
// AC-4
// ---------------------------------------------------------------------------

/// AC-4 — the declared `planning_geometry` view publishes one typed
/// `lightning-plan` scene derived from the same plan, atomically with the pieces,
/// neutrally with respect to capture, and with a named row-06 failure.
#[test]
fn typed_projection_is_atomic_and_neutral() {
    let root = workspace_root();
    let (baseline, _manifest, _text) = load_baseline(&root)
        .unwrap_or_else(|error| panic!("committed lightning baseline is unusable: {error}"));
    let case = baseline
        .cases
        .iter()
        .find(|case| case.case_id == "hole_void_classic")
        .expect("the committed baseline must carry the hole_void_classic case");

    let trace = Arc::new(Trace::default());
    let options = case_options(&root, case_input(&root, &baseline, case), case, false);
    let capture = run_visual(
        options,
        vec![PreparationViewRequest {
            module_id: MODULE_ID.into(),
            view: VIEW.into(),
        }],
        Vec::new(),
        vec![0],
        Arc::clone(&trace),
    )
    .unwrap_or_else(|error| {
        panic!(
            "case `{}`: the declared planning_geometry view is not served: {error}",
            case.case_id
        )
    });

    assert_prepared_owner(&capture.metadata(), &case.case_id);
    assert!(
        !capture.captures().is_empty(),
        "case `{}`: the requested view must publish at least one typed capture",
        case.case_id
    );
    let mut scenes = Vec::new();
    for stage in capture.captures() {
        let CapturedIr::PreparationProjection(snapshot) = &stage.ir else {
            panic!(
                "case `{}`: planning_geometry must publish typed projections, got {:?}",
                case.case_id, stage.ir
            );
        };
        scenes.push(Arc::clone(snapshot));
    }
    assert!(
        scenes
            .windows(2)
            .all(|pair| Arc::ptr_eq(&pair[0], &pair[1])),
        "case `{}`: all rendered layers share one committed whole-print scene",
        case.case_id
    );
    let snapshot = &scenes[0];
    assert_eq!(snapshot.owner_module_id(), MODULE_ID);
    assert_eq!(snapshot.view(), VIEW);
    let scene = snapshot.scene();
    assert_eq!(scene.scene_id, "lightning-plan");
    assert_eq!(scene.view, VIEW);
    let classes: BTreeSet<&str> = scene.classes.iter().map(String::as_str).collect();
    assert_eq!(
        classes,
        ["grounding", "planning_domain", "tree_branch", "tree_root"]
            .into_iter()
            .collect::<BTreeSet<&str>>(),
        "case `{}`: the scene declares exactly the four planning classes",
        case.case_id
    );
    assert!(
        !scene.primitives.is_empty(),
        "case `{}`: the scene must carry primitives",
        case.case_id
    );
    let schedule: BTreeSet<u32> = capture.schedule().iter().map(|layer| layer.index).collect();
    assert!(
        !schedule.is_empty(),
        "case `{}`: the capture must carry the committed schedule",
        case.case_id
    );
    let mut ids = BTreeSet::new();
    let (mut domains, mut branches, mut roots, mut groundings, mut holes, mut labelled) =
        (0, 0, 0, 0, 0, 0);
    for primitive in &scene.primitives {
        assert!(
            !primitive.id.is_empty(),
            "case `{}`: primitive ids are nonempty and scene-local",
            case.case_id
        );
        assert!(
            ids.insert(primitive.id.clone()),
            "case `{}`: primitive ids are unique: {}",
            case.case_id,
            primitive.id
        );
        assert!(
            schedule.contains(&primitive.global_layer_index),
            "case `{}`: primitive {} names a real schedule layer",
            case.case_id,
            primitive.global_layer_index
        );
        if primitive.label.is_some() {
            labelled += 1;
        }
        match (&primitive.class[..], &primitive.geometry) {
            ("planning_domain", PreparationProjectionGeometry::Polygon { polygon, .. }) => {
                domains += 1;
                assert!(
                    polygon.contour.points.len() >= 3,
                    "case `{}`: a planning domain is a real polygon",
                    case.case_id
                );
                holes += polygon.holes.len();
            }
            ("tree_branch", PreparationProjectionGeometry::Polyline(points)) => {
                branches += 1;
                assert!(
                    points.len() >= 2,
                    "case `{}`: a tree branch is an open polyline with a segment",
                    case.case_id
                );
            }
            ("tree_root", PreparationProjectionGeometry::Point(_)) => roots += 1,
            ("grounding", PreparationProjectionGeometry::Point(_)) => groundings += 1,
            (class, geometry) => panic!(
                "case `{}`: class '{class}' carries the wrong geometry {geometry:?}",
                case.case_id
            ),
        }
    }
    assert!(
        domains > 0 && branches > 0 && roots > 0 && groundings > 0,
        "case `{}`: all four classes must carry a nonempty population (domains {domains}, \
         branches {branches}, roots {roots}, groundings {groundings})",
        case.case_id
    );
    assert!(
        labelled > 0,
        "case `{}`: source/checkpoint identity is carried in primitive labels",
        case.case_id
    );
    assert!(
        holes > 0,
        "case `{}`: the hole_void_classic domain preserves its hole (Polygon holes preserved)",
        case.case_id
    );

    // The scene's plan geometry is the same plan the pieces encode: compare the
    // per-layer edge union to the recorded population for the same layers. The
    // plan is print-wide, while the consumer filters per region (a region with
    // no sparse-fill area, or one that does not hold a sparse claim, emits
    // nothing), so the scene may carry branches the consumer did not emit.
    // Every emitted edge, however, must be present in the scene with the same
    // multiplicity: the consumer may filter plan branches, never invent them.
    let expected = population_from_plan_entries(case).expect("baseline population is unique");
    let mut expected_by_layer: BTreeMap<i32, BTreeMap<Edge, u64>> = BTreeMap::new();
    for (key, entry) in &expected {
        let layer = expected_by_layer.entry(key.global_layer_index).or_default();
        for (edge, count) in &entry.edges {
            *layer.entry(*edge).or_insert(0) += count;
        }
    }
    let mut scene_by_layer: BTreeMap<i32, BTreeMap<Edge, u64>> = BTreeMap::new();
    for primitive in &scene.primitives {
        let PreparationProjectionGeometry::Polyline(points) = &primitive.geometry else {
            continue;
        };
        let layer = scene_by_layer
            .entry(primitive.global_layer_index as i32)
            .or_default();
        for pair in points.windows(2) {
            *layer
                .entry((pair[0].x, pair[0].y, pair[1].x, pair[1].y))
                .or_insert(0) += 1;
        }
    }
    for (layer, emitted) in &expected_by_layer {
        let planned = scene_by_layer.get(layer).unwrap_or_else(|| {
            panic!(
                "case `{}`: layer {layer} emitted edges but the scene carries no plan branches",
                case.case_id
            )
        });
        for (edge, count) in emitted {
            assert_eq!(
                planned.get(edge),
                Some(count),
                "case `{}`: emitted edge ({}, {}, {}, {}) is missing from the scene plan",
                case.case_id,
                edge.0,
                edge.1,
                edge.2,
                edge.3
            );
        }
    }

    // Capture neutrality: the same real slice with and without the requested view
    // emits byte-identical G-code.
    let options = case_options(&root, case_input(&root, &baseline, case), case, false);
    let without = run_slice_with_runtime_extensions(
        options,
        extensions(Vec::new(), Vec::new(), Arc::new(Trace::default())),
    )
    .unwrap_or_else(|error| panic!("case `{}`: plain slice failed: {error}", case.case_id));
    let options = case_options(&root, case_input(&root, &baseline, case), case, false);
    let with = run_slice_with_runtime_extensions(
        options,
        extensions(
            vec![PreparationViewRequest {
                module_id: MODULE_ID.into(),
                view: VIEW.into(),
            }],
            Vec::new(),
            Arc::new(Trace::default()),
        ),
    )
    .unwrap_or_else(|error| panic!("case `{}`: captured slice failed: {error}", case.case_id));
    assert_eq!(
        with.gcode_text, without.gcode_text,
        "case `{}`: requesting the typed projection must not perturb emitted G-code",
        case.case_id
    );
    assert!(with.layer_count > 0, "the captured slice must emit layers");

    // A requested-but-undeclared view is a named row-06 failure, never a silent
    // empty scene. This is the undeclared half; the declared-but-unpublished
    // half is the `MissingScene` leg below.
    let options = case_options(&root, case_input(&root, &baseline, case), case, false);
    let error = run_visual(
        options,
        vec![PreparationViewRequest {
            module_id: MODULE_ID.into(),
            view: "lightning_plan_absent".into(),
        }],
        Vec::new(),
        vec![0],
        Arc::new(Trace::default()),
    )
    .expect_err("a requested-but-undeclared view must fail closed");
    assert!(
        matches!(
            error,
            PreparationVisualCaptureError::UnknownView { ref module_id, ref view }
                if module_id.as_str() == MODULE_ID && view == "lightning_plan_absent"
        ),
        "a requested-but-undeclared view must be the named row-06 view error: {error:?}"
    );

    // A declared-but-unpublished scene is `MissingScene`, not `UnknownView`:
    // `missing_scene` keeps the manifest declaration and the plan pieces and
    // skips only the projection publication, so the request reaches a completed
    // owner with nothing published. Without this leg the AC's "requested-but-
    // missing scene" claim is exercised only by the undeclared case, which the
    // host rejects before it ever looks for a scene.
    // test-quality: the diagnostic key is the only real-adapter trigger for the
    // module-owned missing-scene publication path named by AC-4; it is inert at
    // its default and never reaches the kernel.
    let options = diagnostic_options(&root, &baseline, case, "missing_scene");
    let error = run_visual(
        options,
        vec![PreparationViewRequest {
            module_id: MODULE_ID.into(),
            view: VIEW.into(),
        }],
        Vec::new(),
        vec![0],
        Arc::new(Trace::default()),
    )
    .expect_err("a declared view with no published scene must fail closed");
    assert!(
        matches!(
            error,
            PreparationVisualCaptureError::ProjectionRejected {
                ref module_id,
                ref view,
                reason: PreparationProjectionError::MissingScene,
            } if module_id.as_str() == MODULE_ID && view == VIEW
        ),
        "a declared-but-unpublished scene must be the named MissingScene rejection: {error:?}"
    );

    // Explicit empty scene (D6 `empty_plan`): the module publishes one empty
    // plan per layer and an empty `lightning-plan` scene, so a selected owner is
    // Ready/no-work rather than missing a prerequisite.
    // test-quality: the diagnostic key is the only real-adapter trigger for the
    // module-owned empty-plan publication path named by AC-4/AC-N3; it is inert
    // at its default and never reaches the kernel.
    let trace = Arc::new(Trace::default());
    let options = diagnostic_options(&root, &baseline, case, "empty_plan");
    let empty = run_visual(
        options,
        vec![PreparationViewRequest {
            module_id: MODULE_ID.into(),
            view: VIEW.into(),
        }],
        vec![INFILL_TAP.to_owned()],
        layer_indices(case),
        Arc::clone(&trace),
    )
    .unwrap_or_else(|error| {
        panic!(
            "case `{}`: `empty_plan` must succeed as Ready/no-work: {error}",
            case.case_id
        )
    });
    assert_prepared_owner(&empty.metadata(), &case.case_id);
    assert!(
        trace.count(
            MODULE_ID,
            PreparationLifecyclePhase::Preparation,
            PreparationLifecycleStatus::Ready
        ) > 0,
        "case `{}`: the empty preparation is Ready, not a missing prerequisite",
        case.case_id
    );
    assert!(
        trace.failures(MODULE_ID).is_empty(),
        "case `{}`: a successful empty preparation is not a failure",
        case.case_id
    );
    let empty_scene = empty
        .captures()
        .iter()
        .find_map(|stage| match &stage.ir {
            CapturedIr::PreparationProjection(snapshot) => Some(Arc::clone(snapshot)),
            _ => None,
        })
        .unwrap_or_else(|| {
            panic!(
                "case `{}`: `empty_plan` must publish the requested scene",
                case.case_id
            )
        });
    assert_eq!(empty_scene.scene().scene_id, "lightning-plan");
    assert_eq!(empty_scene.scene().view, VIEW);
    assert_eq!(
        empty_scene
            .scene()
            .classes
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>(),
        ["grounding", "planning_domain", "tree_branch", "tree_root"]
            .into_iter()
            .collect::<BTreeSet<_>>(),
        "case `{}`: the empty scene declares the four planning classes",
        case.case_id
    );
    assert!(
        empty_scene.scene().primitives.is_empty(),
        "case `{}`: the explicit empty scene publishes no primitives",
        case.case_id
    );
    for (layer, infill) in captured_infill(&empty) {
        for region in &infill.regions {
            assert!(
                region.sparse_infill.is_empty(),
                "case `{}`: layer {layer} region {}: the empty plan emits no sparse paths",
                case.case_id,
                region.region_id
            );
        }
    }
}

// ---------------------------------------------------------------------------
// AC-5
// ---------------------------------------------------------------------------

/// The six live docs' replacement headings. Their check is a *documentation*
/// check, not an absence check: the docs name the retired symbols legitimately.
const RETIRED_DOC_SECTIONS: &[(&str, &str)] = &[
    (
        "docs/01_system_architecture.md",
        "Lightning private preparation migration",
    ),
    ("docs/02_ir_schemas.md", "Lightning legacy IR retirement"),
    (
        "docs/03_wit_and_manifest.md",
        "Lightning accessor retirement",
    ),
    (
        "docs/04_host_scheduler.md",
        "Lightning selection-driven preparation",
    ),
    (
        "docs/05_module_sdk.md",
        "Lightning prepared module consumption",
    ),
    (
        "docs/19_visual_debug.md",
        "Lightning private preparation migration",
    ),
];

/// Live-contract scope of the AC-5 absence greps (code paths and WIT only).
const LIVE_CONTRACT_ROOTS: &[&str] = &[
    "crates/slicer-ir",
    "crates/slicer-runtime",
    "crates/slicer-wasm-host",
    "crates/slicer-sdk",
    "crates/slicer-macros",
    "crates/slicer-scheduler",
    "crates/slicer-schema/wit",
    "modules/core-modules/lightning-infill/src",
];

/// Paths the packate exempts from the absence greps: the committed fixture, the
/// row-07 case material, and the two migration-harness files that name the
/// retired symbols by design.
const ABSENCE_EXEMPT_PREFIXES: &[&str] = &[
    "crates/slicer-runtime/tests/fixtures/lightning_baseline/",
    "crates/slicer-wasm-host/tests/fixtures/lightning_gate/",
    "crates/slicer-runtime/tests/e2e/lightning_migration_tdd.rs",
    "crates/slicer-runtime/tests/e2e/lightning_baseline.rs",
];

const RETIRED_CODE_SYMBOLS: &[&str] = &[
    "PrePass::LightningTreeGen",
    "host:lightning_tree",
    "LightningTreeIR",
    "LightningTreeEntry",
    "CURRENT_LIGHTNING_TREE_IR_SCHEMA_VERSION",
    "lightning-tree-segments",
    "lightning_tree_segments_for",
    "with_lightning_tree_ir",
    "slicer:lightning-queries",
];

/// AC-5 — the live contract scope carries no retired symbol, the six live docs
/// describe the retirement under their replacement heading, and the slice still
/// reproduces the recorded baseline through the module path.
// test-quality: the absence contract is defined over source and WIT text by the packet; the scan is the protected surface (a retirement that fails open is exactly what this asserts), and the positive equivalence leg is a real module run against the committed oracle.
#[test]
fn legacy_contract_is_retired() {
    let root = workspace_root();

    let mut scanned_files = 0usize;
    let mut hits: Vec<String> = Vec::new();
    for rel in LIVE_CONTRACT_ROOTS {
        let path = root.join(rel);
        assert!(
            path.is_dir(),
            "live-contract root is absent: {} — an absent root would make this check vacuous",
            path.display()
        );
        let mut files = Vec::new();
        collect_source_files(&path, &mut files);
        assert!(
            !files.is_empty(),
            "live-contract root {} contains no source file; the absence check would be vacuous",
            path.display()
        );
        scanned_files += files.len();
        for file in files {
            let relative = file
                .strip_prefix(&root)
                .unwrap_or(&file)
                .to_string_lossy()
                .replace('\\', "/");
            if ABSENCE_EXEMPT_PREFIXES.iter().any(|exempt| {
                relative.starts_with(exempt) || relative == exempt.trim_end_matches('/')
            }) {
                continue;
            }
            let text = read_text(&file).unwrap_or_else(|error| panic!("{error}"));
            for symbol in RETIRED_CODE_SYMBOLS {
                if text.contains(symbol) {
                    hits.push(format!("{relative}: {symbol}"));
                }
            }
        }
    }
    assert!(
        scanned_files > 0,
        "the live-contract scope scanned no files; an empty scan is not a retirement"
    );
    assert!(
        hits.is_empty(),
        "the legacy contract is still present ({} hit(s), first 20: {:?})",
        hits.len(),
        hits.iter().take(20).collect::<Vec<_>>()
    );

    for (doc, heading) in RETIRED_DOC_SECTIONS {
        let path = root.join(doc);
        let text = read_text(&path).unwrap_or_else(|error| panic!("{error}"));
        let marker = format!("### {heading}");
        let occurrences = text
            .lines()
            .filter(|line| line.trim_end() == marker)
            .count();
        assert_eq!(
            occurrences, 1,
            "{doc}: the replacement heading '{marker}' must exist exactly once"
        );
        let mut section = String::new();
        let mut inside = false;
        for line in text.lines() {
            if line.trim_end() == marker {
                inside = true;
                continue;
            }
            if inside && line.starts_with("### ") {
                break;
            }
            if inside {
                section.push_str(line);
                section.push('\n');
            }
        }
        assert!(
            section.contains("The legacy path is retired."),
            "{doc}: the section under '{marker}' must state 'The legacy path is retired.'"
        );
    }

    // The retirement must not be achieved by changing output: the module's own
    // slice still reproduces the recorded plan population.
    let (baseline, _manifest, _text) = load_baseline(&root)
        .unwrap_or_else(|error| panic!("committed lightning baseline is unusable: {error}"));
    let case = &baseline.cases[0];
    let trace = Arc::new(Trace::default());
    let options = case_options(&root, case_input(&root, &baseline, case), case, false);
    let capture = run_visual(
        options,
        Vec::new(),
        vec![INFILL_TAP.to_owned()],
        layer_indices(case),
        Arc::clone(&trace),
    )
    .unwrap_or_else(|error| {
        panic!(
            "case `{}`: the production path failed: {error}",
            case.case_id
        )
    });
    assert_prepared_owner(&capture.metadata(), &case.case_id);
    let mut population = Population::new();
    for (layer, ir) in &captured_infill(&capture) {
        for (key, entry) in
            population_from_infill(&case.case_id, *layer, ir).expect("unique emitted keys")
        {
            assert!(
                population.insert(key.clone(), entry).is_none(),
                "duplicated emitted key {}",
                key.describe()
            );
        }
    }
    let population = nonempty(population);
    let expected = population_from_plan_entries(case).expect("baseline population is unique");
    compare_populations(&case.case_id, &expected, &population)
        .unwrap_or_else(|error| panic!("retirement changed output: {error}"));
}

fn collect_source_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", dir.display()));
    for entry in entries {
        let path = entry.expect("directory entry").path();
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        if path.is_dir() {
            if name == "target" || name == ".git" {
                continue;
            }
            collect_source_files(&path, out);
        } else if matches!(
            path.extension().and_then(|extension| extension.to_str()),
            Some("rs" | "toml" | "wit")
        ) {
            out.push(path);
        }
    }
}

// ---------------------------------------------------------------------------
// AC-N1
// ---------------------------------------------------------------------------

/// AC-N1 — the comparator rejects every corruption class, and a comparator that
/// would pass on an empty population or on a length-only comparison fails its own
/// control. The valid population is the real module's, never the baseline compared
/// to itself.
#[test]
fn equivalence_rejects_corrupted_baseline() {
    let root = workspace_root();
    let (baseline, _manifest, _text) = load_baseline(&root)
        .unwrap_or_else(|error| panic!("committed lightning baseline is unusable: {error}"));
    let case = baseline
        .cases
        .iter()
        .find(|case| case.case_id == "pooled_regions_classic")
        .expect("the committed baseline must carry the pooled_regions_classic case");

    let trace = Arc::new(Trace::default());
    let options = case_options(&root, case_input(&root, &baseline, case), case, false);
    let capture = run_visual(
        options,
        Vec::new(),
        vec![INFILL_TAP.to_owned()],
        layer_indices(case),
        Arc::clone(&trace),
    )
    .unwrap_or_else(|error| {
        panic!(
            "case `{}`: the production path failed: {error}",
            case.case_id
        )
    });
    assert_prepared_owner(&capture.metadata(), &case.case_id);

    let mut candidate = Population::new();
    for (layer, ir) in &captured_infill(&capture) {
        for (key, entry) in
            population_from_infill(&case.case_id, *layer, ir).expect("unique emitted keys")
        {
            assert!(
                candidate.insert(key.clone(), entry).is_none(),
                "duplicated emitted key {}",
                key.describe()
            );
        }
    }
    let candidate = nonempty(candidate);
    let valid = population_from_plan_entries(case).expect("baseline population is unique");
    compare_populations(&case.case_id, &valid, &candidate)
        .unwrap_or_else(|error| panic!("the real module must reproduce the baseline: {error}"));

    // Dropped entry.
    let mut dropped = valid.clone();
    let dropped_key = dropped.keys().next().cloned().expect("nonempty population");
    dropped.remove(&dropped_key);
    let error = compare_populations(&case.case_id, &dropped, &candidate)
        .expect_err("a dropped entry must be rejected");
    assert_eq!(error.key.as_ref(), Some(&dropped_key));
    assert_eq!(error.detail, "unexpected entry");

    // Reversed edge direction.
    let mut reversed = valid.clone();
    {
        let entry = reversed
            .get_mut(&dropped_key)
            .expect("the first key exists");
        let (edge, count) = entry
            .edges
            .iter()
            .next()
            .map(|(edge, count)| (*edge, *count))
            .expect("entries carry edges");
        entry.edges.remove(&edge);
        entry.edges.insert((edge.2, edge.3, edge.0, edge.1), count);
    }
    let error = compare_populations(&case.case_id, &reversed, &candidate)
        .expect_err("a reversed edge must be rejected");
    assert_eq!(error.key.as_ref(), Some(&dropped_key));
    // A reversal changes the directed incidence imbalance, so the witness stage
    // rejects it before the edge multiset is compared. Both are real rejections
    // naming the case and key; the specific stage is not part of the contract.
    assert!(
        error.detail.contains("missing edge")
            || error.detail.contains("unexpected edge")
            || error.detail.contains("witness"),
        "reversed edge: {}",
        error.detail
    );

    // Dropped and reordered `variant_chain` elements: identity is the ordered chain.
    let synthetic_a = PlanKey {
        global_layer_index: dropped_key.global_layer_index,
        object_id: dropped_key.object_id.clone(),
        region_id: dropped_key.region_id,
        variant_chain: vec![
            ("paint".to_owned(), "\"a\"".to_owned()),
            ("modifier".to_owned(), "\"b\"".to_owned()),
        ],
    };
    let mut shortened = synthetic_a.clone();
    shortened.variant_chain.pop();
    let mut reordered = synthetic_a.clone();
    reordered.variant_chain.reverse();
    let entry = EntryPopulation {
        edges: valid[&dropped_key].edges.clone(),
    };
    let base: Population = [(synthetic_a.clone(), entry.clone())].into_iter().collect();
    let with_shortened: Population = [(shortened.clone(), entry.clone())].into_iter().collect();
    let with_reordered: Population = [(reordered.clone(), entry.clone())].into_iter().collect();
    let error = compare_populations(&case.case_id, &with_shortened, &base)
        .expect_err("a dropped variant_chain element must be rejected");
    assert_eq!(error.key.as_ref(), Some(&shortened));
    let error = compare_populations(&case.case_id, &with_reordered, &base)
        .expect_err("a reordered variant_chain must be rejected");
    assert_eq!(error.key.as_ref(), Some(&reordered));

    // Duplicated entry: identity keys are unique, never merged.
    let duplicated = vec![
        (&dropped_key, valid[&dropped_key].edges.clone()),
        (&dropped_key, valid[&dropped_key].edges.clone()),
    ];
    let error = try_population(&case.case_id, duplicated)
        .expect_err("a duplicated entry must be rejected, never merged");
    assert_eq!(
        error.detail,
        "duplicated entry; identity keys are unique by contract"
    );

    // Swapped region identity: same key *set*, wrong edge partition.
    //
    // The keys must actually differ and the two entries must carry different
    // edge sets, or the swap is a no-op that no comparator can (or should)
    // detect. The earlier revision swapped `region_id` between the first two
    // entries of one layer; on the recorded case both those regions are id 0, so
    // nothing changed and the control was vacuous.
    let mut swapped_entries: Vec<RecordedEntry> = case.plan_entries.clone();
    let same_layer: Vec<usize> = swapped_entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| entry.key.global_layer_index == dropped_key.global_layer_index)
        .map(|(index, _)| index)
        .collect();
    let pair = same_layer.iter().enumerate().find_map(|(position, &left)| {
        same_layer[position + 1..]
            .iter()
            .find(|&&right| swapped_entries[left].edges != swapped_entries[right].edges)
            .map(|&right| (left, right))
    });
    let (left, right) = pair.unwrap_or_else(|| {
        panic!(
            "case `{}`: the swapped-identity control needs two entries on one layer with \
             different edge sets",
            case.case_id
        )
    });
    let left_key = swapped_entries[left].key.clone();
    swapped_entries[left].key = swapped_entries[right].key.clone();
    swapped_entries[right].key = left_key;
    let swapped = population_from_plan_entries(&BaselineCase {
        plan_entries: swapped_entries,
        callers: case.callers.clone(),
        ..case.clone()
    })
    .expect("swapped keys are still unique");
    let error = compare_populations(&case.case_id, &swapped, &candidate)
        .expect_err("a swapped region identity must be rejected");
    assert!(
        error.detail.contains("edge") || error.detail.contains("witness"),
        "swapped identity: {}",
        error.detail
    );

    // Length-only stub: totals preserved, geometry lost.
    let mut stubbed = valid.clone();
    {
        let entry = stubbed.get_mut(&dropped_key).expect("the first key exists");
        let total: u64 = entry.edges.values().sum();
        entry.edges.clear();
        entry.edges.insert((0, 0, 0, 0), total);
    }
    compare_populations(&case.case_id, &stubbed, &candidate)
        .expect_err("a length-only stub must be rejected by the real comparator");
    compare_lengths_only(&case.case_id, &stubbed, &candidate).expect(
        "control: a length-only comparator accepts the stub, so the real comparator is not \
         length-only",
    );

    // Empty-population control: an empty population never compares equal.
    let empty = Population::new();
    let error = compare_populations(&case.case_id, &empty, &candidate)
        .expect_err("an empty expected population must be rejected");
    assert!(error.detail.contains("empty population"), "{error}");
    let error = compare_populations(&case.case_id, &valid, &empty)
        .expect_err("an empty candidate population must be rejected");
    assert!(error.detail.contains("empty population"), "{error}");
    let error = compare_lengths_only(&case.case_id, &empty, &candidate)
        .expect_err("the length-only control must also reject an empty population");
    assert!(error.detail.contains("empty population"), "{error}");
}

// ---------------------------------------------------------------------------
// AC-N2
// ---------------------------------------------------------------------------

/// AC-N2 — this module's four failure modes stay distinct, publication is atomic
/// and cleanup is truthful. The host producer is never the fallback.
#[test]
fn failed_or_malformed_plan_never_falls_back() {
    let root = workspace_root();
    let (baseline, _manifest, _text) = load_baseline(&root)
        .unwrap_or_else(|error| panic!("committed lightning baseline is unusable: {error}"));
    let case = baseline
        .cases
        .iter()
        .find(|case| case.case_id == "pooled_regions_classic")
        .expect("the committed baseline must carry the pooled_regions_classic case");

    // Planner-config disagreement across committed regions: give one object a
    // different density so the five print-wide scalars cannot agree.
    let mut mesh = case_input(&root, &baseline, case);
    assert!(
        mesh.objects.len() >= 2,
        "the disagreement needs two objects"
    );
    mesh.objects[1].config.data.insert(
        "infill_density".to_owned(),
        ConfigValue::Float(case.config.infill_density * 2.0),
    );
    let trace = Arc::new(Trace::default());
    let options = case_options(&root, mesh, case, false);
    let error = match run_slice_preparation_capture(
        options,
        extensions(Vec::new(), Vec::new(), Arc::clone(&trace)),
    ) {
        Ok(capture) => panic!(
            "case `{}`: a planner-config disagreement across committed regions must fail \
             preparation with this module's 57412; instead the run completed with {} owner(s) \
             and {} lifecycle record(s). The module publishes no preparation capability yet \
             (Step 7b), so the print-wide config is never read across committed regions and \
             the disagreement cannot be triggered.",
            case.case_id,
            capture.owners.len(),
            capture.lifecycle.len()
        ),
        Err(error) => error,
    };
    let text = format!("{error:?}");
    assert!(
        text.contains("57412"),
        "the disagreement must surface this module's 57412: {text}"
    );

    let failures = trace.failures(MODULE_ID);
    assert!(
        !failures.is_empty(),
        "the failure must be observed on the owner's lifecycle, not swallowed"
    );
    for failure in &failures {
        match failure.failure.as_deref() {
            Some(PreparationFailure::Module(module_error)) => {
                assert_eq!(module_error.code, 57412);
                assert!(module_error.fatal, "57412 is fatal");
                assert!(
                    module_error
                        .message
                        .starts_with("config disagrees across committed regions"),
                    "57412 message: {}",
                    module_error.message
                );
            }
            other => panic!("a config disagreement must be a typed module error: {other:?}"),
        }
    }
    // Atomic publication: no Ready, no consumer, zero retention.
    assert_eq!(
        trace.count(
            MODULE_ID,
            PreparationLifecyclePhase::Preparation,
            PreparationLifecycleStatus::Ready
        ),
        0,
        "a failed preparation publishes neither plan nor scene"
    );
    assert_eq!(
        trace.count(
            MODULE_ID,
            PreparationLifecyclePhase::Consumption,
            PreparationLifecycleStatus::Start
        ),
        0,
        "a failed preparation admits zero consumers"
    );
    assert_eq!(
        trace.max_retained_after_disposal(),
        0,
        "storage is released after the failure"
    );

    // `PreparedPlanView::current()` outside a print is `NotReady`, never an empty
    // plan: the fallback surface is closed.
    let not_ready = match slicer_sdk::preparation::PreparedPlanView::current() {
        Ok(_) => panic!("outside a prepared print there must be no current plan"),
        Err(error) => error,
    };
    assert_eq!(
        not_ready,
        slicer_sdk::preparation::PreparationError::NotReady,
        "a missing plan is NotReady, never empty"
    );

    // The module-owned diagnostic publication mode (D6) makes the remaining
    // three decoder/`require` paths reachable through the deployed artifact. Each
    // leg is a real slice: preparation succeeds, the ordinary stage reads its own
    // piece, and the failure names this module and this code.
    // test-quality: the diagnostic key is the only real-adapter trigger for the
    // module-owned decoder failure paths named by AC-N2; it is inert at its
    // default and never reaches the kernel.
    let topmost_entry_layer = case
        .plan_entries
        .iter()
        .map(|entry| entry.key.global_layer_index)
        .max()
        .expect("the pooled case records plan entries");

    let (missing, missing_trace) = consumer_failure(&root, &baseline, case, "missing_piece");
    assert_failure_cleanup(&missing_trace, &case.case_id, "missing_piece");
    assert!(
        missing.contains(MODULE_ID),
        "57401 must come from the module consumer: {missing}"
    );
    assert!(
        missing.contains("57401"),
        "the absent piece is the inherited 57401: {missing}"
    );
    assert!(missing.contains("fatal=true"), "57401 is fatal: {missing}");
    assert!(
        missing.contains(&format!(
            "required preparation piece missing: lightning-layer-{topmost_entry_layer}"
        )),
        "the absent piece is the topmost planned layer's: {missing}"
    );
    assert!(
        !missing.contains("57410") && !missing.contains("57411") && !missing.contains("57412"),
        "57401 must stay distinct from the other three failures: {missing}"
    );

    let (malformed, malformed_trace) = consumer_failure(&root, &baseline, case, "malformed_piece");
    assert_failure_cleanup(&malformed_trace, &case.case_id, "malformed_piece");
    assert!(
        malformed.contains(MODULE_ID) && malformed.contains("57410"),
        "a decoder-rejected body is this module's 57410: {malformed}"
    );
    assert!(
        malformed.contains("malformed lightning plan piece: lightning-layer-"),
        "57410 names the piece it rejected: {malformed}"
    );
    assert!(
        malformed.contains("fatal=true"),
        "57410 is fatal: {malformed}"
    );
    assert!(
        !malformed.contains("57401")
            && !malformed.contains("57411")
            && !malformed.contains("57412"),
        "57410 must stay distinct from the other three failures: {malformed}"
    );

    let (unsupported, unsupported_trace) =
        consumer_failure(&root, &baseline, case, "unsupported_version");
    assert_failure_cleanup(&unsupported_trace, &case.case_id, "unsupported_version");
    assert!(
        unsupported.contains(MODULE_ID) && unsupported.contains("57411"),
        "a stale-version piece is this module's 57411: {unsupported}"
    );
    assert!(
        unsupported.contains("unsupported lightning plan piece version: 2"),
        "57411 names the version it read: {unsupported}"
    );
    assert!(
        unsupported.contains("fatal=true"),
        "57411 is fatal: {unsupported}"
    );
    assert!(
        !unsupported.contains("57401")
            && !unsupported.contains("57410")
            && !unsupported.contains("57412"),
        "57411 must stay distinct from the other three failures: {unsupported}"
    );

    // None of the three consumer failures is the retired host producer.
    for (code, text) in [
        (57401, &missing),
        (57410, &malformed),
        (57411, &unsupported),
    ] {
        assert!(
            !text.contains("host:lightning_tree"),
            "{code} must never fall back to the host producer: {text}"
        );
    }
}

// ---------------------------------------------------------------------------
// AC-N3
// ---------------------------------------------------------------------------

/// AC-N3 — selection is the gate: an inactive owner prepares, consumes and reads
/// nothing; external selection wins over the integrated pair; a foreign or
/// disposed owner is `AccessDenied`.
#[test]
fn selection_override_empty_and_foreign_owner() {
    let root = workspace_root();
    let (baseline, _manifest, _text) = load_baseline(&root)
        .unwrap_or_else(|error| panic!("committed lightning baseline is unusable: {error}"));
    let case = &baseline.cases[0];

    // The predicate itself, evaluated on the real selection builder with a
    // committed schedule and no committed slice regions. This is the criterion's
    // `selection.invoke && !selection.targets.is_empty()` with the target side
    // empty, independent of whether a whole print can reach that shape.
    let module_id: ModuleId = MODULE_ID.into();
    let claims = vec!["claim:sparse-fill".to_string()];
    let invocation_config = ConfigView::new();
    let index = build_layer_selection_index(
        // exhaustive: the predicate fixture states all selection input authority.
        LayerSelectionIndexInput {
            stage_roster: slicer_schema::STAGES,
            global_layers: &[GlobalLayer {
                index: 0,
                ..GlobalLayer::default()
            }],
            modules: &[
                // exhaustive: the predicate fixture states every bound module field.
                LayerSelectionModuleInput {
                    stage_id: "Layer::Infill",
                    module_id: MODULE_ID,
                    claims: &claims,
                    paint_semantics: None,
                    invocation_config: &invocation_config,
                },
            ],
            slices: &[],
            region_map: None,
            support_plan: None,
            perimeters: &[],
        },
    )
    .expect("a committed schedule with no slices is a valid selection input");
    let selection = index
        .selection_for(0, &module_id)
        .expect("the module is bound to the layer");
    assert!(
        selection.invoke,
        "the layer is invoked for the bound module"
    );
    assert!(
        selection.targets.is_empty(),
        "no committed region means no target: {:?}",
        selection.targets
    );
    let participates = selection.invoke && !selection.targets.is_empty();
    assert!(
        !participates,
        "the shared preparation predicate is false for a targetless layer"
    );

    // Inactive owner: no selected lightning target exists on any layer, so
    // `invoke && !targets.is_empty()` is false everywhere and the owner is never
    // prepared for, invoked or read.
    //
    // The earlier revision used `sparse_fill_holder = "rectilinear-infill"` as
    // the trigger. That is not a no-target scenario: selection targets are per
    // committed region and claim-agnostic, so the loaded module still prepared
    // and still read its pieces; its held claims only suppressed emission
    // (`SliceRegionView::should_emit`). Re-derived, a *loaded* module with at
    // least one committed region always has a target, so the reachable
    // no-target print is one where the artifact is not selected at all. The
    // search roots therefore exclude lightning, and the print still emits real
    // output through the remaining modules.
    let trace = Arc::new(Trace::default());
    let options = case_options_with_config(
        &root,
        case_input(&root, &baseline, case),
        case,
        false,
        native_module_dirs(&root),
        &[],
    );
    let output = run_slice_with_runtime_extensions(
        options,
        extensions(Vec::new(), Vec::new(), Arc::clone(&trace)),
    )
    .unwrap_or_else(|error| {
        panic!("an unselected lightning artifact must not fail the slice: {error}")
    });
    assert!(
        output.layer_count > 0 && !output.gcode_text.is_empty(),
        "the print must still emit output without the lightning artifact"
    );
    assert!(
        trace.records().is_empty(),
        "the inactive owner must have zero preparation, consumption and piece reads: {:?}",
        trace.records()
    );

    // External artifact selected over the integrated one: the owner identity is the
    // staged WASM artifact and exactly one owner participates.
    let trace = Arc::new(Trace::default());
    let options = case_options(&root, case_input(&root, &baseline, case), case, false);
    let capture = run_visual(
        options,
        Vec::new(),
        vec![INFILL_TAP.to_owned()],
        vec![0],
        Arc::clone(&trace),
    )
    .unwrap_or_else(|error| panic!("external selection run failed: {error}"));
    assert_prepared_owner(&capture.metadata(), &case.case_id);
    let owner = &capture.metadata().owners[0];
    assert!(
        owner.artifact_identity.starts_with("wasm:"),
        "external selection must select the staged WASM artifact, got {}",
        owner.artifact_identity
    );
    assert!(
        !owner.selected_targets.is_empty()
            && owner.selected_targets.iter().all(|target| target.invoke),
        "only real invoked targets participate: {:?}",
        owner.selected_targets
    );

    // A foreign owner never reads this print's pieces: outside a prepared print the
    // view is `NotReady`.
    let not_ready = match slicer_sdk::preparation::PreparedPlanView::current() {
        Ok(_) => panic!("a foreign scope must not expose a prepared plan"),
        Err(error) => error,
    };
    assert_eq!(
        not_ready,
        slicer_sdk::preparation::PreparationError::NotReady
    );

    // Ready/no-work: a selected owner whose preparation succeeds with an empty
    // plan is Ready, not a missing prerequisite, and admits consumers that emit
    // no sparse paths. D6's `empty_plan` mode is the module-owned trigger.
    // test-quality: the diagnostic key is the only real-adapter trigger for the
    // module-owned empty-plan publication path named by AC-N3; it is inert at its
    // default and never reaches the kernel.
    let trace = Arc::new(Trace::default());
    let options = diagnostic_options(&root, &baseline, case, "empty_plan");
    let empty = run_visual(
        options,
        Vec::new(),
        vec![INFILL_TAP.to_owned()],
        layer_indices(case),
        Arc::clone(&trace),
    )
    .unwrap_or_else(|error| {
        panic!("an empty successful preparation must not fail the slice: {error}")
    });
    assert_prepared_owner(&empty.metadata(), &case.case_id);
    assert!(
        trace.count(
            MODULE_ID,
            PreparationLifecyclePhase::Preparation,
            PreparationLifecycleStatus::Ready
        ) > 0,
        "a successful empty preparation is Ready, not a missing prerequisite"
    );
    assert!(
        trace.failures(MODULE_ID).is_empty(),
        "Ready/no-work is not a failure: {:?}",
        trace.failures(MODULE_ID)
    );
    assert_eq!(
        trace.max_retained_after_disposal(),
        0,
        "an empty preparation releases every piece"
    );
    let empty_infill = captured_infill(&empty);
    assert!(
        !empty_infill.is_empty(),
        "the consumers must still run on the captured layer"
    );
    for (layer, infill) in &empty_infill {
        for region in &infill.regions {
            assert!(
                region.sparse_infill.is_empty(),
                "layer {layer} region {}: an empty plan emits no sparse paths",
                region.region_id
            );
        }
    }

    // Foreign/disposed owner resources stay `AccessDenied`, driven through the
    // already-public store and the module's real frozen pair. No test-only
    // injector is involved: the piece is staged and opened by the store, then the
    // owner is disposed and the same handle must be refused.
    let (module_id, entry, schema) = slicer_integrated_modules::prepared_native_entries()
        .into_iter()
        .find(|(module_id, _, _)| module_id == MODULE_ID)
        .expect("the module's generated prepared pair must be exported");
    assert_eq!(module_id, MODULE_ID);
    let interface = schema
        .qualified_export()
        .strip_suffix("#prepare-print")
        .expect("the schema names the capability's qualified export");
    let declaration = PreparationDeclaration::new(
        interface,
        vec!["layer_plan".into(), "slices".into(), "region_map".into()],
        vec![VIEW.into()],
    );
    let binding = freeze_native(
        MODULE_ID.into(),
        Some(declaration),
        NativePreparationArtifact::Prepared { entry, schema },
    )
    .expect("the module's real prepared pair must freeze");
    let store = PreparedOwnerStore::new();
    let owner = store.create_owner(&binding).expect("owner creation");
    let mut transaction = store.begin(&owner).expect("staging begins");
    transaction
        .put("lightning-layer-0", b"probe")
        .expect("stage one piece");
    let plan = transaction
        .finish(Ok(()), |_| Ok(()))
        .expect("the staged piece publishes");
    let piece = plan
        .open("lightning-layer-0")
        .expect("a live owner opens its piece")
        .expect("the staged piece exists");
    assert_eq!(
        piece.read(0, 5).expect("a live owner reads its piece"),
        b"probe".to_vec()
    );
    store.dispose_owner(&owner).expect("an idle owner disposes");
    assert_eq!(
        owner.metadata().retained_payload_bytes(),
        0,
        "disposal releases the staged payload"
    );
    assert!(
        matches!(
            piece.read(0, 5),
            Err(PreparationTransportError::AccessDenied)
        ),
        "a disposed owner's piece handle is refused"
    );
    assert!(
        matches!(
            plan.open("lightning-layer-0"),
            Err(PreparationTransportError::AccessDenied)
        ),
        "a disposed owner's plan authority is refused"
    );
    assert!(
        matches!(store.current(&owner), Err(PreparationFailure::Disposed)),
        "the disposed owner is no longer current"
    );
    store.dispose().expect("the print authority disposes");
}

// ---------------------------------------------------------------------------
// AC-N4
// ---------------------------------------------------------------------------

/// One removed-symbol compile witness: a probe that `use`s (or calls) exactly one
/// retired symbol, compiled against the built crates. The witness must fail with a
/// diagnostic naming that symbol, and must not implicate any other retired symbol.
struct CompileWitness {
    crate_name: &'static str,
    symbol: &'static str,
    source: &'static str,
}

const COMPILE_WITNESSES: &[CompileWitness] = &[
    CompileWitness {
        crate_name: "slicer_ir",
        symbol: "LightningTreeIR",
        source: "use slicer_ir::LightningTreeIR;\n",
    },
    CompileWitness {
        crate_name: "slicer_ir",
        symbol: "LightningTreeEntry",
        source: "use slicer_ir::LightningTreeEntry;\n",
    },
    CompileWitness {
        crate_name: "slicer_ir",
        symbol: "CURRENT_LIGHTNING_TREE_IR_SCHEMA_VERSION",
        source: "use slicer_ir::CURRENT_LIGHTNING_TREE_IR_SCHEMA_VERSION;\n",
    },
    CompileWitness {
        crate_name: "slicer_sdk",
        symbol: "lightning_tree_segments_for",
        source: "pub fn probe() {\n    let view = slicer_sdk::traits::PaintRegionLayerView::new(0);\n    let _ = view.lightning_tree_segments_for(\"object\", 0, &[]);\n}\n",
    },
    CompileWitness {
        crate_name: "slicer_sdk",
        symbol: "with_lightning_tree_ir",
        source: "pub fn probe() {\n    let _ = slicer_sdk::traits::PaintRegionLayerView::new(0).with_lightning_tree_ir(std::sync::Arc::new(Default::default()));\n}\n",
    },
];

/// Positive control for the compile-witness harness: a symbol that must still
/// resolve. If this fails, the harness itself is unsound — wrong rlib, wrong
/// `--extern` flags — and every retirement witness it accepts is meaningless.
const SURVIVING_WITNESS: CompileWitness = CompileWitness {
    crate_name: "slicer_ir",
    symbol: "SliceIR",
    source: "use slicer_ir::SliceIR;\npub fn probe(_: Option<SliceIR>) {}\n",
};

/// Locate the newest built rlib for `crate_name` in the target dir that `cargo
/// test` is already using.
fn find_rlib(crate_name: &str) -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace = manifest_dir
        .join("..")
        .join("..")
        .canonicalize()
        .expect("workspace root");
    let prefix = format!("lib{crate_name}-");
    for profile in ["debug", "release"] {
        let deps = workspace.join("target").join(profile).join("deps");
        let Ok(entries) = std::fs::read_dir(&deps) else {
            continue;
        };
        let mut candidates: Vec<(std::time::SystemTime, PathBuf)> = entries
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| {
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                name.starts_with(&prefix) && name.ends_with(".rlib")
            })
            .filter_map(|path| {
                let modified = std::fs::metadata(&path).ok()?.modified().ok()?;
                Some((modified, path))
            })
            .collect();
        candidates.sort();
        if let Some((_, newest)) = candidates.pop() {
            return newest;
        }
    }
    panic!(
        "cannot find a built rlib for '{crate_name}' under {}; \
         the compile witness cannot run without a prior `cargo build` of this package",
        workspace.join("target").display()
    )
}

fn compile_witness(witness: &CompileWitness) -> Result<(), String> {
    let dir = TempDir::new().map_err(|error| format!("temp dir for compile witness: {error}"))?;
    let source_path = dir.path().join("probe.rs");
    std::fs::write(&source_path, witness.source)
        .map_err(|error| format!("write compile witness: {error}"))?;
    let rlib = find_rlib(witness.crate_name);
    let dependency_dir = rlib.parent().expect("rlib has a parent directory");
    let output = std::process::Command::new("rustc")
        .arg("--edition=2021")
        .arg("--crate-type=lib")
        .arg("--emit=metadata")
        .arg("--out-dir")
        .arg(dir.path())
        .arg(&source_path)
        .arg("-L")
        .arg(format!("dependency={}", dependency_dir.display()))
        .arg("--extern")
        .arg(format!(
            "{}={}",
            witness.crate_name,
            rlib.canonicalize().unwrap_or(rlib).display()
        ))
        .output()
        .map_err(|error| format!("cannot invoke rustc (needed by the compile witness): {error}"))?;
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    if output.status.success() {
        return Err(format!(
            "compile witness for '{}' still resolves; the symbol is not retired \
             (rustc emitted no diagnostic)",
            witness.symbol
        ));
    }
    if !stderr.contains(witness.symbol) {
        return Err(format!(
            "compile witness for '{}' failed for an unrelated reason (the diagnostic does not name the symbol): {stderr}",
            witness.symbol
        ));
    }
    // A symbol appearing in stderr is not a name-resolution diagnostic: the
    // compiler's source excerpt reproduces the `use` line, and loader failures
    // such as a missing rlib quote it too. Require an actual resolution error
    // code before accepting the witness.
    const NAME_RESOLUTION_CODES: [&str; 5] = ["E0432", "E0433", "E0425", "E0412", "E0599"];
    if !NAME_RESOLUTION_CODES
        .iter()
        .any(|code| stderr.contains(code))
    {
        return Err(format!(
            "compile witness for '{}' failed without a name-resolution diagnostic \
             (expected one of {NAME_RESOLUTION_CODES:?}): {stderr}",
            witness.symbol
        ));
    }
    for other in COMPILE_WITNESSES {
        if other.symbol != witness.symbol && stderr.contains(other.symbol) {
            return Err(format!(
                "compile witness for '{}' also implicates '{}': {stderr}",
                witness.symbol, other.symbol
            ));
        }
    }
    Ok(())
}

/// AC-N4 — every removed symbol fails to compile on its own name, and the slice
/// still emits baseline-equivalent infill through the module path only.
#[test]
fn retirement_leaves_no_host_fallback() {
    // Harness soundness first: a surviving symbol must compile against the same
    // rlib and `--extern` setup. `compile_witness` reports success as an error
    // ("still resolves"), so the expected outcome here is exactly that message;
    // any other failure means the harness itself is misconfigured and every
    // retirement witness below would prove nothing.
    match compile_witness(&SURVIVING_WITNESS) {
        Err(error) if error.contains("still resolves") => {}
        other => panic!(
            "compile-witness harness is unsound: a surviving symbol did not compile cleanly ({other:?})"
        ),
    }

    let failures: Vec<String> = COMPILE_WITNESSES
        .iter()
        .filter_map(|witness| compile_witness(witness).err())
        .collect();
    assert!(
        failures.is_empty(),
        "retired-symbol compile witnesses still resolve or fail wrong: {failures:?}"
    );

    // No host-owned plan remains: the same slice must still reproduce the recorded
    // infill through the module path, and the host producer must not be the source.
    let root = workspace_root();
    let (baseline, _manifest, _text) = load_baseline(&root)
        .unwrap_or_else(|error| panic!("committed lightning baseline is unusable: {error}"));
    let case = &baseline.cases[0];
    let trace = Arc::new(Trace::default());
    let options = case_options(&root, case_input(&root, &baseline, case), case, false);
    let capture = run_visual(
        options,
        Vec::new(),
        vec![INFILL_TAP.to_owned()],
        layer_indices(case),
        Arc::clone(&trace),
    )
    .unwrap_or_else(|error| panic!("case `{}`: production path failed: {error}", case.case_id));
    assert_prepared_owner(&capture.metadata(), &case.case_id);
    let mut population = Population::new();
    for (layer, ir) in &captured_infill(&capture) {
        for (key, entry) in
            population_from_infill(&case.case_id, *layer, ir).expect("unique emitted keys")
        {
            assert!(
                population.insert(key.clone(), entry).is_none(),
                "duplicated emitted key {}",
                key.describe()
            );
        }
    }
    let population = nonempty(population);
    let expected = population_from_plan_entries(case).expect("baseline population is unique");
    compare_populations(&case.case_id, &expected, &population)
        .unwrap_or_else(|error| panic!("baseline-equivalent infill is required: {error}"));
}
// ---------------------------------------------------------------------------
// Re-record
// ---------------------------------------------------------------------------

/// One recorded caller as JSON, in the same shape `decode_case` accepts.
fn caller_to_value(caller: &RecordedCaller) -> Value {
    json!({
        "key": caller.key.to_value(),
        "segment_count": caller.segment_count,
        "segments": edge_multiset(&caller.segments)
            .iter()
            .map(|((from_x, from_y, to_x, to_y), count)| json!([from_x, from_y, to_x, to_y, count]))
            .collect::<Vec<Value>>(),
    })
}

fn case_to_value(case: &BaselineCase) -> Value {
    json!({
        "case_id": case.case_id,
        "wall_strategy": "Classic",
        "fixture_object_id": case.fixture_object_id,
        "fixture_region_ids": case
            .mesh_objects
            .iter()
            .map(|object| object.rsplit('-').next().unwrap_or(object).parse::<u64>().expect("region id"))
            .collect::<Vec<u64>>(),
        "mesh_objects": case.mesh_objects,
        "config": {
            "sparse_fill_holder": case.config.sparse_fill_holder,
            "infill_density": case.config.infill_density,
            "layer_height": case.config.layer_height,
            "infill_angle": case.config.infill_angle,
            "line_width": case.config.line_width,
        },
        "layer_count": case.layer_count,
        "plan_entry_count": case.plan_entry_count,
        "plan_edge_count": case.plan_edge_count,
        "plan_entries": case.plan_entries.iter().map(entry_to_value).collect::<Vec<Value>>(),
        "caller_count": case.caller_count,
        "caller_segment_count": case.caller_segment_count,
        "callers": case.callers.iter().map(caller_to_value).collect::<Vec<Value>>(),
        "digests": {
            "case_source_sha256": case.digests.case_source_sha256,
            "inputs_sha256": case.digests.inputs_sha256,
            "plan_sha256": case.digests.plan_sha256,
        },
    })
}

fn summary_to_value(case: &BaselineCase) -> Value {
    json!({
        "case_id": case.case_id,
        "fixture_object_id": case.fixture_object_id,
        "mesh_objects": case.mesh_objects,
        "layer_count": case.layer_count,
        "plan_entry_count": case.plan_entry_count,
        "plan_edge_count": case.plan_edge_count,
        "caller_count": case.caller_count,
        "caller_segment_count": case.caller_segment_count,
        "digests": {
            "case_source_sha256": case.digests.case_source_sha256,
            "inputs_sha256": case.digests.inputs_sha256,
            "plan_sha256": case.digests.plan_sha256,
        },
    })
}

/// A case's `a_source` from the committed gate fixture, for the source digest.
fn case_a_source(root: &Path, case_id: &str) -> Value {
    let text = read_text(&root.join(GATE_FIXTURE_REL).join("inputs.json")).expect("inputs.json");
    let inputs: Value = serde_json::from_str(&text).expect("inputs.json is JSON");
    inputs["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .find(|candidate| candidate["case_id"].as_str() == Some(case_id))
        .expect("case present")["a_source"]
        .clone()
}

/// Re-record the committed baseline from the **current** module.
///
/// The committed baseline is the regression pin the migration tests compare
/// against. This recorder replays each case through the module's real prepared
/// path and rewrites the plan/caller populations, counts and digests. It keeps
/// the case material (geometry scale, config, synthetic paint, layer count) from
/// the existing fixture, so the recording changes only *what the module emits*.
///
/// Run explicitly:
///
/// ```text
/// cargo test -p slicer-runtime --features perimeter-spatial-test-support \
///   --test e2e lightning_migration_tdd::record_lightning_baseline -- --ignored --nocapture
/// ```
#[test]
#[ignore = "rewrites the committed baseline; run explicitly after an intentional output change"]
fn record_lightning_baseline() {
    let root = workspace_root();
    let (baseline, _manifest, _text) = load_baseline(&root)
        .unwrap_or_else(|error| panic!("the existing baseline is the case material: {error}"));

    let mut recorded = Vec::new();
    for case in &baseline.cases {
        let trace = Arc::new(Trace::default());
        let options = case_options(&root, case_input(&root, &baseline, case), case, false);
        let supplied = options.config_overrides.clone();
        let capture = run_visual(
            options,
            Vec::new(),
            vec![SLICE_TAP.to_owned(), INFILL_TAP.to_owned()],
            layer_indices(case),
            Arc::clone(&trace),
        )
        .unwrap_or_else(|error| panic!("case `{}`: recording run failed: {error}", case.case_id));
        let slices = captured_slices(&capture);
        assert_eq!(
            slices.len() as u64,
            case.layer_count,
            "case `{}`: recording must expose every layer",
            case.case_id
        );
        let infill = captured_infill(&capture);
        assert_prepared_owner(&capture.metadata(), &case.case_id);

        // Emitted population, per layer, exactly as the acceptance comparison
        // builds it.
        let mut population: Population = BTreeMap::new();
        for (layer, ir) in &infill {
            for (key, entry) in
                population_from_infill(&case.case_id, *layer, ir).expect("unique emitted keys")
            {
                assert!(
                    population.insert(key.clone(), entry).is_none(),
                    "case `{}`: layer {layer} emitted a duplicated key {}",
                    case.case_id,
                    key.describe()
                );
            }
        }
        let population = nonempty(population);
        assert!(
            !population.is_empty(),
            "case `{}`: the module emitted no plan population",
            case.case_id
        );

        // Callers: every committed SliceIR region identity, with the segments
        // the module emitted for it (empty for a region it did not fill).
        let mut callers: BTreeMap<PlanKey, RecordedCaller> = BTreeMap::new();
        for (layer, slice) in &slices {
            for region in &slice.regions {
                let key = PlanKey::from_slice_region(*layer, region);
                let edges = population
                    .get(&key)
                    .map(|entry| entry.edges.clone())
                    .unwrap_or_default();
                let caller = RecordedCaller {
                    key: key.clone(),
                    segment_count: edges.values().sum(),
                    segments: edges.into_iter().collect(),
                };
                assert!(
                    callers.insert(key.clone(), caller).is_none(),
                    "case `{}`: duplicated caller key {}",
                    case.case_id,
                    key.describe()
                );
            }
        }

        let plan_entries: Vec<RecordedEntry> = population
            .iter()
            .map(|(key, entry)| RecordedEntry {
                key: key.clone(),
                edge_count: entry.edges.values().sum(),
                edges: entry
                    .edges
                    .iter()
                    .map(|(edge, count)| (*edge, *count))
                    .collect(),
                witnesses: witness_set(&entry.edges).into_iter().collect(),
            })
            .collect();
        let plan_edge_count: u64 = plan_entries.iter().map(|entry| entry.edge_count).sum();
        let callers: Vec<RecordedCaller> = callers.into_values().collect();
        let caller_segment_count: u64 = callers.iter().map(|caller| caller.segment_count).sum();

        let mut new_case = BaselineCase {
            case_id: case.case_id.clone(),
            fixture_object_id: case.fixture_object_id.clone(),
            mesh_objects: case.mesh_objects.clone(),
            config: case.config.clone(),
            layer_count: case.layer_count,
            plan_entry_count: plan_entries.len() as u64,
            plan_edge_count,
            caller_count: callers.len() as u64,
            caller_segment_count,
            plan_entries,
            callers,
            digests: CaseDigests {
                case_source_sha256: sha256_canonical(&case_a_source(&root, &case.case_id)),
                inputs_sha256: inputs_digest(&supplied, &slices),
                plan_sha256: String::new(),
            },
        };
        new_case.digests.plan_sha256 = plan_digest(&new_case);
        eprintln!(
            "RECORD {}: layers={} entries={} edges={} callers={} caller_segments={}",
            new_case.case_id,
            new_case.layer_count,
            new_case.plan_entry_count,
            new_case.plan_edge_count,
            new_case.caller_count,
            new_case.caller_segment_count
        );
        recorded.push(new_case);
    }

    let case_ids: Vec<String> = recorded.iter().map(|case| case.case_id.clone()).collect();
    let case_values: Vec<Value> = recorded.iter().map(case_to_value).collect();
    let summaries: Vec<Value> = recorded.iter().map(summary_to_value).collect();

    let mut derivation = baseline.derivation.clone();
    // The derivation must describe the harness that produced these bytes.
    // The mesh builder writes PnP units straight into the millimetre-valued
    // `Point3` surface, so the planned domain is 100x the canonical region;
    // `geometry_scale` is applied during the canonical -> PnP conversion.
    derivation["mesh"] = json!(
        "one mesh object per fixture region identity, footprint = that region's recorded surfaces \
         (contours and holes) per layer, extruded through the layer band; the builder writes the \
         converted PnP units into the millimetre-valued MeshIR Point3, so the planned domain is \
         100x the canonical region; band boundaries sit one quarter layer-height above the layer \
         print Z so no slice plane lies in a horizontal face"
    );
    derivation["callers"] =
        json!("emitted Layer::Infill sparse paths grouped by committed SliceIR region identity");
    derivation["witnesses"] = json!(
        "directed incidence imbalance of the committed edge multiset, recorded from the module's \
         emitted sparse paths"
    );

    let baseline_value = json!({
        "schema_version": 1,
        "recorder": "crates/slicer-runtime/tests/e2e/lightning_migration_tdd.rs::record_lightning_baseline",
        "producer": "com.core.lightning-infill whole-print preparation, read back from the emitted Layer::Infill paths",
        "case_source": baseline.case_source,
        "units": {
            "canonical": "canonical_units_1nm",
            "plan": "pnp_units_100nm",
            "pnp_units_per_canonical_unit": 100,
        },
        "derivation": derivation,
        "cases": case_values,
    });
    let baseline_text = format!(
        "{}\n",
        serde_json::to_string(&baseline_value).expect("baseline serializes")
    );

    let fixture_dir = root.join(GATE_FIXTURE_REL);
    let inputs_text = read_text(&fixture_dir.join("inputs.json")).expect("fixture inputs");
    let fixture_manifest_text =
        read_text(&fixture_dir.join("manifest.json")).expect("fixture manifest");
    let manifest_value = json!({
        "schema_version": 1,
        "recorder": "crates/slicer-runtime/tests/e2e/lightning_migration_tdd.rs::record_lightning_baseline",
        "baseline_file": BASELINE_FILE,
        "baseline_sha256": sha256_hex(baseline_text.as_bytes()),
        "fixture": {
            "dir": baseline.case_source,
            "inputs_sha256": sha256_hex(inputs_text.as_bytes()),
            "manifest_sha256": sha256_hex(fixture_manifest_text.as_bytes()),
            "case_count": case_ids.len() as u64,
            "case_ids": case_ids,
        },
        "case_summaries": summaries,
    });
    let manifest_text = format!(
        "{}\n",
        serde_json::to_string(&manifest_value).expect("manifest serializes")
    );

    let dir = root.join(BASELINE_REL);
    std::fs::write(dir.join(BASELINE_FILE), baseline_text).expect("write baseline");
    std::fs::write(dir.join(MANIFEST_FILE), manifest_text).expect("write manifest");
    eprintln!("RECORD wrote {BASELINE_FILE} and {MANIFEST_FILE}");
}
