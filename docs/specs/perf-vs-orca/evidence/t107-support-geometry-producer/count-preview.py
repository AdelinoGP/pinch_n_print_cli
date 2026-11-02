#!/usr/bin/env python3
"""Count the SupportGeometryIR entries a `support-preview` JSON renders.

The preview's `layers[].support` array is built from the **non-sentinel**
`SupportGeometryIR.entries` (see `build_preview_doc` in
`crates/pnp-cli/src/support_preview.rs`); sentinel (`u32::MAX`) entries are
counted in `skipped_intermediate_entries` and not rendered. So these counts
measure the IR's polygon/point volume without needing an instrumented build.

Usage:
  python count-preview.py <preview.json> [--expect preview-counts.json]
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent


def count(path: Path) -> dict:
    doc = json.loads(path.read_text(encoding="utf-8"))
    layers = doc["layers"]
    polys = 0
    points = 0
    for layer in layers:
        for poly in layer["support"]:
            polys += 1
            points += len(poly["contour"])
            for hole in poly.get("holes") or []:
                points += len(hole)
    return {
        "layer_count": doc["layer_count"],
        "layers_with_geometry": len(layers),
        "geometry_support_polygons": polys,
        "geometry_support_points": points,
        "skipped_intermediate_entries": doc["skipped_intermediate_entries"],
    }


def main(argv: list[str]) -> int:
    if not argv:
        print(__doc__)
        return 2
    got = count(Path(argv[0]))
    print(json.dumps(got, indent=1))
    if "--expect" in argv:
        expect_path = Path(argv[argv.index("--expect") + 1])
        want = json.loads(expect_path.read_text(encoding="utf-8"))
        bad = {k: (got[k], want.get(k)) for k in got if got[k] != want.get(k)}
        if bad:
            print(f"counts differ from {expect_path}: {bad}")
            return 1
        print(f"counts match {expect_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
