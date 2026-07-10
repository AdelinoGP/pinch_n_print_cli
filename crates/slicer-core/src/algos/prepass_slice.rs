// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/Slicing.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Pre-pass slicing algorithms.
//!
//! Produces `SliceIR` for a single layer by slicing object meshes at a given Z.

use std::collections::HashMap;
use std::fmt;

use crate::polygon_ops::{closing_ex, difference, intersection, offset, union, OffsetJoinType};
use crate::slice_mesh_ex;
use crate::triangle_mesh_slicer::apply_slice_closing_radius;
use slicer_ir::{
    ExPolygon, GlobalLayer, MeshIR, ObjectId, ObjectMesh, ObjectSurfaceData, Point2, Point3,
    Polygon, RegionKey, RegionMapIR, SliceIR, SlicedRegion, SurfaceClassificationIR, Transform3d,
    CURRENT_SLICE_IR_SCHEMA_VERSION,
};

use slicer_ir::BlackboardError;

/// Structured failures for the host built-in `PrePass::Slice` stage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayerSliceError {
    /// A layer referenced an `ObjectId` that is not present in `MeshIR`.
    UnknownObject {
        /// Layer that referenced the unknown object.
        layer_index: u32,
        /// The missing object identifier.
        object_id: ObjectId,
    },
    /// `commit_slice_builtin` could not commit the produced Vec — typically
    /// because `PrePass::Slice` was committed twice for the same print.
    Blackboard(BlackboardError),
    /// `PrePass::Slice` was invoked before `PrePass::LayerPlanning` committed
    /// `LayerPlanIR`.
    MissingLayerPlan,
}

impl fmt::Display for LayerSliceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownObject {
                layer_index,
                object_id,
            } => write!(
                f,
                "PrePass::Slice at layer {layer_index} references unknown object '{object_id}'"
            ),
            Self::Blackboard(inner) => write!(f, "PrePass::Slice blackboard error: {inner}"),
            Self::MissingLayerPlan => write!(
                f,
                "PrePass::Slice ran before PrePass::LayerPlanning committed LayerPlanIR"
            ),
        }
    }
}

impl From<BlackboardError> for LayerSliceError {
    fn from(value: BlackboardError) -> Self {
        Self::Blackboard(value)
    }
}

impl std::error::Error for LayerSliceError {}

// ============================================================================
// Internal geometry helpers
// ============================================================================

/// Apply a 4x4 column-major transform to a 3-D point.
fn transform_point(t: &Transform3d, p: &Point3) -> Point3 {
    crate::transform_point3(&t.matrix, *p)
}

/// Ray-casting point-in-polygon test (integer coordinate space).
fn point_in_ring(ring: &Polygon, pt: Point2) -> bool {
    let pts = &ring.points;
    if pts.len() < 3 {
        return false;
    }
    let px = pt.x;
    let py = pt.y;
    let mut inside = false;
    let n = pts.len();
    let mut j = n - 1;
    for i in 0..n {
        let xi = pts[i].x;
        let yi = pts[i].y;
        let xj = pts[j].x;
        let yj = pts[j].y;
        if (yi == py && xi == px) || (yj == py && xj == px) {
            return true;
        }
        if ((yi > py) != (yj > py))
            && (px as i128)
                < ((xj - xi) as i128 * (py - yi) as i128 / (yj - yi) as i128 + xi as i128)
        {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Return `true` if any of `polygons` contains `pt` in its contour ring.
fn any_polygon_contains(polygons: &[Polygon], pt: Point2) -> bool {
    polygons.iter().any(|p| point_in_ring(p, pt))
}

// ============================================================================
// Public helper: classify_region_surfaces
// ============================================================================

/// Detect whether the given region carries any bridge facets at this layer.
pub fn classify_region_surfaces(
    object_mesh: &ObjectMesh,
    surface_data: &ObjectSurfaceData,
    region_polygons: &[Polygon],
    layer_z: f32,
) -> bool {
    let mesh = &object_mesh.mesh;
    let t = &object_mesh.transform;
    let tri_count = mesh.indices.len() / 3;

    let bridge_set: std::collections::HashSet<u32> = surface_data
        .bridge_regions
        .iter()
        .flat_map(|br| br.facet_indices.iter().copied())
        .collect();

    for tri_idx in 0..tri_count {
        if !bridge_set.contains(&(tri_idx as u32)) {
            continue;
        }

        let i0 = mesh.indices[tri_idx * 3] as usize;
        let i1 = mesh.indices[tri_idx * 3 + 1] as usize;
        let i2 = mesh.indices[tri_idx * 3 + 2] as usize;
        if i0 >= mesh.vertices.len() || i1 >= mesh.vertices.len() || i2 >= mesh.vertices.len() {
            continue;
        }

        let wv0 = transform_point(t, &mesh.vertices[i0]);
        let wv1 = transform_point(t, &mesh.vertices[i1]);
        let wv2 = transform_point(t, &mesh.vertices[i2]);

        let fz_min = wv0.z.min(wv1.z).min(wv2.z);
        let fz_max = wv0.z.max(wv1.z).max(wv2.z);
        if fz_min <= layer_z && layer_z <= fz_max {
            let p0 = Point2::from_mm(wv0.x, wv0.y);
            let p1 = Point2::from_mm(wv1.x, wv1.y);
            let p2 = Point2::from_mm(wv2.x, wv2.y);
            if any_polygon_contains(region_polygons, p0)
                || any_polygon_contains(region_polygons, p1)
                || any_polygon_contains(region_polygons, p2)
            {
                return true;
            }
        }
    }

    false
}

// ============================================================================
// assemble_bridge_areas
// ============================================================================

/// Assemble expanded bridge polygons for a slice region.
pub fn assemble_bridge_areas(
    region: &mut SlicedRegion,
    surface_class: Option<&SurfaceClassificationIR>,
) {
    let Some(sc) = surface_class else {
        return;
    };
    let Some(obj_data) = sc.per_object.get(&region.object_id) else {
        return;
    };

    let mut best_orientation_deg = 0.0_f32;
    let mut best_bridge_length = 0.0_f32;

    for br in &obj_data.bridge_regions {
        if !br.is_valid {
            continue;
        }
        if br.xy_footprint.is_empty() {
            continue;
        }
        if !br.expansion_margin_mm.is_finite() || br.expansion_margin_mm < 0.0 {
            continue;
        }

        let footprint_as_expoly: Vec<ExPolygon> = br.xy_footprint.to_vec();
        let intersection_result = intersection(&footprint_as_expoly, &region.infill_areas);
        if intersection_result.is_empty() {
            continue;
        }

        let expanded: Vec<ExPolygon> = offset(
            &intersection_result,
            br.expansion_margin_mm,
            OffsetJoinType::Miter,
            0.0,
        );

        let final_polys = intersection(&expanded, &region.infill_areas);

        if final_polys.is_empty() {
            continue;
        }

        region.bridge_areas.extend(final_polys);

        if br.bridge_length_mm > best_bridge_length {
            best_bridge_length = br.bridge_length_mm;
            best_orientation_deg = br.bridge_direction_deg;
        }
    }

    region.bridge_orientation_deg = best_orientation_deg;
}

// ============================================================================
// assemble_flat_bridge_areas
// ============================================================================

/// Maximum flat gap span (mm) the enclosure discriminator will bridge.
///
/// A genuine flat bridge spans a gap *enclosed* by supported material on
/// opposite sides. We detect enclosure by morphologically **closing** the
/// supported region (dilate then erode by [`FLAT_BRIDGE_ENCLOSURE_RADIUS_MM`]):
/// closing re-fills only a gap pinched between supports on opposite sides
/// within `2·R`, and never a convex free-edge band. The dilation from each
/// supported side must reach the middle of the gap, so the closing radius is
/// half the widest span we accept. 24 mm comfortably clears the positive unit
/// test's 20 mm beam-over-gap (the dilations overlap by 4 mm, a solid fill)
/// while staying moderate enough not to close over unrelated free-edge notches.
const MAX_FLAT_BRIDGE_SPAN_MM: f64 = 24.0;

/// Morphological-closing radius (mm) used for the enclosure test — half the
/// widest bridgeable span (see [`MAX_FLAT_BRIDGE_SPAN_MM`]).
const FLAT_BRIDGE_ENCLOSURE_RADIUS_MM: f64 = MAX_FLAT_BRIDGE_SPAN_MM / 2.0;

/// Minimum fraction of a flat-unsupported component that must be covered by the
/// closing's re-filled ("enclosed-gap") mask for it to count as a bridge.
///
/// This gate separates genuine gap-spans from the thin closing artifacts a
/// free-edge bottom leaves along its supported boundary. Measured re-fill
/// fractions (closing radius = 12 mm):
///
/// | case                                    | comp   | refill | fraction |
/// |-----------------------------------------|--------|--------|----------|
/// | wedge interior slot (true bridge)       | 400 mm²| 393 mm²| **0.98** |
/// | beam-over-20 mm-gap unit test (true)    | 200 mm²|  51 mm²| **0.26** |
/// | wedge cantilever lip (free-edge, false) | 160 mm²| 0.9 mm²|   0.006  |
/// | wedge edge sliver (free-edge, false)    |10.4 mm²| 0.3 mm²|   0.03   |
///
/// A wide thin bridge only *partially* re-fills (the round-join dilations meet
/// in a narrow waist), so the true-positive floor is ~0.26 — far below 1.0 but
/// far above the artifact ceiling (~0.03). 0.1 sits in that empty band.
const FLAT_BRIDGE_ENCLOSURE_MIN_FRACTION: f64 = 0.1;

/// Detect and record *flat* (perfectly horizontal) unsupported bridge spans.
///
/// The sloped-overhang bridge pipeline ([`assemble_bridge_areas`]) never sees
/// perfectly-horizontal downward facets: they classify as
/// [`slicer_ir::FacetClass::BottomSurface`], not
/// [`slicer_ir::FacetClass::Overhang`], so a flat beam spanning a gap between
/// two supports is missed (its underside normal points straight down, so the
/// facet-normal test alone cannot tell a build-plate bottom from a
/// gap-spanning bridge). This function recovers those spans by intersecting
/// two pre-computed polygon sets:
///
/// 1. `bottom_surface_footprint` — the XY projection of the region's
///    `BottomSurface` facets (the region's flat-bottom footprint,
///    via [`crate::algos::mesh_analysis::bottom_surface_footprint`]), and
/// 2. `unsupported_region` — this layer's area **not** supported by the layer
///    below (the union of
///    `SurfaceClassificationIR.overhang_quartile_polygons[layer]`, populated
///    by the `PrePass::OverhangAnnotation` step),
///
/// then clipping the result to the region's own printable area
/// (`region.infill_areas`, mirroring [`assemble_bridge_areas`]) and appending
/// it to `region.bridge_areas`. `region.is_bridge` is set whenever any
/// flat-bridge area is added.
///
/// # First-layer / build-plate exclusion
///
/// Build-plate-contact and first-layer bottoms are excluded automatically:
/// `annotate_overhangs` never emits an unsupported region for layer 0 (a layer
/// with no predecessor is never overhanging), and a genuinely *supported*
/// bottom surface (its footprint sits entirely on material below) likewise has
/// an empty `difference(current, previous)` and therefore an empty
/// `unsupported_region`. In both cases this function is a no-op. The
/// discriminator is exactly "is this flat bottom supported by the layer
/// below?" — never the facet normal in isolation.
///
/// # Enclosure discriminator (free-edge vs. gap-span)
///
/// Being flat *and* unsupported is necessary but **not** sufficient to be a
/// bridge. A sloped solid's outward-growing perimeter band, or a cantilever
/// lip, is flat and unsupported at a **free edge** — there is no opposing
/// support — and is a genuine `Bottom surface`, not a bridge. Only a gap
/// **enclosed** by supported material on opposite sides (a slot ceiling, a beam
/// over a gap) is a flat bridge.
///
/// Enclosure is detected by morphologically **closing** the supported region:
///
/// * `support  = region.polygons \ unsupported_region` (this layer's
///   cross-section that *is* carried by the layer below),
/// * `closed   = dilate(support, R)` then `erode(support, R)` with
///   `R = FLAT_BRIDGE_ENCLOSURE_RADIUS_MM`,
/// * `refilled = closed \ support`.
///
/// Closing re-fills a gap only when supported material lies on opposite sides
/// within `2·R`; a convex free-edge band is never re-filled. So `refilled` is
/// the "enclosed-gap" mask, and a flat-unsupported component is a bridge iff it
/// overlaps `refilled`.
///
/// The closing radius is used **only** as a boolean enclosure test: a
/// flat-bridge component that overlaps `refilled` is kept in **full**, and the
/// reported `bridge_areas` geometry is the untouched flat-bottom ∩ unsupported
/// span — never the eroded closing geometry. This decouples the enclosure
/// radius from the reported area, so a radius large enough to span a wide gap
/// cannot pinch the reported span's middle (which would otherwise collapse the
/// detected area of a wide bridge).
pub fn assemble_flat_bridge_areas(
    region: &mut SlicedRegion,
    bottom_surface_footprint: &[ExPolygon],
    unsupported_region: &[ExPolygon],
    square_closing: bool,
) {
    if bottom_surface_footprint.is_empty() || unsupported_region.is_empty() {
        return;
    }

    // Flat unsupported = flat-bottom footprint ∩ not-supported-by-below.
    // Intersecting with the bottom-surface footprint restricts the result to
    // genuinely-flat bottoms, so sloped overhangs (already handled by
    // `assemble_bridge_areas`, and never `BottomSurface`) are not re-flagged
    // here and keep their min-length / anchor-width validity filtering.
    let flat_unsupported = intersection(bottom_surface_footprint, unsupported_region);
    if flat_unsupported.is_empty() {
        return;
    }

    // Clip to this region's printable area (matches `assemble_bridge_areas`).
    let flat_bridge = intersection(&flat_unsupported, &region.infill_areas);
    if flat_bridge.is_empty() {
        return;
    }

    // Enclosure discriminator: keep only flat-unsupported spans that are
    // pinched between supported material on opposite sides. See the fn doc.
    let support = difference(&region.polygons, unsupported_region);
    if support.is_empty() {
        // Entire cross-section is unsupported: nothing to enclose the flat
        // bottom against, so it is a free-floating bottom, not a bridge.
        return;
    }
    // Morphological closing (dilate-then-erode by R) used purely as a BOOLEAN
    // enclosure discriminator: does a gap ≤ 2·R get re-filled? The exact
    // roundness of the offset corners is irrelevant here, and the downstream
    // `FLAT_BRIDGE_ENCLOSURE_MIN_FRACTION` gate (10 %) is loose.
    //
    // `square_closing` (config `flat_bridge_square_closing`, default true)
    // selects `Square` joins: Round joins at a 12 mm radius tessellate every
    // corner of the (high-vertex) cross-section into dozens of arc points, so
    // the `Round` `closing_ex` (0.05 mm arc tolerance) exploded the intermediate
    // polygon and made this call ~92 % of PrePass::Slice wall-clock on Benchy
    // (~28 s / 30 s). `Square` adds a single bevel point per corner instead of
    // an arc — same ≤2·R gap-fill behaviour at ~1.8× less time / ~5× fewer
    // vertices. `Round` (`square_closing == false`) is retained as the opt-in
    // legacy path: bit-identical to pre-optimisation flat-bridge detection.
    let closed = if square_closing {
        let r_mm = FLAT_BRIDGE_ENCLOSURE_RADIUS_MM as f32;
        let dilated = offset(&support, r_mm, OffsetJoinType::Square, 0.0);
        offset(&dilated, -r_mm, OffsetJoinType::Square, 0.0)
    } else {
        closing_ex(&support, FLAT_BRIDGE_ENCLOSURE_RADIUS_MM)
    };
    let refilled = difference(&closed, &support);
    if refilled.is_empty() {
        // No pinched gap anywhere: every flat-unsupported area is a free-edge
        // bottom (growing perimeter band, cantilever lip). Not a bridge.
        return;
    }

    // A component is a genuine bridge iff the enclosed-gap mask (`refilled`)
    // covers a SUBSTANTIAL fraction of it. A real gap-span is almost fully
    // re-filled by the closing (~100% overlap); a free-edge bottom (an
    // outward-growing perimeter band, a cantilever lip) only *grazes*
    // `refilled` through thin dilate/erode artifacts along the shared support
    // boundary (a few % at most). The fraction gate rejects those slivers.
    //
    // Emit the FULL component (not the eroded closing geometry) so the enclosure
    // radius never shrinks the reported bridge area.
    let mut enclosed_bridge: Vec<ExPolygon> = Vec::new();
    for component in &flat_bridge {
        let comp_area = expoly_set_area(std::slice::from_ref(component));
        if comp_area <= 0.0 {
            continue;
        }
        let overlap = intersection(std::slice::from_ref(component), &refilled);
        let overlap_area = expoly_set_area(&overlap);
        if overlap_area >= FLAT_BRIDGE_ENCLOSURE_MIN_FRACTION * comp_area {
            enclosed_bridge.push(component.clone());
        }
    }
    if enclosed_bridge.is_empty() {
        return;
    }

    region.bridge_areas.extend(enclosed_bridge);
    region.is_bridge = true;
}

/// Total area (in workspace units², 1 unit = 100 nm) of a polygon set: the sum
/// of each ExPolygon's outer-contour area minus its holes, via the shoelace
/// formula. Used only for the flat-bridge enclosure fraction test, where the
/// absolute unit scaling cancels (it appears on both sides of the ratio).
fn expoly_set_area(polys: &[ExPolygon]) -> f64 {
    fn ring_area(ring: &Polygon) -> f64 {
        let pts = &ring.points;
        if pts.len() < 3 {
            return 0.0;
        }
        let mut a = 0.0_f64;
        for i in 0..pts.len() {
            let j = (i + 1) % pts.len();
            a += (pts[i].x as f64) * (pts[j].y as f64) - (pts[j].x as f64) * (pts[i].y as f64);
        }
        (a * 0.5).abs()
    }
    polys
        .iter()
        .map(|ep| ring_area(&ep.contour) - ep.holes.iter().map(ring_area).sum::<f64>())
        .sum()
}

// ============================================================================
// batch_slice_objects_by_layer
// ============================================================================

/// Pre-slice every object mesh referenced across `global_layers` in one
/// `slice_mesh_ex` call per object, instead of one call per (layer, object)
/// pair. `slice_mesh_ex` performs a full O(triangle-count) scan of the mesh
/// regardless of how many Z planes it is given — calling it once per layer,
/// as a naive per-layer loop does, makes the whole pre-pass slice step
/// O(layer-count × triangle-count) instead of O(triangle-count +
/// intersections). Returns raw (pre-closing-radius) polygons keyed by
/// `(global_layer_index, object_id)`.
pub fn batch_slice_objects_by_layer(
    mesh: &MeshIR,
    global_layers: &[GlobalLayer],
) -> HashMap<u32, HashMap<ObjectId, Vec<ExPolygon>>> {
    let mut layers_and_zs_by_object: HashMap<ObjectId, (Vec<u32>, Vec<f32>)> = HashMap::new();
    for layer in global_layers {
        for active in &layer.active_regions {
            let entry = layers_and_zs_by_object
                .entry(active.object_id.clone())
                .or_default();
            if entry.0.last() != Some(&layer.index) {
                entry.0.push(layer.index);
                entry.1.push(layer.z);
            }
        }
    }

    let mut result: HashMap<u32, HashMap<ObjectId, Vec<ExPolygon>>> = HashMap::new();
    for (object_id, (layer_indices, zs)) in layers_and_zs_by_object {
        let Some(object) = mesh.objects.iter().find(|o| o.id == object_id) else {
            continue;
        };
        let sliced = slice_mesh_ex(&object.mesh, &zs);
        for (layer_index, polygons) in layer_indices.into_iter().zip(sliced) {
            result
                .entry(layer_index)
                .or_default()
                .insert(object_id.clone(), polygons);
        }
    }
    result
}

// ============================================================================
// batch_bottom_surface_footprints
// ============================================================================

/// Pre-compute each object's whole-mesh bottom-surface XY footprint once.
/// [`crate::algos::mesh_analysis::bottom_surface_footprint`] depends only on
/// an object's mesh, transform, and facet classes — never on a specific
/// layer — so calling it from inside the per-layer loop (as
/// `execute_prepass_slice_single_layer` does when no cache is supplied)
/// recomputes an identical boolean-union result once per layer instead of
/// once per object. Returns `None` if `surface_class` is absent (matching
/// the guard `execute_prepass_slice_single_layer` already applies before
/// ever needing this footprint).
pub fn batch_bottom_surface_footprints(
    mesh: &MeshIR,
    surface_class: Option<&SurfaceClassificationIR>,
) -> HashMap<ObjectId, Vec<ExPolygon>> {
    let mut result = HashMap::new();
    let Some(sc) = surface_class else {
        return result;
    };
    for object in &mesh.objects {
        let Some(obj_data) = sc.per_object.get(&object.id) else {
            continue;
        };
        let footprint = crate::algos::mesh_analysis::bottom_surface_footprint(
            &object.mesh,
            &object.transform,
            &obj_data.facet_classes,
        );
        result.insert(object.id.clone(), footprint);
    }
    result
}

/// Layer-invariant and per-layer data pre-computed once for a whole
/// `PrePass::Slice` run, built by [`batch_slice_objects_by_layer`] and
/// [`batch_bottom_surface_footprints`], and reused across every
/// [`execute_prepass_slice_single_layer_with_cache`] call for that run.
pub struct PrepassSliceCache<'a> {
    /// This layer's raw (pre-closing-radius) slice polygons, keyed by object id.
    pub raw_polygons: &'a HashMap<ObjectId, Vec<ExPolygon>>,
    /// Whole-object bottom-surface XY footprint, keyed by object id.
    pub bottom_surface_footprint: &'a HashMap<ObjectId, Vec<ExPolygon>>,
}

// ============================================================================
// execute_prepass_slice_single_layer
// ============================================================================

/// Produce the `SliceIR` for `layer` by slicing every referenced object mesh
/// at `layer.z`.
pub fn execute_prepass_slice_single_layer(
    mesh: &MeshIR,
    layer: &GlobalLayer,
    surface_class: Option<&SurfaceClassificationIR>,
    region_map: Option<&RegionMapIR>,
) -> Result<SliceIR, LayerSliceError> {
    execute_prepass_slice_single_layer_impl(mesh, layer, surface_class, region_map, None)
}

/// Same as [`execute_prepass_slice_single_layer`], but takes pre-computed
/// per-object data from `cache` (see [`PrepassSliceCache`]) instead of
/// calling `slice_mesh_ex` and `bottom_surface_footprint` from scratch.
pub fn execute_prepass_slice_single_layer_with_cache(
    mesh: &MeshIR,
    layer: &GlobalLayer,
    surface_class: Option<&SurfaceClassificationIR>,
    region_map: Option<&RegionMapIR>,
    cache: &PrepassSliceCache<'_>,
) -> Result<SliceIR, LayerSliceError> {
    execute_prepass_slice_single_layer_impl(mesh, layer, surface_class, region_map, Some(cache))
}

fn execute_prepass_slice_single_layer_impl(
    mesh: &MeshIR,
    layer: &GlobalLayer,
    surface_class: Option<&SurfaceClassificationIR>,
    region_map: Option<&RegionMapIR>,
    cache: Option<&PrepassSliceCache<'_>>,
) -> Result<SliceIR, LayerSliceError> {
    let mut regions = Vec::with_capacity(layer.active_regions.len());
    for active in &layer.active_regions {
        let object = mesh
            .objects
            .iter()
            .find(|o| o.id == active.object_id)
            .ok_or_else(|| LayerSliceError::UnknownObject {
                layer_index: layer.index,
                object_id: active.object_id.clone(),
            })?;

        let (slice_closing_radius_mm, flat_bridge_square_closing) = if let Some(rm) = region_map {
            let key = RegionKey {
                global_layer_index: layer.index,
                object_id: active.object_id.clone(),
                region_id: active.region_id,
                variant_chain: Vec::new(),
            };
            let entry = rm.entries.get(&key);
            if entry.is_none() {
                log::warn!(
                    "PrePass::Slice: region_map present but missing entry for \
                     (layer={}, object={}, region={}) — falling back to legacy \
                     defaults; this indicates a partial RegionMap commit",
                    layer.index,
                    active.object_id,
                    active.region_id,
                );
                debug_assert!(
                    false,
                    "PrePass::Slice: region_map present but lookup miss for \
                     (layer={}, object={}, region={}); partial RegionMap \
                     violates the scheduler contract",
                    layer.index, active.object_id, active.region_id,
                );
            }
            match entry {
                Some(_) => {
                    let cfg = rm.config_for(&key);
                    (cfg.slice_closing_radius, cfg.flat_bridge_square_closing)
                }
                None => (0.0_f32, true),
            }
        } else {
            (0.0_f32, true)
        };

        let raw_polygons = match cache.and_then(|c| c.raw_polygons.get(&active.object_id)) {
            Some(cached) => cached.clone(),
            None => {
                let mut sliced = slice_mesh_ex(&object.mesh, &[layer.z]);
                sliced.pop().unwrap_or_default()
            }
        };
        let polygons = if slice_closing_radius_mm > 0.0 {
            apply_slice_closing_radius(raw_polygons, slice_closing_radius_mm)
        } else {
            raw_polygons
        };

        let is_bridge = if let Some(sc) = surface_class {
            if let Some(obj_data) = sc.per_object.get(&active.object_id) {
                let contours: Vec<Polygon> = if polygons.is_empty() {
                    let verts = &object.mesh.vertices;
                    if verts.is_empty() {
                        Vec::new()
                    } else {
                        let mut min_x = verts[0].x;
                        let mut max_x = verts[0].x;
                        let mut min_y = verts[0].y;
                        let mut max_y = verts[0].y;
                        for v in verts.iter().skip(1) {
                            if v.x < min_x {
                                min_x = v.x;
                            }
                            if v.x > max_x {
                                max_x = v.x;
                            }
                            if v.y < min_y {
                                min_y = v.y;
                            }
                            if v.y > max_y {
                                max_y = v.y;
                            }
                        }
                        vec![Polygon {
                            points: vec![
                                Point2::from_mm(min_x, min_y),
                                Point2::from_mm(max_x, min_y),
                                Point2::from_mm(max_x, max_y),
                                Point2::from_mm(min_x, max_y),
                            ],
                        }]
                    }
                } else {
                    polygons.iter().map(|ep| ep.contour.clone()).collect()
                };
                classify_region_surfaces(object, obj_data, &contours, layer.z)
            } else {
                false
            }
        } else {
            false
        };

        let mut sliced_region = SlicedRegion {
            object_id: active.object_id.clone(),
            region_id: active.region_id,
            polygons: polygons.clone(),
            infill_areas: polygons,
            nonplanar_surface: None,
            effective_layer_height: active.effective_layer_height,
            segment_annotations: HashMap::new(),
            variant_chain: Vec::new(),
            top_shell_index: None,
            bottom_shell_index: None,
            top_solid_fill: Vec::new(),
            bottom_solid_fill: Vec::new(),
            is_bridge,
            bridge_areas: vec![],
            bridge_orientation_deg: 0.0,
            sparse_infill_area: Vec::new(),
        };

        assemble_bridge_areas(&mut sliced_region, surface_class);

        // Flat (perfectly horizontal) unsupported bridge spans. The sloped
        // pipeline above only sees `Overhang`-derived clusters; a flat beam
        // over a gap has a `BottomSurface` underside and is missed there. The
        // discriminator is this layer's unsupported region (from the overhang
        // prepass), which is empty for layer 0 / build-plate contact, so
        // genuinely-supported bottoms are never flagged.
        if let Some(sc) = surface_class {
            if let Some(bands) = sc.overhang_quartile_polygons.get(&layer.index) {
                if let Some(obj_data) = sc.per_object.get(&active.object_id) {
                    // Union this layer's quartile bands into its unsupported region.
                    let mut unsupported: Vec<ExPolygon> = Vec::new();
                    for band in bands {
                        unsupported = union(&unsupported, &band.polygons);
                    }
                    if !unsupported.is_empty() {
                        let bottom_fp = match cache
                            .and_then(|c| c.bottom_surface_footprint.get(&active.object_id))
                        {
                            Some(cached) => cached.clone(),
                            None => crate::algos::mesh_analysis::bottom_surface_footprint(
                                &object.mesh,
                                &object.transform,
                                &obj_data.facet_classes,
                            ),
                        };
                        assemble_flat_bridge_areas(
                            &mut sliced_region,
                            &bottom_fp,
                            &unsupported,
                            flat_bridge_square_closing,
                        );
                    }
                }
            }
        }

        regions.push(sliced_region);
    }

    Ok(SliceIR {
        schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
        global_layer_index: layer.index,
        z: layer.z,
        regions,
    })
}
