#!/usr/bin/env python3
"""t112 prep-slice parity Phase 1: reduced-set verifier.

Exits 0 when the committed reduced evidence under ``reduced/`` is internally
consistent and supports the resolution's headline relations. Reads only the
in-repo reduced set (no gitignored raw tree required).

    python docs/specs/perf-vs-orca/evidence/t112-prep-slice-parity/verify.py
    python .../verify.py --self-test     # adds the negative-control sweep
"""

from __future__ import annotations

import copy
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
REDUCED = HERE / "reduced"

FIXTURES = ("benchy", "base")

# Headline claims this evidence makes. Re-derived from reduced/ below; kept
# here only as the assertion targets so a corrupted payload cannot silently
# redefine them.
CLAIMS = {
    "min_floor_multiple": 19.08,
    "max_floor_multiple": 62.49,
    "benchy_layers_changed": 182,
    "base_layers_changed": 437,
    "benchy_net_area_mm2": 8.86,
    "base_net_area_mm2": 115.90,
}


def approx(a: float, b: float, tol: float) -> bool:
    return abs(a - b) <= tol


def load() -> dict:
    data = {"summaries": {}, "profiles": {}, "ceilings": None}
    for fixture in FIXTURES:
        data["summaries"][fixture] = json.loads(
            (REDUCED / f"summary-{fixture}.json").read_text(encoding="utf-8")
        )
        data["profiles"][fixture] = json.loads(
            (REDUCED / f"profile-{fixture}.json").read_text(encoding="utf-8")
        )
    data["ceilings"] = json.loads(
        (REDUCED / "ceilings.json").read_text(encoding="utf-8")
    )
    return data


def check(data: dict) -> list[str]:
    failures: list[str] = []

    def ck(cond: bool, msg: str) -> None:
        if not cond:
            failures.append(msg)

    for fixture in FIXTURES:
        summary = data["summaries"][fixture]
        rows = summary["layers"]

        ck(
            len(rows) == summary["closing_calls"],
            f"{fixture}: per-layer rows != closing_calls",
        )
        ck(
            summary["closing_calls"] > 200,
            f"{fixture}: implausibly few closing calls",
        )
        for key in (
            "n_comp",
            "prod_emitted",
            "open_emitted",
        ):
            total = sum(int(r[key]) for r in rows)
            stored = {
                "n_comp": summary["components_total"],
                "prod_emitted": summary["prod_emitted_total"],
                "open_emitted": summary["open_components_total"],
            }[key]
            ck(total == stored, f"{fixture}: {key} total {total} != stored {stored}")

        # Per-row values carry 4 decimal places, so recomputed sums drift at
        # most 0.00005 per row per column.
        tol = 0.1 * len(rows) + 0.5
        for key, stored_key in (
            ("prod_area", "prod_area_mm2"),
            ("flat_area", "flat_area_mm2"),
            ("open_area", "open_area_mm2"),
            ("canon_area", "canon_area_mm2"),
            ("close_ms", "close_ms_sum"),
            ("open_ms", "open_ms_sum"),
            ("canon_ms", "canon_ms_sum"),
        ):
            total = sum(float(r[key]) for r in rows)
            ck(
                approx(total, float(summary[stored_key]), tol),
                f"{fixture}: {stored_key} sum {total:.4f} != {summary[stored_key]}",
            )

        ck(
            summary["differ_layers"] == sum(1 for r in rows if int(r["differ"]) == 1),
            f"{fixture}: differ_layers disagrees with rows",
        )
        ck(
            summary["open_gt_prod_layers"] + summary["open_lt_prod_layers"]
            >= summary["differ_layers"],
            f"{fixture}: direction counts cannot be fewer than differing layers",
        )

        # The gate emits a subset of the flat family, and the canonical opening
        # shaves the family but keeps more area than the gate did.
        ck(
            summary["prod_area_mm2"] <= summary["flat_area_mm2"] + 1e-6,
            f"{fixture}: gate area exceeds the flat family",
        )
        ck(
            summary["open_area_mm2"] <= summary["flat_area_mm2"] + 1e-6,
            f"{fixture}: opened area exceeds the flat family",
        )
        ck(
            summary["open_area_mm2"] > summary["prod_area_mm2"],
            f"{fixture}: P1 does not add area over the gate",
        )

        # The opening must be orders of magnitude cheaper than the closing it
        # replaces — this is the price half of the shape.
        ratio = summary["close_ms_sum"] / summary["open_ms_sum"]
        ck(
            ratio > 50.0,
            f"{fixture}: canonical opening is only {ratio:.1f}x cheaper than the closing",
        )

        # The canonical classification set (P2) is much larger than the flat
        # family, which is why its delta cannot be bounded from this scan.
        ck(
            summary["canon_area_mm2"] > 2.0 * summary["flat_area_mm2"],
            f"{fixture}: P2 classification set is not materially larger than the flat family",
        )

        profile = data["profiles"][fixture]
        ck(profile["slice_activations"] > 1000, f"{fixture}: too few slice activations")
        ck(
            profile["closing_thread_ms"] < profile["slice_thread_ms"],
            f"{fixture}: closing thread-time exceeds the stage thread-time",
        )
        ck(
            profile["closing_calls"] == summary["closing_calls"],
            f"{fixture}: production and probe closing-call counts differ "
            f"({profile['closing_calls']} vs {summary['closing_calls']})",
        )
        ck(
            profile["stage_wall_ms"] > 0,
            f"{fixture}: stage wall missing",
        )

    ceilings = data["ceilings"]
    cells = ceilings["cells"]
    ck(len(cells) == 8, "ceilings: expected eight cells")

    by_name = {c["cell"]: c for c in cells}
    expected_cells = {
        "benchy-classic-off",
        "benchy-arachne-off",
        "benchy-classic-on",
        "benchy-arachne-on",
        "base-classic-off",
        "base-arachne-off",
        "base-classic-on",
        "base-arachne-on",
    }
    ck(
        set(by_name) == expected_cells,
        "ceilings: cell set is not the eight-cell matrix",
    )

    for cell in cells:
        fixture = cell["fixture"]
        summary = data["summaries"][fixture]
        profile = data["profiles"][fixture]

        ck(
            approx(cell["closing_thread_ms"], profile["closing_thread_ms"], 1e-6),
            f"{cell['cell']}: closing thread-time != profile",
        )
        ck(
            cell["slice_stage_wall_ms"] == profile["stage_wall_ms"],
            f"{cell['cell']}: stage wall != profile",
        )
        ck(
            cell["slice_thread_ms"] == profile["slice_thread_ms"],
            f"{cell['cell']}: stage thread-time != profile",
        )
        ck(
            approx(cell["opening_thread_ms_p1"], summary["open_ms_sum"], 1e-3),
            f"{cell['cell']}: opening thread-time != probe",
        )

        gross = min(cell["closing_thread_ms"], cell["slice_stage_wall_ms"])
        net = gross - cell["opening_thread_ms_p1"]
        ck(
            approx(cell["gross_ceiling_ms"], gross, 1e-6),
            f"{cell['cell']}: gross ceiling is not min(closing, stage wall)",
        )
        ck(
            approx(cell["net_ceiling_ms"], net, 1e-6),
            f"{cell['cell']}: net ceiling is not gross minus opening",
        )

        floor = ceilings["floor_fraction"] * cell["cell_wall_ms"]
        ck(
            approx(cell["floor_ms"], floor, 1e-3),
            f"{cell['cell']}: floor is not 0.80 % of the cell wall",
        )
        ck(
            approx(cell["floor_multiple"], net / floor, 0.01),
            f"{cell['cell']}: floor multiple inconsistent",
        )
        ck(
            cell["clears_floor"] == (net >= floor),
            f"{cell['cell']}: clears_floor flag inconsistent",
        )
        ck(
            net >= floor,
            f"{cell['cell']}: ceiling does not clear the 0.80 % floor",
        )
        ck(
            approx(
                cell["net_ceiling_pct_of_cell"],
                100.0 * net / cell["cell_wall_ms"],
                0.01,
            ),
            f"{cell['cell']}: ceiling share inconsistent",
        )

        share = cell["closing_thread_ms"] / cell["slice_thread_ms"]
        ck(
            approx(cell["closing_share_of_slice_thread"], share, 1e-4),
            f"{cell['cell']}: closing share inconsistent",
        )
        ck(
            approx(
                cell["expected_wall_ms_uniform_concurrency"],
                share * cell["slice_stage_wall_ms"],
                0.01,
            ),
            f"{cell['cell']}: expected saving inconsistent",
        )
        ck(
            cell["expected_wall_ms_uniform_concurrency"] < cell["net_ceiling_ms"],
            f"{cell['cell']}: attribution estimate exceeds the pre-declared ceiling",
        )

    # Arachne cells share their fixture's prepass costs by construction.
    for fixture in FIXTURES:
        off = by_name[f"{fixture}-classic-off"]
        for cell in cells:
            if cell["fixture"] != fixture:
                continue
            for key in (
                "closing_thread_ms",
                "opening_thread_ms_p1",
                "slice_stage_wall_ms",
                "net_ceiling_ms",
            ):
                ck(
                    cell[key] == off[key],
                    f"{cell['cell']}: {key} differs from {fixture} classic-off",
                )

    # Headline values.
    multiples = [c["floor_multiple"] for c in cells]
    ck(
        approx(min(multiples), CLAIMS["min_floor_multiple"], 0.05),
        f"headline min floor multiple {min(multiples)} != claim",
    )
    ck(
        approx(max(multiples), CLAIMS["max_floor_multiple"], 0.05),
        f"headline max floor multiple {max(multiples)} != claim",
    )
    ck(
        len({max(multiples), min(multiples)}) == 2,
        "headline range collapsed",
    )

    benchy = data["summaries"]["benchy"]
    base = data["summaries"]["base"]
    ck(
        benchy["differ_layers"] == CLAIMS["benchy_layers_changed"],
        "benchy changed-layer count != claim",
    )
    ck(
        base["differ_layers"] == CLAIMS["base_layers_changed"],
        "base changed-layer count != claim",
    )
    ck(
        approx(
            benchy["open_area_mm2"] - benchy["prod_area_mm2"],
            CLAIMS["benchy_net_area_mm2"],
            0.01,
        ),
        "benchy net area delta != claim",
    )
    ck(
        approx(
            base["open_area_mm2"] - base["prod_area_mm2"],
            CLAIMS["base_net_area_mm2"],
            0.01,
        ),
        "base net area delta != claim",
    )

    return failures


def mutations(data: dict):
    """Yield (label, mutated) pairs that the checker MUST reject."""

    def mutate(label: str, fn):
        copy_data = copy.deepcopy(data)
        fn(copy_data)
        yield label, copy_data

    yield from mutate(
        "a benchy-on cell wall inflated so its ceiling drops below the floor",
        lambda d: by_cell(d, "benchy-classic-on").update({"cell_wall_ms": 1_000_000.0}),
    )
    yield from mutate(
        "closing thread-time above the stage thread-time",
        lambda d: d["profiles"]["base"].update(
            {"closing_thread_ms": d["profiles"]["base"]["slice_thread_ms"] * 2.0}
        ),
    )
    yield from mutate(
        "opening priced at the closing's own cost",
        lambda d: d["summaries"]["base"].update(
            {"open_ms_sum": d["summaries"]["base"]["close_ms_sum"]}
        ),
    )
    yield from mutate(
        "gate area inflated above the flat family",
        lambda d: d["summaries"]["benchy"].update(
            {"prod_area_mm2": d["summaries"]["benchy"]["flat_area_mm2"] * 2.0}
        ),
    )
    yield from mutate(
        "differing-layer count detached from the rows",
        lambda d: d["summaries"]["base"].update({"differ_layers": 1}),
    )
    yield from mutate(
        "net ceiling recomputed without subtracting the opening",
        lambda d: by_cell(d, "base-classic-off").update(
            {"net_ceiling_ms": by_cell(d, "base-classic-off")["gross_ceiling_ms"]}
        ),
    )
    yield from mutate(
        "floor multiple detached from the ceiling arithmetic",
        lambda d: by_cell(d, "benchy-classic-off").update({"floor_multiple": 1.0}),
    )
    yield from mutate(
        "an arachne cell given its own prepass cost",
        lambda d: by_cell(d, "base-arachne-on").update({"closing_thread_ms": 1.0}),
    )
    yield from mutate(
        "P2 classification set shrunk to the flat family",
        lambda d: d["summaries"]["benchy"].update(
            {"canon_area_mm2": d["summaries"]["benchy"]["flat_area_mm2"]}
        ),
    )


def by_cell(data: dict, name: str) -> dict:
    for cell in data["ceilings"]["cells"]:
        if cell["cell"] == name:
            return cell
    raise KeyError(name)


def main() -> int:
    data = load()
    failures = check(data)
    if failures:
        print("FAIL")
        for failure in failures:
            print(f"  - {failure}")
        return 1
    print("PASS: reduced evidence is internally consistent and supports the resolution")

    if "--self-test" in sys.argv:
        rejected = 0
        total = 0
        for label, mutated in mutations(data):
            total += 1
            before = len(check(mutated))
            if before > 0:
                rejected += 1
            else:
                print(f"  NEGATIVE CONTROL SURVIVED: {label}")
        print(f"negative controls rejected: {rejected}/{total}")
        if rejected != total:
            return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
