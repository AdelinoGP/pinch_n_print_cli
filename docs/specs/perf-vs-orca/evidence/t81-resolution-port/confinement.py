#!/usr/bin/env python3
"""Confinement proof for the `resolution` port (perf-vs-orca ticket 81).

For every gate cell, render the same two layers through `pnp_cli visual-debug`
in both arms and compare the **typed IR captures** exactly:

  * `Layer::Slice`            must be IDENTICAL  (host prepass is untouched)
  * `PrePass::SupportGeometry` must be IDENTICAL  (supports-on cells only)
  * `Layer::Perimeters`       must DIFFER        (the intended change)

A rasterised PNG could not falsify a sub-pixel leak; the typed captures are the
IR itself, so they can. The PNGs are still written so a human (or an agent
looking at them) can see the layer; they are evidence, never the oracle.

Usage:
    python docs/specs/perf-vs-orca/evidence/t81-resolution-port/confinement.py
"""

from __future__ import annotations

import hashlib
import json
import subprocess
import sys
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parents[5]
ARMS = {
    "baseline": REPO / "target/t81/baseline",
    "candidate": REPO / "target/t81/candidate",
}
CONFIG_DIR = REPO / "docs/specs/perf-vs-orca/evidence/matched-pair/configs"
FIXTURES = {"benchy": REPO / "tmp/3dbenchy.stl", "base": REPO / "tmp/base.stl"}
LAYERS = [12]


def request(
    arm: str,
    fixture: str,
    generator: str,
    supports: str,
    taps: list[str],
    visualization: str,
) -> dict:
    return {
        "schema_version": "1.0.0",
        "source": {
            "kind": "model",
            "model": str(FIXTURES[fixture]).replace("\\", "/"),
            "config": str(
                CONFIG_DIR / f"pnp-{generator}-supports-{supports}.json"
            ).replace("\\", "/"),
            "module_dirs": [str(ARMS[arm] / "modules").replace("\\", "/")],
        },
        "layers": LAYERS,
        "taps": taps,
        "visualizations": [visualization],
        "resolution_scale": 1,
    }


def _render(
    arm: str,
    fixture: str,
    generator: str,
    supports: str,
    taps: list[str],
    visualization: str,
    workdir: Path,
    tag: str,
) -> dict:
    out = workdir / f"{fixture}-{generator}-{supports}-{arm}-{tag}"
    req = workdir / f"{out.name}.json"
    req.write_text(
        json.dumps(
            request(arm, fixture, generator, supports, taps, visualization), indent=2
        ),
        encoding="utf-8",
    )
    proc = subprocess.run(
        [
            str(ARMS[arm] / "pnp_cli.exe"),
            "visual-debug",
            "--request",
            str(req),
            "--output",
            str(out),
            "--overwrite",
        ],
        capture_output=True,
        text=True,
    )
    if proc.returncode != 0:
        raise SystemExit(
            "visual-debug failed for "
            + out.name
            + "\n"
            + proc.stdout
            + "\n"
            + proc.stderr
        )
    manifest = json.loads((out / "manifest.json").read_text(encoding="utf-8"))
    found = {}
    for entry in manifest["images"]:
        key = (entry["layer_index"], entry["tap"])
        capture = entry.get("typed_capture")
        if capture is None:
            raise SystemExit(
                f"{out.name}: {key} produced no typed capture; the confinement check needs IR"
            )
        digest = hashlib.sha256(
            json.dumps(capture["value"], sort_keys=True, separators=(",", ":")).encode()
        ).hexdigest()[:16]
        found[key] = (digest, capture["value"])
    return found


def _render_support_silhouette(
    arm: str, fixture: str, generator: str, supports: str, workdir: Path
) -> dict:
    """Support geometry has no bead width, so `filled_areas` fails closed and the
    tap emits no typed capture. A silhouette is the strongest view it offers; the
    PNG bytes are compared, which is raster evidence (disclosed as such), not IR."""
    out = workdir / f"{fixture}-{generator}-{supports}-{arm}-supportsil"
    req = workdir / f"{out.name}.json"
    request_body = {
        "schema_version": "1.2.0",
        "source": {
            "kind": "model",
            "model": str(FIXTURES[fixture]).replace("\\", "/"),
            "config": str(
                CONFIG_DIR / f"pnp-{generator}-supports-{supports}.json"
            ).replace("\\", "/"),
            "module_dirs": [str(ARMS[arm] / "modules").replace("\\", "/")],
        },
        "layers": LAYERS,
        "taps": ["PrePass::SupportGeometry"],
        "visualizations": [
            {
                "type": "silhouette",
                "tap": "PrePass::SupportGeometry",
                "options": {"view": "front"},
            }
        ],
        "resolution_scale": 1,
    }
    req.write_text(json.dumps(request_body, indent=2), encoding="utf-8")
    proc = subprocess.run(
        [
            str(ARMS[arm] / "pnp_cli.exe"),
            "visual-debug",
            "--request",
            str(req),
            "--output",
            str(out),
            "--overwrite",
        ],
        capture_output=True,
        text=True,
    )
    if proc.returncode != 0:
        raise SystemExit(
            "visual-debug (support silhouette) failed for "
            + out.name
            + "\n"
            + proc.stdout
            + "\n"
            + proc.stderr
        )
    manifest = json.loads((out / "manifest.json").read_text(encoding="utf-8"))
    found = {}
    for entry in manifest["images"]:
        png = out / entry["png_path"]
        digest = hashlib.sha256(png.read_bytes()).hexdigest()[:16]
        found[(entry.get("png_path", "?"), entry["tap"] + " [silhouette png]")] = (
            digest,
            None,
        )
    return found


def captures(
    arm: str, fixture: str, generator: str, supports: str, workdir: Path
) -> dict:
    """Perimeter and slice taps render as filled areas; support geometry has no
    bead width, so it renders as filament lines in its own request."""
    found = _render(
        arm,
        fixture,
        generator,
        supports,
        ["Layer::Slice", "Layer::Perimeters"],
        "filled_areas",
        workdir,
        "fill",
    )
    if supports == "on":
        found.update(
            _render_support_silhouette(
                arm,
                fixture,
                generator,
                supports,
                workdir,
            )
        )
    return found


def main() -> int:
    rows = []
    failures = []
    with tempfile.TemporaryDirectory() as tmp:
        workdir = Path(tmp)
        for fixture in ("benchy", "base"):
            for generator in ("classic", "arachne"):
                for supports in ("off", "on"):
                    if (
                        fixture == "base"
                        and generator == "arachne"
                        and supports == "on"
                    ):
                        # Still checked: the base arachne supports-on cell is a gate cell.
                        pass
                    base = captures("baseline", fixture, generator, supports, workdir)
                    cand = captures("candidate", fixture, generator, supports, workdir)
                    for key in sorted(set(base) | set(cand), key=lambda k: (str(k[1]), str(k[0]))):
                        b = base.get(key)
                        c = cand.get(key)
                        same = b is not None and c is not None and b[0] == c[0]
                        expect_same = key[1] != "Layer::Perimeters"
                        ok = same == expect_same
                        rows.append(
                            {
                                "fixture": fixture,
                                "cell": f"{generator}-{supports}",
                                "layer": key[0],
                                "tap": key[1],
                                "expect": "identical" if expect_same else "differs",
                                "observed": "identical" if same else "differs",
                                "ok": ok,
                                "baseline_sha": b[0] if b else None,
                                "candidate_sha": c[0] if c else None,
                            }
                        )
                        if not ok:
                            failures.append(rows[-1])
                        print(
                            f"{fixture:6s} {generator}-{supports:3s} l{str(key[0]):<34s} {key[1]:28s} "
                            f"expect={rows[-1]['expect']:9s} observed={rows[-1]['observed']:9s} "
                            f"{'PASS' if ok else 'FAIL'}"
                        )

    out = Path(__file__).resolve().parent / "confinement.json"
    out.write_text(
        json.dumps({"rows": rows, "failures": failures, "layers": LAYERS}, indent=2),
        encoding="utf-8",
    )
    print(f"\n{len(rows)} checks, {len(failures)} failure(s) -> {out}")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
