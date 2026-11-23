"""Stream real output evidence; byte equality is independent of digest equality."""

from collections import Counter
import csv
import json
import re
import manage as m


def equal_files(left, right):
    with left.open("rb") as a, right.open("rb") as b:
        while True:
            x, y = a.read(1024 * 1024), b.read(1024 * 1024)
            assert x == y, "ordered G-code bytes differ"
            if not x:
                return True


def disclosure(output, log, generator, supports):
    types, extruding, warnings = Counter(), Counter(), Counter()
    role, relative_e = "", False
    markers = []
    with output.open("rb") as stream:
        for line in stream:
            if line.startswith(b";TYPE:"):
                role = line.decode().strip()[6:]
                types[role] += 1
            if line.strip() in (b"M82", b"M83"):
                relative_e = line.strip() == b"M83"
            if line.startswith(b"G1 "):
                words = line.split()
                es = [float(w[1:]) for w in words if w.startswith(b"E")]
                if es and es[0] > 0 and any(w.startswith((b"X", b"Y")) for w in words):
                    assert relative_e, "extrusion nonvacuity requires verified M83"
                    extruding[role] += 1
            if b"wall_generator" in line:
                markers.append(line.decode().strip())
    assert markers == ["; wall_generator = " + generator], "generator marker"
    assert extruding["Outer wall"] > 0 and extruding["Sparse infill"] > 0, (
        "nonvacuous model roles"
    )
    if supports == "on":
        assert extruding["Support"] > 0 and extruding["Support interface"] > 0, (
            "nonvacuous support roles"
        )
    else:
        assert not any("Support" in k for k in types), "support-off output has support"
    complete = []
    holders = []
    with log.open(encoding="utf-8", errors="replace") as stream:
        for line in stream:
            assert not any(tag in line for tag in ("shadows integrated", "PERF-T")), (
                "isolation/probe failure"
            )
            if line.lstrip().startswith("{"):
                event = json.loads(line)
                assert event.get("event") not in (
                    "module_complete",
                    "stage_complete",
                    "profile_summary",
                ), "timing probe in output"
                if event.get("event") == "slice_complete":
                    complete.append(event)
            holders.extend(
                re.findall(
                    r"claim 'perimeter-generator' already held by 'com\.core\.(classic|arachne)-perimeters'",
                    line,
                )
            )
            clean = re.sub(r"\x1b\[[0-9;]*m", "", line)
            if re.search(r"\bWARN\b", clean):
                message = re.split(r"\bWARN\b", clean, maxsplit=1)[1].strip()
                if "is not closed" in message:
                    message = "open_loop"
                warnings[message] += 1
    assert holders == [generator], "actual generator claim holder"
    assert len(complete) == 1, "completion population"
    c = complete[0]
    assert (
        c["status"] == "ok"
        and c["degraded"] is False
        and c["fatal_error_count"] == c["non_fatal_error_count"] == 0
    ), "unclean completion"
    return {
        "sha256": m.sha(output),
        "bytes": output.stat().st_size,
        "type_counts": dict(types),
        "extruding_xy_moves": dict(extruding),
        "warnings": dict(warnings),
        "completion": {
            k: c[k]
            for k in (
                "status",
                "degraded",
                "fatal_error_count",
                "non_fatal_error_count",
            )
        },
        "generator": generator,
        "support_mode": supports,
        "stderr_sha256": m.sha(log),
    }


def rows(root):
    result = []
    with (root / "rows.csv").open(encoding="utf-8-sig", newline="") as stream:
        for row in csv.DictReader(stream):
            for k in (
                "pair",
                "warmup",
                "threads",
                "exit_code",
                "bytes",
                "fatal",
                "non_fatal",
            ):
                row[k] = int(row[k])
            for k in ("wall_seconds", "cpu_seconds", "cpu_wall_ratio"):
                row[k] = float(row[k])
            row["degraded"] = row["degraded"].lower() == "true"
            row["type_counts"] = json.loads(row["type_counts"])
            row["warnings"] = json.loads(row["warnings"])
            stem = root / row["run_id"]
            actual = disclosure(
                stem.with_suffix(".gcode"),
                stem.with_suffix(".stderr.jsonl"),
                row["generator"],
                row["supports"],
            )
            for k in ("bytes", "sha256", "type_counts"):
                assert row[k] == actual[k], "CSV/output disagreement: " + k
            assert row["threads"] == 12 and row["exit_code"] == 0
            assert (
                row["status"] == "ok"
                and not row["degraded"]
                and row["fatal"] == row["non_fatal"] == 0
            )
            row["output"] = actual
            result.append(row)
    return result


def audit_snapshots():
    identity = json.loads((m.HERE / "identity.json").read_text())
    candidate = json.loads((m.HERE / "candidate-builds.json").read_text())
    assert candidate.get("resolved_graphs"), (
        "actual dependency resolution audit missing"
    )
    for mode, _ in m.MODES:
        assert identity["freshness"][mode] == candidate["freshness"][mode] == 0
        assert (
            m.inv(m.RAW / "snapshots" / (mode + "-baseline"))
            == identity["baseline_snapshots"][mode]
        )
        assert (
            m.inv(m.RAW / "snapshots" / (mode + "-candidate"))
            == candidate["snapshots"][mode]
        )
    assert all(m.sha(m.REPO / p) == h for p, h in identity["inputs"].items())
    return identity, candidate
