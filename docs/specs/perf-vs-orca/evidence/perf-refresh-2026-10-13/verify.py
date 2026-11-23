#!/usr/bin/env python3
"""Verify the checkout-only diagnostic evidence, including adversarial controls."""

import copy
import math

from reduce import HERE, build_summary, load


def validate(capture, inputs, references):
    meta = capture["metadata"]
    cell = "ordinary-benchy-" + meta["generator"] + "-" + meta["supports"]
    ref = references[cell]
    assert inputs["guest_freshness_exit"] == 0 and inputs["threads"] == 12
    assert inputs["provenance"] and all(
        m["provenance"] == "external" for m in inputs["provenance"]
    )
    assert meta["threads"] == 12 and meta["exit_code"] == 0
    assert meta["status"] == "ok" and meta["degraded"] is False
    assert meta["fatal"] == meta["non_fatal"] == 0
    assert meta["sha256"] == meta["reference_sha256"] == ref["sha256"]
    assert meta["bytes"] == ref["bytes"] and meta["type_counts"] == ref["type_counts"]
    assert capture["raw_stderr_sha256"] == meta["stderr_sha256"]
    assert math.isclose(
        meta["cpu_wall_ratio"],
        meta["cpu_seconds"] / meta["wall_seconds"],
        rel_tol=1e-12,
    )
    completion = capture["completion"]
    assert completion["status"] == "ok" and completion["degraded"] is False
    assert completion["fatal_error_count"] == completion["non_fatal_error_count"] == 0
    phases = [
        e["phase"] for e in capture["phase_events"] if e["event"] == "phase_complete"
    ]
    assert phases == ["validation", "prepass", "per_layer", "postpass"]
    layers = capture["layers"]
    assert [r[0] for r in layers] == list(range(240))
    assert all(
        start <= end and abs((end - start) - elapsed) <= 2
        for _, start, end, elapsed in layers
    )
    assert len(capture["module_calls"]) > len(layers)
    profile = capture["profile"]
    assert (profile is not None) == (meta["kind"] == "profile")
    if profile:
        guests = [m for m in profile["modules"] if m["unit"] == "fuel"]
        hosts = [m for m in profile["modules"] if m["unit"] == "wall_ns"]
        assert guests and hosts and len(guests) + len(hosts) == len(profile["modules"])
        assert sum(m["total_fuel"] for m in guests) == profile["fuel_total"]
        for m in guests:
            assert (
                m["self_fuel"] + sum(s["self_fuel"] for s in m["scopes"])
                == m["total_fuel"]
            )
            if m["module_id"] in capture["module_ids"]:
                index = capture["module_ids"].index(m["module_id"])
                calls = [c for c in capture["module_calls"] if c[0] == index]
                assert len(calls) == m["calls"]
                assert all(c[4] is not None for c in calls)
                assert sum(c[4] for c in calls) == m["total_fuel"]
    else:
        assert all(c[4] is None for c in capture["module_calls"])


def main():
    inputs = load(HERE / "inputs.json")
    references = {
        r["cell"]: r
        for r in load(HERE / "../t58-finalization-copies/qualification.json")["cells"]
    }
    captures = [load(path) for path in sorted((HERE / "captures").glob("*.json"))]
    expected = {
        (g, s, "profile") for g in ("classic", "arachne") for s in ("off", "on")
    } | {("arachne", s, "instrumented") for s in ("off", "on")}
    assert len(captures) == len(expected)
    assert {
        (c["metadata"]["generator"], c["metadata"]["supports"], c["metadata"]["kind"])
        for c in captures
    } == expected
    for c in captures:
        validate(c, inputs, references)
    assert build_summary() == load(HERE / "summary.json"), "derived summary drift"
    controls = []
    profiled = next(c for c in captures if c["profile"] is not None)
    for mutation in ("fuel", "output", "layer", "provenance"):
        bad, bad_inputs = copy.deepcopy(profiled), copy.deepcopy(inputs)
        if mutation == "fuel":
            bad["profile"]["fuel_total"] += 1
        elif mutation == "output":
            bad["metadata"]["sha256"] = "0" * 64
        elif mutation == "layer":
            bad["layers"].pop()
        else:
            bad_inputs["provenance"][0]["provenance"] = "integrated"
        try:
            validate(bad, bad_inputs, references)
        except AssertionError:
            controls.append(mutation)
        else:
            raise AssertionError("negative control was not rejected: " + mutation)
    print(
        "PASS: six diagnostic captures, output/reference identity, profile accounting, summary replay"
    )
    print("PASS: rejected negative controls:", ", ".join(controls))


if __name__ == "__main__":
    main()
