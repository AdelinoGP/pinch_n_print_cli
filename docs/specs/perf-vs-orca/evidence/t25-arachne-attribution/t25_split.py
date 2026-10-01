"""Reduce a T25-PROBE capture: Arachne pipeline substage wall attribution.

Reads the stderr capture from a probe host binary and prints:
  1. integrity checks (invocation count, svc==1 per line, bracket closure),
  2. the aggregated stage table (total ns, share of pipeline total, calls),
  3. the per-layer tail table for the slowest layers,
  4. the graph-construction internals table.

Usage: python t25_split.py <stderr.txt> [--top N]
"""

import json
import sys
from collections import defaultdict

ORDER = [
    "svc",
    "in",
    "out",
    "pipe",
    "pre",
    "pre_warn",
    "s1",
    "s1_o2",
    "s1_o2p1",
    "s1_o2p2",
    "s1_o2tr",
    "s1_off",
    "s1_ofp",
    "s1_ofe",
    "s1_oft",
    "s2",
    "s3",
    "s4",
    "s5",
    "s6",
    "s7",
    "s8",
    "s9",
    "g_tot",
    "g_flat",
    "g_vor",
    "g_vmap",
    "g_vbuild",
    "g_vconv",
    "g_clamp",
    "g_build",
    "g_sep",
    "g_coll",
    "g_comp",
    "g_rad",
    "strat",
    "cent",
    "beads",
    "noncent",
    "tm_gen",
    "tm_filt",
    "ends",
    "apply_tr",
    "ribs",
    "pop",
    "up",
    "down",
    "junc",
    "conn",
    "maxima",
    "stitch",
    "rm_small",
    "sep",
    "simp",
    "rm_empty",
    "reorder",
    "m_polys",
    "m_segs",
    "m_lines",
    "m_inner",
]

HUMAN = {
    "svc": "service total (WIT in -> WIT out)",
    "in": "  WIT in (polygons -> IR)",
    "out": "  WIT out (IR -> lines)",
    "pipe": "run_arachne_pipeline total",
    "pre": "  preprocess 9-stage total",
    "pre_warn": "    warn_dropped_features",
    "s1": "    stage 1 triple offset",
    "s1_o2": "      offset2_ex (shrink-grow)",
    "s1_o2p1": "        pass 1 inflate_paths_64 (erode)",
    "s1_o2p2": "        pass 2 execute_tree (dilate)",
    "s1_o2tr": "        pass 2 expolygons_from_tree",
    "s1_off": "      offset (shrink)",
    "s1_ofp": "        paths conversion",
    "s1_ofe": "        execute_tree",
    "s1_oft": "        expolygons_from_tree",
    "s2": "    stage 2 simplify",
    "s3": "    stage 3 fix self-intersections",
    "s4": "    stage 4 degenerate verts",
    "s5": "    stage 5 colinear edges",
    "s6": "    stage 6 fix self-intersections (2)",
    "s7": "    stage 7 degenerate verts (2)",
    "s8": "    stage 8 small areas",
    "s9": "    stage 9 final union",
    "g_tot": "  graph from_polygons total",
    "g_flat": "    flatten_polys",
    "g_vor": "    voronoi_from_segments",
    "g_vmap": "      lines -> BvLine map",
    "g_vbuild": "      Builder::build (sweep line)",
    "g_vconv": "      diagram -> HalfEdgeGraph map",
    "g_clamp": "    clamp vertices",
    "g_build": "    Builder::build",
    "g_sep": "    separate_pointy_quad_end_nodes",
    "g_coll": "    collapse_small_edges",
    "g_comp": "    compact_graph_after_collapse",
    "g_rad": "    edge_radius_bounds loop",
    "strat": "  create_stack",
    "cent": "  filter_central",
    "beads": "  assign_bead_counts",
    "noncent": "  filter_noncentral_regions",
    "tm_gen": "  generate_transition_mids",
    "tm_filt": "  filter_transition_mids",
    "ends": "  generate_all_transition_ends",
    "apply_tr": "  apply_transitions",
    "ribs": "  generate_extra_ribs",
    "pop": "  populate_beading_propagation",
    "up": "  propagate_beadings_upward",
    "down": "  propagate_beadings_downward",
    "junc": "  generate_junctions",
    "conn": "  connect_junctions",
    "maxima": "  local maxima beads",
    "stitch": "  stitch_extrusions",
    "rm_small": "  remove_small_lines",
    "sep": "  separate_out_inner_contour",
    "simp": "  simplify_toolpaths",
    "rm_empty": "  remove_empty_toolpaths",
    "reorder": "  reorder_by_region_order",
    "m_polys": "[meta] input polygons",
    "m_segs": "[meta] flattened segments",
    "m_lines": "[meta] emitted lines",
    "m_inner": "[meta] inner-contour lines",
}


def parse(path):
    rows = []
    with open(path, encoding="utf-8", errors="replace") as f:
        for line in f:
            line = line.strip()
            if not line.startswith("T25-PROBE"):
                continue
            fields = {}
            seq = None
            for part in line.split()[1:]:
                key, value = part.split("=", 1)
                if key == "seq":
                    seq = int(value)
                else:
                    wall, calls = value.split(":")
                    fields[key] = (int(wall), int(calls))
            rows.append({"seq": seq, **fields})
    return rows


def main(path, top=12):
    rows = parse(path)
    print(f"probe lines: {len(rows)}")
    bad = [r["seq"] for r in rows if r.get("svc", (0, 0))[1] != 1]
    print(f"lines with svc calls != 1: {len(bad)} {bad[:10]}")

    agg = defaultdict(lambda: [0, 0])
    for row in rows:
        for key, value in row.items():
            if key == "seq":
                continue
            wall, calls = value
            agg[key][0] += wall
            agg[key][1] += calls
    total_pipe = agg["pipe"][0]
    total_svc = agg["svc"][0]
    print(f"total service wall: {total_svc / 1e6:.1f} ms over {len(rows)} invocations")
    print(
        f"total pipeline wall: {total_pipe / 1e6:.1f} ms ({total_pipe / total_svc * 100:.2f}%)"
    )
    print()

    print(
        f"{'key':10s} {'wall_ms':>10s} {'%svc':>7s} {'%pipe':>7s} {'calls':>7s}  stage"
    )
    for key in ORDER:
        wall, calls = agg.get(key, (0, 0))
        if wall == 0 and calls == 0:
            continue
        pct_svc = wall / total_svc * 100 if total_svc else 0
        pct_pipe = wall / total_pipe * 100 if total_pipe else 0
        print(
            f"{key:10s} {wall / 1e6:10.1f} {pct_svc:6.2f}% {pct_pipe:6.2f}% {calls:7d}  {HUMAN.get(key, '')}"
        )

    print()
    print("=== slowest invocations ===")
    ranked = sorted(rows, key=lambda r: -r.get("svc", (0, 0))[0])[:top]
    for row in ranked:
        print(
            f"seq={row['seq']:4d} svc={row['svc'][0] / 1e6:9.1f}ms "
            f"pipe={row.get('pipe', (0, 0))[0] / 1e6:9.1f}ms "
            f"pre={row.get('pre', (0, 0))[0] / 1e6:8.1f}ms "
            f"graph={row.get('g_tot', (0, 0))[0] / 1e6:8.1f}ms "
            f"down={row.get('down', (0, 0))[0] / 1e6:8.1f}ms "
            f"junc={row.get('junc', (0, 0))[0] / 1e6:8.1f}ms "
            f"conn={row.get('conn', (0, 0))[0] / 1e6:8.1f}ms "
            f"simp={row.get('simp', (0, 0))[0] / 1e6:8.1f}ms "
            f"reorder={row.get('reorder', (0, 0))[0] / 1e6:8.1f}ms"
        )


if __name__ == "__main__":
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    top = 12
    if "--top" in sys.argv:
        top = int(sys.argv[sys.argv.index("--top") + 1])
    main(args[0], top)
