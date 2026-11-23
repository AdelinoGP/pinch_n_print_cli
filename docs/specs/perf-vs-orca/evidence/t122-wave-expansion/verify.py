#!/usr/bin/env python3
"""Checkout-only verifier for the t122 wave-expansion screen.

Reads only the committed `reduced/` files. No raw capture tree, no model
fixtures, no network. Exits 0 when every claim below holds.

`--self-test` mutates each headline claim in memory and asserts the checker
rejects every mutation (negative controls).
"""

from __future__ import annotations

import copy
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
REDUCED = HERE / "reduced"
FLOOR_FRACTION = 0.008

# Cross-checks against the two prior takes, quoted from their FINDINGS:
# t112 measured the canonical unsupported set at 1,307.30 / 4,654.62 mm2.
# t116 measured the canonical flat family at 447.27 / 438.92 mm2.
T112_CANONICAL = {"benchy": 1307.30, "base": 4654.62}
T116_FLAT = {"benchy": 447.27, "base": 438.92}


def close(a: float, b: float, rel: float = 1e-6) -> bool:
    return abs(a - b) <= rel * (1.0 + abs(b))


def load(name: str) -> dict:
    return json.loads((REDUCED / name).read_text(encoding="utf-8"))


def check(benchy: dict, base: dict, ceilings: dict) -> list[str]:
    failures: list[str] = []

    def require(condition: bool, message: str) -> None:
        if not condition:
            failures.append(message)

    for fixture, data, layers in (("benchy", benchy, 239), ("base", base, 494)):
        ms = data["ms"]
        area = data["area_mm2"]

        require(data["layer_count"] == layers, f"{fixture}: layer_count != {layers}")
        require(
            data["first_layer"] == 1, f"{fixture}: first_layer != 1 (layer 0 excluded)"
        )
        require(
            len(data["layers"]) == layers,
            f"{fixture}: len(layers) != {layers}",
        )

        # Aggregate arithmetic must be internally exact.
        require(
            close(
                ms["current_classification"],
                ms["current_classify"] + ms["current_facet"] + ms["current_flat"],
            ),
            f"{fixture}: current_classification sum mismatch",
        )
        require(
            close(
                ms["expansion_only_zone"],
                ms["expansion_solid_zone"]
                + ms["expansion_sparse_zone"]
                + ms["trim_zone"]
                + ms["closing_zone"],
            ),
            f"{fixture}: expansion_only_zone sum mismatch",
        )
        require(
            close(
                ms["classification_only_net"],
                ms["current_classification"] - ms["canonical_classification"],
            ),
            f"{fixture}: classification_only_net mismatch",
        )
        require(
            close(
                ms["combined_net_zone"],
                ms["classification_only_net"] - ms["expansion_only_zone"],
            ),
            f"{fixture}: combined_net_zone mismatch",
        )

        # The expansion is a pure added cost: it must not increase the net.
        require(
            ms["combined_net_zone"] < ms["classification_only_net"],
            f"{fixture}: expansion did not reduce the net",
        )
        require(
            ms["expansion_only_zone"] > 0.0,
            f"{fixture}: expansion cost is not positive",
        )

        # Cross-take reproduction: canonical and flat areas match t112 / t116.
        canonical = area["canonical_bridge"]
        flat = area["flat_family"]
        require(
            abs(canonical - T112_CANONICAL[fixture]) <= 0.001 * T112_CANONICAL[fixture],
            f"{fixture}: canonical area {canonical:.2f} does not reproduce t112 "
            f"{T112_CANONICAL[fixture]:.2f} within 0.1%",
        )
        require(
            abs(flat - T116_FLAT[fixture]) <= 0.001 * T116_FLAT[fixture],
            f"{fixture}: flat area {flat:.2f} does not reproduce t116 "
            f"{T116_FLAT[fixture]:.2f} within 0.1%",
        )

        # Delta bound: every moved area is a disjoint set difference, and the
        # total movement stays below twice the canonical bridge area.
        moved = area["classification_added"] + area["classification_removed"]
        require(moved > 0.0, f"{fixture}: classification delta is empty")
        require(
            moved <= canonical,
            f"{fixture}: classification delta exceeds the canonical bridge area",
        )
        require(
            area["expansion_moved_solid_zone"] >= area["expansion_moved_sparse_zone"],
            f"{fixture}: sparse expansion is not a subset of the solid reach",
        )
        require(
            area["expansion_moved_solid_zone"]
            <= area["expansion_moved_solid_whole_region"],
            f"{fixture}: zone proxy is not tighter than the whole-region proxy",
        )
        require(
            area["closing_measured"] <= area["closing_steiner_bound"],
            f"{fixture}: measured closing exceeds its Steiner bound",
        )
        require(
            area["combined_role_movement_bound"] <= 2.0 * canonical,
            f"{fixture}: combined role-movement bound exceeds 2x the canonical area",
        )

    # Ceiling arithmetic from the t121 walls, recomputed from the raw walls.
    require(ceilings["floor_fraction"] == FLOOR_FRACTION, "floor fraction changed")
    require(len(ceilings["cells"]) == 8, "expected 8 cells")
    for cell in ceilings["cells"]:
        floor = cell["wall_s"] * 1000.0 * FLOOR_FRACTION
        require(close(cell["floor_ms"], floor), f"{cell['cell']}: floor mismatch")
        fixture = "benchy" if cell["cell"].startswith("benchy") else "base"
        net_class = (benchy if fixture == "benchy" else base)["ms"][
            "classification_only_net"
        ]
        net_combined = (benchy if fixture == "benchy" else base)["ms"][
            "combined_net_zone"
        ]
        require(
            close(cell["classification_only_multiple"], net_class / floor),
            f"{cell['cell']}: classification-only multiple mismatch",
        )
        require(
            close(cell["combined_multiple"], net_combined / floor),
            f"{cell['cell']}: combined multiple mismatch",
        )

    # The verdict the map records.
    require(
        all(cell["classification_only_multiple"] >= 1.0 for cell in ceilings["cells"]),
        "classification-only screen does not clear all eight floors",
    )
    benchy_cells = [c for c in ceilings["cells"] if c["cell"].startswith("benchy")]
    base_cells = [c for c in ceilings["cells"] if c["cell"].startswith("base")]
    require(
        all(cell["combined_multiple"] < 0.0 for cell in benchy_cells),
        "a benchy combined ceiling is not negative",
    )
    require(
        all(cell["combined_multiple"] >= 80.0 for cell in base_cells),
        "a base combined ceiling is below 80x",
    )
    require(ceilings["all_eight_clear"] is False, "all_eight_clear should be false")
    require(ceilings["benchy_combined_negative"] is True, "benchy flag should be true")
    require(ceilings["base_combined_clears"] is True, "base flag should be true")

    # The tightest classification-only margin is >= 12x the floor.
    require(
        min(cell["classification_only_multiple"] for cell in ceilings["cells"]) >= 12.0,
        "classification-only margin fell below 12x",
    )
    # The base combined margin is >= 80x; the benchy combined is <= 0.
    require(
        min(cell["combined_multiple"] for cell in base_cells) >= 80.0,
        "base combined margin fell below 80x",
    )

    return failures


def mutate(data: dict, path: list[str], value) -> None:
    cursor = data
    for key in path[:-1]:
        cursor = cursor[key]
    cursor[path[-1]] = value


def self_test(benchy: dict, base: dict, ceilings: dict) -> int:
    mutations = [
        (
            "canonical area +10%",
            "benchy",
            ["area_mm2", "canonical_bridge"],
            benchy["area_mm2"]["canonical_bridge"] * 1.10,
        ),
        (
            "flat area +10%",
            "base",
            ["area_mm2", "flat_family"],
            base["area_mm2"]["flat_family"] * 1.10,
        ),
        ("layer count 238", "benchy", ["layer_count"], 238),
        (
            "classification-only net negative",
            "benchy",
            ["ms", "classification_only_net"],
            -1.0,
        ),
        ("combined benchy positive", "benchy", ["ms", "combined_net_zone"], 1.0),
        (
            "current sum inconsistent",
            "base",
            ["ms", "current_classification"],
            base["ms"]["current_classification"] + 1.0,
        ),
        (
            "sparse reach larger than solid",
            "base",
            ["area_mm2", "expansion_moved_sparse_zone"],
            base["area_mm2"]["expansion_moved_solid_zone"] + 1.0,
        ),
        (
            "closing above Steiner",
            "benchy",
            ["area_mm2", "closing_measured"],
            benchy["area_mm2"]["closing_steiner_bound"] + 1.0,
        ),
        (
            "delta above canonical",
            "benchy",
            ["area_mm2", "classification_added"],
            benchy["area_mm2"]["canonical_bridge"] + 1.0,
        ),
        ("all-eight flag flipped", "ceilings", ["all_eight_clear"], True),
        ("base flag flipped", "ceilings", ["base_combined_clears"], False),
        ("expansion no longer a cost", "benchy", ["ms", "expansion_only_zone"], 0.0),
        (
            "combined net above classification net",
            "base",
            ["ms", "combined_net_zone"],
            base["ms"]["classification_only_net"] + 1.0,
        ),
        ("floor fraction changed", "ceilings", ["floor_fraction"], 0.01),
    ]
    rejected = 0
    for label, target, path, value in mutations:
        b, s, c = copy.deepcopy(benchy), copy.deepcopy(base), copy.deepcopy(ceilings)
        mutate({"benchy": b, "base": s, "ceilings": c}[target], path, value)
        if check(b, s, c):
            rejected += 1
        else:
            print(f"CONTROL NOT REJECTED: {label}")
    print(f"self-test: {rejected}/{len(mutations)} negative controls rejected")
    return 0 if rejected == len(mutations) else 1


def main() -> int:
    benchy = load("summary-benchy.json")
    base = load("summary-base.json")
    ceilings = load("ceilings.json")

    if "--self-test" in sys.argv:
        return self_test(benchy, base, ceilings)

    failures = check(benchy, base, ceilings)
    if failures:
        for failure in failures:
            print(f"FAIL: {failure}")
        return 1
    print("t122 verifier: PASS (all claims hold against reduced/)")
    print(
        "classification-only net: "
        f"benchy {benchy['ms']['classification_only_net']:.1f} ms, "
        f"base {base['ms']['classification_only_net']:.1f} ms"
    )
    print(
        "combined net (tight zone proxy): "
        f"benchy {benchy['ms']['combined_net_zone']:.1f} ms, "
        f"base {base['ms']['combined_net_zone']:.1f} ms"
    )
    print(
        "all eight clear: "
        f"{ceilings['all_eight_clear']} (benchy negative: "
        f"{ceilings['benchy_combined_negative']}, base clear: "
        f"{ceilings['base_combined_clears']})"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
