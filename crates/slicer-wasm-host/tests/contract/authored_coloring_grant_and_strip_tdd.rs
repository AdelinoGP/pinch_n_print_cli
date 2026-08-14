//! Contract: the authored-coloring grant is two-sided, and the marshal/commit
//! boundary is the authoritative enforcement point for authored `tool_index`.
//!
//! Packet 226. A module may author a per-path tool only when BOTH sides agree:
//! it discloses `claim:authored-coloring` in its manifest, AND at least one
//! fill-role claim it actually holds for the region is listed in the profile's
//! `fill_authored_coloring` key. Anything else — no grant, an ungranted region,
//! or an out-of-range index — is silently stripped to `None` (meaning "the host
//! resolves the region tool") by `convert_infill_output`. Stripping is never an
//! error: the call still returns `Ok`.

#![allow(missing_docs)]

use slicer_wasm_host::host::{ExtrusionPath3d, ExtrusionRole, Point3WithWidth};
use slicer_wasm_host::marshal::accumulators::InfillOutputCollected;
use slicer_wasm_host::marshal::OriginId;
use slicer_wasm_host::marshal::{
    authored_coloring_granted, convert_infill_output, AuthoredColoringContext,
};

const OBJECT_ID: &str = "cube";
const REGION_ID: u64 = 7;

fn path_with_tool(tool_index: Option<u32>) -> ExtrusionPath3d {
    ExtrusionPath3d {
        points: vec![
            // exhaustive: Point3WithWidth has no Default; fixture specifies all geometry fields
            Point3WithWidth {
                x: 0.0,
                y: 0.0,
                z: 0.2,
                width: 0.4,
                flow_factor: 1.0,
                overhang_quartile: None,
                dist_to_top_mm: 0.0,
                overhang_distance_mm: None,
            },
            // exhaustive: Point3WithWidth has no Default; fixture specifies all geometry fields
            Point3WithWidth {
                x: 10.0,
                y: 0.0,
                z: 0.2,
                width: 0.4,
                flow_factor: 1.0,
                overhang_quartile: None,
                dist_to_top_mm: 0.0,
                overhang_distance_mm: None,
            },
        ],
        role: ExtrusionRole::SparseInfill,
        speed_factor: 1.0,
        tool_index,
    }
}

/// One tagged sparse path carrying `tool_index`, attributed to
/// `(OBJECT_ID, REGION_ID)`.
fn collected_with_tool(tool_index: Option<u32>) -> InfillOutputCollected {
    InfillOutputCollected {
        sparse_paths: vec![path_with_tool(tool_index)],
        sparse_path_origins: vec![Some(OriginId {
            object_id: OBJECT_ID.to_string(),
            region_id: REGION_ID,
        })],
        ..Default::default()
    }
}

fn granting_ctx(tool_count: u32) -> AuthoredColoringContext {
    AuthoredColoringContext {
        tool_count,
        granted_regions: [(OBJECT_ID.to_string(), REGION_ID)].into_iter().collect(),
    }
}

/// Read back the single committed sparse path's `tool_index`.
fn committed_tool_index(ir: &slicer_ir::InfillIR) -> Option<u32> {
    assert_eq!(ir.regions.len(), 1, "expected exactly one committed region");
    let region = &ir.regions[0];
    assert_eq!(
        region.sparse_infill.len(),
        1,
        "expected exactly one committed sparse path"
    );
    region.sparse_infill[0].tool_index
}

// ── convert_infill_output enforcement ────────────────────────────────────

#[test]
fn granted_region_in_range_keeps_authored_tool_index() {
    let ctx = granting_ctx(4);
    let ir = convert_infill_output(&collected_with_tool(Some(2)), 0, Some(&ctx))
        .expect("granted authored tool must commit successfully");
    assert_eq!(
        committed_tool_index(&ir),
        Some(2),
        "a granted region with an in-range authored tool must KEEP tool_index = Some(2); \
         stripping it here would mean the grant is inert"
    );
}

#[test]
fn ungranted_module_authored_tool_is_stripped_to_none() {
    // `None` context = no module holds the grant on this dispatch.
    let result = convert_infill_output(&collected_with_tool(Some(2)), 0, None);
    let ir = result.expect("stripping must be silent — still Ok, never an error");
    assert_eq!(
        committed_tool_index(&ir),
        None,
        "an ungranted module's authored tool must be stripped to None \
         (host resolves the region tool)"
    );
}

#[test]
fn granted_region_out_of_range_tool_is_stripped_to_none() {
    let ctx = granting_ctx(2); // valid tools are 0 and 1
    let ir = convert_infill_output(&collected_with_tool(Some(2)), 0, Some(&ctx))
        .expect("out-of-range strip must be silent — still Ok, never an error");
    assert_eq!(
        committed_tool_index(&ir),
        None,
        "tool_index >= tool_count must be stripped even for a granted region"
    );
}

#[test]
fn granted_but_different_region_is_stripped_to_none() {
    let ctx = AuthoredColoringContext {
        tool_count: 4,
        granted_regions: [("other-object".to_string(), REGION_ID)]
            .into_iter()
            .collect(),
    };
    let ir = convert_infill_output(&collected_with_tool(Some(2)), 0, Some(&ctx))
        .expect("strip must be silent");
    assert_eq!(
        committed_tool_index(&ir),
        None,
        "the grant is per-region: a region absent from granted_regions is denied"
    );
}

#[test]
fn absent_tool_index_is_unchanged_under_a_grant() {
    let ctx = granting_ctx(4);
    let ir = convert_infill_output(&collected_with_tool(None), 0, Some(&ctx)).expect("must commit");
    assert_eq!(
        committed_tool_index(&ir),
        None,
        "paths that author no tool are passed through untouched"
    );
}

// ── authored_coloring_granted predicate ──────────────────────────────────

const CLAIM: &str = "claim:authored-coloring";

#[test]
fn grant_requires_both_disclosure_and_config_listing() {
    let held = vec!["claim:fill-sparse".to_string()];
    let listed = vec!["claim:fill-sparse".to_string()];
    let disclosed = vec![CLAIM.to_string(), "claim:fill-sparse".to_string()];
    assert!(
        authored_coloring_granted(&held, &listed, &disclosed),
        "both sides agree: the grant must be issued"
    );
}

#[test]
fn disclosed_but_not_listed_in_config_is_denied() {
    let held = vec!["claim:fill-sparse".to_string()];
    // Config opts a DIFFERENT fill claim into authored coloring.
    let listed = vec!["claim:fill-top".to_string()];
    let disclosed = vec![CLAIM.to_string(), "claim:fill-sparse".to_string()];
    assert!(
        !authored_coloring_granted(&held, &listed, &disclosed),
        "disclosure alone must not grant: the held fill claim is not in fill_authored_coloring"
    );
}

#[test]
fn listed_in_config_but_not_disclosed_is_denied() {
    let held = vec!["claim:fill-sparse".to_string()];
    let listed = vec!["claim:fill-sparse".to_string()];
    // Manifest never discloses `claim:authored-coloring`.
    let disclosed = vec!["claim:fill-sparse".to_string()];
    assert!(
        !authored_coloring_granted(&held, &listed, &disclosed),
        "config opt-in alone must not grant: the module never disclosed the claim"
    );
}

#[test]
fn empty_inputs_are_denied_and_do_not_panic() {
    assert!(!authored_coloring_granted(&[], &[], &[]));
    assert!(!authored_coloring_granted(&[], &[], &[CLAIM.to_string()]));
    assert!(!authored_coloring_granted(
        &["claim:fill-sparse".to_string()],
        &[],
        &[CLAIM.to_string()]
    ));
}
