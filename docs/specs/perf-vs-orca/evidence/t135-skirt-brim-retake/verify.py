#!/usr/bin/env python3
"""Re-derive every ticket 135 Stage 1 claim from ``reduced/summary.json``.

The reduced file is the committed evidence; the raw ``target/t135`` tree is
optional. ``--self-test`` mutates the summary in memory and asserts the checks
reject each mutation (negative controls), so a check that silently stops
falsifying anything is caught here.

The oracle is checked against what the FINDINGS actually claims: **presence
parity on every cell, radial extent on every cell, skirt geometry on the
supports-off cells, and a strict 8/8 geometric form that is NOT met** — the two
passing cells and the named residual classes are asserted so a future re-run
cannot quietly turn the disclosure into a blanket pass.
"""

from __future__ import annotations

import copy
import json
import re
import statistics
import sys
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
SUPPORTS_OFF = [c for c in CELLS if c.endswith("-off")]
STRICT_PASS = ["benchy-classic-off", "benchy-arachne-off"]
HEX64 = re.compile(r"^[0-9a-f]{64}$")

# Ticket 123's candidate-ordinary reference set (the baseline this take
# replaces). Cited in evidence/t123-facet-retirement/reduced/summary.json.
T123 = {
    "benchy-classic-off": "90638bf3ea9e592d573b223c3eb9af64473efe804578d2478adfbbc5f0e67008",
    "benchy-classic-on": "711c6334e397e5ae6911b15bfc71d42add9e93822cddd0a29391db6ef531d556",
    "benchy-arachne-off": "2ba5d84ccf3c8deea6f39f4977dd71221faaecde9987d1699b361a5366e86380",
    "benchy-arachne-on": "a972b0f2c25a988e667ecf5a5b3d6c3f5ba87f5c8872733c2a4b4e5d51a9695e",
    "base-classic-off": "b3b4ac5e885c7f637b98f79eca21901a41d85739627d4dcb40820663c15e90ef",
    "base-classic-on": "4e31a2db368ff80f388512211ae824b9660342a465c87f093f6182023e34eb4e",
    "base-arachne-off": "6b865d4a2cb892daa3172add3e3677e0dc35192bcedd0acbc463abca7064d715",
    "base-arachne-on": "62d6eedc02e5db972928f830104aab95c3598f6df6407159cb5438adb35bef13",
}


def check(summary: dict) -> list[str]:
    """Returns the list of failures; empty means every claim holds."""
    fails: list[str] = []

    def require(cond: bool, msg: str) -> None:
        if not cond:
            fails.append(msg)

    # ---- arms -------------------------------------------------------------
    manifest = summary.get("manifest", {})
    require(len(manifest) == 4, "manifest must carry four arms")
    if len(manifest) == 4:
        require(
            manifest["baseline-ordinary"]["exe_sha256"]
            == manifest["candidate-ordinary"]["exe_sha256"],
            "ordinary arms must share one host binary (guest-only change)",
        )
        require(
            manifest["baseline-accelerated"]["exe_sha256"]
            == manifest["candidate-accelerated"]["exe_sha256"],
            "accelerated arms must share one host binary",
        )
        require(
            manifest["baseline-ordinary"]["skirt_brim_wasm_sha256"]
            != manifest["candidate-ordinary"]["skirt_brim_wasm_sha256"],
            "baseline and candidate must carry different skirt-brim wasm",
        )

    # ---- output campaign --------------------------------------------------
    output = summary.get("output", {})
    for cell in CELLS:
        require(cell in output, f"output: missing cell {cell}")
        arms = output.get(cell, {})
        for arm in (
            "baseline-ordinary",
            "baseline-accelerated",
            "candidate-ordinary",
            "candidate-accelerated",
        ):
            require(arm in arms, f"output[{cell}]: missing arm {arm}")
            if arm in arms:
                require(
                    arms[arm]["exit_code"] == 0, f"output[{cell}][{arm}]: non-zero exit"
                )
                require(
                    HEX64.match(arms[arm]["sha256"]) is not None,
                    f"output[{cell}][{arm}]: bad sha",
                )
                require(
                    arms[arm]["gcode_bytes"] > 0, f"output[{cell}][{arm}]: empty gcode"
                )
        if all(
            a in arms
            for a in (
                "baseline-ordinary",
                "baseline-accelerated",
                "candidate-ordinary",
                "candidate-accelerated",
            )
        ):
            require(
                arms["baseline-ordinary"]["sha256"]
                == arms["baseline-accelerated"]["sha256"],
                f"output[{cell}]: baseline arms are not byte-identical",
            )
            require(
                arms["candidate-ordinary"]["sha256"]
                == arms["candidate-accelerated"]["sha256"],
                f"output[{cell}]: candidate arms are not byte-identical",
            )
            require(
                arms["baseline-ordinary"]["sha256"] == T123[cell],
                f"output[{cell}]: baseline does not reproduce ticket 123's reference",
            )
            require(
                arms["candidate-ordinary"]["sha256"]
                != arms["baseline-ordinary"]["sha256"],
                f"output[{cell}]: port moved no output line (candidate == baseline)",
            )

    # ---- delta audit ------------------------------------------------------
    audit = summary.get("audit", {})
    require(bool(audit.get("pass")), "audit: failed")
    audit_cells = {entry["cell"]: entry for entry in audit.get("cells", [])}
    for cell in CELLS:
        require(cell in audit_cells, f"audit: missing cell {cell}")
        if cell in audit_cells:
            entry = audit_cells[cell]
            for section in entry["geometry_sections"]:
                require(
                    section["identical"],
                    f"audit[{cell}]: motion moved under feature {section['feature']!r}",
                )
            require(
                entry["e_stats"]["max_abs_delta_mm"] <= entry["e_stats"]["tolerance_mm"],
                f"audit[{cell}]: E delta {entry['e_stats']['max_abs_delta_mm']} above tolerance",
            )
            census = entry["line_census"]
            for feature in ("Skirt", "Brim"):
                require(
                    census.get(feature, {}).get("removed", 0) + census.get(feature, {}).get("added", 0) > 0,
                    f"audit[{cell}]: no {feature} lines moved",
                )
            require(
                census.get("<config>", {}).get("added", 0) >= 2,
                f"audit[{cell}]: config block did not record the two new keys",
            )

    # ---- parity oracle ----------------------------------------------------
    oracle = summary.get("oracle", {})
    strict_pass: list[str] = []
    for cell in CELLS:
        require(cell in oracle, f"oracle: missing cell {cell}")
        if cell not in oracle:
            continue
        checks = oracle[cell]["checks"]
        require(
            bool(checks.get("brim_presence")),
            f"oracle[{cell}]: brim presence disagrees",
        )
        require(
            bool(checks.get("skirt_presence")),
            f"oracle[{cell}]: skirt presence disagrees",
        )
        require(
            checks["brim_growth_abs_diff_mm"] <= 1.5,
            f"oracle[{cell}]: brim growth delta {checks['brim_growth_abs_diff_mm']} mm",
        )
        require(
            checks["skirt_growth_abs_diff_mm"] <= 1.5,
            f"oracle[{cell}]: skirt growth delta {checks['skirt_growth_abs_diff_mm']} mm",
        )
        if bool(checks.get("pass")):
            strict_pass.append(cell)
        for role in ("Brim", "Skirt"):
            require(
                oracle[cell]["pnp"].get(role, {}).get("present") is not None,
                f"oracle[{cell}]: missing PnP {role} presence",
            )
            require(
                oracle[cell]["orca"].get(role, {}).get("present") is not None,
                f"oracle[{cell}]: missing Orca {role} presence",
            )

    require(
        strict_pass == STRICT_PASS,
        f"oracle: strict pass set is {strict_pass}, expected {STRICT_PASS} "
        "(the strict 8/8 form is disclosed as NOT met)",
    )
    # The residual must stay in the two named classes, not spread.
    for cell in CELLS:
        if cell in STRICT_PASS:
            continue
        checks = oracle[cell]["checks"]
        require(
            checks["brim_swept_area_rel_diff"] > 0.12
            or checks["skirt_outer_loop_area_rel_diff"] > 0.03,
            f"oracle[{cell}]: fails strict form for an unnamed reason",
        )

    # ---- old-vs-new A/B ---------------------------------------------------
    ab = summary.get("ab", {})
    for cell in CELLS:
        require(cell in ab, f"ab: missing cell {cell}")
        if cell not in ab:
            continue
        entry = ab[cell]
        pairs = entry.get("pairs", {})
        require(len(pairs) == 3, f"ab[{cell}]: expected 3 pairs, got {len(pairs)}")
        for name, pair in pairs.items():
            require(
                pair["order"]
                in (
                    "baseline-ordinary>candidate-ordinary",
                    "candidate-ordinary>baseline-ordinary",
                ),
                f"ab[{cell}][{name}]: unknown order {pair['order']!r}",
            )
        for arm in ("baseline", "candidate"):
            values = entry[arm]["wall_values"]
            require(len(values) == 3, f"ab[{cell}][{arm}]: expected 3 wall values")
            if values:
                recomputed = round(statistics.median(values), 4)
                require(
                    abs(recomputed - entry[arm]["wall_median"]) <= 1e-3,
                    f"ab[{cell}][{arm}]: median {entry[arm]['wall_median']} != recomputed {recomputed}",
                )
        require(
            0.95 <= entry["wall_ratio"] <= 1.10,
            f"ab[{cell}]: wall ratio {entry['wall_ratio']} out of band",
        )
        require(
            0.95 <= entry["cpu_ratio"] <= 1.05,
            f"ab[{cell}]: cpu ratio {entry['cpu_ratio']} out of band",
        )
        require(
            not (entry["wall_signs"] == 3 and entry["cpu_signs"] == 3),
            f"ab[{cell}]: a corroborated 3/3 win would contradict the noise-floor claim",
        )

    # ---- scoreboard -------------------------------------------------------
    scoreboard = summary.get("scoreboard", {})
    require(scoreboard.get("clean") is True, "scoreboard: a PnP run was not clean")
    sb_cells = scoreboard.get("cells", {})
    for cell in CELLS:
        require(cell in sb_cells, f"scoreboard: missing cell {cell}")
        if cell not in sb_cells:
            continue
        entry = sb_cells[cell]
        orca = entry["orca"]
        for tool in ("pnp-ordinary", "pnp-accelerated"):
            pnp = entry[tool]
            require(
                len(pnp["sha256"]) == 1,
                f"scoreboard[{cell}][{tool}]: more than one hash",
            )
            require(
                pnp["sha256"] == entry["pnp-ordinary"]["sha256"],
                f"scoreboard[{cell}]: ordinary and accelerated bytes differ",
            )
            require(
                pnp["degraded"] == ["false"], f"scoreboard[{cell}][{tool}]: degraded"
            )
            require(
                pnp["fatal"] in ([], [0]), f"scoreboard[{cell}][{tool}]: fatal errors"
            )
            resolved = pnp["resolved"]
            require(
                resolved["brim_width"] == ["8"],
                f"scoreboard[{cell}][{tool}]: brim width {resolved['brim_width']}",
            )
            require(
                resolved["skirt_loops"] == ["6"],
                f"scoreboard[{cell}][{tool}]: skirt loops {resolved['skirt_loops']}",
            )
            require(
                resolved["skirt_distance"] == ["3"],
                f"scoreboard[{cell}][{tool}]: skirt distance {resolved['skirt_distance']}",
            )
            require(
                resolved["skirt_height"] == ["1"],
                f"scoreboard[{cell}][{tool}]: skirt height {resolved['skirt_height']}",
            )
            require(
                2.0 <= entry["ratio_ordinary"] <= 5.0,
                f"scoreboard[{cell}]: ratio out of band",
            )
        require(
            orca["resolved"]["brim_type"] == ["outer_only"],
            f"scoreboard[{cell}]: Orca brim type not pinned",
        )
        require(
            orca["resolved"]["brim_width"] == ["8"],
            f"scoreboard[{cell}]: Orca brim width not 8",
        )
        require(
            orca["resolved"]["skirt_loops"] == ["6"],
            f"scoreboard[{cell}]: Orca skirt loops not 6",
        )
        require(
            orca["resolved"]["skirt_distance"] == ["3"],
            f"scoreboard[{cell}]: Orca skirt distance not 3",
        )
        require(
            orca["resolved"]["skirt_height"] == ["1"],
            f"scoreboard[{cell}]: Orca skirt height not 1",
        )
        require(
            abs(entry["required_cut_ordinary"] - (1.0 - 1.0 / entry["ratio_ordinary"]))
            < 1e-3,
            f"scoreboard[{cell}]: required-cut arithmetic inconsistent",
        )

    return fails


def self_test(summary: dict) -> int:
    """Each mutation must be rejected; a mutation that survives is a hole."""
    mutations = []

    def mutate(name: str, fn) -> None:
        broken = copy.deepcopy(summary)
        fn(broken)
        mutations.append((name, broken))

    mutate(
        "ordinary binaries differ",
        lambda s: s["manifest"]["candidate-ordinary"].update({"exe_sha256": "0" * 64}),
    )
    mutate(
        "wasm hashes equal",
        lambda s: s["manifest"]["candidate-ordinary"].update(
            {
                "skirt_brim_wasm_sha256": s["manifest"]["baseline-ordinary"][
                    "skirt_brim_wasm_sha256"
                ]
            }
        ),
    )
    mutate(
        "baseline drifted from t123",
        lambda s: s["output"]["benchy-classic-off"]["baseline-ordinary"].update(
            {"sha256": "0" * 64}
        ),
    )
    mutate(
        "candidate == baseline",
        lambda s: s["output"]["base-classic-on"]["candidate-ordinary"].update(
            {"sha256": s["output"]["base-classic-on"]["baseline-ordinary"]["sha256"]}
        ),
    )
    mutate(
        "structural line moved",
        lambda s: s["audit"]["cells"][0]["geometry_sections"][0].update(
            {"identical": False}
        ),
    )
    mutate("audit flag", lambda s: s["audit"].update({"pass": False}))
    mutate(
        "brim presence disagrees",
        lambda s: s["oracle"]["base-classic-on"]["checks"].update(
            {"brim_presence": False}
        ),
    )
    mutate(
        "brim growth out of band",
        lambda s: s["oracle"]["base-classic-off"]["checks"].update(
            {"brim_growth_abs_diff_mm": 9.0}
        ),
    )
    mutate(
        "E delta above tolerance",
        lambda s: s["audit"]["cells"][0]["e_stats"].update({"max_abs_delta_mm": 0.5}),
    )
    mutate(
        "strict pass set widened",
        lambda s: s["oracle"]["base-classic-off"]["checks"].update({"pass": True}),
    )
    mutate(
        "median tampered",
        lambda s: s["ab"]["benchy-arachne-off"]["baseline"].update(
            {"wall_median": 1.0}
        ),
    )
    mutate("pair count", lambda s: s["ab"]["base-arachne-off"]["pairs"].pop("3"))
    mutate(
        "corroborated win claimed",
        lambda s: s["ab"]["benchy-classic-off"].update(
            {"wall_signs": 3, "cpu_signs": 3}
        ),
    )
    mutate(
        "scoreboard orca brim type",
        lambda s: s["scoreboard"]["cells"]["benchy-classic-off"]["orca"][
            "resolved"
        ].update({"brim_type": ["auto_brim"]}),
    )
    mutate(
        "scoreboard pnp hash split",
        lambda s: s["scoreboard"]["cells"]["base-classic-on"]["pnp-accelerated"].update(
            {"sha256": ["0" * 64]}
        ),
    )
    mutate("scoreboard dirty", lambda s: s["scoreboard"].update({"clean": False}))

    holes = 0
    for name, broken in mutations:
        if not check(broken):
            print(f"NEGATIVE CONTROL FAILED: mutation '{name}' was accepted")
            holes += 1
        else:
            print(f"negative control ok: {name}")
    print(f"{len(mutations)} mutations, {holes} holes")
    return 0 if holes == 0 else 1


def main() -> int:
    if not SUMMARY.exists():
        print(f"missing {SUMMARY}", file=sys.stderr)
        return 2
    summary = json.loads(SUMMARY.read_text(encoding="utf-8"))
    if "--self-test" in sys.argv:
        return self_test(summary)
    fails = check(summary)
    for fail in fails:
        print(f"FAIL: {fail}")
    if fails:
        print(f"{len(fails)} failure(s)")
        return 1
    print(
        f"PASS: {len(CELLS)} cells, "
        f"{sum(len(c['pairs']) for c in summary['ab'].values())} A/B pairs, "
        f"{len(summary['oracle'])} oracle cells "
        f"(strict pass: {', '.join(STRICT_PASS)}), "
        f"{len(summary['scoreboard']['cells'])} scoreboard cells"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
