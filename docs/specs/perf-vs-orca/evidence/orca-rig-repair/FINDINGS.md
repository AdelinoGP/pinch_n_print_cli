# Orca rig repair: 2.5.0-beta vendor-preset resolution

Ticket: [98](../../../../issues/98-orca-rig-repair.md) (wayfinder `task`).
Measured 2026-10-30 on the Windows dev machine, OrcaSlicer **2.5.0** (upstream tag
`v2.5.0-beta`, registry `DisplayVersion 2.5.0`).

## The failure

The 2026-10-27 install broke every Orca run of the scoreboard rig:

```
Slic3r::CLI::run(...):: can not resolve preset <machine>.json:
  Preset was not found in the loaded bundle
```

Two changes landed together in that install:

1. `resources/profiles/BBL/{machine,process,filament}` — the preset JSON tree — is
   gone. The vendor now ships as one binary preset cache, `resources/profiles/BBL.opc`
   (2,440,988 bytes, magic `ZCRO`, stamp `2.8.0.16`), beside a `BBL/` directory that
   holds only non-preset assets (cover PNGs, bed STLs).
2. `--load-settings` no longer loads a bare JSON path and flattens its `inherits`
   chain. It resolves the argument **against the loaded preset bundle**.

## Root cause, read from upstream source

`PresetBundle::resolve_preset_config` (`src/libslic3r/PresetBundle.cpp`, tag
`v2.5.0-beta`) is the whole story:

- it first looks for a **loaded** preset whose `preset.file` is byte-equivalent to the
  argument (`find_loaded`);
- on a miss, when the config declares `"from": "system"`, it walks up from the file to
  the nearest ancestor directory `V`, and if `V/../<V>.json` exists it loads that whole
  vendor tree through the canonical loader and re-tries there. That is the path that
  flattens `inherits`/`include` against real vendor values — the fix for upstream
  [#14718](https://github.com/OrcaSlicer/OrcaSlicer/issues/14718), where path loading
  silently used hardcoded defaults;
- otherwise it fails with `Preset was not found in the loaded bundle`.

The shipped install has no `<root>/BBL.json` manifest beside any preset tree, so the
manifest walk has nothing to find and the failure is exact and expected.

## The repair

Two pieces, both committed with this ticket.

**`prepare_orca_vendor.ps1`** materializes a manifest-backed vendor tree at
`target/orca-vendor/` (gitignored, regenerable):

- blobless sparse checkout of upstream tag `v2.5.0-beta`, taking only
  `resources/profiles/{BBL,BBL.json,OrcaFilamentLibrary,OrcaFilamentLibrary.json}`
  (16 MB);
- **provenance gate:** the tree's own `BBL.json` `version` must equal the stamp read
  from the installed `BBL.opc`, or the script fails closed and names the tag to use.
  Measured: installed cache `2.8.0.16` == upstream `02.08.00.16`. The rig therefore
  measures the vendor content the installed slicer itself would use, not a different
  upstream revision;
- the shipped `BBL.opc` is copied **beside** the manifest, so the vendor is served
  from the cache rather than re-parsed (see the price below);
- asserts the three presets the rig names are present (machine 162 / process 278 /
  filament 1,904 JSON files staged).

**`run_scoreboard.ps1`** now loads the vendor machine + vendor process presets by path
from that tree and applies the matched-job keys as CLI overrides
(`--wall-generator`, `--wall-loops`, `--sparse-infill-density`,
`--sparse-infill-pattern`, `--enable-support`, `--layer-height`,
`--initial-layer-print-height`, `--enable-arc-fitting`). The CLI ranks flags above
loaded settings, so the cell's four keys still win over the preset. The generated
"derived" process JSON is gone; two mechanical traps cost a run each while proving the
path and are recorded so nobody re-discovers them:

- the machine preset must name a `printer_model` declared in the manifest's
  `machine_model_list`, and a filament preset must carry a `filament_id`; without them
  the vendor install rejects the whole bundle ("can not find printer_model" /
  "can not find filament_id");
- relative-extruder machines need the key **`layer_change_gcode`** (not the older
  `layer_gcode`) to satisfy the `G92 E0` validation, and `--enable-support 0` (space
  form) is parsed as a positional input file — booleans need the `=` form.

## What the staged tree costs on first touch

The first run against a **freshly staged** JSON tree pays a large one-time cost, and
the rig's warmup absorbs it. Same cell, same command:

| run | wall | process CPU | gcode bytes |
|---|---|---|---|
| first touch of the staged tree (smoke) | **10.084 s** | 8.281 s | 3,012,777 ^ |
| batch warmup (same tree, fresh batch) | **11.684 s** | 7.125 s | 4,000,709 |
| measured runs after the warmup | **1.971 / 2.042 / 1.979 s** | 6.42 / 6.23 / 6.84 s | 4,000,709 |
| no cache beside the manifest, tree warm | **2.007 / 2.032 s** | 6.67 / 6.52 s | 4,000,709 |

^ the smoke row predates the arc-fitting pin, so its bytes are not comparable.

So the premium is ~8-10 s of extra **wall** with process CPU flat: first-touch work
that does not recur. A second arm with the cache deleted entirely and the tree warm
runs at 2.0 s, so the ~2.4k-preset parse is not re-paid per run and the `.opc` copy is
belt-and-braces rather than a requirement. Two consequences worth stating:

- **A `-Warmups 0` batch against a freshly staged tree would be contaminated** by
  roughly the whole steady-state cell time. The rig's warmup discipline is load-bearing
  here; the campaign that consumes this rig must keep `-Warmups 1` or greater.
- `run_scoreboard.ps1` still passes `-OrcaDebugLog` for `--debug 5 --logfile`, because
  CLI fatal messages reach the log file only — the stderr stream carries nothing
  actionable (that is how the original failure presented). The logfile is now always
  pinned into the run's `orca-out/<run_id>/orca-cli.log`: without it the CLI writes one
  1.1 MB `00000.log` into the process working directory, i.e. into the repo root, once
  per run.


## What the repair surfaced: the archived Orca baseline is config-divergent

The rig works now, and the first real cell does **not** reproduce the archive. Same
cell (`benchy`, `classic-off`, supports off), same rig, same preset names:

| | archived 2026-10-25 | repaired 2026-10-30 |
|---|---|---|
| Orca wall (median of 3) | 2.787 s | **1.978 s** |
| process CPU | 9.27 s | 6.50 s |
| gcode bytes | 3,890,392 | 4,000,709 |
| `inner_wall_speed` | 300 | 300 |
| `filament_density` | **0** | **1.24** |
| `filament_max_volumetric_speed` | **2** | **12** |
| `first_layer_temperature` | **200** | **220** |
| `enable_arc_fitting` | 0 | 0 (now pinned; 2.5's default is *enabled*) |

The four bold rows are Orca's **hardcoded defaults**, per
`docs/ORCA_CONFIG_REFERENCE.md` (`filament_density` 0, `filament_max_volumetric_speed`
2, `first_layer_temperature` 200). The archived runs therefore applied the process
chain (300 mm/s walls, so the process leg resolved) but **not** the filament chain:
`Generic PLA.json` inherits `fdm_filament_pla` → `fdm_filament_common`, and that chain
never reached the output. The repaired rig resolves it, which is why Orca is ~29 %
faster on this cell and emits more bytes.

Consequences for the map, stated plainly:

- **Archived Orca medians are not reproducible on this install and were measured with
  an under-resolved filament config.** Every recorded ratio was computed against an
  Orca that ran a 2 mm³/s volumetric cap with density 0 and 200 °C first layer. The
  direction of the defect is known: Orca was doing *less* work than the matched job
  asks for, so PnP's real multiples are **worse** than recorded, not better.
- `enable_arc_fitting` flipped to **enabled** by default in 2.5. No preset in the
  matched chain sets it, so without the pin the repaired rig would start emitting G2/G3
  arcs — a feature PnP does not implement. The pin restores the t82 contract value;
  it does not introduce a new setting.
- No ratio was requoted by this ticket. Re-baselining Orca on all eight cells is
  route work and is recorded in the map's fog.

## Reproduction

```bash
# 1. vendor tree (network + git; needs the OrcaSlicer install for the version gate)
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/orca-rig-repair/prepare_orca_vendor.ps1

# 2. the measured cell (Orca only; the rig's own evidence validator gates each run)
pwsh -NoProfile -Command "& 'docs/specs/perf-vs-orca/evidence/matched-pair/run_scoreboard.ps1' -Fixtures 'benchy' -Cells 'classic-off' -Tools 'orca' -Runs 3 -Warmups 1 -SkipFreshness -BatchId t98-benchy-classic-off -KeepGcode Last"
```

`verify.py --self-test` re-derives every headline claim from the files in this
directory and requires each negative control to be rejected; it exits 0 with no
network and no OrcaSlicer install.

Both supports states pass the rig's own evidence validator on the repaired path, which
is the half of the matrix the process-preset override could most easily have broken
(`support_type` comes from the vendor preset, not from an override):

| cell | wall | process CPU | gcode bytes | appendix |
|---|---|---|---|---|
| benchy classic-off (median of 3) | 1.978 s | 6.50 s | 4,000,709 | `enable_support = 0`, `support_type = tree(auto)` |
| benchy classic-on (1 measured) | 2.598 s | 9.81 s | 7,544,998 | `enable_support = 1`, `support_type = tree(auto)` |

## Files

- `prepare_orca_vendor.ps1` — the vendor-tree materializer and provenance gate.
- `rows-benchy-classic-off.csv` — the measured batch: warmup + three measured runs.
- `rows-benchy-classic-off-no-cache.csv` — two runs with **no** `BBL.opc` beside the
  manifest and the tree warm, proving the first-touch premium does not recur.
- `rows-benchy-classic-on.csv` — the supports-on cell: warmup + one measured run, to
  show the vendor `support_type` survives the override path.
- `appendix-benchy-classic-off-orca-2.5.0-beta.txt` — the resolved-config appendix of
  the repaired run (738 keys).
- `verify.py` — claim checks + self-test.
