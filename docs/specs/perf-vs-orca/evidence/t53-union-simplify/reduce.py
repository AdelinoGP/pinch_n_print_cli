"""Reduce streamed probe logs and falsify headline claims using only tracked evidence."""

import copy
import hashlib
import json
from pathlib import Path
import sys

HERE = Path(__file__).resolve().parent


def digest(p):
    with p.open("rb") as f:
        return hashlib.file_digest(f, "sha256").hexdigest()


def records(p):
    with p.open(encoding="utf-8") as f:
        for line in f:
            yield json.loads(line)


def shape(polys):
    rings = [r for p in polys for r in p]
    return [
        len(polys),
        sum(len(p) - 1 for p in polys),
        sum(len(r) for r in rings),
        max((len(r) for r in rings), default=0),
    ]


def coverage():
    result = {}
    for job in ("benchy", "base"):
        items = list(records(HERE / ("ordinary-" + job + "-records.jsonl")))
        roles = [r for r in items if r["kind"] == "ROLE"]
        captures = [r for r in items if r["kind"] == "CAPTURE"]
        result[job] = {
            "hole_bearing_union_rows": sum(r["union"][1] > 0 for r in roles),
            "selected_union_holes": sum(shape(r["union"])[1] for r in captures),
            "selected_count": len(captures),
            "largest_union_holes": max(r["union"][1] for r in roles),
            "body_union_output_maxima": [
                max(r["union"][i] for r in roles if r["role"] == 0) for i in range(4)
            ],
        }
    return result


def summary(items):
    roles = [r for r in items if r["kind"] == "ROLE"]
    hosts = {r["id"]: r for r in items if r["kind"] == "HOST"}
    by_role = {}
    for role in range(4):
        rows = [r for r in roles if r["role"] == role]
        calls = [r for r in rows if r["input"][0] > 1]
        h = [hosts[r["id"]] for r in calls]
        full = sum(r["full_ns"] for r in h)
        guest = sum(r["union_us"] * 1000 for r in calls)
        by_role[str(role)] = {
            "rows": len(rows),
            "union_calls": len(calls),
            "empty_bypass": sum(r["input"][0] == 0 for r in rows),
            "singleton_bypass": sum(r["input"][0] == 1 for r in rows),
            "simplify_activations": sum(r["tolerance"] >= 0 for r in rows),
            "avg_nodes": sorted(set(r["avg_nodes"] for r in rows)),
            "core_ns": sum(r["core_ns"] for r in h),
            "host_conversion_ns": sum(
                r["input_convert_ns"] + r["output_convert_ns"] for r in h
            ),
            "host_full_ns": full,
            "guest_union_call_ns": guest,
            "roundtrip_residual_ns": guest - full,
            "union_bypass_ns": sum(
                r["union_us"] * 1000 for r in rows if r["input"][0] <= 1
            ),
            "inactive_simplify_branch_ns": sum(
                r["simplify_us"] * 1000 for r in rows if r["tolerance"] < 0
            ),
            "active_simplify_ns": sum(
                r["simplify_us"] * 1000 for r in rows if r["tolerance"] >= 0
            ),
            "structural_ns": sum(r["structural_us"] * 1000 for r in rows),
            "carve_ns": sum(r["carve_us"] * 1000 for r in rows),
            "difference_ns": sum(r["difference_us"] * 1000 for r in rows),
            "shape_totals": {
                k: [sum(r[k][i] for r in rows) for i in range(3)]
                for k in ("input", "union", "simplify", "final")
            },
            "shape_maxima": {
                k: [max((r[k][i] for r in rows), default=0) for i in range(4)]
                for k in ("input", "union", "simplify", "final")
            },
        }
    keys = [
        "rows",
        "union_calls",
        "empty_bypass",
        "singleton_bypass",
        "simplify_activations",
        "core_ns",
        "host_conversion_ns",
        "host_full_ns",
        "guest_union_call_ns",
        "roundtrip_residual_ns",
        "union_bypass_ns",
        "inactive_simplify_branch_ns",
        "active_simplify_ns",
        "structural_ns",
        "carve_ns",
        "difference_ns",
    ]
    total = {k: sum(r[k] for r in by_role.values()) for k in keys}
    total["body_difference_ns"] = sum(
        r["us"] * 1000 for r in items if r["kind"] == "BODY-DIFFERENCE"
    )
    return {"total": total, "by_role": by_role}


def reduce(raw):
    cells = {}
    for mode in ("ordinary", "accelerated"):
        for job in ("benchy", "base"):
            cell = f"{mode}-{job}"
            items = []
            context = None
            with (raw / (cell + "-probe.log")).open(
                encoding="utf-8", errors="replace"
            ) as f:
                for line in f:
                    if not line.startswith("{"):
                        continue
                    try:
                        event = json.loads(line)
                    except json.JSONDecodeError:
                        continue
                    message = event.get("message", "")
                    if not message.startswith("PERF-T53-"):
                        continue
                    tag, _, payload = message.partition(" ")
                    kind = tag[len("PERF-T53-") :]
                    if kind in ("UNION-BEGIN", "UNION-END"):
                        continue
                    row = json.loads(payload)
                    row["kind"] = kind
                    if kind == "CONTEXT":
                        context = {k: v for k, v in row.items() if k != "kind"}
                    else:
                        row["context"] = context
                    items.append(row)
            path = HERE / (cell + "-records.jsonl")
            path.write_text(
                "".join(json.dumps(r, separators=(",", ":")) + "\n" for r in items),
                encoding="utf-8",
            )
            cells[cell] = {
                "summary": summary(items),
                "raw_log_sha256": digest(raw / (cell + "-probe.log")),
                "records_sha256": digest(path),
            }
    (HERE / "evidence.json").write_text(
        json.dumps(cells, indent=2) + "\n", encoding="utf-8"
    )
    (HERE / "coverage.json").write_text(json.dumps(coverage(), indent=2) + "\n")


def validate(data):
    assert set(data) == {
        f"{mode}-{job}"
        for mode in ("ordinary", "accelerated")
        for job in ("benchy", "base")
    }
    assert coverage() == json.loads((HERE / "coverage.json").read_text())
    for cell, e in data.items():
        items = list(records(HERE / (cell + "-records.jsonl")))
        assert digest(HERE / (cell + "-records.jsonl")) == e["records_sha256"]
        assert summary(items) == e["summary"], "wrong attribution totals"
        roles = {r["id"]: r for r in items if r["kind"] == "ROLE"}
        hosts = {r["id"]: r for r in items if r["kind"] == "HOST"}
        assert len(roles) == e["summary"]["total"]["rows"]
        assert set(hosts) == {i for i, r in roles.items() if r["input"][0] > 1}, (
            "uncorrelated host calls"
        )
        for h in hosts.values():
            assert (
                h["full_ns"]
                == h["core_ns"] + h["input_convert_ns"] + h["output_convert_ns"]
            )
        for r in roles.values():
            assert r["context"] and r["role"] == r["id"] % 4
            if r["tolerance"] < 0:
                assert r["union"] == r["simplify"]
        captures = [r for r in items if r["kind"] == "CAPTURE"]
        assert captures and any(len(r["input"]) > 1 for r in captures)
        for r in captures:
            role = roles[r["id"]]
            for k in ("input", "union", "simplify", "final"):
                assert shape(r[k]) == role[k]
                assert all(
                    isinstance(v, int)
                    for p in r[k]
                    for ring in p
                    for point in ring
                    for v in point
                )
            if role["tolerance"] < 0:
                assert r["union"] == r["simplify"]
        baseline = json.loads((HERE / (cell + "-baseline-output.json")).read_text())
        probe = json.loads((HERE / (cell + "-probe-output.json")).read_text())
        assert (
            baseline["sha256"] == probe["sha256"]
            and baseline["bytes"] == probe["bytes"]
        ), "wrong output identities"
        for o in (baseline, probe):
            assert o["completion"]["status"] == "ok" and not o["completion"]["degraded"]
            assert (
                o["completion"]["fatal_error_count"]
                == o["completion"]["non_fatal_error_count"]
                == 0
            )
            assert any("Support" in k and v for k, v in o["types"].items())
            assert o["support_extruding_xy_moves"]["Support"] > 0
            assert any("classic" in marker for marker in o["generator"])
    for job in ("benchy", "base"):
        a = list(records(HERE / ("ordinary-" + job + "-records.jsonl")))
        b = list(records(HERE / ("accelerated-" + job + "-records.jsonl")))
        keys = (
            "id",
            "role",
            "avg_nodes",
            "tolerance",
            "input",
            "union",
            "simplify",
            "final",
            "context",
        )
        assert [{k: r[k] for k in keys} for r in a if r["kind"] == "ROLE"] == [
            {k: r[k] for k in keys} for r in b if r["kind"] == "ROLE"
        ]
        assert [r for r in a if r["kind"] in ("CAPTURE", "RETURN")] == [
            r for r in b if r["kind"] in ("CAPTURE", "RETURN")
        ]
    return True


def falsify(data):
    wrong = copy.deepcopy(data)
    next(iter(wrong.values()))["summary"]["total"]["core_ns"] += 1
    try:
        validate(wrong)
    except AssertionError:
        pass
    else:
        raise AssertionError("wrong totals accepted")
    cell = next(iter(data))
    p = HERE / (cell + "-probe-output.json")
    original = p.read_bytes()
    wrong = json.loads(original)
    wrong["sha256"] = "0" * 64
    try:
        p.write_text(json.dumps(wrong))
        try:
            validate(data)
        except AssertionError:
            pass
        else:
            raise AssertionError("wrong identity accepted")
    finally:
        p.write_bytes(original)
    print("PASS negative controls: wrong totals and output identity rejected")


def replay_fixture():
    rows = []
    for job in ("benchy", "base"):
        items = list(records(HERE / ("ordinary-" + job + "-records.jsonl")))
        roles = {r["id"]: r for r in items if r["kind"] == "ROLE"}
        for r in items:
            if r["kind"] != "CAPTURE":
                continue
            row = [roles[r["id"]]["tolerance"]]
            for k in ("input", "collision", "union", "simplify", "final"):
                polys = r[k]
                row.append(len(polys))
                for poly in polys:
                    row.append(len(poly))
                    for ring in poly:
                        row.append(len(ring))
                        for x, y in ring:
                            row.extend([x, y])
            rows.append(" ".join(map(str, row)))
    (HERE / "replay.txt").write_text(str(len(rows)) + "\n" + "\n".join(rows) + "\n")


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "--reduce":
        reduce(Path(sys.argv[2]))
    data = json.loads((HERE / "evidence.json").read_text())
    validate(data)
    if "--falsify" in sys.argv:
        falsify(data)
    if "--replay-fixture" in sys.argv:
        replay_fixture()
    print("PASS: four qualified cells; attribution rederived from in-repo records")
