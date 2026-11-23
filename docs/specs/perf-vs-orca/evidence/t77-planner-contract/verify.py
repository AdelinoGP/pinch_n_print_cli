#!/usr/bin/env python3
"""Checkout-only verifier for the T77 planner clip-core attribution.

Verifies the committed reduced summary's arithmetic and recorded invariants,
then proves each check is falsifiable with a negative control. Exits 0 only if
every check passes and every negative control is rejected.

The raw capture (94,310 host probe lines) is gitignored and lives under
`target/t77-probe2/`; it is reproducible from `probe.patch` plus `reduce.py`.
"""

from __future__ import annotations

import copy
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
SUMMARY = HERE / "summary.json"

T58_REFERENCE_SHA = "f60e5e9b8c0d836cf73170e645c18f9193b91d47e694d335cfb1d78032682319"
BUILD_ROLES_SITES = ("role_union", "carve_region", "set_diff", "cross_role_diff")
PLANNER_MODULE = "com.core.tree-support-planner"


class CheckFailed(Exception):
    pass


def check(condition: bool, message: str) -> None:
    if not condition:
        raise CheckFailed(message)


def verify(summary: dict) -> None:
    totals = summary["totals"]
    by_site = summary["by_site"]
    planner_core = totals["planner_clip_core_seconds"]

    # 1. Recorded output is the standing t58 reference for this cell.
    check(
        summary["output_sha256"] == T58_REFERENCE_SHA,
        f"output sha {summary['output_sha256']} != t58 reference",
    )

    # 2. The planner total equals the sum of its tagged sites.
    site_sum = sum(
        entry["core_seconds"] for name, entry in by_site.items() if name != "(untagged)"
    )
    check(
        abs(site_sum - planner_core) <= 0.01,
        f"planner site sum {site_sum:.3f}s != recorded total {planner_core:.3f}s",
    )

    # 3. The build_roles bucket is the sum of its four call sites.
    bucket = totals["build_roles_clip_core_seconds"]
    bucket_sum = sum(by_site[name]["core_seconds"] for name in BUILD_ROLES_SITES)
    check(
        abs(bucket_sum - bucket) <= 0.01,
        f"build_roles site sum {bucket_sum:.3f}s != recorded {bucket:.3f}s",
    )

    # 4. Every planner site's share is its own core over the planner total.
    for name, entry in by_site.items():
        if name == "(untagged)":
            continue
        expected = entry["core_seconds"] / planner_core
        check(
            abs(entry["share_of_planner_clip_core"] - expected) <= 0.001,
            f"{name} share {entry['share_of_planner_clip_core']} != {expected:.4f}",
        )

    # 5. The finding itself: build_roles owns essentially all planner clip core.
    check(
        totals["build_roles_share_of_planner_clip_core"] >= 0.99,
        "build_roles share below the recorded 99 % claim",
    )

    # 6. The refutation: the below-chain is not an owner.
    check(
        by_site["below"]["share_of_planner_clip_core"] < 0.01,
        "below-chain no longer negligible: the t77 refutation is stale",
    )

    # 7. Tail buckets are monotone in both calls and core time.
    tail = summary["tail"]
    for tighter, looser in (("ge_10ms", "ge_1ms"), ("ge_100ms", "ge_10ms")):
        check(
            tail[tighter]["calls"] <= tail[looser]["calls"],
            f"tail {tighter} has more calls than {looser}",
        )
        check(
            tail[tighter]["core_seconds"] <= tail[looser]["core_seconds"],
            f"tail {tighter} carries more core time than {looser}",
        )

    # 8. The module wall in the summary is the planner's, and the clip share of
    #    it is arithmetically the recorded ratio.
    wall_ms = summary["module_walls_ms"][PLANNER_MODULE]
    check(wall_ms > 0, "planner module wall missing")
    expected_share = planner_core / (wall_ms / 1000.0)
    check(
        abs(totals["planner_clip_share_of_module_wall"] - expected_share) <= 0.001,
        "planner clip share of module wall is inconsistent",
    )


NEGATIVE_CONTROLS = {
    "wrong_output_sha": lambda s: s.__setitem__("output_sha256", "1" * 64),
    "planner_total_not_sum": lambda s: s["totals"].__setitem__(
        "planner_clip_core_seconds", s["totals"]["planner_clip_core_seconds"] + 1.0
    ),
    "build_roles_total_not_sum": lambda s: s["totals"].__setitem__(
        "build_roles_clip_core_seconds", s["totals"]["build_roles_clip_core_seconds"] + 1.0
    ),
    "share_inconsistent": lambda s: s["by_site"]["role_union"].__setitem__(
        "share_of_planner_clip_core", 0.1
    ),
    "build_roles_claim_weakened": lambda s: s["totals"].__setitem__(
        "build_roles_share_of_planner_clip_core", 0.5
    ),
    "below_chain_claim_inflated": lambda s: s["by_site"]["below"].__setitem__(
        "share_of_planner_clip_core", 0.5
    ),
    "tail_not_monotone": lambda s: s["tail"].__setitem__(
        "ge_100ms", {"calls": 10**9, "core_seconds": 10**9.0}
    ),
    "module_wall_missing": lambda s: s["module_walls_ms"].__setitem__(PLANNER_MODULE, 0),
}


def main() -> int:
    summary = json.loads(SUMMARY.read_text(encoding="utf-8"))
    verify(summary)
    print("PASS: summary arithmetic and recorded invariants hold")

    rejected = 0
    for name, mutate in NEGATIVE_CONTROLS.items():
        mutated = copy.deepcopy(summary)
        mutate(mutated)
        try:
            verify(mutated)
        except CheckFailed:
            rejected += 1
            print(f"  negative control rejected: {name}")
        else:
            print(f"FAIL: negative control survived: {name}")
            return 1

    if rejected != len(NEGATIVE_CONTROLS):
        print(f"FAIL: only {rejected}/{len(NEGATIVE_CONTROLS)} negative controls rejected")
        return 1
    print(f"PASS: {rejected}/{len(NEGATIVE_CONTROLS)} negative controls rejected")
    return 0


if __name__ == "__main__":
    sys.exit(main())
