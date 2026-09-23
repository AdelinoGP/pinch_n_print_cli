# DEV-174 repair evidence — perf-vs-orca ticket 14 (2026-09-23)

Raw heavy artifacts (G-code, instrumented JSONL streams) stay in gitignored
`target/` per the map's evidence policy; everything cited by the ticket is
either committed here or regenerable with the commands in §Reproduction.

## Acceptance — `slice_complete` (base.stl, supports-on)

Before (pre-fix binary):

```json
{"schema_version":"1.5.0","event":"slice_complete","status":"ok","elapsed_ms":487433,"degraded":true,"fatal_error_count":0,"non_fatal_error_count":172181}
```

After (fix):

```json
{"schema_version":"1.5.0","event":"slice_complete","status":"ok","elapsed_ms":669545,"degraded":false,"fatal_error_count":0,"non_fatal_error_count":0}
```

Matched-pair rows: `results/dev174-before.csv` (one run reproduces the
172,181 non-fatals in both PNP modes) and `results/dev174-after.csv` (both
generators × both PNP modes: `degraded=false`, `non_fatal_error_count=0`,
exit 0). Ticket-11 baseline rows: `../matched-pair/results/s2-base.csv`.
Per-run TYPE counts also live in those batches' `stats/` (regenerable).

| Counter (PNP, classic/arachne identical) | Before | After | Orca |
| --- | --- | --- | --- |
| `Support` sections | 280 | 431 | 638–639 |
| `Support interface` sections | 55 | 70 | 223–225 |
| Layers 108–258 with zero `Support` sections | 151 | 0 | — |
| All layers with zero `Support` sections | 215 / 495 | 64 / 495 | — |
| Every other TYPE counter | — | bit-identical (Inner wall 525/527 → 523/526, documented jitter) | — |

## Probe — root-cause evidence (`probe-rejections.log`)

151 rejected entries covering layers 108–258 (the contiguous DEV-174 band),
one entry per `(layer, region 0, object)` carrying 812–1,438 demands each
(Σ = 172,181 unmet demands, every one `body rejected: max-body-extent
violation`). Span extremes across the rejected entries: widest single body
cross-section **1,111,286 × 429,575 units** (111.13 × 42.96 mm); widest
multi-body envelope **1,113,301 × 504,734 units**; the cap was `1 << 20` =
1,048,576 units (104.86 mm). Two mechanisms fired: whole-entry envelope
measurement of packed identity aggregates (e.g. layers 255–258: no single
region over 901,608 units, union over the cap) and single fused
cross-sections marginally over the cap (layers 108–254).

## Stage-level before/after (instrumented both sides — PERF-HANDOFF §3)

| Phase (wall) | Before | After |
| --- | --- | --- |
| prepass | 349.5 s | 455.7 s |
| per_layer | 107.9 s | 152.8 s |
| postpass | 25.1 s | 52.2 s |

| Module (summed worker elapsed) | Before | After |
| --- | --- | --- |
| `com.core.tree-support` | 66.4 s | 143.8 s |
| `com.core.tree-support-planner` | 182.7 s | 210.9 s |
| `com.core.classic-perimeters` | 1,105 s | 1,513 s |
| `com.core.infill-linker` | 53.5 s | 75.9 s |

Reading: growth concentrates in support-emitting work (151 restored layers)
while unrelated stages moved the same way (`classic-perimeters` +37% on walls
a support gate cannot affect) — the §10.3 external-load signature. The fix
adds no cost class (per-region bbox scan is the same O(regions)). No
whole-slice wall claims: after-row walls are single runs under that drift and
the ±13% base.stl spread (§3.6).

## Visual verification (`visual/`)

Before/after `filled_areas` at the same band layers (`layer_z` 30.2 /
46.2 mm; entries `final_gcode_filled_areas_l150`/`l230`, warnings limited to
`M73` time lines). The after renders add exactly the support classes —
packed column field under the flared-hull perimeter and deck overhang,
isolated tree columns in interior pockets, interface bands under the deck —
entirely in cross-section space the before renders show empty; model
walls/infill are unchanged between pairs (matching the bit-identical model
TYPE counters). Palette caveat: `gcode_role_color`
(`crates/slicer-runtime/src/visual_debug_style.rs`) FNV-hashes roles into 6
colors and `Support` collides with `Outer wall` at rgb(220,50,47) — the
identification rests on the before/after differential, not on color. The
documented G-code-source silhouette is rejected by this build (`silhouette is
not supported for tap 'final_gcode'`); vertical continuity is covered by the
per-layer section scan (151/151 band layers carry `Support`).

## Reproduction

```bash
cargo xtask build-guests --check                      # must exit 0
# matched-pair rows (before/after binaries):
pwsh docs/specs/perf-vs-orca/evidence/matched-pair/run_scoreboard.ps1 `
  -Cells classic-on -Fixtures base -Runs 1 -Warmups 0 -BatchId dev174-before
pwsh docs/specs/perf-vs-orca/evidence/matched-pair/run_scoreboard.ps1 `
  -Cells classic-on,arachne-on -Fixtures base -Runs 1 -Warmups 0 -BatchId dev174-after
# instrumented stage pair (both sides instrumented, attribution only):
target/release/pnp_cli.exe slice --model tmp/base.stl \
  --config docs/specs/perf-vs-orca/evidence/matched-pair/configs/pnp-classic-supports-on.json \
  --module-dir modules/core-modules --output target/after-instr.gcode \
  --instrument-stderr 2> target/after-instr.jsonl
# visual bundles (gcode mode):
target/release/pnp_cli.exe visual-debug --request <req.json> --output <bundle>
```
