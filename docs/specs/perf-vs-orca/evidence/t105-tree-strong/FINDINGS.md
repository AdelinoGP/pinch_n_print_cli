# t105 — Supports-on engine-style fairness: Orca organic vs `tree_strong`

Route: [ticket 105](../../issues/105-supports-on-engine-style-fairness.md).
Measurement-only take at HEAD `81219a66` (tree carries this take's evidence and
the opt-in rig parameter only). OrcaSlicer **2.5.0-beta** (CLI log line, binary
`7663f5d3…`), source-manifest vendor tree `target/orca-vendor/`, four supports-on
cells, ordinary Orca only. The PnP side is untouched.

## Headline

**Pinning Orca's `support_style` to `tree_strong` costs Orca +39.6 % / +44.4 %
on the two base cells and +7.6 % / +9.5 % on the two benchy cells** (medians of
4 paired rounds; paired per-round medians +8.69 / +8.31 s base, +0.15 / +0.21 s
benchy). The pinned job is materially different: **+23.5 % to +35.3 % G-code
bytes**, with support-interface sections **8 → 57** (benchy classic-on) and
**230 → 423** (base classic-on).

If the pin is adopted, the four supports-on ratios move **−7.1 % to −30.8 %**
against the same PnP denominators — base classic-on **7.87x → 5.45x**, base
arachne-on **7.86x → 5.63x**, benchy classic-on **5.12x → 4.68x**, benchy
arachne-on **4.60x → 4.28x** (within-take baseline). Applying the measured
style multiplier to the scoreboard of record instead gives **8.25x → 5.71x** and
**7.00x → 5.01x** on base, **5.43x → 4.96x** and **4.78x → 4.45x** on benchy.
Either way the cells stay **4.3x–5.7x** from Orca: the park still needs
**65–73 %** of each cell removed.

## Method

- **Arms:** Orca `support_style` left at the vendor-resolved `default` (which
  canonical `SupportParameters` resolves to `smsTreeOrganic` on a `tree(auto)`
  job) versus `support_style = tree_strong` appended after the matched-job
  overrides. Every row's appendix is asserted against its arm, so an arm cannot
  silently inherit a version default. Both resolved values are recorded per run
  (`runs.csv`).
- **Schedule:** 1 warmup + 4 measured per arm per cell; one rig batch per run;
  the arms alternate order by round, so they interleave inside each cell
  (round 1 organic-first, round 2 strong-first, …). Median claimed; the standing
  starvation rule (cpu/wall below 0.75x the group's best) applies per arm, and
  **paired deltas use only rounds where both arms survived it**.
- **Denominators:** ticket 103's paired PnP ordinary medians (2026-10-31). The
  Orca side is re-measured here, so absolute ratios are mixed-vintage
  (disclosed below); the style price is paired and same-session.
- **Output disclosure:** bytes, SHA, TYPE/section counts, support and
  support-interface section counts per arm.

## Table 1 — per-cell medians (4 retained rounds unless noted)

| cell | organic wall | `tree_strong` wall | delta | delta % | organic cpu | strong cpu |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| base classic-on | 20.449 s | 29.532 s | **+9.083 s** | **+44.42 %** | 50.20 s | 102.91 s |
| base arachne-on | 20.514 s | 28.627 s | **+8.113 s** | **+39.55 %** | 57.35 s | 109.68 s |
| benchy classic-on | 2.422 s | 2.651 s | **+0.230 s** | **+9.48 %** | 9.22 s | 11.16 s |
| benchy arachne-on | 2.655 s | 2.857 s | **+0.202 s** | **+7.60 %** | 10.91 s | 12.69 s |

Benchy arachne-on's organic median rests on **2** retained rounds (see
Disclosures); every other cell/arm retained all 4.

## Table 2 — paired per-round deltas (strong − organic, same round)

| cell | rounds used | deltas (s) | median | sign |
| --- | --- | --- | ---: | --- |
| base classic-on | 1,2,3,4 | +7.048, +9.566, +5.392, +9.752 | +8.307 | 4/4 positive |
| base arachne-on | 1,2,3,4 | +8.750, +9.062, +6.391, +8.639 | +8.694 | 4/4 positive |
| benchy classic-on | 1,2,3,4 | +0.306, +0.237, +0.173, −0.152 | +0.205 | 3/4 positive |
| benchy arachne-on | 1,2 (3,4 starved) | +0.192, +0.100 | +0.146 | 2/2 positive |

The base sign is unanimous across all eight pairs and large enough to survive
the load events below. The benchy effect is small relative to that cell's
2.5 s-scale spread: one negative pair (organic r4 inflated 2.79 s against a
2.42 s median) and two starved organic samples in arachne-on.

## Table 3 — output disclosure, and what PnP's job resembles

| cell | arm | bytes | support sections | interface sections |
| --- | --- | ---: | ---: | ---: |
| base classic-on | organic | 33,366,060 | 622 | 230 |
| base classic-on | `tree_strong` | 44,803,778 | 739 | 423 |
| base classic-on | **PnP ordinary** | **42,507,912** | **431** | **70** |
| base arachne-on | organic | 32,363,932 | 619 | 226 |
| base arachne-on | `tree_strong` | 43,802,018 | 742 | 426 |
| base arachne-on | **PnP ordinary** | **42,330,265** | **431** | **70** |
| benchy classic-on | organic | 7,544,100 | 189 | 7 |
| benchy classic-on | `tree_strong` | 9,481,130 | 186 | 57 |
| benchy classic-on | **PnP ordinary** | **8,252,412** | **179** | **4** |
| benchy arachne-on | organic | 8,367,863 | 191 | 8 |
| benchy arachne-on | `tree_strong` | 10,331,360 | 188 | 58 |
| benchy arachne-on | **PnP ordinary** | **8,390,168** | **179** | **4** |

On the two base cells PnP's byte volume is **within 3.4–5.1 % of the
`tree_strong` arm** but **27.4–30.8 % above organic** — consistent with PnP
doing a Strong-class job (`DEV-156`), and with organic being the cheaper
material job Orca runs by default. On benchy the two arms bracket PnP (0.3–9.4 %
above organic, 13.0–18.8 % below strong), so the shape evidence is weaker there.

## Table 4 — ratio effect and the park

Within-take (same PnP denominator, today's organic arm as baseline):

| cell | organic ratio | strong ratio | ratio move | cut to park: organic → strong |
| --- | ---: | ---: | ---: | ---: |
| base classic-on | 7.87x | **5.45x** | −30.76 % | 80.9 % → 72.5 % |
| base arachne-on | 7.86x | **5.63x** | −28.34 % | 80.9 % → 73.4 % |
| benchy classic-on | 5.12x | **4.68x** | −8.66 % | 70.7 % → 67.9 % |
| benchy arachne-on | 4.60x | **4.28x** | −7.07 % | 67.4 % → 64.9 % |

Derived transfer to the scoreboard of record (t103 Orca medians × measured
style multiplier; disclosed as derived, not measured):
base classic-on **8.25x → 5.71x**, base arachne-on **7.00x → 5.01x**,
benchy classic-on **5.43x → 4.96x**, benchy arachne-on **4.78x → 4.45x**.

## Findings

1. **The style price is real and asymmetric.** Base supports-on pays ~40–44 %
   more Orca wall and ~2x Orca CPU; benchy pays 8–10 % wall. The base cells are
   the map's worst, so the pin's ratio effect is largest exactly where it
   matters most.
2. **The pin changes the job, not just the label.** Bytes rise 23.5–35.3 % and
   interface sections rise 8x on benchy. A scoreboard that adopts the pin is
   comparing a different Orca job; that must be disclosed beside every
   supports-on ratio.
3. **PnP resembles the strong arm on base.** Its output volume sits within
   3–5 % of `tree_strong` and 27–31 % above organic there, which is the
   output-level corroboration of `DEV-156`'s engine substitution.
4. **The pin moves the ratios, not the destination.** The two benchy cells land
   at 4.28x / 4.68x and the two base cells at 5.45x / 5.63x (within-take), or
   4.45x / 4.96x and 5.01x / 5.71x in the derived transfer. The park still needs
   65–73 % of each cell removed. This is a fairness fix, not a route step.
5. **Pinning is a protocol choice, not a neutral default.** Orca's canonical
   default on a tree family is organic; `tree_strong` is PnP's class, not
   Orca's. Adopting it makes the setting like-for-like at the cost of comparing
   against a non-default Orca job — the opposite trade from porting organic.

## Disclosures

- **Load.** `load.txt`: start 6.1 %, round 1 27.4 %, round 2 24.3 %, round 3
  **55.6 %**, round 4 13.9 %, end 8.8 %. The campaign was paused at the human's
  request between 23:58 and 00:51 and resumed; round 3 ran under the heaviest
  load.
- **Starvation.** benchy arachne-on organic r3 (wall 3.948 s, cpu/wall 2.758)
  and r4 (3.419 s, 2.984) fell below 0.75x the group's best ratio and are
  excluded from both the arm median and the paired deltas; r1/r2 are the clean
  pairs. benchy classic-on organic r4 (2.792 s, cpu/wall 3.386) passed the
  threshold but produced the one negative pair; it is retained and disclosed,
  not deleted. Base classic-on organic r3 inflated to 24.351 s under the same
  load event (cpu/wall 2.171) yet its pair stayed +5.392 s.
- **Mixed vintage.** PnP denominators are ticket 103's 2026-10-31 medians; the
  Orca arms are today's. Today's organic medians differ from ticket 103's Orca
  by +5.9 % (benchy classic), +4.0 % (benchy arachne), +4.9 % (base classic)
  and −10.9 % (base arachne, ticket 103's own noisy cell). Absolute ratios are
  therefore not a scoreboard revision; the paired style price is unaffected.
- **CPU.** cpu/wall is quoted per sample in `runs.csv`; the `tree_strong` arm's
  cpu/wall is higher (3.5–4.5 vs 2.4–4.1), i.e. the style does more work, not
  just slower work.
- No production edit, no config change, no parity port. The rig parameter now
  defaults to the pin (`tree_strong`, supports-on only); `-OrcaSupportStyle
  vendor` restores the vendor default used by the baseline arm here.

## Reproduction

`REPRODUCE.md` in this directory. Committed reduced set: `runs.csv` (40 rows),
`summary.json`, `provenance.json`, `load.txt`, `freshness.txt`. `verify.py
--self-test` exits 0 checkout-only (74/74 claims, 6/6 negative controls
rejected). Raw batch dirs, G-code and Orca logs are gitignored under
`target/matched-pair/t105-*`.
