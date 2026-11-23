#!/usr/bin/env python3
"""Checkout-only verification for the t95 exact-Z cross-section batching take.

Re-derives qualification / paired A/B / Phase 1 from the committed reduced rows
and probe logs, checks every claim the findings make, and rejects a set of
negative controls.

    python verify.py             # verify committed evidence
    python verify.py --self-test # verify + negative controls
"""

from __future__ import annotations

import argparse
import copy
import json

import reduce as t95

HERE = t95.HERE


def expect(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def load(name: str) -> dict:
    return json.loads((HERE / name).read_text(encoding="utf-8"))


def derive_all(proof=None, measure=None, phase1=None):
    proof = proof if proof is not None else t95.load_committed("proof-rows.json")
    measure = (
        measure if measure is not None else t95.load_committed("measure-rows.json")
    )
    phase1 = phase1 if phase1 is not None else t95.build_phase1()
    qualification = t95.build_qualification(proof)
    ab = t95.build_ab(measure)
    summary = t95.build_summary(qualification, ab, phase1)
    return qualification, ab, phase1, summary


def check_committed_matches() -> None:
    qualification, ab, phase1, summary = derive_all()
    expect(
        qualification == load("qualification.json"),
        "committed qualification.json does not re-derive",
    )
    expect(
        ab == load("ab-reduction.json"),
        "committed ab-reduction.json does not re-derive",
    )
    expect(phase1 == load("phase1.json"), "committed phase1.json does not re-derive")
    expect(summary == load("summary.json"), "committed summary.json does not re-derive")


def check_claims() -> None:
    qualification = load("qualification.json")
    ab = load("ab-reduction.json")
    phase1 = load("phase1.json")
    summary = load("summary.json")
    references = t95.load_references()

    expect(qualification["pass"], f"qualification failed: {qualification['problems']}")
    expect(len(qualification["cells"]) == 16, "16 proof cells required")
    expect(
        len(qualification["cross_mode"]) == 8
        and all(entry["equal"] for entry in qualification["cross_mode"].values()),
        "ordinary and accelerated bytes must agree in all 8 fixture cells",
    )
    for cell in qualification["cells"]:
        if cell["reference_sha256"] is not None:
            expect(
                cell["sha256"] == cell["reference_sha256"],
                f"{cell['cell']}: reference drift",
            )
        expect(
            len(set(cell["exe_sha256"].values())) == 2,
            f"{cell['cell']}: one arm binary",
        )
        for arm, sha in cell["exe_sha256"].items():
            expect(bool(sha), f"{cell['cell']}: missing {arm} exe sha")

    # Host-only: the module trees are byte-identical across arms, per mode.
    trees = {}
    for mode in t95.MODES:
        for arm in ("baseline", "candidate"):
            provenance = json.loads(
                (HERE / "provenance" / f"{mode}-{arm}.json").read_text(encoding="utf-8")
            )
            expect(
                provenance["arm"] == arm and provenance["mode"] == mode,
                "provenance label",
            )
            expect(
                provenance["module_count"] == 24, f"{mode}-{arm}: 24 modules expected"
            )
            trees[(mode, arm)] = [
                (module["name"], module["sha256"]) for module in provenance["modules"]
            ]
        expect(
            trees[(mode, "baseline")] == trees[(mode, "candidate")],
            f"{mode}: candidate changed a guest module",
        )
    exes = {
        json.loads((HERE / "provenance" / f"{mode}-{arm}.json").read_text())[
            "exe_sha256"
        ]
        for mode in t95.MODES
        for arm in ("baseline", "candidate")
    }
    expect(len(exes) == 4, "each mode/arm must ship its own host binary")

    expect(ab["verdict"] == "PASS", f"gate verdict {ab['verdict']}")
    expect(
        ab["wall_pass"] and ab["cpu_pass"] and ab["pairs_pass"] and ab["output_pass"],
        "gate clauses",
    )
    non_negative = []
    for cell in t95.MEASURE_CELLS:
        result = ab["cells"][cell]
        expect(result["status"] == "reduced", f"{cell}: {result['status']}")
        expect(result["retained_pairs"] == 6, f"{cell}: 6 pairs required")
        expect(not result["excluded_pairs"], f"{cell}: exclusions present")
        expect(
            result["wall_delta_pct_paired_median"] < 0.0,
            f"{cell}: wall did not fall",
        )
        expect(
            result["cpu_delta_pct_paired_median"] <= t95.CPU_CEILING_PCT,
            f"{cell}: CPU above the work-preserving ceiling",
        )
        expect(
            len({pair["sha256"] for pair in result["pairs"]}) == 1,
            f"{cell}: output changed between pairs",
        )
        non_negative.extend(
            (cell, pair["pair"], pair["wall_ratio"])
            for pair in result["pairs"]
            if pair["wall_ratio"] >= 1.0
        )
    # Disclosure pin: exactly one of the 24 pairs regressed, and it is the
    # accelerated-classic-on pair taken during that cell's second-half load
    # event (both arms run ~+10 s in that pair).
    expect(
        len(non_negative) == 1,
        f"expected exactly one non-negative pair, found {non_negative}",
    )
    expect(
        non_negative[0][0] == "accelerated-base-classic-on" and non_negative[0][1] == 4,
        f"the disclosed regression moved: {non_negative[0]}",
    )

    expect(phase1["floor_pct"] == 0.80, "floor is 0.80 %")
    expect(len(phase1["runs"]) >= 3, "at least three Phase-1 probe runs")
    for run in phase1["runs"]:
        expect(run["mismatch"] == 0, f"{run['cell']}: cross-section mismatch")
        expect(run["equal"] == run["distinct_z"], f"{run['cell']}: plane coverage")
        expect(run["equal"] == 431, f"{run['cell']}: 431 planes expected")
        expect(run_ok(run), f"{run['cell']}: batch row does not clear the floor")
        # The stored derived fields must equal the raw-number derivation.
        expect(
            run["saving_ns"] == run["single_ns_total"] - run["batch_ns"],
            f"{run['cell']}: saving_ns does not re-derive",
        )
        expect(
            abs(run["share_pct"] - run["saving_ns"] / 1e9 / run["cell_wall_s"] * 100.0)
            < 1e-9,
            f"{run['cell']}: share_pct does not re-derive",
        )
        expect(run["margin_x"] > 1.0, f"{run['cell']}: no floor margin")
    classics = [
        run for run in phase1["runs"] if run["cell"] == "ordinary-base-classic-on"
    ]
    expect(
        len({run["cell"] for run in phase1["runs"]}) == 2,
        "both screening cells measured",
    )
    expect(len(classics) == 2, "classic cell measured twice")
    spread = max(run["share_pct"] for run in classics) / min(
        run["share_pct"] for run in classics
    )
    expect(spread < 1.15, f"classic repeat spread {spread:.3f} too wide")
    expect(summary["headline"]["gate_verdict"] == "PASS", "summary gate verdict")


def run_ok(run: dict) -> bool:
    """Re-derive a Phase-1 run's saving from its raw numbers, never its fields."""
    saving_ns = run["single_ns_total"] - run["batch_ns"]
    share = saving_ns / 1e9 / run["cell_wall_s"] * 100.0
    return (
        run["equal"] == 431
        and run["mismatch"] == 0
        and saving_ns > 0
        and share > t95.FLOOR_PCT
    )


def _mutate_rows(rows: dict, cell: str, arm: str, field: str, value) -> dict:
    mutated = copy.deepcopy(rows)
    for row in mutated[cell]:
        if row["arm"] == arm:
            row[field] = value
    return mutated


def run_negative_controls() -> None:
    proof = t95.load_committed("proof-rows.json")
    measure = t95.load_committed("measure-rows.json")
    phase1 = t95.build_phase1()
    cell = t95.PROOF_CELLS[0]

    def must_fail(label: str, **kwargs) -> None:
        try:
            qualification, ab, phase1_result, summary = derive_all(**kwargs)
        except AssertionError:
            return
        ok = (
            qualification["pass"]
            and ab["verdict"] == "PASS"
            and all(run_ok(run) for run in phase1_result["runs"])
        )
        if ok:
            raise AssertionError(f"negative control not rejected: {label}")

    # A. one proof cell's candidate output drifts
    mutated = copy.deepcopy(proof)
    for row in mutated[cell]:
        if row["arm"] == "candidate":
            row["sha256"] = "0" * 64
    must_fail("proof output drift", proof=mutated)

    # B. one proof cell reports a fatal error
    mutated = _mutate_rows(proof, cell, "baseline", "fatal", 1)
    must_fail("proof fatal error", proof=mutated)

    # C. a measure pair is missing
    mutated = copy.deepcopy(measure)
    mutated[t95.MEASURE_CELLS[0]] = [
        row
        for row in mutated[t95.MEASURE_CELLS[0]]
        if not (row["arm"] == "candidate" and row["pair"] == 3)
    ]
    must_fail("missing measure pair", measure=mutated)

    # D. a candidate wall regresses
    mutated = _mutate_rows(
        measure, t95.MEASURE_CELLS[0], "candidate", "wall_seconds", 10_000.0
    )
    must_fail("wall regression", measure=mutated)

    # E. a candidate CPU breaches the work-preserving ceiling
    mutated = copy.deepcopy(measure)
    target = t95.MEASURE_CELLS[0]
    for row in mutated[target]:
        if row["arm"] == "candidate":
            row["cpu_seconds"] = row["cpu_seconds"] * 1.10
    must_fail("CPU ceiling breach", measure=mutated)

    # F. one cross-section plane mismatches
    mutated_phase1 = copy.deepcopy(phase1)
    mutated_phase1["runs"][0]["equal"] = 430
    mutated_phase1["runs"][0]["mismatch"] = 1
    must_fail("cross-section mismatch", phase1=mutated_phase1)

    # G. the batched walk is not cheaper
    mutated_phase1 = copy.deepcopy(phase1)
    mutated_phase1["runs"][0]["batch_ns"] = mutated_phase1["runs"][0]["single_ns_total"]
    must_fail("batch not cheaper", phase1=mutated_phase1)


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args(argv)
    check_committed_matches()
    check_claims()
    print("verify: committed evidence re-derives; every claim holds")
    if args.self_test:
        run_negative_controls()
        print("verify: self-test rejected 7 of 7 negative controls")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
