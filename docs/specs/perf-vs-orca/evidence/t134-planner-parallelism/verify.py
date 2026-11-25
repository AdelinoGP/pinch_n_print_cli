#!/usr/bin/env python3
"""Verify the t134 claim set against `reduced/summary.json`.

Every headline number in `FINDINGS.md` is re-derived here from the reduced
summary. `--self-test` mutates the summary one field at a time and requires the
check suite to reject each mutation; a verifier that cannot fail proves
nothing.
"""

from __future__ import annotations

import argparse
import copy
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent

# Ticket 124 ordinary matched-pair medians (the ratio of record).
T124 = {
    "benchy-classic-off": (5770, 2140),
    "benchy-classic-on": (10010, 2770),
    "benchy-arachne-off": (5340, 2140),
    "benchy-arachne-on": (9960, 2770),
    "base-classic-off": (42530, 14440),
    "base-classic-on": (126270, 27280),
    "base-arachne-off": (41180, 17450),
    "base-arachne-on": (122100, 26240),
}

# Ticket 126 phase split: prepass share, cut-to-beat, and the fraction of the
# prepass that must be removed (the "prepass fraction needed" column).
T126 = {
    "benchy-classic-on": (0.764, 8887, 0.95),
    "benchy-arachne-on": (0.734, 10704, 0.98),
    "base-classic-on": (0.871, 117970, 0.90),
    "base-arachne-on": (0.867, 116449, 0.91),
}

EXPECTED_SHA = {
    "benchy-classic-off": "90638bf3ea9e592d573b223c3eb9af64473efe804578d2478adfbbc5f0e67008",
    "benchy-classic-on": "711c6334e397e5ae6911b15bfc71d42add9e93822cddd0a29391db6ef531d556",
    "benchy-arachne-off": "2ba5d84ccf3c8deea6f39f4977dd71221faaecde9987d1699b361a5366e86380",
    "benchy-arachne-on": "a972b0f2c25a988e667ecf5a5b3d6c3f5ba87f5c8872733c2a4b4e5d51a9695e",
    "base-classic-off": "b3b4ac5e885c7f637b98f79eca21901a41d85739627d4dcb40820663c15e90ef",
    "base-classic-on": "4e31a2db368ff80f388512211ae824b9660342a465c87f093f6182023e34eb4e",
    "base-arachne-off": "6b865d4a2cb892daa3172add3e3677e0dc35192bcedd0acbc463abca7064d715",
    "base-arachne-on": "62d6eedc02e5db972928f830104aab95c3598f6df6407159cb5438adb35bef13",
}

SUPPORTS_ON = [
    "base-classic-on",
    "base-arachne-on",
    "benchy-classic-on",
    "benchy-arachne-on",
]
SUPPORTS_OFF = [
    "base-classic-off",
    "base-arachne-off",
    "benchy-classic-off",
    "benchy-arachne-off",
]


class Checker:
    def __init__(self, summary: dict):
        self.s = summary
        self.checks: list[tuple[str, bool, str]] = []

    def check(self, name: str, ok: bool, detail: str = "") -> None:
        self.checks.append((name, bool(ok), detail))

    def close(self, a: float, b: float, tol: float) -> bool:
        return abs(a - b) <= tol

    def cell(self, name: str) -> dict:
        return self.s["cells"][name]

    def planner(self, name: str) -> dict:
        return self.cell(name)["planner"]

    def phase(self, name: str, phase: str) -> dict:
        return self.planner(name)["phases"][phase]

    def run(self) -> list[tuple[str, bool, str]]:
        self.structure()
        self.headline_parallelism()
        self.phase_rollups()
        self.occupancy_ceiling()
        self.gate_arithmetic()
        self.zero_planner_counterfactual()
        self.hash_set()
        self.repeatability()
        return self.checks

    # -- structure ---------------------------------------------------------
    def structure(self) -> None:
        self.check("eight cells present", len(self.s.get("cells", {})) == 8)
        for cell in T124:
            c = self.cell(cell)
            p = c["planner"]
            phases = p["phases"]
            self.check(
                f"{cell}: eight phases",
                set(phases)
                == {
                    "p0_volumes",
                    "p2_contacts",
                    "p3_drop",
                    "p5_smooth",
                    "p6d_occupancy",
                    "p6e_node_roles",
                    "p6f_emit_geometry",
                    "p6a_clip_core",
                },
            )
            if cell in SUPPORTS_ON:
                self.check(
                    f"{cell}: phase walls close the module wall",
                    self.close(
                        sum(v["wall_ms"] for v in phases.values()),
                        p["module_wall_ms"],
                        max(1.5, 0.05 * p["module_wall_ms"]),
                    ),
                    f"{sum(v['wall_ms'] for v in phases.values()):.1f} vs {p['module_wall_ms']}",
                )
            self.check(
                f"{cell}: phase thread sums to module thread",
                self.close(
                    sum(v["thread_ms"] for v in phases.values()), p["thread_ms"], 1.5
                ),
                f"{sum(v['thread_ms'] for v in phases.values()):.1f} vs {p['thread_ms']}",
            )
            self.check(
                f"{cell}: parallelism matches thread/wall",
                self.close(
                    p["parallelism"], p["thread_ms"] / p["module_wall_ms"], 0.01
                ),
            )
            stage = next(
                e for e in c["stage_census"] if e["stage"] == "PrePass::SupportGeometry"
            )
            self.check(
                f"{cell}: planner module wall matches stage census",
                stage.get("planner_module_wall_ms") == p["module_wall_ms"],
            )

    # -- headline ----------------------------------------------------------
    def headline_parallelism(self) -> None:
        self.check(
            "base classic-on planner 2.76x",
            self.close(self.planner("base-classic-on")["parallelism"], 2.76, 0.02),
        )
        self.check(
            "benchy classic-on planner 2.31x",
            self.close(self.planner("benchy-classic-on")["parallelism"], 2.31, 0.02),
        )
        for cell in SUPPORTS_ON:
            stage = next(
                e
                for e in self.cell(cell)["stage_census"]
                if e["stage"] == "PrePass::SupportGeometry"
            )
            self.check(
                f"{cell}: stage parallelism below 2.7x",
                stage["parallelism"] < 2.7,
                f"{stage['parallelism']}x",
            )
            self.check(
                f"{cell}: planner below every peer stage except MeshAnalysis",
                all(
                    e["parallelism"] > stage["parallelism"]
                    for e in self.cell(cell)["stage_census"]
                    if e["stage"]
                    in {
                        "PrePass::Slice",
                        "PrePass::OverhangAnnotation",
                        "PrePass::ShellClassification",
                        "PrePass::SupportAnalysis",
                        "PrePass::SeamPlanning",
                    }
                ),
            )

    # -- phase rollups -----------------------------------------------------
    def phase_rollups(self) -> None:
        p = self.planner("base-classic-on")
        r = p["rollup"]
        self.check(
            "base: emit pass holds >96% of clipper thread-time",
            r["emit_pass"]["thread_share"] > 0.96,
            f"{r['emit_pass']['thread_share']:.4f}",
        )
        self.check(
            "base: drop loop holds <3% of clipper thread-time",
            r["drop_loop"]["thread_share"] < 0.03,
            f"{r['drop_loop']['thread_share']:.4f}",
        )
        self.check(
            "base: emit pass holds ~88.6% of planner wall",
            self.close(r["emit_pass"]["wall_share"], 0.8856, 0.01),
        )
        self.check(
            "base: occupancy is serial (0.95x-1.05x)",
            self.close(
                self.phase("base-classic-on", "p6d_occupancy")["parallelism"],
                0.99,
                0.05,
            ),
        )
        self.check(
            "base: clip core is ~4.8x",
            self.close(
                self.phase("base-classic-on", "p6a_clip_core")["parallelism"],
                4.81,
                0.05,
            ),
        )
        self.check(
            "benchy: emit pass holds >90% of clipper thread-time",
            self.planner("benchy-classic-on")["rollup"]["emit_pass"]["thread_share"]
            > 0.90,
        )

    # -- occupancy ceiling -------------------------------------------------
    def occupancy_ceiling(self) -> None:
        p = self.planner("base-classic-on")
        occ = self.phase("base-classic-on", "p6d_occupancy")
        clip = self.phase("base-classic-on", "p6a_clip_core")
        floor = 0.008 * T124["base-classic-on"][0]
        req = (
            T126["base-classic-on"][0]
            * T126["base-classic-on"][1]
            * T126["base-classic-on"][2]
        )
        for fan, expected_save, expected_multiple, expected_share in [
            (2.6, 8719, 8.6, 9.4),
            (clip["parallelism"], 11222, 11.1, 12.1),
            (12.0, 12988, 12.9, 14.0),
        ]:
            save = occ["thread_ms"] * (1.0 - 1.0 / fan)
            self.check(
                f"base: occupancy batch at {fan:.2f}x saves {expected_save}ms",
                self.close(save, expected_save, 60),
                f"{save:.0f}",
            )
            self.check(
                f"base: occupancy batch at {fan:.2f}x is {expected_multiple}x floor",
                self.close(save / floor, expected_multiple, 0.3),
            )
            self.check(
                f"base: occupancy batch at {fan:.2f}x is {expected_share}% of required cut",
                self.close(100.0 * save / req, expected_share, 0.4),
            )
        clip_save = clip["wall_ms"] - clip["thread_ms"] / 12.0
        occ_save = occ["thread_ms"] * (1.0 - 1.0 / clip["parallelism"])
        combined = clip_save + occ_save
        self.check(
            "base: clip at 12x saves 10.7s",
            self.close(clip_save, 10675, 80),
            f"{clip_save:.0f}",
        )
        self.check(
            "base: combined occupancy+clip12 saves 21.9s",
            self.close(combined, 21896, 120),
            f"{combined:.0f}",
        )
        self.check(
            "base: combined ceiling is ~21.7x floor",
            self.close(combined / floor, 21.7, 0.4),
        )
        self.check(
            "base: combined ceiling is ~23.7% of required cut",
            self.close(100.0 * combined / req, 23.7, 0.5),
        )
        self.check(
            "occupancy call census is 433 unique offsets",
            occ["calls"] == 433,
            str(occ["calls"]),
        )

    # -- gate arithmetic ---------------------------------------------------
    def gate_arithmetic(self) -> None:
        for cell in SUPPORTS_OFF:
            floor = 0.008 * T124[cell][0]
            stage = next(
                e
                for e in self.cell(cell)["stage_census"]
                if e["stage"] == "PrePass::SupportGeometry"
            )
            self.check(
                f"{cell}: whole SupportGeometry stage below 1.0x floor",
                stage["wall_ms"] < floor,
                f"{stage['wall_ms']}ms vs {floor:.1f}ms",
            )
            self.check(
                f"{cell}: planner module below 0.5x floor",
                self.cell(cell)["planner"]["module_wall_ms"] < 0.5 * floor,
                f"{self.cell(cell)['planner']['module_wall_ms']}ms vs {0.5 * floor:.1f}ms",
            )
        for cell in SUPPORTS_ON:
            floor = 0.008 * T124[cell][0]
            occ = self.phase(cell, "p6d_occupancy")
            save = occ["thread_ms"] * (
                1.0 - 1.0 / self.phase(cell, "p6a_clip_core")["parallelism"]
            )
            self.check(
                f"{cell}: occupancy tranche clears the 0.80% floor",
                save > floor,
                f"{save:.0f}ms vs {floor:.1f}ms",
            )

    # -- zero-planner counterfactual --------------------------------------
    def zero_planner_counterfactual(self) -> None:
        ratios = {}
        for cell in SUPPORTS_ON:
            pnp, orca = T124[cell]
            ratios[cell] = (pnp - self.cell(cell)["planner"]["module_wall_ms"]) / orca
        self.check(
            "zero-planner leaves base classic-on at 3.25x",
            self.close(ratios["base-classic-on"], 3.25, 0.02),
            f"{ratios['base-classic-on']:.2f}x",
        )
        self.check(
            "zero-planner leaves base arachne-on at 3.21x",
            self.close(ratios["base-arachne-on"], 3.21, 0.02),
            f"{ratios['base-arachne-on']:.2f}x",
        )
        self.check(
            "zero-planner leaves benchy classic-on at 2.89x",
            self.close(ratios["benchy-classic-on"], 2.89, 0.02),
            f"{ratios['benchy-classic-on']:.2f}x",
        )
        self.check(
            "zero-planner leaves benchy arachne-on at 2.87x",
            self.close(ratios["benchy-arachne-on"], 2.87, 0.02),
            f"{ratios['benchy-arachne-on']:.2f}x",
        )
        self.check(
            "zero-planner still loses all four supports-on cells",
            all(r > 1.0 for r in ratios.values()),
        )

    # -- output set --------------------------------------------------------
    def hash_set(self) -> None:
        for cell, expected in EXPECTED_SHA.items():
            self.check(
                f"{cell}: probe output equals t123 reference",
                self.cell(cell)["run"]["sha256"] == expected,
            )

    # -- repeatability -----------------------------------------------------
    def repeatability(self) -> None:
        for cell in ("base-classic-on", "benchy-classic-on"):
            spread = self.s["repeatability"][cell]["module_wall_spread_pct"]
            self.check(
                f"{cell}: repeats spread below 5%",
                spread < 5.0,
                f"{spread}%",
            )


def run_checks(summary: dict) -> list[tuple[str, bool, str]]:
    return Checker(summary).run()


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--summary", type=Path, default=HERE / "reduced" / "summary.json")
    ap.add_argument("--self-test", action="store_true")
    args = ap.parse_args()
    summary = json.loads(args.summary.read_text(encoding="utf-8"))

    if args.self_test:
        controls = []
        base = run_checks(summary)
        base_failures = [c for c in base if not c[1]]
        if base_failures:
            print(f"baseline failed {len(base_failures)} checks")
            for name, _, detail in base_failures:
                print(f"  FAIL {name} {detail}")
            return 1
        mutations = [
            (
                "occupancy wall",
                lambda s: s["cells"]["base-classic-on"]["planner"]["phases"][
                    "p6d_occupancy"
                ].__setitem__("wall_ms", 10000.0),
            ),
            (
                "clip parallelism",
                lambda s: s["cells"]["base-classic-on"]["planner"]["phases"][
                    "p6a_clip_core"
                ].__setitem__("parallelism", 6.0),
            ),
            (
                "emit thread share",
                lambda s: s["cells"]["base-classic-on"]["planner"]["rollup"][
                    "emit_pass"
                ].__setitem__("thread_share", 0.5),
            ),
            (
                "drop thread share",
                lambda s: s["cells"]["base-classic-on"]["planner"]["rollup"][
                    "drop_loop"
                ].__setitem__("thread_share", 0.2),
            ),
            (
                "planner wall",
                lambda s: s["cells"]["base-classic-on"]["planner"].__setitem__(
                    "module_wall_ms", 30000
                ),
            ),
            (
                "phase removed",
                lambda s: s["cells"]["base-classic-on"]["planner"]["phases"].pop(
                    "p5_smooth"
                ),
            ),
            (
                "hash moved",
                lambda s: s["cells"]["base-classic-on"]["run"].__setitem__(
                    "sha256", "0" * 64
                ),
            ),
            (
                "repeat spread",
                lambda s: s["repeatability"]["base-classic-on"].__setitem__(
                    "module_wall_spread_pct", 9.0
                ),
            ),
            (
                "stage floor",
                lambda s: next(
                    e
                    for e in s["cells"]["base-classic-off"]["stage_census"]
                    if e["stage"] == "PrePass::SupportGeometry"
                ).__setitem__("wall_ms", 1000),
            ),
            (
                "occupancy calls",
                lambda s: s["cells"]["base-classic-on"]["planner"]["phases"][
                    "p6d_occupancy"
                ].__setitem__("calls", 300),
            ),
            (
                "module thread",
                lambda s: s["cells"]["base-classic-on"]["planner"].__setitem__(
                    "thread_ms", 50000.0
                ),
            ),
            (
                "stage module wall",
                lambda s: next(
                    e
                    for e in s["cells"]["base-classic-on"]["stage_census"]
                    if e["stage"] == "PrePass::SupportGeometry"
                ).__setitem__("planner_module_wall_ms", 10000),
            ),
        ]
        rejected = 0
        for name, mutate in mutations:
            mutated = copy.deepcopy(summary)
            mutate(mutated)
            failures = [c for c in run_checks(mutated) if not c[1]]
            ok = bool(failures)
            rejected += int(ok)
            print(
                f"  {'REJECT' if ok else 'ACCEPT'} {name}: {len(failures)} failing check(s)"
            )
        print(f"{rejected}/{len(mutations)} negative controls rejected")
        return 0 if rejected == len(mutations) else 1

    checks = run_checks(summary)
    failed = [c for c in checks if not c[1]]
    for name, ok, detail in checks:
        if not ok:
            print(f"FAIL {name} {detail}")
    print(f"{len(checks) - len(failed)}/{len(checks)} checks passed")
    return 0 if not failed else 1


if __name__ == "__main__":
    sys.exit(main())
