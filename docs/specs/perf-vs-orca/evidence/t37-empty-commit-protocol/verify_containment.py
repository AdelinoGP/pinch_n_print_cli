"""Ticket-37 containment verification: every printed sparse segment must lie
inside its layer's own part cross-section envelope.

This is the ticket-35 style check, run against a candidate G-code and its
per-layer sparse census. It does NOT need the raw STL: it uses the printed
wall bbox of each layer as the part envelope, which is what the ticket-35
finding used (a revived envelope reaching 16.8 mm past the wall bbox).

Usage:
  python verify_containment.py <candidate.gcode> [<baseline.gcode>]

Prints, per file: total sparse segments, total sparse mm, and the worst
out-of-envelope overshoot per layer (max |sparse point - wall bbox| in mm).
"""

import sys
from math import hypot
from pathlib import Path


def census(path):
    layers = {}
    layer = -1
    role = None
    x = y = None
    with Path(path).open("rb") as source:
        for line in source:
            if line.startswith((b";LAYER_CHANGE", b"; CHANGE_LAYER")):
                layer += 1
                role = None
                layers.setdefault(
                    layer,
                    {
                        "sparse_segments": 0,
                        "sparse_mm": 0.0,
                        "wall_min_x": None,
                        "wall_max_x": None,
                        "wall_min_y": None,
                        "wall_max_y": None,
                        "sparse_pts": [],
                    },
                )
            if layer < 0:
                continue
            if line.startswith(b";TYPE:"):
                role = line.removeprefix(b";TYPE:").strip()
            fields = line.split(b";", 1)[0].split()
            if not fields:
                continue
            if fields[0] in (b"G91", b"M82"):
                raise ValueError(f"unsupported mode in {path}")
            if fields[0] == b"G92":
                for field in fields[1:]:
                    if field.startswith(b"X"):
                        x = float(field[1:])
                    elif field.startswith(b"Y"):
                        y = float(field[1:])
            if fields[0] not in (b"G0", b"G1"):
                continue
            values = {}
            for field in fields[1:]:
                if field.startswith((b"X", b"Y", b"E")):
                    values[field[:1]] = float(field[1:])
            new_x = values.get(b"X", x)
            new_y = values.get(b"Y", y)
            if x is not None and y is not None:
                if role in (b"Outer wall", b"Inner wall"):
                    for vx, vy in ((x, y), (new_x, new_y)):
                        if vx is None or vy is None:
                            continue
                        d = layers[layer]
                        d["wall_min_x"] = (
                            vx if d["wall_min_x"] is None else min(d["wall_min_x"], vx)
                        )
                        d["wall_max_x"] = (
                            vx if d["wall_max_x"] is None else max(d["wall_max_x"], vx)
                        )
                        d["wall_min_y"] = (
                            vy if d["wall_min_y"] is None else min(d["wall_min_y"], vy)
                        )
                        d["wall_max_y"] = (
                            vy if d["wall_max_y"] is None else max(d["wall_max_y"], vy)
                        )
                if (
                    fields[0] == b"G1"
                    and role == b"Sparse infill"
                    and values.get(b"E", 0) > 0
                    and new_x is not None
                    and new_y is not None
                ):
                    length = hypot(new_x - x, new_y - y)
                    if length > 0:
                        layers[layer]["sparse_segments"] += 1
                        layers[layer]["sparse_mm"] += length
                        layers[layer]["sparse_pts"].append((new_x, new_y))
            x, y = new_x, new_y
    return layers


def report(path):
    layers = census(path)
    total_segments = sum(d["sparse_segments"] for d in layers.values())
    total_mm = sum(d["sparse_mm"] for d in layers.values())
    worst = None
    for index, d in layers.items():
        if not d["sparse_pts"] or d["wall_min_x"] is None:
            continue
        overshoot = 0.0
        for px, py in d["sparse_pts"]:
            dx = max(d["wall_min_x"] - px, px - d["wall_max_x"], 0.0)
            dy = max(d["wall_min_y"] - py, py - d["wall_max_y"], 0.0)
            overshoot = max(overshoot, hypot(dx, dy))
        if worst is None or overshoot > worst[1]:
            worst = (index, overshoot)
    print(
        f"{Path(path).name}: layers={len(layers)} sparse_segments={total_segments} "
        f"sparse_mm={total_mm:.1f} worst_out_of_wall_bbox_mm="
        f"{'n/a' if worst is None else f'{worst[1]:.2f} (layer {worst[0]})'}"
    )
    return total_segments, total_mm, worst


if __name__ == "__main__":
    if len(sys.argv) < 2:
        raise SystemExit(__doc__)
    for name in sys.argv[1:]:
        report(name)
