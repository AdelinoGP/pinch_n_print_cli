"""Stream selected captures into exact checkout specimens and verify reductions."""

import collections
import copy
import hashlib
import json
from pathlib import Path
import sys

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
RAW = REPO / ".local-artifacts/perimeter-reference-preparation/t55-body-union"


def sha(p):
    with p.open("rb") as f:
        return hashlib.file_digest(f, "sha256").hexdigest()


def rows(path):
    with path.open(encoding="utf8") as f:
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


def cases(path):
    case = None
    for r in rows(path):
        k = r["kind"]
        if k == "BEGIN":
            assert case is None
            case = {
                "context": {a: b for a, b in r.items() if a != "kind"},
                "sets": {},
                "segments": {},
                "returns": {},
                "cuts": [],
            }
        elif k == "ARGS":
            case["args"] = {a: b for a, b in r.items() if a != "kind"}
        elif k == "SET":
            key = (r["label"], r["role"])
            assert key not in case["sets"]
            case["sets"][key] = {"count": r["count"], "polygons": []}
        elif k == "DATA":
            item = case["sets"][(r["label"], r["role"])]
            assert r["index"] == len(item["polygons"])
            item["polygons"].append(r["polygon"])
        elif k == "SEGMENTS":
            case["segments"][r["role"]] = {"count": r["count"], "segments": []}
        elif k == "SEGMENT":
            item = case["segments"][r["role"]]
            assert r["index"] == len(item["segments"])
            item["segments"].append(r["points"])
        elif k == "ROLE":
            case["returns"][r["index"]] = r["name"]
        elif k == "RETURN":
            case["return_count"] = r["count"]
        elif k == "CUT":
            case["cuts"].append({a: b for a, b in r.items() if a != "kind"})
        elif k == "END":
            assert (
                case["return_count"] == len(case["returns"])
                and len(case["segments"]) == 4
            )
            assert len(case["cuts"]) == 3
            for item in case["sets"].values():
                assert item["count"] == len(item["polygons"])
            for item in case["segments"].values():
                assert item["count"] == len(item["segments"])
            yield case
            case = None
    assert case is None


def summary(case):
    polys = lambda label, role: case["sets"][(label, role)]["polygons"]
    input = polys("carved", 0)
    signatures = [json.dumps(p, separators=(",", ":")) for p in input]
    return {
        "context": case["context"],
        "args": case["args"],
        "segments_by_role": [case["segments"][i]["count"] for i in range(4)],
        "areas_by_role": [shape(polys("areas", i)) for i in range(4)],
        "body_carved": shape(input),
        "body_union": shape(polys("union", 0)),
        "exact_duplicate_carved_polygons": len(signatures) - len(set(signatures)),
        "cuts": case["cuts"],
        "returned_roles": [case["returns"][i] for i in range(case["return_count"])],
        "return_shapes": [
            shape(polys("return", i)) for i in range(case["return_count"])
        ],
    }


def reduce():
    result = {}
    for mode in ("ordinary", "accelerated"):
        for job in ("benchy", "base"):
            name = f"{mode}-{job}"
            path = HERE / (name + "-specimens.jsonl")
            with (
                path.open("w", encoding="utf8") as out,
                (RAW / (name + "-probe.log")).open(
                    encoding="utf8", errors="replace"
                ) as stream,
            ):
                for line in stream:
                    if not line.startswith("{"):
                        continue
                    try:
                        e = json.loads(line)
                    except ValueError:
                        continue
                    m = e.get("message", "")
                    if not m.startswith("PERF-T55-"):
                        continue
                    tag, _, payload = m.partition(" ")
                    r = json.loads(payload)
                    r["kind"] = tag[len("PERF-T55-") :]
                    out.write(json.dumps(r, separators=(",", ":")) + "\n")
            result[name] = {
                "raw_log_sha256": sha(RAW / (name + "-probe.log")),
                "specimen_sha256": sha(path),
                "cases": [summary(c) for c in cases(path)],
            }
    (HERE / "evidence.json").write_text(json.dumps(result, indent=2) + "\n")
    replay_text()


def integers(polys):
    yield len(polys)
    for poly in polys:
        yield len(poly)
        for ring in poly:
            yield len(ring)
            for p in ring:
                yield from p


def replay_text(destination=HERE):
    # Framed numeric streaming format: f32 values stored as exact integer bits.
    for job in ("benchy", "base"):
        all_cases = list(cases(HERE / ("ordinary-" + job + "-specimens.jsonl")))
        with (destination / (job + "-replay.txt")).open("w") as out:
            out.write(str(len(all_cases)) + "\n")
            for c in all_cases:
                a = c["args"]
                out.write(
                    f"{c['context']['layer']} {a['radius_bits']} {a['width_bits']} {a['avg_nodes']}\n"
                )
                for role in range(4):
                    segments = c["segments"][role]["segments"]
                    out.write(str(len(segments)) + "\n")
                    for segment in segments:
                        out.write(
                            str(len(segment))
                            + " "
                            + " ".join(str(v) for point in segment for v in point)
                            + "\n"
                        )
                for label, roles in [
                    ("areas", range(4)),
                    ("collision", [0]),
                    ("carved", range(4)),
                    ("union", range(4)),
                    ("simplify", range(4)),
                    ("difference", range(4)),
                    ("body_after_0", [0]),
                    ("body_after_1", [0]),
                    ("body_after_2", [0]),
                ]:
                    for role in roles:
                        out.write(
                            " ".join(
                                map(str, integers(c["sets"][(label, role)]["polygons"]))
                            )
                            + "\n"
                        )
                out.write(str(c["return_count"]) + "\n")
                names = {
                    "SupportBody": 0,
                    "TopInterface": 1,
                    "BaseInterface": 2,
                    "BottomInterface": 3,
                }
                for index in range(c["return_count"]):
                    out.write(
                        str(names[c["returns"][index]])
                        + " "
                        + " ".join(
                            map(str, integers(c["sets"][("return", index)]["polygons"]))
                        )
                        + "\n"
                    )


def verify():
    e = json.loads((HERE / "evidence.json").read_text())
    assert set(e) == {
        f"{m}-{j}" for m in ("ordinary", "accelerated") for j in ("benchy", "base")
    }
    for name, r in e.items():
        path = HERE / (name + "-specimens.jsonl")
        assert sha(path) == r["specimen_sha256"]
        actual = [summary(c) for c in cases(path)]
        assert actual == r["cases"] and len(actual) == 4
        assert any(c["body_union"][1] > 0 for c in actual), "hole coverage absent"
        assert any(
            cut["ran"] and cut["changed"] for c in actual for cut in c["cuts"]
        ), "positive body subtraction absent"
        baseline = json.loads((HERE / (name + "-baseline-output.json")).read_text())
        probe = json.loads((HERE / (name + "-probe-output.json")).read_text())
        assert (
            baseline["sha256"] == probe["sha256"]
            and baseline["bytes"] == probe["bytes"]
        )
        for o in (baseline, probe):
            assert o["completion"]["status"] == "ok" and not o["completion"]["degraded"]
            assert (
                o["completion"]["fatal_error_count"]
                == o["completion"]["non_fatal_error_count"]
                == 0
            )
            assert o["support_extruding_xy_moves"]["Support"] > 0
    for job, largest, holes in [("benchy", 18200, 2), ("base", 102400, 12)]:
        ordinary = HERE / ("ordinary-" + job + "-specimens.jsonl")
        accelerated = HERE / ("accelerated-" + job + "-specimens.jsonl")
        assert sha(ordinary) == sha(accelerated), (
            "ordered arguments/intermediates/results differ by mode"
        )
        assert (
            max(c["body_carved"][2] for c in e["ordinary-" + job]["cases"]) == largest
        )
        assert max(c["body_union"][1] for c in e["ordinary-" + job]["cases"]) == holes
    print(
        "PASS: largest, holes and positive full-role subtraction; mode-identical exact corpus"
    )


if __name__ == "__main__":
    if "--reduce" in sys.argv:
        reduce()
    verify()
