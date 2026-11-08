"""Checkout-only evidence verifier; no models, raw G-code, build, slicing or retries."""

import argparse
import json
import re
import campaign
import manage as m


def load(name):
    return json.loads((m.HERE / name).read_text())


def exactness():
    exact = load("exactness.json")
    assert exact["pass"] is True
    assert all(v["exit_code"] == 0 for v in exact["code_gates"].values())
    for name, digest in exact["logs"].items():
        assert m.sha(m.HERE / "logs" / name) == digest, "exactness log drift"
    provenance = load("patch-provenance.json")
    assert (
        provenance["patch_sha256"]
        == exact["candidate_patch_sha256"]
        == m.sha(m.HERE / "candidate.patch")
    )
    identity = load("identity.json")
    assert (
        provenance["unchanged_top_x_sha256"]
        == identity["registry_inventory"]["src/engine_fns.rs"]
    )
    assert (
        provenance["production_build_inputs_unchanged"]
        and provenance["registry_unchanged"]
    )
    replay_log = (m.HERE / "logs/native-role-replay.log").read_text()
    replay_cases = re.findall(r"T57_EXACT_REPLAY (\w+/\d+) PASS", replay_log)
    # This is the selected, pinned recorded corpus, not a module roster or all-layer claim.
    assert set(replay_cases) == {
        "benchy/168",
        "benchy/152",
        "benchy/43",
        "benchy/0",
        "base/232",
        "base/38",
        "base/21",
        "base/9",
    }
    assert len(replay_cases) == exact["recorded_full_role_cases"] == 8
    builds = load("candidate-builds.json")
    assert (
        set(builds["snapshots"])
        == set(builds["freshness"])
        == {"ordinary", "accelerated"}
    )
    assert all(v == 0 for v in builds["freshness"].values())
    assert all(v == 0 for v in identity["freshness"].values())
    assert builds["resolved_graphs"] and builds["all_graphs"]
    for graph in builds["all_graphs"].values():
        assert (
            graph["identical_nonengine_lock_inventory"]
            and graph["identical_normalized_resolved_packages_edges_features"]
        )
    for consumer in builds["resolved_graphs"].values():
        assert (
            consumer["package"]["version"] == "1.1.0"
            and consumer["package"]["source"] is None
        )
        assert "topx-test-support" not in consumer["features"]
    for mode in ("ordinary", "accelerated"):
        assert (
            identity["baseline_snapshots"][mode].keys()
            == builds["snapshots"][mode].keys()
        )
        diagnostic = builds["diagnose"][mode]
        assert diagnostic["pass"] and diagnostic["modules"]
        assert all(p["provenance"] == "external" for p in diagnostic["modules"])
    controls = load("negative-controls.json")
    for name, control in controls.items():
        assert control["native_exit"] == 101 and control["wasm_exit"] != 0
        assert (
            control["native_assertion_rejected"] and control["wasm_assertion_rejected"]
        )
        suffix = (
            "-native-negative-workspace-resolved.log"
            if name == "stale_y"
            else "-native-negative.log"
        )
        native_log = (m.HERE / "logs" / (name + suffix)).read_text()
        assert "panicked at" in native_log and "0 passed; 1 failed" in native_log
        assert (
            "RuntimeError"
            in (m.HERE / "logs" / (name + "-wasm-negative.log")).read_text()
        )
    print(
        "PASS: exactness/replay/negative-control provenance and actual per-graph integration"
    )


def qualification():
    q = load("qualification.json")
    assert q["pass"] and len(q["cells"]) == 16
    expected = {
        f"{mode}-{fixture}-{generator}-{supports}"
        for mode in ("ordinary", "accelerated")
        for fixture in ("benchy", "base")
        for generator in ("classic", "arachne")
        for supports in ("off", "on")
    }
    assert {c["cell"] for c in q["cells"]} == expected
    frozen = q["inputs"]
    for key, file in [
        ("protocol_sha256", "protocol.json"),
        ("exactness_sha256", "exactness.json"),
        ("identity_sha256", "identity.json"),
        ("candidate_builds_sha256", "candidate-builds.json"),
        ("runner_sha256", "run-cell.ps1"),
        ("output_verifier_sha256", "outputs.py"),
    ]:
        assert frozen[key] == m.sha(m.HERE / file), (
            "frozen qualification identity drift"
        )
    for cell in q["cells"]:
        assert cell["streamed_byte_identical"]
        rows = cell["rows"]
        assert len(rows) == 2 and [r["arm"] for r in rows] == ["baseline", "candidate"]
        for row in rows:
            mode, fixture, generator, supports = cell["cell"].split("-")
            assert [row[k] for k in ("mode", "fixture", "generator", "supports")] == [
                mode,
                fixture,
                generator,
                supports,
            ]
            assert (
                row["threads"] == 12
                and row["exit_code"] == row["fatal"] == row["non_fatal"] == 0
            )
            assert row["status"] == "ok" and row["degraded"] is False
            out = row["output"]
            assert (
                out["sha256"] == row["sha256"]
                and out["bytes"] == row["bytes"]
                and out["type_counts"] == row["type_counts"]
            )
            assert (
                out["extruding_xy_moves"]["Outer wall"] > 0
                and out["extruding_xy_moves"]["Sparse infill"] > 0
            )
            if supports == "on":
                assert (
                    out["extruding_xy_moves"]["Support"] > 0
                    and out["extruding_xy_moves"]["Support interface"] > 0
                )
            else:
                assert not any("Support" in k for k in out["type_counts"])
        for key in (
            "sha256",
            "bytes",
            "type_counts",
            "extruding_xy_moves",
            "warnings",
            "completion",
        ):
            assert rows[0]["output"][key] == rows[1]["output"][key], (
                "qualification arm disclosure drift"
            )
    for fixture in ("benchy", "base"):
        for generator in ("classic", "arachne"):
            for supports in ("off", "on"):
                group = [
                    c["rows"][0]
                    for c in q["cells"]
                    if c["cell"].endswith(f"-{fixture}-{generator}-{supports}")
                ]
                assert len(group) == 2 and group[0]["sha256"] == group[1]["sha256"]
    print(
        "PASS: complete saved matrix, frozen inputs, mode equality and clean nonvacuous roles"
    )


def restored():
    state = load("restoration.json")
    assert (
        state["build_inputs_unchanged"]
        and state["registry_unchanged"]
        and state["archived_artifacts_byte_restored"]
    )
    assert state["freshness"] == {"ordinary": 0, "accelerated": 0}
    seal = load("validation.json")
    campaign = load("campaign.json")
    # A seal must not be able to pass while disagreeing with the measured result
    # or with the restoration record, and its evidence list must be non-vacuous.
    assert seal["outcome"] == campaign["recommendation"], "seal/campaign outcome drift"
    assert seal["final_production_freshness"] == state["freshness"], (
        "seal/restoration freshness drift"
    )
    assert seal["raw_manifest"], "empty raw manifest"
    listed = seal["sealed_evidence"]
    assert listed, "empty sealed evidence inventory"
    for required in (
        "qualification.json",
        "campaign.json",
        "campaign-as-run.py",
        "exactness.json",
        "candidate.patch",
        "protocol.json",
        "restoration.json",
        "test-reducers.py",
    ):
        assert required in listed, "missing sealed evidence: " + required
    for file, digest in listed.items():
        assert m.sha(m.HERE / file) == digest, "sealed evidence drift: " + file
    print(
        "PASS: recorded byte restoration, both production freshness exits and evidence seal"
    )


def interrupted():
    stop = load("interruption.json")
    progress = load("qualification-progress.json")
    assert stop["status"] == "blocked" and stop["qualification_pass"] is False
    assert stop["timing_started"] is False and stop["automatic_retry"] is False
    assert stop["no_performance_verdict"] is True and progress["pass"] is False
    assert not (m.HERE / "qualification.json").exists()
    assert not (m.HERE / "campaign.json").exists()
    expected = [
        f"ordinary-{fixture}-{generator}-{supports}"
        for fixture in ("benchy", "base")
        for generator in ("classic", "arachne")
        for supports in ("off", "on")
    ][:-1]
    assert stop["completed_cells"] == [c["cell"] for c in progress["cells"]] == expected
    assert stop["interrupted_cell"] == "ordinary-base-arachne-on"
    assert stop["interrupted_arm"] == "baseline"
    for cell in progress["cells"]:
        assert cell["streamed_byte_identical"] and len(cell["rows"]) == 2
        a, b = cell["rows"]
        assert a["arm"] == "baseline" and b["arm"] == "candidate"
        for row in (a, b):
            assert row["status"] == "ok" and not row["degraded"]
            assert row["exit_code"] == row["fatal"] == row["non_fatal"] == 0
            moves = row["output"]["extruding_xy_moves"]
            assert moves["Outer wall"] > 0 and moves["Sparse infill"] > 0
            if row["supports"] == "on":
                assert moves["Support"] > 0 and moves["Support interface"] > 0
        for key in (
            "sha256",
            "bytes",
            "type_counts",
            "extruding_xy_moves",
            "warnings",
            "completion",
        ):
            assert a["output"][key] == b["output"][key]
    assert load("validation.json")["outcome"] == "BLOCKED_NO_PERFORMANCE_VERDICT"
    restored()
    print(
        "BLOCKED: partial evidence verified; full qualification and performance acceptance NOT established"
    )


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    scope = parser.add_mutually_exclusive_group()
    scope.add_argument("--exactness-only", action="store_true")
    scope.add_argument("--interrupted", action="store_true")
    args = parser.parse_args()
    exactness()
    if args.interrupted:
        interrupted()
    elif not args.exactness_only:
        qualification()
        campaign.verify()
        restored()
