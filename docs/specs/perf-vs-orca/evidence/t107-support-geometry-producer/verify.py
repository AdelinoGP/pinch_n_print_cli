#!/usr/bin/env python3
"""t107 `PrePass::SupportGeometry` screen -- verifier.

Re-derives every headline number in FINDINGS.md from the committed inputs and
asserts the structural claims. Exits 0 only if every claim holds.

  python verify.py              # verify the committed set
  python verify.py --self-test  # negative controls: mutate the set, demand rejection

Inputs (all in-repo):
  summary.json             reduced brackets + arithmetic
  preview-counts.json      `support-preview` counts for the benchy-on IR
  t104 phase-split.json    the committed value this take reconciles against
  t102 summary.json        the committed base classic-on floor
  t103 summary.json        the PnP ordinary walls
"""

from __future__ import annotations

import copy
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
T104 = (
    REPO
    / "docs"
    / "specs"
    / "perf-vs-orca"
    / "evidence"
    / "t104-phase-split"
    / "phase-split.json"
)
T102 = (
    REPO
    / "docs"
    / "specs"
    / "perf-vs-orca"
    / "evidence"
    / "t102-allocator"
    / "summary.json"
)
T103 = (
    REPO
    / "docs"
    / "specs"
    / "perf-vs-orca"
    / "evidence"
    / "t103-orca-rebaseline"
    / "summary.json"
)

STAGE = "PrePass::SupportGeometry"
PRODUCER = "host:support_geometry"
TREE = "com.core.tree-support-planner"
CELLS = (
    "benchy-classic-off",
    "benchy-classic-on",
    "base-classic-off",
    "base-classic-on",
)
SUPPORTS_ON = ("benchy-classic-on", "base-classic-on")


def load() -> dict:
    return {
        "summary": json.loads((HERE / "summary.json").read_text(encoding="utf-8")),
        "preview": json.loads(
            (HERE / "preview-counts.json").read_text(encoding="utf-8")
        ),
        "t104": json.loads(T104.read_text(encoding="utf-8"))["cells"],
        "t102": json.loads(T102.read_text(encoding="utf-8")),
        "t103": json.loads(T103.read_text(encoding="utf-8"))["cells"],
    }


def check(b: dict) -> list[tuple[str, bool, str]]:
    out: list[tuple[str, bool, str]] = []
    summary, preview = b["summary"], b["preview"]

    def add(name: str, ok: bool, detail: str = "") -> None:
        out.append((name, bool(ok), detail))

    cells = summary["cells"]
    add(
        "all four classic cells present",
        set(cells) == set(CELLS),
        f"got {sorted(cells)}",
    )

    for cell in CELLS:
        if cell not in cells:
            continue
        v = cells[cell]
        brackets = v["stage_brackets"]
        producer = [x for x in brackets if PRODUCER in x["modules"]]
        planner = [x for x in brackets if PRODUCER not in x["modules"]]
        add(
            f"{cell}: exactly two {STAGE} brackets, one producer and one planner",
            len(brackets) == 2 and len(producer) == 1 and len(planner) == 1,
            f"{[(x['role'], x['elapsed_ms']) for x in brackets]}",
        )
        add(
            f"{cell}: producer + planner == bracket sum",
            v["producer_ms"] + v["planner_ms"] == v["stage_sum_ms"],
            f"{v['producer_ms']} + {v['planner_ms']} != {v['stage_sum_ms']}",
        )
        add(
            f"{cell}: bracket sum reconciles with t104's committed {STAGE} value",
            v["stage_sum_ms"]
            == v["t104_committed_stage_ms"]
            == b["t104"][cell]["prepass_stages_ms"].get(STAGE),
            f"{v['stage_sum_ms']} vs {v['t104_committed_stage_ms']} vs "
            f"{b['t104'][cell]['prepass_stages_ms'].get(STAGE)}",
        )
        add(
            f"{cell}: {STAGE} is the only duplicated prepass stage id in the stream",
            v["duplicated_prepass_stage_ids"] == [STAGE],
            f"{v['duplicated_prepass_stage_ids']}",
        )
        add(
            f"{cell}: producer is the {PRODUCER} builtin bracket",
            bool(producer)
            and producer[0]["role"] == "host_producer"
            and producer[0]["module_ms"].get(PRODUCER) == v["producer_ms"],
            f"{producer[0]['module_ms'] if producer else None}",
        )

        # The decisive claim: the whole producer cannot matter.
        add(
            f"{cell}: producer below 1/100 of the 0.80 % floor",
            v["floor_multiple"]["producer"] < 0.01,
            f"{v['floor_multiple']['producer']}x floor",
        )
        add(
            f"{cell}: producer share of cell at most 0.005 %",
            v["share_of_cell_pct"]["producer"] <= 0.005,
            f"{v['share_of_cell_pct']['producer']} %",
        )
        add(
            f"{cell}: producer at most 9 ms",
            v["producer_ms"] <= 9,
            f"{v['producer_ms']} ms",
        )

        # Planner arithmetic, recomputed from the committed wall and floor.
        fixture, generator, supports = cell.split("-", 2)
        wall_s = b["t103"][f"{fixture}/{generator}-{supports}"]["tools"][
            "pnp-ordinary"
        ]["wall_s"]
        floor_s = (
            b["t102"]["cells"]["ordinary-base-classic-on"]["floor_seconds"]
            if cell == "base-classic-on"
            else 0.008 * wall_s
        )
        add(
            f"{cell}: floor is the recorded {round(floor_s, 6)} s",
            abs(v["floor_s"] - floor_s) < 1e-6,
            f"{v['floor_s']} vs {floor_s}",
        )
        share = 100.0 * v["planner_ms"] / v["instrumented_cell_ms"]
        add(
            f"{cell}: planner share recomputes to {v['share_of_cell_pct']['planner']} %",
            abs(share - v["share_of_cell_pct"]["planner"]) < 1e-3,
            f"{share} vs {v['share_of_cell_pct']['planner']}",
        )
        multiple = (v["planner_ms"] / 1000.0) / floor_s
        add(
            f"{cell}: planner floor multiple recomputes",
            abs(multiple - v["floor_multiple"]["planner"]) < 0.01,
            f"{multiple} vs {v['floor_multiple']['planner']}",
        )
        implied = (v["planner_ms"] / v["instrumented_cell_ms"]) * wall_s
        add(
            f"{cell}: planner implied wall recomputes",
            abs(v["implied_wall_s"]["planner"] - implied) < 1e-3,
            f"{v['implied_wall_s']['planner']} vs {implied}",
        )
        add(
            f"{cell}: recorded planner module map matches its bracket",
            bool(planner) and v["planner_module_ms"] == planner[0]["module_ms"],
            "",
        )
        add(
            f"{cell}: module times fit inside the planner bracket",
            sum(v["planner_module_ms"].values()) <= v["planner_ms"],
            f"{v['planner_module_ms']} <= {v['planner_ms']}",
        )

        if cell in SUPPORTS_ON:
            add(
                f"{cell}: planner above the floor by at least 32x",
                v["floor_multiple"]["planner"] >= 32,
                f"{v['floor_multiple']['planner']}x",
            )
            add(
                f"{cell}: planner at least 26 % of the cell",
                v["share_of_cell_pct"]["planner"] >= 26,
                f"{v['share_of_cell_pct']['planner']} %",
            )
            tree = v["planner_module_ms"].get(TREE, 0)
            add(
                f"{cell}: tree planner carries at least 90 % of the planner bracket",
                tree >= 0.90 * v["planner_ms"],
                f"{tree} / {v['planner_ms']}",
            )
        else:
            add(
                f"{cell}: planner below the floor (no support work)",
                v["floor_multiple"]["planner"] < 1.0,
                f"{v['floor_multiple']['planner']}x",
            )

    # IR volume: the reason a full deep copy is cheap.
    add(
        "preview was taken on benchy classic-on over all 240 layers",
        preview["cell"] == "benchy-classic-on"
        and preview["layer_count"] == 240
        and preview["layers_with_geometry"] == 240,
        f"{preview['cell']} {preview['layer_count']}/{preview['layers_with_geometry']}",
    )
    add(
        "geometry outline set is small: 577 polygons / 444,259 points",
        preview["geometry_support_polygons"] == 577
        and preview["geometry_support_points"] == 444259,
        f"{preview['geometry_support_polygons']} / {preview['geometry_support_points']}",
    )
    heap = (
        preview["geometry_support_points"] * 16
        + preview["geometry_support_polygons"] * 24
    )
    add(
        "geometry outline heap estimate stays under 8 MB", heap < 8_000_000, f"{heap} B"
    )
    add(
        "exactly one sentinel (intermediate-layer) entry exists",
        preview["skipped_intermediate_entries"] == 1,
        f"{preview['skipped_intermediate_entries']}",
    )

    # The stop rule, stated as arithmetic rather than taste.
    add(
        "no producer shape can clear the floor: every cell's producer is under 1/100 of it",
        all(cells[c]["floor_multiple"]["producer"] < 0.01 for c in CELLS if c in cells),
    )
    add(
        "ledger correction: on the supports-on cells t104's value is essentially the planner",
        all(
            cells[c]["producer_ms"] < cells[c]["stage_sum_ms"] * 0.001
            for c in SUPPORTS_ON
            if c in cells
        ),
    )
    return out


def _cells(b: dict, cell: str) -> dict:
    return b["summary"]["cells"][cell]


def main() -> int:
    bundle = load()
    rows = check(bundle)
    failed = [r for r in rows if not r[1]]
    for name, ok, detail in rows:
        print(
            f"{'PASS' if ok else 'FAIL'}  {name}" + (f"  [{detail}]" if detail else "")
        )
    print(f"\n{len(rows) - len(failed)}/{len(rows)} claims hold")
    if failed:
        return 1

    if "--self-test" in sys.argv:
        controls = [
            (
                "producer above the floor",
                lambda b: _cells(b, "base-classic-on").__setitem__("producer_ms", 1500),
            ),
            (
                "bracket sum broken",
                lambda b: _cells(b, "base-classic-on").__setitem__(
                    "stage_sum_ms", 91903
                ),
            ),
            (
                "planner bracket dropped",
                lambda b: _cells(b, "base-classic-on")["stage_brackets"].pop(),
            ),
            (
                "tree module erased",
                lambda b: _cells(b, "base-classic-on")["planner_module_ms"].__setitem__(
                    TREE, 0
                ),
            ),
            (
                "producer share inflated",
                lambda b: _cells(b, "base-classic-on")["share_of_cell_pct"].__setitem__(
                    "producer", 1.0
                ),
            ),
            (
                "floor halved",
                lambda b: _cells(b, "base-classic-on").__setitem__("floor_s", 0.659212),
            ),
            (
                "planner module sum overflows",
                lambda b: _cells(b, "base-classic-on")["planner_module_ms"].__setitem__(
                    TREE, 10**9
                ),
            ),
            (
                "preview layer count wrong",
                lambda b: b["preview"].__setitem__("layer_count", 239),
            ),
            (
                "preview geometry empty",
                lambda b: b["preview"].__setitem__("geometry_support_polygons", 0),
            ),
        ]
        missed = 0
        for label, mutate in controls:
            mutated = copy.deepcopy(bundle)
            mutate(mutated)
            if all(ok for _, ok, _ in check(mutated)):
                print(f"SELF-TEST MISSED: {label}")
                missed += 1
        print(
            f"self-test: {len(controls) - missed}/{len(controls)} negative controls detected"
        )
        return 0 if missed == 0 else 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
