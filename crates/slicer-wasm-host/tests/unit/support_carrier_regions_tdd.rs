//! Ticket 19: support carrier regions — a family renderer must receive a
//! region for every `(object, region)` its plan claims on a layer, even when
//! the slice has no geometry there (support lives in free air).

use slicer_ir::{
    ExPolygon, Point2, Polygon, SliceIR, SlicedRegion, SupportPlanEntry, SupportPlanIR,
    SupportPlanRole, SupportPlanRoleRegion,
};
use slicer_wasm_host::dispatch::support_carrier_regions;

fn square() -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 { x: 0, y: 0 },
                Point2 { x: 100, y: 0 },
                Point2 { x: 100, y: 100 },
                Point2 { x: 0, y: 100 },
            ],
        },
        holes: Vec::new(),
    }
}

fn entry(family: &str, region_id: u64, layer: i32, body: Option<ExPolygon>) -> SupportPlanEntry {
    // exhaustive: support-plan identity fixture; SupportPlanEntry has no Default impl and FRU would let a new plan field default silently
    SupportPlanEntry {
        global_layer_index: layer,
        object_id: "obj".into(),
        region_id,
        family_id: family.into(),
        demand_ids: vec!["d".into()],
        body_ids: vec!["b".into()],
        anchor_layer_index: 0,
        anchor_z: 0,
        roles: body
            .into_iter()
            .map(|region| SupportPlanRoleRegion {
                role: SupportPlanRole::SupportBody,
                regions: vec![region],
            })
            .collect(),
        skeleton: None,
        capabilities: Vec::new(),
        provenance: Vec::new(),
        decline_reason: None,
    }
}

fn claims(family: &str) -> Vec<String> {
    vec![
        "support-generator".into(),
        format!("support-family:{family}"),
    ]
}

#[test]
fn carrier_minted_for_planned_region_absent_from_slice() {
    let plan = SupportPlanIR {
        entries: vec![
            entry("traditional", 7, 3, Some(square())),
            entry("traditional", 7, 3, Some(square())), // duplicate identity: one carrier
            entry("traditional", 7, 4, Some(square())), // other layer
            entry("tree", 3, 3, Some(square())),        // other family
            entry("traditional", 9, 3, None),           // no geometry: nothing to render
        ],
        ..SupportPlanIR::default()
    };
    let slice = SliceIR {
        global_layer_index: 3,
        regions: vec![SlicedRegion {
            object_id: "obj".into(),
            region_id: 3,
            polygons: vec![square()],
            effective_layer_height: 0.25,
            ..SlicedRegion::default()
        }],
        ..SliceIR::default()
    };
    let carriers = support_carrier_regions(&claims("traditional"), 3, Some(&plan), Some(&slice));
    assert_eq!(carriers.len(), 1, "{carriers:?}");
    assert_eq!(carriers[0].object_id, "obj");
    assert_eq!(carriers[0].region_id, 7);
    assert!(carriers[0].polygons.is_empty());
    assert_eq!(
        carriers[0].effective_layer_height, 0.25,
        "borrowed from the sibling region of the same object"
    );

    // The tree module gets nothing: its region is already in the slice.
    assert!(support_carrier_regions(&claims("tree"), 3, Some(&plan), Some(&slice)).is_empty());
    // Orca-style alias on the claim resolves to the same family.
    assert_eq!(
        support_carrier_regions(&claims("normal"), 3, Some(&plan), Some(&slice)).len(),
        1
    );
}

#[test]
fn no_carrier_without_family_claim_or_plan() {
    let plan = SupportPlanIR {
        entries: vec![entry("traditional", 7, 3, Some(square()))],
        ..SupportPlanIR::default()
    };
    assert!(support_carrier_regions(&["infill".to_string()], 3, Some(&plan), None).is_empty());
    assert!(support_carrier_regions(&claims("traditional"), 3, None, None).is_empty());
    let mut declined = plan.clone();
    declined.entries[0].decline_reason = Some(slicer_ir::SupportPlanDeclineReason::NoRoute);
    assert!(support_carrier_regions(&claims("traditional"), 3, Some(&declined), None).is_empty());
}
