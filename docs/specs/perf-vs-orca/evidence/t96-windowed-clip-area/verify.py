#!/usr/bin/env python3
"""t96 Phase-1 verifier.

Exits 0 against the in-repo reduced set alone (no `.local-artifacts`, no model
fixtures). Re-derives the price from `probe-rows.json`, checks the deterministic
output drift in `output-drift.json`, and rejects every negative control.

Usage:
    python verify.py            # exit 0 on the committed evidence
    python verify.py --self-test
"""

from __future__ import annotations

import copy
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
REFERENCE_SHA = "292df50179c4033d6204a0353e038534b4d3a98381126998bb5722a41a571a80"
REFERENCE_BYTES = 21023739
CELL_WALL = 73.17
FLOOR_SHARE = 0.008
ROWS_PER_RUN = 130


def check(payload: dict, drift: dict) -> None:
    assert payload["cell_wall_seconds"] == CELL_WALL, "cell wall changed"
    assert payload["floor_share"] == FLOOR_SHARE, "floor changed"
    runs = payload["runs"]
    assert len(runs) == 2, "two probe runs required (order check)"

    shares = []
    for run in runs:
        rows = run["rows"]
        assert len(rows) == ROWS_PER_RUN, f"{run['run']}: row count {len(rows)}"
        full = sum(row["full_ms"] for row in rows)
        win = sum(row["win_total_ms"] for row in rows)
        assert abs(full - run["full_ms"]) < 0.01, f"{run['run']}: full sum"
        assert abs(win - run["window_ms"]) < 0.01, f"{run['run']}: window sum"
        assert abs((full - win) - run["saving_ms"]) < 0.01, f"{run['run']}: saving"
        wall_eq = (full - win) / 1000.0 / run["cpu_wall_ratio"]
        assert abs(wall_eq - run["wall_equivalent_saving_s"]) < 0.001, (
            f"{run['run']}: wall-equivalent"
        )
        share = wall_eq / CELL_WALL
        assert share > FLOOR_SHARE, (
            f"{run['run']}: share {share:.4f} does not clear the floor"
        )
        assert 6.0 < run["cpu_wall_ratio"] < 14.0, f"{run['run']}: cpu/wall"
        shares.append(share)
        assert run["sha256"] != REFERENCE_SHA or run["run"] != "probe-window", (
            "windowed probe run must drift"
        )
        for row in rows:
            assert row["win_area_polys"] >= 1, f"{run['run']}: empty window area"
            assert row["print_z"] >= 0.0, f"{run['run']}: print_z"
    assert abs(shares[0] - shares[1]) / max(shares) < 0.10, (
        "the two probe runs disagree by more than 10 %"
    )

    window = payload["window_arm"]
    assert window["sha256"] == runs[1]["sha256"], "window arm not deterministic"
    assert window["sha256"] != REFERENCE_SHA, "window arm must drift from reference"
    assert window["bytes"] != REFERENCE_BYTES, "window arm byte count must change"

    assert drift["reference_sha256"] == REFERENCE_SHA, "reference sha"
    assert drift["reference_bytes"] == REFERENCE_BYTES, "reference bytes"
    assert drift["window_sha256"] == window["sha256"], "drift sha"
    assert drift["window_bytes"] == window["bytes"], "drift bytes"
    blocks = drift["differing_blocks"]
    assert drift["differing_block_count"] == len(blocks) == 5, "block count"
    assert drift["first_differing_block_index"] == blocks[0]["index"] == 568, (
        "first differing block index"
    )
    first = blocks[0]
    assert "Internal Bridge" in first["type"], "first drift is not a bridge block"
    assert first["z"] == ";Z:21.4", "first drift layer"
    assert (first["moves_reference"], first["moves_window"]) == (8, 10), (
        "bridge move count"
    )
    assert first["reference_first_moves"] != first["window_first_moves"], (
        "bridge moves identical"
    )
    bridge_blocks = [block for block in blocks if "Internal Bridge" in block["type"]]
    assert len(bridge_blocks) == 2, "expected two differing bridge blocks"
    isolation = drift["isolation"]
    assert isolation["mismatch_polys"] == 0, (
        "the isolation argument needs a clean inside-window set check"
    )
    assert isolation["mismatch_vertices"] == 0, "clean check must be vertex-clean"
    assert abs(float(first["z"].removeprefix(";Z:")) - isolation["print_z"]) < 1e-6, (
        "isolation layer must be the first differing block's layer"
    )
    assert isolation["win_area_polys"] >= 1, "isolation layer has no area"


def load() -> tuple[dict, dict]:
    payload = json.loads((HERE / "probe-rows.json").read_text(encoding="utf-8"))
    drift = json.loads((HERE / "output-drift.json").read_text(encoding="utf-8"))
    return payload, drift


def self_test() -> None:
    payload, drift = load()
    controls: list[tuple[str, dict, dict]] = []

    broken = copy.deepcopy(payload)
    for row in broken["runs"][0]["rows"]:
        row["full_ms"] = row["win_total_ms"]
    broken["runs"][0]["full_ms"] = broken["runs"][0]["window_ms"]
    broken["runs"][0]["saving_ms"] = 0.0
    broken["runs"][0]["wall_equivalent_saving_s"] = 0.0
    controls.append(("collapsed price (full := windowed)", broken, drift))

    broken = copy.deepcopy(payload)
    broken["window_arm"]["sha256"] = REFERENCE_SHA
    broken["window_arm"]["bytes"] = REFERENCE_BYTES
    broken["runs"][1]["sha256"] = REFERENCE_SHA
    controls.append(("window arm reported as the reference", broken, drift))

    broken_drift = copy.deepcopy(drift)
    broken_drift["differing_blocks"][0]["type"] = ";TYPE:Outer wall"
    controls.append(("first drift blamed on a wall block", payload, broken_drift))

    broken_drift = copy.deepcopy(drift)
    broken_drift["isolation"]["mismatch_polys"] = 1
    controls.append(("dirty inside-window check", payload, broken_drift))

    broken = copy.deepcopy(payload)
    broken["runs"][1]["cpu_wall_ratio"] *= 1.5
    controls.append(("second run priced with the wrong cpu/wall", broken, drift))

    broken_drift = copy.deepcopy(drift)
    broken_drift["differing_blocks"] = broken_drift["differing_blocks"][1:]
    broken_drift["differing_block_count"] = len(broken_drift["differing_blocks"])
    controls.append(("dropped the first differing block", payload, broken_drift))

    for name, broken_payload, broken_drift in controls:
        try:
            check(broken_payload, broken_drift)
        except AssertionError:
            continue
        except Exception as error:  # noqa: BLE001 - any rejection counts
            print(f"control rejected with {type(error).__name__}: {name}")
            continue
        raise SystemExit(f"negative control not rejected: {name}")
    print(f"self-test: {len(controls)} negative controls rejected")
    check(payload, drift)
    print("self-test: clean evidence passes")


def main(argv: list[str]) -> int:
    payload, drift = load()
    check(payload, drift)
    for run in payload["runs"]:
        share = run["wall_equivalent_saving_s"] / CELL_WALL
        print(
            f"{run['run']}: saving {run['saving_ms']:.1f} ms = "
            f"{run['wall_equivalent_saving_s']:.3f} s wall-eq = {share * 100:.2f} % "
            f"(floor 0.80 %) sha={run['sha256'][:12]}"
        )
    print(
        f"drift: {drift['differing_block_count']} blocks, first "
        f"{drift['differing_blocks'][0]['type']} {drift['differing_blocks'][0]['z']} "
        f"{drift['differing_blocks'][0]['moves_reference']} -> "
        f"{drift['differing_blocks'][0]['moves_window']} moves; "
        f"inside-window mismatch "
        f"{drift['isolation']['mismatch_polys']} polys"
    )
    if "--self-test" in argv:
        self_test()
    print("PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
