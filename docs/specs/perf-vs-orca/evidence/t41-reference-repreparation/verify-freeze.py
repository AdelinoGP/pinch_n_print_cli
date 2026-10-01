"""Read-only t41 reference/snapshot/source identity gate; never generates output.

Usage: python verify-freeze.py <attempt-directory>
Run before any accelerated comparison and again after the campaign. A changed
or missing input is a blocker requiring an explicit new preparation decision;
this gate never repairs and never re-freezes a modified input. Printed JSON can
be saved as a freeze attestation.

Stronger than the t40 gate in three ways the t40 failure justifies:
* a missing frozen root is reported as `MISSING FROZEN ROOT: <path>`, not as a
  generic inventory mismatch;
* frozen roots must be inside the durable
  `.local-artifacts/perimeter-reference-preparation/` namespace and are
  rejected outright if they sit under `target/` (the root cause of issue 41);
* every recorded tool/exercise script hash, the shared validator, the policy
  source, the preflight gates and each cell's input/output/sidecar hashes are
  re-derived, and the sparse-wall-bbox check is recomputed per cell.
"""

from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import runpy
import sys

ROOT = Path(__file__).resolve().parents[5]
DURABLE_NAMESPACE = ROOT / ".local-artifacts" / "perimeter-reference-preparation"
TOOL_FILES = (
    "prepare.py",
    "preflight.py",
    "verify-freeze.py",
    "validate-reference.ps1",
    "test-preparation.py",
    "test-freeze.py",
    "test-retention.py",
)


def digest(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def inventory(directory):
    return {
        path.relative_to(directory).as_posix(): digest(path)
        for path in sorted(Path(directory).rglob("*"))
        if path.is_file()
    }


def is_under(path, parent):
    path = Path(path)
    parent = Path(parent)
    return path == parent or parent in path.parents


def ensure_durable_root(path, *, workspace_root=None, namespace=None):
    """Reject a frozen root that is not in the durable payload namespace.

    The issue-41 root cause was a frozen root under `target/`, which any build
    `target/` cleanup removes. Such a root is rejected here loudly; the
    retention regression (`test-retention.py`) proves the cleanup behaviour.
    `workspace_root`/`namespace` exist so the regression can exercise this
    exact policy against a disposable tree without touching the real one.
    """
    resolved = Path(path)
    target = Path(workspace_root if workspace_root is not None else ROOT) / "target"
    namespace = Path(namespace if namespace is not None else DURABLE_NAMESPACE)
    if is_under(resolved, target):
        raise RuntimeError(
            f"FROZEN ROOT UNDER TARGET IS REJECTED: {resolved}; frozen payloads "
            f"must survive a build target cleanup (issue 41)"
        )
    if not is_under(resolved, namespace) or resolved == namespace:
        raise RuntimeError(
            f"FROZEN ROOT IS NOT IN THE DURABLE NAMESPACE: {resolved}; expected a "
            f"subdirectory of {namespace}"
        )
    return resolved


def require_root(path, label, *, workspace_root=None, namespace=None):
    resolved = ensure_durable_root(
        path, workspace_root=workspace_root, namespace=namespace
    )
    if not resolved.is_dir():
        raise RuntimeError(
            f"MISSING FROZEN ROOT: {label} root {resolved} does not exist"
        )
    return resolved


def require_completion(cell):
    completion = cell["completion_evidence"]
    if (
        completion["ValidatedGenerator"] != cell["generator"]
        or completion["CompletionStatus"] != "ok"
        or completion["Degraded"]
        or completion["FatalErrorCount"] != 0
        or completion["NonFatalErrorCount"] != 0
    ):
        raise RuntimeError(
            f"reference completion is not clean: {cell['workload']}/"
            f"{cell['generator']}: {completion}"
        )


def main():
    if sys.flags.optimize:
        raise RuntimeError("freeze verification requires Python assertions enabled")
    if len(sys.argv) != 2:
        raise RuntimeError("usage: python verify-freeze.py <attempt-directory>")
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
    assert manifest.get("threads") == 12, "reference set did not use 12 threads"
    assert len(manifest["cells"]) == 6 and all(
        cell["status"] == "validated" for cell in manifest["cells"]
    ), "incomplete frozen corpus"
    assert manifest["durable_namespace"] == str(DURABLE_NAMESPACE), (
        "manifest claims a different durable namespace"
    )

    snapshot_root = require_root(manifest["snapshot_root"], "ordinary snapshot")
    corpus_root = require_root(manifest["corpus_root"], "frozen corpus")

    assert inventory(snapshot_root) == manifest["snapshot_sha256"], (
        "ordinary snapshot identity changed"
    )
    assert inventory(corpus_root) == manifest["corpus_sha256"], (
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
    tools = {name: digest(attempt.parent / name) for name in TOOL_FILES}
    assert tools == manifest["tools"], "t41 tool identity changed"
    preparation = runpy.run_path(str(attempt.parent / "prepare.py"))
    assert preparation["source_inputs"]() == sources, "build input set changed"

    assert (
        digest(ROOT / "resources/perimeter-acceptance/validate_measurement.ps1")
        == manifest["shared_validator_sha256"]
    ), "shared validator identity changed"
    assert (
        digest(ROOT / "xtask/src/rustc_driver.rs") == manifest["policy_source_sha256"]
    ), "controlled-build policy source changed"
    assert (
        digest(ROOT / "docs/23_controlled_perimeter_builds.md")
        == manifest["policy_doc_sha256"]
    ), "controlled-build policy doc changed"
    assert (
        digest(attempt.parent / "preflight-results.json")
        == manifest["preflight_gates_sha256"]
    ), "preflight gates identity changed"

    gates = json.loads((attempt.parent / "preflight-results.json").read_text())
    assert gates.get("status") == "passed", "preflight did not pass"
    assert gates["checks"]["freshness"]["final_exit"] == 0, (
        "preflight freshness did not reach exit 0"
    )
    assert gates["checks"]["target_inspection"]["ok"] is True, (
        "preflight target inspection did not pass"
    )
    test_gates = {
        gate["name"]: gate
        for gate in gates["gates"]
        if gate["name"]
        in ("support-repair", "empty-infill-producers", "empty-infill-runtime")
    }
    assert len(test_gates) == 3, "preflight is missing a required test gate"
    for gate in test_gates.values():
        counts = gate["passed_counts"]
        assert (
            gate["exit_code"] == 0
            and not gate["missing_successful_tests"]
            and counts["failed"] == 0
            and counts["passed"] == gate["expected_passed_exact"]
        ), f"preflight test gate is not clean: {gate['name']}: {counts}"

    cells_summary = []
    for cell in manifest["cells"]:
        workload = cell["workload"]
        generator = cell["generator"]
        cell_dir = corpus_root / workload
        recorded = cell["files_sha256"]
        for name, expected in recorded.items():
            actual_path = cell_dir / name
            if not actual_path.is_file():
                raise RuntimeError(f"MISSING FROZEN CELL FILE: {actual_path}")
            if digest(actual_path) != expected:
                raise RuntimeError(f"reference file identity changed: {actual_path}")
        require_completion(cell)
        analysis = preparation["check_output"](
            cell_dir / f"reference-{generator}.gcode"
        )
        if (
            analysis["sparse_segments"] != cell["output_checks"]["sparse_segments"]
            or analysis["worst_out_of_deposited_wall_bbox_mm"]
            != cell["output_checks"]["worst_out_of_deposited_wall_bbox_mm"]
        ):
            raise RuntimeError(
                f"recomputed sparse-bbox check differs: {workload}/{generator}"
            )
        if analysis["worst_out_of_deposited_wall_bbox_mm"] != 0.0:
            raise RuntimeError(
                f"recomputed sparse overshoot for {workload}/{generator}"
            )
        if cell["config"]["enable_support"] != cell.get("support_type_evidence"):
            raise RuntimeError(
                f"support TYPE evidence flag is inconsistent: {workload}/{generator}"
            )
        if cell["config"]["enable_support"] and not any(
            name.lower().startswith("support") for name in analysis["type_counts"]
        ):
            raise RuntimeError(
                f"support-enabled reference lacks support TYPE evidence: "
                f"{workload}/{generator}"
            )
        cells_summary.append(
            {
                "workload": workload,
                "generator": generator,
                "completion": cell["completion_evidence"],
                "sparse_segments": analysis["sparse_segments"],
                "worst_out_of_deposited_wall_bbox_mm": analysis[
                    "worst_out_of_deposited_wall_bbox_mm"
                ],
                "effective_job": {
                    key: analysis["appendix"][key]
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
        "tools_sha256": tools,
        "durable_namespace": str(DURABLE_NAMESPACE),
        "corpus_root": str(corpus_root),
        "ordinary_snapshot_root": str(snapshot_root),
        "claims": (
            "identity and targeted preparation checks only; no adoption "
            "acceptance or full geometry oracle"
        ),
        "cells": cells_summary,
    }
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
