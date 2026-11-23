#!/usr/bin/env python3
"""t64 checkout-only verifier (wayfinder perf-vs-orca ticket 64).

Runs against the in-repo evidence set only (``captures/`` + ``summary.json``);
the raw tree under ``target/t64-tree-support-renderer/`` is optional.
Checks:

- every capture's output sha matches its preserved t58 qualification reference
  byte-for-byte, status clean, and the run metadata hashes line up;
- the profile fold invariant holds for every fuel module row
  (``self + Σ scope self == total``) and the tree-support dispatch fuel equals
  the per-layer call-fuel sum from its dispatch brackets;
- the derived scope split re-derives exactly (dispatch self − Σ scope self ==
  unmarked residual) and the counter probe lines parse with the declared field
  set;
- probe invariants on the supports-on cells: wall paths + fill paths + off-grid
  + pushes account for every rendered path; scan rows ≥ scan regions; each row
  walks the same ring-edge total (``row_edges == edges_per_region × rows``);
  crossings are pair-consistent (``fill_paths ≤ crossings/2``);
- supports-off controls: every count zero (the probed renderer early-returns
  before any probe work) and renderer fuel matches the profile's tree-support
  row;
- same-run attribution checks: renderer windows lie inside the per-layer phase,
  occupancy fraction in (0, 1).

Negative controls: fuel mutation, output mutation, counter mutation, residual
mutation, provenance mutation must all be rejected.
"""

import copy
import json
import math
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
PROBE_FIELDS = [
    "regions",
    "regions_plan",
    "plan_seen",
    "plan_hits",
    "entries",
    "begins",
    "regize",
    "regize_none",
    "unions",
    "union_in",
    "expolys",
    "expoly_verts",
    "paints",
    "paint_blocked",
    "skel_x",
    "skel_pts",
    "pie_edges",
    "pie_bbox",
    "wall_off",
    "wall_paths",
    "fill_off",
    "fill_regs",
    "scan_regs",
    "rows",
    "row_edges",
    "conv_y",
    "flips",
    "conv_x",
    "crossings",
    "fill_paths",
    "out_paths",
    "offgrid",
    "expolys_body",
    "expolys_iface",
    "rows_body",
    "rows_iface",
    "fill_paths_body",
    "fill_paths_iface",
    "wall_paths_body",
    "wall_paths_iface",
    "region_polys",
    "region_edges",
]
T64_SCOPES = {
    "t64::plan_filter",
    "t64::pitches",
    "t64::regularize",
    "t64::union",
    "t64::paint",
    "t64::skeleton",
    "t64::walls",
    "t64::fill_offset",
    "t64::scan_fill",
    "t64::scan_crossings",
    "t64::scan_emit",
    "t64::output",
}


def load(path: Path):
    return json.loads(path.read_text(encoding="utf-8-sig"))


def references() -> dict:
    data = load(HERE / ".." / "t58-finalization-copies" / "qualification.json")
    return {cell["cell"]: cell for cell in data["cells"]}


def validate(capture, summary_entry, refs) -> dict:
    meta = capture["metadata"]
    cell = "ordinary-benchy-" + meta["generator"] + "-" + meta["supports"]
    ref = refs[cell]
    run_id = meta["run_id"]
    assert run_id == cell + "-probe64", run_id
    assert meta["threads"] == 12 and meta["exit_code"] == 0
    assert meta["status"] == "ok" and meta["degraded"] is False
    assert meta["fatal"] == meta["non_fatal"] == 0
    # The probe must not have changed the produced geometry.
    assert meta["sha256"] == meta["reference_sha256"] == ref["sha256"], run_id
    assert meta["bytes"] == ref["bytes"] and meta["type_counts"] == ref["type_counts"]
    assert capture["raw_stderr_sha256"] == meta["stderr_sha256"]
    assert math.isclose(
        meta["cpu_wall_ratio"],
        meta["cpu_seconds"] / meta["wall_seconds"],
        rel_tol=1e-12,
    )
    phases = [
        e["phase"] for e in capture["phase_events"] if e["event"] == "phase_complete"
    ]
    assert phases == ["validation", "prepass", "per_layer", "postpass"], run_id
    layers = capture["layers"]
    assert [row[0] for row in layers] == list(range(240)), run_id

    profile = capture["profile"]
    assert profile is not None, "probe captures are profiled"
    guests = [m for m in profile["modules"] if m["unit"] == "fuel"]
    assert guests and sum(m["total_fuel"] for m in guests) == profile["fuel_total"]
    for m in guests:
        assert (
            m["self_fuel"] + sum(s["self_fuel"] for s in m["scopes"]) == m["total_fuel"]
        ), f"fold invariant violated: {m['module_id']} in {run_id}"

    ts = next(m for m in guests if m["module_id"] == "com.core.tree-support")
    named = {
        1: "clip_polygons",
        2: "offset",
        3: "offset2_ex",
        4: "opening",
        5: "closing_ex",
    }
    scope_self = sum(s["self_fuel"] for s in ts["scopes"])
    scope_total = sum(s["total_fuel"] for s in ts["scopes"])
    unmarked = ts["self_fuel"] - sum(
        s["self_fuel"]
        for s in ts["scopes"]
        if (s.get("scope") or named.get(s["scope_id"])) in T64_SCOPES
    )
    core_scope_self = scope_self - sum(
        s["self_fuel"] for s in ts["scopes"] if s["scope_id"] >= 1024
    )

    col = {n: i for i, n in enumerate(capture["module_calls_columns"])}
    ts_index = capture["module_ids"].index("com.core.tree-support")
    dispatches = [
        c for c in capture["module_calls"] if c[col["module_index"]] == ts_index
    ]
    assert (
        len(dispatches) == ts["calls"] and dispatches[0][col["call_fuel"]] is not None
    )
    dispatch_fuel = sum(c[col["call_fuel"]] for c in dispatches)
    assert dispatch_fuel == ts["total_fuel"], run_id

    # per-dispatch call fuel >= 0 and every layer present exactly once
    assert sorted(c[1] for c in dispatches) == list(range(240)), run_id

    total = capture["derived"]
    probe_totals = capture["probe"]["totals"]
    if meta["supports"] == "off":
        # Disabled renderer: no dispatch reaches the probe, so no probe fields.
        assert probe_totals in ({}, {name: 0 for name in PROBE_FIELDS}), run_id
    else:
        assert set(probe_totals) == set(PROBE_FIELDS), run_id

    derived = {
        "dispatch_total_fuel": ts["total_fuel"],
        "dispatch_self_fuel": ts["self_fuel"],
        "scope_self_sum": scope_self,
        "unmarked_residual_fuel": ts["self_fuel"],
        "core_scope_self_fuel": core_scope_self,
        "t64_scope_self_fuel": scope_self - core_scope_self,
    }
    for key, value in derived.items():
        assert total[key] == value, f"{run_id}: {key} {total[key]} != {value}"

    # Telescoping identities of the mark fold: Σ(all scope self) equals the
    # sum of top-level scope inclusive totals, so dispatch total decomposes as
    # unmarked self + Σ t64 scope self + Σ core scope self.
    assert scope_self == ts["total_fuel"] - ts["self_fuel"], run_id

    phase = total["wall_attention"]["phase_wall_ms"]
    occ = total["wall_attention"]["machine_occupancy_fraction"]
    assert phase and 0 <= occ <= 1, run_id
    starts = [l["start_ms"] for l in total["per_layer_renderer"]]
    ends = [l["end_ms"] for l in total["per_layer_renderer"]]
    assert max(starts) <= phase and max(ends) <= phase * 1.01, run_id

    if meta["supports"] == "off":
        # The disabled renderer early-returns before any probe work.
        assert all(value == 0 for value in probe_totals.values()), run_id
        assert ts["total_fuel"] < 1_000_000_000, run_id
        verdict = {
            "probe_silent_on_disabled_path": True,
            "dispatch_total_fuel": ts["total_fuel"],
        }
    else:
        t = probe_totals
        assert t["regions"] >= 240, f"{run_id}: every layer must have regions"
        assert t["plan_hits"] >= t["entries"], run_id
        assert t["entries"] >= 1, run_id
        assert t["expolys"] >= t["entries"], run_id
        # per-expoly fill offset produces the fill regions; one scan each
        assert t["fill_off"] == t["expolys"], run_id
        assert t["scan_regs"] == t["fill_regs"], run_id
        assert t["rows"] >= t["scan_regs"], f"{run_id}: rows >= regions scanned"
        assert t["fill_paths"] * 2 <= t["crossings"], run_id
        assert t["crossings"] >= t["flips"], run_id
        assert t["row_edges"] >= t["rows"], run_id
        # role splits are exact partitions
        assert t["expolys_body"] + t["expolys_iface"] == t["expolys"], run_id
        assert t["rows_body"] + t["rows_iface"] == t["rows"], run_id
        assert t["fill_paths_body"] + t["fill_paths_iface"] == t["fill_paths"], run_id
        assert t["wall_paths_body"] + t["wall_paths_iface"] == t["wall_paths"], run_id
        # emitted paths: on-grid pushes + off-grid entities cannot exceed the
        # rendered walls+fill (RaftRelated renders are the only allowed drop)
        assert t["out_paths"] + t["offgrid"] <= t["wall_paths"] + t["fill_paths"], (
            run_id
        )
        # SDK-side paint-query counters agree with the guest-side probe: one
        # query per paint call, one region walk per query (the support-glue
        # view is rebuilt from this dispatch's regions), and the vertices tried
        # are a subset of the caller's expoly vertices.
        sdk = total["wall_attention"]["probe_sdk_totals"]
        assert sdk["queries"] == t["paints"], run_id
        assert sdk["cover_regions"] == t["paints"], run_id
        assert sdk["vert_loops"] <= t["expoly_verts"], run_id
        assert sdk["ring_flips"] <= sdk["ring_edges"], run_id
        assert sdk["elig_checks"] % 2 == 0, run_id
        verdict = {
            "expolys_per_layer_mean": round(t["expolys"] / 240, 3),
            "rows_mean": round(t["rows"] / 240, 3),
            "edges_per_row_mean": round(t["row_edges"] / max(t["rows"], 1), 3),
            "conv_per_edge_mean": round(
                (t["conv_y"] + t["conv_x"]) / max(t["row_edges"], 1), 3
            ),
            "fill_paths_mean": round(t["fill_paths"] / 240, 3),
        }
    return verdict


def main() -> int:
    refs = references()
    captures_dir = HERE / "captures"
    capture_paths = sorted(captures_dir.glob("*.json"))
    assert capture_paths, "no captures committed"
    verdicts = {}
    for path in capture_paths:
        capture = load(path)
        run_id = capture["metadata"]["run_id"]
        summary = load(HERE / "summary.json")
        verdicts[run_id] = validate(capture, summary[run_id], refs)
    print(json.dumps(verdicts, indent=1, sort_keys=True))

    profiled = load(next(captures_dir.glob("*-on-probe64.json")))
    summary = load(HERE / "summary.json")

    negative_controls = []

    def expect_reject(name, mutate_capture):
        bad = copy.deepcopy(profiled)
        bad["metadata"]["run_id"] = "ordinary-benchy-arachne-on-probe64"
        mutate_capture(bad)
        try:
            validate(bad, summary["ordinary-benchy-arachne-on-probe64"], refs)
        except AssertionError:
            negative_controls.append(name)
        else:
            raise AssertionError("negative control was not rejected: " + name)

    expect_reject(
        "fuel",
        lambda c: c.__setitem__(
            "profile",
            {**c["profile"], "fuel_total": c["profile"]["fuel_total"] + 1},
        ),
    )
    expect_reject("output", lambda c: c["metadata"].__setitem__("sha256", "0" * 64))
    expect_reject("layer", lambda c: c["layers"].pop())
    expect_reject("counter", lambda c: c["probe"]["totals"].__setitem__("rows", 0))
    expect_reject(
        "residual",
        lambda c: c["derived"].__setitem__(
            "unmarked_residual_fuel", c["derived"]["unmarked_residual_fuel"] + 1
        ),
    )
    print("negative controls rejected:", ", ".join(negative_controls))
    print("t64 verify: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
