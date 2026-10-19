"""Negative controls for the read-only freeze gate; never changes frozen files."""

import json
from pathlib import Path
import subprocess
import sys
import tempfile

EVIDENCE = Path(__file__).resolve().parent
ROOT = EVIDENCE.parents[4]
attempt = EVIDENCE / "attempt-2"
verifier = EVIDENCE / "verify-freeze.py"
results = []
with tempfile.TemporaryDirectory(dir=ROOT / "target") as temporary:
    corrupted = Path(temporary)
    original = attempt / "frozen-manifest.json"
    (corrupted / original.name).write_bytes(original.read_bytes() + b" ")
    (corrupted / "frozen-manifest.sha256").write_bytes(
        (attempt / "frozen-manifest.sha256").read_bytes()
    )
    for name, command, failure in (
        (
            "changed-manifest",
            [sys.executable, str(verifier), str(corrupted)],
            "manifest identity changed",
        ),
        (
            "disabled-assertions",
            [sys.executable, "-O", str(verifier), str(attempt)],
            "assertions enabled",
        ),
    ):
        result = subprocess.run(command, capture_output=True, text=True)
        (EVIDENCE / f"freeze-control-{name}.log").write_text(
            result.stdout + result.stderr
        )
        if result.returncode == 0 or failure not in result.stderr:
            raise RuntimeError(f"freeze negative control failed: {name}")
        results.append(
            {
                "control": name,
                "exit_code": result.returncode,
                "rejection_verified": True,
            }
        )
(EVIDENCE / "freeze-control-results.json").write_text(
    json.dumps(results, indent=2) + "\n"
)
print(
    "PASS: changed manifest and disabled assertions are rejected; frozen files untouched"
)
