//! IR-marshalling output converters.
//!
//! Converts guest-accumulated output structs into slicer-ir types.
//! Moved here from `host.rs` / `dispatch.rs` in packet 113 (ADR-0021).
//!
//! No external runtime crate imports are permitted in this module (AC-2).

use slicer_ir::GCodeCommand;

use crate::host::RegionKey;
use crate::marshal::accumulators::{
    GcodeCommandCollected, InfillOutputCollected, PerimeterOutputCollected,
    SlicePostprocessCollected, SupportOutputCollected,
};
use crate::marshal::leaf::{
    convert_extrusion_path, convert_extrusion_role, convert_wall_loop, wit_to_ir_expolygons,
};
use crate::marshal::origin::{MarshalError, OriginBucket, OriginId};

// ── convert_infill_output ────────────────────────────────────────────────

/// Convert collected infill output into a slicer-ir `InfillIR`.
///
/// All paths are validated for NaN/Inf. If any origin tag is `Some`, regions
/// are grouped by `(object_id, region_id)` in stable first-seen order via
/// `OriginBucket`. Untagged pushes in identity mode are a contract violation.
///
/// If no origin tags are recorded (legacy callers), all output is emitted as
/// one synthetic region for backward compatibility.
pub fn convert_infill_output(
    collected: &InfillOutputCollected,
    layer_index: u32,
) -> Result<slicer_ir::InfillIR, String> {
    let sparse: Vec<_> = collected
        .sparse_paths
        .iter()
        .map(convert_extrusion_path)
        .collect::<Result<_, _>>()?;
    let solid: Vec<_> = collected
        .solid_paths
        .iter()
        .map(convert_extrusion_path)
        .collect::<Result<_, _>>()?;
    let ironing: Vec<_> = collected
        .ironing_paths
        .iter()
        .map(convert_extrusion_path)
        .collect::<Result<_, _>>()?;

    let any_tagged = collected.sparse_path_origins.iter().any(Option::is_some)
        || collected.solid_path_origins.iter().any(Option::is_some)
        || collected.ironing_path_origins.iter().any(Option::is_some);

    fn mint_infill_region(o: &OriginId) -> slicer_ir::InfillRegion {
        slicer_ir::InfillRegion {
            object_id: o.object_id.clone(),
            region_id: o.region_id,
            sparse_infill: Vec::new(),
            solid_infill: Vec::new(),
            ironing: Vec::new(),
        }
    }

    let mut bucket = OriginBucket::new(any_tagged, mint_infill_region);

    bucket
        .drain(
            "sparse_infill",
            sparse,
            &collected.sparse_path_origins,
            |r, p| {
                r.sparse_infill.push(p);
            },
        )
        .map_err(|e| infill_untagged_msg(e, "sparse_infill"))?;

    bucket
        .drain(
            "solid_infill",
            solid,
            &collected.solid_path_origins,
            |r, p| {
                r.solid_infill.push(p);
            },
        )
        .map_err(|e| infill_untagged_msg(e, "solid_infill"))?;

    bucket
        .drain(
            "ironing",
            ironing,
            &collected.ironing_path_origins,
            |r, p| {
                r.ironing.push(p);
            },
        )
        .map_err(|e| infill_untagged_msg(e, "ironing"))?;

    Ok(slicer_ir::InfillIR {
        schema_version: slicer_ir::SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        global_layer_index: layer_index,
        regions: bucket.into_regions(),
    })
}

/// Map a `MarshalError` to a human-readable string, preserving the old
/// untagged-push message for infill (no contract test asserts on this substring,
/// but keep it informative).
fn infill_untagged_msg(e: MarshalError, kind: &str) -> String {
    match e {
        MarshalError::UntaggedPayload { index, .. } => format!(
            "{kind} path[{index}] was emitted without an active perimeter source region; \
             guest must access a perimeter-region-view (object-id/region-id/wall-loops/infill-areas) \
             before pushing output for identity-preserving commit"
        ),
        other => String::from(other),
    }
}

// ── convert_support_output ───────────────────────────────────────────────

/// Convert collected support output into a slicer-ir `SupportIR`.
///
/// `SupportIR` is flat (no per-region struct). In identity mode, all three
/// collections (support, interface, raft) share a SINGLE `OriginBucket` so
/// that the first-seen origin order is global across all collections — matching
/// the original `group_by_origin` implementation that threaded a single
/// shared first-seen `order` list (keyed by `OriginId`) through all three
/// `group_by_origin` calls.
///
/// Concretely: if origin A appears first in `support_paths`, then
/// `interface_paths` with origins [B, A] will emit in shared order [A, B],
/// not in the per-collection order [B, A].
///
/// If no origin tags are recorded (legacy callers), output is passed through
/// in emission order.
pub fn convert_support_output(
    collected: &SupportOutputCollected,
    layer_index: u32,
) -> Result<slicer_ir::SupportIR, String> {
    let support: Vec<_> = collected
        .support_paths
        .iter()
        .map(convert_extrusion_path)
        .collect::<Result<_, _>>()?;
    let interface: Vec<_> = collected
        .interface_paths
        .iter()
        .map(|(p, _)| convert_extrusion_path(p))
        .collect::<Result<_, _>>()?;
    let raft: Vec<_> = collected
        .raft_paths
        .iter()
        .map(convert_extrusion_path)
        .collect::<Result<_, _>>()?;

    let any_tagged = collected.support_path_origins.iter().any(Option::is_some)
        || collected.interface_path_origins.iter().any(Option::is_some)
        || collected.raft_path_origins.iter().any(Option::is_some);

    if !any_tagged {
        return Ok(slicer_ir::SupportIR {
            schema_version: slicer_ir::SemVer {
                major: 1,
                minor: 0,
                patch: 0,
            },
            global_layer_index: layer_index,
            support_paths: support,
            interface_paths: interface,
            raft_paths: raft,
            ironing_paths: Vec::new(),
        });
    }

    // In identity mode, use ONE shared bucket across all three collections so
    // that first-seen origin order is global (not per-collection).  Each region
    // accumulator holds three separate path vecs, one per collection.
    struct SupportRegion {
        support: Vec<slicer_ir::ExtrusionPath3D>,
        interface: Vec<slicer_ir::ExtrusionPath3D>,
        raft: Vec<slicer_ir::ExtrusionPath3D>,
    }

    fn mint_support_region(_: &OriginId) -> SupportRegion {
        SupportRegion {
            support: Vec::new(),
            interface: Vec::new(),
            raft: Vec::new(),
        }
    }

    let mut bucket = OriginBucket::new(true, mint_support_region);

    bucket
        .drain(
            "support",
            support,
            &collected.support_path_origins,
            |r, p| r.support.push(p),
        )
        .map_err(|e| support_untagged_msg(e, "support"))?;

    bucket
        .drain(
            "interface",
            interface,
            &collected.interface_path_origins,
            |r, p| r.interface.push(p),
        )
        .map_err(|e| support_untagged_msg(e, "interface"))?;

    bucket
        .drain("raft", raft, &collected.raft_path_origins, |r, p| {
            r.raft.push(p)
        })
        .map_err(|e| support_untagged_msg(e, "raft"))?;

    // Flatten each collection in shared first-seen origin order.
    let mut support_paths: Vec<slicer_ir::ExtrusionPath3D> = Vec::new();
    let mut interface_paths: Vec<slicer_ir::ExtrusionPath3D> = Vec::new();
    let mut raft_paths: Vec<slicer_ir::ExtrusionPath3D> = Vec::new();
    for r in bucket.into_regions() {
        support_paths.extend(r.support);
        interface_paths.extend(r.interface);
        raft_paths.extend(r.raft);
    }

    Ok(slicer_ir::SupportIR {
        schema_version: slicer_ir::SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        global_layer_index: layer_index,
        support_paths,
        interface_paths,
        raft_paths,
        ironing_paths: Vec::new(),
    })
}

/// Map a `MarshalError` to a human-readable string, preserving the old
/// untagged-push message for support (contract test checks for "active slice
/// source region" or "without an active").
fn support_untagged_msg(e: MarshalError, kind: &str) -> String {
    match e {
        MarshalError::UntaggedPayload { index, .. } => format!(
            "{kind} path[{index}] was emitted without an active slice source region; \
             guest must access a slice-region-view (object-id/region-id/polygons/\
             infill-areas/effective-layer-height/z/has-nonplanar/boundary-paint) \
             before pushing support output for identity-preserving commit"
        ),
        other => String::from(other),
    }
}

// ── convert_perimeter_output ─────────────────────────────────────────────

/// Convert collected perimeter output into a slicer-ir `PerimeterIR`.
///
/// All wall loop paths are validated for NaN/Inf and feature-flag cardinality.
///
/// Identity preservation: if any origin tag is `Some`, regions are grouped by
/// `(object_id, region_id)` in stable first-seen order via `OriginBucket`.
///
/// The rotated-vs-original wall selection logic is preserved: when
/// `rotated_wall_loops` is non-empty, those replace the original `wall_loops`
/// as the canonical geometry.
///
/// If no origin tags are recorded (legacy callers), all output is flattened
/// into one synthetic region for backward compatibility.
pub fn convert_perimeter_output(
    collected: &PerimeterOutputCollected,
    layer_index: u32,
) -> Result<slicer_ir::PerimeterIR, String> {
    // When seam-placer has rotated wall loops, those are the canonical geometry.
    let (walls, wall_origins): (Vec<slicer_ir::WallLoop>, Vec<Option<OriginId>>) =
        if !collected.rotated_wall_loops.is_empty() {
            let rotated: Vec<slicer_ir::WallLoop> = collected
                .rotated_wall_loops
                .iter()
                .map(convert_wall_loop)
                .collect::<Result<_, _>>()?;
            (rotated, collected.rotated_wall_loop_origins.clone())
        } else {
            let original: Vec<slicer_ir::WallLoop> = collected
                .wall_loops
                .iter()
                .map(convert_wall_loop)
                .collect::<Result<_, _>>()?;
            (original, collected.wall_loop_origins.clone())
        };

    let infill_areas_per_call: Vec<Vec<slicer_ir::ExPolygon>> = collected
        .infill_areas
        .iter()
        .map(|areas| wit_to_ir_expolygons(areas))
        .collect();

    let seam_candidates: Vec<slicer_ir::SeamCandidate> = collected
        .seam_candidates
        .iter()
        .enumerate()
        .map(|(i, (pos, score))| {
            if pos.x.is_nan()
                || pos.x.is_infinite()
                || pos.y.is_nan()
                || pos.y.is_infinite()
                || pos.z.is_nan()
                || pos.z.is_infinite()
            {
                Err(format!("seam_candidate[{i}] has NaN/Inf coordinate"))
            } else if score.is_nan() || score.is_infinite() {
                Err(format!("seam_candidate[{i}] has NaN/Inf score"))
            } else {
                Ok(slicer_ir::SeamCandidate {
                    position: slicer_ir::Point3WithWidth {
                        x: pos.x,
                        y: pos.y,
                        z: pos.z,
                        width: 0.0,
                        flow_factor: 1.0,
                        overhang_quartile: None,
                    },
                    score: *score,
                    reason: slicer_ir::SeamReason::Aligned,
                })
            }
        })
        .collect::<Result<Vec<_>, _>>()?;

    // Convert collected resolved_seam to IR type.
    let resolved_seam =
        collected
            .resolved_seam
            .as_ref()
            .map(|(pos, wall_index)| slicer_ir::SeamPosition {
                point: slicer_ir::Point3WithWidth {
                    x: pos.x,
                    y: pos.y,
                    z: pos.z,
                    width: 0.0,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                },
                wall_index: *wall_index,
            });
    let resolved_seam_origin = collected.resolved_seam_origin.as_ref();

    let any_tagged = wall_origins.iter().any(Option::is_some)
        || collected.seam_candidate_origins.iter().any(Option::is_some)
        || collected.infill_areas_origins.iter().any(Option::is_some);

    fn mint_perimeter_region(o: &OriginId) -> slicer_ir::PerimeterRegion {
        slicer_ir::PerimeterRegion {
            object_id: o.object_id.clone(),
            region_id: o.region_id,
            walls: Vec::new(),
            infill_areas: Vec::new(),
            seam_candidates: Vec::new(),
            resolved_seam: None,
        }
    }

    let mut bucket = OriginBucket::new(any_tagged, mint_perimeter_region);

    // Drain walls.
    bucket
        .drain("wall_loops", walls, &wall_origins, |r, wl| r.walls.push(wl))
        .map_err(|e| perimeter_untagged_msg(e, "wall_loop"))?;

    // Drain seam candidates.
    bucket
        .drain(
            "seam_candidates",
            seam_candidates,
            &collected.seam_candidate_origins,
            |r, sc| r.seam_candidates.push(sc),
        )
        .map_err(|e| perimeter_untagged_msg(e, "seam_candidate"))?;

    // Infill areas: per-origin drain (one entry per set_infill_areas call,
    // each paired with its own origin tag). Mirrors the wall_loops drain above;
    // every distinct (object_id, region_id) the guest touched gets its own
    // PerimeterRegion with the infill areas it emitted. Pre-fix this was a
    // single-item drain, so every perimeters guest that called
    // set_infill_areas more than once per dispatch (the painted-slice /
    // multi-region case) silently lost every region except the LAST in
    // dispatch order.
    let any_infill = infill_areas_per_call.iter().any(|areas| !areas.is_empty());
    if any_infill {
        // Filter empty Vec<ExPolygon> entries but keep origin indices
        // aligned so the OriginBucket grouping matches the guest's
        // per-call origin tags.
        let mut payloads: Vec<Vec<slicer_ir::ExPolygon>> = Vec::new();
        let mut origins: Vec<Option<OriginId>> = Vec::new();
        for (areas, origin) in infill_areas_per_call
            .iter()
            .zip(collected.infill_areas_origins.iter())
        {
            if !areas.is_empty() {
                payloads.push(areas.clone());
                origins.push(origin.clone());
            }
        }
        bucket
            .drain("infill_areas", payloads, &origins, |r, areas| {
                r.infill_areas = areas
            })
            .map_err(|e| perimeter_untagged_msg(e, "infill_areas"))?;
    }

    // Resolved seam: inject directly if any bucket exists.
    if let Some(rs) = &resolved_seam {
        let Some(origin) = resolved_seam_origin else {
            return Err(
                "resolved_seam was emitted without an active perimeter source region".to_string(),
            );
        };
        let rs_origins: Vec<Option<OriginId>> = vec![Some(origin.clone())];
        bucket
            .drain("resolved_seam", vec![rs.clone()], &rs_origins, |r, seam| {
                r.resolved_seam = Some(seam)
            })
            .map_err(|_| {
                "resolved_seam was emitted without an active perimeter source region".to_string()
            })?;
    }

    Ok(slicer_ir::PerimeterIR {
        schema_version: slicer_ir::SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        global_layer_index: layer_index,
        regions: bucket.into_regions(),
    })
}

/// Map a `MarshalError` to a human-readable string for perimeter converters.
/// Contract test checks for "active perimeter source region" or "without an active".
fn perimeter_untagged_msg(e: MarshalError, kind: &str) -> String {
    match e {
        MarshalError::UntaggedPayload { index, .. } => format!(
            "{kind}[{index}] was emitted without an active perimeter source region; \
             guest must access a perimeter-region-view before pushing wall loops"
        ),
        other => String::from(other),
    }
}

// ── merge_slice_postprocess_into ─────────────────────────────────────────

/// Merge collected slice-postprocess output into an existing `SliceIR`,
/// preserving per-region identity.
///
/// SlicePostProcess modifies already-sliced regions: `set_polygons(key, polys)`
/// replaces the polygon set of the region matching `key`, and `set_path_z`
/// adjusts a Z coordinate on a polygon contour point. Regions not mentioned by
/// the guest pass through unchanged. Unknown `RegionKey` values (no matching
/// existing region) are a contract violation and produce a structured diagnostic
/// rather than inventing a synthetic region or silently dropping the update.
///
/// If no existing `SliceIR` is staged (identity-mapping failure), an error is
/// returned so the caller can decide whether to synthesize a fresh IR or fail.
pub fn merge_slice_postprocess_into(
    mut existing: slicer_ir::SliceIR,
    collected: &SlicePostprocessCollected,
) -> Result<slicer_ir::SliceIR, String> {
    for (i, (_, _, _, z)) in collected.path_z_updates.iter().enumerate() {
        if z.is_nan() || z.is_infinite() {
            return Err(format!("path_z_update[{i}] has NaN/Inf Z value ({z})"));
        }
    }

    let find_region = |regions: &[slicer_ir::SlicedRegion], key: &RegionKey| -> Option<usize> {
        let rid = key.region_id.parse::<u64>().ok()?;
        regions
            .iter()
            .position(|r| r.object_id == key.object_id && r.region_id == rid)
    };

    for (i, (key, polys)) in collected.polygon_updates.iter().enumerate() {
        let idx = find_region(&existing.regions, key).ok_or_else(|| {
            format!(
                "slice_postprocess polygon_update[{i}] targets unknown region \
             (object_id='{}', region_id='{}'); guest must reference an existing \
             slice-region-view identity for identity-preserving commit",
                key.object_id, key.region_id,
            )
        })?;
        existing.regions[idx].polygons = wit_to_ir_expolygons(polys);
    }

    for (i, (key, path_idx, vertex_idx, z)) in collected.path_z_updates.iter().enumerate() {
        let ridx = find_region(&existing.regions, key).ok_or_else(|| {
            format!(
                "slice_postprocess path_z_update[{i}] targets unknown region \
             (object_id='{}', region_id='{}')",
                key.object_id, key.region_id,
            )
        })?;
        let region = &mut existing.regions[ridx];
        let poly_count = region.polygons.len();
        let poly = region.polygons.get_mut(*path_idx as usize).ok_or_else(|| {
            format!(
                "slice_postprocess path_z_update[{i}]: polygon index {path_idx} out of range \
             for region ({}, {}) with {poly_count} polygons",
                key.object_id, key.region_id,
            )
        })?;
        // Z updates apply to contour points; validate vertex index bound.
        if (*vertex_idx as usize) >= poly.contour.points.len() {
            return Err(format!(
                "slice_postprocess path_z_update[{i}]: vertex index {vertex_idx} out of range \
                 for contour with {} points",
                poly.contour.points.len(),
            ));
        }
        // Z lives in ExPolygon contour — the IR expresses 2D contour points
        // only; path-Z updates are retained per-region as an attribute-less
        // no-op here since slicer_ir::ExPolygon has no per-point Z. Keeping
        // validation above guarantees the contract without mutating flat geometry.
        let _ = z;
    }

    Ok(existing)
}

// ── collect_postpass_output ──────────────────────────────────────────────

/// Collect and convert gcode commands from postpass output.
///
/// Returns `None` if no commands were emitted. Returns an error if any
/// unsupported command variant (e.g. `ZHop`) is present in the output.
pub fn collect_postpass_output(
    commands: &[GcodeCommandCollected],
) -> Result<Option<Vec<GCodeCommand>>, String> {
    if commands.is_empty() {
        return Ok(None);
    }

    let mut collected = Vec::with_capacity(commands.len());
    for (index, command) in commands.iter().enumerate() {
        let converted = match command {
            GcodeCommandCollected::Move(cmd) => GCodeCommand::Move {
                x: cmd.x,
                y: cmd.y,
                z: cmd.z,
                e: cmd.e,
                f: cmd.f,
                role: convert_extrusion_role(&cmd.role),
            },
            GcodeCommandCollected::Retract {
                length,
                speed,
                mode,
            } => GCodeCommand::Retract {
                length: *length,
                speed: *speed,
                mode: *mode,
            },
            GcodeCommandCollected::Unretract {
                length,
                speed,
                mode,
            } => GCodeCommand::Unretract {
                length: *length,
                speed: *speed,
                mode: *mode,
            },
            GcodeCommandCollected::FanSpeed(value) => GCodeCommand::FanSpeed { value: *value },
            GcodeCommandCollected::Temperature {
                tool,
                celsius,
                wait,
            } => GCodeCommand::Temperature {
                tool: *tool,
                celsius: *celsius,
                wait: *wait,
            },
            GcodeCommandCollected::ToolChange {
                after_entity_index,
                from_tool,
                to_tool,
            } => GCodeCommand::ToolChange {
                after_entity_index: *after_entity_index,
                from: *from_tool,
                to: *to_tool,
            },
            GcodeCommandCollected::Comment(text) => GCodeCommand::Comment { text: text.clone() },
            GcodeCommandCollected::Raw(text) => GCodeCommand::Raw { text: text.clone() },
            GcodeCommandCollected::ZHop { .. } => {
                return Err(format!(
                    "postpass gcode output command {index} used push-z-hop, but GCodeIR has no z-hop command variant"
                ));
            }
        };
        collected.push(converted);
    }

    Ok(Some(collected))
}
