# Repair the Orca rig so fresh Orca measurement is possible again

Type: task
Label: wayfinder:task
Status: resolved (2026-10-30; **rig repaired and Orca runs again — and the repair
falsified the archived Orca baselines' config**: the 2.5 install resolves
`--load-settings` through a vendor preset bundle, so the rig now loads a
version-gated source-manifest tree plus CLI overrides. On the benchy classic-off
cell it measures **1.978 s** median wall against the archive's **2.787 s**, because
the archived runs applied Orca's *defaults* for the whole filament chain
(`filament_density` 0, `filament_max_volumetric_speed` 2, `first_layer_temperature`
200) where the repaired rig resolves the vendor values (1.24 / 12 / 220). Re-baselining
Orca on all eight cells is route work, recorded in the map's fog; no ratio was
requoted here. See ## Answer)
Assignee: 2026-10-30 OpenCode wayfinder session — the human said "continue the
perf vs orca map"; the frontier re-derivation at HEAD `6c1a2334` found **97/97
children resolved**, the session offered six measured fog routes, and the human
selected **repair the Orca rig**
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: fog (the 2026-10-27 install blocker recorded in the map's Notes)

## Question

The 2026-10-27 OrcaSlicer install breaks every Orca run of the matched-pair rig.
What does the current CLI actually require, and can the rig be made to measure the
matched job on it without changing what the matched job means?

## Findings

The install is OrcaSlicer **2.5.0** (upstream tag `v2.5.0-beta`). Two changes landed
together:

1. `resources/profiles/BBL/` holds only non-preset assets; the preset tree ships as a
   single binary cache, `BBL.opc` (2,440,988 bytes, stamp `2.8.0.16`).
2. `--load-settings` resolves its argument against the **loaded preset bundle**, not
   against a bare JSON path.

Upstream `PresetBundle::resolve_preset_config` (`src/libslic3r/PresetBundle.cpp`,
`v2.5.0-beta`) is the contract: it matches a loaded preset by file identity, and on a
miss — only for a config declaring `"from": "system"` — walks up to the vendor
directory and loads the whole vendor tree **through the sibling `<Vendor>.json`
manifest**, which is the path that flattens `inherits`/`include`. That is upstream's
fix for [#14718](https://github.com/OrcaSlicer/OrcaSlicer/issues/14718), where path
loading silently used hardcoded defaults. With no manifest beside the preset, the walk
finds nothing and the CLI fails with `Preset was not found in the loaded bundle` —
which is exactly the error the map recorded.

The repair materializes that manifest tree from the upstream tag whose
`BBL.json` version equals the installed cache's stamp (`02.08.00.16` == `2.8.0.16`,
asserted at prepare time, fail-closed), and applies the matched-job keys as CLI
overrides, which the CLI ranks above loaded settings. The generated "derived" process
JSON is gone.

Two mechanical traps cost a run each and are recorded in the evidence: machine presets
must name a `printer_model` declared in `machine_model_list` and filament presets must
carry a `filament_id` (otherwise the whole vendor install is rejected), and a
relative-extruder machine needs `layer_change_gcode` (not `layer_gcode`) plus the `=`
form for boolean flags (`--enable-support 0` is read as an input file).

## Answer

**The rig measures Orca again, and the first real cell disagrees with the archive.**

Repair: `evidence/orca-rig-repair/prepare_orca_vendor.ps1` stages a
version-gated vendor tree at `target/orca-vendor/` (blobless sparse checkout of
`v2.5.0-beta`, 16 MB, plus the shipped `BBL.opc` beside the manifest);
`run_scoreboard.ps1` loads the vendor machine + process presets by path and applies
`--wall-generator`, `--wall-loops`, `--sparse-infill-density`,
`--sparse-infill-pattern`, `--enable-support`, `--layer-height`,
`--initial-layer-print-height`, `--enable-arc-fitting` as overrides. The rig's own
evidence validator gates every run and passed; three measured runs of the benchy
classic-off cell are exit 0 with identical byte counts.

Measured on that cell (median of three, warmup excluded):

| | archived 2026-10-25 | repaired 2026-10-30 |
|---|---|---|
| Orca wall | 2.787 s | **1.978 s** |
| process CPU | 9.27 s | 6.50 s |
| gcode bytes | 3,890,392 | 4,000,709 |

The repair **falsifies the archived Orca baselines' config**, which is the material
finding. The archived appendix (`evidence/t82-fairness-key-audit/appendix/`, same cell)
carries Orca's hardcoded defaults for the entire filament chain —
`filament_density = 0`, `filament_max_volumetric_speed = 2`,
`first_layer_temperature = 200` (all three are the documented defaults in
`docs/ORCA_CONFIG_REFERENCE.md`), while the process chain did resolve
(`inner_wall_speed = 300`). The repaired run resolves the filament chain
(`Generic PLA` → `fdm_filament_pla` → `fdm_filament_common`) and carries 1.24 / 12 /
220. The archived Orca therefore ran a 2 mm³/s volumetric cap and a 200 °C first
layer: **it was doing less work than the matched job asks for**, so PnP's recorded
multiples are optimistic for PnP, and every archived Orca number needs re-derivation
before any bound is quoted. No ratio is requoted by this ticket.

One protocol value needed pinning: OrcaSlicer 2.5 flipped `enable_arc_fitting`'s
**default** to enabled, and no preset in the matched chain sets it. The rig now passes
`--enable-arc-fitting=0`, restoring the t82 contract value rather than silently
adopting a version default for a feature PnP does not implement.

Cost note: the first run against a freshly staged tree is ~10 s slower in wall with
process CPU flat (first-touch work, not per-run: a warm tree runs at 2.0 s with no
cache beside the manifest). The rig's warmup absorbs it, so a `-Warmups 0` batch
against a fresh tree would be contaminated; the campaign must keep warmups.

Evidence: [`evidence/orca-rig-repair/`](../evidence/orca-rig-repair/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only, 4 claims, 7 negative controls
rejected). No production code touched; one evidence script and one rig change.
