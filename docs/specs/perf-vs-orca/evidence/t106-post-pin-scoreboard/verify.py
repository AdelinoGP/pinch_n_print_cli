#!/usr/bin/env python3
"""t106 post-pin supports-on scoreboard -- verifier.

Re-derives every number the take claims from the committed reduced set
(`runs.csv` + `summary.json`) plus ticket 103's PnP ordinary medians and the
ticket-105 style deltas. Independent implementation: it does not import
`reduce.py`, and it never reads the gitignored raw tree except through ticket
103's own committed CSVs.

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
EVIDENCE = REPO / "docs/specs/perf-vs-orca/evidence"
T103 = EVIDENCE / "t103-orca-rebaseline/summary.json"
T103_ROWS = EVIDENCE / "t103-orca-rebaseline"
T105 = EVIDENCE / "t105-tree-strong/summary.json"

TOL_WALL = 0.005  # seconds
TOL_RATIO = 0.005
TOL_PCT = 0.02  # percentage points
PIN_STYLE = "tree_strong"
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
    # Ticket 103's canonical ledger files (its own analyze.py's list); its
    # -r2/-r3/-r4 siblings are extra takes the summary does not use.
    ledger = [
        "rows-benchy.csv",
        "rows-base-classic-off.csv",
        "rows-base-arachne-off.csv",
        "rows-base-classic-on.csv",
        "rows-base-arachne-on.csv",
    ]
    groups: dict[str, list[dict]] = {}
    for name in ledger:
        with (T103_ROWS / name).open(encoding="utf-8-sig") as fh:
            for r in csv.DictReader(fh):
                if r["tool"] != "orca" or r["supports"] != "on":
                    continue
                if str(r["warmup"]) != "0":
                    continue
                key = r["fixture"] + "/" + r["cell"]
                groups.setdefault(key, []).append(
                    {
                        "wall_s": float(r["wall_seconds"]),
                        "cpu_wall": float(r["cpu_wall_ratio"]),
                        "gcode_bytes": int(r["gcode_bytes"]),
                        "sha256": r["sha256_of_gcode"],
                    }
                )
    prior: dict[str, list[dict]] = {}
    for key, group in groups.items():
        best = max(s["cpu_wall"] for s in group)
        prior[key] = [s for s in group if s["cpu_wall"] >= 0.75 * best]
    return {
        "runs": runs,
        "summary": json.loads((HERE / "summary.json").read_text(encoding="utf-8")),
        "t103": json.loads(T103.read_text(encoding="utf-8")),
        "t103_orca": prior,
        "t105": json.loads(T105.read_text(encoding="utf-8")),
    }


def med(xs: list[float]) -> float:
    return statistics.median(xs)


def check(b: dict) -> list[tuple[str, bool, str]]:
    out: list[tuple[str, bool, str]] = []

    def add(name: str, ok: bool, detail: str = "") -> None:
        out.append((name, bool(ok), detail))

    runs, summary = b["runs"], b["summary"]
    measured = [r for r in runs if not r["warmup"]]
    warm = [r for r in runs if r["warmup"]]

    add(
        "the take pins exactly one style",
        summary.get("support_style_pin") == PIN_STYLE,
        str(summary.get("support_style_pin")),
    )
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

    for key in CELLS:
        fixture, cell = key.split("/")
        group = [r for r in measured if r["fixture"] == fixture and r["cell"] == cell]
        group.sort(key=lambda r: r["round"])
        warm_group = [r for r in warm if r["fixture"] == fixture and r["cell"] == cell]
        c = summary["cells"].get(key, {})

        add(
            f"{key}: 3 measured + 1 warmup",
            len(group) == 3 and len(warm_group) == 1,
            f"measured={len(group)} warm={len(warm_group)}",
        )
        add(
            f"{key}: every row resolved {PIN_STYLE}",
            all(r["support_style"] == PIN_STYLE for r in group + warm_group),
            str(sorted({r["support_style"] for r in group + warm_group})),
        )
        add(
            f"{key}: every row resolved enable_support 1",
            all(str(r["enable_support"]) == "1" for r in group + warm_group),
            str(sorted({r["enable_support"] for r in group + warm_group})),
        )
        if not group:
            continue

        # Same standing starvation rule as the reducer, reimplemented here.
        top = max(r["cpu_wall"] for r in group)
        kept = [r for r in group if r["cpu_wall"] >= 0.75 * top]
        dropped = [r["round"] for r in group if r["cpu_wall"] < 0.75 * top]
        walls = [r["wall_s"] for r in kept]
        recomputed = med(walls)

        add(
            f"{key}: retained sample count recomputes",
            c.get("retained") == len(kept) and c.get("samples") == len(group),
            f"retained={c.get('retained')}/{len(kept)} samples={c.get('samples')}/{len(group)}",
        )
        add(
            f"{key}: excluded rounds recompute",
            sorted(c.get("excluded_rounds", [])) == sorted(dropped),
            f"summary={c.get('excluded_rounds')} recomputed={dropped}",
        )
        add(
            f"{key}: median wall recomputes",
            c.get("median_wall_s") is not None
            and abs(c["median_wall_s"] - recomputed) <= TOL_WALL,
            f"summary={c.get('median_wall_s')} recomputed={round(recomputed, 4)}",
        )
        add(
            f"{key}: median cpu recomputes",
            abs(c.get("median_cpu_s", -999.0) - med([r["cpu_s"] for r in kept]))
            <= TOL_WALL,
            f"summary={c.get('median_cpu_s')} "
            f"recomputed={round(med([r['cpu_s'] for r in kept]), 4)}",
        )
        add(
            f"{key}: measured spread recomputes",
            c.get("wall_range_s") == [round(min(walls), 4), round(max(walls), 4)]
            and abs(c.get("spread_s", -999.0) - (max(walls) - min(walls))) <= TOL_WALL,
            f"summary={c.get('wall_range_s')}/{c.get('spread_s')} "
            f"recomputed={[round(min(walls), 4), round(max(walls), 4)]}",
        )
        add(
            f"{key}: one output volume per cell (single byte count)",
            len(c.get("gcode_bytes", [])) == 1,
            f"bytes={c.get('gcode_bytes')} shas={len(c.get('sha256', []))}",
        )

        pnp = float(b["t103"]["cells"][key]["tools"]["pnp-ordinary"]["wall_s"])
        ratio = pnp / recomputed
        add(
            f"{key}: ratio vs ticket 103 PnP recomputes",
            abs(c.get("ratio_vs_pnp", -1.0) - ratio) <= TOL_RATIO,
            f"summary={c.get('ratio_vs_pnp')} recomputed={round(ratio, 4)}",
        )
        cut = 100.0 * (1.0 - 1.5 * recomputed / pnp)
        add(
            f"{key}: cut-to-park recomputes",
            abs(c.get("cut_to_park_pct", -999.0) - cut) <= TOL_PCT,
            f"summary={c.get('cut_to_park_pct')} recomputed={round(cut, 2)}",
        )

        prior = b["t103_orca"].get(key, [])
        prior_walls = [s["wall_s"] for s in prior]
        prior_med = med(prior_walls) if prior_walls else None
        add(
            f"{key}: ticket 103 Orca median recomputes from its committed rows",
            prior_med is not None
            and abs(c.get("t103_orca_wall_s", -999.0) - prior_med) <= TOL_WALL,
            f"summary={c.get('t103_orca_wall_s')} recomputed={round(prior_med, 4) if prior_med else None}",
        )
        add(
            f"{key}: ticket 103 Orca spread recomputes",
            abs(
                c.get("t103_orca_spread_s", -999.0)
                - (max(prior_walls) - min(prior_walls))
            )
            <= TOL_WALL,
            f"summary={c.get('t103_orca_spread_s')} "
            f"recomputed={round(max(prior_walls) - min(prior_walls), 4)}",
        )
        delta = recomputed - prior_med
        add(
            f"{key}: post-pin delta vs ticket 103 Orca recomputes",
            abs(c.get("post_pin_vs_t103_orca", {}).get("delta_s", -999.0) - delta)
            <= TOL_WALL
            and abs(
                c.get("post_pin_vs_t103_orca", {}).get("delta_pct", -999.0)
                - 100.0 * delta / prior_med
            )
            <= TOL_PCT,
            f"summary={c.get('post_pin_vs_t103_orca')} "
            f"recomputed={round(delta, 4)}/{round(100.0 * delta / prior_med, 2)}",
        )

        style_delta = float(b["t105"]["cells"][key]["style_wall_delta_pct"])
        predicted = prior_med * (1.0 + style_delta / 100.0)
        add(
            f"{key}: t105 transfer prediction recomputes",
            abs(c.get("t105_transfer", {}).get("predicted_wall_s", -999.0) - predicted)
            <= TOL_WALL
            and abs(
                100.0 * c["t105_transfer"]["style_wall_delta_pct"] / style_delta - 100.0
            )
            <= 0.5,
            f"summary={c.get('t105_transfer', {}).get('predicted_wall_s')} "
            f"recomputed={round(predicted, 4)}",
        )
        tol = max(max(walls) - min(walls), max(prior_walls) - min(prior_walls))
        agree = abs(recomputed - predicted) <= tol
        tc = c.get("transfer_check", {})
        add(
            f"{key}: transfer verdict recomputes",
            tc.get("agrees") == agree
            and abs(tc.get("tolerance_s", -999.0) - tol) <= TOL_WALL
            and abs(
                tc.get("measured_minus_predicted_s", -999.0) - (recomputed - predicted)
            )
            <= TOL_WALL,
            f"summary agrees={tc.get('agrees')} tol={tc.get('tolerance_s')} "
            f"recomputed agrees={agree} tol={round(tol, 4)}",
        )

        prior_bytes = {s["gcode_bytes"] for s in prior}
        measured_bytes = set(c.get("gcode_bytes", []))
        bytes_delta_pct = (
            100.0
            * (next(iter(measured_bytes)) - next(iter(prior_bytes)))
            / next(iter(prior_bytes))
            if len(prior_bytes) == 1 and len(measured_bytes) == 1
            else None
        )
        add(
            f"{key}: the pin changed the job (output volume moved >=5% vs t103 Orca)",
            bytes_delta_pct is not None
            and abs(c.get("output_bytes_delta_pct_vs_t103_orca", 0.0) - bytes_delta_pct)
            <= TOL_PCT
            and abs(bytes_delta_pct) >= 5.0
            and c.get("non_vacuous_vs_t103_orca") is True,
            f"summary={c.get('output_bytes_delta_pct_vs_t103_orca')} "
            f"recomputed={round(bytes_delta_pct, 2) if bytes_delta_pct is not None else None}",
        )
        add(
            f"{key}: support output disclosed",
            bool(c.get("support_sections"))
            and bool(c.get("support_interface_sections")),
            f"sections={c.get('support_sections')} "
            f"interface={c.get('support_interface_sections')}",
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
    def mutate(fn) -> dict:
        b = copy.deepcopy(base)
        fn(b)
        return b

    def tamper_wall(b: dict) -> None:
        r = next(x for x in b["runs"] if not x["warmup"] and x["fixture"] == "base")
        r["wall_s"] = round(r["wall_s"] * 1.05, 4)

    def flip_style(b: dict) -> None:
        r = next(x for x in b["runs"] if not x["warmup"])
        r["support_style"] = "default"

    def drop_enable_support(b: dict) -> None:
        r = next(x for x in b["runs"] if not x["warmup"])
        r["enable_support"] = "0"

    def drop_row(b: dict) -> None:
        warm = [x for x in b["runs"] if x["warmup"]]
        first = next(x for x in b["runs"] if not x["warmup"])
        b["runs"] = warm + [first]

    def tamper_pnp(b: dict) -> None:
        b["t103"]["cells"]["base/arachne-on"]["tools"]["pnp-ordinary"]["wall_s"] *= 1.05

    def tamper_t105(b: dict) -> None:
        b["t105"]["cells"]["benchy/classic-on"]["style_wall_delta_pct"] *= 1.2

    def same_output(b: dict) -> None:
        key = "base/classic-on"
        c = b["summary"]["cells"][key]
        c["gcode_bytes"] = list(c["t103_orca_gcode_bytes"])
        c["output_bytes_delta_pct_vs_t103_orca"] = 0.0
        c["non_vacuous_vs_t103_orca"] = False

    def flip_verdict(b: dict) -> None:
        tc = b["summary"]["cells"]["benchy/arachne-on"]["transfer_check"]
        tc["agrees"] = not tc["agrees"]

    def tamper_prior_median(b: dict) -> None:
        b["summary"]["cells"]["base/classic-on"]["t103_orca_wall_s"] *= 1.03

    controls = [
        ("tampered t106 wall", mutate(tamper_wall)),
        ("a row resolved the wrong style", mutate(flip_style)),
        ("a row resolved enable_support 0", mutate(drop_enable_support)),
        ("measured rows dropped", mutate(drop_row)),
        ("ticket 103 PnP median moved", mutate(tamper_pnp)),
        ("ticket 105 style delta moved", mutate(tamper_t105)),
        ("post-pin output equals ticket 103 Orca", mutate(same_output)),
        ("transfer verdict flipped", mutate(flip_verdict)),
        ("ticket 103 Orca median moved", mutate(tamper_prior_median)),
    ]

    rejected = 0
    for name, b in controls:
        bad = [r for r in check(b) if not r[1]]
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
