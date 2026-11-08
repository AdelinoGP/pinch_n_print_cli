#!/usr/bin/env python3
"""t60 checkout-only verifier (wayfinder perf-vs-orca ticket 60).

Runs against the in-repo evidence set only (``captures/`` + ``summary.json``);
the raw tree under ``target/t60-host-native/`` is optional. Checks:

- every capture's output sha matches its preserved t58 qualification reference
  byte-for-byte, re-read from the in-repo qualification file (not the capture's
  own copy), status clean, zero fatal/non-fatal errors;
- per-owner call conservation: every wall owner's ``calls`` equals the sum of
  its per-scope calls;
- the probe-bracket entry set and counts are exactly the expected shape for
  each cell (one per layer for the per-layer brackets, three per layer for the
  region-partition commit hook, 13 per layer for the prepare/dispatch guards,
  one each for the prepass/postpass guest dispatches, plus the supports-on-only
  plan-aggregation bracket);
- the attribution claim: for every classic cell and every bucket scope
  (``polygon_ops::offset`` / ``polygon_ops::clip_polygons``), the pre-take
  ``host:native`` counts re-read from the in-repo refreshes equal the counts
  attributed to named probe owners plus the residual ``host:native`` row, and
  that residual is **zero** (the bucket is fully attributed);
- summary.json re-derives from the captures (owner table + reconciliation).

Negative controls: output-sha, owner-count, dropped-owner, probe-entry,
status-flag and fabricated-residual mutations must all be rejected.
"""

import copy
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
BASELINE_BENCHY = (
    REPO / "docs/specs/perf-vs-orca/evidence/perf-refresh-2026-10-13/captures"
)
BASELINE_BASE = (
    REPO
    / "docs/specs/perf-vs-orca/evidence/perf-refresh-2026-10-09/reduced/classic-cells.json"
)
QUALIFICATION = (
    REPO / "docs/specs/perf-vs-orca/evidence/t58-finalization-copies/qualification.json"
)
LAYERS = {"benchy": 240, "base": 495}
DISPATCH_MODULES = [
    "classic-perimeters",
    "arachne-perimeters",
    "infill-linker",
    "gyroid-infill",
    "lightning-infill",
    "rectilinear-infill",
    "tree-support",
    "support-surface-ironing",
    "top-surface-ironing",
    "seam-placer",
    "path-optimization-default",
    "fuzzy-skin",
    "wave-overhangs",
    "raft-default",
]


def load(path):
    return json.loads(Path(path).read_text(encoding="utf-8-sig"))


def references():
    data = load(QUALIFICATION)
    return {cell["cell"]: cell["sha256"] for cell in data["cells"]}


def baseline_host_native(capture):
    model, generator, supports = (
        capture["metadata"]["model"],
        capture["metadata"]["generator"],
        capture["metadata"]["supports"],
    )
    if model == "benchy":
        path = BASELINE_BENCHY / f"ordinary-benchy-{generator}-{supports}-profile.json"
        row = next(
            m
            for m in load(path)["profile"]["modules"]
            if m["module_id"] == "host:native"
        )
    else:
        data = load(BASELINE_BASE)
        row = next(
            r
            for r in data[f"base-{generator}-{supports}"]["profile_host"]
            if r["module_id"] == "host:native"
        )
    return {scope["scope"]: scope["calls"] for scope in row["scopes"]}


def expected_probe_entries(capture):
    model, generator, supports = (
        capture["metadata"]["model"],
        capture["metadata"]["generator"],
        capture["metadata"]["supports"],
    )
    layers = LAYERS[model]
    assert capture["layers"] == layers, capture["layers"]
    expected = {
        "probe:layer_span": layers,
        "probe:region_partition": 3 * layers,
        "probe:derive_ordinary": layers,
        "probe:derive_perimeter_source": layers,
        "probe:prepare_regions": 13 * layers,
        "probe:p:layer-planner-default": 1,
        "probe:p:seam-planner-default": 1,
        "probe:p:traditional-support-planner": 1,
        "probe:p:tree-support-planner": 1,
        "probe:plan_aggregation": 1,
        "probe:postpass_dispatch": 4,
    }
    other = {"classic": "arachne-perimeters", "arachne": "classic-perimeters"}[
        generator
    ]
    for module in DISPATCH_MODULES:
        if module == other:
            continue
        expected[f"probe:d:{module}"] = layers
    return expected


def validate(capture, refs, summary):
    meta = capture["metadata"]
    run_id = meta["run_id"]
    model, generator, supports = meta["model"], meta["generator"], meta["supports"]

    # Output identity, re-read from the committed qualification.
    ref = refs[f"ordinary-{model}-{generator}-{supports}"]
    assert meta["reference_sha256"] == ref, (run_id, "reference drift")
    assert meta["sha256"] == ref, (run_id, "output not byte-identical to reference")

    # Clean run.
    assert meta["status"] == "ok", (run_id, meta["status"])
    assert (
        meta["degraded"] is False and meta["fatal"] == 0 and meta["non_fatal"] == 0
    ), run_id

    # Per-owner call conservation.
    for owner in capture["owners"]:
        assert owner["calls"] == sum(s["calls"] for s in owner["scopes"]), (
            run_id,
            owner["owner"],
        )

    # Probe bracket shape.
    totals = {probe["owner"]: probe["entries"] for probe in capture["probe_totals"]}
    expected = expected_probe_entries(capture)
    assert set(totals) == set(expected), (run_id, sorted(set(totals) ^ set(expected)))
    for name, entries in expected.items():
        assert totals[name] == entries, (run_id, name, totals[name], entries)

    # Full attribution of the pre-take bucket.
    baseline = baseline_host_native(capture)
    attributed = {}
    residual = {}
    for owner in capture["owners"]:
        if owner["owner"].startswith("probe:"):
            target = attributed
        elif owner["owner"] == "host:native":
            target = residual
        else:
            # Prepass built-ins have their own brackets; not part of the bucket.
            continue
        for scope in owner["scopes"]:
            target[scope["scope"]] = target.get(scope["scope"], 0) + scope["calls"]
    for scope, baseline_calls in baseline.items():
        assert attributed.get(scope, 0) + residual.get(scope, 0) == baseline_calls, (
            run_id,
            scope,
        )
        assert residual.get(scope, 0) == 0, (run_id, scope, residual.get(scope, 0))
        assert attributed.get(scope, 0) > 0, (run_id, scope)

    # Summary consistency for this cell.
    cell = summary["cells"][run_id]
    assert cell["output_sha256"] == meta["sha256"], run_id
    assert cell["layers"] == capture["layers"], run_id
    assert cell["phases_ms"] == capture["phases_ms"], run_id
    table = cell["owner_table"]
    for owner in capture["owners"]:
        row = table[owner["owner"]]
        assert row["calls"] == owner["calls"], (run_id, owner["owner"])
        assert row["total_wall_ms"] == round(owner["total_wall_ns"] / 1e6, 3), (
            run_id,
            owner["owner"],
        )
    for row in cell["reconciliation"]:
        assert (
            row["baseline_calls"] == row["attributed_calls"] + row["residual_calls"]
        ), (run_id, row["scope"])
    return {
        "run_id": run_id,
        "output": meta["sha256"][:12],
        "probe_owners": len(totals),
        "attributed_calls": sum(attributed.values()),
        "residual_calls": sum(residual.values()),
        "scopes": {k: v for k, v in sorted(attributed.items())},
    }


def main() -> int:
    refs = references()
    summary = load(HERE / "summary.json")
    captures = [load(path) for path in sorted((HERE / "captures").glob("*.json"))]
    assert len(captures) == 4, len(captures)
    verdicts = {}
    for capture in captures:
        verdicts[capture["metadata"]["run_id"]] = validate(capture, refs, summary)
    print(json.dumps(verdicts, indent=1, sort_keys=True))

    # Negative controls against the base supports-on capture (largest bucket).
    sample = next(
        c for c in captures if c["metadata"]["run_id"] == "base-classic-on-profile"
    )
    negative_controls = []

    def expect_reject(name, mutate):
        bad = copy.deepcopy(sample)
        mutate(bad)
        try:
            validate(bad, refs, summary)
        except AssertionError:
            negative_controls.append(name)
        else:
            raise AssertionError("negative control was not rejected: " + name)

    expect_reject("output", lambda c: c["metadata"].__setitem__("sha256", "0" * 64))
    expect_reject(
        "owner-count",
        lambda c: c["owners"][0]["scopes"][0].__setitem__(
            "calls", c["owners"][0]["scopes"][0]["calls"] + 1
        ),
    )
    expect_reject(
        "dropped-owner",
        lambda c: c["owners"].remove(
            next(o for o in c["owners"] if o["owner"] == "probe:region_partition")
        ),
    )
    expect_reject(
        "probe-entry",
        lambda c: next(
            p for p in c["probe_totals"] if p["owner"] == "probe:layer_span"
        ).__setitem__("entries", c["layers"] + 1),
    )
    expect_reject("status", lambda c: c["metadata"].__setitem__("degraded", True))
    expect_reject(
        "fabricated-residual",
        lambda c: c["owners"].append(
            {
                "owner": "host:native",
                "calls": 1,
                "total_wall_ns": 1,
                "self_wall_ns": 0,
                "scopes": [
                    {
                        "scope": "polygon_ops::offset",
                        "calls": 1,
                        "total_wall_ns": 1,
                        "self_wall_ns": 1,
                    }
                ],
            }
        ),
    )
    print("negative controls rejected:", ", ".join(negative_controls))
    print("t60 verify: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
