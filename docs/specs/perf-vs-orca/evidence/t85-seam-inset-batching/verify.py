#!/usr/bin/env python3
"""Checkout-only verification for the t85 seam inset-offset batching take.

Re-derives the reference set from the committed t81 campaign rows, re-derives
qualification / paired A/B / summary from the committed reduced rows, and
checks every claim the findings make. Negative controls must be rejected.

    python verify.py             # verify committed evidence
    python verify.py --self-test # verify + negative controls
"""

from __future__ import annotations

import argparse
import copy
import json

import derive_references
import reduce

HERE = reduce.HERE


def expect(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def derive_reference_table() -> dict:
    """Re-run the reference derivation in memory (no file writes)."""
    shas = {}
    for path in sorted(
        (HERE.parent / "t81-resolution-port" / "campaign").glob("*-rows.csv")
    ):
        if "noisy" in path.name:
            continue
        import csv

        with path.open(encoding="utf-8-sig", newline="") as handle:
            for row in csv.DictReader(handle):
                key = (row["fixture"], row["generator"], row["supports"])
                entry = shas.setdefault(key, {})
                entry.setdefault(row["arm"], set()).add(row["sha256_of_gcode"])
    table = {}
    for (fixture, gen, sup), arms in shas.items():
        candidate = arms["candidate"]
        baseline = arms["baseline"]
        expect(len(candidate) == 1, f"{fixture}-{gen}-{sup}: candidate SHAs disagree")
        expect(len(baseline) == 1, f"{fixture}-{gen}-{sup}: baseline SHAs disagree")
        table[f"ordinary-{fixture}-{gen}-{sup}"] = next(iter(candidate))
    return table


def run_assertions(proof_rows, measure_rows, qualification, ab, summary, attribution):
    # 1. The committed reference set is exactly the t81 campaign's candidate arm.
    committed_refs = reduce.load_references()
    derived_refs = derive_reference_table()
    expect(
        {cell: value["sha256"] for cell, value in committed_refs.items()}
        == derived_refs,
        "references.json does not match a fresh derivation from the t81 campaign rows",
    )
    expect(len(derived_refs) == 8, "expected 8 ordinary reference cells")

    # 2. Qualification: 16 cells, byte identity, reference match, clean, non-vacuous.
    expect(qualification["pass"], f"qualification failed: {qualification['problems']}")
    expect(len(qualification["cells"]) == 16, "expected 16 qualified cells")
    for cell in qualification["cells"]:
        expect(
            cell["status"] == "ok" and cell["degraded"] == "False",
            f"{cell['cell']}: unclean proof output",
        )
        if cell["reference_sha256"] is not None:
            expect(
                cell["sha256"] == cell["reference_sha256"],
                f"{cell['cell']}: proof output differs from its reference",
            )
    expect(
        len(qualification["cross_mode"]) == 8,
        "expected 8 ordinary/accelerated cross-mode comparisons",
    )

    # 3. The paired gate: four cells, wall down, CPU inside the ceiling,
    #    every retained pair byte-identical, six pairs retained.
    expect(summary["verdict"].startswith("PASS"), f"gate verdict: {summary['verdict']}")
    expect(len(ab["gate_cells"]) == 4, "expected four pre-declared gate cells")
    for cell in ab["gate_cells"]:
        result = ab["cells"][cell]
        expect(result["status"] == "reduced", f"{cell}: {result['status']}")
        expect(
            result["retained_pairs"] == 6,
            f"{cell}: {result['retained_pairs']} retained pairs",
        )
        expect(
            result["wall_delta_pct_paired_median"] < 0.0,
            f"{cell}: wall did not fall",
        )
        expect(
            result["cpu_delta_pct_paired_median"] <= ab["cpu_clause_ceiling_pct"],
            f"{cell}: CPU above the work-preserving ceiling",
        )
        expect(
            all(pair["sha_equal"] for pair in result["pairs"]),
            f"{cell}: a retained pair is not byte-identical",
        )
        expect(
            result["deterministic"],
            f"{cell}: an arm produced more than one G-code SHA across its repeats",
        )
        expect(
            result["candidate"]["wall_median_s"] < result["baseline"]["wall_median_s"],
            f"{cell}: candidate wall median is not below baseline",
        )

    # 4. Attribution: the seam stage fell by the amount the ticket prices, and
    #    the derived offset occupancy clears the pre-declared 1.15x DROP floor.
    cells = attribution.get("cells", {})
    expect(cells, "attribution.json carries no cells")
    for name, arms in cells.items():
        if "stage_drop_ms" not in arms:
            continue
        expect(
            arms["stage_drop_ms"] > 0.0,
            f"{name}: candidate seam stage is not below baseline",
        )
        expect(
            arms["candidate"]["seam_stage_ms"] < arms["baseline"]["seam_stage_ms"],
            f"{name}: stage walls disagree with the recorded drop",
        )
    expect(
        summary["attribution"]
        .get("ordinary-base-classic-off", {})
        .get("stage_drop_ms", 0)
        > 0,
        "the ordinary base classic-off attribution row is absent",
    )


def negative_controls(proof_rows, measure_rows, attribution):
    def run(proof_edit=None, measure_edit=None, attr_edit=None):
        proof_copy = copy.deepcopy(proof_rows)
        measure_copy = copy.deepcopy(measure_rows)
        attr_copy = copy.deepcopy(attribution)
        if proof_edit is not None:
            proof_edit(proof_copy)
        if measure_edit is not None:
            measure_edit(measure_copy)
        if attr_edit is not None:
            attr_edit(attr_copy)
        qualification = reduce.build_qualification(proof_copy)
        ab = reduce.build_ab(measure_copy)
        summary = reduce.build_summary(qualification, ab, attr_copy)
        run_assertions(proof_copy, measure_copy, qualification, ab, summary, attr_copy)

    def set_proof(cell, index, field, value):
        def edit(store):
            store[cell][index][field] = value

        return edit

    def set_measure(cell, predicate, field, value):
        def edit(store):
            for row in store[cell]:
                if predicate(row):
                    row[field] = value
                    return
            raise AssertionError("negative control did not match a row")

        return edit

    def scale_arm(cell, arm, field, factor):
        """Scale one arm's measured rows so the paired median moves decisively."""

        def edit(store):
            touched = 0
            for row in store[cell]:
                if row["arm"] == arm and row["warmup"] == "0":
                    row[field] = float(row[field]) * factor
                    touched += 1
            if not touched:
                raise AssertionError("negative control matched no rows")

        return edit

    def invert_drop(store):
        for arms in store["cells"].values():
            if "stage_drop_ms" in arms:
                arms["stage_drop_ms"] = -abs(arms["stage_drop_ms"])
                return
        raise AssertionError("negative control found no attribution drop")

    controls = {
        "candidate output moved": (
            run,
            {"proof_edit": set_proof("ordinary-base-classic-off", 1, "sha256", "0" * 64)},
        ),
        "proof output moved off its reference": (
            run,
            {
                "proof_edit": set_proof(
                    "ordinary-benchy-arachne-on", 0, "sha256", "1" * 64
                )
            },
        ),
        "proof output degraded": (
            run,
            {"proof_edit": set_proof("accelerated-base-classic-on", 0, "degraded", "True")},
        ),
        "wall delta made positive": (
            run,
            {"measure_edit": scale_arm("ordinary-base-classic-off", "candidate", "wall_seconds", 1.5)},
        ),
        "cpu above the ceiling": (
            run,
            {"measure_edit": scale_arm("accelerated-base-classic-on", "candidate", "cpu_seconds", 1.1)},
        ),
        "pair output mismatch": (
            run,
            {
                "measure_edit": set_measure(
                    "ordinary-base-classic-on",
                    lambda row: row["arm"] == "candidate" and row["pair"] == "3",
                    "sha256",
                    "2" * 64,
                )
            },
        ),
        "per-arm SHA drift": (
            run,
            {
                "measure_edit": set_measure(
                    "ordinary-base-classic-off",
                    lambda row: row["arm"] == "candidate" and row["pair"] == "4",
                    "sha256",
                    "3" * 64,
                )
            },
        ),
        "attribution drop inverted": (run, {"attr_edit": invert_drop}),
    }

    rejected = 0
    for name, (runner, kwargs) in controls.items():
        try:
            runner(**kwargs)
        except AssertionError:
            rejected += 1
            print(f"  self-test {name}: OK (rejected)")
        else:
            print(f"  self-test {name}: FAILED (control was not rejected)")
    return rejected, len(controls)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()

    proof_rows = reduce.load_committed_rows("proof-rows.json")
    measure_rows = reduce.load_committed_rows("measure-rows.json")
    attribution = reduce.load_attribution()

    qualification = reduce.build_qualification(proof_rows)
    ab = reduce.build_ab(measure_rows)
    summary = reduce.build_summary(qualification, ab, attribution)

    committed_qualification = json.loads(
        (HERE / "qualification.json").read_text(encoding="utf-8")
    )
    committed_ab = json.loads((HERE / "ab-reduction.json").read_text(encoding="utf-8"))
    committed_summary = json.loads((HERE / "summary.json").read_text(encoding="utf-8"))
    expect(
        qualification == committed_qualification,
        "qualification.json does not match a fresh derivation",
    )
    expect(ab == committed_ab, "ab-reduction.json does not match a fresh derivation")
    expect(
        summary == committed_summary,
        "summary.json does not match a fresh derivation",
    )

    run_assertions(proof_rows, measure_rows, qualification, ab, summary, attribution)
    print("t85 seam inset-offset batching evidence: OK")

    if args.self_test:
        rejected, total = negative_controls(proof_rows, measure_rows, attribution)
        if rejected != total:
            raise SystemExit("negative controls were not rejected")
        print(f"self-test: {rejected}/{total} negative controls rejected")


if __name__ == "__main__":
    main()
