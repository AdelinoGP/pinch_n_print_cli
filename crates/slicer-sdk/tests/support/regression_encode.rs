// Lossless canonical encodings used only to compare a reconstructed fixture
// with its original value. Floats are encoded by raw bits, never by decimals.
// This file is shared verbatim with the compiled fixture probe.

use slicer_ir::{ExPolygon, ExtrusionRole, IndexedTriangleSet, LayerCollectionIR, PaintValue};

pub fn encode_f32(value: f32) -> String {
    format!("f32:{:08x}", value.to_bits())
}

pub fn encode_f64(value: f64) -> String {
    format!("f64:{:016x}", value.to_bits())
}

pub fn encode_graph(graph: &(Vec<u16>, Vec<(u16, u16)>)) -> String {
    format!("graph:{:?}", graph)
}

fn encode_role(role: &ExtrusionRole) -> String {
    format!("{role:?}")
}

fn encode_paint(value: &PaintValue) -> String {
    match value {
        PaintValue::Flag(flag) => format!("flag:{flag}"),
        PaintValue::Scalar(scalar) => format!("scalar:{}", encode_f32(*scalar)),
        PaintValue::ToolIndex(index) => format!("tool:{index}"),
        PaintValue::Custom(name) => format!("custom:{name:?}"),
    }
}

pub fn encode_layer(layer: &LayerCollectionIR) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "layer:{}:{}:{}:{}",
        layer.schema_version.major,
        layer.schema_version.minor,
        layer.schema_version.patch,
        encode_f32(layer.z)
    ));
    out.push_str(&format!("|index={}", layer.global_layer_index));
    for entity in &layer.ordered_entities {
        out.push_str(&format!(
            "|entity={}:{}:{}:{}:{}",
            entity.entity_id,
            encode_role(&entity.role),
            encode_role(&entity.path.role),
            encode_f32(entity.path.speed_factor),
            entity.topo_order
        ));
        out.push_str(&format!(
            ":tool={:?}:lock={:?}",
            entity.path.tool_index, entity.path.order_lock
        ));
        out.push_str(&format!(
            ":region={}:{:?}:{}:[{}]",
            entity.region_key.global_layer_index,
            entity.region_key.object_id,
            entity.region_key.region_id,
            entity
                .region_key
                .variant_chain
                .iter()
                .map(|(semantic, value)| format!("{semantic:?}={}", encode_paint(value)))
                .collect::<Vec<_>>()
                .join(",")
        ));
        out.push_str(&format!(":entity_tool={}", entity.tool_index));
        for point in &entity.path.points {
            out.push_str(&format!(
                ":(x{},y{},z{},w{},f{},q{:?},d{},o{:?})",
                encode_f32(point.x),
                encode_f32(point.y),
                encode_f32(point.z),
                encode_f32(point.width),
                encode_f32(point.flow_factor),
                point.overhang_quartile,
                encode_f32(point.dist_to_top_mm),
                point.overhang_distance_mm.map(encode_f32)
            ));
        }
    }
    for travel in &layer.travel_moves {
        out.push_str(&format!(
            "|travel={}:{:?}:{:?}:{:?}:{:?}",
            travel.entity_id,
            travel.x.map(encode_f32),
            travel.y.map(encode_f32),
            travel.z.map(encode_f32),
            travel.f.map(encode_f32)
        ));
    }
    out
}

pub fn encode_geometry(polygons: &[ExPolygon]) -> String {
    let mut out = String::from("geometry");
    for polygon in polygons {
        out.push_str(&format!("|contour={:?}", polygon.contour.points));
        for hole in &polygon.holes {
            out.push_str(&format!("|hole={:?}", hole.points));
        }
    }
    out
}

pub fn encode_mesh(mesh: &IndexedTriangleSet) -> String {
    let mut out = String::from("mesh");
    for vertex in &mesh.vertices {
        out.push_str(&format!(
            "|v:{},{},{}",
            encode_f32(vertex.x),
            encode_f32(vertex.y),
            encode_f32(vertex.z)
        ));
    }
    out.push_str(&format!("|i={:?}", mesh.indices));
    out
}
