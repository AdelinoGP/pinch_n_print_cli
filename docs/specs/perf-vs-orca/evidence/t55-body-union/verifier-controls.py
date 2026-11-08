"""Demonstrate checkout verifier rejection without modifying saved evidence."""

import json
from pathlib import Path
import shutil
import tempfile

import corpus
import verify

HERE = Path(__file__).resolve().parent


def run():
    rejected = []
    with tempfile.TemporaryDirectory(dir=HERE, prefix="verifier-controls-") as tmp:
        root = Path(tmp)
        for p in HERE.iterdir():
            if p.is_file():
                shutil.copy2(p, root / p.name)
        corpus.HERE = verify.HERE = root
        path = root / "ordinary-base-specimens.jsonl"
        original = path.read_bytes()
        for control in ("vertex", "hole", "role"):
            changed = False
            with path.open("w") as out:
                for line in original.decode().splitlines():
                    row = json.loads(line)
                    if not changed:
                        if control == "vertex" and row["kind"] == "DATA":
                            row["polygon"][0][0][0] += 1
                            changed = True
                        elif (
                            control == "hole"
                            and row["kind"] == "DATA"
                            and len(row["polygon"]) > 1
                        ):
                            row["polygon"].pop()
                            changed = True
                        elif control == "role" and row["kind"] == "ROLE":
                            row["name"] = "RaftRelated"
                            changed = True
                    out.write(json.dumps(row, separators=(",", ":")) + "\n")
            assert changed, "required sabotage target missing"
            try:
                corpus.verify()
            except AssertionError:
                rejected.append(control)
            else:
                raise AssertionError("verifier accepted " + control)
        path.write_bytes(original)
        report_path = root / "native-internals.json"
        report = json.loads(report_path.read_text())
        report["cases"][0]["execute_internal_ns"] = report["cases"][0]["total_ns"] + 1
        report_path.write_text(json.dumps(report))
        try:
            verify.native()
        except AssertionError:
            rejected.append("native_report")
        else:
            raise AssertionError("verifier accepted native report corruption")
    corpus.HERE = verify.HERE = HERE
    assert rejected == ["vertex", "hole", "role", "native_report"]
    (HERE / "verifier-negative-controls.json").write_text(
        json.dumps({"rejected": rejected}, indent=2) + "\n"
    )
    print(
        "PASS: checkout verifier rejected altered vertex, hole, role and native report"
    )


if __name__ == "__main__":
    run()
