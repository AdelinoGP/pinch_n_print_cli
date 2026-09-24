#!/usr/bin/env python
"""Summarize the output-diff corpus volume for the t17 evidence."""

import glob
import json
import os
import sys

HOME = sys.argv[1] if len(sys.argv) > 1 else "out-110"


def main():
    groups = {}
    for f in sorted(glob.glob(os.path.join(HOME, "*.json"))):
        base = os.path.basename(f)[:-5]
        if base.startswith("_"):
            continue
        group = base.split("__")[0]
        with open(f) as fh:
            data = json.load(fh)
        g = groups.setdefault(
            group,
            {
                "files": 0,
                "polys": 0,
                "holes": 0,
                "points": 0,
                "polylines": 0,
                "polyline_pts": 0,
                "empty": 0,
            },
        )
        g["files"] += 1
        if data and isinstance(data[0], dict) and "contour" in data[0]:
            g["polys"] += len(data)
            for e in data:
                g["holes"] += len(e["holes"])
                g["points"] += len(e["contour"]["points"])
                g["points"] += sum(len(h["points"]) for h in e["holes"])
        else:
            g["polylines"] += len(data)
            g["polyline_pts"] += sum(len(p) for p in data)
        if not data:
            g["empty"] += 1
    tot = {
        "files": 0,
        "polys": 0,
        "holes": 0,
        "points": 0,
        "polylines": 0,
        "polyline_pts": 0,
        "empty": 0,
    }
    print(
        f"{'group':16s} {'files':>6s} {'polys':>7s} {'holes':>7s} {'pts':>8s} "
        f"{'polylines':>10s} {'pline pts':>10s} {'empty':>6s}"
    )
    for k in sorted(groups):
        g = groups[k]
        print(
            f"{k:16s} {g['files']:6d} {g['polys']:7d} {g['holes']:7d} {g['points']:8d} "
            f"{g['polylines']:10d} {g['polyline_pts']:10d} {g['empty']:6d}"
        )
        for t in tot:
            tot[t] += g[t]
    print(
        f"{'TOTAL':16s} {tot['files']:6d} {tot['polys']:7d} {tot['holes']:7d} {tot['points']:8d} "
        f"{tot['polylines']:10d} {tot['polyline_pts']:10d} {tot['empty']:6d}"
    )


if __name__ == "__main__":
    main()
