#!/usr/bin/env python
"""Read criterion estimates.json from two CRITERION_HOME trees and tabulate.

Ticket 17 cost A/B. Like-for-like means AND slopes (t15 finding #1: criterion's
console `time:` line is the slope in Linear mode; never mix statistics).

Usage: python tabulate.py <homeA> <homeB> [labelA] [labelB]
"""

import glob
import json
import os
import sys

BASE = os.path.dirname(os.path.abspath(__file__))


def collect(home):
    out = {}
    pattern = os.path.join(BASE, home, "*", "*", "new", "estimates.json")
    for est in glob.glob(pattern):
        parts = est.replace("\\", "/").split("/")
        # .../<home>/polygon_ops_<op>/<n>/new/estimates.json
        leaf = parts[-4].replace("polygon_ops_", "") + "/" + parts[-3]
        with open(est) as fh:
            j = json.load(fh)
        out[leaf] = {
            "mean": j["mean"]["point_estimate"],
            "median": j["median"]["point_estimate"],
            "slope": j["slope"]["point_estimate"] if j.get("slope") else None,
            "std_dev": j["std_dev"]["point_estimate"],
        }
    return out


def main():
    home_a, home_b = sys.argv[1], sys.argv[2]
    la = sys.argv[3] if len(sys.argv) > 3 else home_a
    lb = sys.argv[4] if len(sys.argv) > 4 else home_b
    a = collect(home_a)
    b = collect(home_b)
    assert set(a) == set(b), set(a) ^ set(b)
    print(
        f"{'leaf':24s} {la + ' mean':>12s} {lb + ' mean':>12s} {'mean d%':>9s} "
        f"{la + ' slope':>13s} {lb + ' slope':>13s} {'slope d%':>9s} {la + ' cv%':>9s} {lb + ' cv%':>9s}"
    )
    for k in sorted(a, key=lambda s: (s.split("/")[0], int(s.split("/")[1]))):
        A, B = a[k], b[k]
        dm = (B["mean"] / A["mean"] - 1) * 100
        ds = (
            (B["slope"] / A["slope"] - 1) * 100
            if (A["slope"] and B["slope"])
            else float("nan")
        )
        cva = A["std_dev"] / A["mean"] * 100
        cvb = B["std_dev"] / B["mean"] * 100
        sa = f"{A['slope'] / 1000:13.2f}" if A["slope"] else f"{'-':>13s}"
        sb = f"{B['slope'] / 1000:13.2f}" if B["slope"] else f"{'-':>13s}"
        print(
            f"{k:24s} {A['mean'] / 1000:12.2f} {B['mean'] / 1000:12.2f} {dm:+8.2f}% "
            f"{sa} {sb} {ds:+8.2f}% {cva:8.2f}% {cvb:8.2f}%"
        )


if __name__ == "__main__":
    main()
