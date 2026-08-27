#![allow(missing_docs)]

use slicer_core::support_regularize::regularize_entry_roles;
use slicer_ir::{mm_to_units, ExPolygon, Point2, Polygon, SupportPlanRole, SupportPlanRoleRegion};

fn square(min_mm: f32, max_mm: f32) -> ExPolygon {
    let lo = mm_to_units(min_mm);
    let hi = mm_to_units(max_mm);
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 { x: lo, y: lo },
                Point2 { x: hi, y: lo },
                Point2 { x: hi, y: hi },
                Point2 { x: lo, y: hi },
            ],
        },
        holes: Vec::new(),
    }
}

fn ring_area(pts: &[Point2]) -> f64 {
    let mut acc = 0.0_f64;
    for i in 0..pts.len() {
        let a = pts[i];
        let b = pts[(i + 1) % pts.len()];
        acc += (a.x as f64) * (b.y as f64) - (b.x as f64) * (a.y as f64);
    }
    acc / 2.0
}

fn area_mm2(polys: &[ExPolygon]) -> f64 {
    polys
        .iter()
        .map(|e| {
            let mut a = ring_area(&e.contour.points).abs();
            for h in &e.holes {
                a -= ring_area(&h.points).abs();
            }
            a
        })
        .sum::<f64>()
        / (slicer_ir::UNITS_PER_MM * slicer_ir::UNITS_PER_MM)
}

fn role_region(role: SupportPlanRole, regions: Vec<ExPolygon>) -> SupportPlanRoleRegion {
    SupportPlanRoleRegion { role, regions }
}

#[test]
fn regularized_interface_never_exceeds_layer_area() {
    let src = vec![role_region(
        SupportPlanRole::TopInterface,
        vec![square(0.0, 10.0)],
    )];
    let out = regularize_entry_roles(&src, 0.35, 0.75, 0.75, true).expect("regularized");
    let iface: Vec<ExPolygon> = out
        .iter()
        .filter(|(r, _)| *r == SupportPlanRole::TopInterface)
        .flat_map(|(_, p)| p.iter().cloned())
        .collect();
    assert!(!iface.is_empty(), "roof must survive regularization");
    assert!(area_mm2(&iface) <= 100.0 + 1e-3);
}

#[test]
fn small_feature_interface_never_loses_coverage() {
    let src = vec![role_region(
        SupportPlanRole::TopInterface,
        vec![square(0.0, 0.15)],
    )];
    let out = regularize_entry_roles(&src, 0.35, 0.75, 0.75, true).expect("regularized");
    assert!(!out.is_empty());
    let covered: Vec<ExPolygon> = out.iter().flat_map(|(_, p)| p.iter().cloned()).collect();
    assert!((area_mm2(&covered) - 0.0225).abs() < 1e-4);
}

#[test]
fn grid_style_does_not_smooth() {
    let src = vec![role_region(
        SupportPlanRole::TopInterface,
        vec![square(0.0, 0.15)],
    )];
    let out = regularize_entry_roles(&src, 0.35, 0.75, 0.75, false).expect("regularized");
    let iface: Vec<ExPolygon> = out
        .iter()
        .filter(|(r, _)| *r == SupportPlanRole::TopInterface)
        .flat_map(|(_, p)| p.iter().cloned())
        .collect();
    assert!((area_mm2(&iface) - 0.0225).abs() < 1e-4);
}

#[test]
fn body_only_entry_is_untouched() {
    let src = vec![role_region(
        SupportPlanRole::SupportBody,
        vec![square(0.0, 10.0)],
    )];
    assert!(regularize_entry_roles(&src, 0.35, 0.75, 0.75, true).is_none());
}

#[test]
fn roof_and_floor_do_not_overlap() {
    let src = vec![
        role_region(SupportPlanRole::TopInterface, vec![square(0.0, 10.0)]),
        role_region(SupportPlanRole::BottomInterface, vec![square(0.0, 10.0)]),
    ];
    let out = regularize_entry_roles(&src, 0.35, 0.75, 0.75, true).expect("regularized");
    let total: f64 = out.iter().map(|(_, p)| area_mm2(p)).sum();
    assert!(total <= 100.0 + 1e-3);
}
