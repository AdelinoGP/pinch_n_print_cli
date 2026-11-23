#!/usr/bin/env python3
"""Checkout-only verification for the t89 parallel exact-Z validation take.

Re-derives the reference set from the committed t81 campaign rows, re-derives
qualification / pre-gate / summary from the committed reduced rows, and checks
every claim the findings make. Negative controls must be rejected.

    python verify.py             # verify committed evidence
    python verify.py --self-test # verify + negative controls

This take's measured outcome is a **gate failure**, not a win: the parallel
fold is exact (16/16 byte identity) and lowers wall, but process CPU rises by
more than twice the pre-declared work-preserving ceiling. The verifier asserts
that measured failure, so the evidence cannot quietly drift into a "PASS".
"""

from __future__ import annotations

import argparse
import copy
import csv
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


def run_assertions(proof_rows, pregate_rows, qualification, pregate, summary, probe):
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
    expect(
        not [
            key
            for key, value in qualification["cross_mode"].items()
            if not value["equal"]
        ],
        "ordinary and accelerated arms disagree on output for some cell",
    )

    # 3. The pre-gate: the win is real, and the CPU clause is the reason it fails.
    expect(pregate["status"] == "reduced", "pre-gate rows are missing")
    expect(pregate["retained_pairs"] == 3, "expected three retained pre-gate pairs")
    expect(
        all(pair["sha_equal"] for pair in pregate["pairs"]),
        "a pre-gate pair is not byte-identical",
    )
    expect(
        pregate["wall_delta_pct_paired_median"] < 0.0,
        "wall did not fall, so the CPU finding cannot be attributed to the fold",
    )
    expect(
        pregate["cpu_delta_pct_paired_median"] > pregate["cpu_clause_ceiling_pct"],
        "process CPU did not exceed the work-preserving ceiling; the take's "
        "measured DROP would no longer be justified",
    )
    expect(
        pregate["wall_clause"] == "pass"
        and pregate["cpu_clause"] == "fail"
        and pregate["output_identity_clause"] == "pass",
        f"pre-gate clauses disagree with the measured numbers: {pregate}",
    )
    expect(
        "GATE NOT MET" in summary["verdict"],
        "summary must carry the measured gate failure, never a PASS",
    )
    expect(
        summary["pregate"]["wall_delta_pct_paired_median"]
        == pregate["wall_delta_pct_paired_median"],
        "summary and pre-gate disagree on the wall delta",
    )
    expect(
        "not run" in summary["standing_campaign"],
        "the four-cell campaign must be disclosed as not run",
    )

    # 4. The probe: no dedupe headroom, and the split terms are the ones claimed.
    counters = probe["counters"]
    expect(
        counters["entries"] == counters["distinct_object_z"],
        "an object-level (object, z) cache would have headroom; the findings "
        "claim it has none",
    )
    expect(
        counters["queries"] == counters["misses"] == counters["entries"],
        "query/miss counts disagree with one distinct key per entry",
    )
    shares = probe["split"]["shares"]
    expect(abs(sum(shares.values()) - 1.0) < 1e-9, "split shares do not sum to one")
    expect(
        shares["z_section_us"] > 0.45,
        "the requested-Z cross-section is no longer the largest term",
    )
    expect(
        shares["top_section_us"] > 0.35,
        "the constant top-Z cross-section is no longer a material share, so the "
        "memo follow-up's premise would be unsupported",
    )
    expect(
        shares["mesh_us"] < 0.10,
        "the world-mesh build is a larger share than the findings claim",
    )


def negative_controls(proof_rows, pregate_rows, probe):
    def run(proof_edit=None, pregate_edit=None, probe_edit=None):
        proof_copy = copy.deepcopy(proof_rows)
        pregate_copy = copy.deepcopy(pregate_rows)
        probe_copy = copy.deepcopy(probe)
        if proof_edit is not None:
            proof_edit(proof_copy)
        if pregate_edit is not None:
            pregate_edit(pregate_copy)
        if probe_edit is not None:
            probe_edit(probe_copy)
        qualification = reduce.build_qualification(proof_copy)
        pregate = reduce.build_pregate(pregate_copy)
        summary = reduce.build_summary(qualification, pregate, probe_copy)
        run_assertions(
            proof_copy, pregate_copy, qualification, pregate, summary, probe_copy
        )

    def set_proof(cell, index, field, value):
        def edit(store):
            store[cell][index][field] = value

        return edit

    def set_row(predicate, field, value):
        def edit(store):
            for row in store:
                if predicate(row):
                    row[field] = value
                    return
            raise AssertionError("negative control did not match a row")

        return edit

    def scale(arm, field, factor):
        def edit(store):
            touched = 0
            for row in store:
                if row["arm"] == arm and row["warmup"] == "0":
                    row[field] = float(row[field]) * factor
                    touched += 1
            if not touched:
                raise AssertionError("negative control matched no rows")

        return edit

    def break_headroom(store):
        store["counters"]["distinct_object_z"] = store["counters"]["entries"] * 2

    def inflate_mesh(store):
        store["split"]["shares"]["mesh_us"] = 0.5

    controls = {
        "candidate output moved": (
            run,
            {
                "proof_edit": set_proof(
                    "ordinary-base-classic-off", 1, "sha256", "0" * 64
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
        "pre-gate wall made positive": (
            run,
            {"pregate_edit": scale("candidate", "wall_seconds", 1.5)},
        ),
        "pre-gate CPU brought under the ceiling": (
            run,
            {"pregate_edit": scale("candidate", "cpu_seconds", 0.5)},
        ),
        "pre-gate pair output mismatch": (
            run,
            {
                "pregate_edit": set_row(
                    lambda row: row["arm"] == "candidate" and row["pair"] == "2",
                    "sha256",
                    "2" * 64,
                )
            },
        ),
        "probe claims dedupe headroom": (run, {"probe_edit": break_headroom}),
        "probe split inflated": (run, {"probe_edit": inflate_mesh}),
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
    pregate_rows = json.loads((HERE / "pregate-rows.json").read_text(encoding="utf-8"))
    probe = json.loads((HERE / "probe.json").read_text(encoding="utf-8"))

    qualification = reduce.build_qualification(proof_rows)
    pregate = reduce.build_pregate(pregate_rows)
    summary = reduce.build_summary(qualification, pregate, probe)

    committed_qualification = json.loads(
        (HERE / "qualification.json").read_text(encoding="utf-8")
    )
    committed_pregate = json.loads((HERE / "pregate.json").read_text(encoding="utf-8"))
    committed_summary = json.loads((HERE / "summary.json").read_text(encoding="utf-8"))
    expect(
        qualification == committed_qualification,
        "qualification.json does not match a fresh derivation",
    )
    expect(
        pregate == committed_pregate,
        "pregate.json does not match a fresh derivation",
    )
    expect(
        summary == committed_summary,
        "summary.json does not match a fresh derivation",
    )

    run_assertions(proof_rows, pregate_rows, qualification, pregate, summary, probe)
    print("t89 parallel exact-Z validation evidence: OK (measured gate failure)")

    if args.self_test:
        rejected, total = negative_controls(proof_rows, pregate_rows, probe)
        if rejected != total:
            raise SystemExit("negative controls were not rejected")
        print(f"self-test: {rejected}/{total} negative controls rejected")


if __name__ == "__main__":
    main()
