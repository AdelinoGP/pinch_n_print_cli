#!/usr/bin/env python3
"""Checkout-only verification for the t91 exact-Z object-geometry memo take.

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


def load_provenance() -> dict:
    path = HERE / "provenance.json"
    if not path.exists():
        return {}
    return json.loads(path.read_text(encoding="utf-8"))


def run_assertions(
    proof_rows, measure_rows, qualification, ab, summary, attribution, provenance
):
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
            cell["exe_sha256"]["baseline"] != cell["exe_sha256"]["candidate"],
            f"{cell['cell']}: both arms ran the same host binary",
        )
    expect(
        len(qualification["cross_mode"]) == 8,
        "expected 8 ordinary/accelerated cross-mode comparisons",
    )

    # 3. The candidate is host-only: both arms ship byte-identical module trees.
    expect(provenance, "provenance.json is absent")
    expect(
        provenance["module_trees_equal"]
        == {
            "ordinary": True,
            "accelerated": True,
        },
        "the two arms do not share byte-identical module trees",
    )
    for mode in ("ordinary", "accelerated"):
        base = provenance["snapshots"][f"{mode}-baseline"]
        cand = provenance["snapshots"][f"{mode}-candidate"]
        expect(
            base["module_count"] == cand["module_count"] == 24,
            f"{mode}: expected 24 external modules per snapshot",
        )
        expect(
            base["candidate_sources_dirty"] == [],
            f"{mode}: baseline arm was built from a dirty candidate tree",
        )
        expect(
            cand["candidate_sources_dirty"],
            f"{mode}: candidate arm was not built with the candidate sources",
        )

    # 4. The paired gate: four cells, wall down in every one, every retained
    #    pair byte-identical, six pairs retained. The CPU clause is the
    #    ticket's "flat or lower": the committed result fails it in exactly one
    #    cell, so the verdict is GATE NOT MET and the recommendation returns to
    #    the human. The verifier pins that measured state; it does not read the
    #    clause as pass. The 0.5 pp bound below is a magnitude pin on the
    #    measured miss (0.08 pp), not an acceptance threshold.
    expect(len(ab["gate_cells"]) == 4, "expected four pre-declared gate cells")
    cpu_positive = []
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
            all(pair["sha_equal"] for pair in result["pairs"]),
            f"{cell}: a retained pair is not byte-identical",
        )
        expect(
            result["deterministic"],
            f"{cell}: an arm produced more than one G-code SHA across its repeats",
        )
        expect(
            result["arms_distinct_binaries"],
            f"{cell}: the two arms did not run distinct host binaries",
        )
        expect(
            result["candidate"]["wall_median_s"] < result["baseline"]["wall_median_s"],
            f"{cell}: candidate wall median is not below baseline",
        )
        if result["cpu_delta_pct_paired_median"] > 0.0:
            cpu_positive.append(cell)
            expect(
                result["cpu_delta_pct_paired_median"] < 0.5,
                f"{cell}: CPU excess is not the measured sub-noise miss",
            )
    expect(
        cpu_positive == ["accelerated-base-classic-on"],
        f"CPU-positive cells changed: {cpu_positive}",
    )
    expect(
        summary["cpu_flat_or_lower_in_all_cells"] is False,
        "the CPU clause is reported as met; the measured result does not meet it",
    )
    expect(
        summary["verdict"].startswith("GATE NOT MET"),
        f"gate verdict: {summary['verdict']}",
    )

    # 5. Attribution: the bracket that carries the exact-Z validation tail moved.
    #    (a) the support-geometry stage and the planner module wall are present
    #        in both arms, so the gap is a difference of two measured terms;
    #    (b) the baseline's post-planner gap is the t88-sized tail, not a stub;
    #    (c) the candidate's gap falls on every measured cell.
    cells = attribution.get("cells", {})
    expect(cells, "attribution.json carries no cells")
    measured = [cell for cell in ab["gate_cells"]]
    for cell in measured:
        arms = cells.get(cell)
        expect(arms is not None, f"{cell}: no attribution capture")
        base = arms["baseline"]
        cand = arms["candidate"]
        expect(
            arms["post_planner_gap_drop_ms"] > 0.0,
            f"{cell}: candidate post-planner gap is not below baseline",
        )
        expect(
            base["post_planner_gap_ms"] > 1000.0,
            f"{cell}: baseline post-planner gap is missing the validation tail",
        )
        expect(
            cand["post_planner_gap_ms"] < base["post_planner_gap_ms"],
            f"{cell}: candidate still pays the full validation tail",
        )
        for arm, terms in (("baseline", base), ("candidate", cand)):
            expect(
                terms["support_geometry_stage_ms"]
                and terms["support_geometry_stage_ms"] > 1000.0,
                f"{cell}: {arm} support-geometry stage bracket is missing",
            )
            expect(
                terms["planner_module_ms"] and terms["planner_module_ms"] > 1000.0,
                f"{cell}: {arm} support planner module wall is missing",
            )
            expect(
                terms["slice_ms"] and terms["slice_ms"] > 0.0,
                f"{cell}: {arm} slice wall is missing",
            )


def assert_memo_probe(attribution: dict) -> None:
    """The probe must show one geometry build where the baseline builds 431."""
    probe = attribution.get("memo_probe", {})
    expect(probe.get("available") is True, "memo_probe evidence is absent")
    base = probe["arms"]["baseline"]
    cand = probe["arms"]["candidate"]
    expect(base["queries"] == cand["queries"] == 431, "probe query counts differ")
    expect(
        base["builds"] == base["misses"] == base["queries"],
        "baseline did not build the object terms on every miss",
    )
    expect(cand["builds"] == 1, "candidate did not build the object terms once")
    expect(
        base["validate_calls"] == cand["validate_calls"] == 431,
        "probe validate call counts differ",
    )
    expect(
        cand["query_ms"] < base["query_ms"],
        "candidate exact-Z query time did not fall",
    )
    expect(
        probe["query_ms_drop_pct"] > 50.0,
        "candidate exact-Z query-time drop is not the measured majority",
    )
    expect(probe["validate_ms_drop"] > 0.0, "validate loop did not fall")


def negative_controls(proof_rows, measure_rows, attribution, provenance):
    def run(proof_edit=None, measure_edit=None, attr_edit=None, prov_edit=None):
        proof_copy = copy.deepcopy(proof_rows)
        measure_copy = copy.deepcopy(measure_rows)
        attr_copy = copy.deepcopy(attribution)
        prov_copy = copy.deepcopy(provenance)
        if proof_edit is not None:
            proof_edit(proof_copy)
        if measure_edit is not None:
            measure_edit(measure_copy)
        if attr_edit is not None:
            attr_edit(attr_copy)
        if prov_edit is not None:
            prov_edit(prov_copy)
        qualification = reduce.build_qualification(proof_copy)
        ab = reduce.build_ab(measure_copy)
        summary = reduce.build_summary(qualification, ab, attr_copy)
        assert_memo_probe(attr_copy)
        run_assertions(
            proof_copy,
            measure_copy,
            qualification,
            ab,
            summary,
            attr_copy,
            prov_copy,
        )

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

    def collapse_gap(store):
        for arms in store["cells"].values():
            arms["baseline"]["post_planner_gap_ms"] = 1.0
            return
        raise AssertionError("negative control found no attribution cell")

    def invert_gap_drop(store):
        for arms in store["cells"].values():
            if "post_planner_gap_drop_ms" in arms:
                arms["post_planner_gap_drop_ms"] = -abs(
                    arms["post_planner_gap_drop_ms"]
                )
                return
        raise AssertionError("negative control found no gap drop")

    def diverge_module_trees(store):
        store["module_trees_equal"]["ordinary"] = False

    controls = {
        "candidate output moved": (
            run,
            {
                "proof_edit": set_proof(
                    "ordinary-base-classic-on", 1, "sha256", "0" * 64
                )
            },
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
            {
                "proof_edit": set_proof(
                    "accelerated-base-classic-on", 0, "degraded", "True"
                )
            },
        ),
        "both arms the same binary": (
            run,
            {
                "proof_edit": set_proof(
                    "ordinary-base-classic-on",
                    1,
                    "exe_sha256",
                    proof_rows["ordinary-base-classic-on"][0]["exe_sha256"],
                )
            },
        ),
        "wall delta made positive": (
            run,
            {
                "measure_edit": scale_arm(
                    "ordinary-base-classic-on", "candidate", "wall_seconds", 1.5
                )
            },
        ),
        "cpu above the ceiling": (
            run,
            {
                "measure_edit": scale_arm(
                    "accelerated-base-arachne-on", "candidate", "cpu_seconds", 1.1
                )
            },
        ),
        "pair output mismatch": (
            run,
            {
                "measure_edit": set_measure(
                    "ordinary-base-arachne-on",
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
                    "ordinary-base-classic-on",
                    lambda row: row["arm"] == "candidate" and row["pair"] == "4",
                    "sha256",
                    "3" * 64,
                )
            },
        ),
        "baseline validation tail erased": (run, {"attr_edit": collapse_gap}),
        "gap drop inverted": (run, {"attr_edit": invert_gap_drop}),
        "module trees diverge": (run, {"prov_edit": diverge_module_trees}),
        "memo rebuilt per query": (
            run,
            {
                "attr_edit": lambda store: store["memo_probe"]["arms"][
                    "candidate"
                ].update({"builds": 2})
            },
        ),
        "query-time drop erased": (
            run,
            {
                "attr_edit": lambda store: store["memo_probe"]["arms"][
                    "candidate"
                ].update(
                    {"query_ms": store["memo_probe"]["arms"]["baseline"]["query_ms"]}
                )
            },
        ),
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
    provenance = load_provenance()

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

    assert_memo_probe(attribution)
    run_assertions(
        proof_rows, measure_rows, qualification, ab, summary, attribution, provenance
    )
    print("t91 exact-Z object-geometry memo evidence: OK")

    if args.self_test:
        rejected, total = negative_controls(
            proof_rows, measure_rows, attribution, provenance
        )
        if rejected != total:
            raise SystemExit("negative controls were not rejected")
        print(f"self-test: {rejected}/{total} negative controls rejected")


if __name__ == "__main__":
    main()
