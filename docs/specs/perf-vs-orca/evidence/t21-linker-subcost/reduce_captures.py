"""Regenerate the in-repo reduced T21 captures and the output-hash manifest.

Reduction rule: keep `T21-PROBE` lines and JSON `profile_summary` events, and
`module_complete` events for `com.core.infill-linker`; drop everything else.
The full captures stay in the durable, gitignored raw tree; this script is the
only bridge, so the reduced set is reproducible rather than hand-edited.

Usage: python reduce_captures.py [--raw-root <dir>] [--out <dir>]
"""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
DEFAULT_RAW = (
    HERE.parents[4]
    / ".local-artifacts"
    / "perimeter-reference-preparation"
    / "t21-linker-subcost-run1"
)
MODULE_ID = "com.core.infill-linker"

SOURCES = {
    "ordinary/profile.reduced.log": "ordinary/profile.jsonl",
    "ordinary/probe-profile.reduced.log": "ordinary/probe-profile.jsonl",
    "ordinary/probe2-profile.reduced.log": "ordinary/probe2-profile.jsonl",
    "accelerated/probe2-profile.reduced.log": "accelerated/probe2-profile.jsonl",
}

OUTPUTS = {
    "ordinary": "ordinary/output.gcode",
    "ordinary-probe": "ordinary/probe-output.gcode",
    "ordinary-probe2": "ordinary/probe2-output.gcode",
    "accelerated-probe2": "accelerated/probe2-output.gcode",
}

REFERENCE = "t41-20260930T232255Z/corpus/supports-off-benchy/reference-arachne.gcode"


def keep(line: str) -> bool:
    if line.startswith("T21-PROBE"):
        return True
    if not line.startswith("{"):
        return False
    try:
        event = json.loads(line)
    except json.JSONDecodeError:
        return False
    if event.get("event") == "profile_summary":
        return True
    if event.get("event") == "module_log" and event.get("message", "").startswith(
        "T21-PROBE"
    ):
        return True
    return (
        event.get("event") == "module_complete" and event.get("module_id") == MODULE_ID
    )


def digest(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def write_text(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8", newline="") as stream:
        stream.write(text)


def main(raw_root: Path, out: Path) -> None:
    for reduced, source in SOURCES.items():
        kept_lines = []
        with (raw_root / source).open(encoding="utf-8", errors="replace") as stream:
            for line in stream:
                if keep(line.strip()):
                    kept_lines.append(line)
        write_text(out / reduced, "".join(kept_lines))
        print(f"{reduced}: {len(kept_lines)} lines kept")

    reference_sha = digest(raw_root.parent / REFERENCE)
    manifest = {
        "kind": "output-sha256-manifest",
        "note": (
            "Every measured output is byte-identical to the frozen Arachne "
            "reference; regenerate with reduce_captures.py."
        ),
        "frozen_reference": {"path": REFERENCE, "sha256": reference_sha},
        "captures": [
            {"capture": name, "output": output, "sha256": digest(raw_root / output)}
            for name, output in sorted(OUTPUTS.items())
        ],
    }
    write_text(out / "output-hashes.json", json.dumps(manifest, indent=2) + "\n")
    print(f"output-hashes.json: reference {reference_sha[:16]}...")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--raw-root", type=Path, default=DEFAULT_RAW)
    parser.add_argument("--out", type=Path, default=HERE / "captures")
    args = parser.parse_args()
    main(args.raw_root, args.out)
