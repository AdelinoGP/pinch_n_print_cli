#!/usr/bin/env python3
"""t63 low-layer dispatch analysis - reconstruct dispatch timelines from t61 captures.

Reads the in-repo reduced captures of the 2026-10-13 perf refresh (all six
Benchy cells) and, for each, reconstructs the per-layer-phase dispatch
timeline without re-running anything:

1. which layers started in the first batch (t=0) and whether the set matches
   a rayon binary-split cascade boundary pattern;
2. the time-weighted open-layer count (worker occupancy field);
3. the tail: interval since the second-to-last layer end, which layers are
   open in it, and the idle worker-time the interval represents;
4. per-layer module-bracket coverage vs layer elapsed (pool-serialization
   wait detection: a wait would show as bracket coverage far below elapsed
   with no other work);
5. the tail layers' module-call breakdown.

Writes summary.json next to this script. Every number printed is re-derived
from the captures; nothing is hard-coded.

Usage (from repo root):
    python docs/specs/perf-vs-orca/evidence/t63-lowlayer-dispatch/analyze.py
"""

import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
CAPTURES = HERE.parent / "perf-refresh-2026-10-13" / "captures"


def open_layer(layers, t):
    return sum(1 for l in layers if l[1] <= t < l[2])


def analyze(name, cap):
    layers = cap["layers"]  # [index, start_ms, end_ms, elapsed_ms]
    threads = cap["metadata"]["threads"]
    calls = cap["module_calls"]  # [module_index, layer_index, elapsed_ms, end_ms, fuel]
    mids = cap["module_ids"]
    ncols = cap["module_calls_columns"]

    phase_end = max(l[2] for l in layers)
    phase_span = phase_end - min(l[1] for l in layers)

    # 1. first batch: layers starting within 2 ms of phase start
    t0 = min(l[1] for l in layers)
    first_batch = sorted(l[0] for l in layers if l[1] <= t0 + 2)

    # 2. time-weighted open-layer histogram + idle worker time
    bounds = sorted({l[1] for l in layers} | {l[2] for l in layers})
    hist = {}
    idle_ms = 0
    for a, b in zip(bounds, bounds[1:] + [phase_end]):
        if b <= a:
            continue
        o = open_layer(layers, a)
        hist[o] = hist.get(o, 0) + (b - a)
        idle_ms += (threads - o) * (b - a)

    # 3. tail structure
    ends = sorted((l[2], l[0]) for l in layers)
    last_end, last_layer = ends[-1]
    second_end, second_layer = ends[-2]
    tail_interval = last_end - second_end
    tail_layers = sorted(l[0] for l in layers if l[1] <= last_end and l[2] > second_end)
    tail_open_hist = {}
    for a, b in zip(bounds, bounds[1:] + [phase_end]):
        if b <= a or b <= second_end:
            continue
        lo = max(a, second_end)
        o = open_layer(layers, lo)
        tail_open_hist[o] = tail_open_hist.get(o, 0) + (b - lo)

    # 4. per-layer module-bracket coverage vs layer elapsed
    per_layer = {}
    for c in calls:
        per_layer.setdefault(c[1], []).append(c)
    coverage = []
    for l in layers:
        li, start, end, elapsed = l
        cs = per_layer.get(li, [])
        s = sum(c[2] for c in cs)
        coverage.append(
            {
                "layer": li,
                "elapsed_ms": elapsed,
                "module_bracket_sum_ms": s,
                "uncovered_ms": elapsed - s,
                "calls": len(cs),
            }
        )
    worst_uncovered = sorted(coverage, key=lambda r: -r["uncovered_ms"])[:5]

    # 5. tail-layer module breakdown
    tail_modules = {}
    for li in tail_layers:
        for c in sorted(per_layer.get(li, []), key=lambda c: -c[2]):
            mid = mids[c[0]]
            rec = tail_modules.setdefault(li, [])
            if len(rec) < 4:
                rec.append({"module": mid, "elapsed_ms": c[2], "end_ms": c[3]})

    result = {
        "cell": name,
        "threads": threads,
        "phase_span_ms": phase_span,
        "first_batch": first_batch,
        "first_batch_is_cascade_boundaries": cascade_match(first_batch, layers),
        "open_hist_ms": {str(k): v for k, v in sorted(hist.items())},
        "idle_worker_seconds": idle_ms / 1000.0,
        "last_layer": last_layer,
        "last_end_ms": last_end,
        "second_last_layer": second_layer,
        "second_last_end_ms": second_end,
        "tail_interval_ms": tail_interval,
        "tail_layers": tail_layers,
        "tail_open_hist_ms": {str(k): v for k, v in sorted(tail_open_hist.items())},
        "worst_uncovered": worst_uncovered,
        "tail_modules": tail_modules,
        "low_layers": {
            str(l[0]): {"start_ms": l[1], "end_ms": l[2], "elapsed_ms": l[3]}
            for l in layers
            if l[0] <= 2
        },
        "low_layer_module_calls": {
            str(li): [
                {"module": mids[c[0]], "elapsed_ms": c[2], "end_ms": c[3]}
                for c in sorted(per_layer.get(li, []), key=lambda c: c[3])
            ]
            for li in (0, 1, 2)
        },
    }
    return result


def cascade_match(first_batch, layers):
    """Does the first batch look like a rayon left-spine split cascade?

    For a halving split of [0..n), the initiator's left spine split points
    are n/2, n/4, n/8, ... down to small leaves; the initiator itself starts
    layer 0. Thief subtrees start at their own left-spine origins. Signature
    check: how many left-spine split points appear as observed starts, and
    which spine member is late (the initiator's own leaf work).
    """
    n = len(layers)
    observed = set(first_batch)
    spine = []
    lo, hi = 0, n
    while hi - lo > 1:
        mid = (lo + hi) // 2
        spine.append(mid)
        hi = mid  # initiator recurses into the LEFT half
    # spine[0] is the top-level thief boundary (n/2); the deepest entries are
    # the initiator's own leaf split points, which start late.
    hits = [b for b in spine if b in observed]
    late = [b for b in spine if b not in observed]
    return {
        "spine_split_points": spine,
        "spine_in_first_batch": hits,
        "spine_not_in_batch": late,
        "spine_hits": len(hits),
        "spine_total": len(spine),
    }


def main():
    results = []
    for path in sorted(CAPTURES.glob("*.json")):
        cap = json.loads(path.read_text())
        results.append(analyze(path.stem, cap))
    summary = {
        "source_captures": "docs/specs/perf-vs-orca/evidence/perf-refresh-2026-10-13/captures/",
        "method": "phase-relative per-layer start/end from reduced t61 captures (no re-slice)",
        "results": results,
    }
    out = HERE / "summary.json"
    out.write_text(json.dumps(summary, indent=1))
    for r in results:
        print(f"== {r['cell']} (threads={r['threads']}, phase {r['phase_span_ms']} ms)")
        print(f"   first batch: {r['first_batch']}")
        print(f"   open-count hist(ms): {r['open_hist_ms']}")
        print(f"   idle worker time: {r['idle_worker_seconds']:.1f} s")
        print(
            f"   tail: layer {r['last_layer']} ends {r['last_end_ms']}, "
            f"prev end {r['second_last_end_ms']} -> interval {r['tail_interval_ms']} ms"
        )
        print(f"   tail open hist: {r['tail_open_hist_ms']}")
        print(f"   tail layers {r['tail_layers']}: {r['tail_modules']}")
        wu = ", ".join(
            f"L{w['layer']}:{w['uncovered_ms']}ms" for w in r["worst_uncovered"]
        )
        print(f"   worst bracket-uncovered layers: {wu}")
        print(f"   low layers: {r['low_layers']}")


if __name__ == "__main__":
    sys.exit(main())
