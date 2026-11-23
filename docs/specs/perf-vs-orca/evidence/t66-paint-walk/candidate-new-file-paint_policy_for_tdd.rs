//! Value contracts for `PaintRegionLayerView::paint_policy_for` under the
//! annotations-empty fast path (wayfinder ticket 66).
//!
//! The fast path returns `DefaultEligible` immediately when no region in the
//! attached slice carries a support-blocker/enforcer paint value. Every
//! expected value below is the answer the previous walk-always implementation
//! produced for the same hand-built geometry, so an over-broad predicate that
//! skipped a painted case would flip `Blocked`/`Enforced` back to
//! `DefaultEligible` and fail these tests.
//!
//! Geometry: the probe always lives at [-1, 1] mm; "covering" regions span
//! [-10, 10] mm and "far" regions sit at [100, 110] mm.

use std::collections::HashMap;
use std::sync::Arc;

use slicer_ir::{ExPolygon, PaintSemantic, PaintValue, Point2, Polygon, SliceIR, SlicedRegion};
use slicer_sdk::traits::{PaintRegionLayerView, SupportPaintPolicy};

type Annotations = HashMap<PaintSemantic, Vec<Vec<Option<PaintValue>>>>;

fn rect(x0: f32, y0: f32, x1: f32, y1: f32) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(x0, y0),
                Point2::from_mm(x1, y0),
                Point2::from_mm(x1, y1),
                Point2::from_mm(x0, y1),
            ],
        },
        holes: Vec::new(),
    }
}

fn full_paint(semantic: PaintSemantic) -> Annotations {
    let mut map = HashMap::new();
    map.insert(semantic, vec![vec![Some(PaintValue::Flag(true))]]);
    map
}

fn region(polys: Vec<ExPolygon>, annotations: Annotations) -> SlicedRegion {
    // `SlicedRegion` is a watched struct-literal type: keep the FRU rest.
    SlicedRegion {
        object_id: "obj".to_string(),
        region_id: 0,
        polygons: polys,
        segment_annotations: annotations,
        ..Default::default()
    }
}

fn covering_region(annotations: Annotations) -> SlicedRegion {
    region(vec![rect(-10.0, -10.0, 10.0, 10.0)], annotations)
}

fn far_region(annotations: Annotations) -> SlicedRegion {
    region(vec![rect(100.0, 100.0, 110.0, 110.0)], annotations)
}

fn view(regions: Vec<SlicedRegion>) -> PaintRegionLayerView {
    PaintRegionLayerView::new(0).with_slice_ir(Arc::new(SliceIR {
        schema_version: slicer_ir::CURRENT_SLICE_IR_SCHEMA_VERSION,
        global_layer_index: 0,
        z: 0.2,
        regions,
    }))
}

fn probe() -> ExPolygon {
    rect(-1.0, -1.0, 1.0, 1.0)
}

#[test]
fn unpainted_covering_region_returns_default_eligible() {
    // Fast-path equivalence case: the walk would find the covering unpainted
    // region and classify it `DefaultEligible`; the fast path answers the same
    // without walking.
    let v = view(vec![covering_region(HashMap::new())]);
    assert_eq!(
        v.paint_policy_for(&probe()),
        SupportPaintPolicy::DefaultEligible
    );
}

#[test]
fn unpainted_non_covering_region_returns_default_eligible() {
    let v = view(vec![far_region(HashMap::new())]);
    assert_eq!(
        v.paint_policy_for(&probe()),
        SupportPaintPolicy::DefaultEligible
    );
}

#[test]
fn blocker_paint_on_covering_region_returns_blocked() {
    // An over-broad fast-path predicate that skipped painted views fails
    // here: the answer must stay `Blocked`.
    let v = view(vec![covering_region(full_paint(
        PaintSemantic::SupportBlocker,
    ))]);
    assert_eq!(v.paint_policy_for(&probe()), SupportPaintPolicy::Blocked);
}

#[test]
fn enforcer_paint_on_covering_region_returns_enforced() {
    let v = view(vec![covering_region(full_paint(
        PaintSemantic::SupportEnforcer,
    ))]);
    assert_eq!(v.paint_policy_for(&probe()), SupportPaintPolicy::Enforced);
}

#[test]
fn blocker_paint_on_non_covering_region_stays_default() {
    // Paint elsewhere in the layer must not leak onto this expoly; the walk
    // runs (the predicate sees paint) and finds no covering region.
    let v = view(vec![far_region(full_paint(PaintSemantic::SupportBlocker))]);
    assert_eq!(
        v.paint_policy_for(&probe()),
        SupportPaintPolicy::DefaultEligible
    );
}

#[test]
fn none_only_paint_values_stay_default() {
    // Key present, every value `None`: no paint to honour, so the walk's
    // classification is `DefaultEligible`.
    let mut none_only: Annotations = HashMap::new();
    none_only.insert(PaintSemantic::SupportBlocker, vec![vec![None, None]]);
    let v = view(vec![covering_region(none_only)]);
    assert_eq!(
        v.paint_policy_for(&probe()),
        SupportPaintPolicy::DefaultEligible
    );
}

#[test]
fn below_floor_painted_region_stays_default() {
    // The painted region is below the 200-unit² non-trivial-area floor
    // (10×10 units = 100 units²), so the classifier suppresses it even though
    // the predicate keeps the walk alive. The fast path must not (and cannot)
    // change this answer; the tiny probe square sits inside the region.
    let below_floor = ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 { x: 0, y: 0 },
                Point2 { x: 10, y: 0 },
                Point2 { x: 10, y: 10 },
                Point2 { x: 0, y: 10 },
            ],
        },
        holes: Vec::new(),
    };
    let tiny_probe = ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 { x: 2, y: 2 },
                Point2 { x: 8, y: 2 },
                Point2 { x: 8, y: 8 },
                Point2 { x: 2, y: 8 },
            ],
        },
        holes: Vec::new(),
    };
    let v = view(vec![region(
        vec![below_floor],
        full_paint(PaintSemantic::SupportBlocker),
    )]);
    assert_eq!(
        v.paint_policy_for(&tiny_probe),
        SupportPaintPolicy::DefaultEligible
    );
}

#[test]
fn holes_subtract_from_region_coverage() {
    // Region square with a hole; the probe sits entirely inside the hole, so
    // the region does not cover it and a blocker painted on the region must
    // stay off this expoly. (Falsifies a fast path that skipped the walk but
    // also a coverage walk that ignored holes.)
    let mut holed = rect(-10.0, -10.0, 10.0, 10.0);
    holed.holes.push(Polygon {
        points: vec![
            Point2::from_mm(-2.0, -2.0),
            Point2::from_mm(2.0, -2.0),
            Point2::from_mm(2.0, 2.0),
            Point2::from_mm(-2.0, 2.0),
        ],
    });
    let v = view(vec![region(
        vec![holed],
        full_paint(PaintSemantic::SupportBlocker),
    )]);
    assert_eq!(
        v.paint_policy_for(&probe()),
        SupportPaintPolicy::DefaultEligible
    );
}

#[test]
fn aggregate_prefers_blocked_across_covering_regions() {
    // One far (non-covering) blocker region plus one covering enforcer
    // region: the aggregate over covering regions is Enforced, not Blocked —
    // and the fast path cannot fire because paint is present.
    let v = view(vec![
        far_region(full_paint(PaintSemantic::SupportBlocker)),
        covering_region(full_paint(PaintSemantic::SupportEnforcer)),
    ]);
    assert_eq!(v.paint_policy_for(&probe()), SupportPaintPolicy::Enforced);

    // Both covering: Blocked wins (precedence).
    let both = view(vec![
        covering_region(full_paint(PaintSemantic::SupportBlocker)),
        covering_region(full_paint(PaintSemantic::SupportEnforcer)),
    ]);
    assert_eq!(both.paint_policy_for(&probe()), SupportPaintPolicy::Blocked);
}

#[test]
fn no_slice_ir_returns_default_eligible() {
    // Contract retained from the pre-ticket-66 implementation: a view without
    // an attached slice has no policy override.
    assert_eq!(
        PaintRegionLayerView::new(0).paint_policy_for(&probe()),
        SupportPaintPolicy::DefaultEligible
    );
}
