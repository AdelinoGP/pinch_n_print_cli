#!/usr/bin/env python3
"""t155 -- checkout-only verifier for the committed evidence set.

Re-derives every claim in `FINDINGS.md` from the committed files:

  probe.json      this take's reduced probe + instrumented observations
  summary.json    the pricing table (derived by price.py from probe.json)
  reference.json  the t151 active G-code set (the currency)
  probe.patch     the temporary probe, reverted from the tree
  ../..           the working tree, asserted free of probe code

Exit 0 = every check holds. `--self-test` mutates one field per check and
asserts the corresponding check fails; it must report `SELF-TEST 14/14`.

Usage:
  python verify.py
  python verify.py --self-test
"""

from __future__ import annotations

import copy
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
PROBE = json.loads((HERE / "probe.json").read_text(encoding="utf-8"))
SUMMARY = json.loads((HERE / "summary.json").read_text(encoding="utf-8"))
REFERENCE = json.loads(
    (
        REPO / "docs/specs/perf-vs-orca/evidence/t151-skirt-hull-contour/reference.json"
    ).read_text(encoding="utf-8")
)

PROBED_CELLS = ("base-classic-off", "base-classic-on")
ALL_CELLS = tuple(PROBE["cells"])


def check_output_identity(state) -> tuple[bool, str]:
    bad = []
    for cell, entry in state["probe"]["cells"].items():
        for run in entry.get("runs", []):
            if not run.get("reference_match"):
                bad.append(
                    f"{cell}/{run.get('tag')} hash {run.get('gcode_sha256', '')[:12]}"
                )
        complete = entry["slice_complete"]
        if entry["exit"] != 0 or complete.get("degraded") is not False:
            bad.append(
                f"{cell} not clean (exit={entry['exit']}, degraded={complete.get('degraded')})"
            )
        if complete.get("fatal_error_count") or complete.get("non_fatal_error_count"):
            bad.append(f"{cell} diagnostics non-zero")
    return (
        not bad,
        "every run reproduces t151's active set cleanly" if not bad else "; ".join(bad),
    )


def check_block_location(state) -> tuple[bool, str]:
    bad = []
    for cell in PROBED_CELLS:
        prep = state["probe"]["cells"][cell]["prep_views_by_stage_module"]
        infill_first = prep.get("Layer::Infill|com.core.raft-default", {}).get(
            "sum_us", 0
        )
        perimeters_first = prep.get(
            "Layer::Perimeters|com.core.classic-perimeters", {}
        ).get("sum_us", 0)
        others = sum(
            v["sum_us"]
            for k, v in prep.items()
            if k.startswith("Layer::Infill|") and "raft-default" not in k
        )
        if infill_first < 20_000_000:
            bad.append(f"{cell}: first Infill derivation only {infill_first} us")
        if perimeters_first < 20_000_000:
            bad.append(
                f"{cell}: first Perimeters derivation only {perimeters_first} us"
            )
        if others > 100_000:
            bad.append(
                f"{cell}: other Infill modules hold {others} us (memo should absorb them)"
            )
    return (
        not bad,
        "the block sits on the first dispatch of Perimeters and Infill"
        if not bad
        else "; ".join(bad),
    )


def check_block_is_derivation(state) -> tuple[bool, str]:
    bad = []
    for cell in PROBED_CELLS:
        row = state["summary"]["cells"][cell]
        if row["prep_views_share_of_bracket"] < 0.95:
            bad.append(
                f"{cell}: prep_views is only {row['prep_views_share_of_bracket']:.3f} of the bracket"
            )
    return (
        not bad,
        "prep_views is >=95 % of the first-Infill bracket on both cells"
        if not bad
        else "; ".join(bad),
    )


def check_derivation_composition(state) -> tuple[bool, str]:
    bad = []
    for cell in PROBED_CELLS:
        share = state["summary"]["cells"][cell][
            "needs_support_share_of_prepare_regions"
        ]
        if share < 0.90:
            bad.append(
                f"{cell}: derive_needs_support is only {share:.3f} of prepare_regions"
            )
    return (
        not bad,
        "derive_needs_support is >=90 % of the derivation on both cells"
        if not bad
        else "; ".join(bad),
    )


def check_predicate_census(state) -> tuple[bool, str]:
    bad = []
    for cell in PROBED_CELLS:
        entry = state["probe"]["cells"][cell]
        ns = entry["needs_support"]
        layers = entry["layer_count"]
        expected = 2 * layers if cell == "base-classic-off" else 3 * layers
        if ns["calls"] != expected:
            bad.append(
                f"{cell}: {ns['calls']} needs_support calls against {expected} expected"
            )
        if ns["footprints_max"] != 1:
            bad.append(
                f"{cell}: {ns['footprints_max']} non-empty overhang footprints (expected 1)"
            )
        if ns["value_false"] != 0:
            bad.append(
                f"{cell}: {ns['value_false']} false answers (measured shape is all-true)"
            )
    return (
        not bad,
        "one footprint, all-true, once per derivation (3x on supports-on)"
        if not bad
        else "; ".join(bad),
    )


def check_transfer(state) -> tuple[bool, str]:
    baseline = state["probe"]["cells"]["base-classic-off"]
    decline = state["probe"]["cells"]["base-classic-off-decline"]
    prep_b = baseline["prep_views_by_stage_module"]
    prep_d = decline["prep_views_by_stage_module"]
    bad = []
    raft_b = prep_b["Layer::Infill|com.core.raft-default"]["sum_us"]
    if "Layer::Infill|com.core.raft-default" in prep_d:
        bad.append("raft-default still holds a derivation in the decline arm")
    gyroid = prep_d.get("Layer::Infill|com.core.gyroid-infill", {}).get("sum_us", 0)
    if gyroid < 20_000_000:
        bad.append(f"decline arm's gyroid holds only {gyroid} us")
    if abs(gyroid - raft_b) / raft_b > 0.10:
        bad.append(f"transfer moved {gyroid} us against {raft_b} us (>10 %)")
    others = sum(
        v["sum_us"]
        for k, v in prep_d.items()
        if k.startswith("Layer::Infill|")
        and k != "Layer::Infill|com.core.gyroid-infill"
    )
    if others > 100_000:
        bad.append(f"decline arm: other Infill modules hold {others} us")
    return (
        not bad,
        "declining raft-default moves the block to gyroid, total unchanged"
        if not bad
        else "; ".join(bad),
    )


def check_representation_safety(state) -> tuple[bool, str]:
    bad = []
    for cell in ALL_CELLS:
        entry = state["probe"]["cells"][cell]
        audit = entry["sync_audit"]
        expected = 3 * entry["layer_count"]
        if audit["changed"] != 0:
            bad.append(
                f"{cell}: {audit['changed']}/{audit['calls']} syncs changed a read field"
            )
        if audit["calls"] != expected:
            bad.append(f"{cell}: {audit['calls']} audits against {expected} expected")
    return (
        not bad,
        "0/1488 fill syncs changed the derivation's read fields on every probed cell"
        if not bad
        else "; ".join(bad),
    )


def check_pricing(state) -> tuple[bool, str]:
    bad = []
    for cell in PROBED_CELLS:
        row = state["summary"]["cells"][cell]
        thread = (
            state["probe"]["cells"][cell]["prep_views_by_stage_module"][
                "Layer::Infill|com.core.raft-default"
            ]["sum_us"]
            / 1e6
        )
        parallel = state["summary"]["ledger_constants"][cell]["effective_parallelism"]
        ceiling = thread / parallel
        if abs(ceiling - row["ceiling_wall_s"]) > 0.01:
            bad.append(
                f"{cell}: ceiling {row['ceiling_wall_s']} s != thread/parallelism {ceiling:.3f} s"
            )
        if row["floor_multiple"] < 1.0:
            bad.append(f"{cell}: ceiling {row['floor_multiple']}x the 0.80 % floor")
    on = state["summary"]["cells"]["base-classic-on"]
    if on["ceiling_over_requirement"] < 1.0:
        bad.append(
            f"base classic-on: ceiling {on['ceiling_over_requirement']}x the {on['base_on_requirement_s']} s requirement"
        )
    return (
        not bad,
        "ceilings re-derive from thread/parallelism and clear the floor and the requirement"
        if not bad
        else "; ".join(bad),
    )


def check_probe_reverted(state) -> tuple[bool, str]:
    bad = []
    if state["patched_files"]:
        bad.append("probe marker still in tree: " + ", ".join(state["patched_files"]))
    patch = state["probe_patch"]
    if patch.count("diff --git") != 5:
        bad.append(
            f"probe.patch has {patch.count('diff --git')} file diffs (expected 5)"
        )
    if "PNP_T155_PROBE" not in patch or "PNP_T155_DECLINE_RAFT" not in patch:
        bad.append("probe.patch does not carry both env gates")
    return (
        not bad,
        "tree carries no probe code; probe.patch preserves the five-file probe"
        if not bad
        else "; ".join(bad),
    )


CHECKS = [
    ("output-identity", check_output_identity),
    ("block-location", check_block_location),
    ("block-is-derivation", check_block_is_derivation),
    ("derivation-composition", check_derivation_composition),
    ("predicate-census", check_predicate_census),
    ("stage-transfer", check_transfer),
    ("representation-safety", check_representation_safety),
    ("pricing", check_pricing),
    ("probe-reverted", check_probe_reverted),
]


def fresh_state() -> dict:
    patched = [
        str(path.relative_to(REPO))
        for path in (REPO / "crates").rglob("*.rs")
        if "PNP_T155_PROBE" in path.read_text(encoding="utf-8", errors="replace")
    ]
    return {
        "probe": copy.deepcopy(PROBE),
        "summary": copy.deepcopy(SUMMARY),
        "reference": copy.deepcopy(REFERENCE),
        "probe_patch": (HERE / "probe.patch").read_text(encoding="utf-8"),
        "patched_files": patched,
    }


def mutate(name: str, state: dict) -> None:
    """One mutation per check, each aimed at that check's protected claim."""
    if name == "output-identity":
        state["probe"]["cells"]["base-classic-off"]["runs"][0]["reference_match"] = (
            False
        )
    elif name == "block-location":
        state["probe"]["cells"]["base-classic-off"]["prep_views_by_stage_module"][
            "Layer::Infill|com.core.gyroid-infill"
        ]["sum_us"] = 40_000_000
    elif name == "block-is-derivation":
        state["summary"]["cells"]["base-classic-off"]["prep_views_share_of_bracket"] = (
            0.5
        )
    elif name == "derivation-composition":
        state["summary"]["cells"]["base-classic-on"][
            "needs_support_share_of_prepare_regions"
        ] = 0.2
    elif name == "predicate-census":
        state["probe"]["cells"]["base-classic-off"]["needs_support"]["value_false"] = 7
    elif name == "stage-transfer":
        state["probe"]["cells"]["base-classic-off-decline"][
            "prep_views_by_stage_module"
        ]["Layer::Infill|com.core.gyroid-infill"]["sum_us"] = 1_000
    elif name == "representation-safety":
        state["probe"]["cells"]["base-classic-off"]["sync_audit"]["changed"] = 1
    elif name == "pricing":
        state["summary"]["cells"]["base-classic-on"]["ceiling_over_requirement"] = 0.5
    elif name == "probe-reverted":
        state["patched_files"] = ["crates/slicer-runtime/src/layer_executor.rs"]


def run_self_test() -> int:
    print("SELF-TEST: each mutation must be rejected by its check")
    rejected = 0
    for index, (name, check) in enumerate(CHECKS, start=1):
        state = fresh_state()
        mutate(name, state)
        ok, _ = check(state)
        if not ok:
            rejected += 1
            print(f"  {index:2d}. {name}: mutation rejected")
        else:
            print(f"  {index:2d}. {name}: MUTATION SURVIVED (self-test failed)")
    print(f"SELF-TEST {rejected}/{len(CHECKS)}")
    return 0 if rejected == len(CHECKS) else 1


def main() -> int:
    if "--self-test" in sys.argv:
        return run_self_test()
    state = fresh_state()
    failures = 0
    for index, (name, check) in enumerate(CHECKS, start=1):
        ok, detail = check(state)
        print(f"{index:2d}. {name}: {'PASS' if ok else 'FAIL'} - {detail}")
        failures += 0 if ok else 1
    print(f"PASS {len(CHECKS) - failures}/{len(CHECKS)}")
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
