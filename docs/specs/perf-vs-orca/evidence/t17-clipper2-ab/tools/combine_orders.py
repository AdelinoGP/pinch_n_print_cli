#!/usr/bin/env python
"""Combine the forward and reversed interleaved runs to cancel order drift.

Forward: A(1.0.3) first, B(1.1.0) second. Reversed: B first, A second.
A within-round monotonic load drift biases whichever side runs second; the two
orders bias in opposite directions, so averaging the paired deltas cancels it.
"""

import json
import os
import statistics

BASE = os.path.dirname(os.path.abspath(__file__))


def load(name):
    with open(os.path.join(BASE, name)) as fh:
        return json.load(fh)


def main():
    fwd = load("interleaved-summary.json")
    rev = load("interleaved-summary-rev.json")
    print(
        f"{'leaf':20s} {'fwd med d%':>11s} {'rev med d%':>11s} {'order-cancelled':>16s} "
        f"{'fwd n':>6s} {'rev n':>6s}"
    )
    out = {}
    for k in sorted(fwd, key=lambda s: (s.split("/")[0], int(s.split("/")[1]))):
        fd = fwd[k]["deltas"]
        rd = rev[k]["deltas"]
        # Both are (B/A - 1) in their own order; the mean of the two medians
        # cancels a constant additive drift in log space (equivalently, to
        # first order, an additive percentage bias).
        comb = (statistics.median(fd) + statistics.median(rd)) / 2
        out[k] = {
            "fwd_median": statistics.median(fd),
            "rev_median": statistics.median(rd),
            "order_cancelled": comb,
        }
        print(
            f"{k:20s} {statistics.median(fd):+10.2f}% {statistics.median(rd):+10.2f}% "
            f"{comb:+15.2f}% {len(fd):6d} {len(rd):6d}"
        )
    with open(os.path.join(BASE, "interleaved-cancelled.json"), "w") as fh:
        json.dump(out, fh, indent=1)
    print("\nwrote interleaved-cancelled.json")


if __name__ == "__main__":
    main()
