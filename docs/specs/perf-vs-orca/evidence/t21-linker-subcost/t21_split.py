"""Reduce a T21-PROBE capture: per-layer linker subcost split.

Reads the `--profile-verbose` JSONL from a probe build and prints:
  1. per-layer rows for the tail layers (linker fuel + module elapsed),
  2. the nested scope tree (self vs total fuel) aggregated over all layers
     and over layers 0-2 only, from `profile_scopes` on the infill-linker
     `module_complete` events,
  3. the `T21-PROBE` counter/wall lines from `module_log`.

Usage: python t21_split.py <capture.jsonl> [--top N]
"""

import json
import sys
from collections import defaultdict

SCOPE_ORDER = [
    "t21::orchestrate_total",
    "t21::records_build",
    "t21::bucket_process",
    "t21::boundary_prep",
    "t21::locked_footprint",
    "t21::group_link",
    "t21::union_boundary",
    "t21::overlap_offset",
    "t21::path_reclip",
    "t21::graph_build",
    "t21::connect_infill",
    "t21::majority_owner",
    "t21::output_emit",
]


def main(path, top=12):
    per_layer = {}
    scope_tot = defaultdict(lambda: {"calls": 0, "self": 0, "total": 0})
    scope_tail = defaultdict(lambda: {"calls": 0, "self": 0, "total": 0})
    probe_lines = {}

    with open(path, encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            if line.startswith("T21-PROBE"):
                parts = dict(p.split("=", 1) for p in line.split()[1:])
                probe_lines[int(parts["layer"])] = {
                    k: int(v) for k, v in parts.items() if k != "layer"
                }
                continue
            if not line.startswith("{"):
                continue
            try:
                e = json.loads(line)
            except Exception:
                continue
            if e.get("event") == "module_log":
                msg = e.get("message", "")
                if msg.startswith("T21-PROBE"):
                    parts = dict(p.split("=", 1) for p in msg.split()[1:])
                    probe_lines[int(parts["layer"])] = {
                        k: int(v) for k, v in parts.items() if k != "layer"
                    }
                continue
            if (
                e.get("event") != "module_complete"
                or e.get("module_id") != "com.core.infill-linker"
            ):
                continue
            li = e.get("layer_index")
            d = e.get("profile_scopes")
            fuel = d["call_fuel"] if d else 0
            per_layer[li] = {"elapsed_ms": e.get("elapsed_ms"), "fuel": fuel}
            for row in (d or {}).get("scopes", []):
                for bucket in ("self", "total"):
                    scope_tot[row["scope"]][bucket] += row[f"{bucket}_fuel"]
                scope_tot[row["scope"]]["calls"] += row["calls"]
                if li in (0, 1, 2):
                    for bucket in ("self", "total"):
                        scope_tail[row["scope"]][bucket] += row[f"{bucket}_fuel"]
                    scope_tail[row["scope"]]["calls"] += row["calls"]

    tail = [li for li in (0, 1, 2)]
    total_fuel = sum(v["fuel"] for v in per_layer.values())
    tail_fuel = sum(per_layer[li]["fuel"] for li in tail if li in per_layer)

    print(f"layers measured: {len(per_layer)}")
    print(f"linker fuel total: {total_fuel:,}")
    print(
        f"layers 0-2 fuel:   {tail_fuel:,} ({100 * tail_fuel / max(total_fuel, 1):.1f}% of linker)"
    )
    print()
    print("top layers by linker fuel:")
    print(f"  {'layer':>5} {'elapsed_ms':>10} {'fuel':>16} {'fn_us':>9} {'orch_us':>9}")
    for li, v in sorted(per_layer.items(), key=lambda kv: -kv[1]["fuel"])[:top]:
        p = probe_lines.get(li, {})
        print(
            f"  {li:>5} {v['elapsed_ms'] or 0:>10} {v['fuel']:>16,} "
            f"{p.get('fn_us', -1):>9} {p.get('orch_us', -1):>9}"
        )
    print()
    print("scope tree, all layers (fuel; calls):")
    print(
        f"  {'scope':28} {'calls':>7} {'self':>16} {'total':>16} {'self%':>7} {'total%':>7}"
    )
    for scope in SCOPE_ORDER:
        t = scope_tot.get(scope)
        if not t:
            continue
        print(
            f"  {scope:28} {t['calls']:>7} {t['self']:>16,} {t['total']:>16,} "
            f"{100 * t['self'] / max(total_fuel, 1):>6.2f}% {100 * t['total'] / max(total_fuel, 1):>6.2f}%"
        )
    print()
    print("scope tree, layers 0-2 only:")
    print(
        f"  {'scope':28} {'calls':>7} {'self':>16} {'total':>16} {'self%':>7} {'total%':>7}"
    )
    for scope in SCOPE_ORDER:
        t = scope_tail.get(scope)
        if not t:
            continue
        print(
            f"  {scope:28} {t['calls']:>7} {t['self']:>16,} {t['total']:>16,} "
            f"{100 * t['self'] / max(tail_fuel, 1):>6.2f}% {100 * t['total'] / max(tail_fuel, 1):>6.2f}%"
        )
    print()
    print("probe counters / guest-call wall (layers 0-2):")
    keys = [
        "fn_us",
        "orch_us",
        "regions",
        "matched",
        "roles",
        "selected",
        "locked",
        "clip_calls",
        "clip_in_pts",
        "clip_bnd_pts",
        "clip_out",
        "fallbacks",
        "endpoints",
        "cands",
        "merges",
        "rings",
        "bpts",
        "unions",
        "rgroups",
        "owners",
        "out",
    ]
    for li in tail:
        p = probe_lines.get(li)
        if not p:
            continue
        print(f"  layer {li}: " + " ".join(f"{k}={p.get(k)}" for k in keys if k in p))
    # median comparison over everything else
    others = [p for li, p in probe_lines.items() if li not in (0, 1, 2)]
    if others:

        def med(key):
            vals = sorted(p[key] for p in others if key in p)
            return vals[len(vals) // 2] if vals else None

        print(
            "  median over layers 3-239: "
            + " ".join(f"{k}={med(k)}" for k in keys if k in others[0])
        )


if __name__ == "__main__":
    args = sys.argv[1:]
    top = 12
    if "--top" in args:
        i = args.index("--top")
        top = int(args[i + 1])
        del args[i : i + 2]
    main(args[0], top)
