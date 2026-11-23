"""Stream raw captures; verify reduced headline evidence without local payloads."""

import hashlib
import json
from collections import Counter
from pathlib import Path
import sys

HERE = Path(__file__).resolve().parent


def output_row(output, log):
    types = Counter()
    generator = []
    fan = 0
    role = ""
    relative_e = False
    support_extruding_xy_moves = Counter()
    with output.open("rb") as f:
        for line in f:
            if line.startswith(b";TYPE:"):
                types[line.decode().strip()[6:]] += 1
                role = line.decode().strip()[6:]
            if line.strip() == b"M83":
                relative_e = True
            if line.strip() == b"M82":
                relative_e = False
            if line.startswith(b"G1 ") and "Support" in role:
                words = line.split()
                es = [float(w[1:]) for w in words if w.startswith(b"E")]
                if es and es[0] > 0 and any(w.startswith((b"X", b"Y")) for w in words):
                    assert relative_e, "nonvacuity counter requires verified M83"
                    support_extruding_xy_moves[role] += 1
            if b"wall_generator" in line:
                generator.append(line.decode().strip())
            if line.startswith((b"M106", b"M107")):
                fan += 1
    events = []
    with log.open(encoding="utf-8", errors="replace") as f:
        for line in f:
            if line.startswith("{"):
                try:
                    event = json.loads(line)
                except json.JSONDecodeError:
                    continue
                if event.get("event") in ("slice_complete", "slice_stats"):
                    events.append(event)
    complete = [e for e in events if e["event"] == "slice_complete"]
    assert len(complete) == 1 and complete[0]["status"] == "ok", complete
    assert complete[0].get("degraded") is False, complete
    assert (
        complete[0].get("non_fatal_error_count") == 0
        and complete[0].get("fatal_error_count") == 0
    ), complete
    assert any("Support" in k and v > 0 for k, v in types.items()), types
    assert support_extruding_xy_moves["Support"] > 0
    assert any("classic" in s for s in generator), generator
    with output.open("rb") as f:
        digest = hashlib.file_digest(f, "sha256").hexdigest()
    return {
        "sha256": digest,
        "bytes": output.stat().st_size,
        "types": dict(types),
        "generator": generator,
        "fan_commands": fan,
        "support_extruding_xy_moves": dict(support_extruding_xy_moves),
        "completion": complete[0],
        "stats": events,
    }


if __name__ == "__main__":
    import reduce

    data = json.loads((HERE / "evidence.json").read_text())
    reduce.validate(data)
    if "--falsify" in sys.argv:
        reduce.falsify(data)
    print(
        "PASS: current-job attribution and output qualification (no raw captures needed)"
    )
