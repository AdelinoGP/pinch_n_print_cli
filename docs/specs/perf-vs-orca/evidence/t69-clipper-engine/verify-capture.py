#!/usr/bin/env python3
"""Checkout-only T69 corpus/identity-evidence verifier; optional actual raw output check."""

import argparse
import collections
import hashlib
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
SITES = {"A0", "A1", "B", "C", "D", "E", "F", "G", "H", "J", "K", "RP1", "RP2"}


def load(path):
    return json.loads(path.read_text(encoding="utf-8-sig"))


def parse(path):
    tokens = iter(
        " ".join(
            line.partition("#")[0]
            for line in path.read_text(encoding="ascii").splitlines()
        ).split()
    )
    counts = collections.Counter()
    while (token := next(tokens, None)) is not None:
        assert token == "case", (path, token)
        site, layer, opcode, index = (
            next(tokens),
            int(next(tokens)),
            int(next(tokens)),
            int(next(tokens)),
        )
        assert site in SITES and layer >= 0 and index >= 0 and opcode in range(5)
        args = {}
        token = next(tokens)
        while token == "arg":
            key, value = int(next(tokens)), int(next(tokens))
            assert key not in args and 0 <= value < 2**64
            args[key] = value
            token = next(tokens)
        assert set(args) == ({1, 2, 3, 4, 5} if opcode == 0 else {10})
        if opcode == 0:
            assert args[2] in range(4) and args[3] in range(5)
        else:
            assert args[10] == 1
        for name in ["subj"] if opcode == 0 else ["subj", "clip"]:
            assert token == name
            paths = int(next(tokens))
            assert paths >= 0
            for _ in range(paths):
                rings = int(next(tokens))
                assert rings == 1, "already-flat engine path framing must remain exact"
                for _ in range(rings):
                    points = int(next(tokens))
                    assert points >= 0
                    for _ in range(points * 2):
                        assert -(2**63) <= int(next(tokens)) < 2**63
            token = next(tokens)
        assert token == "end"
        counts[site] += 1
    return counts


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--raw", type=Path)
    opts = parser.parse_args()
    corpus = HERE / "corpus/t68sites"
    summary = load(corpus / "summary.json")
    assert set(summary["jobs"]) == {"benchy", "base"}
    assert set(summary["files"]) == {
        f"{model}-{suffix}.corpus"
        for model in ("benchy", "base")
        for suffix in ("t68sites", "region-prep")
    }
    refs = {
        c["cell"]: c
        for c in load(
            REPO
            / "docs/specs/perf-vs-orca/evidence/t58-finalization-copies/qualification.json"
        )["cells"]
    }
    actual_counts = collections.defaultdict(collections.Counter)
    for name, item in summary["files"].items():
        path = corpus / name
        assert path.stat().st_size == item["bytes"]
        assert hashlib.sha256(path.read_bytes()).hexdigest() == item["sha256"]
        counts = parse(path)
        assert sum(counts.values()) == item["cases"]
        actual_counts[name.split("-")[0]].update(counts)
        if opts.raw:
            model = name.split("-")[0]
            reconstructed = []
            for source in item["raw_sources"]:
                raw_path = opts.raw / f"{model}-classic-off" / source["filename"]
                raw_bytes = raw_path.read_bytes()
                assert hashlib.sha256(raw_bytes).hexdigest() == source["sha256"]
                header, body = raw_bytes.decode("ascii").split("\n", 1)
                fields = header.split()
                fields[1] = source["site"]
                reconstructed.append(" ".join(fields) + "\n" + body)
            assert "".join(reconstructed).encode("ascii") == path.read_bytes(), (
                name,
                "raw/corpus geometry or parameter drift",
            )
    for model, job in summary["jobs"].items():
        assert dict(actual_counts[model]) == job["sampled_engine_counts"]
        classic = SITES - {"RP1", "RP2"}
        assert all(actual_counts[model][site] > 0 for site in classic)
        assert (
            sum(job["raw_service_or_rp_engine_counts"][site] for site in classic)
            == {"benchy": 2640, "base": 5442}[model]
        )
        assert all(
            job["raw_engine_counts"][site]
            <= job["raw_service_or_rp_engine_counts"][site]
            for site in classic
        )
        meta = job["metadata"]
        reference = refs[f"ordinary-{model}-classic-off"]
        assert meta["sha256"] == meta["reference_sha256"] == reference["sha256"]
        assert meta["bytes"] == reference["bytes"]
        assert (
            meta["byte_identical"] and meta["exit_code"] == 0 and meta["status"] == "ok"
        )
        assert meta["fatal"] == meta["non_fatal"] == 0 and meta["degraded"] is False
        if opts.raw:
            assert (
                hashlib.sha256(
                    (opts.raw / f"{model}-classic-off.gcode").read_bytes()
                ).hexdigest()
                == reference["sha256"]
            )
        print(
            model,
            "PASS",
            "cases",
            sum(actual_counts[model].values()),
            "reference",
            reference["sha256"],
        )
    total = sum(p.stat().st_size for p in corpus.iterdir())
    assert total <= summary["corpus_ceiling_bytes"]
    print("t69 capture verify: PASS; total corpus bytes", total)


if __name__ == "__main__":
    main()
