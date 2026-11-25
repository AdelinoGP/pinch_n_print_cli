#!/usr/bin/env python3
"""t173 verification — re-derive every quoted number from committed artifacts.

Checkout-only: reads the `reduced/` set plus the sibling tickets' committed
artifacts. No raw `target/` tree is needed.

Checks:
  1. the two probe runs reproduce t172's active reference set (5 cells + 2
     combined-window cells);
  2. the 15 instrumented passes are clean and reference-matched;
  3. the planner clip/offset totals re-derive from the probe rows and batches;
  4. the phase-1 in-bbox share re-derives from the operand counts;
  5. the stage share re-derives from the instrumented median;
  6. the route ratios and required cuts re-derive from t163/t169/t172;
  7. every shape price re-derives from the census and the route.

Usage:
  python verify.py            # 0 = every check passes
  python verify.py --self-test # 8/8 negative controls rejected
"""

from __future__ import annotations

import copy
import hashlib
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
EVID = HERE.parent
CELLS = [
    "base-classic-on",
    "base-arachne-on",
    "benchy-classic-on",
    "benchy-arachne-on",
    "base-classic-off",
]
SAME_WINDOW: set[str] = set()
CLIP_OPS = {"union", "intersection", "difference", "xor"}
FAILS: list[str] = []


def check(name: str, ok: bool, detail: str = "") -> None:
    if not ok:
        FAILS.append(f"{name}: {detail}")


def load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def close(a: float, b: float, tol: float = 1e-3) -> bool:
    return abs(a - b) <= tol * max(1e-9, abs(a), abs(b))


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def rederive(
    arith: dict, probe_run: dict, probe_comb: dict, inst: dict, inst_comb: dict
) -> dict:
    out = {"route": {}, "census": {}, "prices": {}, "split": {}}
    t163 = load(EVID / "t163-accelerated-board" / "reduced" / "summary.json")
    t169 = load(EVID / "t169-offset-decimation-flat-opening" / "reduced" / "ab.json")
    t172 = load(EVID / "t172-offset-concave-crossing" / "reduced" / "ab.json")
    for cell in CELLS:
        orca = t163["cells"][cell]["orca"]["wall_median_s"]
        ratio163 = t163["cells"][cell]["pnp-ordinary"]["wall_median_s"] / orca
        ratio = (
            ratio163
            * (1 - t169[cell]["wall"]["median_saving_pct"] / 100.0)
            * (1 - t172[cell]["wall"]["median_saving_pct"] / 100.0)
        )
        out["route"][cell] = {
            "estimated_ratio": round(ratio, 5),
            "required_cut": round(1 - 1 / ratio, 6),
            "pnp": t172[cell]["wall"]["candidate_median_s"],
            "orca": round(orca, 5),
        }
        src = probe_comb if cell in SAME_WINDOW else probe_run
        rec = src["cells"][cell]
        rows = rec["rows"]
        batches = rec["batches"]
        planner_clip_thread = sum(
            r["thread_ns"]
            for r in rows
            if r["module"] == "com.core.tree-support-planner" and r["op"] in CLIP_OPS
        )
        planner_offset_thread = sum(
            r["thread_ns"]
            for r in rows
            if r["module"] == "com.core.tree-support-planner"
            and r["op"] not in CLIP_OPS
        )
        site_op = {(r["module"], r["site"]): r["op"] for r in rows}
        planner_clip_wall = sum(
            b["wall_ns"]
            for b in batches
            if b["module"] == "com.core.tree-support-planner"
            and site_op.get((b["module"], b["site"]), "offset") in CLIP_OPS
        )
        planner_offset_wall = sum(
            b["wall_ns"]
            for b in batches
            if b["module"] == "com.core.tree-support-planner"
            and site_op.get((b["module"], b["site"]), "offset") not in CLIP_OPS
        )
        out["census"][cell] = {
            "planner_clip_thread_ms": round(planner_clip_thread / 1e6, 1),
            "planner_offset_thread_ms": round(planner_offset_thread / 1e6, 1),
            "planner_clip_wall_ms": round(planner_clip_wall / 1e6, 1),
            "planner_offset_wall_ms": round(planner_offset_wall / 1e6, 1),
        }
        inst_src = inst_comb if cell in SAME_WINDOW else inst
        passes = inst_src["cells"][cell]["passes"]
        total = inst_src["cells"][cell]["median"]["slice_complete_ms"]
        stage = sorted(
            p["prepass_stages_ms"].get("PrePass::SupportGeometry", 0) for p in passes
        )
        stage = stage[len(stage) // 2]
        share = stage / total
        out["split"][cell] = {"stage_share": round(share, 6), "stage_ms": stage}
        cut_s = (1 - 1 / ratio) * t172[cell]["wall"]["candidate_median_s"]
        p1 = next(
            (r for r in rows if r["site"] == "planner::batched_phase1_carve"), None
        )
        p1_batch = next(
            (b for b in batches if b["site"] == "planner::batched_phase1_carve"), None
        )
        if p1 and p1_batch and p1["clip_points"]:
            windowed = p1["in_points"] - p1["clip_points"] + p1["in_bbox_points"]
            edge_share = windowed / p1["in_points"]
            fanout = p1["thread_ns"] / p1_batch["wall_ns"]
            saving_thread_ms = (p1["thread_ns"] / 1e6) * (1 - edge_share)
            saving_wall_ms = saving_thread_ms / fanout
            out["prices"][cell] = {
                "phase1_window_pct_cut": round(
                    100 * saving_wall_ms / (cut_s * 1000), 2
                ),
                "phase1_window_ratio_after": round(
                    (t172[cell]["wall"]["candidate_median_s"] * 1000 - saving_wall_ms)
                    / (orca * 1000),
                    4,
                ),
                "in_bbox_share": round(p1["in_bbox_points"] / p1["clip_points"], 9),
                "phase1_fanout": round(fanout, 3),
                "all_clip_pct_cut": round(
                    100 * (planner_clip_wall / 1e6) / (cut_s * 1000), 2
                ),
            }
    return out


def verify(arith: dict, rederived: dict) -> None:
    for cell in CELLS:
        r = arith["route"][cell]
        d = rederived["route"][cell]
        check("route_ratio", close(r["estimated_ratio"], d["estimated_ratio"]), cell)
        check("route_cut", close(r["required_cut"], d["required_cut"]), cell)
        c = arith["census"][cell]
        dc = rederived["census"][cell]
        for field in (
            "planner_clip_thread_ms",
            "planner_offset_thread_ms",
            "planner_clip_wall_ms",
            "planner_offset_wall_ms",
        ):
            check(f"census_{field}", close(c[field], dc[field], 0.02), cell)
        s = arith["split"][cell]
        check(
            "stage_share",
            close(
                s["stage_share_of_cell"], rederived["split"][cell]["stage_share"], 0.03
            ),
            cell,
        )
        p = arith["prices"][cell]["prices"]
        if cell in rederived["prices"]:
            dp = rederived["prices"][cell]
            check(
                "phase1_window_pct_cut",
                close(
                    p["phase1_bbox_window_ceiling"]["pct_of_required_cut"],
                    dp["phase1_window_pct_cut"],
                    0.05,
                ),
                cell,
            )
            check(
                "phase1_window_ratio_after",
                close(
                    p["phase1_bbox_window_ceiling"]["ratio_after"],
                    dp["phase1_window_ratio_after"],
                    0.02,
                ),
                cell,
            )
            check(
                "in_bbox_share",
                close(
                    p["phase1_bbox_window_ceiling"]["in_bbox_share"],
                    dp["in_bbox_share"],
                    0.02,
                ),
                cell,
            )
            check(
                "all_clip_pct_cut",
                close(
                    p["all_planner_clip_free_ceiling"]["pct_of_required_cut"],
                    dp["all_clip_pct_cut"],
                    0.05,
                ),
                cell,
            )


def reference_checks(runs: dict, inst: dict, inst_comb: dict) -> None:
    refs = load(EVID / "t172-offset-concave-crossing" / "reference.json")["active_set"]
    for cell, rec in runs["cells"].items():
        check("reference_match", rec["reference_match"], cell)
        check("reference_hash", rec["gcode_sha256"] == refs.get(cell), cell)
        check("probe_exit", rec["exit_code"] == 0, cell)
    for src in (inst, inst_comb):
        for cell, rec in src["cells"].items():
            for i, p in enumerate(rec["passes"], 1):
                check(
                    "instrumented_ref",
                    p["gcode_sha256"] == refs.get(cell),
                    f"{cell} pass{i}",
                )
                check(
                    "instrumented_degraded",
                    p.get("degraded") is not True,
                    cell,
                )


def run_suite(
    arith: dict,
    census_probe: dict,
    census_comb: dict,
    runs: dict,
    inst: dict,
    inst_comb: dict,
) -> list[str]:
    FAILS.clear()
    rederived = rederive(arith, census_probe, census_comb, inst, inst_comb)
    verify(arith, rederived)
    reference_checks(runs, inst, inst_comb)
    return list(FAILS)


def self_test() -> int:
    arith = load(HERE / "reduced" / "arithmetic.json")
    census_probe = load(HERE / "reduced" / "census-probe.json")
    census_comb = load(HERE / "reduced" / "census-combined.json")
    runs = load(HERE / "reduced" / "runs-probe.json")
    inst = load(HERE / "reduced" / "instrumented.json")
    inst_comb = load(HERE / "reduced" / "instrumented-combined.json")
    controls = []

    def mutate_arith(fn):
        a = copy.deepcopy(arith)
        fn(a)
        return run_suite(a, census_probe, census_comb, runs, inst, inst_comb)

    controls.append(
        (
            "route ratio",
            mutate_arith(
                lambda a: a["route"]["base-classic-on"].__setitem__(
                    "estimated_ratio",
                    a["route"]["base-classic-on"]["estimated_ratio"] * 1.02,
                )
            ),
        )
    )
    controls.append(
        (
            "census thread",
            mutate_arith(
                lambda a: a["census"]["base-classic-on"].__setitem__(
                    "planner_clip_thread_ms",
                    a["census"]["base-classic-on"]["planner_clip_thread_ms"] * 1.05,
                )
            ),
        )
    )
    controls.append(
        (
            "stage share",
            mutate_arith(
                lambda a: a["split"]["base-classic-on"].__setitem__(
                    "stage_share_of_cell",
                    a["split"]["base-classic-on"]["stage_share_of_cell"] * 1.05,
                )
            ),
        )
    )
    controls.append(
        (
            "phase1 price",
            mutate_arith(
                lambda a: a["prices"]["base-classic-on"]["prices"][
                    "phase1_bbox_window_ceiling"
                ].__setitem__(
                    "pct_of_required_cut",
                    a["prices"]["base-classic-on"]["prices"][
                        "phase1_bbox_window_ceiling"
                    ]["pct_of_required_cut"]
                    * 1.10,
                )
            ),
        )
    )
    controls.append(
        (
            "in-bbox share",
            mutate_arith(
                lambda a: a["prices"]["base-classic-on"]["prices"][
                    "phase1_bbox_window_ceiling"
                ].__setitem__(
                    "in_bbox_share",
                    a["prices"]["base-classic-on"]["prices"][
                        "phase1_bbox_window_ceiling"
                    ]["in_bbox_share"]
                    * 2.0,
                )
            ),
        )
    )
    controls.append(
        (
            "all-clip price",
            mutate_arith(
                lambda a: a["prices"]["base-classic-on"]["prices"][
                    "all_planner_clip_free_ceiling"
                ].__setitem__(
                    "pct_of_required_cut",
                    a["prices"]["base-classic-on"]["prices"][
                        "all_planner_clip_free_ceiling"
                    ]["pct_of_required_cut"]
                    * 1.10,
                )
            ),
        )
    )

    def mutate_run(fn):
        r = copy.deepcopy(runs)
        fn(r)
        return run_suite(arith, census_probe, census_comb, r, inst, inst_comb)

    controls.append(
        (
            "reference hash",
            mutate_run(
                lambda r: r["cells"]["base-classic-on"].__setitem__(
                    "gcode_sha256", "0" * 64
                )
            ),
        )
    )
    controls.append(
        (
            "probe exit",
            mutate_run(
                lambda r: r["cells"]["base-classic-on"].__setitem__("exit_code", 1)
            ),
        )
    )
    rejected = 0
    for name, fails in controls:
        if fails:
            rejected += 1
        else:
            print("SELF-TEST MISS:", name)
    print(f"SELF-TEST {rejected}/{len(controls)} negative controls rejected")
    return 0 if rejected == len(controls) else 1


def main() -> int:
    arith = load(HERE / "reduced" / "arithmetic.json")
    census_probe = load(HERE / "reduced" / "census-probe.json")
    census_comb = load(HERE / "reduced" / "census-combined.json")
    runs = load(HERE / "reduced" / "runs-probe.json")
    inst = load(HERE / "reduced" / "instrumented.json")
    inst_comb = load(HERE / "reduced" / "instrumented-combined.json")
    fails = run_suite(arith, census_probe, census_comb, runs, inst, inst_comb)
    if fails:
        print(f"FAIL ({len(fails)} checks)")
        for f in fails:
            print(" ", f)
        return 1
    print("PASS - every quoted number re-derives from the committed reduced set")
    return 0


if __name__ == "__main__":
    if "--self-test" in sys.argv:
        raise SystemExit(self_test())
    raise SystemExit(main())
