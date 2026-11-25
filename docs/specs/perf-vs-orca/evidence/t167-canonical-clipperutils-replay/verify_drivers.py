"""Protocol and geometric-equivalence smoke checks across all four t167 replay engines."""

import json
import struct
import subprocess
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
SCRATCH = REPO / ".local-artifacts/perf-vs-orca/t167-canonical-clipperutils-replay"


def f64_bits(v: float) -> int:
    return struct.unpack("<Q", struct.pack("<d", float(v)))[0]


def rect(x0, y0, x1, y1, ccw=True):
    pts = [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
    if not ccw:
        pts.reverse()
    return pts


def fmt_ring(pts):
    return f"{len(pts)} " + " ".join(f"{x} {y}" for x, y in pts)


def build_controls():
    # Coordinates in PnP units (100 nm = 1e-4 mm); 10000 = 1 mm
    outer = rect(0, 0, 100000, 100000, ccw=True)
    hole = rect(30000, 30000, 70000, 70000, ccw=False)
    clip_r = rect(50000, 0, 120000, 100000, ccw=True)
    thin = rect(0, 0, 100000, 500, ccw=True)  # 0.05 mm tall strip: dies under r=0.04 opening? 0.05 < 2*0.04=0.08 -> YES!
    wide = rect(0, 0, 100000, 2000, ccw=True)  # 0.20 mm tall strip: survives r=0.04 opening (area 2e8 u2)!

    cases = [
        # 0: offset_ex positive (+1000 = +0.1mm) on square with hole
        f"""case CTRL_OFF_POS 0 0
arg 1 {f64_bits(1000.0)}
arg 2 3
arg 3 0
arg 4 {f64_bits(3.0)}
arg 5 {f64_bits(0.0)}
subj 1
2
{fmt_ring(outer)}
{fmt_ring(hole)}
end
""",
        # 1: offset_ex negative (-1000 = -0.1mm) on square with hole
        f"""case CTRL_OFF_NEG 0 0
arg 1 {f64_bits(-1000.0)}
arg 2 3
arg 3 0
arg 4 {f64_bits(3.0)}
arg 5 {f64_bits(0.0)}
subj 1
2
{fmt_ring(outer)}
{fmt_ring(hole)}
end
""",
        # 2: difference_ex
        f"""case CTRL_DIFF 0 1
arg 10 1
subj 1
2
{fmt_ring(outer)}
{fmt_ring(hole)}
clip 1
1
{fmt_ring(clip_r)}
end
""",
        # 3: union_ex
        f"""case CTRL_UNION 0 2
arg 10 1
subj 1
1
{fmt_ring(outer)}
clip 1
1
{fmt_ring(clip_r)}
end
""",
        # 4: intersection_ex
        f"""case CTRL_INTER 0 3
arg 10 1
subj 1
2
{fmt_ring(outer)}
{fmt_ring(hole)}
clip 1
1
{fmt_ring(clip_r)}
end
""",
        # 5: opening_ex canonical (r = 400 = 0.04mm, Miter 3) on thin strip -> empty
        f"""case CTRL_OPEN_EMPTY 0 5
arg 1 {f64_bits(400.0)}
arg 2 3
arg 3 0
arg 4 {f64_bits(3.0)}
arg 5 {f64_bits(0.0)}
subj 1
1
{fmt_ring(thin)}
end
""",
        # 6: opening_ex canonical (r = 400 = 0.04mm, Miter 3) on 0.2mm strip -> survives intact
        f"""case CTRL_OPEN_KEEP 0 5
arg 1 {f64_bits(400.0)}
arg 2 3
arg 3 0
arg 4 {f64_bits(3.0)}
arg 5 {f64_bits(0.0)}
subj 1
1
{fmt_ring(wide)}
end
""",
        # 7: opening_ex legacy (r = 2000 = 0.2mm, Round) on 10x10mm square with 4x4mm hole
        f"""case CTRL_OPEN_ROUND 0 5
arg 1 {f64_bits(2000.0)}
arg 2 2
arg 3 0
arg 4 {f64_bits(2.0)}
arg 5 {f64_bits(0.0)}
subj 1
2
{fmt_ring(outer)}
{fmt_ring(hole)}
end
""",
    ]
    return "\n".join(cases)


def main():
    prov = json.loads((SCRATCH / "provenance.json").read_text())
    ctrl = SCRATCH / "controls.cases"
    ctrl.write_text(build_controls())

    expected_areas = {
        "CTRL_OFF_POS": (102000.0 ** 2) - (38000.0 ** 2),
        "CTRL_OFF_NEG": (98000.0 ** 2) - (42000.0 ** 2),
        "CTRL_DIFF": (50000.0 * 100000.0) - (20000.0 * 40000.0),
        "CTRL_UNION": 120000.0 * 100000.0,
        "CTRL_INTER": (50000.0 * 100000.0) - (20000.0 * 40000.0),
        "CTRL_OPEN_EMPTY": 0.0,
        "CTRL_OPEN_KEEP": 100000.0 * 2000.0,
    }

    results = {}
    for eng, info in prov["binaries"].items():
        out = subprocess.check_output([info["path"], str(ctrl)], text=True, timeout=60)
        rows = [json.loads(line) for line in out.splitlines() if line.strip()]
        assert len(rows) == 8, (eng, len(rows))
        results[eng] = {r["site"]: r for r in rows}

    for site, exp_area in expected_areas.items():
        for eng, by_site in results.items():
            got = by_site[site]["out_area_u2"]
            tol = max(1.0, abs(exp_area) * 1e-6)
            assert abs(got - exp_area) <= tol, f"{eng} {site}: got {got}, expected {exp_area}"

    # Check round opening agrees across all 4 engines within 0.2% area
    ref_area = results["rust_pnp"]["CTRL_OPEN_ROUND"]["out_area_u2"]
    assert ref_area > 0
    for eng, by_site in results.items():
        got = by_site["CTRL_OPEN_ROUND"]["out_area_u2"]
        rel = abs(got - ref_area) / ref_area
        assert rel < 0.002, f"{eng} CTRL_OPEN_ROUND area rel diff {rel}: {got} vs {ref_area}"
        print(f"CTRL_OPEN_ROUND {eng:14s}: area={got:.1f} verts={by_site['CTRL_OPEN_ROUND']['out_verts']} paths={by_site['CTRL_OPEN_ROUND']['out_paths']}")

    print("VERIFY_DRIVERS PASS (all 4 engines x 8 controls)")


if __name__ == "__main__":
    main()
