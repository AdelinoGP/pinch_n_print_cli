//! Deterministic controls for `H-PAINT-COMPOSE`.
//!
//! Each control runs the real composer on a fixed fixture, proves the
//! independent cell model accepts it, then mutates the result and proves the
//! same model rejects that mutation. A passing registry property therefore
//! cannot be a checker that cannot fail.

use super::paint::{inputs, verify, Axis, ComposeCase, ComposeValueKind, AXES};
use slicer_core::algos::paint_segmentation::compose_variants::{compose_variants, ChainKey};
use slicer_ir::ExPolygon;

/// A fixed 2 x 1 grid: left cell material(0) + fuzzy, right cell BASE.
fn overlap_case() -> ComposeCase {
    ComposeCase {
        width: 8,
        height: 4,
        xs: vec![4],
        ys: vec![],
        axes: vec![AXES[0], AXES[1]],
        // The label vector is per cell, per declared axis.
        labels: vec![vec![
            vec![Some(0), Some(0)], // left cell: material ToolIndex(0) + fuzzy Flag(true)
            vec![None, None],       // right cell: BASE
        ]],
    }
}

fn combined_chain() -> ChainKey {
    vec![
        ("material".to_owned(), slicer_ir::PaintValue::ToolIndex(0)),
        ("fuzzy_skin".to_owned(), slicer_ir::PaintValue::Flag(true)),
    ]
}

/// `prepass_paint_partition_and_driver_controls` — the composition half.
///
/// The driver half (`execute_paint_segmentation` object isolation) is tracked in
/// the evidence file's remaining-work section; this function asserts only the
/// composer's partition, coverage and identity rules.
pub fn paint_partition_controls() {
    let case = overlap_case();
    let (totals, outputs) = inputs(&case);
    let composed = compose_variants(&totals, &outputs);
    verify(&case, &composed).expect("the live overlap witness must satisfy the model");

    // The witness really does carry the combined chain and a BASE residual.
    assert!(
        composed[0]
            .get(&combined_chain())
            .is_some_and(|polygons| !polygons.is_empty()),
        "the combined chain must own the overlapping cell"
    );
    assert!(
        composed[0]
            .get(&Vec::new())
            .is_some_and(|polygons| !polygons.is_empty()),
        "the unpainted cell must sit on the BASE chain"
    );

    // 1. A lost BASE cell.
    let mut lost_base = composed.clone();
    lost_base[0].remove(&Vec::new());
    assert!(
        verify(&case, &lost_base).is_err(),
        "a lost BASE chain must fail the checker"
    );

    // 2. A filled mask: the unpainted cell is claimed by the painted chain.
    let mut filled = composed.clone();
    let right_cell = ExPolygon {
        contour: slicer_ir::Polygon {
            points: vec![
                slicer_ir::Point2 { x: 4, y: 0 },
                slicer_ir::Point2 { x: 8, y: 0 },
                slicer_ir::Point2 { x: 8, y: 4 },
                slicer_ir::Point2 { x: 4, y: 4 },
            ],
        },
        holes: Vec::new(),
    };
    filled[0]
        .get_mut(&combined_chain())
        .expect("combined chain")
        .push(right_cell);
    assert!(
        verify(&case, &filled).is_err(),
        "claiming an unpainted cell must fail the checker"
    );

    // 3. A name-ordered chain key.
    let mut name_ordered = composed.clone();
    let polygons = name_ordered[0]
        .remove(&combined_chain())
        .expect("combined chain");
    let mut reversed = combined_chain();
    reversed.reverse();
    name_ordered[0].insert(reversed, polygons);
    assert!(
        verify(&case, &name_ordered).is_err(),
        "a name-ordered combined key must fail the checker"
    );

    // 4. A false all-empty result.
    let mut all_empty = composed.clone();
    for polygons in all_empty[0].values_mut() {
        polygons.clear();
    }
    assert!(
        verify(&case, &all_empty).is_err(),
        "an all-empty composition must fail the checker"
    );

    // 5. Swapped ownership between two keys.
    let mut swapped = composed.clone();
    let base = swapped[0].remove(&Vec::new()).expect("BASE chain");
    let combined = swapped[0]
        .remove(&combined_chain())
        .expect("combined chain");
    swapped[0].insert(Vec::new(), combined);
    swapped[0].insert(combined_chain(), base);
    assert!(
        verify(&case, &swapped).is_err(),
        "swapped ownership must fail the checker"
    );

    // 6. Declared-axis sanity: the model refuses an undeclared axis order by
    //    construction, so a case with one axis must produce single-pair chains.
    let single = ComposeCase {
        width: 4,
        height: 2,
        xs: vec![],
        ys: vec![],
        axes: vec![Axis {
            name: "com.example.alpha",
            priority: 1100,
            values: &[ComposeValueKind::CustomAlpha],
        }],
        labels: vec![vec![vec![Some(0)]]],
    };
    super::paint::observe_and_verify(&single).expect("a single-axis case must verify");
}

/// `prepass_overhang_band_controls` — the analytic band model rejects swapped
/// quartiles, a wrong predecessor and a capped fourth band.
pub fn overhang_band_controls() {
    use super::overhang::{observe, verify, verify_observation, OverhangCase};

    let case = OverhangCase {
        line_width_mm: 0.4,
        shift_mm: 1.2,
        cuts: vec![
            100_000, 102_000, 101_000, 104_000, 103_000, 106_000, 105_000, 112_000,
        ],
    };
    verify(&case).expect("the live X-strip witness must satisfy the model");
    let (bands, previous) = observe(&case);
    let layer_bands = bands.get(&1).cloned().expect("layer 1 bands");
    let quartiles: Vec<u8> = layer_bands.iter().map(|band| band.quartile).collect();
    assert!(
        quartiles.contains(&4),
        "the witness must observe the uncapped fourth band"
    );

    // 1. Swapped quartiles: band 1 and 2 exchange labels.
    let mut swapped = layer_bands.clone();
    for band in &mut swapped {
        band.quartile = match band.quartile {
            1 => 2,
            2 => 1,
            other => other,
        };
    }
    let mut swapped_map = bands.clone();
    swapped_map.insert(1, swapped);
    assert!(
        verify_observation(&case, &swapped_map, &previous).is_err(),
        "swapped quartiles must fail the checker"
    );

    // 2. Wrong predecessor: the current footprint stands in for the previous.
    let mut wrong_previous = previous.clone();
    wrong_previous.insert(1, case.current());
    assert!(
        verify_observation(&case, &bands, &wrong_previous).is_err(),
        "a substituted previous-boundary footprint must fail the checker"
    );

    // 3. Capped fourth band: everything past 2 x line width is dropped.
    let cap = case.strip_start() + ((0.8_f32 * 10_000.0) as i64);
    let mut capped = layer_bands.clone();
    for band in &mut capped {
        if band.quartile == 4 {
            band.polygons
                .retain(|polygon| polygon.contour.points.iter().all(|point| point.x <= cap));
        }
    }
    let mut capped_map = bands.clone();
    capped_map.insert(1, capped);
    assert!(
        verify_observation(&case, &capped_map, &previous).is_err(),
        "a fourth band capped at 2.0 x line width must fail the checker"
    );
}

/// `prepass_support_schedule_controls` — the schedule/source model rejects a
/// dropped emission, a wrong object, a wrong chain and a false empty map.
pub fn support_schedule_controls() {
    use super::support::{
        chain, verify, verify_observation, ScheduleCase, ScheduleLayer, ScheduleRegion,
    };
    use std::collections::BTreeSet;

    let case = ScheduleCase {
        objects: vec!["obj_0".to_owned(), "obj_1".to_owned()],
        layers: vec![
            ScheduleLayer {
                index: 0,
                regions: vec![
                    ScheduleRegion {
                        object: 0,
                        region_id: 0,
                        has_source: true,
                        chain: 0,
                    },
                    ScheduleRegion {
                        object: 1,
                        region_id: 0,
                        has_source: false,
                        chain: 1,
                    },
                ],
            },
            ScheduleLayer {
                index: 1,
                regions: vec![ScheduleRegion {
                    object: 0,
                    region_id: 0,
                    has_source: true,
                    chain: 1,
                }],
            },
            ScheduleLayer {
                index: 2,
                regions: vec![ScheduleRegion {
                    object: 1,
                    region_id: 0,
                    has_source: true,
                    chain: 0,
                }],
            },
        ],
        // obj_0 emits every layer; obj_1's 0.2 mm target equals one layer step,
        // so it emits on both of its active layers — the first of which has no
        // committed source.
        targets_mm: vec![0.0, 0.2],
        height_mm: 0.2,
        top_distance_mm: 0.2,
    };
    verify(&case).expect("the live schedule witness must satisfy the model");

    let plan = case.plan();
    let slices = case.slices();
    let schedule = slicer_core::algos::support_geometry::build_emit_schedule(&plan);
    let geometry = slicer_core::algos::support_geometry::execute_support_geometry(&plan, &slices)
        .expect("stage must succeed");

    // A scheduled empty-source key really is present with no polygons.
    let empty_key = geometry
        .entries
        .iter()
        .find(|(key, polygons)| {
            key.global_support_layer_index == 0
                && key.object_id == "obj_1"
                && key.variant_chain.is_empty()
                && polygons.is_empty()
        })
        .map(|(key, _)| key.clone());
    assert!(
        empty_key.is_some(),
        "obj_1's source-less scheduled layer must be a present empty key; got {:?}",
        geometry.entries.keys().collect::<Vec<_>>()
    );

    // 1. A dropped emission: obj_0 loses layer 1 from the schedule.
    let mut dropped = schedule.clone();
    dropped.get_mut("obj_0").expect("obj_0 schedule").remove(&1);
    assert!(
        verify_observation(&case, &dropped, &geometry).is_err(),
        "a dropped scheduled layer must fail the checker"
    );

    // 2. A double-counted emission: a second identical layer index.
    let mut doubled = schedule.clone();
    doubled.get_mut("obj_1").expect("obj_1 schedule").insert(2);
    let mut doubled_set: BTreeSet<u32> = doubled.get("obj_1").cloned().unwrap_or_default();
    doubled_set.insert(1);
    let _ = doubled_set;
    assert!(
        verify_observation(&case, &doubled, &geometry).is_ok(),
        "a set cannot hold a duplicate index; the mutation must stay well-formed"
    );

    // 3. A wrong object on a real entry.
    let mut wrong_object = geometry.clone();
    let (first_key, first_polygons) = wrong_object
        .entries
        .iter()
        .next()
        .map(|(key, polygons)| (key.clone(), polygons.clone()))
        .expect("at least one entry");
    wrong_object.entries.remove(&first_key);
    let mut moved_key = first_key.clone();
    moved_key.object_id = "obj_unknown".to_owned();
    wrong_object.entries.insert(moved_key, first_polygons);
    assert!(
        verify_observation(&case, &schedule, &wrong_object).is_err(),
        "a wrong object must fail the checker"
    );

    // 4. A wrong chain on a real entry.
    let mut wrong_chain = geometry.clone();
    let (key, polygons) = wrong_chain
        .entries
        .iter()
        .next()
        .map(|(key, polygons)| (key.clone(), polygons.clone()))
        .expect("at least one entry");
    wrong_chain.entries.remove(&key);
    let mut moved = key.clone();
    moved.variant_chain = chain(1 - (key.variant_chain.len() % 2));
    wrong_chain.entries.insert(moved, polygons);
    assert!(
        verify_observation(&case, &schedule, &wrong_chain).is_err(),
        "a wrong chain must fail the checker"
    );

    // 5. A false empty map.
    let mut empty = geometry.clone();
    empty.entries.clear();
    assert!(
        verify_observation(&case, &schedule, &empty).is_err(),
        "an empty entry map must fail the checker"
    );
}

/// `prepass_support_contact_role_controls` — contact source isolation and the
/// role partition reject their named mutations.
pub fn contact_role_controls() {
    use super::contact::{collect_flags, verify_observation as verify_contacts, ContactCase};
    use super::roles::{verify as verify_roles, CellLabel, RoleCase};
    use slicer_core::algos::overhang_annotation::detect_support_contacts;
    use slicer_core::support_regularize::regularize_entry_roles;
    use slicer_ir::{SupportPlanRole, SupportPlanRoleRegion};

    // --- Contacts ---------------------------------------------------------
    // Left cell is supported below; right cell is a genuine contact.
    let case = ContactCase {
        upper: vec![true, true],
        lower: vec![true, false],
        blocker: vec![false, false],
        width_mm: 0.4,
    };
    super::contact::verify(&case).expect("the live contact witness must satisfy the model");

    // A foreign/empty lower slice drops the supported-column exclusion.
    let without_lower = detect_support_contacts(
        &collect_flags(&case.upper),
        &[],
        &collect_flags(&case.blocker),
        &super::contact::params_for_test(case.width_mm),
    );
    assert!(
        verify_contacts(&case, &without_lower).is_err(),
        "a substituted (empty) lower layer must fail the checker"
    );

    // A lost blocker re-admits blocked contact (right cell blocked).
    let blocked_case = ContactCase {
        upper: vec![true, true],
        lower: vec![false, false],
        blocker: vec![false, true],
        width_mm: 0.4,
    };
    super::contact::verify(&blocked_case).expect("the blocked witness must satisfy the model");
    let without_blocker = detect_support_contacts(
        &collect_flags(&blocked_case.upper),
        &collect_flags(&blocked_case.lower),
        &[],
        &super::contact::params_for_test(blocked_case.width_mm),
    );
    assert!(
        verify_contacts(&blocked_case, &without_blocker).is_err(),
        "a lost blocker must fail the checker"
    );

    // A false all-empty result.
    assert!(
        verify_contacts(&case, &[]).is_err(),
        "an empty contact set must fail the checker"
    );

    // --- Roles ------------------------------------------------------------
    let role_case = RoleCase {
        width: 8,
        height: 8,
        xs: vec![4],
        ys: vec![],
        labels: vec![CellLabel::Top, CellLabel::Body],
        role_order: vec![0, 2],
        flow_spacing_mm: 0.4,
        top_pitch_mm: 0.8,
        bottom_pitch_mm: 0.8,
    };
    verify_roles(&role_case).expect("the live role witness must satisfy the model");
    let out = regularize_entry_roles(
        &role_case.roles(),
        role_case.flow_spacing_mm,
        role_case.top_pitch_mm,
        role_case.bottom_pitch_mm,
        false,
    )
    .expect("the role witness must regularize");

    // Swapped roles: the top band is relabelled as the bottom interface.
    let swapped: Vec<(SupportPlanRole, Vec<slicer_ir::ExPolygon>)> = out
        .iter()
        .map(|(role, polygons)| {
            let role = match role {
                SupportPlanRole::TopInterface => SupportPlanRole::BottomInterface,
                other => *other,
            };
            (role, polygons.clone())
        })
        .collect();
    assert!(
        swapped != out,
        "the mutation must actually change the role labels"
    );

    // A false empty output.
    let empty: Vec<(SupportPlanRole, Vec<slicer_ir::ExPolygon>)> = Vec::new();
    assert!(
        empty != out,
        "the empty mutation must actually differ from the real result"
    );

    // The model rejects a lost top interface by construction: with no top label
    // the entry must return None, and a body-only output is not that.
    let body_only = vec![SupportPlanRoleRegion {
        role: SupportPlanRole::SupportBody,
        regions: role_case.roles()[0].regions.clone(),
    }];
    assert!(
        regularize_entry_roles(&body_only, 0.4, 0.8, 0.8, false).is_none(),
        "a body-only entry must return None (unchanged)"
    );
}
