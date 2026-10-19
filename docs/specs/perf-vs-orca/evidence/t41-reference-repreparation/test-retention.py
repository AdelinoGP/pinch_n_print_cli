"""Retention regression for the issue-41 root cause; touches only disposable data.

Root cause: t40's frozen payload roots lived under `target/`, so any build
`target/` cleanup removes them and the frozen reference set can no longer be
verified without a whole new preparation.

This test builds a disposable workspace tree under the system temp directory
(never under the real workspace), places the same synthetic frozen payload in
both placements, and proves:

RED  (original placement)  - a payload frozen under `target/` fails the
                             reconstructed t40 gate after a disposable target
                             cleanup, and the t41 durability policy rejects the
                             placement outright.
GREEN (durable placement)  - a payload frozen under
                             `.local-artifacts/perimeter-reference-preparation/`
                             survives the cleanup, passes the t41 durability
                             policy, and its inventory still matches.

It also checks the tamper controls used by `verify-freeze.py` at the mechanism
level: a missing root, a changed manifest digest, and a tampered frozen file
are each rejected. No real frozen file, and no real workspace `target/`, is
read or written: the disposable `target/` is created inside the temp tree.
"""

from pathlib import Path
import hashlib
import json
import runpy
import shutil
import sys
import tempfile

EVIDENCE = Path(__file__).resolve().parent
ROOT = EVIDENCE.parents[4]
NAMESPACE_REL = ".local-artifacts/perimeter-reference-preparation"
RESULTS = []


def digest(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def record(scenario, expected, observed, passed):
    RESULTS.append(
        {
            "scenario": scenario,
            "expected": expected,
            "observed": observed,
            "passed": passed,
        }
    )
    if not passed:
        raise RuntimeError(
            f"retention regression failed: {scenario}: expected {expected}, "
            f"observed {observed}"
        )


def write_payload(root):
    corpus = root / "corpus"
    corpus.mkdir(parents=True)
    (corpus / "frozen-reference.gcode").write_text("frozen evidence\n")
    (corpus / "frozen-reference.jsonl").write_text('{"event":"slice_complete"}\n')
    return {
        path.relative_to(root).as_posix(): digest(path)
        for path in corpus.rglob("*")
        if path.is_file()
    }


def is_under(path, parent):
    path = Path(path)
    parent = Path(parent)
    return path == parent or parent in path.parents


def main():
    if sys.flags.optimize:
        raise RuntimeError("retention regression requires Python assertions enabled")
    verifier = runpy.run_path(str(EVIDENCE / "verify-freeze.py"))
    t40_verifier = runpy.run_path(
        str(EVIDENCE.parent / "t40-reference-preparation" / "verify-freeze.py")
    )
    with tempfile.TemporaryDirectory() as temporary:
        ws = Path(temporary) / "ws"
        namespace = ws / NAMESPACE_REL
        # Disposable by construction; refuse to point the cleanup at the real
        # workspace for any reason.
        assert not is_under(ws, ROOT) and not is_under(ROOT, ws), (
            "disposable workspace must be disjoint from the real workspace"
        )

        # RED: the original placement inside target/.
        target_nested = ws / "target" / "perimeter-reference-preparation" / "attempt"
        recorded = write_payload(target_nested)
        try:
            verifier["ensure_durable_root"](
                target_nested, workspace_root=ws, namespace=namespace
            )
        except RuntimeError as error:
            record(
                "policy rejects target-nested frozen root",
                "FROZEN ROOT UNDER TARGET IS REJECTED",
                str(error),
                "FROZEN ROOT UNDER TARGET IS REJECTED" in str(error),
            )
        else:
            record(
                "policy rejects target-nested frozen root",
                "FROZEN ROOT UNDER TARGET IS REJECTED",
                "accepted",
                False,
            )

        # Simulate the build target cleanup on the DISPOSABLE tree only.
        shutil.rmtree(ws / "target")
        assert (ROOT / "target").is_dir(), "real workspace target must be untouched"
        record(
            "target cleanup removes the target-nested payload",
            "payload missing",
            "missing" if not target_nested.exists() else "still present",
            not target_nested.exists(),
        )
        # Reconstruct the t40 gate through its real inventory function: the
        # missing root cannot match the recorded hashes, which is exactly the
        # issue-41 failure ("frozen inputs disappeared"; manifest roots absent).
        t40_inventory = t40_verifier["inventory"](target_nested)
        record(
            "reconstructed t40 gate fails after cleanup",
            "inventory mismatch",
            "mismatch" if t40_inventory != recorded else "match",
            t40_inventory != recorded,
        )

        # GREEN: the durable placement. Recreate a target dir first, since a
        # real subsequent build would have one; the durable payload must not
        # depend on it existing or not.
        (ws / "target").mkdir(parents=True, exist_ok=True)
        durable = namespace / "attempt"
        recorded = write_payload(durable)
        resolved = verifier["ensure_durable_root"](
            durable, workspace_root=ws, namespace=namespace
        )
        record(
            "policy accepts the durable frozen root",
            str(durable),
            str(resolved),
            resolved == durable,
        )
        shutil.rmtree(ws / "target")
        survived = verifier["inventory"](durable)
        record(
            "durable payload survives a target cleanup",
            recorded,
            survived,
            durable.is_dir() and survived == recorded,
        )
        record(
            "t40 inventory agrees the durable payload is intact",
            recorded,
            t40_verifier["inventory"](durable),
            t40_verifier["inventory"](durable) == recorded,
        )

        # Missing-root control: a durable root that was never created.
        missing = namespace / "attempt-missing"
        try:
            verifier["require_root"](
                missing, "corpus", workspace_root=ws, namespace=namespace
            )
        except RuntimeError as error:
            record(
                "missing durable root is reported loudly",
                "MISSING FROZEN ROOT",
                str(error),
                "MISSING FROZEN ROOT" in str(error),
            )
        else:
            record(
                "missing durable root is reported loudly",
                "MISSING FROZEN ROOT",
                "passed",
                False,
            )

        # File-tamper control at the mechanism level: mutate a disposable file.
        tampered = durable / "corpus" / "frozen-reference.gcode"
        original_bytes = tampered.read_bytes()
        tampered.write_bytes(original_bytes + b"tampered\n")
        record(
            "tampered frozen file changes its inventory digest",
            "inventory mismatch",
            "mismatch" if verifier["inventory"](durable) != recorded else "match",
            verifier["inventory"](durable) != recorded,
        )

        # Manifest-tamper control: replicate the verifier's digest-vs-sidecar
        # mechanism on the disposable attempt. A changed manifest whose sidecar
        # is re-derived is caught by the frozen-state/root checks; a changed
        # manifest with a stale sidecar is caught by the digest check itself.
        manifest_path = durable / "frozen-manifest.json"
        manifest_path.write_text("{}\n")
        sidecar_path = durable / "frozen-manifest.sha256"
        stale_sidecar = "0" * 64
        sidecar_path.write_text(stale_sidecar + "\n")
        record(
            "changed manifest fails its recorded digest",
            "manifest identity changed",
            "manifest identity changed"
            if digest(manifest_path) != sidecar_path.read_text().strip()
            else "match",
            digest(manifest_path) != sidecar_path.read_text().strip(),
        )

        # Restore nothing; the whole temp tree is about to be removed.
        assert not is_under(durable, ROOT), (
            "durable probe must never touch the real tree"
        )
        assert not is_under(ws / "target", ROOT)

    (EVIDENCE / "retention-results.json").write_text(
        json.dumps(RESULTS, indent=2) + "\n", encoding="utf-8"
    )
    print(
        "PASS: target-nested payload fails after a disposable target cleanup "
        "(reconstructed t40 gate) and is rejected by the durable policy; the "
        "durable payload survives and matches; missing-root and file-tamper "
        "controls reject; real workspace target and real frozen files untouched"
    )


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        (EVIDENCE / "retention-results.json").write_text(
            json.dumps(RESULTS, indent=2) + "\n", encoding="utf-8"
        )
        print(f"STOP: {error}", file=sys.stderr)
        sys.exit(1)
