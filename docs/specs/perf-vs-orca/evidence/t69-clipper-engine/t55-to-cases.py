"""Losslessly extract the exact t55 measured body-union arguments."""

import hashlib
import importlib.util
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
SOURCE = HERE.parent / "t55-body-union"


def main():
    spec = importlib.util.spec_from_file_location("t55_corpus", SOURCE / "corpus.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    destination = HERE / "corpus/t55"
    destination.mkdir(parents=True, exist_ok=True)
    manifest = []
    for mode in ("ordinary", "accelerated"):
        for job in ("base", "benchy"):
            source = SOURCE / f"{mode}-{job}-specimens.jsonl"
            output = destination / f"{mode}-{job}.cases"
            cases = list(module.cases(source))
            assert len(cases) == 4
            with output.open("w", encoding="ascii", newline="\n") as stream:
                for index, case in enumerate(cases):
                    carved = case["sets"][("carved", 0)]["polygons"]
                    assert len(carved) > 1
                    stream.write(
                        f"case 550 {case['context']['layer']} 2 {index}\narg 10 1\n"
                    )
                    for label, polygons in (("subj", carved[:1]), ("clip", carved[1:])):
                        stream.write(
                            label
                            + " "
                            + " ".join(map(str, module.integers(polygons)))
                            + "\n"
                        )
                    stream.write("end\n")
            manifest.append(
                {
                    "source": source.name,
                    "source_sha256": hashlib.sha256(source.read_bytes()).hexdigest(),
                    "output": output.name,
                    "bytes": output.stat().st_size,
                    "sha256": hashlib.sha256(output.read_bytes()).hexdigest(),
                    "layers": [c["context"]["layer"] for c in cases],
                }
            )
    total = sum(m["bytes"] for m in manifest)
    assert total <= 20 * 1024 * 1024, (
        "corpus exceeds in-repo budget; choose documented subset"
    )
    (destination / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print(
        json.dumps(
            {
                "files": len(manifest),
                "cases": sum(len(m["layers"]) for m in manifest),
                "bytes": total,
            }
        )
    )


if __name__ == "__main__":
    main()
