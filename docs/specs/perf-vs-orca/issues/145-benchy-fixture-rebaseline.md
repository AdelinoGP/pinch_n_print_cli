# Benchy fixture replacement: re-record the benchy reference set and retire cross-swap benchy ratios

Type: task / measured re-baseline
Label: wayfinder:task
Status: resolved (2026-11-13; measured re-baseline — the replacement benchy is a
  **tessellation-only** re-export (225,786 → 225,706 triangles, |Δbounds| ≤
  4.05e-7 mm, ten of ten sampled cross-sections equal to t138's recorded
  old-export values), but the **slice is not output-neutral**: one
  internal-bridge layer (L40, z = 8.2) moves **+57.298 mm** of path (**+13.694 %**
  of area) in all four cells, and the tree-support engine moves
  **−1,370.181 mm** (**−2.238 %**) over 176 supports-on layers. The four base
  cells are byte-identical across the two takes — the control that makes the
  benchy delta a fixture effect. The active benchy currency is t144's set
  (**1ae0cfa8… / d9fc80ba… / 8faf02b1… / cd7d4a2d…**), reproduced in 8/8 cells
  today; t139's four hashes are retired. **Post-swap scoreboard (8 cells ×
  2 tools, one window): 1.853x–3.455x ordinary/Orca, required cut 46.03 %–71.06 %**
  (19.05 %–56.59 % for 1.5x); the accelerated timing arm is not taken (the
  controlled host still fails in `libmimalloc-sys`). See ## Answer)
Blocked by: none
Assignee: 2026-11-13 OpenCode wayfinder session — the human said "Continue the
  perf vs orca map, pick an AFK ticket"; 145 is the first unblocked, unclaimed
  AFK frontier ticket (146 is the human's HITL grilling).
Parent: [Slice performance vs OrcaSlicer](../map.md)
Graduated from: [Union-phase request grouping](143-union-phase-request-grouping.md)'s
  2026-11-12 take. The user-supplied `tmp/3dbenchy.stl` was lost and replaced with
  a different export of the same model (240 layers / 48.0 mm, ~225.7k
  triangles). `tmp/base.stl` is byte-identical to the original — t139's four
  base hashes still reproduce **exactly** — so only the benchy half of the
  reference currency moved.

## Question

`tmp/3dbenchy.stl` is a different export from the one every prior benchy number
was measured on. The replacement prints the same model (240 layers, 48.0 mm,
first layer equal) but emits different G-code: benchy classic-on hashes
`dceb7a48…` where t139 recorded `b3f1eccc…`. What is the benchy reference
currency now, which pre-swap benchy claims survive, and what does the map quote
until a post-swap scoreboard exists?

Decide and record, in order:

1. **The new benchy reference set.** [t143](143-union-phase-request-grouping.md)'s
   output campaign already recorded all four benchy hashes from the baseline
   arm on the replacement fixture: `a89e449b…` (classic-off), `dceb7a48…`
   (classic-on), `3286449f…` (arachne-off), `be786718…` (arachne-on). Promote
   them from a local disclosure to the active set, in whatever artifact the map
   treats as the currency ([t139](139-layer-plane-parity.md)'s set is the
   precedent).

   **Update (2026-11-12, [t144](144-brim-anchor-reconstruction.md)):** the
   active set is now t144's candidate set
   ([`evidence/t144-brim-anchor/reference.json`](../evidence/t144-brim-anchor/reference.json)),
   not t143's baseline hashes — the anchor reconstruction moved four of the
   four benchy cells (`1ae0cfa8…` / `d9fc80ba…` / `8faf02b1…` / `cd7d4a2d…`) and
   is byte-identical on both build arms. Treat t143's four hashes as the
   pre-t144 baseline, not the currency. The remaining items (2–5) are unchanged.
2. **What the swap actually changes.** A delta audit between the old and new
   export cannot be taken directly (the old file is gone); reconstruct it from
   the retained t139 G-code if a copy exists, otherwise from the measured
   invariants (layer count, first-layer height, per-layer areas on the retained
   evidence) and say plainly which claims are re-derivable and which are not.
3. **Retire the pre-swap benchy claims.** Every benchy G-code hash, layer-0
   cross-section area, brim/skirt oracle band, A/B pair and scoreboard median
   measured against the old export is historical. Base claims are unaffected.
   The map must mark them rather than leave them quotable.
4. **The post-swap benchy scoreboard.** Orca's four benchy cells must be sliced
   from the replacement file with the matched keys pinned and asserted per run,
   then paired against PnP's four benchy cells. Until that exists, no benchy
   ratio of record may be quoted; the base-only ratios stand with the
   machine-state disclosure.
5. **The disclosure rule.** A benchy number is either pre-swap (historical) or
   post-swap (current); never mixed, never quoted across the boundary.

## Pre-declared gate

This ticket is a re-baseline, not a candidate: no performance gate. Its
correctness bar is the fairness contract already used by
[t130](130-brim-matched-settings-ruling.md) / [t139](139-layer-plane-parity.md):

- the four benchy hashes are recorded and reproduce on both build arms of the
  take that records them;
- the Orca rig pins the matched keys on every benchy run
  (`enable_arc_fitting = disabled`, `elefant_foot_compensation = 0`,
  `brim_width = 8`, `skirt_loops = 6`, `skirt_distance = 3`, `skirt_height = 1`,
  `initial_layer_line_width = 0.4`, `tree_support_brim_width = 0`) and asserts
  them fail-closed;
- every retired benchy claim is named in the map, not silently dropped.

## Acceptance

- A reference-set artifact carrying the four benchy hashes, linked from the map.
- The swap classified: tessellation-only vs geometry change, with the evidence
  that supports the classification.
- A post-swap 8-cell × 3-tool scoreboard (or an explicit ruling that only the
  four benchy cells are re-taken), with the disclosure attached.
- The map's Decisions-so-far entries updated so no pre-swap benchy ratio reads
  as current.

## Answer

Resolved 2026-11-13 as a measured re-baseline. Evidence:
[`evidence/t145-benchy-rebaseline/`](../evidence/t145-benchy-rebaseline/FINDINGS.md)
(`verify.py` 98/98 checks, 25/25 negative controls rejected; `reduce_t145.py`
regenerates the reduced set from the gitignored raw tree).

### 1. The currency

[`reference.json`](../evidence/t145-benchy-rebaseline/reference.json) is the
currency artifact. Active benchy set = t144's candidate hashes
(`1ae0cfa8…` classic-off, `d9fc80ba…` classic-on, `8faf02b1…` arachne-off,
`cd7d4a2d…` arachne-on), reproduced in this take on the ordinary arm — one hash
across three measured runs per cell, and the four base cells reproduce t144's
base hashes in the same campaign. Retired: t139's four old-export hashes
(`93e21fee…`, `b3f1eccc…`, `3510e876…`, `96cdac85…`). Superseded interim: t143's
replacement-fixture baseline (`a89e449b…`, `dceb7a48…`, `3286449f…`,
`be786718…`).

### 2. What the swap is

**Tessellation-only at the model level, not byte-neutral at the output level.**
The replacement is 11,285,384 bytes / 225,706 triangles against the old export's
225,786 (−80, −0.035 %), with bounds equal to within 4.05e-7 mm and the same
240-layer / 48.0 mm schedule. Re-cutting t138's five sampled planes (mid and top,
inward 0.20 offset, convex hull) reproduces **all ten** old-export areas exactly
at 3 decimals (e.g. L50 mid 1,233.326 mm², L0 mid 513.415 mm²). `tmp/base.stl`
is byte-identical to t138's record.

The G-code delta (old export from t142's output campaign vs new export from
t143's baseline — one output-neutral code change apart, with **all four base
cells byte-identical across the two takes** as the control) is:

- outer-wall path −1.591 mm of 31,891.789 (**−0.0050 %**, classic) and +0.702 mm
  (+0.0022 %, arachne); outer-wall loop area −1e-6; per-layer median 0.0, p95
  0.0, one layer above 0.1 % (L153, −0.113 % / −0.064 %);
- **internal bridge: L40 (z = 8.2) only** — 554.316 → 611.614 mm
  (**+57.298 mm, +7.344 %**), area 28.028 → 31.866 mm² (**+13.694 %**), identical
  in all four cells;
- **support: 176 layers, 61,233.300 → 59,863.119 mm (−1,370.181 mm, −2.238 %)**,
  max-loop area 64,863.446 → 65,525.657 mm² (+1.021 %), supports-on only;
  support interface +1.174 %, brim +0.825 %/+0.851 %, skirt +0.352 % follow;
- output bytes: classic-off −812, classic-on −79,465, arachne-off +118,
  arachne-on −78,773.

So "tessellation-only" names the **model**, and the map may not treat the swap as
byte-neutral: all four benchy hashes move.

### 3. The post-swap scoreboard

Ruling: **only the four benchy cells are re-taken** for the fixture reason, and
the four base cells are re-taken in the same window so the composite does not
mix windows. Tools: Orca and PnP ordinary only — the accelerated timing arm is
unavailable (see §5). 1 warmup + 3 measured runs per cell, interleaved; matched
keys asserted per Orca run; every run exit 0.

| cell | PnP | Orca | ratio | required cut | 1.5x cut |
| --- | ---: | ---: | ---: | ---: | ---: |
| benchy classic-off | 3.912 s | 1.915 s | 2.043x | 51.05 % | 26.57 % |
| benchy classic-on | 6.553 s | 2.442 s | 2.683x | 62.73 % | 44.10 % |
| benchy arachne-off | 3.674 s | 1.983 s | 1.853x | 46.03 % | 19.05 % |
| benchy arachne-on | 6.686 s | 2.591 s | 2.580x | 61.24 % | 41.86 % |
| base classic-off | 27.869 s | 13.414 s | 2.078x | 51.87 % | 27.80 % |
| base classic-on | 70.255 s | 23.414 s | 3.001x | 66.67 % | 50.01 % |
| base arachne-off | 29.602 s | 14.920 s | 1.984x | 49.60 % | 24.40 % |
| base arachne-on | 89.224 s | 25.822 s | 3.455x | 71.06 % | 56.59 % |

Ranges: **1.853x–3.455x** overall, **1.853x–2.683x** benchy-only, **1.984x–3.455x**
base-only; required cut **46.03 %–71.06 %** (**19.05 %–56.59 %** at 1.5x). The
lowest bars are the two benchy supports-off cells; the highest remain the base
supports-on cells. 0 of 8 cells are won; the destination is unchanged.

### 4. Retirements (named, not dropped)

Retired as historical: t139's four benchy hashes and every comparison built on
them; t138's benchy **g-code** measurements (layer-0 outer-wall hulls
519.821/513.186 mm², brim/skirt runs and paths, L0/L10 plane separations) — its
benchy **STL** figures survive because the cross-sections reproduce exactly;
t135 Stage 1's benchy oracle numbers; t142's benchy A/B (20.00/10.56/20.23/12.66 %)
and every pre-swap benchy A/B pair; t121/t124/t139 benchy scoreboard medians and
any composite containing a benchy cell; t126's "benchy classic-off 1.02x at zero
prepass" and t134's benchy planner ratios; t143's four benchy hashes as the
**interim** set superseded by t144. Not retired: base cells and base-only ratios;
t143's planner depth table and A/B benchy rows (all post-swap — fixture mtime
2026-11-12T20:48, depth runs from 20:55); t144's hashes, oracle and A/B.

### 5. Disclosures

- **Accelerated timing arm not taken.** `cargo xtask dist --accelerated
  --edition developer` was attempted in-take and failed at exit 1 with `c1xx:
  fatal error C1083: Cannot open source file: '\\mimalloc-static.cc'` — the
  controlled driver's verbatim-path / MSVC 14.51 `libmimalloc-sys` failure
  (t142). A manual `--cfg pnp_perimeter_spatial_accelerated` host is not a legal
  timing artifact under `docs/23_controlled_perimeter_builds.md`, so no
  substitute was run. Accelerated **output** parity is already on record (t144,
  8/8).
- **Supports-on cells are degraded on both fixtures.** Every PnP supports-on run
  carries code-1200 `support demand … unmet … body rejected: exact-Z occupancy`
  diagnostics: 101 per benchy supports-on cell and 44,474 per base supports-on
  cell (0 on supports-off). The counts are identical on the old export (t135,
  t142) and the new one, so the swap did not cause them; they mean the supports-on
  prints are support-incomplete on the PnP side and their ratios compare a
  smaller support job. Recorded as a pre-existing fairness question.
- **Machine state.** Load 9 % / 12 % sampled; the t139 scoreboard this supersedes
  ran on a slower box (base classic-on 126.27 s vs today's 70.25 s), so no causal
  claim is taken from the cross-take movement — each cell is internally
  interleaved.
- **Three measured runs per cell**, spread ≤ 8.6 % except base arachne-on PnP at
  21.2 %.
- **Orca g-code hashes differ per run** while bytes are constant (4,077,463): the
  file carries a per-run timestamp. PnP output is byte-constant.

