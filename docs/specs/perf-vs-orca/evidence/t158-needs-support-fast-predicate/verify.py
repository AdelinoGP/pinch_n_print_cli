#!/usr/bin/env python3
"""Verify the committed t158 evidence without the gitignored raw tree.

Reads only ``reduced/campaign.json`` and re-derives every claim:

  * 8/8 baseline/candidate byte equality and the full t151 active reference;
  * exit 0, ``degraded=false``, zero fatal/non-fatal/diagnostic runs;
  * interleaved pairs per cell (3 or 5), recomputed medians and the 0.80 % floor;
  * CPU corroboration and per-pair candidate wins;
  * the same-arm null control and its measured band;
  * every recorded gate.

``--self-test`` mutates copies of the payload and requires each negative
control to be rejected.

The verdict checked here is the campaign's original mechanical gate verdict.
The later human KEEP ruling lives in ``RULING.md`` and deliberately does not
rewrite the recorded sub-floor cells or turn that gate failure into a pass.
"""

from __future__ import annotations

import argparse
import copy
import json
import statistics
from pathlib import Path
from typing import Callable

HERE = Path(__file__).resolve().parent
DEFAULT = HERE / "reduced" / "campaign.json"
CELLS = [
    "benchy-classic-off",
    "benchy-classic-on",
    "benchy-arachne-off",
    "benchy-arachne-on",
    "base-classic-off",
    "base-classic-on",
    "base-arachne-off",
    "base-arachne-on",
]
NULL_CELLS = ["base-classic-off", "base-classic-on", "base-arachne-off"]


def check(payload: dict) -> list[str]:
    failures: list[str] = []
    if payload.get("ticket") != 158:
        failures.append("payload is not ticket 158")
    floor = float(payload.get("floor_pct", 0.80))
    reference = payload.get("active_reference", {})
    output = payload.get("output", {})
    ab = payload.get("ab", {})
    null = payload.get("null", {})
    gates = payload.get("gates", {})
    saving_by_cell: dict[str, float] = {}

    if set(reference) != set(CELLS):
        failures.append("active reference is not the full 8-cell set")
    if set(output) != set(CELLS):
        failures.append("output evidence is not the full 8-cell set")
    if set(ab) != set(CELLS):
        failures.append("A/B evidence is not the full 8-cell set")

    for cell in CELLS:
        o = output.get(cell)
        if not o:
            continue
        if not o.get("identical"):
            failures.append(f"{cell}: baseline/candidate G-code differs")
        if o.get("candidate_sha256") != reference.get(cell):
            failures.append(f"{cell}: candidate hash is not the active reference")
        if not o.get("baseline_clean") or not o.get("candidate_clean"):
            failures.append(f"{cell}: an output run is not clean")
        if int(o.get("gcode_bytes", 0)) <= 0:
            failures.append(f"{cell}: empty G-code")
        for arm in ("baseline", "candidate"):
            record = o.get(arm) or {}
            clean = record.get("clean") or {}
            if record.get("exit_code") != 0:
                failures.append(f"{cell}/{arm}: exit {record.get('exit_code')}")
            if clean.get("degraded") is not False:
                failures.append(f"{cell}/{arm}: degraded={clean.get('degraded')}")
            for field in (
                "fatal_error_count",
                "non_fatal_error_count",
                "diagnostic_lines",
            ):
                if clean.get(field) != 0:
                    failures.append(f"{cell}/{arm}: {field}={clean.get(field)}")

        entry = ab.get(cell)
        if not entry:
            continue
        rows = entry.get("pairs", [])
        pcount = entry.get("pair_count")
        if pcount not in (3, 5) or len(rows) != pcount:
            failures.append(f"{cell}: A/B pair_count={pcount} len={len(rows)}")
        if len({row["pair"] for row in rows}) != len(rows):
            failures.append(f"{cell}: pair ids are not distinct")
        baseline_wall = statistics.median(row["baseline_wall_s"] for row in rows)
        candidate_wall = statistics.median(row["candidate_wall_s"] for row in rows)
        baseline_cpu = statistics.median(row["baseline_cpu_s"] for row in rows)
        candidate_cpu = statistics.median(row["candidate_cpu_s"] for row in rows)
        saving = (1.0 - candidate_wall / baseline_wall) * 100.0
        saving_by_cell[cell] = saving
        if abs(saving - float(entry.get("wall_saving_pct", 0.0))) > 1e-3:
            failures.append(f"{cell}: recorded wall saving != recomputed")
        if abs(baseline_wall - float(entry.get("baseline_wall_median_s", 0.0))) > 1e-3:
            failures.append(f"{cell}: recorded baseline wall median != recomputed")
        if (
            abs(candidate_wall - float(entry.get("candidate_wall_median_s", 0.0)))
            > 1e-3
        ):
            failures.append(f"{cell}: recorded candidate wall median != recomputed")
        if abs(baseline_cpu - float(entry.get("baseline_cpu_median_s", 0.0))) > 1e-3:
            failures.append(f"{cell}: recorded baseline CPU median != recomputed")
        if abs(candidate_cpu - float(entry.get("candidate_cpu_median_s", 0.0))) > 1e-3:
            failures.append(f"{cell}: recorded candidate CPU median != recomputed")
        if candidate_cpu >= baseline_cpu:
            failures.append(f"{cell}: CPU does not corroborate the wall saving")
        if entry.get("cpu_corroborates") is not (candidate_cpu < baseline_cpu):
            failures.append(f"{cell}: recorded CPU corroboration flag is wrong")
        faster = sum(
            1
            for record in rows
            if record["candidate_wall_s"] < record["baseline_wall_s"]
        )
        if entry.get("pairs_candidate_faster") != faster:
            failures.append(f"{cell}: recorded faster-pair count != recomputed")
        if faster < 1:
            failures.append(f"{cell}: no individual pair favours the candidate")
        if not entry.get("output_sha256_stable"):
            failures.append(f"{cell}: A/B runs do not emit one stable hash")
        if not entry.get("all_runs_clean"):
            failures.append(f"{cell}: an A/B run is not clean")
        for record in rows:
            if record.get("baseline_sha256") != reference.get(cell):
                failures.append(f"{cell}: A/B baseline hash is not the reference")
            if record.get("candidate_sha256") != reference.get(cell):
                failures.append(f"{cell}: A/B candidate hash is not the reference")
            for side in ("baseline_clean", "candidate_clean"):
                clean = record.get(side) or {}
                if clean.get("ok") is not True:
                    failures.append(f"{cell}: {side} is not clean")

    null_cells = null.get("cells", {})
    if set(null_cells) != set(NULL_CELLS):
        failures.append("null control does not cover its three cells")
    recomputed_wall_band = 0.0
    recomputed_cpu_band = 0.0
    for cell, entry in null_cells.items():
        rows = entry.get("pairs", [])
        if entry.get("pair_count") != 3 or len(rows) != 3:
            failures.append(f"null {cell}: not 3 pairs")
        for record in rows:
            if record.get("both_clean") is not True:
                failures.append(f"null {cell}: a same-arm run is not clean")
            if record.get("first_sha256") != reference.get(cell) or record.get(
                "second_sha256"
            ) != reference.get(cell):
                failures.append(f"null {cell}: same-arm output moved")
            wall_delta = (
                (record["second_wall_s"] - record["first_wall_s"])
                / record["first_wall_s"]
                * 100.0
            )
            cpu_delta = (
                (record["second_cpu_s"] - record["first_cpu_s"])
                / record["first_cpu_s"]
                * 100.0
            )
            if abs(wall_delta - record["wall_delta_pct"]) > 1e-3:
                failures.append(f"null {cell}: recorded wall delta != recomputed")
            if abs(cpu_delta - record["cpu_delta_pct"]) > 1e-3:
                failures.append(f"null {cell}: recorded CPU delta != recomputed")
            recomputed_wall_band = max(recomputed_wall_band, abs(wall_delta))
            recomputed_cpu_band = max(recomputed_cpu_band, abs(cpu_delta))
        wall_median = statistics.median(r["wall_delta_pct"] for r in rows)
        cpu_median = statistics.median(r["cpu_delta_pct"] for r in rows)
        if abs(wall_median - entry["wall_delta_pct_median"]) > 1e-3:
            failures.append(f"null {cell}: recorded wall median != recomputed")
        if abs(cpu_median - entry["cpu_delta_pct_median"]) > 1e-3:
            failures.append(f"null {cell}: recorded CPU median != recomputed")
    band = null.get("band_pct", {})
    if abs(float(band.get("wall_abs_max_pct", 0.0)) - recomputed_wall_band) > 1e-3:
        failures.append("null wall band != recomputed")
    if abs(float(band.get("cpu_abs_max_pct", 0.0)) - recomputed_cpu_band) > 1e-3:
        failures.append("null CPU band != recomputed")
    if not recomputed_wall_band > 0.0:
        failures.append("null wall band is zero or missing")
    if not recomputed_cpu_band > 0.0:
        failures.append("null CPU band is zero or missing")

    manifest = payload.get("manifest") or {}
    if set(manifest) != {"baseline-ordinary", "candidate-ordinary"}:
        failures.append("manifest does not carry both arms")
    elif (
        manifest["baseline-ordinary"]["exe_sha256"]
        == manifest["candidate-ordinary"]["exe_sha256"]
    ):
        failures.append("baseline and candidate share one host binary")

    derived_drop = [cell for cell in CELLS if saving_by_cell[cell] < floor]
    derived_gates = {
        "output_identical_8_of_8": all(output[cell]["identical"] for cell in CELLS),
        "reference_set_unmoved_8_of_8": all(
            output[cell]["candidate_sha256"] == reference[cell] for cell in CELLS
        ),
        "output_runs_clean_8_of_8": all(
            output[cell]["baseline_clean"] and output[cell]["candidate_clean"]
            for cell in CELLS
        ),
        "timing_floor_8_of_8": not derived_drop,
        "cpu_corroborates_8_of_8": all(ab[cell]["cpu_corroborates"] for cell in CELLS),
        "no_cell_regresses": all(saving_by_cell[cell] > 0.0 for cell in CELLS),
        "ab_output_stable_8_of_8": all(
            ab[cell]["output_sha256_stable"] for cell in CELLS
        ),
        "ab_runs_clean_8_of_8": all(ab[cell]["all_runs_clean"] for cell in CELLS),
        "null_control_complete": set(null_cells) == set(NULL_CELLS)
        and all(entry.get("pair_count") == 3 for entry in null_cells.values()),
        "null_control_clean": all(
            entry.get("all_pairs_clean") is True for entry in null_cells.values()
        ),
    }
    for name, derived in derived_gates.items():
        if gates.get(name) is not derived:
            failures.append(
                f"gate {name} recorded {gates.get(name)!r}, derived {derived!r}"
            )
    expected_verdict = "KEEP" if all(derived_gates.values()) else "DROP"
    if payload.get("verdict") != expected_verdict:
        failures.append(
            f"verdict {payload.get('verdict')!r} != derived {expected_verdict!r}"
        )
    if payload.get("drop_cells") != derived_drop:
        failures.append(
            f"drop_cells {payload.get('drop_cells')} != derived {derived_drop}"
        )
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--evidence", type=Path, default=DEFAULT)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()

    payload = json.loads(args.evidence.read_text(encoding="utf-8"))
    failures = check(payload)
    if failures:
        for failure in failures:
            print(f"FAIL {failure}")
        return 1
    print(
        f"PASS t158 evidence: verdict={payload['verdict']} "
        f"drop_cells={payload['drop_cells']}; 8/8 output-identical to the active "
        "reference with clean runs and a recorded null band"
    )

    if not args.self_test:
        return 0

    controls: list[tuple[str, Callable[[dict], None]]] = [
        (
            "candidate hash moved",
            lambda p: p["output"]["base-classic-off"].__setitem__(
                "candidate_sha256", "0" * 64
            ),
        ),
        (
            "sub-floor saving",
            lambda p: [
                record.__setitem__("candidate_wall_s", record["baseline_wall_s"] * 2.0)
                for record in p["ab"]["base-classic-on"]["pairs"]
            ],
        ),
        (
            "CPU inversion",
            lambda p: [
                record.__setitem__("candidate_cpu_s", record["baseline_cpu_s"] * 2.0)
                for record in p["ab"]["base-arachne-off"]["pairs"]
            ],
        ),
        (
            "degraded output run",
            lambda p: p["output"]["benchy-classic-off"]["candidate"][
                "clean"
            ].__setitem__("degraded", True),
        ),
        (
            "missing null cell",
            lambda p: p["null"]["cells"].pop("base-classic-on"),
        ),
        (
            "shared host binary",
            lambda p: p["manifest"]["candidate-ordinary"].__setitem__(
                "exe_sha256", p["manifest"]["baseline-ordinary"]["exe_sha256"]
            ),
        ),
    ]
    rejected = 0
    for label, mutate in controls:
        mutated = copy.deepcopy(payload)
        mutate(mutated)
        if check(mutated):
            print(f"SELFTEST PASS (rejected) {label}")
            rejected += 1
        else:
            print(f"SELFTEST FAIL (accepted) {label}")
    if rejected != len(controls):
        return 1
    print(f"SELFTEST PASS {rejected}/{len(controls)} negative controls rejected")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
