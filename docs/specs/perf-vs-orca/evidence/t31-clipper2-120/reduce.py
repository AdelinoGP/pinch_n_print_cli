"""Reduce a full t31 corpus pair to the in-repo evidence set.

The raw capture tree lives under `tmp/` (gitignored): two full 369-file corpus
dumps, one per clipper2-rust arm. The map's evidence policy requires the
in-repo reduced set to be derivable from that raw tree by a script that lives
in-repo, and requires the verifier to exit 0 against the reduced set alone.

    python3 reduce.py <raw-110-dir> <raw-120-dir>

Writes, next to this script:
  manifest.json   SHA-256 of every corpus file on both arms
  out-110/, out-120/  the differing fixtures + the two summary files
"""

import hashlib
import json
import shutil
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent

# The summary files always ship so the verifier can check the counts/validations
SYMMARIES = ["_counts.json", "_validations.json"]


def hash_dir(d):
    return {
        p.name: hashlib.sha256(p.read_bytes()).hexdigest()
        for p in sorted(Path(d).glob("*.json"))
    }


def main():
    if len(sys.argv) != 3:
        print(__doc__)
        return 2
    raw = {"110": Path(sys.argv[1]), "120": Path(sys.argv[2])}

    man = {side: hash_dir(d) for side, d in raw.items()}
    if set(man["110"]) != set(man["120"]):
        print("FAIL: arms have different file sets")
        return 1
    differing = sorted(k for k in man["110"] if man["110"][k] != man["120"][k])
    print(f"{len(man['110'])} files/arm; differing: {differing}")

    (HERE / "manifest.json").write_text(
        json.dumps(man, indent=1, sort_keys=True) + "\n", encoding="utf8"
    )

    for side, d in raw.items():
        out = HERE / f"out-{side}"
        out.mkdir(exist_ok=True)
        for name in differing + SYMMARIES:
            src = d / name
            if not src.exists():
                print(f"FAIL: {src} missing")
                return 1
            shutil.copyfile(src, out / name)
    print(f"wrote manifest.json + out-110/ + out-120/ under {HERE}")

    # the reduced set must satisfy the shipped verifier
    import subprocess

    r = subprocess.run([sys.executable, str(HERE / "verify.py")])
    return r.returncode


if __name__ == "__main__":
    sys.exit(main())
