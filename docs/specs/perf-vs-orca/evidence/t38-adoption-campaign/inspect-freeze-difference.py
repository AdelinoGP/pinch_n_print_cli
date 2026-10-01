"""Diagnose frozen identity differences read-only; never repairs or re-freezes."""

import hashlib
import json
from pathlib import Path
import runpy

EVIDENCE = Path(__file__).resolve().parent
ROOT = EVIDENCE.parents[4]
ATTEMPT = ROOT / "docs/specs/perf-vs-orca/evidence/t40-reference-preparation/attempt-2"


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def inventory(root):
    return {
        path.relative_to(root).as_posix(): digest(path)
        for path in sorted(root.rglob("*"))
        if path.is_file()
    }


def compare(expected, actual):
    return {
        "missing": sorted(expected.keys() - actual.keys()),
        "added": sorted(actual.keys() - expected.keys()),
        "changed": {
            name: {"expected": expected[name], "actual": actual[name]}
            for name in sorted(expected.keys() & actual.keys())
            if expected[name] != actual[name]
        },
        "expected_count": len(expected),
        "actual_count": len(actual),
    }


def main():
    output = EVIDENCE / "freeze-difference.json"
    if output.exists():
        raise RuntimeError("will not overwrite frozen-difference evidence")
    manifest = json.loads((ATTEMPT / "frozen-manifest.json").read_text())
    result = {
        "manifest_identity_matches": digest(ATTEMPT / "frozen-manifest.json")
        == (ATTEMPT / "frozen-manifest.sha256").read_text().strip(),
        "snapshot_root": manifest["snapshot_root"],
        "snapshot": compare(
            manifest["snapshot_sha256"], inventory(Path(manifest["snapshot_root"]))
        ),
        "corpus": compare(
            manifest["corpus_sha256"], inventory(Path(manifest["corpus_root"]))
        ),
        "historical_corpus": compare(
            json.loads((ATTEMPT / "historical-corpus-hashes.json").read_text()),
            inventory(ROOT / "tmp/rtree_query_corpus"),
        ),
        "build_inputs": compare(
            json.loads((ATTEMPT / "source-inputs.json").read_text()),
            runpy.run_path(str(ATTEMPT.parent / "prepare.py"))["source_inputs"](),
        ),
        "preparation_script_identity_matches": digest(ATTEMPT.parent / "prepare.py")
        == manifest["preparation_script_sha256"],
        "limitation": "identity differences only; no recovery or adoption evidence",
    }
    output.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
