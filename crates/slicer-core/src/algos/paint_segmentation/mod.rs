/// Paint-segmentation algorithm modules (ported from OrcaSlicer).
///
/// Coordinate constants divided by 100 (OrcaSlicer: 1 nm, ModularSlicer: 100 nm).
/// Spatial cell index for 2D line segments.
pub mod colorize;
/// Phase 7 — variant-chain composition: compose per-semantic outputs into disjoint chains per layer.
pub mod compose_variants;
/// Spatial grid for fast line-segment lookup and intersection queries.
pub mod edge_grid;
/// Phase 4f — walk the pruned graph and emit colored segments.
pub mod extract_segments;
/// Step 10 / AC-13 / D14 — slice modifier volumes and route to BASE segment_annotations.
pub mod modifier_volumes;
/// Painted line with semantic value and spatial cell membership.
pub mod painted_line;
/// Phase 3 driver — intersect painted triangles with layer Z plane.
pub mod phase3;
/// Phase 1 preprocess — extracts per-layer paint data from mesh objects.
pub mod preprocess;
/// Phase 6 — top/bottom surface propagation across layers.
pub mod top_bottom;
/// Z-plane intersection for triangles.
pub mod triangle_intersect;
/// Voronoi graph construction for MMU segmentation (boostvoronoi wrapper, H561 typed vertices).
pub mod voronoi_graph;
/// Phase 4d/4e — prune redundant arcs and dangling nodes from the MMU graph.
pub mod voronoi_prune;

// ---------------------------------------------------------------------------
// Step 9 — execute_paint_segmentation driver (AC-12)
// ---------------------------------------------------------------------------

use std::sync::Arc;

/// Errors from `execute_paint_segmentation`.
#[derive(Debug)]
pub enum PaintSegmentationError {
    /// boostvoronoi / MMU graph error during Voronoi propagation.
    Voronoi(voronoi_graph::MmuGraphError),
    /// An unexpected empty input was detected after the short-circuit checks passed.
    EmptyInputUnexpected(String),
    /// Catch-all for other errors.
    Other(String),
}

impl std::fmt::Display for PaintSegmentationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Voronoi(e) => write!(f, "voronoi error: {e}"),
            Self::EmptyInputUnexpected(s) => write!(f, "unexpected empty input: {s}"),
            Self::Other(s) => write!(f, "paint segmentation v2 error: {s}"),
        }
    }
}

impl std::error::Error for PaintSegmentationError {}

impl From<voronoi_graph::MmuGraphError> for PaintSegmentationError {
    fn from(e: voronoi_graph::MmuGraphError) -> Self {
        Self::Voronoi(e)
    }
}

/// Returns `true` if any object in `mesh` has at least one painted facet, stroke,
/// or a non-empty support-semantic modifier-volume.  Modifier volumes (D14) are
/// paint sources for the BASE-chain `segment_annotations`, so the short-circuit
/// MUST NOT skip them when the mesh has no facet/stroke paint.
fn mesh_has_any_paint(mesh: &slicer_ir::MeshIR) -> bool {
    for obj in &mesh.objects {
        if let Some(pd) = &obj.paint_data {
            for layer in &pd.layers {
                if layer.facet_values.iter().any(|v| v.is_some()) {
                    return true;
                }
                if !layer.strokes.is_empty() {
                    return true;
                }
            }
        }
        // D14: modifier-volume paint sources.
        for mv in &obj.modifier_volumes {
            let is_support_semantic = matches!(
                mv.config_delta.fields.get("subtype"),
                Some(slicer_ir::ConfigValue::String(s))
                    if s == "support_enforcer" || s == "support_blocker"
            );
            if is_support_semantic && !mv.mesh.vertices.is_empty() && !mv.mesh.indices.is_empty() {
                return true;
            }
        }
    }
    false
}

/// Phase 2–4f pipeline for one layer: build contours, build EdgeGrid, collect
/// painted lines, colorize, build MMU graph, prune, extract segments.
///
/// Returns `Vec<(poly_idx, Option<PaintValue>)>` — one entry per ColoredSegment —
/// ready for conversion to ExPolygons.
///
/// Retained for reference; superseded by the B-4 cell-decomposition path in
/// `execute_paint_segmentation` (which calls `cells_to_expolygons_by_color` directly).
#[cfg(feature = "host-algos")]
#[allow(dead_code)]
fn run_kernel_for_layer(
    layer_slice: &slicer_ir::SliceIR,
    mesh: &slicer_ir::MeshIR,
    num_color_states: usize,
) -> Result<Vec<extract_segments::ColoredSegment>, PaintSegmentationError> {
    use colorize::Contour;
    use triangle_intersect::Line;

    // Build per-region contours from polygons.
    let mut contours: Vec<Contour> = Vec::new();
    for region in &layer_slice.regions {
        for exp in &region.polygons {
            let pts = &exp.contour.points;
            if pts.len() < 2 {
                continue;
            }
            let edges: Vec<Line> = pts
                .windows(2)
                .map(|w| Line {
                    start: w[0],
                    end: w[1],
                })
                .chain(std::iter::once(Line {
                    start: *pts.last().unwrap(),
                    end: pts[0],
                }))
                .collect();
            if !edges.is_empty() {
                contours.push(Contour { edges });
            }
        }
    }

    if contours.is_empty() {
        return Ok(Vec::new());
    }

    // Phase 3 — collect painted lines.
    let painted_lines = phase3::collect_painted_lines(layer_slice, mesh, &contours);
    if painted_lines.is_empty() {
        return Ok(Vec::new());
    }

    // Phase 4a — post-process.
    let filtered = colorize::post_process_painted_lines(&contours, painted_lines);

    // Phase 4b — colorize contours.
    let colored_per_contour = colorize::colorize_contours(&contours, &filtered);

    // Flatten to one Vec<ColoredLine>.
    let colored_lines: Vec<colorize::ColoredLine> =
        colored_per_contour.iter().flatten().cloned().collect();

    if colored_lines.is_empty() {
        return Ok(Vec::new());
    }

    // Phase 4c — build MMU graph.
    let mut graph = voronoi_graph::MMU_Graph::from_colored_lines(&colored_lines)?;

    // Phase 4d/4e — prune.
    // remove_multiple_edges_in_vertices expects &[Vec<ColoredLine>] (colored_per_contour).
    voronoi_prune::remove_multiple_edges_in_vertices(&mut graph, &colored_per_contour);
    voronoi_prune::remove_nodes_with_one_arc(&mut graph);

    // Phase 4f — extract segments.
    let segments = extract_segments::extract_colored_segments(&graph, num_color_states);

    Ok(segments)
}

/// Convert colored segments for one layer into ExPolygons keyed by paint value.
///
/// Emits one ExPolygon per (walk × distinct color found in walk). Builds the
/// full perimeter per poly_idx AND the set of distinct non-None colors in that
/// walk in one pass, then emits one ExPolygon per (walk × color).
///
/// Walks with no painted color are emitted under `None` (unpainted/BASE) so
/// compose downstream sees them.
///
/// Superseded by `cells_to_expolygons_by_color` in the B-4 cell-decomposition path.
#[allow(dead_code)]
fn segments_to_expolygons_by_color(
    segments: &[extract_segments::ColoredSegment],
) -> std::collections::BTreeMap<Option<slicer_ir::PaintValue>, Vec<slicer_ir::ExPolygon>> {
    use slicer_ir::{ExPolygon, Polygon};
    use std::collections::{BTreeMap, BTreeSet};

    let mut result: BTreeMap<Option<slicer_ir::PaintValue>, Vec<ExPolygon>> = BTreeMap::new();

    if segments.is_empty() {
        return result;
    }

    // Per-walk point list + set of distinct non-None colors in that walk.
    let mut walk_pts: BTreeMap<usize, Vec<slicer_ir::Point2>> = BTreeMap::new();
    let mut walk_colors: BTreeMap<usize, BTreeSet<slicer_ir::PaintValue>> = BTreeMap::new();
    for seg in segments {
        walk_pts
            .entry(seg.poly_idx)
            .or_default()
            .push(seg.line.start);
        if let Some(c) = &seg.color {
            walk_colors
                .entry(seg.poly_idx)
                .or_default()
                .insert(c.clone());
        }
    }

    // Emit one ExPolygon per (walk × distinct color).
    for (poly_idx, mut pts) in walk_pts {
        if let Some(&first) = pts.first() {
            pts.push(first);
        }
        if pts.len() < 3 {
            continue;
        }
        // If the walk has no painted color, emit it under None (unpainted/BASE) so compose downstream sees it.
        let colors = walk_colors.get(&poly_idx);
        match colors {
            None => {
                result.entry(None).or_default().push(ExPolygon {
                    contour: Polygon {
                        points: pts.clone(),
                    },
                    holes: Vec::new(),
                });
            }
            Some(set) => {
                for color in set {
                    result
                        .entry(Some(color.clone()))
                        .or_default()
                        .push(ExPolygon {
                            contour: Polygon {
                                points: pts.clone(),
                            },
                            holes: Vec::new(),
                        });
                }
            }
        }
    }

    result
}

/// Legacy single-color variant: returns all painted polygons as one flat Vec.
/// Used only by the `#[cfg(not(feature = "host-algos"))]` stub path.
#[allow(dead_code)]
fn segments_to_expolygons(
    segments: &[extract_segments::ColoredSegment],
) -> Vec<slicer_ir::ExPolygon> {
    use slicer_ir::{ExPolygon, Point2, Polygon};
    use std::collections::BTreeMap;

    if segments.is_empty() {
        return Vec::new();
    }

    // Group by poly_idx.
    let mut by_poly: BTreeMap<usize, Vec<Point2>> = BTreeMap::new();
    for seg in segments {
        let pts = by_poly.entry(seg.poly_idx).or_default();
        pts.push(seg.line.start);
    }
    // Close each polygon.
    for (poly_idx, pts) in &mut by_poly {
        let _ = poly_idx; // suppress unused warning
        if let Some(&first) = pts.first() {
            pts.push(first);
        }
    }

    by_poly
        .into_values()
        .filter(|pts| pts.len() >= 3)
        .map(|points| ExPolygon {
            contour: Polygon { points },
            holes: Vec::new(),
        })
        .collect()
}

/// Execute the full paint-segmentation v2 pipeline.
///
/// # Short-circuit rules (AC-N2)
/// - Empty mesh → return input slice_ir unchanged.
/// - No painted facets or strokes → return input slice_ir unchanged.
/// - Empty region_map → return input slice_ir unchanged.
///
/// # Pipeline (AC-12)
/// For each layer: Phase 3 → Phase 4a/4b/4c/4d/4e/4f → Phase 7 compose →
/// rebuild SlicedRegions per (RegionKey × variant_chain) tuple.
pub fn execute_paint_segmentation(
    mesh: Arc<slicer_ir::MeshIR>,
    slice_ir: Arc<Vec<slicer_ir::SliceIR>>,
    region_map: Arc<slicer_ir::RegionMapIR>,
) -> Result<Arc<Vec<slicer_ir::SliceIR>>, PaintSegmentationError> {
    // ---- AC-N2: short-circuit checks ----------------------------------------
    if mesh.objects.is_empty() {
        return Ok(slice_ir.clone());
    }
    if !mesh_has_any_paint(&mesh) {
        return Ok(slice_ir.clone());
    }
    if region_map.entries.is_empty() {
        return Ok(slice_ir.clone());
    }

    // ---- Working copy --------------------------------------------------------
    let mut working: Vec<slicer_ir::SliceIR> = Vec::from_iter(slice_ir.iter().cloned());

    // ---- Step 10 / AC-13 / D14: slice modifier volumes once for all layers ----
    // Produces per-layer per-semantic polygon lists; routed onto BASE chains only.
    let layer_zs: Vec<f32> = working.iter().map(|s| s.z).collect();
    let modifier_vol_per_layer = modifier_volumes::slice_modifier_volumes(&mesh, &layer_zs);

    for i in 0..working.len() {
        let layer_slice = &working[i];

        if layer_slice.regions.is_empty() {
            continue;
        }

        let global_layer_index = layer_slice.global_layer_index;

        // Collect layer-total contours (BASE chain polygons and reference for per-color regions).
        let layer_total_contours: Vec<slicer_ir::ExPolygon> = layer_slice
            .regions
            .iter()
            .flat_map(|r| r.polygons.iter().cloned())
            .collect();

        // Determine num_color_states from PaintLayer facet values.
        // Retained for reference (was passed to run_kernel_for_layer in the old segment-walk path).
        #[allow(unused_variables)]
        let num_color_states: usize = {
            let mut max_tool: usize = 0;
            for obj in &mesh.objects {
                let Some(pd) = &obj.paint_data else { continue };
                for layer in &pd.layers {
                    for fv in &layer.facet_values {
                        if let Some(slicer_ir::PaintValue::ToolIndex(t)) = fv {
                            max_tool = max_tool.max(*t as usize + 1);
                        }
                    }
                }
            }
            max_tool.max(2)
        };

        // Determine the dominant PaintSemantic for this object (first painted layer's semantic).
        // Used to label the SemanticOutput entries with the correct semantic family.
        let dominant_semantic: slicer_ir::PaintSemantic = {
            let mut sem = slicer_ir::PaintSemantic::Material; // default
            'outer: for obj in &mesh.objects {
                let Some(pd) = &obj.paint_data else { continue };
                for layer in &pd.layers {
                    if layer.facet_values.iter().any(|v| v.is_some()) || !layer.strokes.is_empty() {
                        sem = layer.semantic.clone();
                        break 'outer;
                    }
                }
            }
            sem
        };

        // Run kernel (feature-gated).
        #[cfg(feature = "host-algos")]
        let polys_by_color = {
            use slicer_ir::slice_ir::BoundingBox2;
            use slicer_ir::Point2;

            // Compute bounding box of the layer contour points.
            let mut min_x = i64::MAX;
            let mut min_y = i64::MAX;
            let mut max_x = i64::MIN;
            let mut max_y = i64::MIN;
            for exp in &layer_total_contours {
                for pt in &exp.contour.points {
                    min_x = min_x.min(pt.x);
                    min_y = min_y.min(pt.y);
                    max_x = max_x.max(pt.x);
                    max_y = max_y.max(pt.y);
                }
            }
            let contour_bbox = if min_x <= max_x && min_y <= max_y {
                BoundingBox2 {
                    min: Point2 { x: min_x, y: min_y },
                    max: Point2 { x: max_x, y: max_y },
                }
            } else {
                BoundingBox2 {
                    min: Point2 { x: 0, y: 0 },
                    max: Point2 { x: 1, y: 1 },
                }
            };

            // Build contours → colored lines → MMU graph, then cell-decompose.
            use colorize::Contour;
            use triangle_intersect::Line;

            let mut contours: Vec<Contour> = Vec::new();
            for region in &working[i].regions {
                for exp in &region.polygons {
                    let pts = &exp.contour.points;
                    if pts.len() < 2 {
                        continue;
                    }
                    let edges: Vec<Line> = pts
                        .windows(2)
                        .map(|w| Line {
                            start: w[0],
                            end: w[1],
                        })
                        .chain(std::iter::once(Line {
                            start: *pts.last().unwrap(),
                            end: pts[0],
                        }))
                        .collect();
                    if !edges.is_empty() {
                        contours.push(Contour { edges });
                    }
                }
            }

            if contours.is_empty() {
                std::collections::BTreeMap::new()
            } else {
                let painted_lines = phase3::collect_painted_lines(&working[i], &mesh, &contours);
                if painted_lines.is_empty() {
                    std::collections::BTreeMap::new()
                } else {
                    let filtered = colorize::post_process_painted_lines(&contours, painted_lines);
                    let colored_per_contour = colorize::colorize_contours(&contours, &filtered);
                    let colored_lines: Vec<colorize::ColoredLine> =
                        colored_per_contour.iter().flatten().cloned().collect();

                    if colored_lines.is_empty() {
                        std::collections::BTreeMap::new()
                    } else {
                        match voronoi_graph::MMU_Graph::from_colored_lines(&colored_lines) {
                            Err(e) => return Err(PaintSegmentationError::from(e)),
                            Ok(mut graph) => {
                                voronoi_prune::remove_multiple_edges_in_vertices(
                                    &mut graph,
                                    &colored_per_contour,
                                );
                                voronoi_prune::remove_nodes_with_one_arc(&mut graph);
                                // B-4: cell decomposition replaces segment walk.
                                graph.cells_to_expolygons_by_color(
                                    &contour_bbox,
                                    &layer_total_contours,
                                )
                            }
                        }
                    }
                }
            }
        };
        #[cfg(not(feature = "host-algos"))]
        let polys_by_color: std::collections::BTreeMap<
            Option<slicer_ir::PaintValue>,
            Vec<slicer_ir::ExPolygon>,
        > = std::collections::BTreeMap::new();

        // AC-12 (d): rebuild SlicedRegions.
        //
        // For multi-value same-semantic paint (the common case for MMU: multiple
        // ToolIndex values of the Material semantic), each color is independent —
        // there is no cross-product intersection to compute. We therefore bypass
        // compose_variants (which is designed for multi-SEMANTIC fan-out, e.g.
        // Material × SupportEnforcer) and emit one SlicedRegion per (color, polys)
        // pair directly.
        //
        // The BASE chain (variant_chain = []) uses the full layer contours and
        // carries modifier-volume annotations (D14 invariant).
        {
            // Helper: produce a chain key `[(semantic_name, value)]` for one color.
            let sem_name: String = match &dominant_semantic {
                slicer_ir::PaintSemantic::Material => "material".to_owned(),
                slicer_ir::PaintSemantic::FuzzySkin => "fuzzy_skin".to_owned(),
                slicer_ir::PaintSemantic::SupportEnforcer => "support_enforcer".to_owned(),
                slicer_ir::PaintSemantic::SupportBlocker => "support_blocker".to_owned(),
                slicer_ir::PaintSemantic::Custom(name) => name.clone(),
            };

            let mut new_regions: Vec<slicer_ir::SlicedRegion> = Vec::new();

            // BASE chain — full layer contours; carries modifier-volume annotations.
            let base_segment_annotations = build_modifier_segment_annotations(
                i,
                &layer_total_contours,
                &modifier_vol_per_layer,
            );
            let base_chain_key: Vec<(String, slicer_ir::PaintValue)> = vec![];
            let matching_base: Vec<&slicer_ir::RegionKey> = region_map
                .entries
                .keys()
                .filter(|rk| {
                    rk.global_layer_index == global_layer_index
                        && rk.variant_chain == base_chain_key
                })
                .collect();
            if matching_base.is_empty() {
                if let Some(existing) = working[i].regions.first() {
                    new_regions.push(slicer_ir::SlicedRegion {
                        object_id: existing.object_id.clone(),
                        region_id: existing.region_id,
                        polygons: layer_total_contours.clone(),
                        variant_chain: base_chain_key.clone(),
                        segment_annotations: base_segment_annotations,
                        ..Default::default()
                    });
                }
            } else {
                for rk in matching_base {
                    new_regions.push(slicer_ir::SlicedRegion {
                        object_id: rk.object_id.clone(),
                        region_id: rk.region_id,
                        polygons: layer_total_contours.clone(),
                        variant_chain: base_chain_key.clone(),
                        segment_annotations: base_segment_annotations.clone(),
                        ..Default::default()
                    });
                }
            }

            // One painted chain per distinct (semantic, value) color.
            for (color_opt, polys) in &polys_by_color {
                let Some(value) = color_opt else { continue }; // skip None
                if polys.is_empty() {
                    continue;
                }
                let chain_key: Vec<(String, slicer_ir::PaintValue)> =
                    vec![(sem_name.clone(), value.clone())];

                let matching_keys: Vec<&slicer_ir::RegionKey> = region_map
                    .entries
                    .keys()
                    .filter(|rk| {
                        rk.global_layer_index == global_layer_index && rk.variant_chain == chain_key
                    })
                    .collect();

                if matching_keys.is_empty() {
                    if let Some(existing) = working[i].regions.first() {
                        new_regions.push(slicer_ir::SlicedRegion {
                            object_id: existing.object_id.clone(),
                            region_id: existing.region_id,
                            polygons: polys.clone(),
                            variant_chain: chain_key.clone(),
                            segment_annotations: std::collections::HashMap::new(),
                            ..Default::default()
                        });
                    }
                } else {
                    for rk in matching_keys {
                        new_regions.push(slicer_ir::SlicedRegion {
                            object_id: rk.object_id.clone(),
                            region_id: rk.region_id,
                            polygons: polys.clone(),
                            variant_chain: chain_key.clone(),
                            segment_annotations: std::collections::HashMap::new(),
                            ..Default::default()
                        });
                    }
                }
            }

            if !new_regions.is_empty() {
                working[i].regions = new_regions;
            }
        }
    }

    // ---- Phase 6 — top/bottom propagation (OrcaSlicer parity) ----------------
    //
    // OrcaSlicer order (MultiMaterialSegmentation.cpp:1331-1419,
    // PrintObjectSlice.cpp:924-1081, MultiMaterialSegmentation.cpp:2053-2092):
    //   Phase 4 (colorize + cell decomposition) →
    //   Phase 6 (top/bottom propagation, NEW outputs per extruder) →
    //   Phase 7 merge (diff_ex BASE − phase6 + append/union into per-color regions).
    //
    // Shell-window propagation: a top-painted facet propagates DOWN by
    // `top_shell_layers` layers; a bottom-painted facet propagates UP by
    // `bottom_shell_layers` layers. At shells = 0 both windows collapse to the
    // single layer slab — i.e. intersection(top_proj[l] ∪ bot_proj[l],
    // layer_input_polygons[l]) — preserving the first-cut behaviour. Shell
    // counts are read from the BASE ResolvedConfig (configs[0]); if absent the
    // ResolvedConfig defaults (top=3, bottom=3, matching OrcaSlicer) apply.
    #[cfg(feature = "host-algos")]
    {
        use crate::polygon_ops::{difference_ex, union_ex};
        use std::collections::BTreeMap;

        // Collect distinct (semantic, value) pairs present in the mesh. For
        // each pair build a painted-only IndexedTriangleSet that carries both
        // facet-painted triangles (vertex indices into the object's mesh) AND
        // stroke triangles (whose raw vertex coordinates are appended to the
        // subset's vertex pool with fresh contiguous indices).
        let mut painted_subsets: BTreeMap<
            (String, slicer_ir::PaintValue),
            (slicer_ir::PaintSemantic, slicer_ir::IndexedTriangleSet),
        > = BTreeMap::new();

        let sem_name = |s: &slicer_ir::PaintSemantic| -> String {
            match s {
                slicer_ir::PaintSemantic::Material => "material".to_owned(),
                slicer_ir::PaintSemantic::FuzzySkin => "fuzzy_skin".to_owned(),
                slicer_ir::PaintSemantic::SupportEnforcer => "support_enforcer".to_owned(),
                slicer_ir::PaintSemantic::SupportBlocker => "support_blocker".to_owned(),
                slicer_ir::PaintSemantic::Custom(name) => name.clone(),
            }
        };

        for obj in &mesh.objects {
            let Some(pd) = &obj.paint_data else { continue };
            for layer in &pd.layers {
                // Facet paint: triangles share the object's existing vertex pool.
                for (facet_idx, fv) in layer.facet_values.iter().enumerate() {
                    let Some(value) = fv else { continue };
                    let key = (sem_name(&layer.semantic), value.clone());
                    let entry = painted_subsets.entry(key).or_insert_with(|| {
                        (
                            layer.semantic.clone(),
                            slicer_ir::IndexedTriangleSet {
                                vertices: obj.mesh.vertices.clone(),
                                indices: Vec::new(),
                            },
                        )
                    });
                    let base = facet_idx * 3;
                    if base + 2 < obj.mesh.indices.len() {
                        entry.1.indices.push(obj.mesh.indices[base]);
                        entry.1.indices.push(obj.mesh.indices[base + 1]);
                        entry.1.indices.push(obj.mesh.indices[base + 2]);
                    }
                }
                // Stroke paint: append raw stroke-triangle vertices to the
                // subset's vertex pool and emit fresh indices. Strokes carry
                // their own semantic/value (overriding the layer semantic when
                // they differ, matching `extract_stroke_data` in phase3 prep).
                for stroke in &layer.strokes {
                    let key = (sem_name(&stroke.semantic), stroke.value.clone());
                    let entry = painted_subsets.entry(key).or_insert_with(|| {
                        (
                            stroke.semantic.clone(),
                            slicer_ir::IndexedTriangleSet {
                                vertices: obj.mesh.vertices.clone(),
                                indices: Vec::new(),
                            },
                        )
                    });
                    for tri in &stroke.triangles {
                        let base_idx = entry.1.vertices.len() as u32;
                        entry.1.vertices.push(tri[0]);
                        entry.1.vertices.push(tri[1]);
                        entry.1.vertices.push(tri[2]);
                        entry.1.indices.push(base_idx);
                        entry.1.indices.push(base_idx + 1);
                        entry.1.indices.push(base_idx + 2);
                    }
                }
            }
        }

        // Shell-window counts come from the BASE ResolvedConfig (configs[0]).
        // RegionMapIR pre-seeds configs[0] with `ResolvedConfig::default()`, so
        // the fallback for missing keys is OrcaSlicer's default (top=3, bottom=3).
        let (top_shell_layers, bottom_shell_layers): (usize, usize) =
            match region_map.configs.first() {
                Some(cfg) => (
                    cfg.top_shell_layers as usize,
                    cfg.bottom_shell_layers as usize,
                ),
                // TODO: when per-object/per-region paint configs are wired through
                // execute_paint_segmentation, prefer the region-specific config
                // here instead of the BASE default.
                None => (3, 3),
            };

        if !painted_subsets.is_empty() && !working.is_empty() {
            // layer_zs already computed above for modifier volumes.
            // Per-layer BASE contours come from each layer's current BASE
            // SlicedRegion (the empty-variant_chain region produced by Phase 4
            // above). Fall back to a union over all regions if BASE is missing.
            let layer_input_polygons: Vec<Vec<slicer_ir::ExPolygon>> = working
                .iter()
                .map(|s| {
                    s.regions
                        .iter()
                        .find(|r| r.variant_chain.is_empty())
                        .map(|r| r.polygons.clone())
                        .unwrap_or_else(|| {
                            s.regions
                                .iter()
                                .flat_map(|r| r.polygons.iter().cloned())
                                .collect()
                        })
                })
                .collect();

            // Run Phase 6 for each (semantic, value) and merge into working.
            for ((sname, value), (semantic, painted_mesh)) in &painted_subsets {
                if painted_mesh.indices.is_empty() {
                    continue;
                }
                let phase6 = top_bottom::propagate_top_bottom(
                    painted_mesh,
                    semantic.clone(),
                    value.clone(),
                    &layer_zs,
                    &layer_input_polygons,
                    top_shell_layers,
                    bottom_shell_layers,
                );

                let chain_key: Vec<(String, slicer_ir::PaintValue)> =
                    vec![(sname.clone(), value.clone())];

                for (l, polys) in phase6.per_layer.iter().enumerate() {
                    if polys.is_empty() || l >= working.len() {
                        continue;
                    }
                    // Phase 7 merge step 1: diff_ex BASE − phase6 (phase6 wins).
                    if let Some(base) = working[l]
                        .regions
                        .iter_mut()
                        .find(|r| r.variant_chain.is_empty())
                    {
                        base.polygons = difference_ex(&base.polygons, polys);
                    }

                    // Phase 7 merge step 2: append/union phase6 into per-color
                    // SlicedRegion. If no region has this variant_chain yet,
                    // create one cloning the BASE's object_id / region_id.
                    let existing_idx = working[l]
                        .regions
                        .iter()
                        .position(|r| r.variant_chain == chain_key);
                    match existing_idx {
                        Some(idx) => {
                            let mut combined = working[l].regions[idx].polygons.clone();
                            combined.extend(polys.iter().cloned());
                            working[l].regions[idx].polygons = union_ex(&combined);
                        }
                        None => {
                            // Use first existing region as template for ids.
                            if let Some(template) = working[l].regions.first() {
                                let object_id = template.object_id.clone();
                                let region_id = template.region_id;
                                working[l].regions.push(slicer_ir::SlicedRegion {
                                    object_id,
                                    region_id,
                                    polygons: polys.clone(),
                                    variant_chain: chain_key.clone(),
                                    segment_annotations: std::collections::HashMap::new(),
                                    ..Default::default()
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(Arc::new(working))
}

// ---------------------------------------------------------------------------
// Step 10 helpers
// ---------------------------------------------------------------------------

/// Build `segment_annotations` for a BASE-chain `SlicedRegion` from the
/// pre-sliced modifier-volume polygons for this layer.
///
/// For each modifier-volume semantic present at this layer, we emit one
/// "perimeter" (outer Vec entry) with one segment per point-pair midpoint in
/// `chain_polygons`.  Each segment gets `Some(PaintValue::Flag(true))` when
/// its midpoint falls inside at least one modifier-volume polygon.
///
/// D14 invariant: callers MUST only call this for BASE chains
/// (`chain_key.is_empty() == true`).
fn build_modifier_segment_annotations(
    layer_idx: usize,
    chain_polygons: &[slicer_ir::ExPolygon],
    modifier_vol_per_layer: &[Vec<modifier_volumes::ModifierVolumeLayer>],
) -> std::collections::HashMap<slicer_ir::PaintSemantic, Vec<Vec<Option<slicer_ir::PaintValue>>>> {
    let mut annotations: std::collections::HashMap<
        slicer_ir::PaintSemantic,
        Vec<Vec<Option<slicer_ir::PaintValue>>>,
    > = std::collections::HashMap::new();

    let Some(mv_layers) = modifier_vol_per_layer.get(layer_idx) else {
        return annotations;
    };

    if mv_layers.is_empty() || chain_polygons.is_empty() {
        return annotations;
    }

    for mv_layer in mv_layers {
        // One "perimeter" per ExPolygon in the chain.
        let mut perimeters: Vec<Vec<Option<slicer_ir::PaintValue>>> = Vec::new();

        for exp in chain_polygons {
            let pts = &exp.contour.points;
            if pts.len() < 2 {
                perimeters.push(Vec::new());
                continue;
            }
            // One segment per edge; use the midpoint for classification.
            let n = pts.len();
            let mut segs: Vec<Option<slicer_ir::PaintValue>> = Vec::with_capacity(n);
            for k in 0..n {
                let a = pts[k];
                let b = pts[(k + 1) % n];
                let mid = slicer_ir::Point2 {
                    x: (a.x + b.x) / 2,
                    y: (a.y + b.y) / 2,
                };
                let inside =
                    modifier_volumes::any_expolygon_contains_point(&mv_layer.polygons, mid);
                segs.push(if inside {
                    Some(slicer_ir::PaintValue::Flag(true))
                } else {
                    None
                });
            }
            perimeters.push(segs);
        }

        annotations
            .entry(mv_layer.semantic.clone())
            .or_default()
            .extend(perimeters);
    }

    annotations
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod driver_v2_tests {
    use super::*;
    use slicer_ir::{
        BoundingBox3, ConfigDelta, ConfigValue, ExPolygon, FacetPaintData, IndexedTriangleSet,
        ModifierScope, ModifierVolume, ObjectConfig, ObjectMesh, PaintLayer, PaintSemantic,
        PaintValue, Point2, Point3, Polygon, RegionKey, RegionMapIR, RegionPlan, SliceIR,
        SlicedRegion, Transform3d, CURRENT_MESH_IR_SCHEMA_VERSION,
        CURRENT_REGION_MAP_IR_SCHEMA_VERSION,
    };
    use std::sync::Arc;

    fn identity_transform() -> Transform3d {
        Transform3d {
            matrix: [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
        }
    }

    fn default_build_volume() -> BoundingBox3 {
        BoundingBox3 {
            min: Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            max: Point3 {
                x: 250.0,
                y: 210.0,
                z: 220.0,
            },
        }
    }

    fn empty_mesh() -> slicer_ir::MeshIR {
        slicer_ir::MeshIR {
            schema_version: CURRENT_MESH_IR_SCHEMA_VERSION,
            objects: Vec::new(),
            build_volume: default_build_volume(),
        }
    }

    fn mesh_with_no_paint() -> slicer_ir::MeshIR {
        slicer_ir::MeshIR {
            schema_version: CURRENT_MESH_IR_SCHEMA_VERSION,
            objects: vec![ObjectMesh {
                id: "obj1".to_string(),
                mesh: slicer_ir::IndexedTriangleSet {
                    vertices: vec![
                        Point3 {
                            x: 0.0,
                            y: 0.0,
                            z: 0.0,
                        },
                        Point3 {
                            x: 1.0,
                            y: 0.0,
                            z: 0.0,
                        },
                        Point3 {
                            x: 0.5,
                            y: 1.0,
                            z: 0.0,
                        },
                    ],
                    indices: vec![0, 1, 2],
                },
                transform: identity_transform(),
                config: ObjectConfig {
                    data: std::collections::HashMap::new(),
                },
                modifier_volumes: Vec::new(),
                paint_data: None,
                world_z_extent: None,
            }],
            build_volume: default_build_volume(),
        }
    }

    fn mesh_with_paint(value: PaintValue) -> slicer_ir::MeshIR {
        slicer_ir::MeshIR {
            schema_version: CURRENT_MESH_IR_SCHEMA_VERSION,
            objects: vec![ObjectMesh {
                id: "obj1".to_string(),
                mesh: slicer_ir::IndexedTriangleSet {
                    vertices: vec![
                        Point3 {
                            x: 0.0,
                            y: 0.0,
                            z: 0.0,
                        },
                        Point3 {
                            x: 1.0,
                            y: 0.0,
                            z: 0.0,
                        },
                        Point3 {
                            x: 0.5,
                            y: 1.0,
                            z: 1.0,
                        },
                    ],
                    indices: vec![0, 1, 2],
                },
                transform: identity_transform(),
                config: ObjectConfig {
                    data: std::collections::HashMap::new(),
                },
                modifier_volumes: Vec::new(),
                paint_data: Some(FacetPaintData {
                    layers: vec![PaintLayer {
                        semantic: PaintSemantic::Material,
                        facet_values: vec![Some(value)],
                        strokes: Vec::new(),
                    }],
                }),
                world_z_extent: None,
            }],
            build_volume: default_build_volume(),
        }
    }

    fn one_layer_slice_ir() -> Vec<SliceIR> {
        let u = |mm: f64| -> i64 { (mm * 10_000.0).round() as i64 };
        let region = SlicedRegion {
            object_id: "obj1".to_string(),
            region_id: 0u64,
            polygons: vec![ExPolygon {
                contour: Polygon {
                    points: vec![
                        Point2 {
                            x: u(0.0),
                            y: u(0.0),
                        },
                        Point2 {
                            x: u(1.0),
                            y: u(0.0),
                        },
                        Point2 {
                            x: u(1.0),
                            y: u(1.0),
                        },
                        Point2 {
                            x: u(0.0),
                            y: u(1.0),
                        },
                    ],
                },
                holes: Vec::new(),
            }],
            ..Default::default()
        };
        vec![SliceIR {
            schema_version: slicer_ir::CURRENT_SLICE_IR_SCHEMA_VERSION,
            global_layer_index: 0,
            z: 0.5,
            regions: vec![region],
        }]
    }

    fn empty_region_map() -> RegionMapIR {
        RegionMapIR {
            schema_version: CURRENT_REGION_MAP_IR_SCHEMA_VERSION,
            entries: std::collections::HashMap::new(),
            configs: Vec::new(),
        }
    }

    fn region_map_with_base_entry() -> RegionMapIR {
        let mut entries = std::collections::HashMap::new();
        entries.insert(
            RegionKey {
                global_layer_index: 0,
                object_id: "obj1".to_string(),
                region_id: 0u64,
                variant_chain: vec![],
            },
            RegionPlan::default(),
        );
        entries.insert(
            RegionKey {
                global_layer_index: 0,
                object_id: "obj1".to_string(),
                region_id: 0u64,
                variant_chain: vec![("material".to_string(), PaintValue::ToolIndex(1))],
            },
            RegionPlan::default(),
        );
        RegionMapIR {
            schema_version: CURRENT_REGION_MAP_IR_SCHEMA_VERSION,
            entries,
            configs: vec![slicer_ir::ResolvedConfig::default()],
        }
    }

    // ---- AC-N2 short-circuit tests ----------------------------------------

    /// AC-N2 (1): empty mesh → return input slice_ir unchanged.
    #[test]
    fn driver_v2_empty_mesh_returns_input_slice_ir() {
        let mesh = Arc::new(empty_mesh());
        let slice = Arc::new(one_layer_slice_ir());
        let rmap = Arc::new(region_map_with_base_entry());

        let result = execute_paint_segmentation(mesh, slice.clone(), rmap).unwrap();
        // Must be pointer-equal (same Arc content) or structurally equal.
        assert_eq!(
            result.len(),
            slice.len(),
            "short-circuit: length must match input"
        );
        assert_eq!(result[0].global_layer_index, 0);
        assert_eq!(result[0].regions.len(), slice[0].regions.len());
    }

    /// AC-N2 (2): mesh has objects but no PaintLayer has any Some/strokes → short-circuit.
    #[test]
    fn driver_v2_no_paint_data_short_circuits() {
        let mesh = Arc::new(mesh_with_no_paint());
        let slice = Arc::new(one_layer_slice_ir());
        let rmap = Arc::new(region_map_with_base_entry());

        let result = execute_paint_segmentation(mesh, slice.clone(), rmap).unwrap();
        assert_eq!(result.len(), slice.len());
        // Regions must be unchanged (short-circuit path, no mutation).
        assert_eq!(result[0].regions.len(), slice[0].regions.len());
        assert_eq!(result[0].regions[0].polygons, slice[0].regions[0].polygons);
    }

    /// AC-N2 (3): region_map.entries is empty → short-circuit.
    #[test]
    fn driver_v2_empty_region_map_short_circuits() {
        let mesh = Arc::new(mesh_with_paint(PaintValue::ToolIndex(1)));
        let slice = Arc::new(one_layer_slice_ir());
        let rmap = Arc::new(empty_region_map());

        let result = execute_paint_segmentation(mesh, slice.clone(), rmap).unwrap();
        assert_eq!(result.len(), slice.len());
        assert_eq!(result[0].regions[0].polygons, slice[0].regions[0].polygons);
    }

    // ---- Full-pipeline tests (require kernel; #[ignore] for AC-12 stubs) ---

    /// AC-12 (b)(d): synthetic 1-layer SliceIR + 1-object MeshIR with 1 painted facet.
    /// Expected: result has ≥ 1 SlicedRegion; painted variant_chain entry is present.
    ///
    /// TODO: setting up a valid MMU_Graph from a single painted triangle that produces
    /// non-degenerate Voronoi regions requires carefully constructed geometry.
    /// Stubbed until the Step 11 prepass wiring validates end-to-end geometry.
    #[test]
    #[ignore = "TODO(step 11): requires non-degenerate painted triangle geometry for MMU_Graph"]
    fn driver_v2_synthetic_painted_facet_emits_sliced_region() {
        let mesh = Arc::new(mesh_with_paint(PaintValue::ToolIndex(1)));
        let slice = Arc::new(one_layer_slice_ir());
        let rmap = Arc::new(region_map_with_base_entry());

        let result = execute_paint_segmentation(mesh, slice, rmap).unwrap();
        assert_eq!(result.len(), 1);
        // AC-12(d): count == |base_regions| * |variant_chains| = 1 * 2 = 2.
        assert_eq!(
            result[0].regions.len(),
            2,
            "expected 2 regions: BASE + painted variant"
        );
        // AC-12(b): painted chain polygons must be non-empty.
        let painted = result[0]
            .regions
            .iter()
            .find(|r| r.variant_chain == vec![("material".to_string(), PaintValue::ToolIndex(1))]);
        assert!(painted.is_some(), "painted variant region must be present");
        assert!(
            !painted.unwrap().polygons.is_empty(),
            "painted variant polygons must not be empty"
        );
    }

    /// AC-12 (e): two variant chains should have disjoint polygon sets.
    ///
    /// TODO: requires same geometry fix as above.
    #[test]
    #[ignore = "TODO(step 11): requires non-degenerate painted geometry for disjointness check"]
    fn driver_v2_disjoint_variant_polygons() {
        use crate::polygon_ops::intersection_ex;

        let mesh = Arc::new(mesh_with_paint(PaintValue::ToolIndex(1)));
        let slice = Arc::new(one_layer_slice_ir());
        let rmap = Arc::new(region_map_with_base_entry());

        let result = execute_paint_segmentation(mesh, slice, rmap).unwrap();
        assert!(result[0].regions.len() >= 2);

        // All pairs of regions must have disjoint polygon sets.
        let regions = &result[0].regions;
        for i in 0..regions.len() {
            for j in (i + 1)..regions.len() {
                let overlap = intersection_ex(&regions[i].polygons, &regions[j].polygons);
                assert!(
                    overlap.is_empty(),
                    "regions[{i}] and regions[{j}] have overlapping polygons"
                );
            }
        }
    }

    // ---- Step 10 / D14 invariant test ----------------------------------------

    /// D14 invariant: modifier-volume SupportEnforcer annotations go to the BASE
    /// chain's `segment_annotations` only, NOT to painted variant chains.
    ///
    /// Geometry setup is non-trivial (requires a mesh that survives the Voronoi
    /// pipeline to produce both a BASE and a painted chain). Ignored until the
    /// Step 14 cube exercise provides concrete sliceable geometry.
    ///
    /// TODO(step 14): replace ignore with a real 1mm cube + SupportEnforcer modifier
    /// that produces both a BASE chain and a ToolIndex(1) chain, then assert
    /// base.segment_annotations[SupportEnforcer].is_non_empty() and
    /// painted.segment_annotations[SupportEnforcer].is_empty().
    #[test]
    #[ignore = "TODO(step 14): cube exercise needed for non-degenerate BASE+painted chain geometry"]
    fn driver_v2_routes_modifier_volume_to_base_segment_annotations_only() {
        let u = |mm: f64| -> i64 { (mm * 10_000.0).round() as i64 };

        // Build a 1×1×1 mm SupportEnforcer cube modifier volume.
        let mv_mesh = IndexedTriangleSet {
            vertices: vec![
                Point3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                Point3 {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
                Point3 {
                    x: 1.0,
                    y: 1.0,
                    z: 0.0,
                },
                Point3 {
                    x: 0.0,
                    y: 1.0,
                    z: 0.0,
                },
                Point3 {
                    x: 0.0,
                    y: 0.0,
                    z: 1.0,
                },
                Point3 {
                    x: 1.0,
                    y: 0.0,
                    z: 1.0,
                },
                Point3 {
                    x: 1.0,
                    y: 1.0,
                    z: 1.0,
                },
                Point3 {
                    x: 0.0,
                    y: 1.0,
                    z: 1.0,
                },
            ],
            indices: vec![
                0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 2, 3, 7, 2, 7, 6, 0, 4, 7, 0,
                7, 3, 1, 2, 6, 1, 6, 5,
            ],
        };
        let mut mv_fields = std::collections::HashMap::new();
        mv_fields.insert(
            "subtype".to_string(),
            ConfigValue::String("support_enforcer".to_string()),
        );
        let mv = ModifierVolume {
            id: "mv1".to_string(),
            mesh: mv_mesh,
            config_delta: ConfigDelta { fields: mv_fields },
            priority: 0,
            applies_to: ModifierScope::AllFeatures,
        };

        // Build a mesh with the modifier volume AND a painted facet (ToolIndex(1)).
        let mesh = Arc::new(slicer_ir::MeshIR {
            schema_version: CURRENT_MESH_IR_SCHEMA_VERSION,
            objects: vec![ObjectMesh {
                id: "obj1".to_string(),
                mesh: IndexedTriangleSet {
                    vertices: vec![
                        Point3 {
                            x: 0.0,
                            y: 0.0,
                            z: 0.0,
                        },
                        Point3 {
                            x: 1.0,
                            y: 0.0,
                            z: 0.0,
                        },
                        Point3 {
                            x: 0.5,
                            y: 1.0,
                            z: 1.0,
                        },
                    ],
                    indices: vec![0, 1, 2],
                },
                transform: Transform3d {
                    matrix: [
                        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0,
                        1.0,
                    ],
                },
                config: ObjectConfig {
                    data: std::collections::HashMap::new(),
                },
                modifier_volumes: vec![mv],
                paint_data: Some(FacetPaintData {
                    layers: vec![PaintLayer {
                        semantic: PaintSemantic::Material,
                        facet_values: vec![Some(PaintValue::ToolIndex(1))],
                        strokes: Vec::new(),
                    }],
                }),
                world_z_extent: None,
            }],
            build_volume: BoundingBox3 {
                min: Point3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                max: Point3 {
                    x: 250.0,
                    y: 210.0,
                    z: 220.0,
                },
            },
        });

        // SliceIR: one layer at z=0.5 with a BASE region and a painted variant region.
        let base_region = SlicedRegion {
            object_id: "obj1".to_string(),
            region_id: 0u64,
            polygons: vec![ExPolygon {
                contour: Polygon {
                    points: vec![
                        Point2 {
                            x: u(0.0),
                            y: u(0.0),
                        },
                        Point2 {
                            x: u(1.0),
                            y: u(0.0),
                        },
                        Point2 {
                            x: u(1.0),
                            y: u(1.0),
                        },
                        Point2 {
                            x: u(0.0),
                            y: u(1.0),
                        },
                    ],
                },
                holes: Vec::new(),
            }],
            variant_chain: vec![],
            ..Default::default()
        };
        let painted_region = SlicedRegion {
            object_id: "obj1".to_string(),
            region_id: 0u64,
            polygons: vec![ExPolygon {
                contour: Polygon {
                    points: vec![
                        Point2 {
                            x: u(0.2),
                            y: u(0.2),
                        },
                        Point2 {
                            x: u(0.8),
                            y: u(0.2),
                        },
                        Point2 {
                            x: u(0.8),
                            y: u(0.8),
                        },
                        Point2 {
                            x: u(0.2),
                            y: u(0.8),
                        },
                    ],
                },
                holes: Vec::new(),
            }],
            variant_chain: vec![("material".to_string(), PaintValue::ToolIndex(1))],
            ..Default::default()
        };
        let slice = Arc::new(vec![SliceIR {
            schema_version: slicer_ir::CURRENT_SLICE_IR_SCHEMA_VERSION,
            global_layer_index: 0,
            z: 0.5,
            regions: vec![base_region, painted_region],
        }]);

        let rmap = Arc::new(region_map_with_base_entry());

        let result = execute_paint_segmentation(mesh, slice, rmap).unwrap();
        assert_eq!(result.len(), 1);

        // Find BASE and painted regions in the result.
        let base = result[0]
            .regions
            .iter()
            .find(|r| r.variant_chain.is_empty());
        let painted = result[0]
            .regions
            .iter()
            .find(|r| !r.variant_chain.is_empty());

        assert!(base.is_some(), "BASE chain region must exist");
        assert!(painted.is_some(), "painted chain region must exist");

        let base = base.unwrap();
        let painted = painted.unwrap();

        // D14 (a): BASE chain must have SupportEnforcer annotations (modifier-volume overlaps layer).
        let base_ann = base
            .segment_annotations
            .get(&PaintSemantic::SupportEnforcer);
        assert!(
            base_ann.is_some()
                && base_ann
                    .unwrap()
                    .iter()
                    .any(|p| p.iter().any(|s| s.is_some())),
            "BASE chain must have non-empty SupportEnforcer segment_annotations"
        );

        // D14 (b): painted chain must NOT have SupportEnforcer annotations.
        let painted_ann = painted
            .segment_annotations
            .get(&PaintSemantic::SupportEnforcer);
        assert!(
            painted_ann.is_none()
                || painted_ann
                    .unwrap()
                    .iter()
                    .all(|p| p.iter().all(|s| s.is_none())),
            "painted chain must NOT receive modifier-volume SupportEnforcer annotations (D14)"
        );
    }
}
