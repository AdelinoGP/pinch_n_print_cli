//! TDD tests for the support source-selection key shape: `SupportGeometryKey`
//! (layer/object/region plus defaulted ordered `variant_chain`, `u32::MAX`
//! intermediate sentinel) and the chain-bearing `SupportSourceKey` used only
//! for `SupportAnalysisIR::family_assignments`, plus JSON omission
//! normalization and populated-owner postcard full-identity round-trips.
//!
//! Packet `layer-module-preparation_01`, Step I1 (TASK-572, AC-R3/R8/R9/R10).

#![allow(missing_docs)]

use std::collections::{BTreeMap, HashMap};

use slicer_ir::slice_ir::{
    SupportCandidate, SupportCandidateSource, SupportSourceKey,
    CURRENT_SUPPORT_ANALYSIS_IR_SCHEMA_VERSION, CURRENT_SUPPORT_GEOMETRY_IR_SCHEMA_VERSION,
};
use slicer_ir::{
    ExPolygon, PaintValue, Point2, Polygon, SemVer, SupportAnalysisIR, SupportGeometryIR,
    SupportGeometryKey,
};

fn empty_expolygon() -> ExPolygon {
    ExPolygon {
        contour: Polygon { points: vec![] },
        holes: vec![],
    }
}

/// A distinguishable, nonempty square polygon. Distinct origins produce
/// structurally different payloads, so map membership can never be satisfied
/// by an accidental payload match.
fn square(x: i64, y: i64, side: i64) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 { x, y },
                Point2 { x: x + side, y },
                Point2 {
                    x: x + side,
                    y: y + side,
                },
                Point2 { x, y: y + side },
            ],
        },
        holes: vec![],
    }
}

fn chain(entries: &[(&str, PaintValue)]) -> Vec<(String, PaintValue)> {
    entries
        .iter()
        .map(|(name, value)| ((*name).to_string(), value.clone()))
        .collect()
}

fn geometry_key(
    layer: u32,
    object: &str,
    region: u64,
    chain_entries: &[(&str, PaintValue)],
) -> SupportGeometryKey {
    SupportGeometryKey {
        global_support_layer_index: layer,
        object_id: object.to_string(),
        region_id: region,
        variant_chain: chain(chain_entries),
    }
}

fn source_key(object: &str, region: u64, chain_entries: &[(&str, PaintValue)]) -> SupportSourceKey {
    SupportSourceKey {
        object_id: object.to_string(),
        region_id: region,
        variant_chain: chain(chain_entries),
    }
}

fn candidate_source(
    object: &str,
    region: u64,
    layer: u32,
    z_units: i64,
    chain_entries: &[(&str, PaintValue)],
) -> SupportCandidateSource {
    // exhaustive: postcard identity fixture pins every widened source field
    SupportCandidateSource {
        object_id: object.to_string(),
        region_id: region,
        global_layer_index: layer,
        z_units,
        variant_chain: chain(chain_entries),
    }
}

fn candidate(id: u64, geometry: ExPolygon, source: SupportCandidateSource) -> SupportCandidate {
    // exhaustive: postcard identity fixture pins every candidate field
    SupportCandidate {
        id,
        geometry: vec![geometry],
        source,
        enforced: true,
        blocked: false,
    }
}

/// Verifies:
/// 1. The map key is `SupportGeometryKey` with fields
///    `(global_support_layer_index: u32, object_id: ObjectId, region_id: RegionId)`
///    plus the defaulted ordered `variant_chain`.
/// 2. An entry keyed by `global_support_layer_index = u32::MAX` is retrievable
///    and is the designated intermediate-model-resolution-outline sentinel.
#[test]
fn support_geometry_ir_keys_and_sentinel() {
    let sentinel_key = SupportGeometryKey {
        global_support_layer_index: u32::MAX,
        object_id: String::from("obj-1"),
        region_id: 0,
        variant_chain: Vec::new(),
    };
    let normal_key = SupportGeometryKey {
        global_support_layer_index: 3,
        object_id: String::from("obj-1"),
        region_id: 1,
        variant_chain: Vec::new(),
    };

    let mut entries = HashMap::new();
    entries.insert(sentinel_key.clone(), vec![empty_expolygon()]);
    entries.insert(
        normal_key.clone(),
        vec![empty_expolygon(), empty_expolygon()],
    );

    let ir = SupportGeometryIR {
        schema_version: CURRENT_SUPPORT_GEOMETRY_IR_SCHEMA_VERSION,
        support_layer_height_mm: 0.2,
        support_top_z_distance_mm: 0.1,
        entries,
    };

    // Key tuple shape: (u32, ObjectId/String, RegionId/u64) + ordered chain.
    let sentinel = ir
        .entries
        .get(&sentinel_key)
        .expect("sentinel entry must be retrievable by u32::MAX key");
    assert_eq!(sentinel.len(), 1, "sentinel entry should have one polygon");

    // u32::MAX is the documented sentinel for intermediate model-resolution outlines.
    assert_eq!(
        sentinel_key.global_support_layer_index,
        u32::MAX,
        "sentinel key must be u32::MAX"
    );
    assert!(
        sentinel_key.variant_chain.is_empty(),
        "sentinel key carries an exact empty chain"
    );

    let normal = ir
        .entries
        .get(&normal_key)
        .expect("normal entry must be retrievable");
    assert_eq!(normal.len(), 2, "normal entry should have two polygons");

    assert_eq!(ir.entries.len(), 2);
}

/// Same layer/object/region, different ordered chains: each chain identity
/// selects its own map entry. The oracle is value membership and exact
/// payloads — never hash-inequality.
#[test]
fn support_geometry_key_ordered_chain_selects_distinct_entries() {
    let empty = geometry_key(3, "obj-1", 4, &[]);
    let pos_zero = geometry_key(3, "obj-1", 4, &[("surface", PaintValue::Scalar(0.0))]);
    let neg_zero = geometry_key(3, "obj-1", 4, &[("surface", PaintValue::Scalar(-0.0))]);
    let ab = geometry_key(
        3,
        "obj-1",
        4,
        &[
            ("first", PaintValue::Flag(true)),
            ("second", PaintValue::ToolIndex(2)),
        ],
    );
    let ba = geometry_key(
        3,
        "obj-1",
        4,
        &[
            ("second", PaintValue::ToolIndex(2)),
            ("first", PaintValue::Flag(true)),
        ],
    );
    let sentinel = geometry_key(u32::MAX, "obj-1", 4, &[]);

    let mut entries = HashMap::new();
    entries.insert(empty.clone(), vec![square(0, 0, 100)]);
    entries.insert(pos_zero.clone(), vec![square(1_000, 0, 100)]);
    entries.insert(neg_zero.clone(), vec![square(2_000, 0, 100)]);
    entries.insert(
        ab.clone(),
        vec![square(3_000, 0, 100), square(4_000, 0, 100)],
    );
    entries.insert(ba.clone(), vec![square(5_000, 0, 100)]);
    entries.insert(sentinel.clone(), vec![square(6_000, 0, 100)]);

    // All six chain-distinct identities coexist as distinct entries.
    assert_eq!(
        entries.len(),
        6,
        "ordered-chain variants must not collapse into one entry"
    );

    // Exact membership and payloads, via independently rebuilt key values.
    assert_eq!(entries.get(&empty), Some(&vec![square(0, 0, 100)]));
    assert_eq!(entries.get(&pos_zero), Some(&vec![square(1_000, 0, 100)]));
    assert_eq!(entries.get(&neg_zero), Some(&vec![square(2_000, 0, 100)]));
    assert_eq!(
        entries.get(&ab),
        Some(&vec![square(3_000, 0, 100), square(4_000, 0, 100)])
    );
    assert_eq!(entries.get(&ba), Some(&vec![square(5_000, 0, 100)]));
    assert_eq!(entries.get(&sentinel), Some(&vec![square(6_000, 0, 100)]));

    // Reversed two-member chains are different identities with different rows.
    assert_ne!(ab, ba);
    assert_ne!(entries.get(&ab), entries.get(&ba));

    // Signed zero is a distinct scalar identity under PaintValue's established
    // bit-level equality, and retained scalars stay finite.
    assert_ne!(pos_zero, neg_zero);
    match &pos_zero.variant_chain[0].1 {
        PaintValue::Scalar(value) => {
            assert!(value.is_finite(), "stored scalar must be finite");
            assert_eq!(value.to_bits(), 0.0_f32.to_bits());
        }
        other => panic!("expected scalar paint value, got {other:?}"),
    }
    match &neg_zero.variant_chain[0].1 {
        PaintValue::Scalar(value) => {
            assert!(value.is_finite(), "stored scalar must be finite");
            assert_eq!(value.to_bits(), (-0.0_f32).to_bits());
        }
        other => panic!("expected scalar paint value, got {other:?}"),
    }

    // Order is data: exact member order is preserved, not sorted.
    assert_eq!(
        ab.variant_chain,
        chain(&[
            ("first", PaintValue::Flag(true)),
            ("second", PaintValue::ToolIndex(2)),
        ])
    );
    assert_eq!(
        ba.variant_chain,
        chain(&[
            ("second", PaintValue::ToolIndex(2)),
            ("first", PaintValue::Flag(true)),
        ])
    );

    // The u32::MAX intermediate-model-resolution sentinel survives widening.
    assert_eq!(sentinel.global_support_layer_index, u32::MAX);
}

/// An actual `SupportAnalysisIR::family_assignments` distinguishes siblings
/// that differ only by ordered chain, with independently expected family ids.
#[test]
fn support_source_family_assignment_separates_ordered_variants() {
    let empty = source_key("obj-1", 4, &[]);
    let single = source_key("obj-1", 4, &[("surface", PaintValue::Flag(true))]);
    let ab = source_key(
        "obj-1",
        4,
        &[
            ("first", PaintValue::ToolIndex(1)),
            ("second", PaintValue::Flag(false)),
        ],
    );
    let ba = source_key(
        "obj-1",
        4,
        &[
            ("second", PaintValue::Flag(false)),
            ("first", PaintValue::ToolIndex(1)),
        ],
    );

    let mut analysis = SupportAnalysisIR::default();
    let mut assignments: BTreeMap<SupportSourceKey, String> = BTreeMap::new();
    assignments.insert(empty.clone(), "family-empty".to_string());
    assignments.insert(single.clone(), "family-single".to_string());
    assignments.insert(ab.clone(), "family-forward".to_string());
    assignments.insert(ba.clone(), "family-reversed".to_string());
    analysis.family_assignments = assignments;

    assert_eq!(
        analysis.family_assignments.len(),
        4,
        "an order reversal must not overwrite its sibling assignment"
    );
    assert_eq!(
        analysis.family_assignments.get(&empty).map(String::as_str),
        Some("family-empty")
    );
    assert_eq!(
        analysis.family_assignments.get(&single).map(String::as_str),
        Some("family-single")
    );
    assert_eq!(
        analysis.family_assignments.get(&ab).map(String::as_str),
        Some("family-forward")
    );
    assert_eq!(
        analysis.family_assignments.get(&ba).map(String::as_str),
        Some("family-reversed")
    );

    // An independently rebuilt equal key resolves the same assignment...
    let rebuilt = source_key(
        "obj-1",
        4,
        &[
            ("first", PaintValue::ToolIndex(1)),
            ("second", PaintValue::Flag(false)),
        ],
    );
    assert_eq!(rebuilt, ab);
    assert_eq!(
        analysis
            .family_assignments
            .get(&rebuilt)
            .map(String::as_str),
        Some("family-forward")
    );

    // ...while the reversed chain does not alias it.
    assert_ne!(ab, ba);
    assert_eq!(
        analysis.family_assignments.get(&ba).map(String::as_str),
        Some("family-reversed")
    );

    for expected in [&empty, &single, &ab, &ba] {
        assert!(
            analysis.family_assignments.contains_key(expected),
            "every chain-distinct source identity must be present exactly once"
        );
    }
}

/// Independently authored OLD three-field standalone key JSON omits the chain;
/// it reads as an explicit empty chain and writes every field back out with an
/// explicit `variant_chain: []`. A `SupportSourceKey` omission fixture proves
/// only its own optional-field behavior.
#[test]
fn support_key_json_legacy_missing_chain_normalizes_to_explicit_empty() {
    let legacy_json = r#"{"global_support_layer_index":7,"object_id":"obj-legacy","region_id":9}"#;
    let legacy: SupportGeometryKey =
        serde_json::from_str(legacy_json).expect("legacy three-field key JSON must still parse");
    assert!(
        legacy.variant_chain.is_empty(),
        "an omitted chain normalizes to an exact empty chain"
    );

    let emitted = serde_json::to_value(&legacy).expect("standalone key JSON serializes");
    assert_eq!(
        emitted,
        serde_json::json!({
            "global_support_layer_index": 7,
            "object_id": "obj-legacy",
            "region_id": 9,
            "variant_chain": []
        }),
        "write side emits every field with an explicit empty chain array"
    );

    // Hand-authored populated chain JSON pins exact member order and values.
    let populated_json = r#"{"global_support_layer_index":7,"object_id":"obj-legacy","region_id":9,"variant_chain":[["second",{"ToolIndex":2}],["first",{"Flag":true}]]}"#;
    let populated: SupportGeometryKey =
        serde_json::from_str(populated_json).expect("populated key JSON parses");
    assert_eq!(
        populated.variant_chain,
        chain(&[
            ("second", PaintValue::ToolIndex(2)),
            ("first", PaintValue::Flag(true)),
        ])
    );
    assert_ne!(
        populated.variant_chain,
        chain(&[
            ("first", PaintValue::Flag(true)),
            ("second", PaintValue::ToolIndex(2)),
        ]),
        "the authored member order is authoritative, never sorted"
    );
    assert_eq!(
        serde_json::to_value(&populated).expect("populated standalone key JSON serializes"),
        serde_json::json!({
            "global_support_layer_index": 7,
            "object_id": "obj-legacy",
            "region_id": 9,
            "variant_chain": [["second", {"ToolIndex": 2}], ["first", {"Flag": true}]]
        })
    );

    // SupportSourceKey omission proves only its own optional-field behavior.
    let source_json = r#"{"object_id":"obj-legacy","region_id":9}"#;
    let source: SupportSourceKey =
        serde_json::from_str(source_json).expect("chain-omitted source key JSON parses");
    assert!(source.variant_chain.is_empty());
    assert_eq!(
        serde_json::to_value(&source).expect("standalone source key JSON serializes"),
        serde_json::json!({
            "object_id": "obj-legacy",
            "region_id": 9,
            "variant_chain": []
        })
    );
}

/// Populated actual `SupportGeometryIR`/`SupportAnalysisIR` survive a postcard
/// round-trip with full identity: exact keys/chains/candidates/polygons,
/// non-default settings, and every remaining field. The decoder must consume
/// the whole input.
#[test]
fn support_owner_postcard_populated_maps_preserve_full_identity() {
    let chain_a = [("surface", PaintValue::Flag(true))];
    let chain_ab = [
        ("first", PaintValue::ToolIndex(1)),
        ("second", PaintValue::Flag(false)),
    ];
    let chain_ba = [
        ("second", PaintValue::Flag(false)),
        ("first", PaintValue::ToolIndex(1)),
    ];
    let chain_zero = [("zero", PaintValue::Scalar(0.0))];
    let chain_neg_zero = [("zero", PaintValue::Scalar(-0.0))];

    let mut geometry = SupportGeometryIR {
        support_layer_height_mm: 0.35,
        support_top_z_distance_mm: 0.25,
        ..Default::default()
    };
    geometry
        .entries
        .insert(geometry_key(3, "obj-1", 4, &[]), vec![square(0, 0, 100)]);
    geometry.entries.insert(
        geometry_key(3, "obj-1", 4, &chain_ab),
        vec![square(1_000, 0, 100), square(2_000, 0, 100)],
    );
    geometry.entries.insert(
        geometry_key(u32::MAX, "obj-1", 4, &chain_a),
        vec![square(3_000, 0, 100)],
    );
    geometry.entries.insert(
        geometry_key(3, "obj-1", 4, &chain_ba),
        vec![square(4_000, 0, 100)],
    );

    let mut analysis = SupportAnalysisIR::default();
    analysis.candidates.push(candidate(
        7,
        square(5_000, 0, 100),
        candidate_source("obj-1", 4, 3, 12_345, &chain_ab),
    ));
    analysis.candidates.push(candidate(
        8,
        square(6_000, 0, 100),
        candidate_source("obj-1", 4, 3, 12_345, &chain_ba),
    ));
    analysis.model_occupancy.insert(
        geometry_key(3, "obj-1", 4, &[]),
        vec![square(10_000, 0, 100)],
    );
    analysis.cantilever_surfaces.insert(
        geometry_key(3, "obj-1", 4, &chain_a),
        vec![square(11_000, 0, 100)],
    );
    analysis.termination_surfaces.insert(
        geometry_key(3, "obj-1", 4, &chain_ab),
        vec![square(12_000, 0, 100)],
    );
    analysis.support_territory.insert(
        geometry_key(3, "obj-1", 4, &chain_neg_zero),
        vec![square(13_000, 0, 100)],
    );
    analysis.baseline_feasible_envelope = vec![square(14_000, 0, 100)];
    analysis.shared_settings.insert(
        "support_territory_clearance_mm".to_string(),
        "0.42".to_string(),
    );
    analysis.family_assignments.insert(
        source_key("obj-1", 4, &chain_ab),
        "family-forward".to_string(),
    );
    analysis.family_assignments.insert(
        source_key("obj-1", 4, &chain_ba),
        "family-reversed".to_string(),
    );
    analysis.family_assignments.insert(
        source_key("obj-1", 4, &chain_zero),
        "family-zero".to_string(),
    );

    // Pre-encode: both owners are actually populated with non-default values.
    assert_eq!(
        geometry.schema_version,
        CURRENT_SUPPORT_GEOMETRY_IR_SCHEMA_VERSION
    );
    assert_ne!(geometry.schema_version, SemVer::default());
    assert_eq!(geometry.support_layer_height_mm, 0.35);
    assert_eq!(geometry.support_top_z_distance_mm, 0.25);
    assert_eq!(geometry.entries.len(), 4);

    assert_eq!(
        analysis.schema_version,
        CURRENT_SUPPORT_ANALYSIS_IR_SCHEMA_VERSION
    );
    assert_ne!(analysis.schema_version, SemVer::default());
    assert_eq!(analysis.candidates.len(), 2);
    assert_eq!(
        analysis.candidates[0].source.variant_chain,
        chain(&chain_ab)
    );
    assert_eq!(
        analysis.candidates[1].source.variant_chain,
        chain(&chain_ba)
    );
    assert_ne!(analysis.candidates[0].source, analysis.candidates[1].source);
    assert_eq!(analysis.model_occupancy.len(), 1);
    assert_eq!(analysis.cantilever_surfaces.len(), 1);
    assert_eq!(analysis.termination_surfaces.len(), 1);
    assert_eq!(analysis.support_territory.len(), 1);
    assert_eq!(analysis.family_assignments.len(), 3);
    assert_eq!(
        analysis.baseline_feasible_envelope,
        vec![square(14_000, 0, 100)]
    );
    assert_eq!(
        analysis
            .shared_settings
            .get("support_territory_clearance_mm")
            .map(String::as_str),
        Some("0.42")
    );

    // SupportGeometryIR: whole-object equality plus independent key lookups.
    let geometry_bytes =
        postcard::to_allocvec(&geometry).expect("SupportGeometryIR postcard encode");
    let (decoded_geometry, geometry_rest) =
        postcard::take_from_bytes::<SupportGeometryIR>(&geometry_bytes)
            .expect("SupportGeometryIR postcard decode");
    assert!(
        geometry_rest.is_empty(),
        "postcard decoder must consume the whole SupportGeometryIR input"
    );
    assert_eq!(decoded_geometry, geometry);
    assert_eq!(
        decoded_geometry
            .entries
            .get(&geometry_key(3, "obj-1", 4, &[])),
        Some(&vec![square(0, 0, 100)])
    );
    assert_eq!(
        decoded_geometry
            .entries
            .get(&geometry_key(3, "obj-1", 4, &chain_ab)),
        Some(&vec![square(1_000, 0, 100), square(2_000, 0, 100)])
    );
    assert_eq!(
        decoded_geometry
            .entries
            .get(&geometry_key(u32::MAX, "obj-1", 4, &chain_a)),
        Some(&vec![square(3_000, 0, 100)])
    );
    assert_eq!(
        decoded_geometry
            .entries
            .get(&geometry_key(3, "obj-1", 4, &chain_ba)),
        Some(&vec![square(4_000, 0, 100)])
    );

    // SupportAnalysisIR: whole-object equality plus full-identity lookups.
    let analysis_bytes =
        postcard::to_allocvec(&analysis).expect("SupportAnalysisIR postcard encode");
    let (decoded_analysis, analysis_rest) =
        postcard::take_from_bytes::<SupportAnalysisIR>(&analysis_bytes)
            .expect("SupportAnalysisIR postcard decode");
    assert!(
        analysis_rest.is_empty(),
        "postcard decoder must consume the whole SupportAnalysisIR input"
    );
    assert_eq!(decoded_analysis, analysis);
    assert_eq!(
        decoded_analysis
            .model_occupancy
            .get(&geometry_key(3, "obj-1", 4, &[])),
        Some(&vec![square(10_000, 0, 100)])
    );
    assert_eq!(
        decoded_analysis
            .cantilever_surfaces
            .get(&geometry_key(3, "obj-1", 4, &chain_a)),
        Some(&vec![square(11_000, 0, 100)])
    );
    assert_eq!(
        decoded_analysis
            .termination_surfaces
            .get(&geometry_key(3, "obj-1", 4, &chain_ab)),
        Some(&vec![square(12_000, 0, 100)])
    );
    assert_eq!(
        decoded_analysis
            .support_territory
            .get(&geometry_key(3, "obj-1", 4, &chain_neg_zero)),
        Some(&vec![square(13_000, 0, 100)])
    );
    assert_eq!(
        decoded_analysis
            .family_assignments
            .get(&source_key("obj-1", 4, &chain_ba))
            .map(String::as_str),
        Some("family-reversed"),
        "chain order reversal survives postcard as a distinct assignment"
    );
    assert_eq!(
        decoded_analysis
            .family_assignments
            .get(&source_key("obj-1", 4, &chain_zero))
            .map(String::as_str),
        Some("family-zero")
    );
    assert_eq!(
        decoded_analysis
            .shared_settings
            .get("support_territory_clearance_mm")
            .map(String::as_str),
        Some("0.42")
    );
    assert_eq!(
        decoded_analysis.schema_version,
        CURRENT_SUPPORT_ANALYSIS_IR_SCHEMA_VERSION
    );
}

/// Populated composite-key owners reject ordinary JSON object-map encoding.
/// Standalone key JSON remains the supported representation (see the key JSON
/// test above). This asserts the measured serde_json behavior: no stringify or
/// entry-list codec adapter is introduced.
#[test]
fn support_composite_map_json_rejects_populated_keys() {
    let mut geometry = SupportGeometryIR::default();
    geometry
        .entries
        .insert(geometry_key(3, "obj-1", 4, &[]), vec![square(0, 0, 100)]);
    let geometry_err = serde_json::to_string(&geometry).expect_err(
        "populated composite-key SupportGeometryIR must reject ordinary JSON object-map encoding",
    );
    assert!(
        !geometry_err.to_string().is_empty(),
        "rejection must carry a message"
    );

    let mut analysis = SupportAnalysisIR::default();
    analysis.model_occupancy.insert(
        geometry_key(3, "obj-1", 4, &[]),
        vec![square(1_000, 0, 100)],
    );
    analysis
        .family_assignments
        .insert(source_key("obj-1", 4, &[]), "family".to_string());
    let analysis_err = serde_json::to_string(&analysis).expect_err(
        "populated composite-key SupportAnalysisIR must reject ordinary JSON object-map encoding",
    );
    assert!(
        !analysis_err.to_string().is_empty(),
        "rejection must carry a message"
    );

    // The empty owner still serializes: the limitation is the composite key,
    // not the owner shape.
    assert_eq!(
        serde_json::to_value(SupportGeometryIR::default()).expect("empty owner JSON serializes"),
        serde_json::json!({
            "schema_version": {"major": 2, "minor": 0, "patch": 0},
            "support_layer_height_mm": 0.0,
            "support_top_z_distance_mm": 0.0,
            "entries": {}
        })
    );
}
