#!/usr/bin/env python3
"""t63 worker-identity reducer.

Parses the probe stderr JSONL captures (target/t63-lowlayer-dispatch/) into a
reduced probe_summary.json in this evidence directory:

- per run: every layer's (start, end, worker, os-thread-id) from the T63EVENT
  probe lines, cross-checked against the JSONL layer events;
- layer -> worker assignment sequences (chunk/depth-first structure);
- the first batch's worker set and whether layers 0/1/2 share one worker;
- per-worker layer sequences to show LIFO spine descent vs breadth-first
  thieves;
- T63POOL wait events (count, total, max by run).

Usage (from repo root):
    python docs/specs/perf-vs-orca/evidence/t63-lowlayer-dispatch/reduce.py
Optional raw-tree reconstruction:
    python docs/specs/perf-vs-orca/evidence/t63-lowlayer-dispatch/reduce.py --raw-root target/t63-lowlayer-dispatch
"""

import json
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent

EVENT_RE = re.compile(
    r"^T63EVENT (?P<event>\S+) (?P<layer>layer=\d+) (?P<worker>worker=\S+|worker=None) "
    r"(?P<tid>tid=ThreadId\((\d+|Rust \(.*\)|.*\))) (?P<unix_ms>unix_ms=\d+)$"
)
POOL_RE = re.compile(
    r"^T63POOL wait_us=(?P<wait_us>\d+) wait_ms=(?P<wait_ms>[\d.]+) worker=(?P<worker>SOME|None|\S+)$"
)


def parse_probe_line(line):
    # T63EVENT layer_start layer=3 worker=Some(5) tid=ThreadId(11) unix_ms=123
    m = re.match(
        r"T63EVENT (\w+) layer=(\d+) worker=(\S+) tid=(\S+) unix_ms=(\d+)", line
    )
    if not m:
        return None
    worker = None
    if m.group(3).startswith("Some("):
        worker = int(m.group(3)[5:-1])
    return {
        "event": m.group(1),
        "layer": int(m.group(2)),
        "worker": worker,
        "tid": m.group(4),
        "unix_ms": int(m.group(5)),
    }


def reduce_run(stderr_path):
    probes = []
    json_layers = {}
    pool_waits = []
    bad_lines = 0
    for line in stderr_path.read_text(errors="replace").splitlines():
        if not line.strip():
            continue
        if line.startswith("T63POOL"):
            m = POOL_RE.match(line)
            if m:
                pool_waits.append(float(m.group("wait_ms")))
            continue
        if line.startswith("T63EVENT"):
            p = parse_probe_line(line)
            if p is None:
                bad_lines += 1
            else:
                probes.append(p)
            continue
        if line.startswith("{"):
            try:
                ev = json.loads(line)
            except json.JSONDecodeError:
                continue
            if ev.get("event") == "layer_start":
                json_layers[ev["layer_index"]] = ev["timestamp_ms"]
            elif ev.get("event") == "layer_complete":
                prev = json_layers.setdefault(ev["layer_index"], None)
                if prev is not None:
                    json_layers[ev["layer_index"]] = (prev, ev["timestamp_ms"])
    starts = [p for p in probes if p["event"] == "layer_start"]
    ends = [p for p in probes if p["event"] == "layer_end"]
    by_layer_start = {p["layer"]: p for p in starts}
    by_layer_end = {p["layer"]: p for p in ends}
    t0 = min(p["unix_ms"] for p in starts)

    phases = {}
    linker_l1 = None
    linker_low_sum = 0
    for line in stderr_path.read_text(errors="replace").splitlines():
        if not line.startswith("{"):
            continue
        try:
            ev = json.loads(line)
        except json.JSONDecodeError:
            continue
        if ev.get("event") == "phase_complete" and ev.get("phase") == "PerLayer":
            phases["per_layer_ms"] = ev.get("elapsed_ms")
        if (
            ev.get("event") == "module_complete"
            and ev.get("module_id") == "com.core.infill-linker"
            and ev.get("layer_index") in (0, 1, 2)
        ):
            elapsed = ev.get("elapsed_ms") or 0
            linker_low_sum += elapsed
            if ev.get("layer_index") == 1:
                linker_l1 = elapsed

    layers_out = {}
    for layer, p in sorted(by_layer_start.items()):
        e = by_layer_end.get(layer)
        rel_start = p["unix_ms"] - t0
        rel_end = (e["unix_ms"] - t0) if e else None
        layers_out[layer] = {
            "worker": p["worker"],
            "tid": p["tid"],
            "start_ms": rel_start,
            "end_ms": rel_end,
            "elapsed_ms": (rel_end - rel_start) if rel_end is not None else None,
        }

    # per-worker ordered layer sequences (over time)
    worker_sequences = {}
    for layer, p in sorted(by_layer_start.items(), key=lambda kv: kv[1]["unix_ms"]):
        worker_sequences.setdefault(p["worker"], []).append(layer)
    # per-worker END order (completion order) to show tails
    worker_end_order = {}
    for layer, p in sorted(by_layer_end.items(), key=lambda kv: kv[1]["unix_ms"]):
        worker_end_order.setdefault(p["worker"], []).append(layer)

    first_batch = sorted(l for l, v in layers_out.items() if v["start_ms"] <= 25)
    phase_end = max(v["end_ms"] for v in layers_out.values() if v["end_ms"] is not None)
    second_end = sorted(
        v["end_ms"] for v in layers_out.values() if v["end_ms"] is not None
    )[-2]
    tail_layers = sorted(
        l
        for l, v in layers_out.items()
        if v["start_ms"] <= second_end and v["end_ms"] >= second_end
    )

    # open-count field: occupancy histogram plus every lone-open interval >= 50 ms
    bounds = sorted(
        {v["start_ms"] for v in layers_out.values()}
        | {v["end_ms"] for v in layers_out.values() if v["end_ms"] is not None}
    )
    hist = {}
    lone_open = []
    for a, b in zip(bounds, bounds[1:]):
        if b <= a:
            continue
        open_here = sorted(
            l
            for l, v in layers_out.items()
            if v["start_ms"] <= a and v["end_ms"] is not None and v["end_ms"] >= b
        )
        hist[len(open_here)] = hist.get(len(open_here), 0) + (b - a)
        if len(open_here) == 1 and (b - a) >= 50:
            lone_open.append(
                {"from_ms": a, "to_ms": b, "layer": open_here[0], "ms": b - a}
            )
    total_work = sum(v["elapsed_ms"] or 0 for v in layers_out.values())
    floor_threads = len({layers_out[l]["tid"] for l in layers_out})
    return {
        "layers": layers_out,
        "first_batch": first_batch,
        "first_batch_workers": sorted(
            v["worker"] for v in layers_out.values() if v["start_ms"] <= 25
        ),
        "phase_end_ms": phase_end,
        "phase_per_layer_wall_ms": phases.get("per_layer_ms"),
        "linker_layer1_bracket_ms": linker_l1,
        "linker_layers_012_bracket_sum_ms": linker_low_sum,
        "tail_layers": tail_layers,
        "low_layers_on_one_worker": (
            len(
                {layers_out[l]["worker"] and layers_out[l]["worker"] for l in (0, 1, 2)}
            )
            == 1
        ),
        "low_layers_workers": [layers_out[l]["worker"] for l in (0, 1, 2)],
        "open_hist_ms": {str(k): v for k, v in sorted(hist.items())},
        "lone_open_ms_total": sum(x["ms"] for x in lone_open),
        "lone_open": lone_open,
        "total_layer_elapsed_ms": total_work,
        "work_floor_ms": total_work / floor_threads if floor_threads else None,
        "distinct_worker_tids": floor_threads,
        "worker_sequences": {str(k): v for k, v in worker_sequences.items()},
        "worker_end_order": {str(k): v for k, v in worker_end_order.items()},
        "pool_waits": pool_waits,
        "pool_wait_count": len(pool_waits),
        "pool_wait_max_ms": max(pool_waits) if pool_waits else 0.0,
        "probe_lines": len(probes),
        "unparsed_probe_lines": bad_lines,
        "json_layer_events": len(json_layers),
    }


def main():
    raw = Path("target/t63-lowlayer-dispatch")
    if "--raw-root" in sys.argv:
        raw = Path(sys.argv[sys.argv.index("--raw-root") + 1])
    if not raw.exists():
        candidate = HERE.parent.parent / raw.name
        if candidate.exists():
            raw = candidate
        else:
            print(
                f"no probe raw tree at {raw} (or {candidate}); pass --raw-root",
                file=sys.stderr,
            )
            return 2
    runs = {}
    for path in sorted(raw.glob("*.stderr.jsonl")):
        runs[path.stem] = reduce_run(path)
    out = {"probe_runs": runs}
    (HERE / "probe_summary.json").write_text(json.dumps(out, indent=1))
    for name, r in runs.items():
        print(
            f"== {name}: layers={len(r['layers'])} probe_lines={r['probe_lines']} "
            f"unparsed={r['unparsed_probe_lines']}"
        )
        low = {l: r["layers"][l] for l in (0, 1, 2)}
        print(
            f"   low layers: "
            + "; ".join(
                f"L{l}: w={v['worker']} tid={v['tid']} [{v['start_ms']}..{v['end_ms']}]"
                for l, v in low.items()
            )
        )
        print(
            f"   low layers on one worker: {r['low_layers_on_one_worker']}"
            f" (workers {r['low_layers_workers']})"
        )
        print(
            f"   phase per-layer wall: {r['phase_per_layer_wall_ms']} ms; work floor"
            f" {r['total_layer_elapsed_ms']}ms/{r['distinct_worker_tids']}w ="
            f" {r['work_floor_ms']:.0f} ms"
        )
        print(f"   first batch ({len(r['first_batch'])}): {r['first_batch']}")
        print(f"   first-batch workers: {r['first_batch_workers']}")
        tailstr = ", ".join(
            f"layer {l}: w={r['layers'][l]['worker']} end={r['layers'][l]['end_ms']}ms"
            for l in r["tail_layers"]
        )
        print(f"   tail layers: {r['tail_layers']} | {tailstr}")
        print(f"   open-count hist(ms): {r['open_hist_ms']}")
        print(
            f"   lone-open intervals (>=50 ms, total {r['lone_open_ms_total']} ms): "
            + "; ".join(
                f"[{x['from_ms']}..{x['to_ms']}] L{x['layer']} ({x['ms']} ms)"
                for x in r["lone_open"]
            )
        )
        seqs = {
            w: s[:4]
            for w, s in sorted(r["worker_sequences"].items(), key=lambda kv: str(kv[0]))
        }
        print(f"   worker lead sequences: {seqs}")
        print(
            f"   pool waits: count={r['pool_wait_count']} max_ms={r['pool_wait_max_ms']}"
        )


if __name__ == "__main__":
    sys.exit(main())
