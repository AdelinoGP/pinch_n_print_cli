#!/usr/bin/env python3
"""t154 verification -- checkout-only checks over the committed evidence.

Every claim is re-derived from the committed files; nothing needs the
gitignored raw streams. `--self-test` applies one mutation per family and
requires the verifier to reject each of them, so a green run cannot be green
by vacuity.

Usage:
  python verify.py             # 0 = every claim holds
  python verify.py --self-test # every mutation must be detected
"""

from __future__ import annotations

import copy
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
T151 = HERE.parent / "t151-skirt-hull-contour" / "reference.json"
FLOOR = 0.008


def load(name: str) -> dict:
    return json.loads((HERE / name).read_text(encoding="utf-8"))


def canonical_prepass(cell: str) -> float:
    fixture, _generator, supports = cell.split("-", 2)
    return {
        "benchy-off": 0.0505,
        "benchy-on": 0.5498,
        "base-off": 2.1030,
        "base-on": 10.4448,
    }[f"{fixture}-{supports}"]


def check(summary: dict, arith: dict, carriers: dict, reference: dict) -> list[str]:
    failures: list[str] = []
    cells = summary["cells"]
    active = reference["active_set"]

    if len(cells) != 8:
        failures.append(f"expected 8 cells, found {len(cells)}")

    for cell, red in sorted(cells.items()):
        # 1. Phase reconciliation: the four phase brackets plus the outside
        #    residual are the instrumented slice wall.
        inner = sum(v for v in red["phases_ms"].values())
        if inner + red["outside_phases_ms"] != red["slice_complete_ms"]:
            failures.append(f"{cell}: phases + outside != slice_complete")

        # 2. Validation fields.
        if red["degraded"] is not False:
            failures.append(f"{cell}: degraded is not false")
        if red["fatal_error_count"] != 0 or red["non_fatal_error_count"] != 0:
            failures.append(f"{cell}: non-zero error counts")

        # 3. Output identity against t151's active set.
        if cell in active and red["gcode_sha256"] != active[cell]:
            failures.append(f"{cell}: gcode hash differs from t151's active set")

        # 4. Busy union can never exceed the phase wall, and the host gap is
        #    the difference.
        detail = red["phase_detail"]["per_layer"]
        if detail["module_busy_union_ms"] > detail["wall_ms"]:
            failures.append(f"{cell}: per-layer busy union exceeds the phase wall")
        if detail["host_gap_ms"] != detail["wall_ms"] - detail["module_busy_union_ms"]:
            failures.append(f"{cell}: per-layer host gap is not wall - union")

        # 5. The per-layer module thread sum must exceed the phase wall (the
        #    phase runs on the rayon pool) and every module sum must be positive.
        thread_sum = sum(red["module_thread_sum_ms"]["per_layer"].values())
        if thread_sum < detail["wall_ms"]:
            failures.append(f"{cell}: per-layer thread sum below the phase wall")
        if any(v < 0 for v in red["module_thread_sum_ms"]["per_layer"].values()):
            failures.append(f"{cell}: negative module thread sum")

        # 6. Arithmetic re-derivation.
        row = arith["cells"].get(cell)
        if row is None:
            failures.append(f"{cell}: missing from arithmetic.json")
            continue
        pnp_s = row["ratio_of_record"] * row["orca_s"]
        if abs(pnp_s - row["pnp_s"]) > 5e-3:
            failures.append(f"{cell}: pnp_s is not ratio * orca")
        other_s = (
            (
                (
                    red["phases_ms"]["per_layer"]
                    + red["phases_ms"]["postpass"]
                    + red["phases_ms"]["validation"]
                )
                + red["outside_phases_ms"]
            )
            / red["slice_complete_ms"]
            * pnp_s
        )
        prepass_s = red["phases_ms"]["prepass"] / red["slice_complete_ms"] * pnp_s
        canon = canonical_prepass(cell)
        result = (canon + other_s) / row["orca_s"]
        required = 1 - (row["orca_s"] - other_s) / prepass_s
        if abs(result - row["prepass_at_canonical_result"]) > 1e-3:
            failures.append(f"{cell}: prepass-at-canonical result does not re-derive")
        if abs(required - row["required_prepass_cut"]) > 1e-3:
            failures.append(f"{cell}: required prepass cut does not re-derive")
        if abs((1 - result) - row["margin_at_canonical"]) > 1e-3:
            failures.append(f"{cell}: margin does not re-derive")
        if row["canonical_prepass_s"] != canon:
            failures.append(f"{cell}: canonical prepass cost differs from t110's table")

        # 7. Carrier table consistency.
        car = carriers["cells"].get(cell)
        if car is None:
            failures.append(f"{cell}: missing from carriers.json")
            continue
        if (
            sum(c["thread_sum_ms"] for c in car["carriers"])
            != car["per_layer_thread_sum_ms"]
        ):
            failures.append(f"{cell}: carrier thread sum != per-layer thread sum")
        if car["per_layer_thread_sum_ms"] != thread_sum:
            failures.append(f"{cell}: carrier table disagrees with summary.json")
        parallel = thread_sum / detail["wall_ms"]
        if abs(parallel - car["effective_parallelism"]) > 0.01:
            failures.append(f"{cell}: effective parallelism does not re-derive")
        for c in car["carriers"]:
            ceiling = c["thread_sum_ms"] / parallel
            if abs(ceiling - c["wall_ceiling_ms"]) > 0.1:
                failures.append(f"{cell}/{c['module_id']}: ceiling does not re-derive")

    # 8. The headline claims the ticket states.
    winners = [
        c for c, r in arith["cells"].items() if r["prepass_at_canonical_result"] < 1.0
    ]
    if len(winners) != 7:
        failures.append(
            f"expected 7 cells to win at canonical prepass, found {len(winners)}"
        )
    loser = set(arith["cells"]) - set(winners)
    if loser != {"base-classic-on"}:
        failures.append(
            f"expected base-classic-on to be the only loser, got {sorted(loser)}"
        )
    if (
        not 1.0
        <= arith["cells"]["base-classic-on"]["needed_from_other_than_prepass_s"]
        <= 1.2
    ):
        failures.append(
            "base-classic-on's residual requirement is outside the stated band"
        )
    # The four benchy cells must all clear the line with >= 9 % margin.
    for cell in (
        "benchy-classic-off",
        "benchy-classic-on",
        "benchy-arachne-off",
        "benchy-arachne-on",
    ):
        if arith["cells"][cell]["margin_at_canonical"] < 0.09:
            failures.append(f"{cell}: benchy margin below the stated 9 % floor")

    # 9. The profile arm's carrier scopes are present and small (the per-op
    #    decomposition's measured limit: the carriers are in-module work).
    for cell in ("benchy-classic-off", "base-classic-off", "base-classic-on"):
        prof = load(f"profile-{cell}.json")
        if prof["cell"] != cell or prof["fuel_total"] <= 0:
            failures.append(f"{cell}: profile summary malformed")
        mods = {m["module_id"]: m for m in prof["modules"]}
        for module_id in ("com.core.classic-perimeters", "com.core.infill-linker"):
            if module_id not in mods:
                failures.append(f"{cell}: profile missing {module_id}")
                continue
            m = mods[module_id]
            scoped = sum(s["self_fuel"] for s in m["scopes"])
            # The named scope rows must not explain most of the module's fuel:
            # that is the finding that the carriers are in-module kernels.
            if scoped / m["total_fuel"] > 0.60:
                failures.append(f"{cell}/{module_id}: scope rows explain >60 % of fuel")

    return failures


MUTATIONS = [
    (
        "gcode hash",
        lambda s, a, c, r: s["cells"]["benchy-classic-off"].__setitem__(
            "gcode_sha256", "0" * 64
        ),
    ),
    (
        "phase reconciliation",
        lambda s, a, c, r: s["cells"]["benchy-classic-off"]["phases_ms"].__setitem__(
            "per_layer", 1
        ),
    ),
    (
        "degraded flag",
        lambda s, a, c, r: s["cells"]["benchy-classic-off"].__setitem__(
            "degraded", True
        ),
    ),
    (
        "fatal count",
        lambda s, a, c, r: s["cells"]["benchy-classic-off"].__setitem__(
            "fatal_error_count", 1
        ),
    ),
    (
        "busy union",
        lambda s, a, c, r: s["cells"]["benchy-classic-off"]["phase_detail"][
            "per_layer"
        ].__setitem__("module_busy_union_ms", 10**9),
    ),
    (
        "carrier sum",
        lambda s, a, c, r: c["cells"]["benchy-classic-off"]["carriers"][0].__setitem__(
            "thread_sum_ms", 1
        ),
    ),
    (
        "required cut",
        lambda s, a, c, r: a["cells"]["benchy-classic-off"].__setitem__(
            "required_prepass_cut", 0.5
        ),
    ),
    (
        "margin",
        lambda s, a, c, r: a["cells"]["base-classic-off"].__setitem__(
            "margin_at_canonical", 0.5
        ),
    ),
    (
        "loser set",
        lambda s, a, c, r: a["cells"]["base-classic-on"].__setitem__(
            "prepass_at_canonical_result", 0.9
        ),
    ),
    (
        "benchy margin",
        lambda s, a, c, r: a["cells"]["benchy-arachne-on"].__setitem__(
            "margin_at_canonical", 0.05
        ),
    ),
    (
        "oracle hash",
        lambda s, a, c, r: r["active_set"].__setitem__("benchy-classic-off", "1" * 64),
    ),
]


def main() -> int:
    summary, arith, carriers, reference = (
        load("summary.json"),
        load("arithmetic.json"),
        load("carriers.json"),
        load_json(T151),
    )
    failures = check(summary, arith, carriers, reference)
    if "--self-test" not in sys.argv:
        for f in failures:
            print(f"FAIL: {f}")
        print("PASS" if not failures else f"FAIL ({len(failures)} claims)")
        return 1 if failures else 0

    detected = 0
    for name, mutate in MUTATIONS:
        s, a, c, r = (
            copy.deepcopy(summary),
            copy.deepcopy(arith),
            copy.deepcopy(carriers),
            copy.deepcopy(reference),
        )
        mutate(s, a, c, r)
        try:
            mutated = check(s, a, c, r)
        except Exception:  # a mutation that breaks the shape is a detection
            mutated = ["raised"]
        if mutated:
            detected += 1
            print(f"OK   rejected: {name}")
        else:
            print(f"MISS accepted: {name}")
    print(f"{detected}/{len(MUTATIONS)} negative controls rejected")
    return 0 if detected == len(MUTATIONS) else 1


def load_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


if __name__ == "__main__":
    raise SystemExit(main())
