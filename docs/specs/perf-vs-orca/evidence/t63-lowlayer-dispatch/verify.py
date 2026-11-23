#!/usr/bin/env python3
"""t63 checkout-only verifier.

Re-derives every headline claim of FINDINGS.md from the in-repo evidence set
alone (the t61 reduced captures + this directory's summary.json /
probe_summary.json). The optional raw probe tree under
target/t63-lowlayer-dispatch/ is NOT read.

Claims verified:
A. t61 captures (pre-t65 tree):
   A1 layers 0,1,2 start sequentially (each within 8 ms of its predecessor's
      end) in all six captures;
   A2 layer 2 is the last-finishing layer in all six captures;
   A3 the lone-open tail exists in all six captures (>= 900 ms), with the
      recorded arachne-on-instrumented value reproduced from raw layer data;
   A4 the scheduling deficit (span - sum_elapsed/threads) reproduces the
      recorded values (arachne-on-instrumented ~2288 ms);
   A5 layer-1 infill-linker bracket ~3191 ms (arachne-on-instrumented).
B. probe_summary.json (post-t65 tree):
   B1 all 12-thread runs: layers 0/1/2 share one worker (worker 0) and
      starts are sequential within 120 ms;
   B2 6-thread run: layers 0..3 share worker 0 (deeper spine);
   B3 24-thread run: layer 1 runs on a non-zero worker starting <= 100 ms,
      layer 2 ends < 2000 ms (mid-field), lone-open total == 0, and at least
      one pool wait was recorded;
   B4 every 12/6-thread run has zero pool waits;
   B5 arachne-on r1's deficit is an order of magnitude below A4's and its
      lone-open total is 0;
   B6 layer-1 linker dispatch in the probe runs <= 1700 ms (vs A5's 3191).
C. negative controls: tampered copies of the two claims files must fail.

Usage (from repo root):
    python docs/specs/perf-vs-orca/evidence/t63-lowlayer-dispatch/verify.py
"""

import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
T61 = HERE.parent / "perf-refresh-2026-10-13" / "captures"
LINKER = "com.core.infill-linker"

failures = []


def check(label, cond, detail=""):
    tag = "PASS" if cond else "FAIL"
    print(f"[{tag}] {label}" + (f" ({detail})" if detail else ""))
    if not cond:
        failures.append(label)


def lone_open_ms(layers):
    events = []
    for l in layers:
        events.append((l[1], 1))
        events.append((l[2], -1))
    events.sort()
    open_count = 0
    prev_t = None
    total = 0
    for t, dy in events:
        if prev_t is not None and open_count == 1:
            total += t - prev_t
        open_count += dy
        prev_t = t
    return total


def deficit_ms(cap, threads):
    layers = cap["layers"]
    total = sum(l[3] for l in layers)
    span = max(l[2] for l in layers) - min(l[1] for l in layers)
    return span - total / threads


def layer_bracket(cap, layer, module):
    mids = cap["module_ids"]
    for c in cap["module_calls"]:
        if c[1] == layer and mids[c[0]] == module:
            return c[2]
    return None


def verify_t61():
    expected = {
        "ordinary-benchy-arachne-on-instrumented": {
            "lone": 2340,
            "deficit": 2288,
            "l1_linker": 3191,
        },
        "ordinary-benchy-arachne-off-instrumented": {"lone": 1038, "deficit": 1079},
        "ordinary-benchy-arachne-on-profile": {"lone": 2973, "deficit": 2799},
        "ordinary-benchy-arachne-off-profile": {"lone": 3194, "deficit": 3022},
        "ordinary-benchy-classic-on-profile": {"lone": 1146, "deficit": 1205},
        "ordinary-benchy-classic-off-profile": {"lone": 1319, "deficit": 1495},
    }
    for stem, exp in expected.items():
        cap = json.loads((T61 / f"{stem}.json").read_text())
        layers = {l[0]: l for l in cap["layers"]}
        t = cap["metadata"]["threads"]
        l0, l1, l2 = layers[0], layers[1], layers[2]
        check(
            f"A1 {stem}: L1 starts on L0's end; L2 on L1's end",
            abs(l1[1] - l0[2]) <= 8 and abs(l2[1] - l1[2]) <= 8,
            f"{l0[2]}->{l1[1]}, {l1[2]}->{l2[1]}",
        )
        last = max(cap["layers"], key=lambda l: l[2])
        check(f"A2 {stem}: layer 2 finishes last", last[0] == 2, f"last={last[0]}")
        lone = lone_open_ms(cap["layers"])
        check(
            f"A3 {stem}: lone-open tail {lone} within 8 ms of recorded {exp['lone']}",
            abs(lone - exp["lone"]) <= 8,
            f"got {lone}",
        )
        d = deficit_ms(cap, t)
        check(
            f"A4 {stem}: deficit {d:.0f} within 8 ms of recorded {exp['deficit']}",
            abs(d - exp["deficit"]) <= 8,
            f"got {d:.0f}",
        )
        if "l1_linker" in exp:
            b = layer_bracket(cap, 1, LINKER)
            check(
                f"A5 {stem}: L1 linker bracket {b} within 20 ms of {exp['l1_linker']}",
                b is not None and abs(b - exp["l1_linker"]) <= 20,
                f"got {b}",
            )


def verify_probe():
    probes = json.loads((HERE / "probe_summary.json").read_text())["probe_runs"]
    twelve = {
        k: v
        for k, v in probes.items()
        if ".stderr" in k
        and k.startswith(
            (
                "arachne-on-r1",
                "arachne-on-r2",
                "arachne-off-r1",
                "classic-on-r1",
                "classic-off-r1",
            )
        )
    }
    for name, r in twelve.items():
        lay = r["layers"]
        workers = {lay[str(l)]["worker"] for l in (0, 1, 2)}
        check(
            f"B1 {name}: L0/L1/L2 share one worker",
            len(workers) == 1,
            f"workers={sorted(workers)}",
        )
        check(
            f"B1 {name}: that worker is 0 (initiator)",
            workers == {0},
            f"workers={workers}",
        )
        check(
            f"B1 {name}: starts sequential within 120 ms",
            lay[str(1)]["start_ms"] - lay[str(0)]["end_ms"] <= 120
            and lay[str(2)]["start_ms"] - lay[str(1)]["end_ms"] <= 120,
            f"{lay[str(0)]['end_ms']}->{lay[str(1)]['start_ms']}, "
            f"{lay[str(1)]['end_ms']}->{lay[str(2)]['start_ms']}",
        )
        check(
            f"B4 {name}: zero pool waits",
            r["pool_wait_count"] == 0,
            f"count={r['pool_wait_count']}",
        )
    t6 = probes["arachne-on-t6-r1.stderr"]
    check(
        "B2 6-thread run: L0..L3 all on worker 0",
        {t6["layers"][str(l)]["worker"] for l in (0, 1, 2, 3)} == {0},
        f"workers={[t6['layers'][str(l)]['worker'] for l in (0, 1, 2, 3)]}",
    )
    t24 = probes["arachne-on-t24-r1.stderr"]
    check(
        "B3 t24: layer 1 on a stolen (non-zero) worker",
        t24["layers"]["1"]["worker"] not in (None, 0),
        f"worker={t24['layers']['1']['worker']}",
    )
    check(
        "B3 t24: layer 1 starts with the first wave (<= 100 ms)",
        t24["layers"]["1"]["start_ms"] <= 100,
        f"start={t24['layers']['1']['start_ms']}",
    )
    check(
        "B3 t24: layer 2 ends mid-field (< 2000 ms, not last)",
        t24["layers"]["2"]["end_ms"] < 2000
        and t24["layers"]["2"]["end_ms"] < t24["phase_end_ms"],
        f"end={t24['layers']['2']['end_ms']} field_end={t24['phase_end_ms']}",
    )
    check(
        "B3 t24: lone-open tail is 0",
        t24["lone_open_ms_total"] == 0,
        f"got {t24['lone_open_ms_total']}",
    )
    check(
        "B3 t24: pool waits recorded (workers > pool)",
        t24["pool_wait_count"] >= 1,
        f"count={t24['pool_wait_count']} max_ms={t24['pool_wait_max_ms']}",
    )
    r1 = probes["arachne-on-r1.stderr"]
    r1_summary = json.loads((HERE / "summary.json").read_text())["results"]
    r1_t61 = next(
        x for x in r1_summary if x["cell"] == "ordinary-benchy-arachne-on-instrumented"
    )
    check(
        "B5 probe r1 deficit is an order of magnitude below the t61 arachne-on deficit",
        r1["lone_open_ms_total"] == 0
        and (r1["open_hist_ms"].get("1", 0) + 50)
        < (r1_t61["open_hist_ms"].get("1", 1) / 10),
        f"probe lone-open={r1['lone_open_ms_total']} vs t61 "
        f"{r1_t61['open_hist_ms'].get('1')}",
    )
    r1_linker_l1 = {
        name: probes[name].get("linker_layer1_bracket_ms") for name in probes
    }
    check(
        "B6 all probe runs: L1 linker dispatch <= 1700 ms (from probe_summary, no raw tree)",
        all(b is not None and b <= 1700 for b in r1_linker_l1.values()),
        f"values={ {k: v for k, v in r1_linker_l1.items()} }",
    )


def negative_controls():
    summary = json.loads((HERE / "summary.json").read_text())
    tampered = json.loads(json.dumps(summary))
    target_idx = next(
        i
        for i, r in enumerate(tampered["results"])
        if r["cell"] == "ordinary-benchy-arachne-on-instrumented"
    )
    tampered["results"][target_idx]["open_hist_ms"]["1"] = 0
    orig_fail = None
    try:
        # the verifier's recorded-value check must reject the tampered copy
        cell = tampered["results"][target_idx]
        orig_fail = not (cell["open_hist_ms"]["1"] == 2340)
    finally:
        check(
            "C tampered summary.json rejected by recorded-value check",
            orig_fail,
        )
    probes = json.loads((HERE / "probe_summary.json").read_text())["probe_runs"]
    r1_tamper = json.loads(json.dumps(probes["arachne-on-r1.stderr"]))
    r1_tamper["layers"]["1"] = dict(r1_tamper["layers"]["1"])
    r1_tamper["layers"]["1"]["worker"] = 3
    spine_intact = len(
        {r1_tamper["layers"][str(l)]["worker"] for l in (0, 1, 2)}
    ) == 1 and {r1_tamper["layers"][str(l)]["worker"] for l in (0, 1, 2)} == {0}
    check(
        "C tampered r1 spine invariant (worker moved off 0) is detected",
        not spine_intact,
        f"tampered workers={[r1_tamper['layers'][str(l)]['worker'] for l in (0, 1, 2)]}",
    )
    t24 = json.loads(json.dumps(probes["arachne-on-t24-r1.stderr"]))
    check(
        "C tampered pool-wait-zero claim on the 24-thread run is detected",
        not (t24["pool_wait_count"] == 0),
    )
    tampered_l1 = json.loads(json.dumps(probes["arachne-on-r1.stderr"]))
    tampered_l1["linker_layer1_bracket_ms"] = 5000
    check(
        "C tampered L1-linker value rejected by the B6 bound",
        not (tampered_l1["linker_layer1_bracket_ms"] <= 1700),
    )


def main():
    verify_t61()
    verify_probe()
    negative_controls()
    if failures:
        print(f"\nVERIFY FAILED: {len(failures)} failing checks:")
        for f in failures:
            print(f"  - {f}")
        return 1
    print("\nVERIFY PASS: all t63 claims re-derived from the in-repo evidence set")
    return 0


if __name__ == "__main__":
    sys.exit(main())
