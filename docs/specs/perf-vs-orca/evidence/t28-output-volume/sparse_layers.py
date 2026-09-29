"""Per-layer sparse-infill motion census (absolute XY, relative E).

Usage: python docs/specs/perf-vs-orca/evidence/t28-output-volume/sparse_layers.py \
  target/matched-pair/t28-output-volume/benchy-classic.gcode \
  target/matched-pair/t28-output-volume/benchy-arachne.gcode
Only G1 moves with positive E and a changed XY position count as printed
segments. G0/G1 positions outside sparse sections still update the XY cursor.
Fails on an unsupported coordinate/extrusion mode instead of guessing.
"""

import sys
from collections import defaultdict
from math import hypot
from pathlib import Path


def layers(path):
    result = defaultdict(lambda: [0, 0.0, 0])  # count, printed XY mm, G-code bytes
    layer = -1
    role = None
    x = y = None
    with Path(path).open("rb") as source:
        for line in source:
            if line.startswith((b";LAYER_CHANGE", b"; CHANGE_LAYER")):
                layer += 1
                role = None
            elif line.startswith(b";TYPE:"):
                role = line.removeprefix(b";TYPE:").strip()
            elif line.startswith(b"; FEATURE: "):
                role = line.removeprefix(b"; FEATURE: ").strip()
            if layer < 0:
                # Orca's machine start G-code contains relative XY moves before
                # the first layer; no sparse path depends on that cursor.
                continue
            if role == b"Sparse infill":
                result[layer][2] += len(line)
            fields = line.split(b";", 1)[0].split()
            if not fields:
                continue
            if fields[0] in (b"G91", b"M82"):
                raise ValueError(f"unsupported relative XY or absolute E in {path}")
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
            if (
                fields[0] == b"G1"
                and role == b"Sparse infill"
                and values.get(b"E", 0) > 0
                and x is not None
                and y is not None
                and new_x is not None
                and new_y is not None
            ):
                length = hypot(new_x - x, new_y - y)
                if length > 0:
                    result[layer][0] += 1
                    result[layer][1] += length
            x, y = new_x, new_y
    return result


if __name__ == "__main__":
    if len(sys.argv) != 3:
        raise SystemExit(__doc__)
    left, right = (layers(name) for name in sys.argv[1:])
    print(
        "layer,classic_segments,arachne_segments,classic_xy_mm,arachne_xy_mm,classic_bytes,arachne_bytes"
    )
    for index in sorted(left.keys() | right.keys()):
        a = left[index]
        b = right[index]
        print(f"{index},{a[0]},{b[0]},{a[1]:.3f},{b[1]:.3f},{a[2]},{b[2]}")
