#!/usr/bin/env python3
"""t96 Phase-1 extractor.

Reads the probe captures from this machine's `.local-artifacts` root and writes
two in-repo reduced files next to this script:

- `probe-rows.json`   — per-call rows for the two probe runs and the
  uninstrumented window-arm metadata;
- `output-drift.json` — the deterministic output drift the windowed arm causes,
  reduced to the two differing Internal Bridge blocks and the first differing
  block index.

Attribution only: instrumented timings are never a wall claim.
"""

from __future__ import annotations

import json
import re
from pathlib import Path

HERE = Path(__file__).resolve().parent
CAPTURE = (
    HERE.parents[4]
    / ".local-artifacts"
    / "perimeter-reference-preparation"
    / "t96-windowed-clip-area"
    / "phase1"
    / "ordinary"
)

REFERENCE_SHA = "292df50179c4033d6204a0353e038534b4d3a98381126998bb5722a41a571a80"
REFERENCE_BYTES = 21023739

INT_FIELDS = {
    "layer",
    "surfaces",
    "deep_polys",
    "deep_vertices",
    "win_boxes",
    "win_operand_polys",
    "win_operand_vertices",
    "win_area_polys",
    "mismatch_polys",
    "mismatch_vertices",
}
FLOAT_FIELDS = {
    "print_z",
    "full_ms",
    "win_clip_ms",
    "win_offset_ms",
    "win_result_ms",
    "win_total_ms",
}


def parse_row(line: str) -> dict:
    row: dict = {}
    for key, value in re.findall(r"(\w+)=(\S+)", line):
        if key in INT_FIELDS:
            row[key] = int(value)
        elif key in FLOAT_FIELDS:
            row[key] = float(value)
    return row


def read_rows(stem: str) -> list[dict]:
    log = CAPTURE / f"{stem}.stderr.log"
    return [
        parse_row(line)
        for line in log.read_text(encoding="utf-8", errors="replace").splitlines()
        if line.startswith("T96LAYER ")
    ]


def read_run(stem: str, arm: str, window: bool) -> dict:
    run = json.loads((CAPTURE / f"{stem}.run.json").read_text(encoding="utf-8"))
    rows = read_rows(stem)
    full = sum(row["full_ms"] for row in rows)
    win = sum(row["win_total_ms"] for row in rows)
    return {
        "run": stem,
        "arm": arm,
        "window": window,
        "probe": True,
        "wall_seconds": run["wall_seconds"],
        "cpu_seconds": run["cpu_seconds"],
        "cpu_wall_ratio": run["cpu_wall_ratio"],
        "sha256": run["sha256"],
        "bytes": run["bytes"],
        "reference_sha256": run["reference_sha256"],
        "full_ms": full,
        "window_ms": win,
        "saving_ms": full - win,
        "wall_equivalent_saving_s": (full - win) / 1000.0 / run["cpu_wall_ratio"],
        "mismatch_rows": sum(1 for row in rows if row["mismatch_polys"] > 0),
        "rows": rows,
    }


def gcode_blocks(path: Path) -> list[dict]:
    lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    blocks: list[dict] = []
    z = ""
    index = 0
    i = 0
    while i < len(lines):
        line = lines[i]
        if line.startswith(";Z:"):
            z = line.strip()
        if line.startswith(";TYPE:"):
            kind = line.strip()
            j = i + 1
            moves: list[str] = []
            while (
                j < len(lines)
                and not lines[j].startswith(";TYPE:")
                and not lines[j].startswith(";LAYER_CHANGE")
            ):
                if lines[j].startswith("G1") and "X" in lines[j] and "Y" in lines[j]:
                    moves.append(lines[j].strip())
                j += 1
            blocks.append({"index": index, "type": kind, "z": z, "moves": moves})
            index += 1
            i = j
            continue
        i += 1
    return blocks


def main() -> int:
    probe = read_run("ordinary-probe-1", "probe", window=False)
    probe_window = read_run("ordinary-probe-window-2", "probe-window", window=True)
    window_meta = json.loads(
        (CAPTURE / "ordinary-window-1.run.json").read_text(encoding="utf-8")
    )
    payload = {
        "cell": "ordinary-base-classic-off",
        "cell_wall_seconds": 73.17,
        "floor_share": 0.008,
        "note": (
            "t96 Phase-1 screen. Per-call rows from PNP_T96_PROBE=1 on the "
            "matched job; accumulated over the layer axis and converted to a "
            "wall equivalent with each run's own external cpu/wall ratio. "
            "Instrumented: never a wall claim."
        ),
        "runs": [probe, probe_window],
        "window_arm": {
            "run": "ordinary-window-1",
            "window": True,
            "probe": False,
            "wall_seconds": window_meta["wall_seconds"],
            "cpu_seconds": window_meta["cpu_seconds"],
            "cpu_wall_ratio": window_meta["cpu_wall_ratio"],
            "sha256": window_meta["sha256"],
            "bytes": window_meta["bytes"],
        },
    }
    (HERE / "probe-rows.json").write_text(
        json.dumps(payload, indent=1) + "\n", encoding="utf-8"
    )

    reference = gcode_blocks(CAPTURE / "ordinary-probe-1.gcode")
    windowed = gcode_blocks(CAPTURE / "ordinary-window-1.gcode")
    if len(reference) != len(windowed):
        raise SystemExit("block counts differ; extractor assumption broken")
    differing = [
        (index, a, b) for index, (a, b) in enumerate(zip(reference, windowed)) if a != b
    ]
    reduced_blocks = [
        {
            "index": index,
            "type": a["type"],
            "z": a["z"],
            "moves_reference": len(a["moves"]),
            "moves_window": len(b["moves"]),
            "reference_first_moves": a["moves"][:4],
            "window_first_moves": b["moves"][:4],
        }
        for index, a, b in differing
    ]
    layer_row = next(row for row in probe["rows"] if abs(row["print_z"] - 21.4) < 1e-6)
    drift = {
        "cell": "ordinary-base-classic-off",
        "reference_sha256": REFERENCE_SHA,
        "reference_bytes": REFERENCE_BYTES,
        "window_sha256": window_meta["sha256"],
        "window_bytes": window_meta["bytes"],
        "differing_block_count": len(differing),
        "first_differing_block_index": reduced_blocks[0]["index"],
        "differing_blocks": reduced_blocks,
        "isolation": {
            "note": (
                "At the first differing Internal Bridge layer the probe's "
                "inside-the-window set check is clean, so the qualification and "
                "per-candidate area inputs are unchanged; the bridge direction "
                "is not."
            ),
            "layer": layer_row["layer"],
            "print_z": layer_row["print_z"],
            "mismatch_polys": layer_row["mismatch_polys"],
            "mismatch_vertices": layer_row["mismatch_vertices"],
            "win_area_polys": layer_row["win_area_polys"],
        },
    }
    (HERE / "output-drift.json").write_text(
        json.dumps(drift, indent=1) + "\n", encoding="utf-8"
    )

    for run in (probe, probe_window):
        share = run["wall_equivalent_saving_s"] / 73.17 * 100.0
        print(
            f"{run['run']}: rows={len(run['rows'])} full={run['full_ms']:.1f} ms "
            f"windowed={run['window_ms']:.1f} ms saving={run['saving_ms']:.1f} ms "
            f"({run['wall_equivalent_saving_s']:.3f} s wall-eq, {share:.2f} % of cell) "
            f"sha={run['sha256'][:12]}"
        )
    print(
        f"window arm: sha={window_meta['sha256'][:12]} bytes={window_meta['bytes']} "
        f"(reference {REFERENCE_SHA[:12]} / {REFERENCE_BYTES})"
    )
    print(
        f"differing blocks={len(differing)} first={reduced_blocks[0]['index']} "
        f"{reduced_blocks[0]['type']} z={reduced_blocks[0]['z']} "
        f"moves {reduced_blocks[0]['moves_reference']} -> {reduced_blocks[0]['moves_window']}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
