"""Inventory existing acceptance inputs without generating any reference output."""

import datetime
import hashlib
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[5]
CORPUS = ROOT / "tmp" / "rtree_query_corpus"


def sha256(path):
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


cells = []
for workload in ("supports-off-benchy", "tree-support-benchy", "tree-support-base"):
    directory = CORPUS / workload
    model = directory / "model.stl"
    source_model = (
        ROOT
        / "tmp"
        / ("base.stl" if workload == "tree-support-base" else "3dbenchy.stl")
    )
    model_digest = sha256(model)
    for generator in ("classic", "arachne"):
        config_path = directory / f"{generator}.json"
        reference = directory / f"reference-{generator}.gcode"
        events_path = directory / f"reference-{generator}.jsonl"
        config = json.loads(config_path.read_text(encoding="utf-8-sig"))
        markers = []
        with reference.open(encoding="utf-8-sig") as stream:
            for line in stream:
                if line.startswith(
                    (
                        "; wall_generator",
                        "; enable_support",
                        "; nozzle_diameter",
                        "; wall_loops",
                        "; sparse_infill_density",
                    )
                ):
                    markers.append(line.strip())
        completion = []
        with events_path.open(encoding="utf-8-sig") as stream:
            for line in stream:
                if line.startswith("{"):
                    event = json.loads(line)
                    if event.get("event") == "slice_complete":
                        completion.append(event)
        cells.append(
            {
                "workload": workload,
                "generator": generator,
                "model_sha256": model_digest,
                "model_matches_map_fixture": model_digest == sha256(source_model),
                "config_sha256": sha256(config_path),
                "config": config,
                "reference_sha256": sha256(reference),
                "reference_markers": markers,
                "reference_completion": completion,
            }
        )
print(
    json.dumps(
        {
            "captured_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
            "purpose": "Input inventory only; not fresh exactness or performance acceptance",
            "cells": cells,
        },
        indent=2,
    )
)
