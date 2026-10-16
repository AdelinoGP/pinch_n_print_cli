//! Ticket 67: one region-preparation derivation serves both `LayerArena`
//! region projections when the projections coincide.
//!
//! `ensure_prepared_regions` (ordinary regions) and
//! `ensure_prepared_perimeter_source_regions` (wall-owning regions) each cost a
//! per-region `derive_needs_support` plus one `intersection_ex` per overhang
//! quartile band, once per layer. They consume byte-identical inputs exactly
//! when the perimeter-source projection is the identity on the region list, so
//! the arena shares one derivation then and keeps the two separate otherwise.
//!
//! Two of these tests would pass on a tree that never shared anything if they
//! only compared derived values, because the inputs coincide: the sharing claim
//! is therefore pinned on allocation identity (`std::ptr::eq` on the returned
//! slices), which a second derivation cannot fake, and on the restored-geometry
//! case, where a wrongly shared derivation would report the ordinary answer.

use slicer_ir::{
    ExPolygon, ObjectSurfaceData, OverhangRegion, Point2, Polygon, SliceIR, SlicedRegion,
    SurfaceClassificationIR,
};
use slicer_runtime::LayerArena;

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

fn region(object_id: &str, region_id: u64, polygons: Vec<ExPolygon>) -> SlicedRegion {
    SlicedRegion {
        object_id: object_id.into(),
        region_id,
        infill_areas: polygons.clone(),
        polygons,
        ..SlicedRegion::default()
    }
}

/// A classification whose object-global overhang footprint is `footprint`.
/// `derive_needs_support` decides from that geometry alone and never reads the
/// region's own `needs_support` flag, which stays at its default here so it
/// cannot mask a wrong footprint answer.
fn classification_over(footprint: ExPolygon) -> SurfaceClassificationIR {
    let mut classification = SurfaceClassificationIR::default();
    classification.per_object.insert(
        "obj".into(),
        ObjectSurfaceData {
            overhang_regions: vec![OverhangRegion {
                xy_footprint: vec![footprint],
                ..OverhangRegion::default()
            }],
            ..ObjectSurfaceData::default()
        },
    );
    classification
}

fn coinciding_slice() -> SliceIR {
    SliceIR {
        regions: vec![
            region("obj", 3, vec![rect(0.0, 0.0, 5.0, 10.0)]),
            region("obj", 4, vec![rect(5.0, 0.0, 10.0, 10.0)]),
        ],
        global_layer_index: 7,
        ..SliceIR::default()
    }
}

#[test]
fn one_derivation_serves_both_projections_when_they_coincide() {
    let mut arena = LayerArena::new();
    arena
        .set_slice(coinciding_slice())
        .expect("stage the slice");

    arena.ensure_prepared_regions(None);
    arena.ensure_prepared_perimeter_source_regions(None);

    let ordinary = arena
        .prepared_regions()
        .expect("ordinary projection prepared");
    let source = arena
        .prepared_perimeter_source_regions()
        .expect("perimeter-source projection served");
    assert_eq!(
        ordinary.len(),
        2,
        "both regions are prepared; a dropped region would mean the wall-owning projection leaked into the ordinary slot"
    );
    assert_eq!(
        ordinary, source,
        "coinciding projections must expose equal derived data"
    );
    assert!(
        std::ptr::eq(ordinary.as_ptr(), source.as_ptr()),
        "the perimeter-source consumer must be served by the ordinary derivation's own allocation, not a second run of it"
    );
}

#[test]
fn perimeter_source_first_also_leaves_the_ordinary_slot_shared() {
    let mut arena = LayerArena::new();
    arena
        .set_slice(coinciding_slice())
        .expect("stage the slice");

    arena.ensure_prepared_perimeter_source_regions(None);
    let after_source = arena
        .prepared_perimeter_source_regions()
        .expect("perimeter-source projection prepared");
    let source_ptr = after_source.as_ptr();
    arena.ensure_prepared_regions(None);
    let ordinary = arena
        .prepared_regions()
        .expect("ordinary projection served");

    assert!(
        std::ptr::eq(source_ptr, ordinary.as_ptr()),
        "a layer that dispatches Perimeters first must still end with one derivation, not two"
    );
}

#[test]
fn a_modifier_child_forces_the_two_projections_apart() {
    let child_polygons = vec![rect(5.0, 0.0, 10.0, 10.0)];
    let child_id = slicer_ir::modifier_sub_region_id(3, "obj", &child_polygons);
    let mut arena = LayerArena::new();
    arena
        .set_slice(SliceIR {
            regions: vec![
                region("obj", 3, vec![rect(0.0, 0.0, 5.0, 10.0)]),
                region("obj", child_id, child_polygons),
            ],
            ..SliceIR::default()
        })
        .expect("stage the slice");
    let classification = classification_over(rect(5.0, 0.0, 10.0, 10.0));

    arena.ensure_prepared_regions(Some(&classification));
    arena.ensure_prepared_perimeter_source_regions(Some(&classification));

    let ordinary = arena
        .prepared_regions()
        .expect("ordinary projection prepared");
    let source = arena
        .prepared_perimeter_source_regions()
        .expect("perimeter-source projection prepared");
    assert_eq!(
        ordinary.len(),
        2,
        "the ordinary projection keeps the modifier child"
    );
    assert_eq!(
        source.len(),
        1,
        "the wall-owning projection drops the child and re-absorbs its fill into the base"
    );
    assert!(
        !ordinary[0].needs_support,
        "the reduced base region alone misses the overhang footprint"
    );
    assert!(
        source[0].needs_support,
        "the restored base region reaches the footprint, so reusing the ordinary derivation here would be a wrong answer"
    );
    assert!(
        !std::ptr::eq(ordinary.as_ptr(), source.as_ptr()),
        "coincidence does not hold, so the two projections must be derived separately"
    );
}

#[test]
fn the_sharing_decision_dies_with_its_slice() {
    let mut arena = LayerArena::new();
    arena
        .set_slice(coinciding_slice())
        .expect("stage the slice");
    arena.ensure_prepared_regions(None);
    arena.ensure_prepared_perimeter_source_regions(None);
    assert!(std::ptr::eq(
        arena.prepared_regions().expect("prepared").as_ptr(),
        arena
            .prepared_perimeter_source_regions()
            .expect("served by the same derivation")
            .as_ptr()
    ));

    let child_polygons = vec![rect(5.0, 0.0, 10.0, 10.0)];
    let child_id = slicer_ir::modifier_sub_region_id(3, "obj", &child_polygons);
    assert!(arena.take_slice().is_some());
    assert!(
        arena.prepared_regions().is_none() && arena.prepared_perimeter_source_regions().is_none(),
        "taking the slice must drop both slots, including a sharing decision that named it"
    );
    arena
        .set_slice(SliceIR {
            regions: vec![
                region("obj", 3, vec![rect(0.0, 0.0, 5.0, 10.0)]),
                region("obj", child_id, child_polygons),
            ],
            ..SliceIR::default()
        })
        .expect("stage the next layer's slice");

    let classification = classification_over(rect(5.0, 0.0, 10.0, 10.0));
    arena.ensure_prepared_regions(Some(&classification));
    arena.ensure_prepared_perimeter_source_regions(Some(&classification));
    assert_eq!(
        arena.prepared_regions().expect("prepared").len(),
        2,
        "the next layer's regions are prepared on their own inputs, not the previous slice's"
    );
    assert_eq!(
        arena
            .prepared_perimeter_source_regions()
            .expect("prepared")
            .len(),
        1,
        "the previous slice's sharing decision must not survive into this one"
    );
}

#[test]
fn nothing_is_prepared_without_a_staged_slice() {
    let mut arena = LayerArena::new();
    assert!(arena.ensure_prepared_regions(None).is_none());
    assert!(arena
        .ensure_prepared_perimeter_source_regions(None)
        .is_none());
    assert!(arena.prepared_regions().is_none());
    assert!(arena.prepared_perimeter_source_regions().is_none());
}
