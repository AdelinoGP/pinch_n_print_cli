"""Negative controls for the read-only t41 freeze gate; never changes frozen files.

Usage: python test-freeze.py [attempt-directory]
Default attempt: attempt-1 (the single authorized t41 attempt).

Controls, all read-only against the real frozen set:
* changed manifest (a temp copy with one appended byte) is rejected by the
  manifest-digest check before anything else runs;
* disabled Python assertions are rejected;
* a missing durable root is rejected loudly (`MISSING FROZEN ROOT`);
* a tampered copy of a real frozen file changes its inventory digest, so the
  real gate's inventory comparison would reject it;
* every real frozen file hash is re-derived before and after and must be
  unchanged by these controls.
"""

from pathlib import Path
import hashlib
import json
import runpy
import shutil
import subprocess
import sys
import tempfile

EVIDENCE = Path(__file__).resolve().parent
ROOT = EVIDENCE.parents[4]
VERIFIER = EVIDENCE / "verify-freeze.py"
attempt = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else EVIDENCE / "attempt-1"
results = []


def digest(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def frozen_inventory():
    manifest = json.loads((attempt / "frozen-manifest.json").read_text())
    return {
        root: {
            path.relative_to(root).as_posix(): digest(path)
            for path in sorted(Path(root).rglob("*"))
            if path.is_file()
        }
        for root in (manifest["snapshot_root"], manifest["corpus_root"])
    }


def record(control, command, result, failure, *, expect_stderr=None):
    log = result.stdout + result.stderr
    (EVIDENCE / f"freeze-control-{control}.log").write_text(log, encoding="utf-8")
    ok = result.returncode != 0 and (
        (failure in result.stderr) if expect_stderr is None else (expect_stderr in log)
    )
    if not ok:
        raise RuntimeError(f"freeze negative control failed: {control}")
    results.append(
        {
            "control": control,
            "command": command,
            "exit_code": result.returncode,
            "rejection_verified": True,
        }
    )


def main():
    before = frozen_inventory()
    verifier = runpy.run_path(str(VERIFIER))
    manifest = json.loads((attempt / "frozen-manifest.json").read_text())

    with tempfile.TemporaryDirectory() as temporary:
        corrupted = Path(temporary)
        original = attempt / "frozen-manifest.json"
        (corrupted / original.name).write_bytes(original.read_bytes() + b" ")
        (corrupted / "frozen-manifest.sha256").write_bytes(
            (attempt / "frozen-manifest.sha256").read_bytes()
        )
        command = [sys.executable, str(VERIFIER), str(corrupted)]
        record(
            "changed-manifest",
            command,
            subprocess.run(command, capture_output=True, text=True),
            "manifest identity changed",
        )

        command = [sys.executable, "-O", str(VERIFIER), str(attempt)]
        record(
            "disabled-assertions",
            command,
            subprocess.run(command, capture_output=True, text=True),
            "assertions enabled",
        )

        # Missing durable root: real namespace, a name that was never frozen.
        try:
            verifier["require_root"](
                Path(manifest["durable_namespace"]) / "never-frozen", "corpus"
            )
        except RuntimeError as error:
            if "MISSING FROZEN ROOT" not in str(error):
                raise
            results.append(
                {
                    "control": "missing-durable-root",
                    "command": "require_root(never-frozen)",
                    "rejection": str(error).splitlines()[0],
                    "rejection_verified": True,
                }
            )
        else:
            raise RuntimeError("freeze negative control failed: missing-durable-root")

        # Tampered copy of a real frozen file changes its inventory digest.
        recorded = before[manifest["corpus_root"]]
        relative = next(iter(recorded))
        tampered = corrupted / "tampered.bin"
        shutil.copy2(Path(manifest["corpus_root"]) / relative, tampered)
        tampered.write_bytes(tampered.read_bytes() + b"tamper")
        if digest(tampered) == recorded[relative]:
            raise RuntimeError("tamper control failed to change the frozen digest")
        results.append(
            {
                "control": "tampered-frozen-file-copy",
                "command": f"digest({relative})",
                "rejection": "inventory digest differs",
                "rejection_verified": True,
            }
        )

    after = frozen_inventory()
    if after != before:
        raise RuntimeError("negative controls modified the real frozen files")
    results.append(
        {
            "control": "frozen-files-unchanged",
            "command": "inventory before/after controls",
            "rejection_verified": True,
        }
    )
    (EVIDENCE / "freeze-control-results.json").write_text(
        json.dumps(results, indent=2) + "\n", encoding="utf-8"
    )
    print(
        "PASS: changed manifest, disabled assertions and a missing durable root "
        "are rejected; a tampered frozen-file copy changes its digest; real "
        "frozen files are byte-identical before and after"
    )


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        (EVIDENCE / "freeze-control-results.json").write_text(
            json.dumps(results, indent=2) + "\n", encoding="utf-8"
        )
        print(f"STOP: {error}", file=sys.stderr)
        sys.exit(1)
