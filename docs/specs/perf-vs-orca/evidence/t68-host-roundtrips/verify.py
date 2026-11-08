#!/usr/bin/env python3
"""t68 checkout-only verifier (wayfinder perf-vs-orca ticket 68).

Runs against the in-repo evidence set only (``captures/`` + ``summary.json``);
the raw tree under ``target/t68-host-roundtrips/`` is optional. Checks:

- every capture's output sha matches its preserved t58 qualification reference
  byte-for-byte, re-read from the in-repo qualification file; status clean,
  zero fatal/non-fatal errors;
- the per-layer guest dump count equals the model's layer count;
- host seq buckets cover sites 0..10 exactly (seq 11 = thin-wall probe slot is
  absent), each site's calls within {layers-1, layers, layers+1} (the ±1 is
  the documented bucket shift on the single skip layer), and the total call
  count is exact (2,640 benchy / 5,442 base) ;
- guest per-site census: site 11 == 0; totals equal the same exact counts;
- the decomposition invariants: per-site guest-observed >= host-observed
  (conversion + ops), whole-cell guest remainder >= 0, host ops hold >= 95% of
  the guest-observed bucket, and the removable overhead (guest remainder +
  host conversions) stays <= 5% of the bucket;
- the wall-relevance bounds: removable-overhead serial share of the run wall
  <= 2% and 12-thread-saturated share <= 0.2% (the t68 no-candidate verdict
  margins);
- the rendered host:native bucket is large enough to contain the attributed
  classic-perimeters share, and the measured bucket total is within 25% of
  the committed t60 owner-wall reference (re-read from t60's summary.json);
- summary.json re-derives from the captures.

Negative controls: output-sha, dropped-site, guest-below-host, inflated
overhead, thin-wall-count and dump-count mutations must all be rejected.
"""

import copy
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
QUALIFICATION = (
    REPO / "docs/specs/perf-vs-orca/evidence/t58-finalization-copies/qualification.json"
)
T60_SUMMARY = REPO / "docs/specs/perf-vs-orca/evidence/t60-host-native/summary.json"
LAYERS = {"benchy": 240, "base": 495}
EXPECTED_TOTAL_CALLS = {"benchy": 2640, "base": 5442}
OFFSET_SEQS = {0, 1, 2, 3, 5, 6, 7, 10}
CLIP_SEQS = {4, 8, 9}


def load(path):
    return json.loads(Path(path).read_text(encoding="utf-8-sig"))


def references():
    data = load(QUALIFICATION)
    return {cell["cell"]: cell["sha256"] for cell in data["cells"]}


def t60_reference_ms(run_id):
    data = load(T60_SUMMARY)
    owner = data["cells"][run_id]["owner_table"]["probe:d:classic-perimeters"]
    return owner["total_wall_ms"]


def site_totals(capture):
    host = {
        s["seq"]: s["conv_in_ns"] + s["ops_ns"] + s["conv_out_ns"]
        for s in capture["host_sites"]
    }
    ops = sum(s["ops_ns"] for s in capture["host_sites"])
    conv = sum(s["conv_in_ns"] + s["conv_out_ns"] for s in capture["host_sites"])
    guest = sum(s["guest_us"] for s in capture["guest_sites"]) * 1000
    return host, ops, conv, guest


def validate(capture, refs):
    meta = capture["metadata"]
    run_id = meta["run_id"]
    model, generator, supports = meta["model"], meta["generator"], meta["supports"]
    layers = LAYERS[model]

    # Output identity, re-read from the committed qualification.
    ref = refs[f"ordinary-{model}-{generator}-{supports}"]
    assert meta["reference_sha256"] == ref, (run_id, "reference drift")
    assert meta["sha256"] == ref, (run_id, "output not byte-identical to reference")

    # Clean run.
    assert meta["status"] == "ok", (run_id, meta["status"])
    assert (
        meta["degraded"] is False and meta["fatal"] == 0 and meta["non_fatal"] == 0
    ), run_id
    assert meta["layers"] == layers, (run_id, meta["layers"])

    # Guest dump census.
    assert capture["guest_layer_dumps"] == layers, (
        run_id,
        capture["guest_layer_dumps"],
    )

    # Host seq-bucket shape.
    seqs = [s["seq"] for s in capture["host_sites"]]
    assert seqs == sorted(OFFSET_SEQS | CLIP_SEQS), (run_id, seqs)
    host_calls = 0
    for s in capture["host_sites"]:
        assert s["service"] == ("offset" if s["seq"] in OFFSET_SEQS else "clip"), (
            run_id,
            s["seq"],
            s["service"],
        )
        assert layers - 1 <= s["calls"] <= layers + 1, (run_id, s["seq"], s["calls"])
        host_calls += s["calls"]
    assert host_calls == EXPECTED_TOTAL_CALLS[model], (run_id, host_calls)

    # Guest per-site census (site 11 = thin-wall probe slot must be empty).
    guest = {s["site"]: s for s in capture["guest_sites"]}
    assert guest[11]["calls"] == 0, (run_id, "thin-wall slot must be empty")
    guest_calls = 0
    for site in range(11):
        assert guest[site]["calls"] in (layers, layers - 1), (run_id, site)
        guest_calls += guest[site]["calls"]
    assert guest_calls == EXPECTED_TOTAL_CALLS[model], (run_id, guest_calls)

    # Decomposition invariants.
    host, ops, conv, guest_total = site_totals(capture)
    assert guest_total >= ops + conv, (run_id, "guest below host")
    for s in capture["host_sites"]:
        g = guest[s["seq"]]["guest_us"] * 1000
        assert g >= s["conv_in_ns"] + s["ops_ns"] + s["conv_out_ns"], (run_id, s["seq"])
    overhead = guest_total - ops
    assert ops / guest_total >= 0.95, (run_id, "ops share below 95%")
    assert overhead / guest_total <= 0.05, (run_id, "overhead above 5%")

    # Wall-relevance bounds (the no-candidate verdict margins).
    wall_ns = meta["wall_seconds"] * 1e9
    assert overhead / wall_ns <= 0.02, (run_id, "serial overhead share above 2%")
    assert overhead / (12 * wall_ns) <= 0.002, (run_id, "saturated share above 0.2%")

    # Rendered bucket cross-checks.
    pn = capture["profile_host_native"]
    assert pn is not None and pn["calls"] >= host_calls, (run_id, "bucket calls")
    bucket_wall = pn["total_wall_ns"]
    assert bucket_wall >= ops, (run_id, "bucket wall below attributed ops")
    ref_ms = t60_reference_ms(run_id)
    measured_ms = guest_total / 1e6
    assert abs(measured_ms - ref_ms) / ref_ms <= 0.25, (run_id, measured_ms, ref_ms)

    return {
        "run_id": run_id,
        "output": meta["sha256"][:12],
        "host_calls": host_calls,
        "guest_ns": guest_total,
        "ops_share": round(ops / guest_total, 4),
        "overhead_share": round(overhead / guest_total, 4),
        "overhead_wall_share_serial": round(overhead / wall_ns, 5),
        "overhead_wall_share_saturated": round(overhead / (12 * wall_ns), 5),
        "t60_ref_ms": ref_ms,
        "measured_ms": round(measured_ms, 1),
    }


def main() -> int:
    refs = references()
    summary = load(HERE / "summary.json")
    captures = [load(path) for path in sorted((HERE / "captures").glob("*.json"))]
    assert len(captures) == 2, len(captures)
    verdicts = {}
    for capture in captures:
        run_id = capture["metadata"]["run_id"]
        verdicts[run_id] = validate(capture, refs)
        cell = summary["cells"][run_id]
        assert cell["metadata"]["sha256"] == capture["metadata"]["sha256"], run_id
        assert cell["host_sites"] == capture["host_sites"], run_id
        assert cell["guest_sites"] == capture["guest_sites"], run_id
    print(json.dumps(verdicts, indent=1, sort_keys=True))

    sample = next(
        c for c in captures if c["metadata"]["run_id"] == "base-classic-off-profile"
    )
    negative_controls = []

    def expect_reject(name, mutate):
        bad = copy.deepcopy(sample)
        mutate(bad)
        try:
            validate(bad, refs)
        except AssertionError:
            negative_controls.append(name)
        else:
            raise AssertionError("negative control was not rejected: " + name)

    expect_reject("output", lambda c: c["metadata"].__setitem__("sha256", "0" * 64))
    expect_reject(
        "dropped-site",
        lambda c: c["host_sites"].pop(
            next(i for i, s in enumerate(c["host_sites"]) if s["seq"] == 5)
        ),
    )
    expect_reject(
        "guest-below-host",
        lambda c: next(s for s in c["guest_sites"] if s["site"] == 5).__setitem__(
            "guest_us", 1
        ),
    )
    expect_reject(
        "inflated-overhead",
        lambda c: next(s for s in c["host_sites"] if s["seq"] == 0).__setitem__(
            "conv_in_ns", 5_000_000_000
        ),
    )
    expect_reject(
        "thin-wall-count",
        lambda c: next(s for s in c["guest_sites"] if s["site"] == 11).__setitem__(
            "calls", 1
        ),
    )
    expect_reject(
        "dump-count",
        lambda c: c.__setitem__("guest_layer_dumps", c["guest_layer_dumps"] - 1),
    )
    expect_reject(
        "bucket-below-attributed",
        lambda c: c["profile_host_native"].__setitem__("total_wall_ns", 1),
    )
    print("negative controls rejected:", ", ".join(negative_controls))
    print("t68 verify: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
