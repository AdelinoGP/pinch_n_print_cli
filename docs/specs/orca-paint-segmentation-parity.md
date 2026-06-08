# OrcaSlicer-Parity Paint Segmentation

Status: awaiting Slice Rework (blocked)
Prerequisite: Layer Slicing moved to PrePass stage, all SliceIR layers available in BlackBoard
Date: 2026-05-21
OrcaSlicer reference: `OrcaSlicerDocumented/generated_documentation/pseudocode_multimaterial_segmentation.md`
Author note: this specification replaces the current polygon-based paint annotation pipeline
(`execute_paint_segmentation` → `execute_slice_postprocess_paint_annotation` →
`point_in_paint_region` per-contour-point queries) with an OrcaSlicer-parity contour-line projection
and Voronoi-based layer segmentation pipeline. No `.rs` files are changed by this document;
implementation begins after the Slice Rework lands.

---

## 1. Summary

Replace the current polygon-projection + point-in-region paint annotation with OrcaSlicer's
Phase 3 (triangle → Z-plane intersection → EdgeGrid visitor → painted line records) and
Phase 4 (contour colorization → Voronoi graph → colored ExPolygon extraction). This fixes
all three paint-segmentation gap classes identified in the `cube_4color_paint_tdd.rs`
RED tests:

| Gap | Symptom | Root cause | Fixed by |
|-----|---------|-----------|----------|
| A — Horizontal-facet coverage | Top/bottom face paint misses contour points; unpainted facets fallback to wrong value | Polygon projection doesn't cover contour edges; no "explicitly unpainted" representation | Phase 4 contour colorization assigns color 0 (unpainted) to all unpainted gaps; Phase 6 handles horizontal faces via `slice_mesh_slabs` |
| B — Sub-facet strokes | Per-point ToolIndex variation within a triangle is lost (bands, circles) | `PaintLayer.strokes` never read | Phase 3 iterates ALL painted facets/vertices (including sub-facet detail from hex subdivision); Z-plane intersection produces per-layer line segments with correct color |
| C — Vertical-face projection | Side-face contour points get wrong (fallback) paint values | XY-only projection produces zero-area polygons for vertical faces | Phase 3 Z-plane intersection always produces a finite-length line segment, projected onto contour edges via EdgeGrid — vertical faces work identically to all others |

### What is replaced

The current pipeline path is fully replaced:

- **`execute_paint_segmentation`** (polygon projection + accumulator → `PaintRegionIR`) — replaced by Phase 3 triangle projection → `PaintedLine` records per layer
- **`execute_slice_postprocess_paint_annotation`** (per-contour-point `point_in_paint_region` queries) — replaced by Phase 4 layer segmentation → colored `ExPolygon` per extruder
- **`PaintRegionIR`** (per-layer semantic regions with polygon containment) — repurposed or replaced by per-extruder per-layer colored `ExPolygon` output
- **`PaintRegionRTreeIndex`** — no longer needed (polygon point-in-region queries eliminated)

The `SliceIR.boundary_paint` per-contour-point annotation remains the final output format,
but it is populated from the colored ExPolygon extraction rather than from per-point region queries.

---

## 2. Prerequisites and Blockers

### Hard blocker

- **Slice Rework completed** — `Layer::Slice` promoted to a PrePass stage. All `SliceIR` layers
  (with per-layer `ExPolygon` contours) are available upfront in the BlackBoard before paint
  segmentation runs. This is required because Phase 2 (EdgeGrid) and Phase 4 (Voronoi)
  operate on the full set of layer contours.

### Soft dependencies

- `cube_4color_paint_tdd.rs` RED tests (7 tests) exist on a cherry-pick branch and can be
  applied as validation gates after implementation.
- `cube_fuzzy_painted_tdd.rs` RED tests (similar gap classes for FuzzySkin) also benefit.
- `resources/cube_4color.3mf` and `resources/cube_fuzzyPainted.3mf` fixtures available.

### New crate dependencies

- **`boostvoronoi`** — Rust port of Boost.Polygon Voronoi. Used in Phase 4 for line-segment
  Voronoi diagram construction. Provides `construct_voronoi()`, infinite edge clipping,
  and `vertex.color()` metadata for graph node index tracking.

### What is removed

- `fn project_facet()` XY-only projection
- `PaintFacetEntry` and the entry accumulator in `execute_paint_segmentation`
- `group_and_union_paint_regions()`
- Per-contour-point `point_in_paint_region()` in the annotation hot path
- `PaintRegionRTreeIndex` and its construction

---

## 3. Target Architecture — 5-Phase Pipeline

### Phase 1: Slice Preprocessing

**OrcaSlicer reference**: pseudocode lines 98-109

Performed during or after the Slice Rework. For each layer:
1. Union all region slices into one `Vec<ExPolygon>`
2. Expand by `10 * SCALED_EPSILON` (10 units)
3. Union via `union_ex` to eliminate self-intersections
4. Remove polygons < 0.1 mm² (1,000,000 square units)
5. Contract back by `-10 * SCALED_EPSILON` and simplify by `5 * SCALED_EPSILON` (5 units)

This produces `input_expolygons[layer]` — the canonical layer contour used by all subsequent phases.

> **Coordinate note**: `SCALED_EPSILON` = 1 unit in our system (100 nm). In OrcaSlicer's 1nm
> system it is ~100 units. **Divide ALL OrcaSlicer integer constants by 100 when porting.**
> See `docs/08_coordinate_system.md`.

### Phase 2: EdgeGrid Construction

**OrcaSlicer reference**: pseudocode lines 111-122

Build one EdgeGrid per layer from the preprocessed `input_expolygons`.

**Algorithm**:
```
for each layer_idx (sequential):
    bbox = bounding box of input_expolygons[layer_idx]
    // Merge adjacent-layer bboxes (handles triangles straddling layer boundaries)
    if layer_idx > 0:      bbox.merge(bboxes[layer_idx - 1])
    if layer_idx < last:   bbox.merge(bboxes[layer_idx + 1])
    bbox.offset(20 units)  // 20 * SCALED_EPSILON
    edge_grids[layer_idx] = EdgeGrid::new(input_expolygons[layer_idx], cell_size = 100_000 units)
```

**EdgeGrid data structure**:
- A uniform 2D grid overlay on the bbox
- Each cell stores indices/references to the contour edges that intersect that cell
- `cell_size = 10 mm = 100_000 units` (OrcaSlicer: `scale_(10mm)`)
- Provides: `fn visit_cells_intersecting_line(line: Line, visitor: &mut impl EdgeGridVisitor)`
  — walks cells along the line and calls `visitor.visit(contour_idx, line_idx, grid_line)` for
  each contour edge in each visited cell

**Implementation note**: The grid stores `(contour_idx: usize, line_idx: usize)` pairs
identifying which contour and which segment. `contour_idx` indexes into `input_expolygons`,
`line_idx` indexes the edge within that contour. The `input_expolygons` contours are
pre-flattened into a `Vec<Vec<Line>>` for O(1) edge lookup by `(contour_idx, line_idx)`.

### Phase 3: Triangle Projection

**OrcaSlicer reference**: pseudocode lines 124-187

For each model volume, for each extruder color (1..N, skip 0=unpainted), for each
painted facet, project the 3D triangle onto layer planes.

**Outer structure** (replaces current `execute_paint_segmentation`):

```
for each ObjectMesh in mesh_ir.objects:
    let paint_data = object.paint_data (FacetPaintData)
    let world_transform = object transform matrix

    for each PaintLayer in paint_data.layers:
        let semantic = layer.semantic
        // Collect painted facets for this layer from layer.facet_values + layer.strokes
        let painted_facets = collect_facets(layer)  // unified facet+stroke collection

        for each extruder_idx in 1..num_extruders:   // skip 0 (unpainted, handled in Phase 4)
            let facets = painted_facets for this extruder_idx
            if facets is empty: continue

            // Pre-transform all vertices to world space
            let world_vertices = transform(facets.all_vertices, world_transform)
            let world_indices = facets.indices  // triangles reference world_vertices

            for each facet (3 vertex indices):
                let v0, v1, v2 = world_vertices[indices[facet_base + 0..2]]
                let min_z = min(v0.z, v1.z, v2.z)
                let max_z = max(v0.z, v1.z, v2.z)

                // Skip horizontal triangles (handled by Phase 6)
                if (max_z - min_z) < HORIZONTAL_THRESHOLD: continue

                // Sort vertices by Z ascending
                let sorted = sort_by_z([v0, v1, v2])
                let p0 = sorted[0]  // lowest Z
                let p1 = sorted[1]
                let p2 = sorted[2]  // highest Z

                // Binary search for first/last layers intersecting this triangle
                let first_layer = first layer with slice_z > (min_z - EPSILON_Z)
                let last_layer  = last  layer with slice_z < (max_z + EPSILON_Z)

                for each layer_idx in first_layer..=last_layer:
                    let slice_z = layer_zs[layer_idx]
                    if input_expolygons[layer_idx] is empty: continue
                    if slice_z < p0.z or slice_z > p2.z: continue

                    // Compute intersection line at this Z
                    let line = triangle_z_intersection(p0, p1, p2, slice_z)
                    if line is degenerate (start ≈ end): continue

                    // Project onto contour edges via EdgeGrid visitor
                    let visitor = PaintedLineVisitor::new(
                        edge_grids[layer_idx],
                        color = extruder_idx,
                        painted_lines_mutex = &mutexes[layer_idx & 0x3F]
                    )
                    edge_grids[layer_idx].visit_cells_intersecting_line(line, visitor)
                    // Visitor writes PaintedLine records into painted_lines[layer_idx]
```

**Sub-algorithm: `triangle_z_intersection(p0, p1, p2, slice_z) -> Line`**

Vertices are sorted by Z ascending: `p0.z ≤ p1.z ≤ p2.z`. The slice plane at `z = slice_z`
intersects the triangle edges. There are 4 cases:

```
// line_start is always on edge p0→p2
t = (slice_z - p0.z) / (p2.z - p0.z)
line_start = p0 + t * (p2 - p0)

if p0.z ≈ p1.z ≈ slice_z OR p1.z ≈ p2.z ≈ slice_z:
    line_end = p1  // degenerate: one edge lies in the slice plane
elif p1.z > slice_z:
    // Middle vertex above slice → intersection on p0→p1
    t1 = (slice_z - p0.z) / (p1.z - p0.z)
    line_end = p0 + t1 * (p1 - p0)
else:
    // Middle vertex below slice → intersection on p1→p2
    t2 = (slice_z - p1.z) / (p2.z - p1.z)
    line_end = p1 + t2 * (p2 - p1)

return Line(scale(line_start.xy), scale(line_end.xy))
// scale() converts mm → internal units (×10_000)
```

**Constants**:
- `HORIZONTAL_THRESHOLD`: 0.001 mm — triangles with Z extent below this are "horizontal"
  and deferred to Phase 6
- `EPSILON_Z`: 0.001 mm — tolerance for Z-range layer inclusion
- Z values are in mm (f32 from `Point3 { z: f32 }`). Use f64 for the interpolation
  arithmetic to preserve precision, converting back to f32 for the final coordinates.

### Phase 4: Layer Segmentation

**OrcaSlicer reference**: pseudocode lines 189-215

For each layer, convert raw `PaintedLine` records into colored `ExPolygon` regions per extruder.

```
for each layer_idx in parallel:
    if painted_lines[layer_idx] is empty: continue

    // 4a. Sort, merge, and filter painted line segments
    let post_processed = post_process_painted_lines(
        edge_grids[layer_idx].contours(),
        painted_lines[layer_idx]
    )

    // 4b. Assign color to every sub-segment of every contour edge
    //     Unpainted gaps get color 0 (== "use default extruder" / unpainted)
    let color_poly = colorize_contours(
        edge_grids[layer_idx].contours(),
        &post_processed
    )
    // color_poly[contour_idx] = Vec<ColoredLine>
    // Each ColoredLine has: line (the segment), color (0..N), poly_idx, local_line_idx

    // Fast path: entire layer is one color
    if has_layer_only_one_color(&color_poly):
        let single_color = color_poly[0][0].color
        segmented_regions[layer_idx][single_color] = input_expolygons[layer_idx]
    else:
        // Full Voronoi segmentation path
        let mut graph = build_graph(layer_idx, &color_poly)
        remove_multiple_edges_in_vertices(&mut graph, &color_poly)
        graph.remove_nodes_with_one_arc()
        segmented_regions[layer_idx] = extract_colored_segments(&graph, num_extruders)
```

**Output**: `segmented_regions[layer_idx][extruder_idx] -> Vec<ExPolygon>` where
`extruder_idx = 0` is the unpainted/default region and `extruder_idx = 1..N` are
painted regions per extruder.

**Mapping to our IR**: `segmented_regions` maps to `PaintRegionIR` or a new
`PaintSegmentationIR`. Each `extruder_idx` becomes a `SemanticRegion` with
`value = PaintValue::ToolIndex(extruder_idx)` for Material semantic,
`polygons = segmented_regions[layer][extruder_idx]`, `paint_order = extruder_idx`.
Index 0 (unpainted) maps to no `SemanticRegion` entry — the annotation step
treats "not in any painted region" as "unpainted."

#### 4a. `post_process_painted_lines`

**OrcaSlicer reference**: pseudocode lines 290-308

```
// Sort all PaintedLine records
sort painted_lines by (contour_idx, line_idx,
    distance of projected_line.a from contour segment start)

// Group by (contour_idx, line_idx), then filter each group
let mut filtered: Vec<Vec<PaintedLine>> = vec![vec![]; num_contours]

for each run of records sharing same (contour_idx, line_idx):
    let segment = contours[contour_idx].get_edge(line_idx)
    let merged = filter_painted_lines(segment, run)
    filtered[contour_idx].extend(merged)

return filtered
```

**`filter_painted_lines` sub-algorithm**:
- Merge same-color adjacent projected segments (within 0.1 mm = 1,000 units gap)
- Trim the first projected segment to start exactly at contour start point
- Trim the last projected segment to end exactly at contour end point
- Discard segments shorter than a minimum length (1 unit)

#### 4b. `colorize_contours`

**OrcaSlicer reference**: pseudocode lines 311-329

```
for each contour_idx in 0..num_contours:
    let contour = contours[contour_idx]
    let painted = filtered_painted[contour_idx]  // already sorted by position on contour

    let mut colorized = Vec::new()

    // Insert unpainted (color=0) segment before first painted line
    if painted is empty:
        // Entire contour unpainted
        for each edge in contour:
            colorized.push(ColoredLine { line: edge, color: 0, poly_idx, local_line_idx })
        continue

    // Fill gaps between painted segments with color 0
    let mut cursor = contour.start_distance  // distance along contour from start

    for each painted_segment in painted:
        let gap_start = cursor
        let gap_end = distance of painted_segment.projected_line.a from contour start

        if gap_end > gap_start + GAP_THRESHOLD:
            // Insert unpainted gap
            let gap_line = sub_segment(contour, gap_start, gap_end)
            colorized.push(ColoredLine { line: gap_line, color: 0, ... })

        colorized.push(ColoredLine {
            line: painted_segment.projected_line,
            color: painted_segment.color,
            ...
        })
        cursor = distance of painted_segment.projected_line.b from contour start

    // Insert unpainted segment after last painted line
    if cursor < contour.total_length:
        let tail_line = sub_segment(contour, cursor, contour.total_length)
        colorized.push(ColoredLine { line: tail_line, color: 0, ... })

    colorized_contours[contour_idx] = colorized

return colorized_contours
```

**Key detail**: `ColoredLine.color = 0` means "unpainted / use default extruder."
This is the OrcaSlicer convention (TN-8 in the pseudocode). Gaps between painted
segments always get color 0. This is what fixes GAP A (unpainted faces) — the
contour colorization explicitly assigns color 0 to unpainted areas, and Phase 4
extraction produces an explicit unpainted region from these segments.

#### 4c. `build_graph` (Voronoi)

**OrcaSlicer reference**: pseudocode lines 333-408

Construct the `MMU_Graph` for one layer from colorized polygon contours.

```
INPUT:
    color_poly[contour_idx][line_idx] = ColoredLine
    num_extruders

// Extract geometry from color_poly
let color_poly_tmp: Vec<Polygon> = polygons_from_colored_lines(color_poly)
    // Just the XY point sequences, no color info
let lines_colored: Vec<Line> = flatten all ColoredLine.line across all contours
    // Flat vector, used as Boost segment concept for Voronoi input

// Identify monochrome polygons
let mut force_edge_adding: Vec<bool> = vec![false; num_contours]
for each contour_idx:
    let all_same = all ColoredLines in color_poly[contour_idx] have same color
    force_edge_adding[contour_idx] = all_same
    // Monochrome polygons need a forced Voronoi edge so
    // extract_colored_segments can assign their single color

// Step 1: Build Voronoi diagram from line segments
let vd = boostvoronoi::Builder::with_capacity(lines_colored.len())
    .with_segments(lines_colored.iter().map(|l| {
        (l.start.x, l.start.y, l.end.x, l.end.y)
    }))
    .build()
    .expect("Voronoi construction from line segments must succeed")

// Step 2: Pre-populate graph nodes from all contour points
let mut graph = MMU_Graph::new()
for each point in color_poly_tmp:
    graph.add_node(point)
// graph.nodes[0..all_border_points] = contour vertices

// Step 3: Add BORDER arcs
graph.add_contours(&color_poly)
// One directed BORDER arc per contour edge, following polygon winding direction

// Step 4: Post-process Voronoi vertices
// Merge near-duplicate VD vertices (within SCALED_EPSILON)
// Discard out-of-bbox vertices
// Store graph node index in vertex metadata
graph.append_voronoi_vertices(&vd, &color_poly_tmp, &bbox)

// Step 5: Build double-precision segments copy (for clipping)
let segments = double_precision_copy(&color_poly_tmp)

// Step 6: Iterate VD edges, add NON_BORDER arcs
bbox.offset(10mm)  // expanded bbox for clipping infinite edges

for each edge in vd.edges():
    if edge is second half-edge (source > twin source) or already processed:
        continue

    if edge is infinite (has null vertex):
        // Clip the infinite ray against enlarged bbox
        let clipped = clip_infinite_edge(&segments, &edge, &bbox)
        if clipped is empty: continue
        let edge_line = Line(clipped[0], clipped[1])
        // Find which contour line this intersects
        if let Some(contour_line) = find_intersecting_contour_line(&edge_line, &lines_colored):
            let from_idx = edge.non_null_vertex().graph_node_index()
            let to_idx = nearest_contour_node(contour_line, &edge_line)
            if from_idx.valid() and to_idx.valid() and from_idx != to_idx:
                graph.append_edge(from_idx, to_idx, color = -1, type = NON_BORDER)
            mark_processed(edge)

    else if edge is finite:
        if both vertices on contour or vertex colors equal: skip
        let edge_line = clip_finite_edge(&edge, &bbox)
        let contour_line = lines_colored[edge.cell().source_index()]

        // Determine color for this Voronoi edge using point_inside
        let from_idx = edge.vertex0().graph_node_index()
        let to_idx = edge.vertex1().graph_node_index()
        if from_idx.valid() and to_idx.valid() and from_idx != to_idx:
            let color = determine_edge_color(&edge, &color_poly, &lines_colored, &graph)
            graph.append_edge(from_idx, to_idx, color = color, type = NON_BORDER)
            mark_processed(edge)

// Step 7: Forced edge for monochrome polygons
for each contour_idx where force_edge_adding[contour_idx]:
    // Add one Voronoi edge that starts inside this polygon
    if let Some(edge_info) = find_available_vd_edge_for_polygon(contour_idx):
        graph.append_edge(edge_info.from_idx, edge_info.to_idx,
            color = unique_color_of_polygon(contour_idx))
```

**boostvoronoi integration notes**:

The `boostvoronoi` crate mirrors Boost.Polygon's Voronoi API:
- `.with_segments()` accepts `(x1, y1, x2, y2)` tuples for line-segment sites
- After construction, iterate `.edges()` to get all Voronoi edges
- Each edge exposes `.cell()` (source segment index), `.twin()` (dual edge reference),
  `.vertex0()`, `.vertex1()` (endpoints; null for infinite edges), `.is_primary()`
- Vertices expose `.x()`, `.y()`, and `.color()` (user-defined metadata — we store
  graph node index here, matching OrcaSlicer's dual-use pattern)

**`vertex.color()` dual-use hazard (H561)**:
- Before `append_voronoi_vertices`: holds Voronoi annotation sentinel (-1 unset,
  VERTEX_ON_CONTOUR=1). We store this in boostvoronoi's vertex metadata.
- During `append_voronoi_vertices`: overwritten with graph node index.
- After: any code reading `vertex.color()` must know which phase it's in.
- **Implementation rule**: after `append_voronoi_vertices` completes, expose a
  separate `vertex_graph_node_index()` accessor that panics if not yet set.

**`MMU_Graph` data structure**:
```
struct MMU_Graph {
    nodes: Vec<Point2>,          // [0..all_border_points-1] = contour vertices
                                 // [all_border_points..] = Voronoi interior vertices
    arcs: Vec<Arc>,
    all_border_points: usize,    // split index between contour/Voronoi nodes
    polygon_idx_offset: Vec<usize>,  // node index of polygon[i][0]
    polygon_sizes: Vec<usize>,       // number of points in polygon[i]
}

struct Arc {
    from_idx: usize,
    to_idx: usize,
    color: i32,     // extruder index (0..N, -1 = no color/undetermined)
    arc_type: ArcType,
}

enum ArcType {
    Border,      // one arc per contour edge
    NonBorder,   // Voronoi-derived arc
    Deleted,     // pruned arc
}
```

#### 4d. `remove_multiple_edges_in_vertices`

**OrcaSlicer reference**: pseudocode lines 419-441

```
for each Voronoi interior vertex (node_idx >= all_border_points):
    let non_deleted_edges: Vec<(arc_idx, total_chain_length)>

    for each arc_idx in node.arcs:
        if arc.type == Deleted: continue
        let chain_len = calc_total_edge_length(arc_idx)
        // Follow nearly-straight continuations (within 15°) and sum their lengths
        non_deleted_edges.push((arc_idx, chain_len))

    if non_deleted_edges.len() <= 1: continue

    // Sort by total chain length, descending
    sort non_deleted_edges by total_chain_length descending

    // Keep the longest chain; delete all others
    while non_deleted_edges.len() > 1:
        let (arc_to_delete, _) = non_deleted_edges.pop().last()
        mark_arc_as_deleted(arc_to_delete)
        // Cascade: if far end can now be deleted, delete_vertex_deep(far_end)
```

#### 4e. `remove_nodes_with_one_arc`

**OrcaSlicer reference**: pseudocode lines 445-461

BFS pruning of dangling Voronoi interior nodes (nodes with exactly 1 arc).

```
let mut queue = VecDeque::new()
for node_idx in all_border_points..graph.nodes.len():
    if graph.arcs_for_node(node_idx).len() == 1:
        queue.push_back(node_idx)

while let Some(node_idx) = queue.pop_front():
    if graph.arc_count(node_idx) == 0: continue
    let arc = graph.only_arc(node_idx)
    let to_node = arc.to_idx
    graph.remove_edge(node_idx, to_node)  // removes from both adjacency lists
    if to_node >= all_border_points and graph.arc_count(to_node) == 1:
        queue.push_back(to_node)
```

#### 4f. `extract_colored_segments`

**OrcaSlicer reference**: pseudocode lines 465-529

Extracts per-extruder `ExPolygon` regions by walking the graph using a
leftmost-arc rule (smallest counter-clockwise turn at each junction).

```
let mut used_arcs: Vec<bool> = vec![false; graph.arcs.len()]
let mut expolygons_segments: Vec<Vec<ExPolygon>> = vec![vec![]; num_extruders]

// Walk starts only from contour border nodes
for node_idx in 0..graph.all_border_points:
    for &arc_idx in graph.arcs_for_node(node_idx):
        let arc = &graph.arcs[arc_idx]
        if arc.arc_type != Border: continue
        if used_arcs[arc_idx]: continue

        // Begin a new polygon walk
        used_arcs[arc_idx] = true
        let mut arc_id_to_face_lines: Vec<(usize, Line)> = Vec::new()
        arc_id_to_face_lines.push((arc_idx, Line(graph.nodes[node_idx],
                                                  graph.nodes[arc.to_idx])))
        let start_p = graph.nodes[node_idx]
        let mut p_vec = last line direction
        let mut p_arc = arc

        // Walk loop: follow leftmost unused arc at each junction
        loop:
            let nexts = get_next_arc(&graph, &used_arcs, p_vec, p_arc, arc.color)
            // get_next_arc: at p_arc.to_node, among eligible arcs (same color,
            // not revisiting start unless all consumed, not already used),
            // pick the one with smallest left-turn angle relative to current direction

            let mut flag = false
            for &(next_arc_idx, _) in &nexts:
                if used_arcs[next_arc_idx]:
                    flag = true; break
            if flag: break

            for &(next_arc_idx, next_line) in &nexts:
                arc_id_to_face_lines.push((next_arc_idx, next_line))
                used_arcs[next_arc_idx] = true

            p_vec = last next line direction
            p_arc = &graph.arcs[last next arc_idx]

        while p_arc.to_node != start_p || !all_arcs_at_node_used(p_arc.to_node)

        // Validate and emit polygon
        let poly = to_polygon(&arc_id_to_face_lines)
        if poly.is_counter_clockwise() and poly.is_valid():
            expolygons_segments[arc.color].push(poly)
        else:
            // Repair path: backtrack one arc at a time, add closing chord, retry
            repair_and_emit(arc_id_to_face_lines, used_arcs, expolygons_segments)
```

**`get_next_arc` sub-algorithm** (TN-7):
- Compute the reversed current direction (`-p_vec`)
- For each eligible arc from the current node:
  - Compute the signed angle between the arc's direction and `-p_vec`
  - Sort by angle ascending (smallest = leftmost / most CCW turn)
- Return the leftmost arc(s)

**Repair path** (H562):
- When a walk produces an invalid polygon (CW "hole" or self-intersecting):
  - Backtrack one arc at a time, removing it from `arc_id_to_face_lines`
  - Add a synthetic closing chord (from last line end to first line start)
  - Check if resulting polygon is valid CCW
  - Repeat until valid or arcs exhausted
- The synthetic chord uses a sentinel arc index (`usize::MAX`) — never dereferenced
  beyond the repair loop

### Phase 5: Width Limiting (optional)

**OrcaSlicer reference**: pseudocode lines 538-560

If `mmu_segmented_region_max_width > 0` or `interlocking_depth > 0`:
- For each layer, for each extruder, erode the colored region by diffing against
  `input_expolygons` offset inward by the region width
- Alternates depth between even/odd layers for mechanical interlocking

**Deferred for v1** — not needed to fix RED tests.

### Phase 6: Top/Bottom Layer Propagation

**OrcaSlicer reference**: pseudocode lines 564-646

Handles horizontal (top/bottom) face paint. These faces are SKIPPED in Phase 3
because their Z-range is negligible (`min_z ≈ max_z`).

```
if include_top_and_bottom_layers == Yes:
    let top_raw[N_extruders][N_layers]
    let bottom_raw[N_extruders][N_layers]

    for each ObjectMesh:
        for each extruder_idx in 0..num_extruders:
            let painted_facets = facets for this extruder
            if painted_facets is empty: continue

            // Slice painted mesh at all layer Zs
            let [top_proj, bottom_proj] = slice_mesh_slabs(
                painted_facets,
                layer_zs,
                world_transform
            )
            // slice_mesh_slabs produces top and bottom projections per layer Z

            merge top_proj into top_raw[extruder_idx]
            merge bottom_proj into bottom_raw[extruder_idx]

    // Filter small polygons (< 0.1 mm²)
    filter_small(top_raw); filter_small(bottom_raw)

    // Remove projections occluded by adjacent layers
    for each extruder, layer:
        top_raw[extruder][layer] = diff(top_raw[...], input_expolygons[layer + 1])
        bottom_raw[extruder][layer] = diff(bottom_raw[...], input_expolygons[layer - 1])

    // Propagate through shell layers
    for each extruder, layer:
        for shell_layer in 1..top_shell_layers:
            let src_layer = layer - shell_layer
            if src_layer valid and top_raw[extruder][src_layer] not empty:
                top_region = intersection(top_raw[extruder][src_layer],
                                          input_expolygons[layer])
                top_region = opening(top_region, small_region_threshold)
                top_result[extruder][layer] = union(top_result[...], top_region)
        // Symmetric for bottom

    return top_result, bottom_result
```

**Implementation note for v1**: `slice_mesh_slabs` is a new helper — it takes a triangle
mesh, a set of Z values, and a transform, and returns per-Z top and bottom ExPolygon
projections. This is similar to calling `slice_mesh_ex` at each Z but computes top/bottom
classification (whether the intersection is entering or leaving the mesh).

**Interleaved TBB concurrency note (H564)**: OrcaSlicer uses a two-array interleave
trick for parallel writes. In our Rust implementation with Rayon, we can use
`par_chunks_mut` with non-overlapping layer bands (e.g., chunk size =
`granularity`, stride by 1) to achieve the same effect without the interleave trick.

### Phase 7: Merge Side + Top/Bottom Results

**OrcaSlicer reference**: pseudocode lines 236-243

```
segmented_regions_merged[extruder_idx][layer] =
    union(segmented_regions[extruder_idx][layer],
          top_and_bottom_layers[extruder_idx][layer])
```

---

## 4. Data Structures — New IR Types

All types below are plausible additions to `slicer-ir` or a new `slicer-paint` crate.
Exact placement depends on the Slice Rework's resolution of the IR crate structure.

### `PaintedLine`

```
/// One projected paint line on a contour edge.
struct PaintedLine {
    contour_idx: usize,      // which contour (input_expolygon index)
    line_idx: usize,         // which edge on that contour
    projected_line: Line,    // sub-segment of the edge that is painted
    color: u32,              // extruder index (1..N)
}
```

### `ColoredLine`

```
/// One color-assigned line segment on a contour, after colorization.
/// Covers the ENTIRE contour — gaps are filled with color 0 (unpainted).
struct ColoredLine {
    line: Line,              // the contour segment
    color: u32,              // 0 = unpainted/default, 1..N = extruder
    poly_idx: usize,         // contour index
    local_line_idx: usize,   // segment index within contour
}
```

### `MMU_Graph` (see §4c definition above)

### `EdgeGrid`

```
/// Spatial acceleration grid for contour-edge queries.
struct EdgeGrid {
    /// Bounding box of the grid (in internal units, i64)
    bbox: BoundingBox2,
    /// Cell size (internal units, i64) — typically 100_000 (10 mm)
    cell_size: i64,
    /// Grid dimensions
    cols: usize,
    rows: usize,
    /// Grid cells: each cell stores (contour_idx, edge_idx) pairs
    cells: Vec<Vec<(usize, usize)>>,
    /// Contour edges, pre-flattened for O(1) access
    contours: Vec<Vec<Line>>,
}

impl EdgeGrid {
    fn new(ex_polygons: &[ExPolygon], cell_size: i64) -> Self

    fn visit_cells_intersecting_line(
        &self,
        line: Line,
        visitor: &mut dyn EdgeGridVisitor,
    )
}

trait EdgeGridVisitor {
    fn visit_cell(&mut self, contour_idx: usize, edge_idx: usize, edge_line: Line) -> bool;
    // Return true to continue traversal, false to stop
}
```

### `PaintedLineVisitor`

```
struct PaintedLineVisitor<'a> {
    edge_grid: &'a EdgeGrid,
    line_to_test: Line,
    color: u32,
    painted_lines: &'a mut Vec<PaintedLine>,
    visited_edges: HashSet<(usize, usize)>,  // dedup per layer
}

impl EdgeGridVisitor for PaintedLineVisitor<'_> {
    fn visit_cell(&mut self, contour_idx: usize, edge_idx: usize, edge_line: Line) -> bool {
        // Early-out heuristic (H563): check if ALL four endpoint pairs
        // are farther than append_threshold + line lengths
        if heuristic_early_out(self.line_to_test, edge_line):
            return true  // continue traversal

        // Collinearity test: if lines are within 30° of parallel
        if !nearly_collinear(self.line_to_test, edge_line, 30.0_deg):
            return true

        // Already visited this edge for this layer
        if self.visited_edges.contains(&(contour_idx, edge_idx)):
            return true

        // Proximity test: are endpoints within append_threshold?
        if !line_endpoints_near(self.line_to_test, edge_line):
            return true

        // Project line_to_test onto edge_line
        let projected = project_line_onto_line(edge_line, self.line_to_test)
        // Ensure same orientation as edge_line
        let oriented = orient_same_as(projected, edge_line)

        self.visited_edges.insert((contour_idx, edge_idx))
        self.painted_lines.push(PaintedLine {
            contour_idx,
            line_idx: edge_idx,
            projected_line: oriented,
            color: self.color,
        })

        true  // continue traversal
    }
}
```

---

## 5. Coordinate System and Constant Values

All internal-unit constants below assume **1 unit = 100 nm = 10⁻⁴ mm**.
OrcaSlicer values are divided by 100 (see `docs/08_coordinate_system.md`).

| Constant | OrcaSlicer value (1 nm units) | Our value (100 nm units) | Note |
|----------|------------------------------|--------------------------|------|
| `SCALED_EPSILON` | ~100 | 1 | Minimum representable offset |
| EdgeGrid `cell_size` | `scale(10 mm)` ≈ 10,000,000 | 100,000 | 10 mm cell side |
| `append_threshold` | `50 * SCALED_EPSILON` ≈ 5,000 | 50 | Max distance for line-to-contour projection |
| Collinearity angle | 30° | 30° | Max deviation from parallel |
| Gap merge threshold | `scale(0.1 mm)` ≈ 100,000 | 1,000 | Merge same-color adjacent segments closer than this |
| Color island filter | `scale(0.2 mm)` ≈ 200,000 | 2,000 | Absorb isolated short color islands |
| Phase 1 expansion | `10 * SCALED_EPSILON` ≈ 1,000 | 10 | Offset before `union_ex` |
| Phase 1 simplification | `5 * SCALED_EPSILON` ≈ 500 | 5 | Simplify after contraction |
| Small polygon filter | `scale²(0.1 mm)` ≈ 10¹⁰ | 1,000,000 square units | ~0.01 mm² |
| Phase 2 bbox offset | `20 * SCALED_EPSILON` ≈ 2,000 | 20 | Enlarge bbox for adjacent layers |
| Horizontal threshold | — | 0.001 mm | Z extent below this → horizontal face |
| Z-filter epsilon | — | 0.001 mm | Tolerance for Z-range layer inclusion |
| VD vertex merge | `SCALED_EPSILON` ≈ 100 | 1 | Near-duplicate VD vertex merge radius |
| `cos²(30°)` | ~0.75 | ~0.75 | Pre-filter for collinearity (unitless) |

**Coordinate conversions**:
- Mesh vertices arrive as `Point3 { x: f32, y: f32, z: f32 }` — world-space **millimeters**
- Transform from mesh-local to world-space: `object.transform.matrix` (4×4 f64)
- Z-plane intersection: use f64 arithmetic, convert result to `Point2 { x: i64, y: i64 }` via `Point2::from_mm(x, y)` which multiplies by 10_000
- `Line` type: `{ start: Point2, end: Point2 }` — internal units (i64)
- Distances along contour: internal units (i64)
- `boostvoronoi` coordinates: convert from internal units (i64) to f64 by casting

---

## 6. Threading Model

### Phase 3 (Triangle Projection)
Two nested Rayon `par_iter` loops:
- **Outer**: over extruder indices (typically 1-4)
- **Inner**: over facets of that extruder

This matches OrcaSlicer's nested `tbb::parallel_for` (TN-4).

**Mutex scheme** (TN-1): `painted_lines_mutex[64]` — use `Vec<Mutex<()>>` of size 64,
indexed by `layer_idx & 0x3F`. Each `PaintedLineVisitor` acquires the mutex before
pushing to `painted_lines[layer_idx]`. With 50-500 layers, this gives 1-8 layers
per bucket — low contention.

**Rust equivalent**: `Vec<Mutex<()>>` sized to 64, with `let _lock = mutexes[layer_idx & 63].lock().unwrap()`.

### Phase 4 (Layer Segmentation)
Rayon `par_iter_mut` over layers:
- Each layer's `post_process_painted_lines` + `colorize_contours` + `build_graph` +
  `extract_colored_segments` can run independently
- No inter-layer dependencies in this phase

### Phase 6 (Top/Bottom Propagation)
Rayon `par_chunks_mut` with band size = `granularity` (max top/bottom shell layers - 1).
Within each band, layers write to offset `band_index * num_layers + layer_within_band`,
matching OrcaSlicer's two-array interleave pattern without the interleave hack.

---

## 7. Integration Points

After the Slice Rework, the host pipeline entry points will change. This section
describes the logical integration regardless of exact function names.

### Inputs (from BlackBoard / PrePass outputs)

| Input | Source | Format |
|-------|--------|--------|
| `mesh_ir: Arc<MeshIR>` | Model loading | Objects with `paint_data` (FacetPaintData) and `transform` |
| `layer_plan: Arc<LayerPlanIR>` | Layer planning | Global layer Z values and object participation |
| `input_expolygons: Vec<Vec<ExPolygon>>` | Slice Rework output | One `Vec<ExPolygon>` per layer (processed per Phase 1) |

### Output (to annotation / downstream)

| Output | Destination | Format |
|--------|-------------|--------|
| `segmented_regions: Vec<Vec<Vec<ExPolygon>>>` | Paint annotation | `[extruder_idx][layer_idx] -> Vec<ExPolygon>` |
| Or equivalently: `PaintRegionIR` with per-layer per-extruder regions | SlicePostProcess | `SemanticRegion { value: ToolIndex(extruder_idx), polygons }` |

### Annotation from segmented regions

Replace `execute_slice_postprocess_paint_annotation` with a simplified function
that assigns paint values from the segmented ExPolygon output:

```
fn annotate_from_segmented_regions(
    slice_ir: &mut SliceIR,
    segmented_regions: &[Vec<ExPolygon>],  // for this layer
) {
    for region in &mut slice_ir.regions:
        for (poly_idx, polygon) in region.polygons.iter().enumerate():
            for (pt_idx, point) in polygon.contour.points.iter().enumerate():
                for extruder_idx in 0..num_extruders:
                    if point_in_any_ex_polygon(point, &segmented_regions[extruder_idx]):
                        if extruder_idx == 0:
                            // Unpainted — leave as None
                        else:
                            boundary_paint[Material][poly_idx][pt_idx] =
                                Some(PaintValue::ToolIndex(extruder_idx))
                        break
}
```

This replaces the per-contour-point `point_in_paint_region()` call (which does a
polygon-containment check) with a simpler `point_in_any_ex_polygon()` check on the
pre-computed colored regions. For the common case (single extruder per region),
this is O(N_polygons) not O(N_points × N_regions).

---

## 8. Known Hazards (from OrcaSlicer Pseudocode)

All hazards below are from `OrcaSlicerDocumented/generated_documentation/pseudocode_multimaterial_segmentation.md`. Each should be understood before implementing the corresponding component; some require specific mitigations.

### H561 — `vertex.color()` dual-use
`vertex.color()` holds `VD_ANNOTATION` sentinel before `append_voronoi_vertices()` and graph node index after. **Mitigation**: use a wrapper type `VoronoiVertex { annotation: ... }` during Voronoi construction that transitions to `GraphVertex { node_index: ... }` after `append_voronoi_vertices()`. The boostvoronoi `vertex.color()` field is repurposed the same way — document this in code comments.

### H562 — Repair path sentinel
`arc_id_to_face_lines` stores `(size_t)(-1)` as sentinel arc index for synthetic closing chords. **Mitigation**: use `Option<usize>` instead of a sentinel value. Store `None` for synthetic chords, `Some(arc_idx)` for real arcs. This eliminates the out-of-bounds risk.

### H563 — Heuristic pre-filter
The 4-endpoint-pair early-out uses AND logic — skips only if ALL 4 pairs exceed threshold. Conservative: may pass lines to the expensive collinearity check that are ultimately rejected. **Acceptance**: this is a performance optimization, not a correctness issue. Port as-is.

### H564 — Two-array interleave
Phase 6 writes to `triangles_by_color[*][*]` using `(group_idx & 1) * num_layers` offset to avoid TBB write conflicts. **Mitigation**: use Rayon's `par_chunks_mut` with non-overlapping layer bands (see §6).

### H565 — Hardcoded extruder 0 nozzle
`layer_color_stat()` always reads `nozzle_diameter.get_at(0)`. **Mitigation**: read from the extruder's own config. Track as a separate issue — this is a behavioral correctness bug in OrcaSlicer that we should not replicate.

### H566 — O(degree) deduplication
`append_edge()` deduplication is quadratic in vertex degree. Empirically small (< 10) but not asserted. **Mitigation**: use a `HashSet` for the per-vertex arc presence check, or add a `debug_assert!(degree <= 20)`.

### H567 — Force-edge pointer arithmetic
`force_edge_adding[]` indexed by pointer arithmetic into `color_poly`. Empty polygon entries produce duplicate global indices. **Mitigation**: use explicit index tracking, not pointer arithmetic. Validate `poly_idx` bounds.

---

## 9. Test Strategy

### Unit tests (component-level)

| Component | What to test | Input | Expected output |
|-----------|-------------|-------|-----------------|
| `triangle_z_intersection` | Various triangle orientations vs slice Z | 3 points + Z | Correct line segment or degenerate |
| `EdgeGrid::new` | Construction from known contour | Square ExPolygon | Grid with expected cell contents |
| `EdgeGrid::visit_cells_intersecting_line` | Line through grid | Known line + grid | Correct cells visited, visitor fires |
| `PaintedLineVisitor` | Line-to-contour projection | Grid + line + color | PaintedLine records pushed |
| `post_process_painted_lines` | Sort, merge, filter | Raw PaintedLine records | Merged, sorted, trimmed records |
| `colorize_contours` | Assign color to whole contour | Painted segments + contour | Full ColoredLine coverage (no gaps) |
| `build_graph` | Voronoi graph construction | ColoredLine polygons | Valid MMU_Graph with Border + NonBorder arcs |
| `extract_colored_segments` | Graph walk → ExPolygons | MMU_Graph | Valid colored ExPolygons |

### Integration tests

| Test | What it validates | Fixture |
|------|------------------|---------|
| Full pipeline, simple cube | Phase 1-7 for single-color cube | `benchy.stl` (no paint) |
| Cube with 4 colors (all faces) | GAP A, B, C — all 7 RED tests pass | `cube_4color.3mf` |
| Cube with fuzzy skin | GAP A, C for FuzzySkin semantic | `cube_fuzzyPainted.3mf` |
| Benchy with MMU paint | Engineered cube, deterministic per-face Material paint, 37 KB | `cube_4color.3mf` |

### Regression tests

Cherry-pick `crates/slicer-host/tests/cube_4color_paint_tdd.rs` from the RED-test branch
after implementation. Remove `#[ignore]` markers. All 12 tests (5 original GREEN + 7 RED)
must pass.

### Golden tests (future)

Compare `segmented_regions` output against OrcaSlicer reference for a known model.
Extract OrcaSlicer output for `cube_4color.3mf` and store as golden file.
Byte-for-byte comparison may be impractical due to Voronoi coordinate differences;
verify topological equivalence instead (same number of regions per layer per extruder,
same connectivity).

---

## 10. Implementation Order

Each step can be a separate PR or packet. Steps marked with (*) do NOT depend on the
Slice Rework and can begin immediately (though `EdgeGrid` and `PaintedLineVisitor`
are not useful until integration).

1. **`triangle_z_intersection` function** (*) — pure math, unit-testable with hand-crafted triangles
2. **EdgeGrid data structure** (*) — independent of Slice Rework; accepts `Vec<ExPolygon>`; unit tests with known contours
3. **PaintedLineVisitor** (*) — depends on EdgeGrid (#2); unit tests with known grid + line
4. **Phase 3 integration** — depends on Slice Rework (needs `input_expolygons`); wires triangle→line→visitor loop; outputs `painted_lines[layer]`
5. **`post_process_painted_lines` + `colorize_contours`** — depends on #4 (needs PaintedLine output); unit tests with hand-crafted input
6. **boostvoronoi integration + `build_graph`** — depends on #5 (needs ColoredLine output); unit tests with simple contours
7. **`extract_colored_segments`** — depends on #6; unit tests with simple graphs
8. **Phase 6: top/bottom layer propagation** — depends on #7; needs `slice_mesh_slabs` implementation
9. **Integration wiring** — replace `execute_paint_segmentation` + `execute_slice_postprocess_paint_annotation`; hook into host pipeline
10. **Cherry-pick RED tests** — apply `cube_4color_paint_tdd.rs` and `cube_fuzzy_painted_tdd.rs` RED tests; verify all pass
11. **Golden test (optional)** — compare against OrcaSlicer reference

---

## 11. Open Questions

1. **Does `slice_mesh_slabs` already exist?** If not, Phase 6 needs a new implementation. This function takes a painted triangle mesh and a set of Z values and returns per-layer top/bottom projection ExPolygons. It's similar to calling `slice_mesh_ex` at each Z but also classifies whether intersections are "top" (entering mesh) or "bottom" (leaving mesh).

2. **Where does `PaintRegionIR` go?** The current `PaintRegionIR` (per-layer `SemanticRegion` with polygon containment) may be repurposed or replaced. The new output `segmented_regions[layer][extruder] → Vec<ExPolygon>` could be stored in a new `PaintSegmentationIR` or mapped into the existing `PaintRegionIR` by converting each extruder index to a `SemanticRegion`. Decision depends on how downstream consumers (support modules, gcode emitters) read paint data.

3. **What about `PaintSemantic::FuzzySkin`?** Phase 3-4 is generic over semantic type (Material, FuzzySkin, SupportEnforcer/Blocker). The only difference for FuzzySkin is `num_facets_states = 2` (0=unpainted, 1=fuzzy). The same EdgeGrid + Voronoi pipeline works for all semantics.

4. **How does the model loader populate stroke triangles?** For Phase 3 to process sub-facet detail (GAP B), the stroke triangles from `PaintLayer.strokes` must be accessible alongside the mesh facet data. The loader already transforms strokes to world space (`apply_transform_to_paint_data`). They can be iterated in the same facet loop with the same `triangle_z_intersection` logic.

5. **Is `boostvoronoi` production-ready?** The crate is a direct port of Boost.Polygon's Voronoi. Verify: (a) it supports line-segment sites (not just points), (b) it exposes `vertex.color()` for metadata, (c) it handles infinite edges with `is_primary()` / `twin()` semantics, (d) it compiles on our target platforms.

---

## 12. Doc Impact

- `docs/02_ir_schemas.md` — add `PaintedLine`, `ColoredLine`, `MMU_Graph`, `EdgeGrid` to IR type catalog
- `docs/04_host_scheduler.md` — update PrePass/PaintSegmentation execution contract to describe new Phase 3-7 pipeline
- `docs/08_coordinate_system.md` — add constant conversion table (§5 above)
- `docs/07_implementation_status.md` — mark paint-segmentation parity as in-progress
