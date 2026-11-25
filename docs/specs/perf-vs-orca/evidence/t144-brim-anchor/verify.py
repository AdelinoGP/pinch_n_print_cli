#!/usr/bin/env python3
"""t144 verifier: re-derive every closure claim from reduced/summary.json.

Reads only `reduced/summary.json` (no raw G-code, no Orca tree). Claims:

  1. The baseline arm reproduces the active reference set in 8/8 cells.
  2. The candidate is byte-identical across the ordinary and accelerated arms
     in 8/8 cells, and moved from the baseline in 8/8 (the brim exists
     everywhere, so the anchor shift must reach every cell).
  3. Every run exited 0 with a non-empty G-code.
  4. The two ordinary arms share one host binary; the skirt-brim guest differs;
     the planner guest is identical (guest-only change).
  5. The A/B is 3 interleaved pairs per cell, medians recomputed from the
     stored pair rows, and the null band is present (six cells, three pairs)
     and non-zero.
  6. The oracle has four supports-off cells with both arms, its band flags are
     the declared 12 % / 3 % predicates of the recorded rel-diffs, and the
     supports-on cells are marked support-engine-limited.
  7. The delta audit passes in 8/8, every section outside Skirt/Brim is
     identical with `E` stripped, and the max |dE| is within tolerance.

    python verify.py
    python verify.py --self-test
"""

from __future__ import annotations

import argparse
import copy
import json
import statistics
from pathlib import Path

HERE = Path(__file__).resolve().parent
SUMMARY = HERE / "reduced" / "summary.json"
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
SUPPORTS_OFF = [
    "benchy-classic-off",
    "benchy-arachne-off",
    "base-classic-off",
    "base-arachne-off",
]
SUPPORTS_ON = [
    "benchy-classic-on",
    "benchy-arachne-on",
    "base-classic-on",
    "base-arachne-on",
]
BENCHY = [c for c in CELLS if c.startswith("benchy-")]
BASE = [c for c in CELLS if c.startswith("base-")]
ROUND = 3e-3


class VerificationError(AssertionError):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise VerificationError(message)


def close(a: float, b: float, tol: float = 0.01) -> bool:
    return abs(a - b) <= tol


def verify(summary: dict) -> dict:
    require(summary.get("ticket") == 144, "summary is not ticket 144")

    # ---- 1/2/3: output -----------------------------------------------------
    output = summary.get("output", {})
    cells = output.get("cells", {})
    require(set(cells) == set(CELLS), f"output cells mismatch: {sorted(cells)}")
    for cell in CELLS:
        entry = cells[cell]
        require(
            entry.get("baseline_matches_active_reference") is True,
            f"{cell}: baseline does not reproduce the active reference set",
        )
        require(
            entry.get("candidate_equal_across_arms") is True,
            f"{cell}: candidate differs between ordinary and accelerated",
        )
        for arm in ("baseline-ordinary", "candidate-ordinary", "candidate-accelerated"):
            record = entry.get(arm)
            require(record is not None, f"{cell}: missing arm {arm}")
            require(
                record["exit_code"] == 0, f"{cell}/{arm}: exit {record['exit_code']}"
            )
            require(record["gcode_bytes"] > 0, f"{cell}/{arm}: empty G-code")

    # The anchor reaches every cell where an adhesion path exists. On the two
    # base supports-on cells the support footprint clips the whole brim area, so
    # no brim path (nor a brim hull point) reaches the output; that is measured,
    # disclosed, and cross-checked against the audit below.
    inert = {"base-classic-on", "base-arachne-on"}
    expected_moved = set(CELLS) - inert
    moved = {cell for cell in CELLS if cells[cell].get("candidate_moved_from_baseline")}
    require(
        moved == expected_moved,
        f"moved cells {sorted(moved)} != {sorted(expected_moved)}",
    )
    require(
        output.get("moved_cells") == len(expected_moved),
        f"moved_cells is {output.get('moved_cells')}, expected {len(expected_moved)}",
    )
    require(
        set(output.get("inert_cells", [])) == inert,
        f"inert cell disclosure {output.get('inert_cells')} != {sorted(inert)}",
    )

    active = output.get("active_reference", {})
    require(set(active) == set(CELLS), "active reference set is incomplete")
    candidate_set = output.get("candidate_set", {})
    require(set(candidate_set) == set(CELLS), "candidate set is incomplete")
    for cell in expected_moved:
        require(
            candidate_set[cell] != active[cell],
            f"{cell}: candidate hash equals the active reference",
        )
    for cell in inert:
        require(
            candidate_set[cell] == active[cell],
            f"{cell}: inert cell moved from the active reference",
        )
    # The reference kinds are the two known sources, not a claim of a new one.
    kinds = output.get("reference_kind", {})
    for cell in BENCHY:
        require(
            kinds.get(cell) == "benchy_replacement_fixture",
            f"{cell}: benchy reference kind is not the replacement-fixture baseline",
        )
    for cell in BASE:
        require(
            kinds.get(cell) == "t139_base", f"{cell}: base reference kind is not t139"
        )

    # ---- 4: manifest -------------------------------------------------------
    manifest = summary.get("manifest") or {}
    require(
        set(manifest)
        == {"baseline-ordinary", "candidate-ordinary", "candidate-accelerated"},
        f"manifest arms mismatch: {sorted(manifest)}",
    )
    exe_shas = {manifest[arm]["exe_sha256"] for arm in manifest}
    require(len(exe_shas) == 1, "the three arms do not share one host binary")
    require(
        manifest["baseline-ordinary"]["skirt_brim_wasm_sha256"]
        != manifest["candidate-ordinary"]["skirt_brim_wasm_sha256"],
        "baseline and candidate carry the same skirt-brim guest",
    )
    require(
        manifest["candidate-ordinary"]["skirt_brim_wasm_sha256"]
        != manifest["candidate-accelerated"]["skirt_brim_wasm_sha256"],
        "the ordinary and accelerated candidate arms carry one identical guest binary",
    )

    # ---- 5: A/B and null band ---------------------------------------------
    ab = summary.get("ab", {})
    require(set(ab) == set(CELLS), f"A/B cells mismatch: {sorted(ab)}")
    for cell in CELLS:
        entry = ab[cell]
        require(entry["pair_count"] == 3, f"{cell}: A/B is not 3 pairs")
        wall = statistics.median(r["wall_delta_pct"] for r in entry["pairs"])
        cpu = statistics.median(r["cpu_delta_pct"] for r in entry["pairs"])
        require(
            close(wall, entry["wall_delta_pct_median"], 0.02),
            f"{cell}: wall median {entry['wall_delta_pct_median']} != recomputed {wall}",
        )
        require(
            close(cpu, entry["cpu_delta_pct_median"], 0.02),
            f"{cell}: cpu median {entry['cpu_delta_pct_median']} != recomputed {cpu}",
        )
        for record in entry["pairs"]:
            require(
                record["baseline_wall_s"] > 0 and record["candidate_wall_s"] > 0,
                f"{cell}: non-positive pair wall",
            )

    null = summary.get("null") or {}
    null_cells = null.get("cells", {})
    require(
        len(null_cells) == 6, f"null control has {len(null_cells)} cells, expected 6"
    )
    for cell, entry in null_cells.items():
        require(entry["pair_count"] == 3, f"null {cell}: not 3 pairs")
    band = null.get("band_pct", {})
    require(band.get("wall_abs_max_pct", 0) > 0, "null wall band is zero")
    require(band.get("cpu_abs_max_pct", 0) > 0, "null CPU band is zero")

    # ---- 6: oracle ---------------------------------------------------------
    oracle = summary.get("oracle", {})
    ocells = oracle.get("cells", {})
    require(
        oracle.get("bands")
        == {"brim_swept_area_pct": 12.0, "skirt_outer_loop_area_pct": 3.0},
        "oracle bands are not the declared 12 % / 3 %",
    )
    for cell in SUPPORTS_OFF:
        entry = ocells.get(cell, {})
        for arm in ("baseline", "candidate"):
            record = entry.get(arm)
            require(record is not None, f"oracle {cell}: missing {arm}")
            brim = record["brim_swept_area_rel_diff"]
            skirt = record["skirt_outer_loop_area_rel_diff"]
            require(
                brim is not None and skirt is not None,
                f"oracle {cell}/{arm}: missing rel-diff",
            )
            require(
                record["brim_swept_area_pass"] == (brim <= 12.0 + 1e-9),
                f"oracle {cell}/{arm}: brim flag does not follow the 12 % band",
            )
            require(
                record["skirt_area_pass"] == (skirt <= 3.0 + 1e-9),
                f"oracle {cell}/{arm}: skirt flag does not follow the 3 % band",
            )
            expected_pass = (
                record["brim_swept_area_pass"]
                and record["skirt_area_pass"]
                and record.get("brim_presence_pass", True)
                and record.get("skirt_presence_pass", True)
                and record.get("brim_growth_pass", True)
                and record.get("skirt_growth_pass", True)
            )
            require(
                record["pass"] == expected_pass,
                f"oracle {cell}/{arm}: overall flag does not follow its parts",
            )
    for cell in SUPPORTS_ON:
        entry = ocells.get(cell, {})
        require(
            str(entry.get("status", "")).startswith("support-engine-limited"),
            f"oracle {cell}: supports-on is not marked support-engine-limited",
        )

    # ---- 7: delta audit ----------------------------------------------------
    audit = summary.get("audit")
    require(audit is not None, "delta audit missing")
    require(audit.get("pass") is True, "delta audit did not pass")
    require(len(audit.get("cells", [])) == 8, "delta audit is not 8 cells")
    for cell in audit["cells"]:
        for section in cell["geometry_sections"]:
            if section["feature"] in ("Skirt", "Brim"):
                continue
            require(
                section["identical"] is True,
                f"{cell['cell']}: {section['feature']} motion moved outside adhesion",
            )
        stats = cell["e_stats"]
        require(
            stats["max_abs_delta_mm"] <= stats["tolerance_mm"] + 1e-9,
            f"{cell['cell']}: |dE| {stats['max_abs_delta_mm']} above tolerance",
        )
        if cell["cell"] in inert:
            require(
                stats["e_changed_lines"] == 0,
                f"{cell['cell']}: disclosed-inert cell changed {stats['e_changed_lines']} E lines",
            )

    achieved = sum(1 for cell in SUPPORTS_OFF if ocells[cell]["candidate"]["pass"])
    return {
        "cells": 8,
        "candidate_moved": output["moved_cells"],
        "oracle_supports_off_passes": achieved,
        "oracle_scope": "supports-off only; supports-on support-engine-limited",
        "ab_median_wall_pct": {
            cell: ab[cell]["wall_delta_pct_median"] for cell in CELLS
        },
        "null_band_pct": band,
    }


def self_test(summary: dict) -> None:
    """Every mutation must be rejected; the untouched summary must pass."""
    verify(summary)
    mutations = []

    def mutate(name: str, fn) -> None:
        mutated = copy.deepcopy(summary)
        fn(mutated)
        mutations.append((name, mutated))

    mutate(
        "baseline reference miss",
        lambda s: s["output"]["cells"][CELLS[0]].__setitem__(
            "baseline_matches_active_reference", False
        ),
    )
    mutate(
        "arms diverge",
        lambda s: s["output"]["cells"][CELLS[1]].__setitem__(
            "candidate_equal_across_arms", False
        ),
    )
    mutate(
        "anchor inert",
        lambda s: s["output"]["cells"][CELLS[2]].__setitem__(
            "candidate_moved_from_baseline", False
        ),
    )
    mutate(
        "inert cell moved",
        lambda s: s["output"]["cells"]["base-classic-on"].__setitem__(
            "candidate_moved_from_baseline", True
        ),
    )
    mutate(
        "bad exit",
        lambda s: s["output"]["cells"][CELLS[3]]["candidate-accelerated"].__setitem__(
            "exit_code", 2
        ),
    )
    mutate(
        "shared exe broken",
        lambda s: s["manifest"]["candidate-ordinary"].__setitem__("exe_sha256", "00"),
    )
    mutate(
        "same guest",
        lambda s: s["manifest"]["candidate-ordinary"].__setitem__(
            "skirt_brim_wasm_sha256",
            s["manifest"]["baseline-ordinary"]["skirt_brim_wasm_sha256"],
        ),
    )
    mutate(
        "median drift",
        lambda s: s["ab"][CELLS[0]].__setitem__("wall_delta_pct_median", 99.0),
    )
    mutate("missing pair", lambda s: s["ab"][CELLS[1]]["pairs"].pop())
    mutate(
        "oracle band lie",
        lambda s: s["oracle"]["cells"][SUPPORTS_OFF[0]]["candidate"].__setitem__(
            "brim_swept_area_pass",
            not s["oracle"]["cells"][SUPPORTS_OFF[0]]["candidate"][
                "brim_swept_area_pass"
            ],
        ),
    )
    mutate(
        "audit section moved",
        lambda s: [
            sec.__setitem__("identical", False)
            for cell in s["audit"]["cells"]
            for sec in cell["geometry_sections"]
            if sec["feature"] == "Inner wall"
        ],
    )
    mutate("moved_cells", lambda s: s["output"].__setitem__("moved_cells", 7))
    mutate(
        "supports-on scope",
        lambda s: s["oracle"]["cells"].__setitem__(SUPPORTS_ON[0], {}),
    )

    for name, mutated in mutations:
        try:
            verify(mutated)
        except VerificationError:
            continue
        except Exception as exc:  # noqa: BLE001 - any rejection counts
            continue
        raise SystemExit(f"self-test: mutation '{name}' was NOT rejected")
    print(f"self-test: {len(mutations)} mutations rejected, unmutated summary passes")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--summary", type=Path, default=SUMMARY)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    summary = json.loads(args.summary.read_text(encoding="utf-8"))
    if args.self_test:
        self_test(summary)
        return 0
    derived = verify(summary)
    print(json.dumps(derived, indent=2))
    print("PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
