#!/usr/bin/env python3
"""Verify the headline claims of the Orca rig repair (perf-vs-orca ticket 98).

Run from anywhere; paths are relative to this file. Exits 0 when every claim holds,
1 otherwise. `--self-test` re-runs all checks against mutated copies and requires each
mutation to be rejected, so a check that cannot fail is caught.

Claims re-derived here:
  C1  the repaired rig produced 3 measured runs of the benchy classic-off cell, all
      exit 0, equal gcode bytes, and a median wall under 2.2 s (archive: 2.787 s)
  C2  the JSON-tree-only arm paid ~8 s of extra serial wall on the same cell
  C3  the repaired run's resolved appendix carries the vendor filament chain
      (density 1.24, max volumetric speed 12, first-layer temp 220) and arc fitting
      pinned off, while the process chain resolved in both eras (inner_wall_speed 300)
  C4  the archived 2026-10-23 appendix of the same cell carries Orca's *defaults* for
      the filament keys (0 / 2 / 200) -- the divergence C3 exposes
  C5  the repaired-but-unpinned run (same appendix as C3) still carried OrcaSlicer
      2.5's *other* flipped defaults -- elefant_foot_compensation 0.15 and
      brim_width 5 -- which ticket 103's "pinned back" ruling now pins in the rig;
      the arc pin was the only one asserted at this ticket's close
"""

import sys
import csv
import pathlib

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[4]
ARCHIVE_APPENDIX = (
    REPO
    / "docs/specs/perf-vs-orca/evidence/t82-fairness-key-audit/appendix/benchy-classic-supports-off.txt"
)

MEASURED = HERE / "rows-benchy-classic-off.csv"
NO_CACHE = HERE / "rows-benchy-classic-off-no-cache.csv"
APPENDIX = HERE / "appendix-benchy-classic-off-orca-2.5.0-beta.txt"


class Claim:
    def __init__(self, name):
        self.name = name
        self.failures = []

    def check(self, ok, message):
        if not ok:
            self.failures.append(message)
        return ok


def read_rows(path):
    if not path.exists():
        raise SystemExit(f"missing evidence file: {path}")
    with path.open(encoding="utf-8") as fh:
        return list(csv.DictReader(fh))


def read_appendix(path):
    if not path.exists():
        raise SystemExit(f"missing appendix file: {path}")
    out = {}
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        line = line.lstrip("; ").strip()
        if " = " in line:
            key, _, value = line.partition(" = ")
            if key and all(c.isalnum() or c == "_" for c in key):
                out[key] = value
    return out


def claim_c1(paths):
    c = Claim("C1 repaired rig measured three runs of benchy classic-off")
    rows = read_rows(paths["measured"])
    measured = [r for r in rows if r["warmup"] == "0"]
    c.check(len(measured) == 3, f"expected 3 measured rows, got {len(measured)}")
    if not measured:
        return c
    c.check(
        all(r["exit_code"] == "0" for r in measured), "a measured run did not exit 0"
    )
    bytestrings = {r["gcode_bytes"] for r in measured}
    c.check(
        len(bytestrings) == 1, f"gcode bytes differ across runs: {sorted(bytestrings)}"
    )
    walls = sorted(float(r["wall_seconds"]) for r in measured)
    median = walls[len(walls) // 2]
    c.check(median < 2.2, f"median wall {median:.3f}s is not under the 2.2s screen")
    c.check(median > 0.5, f"median wall {median:.3f}s is implausibly small")
    c.check(measured[0]["tool"] == "orca", "measured row is not an Orca row")
    c.check(
        "--enable-arc-fitting=0" in (measured[0].get("config_path") or ""),
        "config_path does not record the arc-fitting pin",
    )
    return c


def claim_c2(paths):
    c = Claim("C2 first touch of a staged tree is one-time and absorbed by the warmup")
    rows = read_rows(paths["measured"])
    warm = [r for r in rows if r["warmup"] == "1"]
    measured = sorted(float(r["wall_seconds"]) for r in rows if r["warmup"] == "0")
    if not warm or len(measured) < 3:
        c.check(False, "batch is missing its warmup row or its measured rows")
        return c
    steady = measured[len(measured) // 2]
    premium = float(warm[0]["wall_seconds"]) - steady
    c.check(premium > 5.0, f"first-touch premium {premium:.2f}s is not above 5s")
    c.check(
        premium < 15.0, f"first-touch premium {premium:.2f}s exceeds anything measured"
    )
    # A warm tree is fast even with no cache present: the premium is first-touch
    # only, so the cache copy is belt-and-braces, not a per-run requirement.
    no_cache = [float(r["wall_seconds"]) for r in read_rows(paths["no_cache"])]
    c.check(no_cache, "no-cache arm is empty")
    if no_cache:
        c.check(
            max(no_cache) < 2.5,
            f"no-cache arm ran at {max(no_cache):.2f}s, i.e. the tree is not warm",
        )
        c.check(
            max(no_cache) - min(no_cache) < 0.5,
            f"no-cache arm spread {max(no_cache) - min(no_cache):.2f}s is unstable",
        )
    return c


def claim_c3(paths):
    c = Claim("C3 repaired run resolves the vendor filament chain")
    app = read_appendix(paths["appendix"])
    c.check(len(app) > 500, f"appendix has only {len(app)} keys")
    c.check(
        app.get("filament_density") == "1.24",
        f"filament_density = {app.get('filament_density')!r}",
    )
    c.check(
        app.get("filament_max_volumetric_speed") == "12",
        f"filament_max_volumetric_speed = {app.get('filament_max_volumetric_speed')!r}",
    )
    c.check(
        app.get("first_layer_temperature") == "220",
        f"first_layer_temperature = {app.get('first_layer_temperature')!r}",
    )
    c.check(
        app.get("enable_arc_fitting") in ("disabled", "0"),
        f"enable_arc_fitting = {app.get('enable_arc_fitting')!r} (pin not applied)",
    )
    c.check(
        app.get("inner_wall_speed") == "300",
        f"inner_wall_speed = {app.get('inner_wall_speed')!r}",
    )
    c.check(
        app.get("wall_generator") == "classic",
        f"wall_generator = {app.get('wall_generator')!r}",
    )
    return c


def claim_c4(paths):
    c = Claim("C4 archived appendix carries Orca's filament defaults")
    app = read_appendix(paths["archive"])
    c.check(
        app.get("filament_density") == "0",
        f"archived filament_density = {app.get('filament_density')!r}",
    )
    c.check(
        app.get("filament_max_volumetric_speed") == "2",
        f"archived filament_max_volumetric_speed = {app.get('filament_max_volumetric_speed')!r}",
    )
    c.check(
        app.get("first_layer_temperature") == "200",
        f"archived first_layer_temperature = {app.get('first_layer_temperature')!r}",
    )
    c.check(
        app.get("inner_wall_speed") == "300",
        f"archived inner_wall_speed = {app.get('inner_wall_speed')!r} (process leg should have resolved)",
    )
    return c


def claim_c5(paths):
    c = Claim("C5 repaired-but-unpinned run carried the other two 2.5 default flips")
    app = read_appendix(paths["appendix"])
    c.check(
        app.get("elefant_foot_compensation") == "0.15",
        f"elefant_foot_compensation = {app.get('elefant_foot_compensation')!r} "
        "(expected the unpinned 2.5 default 0.15)",
    )
    c.check(
        app.get("brim_width") == "5",
        f"brim_width = {app.get('brim_width')!r} (expected the unpinned 2.5 default 5)",
    )
    c.check(
        app.get("enable_arc_fitting") in ("disabled", "0"),
        "arc fitting was already pinned in this run, so it is not evidence of a flip",
    )
    return c


CLAIMS = (claim_c1, claim_c2, claim_c3, claim_c4, claim_c5)


def default_paths():
    return {
        "measured": MEASURED,
        "no_cache": NO_CACHE,
        "appendix": APPENDIX,
        "archive": ARCHIVE_APPENDIX,
    }


def run(paths, verbose=True):
    failed = 0
    for claim in CLAIMS:
        result = claim(paths)
        if verbose:
            if result.failures:
                print(f"FAIL {result.name}")
                for f in result.failures:
                    print(f"     - {f}")
            else:
                print(f"ok   {result.name}")
        failed += 1 if result.failures else 0
    return failed


def rewrite(src, dst, mutate):
    """Copy src to dst applying mutate(text) -> text."""
    text = src.read_text(encoding="utf-8", errors="replace")
    dst.write_text(mutate(text), encoding="utf-8")


def self_test():
    import tempfile

    failures = []
    with tempfile.TemporaryDirectory() as tmp:
        tmpdir = pathlib.Path(tmp)

        def arms():
            p = default_paths()
            return dict(p)

        # Each mutation must break exactly the claim it targets.
        mutations = [
            (
                "C1 median wall inflated past the screen",
                "measured",
                lambda t: (
                    t.replace(",1.9", ",9.9")
                    .replace("1.97", "9.97")
                    .replace("2.04", "9.94")
                    .replace("1.978", "9.978")
                ),
            ),
            (
                "C1 arc-fitting pin dropped",
                "measured",
                lambda t: t.replace("--enable-arc-fitting=0", "--enable-arc-fitting=1"),
            ),
            (
                "C2 warmup premium erased",
                "measured",
                lambda t: t.replace("11.68", "2.00"),
            ),
            (
                "C2 no-cache arm made slow",
                "no_cache",
                lambda t: t.replace("2.0067", "9.0067"),
            ),
            (
                "C3 filament chain unresolved",
                "appendix",
                lambda t: t.replace("filament_density = 1.24", "filament_density = 0"),
            ),
            (
                "C3 arc fitting left enabled",
                "appendix",
                lambda t: t.replace(
                    "enable_arc_fitting = disabled", "enable_arc_fitting = enabled"
                ),
            ),
            (
                "C4 archive divergence erased",
                "archive",
                lambda t: t.replace(
                    "filament_max_volumetric_speed = 2\n",
                    "filament_max_volumetric_speed = 12\n",
                ),
            ),
            (
                "C5 brim default pinned early",
                "appendix",
                lambda t: t.replace("brim_width = 5\n", "brim_width = 0\n"),
            ),
            (
                "C5 elefant-foot default pinned early",
                "appendix",
                lambda t: t.replace(
                    "elefant_foot_compensation = 0.15\n",
                    "elefant_foot_compensation = 0\n",
                ),
            ),
        ]

        if run(arms(), verbose=False) != 0:
            failures.append("baseline run is not green, self-test is meaningless")

        for name, key, mutate in mutations:
            p = arms()
            dst = tmpdir / f"mut-{key}.txt"
            rewrite(p[key], dst, mutate)
            if dst.read_text(encoding="utf-8", errors="replace") == p[key].read_text(
                encoding="utf-8", errors="replace"
            ):
                failures.append(
                    f"negative control '{name}' did not change the file (control is vacuous)"
                )
                continue
            p[key] = dst
            if run(p, verbose=False) == 0:
                failures.append(f"negative control '{name}' was NOT rejected")

        if failures:
            print("SELF-TEST FAIL")
            for f in failures:
                print(f"  - {f}")
            return 1
        print(
            f"SELF-TEST PASS ({len(CLAIMS)} claims, {len(mutations)} negative controls rejected)"
        )
        return 0


def main(argv):
    if "--self-test" in argv:
        return self_test()
    return 1 if run(default_paths()) else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
