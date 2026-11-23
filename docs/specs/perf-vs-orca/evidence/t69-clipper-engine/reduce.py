#!/usr/bin/env python3
"""Reduce T69-DUMP inputs; geometry is sampled on capture, census is exhaustive.

Run from any directory: python <this-file> [--raw PATH]. Never reads giant JSON
fixtures. Whole-layer uniform sampling is coarsened deterministically until the
ASCII corpus fits the declared 20,000,000-byte ceiling.
"""

import argparse
import collections
import hashlib
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
SITES = ["A0", "A1", "B", "C", "D", "E", "F", "G", "H", "J", "K"]
NORMAL = [
    "offset",
    "offset",
    "offset",
    "offset",
    "clip",
    "offset",
    "offset",
    "offset",
    "clip",
    "clip",
    "offset",
]
SHORT = ["offset", "offset", "offset", "offset", "offset", "clip", "clip", "offset"]
SHORT_SITES = ["A0", "A1", "E", "F", "G", "H", "J", "K"]
LIMIT = 20_000_000


def load(path):
    return json.loads(path.read_text(encoding="utf-8-sig"))


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def read_job(raw, model):
    job = raw / f"{model}-classic-off"
    meta = load(raw / f"{model}-classic-off.json")
    refs = load(
        REPO
        / "docs/specs/perf-vs-orca/evidence/t58-finalization-copies/qualification.json"
    )
    ref = next(c for c in refs["cells"] if c["cell"] == f"ordinary-{model}-classic-off")
    actual = sha(raw / f"{model}-classic-off.gcode")
    assert actual == meta["sha256"] == meta["reference_sha256"] == ref["sha256"]
    assert meta["exit_code"] == 0 and meta["status"] == "ok" and not meta["degraded"]
    assert meta["fatal"] == meta["non_fatal"] == 0
    calls, engines, records = collections.Counter(), collections.Counter(), []
    engine_opcodes = collections.defaultdict(collections.Counter)
    layer_services = {}
    for census in sorted(job.glob("*.calls")):
        owner, layer = census.stem.split("-")
        layer = int(layer)
        lines = [line.split() for line in census.read_text().splitlines()]
        services = [line for line in lines if line[0] == "service"]
        if owner == "CP":
            signature = [s[2] for s in services]
            if signature == NORMAL:
                sites = SITES
            else:
                assert signature == SHORT, (model, layer, signature)
                sites = SHORT_SITES
            layer_services[str(layer)] = sites
            calls.update(sites)
        seen_engine_services = set()
        for row in lines:
            if row[0] != "engine":
                continue
            _, seq, engine, opcode, vertices, filename = row
            if owner == "CP":
                assert int(seq) not in seen_engine_services, (
                    model,
                    layer,
                    "unexpected nested engine call",
                    seq,
                )
                seen_engine_services.add(int(seq))
                assert (services[int(seq)][2], int(opcode)) in [
                    ("offset", 0),
                    ("clip", 1),
                ]
            site = sites[int(seq)] if owner == "CP" else owner
            engines[site] += 1
            engine_opcodes[site][opcode] += 1
            if owner != "CP":
                calls[site] += (
                    1  # RP units are actual engine invocations, not services.
                )
            if filename != "-":
                path = job / filename
                text = path.read_text(encoding="ascii")
                header, body = text.split("\n", 1)
                assert header == f"case {owner} {layer} {opcode} {seq}"
                text = f"case {site} {layer} {opcode} {seq}\n" + body
                records.append(
                    dict(
                        site=site,
                        layer=layer,
                        seq=int(seq),
                        engine=int(engine),
                        text=text,
                        source=filename,
                        vertices=int(vertices),
                    )
                )
    assert len(layer_services) == {"benchy": 240, "base": 495}[model]
    assert (
        sum(v for k, v in calls.items() if k in SITES)
        == {"benchy": 2640, "base": 5442}[model]
    )
    return dict(
        metadata=meta,
        raw_service_or_rp_engine_counts=dict(calls),
        raw_engine_counts=dict(engines),
        raw_engine_opcode_counts={
            site: dict(counts) for site, counts in engine_opcodes.items()
        },
        layer_services=layer_services,
        raw_dump_files=len(list(job.iterdir())),
        raw_dump_bytes=sum(p.stat().st_size for p in job.iterdir()),
        records=records,
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--raw",
        type=Path,
        default=REPO / ".local-artifacts/perf-vs-orca/t69-clipper-engine/raw",
    )
    opts = parser.parse_args()
    jobs = {model: read_job(opts.raw, model) for model in ["benchy", "base"]}
    # Prefer classic-site coverage over the optional RP leg. Each family keeps
    # whole layers; no vertex trimming or selection of only cheap calls.
    strides = (20, 40, 80, 160, 320, 640, 1280)
    for classic_stride in strides:
        for prep_stride in strides:
            chosen = {
                m: [
                    r
                    for r in j["records"]
                    if r["layer"]
                    % (classic_stride if r["site"] in SITES else prep_stride)
                    == 0
                    or r["layer"] in (239, 494, 270)
                ]
                for m, j in jobs.items()
            }
            size = sum(
                len(r["text"].encode("ascii")) for rows in chosen.values() for r in rows
            )
            if size <= LIMIT - 500_000:
                break
        else:
            continue
        break
    else:
        raise AssertionError("mandatory strata alone exceed corpus budget")
    out = HERE / "corpus/t68sites"
    out.mkdir(parents=True, exist_ok=True)
    report = dict(
        format_version=1,
        corpus_ceiling_bytes=LIMIT,
        raw_geometry_sampling="layer % 20 == 0 or layer in {239, 494, 270}",
        reduced_sampling=f"classic: layer % {classic_stride} == 0; RP: layer % {prep_stride} == 0; both additionally retain layer in {{239, 494, 270}} (existing layers only)",
        jobs={},
        files={},
    )
    for model, job in jobs.items():
        selected = chosen[model]
        assert set(SITES) <= {r["site"] for r in selected}, (
            model,
            "sample lost a classic site",
        )
        for suffix, pred in [
            ("t68sites", lambda r: r["site"] in SITES),
            ("region-prep", lambda r: r["site"] not in SITES),
        ]:
            rows = sorted(
                (r for r in selected if pred(r)),
                key=lambda r: (
                    r["layer"],
                    r["site"].startswith("RP"),
                    r["seq"],
                    r["engine"],
                ),
            )
            path = out / f"{model}-{suffix}.corpus"
            path.write_text(
                "".join(r["text"] for r in rows), encoding="ascii", newline="\n"
            )
            report["files"][path.name] = dict(
                bytes=path.stat().st_size,
                sha256=sha(path),
                cases=len(rows),
                raw_sources=[
                    dict(
                        site=r["site"],
                        filename=r["source"],
                        sha256=sha(opts.raw / f"{model}-classic-off" / r["source"]),
                    )
                    for r in rows
                ],
            )
        records = job.pop("records")
        report["jobs"][model] = dict(
            **job,
            sampled_engine_counts=dict(
                collections.Counter(r["site"] for r in selected)
            ),
            sampled_layers=sorted({r["layer"] for r in selected}),
            sampled_classic_layers=sorted(
                {r["layer"] for r in selected if r["site"] in SITES}
            ),
            sampled_region_prep_layers=sorted(
                {r["layer"] for r in selected if r["site"] not in SITES}
            ),
            raw_sampled_engine_counts=dict(
                collections.Counter(r["site"] for r in records)
            ),
        )
    (out / "summary.json").write_text(
        json.dumps(report, indent=2) + "\n", encoding="utf-8"
    )
    mappings = [
        ("A0", "outer wall-loop inset"),
        ("A1", "inner wall-loop inset"),
        ("B", "gap shrunk_prev"),
        ("C", "gap grown_cur"),
        ("D", "gap difference"),
        ("E", "infill_area"),
        ("F", "shrunk_inner"),
        ("G", "grown_infill"),
        ("H", "final-transition difference"),
        ("J", "gap-width-band filtered_gaps"),
        ("K", "infill-boundary inset"),
    ]
    text = """# T68-site engine-input corpus (format v1)

Coordinates are raw signed i64 engine values: 1 unit = 100 nm. No scaling,
sorting, orientation normalization, simplification, or polygon filtering is
performed by the recorder or reducer. Each already-flat engine path is framed
as one path with one ring; flatten all rings, in order, before engine execution.
Case call_index is the zero-based host-service index for CP; for RP it is the
zero-based engine invocation within that layer's preparation derivation.

## Production boundary and parameters

All classic sites are in `ClassicPerimeters::emit_walls`
(`modules/core-modules/classic-perimeters/src/lib.rs`), via `hs::Host::{offset_polygons,clip_polygons}`
(`crates/slicer-wasm-host/src/host.rs`). The recorder runs immediately before
`ClipperOffset::add_paths/execute_tree` in `inflate_once` and before
`boolean_op_tree_64` in `clip_polygons` (`crates/slicer-core/src/polygon_ops.rs`).
Offsets returning early for empty input or zero delta are counted as services
but do NOT invent engine cases; compare raw service and engine counts below.
Offsets use `ClipperOffset::new(miter_limit, arc_tolerance, false, false)` and
`execute_tree`; booleans use the PolyTree helper, not a flat-path approximation.

Opcode: 0 offset, 1 difference, 2 union, 3 intersection, 4 xor.
Arg 1: delta f64 bits in integer units (not mm); 2: JoinType (Square=0,
Bevel=1, Round=2, Miter=3); 3: EndType (Polygon=0, Joined=1, Butt=2,
Square=3, Round=4); 4: miter-limit f64 bits; 5: arc-tolerance f64 bits in
integer units. Boolean arg 10: FillRule (EvenOdd=0, NonZero=1, Positive=2,
Negative=3). These are clipper2-rust 1.1.0 enum discriminants. Both ClipperOffset
boolean constructor flags are false, the production defaults, with no additional
per-case parameters. No separate subject/clip fill rule exists in this API.

| Site | `emit_walls` expression |
| --- | --- |
"""
    text += "".join(f"| {site} | {desc} |\n" for site, desc in mappings)
    text += """
RP1 is the actual derivation entered from `LayerArena::ensure_prepared_regions`;
RP2 is the actual derivation entered from `LayerArena::ensure_prepared_perimeter_source_regions`
(`crates/slicer-runtime/src/blackboard.rs`). All nested `inflate_once` and
`clip_polygons` engine calls are included, not just the outer closing wrapper.
The marshal adapters are `prepare_slice_regions`, `prepare_perimeter_source_regions`,
and `prepare_regions` (`crates/slicer-wasm-host/src/marshal/prepared.rs`);
`SliceRegionView::derive_needs_support` (`crates/slicer-sdk/src/views.rs`) is
one native intersection consumer. The opcode census in `summary.json` records
what this checkout actually invokes; old t60 attribution is not presumed current.
On identity projections RP2 may execute `prepare_slice_regions` and populate
the shared ordinary cache; its RP2 tag describes the initiating arena method,
not an independent second projection. RP1/RP2 files therefore do not imply both
derivations execute on every layer (ticket 67 sharing is preserved).
The optional RP recorder keeps the last snapshot per owner/layer: repeated
derivations overwrite that owner's earlier snapshot. RP raw counts therefore
mean engine calls in retained snapshots, not an exhaustive preparation census.
Do not use RP counts as timing denominators or frequency weights.

## Sampling and census

The classic-site census is checked against the complete matched-job service
totals; the RP census has the retained-snapshot limitation above. Geometry was
captured only for layers divisible by 20, plus layers 239, 270, 494 when present.
The endpoints retain terminal shapes; 270 retains the known collapsed-inset
stratum. The reduced set retains whole layers within each of the classic and RP
families. It tests power-of-two multiples of 20 through 1280, preferring the
smallest classic stride first, then the smallest RP stride that fits the
19,500,000-byte geometry budget (reserving space under the total ceiling for
documentation). Thus the optional RP leg does not crowd out classic coverage.
This is a shape corpus,
not a frequency-weighted benchmark; use the exhaustive census for weights.
Service sequence is mapped only after validating the whole dispatch signature;
the 8-service collapsed-inner-inset signature maps A0,A1,E,F,G,H,J,K, rather
than incorrectly relabeling shifted calls B,C,D,... . Unsupported signatures fail.

"""
    text += f"Reduced rule: `{report['reduced_sampling']}`.\n\n"
    for model, job in report["jobs"].items():
        text += f"### {model}\n\nClassic sampled layers: {job['sampled_classic_layers']}. RP sampled layers: {job['sampled_region_prep_layers']}. Raw dump: {job['raw_dump_files']} files, {job['raw_dump_bytes']} bytes.\n\n"
        text += "| Site | Raw services / RP engine calls | Raw engine calls | Raw geometry cases | Reduced engine cases |\n| --- | ---: | ---: | ---: | ---: |\n"
        for site in SITES + ["RP1", "RP2"]:
            text += f"| {site} | {job['raw_service_or_rp_engine_counts'].get(site, 0)} | {job['raw_engine_counts'].get(site, 0)} | {job['raw_sampled_engine_counts'].get(site, 0)} | {job['sampled_engine_counts'].get(site, 0)} |\n"
    text += "\n## Inventory\n\n| File | Bytes | Cases | SHA-256 |\n| --- | ---: | ---: | --- |\n"
    for name, info in report["files"].items():
        text += f"| {name} | {info['bytes']} | {info['cases']} | `{info['sha256']}` |\n"
    text += "\nRegenerate: `python docs/specs/perf-vs-orca/evidence/t69-clipper-engine/reduce.py`. Raw root defaults to `.local-artifacts/perf-vs-orca/t69-clipper-engine/raw/`; override with `--raw`. Output identity gates are re-read from t58 qualification and applied to actual raw G-code files before reduction.\n"
    (out / "README.md").write_text(text, encoding="utf-8", newline="\n")
    total = sum(p.stat().st_size for p in out.iterdir())
    assert total <= LIMIT, ("corpus total, including documentation", total)
    print(
        json.dumps(
            dict(
                total_corpus_bytes=total,
                classic_stride=classic_stride,
                prep_stride=prep_stride,
                files={
                    name: {key: info[key] for key in ("bytes", "sha256", "cases")}
                    for name, info in report["files"].items()
                },
            ),
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
