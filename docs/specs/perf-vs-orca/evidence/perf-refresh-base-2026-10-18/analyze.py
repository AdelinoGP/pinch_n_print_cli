#!/usr/bin/env python3
"""Print the diagnostic tables used by FINDINGS.md from the reduced summary."""

import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
CELLS = ["classic-off", "classic-on", "arachne-off", "arachne-on"]
STAGES = [
    "PrePass::MeshAnalysis",
    "PrePass::Slice",
    "PrePass::OverhangAnnotation",
    "PrePass::ShellClassification",
    "PrePass::SupportAnalysis",
    "PrePass::SupportGeometry",
    "PrePass::SeamPlanning",
    "PostPass::LayerFinalization",
    "PostPass::GCodeEmit",
]
FIN = (
    "com.core.overhang-classifier-default",
    "com.core.part-cooling",
    "com.core.skirt-brim",
    "com.core.wipe-tower",
)


def main():
    summary = json.loads((HERE / "summary.json").read_text())
    print("## Phase anatomy (instrumented / profile)")
    for cell in CELLS:
        generator, supports = cell.split("-")
        for kind in ("instrumented", "profile"):
            r = summary[f"ordinary-base-{generator}-{supports}-{kind}"]
            print(
                f"{cell:11s} {kind:12s} wall={r['process_wall_s']:7.2f}s "
                f"prepass={r['phases_ms']['prepass'] / 1000:7.2f} "
                f"per_layer={r['phases_ms']['per_layer'] / 1000:7.2f} "
                f"postpass={r['phases_ms']['postpass'] / 1000:7.2f}"
            )
            print(
                "   "
                + ", ".join(
                    f"{s.split('::')[1]}={r['stages_ms'].get(s, 0) / 1000:.2f}"
                    for s in STAGES
                )
            )
    print("\n## Finalization module brackets (supports-on vs supports-off)")
    for cell in CELLS:
        generator, supports = cell.split("-")
        r = summary[f"ordinary-base-{generator}-{supports}-instrumented"]
        mods = {
            k: round(v / 1000, 2) for k, v in r["serial_modules_ms"].items() if k in FIN
        }
        stage = r["stages_ms"].get("PostPass::LayerFinalization", 0) / 1000
        print(
            f"{cell:11s} stage={stage:6.2f} modules={mods} sum={sum(mods.values()):6.2f}"
        )
    print("\n## Guest leaders (profile)")
    for cell in CELLS:
        generator, supports = cell.split("-")
        r = summary[f"ordinary-base-{generator}-{supports}-profile"]
        leaders = sorted(
            r["guest"].items(), key=lambda kv: kv[1]["share_pct"], reverse=True
        )[:3]
        print(
            f"{cell:11s} per_layer={r['phases_ms']['per_layer'] / 1000:6.2f}s "
            f"fuel={r['fuel_total']} "
            + ", ".join(
                f"{k}={v['share_pct']:.2f}% (self {v['self_fuel'] / 1e9:.1f}B)"
                for k, v in leaders
            )
        )
    print("\n## Host accumulated (profile)")
    for cell in CELLS:
        generator, supports = cell.split("-")
        r = summary[f"ordinary-base-{generator}-{supports}-profile"]
        print(cell)
        for mod, ns in sorted(r["host_accumulated_ns"].items(), key=lambda kv: -kv[1]):
            scopes = ", ".join(
                f"{k}={v / 1e9:.2f}"
                for k, v in sorted(
                    r["host_scopes_ns"][mod].items(), key=lambda kv: -kv[1]
                )
            )
            print(f"   {mod:28s} {ns / 1e9:8.2f}s  {scopes}")


if __name__ == "__main__":
    main()
