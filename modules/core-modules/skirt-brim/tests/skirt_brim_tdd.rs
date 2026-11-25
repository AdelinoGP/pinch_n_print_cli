#![allow(missing_docs)]

use std::collections::HashMap;

use skirt_brim::SkirtBrim;
use slicer_ir::{
    ConfigValue, ConfigView, ExtrusionRole, LayerCollectionIR, Point3WithWidth, PrintEntity,
    RegionKey,
};
use slicer_sdk::test_prelude::{print_entity, LayerCollectionFixtureBuilder};

// ---- Helpers ----

/// A closed square wall loop of half-extent `half` around `(x, y)`.
///
/// The module derives its footprint from real closed outer-wall loops, so the
/// fixtures must be loops, not single points: a single point has no polygon and
/// correctly produces no brim and a degenerate (skipped) hull.
fn make_loop_at(x: f32, y: f32, half: f32, z: f32) -> PrintEntity {
    let pt = |px: f32, py: f32| Point3WithWidth {
        x: px,
        y: py,
        z,
        width: 0.4,
        ..Default::default()
    };
    let points = vec![
        pt(x - half, y - half),
        pt(x + half, y - half),
        pt(x + half, y + half),
        pt(x - half, y + half),
        pt(x - half, y - half),
    ];
    print_entity(
        1,
        ExtrusionRole::OuterWall,
        points,
        RegionKey {
            global_layer_index: 0,
            object_id: "obj1".to_string(),
            region_id: 1,
            variant_chain: Vec::new(),
        },
        0,
    )
}

/// A closed square wall loop, `half` mm on each side, around `(x, y)`.
fn make_entity_at(x: f32, y: f32, z: f32) -> PrintEntity {
    make_loop_at(x, y, 1.0, z)
}

fn make_layer_with_entities(index: u32, z: f32, entities: Vec<PrintEntity>) -> LayerCollectionIR {
    entities
        .into_iter()
        .fold(
            LayerCollectionFixtureBuilder::new()
                .global_layer_index(index)
                .z(z),
            |b, e| b.add_entity(e),
        )
        .build()
}

fn empty_config() -> ConfigView {
    // Declared keys (skirt-brim.toml) seeded at their manifest defaults,
    // mirroring the production `seed_registry_defaults` step; the empty-map
    // probe now behaves like a bound view.
    ConfigView::from_map(HashMap::from([
        ("skirt_brim_enabled".to_string(), ConfigValue::Bool(true)),
        ("skirt_loops".to_string(), ConfigValue::Int(6)),
        ("skirt_distance".to_string(), ConfigValue::Float(3.0)),
        ("skirt_height".to_string(), ConfigValue::Int(1)),
        ("brim_width".to_string(), ConfigValue::Float(8.0)),
        ("brim_object_gap".to_string(), ConfigValue::Float(0.1)),
        ("min_skirt_length".to_string(), ConfigValue::Float(0.0)),
        ("line_width".to_string(), ConfigValue::Float(0.4)),
    ]))
}

fn custom_config(
    loops: u32,
    distance: f32,
    height: u32,
    brim: f32,
    line_w: f32,
    enabled: bool,
) -> ConfigView {
    let mut fields = HashMap::new();
    fields.insert("skirt_brim_enabled".to_string(), ConfigValue::Bool(enabled));
    fields.insert("skirt_loops".to_string(), ConfigValue::Int(loops as i64));
    fields.insert(
        "skirt_distance".to_string(),
        ConfigValue::Float(distance as f64),
    );
    fields.insert("skirt_height".to_string(), ConfigValue::Int(height as i64));
    fields.insert("brim_width".to_string(), ConfigValue::Float(brim as f64));
    fields.insert("brim_object_gap".to_string(), ConfigValue::Float(0.1));
    fields.insert("min_skirt_length".to_string(), ConfigValue::Float(0.0));
    fields.insert("line_width".to_string(), ConfigValue::Float(line_w as f64));
    ConfigView::from_map(fields)
}

// ---- Tests ----

#[test]
fn from_config_defaults() {
    let sb = SkirtBrim::from_config(&empty_config()).unwrap();
    assert!(sb.enabled());
    assert_eq!(sb.skirt_loops(), 6);
    assert!((sb.skirt_distance() - 3.0).abs() < 0.001);
    assert_eq!(sb.skirt_height(), 1);
    assert!((sb.brim_width() - 8.0).abs() < 0.001);
    assert!((sb.brim_object_gap() - 0.1).abs() < 0.001);
    assert!((sb.min_skirt_length() - 0.0).abs() < 0.001);
    assert!((sb.line_width() - 0.4).abs() < 0.001);
}

#[test]
fn from_config_custom() {
    let cfg = custom_config(3, 10.0, 5, 4.0, 0.5, true);
    let sb = SkirtBrim::from_config(&cfg).unwrap();
    assert!(sb.enabled());
    assert_eq!(sb.skirt_loops(), 3);
    assert!((sb.skirt_distance() - 10.0).abs() < 0.001);
    assert_eq!(sb.skirt_height(), 5);
    assert!((sb.brim_width() - 4.0).abs() < 0.001);
    assert!((sb.line_width() - 0.5).abs() < 0.001);
}

#[test]
fn disabled_no_changes() {
    let cfg = custom_config(1, 6.0, 1, 0.0, 0.4, false);
    let sb = SkirtBrim::from_config(&cfg).unwrap();
    let mut layers = vec![make_layer_with_entities(
        0,
        0.2,
        vec![make_entity_at(10.0, 10.0, 0.2)],
    )];
    let orig = layers.clone();
    sb.process(&mut layers).unwrap();
    assert_eq!(layers, orig);
}

#[test]
fn empty_layers_no_output() {
    let sb = SkirtBrim::from_config(&empty_config()).unwrap();
    let mut layers: Vec<LayerCollectionIR> = vec![];
    sb.process(&mut layers).unwrap();
    assert!(layers.is_empty());
}

#[test]
fn single_skirt_loop() {
    let cfg = custom_config(1, 6.0, 1, 0.0, 0.4, true);
    let sb = SkirtBrim::from_config(&cfg).unwrap();
    let mut layers = vec![make_layer_with_entities(
        0,
        0.2,
        vec![make_entity_at(50.0, 50.0, 0.2)],
    )];
    sb.process(&mut layers).unwrap();

    let skirt_count = layers[0]
        .ordered_entities
        .iter()
        .filter(|e| e.role == ExtrusionRole::Skirt)
        .count();
    assert_eq!(skirt_count, 1, "expected 1 skirt entity, got {skirt_count}");
    // Skirt should be prepended (first entity)
    assert_eq!(layers[0].ordered_entities[0].role, ExtrusionRole::Skirt);
}

#[test]
fn skirt_has_correct_role() {
    let sb = SkirtBrim::from_config(&empty_config()).unwrap();
    let mut layers = vec![make_layer_with_entities(
        0,
        0.2,
        vec![make_entity_at(50.0, 50.0, 0.2)],
    )];
    sb.process(&mut layers).unwrap();

    for entity in &layers[0].ordered_entities {
        if entity.region_key.object_id == "__skirt__" {
            assert_eq!(entity.role, ExtrusionRole::Skirt);
            assert_eq!(entity.path.role, ExtrusionRole::Skirt);
        }
    }
}

#[test]
fn skirt_surrounds_print() {
    let cfg = custom_config(1, 6.0, 1, 0.0, 0.4, true);
    let sb = SkirtBrim::from_config(&cfg).unwrap();
    let mut layers = vec![make_layer_with_entities(
        0,
        0.2,
        vec![
            make_entity_at(10.0, 10.0, 0.2),
            make_entity_at(20.0, 20.0, 0.2),
        ],
    )];
    sb.process(&mut layers).unwrap();

    // entity bbox is [10, 10] to [20, 20]
    // skirt should be offset by 6.0mm, so skirt bbox should be [4, 4] to [26, 26]
    let skirt_entity = layers[0]
        .ordered_entities
        .iter()
        .find(|e| e.role == ExtrusionRole::Skirt)
        .expect("skirt entity should exist");

    let mut skirt_x_min = f32::MAX;
    let mut skirt_y_min = f32::MAX;
    let mut skirt_x_max = f32::MIN;
    let mut skirt_y_max = f32::MIN;

    for pt in &skirt_entity.path.points {
        skirt_x_min = skirt_x_min.min(pt.x);
        skirt_y_min = skirt_y_min.min(pt.y);
        skirt_x_max = skirt_x_max.max(pt.x);
        skirt_y_max = skirt_y_max.max(pt.y);
    }

    // Skirt should extend at least skirt_distance beyond entity bbox
    assert!(
        skirt_x_min <= 10.0 - 6.0 + 0.01,
        "skirt x_min={skirt_x_min} should be <= 4.0"
    );
    assert!(
        skirt_y_min <= 10.0 - 6.0 + 0.01,
        "skirt y_min={skirt_y_min} should be <= 4.0"
    );
    assert!(
        skirt_x_max >= 20.0 + 6.0 - 0.01,
        "skirt x_max={skirt_x_max} should be >= 26.0"
    );
    assert!(
        skirt_y_max >= 20.0 + 6.0 - 0.01,
        "skirt y_max={skirt_y_max} should be >= 26.0"
    );
}

#[test]
fn multiple_skirt_loops() {
    let cfg = custom_config(3, 6.0, 1, 0.0, 0.4, true);
    let sb = SkirtBrim::from_config(&cfg).unwrap();
    let mut layers = vec![make_layer_with_entities(
        0,
        0.2,
        vec![make_entity_at(50.0, 50.0, 0.2)],
    )];
    sb.process(&mut layers).unwrap();

    let skirt_count = layers[0]
        .ordered_entities
        .iter()
        .filter(|e| e.role == ExtrusionRole::Skirt)
        .count();
    assert_eq!(
        skirt_count, 3,
        "expected 3 skirt entities, got {skirt_count}"
    );
}

#[test]
fn skirt_height_multiple_layers() {
    let cfg = custom_config(1, 6.0, 3, 0.0, 0.4, true);
    let sb = SkirtBrim::from_config(&cfg).unwrap();
    let mut layers = vec![
        make_layer_with_entities(0, 0.2, vec![make_entity_at(50.0, 50.0, 0.2)]),
        make_layer_with_entities(1, 0.4, vec![make_entity_at(50.0, 50.0, 0.4)]),
        make_layer_with_entities(2, 0.6, vec![make_entity_at(50.0, 50.0, 0.6)]),
        make_layer_with_entities(3, 0.8, vec![make_entity_at(50.0, 50.0, 0.8)]),
    ];
    sb.process(&mut layers).unwrap();

    // First 3 layers should have skirt
    for (i, layer) in layers.iter().enumerate().take(3) {
        let has_skirt = layer
            .ordered_entities
            .iter()
            .any(|e| e.role == ExtrusionRole::Skirt);
        assert!(has_skirt, "layer {i} should have skirt entities");
    }

    // Layer 3 should NOT have skirt
    let has_skirt = layers[3]
        .ordered_entities
        .iter()
        .any(|e| e.role == ExtrusionRole::Skirt);
    assert!(!has_skirt, "layer 3 should NOT have skirt entities");
}

#[test]
fn brim_generates_paths() {
    let cfg = custom_config(0, 6.0, 1, 3.0, 0.4, true);
    let sb = SkirtBrim::from_config(&cfg).unwrap();
    let mut layers = vec![
        make_layer_with_entities(0, 0.2, vec![make_entity_at(50.0, 50.0, 0.2)]),
        make_layer_with_entities(1, 0.4, vec![make_entity_at(50.0, 50.0, 0.4)]),
    ];
    sb.process(&mut layers).unwrap();

    // Layer 0 should have brim entities
    let brim_count = layers[0]
        .ordered_entities
        .iter()
        .filter(|e| e.role == ExtrusionRole::Brim)
        .count();
    assert!(
        brim_count > 0,
        "layer 0 should have brim entities, got {brim_count}"
    );

    // Canonical quantisation (`make_brim`): brim_width is rounded down to an
    // even number of extrusion spacings. 3.0 mm at 0.4 mm spacing ->
    // floor(3.0/0.4/2) = 3 -> 2.4 mm, and the ring is laid at
    // 2.3, 1.9, 1.5, 1.1, 0.7, 0.3 mm from the footprint = 6 loops.
    assert_eq!(
        brim_count, 6,
        "expected 6 canonically quantised brim loops, got {brim_count}"
    );

    // Layer 1 should NOT have brim entities
    let brim_on_layer1 = layers[1]
        .ordered_entities
        .iter()
        .any(|e| e.role == ExtrusionRole::Brim);
    assert!(!brim_on_layer1, "layer 1 should NOT have brim entities");
}

#[test]
fn brim_follows_the_contour_not_the_bounding_box() {
    // Two diagonal squares in one object: the union is a concave footprint.
    // A bounding-box brim would be a 5-point rectangle; a contour-following
    // brim traces both squares.
    let cfg = custom_config(0, 6.0, 1, 0.8, 0.4, true);
    let sb = SkirtBrim::from_config(&cfg).unwrap();
    let mut layers = vec![make_layer_with_entities(
        0,
        0.2,
        vec![
            make_loop_at(10.0, 10.0, 1.0, 0.2),
            make_loop_at(20.0, 20.0, 1.0, 0.2),
        ],
    )];
    sb.process(&mut layers).unwrap();

    let brims: Vec<_> = layers[0]
        .ordered_entities
        .iter()
        .filter(|e| e.role == ExtrusionRole::Brim)
        .collect();
    assert!(!brims.is_empty(), "expected a brim around the footprint");
    for brim in &brims {
        assert!(
            brim.path.points.len() > 5,
            "contour-following brim should trace both squares, got {} points",
            brim.path.points.len()
        );
    }
}

#[test]
fn skirt_is_hull_anchored_and_encloses_the_brim() {
    // A 1.0 mm brim plus a 4.0 mm skirt: the outermost skirt loop must clear
    // the brim's outer boundary.
    let cfg = custom_config(2, 4.0, 1, 1.0, 0.4, true);
    let sb = SkirtBrim::from_config(&cfg).unwrap();
    let mut layers = vec![make_layer_with_entities(
        0,
        0.2,
        vec![make_loop_at(10.0, 10.0, 1.0, 0.2)],
    )];
    sb.process(&mut layers).unwrap();

    let bbox = |role: ExtrusionRole| -> (f32, f32) {
        let mut min = f32::MAX;
        let mut max = f32::MIN;
        for e in layers[0].ordered_entities.iter().filter(|e| e.role == role) {
            for p in &e.path.points {
                min = min.min(p.x);
                max = max.max(p.x);
            }
        }
        (min, max)
    };
    let (brim_min, brim_max) = bbox(ExtrusionRole::Brim);
    let (skirt_min, skirt_max) = bbox(ExtrusionRole::Skirt);
    // Outermost skirt loop sits at skirt_distance + spacing/2 + (loops-1)*spacing
    // = 4.0 + 0.2 + 0.4 = 4.6 mm outside the (brim-inclusive) hull.
    assert!(
        skirt_min <= brim_min - 4.0,
        "skirt x_min={skirt_min} should clear the brim x_min={brim_min} by >= 4 mm"
    );
    assert!(
        skirt_max >= brim_max + 4.0,
        "skirt x_max={skirt_max} should clear the brim x_max={brim_max} by >= 4 mm"
    );
}

#[test]
fn brim_is_clipped_by_first_layer_supports() {
    // `no_brim_area_support` in canonical `make_brim`: a support pad that
    // reaches the bed removes brim material under it.
    let cfg = custom_config(0, 6.0, 1, 4.0, 0.4, true);
    let sb = SkirtBrim::from_config(&cfg).unwrap();

    let make = |role: ExtrusionRole, x: f32, y: f32, half: f32| {
        let pt = |px: f32, py: f32| Point3WithWidth {
            x: px,
            y: py,
            z: 0.2,
            width: 0.4,
            ..Default::default()
        };
        let mut entity = make_loop_at(x, y, half, 0.2);
        entity.role = role.clone();
        entity.path.role = role;
        entity.path.points = vec![
            pt(x - half, y - half),
            pt(x + half, y - half),
            pt(x + half, y + half),
            pt(x - half, y + half),
            pt(x - half, y - half),
        ];
        entity
    };

    let mut layers = vec![make_layer_with_entities(
        0,
        0.2,
        vec![
            make_loop_at(10.0, 10.0, 1.0, 0.2),
            make(ExtrusionRole::SupportMaterial, 12.0, 10.0, 0.5),
        ],
    )];
    sb.process(&mut layers).unwrap();

    let support_min_x = 11.5;
    let support_max_x = 12.5;
    let support_min_y = 9.5;
    let support_max_y = 10.5;
    let mut inside = 0usize;
    for e in layers[0]
        .ordered_entities
        .iter()
        .filter(|e| e.role == ExtrusionRole::Brim)
    {
        for p in &e.path.points {
            if p.x > support_min_x
                && p.x < support_max_x
                && p.y > support_min_y
                && p.y < support_max_y
            {
                inside += 1;
            }
        }
    }
    assert_eq!(
        inside, 0,
        "brim must not print inside a first-layer support pad"
    );
}

#[test]
fn brim_quantises_on_the_canonical_flow_spacing() {
    // `Flow::rounded_rectangle_extrusion_spacing`: 0.4 - 0.2 * (1 - pi/4)
    // = 0.35708 mm. 8 mm then quantises to floor(8/0.35708/2) * 2 * 0.35708
    // = 7.856 mm, and the ring carries 22 lines (11 outer + 11 hole
    // boundaries), each a spacing apart.
    let mut fields = HashMap::new();
    fields.insert("skirt_brim_enabled".to_string(), ConfigValue::Bool(true));
    fields.insert("skirt_loops".to_string(), ConfigValue::Int(0));
    fields.insert("skirt_distance".to_string(), ConfigValue::Float(3.0));
    fields.insert("skirt_height".to_string(), ConfigValue::Int(1));
    fields.insert("brim_width".to_string(), ConfigValue::Float(8.0));
    fields.insert("brim_object_gap".to_string(), ConfigValue::Float(0.1));
    fields.insert("min_skirt_length".to_string(), ConfigValue::Float(0.0));
    fields.insert("line_width".to_string(), ConfigValue::Float(0.4));
    fields.insert("first_layer_height".to_string(), ConfigValue::Float(0.2));
    let cfg = ConfigView::from_map(fields);

    let sb = SkirtBrim::from_config(&cfg).unwrap();
    let mut layers = vec![make_layer_with_entities(
        0,
        0.2,
        vec![make_loop_at(10.0, 10.0, 1.0, 0.2)],
    )];
    sb.process(&mut layers).unwrap();

    let brim_count = layers[0]
        .ordered_entities
        .iter()
        .filter(|e| e.role == ExtrusionRole::Brim)
        .count();
    assert_eq!(
        brim_count, 22,
        "flow spacing 0.35708 should yield 22 brim lines"
    );

    let spacing = 0.4 - 0.2 * (1.0 - std::f32::consts::FRAC_PI_4);
    let outer = layers[0]
        .ordered_entities
        .iter()
        .filter(|e| e.role == ExtrusionRole::Brim)
        .map(|e| e.path.points.iter().map(|p| p.x).fold(f32::MIN, f32::max))
        .fold(f32::MIN, f32::max);
    // Outer boundary at gap + quantised width = 0.1 + 11 * 2 * spacing, less
    // the half-spacing first-line inset. The anchor is the reconstructed
    // contour: the wall centerline (half extent 1.0 mm) grown by half the
    // outer-wall width (0.4 mm), so the outermost line's max x is
    // 10 + 1.2 + (boundary - spacing/2).
    let anchor_half_extent = 1.0 + 0.5 * 0.4;
    let expected = 10.0 + anchor_half_extent + (0.1 + 11.0 * 2.0 * spacing) - spacing * 0.5;
    assert!(
        (outer - expected).abs() < 0.05,
        "outermost brim x={outer} should be {expected}"
    );
}

#[test]
fn brim_anchor_is_the_contour_not_the_wall_centerline() {
    // The IR carries the outer-wall *centerline*; canonical `make_brim`
    // anchors on the slice contour, half a wall width outside it. The entity's
    // own point width (0.8 mm here) is wider than the config line width, so a
    // module that read `line_width` instead would place the brim 0.2 mm too
    // far in; this pins the point-width reconstruction.
    let cfg = custom_config(0, 6.0, 1, 8.0, 0.4, true);
    let sb = SkirtBrim::from_config(&cfg).unwrap();
    let mut entity = make_loop_at(10.0, 10.0, 1.0, 0.2);
    for point in &mut entity.path.points {
        point.width = 0.8;
    }
    let mut layers = vec![make_layer_with_entities(0, 0.2, vec![entity])];
    sb.process(&mut layers).unwrap();

    // The innermost brim line is the smallest ring; its left edge sits
    // contour + gap + spacing/2 from the part centre.
    let ring_half_extent = |e: &PrintEntity| -> f32 {
        let min = e.path.points.iter().map(|p| p.x).fold(f32::MAX, f32::min);
        let max = e.path.points.iter().map(|p| p.x).fold(f32::MIN, f32::max);
        (max - min) * 0.5
    };
    let inner_ring = layers[0]
        .ordered_entities
        .iter()
        .filter(|e| e.role == ExtrusionRole::Brim)
        .min_by(|a, b| {
            ring_half_extent(a)
                .partial_cmp(&ring_half_extent(b))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .expect("expected brim entities");
    let brim_min_x = inner_ring
        .path
        .points
        .iter()
        .map(|p| p.x)
        .fold(f32::MAX, f32::min);

    // Contour half extent: 1.0 (centerline) + 0.5 * 0.8 = 1.4 mm. The first
    // concentric line sits gap + spacing/2 outside it, and its inner edge is
    // half the *printed* width (0.4) back in.
    let contour_half = 1.4;
    let line_center = 10.0 - brim_min_x;
    let spacing = 0.4; // `custom_config` authors no first_layer_height
    assert!(
        (line_center - (contour_half + 0.1 + spacing * 0.5)).abs() < 0.03,
        "innermost brim line center at {line_center} mm from the part, expected {}",
        contour_half + 0.1 + spacing * 0.5
    );
    assert!(
        line_center - 0.2 > contour_half,
        "the brim bead's inner edge ({}) must stay outside the part outline ({contour_half})",
        line_center - 0.2
    );
}

#[test]
fn skirt_hull_reconstructs_the_contour_on_higher_layers() {
    // `skirt_height = 2`: layer 1 carries wall centerlines only, so its hull
    // is grown by half its outer-wall width before it joins the final hull.
    // Not exercised by the matched job (`skirt_height = 1`); this pins the
    // path so a later change cannot silently drop it.
    let cfg = custom_config(1, 1.0, 2, 0.0, 0.4, true);
    let sb = SkirtBrim::from_config(&cfg).unwrap();
    let mut layers = vec![
        make_layer_with_entities(0, 0.2, vec![make_loop_at(10.0, 10.0, 1.0, 0.2)]),
        make_layer_with_entities(1, 0.4, vec![make_loop_at(10.0, 10.0, 2.0, 0.4)]),
    ];
    sb.process(&mut layers).unwrap();

    let skirt_max_x = layers[0]
        .ordered_entities
        .iter()
        .filter(|e| e.role == ExtrusionRole::Skirt)
        .flat_map(|e| e.path.points.iter())
        .map(|p| p.x)
        .fold(f32::MIN, f32::max);

    // Layer-1 hull half extent 2.0 + 0.5 * 0.4 = 2.2 mm; one skirt loop sits
    // skirt_distance + spacing/2 = 1.0 + 0.2 outside it.
    let expected = 10.0 + 2.2 + 1.2;
    assert!(
        (skirt_max_x - expected).abs() < 0.05,
        "highest-layer skirt loop at x={skirt_max_x}, expected {expected}"
    );
}

#[test]
fn skirt_hull_folds_the_brim_area_contour_not_the_bead_centreline() {
    // Canonical `Print::_make_skirt` folds `m_objectBrimAreasByInstance[...]`
    // `area.contour.points` into the skirt hull — the brim *area's* outer
    // boundary. Folding the emitted beads' centreline instead sits half a flow
    // spacing inside it, so the whole skirt ring lands half a spacing too far
    // in (measured at 0.1786 mm on the matched job, t146; fixed by t151).
    //
    // Square footprint half extent 1.0 mm, outer-wall width 0.4 mm, no
    // `first_layer_height` so the spacing is the 0.4 mm line width:
    //   contour half extent      = 1.0 + 0.4/2           = 1.2
    //   brim area outer boundary = 1.2 + gap 0.1 + 8.0   = 9.3
    //   outermost skirt loop     = hull + 3.0 + 0.4/2    = hull + 3.2
    // so the loop's max x is 10.0 + 9.3 + 3.2 = 22.5. A centreline hull reads
    // 9.1 and lands at 22.3 — 0.2 mm short, four times this test's tolerance.
    let cfg = custom_config(1, 3.0, 1, 8.0, 0.4, true);
    let sb = SkirtBrim::from_config(&cfg).unwrap();
    let mut layers = vec![make_layer_with_entities(
        0,
        0.2,
        vec![make_loop_at(10.0, 10.0, 1.0, 0.2)],
    )];
    sb.process(&mut layers).unwrap();

    let skirt_max_x = layers[0]
        .ordered_entities
        .iter()
        .filter(|e| e.role == ExtrusionRole::Skirt)
        .flat_map(|e| e.path.points.iter())
        .map(|p| p.x)
        .fold(f32::MIN, f32::max);
    let expected = 10.0 + 9.3 + 3.2;
    assert!(
        (skirt_max_x - expected).abs() < 0.05,
        "outermost skirt x={skirt_max_x} should be {expected} (the brim area's \
         contour, not the bead centreline at {})",
        10.0 + 9.1 + 3.2
    );
}

#[test]
fn non_zero_min_skirt_length_fails_closed() {
    let mut fields = HashMap::new();
    fields.insert("skirt_brim_enabled".to_string(), ConfigValue::Bool(true));
    fields.insert("skirt_loops".to_string(), ConfigValue::Int(2));
    fields.insert("skirt_distance".to_string(), ConfigValue::Float(3.0));
    fields.insert("skirt_height".to_string(), ConfigValue::Int(1));
    fields.insert("brim_width".to_string(), ConfigValue::Float(0.0));
    fields.insert("brim_object_gap".to_string(), ConfigValue::Float(0.1));
    fields.insert("min_skirt_length".to_string(), ConfigValue::Float(250.0));
    fields.insert("line_width".to_string(), ConfigValue::Float(0.4));
    let cfg = ConfigView::from_map(fields);
    let err = match SkirtBrim::from_config(&cfg) {
        Ok(_) => panic!("non-zero min_skirt_length must fail closed"),
        Err(e) => e,
    };
    assert_eq!(err.code, 2);
}

#[test]
fn skirt_at_correct_z() {
    let cfg = custom_config(1, 6.0, 1, 0.0, 0.4, true);
    let sb = SkirtBrim::from_config(&cfg).unwrap();
    let mut layers = vec![make_layer_with_entities(
        0,
        0.2,
        vec![make_entity_at(50.0, 50.0, 0.2)],
    )];
    sb.process(&mut layers).unwrap();

    let skirt_entity = layers[0]
        .ordered_entities
        .iter()
        .find(|e| e.role == ExtrusionRole::Skirt)
        .expect("skirt entity should exist");

    for pt in &skirt_entity.path.points {
        assert!(
            (pt.z - 0.2).abs() < 0.001,
            "skirt point z={} should be 0.2",
            pt.z
        );
    }
}
