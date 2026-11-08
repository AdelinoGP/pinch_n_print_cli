#!/usr/bin/env python3
"""Self-contained verification of the perf-refresh-2026-10-09 reduced set.

Exits 0 when every committed identity holds; exits 1 with a message otherwise.
Needs only the reduced/*.json files (no raw captures, no external deps).

Checks:
  classic-cells.json
    - every cell's phase sum + post-phase tail leaves a non-negative
      head/inter-phase remainder under 2.5 s (the measured module-discovery
      head is ~1.1-1.5 s per ticket 27);
    - every cell's elapsed_ms is present and > 0.
  finalization-gaps.json
    - gaps_total_ms == stage_elapsed_ms - module_bracket_total_ms (1 ms);
    - base-classic-on: 4 module brackets and gaps >= 8 s each (the finding);
    - base-classic-off: total copy overhead <= 100 ms (the negative control).
  t27-ab-postpass.json
    - each arm's stored median recomputes from its runs;
    - every base supports-on postpass sample is > 90 s (the headline: the
      postpass cost is present in every 2026-10-09 A/B run, both modes).
"""

import json
import os
import statistics
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
RED = os.path.join(HERE, "reduced")
failures = []


def check(cond, msg):
    if not cond:
        failures.append(msg)


cells = json.load(open(os.path.join(RED, "classic-cells.json"), encoding="utf-8"))
for name, c in cells.items():
    phases = sum(v for v in c["phases_ms"].values() if v is not None)
    tail = c["tail_ms"] or 0
    remainder = c["elapsed_ms"] - phases - tail
    check(c["elapsed_ms"] and c["elapsed_ms"] > 0, f"{name}: missing elapsed")
    check(
        0 <= remainder <= 2500,
        f"{name}: head/inter-phase remainder {remainder} ms out of range",
    )
    check(c["status"] == "ok" and c["degraded"] is False, f"{name}: not clean")

gaps = json.load(open(os.path.join(RED, "finalization-gaps.json"), encoding="utf-8"))
for name, g in gaps.items():
    delta = abs(g["stage_minus_modules_ms"] - g["gaps_total_ms"])
    # Event timestamps are millisecond-quantized, so the identity can drift
    # by a few ms across the four bracket boundaries.
    check(
        delta <= 5,
        f"{name}: stage-modules {g['stage_minus_modules_ms']} != gaps {g['gaps_total_ms']}",
    )
on = gaps["base-classic-on"]
check(
    len(on["gaps"]) == 4 and all(x["gap_ms"] >= 8000 for x in on["gaps"]),
    "base-classic-on: expected 4 gaps >= 8 s",
)
check(on["gaps_total_ms"] >= 40000, "base-classic-on: gaps_total below 40 s")
off = gaps["base-classic-off"]
check(
    off["gaps_total_ms"] <= 100, "base-classic-off: copy overhead should be <= 100 ms"
)

ab = json.load(open(os.path.join(RED, "t27-ab-postpass.json"), encoding="utf-8"))
check(len(ab) == 4, f"expected 4 A/B arms, found {len(ab)}")
for arm, data in ab.items():
    for fld in ("prepass_ms", "per_layer_ms", "postpass_ms", "elapsed_ms"):
        xs = [r[fld] for r in data["runs"] if r[fld] is not None]
        med = round(statistics.median(xs)) if xs else None
        check(
            med == data[fld + "_median"],
            f"{arm}: {fld} median mismatch ({med} vs {data[fld + '_median']})",
        )
    posts = [r["postpass_ms"] for r in data["runs"]]
    check(all(p > 90000 for p in posts), f"{arm}: postpass sample under 90 s: {posts}")

if failures:
    print("FAIL")
    for f in failures:
        print("  -", f)
    sys.exit(1)
print("PASS: all perf-refresh-2026-10-09 reduced identities hold")
