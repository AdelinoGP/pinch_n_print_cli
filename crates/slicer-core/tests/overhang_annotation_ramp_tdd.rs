#![allow(missing_docs)]
//! TDD (AC-5): 45-degree ramp overhang classified into correctly-ordered
//! quartile distance bands.

use slicer_core::algos::overhang_annotation::annotate_overhangs;
use slicer_core::slice_mesh_ex;
use slicer_ir::{ExPolygon, IndexedTriangleSet, Point3, UNITS_PER_MM};

/// Slice `mesh` at each Z and pair each footprint with its position index,
/// producing the `annotate_overhangs` input (which now consumes pre-computed
/// per-layer cross-sections instead of a mesh).
fn footprints(mesh: &IndexedTriangleSet, layer_zs: &[f32]) -> Vec<(u32, Vec<ExPolygon>)> {
    slice_mesh_ex(mesh, layer_zs)
        .into_iter()
        .enumerate()
        .map(|(i, poly)| (i as u32, poly))
        .collect()
}

/// Builds a frustum-like solid: a 10x10mm-footprint prism at z=0 whose
/// footprint grows linearly in +x as z increases (dx/dz = 1, i.e. a 45
/// degree overhang on the +x face). At height `z`, the cross-section is
/// `x in [0, 10 + z]`, `y in [0, 10]`.
///
/// Winding mirrors the known-good `cube_mesh` fixture pattern used by
/// `algo_prepass_slice_tdd.rs` (bottom `0,1,2 / 0,2,3`; top reversed
/// `4,6,5 / 4,7,6`; side quads `a,b,c / a,c,d` per pair of matching
/// bottom/top edges).
fn ramp_mesh() -> IndexedTriangleSet {
    let p3 = |x: f32, y: f32, z: f32| Point3 { x, y, z };
    let vertices = vec![
        p3(0.0, 0.0, 0.0),
        p3(10.0, 0.0, 0.0),
        p3(10.0, 10.0, 0.0),
        p3(0.0, 10.0, 0.0),
        p3(0.0, 0.0, 10.0),
        p3(20.0, 0.0, 10.0),
        p3(20.0, 10.0, 10.0),
        p3(0.0, 10.0, 10.0),
    ];
    #[rustfmt::skip]
    let indices = vec![
        0, 1, 2,  0, 2, 3, // bottom (z=0)
        4, 6, 5,  4, 7, 6, // top (z=10)
        0, 4, 5,  0, 5, 1, // front (y=0)
        1, 5, 6,  1, 6, 2, // right (slanted +x face, 45deg overhang)
        2, 6, 7,  2, 7, 3, // back (y=10)
        3, 7, 4,  3, 4, 0, // left (x=0)
    ];
    IndexedTriangleSet { vertices, indices }
}

/// Shoelace formula, absolute value, in scaled-unit^2; converted to mm^2 by
/// the caller. Holes are subtracted (matches `ExPolygon` semantics: contour
/// minus holes).
fn expolygon_area_mm2(poly: &ExPolygon) -> f64 {
    let ring_area = |pts: &[slicer_ir::Point2]| -> f64 {
        let n = pts.len();
        if n < 3 {
            return 0.0;
        }
        let mut sum = 0.0_f64;
        for i in 0..n {
            let a = pts[i];
            let b = pts[(i + 1) % n];
            sum += (a.x as f64) * (b.y as f64) - (b.x as f64) * (a.y as f64);
        }
        (sum / 2.0).abs()
    };
    let contour_units2 = ring_area(&poly.contour.points);
    let holes_units2: f64 = poly.holes.iter().map(|h| ring_area(&h.points)).sum();
    (contour_units2 - holes_units2) / (UNITS_PER_MM * UNITS_PER_MM)
}

fn total_area_mm2(polys: &[ExPolygon]) -> f64 {
    polys.iter().map(expolygon_area_mm2).sum()
}

#[test]
fn ramp_overhang_partitions_into_correctly_ordered_bands() {
    let mesh = ramp_mesh();
    const LINE_WIDTH_MM: f32 = 0.4;

    // Layer gaps: 0.2mm (small — lands entirely in band 1) then 0.9mm (large
    // — spans all 4 bands, since thresholds are 0.2/0.4/0.6mm and the strip
    // is 0.9mm wide).
    let layer_zs = vec![3.0_f32, 3.2_f32, 4.1_f32];

    let result = annotate_overhangs(&footprints(&mesh, &layer_zs), LINE_WIDTH_MM);

    // Layer 0 has no previous layer: never overhanging, key absent.
    assert!(
        !result.contains_key(&0),
        "layer 0 must have no key (no previous layer)"
    );

    // --- Layer 1 (small 0.2mm step): entirely within band 1. ---
    let layer1_bands = result
        .get(&1)
        .expect("layer 1 (0.2mm ramp step) must have overhang bands");
    assert_eq!(
        layer1_bands.len(),
        1,
        "small 0.2mm step (<= 0.5*0.4=0.2mm threshold) should land entirely in band 1"
    );
    assert_eq!(layer1_bands[0].quartile, 1);
    let layer1_area = total_area_mm2(&layer1_bands[0].polygons);
    // Expected overhang strip: x in (13.0, 13.2], y in [0, 10] => 0.2 * 10 = 2.0 mm^2.
    assert!(
        (layer1_area - 2.0).abs() < 0.2,
        "layer 1 band-1 area {layer1_area} should be ~2.0 mm^2"
    );

    // --- Layer 2 (large 0.9mm step): spans bands 1-4 in order. ---
    let layer2_bands = result
        .get(&2)
        .expect("layer 2 (0.9mm ramp step) must have overhang bands");
    assert!(
        layer2_bands.len() >= 2,
        "large 0.9mm step should be split across multiple bands, got {}",
        layer2_bands.len()
    );

    // Band ordering: quartiles strictly increasing (1 -> 2 -> 3 -> 4), and
    // every present band's quartile in 1..=4.
    let quartiles: Vec<u8> = layer2_bands.iter().map(|b| b.quartile).collect();
    let mut sorted_quartiles = quartiles.clone();
    sorted_quartiles.sort_unstable();
    assert_eq!(
        quartiles, sorted_quartiles,
        "bands must be ordered ascending by quartile (band 1 nearest support, band 4 furthest)"
    );
    for q in &quartiles {
        assert!((1..=4).contains(q), "quartile {q} out of range");
    }

    // Expected per-band widths (mm), each * 10mm strip height:
    // band1: (13.2,13.4] -> 0.2mm wide -> 2.0 mm^2
    // band2: (13.4,13.6] -> 0.2mm wide -> 2.0 mm^2
    // band3: (13.6,13.8] -> 0.2mm wide -> 2.0 mm^2
    // band4: (13.8,14.1] -> 0.3mm wide -> 3.0 mm^2 (capped by region extent)
    let expected_area: std::collections::HashMap<u8, f64> =
        [(1, 2.0), (2, 2.0), (3, 2.0), (4, 3.0)]
            .into_iter()
            .collect();
    for band in layer2_bands {
        let area = total_area_mm2(&band.polygons);
        let expected = expected_area[&band.quartile];
        assert!(
            (area - expected).abs() < 0.4,
            "band {} area {area} should be ~{expected} mm^2",
            band.quartile
        );
    }

    // Total banded area approx equals the total unsupported (overhang) area
    // for layer 2: 0.9mm * 10mm = 9.0 mm^2.
    let total_banded: f64 = layer2_bands
        .iter()
        .map(|b| total_area_mm2(&b.polygons))
        .sum();
    assert!(
        (total_banded - 9.0).abs() < 0.6,
        "total banded area {total_banded} should be ~9.0 mm^2 (the full overhang area)"
    );
}
