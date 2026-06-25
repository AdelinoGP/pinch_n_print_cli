#![allow(missing_docs)]

//! TDD tests for packet 53 (TASK-154): fan-command G-code emission (host emitter).
//!
//! Verifies that the `DefaultGCodeEmitter` renders `LayerCollectionIR`
//! `annotations` carrying raw fan commands (`M106 S{n}` / `M107`) into the
//! correct per-layer G-code section and position.
//!
//! These tests own the HOST emission contract only. The part-cooling *module*
//! (which layer gets which fan speed, first-layers disable, overhang bump,
//! `fan_speed_max=0`) is exercised by the module's own crate tests
//! (`modules/core-modules/part-cooling/tests/`). Here we hand-construct the
//! `LayerAnnotation`s the module would emit via `FinalizationOutputBuilder`, so
//! this file links no module crate.

// SUT is DefaultGCodeEmitter / DefaultGCodeSerializer. Fan commands reach the
// emitter as `LayerAnnotationKind::Raw` annotations — `push_fan_speed(layer, v)`
// in the SDK produces exactly `Raw("M106 S{v}")` (or `Raw("M107")` for v==0)
// at `after_entity_index: 0` (see slicer-sdk/src/traits.rs).

use slicer_ir::{
    ExtrusionPath3D, ExtrusionRole, LayerAnnotation, LayerAnnotationKind, LayerCollectionIR,
    Point3WithWidth, PrintEntity, RegionKey, SemVer,
};
use slicer_runtime::{DefaultGCodeEmitter, DefaultGCodeSerializer, GCodeEmitter, GCodeSerializer};

// ============================================================================
// Test fixtures
// ============================================================================

fn semver_fixture() -> SemVer {
    SemVer {
        major: 1,
        minor: 0,
        patch: 0,
    }
}

fn point3_with_width(x: f32, y: f32, z: f32) -> Point3WithWidth {
    Point3WithWidth {
        x,
        y,
        z,
        width: 0.4,
        flow_factor: 1.0,
        overhang_quartile: None,
    }
}

fn region_key_fixture(layer_index: u32) -> RegionKey {
    RegionKey {
        global_layer_index: layer_index,
        object_id: "test-object".to_string(),
        region_id: 0,
        variant_chain: Vec::new(),
    }
}

fn wall_entity() -> PrintEntity {
    PrintEntity {
        entity_id: 1,
        path: ExtrusionPath3D {
            points: vec![
                point3_with_width(0.0, 0.0, 0.2),
                point3_with_width(1.0, 0.0, 0.2),
            ],
            role: ExtrusionRole::OuterWall,
            speed_factor: 1.0,
        },
        role: ExtrusionRole::OuterWall,
        tool_index: 0,
        region_key: region_key_fixture(0),
        topo_order: 0,
    }
}

/// Raw fan annotation as `push_fan_speed` would record it: `after_entity_index`
/// 0, body `M106 S{value}` (or `M107` when value is 0).
fn fan_annotation(value: u8) -> LayerAnnotation {
    let text = if value == 0 {
        "M107".to_string()
    } else {
        format!("M106 S{}", value)
    };
    LayerAnnotation {
        after_entity_index: 0,
        kind: LayerAnnotationKind::Raw(text),
    }
}

/// A trailing `M107` after the final entity (`after_entity_index: u32::MAX`),
/// matching the part-cooling module's fan-off annotation.
fn trailing_fan_off() -> LayerAnnotation {
    LayerAnnotation {
        after_entity_index: u32::MAX,
        kind: LayerAnnotationKind::Raw("M107".to_string()),
    }
}

fn layer(index: u32, z: f32, annotations: Vec<LayerAnnotation>) -> LayerCollectionIR {
    LayerCollectionIR {
        schema_version: semver_fixture(),
        global_layer_index: index,
        z,
        ordered_entities: vec![wall_entity()],
        tool_changes: vec![],
        z_hops: vec![],
        annotations,
        retracts: vec![],
        travel_moves: vec![],
    }
}

fn emit(layers: &[LayerCollectionIR]) -> String {
    let emitter = DefaultGCodeEmitter::new("1.0.0-test".to_string());
    let gcode_ir = emitter.emit_gcode(layers).expect("emit_gcode must succeed");
    let serializer = DefaultGCodeSerializer::new();
    serializer
        .serialize_gcode(&gcode_ir)
        .expect("serialize_gcode must succeed")
}

/// Split serialized GCode text into per-layer sections using `;LAYER_CHANGE` as
/// the delimiter. Drops anything before the first `;LAYER_CHANGE` (the emitter
/// writes a header preamble there), so `sections[i]` corresponds to layer `i`.
fn layer_sections(text: &str) -> Vec<&str> {
    let mut sections = Vec::new();
    let mut positions: Vec<usize> = text
        .match_indices(";LAYER_CHANGE")
        .map(|(p, _)| p)
        .collect();
    if positions.is_empty() {
        return sections;
    }
    positions.push(text.len());
    for w in positions.windows(2) {
        sections.push(&text[w[0]..w[1]]);
    }
    sections
}

// ============================================================================
// Positive: fan annotations render into the right per-layer section
// ============================================================================

#[test]
fn m106_annotation_renders_in_its_layer_section() {
    // layer 0: fan off (M107); layer 2: fan on (M106 S255).
    let layers = vec![
        layer(0, 0.2, vec![fan_annotation(0)]),
        layer(1, 0.4, vec![fan_annotation(255)]),
        layer(2, 0.6, vec![fan_annotation(255)]),
    ];

    let text = emit(&layers);
    let sections = layer_sections(&text);

    assert!(
        sections.len() >= 3,
        "expected at least 3 layer sections, got {}",
        sections.len()
    );
    assert!(
        sections[0].contains("M107"),
        "layer 0 annotation (M107) must render in its section"
    );
    assert!(
        sections[2].contains("M106 S255"),
        "layer 2 annotation (M106 S255) must render in its section"
    );
}

#[test]
fn trailing_m107_renders_after_last_layer() {
    let layers = vec![
        layer(0, 0.2, vec![fan_annotation(0)]),
        layer(1, 0.4, vec![fan_annotation(255), trailing_fan_off()]),
    ];

    let text = emit(&layers);

    assert!(
        text.contains("M107"),
        "M107 must be present to turn fan off after last layer"
    );
    // One M107 for layer 0 + one trailing fan-off on the last layer = 2.
    let m107_count = text.matches("M107").count();
    assert_eq!(m107_count, 2, "expected exactly 2 M107 commands");
}

#[test]
fn multiple_annotations_on_one_layer_all_render() {
    // Overhang bump pattern: base M106 then a bumped M106 then a restore.
    let layers = vec![layer(
        0,
        0.2,
        vec![
            fan_annotation(255),
            fan_annotation(100),
            fan_annotation(255),
        ],
    )];

    let text = emit(&layers);
    assert!(text.contains("M106 S255"), "base/restore M106 must render");
    assert!(text.contains("M106 S100"), "bumped M106 must render");
    assert!(
        text.matches("M106 S255").count() >= 2,
        "both base and restore M106 S255 must render"
    );
}

// ============================================================================
// Negative
// ============================================================================

#[test]
fn no_fan_annotation_emits_no_fan_command() {
    let layers = vec![layer(0, 0.2, vec![]), layer(1, 0.4, vec![])];

    let text = emit(&layers);
    assert!(
        !text.contains("M106 S"),
        "layers without fan annotations must emit no M106"
    );
    assert!(
        !text.contains("M107"),
        "layers without fan annotations must emit no M107"
    );
}
