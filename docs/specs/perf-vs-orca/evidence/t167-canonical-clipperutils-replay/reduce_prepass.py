"""Reduce raw t167 prepass capture into deterministic in-repo corpus + census JSON."""

import collections
import hashlib
import json
from pathlib import Path
import statistics
import struct

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
RAW_DIR = REPO / ".local-artifacts/perf-vs-orca/t167-canonical-clipperutils-replay/raw-capture"
OUT_CORPUS = HERE / "corpus/base-prepass-t167.corpus"
OUT_CENSUS = HERE / "corpus/prepass_census.json"

TARGETS = {
    "IB_DEEP_POS": 24,
    "IB_DEEP_NEG": 16,
    "IB_CAND_POS": 16,
}


def u64_to_f64(bits: int) -> float:
    return struct.unpack("<d", struct.pack("<Q", bits))[0]


def parse_case_file(p: Path):
    text = p.read_text()
    lines = text.splitlines()
    head = lines[0].split()
    assert head[0] == "case"
    site = head[1]
    layer = int(head[2])
    opcode = int(head[3])
    args = {}
    i = 1
    while i < len(lines) and lines[i].startswith("arg "):
        parts = lines[i].split()
        args[int(parts[1])] = int(parts[2])
        i += 1
    nverts = 0
    while i < len(lines):
        ln = lines[i]
        if ln == "end":
            break
        if ln.startswith("subj ") or ln.startswith("clip "):
            npolys = int(ln.split()[1])
            i += 1
            for _ in range(npolys):
                nrings = int(lines[i])
                i += 1
                for _ in range(nrings):
                    nverts += int(lines[i].split()[0])
                    i += 1
            continue
        i += 1

    delta_units = u64_to_f64(args.get(1, 0))
    join_type = args.get(2, 0)
    miter_limit = u64_to_f64(args.get(4, 0))
    return {
        "site": site,
        "layer": layer,
        "opcode": opcode,
        "delta_mm": round(delta_units / 10000.0, 6),
        "join_type": join_type,
        "miter_limit": round(miter_limit, 6),
        "nverts": nverts,
        "path": p,
        "text": text.strip() + "\n",
    }


def pick_quantiles(items, k):
    ordered = sorted(items, key=lambda x: (x["nverts"], x["layer"], x["path"].name))
    if len(ordered) <= k:
        return ordered
    idxs = sorted({round(i * (len(ordered) - 1) / (k - 1)) for i in range(k)})
    chosen = [ordered[i] for i in idxs]
    return sorted(chosen, key=lambda x: (x["layer"], x["path"].name))


def main():
    assert RAW_DIR.is_dir(), f"Missing raw capture dir: {RAW_DIR}"
    census_lines = (RAW_DIR / "census.jsonl").read_text().splitlines()
    census_by_site = collections.defaultdict(list)
    for line in census_lines:
        row = json.loads(line)
        census_by_site[row["site"]].append(row)

    by_site = collections.defaultdict(list)
    for p in sorted(RAW_DIR.glob("*.case")):
        rec = parse_case_file(p)
        by_site[rec["site"]].append(rec)

    selected = []
    for site in ("SH_OPEN_TOP_CANON", "SH_OPEN_TOP_LEGACY", "SH_OPEN_BOT_CANON", "SH_OPEN_BOT_LEGACY"):
        items = sorted(by_site[site], key=lambda x: (x["layer"], x["path"].name))
        selected.extend(items)
    for site, k in TARGETS.items():
        selected.extend(pick_quantiles(by_site[site], k))

    selected.sort(key=lambda x: (x["site"], x["layer"], x["path"].name))

    OUT_CORPUS.parent.mkdir(parents=True, exist_ok=True)
    corpus_text = "\n".join(item["text"].rstrip() for item in selected) + "\n"
    OUT_CORPUS.write_text(corpus_text)

    census_summary = {}
    for site, rows in sorted(census_by_site.items()):
        vs = sorted(r["nverts"] for r in rows)
        census_summary[site] = {
            "total_calls": len(rows),
            "nverts_min": vs[0],
            "nverts_median": int(statistics.median(vs)),
            "nverts_p90": vs[int(0.9 * (len(vs) - 1))],
            "nverts_max": vs[-1],
            "delta_mm_values": sorted({round(r["delta_mm"], 6) for r in rows}),
        }

    sample_summary = {}
    sel_by_site = collections.defaultdict(list)
    for item in selected:
        sel_by_site[item["site"]].append(item)
    for site, rows in sorted(sel_by_site.items()):
        vs = sorted(r["nverts"] for r in rows)
        sample_summary[site] = {
            "selected_cases": len(rows),
            "opcode": rows[0]["opcode"],
            "delta_mm": rows[0]["delta_mm"],
            "join_type": rows[0]["join_type"],
            "miter_limit": rows[0]["miter_limit"],
            "layers": [r["layer"] for r in rows],
            "nverts_min": vs[0],
            "nverts_median": int(statistics.median(vs)),
            "nverts_max": vs[-1],
        }

    payload = {
        "model": "tmp/base.stl",
        "config": "docs/specs/perf-vs-orca/evidence/matched-pair/configs/pnp-classic-supports-off.json",
        "capture_gcode_sha256": "0a1ee3df67e5e9713f8acf8428fb44d2a8a3bde2b236de5bdf8ecb03b903af9e",
        "matches_t166_reference": True,
        "corpus_file": "corpus/base-prepass-t167.corpus",
        "corpus_sha256": hashlib.sha256(OUT_CORPUS.read_bytes()).hexdigest(),
        "corpus_bytes": OUT_CORPUS.stat().st_size,
        "total_selected_cases": len(selected),
        "full_census": census_summary,
        "selected_sample": sample_summary,
    }
    OUT_CENSUS.write_text(json.dumps(payload, indent=2) + "\n")
    print(f"Wrote {OUT_CORPUS} ({len(selected)} cases, {OUT_CORPUS.stat().st_size / 1e6:.2f} MB)")
    for site, info in sample_summary.items():
        print(
            f"  {site:22s} n={info['selected_cases']:3d} op={info['opcode']} "
            f"delta={info['delta_mm']:+.4f} join={info['join_type']} miter={info['miter_limit']} "
            f"verts={info['nverts_min']}..{info['nverts_median']}..{info['nverts_max']}"
        )


if __name__ == "__main__":
    main()
