#!/usr/bin/env python3
"""t177 post-t175 attribution -- reduce raw instrumented event streams.

Reads `<raw>/<cell>.pass<N>.jsonl` (stderr of `pnp_cli slice
--instrument-stderr`) plus the matching `.gcode`, and writes the committed
reduced set `reduced/instrumented.json`.

Definitions match t154 / t164 / t171 unchanged so splits are comparable:
- the four phase brackets (`prepass`, `per_layer`, `postpass`, `validation`)
  and the outside-phases residual, which reconcile with `slice_complete`;
- prepass and postpass stage brackets; per-layer stage thread sums
  (thread-accumulated, never wall);
- per-module thread sums, call counts, busy spans, per-module interval unions,
  and call-cost quantiles per phase;
- the module-busy union per phase and host gap (`phase wall - union`);
- validation fields, code-1200 diagnostic count, and G-code SHA-256 + bytes.

Usage:
  python reduce_instrumented.py [--raw <dir>] [--out reduced/instrumented.json]
"""

from __future__ import annotations

import argparse
import collections
import hashlib
import json
import statistics
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]


def load_events(path: Path) -> tuple[list[dict], int]:
    rows = []
    code_1200 = 0
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        if (
            '"code":1200' in line
            or '"code": 1200' in line
            or "exact-Z occupancy" in line
        ):
            code_1200 += 1
        s = line.strip()
        if not s.startswith("{"):
            continue
        try:
            rows.append(json.loads(s))
        except json.JSONDecodeError:
            continue
    return rows, code_1200


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def union_ms(intervals: list[tuple[int, int]]) -> int:
    if not intervals:
        return 0
    intervals = sorted(intervals)
    total = 0
    cur_s, cur_e = intervals[0]
    for s, e in intervals[1:]:
        if s > cur_e:
            total += cur_e - cur_s
            cur_s, cur_e = s, e
        else:
            cur_e = max(cur_e, e)
    total += cur_e - cur_s
    return total


def pair_modules(rows: list[dict]) -> tuple[dict, dict, dict]:
    """Return (per-phase per-module rows, per-phase interval lists, per-phase per-module intervals)."""
    pending: dict[tuple[str, str], collections.deque] = collections.defaultdict(
        collections.deque
    )
    by_phase: dict[str, dict[str, dict]] = collections.defaultdict(dict)
    intervals: dict[str, list[tuple[int, int]]] = collections.defaultdict(list)
    mod_intervals: dict[str, dict[str, list[tuple[int, int]]]] = (
        collections.defaultdict(lambda: collections.defaultdict(list))
    )
    for r in rows:
        ev = r.get("event")
        phase = r.get("phase") or "unknown"
        module_id = r.get("module_id")
        if not module_id:
            continue
        key = (phase, module_id)
        if ev == "module_start":
            pending[key].append(r.get("timestamp_ms"))
        elif ev == "module_complete":
            rec = by_phase[phase].setdefault(
                module_id,
                {
                    "calls": 0,
                    "thread_sum_ms": 0,
                    "spans_ms": 0,
                    "elapsed_ms": [],
                    "max_layer_index": None,
                },
            )
            rec["calls"] += 1
            elapsed = r.get("elapsed_ms") or 0
            rec["thread_sum_ms"] += elapsed
            rec["elapsed_ms"].append(elapsed)
            layer_index = r.get("layer_index")
            if layer_index is not None and elapsed == max(rec["elapsed_ms"]):
                rec["max_layer_index"] = layer_index
            start = pending[key].popleft() if pending[key] else None
            end = r.get("timestamp_ms")
            if start is not None and end is not None:
                rec["spans_ms"] += max(0, end - start)
                if end >= start:
                    intervals[phase].append((start, end))
                    mod_intervals[phase][module_id].append((start, end))
    return by_phase, intervals, mod_intervals


def quantile(values: list[int], q: float) -> int:
    if not values:
        return 0
    ordered = sorted(values)
    idx = min(len(ordered) - 1, int(round(q * (len(ordered) - 1))))
    return ordered[idx]


def first_in_stage(rows: list[dict]) -> dict:
    """Credit each (phase, layer, stage) group's earliest module completion."""
    groups: dict[tuple, list] = collections.defaultdict(list)
    for r in rows:
        if r.get("event") != "module_complete" or not r.get("module_id"):
            continue
        layer_index = r.get("layer_index")
        if layer_index is None:
            continue
        groups[(r.get("phase"), layer_index, r.get("stage"))].append(
            (r.get("timestamp_ms"), r.get("module_id"), r.get("elapsed_ms") or 0)
        )
    out: dict[str, dict[str, dict]] = collections.defaultdict(dict)
    for (phase, _layer, _stage), values in groups.items():
        values.sort()
        for _ts, module_id, elapsed in values:
            rec = out.setdefault(phase, {}).setdefault(
                module_id, {"groups": 0, "first_thread_sum_ms": 0}
            )
            rec["groups"] += 1
            rec["first_thread_sum_ms"] += elapsed
            break
    return out


def module_stats(by_phase: dict, mod_intervals: dict) -> dict:
    """Per-phase, per-module call-cost statistics (ms)."""
    out: dict[str, dict[str, dict]] = {}
    for phase, mods in by_phase.items():
        out[phase] = {}
        for mid, rec in mods.items():
            values = rec["elapsed_ms"]
            out[phase][mid] = {
                "calls": rec["calls"],
                "thread_sum_ms": rec["thread_sum_ms"],
                "spans_ms": rec["spans_ms"],
                "busy_union_ms": union_ms(mod_intervals[phase].get(mid, [])),
                "max_ms": max(values) if values else 0,
                "max_layer_index": rec["max_layer_index"],
                "p50_ms": quantile(values, 0.50),
                "p90_ms": quantile(values, 0.90),
                "p99_ms": quantile(values, 0.99),
                "calls_gt_10ms": sum(1 for v in values if v > 10),
                "calls_gt_50ms": sum(1 for v in values if v > 50),
            }
    return out


def reduce_pass(raw: Path, gcode: Path) -> dict:
    rows, code_1200 = load_events(raw)
    phase_wall: dict[str, int] = {}
    for r in rows:
        if r.get("event") == "phase_complete":
            phase_wall[r["phase"]] = r.get("elapsed_ms") or 0
    complete = next(r for r in rows if r.get("event") == "slice_complete")
    stats = next(r for r in rows if r.get("event") == "slice_stats")

    stage_ms: dict[str, dict[str, int]] = collections.defaultdict(dict)
    for r in rows:
        if r.get("event") != "stage_complete" or not r.get("stage"):
            continue
        phase = r["phase"]
        stage_ms[phase][r["stage"]] = stage_ms[phase].get(r["stage"], 0) + (
            r.get("elapsed_ms") or 0
        )

    by_phase, intervals, mod_intervals = pair_modules(rows)
    call_stats = module_stats(by_phase, mod_intervals)
    first_module = first_in_stage(rows)
    phase_meta = {}
    for phase, wall in phase_wall.items():
        busy = union_ms(intervals.get(phase, []))
        gap = wall - busy
        phase_meta[phase] = {
            "wall_ms": wall,
            "module_busy_union_ms": busy,
            "host_gap_ms": gap,
            "host_gap_fraction": round(gap / wall, 4) if wall else None,
        }

    inner = sum(v for v in phase_wall.values() if v)

    # Exact single-layer straggler tail in per_layer (11/12 cores idle waiting on 1 layer).
    pl_start_ts = next(
        (
            r.get("timestamp_ms")
            for r in rows
            if r.get("event") == "phase_start" and r.get("phase") == "per_layer"
        ),
        None,
    )
    layer_end_ts: dict[int, int] = {}
    layer_mod_max: dict[int, tuple[int, str]] = {}
    for r in rows:
        if r.get("event") != "module_complete" or r.get("phase") != "per_layer":
            continue
        li = r.get("layer_index")
        ts = r.get("timestamp_ms")
        if li is None or ts is None:
            continue
        if ts > layer_end_ts.get(li, -1):
            layer_end_ts[li] = ts
        el = r.get("elapsed_ms") or 0
        prev = layer_mod_max.get(li, (-1, ""))
        if el > prev[0]:
            layer_mod_max[li] = (el, r.get("module_id") or "")
    ranked_layers = sorted(layer_end_ts.items(), key=lambda kv: -kv[1])
    if len(ranked_layers) >= 2 and pl_start_ts is not None:
        last_li, last_ts = ranked_layers[0]
        second_li, second_ts = ranked_layers[1]
        top_el, top_mid = layer_mod_max.get(last_li, (0, ""))
        per_layer_tail = {
            "last_layer_index": last_li,
            "last_layer_end_rel_ms": last_ts - pl_start_ts,
            "second_last_layer_index": second_li,
            "second_last_layer_end_rel_ms": second_ts - pl_start_ts,
            "single_layer_straggler_tail_ms": max(0, last_ts - second_ts),
            "last_layer_top_module": top_mid,
            "last_layer_top_module_ms": top_el,
        }
    else:
        per_layer_tail = {
            "last_layer_index": None,
            "last_layer_end_rel_ms": 0,
            "second_last_layer_index": None,
            "second_last_layer_end_rel_ms": 0,
            "single_layer_straggler_tail_ms": 0,
            "last_layer_top_module": "",
            "last_layer_top_module_ms": 0,
        }

    # Exact post-planner host tail inside PrePass::SupportGeometry
    # (stage_complete.timestamp_ms - tree-support-planner module_complete.timestamp_ms).
    sg_planner_end_ts = None
    sg_stage_end_ts = None
    for r in rows:
        if r.get("stage") != "PrePass::SupportGeometry":
            continue
        if (
            r.get("event") == "module_complete"
            and r.get("module_id") == "com.core.tree-support-planner"
        ):
            sg_planner_end_ts = r.get("timestamp_ms")
        elif r.get("event") == "stage_complete" and sg_planner_end_ts is not None:
            sg_stage_end_ts = r.get("timestamp_ms")
    sg_post_planner_tail_ms = (
        max(0, sg_stage_end_ts - sg_planner_end_ts)
        if sg_planner_end_ts is not None and sg_stage_end_ts is not None
        else 0
    )

    return {
        "raw_events": len(rows),
        "slice_complete_ms": complete["elapsed_ms"],
        "phases_ms": phase_wall,
        "outside_phases_ms": complete["elapsed_ms"] - inner,
        "phase_detail": phase_meta,
        "per_layer_tail": per_layer_tail,
        "support_geometry_post_planner_tail_ms": sg_post_planner_tail_ms,
        "layer_count": stats["layer_count"],
        "degraded": complete.get("degraded"),
        "fatal_error_count": complete.get("fatal_error_count"),
        "non_fatal_error_count": complete.get("non_fatal_error_count"),
        "code_1200_count": code_1200,
        "prepass_stages_ms": dict(
            sorted(stage_ms["prepass"].items(), key=lambda kv: -kv[1])
        ),
        "per_layer_stages_thread_sum_ms": dict(
            sorted(stage_ms["per_layer"].items(), key=lambda kv: -kv[1])
        ),
        "postpass_stages_ms": dict(
            sorted(stage_ms["postpass"].items(), key=lambda kv: -kv[1])
        ),
        "module_thread_sum_ms": {
            phase: {
                mid: rec["thread_sum_ms"]
                for mid, rec in sorted(
                    mods.items(), key=lambda kv: -kv[1]["thread_sum_ms"]
                )
            }
            for phase, mods in by_phase.items()
        },
        "module_calls": {
            phase: {mid: rec["calls"] for mid, rec in mods.items()}
            for phase, mods in by_phase.items()
        },
        "module_spans_ms": {
            phase: {
                mid: rec["spans_ms"]
                for mid, rec in sorted(mods.items(), key=lambda kv: -kv[1]["spans_ms"])
            }
            for phase, mods in by_phase.items()
        },
        "module_call_stats": call_stats,
        "module_first_in_stage": first_module,
        "gcode_sha256": sha256(gcode),
        "gcode_bytes": gcode.stat().st_size,
    }


def shares_of(pass_row: dict) -> dict[str, float]:
    total = pass_row["slice_complete_ms"]
    out = {name: ms / total for name, ms in pass_row["phases_ms"].items()}
    out["outside"] = pass_row["outside_phases_ms"] / total
    return out


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--raw", default=str(REPO / "target" / "t177-instrumented"))
    ap.add_argument("--out", default=str(HERE / "reduced" / "instrumented.json"))
    a = ap.parse_args()

    raw_dir = Path(a.raw)
    by_cell: dict[str, dict[int, dict]] = collections.defaultdict(dict)
    for raw in sorted(raw_dir.glob("*.jsonl")):
        stem = raw.stem  # <cell>.pass<N>
        if ".pass" not in stem:
            raise SystemExit(f"unexpected raw stream name (want <cell>.pass<N>): {raw}")
        cell, pass_tag = stem.rsplit(".pass", 1)
        gcode = raw_dir / f"{stem}.gcode"
        if not gcode.exists():
            raise SystemExit(f"missing gcode for {stem}: {gcode}")
        by_cell[cell][int(pass_tag)] = reduce_pass(raw, gcode)

    cells: dict[str, dict] = {}
    for cell, passes in sorted(by_cell.items()):
        ordered = [passes[k] for k in sorted(passes)]
        share_rows = [shares_of(p) for p in ordered]
        phase_names = sorted(share_rows[0])
        median_shares = {
            name: round(statistics.median(r[name] for r in share_rows), 6)
            for name in phase_names
        }
        max_delta_pp = 0.0
        for name in phase_names:
            values = [r[name] for r in share_rows]
            max_delta_pp = max(max_delta_pp, 100.0 * (max(values) - min(values)))
        cells[cell] = {
            "passes": ordered,
            "pass_shares": [{k: round(v, 6) for k, v in r.items()} for r in share_rows],
            "median": {
                "slice_complete_ms": int(
                    statistics.median(p["slice_complete_ms"] for p in ordered)
                ),
                "layer_count": ordered[0]["layer_count"],
                "shares": median_shares,
                "outside_share": median_shares["outside"],
                "gcode_sha256": ordered[0]["gcode_sha256"],
                "gcode_bytes": ordered[0]["gcode_bytes"],
            },
            "repeat_count": len(ordered),
            "repeat_max_share_delta_pp": round(max_delta_pp, 2),
        }

    out = {
        "schema": 1,
        "note": (
            "Reduced by reduce_instrumented.py from gitignored raw event streams "
            "in target/t177-instrumented/. Phase and stage brackets are "
            "instrumented attribution only; per-layer module and stage sums are "
            "thread-accumulated and never wall claims. The `median` block is the "
            "per-phase median share across this cell's passes."
        ),
        "cells": cells,
    }
    out_path = Path(a.out)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(out, indent=1) + "\n", encoding="utf-8")
    print(f"wrote {a.out}: {len(cells)} cells")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
