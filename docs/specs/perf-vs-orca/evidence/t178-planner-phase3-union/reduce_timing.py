#!/usr/bin/env python3
"""Reduce raw t178 Phase-3 production timing JSONLs into reduced/prod_timing.json."""

from __future__ import annotations

import json
import statistics
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
RAW = REPO / "target" / "t178-capture"
REDUCED = HERE / "reduced"
ROLE_NAMES = {0: "ROLE_BODY", 1: "ROLE_ROOF", 2: "ROLE_BASE", 3: "ROLE_FLOOR"}


def load_jsonl(path: Path) -> list[dict]:
    out = []
    with path.open("r", encoding="utf-8") as fh:
        for line in fh:
            s = line.strip()
            if s:
                out.append(json.loads(s))
    return out


def summarize_pass(batches: list[dict]) -> dict:
    batch_wall_ns = 0
    clip_thread_ns = 0
    total_thread_ns = 0
    sum_batch_max_clip_ns = 0
    sum_batch_max_total_ns = 0
    singletons_skipped = 0
    calls = 0
    in_polys = 0
    in_holes = 0
    in_verts = 0
    out_polys = 0
    out_holes = 0
    out_verts = 0
    kept_polys = 0
    clip_polys = 0
    by_role: dict[int, dict] = {}
    batch_rows = []
    req_rows = []

    for b in batches:
        bw = b["batch_wall_ns"]
        batch_wall_ns += bw
        singletons_skipped += b.get("singletons_skipped", 0)
        b_clip_sum = 0
        b_tot_sum = 0
        b_max_clip = 0
        b_max_tot = 0
        b_verts = 0
        for r in b["requests"]:
            calls += 1
            in_polys += r["in_polys"]
            in_holes += r["in_holes"]
            in_verts += r["in_verts"]
            out_polys += r["out_polys"]
            out_holes += r["out_holes"]
            out_verts += r["out_verts"]
            kept_polys += r["kept_count"]
            clip_polys += r["clip_count"]
            cn = r["clip_ns"]
            tn = r["total_req_ns"]
            clip_thread_ns += cn
            total_thread_ns += tn
            b_clip_sum += cn
            b_tot_sum += tn
            b_max_clip = max(b_max_clip, cn)
            b_max_tot = max(b_max_tot, tn)
            b_verts += r["in_verts"]

            role = r["role"]
            rc = by_role.setdefault(
                role,
                {
                    "role": role,
                    "role_name": ROLE_NAMES.get(role, f"ROLE_{role}"),
                    "calls": 0,
                    "in_polys": 0,
                    "in_holes": 0,
                    "in_verts": 0,
                    "out_polys": 0,
                    "out_holes": 0,
                    "out_verts": 0,
                    "kept_polys": 0,
                    "clip_polys": 0,
                    "clip_ns": 0,
                    "total_ns": 0,
                    "max_clip_ns": 0,
                    "max_in_polys": 0,
                    "max_in_verts": 0,
                },
            )
            rc["calls"] += 1
            rc["in_polys"] += r["in_polys"]
            rc["in_holes"] += r["in_holes"]
            rc["in_verts"] += r["in_verts"]
            rc["out_polys"] += r["out_polys"]
            rc["out_holes"] += r["out_holes"]
            rc["out_verts"] += r["out_verts"]
            rc["kept_polys"] += r["kept_count"]
            rc["clip_polys"] += r["clip_count"]
            rc["clip_ns"] += cn
            rc["total_ns"] += tn
            rc["max_clip_ns"] = max(rc["max_clip_ns"], cn)
            rc["max_in_polys"] = max(rc["max_in_polys"], r["in_polys"])
            rc["max_in_verts"] = max(rc["max_in_verts"], r["in_verts"])

            req_rows.append(
                {
                    "batch_idx": b["batch_idx"],
                    "req_idx": r["req_idx"],
                    "layer": r["layer"],
                    "role": role,
                    "role_name": ROLE_NAMES.get(role, f"ROLE_{role}"),
                    "in_polys": r["in_polys"],
                    "in_verts": r["in_verts"],
                    "out_polys": r["out_polys"],
                    "out_verts": r["out_verts"],
                    "clip_ms": round(cn / 1e6, 3),
                    "total_ms": round(tn / 1e6, 3),
                }
            )
        sum_batch_max_clip_ns += b_max_clip
        sum_batch_max_total_ns += b_max_tot
        batch_rows.append(
            {
                "batch_idx": b["batch_idx"],
                "req_count": len(b["requests"]),
                "in_verts": b_verts,
                "batch_wall_ms": round(bw / 1e6, 3),
                "clip_thread_ms": round(b_clip_sum / 1e6, 3),
                "max_req_clip_ms": round(b_max_clip / 1e6, 3),
                "eff_par": round(b_clip_sum / max(bw, 1), 2),
                "max_share_of_wall_pct": round(100.0 * b_max_clip / max(bw, 1), 2),
            }
        )

    req_rows.sort(key=lambda x: x["clip_ms"], reverse=True)
    clip_ms_list = sorted(x["clip_ms"] for x in req_rows)
    n = len(clip_ms_list)

    def pct(p: float) -> float:
        if n == 0:
            return 0.0
        idx = min(n - 1, max(0, int(round((n - 1) * p))))
        return clip_ms_list[idx]

    return {
        "batches": len(batches),
        "calls": calls,
        "singletons_skipped_pre_phase3": singletons_skipped,
        "in_polys": in_polys,
        "in_holes": in_holes,
        "in_verts": in_verts,
        "out_polys": out_polys,
        "out_holes": out_holes,
        "out_verts": out_verts,
        "kept_polys": kept_polys,
        "clip_polys": clip_polys,
        "batch_wall_ms": round(batch_wall_ns / 1e6, 3),
        "clip_thread_ms": round(clip_thread_ns / 1e6, 3),
        "total_thread_ms": round(total_thread_ns / 1e6, 3),
        "conversion_overhead_pct": round(
            100.0 * (total_thread_ns - clip_thread_ns) / max(total_thread_ns, 1), 3
        ),
        "eff_par_clip": round(clip_thread_ns / max(batch_wall_ns, 1), 3),
        "eff_par_total": round(total_thread_ns / max(batch_wall_ns, 1), 3),
        "sum_batch_max_clip_ms": round(sum_batch_max_clip_ns / 1e6, 3),
        "sum_batch_max_total_ms": round(sum_batch_max_total_ns / 1e6, 3),
        "max_req_share_of_wall_pct": round(
            100.0 * sum_batch_max_clip_ns / max(batch_wall_ns, 1), 2
        ),
        "ideal_12t_floor_ms": round((clip_thread_ns / 12.0) / 1e6, 3),
        "req_clip_ms_quantiles": {
            "min": clip_ms_list[0] if n else 0.0,
            "p50": pct(0.50),
            "p90": pct(0.90),
            "p95": pct(0.95),
            "p99": pct(0.99),
            "max": clip_ms_list[-1] if n else 0.0,
        },
        "by_role": [
            {
                **v,
                "clip_ms": round(v["clip_ns"] / 1e6, 3),
                "total_ms": round(v["total_ns"] / 1e6, 3),
                "max_clip_ms": round(v["max_clip_ns"] / 1e6, 3),
                "clip_thread_share_pct": round(
                    100.0 * v["clip_ns"] / max(clip_thread_ns, 1), 2
                ),
            }
            for _, v in sorted(by_role.items())
        ],
        "top_15_calls": req_rows[:15],
        "batch_rows": batch_rows,
    }


def main() -> int:
    REDUCED.mkdir(parents=True, exist_ok=True)
    capture_runs = json.loads((RAW / "capture_runs.json").read_text(encoding="utf-8"))
    out: dict = {
        "capture_meta": {
            "captured_utc": capture_runs["captured_utc"],
            "exe_sha256": capture_runs["exe_sha256"],
            "planner_wasm_sha256": capture_runs["planner_wasm_sha256"],
            "rayon_num_threads": capture_runs["rayon_num_threads"],
            "failures": capture_runs["failures"],
        },
        "runs": capture_runs["runs"],
        "cells": {},
    }

    for cell, n_passes in [
        ("benchy-classic-on", 3),
        ("base-classic-on", 3),
        ("benchy-arachne-on", 1),
        ("base-arachne-on", 1),
    ]:
        passes = []
        for p in range(1, n_passes + 1):
            path = RAW / f"{cell}.t{p}.phase3.jsonl"
            if path.exists():
                passes.append(summarize_pass(load_jsonl(path)))
        # Pick median pass by batch_wall_ms
        walls = [x["batch_wall_ms"] for x in passes]
        threads = [x["clip_thread_ms"] for x in passes]
        med_wall = statistics.median(walls)
        med_idx = min(range(len(passes)), key=lambda i: abs(passes[i]["batch_wall_ms"] - med_wall))
        out["cells"][cell] = {
            "passes_count": len(passes),
            "batch_wall_ms_runs": walls,
            "clip_thread_ms_runs": threads,
            "batch_wall_ms_median": round(med_wall, 3),
            "clip_thread_ms_median": round(statistics.median(threads), 3),
            "median_pass": passes[med_idx],
        }

    # Verify classic vs arachne structural identity on Phase-3 operands
    checks = {}
    for model in ["benchy", "base"]:
        c = out["cells"][f"{model}-classic-on"]["median_pass"]
        a = out["cells"][f"{model}-arachne-on"]["median_pass"]
        keys = [
            "batches",
            "calls",
            "singletons_skipped_pre_phase3",
            "in_polys",
            "in_holes",
            "in_verts",
            "out_polys",
            "out_holes",
            "out_verts",
            "kept_polys",
            "clip_polys",
        ]
        checks[model] = all(c[k] == a[k] for k in keys)
    out["classic_arachne_operand_identity"] = checks

    dest = REDUCED / "prod_timing.json"
    dest.write_text(json.dumps(out, indent=2) + "\n", encoding="utf-8")
    print(f"Wrote {dest}")
    for cell, info in out["cells"].items():
        mp = info["median_pass"]
        print(
            f"  {cell:20} calls={mp['calls']:3} in={mp['in_polys']:6}p/{mp['in_verts']:8}v "
            f"out={mp['out_polys']:4}p/{mp['out_verts']:7}v "
            f"wall={info['batch_wall_ms_median']:7.2f}ms thread={info['clip_thread_ms_median']:8.2f}ms "
            f"par={mp['eff_par_clip']:.2f}x max_req_sum={mp['sum_batch_max_clip_ms']:.2f}ms ({mp['max_req_share_of_wall_pct']:.1f}%)"
        )
    print("  classic_arachne_operand_identity:", checks)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
