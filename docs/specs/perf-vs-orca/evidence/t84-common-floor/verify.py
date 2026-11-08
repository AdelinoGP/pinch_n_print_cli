#!/usr/bin/env python3
"""Checkout-only verification for the t84 common-floor attribution.

Re-derives ``summary.json`` from the committed per-run captures, checks the
attribution identities and the pre-declared shape verdicts, and runs negative
controls that must be rejected.

    python verify.py             # verify committed evidence
    python verify.py --self-test # verify + negative controls
"""

import argparse
import copy
import json
from pathlib import Path

import reduce

HERE = reduce.HERE

REFERENCES = {
    "base-classic-off": "292df50179c4033d6204a0353e038534b4d3a98381126998bb5722a41a571a80",
    "base-classic-on": "22200671167a97a7efb1b912e9e1661a317b5e81dab169ec154ab5ad4b9f028d",
}

RUN_LABELS = ("head", "probe")
CELLS = ("base-classic-off", "base-classic-on")


def expect(condition, message):
    if not condition:
        raise AssertionError(message)


def base_store():
    store = {
        (cell, label): reduce.capture(cell, label)
        for cell in CELLS
        for label in RUN_LABELS
    }
    store[("base-classic-off", "probe3")] = reduce.capture("base-classic-off", "probe3")
    return store


def run_assertions(summary, store):
    expect(
        summary["floor_pct_of_cell_wall"] == reduce.FLOOR_PCT,
        "floor changed from the ticket's pre-declared 0.80 %",
    )
    expect(len(summary["cells"]) == 2, "expected the two base classic cells")

    for cell, data in summary["cells"].items():
        head = store[(cell, "head")]
        probe = store[(cell, "probe")]

        # Output identity: the probe build must not move the printed G-code,
        # and the HEAD run must reproduce the t81 campaign reference.
        expect(
            head["sha256"] == REFERENCES[cell],
            f"{cell}: HEAD output does not match the t81 reference",
        )
        expect(
            probe["sha256"] == head["sha256"],
            f"{cell}: probe output differs from the HEAD output",
        )
        expect(not head["degraded"] and head["fatal"] == 0, f"{cell}: degraded capture")

        overhang = data["overhang"]
        seam = data["seam"]

        # Overhang: the host terms must be negligible, the kernel must own the
        # stage, and its work must be the canonical three offsets per band
        # layer, already parallel.
        expect(
            overhang["kernel_share_of_stage_pct"] >= 99.5,
            f"{cell}: overhang kernel share fell below 99.5 %",
        )
        expect(
            overhang["host_terms_share_of_stage_pct"] <= 0.5,
            f"{cell}: overhang host terms are not negligible",
        )
        expect(
            overhang["native_offset_calls"] == 3 * overhang["counts"]["band_layers"],
            f"{cell}: expected 3 round offsets per band layer",
        )
        expect(
            overhang["native_occupancy"] >= 10.0,
            f"{cell}: overhang kernel parallelism below 10x",
        )
        expect(
            overhang["offset_share_of_kernel_work_pct"] >= 80.0,
            f"{cell}: offsets are not the dominant kernel op",
        )

        # Seam: one host offset per region dominates; the candidate list does
        # not, which is what refutes the obvious suspect.
        expect(
            seam["offset_project_share_of_stage_pct"] >= 90.0,
            f"{cell}: seam inset offsets fell below 90 % of the stage",
        )
        expect(
            seam["project_share_of_stage_pct"] <= 0.5,
            f"{cell}: seam view projection is not negligible",
        )
        expect(
            seam["push_ms"] <= 5.0,
            f"{cell}: push_seam_plan is not negligible",
        )
        list_terms = seam["candidates_ms"] + seam["choose_ms"] + seam["harvest_ms"]
        expect(
            100.0 * list_terms / seam["stage_ms_probe"] <= 2.0,
            f"{cell}: candidate-list terms exceed 2 % of the seam stage",
        )
        expect(
            seam["counts"]["candidates"] == seam["counts"]["points"] == 2553550,
            f"{cell}: candidate count no longer equals the boundary point count",
        )
        expect(
            seam["counts"]["entries"]
            == seam["pushes"]
            == seam["counts"]["regions"]
            == 495,
            f"{cell}: region/entry/push counts disagree",
        )

        shapes = {shape["name"]: shape for shape in data["shapes"]}
        expect(
            shapes["seam-batch-inset-offsets"]["verdict"] == "GRADUATE",
            f"{cell}: seam batching did not clear the floor",
        )
        expect(
            shapes["seam-batch-inset-offsets"]["ceiling_at_reference_speedup_pct"]
            >= reduce.FLOOR_PCT,
            f"{cell}: seam batching ceiling is below the floor",
        )
        for name in ("overhang-bbox-window-operand", "seam-candidate-list"):
            expect(
                shapes[name]["verdict"] == "DROP",
                f"{cell}: {name} was expected to be a DROP",
            )

    window = summary["operand_window"]
    expect(
        window["retained_frac"] >= 0.95,
        "the operand-window DROP is not backed by near-total retention",
    )
    graduates = [
        (cell, shape["name"])
        for cell, data in summary["cells"].items()
        for shape in data["shapes"]
        if shape["verdict"] == "GRADUATE"
    ]
    expect(
        len(set(name for _, name in graduates)) == 1,
        f"expected exactly one graduated shape, got {graduates}",
    )


def negative_controls(store):
    def mutate(edit):
        def apply(target):
            edit(target)
            return target

        return apply

    def set_path(path, value):
        def edit(store_copy):
            node = store_copy
            for key in path[:-1]:
                node = node[key]
            node[path[-1]] = value

        return edit

    def run(edit):
        store_copy = copy.deepcopy(store)
        edit(store_copy)
        summary = reduce.build(store_copy)
        run_assertions(summary, store_copy)

    controls = {
        "overhang kernel share 90%": set_path(
            [("base-classic-off", "probe"), "terms", "overhang", "kernel_ms"], 8500.0
        ),
        "overhang offset calls 1454": set_path(
            [
                ("base-classic-off", "probe"),
                "profile",
                "overhang_host",
                "scopes",
                0,
                "calls",
            ],
            1454,
        ),
        "overhang occupancy 8x": set_path(
            [
                ("base-classic-off", "probe"),
                "profile",
                "overhang_host",
                "scopes",
                0,
                "wall_ms",
            ],
            8000.0,
        ),
        "seam offset share 80%": set_path(
            [("base-classic-off", "probe"), "profile", "seam", "scopes", 1, "wall_ms"],
            7000.0,
        ),
        "seam candidate list 5 s": set_path(
            [("base-classic-off", "probe"), "profile", "seam", "scopes", 0, "wall_ms"],
            5000.0,
        ),
        "candidate count 2.5M": set_path(
            [("base-classic-off", "probe"), "terms", "seam_harvest", "candidates"],
            2500000,
        ),
        "probe output moved": set_path(
            [("base-classic-on", "probe"), "sha256"], "0" * 64
        ),
        "operand retention 0.50": set_path(
            [
                ("base-classic-off", "probe3"),
                "terms",
                "overhang_window",
                "retained_frac",
            ],
            0.50,
        ),
    }

    rejected = 0
    for name, edit in controls.items():
        try:
            run(edit)
        except AssertionError:
            rejected += 1
            print(f"  self-test {name}: OK")
        else:
            print(f"  self-test {name}: FAILED (control was not rejected)")
    return rejected, len(controls)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()

    store = base_store()
    derived = reduce.build(store)
    committed = json.loads((HERE / "summary.json").read_text(encoding="utf-8"))
    expect(derived == committed, "summary.json does not match a fresh derivation")
    run_assertions(derived, store)
    print("t84 common-floor evidence: OK")

    if args.self_test:
        rejected, total = negative_controls(store)
        if rejected != total:
            raise SystemExit("negative controls were not rejected")
        print(f"self-test: {rejected}/{total} negative controls rejected")


if __name__ == "__main__":
    main()
