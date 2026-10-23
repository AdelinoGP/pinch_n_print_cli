#!/usr/bin/env python3
"""Checkout-only verifier for the t79 emit-batch constant sweep.

Four committed artifacts, each with a negative-control suite:

- `sweep-summary.json` — the instrumented screening table that chose the pair.
- `confirm.json` — the frozen-snapshot pre-gate confirmation pairs.
- `snapshots.json` — the two snapshot trees' provenance.
- `summary.json` — the take's derived verdict (DROP, no constant adopted).

The raw G-code and stderr captures are large and live under `target/` and
`.local-artifacts/`; the committed files are the reduced set produced by
`reduce-sweep.py`, `reduce-confirm.py`, `compare-snapshots.py` and
`build-summary.py`.

Exits 0 only if every check passes and every negative control is rejected.
"""

from __future__ import annotations

import copy
import json
import statistics
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
SWEEP = HERE / "sweep-summary.json"
CONFIRM = HERE / "confirm.json"
SNAPSHOTS = HERE / "snapshots.json"
SUMMARY = HERE / "summary.json"
PATCH = HERE / "candidate.patch"

REFERENCE_SHA = "f60e5e9b8c0d836cf73170e645c18f9193b91d47e694d335cfb1d78032682319"
REFERENCE_BYTES = 43_912_792
CHOSEN = {"e": 4, "c": 4096}
CONTROL = {"e": 4, "c": 256}
CHUNK_VARIANTS = {1, 2, 4, 8, 16, 32}
CAP_VARIANTS = {64, 256, 1024, 4096}
CPU_CEILING_PCT = 3.0
PLANNER_MODULE = "modules/tree-support-planner/tree-support-planner.wasm"


class CheckFailed(Exception):
    pass


def check(condition: bool, message: str) -> None:
    if not condition:
        raise CheckFailed(message)


def load(name: str) -> dict:
    return json.loads((HERE / name).read_text(encoding="utf-8"))


def verify_sweep(sweep: dict) -> None:
    check(
        sweep.get("instrumented_only") is True, "sweep is not marked instrumented-only"
    )
    variants = sweep["variants"]
    seen = {(v["e"], v["c"]) for v in variants}
    expected = {(e, CONTROL["c"]) for e in CHUNK_VARIANTS} | {
        (CONTROL["e"], c) for c in CAP_VARIANTS
    }
    check(
        seen == expected,
        f"sweep variants {sorted(seen)} != expected {sorted(expected)}",
    )
    for variant in variants:
        where = f"e={variant['e']} c={variant['c']}"
        check(variant["clean"], f"{where}: screening run unclean")
        check(
            variant["sha256"] == [REFERENCE_SHA],
            f"{where}: output {variant['sha256']} != preserved t58 reference",
        )
        check(variant["runs"] >= 1, f"{where}: no runs recorded")
        check(
            len(variant["planner_ms"]) == variant["runs"],
            f"{where}: planner samples do not match run count",
        )
    caps = {
        v["c"]: v["planner_median_ms"]
        for v in variants
        if v["e"] == CONTROL["e"] and v["c"] in CAP_VARIANTS
    }
    check(
        min(caps, key=caps.get) == CHOSEN["c"],
        f"screening cap argmin {min(caps, key=caps.get)} != chosen {CHOSEN['c']}",
    )
    chunks = {
        v["e"]: v["planner_median_ms"]
        for v in variants
        if v["c"] == CONTROL["c"] and v["e"] in CHUNK_VARIANTS
    }
    check(
        min(chunks, key=chunks.get) == CONTROL["e"],
        f"chunk knee {min(chunks, key=chunks.get)} != committed {CONTROL['e']}",
    )


def _paired_median(pairs: list[dict], base: str, cand: str) -> float:
    return (
        statistics.median(float(p[cand]) / float(p[base]) for p in pairs) - 1.0
    ) * 100.0


def verify_confirm(confirm: dict) -> None:
    check(confirm["cell"] == "ordinary-base-classic-on", "confirmation cell drifted")
    pairs = confirm["pairs"]
    check(len(pairs) == 2, f"confirmation holds {len(pairs)} pairs, expected 2")
    check(
        all(p["sha_equal"] for p in pairs), "a confirmation pair is not byte-identical"
    )
    wall = _paired_median(pairs, "baseline_wall_s", "candidate_wall_s")
    cpu = _paired_median(pairs, "baseline_cpu_s", "candidate_cpu_s")
    check(
        abs(wall - confirm["wall_delta_pct_paired_median"]) <= 0.01,
        f"recorded confirmation wall {confirm['wall_delta_pct_paired_median']:+.3f}% "
        f"!= re-derived {wall:+.3f}%",
    )
    check(
        abs(cpu - confirm["cpu_delta_pct_paired_median"]) <= 0.01,
        f"recorded confirmation CPU {confirm['cpu_delta_pct_paired_median']:+.3f}% "
        f"!= re-derived {cpu:+.3f}%",
    )
    check(wall > 0.0, f"confirmation wall delta {wall:+.2f}% is not a regression")
    check(cpu <= CPU_CEILING_PCT, f"confirmation CPU {cpu:+.2f}% over the ceiling")
    check(confirm["direction"] == "candidate not faster", "direction label drifted")


def verify_snapshots(snapshots: dict) -> None:
    check(snapshots["pass"] is True, "snapshot provenance did not pass")
    for mode in ("ordinary", "accelerated"):
        record = snapshots["modes"][mode]
        check(record["executable_equal"], f"{mode}: pnp_cli.exe differs between arms")
        check(
            record["module_differing"] == [PLANNER_MODULE],
            f"{mode}: module diff {record['module_differing']} != [{PLANNER_MODULE}]",
        )
        check(
            record["baseline_planner_sha256"] != record["candidate_planner_sha256"],
            f"{mode}: planner wasm is identical between arms",
        )


def verify_summary(summary: dict) -> None:
    check(summary["chosen"] == CHOSEN, "chosen pair drifted")
    check(summary["control"] == CONTROL, "control pair drifted")
    check(summary["reference_sha256"] == REFERENCE_SHA, "reference sha drifted")
    check(summary["reference_bytes"] == REFERENCE_BYTES, "reference bytes drifted")

    screening = summary["screening"]
    check(screening["instrumented_only"] is True, "screening is not instrumented-only")
    check(
        set(screening["chunk_bracket_ms"]) == {str(e) for e in CHUNK_VARIANTS},
        "chunk table does not cover the pre-declared variants",
    )
    check(
        set(screening["cap_bracket_ms"]) == {str(c) for c in CAP_VARIANTS},
        "cap table does not cover the pre-declared variants",
    )
    check(
        min(screening["cap_bracket_ms"], key=screening["cap_bracket_ms"].get)
        == str(CHOSEN["c"]),
        "chosen cap is not the screening minimum",
    )
    delta = 100.0 * (
        screening["chosen_bracket_ms"] / screening["control_bracket_ms"] - 1.0
    )
    check(
        abs(delta - screening["bracket_delta_pct"]) <= 0.01,
        f"recorded bracket delta {screening['bracket_delta_pct']:+.3f}% "
        f"!= re-derived {delta:+.3f}%",
    )
    check(delta < 0.0, "chosen pair did not lead the screening bracket")

    confirmation = summary["confirmation"]
    check(
        confirmation["wall_delta_pct_paired_median"] > 0.0,
        "summary hides the pre-gate wall regression",
    )
    check(
        confirmation["direction"] == "candidate not faster",
        "summary direction does not match the regression",
    )
    rederived = _paired_median(
        confirmation["pairs"], "baseline_wall_s", "candidate_wall_s"
    )
    check(
        abs(rederived - confirmation["wall_delta_pct_paired_median"]) <= 0.01,
        "summary confirmation wall median does not re-derive",
    )

    check(summary["snapshots"]["pass"] is True, "summary snapshots did not pass")
    check(
        summary["gate"]["status"] == "not_run_by_rule",
        f"gate status {summary['gate']['status']} is not not_run_by_rule",
    )
    check(
        "confirmation" in summary["gate"]["reason"],
        "gate reason omits the stop evidence",
    )
    check(summary["verdict"].startswith("DROP"), "verdict is not a DROP")


def verify_patch() -> None:
    text = PATCH.read_text(encoding="utf-8")
    files = [line for line in text.splitlines() if line.startswith("diff --git ")]
    check(len(files) == 1, f"candidate.patch touches {len(files)} files, expected 1")
    check(
        "modules/core-modules/tree-support-planner/src/lib.rs" in files[0],
        "candidate.patch does not target the tree-support planner guest",
    )
    check(
        "-const CLIP_BATCH_MAX_REQUESTS: usize = 256;" in text
        and "+const CLIP_BATCH_MAX_REQUESTS: usize = 4096;" in text,
        "candidate.patch does not change the request cap 256 -> 4096",
    )
    check(
        "EMIT_BATCH_LAYERS: usize = 4;"
        not in "\n".join(
            line for line in text.splitlines() if line.startswith(("+", "-"))
        ),
        "candidate.patch changes EMIT_BATCH_LAYERS, which the sweep kept at 4",
    )
    check(
        "const EMIT_BATCH_LAYERS: usize = 4;"
        not in "\n".join(
            line for line in text.splitlines() if line.startswith(("+", "-"))
        ),
        "candidate.patch changes EMIT_BATCH_LAYERS, which the sweep kept at 4",
    )


def main() -> int:
    sweep = load("sweep-summary.json")
    confirm = load("confirm.json")
    snapshots = load("snapshots.json")
    summary = load("summary.json")
    try:
        verify_sweep(sweep)
        verify_confirm(confirm)
        verify_snapshots(snapshots)
        verify_summary(summary)
        verify_patch()
    except CheckFailed as error:
        print(f"FAIL: {error}")
        return 1

    mutators: dict[str, tuple[object, callable]] = {
        "sweep: an unclean variant": (
            sweep,
            lambda s: s["variants"][0].__setitem__("clean", False),
        ),
        "sweep: output drift": (
            sweep,
            lambda s: s["variants"][0].__setitem__("sha256", ["0" * 64]),
        ),
        "sweep: missing chunk variant": (
            sweep,
            lambda s: s["variants"].pop(0),
        ),
        "sweep: cap argmin moved": (
            sweep,
            lambda s: next(
                v
                for v in s["variants"]
                if v["e"] == CONTROL["e"] and v["c"] == CHOSEN["c"]
            ).__setitem__("planner_median_ms", 999_999),
        ),
        "confirm: a pair is not byte-identical": (
            confirm,
            lambda c: c["pairs"][0].__setitem__("sha_equal", False),
        ),
        "confirm: a pair is faster": (
            confirm,
            lambda c: c["pairs"][0].__setitem__(
                "candidate_wall_s", c["pairs"][0]["baseline_wall_s"] - 1.0
            ),
        ),
        "snapshots: an extra module differs": (
            snapshots,
            lambda s: s["modes"]["ordinary"].__setitem__(
                "module_differing",
                [PLANNER_MODULE, "modules/other/other.wasm"],
            ),
        ),
        "summary: chosen cap not the minimum": (
            summary,
            lambda s: s["screening"]["cap_bracket_ms"].__setitem__("4096", 999999),
        ),
        "summary: confirmation sign flipped": (
            summary,
            lambda s: s["confirmation"].__setitem__(
                "wall_delta_pct_paired_median", -1.0
            ),
        ),
        "summary: gate claimed as run": (
            summary,
            lambda s: s["gate"].__setitem__("status", "pass"),
        ),
        "summary: verdict not a DROP": (
            summary,
            lambda s: s.__setitem__("verdict", "KEEP"),
        ),
    }
    verifiers = {
        "sweep": verify_sweep,
        "confirm": verify_confirm,
        "snapshots": verify_snapshots,
        "summary": verify_summary,
    }
    rejected = 0
    for name, (document, mutate) in mutators.items():
        mutated = copy.deepcopy(document)
        mutate(mutated)
        verifier = verifiers[name.split(":")[0]]
        try:
            verifier(mutated)
        except CheckFailed:
            rejected += 1
        else:
            print(f"FAIL: negative control not rejected: {name}")
            return 1

    # Patch negative control: a text mutation, not a structure mutation.
    original_patch = PATCH.read_text(encoding="utf-8")
    for wrong in (
        original_patch.replace("usize = 4096;", "usize = 512;"),
        original_patch.replace("CLIP_BATCH_MAX_REQUESTS", "EMIT_BATCH_LAYERS"),
    ):
        PATCH.write_text(wrong, encoding="utf-8")
        try:
            verify_patch()
        except CheckFailed:
            rejected += 1
        else:
            print("FAIL: negative control not rejected: patch mutation")
            return 1
        finally:
            PATCH.write_text(original_patch, encoding="utf-8")

    print(
        f"OK: all checks pass; {rejected}/{len(mutators) + 2} negative controls rejected"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
