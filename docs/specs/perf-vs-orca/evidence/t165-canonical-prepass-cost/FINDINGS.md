# t165: canonical prepass cost on OrcaSlicer 2.5.1

Ticket: [Verify canonical's prepass cost](../../issues/165-canonical-prepass-cost.md).
Take: 2026-11-18/19, OrcaSlicer 2.5.1 `orca-slicer.exe` `5013b1bd…` (t161's
boundary, stamped on all 24 rows). No PnP run and no PnP change. Harness:
[`run_tap.py`](run_tap.py), [`reduce.py`](reduce.py), [`verify.py`](verify.py).
Every number here re-derives from [`runs.csv`](runs.csv) into
[`reduced.json`](reduced.json).

## Verdict

**The input holds. t110's canonical prepass reproduces on 2.5.1 within
−0.7…+3.6 % on the three point classes, so t164's 8/8 projection stands.** The
binding cell is still `benchy arachne-on`, at **0.898x** (t164: 0.890x), and its
slack on the new input is **1.47x** (t164: 1.53x). The benchy supports-on
prepass is **569.7 ms** against t110's 549.8 ms (1.036x), far inside the 1.53x
the ticket asked about.

## Per class (medians of 3 measured runs; classic generator)

| class | canonical prepass | t110 (2.5.0) | ratio | label | worst poll gap | per-round prepass | prepass CPU | emit | Orca wall |
| --- | ---: | ---: | ---: | --- | ---: | --- | ---: | ---: | ---: |
| benchy-off | 104.7 ms | 50.5 ms | 2.07x | **bound** | 56.8 ms | 81.9 / 89.8 / 117.6 | 1.83 s | 364.4 ms | 1,880.0 ms |
| benchy-on | 569.7 ms | 549.8 ms | 1.036x | point | 32.4 ms | 561.5 / 554.8 / 578.0 | 6.17 s | 496.9 ms | 2,478.1 ms |
| base-off | 2,172.4 ms | 2,103.0 ms | 1.033x | point | 20.6 ms | 2,293.6 / 2,616.7 / 1,890.6 | 14.03 s | 1,473.0 ms | 13,943.4 ms |
| base-on | 10,368.1 ms | 10,444.8 ms | 0.993x | point | 45.0 ms | 10,650.5 / 10,368.1 / 10,032.5 | 79.52 s | 3,461.2 ms | 24,163.1 ms |

Prepass is the median launch→`plate_1.gcode.tmp` time minus the median `--info`
wall: **1,177.2 ms** for benchy and **10,038.9 ms** for base. The `--info` runs
are in the same rounds. The label follows t110's rule: below 3x the class's worst
poll gap, a value is a bound. benchy-off's 104.7 ms is below 170.4 ms, so it is a
bound, as in t110. Both values are bounds, so their 2.07x ratio is not a
movement.

## Slack table re-derived (ESTIMATE; t164's arithmetic, only the canonical input changed)

| cell | prepass budget | slack (t110) | slack (t165) | at canonical prepass (t110 → t165) | window-scaled |
| --- | ---: | ---: | ---: | ---: | ---: |
| benchy classic-off | 0.791 s | 15.67x | 7.56x | 0.611x → 0.639x | 0.640x |
| benchy classic-on | 0.874 s | 1.59x | 1.53x | 0.874x → 0.882x | 0.890x |
| benchy arachne-off | 0.780 s | 15.44x | 7.45x | 0.649x → 0.675x | 0.676x |
| benchy arachne-on | 0.840 s | 1.53x | **1.47x** | 0.890x → **0.898x** | 0.906x |
| base classic-off | 11.022 s | 5.24x | 5.07x | 0.400x → 0.405x | 0.415x |
| base classic-on | 15.690 s | 1.50x | 1.51x | 0.786x → 0.783x | 0.789x |
| base arachne-off | 12.014 s | 5.71x | 5.53x | 0.396x → 0.400x | 0.409x |
| base arachne-on | 18.228 s | 1.75x | 1.76x | 0.718x → 0.715x | 0.720x |

**8/8 still win at canonical prepass cost, binding cell `benchy arachne-on` at
0.898x.** The "window-scaled" column is a sensitivity check. It multiplies the
new input by t164's Orca classic wall over this take's (1.013–1.067x), on the
assumption that the Orca arm was proportionally slower in t164's window. It
does not flip any cell (worst 0.906x).

## Correctness controls

- **Wall accuracy:** the tap's wall agrees with `GetProcessTimes` (creation→exit,
  handle held from launch) within **0.861 %** on every run, below the 1 %
  bar. The tap is consistently 10–27 ms late, which is the poll cost.
- **Matched job:** every slice run asserts the scoreboard rig's
  `Test-OrcaEvidence` key set from its resolved appendix: generator, loops,
  infill, support type/style, layer height, arc fitting, elephant foot,
  brim 8/`outer_only`, skirt 6/3/1, initial line width 0.4, support brim 0, and
  the t98 filament signature. There were no rejections.
- **Boundary stability:** the emit phase (tmp→rename) reproduces t110's
  (364.4 / 496.9 / 1,473.0 / 3,461.2 ms against 365.8 / 497.5 / 1,349.1 /
  3,230.6 ms). So `plate_1.gcode.tmp` still opens at the end of Orca's slicing on
  2.5.1.

## Disclosures

- **Resolution did not improve.** `timeBeginPeriod(1)` was set, but the worst gaps
  were 16.6–56.8 ms, which is t110's class. Windows 11 appears not to honour the
  request for this process. That is unverified; only the gaps were measured.
- **Base prepass carries the `--info` spread.** The base `--info` runs span 416 ms
  (9,904.5–10,320.7), so the per-round base-off prepass spans 726 ms. The median
  is the value of record, and its uncertainty is that spread, not the poll gap.
- **Matched job differs from t110's.** t110 ran t103's keys (brim 0, no skirt
  pins). This take ran the current t130/t135 adhesion job. The values agree
  anyway.
- **Arachne cells reuse the classic tap.** The prepass is a function of fixture
  and supports (t110 measured it identical across generators), so the arachne
  cells take their class's value, exactly as t164 did.
- **`--info` is still an upper bound on import**, so canonical's prepass is still
  a lower bound and the projection is optimistic in the same direction as t110's.
- **Load:** system CPU utilisation during runs was 13–50 % (Orca's own work
  included). There were no other heavy processes. The browser was open.
- **Cross-window composition.** The slack table combines this take's input with
  t164's walls and shares, as the ticket specifies. It is not a ratio of
  record.
- **A side observation, not a claim.** This window's Orca walls (1.880 / 2.478 /
  13.943 / 24.163 s) are within −1.3…+0.9 % of t110's 2.5.0 walls (1.868 / 2.510 /
  13.815 / 24.140 s). t161 read Orca's +5.4…+24.9 % movement as likely the
  2.5.1 binary. Same-binary walls this close to 2.5.0's weaken that reading,
  though one window cannot settle it.

## Reproduce

```bash
python run_tap.py --repeats 3 --out runs.csv --rawdir ../../../../../target/t165-runs
python reduce.py
python verify.py            # PASS
python verify.py --self-test
```
