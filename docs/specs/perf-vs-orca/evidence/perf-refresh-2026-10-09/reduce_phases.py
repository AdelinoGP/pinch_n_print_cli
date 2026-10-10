#!/usr/bin/env python3
"""Reduce the 2026-10-09 perf-refresh instrumented captures to committed
evidence tables (docs/specs/perf-vs-orca/evidence/perf-refresh-2026-10-09/).

Raw captures (gitignored, the fuller optional source):
  target/perf-refresh-2026-10-09/<cell>.stderr.jsonl   (4 profiled cells)
  target/t27-ab/campaign/measure/<mode>/base/*.stderr.jsonl  (2026-10-09 A/B)

Usage:
  python reduce_phases.py <repo-root> <out-dir>

Emits:
  classic-cells.json        phases / stages / modules / profile rows per cell
  t27-ab-postpass.json      phase splits of the 2026-10-09 A/B base captures
  finalization-gaps.json    LayerFinalization bracket gaps, supports-on/off base

This script is committed; the raw captures it reads are gitignored by policy.
"""

import collections
import glob
import json
import os
import sys

CELLS = [
    "benchy-classic-off",
    "benchy-classic-on",
    "base-classic-off",
    "base-classic-on",
]


def load(path):
    rows = []
    with open(path, encoding="utf-8", errors="replace") as f:
        for line in f:
            line = line.strip()
            if not line.startswith("{"):
                continue
            try:
                rows.append(json.loads(line))
            except json.JSONDecodeError:
                pass
    return rows


def slice_id_ms(rows):
    for r in rows:
        sid = r.get("slice_id")
        if isinstance(sid, str) and sid.startswith("slice-"):
            try:
                return int(sid.split("-", 1)[1])
            except ValueError:
                return None
    return None


def cell_report(rows):
    phases = {}
    phase_end = {}
    stages = collections.Counter()
    mods = collections.Counter()
    mods_n = collections.Counter()
    sc = None
    profile = None
    first_ts = None
    for r in rows:
        ev = r.get("event")
        ts = r.get("timestamp_ms")
        if ts is not None and first_ts is None:
            first_ts = ts
        if ev == "phase_complete":
            phases[r["phase"]] = r.get("elapsed_ms")
            phase_end[r["phase"]] = ts
        elif ev == "stage_complete":
            stages[r["stage"]] += r.get("elapsed_ms", 0)
        elif ev == "module_complete":
            mods[r["module_id"]] += r.get("elapsed_ms", 0)
            mods_n[r["module_id"]] += 1
        elif ev == "slice_complete":
            sc = r
        elif ev == "profile_summary":
            profile = r.get("profile")
    out = {
        "elapsed_ms": sc.get("elapsed_ms"),
        "status": sc.get("status"),
        "degraded": sc.get("degraded"),
        "fatal_error_count": sc.get("fatal_error_count"),
        "non_fatal_error_count": sc.get("non_fatal_error_count"),
        "phases_ms": {
            k: phases.get(k) for k in ("validation", "prepass", "per_layer", "postpass")
        },
        "head_ms": (first_ts - slice_id_ms(rows))
        if (first_ts and slice_id_ms(rows))
        else None,
        "tail_ms": (sc.get("timestamp_ms") - phase_end.get("postpass"))
        if phase_end.get("postpass")
        else None,
        "stages_top": stages.most_common(14),
        "modules_top": [(m, ms, mods_n[m]) for m, ms in mods.most_common(14)],
    }
    if profile:
        host = []
        guest = []
        for m in profile.get("modules", []):
            if m.get("unit") == "wall_ns":
                host.append(
                    {
                        "module_id": m["module_id"],
                        "calls": m.get("calls"),
                        "total_wall_ms": round(m.get("total_wall_ns", 0) / 1e6, 1),
                        "scopes": [
                            {
                                "scope": s["scope"],
                                "calls": s.get("calls"),
                                "total_wall_ms": round(
                                    s.get("total_wall_ns", 0) / 1e6, 1
                                ),
                            }
                            for s in m.get("scopes", [])
                        ],
                    }
                )
            elif m.get("unit") == "fuel":
                guest.append(
                    {
                        "module_id": m["module_id"],
                        "calls": m.get("calls"),
                        "total_fuel": m.get("total_fuel"),
                        "self_fuel": m.get("self_fuel"),
                        "scopes": [
                            {
                                "scope": s["scope"],
                                "calls": s.get("calls"),
                                "total_fuel": s.get("total_fuel"),
                            }
                            for s in m.get("scopes", [])
                        ],
                    }
                )
        out["profile_fuel_total"] = profile.get("fuel_total")
        out["profile_host_wall_ns_total"] = profile.get("wall_total_ns")
        out["profile_guest_top"] = sorted(guest, key=lambda g: -(g["total_fuel"] or 0))[
            :8
        ]
        out["profile_host"] = sorted(host, key=lambda h: -(h["total_wall_ms"] or 0))
    return out


def finalization_gaps(rows):
    evs = [
        r
        for r in rows
        if r.get("stage") == "PostPass::LayerFinalization"
        and r.get("phase") == "postpass"
        and r.get("event")
        in ("stage_start", "stage_complete", "module_start", "module_complete")
    ]
    events = []
    for r in evs:
        events.append(
            {
                "event": r.get("event"),
                "module_id": r.get("module_id"),
                "timestamp_ms": r.get("timestamp_ms"),
                "elapsed_ms": r.get("elapsed_ms"),
            }
        )
    # gaps: module_complete -> next module_start (or stage_complete)
    gaps = []
    pending_end = None
    modules_total = 0
    stage_elapsed = None
    stage_start_ts = None
    for e in events:
        if e["event"] == "stage_start":
            stage_start_ts = e["timestamp_ms"]
        elif e["event"] == "module_complete":
            modules_total += e["elapsed_ms"] or 0
            pending_end = e
        elif e["event"] == "module_start" and pending_end is not None:
            gaps.append(
                {
                    "after": pending_end["module_id"],
                    "before": e["module_id"],
                    "gap_ms": e["timestamp_ms"] - pending_end["timestamp_ms"],
                }
            )
            pending_end = None
        elif e["event"] == "stage_complete":
            stage_elapsed = e["elapsed_ms"]
            if pending_end is not None:
                gaps.append(
                    {
                        "after": pending_end["module_id"],
                        "before": "stage_complete",
                        "gap_ms": e["timestamp_ms"] - pending_end["timestamp_ms"],
                    }
                )
    return {
        "events": events,
        "module_bracket_total_ms": modules_total,
        "stage_elapsed_ms": stage_elapsed,
        "gaps": gaps,
        "gaps_total_ms": sum(g["gap_ms"] for g in gaps),
        "stage_minus_modules_ms": (stage_elapsed - modules_total)
        if stage_elapsed is not None
        else None,
    }


def ab_report(root):
    import statistics

    arms = {}
    pat = os.path.join(
        root, "target", "t27-ab", "campaign", "measure", "*", "base", "*.stderr.jsonl"
    )
    for p in sorted(glob.glob(pat)):
        base = os.path.basename(p)
        if "attempt1-killed" in base or "-w0." in base:
            continue
        mode = p.replace("\\", "/").split("/")[-3]
        arm = "baseline" if "-baseline-" in base else "candidate"
        rows = load(p)
        phases = {}
        sc = None
        for r in rows:
            if r.get("event") == "phase_complete":
                phases[r["phase"]] = r.get("elapsed_ms")
            elif r.get("event") == "slice_complete":
                sc = r.get("elapsed_ms")
        arms.setdefault((mode, arm), []).append(
            {
                "file": base,
                "prepass_ms": phases.get("prepass"),
                "per_layer_ms": phases.get("per_layer"),
                "postpass_ms": phases.get("postpass"),
                "elapsed_ms": sc,
            }
        )
    out = {}
    for (mode, arm), vals in sorted(arms.items()):
        entry = {"runs": vals}
        for fld in ("prepass_ms", "per_layer_ms", "postpass_ms", "elapsed_ms"):
            xs = [v[fld] for v in vals if v[fld] is not None]
            entry[fld + "_median"] = round(statistics.median(xs)) if xs else None
        out[f"{mode}/{arm}"] = entry
    return out


def main():
    root, outdir = sys.argv[1], sys.argv[2]
    os.makedirs(outdir, exist_ok=True)
    raw = os.path.join(root, "target", "perf-refresh-2026-10-09")

    cells = {}
    for cell in CELLS:
        cells[cell] = cell_report(load(os.path.join(raw, cell + ".stderr.jsonl")))
    with open(os.path.join(outdir, "classic-cells.json"), "w", encoding="utf-8") as f:
        json.dump(cells, f, indent=2)

    gaps = {}
    for cell in ("base-classic-off", "base-classic-on"):
        gaps[cell] = finalization_gaps(load(os.path.join(raw, cell + ".stderr.jsonl")))
    with open(
        os.path.join(outdir, "finalization-gaps.json"), "w", encoding="utf-8"
    ) as f:
        json.dump(gaps, f, indent=2)

    with open(os.path.join(outdir, "t27-ab-postpass.json"), "w", encoding="utf-8") as f:
        json.dump(ab_report(root), f, indent=2)

    print(
        "wrote classic-cells.json, finalization-gaps.json, t27-ab-postpass.json to",
        outdir,
    )


if __name__ == "__main__":
    main()
