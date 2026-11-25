#!/usr/bin/env python3
"""Support-path and support-layer census for the t148 print-plane carve.

Reads one or more G-code files and reports, per file:

- total support extrusion path (``Support`` + ``Support interface``),
- the number of layers that carry any support extrusion,
- the sorted list of those layer Z heights (rounded to 4 dp).

The parser is the t147 one, deliberately: t147 measured the missing support with
it, so the recovery is quoted on the same metric. It is also applied unchanged to
the Orca arm where a comparison is wanted.

Usage:
    python measure_support.py FILE [FILE ...] [--json OUT]

Stdlib only.
"""

from __future__ import annotations

import argparse
import json
import math
import re
import sys
from pathlib import Path


def parse_gcode(path: Path) -> dict[float, dict[str, float]]:
    """Per-layer extrusion path length, split by Support / Support interface."""
    layers: dict[float, dict[str, float]] = {}
    z = None
    feature = None
    x = y = None
    with path.open("r", encoding="utf-8", errors="ignore") as handle:
        for line in handle:
            text = line.strip()
            if text.startswith(";"):
                match = re.match(r"; ?Z(?:_HEIGHT)?:\s*([0-9.]+)", text)
                if match:
                    z = round(float(match.group(1)), 4)
                    x = y = None
                    continue
                match = re.match(r"; ?FEATURE:\s*(.+)", text) or re.match(
                    r";TYPE:\s*(.+)", text
                )
                if match:
                    feature = match.group(1).strip()
                continue
            if not (text.startswith("G1 ") or text.startswith("G0 ")):
                continue
            mx = re.search(r"X([-0-9.]+)", text)
            my = re.search(r"Y([-0-9.]+)", text)
            nx = float(mx.group(1)) if mx else x
            ny = float(my.group(1)) if my else y
            me = re.search(r"\bE(-?[0-9.]+)", text)
            extruding = bool(me) and float(me.group(1)) > 0
            if (
                feature
                and feature.startswith("Support")
                and extruding
                and x is not None
                and y is not None
            ):
                bucket = layers.setdefault(
                    z, {"Support": 0.0, "Support interface": 0.0}
                )
                bucket[feature if feature in bucket else "Support"] += math.hypot(
                    nx - x, ny - y
                )
            x, y = nx, ny
    return layers


def census(path: Path) -> dict:
    layers = parse_gcode(path)
    total = 0.0
    body = 0.0
    interface = 0.0
    support_layers: list[float] = []
    for z, bucket in layers.items():
        value = bucket["Support"] + bucket["Support interface"]
        if value <= 0.0:
            continue
        support_layers.append(z)
        body += bucket["Support"]
        interface += bucket["Support interface"]
        total += value
    support_layers.sort()
    return {
        "file": path.name,
        "support_path_mm": round(total, 3),
        "support_body_mm": round(body, 3),
        "support_interface_mm": round(interface, 3),
        "support_layers": len(support_layers),
        "support_layer_z": support_layers,
    }


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("files", nargs="+", type=Path)
    parser.add_argument("--json", type=Path, default=None)
    args = parser.parse_args(argv)

    results = [census(path) for path in args.files]
    for row in results:
        print(
            f"{row['file']:44s} path={row['support_path_mm']:12.3f} mm "
            f"layers={row['support_layers']:4d}"
        )
    if args.json is not None:
        args.json.parent.mkdir(parents=True, exist_ok=True)
        args.json.write_text(json.dumps(results, indent=2) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
