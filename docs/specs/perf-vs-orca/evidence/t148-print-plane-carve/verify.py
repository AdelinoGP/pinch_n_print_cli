#!/usr/bin/env python3
"""Verify every claim in the t148 FINDINGS from `census.json`.

Stdlib only. Reads only `census.json` (the in-repo reduced artifact), so a plain
checkout can check every number without the gitignored raw tree.

    python verify.py
    python verify.py --self-test   # negative controls: each check must reject

Exit 0 = every check passed.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

OUT = Path(__file__).resolve().parent
CENSUS = OUT / "census.json"

EXPECTED_BEFORE_DIAGNOSTICS = {
    "benchy-classic-on": 101,
    "benchy-arachne-on": 101,
    "base-classic-on": 44_474,
    "base-arachne-on": 44_474,
}
EXPECTED_RECOVERED_LAYERS = {
    "benchy-classic-on": 1,
    "benchy-arachne-on": 1,
    "base-classic-on": 32,
    "base-arachne-on": 32,
}
SUPPORTS_OFF = [
    "benchy-classic-off",
    "benchy-arachne-off",
    "base-classic-off",
    "base-arachne-off",
]


class Checker:
    def __init__(self, census: dict) -> None:
        self.census = census
        self.checks = 0

    def check(self, condition: bool, label: str) -> None:
        self.checks += 1
        if not condition:
            raise AssertionError(label)

    def run(self) -> int:
        self.check(
            self.census.get("schema") == "t148-print-plane-carve/v1", "schema tag"
        )

        cells = self.census["cells"]
        coverage = self.census["coverage"]
        for cell, row in cells.items():
            before = row["before"]
            after = row["after"]
            self.check(
                before["diagnostics"] == EXPECTED_BEFORE_DIAGNOSTICS[cell],
                f"{cell}: pre-fix diagnostics are the t147 count",
            )
            self.check(
                after["diagnostics"] == 0,
                f"{cell}: zero exact-Z diagnostics after the fix",
            )
            self.check(before["degraded"] is True, f"{cell}: pre-fix run was degraded")
            self.check(
                after["degraded"] is False, f"{cell}: post-fix run is not degraded"
            )

            self.check(row["lost_z"] == [], f"{cell}: no support layer disappeared")
            self.check(
                len(row["recovered_z"]) == EXPECTED_RECOVERED_LAYERS[cell],
                f"{cell}: recovered layer count",
            )
            self.check(
                after["support_layers"]
                == before["support_layers"] + len(row["recovered_z"]),
                f"{cell}: layer count equals before + recovered",
            )
            self.check(
                len(row["recovered_layers"]) == len(row["recovered_z"]),
                f"{cell}: every recovered layer has a census row",
            )
            for entry in row["recovered_layers"]:
                self.check(
                    entry["path_mm"] > 0.0,
                    f"{cell}: recovered layer {entry['layer']} prints support",
                )

            recovered_sum = sum(entry["path_mm"] for entry in row["recovered_layers"])
            delta = row["delta_support_path_mm"]
            self.check(delta > 0.0, f"{cell}: total support path grew")
            self.check(
                recovered_sum >= 0.9 * delta,
                f"{cell}: the recovered layers account for the path gain",
            )
            self.check(
                recovered_sum <= 1.1 * delta,
                f"{cell}: no gain outside the recovered layers exceeds the noise band",
            )
            self.check(
                recovered_sum >= 0.9 * row["t147_missing_path_mm"],
                f"{cell}: recovery is at least the t147 neighbour-interpolation estimate",
            )

            wall = row["wall"]
            self.check(
                wall["pnp-ordinary"]["runs"] == 3 and wall["orca"]["runs"] == 3,
                f"{cell}: three measured runs per tool",
            )
            self.check(
                abs(
                    wall["ratio"]
                    - wall["pnp-ordinary"]["wall_median_s"]
                    / wall["orca"]["wall_median_s"]
                )
                < 0.01,
                f"{cell}: ratio is the pnp/orca wall median quotient",
            )
            self.check(
                wall["pnp-ordinary"]["cpu_median_s"] > wall["orca"]["cpu_median_s"],
                f"{cell}: process CPU corroborates the wall ordering",
            )
            self.check(
                all(sample > 0 for sample in wall["pnp-ordinary"]["wall_samples_s"]),
                f"{cell}: no non-positive PnP sample",
            )

            # t147 recorded zero PnP coverage on every immediatly-above-overhang
            # candidate plane; the fix must leave that before-state behind as
            # history.
            for candidate in coverage[cell]:
                self.check(
                    candidate["known"],
                    f"{cell}: candidate {candidate['candidate_id']} known",
                )
                self.check(
                    candidate["pnp_coverage_before"] == 0.0,
                    f"{cell}: candidate {candidate['candidate_id']} had zero PnP coverage before",
                )
                self.check(
                    candidate["plane_mm"] is not None,
                    f"{cell}: candidate {candidate['candidate_id']} has a support plane after",
                )

        for cell in SUPPORTS_OFF:
            row = self.census["supports_off"][cell]
            self.check(
                row["matches_active"],
                f"{cell}: byte-identical to the active reference set",
            )
            self.check(
                row["gcode_sha256"] == row["active_sha256"],
                f"{cell}: hash equals active set",
            )
            self.check(row["diagnostics"] == 0, f"{cell}: zero exact-Z diagnostics")
            self.check(
                row["support_path_mm"] == 0.0,
                f"{cell}: no support in a supports-off cell",
            )

        # The in-repo reference set must name exactly the hashes this census
        # measured, with the superseded supports-on set genuinely retired.
        reference = json.loads((OUT / "reference.json").read_text(encoding="utf-8"))
        for cell in cells:
            self.check(
                cells[cell]["after"]["gcode_sha256"] == reference["active_set"][cell],
                f"{cell}: census hash equals reference.json active_set",
            )
            self.check(
                reference["active_set"][cell] != reference["superseded_set"][cell],
                f"{cell}: supports-on hash is not the superseded one",
            )
        for cell in SUPPORTS_OFF:
            self.check(
                self.census["supports_off"][cell]["gcode_sha256"]
                == reference["active_set"][cell],
                f"{cell}: census hash equals reference.json active_set",
            )
        self.check(
            reference["supports_off_unchanged"] is True,
            "reference.json marks supports-off unchanged",
        )

        # The coverage metric is not a PnP criterion: ordinary retained layers
        # show the same near-zero reading, so the recovered layers' reading is
        # not evidence of missing support.
        control_values = [
            item["pnp_coverage"]
            for rows in self.census["retained_layer_control"].values()
            for item in rows
            if item.get("known")
        ]
        self.check(len(control_values) >= 6, "retained-layer control has samples")
        self.check(
            max(control_values) < 0.05,
            "retained-layer control stays under the 5 % coverage reading",
        )
        return self.checks


def negative_controls(census: dict) -> list[str]:
    """Reject a check by mutating a deep copy; return the labels that fired."""
    import copy

    fired: list[str] = []

    def expect_failure(label: str, mutate) -> None:
        probe = copy.deepcopy(census)
        mutate(probe)
        try:
            Checker(probe).run()
        except AssertionError:
            fired.append(label)
        except Exception:  # noqa: BLE001 - a malformed probe is also a rejection
            fired.append(label)

    def diagnostics(probe):
        probe["cells"]["benchy-classic-on"]["after"]["diagnostics"] = 1

    def off_hash(probe):
        probe["supports_off"]["base-classic-off"]["gcode_sha256"] = "0" * 64

    def delta(probe):
        probe["cells"]["base-classic-on"]["delta_support_path_mm"] = -1.0

    def recovered_path(probe):
        probe["cells"]["base-classic-on"]["recovered_layers"][0]["path_mm"] = 0.0

    def lost_layer(probe):
        probe["cells"]["base-classic-on"]["lost_z"] = [3.2]

    def ratio(probe):
        probe["cells"]["benchy-classic-on"]["wall"]["ratio"] = 9.99

    def coverage_before(probe):
        probe["coverage"]["base-classic-on"][0]["pnp_coverage_before"] = 0.5

    def control(probe):
        probe["retained_layer_control"]["base-classic-on"][0]["pnp_coverage"] = 0.9

    def reference_hash(probe):
        probe["cells"]["base-arachne-on"]["after"]["gcode_sha256"] = "0" * 64

    for label, mutate in [
        ("after diagnostics", diagnostics),
        ("supports-off identity", off_hash),
        ("path delta sign", delta),
        ("recovered layer path", recovered_path),
        ("lost layer", lost_layer),
        ("wall ratio", ratio),
        ("before coverage", coverage_before),
        ("retained control", control),
        ("reference active hash", reference_hash),
    ]:
        expect_failure(label, mutate)
    return fired


def main(argv: list[str]) -> int:
    census = json.loads(CENSUS.read_text(encoding="utf-8"))
    if "--self-test" in argv:
        fired = negative_controls(census)
        print(f"negative controls rejected: {len(fired)}/9")
        for label in fired:
            print(f"  rejected: {label}")
        return 0 if len(fired) == 9 else 1
    checks = Checker(census).run()
    print(f"t148 verify: {checks}/{checks} checks passed")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
