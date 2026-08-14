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

// ── infill_ir_to_prior_regions ───────────────────────────────────────────

/// Marshal the committed `InfillIR`'s region buckets into the WIT
/// `prior-infill-region` records passed to `run-infill-postprocess` as the
/// `prior-infill` parameter (ADR-0028 Option 1b). Read-only copies — the
/// `infill-output-builder` stays write-only.
pub fn infill_ir_to_prior_regions(
    infill: &slicer_ir::InfillIR,
) -> Vec<crate::host::PriorInfillRegion> {
    use crate::marshal::leaf::ir_to_wit_extrusion_path;
    infill
        .regions
        .iter()
        .map(|r| crate::host::PriorInfillRegion {
            object_id: r.object_id.clone(),
            region_id: r.region_id.to_string(),
            sparse_infill: r
                .sparse_infill
                .iter()
                .map(ir_to_wit_extrusion_path)
                .collect(),
            solid_infill: r
                .solid_infill
                .iter()
                .map(ir_to_wit_extrusion_path)
                .collect(),
            ironing: r.ironing.iter().map(ir_to_wit_extrusion_path).collect(),
        })
        .collect()
}

// ── authored-coloring grant ──────────────────────────────────────────────

/// The claim a module must disclose in its manifest to be eligible for the
/// authored-coloring grant.
pub const AUTHORED_COLORING_CLAIM: &str = "claim:authored-coloring";

/// Pure two-sided grant predicate for authored per-path tool selection.
///
/// A module may author `ExtrusionPath3D::tool_index` for a region **only** when
/// both sides agree:
///
/// 1. **Module side (disclosure).** The module's manifest claims include
///    [`AUTHORED_COLORING_CLAIM`].
/// 2. **Config side (opt-in).** At least one fill-role claim this module
///    actually *holds* for the region is listed in the profile's
///    `fill_authored_coloring` key.
///
/// Disclosed-but-not-listed is denied; listed-but-not-disclosed is denied.
/// Total and panic-free: any empty input yields `false`.
pub fn authored_coloring_granted(
    held_fill_claims: &[String],
    fill_authored_coloring: &[String],
    disclosed_claims: &[String],
) -> bool {
    if !disclosed_claims
        .iter()
        .any(|c| c == AUTHORED_COLORING_CLAIM)
    {
        return false;
    }
    held_fill_claims
        .iter()
        .any(|held| fill_authored_coloring.iter().any(|listed| listed == held))
}

/// Per-dispatch authority for the authored-coloring strip performed at the
/// commit boundary in [`convert_infill_output`].
///
/// `granted_regions` holds `(object_id, region_id)` for every region where
/// [`authored_coloring_granted`] returned `true` for the dispatching module.
/// Absence from the set is a denial — the default is deny.
#[derive(Debug, Clone, Default)]
pub struct AuthoredColoringContext {
    /// Number of tools visible to the guest via the `tool-count` host function.
    /// Authored indices must be strictly less than this.
    pub tool_count: u32,
    /// `(object_id, region_id)` pairs the module is granted to color.
    pub granted_regions: std::collections::HashSet<(String, u64)>,
}

impl AuthoredColoringContext {
    /// True when `path_tool` may be kept for the given region.
    fn allows(&self, object_id: &str, region_id: u64, path_tool: u32) -> bool {
        path_tool < self.tool_count
            && self
                .granted_regions
                .contains(&(object_id.to_string(), region_id))
    }
}

/// Apply the authored-coloring grant to committed infill regions.
///
/// Every `tool_index` that is not covered by an active grant (no context, region
/// not granted, or index out of range) is silently reset to `None`, which means
/// "host resolves the region tool" — i.e. stripped back to the region tool.
/// This is never an error: an ungranted module simply has no effect.
fn enforce_authored_coloring(
    regions: &mut [slicer_ir::InfillRegion],
    authored: Option<&AuthoredColoringContext>,
) {
    for region in regions.iter_mut() {
        let object_id = region.object_id.clone();
        let region_id = region.region_id;
        for path in region
            .sparse_infill
            .iter_mut()
            .chain(region.solid_infill.iter_mut())
            .chain(region.ironing.iter_mut())
        {
            let keep = match (path.tool_index, authored) {
                (None, _) => continue,
                (Some(t), Some(ctx)) => ctx.allows(&object_id, region_id, t),
                (Some(_), None) => false,
            };
            if !keep {
                path.tool_index = None;
            }
        }
    }
}

// ── convert_infill_output ────────────────────────────────────────────────

/// Convert collected infill output into a slicer-ir `InfillIR`.
///
/// All paths are validated for NaN/Inf. If any origin tag is `Some`, regions
/// are grouped by `(object_id, region_id)` in stable first-seen order via
/// `OriginBucket`. Untagged pushes in identity mode are a contract violation.
///
/// If no origin tags are recorded (legacy callers), all output is emitted as
/// one synthetic region for backward compatibility.
///
/// `authored` carries the authored-coloring authority for this dispatch. It is
/// the authoritative enforcement point for `ExtrusionPath3D::tool_index`:
/// any authored tool without an active grant (or out of range) is silently
/// stripped to `None` here, so downstream consumers see only validated values.
/// `None` means "no module is granted" — every authored tool is stripped.
pub fn convert_infill_output(
    collected: &InfillOutputCollected,
    layer_index: u32,
    authored: Option<&AuthoredColoringContext>,
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

    let mut regions = bucket.into_regions();
    enforce_authored_coloring(&mut regions, authored);

    Ok(slicer_ir::InfillIR {
        schema_version: slicer_ir::SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        global_layer_index: layer_index,
        regions,
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
/// In identity mode, all three
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
            regions: vec![slicer_ir::slice_ir::SupportRegion {
                object_id: String::new(),
                region_id: 0,
                support_paths: support,
                interface_paths: interface,
                raft_paths: raft,
                ironing_paths: Vec::new(),
            }],
        });
    }

    // In identity mode, use ONE shared bucket across all three collections so
    // that first-seen origin order is global (not per-collection).  Each region
    // accumulator holds three separate path vecs, one per collection.
    struct SupportRegion {
        object_id: String,
        region_id: u64,
        support: Vec<slicer_ir::ExtrusionPath3D>,
        interface: Vec<slicer_ir::ExtrusionPath3D>,
        raft: Vec<slicer_ir::ExtrusionPath3D>,
    }

    fn mint_support_region(o: &OriginId) -> SupportRegion {
        SupportRegion {
            object_id: o.object_id.clone(),
            region_id: o.region_id,
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

    let regions = bucket
        .into_regions()
        .into_iter()
        .map(|r| slicer_ir::slice_ir::SupportRegion {
            object_id: r.object_id,
            region_id: r.region_id,
            support_paths: r.support,
            interface_paths: r.interface,
            raft_paths: r.raft,
            ironing_paths: Vec::new(),
        })
        .collect();

    Ok(slicer_ir::SupportIR {
        schema_version: slicer_ir::SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        global_layer_index: layer_index,
        regions,
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
                        dist_to_top_mm: 0.0,
                        overhang_distance_mm: None,
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
                    dist_to_top_mm: 0.0,
                    overhang_quartile: None,
                    overhang_distance_mm: None,
                },
                wall_index: *wall_index,
            });
    let resolved_seam_origin = collected.resolved_seam_origin.as_ref();

    let any_tagged = wall_origins.iter().any(Option::is_some)
        || collected.seam_candidate_origins.iter().any(Option::is_some)
        || collected.infill_areas_origins.iter().any(Option::is_some);

    fn mint_perimeter_region(o: &OriginId) -> slicer_ir::PerimeterRegion {
        slicer_ir::PerimeterRegion {
            variant_chain: Vec::new(),
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
