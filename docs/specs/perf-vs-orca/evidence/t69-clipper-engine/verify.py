"""Checkout-only evidence verification; never reads scratch or executes engines.

Run normally to require the set and independently launch failing negative
controls. --negative-control mutates in memory and must exit nonzero.
"""

import argparse
import hashlib
import json
import math
import subprocess
import sys

from analyze import (
    HERE,
    REPO,
    RESULTS,
    OPS,
    load,
    geometry,
    summarize,
    scope_mapping,
    render,
)
from campaign import cases, sha

CONTROLS = ("changed-timing", "deleted-case", "shifted-site", "changed-scope")


def validate(passes, summary, provenance):
    population = cases()
    expected = {
        (p.relative_to(HERE).as_posix(), i): block for p, i, block in population
    }
    assert provenance["case_count"] == len(expected)
    assert provenance["corpus"] == {
        p.relative_to(HERE).as_posix(): sha(p) for p, _, _ in population
    }
    assert provenance["resolved_clipper"]["version"] == "1.1.0"
    assert "source" not in provenance["resolved_clipper"]
    assert provenance["cpp_version"] == "1.5.2"
    for name, digest in provenance["driver_sources"].items():
        assert sha(HERE / name) == digest, ("driver seal", name)
    indexed = {}
    for order, rows in passes.items():
        serialized = json.dumps(rows, sort_keys=True).encode()
        assert (
            hashlib.sha256(serialized).hexdigest()
            == provenance["reduced_results_sha256"][order]
        ), ("reduced result integrity", order)
        keys = [(row["file"], row["case_index"]) for row in rows]
        assert len(keys) == len(set(keys)) and set(keys) == set(expected), (
            "case population"
        )
        indexed[order] = dict(zip(keys, rows))
        for key, row in zip(keys, rows):
            block = expected[key]
            header = block.splitlines()[0].split()
            assert [row["site"], str(row["layer"]), str(row["opcode"])] == header[
                1:4
            ], ("case identity", key)
            assert row["input"] == geometry(block), ("input geometry", key)
            for engine in ("rust", "cpp"):
                value = row[engine]
                assert math.isfinite(value["median_ns"]) and value["median_ns"] >= 0
                assert 0 <= value["zero_repeats"] <= 5
                assert len(value["raw_sha256"]) == 64
                assert len(value["out_hash"]) == 16 and all(
                    c in "0123456789abcdef" for c in value["out_hash"]
                )
                assert (
                    0 <= value["out_paths"] <= value["out_rings"] <= value["out_verts"]
                )
        assert summarize(rows) == summary["passes"][order], ("summary reduction", order)
    for key in expected:
        for engine in ("rust", "cpp"):
            a, b = indexed["forward"][key][engine], indexed["reverse"][key][engine]
            assert all(
                a[k] == b[k]
                for k in ("out_hash", "out_paths", "out_rings", "out_verts")
            ), ("pass output drift", key, engine)
    for key, spread in summary["spread_absolute_ratio_points"].items():
        a = summary["passes"]["forward"][key]["ratio"]
        b = summary["passes"]["reverse"][key]["ratio"]
        assert spread == abs(a - b)
    assert scope_mapping(summary["passes"]) == summary["scope_mapping"], (
        "scope re-derivation"
    )
    assert (HERE / "FINDINGS.md").read_text(encoding="utf-8") == render(summary), (
        "FINDINGS table/render drift"
    )


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--negative-control", choices=CONTROLS)
    options = parser.parse_args()
    passes = {o: load(RESULTS / (o + ".json")) for o in ("forward", "reverse")}
    summary = load(RESULTS / "summary.json")
    provenance = load(RESULTS / "provenance.json")
    if options.negative_control == "changed-timing":
        passes["forward"][0]["rust"]["median_ns"] *= 2
    elif options.negative_control == "deleted-case":
        passes["forward"].pop()
    elif options.negative_control == "shifted-site":
        passes["forward"][0]["site"] = "A0"
    elif options.negative_control == "changed-scope":
        summary["scope_mapping"]["base"]["totals"]["offset"]["accumulated_seconds"] += 1
    try:
        validate(passes, summary, provenance)
    except (AssertionError, KeyError, ValueError) as error:
        print("FAIL:", str(error))
        return 1
    if options.negative_control:
        print("ERROR: negative control survived")
        return 0  # Parent explicitly requires nonzero rejection.
    rejected = []
    for control in CONTROLS:
        result = subprocess.run(
            [sys.executable, str(HERE / "verify.py"), "--negative-control", control],
            capture_output=True,
            text=True,
            timeout=30,
        )
        assert result.returncode == 1 and "FAIL:" in result.stdout, (
            control,
            result.returncode,
            result.stdout,
            result.stderr,
        )
        rejected.append(control)
    headlines = {
        k: [summary["passes"][o][k]["ratio"] for o in ("forward", "reverse")]
        for k in summary["passes"]["forward"]
        if k.startswith(("opcode/", "site/"))
    }
    print(json.dumps(headlines, indent=2))
    print("Negative controls rejected with exit 1:", ", ".join(rejected))
    print("t69 checkout-only verify: PASS (exit 0)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
