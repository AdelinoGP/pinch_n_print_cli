//! Concrete regression-fixture formatting for property-test failures.
//!
//! A formatted block is a plain Rust statement block, not a generated
//! `#[test]`. The owning regression must add its own independently justified
//! assertion against the production SUT; formatting alone proves nothing about
//! the system under test.

use std::fmt::Write as _;

use slicer_ir::{
    ExPolygon, ExtrusionRole, IndexedTriangleSet, LayerCollectionIR, PaintValue, RegionKey,
    TravelMove,
};

use super::domains::{DomainError, GeometryCase, SmallGraph};

/// A concrete failing input in a form that can be emitted as Rust source.
#[derive(Debug, Clone, PartialEq)]
pub enum ConcreteInput {
    /// Raw `f32` bit pattern, preserving signed zero, infinities and NaN payloads.
    F32Bits(u32),
    /// Raw `f64` bit pattern, preserving signed zero, infinities and NaN payloads.
    F64Bits(u64),
    /// A small directed graph as node IDs and endpoint pairs.
    Graph(SmallGraph),
    /// A single layer's emission-relevant membership and travel fields.
    Layer(LayerCollectionIR),
    /// A polygon-set geometry case.
    Geometry(GeometryCase),
    /// An indexed triangle mesh.
    Mesh(IndexedTriangleSet),
}

fn invalid_input(message: &str) -> DomainError {
    DomainError::InvalidInput(message.into())
}

/// Return one readable comment line, rejecting an empty or whitespace-only value.
fn comment_line(label: &str, value: &str) -> Result<String, DomainError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(invalid_input(&format!(
            "{label} must not be empty or whitespace"
        )));
    }
    let mut sanitized = String::with_capacity(trimmed.len());
    for character in trimmed.chars() {
        if character.is_control() {
            sanitized.push(' ');
        } else {
            sanitized.push(character);
        }
    }
    let sanitized = sanitized.trim_end();
    Ok(format!("// {label}: {sanitized}"))
}

/// Emit an `f32` by exact bit pattern, never by decimal spelling.
fn f32_literal(value: f32) -> String {
    format!("f32::from_bits(0x{:08x})", value.to_bits())
}

/// Emit an `f64` by exact bit pattern, never by decimal spelling.
fn f64_literal(value: f64) -> String {
    format!("f64::from_bits(0x{:016x}u64)", value.to_bits())
}

fn optional_f32(value: Option<f32>) -> String {
    match value {
        None => "None".to_owned(),
        Some(value) => format!("Some({})", f32_literal(value)),
    }
}

/// Emit a role path qualified by the `slicer_ir` crate.
fn role_literal(role: &ExtrusionRole) -> String {
    match role {
        ExtrusionRole::OuterWall => "slicer_ir::ExtrusionRole::OuterWall".to_owned(),
        ExtrusionRole::InnerWall => "slicer_ir::ExtrusionRole::InnerWall".to_owned(),
        ExtrusionRole::ThinWall => "slicer_ir::ExtrusionRole::ThinWall".to_owned(),
        ExtrusionRole::TopSolidInfill => "slicer_ir::ExtrusionRole::TopSolidInfill".to_owned(),
        ExtrusionRole::BottomSolidInfill => {
            "slicer_ir::ExtrusionRole::BottomSolidInfill".to_owned()
        }
        ExtrusionRole::InternalSolidInfill => {
            "slicer_ir::ExtrusionRole::InternalSolidInfill".to_owned()
        }
        ExtrusionRole::SparseInfill => "slicer_ir::ExtrusionRole::SparseInfill".to_owned(),
        ExtrusionRole::RaftInfill => "slicer_ir::ExtrusionRole::RaftInfill".to_owned(),
        ExtrusionRole::SupportMaterial => "slicer_ir::ExtrusionRole::SupportMaterial".to_owned(),
        ExtrusionRole::SupportInterface => "slicer_ir::ExtrusionRole::SupportInterface".to_owned(),
        ExtrusionRole::SupportBaseInterface => {
            "slicer_ir::ExtrusionRole::SupportBaseInterface".to_owned()
        }
        ExtrusionRole::WipeTower => "slicer_ir::ExtrusionRole::WipeTower".to_owned(),
        ExtrusionRole::PrimeTower => "slicer_ir::ExtrusionRole::PrimeTower".to_owned(),
        ExtrusionRole::Ironing => "slicer_ir::ExtrusionRole::Ironing".to_owned(),
        ExtrusionRole::BridgeInfill => "slicer_ir::ExtrusionRole::BridgeInfill".to_owned(),
        ExtrusionRole::InternalBridgeInfill => {
            "slicer_ir::ExtrusionRole::InternalBridgeInfill".to_owned()
        }
        ExtrusionRole::Skirt => "slicer_ir::ExtrusionRole::Skirt".to_owned(),
        ExtrusionRole::Brim => "slicer_ir::ExtrusionRole::Brim".to_owned(),
        ExtrusionRole::Custom(name) => {
            format!("slicer_ir::ExtrusionRole::Custom({:?}.to_string())", name)
        }
        ExtrusionRole::GapFill => "slicer_ir::ExtrusionRole::GapFill".to_owned(),
    }
}

/// Emit a paint-variant value path qualified by the `slicer_ir` crate.
fn paint_literal(value: &PaintValue) -> String {
    match value {
        PaintValue::Flag(flag) => format!("slicer_ir::PaintValue::Flag({flag})"),
        PaintValue::Scalar(scalar) => {
            format!("slicer_ir::PaintValue::Scalar({})", f32_literal(*scalar))
        }
        PaintValue::ToolIndex(index) => format!("slicer_ir::PaintValue::ToolIndex({index}u32)"),
        PaintValue::Custom(name) => {
            format!("slicer_ir::PaintValue::Custom({name:?}.to_string())")
        }
    }
}

fn format_graph(graph: &SmallGraph, out: &mut String) -> Result<(), DomainError> {
    out.push_str("let input: (Vec<u16>, Vec<(u16, u16)>) = (\n    vec![");
    for (index, node) in graph.node_ids.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        write!(out, "{node}u16").map_err(|_| invalid_input("graph node formatting failed"))?;
    }
    out.push_str("],\n    vec![");
    for (index, (source, target)) in graph.edges.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        write!(out, "({source}u16, {target}u16)")
            .map_err(|_| invalid_input("graph edge formatting failed"))?;
    }
    out.push_str("],\n);\n");
    Ok(())
}

fn format_layer(layer: &LayerCollectionIR, out: &mut String) -> Result<(), DomainError> {
    if !layer.support_entity_identities.is_empty()
        || !layer.tool_changes.is_empty()
        || !layer.z_hops.is_empty()
        || !layer.annotations.is_empty()
        || !layer.retracts.is_empty()
        || !layer.speed_profiles.is_empty()
    {
        return Err(invalid_input(
            "only entity and travel fields are representable; layer side tables must be empty",
        ));
    }
    out.push_str("let input: slicer_ir::LayerCollectionIR = slicer_ir::LayerCollectionIR {\n");
    writeln!(
        out,
        "    schema_version: slicer_ir::SemVer {{ major: {}, minor: {}, patch: {} }},",
        layer.schema_version.major, layer.schema_version.minor, layer.schema_version.patch
    )
    .map_err(|_| invalid_input("layer formatting failed"))?;
    writeln!(
        out,
        "    global_layer_index: {}u32,",
        layer.global_layer_index
    )
    .map_err(|_| invalid_input("layer formatting failed"))?;
    writeln!(out, "    z: {},", f32_literal(layer.z))
        .map_err(|_| invalid_input("layer formatting failed"))?;
    out.push_str("    ordered_entities: vec![\n");
    for entity in &layer.ordered_entities {
        let default_region = entity.region_key == RegionKey::default();
        if default_region {
            out.push_str("        slicer_ir::PrintEntity {\n");
        } else {
            out.push_str(
                "        // exhaustive: generated fixture pins every PrintEntity field; regenerate on IR change\n",
            );
            out.push_str("        slicer_ir::PrintEntity {\n");
        }
        writeln!(out, "            entity_id: {}u64,", entity.entity_id)
            .map_err(|_| invalid_input("entity formatting failed"))?;
        out.push_str(
            "            // exhaustive: generated fixture pins every ExtrusionPath3D field; regenerate on IR change\n",
        );
        out.push_str("            path: slicer_ir::ExtrusionPath3D {\n");
        out.push_str("                points: vec![\n");
        for point in &entity.path.points {
            out.push_str(
                "                    // exhaustive: generated fixture pins the full Point3WithWidth shape; regenerate on IR change\n",
            );
            out.push_str("                    slicer_ir::Point3WithWidth {\n");
            writeln!(out, "                        x: {},", f32_literal(point.x))
                .map_err(|_| invalid_input("point formatting failed"))?;
            writeln!(out, "                        y: {},", f32_literal(point.y))
                .map_err(|_| invalid_input("point formatting failed"))?;
            writeln!(out, "                        z: {},", f32_literal(point.z))
                .map_err(|_| invalid_input("point formatting failed"))?;
            writeln!(
                out,
                "                        width: {},",
                f32_literal(point.width)
            )
            .map_err(|_| invalid_input("point formatting failed"))?;
            writeln!(
                out,
                "                        flow_factor: {},",
                f32_literal(point.flow_factor)
            )
            .map_err(|_| invalid_input("point formatting failed"))?;
            writeln!(
                out,
                "                        overhang_quartile: {},",
                optional_u8(point.overhang_quartile)
            )
            .map_err(|_| invalid_input("point formatting failed"))?;
            writeln!(
                out,
                "                        dist_to_top_mm: {},",
                f32_literal(point.dist_to_top_mm)
            )
            .map_err(|_| invalid_input("point formatting failed"))?;
            writeln!(
                out,
                "                        overhang_distance_mm: {},",
                optional_f32(point.overhang_distance_mm)
            )
            .map_err(|_| invalid_input("point formatting failed"))?;
            out.push_str("                    },\n");
        }
        out.push_str("                ],\n");
        writeln!(
            out,
            "                role: {},",
            role_literal(&entity.path.role)
        )
        .map_err(|_| invalid_input("path formatting failed"))?;
        writeln!(
            out,
            "                speed_factor: {},",
            f32_literal(entity.path.speed_factor)
        )
        .map_err(|_| invalid_input("path formatting failed"))?;
        writeln!(
            out,
            "                tool_index: {},",
            optional_u32(entity.path.tool_index)
        )
        .map_err(|_| invalid_input("path formatting failed"))?;
        writeln!(
            out,
            "                order_lock: {},",
            optional_u64(entity.path.order_lock)
        )
        .map_err(|_| invalid_input("path formatting failed"))?;
        out.push_str("            },\n");
        writeln!(out, "            role: {},", role_literal(&entity.role))
            .map_err(|_| invalid_input("entity formatting failed"))?;
        writeln!(out, "            topo_order: {}u32,", entity.topo_order)
            .map_err(|_| invalid_input("entity formatting failed"))?;
        writeln!(out, "            tool_index: {}u32,", entity.tool_index)
            .map_err(|_| invalid_input("entity formatting failed"))?;
        if default_region {
            writeln!(
                out,
                "            ..slicer_sdk::test_support::fixtures::print_entity_base({})",
                role_literal(&entity.role)
            )
            .map_err(|_| invalid_input("entity formatting failed"))?;
        } else {
            out.push_str("            region_key: slicer_ir::RegionKey {\n");
            writeln!(
                out,
                "                global_layer_index: {}u32,",
                entity.region_key.global_layer_index
            )
            .map_err(|_| invalid_input("region formatting failed"))?;
            writeln!(
                out,
                "                object_id: {:?}.to_string(),",
                entity.region_key.object_id
            )
            .map_err(|_| invalid_input("region formatting failed"))?;
            writeln!(
                out,
                "                region_id: {}u64,",
                entity.region_key.region_id
            )
            .map_err(|_| invalid_input("region formatting failed"))?;
            out.push_str("                variant_chain: vec![\n");
            for (semantic, value) in &entity.region_key.variant_chain {
                writeln!(
                    out,
                    "                    ({:?}.to_string(), {}),",
                    semantic,
                    paint_literal(value)
                )
                .map_err(|_| invalid_input("region formatting failed"))?;
            }
            out.push_str("                ],\n            },\n");
        }
        out.push_str("        },\n");
    }
    out.push_str("    ],\n");
    out.push_str("    travel_moves: vec![\n");
    for travel in &layer.travel_moves {
        format_travel(travel, out)?;
    }
    out.push_str("    ],\n");
    out.push_str("    ..Default::default()\n};\n");
    Ok(())
}

fn format_travel(travel: &TravelMove, out: &mut String) -> Result<(), DomainError> {
    out.push_str(
        "        // exhaustive: generated fixture pins every TravelMove field; regenerate on IR change\n",
    );
    out.push_str("        slicer_ir::TravelMove {\n");
    writeln!(out, "            entity_id: {}u64,", travel.entity_id)
        .map_err(|_| invalid_input("travel formatting failed"))?;
    writeln!(out, "            x: {},", optional_f32(travel.x))
        .map_err(|_| invalid_input("travel formatting failed"))?;
    writeln!(out, "            y: {},", optional_f32(travel.y))
        .map_err(|_| invalid_input("travel formatting failed"))?;
    writeln!(out, "            z: {},", optional_f32(travel.z))
        .map_err(|_| invalid_input("travel formatting failed"))?;
    writeln!(out, "            f: {},", optional_f32(travel.f))
        .map_err(|_| invalid_input("travel formatting failed"))?;
    out.push_str("        },\n");
    Ok(())
}

fn optional_u8(value: Option<u8>) -> String {
    match value {
        None => "None".to_owned(),
        Some(value) => format!("Some({value}u8)"),
    }
}

fn optional_u32(value: Option<u32>) -> String {
    match value {
        None => "None".to_owned(),
        Some(value) => format!("Some({value}u32)"),
    }
}

fn optional_u64(value: Option<u64>) -> String {
    match value {
        None => "None".to_owned(),
        Some(value) => format!("Some({value}u64)"),
    }
}

fn format_geometry(polygons: &[ExPolygon], out: &mut String) -> Result<(), DomainError> {
    out.push_str("let input: Vec<slicer_ir::ExPolygon> = vec![\n");
    for polygon in polygons {
        out.push_str("    slicer_ir::ExPolygon {\n        contour: ");
        format_polygon(&polygon.contour, out)?;
        out.push_str(",\n        holes: vec![\n");
        for hole in &polygon.holes {
            out.push_str("            ");
            format_polygon(hole, out)?;
            out.push_str(",\n");
        }
        out.push_str("        ],\n    },\n");
    }
    out.push_str("];\n");
    Ok(())
}

fn format_polygon(polygon: &slicer_ir::Polygon, out: &mut String) -> Result<(), DomainError> {
    out.push_str("slicer_ir::Polygon { points: vec![");
    for (index, point) in polygon.points.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        write!(
            out,
            "slicer_ir::Point2 {{ x: {}i64, y: {}i64 }}",
            point.x, point.y
        )
        .map_err(|_| invalid_input("polygon formatting failed"))?;
    }
    out.push_str("] }");
    Ok(())
}

fn format_mesh(mesh: &IndexedTriangleSet, out: &mut String) -> Result<(), DomainError> {
    out.push_str("let input: slicer_ir::IndexedTriangleSet = slicer_ir::IndexedTriangleSet {\n");
    out.push_str("    vertices: vec![\n");
    for vertex in &mesh.vertices {
        out.push_str("        slicer_ir::Point3 {\n");
        writeln!(out, "            x: {},", f32_literal(vertex.x))
            .map_err(|_| invalid_input("vertex formatting failed"))?;
        writeln!(out, "            y: {},", f32_literal(vertex.y))
            .map_err(|_| invalid_input("vertex formatting failed"))?;
        writeln!(out, "            z: {},", f32_literal(vertex.z))
            .map_err(|_| invalid_input("vertex formatting failed"))?;
        out.push_str("        },\n");
    }
    out.push_str("    ],\n    indices: vec![");
    for (index, value) in mesh.indices.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        write!(out, "{value}u32").map_err(|_| invalid_input("index formatting failed"))?;
    }
    out.push_str("],\n};\n");
    Ok(())
}

/// Format a concrete failing input as a readable Rust fixture statement block.
///
/// The returned text starts with the supplied case name and root-cause comment,
/// then binds `input` to a value of one of these types: `f32`, `f64`,
/// `(Vec<u16>, Vec<(u16, u16)>)`, `slicer_ir::LayerCollectionIR`,
/// `Vec<slicer_ir::ExPolygon>` or `slicer_ir::IndexedTriangleSet`. It never
/// references the feature-gated [`SmallGraph`], [`GeometryCase`] or
/// [`ConcreteInput`], and it needs no Proptest RNG, [`super::PropertyRun`]
/// context or slicing engine to compile or evaluate.
///
/// Floating-point values are emitted with `from_bits`, so signed zero, both
/// infinities and every NaN payload survive. Layer fixtures reference the
/// `test`-gated SDK fixture bases through struct update syntax; only the entity
/// and travel-move fields are representable, and a layer carrying any other
/// side table is rejected rather than silently truncated. An empty case name or
/// root cause is rejected, and comment/string content is escaped without
/// executing it.
///
/// This is not a generated `#[test]`: the owner must add an independently
/// justified assertion against the real SUT.
///
/// # Errors
/// Returns [`DomainError::InvalidInput`] for empty or whitespace-only metadata,
/// a layer side table the formatter cannot represent, or a comment/string that
/// cannot be rendered as one readable line.
pub fn format_regression_fixture(
    name: &str,
    root_cause: &str,
    input: &ConcreteInput,
) -> Result<String, DomainError> {
    let name = comment_line("Regression", name)?;
    let root_cause = comment_line("Root cause", root_cause)?;
    let mut out = String::new();
    out.push_str(&name);
    out.push('\n');
    out.push_str(&root_cause);
    out.push('\n');
    match input {
        ConcreteInput::F32Bits(bits) => {
            writeln!(out, "let input: f32 = f32::from_bits(0x{bits:08x});")
                .map_err(|_| invalid_input("scalar formatting failed"))?;
        }
        ConcreteInput::F64Bits(bits) => {
            writeln!(
                out,
                "let input: f64 = {};",
                f64_literal(f64::from_bits(*bits))
            )
            .map_err(|_| invalid_input("scalar formatting failed"))?;
        }
        ConcreteInput::Graph(graph) => format_graph(graph, &mut out)?,
        ConcreteInput::Layer(layer) => format_layer(layer, &mut out)?,
        ConcreteInput::Geometry(case) => format_geometry(&case.polygons, &mut out)?,
        ConcreteInput::Mesh(mesh) => format_mesh(mesh, &mut out)?,
    }
    Ok(out)
}
