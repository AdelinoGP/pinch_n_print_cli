"""Read-only reference/snapshot/source identity gate; never generates output.

Usage: python verify-freeze.py <attempt-directory>
Run before any accelerated comparison and again after the campaign. A changed
input is a blocker requiring an explicit new preparation decision, not a repair
of this frozen reference set. Printed JSON can be saved as a freeze attestation.
"""

from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import runpy
import sys

ROOT = Path(__file__).resolve().parents[5]


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def inventory(directory):
    return {
        path.relative_to(directory).as_posix(): digest(path)
        for path in sorted(directory.rglob("*"))
        if path.is_file()
    }


def main():
    if sys.flags.optimize:
        raise RuntimeError("freeze verification requires Python assertions enabled")
    attempt = Path(sys.argv[1]).resolve()
    manifest_path = attempt / "frozen-manifest.json"
    assert (
        digest(manifest_path)
        == (attempt / "frozen-manifest.sha256").read_text().strip()
    ), "manifest identity changed"
    manifest = json.loads(manifest_path.read_text())
    assert manifest["status"] == "frozen" and manifest["mode"] == "ordinary", (
        "reference set is not frozen ordinary output"
    )
    assert len(manifest["cells"]) == 6 and all(
        cell["status"] == "validated" for cell in manifest["cells"]
    ), "incomplete frozen corpus"
    assert inventory(Path(manifest["snapshot_root"])) == manifest["snapshot_sha256"], (
        "ordinary snapshot identity changed"
    )
    assert inventory(Path(manifest["corpus_root"])) == manifest["corpus_sha256"], (
        "frozen corpus identity changed"
    )
    expected_historical = json.loads(
        (attempt / "historical-corpus-hashes.json").read_text()
    )
    assert inventory(ROOT / "tmp/rtree_query_corpus") == expected_historical, (
        "historical corpus identity changed"
    )
    sources = json.loads((attempt / "source-inputs.json").read_text())
    assert sources and all(
        digest(ROOT / name) == expected for name, expected in sources.items()
    ), "build source identity changed"
    preparation_script = attempt.parent / "prepare.py"
    assert digest(preparation_script) == manifest["preparation_script_sha256"], (
        "preparation script identity changed"
    )
    assert runpy.run_path(str(preparation_script))["source_inputs"]() == sources, (
        "build input set changed"
    )
    evidence = {
        name: digest(attempt / name)
        for name in (
            "frozen-manifest.json",
            "source-inputs.json",
            "historical-corpus-hashes.json",
            "source-worktree.diff",
            "source-worktree-status.txt",
            "ordinary-dist-build.log",
            "snapshot-ordinary-freshness.log",
            "snapshot-toolchain.log",
        )
    }
    evidence["../preflight-results.json"] = digest(
        attempt.parent / "preflight-results.json"
    )
    summary = {
        "verified_utc": datetime.now(timezone.utc).isoformat(),
        "status": "PASS",
        "evidence_sha256": evidence,
        "corpus_root": manifest["corpus_root"],
        "ordinary_snapshot_root": manifest["snapshot_root"],
        "claims": "identity and targeted preparation checks only; no adoption acceptance or full geometry oracle",
        "cells": [
            {
                "workload": cell["workload"],
                "generator": cell["generator"],
                "completion": cell["completion_evidence"],
                "sparse_segments": cell["output_checks"]["sparse_segments"],
                "worst_out_of_deposited_wall_bbox_mm": cell["output_checks"][
                    "worst_out_of_deposited_wall_bbox_mm"
                ],
                "effective_job": {
                    key: cell["output_checks"]["appendix"][key]
                    for key in (
                        "layer_height",
                        "first_layer_height",
                        "nozzle_diameter",
                        "line_width",
                        "wall_loops",
                        "infill_density",
                        "sparse_fill_holder",
                        "sparse_infill_density",
                        "enable_support",
                        "support_type",
                    )
                },
            }
            for cell in manifest["cells"]
        ],
    }
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
