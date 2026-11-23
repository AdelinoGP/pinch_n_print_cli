#!/usr/bin/env python3
"""t105 supports-on engine-style take -- verifier.

Re-derives every number the ticket claims from the committed reduced set
(`runs.csv` + `summary.json`, both in-repo) against ticket 103's PnP medians.
Independent implementation: it does not import `reduce.py`.

  python verify.py              # verify the committed set
  python verify.py --self-test  # negative controls: mutate, demand rejection

Exit 0 only when every claim holds.
"""

from __future__ import annotations

import copy
import csv
import json
import statistics
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
T103 = REPO / "docs/specs/perf-vs-orca/evidence/t103-orca-rebaseline/summary.json"

TOL_WALL = 0.005  # seconds, median recomputation
TOL_RATIO = 0.005  # ratio recomputation
TOL_PCT = 0.02  # percentage-point recomputation
ARM_STYLE = {"organic": "default", "strong": "tree_strong"}
CELLS = [
    "benchy/classic-on",
    "benchy/arachne-on",
    "base/classic-on",
    "base/arachne-on",
]


def load() -> dict:
    runs = list(csv.DictReader((HERE / "runs.csv").open(encoding="utf-8")))
    for r in runs:
        r["wall_s"] = float(r["wall_s"])
        r["cpu_s"] = float(r["cpu_s"])
        r["cpu_wall"] = float(r["cpu_wall"])
        r["round"] = int(r["round"])
        r["gcode_bytes"] = int(r["gcode_bytes"])
        r["warmup"] = str(r["warmup"]).lower() == "true"
        r["support_sections"] = int(r["support_sections"])
        r["support_interface_sections"] = int(r["support_interface_sections"])
    return {
        "runs": runs,
        "summary": json.loads((HERE / "summary.json").read_text(encoding="utf-8")),
        "t103": json.loads(T103.read_text(encoding="utf-8")),
    }


def med(xs: list[float]) -> float:
    return statistics.median(xs)


def check(b: dict) -> list[tuple[str, bool, str]]:
    out: list[tuple[str, bool, str]] = []

    def add(name: str, ok: bool, detail: str = "") -> None:
        out.append((name, bool(ok), detail))

    runs, summary, t103 = b["runs"], b["summary"], b["t103"]

    add(
        "four supports-on cells are in the summary",
        sorted(summary["cells"]) == sorted(CELLS),
        str(sorted(summary["cells"])),
    )
    add(
        "no supports-off row entered the take",
        all(r["cell"].endswith("-on") for r in runs),
        str(sorted({r["cell"] for r in runs})),
    )

    measured = [r for r in runs if not r["warmup"]]
    warm = [r for r in runs if r["warmup"]]

    for fixture in ("benchy", "base"):
        for cell in ("classic-on", "arachne-on"):
            key = fixture + "/" + cell
            c = summary["cells"].get(key, {})
            for arm in ("organic", "strong"):
                group = [
                    r
                    for r in measured
                    if r["fixture"] == fixture and r["cell"] == cell and r["arm"] == arm
                ]
                expected_group = [
                    r
                    for r in warm
                    if r["fixture"] == fixture and r["cell"] == cell and r["arm"] == arm
                ]
                a = c.get("arms", {}).get(arm, {})
                add(
                    f"{key}/{arm}: 4 measured + 1 warmup",
                    len(group) == 4 and len(expected_group) == 1,
                    f"measured={len(group)} warm={len(expected_group)}",
                )
                add(
                    f"{key}/{arm}: every row resolved {ARM_STYLE[arm]}",
                    all(r["support_style"] == ARM_STYLE[arm] for r in group),
                    str(sorted({r["support_style"] for r in group})),
                )

                if not group:
                    continue
                ratios = [r["cpu_wall"] for r in group]
                kept = [r for r in group if r["cpu_wall"] >= 0.75 * max(ratios)]
                recomputed = med([r["wall_s"] for r in kept])
                add(
                    f"{key}/{arm}: median wall recomputes",
                    a.get("median_wall_s") is not None
                    and abs(a["median_wall_s"] - recomputed) <= TOL_WALL,
                    f"summary={a.get('median_wall_s')} recomputed={round(recomputed, 4)}",
                )
                add(
                    f"{key}/{arm}: retained sample count recomputes",
                    a.get("retained") == len(kept) and a.get("samples") == len(group),
                    f"retained={a.get('retained')}/{len(kept)} samples={a.get('samples')}/{len(group)}",
                )

                pnp = float(t103["cells"][key]["tools"]["pnp-ordinary"]["wall_s"])
                ratio = pnp / recomputed
                add(
                    f"{key}/{arm}: ratio vs ticket 103 recomputes",
                    abs(a.get("ratio_vs_pnp", -1.0) - ratio) <= TOL_RATIO,
                    f"summary={a.get('ratio_vs_pnp')} recomputed={round(ratio, 4)}",
                )
                cut = 100.0 * (1.0 - 1.5 * recomputed / pnp)
                add(
                    f"{key}/{arm}: cut-to-park recomputes",
                    abs(a.get("cut_to_park_pct", -999.0) - cut) <= TOL_PCT,
                    f"summary={a.get('cut_to_park_pct')} recomputed={round(cut, 2)}",
                )

            organic = c.get("arms", {}).get("organic", {})
            strong = c.get("arms", {}).get("strong", {})
            if organic.get("median_wall_s") and strong.get("median_wall_s"):
                delta = strong["median_wall_s"] - organic["median_wall_s"]
                pct = 100.0 * delta / organic["median_wall_s"]
                add(
                    f"{key}: style delta recomputes",
                    abs(c.get("style_wall_delta_s", -999.0) - delta) <= TOL_WALL
                    and abs(c.get("style_wall_delta_pct", -999.0) - pct) <= TOL_PCT,
                    f"summary={c.get('style_wall_delta_s')}/{c.get('style_wall_delta_pct')} "
                    f"recomputed={round(delta, 4)}/{round(pct, 2)}",
                )
                move = (
                    100.0
                    * (strong["ratio_vs_pnp"] - organic["ratio_vs_pnp"])
                    / organic["ratio_vs_pnp"]
                )
                add(
                    f"{key}: ratio move recomputes",
                    abs(c.get("ratio_move_pct", -999.0) - move) <= TOL_PCT,
                    f"summary={c.get('ratio_move_pct')} recomputed={round(move, 2)}",
                )

            # Paired per-round deltas, retained samples only (same starvation
            # rule as the group medians): pairing a starved sample would
            # attribute the stall to the style.
            kept_by_round: dict[str, dict[int, dict]] = {}
            for arm in ("organic", "strong"):
                group = [
                    r
                    for r in measured
                    if r["fixture"] == fixture and r["cell"] == cell and r["arm"] == arm
                ]
                top = max(r["cpu_wall"] for r in group) if group else 0.0
                kept_by_round[arm] = {
                    r["round"]: r for r in group if r["cpu_wall"] >= 0.75 * top
                }
            rounds_seen = sorted(
                {
                    r["round"]
                    for r in measured
                    if r["fixture"] == fixture and r["cell"] == cell
                }
            )
            paired = [
                rnd
                for rnd in rounds_seen
                if rnd in kept_by_round["organic"] and rnd in kept_by_round["strong"]
            ]
            excluded = [rnd for rnd in rounds_seen if rnd not in paired]
            deltas = [
                kept_by_round["strong"][rnd]["wall_s"]
                - kept_by_round["organic"][rnd]["wall_s"]
                for rnd in paired
            ]
            add(
                f"{key}: paired rounds reconcile",
                sorted(c.get("paired_rounds_used", [])) == paired
                and sorted(c.get("paired_rounds_excluded", [])) == excluded
                and len(paired) + len(excluded) == len(rounds_seen),
                f"used={paired} excluded={excluded}",
            )
            if deltas:
                add(
                    f"{key}: paired delta median recomputes",
                    abs(c.get("paired_round_delta_median_s", -999.0) - med(deltas))
                    <= TOL_WALL,
                    f"summary={c.get('paired_round_delta_median_s')} "
                    f"recomputed={round(med(deltas), 4)}",
                )

            # Non-vacuity: the two arms must be different jobs.
            organic_shas = set(organic.get("sha256", []))
            strong_shas = set(strong.get("sha256", []))
            add(
                f"{key}: arms are non-vacuous (distinct output SHA)",
                bool(organic_shas)
                and bool(strong_shas)
                and organic_shas.isdisjoint(strong_shas),
                f"organic={len(organic_shas)} strong={len(strong_shas)}",
            )
            add(
                f"{key}: support sections and bytes disclosed",
                len(organic.get("support_sections", [])) >= 1
                and len(strong.get("support_sections", [])) >= 1
                and len(organic.get("gcode_bytes", [])) >= 1
                and len(strong.get("gcode_bytes", [])) >= 1,
                f"organic={organic.get('support_sections')} strong={strong.get('support_sections')}",
            )

    return out


def run(b: dict) -> int:
    rows = check(b)
    failed = [r for r in rows if not r[1]]
    for name, ok, detail in rows:
        print(
            ("PASS " if ok else "FAIL ")
            + name
            + ("  [" + detail + "]" if detail else "")
        )
    print(f"{len(rows) - len(failed)}/{len(rows)} claims hold")
    return 0 if not failed else 1


def self_test(base: dict) -> int:
    controls: list[tuple[str, dict]] = []

    def mutate(fn) -> dict:
        b = copy.deepcopy(base)
        fn(b)
        return b

    def tamper_wall(b: dict) -> None:
        for r in b["runs"]:
            if not r["warmup"] and r["fixture"] == "base" and r["arm"] == "strong":
                r["wall_s"] = round(r["wall_s"] * 1.05, 4)

    def flip_style(b: dict) -> None:
        r = next(x for x in b["runs"] if not x["warmup"] and x["arm"] == "strong")
        r["support_style"] = "default"

    def same_output(b: dict) -> None:
        cell = b["summary"]["cells"]["base/classic-on"]
        cell["arms"]["strong"]["sha256"] = cell["arms"]["organic"]["sha256"]

    def drop_row(b: dict) -> None:
        warm = [x for x in b["runs"] if x["warmup"]]
        first = next(x for x in b["runs"] if not x["warmup"])
        b["runs"] = warm + [first]

    def tamper_pnp(b: dict) -> None:
        b["t103"]["cells"]["base/arachne-on"]["tools"]["pnp-ordinary"]["wall_s"] *= 1.05

    def tamper_delta(b: dict) -> None:
        b["summary"]["cells"]["benchy/classic-on"]["style_wall_delta_pct"] += 5.0

    controls = [
        ("tampered wall", mutate(tamper_wall)),
        ("arm resolved the wrong style", mutate(flip_style)),
        ("arms share one output", mutate(same_output)),
        ("measured rows dropped", mutate(drop_row)),
        ("ticket 103 PnP median moved", mutate(tamper_pnp)),
        ("summary delta arithmetic moved", mutate(tamper_delta)),
    ]

    rejected = 0
    for name, b in controls:
        rows = check(b)
        bad = [r for r in rows if not r[1]]
        ok = bool(bad)
        print(("REJECTED " if ok else "MISSED   ") + name)
        if ok:
            rejected += 1
    print(f"{rejected}/{len(controls)} negative controls rejected")
    return 0 if rejected == len(controls) else 1


def main(argv: list[str]) -> int:
    base = load()
    if "--self-test" in argv:
        return self_test(base)
    return run(base)


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
