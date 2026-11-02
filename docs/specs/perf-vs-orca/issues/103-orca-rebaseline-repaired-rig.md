# Orca re-baseline on the repaired rig

Type: task
Label: wayfinder:task
Status: resolved
Assignee: 2026-10-31 OpenCode wayfinder session — the human said "continue the
perf vs orca map" (no ticket named), and this was the only open child
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Repair the Orca rig](98-orca-rig-repair.md) ("The archived Orca
side must be re-baselined before any ratio is quoted" — the map's Not yet
specified entry, surfaced 2026-10-30; resolved by this ticket on 2026-10-31/11-01)

## Ruling — settled 2026-10-31

**Human: "Pinned back."** OrcaSlicer 2.5's version-flipped defaults are **not**
accepted as the new matched job; they are pinned to the 2.4-era values so a
version bump cannot silently change the job.

| key | 2.4-era / matched value | 2.5.0 default | why the pin is required |
| --- | --- | --- | --- |
| `enable_arc_fitting` | **0** | enabled | PnP has no arc-fitting feature; already pinned by [ticket 98](98-orca-rig-repair.md) |
| `elefant_foot_compensation` | **0** | 0.15 | PnP has no elephant-foot feature — the key exists only in the viewer padding table (`ORCA_CONFIG_PADDING` in `crates/slicer-gcode/src/serialize.rs`), no module consumes it |
| `brim_width` | **0** | 5 | 2.4 printed no brim, so the archived comparison had none on Orca's side |

The three pins land in `New-OrcaOverrideArgs`
(`docs/specs/perf-vs-orca/evidence/matched-pair/run_scoreboard.ps1`), beside the
existing `--enable-arc-fitting=0`. `t98`'s `verify.py` asserts only the arc pin;
it must be extended to assert all three resolved values from each run's
appendix, so a future version bump fails closed instead of drifting silently.

Pinning to the **2.4-era** value rather than to 2.5's is deliberate beyond
consistency: it keeps the re-baselined job identical to the archived one, so the
only difference between the archive and the fresh Orca side is the **filament
leg** t98 repaired. Any remaining gap change is then attributable to that
repair, not to a second, unannounced change in the job.

**One asymmetry this ruling does not remove, and the measurement must disclose
it.** The pin governs the Orca side only. On the PnP side the matched config
sets no `brim_width`, so the skirt-brim module's own default of **8.0 mm**
applies (`modules/core-modules/skirt-brim/skirt-brim.toml`), and the cell really
prints it: the ordinary base classic-on reference G-code opens layer 1 with
**26 concentric rectangles** and **12,040 mm** of first-layer extrusion against
**2,167 mm** on layer 2, and its emitted config block reads `brim_width = 8`.
So pinned-back Orca prints no brim while PnP prints an 8 mm one — the same
adhesion asymmetry the archive already had, and it is **a different key from the
filament leg** t98 repaired, with a different cost and a different direction. Do
not fold the two into one "Orca was flattered" story: the filament leg changes
which config chain resolves, this one changes who prints adhesion geometry, and
the re-baseline measures their net rather than asserting it.

Its slice-time price is small — `com.core.skirt-brim` reports **235 ms** for the
whole print, on one of 495 layers — but the job is not identical and the
scoreboard must say so.

Do **not** fix this by adding `brim_width` to the matched config in this ticket:
that changes the PnP side of every cell and invalidates the whole
`evidence/t91-exact-z-memo/references.json` byte set. If parity on adhesion
geometry is wanted, it is its own take with its own reference re-derivation,
and it needs a human decision because it changes the job both arms are measured
under.

## Question

**Every archived Orca median is a measurement of the wrong job.** The repaired
rig ([ticket 98](98-orca-rig-repair.md)) measured benchy classic-off at
**1.978 s** median wall where the archive records **2.787 s**, because the
archived runs applied Orca's **hardcoded filament defaults**
(`filament_density` 0, `filament_max_volumetric_speed` 2,
`first_layer_temperature` 200) while the repaired rig resolves the vendor chain
(`Generic PLA` → `fdm_filament_pla` → `fdm_filament_common` → 1.24 / 12 / 220).
The process chain resolved in both eras, so this is specifically the filament
leg; the direction of the defect is **unfavourable to PnP** — the archived Orca
did *less* work, so every recorded multiple is optimistic for PnP.

Which cells does PnP actually lose, and by how much, measured today on both
sides?

This is the map's destination question in its measurable form. The destination
is 8/8 cells won under the matched-pair protocol; the last scoreboard
([ticket 72](72-scoreboard-refresh.md), 2026-10-18) reported Orca winning all
8 cells at **3.30x–18.03x**, but that table predates t81's resolution port, t91's
exact-Z memo, t95's batching, and t102's mimalloc KEEP — and its Orca side is
the falsified one.

## What must happen

1. **The ruling — SETTLED 2026-10-31 ("pinned back"); the pins are specified in
   ## Ruling above.** No Orca run is comparable until `New-OrcaOverrideArgs`
   (`docs/specs/perf-vs-orca/evidence/matched-pair/run_scoreboard.ps1`) carries
   all three pins and the verifier asserts their resolved values.
2. **The measurement (AFK).** Re-measure **Orca on all 8 cells** on the repaired
   rig and re-derive the PnP side from fresh paired runs, so both arms are
   today-vs-today. The vendor tree is staged by
   `evidence/orca-rig-repair/prepare_orca_vendor.ps1` → `target/orca-vendor/`,
   and every campaign must keep `-Warmups 1` or greater (a first-touch vendor
   tree costs ~10 s of one-time wall with process CPU flat — t98's method note).
3. **The scoreboard and the park bound.** A re-derived per-cell table (median
   wall, process CPU corroborating, floor multiples) and a re-derived
   [Gap budget per cell](12-gap-budget-per-cell.md) park condition: which cells
   are within reach of in-map route work and which are beyond it.

## Proof gate

- 16 PnP groups `degraded=false`, ordinary ≡ accelerated bytes in all 8 cells.
- Orca and PnP measured on the same day, paired and alternating, with the
  vendor-chain resolution asserted (not assumed) in the Orca side's log.
- Every Orca run records the resolved `filament_density`,
  `filament_max_volumetric_speed` and `first_layer_temperature`, so the
  falsified-archive defect cannot recur silently — **and** the resolved
  `enable_arc_fitting` = 0, `elefant_foot_compensation` = 0 and `brim_width` = 0,
  asserted in the run's own appendix so the pins fail closed rather than relying
  on a reviewer remembering them.
- The scoreboard states the standing adhesion asymmetry measured in ## Ruling
  (PnP prints an 8 mm brim, pinned-back Orca prints none) beside the numbers,
  not as a footnote.
- Both guest freshness gates exit 0.
- Zero starvation exclusions, or each one named and its cell re-run whole
  (the t57 discipline).

## Deliverables

1. The ruling record (which 2.5 defaults are the matched job, which are pinned).
2. The fresh 8-cell scoreboard with both arms today-vs-today, and the archived
   numbers shown beside it marked **superseded**.
3. The re-derived park bound: which cells are above or below the fold, and the
   route options that follow from it.
4. No production edit, no auto-commit. This ticket moves no PnP code; it decides
   where the remaining route work goes.

## Answer (resolved 2026-10-31 campaign, 2026-11-01 noise disclosure)

**Measured on both sides today-vs-today: Orca still wins all 8 cells, at
3.52x–8.25x.** Both standing trees were rebuilt from HEAD `c135e3e7` (ordinary
`4cb96942…`, accelerated `80897e62…`, 24/24 external modules each, both guest
freshness gates exit 0); Orca is the 2.5.0 install served from the
version-gated vendor tree (installed `2.8.0.16` == upstream `02.08.00.16`).

| Cell | Orca | PnP ordinary | PnP accelerated | gap ord | gap acc | reduction needed to park | floor multiples |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| benchy classic-off | 2.03 | 7.68 | 7.67 | **3.78x** | **3.78x** | 60.3 % | 75x |
| benchy arachne-off | 2.19 | 7.69 | 7.84 | **3.52x** | **3.59x** | 57.4 % | 72x |
| benchy classic-on | 2.29 | 12.40 | 12.57 | **5.43x** | **5.50x** | 72.4 % | 90x |
| benchy arachne-on | 2.55 | 12.22 | 12.40 | **4.78x** | **4.85x** | 68.7 % | 86x |
| base classic-off | 15.28 | 75.38 | 75.74 | **4.93x** | **4.96x** | 69.6 % | 87x |
| base arachne-off | 16.72 | 78.89 | 76.51 | **4.72x** | **4.58x** | 68.2 % | 85x |
| base classic-on | 19.49 | 160.86 | 160.09 | **8.25x** | **8.21x** | 81.8 % | 102x |
| base arachne-on | 23.03 | 161.14 | 173.47 | **7.00x** | **7.53x** | 78.6 % | 98x |

**The archived table is superseded and its error was not one-directional.**
Fresh Orca is **−24…−35 %** on the four benchy cells (the t98 filament-leg
repair, as expected: PnP's benchy multiples worsen, best cell 2.71x → 3.52x)
but **+6…+18 %** on the four base cells, where PnP's multiples *improve* as a
ratio (base classic-on 9.31x → 8.25x, base arachne-on 9.36x → 7.00x). Ticket
98's one-cell extrapolation "every recorded multiple is optimistic for PnP" is
therefore **falsified for the four base cells**; only measurement could settle
it, and no mechanism for the base slowdown is asserted. The PnP side also
carries t102's mimalloc KEEP, which is why base supports-on PnP walls fall
6–14 % while benchy is flat (−3 %…+0.1 %).

**Deliverable 1 (the ruling) is enforced, not remembered.** The three pins land
in `New-OrcaOverrideArgs` beside the arc pin, and `Test-OrcaEvidence` asserts
the resolved `enable_arc_fitting = disabled`, `elefant_foot_compensation = 0`
and `brim_width = 0` in **every** Orca run's own appendix, plus the resolved
filament leg (`1.24 / 12 / 220`) and outright rejection of the archived
hardcoded-default signature (`0 / 2 / 200`). All 32 Orca runs resolved the six
values; a future install or preset bump fails the batch. Ticket 98's
`verify.py` was extended to pin the two 2.5 defaults its archived appendix
carries (`brim_width = 5`, `elefant_foot_compensation = 0.15`) as the pre-pin
record the ruling supersedes (5 claims, 9/9 negative controls).

**Deliverable 3 (the park bound) is re-derived and still binding.** The
nearest cell is benchy arachne-off at 3.52x / 3.59x; every cell needs a
**57–82 %** wall cut to reach ~1.5x, i.e. **72–102x** the map's whole 0.80 %
floor. The release bound is 3.52x versus 2026-10-25's 2.71x, and the cause is
the Orca side moving fast on benchy, not a PnP regression. Supports-on remains
the concentration (+36…+67 % over its supports-off sibling). The route options
that follow (supports-on geometry, the organic support engine `DEV-156`, the
brim job-definition decision) are recorded in the scoreboard and the map, not
graduated as tickets — the take order is the human's.

**The adhesion asymmetry is measured and disclosed beside the numbers.** PnP
resolves `brim_width = 8` in all 16 PnP groups; pinned-back Orca resolves `0` in
all 32 runs. Its slice-time price (~235 ms) is far below any floor, but the
job is not identical, and closing it changes the PnP side of every cell and
re-derives the reference byte set — a separate take with its own human
decision, as the ruling requires.

**Proof gate.** PnP 16/16 groups `degraded=false` / `non_fatal=0`; ordinary ≡
accelerated byte for byte in all 8 cells; the PnP output is **exactly** the t87
post-t81 reference set in all eight cells (so the PnP job is unchanged and only
Orca moved); both freshness gates exit 0. **Two starvation exclusions are
named** (`base-arachne-on-orca-m3`, `base-arachne-on-pnp-ordinary-m3`) and three
whole-cell re-runs were attempted; all three were spoiled by external load (a
competing integration-test binary in another checkout, then sustained browser
CPU at 78–96 %). The human was asked and instructed the take to proceed with
the noise disclosed; all three spoiled CSVs are preserved. That cell's
two-clean-round band is 6.95–7.05x ordinary / 7.23–7.53x accelerated and its
verdict is unaffected.

Evidence: [`evidence/t103-orca-rebaseline/`](../../evidence/t103-orca-rebaseline/SCOREBOARD.md)
(`verify.py` exits 0 checkout-only, 13/13 negative controls detected; no
production edit, no auto-commit).
