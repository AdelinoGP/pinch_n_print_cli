#!/usr/bin/env python3
"""t95 extraction: raw capture trees -> committed reduced rows.

    python extract.py --raw

Reads `.local-artifacts/perimeter-reference-preparation/t95-exact-z-batch/`
(proof and measure `rows.csv` per cell, plus the two provenance JSONs) and
writes the committed reduced set next to this script:

    proof-rows.json    16 cells x 2 arms
    measure-rows.json   4 cells x 14 rows (2 warmups + 6 pairs)
    provenance/         the four arm/mode snapshots' module + exe fingerprints

`reduce.py` turns those into `qualification.json`, `ab-reduction.json` and
`summary.json`; `verify.py` re-derives them from the committed set alone.
"""

from __future__ import annotations

import json
import shutil

import reduce as t95


def dump_rows(name: str, rows: dict[str, list[dict]]) -> None:
    payload = {
        cell: [
            {key: row[key] for key in t95.ROW_FIELDS if key in row} for row in cell_rows
        ]
        for cell, cell_rows in rows.items()
    }
    (t95.HERE / name).write_text(json.dumps(payload, indent=1) + "\n", encoding="utf-8")


def main() -> int:
    proof = t95.load_raw("proof", t95.PROOF_CELLS)
    measure = t95.load_raw("measure", t95.MEASURE_CELLS)
    dump_rows("proof-rows.json", proof)
    dump_rows("measure-rows.json", measure)
    counts = ", ".join(f"{cell}={len(rows)}" for cell, rows in proof.items())
    print(f"proof: {counts}")
    counts = ", ".join(f"{cell}={len(rows)}" for cell, rows in measure.items())
    print(f"measure: {counts}")
    provenance_out = t95.HERE / "provenance"
    provenance_out.mkdir(exist_ok=True)
    for mode in t95.MODES:
        for arm in ("baseline", "candidate"):
            source = t95.ROOT / "provenance" / f"{mode}-{arm}.json"
            if not source.exists():
                raise AssertionError(f"missing provenance {source}")
            shutil.copyfile(source, provenance_out / source.name)
            print(f"provenance: {source.name}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
