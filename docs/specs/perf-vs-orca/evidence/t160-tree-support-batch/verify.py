#!/usr/bin/env python3
"""Verify t160's reduced campaign: re-derive every claim from campaign.json."""

from __future__ import annotations

import argparse
import copy
import json
import statistics
import sys
from pathlib import Path

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
SUPPORTS_ON = [
    "benchy-classic-on",
    "benchy-arachne-on",
    "base-classic-on",
    "base-arachne-on",
]
SUPPORTS_OFF = [c for c in CELLS if c not in SUPPORTS_ON]


def close(a: float, b: float, tol: float = 5e-4) -> bool:
    return abs(a - b) <= tol


def verify(payload: dict) -> list[str]:
    failures: list[str] = []

    def require(condition: bool, message: str) -> None:
        if not condition:
            failures.append(message)

    floor = float(payload.get("floor_pct", 0.0))
    require(close(floor, 0.80), "floor_pct is not 0.80")

    manifest = payload["manifest"]
    require(
        set(manifest) == {"baseline", "batch", "nullrender", "hash"},
        "manifest arms incomplete",
    )
    exe_shas = {arm["exe_sha256"] for arm in manifest.values()}
    tree_shas = {arm["modules_tree_sha256"] for arm in manifest.values()}
    require(len(exe_shas) == 1, "arms do not share one binary")
    require(len(tree_shas) == 1, "arms do not share one module tree")
    require(
        manifest["batch"]["env"].get("PNP_T160_SUPPORT_BATCH") == "1",
        "batch arm lacks its env gate",
    )
    require(
        manifest["nullrender"]["env"].get("PNP_T160_SUPPORT_NULL") == "1",
        "null arm lacks its env gate",
    )
    require(
        manifest["hash"]["env"].get("PNP_T160_INPUT_HASH") == "1",
        "hash arm lacks its env gate",
    )
    require(manifest["baseline"]["env"] == {}, "baseline arm carries an env gate")

    identity = payload["identity"]
    require(
        set(identity["cells"]) == set(CELLS), "identity does not cover the eight cells"
    )
    for cell, entry in identity["cells"].items():
        match = (
            entry["shared_layers"] > 0
            and entry["shared_layers"]
            == entry["batch_layers"]
            == entry["per_layer_layers"]
            and all(count == 0 for count in entry["mismatch_counts"].values())
        )
        require(
            entry["match"] == match, f"identity {cell}: recorded match != recomputed"
        )
        require(entry["run_clean"] is True, f"identity {cell}: run not clean")
    require(
        identity["all_cells_match"]
        == all(v["match"] for v in identity["cells"].values()),
        "identity all_cells_match != recomputed",
    )

    output = payload["output"]
    require(set(output) == set(CELLS), "output does not cover the eight cells")
    for cell, entry in output.items():
        require(entry["identical"] is True, f"output {cell}: arms differ")
        require(
            entry["matches_active_reference"] is True,
            f"output {cell}: not t151's active hash",
        )
        require(
            entry["baseline_sha256"] == entry["active_reference_sha256"],
            f"output {cell}: baseline is not the active hash",
        )
        require(
            entry["candidate_sha256"] == entry["active_reference_sha256"],
            f"output {cell}: batch is not the active hash",
        )
        require(entry["baseline_clean"] is True, f"output {cell}: baseline not clean")
        require(entry["candidate_clean"] is True, f"output {cell}: batch not clean")

    ab = payload["ab"]
    require(set(ab) == set(CELLS), "ab does not cover the eight cells")
    for cell, entry in ab.items():
        pairs = entry["pairs"]
        require(
            len(pairs) == entry["pair_count"], f"ab {cell}: pair_count != len(pairs)"
        )
        if not pairs:
            failures.append(f"ab {cell}: no pairs")
            continue
        b_med = statistics.median(p["baseline_wall_s"] for p in pairs)
        c_med = statistics.median(p["candidate_wall_s"] for p in pairs)
        b_cpu = statistics.median(p["baseline_cpu_s"] for p in pairs)
        c_cpu = statistics.median(p["candidate_cpu_s"] for p in pairs)
        saving = (1.0 - c_med / b_med) * 100.0
        cpu_saving = (1.0 - c_cpu / b_cpu) * 100.0
        require(
            close(entry["baseline_wall_median_s"], b_med),
            f"ab {cell}: baseline median != recomputed",
        )
        require(
            close(entry["candidate_wall_median_s"], c_med),
            f"ab {cell}: candidate median != recomputed",
        )
        require(
            close(entry["wall_saving_pct"], saving),
            f"ab {cell}: wall saving != recomputed",
        )
        require(
            close(entry["cpu_saving_pct"], cpu_saving),
            f"ab {cell}: cpu saving != recomputed",
        )
        require(
            close(entry["floor_multiple"], round(saving / floor, 2)),
            f"ab {cell}: floor multiple != recomputed",
        )
        require(
            entry["clears_floor_0p80pct"] == (saving >= floor),
            f"ab {cell}: floor flag != recomputed",
        )
        wins = sum(1 for p in pairs if p["candidate_wall_s"] < p["baseline_wall_s"])
        require(
            entry["pairs_candidate_faster"] == wins,
            f"ab {cell}: pair wins != recomputed",
        )
        require(
            entry["batch_pass_skipped_all_pairs"]
            == all(p["batch_pass_skipped"] for p in pairs),
            f"ab {cell}: batch-pass skip flag != recomputed",
        )
        require(entry["all_runs_clean"] is True, f"ab {cell}: a run was not clean")
        require(
            entry["output_sha256_stable"] is True,
            f"ab {cell}: output moved across pairs",
        )

    null = payload["null"]
    require(
        set(null["cells"]) == set(SUPPORTS_ON),
        "null control does not cover the four supports-on cells",
    )
    wall_maxima = []
    cpu_maxima = []
    for cell, entry in null["cells"].items():
        pairs = entry["pairs"]
        require(len(pairs) >= 3, f"null {cell}: fewer than 3 pairs")
        require(entry["all_pairs_clean"] is True, f"null {cell}: a pair was not clean")
        wall_max = max(abs(p["wall_delta_pct"]) for p in pairs)
        cpu_max = max(abs(p["cpu_delta_pct"]) for p in pairs)
        wall_maxima.append(wall_max)
        cpu_maxima.append(cpu_max)
        require(
            close(entry["wall_abs_max_pct"], wall_max),
            f"null {cell}: wall max != recomputed",
        )
        require(
            close(entry["cpu_abs_max_pct"], cpu_max),
            f"null {cell}: cpu max != recomputed",
        )
    band_wall = max(wall_maxima)
    band_cpu = max(cpu_maxima)
    require(
        close(null["band_pct"]["wall_abs_max_pct"], band_wall),
        "null wall band != recomputed",
    )
    require(
        close(null["band_pct"]["cpu_abs_max_pct"], band_cpu),
        "null cpu band != recomputed",
    )

    ceiling = payload["ceiling"]
    if ceiling.get("present"):
        for cell, entry in ceiling["cells"].items():
            require(entry["run_count"] >= 5, f"ceiling {cell}: fewer than 5 runs")
            baseline = ab[cell]["baseline_wall_median_s"]
            ceiling_pct = (
                (baseline - entry["nullrender_wall_median_s"]) / baseline * 100.0
            )
            require(
                close(entry["render_ceiling_pct"], ceiling_pct),
                f"ceiling {cell}: ceiling != recomputed",
            )

    verdicts = payload["verdicts"]
    require(
        set(verdicts) == set(SUPPORTS_ON),
        "verdicts do not cover the four supports-on cells",
    )
    for cell, entry in verdicts.items():
        saving = ab[cell]["wall_saving_pct"]
        outside = abs(saving) > band_wall
        expected = "hurts" if saving >= floor and outside else "exonerated"
        require(
            entry["verdict"] == expected,
            f"verdict {cell}: {entry['verdict']} != recomputed {expected}",
        )
        require(
            entry["outside_null_band"] == outside,
            f"verdict {cell}: outside_null_band != recomputed",
        )

    gates = payload["gates"]
    recomputed = {
        "input_identity_recorded": bool(identity["cells"]),
        "input_read_set_identical_8_of_8": all(
            v["match"] for v in identity["cells"].values()
        ),
        "output_identical_8_of_8": all(v["identical"] for v in output.values()),
        "reference_set_unmoved_8_of_8": all(
            v["matches_active_reference"] for v in output.values()
        ),
        "output_runs_clean_8_of_8": all(
            v["baseline_clean"] and v["candidate_clean"] for v in output.values()
        ),
        "ab_output_stable_8_of_8": all(v["output_sha256_stable"] for v in ab.values()),
        "ab_runs_clean_8_of_8": all(v["all_runs_clean"] for v in ab.values()),
        "supports_off_batch_skipped_4_of_4": all(
            ab[c]["batch_pass_skipped_all_pairs"] for c in SUPPORTS_OFF
        ),
        "null_control_complete": all(
            null["cells"].get(c, {}).get("pair_count", 0) >= 3 for c in SUPPORTS_ON
        ),
        "null_control_clean": all(
            null["cells"].get(c, {}).get("all_pairs_clean") for c in SUPPORTS_ON
        ),
    }
    require(set(gates) == set(recomputed), "gate set != recomputed gate set")
    for name, value in recomputed.items():
        if name in gates:
            require(
                gates[name] == value,
                f"gate {name}: recorded {gates[name]} != recomputed {value}",
            )
    require(
        payload["all_gates_pass"] == all(gates.values()),
        "all_gates_pass != all(gates)",
    )
    require(bool(payload.get("disclosures")), "no disclosures recorded")
    return failures


def self_test(payload: dict) -> int:
    failures = verify(payload)
    if failures:
        print("SELF-TEST SETUP FAILED: the unmutated payload does not verify")
        for failure in failures:
            print(f"  - {failure}")
        return 1

    def mutate(name: str, fn):
        probe = copy.deepcopy(payload)
        fn(probe)
        if not verify(probe):
            print(f"SELF-TEST FAILED: mutation '{name}' was not rejected")
            return 1
        print(f"self-test: rejected '{name}'")
        return 0

    cell = "base-classic-on"
    mutations = [
        (
            "output identity flipped",
            lambda p: p["output"][cell].__setitem__(
                "identical", not p["output"][cell]["identical"]
            ),
        ),
        (
            "wall saving altered",
            lambda p: p["ab"][cell].__setitem__(
                "wall_saving_pct", p["ab"][cell]["wall_saving_pct"] + 1.0
            ),
        ),
        (
            "null cell dropped",
            lambda p: p["null"]["cells"].pop(cell),
        ),
        (
            "identity match flipped",
            lambda p: p["identity"]["cells"][cell].__setitem__(
                "match", not p["identity"]["cells"][cell]["match"]
            ),
        ),
        (
            "verdict flipped",
            lambda p: p["verdicts"][cell].__setitem__(
                "verdict",
                "hurts" if p["verdicts"][cell]["verdict"] != "hurts" else "exonerated",
            ),
        ),
        (
            "null band zeroed",
            lambda p: p["null"]["band_pct"].__setitem__("wall_abs_max_pct", 0.0),
        ),
        (
            "gate flag flipped",
            lambda p: p["gates"].__setitem__(
                "output_identical_8_of_8", not p["gates"]["output_identical_8_of_8"]
            ),
        ),
        (
            "arm env removed",
            lambda p: p["manifest"]["batch"].__setitem__("env", {}),
        ),
        (
            "supports-off skip flag flipped",
            lambda p: p["ab"]["base-arachne-off"].__setitem__(
                "batch_pass_skipped_all_pairs",
                not p["ab"]["base-arachne-off"]["batch_pass_skipped_all_pairs"],
            ),
        ),
    ]
    exit_code = 0
    for name, fn in mutations:
        exit_code |= mutate(name, fn)
    if exit_code == 0:
        print(f"SELF-TEST PASS: {len(mutations)}/{len(mutations)} mutations rejected")
    return exit_code


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--payload", type=Path, default=DEFAULT)
    ap.add_argument("--self-test", action="store_true")
    args = ap.parse_args()

    payload = json.loads(args.payload.read_text(encoding="utf-8"))
    if args.self_test:
        return self_test(payload)

    failures = verify(payload)
    if failures:
        print(f"FAIL: {len(failures)} claim(s) rejected")
        for failure in failures:
            print(f"  - {failure}")
        return 1
    verdicts = ", ".join(
        f"{cell}={entry['verdict']}" for cell, entry in payload["verdicts"].items()
    )
    print(
        "PASS: input read set identical 8/8, output identical 8/8, "
        f"paired campaign recorded, verdicts {verdicts}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
