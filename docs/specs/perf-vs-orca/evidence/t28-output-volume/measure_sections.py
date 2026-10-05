"""Account raw G-code bytes by feature marker, without conflating section counts.

Usage: python docs/specs/perf-vs-orca/evidence/t28-output-volume/measure_sections.py FILE...
Prints JSON to stdout; FILE may be a PNP (;TYPE:) or Orca (; FEATURE:) G-code.
Motion counts only include G0/G1/G2/G3, reported in their current bucket.
Untyped headers, layer transitions, and trailing config blocks have their own
bucket.
"""

import json
import sys
from collections import defaultdict
from pathlib import Path


def measure(path):
    roles = defaultdict(
        lambda: {
            "bytes": 0,
            "sections": 0,
            "motions": 0,
            "xy_e_motions": 0,
            "xy_e_motion_bytes": 0,
            "xy_e_z_token_bytes": 0,
            "xy_e_f_token_bytes": 0,
        }
    )
    role = "[untyped]"
    with path.open("rb") as source:
        for line in source:
            if line.startswith(b";TYPE:"):
                role = line.removeprefix(b";TYPE:").strip().decode("utf-8", "replace")
                roles[role]["sections"] += 1
            elif line.startswith(b"; FEATURE: "):
                role = (
                    line.removeprefix(b"; FEATURE: ").strip().decode("utf-8", "replace")
                )
                roles[role]["sections"] += 1
            elif line.startswith(
                (b";LAYER_CHANGE", b"; CHANGE_LAYER", b"; CONFIG_BLOCK_START")
            ):
                role = "[untyped]"
            record = roles[role]
            record["bytes"] += len(line)
            command = line.split(b";", 1)[0].strip()
            fields = command.split()
            if fields and fields[0] in (b"G0", b"G1", b"G2", b"G3"):
                record["motions"] += 1
                # E may also occur without XY (retraction/priming); keep that
                # distinct from an XY extrusion segment when interpreting counts.
                if any(field.startswith(b"E") for field in fields[1:]) and any(
                    field.startswith((b"X", b"Y")) for field in fields[1:]
                ):
                    record["xy_e_motions"] += 1
                    record["xy_e_motion_bytes"] += len(line)
                    record["xy_e_z_token_bytes"] += sum(
                        len(field) + 1 for field in fields[1:] if field.startswith(b"Z")
                    )
                    record["xy_e_f_token_bytes"] += sum(
                        len(field) + 1 for field in fields[1:] if field.startswith(b"F")
                    )
    return {
        "file": str(path),
        "total_bytes": path.stat().st_size,
        "roles": dict(sorted(roles.items())),
    }


if __name__ == "__main__":
    if len(sys.argv) < 2:
        raise SystemExit(__doc__)
    print(json.dumps([measure(Path(name)) for name in sys.argv[1:]], indent=2))
