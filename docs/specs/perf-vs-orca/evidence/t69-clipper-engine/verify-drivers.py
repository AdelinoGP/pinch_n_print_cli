"""Independent rectangle/hash protocol controls, not production cargo tests."""

import itertools
import json
from pathlib import Path
import struct
import subprocess

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
SCRATCH = REPO / ".local-artifacts/perf-vs-orca/t69-clipper-engine"


def rectangle(x0, y0, x1, y1):
    return [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]


def fnv(points):
    value = 14695981039346656037
    for x, y in points:
        for byte in struct.pack("<qq", x, y):
            value = ((value ^ byte) * 1099511628211) & ((1 << 64) - 1)
    return f"{value:016x}"


def allowed_hashes(polys):
    # Rectangle geometry is known independently; allow any engine ring start,
    # orientation or component order, not any change to vertex values.
    variants = []
    for ring in polys:
        variants.append([r[i:] + r[:i] for r in (ring, ring[::-1]) for i in range(4)])
    hashes = set()
    for rings in itertools.product(*variants):
        for ordered in itertools.permutations(rings):
            hashes.add(fnv([point for ring in ordered for point in ring]))
    return hashes


def main():
    raw = SCRATCH / "raw/protocol-controls"
    raw.mkdir(parents=True, exist_ok=True)
    subject = rectangle(-1000, -1000, 0, 0)
    clip = rectangle(-500, -1000, 500, 0)
    expected = [
        [rectangle(-1100, -1100, 100, 100)],
        [rectangle(-1000, -1000, -500, 0)],
        [rectangle(-1000, -1000, 500, 0)],
        [rectangle(-500, -1000, 0, 0)],
        [rectangle(-1000, -1000, -500, 0), rectangle(0, -1000, 500, 0)],
        [],
    ]
    lines = ["# Independent negative-coordinate rectangle controls"]

    def geom(label, ring):
        return f"{label} 1 1 4 " + " ".join(str(v) for p in ring for v in p)

    bits = lambda value: struct.unpack("<Q", struct.pack("<d", value))[0]
    for op in range(5):
        lines.append(f"case 0 -1 {op}")  # optional call_index deliberately absent
        if op == 0:
            lines += [
                f"arg 1 {bits(100.0)}",
                "arg 2 3",
                "arg 3 0",
                f"arg 4 {bits(2.0)}",
                f"arg 5 {bits(0.0)}",
            ]
        else:
            lines += ["arg 10 1", "arg 7 0"]
        lines.append(geom("subj", subject) + " # negative i64 coordinates")
        if op:
            lines.append(geom("clip", clip))
        lines.append("end")
    lines += ["case RP1 -1 3 5", "arg 10 1", geom("subj", subject), "clip 0", "end"]
    corpus = raw / "rectangles.cases"
    corpus.write_text("\n".join(lines) + "\n")
    outcomes = {}
    for name, binary in (
        ("cpp", SCRATCH / "cpp-driver.exe"),
        ("rust", SCRATCH / "rust-harness/target/release/t69-dual-engine.exe"),
    ):
        output = subprocess.check_output(
            [str(binary), str(corpus)], text=True, timeout=60
        )
        (raw / f"{name}.jsonl").write_text(output)
        rows = [json.loads(line) for line in output.splitlines()]
        assert len(rows) == len(expected)
        for index, (row, polys) in enumerate(zip(rows, expected)):
            assert row["case_index"] == index
            assert (row["out_paths"], row["out_rings"], row["out_verts"]) == (
                len(polys),
                len(polys),
                4 * len(polys),
            ), row
            assert row["out_hash"] in allowed_hashes(polys), (
                name,
                index,
                "coordinate/hash oracle mismatch",
                row,
            )
        # Negative control: unknown arg must fail, not be ignored.
        invalid = raw / "invalid-arg.cases"
        invalid.write_text("case 0 0 2\narg 10 1\narg 99 0\nsubj 0\nclip 0\nend\n")
        bad = subprocess.run(
            [str(binary), str(invalid)], capture_output=True, text=True, timeout=60
        )
        assert bad.returncode != 0, "unknown arg silently accepted"
        (raw / f"{name}-invalid.stderr.log").write_text(bad.stderr)
        outcomes[name] = {
            "rectangle_cases_passed": len(rows),
            "unknown_arg_rejected": True,
        }
    (raw / "verification.json").write_text(json.dumps(outcomes, indent=2) + "\n")
    print(json.dumps(outcomes))


if __name__ == "__main__":
    main()
