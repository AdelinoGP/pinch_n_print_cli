//! Pure leaf IR↔WIT converters.
//!
//! Small scalar/enum/struct maps in both directions, with no bucketing or
//! accumulation logic.  Moved here from `host.rs` / `dispatch.rs` in packet
//! 113 (ADR-0021).
//!
//! AC-2: no wasm-runtime (embedder) references in this file.

use crate::host::{
    ExPolygon, ExtrusionPath3d, ExtrusionRole, PaintSemantic, PaintValue, Point2, Point3WithWidth,
    Polygon, WallFeatureFlag, WallLoopType, WallLoopView, WitMaterialBoundarySegment,
    WitRetractMode, WitWallBoundaryType, BUILTIN_EXTRUSION_ROLE_BRIM_TAG,
    BUILTIN_EXTRUSION_ROLE_INTERNAL_SOLID_TAG, BUILTIN_EXTRUSION_ROLE_PRIME_TOWER_TAG,
    BUILTIN_EXTRUSION_ROLE_SKIRT_TAG,
};

// postpass type alias used by convert_postpass_retract_mode.
use crate::host::postpass as ppm;

// ── WIT ↔ slicer-ir polygon conversion ────────────────────────────────

/// Convert WIT ExPolygon to slicer-ir ExPolygon.
pub fn wit_to_ir_expolygon(ep: &ExPolygon) -> slicer_ir::ExPolygon {
    slicer_ir::ExPolygon {
        contour: slicer_ir::Polygon {
            points: ep
                .contour
                .points
                .iter()
                .map(|p| slicer_ir::Point2 { x: p.x, y: p.y })
                .collect(),
        },
        holes: ep
            .holes
            .iter()
            .map(|h| slicer_ir::Polygon {
                points: h
                    .points
                    .iter()
                    .map(|p| slicer_ir::Point2 { x: p.x, y: p.y })
                    .collect(),
            })
            .collect(),
    }
}

/// Convert WIT ExPolygons to slicer-ir ExPolygons.
pub fn wit_to_ir_expolygons(eps: &[ExPolygon]) -> Vec<slicer_ir::ExPolygon> {
    eps.iter().map(wit_to_ir_expolygon).collect()
}

/// Convert slicer-ir ExPolygon to WIT ExPolygon.
pub fn ir_to_wit_expolygon(ep: &slicer_ir::ExPolygon) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: ep
                .contour
                .points
                .iter()
                .map(|p| Point2 { x: p.x, y: p.y })
                .collect(),
        },
        holes: ep
            .holes
            .iter()
            .map(|h| Polygon {
                points: h.points.iter().map(|p| Point2 { x: p.x, y: p.y }).collect(),
            })
            .collect(),
    }
}

/// Convert slicer-ir ExPolygons to WIT ExPolygons.
pub fn ir_to_wit_expolygons(eps: &[slicer_ir::ExPolygon]) -> Vec<ExPolygon> {
    eps.iter().map(ir_to_wit_expolygon).collect()
}

// ── Paint leaf maps ────────────────────────────────────────────────────

/// Convert slicer-ir PaintValue to WIT PaintValue.
/// Note: `PaintValue::Custom` has no WIT counterpart in the output type
/// (`PaintValue` in ir-types.wit has only flag/scalar/tool-index).
/// Custom values are represented as ToolIndex(0) on the WIT output side;
/// the lossless form is only available via PaintValueInput on the input path.
pub fn ir_to_wit_paint_value(v: &slicer_ir::PaintValue) -> PaintValue {
    match v {
        slicer_ir::PaintValue::Flag(b) => PaintValue::Flag(*b),
        slicer_ir::PaintValue::Scalar(s) => PaintValue::Scalar(*s),
        slicer_ir::PaintValue::ToolIndex(t) => PaintValue::ToolIndex(*t),
        slicer_ir::PaintValue::Custom(_) => PaintValue::ToolIndex(0),
    }
}

/// Convert a slicer-ir `PaintSemantic` to the WIT `PaintSemantic` enum.
pub fn ir_to_wit_paint_semantic(s: &slicer_ir::PaintSemantic) -> PaintSemantic {
    match s {
        slicer_ir::PaintSemantic::Material => PaintSemantic::Material,
        slicer_ir::PaintSemantic::FuzzySkin => PaintSemantic::FuzzySkin,
        slicer_ir::PaintSemantic::SupportEnforcer => PaintSemantic::SupportEnforcer,
        slicer_ir::PaintSemantic::SupportBlocker => PaintSemantic::SupportBlocker,
        slicer_ir::PaintSemantic::Custom(tag) => PaintSemantic::Custom(tag.clone()),
    }
}

/// Convert a slicer-ir `PaintSemantic` to a string key for paint segmentation views.
pub fn paint_semantic_to_string(s: &slicer_ir::PaintSemantic) -> String {
    match s {
        slicer_ir::PaintSemantic::Material => "material".to_string(),
        slicer_ir::PaintSemantic::FuzzySkin => "fuzzy-skin".to_string(),
        slicer_ir::PaintSemantic::SupportEnforcer => "support-enforcer".to_string(),
        slicer_ir::PaintSemantic::SupportBlocker => "support-blocker".to_string(),
        slicer_ir::PaintSemantic::Custom(tag) => tag.clone(),
    }
}

/// Convert a slicer-ir `PaintValue` to a WIT `PaintValueView` variant.
/// `PaintValue::Custom` has no WIT view counterpart; it is represented as
/// ToolIndex(0) on the view path (the Custom variant only exists on the input path).
pub fn ir_to_wit_paint_value_view(
    v: &slicer_ir::PaintValue,
) -> crate::host::prepass::PaintValueView {
    use crate::host::prepass::PaintValueView;
    match v {
        slicer_ir::PaintValue::Flag(b) => PaintValueView::Flag(*b),
        slicer_ir::PaintValue::Scalar(s) => PaintValueView::Scalar(*s),
        slicer_ir::PaintValue::ToolIndex(idx) => PaintValueView::ToolIndex(*idx),
        slicer_ir::PaintValue::Custom(_) => PaintValueView::ToolIndex(0),
    }
}

/// Convert a slicer-ir `PaintStroke` to a WIT `PaintStrokeView` record.
pub fn ir_to_wit_paint_stroke_view(
    stroke: &slicer_ir::PaintStroke,
) -> crate::host::prepass::PaintStrokeView {
    use crate::host::prepass::{PaintStrokeView, Point3};
    PaintStrokeView {
        triangles: stroke
            .triangles
            .iter()
            .flat_map(|triangle| triangle.iter())
            .map(|point| Point3 {
                x: point.x,
                y: point.y,
                z: point.z,
            })
            .collect(),
        semantic: paint_semantic_to_string(&stroke.semantic),
        value: ir_to_wit_paint_value_view(&stroke.value),
    }
}

/// Convert a slicer-ir `PaintLayer` to a WIT `PaintLayerView` record.
pub fn ir_to_wit_paint_layer_view(
    layer: &slicer_ir::PaintLayer,
) -> crate::host::prepass::PaintLayerView {
    use crate::host::prepass::PaintLayerView;
    PaintLayerView {
        semantic: paint_semantic_to_string(&layer.semantic),
        facet_values: layer
            .facet_values
            .iter()
            .map(|opt| opt.as_ref().map(ir_to_wit_paint_value_view))
            .collect(),
        strokes: layer
            .strokes
            .iter()
            .map(ir_to_wit_paint_stroke_view)
            .collect(),
    }
}

// ── Wall / extrusion-role leaf maps ───────────────────────────────────

/// Convert slicer-ir `LoopType` to WIT `WallLoopType`.
pub fn ir_to_wit_wall_loop_type(lt: &slicer_ir::LoopType) -> WallLoopType {
    match lt {
        slicer_ir::LoopType::Outer => WallLoopType::Outer,
        slicer_ir::LoopType::Inner => WallLoopType::Inner,
        slicer_ir::LoopType::ThinWall => WallLoopType::ThinWall,
        slicer_ir::LoopType::NonPlanarShell => WallLoopType::NonplanarShell,
        slicer_ir::LoopType::GapFill => WallLoopType::GapFill,
        _ => WallLoopType::Outer,
    }
}

/// Convert slicer-ir `ExtrusionRole` to WIT `ExtrusionRole`.
pub fn ir_to_wit_extrusion_role(role: &slicer_ir::ExtrusionRole) -> ExtrusionRole {
    match role {
        slicer_ir::ExtrusionRole::OuterWall => ExtrusionRole::OuterWall,
        slicer_ir::ExtrusionRole::InnerWall => ExtrusionRole::InnerWall,
        slicer_ir::ExtrusionRole::ThinWall => ExtrusionRole::ThinWall,
        slicer_ir::ExtrusionRole::TopSolidInfill => ExtrusionRole::TopSolidInfill,
        slicer_ir::ExtrusionRole::BottomSolidInfill => ExtrusionRole::BottomSolidInfill,
        slicer_ir::ExtrusionRole::SparseInfill => ExtrusionRole::SparseInfill,
        slicer_ir::ExtrusionRole::SupportMaterial => ExtrusionRole::SupportMaterial,
        slicer_ir::ExtrusionRole::SupportInterface => ExtrusionRole::SupportInterface,
        slicer_ir::ExtrusionRole::Ironing => ExtrusionRole::Ironing,
        slicer_ir::ExtrusionRole::BridgeInfill => ExtrusionRole::BridgeInfill,
        slicer_ir::ExtrusionRole::WipeTower => ExtrusionRole::WipeTower,
        slicer_ir::ExtrusionRole::Custom(tag) => ExtrusionRole::Custom(tag.clone()),
        slicer_ir::ExtrusionRole::PrimeTower => {
            ExtrusionRole::Custom(BUILTIN_EXTRUSION_ROLE_PRIME_TOWER_TAG.to_string())
        }
        slicer_ir::ExtrusionRole::Skirt => {
            ExtrusionRole::Custom(BUILTIN_EXTRUSION_ROLE_SKIRT_TAG.to_string())
        }
        slicer_ir::ExtrusionRole::Brim => {
            ExtrusionRole::Custom(BUILTIN_EXTRUSION_ROLE_BRIM_TAG.to_string())
        }
        slicer_ir::ExtrusionRole::InternalSolidInfill => {
            ExtrusionRole::Custom(BUILTIN_EXTRUSION_ROLE_INTERNAL_SOLID_TAG.to_string())
        }
        slicer_ir::ExtrusionRole::GapFill => ExtrusionRole::GapFill,
        _ => ExtrusionRole::OuterWall,
    }
}

/// Convert slicer-ir `ExtrusionPath3D` to WIT `ExtrusionPath3d`.
pub fn ir_to_wit_extrusion_path(path: &slicer_ir::ExtrusionPath3D) -> ExtrusionPath3d {
    ExtrusionPath3d {
        points: path
            .points
            .iter()
            .map(|p| Point3WithWidth {
                x: p.x,
                y: p.y,
                z: p.z,
                width: p.width,
                flow_factor: p.flow_factor,
                overhang_quartile: p.overhang_quartile,
            })
            .collect(),
        role: ir_to_wit_extrusion_role(&path.role),
        speed_factor: path.speed_factor,
    }
}

/// Convert slicer-ir `WallFeatureFlags` to WIT `WallFeatureFlag`.
pub fn ir_to_wit_wall_feature_flag(f: &slicer_ir::WallFeatureFlags) -> WallFeatureFlag {
    let mut custom: Vec<(String, PaintValue)> = f
        .custom
        .iter()
        .map(|(k, v)| {
            let pv = match v {
                slicer_ir::PaintValue::Flag(b) => PaintValue::Flag(*b),
                slicer_ir::PaintValue::Scalar(s) => PaintValue::Scalar(*s),
                slicer_ir::PaintValue::ToolIndex(t) => PaintValue::ToolIndex(*t),
                slicer_ir::PaintValue::Custom(_) => PaintValue::ToolIndex(0),
            };
            (k.clone(), pv)
        })
        .collect();
    custom.sort_by(|a, b| a.0.cmp(&b.0));
    WallFeatureFlag {
        tool_index: f.tool_index,
        fuzzy_skin: f.fuzzy_skin,
        is_bridge: f.is_bridge,
        is_thin_wall: f.is_thin_wall,
        skip_ironing: f.skip_ironing,
        custom,
    }
}

/// Convert slicer-ir `MaterialBoundarySegment` to its WIT counterpart.
pub fn ir_to_wit_material_boundary_segment(
    seg: &slicer_ir::MaterialBoundarySegment,
) -> WitMaterialBoundarySegment {
    WitMaterialBoundarySegment {
        point_range_start: seg.point_range.start,
        point_range_end: seg.point_range.end,
        near_tool: seg.near_tool,
        far_tool: seg.far_tool,
    }
}

/// Convert a WIT `MaterialBoundarySegment` to its slicer-ir counterpart.
pub fn wit_to_ir_material_boundary_segment(
    seg: &WitMaterialBoundarySegment,
) -> slicer_ir::MaterialBoundarySegment {
    slicer_ir::MaterialBoundarySegment {
        point_range: seg.point_range_start..seg.point_range_end,
        near_tool: seg.near_tool,
        far_tool: seg.far_tool,
    }
}

/// Convert slicer-ir `WallBoundaryType` to its WIT counterpart.
pub fn ir_to_wit_wall_boundary_type(bt: &slicer_ir::WallBoundaryType) -> WitWallBoundaryType {
    match bt {
        slicer_ir::WallBoundaryType::ExteriorSurface => WitWallBoundaryType::ExteriorSurface,
        slicer_ir::WallBoundaryType::Interior => WitWallBoundaryType::Interior,
        slicer_ir::WallBoundaryType::MaterialBoundary { segments } => {
            WitWallBoundaryType::MaterialBoundary(
                segments
                    .iter()
                    .map(ir_to_wit_material_boundary_segment)
                    .collect(),
            )
        }
    }
}

/// Convert a WIT `WallBoundaryType` to its slicer-ir counterpart.
pub fn wit_to_ir_wall_boundary_type(bt: &WitWallBoundaryType) -> slicer_ir::WallBoundaryType {
    match bt {
        WitWallBoundaryType::ExteriorSurface => slicer_ir::WallBoundaryType::ExteriorSurface,
        WitWallBoundaryType::Interior => slicer_ir::WallBoundaryType::Interior,
        WitWallBoundaryType::MaterialBoundary(segments) => {
            slicer_ir::WallBoundaryType::MaterialBoundary {
                segments: segments
                    .iter()
                    .map(wit_to_ir_material_boundary_segment)
                    .collect(),
            }
        }
    }
}

/// Convert slicer-ir `WallLoop` to WIT `WallLoopView`.
pub fn ir_to_wit_wall_loop(wl: &slicer_ir::WallLoop) -> WallLoopView {
    WallLoopView {
        perimeter_index: wl.perimeter_index,
        loop_type: ir_to_wit_wall_loop_type(&wl.loop_type),
        path: ir_to_wit_extrusion_path(&wl.path),
        feature_flags: wl
            .feature_flags
            .iter()
            .map(ir_to_wit_wall_feature_flag)
            .collect(),
        boundary_type: ir_to_wit_wall_boundary_type(&wl.boundary_type),
    }
}

// ── WIT→IR validation and path/role converters ────────────────────────

/// Validate that a float value is finite (not NaN or Inf).
pub fn validate_finite(value: f32, field: &str, index: usize) -> Result<(), String> {
    if value.is_nan() || value.is_infinite() {
        Err(format!("point[{index}].{field} is NaN or Inf ({value})"))
    } else {
        Ok(())
    }
}

/// Validate and convert a WIT `Point3WithWidth` to a slicer-ir `Point3WithWidth`.
pub fn convert_point(
    p: &Point3WithWidth,
    index: usize,
) -> Result<slicer_ir::Point3WithWidth, String> {
    validate_finite(p.x, "x", index)?;
    validate_finite(p.y, "y", index)?;
    validate_finite(p.z, "z", index)?;
    validate_finite(p.width, "width", index)?;
    validate_finite(p.flow_factor, "flow_factor", index)?;
    Ok(slicer_ir::Point3WithWidth {
        x: p.x,
        y: p.y,
        z: p.z,
        width: p.width,
        flow_factor: p.flow_factor,
        overhang_quartile: p.overhang_quartile,
    })
}

/// Convert a WIT `ExtrusionRole` to a slicer-ir `ExtrusionRole`.
pub fn convert_extrusion_role(role: &ExtrusionRole) -> slicer_ir::ExtrusionRole {
    match role {
        ExtrusionRole::OuterWall => slicer_ir::ExtrusionRole::OuterWall,
        ExtrusionRole::InnerWall => slicer_ir::ExtrusionRole::InnerWall,
        ExtrusionRole::ThinWall => slicer_ir::ExtrusionRole::ThinWall,
        ExtrusionRole::TopSolidInfill => slicer_ir::ExtrusionRole::TopSolidInfill,
        ExtrusionRole::BottomSolidInfill => slicer_ir::ExtrusionRole::BottomSolidInfill,
        ExtrusionRole::SparseInfill => slicer_ir::ExtrusionRole::SparseInfill,
        ExtrusionRole::SupportMaterial => slicer_ir::ExtrusionRole::SupportMaterial,
        ExtrusionRole::SupportInterface => slicer_ir::ExtrusionRole::SupportInterface,
        ExtrusionRole::Ironing => slicer_ir::ExtrusionRole::Ironing,
        ExtrusionRole::BridgeInfill => slicer_ir::ExtrusionRole::BridgeInfill,
        ExtrusionRole::WipeTower => slicer_ir::ExtrusionRole::WipeTower,
        ExtrusionRole::Custom(s) if s == BUILTIN_EXTRUSION_ROLE_PRIME_TOWER_TAG => {
            slicer_ir::ExtrusionRole::PrimeTower
        }
        ExtrusionRole::Custom(s) if s == BUILTIN_EXTRUSION_ROLE_SKIRT_TAG => {
            slicer_ir::ExtrusionRole::Skirt
        }
        ExtrusionRole::Custom(s) if s == BUILTIN_EXTRUSION_ROLE_BRIM_TAG => {
            slicer_ir::ExtrusionRole::Brim
        }
        ExtrusionRole::Custom(s) if s == BUILTIN_EXTRUSION_ROLE_INTERNAL_SOLID_TAG => {
            slicer_ir::ExtrusionRole::InternalSolidInfill
        }
        ExtrusionRole::Custom(s) => slicer_ir::ExtrusionRole::Custom(s.clone()),
        ExtrusionRole::GapFill => slicer_ir::ExtrusionRole::GapFill,
    }
}

/// Convert the layer-module WIT `retract-mode` variant to `slicer_ir::RetractMode`.
///
/// Used by `gcode-output-builder` host handlers to forward the retract emission
/// mode declared by guest modules (e.g. `path-optimization-default`) into the
/// host-side `GcodeCommandCollected` queue.
pub fn convert_layer_retract_mode(mode: &WitRetractMode) -> slicer_ir::RetractMode {
    match mode {
        WitRetractMode::Gcode => slicer_ir::RetractMode::Gcode,
        WitRetractMode::Firmware => slicer_ir::RetractMode::Firmware,
    }
}

/// Validate and convert a WIT `ExtrusionPath3d` to a slicer-ir `ExtrusionPath3D`.
///
/// Returns an error if any point coordinate is NaN or Inf (per docs/02_ir_schemas.md).
pub fn convert_extrusion_path(
    path: &ExtrusionPath3d,
) -> Result<slicer_ir::ExtrusionPath3D, String> {
    if path.speed_factor.is_nan() || path.speed_factor.is_infinite() {
        return Err(format!(
            "speed_factor is NaN or Inf ({})",
            path.speed_factor
        ));
    }
    let points: Result<Vec<_>, _> = path
        .points
        .iter()
        .enumerate()
        .map(|(i, p)| convert_point(p, i))
        .collect();
    Ok(slicer_ir::ExtrusionPath3D {
        points: points?,
        role: convert_extrusion_role(&path.role),
        speed_factor: path.speed_factor,
    })
}

/// Convert a WIT `WallLoopType` to a slicer-ir `LoopType`.
pub fn convert_wall_loop_type(lt: &WallLoopType) -> slicer_ir::LoopType {
    match lt {
        WallLoopType::Outer => slicer_ir::LoopType::Outer,
        WallLoopType::Inner => slicer_ir::LoopType::Inner,
        WallLoopType::ThinWall => slicer_ir::LoopType::ThinWall,
        WallLoopType::NonplanarShell => slicer_ir::LoopType::NonPlanarShell,
        WallLoopType::GapFill => slicer_ir::LoopType::GapFill,
    }
}

/// Convert a WIT `PaintValue` variant to a slicer-ir `PaintValue`.
pub fn convert_paint_value(v: &PaintValue) -> slicer_ir::PaintValue {
    match v {
        PaintValue::Flag(b) => slicer_ir::PaintValue::Flag(*b),
        PaintValue::Scalar(s) => slicer_ir::PaintValue::Scalar(*s),
        PaintValue::ToolIndex(t) => slicer_ir::PaintValue::ToolIndex(*t),
    }
}

/// Convert a WIT `WallFeatureFlag` to a slicer-ir `WallFeatureFlags`.
pub fn convert_wall_feature_flag(flag: &WallFeatureFlag) -> slicer_ir::WallFeatureFlags {
    use std::collections::HashMap;
    slicer_ir::WallFeatureFlags {
        tool_index: flag.tool_index,
        fuzzy_skin: flag.fuzzy_skin,
        is_bridge: flag.is_bridge,
        is_thin_wall: flag.is_thin_wall,
        skip_ironing: flag.skip_ironing,
        custom: HashMap::from_iter(
            flag.custom
                .iter()
                .map(|(k, v)| (k.clone(), convert_paint_value(v))),
        ),
    }
}

/// Validate and convert a WIT `WallLoopView` to a slicer-ir `WallLoop`.
///
/// Returns an error if any path coordinate is NaN or Inf, or if feature-flags
/// cardinality does not match path points (per docs/03 wall loop flag invariant).
pub fn convert_wall_loop(wl: &WallLoopView) -> Result<slicer_ir::WallLoop, String> {
    let path = convert_extrusion_path(&wl.path)?;
    if wl.feature_flags.len() != wl.path.points.len() {
        return Err(format!(
            "feature_flags length ({}) != path points length ({}); \
             per docs/03 wall loop flag invariant these must be parallel",
            wl.feature_flags.len(),
            wl.path.points.len()
        ));
    }
    Ok(slicer_ir::WallLoop {
        perimeter_index: wl.perimeter_index,
        loop_type: convert_wall_loop_type(&wl.loop_type),
        path,
        width_profile: slicer_ir::WidthProfile {
            widths: wl.path.points.iter().map(|p| p.width).collect(),
        },
        feature_flags: wl
            .feature_flags
            .iter()
            .map(convert_wall_feature_flag)
            .collect(),
        boundary_type: wit_to_ir_wall_boundary_type(&wl.boundary_type),
    })
}

// ── Retract mode converters ────────────────────────────────────────────

/// Convert the postpass-module WIT `RetractMode` to `slicer_ir::RetractMode`.
///
/// Intentionally separate from `convert_layer_retract_mode`: the two worlds
/// expose distinct generated types even though they carry the same semantic.
/// Body unchanged (AC-1b, packet 113 step 6).
pub fn convert_postpass_retract_mode(mode: &ppm::RetractMode) -> slicer_ir::RetractMode {
    match mode {
        ppm::RetractMode::Gcode => slicer_ir::RetractMode::Gcode,
        ppm::RetractMode::Firmware => slicer_ir::RetractMode::Firmware,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Round-trip: IR→WIT→IR must recover PrimeTower and Skirt from their
    /// Custom builtin tags.  This exercises `ir_to_wit_extrusion_role` (the
    /// outbound converter) composed with `convert_extrusion_role` (the
    /// inbound recovering converter).  Expected to PASS even before packet-115
    /// fixes, because `convert_extrusion_role` already recovers these tags.
    #[test]
    fn extrusion_role_round_trip_recovers_builtin_roles() {
        for role in [
            slicer_ir::ExtrusionRole::PrimeTower,
            slicer_ir::ExtrusionRole::Skirt,
            // RC4/G4: InternalSolidInfill has no dedicated WIT variant; it must
            // survive the boundary via its builtin Custom tag. A regression here
            // would silently turn internal solid shells into outer walls.
            slicer_ir::ExtrusionRole::InternalSolidInfill,
        ] {
            let wit = ir_to_wit_extrusion_role(&role);
            let recovered = convert_extrusion_role(&wit);
            assert_eq!(
                recovered, role,
                "round-trip must recover {:?} (via Custom tag), got {:?}",
                role, recovered
            );
        }
    }
}
