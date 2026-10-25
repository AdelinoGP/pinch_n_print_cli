// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/GCode/SeamPlacer.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Default seam planner for Pinch 'n Print.
//!
//! The planner consumes the host-supplied active `SliceIR` region boundaries.
//! Mesh geometry remains available to the prepass interface for compatibility,
//! but is not a candidate source.

#![warn(missing_docs)]
#![warn(unused_imports)]

#[allow(dead_code)]
mod align;
#[allow(dead_code)]
mod comparator;
#[allow(dead_code)]
mod contours;
// The ray-cast visibility port (`compute_global_visibility`,
// `build_seam_candidates`, and ~20 supporting items) is dormant by design and
// exercised only by tests, so the module keeps a blanket allow.
//
// The paint-classification half is NOT dormant: `candidate_paint_classification`
// is imported below and called from `region_candidates`. Packet 206 originally
// shipped the exact-semantic discriminator here with *no* production caller,
// and this blanket allow is exactly what suppressed the warning that would
// have caught it. The lint cannot distinguish the two halves, so the guard is
// a production-path test instead: `seam_paint_moves_planner_resolved_seam`
// (`tests/seam_region_aware_planning_tdd.rs`) drives
// `run_aligned_planning_entries` and fails if the classifier stops being
// consulted. Do not delete it.
#[allow(dead_code)]
mod visibility;

use slicer_sdk::host_batch::{offset_polygons_batch, OffsetRequest};
use slicer_sdk::prelude::*;

use crate::comparator::EnforcedBlockedSeamPoint;
use crate::comparator::SeamSetup;
use crate::contours::project_point_onto_inset_boundary;
use crate::visibility::candidate_paint_classification;

/// Default extrusion flow width used for seam scoring. Units: mm.
const DEFAULT_FLOW_WIDTH_MM: f32 = 0.4;

/// Seam planning mode parsed from the `seam_mode` config key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SeamPlannerMode {
    /// Score-based nearest selection (default).
    Nearest,
    /// Rear-of-bed bias.
    Rear,
    /// Pseudo-random per-layer selection.
    Random,
    /// Vertically aligned seams.
    Aligned,
    /// Vertically aligned seams biased to the rear.
    AlignedBack,
}

/// Default seam planner that selects seam positions from active region
/// boundaries.
pub struct SeamPlannerDefault {
    /// Seam placement mode.
    mode: SeamPlannerMode,
}

fn region_candidates(region: &SeamPlanningRegionInput) -> Vec<ScoredSeamCandidate> {
    let width = if region.scoring_width.is_finite() && region.scoring_width > 0.0 {
        region.scoring_width
    } else {
        DEFAULT_FLOW_WIDTH_MM
    };
    let paint_annotations: Vec<_> = region
        .segment_annotations
        .iter()
        .map(|(semantic, contours)| (semantic.clone(), contours.as_slice()))
        .collect();
    let paint_annotations = (!paint_annotations.is_empty()).then_some(paint_annotations.as_slice());
    let mut candidates = Vec::new();
    for (contour_idx, polygon) in region.ex_polygons.iter().enumerate() {
        let points = polygon
            .contour
            .points
            .iter()
            .enumerate()
            .map(|(vertex_idx, point)| (point, Some(vertex_idx)))
            .chain(
                polygon
                    .holes
                    .iter()
                    .flat_map(|hole| hole.points.iter())
                    .map(|point| (point, None)),
            );
        for (point, vertex_idx) in points {
            let (point_type, central_enforcer) = vertex_idx
                .map(|vertex_idx| {
                    candidate_paint_classification(paint_annotations, contour_idx, vertex_idx)
                })
                .unwrap_or((EnforcedBlockedSeamPoint::Neutral, false));
            if point_type == EnforcedBlockedSeamPoint::Blocked {
                continue;
            }
            candidates.push(ScoredSeamCandidate {
                position: Point3WithWidth {
                    x: units_to_mm(point.x),
                    y: units_to_mm(point.y),
                    z: region.z,
                    width,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                    overhang_distance_mm: None,
                    dist_to_top_mm: 0.0,
                },
                score: if point_type == EnforcedBlockedSeamPoint::Enforced {
                    if central_enforcer {
                        2.0
                    } else {
                        1.0
                    }
                } else {
                    0.0
                },
                reason: SeamReason {
                    tag: if point_type == EnforcedBlockedSeamPoint::Enforced {
                        "enforced".to_string()
                    } else {
                        "aligned".to_string()
                    },
                },
            });
        }
    }
    candidates
}

/// Pick one candidate for a region.
///
/// Paint priority is applied first and applies to **every** mode: enforced
/// vertices score above neutral ones (`region_candidates`), so restricting to
/// the maximum score selects the enforced set whenever seam paint is present
/// and is a no-op otherwise (every neutral candidate scores 0.0). The mode
/// then only breaks ties *within* that set — an enforcer must not be
/// overridden by a geometric preference.
///
/// Note this narrows `Random`'s pool: `layer_index % candidates.len()` cycles
/// over the enforced set rather than the whole contour on painted models.
/// That is intended — random seam placement inside a painted enforcer region
/// is still enforced — and is pinned by
/// `random_mode_cycles_only_enforced_candidates` below.
fn choose_region_candidate(
    candidates: &[ScoredSeamCandidate],
    mode: SeamPlannerMode,
    layer_index: u32,
) -> Option<ScoredSeamCandidate> {
    let max_score = candidates
        .iter()
        .map(|candidate| candidate.score)
        .fold(f32::NEG_INFINITY, f32::max);
    let candidates = candidates
        .iter()
        .filter(|candidate| candidate.score == max_score)
        .collect::<Vec<_>>();
    if candidates.is_empty() {
        return None;
    }
    match mode {
        SeamPlannerMode::Aligned | SeamPlannerMode::Nearest => candidates
            .iter()
            .min_by(|left, right| {
                left.position
                    .y
                    .total_cmp(&right.position.y)
                    .then(left.position.x.total_cmp(&right.position.x))
            })
            .map(|candidate| (*candidate).clone()),
        SeamPlannerMode::AlignedBack | SeamPlannerMode::Rear => candidates
            .iter()
            .max_by(|left, right| {
                left.position
                    .y
                    .total_cmp(&right.position.y)
                    .then(right.position.x.total_cmp(&left.position.x))
            })
            .map(|candidate| (*candidate).clone()),
        SeamPlannerMode::Random => candidates
            .get(layer_index as usize % candidates.len())
            .map(|candidate| (*candidate).clone()),
    }
}

/// Deterministic fallback position for a candidate-less region.
///
/// Mirrors the sharpest-vertex degradation in
/// `slicer_core::perimeter_utils::generate_sharp_corner_seam_candidates`: the
/// boundary vertex with the largest absolute turn angle wins, the first
/// occurrence taking ties, and the region boundary's first point is the
/// backstop for degenerate rings. Blocked vertices are eligible here by
/// construction — this path only runs once the paint filter has already
/// emptied the ordinary candidate set — so a fully blocked region still gets
/// exactly one plan entry instead of silently dropping out of `SeamPlanIR`
/// (which the host aligned-seam lookup reports as a missing entry, code 6).
///
/// This is a *chosen-position* degradation, not a candidate: the returned
/// position is not appended to `scored_candidates`, whose scored content stays
/// exactly what `region_candidates` produced. Returns `None` only when the
/// region has no boundary point at all.
fn fallback_region_position(region: &SeamPlanningRegionInput) -> Option<Point3WithWidth> {
    let width = if region.scoring_width.is_finite() && region.scoring_width > 0.0 {
        region.scoring_width
    } else {
        DEFAULT_FLOW_WIDTH_MM
    };
    let mut first_point: Option<[f32; 2]> = None;
    let mut sharpest: Option<(f32, [f32; 2])> = None;
    for polygon in &region.ex_polygons {
        let rings = std::iter::once(&polygon.contour).chain(polygon.holes.iter());
        for ring in rings {
            let points = &ring.points;
            if points.len() < 3 {
                if let Some(point) = points.first() {
                    if first_point.is_none() {
                        first_point = Some([units_to_mm(point.x), units_to_mm(point.y)]);
                    }
                }
                continue;
            }
            for (index, point) in points.iter().enumerate() {
                let point_mm = [units_to_mm(point.x), units_to_mm(point.y)];
                if first_point.is_none() {
                    first_point = Some(point_mm);
                }
                let previous = &points[(index + points.len() - 1) % points.len()];
                let next = &points[(index + 1) % points.len()];
                let incoming = [
                    point_mm[0] - units_to_mm(previous.x),
                    point_mm[1] - units_to_mm(previous.y),
                ];
                let outgoing = [
                    units_to_mm(next.x) - point_mm[0],
                    units_to_mm(next.y) - point_mm[1],
                ];
                let incoming_len = (incoming[0] * incoming[0] + incoming[1] * incoming[1]).sqrt();
                let outgoing_len = (outgoing[0] * outgoing[0] + outgoing[1] * outgoing[1]).sqrt();
                if incoming_len == 0.0 || outgoing_len == 0.0 {
                    continue;
                }
                let cross = incoming[0] * outgoing[1] - incoming[1] * outgoing[0];
                let dot = incoming[0] * outgoing[0] + incoming[1] * outgoing[1];
                let turn = cross.atan2(dot).abs();
                if sharpest
                    .as_ref()
                    .is_none_or(|(best_turn, _)| turn > *best_turn)
                {
                    sharpest = Some((turn, point_mm));
                }
            }
        }
    }
    let position = sharpest.map(|(_, point)| point).or(first_point)?;
    Some(Point3WithWidth {
        x: position[0],
        y: position[1],
        z: region.z,
        width,
        flow_factor: 1.0,
        overhang_quartile: None,
        overhang_distance_mm: None,
        dist_to_top_mm: 0.0,
    })
}

/// One admitted region whose seam is chosen but not yet projected.
///
/// Pass A of [`run_region_planning_entries`] fills these in plan order; the
/// inset offset that pass B needs is issued for the whole vector at once.
struct PendingSeamEntry<'a> {
    /// Region the entry belongs to (key fields and polygon source).
    region: &'a SeamPlanningRegionInput,
    /// Candidate-selected or fallback position, still on the raw boundary.
    chosen_position: Point3WithWidth,
    /// Exactly what `region_candidates` produced for this region.
    scored_candidates: Vec<ScoredSeamCandidate>,
    /// Inset distance for this region's toolpath: `-0.5 * width`. Units: mm.
    inset_delta_mm: f32,
}

/// Build the seam plan for every admitted region of a print.
///
/// Two passes with **one** host offset call between them. Every region's inset
/// is an independent offset of its own polygons, so the 495-odd requests that
/// used to be 495 sequential host round trips (each one serial native clipper
/// work on the host thread) are issued as a single `offset_polygons_batch`,
/// which the host fans out over its rayon pool in input order. Per-request
/// arguments are byte-for-byte what the loop passed: the same polygons, the
/// same `-0.5 * width` delta, Miter joins, arc tolerance 0.0, no miter limit.
fn run_region_planning_entries(
    region_input: &SeamPlanningView,
    mode: SeamPlannerMode,
) -> Vec<SeamPlanEntry> {
    let mut regions: Vec<&SeamPlanningRegionInput> = region_input.regions.iter().collect();
    regions.sort_by(|left, right| {
        left.global_layer_index
            .cmp(&right.global_layer_index)
            .then(left.object_id.cmp(&right.object_id))
            .then(left.region_id.cmp(&right.region_id))
            .then(left.variant_chain.cmp(&right.variant_chain))
    });

    // Pass A — choose every region's seam, in plan order, without touching the
    // host. The scoring/fallback logic is unchanged; only the offset moves.
    let mut pending: Vec<PendingSeamEntry<'_>> = Vec::new();
    let mut previous_key: Option<(u32, String, String, Vec<(String, slicer_ir::PaintValue)>)> =
        None;
    for region in regions {
        let key = (
            region.global_layer_index,
            region.object_id.clone(),
            region.region_id.clone(),
            region.variant_chain.clone(),
        );
        if previous_key.as_ref() == Some(&key) {
            continue;
        }
        previous_key = Some(key);

        let scored_candidates = region_candidates(region);
        let chosen_position =
            match choose_region_candidate(&scored_candidates, mode, region.global_layer_index) {
                Some(chosen) => chosen.position,
                // Paint (or an empty boundary) can empty the scored set. The
                // planner must still emit exactly one entry per admitted region:
                // the host aligned-seam lookup keys off the region identity, and a
                // missing entry degrades to seam-placer code 6 and leaves the
                // layer unseamed. The fallback is deterministic and independent of
                // the scored path, which stays untouched; the reported scored
                // candidates stay exactly what `region_candidates` produced.
                None => match fallback_region_position(region) {
                    Some(position) => position,
                    None => continue,
                },
            };
        // The planner emits planner coordinates, not raw region-boundary
        // coordinates: the chosen seam is projected onto the inset boundary the
        // toolpath actually follows for this region's scoring width. The
        // candidate *generation* path (`region_candidates`) deliberately stays
        // on the original vertices — paint annotations are indexed by
        // `(contour_idx, vertex_idx)` on those vertices, and a per-vertex remap
        // would sever that indexing. Only the reported `chosen_position` moves.
        pending.push(PendingSeamEntry {
            region,
            inset_delta_mm: -0.5 * chosen_position.width, // mm
            chosen_position,
            scored_candidates,
        });
    }
    if pending.is_empty() {
        return Vec::new();
    }

    // The whole print's insets in one call. One request per pending entry, in
    // plan order, so `insets[i]` belongs to `pending[i]` by construction.
    let requests: Vec<OffsetRequest> = pending
        .iter()
        .map(|entry| OffsetRequest {
            polygons: entry.region.ex_polygons.clone(),
            delta_mm: entry.inset_delta_mm,
            join: OffsetJoinType::Miter,
            arc_tolerance_mm: 0.0,
            miter_limit: None,
        })
        .collect();
    let insets = offset_polygons_batch(&requests);

    // Pass B — project each chosen seam onto its own returned inset and emit
    // the entry. Field-for-field and order-for-order what the loop emitted.
    let mut entries = Vec::with_capacity(pending.len());
    for (entry, region_insets) in pending.into_iter().zip(insets) {
        let mut chosen_position = entry.chosen_position;
        if let Some(projected) = project_point_onto_inset_boundary(
            &region_insets,
            [chosen_position.x, chosen_position.y],
        ) {
            chosen_position.x = projected[0];
            chosen_position.y = projected[1];
        }
        entries.push(SeamPlanEntry {
            global_layer_index: entry.region.global_layer_index,
            object_id: entry.region.object_id.clone(),
            region_id: entry.region.region_id.clone(),
            variant_chain: entry.region.variant_chain.clone(),
            chosen_position,
            chosen_wall_index: 0,
            scored_candidates: entry.scored_candidates,
        });
    }
    entries
}

/// Build seam-plan entries directly from supplied active region polygons.
///
/// This pure entry point is used by the per-region contract tests. Mesh
/// vertices and layer-plan Z values are deliberately absent from this path.
pub fn run_aligned_planning_entries(
    region_input: &SeamPlanningView,
    aligned_back: bool,
) -> Vec<SeamPlanEntry> {
    run_region_planning_entries(
        region_input,
        if aligned_back {
            SeamPlannerMode::AlignedBack
        } else {
            SeamPlannerMode::Aligned
        },
    )
}

fn run_aligned_planning(
    setup: SeamSetup,
    _objects: &[MeshObjectView],
    _layer_plan: &LayerPlanView,
    region_input: &SeamPlanningView,
    output: &mut SeamPlanningOutput,
) -> Result<(), ModuleError> {
    let mode = match setup {
        SeamSetup::Aligned => SeamPlannerMode::Aligned,
        SeamSetup::AlignedBack => SeamPlannerMode::AlignedBack,
        _ => unreachable!("aligned planning only accepts aligned setups"),
    };
    for entry in run_region_planning_entries(region_input, mode) {
        output
            .push_seam_plan(entry)
            .map_err(|e| ModuleError::fatal(1, format!("push_seam_plan failed: {e}")))?;
    }
    Ok(())
}

#[slicer_module]
impl PrepassModule for SeamPlannerDefault {
    fn from_config(config: &ConfigView) -> Result<Self, ModuleError> {
        let mode = match config.get("seam_mode") {
            Some(ConfigValue::String(s)) => match s.as_str() {
                "nearest" => SeamPlannerMode::Nearest,
                "rear" => SeamPlannerMode::Rear,
                "random" => SeamPlannerMode::Random,
                "aligned" => SeamPlannerMode::Aligned,
                "aligned_back" => SeamPlannerMode::AlignedBack,
                other => {
                    return Err(ModuleError::fatal(1, format!("unknown seam_mode: {other}")));
                }
            },
            _ => SeamPlannerMode::Nearest,
        };

        Ok(Self { mode })
    }

    fn run_seam_planning(
        &self,
        objects: &[MeshObjectView],
        layer_plan: &LayerPlanView,
        output: &mut SeamPlanningOutput,
        _config: &ConfigView,
        region_input: &SeamPlanningView,
    ) -> Result<(), ModuleError> {
        match self.mode {
            SeamPlannerMode::Aligned => run_aligned_planning(
                SeamSetup::Aligned,
                objects,
                layer_plan,
                region_input,
                output,
            ),
            SeamPlannerMode::AlignedBack => run_aligned_planning(
                SeamSetup::AlignedBack,
                objects,
                layer_plan,
                region_input,
                output,
            ),
            mode => {
                for entry in run_region_planning_entries(region_input, mode) {
                    output.push_seam_plan(entry).map_err(|e| {
                        ModuleError::fatal(1, format!("push_seam_plan failed: {e}"))
                    })?;
                }
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Square `[x0, y0] .. [x0 + side, y0 + side]` region boundary, in mm.
    fn square_region(
        global_layer_index: u32,
        region_id: &str,
        x0: f32,
        y0: f32,
        side: f32,
        scoring_width: f32,
    ) -> SeamPlanningRegionInput {
        SeamPlanningRegionInput {
            global_layer_index,
            object_id: "obj".to_string(),
            region_id: region_id.to_string(),
            z: 0.2 * global_layer_index as f32,
            ex_polygons: vec![ExPolygon {
                contour: Polygon {
                    points: vec![
                        Point2::from_mm(x0, y0),
                        Point2::from_mm(x0 + side, y0),
                        Point2::from_mm(x0 + side, y0 + side),
                        Point2::from_mm(x0, y0 + side),
                    ],
                },
                holes: Vec::new(),
            }],
            scoring_width,
            ..Default::default()
        }
    }

    /// The pre-batching loop: one `host::offset_polygons` call per region,
    /// issued inside the entry loop.
    ///
    /// This is the reference the batched production path must reproduce entry
    /// for entry. It exists so the batch restructure can be checked item by
    /// item on native builds, where the host fan-out itself is compiled out —
    /// the two paths differ only in *where* each offset is computed and how
    /// results are carried back, which is precisely what a mis-association
    /// would break.
    fn serial_reference_entries(
        region_input: &SeamPlanningView,
        mode: SeamPlannerMode,
    ) -> Vec<SeamPlanEntry> {
        let mut regions: Vec<&SeamPlanningRegionInput> = region_input.regions.iter().collect();
        regions.sort_by(|left, right| {
            left.global_layer_index
                .cmp(&right.global_layer_index)
                .then(left.object_id.cmp(&right.object_id))
                .then(left.region_id.cmp(&right.region_id))
                .then(left.variant_chain.cmp(&right.variant_chain))
        });

        let mut entries = Vec::new();
        let mut previous_key: Option<(u32, String, String, Vec<(String, PaintValue)>)> = None;
        for region in regions {
            let key = (
                region.global_layer_index,
                region.object_id.clone(),
                region.region_id.clone(),
                region.variant_chain.clone(),
            );
            if previous_key.as_ref() == Some(&key) {
                continue;
            }
            previous_key = Some(key);

            let scored_candidates = region_candidates(region);
            let mut chosen_position = match choose_region_candidate(
                &scored_candidates,
                mode,
                region.global_layer_index,
            ) {
                Some(chosen) => chosen.position,
                None => match fallback_region_position(region) {
                    Some(position) => position,
                    None => continue,
                },
            };
            let inset_delta_mm = -0.5 * chosen_position.width; // mm
            let insets = host::offset_polygons(
                &region.ex_polygons,
                inset_delta_mm,
                OffsetJoinType::Miter,
                0.0,
            );
            if let Some(projected) =
                project_point_onto_inset_boundary(&insets, [chosen_position.x, chosen_position.y])
            {
                chosen_position.x = projected[0];
                chosen_position.y = projected[1];
            }
            entries.push(SeamPlanEntry {
                global_layer_index: region.global_layer_index,
                object_id: region.object_id.clone(),
                region_id: region.region_id.clone(),
                variant_chain: region.variant_chain.clone(),
                chosen_position,
                chosen_wall_index: 0,
                scored_candidates,
            });
        }
        entries
    }

    /// The batched restructure must be indistinguishable from the serial loop,
    /// and each region's projection must use **its own** inset distance.
    ///
    /// Two regions carry deliberately different `scoring_width` values (0.4 mm
    /// and 1.2 mm, so insets of 0.2 mm and 0.6 mm), which makes a
    /// result-to-region mis-association visible: the expected positions are
    /// derived from the fixture arithmetic below, not read back from output.
    #[test]
    fn batched_inset_offsets_match_the_serial_reference_per_region() {
        /// Tolerance for the offset round trip (Clipper2 rounds to 100 nm).
        const TOL_MM: f32 = 1e-4;

        let view = SeamPlanningView {
            regions: vec![
                // Region A: 10 mm square at the origin, 0.4 mm width -> 0.2 mm.
                square_region(0, "a", 0.0, 0.0, 10.0, 0.4),
                // Region B: 20 mm square at (5,5), 1.2 mm width -> 0.6 mm.
                square_region(1, "b", 5.0, 5.0, 20.0, 1.2),
                // Same key as region B: the dedupe must drop it on both paths.
                square_region(1, "b", 40.0, 40.0, 4.0, 9.9),
                // Region C: every vertex paint-blocked -> the fallback path,
                // which must be projected through the same batched inset.
                SeamPlanningRegionInput {
                    segment_annotations: vec![(
                        PaintSemantic::Custom("seam_blocker".to_string()),
                        vec![vec![
                            Some(PaintValue::Flag(true)),
                            Some(PaintValue::Flag(true)),
                            Some(PaintValue::Flag(true)),
                            Some(PaintValue::Flag(true)),
                        ]],
                    )],
                    ..square_region(2, "c", 0.0, 0.0, 20.0, 0.6)
                },
            ],
        };

        let batched = run_region_planning_entries(&view, SeamPlannerMode::Aligned);
        let serial = serial_reference_entries(&view, SeamPlannerMode::Aligned);

        assert_eq!(
            batched.len(),
            3,
            "one entry per admitted region, duplicate key dropped"
        );
        assert_eq!(
            batched, serial,
            "batching must not change any entry's content or order"
        );

        // Fixture-derived expectations: Aligned picks the lowest-y, then
        // lowest-x vertex, so each region projects its (x0, y0) corner inward by
        // its own half-width. A swapped result would move A to 0.6 or B to 0.2.
        let expected = [("a", 0.2_f32), ("b", 0.6), ("c", 0.3)];
        for (entry, (region_id, inset)) in batched.iter().zip(expected) {
            assert_eq!(entry.region_id, region_id);
            let x0 = if region_id == "b" { 5.0 } else { 0.0 };
            assert!(
                (entry.chosen_position.x - (x0 + inset)).abs() < TOL_MM
                    && (entry.chosen_position.y - (x0 + inset)).abs() < TOL_MM,
                "region {region_id} must be inset by {inset} mm from ({x0}, {x0}); got ({}, {})",
                entry.chosen_position.x,
                entry.chosen_position.y
            );
        }
        assert!(
            batched[2].scored_candidates.is_empty(),
            "the fallback region supplies a position, not a scored candidate"
        );
        assert_eq!(
            batched[0].scored_candidates.len(),
            4,
            "the scored path keeps every boundary candidate"
        );
    }

    fn candidate(x: f32, y: f32, score: f32) -> ScoredSeamCandidate {
        ScoredSeamCandidate {
            position: Point3WithWidth {
                x,
                y,
                z: 0.2,
                width: 0.4,
                flow_factor: 1.0,
                overhang_quartile: None,
                overhang_distance_mm: None,
                dist_to_top_mm: 0.0,
            },
            score,
            reason: SeamReason {
                tag: "aligned".to_string(),
            },
        }
    }

    /// Paint priority must gate `Random` like every other mode: the pool it
    /// cycles is the enforced set, not the whole contour.
    #[test]
    fn random_mode_cycles_only_enforced_candidates() {
        let candidates = vec![
            candidate(0.0, 0.0, 0.0),
            candidate(10.0, 0.0, 1.0),
            candidate(10.0, 10.0, 0.0),
            candidate(0.0, 10.0, 1.0),
        ];

        // Two enforced candidates, so the layer index cycles over 2, not 4.
        for layer_index in 0..6u32 {
            let chosen = choose_region_candidate(&candidates, SeamPlannerMode::Random, layer_index)
                .expect("a candidate");
            assert_eq!(chosen.score, 1.0, "Random must not pick a neutral vertex");
        }
        let first = choose_region_candidate(&candidates, SeamPlannerMode::Random, 0).unwrap();
        let second = choose_region_candidate(&candidates, SeamPlannerMode::Random, 1).unwrap();
        let third = choose_region_candidate(&candidates, SeamPlannerMode::Random, 2).unwrap();
        assert_ne!(
            (first.position.x, first.position.y),
            (second.position.x, second.position.y)
        );
        assert_eq!(
            (first.position.x, first.position.y),
            (third.position.x, third.position.y)
        );
    }

    /// With no paint every candidate scores 0.0, so the filter is a no-op and
    /// `Random` still cycles the full contour.
    #[test]
    fn random_mode_cycles_all_candidates_when_unpainted() {
        let candidates = vec![
            candidate(0.0, 0.0, 0.0),
            candidate(10.0, 0.0, 0.0),
            candidate(10.0, 10.0, 0.0),
            candidate(0.0, 10.0, 0.0),
        ];

        let picks: Vec<(f32, f32)> = (0..4u32)
            .map(|layer_index| {
                let chosen =
                    choose_region_candidate(&candidates, SeamPlannerMode::Random, layer_index)
                        .expect("a candidate");
                (chosen.position.x, chosen.position.y)
            })
            .collect();

        assert_eq!(picks.len(), 4);
        let mut unique = picks.clone();
        unique.dedup();
        assert_eq!(unique.len(), 4, "all four vertices must be reachable");
    }

    /// An empty candidate list must yield `None` rather than panicking on the
    /// `NEG_INFINITY` fold or the `% candidates.len()` modulo.
    #[test]
    fn empty_candidate_list_yields_none_in_every_mode() {
        for mode in [
            SeamPlannerMode::Aligned,
            SeamPlannerMode::AlignedBack,
            SeamPlannerMode::Nearest,
            SeamPlannerMode::Rear,
            SeamPlannerMode::Random,
        ] {
            assert!(choose_region_candidate(&[], mode, 0).is_none());
        }
    }

    /// Regression guard for the candidate-less silent skip.
    ///
    /// A region whose every vertex is `seam_blocker`-blocked previously emitted
    /// no `SeamPlanEntry` at all (`else { continue; }`), so the host aligned-seam
    /// lookup found no entry for the region key and `seam-placer` degraded with
    /// code 6. Every admitted region must now get exactly one keyed entry, with
    /// the sharpest-vertex fallback projected through the same inset boundary as
    /// the scored path. `scored_candidates` stays exactly what the paint filter
    /// produced — empty here — because the fallback is a chosen position, not a
    /// candidate.
    #[test]
    fn candidate_less_region_still_emits_one_keyed_plan_entry() {
        /// Half of `scoring_width = 0.4`: the inset distance per edge (mm).
        const HALF_WIDTH_MM: f32 = 0.2;
        /// Tolerance for the offset round trip (Clipper2 rounds to 100 nm).
        const TOL_MM: f32 = 1e-4;

        let blocked_at_every_vertex = vec![(
            PaintSemantic::Custom("seam_blocker".to_string()),
            vec![vec![
                Some(PaintValue::Flag(true)),
                Some(PaintValue::Flag(true)),
                Some(PaintValue::Flag(true)),
                Some(PaintValue::Flag(true)),
            ]],
        )];
        let region = SeamPlanningRegionInput {
            global_layer_index: 3,
            object_id: "obj".to_string(),
            region_id: "7".to_string(),
            variant_chain: Vec::new(),
            z: 0.6,
            height: 0.2,
            ex_polygons: vec![ExPolygon {
                contour: Polygon {
                    points: vec![
                        Point2::from_mm(0.0, 0.0),
                        Point2::from_mm(10.0, 0.0),
                        Point2::from_mm(10.0, 10.0),
                        Point2::from_mm(0.0, 10.0),
                    ],
                },
                holes: Vec::new(),
            }],
            segment_annotations: blocked_at_every_vertex,
            scoring_width: 0.4,
        };
        let view = SeamPlanningView {
            regions: vec![region],
        };

        for aligned_back in [false, true] {
            let entries = run_aligned_planning_entries(&view, aligned_back);
            assert_eq!(
                entries.len(),
                1,
                "a candidate-less region must still yield exactly one plan entry"
            );
            let entry = &entries[0];
            assert_eq!(
                (
                    entry.global_layer_index,
                    entry.object_id.as_str(),
                    entry.region_id.as_str(),
                    entry.variant_chain.len(),
                ),
                (3, "obj", "7", 0),
                "the fallback entry must carry the region's key so lookup matches"
            );
            assert!(
                entry.scored_candidates.is_empty(),
                "the fallback supplies a chosen position, not a scored candidate"
            );
            // Sharpest-vertex tie (all four corners are 90 degrees) goes to the
            // first boundary vertex, (0,0), inset by half the scoring width on
            // both axes — derived from the fixture, not read back.
            assert!(
                (entry.chosen_position.x - HALF_WIDTH_MM).abs() < TOL_MM
                    && (entry.chosen_position.y - HALF_WIDTH_MM).abs() < TOL_MM,
                "fallback must project the first sharpest vertex onto the inset \
                 boundary; got ({}, {})",
                entry.chosen_position.x,
                entry.chosen_position.y
            );
            assert!(
                entry.chosen_position.z == 0.6,
                "fallback must carry the supplied layer z"
            );
        }
    }
}
