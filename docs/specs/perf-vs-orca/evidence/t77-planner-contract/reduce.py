#!/usr/bin/env python3
"""Reduce the T77 tree-support-planner clip attribution capture.

Inputs (raw, gitignored):
  <raw>/clip-lines.tsv   PERF-T77-PROBE host records, one line per guest
                         `clip_polygons` call: op, site, subj_polys,
                         clip_polys, core_ns
  <raw>/stderr.jsonl     --instrument-stderr event stream, carrying
                         `PERF-T77-SITE` transition tags and module_complete
                         walls

Output: <out> summary JSON (committed as evidence).

The script only aggregates recorded samples; it never re-times anything.
"""

from __future__ import annotations

import argparse
import json
import statistics
from collections import defaultdict
from pathlib import Path

# Cost-share thresholds used by the tail analysis (nanoseconds).
TAIL_BUCKETS = (("ge_1ms", 1_000_000), ("ge_10ms", 10_000_000), ("ge_100ms", 100_000_000))


def parse_clip_lines(path: Path):
    rows = []
    with path.open("r", encoding="utf-8") as handle:
        for line in handle:
            parts = line.rstrip("\n").split("\t")
            if len(parts) != 5:
                continue
            op, site, subj, clip, ns = parts
            rows.append(
                {
                    "op": op,
                    "site": site,
                    "subj_polys": int(subj),
                    "clip_polys": int(clip),
                    "core_ns": int(ns),
                }
            )
    return rows


def parse_events(path: Path):
    module_walls = {}
    stage_walls = {}
    site_tags = []
    with path.open("r", encoding="utf-8") as handle:
        for line in handle:
            stripped = line.strip()
            if "PERF-T77-SITE " in stripped and not stripped.startswith("{"):
                site_tags.append(stripped.split("PERF-T77-SITE ", 1)[1])
                continue
            if not stripped.startswith("{"):
                continue
            try:
                event = json.loads(stripped)
            except json.JSONDecodeError:
                continue
            name = event.get("event")
            if name == "module_complete":
                module_walls[event.get("module_id", "?")] = event.get("elapsed_ms")
            elif name == "stage_complete":
                stage_walls[event.get("stage", "?")] = event.get("elapsed_ms")
    return module_walls, stage_walls, site_tags


def summarize_group(rows):
    if not rows:
        return None
    seconds = sorted(r["core_ns"] / 1e9 for r in rows)
    total = sum(seconds)
    return {
        "calls": len(rows),
        "core_seconds": round(total, 3),
        "median_ms": round(statistics.median(seconds) * 1000, 3),
        "p95_ms": round(seconds[min(len(seconds) - 1, int(len(seconds) * 0.95))] * 1000, 3),
        "max_ms": round(seconds[-1] * 1000, 3),
        "mean_subj_polys": round(statistics.fmean(r["subj_polys"] for r in rows), 2),
        "mean_clip_polys": round(statistics.fmean(r["clip_polys"] for r in rows), 2),
        "max_clip_polys": max(r["clip_polys"] for r in rows),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--raw", default="target/t77-probe2")
    parser.add_argument(
        "--out",
        default="docs/specs/perf-vs-orca/evidence/t77-planner-contract/summary.json",
    )
    args = parser.parse_args()

    raw = Path(args.raw)
    rows = parse_clip_lines(raw / "clip-lines.tsv")
    module_walls, stage_walls, site_tags = parse_events(raw / "stderr.jsonl")

    total_core = sum(r["core_ns"] for r in rows) / 1e9
    # The probe file is process-global: the guest modules that run after the
    # planner record untagged calls (their host context starts with no site).
    # They are excluded from every planner share below.
    planner_rows = [r for r in rows if r["site"]]
    planner_core = sum(r["core_ns"] for r in planner_rows) / 1e9

    # `build_roles` is the emit-pass role builder; these four sites are the
    # only clip_polygons call sites inside it.
    BUILD_ROLES_SITES = {"role_union", "carve_region", "set_diff", "cross_role_diff"}
    build_roles_core = (
        sum(r["core_ns"] for r in planner_rows if r["site"] in BUILD_ROLES_SITES) / 1e9
    )

    by_site = defaultdict(list)
    by_site_op = defaultdict(list)
    for row in rows:
        by_site[row["site"] or "(untagged)"].append(row)
        by_site_op[f"{row['site'] or '(untagged)'}/{row['op']}"].append(row)

    site_summary = {}
    for site, group in sorted(by_site.items(), key=lambda kv: -sum(r["core_ns"] for r in kv[1])):
        entry = summarize_group(group)
        if site == "(untagged)":
            entry["share_of_clip_core"] = (
                round(entry["core_seconds"] / total_core, 4) if total_core else None
            )
            entry["role"] = "non-planner modules recorded by the process-global probe"
        else:
            entry["share_of_planner_clip_core"] = (
                round(entry["core_seconds"] / planner_core, 4) if planner_core else None
            )
            entry["role"] = "planner"
        site_summary[site] = entry

    ordered = sorted(rows, key=lambda r: r["core_ns"], reverse=True)
    tail = {
        name: {
            "calls": sum(1 for r in ordered if r["core_ns"] >= threshold),
            "core_seconds": round(
                sum(r["core_ns"] for r in ordered if r["core_ns"] >= threshold) / 1e9, 3
            ),
        }
        for name, threshold in TAIL_BUCKETS
    }
    tail["share_of_core_in_top_100_calls"] = (
        round(sum(r["core_ns"] for r in ordered[:100]) / (total_core * 1e9), 4) if total_core else None
    )

    summary = {
        "probe": "PERF-T77-PROBE (temporary, reverted; no geometry change)",
        "cell": "ordinary base classic supports-on (12 rayon threads)",
        "output_sha256": "f60e5e9b8c0d836cf73170e645c18f9193b91d47e694d335cfb1d78032682319",
        "output_bytes": 43912792,
        "totals": {
            "clip_calls": len(rows),
            "clip_core_seconds": round(total_core, 3),
            "planner_clip_calls": len(planner_rows),
            "planner_clip_core_seconds": round(planner_core, 3),
            "planner_clip_share_of_module_wall": round(
                planner_core / (module_walls["com.core.tree-support-planner"] / 1000.0), 4
            )
            if module_walls.get("com.core.tree-support-planner")
            else None,
            "build_roles_clip_core_seconds": round(build_roles_core, 3),
            "build_roles_share_of_planner_clip_core": round(build_roles_core / planner_core, 4)
            if planner_core
            else None,
            "site_transitions": len(site_tags),
        },
        "by_site": site_summary,
        "by_site_and_op": {
            key: summarize_group(value)
            for key, value in sorted(
                by_site_op.items(), key=lambda kv: -sum(r["core_ns"] for r in kv[1])
            )
        },
        "tail": tail,
        "module_walls_ms": module_walls,
        "stage_walls_ms": stage_walls,
    }

    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")

    print(f"clip calls {len(rows)}  core {total_core:.3f} s  site transitions {len(site_tags)}")
    print(
        f"planner calls {len(planner_rows)}  core {planner_core:.3f} s  "
        f"build_roles {build_roles_core:.3f} s "
        f"({build_roles_core / planner_core * 100:.2f} % of planner clip core)"
    )
    for site, group in site_summary.items():
        share = group.get("share_of_planner_clip_core", group.get("share_of_clip_core"))
        print(
            f"  {site:<20} calls={group['calls']:>7} core={group['core_seconds']:>8.3f}s "
            f"share={share * 100:>5.1f}% median={group['median_ms']:>8.3f}ms "
            f"p95={group['p95_ms']:>8.3f}ms max={group['max_ms']:>8.3f}ms "
            f"mean_clip={group['mean_clip_polys']:>6} max_clip={group['max_clip_polys']}"
        )
    for name, bucket in tail.items():
        print(f"  tail {name}: {bucket}")
    print("planner module wall:", module_walls.get("com.core.tree-support-planner"), "ms")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
